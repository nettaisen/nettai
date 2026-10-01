//! Extract the game's battle content from the original ROM (US Falzar,
//! `MEGAMAN6_FXXBR6E`) into a content pack:
//!
//!     bn6-extract content <rom> <pack-dir> [--overlay <dir>]
//!
//! The pack (see bn6-content and docs/design/content-pack.md) holds the
//! battle data the engine runs on, the graphics and the sound, in open
//! formats, with the scripts of this repository's content/bn6 (the source
//! overlay); everything that plays BN6 loads it. It is the game's own data:
//! write it outside version control (data/content/ is ignored).

mod battle;
mod content;
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

/// The game's text encoding for bytes 0x00-0xDF (from the disassembly's charmap).
pub(crate) const CHARSET: [&str; 0xE0] = [
    "", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E",
    "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U",
    "V", "W", "X", "Y", "Z", "*", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j",
    "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
    "[RV]", "[BX]", "[EX]", "[SP]", "[FZ]", "ウ", "ア", "イ", "オ", "エ", "ケ", "コ", "カ", "ク", "キ", "セ",
    "サ", "ソ", "シ", "ス", "テ", "ト", "ツ", "タ", "チ", "ネ", "ノ", "ヌ", "ナ", "ニ", "ヒ", "ヘ",
    "ホ", "ハ", "フ", "ミ", "マ", "メ", "ム", "モ", "ヤ", "ヨ", "ユ", "ロ", "ル", "リ", "レ", "ラ",
    "ン", "熱", "斗", "ワ", "ヲ", "ギ", "ガ", "ゲ", "ゴ", "グ", "ゾ", "ジ", "ゼ", "ズ", "ザ", "デ",
    "ド", "ヅ", "ダ", "ヂ", "ベ", "ビ", "ボ", "バ", "ブ", "ピ", "パ", "ペ", "プ", "ポ", "ゥ", "ァ",
    "ィ", "ォ", "ェ", "ュ", "ヴ", "ッ", "ョ", "ャ", "-", "×", "=", ":", "%", "?", "+", "█",
    "[bat]", "ー", "!", "&", ",", "゜", ".", "・", ";", "'", "\"", "~", "/", "(", ")", "｢",
    "｣", " ", "_", "[z]", "[L]", "[B]", "[R]", "[A]", "あ", "い", "け", "く", "き", "こ", "か", "せ",
    "そ", "す", "さ", "し", "つ", "と", "て", "た", "ち", "ね", "の", "な", "ぬ", "に", "へ", "ふ",
    "ほ", "は", "ひ", "め", "む", "み", "も", "ま", "ゆ", "よ", "や", "る", "ら", "り", "ろ", "れ",
];

/// Decode string `index` of a text archive (u16 offsets, then strings).
pub(crate) fn archive_string(rom: &Rom, archive: u32, index: u32) -> String {
    let mut a = archive + rom.u16(archive + 2 * index) as u32;
    let mut s = String::new();
    loop {
        let b = rom.u8(a) as usize;
        if b >= CHARSET.len() {
            return s;
        }
        s.push_str(CHARSET[b]);
        a += 1;
    }
}

pub(crate) fn u32at(rom: &Rom, a: u32) -> u32 {
    u32::from_le_bytes(rom.bytes(a, 4).try_into().unwrap())
}

/// IWRAM code/data is a copy of the ROM at 0x081D6000.
pub(crate) fn iwram(a: u32) -> u32 {
    a - 0x0300_5B00 + 0x081D_6000
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
            eprintln!("usage: bn6-extract content <rom> <pack-dir> [--overlay <dir>]");
            std::process::exit(2);
        }
    }
}
