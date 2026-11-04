//! `EgressClient::fetch` contract. The original exercised it through
//! `ImagesService.fetch_external_image` (test/services/images_service_test.rb);
//! media-level ports of those tests live in frankmd-media. Here the same
//! guarantees are asserted directly on the client: wiremock plays the remote
//! host, [`StaticResolver`] plays DNS (public answers), and
//! `with_dial_override` routes the vetted connection to the mock.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use frankmd_net::{EgressClient, Limits, NetError, StaticResolver};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn public() -> IpAddr {
    "93.184.216.34".parse().expect("ip")
}

fn client(server: &MockServer, resolver: StaticResolver, limits: Limits) -> EgressClient {
    EgressClient::with_resolver(limits, Arc::new(resolver))
        .expect("client")
        .with_dial_override(*server.address())
}

fn public_resolver() -> StaticResolver {
    StaticResolver::new()
        .with("example.com", &[public()])
        .with("cdn.example.com", &[public()])
}

// derived from: test/services/images_service_test.rb:697 "download_and_upload_to_s3 downloads image and uploads to S3"
#[tokio::test]
async fn fetches_body_and_content_type_with_frankmd_user_agent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/photo.jpg"))
        .and(header(
            "user-agent",
            "Mozilla/5.0 (compatible; FrankMD/1.0)",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes("fake image content")
                .insert_header("content-type", "image/jpeg"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let got = client(&server, public_resolver(), Limits::EXTERNAL_IMAGE)
        .fetch("http://example.com/photo.jpg")
        .await
        .expect("fetch");
    assert_eq!(got.bytes, b"fake image content");
    assert_eq!(got.content_type.as_deref(), Some("image/jpeg"));
    assert_eq!(got.url.as_str(), "http://example.com/photo.jpg");
}

// derived from: test/services/images_service_test.rb:733 "download_and_upload_to_s3 returns nil on download failure"
#[tokio::test]
async fn non_success_status_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/missing.jpg"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;
    let err = client(&server, public_resolver(), Limits::EXTERNAL_IMAGE)
        .fetch("http://example.com/missing.jpg")
        .await
        .expect_err("should fail");
    assert!(matches!(err, NetError::Status(404)), "{err:?}");
}

// derived from: test/services/images_service_test.rb:742 "download_and_upload_to_s3 rejects a body over the size cap (Content-Length)"
#[tokio::test]
async fn content_length_over_cap_is_refused_before_reading() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/huge.jpg"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(vec![b'x'; 2048])
                .insert_header("content-type", "image/jpeg"),
        )
        .mount(&server)
        .await;
    let limits = Limits {
        max_bytes: 1024,
        ..Limits::EXTERNAL_IMAGE
    };
    let err = client(&server, public_resolver(), limits)
        .fetch("http://example.com/huge.jpg")
        .await
        .expect_err("should fail");
    assert!(matches!(err, NetError::TooLarge), "{err:?}");
}

// derived from: test/services/images_service_test.rb:756 "download_and_upload_to_s3 follows a redirect, re-validating the target"
#[tokio::test]
async fn follows_redirects_revalidating_each_hop() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/go.jpg"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "http://cdn.example.com/real.jpg"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/real.jpg"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes("fake image content")
                .insert_header("content-type", "image/jpeg"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let resolver = Arc::new(public_resolver());
    let c = EgressClient::with_resolver(Limits::EXTERNAL_IMAGE, resolver.clone())
        .expect("client")
        .with_dial_override(*server.address());
    let got = c.fetch("http://example.com/go.jpg").await.expect("fetch");
    assert_eq!(got.url.as_str(), "http://cdn.example.com/real.jpg");
    assert_eq!(
        resolver.calls(),
        vec!["example.com".to_owned(), "cdn.example.com".to_owned()]
    );
}

// derived from: test/controllers/image_import_test.rb "follows redirects" (302 with a relative Location)
#[tokio::test]
async fn relative_redirect_location_is_joined_to_the_current_url() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/a/go"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "../final.png"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/final.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes("png"))
        .expect(1)
        .mount(&server)
        .await;
    let got = client(&server, public_resolver(), Limits::EXTERNAL_IMAGE)
        .fetch("http://example.com/a/go")
        .await
        .expect("fetch");
    assert_eq!(got.url.as_str(), "http://example.com/final.png");
}

// derived from: test/services/images_service_test.rb:769 "download_and_upload_to_s3 blocks a redirect to a non-public address (SSRF via redirect)"
#[tokio::test]
async fn redirect_to_metadata_address_is_blocked_and_never_requested() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rebind.jpg"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", "http://169.254.169.254/latest/meta-data/"),
        )
        .mount(&server)
        .await;
    Mock::given(path("/latest/meta-data/"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let resolver =
        public_resolver().with("169.254.169.254", &["169.254.169.254".parse().expect("ip")]);
    let err = client(&server, resolver, Limits::EXTERNAL_IMAGE)
        .fetch("http://example.com/rebind.jpg")
        .await
        .expect_err("should fail");
    assert!(
        matches!(err, NetError::Blocked(ref m) if m.contains("non-public")),
        "{err:?}"
    );
}

// derived from: test/services/images_service_test.rb:784 "download_and_upload_to_s3 refuses a host resolving to a non-public address (SSRF)"
#[tokio::test]
async fn host_resolving_to_private_address_is_never_contacted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let resolver =
        StaticResolver::new().with("internal.example.com", &["10.0.0.5".parse().expect("ip")]);
    let err = client(&server, resolver, Limits::EXTERNAL_IMAGE)
        .fetch("http://internal.example.com/x.png")
        .await
        .expect_err("should fail");
    assert!(matches!(err, NetError::Blocked(_)), "{err:?}");
}

// derived from: test/services/images_service_test.rb:793 "download_and_upload_to_s3 refuses a non-http scheme (SSRF)"
#[tokio::test]
async fn non_http_scheme_is_refused() {
    let c = EgressClient::with_resolver(Limits::EXTERNAL_IMAGE, Arc::new(StaticResolver::new()))
        .expect("client");
    let err = c
        .fetch("file:///etc/passwd")
        .await
        .expect_err("should fail");
    assert!(matches!(err, NetError::Blocked(_)), "{err:?}");
}

// derived from: images_service.rb MAX_EXTERNAL_IMAGE_REDIRECTS = 5
#[tokio::test]
async fn more_than_max_redirects_fails() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/loop"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/loop"))
        .mount(&server)
        .await;
    let err = client(&server, public_resolver(), Limits::EXTERNAL_IMAGE)
        .fetch("http://example.com/loop")
        .await
        .expect_err("should fail");
    assert!(matches!(err, NetError::TooManyRedirects), "{err:?}");
    // 1 initial request + MAX redirects followed.
    assert_eq!(server.received_requests().await.map(|r| r.len()), Some(6));
}

// derived from: test/services/images_service_test.rb:691 "fetch_external_image stops when its overall deadline expires"
#[tokio::test]
async fn overall_deadline_is_enforced() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow.jpg"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes("x")
                .set_delay(Duration::from_millis(500)),
        )
        .mount(&server)
        .await;
    let limits = Limits {
        timeout: Duration::from_millis(50),
        ..Limits::EXTERNAL_IMAGE
    };
    let err = client(&server, public_resolver(), limits)
        .fetch("http://example.com/slow.jpg")
        .await
        .expect_err("should fail");
    assert!(matches!(err, NetError::Timeout), "{err:?}");
}
