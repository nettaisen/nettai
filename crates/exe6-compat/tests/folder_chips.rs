//! EXE6's folder rules hold a folder to the chips Tango's do, by the
//! original's chip numbers: for each version, the chips a new build's
//! library takes that are no version's or that version's own
//! (content/exe6/rules/folder; a chip's `version`) are the Japanese game's legal
//! chips as Tango lists them (tango-gamesupport-bn6-dataview's
//! `EXE6_GREGAR_LEGAL_CHIPS` and `EXE6_FALZAR_LEGAL_CHIPS`: the chips its
//! netplay saves' packs hold).

use exe6_compat::Compat;
use nettai_match::build::{Builds, Checked};
use nettai_match::ruleset::{Item, Rulesets};
use nettai_match::{Folder, ids};
use nettai_content_api::Registry;
use std::ops::RangeInclusive;

/// Tango's legal chips of the Japanese games, by version.
const GREGAR: [RangeInclusive<u16>; 4] = [1..=202, 221..=280, 301..=305, 311..=312];
const FALZAR: [RangeInclusive<u16>; 4] = [1..=202, 221..=280, 306..=311, 313..=313];

#[test]
fn a_folder_holds_tangos_legal_chips() {
    let content = nettai_match::testing::exe6_content();
    let compat = Compat::exe6();
    let builds = Builds::of(&content, "exe6").unwrap();
    let rulesets = Rulesets::of(&content, "exe6").unwrap();
    for (version, legal) in [("gregar", &GREGAR), ("falzar", &FALZAR)] {
        let side = builds.new_build(&content, version).unwrap();
        let view = Checked::of(&content, "exe6", &side).folder_view(&content, &Folder::EMPTY).unwrap();
        for number in 1..=0x1A0u16 {
            let Some(chip) = compat.chip_key(number).and_then(|k| ids::chip(&content, "exe6", k)) else {
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
    // Each version's own chips are its group's.
    let owners: std::collections::HashMap<_, Vec<&str>> = rulesets
        .groups
        .iter()
        .flat_map(|g| rulesets.group_items(rulesets.category("folder").unwrap(), g).into_iter().map(move |i| (nettai_content_api::ChipHandle(i.def), vec![g.id.as_str()])))
        .collect();
    let key = |n: u16| ids::chip(&content, "exe6", compat.chip_key(n).unwrap()).unwrap();
    for n in (301..=305).chain([312]) {
        assert_eq!(owners[&key(n)], ["gregar"], "chip {n}");
    }
    for n in (306..=310).chain([313]) {
        assert_eq!(owners[&key(n)], ["falzar"], "chip {n}");
    }
    assert_eq!(owners.len(), 12, "DblBeast (311) is both versions'");
}
