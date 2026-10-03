//! BN6's folder rules, as its folder editor enforces them, and a random
//! folder that keeps them (live play's: docs/frontend.md §2). A match file's
//! folders are checked by them (`crate::check`).
//!
//! The editor's check on putting a chip in (`sub_8135080`, with
//! `sub_8135500`), on 30 chips:
//!
//! - **Copies of one chip** (in any codes), by its MB: up to 19 MB five,
//!   20-29 four, 30-39 three, 40-49 two, 50 and up one (`sub_8135500`).
//! - **Mega chips**: at most the navi's Mega level (NaviStats+0x0B); **Giga
//!   chips**: its Giga level (+0x0C). A link battle checks the folder for
//!   both again before it starts (`sub_8120D10`, counting with
//!   `sub_8120AE8`).
//! - **Chips with the dark flag**: at most three (no BN6 chip has it).
//!
//! Besides: each chip's code is one of its codes (`sub_8006EE8`; a battle
//! counts any other as the invalid chip), and the Regular chip's MB is at
//! most the navi's Regular memory (NaviStats+0x09: the editor drops a
//! Regular chip past it, `sub_81352A0` and `sub_813CEA0`). The tag chips
//! are two other entries whose MB together are at most 60 (`sub_81349E8`:
//! a chip can be tagged while the tag chips' MB leave room for it).
//!
//! What can be in a folder is what the chip pack lists (`sub_811FE7C`):
//! chips 1 to 0x13A, without the extra flag 0x20 (the dark chips); of them
//! the folder chips, Standard, Mega and Giga. The JP-content chips are among
//! them: content/bn6 has the Japanese games' records and routines for
//! GunDelEX, Otenko, Count's, Django's, Gregar and Falzar (GunDelEX and
//! Django's are folder chips only in the Japanese records; a US console has
//! no routine for Count's, Django's, Gregar or Falzar). Another game's
//! chips (a root that isn't BN6's) can be in a folder if they are Standard,
//! Mega or Giga chips with a code, as no chip pack of theirs is known yet.

use bn6_compat::Compat;
use nettai_battle::content::{ChipClass, ChipCode, ChipFlags, Content};
use nettai_battle::custom::folder::FOLDER_SIZE;
use nettai_battle::custom::{FolderChip, SavedFolder};
use nettai_battle::setup::NaviStats;
use nettai_content_api::ChipHandle;
use crate::draw::Draws;

/// The last chip the pack lists (`sub_811FE7C`).
const LAST_PACK_CHIP: u16 = 0x13A;
/// The extra flag of the chips the pack doesn't list (the dark chips).
const NOT_IN_PACK: u8 = 0x20;
/// Chips with the dark flag a folder can hold (`sub_8135080`).
const DARK_CHIPS: usize = 3;
/// The tag chips' MB together at most (`sub_81349E8`).
pub const TAG_MB: u32 = 60;

/// What a navi's stats allow its folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderLimits {
    /// Mega and Giga chips (NaviStats+0x0B, +0x0C).
    pub mega: u8,
    pub giga: u8,
    /// The Regular chip's MB at most (NaviStats+0x09).
    pub regular_mb: u8,
}

impl FolderLimits {
    pub fn of(stats: &NaviStats) -> FolderLimits {
        FolderLimits { mega: stats.mega_level, giga: stats.giga_level, regular_mb: stats.reg_up }
    }
}

/// `sub_8135500`: how many copies of a chip of `mb` MB a folder can hold.
pub fn copies_allowed(mb: u8) -> usize {
    match mb {
        0..=19 => 5,
        20..=29 => 4,
        30..=39 => 3,
        40..=49 => 2,
        _ => 1,
    }
}

/// The chips a folder can be made of: the pack's folder chips (above),
/// with at least one code, in handle order.
pub fn folder_chips(content: &Content) -> Vec<ChipHandle> {
    let compat = Compat::bn6_for(content);
    content
        .defs
        .chips
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            let r = &d.record;
            // A BN6 chip is listed by its number; another game's by its
            // class alone.
            let listed = match compat.compat_key(content, &d.key) {
                Some(k) => compat.chips.get(k).is_some_and(|c| (1..=LAST_PACK_CHIP).contains(&c.id)),
                None => nettai_content_api::keys::root_of(&d.key) != Some("bn6"),
            };
            matches!(r.class, ChipClass::Standard | ChipClass::Mega | ChipClass::Giga)
                && !r.codes.is_empty()
                && listed
                && r.extra_flags.0 & NOT_IN_PACK == 0
        })
        .map(|(i, _)| ChipHandle(i as u16))
        .collect()
}

