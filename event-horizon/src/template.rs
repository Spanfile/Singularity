pub mod about;
pub mod index;
pub mod settings;

use actix_web::{http::StatusCode, HttpResponse};
use maud::{html, Markup, DOCTYPE};

struct ResponseBuilder<'a> {
    content: Markup,
    current_path: Option<&'a str>,
}

impl<'a> ResponseBuilder<'a> {
    fn new(content: Markup) -> Self {
        ResponseBuilder {
            content,
            current_path: None,
        }
    }

    fn build(self) -> HttpResponse {
        HttpResponse::build(StatusCode::OK)
            .content_type("text/html; charset=utf-8")
            .body(self.base().into_string())
    }

    fn current_path(mut self, path: &'a str) -> Self {
        self.current_path = Some(path);
        self
    }

    fn base(&self) -> Markup {
        html! {
            (self.header())
            (self.nav())
            .container ."pt-3" ."pb-3" {
                (self.content)
            }
        }
    }

    fn header(&self) -> Markup {
        html! {
            (DOCTYPE)
            title { "Event Horizon" }
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            link rel="stylesheet" href="/static/bootstrap-5.1.3-dist/css/bootstrap.min.css";
            script src="/static/bootstrap-5.1.3-dist/js/bootstrap.min.js" {}
        }
    }

    fn nav(&self) -> Markup {
        html! {
            nav ."navbar" ."navbar-expand-lg" ."navbar-dark" ."bg-dark" {
                ."container-md" {
                    a ."navbar-brand" href="/" { "Event Horizon" }
                    button ."navbar-toggler" type="button" data-bs-toggle="collapse" data-bs-target="#navbarSupportedContent" {
                        span ."navbar-toggler-icon" {}
                    }
                    #navbarSupportedContent .collapse ."navbar-collapse" {
                        ul ."navbar-nav" ."me-auto" ."mb-2" ."mb-lg-0" {
                            li ."nav-item" { a ."nav-link" .active[self.current_path == Some("/")] href="/" { "Home" } }
                            li ."nav-item" { a ."nav-link" .active[self.current_path == Some("/settings")] href="/settings" { "Settings" } }
                            li ."nav-item" { a ."nav-link" .active[self.current_path == Some("/about")] href="/about" { "About" } }
                        }
                    }
                }
            }
        }
    }
}
