//! The languages: the app's own strings come from the catalogs bundled into
//! it (`build.rs`: Slint's `@tr`), and the content's names from its
//! strings tables (its locales), in the same language. The language shown
//! at first is the system's, if one is bundled, else English; Settings
//! changes it while the app runs.

include!(concat!(env!("OUT_DIR"), "/languages.rs"));

/// The language the system asks for, if it is bundled (by its primary
/// subtag: `ja-JP` is `ja`), else English.
pub fn system() -> &'static str {
    let wanted = sys_locale::get_locale().unwrap_or_default();
    let primary = wanted.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    LANGUAGES.iter().map(|(code, _)| *code).find(|code| *code == primary).unwrap_or("en")
}

/// The language `NETTAI_LANG` names, if it is bundled, else the system's.
pub fn initial() -> &'static str {
    match std::env::var("NETTAI_LANG") {
        Ok(asked) => LANGUAGES.iter().map(|(code, _)| *code).find(|code| *code == asked).unwrap_or_else(system),
        Err(_) => system(),
    }
}

/// Show the app's strings in `code` (from the next frame; Slint updates
/// every `@tr` string).
pub fn select(code: &str) {
    // (English is the source: it has no catalog.)
    let _ = slint::select_bundled_translation(if code == "en" { "" } else { code });
}

/// How far apart tracked capitals are set in `code`'s script: Latin's
/// letters spaced, the scripts set solid (Chinese, Japanese, Korean)
/// nearly not.
pub fn tracking(code: &str) -> f32 {
    if matches!(code, "ja" | "zh" | "ko") { 0.2 } else { 1.0 }
}

/// The language's own name.
pub fn name(code: &str) -> &'static str {
    LANGUAGES.iter().find(|(c, _)| *c == code).map_or("English", |(_, n)| n)
}

/// The language after (or before) `code` in the list.
pub fn step(code: &str, by: i32) -> &'static str {
    let at = LANGUAGES.iter().position(|(c, _)| *c == code).unwrap_or(0) as i32;
    let n = LANGUAGES.len() as i32;
    LANGUAGES[((at + by) % n + n) as usize % LANGUAGES.len()].0
}
