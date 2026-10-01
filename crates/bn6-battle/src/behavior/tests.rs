//! The content scripts' tests, on the hand-authored test content (whose
//! scripts are this repository's BN6 scripts): what the content registers,
//! and the Luau runtime's rules (no state kept in the VM, no
//! nondeterminism, integers only, bounded work).

use crate::battle::Battle;
use crate::behavior::{Behaviors, Options, with_runtime};
use crate::content::{Content, testing};
use crate::scenario;

/// The duel's battle at its start.
fn battle() -> Battle {
    Battle::new(scenario::setup(), scenario::content())
}

/// Each tick's state digest over a tape.
fn digests(tape: &[scenario::Tick], mut b: Battle) -> Vec<u64> {
    tape.iter()
        .map(|t| {
            b.tick(&t.input, t.events.clone());
            b.digest()
        })
        .collect()
}

#[test]
fn battles_run_the_content_scripts() {
    let b = Battle::new(scenario::setup(), scenario::content());
    assert_eq!(Behaviors::for_content(&b.content).unwrap().runtime(), "luau");
    let kinds: Vec<&str> = b
        .content
        .defs
        .kinds
        .iter()
        .filter(|k| matches!(k.implementation, crate::content::KindImpl::Script { .. }))
        .map(|k| k.key.as_str())
        .collect();
    assert_eq!(
        kinds,
        [
            "absorbed-obstacle",
            "airraid/controller",
            "airraid/plane",
            "airraid/propeller",
            "aqua-surge",
            "attachment",
            "barrier-visual",
            "barriers/controller",
            "bass",
            "blast-fire",
            "blast-man",
            "blkbomb/bomb",
            "bomb",
            "bomb-slash",
            "boomerang",
            "boulder",
            "bugbomb/bomb",
            "bugfix/controller",
            "bugfix/glow",
            "charge-car",
            "charge-man",
            "charge-wave",
            "chargeman/volcano-rock",
            "colorpt/controller",
            "colorpt/point",
            "countdown-bomb",
            "crakshot/shot",
            "dash-hit",
            "dolthdr/doll",
            "dragon-body",
            "dragon-head",
            "drill",
            "dust-ball",
            "dustman/cloud",
            "dustman/overlay",
            "elec-man",
            "elec-thunder",
            "elem-trap",
            "elem-trap-strike",
            "element-pillar",
            "elmnt-bolt",
            "elmnt-ice",
            "elmnt-man",
            "elmnt-vine",
            "energbom/burst",
            "erase-drop",
            "erase-ray",
            "eraseman/beam",
            "eraseman/mark",
            "eraseman/navi",
            "falling-rock",
            "falling-rock/chip",
            "firehit/fist",
            "flame",
            "flmhook/fire",
            "flmhook/hook",
            "flshbom/bomb",
            "flying-shot",
            "follow-effect",
            "gauge-speed",
            "golmhit/golem",
            "grab/controller",
            "grab/shot",
            "grndman/drill",
            "grndman/rock",
            "gundels/beam",
            "gust",
            "heat-man",
            "heatman/flame",
            "hit-flash",
            "instrument",
            "instruments/controller",
            "invisible",
            "junk-shot",
            "justcone/strike",
            "lance/lance",
            "land-mine",
            "lilbolr/boiler",
            "lilbolr/layer",
            "lunge-slash",
            "meteor",
            "mine",
            "moon-beam",
            "navi-boost",
            "panel-bursts",
            "panel-changer",
            "panel-chips/controller",
            "panel-strike",
            "projectile",
            "rflectr/shield",
            "rflectr/shot",
            "rising-bubble",
            "rock",
            "rock/debris",
            "rockcube/cube",
            "rskyhny/bee",
            "sandwrm/hole",
            "sandwrm/spray",
            "sandwrm/worm",
            "seed",
            "sensor/controller",
            "sensor/laser",
            "sensor/scanner",
            "sensor/turret",
            "slash-man",
            "slash-wave",
            "slashman/riding-hit",
            "spout-ball",
            "spout-geyser",
            "spout-man",
            "spout-mark",
            "spout-pillar",
            "spout-splash",
            "spoutman/drip-shower",
            "sumnblk/controller",
            "sumnblk/navi",
            "sun-meteor",
            "sun-moon",
            "sword-wave",
            "tengu-man",
            "tenguman/tornado",
            "thunder-column",
            "time-bom",
            "tomahawk-man",
            "tomahawkman/axe",
            "tomahawkman/strike",
            "trap-chip",
            "vdoll/curse",
            "vdoll/doll",
            "vdoll/sparkles",
            "whirlwind",
        ]
    );
    assert!(b.content.defs.action_numbered(0x37).is_some(), "GunDelSol is a script");
    assert!(b.content.defs.action_numbered(0x10).is_none(), "the step is the engine's");
}

#[test]
fn the_duel_fires_scripted_gun_del_sols() {
    // The tape's players fire their level-3 GunDelSols: the scripted gun
    // and beam appear, and the hits land.
    let tape = scenario::record(900);
    let mut b = Battle::new(scenario::setup(), scenario::content());
    let (mut beams, mut guns) = (0, 0);
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            match b.kind_key(r) {
                "gundels/beam" => beams += 1,
                "attachment" => guns += 1,
                _ => {}
            }
        }
    }
    assert!(beams > 0 && guns > 0, "{beams} beam and {guns} gun object-ticks");
    let hp: Vec<u16> = (0..2).map(|s| b.objects.get(b.player(s).unwrap()).hp).collect();
    assert!(hp.iter().any(|&h| h < 1000), "someone got hit: {hp:?}");
}

/// The eraser navi chip, AreaGrab and PanelGrab (dimming chips content
/// defines) and GunDelSols, by handle.
fn navi_and_dimming_chips() -> Vec<bn6_content_api::ChipHandle> {
    vec![
        testing::chip_handle(testing::ERASER),
        testing::defined_chip(testing::AREA_GRAB),
        testing::defined_chip(testing::PANEL_GRAB),
        testing::chip_handle(testing::SUN_GUN_3),
    ]
}

/// A duel with the eraser navi chip, the grab dimming chips and GunDelSols
/// in the folders: the ticks each scripted kind was on the field, by
/// key.
fn chip_duel(ticks: usize) -> std::collections::BTreeMap<String, usize> {
    duel_with(&navi_and_dimming_chips(), ticks, 11)
}

/// A duel with `chips` in the folders and the players' moves from `seed`:
/// the ticks each kind was on the field, by key.
fn duel_with(chips: &[bn6_content_api::ChipHandle], ticks: usize, seed: u32) -> std::collections::BTreeMap<String, usize> {
    let setup = || scenario::setup_with_handles(chips);
    let tape = scenario::record_on(setup(), ticks, seed);
    let mut b = Battle::new(setup(), scenario::content());
    let mut seen = std::collections::BTreeMap::new();
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            *seen.entry(b.kind_key(r).to_string()).or_insert(0) += 1;
        }
    }
    seen
}

#[test]
fn the_scripted_navi_and_dimming_chips_play() {
    
    let seen = chip_duel(2400);
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    // The eraser navi comes, marks its aim and slashes along it.
    assert!(ticks("eraseman/navi") > 0, "EraseMan: {seen:?}");
    assert!(ticks("eraseman/mark") > 0, "EraseMan's marks: {seen:?}");
    assert!(ticks("eraseman/beam") > 0, "EraseMan's slash: {seen:?}");
    // The grabs' controller drops grab shots.
    assert!(ticks("grab/controller") > 0, "the grabs' controller: {seen:?}");
    assert!(ticks("grab/shot") > 0, "grab shots: {seen:?}");
}

