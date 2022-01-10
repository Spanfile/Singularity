use super::schema::*;

#[derive(Identifiable, Queryable, PartialEq, Debug)]
pub struct SingularityConfig {
    pub id: i32,
    pub dirty: bool,
    pub http_timeout: i32,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[belongs_to(SingularityConfig)]
pub struct SingularityAdlist {
    pub id: i32,
    pub singularity_config_id: i32,
    pub source: String,
    pub format: String,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[belongs_to(SingularityConfig)]
pub struct SingularityOutput {
    pub id: i32,
    pub singularity_config_id: i32,
    pub ty: String,
    pub destination: String,
    pub blackhole_address: String,
    pub deduplicate: bool,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[belongs_to(SingularityConfig)]
pub struct SingularityWhitelist {
    pub id: i32,
    pub singularity_config_id: i32,
    pub domain: String,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[belongs_to(SingularityOutput)]
pub struct SingularityOutputHostsInclude {
    pub id: i32,
    pub singularity_output_id: i32,
    pub path: String,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[belongs_to(SingularityOutput)]
#[table_name = "singularity_output_pdns_lua"]
pub struct SingularityOutputPdnsLua {
    pub id: i32,
    pub singularity_output_id: i32,
    pub output_metric: bool,
    pub metric_name: String,
}
