//! Folders: the 30 chips a navi brings, and the battle folder the custom
//! screen deals from, shuffled once per round (docs/engine/custom-screen.md
//! §1).

use super::library::Library;
use crate::content::{ChipClass, ChipCode};
use nettai_content_api::ChipHandle;
use crate::rng::Rng;

/// Chips in a folder.
pub const FOLDER_SIZE: usize = 30;

/// A chip with its code, as folders and selections hold it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FolderChip {
    pub id: ChipHandle,
    pub code: ChipCode,
}

impl FolderChip {
    pub fn new(id: ChipHandle, code: ChipCode) -> FolderChip {
        FolderChip { id, code }
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

    /// A round's battle folder (`sub_800A3E4`, then `sub_800A570`), drawn
    /// by the console that owns the folder from its own RNG1 stream at the
    /// round's init. Battle mode 1 ignores the regular and tag chips.
    ///
    /// The regular chip goes first and stays there. The tag chips go
    /// last, out of the shuffle, and then move together to a random place
    /// among the first 21. Gigas are shuffled apart and each is inserted at
    /// a random place at least 10 chips in (8 to 18 in battle mode 1). Each
    /// shuffle (`sub_8000D12`, [`shuffle`]) takes two draws a round, as
    /// many rounds as chips.
    pub fn shuffled(saved: &SavedFolder, battle_mode: u8, rng: &mut Rng, library: &dyn Library) -> BattleFolder {
        BattleFolder::shuffled_with_tag_pair(saved, battle_mode, rng, library).0
    }

    /// `shuffled`, and where the tag pair went (BattleState+0x45, which
    /// ChpShufl's re-deal reads: `ConsoleSetup::tag_pair`).
    pub fn shuffled_with_tag_pair(
        saved: &SavedFolder,
        battle_mode: u8,
        rng: &mut Rng,
        library: &dyn Library,
    ) -> (BattleFolder, Option<u8>) {
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
            shuffle(&mut normal[skip_front..skip_front + n], 2 * n, || rng.next_positive().into());
        }
        if !gigas.is_empty() {
            let n = gigas.len();
            shuffle(&mut gigas, 2 * n, || rng.next_positive().into());
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
        let mut tag_pair = None;
        if tags.is_some() && normal.len() == FOLDER_SIZE {
            let at = (rng.next_positive() % 19 + 1) as usize;
            normal.swap(FOLDER_SIZE - 2, at);
            normal.swap(FOLDER_SIZE - 1, at + 1);
            tag_pair = Some(at as u8);
        }
        let folder =
            BattleFolder { chips: std::array::from_fn(|i| normal.get(i).copied()), regular_pending: regular.is_some() };
        (folder, tag_pair)
    }
}

/// Shuffle `items` over `draws` values of `draw`: for i
/// from len − 1 down to 1, swap entry i with entry `draw() % (i + 1)`; the
/// draws after the first len − 1 are taken and dropped (`draws` is at least
/// len − 1). The game's shuffles take two draws a round: `sub_8000D12` on a
/// console's RNG1 (the folder, the re-deal) and `sub_8000C72` on RNG2
/// (panel lists), whose callers pass 2 × rounds (docs/engine/battle-flow.md
/// §3.1).
pub fn shuffle<T>(items: &mut [T], draws: usize, mut draw: impl FnMut() -> u64) {
    let used = items.len().saturating_sub(1);
    assert!(used <= draws, "a shuffle of {} entries needs {used} draws, not {draws}", items.len());
    for i in (1..items.len()).rev() {
        let j = (draw() % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
    for _ in used..draws {
        draw();
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

    /// Folder chips from the game's packed halfwords (`code << 9 | id`),
    /// on the test library (whose chip handles are their numbers).
    fn chips(packed: [u16; FOLDER_SIZE]) -> [FolderChip; FOLDER_SIZE] {
        packed.map(|v| FolderChip::new(ChipHandle(v & 0x1FF), ChipCode((v >> 9) as u8)))
    }

    /// `expected`: the folder shuffled from RNG1 at `rng1`;
    /// `rng1_next_frame`: RNG1 at the start of the next frame, after the
    /// main loop's own step. `recorded`: the recording's folder, whose
    /// Regular chip, tag pair and Gigas are in the places the expected
    /// folder has them.
    fn check(saved: SavedFolder, rng1: u32, recorded: [u16; FOLDER_SIZE], expected: [u16; FOLDER_SIZE], rng1_next_frame: u32) {
        let mut rng = Rng::new(rng1);
        let (f, tag_pair) = BattleFolder::shuffled_with_tag_pair(&saved, 0, &mut rng, &library());
        rng.next();
        assert_eq!(rng.state, rng1_next_frame, "RNG1 after the shuffle");
        assert_eq!(f.chips, chips(expected).map(Some));

        let recorded = chips(recorded);
        let sorted = |c: [FolderChip; FOLDER_SIZE]| {
            let mut c = c.map(|c| (c.id.0, c.code.0));
            c.sort();
            c
        };
        assert_eq!(sorted(chips(expected)), sorted(recorded), "the folder's chips");
        let library = library();
        let gigas = |c: [FolderChip; FOLDER_SIZE]| c.map(|c| library.chip(c.id).class == ChipClass::Giga);
        assert_eq!(gigas(chips(expected)), gigas(recorded), "the Gigas' places");
        if let Some(r) = saved.regular {
            assert_eq!(f.chips[0], Some(saved.chips[r as usize]), "the Regular chip first");
            assert_eq!(recorded[0], saved.chips[r as usize]);
        }
        if let Some((a, b)) = saved.tags {
            let at = tag_pair.expect("a tag pair placed") as usize;
            let pair = [saved.chips[a as usize], saved.chips[b as usize]];
            assert_eq!([f.chips[at].unwrap(), f.chips[at + 1].unwrap()], pair, "the tag pair together");
            assert_eq!(recorded[at..at + 2], pair, "the tag pair's place");
        }
    }

    // Folders, RNG1 states and the folders dealt (`recorded`), measured in
    // the original game at the init of recorded netbattles (battle mode 0),
    // on both consoles.

    #[test]
    fn plain_folder() {
        let mut saved = [0x34A7; FOLDER_SIZE];
        saved[10..].fill(0x1A11);
        let recorded = [
            0x34A7, 0x1A11, 0x1A11, 0x1A11, 0x1A11, 0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x1A11, 0x1A11,
            0x1A11, 0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x1A11, 0x34A7, 0x1A11, 0x1A11, 0x34A7, 0x1A11,
            0x34A7, 0x1A11, 0x1A11, 0x1A11,
        ];
        let expected = [
            0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x1A11, 0x1A11, 0x1A11, 0x1A11, 0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x1A11,
            0x1A11, 0x34A7, 0x1A11, 0x1A11, 0x1A11, 0x34A7, 0x1A11, 0x34A7, 0x1A11, 0x1A11, 0x34A7, 0x34A7, 0x34A7,
            0x1A11, 0x1A11, 0x1A11, 0x1A11,
        ];
        check(SavedFolder { chips: chips(saved), regular: None, tags: None }, 0x62FA_BD30, recorded, expected, 0x9EC7_5BBF);
    }

    #[test]
    fn regular_and_tag_chips() {
        let saved = [
            0x1602, 0x0001, 0x0201, 0x0201, 0x3404, 0x3404, 0x0605, 0x0605, 0x0605, 0x0236, 0x0236, 0x1636, 0x1636,
            0x2447, 0x2447, 0x2447, 0x2447, 0x2448, 0x2448, 0x34AF, 0x3459, 0x3459, 0x009A, 0x009A, 0x169A, 0x169A,
            0x24A3, 0x34C0, 0x34C0, 0x34B8,
        ];
        let recorded = [
            0x0001, 0x24A3, 0x1636, 0x3404, 0x2448, 0x009A, 0x169A, 0x0236, 0x34B8, 0x0605, 0x3459, 0x34C0, 0x0201,
            0x3404, 0x2447, 0x0605, 0x2448, 0x1636, 0x0605, 0x0236, 0x2447, 0x2447, 0x34C0, 0x2447, 0x0201, 0x34AF,
            0x1602, 0x3459, 0x009A, 0x169A,
        ];
        let expected = [
            0x0001, 0x3459, 0x2448, 0x34C0, 0x169A, 0x2447, 0x1602, 0x2447, 0x34AF, 0x2447, 0x24A3, 0x3459, 0x0201,
            0x3404, 0x2448, 0x009A, 0x0605, 0x0236, 0x34B8, 0x2447, 0x3404, 0x34C0, 0x0236, 0x1636, 0x1636, 0x0605,
            0x0605, 0x0201, 0x169A, 0x009A,
        ];
        let saved = SavedFolder { chips: chips(saved), regular: Some(1), tags: Some((3, 4)) };
        check(saved, 0x0B48_8EEC, recorded, expected, 0x40D3_D040);
    }

    #[test]
    fn giga_chip() {
        let saved = [
            0x2130, 0x00F4, 0x130C, 0x28B6, 0x32A9, 0x350A, 0x34EC, 0x20A5, 0x34BA, 0x350D, 0x12A1, 0x12A1, 0x002F,
            0x002F, 0x2011, 0x0027, 0x0027, 0x004D, 0x006D, 0x34B1, 0x34B1, 0x34BB, 0x34BB, 0x3450, 0x3450, 0x34A6,
            0x34A3, 0x34A3, 0x34A3, 0x34A3,
        ];
        let recorded = [
            0x3450, 0x00F4, 0x3450, 0x350D, 0x34BB, 0x34B1, 0x34EC, 0x34BA, 0x0027, 0x350A, 0x34BB, 0x20A5, 0x32A9,
            0x2011, 0x34A6, 0x34A3, 0x2130, 0x130C, 0x006D, 0x002F, 0x34A3, 0x34A3, 0x0027, 0x004D, 0x12A1, 0x28B6,
            0x34A3, 0x12A1, 0x34B1, 0x002F,
        ];
        let expected = [
            0x34B1, 0x32A9, 0x34A3, 0x350A, 0x002F, 0x34A6, 0x34BA, 0x34EC, 0x3450, 0x34BB, 0x004D, 0x2011, 0x12A1,
            0x0027, 0x34A3, 0x34BB, 0x2130, 0x28B6, 0x3450, 0x12A1, 0x34A3, 0x20A5, 0x350D, 0x006D, 0x34B1, 0x002F,
            0x00F4, 0x130C, 0x34A3, 0x0027,
        ];
        check(SavedFolder { chips: chips(saved), regular: None, tags: None }, 0x15EE_C9DC, recorded, expected, 0x5D37_B35D);
    }

    #[test]
    fn regular_chip_with_gigas() {
        let saved = [
            0x32B5, 0x342B, 0x34BC, 0x34BA, 0x34BB, 0x34BB, 0x34A3, 0x34A3, 0x28B4, 0x34AC, 0x32A9, 0x34B0, 0x34AE,
            0x28B6, 0x3450, 0x12A1, 0x12A1, 0x34B1, 0x34B1, 0x34B1, 0x0706, 0x130C, 0x34EC, 0x350A, 0x350D, 0x1B33,
            0x0B32, 0x0734, 0x1335, 0x2B36,
        ];
        let recorded = [
            0x34AE, 0x34A3, 0x34B1, 0x12A1, 0x34EC, 0x32A9, 0x342B, 0x34B0, 0x12A1, 0x34B1, 0x2B36, 0x34AC, 0x350D,
            0x1335, 0x28B4, 0x28B6, 0x34BA, 0x32B5, 0x34A3, 0x350A, 0x34B1, 0x0B32, 0x1B33, 0x34BB, 0x34BB, 0x0734,
            0x130C, 0x34BC, 0x0706, 0x3450,
        ];
        let expected = [
            0x34AE, 0x34EC, 0x342B, 0x34BB, 0x350A, 0x28B6, 0x32B5, 0x0706, 0x130C, 0x34A3, 0x0734, 0x350D, 0x34BA,
            0x2B36, 0x34A3, 0x34AC, 0x34B0, 0x34B1, 0x32A9, 0x34B1, 0x34B1, 0x1B33, 0x0B32, 0x12A1, 0x12A1, 0x1335,
            0x3450, 0x28B4, 0x34BB, 0x34BC,
        ];
        check(SavedFolder { chips: chips(saved), regular: Some(12), tags: None }, 0x2C38_F097, recorded, expected, 0xB4DC_B2B5);
    }

    /// 20,000 shuffles of 30 on 60 draws each, from fixed seeds: each chip
    /// lands in each place about 1 time in 30 (one standard deviation over
    /// 20,000 is about 0.0013), and the first hand of 5 holds about 5 × 5/30
    /// of the first 5 chips.
    #[test]
    fn each_position_is_equally_likely() {
        const N: usize = 30;
        const TRIALS: usize = 20_000;
        // `at[chip][place]`: shuffles that put `chip` in `place`.
        let mut at = [[0u32; N]; N];
        let mut first_hand = 0;
        for seed in 0..TRIALS as u32 {
            let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9) ^ 0x62FA_BD30);
            let mut chips: [u8; N] = std::array::from_fn(|i| i as u8);
            shuffle(&mut chips, 2 * N, || rng.next_positive().into());
            for (place, &chip) in chips.iter().enumerate() {
                at[chip as usize][place] += 1;
            }
            first_hand += chips[..5].iter().filter(|&&c| c < 5).count();
        }
        let rate = |n: u32| n as f64 / TRIALS as f64;
        let range = |r: &mut dyn Iterator<Item = f64>| r.fold((1.0f64, 0.0f64), |(lo, hi), x| (lo.min(x), hi.max(x)));
        let stays: Vec<f64> = (0..N).map(|i| rate(at[i][i])).collect();
        let (stay_low, stay_high) = range(&mut stays.iter().copied());
        let (any_low, any_high) = range(&mut at.iter().flatten().map(|&n| rate(n)));
        let first_hand = first_hand as f64 / TRIALS as f64;
        eprintln!(
            "stays {:.4} on average ({stay_low:.4}..{stay_high:.4}), any chip in any place {any_low:.4}..{any_high:.4}, \
             first hand {first_hand:.3}",
            stays.iter().sum::<f64>() / N as f64,
        );
        // 1/30 ≈ 0.0333, one standard deviation over 20,000 ≈ 0.0013.
        assert!(any_low > 0.026 && any_high < 0.041, "any chip in any place: {any_low:.4}..{any_high:.4}");
        assert!(stay_low > 0.026 && stay_high < 0.041, "a chip stays put: {stay_low:.4}..{stay_high:.4}");
        assert!((first_hand - 5.0 * 5.0 / N as f64).abs() < 0.03, "first hand: {first_hand:.3}");
    }

    /// A shuffle takes all its draws however many entries it has: MetaGel's
    /// 4 rounds over 3 rows, a folder's 30 over 30.
    #[test]
    fn a_shuffle_takes_all_its_draws() {
        for (n, rounds) in [(0, 0), (1, 1), (3, 4), (3, 3), (18, 20), (30, 30), (64, 64)] {
            let mut items: Vec<usize> = (0..n).collect();
            let mut taken = 0;
            shuffle(&mut items, 2 * rounds, || {
                taken += 1;
                taken as u64 * 0x2545_F491
            });
            assert_eq!(taken, 2 * rounds, "{n} entries, {rounds} rounds");
            items.sort();
            assert_eq!(items, (0..n).collect::<Vec<_>>());
        }
    }

    /// Three rows on MetaGel's 8 draws: each of the 6 orders about 1,000 times in 6,000.
    #[test]
    fn three_rows_on_metagels_draws() {
        let mut seen = std::collections::BTreeMap::<[u8; 3], u32>::new();
        for seed in 0..6000u32 {
            let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9));
            let mut rows = [1u8, 2, 3];
            shuffle(&mut rows, 8, || rng.next_positive().into());
            *seen.entry(rows).or_default() += 1;
        }
        assert_eq!(seen.len(), 6);
        assert!(seen.values().all(|&n| (850..1150).contains(&n)), "{seen:?}");
    }
}
