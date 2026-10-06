//! Shared ROM access and GBA compression. An empty image represents an absent source;
//! callers must test `is_present` before reading it.

/// One ROM image.
#[derive(Default)]
pub(crate) struct Rom(pub Vec<u8>);

impl Rom {
    pub fn is_present(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn u8(&self, a: u32) -> u8 {
        self.0[(a & 0x01FF_FFFF) as usize]
    }

    pub fn u16(&self, a: u32) -> u16 {
        u16::from_le_bytes([self.u8(a), self.u8(a + 1)])
    }

    pub fn u32(&self, a: u32) -> u32 {
        u32::from_le_bytes(self.bytes(a, 4).try_into().unwrap())
    }

    pub fn bytes(&self, a: u32, n: usize) -> &[u8] {
        let o = (a & 0x01FF_FFFF) as usize;
        &self.0[o..o + n]
    }

    /// Up to `max` bytes from `a` (fewer at the ROM's end).
    pub fn from(&self, a: u32, max: usize) -> &[u8] {
        let o = (a & 0x01FF_FFFF) as usize;
        &self.0[o..(o + max).min(self.0.len())]
    }

    /// Whether `a` points into this ROM.
    pub fn contains(&self, a: u32) -> bool {
        (0x0800_0000..0x0800_0000 + self.0.len() as u32).contains(&a)
    }

    /// A GBA BIOS LZ77 (type 0x10) block at `src`, decompressed.
    pub fn lz77(&self, src: u32) -> Option<Vec<u8>> {
        let start = (src & 0x01ff_ffff) as usize;
        let data = self.0.get(start..)?;
        let hdr = u32::from_le_bytes(data.get(..4)?.try_into().ok()?);
        if hdr & 0xff != 0x10 {
            return None;
        }
        let size = (hdr >> 8) as usize;
        let mut out = Vec::with_capacity(size);
        let mut o = 4;
        while out.len() < size {
            let flags = *data.get(o)?;
            o += 1;
            for bit in 0..8 {
                if out.len() >= size {
                    break;
                }
                if flags & (0x80 >> bit) != 0 {
                    let (b1, b2) = (*data.get(o)? as usize, *data.get(o + 1)? as usize);
                    o += 2;
                    let n = (b1 >> 4) + 3;
                    let disp = ((b1 & 0xf) << 8 | b2) + 1;
                    for _ in 0..n.min(size - out.len()) {
                        let v = *out.get(out.len().checked_sub(disp)?)?;
                        out.push(v);
                    }
                } else {
                    out.push(*data.get(o)?);
                    o += 1;
                }
            }
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literals_and_overlapping_back_references() {
        let rom = Rom(vec![0x10, 9, 0, 0, 0x10, b'A', b'B', b'C', 0x30, 2]);
        assert_eq!(rom.lz77(0x0800_0000), Some(b"ABCABCABC".to_vec()));
        let rom = Rom(vec![0x10, 4, 0, 0, 0x40, b'x', 0xf0, 0]);
        assert_eq!(rom.lz77(0), Some(b"xxxx".to_vec()));
    }
    #[test]
    fn bad_streams_fail_without_panicking() {
        for bytes in [
            vec![],
            vec![0x10, 4],
            vec![0x10, 4, 0, 0, 0],
            vec![0x10, 4, 0, 0, 0x80, 0, 0],
            vec![0x11, 0, 0, 0],
        ] {
            let rom = Rom(bytes);
            assert_eq!(rom.lz77(0), None);
            assert_eq!(rom.lz77(0x08ff_ffff), None);
        }
    }
}
