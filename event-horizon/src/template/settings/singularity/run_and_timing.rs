use crate::util::round_duration::RoundDuration;
use chrono::{DateTime, Local};
use maud::{html, Markup};

pub fn run_and_timing_card(last_run: Option<DateTime<Local>>, next_run: DateTime<Local>, timing: &str) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Running and timing" }
            ."card-body" {
                (run_card(last_run, next_run))
                hr;
                (timing_body(timing))
            }
        }
    }
}

fn run_card(last_run: Option<DateTime<Local>>, next_run: DateTime<Local>) -> Markup {
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

        a ."btn" ."btn-primary" href="/run_singularity" { "Run Singularity now" }
    }
}

fn timing_body(timing: &str) -> Markup {
    html! {
        p {
            "Define the interval when to automatically run Singularity with a "
            a href="https://en.wikipedia.org/wiki/Cron#Overview" target="_blank" { "cronjob expression." }
            " Several presets are available, and the website "
            a href="https://crontab.guru/" target="_blank" { "https://crontab.guru/" }
            " may be used to help write the expressions."
        }

        form method="POST" action="/settings/singularity/set_timing" {
            .row ."mb-3" {
                ."col-md-10" ."offset-md-2" {
                    .row ."g-3" {
                        ."col-auto" {
                            button ."btn" ."btn-primary" name="expression" value="0 0 * * *" type="submit" {
                                "Once daily at midnight"
                            }
                        }

                        ."col-auto" {
                            button ."btn" ."btn-primary" name="expression" value="0 */12 * * *" type="submit" {
                                "Twice daily at midnight and at noon"
                            }
                        }

                        ."col-auto" {
                            button ."btn" ."btn-primary" name="expression" value="0 */6 * * *" type="submit" {
                                "Every six hours beginning at midnight"
                            }
                        }
                    }
                }
            }
        }

        form method="POST" action="/settings/singularity/set_timing" {
            .row ."mb-3" {
                label ."col-md-2" ."col-form-label" for="expression" { "Schedule" }
                . "col-md-10" {
                    input #expression ."form-control" type="text" name="expression" value=(timing);
                }
            }

            button ."btn" ."btn-primary" type="submit" { "Save" }
        }
    }
}
