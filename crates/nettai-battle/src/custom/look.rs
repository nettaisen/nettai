//! What a player's custom screen shows that the simulation never reads:
//! the counters its window and sprites animate by, and the screen fades
//! its console runs (presentation; the state digest leaves it out, like
//! `Look`). A frontend draws the screen from this and the `Screen` itself
//! (nettai-render `custom`). See docs/engine/custom-screen.md §9.

use crate::battle::{Fade, FadeMode};
use crate::content::SoundRole;
use crate::custom::GameVersion;

/// The original's presentation state of a screen (the control block at
/// `0x020364C0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenLook {
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
    /// The screen fade the screen runs on its console (Beast Out's, the
    /// Program Advance's, a dark chip's).
    pub fade: Fade,
    /// The console's second fade record (`loc_8006274`), which only a dark
    /// chip's hover runs: the window and the screen's sprites.
    pub window_fade: Fade,
    /// `+0x12`, `+0x13`: the cursor's dark-chip hover (`sub_802A2B0`).
    pub dark: DarkHover,
    /// What this tick drew.
    pub drawn: Drawn,
    /// What the chip window shows: what it was drawn for last
    /// (`sub_8028476`).
    pub chip_window: ChipWindow,
    /// The window has the Cross tab (MegaMan, with a Cross he owns and
    /// hasn't used this round: `sub_8029EC8`).
    pub cross_tab: bool,
    /// The picked column's icons, as the screen copied them (`sub_80281D4`:
    /// each pick's chip as checked, Beast Out's the BeastOut chip's; a
    /// Beast Out puts the picks back in their new order, unchecked).
    pub column: [Option<super::FolderChip>; 5],
    /// A column cell past the picks whose frame is still drawn filled: the
    /// cell a button's held chip left, on the tick it leaves (EXE5's Arm
    /// Change, 0x080236C0, takes the pick without drawing; its blink draws
    /// the cell empty from the next tick, 0x08023712).
    pub column_kept: Option<u8>,
    /// The hover's counter (the screen's `+0x14`): every tick, 0 to 63.
    pub hover_count: u8,
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

/// The cursor's dark-chip hover (`sub_802A2B0`, `+0x12`), with the step
/// of its volume ramp (`+0x13`, a halfword's offset in the original).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DarkHover {
    /// 0: not on a dark chip.
    Clear,
    /// 4: the fades run toward dark.
    Darkening { step: u8 },
    /// 8: dark.
    Dark,
    /// 0xC: the fades run back.
    Clearing { step: u8 },
}

/// The dark-chip hover's fades' speed.
const DARK_FADE_SPEED: u8 = 0xA;

/// The hover's volume ramps (`byte_802A3F4`, `byte_802A400`): down for the
/// music and up for the screen's player while darkening, the other way
/// back. The window's fade takes 5 steps out and 6 back, so the ramps
/// never run past their ends.
const VOLUME_DOWN: [u16; 6] = [0x100, 0xE0, 0xC0, 0xA0, 0x80, 0x80];
const VOLUME_UP: [u16; 6] = [0x80, 0x80, 0xA0, 0xC0, 0xE0, 0x100];

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
}

