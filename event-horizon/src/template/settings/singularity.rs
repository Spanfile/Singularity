mod adlists;
mod general;
mod outputs;

use crate::database::DbId;
use maud::{html, Markup};
use singularity::{Adlist, Output};

#[derive(PartialEq, Eq)]
pub enum SingularitySubPage<'a> {
    Main {
        adlists: &'a [(DbId, Adlist)],
        outputs: &'a [(DbId, Output)],
    },
    AddNewAdlist,
    DeleteAdlist(DbId, &'a Adlist),
    AddNewHostsOutput,
    AddNewLuaOutput,
}

pub fn singularity(sub_page: SingularitySubPage) -> Markup {
    match sub_page {
        SingularitySubPage::Main { adlists, outputs } => main(adlists, outputs),
        SingularitySubPage::AddNewAdlist => adlists::add_new_adlist(),
        SingularitySubPage::DeleteAdlist(id, adlist) => adlists::delete_adlist(id, adlist),
        SingularitySubPage::AddNewHostsOutput => outputs::add_new_hosts_output(),
        SingularitySubPage::AddNewLuaOutput => outputs::add_new_lua_output(),
    }
}

fn main(adlists: &[(DbId, Adlist)], outputs: &[(DbId, Output)]) -> Markup {
    html! {
        (general::general_card())
        (adlists::adlists_card(adlists))
        (outputs::outputs_card(outputs))
    }
}
