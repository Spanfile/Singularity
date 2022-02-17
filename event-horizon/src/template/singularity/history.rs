use crate::{
    database::models::SingularityRunHistoryResult, logging::LogLevel, singularity::runner::history::HistoryEvent,
    template::DATETIME_FORMAT,
};
use chrono::{DateTime, Local};
use maud::{html, Markup};

pub fn histories_card(histories: &[(String, SingularityRunHistoryResult, DateTime<Local>)]) -> Markup {
    html! {
        .card ."border-dark" ."w-100" ."mb-3" {
            ."card-header" { "Singularity run history" }
            ."card-body" {
                table .table ."table-striped" ."table-borderless" ."mt-3" ."mb-0" {
                    thead {
                        tr {
                            th scope="col" { "Timestamp" }
                            th scope="col" { "Result" }
                            th scope="col" ."w-auto" { }
                        }
                    }
                    tbody {
                        @for (id, result, timestamp) in histories {
                            tr {
                                td { (timestamp.format(DATETIME_FORMAT)) }
                                td { (result) }
                                td {
                                    a .btn ."btn-outline-primary" ."btn-sm" ."float-end" href={
                                        "/singularity/history/" (id)
                                    } { "View" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn history_card(
    timestamp: DateTime<Local>,
    result: SingularityRunHistoryResult,
    events: &[HistoryEvent],
) -> Markup {
    html! {
        .card ."border-dark" ."w-100" ."mb-3" {
            ."card-header" { "Single run history" }
            ."card-body" {
                p {
                    "Timestamp: " (timestamp.format(DATETIME_FORMAT))
                    br;
                    "Result: " (result)
                }

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
                            tr ."table-warning"[event.severity() == LogLevel::Warn]
                                ."table-danger"[event.severity() == LogLevel::Error]  {
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
