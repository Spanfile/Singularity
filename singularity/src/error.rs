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
    #[error("Invalid output: empty destination path")]
    EmptyDestination,
    #[error("Invalid output: metric name is empty while metric is enabled")]
    EmptyMetricName,

    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    HTTP(#[from] ureq::Error),
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),
}
