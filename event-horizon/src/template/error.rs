use super::ResponseBuilder;
use maud::html;

pub fn error() -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        p { "Hello!" }
    })
}
