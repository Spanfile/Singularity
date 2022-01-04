use super::ResponseBuilder;
use crate::built_info::*;
use maud::html;

pub fn about() -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        .row {
            ."col-lg-6" ."offset-lg-3" {
                dl .row {
                    dt ."col-sm-4" { "Event Horizon version" }
                    dd ."col-sm-8" { (PKG_VERSION) }

                    dt ."col-sm-4" { "Singularity version" }
                    dd ."col-sm-8" { "TODO" }

                    dt ."col-sm-4" { "Git commit" }
                    dd ."col-sm-8" { "TODO" }
                }
            }

            ."col-lg-6" ."offset-lg-3" {
                h3 { "Build information" }
                dl .row {
                    dt ."col-sm-4" { "Current target" }
                    dd ."col-sm-8" { (TARGET) }

                    dt ."col-sm-4" { "Rust version" }
                    dd ."col-sm-8" { (RUSTC_VERSION) }

                    dt ."col-sm-4" { "Build profile" }
                    dd ."col-sm-8" { (PROFILE) }
                }
            }
        }
    })
}
