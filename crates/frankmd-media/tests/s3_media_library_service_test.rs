//! Port of `test/services/s3_media_library_service_test.rb` (5 tests). The mocked
//! `Aws::S3::Client` becomes a wiremock S3-compatible endpoint (path style:
//! `/<bucket>/<key>`); object URLs keep the AWS virtual-host form.

#[path = "common/library.rs"]
mod library;

use frankmd_media::s3_library::S3MediaLibrary;
use library::{Notes, env, list_objects_xml, s3_config};
use wiremock::matchers::{header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn service(server: &MockServer) -> (Notes, S3MediaLibrary) {
    let n = Notes::new();
    let cfg = s3_config(n.root());
    let e = env(cfg.clone(), Some(&server.uri()));
    let lib = S3MediaLibrary::new(&e, &cfg).expect("s3 library");
    (n, lib)
}

// port: test/services/s3_media_library_service_test.rb:20 "lists only supported objects below the FrankMD prefix across pages"
#[tokio::test]
async fn lists_supported_objects_below_prefix_across_pages() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/bucket"))
        .and(query_param("list-type", "2"))
        .and(query_param("prefix", "frankmd/"))
        .and(query_param_is_missing("continuation-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_objects_xml(
            "bucket",
            &[("frankmd/2026/photo.png", 12), ("foreign/photo.png", 9), ("frankmd/readme.txt", 5)],
            Some("next"),
        )))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/bucket"))
        .and(query_param("prefix", "frankmd/"))
        .and(query_param("continuation-token", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_objects_xml("bucket", &[("frankmd/2026/clip.mp4", 20)], None)))
        .expect(1)
        .mount(&server)
        .await;
    let (_n, lib) = service(&server).await;
    let result = lib.list().await.expect("list");
    assert_eq!(result.images.iter().map(|i| i.path.as_str()).collect::<Vec<_>>(), vec!["frankmd/2026/photo.png"]);
    assert_eq!(result.videos.iter().map(|i| i.path.as_str()).collect::<Vec<_>>(), vec!["frankmd/2026/clip.mp4"]);
    // provider "s3" ⇔ s3_url present
    let url = result.images[0].s3_url.as_deref().expect("provider s3 item has url");
    assert_eq!(url, "https://bucket.s3.us-east-1.amazonaws.com/frankmd/2026/photo.png");
}

// port: test/services/s3_media_library_service_test.rb:37 "delete rejects foreign and unsupported keys without calling S3"
#[tokio::test]
async fn delete_rejects_foreign_and_unsupported_keys_without_calling_s3() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE")).respond_with(ResponseTemplate::new(204)).expect(0).mount(&server).await;
    let (_n, lib) = service(&server).await;
    assert!(!lib.delete("foreign/photo.png").await.expect("no error"));
    assert!(!lib.delete("frankmd/../../photo.png").await.expect("no error"));
    assert!(!lib.delete("frankmd/readme.txt").await.expect("no error"));
    assert_eq!(server.received_requests().await.map(|r| r.len()), Some(0));
}

// port: test/services/s3_media_library_service_test.rb:45 "delete removes an exact managed key"
#[tokio::test]
async fn delete_removes_an_exact_managed_key() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/bucket/frankmd/2026/photo.png"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let (_n, lib) = service(&server).await;
    assert!(lib.delete("frankmd/2026/photo.png").await.expect("delete"));
}

// port: test/services/s3_media_library_service_test.rb:51 "rename copies to a free sibling key before deleting the source"
#[tokio::test]
async fn rename_copies_to_free_sibling_then_deletes_source() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/bucket/frankmd/2026/renamed.png"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/bucket/frankmd/2026/renamed.png"))
        .and(header("x-amz-copy-source", "bucket/frankmd/2026/photo.png"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<CopyObjectResult><ETag>\"x\"</ETag><LastModified>2026-10-16T00:00:00.000Z</LastModified></CopyObjectResult>",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/bucket/frankmd/2026/photo.png"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let (_n, lib) = service(&server).await;
    let renamed = lib.rename("frankmd/2026/photo.png", "renamed.png").await.expect("rename").expect("renamed");
    assert_eq!(renamed.path, "frankmd/2026/renamed.png");
    assert_eq!(renamed.name, "renamed.png");
    // in sequence: HEAD, PUT (copy), DELETE
    let order: Vec<String> =
        server.received_requests().await.unwrap_or_default().iter().map(|r| r.method.to_string()).collect();
    assert_eq!(order, vec!["HEAD", "PUT", "DELETE"]);
}

// port: test/services/s3_media_library_service_test.rb:66 "rename rejects traversal, unsupported types, and collisions without mutation"
#[tokio::test]
async fn rename_rejects_traversal_unsupported_and_collisions_without_mutation() {
    let server = MockServer::start().await;
    Mock::given(method("PUT")).respond_with(ResponseTemplate::new(200)).expect(0).mount(&server).await;
    Mock::given(method("DELETE")).respond_with(ResponseTemplate::new(204)).expect(0).mount(&server).await;
    Mock::given(method("HEAD"))
        .and(path("/bucket/frankmd/2026/existing.png"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let (_n, lib) = service(&server).await;
    assert_eq!(lib.rename("frankmd/2026/photo.png", "../outside.png").await.expect("ok"), None);
    assert_eq!(lib.rename("frankmd/2026/photo.png", "readme.txt").await.expect("ok"), None);
    let long = format!("{}.png", "a".repeat(1_100));
    assert_eq!(lib.rename("frankmd/2026/photo.png", &long).await.expect("ok"), None);
    assert_eq!(lib.rename("frankmd/2026/photo.png", "existing.png").await.expect("ok"), None);
}
