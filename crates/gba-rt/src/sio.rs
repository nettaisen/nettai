//! The serial port, following mGBA's GBASIO register semantics. Transfers
//! complete immediately (there is no cycle timing). A [`SioLink`] supplies
//! the other end of a multiplayer cable; without one the port behaves like
//! mGBA with nothing attached.

use crate::Cpu;

pub const REG_SIOMULTI0: u32 = 0x120;
pub const REG_SIOCNT: u32 = 0x128;
pub const REG_SIOMLT_SEND: u32 = 0x12A;
pub const REG_RCNT: u32 = 0x134;
const IRQ_SERIAL: u32 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SioMode {
    #[default]
    Normal8,
    Normal32,
    Multi,
    Uart,
    Gpio,
    Joybus,
}

/// The far end of a multiplayer cable.
pub trait SioLink {
    /// This console's multiplayer id (0 = master).
    fn id(&self) -> u32;
    /// Number of other consoles on the cable.
    fn connected(&self) -> u32;
    /// The master started a transfer sending `send`: deliver it and return
    /// the words every console sent (index = id; 0xFFFF for absent ones).
    fn transfer(&mut self, send: u16) -> [u16; 4];
}

#[derive(Default)]
pub struct SioState {
    pub siocnt: u16,
    pub rcnt: u16,
    pub mode: SioMode,
}

impl SioState {
    fn switch_mode(&mut self) {
        let m = ((self.rcnt & 0xC000) | (self.siocnt & 0x3000)) >> 12;
        self.mode = if m < 8 {
            match m & 3 {
                0 => SioMode::Normal8,
                1 => SioMode::Normal32,
                2 => SioMode::Multi,
                _ => SioMode::Uart,
            }
        } else if m & 0xC == 8 {
            SioMode::Gpio
        } else {
            SioMode::Joybus
        };
    }
}

impl Cpu {
    pub(crate) fn sio_read(&mut self, reg: u32) -> Option<u16> {
        match reg {
            REG_SIOCNT => Some(self.io.sio.siocnt),
            REG_RCNT => Some(self.io.sio.rcnt),
            _ => None,
        }
    }

    /// Handle a write to a serial register; returns true if handled.
    pub(crate) fn sio_write(&mut self, reg: u32, v: u16) -> bool {
        match reg {
            REG_SIOCNT => {
                self.write_siocnt(v & 0x7FFF);
                true
            }
            REG_RCNT => {
                let v = v & 0xC1FF;
                let s = &mut self.io.sio;
                s.rcnt &= 0x1FF;
                s.rcnt |= v & 0xC000;
                s.switch_mode();
                if s.mode == SioMode::Gpio {
                    s.rcnt &= 0xC000;
                    s.rcnt |= v & 0x1FF;
                } else {
                    s.rcnt &= 0xC00F;
                    s.rcnt |= v & 0x1F0;
                }
                true
            }
            _ => false,
        }
    }

    fn write_siocnt(&mut self, mut value: u16) {
        {
            let s = &mut self.io.sio;
            if (value ^ s.siocnt) & 0x3000 != 0 {
                s.siocnt = value & 0x3000;
                s.switch_mode();
            }
        }
        let (id, connected) = match &self.link {
            Some(l) => (l.id(), l.connected()),
            None => (0, 0),
        };
        let mut start = false;
        match self.io.sio.mode {
            SioMode::Multi => {
                let old = self.io.sio.siocnt;
                value &= 0xFF83;
                if id != 0 || connected == 0 {
                    value |= 1 << 2; // slave
                } else {
                    value &= !(1 << 2);
                }
                value = (value & !0x30) | ((id as u16 & 3) << 4);
                value |= old & 0x00FC;
                self.io.sio.rcnt |= 1; // SC floats high
                if value & 0x80 != 0 && old & 0x80 == 0 && id == 0 {
                    for i in 0..4 {
                        self.io_poke16_pub(REG_SIOMULTI0 + 2 * i, 0xFFFF);
                    }
                    self.io.sio.rcnt &= !1;
                    start = true;
                }
                if self.link.is_none() || connected > 0 {
                    value |= 1 << 3; // ready
                }
            }
            SioMode::Normal8 | SioMode::Normal32 => {
                if value & 1 != 0 {
                    self.io.sio.rcnt |= 1;
                }
                if value & 0x80 != 0 && self.io.sio.siocnt & 0x80 == 0 {
                    start = true;
                }
                value |= 1 << 2; // SI pulled up: nobody there
            }
            _ => {}
        }
        self.io.sio.siocnt = value;
        if start {
            self.finish_transfer();
        }
    }

    fn finish_transfer(&mut self) {
        match self.io.sio.mode {
            SioMode::Multi => {
                let send = self.io_peek16(REG_SIOMLT_SEND);
                let mut link = self.link.take();
                let (data, id) = match link.as_mut() {
                    Some(l) => (l.transfer(send), l.id()),
                    None => ([0; 4], 0),
                };
                self.link = link;
                for (i, d) in data.iter().enumerate() {
                    self.io_poke16_pub(REG_SIOMULTI0 + 2 * i as u32, *d);
                }
                self.finish_multiplayer(id);
            }
            SioMode::Normal8 => {
                let s = &mut self.io.sio;
                s.siocnt &= !0x80;
                self.io_poke16_pub(0x12A, 0);
                if self.io.sio.siocnt & (1 << 14) != 0 {
                    self.request_irq(IRQ_SERIAL);
                }
            }
            SioMode::Normal32 => {
                self.io.sio.siocnt &= !0x80;
                self.io_poke16_pub(0x120, 0);
                self.io_poke16_pub(0x122, 0);
                if self.io.sio.siocnt & (1 << 14) != 0 {
                    self.request_irq(IRQ_SERIAL);
                }
            }
            _ => {}
        }
    }

    /// Complete a multiplayer transfer on this console (SIOMULTI already
    /// filled): clear busy, set the id, raise the serial IRQ if enabled.
    pub fn finish_multiplayer(&mut self, id: u32) {
        let s = &mut self.io.sio;
        s.siocnt &= !0x80;
        s.siocnt = (s.siocnt & !0x30) | ((id as u16 & 3) << 4);
        s.rcnt |= 1;
        if s.siocnt & (1 << 14) != 0 {
            self.request_irq(IRQ_SERIAL);
        }
    }
}
