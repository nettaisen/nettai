//! One player's custom screen: the slots, the cursor, the selection and
//! the sub-screens (the Cross window, Beast Out, the chip descriptions),
//! driven by that player's joypad. The original runs this on each
//! console for its own player only (`sub_8026A88` and its states); here
//! both players' screens run in the simulation. See
//! docs/engine/custom-screen.md §2-§4.

use super::folder::{BattleFolder, FolderChip};
use super::builder::{ClassCounts, FormedAdvance};
use super::library::Library;
use super::{GameVersion, Unlocks};
use crate::content::{BannerId, ChipClass, ChipCode, ChipId, CustomScreenLayout, TemplateSlot};
use crate::hud::{Banner, BannerStatus};
use crate::input::{Joypad, keys};
use crate::kinds::player::Emotion;
use crate::setup::{Form, NaviStats};

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

/// The chip id a selection turns into when it isn't allowed (the
/// "error" chip), with code 0x1B.
pub const INVALID_CHIP: FolderChip = FolderChip { id: 0x185, code: ChipCode(0x1B) };
/// Codes outside the alphabet that the selection rules treat apart:
/// the invalid chip's, and one no chip has.
const SPECIAL_CODES: [ChipCode; 2] = [ChipCode(0x1B), ChipCode(0x1C)];
/// The "BeastOut" chip, as a folder chip (not the Beast Out button).
const BEAST_OUT_CHIP: ChipId = 0x13F;

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

/// The Crosses a screen offers (`+0x50`), by the version's Cross number
/// (0-4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CrossWindow {
    pub offered: [u8; CROSSES],
    pub count: u8,
    /// Per offered entry: chosen (only one can be).
    pub marked: [bool; CROSSES],
    /// The entry under the window's cursor.
    pub cursor: u8,
    /// The chosen Cross (its number), if any.
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
    /// for a Cross's description (`sub_8026E78`).
    Description { from_cross_window: bool, elapsed: u16, dismissed_at: Option<u16> },
    /// L: the "no time to run" message (`sub_8026E98`).
    RunMessage { elapsed: u16, dismissed_at: Option<u16> },
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
    /// DustCross scraps the selected chips (`sub_8027406`).
    /// `done`: the last scrap is over; the next tick returns to choosing.
    Scrapping { tick: u16, done: bool, scrapped: [Option<FolderChip>; MAX_SELECTIONS], count: u8 },
    /// OK was pressed; the window slides out (`sub_8026BF4`, 10 ticks).
    Closing { tick: u8 },
    /// The Program Advance animation (`sub_8026DB0`).
    ProgramAdvance { anim: ProgramAdvanceAnimation },
    /// The result is on its way to the other player (`sub_8026DC4`);
    /// `started`: its first tick, which sends it, has run.
    Sending { started: bool },
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
const PROGRAM_ADVANCE_BANNER: BannerId = BannerId(0x24);
const EMPTY_RECIPE_BANNER: BannerId = BannerId(0x34);

/// Frames the animation's screen fades take (`SetScreenFade(0x14, 8)` out
/// to level 0x40, `SetScreenFade(0x10, 8)` back in to 0; the level starts
/// at 0, where every fade before the custom screen left it; a fade in
/// holds its first frame).
const FADE_OUT_FRAMES: u8 = 0x40 / 8;
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
}

/// What a player's screens remember through a round (`dword_20349A0`,
/// cleared on the round's first screen).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RoundMemory {
    /// Crosses used this round, by the version's Cross number.
    pub crosses_used: [bool; CROSSES],
    /// Beast Out was picked this round.
    pub beast_out_used: bool,
    /// Consecutive screens opened in ChargeCross (or its Beast form),
    /// up to 3: each deals one more chip.
    pub charge_cross_screens: u8,
    /// Navi chips (ids 0x190 and up) put in a hand this round, by id.
    pub navi_chips_used: u16,
}

