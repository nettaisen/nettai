//! A player's own custom screen layer: the cursor and what only shows,
//! outside the simulation (docs/design/custom-screen-actions.md §4).
//!
//! The battle's custom screen (`Screen`) keeps the decisions: the deal, the
//! picks and their order, the rules' buttons and windows, OK. It takes a
//! player's choices (`ScreenChoice`, in their tick's input) and reads no
//! key. Each player's [`Layer`] holds the rest, as the original's screen ran
//! it on its console: the cursor and its blink, where it starts (the first
//! dark chip), the chip window's chip, the dark chips' hover and its shade,
//! R's description, L's run message, SELECT's hiding and the rules' form list
//! (EXE6's Cross window) with its own cursor. Keys drive it by the code that
//! drove the simulated cursor (each game's slot neighbors and EXE4's key
//! orders, the auto-repeat with its console's beat, START to OK, UP into the
//! form list), moved here: a key that decides makes the tick's choice. It
//! never enters the battle's state, its snapshot or its digest.
//!
//! A tick has two halves around the battle's own: [`Layer::input`], before
//! it, reads the player's buttons against the battle as it is presented
//! (the state the last tick left) and gives the tick's input with its
//! choice; [`Layer::follow`], after it, draws what the tick did: the window's
//! counters and sprites by what the screen asked of the drawing (`Draw`)
//! and by the choosing's own drawing as the original's states drew it, the
//! chip window, and the hover. Stepped in lockstep with no delay
//! ([`Lockstep`]), it gives the choices on the ticks the original's screen
//! took its keys: a recording's buttons through it replay the original.
//!
//! What the layer draws and plays is the frontend's: nettai-render draws a
//! side's screen from the battle and that side's layer, and the layer's
//! sounds ([`Layer::cues`]) go among the battle's cues where its screen's end
//! (`Battle::screen_cues_at`).

use super::chatbox::{Chatbox, Script};
use super::folder::BattleFolder;
use super::look::{ChipWindow, Draw, Hover, ScreenCall, ScreenSound, Shown, WindowLook};
use super::screen::{FORM_LIST_ENTRIES, MAX_SELECTIONS, OK_SLOT, Phase, PlayerView, SLOTS, Screen, SlotKind, checked};
use crate::battle::{Battle, TickEvents, mode};
use crate::rules::FormList;
use crate::content::{CustomScreenLayout, FormListRules, HoverRuns, ScreenKey, SlotMoves};
use crate::input::{Joypad, PlayerTick, ScreenChoice, keys};
use crate::sound::SoundCue;
use nettai_content_api::FormHandle;

/// What a player's layer reads of their custom screen as it is presented
/// (`Battle::presented`): the screen and the folder it deals from, what it
/// reads of its player, its number in the round, the ticks left of the
/// round's time limit while it counts, and the rules' form list.
#[derive(Clone, Copy)]
pub struct Presented<'a> {
    pub screen: &'a Screen,
    pub folder: &'a BattleFolder,
    pub view: PlayerView<'a>,
    /// The screen's number in the round (from 1): a new one's is another.
    pub turn: u8,
    pub time_left: Option<u16>,
    pub form_list: Option<FormList>,
}

impl Battle {
    /// Side `side`'s custom screen as its layer reads it, while the custom
    /// screen's screens are open (from their first tick).
    pub fn presented(&self, side: u8) -> Option<Presented<'_>> {
        if self.round.mode != mode::CUSTOM || self.round.init == 0 {
            return None;
        }
        let i = side as usize & 1;
        Some(Presented {
            screen: self.custom.sides[i].screen.as_ref()?,
            folder: &self.custom.sides[i].folder,
            view: self.screen_view(side),
            turn: self.round.turn,
            time_left: self.custom_time_left(side),
            form_list: self.form_list(side),
        })
    }
}

/// SELECT's hiding's steps (`sub_8026D06`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HiddenStage {
    Hiding,
    Waiting,
    Restoring,
}

/// Where the rules' form list's window is (EXE6's Cross window), by its
/// tick from 1: opening (`sub_8027834`), up (`sub_802794A`: its first tick
/// reads no keys), closing (`sub_802790C`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListStage {
    Opening(u16),
    Up(u16),
    Closing(u16),
}

/// What the layer has up over the chips (the original's screen's own
/// states): SELECT's hiding, R's description (of the chip under the cursor,
/// or of a form from the form list, which it goes back to), L's run
/// message, the form list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Sub {
    #[default]
    None,
    Hidden(HiddenStage),
    Description { form: Option<FormHandle>, from_list: bool, chatbox: Chatbox },
    RunMessage { chatbox: Option<Chatbox> },
    List(ListStage),
}

/// What a pointer is on, of a side's custom screen: a place the frontend
/// draws (nettai-render's `custom::targets` gives each one's box).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// A slot the cursor goes to: a chip, OK, a cell of a rules' button.
    Slot(u8),
    /// The `k`th pick (from 0) in the column of picks.
    Pick(u8),
    /// An entry of the rules' form list, while its window is up.
    Form(u8),
    /// The form list's tab (EXE6's CROSSSELECT label), which opens and
    /// closes it.
    FormTab,
    /// The form list's exit label, while its window is up (EXE6's B:EXIT),
    /// which closes it as B does.
    FormExit,
}

/// The targets a gesture's number tells apart ([`Target::index`]).
const TARGETS: u32 = SLOTS as u32 + MAX_SELECTIONS as u32 + FORM_LIST_ENTRIES as u32 + 2;

impl Target {
    /// The target's number among them all, 0 to 23: the slots, the picks,
    /// the form list's entries, its tab and its exit.
    const fn index(self) -> u32 {
        const PICKS: u32 = SLOTS as u32;
        const FORMS: u32 = PICKS + MAX_SELECTIONS as u32;
        const TAB: u32 = FORMS + FORM_LIST_ENTRIES as u32;
        match self {
            Target::Slot(s) => s as u32 % SLOTS as u32,
            Target::Pick(k) => PICKS + k as u32 % MAX_SELECTIONS as u32,
            Target::Form(k) => FORMS + k as u32 % FORM_LIST_ENTRIES as u32,
            Target::FormTab => TAB,
            Target::FormExit => TAB + 1,
        }
    }

    /// The target of a number ([`Target::index`]), if one has it.
    const fn of_index(i: u32) -> Option<Target> {
        const PICKS: u32 = SLOTS as u32;
        const FORMS: u32 = PICKS + MAX_SELECTIONS as u32;
        const TAB: u32 = FORMS + FORM_LIST_ENTRIES as u32;
        Some(match i {
            i if i < PICKS => Target::Slot(i as u8),
            i if i < FORMS => Target::Pick((i - PICKS) as u8),
            i if i < TAB => Target::Form((i - FORMS) as u8),
            i if i == TAB => Target::FormTab,
            i if i == TAB + 1 => Target::FormExit,
            _ => return None,
        })
    }
}

