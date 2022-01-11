use maud::{html, Markup};

pub fn general_card() -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "General" }
            ."card-body" {
                // add settings for:
                // - timing
                // - selecting a config

                a .btn ."btn-primary" href="/settings/event_horizon/import_singularity_config" { "Import Singularity config" }
            }
        }
    }
}

pub fn import_singularity_config() -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Import Singularity config" }
            ."card-body" {
                form method="POST" enctype="multipart/form-data" {
                    ."mb-3" {
                        label ."form-label" for="file" { "Import config from file" }
                        input ."form-control" #file name="file" type="file" accept=".toml,application/toml" required;
                    }

                    button ."btn" ."btn-primary" type="submit" { "Import" }
                }

                p ."mt-3" { "Or" }

                form method="POST" {
                    ."mb-3" {
                        label ."form-label" for="text" { "Paste config text" }
                        textarea ."form-control" #text name="text" rows="5" {}
                    }

                    button ."btn" ."btn-primary" type="submit" { "Import" }
                }
            }
        }
    }
}
