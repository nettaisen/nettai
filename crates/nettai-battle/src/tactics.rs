//! A player's tactics: EXE5's computer-navi data (0xE0 bytes a player at
//! 0x02034C20), which each console builds from its save at its last
//! battle's end (0x0802C540, from the chips its player used most and the
//! runs of them it played from a place by its target) and the link
//! exchanges as a battle starts (0x08009B9A). A computer navi on the other
//! side plays them (EXE5's Dark MegaMan, docs/design/exe5-map.md §15.9): the
//! entries in order, a chip or a pattern (a place by its target and a run
//! of chips). Each side's tactics are battle state: the computer navi's AI
//! turns their entries as it plays them.
//!
//! A pattern's record is 16 bytes: its place (two signed bytes), five chip
//! places (halfwords, to the first 0xFFFF) and its score (a word: what the
//! save's learning ranks its patterns by, 0x0802C540). The AI reads a
//! pattern's chips to the first 0xFFFF with no other end (0x0802BCD6), so
//! from a record whose five chip places are all filled it reads on into
//! the score: a pattern carries it.

use nettai_content_api::ChipHandle;

/// Most entries a player's tactics hold (0x0802BEB0's loop: 42).
pub const MAX_ENTRIES: usize = 42;
/// Most patterns (the block's 16-byte records from +0x58: 8).
pub const MAX_PATTERNS: usize = 8;
/// Most chips a pattern runs (its record's five chip places, between its
/// place and its score).
pub const MAX_PATTERN_CHIPS: usize = 5;

/// An entry of a player's tactics: a halfword of the block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tactic {
    /// A chip.
    Chip(ChipHandle),
    /// A pattern, by its place among the patterns (bit 15 with its index).
    Pattern(u8),
    /// Nothing (the halfword 0, chip 0: a block no save filled).
    Nothing,
    /// An empty place (0xFFFF), which a turn of the entries leaves.
    Empty,
}

/// A pattern: where to stand from the target (`dx` columns toward the
/// computer navi's enemies, `dy` rows), the chips to use there, in order,
/// and the record's score (its word at +12, little-endian: its low half
/// the halfword at +12, which follows the fifth chip place, its high half
/// the one at +14).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TacticPattern {
    pub dx: i8,
    pub dy: i8,
    pub chips: Vec<ChipHandle>,
    pub score: u32,
}

/// A player's tactics.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Tactics {
    /// The entries (their count is the block's +0x54).
    pub entries: Vec<Tactic>,
    pub patterns: Vec<TacticPattern>,
}

impl Tactics {
    /// The entry in place `i` (from 0): past the count, an empty place.
    pub fn get(&self, i: usize) -> Tactic {
        self.entries.get(i).copied().unwrap_or(Tactic::Empty)
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
    pub fn sent(&self, rng: &mut crate::rng::Rng) -> Tactics {
        let mut places = self.entries.clone();
        places.resize(MAX_ENTRIES, Tactic::Empty);
        let mut shuffle = |places: &mut [Tactic], swaps: u32| {
            let n = places.len() as u32;
            for _ in 0..swaps {
                let a = rng.next_positive() % n;
                let b = rng.next_positive() % n;
                places.swap(a as usize, b as usize);
            }
        };
        shuffle(&mut places[..3], 3);
        shuffle(&mut places[3..], 39);
        let entries = places.into_iter().filter(|&e| e != Tactic::Empty).collect();
        Tactics { entries, patterns: self.patterns.clone() }
    }

    /// 0x0802BF1C's list part: the first entry goes last, the rest move up
    /// (`sub_8000EB6` packs the entries but the emptied first; the count
    /// stays). With no entries the original loops 2^32 times.
    pub fn turn(&mut self) -> Result<(), String> {
        if self.entries.is_empty() {
            return Err("a computer navi turns empty tactics (0x0802BF1C: sub_8000EB6 counts 2^32 entries)".into());
        }
        let first = self.entries[0];
        self.entries[0] = Tactic::Empty;
        let mut packed: Vec<Tactic> = self.entries.iter().copied().filter(|&e| e != Tactic::Empty).collect();
        packed.resize(self.entries.len(), Tactic::Empty);
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
        let chip = |n: u16| Tactic::Chip(ChipHandle(n));
        let t = Tactics { entries: vec![chip(1), Tactic::Empty, chip(2), Tactic::Nothing, Tactic::Pattern(0)], patterns: Vec::new() };
        let mut rng = Rng::new(7);
        let sent = t.sent(&mut rng);
        let mut after = Rng::new(7);
        for _ in 0..84 {
            after.next_positive();
        }
        assert_eq!(rng, after);
        assert_eq!(sent.entries.len(), 4);
        for e in [chip(1), chip(2), Tactic::Nothing, Tactic::Pattern(0)] {
            assert!(sent.entries.contains(&e), "{e:?} in {:?}", sent.entries);
        }
        assert_eq!(Tactics::default().sent(&mut Rng::new(7)), Tactics::default());
    }
}
