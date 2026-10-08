use crate::i18n::{Language, tr};

/// Stable, credential-free errors. Do not forward server bodies or URLs to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("file operation failed")]
    File,
    #[error("invalid image")]
    Image,
    #[error("image exceeds decode limit")]
    ImageLimit,
    #[error("network request failed")]
    Network,
    #[error("invalid API key")]
    InvalidKey,
    #[error("quota exhausted")]
    Quota,
    #[error("Tinify service unavailable")]
    Service,
    #[error("unsupported image")]
    Unsupported,
    #[error("secure storage unavailable")]
    SecureStorage,
    #[error("configuration unreadable")]
    Configuration,
    #[error("legacy configuration requires re-entry")]
    Legacy,
    #[error("invalid key format")]
    KeyFormat,
    #[error("duplicate key")]
    KeyDuplicate,
    #[error("too many keys")]
    KeyLimit,
    #[error("note too long")]
    NoteLimit,
    #[error("unsafe output path")]
    UnsafePath,
    #[error("original changed during processing")]
    OriginalChanged,
}
impl Error {
    pub fn text(self, language: Language) -> &'static str {
        tr(
            language,
            match self {
                Self::File => "error.file",
                Self::Image => "error.image",
                Self::ImageLimit => "error.image_limit",
                Self::Network => "error.network",
                Self::InvalidKey => "error.invalid_key",
                Self::Quota => "error.quota",
                Self::Service => "error.service",
                Self::Unsupported => "error.unsupported",
                Self::SecureStorage => "error.storage",
                Self::Configuration => "error.config",
                Self::Legacy => "error.legacy",
                Self::KeyFormat => "error.key_format",
                Self::KeyDuplicate => "error.duplicate",
                Self::KeyLimit => "error.key_limit",
                Self::NoteLimit => "error.note_limit",
                Self::UnsafePath => "error.path",
                Self::OriginalChanged => "error.changed",
            },
        )
    }
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::File
    }
}
