use super::ResponseBuilder;
use maud::html;

pub fn settings() -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        p { "Settings" }
    })
}
