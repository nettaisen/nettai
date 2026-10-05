//! A player's auto battle data: EXE5's (0xE0 bytes a player at
//! 0x02034C20), which each console builds from its save at its last
//! battle's end (0x0802C540, from the chips its player used most and the
//! runs of them it played from a place by its target) and the link
//! exchanges as a battle starts (0x08009B9A). A navi in auto battle on the other
//! side plays it (EXE5's Dark MegaMan, docs/design/exe5-map.md §15.9): the
//! entries in order, a chip or a pattern (a place by its target and a run
//! of chips). Each side's data is battle state: the auto-battling navi's AI
//! turns its entries as it plays them.
//!
//! The block: 42 halfword places, their count (+0x54), eight pattern
//! records of 16 bytes from +0x58 (`dx`, `dy`, five chip places, the
//! pattern's score, a word) and eight bytes more, which nothing writes
//! (0xFF in every save and every block a battle exchanged).

use nettai_content_api::ChipHandle;

/// Most entries a player's auto battle data holds (0x0802BEB0's loop: 42).
pub const MAX_ENTRIES: usize = 42;
/// Most patterns (the block's 16-byte records from +0x58: 8).
pub const MAX_PATTERNS: usize = 8;
/// The chip places of a pattern's record.
pub const PATTERN_CHIPS: usize = 5;

/// An entry of a player's auto battle data: a halfword of the block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AutoBattleEntry {
    /// A chip.
    Chip(ChipHandle),
    /// A pattern, by its place among the patterns (bit 15 with its index).
    Pattern(u8),
    /// Nothing (the halfword 0, chip 0: a block no save filled).
    Nothing,
    /// An empty place (0xFFFF), which a turn of the entries leaves.
    Empty,
}

/// A chip place of a pattern's record: a halfword.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PatternChip {
    Chip(ChipHandle),
    /// The halfword 0, which the AI plays as the game's chip 0 (a zeroed
    /// record's: a played save's unused ones can be).
    Nothing,
    /// An empty place (0xFFFF): the run's end.
    Empty,
}

/// A pattern record: where to stand from the target (`dx` columns toward
/// the auto-battling navi's enemies, `dy` rows), the chips to use there, in
/// order, to the first empty place, and the pattern's score, which the
/// game's learning keeps (a battle's end takes 1 off it, or adds 5 to one
/// the battle saw again, and writes the eight highest: 0x0802C540) and
/// nothing of a battle means to read. The AI's read of a record all of
/// whose places hold a chip reaches it all the same
/// ([`AutoBattleData::pattern_read`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PatternRecord {
    pub dx: i8,
    pub dy: i8,
    pub chips: [PatternChip; PATTERN_CHIPS],
    pub score: u32,
}

impl PatternRecord {
    /// A record nothing has written (0xFF throughout): a save's that has
    /// learned no pattern for it.
    pub const UNUSED: PatternRecord = PatternRecord { dx: -1, dy: -1, chips: [PatternChip::Empty; PATTERN_CHIPS], score: u32::MAX };

    /// A pattern of `chips` (the first [`PATTERN_CHIPS`] of them), the
    /// places after them empty.
    pub fn of(dx: i8, dy: i8, chips: &[ChipHandle], score: u32) -> PatternRecord {
        let mut places = [PatternChip::Empty; PATTERN_CHIPS];
        for (place, &c) in places.iter_mut().zip(chips) {
            *place = PatternChip::Chip(c);
        }
        PatternRecord { dx, dy, chips: places, score }
    }

    /// The chips of its run: those before its first place that holds none.
    pub fn run(&self) -> Vec<ChipHandle> {
        self.chips
            .iter()
            .map_while(|c| match c {
                PatternChip::Chip(h) => Some(*h),
                _ => None,
            })
            .collect()
    }
}

impl Default for PatternRecord {
    fn default() -> PatternRecord {
        PatternRecord::UNUSED
    }
}

/// What the AI's read of a pattern finds ([`AutoBattleData::pattern_read`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternRead {
    /// A chip place's chip.
    Chip(ChipHandle),
    /// A halfword that is no chip place's chip, which the game takes for a
    /// chip's number all the same (its chip table's record of that number):
    /// a place holding 0, half of a score, a record's `dx` and `dy`.
    Number(u16),
    /// An empty place (0xFFFF): the run's end.
    End,
}

