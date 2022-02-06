use super::ResponseBuilder;
use maud::html;

pub fn index() -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        p { "Hello!" }
    })
    .current_path("/")
}
