mod recursor;

use super::ResponseBuilder;
use maud::html;

pub fn stats() -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        p { "Hopefully there'll be fancy images here!" }
        a href="/statistics/recursor" { "Raw Recursor statistics" }
    })
    .current_path("/statistics")
}

pub fn raw_recursor(recursor: Option<&[(&str, &str)]>) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        (recursor::raw_recursor(recursor))
    })
    .current_path("/statistics")
}
