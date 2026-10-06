//! Matches of each game (docs/frontend.md §6), each on its game's content
//! alone (`testing::exe5_content`, `exe6_content`): a match is of one game,
//! its arena's, and names nothing of another's. EXE5's folder rules on
//! EXE5's sides (its rules' folder part,
//! content/exe5/rules/folder), an EXE5 match played a few hundred ticks,
//! EXE5's karma and souls, and a name another game has but the match's
//! hasn't refused as any unknown name is.

use crate::facts::Stated;
use crate::testing::{exe5_content, exe6_content};
use crate::{Match, check_match};
use nettai_battle::content::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{FormHandle, Registry, Value};
use std::sync::Arc;

/// A number, a flag and forms of `game` by name, as a side's fact takes
/// them.
fn number(n: i64) -> [Fact<'static>; 1] {
    [Fact::Value(Value::Int(n))]
}

fn flag(on: bool) -> [Fact<'static>; 1] {
    [Fact::Value(Value::Bool(on))]
}

fn forms(content: &Content, game: &str, names: &[&str]) -> Vec<Fact<'static>> {
    names.iter().map(|n| Fact::Value(Value::Def(Registry::Form, crate::ids::form(content, game, n).unwrap().0))).collect()
}

/// EXE5's souls, of both versions: what a side that says nothing has, in
/// the forms' order.
const EXE5_SOULS: [&str; 12] = [
    "colonelsoul",
    "gyrosoul",
    "knightsoul",
    "magnetsoul",
    "meddysoul",
    "napalmsoul",
    "numbersoul",
    "protosoul",
    "searchsoul",
    "shadowsoul",
    "toadsoul",
    "tomahawksoul",
];

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

/// An EXE5 match of these sides.
fn parse(content: &Arc<Content>, left: &str, right: &str) -> Result<Match, Vec<String>> {
    parse_in(content, "exe5", left, right)
}

/// A folder of Tango's EXE5 saves (their first): Standard chips only, four
/// copies at most.
const TANGO_EXE5: [&str; 30] = [
    "cannon A", "cannon A", "cannon B", "cannon B", "airshot *", "airshot *", "airshot *",
    "vulcan1 D", "vulcan1 D", "vulcan1 D", "minibomb B", "minibomb B", "minibomb L", "minibomb L",
    "sword S", "sword S", "sword S", "sword S", "wideswrd S", "wideswrd S", "busterup *",
    "crakout *", "crakout *", "recov10 A", "recov10 A", "recov10 L", "recov10 L", "areagrab S",
    "attck-10 *", "attck-10 *",
];

/// An EXE6 folder: the live navi's kind of folder, Standard chips.
const EXE6: [&str; 30] = [
    "cannon A", "cannon A", "cannon B", "airshot *", "airshot *", "vulcan1 D", "vulcan1 D",
    "minibomb B", "minibomb B", "minibomb L", "sword S", "sword S", "sword S", "wideswrd S",
    "wideswrd S", "recov10 A", "recov10 A", "recov10 L", "areagrab S", "areagrab S", "spreadr1 L",
    "spreadr1 L", "longswrd S", "longswrd S", "crakshot A", "crakshot A", "widesht P", "widesht P",
    "cannon C", "recov30 L",
];

