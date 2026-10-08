//! The facts a build states only as one of a few presets (`builds.toml`'s
//! `presets`: EXE6's version, EXE5's team, EXE5's light and dark MegaMan).
//! A preset sets several facts together (a version and its form list); the
//! creator offers the presets in the fact's row, and shows none of the other
//! facts they set. The app's own data, by setup field name: the rules
//! declare no presets.
//!
//! A preset's values are the facts' own (a number, a flag, a variant's
//! name, a list of definitions by name), or `{ forms_of = "<list>" }`: the
//! side's navi's forms listed under that name (EXE6's MegaMan's `gregar`
//! Crosses), none for a navi that has no such list. Which preset a side's
//! fact is on: for a number, the one with the highest `from` it reaches; for
//! a variant, the one that sets it; for a list, the one that shares the most
//! of it (the first of those); for an unstated variant, none (no version is
//! assumed).

use crate::builds::{edit, layout};
use nettai_battle::content::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{FieldType, Registry, Value};
use nettai_match::Side;
use nettai_match::facts::{self, Stated};

/// A preset of a fact: its name, the least value of the fact on its side
/// of the game's line (a number's), and the facts it sets.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Preset {
    pub choice: String,
    #[serde(default)]
    pub from: Option<i64>,
    /// The row's name (the window words it), where it isn't the fact's:
    /// EXE5's team, whose fact is its souls.
    #[serde(default)]
    pub title: Option<String>,
    pub set: toml::Table,
}

impl Preset {
    /// What it sets, as a person reads it: `hp 997, karma 0`; a list by
    /// how many it holds, the navi's forms by their list's name.
    pub fn said(&self) -> String {
        let one = |(k, v): (&String, &toml::Value)| match v {
            toml::Value::Array(items) => format!("{k}: {}", items.len()),
            toml::Value::Table(t) => format!("{k}: its {} forms", t.get("forms_of").and_then(|f| f.as_str()).unwrap_or("?")),
            v => format!("{k} {v}"),
        };
        self.set.iter().map(one).collect::<Vec<_>>().join(", ")
    }
}

/// The presets of game `game`'s fact `fact`, in their order (none: the fact
/// is stated as it is).
pub fn of(game: &str, fact: &str) -> Option<&'static [Preset]> {
    layout::config().presets.get(game)?.get(fact).map(Vec::as_slice)
}

/// The name of the row of `fact`'s presets: their `title`, else the fact's.
pub fn title(game: &str, fact: &str) -> String {
    of(game, fact).and_then(|p| p.iter().find_map(|p| p.title.clone())).unwrap_or_else(|| fact.to_string())
}

/// The facts the presets of `fact` set besides it (EXE5's base HP, with the
/// light/dark value; EXE6's Crosses, with the version).
pub fn others(game: &str, fact: &str) -> Vec<String> {
    let mut names: Vec<String> = of(game, fact).unwrap_or_default().iter().flat_map(|p| p.set.keys().cloned()).filter(|k| k != fact).collect();
    names.sort();
    names.dedup();
    names
}

/// Whether fact `name` of game `game` is one a preset of another fact sets.
pub fn set_by_another(game: &str, name: &str) -> bool {
    layout::config().presets.get(game).is_some_and(|by_fact| by_fact.keys().any(|f| f != name && others(game, f).iter().any(|o| o == name)))
}

