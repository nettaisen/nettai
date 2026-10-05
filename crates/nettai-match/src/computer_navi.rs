//! EXE5's computer-navi data, as a match states it for a side: what a
//! computer navi plays from that player's save.
//!
//! **What it is.** An EXE5 save keeps a block of 0xE0 bytes for its player
//! (save +0x554C, the toolkit's +0x78; `nettai_battle::tactics`): 42
//! places, each a chip, a pattern or empty, and eight pattern records, each
//! a place by a target (`dx` columns toward the computer navi's enemies,
//! `dy` rows), up to five chips used there in a row, and the pattern's
//! score. A computer navi plays it (content/exe5/rules/computer-navi): the
//! Dark MegaMan a failed Chaos Unison brings plays the data of the player
//! whose Chaos Unison failed, and a navi under DarkInvs its own side's. It
//! plays the entries in order, the played one going last: three times in
//! four the first, and otherwise, or with none, it steps into an enemy's
//! row and fires its buster.
//!
//! **Where it comes from.** The game learns it from its own player
//! alone. A battle counts each chip the player uses (0x0802C1FC, by the
//! chip's class: a standard or mega chip 1 or 3 a use, a giga chip or a
//! program advance 1) and remembers each run of chips used from one column
//! within 30 ticks of each other, two or more, with where its first hits
//! landed from there (0x0802C294, 0x0802C3E2, 0x0802C2DC: a pattern of up
//! to five chips, eight a battle, each starting at a score of 10). The
//! battle's end (0x0802C540) scores the patterns (16 kept: one seen again
//! gains 5, the others lose 1, not under 1) and writes the block from the
//! counts, in six lists ([`LISTS`]):
//!
//! - `first`, places 1 to 3: the three most counted standard chips of a
//!   second count (save +0x2340), which the battle's code never raises (its
//!   weight is always 0, 0x0802C220), so they are empty in every US save
//!   seen (one Japanese save has them filled: nothing here knows what
//!   filled them);
//! - `standard`, places 4 to 27: the sixteen most used standard chips, the
//!   two most used four times each, the next two twice, the rest once;
//! - `mega`, places 28 to 32: the five most used mega chips;
//! - `giga`, place 33: the most used giga chip;
//! - `patterns`, places 34 to 41: the patterns, up to eight, the highest
//!   scored first;
//! - `program_advance`, place 42: the most used program advance.
//!
//! Each list is filled from its first place, the places left over empty. A
//! new save's block is empty (0xFFFF throughout: nothing learned), as are
//! the six blocks after it, which nothing writes.
//!
//! **What a battle makes of it.** As a battle starts each console sends its
//! block shuffled (0x0802C7BE, `Tactics::sent`): three swaps among the
//! first three places, 39 swaps among the other 39 (each swap two places
//! drawn at random), the entries then packed to the front. That is no even
//! shuffle: a place is in none of 39 swaps about one time in eight, so the
//! entry in place 4 leads the sent list far more often than another, and an
//! empty place between two entries changes what a seed sends. So a match
//! states every place: the six lists by the places they have, an entry a
//! chip, a pattern or an empty place. They are named for what the game
//! writes there; any entry may stand in any place, as in the block (a save
//! made by hand can have a giga chip where the game writes patterns).
//!
//! **What a match leaves out of the block**, none of it read by a battle:
//! which record a pattern is in (the AI reaches a pattern through the entry
//! naming it: here the records are the entries' patterns in the order they
//! come, as the game's own write has them) and records no entry names; the
//! count at +0x54, which the send writes; the block's last eight bytes. A
//! place holding 0 (no chip) isn't stated either: no save the game wrote
//! has one.
//!
//! **A pattern's score** is read by a battle in one case: the AI reads a
//! pattern's chips to the first empty place with no other end (0x0802BCD6),
//! so from a pattern that fills its five chip places it reads on into the
//! score. A pattern of five chips states its score; another may.
//!
//! **What the game can't hold** is refused ([`ComputerNavi::check`], and a
//! file's own reading): more entries than a list has places, more than
//! [`PATTERNS`] different patterns, a pattern of more than
//! [`PATTERN_CHIPS`] chips or of none, a pattern of five chips without its
//! score, a pattern's place off the field from any target, a chip the game
//! hasn't, and any of it for a game whose rules have no computer navis
//! (EXE6).

use crate::ids;
use nettai_battle::content::{ChipClass, Content};
use nettai_battle::tactics::{MAX_ENTRIES, MAX_PATTERN_CHIPS, MAX_PATTERNS, Tactic, TacticPattern, Tactics};
use nettai_content_api::ChipHandle;

