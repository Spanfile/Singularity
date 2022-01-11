use crate::database::DbId;
use maud::{html, Markup};

pub fn whitelist_card(whitelist: &[(DbId, String)]) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Whitelisted domains" }
            ."card-body" {
                a .btn ."btn-primary" href="/settings/singularity/add_whitelisted_domain" { "Add new whitelisted domain" }

                table .table ."mt-3" {
                    thead {
                        tr {
                            th scope="col" { "Domain name" }
                            th scope="col" ."w-auto" { }
                        }
                    }
                    tbody {
                        @for (id, domain) in whitelist {
                            tr {
                                // TODO: horizontal overflow to this element
                                td ."align-middle" { (domain) }
                                td {
                                    a .btn ."btn-danger" ."btn-sm" ."float-end" href={ "/settings/singularity/delete_whitelisted_domain?id=" (id) } { "Delete" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn add_new_whitelisted_domain() -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Add new whitelisted domain" }
            ."card-body" {
                form method="POST" {
                    ."mb-3" {
                        label ."form-label" for="domain" { "Domain name" }
                        input #domain ."form-control" name="domain" type="text";
                    }

                    button .btn ."btn-primary" ."me-3" type="submit" { "Add new whitelisted domain" }
                    a .btn ."btn-secondary" href="/settings/singularity" { "Cancel" }

                }
            }
        }
    }
}

pub fn delete_whitelisted_domain(id: DbId, domain: &str) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" ."bg-danger" ."text-white" { "Delete whitelisted domain" }
            ."card-body" {
                p ."card-text" { "Are you sure you want to delete this whitelisted domain? The operation is irreversible!" }
                p ."card-text" {
                    (domain)
                }

                form method="POST" {
                    input name="id" value=(id) type="hidden";
                    button .btn ."btn-danger" ."me-3" type="submit" { "Delete" }
                    a .btn ."btn-secondary" href="/settings/singularity" { "Cancel" }
                }
            }
        }
    }
}
