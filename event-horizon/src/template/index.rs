use super::ResponseBuilder;
use actix_web::HttpResponse;
use maud::html;

pub fn index() -> HttpResponse {
    ResponseBuilder::new(html! {
        p { "Hello!" }
    })
    .current_path("/")
    .build()
}
