//! Folders: the 30 chips a navi brings, and the battle folder the custom
//! screen deals from, shuffled once per round (docs/engine/custom-screen.md
//! §1).

use super::library::Library;
use crate::content::{ChipClass, ChipCode, ChipId};
use crate::rng::Rng;

/// Chips in a folder.
pub const FOLDER_SIZE: usize = 30;

/// A chip with its code, as folders and selections hold it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FolderChip {
    pub id: ChipId,
    pub code: ChipCode,
}

impl FolderChip {
    pub fn new(id: ChipId, code: ChipCode) -> FolderChip {
        FolderChip { id, code }
    }

    /// Decode the game's packed form, `code << 9 | id`.
    pub fn from_packed(v: u16) -> FolderChip {
        FolderChip { id: v & 0x1FF, code: ChipCode((v >> 9) as u8) }
    }

    /// The game's packed form, `code << 9 | id`.
    pub fn packed(self) -> u16 {
        (self.code.0 as u16) << 9 | self.id
    }
}

/// A navi's folder as the save holds it, with its regular chip and tag
/// chips (entries of `chips`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SavedFolder {
    pub chips: [FolderChip; FOLDER_SIZE],
    /// The regular chip, always dealt in the first custom screen.
    pub regular: Option<u8>,
    /// Two tag chips, dealt next to each other.
    pub tags: Option<(u8, u8)>,
}

/// The battle folder (`eBattleFolder`): the chips in the order they are
/// dealt. Chips leave it as they are picked, and the ones dealt but not
/// picked go back in (see `custom::Screen`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BattleFolder {
    pub chips: [Option<FolderChip>; FOLDER_SIZE],
    /// The first chip is the Regular chip and hasn't been used yet
    /// (BattleState+0x17).
    pub regular_pending: bool,
}

impl BattleFolder {
    /// A folder with no chips.
    pub fn empty() -> BattleFolder {
        BattleFolder { chips: [None; FOLDER_SIZE], regular_pending: false }
    }

    /// A battle folder from the game's 0x3C-byte encoding (packed chips,
    /// 0xFFFF = empty).
    pub fn from_bytes(b: &[u8], regular_pending: bool) -> BattleFolder {
        BattleFolder {
            chips: std::array::from_fn(|i| {
                let v = u16::from_le_bytes([b[2 * i], b[2 * i + 1]]);
                (v != 0xFFFF).then(|| FolderChip::from_packed(v))
            }),
            regular_pending,
        }
    }

    /// Chips left.
    pub fn count(&self) -> usize {
        self.chips.iter().flatten().count()
    }

    /// `sub_802945A`: close up the gaps, keeping the order.
    pub fn compact(&mut self) {
        let mut chips = [None; FOLDER_SIZE];
        for (slot, c) in chips.iter_mut().zip(self.chips.iter().flatten()) {
            *slot = Some(*c);
        }
        self.chips = chips;
    }

    /// Take entry `i` out, leaving a gap.
    pub fn take(&mut self, i: usize) -> Option<FolderChip> {
        self.chips[i].take()
    }

    /// Put a chip in the first gap.
    pub fn put_in_first_gap(&mut self, c: FolderChip) {
        if let Some(slot) = self.chips.iter_mut().find(|s| s.is_none()) {
            *slot = Some(c);
        }
    }

    /// The game's encoding (for comparisons with recordings).
    pub fn to_bytes(&self) -> [u8; 2 * FOLDER_SIZE] {
        let mut b = [0u8; 2 * FOLDER_SIZE];
        for (i, c) in self.chips.iter().enumerate() {
            let v = c.map_or(0xFFFF, FolderChip::packed);
            b[2 * i..2 * i + 2].copy_from_slice(&v.to_le_bytes());
        }
        b
    }

