//! The parts of the battle HUD that gate simulation: the custom gauge and
//! the banner lifetime. Drawing is left to the frontend.

use crate::content::BannerId;
use nettai_content_api::ChipHandle;
use crate::setup::GaugeSpeed;

/// The custom gauge: one per battle, shared by both players.
#[derive(Clone, Debug, Hash)]
pub struct CustomGauge {
    /// 0..=FULL.
    pub value: u16,
    /// Added per tick while filling.
    pub rate: u16,
    /// Whether the gauge task runs (off while the custom screen is up and
    /// after the round is decided).
    pub enabled: bool,
}

impl CustomGauge {
    pub const FULL: u16 = 0x4000;

    pub fn new() -> CustomGauge {
        CustomGauge { value: 0, rate: 0x20, enabled: false }
    }

    /// Fill rate from both players' gauge speeds.
    pub fn rate_for(speed0: GaugeSpeed, speed1: GaugeSpeed) -> u16 {
        const RATES: [u16; 9] = [0x20, 0x40, 0x10, 0x40, 0x40, 0x20, 0x10, 0x20, 0x10];
        RATES[3 * speed1 as usize + speed0 as usize]
    }
}

/// A banner's life: slide in 5 ticks, hold 0x30, slide out 5, then clear.
/// Flow code waits on banners, so their lifetime is simulation state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Banner {
    pub active: bool,
    /// 0 slide-in, 4 hold, 8 slide-out, 0xC finished (the game's step codes).
    pub step: u8,
    pub timer: u8,
    /// Stays up until removed instead of sliding out.
    pub holds: bool,
    /// Which banner is showing (presentation only).
    pub id: Option<BannerId>,
    /// A telop's text (presentation only).
    pub telop: Option<Telop>,
}

/// What a dimming controller's telop will say of its chip (the
/// controller's +0x30 and +0x32, and its damage). Presentation only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TelopChip {
    /// None: the controller's chip field held none (the game's 0).
    pub chip: Option<ChipHandle>,
    /// The Atk+ / cross bonus.
    pub bonus: u16,
    /// The controller's damage, when it isn't in the object's `damage`
    /// (the engine's navi chip controller keeps its damage word apart).
    pub damage: Option<u16>,
}

/// What a telop says (`sub_801E792`'s arguments for banners 0x4C and
/// 0x50): the chip's name and, for a chip whose damage shows, the damage
/// and the bonus. Presentation only; what each player is shown is
/// [`Battle::telop_for`](crate::Battle::telop_for).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Telop {
    /// The side whose chip it is.
    pub side: u8,
    /// The chip named. None: the controller's telop names a chip the
    /// engine wasn't told (a dimming content starts itself).
    pub chip: Option<ChipHandle>,
    /// The controller's damage (0: none shown).
    pub damage: u16,
    /// The damage word's double flag: "x2" after the numbers.
    pub doubled: bool,
    /// The bonus shown as "+N" after the damage (0: none shown).
    pub bonus: u16,
    pub hidden: TelopHidden,
}

/// What a player's console shows of their own navi's chips: the icons over
/// the navi (HUD draw task 1) and the chip name window at the bottom left
/// (task 6), as the navis' and the flow's HUD calls turn them on and off
/// (`sub_801DA48`, `sub_801DACC`). Presentation only: the engine keeps one
/// for each side, since each player's console shows its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChipHud {
    pub icons: bool,
    pub window: bool,
}

/// The HUD parts a chip's effect hides while it plays (`sub_801DACC`, and
/// `sub_801DA48` showing them again): the Gregar and Falzar chips' hide the
/// custom gauge and the emotion window (draw tasks 4 and 14; with each side's
/// own gauge, battle flag 0x40, 17 and 14). Presentation only: each console
/// runs the same calls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HudHidden {
    /// The custom gauge (draw task 4, `sub_801C4E4`).
    pub gauge: bool,
    /// The emotion window (draw task 14, `sub_801CDEC`).
    pub emotion_window: bool,
    /// The own-gauges mode's gauge, drawn by its levels (draw task
    /// 17, `sub_801C640`).
    pub level_gauge: bool,
}

/// A warning marker on a console's HUD for one tick (`sub_800AE90`: a
/// blinking arrow): over the custom gauge (`at` none), or over a place on
/// the field, which the console projects. Presentation only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Warning {
    pub at: Option<crate::object::Vec3>,
}

