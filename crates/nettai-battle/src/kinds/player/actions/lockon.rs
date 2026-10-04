//! `ho_8026554`: the panel a navi attacks a target from in a lock-on mode
//! (BN6's rules/lockon: records of type "lockon"): what the game's wrapper, BN6's Beast Out
//! rush (rules/beast/rush.luau), and the Beast claw's, the lunge's and
//! GroundCross's drill's setups ask (`CoreApi::lockon_panel`).

use crate::battle::Battle;
use crate::collision::f1;
use crate::content::PanelOffset;
use crate::field;
use crate::kinds::common::facing;
use crate::kinds::player::flag1;
use crate::object::{ObjectRef, PanelPos};
use nettai_content_api::RecordHandle;

/// `ho_8026554` as content sees it: the panel, or (0, 0x7F) when no panel
/// fits (`sub_80265D0`'s registers then; the rush stays on its own panel).
pub(crate) fn lockon_panel(b: &Battle, r: ObjectRef, target: PanelPos, mode: Option<RecordHandle>) -> PanelPos {
    destination(b, r, target, mode).unwrap_or(PanelPos { x: 0, y: 0x7F })
}

/// `ho_8026554`: the panel to attack `target` from in lock-on `mode`;
/// None to stay. No mode (the original's mode 0), or a target off the
/// field's playable panels, is the navi's own panel.
fn destination(b: &Battle, r: ObjectRef, target: PanelPos, mode: Option<RecordHandle>) -> Option<PanelPos> {
    use crate::content::LockonRule;
    let Some(mode) = mode.filter(|_| field::is_valid(target.x, target.y)) else {
        // sub_802661C: the navi's own panel.
        return Some(b.objects.get(r).panel);
    };
    let m = b.content.lockon(mode).unwrap_or_else(|| panic!("record {mode:?} is no lock-on mode (the API checks)"));
    let found = match m.rule {
        LockonRule::Stay => return Some(b.objects.get(r).panel),
        LockonRule::Row => search_row(b, r, target, m),
        LockonRule::Near => search_near(b, r, target, m),
    };
    if !m.prefers_middle_row {
        return found;
    }
    // sub_80265FE: the middle row of that column, if it will do. (Its
    // not-found test reads flags a `mov` just set, so it always runs.)
    let x = found.map_or(0, |p| p.x);
    if can_stand(b, r, x, 2) { Some(PanelPos { x, y: 2 }) } else { found }
}

/// `sub_8026622`: along the target's row from the navi's own column (the
/// same-row list when they share a row), then along the rows the mode's
/// shifts away from the target's, with the plain list.
fn search_row(b: &Battle, r: ObjectRef, target: PanelPos, m: &crate::content::LockonMode) -> Option<PanelPos> {
    let own = b.objects.get(r).panel;
    let first = if target.y == own.y { &m.same_row_offsets } else { &m.offsets };
    let x = own.x as i32;
    let rows = std::iter::once((target.y as i32, first)).chain(m.row_shifts.iter().map(|&s| (target.y as i32 + s as i32, &m.offsets)));
    rows.into_iter().find_map(|(y, offsets)| search_from(b, r, target, x, y, offsets, false))
}

/// `sub_80265D0` (with the column shifts) or `sub_8026450` /
/// `sub_80264A8` alone: next to the target, from the far-column list
/// when the target stands in the column farthest ahead.
fn search_near(b: &Battle, r: ObjectRef, target: PanelPos, m: &crate::content::LockonMode) -> Option<PanelPos> {
    let o = b.objects.get(r);
    let front = facing(o.alliance, o.flip);
    // sub_80266BA: column 6 facing right, 1 facing left.
    let far_column = if front > 0 { 6 } else { 1 };
    let offsets = match &m.far_column_offsets {
        Some(far) if target.x == far_column => far,
        _ => &m.offsets,
    };
    let shifts: &[i8] = if m.column_shifts { &b.game_rules().lockon.column_shifts } else { &[] };
    std::iter::once(0).chain(shifts.iter().copied()).find_map(|shift| {
        let x = target.x as i32 + front * shift as i32;
        search_from(b, r, target, x, target.y as i32, offsets, m.clear_path)
    })
}

/// `sub_8026450` (`sub_80264A8` with `clear_path`): the first panel of
/// `offsets` from (x, y) the navi can stand on, on the field's columns,
/// not past the target's column, and with a clear path to it if asked.
fn search_from(
    b: &Battle,
    r: ObjectRef,
    target: PanelPos,
    x: i32,
    y: i32,
    offsets: &[PanelOffset],
    clear_path: bool,
) -> Option<PanelPos> {
    let o = b.objects.get(r);
    let front = facing(o.alliance, o.flip);
    offsets.iter().find_map(|off| {
        let cx = x + front * off.dx as i32;
        if !(0..=6).contains(&cx) || cx * front > target.x as i32 * front {
            return None;
        }
        let cy = y + off.dy as i32;
        if !(0..=3).contains(&cy) {
            return None;
        }
        let p = PanelPos { x: cx as u8, y: cy as u8 };
        (can_stand(b, r, p.x, p.y) && (!clear_path || path_clear(b, r, p, target))).then_some(p)
    })
}

/// `sub_8026510`: every panel from `from` toward the front, up to the
/// target's column, meets the clear-path condition (a panel off the field
/// never does).
fn path_clear(b: &Battle, r: ObjectRef, from: PanelPos, target: PanelPos) -> bool {
    let o = b.objects.get(r);
    let front = facing(o.alliance, o.flip);
    let rule = b.game_rules().lockon.clear_path[o.alliance as usize & 1];
    let mut x = from.x as i32;
    loop {
        if !(0..=0xFF).contains(&x) || !b.field.meets(x as u8, from.y, rule) {
            return false;
        }
        x += front;
        if x == target.x as i32 {
            return true;
        }
    }
}

/// `sub_800E680`: the navi could stand on (x, y), whichever side owns it.
fn can_stand(b: &Battle, r: ObjectRef, x: u8, y: u8) -> bool {
    if !field::is_valid(x, y) {
        return false;
    }
    let o = b.objects.get(r);
    let floor_free = flag1(b, r) & f1::AIRSHOE != 0 || !b.field.is_solid(o.panel.x, o.panel.y);
    let rule = b.game_rules().panels.any_side_step.get(floor_free, o.alliance);
    b.field.meets(x, y, rule)
}
