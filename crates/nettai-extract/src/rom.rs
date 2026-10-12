//! Shared ROM access and GBA compression. An empty image represents an absent source;
//! callers must test `is_present` before reading it.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// One ROM image.
#[derive(Default)]
pub(crate) struct Rom(pub Vec<u8>);

/// A range of a ROM the extraction read while reads were recorded
/// (`record_reads`), in the order read: the ROM (its image's place in
/// memory), the address, the length, and how it was read.
pub(crate) struct Read {
    pub rom: usize,
    pub address: u32,
    pub len: u32,
    pub kind: ReadKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadKind {
    /// Bytes the extraction decodes.
    Data,
    /// A word that is a ROM address (0x08000000..0x0A000000), which a
    /// table or a record holds: what is read at it is its target's.
    Pointer(u32),
    /// A span read whole for a parser that takes what it needs of it (an
    /// uncompressed sprite archive): how far the parser went isn't known.
    Open,
}

static RECORDING: AtomicBool = AtomicBool::new(false);
static READS: Mutex<Vec<Read>> = Mutex::new(Vec::new());

/// Run `f` with every read of every ROM recorded, and give the reads with
/// its result: what a cross-ROM comparison needs to look where the
/// extraction looks. Reads of bytes that follow one another are one read.
pub(crate) fn record_reads<T>(f: impl FnOnce() -> T) -> (T, Vec<Read>) {
    READS.lock().unwrap().clear();
    RECORDING.store(true, Ordering::Relaxed);
    let out = f();
    RECORDING.store(false, Ordering::Relaxed);
    (out, std::mem::take(&mut *READS.lock().unwrap()))
}

impl Rom {
    pub fn is_present(&self) -> bool {
        !self.0.is_empty()
    }

    fn note(&self, a: u32, n: usize, kind: ReadKind) {
        if !RECORDING.load(Ordering::Relaxed) {
            return;
        }
        let (rom, address, len) = (self.0.as_ptr() as usize, 0x0800_0000 | (a & 0x01FF_FFFF), n as u32);
        let mut reads = READS.lock().unwrap();
        if kind == ReadKind::Data
            && let Some(last) = reads.last_mut()
            && last.kind == ReadKind::Data
            && last.rom == rom
            && last.address + last.len == address
        {
            last.len += len;
            return;
        }
        reads.push(Read { rom, address, len, kind });
    }

    pub fn u8(&self, a: u32) -> u8 {
        self.note(a, 1, ReadKind::Data);
        self.0[(a & 0x01FF_FFFF) as usize]
    }

    pub fn u16(&self, a: u32) -> u16 {
        u16::from_le_bytes([self.u8(a), self.u8(a + 1)])
    }

    pub fn u32(&self, a: u32) -> u32 {
        let o = (a & 0x01FF_FFFF) as usize;
        let v = u32::from_le_bytes(self.0[o..o + 4].try_into().unwrap());
        let kind = if (0x0800_0000..0x0A00_0000).contains(&(v & 0x7FFF_FFFF)) { ReadKind::Pointer(v) } else { ReadKind::Data };
        self.note(a, 4, kind);
        v
    }

    pub fn bytes(&self, a: u32, n: usize) -> &[u8] {
        self.note(a, n, ReadKind::Data);
        let o = (a & 0x01FF_FFFF) as usize;
        &self.0[o..o + n]
    }

    /// Up to `max` bytes from `a` (fewer at the ROM's end).
    pub fn from(&self, a: u32, max: usize) -> &[u8] {
        let o = (a & 0x01FF_FFFF) as usize;
        let end = (o + max).min(self.0.len());
        self.note(a, end - o, ReadKind::Open);
        &self.0[o..end]
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
        self.note(src, o, ReadKind::Data);
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
    fn reads_are_recorded_with_what_a_pointer_word_holds() {
        // A pointer word, two bytes read one after the other, an LZ77
        // stream (its length is what the decoder took) and an open span.
        let mut bytes = vec![0x10, 0, 0, 0x08, 7, 9, 0, 0];
        bytes.extend([0x10, 4, 0, 0, 0x40, b'x', 0xf0, 0]);
        let rom = Rom(bytes);
        let (_, reads) = record_reads(|| {
            assert_eq!(rom.u32(0x0800_0000), 0x0800_0010);
            assert_eq!((rom.u8(4), rom.u8(5)), (7, 9));
            assert_eq!(rom.lz77(0x0800_0008), Some(b"xxxx".to_vec()));
            rom.from(0x0800_000C, 100);
        });
        // (Other tests' ROMs may be read meanwhile: this ROM's alone.)
        let own: Vec<_> = reads.iter().filter(|r| r.rom == rom.0.as_ptr() as usize).collect();
        assert!(own.iter().any(|r| (r.address, r.len) == (0x0800_0000, 4) && r.kind == ReadKind::Pointer(0x0800_0010)));
        let data: u32 = own.iter().filter(|r| r.kind == ReadKind::Data).map(|r| r.len).sum();
        assert_eq!(data, 2 + 8);
        assert!(own.iter().any(|r| r.address == 0x0800_0008 && r.kind == ReadKind::Data));
        assert!(own.iter().any(|r| (r.address, r.len) == (0x0800_000C, 4) && r.kind == ReadKind::Open));
        // Nothing is recorded outside `record_reads`.
        rom.u8(0);
        assert!(READS.lock().unwrap().iter().all(|r| r.rom != rom.0.as_ptr() as usize));
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
