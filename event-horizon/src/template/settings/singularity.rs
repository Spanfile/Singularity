mod adlists;
mod outputs;
mod whitelist;

use crate::database::DbId;
use maud::{html, Markup};
use singularity::{Adlist, Output};

#[derive(PartialEq, Eq)]
pub enum SingularitySubPage<'a> {
    Main {
        cfg_name: &'a str,
        adlists: &'a [(DbId, Adlist)],
        outputs: &'a [(DbId, Output)],
        whitelist: &'a [(DbId, String)],
    },
    AddNewAdlist,
    DeleteAdlist(DbId, &'a Adlist),
    AddNewHostsOutput,
    AddNewLuaOutput,
    DeleteOutput(DbId, &'a Output),
    AddNewWhitelistedDomain,
    DeleteWhitelistedDomain(DbId, &'a str),
}

pub fn singularity(sub_page: SingularitySubPage) -> Markup {
    match sub_page {
        SingularitySubPage::Main {
            cfg_name,
            adlists,
            outputs,
            whitelist,
        } => main(cfg_name, adlists, outputs, whitelist),
        SingularitySubPage::AddNewAdlist => adlists::add_new_adlist(),
        SingularitySubPage::DeleteAdlist(id, adlist) => adlists::delete_adlist(id, adlist),
        SingularitySubPage::AddNewHostsOutput => outputs::add_new_hosts_output(),
        SingularitySubPage::AddNewLuaOutput => outputs::add_new_lua_output(),
        SingularitySubPage::DeleteOutput(id, output) => outputs::delete_output(id, output),
        SingularitySubPage::AddNewWhitelistedDomain => whitelist::add_new_whitelisted_domain(),
        SingularitySubPage::DeleteWhitelistedDomain(id, domain) => whitelist::delete_whitelisted_domain(id, domain),
    }
}

fn main(
    cfg_name: &str,
    adlists: &[(DbId, Adlist)],
    outputs: &[(DbId, Output)],
    whitelist: &[(DbId, String)],
) -> Markup {
    html! {
        .row {
            label ."col-auto" ."col-form-label" for="configName" { "Current active Singularity configuration:" }
            ."col-auto" {
                input ."form-control-plaintext" #configName type="text" value=(cfg_name) readonly;
            }
        }

        p {
            "You may change the active configuration in the "
            a href="/settings/event_horizon" { "Event Horizon settings. " }
            "Only one configuration may be active at one time."
        }

        (adlists::adlists_card(adlists))
        (outputs::outputs_card(outputs))
        (whitelist::whitelist_card(whitelist))
    }
}
