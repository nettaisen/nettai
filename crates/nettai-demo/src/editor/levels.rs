//! A navi's level in the editor: a link navi's navi code level (EXE6), a
//! team navi's story level (EXE5). No player states it: every side plays at
//! its navi's highest (`Side::set_navi` states it), and the rules build the
//! navi's stats from it as the round is set up (a link navi's reload, a
//! team navi's story HP: content's rules/save), which the stats pane
//! shows.

use nettai_battle::Content;
use nettai_content_api::NaviHandle;
use nettai_match::{Side, link_navis};

/// Whether the side's navi takes its stats from its level.
pub fn has_levels(content: &Content, side: &Side) -> bool {
    link_navis::has_levels(content, side.navi(content))
}

/// Switch the side to `navi`: at its level as every side plays (its
/// highest; MegaMan with no navi code). His NaviCust is his alone: another
/// navi's side has none (its programs and its board back at the rules'
/// defaults), and MegaMan's comes back as it was. The form list follows the
/// navi (`state_own_forms`).
pub fn switch_navi(content: &Content, side: &mut Side, navi: NaviHandle) {
    let _ = side.set_navi(content, navi);
    if content.navi(navi).forms.is_none() {
        for fact in ["navicust_programs", "navicust_expansions"] {
            side.facts.reset(content, fact);
        }
    }
    side.state_own_forms(content);
}
