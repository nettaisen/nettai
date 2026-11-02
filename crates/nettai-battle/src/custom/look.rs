//! What a player's custom screen shows that the simulation never reads
//! (presentation; the state digest leaves it out, like `Look`): what the
//! screen's decisions leave drawn ([`ScreenLook`]: the picked column, the
//! slots as last drawn, the face, the Program Advance's colors), and what
//! each tick asks of the window's drawing ([`Draw`]) and of the sound
//! driver. The window's counters, the cursor, the chip window and the dark
//! chips' hover are its player's screen layer's (`super::layer`, outside
//! the simulation: [`WindowLook`], [`ChipWindow`], [`Hover`]), which draws
//! by those asks and by what its own player does. A frontend draws the
//! screen from both (nettai-render `custom`). See
//! docs/engine/custom-screen.md §9 and docs/design/custom-screen-actions.md
//! §4.

use crate::battle::{Fade, FadeMode};
use crate::SoundId;
use crate::content::{HoverRules, HoverSound, SoundRole};

/// The original's presentation state of a screen (the control block at
/// `0x020364C0`), as far as the screen's decisions make it: the layer
/// (`super::layer`) keeps the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenLook {
    /// `+0x40` as the screen's own states and its rules' windows set it
    /// (`custom.frame`): a window's own timer. What is drawn by it (the
    /// cursor's blink, the Regular chip's frame, a slide's offset) is the
    /// layer's, which counts it through the choosing as the original does
    /// (`WindowLook::frame`), by the tick's [`Draw`]s.
    pub frame: u32,
    /// What this tick asked of the window's drawing and of the sound
    /// driver.
    pub drawn: Drawn,
    /// The window has the form list's tab (EXE6's Cross tab: MegaMan, with
    /// a Cross he owns and hasn't used this round, `sub_8029EC8`).
    pub form_list_tab: bool,
    /// The picked column's icons, as the screen copied them (`sub_80281D4`:
    /// each pick's chip as checked, Beast Out's the BeastOut chip's; a
    /// Beast Out puts the picks back in their new order, unchecked).
    pub column: [Option<super::FolderChip>; 5],
    /// A column cell past the picks whose frame is still drawn filled: the
    /// cell a button's held chip left, on the tick it leaves (EXE5's Arm
    /// Change, 0x080236C0, takes the pick without drawing; its blink draws
    /// the cell empty from the next tick, 0x08023712).
    pub column_kept: Option<u8>,
    /// The chips the slots' tiles show, as checked when the screen last
    /// drew them (`sub_8028250`, on opening and after every pick or take
    /// back: the chips OK takes out of the folder stay drawn).
    pub slot_chips: [Option<super::FolderChip>; 12],
    /// And which of them were picked then (their tiles show the empty
    /// icon until the slots are drawn again).
    pub slot_picked: [bool; 12],
    /// The form whose face the emotion window shows while the screen is up:
    /// the Beast Out or Cross chosen (`sub_802A040`, `sub_802A088`;
    /// `sub_802A0EC` takes it back).
    pub face: Option<nettai_content_api::FormHandle>,
    /// The battle's last turns have come (`sub_800A97A`).
    pub late_turns: bool,
    /// The Program Advance animation's counter (`word_2036660`+0xC: every
    /// tick, from 0 when the names begin), and which of its three color
    /// sets background palette 10's first colors have (`byte_802BA48`).
    pub pa_ticks: u32,
    pub pa_palette: u8,
}

