//! One player's custom screen: the slots, the cursor, the selection and
//! the sub-screens (the Cross window, Beast Out, the chip descriptions),
//! driven by that player's joypad. The original runs this on each
//! console for its own player only (`sub_8026A88` and its states); here
//! both players' screens run in the simulation. See
//! docs/engine/custom-screen.md §2-§4.

use super::folder::{BattleFolder, FOLDER_SIZE, FolderChip, shuffle};
use super::builder::{ClassCounts, FormedAdvance};
use super::chatbox::{Chatbox, Script};
use super::library::Library;
use super::look::{ScreenLook, ScreenSound};
use super::Unlocks;
use crate::console::Console;
use crate::battle::FadeMode;
use crate::content::{ChipClass, ChipCode, CustomScreenLayout, TemplateSlot};
use crate::hud::{Banner, BannerStatus};
use crate::input::{Joypad, keys};
use crate::kinds::player::Emotion;
use crate::content::FormTraits;
use crate::setup::NaviStats;

/// Slots: 0-4 the top row, 5-9 the bottom row, then these two.
pub const SLOTS: usize = 12;
/// The OK button, at the right end of the top row.
pub const OK_SLOT: u8 = 10;
/// The button under OK (Beast Out).
pub const SPECIAL_SLOT: u8 = 11;
/// Chips (and Beast Out) a player can pick per screen.
pub const MAX_SELECTIONS: usize = 5;
/// Crosses a version has.
pub const CROSSES: usize = 5;

/// The code a selection that isn't allowed takes, with the invalid chip
/// (the "error" chip, `Library::invalid_chip`).
pub const INVALID_CODE: ChipCode = ChipCode(0x1B);
/// Codes outside the alphabet that the selection rules treat apart:
/// the invalid chip's, and one no chip has.
const SPECIAL_CODES: [ChipCode; 2] = [ChipCode(0x1B), ChipCode(0x1C)];
/// Whether `c` is the "BeastOut" chip, as a folder chip (not the Beast Out
/// button): `Library::beast_out_chip`.
fn is_beast_out(c: FolderChip, view: &PlayerView) -> bool {
    view.library.beast_out_chip() == Some(c.id)
}

/// What a slot holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotKind {
    /// A dealt chip: entry `index` of the battle folder. `regular`: it is
    /// the folder's Regular chip.
    Chip { index: u8, regular: bool },
    /// A link navi's own chip (`sub_80280A2`), offered once a round.
    NaviChip(FolderChip),
    /// The OK button.
    Ok,
    /// The Beast Out button.
    BeastOut,
    /// DustCross's scrap button, two cells wide (slots 8 and 9).
    Scrap { right_half: bool },
    /// ChpShufl's re-deal button, two cells wide (slots 8 and 9).
    Redeal { right_half: bool },
    /// A chip position with no chip dealt.
    Empty,
    /// Not on this screen.
    Hidden,
}

impl SlotKind {
    /// The cursor never goes to these.
    fn is_absent(self) -> bool {
        matches!(self, SlotKind::Empty | SlotKind::Hidden)
    }
}

/// Whether a slot can be picked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SlotState {
    #[default]
    Selectable,
    /// Greyed out: it doesn't go with the selection, or it can't be used
    /// now.
    Unavailable,
    /// Picked (a chip or Beast Out), or used up (a button).
    Selected,
}

/// A slot and its neighbours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Slot {
    pub kind: SlotKind,
    /// UP and DOWN both move here.
    pub vertical: Option<u8>,
    pub left: Option<u8>,
    pub right: Option<u8>,
    pub state: SlotState,
    /// Buttons: uses left this screen.
    pub uses_left: u8,
}

/// The Crosses a screen offers (`+0x50`), by their place among the
/// player's Crosses (0-4: the version's Cross number, or the place in the
/// setup's Cross list, `Unlocks::cross_at`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CrossWindow {
    pub offered: [u8; CROSSES],
    pub count: u8,
    /// Per offered entry: chosen (only one can be).
    pub marked: [bool; CROSSES],
    /// The entry under the window's cursor.
    pub cursor: u8,
    /// The chosen Cross (its place), if any.
    pub chosen: Option<u8>,
}

/// Where a player's screen is. Tick counts start at 1 on the tick after
/// the one that entered the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// The window slides in (`sub_8026B04`, 10 ticks).
    Opening { tick: u8 },
    /// Picking chips (`sub_8026CCC`).
    Choosing,
    /// SELECT hid the window to look at the field (`sub_8026D06`).
    Hidden { stage: HiddenStage },
    /// R shows the chip's description (`sub_8026E4C`); `from_cross_window`
    /// for a Cross's description (`sub_8026E78`). The screen waits for its
    /// chatbox to close.
    Description { from_cross_window: bool, chatbox: Chatbox },
    /// L: the "no time to run" message (`sub_8026E98`), whose chatbox
    /// starts on the state's first tick (`sub_8026EC8`) and is waited for
    /// from the next (`sub_8026FAA`).
    RunMessage { chatbox: Option<Chatbox> },
    /// The Cross window opens (`sub_8027834`, 12 ticks).
    CrossWindowOpening { tick: u8 },
    /// The Cross window (`sub_802794A`); `entered`: its first tick, which
    /// reads no input, has run.
    CrossWindow { entered: bool },
    /// The Cross window closes (`sub_802790C`, 6 ticks).
    CrossWindowClosing { tick: u8 },
    /// A Cross was chosen (`sub_8027A58`, 34 ticks).
    CrossChosen { tick: u8 },
    /// Beast Out was picked (`sub_802770C`, 70 ticks).
    BeastOutChosen { tick: u8 },
    /// The BeastOut chip was picked from a chip slot (`sub_80275EC`, 85
    /// ticks).
    BeastOutChipChosen { tick: u8 },
    /// DustCross scraps the selected chips (`sub_8027406`).
    /// `done`: the last scrap is over; the next tick returns to choosing.
    Scrapping { tick: u16, done: bool, scrapped: [Option<FolderChip>; MAX_SELECTIONS], count: u8 },
    /// ChpShufl re-deals (`sub_80271F8`, state 0x28): `deal` is the new
    /// order, drawn on the first tick; every 4 ticks the chips are shown
    /// shuffled again, and on the 32nd the deal lands. `started`: the first
    /// tick has run; `elapsed`: ticks since (`+0x40`).
    Redealing { started: bool, elapsed: u8, deal: Deal },
    /// OK was pressed; the window slides out (`sub_8026BF4`, 10 ticks).
    Closing { tick: u8 },
    /// The Program Advance animation (`sub_8026DB0`).
    ProgramAdvance { anim: ProgramAdvanceAnimation },
    /// The result is on its way to the other player (`sub_8026DC4`);
    /// `started`: its first tick, which sends it, has run.
    Sending { started: bool },
}

/// The chips a re-deal shuffles, in the order it walks the folder (the
/// buffer at `word_2036660`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Deal {
    pub chips: [Option<FolderChip>; FOLDER_SIZE],
    pub count: u8,
}

/// The Program Advance animation (`sub_802B734`, its state at
/// `word_2036660`): the screen fades, the Program Advance banner comes up
/// and holds while the picked chips' names and then the Program Advance's
/// are shown, then the banner goes and the screen comes back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ProgramAdvanceAnimation {
    /// +0: 0 before its first call (`sub_802B75C`), 4 running, 8 done.
    pub state: AnimationState,
    /// +1.
    pub step: ProgramAdvanceStep,
    /// +2: the step's entry ran.
    pub started: bool,
    /// +0x10.
    pub timer: u16,
    /// The console's screen fade: frames left (`SetScreenFade`, whose
    /// engine runs after the frame's logic).
    pub fade: u8,
}

/// `word_2036660`+0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnimationState {
    #[default]
    Starting,
    Running,
    Done,
}

