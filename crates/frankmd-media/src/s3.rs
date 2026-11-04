//! S3 client: `aws-sdk-s3` (SigV4, the same semantics as the Ruby SDK) whose
//! HTTP traffic goes through `frankmd_net::TrustedClient` (one client stack,
//! no proxies; see [`crate::s3_http`]). Virtual-host style on AWS;
//! `endpoint` + `force_path_style` for MinIO & co. Tests point the transport
//! at a wiremock server with path style.

use std::time::Duration;

use aws_sdk_s3::config::{
    BehaviorVersion, Credentials, Region, RequestChecksumCalculation, ResponseChecksumValidation,
    retry::RetryConfig,
};
use aws_sdk_s3::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_s3::primitives::ByteStream;

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
    /// `IsTruncated` (a truncated page without a token is a pagination failure).
    pub truncated: bool,
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
    region: String,
}

impl S3Client {
    /// **[COMPAT]** `UploadStorage.s3_client(cfg)`: region defaults to
    /// [`DEFAULT_S3_REGION`]; `endpoint` and `force_path_style: true` only when configured.
    /// A region that is not a valid host label is [`S3Error::InvalidRegion`]
    /// (`Aws::Errors::InvalidRegionError` was raised at construction too).
    pub fn new(settings: &S3Settings, http: &frankmd_net::TrustedClient) -> Result<Self, S3Error> {
        let region = settings
            .region
            .as_deref()
            .filter(|r| !r.trim().is_empty())
            .unwrap_or(DEFAULT_S3_REGION)
            .to_owned();
        if !valid_region(&region) {
            return Err(S3Error::InvalidRegion);
        }
        Ok(Self {
            settings: settings.clone(),
            http: http.clone(),
            transport: None,
            region,
        })
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
        &self.region
    }

    /// Custom endpoint, if any.
    pub fn endpoint(&self) -> Option<&str> {
        self.settings
            .endpoint
            .as_deref()
            .filter(|e| !e.trim().is_empty())
    }

    /// Whether path-style addressing is used.
    pub fn force_path_style(&self) -> bool {
        self.endpoint().is_some() && self.settings.force_path_style
    }

    /// The configured bucket (`""` when unset; the provider gate prevents that).
    pub fn bucket(&self) -> &str {
        self.settings.bucket.as_deref().unwrap_or_default()
    }

    /// `PutObject` with `If-None-Match: *` (never overwrites) and `Content-Type`.
    pub async fn put_object(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<(), S3Error> {
        self.sdk()
            .put_object()
            .bucket(self.bucket())
            .key(key)
            .body(ByteStream::from(bytes))
            .content_type(content_type)
            .if_none_match("*")
            .send()
            .await
            .map(|_| ())
            .map_err(classify)
    }

    /// `ListObjectsV2(prefix, continuation_token)`.
    pub async fn list_objects_v2(
        &self,
        prefix: &str,
        continuation_token: Option<&str>,
    ) -> Result<ListPage, S3Error> {
        let out = self
            .sdk()
            .list_objects_v2()
            .bucket(self.bucket())
            .prefix(prefix)
            .set_continuation_token(continuation_token.map(str::to_owned))
            .send()
            .await
            .map_err(classify)?;
        let objects = out
            .contents()
            .iter()
            .filter_map(|o| {
                Some(S3Object {
                    key: o.key()?.to_owned(),
                    size: o.size().and_then(|s| u64::try_from(s).ok()).unwrap_or(0),
                    last_modified: o
                        .last_modified()
                        .and_then(|t| t.fmt(aws_smithy_types::date_time::Format::DateTime).ok()),
                })
            })
            .collect();
        let next = if out.is_truncated() == Some(true) {
            out.next_continuation_token().map(str::to_owned)
        } else {
            None
        };
        Ok(ListPage {
            objects,
            next_continuation_token: next,
            truncated: out.is_truncated() == Some(true),
        })
    }

    /// `HeadObject`: `Ok(true)` exists, `Ok(false)` on NotFound.
    pub async fn head_object(&self, key: &str) -> Result<bool, S3Error> {
        match self
            .sdk()
            .head_object()
            .bucket(self.bucket())
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => match classify(e) {
                S3Error::Service { status: 404, .. } => Ok(false),
                S3Error::Service { code, .. } if code == "NotFound" || code == "NoSuchKey" => {
                    Ok(false)
                }
                other => Err(other),
            },
        }
    }

