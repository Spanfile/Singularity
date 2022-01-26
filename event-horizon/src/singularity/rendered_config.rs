use serde::{Deserialize, Serialize};
use singularity::{Adlist, Output};
use std::collections::HashSet;

use crate::error::{EvhError, EvhResult};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RenderedConfig {
    #[serde(skip)]
    pub name: String,
    #[serde(default)]
    pub whitelist: HashSet<String>,
    pub adlist: Vec<Adlist>,
    pub output: Vec<Output>,
}

impl RenderedConfig {
    pub fn from_str<S, SR>(name: S, str: SR) -> EvhResult<Self>
    where
        S: Into<String>,
        SR: AsRef<str>,
    {
        let mut cfg: Self = toml::from_str(str.as_ref()).map_err(EvhError::RenderedConfigReadFailed)?;
        cfg.name = name.into();
        Ok(cfg)
    }

    pub fn as_string(&self) -> EvhResult<String> {
        toml::to_string_pretty(self).map_err(EvhError::RenderedConfigWriteFailed)
    }

    pub fn into_name_string_tuple(self) -> EvhResult<(String, String)> {
        let rendered = self.as_string()?;
        Ok((self.name, rendered))
    }
}