/// What a tick asks of the window's drawing, in the order it asks: the
/// layer (`super::layer`) does it on the window's counters and sprites
/// ([`WindowLook`]) as the original's routines did, and on its chip window
/// and hover. A choosing tick asks only what its decision changed; the
/// choosing's own drawing (and its sub-screens', the layer's) is the
/// layer's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Draw {
    /// `+0x40` set: a slide's offset, a sub-screen's timer.
    SetFrame(u32),
    /// `+0x40` counted on, a tick.
    CountFrame,
    /// The emblem's spin starts (a pick, `sub_8028CCC`; Beast Out,
    /// `sub_8027738`).
    Spin,
    /// `sub_8029C08`: the emblem at the window's offset `x`.
    Emblem(u32),
    /// `sub_802899C`: the Regular chip's frame, while the folder still has
    /// its Regular chip (`pending`); `counted`: the tick's count is done
    /// ([`WindowLook::draw_regular`]).
    Regular { pending: bool, counted: bool },
    /// `sub_8029D34`: the last turns' block, by the frame.
    TurnLimit,
    /// `sub_8029D80`: the block taken off.
    TurnLimitOff,
    /// `sub_8028820`: the cursor, by the frame.
    Cursor,
    /// The chip window's element icon where the cursor rests on a chip
    /// (EXE4's 0x0801EECC under its chips' state's own test).
    ElementOnChip,
    /// EXE4's 0x0801EECC: the chip window's element icon as a sprite.
    Element,
    /// The chip a button holds, over the button (EXE5's 0x080254F4), or
    /// not.
    Held(bool),
    /// `sub_8028476`: the chip window drawn again, for the slot under the
    /// cursor, as the screen stands at the tick's end.
    ChipWindow,
    /// The screen's fade (its first record) started: what the layer draws
    /// of its first record is the fade started last, this or its hover's.
    FadeStarted,
    /// `sub_80062D0`, `sub_80062D6`: both fade records cleared (EXE4's OK,
    /// a window cancelled by the time limit): the hover's too.
    FadesCleared,
    /// The hover over a dark chip ends at once (the time limit: where it
    /// would not run again, `Screen::tick_time_up`).
    HoverCleared,
    /// The slots were laid out again (EXE5 DS's switch, `sub_8027E2C`): the
    /// cursor goes to the first slot there.
    LaidOut,
}

/// The counters and sprites of the window as the original draws them
/// (the control block's `+0x40`, `+0xF` and the sprites a tick queues),
/// which the player's screen layer keeps (`super::layer`): from the
/// screen's [`Draw`]s, and through the choosing and its sub-screens by its
/// own drawing, as the original's states drew.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowLook {
    /// `+0x40`: while the window slides in or out its offset (0x78 off the
    /// screen, 0 in place); while the chips are chosen a frame counter, which
    /// the cursor and the Regular chip's frame blink by; the sub-screens'
    /// own timers otherwise.
    pub frame: u32,
    /// `+0xF`: the emblem's spin after a pick (0 at rest, else its step,
    /// up to 0x14).
    pub spin: u8,
    /// The emblem's affine matrix as last set (`sub_802FE7A`): its angle
    /// and scale (`byte_8029CAC`).
    pub emblem_matrix: (u8, u8),
    /// The late turns' block is on the window (`sub_8029D34` draws it,
    /// `sub_8029D80` takes it off).
    pub turn_limit: bool,
    /// The Regular chip's frame the sprite's tiles hold (`sub_802899C`
    /// copies one every 8 frames).
    pub regular_frame: u8,
    /// What this tick drew.
    pub drawn: Shown,
}

/// The sprites a tick of the screen queued.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    /// The cursor (`sub_8028820`), in its first or second frame.
    pub cursor: Option<u8>,
    /// The form list window's cursor (EXE6's Cross window's, `sub_80289E4`),
    /// in its first or second frame.
    pub form_list_cursor: Option<u8>,
    /// The emblem (`sub_8029C08`): the window's offset it was drawn at, and
    /// the spin it was drawn with.
    pub emblem: Option<(u32, u8)>,
    /// The Regular chip's frame (`sub_802899C`).
    pub regular: bool,
    /// The chip window's element icon where it is a sprite (EXE4's
    /// 0x0801EECC, which each state calls under its own test; a screen whose
    /// window's map holds the icon draws none).
    pub element: bool,
    /// The icon of the chip a button holds, over the button (EXE5's Arm
    /// Change, 0x080254F4).
    pub held: bool,
}

/// The shade the cursor's hover over a dark chip casts on the screen
/// (`sub_802A2B0`, `+0x12`), with the step of its volume ramp (`+0x13`, a
/// halfword's offset in the original).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipShade {
    /// 0: not on a dark chip.
    Clear,
    /// 4: the fades run toward the shade.
    Shading { step: u8 },
    /// 8: shaded.
    Shaded,
    /// 0xC: the fades run back.
    Clearing { step: u8 },
}


