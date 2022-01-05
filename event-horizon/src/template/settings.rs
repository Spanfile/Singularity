mod singularity;

pub use self::singularity::SingularitySubPage;

use super::ResponseBuilder;
use crate::singularity::SingularityConfig;
use maud::{html, Markup};

#[derive(PartialEq, Eq)]
pub enum SettingsPage {
    EventHorizon,
    Singularity(SingularitySubPage),
    Recursor,
}

pub fn settings(page: SettingsPage, singularity_config: &SingularityConfig) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        .row {
            ."col-lg-2" {
                nav .nav ."nav-pills" ."flex-column" {
                    a ."nav-link" .active[page == SettingsPage::EventHorizon] href="/settings/eventhorizon" { "Event Horizon" }
                    a ."nav-link" .active[matches!(page, SettingsPage::Singularity(_))] href="/settings/singularity" { "Singularity" }
                    a ."nav-link" .active[page == SettingsPage::Recursor] href="/settings/recursor" { "PDNS Recursor" }
                }
            }

            ."col-lg-10" {
                @match page {
                    SettingsPage::EventHorizon => (event_horizon()),
                    SettingsPage::Singularity(sub) => (singularity::singularity(sub, singularity_config)),
                    SettingsPage::Recursor => (recursor()),
                }
            }
        }
    })
    .current_path("/settings")
}

fn event_horizon() -> Markup {
    html! {
        p { "Event Horizon settings" }
    }
}

fn recursor() -> Markup {
    // things to have settings for:
    //

    html! {
        p { "PDNS Recursor settings" }
    }
}