    /// A round's battle folder (`sub_800A3E4`, then `sub_800A570`), drawn
    /// by the console that owns the folder from its own RNG1 stream at the
    /// round's init. Battle mode 1 ignores the regular and tag chips.
    ///
    /// The regular chip goes first and stays there. The tag chips go
    /// last, out of the shuffle, and then move together to a random place
    /// among the first 21. Gigas are shuffled apart and each is inserted at
    /// a random place at least 10 chips in (8 to 18 in battle mode 1).
    pub fn shuffled(saved: &SavedFolder, battle_mode: u8, rng: &mut Rng, library: &dyn Library) -> BattleFolder {
        let (regular, tags) = if battle_mode == 1 { (None, None) } else { (saved.regular, saved.tags) };
        // Lay out: the regular chip first, the tags last.
        let mut laid = [saved.chips[0]; FOLDER_SIZE];
        let mut next = regular.is_some() as usize;
        for (i, &c) in saved.chips.iter().enumerate() {
            match (regular, tags) {
                (Some(r), _) if r as usize == i => laid[0] = c,
                (_, Some((t, _))) if t as usize == i => laid[FOLDER_SIZE - 2] = c,
                (_, Some((_, t))) if t as usize == i => laid[FOLDER_SIZE - 1] = c,
                _ => {
                    laid[next] = c;
                    next += 1;
                }
            }
        }
        let (mut normal, gigas): (Vec<FolderChip>, Vec<FolderChip>) =
            laid.iter().partition(|c| library.chip(c.id).class != ChipClass::Giga);
        let mut gigas = gigas;
        if !normal.is_empty() {
            let skip_front = regular.is_some() as usize;
            let skip_back = if tags.is_some() { 2 } else { 0 };
            let n = normal.len() - skip_front - skip_back;
            shuffle(&mut normal[skip_front..skip_front + n], n, rng);
        }
        if !gigas.is_empty() {
            let n = gigas.len();
            shuffle(&mut gigas, n, rng);
            for g in gigas {
                let at = if battle_mode == 1 {
                    8 + rng.next_positive() % 11
                } else {
                    10 + rng.next_positive() % (normal.len() as u32 - 12)
                } as usize;
                if at <= normal.len() {
                    normal.insert(at, g);
                }
            }
        }
        if tags.is_some() && normal.len() == FOLDER_SIZE {
            let at = (rng.next_positive() % 19 + 1) as usize;
            normal.swap(FOLDER_SIZE - 2, at);
            normal.swap(FOLDER_SIZE - 1, at + 1);
        }
        BattleFolder { chips: std::array::from_fn(|i| normal.get(i).copied()), regular_pending: regular.is_some() }
    }
}

