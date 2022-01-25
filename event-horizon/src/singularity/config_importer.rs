use super::RenderedConfig;
use crate::{
    config::EvhConfig,
    error::{EvhError, EvhResult},
};
use nanoid::nanoid;

#[derive(Debug, Clone)]
pub struct ConfigImporter {
    max_concurrent_imports: usize,
    max_import_lifetime: u64,
}

impl ConfigImporter {
    pub fn new(evh_config: &EvhConfig) -> Self {
        Self {
            max_concurrent_imports: evh_config.redis.max_concurrent_imports,
            max_import_lifetime: evh_config.redis.max_import_lifetime,
        }
    }

    // i know, i know, redis best practices say to not store JSON as-is in there to avoid losing writes but in this
    // case it's literally just storing the serialised form and deleting it later, never altering it in-place, so
    // it's fine
    pub fn add_blocking<C>(&self, rendered: RenderedConfig, redis: &mut C) -> EvhResult<String>
    where
        C: redis::ConnectionLike,
    {
        let serialised = rendered.as_string()?;
        let id = nanoid!();

        redis::cmd("set")
            .arg(format!("config_import:{id}"))
            .arg(serialised)
            .arg("ex")
            .arg(self.max_import_lifetime)
            .query(redis)?;

        Ok(id)
    }

    pub fn get_blocking<C>(&self, import_id: &str, redis: &mut C) -> EvhResult<RenderedConfig>
    where
        C: redis::ConnectionLike,
    {
        // TODO: this should be an option or smth?
        let serialised: String = redis::cmd("get")
            .arg(format!("config_import:{import_id}"))
            .query::<Option<String>>(redis)?
            .ok_or_else(|| EvhError::NoActiveImport(import_id.to_string()))?;

        let rendered = RenderedConfig::from_str(&serialised)?;
        Ok(rendered)
    }

    pub fn remove_blocking<C>(&self, import_id: &str, redis: &mut C) -> EvhResult<RenderedConfig>
    where
        C: redis::ConnectionLike,
    {
        let serialised: String = redis::cmd("getdel")
            .arg(format!("config_import:{import_id}"))
            .query::<Option<String>>(redis)?
            .ok_or_else(|| EvhError::NoActiveImport(import_id.to_string()))?;

        let rendered = RenderedConfig::from_str(&serialised)?;
        Ok(rendered)
    }
}
