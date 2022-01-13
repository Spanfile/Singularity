use super::ResponseBuilder;
use maud::html;

pub fn error(message: &str) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        h3 { "An internal server error occurred" }
        textarea readonly ."form-control" rows="10" { (message) }
        br;
        p { "The application logs may have more information. You might want to report this error in "
            a href="https://github.com/Spanfile/Singularity/issues" target="_blank" { "the issue tracker." }
        }
        a href="/" { "Back to the front page." }
    })
}
