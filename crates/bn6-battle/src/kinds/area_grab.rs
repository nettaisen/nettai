//! The AreaGrab and PanelGrab chips' dimming controller (effect object
//! #3, `sub_80E0710`): the usual dimming phases (`dimming`), with an
//! effect that drops grab shots (`grab_shot`) on the first column in
//! front not wholly the user's side's (AreaGrab, Param1 set) or on the
//! first enemy panel in the user's row (PanelGrab), then waits 61 ticks.
//! See docs/engine/chips.md §3.6.8.

use crate::battle::Battle;
use crate::field::{self, pflags};
use crate::kinds::{common, grab_shot};
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, state};
use crate::dimming::{self, DimmingChip};

pub const INDEX: u8 = 3;

/// The controller's own state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    pub chip: DimmingChip,
    /// Param1: a whole column (AreaGrab) rather than one panel.
    pub column: bool,
    /// The damage word (object +0x2C).
    pub damage: u32,
}

fn vars(b: &Battle, r: ObjectRef) -> &Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::AreaGrab(v) => v,
        v => panic!("grab controller with {v:?}"),
    }
}

/// `sub_80E07E0`: the controller for `user`, on its panel. (Its position
/// is register garbage nothing reads.)
pub fn spawn(
    b: &mut Battle,
    user: ObjectRef,
    element: u8,
    params: [u8; 4],
    damage: u32,
    chip: DimmingChip,
) -> Option<ObjectRef> {
    let r = b.objects.spawn(Pool::Effect, INDEX, Vec3::default(), params)?;
    let (panel, alliance, flip) = {
        let o = b.objects.get(user);
        (o.panel, o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.panel = PanelPos { x: panel.x, y: panel.y };
    o.element = element;
    o.related[0] = Some(user);
    o.alliance = alliance;
    o.flip = flip;
    o.vars = crate::kinds::Vars::AreaGrab(Vars { chip, column: params[0] != 0, damage });
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => dimming::begin(b, r),
        state::UPDATE => match b.objects.get(r).action {
            0 => dimming::dim_screen(b, r),
            4 => dimming::show_telop(b, r),
            8 => effect(b, r),
            _ => dimming::undim_screen(b, r),
        },
        _ => dimming::end(b, r),
    }
}

/// `sub_80E0754`: the grab shots (running while dimmed), then 61 ticks.
fn effect(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        let v = vars(b, r).clone();
        let damage = v.damage.wrapping_add(v.chip.bonus as u32);
        let o = b.objects.get(r);
        let element = o.element;
        if v.column {
            // sub_800D58C
            if let Some(x) = column_to_grab(b, r) {
                for y in 1..=3 {
                    grab_shot::spawn(b, r, PanelPos { x, y }, element, damage);
                }
            }
        } else if let Some(p) = edge_panel(b, r) {
            grab_shot::spawn(b, r, p, element, damage);
        }
        let o = b.objects.get_mut(r);
        o.timer = 0x3C;
        o.phase_init = 4;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left < 0 {
        common::set_action(b, r, 0xC);
    }
}

/// `sub_800D5F0`: the panels of column `x` not owned by `side`.
fn foreign_panels(b: &Battle, x: u8, side: u8) -> u8 {
    (1..=3u8).filter(|&y| b.field.panel(x, y).is_some_and(|p| p.alliance != side)).count() as u8
}

/// `sub_800D5BA` then `sub_800D58C`: from the column nearest the user
/// wholly its side's, forward to the first that isn't.
fn column_to_grab(b: &Battle, r: ObjectRef) -> Option<u8> {
    let o = b.objects.get(r);
    let (side, front) = (o.alliance, common::facing(o.alliance, o.flip));
    // sub_800D5BA: back from the user's column, then (none found) forward.
    let mut x = o.panel.x as i32;
    let mut step = -front;
    let start = loop {
        if !field::is_valid(x as u8, 1) {
            if step == front {
                panic!("no column of the user's side (sub_800D5BA loops) is not handled");
            }
            x = o.panel.x as i32;
            step = front;
            continue;
        }
        if foreign_panels(b, x as u8, side) == 0 {
            break x;
        }
        x += step;
    };
    // sub_800D58C
    let mut x = start;
    loop {
        if foreign_panels(b, x as u8, side) != 0 {
            return Some(x as u8);
        }
        x += front;
        if !field::is_valid(x as u8, 1) {
            return None;
        }
    }
}

/// `object_getEdgePanelMatchingRow`: in the controller's row, the first
/// panel past the user's side's front.
fn edge_panel(b: &Battle, r: ObjectRef) -> Option<PanelPos> {
    let o = b.objects.get(r);
    let (side, y) = (o.alliance, o.panel.y);
    let dir = common::facing(o.alliance, o.flip);
    // sub_800D53C: from the far edge, back while the panels are the
    // user's side's.
    let mut x = 6 - 5 * (o.alliance ^ o.flip) as i32;
    loop {
        if !field::is_valid(x as u8, y) {
            return None;
        }
        if b.field.panel(x as u8, y).is_some_and(|p| p.alliance != side) {
            break;
        }
        x -= dir;
    }
    // object_getFirstPanelInDirectionFiltered: on back to the user's
    // side's first panel.
    let (require, forbid) = if side == 0 { (0, pflags::ALLIANCE_1) } else { (pflags::ALLIANCE_1, 0) };
    loop {
        if b.field.check(x as u8, y, require, forbid) {
            break;
        }
        x -= dir;
        if !field::is_valid(x as u8, y) {
            return None;
        }
    }
    let x = x + dir;
    field::is_valid(x as u8, y).then_some(PanelPos { x: x as u8, y })
}
