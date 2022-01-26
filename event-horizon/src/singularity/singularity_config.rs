pub(super) mod config_manager;

use super::RenderedConfig;
use crate::{
    database::{models, DbConn, DbId},
    error::{EvhError, EvhResult},
};
use diesel::prelude::*;
use log::*;
use singularity::{Adlist, Output, OutputType, HTTP_CONNECT_TIMEOUT};
use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
};

pub type AdlistCollection = Vec<(DbId, Adlist)>;
pub type OutputCollection = Vec<(DbId, Output)>;
pub type WhitelistCollection = Vec<(DbId, String)>;

#[derive(Debug, Default)]
pub struct SingularityConfig(DbId);

impl SingularityConfig {
    pub fn new<S>(conn: &mut DbConn, name: S) -> EvhResult<Self>
    where
        S: AsRef<str>,
    {
        use crate::database::schema::singularity_configs;

        let cfg = diesel::insert_into(singularity_configs::table)
            .values(models::NewSingularityConfig {
                dirty: false,
                http_timeout: HTTP_CONNECT_TIMEOUT as i32,
                name: name.as_ref(),
            })
            .get_result::<models::SingularityConfig>(conn)?;

        debug!("Insert Singularity config: {:#?}", cfg);
        Ok(Self(cfg.id))
    }

    pub fn load(id: DbId, conn: &mut DbConn) -> EvhResult<(String, Self)> {
        use crate::database::schema::singularity_configs;

        let cfg = singularity_configs::table
            .filter(singularity_configs::id.eq(id))
            .first::<models::SingularityConfig>(conn)
            .optional()?
            .ok_or(EvhError::NoSuchConfig(id))?;

        debug!("Singularity config {}: {:?}", id, cfg);
        Ok((cfg.name, Self(cfg.id)))
    }

    pub fn load_all(conn: &mut DbConn) -> EvhResult<Vec<(String, Self)>> {
        use crate::database::schema::singularity_configs;

        let cfgs = singularity_configs::table
            .load::<models::SingularityConfig>(conn)?
            .into_iter()
            .map(|cfg| (cfg.name, Self(cfg.id)))
            .collect::<Vec<_>>();

        debug!("Singularity configs: {}", cfgs.len());
        Ok(cfgs)
    }

    pub fn id(&self) -> DbId {
        self.0
    }

    pub fn overwrite(&self, conn: &mut DbConn, rendered: RenderedConfig) -> EvhResult<()> {
        conn.immediate_transaction(|conn| {
            let own_model = self.own_model(conn)?;

            let adlists = diesel::delete(models::SingularityAdlist::belonging_to(&own_model)).execute(conn)?;
            let outputs = diesel::delete(models::SingularityOutput::belonging_to(&own_model)).execute(conn)?;
            let whitelist = diesel::delete(models::SingularityWhitelist::belonging_to(&own_model)).execute(conn)?;

            debug!(
                "Overwriting {}: {} adlists deleted, {} outputs deleted, {} whitelist domains deleted",
                self.0, adlists, outputs, whitelist
            );

            for adlist in rendered.adlist {
                self.add_adlist(conn, &adlist)?;
            }

            for output in rendered.output {
                self.add_output_without_transaction(conn, &output)?;
            }

            for domain in rendered.whitelist {
                self.add_whitelisted_domain(conn, &domain)?;
            }

            self.set_dirty(conn, true)
        })
    }

    pub fn merge(&self, conn: &mut DbConn, rendered: RenderedConfig) -> EvhResult<()> {
        fn ignore_duplicate<F, R>(mut f: F) -> EvhResult<bool>
        where
            F: FnMut() -> EvhResult<R>,
        {
            match (f)() {
                Ok(_) => Ok(true),
                Err(EvhError::Database(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                ))) => Ok(false),
                Err(e) => Err(e),
            }
        }

