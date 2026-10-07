//! An EXE4 save file (the .sav an emulator keeps), and what nettai reads of
//! it for a player's setup: the version and region, the equipped folder with
//! its Regular chip, the NaviCust (its list and grid), the Mod Cards (BN4's
//! patch cards: six slots, each a card on or off), the base max HP and the
//! Regular memory.
//!
//! The file holds the save image at 0: 0x73D2 bytes, each XORed with the
//! low byte of the mask word at 0x1554 (the word itself stored plain). The
//! word at 0x1550 is the shift: the game moves the region 0x2130..0x5E20 on
//! by it (a multiple of 4 up to 0x1FC), and the game's name ("ROCKMANEXE4
//! 20031022") and the checksum move with it, from 0x2208 and 0x21E8. The
//! checksum is the image's bytes summed, less the checksum's own, plus
//! 0x16 for Red Sun and 0x22 for Blue Moon; a Japanese ROM's leaves out the
//! image's first byte. (The format as Tango's BN4 save support reads it;
//! Tango's raw netplay saves are such an image unmasked and unshifted, whose
//! version and region their names give: [`Save::from_image`].)
//!
//! Once unshifted: MegaMan's HP and maximum HP at 0x2150 and 0x2152, the
//! equipped folder's number at 0x2132, the Regular chip in battle at 0x214C
//! (an entry of the folder, 0xFF none) and each folder's at 0x214D, the
//! Regular memory at 0x2148 (less 4), the base max HP at 0x21CA, the folders at 0x262C (three of 30 halfwords: the chip in the low
//! 9 bits, the code above), the NaviCust's list at 0x4564 (25 parts of 8
//! bytes: the part id, 0 none; its column, row, rotation and compression
//! flag at +2 to +5) and its 5x5 grid at 0x4540 (a cell the list's entry
//! from 1, 0 empty), the Mod Cards at 0x464C (six slots, a card on) and
//! 0x4653 (the same six, a card off; 0xFF none), MegaMan's NaviStats block
//! at 0x4E60 (the first of eight).

use crate::Version;

/// The image's size, and where the mask word, the shift, the checksum and
/// the game's name are (the last two before the shift).
pub const IMAGE_SIZE: usize = 0x73D2;
const MASK: usize = 0x1554;
const SHIFT: usize = 0x1550;
const CHECKSUM: usize = 0x21E8;
const GAME_NAME: usize = 0x2208;
const NAME: &[u8; 20] = b"ROCKMANEXE4 20031022";
/// The region the shift moves, and the largest shift.
const SHIFTED: std::ops::Range<usize> = 0x2130..0x5E20;
const MAX_SHIFT: usize = 0x1FC;
/// What a version adds to the checksum.
fn checksum_start(v: Version) -> u32 {
    match v {
        Version::RedSun => 0x16,
        Version::BlueMoon => 0x22,
    }
}

const EQUIPPED_FOLDER: usize = 0x2132;
const REGULAR_CHIP: usize = 0x214C;
const REGULAR_MEMORY: usize = 0x2148;
const BASE_MAX_HP: usize = 0x21CA;
/// MegaMan's HP and maximum HP (the game state's; a battle copies them into
/// his NaviStats block, 0x0800D726, whose own words the save leaves stale).
const HP: usize = 0x2150;
const MAX_HP: usize = 0x2152;
const FOLDERS: usize = 0x262C;
pub const FOLDERS_COUNT: usize = 3;
pub const FOLDER_SIZE: usize = 30;
const NAVICUST: usize = 0x4564;
pub const NAVICUST_PARTS: usize = 25;
const NAVICUST_GRID: usize = 0x4540;
pub const NAVICUST_SIZE: usize = 5;
/// MegaMan's NaviStats block, the first of the save's eight (the toolkit's
/// +0x78): his stats as the PET's last reload left them.
const NAVI_STATS: usize = 0x4E60;
const MOD_CARDS_ON: usize = 0x464C;
const MOD_CARDS_OFF: usize = 0x4653;
pub const MOD_CARD_SLOTS: usize = 6;

/// A folder's chip: its number and its code (0 A to 25 Z, 26 the asterisk).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderChip {
    pub id: u16,
    pub code: u8,
}