/// What the chip window was last drawn for (`sub_8028476`): the slot
/// under the cursor and how many picks there were then (OK's picture
/// shows whether there are any), and the last chip it showed, whose
/// element's colors palette 11 keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChipWindow {
    pub slot: u8,
    pub picks: u8,
    pub last_chip: Option<super::FolderChip>,
    /// The chip whose class colors the window's frame (palette 9): the
    /// last chip slot shown, or none (the standard colors) once OK or a
    /// button's picture was. A button that shows a chip (`Slot::face`)
    /// leaves the frame as it was (EXE5's capsules, 0x08024422).
    pub framed: Option<super::FolderChip>,
    /// A button's uses left as the window was drawn (EXE4's SHUFFLE,
    /// 0x0801FEA2: the digit stays through a shuffle, which took a use at
    /// its start and draws the window again at its end).
    pub uses: u8,
}

/// What a tick of the screen asked: of the window's drawing (the layer
/// draws it), and of the sound driver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drawn {
    /// What it asked of the drawing, in order.
    pub draws: [Option<Draw>; 16],
    /// What it asked of the sound driver, in the order it asked (its player
    /// hears it): the sounds of the tick's state. (The dark chip hover's,
    /// whose routine runs after the state's, are the layer's.)
    pub calls: [Option<ScreenCall>; 8],
}

/// What a tick of the screen asks of the sound driver, which only its own
/// player hears.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenCall {
    /// A sound (`PlaySoundEffect`).
    Sound(ScreenSound),
    /// The volumes a dark chip's hover sets (`sub_802A30C`, `sub_802A362`:
    /// volume control on the music's player, 31, and the screen's, 22).
    Volume { music: u16, screen: u16 },
}

impl ScreenSound {
    /// The rules' role for it; none for a sound the rules give themselves.
    pub fn role(self) -> Option<SoundRole> {
        Some(match self {
            ScreenSound::Open => SoundRole::CustomOpen,
            ScreenSound::Cursor => SoundRole::CustomCursor,
            ScreenSound::Hide => SoundRole::CustomHide,
            ScreenSound::Shade => SoundRole::CustomShade,
            ScreenSound::Pick => SoundRole::CustomPick,
            ScreenSound::Ok => SoundRole::CustomOk,
            ScreenSound::Back => SoundRole::CustomBack,
            ScreenSound::Refused => SoundRole::Refused,
            ScreenSound::RunMessage => SoundRole::CustomRunMessage,
            ScreenSound::Description => SoundRole::CustomDescription,
            ScreenSound::DescriptionClose => SoundRole::CustomDescriptionClose,
            ScreenSound::Cancel => SoundRole::CustomCancel,
            ScreenSound::Redeal => SoundRole::CustomRedeal,
            ScreenSound::RedealShuffle => SoundRole::CustomRedealShuffle,
            ScreenSound::Scrap => SoundRole::CustomScrap,
            ScreenSound::ScrapDone => SoundRole::CustomScrapDone,
            ScreenSound::ProgramAdvancePart => SoundRole::ProgramAdvancePart,
            ScreenSound::ProgramAdvance => SoundRole::ProgramAdvance,
            ScreenSound::NaviSwitch => SoundRole::NaviSwitch,
            ScreenSound::Rules(_) => return None,
        })
    }
}

impl Drawn {
    /// What the tick asked of the window's drawing, in order.
    pub fn draws(&self) -> impl Iterator<Item = Draw> + '_ {
        self.draws.iter().flatten().copied()
    }

    /// What the tick asked of the sound driver, in order.
    pub fn calls(&self) -> impl Iterator<Item = ScreenCall> + '_ {
        self.calls.iter().flatten().copied()
    }

    /// The sounds the tick made, in order.
    pub fn sounds(&self) -> impl Iterator<Item = ScreenSound> + '_ {
        self.calls().filter_map(|c| match c {
            ScreenCall::Sound(s) => Some(s),
            ScreenCall::Volume { .. } => None,
        })
    }

    /// The volumes the tick set (the music's, the screen's player's), if
    /// it set them.
    pub fn volume(&self) -> Option<(u16, u16)> {
        self.calls().find_map(|c| match c {
            ScreenCall::Volume { music, screen } => Some((music, screen)),
            ScreenCall::Sound(_) => None,
        })
    }
}

