//! Matches across games (docs/frontend.md §6), on every game's content
//! (`testing::every_game`): BN5's folder rules on a BN5 side (its stock
//! ruleset's folder system, content/bn5/rules/folder), a BN6 side's folder
//! holding BN5 chips under BN6's rules (a mixed folder), and a BN5 side
//! against a BN6 side, each played a few hundred ticks.

use crate::testing::every_game;
use crate::{Match, check_match};
use nettai_battle::content::Content;
use std::sync::Arc;

/// A match file's text: the content's first link battle stage, and the
/// sides `left` and `right` (each a side's table body).
fn match_text(content: &Content, left: &str, right: &str) -> String {
    let stage = &content.defs.stage(crate::link_battle_stages(content)[0]).key;
    format!("[arena]\nstage = \"{stage}\"\n\n[left]\n{left}\n\n[right]\n{right}\n")
}

/// A side's table body: its ruleset and navi, the folder's chips (each
/// "key code"), and `more` lines in the folder's table (`regular = 3`).
fn side(ruleset: &str, navi: &str, chips: &[&str], more: &str) -> String {
    let chips: Vec<String> = chips.iter().map(|c| format!("\"{c}\"")).collect();
    format!("ruleset = \"{ruleset}\"\nnavi = \"{navi}\"\n\n[{{side}}.folder]\nchips = [{}]\n{more}", chips.join(", "))
}

/// The match these sides make, or every problem its checks find.
fn parse(content: &Arc<Content>, left: &str, right: &str) -> Result<Match, Vec<String>> {
    let text = match_text(content, &left.replace("{side}", "left"), &right.replace("{side}", "right"));
    crate::parse(content, &text)
}

/// A folder of Tango's BN5 saves (their first): Standard chips only, four
/// copies at most.
const TANGO_BN5: [&str; 30] = [
    "cannon A", "cannon A", "cannon B", "cannon B", "airshot *", "airshot *", "airshot *",
    "vulcan1 D", "vulcan1 D", "vulcan1 D", "minibomb B", "minibomb B", "minibomb L", "minibomb L",
    "sword S", "sword S", "sword S", "sword S", "wideswrd S", "wideswrd S", "busterup *",
    "crakout *", "crakout *", "recov10 A", "recov10 A", "recov10 L", "recov10 L", "areagrab S",
    "attck-10 *", "attck-10 *",
];

/// A BN6 folder: the live navi's kind of folder, Standard chips.
const BN6: [&str; 30] = [
    "cannon A", "cannon A", "cannon B", "airshot *", "airshot *", "vulcan1 D", "vulcan1 D",
    "minibomb B", "minibomb B", "minibomb L", "sword S", "sword S", "sword S", "wideswrd S",
    "wideswrd S", "recov10 A", "recov10 A", "recov10 L", "areagrab S", "areagrab S", "spreadr1 L",
    "spreadr1 L", "longswrd S", "longswrd S", "crakshot A", "crakshot A", "widesht P", "widesht P",
    "cannon C", "recov30 L",
];

/// A BN5 side by BN5's stock rules, with this folder.
fn bn5(chips: &[&str], more: &str) -> String {
    side("stock", "megaman", chips, more)
}

