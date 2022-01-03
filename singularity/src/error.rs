use thiserror::Error;

pub type Result<T> = std::result::Result<T, SingularityError>;

#[derive(Debug, Error)]
pub enum SingularityError {
    #[error("The HTTP request failed with status code {0}. Body: {1}")]
    RequestFailed(u16, String),
    #[error("The HTTP response was invalid: {0}")]
    InvalidResponse(String),
    #[error("Unsupported URL scheme: {0}")]
    UnsupportedUrlScheme(String),
    #[error("Invalid file path: {0}")]
    InvalidFilePath(String),

    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    HTTP(#[from] ureq::Error),
}
