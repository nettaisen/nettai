//! The DS's Double Attacks in EXE5's party battle (content/exe5/double_attacks,
//! rules/double_attacks): a switch out of Full Synchro to the navi a Double
//! Attack pairs with fires it, once a round, through the party's switch.

use crate::testing::exe5_content;
use crate::Match;
use nettai_battle::battle::{fight, mode};
use nettai_battle::input::PlayerTick;
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

/// An EXE5 MegaMan's side with that folder and `partners` in his party (at
/// the party's highest level: a side that states none).
fn side(partners: &[&str]) -> String {
    let folder: Vec<String> = FOLDER
        .iter()
        .map(|c| {
            let (name, code) = c.rsplit_once(' ').unwrap();
            format!("{{ chip = \"{name}\", code = \"{code}\" }}")
        })
        .collect();
    let party: Vec<String> = partners.iter().map(|p| format!("{{ navi = \"{p}\" }}")).collect();
    format!("navi = \"megaman\"\nfolder = [{}]\nparty = [{}]\n", folder.join(", "), party.join(", "))
}

fn round_match(content: &Arc<Content>, partners: &[&str]) -> Match {
    let stage = crate::ids::local(&content.defs.stage(crate::link_battle_stages(content, "exe5")[0]).key);
    let text = format!(
        "game = \"exe5\"\n\n[[round]]\nstage = \"{stage}\"\n\n[left]\n{}\n\n[right]\n{}\n",
        side(partners),
        side(&[])
    );
    crate::parse(content, &text).unwrap_or_else(|p| panic!("{p:?}"))
}

