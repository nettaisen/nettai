//! Beast Over's berserk controller (`sub_802D322`): in forms 0x17 and
//! 0x18 the navi's idle action doesn't read the player's buttons; it
//! cycles on its own through moving next to an opponent, using the next
//! chip, and firing the buster. Its state lives in the last 0x10 bytes of
//! the navi's AIData (+0xF0), which the form's flags clear
//! (`sub_802D310`).

use super::{ai, ai_mut, flag1};
use super::actions::movement::{self, MoveKind};
use crate::actor::request;
use crate::battle::Battle;
use crate::collision::f1;
use crate::content::PanelCondition;
use bn6_content_api::ChipHandle;
use crate::object::{ObjectRef, PanelPos};

/// Where the controller is (+0, read as a jump-table offset).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Step {
    /// `sub_802D358`: use the next chip.
    #[default]
    UseChip,
    /// `sub_802D3A8`: fire the buster.
    Buster,
    /// `sub_802D3CA`: step next to an opponent.
    Move,
}

/// The controller's state (AIData+0xF0..+0xFF).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct State {
    /// +0.
    pub step: Step,
    /// +4: moves since the last chip or buster shot.
    pub moves: u16,
    /// +6: the Cross special's buster volley: shots left while an opponent
    /// stays in the row (`sub_802D588`).
    pub volley: u16,
    /// +8: the controller has started (its first step is a move).
    pub started: bool,
}

/// What a tick of the controller did (`sub_802D322`'s result).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing started (0).
    Nothing,
    /// A chip's action started (1).
    Chip,
    /// The buster is to fire (2).
    Buster,
    /// A step started (3).
    Moved,
}

/// `sub_802D310`: clear the controller's state.
pub(super) fn reset(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).berserk = State::default();
}

/// `sub_802D322`: one tick of the controller, from the idle action.
pub(super) fn control(b: &mut Battle, r: ObjectRef) -> Outcome {
    let s = &mut ai_mut(b, r).berserk;
    if !s.started {
        s.started = true;
        s.step = Step::Move;
    }
    match ai(b, r).berserk.step {
        Step::UseChip => use_chip(b, r),
        Step::Buster => buster(b, r),
        Step::Move => step_toward_opponent(b, r),
    }
}

fn set_step(b: &mut Battle, r: ObjectRef, step: Step) {
    ai_mut(b, r).berserk.step = step;
}

/// `sub_802D358`: use the next chip, unless (for the first three moves
/// after a chip) the opponent under the lock-on marker is flashing, or no
/// chip is left or it can't start; then the buster is next.
fn use_chip(b: &mut Battle, r: ObjectRef) -> Outcome {
    if ai(b, r).berserk.moves <= 3 {
        let p = crate::kinds::lockon_marker::panel_of(b, ai(b, r).lockon_marker);
        let alliance = b.objects.get(r).alliance;
        if let Some(t) = opponent_on(b, p, alliance) {
            let flashing = b.objects.get(t).collision.is_some_and(|c| b.collision.get(c).f1 & f1::FLASHING != 0);
            if flashing {
                set_step(b, r, Step::Buster);
                return Outcome::Nothing;
            }
        }
    }
    if super::next_chip(b, r).is_none() {
        set_step(b, r, Step::Buster);
        return Outcome::Nothing;
    }
    ai_mut(b, r).requests |= request::CHIP;
    if super::chip_use::use_chip(b, r).is_none() {
        set_step(b, r, Step::Buster);
        return Outcome::Nothing;
    }
    let s = &mut ai_mut(b, r).berserk;
    s.moves = 0;
    s.step = Step::Move;
    Outcome::Chip
}

/// `sub_802D3A8`: after three moves, or with an opponent in the navi's
/// row, fire the buster; a move is next either way.
fn buster(b: &mut Battle, r: ObjectRef) -> Outcome {
    set_step(b, r, Step::Move);
    let fire = ai(b, r).berserk.moves > 2 || {
        let y = b.objects.get(r).panel.y;
        opponent_in_row(b, r, y).is_some()
    };
    if !fire {
        return Outcome::Nothing;
    }
    ai_mut(b, r).berserk.moves = 0;
    Outcome::Buster
}

/// `sub_802D3CA`: step to a panel near an opponent (`sub_802D430`); a
/// chip is next.
fn step_toward_opponent(b: &mut Battle, r: ObjectRef) -> Outcome {
    let rules = b.content.rules.berserk;
    let floor_free = flag1(b, r) & f1::AIRSHOE != 0;
    let cond = rules.step.get(floor_free, b.objects.get(r).alliance);
    let target = choose_panel(b, r, cond);
    movement::start_absolute(b, r, target, 8, MoveKind::Absolute);
    let s = &mut ai_mut(b, r).berserk;
    s.moves = s.moves.wrapping_add(1);
    s.step = Step::UseChip;
    Outcome::Moved
}

