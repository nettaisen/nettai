//! A link navi's level in the editor: changing the level or switching to a
//! link navi fills in the stats its save's reload gives it
//! (nettai-match's `link_navis`, docs/engine/link-navis.md), what the save
//! keeps carried over; an edited stat stays edited, and the stats pane shows
//! where it differs from the level's.

use nettai_battle::Content;
use nettai_battle::setup::NaviStats;
use nettai_content_api::NaviHandle;
use nettai_match::{Side, link_navis, stats};

/// Whether the side's navi takes its stats from its link navi level.
pub fn has_levels(content: &Content, side: &Side) -> bool {
    link_navis::has_levels(content, side.navi)
}

/// Switch the side to `navi` as the game does, if it is a link navi: its
/// reload at the side's level over the side's stats. False (nothing done)
/// for another navi.
pub fn switch_navi(content: &Content, side: &mut Side, navi: NaviHandle) -> bool {
    let Some(stats) = side.reloaded_as(content, navi) else { return false };
    side.navi = navi;
    side.stats = stats;
    side.crosses = None;
    true
}

/// The level changed: a link navi's stats are its reload's at the new
/// level (nothing changes for a level past its table, which the checks
/// refuse).
pub fn level_changed(content: &Content, side: &mut Side) {
    if let Some(stats) = side.reloaded(content) {
        side.stats = stats;
    }
}

/// What "reset" gives the side: the navi's stats as a save gives them (a
/// link navi's at its level).
pub fn reset(content: &Content, side: &Side) -> NaviStats {
    Side::save_base(content, side.navi, side.game, side.navi_level)
}

/// For a link navi, what its level gives stat `f` where the side's differs
/// (an edited stat), as the stats pane says it.
pub fn differs(content: &Content, side: &Side, f: &stats::Field) -> Option<String> {
    let derived = side.reloaded(content)?;
    let level = (f.get)(&derived);
    (level != (f.get)(&side.stats)).then(|| format!("level {} gives {}", side.navi_level, stats::to_toml(content, level)))
}
