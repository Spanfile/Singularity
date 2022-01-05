use crate::singularity::SingularityConfig;
use maud::{html, Markup};

#[derive(PartialEq, Eq)]
pub enum SingularitySubPage {
    Main,
    AddNewAdlist,
}

pub fn singularity(sub_page: SingularitySubPage, singularity_config: &SingularityConfig) -> Markup {
    match sub_page {
        SingularitySubPage::Main => main(singularity_config),
        SingularitySubPage::AddNewAdlist => add_new_adlist(),
    }
}

fn main(singularity_config: &SingularityConfig) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "General" }
            ."card-body" {
                // add settings for:
                // - whitelists
            }
        }

        .card ."w-100" ."mb-3" {
            ."card-header" { "Adlists" }
            ."card-body" {
                a .btn ."btn-primary" href="singularity/add_new_adlist" { "Add new adlist" }

                table .table ."mt-3" {
                    thead {
                        tr {
                            th scope="col" { "Source URL" }
                            th scope="col" { "Format" }
                            th scope="col" ."w-auto" { }
                        }
                    }
                    tbody {
                        @for (_, adlist) in singularity_config.adlists() {
                            tr {
                                // TODO: horizontal overflow to this element
                                td ."align-middle" {a href=(adlist.source()) { (adlist.source()) } }
                                td ."align-middle" { (adlist.format()) }
                                td {
                                    form method="POST" {
                                        input name="submitted_form" value="remove_adlist" type="hidden";
                                        input name="source" value=(adlist.source()) type="hidden";
                                        button .btn ."btn-danger" ."btn-sm" ."float-end" type="submit" { "Delete" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        .card ."w-100" ."mb-3" {
            ."card-header" { "Outputs" }
            ."card-body" { }
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
