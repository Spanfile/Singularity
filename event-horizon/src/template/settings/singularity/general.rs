use maud::{html, Markup};

pub fn general_card() -> Markup {
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
