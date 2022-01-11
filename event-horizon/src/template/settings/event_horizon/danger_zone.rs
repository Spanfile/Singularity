use maud::{html, Markup};

pub fn danger_zone_card() -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" ."bg-danger" ."text-white" { "Danger zone" }
            ."card-body" {
                p { "These options are internal and critical to Event Horizon's functionality. You probably shouldn't \
                    edit them. If you do, you'll have to restart Event Horizon afterwards." }
                // display things in EvhConfig, and relevant environment variables
            }
        }
    }
}
