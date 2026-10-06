//! An EXE6 save file (the .sav an emulator keeps), and what nettai reads of
//! it for a player's setup: the game, what it unlocks on the custom screen,
//! the navi operated and its navi code's level, the SP navi deletion times,
//! the equipped folder with its Regular and tag chips, the NaviCust, the
//! patch cards, the BugFrags, and the operated navi's NaviStats block (what
//! the save brings to its stats).
//!
//! The file holds the save image at 0x100: 0x6710 bytes, the game's EWRAM
//! from 0x02000000 as the game saves it (an address's offset in the image
//! is its offset from 0x02000000), each byte XORed with the low byte of the
//! mask word at 0x1064 (the word itself stored plain). The word at 0x1060
//! is 0 in a save the game wrote; the game's name is at 0x1C70; the word at
//! 0x1C6C is the checksum: the image's bytes summed, less the checksum's
//! own, plus 0x72 for Gregar and 0x18 for Falzar. The four games (US and
//! Japanese, Gregar and Falzar) keep what is read here at the same places,
//! but that a Japanese image's shop region is 0x40 shorter: what a US one
//! keeps from 0x414C to 0x513C sits 0x40 earlier there (the NaviCust's list,
//! the NaviStats blocks). (The format as Tango's save support reads it;
//! Tango's raw netplay saves are such an image, unmasked:
//! [`Save::from_image`].)

use crate::codec;
use crate::unlocks::Unlocks;
use crate::GameVersion;
use crate::codec::SpTimes;

/// Where the image starts in the file, and its size.
const IMAGE_START: usize = 0x100;
pub const IMAGE_SIZE: usize = 0x6710;
/// The mask word, the shift word, the checksum and the game's name.
const MASK: usize = 0x1064;
const SHIFT: usize = 0x1060;
const CHECKSUM: usize = 0x1C6C;
const GAME_NAME: usize = 0x1C70;

/// The event flags (`oToolkit_EventFlagsPtr`, 0x02001C88): flag `f` is bit
/// `0x80 >> (f & 7)` of byte `f >> 3`.
pub const EVENT_FLAGS: usize = 0x1C88;
/// The navi operated (`GameState+0x01`, 0x02001B81: 0 MegaMan, 1 to 11 the
/// link navis), and the navi code received (`S2001c04+0x30`, 0x02001C34, a
/// word: `0x141 + 15 · navi + level`, `sub_8121198`).
const NAVI: usize = 0x1B81;
const NAVI_CODE: usize = 0x1C34;
/// The navi codes of a navi (`sub_8121198`): one a level, 0 to 14.
const NAVI_CODES: u32 = 15;
/// The SP navi deletion times (0x020018C0, 0x28 bytes), which the init
/// exchange sends (`sub_800B144`, the block's +0x70) and a battle reads at
/// `byte_203EB00`.
const SP_TIMES: usize = 0x18C0;

/// The folders (three of 30 halfwords: the chip in the low 9 bits, the code
/// above; `sub_8133B70`), and the BugFrags (a word, which the game checks
/// against its two mirrors before it trusts it, `sub_8006FD0`).
pub const FOLDERS: usize = 0x2178;
const FOLDER_SIZE: usize = 30;
pub const BUG_FRAGS: usize = 0x1BE0;
/// The key items' counts (a byte an item: ExpMemry's, item 0x71, is the
/// NaviCust board's expansions).
pub const KEY_ITEMS: usize = 0x3134;
pub const EXP_MEMORY: u8 = 0x71;
/// The NaviCust's list (0x31 parts of 8 bytes, at 0x02004190), and the
/// event flags that compress a part (0x2660 + its id, which `sub_813B7A0`
/// reads).
pub const NAVICUST: usize = 0x4190;
pub const NAVICUST_PARTS: usize = 0x31;
const COMPRESSED_FLAG: u16 = 0x2660;
/// The NaviStats blocks (0x64 bytes): MegaMan's, then the link navi's the
/// save operates (`sub_80136CC`'s, by the navi operated).
pub const NAVI_STATS: usize = 0x47CC;
pub const NAVI_STATS_SIZE: usize = 0x64;
/// A block's equipped folder, its Regular chip by folder (an entry, from 0;
/// 30 and on none), and its tag chips by folder (two entries; 0xFF none).
const EQUIPPED_FOLDER: usize = 0x2D;
const REGULAR_CHIPS: usize = 0x2E;
const TAG_CHIPS: usize = 0x56;
/// The patch cards' count and list (a byte a card: its number, bit 7 when
/// switched off).
pub const CARD_COUNT: usize = 0x65F0;
pub const CARDS: usize = 0x6620;
const MAX_CARDS: usize = 0x30;
/// The Japanese images' shorter shop region: from here on (the US's), a
/// Japanese image's bytes sit 0x40 earlier, to its end.
const JP_SHIFTED: std::ops::Range<usize> = 0x414C..0x513C;
const JP_SHIFT: usize = 0x40;