impl Screen {
    /// The screen a player gets when the custom screen opens (`sub_8026840`):
    /// compact the folder, deal, and lay out the slots. `turn`: the
    /// screen's number in the round (1 = first).
    pub fn open(folder: &mut BattleFolder, view: &PlayerView, turn: u8, round: &mut RoundMemory) -> Screen {
        let stats = view.stats;
        let megaman = stats.navi == crate::setup::Navi::MEGAMAN;
        // sub_802A49C: ChargeCross deals one more chip per screen spent in
        // it, up to three.
        let form = stats.form;
        round.charge_cross_screens = if megaman && (form == Form::CHARGE_CROSS || form == Form::CHARGE_CROSS.with_beast()) {
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
        };
        if view.crosses_allowed() && view.emotion != Emotion::WornOut {
            screen.crosses = view.offered_crosses();
        }
        screen.hand_size = hand_size(view, turn, round.charge_cross_screens, false);
        screen.lay_out(view);
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
        let form = view.stats.form;
        if self.megaman && (form == Form::DUST_CROSS || form == Form::DUST_CROSS.with_beast()) {
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

    /// One tick of the screen with this joypad, then the console's HUD
    /// banner.
    pub fn tick(&mut self, joy: &Joypad, view: &PlayerView, folder: &mut BattleFolder) -> Option<Request> {
        let request = self.step(joy, view, folder);
        self.hud.tick();
        if let Phase::ProgramAdvance { anim } = &mut self.phase {
            anim.fade = anim.fade.saturating_sub(1);
        }
        request
    }

    fn step(&mut self, joy: &Joypad, view: &PlayerView, folder: &mut BattleFolder) -> Option<Request> {
        match self.phase {
            Phase::Opening { tick } => {
                let tick = tick + 1;
                self.phase = if tick >= 10 { Phase::Choosing } else { Phase::Opening { tick } };
                None
            }
            Phase::Choosing => self.choose(joy, view, folder),
            Phase::Hidden { stage } => {
                self.phase = match stage {
                    HiddenStage::Hiding => Phase::Hidden { stage: HiddenStage::Waiting },
                    HiddenStage::Waiting if joy.pressed != 0 => Phase::Hidden { stage: HiddenStage::Restoring },
                    HiddenStage::Waiting => self.phase,
                    HiddenStage::Restoring => Phase::Choosing,
                };
                None
            }
            Phase::Description { from_cross_window, elapsed, dismissed_at } => {
                // The screen waits for the chatbox to close; the chatbox
                // (after the screen, each tick) takes any key once armed.
                let elapsed = elapsed + 1;
                if dismissed_at.is_some_and(|d| elapsed >= d + DISMISS_TICKS) {
                    self.phase = if from_cross_window { Phase::CrossWindow { entered: false } } else { Phase::Choosing };
                    return None;
                }
                let dismissed_at = dismissed_at.or((elapsed >= DESCRIPTION_ARM && joy.pressed != 0).then_some(elapsed));
                self.phase = Phase::Description { from_cross_window, elapsed, dismissed_at };
                None
            }
            Phase::RunMessage { elapsed, dismissed_at } => {
                let elapsed = elapsed + 1;
                if dismissed_at.is_some_and(|d| elapsed >= d + DISMISS_TICKS) {
                    self.phase = Phase::Choosing;
                    return None;
                }
                let answered = joy.pressed & (keys::A | keys::B) != 0;
                let dismissed_at = dismissed_at.or((elapsed >= RUN_MESSAGE_ARM && answered).then_some(elapsed));
                self.phase = Phase::RunMessage { elapsed, dismissed_at };
                None
            }
            Phase::CrossWindowOpening { tick } => {
                let tick = tick + 1;
                self.phase = if tick >= 12 { Phase::CrossWindow { entered: true } } else { Phase::CrossWindowOpening { tick } };
                None
            }
            Phase::CrossWindow { entered: false } => {
                self.phase = Phase::CrossWindow { entered: true };
                None
            }
            Phase::CrossWindow { entered: true } => {
                self.cross_window(joy, view);
                None
            }
            Phase::CrossWindowClosing { tick } => {
                let tick = tick + 1;
                self.phase = if tick >= 6 { Phase::Choosing } else { Phase::CrossWindowClosing { tick } };
                None
            }
            Phase::CrossChosen { tick } => {
                let tick = tick + 1;
                self.phase = if tick >= 34 { Phase::Choosing } else { Phase::CrossChosen { tick } };
                None
            }
            Phase::BeastOutChosen { tick } => {
                let tick = tick + 1;
                match tick {
                    1 => self.beast_out = true,
                    53 => {
                        // Beast Out goes first in the selection, so B takes
                        // it back last.
                        let n = self.selected as usize;
                        self.selection[..n].rotate_right(1);
                        self.slots[SPECIAL_SLOT as usize].state = SlotState::Selected;
                        self.update_availability(view, folder);
                    }
                    _ => {}
                }
                self.phase = if tick >= 70 { Phase::Choosing } else { Phase::BeastOutChosen { tick } };
                None
            }
            Phase::Scrapping { .. } => {
                self.scrap(view, folder);
                None
            }
            Phase::Closing { tick } => {
                let tick = tick + 1;
                self.phase = match tick {
                    10 if self.program_advance.is_some() => Phase::ProgramAdvance { anim: Default::default() },
                    10 => Phase::Sending { started: false },
                    _ => Phase::Closing { tick },
                };
                None
            }
            Phase::ProgramAdvance { mut anim } => {
                let pa = self.program_advance.expect("a Program Advance formed");
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
        match anim.step {
            S::FadeOut => {
                if !anim.started {
                    anim.started = true;
                    anim.fade = FADE_OUT_FRAMES;
                    return;
                }
                if anim.fade != 0 {
                    return;
                }
                let id = if pa.len != 0 { PROGRAM_ADVANCE_BANNER } else { EMPTY_RECIPE_BANNER };
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
                self.cursor = t;
            }
            return None;
        }
        let p = joy.pressed;
        if p & keys::A != 0 {
            return self.press_a(view, folder);
        } else if p & keys::B != 0 {
            self.deselect(view, folder);
        } else if p & keys::START != 0 {
            self.cursor = OK_SLOT;
        } else if p & keys::SELECT != 0 {
            self.phase = Phase::Hidden { stage: HiddenStage::Hiding };
        } else if p & keys::R != 0 {
            if matches!(here.kind, SlotKind::Chip { .. } | SlotKind::NaviChip(_)) {
                self.phase = Phase::Description { from_cross_window: false, elapsed: 0, dismissed_at: None };
            }
        } else if p & keys::L != 0 {
            self.phase = Phase::RunMessage { elapsed: 0, dismissed_at: None };
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
                    return None;
                }
                self.push_selection(cursor);
                self.slots[cursor as usize].state = SlotState::Selected;
                self.update_availability(view, folder);
                if self.chip_in(cursor, folder).is_some_and(|c| c.id == BEAST_OUT_CHIP) {
                    panic!("the BeastOut chip in a chip slot (custom screen state 0x44) is not implemented yet");
                }
            }
            SlotKind::Ok => {
                // sub_8028D3A: the battle builds the hand and the
                // transform request, then the window slides out.
                self.phase = Phase::Closing { tick: 0 };
                return Some(Request::Confirm);
            }
            SlotKind::BeastOut => {
                // sub_8028D6C
                if here.state != SlotState::Selectable || self.selected as usize >= MAX_SELECTIONS {
                    return None;
                }
                self.push_selection(cursor);
                self.phase = Phase::BeastOutChosen { tick: 0 };
            }
            SlotKind::Scrap { right_half } => {
                // sub_8028E04
                let button = if right_half { 8 } else { cursor };
                if self.slots[button as usize].state == SlotState::Selectable {
                    self.phase = Phase::Scrapping { tick: 0, done: false, scrapped: [None; MAX_SELECTIONS], count: 0 };
                }
            }
            SlotKind::Redeal { right_half } => {
                let button = if right_half { 8 } else { cursor };
                if self.slots[button as usize].state == SlotState::Selectable {
                    panic!("ChpShufl's re-deal (custom screen state 0x28) is not implemented yet");
                }
            }
            SlotKind::Empty | SlotKind::Hidden => {}
        }
        None
    }

    fn push_selection(&mut self, slot: u8) {
        self.selection[self.selected as usize] = slot;
        self.selected += 1;
    }

    /// B (`sub_8029032`): take back the last pick; with none, the Cross.
    fn deselect(&mut self, view: &PlayerView, folder: &BattleFolder) {
        if self.selected == 0 {
            let Some(_) = self.crosses.chosen else { return };
            self.crosses.marked[self.crosses.cursor as usize] = false;
            self.crosses.chosen = None;
        } else {
            let last = self.selection[self.selected as usize - 1];
            self.selected -= 1;
            if self.slots[last as usize].kind == SlotKind::BeastOut {
                self.beast_out = false;
            }
            self.slots[last as usize].state = SlotState::Selectable;
        }
        self.update_availability(view, folder);
    }

    /// The Cross window's keys (`sub_8028A78`).
    fn cross_window(&mut self, joy: &Joypad, view: &PlayerView) {
        let w = &mut self.crosses;
        if w.chosen.is_none() {
            let n = w.count;
            if joy.repeat & keys::UP != 0 {
                w.cursor = if w.cursor == 0 { n - 1 } else { w.cursor - 1 };
                return;
            }
            if joy.repeat & keys::DOWN != 0 {
                w.cursor = if w.cursor + 1 >= n { 0 } else { w.cursor + 1 };
                return;
            }
            if joy.pressed & keys::A != 0 {
                let i = w.cursor as usize;
                if w.marked[i] {
                    return;
                }
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
        } else if p & keys::R != 0 {
            self.phase = Phase::Description { from_cross_window: true, elapsed: 0, dismissed_at: None };
        }
    }

    /// DustCross's scrap (`sub_8027406`): every 25 ticks the last picked
    /// chip leaves the folder; then the folder closes up, the scrapped
    /// chips go to its end, and the hand is dealt again.
    fn scrap(&mut self, view: &PlayerView, folder: &mut BattleFolder) {
        let Phase::Scrapping { tick, done, mut scrapped, mut count } = self.phase else { unreachable!() };
        if done {
            // sub_802750C
            let button = &mut self.slots[8];
            button.uses_left = button.uses_left.saturating_sub(1);
            button.state = if button.uses_left == 0 { SlotState::Selected } else { SlotState::Selectable };
            self.phase = Phase::Choosing;
            self.update_availability(view, folder);
            return;
        }
        let tick = tick + 1;
        if tick >= 2 && (tick - 2) % 25 == 0 {
            let last = self.selected.checked_sub(1).map(|i| self.selection[i as usize]);
            match last.map(|s| self.slots[s as usize].kind) {
                Some(SlotKind::Chip { index, .. }) => {
                    scrapped[count as usize] = folder.take(index as usize);
                    count += 1;
                    self.selected -= 1;
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

    /// `sub_8028E32`: grey out what doesn't go with the selection.
    pub(crate) fn update_availability(&mut self, view: &PlayerView, folder: &BattleFolder) {
        // sub_8028E4C: what the picked chips have in common.
        let mut same_chip = Common::Any;
        let mut code = Common::Any;
        let mut special = None;
        for &s in self.selection() {
            let Some(c) = self.chip_in(s, folder) else { continue };
            let c = checked(c, view);
            if c.id == BEAST_OUT_CHIP {
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
            } else if c.id == BEAST_OUT_CHIP {
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

/// A chip description's chatbox (`chatbox_onUpdate`): it takes a key from
/// the 6th tick after R; the screen sees it closed 5 ticks after the key
/// and reads input again the tick after.
const DESCRIPTION_ARM: u16 = 6;
const DISMISS_TICKS: u16 = 5;
/// [unverified] The run message ("no time to run away!") takes A or B once
/// printed; this is an estimate of its printing time (the chatbox's text
/// timing isn't ported; no recording has one).
const RUN_MESSAGE_ARM: u16 = 72;
/// Scan `list` from `start` for the first slot present.
fn scan(list: &[u8], start: u8, absent: impl Fn(u8) -> bool) -> u8 {
    let mut i = start as usize;
    while absent(list[i]) {
        i += 1;
    }
    list[i]
}

/// `getChipID_802A54E`: a chip as it counts in a selection. It is the
/// invalid chip when it is a Mega or Giga chip past the navi's limit for
/// the battle, or its code isn't one the chip comes in.
pub fn checked(c: FolderChip, view: &PlayerView) -> FolderChip {
    let d = view.library.chip(c.id);
    if !SPECIAL_CODES.contains(&c.code) {
        let limit = match d.class {
            ChipClass::Mega => Some((view.class_uses.mega, view.stats.mega_level)),
            ChipClass::Giga => Some((view.class_uses.giga, view.stats.giga_level)),
            _ => None,
        };
        if limit.is_some_and(|(used, max)| used > max) {
            return INVALID_CHIP;
        }
    }
    let code_checked = c.code != ChipCode(0x1B) && c.id < 0x19B;
    if code_checked && !d.codes.contains(&c.code) {
        return INVALID_CHIP;
    }
    c
}

/// `sub_80280A2`: a link navi's own chip (`word_802A828`), unless it was
/// used this round.
fn navi_chip(view: &PlayerView) -> Option<FolderChip> {
    let navi = view.stats.navi;
    if view.round.navi_chips_used & (1 << navi.0) != 0 {
        return None;
    }
    view.library.navi_chip(navi)
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
    if s.form != Form::DUST_CROSS && s.form != Form::DUST_CROSS.with_beast() && !scrap_button && s.number_open {
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
    /// `sub_8029F70` (battle modes 0, 0xA and 0xB).
    fn crosses_allowed(&self) -> bool {
        !self.random_battle
            && if self.unlocks.beast_out_sealed {
                self.stats.navi == crate::setup::Navi::MEGAMAN
            } else {
                !self.per_player_gauges
            }
    }

    /// `sub_8029EF8`: the Crosses owned, not used this round, and not the
    /// one the navi starts battles in.
    fn offered_crosses(&self) -> CrossWindow {
        let mut w = CrossWindow::default();
        for i in 0..CROSSES as u8 {
            let form = self.unlocks.version.cross_form(i);
            if self.unlocks.crosses[i as usize] && !self.round.crosses_used[i as usize] && self.stats.starting_form != form {
                w.offered[w.count as usize] = i;
                w.count += 1;
            }
        }
        w
    }

    /// `sub_8029FB4` (battle mode 0): the Beast Out button is on the
    /// screen.
    fn beast_out_button(&self) -> bool {
        self.stats.navi == crate::setup::Navi::MEGAMAN
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
            && !self.stats.form.is_beast()
    }
}

impl GameVersion {
    /// The form of Cross number `i` (0-4).
    pub fn cross_form(self, i: u8) -> Form {
        match self {
            GameVersion::Gregar => Form(1 + i),
            GameVersion::Falzar => Form(6 + i),
        }
    }

    pub fn beast_out(self) -> Form {
        match self {
            GameVersion::Gregar => Form::GREGAR_BEAST,
            GameVersion::Falzar => Form::FALZAR_BEAST,
        }
    }

    pub fn beast_over(self) -> Form {
        match self {
            GameVersion::Gregar => Form::GREGAR_BEAST_OVER,
            GameVersion::Falzar => Form::FALZAR_BEAST_OVER,
        }
    }
}

impl Form {
    /// A Cross's Beast form (Cross + Beast Out).
    pub fn with_beast(self) -> Form {
        Form(self.0 + 0x0C)
    }
}
