use serde::ser::SerializeStruct;

/// Error type shared by the whole engine. It serialises to `{ kind, message }` for the UI.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Msg(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Http(String),
    #[error("Invalid data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Archive error: {0}")]
    Zip(String),
    #[error("Cancelled")]
    Cancelled,
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Incompatible(String),
    #[error("{0}")]
    Auth(String),
    #[error("Unsafe path rejected: {0}")]
    UnsafePath(String),
    #[error("Hash mismatch for {file}: expected {expected}, got {actual}")]
    HashMismatch { file: String, expected: String, actual: String },
    #[error("{0}")]
    MissingConfig(String),
}

impl Error {
    pub fn msg(s: impl Into<String>) -> Self {
        Error::Msg(s.into())
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Error::Msg(_) => "error",
            Error::Io(_) => "io",
            Error::Http(_) => "network",
            Error::Json(_) => "data",
            Error::Zip(_) => "archive",
            Error::Cancelled => "cancelled",
            Error::NotFound(_) => "not_found",
            Error::Invalid(_) => "invalid",
            Error::Incompatible(_) => "incompatible",
            Error::Auth(_) => "auth",
            Error::UnsafePath(_) => "unsafe_path",
            Error::HashMismatch { .. } => "hash_mismatch",
            Error::MissingConfig(_) => "missing_config",
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        // reqwest errors can contain the full URL; URLs we build never contain secrets
        // (keys travel in headers), so the message is safe to show.
        Error::Http(e.to_string())
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error::Zip(e.to_string())
    }
}

impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Error", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &crate::redact::redact(&self.to_string()))?;
        st.end()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
