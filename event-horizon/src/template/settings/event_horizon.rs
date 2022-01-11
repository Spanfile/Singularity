mod danger_zone;
mod general;
mod import_singularity_config;

use maud::{html, Markup};

#[derive(PartialEq, Eq)]
pub enum EventHorizonSubPage {
    Main,
    ImportSingularityConfig,
    FinishConfigImport,
}

pub fn event_horizon(sub: EventHorizonSubPage) -> Markup {
    match sub {
        EventHorizonSubPage::Main => main(),
        EventHorizonSubPage::ImportSingularityConfig => import_singularity_config::import_singularity_config(),
        EventHorizonSubPage::FinishConfigImport => import_singularity_config::finish_config_import(),
    }
}

fn main() -> Markup {
    html! {
        (general::general_card())
        (danger_zone::danger_zone_card())
    }
}
