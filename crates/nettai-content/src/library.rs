//! A game pack's library order: `library.toml`, the order the game's own
//! library screens list its chips in, by category, and the order of the
//! entries of each of the game's collections (EXE6's `navicust_programs`
//! and `patch_cards`), by their ids there. Menus list things in
//! it (the editor's chip, program and card lists); the battle reads none of
//! it, so it stays out of the content's hash. The verification workspace's
//! generators write it from the ROMs and their checks compare it with them.
//!
//! ```toml
//! navicust_programs = [
//!     "superarmor",
//! ]
//! patch_cards = [
//!     "canodumb",
//! ]
//!
//! [chips]
//! standard = [
//!     "cannon",
//!     "hicannon",
//! ]
//! mega = [
//!     "roll",
//! ]
//! ```
//!
//! The chip sections are the game's library tabs, in the order its library
//! screen has them (the file's: the loader keeps the `chips` table's key
//! order): `standard`, `mega`, `giga`, `secret`, `program_advance`
//! and, in EXE5, `dark`. Where the versions of a game list different chips
//! at the same places (EXE6's Gregar and Falzar, EXE5's Team ProtoMan and
//! Team Colonel), the first version's come first. A standard, mega, giga or
//! program advance section holds chips of that class, the dark section dark
//! chips, the secret section any. A chip with no place in any library (a
//! navi's own chip, a chip only a rule uses) is in none: a menu lists those
//! after the rest, in key order.

use nettai_battle::content::{ChipClass, ChipFlags, Defs};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The file in a game pack's folder.
pub const FILE: &str = "library.toml";

/// A library category: one of the game's library tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Standard,
    Mega,
    Giga,
    Secret,
    ProgramAdvance,
    Dark,
}

impl Section {
    /// The names `library.toml` writes, by section.
    pub const NAMES: [(Section, &str); 6] = [
        (Section::Standard, "standard"),
        (Section::Mega, "mega"),
        (Section::Giga, "giga"),
        (Section::Secret, "secret"),
        (Section::ProgramAdvance, "program_advance"),
        (Section::Dark, "dark"),
    ];

    pub fn name(self) -> &'static str {
        Self::NAMES.iter().find(|(s, _)| *s == self).map(|(_, n)| *n).expect("every section has a name")
    }

    pub fn from_name(name: &str) -> Option<Section> {
        Self::NAMES.iter().find(|(_, n)| *n == name).map(|(s, _)| *s)
    }

    /// Whether a chip of class `class` and flags `flags` belongs in it.
    pub fn fits(self, class: ChipClass, flags: ChipFlags) -> bool {
        match self {
            Section::Standard => class == ChipClass::Standard,
            Section::Mega => class == ChipClass::Mega,
            Section::Giga => class == ChipClass::Giga,
            Section::ProgramAdvance => class == ChipClass::ProgramAdvance,
            Section::Dark => flags.0 & ChipFlags::DARK != 0,
            Section::Secret => true,
        }
    }
}

/// A game's library order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Library {
    /// The chip sections, in the game's tab order, each its chips' keys in
    /// order.
    pub chips: Vec<(Section, Vec<String>)>,
    /// Each of the game's collections' entries' ids, in order, by the
    /// collection's name.
    pub collections: std::collections::BTreeMap<String, Vec<String>>,
}

/// A place in an order: compared, it sorts what has one in the order and
/// what has none after it.
pub type Rank = (usize, usize);

impl Library {
    /// The section and the place in it of chip `key`; none outside every
    /// section.
    pub fn chip_rank(&self, key: &str) -> Option<Rank> {
        self.chips.iter().enumerate().find_map(|(s, (_, keys))| keys.iter().position(|k| k == key).map(|i| (s, i)))
    }

    /// The section chip `key` is in.
    pub fn chip_section(&self, key: &str) -> Option<Section> {
        self.chip_rank(key).map(|(s, _)| self.chips[s].0)
    }

    /// An index of the chips' places, for sorting many lists.
    pub fn chip_ranks(&self) -> HashMap<&str, Rank> {
        let mut out = HashMap::new();
        for (s, (_, keys)) in self.chips.iter().enumerate() {
            for (i, k) in keys.iter().enumerate() {
                out.entry(k.as_str()).or_insert((s, i));
            }
        }
        out
    }

    /// The place of entry `id` in collection `collection`'s order.
    pub fn entry_rank(&self, collection: &str, id: &str) -> Option<Rank> {
        self.collections.get(collection)?.iter().position(|k| k == id).map(|i| (0, i))
    }
}

