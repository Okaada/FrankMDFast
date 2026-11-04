//! Media errors. Messages are the original's user-visible strings verbatim
//! (they are asserted by tests and golden fixtures); the UI maps them to toasts.

use crate::s3::S3Error;

/// Upload / library errors.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// `UploadStorage::RejectedError` (provider conflicts, unknown provider,
    /// "S3 is not configured").
    #[error("{0}")]
    Rejected(String),
    /// Validation failure returned as `{error: msg}` (bad extension, too large,
    /// invalid base64, no file, ...). HTTP 400/422 in the original.
    #[error("{0}")]
    Invalid(String),
    /// Requested media does not exist / is not a contained, supported file (404).
    #[error("not found")]
    NotFound,
    /// Feature not configured (`images_path`, Google CSE, YouTube key) (404/503).
    #[error("{0}")]
    NotConfigured(String),
    /// **[COMPAT]** `MediaStorageRecovery`: S3 was selected but unusable, `.fed`
    /// now says `local`; the caller reports `media_storage_fallback: true`.
    #[error(
        "S3 is no longer available. The upload provider is now Local. Please retry the upload."
    )]
    StorageFallback,
    /// S3 call failed (provider detail never shown to the user).
    #[error("storage request failed")]
    S3(#[from] S3Error),
    /// Generic upstream failure with the original's generic message
    /// (e.g. "Video upload failed"); never carries provider detail.
    #[error("{0}")]
    Upstream(String),
    /// `MediaUsageService::ScanError` ("Could not read Markdown note ...").
    #[error("{0}")]
    Scan(String),
    /// Filesystem permission / IO failure that the original re-raised (EACCES).
    #[error("io: {0:?}")]
    Io(std::io::ErrorKind),
    /// Filesystem error.
    #[error(transparent)]
    Fs(#[from] frankmd_fs::FsError),
    /// Network error.
    #[error(transparent)]
    Net(#[from] frankmd_net::NetError),
}
