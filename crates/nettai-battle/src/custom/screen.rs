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
/// The entries a form list's window shows at most (EXE6's Cross window:
/// a version's five Crosses).
pub const FORM_LIST_ENTRIES: usize = 5;

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
    /// A cell of a button of the rules (docs/design/rules-in-luau.md §4.4:
    /// EXE6's ChpShufl re-deal and DustCross scrap, two cells wide on slots
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

/// Where a button of the rules sits on a screen, as the screen's extras say
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
    /// The chip it shows (its `chip`: EXE5's capsules), if it has one.
    pub chip: Option<ChipHandle>,
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
    /// A picked chip: the hand's modifier bits a button mixed into it
    /// (`custom.attach_to_last_pick`: EXE5's capsules, the slot's +4's bits
    /// 0x3E), and that button's slot (its +5). B on the chip clears them and
    /// frees the button.
    pub marks: u8,
    pub attached: Option<u8>,
    /// A button: the chip it shows (EXE5's capsules, the chip its slot's +8
    /// points at), which the chip window shows and R describes.
    pub face: Option<ChipHandle>,
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
    /// R shows the chip's description (`sub_8026E4C`), or a window of the rules
    /// a form's (EXE6's Cross window: the Cross's, `sub_8026E78`): `window`
    /// the window it returns to, `form` the form described. The screen
    /// waits for its chatbox to close.
    Description { window: Option<WindowHandle>, form: Option<nettai_content_api::FormHandle>, chatbox: Chatbox },
    /// L: the "no time to run" message (`sub_8026E98`), whose chatbox
    /// starts on the state's first tick (`sub_8026EC8`) and is waited for
    /// from the next (`sub_8026FAA`).
    RunMessage { chatbox: Option<Chatbox> },
    /// A window of the rules is up (docs/design/rules-in-luau.md §4.4: EXE6's
    /// Beast Out, `sub_802770C`, and the Cross window): its `update` runs
    /// each tick, `tick` from 1, until it says it is done.
    Window { window: WindowHandle, tick: u16 },
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
    /// The form a pick puts the navi in at the turn's start (EXE6's Beast
    /// Out, `+0x17`), and which of the rules' buttons or windows made it (a
    /// button's place, 0x100 past a window's).
    pub form: Option<nettai_content_api::FormHandle>,
    pub form_owner: Option<u16>,
    /// The transform record's turns and alternate flag beside the form
    /// (EXE5's Soul Unison: +3 and +1, Chaos Unison), as the rules that set
    /// the form say.
    pub form_turns: u8,
    pub form_alternate: bool,
    /// A button of the rules picked in the place of the chip given up for it
    /// (EXE5's soul button, 0x080233E0): B on it puts the chip back, and at
    /// OK the chip leaves the folder where the button stands.
    pub trade: Option<Trade>,
    /// A button of the rules holding a chip taken out of the picks
    /// (`custom.hold_last_pick`: EXE5's Arm Change, 0x080236C0): B, with the
    /// picks as they were, puts it back; at OK the chip leaves the folder.
    pub hold: Option<Hold>,
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

/// A button picked in a chip's place (`custom.trade_last_pick`): the
/// button's slot, and the slot of the chip given up for it (EXE5's soul
/// button: slot 11's +4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Trade {
    pub button: u8,
    pub chip: u8,
}

/// A chip a button holds (`custom.hold_last_pick`): the button's slot, the
/// chip's slot (which stays picked), and how many picks there were without
/// it (EXE5's Arm Change: slot 8's +4 and +5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hold {
    pub button: u8,
    pub chip: u8,
    pub at: u8,
}

/// The last pick, as `custom.last_pick` gives it: its slot, its chip as the
/// screen checked it, whether it is the folder's Regular chip or a link
/// navi's own chip, and whether a button is attached to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LastPick {
    pub slot: u8,
    pub chip: FolderChip,
    pub regular: bool,
    pub navi_chip: bool,
    pub attached: bool,
}

/// What the screen reads of its player when it opens and while it runs.
#[derive(Clone, Copy)]
pub struct PlayerView<'a> {
    pub library: &'a dyn Library,
    pub stats: &'a NaviStats,
    pub emotion: Emotion,
    /// Chips sent this round, by class (`dword_20367E0`).
    pub class_uses: &'a ClassCounts,
    /// The round's Beast Out and Crosses so far.
    pub round: &'a RoundMemory,
    /// The folder's Regular chip hasn't been used yet.
    pub regular_pending: bool,
    /// Battle flag 0x40 (the own-gauges mode; never in a netbattle without chip gates).
    pub own_gauges: bool,
    /// Battle effects 0x200000 (random battles).
    pub random_battle: bool,
    /// A netbattle's last turns (presentation: the window's block).
    pub late_turns: bool,
}