/// A preset's value of fact `name` as the facts to state (none: one the
/// game's setup can't hold).
fn facts_of<'v>(content: &Content, game: &str, side: &Side, name: &str, value: &'v toml::Value) -> Option<Vec<Fact<'v>>> {
    let def = |registry: Registry, h: u16| Fact::Value(Value::Def(registry, h));
    Some(match value {
        toml::Value::Integer(n) => vec![Fact::Value(Value::Int(*n))],
        toml::Value::Boolean(b) => vec![Fact::Value(Value::Bool(*b))],
        toml::Value::String(s) => vec![Fact::Name(s)],
        toml::Value::Array(items) => {
            let field = facts::field(content, name)?;
            let FieldType::Ref(registry, of) = layout::element(field.ty) else { return None };
            let of = of.as_deref();
            items.iter().map(|i| Some(def(*registry, nettai_match::ids::handle_of(content, game, *registry, of, i.as_str()?)?))).collect::<Option<Vec<_>>>()?
        }
        toml::Value::Table(t) => {
            let list = t.get("forms_of")?.as_str()?;
            let forms = content.navi(side.navi(content)).forms.as_ref().map(|f| f.listed(list).to_vec()).unwrap_or_default();
            forms.iter().map(|f| def(Registry::Form, f.0)).collect()
        }
        _ => return None,
    })
}

