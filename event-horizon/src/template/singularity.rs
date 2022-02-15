use super::ResponseBuilder;
use crate::util::round_duration::RoundDuration;
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

pub fn singularity_finished() -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        p { "The results for the run are going to be displayed here." }

        (singularity_run_now_button())
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
                " ago at " (last_run.format("%H:%M, %a %x"))
            } @else {
                "Never"
            }
        }

        p {
            "Next scheduled run: in " (to_next_run) " at " (next_run.format("%H:%M, %a %x"))
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