/// `sub_802B76C`'s steps (`off_802B784`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProgramAdvanceStep {
    /// `sub_802B7A0`: fade the screen out, then put up the banner.
    #[default]
    FadeOut,
    /// `sub_802B7E0`: once the banner holds, 20 ticks.
    BannerIn,
    /// `sub_802B80C`: a picked chip's name every 8 ticks.
    Names,
    /// `sub_802B8E0`: 24 ticks.
    Pause,
    /// `sub_802B920`: the Program Advance's name; 96 ticks, then the
    /// banner is let go.
    Result,
    /// `sub_802B9B8`: once the banner is gone, fade the screen back in.
    BannerOut,
    /// `sub_802B9D4`: once it is back, done.
    FadeIn,
}

/// The Program Advance banner (`sub_802B7A0`): 0x24, or 0x34 for a recipe
/// of no chips.

/// Frames the animation's screen fades take (`SetScreenFade(0x14, 8)` out
/// to level 0x40, `SetScreenFade(0x10, 8)` back in to 0; the level starts
/// at 0, where every fade before the custom screen left it; a fade in
/// holds its first frame).
const FADE_OUT_FRAMES: u8 = 0x40 / 8;
/// The Program Advance's fade's speed (`SetScreenFade(0x14, 8)`).
const PROGRAM_ADVANCE_FADE_SPEED: u8 = 8;
const FADE_IN_FRAMES: u8 = 1 + 0x40 / 8;

/// The SELECT sub-screen's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HiddenStage {
    Hiding,
    Waiting,
    Restoring,
}

/// What a tick of the screen asks the battle to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Request {
    /// OK: build the hand and the transform request, and close.
    Confirm,
    /// Send the result (the first tick of `Sending`).
    Send,
}

/// A player's custom screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Screen {
    pub phase: Phase,
    pub slots: [Slot; SLOTS],
    pub cursor: u8,
    /// Picked slots, in order.
    pub selection: [u8; MAX_SELECTIONS],
    pub selected: u8,
    /// Chips left in the folder when the screen opened.
    pub chips_left: u8,
    /// Chips dealt at most.
    pub hand_size: u8,
    /// The navi is MegaMan (the Cross window and Beast Out are his).
    pub megaman: bool,
    /// Beast Out is picked (`+0x17`).
    pub beast_out: bool,
    pub crosses: CrossWindow,
    /// A Program Advance formed at OK: its animation runs after the
    /// window slides out.
    pub program_advance: Option<FormedAdvance>,
    /// The console's HUD banner as the screen uses it (the Program
    /// Advance's), stepped after the screen's logic each tick as the HUD
    /// task is.
    pub hud: Banner,
    /// What the screen shows (presentation).
    pub look: ScreenLook,
}

/// What the screen reads of its player when it opens and while it runs.
#[derive(Clone, Copy)]
pub struct PlayerView<'a> {
    pub library: &'a dyn Library,
    pub stats: &'a NaviStats,
    pub emotion: Emotion,
    pub unlocks: &'a Unlocks,
    /// Chips sent this round, by class (`dword_20367E0`).
    pub class_uses: &'a ClassCounts,
    /// The round's Beast Out and Crosses so far.
    pub round: &'a RoundMemory,
    /// The folder's Regular chip hasn't been used yet.
    pub regular_pending: bool,
    /// Battle flag 0x40 (per-player gauges; never set in netbattles).
    pub per_player_gauges: bool,
    /// Battle effects 0x200000 (random battles).
    pub random_battle: bool,
    /// A netbattle's last turns (presentation: the window's block).
    pub late_turns: bool,
}

/// What a player's screens remember through a round (`dword_20349A0`,
/// cleared on the round's first screen).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RoundMemory {
    /// Crosses used this round, by their place (`CrossWindow`).
    pub crosses_used: [bool; CROSSES],
    /// Beast Out was picked this round.
    pub beast_out_used: bool,
    /// Consecutive screens opened in ChargeCross (or its Beast form),
    /// up to 3: each deals one more chip.
    pub charge_cross_screens: u8,
    /// The link navis whose own chips were put in a hand this round (a bit
    /// each: the original's by the chip's place among them).
    pub navi_chips_used: u32,
}

impl Screen {
    /// The screen a player gets when the custom screen opens (`sub_8026840`):
    /// compact the folder, deal, and lay out the slots. `turn`: the
    /// screen's number in the round (1 = first).
    pub fn open(folder: &mut BattleFolder, view: &PlayerView, turn: u8, round: &mut RoundMemory) -> Screen {
        let megaman = view.megaman();
        // sub_802A49C: ChargeCross (a form with `extra_chips`) deals one
        // more chip per screen spent in it, up to three.
        let traits = view.form_traits();
        round.charge_cross_screens = if megaman && traits.has(FormTraits::EXTRA_CHIPS) {
            (round.charge_cross_screens + 1).min(3)
        } else {
            0
        };
        folder.compact();
        let chips_left = folder.count() as u8;
        let mut screen = Screen {
            phase: Phase::Opening { tick: 0 },
            slots: [Slot {
                kind: SlotKind::Hidden,
                vertical: None,
                left: None,
                right: None,
                state: SlotState::Selectable,
                uses_left: 0,
            }; SLOTS],
            cursor: 0,
            selection: [0; MAX_SELECTIONS],
            selected: 0,
            chips_left,
            hand_size: 0,
            megaman,
            beast_out: false,
            crosses: CrossWindow::default(),
            program_advance: None,
            hud: Banner::default(),
            look: ScreenLook::new(view.late_turns, false, None),
        };
        if view.crosses_allowed() && view.emotion != Emotion::WornOut {
            screen.crosses = view.offered_crosses();
        }
        screen.hand_size = hand_size(view, turn, round.charge_cross_screens, false);
        screen.lay_out(view);
        // sub_802806C: a cursor on the first slot goes to the first dark
        // chip dealt (as the class limits count it).
        if screen.cursor == 0 {
            if let Some(s) = (0..OK_SLOT).find(|&s| screen.chip_in(s, folder).is_some_and(|c| is_dark(within_limit(c, view), view))) {
                screen.cursor = s;
            }
        }
        // sub_8026840: the window with the Cross tab while MegaMan has a
        // Cross left this round; sub_8028476: the chip window shows the
        // first slot.
        screen.look.cross_tab =
            screen.megaman && view.crosses_allowed() && view.emotion != Emotion::WornOut && view.crosses_left() != 0;
        screen.show_chip_window(folder, view);
        screen.draw_slots(folder, view);
        screen
    }

    /// `sub_8027E2C`: the slots, the dealt chips and the cursor.
    fn lay_out(&mut self, view: &PlayerView) {
        let layout = view.library.layout();
        for (slot, t) in self.slots.iter_mut().zip(layout.slots.iter()) {
            *slot = Slot {
                kind: match t.kind {
                    TemplateSlot::ChipPosition => SlotKind::Empty,
                    TemplateSlot::Ok => SlotKind::Ok,
                    TemplateSlot::Hidden => SlotKind::Hidden,
                },
                vertical: Some(t.vertical),
                left: Some(t.left),
                right: Some(t.right),
                state: SlotState::Selectable,
                uses_left: 0,
            };
        }
        if view.beast_out_button() {
            let s = &mut self.slots[SPECIAL_SLOT as usize];
            s.kind = SlotKind::BeastOut;
            s.state = if view.beast_out_available() { SlotState::Selectable } else { SlotState::Unavailable };
        }
        let dealt = self.chips_left.min(self.hand_size);
        for i in 0..dealt {
            self.slots[i as usize].kind = SlotKind::Chip { index: i, regular: i == 0 && view.regular_pending };
        }
        if self.megaman && view.form_traits().has(FormTraits::SCRAP_BUTTON) {
            // DustCross (sub_8027F10): the scrap button, usable once.
            self.slots[8] = Slot { kind: SlotKind::Scrap { right_half: false }, right: Some(11), state: SlotState::Unavailable, uses_left: 1, ..self.slots[8] };
            self.slots[9] = Slot { kind: SlotKind::Scrap { right_half: true }, left: Some(7), ..self.slots[9] };
        } else if self.megaman && view.stats.chip_shuffle {
            // ChpShufl (sub_80280E0): the re-deal button.
            self.slots[8] = Slot { kind: SlotKind::Redeal { right_half: false }, right: Some(11), uses_left: 1, ..self.slots[8] };
            self.slots[9] = Slot { kind: SlotKind::Redeal { right_half: true }, left: Some(7), ..self.slots[9] };
        }
        if let Some(chip) = navi_chip(view) {
            // sub_80280A2: a link navi's own chip, once a round.
            self.slots[9].kind = SlotKind::NaviChip(chip);
        }
        self.fix_neighbours(layout);
        self.cursor = (0..SLOTS as u8).find(|&s| !self.slots[s as usize].kind.is_absent()).unwrap_or(OK_SLOT);
    }

