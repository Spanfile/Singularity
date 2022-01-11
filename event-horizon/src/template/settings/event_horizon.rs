mod configuration;
mod danger_zone;
mod import_singularity_config;

use maud::{html, Markup};

#[derive(PartialEq, Eq)]
pub enum EventHorizonSubPage<'a> {
    Main,
    ImportSingularityConfig,
    FinishConfigImport(&'a str),
}

pub fn event_horizon(sub: EventHorizonSubPage) -> Markup {
    match sub {
        EventHorizonSubPage::Main => main(),
        EventHorizonSubPage::ImportSingularityConfig => import_singularity_config::import_singularity_config(),
        EventHorizonSubPage::FinishConfigImport(rendered_str) => {
            import_singularity_config::finish_config_import(rendered_str)
        }
    }
}

fn main() -> Markup {
    html! {
        (configuration::config_card())
        (danger_zone::danger_zone_card())
    }
}
