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
