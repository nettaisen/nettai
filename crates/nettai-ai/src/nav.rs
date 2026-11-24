//! Custom screen navigator: turns high-level screen actions (pick a slot, press
//! a button, or choose a form-list entry) into tick-by-tick `Keys` driven
//! through `nettai_battle::custom::Layer`, generically for any game.

use crate::sim::step_ticks;
use nettai_battle::battle::mode;
use nettai_battle::content::{CustomScreenLayout, ScreenKey, WindowView};
use nettai_battle::custom::layer::{ListStage, Sub};
use nettai_battle::custom::screen::{ButtonCell, FORM_LIST_ENTRIES, MAX_SELECTIONS, Phase, SLOTS, Screen, SlotKind, SlotState};
use nettai_battle::custom::Layer;
use nettai_battle::input::Keys;
use nettai_battle::{Battle, PlayerTick};
use std::collections::VecDeque;

/// A high-level action on the custom screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenAction {
    /// Walk cursor to slot `0..12` (chip, button, or OK) and press A.
    Slot(u8),
    /// Open the rules' form list (e.g. Cross window) and select entry `0..5`.
    Form(u8),
}

/// Keys probed to discover which key opens the form list (e.g. UP on top row in EXE6).
const PROBE_KEYS: [Keys; 7] = [Keys::UP, Keys::DOWN, Keys::LEFT, Keys::RIGHT, Keys::A, Keys::B, Keys::START];
const MOVES: [(Keys, ScreenKey); 4] = [
    (Keys::UP, ScreenKey::Up),
    (Keys::DOWN, ScreenKey::Down),
    (Keys::LEFT, ScreenKey::Left),
    (Keys::RIGHT, ScreenKey::Right),
];
const CLOSING_KEY: Keys = Keys::B;
const MOST_PRESSES: u8 = 24;

/// How to open the rules' form list on this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opener {
    pub key: Keys,
    pub cursor: u8,
}

/// Whether the screen is currently accepting chip/button choices with no sub-window up.
pub fn chooses(screen: &Screen, layer: &Layer) -> bool {
    screen.phase == Phase::Choosing && !layer.is_up()
}

/// Shortest cursor step count from `cursor` to every slot.
pub fn steps(screen: &Screen, cursor: u8, layout: &CustomScreenLayout) -> [Option<u8>; SLOTS] {
    let mut out = [None; SLOTS];
    if (cursor as usize) >= SLOTS {
        return out;
    }
    out[cursor as usize] = Some(0);
    let mut queue = VecDeque::from([cursor]);
    while let Some(at) = queue.pop_front() {
        let here = out[at as usize].unwrap_or(0);
        for (_, key) in MOVES {
            let Some(to) = screen.target_from(at, key, layout).filter(|t| (*t as usize) < SLOTS) else {
                continue;
            };
            if out[to as usize].is_none() {
                out[to as usize] = Some(here + 1);
                queue.push_back(to);
            }
        }
    }
    out
}

fn move_bit(at: u8, dir: usize) -> u64 {
    1u64 << (at as usize * MOVES.len() + dir)
}

