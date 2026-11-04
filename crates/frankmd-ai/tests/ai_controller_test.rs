//! Port of `test/controllers/ai_controller_test.rb` (17 tests). `GET /ai/config` →
//! [`AiService::provider_info`], `GET /ai/image_config` → `image_generation_info`,
//! `POST /ai/fix_grammar` → [`fix_grammar_note`] (note read from disk through
//! [`NoteSource`]), `POST /ai/generate_image` → [`generate_image_request`].
//! HTTP statuses become [`AiRequestError`] variants carrying the i18n key.

mod common;

use std::collections::HashMap;

use common::*;
use frankmd_ai::{
    AiError, AiRequestError, NoReferenceImages, NoteSource, fix_grammar_note,
    generate_image_request,
};
use frankmd_config::AiProviderId;

#[derive(Debug, Default)]
struct Notes(HashMap<String, String>);

impl NoteSource for Notes {
    fn read_note(&self, path: &str) -> Option<String> {
        self.0.get(path).cloned()
    }
}

fn notes(entries: &[(&str, &str)]) -> Notes {
    Notes(
        entries
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
    )
}

// port: test/controllers/ai_controller_test.rb:34 "config returns enabled false when no API keys"
#[test]
fn config_disabled_without_keys() {
    let info = service(None, &[]).provider_info();
    assert!(!info.enabled);
    assert_eq!(info.provider, None);
    assert_eq!(info.model, None);
    assert!(info.available_providers.is_empty());
}

// port: test/controllers/ai_controller_test.rb:45 "config returns enabled true when OpenAI key is set"
#[test]
fn config_openai() {
    let info = service(None, &[("OPENAI_API_KEY", "sk-test")]).provider_info();
    assert!(info.enabled);
    assert_eq!(info.provider, Some(AiProviderId::Openai));
    assert_eq!(info.model.as_deref(), Some("gpt-4o-mini"));
    assert!(info.available_providers.contains(&AiProviderId::Openai));
}

// port: test/controllers/ai_controller_test.rb:58 "config returns enabled true when OpenRouter key is set"
#[test]
fn config_openrouter() {
    let info = service(None, &[("OPENROUTER_API_KEY", "sk-or-test")]).provider_info();
    assert!(info.enabled);
    assert_eq!(info.provider, Some(AiProviderId::Openrouter));
    assert_eq!(info.model.as_deref(), Some("openai/gpt-4o-mini"));
}

// port: test/controllers/ai_controller_test.rb:70 "config returns enabled true when Anthropic key is set"
#[test]
fn config_anthropic() {
    let info = service(None, &[("ANTHROPIC_API_KEY", "sk-ant-test")]).provider_info();
    assert!(info.enabled);
    assert_eq!(info.provider, Some(AiProviderId::Anthropic));
    assert_eq!(info.model.as_deref(), Some("claude-sonnet-4-20250514"));
}

// port: test/controllers/ai_controller_test.rb:82 "config returns enabled true when Gemini key is set"
#[test]
fn config_gemini() {
    let info = service(None, &[("GEMINI_API_KEY", "gemini-test")]).provider_info();
    assert!(info.enabled);
    assert_eq!(info.provider, Some(AiProviderId::Gemini));
    assert_eq!(info.model.as_deref(), Some("gemini-2.0-flash"));
}

// port: test/controllers/ai_controller_test.rb:94 "config returns enabled true when Ollama base is set"
#[test]
fn config_ollama() {
    let info = service(None, &[("OLLAMA_API_BASE", "http://localhost:11434")]).provider_info();
    assert!(info.enabled);
    assert_eq!(info.provider, Some(AiProviderId::Ollama));
    assert_eq!(info.model.as_deref(), Some("llama3.2:latest"));
}

// port: test/controllers/ai_controller_test.rb:106 "config returns correct priority with multiple providers"
#[test]
fn config_priority_with_multiple() {
    let info = service(
        None,
        &[
            ("OLLAMA_API_BASE", "http://localhost:11434"),
            ("OPENAI_API_KEY", "sk-test"),
            ("ANTHROPIC_API_KEY", "sk-ant-test"),
        ],
    )
    .provider_info();
    assert_eq!(info.provider, Some(AiProviderId::Openai)); // Highest priority
    assert_eq!(info.available_providers.len(), 3);
    for p in [
        AiProviderId::Ollama,
        AiProviderId::Openai,
        AiProviderId::Anthropic,
    ] {
        assert!(info.available_providers.contains(&p), "{p:?}");
    }
}