/// `chips` with entries `at` replaced by `with`.
fn with(chips: &[&str], at: usize, with: &[&'static str]) -> Vec<String> {
    let mut v: Vec<String> = chips.iter().map(|s| s.to_string()).collect();
    for (i, c) in with.iter().enumerate() {
        v[at + i] = c.to_string();
    }
    v
}

fn refs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

/// BN5's MegaMan, on BN5's stock rules, with Tango's BN5 folder: the
/// rules accept it (the Regular chip too), against a BN6 side.
#[test]
fn a_bn5_folder_keeps_bn5s_rules() {
    let content = every_game();
    let right = side("stock", "megaman", &BN6, "");
    parse(&content, &bn5(&TANGO_BN5, ""), &right).unwrap_or_else(|p| panic!("{p:?}"));
    // A Regular chip within the fresh navi's Regular memory (4 MB: CrakOut).
    parse(&content, &bn5(&TANGO_BN5, "regular = 21"), &right).unwrap_or_else(|p| panic!("{p:?}"));
    // Three Mega chips, three dark chips (one of each), one Giga chip.
    let mixed = with(
        &TANGO_BN5,
        0,
        &["blizman-ds B", "cloudmn-ds C", "colonel C", "drksword Z", "darkthnd M", "darkwide T", "bass F"],
    );
    parse(&content, &bn5(&refs(&mixed), ""), &right).unwrap_or_else(|p| panic!("{p:?}"));
}

/// What BN5's folder rules refuse, each said with its rule.
#[test]
fn bn5s_rules_refuse() {
    let content = every_game();
    let right = side("stock", "megaman", &BN6, "");
    let refused = |chips: &[String], more: &str| -> Vec<String> {
        let e = parse(&content, &bn5(&refs(chips), more), &right).expect_err("refused");
        assert!(e.iter().all(|p| p.starts_with("left: folder: ")), "{e:?}");
        e
    };
    let says = |e: &[String], what: &str| assert!(e.iter().any(|p| p.contains(what)), "{what:?} not in {e:?}");
    // A fifth copy of a Standard chip (BN6 would take five 8 MB Cannons).
    let five = with(&TANGO_BN5, 4, &["cannon C"]);
    says(&refused(&five, ""), "5 copies of bn5:cannon (a standard chip), past 4");
    // Two copies of a Mega chip, of a Giga chip, of a dark chip.
    let two = with(&TANGO_BN5, 0, &["colonel C", "colonel C", "bass F", "bass F", "drksword Z", "drksword Z"]);
    let e = refused(&two, "");
    says(&e, "2 copies of bn5:colonel (a mega chip), past 1");
    says(&e, "2 copies of bn5:bass (a giga chip), past 1");
    says(&e, "2 copies of bn5:drksword (a dark chip), past 1");
    says(&e, "2 Giga chips, past the navi's 1");
    // Six Mega chips; four dark chips.
    let six = with(
        &TANGO_BN5,
        0,
        &["colonel C", "meddy M", "gyroman G", "knightmn K", "larkman S", "gridman F"],
    );
    says(&refused(&six, ""), "6 Mega chips, past the navi's 5");
    let four = with(&TANGO_BN5, 0, &["drksword Z", "darkthnd M", "darkwide T", "drklance W"]);
    says(&refused(&four, ""), "4 dark chips, past 3");
    // A chip past the pack (BatCan1), a Program Advance; a code the chip
    // doesn't come in.
    let pack = with(&TANGO_BN5, 0, &["batcan1 *", "lifesrd *"]);
    let e = refused(&pack, "");
    says(&e, "entry 0: bn5:batcan1 is no chip a folder can hold");
    says(&e, "entry 1: bn5:lifesrd is no chip a folder can hold");
    let code = with(&TANGO_BN5, 0, &["cannon Z"]);
    says(&refused(&code, ""), "entry 0: bn5:cannon doesn't come in code Z");
    // A Regular chip past the Regular memory (Cannon is 8 MB; a fresh
    // navi's memory is 4); tag chips, which BN5's folders haven't.
    says(&refused(&with(&TANGO_BN5, 0, &[]), "regular = 0"), "the Regular chip bn5:cannon is 8 MB, past the navi's 4");
    says(&refused(&with(&TANGO_BN5, 0, &[]), "tags = [4, 5]"), "tag chips, but BN5's folders have none");
}

/// A BN6 side's folder of BN5 chips (a mixed folder) is held to BN6's
/// rules, by the chips' records: five 8 MB Cannons of BN5's are BN6's
/// five copies, which BN5's own rules refuse.
#[test]
fn a_mixed_folder_keeps_its_sides_rules() {
    let content = every_game();
    let five = with(&TANGO_BN5, 4, &["cannon C"]);
    let mixed: Vec<String> = refs(&five).iter().take(15).map(|s| s.to_string()).chain(BN6[15..].iter().map(|s| s.to_string())).collect();
    let left = side("stock", "megaman", &refs(&mixed), "");
    let right = bn5(&TANGO_BN5, "");
    parse(&content, &left, &right).unwrap_or_else(|p| panic!("{p:?}"));
    // And BN6's limits: a sixth copy is past BN6's five.
    let six = with(&refs(&mixed), 5, &["cannon *"]);
    let e = parse(&content, &side("stock", "megaman", &refs(&six), ""), &right).expect_err("refused");
    assert!(e.iter().any(|p| p.contains("left: folder: 6 copies of bn5:cannon")), "{e:?}");
}

/// The chips a BN6 side's rules let a folder hold are every game's (a
/// BN5 Cannon among them), and a BN5 side's BN5's own without its BatCans
/// and Program Advances.
#[test]
fn every_games_chips_are_in_the_pool() {
    let content = every_game();
    let m = parse(&content, &side("stock", "megaman", &BN6, ""), &bn5(&TANGO_BN5, "")).unwrap();
    let mut b = crate::check::start(&content, &m).unwrap();
    let key = |h: nettai_content_api::ChipHandle| content.defs.chip(h).key.clone();
    let six: Vec<String> = crate::folders::pool(&content, &mut b, 0).into_iter().map(key).collect();
    let five: Vec<String> = crate::folders::pool(&content, &mut b, 1).into_iter().map(key).collect();
    for k in ["cannon", "cannon", "colonel", "bass"] {
        assert!(six.contains(&k.to_string()) && five.contains(&k.to_string()), "{k}");
    }
    for k in ["batcan1", "lifesrd", "invalid"] {
        assert!(!five.contains(&k.to_string()), "{k}");
    }
}

/// Play a match `ticks` ticks: the scenario's players (picking chips at
/// each custom screen, stepping about, firing and using them) on the
/// match's round. The chips each side used, by key, in order.
fn play(content: &Arc<Content>, m: &Match, ticks: usize) -> [Vec<String>; 2] {
    assert_eq!(check_match(content, m), Vec::<String>::new());
    let setup = m.round(content, 0x5EED);
    let tape = nettai_battle::scenario::record_on_content(setup.clone(), content.clone(), ticks, 11);
    let mut b = nettai_battle::Battle::new(setup, content.clone());
    let mut used: [Vec<String>; 2] = Default::default();
    let mut shown = [None, None];
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for side in 0..2 {
            let now = b.used_chips[side].map(|u| u.chip);
            if now.is_some() && b.used_chips[side].is_some_and(|u| u.ticks > 0) && now != shown[side] {
                used[side].push(content.defs.chip(now.unwrap()).key.clone());
            }
            shown[side] = now;
        }
    }
    used
}