/// The duel's round with the bee and dragon chips (RskyHny2 and ElecDrgn,
/// which content defines) in the folders, in turn, both in the code they
/// share (V), so a hand takes both.
fn bee_and_dragon_setup() -> crate::setup::RoundSetup {
    use crate::content::ChipCode;
    use crate::custom::{BattleFolder, FolderChip};
    let chips = [testing::defined_chip(testing::BEES), testing::defined_chip(testing::DRAGON)];
    let code = ChipCode::from_letter('V').unwrap();
    let content = testing::content();
    let mut folder = BattleFolder::empty();
    for (slot, &h) in folder.chips.iter_mut().zip(chips.iter().cycle()) {
        assert!(content.chip(h).codes.contains(&code), "chip {h:?} doesn't come in V");
        *slot = Some(FolderChip::new(h, code));
    }
    let mut s = scenario::setup();
    for p in &mut s.players {
        p.folder = Some(folder);
    }
    s
}

/// A duel with the bee and dragon chips in the folders: the ticks each
/// scripted kind was on the field, and the players' HP at the end.
fn bee_and_dragon_duel(ticks: usize) -> (std::collections::BTreeMap<String, usize>, [u16; 2]) {
    let setup = bee_and_dragon_setup;
    let tape = scenario::record_on(setup(), ticks, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let mut seen = std::collections::BTreeMap::new();
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            *seen.entry(b.kind_key(r).to_string()).or_insert(0) += 1;
        }
    }
    let hp = [0, 1].map(|s| b.objects.get(b.player(s).unwrap()).hp);
    (seen, hp)
}

#[test]
fn the_bees_and_dragons_play() {
    
    let (seen, hp) = bee_and_dragon_duel(2400);
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    // The hive in hand, the bees it sends, the dragon's head and body.
    assert!(ticks("attachment") > 0, "the hive: {seen:?}");
    assert!(ticks("rskyhny/bee") > 0, "bees: {seen:?}");
    assert!(ticks("dragon-head") > 0, "dragon heads: {seen:?}");
    assert!(ticks("dragon-body") >= 4 * ticks("dragon-head"), "four body segments a head: {seen:?}");
    assert!(hp.iter().any(|&h| h < 1000), "someone got hit: {hp:?}");
}

