//! Matches of each game (docs/frontend.md §6), each on its game's content
//! alone (`testing::bn5_content`, `bn6_content`): a match is of one game,
//! its arena's, and names nothing of another's. BN5's folder rules on
//! BN5's sides (its stock ruleset's folder system,
//! content/bn5/rules/folder), a BN5 match played a few hundred ticks,
//! BN5's karma and souls, and a name another game has but the match's
//! hasn't refused as any unknown name is.

use crate::testing::{bn5_content, bn6_content};
use crate::{Match, check_match};
use nettai_battle::content::Content;
use std::sync::Arc;

/// A match file's text of `game`: its first link battle stage, and the
/// sides `left` and `right` (each a side's table body).
fn match_text(content: &Content, game: &str, left: &str, right: &str) -> String {
    let stage = crate::ids::local(&content.defs.stage(crate::link_battle_stages(content, game)[0]).key);
    format!("game = \"{game}\"\n\n[arena]\nstage = \"{stage}\"\n\n[left]\n{left}\n\n[right]\n{right}\n")
}

/// A side's table body: its navi, the folder's chips (each "name code"),
/// and `more` lines (`regular = 3`).
fn side(navi: &str, chips: &[&str], more: &str) -> String {
    let chips: Vec<String> = chips
        .iter()
        .map(|c| {
            let (name, code) = c.rsplit_once(' ').unwrap();
            format!("[\"{name}\", \"{code}\"]")
        })
        .collect();
    format!("navi = \"{navi}\"\nfolder = [{}]\n{more}", chips.join(", "))
}

/// The match of `game` these sides make, or every problem its checks find.
fn parse_in(content: &Arc<Content>, game: &str, left: &str, right: &str) -> Result<Match, Vec<String>> {
    crate::parse(content, &match_text(content, game, left, right))
}

