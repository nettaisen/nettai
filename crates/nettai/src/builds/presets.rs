//! The facts a build states only as one of a few presets (`builds.toml`'s
//! `presets`: EXE5's light and dark MegaMan, as Tango's save templates have
//! them). A preset sets several facts together; the creator offers the
//! presets in the fact's row, and shows none of the other facts they set.
//! The app's own data, by setup field name: the rules declare no presets.

use crate::builds::{edit, layout};
use nettai_battle::content::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::Value;
use nettai_match::Side;
use nettai_match::facts::{self, Stated};

/// A preset of a fact: its name, the least value of the fact on its side
/// of the game's line, and the facts it sets.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Preset {
    pub choice: String,
    pub from: i64,
    pub set: toml::Table,
}

impl Preset {
    /// What it sets, as a person reads it: `hp 997, karma 0`.
    pub fn said(&self) -> String {
        self.set.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", ")
    }
}

/// The presets of game `game`'s fact `fact`, in their order (none: the fact
/// is stated as it is).
pub fn of(game: &str, fact: &str) -> Option<&'static [Preset]> {
    layout::config().presets.get(game)?.get(fact).map(Vec::as_slice)
}

/// The facts the presets of `fact` set besides it (EXE5's base HP, with the
/// light/dark value).
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

/// The preset the side's fact `fact` is on the side of: the one with the
/// highest `from` its value reaches (a value that is no number: the first).
pub fn current(content: &Content, side: &Side, fact: &str, presets: &'static [Preset]) -> Option<&'static Preset> {
    let value = match side.facts.get(content, fact) {
        Some(Stated::Number(n) | Stated::Optional(Some(n))) => n,
        _ => return presets.first(),
    };
    presets.iter().filter(|p| p.from <= value).max_by_key(|p| p.from).or(presets.first())
}

/// Set the side's facts to `preset`'s: whether that changed it. What the
/// side's navi has none of stays the rules' default
/// (`edit::without_forms`: a base HP where the navi doesn't change form).
pub fn take(content: &Content, side: &mut Side, preset: &Preset) -> bool {
    let before = side.facts.clone();
    for (name, value) in &preset.set {
        let fact = match value {
            toml::Value::Integer(n) => Fact::Value(Value::Int(*n)),
            toml::Value::Boolean(b) => Fact::Value(Value::Bool(*b)),
            toml::Value::String(s) => Fact::Name(s),
            _ => continue,
        };
        // (A fact the game doesn't take, or a value its type doesn't: the
        // presets' test says so.)
        let _ = side.set_fact(content, name, &[fact]);
    }
    edit::without_forms(content, side);
    side.facts != before
}

/// The side's fact `fact` at the preset `by` places on from the one it is
/// on: whether that changed the side.
pub fn step(content: &Content, game: &str, side: &mut Side, fact: &str, by: i64) -> bool {
    let Some(presets) = of(game, fact).filter(|p| !p.is_empty()) else { return false };
    let at = current(content, side, fact, presets).and_then(|p| presets.iter().position(|x| std::ptr::eq(x, p))).unwrap_or(0) as i64;
    take(content, side, &presets[(at + by).rem_euclid(presets.len() as i64) as usize])
}

/// A fact whose value was set to a preset's: the value it had, and the
/// preset its value was on the side of.
#[derive(Clone, Debug)]
pub struct Taken {
    pub fact: String,
    pub was: String,
    pub preset: &'static Preset,
}

impl Taken {
    /// As an import says it.
    pub fn said(&self) -> String {
        format!("{} {} is {}'s: the {} preset ({})", self.fact, self.was, self.preset.choice, self.preset.choice, self.preset.said())
    }
}

/// Set each of the side's facts stated as a preset to the preset its value
/// is on the side of ([`current`]), and the facts that preset sets with
/// it: the facts whose own value that changed.
pub fn settle(content: &Content, game: &str, side: &mut Side) -> Vec<Taken> {
    let mut taken = Vec::new();
    let Some(by_fact) = layout::config().presets.get(game) else { return taken };
    for (fact, presets) in by_fact {
        let was = side.facts.get(content, fact);
        let Some(preset) = current(content, side, fact, presets) else { continue };
        take(content, side, preset);
        if side.facts.get(content, fact) != was {
            let was = was.map(|v| facts::shown(content, &v)).unwrap_or_default();
            taken.push(Taken { fact: fact.clone(), was, preset });
        }
    }
    taken
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::facts::Stated;

    fn number(content: &Content, side: &Side, fact: &str) -> Option<i64> {
        match side.facts.get(content, fact) {
            Some(Stated::Number(n)) => Some(n),
            _ => None,
        }
    }

    /// Every preset of the app's sets each of its facts as it says, in the
    /// game it is of.
    #[test]
    fn every_preset_sets_its_facts() {
        for (content, game) in [(nettai_match::testing::exe6_content(), "exe6"), (nettai_match::testing::exe5_content(), "exe5")] {
            let Some(by_fact) = layout::config().presets.get(game) else { continue };
            for presets in by_fact.values() {
                for p in presets {
                    let mut side = Side::fresh(&content, game).unwrap();
                    take(&content, &mut side, p);
                    for (name, value) in &p.set {
                        assert!(facts::field(&content, name).is_some(), "{game}'s preset {}: no fact {name}", p.choice);
                        assert_eq!(number(&content, &side, name), value.as_integer(), "{game}'s preset {}: {name}", p.choice);
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
        let taken = settle(&five, "exe5", &mut side);
        assert_eq!((number(&five, &side, "karma"), number(&five, &side, "hp")), (Some(1000), Some(1000)));
        assert_eq!(taken.iter().map(Taken::said).collect::<Vec<_>>(), ["karma 500 is light's: the light preset (hp 1000, karma 1000)"]);
        for (value, preset) in [(469, "dark"), (470, "light"), (0, "dark"), (1000, "light")] {
            side.set_fact(&five, "karma", &[Fact::Value(Value::Int(value))]).unwrap();
            assert_eq!(current(&five, &side, "karma", of("exe5", "karma").unwrap()).unwrap().choice, preset, "{value}");
        }
        assert!(settle(&five, "exe5", &mut side).is_empty(), "1000 is light's own");
        assert!(step(&five, "exe5", &mut side, "karma", 1));
        assert_eq!((number(&five, &side, "karma"), number(&five, &side, "hp")), (Some(0), Some(997)));
        assert!(step(&five, "exe5", &mut side, "karma", 1));
        assert_eq!((number(&five, &side, "karma"), number(&five, &side, "hp")), (Some(1000), Some(1000)));
        assert!(step(&five, "exe5", &mut side, "karma", -1));
        let protoman = nettai_match::ids::navi(&five, "exe5", "protoman").unwrap();
        edit::switch_navi(&five, &mut side, protoman);
        settle(&five, "exe5", &mut side);
        assert_eq!(number(&five, &side, "karma"), Some(0));
        assert!(side.facts.is_default(&five, "hp"), "a team navi's HP is its level's");
        assert_eq!(others("exe5", "karma"), ["hp"]);
        assert!(set_by_another("exe5", "hp") && !set_by_another("exe5", "karma") && !set_by_another("exe6", "hp"));
    }
}
