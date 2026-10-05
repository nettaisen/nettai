//! An EXE5 save file (the .sav an emulator keeps), and what nettai reads of
//! it for a player's setup: the version, the light/dark value, the souls it
//! has, its NaviCust (the list, the compression flags, whether the compile
//! leaves the HP), its patch cards and its computer-navi data.
//!
//! The file holds the save image at 0x100: 0x7C14 bytes, the game's EWRAM
//! from 0x02000000 as the game saves it (an address's offset in the image
//! is its offset from 0x02000000), each byte XORed with the low byte of the
//! mask word at 0x1A34 (the word itself stored plain). The word at 0x1A30
//! is 0 in a save the game wrote; the game's name is at 0x29E0; the word at
//! 0x29DC is the checksum: the image's bytes summed, less the checksum's
//! own, plus 0x72 for Team ProtoMan and 0x18 for Team Colonel. (The format
//! as Tango's save support reads it; Tango's raw netplay saves are such an
//! image, unmasked: [`Save::from_image`].)
//!
//! The event flags are at 0x029F8 (the toolkit's +0x44: 0x02002940, the
//! game state's block, + 0xB8; flag `f` is bit `0x80 >> (f & 7)` of byte
//! `f >> 3`), MegaMan's NaviStats at 0x52A8 (0x60 bytes: the light/dark
//! value at +0x44), the NaviCust's list at 0x4D6C (25 parts of 8 bytes) and
//! its grid at 0x4D48, the patch cards' count at 0x79A0 and their list at
//! 0x79D0 (a byte each: the card, bit 7 switched off), the area at 0x2944
//! (the game state's +4: the cyberworld's from 0x80), the key items' counts
//! at 0x3DB0 (the toolkit's +0x50, a byte an item: ExpMemry's, item 0x61,
//! is the NaviCust board's expansions), the computer-navi data at 0x554C
//! (the toolkit's +0x78: seven blocks of 0xE0 bytes, the player's the
//! first; docs/design/exe5-map.md §15.9).

use crate::Version;

/// Where the image starts in the file, and its size.
const IMAGE_START: usize = 0x100;
pub const IMAGE_SIZE: usize = 0x7C14;
/// The mask word, the shift word, the checksum and the game's name.
const MASK: usize = 0x1A34;
const SHIFT: usize = 0x1A30;
const CHECKSUM: usize = 0x29DC;
const GAME_NAME: usize = 0x29E0;
const EVENT_FLAGS: usize = 0x29F8;
/// MegaMan's NaviStats, and the light/dark value in it.
const NAVI_STATS: usize = 0x52A8;
const LIGHT_DARK: usize = 0x44;
/// The NaviCust's list (`NAVICUST_PARTS` parts of 8 bytes), the patch cards'
/// count and list, the area.
const NAVICUST: usize = 0x4D6C;
const CARD_COUNT: usize = 0x79A0;
const CARDS: usize = 0x79D0;
const AREA: usize = 0x2944;
/// The key items' counts (a byte an item, which 0x0803C120 reads), and
/// ExpMemry among them: the NaviCust board's expansions, which the NaviCust
/// screen reads as it opens (0x08132928) to pick its board (0x0813F138).
const KEY_ITEMS: usize = 0x3DB0;
pub const EXP_MEMORY: u8 = 0x61;

/// The player's computer-navi data: the first of seven blocks, which a
/// battle's end writes from what the save has learned (0x0802C540) and a
/// battle's start sends (0x08009B64).
const COMPUTER_NAVI: usize = 0x554C;
/// The block's size, its places (a halfword each), and its pattern records
/// (16 bytes each from +0x58: `dx`, `dy`, five chip places, the score).
/// Between them, at +0x54, a count that only the send writes; after the
/// records, eight bytes nothing reads.
pub const COMPUTER_NAVI_SIZE: usize = 0xE0;
pub const COMPUTER_NAVI_PLACES: usize = 42;
pub const COMPUTER_NAVI_PATTERNS: usize = 8;
pub const COMPUTER_NAVI_PATTERN_CHIPS: usize = 5;
const COMPUTER_NAVI_COUNT: usize = 0x54;
const COMPUTER_NAVI_RECORDS: usize = 0x58;
/// An empty place (of the block's 42, or of a record's five), and what
/// marks a place of the 42 as a pattern record's number.
pub const COMPUTER_NAVI_EMPTY: u16 = 0xFFFF;
pub const COMPUTER_NAVI_PATTERN: u16 = 0x8000;

