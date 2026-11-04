//! User-visible text as data: an i18n key with `%{var}` arguments, or a
//! message shown verbatim (service rejections such as
//! `".html is not an accepted image type. ..."`).

/// A string for the view to resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Text {
    /// Locale key plus interpolation arguments (themselves texts, so an
    /// argument can be another key, e.g. `no_file_extension`).
    Key(&'static str, Vec<(&'static str, Text)>),
    /// Shown as is.
    Raw(String),
}

impl Text {
    /// A key without arguments.
    pub fn key(key: &'static str) -> Self {
        Self::Key(key, Vec::new())
    }

    /// A key with arguments.
    pub fn with(key: &'static str, args: &[(&'static str, &str)]) -> Self {
        Self::Key(
            key,
            args.iter()
                .map(|(k, v)| (*k, Self::Raw((*v).to_owned())))
                .collect(),
        )
    }

    /// The key, when this is one.
    pub fn as_key(&self) -> Option<&'static str> {
        match self {
            Self::Key(key, _) => Some(key),
            Self::Raw(_) => None,
        }
    }

    /// An `ApiMessage` from the media endpoints.
    pub fn from_api(message: &crate::api::ApiMessage) -> Self {
        match message {
            crate::api::ApiMessage::Key(key) => Self::key(key),
            crate::api::ApiMessage::Text(text) => Self::Raw(text.clone()),
        }
    }
}
