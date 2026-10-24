//! What a side presses (docs/design/training-ai.md §4).
//!
//! - In the fight, one of [`spec::FIGHT_ACTIONS`] joint actions, held until
//!   the next decision: a direction (none, up, down, toward the other side,
//!   away from it) × A held or not × B held or not ([`fight_buttons`]).
//!   Toward is RIGHT for either side: each console shows its own player on
//!   the left.
//! - On the custom screen, what to choose: one of its 12 slots or an entry
//!   of the rules' form list ([`spec::FORM_ENTRIES`]: EXE6's Crosses). The
//!   [`Navigator`] presses the way there and carries the choice out before
//!   the next is asked: it walks the cursor to a slot along the rules' own
//!   layout (`Screen::target_from`) and presses A, or opens the form list
//!   and chooses in it. The network never presses a key of the screen
//!   itself. A choice that changes nothing (a screen that refuses it) is
//!   masked for the rest of that screen.

use crate::spec;
use nettai_battle::Battle;
use nettai_battle::battle::mode;
use nettai_battle::content::{CustomScreenLayout, ScreenKey, WindowHandle, WindowView};
use nettai_battle::custom::screen::{MAX_SELECTIONS, OK_SLOT, Phase, SLOTS, Screen, SlotKind, SlotState};
use nettai_battle::input::keys;
use std::collections::VecDeque;

/// The directions of a fight action, by its index's quotient by 4: none,
/// up, down, toward the other side, away from it.
pub const DIRECTIONS: usize = 5;

/// The buttons fight action `action` holds (toward the other side is a
/// side's RIGHT and away its LEFT, for either side: each console shows its
/// own player on the left, and the engine reads LEFT as back and RIGHT as
/// forward). Out of range: none.
pub fn fight_buttons(action: i32) -> u16 {
    if !(0..spec::FIGHT_ACTIONS as i32).contains(&action) {
        return 0;
    }
    let a = action as u16;
    let dir = [0, keys::UP, keys::DOWN, keys::RIGHT, keys::LEFT][(a / 4) as usize];
    dir | if a & 2 != 0 { keys::A } else { 0 } | if a & 1 != 0 { keys::B } else { 0 }
}

/// The fight action holding `buttons` (directions other than one of the
/// five, and other buttons, left out): what a scripted player's buttons are
/// as an action, for its measures.
pub fn fight_action(buttons: u16) -> i32 {
    let dir = if buttons & keys::UP != 0 {
        1
    } else if buttons & keys::DOWN != 0 {
        2
    } else if buttons & keys::RIGHT != 0 {
        3
    } else if buttons & keys::LEFT != 0 {
        4
    } else {
        0
    };
    dir * 4 + if buttons & keys::A != 0 { 2 } else { 0 } + if buttons & keys::B != 0 { 1 } else { 0 }
}

/// The keys the key probe presses (§4.2), as [`spec::PROBED_KEYS`] names
/// them.
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

/// A move's bit in [`Navigator`]'s blocked moves: slot `at`, direction
/// `dir` (its place in `MOVES`).
fn move_bit(at: u8, dir: usize) -> u64 {
    1 << (at as usize * MOVES.len() + dir)
}
const _: () = assert!(SLOTS * MOVES.len() <= 64);