/// The system that drives the computer navis (EXE5's).
pub const SYSTEM: &str = "computer-navi";
/// The block's places, and how many of them (the first) the send shuffles
/// apart from the rest.
pub const PLACES: usize = MAX_ENTRIES;
pub const FIRST: usize = 3;
/// The block's pattern records, and the chips a record has places for.
pub const PATTERNS: usize = MAX_PATTERNS;
pub const PATTERN_CHIPS: usize = MAX_PATTERN_CHIPS;
/// The furthest a pattern's place can be from a target on a field of six
/// columns and three rows.
pub const MAX_DX: i8 = 5;
pub const MAX_DY: i8 = 2;
/// The score a battle gives a pattern it has just seen (0x0802C4D0).
pub const NEW_SCORE: u32 = 10;
/// The score bytes of a pattern that states none (a record's bytes left as
/// a blank block has them).
const NO_SCORE: u32 = 0xFFFF_FFFF;

/// What the game writes into a list of the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holds {
    /// Chips of a class: its player's most used.
    Chips(ChipClass),
    Patterns,
}

/// A list of the data: its name (a match file's key), the places it has
/// (from `start`, counting from 0), and what the game writes there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct List {
    pub name: &'static str,
    pub start: usize,
    pub len: usize,
    pub holds: Holds,
}

impl List {
    /// The list's places.
    pub fn places(&self) -> std::ops::Range<usize> {
        self.start..self.start + self.len
    }
}

/// The data's six lists, in the block's order (0x0802C692 to 0x0802C6E6).
pub const LISTS: [List; 6] = [
    List { name: "first", start: 0, len: FIRST, holds: Holds::Chips(ChipClass::Standard) },
    List { name: "standard", start: 3, len: 24, holds: Holds::Chips(ChipClass::Standard) },
    List { name: "mega", start: 27, len: 5, holds: Holds::Chips(ChipClass::Mega) },
    List { name: "giga", start: 32, len: 1, holds: Holds::Chips(ChipClass::Giga) },
    List { name: "patterns", start: 33, len: PATTERNS, holds: Holds::Patterns },
    List { name: "program_advance", start: 41, len: 1, holds: Holds::Chips(ChipClass::ProgramAdvance) },
];
/// The lists by name.
pub const STANDARD: List = LISTS[1];
pub const MEGA: List = LISTS[2];
pub const GIGA: List = LISTS[3];
pub const PROGRAM_ADVANCE: List = LISTS[5];

/// The list place `place` (from 0) is in.
pub fn list_of(place: usize) -> &'static List {
    LISTS.iter().find(|l| l.places().contains(&place)).unwrap_or(&LISTS[LISTS.len() - 1])
}

/// How many times the game writes each of its player's sixteen most used
/// standard chips into the standard list (0x0802C790).
const STANDARD_TIMES: [usize; 16] = [4, 4, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];

/// A pattern: where the computer navi stands from its target (`dx` columns
/// toward its enemies, so a negative one is short of the target; `dy` rows,
/// down the screen), the chips it uses there, in order, and the pattern's
/// score as the game's learning keeps it (a new pattern's is 10; one seen
/// again in a battle gains 5, the others lose 1): what the AI reads on into
/// from a pattern of five chips, which states it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pattern {
    pub dx: i8,
    pub dy: i8,
    pub chips: Vec<ChipHandle>,
    pub score: Option<u32>,
}

/// An entry of the data: a chip, or a pattern.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Play {
    Chip(ChipHandle),
    Pattern(Pattern),
}

/// A player's computer-navi data: the block's places in order, each an
/// entry or empty. All empty: a save that has learned nothing (the computer
/// navi only fires its buster).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComputerNavi {
    pub places: [Option<Play>; PLACES],
}

impl Default for ComputerNavi {
    fn default() -> ComputerNavi {
        ComputerNavi { places: std::array::from_fn(|_| None) }
    }
}

/// Whether the game's rules have computer navis (a side takes the data).
pub fn has(content: &Content) -> bool {
    crate::ruleset_has_system(content, SYSTEM)
}

impl ComputerNavi {
    pub fn is_empty(&self) -> bool {
        self.places.iter().all(Option::is_none)
    }

    /// The entries, in the places' order.
    pub fn plays(&self) -> impl Iterator<Item = &Play> {
        self.places.iter().flatten()
    }

    /// A list's places.
    pub fn list(&self, list: &List) -> &[Option<Play>] {
        &self.places[list.places()]
    }

    /// The different patterns among the entries, in the order they come.
    pub fn patterns(&self) -> Vec<&Pattern> {
        let mut out: Vec<&Pattern> = Vec::new();
        for p in self.plays() {
            if let Play::Pattern(p) = p
                && !out.contains(&p)
            {
                out.push(p);
            }
        }
        out
    }

