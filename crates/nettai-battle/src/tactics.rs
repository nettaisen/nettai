//! A player's tactics: BN5's computer-navi data (0xE0 bytes a player at
//! 0x02034C20), which each console builds from its save at its last
//! battle's end (0x0802C540, from the chips its player used most and the
//! runs of them it played from a place by its target) and the link
//! exchanges as a battle starts (0x08009B9A). A computer navi on the other
//! side plays them (BN5's Dark MegaMan, docs/design/bn5-map.md §15.9): the
//! entries in order, a chip or a pattern (a place by its target and a run
//! of chips). Each side's tactics are battle state: the computer navi's AI
//! turns their entries as it plays them.

use nettai_content_api::ChipHandle;

/// Most entries a player's tactics hold (0x0802BEB0's loop: 42).
pub const MAX_ENTRIES: usize = 42;
/// Most patterns (the block's 16-byte records from +0x58: 8).
pub const MAX_PATTERNS: usize = 8;
/// Most chips a pattern runs (its record's seven halfwords after its
/// place, the last its end).
pub const MAX_PATTERN_CHIPS: usize = 6;

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
/// computer navi's enemies, `dy` rows) and the chips to use there, in
/// order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TacticPattern {
    pub dx: i8,
    pub dy: i8,
    pub chips: Vec<ChipHandle>,
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
