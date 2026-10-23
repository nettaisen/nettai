//! What a side presses (docs/design/training-ai.md §4).
//!
//! - In the fight, one of [`spec::FIGHT_ACTIONS`] joint actions, held until
//!   the next decision: a direction (none, up, down, toward the other side,
//!   away from it) × A held or not × B held or not ([`fight_buttons`]).
//! - On the custom screen, a pointer at one of its 12 slots or one of seven
//!   keys ([`spec::SCREEN_KEYS`]), which the [`Navigator`] presses: it walks
//!   the cursor to a slot along the rules' own layout
//!   (`Screen::target_from`) and presses A, or presses a key once. A pick
//!   that changes nothing (a screen that refuses it) is masked for the rest
//!   of that screen.

use crate::spec;
use nettai_battle::Battle;
use nettai_battle::battle::mode;
use nettai_battle::content::{CustomScreenLayout, ScreenKey};
use nettai_battle::custom::screen::{MAX_SELECTIONS, OK_SLOT, Phase, SLOTS, Screen, SlotKind, SlotState};
use nettai_battle::input::keys;
use std::collections::VecDeque;

/// The directions of a fight action, by its index's quotient by 4: none,
/// up, down, toward the other side, away from it.
pub const DIRECTIONS: usize = 5;

/// The buttons fight action `action` holds for side `side` (toward the
/// other side is right for side 0, left for side 1). Out of range: none.
pub fn fight_buttons(action: i32, side: u8) -> u16 {
    if !(0..spec::FIGHT_ACTIONS as i32).contains(&action) {
        return 0;
    }
    let a = action as u16;
    let (toward, away) = if side & 1 == 0 { (keys::RIGHT, keys::LEFT) } else { (keys::LEFT, keys::RIGHT) };
    let dir = [0, keys::UP, keys::DOWN, toward, away][(a / 4) as usize];
    dir | if a & 2 != 0 { keys::A } else { 0 } | if a & 1 != 0 { keys::B } else { 0 }
}

/// The fight action holding `buttons` for side `side` (directions other
/// than one of the five, and other buttons, left out): what a scripted
/// player's buttons are as an action, for its measures.
pub fn fight_action(buttons: u16, side: u8) -> i32 {
    let (toward, away) = if side & 1 == 0 { (keys::RIGHT, keys::LEFT) } else { (keys::LEFT, keys::RIGHT) };
    let dir = if buttons & keys::UP != 0 {
        1
    } else if buttons & keys::DOWN != 0 {
        2
    } else if buttons & toward != 0 {
        3
    } else if buttons & away != 0 {
        4
    } else {
        0
    };
    dir * 4 + if buttons & keys::A != 0 { 2 } else { 0 } + if buttons & keys::B != 0 { 1 } else { 0 }
}

/// The custom screen's keys, as [`spec::SCREEN_KEYS`] orders them.
pub const KEYS: [u16; 7] = [keys::UP, keys::DOWN, keys::LEFT, keys::RIGHT, keys::A, keys::B, keys::START];

const MOVES: [(u16, ScreenKey); 4] = [(keys::UP, ScreenKey::Up), (keys::DOWN, ScreenKey::Down), (keys::LEFT, ScreenKey::Left), (keys::RIGHT, ScreenKey::Right)];

/// How many presses of a direction take the cursor from where it is to
/// each slot, by the screen's own moves (none: it can't get there).
pub fn steps(screen: &Screen, layout: &CustomScreenLayout) -> [Option<u8>; SLOTS] {
    let mut out = [None; SLOTS];
    out[screen.cursor as usize] = Some(0);
    let mut queue = VecDeque::from([screen.cursor]);
    while let Some(at) = queue.pop_front() {
        let here = out[at as usize].unwrap_or(0);
        for (_, key) in MOVES {
            let Some(to) = screen.target_from(at, key, layout).filter(|t| (*t as usize) < SLOTS) else { continue };
            if out[to as usize].is_none() {
                out[to as usize] = Some(here + 1);
                queue.push_back(to);
            }
        }
    }
    out
}

/// The first direction of the shortest way from the cursor to `target`.
fn toward(screen: &Screen, layout: &CustomScreenLayout, target: u8) -> Option<u16> {
    let mut first: [Option<u16>; SLOTS] = [None; SLOTS];
    let mut seen = [false; SLOTS];
    seen[screen.cursor as usize] = true;
    let mut queue = VecDeque::from([screen.cursor]);
    while let Some(at) = queue.pop_front() {
        for (dir, key) in MOVES {
            let Some(to) = screen.target_from(at, key, layout).filter(|t| (*t as usize) < SLOTS && !seen[*t as usize]) else { continue };
            seen[to as usize] = true;
            first[to as usize] = first[at as usize].or(Some(dir));
            if to == target {
                return first[to as usize];
            }
            queue.push_back(to);
        }
    }
    None
}