/// The preset the side's fact `fact` is on (the module's rules): none for an
/// unstated variant, or one no preset sets.
pub fn current(content: &Content, game: &str, side: &Side, fact: &str, presets: &'static [Preset]) -> Option<&'static Preset> {
    match side.facts.get(content, fact) {
        Some(Stated::Number(n) | Stated::Optional(Some(n))) => {
            presets.iter().filter(|p| p.from.is_some_and(|f| f <= n)).max_by_key(|p| p.from).or(presets.first())
        }
        Some(Stated::Variant(None)) => None,
        Some(Stated::Variant(Some(v))) => presets.iter().find(|p| p.set.get(fact).and_then(|x| x.as_str()) == Some(v.as_str())),
        Some(list @ Stated::List(_)) => {
            let held = list.defs();
            let shared = |p: &Preset| {
                let theirs = p.set.get(fact).and_then(|v| facts_of(content, game, side, fact, v)).unwrap_or_default();
                theirs.iter().filter(|f| matches!(f, Fact::Value(Value::Def(_, h)) if held.contains(h))).count()
            };
            // (The first of those that share the most.)
            let mut best: Option<(&'static Preset, usize)> = None;
            for p in presets {
                let n = shared(p);
                if best.is_none_or(|(_, m)| n > m) {
                    best = Some((p, n));
                }
            }
            best.map(|(p, _)| p)
        }
        _ => presets.first(),
    }
}

/// The version a side is on, by its preset's name (EXE6's version, EXE5's
/// team): the first of the game's presets that is a choice (no `from`, as
/// the light/dark value's are); none where the side states none.
pub fn version(content: &Content, game: &str, side: &Side) -> String {
    version_fact(game)
        .and_then(|fact| current(content, game, side, &fact, of(game, &fact)?))
        .map(|p| p.choice.clone())
        .unwrap_or_default()
}

/// The fact whose presets are the game's versions (`version`); none for a
/// game without.
pub fn version_fact(game: &str) -> Option<String> {
    let by_fact = layout::config().presets.get(game)?;
    by_fact.iter().find(|(_, p)| p.iter().all(|p| p.from.is_none())).map(|(fact, _)| fact.clone())
}

/// Set the side's facts to `preset`'s: whether that changed it. What the
/// side's navi has none of stays the rules' default
/// (`edit::without_forms`: a base HP where the navi doesn't change form).
pub fn take(content: &Content, game: &str, side: &mut Side, preset: &Preset) -> bool {
    let before = side.facts.clone();
    // (The facts of one value first: a list of the navi's forms follows
    // the version set with it.)
    let mut order: Vec<(&String, &toml::Value)> = preset.set.iter().collect();
    order.sort_by_key(|(_, v)| matches!(v, toml::Value::Array(_) | toml::Value::Table(_)));
    for (name, value) in order {
        // (A fact the game doesn't take, or a value its type doesn't: the
        // presets' test says so.)
        if let Some(facts) = facts_of(content, game, side, name, value) {
            let _ = side.set_fact(content, name, &facts);
        }
    }
    edit::without_forms(content, side);
    side.facts != before
}

/// The side's fact `fact` at the preset `by` places on from the one it is
/// on (from none, the first or the last): whether that changed the side.
pub fn step(content: &Content, game: &str, side: &mut Side, fact: &str, by: i64) -> bool {
    let Some(presets) = of(game, fact).filter(|p| !p.is_empty()) else { return false };
    let n = presets.len() as i64;
    let to = match current(content, game, side, fact, presets).and_then(|p| presets.iter().position(|x| std::ptr::eq(x, p))) {
        Some(at) => (at as i64 + by).rem_euclid(n),
        None if by < 0 => n - 1,
        None => 0,
    };
    take(content, game, side, &presets[to as usize])
}

/// A preset taken that changed the side: the fact's value before, the
/// preset, and the facts it changed besides.
#[derive(Clone, Debug)]
pub struct Taken {
    pub fact: String,
    pub was: String,
    pub preset: &'static Preset,
    /// The fact's own value changed (else only the facts set with it).
    pub moved: bool,
}

impl Taken {
    /// As an import says it.
    pub fn said(&self) -> String {
        if self.moved {
            format!("{} {} is {}'s: the {} preset ({})", self.fact, self.was, self.preset.choice, self.preset.choice, self.preset.said())
        } else {
            format!("{} {}: the {} preset's {}", self.fact, self.was, self.preset.choice, self.preset.said())
        }
    }
}

/// Set each of the side's facts stated as a preset to the preset it is on
/// ([`current`]; `hint`, a save's version or team, names the preset where
/// one has that name), and the facts that preset sets with it: the presets
/// that changed the side.
pub fn settle(content: &Content, game: &str, side: &mut Side, hint: Option<&str>) -> Vec<Taken> {
    let mut taken = Vec::new();
    let Some(by_fact) = layout::config().presets.get(game) else { return taken };
    for (fact, presets) in by_fact {
        let was = side.facts.get(content, fact);
        let named = hint.and_then(|h| presets.iter().find(|p| p.choice == h));
        let Some(preset) = named.or_else(|| current(content, game, side, fact, presets)) else { continue };
        if take(content, game, side, preset) {
            let moved = side.facts.get(content, fact) != was;
            let was = was.map(|v| facts::shown(content, &v)).unwrap_or_default();
            taken.push(Taken { fact: fact.clone(), was, preset, moved });
        }
    }
    taken
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_content_api::FormHandle;
    use nettai_match::facts::Stated;

    fn number(content: &Content, side: &Side, fact: &str) -> Option<i64> {
        match side.facts.get(content, fact) {
            Some(Stated::Number(n)) => Some(n),
            _ => None,
        }
    }

    /// The forms a side's list holds, by their keys' last part.
    fn forms(content: &Content, side: &Side, fact: &str) -> Vec<String> {
        let key = |h: u16| nettai_match::ids::local(&content.defs.form(FormHandle(h)).key).to_string();
        side.facts.get(content, fact).map(|v| v.defs().into_iter().map(key).collect()).unwrap_or_default()
    }

    /// Every preset of the app's sets each of its facts as it says, in the
    /// game it is of, on the game's MegaMan.
    #[test]
    fn every_preset_sets_its_facts() {
        let games = [(nettai_match::testing::exe6_content(), "exe6"), (nettai_match::testing::exe5_content(), "exe5"), (nettai_match::testing::exe4_content(), "exe4")];
        for (content, game) in games {
            let Some(by_fact) = layout::config().presets.get(game) else { continue };
            for presets in by_fact.values() {
                for p in presets {
                    let mut side = Side::fresh(&content, game).unwrap();
                    take(&content, game, &mut side, p);
                    for (name, value) in &p.set {
                        assert!(facts::field(&content, name).is_some(), "{game}'s preset {}: no fact {name}", p.choice);
                        let wanted = facts_of(&content, game, &side, name, value).unwrap_or_else(|| panic!("{game}'s preset {}: {name} isn't the fact's", p.choice));
                        assert!(!wanted.is_empty(), "{game}'s preset {}: {name} holds nothing", p.choice);
                        match (side.facts.get(&content, name), value) {
                            (Some(Stated::Number(n)), toml::Value::Integer(m)) => assert_eq!(n, *m),
                            (Some(Stated::Variant(v)), toml::Value::String(s)) => assert_eq!(v.as_deref(), Some(s.as_str())),
                            (Some(list), toml::Value::Array(_) | toml::Value::Table(_)) => {
                                let want: Vec<u16> = wanted.iter().filter_map(|f| if let Fact::Value(Value::Def(_, h)) = f { Some(*h) } else { None }).collect();
                                assert_eq!(list.defs(), want, "{game}'s preset {}: {name}", p.choice);
                            }
                            (got, _) => panic!("{game}'s preset {}: {name} is {got:?}", p.choice),
                        }
                    }
                }
            }
        }
    }

    /// EXE5's light and dark MegaMan: a new build is light (the rules'
    /// default value, 500, is on light's side), a value under the game's
    /// line of 470 is dark's, and a step goes from one to the other. A
    /// navi that doesn't change form keeps the rules' base HP.
    #[test]
    fn exe5s_light_and_dark() {
        let five = nettai_match::testing::exe5_content();
        let mut side = Side::fresh(&five, "exe5").unwrap();
        let taken = settle(&five, "exe5", &mut side, None);
        assert_eq!((number(&five, &side, "karma"), number(&five, &side, "hp")), (Some(1000), Some(1000)));
        let karma: Vec<String> = taken.iter().filter(|t| t.fact == "karma").map(Taken::said).collect();
        assert_eq!(karma, ["karma 500 is light's: the light preset (hp 1000, karma 1000)"]);
        for (value, preset) in [(469, "dark"), (470, "light"), (0, "dark"), (1000, "light")] {
            side.set_fact(&five, "karma", &[Fact::Value(Value::Int(value))]).unwrap();
            assert_eq!(current(&five, "exe5", &side, "karma", of("exe5", "karma").unwrap()).unwrap().choice, preset, "{value}");
        }
        assert!(settle(&five, "exe5", &mut side, None).is_empty(), "1000 is light's own");
        assert!(step(&five, "exe5", &mut side, "karma", 1));
        assert_eq!((number(&five, &side, "karma"), number(&five, &side, "hp")), (Some(0), Some(997)));
        assert!(step(&five, "exe5", &mut side, "karma", 1));
        assert_eq!((number(&five, &side, "karma"), number(&five, &side, "hp")), (Some(1000), Some(1000)));
        assert!(step(&five, "exe5", &mut side, "karma", -1));
        let protoman = nettai_match::ids::navi(&five, "exe5", "protoman").unwrap();
        edit::switch_navi(&five, &mut side, protoman);
        settle(&five, "exe5", &mut side, None);
        assert_eq!(number(&five, &side, "karma"), Some(0));
        assert!(side.facts.is_default(&five, "hp"), "a team navi's HP is its level's");
        assert_eq!(others("exe5", "karma"), ["hp"]);
        assert!(set_by_another("exe5", "hp") && !set_by_another("exe5", "karma") && !set_by_another("exe6", "hp"));
    }

    /// EXE4's light and dark MegaMan (Tango's EXE4 templates: 1000 and
    /// 1000, 460 and 997): a new build's 500 is light's, 499 dark's; a step
    /// goes from one to the other.
    #[test]
    fn exe4s_light_and_dark() {
        let four = nettai_match::testing::exe4_content();
        let mut side = Side::fresh(&four, "exe4").unwrap();
        settle(&four, "exe4", &mut side, None);
        assert_eq!((number(&four, &side, "karma"), number(&four, &side, "hp")), (Some(1000), Some(1000)));
        for (value, preset) in [(499, "dark"), (500, "light"), (460, "dark"), (1000, "light")] {
            side.set_fact(&four, "karma", &[Fact::Value(Value::Int(value))]).unwrap();
            assert_eq!(current(&four, "exe4", &side, "karma", of("exe4", "karma").unwrap()).unwrap().choice, preset, "{value}");
        }
        assert!(step(&four, "exe4", &mut side, "karma", 1));
        assert_eq!((number(&four, &side, "karma"), number(&four, &side, "hp")), (Some(460), Some(997)));
    }

    /// EXE4's version, by its souls (the content has no version): a new
    /// build's (every soul, the rules' default) is Red Sun's six, the app's
    /// version Red Sun; a step goes to Blue Moon's; the row is titled the
    /// version.
    #[test]
    fn exe4s_version_by_its_souls() {
        let four = nettai_match::testing::exe4_content();
        let mut side = Side::fresh(&four, "exe4").unwrap();
        settle(&four, "exe4", &mut side, None);
        let red = ["rollsoul", "gutssoul", "windsoul", "searchsoul", "firesoul", "thundersoul"].map(String::from).to_vec();
        assert_eq!((forms(&four, &side, "souls"), version(&four, "exe4", &side)), (red, "redsun".to_string()));
        assert!(step(&four, "exe4", &mut side, "souls", 1));
        let blue = ["protosoul", "numbersoul", "metalsoul", "junksoul", "aquasoul", "woodsoul"].map(String::from).to_vec();
        assert_eq!((forms(&four, &side, "souls"), version(&four, "exe4", &side)), (blue, "bluemoon".to_string()));
        assert_eq!(side.version(&four), None, "the content's rules take no version");
        assert_eq!(title("exe4", "souls"), "version");
    }

    /// Each EXE4 version's six souls are those its save can have
    /// (exe4-compat's soul flags, by compat's numbers of the forms).
    #[test]
    fn exe4s_versions_are_the_games() {
        let compat = exe4_compat::Compat::exe4();
        for version in [exe4_compat::Version::RedSun, exe4_compat::Version::BlueMoon] {
            let preset = of("exe4", "souls").unwrap().iter().find(|p| p.choice == version.name()).unwrap();
            let mut numbers: Vec<u8> = preset.set["souls"].as_array().unwrap().iter().map(|v| compat.form_number(v.as_str().unwrap()).unwrap()).collect();
            numbers.sort();
            let theirs: Vec<u8> = (1..=12).filter(|&n| version.soul_flag(n).is_some()).collect();
            assert_eq!(numbers, theirs, "{}", version.name());
        }
    }

    /// EXE6's version: a new build states none (none is assumed) and is on
    /// no preset; a step states one with its five Crosses, the next the
    /// other's. A file's mixed Crosses are the version's own five again,
    /// and so said. A link navi has no Crosses of either.
    #[test]
    fn exe6s_version_and_its_crosses() {
        let six = nettai_match::testing::exe6_content();
        let mut side = Side::fresh(&six, "exe6").unwrap();
        assert!(settle(&six, "exe6", &mut side, None).is_empty(), "no version, no preset");
        assert_eq!(side.version(&six), None);
        assert!(step(&six, "exe6", &mut side, "version", 1));
        assert_eq!((side.version(&six), forms(&six, &side, "crosses")), (Some("gregar"), ["heatcross", "eleccross", "slashcross", "erasecross", "chargecross"].map(String::from).to_vec()));
        assert!(step(&six, "exe6", &mut side, "version", 1));
        assert_eq!((side.version(&six), forms(&six, &side, "crosses")), (Some("falzar"), ["spoutcross", "tomahawkcross", "tengucross", "groundcross", "dustcross"].map(String::from).to_vec()));
        // A file's Gregar build with Crosses of both versions.
        let mixed: Vec<Fact> = ["heatcross", "spoutcross", "eleccross"].iter().map(|n| Fact::Value(Value::Def(Registry::Form, nettai_match::ids::form(&six, "exe6", n).unwrap().0))).collect();
        side.set_fact(&six, "version", &[Fact::Name("gregar")]).unwrap();
        side.set_fact(&six, "crosses", &mixed).unwrap();
        let taken = settle(&six, "exe6", &mut side, None);
        assert_eq!(forms(&six, &side, "crosses").len(), 5);
        assert!(forms(&six, &side, "crosses").iter().all(|f| six.form(nettai_match::ids::form(&six, "exe6", f).unwrap()).version.as_deref() == Some("gregar")));
        assert_eq!(taken.iter().map(Taken::said).collect::<Vec<_>>(), ["version gregar: the gregar preset's crosses: its gregar forms, version \"gregar\""]);
        assert_eq!(others("exe6", "version"), ["crosses"]);
        assert!(set_by_another("exe6", "crosses"));
        let elecman = nettai_match::ids::navi(&six, "exe6", "elecman").unwrap();
        edit::switch_navi(&six, &mut side, elecman);
        settle(&six, "exe6", &mut side, None);
        assert!(forms(&six, &side, "crosses").is_empty(), "a link navi has no Crosses");
    }

    /// EXE5's team, by its souls: a new build's (every soul, the rules'
    /// default) is the first team's six; a mixed list is the team it shares
    /// the most with; none is the first team's; a save's team names its
    /// own; a step goes to the other team.
    #[test]
    fn exe5s_team_and_its_souls() {
        let five = nettai_match::testing::exe5_content();
        let team = |name: &str| of("exe5", "souls").unwrap().iter().find(|p| p.choice == name).unwrap();
        let six_of = |p: &Preset| p.set["souls"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect::<Vec<_>>();
        let mut side = Side::fresh(&five, "exe5").unwrap();
        settle(&five, "exe5", &mut side, None);
        assert_eq!(forms(&five, &side, "souls"), six_of(team("protoman")));
        let state = |side: &mut Side, names: &[&str]| {
            let list: Vec<Fact> = names.iter().map(|n| Fact::Value(Value::Def(Registry::Form, nettai_match::ids::form(&five, "exe5", n).unwrap().0))).collect();
            side.set_fact(&five, "souls", &list).unwrap();
        };
        state(&mut side, &["protosoul", "colonelsoul", "shadowsoul", "toadsoul"]);
        settle(&five, "exe5", &mut side, None);
        assert_eq!(forms(&five, &side, "souls"), six_of(team("colonel")), "three of Team Colonel's to one");
        state(&mut side, &[]);
        settle(&five, "exe5", &mut side, None);
        assert_eq!(forms(&five, &side, "souls"), six_of(team("protoman")), "none: the first team");
        state(&mut side, &[]);
        settle(&five, "exe5", &mut side, Some("colonel"));
        assert_eq!(forms(&five, &side, "souls"), six_of(team("colonel")), "a Team Colonel save's");
        assert!(step(&five, "exe5", &mut side, "souls", 1));
        assert_eq!(forms(&five, &side, "souls"), six_of(team("protoman")));
        assert_eq!(title("exe5", "souls"), "team");
        assert_eq!(title("exe6", "version"), "version");
    }

    /// Each team's six souls are those its version's save can have
    /// (exe5-compat's soul flags, by compat's numbers of the forms).
    #[test]
    fn exe5s_teams_are_the_games() {
        let compat = exe5_compat::Compat::exe5();
        for (version, name) in [(exe5_compat::Version::Protoman, "protoman"), (exe5_compat::Version::Colonel, "colonel")] {
            let preset = of("exe5", "souls").unwrap().iter().find(|p| p.choice == name).unwrap();
            assert_eq!(version.name(), name);
            let mut numbers: Vec<u8> = preset.set["souls"].as_array().unwrap().iter().map(|v| compat.form_number(v.as_str().unwrap()).unwrap()).collect();
            numbers.sort();
            let theirs: Vec<u8> = (1..=12).filter(|&n| version.soul_flag(n).is_some()).collect();
            assert_eq!(numbers, theirs, "{name}");
        }
    }
}
