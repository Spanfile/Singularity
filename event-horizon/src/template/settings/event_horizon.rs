mod configuration;
mod danger_zone;
mod import_singularity_config;
mod modify_singularity_config;

use crate::{
    config::{EnvConfig, EvhConfig},
    database::DbId,
    singularity::SingularityConfig,
};
use maud::{html, Markup};

pub enum EventHorizonSubPage<'a> {
    Main {
        cfgs: Option<&'a [(String, SingularityConfig)]>,
        active_cfg: DbId,
    },
    DangerZone {
        evh_config: &'a EvhConfig,
        env_config: &'a EnvConfig,
    },
    ImportSingularityConfig,
    FinishConfigImport(Option<(&'a str, &'a str)>),
    UseSingularityConfig(Option<&'a str>),
    RenameSingularityConfig(Option<&'a str>),
    DeleteSingularityConfig(Option<&'a str>),
}

pub fn event_horizon(sub: EventHorizonSubPage) -> Markup {
    match sub {
        EventHorizonSubPage::Main { cfgs, active_cfg } => main(cfgs, active_cfg),
        EventHorizonSubPage::DangerZone { evh_config, env_config } => danger_zone::danger_zone(evh_config, env_config),
        EventHorizonSubPage::ImportSingularityConfig => import_singularity_config::import_singularity_config(),
        EventHorizonSubPage::FinishConfigImport(rendered_cfg) => {
            import_singularity_config::finish_config_import(rendered_cfg)
        }
        EventHorizonSubPage::UseSingularityConfig(name) => modify_singularity_config::use_singularity_config(name),
        EventHorizonSubPage::RenameSingularityConfig(name) => {
            modify_singularity_config::rename_singularity_config(name)
        }
        EventHorizonSubPage::DeleteSingularityConfig(name) => {
            modify_singularity_config::delete_singularity_config(name)
        }
    }
}

fn main(cfgs: Option<&[(String, SingularityConfig)]>, active_cfg: DbId) -> Markup {
    html! {
        (configuration::config_card(cfgs, active_cfg))

        a href="/settings/event_horizon/danger_zone" { "Danger zone" }
    }
}
