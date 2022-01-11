mod danger_zone;
mod general;

use maud::{html, Markup};

pub fn event_horizon() -> Markup {
    html! {
        (general::general_card())
        (danger_zone::danger_zone_card())
    }
}
