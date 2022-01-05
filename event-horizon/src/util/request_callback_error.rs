// dear god forgive me for what I'm about to do

use actix_web::{http::StatusCode, HttpResponse};

pub struct RequestCallbackError<F>
where
    F: Fn() -> HttpResponse,
{
    status_code: StatusCode,
    response_callback: F,
}

impl<F> std::fmt::Debug for RequestCallbackError<F>
where
    F: Fn() -> HttpResponse,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestCallbackError")
            .field("status_code", &self.status_code)
            .finish()
    }
}

impl<F> std::fmt::Display for RequestCallbackError<F>
where
    F: Fn() -> HttpResponse,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RequestCallbackError: {}", self.status_code)
    }
}

impl<F> actix_web::error::ResponseError for RequestCallbackError<F>
where
    F: Fn() -> HttpResponse,
{
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn error_response(&self) -> HttpResponse {
        (self.response_callback)()
    }
}

impl<F> RequestCallbackError<F>
where
    F: Fn() -> HttpResponse,
{
    pub fn new(status_code: StatusCode, response_callback: F) -> Self {
        Self {
            status_code,
            response_callback,
        }
    }
}