/// A player's auto battle data.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AutoBattleData {
    /// The entries (their count is the block's +0x54).
    pub entries: Vec<AutoBattleEntry>,
    /// The pattern records in their places, from the first (those past the
    /// last here, unused).
    pub patterns: Vec<PatternRecord>,
}

impl AutoBattleData {
    /// 0x0802BCD6: the `k`th halfword (from 0) the AI reads of pattern
    /// `pattern`'s run, the one after its record's `dx` and `dy` being the
    /// first. The read has no end but a halfword of 0xFFFF and takes any
    /// other for a chip's number (0x0802C094), so past a record's five
    /// chip places it goes on: the two halves of its score, the next
    /// record's `dx` and `dy` as one halfword, that record's places and
    /// score, and so through the records to the block's last eight bytes,
    /// which are 0xFF.
    pub fn pattern_read(&self, pattern: usize, k: usize) -> PatternRead {
        /// The halfwords of a record: its place, its chip places, its score.
        const RECORD: usize = 3 + PATTERN_CHIPS;
        let at = pattern * RECORD + 1 + k;
        let (record, field) = (at / RECORD, at % RECORD);
        if record >= MAX_PATTERNS {
            return PatternRead::End;
        }
        let p = self.patterns.get(record).copied().unwrap_or(PatternRecord::UNUSED);
        let number = |n: u16| if n == 0xFFFF { PatternRead::End } else { PatternRead::Number(n) };
        match field {
            0 => number(u16::from_le_bytes([p.dx as u8, p.dy as u8])),
            f if f <= PATTERN_CHIPS => match p.chips[f - 1] {
                PatternChip::Chip(c) => PatternRead::Chip(c),
                PatternChip::Nothing => PatternRead::Number(0),
                PatternChip::Empty => PatternRead::End,
            },
            f if f == PATTERN_CHIPS + 1 => number(p.score as u16),
            _ => number((p.score >> 16) as u16),
        }
    }

    /// The entry in place `i` (from 0): past the count, an empty place.
    pub fn get(&self, i: usize) -> AutoBattleEntry {
        self.entries.get(i).copied().unwrap_or(AutoBattleEntry::Empty)
    }

    /// 0x0802C0DC / 0x0802C0D6's swap: the first entry and entry `i`
    /// change places.
    pub fn swap_first(&mut self, i: usize) {
        if i < self.entries.len() {
            self.entries.swap(0, i);
        }
    }

    /// EXE5's 0x0802C7BE: the block a console sends as a battle starts, from
    /// its player's (the save's): the first three places shuffled by three
    /// swaps, the next 39 by 39 (each swap two places drawn from `rng`,
    /// RNG2 then: `sub_8000CDA`), the entries packed to the front
    /// (`sub_8000EB6`) and counted up to the first empty place.
    pub fn sent(&self, rng: &mut crate::rng::Rng) -> AutoBattleData {
        let mut places = self.entries.clone();
        places.resize(MAX_ENTRIES, AutoBattleEntry::Empty);
        let mut shuffle = |places: &mut [AutoBattleEntry], swaps: u32| {
            let n = places.len() as u32;
            for _ in 0..swaps {
                let a = rng.next_positive() % n;
                let b = rng.next_positive() % n;
                places.swap(a as usize, b as usize);
            }
        };
        shuffle(&mut places[..3], 3);
        shuffle(&mut places[3..], 39);
        let entries = places.into_iter().filter(|&e| e != AutoBattleEntry::Empty).collect();
        AutoBattleData { entries, patterns: self.patterns.clone() }
    }