/// A pattern record of a computer-navi data block, as it is: its place
/// from a target (signed bytes), its five chip places (each a chip's
/// number, 0, or 0xFFFF, empty: as written the AI reads them to the
/// first 0xFFFF, and on past the fifth; no battle gets to that read,
/// docs/design/exe5-map.md §15.9) and its score (the word at +12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputerNaviPattern {
    pub dx: i8,
    pub dy: i8,
    pub chips: [u16; COMPUTER_NAVI_PATTERN_CHIPS],
    pub score: u32,
}

impl ComputerNaviPattern {
    /// The record in `bytes`.
    pub fn read(bytes: &[u8; 16]) -> ComputerNaviPattern {
        let chips = std::array::from_fn(|k| u16::from_le_bytes([bytes[2 + k * 2], bytes[3 + k * 2]]));
        let score = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        ComputerNaviPattern { dx: bytes[0] as i8, dy: bytes[1] as i8, chips, score }
    }

    /// The record's bytes.
    pub fn bytes(&self) -> [u8; 16] {
        let mut out = [0; 16];
        (out[0], out[1]) = (self.dx as u8, self.dy as u8);
        for (k, chip) in self.chips.iter().enumerate() {
            out[2 + k * 2..4 + k * 2].copy_from_slice(&chip.to_le_bytes());
        }
        out[12..].copy_from_slice(&self.score.to_le_bytes());
        out
    }
}

/// A computer-navi data block by number, as it is but for its count and its
/// last eight bytes: its 42 places in order, each a chip's number, 0,
/// 0x8000 with a pattern record's number (from 0), or 0xFFFF (empty), and
/// its eight pattern records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputerNaviBlock {
    pub places: [u16; COMPUTER_NAVI_PLACES],
    pub patterns: [ComputerNaviPattern; COMPUTER_NAVI_PATTERNS],
}

impl ComputerNaviBlock {
    /// The block in `bytes` (0xE0 of them).
    pub fn read(bytes: &[u8]) -> Result<ComputerNaviBlock, String> {
        if bytes.len() != COMPUTER_NAVI_SIZE {
            return Err(format!("a computer-navi data block is {COMPUTER_NAVI_SIZE:#x} bytes, not {:#x}", bytes.len()));
        }
        let half = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]);
        let pattern = |i: usize| {
            let at = COMPUTER_NAVI_RECORDS + i * 16;
            ComputerNaviPattern::read(bytes[at..at + 16].try_into().expect("a record of the block"))
        };
        Ok(ComputerNaviBlock { places: std::array::from_fn(|i| half(i * 2)), patterns: std::array::from_fn(pattern) })
    }

    /// The block's bytes as a save holds them: its count 0xFFFFFFFF (the
    /// send writes it) and its last eight bytes 0xFF.
    pub fn bytes(&self) -> [u8; COMPUTER_NAVI_SIZE] {
        let mut out = [0xFF; COMPUTER_NAVI_SIZE];
        for (i, place) in self.places.iter().enumerate() {
            out[i * 2..i * 2 + 2].copy_from_slice(&place.to_le_bytes());
        }
        out[COMPUTER_NAVI_COUNT..COMPUTER_NAVI_RECORDS].fill(0xFF);
        for (i, p) in self.patterns.iter().enumerate() {
            out[COMPUTER_NAVI_RECORDS + i * 16..COMPUTER_NAVI_RECORDS + (i + 1) * 16].copy_from_slice(&p.bytes());
        }
        out
    }
}

/// The NaviCust list's room.
pub const NAVICUST_PARTS: usize = 25;
/// A part is compressed by its event flag (this + the part id, which the
/// shape's lookup, 0x0813EEFC, tests).
pub const COMPRESSED_FLAG: u16 = 0x1EC0;
/// The flag that keeps the NaviCust's HP routine (0x0803C13C) from running,
/// as the cyberworld does.
pub const KEEPS_HP_FLAG: u16 = 0x10B2;

/// The soul button (Soul Unison, event flag 0), and Chaos Unison (0x236:
/// the soul button's check of a dark chip, 0x08024B7E).
pub const SOUL_UNISON_FLAG: u16 = 0;
pub const CHAOS_UNISON_FLAG: u16 = 0x236;
/// EXE5's souls by number (1 to 12, NaviStats +0x2C).
pub const SOULS: std::ops::RangeInclusive<u8> = 1..=12;

