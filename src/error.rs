use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum SingularityError {
    #[error("The HTTP request failed with status code {0}. Body: {1}")]
    RequestFailed(u16, String),
    #[error("Unsupported URL scheme: {0}")]
    UnsupportedUrlScheme(String),
    #[error("Invalid file path: {0}")]
    InvalidFilePath(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
