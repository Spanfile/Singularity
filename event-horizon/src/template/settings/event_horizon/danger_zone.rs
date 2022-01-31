use crate::config::{EnvConfig, EvhConfig};
use maud::{html, Markup};

pub fn danger_zone(evh_config: &EvhConfig, env_config: &EnvConfig) -> Markup {
    html! {
        .card ."border-danger" ."w-100" ."mb-3" {
            ."card-header" ."bg-danger" ."text-white" { "Danger zone" }
            ."card-body" {
                p { "These options are internal and critical to Event Horizon's functionality. You probably shouldn't \
                    edit them. If you do, you'll have to restart Event Horizon for changes to apply." }

                form method="POST" {
                    .row ."mb-3" {
                        label ."col-form-label" ."col-lg-3" for="config_file_location" { "Configuration file location" }
                        ."col-lg-9" {
                            input #config_file_location ."form-control-plaintext" name="config_file_location" type="text"
                                value=(env_config.config.display()) readonly;
                        }
                    }

                    .row ."mb-3" {
                        label ."col-form-label" ."col-lg-3" for="database_url" { "Database URL" }
                        ."col-lg-9" {
                            input #database_url ."form-control" name="database_url" type="text" value=(evh_config.database_url);
                        }
                    }

                    (redis_card(evh_config))
                    (recursor_card(evh_config))

                    button .btn ."btn-outline-danger" type="submit" { "Save" }
                }
            }
        }
    }
}

fn redis_card(evh_config: &EvhConfig) -> Markup {
    html! {
        .card ."border-dark" ."w-100" ."mb-3" {
            ."card-header" { "Redis" }
            ."card-body" {
                .row ."mb-3" {
                    label ."col-form-label" ."col-lg-3" for="redis_url" { "Redis URL" }
                    ."col-lg-9" {
                        input #redis_url ."form-control" name="redis_url" type="text" value=(evh_config.redis.redis_url);
                    }
                }

                .row ."mb-3" {
                    label ."col-form-label" ."col-lg-3" for="max_concurrent_imports" { "Max. concurrent imports" }
                    ."col-lg-9" {
                        input #max_concurrent_imports ."form-control" name="max_concurrent_imports" type="number"
                            value=(evh_config.redis.max_concurrent_imports);
                    }
                }

                .row ."mb-3" {
                    label ."col-form-label" ."col-lg-3" for="max_import_lifetime" { "Max. import lifetime" }
                    ."col-lg-9" {
                        input #max_import_lifetime ."form-control" name="max_import_lifetime" type="number"
                            value=(evh_config.redis.max_import_lifetime);
                    }
                }
            }
        }
    }
}

fn recursor_card(evh_config: &EvhConfig) -> Markup {
    html! {
        .card ."border-dark" ."w-100" ."mb-3" {
            ."card-header" { "PDNS Recursor" }
            ."card-body" {
                .row ."mb-3" {
                    label ."col-form-label" ."col-lg-3" for="controlSocket" { "Recursor control socket" }
                    ."col-lg-9" {
                        input #controlSocket ."form-control" name="control_socket" type="text"
                            value=(evh_config.recursor.control_socket.display());
                    }
                }
            }
        }
    }
}