/// A BN6 side with BN5 chips, against BN6's own, and a BN5 side against
/// a BN6 side: each plays 900 ticks (custom screens, chips used, a BN5
/// chip among them) without the engine stopping.
#[test]
fn mixed_and_cross_game_matches_play() {
    let content = every_game();
    let mixed: Vec<String> = TANGO_BN5.iter().take(15).map(|s| s.to_string()).chain(BN6[15..].iter().map(|s| s.to_string())).collect();
    let m = parse(&content, &side("stock", "megaman", &refs(&mixed), ""), &side("stock", "megaman", &BN6, "")).unwrap();
    let used = play(&content, &m, 900);
    assert!(used[0].iter().any(|k| k.starts_with("bn5:")), "the mixed side used a BN5 chip: {used:?}");
    let m = parse(&content, &bn5(&TANGO_BN5, ""), &side("stock", "megaman", &BN6, "")).unwrap();
    let used = play(&content, &m, 900);
    assert!(used[0].iter().any(|k| k.starts_with("bn5:")) && used[1].iter().any(|k| k.starts_with("bn6:")), "{used:?}");
}

/// A BN5 side's karma and souls write to a match file and read back; the
/// karma's default (a fresh save's 500) and an unlisted soul list (every
/// soul) are left out, so old files load unchanged.
#[test]
fn karma_and_souls_write_and_read_back() {
    let content = every_game();
    let left = bn5(&TANGO_BN5, "")
        .replacen("navi = \"megaman\"\n", "navi = \"megaman\"\nkarma = 100\nsouls = [\"protosoul\", \"colonelsoul\"]\n", 1);
    let m = parse(&content, &left, &side("stock", "megaman", &BN6, "")).unwrap_or_else(|p| panic!("{p:?}"));
    let s = &m.sides[0];
    assert_eq!(s.karma, 100);
    let souls = ["protosoul", "colonelsoul"].map(|k| content.defs.form_by_key(k).unwrap());
    assert_eq!(s.souls, Some(souls.to_vec()), "either version's");
    let text = crate::write(&content, &m);
    assert!(text.contains("karma = 100") && text.contains("souls = [") && text.contains("\"colonelsoul\""), "{text}");
    assert_eq!(crate::parse(&content, &text).unwrap(), m);
    // A side that says nothing of them: the defaults, nothing written.
    let plain = parse(&content, &bn5(&TANGO_BN5, ""), &side("stock", "megaman", &BN6, "")).unwrap();
    assert_eq!((plain.sides[0].karma, plain.sides[0].souls.clone()), (500, None));
    let text = crate::write(&content, &plain);
    assert!(!text.contains("karma") && !text.contains("souls"), "{text}");
    // Karma past 1000; karma and a soul list under BN6's rules; a form that
    // is no soul: said.
    let bad = side("stock", "megaman", &BN6, "")
        .replacen("navi = \"megaman\"\n", "navi = \"megaman\"\nkarma = 1200\nsouls = [\"heatcross\"]\n", 1);
    let e = parse(&content, &bn5(&TANGO_BN5, ""), &bad).unwrap_err();
    for p in [
        "right: karma 1200: the light/dark value is 0 to 1000",
        "right: karma, but the ruleset has no light and dark MegaMan (no system takes `karma`)",
        "right: a soul list, but the ruleset has no Soul Unison (no system takes `souls`)",
        "right: HeatCross is no soul",
    ] {
        assert!(e.iter().any(|x| x == p), "{p:?} not in {e:?}");
    }
}

