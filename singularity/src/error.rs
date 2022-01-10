use thiserror::Error;

/// The result type returned from the library.
pub type Result<T> = std::result::Result<T, SingularityError>;

/// The error type returned from the library.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SingularityError {
    /// An HTTP request failed.
    #[error("The HTTP request failed with status code {0}. Body: {1}")]
    RequestFailed(
        /// The response's status code.
        u16,
        /// The response's body.
        String,
    ),
    /// An HTTP response was invalid.
    #[error("The HTTP response was invalid: {0}")]
    InvalidResponse(
        /// Reason why the response is invalid.
        String,
    ),
    /// The source URL In an adlist is using an unsupported URL scheme.
    #[error("Unsupported URL scheme: {0}")]
    UnsupportedUrlScheme(
        /// The invalid scheme from the URL.
        String,
    ),
    /// The path in the `file://` source URL for an adlist is invalid.
    #[error("Invalid file path: {0}")]
    InvalidFilePath(
        /// The file path from the URL.
        String,
    ),
    /// The destination path for an [`Output`](crate::Output) is empty.
    #[error("Invalid output: empty destination path")]
    EmptyDestination,
    /// In a PDNS Lua script output, the metric name is empty while the metric is enabled.
    #[error("Invalid output: PDNS Lua Script metric name is empty while metric is enabled")]
    EmptyMetricName,
    /// An IP address was invalid.
    #[error("Invalid IP address: {0}")]
    InvalidIpAddress(#[from] std::net::AddrParseError),
    /// No adlists were configured when building a new Singularity
    #[error("No adlists were configured when building a new Singularity")]
    NoAdlists,
    /// No outputs were configured when building a new Singularity
    #[error("No outputs were configured when building a new Singularity")]
    NoOutputs,
    /// One or more of the runtime threads panicked.
    #[error("One or more of the runtime threads panicked")]
    Panicked,

    /// Transparent wrapper for an [IO error](std::io::Error).
    #[error(transparent)]
    IO(#[from] std::io::Error),
    /// Transparent wrapper for an [`ureq` error](ureq::Error).
    #[error(transparent)]
    HTTP(#[from] ureq::Error),
    /// Transparent wrapper for an [URL parsing error](url::ParseError).
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),
}
