mod configuration;
mod danger_zone;
mod import_singularity_config;

use crate::config::EvhConfig;
use maud::{html, Markup};

pub enum EventHorizonSubPage<'a> {
    Main { evh_config: &'a EvhConfig },
    ImportSingularityConfig,
    FinishConfigImport(&'a str),
}

pub fn event_horizon(sub: EventHorizonSubPage) -> Markup {
    match sub {
        EventHorizonSubPage::Main { evh_config } => main(evh_config),
        EventHorizonSubPage::ImportSingularityConfig => import_singularity_config::import_singularity_config(),
        EventHorizonSubPage::FinishConfigImport(rendered_str) => {
            import_singularity_config::finish_config_import(rendered_str)
        }
    }
}

fn main(evh_config: &EvhConfig) -> Markup {
    html! {
        (configuration::config_card())
        (danger_zone::danger_zone_card(evh_config))
    }
}
