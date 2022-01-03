use super::ResponseBuilder;
use actix_web::HttpResponse;
use maud::html;

pub fn settings() -> HttpResponse {
    ResponseBuilder::new(html! {
        p { "Settings" }
    })
    .current_path("/settings")
    .build()
}