/// An EXE5 save, its image unmasked.
pub struct Save {
    image: Box<[u8]>,
    version: Version,
    japanese: bool,
}

impl std::fmt::Debug for Save {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Save").field("version", &self.version).field("japanese", &self.japanese).finish_non_exhaustive()
    }
}

impl Save {
    /// The save in `file` (a .sav's bytes), checked: its game's name, the
    /// shift word and the checksum.
    pub fn read(file: &[u8]) -> Result<Save, String> {
        let mut image: Box<[u8]> = file
            .get(IMAGE_START..IMAGE_START + IMAGE_SIZE)
            .ok_or_else(|| format!("an EXE5 save is {:#x} bytes or more, not {:#x}", IMAGE_START + IMAGE_SIZE, file.len()))?
            .into();
        let mask_word: [u8; 4] = image[MASK..MASK + 4].try_into().expect("four bytes");
        for b in image.iter_mut() {
            *b ^= mask_word[0];
        }
        image[MASK..MASK + 4].copy_from_slice(&mask_word);
        let save = Save::from_image(&image)?;
        let word = |at: usize| u32::from_le_bytes(save.image[at..at + 4].try_into().expect("four bytes"));
        let sum: u32 = save.image.iter().map(|&b| b as u32).sum::<u32>() - save.image[CHECKSUM..CHECKSUM + 4].iter().map(|&b| b as u32).sum::<u32>();
        let expected = sum
            + match save.version {
                Version::Protoman => 0x72,
                Version::Colonel => 0x18,
            };
        if word(CHECKSUM) != expected {
            return Err(format!("the save's checksum is {:#x}, its bytes give {expected:#x}: a damaged save", word(CHECKSUM)));
        }
        Ok(save)
    }

