use thiserror::Error;

pub type EvhResult<T> = std::result::Result<T, EvhError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EvhError {
    // direct errors
    #[error("Failed to initialise database pool: {0}")]
    DatabasePoolInitialisationFailed(diesel::r2d2::PoolError),
    #[error("Failed to acquire database connection: {0}")]
    DatabaseConnectionAcquireFailed(diesel::r2d2::PoolError),
    #[error("Failed to deserialise Event Horizon configuration: {0}")]
    EvhConfigReadFailed(toml::de::Error),
    #[error("Failed to serialise Event Horizon configuration: {0}")]
    EvhConfigWriteFailed(toml::ser::Error),
    #[error("Failed to deserialise Singularity configuration: {0}")]
    RenderedConfigReadFailed(toml::de::Error),
    #[error("Failed to serialise Singularity configuration: {0}")]
    RenderedConfigWriteFailed(toml::ser::Error),
    #[error("No active Singularity configuration import with ID {0}")]
    NoActiveImport(String),
    #[error("Multipart failed in file upload")]
    MultipartError(#[from] actix_multipart::MultipartError),
    #[error("Multipart field in file upload was empty")]
    EmptyMultipartField,
    #[error("The uploaded file was not encoded in UTF-8")]
    UploadedFileNotUtf8,

    // errors created from other error types
    #[error("Failed to read environment variables: {0}")]
    EnvConfigFailed(#[from] envy::Error),
    #[error("Failed to initialise logger: {0}")]
    Logger(#[from] log::SetLoggerError),
    #[error("Database returned error: {0}")]
    Database(#[from] diesel::result::Error),

    // transparent errors
    #[error(transparent)]
    Singularity(#[from] singularity::SingularityError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
