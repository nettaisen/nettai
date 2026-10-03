//! Folders against their rules, which are each game's (its ruleset's
//! `folder_check`: BN6's are content/bn6/rules/folder/system.luau): one
//! check that a match file, a netplay offer, the editor and live play's
//! random folder all go through (`Battle::check_folder`). The rules read
//! the side's stats as its round set them up (the NaviCust's and the patch
//! cards' folder limits), so a check runs on that side's battle.
//!
//! What a folder can be made of is what the rules accept a chip of alone
//! (`pool`), and live play's random folder (`random_folder`) is drawn one
//! chip at a time, each kept if the rules still accept the chips so far.

use crate::draw::Draws;
use nettai_battle::Battle;
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::folder::FOLDER_SIZE;
use nettai_battle::custom::{FolderChip, SavedFolder};
use nettai_battle::rules::FolderProblem;
use nettai_content_api::ChipHandle;

/// What side `side`'s rules say of `folder`, each rule it breaks named and
/// said (none: it keeps them).
pub fn problems(b: &mut Battle, side: u8, folder: &SavedFolder) -> Vec<FolderProblem> {
    b.check_folder(side, &folder.chips, folder.regular, folder.tags, true)
}

/// The rule a folder rule's problem names when a chip is none a folder can
/// hold (BN6's own name for it: the rules name their problems).
pub const CHIP_RULE: &str = "chip";

/// The chips side `side`'s rules let a folder hold, in handle order: those
/// they accept alone (in their first code).
pub fn pool(content: &Content, b: &mut Battle, side: u8) -> Vec<ChipHandle> {
    (0..content.defs.chips.len() as u16)
        .map(ChipHandle)
        .filter(|&id| {
            let code = content.chip(id).codes.first().copied().unwrap_or(ChipCode(0));
            !b.check_folder(side, &[FolderChip::new(id, code)], None, None, false).iter().any(|p| p.rule == CHIP_RULE)
        })
        .collect()
}

/// A random folder that side `side`'s rules accept: chips drawn one at a
/// time from the pool, each kept if the rules still accept the chips so far
/// (the copies of a chip, the Mega and Giga limits), in one of its codes.
/// The codes lean to two the folder favors (and `*`), as a player's would,
/// so that a hand often has chips to pick together. Its Regular chip is one
/// the rules accept as Regular, if any is.
pub fn random_folder(content: &Content, b: &mut Battle, side: u8, draws: &mut Draws) -> SavedFolder {
    let pool = pool(content, b, side);
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

/// A folder in a line: each chip's name and code, the Regular chip marked.
pub fn describe(content: &Content, folder: &SavedFolder) -> String {
    folder
        .chips
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mark = if folder.regular == Some(i as u8) { " (Regular)" } else { "" };
            format!("{} {}{mark}", crate::names::chip(content, c.id), c.code.letter())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::bn6_content;

    /// A battle of the live navi on both sides, to ask BN6's rules.
    fn rules(content: &std::sync::Arc<Content>) -> Battle {
        crate::check::start(content, &crate::draw::live(content, 1, None).unwrap()).unwrap()
    }

    /// Random folders keep BN6's folder rules, with a Regular chip that
    /// fits; the pool is the chip pack's folder chips.
    #[test]
    fn random_folders_are_legal() {
        let content = bn6_content();
        let mut b = rules(&content);
        let pool = pool(&content, &mut b, 0);
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
        for seed in 0..20 {
            let f = random_folder(&content, &mut b, 0, &mut Draws::new(seed));
            assert_eq!(problems(&mut b, 0, &f), Vec::new(), "seed {seed}: {}", describe(&content, &f));
            let r = f.regular.expect("a Regular chip");
            assert!(content.chip(f.chips[r as usize].id).mb <= 50);
        }
        // Another seed, another folder; the same seed, the same.
        let mut one = |seed| random_folder(&content, &mut b, 0, &mut Draws::new(seed));
        assert_ne!(one(1).chips, one(2).chips);
        assert_eq!(one(3).chips, one(3).chips);
    }

    /// What BN6's rules refuse.
    #[test]
    fn the_rules_refuse() {
        let content = bn6_content();
        let mut b = rules(&content);
        let chip = |key: &str| {
            let id = content.defs.chip_by_key(key).unwrap();
            FolderChip::new(id, content.chip(id).codes[0])
        };
        let base = |chips: [FolderChip; FOLDER_SIZE]| SavedFolder { chips, regular: None, tags: None };
        let mut said = |f: &SavedFolder| -> Vec<String> { problems(&mut b, 0, f).into_iter().map(|p| format!("{}: {}", p.rule, p.text)).collect() };
        // Five Recov10s (4 MB) and the rest plain chips is legal; a sixth
        // isn't.
        let mut chips = [chip("bn6:recov10"); FOLDER_SIZE];
        let mut plain = ["bn6:cannon", "bn6:airshot", "bn6:vulcan1", "bn6:spreadr1", "bn6:minibomb", "bn6:sword"].iter().cycle();
        for c in chips.iter_mut().skip(5) {
            *c = chip(plain.next().unwrap());
        }
        assert_eq!(said(&base(chips)), Vec::<String>::new());
        let mut six = chips;
        six[5] = chip("bn6:recov10");
        assert!(said(&base(six)).iter().any(|v| v.contains("copies: 6 copies of bn6:recov10")));
        // A code the chip doesn't come in.
        let mut code = chips;
        code[0].code = ChipCode(25);
        assert!(said(&base(code)).iter().any(|v| v.contains("code Z")));
        // Six Mega chips; two Giga chips.
        let mut megas = chips;
        for (i, key) in ["bn6:roll", "bn6:roll2", "bn6:heatman", "bn6:elecman", "bn6:slashman", "bn6:eraseman"].iter().enumerate() {
            megas[10 + i] = chip(key);
        }
        assert!(said(&base(megas)).iter().any(|v| v.contains("6 Mega chips")));
        let mut gigas = chips;
        gigas[10] = chip("bn6:bass");
        gigas[11] = chip("bn6:deltaray");
        assert!(said(&base(gigas)).iter().any(|v| v.contains("2 Giga chips")));
        // A Regular chip past the Regular memory (Roll3 is 60 MB).
        let mut regular = base(chips);
        regular.chips[10] = chip("bn6:roll3");
        regular.regular = Some(10);
        assert!(said(&regular).iter().any(|v| v.contains("the Regular chip bn6:roll3")));
        // Tag chips: two entries of 60 MB together at most, not the Regular.
        let mut tags = base(chips);
        tags.tags = Some((5, 6));
        assert_eq!(said(&tags), Vec::<String>::new());
        tags.chips[6] = chip("bn6:roll3");
        assert!(said(&tags).iter().any(|v| v.contains("tags: the tag chips")), "{:?}", said(&tags));
        // A chip the pack doesn't list.
        let mut dark = chips;
        dark[0] = chip("bn6:drksword");
        assert!(said(&base(dark)).iter().any(|v| v.contains("chip: entry 0: bn6:drksword is no chip a folder can hold")));
    }
}
