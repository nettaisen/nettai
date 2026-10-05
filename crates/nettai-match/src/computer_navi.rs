//! EXE5's computer-navi data, as a match states it for a side: what a
//! computer navi plays from that player's save.
//!
//! **What it is.** An EXE5 save keeps a block of 0xE0 bytes for its player
//! (save +0x554C, the toolkit's +0x78; `nettai_battle::tactics`): 42
//! places, each a chip, a pattern or empty, and eight pattern records, each
//! a place by a target (`dx` columns toward the computer navi's enemies,
//! `dy` rows) and up to five chips used there in a row. A computer navi
//! plays it (content/exe5/rules/computer-navi): the Dark MegaMan a failed
//! Chaos Unison brings plays the data of the player whose Chaos Unison
//! failed, and a navi under DarkInvs its own side's. It plays the entries
//! in order, the played one going last: three times in four the first, and
//! otherwise, or with none, it steps into an enemy's row and fires its
//! buster.
//!
//! **Where it comes from.** The game learns it from its own player
//! alone. A battle counts each chip the player uses (0x0802C1FC, by the
//! chip's class: a standard or mega chip 1 or 3 a use, a giga chip or a
//! program advance 1) and remembers each run of chips used from one column
//! within 30 ticks of each other, two or more, with where its first hits
//! landed from there (0x0802C294, 0x0802C3E2, 0x0802C2DC: a pattern of up
//! to five chips, eight a battle). The battle's end (0x0802C540) scores
//! the patterns (16 kept: one seen again gains 5, the others lose 1) and
//! writes the block from the counts:
//!
//! - places 1 to 3: the three most counted standard chips of a second
//!   count (save +0x2340), which the battle's code never raises (its weight
//!   is always 0, 0x0802C220), so they are empty in every US save seen (one
//!   Japanese save has them filled: nothing here knows what filled them);
//! - places 4 to 27: the sixteen most used standard chips, the two most
//!   used four times each, the next two twice, the rest once;
//! - places 28 to 32: the five most used mega chips; place 33: the most
//!   used giga chip;
//! - places 34 to 41: the patterns, up to eight, the highest scored first;
//! - place 42: the most used program advance.
//!
//! A new save's block is empty (0xFFFF throughout: nothing learned), as are
//! the six blocks after it, which nothing writes.
//!
//! **What a battle makes of it.** As a battle starts each console sends its
//! block shuffled (0x0802C7BE, `Tactics::sent`): the first three places
//! among themselves, the other 39 among themselves, the entries then packed
//! to the front. So all a battle reads of a place is which of the two groups
//! it is in, and a match states just that: what the computer navi plays
//! `first` (up to [`FIRST`] entries) and the `rest` (up to [`REST`]), each
//! entry a chip or a pattern, each group in an order the seed draws. A chip
//! there several times is played that much more often.
//!
//! **What the game can't hold** is refused ([`ComputerNavi::check`]): more
//! entries than a group has places, more than [`PATTERNS`] different
//! patterns, a pattern of more than [`PATTERN_CHIPS`] chips or of none, a
//! pattern's place off the field from any target, a chip the game hasn't,
//! and any of it for a game whose rules have no computer navis (EXE6).

use crate::ids;
use nettai_battle::content::{ChipClass, Content};
use nettai_battle::tactics::{MAX_ENTRIES, MAX_PATTERNS, Tactic, TacticPattern, Tactics};
use nettai_content_api::ChipHandle;

/// The system that drives the computer navis (EXE5's).
pub const SYSTEM: &str = "computer-navi";
/// The places of the block's two groups: those it plays first, and the
/// rest.
pub const FIRST: usize = 3;
pub const REST: usize = MAX_ENTRIES - FIRST;
/// The block's pattern records, and the chips a record has places for (its
/// last four bytes are the pattern's score).
pub const PATTERNS: usize = MAX_PATTERNS;
pub const PATTERN_CHIPS: usize = 5;
/// The furthest a pattern's place can be from a target on a field of six
/// columns and three rows.
pub const MAX_DX: i8 = 5;
pub const MAX_DY: i8 = 2;