/// Whether a screen reads its player's keys now: choosing, or a window of
/// the rules up.
pub fn reads_keys(screen: &Screen) -> bool {
    matches!(screen.phase, Phase::Choosing | Phase::Window { .. })
}

/// The slots a pointer may be at (§4.2): while the screen is choosing, a
/// chip it can still pick (selectable, the selection not full) or a
/// button of the rules with uses left, or OK, that the cursor can reach and
/// the screen hasn't refused (`refused`, a bit a slot); with `only_ok`, OK
/// alone. While a window is up, none: keys only.
pub fn legal_slots(screen: &Screen, steps: &[Option<u8>; SLOTS], refused: u16, only_ok: bool) -> [bool; SLOTS] {
    let mut out = [false; SLOTS];
    if screen.phase != Phase::Choosing {
        return out;
    }
    for (i, s) in screen.slots.iter().enumerate() {
        let wanted = match s.kind {
            SlotKind::Ok => true,
            _ if only_ok => false,
            SlotKind::Chip { .. } | SlotKind::Offered(_) => s.state == SlotState::Selectable && (screen.selected as usize) < MAX_SELECTIONS,
            SlotKind::Button { .. } => s.state == SlotState::Selectable && s.uses_left > 0,
            SlotKind::Empty | SlotKind::Hidden => false,
        };
        out[i] = wanted && steps[i].is_some() && refused & (1 << i) == 0;
    }
    out
}

/// The screen actions allowed now: the legal slots, then the keys (all of
/// them while the screen reads keys).
pub fn legal_screen(screen: &Screen, layout: &CustomScreenLayout, refused: u16, only_ok: bool) -> [bool; spec::SCREEN_ACTIONS] {
    let mut out = [false; spec::SCREEN_ACTIONS];
    if !reads_keys(screen) {
        return out;
    }
    let slots = legal_slots(screen, &steps(screen, layout), refused, only_ok);
    out[..SLOTS].copy_from_slice(&slots);
    let keys = !(only_ok && screen.phase == Phase::Choosing);
    for k in 0..KEYS.len() {
        out[SLOTS + k] = keys;
    }
    out
}

/// What a screen shows that a pick changes: whether a press of A did
/// anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Picked {
    selected: u8,
    selection: [u8; MAX_SELECTIONS],
    states: [SlotState; SLOTS],
    phase: std::mem::Discriminant<Phase>,
    form: Option<nettai_content_api::FormHandle>,
    cursor: u8,
}

impl Picked {
    fn of(screen: &Screen) -> Picked {
        Picked {
            selected: screen.selected,
            selection: screen.selection,
            states: screen.slots.map(|s| s.state),
            phase: std::mem::discriminant(&screen.phase),
            form: screen.form,
            cursor: screen.cursor,
        }
    }
}

/// What the navigator is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Doing {
    /// Waiting for a decision.
    #[default]
    Idle,
    /// Walking the cursor to a slot, then pressing A on it.
    Going(u8),
    /// Pressing a key once.
    Key(u16),
}

/// Presses the custom screen's buttons for a pointer (§4.2), a tick at a
/// time: a direction is held two ticks (the screen moves on its second,
/// its auto-repeat's) and let go a tick; A and the other keys are held a
/// tick and let go a tick. Whether a pick changed the screen is looked at
/// once A is let go: one that didn't is refused for the rest of the
/// screen.
#[derive(Clone, Copy, Debug, Default)]
pub struct Navigator {
    doing: Doing,
    /// The button held and the ticks left to hold it, then the tick let go.
    held: u16,
    hold: u8,
    release: u8,
    /// The screen as it was when A was pressed on a slot, for the slot.
    pressed: Option<(u8, Picked)>,
    /// Slots refused on this screen (a bit a slot).
    pub refused: u16,
    /// Presses made toward the current target, to give up on one the
    /// cursor never reaches.
    tries: u8,
}

/// Presses toward a target at most, before it is given up.
const MOST_PRESSES: u8 = 24;

impl Navigator {
    /// A new screen: nothing refused.
    pub fn reset(&mut self) {
        *self = Navigator::default();
    }

