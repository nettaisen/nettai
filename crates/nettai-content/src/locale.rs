//! A game pack's display text: `locales/<lang>.toml`, one table a
//! language, by definition id (docs/design/text-rendering.md §10; the
//! engine's `content::strings`; docs/design/content-model-v2.md §4.0).
//!
//! A definition holds no display text. A chip's name and description, a
//! navi's name and no-running message, a Cross's name and description and
//! a patch card's name are its language's table's. The
//! content's own language (`OWN`, English for BN6) is part of the content:
//! the loader puts its table in `Content::strings`, and the define phase
//! counts what the battle reads of it (a description's lines, a message's
//! characters per line).
//! Another language's table is a frontend's alone (`--lang`): it changes
//! neither the battle nor `Content::hash()`, so each player reads their own
//! language in one netbattle.
//!
//! ```toml
//! language = "ja"
//!
//! [chips]
//! "cannon" = { name = "キャノン", description = "..." }
//!
//! [navis]
//! "megaman" = { name = "ロックマン", run_message = "..." }
//!
//! [forms]
//! "heatcross" = { name = "...", description = "..." }
//!
//! [patch-cards]
//! "canodumb" = { name = "..." }
//! ```
//!
//! A table names its game's definitions by their ids, as the game's
//! modules do (local to the game: `cannon`; docs/design/rules-in-luau.md,
//! the namespace).
//!
//! A line break in a description or a message is `\n`. A translated
//! description may have another number of lines than the own language's:
//! the battle keeps the own language's timing. The game's marks are
//! characters (Ⓐ, Ⓑ; the Private Use Area's for the stacked EX and SP,
//! `"\uE002"` in a table: docs/design/text-rendering.md §10.5).
//!
//! What reads them: a chip's name and description, a navi's name and
//! no-running message (the battle's screens); a Cross's description (R in
//! the Cross window) and its name (a frontend's own text: live play's
//! Crosses, the plain-text screen); a patch card's name is for its menu
//! (and the frontend's `--cards`' messages). Nothing shows another form's name or a weapon's, so a table has
//! none.

pub use nettai_battle::content::strings::{ChipStrings, FormStrings, NaviStrings, PatchCardStrings, Strings};
use nettai_battle::content::Defs;
use std::path::{Path, PathBuf};

/// The directory of a pack that holds its tables.
pub const DIR: &str = "locales";

/// The content's own language: its table is the content's words.
pub const OWN: &str = "en";

/// The table of `lang` in pack folder `pack` (content/bn6).
pub fn path(pack: &Path, lang: &str) -> PathBuf {
    pack.join(DIR).join(format!("{lang}.toml"))
}

/// Parse a table (`file` names it in messages).
pub fn parse(text: &str, file: &str) -> Result<Strings, String> {
    toml::from_str(text).map_err(|e| format!("{file}: {e}"))
}

/// Pack folder `pack`'s table of `lang`; `None` when it has none.
pub fn load(pack: &Path, lang: &str) -> Result<Option<Strings>, String> {
    let path = path(pack, lang);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let s = parse(&text, &path.display().to_string())?;
    if s.language != lang {
        return Err(format!("{} says it is {:?}", path.display(), s.language));
    }
    Ok(Some(s))
}

/// The tables of `lang` of packs `packs` of content `dir`, merged (a
/// frontend's other language for what it loaded); `None` when none has
/// one.
pub fn load_for(dir: &Path, packs: &[String], lang: &str) -> Result<Option<Strings>, String> {
    let mut out: Option<Strings> = None;
    for p in packs {
        if let Some(s) = load(&dir.join(p), lang)? {
            out.get_or_insert_with(|| Strings { language: lang.to_string(), ..Default::default() }).merge(s);
        }
    }
    Ok(out)
}

/// The languages pack folder `pack` has tables of.
pub fn languages_of(pack: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(pack.join(DIR)) else { return Vec::new() };
    let mut out: Vec<String> =
        entries.flatten().filter_map(|e| e.path().file_name()?.to_str()?.strip_suffix(".toml").map(String::from)).collect();
    out.sort();
    out
}

/// The languages any pack of content `dir` has tables of.
pub fn languages(dir: &Path) -> Vec<String> {
    let packs = crate::index::manifests(dir).map(|m| m.into_keys().collect::<Vec<_>>()).unwrap_or_default();
    let mut out: Vec<String> = packs.iter().flat_map(|p| languages_of(&dir.join(p))).collect();
    out.sort();
    out.dedup();
    out
}

