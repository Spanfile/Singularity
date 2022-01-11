use serde::{Deserialize, Serialize};
use singularity::{Adlist, Output};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RenderedConfig {
    #[serde(default)]
    pub whitelist: HashSet<String>,
    pub adlist: Vec<Adlist>,
    pub output: Vec<Output>,
}

impl RenderedConfig {
    pub fn from_str(str: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(str)?)
    }
}
