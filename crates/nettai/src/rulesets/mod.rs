//! Rulesets (docs/design/app-ui.md §2.59; docs/app.md §8): what may be played in a match, the game's presets and
//! the player's own. What any screen needs of them is here:
//!
//! - the player's own on disk ([`store`]: a file each, by game, as builds are kept);
//! - what a player can pick of a game, in order ([`pickable`]: the presets, then their own by name) and a
//!   ruleset's name in the language shown ([`shown_name`]);
//! - the ruleset a player plays by, one for each game, remembered ([`played_by`], [`remember`]): the one a lobby
//!   they host starts on and the one the build editor marks builds against;
//! - a ruleset's page ([`page`]), which the lobby and the build editor open over themselves.
//!
//! What a ruleset is, what a game's categories, groups and presets are and what a build breaks under one are
//! nettai-match's `ruleset`; the app names no category, no version and no preset.

use crate::builds::Kit;
use crate::games::Names;
use crate::shell::Shell;
use nettai_match::ruleset::{Name, Ruleset, TEXT_TABLE};
use std::path::PathBuf;

pub mod page;
pub mod store;

/// The most letters a ruleset's name has.
pub const NAME_LENGTH: usize = 32;

/// A ruleset a player can pick: a preset (no file), or their own (its file).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pickable {
    pub ruleset: Ruleset,
    pub path: Option<PathBuf>,
}

/// The rulesets a player can pick of `kit`'s game, in order: the game's presets, then the player's own by name
/// (one whose file doesn't read as one is left out; why is in the log). None where the game says nothing of its
/// rulesets.
pub fn pickable(kit: &Kit) -> Vec<Pickable> {
    let Some(rulesets) = kit.rulesets() else { return Vec::new() };
    let mut out: Vec<Pickable> = rulesets.presets.iter().map(|p| Pickable { ruleset: p.clone(), path: None }).collect();
    for listed in store::list(&kit.game) {
        match store::read(&rulesets, kit.content(), &listed.text) {
            Ok(ruleset) => out.push(Pickable { ruleset, path: Some(listed.path) }),
            Err(why) => crate::errors::log(&format!("reading the ruleset {}", listed.path.display()), &why),
        }
    }
    out
}

/// A ruleset's name as a player reads it: a preset's in the game's strings in `names`' language (its id where
/// they have none), the player's own as they named it.
pub fn shown_name(names: &Names, name: &Name) -> String {
    match name {
        Name::Preset(id) => name.text_key().and_then(|k| names.text(TEXT_TABLE, &k)).unwrap_or_else(|| id.clone()),
        Name::Theirs => name.text_key().and_then(|k| names.text(TEXT_TABLE, &k)).unwrap_or_default(),
        Name::Own(own) => own.clone(),
    }
}

/// What the settings remember the ruleset a player plays by under, by game.
fn remembered_key(game: &str) -> String {
    format!("ruleset.{game}")
}

/// A ruleset's name as the settings remember it: `preset:<id>` or `own:<name>` (the other player's, which no
/// player picks to play by, `theirs`).
fn remembered_text(name: &Name) -> String {
    match name {
        Name::Preset(id) => format!("preset:{id}"),
        Name::Own(own) => format!("own:{own}"),
        Name::Theirs => "theirs".to_string(),
    }
}

/// The ruleset the player plays by in `kit`'s game: the one remembered for it, where it can still be picked;
/// else the game's first preset (its Standard). None where the game says nothing of its rulesets.
pub fn played_by(shell: &Shell, kit: &Kit) -> Option<Pickable> {
    let picks = pickable(kit);
    let remembered = shell.settings.remembered.get(&remembered_key(&kit.game));
    remembered.and_then(|text| picks.iter().find(|p| remembered_text(&p.ruleset.name) == *text)).or_else(|| picks.first()).cloned()
}

/// Remember `name` as the ruleset the player plays by in `game`.
pub fn remember(shell: &mut Shell, game: &str, name: &Name) {
    let (key, text) = (remembered_key(game), remembered_text(name));
    if shell.settings.remembered.get(&key) != Some(&text) {
        shell.settings.remembered.insert(key, text);
        shell.save_settings();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ruleset's name is remembered by kind and name, so a player's own named like a preset's id isn't
    /// taken for it.
    #[test]
    fn a_name_is_remembered_by_its_kind() {
        assert_eq!(remembered_text(&Name::Preset("standard".into())), "preset:standard");
        assert_eq!(remembered_text(&Name::Own("standard".into())), "own:standard");
        assert_eq!(remembered_key("exe6"), "ruleset.exe6");
    }
}