/// `sub_8000D12`: `rounds` swaps of two entries picked at random.
fn shuffle(chips: &mut [FolderChip], rounds: usize, rng: &mut Rng) {
    let n = chips.len() as u32;
    for _ in 0..rounds {
        let a = (rng.next_positive() % n) as usize;
        let b = (rng.next_positive() % n) as usize;
        chips.swap(a, b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custom::library::testing::{EVERY_CODE, TestLibrary, chip};

    /// The recorded folders' Giga chips (the rest are standard).
    fn library() -> TestLibrary {
        let giga = || chip(ChipClass::Giga, EVERY_CODE, 0, 0);
        TestLibrary::new([0x130, 0x132, 0x133, 0x134, 0x135, 0x136].map(|id| (id, giga())).to_vec(), Vec::new())
    }

    fn chips(packed: [u16; FOLDER_SIZE]) -> [FolderChip; FOLDER_SIZE] {
        packed.map(FolderChip::from_packed)
    }

    /// `rng1_next_frame`: RNG1 at the start of the next frame, after the
    /// main loop's own step.
    fn check(saved: SavedFolder, rng1: u32, expected: [u16; FOLDER_SIZE], rng1_next_frame: u32) {
        let mut rng = Rng::new(rng1);
        let f = BattleFolder::shuffled(&saved, 0, &mut rng, &library());
        assert_eq!(f.chips, chips(expected).map(Some));
        rng.next();
        assert_eq!(rng.state, rng1_next_frame);
    }

    // Folders, RNG1 states and results measured in the original game at
    // the init of recorded netbattles (battle mode 0), on both consoles.

    #[test]
    fn plain_folder() {
        let mut saved = [0x34A7; FOLDER_SIZE];
        saved[10..].fill(0x1A11);
        let expected = [
            0x34A7, 0x1A11, 0x1A11, 0x1A11, 0x1A11, 0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x1A11, 0x1A11,
            0x1A11, 0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x1A11, 0x34A7, 0x1A11, 0x1A11, 0x34A7, 0x1A11,
            0x34A7, 0x1A11, 0x1A11, 0x1A11,
        ];
        check(SavedFolder { chips: chips(saved), regular: None, tags: None }, 0x62FA_BD30, expected, 0x9EC7_5BBF);
    }

    #[test]
    fn regular_and_tag_chips() {
        let saved = [
            0x1602, 0x0001, 0x0201, 0x0201, 0x3404, 0x3404, 0x0605, 0x0605, 0x0605, 0x0236, 0x0236, 0x1636, 0x1636,
            0x2447, 0x2447, 0x2447, 0x2447, 0x2448, 0x2448, 0x34AF, 0x3459, 0x3459, 0x009A, 0x009A, 0x169A, 0x169A,
            0x24A3, 0x34C0, 0x34C0, 0x34B8,
        ];
        let expected = [
            0x0001, 0x24A3, 0x1636, 0x3404, 0x2448, 0x009A, 0x169A, 0x0236, 0x34B8, 0x0605, 0x3459, 0x34C0, 0x0201,
            0x3404, 0x2447, 0x0605, 0x2448, 0x1636, 0x0605, 0x0236, 0x2447, 0x2447, 0x34C0, 0x2447, 0x0201, 0x34AF,
            0x1602, 0x3459, 0x009A, 0x169A,
        ];
        let saved = SavedFolder { chips: chips(saved), regular: Some(1), tags: Some((3, 4)) };
        check(saved, 0x0B48_8EEC, expected, 0x40D3_D040);
    }

    #[test]
    fn giga_chip() {
        let saved = [
            0x2130, 0x00F4, 0x130C, 0x28B6, 0x32A9, 0x350A, 0x34EC, 0x20A5, 0x34BA, 0x350D, 0x12A1, 0x12A1, 0x002F,
            0x002F, 0x2011, 0x0027, 0x0027, 0x004D, 0x006D, 0x34B1, 0x34B1, 0x34BB, 0x34BB, 0x3450, 0x3450, 0x34A6,
            0x34A3, 0x34A3, 0x34A3, 0x34A3,
        ];
        let expected = [
            0x3450, 0x00F4, 0x3450, 0x350D, 0x34BB, 0x34B1, 0x34EC, 0x34BA, 0x0027, 0x350A, 0x34BB, 0x20A5, 0x32A9,
            0x2011, 0x34A6, 0x34A3, 0x2130, 0x130C, 0x006D, 0x002F, 0x34A3, 0x34A3, 0x0027, 0x004D, 0x12A1, 0x28B6,
            0x34A3, 0x12A1, 0x34B1, 0x002F,
        ];
        check(SavedFolder { chips: chips(saved), regular: None, tags: None }, 0x15EE_C9DC, expected, 0x5D37_B35D);
    }

    #[test]
    fn regular_chip_with_gigas() {
        let saved = [
            0x32B5, 0x342B, 0x34BC, 0x34BA, 0x34BB, 0x34BB, 0x34A3, 0x34A3, 0x28B4, 0x34AC, 0x32A9, 0x34B0, 0x34AE,
            0x28B6, 0x3450, 0x12A1, 0x12A1, 0x34B1, 0x34B1, 0x34B1, 0x0706, 0x130C, 0x34EC, 0x350A, 0x350D, 0x1B33,
            0x0B32, 0x0734, 0x1335, 0x2B36,
        ];
        let expected = [
            0x34AE, 0x34A3, 0x34B1, 0x12A1, 0x34EC, 0x32A9, 0x342B, 0x34B0, 0x12A1, 0x34B1, 0x2B36, 0x34AC, 0x350D,
            0x1335, 0x28B4, 0x28B6, 0x34BA, 0x32B5, 0x34A3, 0x350A, 0x34B1, 0x0B32, 0x1B33, 0x34BB, 0x34BB, 0x0734,
            0x130C, 0x34BC, 0x0706, 0x3450,
        ];
        check(SavedFolder { chips: chips(saved), regular: Some(12), tags: None }, 0x2C38_F097, expected, 0xB4DC_B2B5);
    }
}