/// A round of `m` after `ticks` ticks of nothing pressed.
fn started(content: &Arc<Content>, m: &Match, ticks: usize) -> nettai_battle::Battle {
    let mut b = nettai_battle::Battle::new(m.round(content, 0x5EED), content.clone());
    for _ in 0..ticks {
        b.tick(&Default::default(), Default::default());
    }
    b
}

/// A dark MegaMan (light/dark value 100) starts with mood 0 and the dark
/// face (the worn-out one: `bn5:megaman-dark`); a fresh save's (500)
/// with mood 128 and his plain face; a light one (1000) at 190.
#[test]
fn a_dark_side_starts_dark() {
    use nettai_battle::kinds::player::Emotion;
    let content = every_game();
    let at = |value: Option<u16>| {
        let mut m = parse(&content, &bn5(&TANGO_BN5, ""), &side("stock", "megaman", &BN6, "")).unwrap();
        if let Some(v) = value {
            m.sides[0].karma = v;
        }
        let b = started(&content, &m, 40);
        (b.stats[0].mood, nettai_battle::kinds::player::emotion(&b, 0))
    };
    assert_eq!(at(Some(100)), (0, Emotion::WornOut));
    assert_eq!(at(None), ((500 / 20 + 103) as u8, Emotion::Normal));
    assert_eq!(at(Some(1000)), (190, Emotion::Normal));
    // The face a mood of 0 shows: the base form's dark one.
    let base = content.base_form_for(content.defs.navi_by_key("megaman").unwrap());
    let face = content.form(base).mugshot.unwrap().of(Emotion::WornOut);
    assert_eq!(content.assets.handle(nettai_content_api::AssetKind::Mugshot, "megaman-dark"), Some(face.0));
}

/// The soul button offers only a soul the side has: with every soul
/// (ProtoSoul among them), a Sword picked offers ProtoSoul; with none, or
/// with GyroSoul alone, the button is there but the sword's soul isn't
/// offered.
#[test]
fn an_unowned_soul_cant_be_chosen() {
    use nettai_battle::custom::screen::{Phase, SPECIAL_SLOT, SlotKind, SlotState};
    let content = every_game();
    // Swords alone, so the first chip dealt is one (no folder the rules
    // take: the round is played as set up).
    let sword = content.defs.chip_by_key("sword").unwrap();
    let offered = |souls: Option<Vec<nettai_content_api::FormHandle>>| {
        let mut m = parse(&content, &bn5(&TANGO_BN5, ""), &side("stock", "megaman", &BN6, "")).unwrap();
        m.sides[0].folder.chips = [Some(nettai_battle::custom::FolderChip::new(sword, nettai_battle::content::ChipCode(18))); 30];
        m.sides[0].souls = souls;
        let mut b = nettai_battle::Battle::new(m.round(&content, 0x5EED), content.clone());
        let mut last = 0u16;
        for _ in 0..400 {
            let screen = b.custom.sides[0].screen.as_ref().filter(|s| s.phase == Phase::Choosing);
            if let Some(s) = screen
                && s.selected == 1
            {
                let slot = s.slots[SPECIAL_SLOT as usize];
                let soul = slot.kind == SlotKind::Soul;
                return (soul, soul && slot.state == SlotState::Selectable);
            }
            let a = if screen.is_some() && last == 0 { nettai_battle::input::keys::A } else { 0 };
            last = a;
            b.tick(&[nettai_battle::input::PlayerTick { held: a }, Default::default()], Default::default());
        }
        panic!("no chip picked");
    };
    assert_eq!(offered(None), (true, true));
    assert_eq!(offered(Some(Vec::new())), (true, false));
    assert_eq!(offered(Some(vec![content.defs.form_by_key("gyrosoul").unwrap()])), (true, false));
    // ProtoSoul alone, or with Team Colonel's ColonelSoul: offered.
    let proto = content.defs.form_by_key("protosoul").unwrap();
    assert_eq!(offered(Some(vec![proto, content.defs.form_by_key("colonelsoul").unwrap()])), (true, true));
}

