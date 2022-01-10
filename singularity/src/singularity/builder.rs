use super::{noop_callback, Adlist, Output, Singularity, HTTP_CONNECT_TIMEOUT};
use std::collections::HashSet;

/// Builder for a new [`Singularity`].
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

    /// Finalises the builder and returns a new [`Singularity`].
    pub fn build(self) -> Singularity<'a> {
        Singularity {
            adlists: self.adlists,
            outputs: self.outputs,
            whitelist: self.whitelist,
            http_timeout: self.http_timeout,
            prog_callback: Box::new(noop_callback),
        }
    }

    /// Adds a given [`Adlist`] to the builder.
    #[must_use]
    pub fn add_adlist(mut self, adlist: Adlist) -> Self {
        self.adlists.push(adlist);
        self
    }

    /// Adds multiple [Adlists][Adlist] to the builder from an iterator.
    #[must_use]
    pub fn add_many_adlists<I>(mut self, adlists: I) -> Self
    where
        I: IntoIterator<Item = Adlist>,
    {
        self.adlists.extend(adlists);
        self
    }

    /// Adds a given [`Output`] to the builder.
    #[must_use]
    pub fn add_output(mut self, output: Output) -> Self {
        self.outputs.push(output);
        self
    }

    /// Adds multiple [`Output`s](Output) to the builder from an iterator.
    #[must_use]
    pub fn add_many_outputs<I>(mut self, outputs: I) -> Self
    where
        I: IntoIterator<Item = Output>,
    {
        self.outputs.extend(outputs.into_iter());
        self
    }

    /// Whitelist a certain domain.
    #[must_use]
    pub fn whitelist_domain<S>(mut self, domain: S) -> Self
    where
        S: Into<String>,
    {
        self.whitelist.insert(domain.into());
        self
    }

    /// Whitelist multiple domains from an iterator.
    #[must_use]
    pub fn whitelist_many_domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.whitelist.extend(domains.into_iter().map(|s| s.into()));
        self
    }

    /// Set the HTTP timeout for requests.
    #[must_use]
    pub fn http_timeout(mut self, timeout: u64) -> Self {
        self.http_timeout = timeout;
        self
    }
}