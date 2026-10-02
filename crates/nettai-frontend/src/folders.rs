//! BN6's folder rules, as its folder editor enforces them, and a random
//! folder that keeps them (live play's: docs/frontend.md §2).
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
//! are left unset.
//!
//! What can be in a folder is what the chip pack lists (`sub_811FE7C`):
//! chips 1 to 0x13A, without the extra flag 0x20 (the dark chips); of them
//! the folder chips, Standard, Mega and Giga. The JP-content chips are among
//! them: content/bn6 has the Japanese games' records and routines for
//! GunDelEX, Otenko, HackJack's, Django's, Gregar and Falzar (GunDelEX and
//! Django's are folder chips only in the Japanese records; a US console has
//! no routine for HackJack's, Django's, Gregar or Falzar).

use bn6_compat::Compat;
use nettai_battle::content::{ChipClass, ChipCode, ChipFlags, Content};
use nettai_battle::custom::folder::FOLDER_SIZE;
use nettai_battle::custom::{FolderChip, SavedFolder};
use nettai_battle::setup::NaviStats;
use nettai_content_api::ChipHandle;

/// The last chip the pack lists (`sub_811FE7C`).
const LAST_PACK_CHIP: u16 = 0x13A;
/// The extra flag of the chips the pack doesn't list (the dark chips).
const NOT_IN_PACK: u8 = 0x20;
/// Chips with the dark flag a folder can hold (`sub_8135080`).
const DARK_CHIPS: usize = 3;

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
    let compat = Compat::bn6();
    content
        .defs
        .chips
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            let r = &d.record;
            let listed = compat.chips.get(&d.key).is_some_and(|c| (1..=LAST_PACK_CHIP).contains(&c.id));
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
    let name = |c: ChipHandle| content.chip(c).name.clone();
    let mut copies: std::collections::BTreeMap<ChipHandle, usize> = Default::default();
    let (mut mega, mut giga, mut dark) = (0, 0, 0);
    for (i, c) in folder.chips.iter().enumerate() {
        let d = content.chip(c.id);
        if !pool.contains(&c.id) {
            out.push(format!("entry {i}: {} is no chip a folder can hold", d.name));
        }
        if !d.codes.contains(&c.code) {
            out.push(format!("entry {i}: {} doesn't come in code {}", d.name, c.code.letter()));
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
    if folder.tags.is_some() {
        out.push("tag chips are set".into());
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

/// The frontend's own random draws for a setup (splitmix64): not the
/// game's RNG, which the battle keeps.
#[derive(Clone, Debug)]
pub struct Draws(u64);

impl Draws {
    pub fn new(seed: u32) -> Draws {
        Draws(seed as u64 ^ 0x6E65_7474_6169_0000)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number below `n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// `items` in a random order.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, self.below(i + 1));
        }
    }
}

/// A folder in a line: each chip's name and code, the Regular chip marked.
pub fn describe(content: &Content, folder: &SavedFolder) -> String {
    folder
        .chips
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mark = if folder.regular == Some(i as u8) { " (Regular)" } else { "" };
            format!("{} {}{mark}", content.chip(c.id).name, c.code.letter())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// BN6's content for tests: content/bn6's definitions on a made-up asset
/// index (`testing::asset_names_used`), every sprite timed as the test
/// content's navi is (nothing from a ROM).
#[cfg(test)]
pub(crate) fn bn6_test_content() -> std::sync::Arc<Content> {
    use nettai_battle::content::testing;
    static BN6: std::sync::OnceLock<std::sync::Arc<Content>> = std::sync::OnceLock::new();
    BN6.get_or_init(|| {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");
        let mut c = Content::default();
        c.scripts.modules = testing::modules_under(dir);
        c.assets = testing::asset_names_used(&c.scripts.modules);
        let mut navi = testing::content().animations.sprites[&testing::NAVI_SPRITE].clone();
        navi.resize(0x40, navi[1].clone());
        for &id in c.assets.sprites.values() {
            c.animations.sprites.insert(id, navi.clone());
        }
        c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
        std::sync::Arc::new(c)
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let keys: Vec<&str> = pool.iter().map(|&c| content.defs.chip(c).key.as_str()).collect();
        assert!(keys.len() > 250, "{} chips", keys.len());
        for key in ["cannon", "roll", "bass", "gundels1", "gundelex", "areagrab", "otenko", "gregar", "falzar", "hackjack", "django"] {
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
        let mut chips = [chip("recov10"); FOLDER_SIZE];
        let mut plain = ["cannon", "airshot", "vulcan1", "spreadr1", "minibomb", "sword"].iter().cycle();
        for c in chips.iter_mut().skip(5) {
            *c = chip(plain.next().unwrap());
        }
        assert_eq!(violations(&content, &base(chips), limits()), Vec::<String>::new());
        let mut six = chips;
        six[5] = chip("recov10");
        assert!(violations(&content, &base(six), limits()).iter().any(|v| v.contains("6 copies of Recov10")));
        // A code the chip doesn't come in.
        let mut code = chips;
        code[0].code = ChipCode(25);
        assert!(violations(&content, &base(code), limits()).iter().any(|v| v.contains("code Z")));
        // Six Mega chips; two Giga chips.
        let mut megas = chips;
        for (i, key) in ["roll", "roll2", "heatman", "elecman", "slashman", "eraseman"].iter().enumerate() {
            megas[10 + i] = chip(key);
        }
        assert!(violations(&content, &base(megas), limits()).iter().any(|v| v.contains("6 Mega chips")));
        let mut gigas = chips;
        gigas[10] = chip("bass");
        gigas[11] = chip("deltaray");
        assert!(violations(&content, &base(gigas), limits()).iter().any(|v| v.contains("2 Giga chips")));
        // A Regular chip past the Regular memory (Roll3 is 60 MB).
        let mut regular = base(chips);
        regular.chips[10] = chip("roll3");
        regular.regular = Some(10);
        assert!(violations(&content, &regular, limits()).iter().any(|v| v.contains("the Regular chip Roll3")));
        // A chip the pack doesn't list.
        let mut dark = chips;
        dark[0] = chip("drksword");
        assert!(violations(&content, &base(dark), limits()).iter().any(|v| v.contains("no chip a folder can hold")));
        assert_eq!(copies_allowed(19), 5);
        assert_eq!((copies_allowed(20), copies_allowed(39), copies_allowed(49), copies_allowed(50)), (4, 3, 2, 1));
    }
}
