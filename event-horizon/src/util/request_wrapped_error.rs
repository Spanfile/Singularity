// dear god forgive me for what I'm about to do

use actix_web::{http::StatusCode, HttpRequest, HttpResponse};

pub trait WrappedError {
    fn status_code(&self) -> StatusCode;
    fn error_response(&self, req: &HttpRequest) -> HttpResponse;
}

#[derive(Debug)]
pub struct RequestWrappedError<E>
where
    E: WrappedError + std::fmt::Display + std::fmt::Debug,
{
    error: E,
    req: HttpRequest,
}

impl<E> std::fmt::Display for RequestWrappedError<E>
where
    E: WrappedError + std::fmt::Display + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl<E> actix_web::error::ResponseError for RequestWrappedError<E>
where
    E: WrappedError + std::fmt::Display + std::fmt::Debug,
{
    fn status_code(&self) -> StatusCode {
        self.error.status_code()
    }

    fn error_response(&self) -> HttpResponse {
        self.error.error_response(&self.req)
    }
}

impl<E> RequestWrappedError<E>
where
    E: WrappedError + std::fmt::Display + std::fmt::Debug,
{
    pub fn new(error: E, req: &HttpRequest) -> Self {
        Self {
            error,
            req: req.clone(), // a request is just an Rc to an internal request structure so it is cheap to clone
        }
    }
}
