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

/// A language's writing rules, made once when it is chosen (the user: "parsing the icu language identifier each time
/// seems expensive, why not just parse it once at the top level then pass it in everywhere? or like an i18n context
/// that holds a casemapper and a list formatter"): its code, its capitals and its lists. Choosing another makes a new
/// one whole ("on language switch why not just rebuild the i18n context with everything?"). A code that names no
/// language (the pseudo-locale's) takes the rules every language shares.
pub struct I18n {
    /// The language's code.
    pub code: &'static str,
    language: icu_locale_core::LanguageIdentifier,
    caps: icu_casemap::CaseMapperBorrowed<'static>,
    lists: icu_list::ListFormatter,
}

impl I18n {
    /// The rules of the language `code`.
    pub fn new(code: &'static str) -> I18n {
        use icu_list::options::{ListFormatterOptions, ListLength};
        let language = code.parse::<icu_locale_core::LanguageIdentifier>().unwrap_or(icu_locale_core::LanguageIdentifier::UNKNOWN);
        // CLDR's "and" list at its narrow length: it has no "and" before the last ("A, B, C", "A、B、C").
        let narrow = ListFormatterOptions::default().with_length(ListLength::Narrow);
        // (The data built in has every language's lists, and the one every language shares under them.)
        let lists = icu_list::ListFormatter::try_new_and((&language).into(), narrow).expect("the lists' data is built in");
        I18n { code, language, caps: icu_casemap::CaseMapper::new(), lists }
    }

    /// `text` in capitals by the language's rules (Unicode's: a Turkish or an Azeri i is İ, a Lithuanian i keeps its
    /// dot above where it has one), for the words the UI sets in capitals (`Caps` in theme.slint).
    pub fn upper(&self, text: &str) -> String {
        self.caps.uppercase_to_string(text, &self.language).into_owned()
    }

    /// `items` one after the other by the language's own separator ("A, B, C", "A、B、C"), for the lines the UI lists
    /// things on (`Lists` in theme.slint: a build's abilities, its patch cards): the user, "you should also join items
    /// by locale-aware separator in the ui".
    pub fn list<S: AsRef<str>>(&self, items: &[S]) -> String {
        self.lists.format(items.iter().map(AsRef::as_ref)).to_string()
    }
}

/// Show the app in the language `code`: its rules made (`I18n`) and given to the UI's `Caps` and `Lists`, then its
/// strings from its catalog (from the next frame: Slint says every `@tr` string again, and what they set in capitals
/// or list with it, by the new rules). The app keeps what this returns; the words it sets itself it sets again.
pub fn select(ui: &nettai_ui::AppWindow, code: &'static str) -> std::rc::Rc<I18n> {
    use slint::ComponentHandle;
    let i18n = std::rc::Rc::new(I18n::new(code));
    let rules = i18n.clone();
    ui.global::<nettai_ui::Caps>().on_of(move |text| rules.upper(&text).into());
    let rules = i18n.clone();
    ui.global::<nettai_ui::Lists>().on_of(move |items| rules.list(&slint::Model::iter(&items).collect::<Vec<_>>()).into());
    // (English is the source: it has no catalog.)
    let _ = slint::select_bundled_translation(if code == "en" { "" } else { code });
    ui.global::<nettai_ui::Theme>().set_cjk(cjk(code));
    i18n
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

    /// Capitals by the language's own rules, each language's made once and used for every text after: a Turkish i is
    /// İ where an English one is I; kana have no capitals; the pseudo-locale's code names no language and takes the
    /// rules every language shares.
    #[test]
    fn capitals_by_the_language() {
        let (en, tr, ja, pseudo, de) = (I18n::new("en"), I18n::new("tr"), I18n::new("ja"), I18n::new(PSEUDO), I18n::new("de"));
        assert_eq!(en.upper("Show file"), "SHOW FILE");
        assert_eq!(tr.upper("Dosya iste"), "DOSYA İSTE");
        assert_eq!(en.upper("Dosya iste"), "DOSYA ISTE");
        assert_eq!(tr.upper("Dosya iste"), "DOSYA İSTE", "the same rules, again");
        assert_eq!(ja.upper("ファイルを表示"), "ファイルを表示");
        assert_eq!(pseudo.upper("Ŝĥöŵ ƒîļé"), "ŜĤÖŴ ƑÎĻÉ");
        assert_eq!(de.upper("Straße"), "STRASSE");
    }

    /// Lists by the language's own separator, each language's rules made once and used for every list after, with no
    /// "and" before the last: a comma and a space in English, the ideographic comma in Japanese, the comma every
    /// language shares in the pseudo-locale; one item is itself, none is nothing.
    #[test]
    fn lists_by_the_language() {
        let (en, ja, pseudo) = (I18n::new("en"), I18n::new("ja"), I18n::new(PSEUDO));
        assert_eq!(en.list(&["SprArmr", "AirShoes", "FlotShoe"]), "SprArmr, AirShoes, FlotShoe");
        assert_eq!(en.list(&["HP+200", "NormBody"]), "HP+200, NormBody");
        assert_eq!(ja.list(&["スーパーアーマー", "エアシューズ", "フロートシューズ"]), "スーパーアーマー、エアシューズ、フロートシューズ");
        assert_eq!(ja.list(&["ナビ 3体", "プログラム 5個"]), "ナビ 3体、プログラム 5個");
        assert_eq!(pseudo.list(&["Â", "ß", "Ç"]), "Â, ß, Ç");
        assert_eq!(en.list(&["Cannon"]), "Cannon");
        assert_eq!(ja.list::<&str>(&[]), "");
        assert_eq!(en.upper(&en.list(&["Show", "file"])), "SHOW, FILE", "one language's rules, for both");
    }

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