    /// 0x0802BF1C's list part: the first entry goes last, the rest move up
    /// (`sub_8000EB6` packs the entries but the emptied first; the count
    /// stays). With no entries the original loops 2^32 times.
    pub fn turn(&mut self) -> Result<(), String> {
        if self.entries.is_empty() {
            return Err("a navi in auto battle turns auto battle data with no entries (0x0802BF1C: sub_8000EB6 counts 2^32 entries)".into());
        }
        let first = self.entries[0];
        self.entries[0] = AutoBattleEntry::Empty;
        let mut packed: Vec<AutoBattleEntry> = self.entries.iter().copied().filter(|&e| e != AutoBattleEntry::Empty).collect();
        packed.resize(self.entries.len(), AutoBattleEntry::Empty);
        let last = packed.len() - 1;
        packed[last] = first;
        self.entries = packed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    /// The send keeps what the block holds, packed to the front (the empty
    /// places gone), and draws two numbers a swap: 84.
    #[test]
    fn the_send_packs_the_shuffled_block() {
        let chip = |n: u16| AutoBattleEntry::Chip(ChipHandle(n));
        let t = AutoBattleData { entries: vec![chip(1), AutoBattleEntry::Empty, chip(2), AutoBattleEntry::Nothing, AutoBattleEntry::Pattern(0)], patterns: Vec::new() };
        let mut rng = Rng::new(7);
        let sent = t.sent(&mut rng);
        let mut after = Rng::new(7);
        for _ in 0..84 {
            after.next_positive();
        }
        assert_eq!(rng, after);
        assert_eq!(sent.entries.len(), 4);
        for e in [chip(1), chip(2), AutoBattleEntry::Nothing, AutoBattleEntry::Pattern(0)] {
            assert!(sent.entries.contains(&e), "{e:?} in {:?}", sent.entries);
        }
        assert_eq!(AutoBattleData::default().sent(&mut Rng::new(7)), AutoBattleData::default());
    }

    /// 0x0802BCD6's read ends at an empty place alone: a record of five
    /// chips is read on into its score and the records after it.
    #[test]
    fn a_patterns_read_ends_at_an_empty_place_alone() {
        use PatternRead::{Chip, End, Number};
        let c = ChipHandle;
        let reads = |t: &AutoBattleData, pattern: usize| -> Vec<PatternRead> {
            (0..).map(|k| t.pattern_read(pattern, k)).take_while(|r| *r != End).collect()
        };
        // Two chips: its run, and no more.
        let short = PatternRecord::of(1, 0, &[c(7), c(8)], 3);
        let t = AutoBattleData { entries: Vec::new(), patterns: vec![short] };
        assert_eq!(reads(&t, 0), [Chip(c(7)), Chip(c(8))]);
        // Five: then the score's halves; an unused record after it ends
        // the read at its place (0xFF, 0xFF).
        let full = PatternRecord::of(1, 0, &[c(1), c(2), c(3), c(4), c(5)], 3);
        let t = AutoBattleData { entries: Vec::new(), patterns: vec![full] };
        assert_eq!(reads(&t, 0), [Chip(c(1)), Chip(c(2)), Chip(c(3)), Chip(c(4)), Chip(c(5)), Number(3), Number(0)]);
        // A pattern after it: its place as a number (dx the low byte), then
        // its chips.
        let t = AutoBattleData { entries: Vec::new(), patterns: vec![full, PatternRecord::of(2, -1, &[c(9)], 1)] };
        assert_eq!(reads(&t, 0)[5..], [Number(3), Number(0), Number(0xFF02), Chip(c(9))]);
        assert_eq!(reads(&t, 1), [Chip(c(9))]);
        // A score of 0xFFFFFFFF ends it after the chips; one whose upper
        // half alone is 0xFFFF, after its lower half.
        let t = AutoBattleData { entries: Vec::new(), patterns: vec![PatternRecord { score: u32::MAX, ..full }] };
        assert_eq!(reads(&t, 0).len(), 5);
        let t = AutoBattleData { entries: Vec::new(), patterns: vec![PatternRecord { score: 0xFFFF_0004, ..full }] };
        assert_eq!(reads(&t, 0)[5..], [Number(4)]);
        // Zeroed records after it (a played save's unused ones): chip 0 a
        // halfword, to the block's last bytes past the eighth record.
        let zeroed = PatternRecord { dx: 0, dy: 0, chips: [PatternChip::Nothing; PATTERN_CHIPS], score: 0 };
        let mut patterns = vec![zeroed; MAX_PATTERNS];
        patterns[6] = full;
        let t = AutoBattleData { entries: Vec::new(), patterns };
        let r = reads(&t, 6);
        assert_eq!(r.len(), 5 + 2 + 8);
        assert!(r[7..].iter().all(|x| *x == Number(0)));
        assert_eq!(t.pattern_read(7, 7), End);
    }
}
