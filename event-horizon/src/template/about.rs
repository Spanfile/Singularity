use super::ResponseBuilder;
use crate::built_info::*;
use maud::html;

pub fn about(recursor_version: &str, redis_version: &str) -> ResponseBuilder<'static> {
    ResponseBuilder::new(html! {
        .row {
            ."col-lg-6" ."offset-lg-3" {
                h5 { "Version information" }
                dl .row {
                    dt ."col-sm-4" { "Event Horizon version" }
                    dd ."col-sm-8" { (PKG_VERSION) }

                    dt ."col-sm-4" { "Event Horizon Git commit" }
                    dd ."col-sm-8" { "TODO" }

                    dt ."col-sm-4" { "Singularity version" }
                    dd ."col-sm-8" { "TODO" }

                    dt ."col-sm-4" { "PDNS Recursor version" }
                    dd ."col-sm-8" { (recursor_version) }

                    dt ."col-sm-4" { "Redis version" }
                    dd ."col-sm-8" { (redis_version) }
                }
            }

            ."col-lg-6" ."offset-lg-3" {
                h5 { "Build information" }
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
    .current_path("/about")
}
