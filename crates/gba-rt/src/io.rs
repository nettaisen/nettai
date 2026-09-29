//! IO registers and DMA.
//!
//! Registers are stored as written, in `Memory::io`. Reads with side
//! channels (keys, display status, interrupt flags) are synthesized from
//! host-controlled state. DMA transfers run to completion at their trigger:
//! immediately on enable, or when the host signals VBlank/HBlank. Timers and
//! serial are left to the host (see [`crate::Host`]).

use crate::Cpu;

pub const REG_DISPSTAT: u32 = 0x04;
pub const REG_VCOUNT: u32 = 0x06;
pub const REG_KEYINPUT: u32 = 0x130;
pub const REG_IE: u32 = 0x200;
pub const REG_IF: u32 = 0x202;
pub const REG_IME: u32 = 0x208;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DmaTiming {
    #[default]
    Immediate,
    VBlank,
    HBlank,
    Special,
}

/// A DMA channel's latched state.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dma {
    pub enabled: bool,
    pub src: u32,
    pub dst: u32,
    pub count: u32,
    pub control: u16,
}

impl Dma {
    pub fn timing(&self) -> DmaTiming {
        match (self.control >> 12) & 3 {
            0 => DmaTiming::Immediate,
            1 => DmaTiming::VBlank,
            2 => DmaTiming::HBlank,
            _ => DmaTiming::Special,
        }
    }
}

#[derive(Default)]
pub struct IoState {
    /// Buttons currently held (GBA bit order, 1 = pressed).
    pub keys: u16,
    /// Whether DISPSTAT reads report VBlank.
    pub in_vblank: bool,
    pub vcount: u16,
    pub dma: [Dma; 4],
    /// Reads/writes to unmapped memory (should stay 0).
    pub bad_accesses: u64,
}

impl Cpu {
    /// The raw stored value of an IO register (no side effects).
    pub fn io_peek16(&self, addr: u32) -> u16 {
        let o = (addr & 0x3FE) as usize;
        u16::from_le_bytes([self.mem.io[o], self.mem.io[o + 1]])
    }

    fn io_poke16(&mut self, addr: u32, v: u16) {
        let o = (addr & 0x3FE) as usize;
        self.mem.io[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }

    pub fn io_read16(&mut self, addr: u32) -> u16 {
        let reg = addr & 0x00FF_FFFE;
        if reg >= 0x400 {
            self.io.bad_accesses += 1;
            return 0;
        }
        if let Some(v) = self.with_host(|h, c| h.io_read16(c, addr)) {
            return v;
        }
        match reg {
            REG_DISPSTAT => {
                let stored = self.io_peek16(reg) & 0xFFF8;
                let vcount_match = (self.io.vcount == (stored >> 8)) as u16;
                stored | self.io.in_vblank as u16 | (vcount_match << 2)
            }
            REG_VCOUNT => self.io.vcount,
            REG_KEYINPUT => !self.io.keys & 0x3FF,
            _ => self.io_peek16(reg),
        }
    }

    pub fn io_write16(&mut self, addr: u32, v: u16) {
        let reg = addr & 0x00FF_FFFE;
        if reg >= 0x400 {
            self.io.bad_accesses += 1;
            return;
        }
        match reg {
            REG_IF => {
                // Writing 1 acknowledges.
                let cur = self.io_peek16(reg);
                self.io_poke16(reg, cur & !v);
            }
            REG_VCOUNT | REG_KEYINPUT => {}
            0xB0..=0xDF => {
                self.io_poke16(reg, v);
                let ch = ((reg - 0xB0) / 12) as usize;
                if (reg - 0xB0) % 12 == 10 {
                    self.dma_control_written(ch, v);
                }
            }
            _ => self.io_poke16(reg, v),
        }
        self.with_host(|h, c| h.io_write16(c, addr, v));
    }

    fn dma_control_written(&mut self, ch: usize, control: u16) {
        let was = self.io.dma[ch].enabled;
        let base = 0xB0 + 12 * ch as u32;
        let enable = control & 0x8000 != 0;
        self.io.dma[ch].enabled = enable;
        self.io.dma[ch].control = control;
        if enable && !was {
            let src = self.io_peek16(base) as u32 | (self.io_peek16(base + 2) as u32) << 16;
            let dst = self.io_peek16(base + 4) as u32 | (self.io_peek16(base + 6) as u32) << 16;
            let mut count = self.io_peek16(base + 8) as u32;
            let max = if ch == 3 { 0x10000 } else { 0x4000 };
            count &= max - 1;
            if count == 0 {
                count = max;
            }
            let (src_mask, dst_mask) = if ch == 0 { (0x07FF_FFFF, 0x07FF_FFFF) } else { (0x0FFF_FFFF, 0x07FF_FFFF) };
            let d = &mut self.io.dma[ch];
            d.src = src & src_mask;
            d.dst = dst & if ch == 3 { 0x0FFF_FFFF } else { dst_mask };
            d.count = count;
            if d.timing() == DmaTiming::Immediate {
                self.run_dma(ch);
            }
        }
    }

    /// Run every enabled DMA channel with the given start timing.
    pub fn trigger_dma(&mut self, timing: DmaTiming) {
        for ch in 0..4 {
            if self.io.dma[ch].enabled && self.io.dma[ch].timing() == timing {
                self.run_dma(ch);
            }
        }
    }

    fn run_dma(&mut self, ch: usize) {
        let d = self.io.dma[ch];
        let word = d.control & (1 << 10) != 0;
        let size = if word { 4 } else { 2 };
        let step = |mode: u16| -> i32 {
            match mode {
                0 | 3 => size,
                1 => -size,
                _ => 0,
            }
        };
        let dst_mode = (d.control >> 5) & 3;
        let src_step = step((d.control >> 7) & 3);
        let dst_step = step(dst_mode);
        let (mut src, mut dst) = (d.src, d.dst);
        for _ in 0..d.count {
            if word {
                let v = self.ld32(src & !3);
                self.st32(dst & !3, v);
            } else {
                let v = self.ld16(src & !1);
                self.st16(dst & !1, v);
            }
            src = src.wrapping_add(src_step as u32);
            dst = dst.wrapping_add(dst_step as u32);
        }
        let base = 0xB0 + 12 * ch as u32;
        let repeat = d.control & (1 << 9) != 0 && d.timing() != DmaTiming::Immediate;
        let reload = self.io_peek16(base + 4) as u32 | (self.io_peek16(base + 6) as u32) << 16;
        let control = self.io_peek16(base + 10);
        let dm = &mut self.io.dma[ch];
        dm.src = src;
        dm.dst = if repeat && dst_mode == 3 { reload } else { dst };
        if !repeat {
            dm.enabled = false;
            self.io_poke16(base + 10, control & !0x8000);
        }
        if d.control & (1 << 14) != 0 {
            let flags = self.io_peek16(REG_IF) | (1 << (8 + ch));
            self.io_poke16(REG_IF, flags);
        }
    }

    /// Latch an interrupt request (bit numbers as in IE/IF).
    pub fn request_irq(&mut self, bit: u32) {
        let flags = self.io_peek16(REG_IF) | (1 << bit);
        self.io_poke16(REG_IF, flags);
    }

    /// Whether any enabled interrupt is pending and IME is set.
    pub fn irq_pending(&self) -> bool {
        self.io_peek16(REG_IME) & 1 != 0 && self.io_peek16(REG_IE) & self.io_peek16(REG_IF) != 0
    }
}
