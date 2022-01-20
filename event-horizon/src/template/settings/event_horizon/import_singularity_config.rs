use maud::{html, Markup};

pub fn import_singularity_config() -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Import Singularity configuration" }
            ."card-body" {
                form method="POST" enctype="multipart/form-data" {
                    ."mb-3" {
                        label ."form-label" for="file" { "Import configuration from file" }
                        input ."form-control" #file name="file" type="file" accept=".toml,application/toml" required;
                    }

                    button .btn ."btn-primary" type="submit" { "Import" }
                }

                p ."mt-3" { "Or" }

                form method="POST" {
                    ."mb-3" {
                        label ."form-label" for="text" { "Paste configuration text" }
                        textarea ."form-control" #text name="text" rows="5" {}
                    }

                    button .btn ."btn-primary" type="submit" { "Import" }
                }
            }
        }
    }
}

pub fn finish_config_import(rendered_cfg: Option<&str>) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Finish importing Singularity configuration" }
            ."card-body" {
                p { "Choose how to import the pending Singularity configuration." }

                .row {
                    ."col-sm-3" {
                        form method="POST" {
                            input name="strategy" value="New" type="hidden";
                            button .btn ."btn-primary" ."w-100" type="submit" disabled[rendered_cfg.is_none()] {
                                "Import into new configuration"
                            }
                        }
                    }

                    ."col-sm-9" {
                        p {
                            strong { "Import into new configuration." }
                            " The pending configuration is imported into a new separate configuration. The \
                            current configuration is left active. You may choose which configuration to use at \
                            any time in the "
                            a href="/settings/event_horizon" target="_blank" { "Event Horizon settings page." }
                            " Only one configuration may be active at one time."
                        }
                    }

                    ."col-sm-3" {
                        form method="POST" {
                            input name="strategy" value="Merge" type="hidden";
                            button .btn ."btn-primary" ."w-100" type="submit" disabled[rendered_cfg.is_none()] {
                                "Merge into current configuration"
                            }
                        }
                    }

                    ."col-sm-9" {
                        p {
                            strong { "Merge into current configuration." }
                            " The pending configuration is merged into the current active configuration. \
                            Adlists, outputs and whitelisted domains from the pending configuration are \
                            inserted into the current active configuration. Duplicate entries in any of the \
                            collections are ignored. No setting in the current configuration is overwritten."
                        }
                    }

                    ."col-sm-3" {
                        form method="POST" {
                            input name="strategy" value="Overwrite" type="hidden";
                            button .btn ."btn-outline-danger" ."w-100" type="submit" disabled[rendered_cfg.is_none()] {
                                "Overwrite current configuration"
                            }
                        }
                    }

                    ."col-sm-9" {
                        p {
                            strong { "Overwrite current configuration." }
                            " The current active configuration is entirely replaced with the pending \
                            configuration. The current configuration cannot be restored after it is \
                            overwritten."
                        }
                    }

                    ."col-sm-12" ."mt-3" {
                        p { "Rendered pending configuration:" }
                        textarea ."form-control" ."font-monospace" rows="16" readonly disabled[rendered_cfg.is_none()] {
                            @if let Some(rendered_str) = rendered_cfg {
                                (rendered_str)
                            }
                        }
                    }

                    ."col-sm-3" ."mt-3" {
                        form method="POST" {
                            input name="strategy" value="Cancel" type="hidden";
                            button .btn ."btn-secondary" ."w-100" type="submit" { "Cancel import" }
                        }
                    }
                }
            }
        }
    }
}