// ---- The Cross special (DarkInvs) --------------------------------------------

/// `byte_802D5E4`: the chips the Cross special uses, three by its navi's
/// base max HP in hundreds (1..=9 and up), and six for 1000 and up.
///
/// `sub_802D4C6`: one tick of the Cross special's controller (the dark
/// chips' auto-battle, while the side's Cross special runs): like Beast
/// Over's, it moves next to an opponent and attacks, but with chips of its
/// own, and a buster volley while an opponent stays in its row.
pub(super) fn cross_special(b: &mut Battle, r: ObjectRef) -> Outcome {
    match ai(b, r).berserk.step {
        Step::UseChip => special_chip(b, r),
        Step::Buster => volley(b, r),
        Step::Move => step_toward_opponent(b, r),
    }
}

/// `sub_802D4F0`: (the first time, sound 0x182) for the first three moves
/// after an attack, wait with a volley while the opponent under the lock-on
/// marker flashes; otherwise use one of the Cross special's chips (not the
/// hand's: the attack variables come from its data, and the chip isn't
/// consumed) and move next.
fn special_chip(b: &mut Battle, r: ObjectRef) -> Outcome {
    if !ai(b, r).berserk.started {
        ai_mut(b, r).berserk.started = true;
        b.sound(crate::content::SoundRole::CrossSpecial);
    }
    if ai(b, r).berserk.moves <= 3 {
        // sub_80E164A (outside Beast Out there is no marker: nobody).
        let p = crate::kinds::lockon_marker::panel_of(b, ai(b, r).lockon_marker);
        let target = opponent_on(b, p, b.objects.get(r).alliance);
        let flashing =
            target.is_some_and(|t| b.objects.get(t).collision.is_some_and(|c| b.collision.get(c).f1 & f1::FLASHING != 0));
        if flashing {
            let volley = super::form_of(b, r).special_volley;
            let s = &mut ai_mut(b, r).berserk;
            s.step = Step::Buster;
            // The game means 6 (2 in Beast Out) but stores the form's
            // number (a register mixup), which is what a form's
            // `special_volley` says: in the base form (0) the volley
            // lasts until the row is clear (or 65536 shots).
            s.volley = volley;
            return Outcome::Nothing;
        }
    }
    ai_mut(b, r).berserk.moves = 0;
    let (chip, damage_of) = pick_special_chip(b, r);
    let content = b.content.clone();
    let cd = content.chip(chip);
    let a = &mut ai_mut(b, r).attack;
    a.chip = Some(chip);
    // (The original copies the record's subtype and parameter bytes too:
    // a chip has neither here; what its action needs is its definition's.)
    a.damage = cd.damage;
    a.hit_param = (cd.hit_param | 0x80) as u16;
    // (The last rows' LifeSrd strikes with VarSwrd's damage.)
    if let Some(other) = damage_of {
        a.damage = content.chip(other).damage;
    }
    let action = super::chip_use::chip_action(b, r, Some(chip));
    super::set_attack(b, r, action, 5);
    ai_mut(b, r).attack.beast_lockon = cd.beast_lockon as u8;
    ai_mut(b, r).berserk.step = Step::Move;
    Outcome::Chip
}

/// `sub_802D5A8`: a chip for the Cross special, at random from its navi's
/// row of the rules' Cross special chips (by the hundreds of the base max
/// HP, NaviStats+0x3E: the first row up to 199, the tenth from 1000), with
/// the chip whose damage it strikes with, if another's.
fn pick_special_chip(b: &mut Battle, r: ObjectRef) -> (ChipHandle, Option<ChipHandle>) {
    let hundreds = super::stats(b, r).max_base_hp / 100;
    let content = b.content.clone();
    let rows = &content.defs.cross_special;
    let row = (hundreds.saturating_sub(1) as usize).min(rows.len().saturating_sub(1));
    let chips = rows
        .get(row)
        .filter(|chips| !chips.is_empty())
        .unwrap_or_else(|| panic!("content error: the Cross special has no chips for row {row} (rules cross-special)"));
    let i = b.rng.next_positive() % chips.len() as u32;
    chips[i as usize]
}

/// `sub_802D588`: the volley: the buster fires while an opponent is in the
/// navi's row and shots are left; then a move is next.
fn volley(b: &mut Battle, r: ObjectRef) -> Outcome {
    let y = b.objects.get(r).panel.y;
    if opponent_in_row(b, r, y).is_some() {
        let s = &mut ai_mut(b, r).berserk;
        s.volley = s.volley.wrapping_sub(1);
        if s.volley != 0 {
            return Outcome::Buster;
        }
    }
    ai_mut(b, r).berserk.step = Step::Move;
    Outcome::Nothing
}

