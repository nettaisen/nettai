//! A game pack's display text: `locales/<lang>.toml`, one table a
//! language, by definition id (docs/design/text-rendering.md §10; the
//! engine's `content::strings`; docs/design/content-model-v2.md §4.0).
//!
//! A definition holds no display text. A chip's name and description, a
//! navi's name and no-running message, a Cross's name and description and
//! a patch card's name are its language's table's. The
//! content's own language (`OWN`, English for EXE6) is part of the content:
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
//! [backgrounds]
//! "lans-hp" = { name = "熱斗のHP" }
//!
//! [patch-cards]
//! "canodumb" = { name = "..." }
//!
//! [navicust_programs]
//! "airshoes" = { name = "AirShoes", description = "Move\novr hole" }
//!
//! [patch_card_effects]
//! "hp_add" = "HP+{amount}"
//! ```
//!
//! A table names its game's definitions by their ids, as the game's
//! modules do (local to the game: `cannon`; docs/design/rules-in-luau.md,
//! the namespace); `[backgrounds]` names its pack's backgrounds by their
//! asset names (compat/assets.toml's), the name a player sees for each
//! (the app's arenas): the area whose maps draw it, as the game's menus name
//! it (docs/frontend.md §1).
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
//!
//! A game's text tables, key to text that no definition owns, are top-level
//! tables its manifest declares (`text = ["patch_card_effects"]`), which
//! tools show (EXE6's and EXE5's `patch_card_effects`: a patch card's
//! effects as its menu lists them, by the effect's kind and choice,
//! `{field}` the effect's number of that name; the build creator's patch card list
//! shows them).

pub use nettai_battle::content::strings::{BackgroundStrings, ChipStrings, EntryStrings, FormStrings, NaviStrings, Strings, Table};
use nettai_battle::content::Defs;
use std::path::{Path, PathBuf};

/// The directory of a pack that holds its tables.
pub const DIR: &str = "locales";

/// The content's own language: its table is the content's words.
pub const OWN: &str = "en";

/// The table of `lang` in pack folder `pack` (content/exe6).
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

/// What is wrong with a game's table against its definitions and the text
/// tables its manifest declares (`text_tables`): an id no definition has, a
/// top-level table that is no collection of the game and no declared text
/// table (a misspelt name), a string with a combining mark (write the
/// composed character); and in the own language's (`own`), a chip or navi
/// without a name, which a frontend would show by its id, and a declared
/// text table missing. (Which forms' strings
/// something shows is a game's: EXE6's Crosses', which nettai-match's EXE6
/// tests check.) (A string may be empty: the invalid chip's name is, and
/// the Japanese games print no description for some chips.)
pub fn check(s: &Strings, defs: &Defs, text_tables: &[String], own: bool) -> Vec<String> {
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
        text(format!("navis.{key}.variant_name"), &n.variant_name);
        text(format!("navis.{key}.run_message"), &n.run_message);
    }
    for (key, f) in &s.forms {
        if defs.form_by_key(key).is_none() {
            unknown.push(format!("forms.{key}: no form has this key"));
        }
        text(format!("forms.{key}.name"), &f.name);
        text(format!("forms.{key}.description"), &f.description);
    }
    // (The game's own tables: each a collection of its root, or a text
    // table its manifest declares.)
    let collections = defs.collections();
    for (name, table) in &s.tables {
        let collection = collections.contains(&name.as_str());
        let declared = text_tables.iter().any(|t| t == name);
        match table {
            Table::Entries(entries) if collection => {
                for (id, e) in entries {
                    if defs.entry_in(name, id).is_none() {
                        unknown.push(format!("{name}.{id}: no entry of {name} has this id"));
                    }
                    text(format!("{name}.{id}.name"), &e.name);
                    text(format!("{name}.{id}.description"), &e.description);
                }
            }
            Table::Text(lines) if declared => {
                for (key, line) in lines {
                    text(format!("{name}.{key}"), &Some(line.clone()));
                }
            }
            Table::Text(_) if collection => unknown.push(format!("{name}: a collection's table, whose entries are tables (`{{ name = ... }}`), not text")),
            Table::Entries(_) if declared => unknown.push(format!("{name}: a text table, key to text, not entries")),
            _ => unknown.push(format!(
                "{name}: no table of this name (the game's root holds no such collection, and its manifest declares no such text table: {})",
                if text_tables.is_empty() { "none".to_string() } else { text_tables.join(", ") }
            )),
        }
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
        for e in &defs.entries {
            if !named(s.entry(&e.collection, e.id()).map(|c| &c.name)) {
                unknown.push(format!("{}.{}: the content's own language names every entry of the game's collections", e.collection, e.id()));
            }
        }
        for t in text_tables {
            if !matches!(s.tables.get(t), Some(Table::Text(_))) {
                unknown.push(format!("{t}: the game's manifest declares this text table, which the content's own language hasn't"));
            }
        }
    }
    unknown.extend(out);
    unknown
}