/// The chips the game writes into the block from its counts (0x0802C540):
/// how many times each of the sixteen most used standard chips
/// (0x0802C790), how many mega chips, giga chips and program advances.
const STANDARD_TIMES: [usize; 16] = [4, 4, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
const MEGAS: usize = 5;
const GIGAS: usize = 1;
const PROGRAM_ADVANCES: usize = 1;

/// A pattern: where the computer navi stands from its target (`dx` columns
/// toward its enemies, so a negative one is short of the target; `dy` rows,
/// down the screen) and the chips it uses there, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pattern {
    pub dx: i8,
    pub dy: i8,
    pub chips: Vec<ChipHandle>,
}

/// An entry of the data: a chip, or a pattern.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Play {
    Chip(ChipHandle),
    Pattern(Pattern),
}

/// A player's computer-navi data: what a computer navi plays first, and
/// the rest. Empty: a save that has learned nothing (the computer navi
/// only fires its buster).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ComputerNavi {
    pub first: Vec<Play>,
    pub rest: Vec<Play>,
}

/// Whether the game's rules have computer navis (a side takes the data).
pub fn has(content: &Content) -> bool {
    crate::ruleset_has_system(content, SYSTEM)
}

impl ComputerNavi {
    pub fn is_empty(&self) -> bool {
        self.first.is_empty() && self.rest.is_empty()
    }