/// A sound the screen makes (`PlaySoundEffect` in its states), which only
/// its own player hears. The battle plays each by its role
/// (`SoundRole::Custom*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenSound {
    /// The window starts sliding in (`sub_8026B04`).
    Open,
    /// The cursor moves to another slot (`sub_8028B74`), or in a rules'
    /// window.
    Cursor,
    /// SELECT hides the window, and a key brings it back (`sub_8026D06`).
    Hide,
    /// The hover over a dark chip, every 64 ticks while the screen isn't
    /// clear of its shade (EXE5's 0x08025AA2; a game without the role plays
    /// none).
    Shade,
    /// A chip or a button's pick (EXE6's Beast Out, the scrap, a Cross).
    Pick,
    /// OK (`sub_8028D3A`).
    Ok,
    /// A pick taken back (`sub_8029032`).
    Back,
    /// What can't be picked or taken back.
    Refused,
    /// L: the no-running message (`sub_8026EC8`).
    RunMessage,
    /// R: a description opens, and closes (`sub_8026E4C`).
    Description,
    DescriptionClose,
    /// A button's pick taken back (EXE6's Beast Out or Cross).
    Cancel,
    /// ChpShufl's re-deal pressed, and each of its shuffles
    /// (`sub_802723A`).
    Redeal,
    RedealShuffle,
    /// DustCross scraps a chip (`sub_8027458`), and is done (`sub_802750C`).
    Scrap,
    ScrapDone,
    /// The Program Advance animation names a chip of the recipe
    /// (`sub_802B80C`), and the Program Advance (`sub_802B920`).
    ProgramAdvancePart,
    ProgramAdvance,
    /// A switch to another member of the party is taken (EXE5 DS's 0x8D,
    /// 0x0206391C).
    NaviSwitch,
    /// A sound of the rules' own (`custom.play_sound`): a window's or a
    /// button's (EXE6's Cross window's, Beast Out's).
    Rules(SoundId),
}

/// The emblem's spin (`byte_8029CAC`): per step, the angle and the scale
/// `sub_802FE7A` sets.
const SPIN: [(u8, u8); 19] = [
    (0x00, 0x40),
    (0x20, 0x3C),
    (0x40, 0x3A),
    (0x60, 0x38),
    (0x80, 0x37),
    (0xA0, 0x36),
    (0xB0, 0x36),
    (0xC0, 0x36),
    (0xD0, 0x37),
    (0xD8, 0x38),
    (0xE0, 0x39),
    (0xE8, 0x3A),
    (0xEC, 0x3B),
    (0xF0, 0x3C),
    (0xF4, 0x3D),
    (0xF8, 0x3E),
    (0xFB, 0x3F),
    (0xFE, 0x40),
    (0x00, 0x40),
];
/// The window's offset past which the emblem isn't drawn.
const EMBLEM_HIDDEN_PAST: u32 = 0x67;
/// The spin's last step.
const SPIN_STEPS: u8 = 0x14;

impl ScreenLook {
    /// The screen makes a sound this tick.
    pub(crate) fn play(&mut self, sound: ScreenSound) {
        self.call(ScreenCall::Sound(sound));
    }

    /// The screen asks the sound driver for something this tick, after
    /// what it asked before.
    fn call(&mut self, call: ScreenCall) {
        if let Some(slot) = self.drawn.calls.iter_mut().find(|s| s.is_none()) {
            *slot = Some(call);
        }
    }

    /// The tick asks the layer to draw, after what it asked before.
    pub(crate) fn draw(&mut self, draw: Draw) {
        let slot = self.drawn.draws.iter_mut().find(|s| s.is_none());
        debug_assert!(slot.is_some(), "a tick asks of the drawing at most {} times", self.drawn.draws.len());
        if let Some(slot) = slot {
            *slot = Some(draw);
        }
    }

    /// `+0x40` set (a slide's offset, a sub-screen's timer).
    pub(crate) fn set_frame(&mut self, frame: u32) {
        self.frame = frame;
        self.draw(Draw::SetFrame(frame));
    }