    /// The block a save holds of it (the engine's, in place order): each
    /// place its entry, each different pattern a record, in the order the
    /// entries come.
    pub fn tactics(&self) -> Tactics {
        let patterns = self.patterns();
        let entries = self
            .places
            .iter()
            .map(|p| match p {
                None => Tactic::Empty,
                Some(Play::Chip(c)) => Tactic::Chip(*c),
                Some(Play::Pattern(p)) => Tactic::Pattern(patterns.iter().position(|x| *x == p).expect("one of the entries' patterns") as u8),
            })
            .collect();
        let patterns =
            patterns.into_iter().map(|p| TacticPattern { dx: p.dx, dy: p.dy, chips: p.chips.clone(), score: p.score.unwrap_or(NO_SCORE) }).collect();
        Tactics { entries, patterns }
    }

    /// The data a block in place order holds (a save's): each place's
    /// entry, a pattern entry with its record. What it holds that a match
    /// doesn't state is left out and said: the places holding 0 (no chip:
    /// no save the game wrote has one), an entry for a pattern the block
    /// hasn't, entries past the block's places.
    pub fn of_block(block: &Tactics) -> (ComputerNavi, Vec<String>) {
        let mut notes = Vec::new();
        let mut out = ComputerNavi::default();
        let mut zeros = 0;
        for (i, e) in block.entries.iter().enumerate() {
            let play = match *e {
                Tactic::Empty => continue,
                Tactic::Chip(c) => Play::Chip(c),
                Tactic::Nothing => {
                    zeros += 1;
                    continue;
                }
                Tactic::Pattern(n) => match block.patterns.get(n as usize) {
                    Some(p) => Play::Pattern(Pattern { dx: p.dx, dy: p.dy, chips: p.chips.clone(), score: Some(p.score) }),
                    None => {
                        notes.push(format!("place {} of the computer-navi data names pattern {}, which it hasn't: left out", i + 1, n as u16 + 1));
                        continue;
                    }
                },
            };
            match out.places.get_mut(i) {
                Some(place) => *place = Some(play),
                None => {
                    notes.push(format!("the computer-navi data has {} entries; a block has {PLACES} places: the rest left out", block.entries.len()));
                    break;
                }
            }
        }
        if zeros > 0 {
            notes.push(format!("{zeros} of the computer-navi data's places hold no chip (0): left out"));
        }
        (out, notes)
    }

    /// The data as the game writes it at a battle's end (0x0802C540) from
    /// how often its player has used each chip (`uses`: a chip and its
    /// count, each chip once): the sixteen most used standard chips (the two
    /// most used four times each, the next two twice, the rest once), the
    /// five most used mega chips, the most used giga chip and the most used
    /// program advance, each list in its places; no patterns, and nothing
    /// in the first three places (the US games' second count stays 0).
    /// Chips used equally often come as the game's sort leaves them
    /// (0x0814301C: the higher chip number first).
    pub fn learned(content: &Content, uses: &[(ChipHandle, u32)]) -> ComputerNavi {
        let number = |c: ChipHandle| exe5_compat::Compat::exe5().chip_entry(ids::local(&content.defs.chip(c).key)).map_or(c.0, |e| e.id);
        let most = |class: ChipClass| -> Vec<ChipHandle> {
            let mut list: Vec<(u32, u16, ChipHandle)> =
                uses.iter().filter(|(c, n)| *n > 0 && content.chip(*c).class == class).map(|&(c, n)| (n, number(c), c)).collect();
            list.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
            list.into_iter().map(|x| x.2).collect()
        };
        let mut out = ComputerNavi::default();
        let mut write = |list: &List, chips: &mut dyn Iterator<Item = ChipHandle>| {
            for (place, chip) in list.places().zip(chips) {
                out.places[place] = Some(Play::Chip(chip));
            }
        };
        let standard = most(ChipClass::Standard);
        write(&STANDARD, &mut standard.iter().zip(STANDARD_TIMES).flat_map(|(&chip, times)| std::iter::repeat_n(chip, times)));
        write(&MEGA, &mut most(ChipClass::Mega).into_iter());
        write(&GIGA, &mut most(ChipClass::Giga).into_iter());
        write(&PROGRAM_ADVANCE, &mut most(ChipClass::ProgramAdvance).into_iter());
        out
    }