    /// Every entry, those played first then the rest.
    pub fn plays(&self) -> impl Iterator<Item = &Play> {
        self.first.iter().chain(&self.rest)
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

    /// The block a save holds of it (the engine's, in place order): the
    /// entries played first in the first three places (those left over
    /// empty), the rest after them, each different pattern a record.
    pub fn tactics(&self) -> Tactics {
        let patterns: Vec<TacticPattern> =
            self.patterns().into_iter().map(|p| TacticPattern { dx: p.dx, dy: p.dy, chips: p.chips.clone() }).collect();
        let entry = |p: &Play| match p {
            Play::Chip(c) => Tactic::Chip(*c),
            Play::Pattern(p) => {
                let at = patterns.iter().position(|t| (t.dx, t.dy) == (p.dx, p.dy) && t.chips == p.chips).expect("one of the entries' patterns");
                Tactic::Pattern(at as u8)
            }
        };
        let mut entries: Vec<Tactic> = self.first.iter().map(entry).collect();
        if entries.len() < FIRST {
            entries.resize(FIRST, Tactic::Empty);
        }
        entries.extend(self.rest.iter().map(entry));
        Tactics { entries, patterns }
    }

    /// The data a block in place order holds (a save's): its first three
    /// places and the others, the empty ones left out, each pattern entry
    /// with its record. What it holds that a match doesn't state is said: a
    /// place holding 0 (no chip: no save the game wrote has one) and an
    /// entry for a pattern the block hasn't.
    pub fn of_block(block: &Tactics) -> (ComputerNavi, Vec<String>) {
        let mut notes = Vec::new();
        let mut out = ComputerNavi::default();
        for (i, e) in block.entries.iter().enumerate() {
            let play = match *e {
                Tactic::Empty => continue,
                Tactic::Chip(c) => Play::Chip(c),
                Tactic::Nothing => {
                    notes.push(format!("place {} of the computer-navi data holds no chip (0): left out", i + 1));
                    continue;
                }
                Tactic::Pattern(n) => match block.patterns.get(n as usize) {
                    Some(p) => Play::Pattern(Pattern { dx: p.dx, dy: p.dy, chips: p.chips.clone() }),
                    None => {
                        notes.push(format!("place {} of the computer-navi data names pattern {}, which it hasn't: left out", i + 1, n as u16 + 1));
                        continue;
                    }
                },
            };
            if i < FIRST { out.first.push(play) } else { out.rest.push(play) }
        }
        (out, notes)
    }

    /// The data as the game writes it at a battle's end (0x0802C540) from
    /// how often its player has used each chip (`uses`: a chip and its
    /// count, each chip once): the sixteen most used standard chips (the two
    /// most used four times each, the next two twice, the rest once), the
    /// five most used mega chips, the most used giga chip and the most used
    /// program advance, in the block's order; no patterns, and nothing
    /// played first (the US games' second count stays 0). Chips used
    /// equally often come as the game's sort leaves them (0x0814301C: the
    /// higher chip number first).
    pub fn learned(content: &Content, uses: &[(ChipHandle, u32)]) -> ComputerNavi {
        let number = |c: ChipHandle| exe5_compat::Compat::exe5().chip_entry(ids::local(&content.defs.chip(c).key)).map_or(c.0, |e| e.id);
        let most = |class: ChipClass| -> Vec<ChipHandle> {
            let mut list: Vec<(u32, u16, ChipHandle)> =
                uses.iter().filter(|(c, n)| *n > 0 && content.chip(*c).class == class).map(|&(c, n)| (n, number(c), c)).collect();
            list.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
            list.into_iter().map(|x| x.2).collect()
        };
        let mut rest = Vec::new();
        for (chip, times) in most(ChipClass::Standard).into_iter().zip(STANDARD_TIMES) {
            rest.extend(std::iter::repeat_n(Play::Chip(chip), times));
        }
        rest.extend(most(ChipClass::Mega).into_iter().take(MEGAS).map(Play::Chip));
        rest.extend(most(ChipClass::Giga).into_iter().take(GIGAS).map(Play::Chip));
        rest.extend(most(ChipClass::ProgramAdvance).into_iter().take(PROGRAM_ADVANCES).map(Play::Chip));
        ComputerNavi { first: Vec::new(), rest }
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
        if self.first.len() > FIRST {
            out.push(format!("the computer navi plays {} entries first; its data has {FIRST} places for them", self.first.len()));
        }
        if self.rest.len() > REST {
            out.push(format!("the computer navi's data has {} entries after its first; it has {REST} places for them", self.rest.len()));
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

    /// The data in a line, for the terminal: each group's entries by name,
    /// a chip there several times once with its count.
    pub fn describe(&self, content: &Content) -> String {
        let group = |plays: &[Play]| -> String {
            let mut seen: Vec<(String, usize)> = Vec::new();
            for p in plays {
                let name = match p {
                    Play::Chip(c) => crate::names::chip(content, *c).to_string(),
                    Play::Pattern(p) => {
                        let chips: Vec<&str> = p.chips.iter().map(|&c| crate::names::chip(content, c)).collect();
                        format!("[{}: {}]", place(p.dx, p.dy), chips.join(", "))
                    }
                };
                match seen.iter_mut().find(|(n, _)| *n == name) {
                    Some((_, n)) => *n += 1,
                    None => seen.push((name, 1)),
                }
            }
            let items: Vec<String> = seen.into_iter().map(|(name, n)| if n > 1 { format!("{name} x{n}") } else { name }).collect();
            items.join(", ")
        };
        match (self.first.is_empty(), self.rest.is_empty()) {
            (true, true) => "nothing learned (its buster alone)".into(),
            (true, false) => group(&self.rest),
            (false, true) => format!("first {}", group(&self.first)),
            (false, false) => format!("first {}; then {}", group(&self.first), group(&self.rest)),
        }
    }
}

/// A pattern's place in words: "2 columns short of its target, 1 row up".
pub fn place(dx: i8, dy: i8) -> String {
    let columns = |n: u8| if n == 1 { "1 column".to_string() } else { format!("{n} columns") };
    let rows = |n: u8| if n == 1 { "1 row".to_string() } else { format!("{n} rows") };
    let across = match dx {
        0 => "its target's column".to_string(),
        d if d < 0 => format!("{} short of its target", columns(d.unsigned_abs())),
        d => format!("{} past its target", columns(d.unsigned_abs())),
    };
    let down = match dy {
        0 => "its row".to_string(),
        d if d < 0 => format!("{} up", rows(d.unsigned_abs())),
        d => format!("{} down", rows(d.unsigned_abs())),
    };
    format!("{across}, {down}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{exe5_content, exe6_content};

    fn chip(content: &Content, name: &str) -> ChipHandle {
        ids::chip(content, "exe5", name).unwrap_or_else(|| panic!("exe5 has no {name}"))
    }

    /// The block a save holds of the data: the first group in the first
    /// three places (the rest of them empty), the others after, a pattern
    /// named twice one record; and the block read back is the data.
    #[test]
    fn the_data_is_the_block_s_two_groups() {
        let content = exe5_content();
        let (cannon, sword) = (chip(&content, "cannon"), chip(&content, "sword"));
        let pattern = Pattern { dx: -2, dy: 1, chips: vec![sword, cannon] };
        let data = ComputerNavi {
            first: vec![Play::Chip(cannon)],
            rest: vec![Play::Pattern(pattern.clone()), Play::Chip(sword), Play::Pattern(pattern.clone()), Play::Chip(sword)],
        };
        let block = data.tactics();
        assert_eq!(
            block.entries,
            [Tactic::Chip(cannon), Tactic::Empty, Tactic::Empty, Tactic::Pattern(0), Tactic::Chip(sword), Tactic::Pattern(0), Tactic::Chip(sword)]
        );
        assert_eq!(block.patterns, [TacticPattern { dx: -2, dy: 1, chips: vec![sword, cannon] }]);
        assert_eq!(ComputerNavi::of_block(&block), (data.clone(), Vec::new()));
        assert_eq!(data.check(&content, "exe5"), Vec::<String>::new());
        // What the console sends of it: the entries packed, the first
        // group's still first.
        let sent = block.sent(&mut nettai_battle::Rng::new(3));
        assert_eq!((sent.entries.len(), sent.entries[0]), (5, Tactic::Chip(cannon)));
        // A place holding 0, and a pattern the block hasn't, are said.
        let odd = Tactics { entries: vec![Tactic::Nothing, Tactic::Chip(sword), Tactic::Empty, Tactic::Pattern(4)], patterns: Vec::new() };
        let (read, notes) = ComputerNavi::of_block(&odd);
        assert_eq!(read, ComputerNavi { first: vec![Play::Chip(sword)], rest: Vec::new() });
        assert!(notes[0].contains("place 1") && notes[1].contains("pattern 5"), "{notes:?}");
        assert_eq!(ComputerNavi::default().tactics().sent(&mut nettai_battle::Rng::new(3)), Tactics::default());
    }

    /// What the game can't hold is said.
    #[test]
    fn what_the_game_cant_hold_is_refused() {
        let content = exe5_content();
        let cannon = chip(&content, "cannon");
        let has = |data: &ComputerNavi, said: &str| {
            let problems = data.check(&content, "exe5");
            assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        };
        let chips = |n: usize| vec![Play::Chip(cannon); n];
        has(&ComputerNavi { first: chips(4), rest: Vec::new() }, "plays 4 entries first; its data has 3 places");
        has(&ComputerNavi { first: Vec::new(), rest: chips(40) }, "40 entries after its first; it has 39 places");
        assert!(ComputerNavi { first: chips(3), rest: chips(39) }.check(&content, "exe5").is_empty());
        let pattern = |dx: i8, dy: i8, n: usize| Play::Pattern(Pattern { dx, dy, chips: vec![cannon; n] });
        has(&ComputerNavi { first: Vec::new(), rest: vec![pattern(0, 0, 0)] }, "pattern 1 has no chips");
        has(&ComputerNavi { first: Vec::new(), rest: vec![pattern(0, 0, 6)] }, "pattern 1 has 6 chips; a pattern holds 5");
        has(&ComputerNavi { first: Vec::new(), rest: vec![pattern(6, 0, 1)] }, "6 columns and 0 rows from its target");
        has(&ComputerNavi { first: Vec::new(), rest: vec![pattern(0, -3, 1)] }, "0 columns and -3 rows from its target");
        let nine: Vec<Play> = (0..9).map(|i| pattern(i % 5, 0, 1 + (i as usize / 5))).collect();
        has(&ComputerNavi { first: Vec::new(), rest: nine }, "9 different patterns; it holds 8");
        // The same pattern nine times is one record.
        assert!(ComputerNavi { first: Vec::new(), rest: vec![pattern(-1, 0, 2); 9] }.check(&content, "exe5").is_empty());
        has(&ComputerNavi { first: vec![Play::Chip(ChipHandle(u16::MAX))], rest: Vec::new() }, "names a chip exe5 hasn't");
        // EXE6 has no computer navis.
        let six = exe6_content();
        let cannon6 = ids::chip(&six, "exe6", "cannon").unwrap();
        let problems = ComputerNavi { first: vec![Play::Chip(cannon6)], rest: Vec::new() }.check(&six, "exe6");
        assert_eq!(problems, ["computer-navi data, but exe6 has no computer navis (no computer-navi system)"]);
        assert!(ComputerNavi::default().check(&six, "exe6").is_empty());
        assert!(has_system(&content) && !has_system(&six));
    }

    fn has_system(content: &Content) -> bool {
        super::has(content)
    }

    /// The game's own write of the block from its counts: sixteen standard
    /// chips by use (4, 4, 2, 2, then once each), five megas, a giga and a
    /// program advance; equal counts by the higher chip number.
    #[test]
    fn the_data_is_learned_as_the_game_writes_it() {
        let content = exe5_content();
        let of = |class: ChipClass| -> Vec<ChipHandle> {
            (0..content.defs.chips.len() as u16)
                .map(ChipHandle)
                .filter(|&c| content.chip(c).class == class && ids::in_game(&content, "exe5", &content.defs.chip(c).key))
                .collect()
        };
        let (standard, mega, giga) = (of(ChipClass::Standard), of(ChipClass::Mega), of(ChipClass::Giga));
        assert!(standard.len() >= 18 && mega.len() >= 6 && giga.len() >= 2);
        // Eighteen standard chips used 18, 17, ... times, six megas, two gigas.
        let mut uses: Vec<(ChipHandle, u32)> = standard[..18].iter().enumerate().map(|(i, &c)| (c, 18 - i as u32)).collect();
        uses.extend(mega[..6].iter().enumerate().map(|(i, &c)| (c, 6 - i as u32)));
        uses.extend(giga[..2].iter().enumerate().map(|(i, &c)| (c, 2 - i as u32)));
        let data = ComputerNavi::learned(&content, &uses);
        assert!(data.first.is_empty());
        let mut want: Vec<Play> = Vec::new();
        for (i, &c) in standard[..16].iter().enumerate() {
            want.extend(vec![Play::Chip(c); STANDARD_TIMES[i]]);
        }
        want.extend(mega[..5].iter().map(|&c| Play::Chip(c)));
        want.push(Play::Chip(giga[0]));
        assert_eq!(data.rest, want);
        assert_eq!(data.rest.len(), 24 + 5 + 1);
        assert_eq!(data.check(&content, "exe5"), Vec::<String>::new());
        // Equal counts: the higher chip number first (Cannon is chip 1,
        // HiCannon 2).
        let (cannon, hicannon) = (chip(&content, "cannon"), chip(&content, "hicannon"));
        let tied = ComputerNavi::learned(&content, &[(cannon, 2), (hicannon, 2)]);
        assert_eq!(tied.rest[..8], [vec![Play::Chip(hicannon); 4], vec![Play::Chip(cannon); 4]].concat());
        assert!(ComputerNavi::learned(&content, &[(cannon, 0)]).is_empty());
    }

    #[test]
    fn a_pattern_s_place_is_said_in_words() {
        assert_eq!(place(-2, 1), "2 columns short of its target, 1 row down");
        assert_eq!(place(0, 0), "its target's column, its row");
        assert_eq!(place(1, -2), "1 column past its target, 2 rows up");
    }
}
