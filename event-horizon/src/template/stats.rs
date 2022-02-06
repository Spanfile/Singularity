mod recursor;

use super::ResponseBuilder;
use maud::html;

pub fn stats(recursor: Option<&[(&str, &str)]>) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        (recursor::recursor(recursor))
    })
}
