//! A BN6 save file (the .sav an emulator keeps), and what nettai reads of
//! it for a player's setup: the game, what it unlocks on the custom screen,
//! the navi code's level and the SP navi deletion times. (More of a side, its
//! folder, NaviCust and patch cards, can follow.)
//!
//! The file holds the save image at 0x100: 0x6710 bytes, the game's EWRAM
//! from 0x02000000 as the game saves it (an address's offset in the image
//! is its offset from 0x02000000), each byte XORed with the low byte of the
//! mask word at 0x1064 (the word itself stored plain). The word at 0x1060
//! is 0 in a save the game wrote; the game's name is at 0x1C70; the word at
//! 0x1C6C is the checksum: the image's bytes summed, less the checksum's
//! own, plus 0x72 for Gregar and 0x18 for Falzar. The four games (US and
//! Japanese, Gregar and Falzar) keep what is read here at the same places.
//! (The format as Tango's save support reads it.)

use crate::codec;
use crate::unlocks::Unlocks;
use nettai_battle::custom::GameVersion;
use nettai_battle::setup::SpTimes;

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
const EVENT_FLAGS: usize = 0x1C88;
/// The navi operated (`GameState+0x01`, 0x02001B81: 0 MegaMan, 1 to 11 the
/// link navis), and the navi code received (`S2001c04+0x30`, 0x02001C34, a
/// word: `0x141 + 15 · navi + level`, `sub_8121198`).
const NAVI: usize = 0x1B81;
const NAVI_CODE: usize = 0x1C34;
/// The SP navi deletion times (0x020018C0, 0x28 bytes), which the init
/// exchange sends (`sub_800B144`, the block's +0x70) and a battle reads at
/// `byte_203EB00`.
const SP_TIMES: usize = 0x18C0;

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

/// A BN6 save, its image unmasked.
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

impl Save {
    /// The save in `file` (a .sav's bytes), checked: its game's name, the
    /// shift word and the checksum.
    pub fn read(file: &[u8]) -> Result<Save, String> {
        let mut image: Box<[u8]> = file
            .get(IMAGE_START..IMAGE_START + IMAGE_SIZE)
            .ok_or_else(|| format!("a BN6 save is {:#x} bytes or more, not {:#x}", IMAGE_START + IMAGE_SIZE, file.len()))?
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
        let (version, region) = match &image[GAME_NAME..GAME_NAME + 20] {
            b"REXE6 G 20060110a US" => (GameVersion::Gregar, Region::Us),
            b"REXE6 F 20060110a US" => (GameVersion::Falzar, Region::Us),
            b"REXE6 G 20050924a JP" => (GameVersion::Gregar, Region::Jp),
            b"REXE6 F 20050924a JP" => (GameVersion::Falzar, Region::Jp),
            name => return Err(format!("not a BN6 save (its game is {:?})", String::from_utf8_lossy(name))),
        };
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
    /// owns and Beast Out (no Cross list: the save's are the version's).
    pub fn unlocks(&self) -> Unlocks {
        let first = match self.version {
            GameVersion::Gregar => GREGAR_CROSSES,
            GameVersion::Falzar => FALZAR_CROSSES,
        };
        Unlocks {
            crosses: std::array::from_fn(|i| self.event_flag(first + i as u16)),
            beast_out: self.event_flag(BEAST_OUT_FLAG),
            ..Unlocks::nothing(self.version)
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
        let base = 0x141 + 15 * self.navi() as u32;
        match code.checked_sub(base) {
            Some(level) if level <= nettai_battle::custom::MAX_NAVI_LEVEL as u32 => Ok(Some(level as u8)),
            _ => Err(format!("the save's navi code {code:#x} isn't one of its navi's ({base:#x} to {:#x})", base + 14)),
        }
    }

    /// The SP navi deletion times.
    pub fn sp_times(&self) -> SpTimes {
        codec::sp_times(&self.image[SP_TIMES..SP_TIMES + 0x28])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save image with these bytes set, masked with `mask` and given its
    /// checksum, as a file.
    fn file(version: GameVersion, set: &[(usize, &[u8])], mask: u8) -> Vec<u8> {
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
        let mask_word: [u8; 4] = image[MASK..MASK + 4].try_into().unwrap();
        for b in image.iter_mut() {
            *b ^= mask;
        }
        image[MASK..MASK + 4].copy_from_slice(&mask_word);
        let mut f = vec![0xFFu8; IMAGE_START];
        f.extend(image);
        f
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
        assert_eq!((u.crosses, u.beast_out, u.cross_list), ([false, true, false, true, false], true, None));
        assert_eq!(s.navi_level(), Ok(Some(5)));
        assert_eq!((s.sp_times().0[0], s.sp_times().0[19]), (721, 1500));
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
        assert!(Save::read(&g).unwrap_err().contains("not a BN6 save"));
        assert!(Save::read(&g[..0x200]).is_err());
        // A code of another navi.
        let code = (0x141u32 + 15 * 3).to_le_bytes();
        let h = file(GameVersion::Falzar, &[(EVENT_FLAGS + 0x2C, &[0x10]), (NAVI, &[11]), (NAVI_CODE, &code)], 0);
        assert!(Save::read(&h).unwrap().navi_level().is_err());
    }
}
