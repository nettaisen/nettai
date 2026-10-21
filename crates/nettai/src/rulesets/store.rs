//! The player's own rulesets on disk: each a file of its own (TOML) in the rulesets folder
//! (`crate::paths::rulesets`), a folder per game (`rulesets/exe6/<name>.nettairuleset`), as builds are kept. A ruleset's
//! file names its game and the ruleset, whether each build must be one version's, and what each category holds
//! back, by the category's fact and the things' names in the game (`nettai_match::ruleset::Ruleset::banned_names`):
//!
//! ```toml
//! game = "exe6"
//! name = "Tournament"
//! versions = "one_per_build"
//!
//! [banned]
//! folder = ["bass", "deltaray"]
//! patch_cards = ["starman"]
//! ```
//!
//! A category kept per record by a field of its records (`nettai_match::ruleset::Category::by`: EXE5's partners'
//! programs, by each partner's navi) has what it holds back for one key's records alone in `[banned_by]`, by the
//! category's fact, then the key's name, besides what `[banned]` holds back for every record:
//!
//! ```toml
//! [banned]
//! "party.programs" = ["p-hp-300"]
//!
//! [banned_by."party.programs"]
//! protoman = ["p-btlpk2"]
//! ```
//!
//! What it doesn't name is allowed. The presets are the game's (its rules/rulesets) and have no file.

use nettai_battle::Content;
use nettai_match::ruleset::{Name, Ruleset, Rulesets};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A ruleset file's extension (its text is TOML), the player's own and one exported alike.
pub const EXTENSION: &str = "nettairuleset";

/// Each build one version's, as a file says it.
const ONE_PER_BUILD: &str = "one_per_build";
/// The versions mixed, as a file says it.
const MIXED: &str = "mixed";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesetFile {
    game: String,
    name: String,
    versions: String,
    #[serde(default)]
    banned: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    banned_by: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

/// A ruleset's file, as it is listed: its path, its name, and its text (read against its game's rulesets once
/// they are at hand: [`read`]).
#[derive(Clone, Debug)]
pub struct Listed {
    pub path: PathBuf,
    pub name: String,
    pub text: String,
}

/// The folder of game `game`'s rulesets.
pub fn folder(game: &str) -> PathBuf {
    crate::paths::rulesets().join(game)
}

/// Game `game`'s own rulesets, by name: each file of its folder that is a ruleset of it (a file that isn't one is
/// left out).
pub fn list(game: &str) -> Vec<Listed> {
    let Ok(dir) = nettai_platform::fs::read_dir(folder(game)) else { return Vec::new() };
    let mut out: Vec<Listed> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == EXTENSION))
        .filter_map(|path| {
            let text = nettai_platform::fs::read_to_string(&path).ok()?;
            let f: RulesetFile = toml::from_str(&text).ok().filter(|f: &RulesetFile| f.game == game)?;
            Some(Listed { path, name: f.name, text })
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.path.cmp(&b.path)));
    out
}

/// A ruleset file's text read against its game's rulesets: the ruleset (named as the player's own), or what keeps
/// it from being one (a category or a name the game hasn't), in the log's words.
pub fn read(rulesets: &Rulesets, content: &Content, text: &str) -> Result<Ruleset, String> {
    let f: RulesetFile = toml::from_str(text).map_err(|e| e.message().to_string())?;
    if f.game != rulesets.game() {
        return Err(format!("a ruleset of {}, not of {}", f.game, rulesets.game()));
    }
    let one_per_build = match f.versions.as_str() {
        ONE_PER_BUILD => true,
        MIXED => false,
        other => return Err(format!("versions {other:?}: \"{ONE_PER_BUILD}\" or \"{MIXED}\"")),
    };
    let banned: Vec<(String, Vec<String>)> = f.banned.into_iter().collect();
    let banned_by: Vec<(String, Vec<(String, Vec<String>)>)> = f.banned_by.into_iter().map(|(fact, keys)| (fact, keys.into_iter().collect())).collect();
    Ruleset::of_names(rulesets, content, Name::Own(f.name), one_per_build, &banned, &banned_by)
}

/// A ruleset as its file's text: the player's own, named as it is (a preset's id where it is one).
pub fn text(content: &Content, game: &str, ruleset: &Ruleset) -> String {
    let name = match &ruleset.name {
        Name::Own(name) | Name::Preset(name) => name.clone(),
        // (The other player's is never saved as it is: a copy of it is the player's own, named.)
        Name::Theirs => String::new(),
    };
    let f = RulesetFile {
        game: game.to_string(),
        name,
        versions: if ruleset.one_per_build { ONE_PER_BUILD } else { MIXED }.to_string(),
        banned: ruleset.banned_names(content).into_iter().collect(),
        banned_by: ruleset.banned_by_names(content).into_iter().map(|(fact, keys)| (fact, keys.into_iter().collect())).collect(),
    };
    toml::to_string(&f).expect("a ruleset serializes")
}

/// Write a ruleset to `path` (its folder made if it isn't there).
pub fn write(path: &Path, content: &Content, game: &str, ruleset: &Ruleset) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        nettai_platform::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    nettai_platform::fs::write(path, text(content, game, ruleset)).map_err(|e| format!("{}: {e}", path.display()))
}

/// A file's name for a ruleset named `name`, without its extension: the name's letters and digits (`tournament`).
pub fn stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_lowercase().next().unwrap_or(c) } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if stem.is_empty() { "ruleset".into() } else { stem }
}

/// A path for a new ruleset of `game` named `name` that no file has: its [`stem`] (`tournament.nettairuleset`), numbered if
/// it is taken.
pub fn new_path(game: &str, name: &str) -> PathBuf {
    let stem = stem(name);
    let dir = folder(game);
    let mut path = dir.join(format!("{stem}.{EXTENSION}"));
    for n in 2.. {
        if !nettai_platform::fs::exists(&path) {
            break;
        }
        path = dir.join(format!("{stem}-{n}.{EXTENSION}"));
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::{exe6_content, rulesets};

    /// A ruleset's file reads back as the ruleset it was written from, by names; one of another game, or naming
    /// what the game hasn't, isn't one.
    #[test]
    fn a_ruleset_reads_back() {
        let content = exe6_content();
        let r = rulesets("exe6");
        let mut own = Ruleset::open(Name::Own("Gregar \"cup\"".into()));
        own.one_per_build = true;
        for c in r.categories.clone() {
            own.allow_groups(&r, &c, &["gregar"]);
        }
        own.set_allowed("folder", &nettai_match::ruleset::Item::of(nettai_match::ids::chip(&content, "exe6", "bass").unwrap().0), false);
        let text = text(&content, "exe6", &own);
        assert!(text.contains("versions = \"one_per_build\"") && text.contains("bass"), "{text}");
        assert!(text.contains("tengucross"), "Falzar's Crosses are held back by name: {text}");
        assert_eq!(read(&r, &content, &text), Ok(own.clone()));
        assert!(read(&rulesets("exe5"), &nettai_match::testing::exe5_content(), &text).unwrap_err().contains("not of exe5"));
        assert!(read(&r, &content, &text.replacen("bass", "nobody", 1)).is_err());
        assert!(read(&r, &content, &text.replacen("one_per_build", "sometimes", 1)).is_err());
        let mut mixed = r.preset("unrestricted").unwrap().clone();
        mixed.name = Name::Own("Open".into());
        assert_eq!(read(&r, &content, &super::text(&content, "exe6", &mixed)), Ok(mixed));
    }
}
