use diesel::{prelude::*, SqliteConnection};
use itertools::Itertools;
use singularity::{Adlist, Output};
use std::path::Path;

#[derive(Debug, Default)]
pub struct SingularityConfig(i32);

// YE AIGHT SO THIS IS WHATCHU GONNA DO:
// STORE THE ID AS IS IN THE THING, AND THEN JUST QUERY THE DATABASE AS NEEDED WHEN OPERATING

impl SingularityConfig {
    pub fn new(id: i32) -> Self {
        // TODO: might as well check if this ID is in the database
        Self(id)
    }

    pub fn import_singularity_config<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        todo!()
    }

    /// Adds a new adlist to the configuration. Returns whether the adlist was succesfully added.
    pub fn add_adlist(&mut self, adlist: Adlist) -> bool {
        if self.adlists.values().any(|other| &adlist == other) {
            return false;
        }

        self.adlists.insert(self.last_id, adlist);
        self.last_id += 1;
        self.dirty = true;
        true
    }

    /// Removes a given adlist from the configuration. Returns whether the adlist was succesfully removed.
    pub fn remove_adlist(&mut self, id: u64) -> bool {
        if self.adlists.remove(&id).is_some() {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn get_adlist(&self, id: u64) -> Option<&Adlist> {
        self.adlists.get(&id)
    }

    pub fn adlists(&self) -> impl Iterator<Item = (u64, &Adlist)> {
        self.adlists.iter().map(|(k, v)| (*k, v)).sorted_by_key(|(k, _)| *k)
    }

    /// Adds a new output to the configuration. Returns whether the output was succesfully added.
    pub fn add_output(&mut self, output: Output) -> bool {
        if self.outputs.values().any(|other| &output == other) {
            return false;
        }

        self.outputs.insert(self.last_id, output);
        self.last_id += 1;
        self.dirty = true;
        true
    }

    /// Removes a given output from the configuration. Returns whether the output was succesfully removed.
    pub fn remove_output(&mut self, id: u64) -> bool {
        if self.outputs.remove(&id).is_some() {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn get_output(&self, id: u64) -> Option<&Output> {
        self.outputs.get(&id)
    }

    pub fn outputs(&self) -> impl Iterator<Item = (u64, &Output)> {
        self.outputs.iter().map(|(k, v)| (*k, v)).sorted_by_key(|(k, _)| *k)
    }

    /// Adds a new domain to the whitelist. Returns whether the domain was succesfully added.
    pub fn add_whitelisted_domain(&mut self, domain: String) -> bool {
        if self.whitelist.values().any(|other| &domain == other) {
            return false;
        }

        self.whitelist.insert(self.last_id, domain);
        self.last_id += 1;
        self.dirty = true;
        true
    }

    /// Removes a given domain from the whitelist. Returns whether the domain was succesfully removed.
    pub fn remove_whitelisted_domain(&mut self, id: u64) -> bool {
        if self.whitelist.remove(&id).is_some() {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn get_whitelist(&self, id: u64) -> Option<&str> {
        self.whitelist.get(&id).map(|s| s.as_str())
    }

    pub fn whitelist(&self) -> impl Iterator<Item = (u64, &str)> {
        self.whitelist
            .iter()
            .map(|(k, v)| (*k, v.as_ref()))
            .sorted_by_key(|(k, _)| *k)
    }
}