/// Beast Out unlocked; a navi code received; the version's first Cross
/// (`sub_8029EF8`'s table).
const BEAST_OUT_FLAG: u16 = 0xE0;
const NAVI_CODE_FLAG: u16 = 0x163;
const GREGAR_CROSSES: u16 = 0xE2;
const FALZAR_CROSSES: u16 = 0xE7;

/// A game's region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Us,
    Jp,
}

/// An EXE6 save, its image unmasked.
pub struct Save {
    image: Box<[u8]>,
    version: GameVersion,
    region: Region,
}

impl std::fmt::Debug for Save {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Save").field("version", &self.version).field("region", &self.region).finish_non_exhaustive()
    }
}

/// A save's equipped folder, by the original's numbers: its 30 chips (each
/// `code << 9 | chip`), its Regular chip and its tag chips (entries, from 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveFolder {
    pub chips: [u16; FOLDER_SIZE],
    pub regular: Option<u8>,
    pub tags: Option<[u8; 2]>,
}

impl Save {
    /// A save image, unmasked (Tango's raw netplay saves): its game's name
    /// and the shift word checked, not its checksum.
    pub fn from_image(image: &[u8]) -> Result<Save, String> {
        let image: Box<[u8]> = image.get(..IMAGE_SIZE).ok_or_else(|| format!("an EXE6 save image is {IMAGE_SIZE:#x} bytes, not {:#x}", image.len()))?.into();
        let word = |at: usize| u32::from_le_bytes(image[at..at + 4].try_into().expect("four bytes"));
        if word(SHIFT) != 0 {
            return Err(format!("the save's shift word is {:#x}, not 0: not a save the game wrote", word(SHIFT)));
        }
        let (version, region) = game_of(&image)?;
        Ok(Save { image, version, region })
    }

    /// The save in `file` (a .sav's bytes), checked: its game's name, the
    /// shift word and the checksum.
    pub fn read(file: &[u8]) -> Result<Save, String> {
        let mut image: Box<[u8]> = file
            .get(IMAGE_START..IMAGE_START + IMAGE_SIZE)
            .ok_or_else(|| format!("an EXE6 save is {:#x} bytes or more, not {:#x}", IMAGE_START + IMAGE_SIZE, file.len()))?
            .into();
        let mask_word: [u8; 4] = image[MASK..MASK + 4].try_into().expect("four bytes");
        for b in image.iter_mut() {
            *b ^= mask_word[0];
        }
        image[MASK..MASK + 4].copy_from_slice(&mask_word);
        let word = |at: usize| u32::from_le_bytes(image[at..at + 4].try_into().expect("four bytes"));
        if word(SHIFT) != 0 {
            return Err(format!("the save's shift word is {:#x}, not 0: not a save the game wrote", word(SHIFT)));
        }
        let (version, region) = game_of(&image)?;
        let sum: u32 = image.iter().map(|&b| b as u32).sum::<u32>() - image[CHECKSUM..CHECKSUM + 4].iter().map(|&b| b as u32).sum::<u32>();
        let expected = sum
            + match version {
                GameVersion::Gregar => 0x72,
                GameVersion::Falzar => 0x18,
            };
        if word(CHECKSUM) != expected {
            return Err(format!("the save's checksum is {:#x}, its bytes give {expected:#x}: a damaged save", word(CHECKSUM)));
        }
        Ok(Save { image, version, region })
    }

