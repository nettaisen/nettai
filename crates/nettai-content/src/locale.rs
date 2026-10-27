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
//! A game's text tables, key to text that no definition owns, are its
//! top-level tables that are no collection of its root ([`text_tables`]),
//! which tools show (EXE6's and EXE5's `patch_card_effects`: a patch card's
//! effects as its menu lists them, by the effect's kind and choice,
//! `{field}` the effect's number of that name; the build creator's patch card list
//! shows them).

pub use nettai_battle::content::strings::{BackgroundStrings, ChipStrings, EntryStrings, FormStrings, NaviStrings, Strings, Table};
use nettai_battle::content::Defs;
use std::path::{Path, PathBuf};

/// The directory of a pack that holds its tables.
pub const DIR: &str = "locales";

/// A game's text table of its own words: `name`, the game's name as a player sees it in the language (BN6 in
/// English, EXE6 in Japanese).
pub const GAME: &str = "game";

/// The key of [`GAME`] that is the game's name.
pub const GAME_NAME: &str = "name";

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
    if !nettai_platform::fs::is_file(&path) {
        return Ok(None);
    }
    let text = nettai_platform::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
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
    crate::pack::read_a_part();
    let mut out: Option<Strings> = None;
    for p in packs {
        if let Some(s) = load(&dir.join(p), lang)? {
            out.get_or_insert_with(|| Strings { language: lang.to_string(), ..Default::default() }).merge(s);
        }
    }
    Ok(out)
}

/// Game `game`'s name as a player sees it in `lang`, from its locales in content folder `dir` ([`GAME`]'s
/// [`GAME_NAME`]): its own language's where `lang` has no table or says none. Read from the content alone, so a
/// game whose asset pack is missing or can't be loaded has its name too. None where neither table says one, or
/// they can't be read.
pub fn game_name(dir: &Path, game: &str, lang: &str) -> Option<String> {
    // (Only the game's own words: the rest of the table isn't read into anything.)
    #[derive(serde::Deserialize)]
    struct Words {
        #[serde(default, rename = "game")]
        own: std::collections::BTreeMap<String, String>,
    }
    let read = |lang: &str| -> Option<String> {
        let text = nettai_platform::fs::read_to_string(path(&dir.join(game), lang)).ok()?;
        let words: Words = toml::from_str(&text).ok()?;
        words.own.get(GAME_NAME).cloned()
    };
    read(lang).or_else(|| read(OWN))
}

/// Game `game`'s `[backgrounds]` in `lang` (none: the table can't be read),
/// from the locales of content `dir` alone: each background's name there,
/// by its name in the game's pack.
fn backgrounds_in(dir: &Path, game: &str, lang: &str) -> Option<std::collections::BTreeMap<String, BackgroundStrings>> {
    // (Only the backgrounds: the rest of the table isn't read into anything.)
    #[derive(serde::Deserialize)]
    struct Backgrounds {
        #[serde(default)]
        backgrounds: std::collections::BTreeMap<String, BackgroundStrings>,
    }
    let text = nettai_platform::fs::read_to_string(path(&dir.join(game), lang)).ok()?;
    toml::from_str::<Backgrounds>(&text).ok().map(|b| b.backgrounds)
}

/// The background pool of content `dir` (docs/app.md §1, "The battle's
/// background"): every background of each of its games, each by its game
/// and its name in its game's pack, as the game's own language's
/// `[backgrounds]` names them (that table names every background the
/// game's pack has, and none it hasn't: [`check_backgrounds`]). Read from
/// the content alone, so it is the same whatever packs are installed; a
/// game whose table can't be read has none in it.
pub fn background_pool(dir: &Path) -> nettai_battle::content::BackgroundPool {
    let games = crate::index::games(dir).unwrap_or_default();
    background_pool_of(&games.into_iter().map(|game| (game, dir.to_path_buf())).collect::<Vec<_>>())
}

/// The background pool of `games`, each a game and the content directory its locales are read from (an app whose
/// games each have content of their own: a bundle each, docs/design/content-deployment.md §5).
pub fn background_pool_of(games: &[(String, std::path::PathBuf)]) -> nettai_battle::content::BackgroundPool {
    nettai_battle::content::BackgroundPool::new(
        games.iter().flat_map(|(game, dir)| backgrounds_in(dir, game, OWN).unwrap_or_default().into_keys().map(move |name| (game.clone(), name))),
    )
}

/// Game `game`'s backgrounds' names as a player sees them in `lang` (each
/// the area it comes from, as the game's menus name it), by their names in
/// the game's pack: `lang`'s, else the content's own language's. From the
/// locales of content `dir` alone (a game without its pack has them too).
pub fn background_names(dir: &Path, game: &str, lang: &str) -> std::collections::BTreeMap<String, String> {
    let read = |lang: &str| -> std::collections::BTreeMap<String, String> {
        backgrounds_in(dir, game, lang).unwrap_or_default().into_iter().filter_map(|(k, b)| Some((k, b.name?))).collect()
    };
    let mut out = read(OWN);
    if lang != OWN {
        out.extend(read(lang));
    }
    out
}

