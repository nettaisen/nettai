//! EXE4's folder rules hold a folder to the chips Tango's do, by the
//! original's chip numbers: for each version, the chips a new build's
//! library takes that are no version's or that version's own (content/exe4/rules/folder; a chip's `version`)
//! are the game's legal chips as Tango lists
//! them (tango-gamesupport-bn4-dataview's `RED_SUN_LEGAL_CHIPS` and
//! `BLUE_MOON_LEGAL_CHIPS`: the chips its netplay saves' packs hold).

use exe4_compat::Compat;
use nettai_match::build::{Builds, Checked};
use nettai_match::ruleset::{Item, Rulesets};
use nettai_match::{Folder, ids};
use nettai_content_api::Registry;
use std::ops::RangeInclusive;

/// Tango's legal chips, by version.
const RED_SUN: [RangeInclusive<u16>; 6] = [1..=186, 201..=280, 301..=301, 304..=304, 307..=308, 310..=312];
const BLUE_MOON: [RangeInclusive<u16>; 6] = [1..=186, 201..=280, 302..=303, 305..=306, 309..=309, 311..=312];

#[test]
fn a_folder_holds_tangos_legal_chips() {
    let content = nettai_match::testing::exe4_content();
    let compat = Compat::exe4();
    let builds = Builds::of(&content, "exe4").unwrap();
    let rulesets = Rulesets::of(&content, "exe4").unwrap();
    for (version, legal) in [("redsun", &RED_SUN), ("bluemoon", &BLUE_MOON)] {
        let side = builds.new_build(&content, version).unwrap();
        let view = Checked::of(&content, "exe4", &side).folder_view(&content, &Folder::EMPTY).unwrap();
        for number in 1..=0x184u16 {
            let Some(chip) = compat.chip_key(number).and_then(|k| ids::chip(&content, "exe4", k)) else {
                assert!(!legal.iter().any(|r| r.contains(&number)), "{version}: Tango's legal chip {number} is none of the content's");
                continue;
            };
            // (A build may hold another version's own chips: a save of the version holds those of its group and of
            // none, docs/design/versions.md §2.1.)
            let own = rulesets.group_of(Registry::Chip, &Item::of(chip.0)).is_none_or(|g| g.id == version);
            let held = view.lists(chip) && own;
            assert_eq!(held, legal.iter().any(|r| r.contains(&number)), "{version}: chip {number} ({})", compat.chip_key(number).unwrap());
        }
    }
    // Each version's five Giga chips are its own; the e-Reader's Duo (311)
    // and PrixPowr (312, a Mega chip) are both's.
    let owners: std::collections::HashMap<_, Vec<&str>> = rulesets
        .groups
        .iter()
        .flat_map(|g| rulesets.group_items(rulesets.category("folder").unwrap(), g).into_iter().map(move |i| (nettai_content_api::ChipHandle(i.def), vec![g.id.as_str()])))
        .collect();
    let key = |n: u16| ids::chip(&content, "exe4", compat.chip_key(n).unwrap()).unwrap();
    for n in [301, 304, 307, 308, 310] {
        assert_eq!(owners[&key(n)], ["redsun"], "chip {n}");
    }
    for n in [302, 303, 305, 306, 309] {
        assert_eq!(owners[&key(n)], ["bluemoon"], "chip {n}");
    }
    assert_eq!(owners.len(), 10);
}
