use maud::{html, Markup};

pub fn raw_recursor(stats: Option<&[(&str, &str)]>) -> Markup {
    html! {
        .card ."border-dark" ."w-100" ."mb-3" {
            ."card-header" { "Recursor statistics" }
            ."card-body" {
                table .table ."table-sm" ."table-striped" ."table-borderless" ."mt-3" ."mb-0" {
                    thead {
                        tr {
                            th scope="col" { "Statistic" }
                            th scope="col" { "Value" }
                        }
                    }
                    tbody {
                        @if let Some(stats) = stats {
                            @for (name, value) in stats {
                                tr {
                                    td ."font-monospace" { (name) }
                                    td ."font-monospace" { (value) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
