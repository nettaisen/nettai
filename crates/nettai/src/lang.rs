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

/// The language the system asks for, if it is bundled (by its primary subtag: `ja-JP` is `ja`; Chinese by its
/// script, `zh-Hant-TW`, or its region, `zh-TW`, `zh-HK` and `zh-MO` Traditional, the rest Simplified), else English.
pub fn system() -> &'static str {
    of_locale(&sys_locale::get_locale().unwrap_or_default())
}

fn of_locale(wanted: &str) -> &'static str {
    let tags: Vec<String> = wanted.split(['-', '_', '.', '@']).map(str::to_ascii_lowercase).collect();
    let primary = tags.first().map_or("", String::as_str);
    let code = if primary == "zh" {
        let traditional = tags[1..].iter().any(|t| matches!(t.as_str(), "hant" | "tw" | "hk" | "mo"));
        let simplified = tags[1..].iter().any(|t| t == "hans");
        if traditional && !simplified { "zh-Hant" } else { "zh-Hans" }
    } else {
        primary
    };
    bundled(code).filter(|code| *code != PSEUDO).unwrap_or("en")
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
    /// dot above where it has one), for the words the UI sets in capitals (`I18n.upper` in theme.slint).
    pub fn upper(&self, text: &str) -> String {
        self.caps.uppercase_to_string(text, &self.language).into_owned()
    }

    /// `items` one after the other by the language's own separator ("A, B, C", "A、B、C"), for the lines the UI lists
    /// things on (`I18n.join` in theme.slint: a build's abilities, its patch cards): the user, "you should also join items
    /// by locale-aware separator in the ui".
    pub fn list<S: AsRef<str>>(&self, items: &[S]) -> String {
        self.lists.format(items.iter().map(AsRef::as_ref)).to_string()
    }
}

/// Show the app in the language `code`: its rules made (`I18n`) and given to the UI's `I18n`, then its
/// strings from its catalog (from the next frame: Slint says every `@tr` string again, and what they set in capitals
/// or list with it, by the new rules). The app keeps what this returns; the words it sets itself it sets again.
pub fn select(ui: &nettai_ui::AppWindow, code: &'static str) -> std::rc::Rc<I18n> {
    use slint::ComponentHandle;
    let i18n = std::rc::Rc::new(I18n::new(code));
    let rules = i18n.clone();
    let ui_i18n = ui.global::<nettai_ui::I18n>();
    ui_i18n.on_uppercased(move |text| rules.upper(&text).into());
    let rules = i18n.clone();
    ui_i18n.on_joined(move |items| rules.list(&slint::Model::iter(&items).collect::<Vec<_>>()).into());
    // (English is the source: it has no catalog.)
    let _ = slint::select_bundled_translation(if code == "en" { "" } else { code });
    let theme = ui.global::<nettai_ui::Theme>();
    theme.set_cjk(cjk(code));
    theme.set_body_face(nettai_render::vfont::Cut::of(code).family().into());
    theme.set_cjk_display(display_face(code).into());
    // (Everything set in capitals or listed is set again by the new rules: the words the app gave too, which no
    // `@tr` brings back. The user: "fix the turkish issue".)
    ui_i18n.set_language(ui_i18n.get_language().wrapping_add(1));
    i18n
}

/// Whether `code`'s script is one the display face (Chakra Petch) has no letters for, and that is set solid:
/// its headings are in its own display face ([`display_face`]), and its labels' tracking is nearly none.
pub fn cjk(code: &str) -> bool {
    matches!(code, "ja" | "zh-Hans" | "zh-Hant" | "ko")
}

/// The display face of a language Chakra Petch has no letters for: Corporate Logo for Japanese; for Chinese, whose
/// hanzi it lacks, the body's family at its heaviest (Noto Sans SC, TC).
pub fn display_face(code: &str) -> &'static str {
    match code {
        "zh-Hans" | "zh-Hant" => nettai_render::vfont::Cut::of(code).family(),
        _ => "Corporate Logo ver3",
    }
}

/// Give the UI the bundled Noto Sans CJK, the battle's text font (nettai-render's `vfont`: one copy of its three
/// variable files in the program), by its cuts' families ("Noto Sans JP", "Noto Sans SC", "Noto Sans TC": the theme's
/// `body-face`), and as every face's fallback for hanzi and kana; and M PLUS 1 Code, the theme's `mono`. The program
/// holds their files as WOFF2, and this unpacks them (nettai-render's `bundled`). Before the first window: the theme
/// names them.
pub fn register_fonts() {
    use slint::fontique_011::fontique;
    use nettai_render::vfont::Cut;
    let mut collection = slint::fontique_011::shared_collection();
    let mut families = Vec::new();
    for cut in [Cut::Japanese, Cut::Simplified, Cut::Traditional] {
        let blob = fontique::Blob::new(std::sync::Arc::new(cut.data()));
        families.extend(collection.register_fonts(blob, None).into_iter().map(|(family, _)| family));
    }
    for script in ["Hani", "Hira", "Kana"] {
        collection.append_fallbacks(fontique::FallbackKey::new(fontique::Script::from_str_unchecked(script), None), families.iter().copied());
    }
    collection.register_fonts(fontique::Blob::new(std::sync::Arc::new(nettai_render::bundled::m_plus_1_code())), None);
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

    /// The system's locale names a bundled language by its primary subtag, a Chinese one by its script or its
    /// region (Taiwan's, Hong Kong's and Macau's Traditional, the rest Simplified); one not bundled is English.
    #[test]
    fn the_systems_language() {
        assert_eq!(of_locale("ja-JP"), "ja");
        assert_eq!(of_locale("zh-CN"), "zh-Hans");
        assert_eq!(of_locale("zh-Hans-CN"), "zh-Hans");
        assert_eq!(of_locale("zh_SG"), "zh-Hans");
        assert_eq!(of_locale("zh"), "zh-Hans");
        assert_eq!(of_locale("zh-TW"), "zh-Hant");
        assert_eq!(of_locale("zh-Hant-HK"), "zh-Hant");
        assert_eq!(of_locale("zh_MO"), "zh-Hant");
        assert_eq!(of_locale("en-US"), "en");
        assert_eq!(of_locale("fr-FR"), "en");
        assert!(cjk("zh-Hans") && cjk("zh-Hant") && cjk("ja") && !cjk("en"));
    }
}
