//! Chat wire formats: one system instruction plus one user message, non-streaming.
//!
//! Three shapes cover the five providers:
//! - OpenAI-compatible chat completions (OpenAI, OpenRouter, Ollama — and any
//!   future generic OpenAI-compatible endpoint, HP-020),
//! - Anthropic Messages,
//! - Gemini `generateContent`.

use secrecy::SecretString;
use serde_json::{Value, json};

use crate::http::{Header, PostError, post_json};

/// Anthropic API version header value (ruby_llm 2.1.1).
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Title header OpenRouter shows in its dashboard (ruby_llm sent its own name).
const OPENROUTER_TITLE: &str = "FrankMD";

/// Where and how to send one chat request.
pub(crate) enum ChatTarget {
    /// `POST <url>` with an OpenAI chat-completions body.
    OpenAiCompatible {
        /// Full endpoint URL (`…/chat/completions`).
        url: String,
        /// Bearer token; `None` for keyless servers such as Ollama.
        bearer: Option<SecretString>,
        /// OpenRouter wants an `X-Title`.
        openrouter: bool,
    },
    /// `POST <url>` with an Anthropic Messages body.
    Anthropic {
        /// Full endpoint URL (`…/v1/messages`).
        url: String,
        /// `x-api-key`.
        key: SecretString,
    },
    /// `POST <url>` with a Gemini `generateContent` body.
    Gemini {
        /// Full endpoint URL (`…/v1beta/models/<model>:generateContent`).
        url: String,
        /// `x-goog-api-key` (never put in the query string).
        key: SecretString,
    },
}

/// Why a chat request produced no text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ChatError {
    /// HTTP-level failure.
    #[error(transparent)]
    Post(#[from] PostError),
    /// 2xx answer without the expected text.
    #[error("response without text content")]
    NoContent,
}

/// Send `system` + `user` to `model` and return the assistant text.
pub(crate) async fn complete(
    client: &frankmd_net::TrustedClient,
    target: ChatTarget,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String, ChatError> {
    match target {
        ChatTarget::OpenAiCompatible {
            url,
            bearer,
            openrouter,
        } => {
            let mut headers = Vec::new();
            if let Some(token) = bearer {
                headers.push(Header::Secret("authorization", bearer_value(&token)));
            }
            if openrouter {
                headers.push(Header::Plain("x-title", OPENROUTER_TITLE));
            }
            let body = openai_body(model, system, user);
            let answer = post_json(client, &url, &headers, &body).await?;
            openai_text(&answer).ok_or(ChatError::NoContent)
        }
        ChatTarget::Anthropic { url, key } => {
            let headers = [
                Header::Secret("x-api-key", key),
                Header::Plain("anthropic-version", ANTHROPIC_VERSION),
            ];
            let body = json!({
                "model": model,
                "max_tokens": anthropic_max_tokens(model),
                "system": system,
                "messages": [{"role": "user", "content": user}],
            });
            let answer = post_json(client, &url, &headers, &body).await?;
            anthropic_text(&answer).ok_or(ChatError::NoContent)
        }
        ChatTarget::Gemini { url, key } => {
            let headers = [Header::Secret("x-goog-api-key", key)];
            let body = json!({
                "systemInstruction": {"parts": [{"text": system}]},
                "contents": [{"role": "user", "parts": [{"text": user}]}],
            });
            let answer = post_json(client, &url, &headers, &body).await?;
            gemini_text(&answer).ok_or(ChatError::NoContent)
        }
    }
}

/// `Bearer <token>` as a secret.
pub(crate) fn bearer_value(token: &SecretString) -> SecretString {
    use secrecy::ExposeSecret;
    SecretString::from(format!("Bearer {}", token.expose_secret()))
}

fn openai_body(model: &str, system: &str, user: &str) -> Value {
    json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
    })
}

/// Anthropic requires `max_tokens`. ruby_llm took it from its model registry;
/// these are the output limits of the Claude families (Claude 3 = 4096,
/// Claude 3.5 = 8192, later models ≥ 32000).
fn anthropic_max_tokens(model: &str) -> u32 {
    if model.starts_with("claude-3-5") {
        8192
    } else if model.starts_with("claude-3-") && !model.starts_with("claude-3-7") {
        4096
    } else {
        32000
    }
}

/// `choices[0].message.content`: a string, or an array of `{type:"text", text}` parts.
pub(crate) fn openai_text(answer: &Value) -> Option<String> {
    let content = answer.pointer("/choices/0/message/content")?;
    match content {
        Value::String(s) => Some(s.clone()),
        Value::Array(parts) => Some(join_texts(parts.iter().filter_map(|p| p.get("text")))),
        _ => None,
    }
}

/// Concatenated `content[].text` of the `text` blocks.
fn anthropic_text(answer: &Value) -> Option<String> {
    let blocks = answer.get("content")?.as_array()?;
    let texts = blocks
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text"));
    Some(join_texts(texts))
}

/// Concatenated `candidates[0].content.parts[].text`, skipping thought parts.
fn gemini_text(answer: &Value) -> Option<String> {
    let parts = answer.pointer("/candidates/0/content/parts")?.as_array()?;
    let texts = parts
        .iter()
        .filter(|p| p.get("thought").and_then(Value::as_bool) != Some(true))
        .filter_map(|p| p.get("text"));
    Some(join_texts(texts))
}

fn join_texts<'a>(texts: impl Iterator<Item = &'a Value>) -> String {
    texts.filter_map(Value::as_str).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_limits_follow_model_family() {
        assert_eq!(anthropic_max_tokens("claude-3-opus-20240229"), 4096);
        assert_eq!(anthropic_max_tokens("claude-3-5-sonnet-20241022"), 8192);
        assert_eq!(anthropic_max_tokens("claude-3-7-sonnet-20250219"), 32000);
        assert_eq!(anthropic_max_tokens("claude-sonnet-4-20250514"), 32000);
    }

    #[test]
    fn openai_text_accepts_part_arrays() {
        let v = json!({"choices": [{"message": {"content": [{"type": "text", "text": "a"}, {"type": "text", "text": "b"}]}}]});
        assert_eq!(openai_text(&v).as_deref(), Some("ab"));
        assert_eq!(openai_text(&json!({"choices": []})), None);
    }

    #[test]
    fn gemini_skips_thoughts() {
        let v = json!({"candidates": [{"content": {"parts": [{"text": "hmm", "thought": true}, {"text": "ok"}]}}]});
        assert_eq!(gemini_text(&v).as_deref(), Some("ok"));
    }
}
