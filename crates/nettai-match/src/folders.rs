//! Folders against their rules, which are each game's (its ruleset's
//! `folder_check`: EXE6's are content/exe6/rules/folder/init.luau): one
//! check that a match file, a netplay offer, the editor and live play's
//! random folder all go through (`Battle::check_folder`). The rules read
//! the side's stats as its round set them up (the NaviCust's and the patch
//! cards' folder limits), so a check runs on that side's battle.
//!
//! What a folder can be made of is what the rules accept a chip of alone
//! (`pool`), and live play's random folder (`random_folder`) is drawn one
//! chip at a time, each kept if the rules still accept the chips so far.
//!
//! A match keeps a folder as it is being made ([`Folder`]): entries may be
//! empty (a new match's all are). The rules see the chips there, and say
//! that a folder of fewer than 30 is no folder (EXE6's `size`); a round is
//! played only with a whole one ([`Folder::saved`]).

use crate::draw::Draws;
use nettai_battle::Battle;
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::folder::FOLDER_SIZE;
use nettai_battle::custom::{FolderChip, SavedFolder};
use nettai_battle::rules::FolderProblem;
use nettai_content_api::ChipHandle;

/// A side's folder as a match keeps it: its 30 entries, each a chip in one
/// of its codes or empty (a folder being made), and its Regular and tag
/// chips (entries).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Folder {
    pub chips: [Option<FolderChip>; FOLDER_SIZE],
    pub regular: Option<u8>,
    pub tags: Option<(u8, u8)>,
}

impl Folder {
    /// No chips: a new match's folder.
    pub const EMPTY: Folder = Folder { chips: [None; FOLDER_SIZE], regular: None, tags: None };

    /// The folder a save holds, when every entry has a chip.
    pub fn saved(&self) -> Option<SavedFolder> {
        let chips: Vec<FolderChip> = self.chips.iter().copied().collect::<Option<_>>()?;
        Some(SavedFolder { chips: chips.try_into().ok()?, regular: self.regular, tags: self.tags })
    }

    /// The chips there, in order.
    pub fn chips(&self) -> impl Iterator<Item = FolderChip> + '_ {
        self.chips.iter().flatten().copied()
    }

    /// Whether entry `i` has a chip.
    pub fn has(&self, i: u8) -> bool {
        self.chips.get(i as usize).is_some_and(|c| c.is_some())
    }

    /// The folder with each empty entry holding `filler`: a whole folder
    /// for a round its checks set up (the folder's own problems are its
    /// own entries').
    pub fn filled_with(&self, filler: FolderChip) -> SavedFolder {
        SavedFolder { chips: self.chips.map(|c| c.unwrap_or(filler)), regular: self.regular, tags: self.tags }
    }
}

impl From<SavedFolder> for Folder {
    fn from(f: SavedFolder) -> Folder {
        Folder { chips: f.chips.map(Some), regular: f.regular, tags: f.tags }
    }
}

/// What side `side`'s rules say of `folder`, each rule it breaks named and
/// said (none: it keeps them). The rules see the chips there, in order, and
/// the Regular and tag chips by their place among them (one on an empty
/// entry is left out: the match's own checks say so).
pub fn problems(b: &mut Battle, side: u8, folder: &Folder) -> Vec<FolderProblem> {
    let chips: Vec<FolderChip> = folder.chips().collect();
    let place = |i: u8| folder.has(i).then(|| folder.chips[..i as usize].iter().flatten().count() as u8);
    let regular = folder.regular.and_then(place);
    let tags = folder.tags.and_then(|(a, b)| Some((place(a)?, place(b)?)));
    b.check_folder(side, &chips, regular, tags, true)
}

/// The rule a folder rule's problem names when a chip is none a folder can
/// hold (EXE6's own name for it: the rules name their problems).
pub const CHIP_RULE: &str = "chip";