// port: test/controllers/ai_controller_test.rb:122 "config respects ai_provider override"
#[test]
fn config_ai_provider_override() {
    let info = service(
        None,
        &[
            ("OLLAMA_API_BASE", "http://localhost:11434"),
            ("OPENAI_API_KEY", "sk-test"),
            ("AI_PROVIDER", "openai"),
        ],
    )
    .provider_info();
    assert_eq!(info.provider, Some(AiProviderId::Openai));
}

// port: test/controllers/ai_controller_test.rb:134 "config respects ai_model override"
#[test]
fn config_ai_model_override() {
    let info = service(
        None,
        &[("OPENAI_API_KEY", "sk-test"), ("AI_MODEL", "gpt-4-turbo")],
    )
    .provider_info();
    assert_eq!(info.model.as_deref(), Some("gpt-4-turbo"));
}

// port: test/controllers/ai_controller_test.rb:146 "fix_grammar returns error when path is blank"
#[tokio::test]
async fn fix_grammar_blank_path() {
    let err = fix_grammar_note(&service(None, &[]), &notes(&[]), "")
        .await
        .expect_err("blank");
    // en: "No file path provided"
    assert_eq!(err, AiRequestError::BadRequest("errors.no_file_provided"));
}

// port: test/controllers/ai_controller_test.rb:154 "fix_grammar returns error when note not found"
#[tokio::test]
async fn fix_grammar_note_not_found() {
    let err = fix_grammar_note(&service(None, &[]), &notes(&[]), "nonexistent.md")
        .await
        .expect_err("404");
    assert_eq!(err, AiRequestError::NotFound("errors.note_not_found"));
}

// port: test/controllers/ai_controller_test.rb:162 "fix_grammar returns error when note is empty"
#[tokio::test]
async fn fix_grammar_empty_note() {
    let err = fix_grammar_note(&service(None, &[]), &notes(&[("empty.md", "")]), "empty.md")
        .await
        .expect_err("empty");
    // en: "Note is empty"
    assert_eq!(err, AiRequestError::BadRequest("errors.note_is_empty"));
}

// port: test/controllers/ai_controller_test.rb:174 "fix_grammar returns error when AI not configured"
#[tokio::test]
async fn fix_grammar_not_configured() {
    // Validation runs before any network call: no provider → 422 "AI not configured".
    let err = fix_grammar_note(
        &service(None, &[]),
        &notes(&[("test.md", "Hello world")]),
        "test.md",
    )
    .await
    .expect_err("not configured");
    assert_eq!(err, AiRequestError::Service(AiError::NotConfigured));
    assert!(err.to_string().contains("not configured"));
}

// port: test/controllers/ai_controller_test.rb:187 "image_config returns enabled false when no OpenRouter key"
#[test]
fn image_config_disabled() {
    let info = service(None, &[]).image_generation_info();
    assert!(!info.enabled);
    assert_eq!(info.model, "google/gemini-3.1-flash-image-preview");
}

// port: test/controllers/ai_controller_test.rb:196 "image_config returns enabled true when OpenRouter key is set"
#[test]
fn image_config_enabled() {
    let info = service(None, &[("OPENROUTER_API_KEY", "sk-or-test-key")]).image_generation_info();
    assert!(info.enabled);
    assert_eq!(info.model, "google/gemini-3.1-flash-image-preview");
}

// port: test/controllers/ai_controller_test.rb:208 "generate_image returns error when prompt is blank"
#[tokio::test]
async fn generate_image_blank_prompt() {
    let err = generate_image_request(&service(None, &[]), "", None, &NoReferenceImages)
        .await
        .expect_err("blank");
    // en: "No prompt provided"
    assert_eq!(err, AiRequestError::BadRequest("errors.no_prompt_provided"));
}

// port: test/controllers/ai_controller_test.rb:216 "generate_image returns error when not configured"
#[tokio::test]
async fn generate_image_not_configured() {
    let err = generate_image_request(&service(None, &[]), "A sunset", None, &NoReferenceImages)
        .await
        .expect_err("nc");
    assert_eq!(err, AiRequestError::Service(AiError::ImageNotConfigured));
    assert!(err.to_string().contains("not configured"));
}
