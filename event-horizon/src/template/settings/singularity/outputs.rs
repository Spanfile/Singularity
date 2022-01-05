use crate::singularity::SingularityConfig;
use maud::{html, Markup};
use singularity::{
    Output, OutputType, DEFAULT_BLACKHOLE_ADDRESS_V4, DEFAULT_DEDUPLICATE, DEFAULT_METRIC_NAME, DEFAULT_OUTPUT_METRIC,
};

pub fn outputs_card(cfg: &SingularityConfig) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Outputs" }
            ."card-body" {
                .row ."g-3" {
                    ."col-auto" {
                        a .btn ."btn-primary" href="/settings/singularity/add_new_lua_output" { "Add new PDNS Lua script output" }
                    }

                    ."col-auto" {
                        a .btn ."btn-primary" href="/settings/singularity/add_new_hosts_output" { "Add new hosts-file output" }
                    }
                }

                ."list-group" ."mt-3" {
                    @for (id, output) in cfg.outputs() {
                        (single_output_card(id, output))
                    }
                }
            }
        }
    }
}

pub fn single_output_card(id: u64, output: &Output) -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" ."container-fluid" {
                .row ."g-3" {
                    ."col-auto" ."me-auto" ."d-flex" ."align-items-center" { (output.ty) " - " (output.destination.display()) }
                    ."col-auto" {
                        a ."btn" ."btn-primary" ."btn-sm" ."mb-auto" href={ "/settings/singularity/edit_output?id=" (id) } { "Edit" }
                    }
                    ."col-auto" {
                        a ."btn" ."btn-danger" ."btn-sm" href={ "/settings/singularity/delete_output?id=" (id) } { "Delete" }
                    }
                }
            }

            ."card-body" {
                .row {
                    ."col-md" {
                        dl .row {
                            dt ."col-lg-6" { "Blackhole address" }
                            dd ."col-lg-6" { (output.blackhole_address) }

                            dt ."col-lg-6" { "Deduplicate" }
                            dd ."col-lg-6" { (output.deduplicate) }
                        }
                    }

                    ."col-md" {
                        dl .row {
                            @match &output.ty {
                                OutputType::Hosts { include } => {
                                    dt ."col-xl-12" { "Included files" }
                                    dd ."col-xl-12" {
                                        ul ."list-group" ."list-group-flush" {
                                            @for file in include {
                                                li ."list-group-item" { (file.display()) }
                                            }
                                        }
                                    }
                                },
                                OutputType::PdnsLua { output_metric, metric_name } => {
                                    dt ."col-lg-6" { "Metric enabled" }
                                    dd ."col-lg-6" { (output_metric) }

                                    dt ."col-lg-6" { "Metric name" }
                                    dd ."col-lg-6" { (metric_name) }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn add_new_hosts_output() -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Add new hosts output" }
            ."card-body" {
                form method="POST" {
                    (common_output_form("hosts"))

                    button .btn ."btn-primary" ."me-3" type="submit" { "Add new output" }
                    a .btn ."btn-secondary" href="/settings/singularity" { "Cancel" }
                }
            }
        }
    }
}

pub fn add_new_lua_output() -> Markup {
    html! {
        .card ."w-100" ."mb-3" {
            ."card-header" { "Add new PDNS Lua script output" }
            ."card-body" {
                form method="POST" {
                    (common_output_form("pdns-lua"))

                    ."mb-3" ."form-check" {
                        input ."form-check-input" #output_metric name="output_metric" type="checkbox" value="true" checked[DEFAULT_OUTPUT_METRIC];
                        label ."form-check-label" for="output_metric" { "Metric enabled" }
                    }

                    ."mb-3" {
                        label ."form-label" for="metric_name" { "Metric name" }
                        input #source ."form-control" name="metric_name" type="text" value=(DEFAULT_METRIC_NAME);
                    }

                    button .btn ."btn-primary" ."me-3" type="submit" { "Add new output" }
                    a .btn ."btn-secondary" href="/settings/singularity" { "Cancel" }
                }
            }
        }
    }
}

fn common_output_form(ty: &str) -> Markup {
    html! {
        input name="type" value=(ty) hidden;

        ."mb-3" {
            label ."form-label" for="destination" { "Destination" }
            input #source ."form-control" name="destination" type="text";
        }

        ."mb-3" {
            label ."form-label" for="blackhole_address" { "Blackhole address" }
            input #source ."form-control" name="blackhole_address" type="text" value=(DEFAULT_BLACKHOLE_ADDRESS_V4);
        }

        ."mb-3" ."form-check" {
            input ."form-check-input" #deduplicate name="deduplicate" type="checkbox" value="true" checked[DEFAULT_DEDUPLICATE];
            label ."form-check-label" for="deduplicate" { "Deduplicate" }
        }
    }
}
