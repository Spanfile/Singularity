use serde::{Deserialize, Serialize};
use singularity::{Adlist, Output};
use std::collections::HashSet;

use crate::error::{EvhError, EvhResult};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RenderedConfig {
    #[serde(default)]
    pub whitelist: HashSet<String>,
    pub adlist: Vec<Adlist>,
    pub output: Vec<Output>,
}

impl RenderedConfig {
    pub fn from_str(str: &str) -> EvhResult<Self> {
        Ok(toml::from_str(str).map_err(|e| EvhError::RenderedConfigReadFailed(e))?)
    }

    pub fn as_string(&self) -> EvhResult<String> {
        Ok(toml::to_string_pretty(self).map_err(|e| EvhError::RenderedConfigWriteFailed(e))?)
    }
}
