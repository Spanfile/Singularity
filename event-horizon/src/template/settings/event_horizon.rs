mod configuration;
mod danger_zone;
mod import_singularity_config;

use crate::config::EvhConfig;
use maud::{html, Markup};

pub enum EventHorizonSubPage<'a> {
    Main,
    DangerZone { evh_config: &'a EvhConfig },
    ImportSingularityConfig,
    FinishConfigImport(&'a str),
}

pub fn event_horizon(sub: EventHorizonSubPage) -> Markup {
    match sub {
        EventHorizonSubPage::Main => main(),
        EventHorizonSubPage::DangerZone { evh_config } => danger_zone::danger_zone(evh_config),
        EventHorizonSubPage::ImportSingularityConfig => import_singularity_config::import_singularity_config(),
        EventHorizonSubPage::FinishConfigImport(rendered_str) => {
            import_singularity_config::finish_config_import(rendered_str)
        }
    }
}

fn main() -> Markup {
    html! {
        (configuration::config_card())

        a href="/settings/event_horizon/danger_zone" { "Danger zone" }
    }
}
