//! The ROM image and the RAM regions whose code is copied out of it.

/// A RAM code region: `len` bytes at `vma` are a copy of the ROM at `lma`.
#[derive(Clone, Copy, Debug)]
pub struct Overlay {
    pub vma: u32,
    pub lma: u32,
    pub len: u32,
}

pub struct Image {
    pub rom: Vec<u8>,
    pub overlays: Vec<Overlay>,
}

pub const ROM_BASE: u32 = 0x0800_0000;

impl Image {
    /// Offset into the ROM backing a code or constant address.
    pub fn offset(&self, addr: u32) -> Option<usize> {
        if (ROM_BASE..ROM_BASE + self.rom.len() as u32).contains(&addr) {
            return Some((addr - ROM_BASE) as usize);
        }
        for o in &self.overlays {
            if addr >= o.vma && addr < o.vma + o.len {
                return Some((addr - o.vma + o.lma - ROM_BASE) as usize);
            }
        }
        None
    }

    pub fn read16(&self, addr: u32) -> Option<u16> {
        let o = self.offset(addr)?;
        Some(u16::from_le_bytes(self.rom.get(o..o + 2)?.try_into().ok()?))
    }

    pub fn read32(&self, addr: u32) -> Option<u32> {
        let o = self.offset(addr)?;
        Some(u32::from_le_bytes(self.rom.get(o..o + 4)?.try_into().ok()?))
    }

}
