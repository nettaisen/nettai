//! The GBA memory map.
//!
//! Semantics follow mGBA where the game can observe them: unaligned 32-bit and
//! 16-bit loads rotate, stores align down, byte stores to palette and BG VRAM
//! write both bytes of the halfword, byte stores to OAM/OBJ VRAM are dropped.

use crate::Cpu;
use std::sync::Arc;

pub const EWRAM_SIZE: usize = 0x40000;
pub const IWRAM_SIZE: usize = 0x8000;
pub const IO_SIZE: usize = 0x400;
pub const PALETTE_SIZE: usize = 0x400;
pub const VRAM_SIZE: usize = 0x18000;
pub const OAM_SIZE: usize = 0x400;
pub const SRAM_SIZE: usize = 0x10000;

pub struct Memory {
    pub ewram: Box<[u8; EWRAM_SIZE]>,
    pub iwram: Box<[u8; IWRAM_SIZE]>,
    pub io: Box<[u8; IO_SIZE]>,
    pub palette: Box<[u8; PALETTE_SIZE]>,
    pub vram: Box<[u8; VRAM_SIZE]>,
    pub oam: Box<[u8; OAM_SIZE]>,
    pub sram: Box<[u8; SRAM_SIZE]>,
    pub rom: Arc<[u8]>,
}

fn zeroed<const N: usize>() -> Box<[u8; N]> {
    vec![0u8; N].into_boxed_slice().try_into().unwrap()
}

impl Memory {
    pub fn new(rom: Arc<[u8]>) -> Memory {
        Memory {
            ewram: zeroed(),
            iwram: zeroed(),
            io: zeroed(),
            palette: zeroed(),
            vram: zeroed(),
            oam: zeroed(),
            sram: Box::new([0xFF; SRAM_SIZE]),
            rom,
        }
    }

    #[inline(always)]
    fn vram_offset(addr: u32) -> usize {
        let a = (addr & 0x1FFFF) as usize;
        if a >= VRAM_SIZE { a - 0x8000 } else { a }
    }

    /// Read-only view of an address range in a RAM region, for hosts and
    /// harnesses (no side effects).
    pub fn peek(&self, addr: u32) -> u8 {
        match addr >> 24 {
            0x02 => self.ewram[(addr as usize) & (EWRAM_SIZE - 1)],
            0x03 => self.iwram[(addr as usize) & (IWRAM_SIZE - 1)],
            0x04 => *self.io.get((addr & 0xFFFFFF) as usize).unwrap_or(&0),
            0x05 => self.palette[(addr as usize) & (PALETTE_SIZE - 1)],
            0x06 => self.vram[Self::vram_offset(addr)],
            0x07 => self.oam[(addr as usize) & (OAM_SIZE - 1)],
            0x08..=0x0D => *self.rom.get((addr & 0x01FF_FFFF) as usize).unwrap_or(&0),
            0x0E | 0x0F => self.sram[(addr as usize) & (SRAM_SIZE - 1)],
            _ => 0,
        }
    }

    pub fn poke(&mut self, addr: u32, v: u8) {
        match addr >> 24 {
            0x02 => self.ewram[(addr as usize) & (EWRAM_SIZE - 1)] = v,
            0x03 => self.iwram[(addr as usize) & (IWRAM_SIZE - 1)] = v,
            0x04 => {
                if let Some(b) = self.io.get_mut((addr & 0xFFFFFF) as usize) {
                    *b = v
                }
            }
            0x05 => self.palette[(addr as usize) & (PALETTE_SIZE - 1)] = v,
            0x06 => self.vram[Self::vram_offset(addr)] = v,
            0x07 => self.oam[(addr as usize) & (OAM_SIZE - 1)] = v,
            0x0E | 0x0F => self.sram[(addr as usize) & (SRAM_SIZE - 1)] = v,
            _ => {}
        }
    }
}

#[inline(always)]
fn rd32(s: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(s[o..o + 4].try_into().unwrap())
}

#[inline(always)]
fn rd16(s: &[u8], o: usize) -> u32 {
    u16::from_le_bytes(s[o..o + 2].try_into().unwrap()) as u32
}