/// The sprites a tick of the screen queued.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drawn {
    /// The cursor (`sub_8028820`), in its first or second frame.
    pub cursor: Option<u8>,
    /// The Cross window's cursor (`sub_80289E4`), in its first or second
    /// frame.
    pub cross_cursor: Option<u8>,
    /// The emblem (`sub_8029C08`): the window's offset it was drawn at, and
    /// the spin it was drawn with.
    pub emblem: Option<(u32, u8)>,
    /// The Regular chip's frame (`sub_802899C`).
    pub regular: bool,
    /// The icon of the chip a button holds, over the button (EXE5's Arm
    /// Change, 0x080254F4).
    pub held: bool,
    /// What the tick asked of the sound driver, in the order it asked (its
    /// player hears it): the sounds of the tick's state, then the dark
    /// chip hover's, whose routine runs after the state's and sets its
    /// volumes before it sounds.
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
    /// The ruleset's role for it.
    pub fn role(self) -> SoundRole {
        match self {
            ScreenSound::Open => SoundRole::CustomOpen,
            ScreenSound::Cursor => SoundRole::CustomCursor,
            ScreenSound::Hide => SoundRole::CustomHide,
            ScreenSound::DarkHover => SoundRole::CustomDarkHover,
            ScreenSound::Pick => SoundRole::CustomPick,
            ScreenSound::Ok => SoundRole::CustomOk,
            ScreenSound::Back => SoundRole::CustomBack,
            ScreenSound::Refused => SoundRole::Refused,
            ScreenSound::CrossWindowOpen => SoundRole::CustomCrossOpen,
            ScreenSound::CrossWindowClose => SoundRole::CustomCrossClose,
            ScreenSound::CrossChosen => SoundRole::CustomCrossChosen,
            ScreenSound::RunMessage => SoundRole::CustomRunMessage,
            ScreenSound::Description => SoundRole::CustomDescription,
            ScreenSound::DescriptionClose => SoundRole::CustomDescriptionClose,
            ScreenSound::BeastOut(GameVersion::Falzar) => SoundRole::CustomBeastOutFalzar,
            ScreenSound::BeastOut(GameVersion::Gregar) => SoundRole::CustomBeastOutGregar,
            ScreenSound::BeastOutFlash => SoundRole::CustomBeastOutFlash,
            ScreenSound::Cancel => SoundRole::CustomCancel,
            ScreenSound::Redeal => SoundRole::CustomRedeal,
            ScreenSound::RedealShuffle => SoundRole::CustomRedealShuffle,
            ScreenSound::Scrap => SoundRole::CustomScrap,
            ScreenSound::ScrapDone => SoundRole::CustomScrapDone,
            ScreenSound::ProgramAdvancePart => SoundRole::ProgramAdvancePart,
            ScreenSound::ProgramAdvance => SoundRole::ProgramAdvance,
        }
    }
}

impl Drawn {
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

    /// The volumes a dark chip's hover set this tick (the music's, the
    /// screen's player's), if it set them.
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
    /// The cursor moves to another slot (`sub_8028B74`), or in the Cross
    /// window.
    Cursor,
    /// SELECT hides the window, and a key brings it back (`sub_8026D06`).
    Hide,
    /// The hover over a dark chip, every 64 ticks while the screen isn't
    /// clear of it (EXE5's 0x08025AA2; a game without the role plays none).
    DarkHover,
    /// A chip, Beast Out, the scrap or a Cross picked.
    Pick,
    /// OK (`sub_8028D3A`).
    Ok,
    /// A pick taken back (`sub_8029032`).
    Back,
    /// What can't be picked or taken back.
    Refused,
    /// The Cross window opens (`sub_8027834`) and closes (`sub_802790C`);
    /// a Cross is put on (`sub_8027AAE`).
    CrossWindowOpen,
    CrossWindowClose,
    CrossChosen,
    /// L: the no-running message (`sub_8026EC8`).
    RunMessage,
    /// R: a description opens, and closes (`sub_8026E4C`).
    Description,
    DescriptionClose,
    /// Beast Out chosen (`sub_802774C`, and the BeastOut chip's
    /// `sub_8027624`): its two sounds with the pick's. The first is the
    /// version's (the console's own: Gregar's on a Gregar console).
    BeastOut(GameVersion),
    BeastOutFlash,
    /// A Beast Out or a Cross taken back.
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

