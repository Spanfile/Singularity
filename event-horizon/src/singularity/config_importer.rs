use crate::config::EvhConfig;
use indexmap::IndexMap;
use nanoid::nanoid;
use std::{fs::File, io::Write, time::Instant};
use tempfile::tempfile;

pub struct ConfigImporter {
    imports: IndexMap<String, PendingImport>,
}

#[derive(Debug)]
struct PendingImport {
    file: File,
    time: Instant,
}

impl ConfigImporter {
    pub fn new() -> Self {
        Self {
            imports: IndexMap::new(),
        }
    }

    pub fn begin_import(&mut self, contents: String, evh_config: &EvhConfig) -> anyhow::Result<String> {
        let time = Instant::now();

        let mut file = tempfile()?;
        write!(file, "{}", contents)?;

        // ensure a duplicate ID won't be generated
        let id = loop {
            let id = nanoid!();
            if !self.imports.contains_key(&id) {
                break id;
            }
        };

        self.imports.insert(id.clone(), PendingImport { file, time });
        self.cleanup(evh_config);
        Ok(id)
    }

    pub fn cancel_import(&mut self, id: &str, evh_config: &EvhConfig) {
        self.imports.remove(id);
        self.cleanup(evh_config);
    }

    pub fn cleanup(&mut self, evh_config: &EvhConfig) {
        loop {
            // keep removing imports until there's as much as concurrently allowed
            if self.imports.len() > evh_config.max_concurrent_imports {
                self.imports.pop();
                continue;
            }

            // pop imports until such is hit that is younger than the maximum allowed lifetime, therefore any imports
            // after it are also younger
            if let Some((_, last)) = self.imports.last() {
                if last.time.elapsed().as_secs() >= evh_config.max_import_lifetime {
                    self.imports.pop();
                } else {
                    break;
                }
            }
        }
    }
}