    /// A save image, unmasked (Tango's raw netplay saves): its game's name
    /// and the shift word checked, not its checksum.
    pub fn from_image(image: &[u8]) -> Result<Save, String> {
        let image: Box<[u8]> = image.get(..IMAGE_SIZE).ok_or_else(|| format!("an EXE5 save image is {IMAGE_SIZE:#x} bytes, not {:#x}", image.len()))?.into();
        let word = |at: usize| u32::from_le_bytes(image[at..at + 4].try_into().expect("four bytes"));
        if word(SHIFT) != 0 {
            return Err(format!("the save's shift word is {:#x}, not 0: not a save the game wrote", word(SHIFT)));
        }
        let (version, japanese) = match &image[GAME_NAME..GAME_NAME + 20] {
            b"REXE5TOB 20041006 US" => (Version::Protoman, false),
            b"REXE5TOK 20041006 US" => (Version::Colonel, false),
            b"REXE5TOB 20041104 JP" => (Version::Protoman, true),
            b"REXE5TOK 20041104 JP" => (Version::Colonel, true),
            name => return Err(format!("not an EXE5 save (its game is {:?})", String::from_utf8_lossy(name))),
        };
        Ok(Save { image, version, japanese })
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn japanese(&self) -> bool {
        self.japanese
    }

    /// Event flag `flag`.
    pub fn event_flag(&self, flag: u16) -> bool {
        self.image[EVENT_FLAGS + (flag >> 3) as usize] & (0x80 >> (flag & 7)) != 0
    }

    /// MegaMan's light/dark value (NaviStats +0x44: 0 to 1000).
    pub fn light_dark(&self) -> u16 {
        u16::from_le_bytes([self.image[NAVI_STATS + LIGHT_DARK], self.image[NAVI_STATS + LIGHT_DARK + 1]])
    }

    /// The souls the save has, by number, as the soul button checks them
    /// (0x08024B28): those of its version whose event flag is set (the
    /// other version's never, whatever the flags say).
    pub fn souls(&self) -> Vec<u8> {
        SOULS.filter(|&n| self.version.soul_flag(n).is_some_and(|f| self.event_flag(f))).collect()
    }

    /// The save has Soul Unison (the button) and Chaos Unison.
    pub fn soul_unison(&self) -> bool {
        self.event_flag(SOUL_UNISON_FLAG)
    }

    pub fn chaos_unison(&self) -> bool {
        self.event_flag(CHAOS_UNISON_FLAG)
    }

    /// The unmasked image.
    pub fn image(&self) -> &[u8] {
        &self.image
    }

    /// MegaMan's NaviStats block.
    pub fn navi_stats(&self) -> [u8; crate::codec::NAVI_STATS] {
        self.image[NAVI_STATS..NAVI_STATS + crate::codec::NAVI_STATS].try_into().expect("a NaviStats block")
    }

    /// The NaviCust's list (`trace::navicust` reads it).
    pub fn navicust_list(&self) -> &[u8] {
        &self.image[NAVICUST..NAVICUST + NAVICUST_PARTS * 8]
    }

    /// The player's computer-navi data: what a computer navi plays from
    /// this save.
    pub fn computer_navi(&self) -> ComputerNaviBlock {
        ComputerNaviBlock::read(&self.image[COMPUTER_NAVI..COMPUTER_NAVI + COMPUTER_NAVI_SIZE]).expect("a block of the image")
    }

    /// How many of key item `item` the save has.
    pub fn key_item(&self, item: u8) -> u8 {
        self.image[KEY_ITEMS + item as usize]
    }

    /// The NaviCust board's expansions: the ExpMemry the save has (none a
    /// 4x4 board, one 5x4, two 5x5; a finished save has both).
    pub fn expansions(&self) -> u8 {
        self.key_item(EXP_MEMORY)
    }

    /// Whether the NaviCust compresses part `part` (its event flag).
    pub fn compressed(&self, part: u8) -> bool {
        self.event_flag(COMPRESSED_FLAG + part as u16)
    }

    /// Whether the NaviCust's compile leaves the HP (the EXE5 navicust
    /// system's setup `cyberworld`): the save is in the cyberworld (its
    /// area from 0x80), or has flag 0x10B2.
    pub fn cyberworld(&self) -> bool {
        self.image[AREA] >= 0x80 || self.event_flag(KEEPS_HP_FLAG)
    }

    /// The patch cards installed, in order: each card's number and whether
    /// it is switched on.
    pub fn patch_cards(&self) -> Vec<(u8, bool)> {
        let n = self.image[CARD_COUNT] as usize;
        self.image[CARDS..CARDS + n.min(0x30)].iter().map(|&b| (b & 0x7F, b & 0x80 == 0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up image of `version`: its name, the soul flags `flags` set,
    /// the light/dark value `value`.
    fn image(version: Version, flags: &[u16], value: u16) -> Vec<u8> {
        let mut image = vec![0u8; IMAGE_SIZE];
        let name: &[u8; 20] = match version {
            Version::Protoman => b"REXE5TOB 20041006 US",
            Version::Colonel => b"REXE5TOK 20041006 US",
        };
        image[GAME_NAME..GAME_NAME + 20].copy_from_slice(name);
        for &f in flags {
            image[EVENT_FLAGS + (f >> 3) as usize] |= 0x80 >> (f & 7);
        }
        image[NAVI_STATS + LIGHT_DARK..NAVI_STATS + LIGHT_DARK + 2].copy_from_slice(&value.to_le_bytes());
        image
    }

    /// A save's computer-navi data is the first block at 0x554C, read as it
    /// is: its places by number, and each pattern record's place, five chip
    /// places (a chip's number, 0 or 0xFFFF) and score; and written back,
    /// the same bytes.
    #[test]
    fn a_saves_computer_navi_data_is_its_first_block() {
        let mut image = image(Version::Colonel, &[], 500);
        // Two blocks: the player's, and the next (which nothing reads).
        image[COMPUTER_NAVI..COMPUTER_NAVI + 2 * COMPUTER_NAVI_SIZE].fill(0xFF);
        let put = |image: &mut [u8], at: usize, halves: &[u16]| {
            for (i, h) in halves.iter().enumerate() {
                image[COMPUTER_NAVI + at + i * 2..COMPUTER_NAVI + at + i * 2 + 2].copy_from_slice(&h.to_le_bytes());
            }
        };
        put(&mut image, 6, &[0x51, 0x51, 0, 0xA2]);
        put(&mut image, 66, &[0x8000, 0x8001]);
        put(&mut image, 82, &[0x157]);
        // Record 0: 3 columns short, 1 row down, chips 0xC9 and 0x28, score 7.
        image[COMPUTER_NAVI + 0x58..COMPUTER_NAVI + 0x5A].copy_from_slice(&[0xFD, 0x01]);
        put(&mut image, 0x5A, &[0xC9, 0x28]);
        put(&mut image, 0x64, &[7, 0]);
        // Record 1: a full one, a 0 among its chips, its score a long word.
        image[COMPUTER_NAVI + 0x68..COMPUTER_NAVI + 0x6A].copy_from_slice(&[0xFF, 0xFE]);
        put(&mut image, 0x6A, &[1, 2, 0, 4, 5, 0x000A, 0x0001]);
        // Record 2: zeroed, as the game's write leaves one its learning hasn't filled.
        image[COMPUTER_NAVI + 0x78..COMPUTER_NAVI + 0x88].fill(0);
        put(&mut image, COMPUTER_NAVI_SIZE, &[0x99]);
        let block = Save::from_image(&image).unwrap().computer_navi();
        let mut places = [0xFFFFu16; COMPUTER_NAVI_PLACES];
        (places[3], places[4], places[5], places[6], places[33], places[34], places[41]) = (0x51, 0x51, 0, 0xA2, 0x8000, 0x8001, 0x157);
        assert_eq!(block.places, places);
        let none = COMPUTER_NAVI_EMPTY;
        assert_eq!(block.patterns[0], ComputerNaviPattern { dx: -3, dy: 1, chips: [0xC9, 0x28, none, none, none], score: 7 });
        assert_eq!(block.patterns[1], ComputerNaviPattern { dx: -1, dy: -2, chips: [1, 2, 0, 4, 5], score: 0x0001_000A });
        assert_eq!(block.patterns[2], ComputerNaviPattern { dx: 0, dy: 0, chips: [0; 5], score: 0 });
        assert_eq!(block.patterns[3], ComputerNaviPattern { dx: -1, dy: -1, chips: [none; 5], score: 0xFFFF_FFFF });
        assert_eq!(block.bytes()[..], image[COMPUTER_NAVI..COMPUTER_NAVI + COMPUTER_NAVI_SIZE]);
        assert_eq!(ComputerNaviBlock::read(&block.bytes()), Ok(block));
        assert!(ComputerNaviBlock::read(&[0; 4]).is_err());
    }

    /// A save's souls are its version's whose flags are set: flags 2 to 7
    /// are Team ProtoMan's souls 1 to 6, 8 to 0x0D Team Colonel's 7 to 12,
    /// and a version never has the other's.
    #[test]
    fn a_saves_souls_are_its_versions() {
        let all: Vec<u16> = (0..=13).chain([CHAOS_UNISON_FLAG]).collect();
        let s = Save::from_image(&image(Version::Protoman, &all, 1000)).unwrap();
        assert_eq!(s.souls(), [1, 2, 3, 4, 5, 6]);
        assert!(s.soul_unison() && s.chaos_unison());
        assert_eq!(s.light_dark(), 1000);
        let s = Save::from_image(&image(Version::Colonel, &[0, 9, 13], 0)).unwrap();
        assert_eq!(s.souls(), [8, 12]);
        assert!(!s.chaos_unison());
        assert_eq!(s.light_dark(), 0);
    }

    /// The NaviCust board's expansions are the save's ExpMemry (key item
    /// 0x61's count).
    #[test]
    fn a_saves_expansions_are_its_exp_memory() {
        let mut img = image(Version::Protoman, &[], 500);
        assert_eq!(Save::from_image(&img).unwrap().expansions(), 0);
        img[0x3DB0 + 0x61] = 2;
        img[0x3DB0 + 0x60] = 9;
        let s = Save::from_image(&img).unwrap();
        assert_eq!((s.expansions(), s.key_item(0x60)), (2, 9));
    }

    /// A .sav: the image at 0x100, masked, with its checksum; a damaged one
    /// is refused.
    #[test]
    fn a_save_file_unmasks_and_checks() {
        let mut img = image(Version::Colonel, &[0, 8], 480);
        img[MASK..MASK + 4].copy_from_slice(&[0x5A, 1, 2, 3]);
        let sum: u32 = img.iter().map(|&b| b as u32).sum::<u32>() + 0x18;
        img[CHECKSUM..CHECKSUM + 4].copy_from_slice(&sum.to_le_bytes());
        let mut file = vec![0u8; IMAGE_START];
        file.extend(img.iter().enumerate().map(|(i, &b)| if (MASK..MASK + 4).contains(&i) { b } else { b ^ 0x5A }));
        let s = Save::read(&file).unwrap();
        assert_eq!((s.version(), s.souls(), s.light_dark()), (Version::Colonel, vec![7], 480));
        file[IMAGE_START + 0x10] ^= 1;
        assert!(Save::read(&file).unwrap_err().contains("checksum"));
    }
}
