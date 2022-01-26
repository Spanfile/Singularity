use crate::singularity::SingularityConfig;
use maud::{html, Markup};

pub fn config_card(cfgs: Option<&[(String, SingularityConfig)]>) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            // TODO: add settings for:
            // - timing
            // - selecting a config

            ."card-header" { "Available Singularity configurations" }
            ."card-body" {
                a .btn ."btn-primary" href="/settings/event_horizon/import_singularity_config" {
                    "Import Singularity configuration"
                }

                table .table ."mt-3" {
                    thead {
                        tr {
                            th scope="col" { "Name" }
                            th scope="col" ."w-auto" { }
                        }
                    }
                    tbody {
                        @if let Some(cfgs) = cfgs {
                            @for (name, cfg) in cfgs {
                                tr {
                                    td ."align-middle" { (name) }
                                    td {
                                        a .btn ."btn-outline-danger" ."btn-sm" ."float-end" href={
                                            "/settings/event_horizon/delete_singularity_config?id=" (cfg.id())
                                        } { "Delete" }

                                        a .btn ."btn-primary" ."btn-sm" ."float-end" ."me-3" href={
                                            "/settings/event_horizon/use_singularity_config?id=" (cfg.id())
                                        } { "Use" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
