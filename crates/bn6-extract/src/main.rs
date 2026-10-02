//! Extract the game's battle assets from the original ROM (US Falzar,
//! `MEGAMAN6_FXXBR6E`) into a content pack:
//!
//!     bn6-extract content <rom> <pack-dir> [--content <dir>]
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

pub(crate) fn load_rom(path: &str) -> Rom {
    let rom = Rom(std::fs::read(path).expect("reading ROM"));
    assert_eq!(&rom.0[0xA0..0xB0], b"MEGAMAN6_FXXBR6E", "expected the US Falzar ROM (MEGAMAN6_FXXBR6E)");
    rom
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("content") => content::main(&args[1..]),
        _ => {
            eprintln!("usage: bn6-extract content <rom> <pack-dir> [--content <dir>]");
            std::process::exit(2);
        }
    }
}