fn toward(screen: &Screen, cursor: u8, layout: &CustomScreenLayout, target: u8, blocked: u64) -> Option<usize> {
    let mut first: [Option<usize>; SLOTS] = [None; SLOTS];
    let mut seen = [false; SLOTS];
    seen[cursor as usize] = true;
    let mut queue = VecDeque::from([cursor]);
    while let Some(at) = queue.pop_front() {
        for (dir, (_, key)) in MOVES.into_iter().enumerate() {
            if blocked & move_bit(at, dir) != 0 {
                continue;
            }
            let Some(to) = screen.target_from(at, key, layout).filter(|t| (*t as usize) < SLOTS && !seen[*t as usize]) else {
                continue;
            };
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

/// Legal slots on `screen` reachable from `layer.cursor` and not refused.
pub fn legal_slots(screen: &Screen, layer: &Layer, layout: &CustomScreenLayout, refused: u16, only_ok: bool) -> [bool; SLOTS] {
    let mut out = [false; SLOTS];
    if !chooses(screen, layer) {
        return out;
    }
    let dist = steps(screen, layer.cursor, layout);
    for (i, s) in screen.slots.iter().enumerate() {
        let wanted = match s.kind {
            SlotKind::Ok => true,
            _ if only_ok => false,
            SlotKind::Chip { .. } | SlotKind::Offered(_) => s.state == SlotState::Selectable && (screen.selected as usize) < MAX_SELECTIONS,
            SlotKind::Button { cell, .. } => cell != ButtonCell::Right && s.state == SlotState::Selectable,
            SlotKind::Empty | SlotKind::Hidden => false,
        };
        out[i] = wanted && dist[i].is_some() && refused & (1 << i) == 0;
    }
    out
}

/// Legal entries of the rules' form list (e.g. EXE6 Crosses).
pub fn legal_forms(b: &Battle, side: u8, layer: &Layer, nav: &Navigator) -> [bool; FORM_LIST_ENTRIES] {
    let mut out = [false; FORM_LIST_ENTRIES];
    let Some(screen) = b.custom.sides[side as usize].screen.as_ref().filter(|s| chooses(s, layer)) else {
        return out;
    };
    let (Some(opener), Some(list)) = (nav.opener, b.form_list(side)) else {
        return out;
    };
    let dist = steps(screen, layer.cursor, &b.game_rules().custom_screen);
    if list.chosen.is_some() || dist[opener.cursor as usize].is_none() {
        return out;
    }
    for (k, legal) in out.iter_mut().enumerate().take((list.count as usize).min(FORM_LIST_ENTRIES)) {
        *legal = list.forms[k].is_some() && !list.marked[k] && nav.refused_forms & (1 << k) == 0;
    }
    out
}

/// Probe which key opens the rules' form list from the current cursor position.
pub fn probe_opener(b: &Battle, layer: &Layer, side: u8) -> Option<Opener> {
    if b.form_list(side).is_none() {
        return None;
    }
    for &key in &PROBE_KEYS {
        let mut copy = b.clone();
        let mut mine = *layer;
        let direction = key.intersects(Keys::UP | Keys::DOWN | Keys::LEFT | Keys::RIGHT);
        let none = Keys::empty();
        let presses: &[Keys] = if direction { &[key, key, none, none] } else { &[key, none, none] };
        for &held in presses {
            let mut both = [PlayerTick::default(); 2];
            both[side as usize] = mine.input(PlayerTick::buttons(held), &copy);
            if !step_ticks(&mut copy, both) {
                break;
            }
            mine.follow(&copy);
            if mine.list_stage().is_some() {
                return Some(Opener { key, cursor: layer.cursor });
            }
        }
    }
    None
}

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
    fn of(screen: &Screen, cursor: u8) -> Picked {
        Picked {
            selected: screen.selected,
            selection: screen.selection,
            states: screen.slots.map(|s| s.state),
            phase: std::mem::discriminant(&screen.phase),
            form: screen.form,
            cursor,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Over {
    Nothing,
    ListOpening,
    List,
    ListClosing,
    Chosen,
    Window,
    Busy,
}

fn over(b: &Battle, screen: &Screen, layer: &Layer) -> Over {
    if let Phase::Window { window, .. } = screen.phase {
        return if b.content.defs.window(window).view == Some(WindowView::FormChosen) {
            Over::Chosen
        } else {
            Over::Window
        };
    }
    match layer.sub {
        Sub::None => Over::Nothing,
        Sub::List(ListStage::Opening(_)) => Over::ListOpening,
        Sub::List(ListStage::Up(_)) => Over::List,
        Sub::List(ListStage::Closing(_)) => Over::ListClosing,
        _ => Over::Busy,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Doing {
    #[default]
    Idle,
    Going(u8),
    Listing(u8),
}

#[derive(Clone, Copy, Debug, Default)]
struct Listing {
    opened: bool,
    last: Option<u8>,
    dir: u8,
    pressed: bool,
    chosen: bool,
    leaving: bool,
}

/// Carries out [`ScreenAction`]s tick-by-tick through `Keys`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Navigator {
    doing: Doing,
    held: Keys,
    hold: u8,
    release: u8,
    pressed: Option<(u8, Picked)>,
    pub refused: u16,
    pub refused_forms: u8,
    pub opener: Option<Opener>,
    listing: Listing,
    tries: u8,
    moved: Option<(u8, usize)>,
    blocked: u64,
}

impl Navigator {
    pub fn reset(&mut self) {
        *self = Navigator::default();
    }

    pub fn idle(&self) -> bool {
        self.doing == Doing::Idle && self.hold == 0 && self.release == 0
    }

    pub fn point(&mut self, action: ScreenAction) {
        self.tries = 0;
        self.listing = Listing::default();
        self.doing = match action {
            ScreenAction::Slot(s) if (s as usize) < SLOTS => Doing::Going(s),
            ScreenAction::Form(e) if (e as usize) < FORM_LIST_ENTRIES => Doing::Listing(e),
            _ => Doing::Idle,
        };
    }

    pub fn buttons(&mut self, b: &Battle, side: u8, layer: &Layer) -> Keys {
        let s = &b.custom.sides[side as usize];
        let Some(screen) = s.screen.as_ref().filter(|_| b.round.mode == mode::CUSTOM) else {
            return Keys::empty();
        };
        if self.hold > 0 {
            self.hold -= 1;
            return self.held;
        }
        if self.release > 0 {
            self.release -= 1;
            if self.release == 0
                && let Some((slot, before)) = self.pressed.take()
                && Picked::of(screen, layer.cursor) == before
            {
                self.refused |= 1 << slot;
            }
            return Keys::empty();
        }
        let up = over(b, screen, layer);
        if let Some((at, dir)) = self.moved.take()
            && !matches!(up, Over::Nothing | Over::Busy)
        {
            self.blocked |= move_bit(at, dir);
        }
        match up {
            Over::Nothing => {}
            Over::Busy => return Keys::empty(),
            up => {
                return match self.doing {
                    Doing::Listing(entry) => self.in_list(b, side, layer, entry, up),
                    _ if up == Over::List => self.press(CLOSING_KEY),
                    _ => Keys::empty(),
                };
            }
        }
        if screen.phase != Phase::Choosing {
            return Keys::empty();
        }
        let cursor = layer.cursor;
        let layout = &b.game_rules().custom_screen;
        match self.doing {
            Doing::Idle => Keys::empty(),
            Doing::Going(target) => {
                if self.tries >= MOST_PRESSES {
                    self.refused |= 1 << target;
                    self.doing = Doing::Idle;
                    return Keys::empty();
                }
                self.tries += 1;
                if cursor == target {
                    self.doing = Doing::Idle;
                    self.pressed = Some((target, Picked::of(screen, cursor)));
                    return self.press(Keys::A);
                }
                self.walk(screen, cursor, layout, target).unwrap_or_else(|| {
                    self.refused |= 1 << target;
                    self.doing = Doing::Idle;
                    Keys::empty()
                })
            }
            Doing::Listing(entry) => {
                let l = self.listing;
                if l.chosen {
                    self.doing = Doing::Idle;
                    return Keys::empty();
                }
                let opener = self.opener.filter(|_| !l.opened && !l.leaving && self.tries < MOST_PRESSES);
                let walked = opener.and_then(|o| {
                    if cursor == o.cursor {
                        self.listing.opened = true;
                        Some(self.press(o.key))
                    } else {
                        self.walk(screen, cursor, layout, o.cursor)
                    }
                });
                self.tries += 1;
                walked.unwrap_or_else(|| {
                    self.refused_forms |= 1 << entry;
                    self.doing = Doing::Idle;
                    Keys::empty()
                })
            }
        }
    }

    fn walk(&mut self, screen: &Screen, cursor: u8, layout: &CustomScreenLayout, target: u8) -> Option<Keys> {
        let dir = toward(screen, cursor, layout, target, self.blocked)?;
        self.moved = Some((cursor, dir));
        Some(self.press(MOVES[dir].0))
    }

    fn in_list(&mut self, b: &Battle, side: u8, layer: &Layer, entry: u8, up: Over) -> Keys {
        match up {
            Over::Chosen => {
                self.listing.chosen = true;
                Keys::empty()
            }
            Over::List => {
                let Some(list) = b.form_list(side) else {
                    return self.back_out();
                };
                if list.chosen.is_some() {
                    self.listing.chosen = true;
                    return Keys::empty();
                }
                let l = self.listing;
                if l.leaving || l.pressed || self.tries >= MOST_PRESSES {
                    return self.back_out();
                }
                self.tries += 1;
                let at = layer.list_cursor;
                if at == entry {
                    self.listing.pressed = true;
                    return self.press(Keys::A);
                }
                if l.last == Some(at) {
                    self.listing.dir += 1;
                }
                let Some(&(key, _)) = MOVES.get(self.listing.dir as usize) else {
                    return self.back_out();
                };
                self.listing.last = Some(at);
                self.press(key)
            }
            _ => Keys::empty(),
        }
    }

    fn back_out(&mut self) -> Keys {
        self.listing.leaving = true;
        self.press(CLOSING_KEY)
    }

    fn press(&mut self, key: Keys) -> Keys {
        let direction = key.intersects(Keys::UP | Keys::DOWN | Keys::LEFT | Keys::RIGHT);
        self.held = key;
        self.hold = direction as u8;
        self.release = 1;
        key
    }
}
