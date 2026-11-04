//! Security decision tests: AI requests never carry config or secrets.
//! - the grammar check refuses `.fed`, dotfiles and non-note paths before any read or HTTP call;
//! - credentials travel only in the selected provider's auth header, never in bodies or URLs;
//! - neither error messages nor logs echo credentials or upstream bodies.

mod common;

use std::sync::{Arc, Mutex};

use common::*;
use frankmd_ai::{AiRequestError, NoReferenceImages, NoteSource, fix_grammar_note};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A `.fed` full of secrets, as a note source would see it.
const FED_WITH_SECRETS: &str =
    "openai_api_key = sk-openai-secret\naws_secret_access_key = aws-very-secret\n";

/// Records every read; `.fed` answers with the secrets above.
#[derive(Default)]
struct RecordingNotes(Mutex<Vec<String>>);

impl NoteSource for RecordingNotes {
    fn read_note(&self, path: &str) -> Option<String> {
        self.0.lock().expect("lock").push(path.to_owned());
        Some(if path.ends_with(".fed") {
            FED_WITH_SECRETS.to_owned()
        } else {
            "Some note text".to_owned()
        })
    }
}

#[derive(Clone, Default)]
struct LogBuf(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for LogBuf {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("lock").extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl LogBuf {
    fn install(&self) -> tracing::subscriber::DefaultGuard {
        let writer = self.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_writer(move || writer.clone())
            .finish();
        tracing::subscriber::set_default(subscriber)
    }
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("lock")).into_owned()
    }
}

/// Every request the mock saw, as one string: URL + headers + body.
async fn everything_sent(server: &MockServer) -> Vec<(String, String)> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            (
                format!("{}{:?}", r.url, r.headers),
                String::from_utf8_lossy(&r.body).into_owned(),
            )
        })
        .collect()
}

// security: AI requests never include config or secrets (`.fed` passed `Note.find` in the original)
#[tokio::test]
async fn grammar_fix_refuses_config_and_non_note_paths_before_any_http_call() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_completion("x")))
        .expect(0)
        .mount(&server)
        .await;
    let s = mocked(&server.uri(), None, &[("OPENAI_API_KEY", "sk-test")]);
    let notes = RecordingNotes::default();
    for path in [
        ".fed",
        "sub/.fed",
        "./.fed",
        "../.fed",
        ".hidden.md",
        ".git/config",
        ".obsidian/x.md",
        "../outside.md",
        "/etc/passwd",
        "a/../.fed",
    ] {
        let err = fix_grammar_note(&s, &notes, path).await.expect_err(path);
        assert_eq!(err, AiRequestError::Forbidden, "{path}");
    }
    assert!(
        notes.0.lock().expect("lock").is_empty(),
        "refused paths must not even be read"
    );
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty()
    );
}

// security: a normal note still works, and only its text is sent
#[tokio::test]
async fn grammar_fix_on_a_note_sends_only_the_note_text() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_completion("Fixed")))
        .expect(1)
        .mount(&server)
        .await;
    let fed = "openai_api_key = sk-openai-secret\nyoutube_api_key = yt-secret\n";
    let s = mocked(
        &server.uri(),
        Some(fed),
        &[("AWS_SECRET_ACCESS_KEY", "aws-very-secret")],
    );
    let resp = fix_grammar_note(&s, &RecordingNotes::default(), "folder/note")
        .await
        .expect("fix");
    assert_eq!(resp.original, "Some note text");
    for (_, body) in everything_sent(&server).await {
        assert!(body.contains("Some note text"));
        for secret in [
            "sk-openai-secret",
            "yt-secret",
            "aws-very-secret",
            "youtube_api_key",
        ] {
            assert!(!body.contains(secret), "{secret} in body");
        }
    }
}

// security: credentials live only in the provider's auth header — never in body or URL
#[tokio::test]
async fn api_keys_never_appear_in_request_bodies_or_urls() {
    let cases: [(&str, &str); 4] = [
        ("OPENAI_API_KEY", "sk-openai-xyz"),
        ("OPENROUTER_API_KEY", "sk-or-xyz"),
        ("ANTHROPIC_API_KEY", "sk-ant-xyz"),
        ("GEMINI_API_KEY", "gemini-xyz"),
    ];
    for (var, key) in cases {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(chat_completion("ok")))
            .mount(&server)
            .await;
        let _ = mocked(&server.uri(), None, &[(var, key)])
            .fix_grammar(Some("hello"))
            .await;
        let sent = everything_sent(&server).await;
        assert!(!sent.is_empty(), "{var}: no request");
        for (url_and_headers, body) in sent {
            assert!(!body.contains(key), "{var}: key in body");
            assert!(
                !url_and_headers
                    .split("{")
                    .next()
                    .unwrap_or_default()
                    .contains(key),
                "{var}: key in URL"
            );
        }
    }
}

// security: image generation sends the key only as the bearer header
#[tokio::test]
async fn image_generation_key_never_in_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openrouter_image(
            "",
            "image/png",
            &b64(b"img"),
        )))
        .mount(&server)
        .await;
    let s = mocked(
        &server.uri(),
        Some("openrouter_api_key = sk-or-img-secret\n"),
        &[],
    );
    s.generate_image(Some("A cat"), None, &NoReferenceImages)
        .await
        .expect("image");
    for (_, body) in everything_sent(&server).await {
        assert!(!body.contains("sk-or-img-secret"));
    }
}

// security: error messages and logs never echo keys, upstream bodies or note content
#[tokio::test]
async fn errors_and_logs_never_echo_keys_or_content() {
    let logs = LogBuf::default();
    let _guard = logs.install();
    let cases: [(&str, &str, u16); 4] = [
        ("OPENAI_API_KEY", "sk-openai-leak", 401),
        ("OPENROUTER_API_KEY", "sk-or-leak", 403),
        ("ANTHROPIC_API_KEY", "sk-ant-leak", 400),
        ("GEMINI_API_KEY", "gemini-leak", 500),
    ];
    for (var, key, status) in cases {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_string(format!("invalid key {key} for request")),
            )
            .mount(&server)
            .await;
        let err = mocked(&server.uri(), None, &[(var, key)])
            .fix_grammar(Some("private note body"))
            .await
            .expect_err(var);
        assert_eq!(err.to_string(), "AI provider request failed");
        assert!(!format!("{err:?}").contains(key));
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string("bad key sk-or-img-leak"))
        .mount(&server)
        .await;
    let s = mocked(
        &server.uri(),
        Some("openrouter_api_key = sk-or-img-leak\n"),
        &[],
    );
    let err = s
        .generate_image(Some("secret prompt words"), None, &NoReferenceImages)
        .await
        .expect_err("401");
    assert_eq!(err.to_string(), "Image generation failed");

    let logged = logs.text();
    for needle in [
        "leak",
        "private note body",
        "secret prompt words",
        "invalid key",
    ] {
        assert!(!logged.contains(needle), "{needle} logged: {logged}");
    }
    assert!(
        logged.contains("AI error"),
        "failures are still logged: {logged}"
    );
}

// security: the service's Debug output never prints the config (which holds credentials)
#[test]
fn service_debug_hides_config() {
    let s = service(Some("openai_api_key = sk-debug-secret\n"), &[]);
    assert!(!format!("{s:?}").contains("sk-debug-secret"));
}