    /// `sub_8027F42`: point neighbours that are absent at the next slot
    /// present along the row's scan list.
    fn fix_neighbours(&mut self, layout: &CustomScreenLayout) {
        let absent = |slots: &[Slot; SLOTS], s: u8| slots[s as usize].kind.is_absent();
        for s in (0..SLOTS as u8).rev() {
            let d = self.slots[s as usize];
            if d.kind.is_absent() {
                continue;
            }
            let bottom = s != OK_SLOT && s >= 5;
            let mut fixed = d;
            if d.vertical.is_some_and(|v| absent(&self.slots, v)) {
                fixed.vertical = None;
            }
            if d.left.is_some_and(|l| absent(&self.slots, l)) {
                let from = if bottom && matches!(d.kind, SlotKind::Scrap { right_half: true } | SlotKind::Redeal { right_half: true }) { s - 1 } else { s };
                let list: &[u8] = if bottom { &layout.left_scan_bottom } else { &layout.left_scan_top };
                let n = scan(list, layout.left_scan_start[from as usize], |x| absent(&self.slots, x));
                fixed.left = (n != from).then_some(n);
            }
            if d.right.is_some_and(|r| absent(&self.slots, r)) {
                let from = if bottom && matches!(d.kind, SlotKind::Scrap { right_half: false } | SlotKind::Redeal { right_half: false }) { s + 1 } else { s };
                let list: &[u8] = if bottom { &layout.right_scan_bottom } else { &layout.right_scan_top };
                let n = scan(list, layout.right_scan_start[from as usize], |x| absent(&self.slots, x));
                fixed.right = (n != from).then_some(n);
            }
            self.slots[s as usize] = fixed;
        }
    }

    /// The chips dealt, by slot.
    pub fn dealt<'a>(&'a self, folder: &'a BattleFolder) -> impl Iterator<Item = (u8, FolderChip)> + 'a {
        (0..SLOTS as u8).filter_map(move |s| self.chip_in(s, folder).map(|c| (s, c)))
    }

    /// The chip a slot shows, if it is a chip slot.
    pub fn chip_in(&self, slot: u8, folder: &BattleFolder) -> Option<FolderChip> {
        match self.slots[slot as usize].kind {
            SlotKind::Chip { index, .. } => folder.chips[index as usize],
            SlotKind::NaviChip(c) => Some(c),
            _ => None,
        }
    }

    /// The picked slots, in order.
    pub fn selection(&self) -> &[u8] {
        &self.selection[..self.selected as usize]
    }

    /// Whether the window is on screen (for the HUD).
    pub fn window_shown(&self) -> bool {
        !matches!(self.phase, Phase::Hidden { .. } | Phase::Closing { .. } | Phase::ProgramAdvance { .. } | Phase::Sending { .. })
    }

    /// One tick of the screen with this joypad, on the player's console
    /// (its RNG, for ChpShufl's re-deal, and its camera), then the
    /// console's HUD banner.
    pub fn tick(&mut self, joy: &Joypad, view: &PlayerView, folder: &mut BattleFolder, console: &mut Console) -> Option<Request> {
        self.look.drawn = Default::default();
        let request = self.step(joy, view, folder, console);
        let on_dark = self.on_dark_chip(view, folder);
        self.look.hover(on_dark);
        self.hud.tick();
        if let Phase::ProgramAdvance { anim } = &mut self.phase {
            anim.fade = anim.fade.saturating_sub(1);
        }
        self.look.fade.step();
        self.look.window_fade.step();
        request
    }

    /// `sub_80279C8`: the emblem, the Regular chip's frame and the last
    /// turns' block, drawn by the window's frame counter.
    fn draw_window(&mut self, folder: &BattleFolder) {
        self.look.draw_emblem(0);
        self.look.draw_regular(folder.regular_pending);
        self.look.draw_turn_limit();
    }

    /// `sub_802794A`'s drawing: the window's sprites, the Crosses' names
    /// (the frontend's, from `crosses`) and the Cross window's cursor; the
    /// frame counts on.
    fn draw_cross_window(&mut self, folder: &BattleFolder) {
        self.draw_window(folder);
        self.look.draw_cross_cursor();
        self.look.frame += 1;
    }

    /// `sub_802A394`: choosing chips or reading a chip's description, the
    /// cursor rests on a dark chip (as it counts in a selection).
    fn on_dark_chip(&self, view: &PlayerView, folder: &BattleFolder) -> bool {
        matches!(self.phase, Phase::Choosing | Phase::Description { from_cross_window: false, .. })
            && self.chip_in(self.cursor, folder).is_some_and(|c| is_dark(checked(c, view), view))
    }

