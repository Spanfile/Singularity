mod history;

use super::ResponseBuilder;
use crate::{
    singularity::runner::history::{HistoryEvent, RunnerHistory},
    template::DATETIME_FORMAT,
    util::round_duration::RoundDuration,
};
use chrono::{DateTime, Local};
use maud::{html, Markup};

pub fn singularity_page(
    last_run: Option<DateTime<Local>>,
    next_run: DateTime<Local>,
    currently_running: bool,
) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        (singularity_last_run(last_run, next_run))
        p {
            "You may edit the run schedule in the "
            a href="/settings/singularity" { "Singularity settings." }
        }

        @if currently_running {
            p {
                "Singularity is currently running. "
                a href="/singularity/run" { "See its status here." }
            }
        } @else {
            (singularity_run_now_button())
            br;
        }

        a href="/singularity/history" { "Previous run history" }
    })
    .current_path("/singularity")
}

pub fn singularity_running() -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        p { "Singularity is running. Please refresh this page in a moment to see its result." }
    })
    .current_path("/singularity")
}

pub fn singularity_finished(timestamp: DateTime<Local>, events: &[HistoryEvent]) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        (history::history_card(timestamp, events))
        (singularity_run_now_button())
    })
    .current_path("/singularity")
}

pub fn singularity_history(timestamp: DateTime<Local>, events: &[HistoryEvent]) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        (history::history_card(timestamp, events))
    })
    .current_path("/singularity")
}

pub fn singularity_histories(histories: &[(String, DateTime<Local>)]) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        (history::histories_card(histories))
    })
    .current_path("/singularity")
}

pub fn singularity_last_run(last_run: Option<DateTime<Local>>, next_run: DateTime<Local>) -> Markup {
    let to_next_run = humantime::format_duration(
        (next_run - Local::now())
            .to_std()
            .expect("failed to convert chrono duration to std duration")
            .round_to_minutes(),
    );

    html! {
        p {
            "Singularity was last run: "
            @if let Some(last_run) = last_run {
                (humantime::format_duration(
                    (Local::now() - last_run)
                    .to_std()
                    .expect("failed to convert chrono duration to std duration")
                    .round_to_minutes()))
                " ago at " (last_run.format(DATETIME_FORMAT))
            } @else {
                "Never"
            }
        }

        p {
            "Next scheduled run: in " (to_next_run) " at " (next_run.format(DATETIME_FORMAT))
        }
    }
}

pub fn singularity_run_now_button() -> Markup {
    html! {
        form method="POST" action="/singularity/run" {
            button ."btn" ."btn-outline-success" type="submit" { "Run Singularity now" }
        }
    }
}
