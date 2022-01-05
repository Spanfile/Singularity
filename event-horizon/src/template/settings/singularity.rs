mod adlists;
mod general;
mod outputs;

use crate::singularity::SingularityConfig;
use maud::{html, Markup};

#[derive(PartialEq, Eq)]
pub enum SingularitySubPage {
    Main,
    AddNewAdlist,
    RemoveAdlist(u64),
    AddNewHostsOutput,
    AddNewLuaOutput,
}

pub fn singularity(sub_page: SingularitySubPage, cfg: &SingularityConfig) -> Markup {
    match sub_page {
        SingularitySubPage::Main => main(cfg),
        SingularitySubPage::AddNewAdlist => adlists::add_new_adlist(),
        SingularitySubPage::RemoveAdlist(id) => adlists::remove_adlist(id, cfg),
        SingularitySubPage::AddNewHostsOutput => outputs::add_new_hosts_output(),
        SingularitySubPage::AddNewLuaOutput => outputs::add_new_lua_output(),
    }
}

fn main(cfg: &SingularityConfig) -> Markup {
    html! {
        (general::general_card(cfg))
        (adlists::adlists_card(cfg))
        (outputs::outputs_card(cfg))
    }
}
