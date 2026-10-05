//! The four EXE5 ROMs the pack is made from, checked by their headers, and
//! reading them: words, LZ77-compressed blocks.

/// One ROM image.
pub struct Rom(pub Vec<u8>);

impl Rom {
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
        let hdr = self.u32(src);
        if hdr & 0xFF != 0x10 {
            return None;
        }
        let size = (hdr >> 8) as usize;
        let mut out = Vec::with_capacity(size);
        let mut o = src + 4;
        while out.len() < size {
            let flags = self.u8(o);
            o += 1;
            for bit in 0..8 {
                if out.len() >= size {
                    break;
                }
                if flags & (0x80 >> bit) != 0 {
                    let (b1, b2) = (self.u8(o) as usize, self.u8(o + 1) as usize);
                    o += 2;
                    let n = (b1 >> 4) + 3;
                    let disp = ((b1 & 0xF) << 8 | b2) + 1;
                    for _ in 0..n {
                        let v = *out.get(out.len().checked_sub(disp)?)?;
                        out.push(v);
                    }
                } else {
                    out.push(self.u8(o));
                    o += 1;
                }
            }
        }
        Some(out)
    }
}

/// A ROM's version: Team ProtoMan's or Team Colonel's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    ProtoMan,
    Colonel,
}

impl Version {
    /// The suffix of an asset that differs by version (`-protoman`,
    /// `-colonel`), as EXE6's packs name theirs `-falzar`/`-gregar`.
    pub fn name(self) -> &'static str {
        match self {
            Version::ProtoMan => "protoman",
            Version::Colonel => "colonel",
        }
    }
}

/// The four ROMs: the US Team ProtoMan (`MEGAMAN5_TP_`, BRBE) and Team
/// Colonel (`MEGAMAN5_TC_`, BRKE), the Japanese Team of Blues
/// (`ROCKEXE5_TOB`, BRBJ) and Team of Colonel (`ROCKEXE5_TOC`, BRKJ).
pub struct Roms {
    pub protoman: Rom,
    pub colonel: Rom,
    pub protoman_jp: Rom,
    pub colonel_jp: Rom,
}

impl Roms {
    pub fn us(&self, v: Version) -> &Rom {
        match v {
            Version::ProtoMan => &self.protoman,
            Version::Colonel => &self.colonel,
        }
    }

    pub fn jp(&self, v: Version) -> &Rom {
        match v {
            Version::ProtoMan => &self.protoman_jp,
            Version::Colonel => &self.colonel_jp,
        }
    }
}

/// The ROMs in the order the command takes them: each one's game code and
/// what it is.
pub const ROM_ORDER: [(&str, &str); 4] = [
    ("BRBE", "the US Team ProtoMan ROM (MEGAMAN5_TP_, BRBE)"),
    ("BRKE", "the US Team Colonel ROM (MEGAMAN5_TC_, BRKE)"),
    ("BRBJ", "the Japanese Team of Blues ROM (ROCKEXE5_TOB, BRBJ)"),
    ("BRKJ", "the Japanese Team of Colonel ROM (ROCKEXE5_TOC, BRKJ)"),
];

/// What a ROM file's header says it is: its place in `ROM_ORDER` (`None`
/// for none of the four), its game code and its title.
fn identify(bytes: &[u8]) -> (Option<usize>, String, String) {
    let text = |r: std::ops::Range<usize>| bytes.get(r).map(|c| String::from_utf8_lossy(c).into_owned()).unwrap_or_default();
    let (code, title) = (text(0xAC..0xB0), text(0xA0..0xAC));
    (ROM_ORDER.iter().position(|(c, _)| *c == code), code, title)
}

/// The four ROMs, in `ROM_ORDER`: each must be the one its place names, or
/// the error says which file is which.
pub fn load(paths: [&str; 4]) -> Result<Roms, String> {
    let mut roms = Vec::with_capacity(4);
    for (place, path) in paths.into_iter().enumerate() {
        let wanted = ROM_ORDER[place].1;
        let bytes = std::fs::read(path).map_err(|e| format!("{path} (given as {wanted}): {e}"))?;
        match identify(&bytes) {
            (Some(p), ..) if p == place => roms.push(Rom(bytes)),
            (Some(p), ..) => {
                return Err(format!(
                    "{path} is {}, given as {wanted}: the ROMs go in this order: {}",
                    ROM_ORDER[p].1,
                    ROM_ORDER.iter().map(|(_, what)| *what).collect::<Vec<_>>().join(", ")
                ));
            }
            (None, code, title) => {
                return Err(format!(
                    "{path}, given as {wanted}, is no EXE5 ROM it takes: its header says {title:?}, game code {code:?} (expected {})",
                    ROM_ORDER[place].0
                ));
            }
        }
    }
    let [protoman, colonel, protoman_jp, colonel_jp]: [Rom; 4] = roms.try_into().unwrap_or_else(|_| unreachable!());
    Ok(Roms { protoman, colonel, protoman_jp, colonel_jp })
}
