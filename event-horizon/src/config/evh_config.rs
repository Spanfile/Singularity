use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const EVH_CONFIG_LOCATION: &str = "evh.toml";
const EVH_CONFIG_WARNING: &str =
    "# These options are internal and critical to Event Horizon's functionality. You probably shouldn't edit them";

const DEFAULT_DATABASE_URL: &str = "evh.sqlite";

#[derive(Debug, Serialize, Deserialize)]
pub struct EvhConfig {
    #[serde(default)]
    pub database_url: String,
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