    pub fn version(&self) -> GameVersion {
        self.version
    }

    pub fn region(&self) -> Region {
        self.region
    }

    /// Event flag `flag`.
    pub fn event_flag(&self, flag: u16) -> bool {
        self.image[EVENT_FLAGS + (flag >> 3) as usize] & (0x80 >> (flag & 7)) != 0
    }

    /// What the save unlocks on the custom screen: the version's Crosses it
    /// owns, by Cross number, and Beast Out.
    pub fn unlocks(&self) -> Unlocks {
        let first = match self.version {
            GameVersion::Gregar => GREGAR_CROSSES,
            GameVersion::Falzar => FALZAR_CROSSES,
        };
        Unlocks {
            version: self.version,
            crosses: std::array::from_fn(|i| self.event_flag(first + i as u16)),
            beast_out: self.event_flag(BEAST_OUT_FLAG),
        }
    }

    /// The navi operated (0 MegaMan, 1 to 11 the link navis, by the
    /// original's number).
    pub fn navi(&self) -> u8 {
        self.image[NAVI]
    }

    /// The level of the navi code received, as a battle's init exchange
    /// gives it (`sub_800B144`, `sub_8121198`): none without event flag
    /// 0x163; an error for a code of another navi (the game checks nothing,
    /// and reads past its tables).
    pub fn navi_level(&self) -> Result<Option<u8>, String> {
        if !self.event_flag(NAVI_CODE_FLAG) {
            return Ok(None);
        }
        let code = u32::from_le_bytes(self.image[NAVI_CODE..NAVI_CODE + 4].try_into().expect("four bytes"));
        // (A navi's 15 codes, levels 0 to 14.)
        let base = 0x141 + NAVI_CODES * self.navi() as u32;
        match code.checked_sub(base) {
            Some(level) if level < NAVI_CODES => Ok(Some(level as u8)),
            _ => Err(format!("the save's navi code {code:#x} isn't one of its navi's ({base:#x} to {:#x})", base + NAVI_CODES - 1)),
        }
    }

    /// The SP navi deletion times.
    pub fn sp_times(&self) -> SpTimes {
        codec::sp_times(&self.image[SP_TIMES..SP_TIMES + 0x28])
    }

    /// Where the image keeps what a US one keeps at `us`.
    fn at(&self, us: usize) -> usize {
        if self.region == Region::Jp && JP_SHIFTED.contains(&us) { us - JP_SHIFT } else { us }
    }

    /// The NaviStats block of the navi operated (MegaMan's, or the link
    /// navi's after it).
    pub fn navi_stats(&self) -> [u8; NAVI_STATS_SIZE] {
        let at = self.at(NAVI_STATS) + if self.navi() == 0 { 0 } else { NAVI_STATS_SIZE };
        self.image[at..at + NAVI_STATS_SIZE].try_into().expect("a NaviStats block")
    }

    /// The equipped folder of the navi operated, with its Regular and tag
    /// chips.
    pub fn folder(&self) -> SaveFolder {
        let stats = self.navi_stats();
        let folder = (stats[EQUIPPED_FOLDER] as usize).min(2);
        let at = FOLDERS + folder * FOLDER_SIZE * 2;
        let chips = std::array::from_fn(|i| u16::from_le_bytes([self.image[at + i * 2], self.image[at + i * 2 + 1]]));
        let regular = Some(stats[REGULAR_CHIPS + folder]).filter(|&r| (r as usize) < FOLDER_SIZE);
        let tags = [stats[TAG_CHIPS + folder * 2], stats[TAG_CHIPS + folder * 2 + 1]];
        let tags = (tags.iter().all(|&t| (t as usize) < FOLDER_SIZE)).then_some(tags);
        SaveFolder { chips, regular, tags }
    }

    /// The NaviCust's list (`codec::navicust` reads it).
    pub fn navicust_list(&self) -> &[u8] {
        let at = self.at(NAVICUST);
        &self.image[at..at + NAVICUST_PARTS * 8]
    }

    /// Whether the NaviCust compresses part `part` (its event flag).
    pub fn compressed(&self, part: u8) -> bool {
        self.event_flag(COMPRESSED_FLAG + part as u16)
    }