    fn step(&mut self, joy: &Joypad, view: &PlayerView, folder: &mut BattleFolder, console: &mut Console) -> Option<Request> {
        match self.phase {
            Phase::Opening { tick } => {
                // sub_8026B04: the window moves in 12 pixels a tick.
                let tick = tick + 1;
                if tick == 1 {
                    self.look.play(ScreenSound::Open);
                }
                self.phase = if tick >= 10 { Phase::Choosing } else { Phase::Opening { tick } };
                self.look.frame = SLIDE - SLIDE_STEP * tick as u32;
                self.look.draw_emblem(self.look.frame);
                None
            }
            Phase::Choosing => {
                // sub_8026CCC: the keys, then the cursor, the emblem, the
                // Regular chip's frame and the last turns' block are drawn,
                // and the frame counts on.
                let request = self.choose(joy, view, folder);
                // (OK takes the Regular chip out of the folder before the
                // frame is drawn: `sub_80293F8`.)
                let regular_taken = request == Some(Request::Confirm)
                    && self.selection().iter().any(|&s| matches!(self.slots[s as usize].kind, SlotKind::Chip { regular: true, .. }));
                self.look.draw_cursor();
                self.look.draw_emblem(0);
                self.look.draw_regular(folder.regular_pending && !regular_taken);
                self.look.draw_turn_limit();
                self.look.frame += 1;
                request
            }
            Phase::Hidden { stage } => {
                // sub_8026D06: hiding takes the last turns' block off;
                // coming back draws the emblem on that tick and the next.
                self.phase = match stage {
                    HiddenStage::Hiding => {
                        self.look.turn_limit = false;
                        self.look.play(ScreenSound::Hide);
                        Phase::Hidden { stage: HiddenStage::Waiting }
                    }
                    HiddenStage::Waiting if joy.pressed != 0 => {
                        self.look.draw_emblem(0);
                        self.look.play(ScreenSound::Hide);
                        Phase::Hidden { stage: HiddenStage::Restoring }
                    }
                    HiddenStage::Waiting => self.phase,
                    HiddenStage::Restoring => {
                        self.look.draw_emblem(0);
                        Phase::Choosing
                    }
                };
                None
            }
            Phase::Description { from_cross_window, mut chatbox } => {
                // The screen sees the chatbox closed the tick after it
                // closes, and reads keys again the tick after that; the
                // chatbox runs after the screen, each tick. The emblem is
                // drawn every tick (`sub_8026E4C`).
                self.look.draw_emblem(0);
                if !chatbox.is_open() {
                    self.look.play(ScreenSound::DescriptionClose);
                    self.phase = if from_cross_window { Phase::CrossWindow { entered: false } } else { Phase::Choosing };
                    return None;
                }
                chatbox.update(joy.held, joy.pressed);
                self.phase = Phase::Description { from_cross_window, chatbox };
                None
            }
            Phase::RunMessage { chatbox } => {
                self.look.draw_emblem(0);
                let mut chatbox = match chatbox {
                    None => {
                        // sub_8026EC8
                        self.look.play(ScreenSound::RunMessage);
                        Chatbox::new(Script::RunMessage { lines: view.library.run_message(view.stats.navi) })
                    }
                    Some(c) if !c.is_open() => {
                        self.phase = Phase::Choosing;
                        return None;
                    }
                    Some(c) => c,
                };
                chatbox.update(joy.held, joy.pressed);
                self.phase = Phase::RunMessage { chatbox: Some(chatbox) };
                None
            }
            Phase::CrossWindowOpening { tick } => {
                // sub_8027834: the window's map changes every 3 ticks (the
                // frontend's); on the 12th the Cross window is up, and its
                // state runs at once.
                let tick = tick + 1;
                if tick == 1 {
                    self.look.play(ScreenSound::CrossWindowOpen);
                }
                if tick >= 12 {
                    self.phase = Phase::CrossWindow { entered: true };
                    self.look.frame = 0;
                    self.draw_cross_window(folder);
                } else {
                    self.phase = Phase::CrossWindowOpening { tick };
                    self.look.frame = tick as u32;
                    self.draw_window(folder);
                }
                None
            }
            Phase::CrossWindow { entered } => {
                // sub_802794A: its first tick reads no keys; every tick
                // draws, after the keys.
                if entered {
                    self.cross_window(joy, view);
                } else {
                    self.phase = Phase::CrossWindow { entered: true };
                    self.look.frame = 0;
                }
                self.draw_cross_window(folder);
                None
            }
            Phase::CrossWindowClosing { tick } => {
                // sub_802790C: after 5 ticks the window is the chips' again
                // (`sub_80279FC`, `sub_80279C8`, `sub_8028476`).
                let tick = tick + 1;
                if tick == 1 {
                    self.look.play(ScreenSound::CrossWindowClose);
                }
                self.look.frame = tick as u32;
                if tick >= 6 {
                    self.phase = Phase::Choosing;
                    self.draw_window(folder);
                    self.show_chip_window(folder, view);
                } else {
                    self.phase = Phase::CrossWindowClosing { tick };
                }
                self.look.draw_emblem(0);
                self.look.draw_regular(folder.regular_pending);
                None
            }
            Phase::CrossChosen { tick } => {
                // sub_8027A58: 16 ticks, then the screen fades to white and
                // back; when it is white the Cross is put on (`sub_8027AAE`:
                // the face, the window, the sound), and when it is clear
                // again the chips are chosen (`sub_8027ADE`).
                let tick = tick + 1;
                match tick {
                    1 => self.look.frame = 0,
                    2..=16 => self.look.frame += 1,
                    17 => {
                        self.look.frame = 0;
                        self.look.fade.start(FadeMode::EndToWhite, CROSS_FADE_SPEED);
                    }
                    CROSS_PUT_ON_TICK => {
                        self.look.fade.start(FadeMode::IntroFromWhite, CROSS_FADE_SPEED);
                        if let Some(cross) = self.crosses.chosen {
                            self.look.face = cross_face(view, cross);
                        }
                        self.draw_window(folder);
                        self.show_chip_window(folder, view);
                        self.look.play(ScreenSound::CrossChosen);
                    }
                    34 => {
                        self.draw_window(folder);
                        self.show_chip_window(folder, view);
                    }
                    _ => {}
                }
                self.look.draw_emblem(0);
                self.look.draw_regular(folder.regular_pending);
                self.phase = if tick >= 34 { Phase::Choosing } else { Phase::CrossChosen { tick } };
                None
            }
            Phase::BeastOutChosen { tick } => {
                let tick = tick + 1;
                match tick {
                    // sub_8027738: the emblem spins.
                    1 => {
                        self.beast_out = true;
                        self.look.frame = 0;
                        self.look.spin = 1;
                    }
                    // sub_802774C: the screen fades (0x64) and this
                    // console's camera shakes, 40 ticks at magnitude 1.
                    2 => {
                        console.shake_secondary(BEAST_OUT_SHAKE.0, BEAST_OUT_SHAKE.1);
                        self.look.frame = 0;
                        self.look.fade.start(FadeMode::BeastOut, BEAST_OUT_FADE_SPEED);
                        self.look.play(ScreenSound::BeastOut(view.beast_game()));
                        self.look.play(ScreenSound::Pick);
                        self.look.play(ScreenSound::BeastOutFlash);
                    }
                    // sub_802777C
                    3..=52 => self.look.frame += 1,
                    // sub_8027796: the screen fades back in, and the
                    // emotion window shows the Beast form (sub_802A040).
                    53 => {
                        self.look.fade.start(FadeMode::BeastOutBack, BEAST_OUT_FADE_SPEED);
                        self.look.face = beast_face(view, view.emotion == Emotion::Tired);
                        // Beast Out goes first in the selection, so B takes
                        // it back last.
                        let n = self.selected as usize;
                        self.selection[..n].rotate_right(1);
                        self.reorder_column(folder, beast_out_icon(view));
                        self.slots[SPECIAL_SLOT as usize].state = SlotState::Selected;
                        self.update_availability(view, folder);
                    }
                    _ => {}
                }
                self.phase = if tick >= 70 { Phase::Choosing } else { Phase::BeastOutChosen { tick } };
                self.look.draw_emblem(0);
                None
            }
            Phase::BeastOutChipChosen { tick } => {
                // Like Beast Out's (`sub_802770C`), 16 ticks later: its fade
                // out (0x64) starts at tick 17, with sounds 0x193 and 0xBC;
                // 50 ticks on (tick 68) the chip moves to the front of the
                // selection (the Beast Out button's state and the Beast Out
                // flag stay as they are: the hand's chip is the Beast Out),
                // and the fade back in (0x60) ends it.
                let tick = tick + 1;
                match tick {
                    // sub_8027618
                    1 => self.look.frame = 0,
                    // sub_8027624
                    2..=16 | 18..=67 => self.look.frame += 1,
                    // sub_8027624's last tick: this console's camera
                    // shakes as for Beast Out, and the screen fades.
                    17 => {
                        console.shake_secondary(BEAST_OUT_SHAKE.0, BEAST_OUT_SHAKE.1);
                        self.look.frame = 0;
                        self.look.fade.start(FadeMode::BeastOut, BEAST_OUT_FADE_SPEED);
                        self.look.play(ScreenSound::BeastOut(view.beast_game()));
                        self.look.play(ScreenSound::BeastOutFlash);
                    }
                    // sub_8027672
                    68 => {
                        let n = self.selected as usize;
                        self.selection[..n].rotate_right(1);
                        let first = self.chip_in(self.selection[0], folder);
                        self.reorder_column(folder, first);
                        self.look.face = beast_face(view, false);
                        self.update_availability(view, folder);
                        self.look.fade.start(FadeMode::BeastOutBack, BEAST_OUT_FADE_SPEED);
                    }
                    _ => {}
                }
                self.phase = if tick >= 85 { Phase::Choosing } else { Phase::BeastOutChipChosen { tick } };
                self.look.draw_emblem(0);
                None
            }
            Phase::Scrapping { .. } => {
                // sub_8027406: every tick also draws the emblem and the
                // Regular chip's frame.
                self.scrap(view, folder);
                self.look.draw_emblem(0);
                self.look.draw_regular(folder.regular_pending);
                None
            }
            Phase::Redealing { .. } => {
                // sub_80271F8: every tick also draws the emblem and the
                // Regular chip's frame.
                self.redeal(view, folder, console);
                self.look.draw_emblem(0);
                self.look.draw_regular(folder.regular_pending);
                None
            }
            Phase::Closing { tick } => {
                // sub_8026BF4: the window moves out 12 pixels a tick; its
                // first tick takes the last turns' block off.
                let tick = tick + 1;
                if tick == 1 {
                    self.look.turn_limit = false;
                }
                self.look.frame = SLIDE_STEP * tick as u32;
                self.look.draw_emblem(self.look.frame);
                self.phase = match tick {
                    10 if self.program_advance.is_some() => Phase::ProgramAdvance { anim: Default::default() },
                    10 => Phase::Sending { started: false },
                    _ => Phase::Closing { tick },
                };
                None
            }
            Phase::ProgramAdvance { mut anim } => {
                let pa = self.program_advance.expect("a Program Advance formed");
                // sub_802B734: the animation's own counter, every tick.
                self.look.pa_ticks = self.look.pa_ticks.wrapping_add(1);
                self.phase = match anim.state {
                    // sub_802B75C
                    AnimationState::Starting => {
                        anim.state = AnimationState::Running;
                        Phase::ProgramAdvance { anim }
                    }
                    AnimationState::Running => {
                        self.animate_program_advance(&mut anim, pa, view);
                        Phase::ProgramAdvance { anim }
                    }
                    // sub_802B766: done; on to sending (state 0x14).
                    AnimationState::Done => Phase::Sending { started: false },
                };
                None
            }
            Phase::Sending { started: false } => {
                self.phase = Phase::Sending { started: true };
                Some(Request::Send)
            }
            Phase::Sending { started: true } => None,
        }
    }