    pub fn new(late_turns: bool, cross_tab: bool, last_chip: Option<super::FolderChip>) -> ScreenLook {
        ScreenLook {
            frame: 0,
            spin: 0,
            // sub_8026A50: the emblem's matrix starts unrotated, at scale 1.
            emblem_matrix: (0, 0x40),
            turn_limit: false,
            regular_frame: 0,
            fade: Fade { mode: FadeMode::BeastOutBack, level: 0, speed: 0, target: 0, active: false, stepped: false },
            window_fade: Fade {
                mode: FadeMode::DarkChipWindowBack,
                level: 0,
                speed: 0,
                target: 0,
                active: false,
                stepped: false,
            },
            dark: DarkHover::Clear,
            drawn: Drawn::default(),
            chip_window: ChipWindow { slot: 0, picks: 0, last_chip, framed: None },
            cross_tab,
            column: [None; 5],
            column_kept: None,
            hover_count: 0,
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

    /// `sub_802A2B0`, after every tick's state: the hover over a dark chip.
    /// Resting on one (`on_dark`, `sub_802A394`) darkens the screen and
    /// the window and turns the music down and the screen's player up, a
    /// step a tick until the window's fade is done; leaving it undoes that
    /// the same way. Its `+0x14` counter runs every tick from the screen's
    /// opening; each time it wraps (every 64 ticks) while the hover isn't
    /// clear, EXE5 plays the hover's sound (0x08025A8C; EXE6's routine
    /// counts and plays nothing: its rules fill no role for it). The
    /// routine runs its state first and steps the counter after (EXE5's
    /// 0x08025A80), so on a tick with both, the volumes are set before the
    /// sound is asked for.
    pub(crate) fn hover(&mut self, on_dark: bool) {
        self.hover_state(on_dark);
        self.hover_count = (self.hover_count + 1) & 63;
        if self.hover_count == 0 && self.dark != DarkHover::Clear {
            self.play(ScreenSound::DarkHover);
        }
    }

    fn hover_state(&mut self, on_dark: bool) {
        self.dark = match self.dark {
            DarkHover::Clear if on_dark => {
                // sub_802A2E8
                self.fade.start(FadeMode::DarkChip, DARK_FADE_SPEED);
                self.window_fade.start(FadeMode::DarkChipWindow, DARK_FADE_SPEED);
                DarkHover::Darkening { step: 0 }
            }
            DarkHover::Dark if !on_dark => {
                // sub_802A33E
                self.fade.start(FadeMode::DarkChipBack, DARK_FADE_SPEED);
                self.window_fade.start(FadeMode::DarkChipWindowBack, DARK_FADE_SPEED);
                DarkHover::Clearing { step: 0 }
            }
            DarkHover::Darkening { step } => {
                // sub_802A30C
                self.call(ScreenCall::Volume { music: VOLUME_DOWN[step as usize], screen: VOLUME_UP[step as usize] });
                if self.window_fade.active() { DarkHover::Darkening { step: step + 1 } } else { DarkHover::Dark }
            }
            DarkHover::Clearing { step } => {
                // sub_802A362
                self.call(ScreenCall::Volume { music: VOLUME_UP[step as usize], screen: VOLUME_DOWN[step as usize] });
                if self.window_fade.active() { DarkHover::Clearing { step: step + 1 } } else { DarkHover::Clear }
            }
            d => d,
        };
    }

    /// `sub_8029C08`: the emblem over the picked column, at the window's
    /// offset `x` (Beast Out's states pass 0), not drawn while the window
    /// is further out than 0x67; its spin steps on.
    pub(crate) fn draw_emblem(&mut self, x: u32) {
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
    pub(crate) fn draw_cursor(&mut self) {
        self.drawn.cursor = Some(((self.frame >> 3) & 1) as u8);
    }

    /// `sub_80289E4`: the Cross window's cursor, by the same counter.
    pub(crate) fn draw_cross_cursor(&mut self) {
        self.drawn.cross_cursor = Some(((self.frame >> 3) & 1) as u8);
    }

    /// `sub_802899C`: the Regular chip's frame while the folder still has
    /// its Regular chip, its tiles changed every 8 frames.
    pub(crate) fn draw_regular(&mut self, regular_pending: bool) {
        if !regular_pending {
            return;
        }
        if self.frame & 7 == 0 {
            self.regular_frame = ((self.frame >> 3) & 1) as u8;
        }
        self.drawn.regular = true;
    }

    /// The chip a button holds (`Screen::hold`), drawn over it (EXE5's
    /// 0x080254F4: slot 8 an Arm Change button, picked).
    pub(crate) fn draw_held(&mut self, held: bool) {
        self.drawn.held = held;
    }

    /// `sub_8029D34`: in the last turns the block blinks, off 4 frames of
    /// every 32.
    pub(crate) fn draw_turn_limit(&mut self) {
        if self.late_turns {
            self.turn_limit = self.frame & 0x1F < 0x1C;
        }
    }
}

impl std::hash::Hash for ScreenLook {
    /// Presentation: left out of the state digest.
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}
