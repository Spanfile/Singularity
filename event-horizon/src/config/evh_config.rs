use crate::error::{EvhError, EvhResult};
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use url::Url;

const EVH_CONFIG_WARNING: &str = "# These options are internal and critical to Event Horizon's functionality. You \
                                  probably shouldn't edit them. The structure of this file is subject to change at \
                                  any time without warning.";

const DEFAULT_DATABASE_URL: &str = "evh.sqlite";
const DEFAULT_REDIS_URL: &str = "redis://redis/";
const REDIS_CONNECTION_TIMEOUT: u64 = 5000;
const MAX_CONCURRENT_IMPORTS: usize = 5;
const MAX_IMPORT_LIFETIME: u64 = 300;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EvhConfig {
    pub database_url: String,
    pub redis: RedisSettings,
    pub recursor: RecursorSettings,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RedisSettings {
    pub redis_url: Url,
    pub connection_timeout: u64,
    pub max_concurrent_imports: usize,
    pub max_import_lifetime: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RecursorSettings {
    pub hostname: String,
    pub username: String,
    pub private_key_location: PathBuf,
    pub remote_host_key: String,
    pub verify_remote_host_key: bool,
}

impl EvhConfig {
    pub fn load(path: &Path) -> EvhResult<Self> {
        if path.exists() {
            Ok(toml::from_str(&fs::read_to_string(path)?).map_err(EvhError::EvhConfigReadFailed)?)
        } else {
            debug!("{} doesn't exist, creating new EvhConfig", path.display());

            let default = Self::default();
            fs::write(
                path,
                format!(
                    "{}\n{}",
                    EVH_CONFIG_WARNING,
                    toml::to_string_pretty(&default).map_err(EvhError::EvhConfigWriteFailed)?
                ),
            )?;
            Ok(default)
        }
    }
}

impl Default for EvhConfig {
    fn default() -> Self {
        Self {
            database_url: DEFAULT_DATABASE_URL.to_string(),
            redis: Default::default(),
            recursor: Default::default(),
        }
    }
}

impl Default for RedisSettings {
    fn default() -> Self {
        Self {
            redis_url: Url::parse(DEFAULT_REDIS_URL).expect("failed to parse DEFAULT_REDIS_URL"),
            connection_timeout: REDIS_CONNECTION_TIMEOUT,
            max_concurrent_imports: MAX_CONCURRENT_IMPORTS,
            max_import_lifetime: MAX_IMPORT_LIFETIME,
        }
    }
}
