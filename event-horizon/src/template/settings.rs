use super::ResponseBuilder;
use maud::{html, Markup};

#[derive(PartialEq, Eq)]
pub enum SettingsPage {
    EventHorizon,
    Singularity,
    Recursor,
}

pub fn settings(page: SettingsPage) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        .row {
            ."col-lg-2" {
                nav .nav ."nav-pills" ."flex-column" {
                    a ."nav-link" .active[page == SettingsPage::EventHorizon] href="/settings/eventhorizon" { "Event Horizon" }
                    a ."nav-link" .active[page == SettingsPage::Singularity] href="/settings/singularity" { "Singularity" }
                    a ."nav-link" .active[page == SettingsPage::Recursor] href="/settings/recursor" { "PDNS Recursor" }
                }
            }

            ."col-lg-10" {
                @match page {
                    SettingsPage::EventHorizon => (event_horizon()),
                    SettingsPage::Singularity => (singularity()),
                    SettingsPage::Recursor => (recursor()),
                }
            }
        }
    })
    .current_path("/settings")
}

fn event_horizon() -> Markup {
    html! {
        p { "Event Horizon settings" }
    }
}

fn singularity() -> Markup {
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
                form method="POST" {
                    input name="submitted_form" value="add_adlist" type="hidden";
                    .row ."g-3" ."align-items-center" {
                        ."col-auto" {
                            label ."col-form-label" for="source" { "Source URL" }
                        }
                        ."col" {
                            input ."form-control" #source name="source" type="text";
                        }

                        ."col-auto" {
                            label ."col-form-label" for="format" { "Format" }
                        }
                        ."col-auto" {
                            select ."form-select" #format name="format" {
                                option selected value="hosts" { "Hosts" }
                                option value="dnsmasq" { "dnsmasq" }
                                option value="domains" { "Domains" }
                            }
                        }

                        ."col-auto" {
                            button .btn ."btn-primary" type="submit" { "Add new adlist" }
                        }
                    }
                }

                table .table ."mt-3" {
                    thead {
                        tr {
                            th scope="col" { "Source URL" }
                            th scope="col" { "Format" }
                            th scope="col" ."w-auto" { }
                        }
                    }
                    tbody {
                        tr {
                            td ."align-middle" { a href="https://penis" { "https://penis" } }
                            td ."align-middle" { "Hosts" }
                            td {
                                form method="POST" {
                                    input name="submitted_form" value="remove_adlist" type="hidden";
                                    input name="source" value="https://penis" type="hidden";
                                    button .btn ."btn-danger" ."btn-sm" ."float-end" type="submit" { "Delete" }
                                }
                            }
                        }
                        tr {
                            td ."align-middle" { a href="https://dick" { "https://dick" } }
                            td ."align-middle" { "dnsmasq" }
                            td {
                                form method="POST" {
                                    input name="submitted_form" value="remove_adlist" type="hidden";
                                    input name="source" value="https://dick" type="hidden";
                                    button .btn ."btn-danger" ."btn-sm" ."float-end" type="submit" { "Delete" }
                                }
                            }
                        }
                    }
                }
            }
        }

        .card ."w-100" ."mb-3" {
            ."card-header" { "Outputs" }
            ."card-body" {

            }
        }
    }
}

fn recursor() -> Markup {
    // things to have settings for:
    //

    html! {
        p { "PDNS Recursor settings" }
    }
}
