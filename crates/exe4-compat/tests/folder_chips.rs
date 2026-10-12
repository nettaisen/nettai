//! EXE4's folder rules hold a folder to the chips Tango's do, by the
//! original's chip numbers: for each version, the chips a new build's
//! library takes (content/exe4/rules/folder, and the version's `giga_chips`
//! of content/exe4/rules/builds) are the game's legal chips as Tango lists
//! them (tango-gamesupport-bn4-dataview's `RED_SUN_LEGAL_CHIPS` and
//! `BLUE_MOON_LEGAL_CHIPS`: the chips its netplay saves' packs hold).

use exe4_compat::Compat;
use nettai_match::build::{Builds, Checked};
use nettai_match::{Folder, Wrong, ids};
use std::ops::RangeInclusive;

/// Tango's legal chips, by version.
const RED_SUN: [RangeInclusive<u16>; 6] = [1..=186, 201..=280, 301..=301, 304..=304, 307..=308, 310..=312];
const BLUE_MOON: [RangeInclusive<u16>; 6] = [1..=186, 201..=280, 302..=303, 305..=306, 309..=309, 311..=312];

#[test]
fn a_folder_holds_tangos_legal_chips() {
    let content = nettai_match::testing::exe4_content();
    let compat = Compat::exe4();
    let builds = Builds::of(&content, "exe4").unwrap();
    for (version, legal) in [("redsun", &RED_SUN), ("bluemoon", &BLUE_MOON)] {
        let side = builds.new_build(&content, version).unwrap();
        let view = Checked::of(&content, "exe4", &side).folder_view(&content, &Folder::EMPTY).unwrap();
        for number in 1..=0x184u16 {
            let Some(chip) = compat.chip_key(number).and_then(|k| ids::chip(&content, "exe4", k)) else {
                assert!(!legal.iter().any(|r| r.contains(&number)), "{version}: Tango's legal chip {number} is none of the content's");
                continue;
            };
            let held = view.lists(chip) && !matches!(view.refusal(chip), Some(Wrong::ChipOfAnotherVersion { .. }));
            assert_eq!(held, legal.iter().any(|r| r.contains(&number)), "{version}: chip {number} ({})", compat.chip_key(number).unwrap());
        }
    }
    // Each version's five Giga chips are its own; the e-Reader's Duo (311)
    // and PrixPowr (312, a Mega chip) are both's.
    let owners = builds.version_chips(&content).unwrap();
    let key = |n: u16| ids::chip(&content, "exe4", compat.chip_key(n).unwrap()).unwrap();
    for n in [301, 304, 307, 308, 310] {
        assert_eq!(owners[&key(n)], ["redsun"], "chip {n}");
    }
    for n in [302, 303, 305, 306, 309] {
        assert_eq!(owners[&key(n)], ["bluemoon"], "chip {n}");
    }
    assert_eq!(owners.len(), 10);
}
