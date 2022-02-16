use super::{schema::*, DbId};
use crate::error::EvhError;
use singularity::{Adlist, AdlistFormat};

#[derive(Identifiable, Queryable, Insertable, PartialEq, Debug)]
pub struct EvhSetting {
    pub id: DbId,
    pub setting_type: DbId,
    pub value: String,
}

#[derive(Insertable)]
#[diesel(table_name = evh_settings)]
pub struct NewEvhSetting<'a> {
    pub setting_type: DbId,
    pub value: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EvhSettingType {
    ActiveSingularityConfig = 0,
}

#[derive(Identifiable, Queryable, PartialEq, Debug)]
pub struct SingularityConfig {
    pub id: DbId,
    pub name: String,
    pub dirty: bool,
    pub http_timeout: i32,
    pub timing: String,
    pub last_run: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = singularity_configs)]
pub struct NewSingularityConfig<'a> {
    pub name: &'a str,
    pub dirty: bool,
    pub http_timeout: i32,
    pub timing: &'a str,
    pub last_run: Option<&'a str>,
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
    pub builtin: bool,
}

#[derive(Insertable)]
#[diesel(table_name = singularity_outputs)]
pub struct NewSingularityOutput<'a> {
    pub singularity_config_id: DbId,
    pub ty: &'a str,
    pub destination: &'a [u8],
    pub blackhole_address: &'a str,
    pub deduplicate: bool,
    pub builtin: bool,
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

#[derive(Identifiable, Queryable, PartialEq, Debug)]
#[diesel(table_name = singularity_run_histories, primary_key(run_id))]
pub struct SingularityRunHistory {
    pub run_id: String,
    pub timestamp: String,
}

#[derive(Insertable)]
#[diesel(table_name = singularity_run_histories)]
pub struct NewSingularityRunHistory<'a> {
    pub run_id: &'a str,
    pub timestamp: &'a str,
}

impl TryFrom<SingularityAdlist> for Adlist {
    type Error = EvhError;

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