/// What is wrong with a game's table's `[backgrounds]` against the
/// backgrounds its pack has (`backgrounds`, their asset names; its
/// placeholders, `background-0c`, none of them): a name the pack hasn't, a
/// string with a combining mark, and in the own language's (`own`), a
/// background without a name, which a frontend would show by its asset name.
pub fn check_backgrounds(s: &Strings, backgrounds: &[&str], own: bool) -> Vec<String> {
    let mut out = Vec::new();
    for (name, b) in &s.backgrounds {
        if !backgrounds.contains(&name.as_str()) {
            out.push(format!("backgrounds.{name}: the pack has no background of this name"));
        }
        if let Some(c) = b.name.as_deref().and_then(|v| v.chars().find(|&c| matches!(c, '\u{0300}'..='\u{036F}' | '\u{3099}' | '\u{309A}'))) {
            out.push(format!("backgrounds.{name}.name has a combining mark ({c:?}): write the composed character"));
        }
    }
    if own {
        for name in backgrounds {
            if s.background(name).and_then(|b| b.name.as_ref()).is_none() {
                out.push(format!("backgrounds.{name}: the content's own language names every background of the pack"));
            }
        }
    }
    out
}

/// Check the tables of each game `c` loaded from content `dir` against
/// `c`: each language's table of the game's pack (a chip the game's
/// init.luau doesn't require yet, its folder `chips/<key>/` there without
/// a use, may have its strings before it loads), and the own language's
/// present; the backgrounds' against the content's assets' (its pack's, or
/// on made-up assets those its modules name).
pub fn check_games(dir: &Path, c: &nettai_battle::Content, r: &mut crate::report::Report) {
    use nettai_content_api::{AssetKind, AssetNames};
    let backgrounds: Vec<&str> =
        c.assets.names(AssetKind::Background).into_iter().filter(|n| !AssetNames::is_placeholder(AssetKind::Background, n)).collect();
    for game in c.scripts.games() {
        let pack = dir.join(&game);
        let text_tables = c.scripts.manifest(&game).map(|m| m.text.clone()).unwrap_or_default();
        // (A chip folder whose main module didn't load.)
        let unported = |key: &str| {
            let module = format!("chips/{key}/init");
            pack.join(format!("{module}.luau")).is_file() && !c.scripts.modules.contains_key(&nettai_battle::content::Scripts::name(&game, &module))
        };
        let langs = languages_of(&pack);
        if !langs.iter().any(|l| l == OWN) {
            r.error(format!("{game}/{DIR}/{OWN}.toml"), "the game's own words are missing");
        }
        for lang in langs {
            let at = format!("{game}/{DIR}/{lang}.toml");
            match load(&pack, &lang) {
                Ok(Some(mut s)) => {
                    s.chips.retain(|k, _| !unported(k));
                    for problem in check(&s, &c.defs, &text_tables, lang == OWN) {
                        r.error(&at, problem);
                    }
                    for problem in check_backgrounds(&s, &backgrounds, lang == OWN) {
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

    /// A game's top-level table is a collection's (entries) or a text
    /// table (key to text), read as which its values are; the check takes
    /// a text table only where the manifest declares it, so a misspelt name
    /// is refused, not a new table.
    #[test]
    fn a_text_table_is_declared() {
        let s = parse("language = \"en\"\n[patch_card_effects]\nhp_add = \"HP+{amount}\"\n[patch_card_efects]\nx = \"y\"\n", "en.toml").unwrap();
        assert_eq!(s.text("patch_card_effects", "hp_add"), Some("HP+{amount}"));
        let defs = Defs::default();
        let problems = check(&s, &defs, &["patch_card_effects".to_string()], false);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("patch_card_efects: no table of this name"), "{problems:?}");
        let undeclared = check(&s, &defs, &[], false);
        assert_eq!(undeclared.len(), 2, "{undeclared:?}");
        // (Own language: a declared table it hasn't.)
        let none = parse("language = \"en\"\n", "en.toml").unwrap();
        assert_eq!(check(&none, &defs, &["patch_card_effects".to_string()], true).len(), 1);
    }

    /// `[backgrounds]` names the pack's backgrounds by their asset names:
    /// a name the pack hasn't is refused, and the own language names every
    /// one the pack has.
    #[test]
    fn the_backgrounds_are_the_packs() {
        let s = parse("language = \"en\"\n[backgrounds]\n\"lans-hp\" = { name = \"Lan's HP\" }\n\"lan-hp\" = { name = \"?\" }\n", "en.toml").unwrap();
        assert_eq!(s.background("lans-hp").and_then(|b| b.name.as_deref()), Some("Lan's HP"));
        assert_eq!(
            check_backgrounds(&s, &["lans-hp", "comp"], true),
            ["backgrounds.lan-hp: the pack has no background of this name", "backgrounds.comp: the content's own language names every background of the pack"]
        );
        assert_eq!(check_backgrounds(&s, &["lans-hp", "comp", "lan-hp"], false), Vec::<String>::new());
    }
}
