use maud::{html, Markup};

pub fn recursor(recursor: Option<&[(&str, &str)]>) -> Markup {
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
                        @if let Some(recursor) = recursor {
                            @for (name, value) in recursor {
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
