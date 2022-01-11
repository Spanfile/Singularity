mod event_horizon;
mod recursor;
mod singularity;

pub use self::singularity::SingularitySubPage;

use super::ResponseBuilder;
use maud::html;

#[derive(PartialEq, Eq)]
pub enum SettingsPage<'a> {
    EventHorizon,
    Singularity(SingularitySubPage<'a>),
    Recursor,
}

pub fn settings(page: SettingsPage) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        .row {
            ."col-lg-2" {
                nav .nav ."nav-pills" ."flex-column" {
                    a ."nav-link" .active[page == SettingsPage::EventHorizon] href="/settings/event_horizon" { "Event Horizon" }
                    a ."nav-link" .active[matches!(page, SettingsPage::Singularity(_))] href="/settings/singularity" { "Singularity" }
                    a ."nav-link" .active[page == SettingsPage::Recursor] href="/settings/recursor" { "PDNS Recursor" }
                }
            }

            ."col-lg-10" {
                @match page {
                    SettingsPage::EventHorizon => (event_horizon::event_horizon()),
                    SettingsPage::Singularity(sub) => (singularity::singularity(sub)),
                    SettingsPage::Recursor => (recursor::recursor()),
                }
            }
        }
    })
    .current_path("/settings")
}
