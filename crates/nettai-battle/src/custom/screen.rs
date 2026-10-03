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
use crate::console::Console;
use crate::battle::FadeMode;
use crate::content::{ButtonHandle, WindowHandle};
use nettai_content_api::ChipHandle;
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
    /// BN5's soul button (Soul Unison: slot 11, kind 2 of BN5's screen,
    /// 0x08023C54): it gives up the last chip picked for the soul of its
    /// family (`Screen::soul`).
    Soul,
    /// A cell of a system's button (docs/design/rules-in-luau.md §4.4:
    /// BN6's ChpShufl re-deal and DustCross scrap, two cells wide on slots
    /// 8 and 9).
    Button { button: ButtonHandle, cell: ButtonCell },
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

/// Which cell of a button a slot is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonCell {
    /// A button one cell wide.
    Only,
    /// The left and right cells of a button two wide.
    Left,
    Right,
}

/// Where a system's button sits on a screen, as the screen's extras say
/// when it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ButtonPlace {
    pub button: ButtonHandle,
    /// Its first slot, and how many it takes (1 or 2).
    pub slot: u8,
    pub cells: u8,
    /// Its uses on the screen.
    pub uses: u8,
    /// Its first cell's right neighbor and its last cell's left one, over
    /// the layout's.
    pub right: Option<u8>,
    pub left: Option<u8>,
}

/// Whether a slot can be picked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SlotState {
    #[default]
    Selectable,
    /// Grayed out: it doesn't go with the selection, or it can't be used
    /// now.
    Unavailable,
    /// Picked (a chip or Beast Out), or used up (a button).
    Selected,
}

/// A slot and its neighbors.
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
    /// R shows the chip's description (`sub_8026E4C`), or a system's window
    /// a form's (BN6's Cross window: the Cross's, `sub_8026E78`): `window`
    /// the window it returns to, `form` the form described. The screen
    /// waits for its chatbox to close.
    Description { window: Option<WindowHandle>, form: Option<nettai_content_api::FormHandle>, chatbox: Chatbox },
    /// L: the "no time to run" message (`sub_8026E98`), whose chatbox
    /// starts on the state's first tick (`sub_8026EC8`) and is waited for
    /// from the next (`sub_8026FAA`).
    RunMessage { chatbox: Option<Chatbox> },
    /// A system's window is up (docs/design/rules-in-luau.md §4.4: BN6's
    /// Beast Out, `sub_802770C`, and the Cross window): its `update` runs
    /// each tick, `tick` from 1, until it says it is done.
    Window { window: WindowHandle, tick: u16 },
    /// BN5's soul button was picked (its state 9, 0x080232D0): `sub` its
    /// sub-state (0x080232F0's offsets 0 to 0x18), `counter` its count
    /// (+0x40).
    SoulChosen { sub: u8, counter: u8 },
    /// The selected chips are scrapped (DustCross's, `sub_8027406`), for
    /// the button in slot `button`. `done`: the last scrap is over; the
    /// next tick returns to choosing.
    Scrapping { button: u8, tick: u16, done: bool, scrapped: [Option<FolderChip>; MAX_SELECTIONS], count: u8 },
    /// ChpShufl re-deals (`sub_80271F8`, state 0x28): `deal` is the new
    /// order, drawn on the first tick; every 4 ticks the chips are shown
    /// shuffled again, and on the 32nd the deal lands. `started`: the first
    /// tick has run; `elapsed`: ticks since (`+0x40`).
    Redealing { button: u8, started: bool, elapsed: u8, deal: Deal },
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
    /// The navi changes form (MegaMan: the dark chips' cursor and the
    /// window's colors are his).
    pub megaman: bool,
    /// The form a system's pick puts the navi in at the turn's start (BN6's
    /// Beast Out, `+0x17`), and the system's place in the side's ruleset.
    pub form: Option<nettai_content_api::FormHandle>,
    pub form_owner: Option<u8>,
    /// BN5's soul button: the soul it offers or gave (slot 11's +5 and +6)
    /// and the slot of the chip given up for it (+4).
    pub soul: SoulButton,
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

/// BN5's soul button's offer (slot 11's record): the soul's number (0:
/// none), whether a dark chip makes it Chaos Unison, and the slot of the
/// chip given up for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SoulButton {
    pub number: u8,
    pub chaos: bool,
    pub given_up: u8,
}