/// A round of MegaMan (side 0, with `partners` in his party, members 1 and
/// 2) against MegaMan, both pressing through their custom screens until the
/// fight runs.
fn fighting(content: &Arc<Content>, partners: &[&str]) -> Battle {
    let m = round_match(content, partners);
    let mut b = Battle::new(m.round(content, 0x5EED), content.clone());
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
/// the most controllers of Double Attack `key` seen at once, and of its
/// navis. (`mood` first: the outgoing navi's, 0xFF Full Synchro.)
fn switch(b: &mut Battle, member: u8, mood: u8, ticks: usize, key: &str) -> (usize, usize) {
    b.stats[0].mood = mood;
    let ask = PlayerTick { switch_to: Some(member), ..Default::default() };
    b.tick(&[ask, Default::default()], Default::default());
    let (mut controllers, mut navis) = (0, 0);
    for _ in 0..ticks {
        b.tick(&Default::default(), Default::default());
        controllers = controllers.max(count(b, &format!("{key}/controller")));
        navis = navis.max(count(b, &format!("{key}/navi")));
    }
    (controllers, navis)
}

/// D-HERO: MegaMan in Full Synchro switching to ProtoMan fires it, its
/// controller bringing both on; MegaMan's seven volleys each hit the other
/// side for the row's damage at the side's level (40 at 9, as the DS's
/// recording dealt), and the user comes back. A second Full Synchro switch
/// in the round fires nothing.
#[test]
fn d_hero_fires_from_full_synchro() {
    let content = exe5_content();
    let mut b = fighting(&content, &["protoman"]);
    let before = hp(&b, 1);
    let (controllers, navis) = switch(&mut b, 1, 0xFF, 600, "d-hero");
    assert_eq!((controllers, navis), (1, 2), "one controller, two navis");
    assert_eq!(count(&b, "d-hero/controller") + count(&b, "d-hero/navi"), 0, "all gone again");
    assert_eq!(before - hp(&b, 1), 7 * 40, "seven volleys of 40");
    assert!(!b.is_dimmed(), "the dimming is over");
    // Back to MegaMan out of ProtoMan's Full Synchro: spent for the round.
    let (controllers, _) = switch(&mut b, 0, 0xFF, 200, "d-hero");
    assert_eq!(controllers, 0, "once a round");
}

/// Out of any mood but Full Synchro, nothing.
#[test]
fn d_hero_needs_full_synchro() {
    let content = exe5_content();
    let mut b = fighting(&content, &["protoman"]);
    let (controllers, _) = switch(&mut b, 1, 0x80, 200, "d-hero");
    assert_eq!(controllers, 0);
}

// ---- Each Double Attack's own (in the DS's order) ----------------------------------------

/// M-ZAN: MegaMan to ProtoMan out of no Full Synchro (nothing), then
/// ProtoMan in Full Synchro to MagnetMan: MagnetMan holds the opponent in
/// place, ProtoMan steps before it and slashes once, 240 at level 9 (the
/// DS's recording's).
#[test]
fn m_zan_fires_from_protoman_to_magnetman() {
    let content = exe5_content();
    let mut b = fighting(&content, &["protoman", "magnetman"]);
    let (controllers, _) = switch(&mut b, 1, 0x80, 100, "m-zan");
    assert_eq!(controllers, 0, "MegaMan and ProtoMan out of no Full Synchro: nothing");
    let before = hp(&b, 1);
    let (controllers, navis) = switch(&mut b, 2, 0xFF, 700, "m-zan");
    assert_eq!((controllers, navis), (1, 2), "one controller, two navis");
    assert_eq!(count(&b, "m-zan/controller") + count(&b, "m-zan/navi"), 0, "all gone again");
    assert_eq!(before - hp(&b, 1), 240, "one slash of 240");
}

/// Side 0 asks to switch to `member` (the outgoing navi's mood `mood`
/// first), then `ticks` of nothing: how many objects of each kind of `keys`
/// came in all (each counted as it first shows up in a slot).
fn switch_counting(b: &mut Battle, member: u8, mood: u8, ticks: usize, keys: &[&str]) -> Vec<usize> {
    b.stats[0].mood = mood;
    let kinds: Vec<_> = keys.iter().map(|k| b.content.defs.kind_by_key(k).unwrap_or_else(|| panic!("no kind {k}"))).collect();
    let mut counts = vec![0; keys.len()];
    let mut before: Vec<Vec<nettai_battle::object::ObjectRef>> = vec![Vec::new(); keys.len()];
    let ask = PlayerTick { switch_to: Some(member), ..Default::default() };
    b.tick(&[ask, Default::default()], Default::default());
    for _ in 0..ticks {
        b.tick(&Default::default(), Default::default());
        for (i, k) in kinds.iter().enumerate() {
            let now: Vec<_> = b.objects.in_order().filter(|&r| b.objects.get(r).kind == *k).collect();
            counts[i] += now.iter().filter(|&r| !before[i].contains(r)).count();
            before[i] = now;
        }
    }
    counts
}

/// M-AIR: MegaMan to GyroMan out of no Full Synchro (nothing), then GyroMan
/// in Full Synchro to MagnetMan: MagnetMan sends a bomb up for each panel of
/// the other side's but one (eight), GyroMan drops one on each of them as he
/// flies over (nine), and the one on the opponent's panel hits: 200 at level
/// 9, Elec, the DS's recording's.
#[test]
fn m_air_fires_from_gyroman_to_magnetman() {
    let content = exe5_content();
    let mut b = fighting(&content, &["gyroman", "magnetman"]);
    let (controllers, _) = switch(&mut b, 1, 0x80, 100, "m-air");
    assert_eq!(controllers, 0, "MegaMan and GyroMan out of no Full Synchro: nothing");
    let before = hp(&b, 1);
    let keys = ["m-air/controller", "m-air/navi", "m-air/toss", "m-air/bomb"];
    let counts = switch_counting(&mut b, 2, 0xFF, 900, &keys);
    assert_eq!(counts, [1, 2, 8, 9], "{keys:?}");
    assert_eq!(count(&b, "m-air/controller") + count(&b, "m-air/navi"), 0, "all gone again");
    assert_eq!(before - hp(&b, 1), 200, "one bomb of 200");
}

/// D-VALCAN: MegaMan in Full Synchro to NapalmMan: each fires 30 bullets
/// down the row, every one hitting the opponent: 60 of 6 at level 9, 360,
/// the DS's recording's.
#[test]
fn d_valcan_fires_from_megaman_to_napalmman() {
    let content = exe5_content();
    let mut b = fighting(&content, &["napalmman"]);
    let before = hp(&b, 1);
    let keys = ["d-valcan/controller", "d-valcan/navi", "bullet"];
    let counts = switch_counting(&mut b, 1, 0xFF, 900, &keys);
    assert_eq!(counts, [1, 2, 60], "{keys:?}");
    assert_eq!(count(&b, "d-valcan/controller") + count(&b, "d-valcan/navi"), 0, "all gone again");
    assert_eq!(before - hp(&b, 1), 360, "60 bullets of 6");
}

/// BR-RUSH: MegaMan in Full Synchro to Colonel: Colonel's cape flies from
/// the panel before the opponent and catches it; both drop there and land
/// four hits each, eight of 50 at level 9, 400 (the DS's recording's); the
/// cape goes with them.
#[test]
fn br_rush_fires_from_megaman_to_colonel() {
    let content = exe5_content();
    let mut b = fighting(&content, &["colonel"]);
    let before = hp(&b, 1);
    let keys = ["br-rush/controller", "br-rush/navi", "br-rush/cape"];
    let counts = switch_counting(&mut b, 1, 0xFF, 900, &keys);
    assert_eq!(counts, [1, 2, 1], "{keys:?}");
    for key in keys {
        assert_eq!(count(&b, key), 0, "{key}: gone again");
    }
    assert_eq!(before - hp(&b, 1), 400, "eight hits of 50");
}

/// E-SAPPOU: MegaMan to Colonel out of no Full Synchro (nothing), then
/// Colonel in Full Synchro to ShadowMan: Colonel drops a rock before the
/// opponent and one behind it; each becomes a copy of ShadowMan whose moon
/// blade hits it once, 150 at level 9, with an HP drain of level 1 (bug
/// 0x18, argument 1), a point every 40 ticks once the fight goes on: the
/// DS's recording lost 300, then 6 by its end.
#[test]
fn e_sappou_fires_from_colonel_to_shadowman() {
    let content = exe5_content();
    let mut b = fighting(&content, &["colonel", "shadowman"]);
    let (controllers, _) = switch(&mut b, 1, 0x80, 100, "e-sappou");
    assert_eq!(controllers, 0, "MegaMan and Colonel out of no Full Synchro: nothing");
    let before = hp(&b, 1);
    let keys = ["e-sappou/controller", "e-sappou/navi", "e-sappou/rock", "e-sappou/blade"];
    let counts = switch_counting(&mut b, 2, 0xFF, 500, &keys);
    assert_eq!(counts, [1, 2, 2, 2], "{keys:?}");
    for key in keys {
        assert_eq!(count(&b, key), 0, "{key}: gone again");
    }
    assert_eq!(before - hp(&b, 1), 300, "two slashes of 150");
    for _ in 0..200 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!(before - hp(&b, 1), 305, "and the drain: a point every 40 ticks");
}

/// K-THROW: MegaMan to TomahawkMan out of no Full Synchro (nothing), then
/// TomahawkMan in Full Synchro to KnightMan: KnightMan, thrown, lands three
/// panels ahead on the opponent: one Wood hit of 450 at level 9 (the DS's
/// recording's), and the 3x3 around it cracked.
#[test]
fn k_throw_fires_from_tomahawkman_to_knightman() {
    let content = exe5_content();
    let mut b = fighting(&content, &["tomahawkman", "knightman"]);
    let (controllers, _) = switch(&mut b, 1, 0x80, 100, "k-throw");
    assert_eq!(controllers, 0, "MegaMan and TomahawkMan out of no Full Synchro: nothing");
    let before = hp(&b, 1);
    let (controllers, navis) = switch(&mut b, 2, 0xFF, 700, "k-throw");
    assert_eq!((controllers, navis), (1, 2), "one controller, two navis");
    assert_eq!(count(&b, "k-throw/controller") + count(&b, "k-throw/navi"), 0, "all gone again");
    assert_eq!(before - hp(&b, 1), 450, "one hit of 450");
}

/// G-OTOSHI: MegaMan to ShadowMan (no pair: nothing), then ShadowMan in
/// Full Synchro to ToadMan: twelve frogs, each 60 at level 9 where it hits,
/// turning panels to sea. Their panels are RNG2's: in this round nine hit,
/// 540, as nine did in the DS's recording.
#[test]
fn g_otoshi_fires_from_shadowman_to_toadman() {
    let content = exe5_content();
    let mut b = fighting(&content, &["shadowman", "toadman"]);
    let (controllers, _) = switch(&mut b, 1, 0xFF, 100, "g-otoshi");
    assert_eq!(controllers, 0, "MegaMan and ShadowMan make no Double Attack");
    let before = hp(&b, 1);
    let keys = ["g-otoshi/controller", "g-otoshi/navi", "g-otoshi/frog"];
    let counts = switch_counting(&mut b, 2, 0xFF, 700, &keys);
    assert_eq!(counts, [1, 2, 12], "{keys:?}");
    for key in keys {
        assert_eq!(count(&b, key), 0, "{key}: gone again");
    }
    assert_eq!(before - hp(&b, 1), 540, "nine hits of 60");
    let sea = (1..=6u8).flat_map(|x| (1..=3u8).map(move |y| (x, y))).filter(|&(x, y)| b.field.panel(x, y).is_some_and(|p| b.content.rules().panels.name(p.kind) == "sea")).count();
    assert!(sea > 0, "panels turned to sea");
}

/// B-SHOOT: MegaMan in Full Synchro to KnightMan: KnightMan drops six balls
/// and MegaMan kicks them into the row above KnightMan's, his, and the one
/// below in turn; the two in the opponent's row hit it, two of 120 at level
/// 9, 240 (the DS's recording's).
#[test]
fn b_shoot_fires_from_megaman_to_knightman() {
    let content = exe5_content();
    let mut b = fighting(&content, &["knightman"]);
    let before = hp(&b, 1);
    let keys = ["b-shoot/controller", "b-shoot/navi", "b-shoot/ball"];
    let counts = switch_counting(&mut b, 1, 0xFF, 900, &keys);
    assert_eq!(counts, [1, 2, 6], "{keys:?}");
    for key in keys {
        assert_eq!(count(&b, key), 0, "{key}: gone again");
    }
    assert_eq!(before - hp(&b, 1), 240, "two balls of 120");
}

/// C-SCOPE: MegaMan in Full Synchro to SearchMan; the scopes walk their
/// crossing diagonals, and side 0's player presses A (once, while dimmed)
/// as one stands on the opponent's panel: they stop where they cross, and
/// each navi's five shots hit it, ten of 40 at level 9, 400 (the DS's
/// recording `c-scope`'s). The scopes come and go.
#[test]
fn c_scope_stops_on_a() {
    let content = exe5_content();
    let mut b = fighting(&content, &["searchman"]);
    let before = hp(&b, 1);
    b.stats[0].mood = 0xFF;
    let ask = PlayerTick { switch_to: Some(1), ..Default::default() };
    b.tick(&[ask, Default::default()], Default::default());
    let scope = b.content.defs.kind_by_key("c-scope/scope").unwrap();
    let target = b.objects.get(b.player(1).unwrap()).panel;
    let (mut pressed, mut scopes, mut controllers) = (false, 0, 0);
    for _ in 0..900 {
        let on = b.objects.in_order().any(|r| b.objects.get(r).kind == scope && b.objects.get(r).panel == target);
        let a = if on && !pressed {
            pressed = true;
            nettai_battle::input::keys::A
        } else {
            0
        };
        b.tick(&[PlayerTick::buttons(a), Default::default()], Default::default());
        scopes = scopes.max(count(&b, "c-scope/scope"));
        controllers = controllers.max(count(&b, "c-scope/controller"));
    }
    assert!(pressed, "a scope stood on the opponent's panel");
    assert_eq!((controllers, scopes), (1, 2), "one controller, two scopes");
    for key in ["c-scope/controller", "c-scope/navi", "c-scope/scope"] {
        assert_eq!(count(&b, key), 0, "{key}: gone again");
    }
    assert_eq!(before - hp(&b, 1), 400, "ten shots of 40");
}

/// C-SCOPE left alone: the scopes walk their eight rounds and stop where the
/// last ends, on the panels beside the opponent's (the DS's recording
/// `c-scope-no-stop` misses).
#[test]
fn c_scope_unstopped_misses() {
    let content = exe5_content();
    let mut b = fighting(&content, &["searchman"]);
    let before = hp(&b, 1);
    let (controllers, navis) = switch(&mut b, 1, 0xFF, 900, "c-scope");
    assert_eq!((controllers, navis), (1, 2), "one controller, two navis");
    assert_eq!(count(&b, "c-scope/controller") + count(&b, "c-scope/navi") + count(&b, "c-scope/scope"), 0, "all gone again");
    assert_eq!(before - hp(&b, 1), 0, "every shot misses");
}

/// C-BOMB: MegaMan to NapalmMan out of no Full Synchro (nothing), then
/// NapalmMan in Full Synchro to Meddy: Meddy puts a capsule out, NapalmMan
/// lobs his napalm bomb four columns ahead; its fire hits the opponent and
/// sets the capsule off: two hits of 150 at level 9, 300 (the DS's
/// recording's).
#[test]
fn c_bomb_fires_from_napalmman_to_meddy() {
    let content = exe5_content();
    let mut b = fighting(&content, &["napalmman", "meddy"]);
    let (controllers, _) = switch(&mut b, 1, 0x80, 100, "c-bomb");
    assert_eq!(controllers, 0, "MegaMan and NapalmMan out of no Full Synchro: nothing");
    let before = hp(&b, 1);
    let keys = ["c-bomb/controller", "c-bomb/navi", "c-bomb/capsule", "napalmmn/bomb"];
    let counts = switch_counting(&mut b, 2, 0xFF, 900, &keys);
    assert_eq!(counts, [1, 2, 1, 1], "{keys:?}");
    for key in keys {
        assert_eq!(count(&b, key), 0, "{key}: gone again");
    }
    assert_eq!(before - hp(&b, 1), 300, "the bomb's fire and the capsule, 150 each");
}

/// K-BOMB: MegaMan in Full Synchro to NumberMan: four bombs at the panel
/// three columns ahead, each bursting on a quarter of the 3x3 around it with
/// the middle panel in every quarter: the opponent there takes four hits of
/// 100 at level 9, 400 (the DS's recording's).
#[test]
fn k_bomb_fires_from_megaman_to_numberman() {
    let content = exe5_content();
    let mut b = fighting(&content, &["numberman"]);
    let before = hp(&b, 1);
    let keys = ["k-bomb/controller", "k-bomb/navi", "k-bomb/bomb"];
    let counts = switch_counting(&mut b, 1, 0xFF, 900, &keys);
    assert_eq!(counts, [1, 2, 4], "{keys:?}");
    for key in keys {
        assert_eq!(count(&b, key), 0, "{key}: gone again");
    }
    assert_eq!(before - hp(&b, 1), 400, "four hits of 100");
}

/// The paused path: a switch on the custom screen out of Full Synchro (EXE5
/// DS's state 0x40, then the turn's paused change) to the partner keeps it
/// and asks for the follow-up (action 0x5C), which dims, lands and launches
/// D-HERO once the fight goes on: the same seven volleys, its telop 58 ticks
/// after the follow-up's landing sound (the DS's d-hero-custom-screen
/// recording: 0x8D at tick 875, 0x173 at 933).
#[test]
fn d_hero_fires_after_a_paused_switch() {
    use nettai_battle::custom::screen::{OK_SLOT, Phase};
    use nettai_battle::input::keys;
    let content = exe5_content();
    let m = round_match(&content, &["protoman"]);
    let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
    let choosing = |b: &Battle| b.custom.sides[0].screen.is_some_and(|s| s.phase == Phase::Choosing);
    for _ in 0..400 {
        if choosing(&b) {
            break;
        }
        b.tick(&Default::default(), Default::default());
    }
    assert!(choosing(&b));
    let ask = PlayerTick { switch_to: Some(1), ..Default::default() };
    b.tick(&[ask, Default::default()], Default::default());
    for _ in 0..100 {
        if choosing(&b) {
            break;
        }
        b.tick(&Default::default(), Default::default());
    }
    // MegaMan leaves in Full Synchro.
    b.stats[0].mood = 0xFF;
    b.custom.sides[0].screen.as_mut().unwrap().cursor = OK_SLOT;
    b.tick(&[PlayerTick::buttons(keys::A), Default::default()], Default::default());
    let before = hp(&b, 1);
    let (mut controllers, mut navis) = (0, 0);
    let names = content.assets.names(nettai_content_api::AssetKind::Sound);
    let (mut landed, mut telop) = (None, None);
    for t in 0..1500 {
        if b.custom.sides[1].built.is_none()
            && let Some(s) = b.custom.sides[1].screen.as_mut()
            && s.phase == Phase::Choosing
        {
            s.cursor = OK_SLOT;
            b.tick(&[Default::default(), PlayerTick::buttons(keys::A)], Default::default());
            continue;
        }
        b.tick(&Default::default(), Default::default());
        for c in b.sound_cues() {
            if let nettai_battle::sound::SoundCue::Effect(id) = c {
                match names.get(id.0 as usize).copied() {
                    Some("cross-change") if telop.is_none() => landed = Some(t),
                    Some("telop") if telop.is_none() => telop = Some(t),
                    _ => {}
                }
            }
        }
        controllers = controllers.max(count(&b, "d-hero/controller"));
        navis = navis.max(count(&b, "d-hero/navi"));
        if controllers > 0 && count(&b, "d-hero/controller") == 0 {
            break;
        }
    }
    assert_eq!(b.party[0].active, 1, "ProtoMan came in");
    assert_eq!(telop.zip(landed).map(|(a, b)| a - b), Some(58), "the telop after the follow-up's landing");
    assert_eq!((controllers, navis), (1, 2), "one controller, two navis");
    assert_eq!(before - hp(&b, 1), 7 * 40, "seven volleys of 40");
}
