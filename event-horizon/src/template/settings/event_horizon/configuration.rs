use maud::{html, Markup};

pub fn config_card() -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Available Singularity configurations" }
            ."card-body" {
                // TODO: add settings for:
                // - timing
                // - selecting a config

                a .btn ."btn-primary" href="/settings/event_horizon/import_singularity_config" {
                    "Import Singularity configuration"
                }
            }
        }
    }
}
