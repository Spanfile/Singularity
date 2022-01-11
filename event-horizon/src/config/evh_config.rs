use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const EVH_CONFIG_LOCATION: &str = "evh.toml";
const EVH_CONFIG_WARNING: &str =
    "# These options are internal and critical to Event Horizon's functionality. You probably shouldn't edit them";

const DEFAULT_DATABASE_URL: &str = "evh.sqlite";
const MAX_CONCURRENT_IMPORTS: usize = 5;
const MAX_IMPORT_LIFETIME: u64 = 300;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EvhConfig {
    pub database_url: String,
    pub max_concurrent_imports: usize,
    pub max_import_lifetime: u64,
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
            max_concurrent_imports: MAX_CONCURRENT_IMPORTS,
            max_import_lifetime: MAX_IMPORT_LIFETIME,
        }
    }
}