/// The chips of `game` side `side`'s rules let a folder hold, in handle
/// order: those they accept alone (in their first code).
pub fn pool(content: &Content, game: &str, b: &mut Battle, side: u8) -> Vec<ChipHandle> {
    (0..content.defs.chips.len() as u16)
        .map(ChipHandle)
        .filter(|&id| crate::ids::in_game(content, game, &content.defs.chip(id).key))
        .filter(|&id| {
            let code = content.chip(id).codes.first().copied().unwrap_or(ChipCode(0));
            !b.check_folder(side, &[FolderChip::new(id, code)], None, None, false).iter().any(|p| p.rule == CHIP_RULE)
        })
        .collect()
}

/// A random folder of `game`'s chips that side `side`'s rules accept: chips
/// drawn one at a time from the pool, each kept if the rules still accept
/// the chips so far
/// (the copies of a chip, the Mega and Giga limits), in one of its codes.
/// The codes lean to two the folder favors (and `*`), as a player's would,
/// so that a hand often has chips to pick together. Its Regular chip is one
/// the rules accept as Regular, if any is.
pub fn random_folder(content: &Content, game: &str, b: &mut Battle, side: u8, draws: &mut Draws) -> SavedFolder {
    let pool = pool(content, game, b, side);
    assert!(!pool.is_empty(), "the rules let a folder hold no chip");
    let favored = [ChipCode(draws.below(26) as u8), ChipCode(draws.below(26) as u8)];
    let mut chips: Vec<FolderChip> = Vec::with_capacity(FOLDER_SIZE);
    let mut tries = 0;
    while chips.len() < FOLDER_SIZE {
        tries += 1;
        assert!(tries < 100_000, "no folder the rules accept in the content's chips");
        let id = pool[draws.below(pool.len())];
        let d = content.chip(id);
        chips.push(FolderChip::new(id, d.codes[0]));
        let kept = b.check_folder(side, &chips, None, None, false).is_empty();
        chips.pop();
        if !kept {
            continue;
        }
        let liked: Vec<ChipCode> =
            d.codes.iter().copied().filter(|c| *c == ChipCode::ASTERISK || favored.contains(c)).collect();
        let codes = if liked.is_empty() { &d.codes } else { &liked };
        let code = codes[draws.below(codes.len())];
        chips.push(FolderChip::new(id, code));
    }
    let fits: Vec<u8> =
        (0..FOLDER_SIZE as u8).filter(|&i| b.check_folder(side, &chips, Some(i), None, true).is_empty()).collect();
    let regular = (!fits.is_empty()).then(|| fits[draws.below(fits.len())]);
    SavedFolder { chips: chips.try_into().expect("30 chips"), regular, tags: None }
}

