//! Minimal S3 client over `frankmd_net::TrustedClient` (SigV4). Virtual-host
//! style on AWS; `endpoint` + `force_path_style` for MinIO & co.
//! Tests point `endpoint` at a wiremock server with path style.

use crate::config::S3Settings;

/// **[COMPAT]** `DEFAULT_S3_REGION`.
pub const DEFAULT_S3_REGION: &str = "us-east-1";

/// An S3 failure, classified the way the original's rescue clauses need it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum S3Error {
    /// S3 returned an error document: `<Code>` + HTTP status.
    #[error("s3 service error {code} ({status})")]
    Service {
        /// Error code (`AccessDenied`, `PreconditionFailed`, `SlowDown`, ...).
        code: String,
        /// HTTP status.
        status: u16,
    },
    /// `Aws::Errors::InvalidRegionError` (region not a valid identifier).
    #[error("invalid region")]
    InvalidRegion,
    /// Transport / timeout (`Seahorse::Client::NetworkingError`, `Timeout::Error`).
    #[error("network: {0}")]
    Network(String),
}

/// One `ListObjectsV2` page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListPage {
    /// `(key, size, last_modified ISO-8601)` per object.
    pub objects: Vec<S3Object>,
    /// `NextContinuationToken` when truncated.
    pub next_continuation_token: Option<String>,
}

/// One listed object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3Object {
    /// Key.
    pub key: String,
    /// Size in bytes.
    pub size: u64,
    /// `LastModified` as sent by S3.
    pub last_modified: Option<String>,
}

/// S3 client for one bucket.
#[derive(Debug, Clone)]
pub struct S3Client {
    settings: S3Settings,
    http: frankmd_net::TrustedClient,
    transport: Option<url::Url>,
}

impl S3Client {
    /// **[COMPAT]** `UploadStorage.s3_client(cfg)`: region defaults to
    /// [`DEFAULT_S3_REGION`]; `endpoint` and `force_path_style: true` only when configured.
    pub fn new(settings: &S3Settings, http: &frankmd_net::TrustedClient) -> Result<Self, S3Error> {
        todo!()
    }

    /// **Test seam**: send requests to `base` (path style: `<base>/<bucket>/<key>`)
    /// instead of the configured/AWS endpoint. Signing and public-URL
    /// formatting are unchanged.
    pub fn with_transport_base(mut self, base: url::Url) -> Self {
        self.transport = Some(base);
        self
    }

    /// Effective region.
    pub fn region(&self) -> &str {
        todo!()
    }

    /// Custom endpoint, if any.
    pub fn endpoint(&self) -> Option<&str> {
        todo!()
    }

    /// Whether path-style addressing is used.
    pub fn force_path_style(&self) -> bool {
        todo!()
    }

    /// `PutObject` with `If-None-Match: *` (never overwrites) and `Content-Type`.
    pub async fn put_object(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<(), S3Error> {
        todo!()
    }

    /// `ListObjectsV2(prefix, continuation_token)`.
    pub async fn list_objects_v2(&self, prefix: &str, continuation_token: Option<&str>) -> Result<ListPage, S3Error> {
        todo!()
    }

    /// `HeadObject`: `Ok(true)` exists, `Ok(false)` on NotFound.
    pub async fn head_object(&self, key: &str) -> Result<bool, S3Error> {
        todo!()
    }

    /// `CopyObject(copy_source: "<bucket>/<from>", key: to)`.
    pub async fn copy_object(&self, from: &str, to: &str) -> Result<(), S3Error> {
        todo!()
    }

    /// `DeleteObject`.
    pub async fn delete_object(&self, key: &str) -> Result<(), S3Error> {
        todo!()
    }
}
