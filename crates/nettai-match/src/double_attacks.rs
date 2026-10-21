//! The DS's Double Attacks in EXE5's party battle (content/exe5/double_attacks,
//! rules/double_attacks): a switch out of Full Synchro to the navi a Double
//! Attack pairs with fires it, once a round, through the party's switch.

use crate::testing::exe5_content;
use crate::Match;
use nettai_battle::battle::{fight, mode};
use nettai_battle::input::PlayerTick;
use nettai_battle::setup::NaviStats;
use nettai_battle::{Battle, Content};
use std::sync::Arc;

/// A folder of Tango's EXE5 saves (their first): Standard chips only.
const FOLDER: [&str; 30] = [
    "cannon A", "cannon A", "cannon B", "cannon B", "airshot *", "airshot *", "airshot *",
    "vulcan1 D", "vulcan1 D", "vulcan1 D", "minibomb B", "minibomb B", "minibomb L", "minibomb L",
    "sword S", "sword S", "sword S", "sword S", "wideswrd S", "wideswrd S", "busterup *",
    "crakout *", "crakout *", "recov10 A", "recov10 A", "recov10 L", "recov10 L", "areagrab S",
    "attck-10 *", "attck-10 *",
];

/// An EXE5 MegaMan's side with that folder (at the party's highest level:
/// a side that states none).
fn side() -> String {
    let folder: Vec<String> = FOLDER
        .iter()
        .map(|c| {
            let (name, code) = c.rsplit_once(' ').unwrap();
            format!("{{ chip = \"{name}\", code = \"{code}\" }}")
        })
        .collect();
    format!("navi = \"megaman\"\nfolder = [{}]\n", folder.join(", "))
}

fn round_match(content: &Arc<Content>) -> Match {
    let stage = crate::ids::local(&content.defs.stage(crate::link_battle_stages(content, "exe5")[0]).key);
    let text = format!(
        "game = \"exe5\"\n\n[[round]]\nstage = \"{stage}\"\n\n[left]\n{}\n\n[right]\n{}\n",
        side(),
        side()
    );
    crate::parse(content, &text).unwrap_or_else(|p| panic!("{p:?}"))
}

/// A round of MegaMan (side 0, with `partners` in his party, members 1 and
/// 2) against MegaMan, both pressing through their custom screens until the
/// fight runs.
fn fighting(content: &Arc<Content>, partners: &[&str]) -> Battle {
    let m = round_match(content);
    let mut b = Battle::new(m.round(content, 0x5EED), content.clone());
    for partner in partners {
        let navi = crate::ids::navi(content, "exe5", partner).unwrap();
        b.party[0].join(NaviStats::fresh(navi, content).expect("the partner's fresh stats"));
    }
    let mut last = [0u16; 2];
    for _ in 0..3000 {
        if b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING {
            return b;
        }
        let input = [0, 1].map(|side| {
            last[side] = nettai_battle::scenario::custom_buttons(&b, side, last[side]);
            PlayerTick::buttons(last[side])
        });
        b.tick(&input, Default::default());
    }
    panic!("the fight never started");
}

/// The objects of kind `key` on the field.
fn count(b: &Battle, key: &str) -> usize {
    let kind = b.content.defs.kind_by_key(key).unwrap_or_else(|| panic!("no kind {key}"));
    b.objects.in_order().filter(|&r| b.objects.get(r).kind == kind).count()
}

fn hp(b: &Battle, side: u8) -> u16 {
    b.objects.get(b.player(side).expect("the side's navi")).hp
}

/// Side 0 asks to switch to `member` on one tick, then `ticks` of nothing;
/// the most directors of Double Attack `key` seen at once, and of its
/// actors. (`mood` first: the outgoing navi's, 0xFF Full Synchro.)
fn switch(b: &mut Battle, member: u8, mood: u8, ticks: usize, key: &str) -> (usize, usize) {
    b.stats[0].mood = mood;
    let ask = PlayerTick { switch_to: Some(member), ..Default::default() };
    b.tick(&[ask, Default::default()], Default::default());
    let (mut directors, mut actors) = (0, 0);
    for _ in 0..ticks {
        b.tick(&Default::default(), Default::default());
        directors = directors.max(count(b, &format!("{key}/director")));
        actors = actors.max(count(b, &format!("{key}/actor")));
    }
    (directors, actors)
}

/// D-HERO: MegaMan in Full Synchro switching to ProtoMan fires it, its
/// director bringing both on; MegaMan's seven volleys each hit the other
/// side for the row's damage at the side's level (40 at 9, as the DS's
/// recording dealt), and the user comes back. A second Full Synchro switch
/// in the round fires nothing.
#[test]
fn d_hero_fires_from_full_synchro() {
    let content = exe5_content();
    let mut b = fighting(&content, &["protoman"]);
    let before = hp(&b, 1);
    let (directors, actors) = switch(&mut b, 1, 0xFF, 600, "d-hero");
    assert_eq!((directors, actors), (1, 2), "one director, two actors");
    assert_eq!(count(&b, "d-hero/director") + count(&b, "d-hero/actor"), 0, "all gone again");
    assert_eq!(before - hp(&b, 1), 7 * 40, "seven volleys of 40");
    assert!(!b.is_dimmed(), "the dimming is over");
    // Back to MegaMan out of ProtoMan's Full Synchro: spent for the round.
    let (directors, _) = switch(&mut b, 0, 0xFF, 200, "d-hero");
    assert_eq!(directors, 0, "once a round");
}

/// Out of any mood but Full Synchro, nothing.
#[test]
fn d_hero_needs_full_synchro() {
    let content = exe5_content();
    let mut b = fighting(&content, &["protoman"]);
    let (directors, _) = switch(&mut b, 1, 0x80, 200, "d-hero");
    assert_eq!(directors, 0);
}

// ---- Each Double Attack's own (in the DS's order) ----------------------------------------