/// An EXE5 MegaMan with this folder.
fn exe5(chips: &[&str], more: &str) -> String {
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

/// EXE5's MegaMan, by EXE5's rules, with Tango's EXE5 folder: the
/// rules accept it (the Regular chip too).
#[test]
fn a_exe5_folder_keeps_exe5s_rules() {
    let content = exe5_content();
    let right = exe5(&TANGO_EXE5, "");
    parse(&content, &exe5(&TANGO_EXE5, ""), &right).unwrap_or_else(|p| panic!("{p:?}"));
    // A Regular chip within the fresh navi's Regular memory (4 MB: CrakOut).
    parse(&content, &exe5(&TANGO_EXE5, "regular = 21"), &right).unwrap_or_else(|p| panic!("{p:?}"));
    // Three Mega chips, three dark chips (one of each), one Giga chip.
    let megas = with(&TANGO_EXE5, 0, &["blizman-ds B", "cloudmn-ds C", "colonel C", "drksword Z", "darkthnd M", "darkwide T", "bass F"]);
    parse(&content, &exe5(&refs(&megas), ""), &right).unwrap_or_else(|p| panic!("{p:?}"));
}

/// What EXE5's folder rules refuse, each said with its rule.
#[test]
fn exe5s_rules_refuse() {
    let content = exe5_content();
    let right = exe5(&TANGO_EXE5, "");
    let refused = |chips: &[String], more: &str| -> Vec<String> {
        let e = parse(&content, &exe5(&refs(chips), more), &right).expect_err("refused");
        assert!(e.iter().all(|p| p.starts_with("left: folder: ")), "{e:?}");
        e
    };
    let says = |e: &[String], what: &str| assert!(e.iter().any(|p| p.contains(what)), "{what:?} not in {e:?}");
    // A fifth copy of a Standard chip (EXE6 would take five 8 MB Cannons).
    let five = with(&TANGO_EXE5, 4, &["cannon C"]);
    says(&refused(&five, ""), "5 copies of cannon (a standard chip), past 4");
    // Two copies of a Mega chip, of a Giga chip, of a dark chip.
    let two = with(&TANGO_EXE5, 0, &["colonel C", "colonel C", "bass F", "bass F", "drksword Z", "drksword Z"]);
    let e = refused(&two, "");
    says(&e, "2 copies of colonel (a mega chip), past 1");
    says(&e, "2 copies of bass (a giga chip), past 1");
    says(&e, "2 copies of drksword (a dark chip), past 1");
    says(&e, "2 Giga chips, past the navi's 1");
    // Six Mega chips; four dark chips.
    let six = with(&TANGO_EXE5, 0, &["colonel C", "meddy M", "gyroman G", "knightmn K", "larkman S", "gridman F"]);
    says(&refused(&six, ""), "6 Mega chips, past the navi's 5");
    let four = with(&TANGO_EXE5, 0, &["drksword Z", "darkthnd M", "darkwide T", "drklance W"]);
    says(&refused(&four, ""), "4 dark chips, past 3");
    // A chip past the pack (BatCan1), a Program Advance; a code the chip
    // doesn't come in.
    let pack = with(&TANGO_EXE5, 0, &["batcan1 *", "lifesrd *"]);
    let e = refused(&pack, "");
    says(&e, "entry 0: batcan1 is no chip a folder can hold");
    says(&e, "entry 1: lifesrd is no chip a folder can hold");
    let code = with(&TANGO_EXE5, 0, &["cannon Z"]);
    says(&refused(&code, ""), "entry 0: cannon doesn't come in code Z");
    // A Regular chip past the Regular memory (Cannon is 8 MB; a fresh
    // navi's memory is 4); tag chips, which EXE5's folders haven't.
    says(&refused(&with(&TANGO_EXE5, 0, &[]), "regular = 0"), "the Regular chip cannon is 8 MB, past the navi's 4");
    says(&refused(&with(&TANGO_EXE5, 0, &[]), "tags = [4, 5]"), "tag chips, but EXE5's folders have none");
}

/// A name the match's game hasn't is refused as any unknown name is, the
/// same whether another game has it or nothing does: an EXE6 Cross, navi
/// or chip in an EXE5 match, an EXE5 chip in an EXE6 match, a name written with
/// a game (`exe6:cannon`: no name is written so) and a misspelling alike.
#[test]
fn an_unknown_name_is_refused() {
    let content = exe5_content();
    let says = |e: Vec<String>, what: &str| assert!(e.iter().any(|p| p.starts_with(what)), "{what:?} not in {e:?}");
    let ok = exe5(&TANGO_EXE5, "");
    // An EXE6 navi, Cross (as a soul: EXE5's rules take no Cross list at
    // all) and NaviCust program: none of EXE5's.
    // (EXE5 has a ProtoMan of its own: HeatMan is EXE6's alone.)
    says(parse(&content, &side("heatman", &TANGO_EXE5, ""), &ok).unwrap_err(), "left: no navi \"heatman\" in exe5");
    let crosses = exe5(&TANGO_EXE5, "souls = [\"heatcross\"]");
    says(parse(&content, &crosses, &ok).unwrap_err(), "left: souls: no form \"heatcross\" in exe5");
    let crosses = exe5(&TANGO_EXE5, "crosses = [\"heatcross\"]");
    says(parse(&content, &crosses, &ok).unwrap_err(), "left: no field \"crosses\" (a side of exe5 takes chaos_unison, hp, karma, level, reg_up, soul_unison, souls)");
    // A chip of EXE6's alone (HeatMan), a qualified name, a misspelling:
    // one error.
    let six = exe6_content();
    assert!(crate::ids::chip(&six, "exe6", "heatman").is_some() && crate::ids::chip(&content, "exe5", "heatman").is_none());
    for name in ["heatman", "exe6:cannon", "exe5:cannon", "canon"] { // (written in full)
        let chips = with(&TANGO_EXE5, 0, &[]);
        let mut chips = refs(&chips);
        let entry = format!("{name} A");
        chips[3] = &entry;
        says(parse(&content, &exe5(&chips, ""), &ok).unwrap_err(), &format!("left: folder entry 3: no chip {name:?} in exe5"));
    }
    // An EXE5 chip (GyroMan) in an EXE6 match.
    assert!(crate::ids::chip(&content, "exe5", "gyroman").is_some() && crate::ids::chip(&six, "exe6", "gyroman").is_none());
    let mut chips = EXE6.to_vec();
    chips[0] = "gyroman G";
    let e = parse_in(&six, "exe6", &side("megaman", &chips, ""), &side("megaman", &EXE6, "")).unwrap_err();
    says(e, "left: folder entry 0: no chip \"gyroman\" in exe6");
    // A game the content hasn't (EXE6's file on EXE5's content, a game no
    // content has).
    let text = match_text(&content, "exe5", &ok, &ok);
    let e = crate::parse(&content, &text.replacen("game = \"exe5\"", "game = \"bn7\"", 1)).unwrap_err();
    says(e, "no game \"bn7\" (the content's are exe5)");
    let e = crate::parse(&six, &text).unwrap_err();
    says(e, "no game \"exe5\" (the content's are exe6)");
}

/// A match's lookups see only its game: its stages, navis, chips (the
/// rules' pool), souls and patch cards are its own, so an EXE5 match's
/// sides, made or picked, hold EXE5's alone and an EXE6 match's EXE6's.
#[test]
fn a_matchs_lookups_only_see_its_game() {
    for (game, content) in [("exe5", exe5_content()), ("exe6", exe6_content())] {
        let of = |key: &str| crate::ids::in_game(&content, game, key);
        assert!(crate::link_battle_stages(&content, game).iter().all(|&s| of(&content.defs.stage(s).key)));
        assert!(crate::navis(&content, game).iter().all(|&n| of(&content.defs.navi(n).key)));
        let m = crate::pick::live(&content, game, 5, None).unwrap();
        assert_eq!(check_match(&content, &m), Vec::<String>::new(), "{game}");
        // (The forms its sides' facts hold: a picked Cross list, the souls
        // a side that says nothing has.)
        let held: Vec<u16> = crate::facts::fields(&content).iter().flat_map(|f| m.sides[0].facts.get(&content, f.name).unwrap().defs()).collect();
        assert!(!held.is_empty() && held.iter().all(|&f| of(&content.defs.form(FormHandle(f)).key)), "{game}");
        let mut b = crate::check::start(&content, &m).unwrap();
        let pool = crate::folders::pool(&content, game, &mut b, 0);
        assert!(pool.len() > 100 && pool.iter().all(|&c| of(&content.defs.chip(c).key)), "{game}: {} chips", pool.len());
        for s in &m.sides {
            assert!(of(&content.defs.navi(s.navi).key) && s.folder.chips().all(|c| of(&content.defs.chip(c.id).key)));
        }
        // Written, it names its game once, and nothing else qualified.
        let text = crate::write(&content, &m);
        assert!(text.contains(&format!("game = \"{game}\"")) && !text.contains("exe5:") && !text.contains("exe6:"), "{text}");
        assert_eq!(crate::parse(&content, &text).unwrap(), m);
    }
    // EXE5's souls aren't EXE6's; EXE6's names are none of EXE5's
    // content's.
    assert_eq!(crate::ids::form(&exe6_content(), "exe6", "protosoul"), None);
    assert!(crate::ids::form(&exe5_content(), "exe5", "protosoul").is_some());
    assert_eq!(crate::ids::form(&exe5_content(), "exe6", "heatcross"), None);
    assert_eq!(crate::ids::games(&exe5_content()), ["exe5"]);
    // A match of another game than the content's makes no match.
    assert!(Match::empty(&exe5_content(), "exe6").is_err());
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

/// An EXE5 match plays 900 ticks (custom screens, chips used) without the
/// engine stopping, its sides using EXE5's chips.
#[test]
fn a_exe5_match_plays() {
    let content = exe5_content();
    let m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let used = play(&content, &m, 900);
    assert!(used.iter().all(|u| !u.is_empty() && u.iter().all(|k| crate::ids::in_game(&content, "exe5", k))), "{used:?}");
}

/// What a match is, in words: each side's navi and the facts it states,
/// each as its name and value; a fact at its rules' default isn't said.
#[test]
fn a_match_is_described_by_its_facts() {
    let six = exe6_content();
    let mut m = crate::pick::live(&six, "exe6", 1, None).unwrap();
    let side = &mut m.sides[0];
    side.set_fact(&six, "version", &[Fact::Name("falzar")]).unwrap();
    side.set_fact(&six, "crosses", &forms(&six, "exe6", &["heatcross", "spoutcross"])).unwrap();
    side.set_fact(&six, "beast_out", &flag(false)).unwrap();
    side.set_fact(&six, "bug_frags", &number(9)).unwrap();
    let said = crate::describe(&six, &m, 1, false, 0);
    let line = said.lines().nth(1).unwrap();
    assert_eq!(line, "  MegaMan (you); beast_out: no; bug_frags: 9; crosses: HeatCross, SpoutCross; version: falzar");
    // (No Crosses is the list's default, which goes unsaid.)
    m.sides[0].set_fact(&six, "crosses", &[]).unwrap();
    assert!(crate::describe(&six, &m, 1, false, 0).lines().nth(1).unwrap().starts_with("  MegaMan (you); beast_out: no; bug_frags: 9; version: falzar"));
    let five = exe5_content();
    let mut m = parse(&five, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    assert_eq!(crate::describe(&five, &m, 1, false, 0).lines().nth(1).unwrap(), "  MegaMan (you)");
    m.sides[0].set_fact(&five, "karma", &number(100)).unwrap();
    m.sides[0].set_fact(&five, "souls", &[]).unwrap();
    assert_eq!(crate::describe(&five, &m, 1, false, 0).lines().nth(1).unwrap(), "  MegaMan (you); karma: 100; souls: none");
}

/// What a tool offers for a side's list of definitions: for the engine's
/// form list, the forms of the navi's own lists (EXE6's ten Crosses, in its
/// versions' order; none for a navi without); else what the rules' default
/// lists (EXE5's twelve souls), with whatever else the side's list holds.
#[test]
fn a_list_fact_offers_its_definitions() {
    let six = exe6_content();
    let m = crate::pick::live(&six, "exe6", 1, None).unwrap();
    let field = crate::facts::field(&six, "crosses").unwrap();
    let offered = crate::facts::offered(&six, "exe6", &m.sides[0], &field).unwrap();
    let megaman = six.navi(m.sides[0].navi).forms.as_ref().unwrap();
    let own: Vec<u16> = megaman.listed("gregar").iter().chain(megaman.listed("falzar")).map(|f| f.0).collect();
    assert_eq!((offered.len(), &offered), (10, &own));
    let mut link = m.sides[0].clone();
    link.navi = crate::ids::navi(&six, "exe6", "protoman").unwrap();
    assert_eq!(crate::facts::offered(&six, "exe6", &link, &field), Some(Vec::new()));
    // (Flags and single values are no lists of definitions.)
    for name in ["version", "beast_out", "bug_frags"] {
        assert_eq!(crate::facts::offered(&six, "exe6", &m.sides[0], &crate::facts::field(&six, name).unwrap()), None, "{name}");
    }
    let five = exe5_content();
    let mut m = parse(&five, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let field = crate::facts::field(&five, "souls").unwrap();
    let souls: Vec<u16> = EXE5_SOULS.map(|n| crate::ids::form(&five, "exe5", n).unwrap().0).to_vec();
    assert_eq!(crate::facts::offered(&five, "exe5", &m.sides[0], &field), Some(souls.clone()));
    // (A form the default doesn't list, MegaMan's base form, stated all the
    // same: offered after.)
    m.sides[0].set_fact(&five, "souls", &forms(&five, "exe5", &["protosoul", "base"])).unwrap();
    let base = crate::ids::form(&five, "exe5", "base").unwrap().0;
    assert_eq!(crate::facts::offered(&five, "exe5", &m.sides[0], &field).unwrap(), [souls, vec![base]].concat());
}

/// An EXE5 side's karma and souls write to a match file and read back; the
/// karma's default (a fresh save's 500) and an unlisted soul list (every
/// soul) are left out, so old files load unchanged.
#[test]
fn karma_and_souls_write_and_read_back() {
    let content = exe5_content();
    let left = exe5(&TANGO_EXE5, "").replacen("navi = \"megaman\"\n", "navi = \"megaman\"\nkarma = 100\nsouls = [\"protosoul\", \"colonelsoul\"]\n", 1);
    let m = parse(&content, &left, &exe5(&TANGO_EXE5, "")).unwrap_or_else(|p| panic!("{p:?}"));
    let s = &m.sides[0];
    assert_eq!(s.facts.get(&content, "karma"), Some(Stated::Number(100)));
    let souls = ["protosoul", "colonelsoul"].map(|n| crate::ids::form(&content, "exe5", n).unwrap().0);
    assert_eq!(s.facts.get(&content, "souls").unwrap().defs(), souls, "either version's");
    let text = crate::write(&content, &m);
    assert!(text.contains("karma = 100") && text.contains("souls = [") && text.contains("\"colonelsoul\""), "{text}");
    assert_eq!(crate::parse(&content, &text).unwrap(), m);
    // A side that says nothing of them: the defaults, nothing written.
    let plain = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    assert_eq!(plain.sides[0].facts, crate::Facts::defaults(&content));
    assert_eq!(plain.sides[0].facts.get(&content, "karma"), Some(Stated::Number(500)));
    let all: Vec<u16> = EXE5_SOULS.map(|n| crate::ids::form(&content, "exe5", n).unwrap().0).to_vec();
    assert_eq!(plain.sides[0].facts.get(&content, "souls").unwrap().defs(), all);
    let text = crate::write(&content, &plain);
    assert!(!text.contains("karma") && !text.contains("souls"), "{text}");
    // No souls at all is stated too (an empty list isn't the default).
    let none = exe5(&TANGO_EXE5, "").replacen("navi = \"megaman\"\n", "navi = \"megaman\"\nsouls = []\n", 1);
    let m = parse(&content, &none, &exe5(&TANGO_EXE5, "")).unwrap();
    assert!(m.sides[0].facts.get(&content, "souls").unwrap().defs().is_empty());
    let text = crate::write(&content, &m);
    assert!(text.contains("souls = []"), "{text}");
    assert_eq!(crate::parse(&content, &text).unwrap(), m);
    // Karma and a soul list under EXE6's rules, which take neither: the
    // facts EXE6 takes are said. (An EXE6 Cross is none of EXE5's forms.)
    let bad = side("megaman", &EXE6, "").replacen("navi = \"megaman\"\n", "navi = \"megaman\"\nkarma = 1200\nsouls = [\"heatcross\"]\n", 1);
    let six = exe6_content();
    let e = parse_in(&six, "exe6", &side("megaman", &EXE6, ""), &bad).unwrap_err();
    let takes = "(a side of exe6 takes beast_out, bug_frags, crosses, hp, level, reg_up, sun, version)";
    for p in [format!("right: no field \"karma\" {takes}"), format!("right: no field \"souls\" {takes}")] {
        assert!(e.iter().any(|x| *x == p), "{p:?} not in {e:?}");
    }
    let bad = exe5(&TANGO_EXE5, "souls = [\"heatcross\"]");
    let e = parse(&content, &bad, &exe5(&TANGO_EXE5, "")).unwrap_err();
    assert!(e.contains(&"left: souls: no form \"heatcross\" in exe5".to_string()), "{e:?}");
    // A value past its field's type, a soul twice, more souls than the list
    // holds.
    for (stated, said) in [
        ("karma = 70000", "left: karma: 70000 is past a u16 (0 to 65535)"),
        ("souls = [\"protosoul\", \"protosoul\"]", "left: souls: ProtSoul is there twice"),
        ("soul_unison = \"no\"", "left: soul_unison: \"no\" is neither true nor false"),
    ] {
        let e = parse(&content, &exe5(&TANGO_EXE5, stated), &exe5(&TANGO_EXE5, "")).unwrap_err();
        assert!(e.contains(&said.to_string()), "{said:?} not in {e:?}");
    }
}

/// What a setup that says nothing has is the rules' own to state (their
/// systems' `setup_defaults`), and a match writes none of it: EXE6's, Beast
/// Out (its version and its Crosses have no default: a setup that says
/// nothing states neither); EXE5's, every soul of either version, Soul
/// Unison, Chaos Unison and a fresh save's karma.
#[test]
fn a_setup_that_says_nothing_has_the_rules_defaults() {
    use nettai_battle::custom::PlayerSetup;
    use nettai_content_api::{FieldValue, Registry};
    let nothing = PlayerSetup::default();
    let six = exe6_content();
    let (schema, block) = nothing.rules_block(&six).expect("EXE6's rules");
    assert!(!block.stated(schema, schema.index_of("version").unwrap()), "version");
    // (A Cross list left out is empty: no Crosses.)
    let crosses = schema.index_of("crosses").unwrap();
    assert!(block.stated(schema, crosses) && (0..5).all(|k| block.get_elem(schema, crosses, k) == Some(FieldValue::Ref(None))));
    assert_eq!(block.get(schema, schema.index_of("beast_out").unwrap()), FieldValue::Bool(true));
    let five = exe5_content();
    let (schema, block) = nothing.rules_block(&five).expect("EXE5's rules");
    let souls = schema.index_of("souls").unwrap();
    let listed: Vec<_> = (0..16).filter_map(|k| block.get_elem(schema, souls, k)).take_while(|v| *v != FieldValue::Ref(None)).collect();
    let all: Vec<_> = EXE5_SOULS.iter().map(|n| FieldValue::Ref(Some((Registry::Form, crate::ids::form(&five, "exe5", n).unwrap().0)))).collect();
    assert_eq!(listed, all, "every soul the game has, in the forms' order");
    assert_eq!(all.len(), 12);
    for flag in ["soul_unison", "chaos_unison"] {
        assert_eq!(block.get(schema, schema.index_of(flag).unwrap()), FieldValue::Bool(true), "{flag}");
    }
    // A side that says nothing of them plays with exactly that.
    let plain = parse(&five, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let setup = plain.round(&five, 3);
    let (_, played) = setup.players[0].rules_block(&five).unwrap();
    assert_eq!(played, block);
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
/// face (the worn-out one: `megaman-dark`); a fresh save's (500)
/// with mood 128 and his plain face; a light one (1000) at 190.
#[test]
fn a_dark_side_starts_dark() {
    use nettai_battle::kinds::player::Emotion;
    let content = exe5_content();
    let at = |value: Option<u16>| {
        let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
        if let Some(v) = value {
            m.sides[0].set_fact(&content, "karma", &number(v as i64)).unwrap();
        }
        let b = started(&content, &m, 40);
        (b.stats[0].mood, nettai_battle::kinds::player::emotion(&b, 0))
    };
    assert_eq!(at(Some(100)), (0, Emotion::WornOut));
    assert_eq!(at(None), ((500 / 20 + 103) as u8, Emotion::Normal));
    assert_eq!(at(Some(1000)), (190, Emotion::Normal));
    // The face a mood of 0 shows: the base form's dark one.
    let base = content.base_form_for(crate::ids::navi(&content, "exe5", "megaman").unwrap());
    let face = content.form(base).mugshot.unwrap().of(Emotion::WornOut);
    assert_eq!(content.assets.handle(nettai_content_api::AssetKind::Mugshot, "megaman-dark"), Some(face.0));
}

/// The soul button (EXE5's souls part's) offers only a soul the side has:
/// with every soul (ProtoSoul among them), a Sword picked offers ProtoSoul;
/// with none, or with GyroSoul alone, the button is there but the sword's
/// soul isn't offered. Without Soul Unison there is no button.
#[test]
fn an_unowned_soul_cant_be_chosen() {
    use nettai_battle::custom::screen::{Phase, SPECIAL_SLOT, SlotKind, SlotState};
    let content = exe5_content();
    // Swords alone, so the first chip dealt is one (no folder the rules
    // take: the round is played as set up).
    let sword = crate::ids::chip(&content, "exe5", "sword").unwrap();
    let offered_with = |souls: Option<&[&str]>, soul_unison: bool| {
        let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
        m.sides[0].folder.chips = [Some(nettai_battle::custom::FolderChip::new(sword, nettai_battle::content::ChipCode(18))); 30];
        if let Some(names) = souls {
            m.sides[0].set_fact(&content, "souls", &forms(&content, "exe5", names)).unwrap();
        }
        m.sides[0].set_fact(&content, "soul_unison", &flag(soul_unison)).unwrap();
        let mut b = nettai_battle::Battle::new(m.round(&content, 0x5EED), content.clone());
        let mut last = 0u16;
        for _ in 0..400 {
            let screen = b.custom.sides[0].screen.as_ref().filter(|s| s.phase == Phase::Choosing);
            if let Some(s) = screen
                && s.selected == 1
            {
                let slot = s.slots[SPECIAL_SLOT as usize];
                let soul = matches!(slot.kind, SlotKind::Button { button, .. } if content.defs.button(button).name == "soul");
                return (soul, soul && slot.state == SlotState::Selectable);
            }
            let a = if screen.is_some() && last == 0 { nettai_battle::input::keys::A } else { 0 };
            last = a;
            b.tick(&[nettai_battle::input::PlayerTick { held: a }, Default::default()], Default::default());
        }
        panic!("no chip picked");
    };
    let offered = |souls| offered_with(souls, true);
    assert_eq!(offered(None), (true, true));
    assert_eq!(offered_with(None, false), (false, false), "no Soul Unison, no button");
    assert_eq!(offered(Some(&[])), (true, false));
    assert_eq!(offered(Some(&["gyrosoul"])), (true, false));
    // ProtoSoul alone, or with Team Colonel's ColonelSoul: offered.
    assert_eq!(offered(Some(&["protosoul", "colonelsoul"])), (true, true));
}

/// The soul given for a chip (the souls part's button and window): the
/// soul takes the Sword's place, first in the selection; B puts the Sword
/// back and offers the soul again; given again and OK, the turn's form is
/// ProtoSoul for 3 turns and the Sword leaves the folder in its place.
#[test]
fn the_soul_takes_the_chips_place() {
    use nettai_battle::custom::screen::{Phase, SPECIAL_SLOT, SlotKind, SlotState};
    use nettai_battle::input::{PlayerTick, keys};
    let content = exe5_content();
    let sword = crate::ids::chip(&content, "exe5", "sword").unwrap();
    let proto = crate::ids::form(&content, "exe5", "protosoul").unwrap();
    let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    m.sides[0].folder.chips = [Some(nettai_battle::custom::FolderChip::new(sword, nettai_battle::content::ChipCode(18))); 30];
    let mut b = nettai_battle::Battle::new(m.round(&content, 0x5EED), content.clone());
    let screen = |b: &nettai_battle::Battle| b.custom.sides[0].screen.expect("a screen");
    // A tick with `key` pressed, then ticks without until the screen is
    // back to choosing.
    let press = |b: &mut nettai_battle::Battle, key: u16| {
        b.tick(&[PlayerTick { held: key }, Default::default()], Default::default());
        for _ in 0..200 {
            b.tick(&[PlayerTick { held: 0 }, Default::default()], Default::default());
            if b.custom.sides[0].screen.is_some_and(|s| s.phase == Phase::Choosing) {
                return;
            }
        }
        panic!("the screen stays out of choosing");
    };
    for _ in 0..400 {
        if b.custom.sides[0].screen.is_some_and(|s| s.phase == Phase::Choosing) {
            break;
        }
        b.tick(&[PlayerTick { held: 0 }, Default::default()], Default::default());
    }
    // The first Sword picked: the soul is on offer.
    press(&mut b, keys::A);
    let s = screen(&b);
    assert_eq!((s.selection(), s.slots[SPECIAL_SLOT as usize].state), (&[0u8][..], SlotState::Selectable));
    assert!(matches!(s.slots[SPECIAL_SLOT as usize].kind, SlotKind::Button { .. }));
    // Given: the soul first in the selection, the Sword's slot still picked.
    b.custom.sides[0].screen.as_mut().unwrap().cursor = SPECIAL_SLOT;
    press(&mut b, keys::A);
    let s = screen(&b);
    assert_eq!(s.selection(), &[SPECIAL_SLOT][..]);
    assert_eq!((s.slots[0].state, s.slots[SPECIAL_SLOT as usize].state), (SlotState::Selected, SlotState::Selected));
    assert_eq!(s.trade.map(|t| (t.button, t.chip)), Some((SPECIAL_SLOT, 0)));
    // B: the Sword back in its place, the soul on offer again.
    press(&mut b, keys::B);
    let s = screen(&b);
    assert_eq!((s.selection(), s.trade), (&[0u8][..], None));
    assert_eq!(s.slots[SPECIAL_SLOT as usize].state, SlotState::Selectable);
    // Given again, and OK: the turn's form, its turns, the Sword gone.
    press(&mut b, keys::A);
    b.custom.sides[0].screen.as_mut().unwrap().cursor = nettai_battle::custom::screen::OK_SLOT;
    b.tick(&[PlayerTick { held: keys::A }, Default::default()], Default::default());
    let (hand, transform) = b.custom.sides[0].built.clone().expect("OK built the hand");
    assert_eq!((transform.form, transform.turns, transform.chaos), (Some(proto), 3, false));
    assert!(hand.is_some(), "the soul is a pick");
    let folder = b.custom.sides[0].folder;
    assert_eq!(folder.chips[0], None, "the Sword given up left the folder");
    assert_eq!(folder.count(), 29);
}

/// What a soul keeps of a custom screen in the souls part's state (its
/// form's `custom.state`) is as a fresh state has it when the next screen
/// deals, whatever soul the navi is in then: the original zeroes the
/// screen's record as it opens (0x08022CA2). MegaMan in MeddySoul is dealt
/// two capsules and mixes the first into a Sword; his soul has no turns
/// left, so the turn's start takes him back to his base form, and the next
/// screen's deal, out of MeddySoul, leaves no capsule and no mix.
#[test]
fn what_a_soul_keeps_of_a_screen_is_fresh_at_the_next_deal() {
    use nettai_battle::Battle;
    use nettai_battle::custom::screen::{OK_SLOT, Phase, SlotKind};
    use nettai_battle::input::{PlayerTick, keys};
    use nettai_content_api::{FieldValue, Registry};
    let content = exe5_content();
    let sword = crate::ids::chip(&content, "exe5", "sword").unwrap();
    let meddy = crate::ids::form(&content, "exe5", "meddysoul").unwrap();
    let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    m.sides[0].folder.chips = [Some(nettai_battle::custom::FolderChip::new(sword, nettai_battle::content::ChipCode(18))); 30];
    let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
    let base = b.stats[0].form;
    // (In MeddySoul from the start, his stats' starting form: the souls
    // part's state is a fresh one, with no turns of the soul.)
    b.stats[0].starting_form = meddy;
    let field = |b: &Battle, name: &str| {
        let (schema, state) = b.rules_state(0).expect("EXE5's rules");
        state.get(schema, schema.index_of(name).unwrap_or_else(|| panic!("the rules keep no `{name}`")))
    };
    let choosing = |b: &Battle, side: usize| {
        let s = &b.custom.sides[side];
        s.in_custom && s.screen.is_some_and(|s| s.phase == Phase::Choosing)
    };
    let tick = |b: &mut Battle, left: u16, right: u16| b.tick(&[PlayerTick { held: left }, PlayerTick { held: right }], Default::default());
    // A tick with `key` pressed by the left side, then ticks without until
    // its screen is back to choosing.
    let press = |b: &mut Battle, key: u16| {
        tick(b, key, 0);
        for _ in 0..200 {
            tick(b, 0, 0);
            if choosing(b, 0) {
                return;
            }
        }
        panic!("the screen stays out of choosing");
    };
    for _ in 0..400 {
        if choosing(&b, 0) && choosing(&b, 1) {
            break;
        }
        tick(&mut b, 0, 0);
    }
    // The first screen, in MeddySoul: two capsules, in the hand's last two
    // slots.
    assert!(choosing(&b, 0) && choosing(&b, 1));
    assert_eq!(b.stats[0].form, meddy);
    let a_capsule = |v: FieldValue| matches!(v, FieldValue::Ref(Some((Registry::Chip, _))));
    assert!(a_capsule(field(&b, "capsule_1")) && a_capsule(field(&b, "capsule_2")), "MeddySoul's deal picks two capsules");
    let screen = b.custom.sides[0].screen.unwrap();
    for slot in [8, 9] {
        assert!(matches!(screen.slots[slot].kind, SlotKind::Button { button, .. } if content.defs.button(button).name.starts_with("capsule_")));
    }
    // A Sword picked, and the first capsule mixed into it: its sequence's
    // end is in the state.
    press(&mut b, keys::A);
    b.custom.sides[0].screen.as_mut().unwrap().cursor = 8;
    press(&mut b, keys::A);
    assert_eq!((field(&b, "mix_capsule"), field(&b, "mix_step")), (FieldValue::U8(1), FieldValue::U8(24)));
    assert!(b.custom.sides[0].screen.unwrap().slots[0].attached.is_some(), "the capsule is in the Sword");
    // OK on both screens, and the fight: the turn's start reverts a soul
    // with no turns left.
    for side in 0..2 {
        b.custom.sides[side].screen.as_mut().unwrap().cursor = OK_SLOT;
    }
    tick(&mut b, keys::A, keys::A);
    for _ in 0..600 {
        if !b.custom.sides[0].in_custom && !b.custom.sides[1].in_custom {
            break;
        }
        tick(&mut b, 0, 0);
    }
    assert!(!b.custom.sides[0].in_custom, "the fight goes on");
    // L each other tick until the gauge is full and the next screen opens.
    for t in 0..6000 {
        if choosing(&b, 0) {
            break;
        }
        tick(&mut b, if t % 2 == 0 { keys::L } else { 0 }, 0);
    }
    assert!(choosing(&b, 0), "the second screen");
    assert_eq!(b.stats[0].form, base, "out of MeddySoul");
    // Its deal left what MeddySoul kept of the first screen as a fresh
    // state has it.
    for name in ["capsule_1", "capsule_2"] {
        assert_eq!(field(&b, name), FieldValue::Ref(None), "{name}");
    }
    for name in ["mix_capsule", "mix_step", "mix_count", "arm_step", "arm_timer"] {
        assert_eq!(field(&b, name), FieldValue::U8(0), "{name}");
    }
    let screen = b.custom.sides[0].screen.unwrap();
    assert!(!matches!(screen.slots[8].kind, SlotKind::Button { .. }), "no capsule's slot out of MeddySoul");
}

/// A side's karma and souls go into its round's setup: the rules'
/// setup holds the karma, the souls (which the
/// soul button offers), Soul Unison and Chaos Unison (on unless the side
/// says).
#[test]
fn karma_and_souls_reach_the_round() {
    let content = exe5_content();
    let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    m.sides[0].set_fact(&content, "karma", &number(300)).unwrap();
    let colonel = crate::ids::form(&content, "exe5", "colonelsoul").unwrap();
    m.sides[0].set_fact(&content, "souls", &forms(&content, "exe5", &["colonelsoul"])).unwrap();
    m.sides[0].set_fact(&content, "chaos_unison", &flag(false)).unwrap();
    let b = started(&content, &m, 1);
    let (schema, block) = b.rules_setup(0).unwrap();
    assert_eq!(block.get(schema, schema.index_of("karma").unwrap()), nettai_content_api::FieldValue::U16(300));
    let field = |name: &str| block.get(schema, schema.index_of(name).unwrap());
    let soul = |k: usize| block.get_elem(schema, schema.index_of("souls").unwrap(), k);
    use nettai_content_api::{FieldValue, Registry};
    assert_eq!(soul(0), Some(FieldValue::Ref(Some((Registry::Form, colonel.0)))));
    assert_eq!(soul(1), Some(FieldValue::Ref(None)));
    assert_eq!((field("soul_unison"), field("chaos_unison")), (FieldValue::Bool(true), FieldValue::Bool(false)));
    // An EXE6 match's sides have no souls.
    let six_content = exe6_content();
    let six = crate::pick::live(&six_content, "exe6", 1, None).unwrap();
    let b = started(&six_content, &six, 1);
    let (schema, _) = b.rules_setup(1).unwrap();
    assert!(schema.index_of("souls").is_none());
}

/// What the match's rules and a side's navi take decide the side's own
/// fields: an EXE6 match's side takes its version, a navi code's level and
/// EXE6's SP times; an EXE5 match's no version and no level, and EXE5's own SP
/// times (its SP navi chips', not EXE6's).
#[test]
fn a_sides_fields_are_its_rules() {
    let (c5, c6) = (exe5_content(), exe6_content());
    let six = crate::pick::live(&c6, "exe6", 3, None).unwrap();
    let five = parse(&c5, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    assert!(crate::Side::takes_version(&c6) && six.sides[0].takes_level(&c6) && crate::Side::takes_sp_times(&c6));
    assert!(!crate::Side::takes_version(&c5) && !five.sides[0].takes_level(&c5) && crate::Side::takes_sp_times(&c5));
    // Each slot's chip is of the match's game.
    let chip = |c: &nettai_battle::Content, m: &Match, slot| crate::facts::sp_chip(c, &m.arena, slot).map(|h| c.defs.chip(h).key.clone());
    assert!(chip(&c6, &six, 0).is_some_and(|k| crate::ids::in_game(&c6, "exe6", &k)), "{:?}", chip(&c6, &six, 0));
    assert!(chip(&c5, &five, 1).is_some_and(|k| crate::ids::in_game(&c5, "exe5", &k)), "{:?}", chip(&c5, &five, 1));
}