    /// `+0x40` counted on, a tick.
    pub(crate) fn count_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        self.draw(Draw::CountFrame);
    }

    pub fn new(late_turns: bool, form_list_tab: bool) -> ScreenLook {
        ScreenLook {
            frame: 0,
            drawn: Drawn::default(),
            form_list_tab,
            column: [None; 5],
            column_kept: None,
            slot_chips: [None; 12],
            slot_picked: [false; 12],
            face: None,
            late_turns,
            pa_ticks: 0,
            pa_palette: 0,
        }
    }

    /// `sub_802B9E4`: every 4 ticks of the names, the pause and the
    /// result, the names' colors step through three sets, 16 ticks each
    /// (the fourth is the second's).
    pub(crate) fn blink_program_advance(&mut self) {
        if self.pa_ticks & 3 == 0 {
            let set = ((self.pa_ticks >> 4) & 3) as u8;
            self.pa_palette = if set == 3 { 1 } else { set };
        }
    }

    /// `sub_8029C08`: the emblem over the picked column, at the window's
    /// offset `x` (Beast Out's states pass 0).
    pub(crate) fn draw_emblem(&mut self, x: u32) {
        self.draw(Draw::Emblem(x));
    }

    /// `sub_802899C`: the Regular chip's frame while the folder still has
    /// its Regular chip (`counted`: this tick's count is done).
    pub(crate) fn draw_regular(&mut self, regular_pending: bool, counted: bool) {
        self.draw(Draw::Regular { pending: regular_pending, counted });
    }

    /// The chip a button holds (`Screen::hold`), drawn over it.
    pub(crate) fn draw_held(&mut self, held: bool) {
        self.draw(Draw::Held(held));
    }

    /// `sub_8029D34`: the last turns' block.
    pub(crate) fn draw_turn_limit(&mut self) {
        self.draw(Draw::TurnLimit);
    }
}

/// The console's two fade records as a screen finds them, clear (and as
/// `sub_80062D0` and `sub_80062D6` leave them, zeroed).
pub(crate) const CLEAR_FADE: Fade = Fade { mode: FadeMode::HalfOutBack, level: 0, speed: 0, target: 0, active: false, stepped: false };
const CLEAR_WINDOW_FADE: Fade =
    Fade { mode: FadeMode::ShadeWindowBack, level: 0, speed: 0, target: 0, active: false, stepped: false };

/// The dark-chip hover's fades' speed.
const SHADE_FADE_SPEED: u8 = 0xA;

impl WindowLook {
    /// The look a screen opens with (`sub_8026A50`: the emblem's matrix
    /// unrotated, at scale 1).
    pub fn new() -> WindowLook {
        WindowLook { emblem_matrix: (0, 0x40), ..WindowLook::default() }
    }

    /// Draw as the screen asked: `late_turns`, the battle's last turns
    /// (the block blinks); `on_chip`, the cursor rests on a chip.
    pub fn apply(&mut self, d: Draw, late_turns: bool, on_chip: bool) {
        match d {
            Draw::SetFrame(f) => self.frame = f,
            Draw::CountFrame => self.frame = self.frame.wrapping_add(1),
            Draw::Spin => self.spin = 1,
            Draw::Emblem(x) => self.draw_emblem(x),
            Draw::Regular { pending, counted } => self.draw_regular(pending, counted),
            Draw::TurnLimit => self.draw_turn_limit(late_turns),
            Draw::TurnLimitOff => self.turn_limit = false,
            Draw::Cursor => self.draw_cursor(),
            Draw::ElementOnChip if on_chip => self.draw_element(),
            Draw::Element => self.draw_element(),
            Draw::Held(held) => self.drawn.held = held,
            Draw::ElementOnChip | Draw::ChipWindow | Draw::FadeStarted | Draw::FadesCleared | Draw::HoverCleared | Draw::LaidOut => {}
        }
    }