/// A NaviCust part of the list: its id (the program `id >> 2`, its color
/// `id & 3`), its place and rotation, and whether it is compressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Part {
    pub id: u8,
    pub column: u8,
    pub row: u8,
    pub rotation: u8,
    pub compressed: bool,
}

/// A NaviCust list as the save keeps it (25 entries of 8 bytes: the part id,
/// 0 none; its column, row, rotation and compression flag at +2 to +5).
pub fn parts(list: &[u8]) -> [Option<Part>; NAVICUST_PARTS] {
    std::array::from_fn(|i| {
        let p = &list[8 * i..8 * i + 8];
        (p[0] != 0).then_some(Part { id: p[0], column: p[2], row: p[3], rotation: p[4], compressed: p[5] != 0 })
    })
}

/// A Mod Card slot's card and whether it is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModCard {
    pub id: u8,
    pub on: bool,
}

/// An EXE4 save: its image unmasked and unshifted, its version and region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Save {
    image: Vec<u8>,
    pub version: Version,
    pub japanese: bool,
}

impl Save {
    /// A save file: unmask it, check its name and its checksum (which says
    /// its version and region), and unshift it.
    pub fn read(file: &[u8]) -> Result<Save, String> {
        let mut image = file.get(..IMAGE_SIZE).ok_or_else(|| format!("{} bytes: no EXE4 save image", file.len()))?.to_vec();
        let mask = u32::from_le_bytes(image[MASK..MASK + 4].try_into().unwrap());
        for b in &mut image {
            *b ^= mask as u8;
        }
        image[MASK..MASK + 4].copy_from_slice(&mask.to_le_bytes());
        let shift = u32::from_le_bytes(image[SHIFT..SHIFT + 4].try_into().unwrap()) as usize;
        if shift > MAX_SHIFT || shift % 4 != 0 {
            return Err(format!("shift {shift:#x}: not an EXE4 save"));
        }
        if &image[shift + GAME_NAME..shift + GAME_NAME + NAME.len()] != NAME {
            return Err("not an EXE4 save (its game's name isn't ROCKMANEXE4's)".into());
        }
        let stored = u32::from_le_bytes(image[shift + CHECKSUM..shift + CHECKSUM + 4].try_into().unwrap());
        let sum = image.iter().map(|&b| b as u32).sum::<u32>()
            - image[shift + CHECKSUM..shift + CHECKSUM + 4].iter().map(|&b| b as u32).sum::<u32>();
        let mut found = None;
        'find: for japanese in [false, true] {
            let raw = sum - if japanese { image[0] as u32 } else { 0 };
            for v in [Version::RedSun, Version::BlueMoon] {
                if stored == raw.wrapping_add(checksum_start(v)) {
                    found = Some((v, japanese));
                    break 'find;
                }
            }
        }
        let (version, japanese) = found.ok_or_else(|| format!("checksum {stored:#x} is no version's (the bytes sum to {sum:#x})"))?;
        image.copy_within(SHIFTED.start + shift..SHIFTED.end + shift, SHIFTED.start);
        image[SHIFTED.end..SHIFTED.end + shift].fill(0);
        image[SHIFT..SHIFT + 4].fill(0);
        Ok(Save { image, version, japanese })
    }

    /// A save image as it is (unmasked, unshifted: Tango's raw saves), of
    /// a version and region.
    pub fn from_image(image: &[u8], version: Version, japanese: bool) -> Result<Save, String> {
        let image = image.get(..IMAGE_SIZE).ok_or_else(|| format!("{} bytes: no EXE4 save image", image.len()))?.to_vec();
        if image[SHIFT..SHIFT + 4] != [0; 4] {
            return Err("a shifted image: read the save file instead".into());
        }
        if &image[GAME_NAME..GAME_NAME + NAME.len()] != NAME {
            return Err("not an EXE4 save image (its game's name isn't ROCKMANEXE4's)".into());
        }
        Ok(Save { image, version, japanese })
    }

    /// The unshifted image.
    pub fn image(&self) -> &[u8] {
        &self.image
    }

    fn u16(&self, at: usize) -> u16 {
        u16::from_le_bytes([self.image[at], self.image[at + 1]])
    }

    /// The equipped folder's number (0 to 2).
    pub fn equipped_folder(&self) -> usize {
        self.image[EQUIPPED_FOLDER] as usize
    }

    /// Folder `f`'s chips (an entry with no chip: none).
    pub fn folder(&self, f: usize) -> [Option<FolderChip>; FOLDER_SIZE] {
        std::array::from_fn(|i| {
            let v = self.u16(FOLDERS + 2 * (FOLDER_SIZE * f + i));
            let (id, code) = (v & 0x1FF, (v >> 9) as u8);
            (code <= 26).then_some(FolderChip { id, code })
        })
    }

    /// The equipped folder's Regular chip in battle: its entry.
    pub fn regular_chip(&self) -> Option<usize> {
        let i = self.image[REGULAR_CHIP] as usize;
        (i < FOLDER_SIZE).then_some(i)
    }

    /// The Regular memory (the largest MB a Regular chip may have).
    pub fn regular_memory(&self) -> u8 {
        self.image[REGULAR_MEMORY] + 4
    }

    /// The base max HP (before the NaviCust's and the Mod Cards' HP).
    pub fn base_max_hp(&self) -> u16 {
        self.u16(BASE_MAX_HP)
    }

    /// MegaMan's HP and maximum HP (the maximum the PET's last reload made:
    /// the base, the NaviCust's and the Mod Cards' HP).
    pub fn hp(&self) -> u16 {
        self.u16(HP)
    }

    pub fn max_hp(&self) -> u16 {
        self.u16(MAX_HP)
    }

    /// The NaviCust's list (an entry with no part: none).
    pub fn navicust(&self) -> [Option<Part>; NAVICUST_PARTS] {
        parts(&self.image[NAVICUST..NAVICUST + 8 * NAVICUST_PARTS])
    }

    /// The NaviCust's grid, row by row: a cell's part, by its entry in the
    /// list.
    pub fn navicust_grid(&self) -> [[Option<usize>; NAVICUST_SIZE]; NAVICUST_SIZE] {
        std::array::from_fn(|y| {
            std::array::from_fn(|x| match self.image[NAVICUST_GRID + NAVICUST_SIZE * y + x] {
                0 => None,
                n => Some(n as usize - 1),
            })
        })
    }

    /// MegaMan's NaviStats block (`codec::navi_stats` reads it), as the PET's
    /// last reload left it but its HP words (+0x30 to +0x35: [`Save::hp`],
    /// [`Save::max_hp`], [`Save::base_max_hp`] hold his).
    pub fn navi_stats(&self) -> [u8; crate::codec::NAVI_STATS] {
        self.image[NAVI_STATS..NAVI_STATS + crate::codec::NAVI_STATS].try_into().unwrap()
    }

    /// The Mod Cards by slot (a slot with no card: none).
    pub fn mod_cards(&self) -> [Option<ModCard>; MOD_CARD_SLOTS] {
        std::array::from_fn(|s| match (self.image[MOD_CARDS_ON + s], self.image[MOD_CARDS_OFF + s]) {
            (0xFF, 0xFF) => None,
            (0xFF, id) => Some(ModCard { id, on: false }),
            (id, _) => Some(ModCard { id, on: true }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An image of a version: the game's name, a folder of Cannons (code A,
    /// then B) and an AirShot, a Regular chip, a part, a Mod Card on and one
    /// off.
    fn image() -> Vec<u8> {
        let mut img = vec![0; IMAGE_SIZE];
        img[GAME_NAME..GAME_NAME + NAME.len()].copy_from_slice(NAME);
        img[EQUIPPED_FOLDER] = 1;
        let folder = FOLDERS + 2 * FOLDER_SIZE;
        img[folder..folder + 2].copy_from_slice(&(1u16).to_le_bytes());
        img[folder + 2..folder + 4].copy_from_slice(&(1u16 | 1 << 9).to_le_bytes());
        img[folder + 4..folder + 6].copy_from_slice(&(4u16 | 26 << 9).to_le_bytes());
        for i in 3..FOLDER_SIZE {
            img[folder + 2 * i..folder + 2 * i + 2].copy_from_slice(&[0xFF, 0xFF]);
        }
        img[REGULAR_CHIP] = 2;
        img[REGULAR_MEMORY] = 6;
        img[BASE_MAX_HP..BASE_MAX_HP + 2].copy_from_slice(&760u16.to_le_bytes());
        img[NAVICUST..NAVICUST + 8].copy_from_slice(&[0xA5, 0, 1, 4, 1, 0, 0, 0]);
        img[NAVICUST_GRID + 5 * 4 + 1] = 1;
        img[MOD_CARDS_ON..MOD_CARDS_ON + 6].copy_from_slice(&[7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        img[MOD_CARDS_OFF..MOD_CARDS_OFF + 6].copy_from_slice(&[0xFF, 9, 0xFF, 0xFF, 0xFF, 0xFF]);
        img
    }

    /// The file a console of `v` would write of `img`: shifted, checksummed
    /// and masked.
    fn file(mut img: Vec<u8>, v: Version, japanese: bool, shift: usize, mask: u8) -> Vec<u8> {
        img.copy_within(SHIFTED.start..SHIFTED.end, SHIFTED.start + shift);
        img[SHIFTED.start..SHIFTED.start + shift].fill(0);
        img[SHIFT..SHIFT + 4].copy_from_slice(&(shift as u32).to_le_bytes());
        img[MASK..MASK + 4].copy_from_slice(&[mask, 0x12, 0x34, 0x56]);
        img[0] = 0x3C;
        let at = shift + CHECKSUM;
        img[at..at + 4].fill(0);
        let sum = img.iter().map(|&b| b as u32).sum::<u32>() - if japanese { img[0] as u32 } else { 0 };
        img[at..at + 4].copy_from_slice(&(sum + checksum_start(v)).to_le_bytes());
        let word: [u8; 4] = img[MASK..MASK + 4].try_into().unwrap();
        for b in &mut img {
            *b ^= mask;
        }
        img[MASK..MASK + 4].copy_from_slice(&word);
        img
    }

    #[test]
    fn a_save_file_unmasks_unshifts_and_says_its_version() {
        for (v, japanese) in [(Version::RedSun, false), (Version::BlueMoon, false), (Version::RedSun, true), (Version::BlueMoon, true)] {
            let s = Save::read(&file(image(), v, japanese, 0x88, 0xA7)).unwrap();
            assert_eq!((s.version, s.japanese), (v, japanese));
            assert_eq!(s.equipped_folder(), 1);
            let f = s.folder(1);
            assert_eq!(f[..3], [Some(FolderChip { id: 1, code: 0 }), Some(FolderChip { id: 1, code: 1 }), Some(FolderChip { id: 4, code: 26 })]);
            assert!(f[3..].iter().all(Option::is_none));
            assert_eq!((s.regular_chip(), s.regular_memory(), s.base_max_hp()), (Some(2), 10, 760));
            assert_eq!(s.navicust()[0], Some(Part { id: 0xA5, column: 1, row: 4, rotation: 1, compressed: false }));
            assert!(s.navicust()[1..].iter().all(Option::is_none));
            assert_eq!(s.navicust_grid()[4][1], Some(0));
            assert_eq!(s.mod_cards()[..3], [Some(ModCard { id: 7, on: true }), Some(ModCard { id: 9, on: false }), None]);
        }
    }

    #[test]
    fn a_bad_checksum_or_name_is_no_save() {
        let mut f = file(image(), Version::RedSun, false, 0, 0);
        f[0x100] ^= 1;
        assert!(Save::read(&f).unwrap_err().contains("checksum"));
        let mut img = image();
        img[GAME_NAME] = b'X';
        assert!(Save::read(&file(img, Version::RedSun, false, 0, 0)).unwrap_err().contains("name"));
        assert!(Save::read(&[0; 16]).is_err());
    }

    #[test]
    fn an_image_reads_as_it_is() {
        let s = Save::from_image(&image(), Version::BlueMoon, true).unwrap();
        assert_eq!((s.version, s.japanese, s.base_max_hp()), (Version::BlueMoon, true, 760));
    }
}
