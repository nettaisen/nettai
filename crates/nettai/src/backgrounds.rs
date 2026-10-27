//! The battle's background (docs/app.md §1, "The battle's background"): one pool of every game's backgrounds,
//! any of which a battle of any game may be drawn on. The user: "is it possible to use e.g. exe4 backgrounds in
//! exe5? as in if they all came from the same pool of backgrounds?", then "sure do that"; and, asked whether
//! both players of an online match see the round's one or each their own, "Shared, as now".
//!
//! What the pool is is the content's (`Content::background_pool`, from each game's locales, the same whatever
//! packs are installed), and a round's background is picked and carried by the library (`nettai_match`'s
//! `BackgroundPick::Pool`, the lobby's settings, a replay's match); this names each for a list: its game's name
//! in the language shown, then its own ("BN4 ACDC Area", "EXE4 秋原エリア"), from the content's locales alone, so
//! a game whose pack isn't there has its names too.

use crate::shell::Shell;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// The backgrounds' names in the language shown, by game and name in its game: [`name_backgrounds`]'.
    static NAMES: RefCell<HashMap<(String, String), String>> = RefCell::new(HashMap::new());
}

/// Read the backgrounds' names of `games` in `lang`, for [`name`]: from the content's locales alone, at start-up
/// and when the language changes (`Shell::read_game_names`).
pub fn name_backgrounds<'a>(games: impl IntoIterator<Item = &'a str>, lang: &str) {
    let mut names = HashMap::new();
    for game in games {
        let Some(dir) = crate::contents::dir(game) else { continue };
        for (background, name) in nettai_content::locale::background_names(&dir, game, lang) {
            names.insert((game.to_string(), background), name);
        }
    }
    NAMES.with(|n| *n.borrow_mut() = names);
}

/// Game `game`'s background `background` (its name in the game) as a list shows it, in the language
/// shown: the game's name, then the background's ("BN4 ACDC Area"); one its locales don't name, by its name in
/// the game's graphics.
pub fn name(game: &str, background: &str) -> String {
    let own = NAMES.with(|n| n.borrow().get(&(game.to_string(), background.to_string())).cloned()).unwrap_or_else(|| background.to_string());
    format!("{} {own}", Shell::game_name(game))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A background is named by its game and its own name in the language shown, from the content alone; the
    /// pool is every game's, the same whichever game's content it is read beside.
    #[test]
    fn a_background_is_named_by_its_game() {
        Shell::read_game_names(&["exe4", "exe5", "exe6"], "en");
        assert_eq!(name("exe4", "acdc-area"), "BN4 ACDC Area");
        assert_eq!(name("exe6", "mr-weather-comp-storm"), "BN6 MrWeathrCmp (Typhoon)");
        assert_eq!(name("exe5", "no-such-area"), "BN5 no-such-area");
        Shell::read_game_names(&["exe4", "exe5", "exe6"], "ja");
        assert_eq!(name("exe4", "acdc-area"), "EXE4 秋原エリア");
        assert_eq!(name("exe5", "nebulagray"), "EXE5 ネビュラグレイ");
        Shell::read_game_names(&["exe4", "exe5", "exe6"], "en");
        let pools = [nettai_match::testing::exe4_content(), nettai_match::testing::exe5_content(), nettai_match::testing::exe6_content()]
            .map(|c| c.background_pool.keys().map(str::to_string).collect::<Vec<_>>());
        assert_eq!(pools[0], pools[1]);
        assert_eq!(pools[1], pools[2]);
        assert_eq!(pools[0].len(), 27 + 22 + 21, "every game's");
    }
}
