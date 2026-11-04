//! OpenRouter image generation: request body and response parsing.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::{AiError, Attachment, ChatResponse, GeneratedImage, ReferenceImage};

/// MIME type used when the provider does not state one (**[COMPAT]**).
const DEFAULT_IMAGE_MIME: &str = "image/png";

/// Chat-completions body asking for `modalities: ["text","image"]`. The prompt
/// is sent verbatim; a reference image becomes a second, `image_url` part.
pub(crate) fn request_body(model: &str, prompt: &str, reference: Option<&ReferenceImage>) -> Value {
    let content = match reference {
        None => json!(prompt),
        Some(image) => json!([
            {"type": "text", "text": prompt},
            {"type": "image_url", "image_url": {"url": data_uri(image)}},
        ]),
    };
    json!({
        "model": model,
        "modalities": ["text", "image"],
        "messages": [{"role": "user", "content": content}],
    })
}

fn data_uri(image: &ReferenceImage) -> String {
    format!(
        "data:{};base64,{}",
        image.mime_type,
        STANDARD.encode(&image.bytes)
    )
}

/// Parse an OpenRouter chat-completions body into a [`ChatResponse`]
/// (`choices[0].message.content` + `message.images[].image_url.url` data URIs).
///
/// Plain (non-`data:`) image URLs are skipped: fetching them would need the
/// egress-guarded client, and OpenRouter's image models answer with data URIs.
pub fn parse_openrouter_response(body: &[u8]) -> Result<ChatResponse, AiError> {
    let answer: Value = serde_json::from_slice(body).map_err(|_| AiError::ImageFailed)?;
    Ok(chat_response(&answer))
}

/// [`parse_openrouter_response`] on an already-parsed body.
pub(crate) fn chat_response(answer: &Value) -> ChatResponse {
    let content = crate::chat::openai_text(answer).unwrap_or_default();
    let images = answer
        .pointer("/choices/0/message/images")
        .and_then(Value::as_array);
    let attachments = images
        .into_iter()
        .flatten()
        .filter_map(|image| image.pointer("/image_url/url").and_then(Value::as_str))
        .filter_map(|url| {
            let decoded = decode_data_uri(url);
            if decoded.is_none() {
                tracing::warn!("skipping generated image that is not a base64 data URI");
            }
            decoded
        })
        .collect();
    ChatResponse {
        content,
        attachments,
    }
}

/// `data:<mime>;base64,<payload>` → bytes + mime.
fn decode_data_uri(url: &str) -> Option<Attachment> {
    let rest = url.strip_prefix("data:")?;
    let (meta, payload) = rest.split_once(',')?;
    let mime = meta.strip_suffix(";base64")?;
    let content = STANDARD.decode(payload.trim()).ok()?;
    let mime_type = (!mime.is_empty()).then(|| mime.to_owned());
    Some(Attachment { content, mime_type })
}

/// **[COMPAT]** `AiService.extract_image_from_response`: first attachment →
/// base64 + mime (default `image/png`) + model; none → [`AiError::NoImageData`].
pub fn extract_image_from_response(
    response: &ChatResponse,
    model: &str,
) -> Result<GeneratedImage, AiError> {
    let Some(attachment) = response.attachments.first() else {
        if !response.content.trim().is_empty() {
            // Length only: the text may quote the prompt.
            tracing::warn!(
                text_length = response.content.chars().count(),
                "image model returned text instead of an image"
            );
        }
        return Err(AiError::NoImageData);
    };
    Ok(GeneratedImage {
        data: STANDARD.encode(&attachment.content),
        mime_type: attachment
            .mime_type
            .clone()
            .unwrap_or_else(|| DEFAULT_IMAGE_MIME.to_owned()),
        model: model.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_data_uris_and_skips_plain_urls() {
        let body = json!({"choices": [{"message": {"content": null, "images": [
            {"image_url": {"url": "https://example.com/a.png"}},
            {"image_url": {"url": "data:image/webp;base64,AAEC"}},
        ]}}]});
        let resp = parse_openrouter_response(body.to_string().as_bytes()).expect("parse");
        assert_eq!(resp.content, "");
        assert_eq!(
            resp.attachments,
            vec![Attachment {
                content: vec![0, 1, 2],
                mime_type: Some("image/webp".into())
            }]
        );
    }

    #[test]
    fn invalid_json_is_image_failed() {
        assert_eq!(
            parse_openrouter_response(b"not json"),
            Err(AiError::ImageFailed)
        );
    }

    #[test]
    fn missing_mime_defaults_to_png() {
        let resp = ChatResponse {
            content: String::new(),
            attachments: vec![Attachment {
                content: vec![1],
                mime_type: None,
            }],
        };
        assert_eq!(
            extract_image_from_response(&resp, "m")
                .expect("image")
                .mime_type,
            "image/png"
        );
    }
}
