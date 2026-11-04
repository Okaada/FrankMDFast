//! Domain error type.

/// Errors produced by pure domain operations (validation, parsing).
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// The path is not a valid vault-relative path (absolute, `..` escaping the
    /// root, NUL byte, empty segment, ...).
    #[error("invalid path: {0}")]
    InvalidPath(String),
    /// Front matter could not be parsed.
    #[error("invalid front matter: {0}")]
    InvalidFrontMatter(String),
}
