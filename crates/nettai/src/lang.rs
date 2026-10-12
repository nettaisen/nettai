//! The languages (docs/app.md §3): the app's own strings come from the catalogs bundled into the UI (nettai-ui's
//! `build.rs`: Slint's `@tr`), and the content's names from its strings tables (its locales), in the same
//! language. The language shown at first is the settings', else the system's if it is bundled, else English;
//! Settings changes it while the app runs.
//!
//! One bundled language is no player's: the pseudo-locale (`pseudo`: every string lengthened and accented, made
//! by nettai-ui's build), a testing device. It is shown by `NETTAI_LANG=pseudo` and walked by the screenshot
//! pass; a player's list doesn't offer it, and the settings' file doesn't bring it back.

pub use nettai_ui::LANGUAGES;

/// The pseudo-locale's code.
pub const PSEUDO: &str = "pseudo";

/// The languages Settings offers while `shown` is the one shown: every bundled one a player may choose, and
/// the pseudo-locale only while it is the one shown (so the list always holds what it shows).
pub fn offered(shown: &str) -> Vec<(&'static str, &'static str)> {
    LANGUAGES.iter().copied().filter(|(code, _)| *code != PSEUDO || shown == PSEUDO).collect()
}

/// The language the system asks for, if it is bundled (by its primary subtag: `ja-JP` is `ja`), else English.
pub fn system() -> &'static str {
    let wanted = sys_locale::get_locale().unwrap_or_default();
    let primary = wanted.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    bundled(&primary).filter(|code| *code != PSEUDO).unwrap_or("en")
}

/// The bundled language of code `code`, if there is one.
pub fn bundled(code: &str) -> Option<&'static str> {
    LANGUAGES.iter().map(|(c, _)| *c).find(|c| *c == code)
}

/// The first language shown: `NETTAI_LANG`'s, else the settings' (never the pseudo-locale: only `NETTAI_LANG`
/// asks for that), else the system's.
pub fn initial(kept: Option<&str>) -> &'static str {
    first(std::env::var("NETTAI_LANG").ok().as_deref(), kept)
}

fn first(asked: Option<&str>, kept: Option<&str>) -> &'static str {
    asked.and_then(bundled).or_else(|| kept.filter(|k| *k != PSEUDO).and_then(bundled)).unwrap_or_else(system)
}

/// Show the app's strings in `code` (from the next frame; Slint updates every `@tr` string).
pub fn select(code: &str) {
    // (English is the source: it has no catalog.)
    let _ = slint::select_bundled_translation(if code == "en" { "" } else { code });
}

/// Whether `code`'s script is one the display face (Chakra Petch) has no letters for, and that is set solid:
/// its headings are in Murecho, and its labels' tracking is nearly none.
pub fn cjk(code: &str) -> bool {
    matches!(code, "ja" | "zh" | "ko")
}

/// The language's own name.
pub fn name(code: &str) -> &'static str {
    LANGUAGES.iter().find(|(c, _)| *c == code).map_or("English", |(_, n)| n)
}

/// The language after (or before) `code` in the list Settings offers, around from its end.
pub fn step(code: &str, by: i32) -> &'static str {
    let list = offered(code);
    let at = list.iter().position(|(c, _)| *c == code).unwrap_or(0) as i32;
    list[(at + by).rem_euclid(list.len() as i32) as usize].0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pseudo-locale is bundled, and no player meets it: the list leaves it out unless it is the one shown,
    /// stepping through the list passes it by, and a settings file that names it doesn't bring it back.
    #[test]
    fn the_pseudo_locale_is_no_players() {
        assert!(bundled(PSEUDO).is_some());
        assert!(offered("en").iter().all(|(code, _)| *code != PSEUDO));
        assert!(offered("en").iter().any(|(code, _)| *code == "ja"));
        assert!(offered(PSEUDO).iter().any(|(code, _)| *code == PSEUDO));
        let mut at = "en";
        for _ in 0..LANGUAGES.len() * 2 {
            at = step(at, 1);
            assert_ne!(at, PSEUDO);
        }
        assert_eq!(first(Some(PSEUDO), None), PSEUDO);
        assert_ne!(first(None, Some(PSEUDO)), PSEUDO);
        assert_eq!(first(None, Some("ja")), "ja");
    }
}
