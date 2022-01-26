use super::RenderedConfig;
use crate::{
    config::EvhConfig,
    error::{EvhError, EvhResult},
};
use nanoid::nanoid;

#[derive(Debug)]
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
        let (name, serialised) = rendered.into_name_string_tuple()?;
        let id = nanoid!();

        redis::pipe()
            .atomic()
            .cmd("set")
            .arg(config_import_name_key(&id))
            .arg(name)
            .arg("ex")
            .arg(self.max_import_lifetime)
            .cmd("set")
            .arg(config_import_key(&id))
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
        let (name, serialised): (String, String) = redis::pipe()
            .atomic()
            .cmd("get")
            .arg(config_import_name_key(import_id))
            .cmd("get")
            .arg(config_import_key(import_id))
            .query::<Option<_>>(redis)?
            .ok_or_else(|| EvhError::NoActiveImport(import_id.to_string()))?;

        let rendered = RenderedConfig::from_str(name, &serialised)?;
        Ok(rendered)
    }

    pub fn remove_blocking<C>(&self, import_id: &str, redis: &mut C) -> EvhResult<RenderedConfig>
    where
        C: redis::ConnectionLike,
    {
        let (name, serialised): (String, String) = redis::pipe()
            .atomic()
            .cmd("getdel")
            .arg(config_import_name_key(import_id))
            .cmd("getdel")
            .arg(config_import_key(import_id))
            .query::<Option<_>>(redis)?
            .ok_or_else(|| EvhError::NoActiveImport(import_id.to_string()))?;

        let rendered = RenderedConfig::from_str(name, &serialised)?;
        Ok(rendered)
    }
}

fn config_import_key(id: &str) -> String {
    format!("config_import:{id}")
}

fn config_import_name_key(id: &str) -> String {
    format!("config_import:{id}:name")
}
