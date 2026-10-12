//! The languages (docs/app.md §3): the app's own strings come from the catalogs bundled into the UI (nettai-ui's
//! `build.rs`: Slint's `@tr`), and the content's names from its strings tables (its locales), in the same
//! language. The language shown at first is the settings', else the system's if it is bundled, else English;
//! Settings changes it while the app runs.

pub use nettai_ui::LANGUAGES;

/// The language the system asks for, if it is bundled (by its primary subtag: `ja-JP` is `ja`), else English.
pub fn system() -> &'static str {
    let wanted = sys_locale::get_locale().unwrap_or_default();
    let primary = wanted.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    bundled(&primary).unwrap_or("en")
}

/// The bundled language of code `code`, if there is one.
pub fn bundled(code: &str) -> Option<&'static str> {
    LANGUAGES.iter().map(|(c, _)| *c).find(|c| *c == code)
}

/// The first language shown: `NETTAI_LANG`'s, else the settings', else the system's.
pub fn initial(kept: Option<&str>) -> &'static str {
    let asked = std::env::var("NETTAI_LANG").ok();
    asked.as_deref().and_then(bundled).or_else(|| kept.and_then(bundled)).unwrap_or_else(system)
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

/// The language after (or before) `code` in the list, around from its end.
pub fn step(code: &str, by: i32) -> &'static str {
    let at = LANGUAGES.iter().position(|(c, _)| *c == code).unwrap_or(0) as i32;
    LANGUAGES[(at + by).rem_euclid(LANGUAGES.len() as i32) as usize].0
}
