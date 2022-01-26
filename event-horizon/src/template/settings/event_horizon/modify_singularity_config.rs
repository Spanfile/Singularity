use maud::{html, Markup};

pub fn use_singularity_config(name: Option<&str>) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Use this Singularity configuration?" }
            ."card-body" {
                form method="POST" {
                    input ."form-control" ."mb-3" type="text" value=(name.unwrap_or("")) disabled readonly;

                    button .btn ."btn-primary" ."me-3" type="submit" disabled[name.is_none()] { "Use" }
                    a .btn ."btn-secondary" href="/settings/event_horizon" { "Cancel" }
                }
            }
        }
    }
}
