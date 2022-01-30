mod adlists;
mod outputs;
mod run_and_timing;
mod whitelist;

use crate::{
    database::DbId,
    singularity::{AdlistCollection, OutputCollection, WhitelistCollection},
};
use chrono::{DateTime, Local};
use maud::{html, Markup};
use singularity::{Adlist, Output};

#[derive(PartialEq, Eq)]
pub enum SingularitySubPage<'a> {
    Main(SingularityMainPageInformation),
    AddNewAdlist,
    DeleteAdlist(Option<(DbId, &'a Adlist)>),
    AddNewHostsOutput,
    AddNewLuaOutput,
    DeleteOutput(Option<(DbId, &'a Output)>),
    AddNewWhitelistedDomain,
    DeleteWhitelistedDomain(Option<(DbId, &'a str)>),
}

#[derive(PartialEq, Eq)]
pub struct SingularityMainPageInformation {
    pub cfg_name: String,
    pub last_run: Option<DateTime<Local>>,
    pub next_run: DateTime<Local>,
    pub timing: String,
    pub adlists: AdlistCollection,
    pub outputs: OutputCollection,
    pub whitelist: WhitelistCollection,
}

pub fn singularity(sub_page: SingularitySubPage) -> Markup {
    match sub_page {
        SingularitySubPage::Main(info) => main(info),
        SingularitySubPage::AddNewAdlist => adlists::add_new_adlist(),
        SingularitySubPage::DeleteAdlist(id_adlist) => adlists::delete_adlist(id_adlist),
        SingularitySubPage::AddNewHostsOutput => outputs::add_new_hosts_output(),
        SingularitySubPage::AddNewLuaOutput => outputs::add_new_lua_output(),
        SingularitySubPage::DeleteOutput(id_output) => outputs::delete_output(id_output),
        SingularitySubPage::AddNewWhitelistedDomain => whitelist::add_new_whitelisted_domain(),
        SingularitySubPage::DeleteWhitelistedDomain(id_domain) => whitelist::delete_whitelisted_domain(id_domain),
    }
}

fn main(info: SingularityMainPageInformation) -> Markup {
    html! {
        .row {
            label ."col-auto" ."col-form-label" for="configName" { "Current active Singularity configuration:" }
            ."col-auto" {
                input ."form-control-plaintext" #configName type="text" value=(info.cfg_name) readonly;
            }
        }

        p {
            "You may change the active configuration in the "
            a href="/settings/event_horizon" { "Event Horizon settings. " }
            "Only one configuration may be active at one time."
        }

        (run_and_timing::run_and_timing_card(info.last_run, info.next_run, &info.timing))
        (adlists::adlists_card(&info.adlists))
        (outputs::outputs_card(&info.outputs))
        (whitelist::whitelist_card(&info.whitelist))
    }
}