    /// The NaviCust board's expansions: the ExpMemry the save has.
    pub fn expansions(&self) -> u8 {
        self.image[KEY_ITEMS + EXP_MEMORY as usize]
    }

    /// The patch cards' list (`codec::patch_cards` reads it): a byte a card,
    /// its number, bit 7 when switched off.
    pub fn patch_cards(&self) -> &[u8] {
        let n = (self.image[CARD_COUNT] as usize).min(MAX_CARDS);
        &self.image[CARDS..CARDS + n]
    }

    /// The BugFrags.
    pub fn bug_frags(&self) -> u32 {
        u32::from_le_bytes(self.image[BUG_FRAGS..BUG_FRAGS + 4].try_into().expect("four bytes"))
    }
}

/// The game of an image: its version and region, by its game's name.
fn game_of(image: &[u8]) -> Result<(GameVersion, Region), String> {
    Ok(match &image[GAME_NAME..GAME_NAME + 20] {
        b"REXE6 G 20060110a US" => (GameVersion::Gregar, Region::Us),
        b"REXE6 F 20060110a US" => (GameVersion::Falzar, Region::Us),
        b"REXE6 G 20050924a JP" => (GameVersion::Gregar, Region::Jp),
        b"REXE6 F 20050924a JP" => (GameVersion::Falzar, Region::Jp),
        name => return Err(format!("not an EXE6 save (its game is {:?})", String::from_utf8_lossy(name))),
    })
}

/// Save files for tests: what a save says of these facts.
pub mod testing {
    use super::*;

    /// A US save file of `version` with Beast Out (or not), the version's
    /// Crosses owned, the navi `navi` operated and its navi code at `level`
    /// (none: event flag 0x163 clear), and these SP times; nothing else.
    pub fn file(version: GameVersion, beast_out: bool, crosses: [bool; 5], navi: u8, level: Option<u8>, sp_times: &SpTimes) -> Vec<u8> {
        file_with(version, beast_out, crosses, navi, level, sp_times, &[])
    }

    /// [`file`], with these bytes of the image set besides (a US image's
    /// offsets; event flags past the first 0x30 bytes among them).
    pub fn file_with(
        version: GameVersion,
        beast_out: bool,
        crosses: [bool; 5],
        navi: u8,
        level: Option<u8>,
        sp_times: &SpTimes,
        set: &[(usize, &[u8])],
    ) -> Vec<u8> {
        let mut flags = [0u8; 0x30];
        let mut set_flag = |f: u16| flags[(f >> 3) as usize] |= 0x80 >> (f & 7);
        if beast_out {
            set_flag(BEAST_OUT_FLAG);
        }
        let first = if version == GameVersion::Gregar { GREGAR_CROSSES } else { FALZAR_CROSSES };
        for (i, _) in crosses.iter().enumerate().filter(|(_, o)| **o) {
            set_flag(first + i as u16);
        }
        if level.is_some() {
            set_flag(NAVI_CODE_FLAG);
        }
        let code = level.map_or(0, |l| 0x141 + 15 * navi as u32 + l as u32).to_le_bytes();
        let times: Vec<u8> = sp_times.iter().flat_map(|t| t.to_le_bytes()).collect();
        let navi = [navi];
        let mut all: Vec<(usize, &[u8])> = vec![(EVENT_FLAGS, &flags), (NAVI, &navi), (NAVI_CODE, &code), (SP_TIMES, &times)];
        all.extend_from_slice(set);
        tests_file(version, &all, 0x3C)
    }

