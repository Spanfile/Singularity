use crate::database::{models, DbConn, DbId};
use diesel::prelude::*;
use log::*;
use singularity::{Adlist, Output, OutputType, HTTP_CONNECT_TIMEOUT};
use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
pub struct SingularityConfig(DbId);

impl SingularityConfig {
    pub fn new(conn: &mut DbConn) -> anyhow::Result<Self> {
        use crate::database::schema::singularity_configs;

        let cfg = diesel::insert_into(singularity_configs::table)
            .values(models::NewSingularityConfig {
                dirty: false,
                http_timeout: HTTP_CONNECT_TIMEOUT as i32,
            })
            .get_result::<models::SingularityConfig>(conn)?;

        debug!("Insert Singularity config: {:#?}", cfg);
        Ok(Self(cfg.id))
    }

    pub fn load(id: DbId, conn: &mut DbConn) -> anyhow::Result<Self> {
        use crate::database::schema::singularity_configs;

        let cfg = singularity_configs::table
            .filter(singularity_configs::id.eq(id))
            .first::<models::SingularityConfig>(conn)?;

        debug!("Singularity config {}: {:?}", id, cfg);
        Ok(Self(cfg.id))
    }

    pub fn import_singularity_config<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        todo!()
    }

    fn own_model(&self, conn: &mut DbConn) -> anyhow::Result<models::SingularityConfig> {
        use crate::database::schema::singularity_configs;

        let model = singularity_configs::table
            .filter(singularity_configs::id.eq(self.0))
            .first::<models::SingularityConfig>(conn)?;

        debug!("{:?}: {:?}", self, model);
        Ok(model)
    }

    /// Sets the dirty flag for this config.
    pub fn set_dirty(&self, conn: &mut DbConn, dirty: bool) -> anyhow::Result<()> {
        use crate::database::schema::singularity_configs;

        diesel::update(singularity_configs::table.filter(singularity_configs::id.eq(self.0)))
            .set(singularity_configs::dirty.eq(dirty))
            .execute(conn)?;

        debug!("{:?} dirty: {}", self, dirty);
        Ok(())
    }

    /// Adds a new adlist to the configuration. Returns the ID of the newly added adlist.
    pub fn add_adlist(&self, conn: &mut DbConn, adlist: Adlist) -> anyhow::Result<DbId> {
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
    pub fn delete_adlist(&self, conn: &mut DbConn, id: DbId) -> anyhow::Result<()> {
        use crate::database::schema::singularity_adlists;

        let rows = diesel::delete(singularity_adlists::table.filter(singularity_adlists::id.eq(id))).execute(conn)?;
        debug!("Delete adlist {}: {} rows deleted", id, rows);

        self.set_dirty(conn, true)
    }

    pub fn get_adlist(&self, conn: &mut DbConn, id: DbId) -> anyhow::Result<Adlist> {
        use crate::database::schema::singularity_adlists;

        let adlist = singularity_adlists::table
            .filter(singularity_adlists::id.eq(id))
            .first::<models::SingularityAdlist>(conn)?
            .try_into()?;

        debug!("Adlist {}: {:#?}", id, adlist);
        Ok(adlist)
    }

    pub fn adlists(&self, conn: &mut DbConn) -> anyhow::Result<Vec<(DbId, Adlist)>> {
        let own_model = self.own_model(conn)?;
        let adlists = models::SingularityAdlist::belonging_to(&own_model)
            .load::<models::SingularityAdlist>(conn)?
            .into_iter()
            .map(|model| Ok((model.id, model.try_into()?)))
            .collect::<anyhow::Result<Vec<_>>>()?;

        debug!("Adlists in {}: {}", self.0, adlists.len());
        Ok(adlists)
    }

    /// Adds a new output to the configuration. Returns the ID of the newly added output.
    pub fn add_output(&self, conn: &mut DbConn, output: Output) -> anyhow::Result<DbId> {
        use crate::database::schema::{
            singularity_output_hosts_includes, singularity_output_pdns_lua, singularity_outputs,
        };

        let id = conn.immediate_transaction::<_, anyhow::Error, _>(|conn| {
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
        })?;

        Ok(id)
    }

    /// Deletes a given output from the configuration.
    pub fn delete_output(&self, conn: &mut DbConn, id: DbId) -> anyhow::Result<()> {
        use crate::database::schema::singularity_outputs;

        // TODO: so uhh the ON DELETE CASCADE in the pdns lua table isn't working?
        let rows = diesel::delete(singularity_outputs::table.filter(singularity_outputs::id.eq(id))).execute(conn)?;
        debug!("Delete output {}: {} rows deleted", id, rows);

        self.set_dirty(conn, true)
    }

    pub fn get_output(&self, conn: &mut DbConn, id: DbId) -> anyhow::Result<Output> {
        use crate::database::schema::singularity_outputs;

        let output = singularity_outputs::table
            .filter(singularity_outputs::id.eq(id))
            .first::<models::SingularityOutput>(conn)?;
        let output = self.output_from_model(conn, output)?;

        debug!("Output {}: {:#?}", id, output);
        Ok(output)
    }

    pub fn outputs(&self, conn: &mut DbConn) -> anyhow::Result<Vec<(DbId, Output)>> {
        let own_model = self.own_model(conn)?;
        let outputs = models::SingularityOutput::belonging_to(&own_model)
            .load::<models::SingularityOutput>(conn)?
            .into_iter()
            .map(|model| Ok((model.id, self.output_from_model(conn, model)?)))
            .collect::<anyhow::Result<Vec<_>>>()?;

        debug!("Outputs in {}: {}", self.0, outputs.len());
        Ok(outputs)
    }

    fn output_from_model(&self, conn: &mut DbConn, mut output: models::SingularityOutput) -> anyhow::Result<Output> {
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

    /// Adds a new domain to the whitelist. Returns whether the domain was succesfully added.
    pub fn add_whitelisted_domain(&mut self, domain: String) -> bool {
        todo!()
    }

    /// Deletes a given domain from the whitelist.
    pub fn delete_whitelisted_domain(&mut self, id: u64) -> bool {
        todo!()
    }

    pub fn get_whitelist(&self, id: u64) -> Option<&str> {
        todo!()
    }

    pub fn whitelist(&self) -> anyhow::Result<Vec<(DbId, String)>> {
        todo!()
    }
}