#[test]
fn the_bees_and_dragons_roll_back() {
    let setup = bee_and_dragon_setup;
    let tape = scenario::record_on(setup(), 1600, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let whole = digests(&tape, Battle::new(setup(), scenario::content()));
    for (i, t) in tape.iter().enumerate() {
        if i % 89 == 0 {
            let copy = digests(&tape[i..], b.clone());
            assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
    }
}

#[test]
fn the_thrown_chips_play_and_roll_back() {
    // Duels with the bomb, seed, flash bomb and bug bomb in the folders:
    // each thrown kind flies, and a copy of the battle at any tick plays
    // on exactly as the battle does.
    
    let setup = || scenario::setup_with(&[testing::BOMB, testing::SEED, testing::FLASH, testing::BUG]);
    let tape = scenario::record_on(setup(), 2400, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let whole = digests(&tape, Battle::new(setup(), scenario::content()));
    let mut seen = std::collections::BTreeMap::new();
    for (i, t) in tape.iter().enumerate() {
        if i % 131 == 0 {
            let copy = digests(&tape[i..], b.clone());
            assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            *seen.entry(b.kind_key(r).to_string()).or_insert(0) += 1;
        }
    }
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    assert!(ticks("bomb") > 0, "the bomb: {seen:?}");
    assert!(ticks("seed") > 0, "the seed: {seen:?}");
    assert!(ticks("flshbom/bomb") > 0, "the flash bomb: {seen:?}");
    assert!(ticks("bugbomb/bomb") > 0, "the bug bomb: {seen:?}");
}

#[test]
fn the_elements_navi_attacks() {
    
    let seen = duel_with(&[testing::chip_handle(testing::ELEMENTS)], 2400, 11);
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    // The elements navi comes with his overlay and attacks with an
    // element: meteors, ice, bolts or vines.
    assert!(ticks("elmnt-man") > 0, "ElmntMan: {seen:?}");
    assert!(ticks("engine/body-overlay") > 0, "ElmntMan's overlay: {seen:?}");
    let attacks = ["meteor", "elmnt-ice", "elmnt-bolt", "elmnt-vine"].map(ticks);
    assert!(attacks.iter().any(|&t| t > 0), "ElmntMan's attacks: {seen:?}");
}

#[test]
fn the_water_navi_attacks() {
    
    let seen = duel_with(&[testing::chip_handle(testing::SPOUT)], 2400, 11);
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    // The water navi comes in his water (his layer) and throws his ball
    // or raises his geyser, which marks its column.
    assert!(ticks("spout-man") > 0, "SpoutMan: {seen:?}");
    assert!(ticks("engine/idle-overlay") > 0, "SpoutMan's layer: {seen:?}");
    let ball = ticks("spout-ball") > 0 && ticks("spout-splash") > 0;
    let geyser = ticks("spout-pillar") > 0 && ticks("spout-geyser") > 0 && ticks("spout-mark") > 0;
    assert!(ball || geyser, "SpoutMan's attacks: {seen:?}");
}

#[test]
fn the_navi_chip_navis_come_and_go() {
    
    // Each navi chip's navi comes (and the duel goes on without a content
    // error); its attacks depend on where the players stand.
    for (chip, navi) in [
        (testing::HEAT, "heat-man"),
        (testing::ELEC, "elec-man"),
        (testing::SLASH, "slash-man"),
        (testing::CHARGE, "charge-man"),
        (testing::TOMAHAWK, "tomahawk-man"),
        (testing::TENGU, "tengu-man"),
        (testing::BLAST, "blast-man"),
    ] {
        let seen = duel_with(&[testing::chip_handle(chip)], 1500, 11);
        assert!(seen.get(navi).copied().unwrap_or(0) > 0, "navi {navi} of chip {chip:#x}: {seen:?}");
    }
}

#[test]
fn the_shooting_and_sun_moon_navis_attack() {
    
    let seen = duel_with(&[testing::chip_handle(testing::BASS)], 2400, 11);
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    // The shooting navi comes with his cape (a form overlay) and fires
    // panel strikes.
    assert!(ticks("bass") > 0, "Bass: {seen:?}");
    assert!(ticks("engine/form-overlay") > 0, "Bass's cape: {seen:?}");
    assert!(ticks("panel-strike") > 0, "Bass's shots: {seen:?}");
    // The sun-and-moon navi throws meteors, shines and dives.
    let seen = duel_with(&[testing::chip_handle(testing::SUN_MOON)], 2400, 11);
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    assert!(ticks("sun-moon") > 0, "SunMoon: {seen:?}");
    assert!(ticks("sun-meteor") > 0, "SunMoon's meteors: {seen:?}");
    assert!(ticks("moon-beam") > 0, "SunMoon's moonlight: {seen:?}");
}

#[test]
fn scripted_chips_roll_back() {
    // A copy of the battle taken at any tick plays on exactly as the
    // battle does: the scripts' state is all in the battle.
    let numbered = |ids: &[crate::content::ChipId]| ids.iter().map(|&id| testing::chip_handle(id)).collect::<Vec<_>>();
    for chips in [
        navi_and_dimming_chips(),
        numbered(&[testing::ELEMENTS]),
        numbered(&[testing::SPOUT]),
        numbered(&[testing::HEAT, testing::ELEC, testing::SLASH, testing::CHARGE, testing::TOMAHAWK, testing::TENGU, testing::BLAST]),
        numbered(&[testing::BASS]),
        numbered(&[testing::SUN_MOON]),
    ] {
        let chips = &chips[..];
        let setup = || scenario::setup_with_handles(chips);
        let tape = scenario::record_on(setup(), 2400, 11);
        let mut b = Battle::new(setup(), scenario::content());
        let whole = digests(&tape, Battle::new(setup(), scenario::content()));
        for (i, t) in tape.iter().enumerate() {
            if i % 97 == 0 {
                let copy = digests(&tape[i..], b.clone());
                assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way ({chips:?})");
            }
            b.tick(&t.input, t.events.clone());
        }
    }
}

/// The standard chips content defines that the test content has (actions
/// that don't fire the buster's projectile), for the folders of a duel.
fn standard_chips() -> Vec<bn6_content_api::ChipHandle> {
    vec![testing::defined_chip(testing::CRAK_SHOT)]
}

/// A duel with the standard chips: the ticks each kind was on the field,
/// by key, and whether a panel was ever broken.
fn standard_duel() -> (std::collections::BTreeMap<String, usize>, bool) {
    let chips = standard_chips();
    let setup = || scenario::setup_with_handles(&chips);
    let tape = scenario::record_on(setup(), 2400, 11);
    let mut b = Battle::new(setup(), scenario::content());
    let mut seen = std::collections::BTreeMap::new();
    let mut broken = false;
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            *seen.entry(b.kind_key(r).to_string()).or_insert(0) += 1;
        }
        broken |= (1..=6).any(|x| (1..=3).any(|y| b.field.panel(x, y).unwrap().kind == crate::field::PanelType::Broken));
    }
    (seen, broken)
}

#[test]
fn the_standard_chips_play() {
    
    let (seen, broken) = standard_duel();
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    // CrakShot digs up the panel ahead and flings it.
    assert!(ticks("crakshot/shot") > 0, "crack shots: {seen:?}");
    assert!(broken, "a dug-up panel is broken");
}

#[test]
fn the_standard_chips_roll_back() {
    let chips = standard_chips();
    let setup = || scenario::setup_with_handles(&chips);
    let tape = scenario::record_on(setup(), 2400, 11);
    let mut b = Battle::new(setup(), scenario::content());
    let whole = digests(&tape, Battle::new(setup(), scenario::content()));
    for (i, t) in tape.iter().enumerate() {
        if i % 97 == 0 {
            let copy = digests(&tape[i..], b.clone());
            assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
    }
}

#[test]
fn the_scripted_swords_play_and_roll_back() {
    // Duels with swords, a step sword and the strike at stunned navis in
    // the folders: the navis swing (actions 0x13 and 0x49) holding their
    // blades, the slashes show, the step sword leaves afterimages, and a
    // copy taken at any tick plays on as the battle does.
    
    let setup = || scenario::setup_with(&[testing::BLADE, testing::STEP_BLADE, testing::STUN_BLADE]);
    let tape = scenario::record_on(setup(), 2400, 11);
    let whole = digests(&tape, Battle::new(setup(), scenario::content()));
    let mut b = Battle::new(setup(), scenario::content());
    let mut seen = std::collections::BTreeMap::new();
    for (i, t) in tape.iter().enumerate() {
        if i % 101 == 0 {
            let copy = digests(&tape[i..], b.clone());
            assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            let key = match b.kind_key(r) {
                "engine/player" => {
                    format!("engine/player in action {:#04x}", crate::kinds::player::navi_action(&b, r).number(&b.content.defs))
                }
                k => k.to_string(),
            };
            *seen.entry(key).or_insert(0) += 1;
        }
    }
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    assert!(ticks("engine/player in action 0x13") > 0, "a sword swung: {seen:?}");
    assert!(ticks("engine/player in action 0x49") > 0, "a strike swung: {seen:?}");
    assert!(ticks("attachment") > 0, "a blade: {seen:?}");
    assert!(ticks("engine/effect") > 0, "a slash: {seen:?}");
    assert!(ticks("engine/afterimage") > 0, "a step sword's afterimages: {seen:?}");
}

/// Two link navis with some of the link navis' own chips (`LINK_CHIPS`'
/// entries `chips`) in their folders.
fn link_chip_setup(chips: &[usize]) -> crate::setup::RoundSetup {
    let chips: Vec<_> = chips.iter().map(|&i| testing::chip_handle(testing::LINK_CHIPS[i].0)).collect();
    let mut s = scenario::setup_with_handles(&chips);
    let navi = testing::content().navi_numbered(testing::LINK_NAVI);
    for stats in &mut s.navi_stats {
        stats.navi = navi;
    }
    s
}

#[test]
fn the_link_navis_chips_play_and_roll_back() {
    // Each link navi chip's record runs the action its module exports (its
    // CurAction the content action's): every one runs, with what it
    // spawns, and a copy taken at any tick plays on as the battle does.
    let mut seen = std::collections::BTreeMap::new();
    for chips in [&[0, 1, 2][..], &[3, 4, 5], &[6, 7, 8], &[9]] {
        let setup = || link_chip_setup(chips);
        let tape = scenario::record_on(setup(), 2000, 11);
        let whole = digests(&tape, Battle::new(setup(), scenario::content()));
        let mut b = Battle::new(setup(), scenario::content());
        for (i, t) in tape.iter().enumerate() {
            if i % 211 == 0 {
                let copy = digests(&tape[i..], b.clone());
                assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way ({chips:?})");
            }
            b.tick(&t.input, t.events.clone());
            for r in b.objects.in_order() {
                let key = match crate::kinds::player::running_content_action(&b, r) {
                    Some(h) => format!("action {}", b.content.defs.action(h).key),
                    None => b.kind_key(r).to_string(),
                };
                *seen.entry(key).or_insert(0) += 1;
            }
        }
    }
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    for chip in
        ["heatpres", "delecswd", "rslash", "edeletbm", "volcchrg", "dripshwr", "etomahwk", "ftornado", "rc-brakr", "dustbrk"]
    {
        assert!(ticks(&format!("action {chip}/action")) > 0, "{chip} ran: {seen:?}");
    }
    for kind in [
        "heatman/flame",
        "follow-effect",
        "slashman/riding-hit",
        "eraseman/beam",
        "chargeman/volcano-rock",
        "tomahawkman/axe",
        "tomahawkman/strike",
        "grndman/drill",
        "dustman/cloud",
        "dustman/overlay",
    ] {
        assert!(ticks(kind) > 0, "{kind}: {seen:?}");
    }
}

#[test]
fn registrations_follow_the_content_data() {
    let c = testing::build();
    let d = &c.defs;
    // The four SunGun chips share one action, as the thrown chips and the
    // three swords share theirs; the v1 weapons have theirs (the buster's,
    // the charged shot's and the blank shot's are definitions). (The mend,
    // mirror, bee and dragon chips are definitions.)
    let mut actions: Vec<u8> = d.actions.iter().filter_map(|a| a.number).collect();
    actions.sort();
    let expected = [
        0x12, 0x13, 0x1A, 0x1D, 0x1E, 0x35, 0x37, 0x3A, 0x3C, 0x3D, 0x41, 0x45, 0x46, 0x49, 0x4A, 0x4C, 0x4D, 0x4E, 0x4F,
        0x50, 0x52, 0x56, 0x57, 0x58,
    ];
    assert_eq!(actions, expected, "{:?}", d.actions);
    // An instant chip's record resolves its subtype's effect (the plus
    // chips' records, the shim's), and a weapon that names an effect no
    // chip has (TenguCross's wind) has its own.
    let plus = d.chip(c.chip_numbered(testing::PLUS).unwrap());
    assert!(matches!(plus.usage, crate::content::ChipUsage::Instant(_)), "{:?}", plus.usage);
    assert!(d.weapon(c.weapon_numbered(0x10)).instant.is_some());
    // A record whose module exports an action runs it, whatever its
    // action number names (the link navis' chips: action 0x0A).
    for (id, _, module) in testing::LINK_CHIPS {
        let chip = d.chip(c.chip_numbered(id).unwrap());
        assert_eq!(chip.record.action, 0x0A);
        match chip.usage {
            crate::content::ChipUsage::Action(h) => assert!(d.action(h).number.is_none(), "{module}: {:?}", d.action(h)),
            ref other => panic!("{module}: {other:?}"),
        }
    }
    // Handles number each registry in key order: the engine's kinds and
    // the content's together.
    assert!(d.kinds.windows(2).all(|w| w[0].key < w[1].key));
    // The engine's kinds have no object slot (the validator has theirs).
    let h = d.kind_by_key("engine/hitbox").unwrap();
    assert_eq!(d.kind(h).slot, None);
    assert_eq!(d.kind_at(crate::object::Pool::Attack, 3), None);
    // Two chips implementing one action with different scripts is an error.
    let mut c = testing::build();
    c.chips[testing::SUN_GUN_2 as usize].script = Some("chips/009-mend/chip".into());
    let e = c.define().unwrap_err().message;
    assert!(e.contains("implements action 0x37"), "{e}");
    // So is naming a script the pack doesn't have.
    let mut c = testing::build();
    c.objects.kinds[0].script = "objects/nowhere".into();
    let e = c.define().unwrap_err().message;
    assert!(e.contains("isn't in the pack"), "{e}");
    // And a kind two registrations fill.
    let mut c = testing::build();
    c.objects.kinds[1].index = c.objects.kinds[0].index;
    c.objects.kinds[1].pool = c.objects.kinds[0].pool;
    let e = c.define().unwrap_err().message;
    assert!(e.contains("both fill"), "{e}");
}

#[test]
fn scripted_instant_chips_play_and_roll_back() {
    // Folders of instant chips (the gauge filler, a plus chip, BusterUp)
    // with GunDelSols: the plus chip's sparkle shows, and a copy of the
    // battle taken at any tick plays on as the battle does.
    let chips = [
        testing::defined_chip(testing::FULL_CUST),
        testing::chip_handle(testing::PLUS),
        testing::defined_chip(testing::BUSTER_UP),
        testing::chip_handle(testing::SUN_GUN_3),
    ];
    let setup = || scenario::setup_with_handles(&chips);
    let tape = scenario::record_on(setup(), 2400, 13);
    let mut b = Battle::new(setup(), scenario::content());
    let whole = digests(&tape, Battle::new(setup(), scenario::content()));
    let mut sparkles = 0;
    for (i, t) in tape.iter().enumerate() {
        if i % 101 == 0 {
            let copy = digests(&tape[i..], b.clone());
            assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
        sparkles += b.objects.in_order().filter(|&r| b.kind_key(r) == "rising-bubble").count();
    }
    assert!(sparkles > 0, "no plus chip was used");
}

#[test]
fn spawning_instant_chips_play_in_a_duel_and_roll_back() {
    // Folders of instant chips that spawn objects (boomerangs, lances,
    // fists, flame hooks, falling fists, golems): a copy of the battle taken
    // at any tick plays on as the battle does.
    let chips = [
        testing::defined_chip(testing::BOOMER),
        testing::defined_chip(testing::LANCE),
        testing::chip_handle(testing::FIST),
        testing::chip_handle(testing::FLAME_HOOK),
        testing::defined_chip(testing::JUSTICE_ONE),
        testing::defined_chip(testing::GOLEM_HIT),
    ];
    let setup = || scenario::setup_with_handles(&chips);
    let tape = scenario::record_on(setup(), 2400, 17);
    let mut b = Battle::new(setup(), scenario::content());
    let whole = digests(&tape, Battle::new(setup(), scenario::content()));
    let mut spawned = 0;
    for (i, t) in tape.iter().enumerate() {
        if i % 97 == 0 {
            let copy = digests(&tape[i..], b.clone());
            assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
        spawned += b
            .objects
            .in_order()
            .filter(|&r| !b.kind_key(r).starts_with("engine/") && b.kind_key(r) != "attachment")
            .count();
    }
    assert!(spawned > 0, "no instant chip spawned anything");
}

// ---- Dimming chips and rocks ------------------------------------------------------------

/// A duel with the cube, veil and trap dimming chips in side 0's folder
/// (and GunDelSols in side 1's: two sides with dimming chips would cut in
/// on each other).
fn dimming_setup() -> crate::setup::RoundSetup {
    let chips = [
        testing::defined_chip(testing::ROCK_CUBE),
        testing::chip_handle(testing::VEIL),
        testing::chip_handle(testing::TRAP),
    ];
    let mut s = scenario::setup_with_handles(&chips);
    s.players[1] = scenario::setup().players[1];
    s
}

/// `dimming_setup`'s duel: the ticks each object kind was on the field, by
/// key (and invisible navi-ticks under "an invisible navi"), and the
/// battle at the end.
fn dimming_duel(ticks: usize) -> (std::collections::BTreeMap<String, usize>, Battle) {
    let setup = dimming_setup;
    let tape = scenario::record_on(setup(), ticks, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let mut seen = std::collections::BTreeMap::new();
    let mut invisible = 0;
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            *seen.entry(b.kind_key(r).to_string()).or_insert(0) += 1;
        }
        for side in 0..2 {
            let Some(p) = b.player(side) else { continue };
            let c = b.objects.get(p).collision.unwrap();
            if b.collision.get(c).f1 & crate::collision::f1::INVISIBLE != 0 {
                invisible += 1;
            }
        }
    }
    seen.insert("an invisible navi".to_string(), invisible);
    (seen, b)
}

#[test]
fn the_scripted_dimming_chips_and_rocks_play() {
    
    let (seen, _) = dimming_duel(2400);
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    // The cube's controller places rocks, which break into debris.
    assert!(ticks("rockcube/cube") > 0, "the cube's controller: {seen:?}");
    assert!(ticks("rock") > 0, "rocks: {seen:?}");
    // The veil's controller makes its user invisible.
    assert!(ticks("invisible") > 0, "the veil's controller: {seen:?}");
    assert!(ticks("an invisible navi") > 0, "an invisible navi: {seen:?}");
    // The trap's controller runs its hidden telop.
    assert!(ticks("trap-chip") > 0, "the trap's controller: {seen:?}");
}

#[test]
fn scripted_dimming_chips_and_rocks_roll_back() {
    let setup = dimming_setup;
    let tape = scenario::record_on(setup(), 2400, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let whole = digests(&tape, Battle::new(setup(), scenario::content()));
    for (i, t) in tape.iter().enumerate() {
        if i % 89 == 0 {
            let copy = digests(&tape[i..], b.clone());
            assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
    }
}

/// A round on the test content's battle settings `settings` (panel
/// pattern 0x38: columns 1-3 are side 0's), not a link battle.
fn rock_battle() -> Battle {
    use crate::setup::NaviStats;
    // An all-zero stats block: support navis byte 0 (none of them, but not
    // 0xFF), everything else zero.
    let stats = NaviStats { support: Some(Default::default()), ..Default::default() };
    let mut setup = testing::round_setup(testing::ROCK_BATTLE, stats);
    setup.settings.effects = 0;
    Battle::new(setup, testing::content())
}

/// Step the objects of the given kinds, in list order and with the game's
/// pause and dimming gating (as `Battle::run_objects`).
fn run_only(b: &mut Battle, kinds: &[&str]) {
    use crate::object::flags;
    let mut cur = b.objects.loop_first();
    while let Some(r) = cur {
        let o = b.objects.get(r);
        let gated = (b.paused && o.flags & flags::RUN_WHILE_PAUSED == 0)
            || (b.is_dimmed() && o.flags & flags::RUN_WHILE_DIMMED == 0);
        if !gated && kinds.contains(&b.kind_key(r)) {
            crate::kinds::update(b, r);
        }
        cur = b.objects.loop_next();
    }
}

#[test]
fn actor_lists_place_scripted_rocks_outside_the_navi_bookkeeping() {
    use crate::object::Pool;
    use crate::setup::ActorKind;
    let mut b = rock_battle();
    let list = b.content.rules.stages.actor_list(testing::NAVIS_AND_ROCKS).clone();
    assert_eq!(
        list.entries.iter().map(|e| e.kind).collect::<Vec<_>>(),
        [ActorKind::Navi, ActorKind::Navi, ActorKind::Rock { variant: 1 }, ActorKind::Rock { variant: 1 }]
    );
    b.spawn_actors();
    assert_eq!(b.round.alive, [1, 1]);
    assert_eq!(b.round.name_counts, [1, 1]);
    let rocks: Vec<_> = b.objects.in_order().filter(|r| r.pool == Pool::Attack).collect();
    assert_eq!(rocks.len(), 2);
    let o = b.objects.get(rocks[0]);
    assert_eq!(b.kind_key(rocks[0]), "rock");
    assert_eq!((o.damage, o.stamina), (200, 0), "the stage's rocks hit with 200 when thrown");
    // Registered on their panels' sides: (3,3) is side 0's, (4,1) side 1's.
    assert_eq!(b.field.objects.slots[0], Some(rocks[0]));
    assert_eq!(b.field.objects.slots[3], Some(rocks[1]));
}

/// A rock broken by damage throws two debris chunks (a jitter draw each,
/// then two draws in each chunk's init) and a dust effect.
#[test]
fn breaking_a_scripted_rock_throws_debris() {
    use crate::object::{Pool, state};
    const ROCK_KINDS: [&str; 3] = ["rock", "rock/debris", "engine/effect"];
    let mut b = rock_battle();
    b.spawn_actors();
    let r = b.objects.in_order().find(|r| r.pool == Pool::Attack).unwrap();
    // A fight in progress (with nobody alive the battle is over, and
    // obstacles break).
    b.round.flags |= crate::battle::battle_flags::FIGHTING;
    run_only(&mut b, &ROCK_KINDS);
    assert_eq!(b.objects.get(r).action, 8, "placed rocks stand at once");
    assert_eq!(b.objects.get(r).hp, 200, "the stages' rock cubes have RockCube's HP");
    let before = b.rng.state;
    b.objects.get_mut(r).hp = 0;
    run_only(&mut b, &ROCK_KINDS);
    let mut rng = crate::rng::Rng::new(before);
    for _ in 0..6 {
        rng.next();
    }
    assert_eq!(b.rng.state, rng.state);
    let order: Vec<_> = b.objects.in_order().filter(|o| o.pool != Pool::Actor).map(|o| b.kind_key(o)).collect();
    assert_eq!(
        &order[..4],
        ["rock", "engine/effect", "rock/debris", "rock/debris"],
        "the rock, then what it spawned in reverse order"
    );
    assert_eq!(b.objects.get(r).state, state::DESTROY);
    assert_eq!(b.field.objects.slots[0], None);
    run_only(&mut b, &ROCK_KINDS);
    assert!(!b.objects.is_allocated(r));
}

/// A stage's boulders (the actor lists' entry type 3) take the field's two
/// stage slots, on their panels' sides, with the header flags their
/// spawner's bug leaves; they stand once the fight is on, and break into
/// two debris chunks and dust, leaving their slot.
#[test]
fn actor_lists_place_boulders_in_the_stage_slots() {
    use crate::object::{PanelPos, Pool, flags, state};
    use crate::setup::NaviStats;
    const BOULDER_KINDS: [&str; 3] = ["boulder", "rock/debris", "engine/effect"];
    let stats = NaviStats { support: Some(Default::default()), ..Default::default() };
    let mut setup = testing::round_setup(testing::BOULDER_BATTLE, stats);
    setup.settings.effects = 0;
    let mut b = Battle::new(setup, testing::content());
    b.spawn_actors();
    assert_eq!(b.round.alive, [1, 1]);
    let boulders: Vec<_> = b.objects.in_order().filter(|r| r.pool == Pool::Attack).collect();
    assert_eq!(boulders.len(), 2, "the field has two stage slots: the list's third boulder isn't placed");
    assert!(boulders.iter().all(|&r| b.kind_key(r) == "boulder"));
    let panels: Vec<_> = boulders.iter().map(|&r| b.objects.get(r).panel).collect();
    assert_eq!(panels, [PanelPos { x: 2, y: 2 }, PanelPos { x: 5, y: 2 }]);
    // Their panels' sides, and the registry's stage slots (not the sides').
    assert_eq!(boulders.iter().map(|&r| b.objects.get(r).alliance).collect::<Vec<_>>(), [0, 1]);
    assert_eq!(b.field.objects.slots, [None, None, None, None, None, None, Some(boulders[0]), Some(boulders[1])]);
    // The spawner reads the flags through the column (open bus): pause and
    // dimming bits over garbage, and "not in use".
    assert_eq!(boulders.iter().map(|&r| b.objects.get(r).flags).collect::<Vec<_>>(), [0xB4, 0x34]);
    // Before the fight they wait, a pixel back and a pixel down, 500 HP.
    b.paused = true;
    run_only(&mut b, &BOULDER_KINDS);
    run_only(&mut b, &BOULDER_KINDS);
    let o = b.objects.get(boulders[0]);
    assert_eq!((o.hp, o.action, o.pos.z, o.flags), (500, 0, -0x1_0000, 0xB4 | flags::VISIBLE));
    let (_, y) = crate::kinds::player::panel_coordinates(2, 2);
    assert_eq!(o.pos.y, y - 0x1_0000);
    // The fight is on: they stand, and stop running while paused.
    b.paused = false;
    b.round.flags |= crate::battle::battle_flags::FIGHTING;
    run_only(&mut b, &BOULDER_KINDS);
    let o = b.objects.get(boulders[0]);
    assert_eq!((o.action, o.flags), (8, 0xB0 | flags::VISIBLE));
    // Broken: two chunks (a jitter draw each, two draws in each one's
    // init) and dust; it leaves its slot.
    let before = b.rng.state;
    b.objects.get_mut(boulders[0]).hp = 0;
    run_only(&mut b, &BOULDER_KINDS);
    let mut rng = crate::rng::Rng::new(before);
    for _ in 0..6 {
        rng.next();
    }
    assert_eq!(b.rng.state, rng.state);
    let order: Vec<_> = b.objects.in_order().filter(|o| o.pool != Pool::Actor).map(|o| b.kind_key(o)).collect();
    assert_eq!(&order[..4], ["boulder", "engine/effect", "rock/debris", "rock/debris"]);
    assert_eq!(b.objects.get(boulders[0]).state, state::DESTROY);
    assert_eq!(b.field.objects.slots[6..], [None, Some(boulders[1])]);
    run_only(&mut b, &BOULDER_KINDS);
    assert!(!b.objects.is_allocated(boulders[0]));
}

/// A rock picked up and thrown (`sub_8018002`, the request `sub_800F6AC`
/// makes) rises for 32 ticks, shakes for its ticks (a jitter draw each),
/// flies to its target panel (10 ticks from (3,3) to (5,2)) and breaks
/// there with a hit.
#[test]
fn a_thrown_rock_flies_to_its_target_and_breaks() {
    use crate::kinds::obstacle::{f2, obstacle_f1};
    use crate::object::{PanelPos, Pool};
    const ROCK: [&str; 1] = ["rock"];
    let mut b = rock_battle();
    b.spawn_actors();
    b.round.flags |= crate::battle::battle_flags::FIGHTING;
    let r = b.objects.in_order().find(|r| r.pool == Pool::Attack).unwrap();
    run_only(&mut b, &ROCK);
    assert_eq!(b.objects.get(r).panel, PanelPos { x: 3, y: 3 });
    // sub_800F6AC(rock, side 0, (5, 2), 10 ticks of shaking, 60 damage).
    let c = b.objects.get(r).collision.unwrap();
    b.collision.get_mut(c).f2 |= f2::THROWN_BY_0;
    let o = b.objects.get_mut(r);
    (o.slide_dx, o.slide_dy, o.slide_timer, o.damage) = (5, 2, 10, 60);
    run_only(&mut b, &ROCK);
    assert_eq!(b.collision.get(c).f2 & f2::THROWN, 0);
    assert_ne!(b.collision.get(c).f1 & obstacle_f1::CARRIED, 0);
    assert_eq!(b.objects.get(r).alliance, 0);
    for _ in 0..32 {
        run_only(&mut b, &ROCK);
    }
    assert_eq!(b.objects.get(r).pos.z, 0x40_0000, "lifted 64 px");
    run_only(&mut b, &ROCK);
    let before = b.rng.state;
    for _ in 0..10 {
        run_only(&mut b, &ROCK);
    }
    let mut rng = crate::rng::Rng::new(before);
    for _ in 0..10 {
        rng.next();
    }
    assert_eq!(b.rng.state, rng.state, "a jitter draw a shaking tick");
    assert_eq!(b.objects.get(r).future_panel, PanelPos { x: 5, y: 2 });
    for _ in 0..9 {
        run_only(&mut b, &ROCK);
        assert_ne!(b.objects.get(r).action, 2, "still flying");
    }
    run_only(&mut b, &ROCK);
    let o = b.objects.get(r);
    assert_eq!((o.panel, o.hp, o.action), (PanelPos { x: 5, y: 2 }, 0, 2), "landed and breaking");
    let hit = b.objects.in_order().find(|&h| b.kind_key(h) == "engine/hitbox").unwrap();
    let h = b.objects.get(hit);
    assert_eq!((h.panel, h.params, h.damage), (PanelPos { x: 5, y: 2 }, [1, 5, 5, 6], 60));
}

// ---- The navi-changing chips (subtype 38) ----------------------------------------------------

#[test]
fn the_navi_changing_chips_change_the_navi() {
    
    let setup = || {
        let mut s = scenario::setup_with(&[testing::BOOST, testing::ARM]);
        s.players[1] = scenario::setup().players[1];
        s
    };
    let tape = scenario::record_on(setup(), 2400, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let before = b.stats[0].clone();
    let mut controllers = 0;
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        controllers += b.objects.in_order().filter(|&r| b.kind_key(r) == "navi-boost").count();
    }
    assert!(controllers > 0, "the controller ran");
    let after = &b.stats[0];
    assert_eq!((after.rapid, after.charge, after.custom_level), (4, 4, 8), "{before:?}");
    assert!(after.float_shoes && after.air_shoes && after.undershirt);
    assert_eq!(after.weapons.charge_shot, testing::weapon(1), "the arm's charged shot");
    let copy = digests(&tape, Battle::new(setup(), scenario::content()));
    let mut b = Battle::new(setup(), scenario::content());
    for (i, t) in tape.iter().enumerate() {
        if i % 131 == 0 {
            assert_eq!(digests(&tape[i..], b.clone()), copy[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
    }
}

// ---- The gauge chips (subtype 25) -----------------------------------------------------------

#[test]
fn the_slow_gauge_chip_slows_the_gauge() {
    let setup = || {
        let mut s = scenario::setup_with(&[testing::SLOW_GAUGE]);
        s.players[1] = scenario::setup().players[1];
        s
    };
    let tape = scenario::record_on(setup(), 2400, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let mut slowed = false;
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        slowed |= b.gauge.rate == 0x10 && b.sides[0].slow_gauge_ticks > 0;
    }
    assert!(slowed, "the gauge slowed");
}

// ---- Luau keeps no state ----------------------------------------------------------------

/// The test content with text replaced in one module (each `(from, to)`
/// once).
fn patched(module: &str, edits: &[(&str, &str)]) -> Content {
    let mut c = testing::build();
    let src = c.scripts.modules.get_mut(module).unwrap_or_else(|| panic!("no module {module}"));
    for (from, to) in edits {
        assert!(src.contains(from), "{module}.luau has no {from:?}");
        *src = src.replacen(from, to, 1);
    }
    c
}

fn load(content: &Content) -> Result<Behaviors, String> {
    Behaviors::load(content, Options::default()).map_err(|e| e.to_string())
}

/// Play the duel with `behaviors`: the content error it stopped on, if
/// any.
fn play_error(behaviors: Behaviors) -> Option<String> {
    let tape = scenario::record(500);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scenario::play(&tape, behaviors)));
    r.err().map(|e| e.downcast_ref::<String>().cloned().unwrap_or_default())
}

const GUN_DEL_SOL: &str = "chips/001-sungun1/chip";
const UPDATE: &str = "function by_number.update(me: Object, s: any)\n";

/// GunDelSol's update (the numbered registration's, which runs the chip's
/// own) with `line` added at its top.
fn in_update(line: &str) -> Content {
    patched(GUN_DEL_SOL, &[(UPDATE, &format!("{UPDATE}    {line}\n"))])
}

#[test]
fn state_in_module_locals_is_rejected_at_load() {
    let c = patched(
        "chips/gundels/beam",
        &[("local beam = {}", "local hums = 0\nlocal beam = {}"), ("    s.ticks += 1\n", "    s.ticks += 1\n    hums += 1\n")],
    );
    let e = load(&c).err().expect("rejected");
    assert!(e.contains("assigns the module-level local `hums`"), "{e}");
}

#[test]
fn global_writes_are_rejected_at_load() {
    let e = load(&in_update("last_user = me")).err().expect("rejected");
    assert!(e.contains("assigns a global"), "{e}");
}

#[test]
fn module_tables_and_data_are_frozen() {
    for line in ["by_number.uses = me.step", "data.chips[me.chip].gun_del_sol.firing_ticks = 1", "math.floor = math.ceil"] {
        let e = play_error(load(&in_update(line)).unwrap()).expect("the write fails");
        assert!(e.contains("readonly"), "{line}: {e}");
    }
}

#[test]
fn tables_captured_by_functions_are_frozen() {
    let c = patched(
        GUN_DEL_SOL,
        &[("local by_number = {", "local seen = {}\nlocal by_number = {"), (UPDATE, &format!("{UPDATE}    seen[1] = me.step\n"))],
    );
    let e = play_error(load(&c).unwrap()).expect("the write fails");
    assert!(e.contains("readonly"), "{e}");
}

#[test]
fn nondeterministic_libraries_are_absent() {
    for call in ["math.random()", "math.sin(1)", "os.time()", "collectgarbage()", "coroutine.create(print)", "buffer.create(4)"]
    {
        let e = play_error(load(&in_update(&format!("local _ = {call}"))).unwrap())
            .unwrap_or_else(|| panic!("{call} ran"));
        assert!(e.contains("attempt to"), "{call}: {e}");
    }
}

#[test]
fn weak_tables_are_refused() {
    let b = load(&in_update("local _ = setmetatable({}, { __mode = \"k\" })")).unwrap();
    assert!(play_error(b).expect("refused").contains("weak tables"));
}

#[test]
fn fractions_cannot_enter_battle_state() {
    let b = load(&patched(
        "chips/gundels/action",
        &[("            s.timer = WIND_UP_TICKS\n", "            s.timer = 13 / 2\n")],
    ))
    .unwrap();
    let e = play_error(b).expect("refused");
    assert!(e.contains("6.5 is not an integer"), "{e}");
}

#[test]
fn runaway_scripts_stop() {
    let b = load(&in_update("while true do end")).unwrap();
    assert!(play_error(b).expect("stopped").contains("past its budget"));
}

#[test]
fn content_errors_name_the_script() {
    let b = load(&in_update("error(\"boom\")")).unwrap();
    let e = play_error(b).expect("stopped");
    assert!(e.contains("action v1/action-37 (chips/001-sungun1/chip.luau's update)") && e.contains("boom"), "{e}");
}

#[test]
fn the_vm_is_not_part_of_the_battle() {
    let tape = scenario::record(900);
    let shared = load(&testing::build()).unwrap();
    let want = with_runtime(&shared, || digests(&tape, battle()));
    // Halfway, move the battle to a fresh VM, as restoring a snapshot on
    // another machine would.
    let mut b = battle();
    let fresh = load(&testing::build()).unwrap();
    let mut have = Vec::new();
    for (i, t) in tape.iter().enumerate() {
        let vm = if i < 450 { &shared } else { &fresh };
        with_runtime(vm, || b.tick(&t.input, t.events.clone()));
        have.push(b.digest());
    }
    assert_eq!(have, want, "a fresh VM continues the battle identically");
    // Interleave a second battle, playing a different tape, on the same VM.
    let other = scenario::record_seeded(900, 11);
    let (mut a, mut c) = (battle(), battle());
    let mut have = Vec::new();
    for (t, u) in tape.iter().zip(other.iter()) {
        with_runtime(&shared, || {
            a.tick(&t.input, t.events.clone());
            have.push(a.digest());
            c.tick(&[u.input[1].clone(), u.input[0].clone()], u.events.clone());
        });
    }
    let first = have.iter().zip(&want).position(|(a, b)| a != b);
    assert_eq!(first, None, "another battle on the same VM changes nothing");
}

/// An engine panic inside a script's API call unwinds through the Luau VM;
/// the VM stays sound for every other battle using it.
#[test]
fn a_panic_inside_content_leaves_the_vm_sound() {
    let tape = scenario::record(900);
    let shared = load(&testing::build()).unwrap();
    let want = with_runtime(&shared, || digests(&tape, battle()));
    let other = scenario::record_seeded(900, 11);
    let (mut a, mut c) = (battle(), battle());
    let mut have = Vec::new();
    let mut panics = 0;
    for (t, u) in tape.iter().zip(&other) {
        with_runtime(&shared, || a.tick(&t.input, t.events.clone()));
        have.push(a.digest());
        // A Rust panic inside a Luau call: the reactive abort GunDelSol
        // calls isn't ported and panics when a defense triggered.
        if let Some(p) = c.player(0)
            && crate::kinds::player::navi_action(&c, p).number(&c.content.defs) == 0x37
        {
            let actor = c.objects.get(p).actor.unwrap();
            c.actors.get_mut(actor).requests |= crate::actor::request::ANTI_SWORD_TRIGGERED;
        }
        let tick = || with_runtime(&shared, || c.tick(&u.input, u.events.clone()));
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(tick)).is_err() {
            panics += 1;
        }
    }
    let first = have.iter().zip(&want).position(|(x, y)| x != y);
    assert!(panics > 0, "the other battle should have panicked inside GunDelSol");
    assert_eq!(first, None, "a battle sharing the VM with {panics} panics diverged");
}

#[test]
fn gc_timing_does_not_reach_the_battle() {
    let tape = scenario::record(900);
    let want = digests(&tape, Battle::new(scenario::setup(), scenario::content()));
    let options = Options { collect_garbage: true, ..Default::default() };
    let b = Behaviors::load(&testing::build(), options).unwrap();
    let have = with_runtime(&b, || digests(&tape, battle()));
    assert_eq!(have, want);
}

// ---- Dimming chip subtypes 2, 3, 5, 15 and 27 (the panel changes) ---------------------------

/// The field operations their scripts call: `object_breakPanel_dup2`
/// breaks an empty panel and cracks an occupied one,
/// `object_panel_setPoison` poisons a solid one, and a blink
/// (`object_setPanelTypeBlink`) only changes how the panel is drawn, for
/// one tick.
#[test]
fn panels_break_poison_and_blink() {
    use crate::field::{PanelType, pflags};
    let mut b = rock_battle();
    b.field.refresh_all(&b.content, &b.collision);
    let (empty, occupied) = ((2, 1), (2, 2));
    b.field.panels[occupied.1][occupied.0].flags |= pflags::BODY_SIDE0;
    let kind = |b: &Battle, (x, y): (usize, usize)| b.field.panel(x as u8, y as u8).unwrap().kind;
    assert!(b.break_panel(empty.0 as u8, empty.1 as u8));
    assert_eq!(kind(&b, empty), PanelType::Broken);
    assert!(!b.break_panel(empty.0 as u8, empty.1 as u8), "a broken panel isn't solid");
    assert!(b.break_panel(occupied.0 as u8, occupied.1 as u8));
    assert_eq!(kind(&b, occupied), PanelType::Cracked);
    assert!(b.poison_panel(1, 3));
    assert_eq!(kind(&b, (1, 3)), PanelType::Poison);
    assert!(!b.poison_panel(empty.0 as u8, empty.1 as u8));
    b.blink_panel(4, 2, PanelType::Holy, 0);
    let p = b.field.panel(4, 2).unwrap();
    assert_eq!((p.kind, p.blink), (PanelType::Normal, Some((PanelType::Holy, 0))));
    b.field.clear_one_frame_looks();
    assert_eq!(b.field.panel(4, 2).unwrap().blink, None);
}

#[cfg(feature = "luau-jit")]
#[test]
fn native_code_plays_the_duel_like_the_interpreter() {
    if !bn6_luau::native_code_supported() {
        return;
    }
    let tape = scenario::record(900);
    let want = digests(&tape, Battle::new(scenario::setup(), scenario::content()));
    let options = Options { native_code: true, ..Default::default() };
    let b = Behaviors::load(&testing::build(), options).unwrap();
    let have = with_runtime(&b, || digests(&tape, battle()));
    assert_eq!(have, want);
}

// ---- ElemTrap (dimming chip subtype 20), TimeBom (10), Mine (11) ------------------------

fn trap_bomb_mine_setup() -> crate::setup::RoundSetup {
    let mut s = scenario::setup_with(&[testing::ELEM_TRAP, testing::TIME_BOMB, testing::TIME_BOMB_PLUS, testing::MINE]);
    s.players[1] = scenario::setup().players[1];
    s
}

/// A duel with the element trap, the time bombs and the mine in side 0's
/// folder (and GunDelSols in side 1's): after each tick `poke` may reach
/// into the battle; the ticks each object kind was on the field, by (pool,
/// index).
fn trap_bomb_mine_duel(
    ticks: usize,
    mut poke: impl FnMut(&mut Battle),
) -> std::collections::BTreeMap<String, usize> {
    let setup = trap_bomb_mine_setup;
    let tape = scenario::record_on(setup(), ticks, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let mut seen = std::collections::BTreeMap::new();
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        poke(&mut b);
        for r in b.objects.in_order() {
            *seen.entry(b.kind_key(r).to_string()).or_insert(0) += 1;
        }
    }
    seen
}

#[test]
fn the_trap_bomb_and_mine_chips_play() {
    
    let seen = trap_bomb_mine_duel(2400, |_| {});
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    // The element trap waits on the field.
    assert!(ticks("trap-chip") > 0 && ticks("elem-trap") > 0, "the element trap: {seen:?}");
    // The time bombs' controller sets bombs; the mine's lays mines.
    assert!(ticks("time-bom") > 0 && ticks("countdown-bomb") > 0, "the time bombs: {seen:?}");
    assert!(ticks("mine") > 0 && ticks("land-mine") > 0, "the mine: {seen:?}");
}

#[test]
fn a_sprung_element_trap_strikes_back() {
    
    use crate::object::state;
    // Once the trap stands (and the battle isn't dimmed), aqua damage
    // reaches it: its counterattack's dimming strikes, and bursts follow.
    let mut sprung = false;
    let seen = trap_bomb_mine_duel(2400, |b| {
        if sprung || b.is_dimmed() {
            return;
        }
        let trap = b.objects.in_order().find(|&r| b.kind_key(r) == "elem-trap");
        let Some(trap) = trap.filter(|&r| b.objects.get(r).state == state::UPDATE) else { return };
        let c = b.objects.get(trap).collision.unwrap();
        b.collision.get_mut(c).acc.element_damage[2] = 10;
        sprung = true;
    });
    assert!(sprung, "no element trap stood: {seen:?}");
    let ticks = |k: &str| seen.get(k).copied().unwrap_or(0);
    assert!(ticks("elem-trap-strike") > 0, "the counterattack's controller: {seen:?}");
    assert!(ticks("panel-bursts") > 0, "the bursts: {seen:?}");
}

#[test]
fn trap_bomb_and_mine_chips_roll_back() {
    let setup = trap_bomb_mine_setup;
    let tape = scenario::record_on(setup(), 2400, 5);
    let mut b = Battle::new(setup(), scenario::content());
    let whole = digests(&tape, Battle::new(setup(), scenario::content()));
    for (i, t) in tape.iter().enumerate() {
        if i % 83 == 0 {
            let copy = digests(&tape[i..], b.clone());
            assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way");
        }
        b.tick(&t.input, t.events.clone());
    }
}

// ---- Dimming chips of subtypes 4, 5, 9, 13, 26, 27, 28 and 36 ------------------------------

/// The barriers (subtype 4), as content defines them.
const BARRIER_CHIPS: &[&str] = &["barrier", "barr100", "barr200", "bblwrap", "lifeaur"];
/// BugFix (subtype 26).
const BUGFIX_CHIPS: &[&str] = &["bugfix"];
/// The panel chips (subtype 5).
const PANEL_CHIPS: &[&str] = &["pnlretrn", "holypanl", "snctuary", "comingrd", "goingrd"];
/// The instruments (subtype 9).
const INSTRUMENT_CHIPS: &[&str] = &["fanfare", "discord", "timpani", "silence"];
/// AirRaid (subtype 13).
const AIR_RAID_CHIPS: &[&str] = &["airraid1", "airraid2", "airraid3"];
/// ColorPt and DblPoint (subtype 27).
const POINT_CHIPS: &[&str] = &["colorpt", "dblpoint"];
/// Sensor (subtype 28).
const SENSOR_CHIPS: &[&str] = &["sensor1", "sensor2", "sensor3"];
/// SumnBlk (subtype 36).
const SUMMON_CHIPS: &[&str] = &["sumnblk1", "sumnblk2", "sumnblk3"];

fn defined(keys: &[&str]) -> Vec<bn6_content_api::ChipHandle> {
    keys.iter().map(|k| testing::defined_chip(k)).collect()
}

/// The test content with the middle column (4) missing: the panel in
/// front of either navi, once side 0 steps up, is a hole.
fn holed_content() -> std::sync::Arc<Content> {
    let mut c = testing::build();
    for row in &mut c.panel_layouts[0].rows {
        row[3] = crate::field::PanelType::Missing;
    }
    std::sync::Arc::new(c)
}

/// What a duel showed: the ticks each kind was on the field, by key, and
/// whether a panel was ever held by the side it isn't home to.
struct Duel {
    seen: std::collections::BTreeMap<String, usize>,
    panel_changed_hands: bool,
}

impl Duel {
    fn ticks(&self, key: &str) -> usize {
        self.seen.get(key).copied().unwrap_or(0)
    }
}

/// A duel with `chips` in the folders on `content`.
fn duel_on(chips: &[bn6_content_api::ChipHandle], content: std::sync::Arc<Content>, ticks: usize, seed: u32) -> Duel {
    let setup = || {
        let mut s = scenario::setup_with_handles(chips);
        s.content = content.hash();
        s
    };
    let tape = scenario::record_on_content(setup(), content.clone(), ticks, seed);
    let mut b = Battle::new(setup(), content.clone());
    let mut d = Duel { seen: Default::default(), panel_changed_hands: false };
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for r in b.objects.in_order() {
            *d.seen.entry(b.kind_key(r).to_string()).or_insert(0) += 1;
        }
        d.panel_changed_hands |= (1..=6).any(|x| {
            (1..=3).any(|y| b.field.panel(x, y).is_some_and(|p| p.kind != crate::field::PanelType::Missing && p.alliance != p.home))
        });
    }
    d
}

fn duel(keys: &[&str]) -> Duel {
    duel_on(&defined(keys), scenario::content(), 2400, 11)
}

#[test]
fn the_barriers_play() {
    let d = duel(BARRIER_CHIPS);
    assert!(d.ticks("barriers/controller") > 0, "the barriers' controller: {:?}", d.seen);
    assert!(d.ticks("barrier-visual") > 0, "a barrier's visual: {:?}", d.seen);
}

#[test]
fn bugfix_plays() {
    let d = duel(BUGFIX_CHIPS);
    assert!(d.ticks("bugfix/controller") > 0, "BugFix's controller: {:?}", d.seen);
    assert!(d.ticks("bugfix/glow") > 0, "BugFix's glow: {:?}", d.seen);
}

#[test]
fn the_panel_chips_play() {
    let d = duel(PANEL_CHIPS);
    assert!(d.ticks("panel-chips/controller") > 0, "the panel chips' controller: {:?}", d.seen);
    assert!(d.ticks("panel-changer") > 0, "a panel changer: {:?}", d.seen);
}

#[test]
fn the_instruments_play() {
    let d = duel(INSTRUMENT_CHIPS);
    assert!(d.ticks("instruments/controller") > 0, "the instruments' controller: {:?}", d.seen);
    assert!(d.ticks("instrument") > 0, "an instrument: {:?}", d.seen);
}

#[test]
fn air_raid_bombs() {
    let d = duel(AIR_RAID_CHIPS);
    assert!(d.ticks("airraid/plane") > 0, "a plane: {:?}", d.seen);
    assert!(d.ticks("airraid/propeller") > 0, "its propeller: {:?}", d.seen);
    assert!(d.ticks("lilbolr/layer") > 0, "its jet flame: {:?}", d.seen);
    assert!(d.ticks("panel-strike") > 0, "its bombs: {:?}", d.seen);
}

#[test]
fn the_points_give_the_front_column_away() {
    let d = duel(POINT_CHIPS);
    assert!(d.ticks("colorpt/controller") > 0, "the points' controller: {:?}", d.seen);
    assert!(d.ticks("colorpt/point") > 0, "a point: {:?}", d.seen);
    assert!(d.panel_changed_hands, "a point gave its panel to the other side");
}

#[test]
fn sensor_scans() {
    let d = duel(SENSOR_CHIPS);
    assert!(d.ticks("sensor/controller") > 0, "Sensor's controller: {:?}", d.seen);
    assert!(d.ticks("sensor/turret") > 0, "a turret: {:?}", d.seen);
    assert!(d.ticks("sensor/scanner") > 0, "its scanner: {:?}", d.seen);
}

#[test]
fn the_summoned_navi_comes_out_of_a_hole() {
    // Without a hole in front, SumnBlk does nothing; with the middle column
    // missing, the navi comes out of it.
    let plain = duel(SUMMON_CHIPS);
    assert!(plain.ticks("sumnblk/controller") > 0, "SumnBlk was used: {:?}", plain.seen);
    assert_eq!(plain.ticks("sumnblk/navi"), 0, "no hole, no navi: {:?}", plain.seen);
    let holed = duel_on(&defined(SUMMON_CHIPS), holed_content(), 2400, 11);
    assert!(holed.ticks("sumnblk/navi") > 0, "the navi: {:?}", holed.seen);
}

#[test]
fn first_barrier_raises_a_barrier() {
    // The NaviCust FirstBarrier (the navi stat): the role's hook, which
    // the test content fills as BN6's roles do, raises the Barrier chip's
    // barrier, with its visual, as the navi comes in.
    let mut s = scenario::setup();
    s.navi_stats[0].first_barrier = 1;
    let mut b = Battle::new(s, scenario::content());
    for _ in 0..300 {
        b.tick(&Default::default(), Default::default());
    }
    let p = b.player(0).unwrap();
    let c = b.collision.get(b.objects.get(p).collision.unwrap());
    assert_eq!((c.barrier, c.barrier_hp), (1, 10), "the Barrier chip's barrier");
    let a = b.objects.get(p).actor.unwrap();
    assert!(b.actors.get(a).barrier_visual.is_some(), "its visual");
    let other = b.objects.get(b.player(1).unwrap()).collision.unwrap();
    assert_eq!(b.collision.get(other).barrier, 0, "the other side has none");
}

#[test]
fn the_support_dimming_chips_roll_back() {
    // A copy of the battle taken at any tick plays on exactly as the
    // battle does.
    let holed = holed_content();
    for (keys, content) in [
        (BARRIER_CHIPS, scenario::content()),
        (BUGFIX_CHIPS, scenario::content()),
        (PANEL_CHIPS, scenario::content()),
        (INSTRUMENT_CHIPS, scenario::content()),
        (AIR_RAID_CHIPS, scenario::content()),
        (POINT_CHIPS, scenario::content()),
        (SENSOR_CHIPS, scenario::content()),
        (SUMMON_CHIPS, holed.clone()),
    ] {
        let chips = defined(keys);
        let setup = || {
            let mut s = scenario::setup_with_handles(&chips);
            s.content = content.hash();
            s
        };
        let tape = scenario::record_on_content(setup(), content.clone(), 2400, 11);
        let mut b = Battle::new(setup(), content.clone());
        let whole = digests(&tape, Battle::new(setup(), content.clone()));
        for (i, t) in tape.iter().enumerate() {
            if i % 197 == 0 {
                let copy = digests(&tape[i..], b.clone());
                assert_eq!(copy, whole[i..], "the copy from tick {i} went its own way ({keys:?})");
            }
            b.tick(&t.input, t.events.clone());
        }
    }
}