    /// A save image with these bytes set, masked with `mask` and given its
    /// checksum, as a file.
    pub(super) fn tests_file(version: GameVersion, set: &[(usize, &[u8])], mask: u8) -> Vec<u8> {
        let mut image = vec![0u8; IMAGE_SIZE];
        let name: &[u8; 20] = match version {
            GameVersion::Gregar => b"REXE6 G 20060110a US",
            GameVersion::Falzar => b"REXE6 F 20060110a US",
        };
        image[GAME_NAME..GAME_NAME + 20].copy_from_slice(name);
        for (at, bytes) in set {
            image[*at..*at + bytes.len()].copy_from_slice(bytes);
        }
        image[MASK..MASK + 4].copy_from_slice(&[mask, 0x12, 0x34, 0x56]);
        let sum: u32 = image.iter().map(|&b| b as u32).sum();
        let checksum = sum + if version == GameVersion::Gregar { 0x72 } else { 0x18 };
        image[CHECKSUM..CHECKSUM + 4].copy_from_slice(&checksum.to_le_bytes());
        let mask_word: [u8; 4] = image[MASK..MASK + 4].try_into().expect("four bytes");
        for b in image.iter_mut() {
            *b ^= mask;
        }
        image[MASK..MASK + 4].copy_from_slice(&mask_word);
        let mut f = vec![0xFFu8; IMAGE_START];
        f.extend(image);
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(version: GameVersion, set: &[(usize, &[u8])], mask: u8) -> Vec<u8> {
        testing::tests_file(version, set, mask)
    }

    #[test]
    fn the_testing_file_reads_back() {
        let times: SpTimes = std::array::from_fn(|i| i as u16 * 37);
        let f = testing::file(GameVersion::Gregar, false, [true, false, true, false, false], 3, Some(9), &times);
        let s = Save::read(&f).unwrap();
        assert_eq!((s.version(), s.navi(), s.navi_level(), s.sp_times()), (GameVersion::Gregar, 3, Ok(Some(9)), times));
        assert_eq!((s.unlocks().beast_out, s.unlocks().crosses), (false, [true, false, true, false, false]));
    }

    #[test]
    fn a_save_reads_its_game_unlocks_level_and_times() {
        let mut times = [0u8; 0x28];
        times[0..2].copy_from_slice(&721u16.to_le_bytes());
        times[0x26..0x28].copy_from_slice(&1500u16.to_le_bytes());
        // ProtoMan (navi 11) from his level-5 code: 0x141 + 15 * 11 + 5.
        let code = (0x141u32 + 15 * 11 + 5).to_le_bytes();
        // Beast Out (0xE0) and Falzar's second and fourth Crosses (0xE8,
        // 0xEA); flag 0x163.
        let f = file(
            GameVersion::Falzar,
            &[(EVENT_FLAGS + 0x1C, &[0x80]), (EVENT_FLAGS + 0x1D, &[0xA0]), (EVENT_FLAGS + 0x2C, &[0x10]), (NAVI, &[11]), (NAVI_CODE, &code), (SP_TIMES, &times)],
            0xA5,
        );
        let s = Save::read(&f).unwrap();
        assert_eq!((s.version(), s.region(), s.navi()), (GameVersion::Falzar, Region::Us, 11));
        let u = s.unlocks();
        assert_eq!((u.crosses, u.beast_out), ([false, true, false, true, false], true));
        assert_eq!(s.navi_level(), Ok(Some(5)));
        assert_eq!((s.sp_times()[0], s.sp_times()[19]), (721, 1500));
    }

    #[test]
    fn a_save_without_a_code_has_no_level() {
        let s = Save::read(&file(GameVersion::Gregar, &[(NAVI_CODE, &[0x41, 1, 0, 0])], 0)).unwrap();
        assert_eq!((s.version(), s.navi_level()), (GameVersion::Gregar, Ok(None)));
        assert!(!s.unlocks().beast_out);
    }

    #[test]
    fn a_damaged_or_foreign_save_is_refused() {
        let mut f = file(GameVersion::Falzar, &[], 0x5A);
        f[IMAGE_START + 0x200] ^= 1;
        assert!(Save::read(&f).unwrap_err().contains("checksum"));
        let mut g = file(GameVersion::Falzar, &[], 0);
        g[IMAGE_START + GAME_NAME] = b'X';
        assert!(Save::read(&g).unwrap_err().contains("not an EXE6 save"));
        assert!(Save::read(&g[..0x200]).is_err());
        // A code of another navi.
        let code = (0x141u32 + 15 * 3).to_le_bytes();
        let h = file(GameVersion::Falzar, &[(EVENT_FLAGS + 0x2C, &[0x10]), (NAVI, &[11]), (NAVI_CODE, &code)], 0);
        assert!(Save::read(&h).unwrap().navi_level().is_err());
    }
}