    /// The data the game would have learned from a player who used each
    /// chip of `folder` once ([`ComputerNavi::learned`], each chip counted
    /// as often as the folder holds it).
    pub fn of_folder(content: &Content, folder: &crate::Folder) -> ComputerNavi {
        let mut uses: Vec<(ChipHandle, u32)> = Vec::new();
        for c in folder.chips() {
            match uses.iter_mut().find(|(id, _)| *id == c.id) {
                Some((_, n)) => *n += 1,
                None => uses.push((c.id, 1)),
            }
        }
        ComputerNavi::learned(content, &uses)
    }

    /// What is wrong with the data for a side of a match of `game`: what
    /// the game can't hold.
    pub fn check(&self, content: &Content, game: &str) -> Vec<String> {
        let mut out = Vec::new();
        if self.is_empty() {
            return out;
        }
        if !has(content) {
            out.push(format!("computer-navi data, but {game} has no computer navis (no {SYSTEM} system)"));
        }
        let patterns = self.patterns();
        if patterns.len() > PATTERNS {
            out.push(format!("the computer navi's data has {} different patterns; it holds {PATTERNS}", patterns.len()));
        }
        for (i, p) in patterns.iter().enumerate() {
            let at = format!("the computer navi's pattern {}", i + 1);
            if p.chips.is_empty() {
                out.push(format!("{at} has no chips"));
            }
            if p.chips.len() > PATTERN_CHIPS {
                out.push(format!("{at} has {} chips; a pattern holds {PATTERN_CHIPS}", p.chips.len()));
            }
            if p.chips.len() == PATTERN_CHIPS && p.score.is_none() {
                out.push(format!(
                    "{at} has {PATTERN_CHIPS} chips and no score: a pattern that fills its chips' places is read on into its score, so it states it (a new pattern's is {NEW_SCORE})"
                ));
            }
            if p.dx.unsigned_abs() > MAX_DX as u8 || p.dy.unsigned_abs() > MAX_DY as u8 {
                out.push(format!(
                    "{at} is {} columns and {} rows from its target; no panel is more than {MAX_DX} columns and {MAX_DY} rows from another",
                    p.dx, p.dy
                ));
            }
        }
        let foreign = |c: &ChipHandle| c.index() >= content.defs.chips.len() || !ids::in_game(content, game, &content.defs.chip(*c).key);
        let chips = self.plays().flat_map(|p| match p {
            Play::Chip(c) => std::slice::from_ref(c),
            Play::Pattern(p) => p.chips.as_slice(),
        });
        if chips.into_iter().any(foreign) {
            out.push(format!("the computer navi's data names a chip {game} hasn't"));
        }
        out
    }

    /// The data in a line, for the terminal: each list that has entries by
    /// its name, its entries by theirs (a chip there several times once
    /// with its count, an empty place before an entry as `-`).
    pub fn describe(&self, content: &Content) -> String {
        let mut lists = Vec::new();
        for list in &LISTS {
            let places = self.list(list);
            let Some(last) = places.iter().rposition(Option::is_some) else { continue };
            let mut seen: Vec<(String, usize)> = Vec::new();
            for p in &places[..=last] {
                let name = match p {
                    None => "-".to_string(),
                    Some(Play::Chip(c)) => crate::names::chip(content, *c).to_string(),
                    Some(Play::Pattern(p)) => {
                        let chips: Vec<&str> = p.chips.iter().map(|&c| crate::names::chip(content, c)).collect();
                        format!("[{}: {}]", place(p.dx, p.dy), chips.join(", "))
                    }
                };
                // (Only neighbors are counted together: the places' order shows.)
                match seen.last_mut() {
                    Some((n, times)) if *n == name && name != "-" => *times += 1,
                    _ => seen.push((name, 1)),
                }
            }
            let items: Vec<String> = seen.into_iter().map(|(name, n)| if n > 1 { format!("{name} x{n}") } else { name }).collect();
            lists.push(format!("{} {}", list.name.replace('_', " "), items.join(", ")));
        }
        if lists.is_empty() { "nothing learned (its buster alone)".into() } else { lists.join("; ") }
    }
}

/// A pattern's place across the field, in words: "2 columns short of its
/// target" (`dx` is toward the computer navi's enemies).
pub fn across(dx: i8) -> String {
    let columns = |n: u8| if n == 1 { "1 column".to_string() } else { format!("{n} columns") };
    match dx {
        0 => "its target's column".to_string(),
        d if d < 0 => format!("{} short of its target", columns(d.unsigned_abs())),
        d => format!("{} past its target", columns(d.unsigned_abs())),
    }
}

/// A pattern's place up and down the field, in words: "1 row up".
pub fn down(dy: i8) -> String {
    let rows = |n: u8| if n == 1 { "1 row".to_string() } else { format!("{n} rows") };
    match dy {
        0 => "its target's row".to_string(),
        d if d < 0 => format!("{} up", rows(d.unsigned_abs())),
        d => format!("{} down", rows(d.unsigned_abs())),
    }
}