/// A folder in a line: each chip's name and code, the Regular chip marked,
/// an empty entry a dash.
pub fn describe(content: &Content, folder: &Folder) -> String {
    folder
        .chips
        .iter()
        .enumerate()
        .map(|(i, c)| match c {
            Some(c) => {
                let mark = if folder.regular == Some(i as u8) { " (Regular)" } else { "" };
                format!("{} {}{mark}", crate::names::chip(content, c.id), c.code.letter())
            }
            None => "-".into(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::exe6_content;

    /// A battle of the live navi on both sides, to ask EXE6's rules.
    fn rules(content: &std::sync::Arc<Content>) -> Battle {
        crate::check::start(content, &crate::draw::live(content, "exe6", 1, None).unwrap()).unwrap()
    }

    /// Random folders keep EXE6's folder rules, with a Regular chip that
    /// fits; the pool is the chip pack's folder chips.
    #[test]
    fn random_folders_are_legal() {
        let content = exe6_content();
        let mut b = rules(&content);
        let pool = pool(&content, "exe6", &mut b, 0);
        // The pack's folder chips: no Program Advance, no dark chip, none
        // past the pack's (the BeastOut chip). The JP-content chips are
        // folder chips in the Japanese games' records, which the content
        // has (GunDelEX and Django's aren't in the US games').
        let keys: Vec<&str> = pool.iter().map(|&c| nettai_content_api::keys::local(&content.defs.chip(c).key)).collect();
        assert!(keys.len() > 250, "{} chips", keys.len());
        for key in ["cannon", "roll", "bass", "gundels1", "gundelex", "areagrab", "otenko", "gregar", "falzar", "count", "django"] {
            assert!(keys.contains(&key), "{key}");
        }
        for key in ["drksword", "beastout", "gigacan1", "batcan1"] {
            assert!(!keys.contains(&key), "{key}");
        }
        // (The Regular chip: one within the fresh navi's Regular memory,
        // when the folder drew one.)
        let memory = b.stats[0].reg_up;
        let mut regulars = 0;
        for seed in 0..20 {
            let f = random_folder(&content, "exe6", &mut b, 0, &mut Draws::new(seed));
            assert_eq!(problems(&mut b, 0, &f.into()), Vec::new(), "seed {seed}: {}", describe(&content, &f.into()));
            if let Some(r) = f.regular {
                assert!(content.chip(f.chips[r as usize].id).mb <= memory);
                regulars += 1;
            }
        }
        assert!(regulars > 0, "no folder of twenty has a chip within the Regular memory ({memory} MB)");
        // Another seed, another folder; the same seed, the same.
        let mut one = |seed| random_folder(&content, "exe6", &mut b, 0, &mut Draws::new(seed));
        assert_ne!(one(1).chips, one(2).chips);
        assert_eq!(one(3).chips, one(3).chips);
    }

    /// What EXE6's rules refuse.
    #[test]
    fn the_rules_refuse() {
        let content = exe6_content();
        let mut b = rules(&content);
        let chip = |key: &str| {
            let id = content.defs.chip_by_key(key).unwrap();
            FolderChip::new(id, content.chip(id).codes[0])
        };
        let base = |chips: [FolderChip; FOLDER_SIZE]| SavedFolder { chips, regular: None, tags: None };
        let mut said = |f: &SavedFolder| -> Vec<String> { problems(&mut b, 0, &(*f).into()).into_iter().map(|p| format!("{}: {}", p.rule, p.text)).collect() };
        // Five Recov10s (4 MB) and the rest plain chips is legal; a sixth
        // isn't.
        let mut chips = [chip("recov10"); FOLDER_SIZE];
        let mut plain = ["cannon", "airshot", "vulcan1", "spreadr1", "minibomb", "sword"].iter().cycle();
        for c in chips.iter_mut().skip(5) {
            *c = chip(plain.next().unwrap());
        }
        assert_eq!(said(&base(chips)), Vec::<String>::new());
        let mut six = chips;
        six[5] = chip("recov10");
        assert!(said(&base(six)).iter().any(|v| v.contains("copies: 6 copies of recov10")));
        // A code the chip doesn't come in.
        let mut code = chips;
        code[0].code = ChipCode(25);
        assert!(said(&base(code)).iter().any(|v| v.contains("code Z")));
        // Six Mega chips; two Giga chips.
        let mut megas = chips;
        for (i, key) in ["roll", "roll2", "heatman", "elecman", "slashman", "eraseman"].iter().enumerate() {
            megas[10 + i] = chip(key);
        }
        assert!(said(&base(megas)).iter().any(|v| v.contains("6 Mega chips")));
        let mut gigas = chips;
        gigas[10] = chip("bass");
        gigas[11] = chip("deltaray");
        assert!(said(&base(gigas)).iter().any(|v| v.contains("2 Giga chips")));
        // A Regular chip past the Regular memory (Roll3 is 60 MB).
        let mut regular = base(chips);
        regular.chips[10] = chip("roll3");
        regular.regular = Some(10);
        assert!(said(&regular).iter().any(|v| v.contains("the Regular chip roll3")));
        // Tag chips: two entries of 60 MB together at most, not the Regular.
        let mut tags = base(chips);
        tags.tags = Some((5, 6));
        assert_eq!(said(&tags), Vec::<String>::new());
        tags.chips[6] = chip("roll3");
        assert!(said(&tags).iter().any(|v| v.contains("tags: the tag chips")), "{:?}", said(&tags));
        // A chip the pack doesn't list.
        let mut dark = chips;
        dark[0] = chip("drksword");
        assert!(said(&base(dark)).iter().any(|v| v.contains("chip: entry 0: drksword is no chip a folder can hold")));
    }
}