/// What a player's screens remember through a round (`dword_20349A0`,
/// cleared on the round's first screen).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RoundMemory {
    /// The link navis whose own chips were put in a hand this round (a bit
    /// each: the original's by the chip's place among them).
    pub navi_chips_used: u32,
}

impl Screen {
    /// The screen a player gets when the custom screen opens (`sub_8026840`):
    /// compact the folder, deal, and lay out the slots. `turn`: the
    /// screen's number in the round (1 = first).
    pub fn open(
        folder: &mut BattleFolder,
        view: &PlayerView,
        turn: u8,
        console: &mut crate::console::Console,
        extras: &mut dyn super::Extras,
    ) -> Screen {
        let megaman = view.megaman();
        let mut screen = Screen {
            phase: Phase::Opening { tick: 0 },
            slots: [Slot {
                kind: SlotKind::Hidden,
                vertical: None,
                left: None,
                right: None,
                state: SlotState::Selectable,
                uses_left: 0,
                marks: 0,
                attached: None,
                face: None,
            }; SLOTS],
            cursor: 0,
            selection: [0; MAX_SELECTIONS],
            selected: 0,
            chips_left: 0,
            hand_size: hand_size(view, turn),
            megaman,
            form: None,
            form_owner: None,
            form_turns: 0,
            form_alternate: false,
            trade: None,
            hold: None,
            program_advance: None,
            hud: Banner::default(),
            look: ScreenLook::new(view.late_turns, false, None),
        };
        // The side's rules as the screen deals, on the folder as the last
        // screen left it and the framework's hand size (EXE5's opening,
        // 0x08022C5C: its dark chip offered, 0x08025114, after the hand
        // size, 0x08025BE4, before the folder closes up, 0x080250E6; the
        // hand size its rules give, NumberSoul's ten, comes after here,
        // which no deal sees: a MegaMan in a soul is dealt no dark chip).
        extras.dealing(&mut screen, folder, console);
        folder.compact();
        screen.chips_left = folder.count() as u8;
        // The side's rules as the screen opens (EXE6's: the round's
        // Beast Out and Crosses forgotten on its first screen, ChargeCross's
        // screens, the Crosses offered and the window's Cross tab).
        extras.opened(&mut screen);
        // sub_802A40C: the side's rules' hand size (EXE6's rules/cross's,
        // with ChargeCross's chips), else the framework's.
        screen.hand_size = extras.hand_size().unwrap_or_else(|| hand_size(view, turn));
        screen.lay_out(view, extras);
        // EXE5's offer of a team navi's own chip (0x08023EFE) picks between
        // its table's two entries for the navi, the same chip, with a draw
        // of the console's RNG.
        if matches!(screen.slots[9].kind, SlotKind::NaviChip(_)) && view.library.navi_chip_draws(view.stats.navi) {
            console.rng.next();
        }
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
                marks: 0,
                attached: None,
                face: None,
            };
        }
        let dealt = self.chips_left.min(self.hand_size);
        for i in 0..dealt {
            self.slots[i as usize].kind = SlotKind::Chip { index: i, regular: i == 0 && view.regular_pending };
        }
        // The side's rules' buttons (EXE6's: DustCross's scrap,
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
                    face: place.chip,
                    ..d
                };
            } else {
                let d = self.slots[first];
                self.slots[first] = Slot {
                    kind: SlotKind::Button { button: place.button, cell: ButtonCell::Left },
                    right: place.right.or(d.right),
                    state,
                    uses_left: place.uses,
                    face: place.chip,
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
        let on_shading = self.on_shading_chip(view, folder);
        self.look.hover(on_shading);
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
    fn on_shading_chip(&self, view: &PlayerView, folder: &BattleFolder) -> bool {
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
                // (EXE5's 0x08023012: the chip a button holds, over it.)
                self.look.draw_held(self.hold.is_some());
                self.look.draw_turn_limit();
                self.look.frame += 1;
                request
            }
            Phase::Hidden { stage } => {
                // sub_8026D06: hiding takes the last turns' block off;
                // coming back draws the emblem on that tick and the next,
                // where the game's screen does (the layout's
                // `emblem_at_window_return`: EXE5's 0x08023022 draws it on
                // neither).
                let emblem = view.library.layout().emblem_at_window_return;
                self.phase = match stage {
                    HiddenStage::Hiding => {
                        self.look.turn_limit = false;
                        self.look.play(ScreenSound::Hide);
                        Phase::Hidden { stage: HiddenStage::Waiting }
                    }
                    HiddenStage::Waiting if joy.pressed != 0 => {
                        if emblem {
                            self.look.draw_emblem(0);
                        }
                        self.look.play(ScreenSound::Hide);
                        Phase::Hidden { stage: HiddenStage::Restoring }
                    }
                    HiddenStage::Waiting => self.phase,
                    HiddenStage::Restoring => {
                        if emblem {
                            self.look.draw_emblem(0);
                        }
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
                            .commands_wait_for_text(view.library.layout().chatbox_commands_wait_for_text)
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
                // The rules' window (EXE6's Beast Out, `sub_802770C`): its
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
        // The side's rules first (EXE6's UP opening the Cross window, which
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
            } else if let Some(c) = self.slots[self.cursor as usize].face {
                // A button that shows a chip (EXE5's capsules, 0x0802487C: the
                // slot's kinds 6 and 7): the chip's description.
                let lines = view.library.chip(c).description_lines;
                self.describe(joy, lines, None, None);
                self.look.play(ScreenSound::Description);
            }
        } else if p & keys::L != 0 && view.library.run_message(view.stats.navi)[0] != 0 {
            // (The navi's message: every navi of the originals has one. A
            // navi whose content states none has no box to open; the
            // frontend's audit lists it.)
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
                // sub_802A00C: the side's rules (EXE6's BeastOut chip starts
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
            // A button of the rules (EXE6's: the scrap, `sub_8028E04`; the
            // re-deal, `sub_8028DD6`): its `pressed`, which may start the
            // shared machinery (`custom.sacrifice`, `custom.redeal`).
            SlotKind::Button { button, .. } => extras.button_pressed(self, folder, button),
            SlotKind::Empty | SlotKind::Hidden => {}
        }
        None
    }

    /// The rules' buttons that say (EXE6's Beast Out, `sub_8028F48`; its
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
            "cursor" => ScreenSound::Cursor,
            "description" => ScreenSound::Description,
            "program_advance_part" => ScreenSound::ProgramAdvancePart,
            "program_advance" => ScreenSound::ProgramAdvance,
            "redeal" => ScreenSound::Redeal,
            _ => return false,
        };
        self.look.play(sound);
        true
    }

    /// `custom.set_column_icon`: the last pick's cell of the column shows
    /// `chip` (EXE6's Beast Out: the BeastOut chip, `sub_802A034`).
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

    /// `custom.sacrifice` (EXE6's scrap, `sub_8028E04`): the pick's sound,
    /// and the picked chips are scrapped for the button `button`.
    pub fn start_sacrifice(&mut self, button: u8) {
        self.look.play(ScreenSound::Pick);
        self.phase = Phase::Scrapping { button, tick: 0, done: false, scrapped: [None; MAX_SELECTIONS], count: 0 };
    }

    /// `custom.redeal` (EXE6's ChpShufl, `sub_8028DD6`; EXE5's SearchSoul's
    /// Shuffle, 0x080249B0): the chips not picked are dealt again, for the
    /// button `button`. (EXE6's plays a sound as it starts, which its
    /// button's content plays: EXE5's plays none.)
    pub fn start_redeal(&mut self, button: u8) {
        let deal = Deal { chips: [None; FOLDER_SIZE], count: 0 };
        self.phase = Phase::Redealing { button, started: false, elapsed: 0, deal };
    }

    /// Whether the last pick is a chip (`sub_8028F84`'s test).
    pub fn last_pick_is_chip(&self) -> bool {
        self.selected > 0 && matches!(self.slots[self.selection[self.selected as usize - 1] as usize].kind, SlotKind::Chip { .. })
    }

    /// `custom.last_pick`: the last pick, if it is a chip (one dealt from
    /// the folder or a link navi's own), as the screen checked it.
    pub fn last_pick(&self, folder: &BattleFolder, view: &PlayerView) -> Option<LastPick> {
        let slot = *self.selection().last()?;
        let (regular, navi_chip) = match self.slots[slot as usize].kind {
            SlotKind::Chip { regular, .. } => (regular, false),
            SlotKind::NaviChip(_) => (false, true),
            _ => return None,
        };
        let chip = checked(self.chip_in(slot, folder)?, view);
        Some(LastPick { slot, chip, regular, navi_chip, attached: self.slots[slot as usize].attached.is_some() })
    }

    /// `custom.attach_to_last_pick` (EXE5's capsules, 0x080237B4): the button
    /// in slot `button` is used on the last pick, a chip with no button
    /// attached yet: the chip's pick carries `modifiers` (the hand's
    /// modifier bits, past the Regular chip's) into the hand, and the button
    /// is picked until B takes the chip back. False when the last pick
    /// isn't such a chip.
    pub fn attach_to_last_pick(&mut self, button: u8, modifiers: u8, folder: &BattleFolder, view: &PlayerView) -> bool {
        let Some(last) = self.last_pick(folder, view).filter(|l| !l.attached) else { return false };
        let slot = &mut self.slots[last.slot as usize];
        slot.marks |= modifiers & !super::builder::modifier_bits::REGULAR;
        slot.attached = Some(button);
        self.slots[button as usize].state = SlotState::Selected;
        true
    }

    /// `custom.hold_last_pick` (EXE5's Arm Change, 0x080236C0): the last
    /// pick, a chip dealt from the folder, leaves the picks for the button
    /// in slot `button`, which is picked; its slot stays picked. False when
    /// the last pick isn't such a chip, or the button holds one already.
    pub fn hold_last_pick(&mut self, button: u8, folder: &BattleFolder, view: &PlayerView) -> bool {
        if self.hold.is_some() {
            return false;
        }
        let Some(last) = self.last_pick(folder, view).filter(|l| !l.navi_chip) else { return false };
        self.selected -= 1;
        self.hold = Some(Hold { button, chip: last.slot, at: self.selected });
        self.look.column_kept = Some(self.selected);
        self.slots[button as usize].state = SlotState::Selected;
        true
    }

    /// The chip the button in slot `button` holds, as the screen checked it.
    pub fn held_pick(&self, button: u8, folder: &BattleFolder, view: &PlayerView) -> Option<FolderChip> {
        let h = self.hold.filter(|h| h.button == button)?;
        self.chip_in(h.chip, folder).map(|c| checked(c, view))
    }

    /// `custom.set_held_icon` (EXE5's Arm Change's blink, 0x080236EC): the
    /// held chip's icon in the column cell it left, shown or not.
    pub fn set_held_icon(&mut self, button: u8, shown: bool, folder: &BattleFolder, view: &PlayerView) -> bool {
        let Some(h) = self.hold.filter(|h| h.button == button) else { return false };
        self.look.column[h.at as usize] = if shown { self.chip_in(h.chip, folder).map(|c| checked(c, view)) } else { None };
        // (0x08023712: the cell's frame drawn empty.)
        self.look.column_kept = None;
        true
    }

    /// `custom.trade_last_pick` (EXE5's soul, 0x080233E0): the button in slot
    /// `button` takes the last pick's place, first in the selection (the
    /// chip's slot stays picked); the column follows, the button's cell
    /// first (drawn by the button's own look). False when the last pick
    /// isn't a chip.
    pub fn trade_last_pick(&mut self, button: u8, folder: &BattleFolder, view: &PlayerView) -> bool {
        let Some(last) = self.last_pick(folder, view) else { return false };
        let n = self.selected as usize;
        self.selection[..n].rotate_right(1);
        self.selection[0] = button;
        for j in 1..n {
            self.look.column[j] = self.chip_in(self.selection[j], folder).map(|c| checked(c, view));
        }
        self.look.column[0] = None;
        self.trade = Some(Trade { button, chip: last.slot });
        true
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
        let here = self.slots[self.cursor as usize];
        let w = &mut self.look.chip_window;
        w.slot = self.cursor;
        w.picks = self.selected;
        if chip.is_some() {
            w.last_chip = chip;
        }
        // The frame's colors: a chip's by its class (`sub_80284E2`), the
        // standard ones for OK and a button's picture (`sub_80287D2`); a
        // button that shows a chip sets none (EXE5's 0x08024422), and an
        // empty or hidden slot draws nothing.
        match here.kind {
            SlotKind::Chip { .. } | SlotKind::NaviChip(_) if chip.is_some() => w.framed = chip,
            SlotKind::Ok => w.framed = None,
            SlotKind::Button { .. } if here.face.is_none() => w.framed = None,
            _ => {}
        }
    }

    fn push_selection(&mut self, slot: u8) {
        self.selection[self.selected as usize] = slot;
        self.selected += 1;
    }

    /// B (`sub_8029032`): take back the last pick; with none, what the
    /// rules take back (EXE6's Cross).
    fn deselect(&mut self, view: &PlayerView, folder: &BattleFolder, extras: &mut dyn super::Extras) {
        if let Some(h) = self.hold.filter(|h| h.at == self.selected) {
            // A button's held chip, with the picks as they were when it
            // took it (EXE5's Arm Change, 0x08024CFC: before anything else,
            // with no picks too): the chip is the last pick again, and the
            // button selectable.
            self.selection[h.at as usize] = h.chip;
            self.selected += 1;
            self.look.column_kept = None;
            self.look.column[h.at as usize] = self.chip_in(h.chip, folder).map(|c| checked(c, view));
            self.slots[h.button as usize].state = SlotState::Selectable;
            self.hold = None;
            if let SlotKind::Button { button, .. } = self.slots[h.button as usize].kind {
                extras.button_taken_back(self, button);
            }
        } else if self.selected == 0 {
            if !extras.take_back(self, folder) {
                self.look.play(ScreenSound::Refused);
                return;
            }
        } else if let Some(t) = self.trade.filter(|t| t.button == self.selection[self.selected as usize - 1]) {
            // A button picked in a chip's place (EXE5's soul, 0x08024D44):
            // the chip given up for it goes back in its place, and the
            // button is selectable again.
            let last = self.selected as usize - 1;
            self.selection[last] = t.chip;
            self.look.column[last] = self.chip_in(t.chip, folder).map(|c| checked(c, view));
            self.slots[t.button as usize].state = SlotState::Selectable;
            self.trade = None;
            if let SlotKind::Button { button, .. } = self.slots[t.button as usize].kind {
                extras.button_taken_back(self, button);
            }
        } else {
            let last = self.selection[self.selected as usize - 1];
            self.selected -= 1;
            self.slots[last as usize].state = SlotState::Selectable;
            self.look.column[self.selected as usize] = None;
            // A button attached to the chip (EXE5's capsule, 0x08024D78): the
            // chip's marks go, and the button is selectable again.
            if let Some(button) = self.slots[last as usize].attached.take() {
                self.slots[last as usize].marks = 0;
                self.slots[button as usize].state = SlotState::Selectable;
            }
            // sub_802A0EC: the side's rules (taking Beast Out, or the
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
            // How many of the hand's chips stay in the hand, by how many of
            // them are dealt again (`byte_80298C8`, the custom screen's
            // `redeal_kept`: EXE6's all zeros, EXE5's 0x080254C8): with some
            // kept, the hand's chips are shuffled first, and then all but
            // that many at the front; with none, one shuffle of them all.
            let hand = self.redeal_hand_places();
            let kept = view.library.layout().redeal_kept.get(hand).copied().unwrap_or(0) as usize;
            if n != 0 {
                if kept != 0 {
                    shuffle(&mut deal.chips[..hand], hand, &mut console.rng);
                }
                shuffle(&mut deal.chips[kept..n], n - kept, &mut console.rng);
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
    /// How many of [`Screen::redeal_places`] are the hand's (its first
    /// walk's: the chip slots not picked, the Regular chip apart).
    fn redeal_hand_places(&self) -> usize {
        self.slots[..(self.hand_size as usize).min(SLOTS)]
            .iter()
            .filter(|slot| matches!(slot.kind, SlotKind::Chip { regular: false, .. }) && slot.state != SlotState::Selected)
            .count()
    }

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
            // (A chip that goes with any selection constrains none: EXE6's
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
        self.refresh_buttons(extras);
        // sub_8028250: the slots are drawn again.
        self.draw_slots(folder, view);
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

/// `sub_802A40C`'s framework part, when the side's rules don't say (a
/// game's own, ChargeCross's chips and NumbrOpn's ten, is its rules'
/// `custom.hand_size`: EXE6's rules/cross): how many chips a screen deals
/// by the custom level and the hand-shrink bug.
fn hand_size(view: &PlayerView, turn: u8) -> u8 {
    let s = view.stats;
    let mut extra: i16 = 0;
    let mut n = s.custom_level as i16;
    if n > 8 {
        extra = n - 8;
        n = 8;
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
}