/// A pattern's place in words: "2 columns short of its target, 1 row up".
pub fn place(dx: i8, dy: i8) -> String {
    format!("{}, {}", across(dx), down(dy))
}

/// The computer-navi data of the EXE5 save in `file` (a .sav's bytes, or a
/// raw save image), for a side of a match of `game`: what the game has
/// learned of the save's player (`crate::import_exe5`), and what is worth
/// saying about it; or why the file gives none.
pub fn of_save(content: &Content, game: &str, file: &[u8]) -> Result<(ComputerNavi, Vec<String>), String> {
    if !has(content) {
        return Err(format!("{game} has no computer navis"));
    }
    match crate::save_game(file)? {
        exe5_compat::ROOT => {
            let save = crate::import_exe5::read(file)?;
            Ok(crate::import_exe5::computer_navi(content, game, &save.computer_navi()))
        }
        other => Err(format!("a save of {other}, which keeps no computer-navi data")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{exe5_content, exe6_content};
    use nettai_battle::Rng;

    fn chip(content: &Content, name: &str) -> ChipHandle {
        ids::chip(content, "exe5", name).unwrap_or_else(|| panic!("exe5 has no {name}"))
    }

    /// The data with these entries in these places (from 0).
    fn data(entries: &[(usize, Play)]) -> ComputerNavi {
        let mut out = ComputerNavi::default();
        for (place, play) in entries {
            out.places[*place] = Some(play.clone());
        }
        out
    }

    /// The lists are the block's 42 places, in order, none shared.
    #[test]
    fn the_lists_are_the_blocks_places() {
        let mut next = 0;
        for list in &LISTS {
            assert_eq!(list.start, next, "{}", list.name);
            next += list.len;
        }
        assert_eq!(next, PLACES);
        assert_eq!((STANDARD.name, MEGA.name, GIGA.name, PROGRAM_ADVANCE.name), ("standard", "mega", "giga", "program_advance"));
        assert_eq!((list_of(0).name, list_of(3).name, list_of(32).name, list_of(40).name, list_of(41).name), ("first", "standard", "giga", "patterns", "program_advance"));
        assert_eq!(STANDARD_TIMES.iter().sum::<usize>(), STANDARD.len);
    }

    /// The block a save holds of the data: each place its entry, a pattern
    /// named twice one record; and the block read back is the data.
    #[test]
    fn the_data_is_the_blocks_places() {
        let content = exe5_content();
        let (cannon, sword) = (chip(&content, "cannon"), chip(&content, "sword"));
        let pattern = Pattern { dx: -2, dy: 1, chips: vec![sword, cannon], score: Some(7) };
        let d = data(&[
            (1, Play::Chip(cannon)),
            (3, Play::Chip(sword)),
            (5, Play::Chip(sword)),
            (33, Play::Pattern(pattern.clone())),
            (34, Play::Pattern(pattern.clone())),
            (41, Play::Chip(cannon)),
        ]);
        let block = d.tactics();
        assert_eq!(block.entries.len(), PLACES);
        let filled: Vec<(usize, Tactic)> = block.entries.iter().copied().enumerate().filter(|(_, e)| *e != Tactic::Empty).collect();
        assert_eq!(
            filled,
            [(1, Tactic::Chip(cannon)), (3, Tactic::Chip(sword)), (5, Tactic::Chip(sword)), (33, Tactic::Pattern(0)), (34, Tactic::Pattern(0)), (41, Tactic::Chip(cannon))]
        );
        assert_eq!(block.patterns, [TacticPattern { dx: -2, dy: 1, chips: vec![sword, cannon], score: 7 }]);
        assert_eq!(ComputerNavi::of_block(&block), (d.clone(), Vec::new()));
        assert_eq!(d.check(&content, "exe5"), Vec::<String>::new());
        assert_eq!(d.plays().count(), 6);
        assert_eq!(d.list(&STANDARD)[..3], [Some(Play::Chip(sword)), None, Some(Play::Chip(sword))]);
        // The places holding 0, and a pattern the block hasn't, are said.
        let odd = Tactics {
            entries: vec![Tactic::Nothing, Tactic::Chip(sword), Tactic::Empty, Tactic::Pattern(4), Tactic::Nothing],
            patterns: Vec::new(),
        };
        let (read, notes) = ComputerNavi::of_block(&odd);
        assert_eq!(read, data(&[(1, Play::Chip(sword))]));
        assert!(notes[0].contains("place 4") && notes[0].contains("pattern 5") && notes[1].starts_with("2 of the"), "{notes:?}");
        assert_eq!(ComputerNavi::default().tactics().sent(&mut Rng::new(3)), Tactics::default());
    }

    /// An entry of a sent list by what it is: a pattern by its record, not
    /// its number.
    #[derive(Clone, Debug, PartialEq)]
    enum Sent {
        Chip(ChipHandle),
        Pattern(TacticPattern),
        Nothing,
    }

    fn sent(block: &Tactics, rng: &mut Rng) -> Vec<Sent> {
        let s = block.sent(rng);
        s.entries
            .iter()
            .map(|e| match *e {
                Tactic::Chip(c) => Sent::Chip(c),
                Tactic::Pattern(n) => Sent::Pattern(s.patterns[n as usize].clone()),
                Tactic::Nothing => Sent::Nothing,
                Tactic::Empty => panic!("an empty place in a sent list"),
            })
            .collect()
    }

    /// A block and the data a match states of it send the same list, entry
    /// for entry (a pattern by what it is), and draw the same numbers, for
    /// every seed: blocks as a battle's end writes them, sparse ones, ones
    /// with empty places between entries, with a chip where the game writes
    /// patterns, with patterns numbered out of their order and a record no
    /// entry names, a full one and an empty one. So the match's form loses
    /// nothing a battle's start reads.
    #[test]
    fn the_data_sends_what_its_block_sends() {
        let content = exe5_content();
        let chips: Vec<ChipHandle> =
            (0..content.defs.chips.len() as u16).map(ChipHandle).filter(|&c| !content.chip(c).codes.is_empty()).take(48).collect();
        assert_eq!(chips.len(), 48);
        let pattern = |n: usize| TacticPattern { dx: -(n as i8 % 5) - 1, dy: n as i8 % 3 - 1, chips: chips[n..n + 1 + n % 5].to_vec(), score: 1 + n as u32 };
        let block = |entries: &[(usize, Tactic)], patterns: Vec<TacticPattern>| {
            let mut places = vec![Tactic::Empty; PLACES];
            for &(place, e) in entries {
                places[place] = e;
            }
            Tactics { entries: places, patterns }
        };
        let at = |places: std::ops::Range<usize>| -> Vec<(usize, Tactic)> { places.map(|i| (i, Tactic::Chip(chips[i]))).collect() };
        // As a battle's end writes one: the standard chips (the most used
        // four times), megas, a giga, four patterns, a program advance.
        let mut written = at(3..22);
        for i in 3..7 {
            written[i - 3] = (i, Tactic::Chip(chips[0]));
        }
        written.extend(at(27..30));
        written.push((32, Tactic::Chip(chips[40])));
        written.extend((0..4).map(|n| (33 + n, Tactic::Pattern(n as u8))));
        written.push((41, Tactic::Chip(chips[41])));
        let blocks = [
            ("written", block(&written, (0..4).map(pattern).collect())),
            // (The fifth record is one no entry names.)
            ("written, a record more", block(&written, (0..5).map(pattern).collect())),
            ("a mega and a program advance", block(&[(27, Tactic::Chip(chips[1])), (41, Tactic::Chip(chips[2]))], Vec::new())),
            ("the first three", block(&at(0..3), Vec::new())),
            ("the second of the first three", block(&[(1, Tactic::Chip(chips[1])), (20, Tactic::Chip(chips[2]))], Vec::new())),
            (
                "empty places between entries, a chip among the patterns' places",
                block(&[(3, Tactic::Chip(chips[1])), (9, Tactic::Chip(chips[2])), (10, Tactic::Chip(chips[1])), (26, Tactic::Chip(chips[3])), (40, Tactic::Chip(chips[4]))], Vec::new()),
            ),
            (
                "patterns out of their order, in the first places, one twice",
                block(&[(0, Tactic::Pattern(3)), (2, Tactic::Pattern(1)), (12, Tactic::Pattern(3)), (13, Tactic::Chip(chips[5])), (33, Tactic::Pattern(0))], (0..4).map(pattern).collect()),
            ),
            ("full", block(&at(0..PLACES), Vec::new())),
            ("empty", block(&[], Vec::new())),
        ];
        for (name, original) in &blocks {
            let (stated, notes) = ComputerNavi::of_block(original);
            assert_eq!(notes, Vec::<String>::new(), "{name}");
            assert_eq!(stated.check(&content, "exe5"), Vec::<String>::new(), "{name}");
            let compiled = stated.tactics();
            // The places are the block's own; only the patterns' records
            // may be numbered otherwise.
            assert_eq!(compiled.entries.len(), original.entries.len(), "{name}");
            for seed in 0..500u32 {
                let (mut a, mut b) = (Rng::new(seed.wrapping_mul(0x9E37_79B9) ^ 0x1234_5678), Rng::new(seed.wrapping_mul(0x9E37_79B9) ^ 0x1234_5678));
                assert_eq!(sent(original, &mut a), sent(&compiled, &mut b), "{name}, seed {seed}");
                assert_eq!(a, b, "{name}, seed {seed}");
            }
            // And the data read from what it compiles to is itself.
            assert_eq!(ComputerNavi::of_block(&compiled), (stated, Vec::new()), "{name}");
        }
        // A block the game wrote compiles to itself, records and all.
        assert_eq!(ComputerNavi::of_block(&blocks[0].1).0.tactics(), blocks[0].1);
    }

    /// Why a match states the places: the send's swaps are no even shuffle.
    /// The entry in place 4 leads what the other 39 places send far more
    /// often than one in 39 (a place is in none of the 39 swaps about one
    /// time in eight), and where the empty places are changes what a seed
    /// sends.
    #[test]
    fn the_places_show_in_what_is_sent() {
        let content = exe5_content();
        let chips: Vec<ChipHandle> =
            (0..content.defs.chips.len() as u16).map(ChipHandle).filter(|&c| !content.chip(c).codes.is_empty()).take(39).collect();
        let full = Tactics { entries: [vec![Tactic::Empty; 3], chips.iter().map(|&c| Tactic::Chip(c)).collect()].concat(), patterns: Vec::new() };
        let seeds = 4000u32;
        let leads = (0..seeds).filter(|&s| full.sent(&mut Rng::new(s.wrapping_mul(0x9E37_79B9) ^ 0xA5A5_5A5A)).entries[0] == Tactic::Chip(chips[0])).count();
        // (An even shuffle: about 100 of 4,000. The swaps: about 600.)
        assert!(leads > 400, "{leads} of {seeds}");
        // Two chips in places 4 and 5, or in places 28 and 42: the same
        // entries in the same order, sent differently by some seeds.
        let two = |a: usize, b: usize| {
            let mut entries = vec![Tactic::Empty; PLACES];
            (entries[a], entries[b]) = (Tactic::Chip(chips[0]), Tactic::Chip(chips[1]));
            Tactics { entries, patterns: Vec::new() }
        };
        let differ = (0..200u32).filter(|&s| two(3, 4).sent(&mut Rng::new(s)).entries != two(27, 41).sent(&mut Rng::new(s)).entries).count();
        assert!(differ > 20, "{differ} of 200 seeds");
    }

    /// What the game can't hold is said.
    #[test]
    fn what_the_game_cant_hold_is_refused() {
        let content = exe5_content();
        let cannon = chip(&content, "cannon");
        let has = |d: &ComputerNavi, said: &str| {
            let problems = d.check(&content, "exe5");
            assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        };
        let pattern = |dx: i8, dy: i8, n: usize, score: Option<u32>| Play::Pattern(Pattern { dx, dy, chips: vec![cannon; n], score });
        has(&data(&[(33, pattern(0, 0, 0, None))]), "pattern 1 has no chips");
        has(&data(&[(33, pattern(0, 0, 6, Some(1)))]), "pattern 1 has 6 chips; a pattern holds 5");
        has(&data(&[(33, pattern(6, 0, 1, None))]), "6 columns and 0 rows from its target");
        has(&data(&[(33, pattern(0, -3, 1, None))]), "0 columns and -3 rows from its target");
        // A pattern of five chips states its score; a shorter one needn't.
        has(&data(&[(33, pattern(-1, 0, 5, None))]), "pattern 1 has 5 chips and no score");
        assert!(data(&[(33, pattern(-1, 0, 5, Some(10))), (34, pattern(-1, 0, 4, None))]).check(&content, "exe5").is_empty());
        let nine: Vec<(usize, Play)> = (0..9).map(|i| (i as usize, pattern(i % 5, 0, 1 + (i as usize / 5), None))).collect();
        has(&data(&nine), "9 different patterns; it holds 8");
        // The same pattern in nine places is one record; with another
        // score it is another pattern.
        let same: Vec<(usize, Play)> = (0..9).map(|i| (i, pattern(-1, 0, 2, None))).collect();
        assert!(data(&same).check(&content, "exe5").is_empty());
        assert_eq!(data(&[(0, pattern(-1, 0, 2, Some(1))), (1, pattern(-1, 0, 2, Some(2)))]).patterns().len(), 2);
        has(&data(&[(0, Play::Chip(ChipHandle(u16::MAX)))]), "names a chip exe5 hasn't");
        // Every place filled, any class anywhere: the block holds it.
        let full: Vec<(usize, Play)> = (0..PLACES).map(|i| (i, Play::Chip(cannon))).collect();
        assert!(data(&full).check(&content, "exe5").is_empty());
        // EXE6 has no computer navis.
        let six = exe6_content();
        let cannon6 = ids::chip(&six, "exe6", "cannon").unwrap();
        let problems = data(&[(0, Play::Chip(cannon6))]).check(&six, "exe6");
        assert_eq!(problems, ["computer-navi data, but exe6 has no computer navis (no computer-navi system)"]);
        assert!(ComputerNavi::default().check(&six, "exe6").is_empty());
        assert!(super::has(&content) && !super::has(&six));
    }

    /// The game's own write of the block from its counts: sixteen standard
    /// chips by use (4, 4, 2, 2, then once each) from place 4, five megas
    /// from place 28, a giga in place 33 and a program advance in place 42;
    /// equal counts by the higher chip number.
    #[test]
    fn the_data_is_learned_as_the_game_writes_it() {
        let content = exe5_content();
        let of = |class: ChipClass| -> Vec<ChipHandle> {
            (0..content.defs.chips.len() as u16)
                .map(ChipHandle)
                .filter(|&c| content.chip(c).class == class && ids::in_game(&content, "exe5", &content.defs.chip(c).key))
                .collect()
        };
        let (standard, mega, giga, advances) = (of(ChipClass::Standard), of(ChipClass::Mega), of(ChipClass::Giga), of(ChipClass::ProgramAdvance));
        assert!(standard.len() >= 18 && mega.len() >= 6 && giga.len() >= 2 && advances.len() >= 2);
        // Eighteen standard chips used 18, 17, ... times, six megas, two
        // gigas, two program advances.
        let counted = |chips: &[ChipHandle]| -> Vec<(ChipHandle, u32)> { chips.iter().enumerate().map(|(i, &c)| (c, (chips.len() - i) as u32)).collect() };
        let uses = [counted(&standard[..18]), counted(&mega[..6]), counted(&giga[..2]), counted(&advances[..2])].concat();
        let d = ComputerNavi::learned(&content, &uses);
        let mut want = ComputerNavi::default();
        let mut place = STANDARD.start;
        for (i, &c) in standard[..16].iter().enumerate() {
            for _ in 0..STANDARD_TIMES[i] {
                want.places[place] = Some(Play::Chip(c));
                place += 1;
            }
        }
        assert_eq!(place, MEGA.start);
        for (i, &c) in mega[..5].iter().enumerate() {
            want.places[MEGA.start + i] = Some(Play::Chip(c));
        }
        want.places[GIGA.start] = Some(Play::Chip(giga[0]));
        want.places[PROGRAM_ADVANCE.start] = Some(Play::Chip(advances[0]));
        assert_eq!(d, want);
        assert!(d.list(&LISTS[0]).iter().chain(d.list(&LISTS[4])).all(Option::is_none));
        assert_eq!(d.check(&content, "exe5"), Vec::<String>::new());
        // Equal counts: the higher chip number first (Cannon is chip 1,
        // HiCannon 2). Two standard chips alone: places 4 to 11, the mega
        // chips' places empty.
        let (cannon, hicannon) = (chip(&content, "cannon"), chip(&content, "hicannon"));
        let tied = ComputerNavi::learned(&content, &[(cannon, 2), (hicannon, 2)]);
        assert_eq!(tied.places[3..11], [vec![Some(Play::Chip(hicannon)); 4], vec![Some(Play::Chip(cannon)); 4]].concat());
        assert_eq!(tied.plays().count(), 8);
        assert!(ComputerNavi::learned(&content, &[(cannon, 0)]).is_empty());
    }

    /// The terminal's line: the lists with entries by name, an empty place
    /// before an entry shown.
    #[test]
    fn the_data_is_described_by_its_lists() {
        let content = exe5_content();
        let (cannon, sword) = (chip(&content, "cannon"), chip(&content, "sword"));
        let d = data(&[
            (3, Play::Chip(cannon)),
            (4, Play::Chip(cannon)),
            (6, Play::Chip(sword)),
            (33, Play::Pattern(Pattern { dx: -2, dy: 1, chips: vec![sword, cannon], score: None })),
        ]);
        assert_eq!(d.describe(&content), "standard Cannon x2, -, Sword; patterns [2 columns short of its target, 1 row down: Sword, Cannon]");
        assert_eq!(ComputerNavi::default().describe(&content), "nothing learned (its buster alone)");
        assert_eq!(place(0, 0), "its target's column, its target's row");
        assert_eq!(place(1, -2), "1 column past its target, 2 rows up");
    }
}
