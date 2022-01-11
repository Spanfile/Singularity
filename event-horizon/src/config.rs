use crate::logging::LogLevel;
use serde::{Deserialize, Serialize};
use serde_with::with_prefix;
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

with_prefix!(listen "listen_");

const EVH_ENV_PREFIX: &str = "EVH_";
const EVH_CONFIG_LOCATION: &str = "evh.toml";
const EVH_CONFIG_WARNING: &str =
    "# These options are internal and critical to Event Horizon's functionality. You probably shouldn't edit them";

const DEFAULT_DATABASE_URL: &str = "evh.sqlite";

#[derive(Debug, Deserialize, Clone)]
pub struct EnvConfig {
    #[serde(default)]
    pub log_level: LogLevel,
    #[serde(flatten, with = "listen")]
    pub listen: Listen,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvhConfig {
    #[serde(default)]
    pub database_url: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "socket", rename_all = "snake_case")]
pub enum Listen {
    Http {
        bind: SocketAddr,
    },
    Https {
        bind: SocketAddr,
        tls_certificate: PathBuf,
        tls_certificate_key: PathBuf,
    },
    Unix {
        bind: PathBuf,
    },
}

impl EnvConfig {
    pub fn load() -> anyhow::Result<Self> {
        Ok(envy::prefixed(EVH_ENV_PREFIX).from_env::<EnvConfig>()?)
    }
}

impl EvhConfig {
    pub fn load() -> anyhow::Result<Self> {
        let path = Path::new(EVH_CONFIG_LOCATION);

        if path.exists() {
            Ok(toml::from_str(&fs::read_to_string(path)?)?)
        } else {
            let default = Self::default();
            fs::write(
                path,
                format!("{}\n{}", EVH_CONFIG_WARNING, toml::to_string_pretty(&default)?),
            )?;
            Ok(default)
        }
    }
}

impl Default for EvhConfig {
    fn default() -> Self {
        Self {
            database_url: DEFAULT_DATABASE_URL.to_string(),
        }
    }
}
