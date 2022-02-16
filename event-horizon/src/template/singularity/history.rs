use crate::{singularity::runner::history::HistoryEvent, template::DATETIME_FORMAT};
use chrono::{DateTime, Local};
use maud::{html, Markup};

pub fn history_card(timestamp: DateTime<Local>, events: &[HistoryEvent]) -> Markup {
    html! {
        .card ."border-dark" ."w-100" ."mb-3" {
            ."card-header" { "Singularity run history" }
            ."card-body" {
                p { "Run timestamp: " (timestamp.format(DATETIME_FORMAT)) }

                table .table ."table-striped" ."table-borderless" ."mt-3" ."mb-0" {
                    thead {
                        tr {
                            th scope="col" { "Timestamp" }
                            th scope="col" { "Severity" }
                            th scope="col" { "Message" }
                        }
                    }
                    tbody {
                        @for event in events {
                            tr {
                                td { (event.timestamp()) }
                                td { (event.severity()) }
                                td { (event.message()) }
                            }
                        }
                    }
                }
            }
        }
    }
}
