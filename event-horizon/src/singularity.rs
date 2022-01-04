use singularity::{Adlist, Output};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

#[derive(Debug, Default)]
pub struct SingularityConfig {
    /// If true, Singularity hasn't yet been ran with this config.
    dirty: bool,

    adlists: HashMap<String, Adlist>,
    outputs: HashSet<Output>,
    whitelist: HashSet<String>,
    http_timeout: u64,
}

impl SingularityConfig {
    pub fn import_singularity_config<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        todo!()
    }

    /// Adds a new adlist to the configuration. Returns whether the adlist was succesfully added.
    pub fn add_adlist(&mut self, adlist: Adlist) -> bool {
        if self.adlists.insert(adlist.source().to_string(), adlist).is_none() {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Removes a given adlist from the configuration. Returns whether the adlist was succesfully removed.
    pub fn remove_adlist(&mut self, source: &str) -> bool {
        if self.adlists.remove(source).is_some() {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn adlists(&self) -> impl Iterator<Item = (&str, &Adlist)> {
        self.adlists.iter().map(|(k, v)| (k.as_ref(), v))
    }

    /// Adds a new output to the configuration. Returns whether the output was succesfully added.
    pub fn add_output(&mut self, output: Output) -> bool {
        if self.outputs.insert(output) {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Removes a given output from the configuration. Returns whether the output was succesfully removed.
    pub fn remove_output(&mut self, output: Output) -> bool {
        if self.outputs.remove(&output) {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.outputs.iter()
    }

    /// Adds a new domain to the whitelist. Returns whether the domain was succesfully added.
    pub fn add_whitelisted_domain(&mut self, domain: String) -> bool {
        if self.whitelist.insert(domain) {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Removes a given domain from the whitelist. Returns whether the domain was succesfully removed.
    pub fn remove_whitelisted_domain(&mut self, domain: &str) -> bool {
        if self.whitelist.remove(domain) {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn whitelist(&self) -> impl Iterator<Item = &str> {
        self.whitelist.iter().map(|s| s.as_ref())
    }
}