    /// `sub_8029C08`: the emblem over the picked column, at the window's
    /// offset `x` (Beast Out's states pass 0), not drawn while the window
    /// is further out than 0x67; its spin steps on.
    pub fn draw_emblem(&mut self, x: u32) {
        if x > EMBLEM_HIDDEN_PAST {
            return;
        }
        self.drawn.emblem = Some((x, self.spin));
        if self.spin == 0 {
            return;
        }
        let step = self.spin;
        self.spin = if step >= SPIN_STEPS { 0 } else { step + 1 };
        if step < SPIN_STEPS {
            self.emblem_matrix = SPIN[step as usize - 1];
        }
    }

    /// `sub_8028820`: the cursor, in the frame the counter gives.
    pub fn draw_cursor(&mut self) {
        self.drawn.cursor = Some(((self.frame >> 3) & 1) as u8);
    }

    /// EXE4's 0x0801EECC: the chip window's element icon as a sprite.
    pub fn draw_element(&mut self) {
        self.drawn.element = true;
    }

    /// `sub_80289E4`: the form list window's cursor, by the same counter.
    pub fn draw_form_list_cursor(&mut self) {
        self.drawn.form_list_cursor = Some(((self.frame >> 3) & 1) as u8);
    }

    /// `sub_802899C`: the Regular chip's frame while the folder still has
    /// its Regular chip, its tiles changed every 8 frames: on a tick whose
    /// frame, before it counts on, is a multiple of 8 (`counted`: this
    /// tick's count is done, EXE4's 0x0801EF12 testing 1).
    pub fn draw_regular(&mut self, regular_pending: bool, counted: bool) {
        if !regular_pending {
            return;
        }
        if self.frame.wrapping_sub(counted as u32) & 7 == 0 {
            self.regular_frame = ((self.frame >> 3) & 1) as u8;
        }
        self.drawn.regular = true;
    }

    /// `sub_8029D34`: in the last turns the block blinks, off 4 frames of
    /// every 32.
    pub fn draw_turn_limit(&mut self, late_turns: bool) {
        if late_turns {
            self.turn_limit = self.frame & 0x1F < 0x1C;
        }
    }
}

/// The cursor's hover over a dark chip (`sub_802A2B0`), which the player's
/// screen layer runs after each tick's state: its shade, the console's two
/// fade records as it runs them, and its sound's counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hover {
    /// `+0x12`, `+0x13` (EXE4's `+0x54`, `+0x58`, `+0x5D`).
    pub shade: ChipShade,
    /// The console's first fade record as the hover runs it (modes 0x54 and
    /// 0x50): the screen's own fades run on it too in the original, on the
    /// battle's side here (`Screen::fade`); what is shown is the one started
    /// last.
    pub fade: Fade,
    /// The console's second fade record (`loc_8006274`), which only the
    /// hover runs: the window and the screen's sprites.
    pub window_fade: Fade,
    /// The hover's sound's counter (the screen's `+0x14`: every tick, 0 to
    /// 63; EXE4's `+0x5C`, down from each turn of the hover: the rules'
    /// `hover.sound`).
    pub count: u8,
}

impl Default for Hover {
    fn default() -> Hover {
        Hover { shade: ChipShade::Clear, fade: CLEAR_FADE, window_fade: CLEAR_WINDOW_FADE, count: 0 }
    }
}

impl Hover {
    /// `sub_80062D0`, `sub_80062D6`: both fade records cleared at once (EXE4's
    /// OK, 0x08020656: the rules' `fades_clear_at_ok`).
    pub fn clear_fades(&mut self) {
        self.fade = CLEAR_FADE;
        self.window_fade = CLEAR_WINDOW_FADE;
    }

    /// The fades' step a tick (`SetScreenFade`'s records: a fade toward
    /// clear ends as the rules' do).
    pub fn step_fades(&mut self, clear: crate::content::FadeClear) {
        self.fade.step(clear);
        self.window_fade.step(clear);
    }

    /// The hover over a dark chip ends at once, its music and the
    /// screen's player at the volumes its clearing ramp ends on (the
    /// round's time limit, nettai's own, closing the screen where the hover
    /// would not run again: `Screen::tick_time_up`).
    pub fn clear(&mut self, rules: &HoverRules, calls: &mut Vec<ScreenCall>) {
        if self.shade == ChipShade::Clear {
            return;
        }
        if let Some(&[music, screen]) = rules.to_clear.iter().rev().find_map(|v| v.as_ref()) {
            calls.push(ScreenCall::Volume { music, screen });
        }
        self.shade = ChipShade::Clear;
    }

