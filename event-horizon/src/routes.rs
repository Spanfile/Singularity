pub mod about;
pub mod index;
pub mod run_singularity;
pub mod settings;
pub mod stats;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.configure(index::config)
        .configure(about::config)
        .configure(settings::config)
        .configure(stats::config)
        .configure(run_singularity::config);
}
