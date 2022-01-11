use crate::database::{models, DbConn, DbId};
use diesel::prelude::*;
use singularity::{Adlist, Output, OutputType};
use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
pub struct SingularityConfig(DbId);

impl SingularityConfig {
    pub fn new(id: DbId) -> Self {
        // TODO: might as well check if this ID is in the database
        Self(id)
    }

    pub fn import_singularity_config<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        todo!()
    }

    /// Sets the dirty flag for this config.
    pub fn set_dirty(&self, conn: &mut DbConn, dirty: bool) -> anyhow::Result<()> {
        use crate::database::schema::singularity_configs;

        diesel::update(singularity_configs::table.filter(singularity_configs::id.eq(self.0)))
            .set(singularity_configs::dirty.eq(dirty))
            .execute(conn)?;
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

        self.set_dirty(conn, true)?;
        Ok(adlist.id)
    }

    /// Removes a given adlist from the configuration. Returns whether the adlist was succesfully removed.
    pub fn remove_adlist(&self, conn: &mut DbConn, id: DbId) -> anyhow::Result<()> {
        use crate::database::schema::singularity_adlists;

        diesel::delete(singularity_adlists::table.filter(singularity_adlists::id.eq(id))).execute(conn)?;
        Ok(())
    }

    pub fn get_adlist(&self, conn: &mut DbConn, id: DbId) -> anyhow::Result<Adlist> {
        use crate::database::schema::singularity_adlists;

        singularity_adlists::table
            .filter(singularity_adlists::id.eq(id))
            .first::<models::SingularityAdlist>(conn)?
            .try_into()
    }

    pub fn adlists(&self, conn: &mut DbConn) -> anyhow::Result<Vec<(DbId, Adlist)>> {
        use crate::database::schema::singularity_adlists;

        singularity_adlists::table
            .load::<models::SingularityAdlist>(conn)?
            .into_iter()
            .map(|model| Ok((model.id, model.try_into()?)))
            .collect::<anyhow::Result<Vec<_>>>()
    }

    /// Adds a new output to the configuration. Returns the ID of the newly added output.
    pub fn add_output(&self, conn: &mut DbConn, output: Output) -> anyhow::Result<DbId> {
        use crate::database::schema::{
            singularity_output_hosts_includes, singularity_output_pdns_lua, singularity_outputs,
        };

        let id = conn.immediate_transaction::<_, anyhow::Error, _>(|conn| {
            let mut hosts_includes = Vec::new();
            let mut pdns_lua = Vec::new();

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
                        pdns_lua.push((*output_metric, metric_name.as_str()));

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

            for include in hosts_includes {
                diesel::insert_into(singularity_output_hosts_includes::table)
                    .values(models::NewSingularityOutputHostsInclude {
                        singularity_output_id: output.id,
                        path: include.as_os_str().as_bytes(),
                    })
                    .execute(conn)?;
            }

            for (output_metric, metric_name) in pdns_lua {
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

    /// Removes a given output from the configuration. Returns whether the output was succesfully removed.
    pub fn remove_output(&self, conn: &mut DbConn, id: DbId) -> anyhow::Result<()> {
        use crate::database::schema::singularity_outputs;

        diesel::delete(singularity_outputs::table.filter(singularity_outputs::id.eq(id))).execute(conn)?;
        Ok(())
    }

    pub fn get_output(&self, conn: &mut DbConn, id: DbId) -> anyhow::Result<Output> {
        use crate::database::schema::singularity_outputs;

        let output = singularity_outputs::table
            .filter(singularity_outputs::id.eq(id))
            .first::<models::SingularityOutput>(conn)?;
        self.output_from_model(conn, output)
    }

    pub fn outputs(&self, conn: &mut DbConn) -> anyhow::Result<Vec<(DbId, Output)>> {
        use crate::database::schema::singularity_outputs;

        singularity_outputs::table
            .load::<models::SingularityOutput>(conn)?
            .into_iter()
            .map(|model| Ok((model.id, self.output_from_model(conn, model)?)))
            .collect::<anyhow::Result<Vec<_>>>()
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

    /// Removes a given domain from the whitelist. Returns whether the domain was succesfully removed.
    pub fn remove_whitelisted_domain(&mut self, id: u64) -> bool {
        todo!()
    }

    pub fn get_whitelist(&self, id: u64) -> Option<&str> {
        todo!()
    }

    pub fn whitelist(&self) -> anyhow::Result<Vec<(DbId, String)>> {
        todo!()
    }
}