    /// `sub_802B76C`: one step of the Program Advance animation.
    fn animate_program_advance(&mut self, anim: &mut ProgramAdvanceAnimation, pa: FormedAdvance, view: &PlayerView) {
        use ProgramAdvanceStep as S;
        let next = |anim: &mut ProgramAdvanceAnimation, step| {
            anim.step = step;
            anim.started = false;
            anim.timer = 0;
        };
        if matches!(anim.step, S::Names | S::Pause | S::Result) {
            self.look.blink_program_advance();
        }
        match anim.step {
            S::FadeOut => {
                if !anim.started {
                    anim.started = true;
                    anim.fade = FADE_OUT_FRAMES;
                    // sub_802B9FE(0); the screen fades a quarter of the way.
                    self.look.pa_palette = 0;
                    self.look.fade.start(FadeMode::ProgramAdvance, PROGRAM_ADVANCE_FADE_SPEED);
                    return;
                }
                if anim.fade != 0 {
                    return;
                }
                let id = view.library.program_advance_banner(pa.len != 0);
                self.hud.start(id, view.library.banner_holds(id));
                next(anim, S::BannerIn);
            }
            S::BannerIn => {
                if !anim.started {
                    if self.hud.status() != BannerStatus::Holding {
                        return;
                    }
                    anim.started = true;
                }
                anim.timer += 1;
                if anim.timer >= 0x14 {
                    next(anim, S::Names);
                    self.look.pa_ticks = 0;
                }
            }
            S::Names => {
                // A name every 8 ticks (drawn only), the recipe's with a
                // sound.
                let old = anim.timer;
                anim.timer = old + 1;
                if old & 7 != 0 {
                    return;
                }
                let k = old >> 3;
                if (pa.start as u16..(pa.start + pa.len) as u16).contains(&k) {
                    self.look.play(ScreenSound::ProgramAdvancePart);
                }
                if (anim.timer >> 3) + 1 >= pa.picks as u16 {
                    next(anim, S::Pause);
                }
            }
            S::Pause => {
                anim.timer += 1;
                if anim.timer >= 0x18 {
                    next(anim, S::Result);
                }
            }
            S::Result => {
                // The Program Advance's name shows at 16 ticks.
                anim.timer += 1;
                if anim.timer == 0x10 {
                    self.look.play(ScreenSound::ProgramAdvance);
                }
                if anim.timer >= 0x60 {
                    next(anim, S::BannerOut);
                    // sub_801E780
                    self.hud.release();
                }
            }
            S::BannerOut => {
                if self.hud.status() == BannerStatus::Done {
                    next(anim, S::FadeIn);
                    anim.fade = FADE_IN_FRAMES;
                    self.look.fade.start(FadeMode::ProgramAdvanceBack, PROGRAM_ADVANCE_FADE_SPEED);
                }
            }
            S::FadeIn => {
                if anim.fade == 0 {
                    anim.state = AnimationState::Done;
                }
            }
        }
    }

    /// State 4 (`sub_8028B74`): one key per tick. Directions (auto-repeat)
    /// come first, then A, B, START, SELECT, R and L (pressed).
    fn choose(&mut self, joy: &Joypad, view: &PlayerView, folder: &mut BattleFolder) -> Option<Request> {
        let here = self.slots[self.cursor as usize];
        let rep = joy.repeat;
        let target = if rep & (keys::UP | keys::DOWN) != 0 {
            if rep & keys::UP != 0
                && self.megaman
                && !self.beast_out
                && self.crosses.count != 0
                && (self.cursor == OK_SLOT || self.cursor <= 4)
            {
                self.phase = Phase::CrossWindowOpening { tick: 0 };
                return None;
            }
            Some(here.vertical)
        } else if rep & keys::LEFT != 0 {
            Some(here.left)
        } else if rep & keys::RIGHT != 0 {
            Some(here.right)
        } else {
            None
        };
        if let Some(target) = target {
            if let Some(t) = target {
                if t != self.cursor {
                    self.look.play(ScreenSound::Cursor);
                }
                self.cursor = t;
                self.show_chip_window(folder, view);
            }
            return None;
        }
        let p = joy.pressed;
        if p & keys::A != 0 {
            return self.press_a(view, folder);
        } else if p & keys::B != 0 {
            self.deselect(view, folder);
        } else if p & keys::START != 0 {
            if self.cursor != OK_SLOT {
                self.look.play(ScreenSound::Cursor);
            }
            self.cursor = OK_SLOT;
            self.show_chip_window(folder, view);
        } else if p & keys::SELECT != 0 {
            self.phase = Phase::Hidden { stage: HiddenStage::Hiding };
        } else if p & keys::R != 0 {
            // The chip as the screen checks it: an invalid chip shows the
            // invalid chip's description.
            if let Some(c) = self.chip_in(self.cursor, folder) {
                let lines = view.library.chip(checked(c, view).id).description_lines();
                self.describe(joy, lines, false);
                self.look.play(ScreenSound::Description);
            }
        } else if p & keys::L != 0 {
            self.phase = Phase::RunMessage { chatbox: None };
        }
        None
    }

    /// A on the slot under the cursor (`off_8028C9C`).
    fn press_a(&mut self, view: &PlayerView, folder: &mut BattleFolder) -> Option<Request> {
        let cursor = self.cursor;
        let here = self.slots[cursor as usize];
        match here.kind {
            SlotKind::Chip { .. } | SlotKind::NaviChip(_) => {
                // sub_8028CCC
                if here.state != SlotState::Selectable || self.selected as usize >= MAX_SELECTIONS {
                    self.look.play(ScreenSound::Refused);
                    return None;
                }
                self.push_selection(cursor);
                self.slots[cursor as usize].state = SlotState::Selected;
                self.look.play(ScreenSound::Pick);
                self.update_availability(view, folder);
                // The pick's icon in the column, and the emblem spins.
                self.look.column[self.selected as usize - 1] = self.chip_in(cursor, folder).map(|c| checked(c, view));
                self.look.spin = 1;
                // sub_802A00C
                if self.chip_in(cursor, folder).is_some_and(|c| is_beast_out(c, view)) {
                    self.phase = Phase::BeastOutChipChosen { tick: 0 };
                }
            }
            SlotKind::Ok => {
                // sub_8028D3A: the battle builds the hand and the
                // transform request, then the window slides out.
                self.phase = Phase::Closing { tick: 0 };
                self.look.play(ScreenSound::Ok);
                return Some(Request::Confirm);
            }
            SlotKind::BeastOut => {
                // sub_8028D6C
                if here.state != SlotState::Selectable || self.selected as usize >= MAX_SELECTIONS {
                    self.look.play(ScreenSound::Refused);
                    return None;
                }
                self.push_selection(cursor);
                self.look.play(ScreenSound::Pick);
                self.phase = Phase::BeastOutChosen { tick: 0 };
                // (sub_802A034: the column shows the BeastOut chip.)
                self.look.column[self.selected as usize - 1] = beast_out_icon(view);
            }
            SlotKind::Scrap { right_half } => {
                // sub_8028E04
                let button = if right_half { 8 } else { cursor };
                if self.slots[button as usize].state == SlotState::Selectable {
                    self.look.play(ScreenSound::Pick);
                    self.phase = Phase::Scrapping { tick: 0, done: false, scrapped: [None; MAX_SELECTIONS], count: 0 };
                } else {
                    self.look.play(ScreenSound::Refused);
                }
            }
            SlotKind::Redeal { right_half } => {
                // sub_8028DD6
                let button = if right_half { 8 } else { cursor };
                if self.slots[button as usize].state == SlotState::Selectable {
                    let deal = Deal { chips: [None; FOLDER_SIZE], count: 0 };
                    self.phase = Phase::Redealing { started: false, elapsed: 0, deal };
                    self.look.play(ScreenSound::Redeal);
                } else {
                    self.look.play(ScreenSound::Refused);
                }
            }
            SlotKind::Empty | SlotKind::Hidden => {}
        }
        None
    }

