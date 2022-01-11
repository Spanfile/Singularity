use crate::config::EvhConfig;
use maud::{html, Markup};

pub fn danger_zone_card(evh_config: &EvhConfig) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" ."bg-danger" ."text-white" { "Danger zone" }
            ."card-body" {
                p { "These options are internal and critical to Event Horizon's functionality. You probably shouldn't \
                    edit them. If you do, you'll have to restart Event Horizon for changes to apply." }

                form method="POST" {
                    .row ."mb-3" {
                        label ."col-form-label" ."col-lg-3" for="database_url" { "Database URL" }
                        ."col-lg-9" {
                            input #database_url ."form-control" name="database_url" type="text" value=(evh_config.database_url);
                        }
                    }

                    .row ."mb-3" {
                        label ."col-form-label" ."col-lg-3" for="max_concurrent_imports" { "Max. concurrent imports" }
                        ."col-lg-9" {
                            input #max_concurrent_imports ."form-control" name="max_concurrent_imports" type="number"
                                value=(evh_config.max_concurrent_imports);
                        }
                    }

                    .row ."mb-3" {
                        label ."col-form-label" ."col-lg-3" for="max_import_lifetime" { "Max. import lifetime" }
                        ."col-lg-9" {
                            input #max_import_lifetime ."form-control" name="max_import_lifetime" type="number"
                                value=(evh_config.max_import_lifetime);
                        }
                    }

                    button .btn ."btn-outline-danger" type="submit" { "Save" }
                }
            }
        }
    }
}
