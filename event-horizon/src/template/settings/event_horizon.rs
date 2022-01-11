mod danger_zone;
mod general;

use maud::{html, Markup};

#[derive(PartialEq, Eq)]
pub enum EventHorizonSubPage {
    Main,
    ImportSingularityConfig,
}

pub fn event_horizon(sub: EventHorizonSubPage) -> Markup {
    match sub {
        EventHorizonSubPage::Main => main(),
        EventHorizonSubPage::ImportSingularityConfig => general::import_singularity_config(),
    }
}

fn main() -> Markup {
    html! {
        (general::general_card())
        (danger_zone::danger_zone_card())
    }
}
