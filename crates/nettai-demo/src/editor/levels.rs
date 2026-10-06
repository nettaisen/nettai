//! A navi's level in the editor: a link navi's navi code level (EXE6), a
//! team navi's story level (EXE5). The side states the level and the rules
//! build the navi's stats from it as the round is set up (a link navi's
//! reload, a team navi's story HP: content's rules/save), which the stats
//! pane shows.

use nettai_battle::Content;
use nettai_content_api::NaviHandle;
use nettai_match::{Side, link_navis};

/// Whether the side's navi takes its stats from its level.
pub fn has_levels(content: &Content, side: &Side) -> bool {
    link_navis::has_levels(content, side.navi(content))
}

/// Switch the side to `navi` as the game does: a navi that must have a
/// level keeps the side's (from no level, 0: a link navi exists through its
/// navi code, a team navi at its story's progress); MegaMan again has no navi code
/// (`sub_809CD60`). His NaviCust is his alone: another navi's side has
/// none (its programs, its board and its base HP back at the rules'
/// defaults: its HP is its level's), and MegaMan's comes back as it was.
/// The form list follows the navi (`state_own_forms`).
pub fn switch_navi(content: &Content, side: &mut Side, navi: NaviHandle) {
    let _ = side.set_navi(content, navi);
    let level = nettai_match::level_required(content, navi).then(|| side.level(content).unwrap_or(0));
    let _ = side.set_level(content, level);
    if content.navi(navi).forms.is_none() {
        for fact in ["navicust_programs", "navicust_expansions", nettai_battle::content::PlayerFact::BaseHp.name()] {
            side.facts.reset(content, fact);
        }
    }
    side.state_own_forms(content);
}