    /// `sub_8027796`, `sub_8027672`: the column's icons in the picks' new
    /// order, `first` first (the picks' chips as dealt, unchecked).
    fn reorder_column(&mut self, folder: &BattleFolder, first: Option<FolderChip>) {
        for j in 1..self.selected as usize {
            self.look.column[j] = self.chip_in(self.selection[j], folder);
        }
        self.look.column[0] = first;
    }

    /// `sub_8028250`: the slots' tiles show the chips dealt now.
    fn draw_slots(&mut self, folder: &BattleFolder, view: &PlayerView) {
        for s in 0..SLOTS as u8 {
            self.look.slot_chips[s as usize] = self.chip_in(s, folder).map(|c| checked(c, view));
            self.look.slot_picked[s as usize] = self.slots[s as usize].state == SlotState::Selected;
        }
    }

    /// `sub_8028476`: the chip window shows the slot under the cursor.
    fn show_chip_window(&mut self, folder: &BattleFolder, view: &PlayerView) {
        let chip = self.chip_in(self.cursor, folder).map(|c| checked(c, view));
        let w = &mut self.look.chip_window;
        w.slot = self.cursor;
        w.picks = self.selected;
        if chip.is_some() {
            w.last_chip = chip;
        }
    }

    fn push_selection(&mut self, slot: u8) {
        self.selection[self.selected as usize] = slot;
        self.selected += 1;
    }

    /// B (`sub_8029032`): take back the last pick; with none, the Cross.
    fn deselect(&mut self, view: &PlayerView, folder: &BattleFolder) {
        if self.selected == 0 {
            let Some(_) = self.crosses.chosen else {
                self.look.play(ScreenSound::Refused);
                return;
            };
            self.crosses.marked[self.crosses.cursor as usize] = false;
            self.crosses.chosen = None;
            self.look.face = None;
            self.look.play(ScreenSound::Cancel);
        } else {
            let last = self.selection[self.selected as usize - 1];
            self.selected -= 1;
            if self.slots[last as usize].kind == SlotKind::BeastOut {
                self.beast_out = false;
            }
            self.slots[last as usize].state = SlotState::Selectable;
            self.look.column[self.selected as usize] = None;
            // sub_802A0EC: taking Beast Out (or the BeastOut chip) back
            // takes its face back.
            let beast_chip = self.chip_in(last, folder).is_some_and(|c| is_beast_out(c, view));
            if self.slots[last as usize].kind == SlotKind::BeastOut || beast_chip {
                self.look.face = None;
                self.look.play(ScreenSound::Back);
                self.look.play(ScreenSound::Cancel);
            }
        }
        self.update_availability(view, folder);
        self.show_chip_window(folder, view);
        self.look.play(ScreenSound::Back);
    }

    /// The Cross window's keys (`sub_8028A78`).
    fn cross_window(&mut self, joy: &Joypad, view: &PlayerView) {
        let w = &mut self.crosses;
        if w.chosen.is_none() {
            let n = w.count;
            if joy.repeat & (keys::UP | keys::DOWN) != 0 {
                w.cursor = if joy.repeat & keys::UP != 0 {
                    if w.cursor == 0 { n - 1 } else { w.cursor - 1 }
                } else if w.cursor + 1 >= n {
                    0
                } else {
                    w.cursor + 1
                };
                if n > 1 {
                    self.look.play(ScreenSound::Cursor);
                }
                return;
            }
            if joy.pressed & keys::A != 0 {
                let i = w.cursor as usize;
                if w.marked[i] {
                    self.look.play(ScreenSound::Refused);
                    return;
                }
                self.look.play(ScreenSound::Pick);
                let w = &mut self.crosses;
                w.marked[i] = true;
                w.chosen = Some(w.offered[i]);
                self.phase = Phase::CrossChosen { tick: 0 };
                // A chosen Cross greys out Beast Out.
                self.update_beast_out(view);
                return;
            }
        }
        let p = joy.pressed;
        if p & keys::B != 0 {
            self.phase = Phase::CrossWindowClosing { tick: 0 };
        } else if p & keys::START != 0 {
            self.cursor = OK_SLOT;
            self.phase = Phase::CrossWindowClosing { tick: 0 };
            self.look.play(ScreenSound::Cursor);
        } else if p & keys::R != 0 {
            // Every Cross's description has three lines.
            self.describe(joy, 3, true);
            self.look.play(ScreenSound::Description);
        }
    }

    /// R: a description's chatbox (`chatbox_runScript` in the key's
    /// handler), which runs its first tick this tick.
    fn describe(&mut self, joy: &Joypad, lines: u8, from_cross_window: bool) {
        let mut chatbox = Chatbox::new(Script::Description { breaks: lines.saturating_sub(1) });
        chatbox.update(joy.held, joy.pressed);
        self.phase = Phase::Description { from_cross_window, chatbox };
    }

    /// DustCross's scrap (`sub_8027406`): every 25 ticks the last picked
    /// chip leaves the folder; then the folder closes up, the scrapped
    /// chips go to its end, and the hand is dealt again.
    fn scrap(&mut self, view: &PlayerView, folder: &mut BattleFolder) {
        let Phase::Scrapping { tick, done, mut scrapped, mut count } = self.phase else { unreachable!() };
        if done {
            // sub_802750C
            self.look.play(ScreenSound::ScrapDone);
            let button = &mut self.slots[8];
            button.uses_left = button.uses_left.saturating_sub(1);
            button.state = if button.uses_left == 0 { SlotState::Selected } else { SlotState::Selectable };
            self.phase = Phase::Choosing;
            self.update_availability(view, folder);
            return;
        }
        let tick = tick + 1;
        // The window's frame counter is the scrap's timer, from 24
        // (`sub_8027434`).
        self.look.frame = if tick == 1 { SCRAP_TIMER_START } else { self.look.frame + 1 };
        if tick >= 2 && (tick - 2) % 25 == 0 {
            let last = self.selected.checked_sub(1).map(|i| self.selection[i as usize]);
            match last.map(|s| self.slots[s as usize].kind) {
                Some(SlotKind::Chip { index, .. }) => {
                    scrapped[count as usize] = folder.take(index as usize);
                    count += 1;
                    self.selected -= 1;
                    // sub_80281D4, sub_8029CD4: the pick's icon and cell
                    // go; the chip window is drawn again.
                    self.look.column[self.selected as usize] = None;
                    self.show_chip_window(folder, view);
                    self.look.play(ScreenSound::Scrap);
                }
                _ => {
                    // sub_802945A, sub_80294E0, sub_802A61A: close up, put
                    // the scrapped chips back at the end (in pick order),
                    // deal again.
                    folder.compact();
                    for c in scrapped[..count as usize].iter().rev().flatten() {
                        folder.put_in_first_gap(*c);
                    }
                    let dealt = folder.count().min(self.hand_size as usize);
                    for slot in self.slots[..dealt].iter_mut() {
                        if slot.state == SlotState::Selected {
                            slot.state = SlotState::Selectable;
                        }
                    }
                    self.phase = Phase::Scrapping { tick, done: true, scrapped, count };
                    return;
                }
            }
        }
        self.phase = Phase::Scrapping { tick, done: false, scrapped, count };
    }

