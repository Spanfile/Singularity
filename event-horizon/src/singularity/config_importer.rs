use super::RenderedConfig;
use crate::{
    config::EvhConfig,
    error::{EvhError, EvhResult},
};
use indexmap::IndexMap;
use nanoid::nanoid;
use std::time::Instant;

pub struct ConfigImporter {
    imports: IndexMap<String, PendingImport>,
}

#[derive(Debug)]
struct PendingImport {
    rendered: RenderedConfig,
    time: Instant,
}

impl ConfigImporter {
    pub fn new() -> Self {
        Self {
            imports: IndexMap::new(),
        }
    }

    pub fn begin_import(&mut self, rendered: RenderedConfig, evh_config: &EvhConfig) -> String {
        let time = Instant::now();

        // ensure a duplicate ID won't be generated
        let id = loop {
            let id = nanoid!();
            if !self.imports.contains_key(&id) {
                break id;
            }
        };

        self.imports.insert(id.clone(), PendingImport { rendered, time });
        self.cleanup(evh_config);
        id
    }

    pub fn cancel_import(&mut self, id: &str, evh_config: &EvhConfig) {
        self.imports.remove(id);
        self.cleanup(evh_config);
    }

    pub fn get_string(&self, id: &str) -> EvhResult<String> {
        self.imports
            .get(id)
            .ok_or_else(|| EvhError::NoActiveImport(id.to_string()))
            .and_then(|import| import.rendered.as_string())
    }

    pub fn finish(&mut self, id: &str, evh_config: &EvhConfig) -> EvhResult<RenderedConfig> {
        let rendered = self
            .imports
            .remove(id)
            .map(|import| import.rendered)
            .ok_or_else(|| EvhError::NoActiveImport(id.to_string()))?;
        self.cleanup(evh_config);

        Ok(rendered)
    }

    pub fn cleanup(&mut self, evh_config: &EvhConfig) {
        loop {
            // keep removing imports until there's as much as concurrently allowed
            if self.imports.len() > evh_config.max_concurrent_imports {
                self.imports.pop();
                continue;
            }

            // pop imports until such is hit that is younger than the maximum allowed lifetime, therefore any imports
            // after it are also younger, or that there aren't any imports left
            if let Some((_, last)) = self.imports.last() {
                if last.time.elapsed().as_secs() >= evh_config.max_import_lifetime {
                    self.imports.pop();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
}