/// What the screen reads of its player when it opens and while it runs.
#[derive(Clone, Copy)]
pub struct PlayerView<'a> {
    pub library: &'a dyn Library,
    pub stats: &'a NaviStats,
    pub emotion: Emotion,
    /// BN5's Soul Unison (the setup's).
    pub souls: &'a super::SoulUnlocks,
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
    /// BN5's souls given this round (0x02034E10): bit n Soul Unison with
    /// soul n, bit 16 + n its Chaos Unison.
    pub souls_used: u32,
    /// The link navis whose own chips were put in a hand this round (a bit
    /// each: the original's by the chip's place among them).
    pub navi_chips_used: u32,
}

impl Screen {
    /// The screen a player gets when the custom screen opens (`sub_8026840`):
    /// compact the folder, deal, and lay out the slots. `turn`: the
    /// screen's number in the round (1 = first).
    pub fn open(folder: &mut BattleFolder, view: &PlayerView, turn: u8, extras: &mut dyn super::Extras) -> Screen {
        let megaman = view.megaman();
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
            form: None,
            form_owner: None,
            soul: SoulButton::default(),
            program_advance: None,
            hud: Banner::default(),
            look: ScreenLook::new(view.late_turns, false, None),
        };
        // The side's systems as the screen opens (BN6's: the round's
        // Beast Out and Crosses forgotten on its first screen, ChargeCross's
        // screens, the Crosses offered and the window's Cross tab).
        extras.opened(&mut screen);
        // sub_802A40C: the side's rules' hand size (BN6's cross system's,
        // with ChargeCross's chips), else the framework's.
        screen.hand_size = extras.hand_size().unwrap_or_else(|| hand_size(view, turn));
        screen.lay_out(view, extras);
        // sub_802806C: a cursor on the first slot goes to the first dark
        // chip dealt (as the class limits count it).
        if screen.cursor == 0 {
            if let Some(s) = (0..OK_SLOT).find(|&s| screen.chip_in(s, folder).is_some_and(|c| is_dark(within_limit(c, view), view))) {
                screen.cursor = s;
            }
        }
        // sub_8028476: the chip window shows the first slot.
        screen.show_chip_window(folder, view);
        screen.draw_slots(folder, view);
        screen
    }

    /// `sub_8027E2C`: the slots, the dealt chips and the cursor.
    fn lay_out(&mut self, view: &PlayerView, extras: &mut dyn super::Extras) {
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
        if view.soul_button() {
            // 0x08023C54: the soul button, unavailable until a pick
            // offers a soul.
            let s = &mut self.slots[SPECIAL_SLOT as usize];
            s.kind = SlotKind::Soul;
            s.state = SlotState::Unavailable;
        }
        let dealt = self.chips_left.min(self.hand_size);
        for i in 0..dealt {
            self.slots[i as usize].kind = SlotKind::Chip { index: i, regular: i == 0 && view.regular_pending };
        }
        // The side's systems' buttons (BN6's: DustCross's scrap,
        // `sub_8027F10`, else ChpShufl's re-deal, `sub_80280E0`), over the
        // chips dealt; the first to claim a slot keeps it.
        let mut taken = [false; SLOTS];
        for place in extras.buttons(self) {
            let cells = place.slot as usize..(place.slot + place.cells) as usize;
            if cells.end > SLOTS || taken[cells.clone()].iter().any(|&t| t) {
                continue;
            }
            taken[cells].fill(true);
            let state = extras.button_state(self, place.button).unwrap_or(SlotState::Selectable);
            let first = place.slot as usize;
            if place.cells == 1 {
                let d = self.slots[first];
                self.slots[first] = Slot {
                    kind: SlotKind::Button { button: place.button, cell: ButtonCell::Only },
                    right: place.right.or(d.right),
                    left: place.left.or(d.left),
                    state,
                    uses_left: place.uses,
                    ..d
                };
            } else {
                let d = self.slots[first];
                self.slots[first] = Slot {
                    kind: SlotKind::Button { button: place.button, cell: ButtonCell::Left },
                    right: place.right.or(d.right),
                    state,
                    uses_left: place.uses,
                    ..d
                };
                let d = self.slots[first + 1];
                self.slots[first + 1] =
                    Slot { kind: SlotKind::Button { button: place.button, cell: ButtonCell::Right }, left: place.left.or(d.left), ..d };
            }
        }
        if let Some(chip) = navi_chip(view) {
            // sub_80280A2: a link navi's own chip, once a round.
            self.slots[9].kind = SlotKind::NaviChip(chip);
        }
        self.fix_neighbors(layout);
        self.cursor = (0..SLOTS as u8).find(|&s| !self.slots[s as usize].kind.is_absent()).unwrap_or(OK_SLOT);
    }

    /// `sub_8027F42`: point neighbors that are absent at the next slot
    /// present along the row's scan list.
    fn fix_neighbors(&mut self, layout: &CustomScreenLayout) {
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
                let from = if bottom && matches!(d.kind, SlotKind::Button { cell: ButtonCell::Right, .. }) { s - 1 } else { s };
                let list: &[u8] = if bottom { &layout.left_scan_bottom } else { &layout.left_scan_top };
                let n = scan(list, layout.left_scan_start[from as usize], |x| absent(&self.slots, x));
                fixed.left = (n != from).then_some(n);
            }
            if d.right.is_some_and(|r| absent(&self.slots, r)) {
                let from = if bottom && matches!(d.kind, SlotKind::Button { cell: ButtonCell::Left, .. }) { s + 1 } else { s };
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
    pub fn tick(&mut self, joy: &Joypad, view: &PlayerView, folder: &mut BattleFolder, console: &mut Console, extras: &mut dyn super::Extras) -> Option<Request> {
        self.look.drawn = Default::default();
        let request = self.step(joy, view, folder, console, extras);
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
    pub(crate) fn draw_window(&mut self, folder: &BattleFolder) {
        self.look.draw_emblem(0);
        self.look.draw_regular(folder.regular_pending);
        self.look.draw_turn_limit();
    }

    /// `sub_802A394`: choosing chips or reading a chip's description, the
    /// cursor rests on a dark chip (as it counts in a selection).
    fn on_dark_chip(&self, view: &PlayerView, folder: &BattleFolder) -> bool {
        matches!(self.phase, Phase::Choosing | Phase::Description { window: None, .. })
            && self.chip_in(self.cursor, folder).is_some_and(|c| is_dark(checked(c, view), view))
    }

    fn step(&mut self, joy: &Joypad, view: &PlayerView, folder: &mut BattleFolder, console: &mut Console, extras: &mut dyn super::Extras) -> Option<Request> {
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
                let request = self.choose(joy, view, folder, extras);
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
            Phase::Description { window, form, mut chatbox } => {
                // The screen sees the chatbox closed the tick after it
                // closes, and reads keys again the tick after that; the
                // chatbox runs after the screen, each tick. The emblem is
                // drawn every tick (`sub_8026E4C`).
                self.look.draw_emblem(0);
                if !chatbox.is_open() {
                    self.look.play(ScreenSound::DescriptionClose);
                    // (Back to the window it came from: its first tick
                    // again, which reads no keys.)
                    self.phase = match window {
                        Some(window) => Phase::Window { window, tick: 0 },
                        None => Phase::Choosing,
                    };
                    return None;
                }
                chatbox.update(joy.held, joy.pressed);
                self.phase = Phase::Description { window, form, chatbox };
                None
            }
            Phase::RunMessage { chatbox } => {
                self.look.draw_emblem(0);
                let mut chatbox = match chatbox {
                    None => {
                        // sub_8026EC8
                        self.look.play(ScreenSound::RunMessage);
                        let navi = view.stats.navi;
                        Chatbox::new(Script::RunMessage { lines: view.library.run_message(navi) })
                            .talking(view.library.run_message_talking(navi))
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
            Phase::Window { window, tick } => {
                // The system's window (BN6's Beast Out, `sub_802770C`): its
                // update, then back to choosing when it is done (unless it
                // moved the screen on itself).
                let tick = tick.saturating_add(1);
                self.phase = Phase::Window { window, tick };
                let stays = extras.window_update(self, folder, console, joy, window);
                if !stays && self.phase == (Phase::Window { window, tick }) {
                    self.phase = Phase::Choosing;
                }
                None
            }
            Phase::SoulChosen { sub, counter } => {
                self.soul_chosen(sub, counter, view, folder, extras);
                self.look.draw_emblem(0);
                None
            }
            Phase::Scrapping { .. } => {
                // sub_8027406: every tick also draws the emblem and the
                // Regular chip's frame.
                self.scrap(view, folder, extras);
                self.look.draw_emblem(0);
                self.look.draw_regular(folder.regular_pending);
                None
            }
            Phase::Redealing { .. } => {
                // sub_80271F8: every tick also draws the emblem and the
                // Regular chip's frame.
                self.redeal(view, folder, console, extras);
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
    fn choose(&mut self, joy: &Joypad, view: &PlayerView, folder: &mut BattleFolder, extras: &mut dyn super::Extras) -> Option<Request> {
        // The side's systems first (BN6's UP opening the Cross window, which
        // the original asks before the cursor's UP).
        if (joy.repeat | joy.pressed) != 0 && extras.keys(self, folder, joy) {
            return None;
        }
        let here = self.slots[self.cursor as usize];
        let rep = joy.repeat;
        let target = if rep & (keys::UP | keys::DOWN) != 0 {
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
            return self.press_a(view, folder, extras);
        } else if p & keys::B != 0 {
            self.deselect(view, folder, extras);
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
                let lines = view.library.chip(checked(c, view).id).description_lines;
                self.describe(joy, lines, None, None);
                self.look.play(ScreenSound::Description);
            }
        } else if p & keys::L != 0 {
            self.phase = Phase::RunMessage { chatbox: None };
        }
        None
    }

    /// A on the slot under the cursor (`off_8028C9C`).
    fn press_a(&mut self, view: &PlayerView, folder: &mut BattleFolder, extras: &mut dyn super::Extras) -> Option<Request> {
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
                self.update_availability(view, folder, extras);
                // The pick's icon in the column, and the emblem spins.
                self.look.column[self.selected as usize - 1] = self.chip_in(cursor, folder).map(|c| checked(c, view));
                self.look.spin = 1;
                // sub_802A00C: the side's systems (BN6's BeastOut chip starts
                // its animation).
                if let Some(c) = self.chip_in(cursor, folder) {
                    extras.chip_picked(self, folder, c.id);
                }
            }
            SlotKind::Ok => {
                // sub_8028D3A: the battle builds the hand and the
                // transform request, then the window slides out.
                self.phase = Phase::Closing { tick: 0 };
                self.look.play(ScreenSound::Ok);
                return Some(Request::Confirm);
            }
            SlotKind::Soul => {
                // 0x08024972: a soul on offer starts its sequence.
                if here.state != SlotState::Selectable {
                    self.look.play(ScreenSound::Refused);
                    return None;
                }
                self.look.play(ScreenSound::Pick);
                self.phase = Phase::SoulChosen { sub: 0, counter: 0 };
            }
            // A system's button (BN6's: the scrap, `sub_8028E04`; the
            // re-deal, `sub_8028DD6`): its `pressed`, which may start the
            // shared machinery (`custom.sacrifice`, `custom.redeal`).
            SlotKind::Button { button, .. } => extras.button_pressed(self, folder, button),
            SlotKind::Empty | SlotKind::Hidden => {}
        }
        None
    }

    /// The systems' buttons that say (BN6's Beast Out, `sub_8028F48`; its
    /// scrap, `sub_8028F84`), unless used up or picked.
    pub(crate) fn refresh_buttons(&mut self, extras: &mut dyn super::Extras) {
        for s in 0..SLOTS {
            if let SlotKind::Button { button, cell: ButtonCell::Only | ButtonCell::Left } = self.slots[s].kind
                && self.slots[s].state != SlotState::Selected
                && let Some(state) = extras.button_state(self, button)
            {
                self.slots[s].state = state;
            }
        }
    }

    /// `custom.pick`: the slot under the cursor is picked.
    pub fn pick_cursor(&mut self) {
        self.push_selection(self.cursor);
    }

    /// `custom.play`: a screen sound by its name.
    pub fn play_named(&mut self, name: &str) -> bool {
        let sound = match name {
            "pick" => ScreenSound::Pick,
            "refused" => ScreenSound::Refused,
            "back" => ScreenSound::Back,
            "cancel" => ScreenSound::Cancel,
            "beast_out_gregar" => ScreenSound::BeastOut(super::GameVersion::Gregar),
            "beast_out_falzar" => ScreenSound::BeastOut(super::GameVersion::Falzar),
            "beast_out_flash" => ScreenSound::BeastOutFlash,
            "cursor" => ScreenSound::Cursor,
            "description" => ScreenSound::Description,
            "cross_window_open" => ScreenSound::CrossWindowOpen,
            "cross_window_close" => ScreenSound::CrossWindowClose,
            "cross_chosen" => ScreenSound::CrossChosen,
            _ => return false,
        };
        self.look.play(sound);
        true
    }

    /// `custom.set_column_icon`: the last pick's cell of the column shows
    /// `chip` (BN6's Beast Out: the BeastOut chip, `sub_802A034`).
    pub fn set_column_icon(&mut self, chip: Option<ChipHandle>) {
        if self.selected > 0 {
            self.look.column[self.selected as usize - 1] = chip.map(|id| FolderChip { id, code: ChipCode(0) });
        }
    }

    /// `custom.open_window`: `ticks` the ticks it has had already.
    pub fn open_window(&mut self, window: WindowHandle, ticks: u16) {
        self.phase = Phase::Window { window, tick: ticks };
    }

    /// The window's ticks so far (from 1), if one is up.
    pub fn window_tick(&self) -> Option<u16> {
        match self.phase {
            Phase::Window { tick, .. } => Some(tick),
            _ => None,
        }
    }

    /// `custom.pick_first` (`sub_8027796`, `sub_8027672`): the last pick
    /// goes first in the selection, so B takes it back last, and the
    /// column follows, `icon` first (else the first pick's chip).
    pub fn pick_first(&mut self, folder: &BattleFolder, icon: Option<ChipHandle>) {
        let n = self.selected as usize;
        self.selection[..n].rotate_right(1);
        let first = match icon {
            Some(id) => Some(FolderChip { id, code: ChipCode(0) }),
            None => self.chip_in(self.selection[0], folder),
        };
        self.reorder_column(folder, first);
    }

    /// The slot of the button under the cursor (its first cell).
    pub fn cursor_button_slot(&self) -> Option<u8> {
        match self.slots[self.cursor as usize].kind {
            SlotKind::Button { cell: ButtonCell::Right, .. } => Some(self.cursor - 1),
            SlotKind::Button { .. } => Some(self.cursor),
            _ => None,
        }
    }

    /// `custom.refuse`: the refusal sound.
    pub fn refuse(&mut self) {
        self.look.play(ScreenSound::Refused);
    }

    /// `custom.sacrifice` (BN6's scrap, `sub_8028E04`): the pick's sound,
    /// and the picked chips are scrapped for the button `button`.
    pub fn start_sacrifice(&mut self, button: u8) {
        self.look.play(ScreenSound::Pick);
        self.phase = Phase::Scrapping { button, tick: 0, done: false, scrapped: [None; MAX_SELECTIONS], count: 0 };
    }

    /// `custom.redeal` (BN6's ChpShufl, `sub_8028DD6`): the chips not
    /// picked are dealt again, for the button `button`.
    pub fn start_redeal(&mut self, button: u8) {
        let deal = Deal { chips: [None; FOLDER_SIZE], count: 0 };
        self.phase = Phase::Redealing { button, started: false, elapsed: 0, deal };
        self.look.play(ScreenSound::Redeal);
    }

    /// Whether the last pick is a chip (`sub_8028F84`'s test).
    pub fn last_pick_is_chip(&self) -> bool {
        self.selected > 0 && matches!(self.slots[self.selection[self.selected as usize - 1] as usize].kind, SlotKind::Chip { .. })
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
    pub(crate) fn show_chip_window(&mut self, folder: &BattleFolder, view: &PlayerView) {
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

    /// B (`sub_8029032`): take back the last pick; with none, what a
    /// system takes back (BN6's Cross).
    fn deselect(&mut self, view: &PlayerView, folder: &BattleFolder, extras: &mut dyn super::Extras) {
        if self.selected == 0 {
            if !extras.take_back(self, folder) {
                self.look.play(ScreenSound::Refused);
                return;
            }
        } else if self.slots[self.selection[self.selected as usize - 1] as usize].kind == SlotKind::Soul {
            // 0x08024D44: taking the soul back puts the chip given up for
            // it back in its place, and the button is on offer again.
            let last = self.selected as usize - 1;
            self.selection[last] = self.soul.given_up;
            self.look.column[last] = self.chip_in(self.soul.given_up, folder).map(|c| checked(c, view));
            self.slots[SPECIAL_SLOT as usize].state = SlotState::Selectable;
        } else {
            let last = self.selection[self.selected as usize - 1];
            self.selected -= 1;
            self.slots[last as usize].state = SlotState::Selectable;
            self.look.column[self.selected as usize] = None;
            // sub_802A0EC: the side's systems (taking Beast Out, or the
            // BeastOut chip, back takes its face back).
            if let SlotKind::Button { button, .. } = self.slots[last as usize].kind {
                extras.button_taken_back(self, button);
            } else if let Some(c) = self.chip_in(last, folder) {
                extras.chip_taken_back(self, c.id);
            }
        }
        self.update_availability(view, folder, extras);
        self.show_chip_window(folder, view);
        self.look.play(ScreenSound::Back);
    }

    /// R: a description's chatbox (`chatbox_runScript` in the key's
    /// handler), which runs its first tick this tick.
    fn describe(&mut self, joy: &Joypad, lines: u8, window: Option<WindowHandle>, form: Option<nettai_content_api::FormHandle>) {
        let mut chatbox = Chatbox::new(Script::Description { breaks: lines.saturating_sub(1) });
        chatbox.update(joy.held, joy.pressed);
        self.phase = Phase::Description { window, form, chatbox };
    }

    /// `custom.describe`: R in the window up, `form`'s description (`lines`
    /// long), back to the window when it closes. False when no window is
    /// up.
    pub fn describe_form(&mut self, joy: &Joypad, lines: u8, form: Option<nettai_content_api::FormHandle>) -> bool {
        let Phase::Window { window, .. } = self.phase else { return false };
        self.describe(joy, lines, Some(window), form);
        true
    }

    /// DustCross's scrap (`sub_8027406`): every 25 ticks the last picked
    /// chip leaves the folder; then the folder closes up, the scrapped
    /// chips go to its end, and the hand is dealt again.
    fn scrap(&mut self, view: &PlayerView, folder: &mut BattleFolder, extras: &mut dyn super::Extras) {
        let Phase::Scrapping { button, tick, done, mut scrapped, mut count } = self.phase else { unreachable!() };
        if done {
            // sub_802750C
            self.look.play(ScreenSound::ScrapDone);
            let button = &mut self.slots[button as usize];
            button.uses_left = button.uses_left.saturating_sub(1);
            button.state = if button.uses_left == 0 { SlotState::Selected } else { SlotState::Selectable };
            self.phase = Phase::Choosing;
            self.update_availability(view, folder, extras);
            return;
        }
        let tick = tick + 1;
        // The window's frame counter is the scrap's timer, from 24
        // (`sub_8027434`).
        self.look.frame = if tick == 1 { SCRAP_TIMER_START } else { self.look.frame + 1 };
        if tick >= 2 && (tick - 2) % 25 == 0 {
            let last = self.selected.checked_sub(1).map(|i| self.selection[i as usize]);
            match last.map(|s| self.slots[s as usize].kind) {
                Some(SlotKind::Chip { index, regular }) => {
                    // sub_8027458: scrapping the Regular chip ends it
                    // (BattleState+0x17 = 0), as taking it at OK does.
                    if regular {
                        folder.regular_pending = false;
                    }
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
                    self.phase = Phase::Scrapping { button, tick, done: true, scrapped, count };
                    return;
                }
            }
        }
        self.phase = Phase::Scrapping { button, tick, done: false, scrapped, count };
    }

    /// ChpShufl's re-deal (`sub_80271F8`, state 0x28), drawing from the
    /// console's RNG. The first tick (`sub_802721C`) shuffles the chips it
    /// deals again into a new order (`sub_8029788`) and marks the button
    /// in use. Then every 4 ticks (`sub_802723A`) the chips are shown
    /// shuffled once more (`sub_8029688`: shuffled in the folder itself),
    /// until on the 32nd the new order lands (`sub_802983C`), the button
    /// has a use fewer, and the grid takes keys again. The availability is
    /// redone each time.
    fn redeal(&mut self, view: &PlayerView, folder: &mut BattleFolder, console: &mut Console, extras: &mut dyn super::Extras) {
        let Phase::Redealing { button, started, elapsed, mut deal } = self.phase else { unreachable!() };
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
            self.slots[button as usize].state = SlotState::Selected;
            self.update_availability(view, folder, extras);
            self.phase = Phase::Redealing { button, started: true, elapsed: 0, deal };
            // The window's frame counter is the re-deal's (`+0x40`).
            self.look.frame = 0;
            return;
        }
        let elapsed = elapsed + 1;
        self.look.frame = elapsed as u32;
        self.phase = Phase::Redealing { button, started, elapsed, deal };
        if elapsed % REDEAL_STEP != 0 {
            return;
        }
        if elapsed / REDEAL_STEP >= REDEAL_STEPS {
            for (&i, &c) in places.iter().zip(&deal.chips[..deal.count as usize]) {
                folder.chips[i] = c;
            }
            let button = &mut self.slots[button as usize];
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
        self.update_availability(view, folder, extras);
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

    /// `sub_8028E32`: gray out what doesn't go with the selection.
    pub(crate) fn update_availability(&mut self, view: &PlayerView, folder: &BattleFolder, extras: &mut dyn super::Extras) {
        // sub_8028E4C: what the picked chips have in common.
        let mut same_chip = Common::Any;
        let mut code = Common::Any;
        let mut special = None;
        for &s in self.selection() {
            let Some(c) = self.chip_in(s, folder) else { continue };
            let c = checked(c, view);
            // (A chip that goes with any selection constrains none: BN6's
            // BeastOut chip.)
            if view.library.chip(c.id).traits.has(crate::content::ChipTraits::GOES_WITH_ANY) {
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
            } else if view.library.chip(c.id).traits.has(crate::content::ChipTraits::GOES_WITH_ANY) {
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
        self.update_soul(view, folder);
        self.refresh_buttons(extras);
        // sub_8028250: the slots are drawn again.
        self.draw_slots(folder, view);
    }

    /// 0x08024B28: BN5's soul button offers the soul of the last pick's
    /// family: a chip (not the Regular chip) of a family one of the navi's
    /// souls is for, a soul the player has (a dark chip's Chaos Unison
    /// needs the save's Chaos Unison), not used yet this round (Soul Unison
    /// and Chaos Unison apart).
    fn update_soul(&mut self, view: &PlayerView, folder: &BattleFolder) {
        let s = self.slots[SPECIAL_SLOT as usize];
        if s.kind != SlotKind::Soul || s.state == SlotState::Selected {
            return;
        }
        let offer = (|| {
            let last = *self.selection().last()?;
            let SlotKind::Chip { regular: false, .. } = self.slots[last as usize].kind else { return None };
            let chip = checked(self.chip_in(last, folder)?, view);
            let data = view.library.chip(chip.id);
            let chaos = data.flags.has(crate::content::ChipFlags::DARK);
            if chaos && !view.souls.chaos {
                return None;
            }
            let (number, _) = view.library.soul_for_family(view.stats.navi, data.family)?;
            if view.souls.owned & (1 << number) == 0 {
                return None;
            }
            let bit = if chaos { 1u32 << (16 + number) } else { 1 << number };
            if view.round.souls_used & bit != 0 {
                return None;
            }
            Some(SoulButton { number, chaos, given_up: last })
        })();
        let slot = &mut self.slots[SPECIAL_SLOT as usize];
        match offer {
            Some(o) => {
                self.soul = o;
                slot.state = SlotState::Selectable;
            }
            None => {
                self.soul.number = 0;
                self.soul.chaos = false;
                slot.state = SlotState::Unavailable;
            }
        }
    }

    /// BN5's state 9 (0x080232D0): the soul's icon flies to the column (16
    /// ticks, then 8 sliding), the screen flashes (fades 0x34 and 0x30) and
    /// whitens (fade 4); white, the soul goes first in the selection in
    /// place of the chip given up for it, the button is used, and the
    /// screen clears (fade 0); then the picking goes on.
    fn soul_chosen(&mut self, sub: u8, counter: u8, view: &PlayerView, folder: &BattleFolder, extras: &mut dyn super::Extras) {
        let fading = self.look.fade.active();
        let (sub, counter) = match sub {
            // 0x0802330C
            0 => (4, 0),
            // 0x08023360
            4 => {
                let c = counter + 1;
                if c >= 16 { (8, 0) } else { (4, c) }
            }
            // 0x0802337A
            8 => {
                let c = counter + 1;
                if c >= 8 {
                    self.look.fade.start(FadeMode::SoulFlash, SOUL_FADE_SPEED);
                    self.look.play(ScreenSound::ProgramAdvancePart);
                    (12, 0)
                } else {
                    (8, c)
                }
            }
            // 0x080233A8
            12 if !fading => {
                self.look.fade.start(FadeMode::SoulFlashBack, SOUL_FADE_SPEED);
                (16, 0)
            }
            // 0x080233C4
            16 if !fading => {
                self.look.fade.start(FadeMode::EndToWhite, SOUL_FADE_SPEED);
                (20, 0)
            }
            // 0x080233E0
            20 if !fading => {
                let n = self.selected as usize;
                self.selection[..n].rotate_right(1);
                self.selection[0] = SPECIAL_SLOT;
                for j in 1..n {
                    self.look.column[j] = self.chip_in(self.selection[j], folder).map(|c| checked(c, view));
                }
                self.look.column[0] = None;
                self.slots[SPECIAL_SLOT as usize].state = SlotState::Selected;
                self.update_availability(view, folder, extras);
                self.look.fade.start(FadeMode::IntroFromWhite, SOUL_FADE_SPEED);
                self.look.play(ScreenSound::ProgramAdvance);
                (24, 0)
            }
            // 0x08023452
            24 if !fading => {
                self.phase = Phase::Choosing;
                return;
            }
            _ => (sub, counter),
        };
        self.phase = Phase::SoulChosen { sub, counter };
        // (The count is the screen's frame counter, +0x40: the cursor
        // blinks on from where it leaves it.)
        self.look.frame = counter as u32;
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

/// The speed of BN5's soul sequence's fades (`SetScreenFade(m, 32)`).
const SOUL_FADE_SPEED: u8 = 32;

/// The scrap's timer starts at 24, so that its first chip goes on its
/// second tick (`sub_8027434`).
const SCRAP_TIMER_START: u32 = 0x18;

/// The window's offset off the screen, and its slide a tick.
const SLIDE: u32 = 0x78;
const SLIDE_STEP: u32 = 12;

/// The re-deal's steps: every 4 ticks, the 8th lands.
const REDEAL_STEP: u8 = 4;
const REDEAL_STEPS: u8 = 8;

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

/// `sub_802A40C` without a form's share (BN6's: ChargeCross's chips, and
/// NumbrOpn not in DustCross, which its cross system's
/// `custom.hand_size` adds): how many chips a screen deals by the custom
/// level, NumbrOpn and the hand-shrink bug, when the side's rules don't
/// say.
fn hand_size(view: &PlayerView, turn: u8) -> u8 {
    let s = view.stats;
    let mut extra: i16 = 0;
    let mut n = s.custom_level as i16;
    if n > 8 {
        extra = n - 8;
        n = 8;
    }
    if s.number_open {
        n = 10;
        extra = 0;
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


    /// `sub_802A57E`: the navi can Beast Out now: not worn out, not in a
    /// Beast form. Tired, it can only once it has this round (and then
    /// goes Beast Over).
    /// 0x08023C54: BN5's soul button: a navi with souls, outside battle
    /// flag 0x40, with the save's Soul Unison (event flag 0), unless his
    /// BN5 emotion is worried or dark (0x08012740: in a soul or angry he
    /// may; at mood 0 he is dark, below 65 worried, at 0xFF Full Synchro).
    fn soul_button(&self) -> bool {
        let in_soul = self.library.form_kind(self.stats.form) == crate::content::FormKind::Soul;
        let mood = self.stats.mood;
        let hidden = !in_soul && self.emotion != Emotion::Angry && (mood == 0 || (mood != 0xFF && mood < 65));
        self.library.has_souls(self.stats.navi) && self.souls.button && !self.per_player_gauges && !hidden
    }

}
