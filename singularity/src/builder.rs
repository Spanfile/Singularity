use crate::{noop_callback, Adlist, Output, Singularity, HTTP_CONNECT_TIMEOUT};
use std::collections::HashSet;

pub struct SingularityBuilder {
    adlists: Vec<Adlist>,
    outputs: Vec<Output>,
    whitelist: HashSet<String>,
    http_timeout: u64,
}

impl<'a> SingularityBuilder {
    pub(crate) fn new() -> Self {
        Self {
            adlists: Vec::new(),
            outputs: Vec::new(),
            whitelist: HashSet::new(),
            http_timeout: HTTP_CONNECT_TIMEOUT,
        }
    }

    pub fn build(self) -> Singularity<'a> {
        Singularity {
            adlists: self.adlists,
            outputs: self.outputs,
            whitelist: self.whitelist,
            http_timeout: self.http_timeout,
            prog_callback: Box::new(noop_callback),
        }
    }

    #[must_use]
    pub fn add_adlist(mut self, adlist: Adlist) -> Self {
        self.adlists.push(adlist);
        self
    }

    #[must_use]
    pub fn add_many_adlists<I>(mut self, adlists: I) -> Self
    where
        I: IntoIterator<Item = Adlist>,
    {
        self.adlists.extend(adlists);
        self
    }

    #[must_use]
    pub fn add_output(mut self, output: Output) -> Self {
        self.outputs.push(output);
        self
    }

    #[must_use]
    pub fn add_outputs_from_configs<I>(mut self, output_configs: I) -> Self
    where
        I: IntoIterator<Item = Output>,
    {
        self.outputs.extend(output_configs.into_iter());
        self
    }

    #[must_use]
    pub fn whitelist_domain<S>(mut self, domain: S) -> Self
    where
        S: Into<String>,
    {
        self.whitelist.insert(domain.into());
        self
    }

    #[must_use]
    pub fn whitelist_many_domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.whitelist.extend(domains.into_iter().map(|s| s.into()));
        self
    }

    #[must_use]
    pub fn http_timeout(mut self, timeout: u64) -> Self {
        self.http_timeout = timeout;
        self
    }
}
