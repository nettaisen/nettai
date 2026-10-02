//! Extract the game's battle assets from the original ROMs, the US Falzar
//! (`MEGAMAN6_FXXBR6E`) and the US Gregar (`MEGAMAN6_GXXBR5E`), into a
//! content pack:
//!
//!     bn6-extract content <falzar-rom> <gregar-rom> <pack-dir> [--content <dir>]
//!
//! Most of the pack is the Falzar ROM's. The Gregar ROM gives what only it
//! has right (Gregar's own faces, five chips' pictures and icons) and what a
//! Gregar console shows of its own (its Crosses' names on the custom
//! screen, its Beast; nettai-assets `Versioned`): see `gregar`.
//!
//! The pack (see nettai-content and docs/design/content-pack.md) holds the
//! graphics and the sound in open formats, by the names this repository's
//! content/bn6 gives them (its compat/assets.toml); the battle content is
//! content/bn6's definitions, which name them. Everything that plays BN6
//! loads the two. The pack is the game's own data: write it outside
//! version control (data/content/ is ignored).

mod content;
mod custom;
mod graphics;
mod gregar;
mod hud;

pub(crate) struct Rom(Vec<u8>);

impl Rom {
    fn u8(&self, a: u32) -> u8 {
        self.0[(a & 0x01FF_FFFF) as usize]
    }
    fn u16(&self, a: u32) -> u16 {
        u16::from_le_bytes([self.u8(a), self.u8(a + 1)])
    }
    fn bytes(&self, a: u32, n: usize) -> &[u8] {
        let o = (a & 0x01FF_FFFF) as usize;
        &self.0[o..o + n]
    }
}

pub(crate) fn u32at(rom: &Rom, a: u32) -> u32 {
    u32::from_le_bytes(rom.bytes(a, 4).try_into().unwrap())
}

/// GBA BIOS LZ77 (type 0x10) decompression.
pub(crate) fn lz77(rom: &Rom, src: u32) -> Option<Vec<u8>> {
    let hdr = u32at(rom, src);
    if hdr & 0xFF != 0x10 {
        return None;
    }
    let size = (hdr >> 8) as usize;
    let mut out = Vec::with_capacity(size);
    let mut o = src + 4;
    while out.len() < size {
        let flags = rom.u8(o);
        o += 1;
        for bit in 0..8 {
            if out.len() >= size {
                break;
            }
            if flags & (0x80 >> bit) != 0 {
                let (b1, b2) = (rom.u8(o) as usize, rom.u8(o + 1) as usize);
                o += 2;
                let n = (b1 >> 4) + 3;
                let disp = ((b1 & 0xF) << 8 | b2) + 1;
                for _ in 0..n {
                    let v = *out.get(out.len().checked_sub(disp)?)?;
                    out.push(v);
                }
            } else {
                out.push(rom.u8(o));
                o += 1;
            }
        }
    }
    Some(out)
}

/// The two ROMs the pack is made from, each checked by its header (its
/// game code at 0xAC: `BR6E` the US Falzar, `BR5E` the US Gregar).
pub(crate) struct Roms {
    pub falzar: Rom,
    pub gregar: Rom,
}

/// What a ROM file is, by its header: the US Falzar or the US Gregar, or
/// what it is instead.
fn identify(path: &str) -> Result<(bool, Rom), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let code = bytes.get(0xAC..0xB0).map(|c| String::from_utf8_lossy(c).into_owned()).unwrap_or_default();
    let title = bytes.get(0xA0..0xAC).map(|c| String::from_utf8_lossy(c).into_owned()).unwrap_or_default();
    match code.as_str() {
        "BR6E" => Ok((true, Rom(bytes))),
        "BR5E" => Ok((false, Rom(bytes))),
        "BR6J" | "BR5J" => Err(format!("{path}: the Japanese {title} ({code}); the pack is made from the US ROMs (BR6E, BR5E)")),
        _ => Err(format!("{path}: not a BN6 US ROM (its header says {title:?}, game code {code:?}; expected BR6E or BR5E)")),
    }
}

/// The US Falzar and US Gregar ROMs, given in that order; either order is
/// taken, but not two of one version.
pub(crate) fn load_roms(first: &str, second: &str) -> Result<Roms, String> {
    let (a_falzar, a) = identify(first)?;
    let (b_falzar, b) = identify(second)?;
    match (a_falzar, b_falzar) {
        (true, false) => Ok(Roms { falzar: a, gregar: b }),
        (false, true) => Ok(Roms { falzar: b, gregar: a }),
        (true, true) => Err(format!("{first} and {second} are both the US Falzar ROM (BR6E); give the US Gregar ROM (BR5E) too")),
        (false, false) => Err(format!("{first} and {second} are both the US Gregar ROM (BR5E); give the US Falzar ROM (BR6E) too")),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("content") => content::main(&args[1..]),
        _ => {
            eprintln!("usage: bn6-extract content <falzar-rom> <gregar-rom> <pack-dir> [--content <dir>]");
            std::process::exit(2);
        }
    }
}
