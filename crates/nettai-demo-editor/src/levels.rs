//! A navi's level in the editor: a link navi's navi code level (EXE6), a
//! team navi's story level (EXE5). The side states the level and the rules
//! build the navi's stats from it as the round is set up (a link navi's
//! reload, a team navi's story HP: content's save systems), which the stats
//! pane shows.

use nettai_battle::Content;
use nettai_content_api::NaviHandle;
use nettai_match::{Side, link_navis};

/// Whether the side's navi takes its stats from its level.
pub fn has_levels(content: &Content, side: &Side) -> bool {
    link_navis::has_levels(content, side.navi)
}

/// Switch the side to `navi` as the game does: a navi that takes its stats
/// from a level keeps the side's (a link navi from no level, level 0: it
/// exists through its navi code); MegaMan again has no navi code
/// (`sub_809CD60`). His NaviCust is his alone: another navi's side has
/// none, and states no base HP (its HP is its level's), and MegaMan's
/// comes back empty. The form list follows the navi (`state_own_forms`).
pub fn switch_navi(content: &Content, side: &mut Side, navi: NaviHandle) {
    side.navi = navi;
    let level = if link_navis::has_levels(content, navi) {
        side.level(content).or(nettai_match::default_navi_level(content, navi)).or(Some(0))
    } else {
        nettai_match::default_navi_level(content, navi)
    };
    let _ = side.set_level(content, level);
    match nettai_match::empty_navicust(content, navi) {
        Some(empty) => {
            side.navicust.get_or_insert(empty);
        }
        None => {
            side.navicust = None;
            side.facts.reset(content, nettai_battle::content::PlayerFact::BaseHp.name());
        }
    }
    side.state_own_forms(content);
}