/// What breaks the rules in `folder` (empty: it is legal), each said.
pub fn violations(content: &Content, folder: &SavedFolder, limits: FolderLimits) -> Vec<String> {
    let pool = folder_chips(content);
    let mut out = Vec::new();
    let name = |c: ChipHandle| crate::names::chip(content, c).to_string();
    let mut copies: std::collections::BTreeMap<ChipHandle, usize> = Default::default();
    let (mut mega, mut giga, mut dark) = (0, 0, 0);
    for (i, c) in folder.chips.iter().enumerate() {
        let d = content.chip(c.id);
        if !pool.contains(&c.id) {
            out.push(format!("entry {i}: {} is no chip a folder can hold", name(c.id)));
        }
        if !d.codes.contains(&c.code) {
            out.push(format!("entry {i}: {} doesn't come in code {}", name(c.id), c.code.letter()));
        }
        *copies.entry(c.id).or_default() += 1;
        if d.flags.has(ChipFlags::DARK) {
            dark += 1;
        }
        match d.class {
            ChipClass::Mega => mega += 1,
            ChipClass::Giga => giga += 1,
            _ => {}
        }
    }
    for (&c, &n) in &copies {
        let allowed = copies_allowed(content.chip(c).mb);
        if n > allowed {
            out.push(format!("{} copies of {} ({} MB), past {allowed}", n, name(c), content.chip(c).mb));
        }
    }
    if mega > limits.mega as usize {
        out.push(format!("{mega} Mega chips, past the navi's {}", limits.mega));
    }
    if giga > limits.giga as usize {
        out.push(format!("{giga} Giga chips, past the navi's {}", limits.giga));
    }
    if dark > DARK_CHIPS {
        out.push(format!("{dark} dark chips, past {DARK_CHIPS}"));
    }
    if let Some(r) = folder.regular {
        match folder.chips.get(r as usize) {
            Some(c) if content.chip(c.id).mb > limits.regular_mb => out.push(format!(
                "the Regular chip {} is {} MB, past the navi's {}",
                name(c.id),
                content.chip(c.id).mb,
                limits.regular_mb
            )),
            Some(_) => {}
            None => out.push(format!("the Regular chip is entry {r}, past the folder")),
        }
    }
    if let Some((a, b)) = folder.tags {
        match (folder.chips.get(a as usize), folder.chips.get(b as usize)) {
            _ if a == b => out.push(format!("the tag chips are both entry {a}")),
            (Some(x), Some(y)) => {
                let mb = content.chip(x.id).mb as u32 + content.chip(y.id).mb as u32;
                if mb > TAG_MB {
                    out.push(format!("the tag chips {} and {} are {mb} MB, past {TAG_MB}", name(x.id), name(y.id)));
                }
            }
            _ => out.push(format!("the tag chips are entries {a} and {b}, past the folder")),
        }
        if folder.regular.is_some_and(|r| r == a || r == b) {
            out.push("the Regular chip is a tag chip".into());
        }
    }
    out
}