/// The first direction of the shortest way from the cursor to `target`
/// (its place in `MOVES`), leaving out the moves in `blocked`.
fn toward(screen: &Screen, layout: &CustomScreenLayout, target: u8, blocked: u64) -> Option<usize> {
    let mut first: [Option<usize>; SLOTS] = [None; SLOTS];
    let mut seen = [false; SLOTS];
    seen[screen.cursor as usize] = true;
    let mut queue = VecDeque::from([screen.cursor]);
    while let Some(at) = queue.pop_front() {
        for (dir, (_, key)) in MOVES.into_iter().enumerate() {
            if blocked & move_bit(at, dir) != 0 {
                continue;
            }
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

/// Whether a screen takes a decision now: it is choosing chips. A window
/// of the rules is the navigator's alone.
pub fn chooses(screen: &Screen) -> bool {
    screen.phase == Phase::Choosing
}

/// The slots a pointer may be at (§4.2): while the screen is choosing, a
/// chip it can still pick (selectable, the selection not full) or a
/// button of the rules with uses left, or OK, that the cursor can reach and
/// the screen hasn't refused (`refused`, a bit a slot); with `only_ok`, OK
/// alone. Otherwise none.
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

/// Where the rules' form list opens from (EXE6's Cross window): the key
/// the key probe found opening it, and where the cursor was then.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opener {
    pub key: u16,
    pub cursor: u8,
}

/// The opener among the windows the key probe's keys opened
/// (`env::probe_keys`, by [`KEYS`]), the cursor at `cursor`: the first key
/// whose window is a form list's (its view the list's, or the list's
/// opening).
pub fn opener(b: &Battle, cursor: u8, opened: &[Option<WindowHandle>; KEYS.len()]) -> Option<Opener> {
    KEYS.iter().zip(opened).find_map(|(&key, w)| {
        let view = b.content.defs.window((*w)?).view;
        matches!(view, Some(WindowView::FormListOpening | WindowView::FormList)).then_some(Opener { key, cursor })
    })
}

/// The entries of the rules' form list (EXE6's Crosses) an action may
/// choose (§4.2): with an opener whose place the cursor can get back to,
/// while no entry is chosen, each entry that holds a form, isn't marked (a
/// Cross used this battle) and wasn't refused on this screen
/// (`Navigator::refused_forms`).
pub fn legal_forms(b: &Battle, side: u8, steps: &[Option<u8>; SLOTS], nav: &Navigator) -> [bool; spec::FORM_ENTRIES] {
    let mut out = [false; spec::FORM_ENTRIES];
    let (Some(opener), Some(list)) = (nav.opener, b.form_list(side)) else { return out };
    if list.chosen.is_some() || steps[opener.cursor as usize].is_none() {
        return out;
    }
    for (k, legal) in out.iter_mut().enumerate().take(list.count as usize) {
        *legal = list.forms[k].is_some() && !list.marked[k] && nav.refused_forms & (1 << k) == 0;
    }
    out
}

/// The key that backs out of a form list: the navigator closes with it a
/// list it gave up on, or opened on its way to a slot.
pub const CLOSING_KEY: u16 = keys::B;

/// The screen actions allowed now on side `side`'s screen in `b` (§4.2):
/// while it is choosing, the legal slots, then the legal entries of the
/// form list; with `only_ok` (the screen's decisions spent), OK alone.
/// Otherwise none.
pub fn legal_screen(b: &Battle, side: u8, nav: &Navigator, only_ok: bool) -> [bool; spec::SCREEN_ACTIONS] {
    let mut out = [false; spec::SCREEN_ACTIONS];
    let Some(screen) = b.custom.sides[side as usize].screen.as_ref().filter(|s| chooses(s)) else { return out };
    let steps = steps(screen, &b.game_rules().custom_screen);
    out[..SLOTS].copy_from_slice(&legal_slots(screen, &steps, nav.refused, only_ok));
    if !only_ok {
        out[SLOTS..].copy_from_slice(&legal_forms(b, side, &steps, nav));
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
    /// Choosing an entry of the rules' form list.
    Listing(u8),
}

/// How a choice in the form list goes.
#[derive(Clone, Copy, Debug, Default)]
struct Listing {
    /// The opener's key was pressed.
    opened: bool,
    /// The list's cursor before the last move, and the move tried (its
    /// place in `MOVES`): a move that left the cursor where it was is
    /// passed over for the next.
    last: Option<u8>,
    dir: u8,
    /// A was pressed on the entry.
    pressed: bool,
    /// The entry is chosen: the list says so, or its chosen window is up.
    chosen: bool,
    /// Given up: backing out of the list.
    leaving: bool,
}

/// Presses the custom screen's buttons for a screen action (§4.2), a tick
/// at a time, the action done before the next is asked: a direction is
/// held two ticks (the screen moves on its second, its auto-repeat's) and
/// let go a tick; A and the other keys are held a tick and let go a tick.
///
/// - **A slot.** The cursor walks there along the rules' layout and A is
///   pressed. Whether the pick changed the screen is looked at once A is
///   let go: one that didn't is refused for the rest of the screen. A move
///   on the way that opened a window of the rules rather than moving the
///   cursor (EXE6's UP on the top row or OK while a Cross is on offer) is
///   backed out of and left out of the ways for the rest of the screen.
/// - **An entry of the form list.** The cursor walks to the opener's place
///   and the opener's key is pressed; in the list, a direction is pressed
///   till the list's cursor is on the entry (one that doesn't move it is
///   passed over for the next), then A; the list's own windows play out
///   till the screen is choosing again. An entry the list didn't take (the
///   key opened nothing, A chose nothing, no direction got there) is backed
///   out of with B and refused for the rest of the screen.
/// - **A window it didn't open.** A form list is backed out of; the rules'
///   other windows (EXE6's Beast Out, EXE5's soul) play out alone.
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
    /// Entries of the form list refused on this screen (a bit an entry).
    pub refused_forms: u8,
    /// Where the form list opens from on this screen, as the key probe
    /// found it.
    pub opener: Option<Opener>,
    listing: Listing,
    /// Presses made toward the current action, to give up on one that never
    /// gets done.
    tries: u8,
    /// The move last pressed toward a slot (the slot, the direction's
    /// place in `MOVES`), till the screen is looked at after it.
    moved: Option<(u8, usize)>,
    /// The moves that opened a window on this screen (`move_bit`).
    blocked: u64,
}

/// Presses toward an action at most, before it is given up.
const MOST_PRESSES: u8 = 24;

impl Navigator {
    /// A new screen: nothing refused, no opener.
    pub fn reset(&mut self) {
        *self = Navigator::default();
    }

    /// Whether it waits for a decision.
    pub fn idle(&self) -> bool {
        self.doing == Doing::Idle && self.hold == 0 && self.release == 0
    }

    /// Carry out screen action `action` (a slot, then an entry of the form
    /// list: §6.6's numbering); another: nothing.
    pub fn point(&mut self, action: i32) {
        self.tries = 0;
        self.listing = Listing::default();
        self.doing = match action {
            a if (0..SLOTS as i32).contains(&a) => Doing::Going(a as u8),
            a if (SLOTS as i32..spec::SCREEN_ACTIONS as i32).contains(&a) => Doing::Listing((a as usize - SLOTS) as u8),
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
        if let Some((at, dir)) = self.moved.take()
            && matches!(screen.phase, Phase::Window { .. })
        {
            self.blocked |= move_bit(at, dir);
        }
        if let Phase::Window { window, .. } = screen.phase {
            let view = b.content.defs.window(window).view;
            return match self.doing {
                Doing::Listing(entry) => self.in_list(b, side, entry, view),
                _ if view == Some(WindowView::FormList) => self.press(CLOSING_KEY),
                _ => 0,
            };
        }
        if !chooses(screen) {
            return 0;
        }
        let layout = &b.game_rules().custom_screen;
        match self.doing {
            Doing::Idle => 0,
            Doing::Going(target) => {
                if self.tries >= MOST_PRESSES {
                    self.refused |= 1 << target;
                    self.doing = Doing::Idle;
                    return 0;
                }
                self.tries += 1;
                if screen.cursor == target {
                    self.doing = Doing::Idle;
                    self.pressed = Some((target, Picked::of(screen)));
                    return self.press(keys::A);
                }
                self.walk(screen, layout, target).unwrap_or_else(|| {
                    self.refused |= 1 << target;
                    self.doing = Doing::Idle;
                    0
                })
            }
            Doing::Listing(entry) => {
                let l = self.listing;
                if l.chosen {
                    self.doing = Doing::Idle;
                    return 0;
                }
                let opener = self.opener.filter(|_| !l.opened && !l.leaving && self.tries < MOST_PRESSES);
                let walked = opener.and_then(|o| {
                    if screen.cursor == o.cursor {
                        self.listing.opened = true;
                        Some(self.press(o.key))
                    } else {
                        self.walk(screen, layout, o.cursor)
                    }
                });
                self.tries += 1;
                walked.unwrap_or_else(|| {
                    // (Choosing again without the entry: the key opened
                    // nothing, the list was given up on, or the cursor can't
                    // get to the opener's place.)
                    self.refused_forms |= 1 << entry;
                    self.doing = Doing::Idle;
                    0
                })
            }
        }
    }

    /// A press toward slot `target` along the screen's moves, leaving out
    /// the moves that opened a window; none: there is no way there.
    fn walk(&mut self, screen: &Screen, layout: &CustomScreenLayout, target: u8) -> Option<u16> {
        let dir = toward(screen, layout, target, self.blocked)?;
        self.moved = Some((screen.cursor, dir));
        Some(self.press(MOVES[dir].0))
    }

    /// A tick in a window of the rules while choosing entry `entry` of the
    /// form list, the window's view `view`.
    fn in_list(&mut self, b: &Battle, side: u8, entry: u8, view: Option<WindowView>) -> u16 {
        match view {
            Some(WindowView::FormChosen) => {
                self.listing.chosen = true;
                0
            }
            Some(WindowView::FormList) => {
                let Some(list) = b.form_list(side) else { return self.back_out() };
                if list.chosen.is_some() {
                    self.listing.chosen = true;
                    return 0;
                }
                let l = self.listing;
                if l.leaving || l.pressed || self.tries >= MOST_PRESSES {
                    // (Given up, or A chose nothing: the list refuses the entry.)
                    return self.back_out();
                }
                self.tries += 1;
                if list.cursor == entry {
                    self.listing.pressed = true;
                    return self.press(keys::A);
                }
                if l.last == Some(list.cursor) {
                    self.listing.dir += 1;
                }
                let Some(&(key, _)) = MOVES.get(self.listing.dir as usize) else { return self.back_out() };
                self.listing.last = Some(list.cursor);
                self.press(key)
            }
            // (The list opening or closing, the rules' other windows: they
            // play out.)
            _ => 0,
        }
    }

    /// Back out of the form list: B, the entry given up on.
    fn back_out(&mut self) -> u16 {
        self.listing.leaving = true;
        self.press(CLOSING_KEY)
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
/// modifier is passed over. In a window, nothing: the navigator's.
pub fn scripted_pointer(b: &Battle, side: u8, nav: &Navigator) -> i32 {
    let s = &b.custom.sides[side as usize];
    let Some(screen) = s.screen.as_ref() else { return OK_SLOT as i32 };
    if !chooses(screen) {
        return -1;
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
    fn fight_actions_are_directions_by_a_by_b() {
        assert_eq!(fight_buttons(0), 0);
        assert_eq!(fight_buttons(1), keys::B);
        assert_eq!(fight_buttons(2), keys::A);
        assert_eq!(fight_buttons(3 * 4), keys::RIGHT);
        assert_eq!(fight_buttons(4 * 4 + 3), keys::LEFT | keys::A | keys::B);
        assert_eq!(fight_buttons(20), 0);
        for a in 0..20 {
            assert_eq!(fight_action(fight_buttons(a)), a);
        }
    }
}
