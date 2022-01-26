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

pub fn finish_config_import(rendered_cfg: Option<(&str, &str)>) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Finish importing Singularity configuration" }
            ."card-body" {
                p { "Choose a name for the configuration and decide how to import it." }

                form method="POST" {
                    .row {
                        ."mb-3" {
                            input ."form-control" #configName name="config_name" required
                                value=(rendered_cfg.map(|(name, _)| name).unwrap_or(""))
                                disabled[rendered_cfg.is_none()];
                            ."form-text" #configNameHelp {
                                "Choose a friendly name for this configuration. The name is purely visual and meant to \
                                help you distinguish different configurations, so it has to be unique."
                            }
                        }

                        ."col-lg-3" {
                            button .btn ."btn-primary" ."w-100" type="submit" name="strategy" value="New"
                                disabled[rendered_cfg.is_none()] {
                                "Import into new configuration"
                            }
                        }

                        ."col-lg-9" {
                            p {
                                strong { "Import into new configuration." }
                                " The pending configuration is imported into a new separate configuration. The \
                                current configuration is left active. You may choose which configuration to use at \
                                any time in the "
                                a href="/settings/event_horizon" target="_blank" { "Event Horizon settings page." }
                                " Only one configuration may be active at one time."
                            }
                        }

                        ."col-lg-3" {
                            button .btn ."btn-primary" ."w-100" type="submit" name="strategy" value="Merge"
                                disabled[rendered_cfg.is_none()] {
                                "Merge into current configuration"
                            }
                        }

                        ."col-lg-9" {
                            p {
                                strong { "Merge into current configuration." }
                                " The pending configuration is merged into the current active configuration. \
                                Adlists, outputs and whitelisted domains from the pending configuration are \
                                inserted into the current active configuration. Duplicate entries in any of the \
                                collections are ignored. No setting in the current configuration is overwritten. The \
                                current configuration's name is retained; the name given here is ignored."
                            }
                        }

                        ."col-lg-3" {
                            button .btn ."btn-outline-danger" ."w-100" type="submit" name="strategy" value="Overwrite"
                                disabled[rendered_cfg.is_none()] {
                                "Overwrite current configuration"
                            }
                        }

                        ."col-lg-9" {
                            p {
                                strong { "Overwrite current configuration." }
                                " The current active configuration is entirely replaced with the pending \
                                configuration. The current configuration cannot be restored after it is \
                                overwritten."
                            }
                        }

                        ."col-lg-12" ."mt-3" {
                            p { "Rendered pending configuration:" }
                            textarea ."form-control" ."font-monospace" rows="16" readonly disabled[rendered_cfg.is_none()] {
                                @if let Some((_, rendered)) = rendered_cfg {
                                    (rendered)
                                }
                            }
                        }

                        ."col-lg-3" ."mt-3" {
                            button .btn ."btn-secondary" ."w-100" type="submit" name="strategy" value="Cancel" {
                                "Cancel import"
                            }
                        }
                    }
                }
            }
        }
    }
}
