//! Web-search tests of `test/controllers/images_controller_test.rb` (the 4
//! blank-query/no-key tests + the 5 `SearchActionsTest` tests; the rest of that
//! file is ported in images_controller_test.rs by the media-uploads port).
//! WebMock stubs become wiremock + recorded bodies in `tests/data/`.

#[path = "common/client.rs"]
mod client;

use client::{bare_config, data, google_config, http};
use frankmd_media::search::{ImageSearch, SearchEndpoints, SearchError, search_flags};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn search(server: &MockServer) -> ImageSearch {
    ImageSearch::new(
        http(),
        SearchEndpoints {
            duckduckgo: server.uri(),
            google: server.uri(),
        },
    )
}

fn offline() -> ImageSearch {
    // Nothing listens here; validation must fail before any request.
    ImageSearch::new(
        http(),
        SearchEndpoints {
            duckduckgo: "http://127.0.0.1:9".into(),
            google: "http://127.0.0.1:9".into(),
        },
    )
}

async fn mount_ddg(server: &MockServer, page: &str, ijs: &str) {
    Mock::given(method("GET"))
        .and(path("/"))
        .and(query_param("iax", "images"))
        .and(query_param("ia", "images"))
        .respond_with(ResponseTemplate::new(200).set_body_string(page))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/i.js"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(ijs)
                .insert_header("content-type", "application/json"),
        )
        .mount(server)
        .await;
}

// port: test/controllers/images_controller_test.rb:31 "search_web returns error when query is blank"
#[tokio::test]
async fn search_web_blank_query() {
    assert_eq!(
        offline().search_web("").await.expect_err("400"),
        SearchError::QueryRequired
    );
}

// port: test/controllers/images_controller_test.rb:41 "search_google returns error when no API key"
#[tokio::test]
async fn search_google_without_key() {
    let err = offline()
        .search_google(&bare_config(), "test", 0)
        .await
        .expect_err("503");
    assert_eq!(
        err,
        SearchError::NotConfigured("errors.google_not_configured")
    );
    assert!(!search_flags(&bare_config()).google_enabled);
    assert!(search_flags(&google_config()).google_enabled);
}

// port: test/controllers/images_controller_test.rb:52 "search_google returns error when query is blank"
#[tokio::test]
async fn search_google_blank_query() {
    assert_eq!(
        offline()
            .search_google(&google_config(), "  ", 0)
            .await
            .expect_err("400"),
        SearchError::QueryRequired
    );
}

// port: test/controllers/images_controller_test.rb:62 "search_pinterest returns error when query is blank"
#[tokio::test]
async fn search_pinterest_blank_query() {
    assert_eq!(
        offline().search_pinterest("").await.expect_err("400"),
        SearchError::QueryRequired
    );
}

// port: test/controllers/images_controller_test.rb:496 "search_web returns images from DuckDuckGo"
#[tokio::test]
async fn search_web_returns_duckduckgo_images() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/i.js"))
        .and(query_param("q", "cats"))
        .and(query_param("vqd", "12345-67890"))
        .and(query_param("o", "json"))
        .and(query_param("l", "us-en"))
        .and(header("referer", "https://duckduckgo.com/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(data("ddg_i_js.json")))
        .expect(1)
        .mount(&server)
        .await;
    mount_ddg(&server, &data("ddg_vqd_page.html"), "{}").await;
    let r = search(&server).search_web("cats").await.expect("results");
    assert_eq!(r.images.len(), 1);
    assert_eq!(r.images[0].url, "https://example.com/cat.jpg");
    assert_eq!(r.images[0].title.as_deref(), Some("A cute cat"));
    assert_eq!(r.images[0].width, Some(800));
}

// port: test/controllers/images_controller_test.rb:529 "search_web handles missing vqd token gracefully"
#[tokio::test]
async fn search_web_missing_vqd_is_empty() {
    let server = MockServer::start().await;
    mount_ddg(&server, &data("ddg_no_vqd.html"), "{}").await;
    let r = search(&server).search_web("test").await.expect("200");
    assert!(r.images.is_empty());
    assert_eq!(
        r.note.as_deref(),
        Some("DuckDuckGo search temporarily unavailable")
    );
    let ijs = server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| r.url.path() == "/i.js")
        .count();
    assert_eq!(ijs, 0, "no i.js request without a vqd token");
}

// port: test/controllers/images_controller_test.rb:540 "search_google returns images from Google CSE API"
#[tokio::test]
async fn search_google_returns_cse_images() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .and(query_param("key", "test-google-key"))
        .and(query_param("cx", "test-cse-id"))
        .and(query_param("q", "landscape"))
        .and(query_param("searchType", "image"))
        .and(query_param("num", "10"))
        .and(query_param("start", "1"))
        .and(query_param("safe", "active"))
        .respond_with(ResponseTemplate::new(200).set_body_string(data("google_cse.json")))
        .expect(1)
        .mount(&server)
        .await;
    let r = search(&server)
        .search_google(&google_config(), "landscape", 0)
        .await
        .expect("results");
    assert_eq!(r.images.len(), 1);
    assert_eq!(r.images[0].url, "https://example.com/photo.jpg");
    assert_eq!(r.images[0].title.as_deref(), Some("Beautiful photo"));
    assert_eq!(
        r.images[0].thumbnail.as_deref(),
        Some("https://example.com/photo_thumb.jpg")
    );
    assert_eq!(r.total, Some(42));
    assert_eq!(r.next_start, Some(10));
}

// port: test/controllers/images_controller_test.rb:572 "search_google handles API error response"
#[tokio::test]
async fn search_google_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .respond_with(ResponseTemplate::new(403).set_body_string(
            r#"{"error":{"message":"quotaExceeded","errors":[{"reason":"dailyLimitExceeded"}]}}"#,
        ))
        .mount(&server)
        .await;
    let r = search(&server)
        .search_google(&google_config(), "test", 0)
        .await
        .expect("200");
    assert!(r.images.is_empty());
    assert!(
        r.error
            .as_deref()
            .is_some_and(|e| e.contains("quotaExceeded"))
    );
}

// port: test/controllers/images_controller_test.rb:587 "search_pinterest uses DuckDuckGo with site filter"
#[tokio::test]
async fn search_pinterest_uses_site_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .and(query_param("q", "home decor site:pinterest.com"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"vqd="99999-11111""#))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/i.js"))
        .and(query_param("q", "home decor site:pinterest.com"))
        .and(query_param("vqd", "99999-11111"))
        .respond_with(ResponseTemplate::new(200).set_body_string(data("ddg_i_js_pinterest.json")))
        .expect(1)
        .mount(&server)
        .await;
    let r = search(&server)
        .search_pinterest("home decor")
        .await
        .expect("results");
    assert_eq!(r.images.len(), 1);
    assert_eq!(r.images[0].title.as_deref(), Some("Pinterest Pin"));
}
