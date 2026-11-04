//! Port of `test/controllers/youtube_controller_test.rb` (10 tests; 1 P skipped:
//! the HTML partial's `data-action` attribute is Rails view plumbing).

#[path = "common/client.rs"]
mod client;

use client::{bare_config, data, http, youtube_config};
use frankmd_media::search::SearchError;
use frankmd_media::youtube::{YoutubeSearch, enabled};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn mock_api(status: u16, body: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/youtube/v3/search"))
        .respond_with(
            ResponseTemplate::new(status)
                .set_body_string(body)
                .insert_header("content-type", "application/json"),
        )
        .mount(&server)
        .await;
    server
}

// port: test/controllers/youtube_controller_test.rb:6 "config returns enabled false when no API key"
#[test]
fn config_disabled_without_key() {
    assert!(!enabled(&bare_config()));
}

// port: test/controllers/youtube_controller_test.rb:16 "config returns enabled true when API key is set"
#[test]
fn config_enabled_with_key() {
    assert!(enabled(&youtube_config("test_api_key")));
}

// port: test/controllers/youtube_controller_test.rb:33 "search returns error when no API key"
#[tokio::test]
async fn search_without_key_is_not_configured() {
    let err = YoutubeSearch::new(http())
        .search(&bare_config(), "test")
        .await
        .expect_err("503");
    assert_eq!(
        err,
        SearchError::NotConfigured("errors.youtube_not_configured")
    );
}

// port: test/controllers/youtube_controller_test.rb:43 "search returns error when query is blank"
#[tokio::test]
async fn search_blank_query_is_required() {
    let err = YoutubeSearch::new(http())
        .search(&youtube_config("k"), "")
        .await
        .expect_err("400");
    assert_eq!(err, SearchError::QueryRequired);
    let err = YoutubeSearch::new(http())
        .search(&youtube_config("k"), "   ")
        .await
        .expect_err("400");
    assert_eq!(err.i18n_key(), "errors.query_required");
}

// port: test/controllers/youtube_controller_test.rb:102 "search returns videos as JSON"
#[tokio::test]
async fn search_returns_videos() {
    let server = mock_api(200, &data("youtube_search.json")).await;
    let r = YoutubeSearch::with_base(http(), server.uri())
        .search(&youtube_config("test_youtube_key"), "rails tutorial")
        .await
        .expect("results");
    assert_eq!(r.videos.len(), 2);
    let v = &r.videos[0];
    assert_eq!(v.id.as_deref(), Some("abc123"));
    assert_eq!(v.title.as_deref(), Some("Test Video Title"));
    assert_eq!(v.channel.as_deref(), Some("Test Channel"));
    assert_eq!(
        v.thumbnail.as_deref(),
        Some("https://i.ytimg.com/vi/abc123/mqdefault.jpg")
    );
}

// port: test/controllers/youtube_controller_test.rb:116 "search falls back to default thumbnail when medium is missing"
#[tokio::test]
async fn search_falls_back_to_default_thumbnail() {
    let server = mock_api(200, &data("youtube_search.json")).await;
    let r = YoutubeSearch::with_base(http(), server.uri())
        .search(&youtube_config("k"), "test")
        .await
        .expect("results");
    assert_eq!(
        r.videos[1].thumbnail.as_deref(),
        Some("https://i.ytimg.com/vi/def456/default.jpg")
    );
}

// skipped (P): test/controllers/youtube_controller_test.rb:125 "HTML search results use the thumbnail error action without inline handlers"

// port: test/controllers/youtube_controller_test.rb:133 "search passes query to YouTube API"
#[tokio::test]
async fn search_passes_query_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/youtube/v3/search"))
        .and(query_param("q", "ruby on rails"))
        .and(query_param("type", "video"))
        .and(query_param("maxResults", "6"))
        .and(query_param("part", "snippet"))
        .and(query_param("key", "test_youtube_key"))
        .respond_with(ResponseTemplate::new(200).set_body_string(data("youtube_search.json")))
        .expect(1)
        .mount(&server)
        .await;
    YoutubeSearch::with_base(http(), server.uri())
        .search(&youtube_config("test_youtube_key"), "ruby on rails")
        .await
        .expect("results");
}

// port: test/controllers/youtube_controller_test.rb:141 "search handles YouTube API error"
#[tokio::test]
async fn search_api_error_is_empty_with_error() {
    let server = mock_api(403, r#"{"error": {"message": "quotaExceeded"}}"#).await;
    let r = YoutubeSearch::with_base(http(), server.uri())
        .search(&youtube_config("k"), "test")
        .await
        .expect("200");
    assert!(r.videos.is_empty());
    assert!(r.error.as_deref().is_some_and(|e| !e.is_empty()));
}

// port: test/controllers/youtube_controller_test.rb:153 "search handles YouTube API returning empty items"
#[tokio::test]
async fn search_empty_items() {
    let server = mock_api(200, r#"{"items": []}"#).await;
    let r = YoutubeSearch::with_base(http(), server.uri())
        .search(&youtube_config("k"), "obscure query")
        .await
        .expect("200");
    assert!(r.videos.is_empty());
}
