pub mod about;
pub mod index;
pub mod settings;

// re-exported for convenience
pub use about::about;
pub use index::index;
pub use settings::settings;

use actix_web::{http::StatusCode, HttpResponse};
use maud::{html, Markup, DOCTYPE};

pub struct ResponseBuilder<'a> {
    content: Markup,
    current_path: Option<&'a str>,
    alert: Option<Alert>,
}

pub enum Alert {
    Information(String),
    Success(String),
    Warning(String),
    Error(String),
}

impl<'a> ResponseBuilder<'a> {
    fn new(content: Markup) -> Self {
        ResponseBuilder {
            content,
            current_path: None,
            alert: None,
        }
    }

    pub fn ok(self) -> HttpResponse {
        HttpResponse::build(StatusCode::OK)
            .content_type("text/html; charset=utf-8")
            .body(self.markup_base().into_string())
    }

    pub fn bad_request(self) -> HttpResponse {
        HttpResponse::build(StatusCode::BAD_REQUEST)
            .content_type("text/html; charset=utf-8")
            .body(self.markup_base().into_string())
    }

    #[must_use]
    pub fn current_path(mut self, path: &'a str) -> Self {
        self.current_path = Some(path);
        self
    }

    #[must_use]
    pub fn alert(mut self, alert: Alert) -> Self {
        self.alert = Some(alert);
        self
    }

    fn markup_base(&self) -> Markup {
        html! {
            (self.markup_header())
            (self.markup_nav())
            .container ."pt-3" ."pb-3" {
                @if let Some(alert) = self.markup_alert() { (alert) }
                (self.content)
            }
        }
    }

    fn markup_header(&self) -> Markup {
        html! {
            (DOCTYPE)
            title { "Event Horizon" }
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            link rel="stylesheet" href="/static/bootstrap-5.1.3-dist/css/bootstrap.min.css";
            script src="/static/bootstrap-5.1.3-dist/js/bootstrap.min.js" {}
            // TODO: figure out a way to get rid of JS
            // this is needed for:
            // - the navbar

            svg xmlns="http://www.w3.org/2000/svg" style="display: none;" {
                symbol id="check-circle-fill" fill="currentColor" viewBox="0 0 16 16" {
                    path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zm-3.97-3.03a.75.75 0 0 0-1.08.022L7.477 9.417 5.384 7.323a.75.75 0 0 0-1.06 1.06L6.97 11.03a.75.75 0 0 0 1.079-.02l3.992-4.99a.75.75 0 0 0-.01-1.05z";
                }
                symbol id="info-fill" fill="currentColor" viewBox="0 0 16 16" {
                    path d="M8 16A8 8 0 1 0 8 0a8 8 0 0 0 0 16zm.93-9.412-1 4.705c-.07.34.029.533.304.533.194 0 .487-.07.686-.246l-.088.416c-.287.346-.92.598-1.465.598-.703 0-1.002-.422-.808-1.319l.738-3.468c.064-.293.006-.399-.287-.47l-.451-.081.082-.381 2.29-.287zM8 5.5a1 1 0 1 1 0-2 1 1 0 0 1 0 2z";
                }
                symbol id="exclamation-triangle-fill" fill="currentColor" viewBox="0 0 16 16" {
                    path d="M8.982 1.566a1.13 1.13 0 0 0-1.96 0L.165 13.233c-.457.778.091 1.767.98 1.767h13.713c.889 0 1.438-.99.98-1.767L8.982 1.566zM8 5c.535 0 .954.462.9.995l-.35 3.507a.552.552 0 0 1-1.1 0L7.1 5.995A.905.905 0 0 1 8 5zm.002 6a1 1 0 1 1 0 2 1 1 0 0 1 0-2z";
                }
            }
        }
    }

    fn markup_nav(&self) -> Markup {
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

    fn markup_alert(&self) -> Option<Markup> {
        fn alert_base(kind: &str, icon: &str, msg: &str) -> Option<Markup> {
            Some(html! {
                .alert .{ "alert-" (kind) } ."d-flex" ."align-items-center" {
                    svg .bi ."flex-shrink-0" ."me-2" width="24" height="24" { use xlink:href={ "#" (icon) }; }
                    div { (msg) }
                }
            })
        }

        match &self.alert {
            Some(Alert::Information(msg)) => alert_base("primary", "info-fill", msg),
            Some(Alert::Success(msg)) => alert_base("success", "check-circle-fill", msg),
            Some(Alert::Warning(msg)) => alert_base("warning", "exclamation-triangle-fill", msg),
            Some(Alert::Error(msg)) => alert_base("danger", "exclamation-triangle-fill", msg),
            None => None,
        }
    }
}
