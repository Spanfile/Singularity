table! {
    singularity_adlists (id) {
        id -> Integer,
        singularity_config_id -> Integer,
        source -> Text,
        format -> Text,
    }
}

table! {
    singularity_configs (id) {
        id -> Integer,
        name -> Text,
        dirty -> Bool,
        http_timeout -> Integer,
    }
}

table! {
    singularity_output_hosts_includes (id) {
        id -> Integer,
        singularity_output_id -> Integer,
        path -> Binary,
    }
}

table! {
    singularity_output_pdns_lua (id) {
        id -> Integer,
        singularity_output_id -> Integer,
        output_metric -> Bool,
        metric_name -> Text,
    }
}

table! {
    singularity_outputs (id) {
        id -> Integer,
        singularity_config_id -> Integer,
        ty -> Text,
        destination -> Binary,
        blackhole_address -> Text,
        deduplicate -> Bool,
    }
}

table! {
    singularity_whitelists (id) {
        id -> Integer,
        singularity_config_id -> Integer,
        domain -> Text,
    }
}

joinable!(singularity_adlists -> singularity_configs (singularity_config_id));
joinable!(singularity_output_hosts_includes -> singularity_outputs (singularity_output_id));
joinable!(singularity_output_pdns_lua -> singularity_outputs (singularity_output_id));
joinable!(singularity_outputs -> singularity_configs (singularity_config_id));
joinable!(singularity_whitelists -> singularity_configs (singularity_config_id));

allow_tables_to_appear_in_same_query!(
    singularity_adlists,
    singularity_configs,
    singularity_output_hosts_includes,
    singularity_output_pdns_lua,
    singularity_outputs,
    singularity_whitelists,
);
