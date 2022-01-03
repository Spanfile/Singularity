use serde::{Deserialize, Serialize};
use singularity::{Adlist, OutputConfig};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    #[serde(default)]
    pub whitelist: HashSet<String>,
    pub adlist: Vec<Adlist>,
    pub output: Vec<OutputConfig>,
}