/// A random legal folder: chips drawn one at a time from the pool, each
/// kept if the rules still hold with it, in one of its codes. The codes
/// lean to two the folder favours (and `*`), as a player's would, so that
/// a hand often has chips to pick together. Its Regular chip is one that
/// fits the navi's Regular memory, if any does.
pub fn random_folder(content: &Content, limits: FolderLimits, draws: &mut Draws) -> SavedFolder {
    let pool = folder_chips(content);
    assert!(!pool.is_empty(), "the content has no chip a folder can hold");
    let favoured = [ChipCode(draws.below(26) as u8), ChipCode(draws.below(26) as u8)];
    let mut chips: Vec<FolderChip> = Vec::with_capacity(FOLDER_SIZE);
    let (mut mega, mut giga) = (0u8, 0u8);
    let mut tries = 0;
    while chips.len() < FOLDER_SIZE {
        tries += 1;
        assert!(tries < 100_000, "no legal folder in the content's chips");
        let id = pool[draws.below(pool.len())];
        let d = content.chip(id);
        let held = chips.iter().filter(|c| c.id == id).count();
        let class_ok = match d.class {
            ChipClass::Mega => mega < limits.mega,
            ChipClass::Giga => giga < limits.giga,
            _ => true,
        };
        if held >= copies_allowed(d.mb) || !class_ok || d.flags.has(ChipFlags::DARK) {
            continue;
        }
        let liked: Vec<ChipCode> =
            d.codes.iter().copied().filter(|c| *c == ChipCode::ASTERISK || favoured.contains(c)).collect();
        let codes = if liked.is_empty() { &d.codes } else { &liked };
        let code = codes[draws.below(codes.len())];
        match d.class {
            ChipClass::Mega => mega += 1,
            ChipClass::Giga => giga += 1,
            _ => {}
        }
        chips.push(FolderChip::new(id, code));
    }
    let fits: Vec<u8> = (0..FOLDER_SIZE as u8).filter(|&i| content.chip(chips[i as usize].id).mb <= limits.regular_mb).collect();
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
    use crate::testing::bn6_content as bn6_test_content;

    fn limits() -> FolderLimits {
        FolderLimits { mega: 5, giga: 1, regular_mb: 50 }
    }

    /// Random folders keep the folder editor's rules, with a Regular chip
    /// that fits.
    #[test]
    fn random_folders_are_legal() {
        let content = bn6_test_content();
        let pool = folder_chips(&content);
        // The pack's folder chips: no Program Advance, no dark chip, none
        // past the pack's (the BeastOut chip). The JP-content chips are
        // folder chips in the Japanese games' records, which the content
        // has (GunDelEX and Django's aren't in the US games').
        let keys: Vec<&str> = pool.iter().map(|&c| nettai_content_api::keys::local(&content.defs.chip(c).key)).collect();
        assert!(keys.len() > 250, "{} chips", keys.len());
        for key in ["cannon", "roll", "bass", "gundels1", "gundelex", "areagrab", "otenko", "gregar", "falzar", "count", "django"] {
            assert!(keys.contains(&key), "{key}");
        }
        for key in ["drksword", "beastout", "gigacan1"] {
            assert!(!keys.contains(&key), "{key}");
        }
        for seed in 0..50 {
            let f = random_folder(&content, limits(), &mut Draws::new(seed));
            assert_eq!(violations(&content, &f, limits()), Vec::<String>::new(), "seed {seed}: {}", describe(&content, &f));
            let r = f.regular.expect("a Regular chip");
            assert!(content.chip(f.chips[r as usize].id).mb <= 50);
        }
        // Another seed, another folder; the same seed, the same.
        let one = |seed| random_folder(&content, limits(), &mut Draws::new(seed));
        assert_ne!(one(1).chips, one(2).chips);
        assert_eq!(one(3).chips, one(3).chips);
    }

    /// What the rules refuse.
    #[test]
    fn the_rules_refuse() {
        let content = bn6_test_content();
        let chip = |key: &str| {
            let id = content.defs.chip_by_key(key).unwrap();
            FolderChip::new(id, content.chip(id).codes[0])
        };
        let base = |chips: [FolderChip; FOLDER_SIZE]| SavedFolder { chips, regular: None, tags: None };
        // Five Recov10s (4 MB) and the rest plain chips is legal; a sixth
        // isn't.
        let mut chips = [chip("bn6:recov10"); FOLDER_SIZE];
        let mut plain = ["bn6:cannon", "bn6:airshot", "bn6:vulcan1", "bn6:spreadr1", "bn6:minibomb", "bn6:sword"].iter().cycle();
        for c in chips.iter_mut().skip(5) {
            *c = chip(plain.next().unwrap());
        }
        assert_eq!(violations(&content, &base(chips), limits()), Vec::<String>::new());
        let mut six = chips;
        six[5] = chip("bn6:recov10");
        assert!(violations(&content, &base(six), limits()).iter().any(|v| v.contains("6 copies of Recov10")));
        // A code the chip doesn't come in.
        let mut code = chips;
        code[0].code = ChipCode(25);
        assert!(violations(&content, &base(code), limits()).iter().any(|v| v.contains("code Z")));
        // Six Mega chips; two Giga chips.
        let mut megas = chips;
        for (i, key) in ["bn6:roll", "bn6:roll2", "bn6:heatman", "bn6:elecman", "bn6:slashman", "bn6:eraseman"].iter().enumerate() {
            megas[10 + i] = chip(key);
        }
        assert!(violations(&content, &base(megas), limits()).iter().any(|v| v.contains("6 Mega chips")));
        let mut gigas = chips;
        gigas[10] = chip("bn6:bass");
        gigas[11] = chip("bn6:deltaray");
        assert!(violations(&content, &base(gigas), limits()).iter().any(|v| v.contains("2 Giga chips")));
        // A Regular chip past the Regular memory (Roll3 is 60 MB).
        let mut regular = base(chips);
        regular.chips[10] = chip("bn6:roll3");
        regular.regular = Some(10);
        assert!(violations(&content, &regular, limits()).iter().any(|v| v.contains("the Regular chip Roll3")));
        // A chip the pack doesn't list.
        let mut dark = chips;
        dark[0] = chip("bn6:drksword");
        assert!(violations(&content, &base(dark), limits()).iter().any(|v| v.contains("no chip a folder can hold")));
        assert_eq!(copies_allowed(19), 5);
        assert_eq!((copies_allowed(20), copies_allowed(39), copies_allowed(49), copies_allowed(50)), (4, 3, 2, 1));
    }
}
