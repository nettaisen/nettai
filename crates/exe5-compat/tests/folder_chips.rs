//! EXE5's folder rules hold a folder to the chips Tango's do, by the
//! original's chip numbers: for each version, the chips a new build's
//! library takes that are no version's or that version's own (content/exe5/rules/folder; a chip's `version`)
//! are the game's legal chips as Tango lists
//! them (tango-gamesupport-bn5-dataview's `PROTOMAN_LEGAL_CHIPS` and
//! `COLONEL_LEGAL_CHIPS`: the chips its netplay saves' packs hold).

use exe5_compat::Compat;
use nettai_match::build::{Builds, Checked};
use nettai_match::ruleset::{Item, Rulesets};
use nettai_match::{Folder, ids};
use nettai_content_api::Registry;
use std::ops::RangeInclusive;

/// Tango's legal chips, by version.
const PROTOMAN: [RangeInclusive<u16>; 7] = [1..=180, 187..=198, 201..=280, 301..=303, 307..=308, 311..=312, 314..=314];
const COLONEL: [RangeInclusive<u16>; 5] = [1..=180, 187..=198, 201..=280, 304..=306, 309..=313];

#[test]
fn a_folder_holds_tangos_legal_chips() {
    let content = nettai_match::testing::exe5_content();
    let compat = Compat::exe5();
    let builds = Builds::of(&content, "exe5").unwrap();
    let rulesets = Rulesets::of(&content, "exe5").unwrap();
    for (version, legal) in [("protoman", &PROTOMAN[..]), ("colonel", &COLONEL[..])] {
        let side = builds.new_build(&content, version).unwrap();
        let view = Checked::of(&content, "exe5", &side).folder_view(&content, &Folder::EMPTY).unwrap();
        for number in 1..=0x1A6u16 {
            let Some(chip) = compat.chip_key(number).and_then(|k| ids::chip(&content, "exe5", k)) else {
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
    // Each version's six Giga chips are its own; ChaosLrd (312) and LeadRaid
    // (311, a Mega chip) are both's.
    let owners: std::collections::HashMap<_, Vec<&str>> = rulesets
        .groups
        .iter()
        .flat_map(|g| rulesets.group_items(rulesets.category("folder").unwrap(), g).into_iter().map(move |i| (nettai_content_api::ChipHandle(i.def), vec![g.id.as_str()])))
        .collect();
    let key = |n: u16| ids::chip(&content, "exe5", compat.chip_key(n).unwrap()).unwrap();
    for n in [301, 302, 303, 307, 308, 314] {
        assert_eq!(owners[&key(n)], ["protoman"], "chip {n}");
    }
    for n in [304, 305, 306, 309, 310, 313] {
        assert_eq!(owners[&key(n)], ["colonel"], "chip {n}");
    }
    assert_eq!(owners.len(), 12);
}