/// A pointer's gesture on the custom screen (the mouse's, a finger's), for
/// the layer's next tick (docs/design/custom-screen-actions.md §5). The
/// tick's input carries the one the layer took (`PlayerTick::gesture`), for
/// a replay to give its layer again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gesture {
    /// The pointer is over a target: the cursor goes there, and no choice
    /// is made (a finger down, the mouse's hover).
    Hover(Target),
    /// A click or a tap on a target (none: on none, which closes a
    /// description or brings the hidden window back as a key does).
    Click(Option<Target>),
    /// Right-click, a long press: R on the target.
    Describe(Target),
}

impl Gesture {
    /// The gesture's number in a tick word's seven bits (none is 0): a
    /// hover's 1 + its target's number ([`Target::index`]), then a click's
    /// on each target, a click on none, a describe's.
    pub const fn code(self) -> u32 {
        match self {
            Gesture::Hover(t) => 1 + t.index(),
            Gesture::Click(Some(t)) => 1 + TARGETS + t.index(),
            Gesture::Click(None) => 1 + 2 * TARGETS,
            Gesture::Describe(t) => 2 + 2 * TARGETS + t.index(),
        }
    }

    /// The gesture a number names; none for 0 and the numbers no gesture
    /// has.
    pub const fn from_code(code: u32) -> Option<Gesture> {
        if code == 0 {
            return None;
        }
        let i = code - 1;
        if i < TARGETS {
            return match Target::of_index(i) {
                Some(t) => Some(Gesture::Hover(t)),
                None => None,
            };
        }
        if i < 2 * TARGETS {
            return match Target::of_index(i - TARGETS) {
                Some(t) => Some(Gesture::Click(Some(t))),
                None => None,
            };
        }
        if i == 2 * TARGETS {
            return Some(Gesture::Click(None));
        }
        match Target::of_index(i - 2 * TARGETS - 1) {
            Some(t) => Some(Gesture::Describe(t)),
            None => None,
        }
    }
}

/// The most gestures a layer keeps for its next ticks.
const GESTURES: usize = 4;

/// The state the original's screen was in, of the battle's and the layer's
/// together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Was {
    Screen(Phase),
    Hidden,
    Description { from_list: bool },
    RunMessage,
    List,
}

impl Was {
    /// The original's `Phase::chooses`: the window in and its own.
    fn chooses(self) -> bool {
        match self {
            Was::Screen(p) => p.chooses(),
            Was::Hidden => false,
            Was::Description { .. } | Was::RunMessage | Was::List => true,
        }
    }

    /// The states the dark chip's hover shades in (`sub_802A394`: choosing
    /// chips, or a chip's description).
    fn hovers(self) -> bool {
        matches!(self, Was::Screen(Phase::Settling | Phase::Choosing) | Was::Description { from_list: false })
    }
}

/// What the pointer's gesture did over the chips.
enum Pointed {
    /// It chose.
    Choice(ScreenChoice),
    /// It took the tick (opened something up, or was refused): no keys.
    Took,
    /// Nothing that takes the tick: the keys have it.
    Nothing,
}

/// What a tick's first half saw, for its second.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TickKind {
    /// No screen presented, or a tick whose drawing is the screen's alone.
    #[default]
    Screen,
    /// The screen's settling tick (EXE4's): the choosing's drawing.
    Settling,
    /// A choosing tick of the original's state 4: its keys (none, its time
    /// up), then its drawing.
    Choosing,
    /// A tick of the layer's own state up (drawn by the first half).
    Sub,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Plan {
    kind: TickKind,
    /// The original's state at the tick's start.
    was: Option<Was>,
    /// The cursor moved (or START took it to OK): the chip window is drawn
    /// again for it.
    chip_window: bool,
    /// The form list went as the time ran out: as a window of the rules
    /// cancelled, the fades and the hover clear.
    list_cancelled: bool,
}

/// A player's own custom screen layer. One a side, a battle (a round) long:
/// a new round's battle gets new layers.
#[derive(Clone, Copy, Debug)]
pub struct Layer {
    side: u8,
    /// The player's joypad (`eJoypad`): held, pressed and the auto-repeat on
    /// their console's beat (`PlayerSetup::joypad_phase`), updated every
    /// tick, the fight's too (a hold begun there repeats on the screen).
    pub joypad: Joypad,
    /// The screen it follows (its number in the round), once one opened.
    turn: Option<u8>,
    /// The slot under the cursor.
    pub cursor: u8,
    /// What it has up over the chips.
    pub sub: Sub,
    /// The form list's cursor: the entry under it (`form_list.cursor`),
    /// from the first at each screen's opening.
    pub list_cursor: u8,
    /// What the chip window shows (`sub_8028476`).
    pub chip_window: ChipWindow,
    /// The window's counters and this tick's sprites.
    pub look: WindowLook,
    /// The dark chips' hover.
    pub hover: Hover,
    /// The first fade record shows the hover's fade (it started last), else
    /// the screen's (`Screen::fade`).
    pub hover_fade_last: bool,
    /// What the tick asked of the sound driver, in order: its states'
    /// sounds, then the hover's.
    calls: [Option<ScreenCall>; 8],
    plan: Plan,
    /// The pointer's gestures for the next ticks, in order (`Layer::point`):
    /// one a tick, a hover replaced by what follows it.
    gestures: [Option<Gesture>; GESTURES],
    /// The gesture this tick takes, until what it is for takes it.
    gesture: Option<Gesture>,
    /// Take-backs a click on a pick asked for, still to send: one a tick
    /// while the screen chooses, down to the pick clicked.
    take_backs: u8,
    /// This tick's choice is the pointer's (its gesture's, a take-back it
    /// asked for).
    pointer_chose: bool,
}

impl Layer {
    /// Side `side`'s layer for the battle `b` (its round), the joypad on its
    /// console's beat.
    pub fn new(side: u8, b: &Battle) -> Layer {
        Layer::on_beat(side, b.setup.players[side as usize & 1].joypad_phase)
    }

    /// Side `side`'s layer, its joypad's first update on the repeat's beat
    /// `joypad_phase` (`PlayerSetup::joypad_phase`).
    pub fn on_beat(side: u8, joypad_phase: u8) -> Layer {
        Layer {
            side: side & 1,
            joypad: Joypad::new(joypad_phase),
            turn: None,
            cursor: 0,
            sub: Sub::None,
            list_cursor: 0,
            chip_window: ChipWindow::default(),
            look: WindowLook::new(),
            hover: Hover::default(),
            hover_fade_last: false,
            calls: [None; 8],
            plan: Plan::default(),
            gestures: [None; GESTURES],
            gesture: None,
            take_backs: 0,
            pointer_chose: false,
        }
    }