/// A BN5 match of these sides.
fn parse(content: &Arc<Content>, left: &str, right: &str) -> Result<Match, Vec<String>> {
    parse_in(content, "bn5", left, right)
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

/// A BN5 MegaMan with this folder.
fn bn5(chips: &[&str], more: &str) -> String {
    side("megaman", chips, more)
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
/// rules accept it (the Regular chip too).
#[test]
fn a_bn5_folder_keeps_bn5s_rules() {
    let content = bn5_content();
    let right = bn5(&TANGO_BN5, "");
    parse(&content, &bn5(&TANGO_BN5, ""), &right).unwrap_or_else(|p| panic!("{p:?}"));
    // A Regular chip within the fresh navi's Regular memory (4 MB: CrakOut).
    parse(&content, &bn5(&TANGO_BN5, "regular = 21"), &right).unwrap_or_else(|p| panic!("{p:?}"));
    // Three Mega chips, three dark chips (one of each), one Giga chip.
    let megas = with(&TANGO_BN5, 0, &["blizman-ds B", "cloudmn-ds C", "colonel C", "drksword Z", "darkthnd M", "darkwide T", "bass F"]);
    parse(&content, &bn5(&refs(&megas), ""), &right).unwrap_or_else(|p| panic!("{p:?}"));
}

/// What BN5's folder rules refuse, each said with its rule.
#[test]
fn bn5s_rules_refuse() {
    let content = bn5_content();
    let right = bn5(&TANGO_BN5, "");
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
    let six = with(&TANGO_BN5, 0, &["colonel C", "meddy M", "gyroman G", "knightmn K", "larkman S", "gridman F"]);
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

/// A name the match's game hasn't is refused as any unknown name is, the
/// same whether another game has it or nothing does: a BN6 Cross, navi
/// or chip in a BN5 match, a BN5 chip in a BN6 match, a qualified name
/// (`bn6:cannon`: no game's name is written so) and a misspelling alike.
#[test]
fn an_unknown_name_is_refused() {
    let content = bn5_content();
    let says = |e: Vec<String>, what: &str| assert!(e.iter().any(|p| p.starts_with(what)), "{what:?} not in {e:?}");
    let ok = bn5(&TANGO_BN5, "");
    // A BN6 navi, Cross and NaviCust program: none of BN5's.
    says(parse(&content, &side("protoman", &TANGO_BN5, ""), &ok).unwrap_err(), "left: no navi \"protoman\" in bn5");
    let crosses = bn5(&TANGO_BN5, "crosses = [\"heatcross\"]");
    says(parse(&content, &crosses, &ok).unwrap_err(), "left: no Cross \"heatcross\" in bn5");
    // A chip of BN6's alone (HeatMan), a qualified name, a misspelling:
    // one error.
    let six = bn6_content();
    assert!(crate::ids::chip(&six, "bn6", "heatman").is_some() && crate::ids::chip(&content, "bn5", "heatman").is_none());
    for name in ["heatman", "bn6:cannon", "bn5:cannon", "canon"] {
        let chips = with(&TANGO_BN5, 0, &[]);
        let mut chips = refs(&chips);
        let entry = format!("{name} A");
        chips[3] = &entry;
        says(parse(&content, &bn5(&chips, ""), &ok).unwrap_err(), &format!("left: folder entry 3: no chip {name:?} in bn5"));
    }
    // A BN5 chip (GyroMan) in a BN6 match.
    assert!(crate::ids::chip(&content, "bn5", "gyroman").is_some() && crate::ids::chip(&six, "bn6", "gyroman").is_none());
    let mut chips = BN6.to_vec();
    chips[0] = "gyroman G";
    let e = parse_in(&six, "bn6", &side("megaman", &chips, ""), &side("megaman", &BN6, "")).unwrap_err();
    says(e, "left: folder entry 0: no chip \"gyroman\" in bn6");
    // A game the content hasn't (BN6's file on BN5's content, a game no
    // content has); a ruleset the game hasn't.
    let text = match_text(&content, "bn5", &ok, &ok);
    let e = crate::parse(&content, &text.replacen("game = \"bn5\"", "game = \"bn7\"", 1)).unwrap_err();
    says(e, "no game \"bn7\" (the content's are bn5)");
    let e = crate::parse(&six, &text).unwrap_err();
    says(e, "no game \"bn5\" (the content's are bn6)");
    let e = crate::parse(&content, &text.replacen("game = \"bn5\"\n", "game = \"bn5\"\nruleset = \"bn6:stock\"\n", 1)).unwrap_err();
    assert!(e[0].starts_with("no ruleset \"bn6:stock\" in bn5"), "{e:?}");
}

/// A match's lookups see only its game: its stages, navis, chips (the
/// rules' pool), souls and patch cards are its own, so a BN5 match's
/// sides, made or drawn, hold BN5's alone and a BN6 match's BN6's.
#[test]
fn a_matchs_lookups_only_see_its_game() {
    for (game, content) in [("bn5", bn5_content()), ("bn6", bn6_content())] {
        let of = |key: &str| crate::ids::in_game(game, key);
        assert!(crate::link_battle_stages(&content, game).iter().all(|&s| of(&content.defs.stage(s).key)));
        assert!(crate::navis(&content, game).iter().all(|&n| of(&content.defs.navi(n).key)));
        assert!(crate::facts::all_souls(&content, game).iter().all(|&f| of(&content.defs.form(f).key)));
        let m = crate::draw::live(&content, game, 5, None).unwrap();
        assert_eq!(check_match(&content, &m), Vec::<String>::new(), "{game}");
        let mut b = crate::check::start(&content, &m).unwrap();
        let pool = crate::folders::pool(&content, game, &mut b, 0);
        assert!(pool.len() > 100 && pool.iter().all(|&c| of(&content.defs.chip(c).key)), "{game}: {} chips", pool.len());
        for s in &m.sides {
            assert!(of(&content.defs.navi(s.navi).key) && s.folder.chips().all(|c| of(&content.defs.chip(c.id).key)));
        }
        assert!(of(&content.defs.ruleset(m.arena.ruleset).key));
        // Written, it names its game once, and nothing else qualified.
        let text = crate::write(&content, &m);
        assert!(text.contains(&format!("game = \"{game}\"")) && !text.contains("bn5:") && !text.contains("bn6:"), "{text}");
        assert_eq!(crate::parse(&content, &text).unwrap(), m);
    }
    // BN5's souls aren't BN6's (BN6 has none); BN6's names are none of
    // BN5's content's.
    assert!(crate::facts::all_souls(&bn6_content(), "bn6").is_empty());
    assert!(!crate::facts::all_souls(&bn5_content(), "bn5").is_empty());
    assert_eq!(crate::ids::form(&bn5_content(), "bn6", "heatcross"), None);
    assert_eq!(crate::ids::games(&bn5_content()), ["bn5"]);
    // A match of another game than the content's makes no match.
    assert!(Match::empty(&bn5_content(), "bn6").is_err());
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

/// A BN5 match plays 900 ticks (custom screens, chips used) without the
/// engine stopping, its sides using BN5's chips.
#[test]
fn a_bn5_match_plays() {
    let content = bn5_content();
    let m = parse(&content, &bn5(&TANGO_BN5, ""), &bn5(&TANGO_BN5, "")).unwrap();
    let used = play(&content, &m, 900);
    assert!(used.iter().all(|u| !u.is_empty() && u.iter().all(|k| k.starts_with("bn5:"))), "{used:?}");
}

/// A BN5 side's karma and souls write to a match file and read back; the
/// karma's default (a fresh save's 500) and an unlisted soul list (every
/// soul) are left out, so old files load unchanged.
#[test]
fn karma_and_souls_write_and_read_back() {
    let content = bn5_content();
    let left = bn5(&TANGO_BN5, "").replacen("navi = \"megaman\"\n", "navi = \"megaman\"\nkarma = 100\nsouls = [\"protosoul\", \"colonelsoul\"]\n", 1);
    let m = parse(&content, &left, &bn5(&TANGO_BN5, "")).unwrap_or_else(|p| panic!("{p:?}"));
    let s = &m.sides[0];
    assert_eq!(s.karma, 100);
    let souls = ["protosoul", "colonelsoul"].map(|n| crate::ids::form(&content, "bn5", n).unwrap());
    assert_eq!(s.souls, Some(souls.to_vec()), "either version's");
    let text = crate::write(&content, &m);
    assert!(text.contains("karma = 100") && text.contains("souls = [") && text.contains("\"colonelsoul\""), "{text}");
    assert_eq!(crate::parse(&content, &text).unwrap(), m);
    // A side that says nothing of them: the defaults, nothing written.
    let plain = parse(&content, &bn5(&TANGO_BN5, ""), &bn5(&TANGO_BN5, "")).unwrap();
    assert_eq!((plain.sides[0].karma, plain.sides[0].souls.clone()), (500, None));
    let text = crate::write(&content, &plain);
    assert!(!text.contains("karma") && !text.contains("souls"), "{text}");
    // Karma past 1000; karma and a soul list under BN6's rules; a BN6
    // Cross, no soul of BN5's.
    let bad = side("megaman", &BN6, "").replacen("navi = \"megaman\"\n", "navi = \"megaman\"\nkarma = 1200\nsouls = [\"heatcross\"]\n", 1);
    let six = bn6_content();
    let e = parse_in(&six, "bn6", &side("megaman", &BN6, ""), &bad).unwrap_err();
    for p in [
        "right: karma 1200: the light/dark value is 0 to 1000",
        "right: karma, but the ruleset has no light and dark MegaMan (no system takes `karma`)",
        "right: a soul list, but the ruleset has no Soul Unison (no system takes `souls`)",
        "right: HeatCross is no soul",
    ] {
        assert!(e.iter().any(|x| x == p), "{p:?} not in {e:?}");
    }
    let bad = bn5(&TANGO_BN5, "souls = [\"heatcross\"]");
    let e = parse(&content, &bad, &bn5(&TANGO_BN5, "")).unwrap_err();
    assert!(e.contains(&"left: souls: no soul \"heatcross\" in bn5".to_string()), "{e:?}");
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
    let content = bn5_content();
    let at = |value: Option<u16>| {
        let mut m = parse(&content, &bn5(&TANGO_BN5, ""), &bn5(&TANGO_BN5, "")).unwrap();
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
    let base = content.base_form_for(crate::ids::navi(&content, "bn5", "megaman").unwrap());
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
    let content = bn5_content();
    // Swords alone, so the first chip dealt is one (no folder the rules
    // take: the round is played as set up).
    let sword = crate::ids::chip(&content, "bn5", "sword").unwrap();
    let offered = |souls: Option<Vec<nettai_content_api::FormHandle>>| {
        let mut m = parse(&content, &bn5(&TANGO_BN5, ""), &bn5(&TANGO_BN5, "")).unwrap();
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
    assert_eq!(offered(Some(vec![crate::ids::form(&content, "bn5", "gyrosoul").unwrap()])), (true, false));
    // ProtoSoul alone, or with Team Colonel's ColonelSoul: offered.
    let proto = crate::ids::form(&content, "bn5", "protosoul").unwrap();
    assert_eq!(offered(Some(vec![proto, crate::ids::form(&content, "bn5", "colonelsoul").unwrap()])), (true, true));
}

/// A side's karma and souls go into its round's setup: the light and dark
/// system's block holds the karma, the souls system's the souls, which
/// are the soul button's (by their numbers).
#[test]
fn karma_and_souls_reach_the_round() {
    let content = bn5_content();
    let mut m = parse(&content, &bn5(&TANGO_BN5, ""), &bn5(&TANGO_BN5, "")).unwrap();
    m.sides[0].karma = 300;
    m.sides[0].souls = Some(vec![crate::ids::form(&content, "bn5", "colonelsoul").unwrap()]);
    let b = started(&content, &m, 1);
    let (schema, block) = b.system_setup(0, "light-dark").unwrap();
    assert_eq!(block.get(schema, schema.index_of("karma").unwrap()), nettai_content_api::FieldValue::U16(300));
    let souls = b.setup.players[0].souls;
    assert!(souls.button && souls.chaos && souls.owned == 1 << 7, "{souls:?}");
    // A BN6 match's sides have no souls.
    let six_content = bn6_content();
    let six = crate::draw::live(&six_content, "bn6", 1, None).unwrap();
    let b = started(&six_content, &six, 1);
    assert_eq!(b.setup.players[1].souls, nettai_battle::custom::SoulUnlocks::default());
}

/// What the match's rules and a side's navi take decide the side's own
/// fields: a BN6 match's side takes its version, a navi code's level and
/// BN6's SP times; a BN5 match's no version and no level, and BN5's own SP
/// times (its SP navi chips', not BN6's).
#[test]
fn a_sides_fields_are_its_rules() {
    let (c5, c6) = (bn5_content(), bn6_content());
    let six = crate::draw::live(&c6, "bn6", 3, None).unwrap();
    let five = parse(&c5, &bn5(&TANGO_BN5, ""), &bn5(&TANGO_BN5, "")).unwrap();
    let (r6, r5) = (six.arena.ruleset, five.arena.ruleset);
    assert!(crate::Side::takes_game(&c6, r6) && six.sides[0].takes_level(&c6) && crate::Side::takes_sp_times(&c6));
    assert!(!crate::Side::takes_game(&c5, r5) && !five.sides[0].takes_level(&c5) && crate::Side::takes_sp_times(&c5));
    // Each slot's chip is of the match's game.
    let chip = |c: &nettai_battle::Content, m: &Match, slot| crate::facts::sp_chip(c, &m.arena, slot).map(|h| c.defs.chip(h).key.clone());
    assert!(chip(&c6, &six, 0).is_some_and(|k| k.starts_with("bn6:")), "{:?}", chip(&c6, &six, 0));
    assert!(chip(&c5, &five, 1).is_some_and(|k| k.starts_with("bn5:")), "{:?}", chip(&c5, &five, 1));
}

/// A ruleset change drops what the new rules don't take (the test
/// content's mix has no forms system: the Crosses go), and the same rules
/// keep everything.
#[test]
fn a_ruleset_change_drops_what_the_rules_dont_take() {
    let content = nettai_battle::content::testing::content();
    let stock = content.defs.stock_ruleset().unwrap();
    let mix = content.defs.ruleset_by_key("test-mix").unwrap();
    let navi = content.form_changing_navi().unwrap();
    let mut s = crate::Side {
        navi,
        game: nettai_battle::custom::GameVersion::Gregar,
        stats: crate::Side::base_stats(&content, navi, nettai_battle::custom::GameVersion::Gregar),
        emotion_window_glitch: false,
        folder: crate::Folder::EMPTY,
        crosses: crate::navi_crosses(&content, navi).map(|c| crate::CrossList::new(&c[..c.len().min(2)])),
        beast_out: true,
        cards: Vec::new(),
        navi_level: None,
        bug_frags: 0,
        sp_times: Default::default(),
        navicust: None,
        tactics: Default::default(),
        karma: crate::facts::DEFAULT_KARMA,
        souls: None,
    };
    let kept = s.clone();
    s.fit_rules(&content, stock);
    assert_eq!(s, kept, "the same rules keep everything");
    assert!(crate::ruleset_has_system(&content, stock, crate::FORMS_SYSTEM) && !crate::ruleset_has_system(&content, mix, crate::FORMS_SYSTEM));
    s.fit_rules(&content, mix);
    assert_eq!(s.crosses, None);
}