#[inline(always)]
fn wr32(s: &mut [u8], o: usize, v: u32) {
    s[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

#[inline(always)]
fn wr16(s: &mut [u8], o: usize, v: u32) {
    s[o..o + 2].copy_from_slice(&(v as u16).to_le_bytes());
}

impl Cpu {
    /// Aligned 32-bit read with no rotation.
    #[inline(always)]
    fn read32_aligned(&mut self, a: u32) -> u32 {
        let m = &self.mem;
        match a >> 24 {
            0x02 => rd32(&m.ewram[..], (a as usize) & (EWRAM_SIZE - 4)),
            0x03 => rd32(&m.iwram[..], (a as usize) & (IWRAM_SIZE - 4)),
            0x08..=0x0D => {
                let o = (a & 0x01FF_FFFC) as usize;
                if o + 4 <= m.rom.len() { rd32(&m.rom, o) } else { rom_open_bus(a) | (rom_open_bus(a + 2) << 16) }
            }
            _ => self.read32_slow(a),
        }
    }

    #[cold]
    #[inline(never)]
    fn read32_slow(&mut self, a: u32) -> u32 {
        match a >> 24 {
            0x04 => self.io_read16(a) as u32 | ((self.io_read16(a + 2) as u32) << 16),
            0x05 => rd32(&self.mem.palette[..], (a as usize) & (PALETTE_SIZE - 4)),
            0x06 => rd32(&self.mem.vram[..], Memory::vram_offset(a) & !3),
            0x07 => rd32(&self.mem.oam[..], (a as usize) & (OAM_SIZE - 4)),
            0x0E | 0x0F => {
                let b = self.mem.sram[(a as usize) & (SRAM_SIZE - 1)] as u32;
                b * 0x0101_0101
            }
            _ => {
                self.io.bad_accesses += 1;
                0
            }
        }
    }

    #[inline(always)]
    fn read16_aligned(&mut self, a: u32) -> u32 {
        let m = &self.mem;
        match a >> 24 {
            0x02 => rd16(&m.ewram[..], (a as usize) & (EWRAM_SIZE - 2)),
            0x03 => rd16(&m.iwram[..], (a as usize) & (IWRAM_SIZE - 2)),
            0x08..=0x0D => {
                let o = (a & 0x01FF_FFFE) as usize;
                if o + 2 <= m.rom.len() { rd16(&m.rom, o) } else { rom_open_bus(a) }
            }
            _ => self.read16_slow(a),
        }
    }

    #[cold]
    #[inline(never)]
    fn read16_slow(&mut self, a: u32) -> u32 {
        match a >> 24 {
            0x04 => self.io_read16(a) as u32,
            0x05 => rd16(&self.mem.palette[..], (a as usize) & (PALETTE_SIZE - 2)),
            0x06 => rd16(&self.mem.vram[..], Memory::vram_offset(a) & !1),
            0x07 => rd16(&self.mem.oam[..], (a as usize) & (OAM_SIZE - 2)),
            0x0E | 0x0F => {
                let b = self.mem.sram[(a as usize) & (SRAM_SIZE - 1)] as u32;
                b * 0x0101
            }
            _ => {
                self.io.bad_accesses += 1;
                0
            }
        }
    }

    #[inline(always)]
    pub fn ld32(&mut self, a: u32) -> u32 {
        let v = self.read32_aligned(a & !3);
        v.rotate_right((a & 3) * 8)
    }

    #[inline(always)]
    pub fn ld16(&mut self, a: u32) -> u32 {
        let v = self.read16_aligned(a & !1);
        v.rotate_right((a & 1) * 8)
    }

    #[inline(always)]
    pub fn ld16s(&mut self, a: u32) -> u32 {
        if a & 1 != 0 { self.ld8s(a) } else { self.read16_aligned(a) as u16 as i16 as i32 as u32 }
    }

    #[inline(always)]
    pub fn ld8(&mut self, a: u32) -> u32 {
        let m = &self.mem;
        match a >> 24 {
            0x02 => m.ewram[(a as usize) & (EWRAM_SIZE - 1)] as u32,
            0x03 => m.iwram[(a as usize) & (IWRAM_SIZE - 1)] as u32,
            0x08..=0x0D => match m.rom.get((a & 0x01FF_FFFF) as usize) {
                Some(b) => *b as u32,
                None => (rom_open_bus(a) >> ((a & 1) * 8)) & 0xFF,
            },
            0x04 => {
                let h = self.io_read16(a & !1) as u32;
                (h >> ((a & 1) * 8)) & 0xFF
            }
            _ => self.mem.peek(a) as u32,
        }
    }

    #[inline(always)]
    pub fn ld8s(&mut self, a: u32) -> u32 {
        self.ld8(a) as u8 as i8 as i32 as u32
    }

    #[inline(always)]
    pub fn st32(&mut self, a: u32, v: u32) {
        let a = a & !3;
        let m = &mut self.mem;
        match a >> 24 {
            0x02 => wr32(&mut m.ewram[..], (a as usize) & (EWRAM_SIZE - 4), v),
            0x03 => wr32(&mut m.iwram[..], (a as usize) & (IWRAM_SIZE - 4), v),
            _ => self.st32_slow(a, v),
        }
    }

    #[cold]
    #[inline(never)]
    fn st32_slow(&mut self, a: u32, v: u32) {
        match a >> 24 {
            0x04 => {
                self.io_write16(a, v as u16);
                self.io_write16(a + 2, (v >> 16) as u16);
            }
            0x05 => wr32(&mut self.mem.palette[..], (a as usize) & (PALETTE_SIZE - 4), v),
            0x06 => wr32(&mut self.mem.vram[..], Memory::vram_offset(a) & !3, v),
            0x07 => wr32(&mut self.mem.oam[..], (a as usize) & (OAM_SIZE - 4), v),
            0x0E | 0x0F => self.mem.sram[(a as usize) & (SRAM_SIZE - 1)] = v as u8,
            _ => self.io.bad_accesses += 1,
        }
    }

    #[inline(always)]
    pub fn st16(&mut self, a: u32, v: u32) {
        let a = a & !1;
        let m = &mut self.mem;
        match a >> 24 {
            0x02 => wr16(&mut m.ewram[..], (a as usize) & (EWRAM_SIZE - 2), v),
            0x03 => wr16(&mut m.iwram[..], (a as usize) & (IWRAM_SIZE - 2), v),
            _ => self.st16_slow(a, v),
        }
    }

    #[cold]
    #[inline(never)]
    fn st16_slow(&mut self, a: u32, v: u32) {
        match a >> 24 {
            0x04 => self.io_write16(a, v as u16),
            0x05 => wr16(&mut self.mem.palette[..], (a as usize) & (PALETTE_SIZE - 2), v),
            0x06 => wr16(&mut self.mem.vram[..], Memory::vram_offset(a) & !1, v),
            0x07 => wr16(&mut self.mem.oam[..], (a as usize) & (OAM_SIZE - 2), v),
            0x0E | 0x0F => self.mem.sram[(a as usize) & (SRAM_SIZE - 1)] = (v >> ((a & 1) * 8)) as u8,
            _ => self.io.bad_accesses += 1,
        }
    }

    #[inline(always)]
    pub fn st8(&mut self, a: u32, v: u32) {
        let m = &mut self.mem;
        match a >> 24 {
            0x02 => m.ewram[(a as usize) & (EWRAM_SIZE - 1)] = v as u8,
            0x03 => m.iwram[(a as usize) & (IWRAM_SIZE - 1)] = v as u8,
            _ => self.st8_slow(a, v),
        }
    }

    #[cold]
    #[inline(never)]
    fn st8_slow(&mut self, a: u32, v: u32) {
        let b = v as u8 as u32;
        match a >> 24 {
            0x04 => {
                let old = self.io_peek16(a & !1) as u32;
                let shift = (a & 1) * 8;
                let merged = (old & !(0xFF << shift)) | (b << shift);
                self.io_write16(a & !1, merged as u16);
            }
            0x05 => wr16(&mut self.mem.palette[..], (a as usize) & (PALETTE_SIZE - 2), b * 0x0101),
            0x06 => {
                // Byte writes reach BG VRAM only (as both bytes of the halfword).
                let o = Memory::vram_offset(a);
                if o < 0x10000 {
                    wr16(&mut self.mem.vram[..], o & !1, b * 0x0101);
                }
            }
            0x07 => {}
            0x0E | 0x0F => self.mem.sram[(a as usize) & (SRAM_SIZE - 1)] = b as u8,
            _ => self.io.bad_accesses += 1,
        }
    }
}

/// What mGBA returns for reads past the end of the ROM.
fn rom_open_bus(a: u32) -> u32 {
    (a >> 1) & 0xFFFF
}