/// Sort `items` by `rank` of their keys: what has a place first, in its
/// order; what has none after it, by key.
pub fn sort_by_rank<'k, T>(items: &mut [T], key: impl Fn(&T) -> &'k str, rank: impl Fn(&str) -> Option<Rank>) {
    items.sort_by(|a, b| {
        let (ka, kb) = (key(a), key(b));
        match (rank(ka), rank(kb)) {
            (Some(x), Some(y)) => x.cmp(&y).then_with(|| ka.cmp(kb)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => ka.cmp(kb),
        }
    });
}

/// The file of pack folder `pack` (content/exe6).
pub fn path(pack: &Path) -> PathBuf {
    pack.join(FILE)
}

/// Parse a library order (`file` names it in messages): the lists above,
/// each of keys; the `chips` table's in the file's order (`toml_edit`
/// keeps a table's key order).
pub fn parse(text: &str, file: &str) -> Result<Library, String> {
    let doc: toml_edit::DocumentMut = text.parse().map_err(|e| format!("{file}: {e}"))?;
    let keys = |item: &toml_edit::Item, at: &str| -> Result<Vec<String>, String> {
        let list = item.as_array().ok_or_else(|| format!("{file}: `{at}` is not a list of keys"))?;
        list.iter().map(|v| v.as_str().map(String::from).ok_or_else(|| format!("{file}: `{at}` has {v}, not a key"))).collect()
    };
    let mut out = Library::default();
    for (k, item) in doc.iter() {
        match k {
            "chips" => {
                let table = item.as_table().ok_or_else(|| format!("{file}: `chips` is not a table of lists"))?;
                for (name, list) in table.iter() {
                    let s = Section::from_name(name).ok_or_else(|| {
                        let names: Vec<&str> = Section::NAMES.iter().map(|(_, n)| *n).collect();
                        format!("{file}: `chips.{name}` is none of the sections ({})", names.join(", "))
                    })?;
                    out.chips.push((s, keys(list, &format!("chips.{name}"))?));
                }
            }
            // (A collection of the game's root, by its name: the check
            // holds it to the root's.)
            collection => {
                out.collections.insert(collection.to_string(), keys(item, collection)?);
            }
        }
    }
    Ok(out)
}

/// Pack folder `pack`'s library order; `None` when it has none (its menus
/// list by key).
pub fn load(pack: &Path) -> Result<Option<Library>, String> {
    let path = path(pack);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    parse(&text, &path.display().to_string()).map(Some)
}

/// What is wrong with `lib` against the game's definitions `defs`: a key
/// no definition has, a key twice, a chip in a section its class (or, in
/// the dark section, its flags) doesn't fit.
pub fn check(lib: &Library, defs: &Defs) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen: HashMap<&str, String> = HashMap::new();
    for (section, keys) in &lib.chips {
        let at = format!("chips.{}", section.name());
        for key in keys {
            if let Some(first) = seen.insert(key, at.clone()) {
                out.push(format!("{at}: {key} is in {first} already"));
                continue;
            }
            let Some(h) = defs.chip_by_key(key) else {
                out.push(format!("{at}: no chip has the key {key}"));
                continue;
            };
            let record = &defs.chip(h).record;
            if !section.fits(record.class, record.flags) {
                let what = if *section == Section::Dark { "not a dark chip".to_string() } else { format!("a {:?} chip", record.class) };
                out.push(format!("{at}: {key} is {what}"));
            }
        }
    }
    for (collection, ids) in &lib.collections {
        if !defs.collections().contains(&collection.as_str()) {
            out.push(format!("{collection}: the game's root holds no such collection (its are chips and {})", defs.collections().join(", ")));
            continue;
        }
        let mut seen = std::collections::HashSet::new();
        for id in ids {
            if !seen.insert(id.as_str()) {
                out.push(format!("{collection}: {id} twice"));
            } else if defs.entry_in(collection, id).is_none() {
                out.push(format!("{collection}: nothing has the key {id}"));
            }
        }
    }
    out
}

/// Check the library order of each game `c` loaded from content `dir`
/// against `c`'s definitions.
pub fn check_games(dir: &Path, c: &nettai_battle::Content, r: &mut crate::report::Report) {
    for game in c.scripts.games() {
        let at = format!("{game}/{FILE}");
        match load(&dir.join(&game)) {
            Ok(Some(lib)) => {
                for problem in check(&lib, &c.defs) {
                    r.error(&at, problem);
                }
            }
            Ok(None) => {}
            Err(e) => r.error(&at, e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "navicust_programs = [\"superarmor\"]\npatch_cards = [\"canodumb\"]\n\n\
                        [chips]\nstandard = [\"cannon\", \"hicannon\"]\nmega = [\"roll\"]\n";

    #[test]
    fn an_order_parses_in_its_sections_order_and_ranks() {
        let lib = parse(TEXT, "library.toml").unwrap();
        assert_eq!(lib.chips.iter().map(|(s, _)| *s).collect::<Vec<_>>(), [Section::Standard, Section::Mega]);
        assert_eq!(lib.chip_rank("hicannon"), Some((0, 1)));
        assert_eq!(lib.chip_rank("roll"), Some((1, 0)));
        assert_eq!(lib.chip_rank("lance"), None);
        assert_eq!(lib.chip_section("roll"), Some(Section::Mega));
        assert_eq!(lib.collections["navicust_programs"], ["superarmor"]);
        assert_eq!(lib.collections["patch_cards"], ["canodumb"]);
        // Mega's section is first where the file says so.
        let swapped = parse("[chips]\nmega = [\"roll\"]\nstandard = [\"cannon\"]\n", "f").unwrap();
        assert_eq!(swapped.chip_rank("cannon"), Some((1, 0)));
    }

    #[test]
    fn a_sort_puts_what_has_no_place_after_by_key() {
        let lib = parse(TEXT, "library.toml").unwrap();
        let mut keys = vec!["zzz", "roll", "aaa", "hicannon", "cannon"];
        sort_by_rank(&mut keys, |k| *k, |k| lib.chip_rank(k));
        assert_eq!(keys, ["cannon", "hicannon", "roll", "aaa", "zzz"]);
    }

    #[test]
    fn an_order_refuses_what_it_doesnt_know() {
        assert!(parse("[chips]\nextra = []\n", "f").unwrap_err().contains("`chips.extra` is none of the sections"));
        assert!(parse("[chips]\nmega = \"roll\"\n", "f").unwrap_err().contains("`chips.mega` is not a list of keys"));
        assert!(parse("[chips]\nmega = [1]\n", "f").unwrap_err().contains("not a key"));
        // (Another key is a collection's order, which the check holds to the
        // game's root's.)
        assert!(parse("folders = 3\n", "f").unwrap_err().contains("`folders` is not a list of keys"));
        assert_eq!(parse("folders = [\"a\"]\n", "f").unwrap().collections["folders"], ["a"]);
    }
}