/// Game `game`'s text table `table` (the battle themes' names,
/// [`BATTLE_THEMES`]) in `lang`, from the locales of content `dir`
/// alone (a game without its pack has it too): each key's text in `lang`,
/// else in the content's own language. Empty where neither has the table.
pub fn text_table(dir: &Path, game: &str, lang: &str, table: &str) -> std::collections::BTreeMap<String, String> {
    let read = |lang: &str| -> std::collections::BTreeMap<String, String> {
        let Ok(text) = nettai_platform::fs::read_to_string(path(&dir.join(game), lang)) else { return Default::default() };
        let Ok(toml::Value::Table(mut tables)) = toml::from_str::<toml::Value>(&text) else { return Default::default() };
        match tables.remove(table) {
            Some(toml::Value::Table(t)) => t.into_iter().filter_map(|(k, v)| Some((k, v.as_str()?.to_string()))).collect(),
            _ => Default::default(),
        }
    };
    let mut out = read(OWN);
    if lang != OWN {
        out.extend(read(lang));
    }
    out
}

/// The languages pack folder `pack` has tables of.
pub fn languages_of(pack: &Path) -> Vec<String> {
    let Ok(entries) = nettai_platform::fs::read_dir(pack.join(DIR)) else { return Vec::new() };
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

/// The text tables of a game's tables `tables` (its languages'): each
/// top-level table of key to text that is no collection of the game's root
/// (`defs`), by name, in name order, whichever language has it.
pub fn text_tables<'a>(tables: impl IntoIterator<Item = &'a Strings>, defs: &Defs) -> Vec<String> {
    let collections = defs.collections();
    let text = tables.into_iter().flat_map(|s| &s.tables).filter(|(n, t)| matches!(t, Table::Text(_)) && !collections.contains(&n.as_str()));
    let mut out: Vec<String> = text.map(|(n, _)| n.clone()).collect();
    out.sort();
    out.dedup();
    out
}