    /// A gesture of the pointer, for the next tick ([`Layer::input`]): a
    /// hover the next gesture replaces, the others one a tick, in order (a
    /// full queue drops the newest).
    pub fn point(&mut self, gesture: Gesture) {
        let n = self.gestures.iter().take_while(|g| g.is_some()).count();
        let n = if n > 0 && matches!(self.gestures[n - 1], Some(Gesture::Hover(_))) {
            self.gestures[n - 1] = None;
            n - 1
        } else {
            n
        };
        if n < GESTURES {
            self.gestures[n] = Some(gesture);
        }
    }

    pub fn side(&self) -> u8 {
        self.side
    }

    /// What the tick asked of the sound driver, in order: what the layer
    /// did (the cursor's moves, the sub-screens'), then the dark chip
    /// hover's. Only its player hears them.
    pub fn calls(&self) -> impl Iterator<Item = ScreenCall> + '_ {
        self.calls.iter().flatten().copied()
    }

    /// The tick's sounds as cues for its player (`Battle::screen_cues_at`
    /// says where they go among the battle's).
    pub fn cues(&self, b: &Battle) -> Vec<SoundCue> {
        let mut out = Vec::new();
        for call in self.calls() {
            match call {
                ScreenCall::Sound(sound) => {
                    if let Some(id) = b.screen_sound(sound) {
                        out.push(SoundCue::Effect(id));
                    }
                }
                ScreenCall::Volume { music, screen } => {
                    let [m, s] = b.content.rules().custom_screen.hover.players;
                    out.push(SoundCue::Volume { player: m, volume: music });
                    out.push(SoundCue::Volume { player: s, volume: screen });
                }
            }
        }
        out
    }

    /// The battle's cues of the last tick as this layer's player heard them:
    /// the battle's own for its side, the layer's among them where its
    /// screen's end.
    pub fn heard(&self, b: &Battle) -> Vec<SoundCue> {
        let cues = b.sound_cues_for(self.side);
        let at = b.screen_cues_at(self.side).unwrap_or(cues.len()).min(cues.len());
        let mut out = cues[..at].to_vec();
        out.extend(self.cues(b));
        out.extend_from_slice(&cues[at..]);
        out
    }

    fn play(&mut self, sound: ScreenSound) {
        self.call(ScreenCall::Sound(sound));
    }

    fn call(&mut self, call: ScreenCall) {
        if let Some(slot) = self.calls.iter_mut().find(|s| s.is_none()) {
            *slot = Some(call);
        }
    }

    /// Whether the layer shows its own state over the chips: SELECT's
    /// hiding, a description, the run message or the form list.
    pub fn is_up(&self) -> bool {
        self.sub != Sub::None
    }

    /// The form list's stage, while its window is up (or a form's
    /// description from it: up).
    pub fn list_stage(&self) -> Option<ListStage> {
        match self.sub {
            Sub::List(stage) => Some(stage),
            Sub::Description { from_list: true, .. } => Some(ListStage::Up(0)),
            _ => None,
        }
    }

    /// The window hidden by SELECT, at its step.
    pub fn hidden(&self) -> Option<HiddenStage> {
        match self.sub {
            Sub::Hidden(stage) => Some(stage),
            _ => None,
        }
    }

    /// The chatbox up, of a description or the run message.
    pub fn chatbox(&self) -> Option<&Chatbox> {
        match &self.sub {
            Sub::Description { chatbox, .. } => Some(chatbox),
            Sub::RunMessage { chatbox } => chatbox.as_ref(),
            _ => None,
        }
    }

    /// The form described, of a form's description from the form list.
    pub fn described_form(&self) -> Option<Option<FormHandle>> {
        match self.sub {
            Sub::Description { form, from_list: true, .. } => Some(form),
            _ => None,
        }
    }

    /// The first half of a tick: the player's input of the tick (`input`:
    /// their buttons, as they hold them now; a switch request; a choice of
    /// their own; a pointer's gesture, a recording's) against the battle as
    /// presented, `b`, before it ticks. The tick's input, with the choice its
    /// keys made (a choice given stays), a switch request held back while
    /// the layer has something up over the chips (the original's screen reads
    /// it only while choosing), and the gesture the tick took: the one given,
    /// else the pointer's next ([`Layer::point`]).
    pub fn input(&mut self, input: PlayerTick, b: &Battle) -> PlayerTick {
        self.input_on(input, b.presented(self.side).as_ref())
    }

    /// [`Layer::input`] on the screen as presented (none: no screen up).
    pub fn input_on(&mut self, input: PlayerTick, p: Option<&Presented>) -> PlayerTick {
        self.joypad.update(input.held);
        self.calls = [None; 8];
        self.look.drawn = Shown::default();
        self.plan = Plan::default();
        // (The tick's gesture: one given with the input, a recording's; else
        // the pointer's next. What it is for takes it, or it is dropped with
        // the tick; the tick's input carries it either way.)
        self.gesture = match input.gesture {
            Some(g) => Some(g),
            None => {
                let next = self.gestures[0];
                self.gestures.rotate_left(1);
                self.gestures[GESTURES - 1] = None;
                next
            }
        };
        self.pointer_chose = false;
        let mut out = PlayerTick { choice: None, gesture: self.gesture, ..input };
        let Some(p) = p else { return PlayerTick { gesture: self.gesture, ..input } };
        let screen = p.screen;
        if self.turn != Some(p.turn) {
            // (A screen it hasn't followed from its opening: a layer made
            // since, as a replay is sought. It takes it up as it opened.)
            self.open(p);
        }
        let layout = p.view.library.layout();
        let phase = screen.phase;
        // The round's time limit (nettai's own): the tick the screen counts
        // its last, it takes no choice and closes as if OK were pressed;
        // what is up over the chips goes first, silently.
        let time_up = p.time_left.is_some_and(|left| left <= 1);
        // What the layer has up goes as the screen stops choosing (its time
        // is up; a choice of an earlier tick, given while it showed the
        // choosing, took it busy).
        if self.sub != Sub::None && (phase != Phase::Choosing || time_up) {
            self.plan.list_cancelled = time_up && matches!(self.sub, Sub::List(_));
            self.sub = Sub::None;
        }
        if self.sub != Sub::None {
            out.switch_to = None;
        }
        let switching = phase == Phase::Choosing && !time_up && out.switch_to.is_some_and(|m| switch_taken(&p.view, screen, m));
        self.plan.was = Some(self.was(phase));
        self.plan.kind = match phase {
            Phase::Settling | Phase::Choosing | Phase::Window { .. } if time_up => {
                self.plan.was = Some(Was::Screen(Phase::Choosing));
                TickKind::Choosing
            }
            Phase::Choosing if switching => TickKind::Screen,
            Phase::Choosing => {
                // (The pointer's gesture first: what it chooses or opens
                // takes the tick from the keys.)
                let kind = if self.sub == Sub::None {
                    out.choice = match self.point_on_chips(p, layout) {
                        Pointed::Choice(c) => Some(c),
                        Pointed::Took => None,
                        Pointed::Nothing => self.choose(p, layout),
                    };
                    TickKind::Choosing
                } else {
                    out.choice = self.step_sub(p, layout);
                    TickKind::Sub
                };
                // The take-backs a click on a pick asked for, one a tick
                // while nothing else is chosen and nothing but the form list
                // is up; a choice of the keys' ends them.
                let scheduled = self.take_backs;
                if out.choice.is_some() && !self.pointer_chose {
                    self.take_backs = 0;
                } else if out.choice.is_none() && scheduled > 0 && matches!(self.sub, Sub::None | Sub::List(_)) {
                    out.choice = Some(ScreenChoice::TakeBack);
                    self.take_backs = scheduled - 1;
                }
                kind
            }
            Phase::Settling => TickKind::Settling,
            _ => TickKind::Screen,
        };
        if input.choice.is_some() {
            out.choice = input.choice;
        }
        out
    }

    /// The original's state, of the screen's `phase` and what the layer has
    /// up.
    fn was(&self, phase: Phase) -> Was {
        match (phase, self.sub) {
            (Phase::Choosing, Sub::Hidden(_)) => Was::Hidden,
            (Phase::Choosing, Sub::Description { from_list, .. }) => Was::Description { from_list },
            (Phase::Choosing, Sub::RunMessage { .. }) => Was::RunMessage,
            (Phase::Choosing, Sub::List(_)) => Was::List,
            _ => Was::Screen(phase),
        }
    }

    /// A screen opened (`sub_8026840`): the cursor where it starts
    /// (`sub_802806C`), nothing up, the form list's cursor on its first
    /// entry, the window's counters and the hover as a screen starts them,
    /// and the chip window's last chip kept from the round's screen before
    /// (palette 11).
    fn open(&mut self, p: &Presented) {
        self.turn = Some(p.turn);
        self.cursor = p.screen.first_cursor(p.folder, &p.view, true);
        self.sub = Sub::None;
        self.list_cursor = 0;
        self.take_backs = 0;
        self.look = WindowLook::new();
        self.hover = Hover::default();
        self.hover_fade_last = false;
        let last_chip = self.chip_window.last_chip.filter(|_| p.turn != 1);
        self.chip_window = ChipWindow { last_chip, ..ChipWindow::default() };
    }

    /// State 4's keys (`sub_8028B74`): one a tick, the first down in the
    /// order the slot under the cursor reads them (the neighbors model's:
    /// the directions by their auto-repeat, then A, B, START, SELECT, R and
    /// L pressed; EXE4's: each place's own, 0x08020350, 0x08020620,
    /// 0x08020728), after the rules' form list's UP. A, B, and A in the form
    /// list are the screen's: the choice.
    fn choose(&mut self, p: &Presented, layout: &CustomScreenLayout) -> Option<ScreenChoice> {
        let (screen, view) = (p.screen, &p.view);
        let joy = self.joypad;
        // The rules' form list first (EXE6's UP opening the Cross window,
        // which the original asks before the cursor's UP: `custom.keys`).
        if (joy.repeat | joy.pressed) != 0 && joy.repeat & keys::UP != 0 && self.list_opens(p, layout) {
            self.sub = Sub::List(ListStage::Opening(0));
            return None;
        }
        let order: &[ScreenKey] = match &layout.slots[self.cursor as usize].moves {
            SlotMoves::Neighbors { .. } => &ScreenKey::NEIGHBORS_ORDER,
            SlotMoves::Candidates { keys, .. } => keys,
        };
        let key = order.iter().copied().find(|&k| (if k.is_direction() { joy.repeat } else { joy.pressed }) & key_bits(k) != 0)?;
        match key {
            ScreenKey::Up | ScreenKey::Down | ScreenKey::Left | ScreenKey::Right => {
                if let Some(t) = screen.target_from(self.cursor, key, layout) {
                    if t != self.cursor {
                        self.play(ScreenSound::Cursor);
                    }
                    self.cursor = t;
                    self.plan.chip_window = true;
                }
                None
            }
            // A on the slot under the cursor (`off_8028C9C`).
            ScreenKey::A => match screen.slots[self.cursor as usize].kind {
                SlotKind::Chip { .. } | SlotKind::Offered(_) => Some(ScreenChoice::Pick(self.cursor)),
                SlotKind::Ok => Some(ScreenChoice::Ok),
                SlotKind::Button { .. } => Some(ScreenChoice::Press(self.cursor)),
                SlotKind::Empty | SlotKind::Hidden => None,
            },
            ScreenKey::B => Some(ScreenChoice::TakeBack),
            ScreenKey::Start => {
                if self.cursor != OK_SLOT {
                    self.play(ScreenSound::Cursor);
                }
                self.cursor = OK_SLOT;
                self.plan.chip_window = true;
                None
            }
            ScreenKey::Select => {
                self.sub = Sub::Hidden(HiddenStage::Hiding);
                None
            }
            ScreenKey::R => {
                self.describe_slot(self.cursor, p, layout);
                None
            }
            ScreenKey::L => {
                // (The navi's message: every navi of the originals has one. A
                // navi whose content states none has no box to open; the
                // frontend's audit lists it. Nor does a round whose host took
                // the run key, `HostKeys::run`, where the original's
                // `sub_8028B74` goes on to state 0x1C, `sub_8026E98`: the key
                // is read, as it was the tick's first down, and does
                // nothing.)
                if view.run_key && view.library.run_message(view.stats.navi)[0] != 0 {
                    self.sub = Sub::RunMessage { chatbox: None };
                    if layout.run_message_at_key {
                        // (EXE4's 0x080205C6 runs its script with the key, and
                        // its state waits for it from the next tick.)
                        self.step_run_message(view, layout);
                    }
                }
                None
            }
        }
    }

    /// Whether UP opens the rules' form list (EXE6's `custom.keys`,
    /// `sub_8028B74`): the rules have one, offering an entry, for a navi that
    /// changes form with no other pick holding the turn's form (the list's
    /// own choice may), the cursor on a slot it opens from (EXE6's top row
    /// and OK).
    fn list_opens(&self, p: &Presented, layout: &CustomScreenLayout) -> bool {
        self.list_offered(p, layout) && layout.form_list.as_ref().is_some_and(|rules| rules.opens_from.contains(&self.cursor))
    }

    /// Whether the rules' form list opens, wherever the cursor is (its tab's
    /// click: `list_opens` but the cursor's place).
    fn list_offered(&self, p: &Presented, layout: &CustomScreenLayout) -> bool {
        let Some(list) = p.form_list.filter(|_| layout.form_list.is_some()) else { return false };
        p.screen.megaman && list.count != 0 && (p.screen.form.is_none() || list.chosen.is_some())
    }

    /// R on a slot (the cursor's, or the one a pointer describes): the chip
    /// in it as the screen checks it (an invalid chip shows the invalid
    /// chip's description), or the chip a button shows.
    fn describe_slot(&mut self, slot: u8, p: &Presented, layout: &CustomScreenLayout) {
        let (screen, view) = (p.screen, &p.view);
        let Some(s) = screen.slots.get(slot as usize) else { return };
        let chip = screen.chip_in(slot, p.folder).map(|c| checked(c, view).id).or(s.face);
        if let Some(c) = chip {
            // (A button that shows a chip, EXE5's capsules, 0x0802487C:
            // the slot's kinds 6 and 7, the chip's description.)
            let lines = view.library.chip(c).description_lines;
            self.describe(lines, None, false, layout.chatbox_end_clears_tiles);
            self.play(ScreenSound::Description);
        }
    }

    /// The cursor to slot `slot` for the pointer, if the cursor goes there:
    /// as a key's move, its sound and the chip window. False where it
    /// doesn't go.
    fn point_cursor(&mut self, slot: u8, screen: &Screen) -> bool {
        let Some(s) = screen.slots.get(slot as usize) else { return false };
        if s.kind.is_absent() {
            return false;
        }
        if slot != self.cursor {
            self.play(ScreenSound::Cursor);
            self.cursor = slot;
            self.plan.chip_window = true;
        }
        true
    }

    /// The tick's gesture over the chips (the choosing's, nothing up): a
    /// hover moves the cursor; a click chooses what A on it would, takes
    /// back down to a pick, or opens the form list from its tab; a
    /// describe is R on what it is on.
    fn point_on_chips(&mut self, p: &Presented, layout: &CustomScreenLayout) -> Pointed {
        let Some(gesture) = self.gesture else { return Pointed::Nothing };
        let screen = p.screen;
        match gesture {
            Gesture::Hover(Target::Slot(s)) => {
                self.point_cursor(s, screen);
                Pointed::Nothing
            }
            Gesture::Click(Some(Target::Slot(s))) => {
                if !self.point_cursor(s, screen) {
                    return Pointed::Nothing;
                }
                let choice = match screen.slots[s as usize].kind {
                    SlotKind::Chip { .. } | SlotKind::Offered(_) => ScreenChoice::Pick(s),
                    SlotKind::Ok => ScreenChoice::Ok,
                    SlotKind::Button { .. } => ScreenChoice::Press(s),
                    SlotKind::Empty | SlotKind::Hidden => return Pointed::Nothing,
                };
                self.take_backs = 0;
                self.pointer_chose = true;
                Pointed::Choice(choice)
            }
            // (Down to the pick clicked, the last first: as many take-backs
            // as picks from it on, one a tick.)
            Gesture::Click(Some(Target::Pick(k))) => {
                let picks = screen.selection().len() as u8;
                if k >= picks {
                    return Pointed::Nothing;
                }
                self.take_backs = picks - k - 1;
                self.pointer_chose = true;
                Pointed::Choice(ScreenChoice::TakeBack)
            }
            // (The tab opens the list as UP does, from anywhere.)
            Gesture::Click(Some(Target::FormTab)) => {
                if self.list_offered(p, layout) {
                    self.sub = Sub::List(ListStage::Opening(0));
                    Pointed::Took
                } else {
                    self.play(ScreenSound::Refused);
                    Pointed::Took
                }
            }
            Gesture::Describe(Target::Slot(s)) => {
                if self.point_cursor(s, screen) {
                    self.describe_slot(s, p, layout);
                }
                Pointed::Took
            }
            Gesture::Describe(Target::Pick(k)) => {
                if let Some(&slot) = screen.selection().get(k as usize) {
                    self.describe_slot(slot, p, layout);
                }
                Pointed::Took
            }
            Gesture::Hover(_) | Gesture::Click(_) | Gesture::Describe(_) => Pointed::Nothing,
        }
    }

    /// The tick's gesture in the form list's window, while it reads keys: a
    /// hover moves its cursor and a click on an entry chooses it, until one
    /// is chosen; a click on the chosen one takes it back (and the picks
    /// after it, one a tick); a click on the tab or the exit label closes
    /// the window, as B does; a describe is R on the entry. None: it takes nothing, and the
    /// keys have the tick.
    fn point_in_list(&mut self, p: &Presented, layout: &CustomScreenLayout) -> Option<Option<ScreenChoice>> {
        let gesture = self.gesture?;
        let list = p.form_list.unwrap_or_default();
        let open = list.chosen.is_none();
        match gesture {
            Gesture::Hover(Target::Form(k)) if open && k < list.count => {
                if k != self.list_cursor {
                    self.list_cursor = k;
                    if list.count > 1 {
                        self.play(ScreenSound::Cursor);
                    }
                }
                None
            }
            Gesture::Click(Some(Target::Form(k))) if k < list.count => {
                if open {
                    self.list_cursor = k;
                    self.pointer_chose = true;
                    Some(Some(ScreenChoice::ChooseForm(k)))
                } else if list.chosen == Some(k) {
                    self.take_backs = p.screen.selection().len() as u8;
                    self.pointer_chose = true;
                    Some(Some(ScreenChoice::TakeBack))
                } else {
                    self.play(ScreenSound::Refused);
                    Some(None)
                }
            }
            // (The tab again, or the exit label: as B.)
            Gesture::Click(Some(Target::FormTab | Target::FormExit)) => {
                self.sub = Sub::List(ListStage::Closing(0));
                Some(None)
            }
            Gesture::Describe(Target::Form(k)) if k < list.count => {
                if open {
                    self.list_cursor = k;
                }
                let form = list.forms.get(k as usize).copied().flatten();
                let lines = form.map_or(3, |f| p.view.library.form_description_lines(f));
                self.describe(lines, form, true, layout.chatbox_end_clears_tiles);
                self.play(ScreenSound::Description);
                Some(None)
            }
            _ => None,
        }
    }

    /// A click of this tick's, where what is up takes any key (a
    /// description's or the run message's box, the hidden window): A's bit.
    fn click_as_key(&self) -> u16 {
        if matches!(self.gesture, Some(Gesture::Click(_))) { keys::A } else { 0 }
    }

    /// R: a description's chatbox (`chatbox_runScript` in the key's
    /// handler), which runs its first tick this tick.
    fn describe(&mut self, lines: u8, form: Option<FormHandle>, from_list: bool, end_clears_tiles: bool) {
        let mut chatbox = Chatbox::new(Script::Description { breaks: lines.saturating_sub(1) }).end_clears_tiles(end_clears_tiles);
        chatbox.update(self.joypad.held, self.joypad.pressed);
        self.sub = Sub::Description { form, from_list, chatbox };
    }

    /// A tick of the no-running message's chatbox: its first (`sub_8026EC8`:
    /// the message's sound, the script), or the next.
    fn step_run_message(&mut self, view: &PlayerView, layout: &CustomScreenLayout) {
        let Sub::RunMessage { chatbox } = self.sub else { return };
        let mut chatbox = match chatbox {
            Some(c) => c,
            None => {
                self.play(ScreenSound::RunMessage);
                let navi = view.stats.navi;
                Chatbox::new(Script::RunMessage { lines: view.library.run_message(navi) })
                    .commands_wait_for_text(layout.chatbox_commands_wait_for_text)
                    .end_clears_tiles(layout.chatbox_end_clears_tiles)
                    .character_ends_tick(layout.chatbox_character_ends_tick)
                    .talking(view.library.run_message_talking(navi))
            }
        };
        chatbox.update(self.joypad.held, self.joypad.pressed | self.click_as_key());
        self.sub = Sub::RunMessage { chatbox: Some(chatbox) };
    }

    /// A tick of what the layer has up (the original's states but for the
    /// choosing's own, `sub_8026D06`, `sub_8026E4C`, `sub_8026E98`, the
    /// rules' form list's windows): its keys and its drawing. The form
    /// list's A is the tick's choice.
    fn step_sub(&mut self, p: &Presented, layout: &CustomScreenLayout) -> Option<ScreenChoice> {
        let (screen, view, folder) = (p.screen, &p.view, p.folder);
        let joy = self.joypad;
        let late = screen.look.late_turns;
        let on_chip = matches!(screen.slots[self.cursor as usize].kind, SlotKind::Chip { .. } | SlotKind::Offered(_));
        match self.sub {
            Sub::None => None,
            Sub::Hidden(stage) => {
                // sub_8026D06: hiding takes the last turns' block off;
                // coming back draws the emblem on that tick and the next,
                // where the game's screen does (the layout's
                // `emblem_at_window_return`: EXE5's 0x08023022 draws it on
                // neither).
                let emblem = layout.emblem_at_window_return;
                self.sub = match stage {
                    HiddenStage::Hiding => {
                        self.look.turn_limit = false;
                        self.play(ScreenSound::Hide);
                        Sub::Hidden(HiddenStage::Waiting)
                    }
                    HiddenStage::Waiting if joy.pressed | self.click_as_key() != 0 => {
                        if emblem {
                            self.look.draw_emblem(0);
                        }
                        self.play(ScreenSound::Hide);
                        Sub::Hidden(HiddenStage::Restoring)
                    }
                    HiddenStage::Waiting => self.sub,
                    HiddenStage::Restoring => {
                        if emblem {
                            self.look.draw_emblem(0);
                        }
                        Sub::None
                    }
                };
                None
            }
            Sub::Description { form, from_list, mut chatbox } => {
                // The screen sees the chatbox closed the tick after it
                // closes, and reads keys again the tick after that; the
                // chatbox runs after the screen, each tick. The emblem is
                // drawn every tick (`sub_8026E4C`). (A description that is
                // a state of the choosing, EXE4's, runs the choosing's tick
                // around it: the rule `states_in_choosing`.)
                let in_choosing = !from_list && layout.states_in_choosing;
                if in_choosing {
                    self.count_choosing_frame(late);
                }
                self.look.draw_emblem(0);
                if !chatbox.is_open() {
                    self.play(ScreenSound::DescriptionClose);
                    // (Back to the form list it came from: its first tick
                    // again, which reads no keys.)
                    self.sub = if from_list { Sub::List(ListStage::Up(0)) } else { Sub::None };
                    if in_choosing {
                        self.draw_choosing_return(folder.regular_pending, on_chip);
                    }
                    return None;
                }
                chatbox.update(joy.held, joy.pressed | self.click_as_key());
                self.sub = Sub::Description { form, from_list, chatbox };
                if in_choosing && on_chip {
                    self.look.draw_element();
                }
                None
            }
            Sub::RunMessage { chatbox } => {
                let in_choosing = layout.states_in_choosing;
                if in_choosing {
                    self.count_choosing_frame(late);
                }
                self.look.draw_emblem(0);
                if chatbox.is_some_and(|c| !c.is_open()) {
                    self.sub = Sub::None;
                    if in_choosing {
                        self.draw_choosing_return(folder.regular_pending, on_chip);
                    }
                    return None;
                }
                self.step_run_message(view, layout);
                if in_choosing && on_chip {
                    self.look.draw_element();
                }
                None
            }
            Sub::List(stage) => {
                // The form list's window (a window of the rules' in the
                // original: EXE6's `sub_8027834`, `sub_802794A`, `sub_802790C`),
                // its update a tick, then back to choosing when it is done.
                let Some(rules) = &layout.form_list else {
                    self.sub = Sub::None;
                    return None;
                };
                let in_choosing = layout.states_in_choosing;
                if in_choosing {
                    self.count_choosing_frame(late);
                }
                let regular = folder.regular_pending;
                let (stays, choice) = self.step_list(stage, p, rules, layout, regular, late);
                if !stays {
                    self.sub = Sub::None;
                }
                if in_choosing {
                    self.look.draw_emblem(0);
                    if self.sub == Sub::None {
                        self.draw_choosing_return(regular, on_chip);
                    } else {
                        self.look.draw_regular(regular, true);
                    }
                }
                choice
            }
        }
    }

    /// A tick of the form list's window at `stage` (its tick before this
    /// one): whether it stays up, and the entry chosen.
    fn step_list(
        &mut self,
        stage: ListStage,
        p: &Presented,
        rules: &FormListRules,
        layout: &CustomScreenLayout,
        regular: bool,
        late: bool,
    ) -> (bool, Option<ScreenChoice>) {
        let joy = self.joypad;
        // (`custom.draw_regular`: after the tick's count where the choosing
        // counts first.)
        let counted = layout.frame_counts_first;
        match stage {
            // `sub_8027834`: the window's map changes every 3 ticks (the
            // frontend's); on its last tick the window is up, and its first
            // tick runs at once.
            ListStage::Opening(tick) => {
                let tick = tick + 1;
                if tick == 1 {
                    self.play(ScreenSound::Rules(rules.open_sound));
                }
                if tick >= rules.opening {
                    self.sub = Sub::List(ListStage::Up(1));
                    self.look.frame = 0;
                    self.draw_list(regular, late);
                } else {
                    self.sub = Sub::List(ListStage::Opening(tick));
                    self.look.frame = tick as u32;
                    self.draw_window(regular, late);
                }
                (true, None)
            }
            // `sub_802794A`: its first tick reads no keys; every tick draws,
            // after the keys.
            ListStage::Up(tick) => {
                let tick = tick.saturating_add(1);
                self.sub = Sub::List(ListStage::Up(tick));
                let mut choice = None;
                if tick == 1 {
                    self.look.frame = 0;
                } else {
                    choice = match self.point_in_list(p, layout) {
                        Some(pointed) => pointed,
                        None => self.list_keys(joy, p, layout),
                    };
                }
                self.draw_list(regular, late);
                (true, choice)
            }
            // `sub_802790C`: after 5 ticks the window is the chips' again
            // (`sub_80279FC`, `sub_80279C8`, `sub_8028476`).
            ListStage::Closing(tick) => {
                let tick = tick + 1;
                if tick == 1 {
                    self.play(ScreenSound::Rules(rules.close_sound));
                }
                self.sub = Sub::List(ListStage::Closing(tick));
                self.look.frame = tick as u32;
                if tick >= rules.closing {
                    self.draw_window(regular, late);
                    self.plan.chip_window = true;
                }
                self.look.draw_emblem(0);
                self.look.draw_regular(regular, counted);
                (tick < rules.closing, None)
            }
        }
    }

    /// `sub_8028A78`: the form list's keys. Until a form is chosen, UP and
    /// DOWN move its cursor (wrapping) and A chooses the entry under it (the
    /// choice: a marked one the rules refuse); then B closes the window,
    /// START closes it with the screen's cursor on OK, and R describes the
    /// form under the cursor (its own, of whichever game it is).
    fn list_keys(&mut self, joy: Joypad, p: &Presented, layout: &CustomScreenLayout) -> Option<ScreenChoice> {
        let view = &p.view;
        let list = p.form_list.unwrap_or_default();
        if list.chosen.is_none() {
            let n = list.count;
            let (up, down) = (joy.repeat & keys::UP != 0, joy.repeat & keys::DOWN != 0);
            if up || down {
                self.list_cursor = if up {
                    if self.list_cursor == 0 { n.saturating_sub(1) } else { self.list_cursor - 1 }
                } else if self.list_cursor + 1 >= n {
                    0
                } else {
                    self.list_cursor + 1
                };
                if n > 1 {
                    self.play(ScreenSound::Cursor);
                }
                return None;
            }
            if joy.pressed & keys::A != 0 {
                return Some(ScreenChoice::ChooseForm(self.list_cursor));
            }
        }
        if joy.pressed & keys::B != 0 {
            self.sub = Sub::List(ListStage::Closing(0));
        } else if joy.pressed & keys::START != 0 {
            self.cursor = OK_SLOT;
            self.sub = Sub::List(ListStage::Closing(0));
            self.play(ScreenSound::Cursor);
        } else if joy.pressed & keys::R != 0 {
            let form = list.forms.get(self.list_cursor as usize).copied().flatten();
            // (Three lines for a form the content has no description of.)
            let lines = form.map_or(3, |f| view.library.form_description_lines(f));
            self.describe(lines, form, true, layout.chatbox_end_clears_tiles);
            self.play(ScreenSound::Description);
        }
        None
    }

    /// `sub_80279C8`: the emblem, the Regular chip's frame and the last
    /// turns' block, drawn by the window's frame counter.
    fn draw_window(&mut self, regular: bool, late: bool) {
        self.look.draw_emblem(0);
        self.look.draw_regular(regular, false);
        self.look.draw_turn_limit(late);
    }

    /// `sub_802794A`'s drawing: the window's sprites and the form list's
    /// cursor (the frontend draws the forms' names from the rules' state);
    /// the frame counts on.
    fn draw_list(&mut self, regular: bool, late: bool) {
        self.draw_window(regular, late);
        self.look.draw_form_list_cursor();
        self.look.frame = self.look.frame.wrapping_add(1);
    }

    /// EXE4's choosing tick's start (0x0801E3DA, 0x0801E3DE): the last turns'
    /// block by the frame so far, then the frame counts on.
    fn count_choosing_frame(&mut self, late: bool) {
        self.look.draw_turn_limit(late);
        self.look.frame = self.look.frame.wrapping_add(1);
    }

    /// EXE4's 0x08020A0A: a state of the choosing gives way to the one it
    /// came from, which the choosing's tick then draws (0x0801E3F2,
    /// 0x0801E412): the Regular chip's frame and the cursor, by the frame
    /// counted.
    fn draw_choosing_return(&mut self, regular: bool, on_chip: bool) {
        self.look.draw_regular(regular, true);
        self.draw_cursor(on_chip);
    }

    /// The cursor as the choosing's state draws it: on a chip, with the chip
    /// window's element icon (EXE4's chips' state, 0x08020D8C).
    fn draw_cursor(&mut self, on_chip: bool) {
        self.look.draw_cursor();
        if on_chip {
            self.look.draw_element();
        }
    }

    /// The second half of a tick: the battle as the tick left it, `b`. The
    /// window's drawing (what the screen asked; a choosing tick's own, as
    /// the original's state 4 drew it), the chip window, the hover and its
    /// fades.
    pub fn follow(&mut self, b: &Battle) {
        self.follow_on(b.presented(self.side).as_ref());
    }

    /// [`Layer::follow`] on the screen as the tick left it (none: no screen
    /// up).
    pub fn follow_on(&mut self, p: Option<&Presented>) {
        let Some(p) = p else {
            self.plan = Plan::default();
            return;
        };
        let opened = self.turn != Some(p.turn);
        if opened {
            self.open(p);
            self.plan = Plan::default();
        }
        let (screen, view, folder) = (p.screen, &p.view, p.folder);
        let layout = view.library.layout();
        let late = screen.look.late_turns;
        let plan = self.plan;
        if plan.list_cancelled {
            self.hover.clear_fades();
            let mut calls = Vec::new();
            self.hover.clear(&layout.hover, &mut calls);
            for c in calls {
                self.call(c);
            }
        }
        let counts_first = layout.frame_counts_first;
        if plan.kind == TickKind::Choosing && counts_first {
            self.count_choosing_frame(late);
        }
        // What the screen asked of the drawing, in its order.
        for d in screen.look.drawn.draws() {
            self.apply(d, p, layout);
        }
        if plan.chip_window {
            screen.chip_window_at(self.cursor, &mut self.chip_window, folder, view);
        }
        let on_chip = matches!(screen.slots[self.cursor as usize].kind, SlotKind::Chip { .. } | SlotKind::Offered(_));
        // A form list whose choice the screen took (its window up), or
        // whatever the layer had up as the screen stopped choosing: gone.
        if self.sub != Sub::None && screen.phase != Phase::Choosing {
            self.sub = Sub::None;
        }
        let now = self.was(screen.phase);
        match plan.kind {
            TickKind::Settling => {
                // EXE4's 0x08020340, then the rest of 0x0801E3D8's tick: the
                // last turns' block, the frame count, the emblem's spin, the
                // Regular chip's frame and the cursor its state draws
                // (0x0801E41E).
                self.count_choosing_frame(late);
                self.look.draw_emblem(0);
                self.look.draw_regular(folder.regular_pending, true);
                self.draw_cursor(on_chip);
            }
            TickKind::Choosing => {
                // sub_8026CCC: the cursor, the emblem, the Regular chip's
                // frame (OK took it out of the folder before the frame is
                // drawn: `sub_80293F8`) and the last turns' block are drawn,
                // and the frame counts on. (EXE4's 0x0801E3D8 draws the last
                // turns' block and counts first, then reads the keys and
                // draws the rest: the rule `frame_counts_first`.) (A screen
                // that draws by the state its keys left it in, EXE4's, draws
                // no cursor on the tick a key leaves the choosing, and takes
                // the last turns' block off where the key leaves the window,
                // OK or SELECT: 0x0801E412.)
                let drawn_after = layout.cursor_after_leaving || now == Was::Screen(Phase::Choosing);
                if drawn_after {
                    self.draw_cursor(on_chip);
                } else if matches!(now, Was::Description { from_list: false } | Was::RunMessage) && on_chip {
                    // (R's description and L's message are states of the
                    // choosing, which its tick draws by the state the key
                    // left: the element icon, no cursor, 0x08020E10 and
                    // 0x08020E24.)
                    self.look.draw_element();
                }
                self.look.draw_emblem(0);
                self.look.draw_regular(folder.regular_pending, counts_first);
                // (EXE5's 0x08023012: the chip a button holds, over it.)
                self.look.drawn.held = screen.hold.is_some();
                if drawn_after {
                    if !counts_first {
                        self.look.draw_turn_limit(late);
                    }
                } else if matches!(now, Was::Screen(Phase::Closing { .. }) | Was::Hidden) {
                    self.look.turn_limit = false;
                }
                if !counts_first {
                    self.look.frame = self.look.frame.wrapping_add(1);
                }
            }
            TickKind::Screen | TickKind::Sub => {}
        }
        // sub_802A2B0: the dark chip's hover, after the tick's state: on
        // every tick, or (EXE4's) on the ticks the screen began choosing.
        if !opened {
            let was = plan.was.unwrap_or(now);
            let shading = || now.hovers() && screen.shades_at(self.cursor, view, folder);
            let hover = &layout.hover;
            let on = match hover.runs {
                HoverRuns::EveryTick => Some(Some(shading())),
                HoverRuns::WhileChoosing if was.chooses() => Some(now.chooses().then(shading)),
                HoverRuns::WhileChoosing => None,
            };
            if let Some(on) = on {
                let mut calls = Vec::new();
                if self.hover.hover(on, hover, &mut calls) {
                    self.hover_fade_last = true;
                }
                for c in calls {
                    self.call(c);
                }
            }
            self.hover.step_fades(view.library.fade_clear());
        }
        self.plan = Plan::default();
    }

    /// What the screen asked of the drawing.
    fn apply(&mut self, d: Draw, p: &Presented, layout: &CustomScreenLayout) {
        let (screen, view, folder) = (p.screen, &p.view, p.folder);
        let on_chip = matches!(screen.slots[self.cursor as usize].kind, SlotKind::Chip { .. } | SlotKind::Offered(_));
        match d {
            Draw::ChipWindow => screen.chip_window_at(self.cursor, &mut self.chip_window, folder, view),
            Draw::FadeStarted => self.hover_fade_last = false,
            Draw::FadesCleared => self.hover.clear_fades(),
            Draw::HoverCleared => {
                let mut calls = Vec::new();
                self.hover.clear(&layout.hover, &mut calls);
                for c in calls {
                    self.call(c);
                }
            }
            Draw::LaidOut => self.cursor = screen.first_cursor(folder, view, false),
            d => self.look.apply(d, screen.look.late_turns, on_chip),
        }
    }

    /// The first fade record as the screen shows it: the fade started last,
    /// the screen's or the hover's.
    pub fn fade(&self, screen: &Screen) -> crate::battle::Fade {
        if self.hover_fade_last { self.hover.fade } else { screen.fade }
    }
}

