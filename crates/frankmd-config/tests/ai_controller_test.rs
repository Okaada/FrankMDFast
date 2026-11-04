//! Port of the config-resolution parts of `test/controllers/ai_controller_test.rb`
//! (17 tests: 11 ported, 6 skipped → owned by frankmd-ai).
//!
//! `GET /ai/config` ↦ `{enabled: feature_available(Ai), provider:
//! effective_ai_provider, model: effective_ai_model, available_providers:
//! ai_providers_available}`; `GET /ai/image_config` ↦ `{enabled:
//! image_generation_key.is_some(), model: get(image_generation_model)}`.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_config::{AiProviderId, EnvSnapshot, Feature, Key};

/// `GET /ai/config` as the original controller assembles it.
struct AiConfig {
    enabled: bool,
    provider: Option<&'static str>,
    model: Option<String>,
    available: Vec<&'static str>,
}

fn ai_config(env: EnvSnapshot) -> AiConfig {
    let rc = TestVault::new().open(env).current();
    AiConfig {
        enabled: rc.feature_available(Feature::Ai),
        provider: rc.effective_ai_provider().map(AiProviderId::as_str),
        model: rc.effective_ai_model(),
        available: rc
            .ai_providers_available()
            .into_iter()
            .map(AiProviderId::as_str)
            .collect(),
    }
}

// port: test/controllers/ai_controller_test.rb:34 "config returns enabled false when no API keys"
#[test]
fn config_returns_enabled_false_when_no_api_keys() {
    let c = ai_config(EnvSnapshot::default());
    assert!(!c.enabled);
    assert_eq!(c.provider, None);
    assert_eq!(c.model, None);
    assert!(c.available.is_empty());
}

// port: test/controllers/ai_controller_test.rb:45 "config returns enabled true when OpenAI key is set"
#[test]
fn config_returns_enabled_true_when_openai_key_is_set() {
    let c = ai_config(env(&[("OPENAI_API_KEY", "sk-test-key")]));
    assert!(c.enabled);
    assert_eq!(c.provider, Some("openai"));
    assert_eq!(c.model.as_deref(), Some("gpt-4o-mini"));
    assert!(c.available.contains(&"openai"));
}

// port: test/controllers/ai_controller_test.rb:58 "config returns enabled true when OpenRouter key is set"
#[test]
fn config_returns_enabled_true_when_openrouter_key_is_set() {
    let c = ai_config(env(&[("OPENROUTER_API_KEY", "sk-or-test-key")]));
    assert!(c.enabled);
    assert_eq!(c.provider, Some("openrouter"));
    assert_eq!(c.model.as_deref(), Some("openai/gpt-4o-mini"));
}

// port: test/controllers/ai_controller_test.rb:70 "config returns enabled true when Anthropic key is set"
#[test]
fn config_returns_enabled_true_when_anthropic_key_is_set() {
    let c = ai_config(env(&[("ANTHROPIC_API_KEY", "sk-ant-test-key")]));
    assert!(c.enabled);
    assert_eq!(c.provider, Some("anthropic"));
    assert_eq!(c.model.as_deref(), Some("claude-sonnet-4-20250514"));
}

// port: test/controllers/ai_controller_test.rb:82 "config returns enabled true when Gemini key is set"
#[test]
fn config_returns_enabled_true_when_gemini_key_is_set() {
    let c = ai_config(env(&[("GEMINI_API_KEY", "gemini-test-key")]));
    assert!(c.enabled);
    assert_eq!(c.provider, Some("gemini"));
    assert_eq!(c.model.as_deref(), Some("gemini-2.0-flash"));
}

// port: test/controllers/ai_controller_test.rb:94 "config returns enabled true when Ollama base is set"
#[test]
fn config_returns_enabled_true_when_ollama_base_is_set() {
    let c = ai_config(env(&[("OLLAMA_API_BASE", "http://localhost:11434")]));
    assert!(c.enabled);
    assert_eq!(c.provider, Some("ollama"));
    assert_eq!(c.model.as_deref(), Some("llama3.2:latest"));
}

// port: test/controllers/ai_controller_test.rb:106 "config returns correct priority with multiple providers"
#[test]
fn config_returns_correct_priority_with_multiple_providers() {
    let c = ai_config(env(&[
        ("OLLAMA_API_BASE", "http://localhost:11434"),
        ("OPENAI_API_KEY", "sk-test"),
        ("ANTHROPIC_API_KEY", "sk-ant-test"),
    ]));
    assert_eq!(c.provider, Some("openai"));
    assert_eq!(c.available.len(), 3);
    for p in ["ollama", "openai", "anthropic"] {
        assert!(c.available.contains(&p), "missing {p}");
    }
}

// port: test/controllers/ai_controller_test.rb:122 "config respects ai_provider override"
#[test]
fn config_respects_ai_provider_override() {
    let c = ai_config(env(&[
        ("OLLAMA_API_BASE", "http://localhost:11434"),
        ("OPENAI_API_KEY", "sk-test"),
        ("AI_PROVIDER", "openai"),
    ]));
    assert_eq!(c.provider, Some("openai"));
}

// port: test/controllers/ai_controller_test.rb:134 "config respects ai_model override"
#[test]
fn config_respects_ai_model_override() {
    let c = ai_config(env(&[
        ("OPENAI_API_KEY", "sk-test"),
        ("AI_MODEL", "gpt-4-turbo"),
    ]));
    assert_eq!(c.model.as_deref(), Some("gpt-4-turbo"));
}

// skip: test/controllers/ai_controller_test.rb:146 "fix_grammar returns error when path is blank"        → frankmd-ai (request validation)
// skip: test/controllers/ai_controller_test.rb:154 "fix_grammar returns error when note not found"       → frankmd-ai / workspace
// skip: test/controllers/ai_controller_test.rb:162 "fix_grammar returns error when note is empty"        → frankmd-ai
// skip: test/controllers/ai_controller_test.rb:174 "fix_grammar returns error when AI not configured"    → frankmd-ai (its config input is `effective_ai_provider() == None`, covered above)
// skip: test/controllers/ai_controller_test.rb:208 "generate_image returns error when prompt is blank"   → frankmd-ai
// skip: test/controllers/ai_controller_test.rb:216 "generate_image returns error when not configured"    → frankmd-ai (config input is `image_generation_key() == None`, covered below)

const IMAGE_MODEL: &str = "google/gemini-3.1-flash-image-preview";

// port: test/controllers/ai_controller_test.rb:187 "image_config returns enabled false when no OpenRouter key"
#[test]
fn image_config_returns_enabled_false_when_no_openrouter_key() {
    let rc = TestVault::new().open_empty_env().current();
    assert!(rc.image_generation_key().is_none());
    assert_eq!(
        get_str(&rc, Key::ImageGenerationModel).as_deref(),
        Some(IMAGE_MODEL)
    );
}

// port: test/controllers/ai_controller_test.rb:196 "image_config returns enabled true when OpenRouter key is set"
#[test]
fn image_config_returns_enabled_true_when_openrouter_key_is_set() {
    let rc = TestVault::new()
        .open(env(&[("OPENROUTER_API_KEY", "sk-or-test-key")]))
        .current();
    assert!(rc.image_generation_key().is_some());
    assert_eq!(
        get_str(&rc, Key::ImageGenerationModel).as_deref(),
        Some(IMAGE_MODEL)
    );
}