    /// A tick of the hover over a dark chip, as the rules' `hover` has it
    /// (EXE6's `sub_802A2B0`, EXE4's 0x0801E478). Resting on one
    /// (`on_shading`, `sub_802A394`; none: the tick's state left the chips,
    /// and the hover turns neither way) darkens the screen and the window,
    /// and ramps the music down and the screen's player up a tick at a time
    /// (the rules' `to_dark`); leaving it undoes that (`to_clear`). A ramp
    /// runs to its end before the hover turns again. Its sound: the
    /// `+0x14` counter from the screen's opening, stepped after the ramp,
    /// sounding as it wraps while the hover isn't clear (EXE5's 0x08025A8C;
    /// on a tick with both, the volumes are set before the sound is asked
    /// for); or EXE4's `+0x5C`, from each turn of the hover, stepped before
    /// it while the shade has settled, sounding on its first such tick and
    /// every `every` after. Whether it started a fade.
    pub fn hover(&mut self, on_shading: Option<bool>, rules: &HoverRules, calls: &mut Vec<ScreenCall>) -> bool {
        match rules.sound {
            HoverSound::FromOpening { every } => {
                let turned = self.hover_state(on_shading, rules, calls);
                self.count = (self.count + 1) % every;
                if self.count == 0 && self.shade != ChipShade::Clear {
                    calls.push(ScreenCall::Sound(ScreenSound::Shade));
                }
                turned
            }
            HoverSound::WhileDark { every } => {
                if self.shade == ChipShade::Shaded {
                    self.count = self.count.wrapping_sub(1);
                    if self.count == 0 {
                        calls.push(ScreenCall::Sound(ScreenSound::Shade));
                        self.count = every;
                    }
                }
                let turned = self.hover_state(on_shading, rules, calls);
                if turned {
                    self.count = 1;
                }
                turned
            }
        }
    }

    /// The hover's state a tick; whether it turned.
    fn hover_state(&mut self, on_shading: Option<bool>, rules: &HoverRules, calls: &mut Vec<ScreenCall>) -> bool {
        let ramp = |calls: &mut Vec<ScreenCall>, ticks: &[Option<[u16; 2]>], step: u8| -> bool {
            if let Some(Some([music, screen])) = ticks.get(step as usize) {
                calls.push(ScreenCall::Volume { music: *music, screen: *screen });
            }
            step as usize + 1 < ticks.len()
        };
        let (shade, turned) = match self.shade {
            ChipShade::Clear if on_shading == Some(true) => {
                // sub_802A2E8
                self.fade.start(FadeMode::Shade, SHADE_FADE_SPEED);
                self.window_fade.start(FadeMode::ShadeWindow, SHADE_FADE_SPEED);
                (ChipShade::Shading { step: 0 }, true)
            }
            ChipShade::Shaded if on_shading == Some(false) => {
                // sub_802A33E
                self.fade.start(FadeMode::ShadeBack, SHADE_FADE_SPEED);
                self.window_fade.start(FadeMode::ShadeWindowBack, SHADE_FADE_SPEED);
                (ChipShade::Clearing { step: 0 }, true)
            }
            // sub_802A30C
            ChipShade::Shading { step } => {
                (if ramp(calls, &rules.to_dark, step) { ChipShade::Shading { step: step + 1 } } else { ChipShade::Shaded }, false)
            }
            // sub_802A362
            ChipShade::Clearing { step } => {
                (if ramp(calls, &rules.to_clear, step) { ChipShade::Clearing { step: step + 1 } } else { ChipShade::Clear }, false)
            }
            d => (d, false),
        };
        self.shade = shade;
        turned
    }
}

impl std::hash::Hash for ScreenLook {
    /// Presentation: left out of the state digest.
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

impl Default for WindowLook {
    fn default() -> WindowLook {
        WindowLook { frame: 0, spin: 0, emblem_matrix: (0, 0), turn_limit: false, regular_frame: 0, drawn: Shown::default() }
    }
}