/// What is wrong with a game's table against its definitions and its text
/// tables (`text_tables`, every one its languages have: [`text_tables`]):
/// an id no definition has, a collection's table of text or a text table of
/// entries, a string with a combining mark (write the composed character);
/// and in the own language's (`own`), a chip or navi without a name, which
/// a frontend would show by its id, and a text table the game's other
/// languages have and it hasn't. (Which forms' strings something shows is a
/// game's: EXE6's Crosses', which nettai-match's EXE6 tests check.) (A
/// string may be empty: the invalid chip's name is, and the Japanese games
/// print no description for some chips.)
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
    // (The game's own tables: each a collection of its root, else a text
    // table.)
    let collections = defs.collections();
    for (name, table) in &s.tables {
        let collection = collections.contains(&name.as_str());
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
            Table::Text(_) if collection => unknown.push(format!("{name}: a collection's table, whose entries are tables (`{{ name = ... }}`), not text")),
            Table::Text(lines) => {
                for (key, line) in lines {
                    text(format!("{name}.{key}"), &Some(line.clone()));
                }
            }
            Table::Entries(_) => unknown.push(format!(
                "{name}: the game's root holds no collection of this name, and a text table is key to text, not entries"
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
            if !s.tables.contains_key(t) {
                unknown.push(format!("{t}: the game's other languages have this text table, which the content's own language hasn't"));
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

/// The text table that names a game's battle themes (its rules'
/// `battle_themes`), by their songs' asset names.
pub const BATTLE_THEMES: &str = "battle_themes";

/// What is wrong with a game's table's battle themes' names
/// ([`BATTLE_THEMES`]) against the themes its rules state (`themes`, their
/// songs' asset names): a name of a song that is no theme, and in the own
/// language's (`own`), a theme without a name, which a list would show by
/// its asset name.
pub fn check_battle_themes(s: &Strings, themes: &[&str], own: bool) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(Table::Text(names)) = s.tables.get(BATTLE_THEMES) {
        for name in names.keys() {
            if !themes.contains(&name.as_str()) {
                out.push(format!("{BATTLE_THEMES}.{name}: the rules' battle_themes have no song of this name"));
            }
        }
    }
    if own {
        for name in themes {
            if s.text(BATTLE_THEMES, name).is_none() {
                out.push(format!("{BATTLE_THEMES}.{name}: the content's own language names every battle theme"));
            }
        }
    }
    out
}

/// The text table that names a game's emotions a navi can be held in (its
/// rules' `EmotionRules::holdable`), by name: the app's practice menu
/// offers them.
pub const EMOTIONS: &str = "emotions";

/// What is wrong with a game's table's emotions' names ([`EMOTIONS`])
/// against those its rules can hold a navi in (`emotions`): a name of none
/// of them, and in the own language's (`own`), one without a name, which a
/// list would show by its name in the rules.
pub fn check_emotions(s: &Strings, emotions: &[&str], own: bool) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(Table::Text(names)) = s.tables.get(EMOTIONS) {
        for name in names.keys() {
            if !emotions.contains(&name.as_str()) {
                out.push(format!("{EMOTIONS}.{name}: the rules can hold a navi in no emotion of this name"));
            }
        }
    }
    if own {
        for name in emotions {
            if s.text(EMOTIONS, name).is_none() {
                out.push(format!("{EMOTIONS}.{name}: the content's own language names every emotion a navi can be held in"));
            }
        }
    }
    out
}

/// Check the tables of each game `c` loaded from content `dir` against
/// `c`: each language's table of the game's pack (a chip the game's
/// init.luau doesn't require yet, its folder `chips/<key>/` there without
/// a use, may have its strings before it loads), and the own language's
/// present; the backgrounds' against the backgrounds of the game's asset
/// pack, whose index is `pack`. With no pack, only that the own language
/// names every background the content's modules name (whether the pack has
/// the others the tables name is the pack's to say).
pub fn check_games(dir: &Path, c: &nettai_battle::Content, pack: Option<&nettai_content_api::PackIndex>, r: &mut crate::report::Report) {
    use nettai_content_api::{AssetKind, AssetNames};
    let own_names = load(&dir.join(c.game()), OWN).ok().flatten().map(|s| s.backgrounds.keys().cloned().collect::<Vec<_>>()).unwrap_or_default();
    let backgrounds: Vec<&str> = match pack {
        Some(p) => p.backgrounds.keys().map(String::as_str).filter(|n| !AssetNames::is_placeholder(AssetKind::Background, n)).collect(),
        None => c.assets.backgrounds.iter().chain(&own_names).map(String::as_str).collect(),
    };
    let sounds = c.assets.names(AssetKind::Sound);
    let themes: Vec<&str> = c.battle_themes().iter().filter_map(|t| sounds.get(t.0 as usize).copied()).collect();
    let holdable = c.rules.as_ref().map(|r| r.emotion.holdable().into_iter().map(|(e, _)| r.emotion.name(e)).collect::<Vec<_>>()).unwrap_or_default();
    for game in c.scripts.games() {
        let pack = dir.join(&game);
        // (A chip folder whose main module didn't load.)
        let unported = |key: &str| {
            let module = format!("chips/{key}/init");
            nettai_platform::fs::is_file(pack.join(format!("{module}.luau"))) && !c.scripts.modules.contains_key(&nettai_battle::content::Scripts::name(&game, &module))
        };
        let langs = languages_of(&pack);
        if !langs.iter().any(|l| l == OWN) {
            r.error(format!("{game}/{DIR}/{OWN}.toml"), "the game's own words are missing");
        }
        let tables: Vec<(String, Result<Option<Strings>, String>)> = langs.into_iter().map(|lang| (lang.clone(), load(&pack, &lang))).collect();
        let text_tables = text_tables(tables.iter().filter_map(|(_, s)| s.as_ref().ok()?.as_ref()), &c.defs);
        for (lang, table) in tables {
            let at = format!("{game}/{DIR}/{lang}.toml");
            match table {
                Ok(Some(mut s)) => {
                    s.chips.retain(|k, _| !unported(k));
                    for problem in check(&s, &c.defs, &text_tables, lang == OWN) {
                        r.error(&at, problem);
                    }
                    for problem in check_backgrounds(&s, &backgrounds, lang == OWN) {
                        r.error(&at, problem);
                    }
                    for problem in check_battle_themes(&s, &themes, lang == OWN) {
                        r.error(&at, problem);
                    }
                    for problem in check_emotions(&s, &holdable, lang == OWN) {
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

    /// Each game states its name in English and Japanese: in English the English games' (BN4, BN5, BN6), in
    /// Japanese the Japanese games' (EXE4, EXE5, EXE6). A language with no table of its own has the game's own
    /// language's.
    #[test]
    fn each_game_is_named_in_its_languages() {
        let dir = crate::index::content();
        let games = crate::index::games(&dir).unwrap();
        assert_eq!(games, ["exe4", "exe5", "exe6"]);
        for (game, en, ja) in [("exe4", "BN4", "EXE4"), ("exe5", "BN5", "EXE5"), ("exe6", "BN6", "EXE6")] {
            assert_eq!(game_name(&dir, game, "en").as_deref(), Some(en), "{game}");
            assert_eq!(game_name(&dir, game, "ja").as_deref(), Some(ja), "{game}");
            assert_eq!(game_name(&dir, game, "pseudo").as_deref(), Some(en), "{game}: no table of its own");
        }
        assert_eq!(game_name(&dir, "nowhere", "en"), None);
    }

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
    /// table (key to text), read as which its values are: one that is no
    /// collection of the game is a text table, and the own language has
    /// every text table the game's other languages have.
    #[test]
    fn a_text_table_is_any_table_no_collection_is() {
        let s = parse("language = \"en\"\n[patch_card_effects]\nhp_add = \"HP+{amount}\"\n[bugs]\nx = \"y\"\n", "en.toml").unwrap();
        assert_eq!(s.text("patch_card_effects", "hp_add"), Some("HP+{amount}"));
        let defs = Defs::default();
        let ja = parse("language = \"ja\"\n[builds]\na = \"b\"\n", "ja.toml").unwrap();
        let tables = text_tables([&s, &ja], &defs);
        assert_eq!(tables, ["bugs", "builds", "patch_card_effects"]);
        assert!(check(&s, &defs, &tables, false).is_empty());
        // (Own language: a text table another language has and it hasn't.)
        let problems = check(&s, &defs, &tables, true);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("builds: the game's other languages have this text table"), "{problems:?}");
        // (A table of entries that is no collection: no text table either.)
        let entries = parse("language = \"en\"\n[patch_card_efects]\nx = { name = \"y\" }\n", "en.toml").unwrap();
        let problems = check(&entries, &defs, &text_tables([&entries], &defs), false);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("patch_card_efects: the game's root holds no collection of this name"), "{problems:?}");
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

    /// `[battle_themes]` names the rules' battle themes by their songs' asset names: a song that is no theme is
    /// refused, and the own language names every theme.
    #[test]
    fn the_battle_themes_are_the_rules() {
        let s = parse("language = \"en\"\n[battle_themes]\n\"virus-battle\" = \"Virus Battle\"\n\"winner\" = \"?\"\n", "en.toml").unwrap();
        assert_eq!(s.text(BATTLE_THEMES, "virus-battle"), Some("Virus Battle"));
        assert_eq!(
            check_battle_themes(&s, &["virus-battle", "boss-battle"], true),
            [
                "battle_themes.winner: the rules' battle_themes have no song of this name",
                "battle_themes.boss-battle: the content's own language names every battle theme"
            ]
        );
        assert_eq!(check_battle_themes(&s, &["virus-battle", "winner"], false), Vec::<String>::new());
    }

    /// `[emotions]` names the emotions the rules can hold a navi in: another
    /// is refused, and the own language names every one.
    #[test]
    fn the_emotions_are_the_rules() {
        let s = parse("language = \"en\"\n[emotions]\n\"angry\" = \"Anger\"\n\"soul\" = \"?\"\n", "en.toml").unwrap();
        assert_eq!(s.text(EMOTIONS, "angry"), Some("Anger"));
        assert_eq!(
            check_emotions(&s, &["angry", "worn_out"], true),
            [
                "emotions.soul: the rules can hold a navi in no emotion of this name",
                "emotions.worn_out: the content's own language names every emotion a navi can be held in"
            ]
        );
        assert_eq!(check_emotions(&s, &["angry", "soul"], false), Vec::<String>::new());
    }

    /// Each game's battle themes are named in English and in Japanese, and English names them after the game's
    /// name as the user named them.
    #[test]
    fn each_games_battle_themes_are_named() {
        let dir = crate::index::content();
        for (game, en) in [
            ("exe4", &["virus-battle", "boss-battle", "tournament-battle", "final-battle"][..]),
            ("exe5", &["virus-battle", "boss-battle", "final-battle"][..]),
            ("exe6", &["virus-battle", "boss-battle", "final-battle", "crossover-battle"][..]),
        ] {
            for lang in ["en", "ja"] {
                let s = load(&dir.join(game), lang).unwrap().unwrap();
                for song in en {
                    assert!(s.text(BATTLE_THEMES, song).is_some(), "{game} {lang}: {song}");
                }
            }
        }
        // (Read alone, as the app lists them: a language's, else the own language's.)
        let names = |game, lang| text_table(&dir, game, lang, BATTLE_THEMES);
        assert_eq!(names("exe4", "en")["tournament-battle"], "Tournament Battle");
        assert_eq!(names("exe4", "en")["final-battle"], "vs Duo");
        assert_eq!(names("exe5", "en")["final-battle"], "vs Nebula Gray");
        assert_eq!(names("exe6", "en")["final-battle"], "vs Cybeast");
        assert_eq!(names("exe6", "ja")["final-battle"], "VS 電脳獣");
        assert_eq!(names("exe6", "pseudo")["crossover-battle"], "Crossover Battle", "no table of its own");
        assert!(names("nowhere", "en").is_empty());
    }
}
