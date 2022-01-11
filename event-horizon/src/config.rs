mod evh_config;

pub use evh_config::EvhConfig;

use crate::logging::LogLevel;
use serde::Deserialize;
use serde_with::with_prefix;
use std::{net::SocketAddr, path::PathBuf};

with_prefix!(listen "listen_");

const EVH_ENV_PREFIX: &str = "EVH_";

#[derive(Debug, Deserialize, Clone)]
pub struct EnvConfig {
    #[serde(default)]
    pub log_level: LogLevel,
    #[serde(flatten, with = "listen")]
    pub listen: Listen,
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
