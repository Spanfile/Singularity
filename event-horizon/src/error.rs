use crate::database::DbId;
use thiserror::Error;

pub type EvhResult<T> = std::result::Result<T, EvhError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EvhError {
    // direct errors
    #[error("Failed to initialise database pool: {0}")]
    DatabasePoolInitialisationFailed(diesel::r2d2::PoolError),
    #[error("Failed to run database migrations: {0}")]
    DatabaseMigrationsFailed(Box<dyn std::error::Error + Send + Sync>),
    #[error("Failed to initialise Redis pool: {0}")]
    RedisPoolInitialisationFailed(diesel::r2d2::PoolError),
    #[error("Failed to get Redis information")]
    RedisInfoFailed,
    #[error("Failed to acquire database connection: {0}")]
    DatabaseConnectionAcquireFailed(diesel::r2d2::PoolError),
    #[error("Failed to acquire Redis connection: {0}")]
    RedisConnectionAcquireFailed(diesel::r2d2::PoolError),
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
    #[error("Multipart field in the file upload has no filename")]
    MissingFieldFilename,
    #[error("Received text was not encoded in UTF-8")]
    TextNotUtf8,
    #[error("EVH setting has invalid value for type {0}: {1}")]
    InvalidSetting(DbId, String),
    #[error("No such Singularity configuration item: {0}")]
    NoSuchConfigItem(DbId),
    #[error("The provided name was empty")]
    EmptyConfigName,
    #[error("The provided name is already set for some other configuration")]
    DuplicateConfigName,
    #[error("Invalid Cron schedule: {0}")]
    InvalidCronSchedule(cron_clock::error::Error),
    #[error("Object is missing from the app data")]
    MissingAppData,
    #[error("A request's query string is invalid")]
    InvalidQueryString(actix_web::error::QueryPayloadError),
    #[error("Attempt to delete builtin output ID {0}")]
    AttemptToDeleteBuiltinOutput(DbId, singularity::Output),
    #[error("Recursor returned non-zero code {0} in control call: {1}")]
    RecControl(i32, String),

    // errors created from other error types
    #[error("Failed to read environment variables: {0}")]
    EnvConfigFailed(#[from] envy::Error),
    #[error("Failed to initialise logger: {0}")]
    Logger(#[from] log::SetLoggerError),
    #[error("Database returned error: {0}")]
    Database(#[from] diesel::result::Error),
    #[error("Redis returned error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Failed to parse datetime: {0}")]
    DateTime(#[from] chrono::format::ParseError),

    // transparent errors
    #[error(transparent)]
    Singularity(#[from] singularity::SingularityError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
