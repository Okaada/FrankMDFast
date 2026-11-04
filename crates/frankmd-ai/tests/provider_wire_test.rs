//! Wire-shape tests for the providers the original only reached through
//! RubyLLM (no original test per provider; derived from backend.md §9.1's
//! endpoint table). Each asserts the request shape and that only the selected
//! provider's credential is sent.

mod common;

use common::*;
use frankmd_ai::GRAMMAR_PROMPT;
use frankmd_config::AiProviderId;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// derived from: backend.md §9.1 (openrouter row) + test/services/ai_service_test.rb:265
#[tokio::test]
async fn openrouter_grammar_fix_uses_chat_completions() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .and(header("authorization", "Bearer sk-or-test"))
        .and(body_partial_json(serde_json::json!({"model": "openai/gpt-4o-mini",
            "messages": [{"role": "system", "content": GRAMMAR_PROMPT}, {"role": "user", "content": "teh text"}]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_completion("the text")))
        .expect(1)
        .mount(&server)
        .await;
    let fix = mocked(&server.uri(), None, &[("OPENROUTER_API_KEY", "sk-or-test")])
        .fix_grammar(Some("teh text"))
        .await
        .expect("fix");
    assert_eq!(fix.corrected, "the text");
    assert_eq!(fix.provider, AiProviderId::Openrouter);
}

// derived from: backend.md §9.1 (gemini row)
#[tokio::test]
async fn gemini_grammar_fix_uses_generate_content() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .and(header("x-goog-api-key", "gemini-test"))
        .and(body_partial_json(serde_json::json!({
            "systemInstruction": {"parts": [{"text": GRAMMAR_PROMPT}]},
            "contents": [{"role": "user", "parts": [{"text": "teh text"}]}]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "candidates": [{"content": {"role": "model", "parts": [{"text": "the text"}]}}]})))
        .expect(1)
        .mount(&server)
        .await;
    let fix = mocked(&server.uri(), None, &[("GEMINI_API_KEY", "gemini-test")])
        .fix_grammar(Some("teh text"))
        .await
        .expect("fix");
    assert_eq!(fix.corrected, "the text");
    assert_eq!(fix.model, "gemini-2.0-flash");
}

// derived from: backend.md §9.1 (ollama row: `<ollama_api_base>/chat/completions`, no auth)
#[tokio::test]
async fn ollama_grammar_fix_posts_to_configured_base_without_auth() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_partial_json(
            serde_json::json!({"model": "llama3.2:latest"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_completion("the text")))
        .expect(1)
        .mount(&server)
        .await;
    let base = format!("{}/v1", server.uri());
    let fix = service(None, &[("OLLAMA_API_BASE", base.as_str())])
        .fix_grammar(Some("teh text"))
        .await
        .expect("fix");
    assert_eq!(fix.provider, AiProviderId::Ollama);
    let reqs = server.received_requests().await.unwrap_or_default();
    assert!(
        reqs.iter()
            .all(|r| !r.headers.contains_key("authorization")),
        "ollama must not get a bearer token"
    );
}

// derived from: ai_service.rb configure_client ("Clear ALL provider keys first to avoid cross-contamination")
#[tokio::test]
async fn only_the_selected_providers_key_is_sent() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_completion("ok")))
        .mount(&server)
        .await;
    let s = mocked(
        &server.uri(),
        None,
        &[
            ("OPENAI_API_KEY", "sk-openai"),
            ("ANTHROPIC_API_KEY", "sk-ant-secret"),
        ],
    );
    s.fix_grammar(Some("x")).await.expect("fix");
    for r in server.received_requests().await.unwrap_or_default() {
        let all = format!("{:?}{}", r.headers, String::from_utf8_lossy(&r.body));
        assert!(
            !all.contains("sk-ant-secret"),
            "anthropic key leaked into openai request"
        );
    }
}

// derived from: backend.md §9.1 ("Errors are logged without the content"; HP-011): upstream error text never surfaces
#[tokio::test]
async fn anthropic_error_body_is_not_surfaced() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"error":{"message":"invalid x-api-key sk-ant-xyz"}}"#),
        )
        .mount(&server)
        .await;
    let err = mocked(&server.uri(), None, &[("ANTHROPIC_API_KEY", "sk-ant-xyz")])
        .fix_grammar(Some("x"))
        .await
        .expect_err("401");
    assert_eq!(err.to_string(), "AI provider request failed");
}

// derived from: backend.md §9.2 (gemini key via query is NOT used; header only)
#[tokio::test]
async fn gemini_key_is_not_put_in_the_query_string() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("key", "gemini-test"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "candidates": [{"content": {"parts": [{"text": "ok"}]}}]})))
        .mount(&server)
        .await;
    let _ = mocked(&server.uri(), None, &[("GEMINI_API_KEY", "gemini-test")])
        .fix_grammar(Some("x"))
        .await;
}