    /// Whether it waits for a decision.
    pub fn idle(&self) -> bool {
        self.doing == Doing::Idle && self.hold == 0 && self.release == 0
    }

    /// Point at screen action `action` (a slot, then a key: §6.6's
    /// numbering).
    pub fn point(&mut self, action: i32) {
        self.tries = 0;
        self.doing = match action {
            a if (0..SLOTS as i32).contains(&a) => Doing::Going(a as u8),
            a if (SLOTS as i32..spec::SCREEN_ACTIONS as i32).contains(&a) => Doing::Key(KEYS[a as usize - SLOTS]),
            _ => Doing::Idle,
        };
    }

    /// This tick's buttons on side `side`'s screen in `b`.
    pub fn buttons(&mut self, b: &Battle, side: u8) -> u16 {
        let s = &b.custom.sides[side as usize];
        let Some(screen) = s.screen.as_ref().filter(|_| b.round.mode == mode::CUSTOM) else {
            return 0;
        };
        if self.hold > 0 {
            self.hold -= 1;
            return self.held;
        }
        if self.release > 0 {
            self.release -= 1;
            if self.release == 0
                && let Some((slot, before)) = self.pressed.take()
                && Picked::of(screen) == before
            {
                // (A pick that changed nothing: the screen refuses it.)
                self.refused |= 1 << slot;
            }
            return 0;
        }
        if !reads_keys(screen) {
            return 0;
        }
        let layout = &b.game_rules().custom_screen;
        match self.doing {
            Doing::Idle => 0,
            Doing::Key(key) => {
                self.doing = Doing::Idle;
                self.press(key)
            }
            Doing::Going(target) => {
                if screen.phase != Phase::Choosing || self.tries >= MOST_PRESSES {
                    self.doing = Doing::Idle;
                    return 0;
                }
                self.tries += 1;
                if screen.cursor == target {
                    self.doing = Doing::Idle;
                    self.pressed = Some((target, Picked::of(screen)));
                    return self.press(keys::A);
                }
                match toward(screen, layout, target) {
                    Some(dir) => self.press(dir),
                    None => {
                        self.refused |= 1 << target;
                        self.doing = Doing::Idle;
                        0
                    }
                }
            }
        }
    }

    /// Press `key` this tick: a direction held one tick more, then let go.
    fn press(&mut self, key: u16) -> u16 {
        let direction = key & (keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT) != 0;
        self.held = key;
        self.hold = direction as u8;
        self.release = 1;
        key
    }
}

/// A scripted player's custom screen (§9's players, and the env's seats
/// that no policy plays): every chip it can pick, in slot order, then OK,
/// pointed at through the navigator. A chip that would stand alone as a
/// modifier is passed over.
pub fn scripted_pointer(b: &Battle, side: u8, nav: &Navigator) -> i32 {
    let s = &b.custom.sides[side as usize];
    let Some(screen) = s.screen.as_ref() else { return OK_SLOT as i32 };
    if screen.phase != Phase::Choosing {
        // (A window it didn't open: B closes it.)
        return (SLOTS + 5) as i32;
    }
    let layout = &b.game_rules().custom_screen;
    let legal = legal_slots(screen, &steps(screen, layout), nav.refused, false);
    let last = screen.selection().last().and_then(|&slot| screen.chip_in(slot, &s.folder)).map(|c| c.id);
    let pick = (0..SLOTS).find(|&i| {
        legal[i]
            && matches!(screen.slots[i].kind, SlotKind::Chip { .. } | SlotKind::Offered(_))
            && screen.chip_in(i as u8, &s.folder).is_some_and(|c| !nettai_battle::custom::builder::stays_unfolded(&*b.content, last, c.id))
    });
    pick.unwrap_or(OK_SLOT as usize) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fight_actions_are_directions_by_a_by_b_mirrored_for_side_1() {
        assert_eq!(fight_buttons(0, 0), 0);
        assert_eq!(fight_buttons(1, 0), keys::B);
        assert_eq!(fight_buttons(2, 0), keys::A);
        assert_eq!(fight_buttons(3 * 4, 0), keys::RIGHT);
        assert_eq!(fight_buttons(3 * 4, 1), keys::LEFT);
        assert_eq!(fight_buttons(4 * 4 + 3, 1), keys::RIGHT | keys::A | keys::B);
        assert_eq!(fight_buttons(20, 0), 0);
        for side in 0..2 {
            for a in 0..20 {
                assert_eq!(fight_action(fight_buttons(a, side), side), a);
            }
        }
    }
}
