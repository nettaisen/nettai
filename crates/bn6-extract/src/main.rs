//! Extract the game's battle assets from the original ROMs, the US Falzar
//! (`MEGAMAN6_FXXBR6E`), the US Gregar (`MEGAMAN6_GXXBR5E`), the Japanese
//! Falzar (`ROCKEXE6_RXXBR6J`) and the Japanese Gregar
//! (`ROCKEXE6_GXXBR5J`), into a content pack:
//!
//!     bn6-extract content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> <pack-dir> [--content <dir>]
//!
//! Most of the pack is the US Falzar ROM's. The US Gregar ROM gives what
//! only it has right (Gregar's own faces, five chips' pictures and icons)
//! and what a Gregar console shows of its own (its Crosses' names on the
//! custom screen, its Beast; nettai-assets `Versioned`): see `gregar`. The
//! Japanese ROMs give what the US release cut (the sprites it left a
//! placeholder in, the cut chips' pictures): see `jp`.
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
mod jp;

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

/// The four ROMs the pack is made from, each checked by its header (its
/// game code at 0xAC): the US Falzar (`BR6E`), the US Gregar (`BR5E`), the
/// Japanese Falzar (`BR6J`) and the Japanese Gregar (`BR5J`).
pub(crate) struct Roms {
    pub falzar: Rom,
    pub gregar: Rom,
    pub falzar_jp: Rom,
    pub gregar_jp: Rom,
}

/// The ROMs in the order the command takes them: each one's game code
/// and what it is.
pub(crate) const ROM_ORDER: [(&str, &str); 4] = [
    ("BR6E", "the US Falzar ROM (MEGAMAN6_FXX, BR6E)"),
    ("BR5E", "the US Gregar ROM (MEGAMAN6_GXX, BR5E)"),
    ("BR6J", "the Japanese Falzar ROM (ROCKEXE6_RXX, BR6J)"),
    ("BR5J", "the Japanese Gregar ROM (ROCKEXE6_GXX, BR5J)"),
];

/// What a ROM file's header says it is: its game code (`None` for none of
/// the four) and its title.
fn identify(bytes: &[u8]) -> (Option<usize>, String, String) {
    let text = |r: std::ops::Range<usize>| bytes.get(r).map(|c| String::from_utf8_lossy(c).into_owned()).unwrap_or_default();
    let (code, title) = (text(0xAC..0xB0), text(0xA0..0xAC));
    (ROM_ORDER.iter().position(|(c, _)| *c == code), code, title)
}

/// The four ROMs, in `ROM_ORDER`: each must be the one its place names, or
/// the error says which file is which.
pub(crate) fn load_roms(paths: [&str; 4]) -> Result<Roms, String> {
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
                    "{path}, given as {wanted}, is no BN6 ROM it takes: its header says {title:?}, game code {code:?} (expected {})",
                    ROM_ORDER[place].0
                ));
            }
        }
    }
    let [falzar, gregar, falzar_jp, gregar_jp]: [Rom; 4] = roms.try_into().unwrap_or_else(|_| unreachable!());
    Ok(Roms { falzar, gregar, falzar_jp, gregar_jp })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("content") => content::main(&args[1..]),
        _ => {
            eprintln!("{}", content::USAGE);
            std::process::exit(2);
        }
    }
}
