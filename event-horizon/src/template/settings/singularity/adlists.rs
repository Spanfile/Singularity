use crate::singularity::SingularityConfig;
use maud::{html, Markup};

pub fn adlists_card(cfg: &SingularityConfig) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Adlists" }
            ."card-body" {
                a .btn ."btn-primary" href="/settings/singularity/add_new_adlist" { "Add new adlist" }

                table .table ."mt-3" {
                    thead {
                        tr {
                            th scope="col" { "Source URL" }
                            th scope="col" { "Format" }
                            th scope="col" ."w-auto" { }
                        }
                    }
                    tbody {
                        @for (id, adlist) in cfg.adlists() {
                            tr {
                                // TODO: horizontal overflow to this element
                                td ."align-middle" {a href=(adlist.source()) { (adlist.source()) } }
                                td ."align-middle" { (adlist.format()) }
                                td {
                                    a .btn ."btn-danger" ."btn-sm" ."float-end" href={ "/settings/singularity/remove_adlist?id=" (id) } { "Delete" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn add_new_adlist() -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Add new adlist" }
            ."card-body" {
                form method="POST" {
                    ."mb-3" {
                        label ."form-label" for="source" { "Source URL" }
                        input #source ."form-control" name="source" type="text";
                    }

                    ."mb-3" {
                        label ."form-label" for="format" { "Format" }
                        select #format ."form-select" name="format" {
                            option selected value="hosts" { "Hosts" }
                            option value="domains" { "Domains" }
                            option value="dnsmasq" { "Dnsmasq" }
                        }
                    }

                    button .btn ."btn-primary" ."me-3" type="submit" { "Add new adlist" }
                    a .btn ."btn-secondary" href="/settings/singularity" { "Cancel" }

                }
            }
        }
    }
}

pub fn remove_adlist(id: u64, cfg: &SingularityConfig) -> Markup {
    let adlist = cfg.get_adlist(id).expect("no adlist found when generating template");

    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" ."bg-danger" ."text-white" { "Remove adlist" }
            ."card-body" {
                p ."card-text" { "Are you sure you want to delete this adlist? The operation is irreversible!" }
                p ."card-text" {
                    a href=(adlist.source()) { (adlist.source()) }
                }

                form method="POST" {
                    input name="source" value=(id) type="hidden";
                    button .btn ."btn-danger" ."me-3" type="submit" { "Delete" }
                    a .btn ."btn-secondary" href="/settings/singularity" { "Cancel" }
                }
            }
        }
    }
}