    /// ChpShufl's re-deal (`sub_80271F8`, state 0x28), drawing from the
    /// console's RNG. The first tick (`sub_802721C`) shuffles the chips it
    /// deals again into a new order (`sub_8029788`) and marks the button
    /// in use. Then every 4 ticks (`sub_802723A`) the chips are shown
    /// shuffled once more (`sub_8029688`: shuffled in the folder itself),
    /// until on the 32nd the new order lands (`sub_802983C`), the button
    /// has a use fewer, and the grid takes keys again. The availability is
    /// redone each time.
    fn redeal(&mut self, view: &PlayerView, folder: &mut BattleFolder, console: &mut Console) {
        let Phase::Redealing { started, elapsed, mut deal } = self.phase else { unreachable!() };
        let places = self.redeal_places(console.tag_pair);
        if !started {
            let n = places.len();
            for (d, &i) in deal.chips.iter_mut().zip(&places) {
                *d = folder.chips[i];
            }
            deal.count = n as u8;
            // The table that could shuffle the dealt chips apart
            // (`byte_80298C8`) is all zeros: one shuffle of them all.
            if n != 0 {
                shuffle(&mut deal.chips[..n], n, &mut console.rng);
            }
            self.slots[8].state = SlotState::Selected;
            self.update_availability(view, folder);
            self.phase = Phase::Redealing { started: true, elapsed: 0, deal };
            // The window's frame counter is the re-deal's (`+0x40`).
            self.look.frame = 0;
            return;
        }
        let elapsed = elapsed + 1;
        self.look.frame = elapsed as u32;
        self.phase = Phase::Redealing { started, elapsed, deal };
        if elapsed % REDEAL_STEP != 0 {
            return;
        }
        if elapsed / REDEAL_STEP >= REDEAL_STEPS {
            for (&i, &c) in places.iter().zip(&deal.chips[..deal.count as usize]) {
                folder.chips[i] = c;
            }
            let button = &mut self.slots[8];
            button.uses_left = button.uses_left.wrapping_sub(1);
            button.state = if button.uses_left != 0 { SlotState::Selectable } else { SlotState::Unavailable };
            self.phase = Phase::Choosing;
            self.show_chip_window(folder, view);
        } else {
            let mut shown: Vec<Option<FolderChip>> = places.iter().map(|&i| folder.chips[i]).collect();
            let n = shown.len();
            if n != 0 {
                shuffle(&mut shown, n, &mut console.rng);
            }
            for (&i, c) in places.iter().zip(shown) {
                folder.chips[i] = c;
            }
        }
        self.update_availability(view, folder);
        self.look.play(ScreenSound::RedealShuffle);
    }

    /// The folder entries a re-deal shuffles, as its routines walk them
    /// (`sub_8029788`, `sub_8029688`, `sub_802983C`): over the first
    /// hand-size slots, each chip slot is the next entry, taken unless it
    /// is picked or the Regular chip; then as many more entries as the
    /// folder had chips beyond the hand size, skipping the tag pair (two
    /// entries at `tag_pair`) when the walk reaches it. (With NumbrOpn's
    /// ten chips the re-deal button covers slots 8 and 9, so the walk
    /// counts eight dealt entries and leaves the folder's last two out.
    /// Where the tag pair straddles the end the original's walk runs on
    /// past the folder; this one stops at it.)
    fn redeal_places(&self, tag_pair: Option<u8>) -> Vec<usize> {
        let mut places = Vec::new();
        let mut at = 0usize;
        for slot in &self.slots[..(self.hand_size as usize).min(SLOTS)] {
            if let SlotKind::Chip { regular, .. } = slot.kind {
                if slot.state != SlotState::Selected && !regular {
                    places.push(at);
                }
                at += 1;
            }
        }
        let mut left = self.chips_left as i32 - self.hand_size as i32;
        while left > 0 && at < FOLDER_SIZE {
            if tag_pair.is_some_and(|t| t != 0 && t as usize == at) {
                at += 2;
                left -= 2;
            } else {
                places.push(at);
                at += 1;
                left -= 1;
            }
        }
        places
    }

    /// `sub_8028E32`: grey out what doesn't go with the selection.
    pub(crate) fn update_availability(&mut self, view: &PlayerView, folder: &BattleFolder) {
        // sub_8028E4C: what the picked chips have in common.
        let mut same_chip = Common::Any;
        let mut code = Common::Any;
        let mut special = None;
        for &s in self.selection() {
            let Some(c) = self.chip_in(s, folder) else { continue };
            let c = checked(c, view);
            if is_beast_out(c, view) {
                continue;
            }
            if SPECIAL_CODES.contains(&c.code) {
                special = Some(c.code);
                continue;
            }
            if c.code != ChipCode::ASTERISK {
                code = code.and(c.code);
            }
            same_chip = same_chip.and(c.id);
        }
        // updateCustomScreen_WhenUnselectingChip_8028EC8
        let full = self.selected as usize >= MAX_SELECTIONS;
        for s in 0..10u8 {
            let Some(c) = self.chip_in(s, folder) else { continue };
            let slot = &mut self.slots[s as usize];
            if slot.state == SlotState::Selected {
                continue;
            }
            let c = checked(c, view);
            let ok = if full {
                false
            } else if is_beast_out(c, view) {
                true
            } else if SPECIAL_CODES.contains(&c.code) {
                special.is_none_or(|s| s == c.code)
            } else if same_chip == Common::One(c.id) {
                true
            } else {
                match code {
                    Common::Mixed => false,
                    Common::Any => true,
                    Common::One(k) => c.code == ChipCode::ASTERISK || c.code == k,
                }
            };
            slot.state = if ok { SlotState::Selectable } else { SlotState::Unavailable };
        }
        self.update_beast_out(view);
        // sub_8028F84: the scrap button needs a picked chip last.
        let last_is_chip = self.selected > 0
            && matches!(self.slots[self.selection[self.selected as usize - 1] as usize].kind, SlotKind::Chip { .. });
        let button = &mut self.slots[8];
        if matches!(button.kind, SlotKind::Scrap { right_half: false }) && button.state != SlotState::Selected {
            button.state = if last_is_chip { SlotState::Selectable } else { SlotState::Unavailable };
        }
        // sub_8028250: the slots are drawn again.
        self.draw_slots(folder, view);
    }

    /// `sub_8028F48`: Beast Out can be picked with room left, no Cross
    /// chosen, and the navi able to.
    fn update_beast_out(&mut self, view: &PlayerView) {
        let full = self.selected as usize >= MAX_SELECTIONS;
        let cross = self.crosses.chosen.is_some();
        let s = &mut self.slots[SPECIAL_SLOT as usize];
        if s.kind == SlotKind::BeastOut && s.state != SlotState::Selected {
            s.state = if !full && view.beast_out_available() && !cross { SlotState::Selectable } else { SlotState::Unavailable };
        }
    }
}

/// What picked chips share: nothing yet, one value, or differing values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Common<T> {
    Any,
    One(T),
    Mixed,
}

impl<T: PartialEq + Copy> Common<T> {
    fn and(self, v: T) -> Common<T> {
        match self {
            Common::Any => Common::One(v),
            Common::One(x) if x == v => self,
            _ => Common::Mixed,
        }
    }
}

/// The scrap's timer starts at 24, so that its first chip goes on its
/// second tick (`sub_8027434`).
const SCRAP_TIMER_START: u32 = 0x18;

/// The tick of a Cross's choice the white fade is over and the Cross put
/// on (`sub_8027AAE`: 16 ticks, then 8 steps of the fade, then one more).
pub const CROSS_PUT_ON_TICK: u8 = 25;