/// What is wrong with a game's table against its definitions: an id no
/// definition has, a string with a combining mark (write the composed
/// character); and in the own language's (`own`), a chip or navi without a
/// name, which a frontend would show by its id. (Which forms' strings
/// something shows is a game's: BN6's Crosses', which nettai-match's BN6
/// tests check.) (A string may be empty: the invalid chip's name is, and
/// the Japanese games print no description for some chips.)
pub fn check(s: &Strings, defs: &Defs, own: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut text = |what: String, v: &Option<String>| {
        if let Some(v) = v {
            if let Some(c) = v.chars().find(|&c| matches!(c, '\u{0300}'..='\u{036F}' | '\u{3099}' | '\u{309A}')) {
                out.push(format!("{what} has a combining mark ({c:?}): write the composed character"));
            }
        }
    };
    let mut unknown = Vec::new();
    for (key, c) in &s.chips {
        if defs.chip_by_key(key).is_none() {
            unknown.push(format!("chips.{key}: no chip has this key"));
        }
        text(format!("chips.{key}.name"), &c.name);
        text(format!("chips.{key}.description"), &c.description);
    }
    for (key, n) in &s.navis {
        if defs.navi_by_key(key).is_none() {
            unknown.push(format!("navis.{key}: no navi has this key"));
        }
        text(format!("navis.{key}.name"), &n.name);
        text(format!("navis.{key}.run_message"), &n.run_message);
    }
    for (key, f) in &s.forms {
        if defs.form_by_key(key).is_none() {
            unknown.push(format!("forms.{key}: no form has this key"));
        }
        text(format!("forms.{key}.name"), &f.name);
        text(format!("forms.{key}.description"), &f.description);
    }
    for (key, c) in &s.patch_cards {
        if defs.patch_card_by_key(key).is_none() {
            unknown.push(format!("patch-cards.{key}: no patch card has this key"));
        }
        text(format!("patch-cards.{key}.name"), &c.name);
    }
    for (key, c) in &s.navicust_programs {
        if defs.navicust_program_by_key(key).is_none() {
            unknown.push(format!("navicust-programs.{key}: no NaviCust program has this key"));
        }
        text(format!("navicust-programs.{key}.name"), &c.name);
    }
    if own {
        let named = |n: Option<&Option<String>>| n.is_some_and(|n| n.is_some());
        for d in &defs.chips {
            if !named(s.chip(&d.key).map(|c| &c.name)) {
                unknown.push(format!("chips.{}: the content's own language names every chip", d.key));
            }
        }
        for d in &defs.navis {
            if !named(s.navi(&d.key).map(|n| &n.name)) {
                unknown.push(format!("navis.{}: the content's own language names every navi", d.key));
            }
        }
        for d in &defs.patch_cards {
            if !named(s.patch_card(&d.key).map(|c| &c.name)) {
                unknown.push(format!("patch-cards.{}: the content's own language names every patch card", d.key));
            }
        }
        for d in &defs.navicust_programs {
            if !named(s.navicust_program(&d.key).map(|c| &c.name)) {
                unknown.push(format!("navicust-programs.{}: the content's own language names every NaviCust program", d.key));
            }
        }
    }
    unknown.extend(out);
    unknown
}

/// Check the tables of each game `c` loaded from content `dir` against
/// `c`: each language's table of the game's pack (a chip its manifest
/// leaves unported, `chips/<key>/chip`, may have its strings before its
/// use), and the own language's present.
pub fn check_games(dir: &Path, c: &nettai_battle::Content, r: &mut crate::report::Report) {
    for game in c.scripts.games() {
        let pack = dir.join(&game);
        let unported: std::collections::BTreeSet<String> = c
            .scripts
            .manifest(&game)
            .map(|m| m.definitions.unported.iter())
            .into_iter()
            .flatten()
            .filter_map(|p| Some(p.strip_prefix("chips/")?.strip_suffix("/chip")?.to_string()))
            .collect();
        let langs = languages_of(&pack);
        if !langs.iter().any(|l| l == OWN) {
            r.error(format!("{game}/{DIR}/{OWN}.toml"), "the game's own words are missing");
        }
        for lang in langs {
            let at = format!("{game}/{DIR}/{lang}.toml");
            match load(&pack, &lang) {
                Ok(Some(mut s)) => {
                    s.chips.retain(|k, _| !unported.contains(k));
                    for problem in check(&s, &c.defs, lang == OWN) {
                        r.error(&at, problem);
                    }
                }
                Ok(None) => {}
                Err(e) => r.error(&at, e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_parses_and_refuses_unknown_fields() {
        let s = parse(
            "language = \"ja\"\n[chips]\ncannon = { name = \"キャノン\", description = \"a\\nb\" }\n[navis]\nmegaman = { name = \"ロックマン\" }\n",
            "ja.toml",
        )
        .unwrap();
        assert_eq!(s.chip("cannon").and_then(|c| c.name.as_deref()), Some("キャノン"));
        assert_eq!(s.chip("cannon").and_then(|c| c.description.as_deref()), Some("a\nb"));
        assert_eq!(s.navi("megaman").and_then(|n| n.run_message.as_deref()), None);
        assert!(parse("language = \"ja\"\n[chips]\ncannon = { nmae = \"x\" }\n", "ja.toml").is_err());
    }
}
