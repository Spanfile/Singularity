use crate::{database::DbId, singularity::SingularityConfig};
use maud::{html, Markup};

pub fn config_card(cfgs: Option<&[(String, SingularityConfig)]>, active_cfg: DbId) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
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
                                @if cfg.id() == active_cfg {
                                    (active_config_table_row(name, cfg))
                                } @else {
                                    (config_table_row(name, cfg))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn active_config_table_row(name: &str, cfg: &SingularityConfig) -> Markup {
    html! {
        tr {
            td ."align-middle" { strong { "Active: " } (name) }
            td ."d-flex" {
                // this element is used to push the other elements to the right
                ."me-auto" {}

                a .btn ."btn-primary" ."btn-sm" .disabled ."me-3" ."px-4" href="#" { "Use" }

                a .btn ."btn-primary" ."btn-sm" ."me-3" href={
                    "/settings/event_horizon/rename_singularity_config?id=" (cfg.id())
                } { "Rename" }

                a .btn ."btn-outline-danger" ."btn-sm" .disabled href="#" { "Delete" }
            }
        }
    }
}

fn config_table_row(name: &str, cfg: &SingularityConfig) -> Markup {
    html! {
        tr {
            td ."align-middle" { (name) }
            td ."d-flex" {
                // this element is used to push the other elements to the right
                ."me-auto" {}

                a .btn ."btn-primary" ."btn-sm" ."me-3" ."px-4" href={
                    "/settings/event_horizon/use_singularity_config?id=" (cfg.id())
                } { "Use" }

                a .btn ."btn-primary" ."btn-sm" ."me-3" href={
                    "/settings/event_horizon/rename_singularity_config?id=" (cfg.id())
                } { "Rename" }

                a .btn ."btn-outline-danger" ."btn-sm" href={
                    "/settings/event_horizon/delete_singularity_config?id=" (cfg.id())
                } { "Delete" }
            }
        }
    }
}
