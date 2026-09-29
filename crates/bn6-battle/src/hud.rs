//! The parts of the battle HUD that gate simulation: the custom gauge and
//! the banner lifetime. Drawing is left to the frontend.

/// The custom gauge: one per battle, shared by both players.
#[derive(Clone, Debug)]
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

    /// Fill rate from both players' gauge speed classes (NaviStats+0x08:
    /// 0 normal, 1 fast, 2 slow).
    pub fn rate_for(speed0: u8, speed1: u8) -> u16 {
        const RATES: [u16; 9] = [0x20, 0x40, 0x10, 0x40, 0x40, 0x20, 0x10, 0x20, 0x10];
        RATES[(3 * speed1 as usize + speed0 as usize).min(8)]
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
    /// Banner type; types 2 and 4 hold until removed.
    pub kind: u8,
}

/// `sub_801E754`: what the flow sees of the banner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerStatus {
    Done,
    Showing,
    /// A type 2/4 banner holding until removed.
    Holding,
}

impl Banner {
    /// Start a banner unless one is showing. Returns false if one was.
    pub fn start(&mut self, kind: u8) -> bool {
        if self.active {
            return false;
        }
        *self = Banner { active: true, step: 0, timer: 0, kind };
        true
    }

    pub fn status(&self) -> BannerStatus {
        if !self.active {
            BannerStatus::Done
        } else if self.step == 4 && matches!(self.kind, 2 | 4) {
            BannerStatus::Holding
        } else {
            BannerStatus::Showing
        }
    }

    /// One tick of the banner task (`sub_801CE28`).
    pub fn tick(&mut self) {
        if !self.active {
            return;
        }
        let next = self.timer.wrapping_add(1);
        if !(next == 5 && self.step == 4 && matches!(self.kind, 2 | 4)) {
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

/// A banner id's type (ids are multiples of 4).
pub fn banner_kind(id: u8) -> u8 {
    crate::data::BANNER_TYPES[(id / 4) as usize]
}