/// The speed of the white fade a Cross's choice runs.
const CROSS_FADE_SPEED: u8 = 0x20;

/// The window's offset off the screen, and its slide a tick.
const SLIDE: u32 = 0x78;
const SLIDE_STEP: u32 = 12;
/// Beast Out's screen fades step 8 a frame.
const BEAST_OUT_FADE_SPEED: u8 = 8;

/// The re-deal's steps: every 4 ticks, the 8th lands.
const REDEAL_STEP: u8 = 4;
const REDEAL_STEPS: u8 = 8;
/// The camera shake of Beast Out on the custom screen (`sub_80302B6(1,
/// 0x28)`): magnitude 1, 40 ticks.
const BEAST_OUT_SHAKE: (u16, u16) = (1, 0x28);

/// Scan `list` from `start` for the first slot present.
fn scan(list: &[u8], start: u8, absent: impl Fn(u8) -> bool) -> u8 {
    let mut i = start as usize;
    while absent(list[i]) {
        i += 1;
    }
    list[i]
}

/// `sub_802A040`: the face of the Beast form Beast Out takes the navi to
/// (when `tired` counts, Beast Over's).
fn beast_face(view: &PlayerView, tired: bool) -> Option<nettai_content_api::FormHandle> {
    view.unlocks.beast_form(view.library, view.stats.navi, view.stats.form, tired)
}

/// `sub_802A088`: the face of the Cross chosen (by its place), its Beast
/// form's while the navi is in Beast Out.
fn cross_face(view: &PlayerView, cross: u8) -> Option<nettai_content_api::FormHandle> {
    let form = view.unlocks.cross_at(view.library, view.stats.navi, cross)?;
    match view.library.form_kind(view.stats.form) {
        crate::content::FormKind::Base | crate::content::FormKind::Cross => Some(form),
        _ => view.library.form_in_beast_out(form),
    }
}

/// The BeastOut chip, as the column shows Beast Out.
fn beast_out_icon(view: &PlayerView) -> Option<FolderChip> {
    view.library.beast_out_chip().map(|id| FolderChip { id, code: ChipCode(0) })
}

/// `getChipID_802A54E`: a chip as it counts in a selection. It is the
/// invalid chip when it is a Mega or Giga chip past the navi's limit for
/// the battle, or its code isn't one the chip comes in.
pub fn checked(c: FolderChip, view: &PlayerView) -> FolderChip {
    let limited = within_limit(c, view);
    if limited != c {
        return limited;
    }
    // (The original also skips the check for chips past its chip table,
    // which no folder holds.)
    if c.code != INVALID_CODE && !view.library.chip(c.id).codes.contains(&c.code) {
        return FolderChip { id: view.library.invalid_chip(), code: INVALID_CODE };
    }
    c
}

/// `sub_802A53C`: a chip as the class limits count it, the invalid chip
/// when it is a Mega or Giga chip past the navi's limit for the battle
/// (`checked` without the code).
fn within_limit(c: FolderChip, view: &PlayerView) -> FolderChip {
    let d = view.library.chip(c.id);
    if !SPECIAL_CODES.contains(&c.code) {
        let limit = match d.class {
            ChipClass::Mega => Some((view.class_uses.mega, view.stats.mega_level)),
            ChipClass::Giga => Some((view.class_uses.giga, view.stats.giga_level)),
            _ => None,
        };
        if limit.is_some_and(|(used, max)| used > max) {
            return FolderChip { id: view.library.invalid_chip(), code: INVALID_CODE };
        }
    }
    c
}

/// A chip with the dark flag (its record's `0x20`).
fn is_dark(c: FolderChip, view: &PlayerView) -> bool {
    view.library.chip(c.id).flags.has(crate::content::ChipFlags::DARK)
}

/// `sub_80280A2`: a link navi's own chip (`word_802A828`), unless it was
/// used this round.
fn navi_chip(view: &PlayerView) -> Option<FolderChip> {
    if view.round.navi_chips_used & (1 << view.stats.navi.0) != 0 {
        return None;
    }
    view.library.navi_chip(view.stats.navi)
}

/// `sub_802A40C`: how many chips a screen deals.
fn hand_size(view: &PlayerView, turn: u8, charge_cross_screens: u8, scrap_button: bool) -> u8 {
    let s = view.stats;
    let mut extra: i16 = 0;
    let mut n = s.custom_level as i16 + charge_cross_screens as i16;
    if n > 8 {
        extra = n - 8;
        n = 8;
    }
    if !view.form_traits().has(FormTraits::SCRAP_BUTTON) && !scrap_button && s.number_open {
        n = 10;
        extra = charge_cross_screens as i16;
    }
    let bug = s.bugs.hand_shrink_turn;
    if bug != 0 && turn >= bug {
        extra -= (turn - bug + 1) as i16;
        if extra < 0 {
            n = (n + extra).max(2);
        }
    }
    n as u8
}

impl PlayerView<'_> {
    /// The navi changes form: where the original asks whether it is
    /// MegaMan.
    pub fn megaman(&self) -> bool {
        self.library.changes_form(self.stats.navi)
    }

    /// What the screen asks of the navi's form.
    pub fn form_traits(&self) -> FormTraits {
        self.library.form_traits(self.stats.form)
    }

    /// `sub_8029F70` (battle modes 0, 0xA and 0xB).
    fn crosses_allowed(&self) -> bool {
        !self.random_battle
            && if self.unlocks.beast_out_sealed {
                self.megaman()
            } else {
                !self.per_player_gauges
            }
    }

    /// `sub_8029EC8`: how many Crosses the navi owns and hasn't used this
    /// round.
    fn crosses_left(&self) -> usize {
        (0..CROSSES as u8).filter(|&i| self.unlocks.owns_cross(i) && !self.round.crosses_used[i as usize]).count()
    }

    /// `sub_8029EF8`: the Crosses owned, not used this round, and not the
    /// one the navi starts battles in.
    fn offered_crosses(&self) -> CrossWindow {
        let mut w = CrossWindow::default();
        for i in 0..CROSSES as u8 {
            // (A Cross the content doesn't have isn't offered.)
            let Some(form) = self.unlocks.cross_at(self.library, self.stats.navi, i) else { continue };
            let starting = self.stats.starting_form;
            if self.unlocks.owns_cross(i) && !self.round.crosses_used[i as usize] && starting != form && self.listed_cross_fits(form) {
                w.offered[w.count as usize] = i;
                w.count += 1;
            }
        }
        w
    }

    /// What a setup's Cross list (`Unlocks::cross_list`, nettai's
    /// extension) offers of its entries: Crosses only, and in a Beast
    /// form only the Crosses whose Beast it is (that game's: their forms
    /// in Beast Out are that Beast's). Everything else offers.
    fn listed_cross_fits(&self, form: nettai_content_api::FormHandle) -> bool {
        if self.unlocks.cross_list.is_none() {
            return true;
        }
        let current = self.stats.form;
        let in_beast = self.library.form_kind(current).is_beast();
        self.library.form_kind(form) == crate::content::FormKind::Cross
            && !(in_beast && self.library.form_game(form) != self.library.form_game(current))
    }

    /// The game of the Beast the navi goes into, or is in
    /// (`Unlocks::beast_game`): the Beast Out roar's.
    fn beast_game(&self) -> super::GameVersion {
        self.unlocks.beast_game(self.library, self.stats.form)
    }

    /// `sub_8029FB4` (battle mode 0): the Beast Out button is on the
    /// screen.
    fn beast_out_button(&self) -> bool {
        self.megaman()
            && !self.unlocks.beast_out_sealed
            && !self.per_player_gauges
            && !self.random_battle
            && self.unlocks.beast_out
    }

    /// `sub_802A57E`: the navi can Beast Out now: not worn out, not in a
    /// Beast form. Tired, it can only once it has this round (and then
    /// goes Beast Over).
    fn beast_out_available(&self) -> bool {
        self.emotion != Emotion::WornOut
            && (self.emotion != Emotion::Tired || self.round.beast_out_used)
            && !self.library.form_kind(self.stats.form).is_beast()
    }
}