/// A side's karma and souls go into its round's setup: the light and dark
/// system's block holds the karma, the souls system's the souls, which
/// are the soul button's (by their numbers).
#[test]
fn karma_and_souls_reach_the_round() {
    let content = every_game();
    let mut m = parse(&content, &bn5(&TANGO_BN5, ""), &side("stock", "megaman", &BN6, "")).unwrap();
    m.sides[0].karma = 300;
    m.sides[0].souls = Some(vec![content.defs.form_by_key("colonelsoul").unwrap()]);
    let b = started(&content, &m, 1);
    let (schema, block) = b.system_setup(0, "light-dark").unwrap();
    assert_eq!(block.get(schema, schema.index_of("karma").unwrap()), nettai_content_api::FieldValue::U16(300));
    let souls = b.setup.players[0].souls;
    assert!(souls.button && souls.chaos && souls.owned == 1 << 7, "{souls:?}");
    // A BN6 side has no souls.
    assert_eq!(b.setup.players[1].souls, nettai_battle::custom::SoulUnlocks::default());
}

/// What a side's rules and navi take decides its own fields: a BN6 side
/// (a mixed one too: BN6's rules, BN5 chips) takes its game, a navi code's
/// level and BN6's SP times; a BN5 side takes no game and no level, and
/// BN5's own SP times (its SP navi chips', not BN6's).
#[test]
fn a_sides_fields_are_its_rules() {
    let content = every_game();
    let mixed: Vec<String> = TANGO_BN5.iter().take(15).map(|s| s.to_string()).chain(BN6[15..].iter().map(|s| s.to_string())).collect();
    let m = parse(&content, &side("stock", "megaman", &refs(&mixed), ""), &bn5(&TANGO_BN5, "")).unwrap();
    let (six, five) = (&m.sides[0], &m.sides[1]);
    assert!(six.takes_game(&content) && six.takes_level(&content) && six.takes_sp_times(&content));
    assert!(!five.takes_game(&content) && !five.takes_level(&content) && five.takes_sp_times(&content));
    // Each slot's chip is of the side's rules' game.
    let chip = |s: &crate::Side, slot| crate::facts::sp_chip(&content, s, slot).map(|h| content.defs.chip(h).key.clone());
    assert!(chip(six, 0).is_some_and(|k| k.starts_with("bn6:")), "{:?}", chip(six, 0));
    assert!(chip(five, 1).is_some_and(|k| k.starts_with("bn5:")), "{:?}", chip(five, 1));
}

/// A ruleset change drops what the new rules don't take: BN6's game
/// (back to Falzar), Crosses, patch cards and NaviCust going to BN5's
/// rules, the SP times (another game's SP navis); BN5's karma and souls
/// going to BN6's. A BN6 side that stays on BN6's rules keeps them.
#[test]
fn a_ruleset_change_drops_what_the_rules_dont_take() {
    let content = every_game();
    let bn6 = content.defs.ruleset_by_key("stock");
    let bn5 = content.defs.ruleset_by_key("stock");
    let mut m = crate::draw::live(&content, 3, None).unwrap();
    let s = &mut m.sides[0];
    s.ruleset = bn6;
    s.game = nettai_battle::custom::GameVersion::Gregar;
    s.crosses = crate::navi_crosses(&content, s.navi).map(|c| crate::CrossList::new(&c[..2]));
    assert!(s.crosses.is_some(), "live play's navi changes form");
    s.sp_times.0[0] = 600;
    let kept = s.clone();
    s.set_ruleset(&content, bn6);
    assert_eq!(*s, kept, "the same rules keep everything");
    s.set_ruleset(&content, bn5);
    assert_eq!((s.game, s.crosses, s.navicust, s.cards.len(), s.sp_times.0[0]), (nettai_battle::custom::GameVersion::Falzar, None, None, 0, 0));
    s.karma = 100;
    s.souls = Some(Vec::new());
    s.set_ruleset(&content, bn6);
    assert_eq!((s.karma, s.souls.clone()), (crate::facts::DEFAULT_KARMA, None));
}
