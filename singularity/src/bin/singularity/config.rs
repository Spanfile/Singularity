use serde::{Deserialize, Serialize};
use singularity::{Adlist, Output};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    #[serde(default)]
    pub whitelist: HashSet<String>,
    pub adlist: Vec<Adlist>,
    pub output: Vec<Output>,
}