/// Whether a switch to `member` asked on side `side`'s choosing screen is
/// taken (EXE5 DS's 0x0200C82C, as `Screen::switch_asked` says but for the
/// rules' `member_busy`, which the layer can't ask): its keys aren't read on
/// that tick.
fn switch_taken(view: &PlayerView, screen: &Screen, member: u8) -> bool {
    if !matches!(view.battle_mode, 0 | 8) {
        return false;
    }
    let Some(asked) = view.party.member(member) else { return false };
    let own = screen.member.and_then(|m| view.party.member(m)).map_or(view.stats.navi, |s| s.navi);
    asked.navi != own
}

/// A key's joypad bit.
fn key_bits(k: ScreenKey) -> u16 {
    match k {
        ScreenKey::A => keys::A,
        ScreenKey::B => keys::B,
        ScreenKey::Start => keys::START,
        ScreenKey::Select => keys::SELECT,
        ScreenKey::R => keys::R,
        ScreenKey::L => keys::L,
        ScreenKey::Up => keys::UP,
        ScreenKey::Down => keys::DOWN,
        ScreenKey::Left => keys::LEFT,
        ScreenKey::Right => keys::RIGHT,
    }
}

/// Both sides' layers stepped with a battle in lockstep, with no delay:
/// each tick's buttons through their side's layer, its choice into the
/// tick, as the original's screens took their keys (a recording's replay,
/// the tools, a test).
#[derive(Clone, Debug)]
pub struct Lockstep {
    pub layers: [Layer; 2],
}

impl Lockstep {
    pub fn new(b: &Battle) -> Lockstep {
        Lockstep { layers: [Layer::new(0, b), Layer::new(1, b)] }
    }

    /// The tick's input with each side's choice, before `b` ticks.
    pub fn input(&mut self, b: &Battle, input: &[PlayerTick; 2]) -> [PlayerTick; 2] {
        [self.layers[0].input(input[0], b), self.layers[1].input(input[1], b)]
    }

    /// After `b` ticked.
    pub fn follow(&mut self, b: &Battle) {
        for layer in &mut self.layers {
            layer.follow(b);
        }
    }

    /// A tick of `b` with these buttons (and switches) through the layers:
    /// the input it ticked with.
    pub fn tick(&mut self, b: &mut Battle, input: &[PlayerTick; 2], events: TickEvents) -> [PlayerTick; 2] {
        let input = self.input(b, input);
        b.tick(&input, events);
        self.follow(b);
        input
    }

    /// The layers start over, on a new round's battle.
    pub fn reset(&mut self, b: &Battle) {
        *self = Lockstep::new(b);
    }
}
