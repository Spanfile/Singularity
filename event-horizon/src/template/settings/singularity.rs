use crate::singularity::SingularityConfig;
use maud::{html, Markup};
use singularity::{Output, OutputType};

#[derive(PartialEq, Eq)]
pub enum SingularitySubPage {
    Main,
    AddNewAdlist,
    RemoveAdlist(u64),
}

pub fn singularity(sub_page: SingularitySubPage, singularity_config: &SingularityConfig) -> Markup {
    match sub_page {
        SingularitySubPage::Main => main(singularity_config),
        SingularitySubPage::AddNewAdlist => add_new_adlist(),
        SingularitySubPage::RemoveAdlist(id) => remove_adlist(id, singularity_config),
    }
}

fn main(cfg: &SingularityConfig) -> Markup {
    html! {
            (general_card(cfg))
            (adlists_card(cfg))
            (outputs_card(cfg))
    }
}

fn general_card(cfg: &SingularityConfig) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "General" }
            ."card-body" {
                // add settings for:
                // - whitelists
            }
        }
    }
}

fn adlists_card(cfg: &SingularityConfig) -> Markup {
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

fn outputs_card(cfg: &SingularityConfig) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Outputs" }
            ."card-body" {
                .row ."g-3" {
                    ."col-auto" {
                        a .btn ."btn-primary" href="/settings/singularity/add_new_pdns_lua_output" { "Add new PDNS Lua script output" }
                    }

                    ."col-auto" {
                        a .btn ."btn-primary" href="/settings/singularity/add_new_hosts_output" { "Add new hosts-file output" }
                    }
                }

                ."list-group" ."mt-3" {
                    @for (id, output) in cfg.outputs() {
                        (single_output_card(id, output))
                    }
                }
            }
        }
    }
}

fn single_output_card(id: u64, output: &Output) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" ."container-fluid" {
                .row ."g-3" {
                    ."col-auto" ."me-auto" ."d-flex" ."align-items-center" { (output.ty) " - " (output.destination.display()) }
                    ."col-auto" {
                        a ."btn" ."btn-primary" ."btn-sm" ."mb-auto" href={ "/settings/singularity/edit_output?id=" (id) } { "Edit" }
                    }
                    ."col-auto" {
                        a ."btn" ."btn-danger" ."btn-sm" href={ "/settings/singularity/delete_output?id=" (id) } { "Delete" }
                    }
                }
            }

            ."card-body" {
                .row {
                    ."col-md" {
                        dl .row {
                            dt ."col-lg-6" { "Blackhole address" }
                            dd ."col-lg-6" { (output.blackhole_address) }

                            dt ."col-lg-6" { "Deduplicate" }
                            dd ."col-lg-6" { (output.deduplicate) }
                        }
                    }

                    ."col-md" {
                        dl .row {
                            @match &output.ty {
                                OutputType::Hosts { include } => {
                                    dt ."col-xl-12" { "Included files" }
                                    dd ."col-xl-12" {
                                        ul ."list-group" ."list-group-flush" {
                                            @for file in include {
                                                li ."list-group-item" { (file.display()) }
                                            }
                                        }
                                    }
                                },
                                OutputType::PdnsLua { output_metric, metric_name } => {
                                    dt ."col-lg-6" { "Metric enabled" }
                                    dd ."col-lg-6" { (output_metric) }

                                    dt ."col-lg-6" { "Metric name" }
                                    dd ."col-lg-6" { (metric_name) }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

fn add_new_adlist() -> Markup {
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

fn remove_adlist(id: u64, cfg: &SingularityConfig) -> Markup {
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
