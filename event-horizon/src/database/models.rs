use super::{schema::*, DbId};
use singularity::{Adlist, AdlistFormat};

#[derive(Identifiable, Queryable, Insertable, PartialEq, Debug)]
pub struct SingularityConfig {
    pub id: DbId,
    pub dirty: bool,
    pub http_timeout: i32,
}

#[derive(Insertable)]
#[diesel(table_name = singularity_configs)]
pub struct NewSingularityConfig {
    pub dirty: bool,
    pub http_timeout: i32,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[diesel(belongs_to(SingularityConfig))]
pub struct SingularityAdlist {
    pub id: DbId,
    pub singularity_config_id: DbId,
    pub source: String,
    pub format: String,
}

#[derive(Insertable)]
#[diesel(table_name = singularity_adlists)]
pub struct NewSingularityAdlist<'a> {
    pub singularity_config_id: DbId,
    pub source: &'a str,
    pub format: &'a str,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[diesel(belongs_to(SingularityConfig))]
pub struct SingularityOutput {
    pub id: DbId,
    pub singularity_config_id: DbId,
    pub ty: String,
    pub destination: Vec<u8>,
    pub blackhole_address: String,
    pub deduplicate: bool,
}

#[derive(Insertable)]
#[diesel(table_name = singularity_outputs)]
pub struct NewSingularityOutput<'a> {
    pub singularity_config_id: DbId,
    pub ty: &'a str,
    pub destination: &'a [u8],
    pub blackhole_address: &'a str,
    pub deduplicate: bool,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[diesel(belongs_to(SingularityConfig))]
pub struct SingularityWhitelist {
    pub id: DbId,
    pub singularity_config_id: DbId,
    pub domain: String,
}

#[derive(Insertable)]
#[diesel(table_name = singularity_whitelists)]
pub struct NewSingularityWhitelist<'a> {
    pub singularity_config_id: DbId,
    pub domain: &'a str,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[diesel(belongs_to(SingularityOutput))]
pub struct SingularityOutputHostsInclude {
    pub id: DbId,
    pub singularity_output_id: DbId,
    pub path: Vec<u8>,
}

#[derive(Insertable)]
#[diesel(table_name = singularity_output_hosts_includes)]
pub struct NewSingularityOutputHostsInclude<'a> {
    pub singularity_output_id: DbId,
    pub path: &'a [u8],
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[diesel(table_name = singularity_output_pdns_lua, belongs_to(SingularityOutput))]
pub struct SingularityOutputPdnsLua {
    pub id: DbId,
    pub singularity_output_id: DbId,
    pub output_metric: bool,
    pub metric_name: String,
}

#[derive(Insertable)]
#[diesel(table_name = singularity_output_pdns_lua)]
pub struct NewSingularityOutputPdnsLua<'a> {
    pub singularity_output_id: DbId,
    pub output_metric: bool,
    pub metric_name: &'a str,
}

impl TryFrom<SingularityAdlist> for Adlist {
    type Error = anyhow::Error;

    fn try_from(mut model: SingularityAdlist) -> Result<Self, Self::Error> {
        model.format.make_ascii_lowercase();

        Ok(Self::new(
            &model.source,
            match model.format.as_str() {
                "hosts" => AdlistFormat::Hosts,
                "domains" => AdlistFormat::Domains,
                "dnsmasq" => AdlistFormat::Dnsmasq,
                _ => todo!(),
            },
        )?)
    }
}