        conn.immediate_transaction(|conn| {
            for adlist in rendered.adlist {
                if !ignore_duplicate(|| self.add_adlist(conn, &adlist))? {
                    warn!("Ignored duplicate adlist {:?}", adlist);
                }
            }

            for output in rendered.output {
                if !ignore_duplicate(|| self.add_output_without_transaction(conn, &output))? {
                    warn!("Ignored duplicate output {:?}", output);
                }
            }

            for domain in rendered.whitelist {
                if !ignore_duplicate(|| self.add_whitelisted_domain(conn, &domain))? {
                    warn!("Ignored duplicate whitelisted domain {}", domain);
                }
            }

            self.set_dirty(conn, true)
        })
    }

    fn own_model(&self, conn: &mut DbConn) -> EvhResult<models::SingularityConfig> {
        use crate::database::schema::singularity_configs;

        let model = singularity_configs::table
            .filter(singularity_configs::id.eq(self.0))
            .first::<models::SingularityConfig>(conn)?;

        debug!("{:?}: {:?}", self, model);
        Ok(model)
    }

    /// Sets the dirty flag for this config.
    pub fn set_dirty(&self, conn: &mut DbConn, dirty: bool) -> EvhResult<()> {
        use crate::database::schema::singularity_configs;

        diesel::update(singularity_configs::table.filter(singularity_configs::id.eq(self.0)))
            .set(singularity_configs::dirty.eq(dirty))
            .execute(conn)?;

        debug!("{:?} dirty: {}", self, dirty);
        Ok(())
    }

    /// Adds a new adlist to the configuration. Returns the ID of the newly added adlist.
    pub fn add_adlist(&self, conn: &mut DbConn, adlist: &Adlist) -> EvhResult<DbId> {
        use crate::database::schema::singularity_adlists;

        let model = models::NewSingularityAdlist {
            singularity_config_id: self.0,
            source: adlist.source().as_str(),
            format: &adlist.format().to_string(),
        };

        let adlist = diesel::insert_into(singularity_adlists::table)
            .values(&model)
            .get_result::<models::SingularityAdlist>(conn)?;

        debug!("Insert adlist: {:#?}", adlist);
        self.set_dirty(conn, true)?;
        Ok(adlist.id)
    }

    /// Deletes a given adlist from the configuration.
    pub fn delete_adlist(&self, conn: &mut DbConn, id: DbId) -> EvhResult<()> {
        use crate::database::schema::singularity_adlists;

        let rows = diesel::delete(singularity_adlists::table.filter(singularity_adlists::id.eq(id))).execute(conn)?;
        debug!("Delete adlist {}: {} rows deleted", id, rows);

        self.set_dirty(conn, true)
    }

    pub fn get_adlist(&self, conn: &mut DbConn, id: DbId) -> EvhResult<Adlist> {
        use crate::database::schema::singularity_adlists;

        let adlist = singularity_adlists::table
            .filter(singularity_adlists::id.eq(id))
            .first::<models::SingularityAdlist>(conn)?
            .try_into()?;

        debug!("Adlist {}: {:#?}", id, adlist);
        Ok(adlist)
    }

    pub fn adlists(&self, conn: &mut DbConn) -> EvhResult<AdlistCollection> {
        let own_model = self.own_model(conn)?;
        let adlists = models::SingularityAdlist::belonging_to(&own_model)
            .load::<models::SingularityAdlist>(conn)?
            .into_iter()
            .map(|model| Ok((model.id, model.try_into()?)))
            .collect::<EvhResult<AdlistCollection>>()?;

        debug!("Adlists in {}: {}", self.0, adlists.len());
        Ok(adlists)
    }

    /// Adds a new output to the configuration. Returns the ID of the newly added output.
    pub fn add_output(&self, conn: &mut DbConn, output: &Output) -> EvhResult<DbId> {
        conn.immediate_transaction::<_, EvhError, _>(|conn| self.do_add_output(conn, output))
    }

    /// Adds a new output to the configuration. Returns the ID of the newly added output.
    ///
    /// The counterpart to this function, [`add_output`], spawns a database transaction to run its own multiple queries
    /// inside. This function doesn't use a transaction, and is meant to use in scenarios where you already have a
    /// running transaction.
    pub fn add_output_without_transaction(&self, conn: &mut DbConn, output: &Output) -> EvhResult<DbId> {
        self.do_add_output(conn, output)
    }

    fn do_add_output(&self, conn: &mut DbConn, output: &Output) -> EvhResult<DbId> {
        use crate::database::schema::{
            singularity_output_hosts_includes, singularity_output_pdns_lua, singularity_outputs,
        };

        let mut hosts_includes = Vec::new();
        let mut pdns_lua = None;

        let blackhole_address = output.blackhole_address().to_string();
        let model = models::NewSingularityOutput {
            singularity_config_id: self.0,
            ty: match output.ty() {
                OutputType::Hosts { include } => {
                    for path in include {
                        hosts_includes.push(path.as_path());
                    }

                    "Hosts"
                }
                OutputType::PdnsLua {
                    output_metric,
                    metric_name,
                } => {
                    pdns_lua = Some((*output_metric, metric_name.as_str()));

                    "PdnsLua"
                }
            },
            destination: output.destination().as_os_str().as_bytes(),
            blackhole_address: blackhole_address.as_str(),
            deduplicate: output.deduplicate(),
        };

        let output = diesel::insert_into(singularity_outputs::table)
            .values(&model)
            .get_result::<models::SingularityOutput>(conn)?;

        debug!("Insert output: {:#?}", output);
        debug!("Hosts includes: {:?}", hosts_includes);
        debug!("PDNS Lua: {:?}", pdns_lua);

        for include in hosts_includes {
            diesel::insert_into(singularity_output_hosts_includes::table)
                .values(models::NewSingularityOutputHostsInclude {
                    singularity_output_id: output.id,
                    path: include.as_os_str().as_bytes(),
                })
                .execute(conn)?;
        }

        if let Some((output_metric, metric_name)) = pdns_lua {
            diesel::insert_into(singularity_output_pdns_lua::table)
                .values(models::NewSingularityOutputPdnsLua {
                    singularity_output_id: output.id,
                    output_metric,
                    metric_name,
                })
                .execute(conn)?;
        }

        self.set_dirty(conn, true)?;
        Ok(output.id)
    }

    /// Deletes a given output from the configuration.
    pub fn delete_output(&self, conn: &mut DbConn, id: DbId) -> EvhResult<()> {
        use crate::database::schema::singularity_outputs;

        // TODO: so uhh the ON DELETE CASCADE in the pdns lua table isn't working?
        let rows = diesel::delete(singularity_outputs::table.filter(singularity_outputs::id.eq(id))).execute(conn)?;
        debug!("Delete output {}: {} rows deleted", id, rows);

        self.set_dirty(conn, true)
    }

    pub fn get_output(&self, conn: &mut DbConn, id: DbId) -> EvhResult<Output> {
        use crate::database::schema::singularity_outputs;

        let output = singularity_outputs::table
            .filter(singularity_outputs::id.eq(id))
            .first::<models::SingularityOutput>(conn)?;
        let output = self.output_from_model(conn, output)?;

        debug!("Output {}: {:#?}", id, output);
        Ok(output)
    }

    pub fn outputs(&self, conn: &mut DbConn) -> EvhResult<OutputCollection> {
        let own_model = self.own_model(conn)?;
        let outputs = models::SingularityOutput::belonging_to(&own_model)
            .load::<models::SingularityOutput>(conn)?
            .into_iter()
            .map(|model| Ok((model.id, self.output_from_model(conn, model)?)))
            .collect::<EvhResult<OutputCollection>>()?;

        debug!("Outputs in {}: {}", self.0, outputs.len());
        Ok(outputs)
    }

    fn output_from_model(&self, conn: &mut DbConn, mut output: models::SingularityOutput) -> EvhResult<Output> {
        output.ty.make_ascii_lowercase();

        let output_type = match output.ty.as_ref() {
            "hosts" => {
                let includes = models::SingularityOutputHostsInclude::belonging_to(&output)
                    .load::<models::SingularityOutputHostsInclude>(conn)?;

                OutputType::Hosts {
                    include: includes
                        .into_iter()
                        .map(|model| PathBuf::from(OsString::from_vec(model.path)))
                        .collect(),
                }
            }
            "pdnslua" => {
                let pdns_lua = models::SingularityOutputPdnsLua::belonging_to(&output)
                    .first::<models::SingularityOutputPdnsLua>(conn)?;

                OutputType::PdnsLua {
                    output_metric: pdns_lua.output_metric,
                    metric_name: pdns_lua.metric_name,
                }
            }
            _ => todo!(),
        };

        Ok(
            Output::builder(output_type, PathBuf::from(OsString::from_vec(output.destination)))
                .blackhole_address(output.blackhole_address)?
                .deduplicate(output.deduplicate)
                .build()?,
        )
    }

    /// Adds a new domain to the whitelist. Returns the ID of the newly whitelisted domain.
    pub fn add_whitelisted_domain(&self, conn: &mut DbConn, domain: &str) -> EvhResult<DbId> {
        use crate::database::schema::singularity_whitelists;

        let model = models::NewSingularityWhitelist {
            singularity_config_id: self.0,
            domain,
        };

        let whitelist = diesel::insert_into(singularity_whitelists::table)
            .values(&model)
            .get_result::<models::SingularityWhitelist>(conn)?;

        debug!("Insert whitelist: {:#?}", whitelist);
        self.set_dirty(conn, true)?;
        Ok(whitelist.id)
    }

    /// Deletes a given domain from the whitelist.
    pub fn delete_whitelisted_domain(&self, conn: &mut DbConn, id: DbId) -> EvhResult<()> {
        use crate::database::schema::singularity_whitelists;

        let rows =
            diesel::delete(singularity_whitelists::table.filter(singularity_whitelists::id.eq(id))).execute(conn)?;
        debug!("Delete whitelist {}: {} rows deleted", id, rows);

        self.set_dirty(conn, true)
    }

    pub fn get_whitelist(&self, conn: &mut DbConn, id: DbId) -> EvhResult<String> {
        use crate::database::schema::singularity_whitelists;

        let whitelist = singularity_whitelists::table
            .filter(singularity_whitelists::id.eq(id))
            .first::<models::SingularityWhitelist>(conn)?;

        debug!("Whitelist {}: {:#?}", id, whitelist);
        Ok(whitelist.domain)
    }

    pub fn whitelist(&self, conn: &mut DbConn) -> EvhResult<WhitelistCollection> {
        let own_model = self.own_model(conn)?;
        let whitelist = models::SingularityWhitelist::belonging_to(&own_model)
            .load::<models::SingularityWhitelist>(conn)?
            .into_iter()
            .map(|model| (model.id, model.domain))
            .collect::<WhitelistCollection>();

        debug!("Whitelist in {}: {}", self.0, whitelist.len());
        Ok(whitelist)
    }
}