/// What the HUD's message line says (the game's text script for it has
/// more: the multiple deletions of virus battles).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    /// "COUNTER HIT!" (`sub_801E270`).
    CounterHit,
}

/// A message every console shows for a second (HUD task bit 8).
/// Presentation only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageLine {
    pub message: Message,
    /// HUD ticks left of its `SHOWN_TICKS`.
    pub ticks: u8,
}

impl MessageLine {
    /// How long a message shows.
    pub const SHOWN_TICKS: u8 = 0x3C;
}

/// An HP number a console's HUD shows under an object (`sub_801DC7C`: one
/// of the four places of `byte_203EB50`, HUD task bit 2): the opponent's
/// navi's, LilBoiler's. Presentation only; the number rolls toward the
/// object's HP as the frontend draws it (`sub_801C168`), and the place is
/// free again once the object's HP is 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HpNumber {
    pub object: crate::object::ObjectRef,
    /// Pixels from where the HUD projects the object's position (+4, +5).
    pub dx: i8,
    pub dy: i8,
    /// The object's HP when the number was asked for: where it starts
    /// rolling (+2).
    pub hp: u16,
    /// It shows the damage taken (max HP less HP) instead, without
    /// centering its digits (flags 0x18, which the original gives
    /// LilBoiler's NameID and its training viruses').
    pub damage: bool,
}

impl HpNumber {
    /// How many objects a console's HUD numbers.
    pub const PLACES: usize = 4;
}

/// A chip a player just used, as the other player's console names it for
/// a second (`sub_801EB18`; HUD task bit 16): any chip but a cut-in chip,
/// whose telop both see. Presentation only; what a player is shown is
/// [`Battle::used_chip_for`](crate::Battle::used_chip_for).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsedChip {
    pub chip: ChipHandle,
    /// The attack's damage, for a chip whose damage shows (0: none shown).
    pub damage: u16,
    /// The damage word's double flag: "x2" after the numbers.
    pub doubled: bool,
    /// The bonus shown as "+N" after the damage (0: none shown).
    pub bonus: u16,
    /// HUD ticks left of its `SHOWN_TICKS`.
    pub ticks: u8,
}

impl UsedChip {
    /// How long the name shows.
    pub const SHOWN_TICKS: u8 = 0x3C;
}

/// Who sees "????" for a telop's chip (`sub_800BBA8`, the trap chips).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TelopHidden {
    /// Both players see the chip.
    #[default]
    No,
    /// The other player sees "????".
    FromOpponent,
    /// Both players see "????".
    FromBoth,
}

/// `sub_801E754`: what the flow sees of the banner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerStatus {
    Done,
    Showing,
    /// A holding banner, up until removed.
    Holding,
}

impl Banner {
    /// Start a banner unless one is showing; `holds`: it stays up until
    /// removed (`Rules::banner_holds`). Returns false if one was showing.
    pub fn start(&mut self, id: BannerId, holds: bool) -> bool {
        if self.active {
            return false;
        }
        *self = Banner { active: true, step: 0, timer: 0, holds, id: Some(id), telop: None };
        true
    }

    pub fn status(&self) -> BannerStatus {
        if !self.active {
            BannerStatus::Done
        } else if self.step == 4 && self.holds {
            BannerStatus::Holding
        } else {
            BannerStatus::Showing
        }
    }

    /// `sub_801E780`: let a holding banner go: it holds three more ticks,
    /// then slides out.
    pub fn release(&mut self) {
        if self.holds {
            self.timer = 0x2D;
        }
    }

    /// One tick of the banner task (`sub_801CE28`).
    pub fn tick(&mut self) {
        if !self.active {
            return;
        }
        let next = self.timer.wrapping_add(1);
        if !(next == 5 && self.step == 4 && self.holds) {
            self.timer = next;
        }
        match self.step {
            0 if self.timer >= 5 => {
                self.step = 4;
                self.timer = 0;
            }
            4 if self.timer >= 0x30 => {
                self.step = 8;
                self.timer = 0;
            }
            8 if self.timer >= 5 => self.step = 0xC,
            0xC => self.active = false,
            _ => {}
        }
    }
}
