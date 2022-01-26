use maud::{html, Markup};

pub fn use_singularity_config(name: Option<&str>) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Use this Singularity configuration?" }
            ."card-body" {
                form method="POST" {
                    input ."form-control-plaintext" ."mb-3" type="text" value=(name.unwrap_or("")) readonly;

                    button .btn ."btn-primary" ."me-3" type="submit" disabled[name.is_none()] { "Use" }
                    a .btn ."btn-secondary" href="/settings/event_horizon" { "Cancel" }
                }
            }
        }
    }
}

pub fn rename_singularity_config(name: Option<&str>) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Rename a Singularity configuration" }
            ."card-body" {
                form method="POST" {
                    .row ."mb-3" {
                        label ."col-sm-2" ."col-form-label" for="previousName" { "Previous name" }
                        ."col-sm-10" {
                            input ."form-control-plaintext" #previousName type="text" value=(name.unwrap_or("")) readonly;
                        }
                    }

                    .row ."mb-3" {
                        label ."col-sm-2" ."col-form-label" for="name" { "New name" }
                        ."col-sm-10" {
                            input ."form-control" #name name="name" type="text" required;
                        }
                    }

                    button .btn ."btn-primary" ."me-3" type="submit" disabled[name.is_none()] { "Rename" }
                    a .btn ."btn-secondary" href="/settings/event_horizon" { "Cancel" }
                }
            }
        }
    }
}

pub fn delete_singularity_config(name: Option<&str>) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" ."bg-danger" ."text-white" { "Delete this Singularity configuration?" }
            ."card-body" {
                p { "This action is irreversable!" }

                form method="POST" {
                    input ."form-control-plaintext" ."mb-3" type="text" value=(name.unwrap_or("")) readonly;

                    button .btn ."btn-danger" ."me-3" type="submit" disabled[name.is_none()] { "Delete" }
                    a .btn ."btn-secondary" href="/settings/event_horizon" { "Cancel" }
                }
            }
        }
    }
}
