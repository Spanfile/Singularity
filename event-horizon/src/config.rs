use crate::logging::LogLevel;
use serde::{Deserialize, Serialize};
use serde_with::with_prefix;
use std::{net::SocketAddr, path::PathBuf};

with_prefix!(listen "listen_");

const EVH_ENV_PREFIX: &str = "EVH_";
const EVH_CONFIG_LOCATION: &str = "evh.toml";

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
    #[serde(default = "default_database_url")]
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
        Ok(toml::from_str(&std::fs::read_to_string(EVH_CONFIG_LOCATION)?)?)
    }
}

fn default_database_url() -> String {
    DEFAULT_DATABASE_URL.to_string()
}
