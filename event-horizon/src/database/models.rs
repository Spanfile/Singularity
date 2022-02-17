use std::fmt::Display;

use super::{schema::*, DbId};
use crate::error::EvhError;
use diesel::{
    backend::{Backend, RawValue},
    deserialize::{self, FromSql},
    serialize::{self, ToSql},
    sql_types::Integer,
};
use singularity::{Adlist, AdlistFormat};

#[derive(Identifiable, Queryable, Insertable, PartialEq, Debug)]
pub struct EvhSetting {
    pub id: DbId,
    pub setting_type: EvhSettingType,
    pub value: String,
}

#[derive(Insertable)]
#[diesel(table_name = evh_settings)]
pub struct NewEvhSetting<'a> {
    pub setting_type: EvhSettingType,
    pub value: &'a str,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, AsExpression, FromSqlRow)]
#[diesel(sql_type = Integer)]
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
    pub result: SingularityRunHistoryResult,
}

#[derive(Insertable)]
#[diesel(table_name = singularity_run_histories)]
pub struct NewSingularityRunHistory<'a> {
    pub run_id: &'a str,
    pub timestamp: &'a str,
    pub result: SingularityRunHistoryResult,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, AsExpression, FromSqlRow)]
#[diesel(sql_type = Integer)]
pub enum SingularityRunHistoryResult {
    Success = 0,
    SuccessWithErrors = 1,
    SuccessWithWarnings = 2,
    Failed = 3,
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

impl<DB> ToSql<Integer, DB> for SingularityRunHistoryResult
where
    DB: Backend,
    DbId: ToSql<Integer, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, DB>) -> serialize::Result {
        match self {
            SingularityRunHistoryResult::Success => 0.to_sql(out),
            SingularityRunHistoryResult::SuccessWithErrors => 1.to_sql(out),
            SingularityRunHistoryResult::SuccessWithWarnings => 2.to_sql(out),
            SingularityRunHistoryResult::Failed => 3.to_sql(out),
        }
    }
}

impl<DB> FromSql<Integer, DB> for SingularityRunHistoryResult
where
    DB: Backend,
    DbId: FromSql<Integer, DB>,
{
    fn from_sql(bytes: RawValue<'_, DB>) -> deserialize::Result<Self> {
        match DbId::from_sql(bytes)? {
            0 => Ok(Self::Success),
            1 => Ok(Self::SuccessWithErrors),
            2 => Ok(Self::SuccessWithWarnings),
            3 => Ok(Self::Failed),
            i => Err(format!("Unrecognised SingularityRunHistoryResult variant {}", i).into()),
        }
    }
}

impl Display for SingularityRunHistoryResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SingularityRunHistoryResult::Success => write!(f, "Success"),
            SingularityRunHistoryResult::SuccessWithErrors => write!(f, "Success with errors"),
            SingularityRunHistoryResult::SuccessWithWarnings => write!(f, "Success with warnings"),
            SingularityRunHistoryResult::Failed => write!(f, "Failed"),
        }
    }
}

impl<DB> ToSql<Integer, DB> for EvhSettingType
where
    DB: Backend,
    DbId: ToSql<Integer, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, DB>) -> serialize::Result {
        match self {
            Self::ActiveSingularityConfig => 0.to_sql(out),
        }
    }
}

impl<DB> FromSql<Integer, DB> for EvhSettingType
where
    DB: Backend,
    DbId: FromSql<Integer, DB>,
{
    fn from_sql(bytes: RawValue<'_, DB>) -> deserialize::Result<Self> {
        match DbId::from_sql(bytes)? {
            0 => Ok(Self::ActiveSingularityConfig),
            i => Err(format!("Unrecognised EvhSettingType variant {}", i).into()),
        }
    }
}