    /// `CopyObject(copy_source: "<bucket>/<url-encoded from>", key: to)`.
    pub async fn copy_object(&self, from: &str, to: &str) -> Result<(), S3Error> {
        let encoded = from
            .split('/')
            .map(crate::upload_storage::erb_url_encode)
            .collect::<Vec<_>>()
            .join("/");
        self.sdk()
            .copy_object()
            .bucket(self.bucket())
            .copy_source(format!("{}/{encoded}", self.bucket()))
            .key(to)
            .send()
            .await
            .map(|_| ())
            .map_err(classify)
    }

    /// `DeleteObject`.
    pub async fn delete_object(&self, key: &str) -> Result<(), S3Error> {
        self.sdk()
            .delete_object()
            .bucket(self.bucket())
            .key(key)
            .send()
            .await
            .map(|_| ())
            .map_err(classify)
    }

    /// Build the SDK client for one call (cheap: the HTTP pool lives in
    /// `TrustedClient`). Checksums are only sent when an operation requires
    /// them, so bodies go out as plain payloads like the original's.
    fn sdk(&self) -> aws_sdk_s3::Client {
        let credentials = Credentials::new(
            self.settings.access_key_id.clone().unwrap_or_default(),
            self.settings.secret_access_key.clone().unwrap_or_default(),
            None,
            None,
            "frankmd-config",
        );
        let mut builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(self.region.clone()))
            .credentials_provider(credentials)
            .http_client(crate::s3_http::TrustedHttpClient::new(self.http.clone()))
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            .retry_config(
                RetryConfig::standard()
                    .with_max_attempts(3)
                    .with_initial_backoff(Duration::from_millis(100)),
            );
        if let Some(base) = &self.transport {
            builder = builder
                .endpoint_url(base.as_str().trim_end_matches('/'))
                .force_path_style(true);
        } else if let Some(endpoint) = self.endpoint() {
            builder = builder
                .endpoint_url(endpoint)
                .force_path_style(self.settings.force_path_style);
        }
        aws_sdk_s3::Client::from_conf(builder.build())
    }
}

/// `Aws::Errors::InvalidRegionError`: the region must be a DNS label.
fn valid_region(region: &str) -> bool {
    !region.is_empty()
        && region.len() <= 63
        && region
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !region.starts_with('-')
        && !region.ends_with('-')
}

/// Map an SDK failure onto the classes the original's rescue clauses used.
/// Provider messages are dropped (they may echo request details).
fn classify<E: ProvideErrorMetadata, R: HttpStatus>(error: SdkError<E, R>) -> S3Error {
    match &error {
        SdkError::ServiceError(ctx) => S3Error::Service {
            code: ctx
                .err()
                .code()
                .map_or_else(|| status_code_name(ctx.raw().status_u16()), str::to_owned),
            status: ctx.raw().status_u16(),
        },
        SdkError::ResponseError(ctx) => S3Error::Service {
            code: status_code_name(ctx.raw().status_u16()),
            status: ctx.raw().status_u16(),
        },
        SdkError::TimeoutError(_) => S3Error::Network("timeout".to_owned()),
        SdkError::DispatchFailure(_) => S3Error::Network("networking".to_owned()),
        _ => S3Error::Network("request".to_owned()),
    }
}

/// HTTP status of a raw SDK response.
trait HttpStatus {
    fn status_u16(&self) -> u16;
}

impl HttpStatus for aws_smithy_runtime_api::http::Response {
    fn status_u16(&self) -> u16 {
        self.status().as_u16()
    }
}

/// Error code the SDK infers for body-less errors (`HEAD` 404 → `NotFound`).
fn status_code_name(status: u16) -> String {
    match status {
        404 => "NotFound",
        403 => "Forbidden",
        412 => "PreconditionFailed",
        _ => "Unknown",
    }
    .to_owned()
}