/// `sub_80E7486`: the opposing player standing on `p` (the panel shows
/// the other side's player flag and an alive actor's collision sits
/// there).
fn opponent_on(b: &Battle, p: PanelPos, alliance: u8) -> Option<ObjectRef> {
    let flag = b.content.rules.berserk.opposing_player[alliance as usize & 1];
    if b.field.flags(p.x, p.y) & flag == 0 {
        return None;
    }
    b.round.alive_actors.iter().flatten().flatten().copied().find(|&o| {
        b.objects.get(o).collision.is_some_and(|c| b.collision.get(c).panel == p)
    })
}

/// `sub_810971A`: an opponent's panel in row `y`, picked at random
/// among them (one RNG draw when there is one).
fn opponent_in_row(b: &mut Battle, r: ObjectRef, y: u8) -> Option<PanelPos> {
    let cond = b.content.rules.berserk.opponent[b.objects.get(r).alliance as usize & 1];
    let panels = panels_in_row(b, y, cond);
    pick(b, &panels)
}

/// `object_getPanelsInRowFiltered`: the panels of row `y` meeting `cond`,
/// left to right.
fn panels_in_row(b: &Battle, y: u8, cond: PanelCondition) -> Vec<PanelPos> {
    (1..=6).filter(|&x| b.field.meets(x, y, cond)).map(|x| PanelPos { x, y }).collect()
}

/// A random one of `panels` (`GetPositiveSignedRNG2` modulo their count;
/// no draw when there are none).
fn pick(b: &mut Battle, panels: &[PanelPos]) -> Option<PanelPos> {
    if panels.is_empty() {
        return None;
    }
    let i = b.rng.next_positive() % panels.len() as u32;
    Some(panels[i as usize])
}

/// A panel list entry as the game keeps it: `x | y << 4` in a byte.
fn pack(p: PanelPos) -> u8 {
    p.x | p.y << 4
}

fn unpack(v: u8) -> PanelPos {
    PanelPos { x: v & 7, y: v >> 4 }
}

/// `sub_802D430`: where to step. For each row with an opponent, a panel
/// behind it (toward the navi's side) that meets `cond`; if none of those
/// turns up, any panel meeting `cond` in the rows with an opponent; if
/// none, the navi's own panel. The navi's own panel counts as reserved
/// meanwhile.
fn choose_panel(b: &mut Battle, r: ObjectRef, cond: PanelCondition) -> PanelPos {
    let own = b.objects.get(r).panel;
    b.reserve_panel(r, own.x, own.y);
    let mut entries: Vec<u8> = Vec::new();
    let mut count = 0;
    for y in 1..=3 {
        if let Some(opponent) = opponent_in_row(b, r, y) {
            let v = behind(b, r, opponent, cond);
            entries.push(v);
            if v != 0 {
                count += 1;
            }
        }
    }
    if count == 0 {
        entries.clear();
        for y in 1..=3 {
            if opponent_in_row(b, r, y).is_some() {
                entries.extend(panels_in_row(b, y, cond).into_iter().map(pack));
            }
        }
        count = entries.len();
    }
    let target = if count == 0 {
        own
    } else {
        // The draw only covers the first `count` entries, which with the
        // first search's misses among them may include a miss.
        let i = b.rng.next_positive() % count as u32;
        unpack(entries[i as usize])
    };
    b.unreserve_panel(r, own.x, own.y);
    target
}

/// `sub_8015CC0(x, y, cond, 0)`: from the opponent at `from`, the panels
/// behind it (toward the navi's side), up to the first one with a
/// neutral object or the navi side's other bodies, that meet `cond`; one
/// at random, packed. None found: 0 if something blocked the way, else
/// the row alone (x 0, the register the scan leaves).
fn behind(b: &mut Battle, r: ObjectRef, from: PanelPos, cond: PanelCondition) -> u8 {
    let alliance = b.objects.get(r).alliance as usize & 1;
    let dir: i32 = if alliance == 0 { -1 } else { 1 };
    let blocking = b.content.rules.berserk.blocking[alliance];
    let mut found = Vec::new();
    let mut x = from.x as i32 + dir;
    let y = from.y;
    let blocked = loop {
        let p = PanelPos { x: x as u8, y };
        if b.field.flags(p.x, p.y) & blocking != 0 {
            break true;
        }
        if b.field.meets(p.x, p.y, cond) {
            found.push(p);
        }
        x += dir;
        if !crate::field::is_valid(x as u8, y) {
            break false;
        }
    };
    match pick(b, &found) {
        Some(p) => pack(p),
        None if blocked => 0,
        None => y << 4,
    }
}
