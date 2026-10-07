//! Compatibility with EXE4's original games (Red Sun and Blue Moon, US and
//! Japanese): their numbers for EXE4's content (content/exe4/compat:
//! compat is per game), and what reads them.
//!
//! - [`Compat`]: the tables, content key to the original's numbers: the
//!   chips' ids, the names the extractor writes EXE4's assets under, the
//!   text encodings, the statuses, the netbattle stages, the object kinds,
//!   the navi actions, what NaviStats name by number, and where the other
//!   ROMs have what compat addresses. [`Compat::exe4`] is this repository's,
//!   built in.
//! - [`codec`]: EXE4's records in the engine's terms: the 0x40-byte
//!   NaviStats, the field's panels, the chip blocks, the link record.
//! - [`save`]: an EXE4 save file, and what a player's setup takes of it.
//! - `trace` (feature `trace`): the chip lab's EXE4 recordings, read,
//!   decoded and replayed (docs/design/exe4-map.md §17).
//!
//! The verification workspace's tools/exe4/gen_content.py and gen_rules.py
//! write the generated tables from the ROMs; kinds.toml, actions.toml,
//! records.toml and games.toml are written by hand from the code and the
//! recordings, as the port reaches them.
//!
//! Keys: the tables are keyed by EXE4's ids, local to the game (`cannon`),
//! as content writes them (docs/design/content-model-v2.md §4.0). The engine
//! never reads any of it (a test guards it).

pub mod codec;
pub mod save;
pub mod setup;
#[cfg(feature = "trace")]
pub mod trace;

use nettai_battle::field::PanelType;
use nettai_content_api::Pool;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// EXE4's game (content/exe4).
pub const ROOT: &str = "exe4";

/// An EXE4 game's version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Version {
    RedSun,
    BlueMoon,
}

impl Version {
    /// The version's name as compat, a pack and the recordings write it
    /// (`redsun`, `bluemoon`).
    pub fn name(self) -> &'static str {
        match self {
            Version::RedSun => "redsun",
            Version::BlueMoon => "bluemoon",
        }
    }

    /// The version a name names.
    pub fn named(name: &str) -> Option<Version> {
        match name {
            "redsun" => Some(Version::RedSun),
            "bluemoon" => Some(Version::BlueMoon),
            _ => None,
        }
    }
}

/// EXE4's object pools: how many slots each has (exe4-map.md §3.1: 8 actors,
/// 32 attacks, 32 effects).
pub fn pool_slots(pool: Pool) -> usize {
    match pool {
        Pool::Actor => 8,
        Pool::Attack => 32,
        Pool::Effect => 32,
    }
}

/// The pool of an object type number as the recordings print it (1, 3, 4).
pub fn pool_of_type(t: u8) -> Option<Pool> {
    match t {
        1 => Some(Pool::Actor),
        3 => Some(Pool::Attack),
        4 => Some(Pool::Effect),
        _ => None,
    }
}

/// The type number of a pool, as the recordings print it.
pub fn type_of_pool(p: Pool) -> u8 {
    match p {
        Pool::Actor => 1,
        Pool::Attack => 3,
        Pool::Effect => 4,
    }
}

/// A chip (chips.toml): its id (its record's number in the chip table), and
/// for a version's own giga chip its version (the other version's ROM has
/// its record, but no folder of that version holds it).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipEntry {
    pub id: u16,
    #[serde(default)]
    pub version: Option<Version>,
}

/// EXE4's asset names (assets.toml): the names the extractor writes its
/// assets under. Sprites as "cc-ii" (the category's byte offset in the
/// sprite list and the index), sounds by the song table's numbers, battle
/// backgrounds by the background loader's (0x08085430), banners by the
/// banner block's (0x02037CE0's +1), the emotion window's faces by the
/// number the extractor gives them (its exe4/hud.rs).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetNames {
    #[serde(default)]
    pub sprites: BTreeMap<String, String>,
    #[serde(default)]
    pub sounds: BTreeMap<String, u16>,
    #[serde(default)]
    pub backgrounds: BTreeMap<String, u8>,
    #[serde(default)]
    pub banners: BTreeMap<String, u8>,
    #[serde(default)]
    pub mugshots: BTreeMap<String, u8>,
}

/// EXE4's text encodings (text.toml): what each byte below `first_control`
/// draws (the US ROMs' fonts draw 0x00 to 0x6F), and the Japanese ROMs'
/// (`jp`): its glyphs below `first_control`, then the second page's
/// (`dialogue_glyphs`, E4 xx: glyph 0xE4 + xx).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub first_control: u8,
    pub glyphs: Vec<String>,
    #[serde(default)]
    pub dialogue_glyphs: Vec<String>,
    #[serde(default)]
    pub jp: Encoding,
}

/// The Japanese ROMs' encoding (`Text::jp`), in the same shape.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encoding {
    pub glyphs: Vec<String>,
    #[serde(default)]
    pub dialogue_glyphs: Vec<String>,
}

/// An EXE4 panel type (panels.toml): the flag word the game gives it
/// (0x0800A3A8), and the engine's panel type it is by content's name for it
/// (none: no stage of content's has it yet).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelEntry {
    #[serde(default)]
    pub name: Option<String>,
    pub flags: u32,
    #[serde(default)]
    pub engine: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelsFile {
    types: BTreeMap<String, PanelEntry>,
}

/// The original's numbers of EXE4's rule definitions (rules.toml).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleNumbers {
    /// Statuses (0x08018550): a hit's status byte, by key.
    #[serde(default)]
    pub statuses: BTreeMap<String, u8>,
}

/// A netbattle stage (stages.toml): the settings records that are it (their
/// numbers in the table, 0x080FC138), its panel layout's number (the
/// record's +1) and its actor list's address (+8, Red Sun US's: games.toml
/// has the other ROMs' offsets).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageEntry {
    pub settings: Vec<u8>,
    pub layout: u8,
    pub actor_list: u32,
}

/// An object kind (kinds.toml): EXE4's pool and index of it, and the
/// position bytes the comparison skips (as exe5-compat's).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindEntry {
    pub pool: String,
    pub index: u8,
    /// Its position is register garbage until its init places it.
    #[serde(default)]
    pub scratch_position: bool,
    /// Its position is garbage while it has no sprite (the charge glow
    /// before its first update).
    #[serde(default)]
    pub scratch_position_without_sprite: bool,
    /// The fraction of its Z is register garbage.
    #[serde(default)]
    pub scratch_z_fraction: bool,
}

/// Where a ROM other than Red Sun's US one has what compat names by that
/// ROM's addresses (games.toml, one section a ROM).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameAddresses {
    /// How far its battle settings records (and the stage actor lists their
    /// +8 names) are from Red Sun US's.
    pub settings: i32,
}

/// games.toml: the other three ROMs' [`GameAddresses`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Games {
    pub bluemoon: GameAddresses,
    #[serde(rename = "jp-redsun")]
    pub jp_redsun: GameAddresses,
    #[serde(rename = "jp-bluemoon")]
    pub jp_bluemoon: GameAddresses,
}

impl Games {
    /// The addresses of a version's ROM of a region (none: Red Sun's US
    /// one, compat's own).
    pub fn of(&self, version: Version, japanese: bool) -> Option<&GameAddresses> {
        match (version, japanese) {
            (Version::RedSun, false) => None,
            (Version::BlueMoon, false) => Some(&self.bluemoon),
            (Version::RedSun, true) => Some(&self.jp_redsun),
            (Version::BlueMoon, true) => Some(&self.jp_bluemoon),
        }
    }
}

/// What EXE4's NaviStats name by number (records.toml): navis by navi
/// number (+0x23), weapons by routine number (+0x09, +0x0A, +0x0C), MegaMan's
/// forms by soul number (+0x24: 0 his base form), auras by number (+0x21),
/// each by its key.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordNumbers {
    #[serde(default)]
    pub navis: BTreeMap<String, u8>,
    #[serde(default)]
    pub weapons: BTreeMap<String, Vec<u8>>,
    #[serde(default)]
    pub forms: BTreeMap<String, u8>,
    #[serde(default)]
    pub barriers: BTreeMap<String, u8>,
}

/// EXE4's compat tables (content/exe4/compat).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Compat {
    /// chips.toml: by key.
    pub chips: BTreeMap<String, ChipEntry>,
    /// The chips' keys by id.
    pub chip_keys: BTreeMap<u16, String>,
    /// assets.toml: the assets' names.
    pub assets: AssetNames,
    /// text.toml: the text encodings.
    pub text: Text,
    /// rules.toml: the rule definitions' numbers.
    pub rules: RuleNumbers,
    /// stages.toml: the netbattle stages, by key.
    pub stages: BTreeMap<String, StageEntry>,
    /// games.toml: where the other ROMs have what compat addresses.
    pub games: Games,
    /// kinds.toml: the object kinds' numbers, by key.
    pub kinds: BTreeMap<String, KindEntry>,
    /// actions.toml: the navi action numbers, by key.
    pub actions: BTreeMap<String, u8>,
    /// records.toml: what NaviStats name by number.
    pub records: RecordNumbers,
    /// panels.toml: by EXE4's panel type number.
    pub panels: BTreeMap<u8, PanelEntry>,
    /// navicust.toml: the NaviCust programs' numbers and colored variants.
    pub navicust: NaviCustNumbers,
}

/// EXE4's NaviCust programs (navicust.toml): each program's number (a part
/// id's high bits), by key, and its colored variants (a part id's low
/// bits) in the order of its definition's colors.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct NaviCustNumbers {
    #[serde(default)]
    pub programs: BTreeMap<String, u8>,
    #[serde(default)]
    pub variants: BTreeMap<String, Vec<u8>>,
}

/// EXE4's navi states, by their CurAction (the player's state table,
/// 0x080EAEFC: entry, take control, deletion, flinch, paralysis, drag, then
/// the navi's own from 6), as EXE5's: idle is 6.
const EXE4_STATES: [nettai_battle::kinds::player::NaviAction; 7] = {
    use nettai_battle::kinds::player::NaviAction::*;
    [Entry, TakeControl, Deletion, Flinch, Paralysis, Drag, Idle]
};

/// The files of a compat folder.
pub const FILES: [&str; 11] = [
    "chips.toml",
    "assets.toml",
    "text.toml",
    "rules.toml",
    "stages.toml",
    "games.toml",
    "kinds.toml",
    "actions.toml",
    "records.toml",
    "panels.toml",
    "navicust.toml",
];

/// This repository's compat (content/exe4/compat), built in.
const EXE4: [(&str, &str); 11] = [
    ("chips.toml", include_str!("../../../content/exe4/compat/chips.toml")),
    ("assets.toml", include_str!("../../../content/exe4/compat/assets.toml")),
    ("text.toml", include_str!("../../../content/exe4/compat/text.toml")),
    ("rules.toml", include_str!("../../../content/exe4/compat/rules.toml")),
    ("stages.toml", include_str!("../../../content/exe4/compat/stages.toml")),
    ("games.toml", include_str!("../../../content/exe4/compat/games.toml")),
    ("kinds.toml", include_str!("../../../content/exe4/compat/kinds.toml")),
    ("actions.toml", include_str!("../../../content/exe4/compat/actions.toml")),
    ("records.toml", include_str!("../../../content/exe4/compat/records.toml")),
    ("panels.toml", include_str!("../../../content/exe4/compat/panels.toml")),
    ("navicust.toml", include_str!("../../../content/exe4/compat/navicust.toml")),
];

/// The engine's panel type of an engine panel name as panels.toml writes
/// it (content's names for the engine's types).
fn engine_panel(name: &str) -> Option<PanelType> {
    Some(match name {
        "missing" => PanelType::Missing,
        "broken" => PanelType::Broken,
        "normal" => PanelType::Normal,
        "cracked" => PanelType::Cracked,
        "poison" => PanelType::Poison,
        "holy" => PanelType::Holy,
        "grass" => PanelType::Grass,
        "ice" => PanelType::Ice,
        "volcano" => PanelType::Volcano,
        "metal" => PanelType::Metal,
        "lava" => PanelType::Lava,
        "sea" => PanelType::Sea,
        "pitfall" => PanelType::Pitfall,
        "hole" => PanelType::Hole,
        _ => return None,
    })
}

/// A sprite's "cc-ii".
fn parse_sprite(id: &str) -> Option<(u8, u8)> {
    let (c, i) = id.split_once('-')?;
    Some((u8::from_str_radix(c, 16).ok()?, u8::from_str_radix(i, 16).ok()?))
}

/// The pool a kinds.toml entry names.
fn parse_pool(name: &str) -> Option<Pool> {
    match name {
        "actor" => Some(Pool::Actor),
        "attack" => Some(Pool::Attack),
        "effect" => Some(Pool::Effect),
        _ => None,
    }
}

impl Compat {
    /// EXE4's compat, as this repository's content/exe4/compat has it.
    pub fn exe4() -> &'static Compat {
        static EXE4_COMPAT: std::sync::OnceLock<Compat> = std::sync::OnceLock::new();
        EXE4_COMPAT.get_or_init(|| {
            Compat::parse(|file| Ok(EXE4.iter().find(|(f, _)| *f == file).map(|(_, t)| t.to_string()).unwrap_or_default()))
                .unwrap_or_else(|e| panic!("content/exe4/compat: {e}"))
        })
    }

    /// Read a compat folder.
    pub fn read(dir: &Path) -> Result<Compat, String> {
        Compat::parse(|file| {
            let path = dir.join(file);
            std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))
        })
    }

    fn parse(text: impl Fn(&str) -> Result<String, String>) -> Result<Compat, String> {
        let chips: BTreeMap<String, ChipEntry> = toml::from_str(&text("chips.toml")?).map_err(|e| format!("chips.toml: {e}"))?;
        let mut chip_keys = BTreeMap::new();
        for (k, c) in &chips {
            if let Some(other) = chip_keys.insert(c.id, k.clone()) {
                return Err(format!("chips.toml: {k} and {other} are both chip {:#05x}", c.id));
            }
        }
        let assets: AssetNames = toml::from_str(&text("assets.toml")?).map_err(|e| format!("assets.toml: {e}"))?;
        for (name, id) in &assets.sprites {
            parse_sprite(id).ok_or_else(|| format!("assets.toml: sprite {name} is {id:?}, not \"cc-ii\""))?;
        }
        let mut sounds = BTreeMap::new();
        for (name, &id) in &assets.sounds {
            if let Some(other) = sounds.insert(id, name) {
                return Err(format!("assets.toml: sounds {name} and {other} are both {id:#05x}"));
            }
        }
        let text_file: Text = toml::from_str(&text("text.toml")?).map_err(|e| format!("text.toml: {e}"))?;
        if text_file.glyphs.len() > text_file.first_control as usize || text_file.jp.glyphs.len() > text_file.first_control as usize {
            return Err(format!("text.toml: more glyphs than the bytes below first_control ({:#04x})", text_file.first_control));
        }
        let rules: RuleNumbers = toml::from_str(&text("rules.toml")?).map_err(|e| format!("rules.toml: {e}"))?;
        let mut statuses = BTreeMap::new();
        for (k, &n) in &rules.statuses {
            if let Some(other) = statuses.insert(n, k) {
                return Err(format!("rules.toml: statuses {k} and {other} are both {n:#04x}"));
            }
        }
        let stages: BTreeMap<String, StageEntry> = toml::from_str(&text("stages.toml")?).map_err(|e| format!("stages.toml: {e}"))?;
        let games: Games = toml::from_str(&text("games.toml")?).map_err(|e| format!("games.toml: {e}"))?;
        let kinds: BTreeMap<String, KindEntry> = toml::from_str(&text("kinds.toml")?).map_err(|e| format!("kinds.toml: {e}"))?;
        for (k, e) in &kinds {
            parse_pool(&e.pool).ok_or_else(|| format!("kinds.toml: {k}'s pool {:?} is no pool", e.pool))?;
        }
        let actions: BTreeMap<String, u8> = toml::from_str(&text("actions.toml")?).map_err(|e| format!("actions.toml: {e}"))?;
        let records: RecordNumbers = toml::from_str(&text("records.toml")?).map_err(|e| format!("records.toml: {e}"))?;
        let mut forms = BTreeMap::new();
        for (k, n) in &records.forms {
            if let Some(other) = forms.insert(*n, k) {
                return Err(format!("records.toml: forms {k} and {other} are both {n}"));
            }
        }
        let panels_file: PanelsFile = toml::from_str(&text("panels.toml")?).map_err(|e| format!("panels.toml: {e}"))?;
        let mut panels = BTreeMap::new();
        for (n, p) in panels_file.types {
            let n: u8 = n.parse().map_err(|_| format!("panels.toml: type {n:?} is no number"))?;
            if let Some(e) = &p.engine {
                engine_panel(e).ok_or_else(|| format!("panels.toml: type {n}'s engine type {e:?} isn't the engine's"))?;
            }
            panels.insert(n, p);
        }
        let navicust: NaviCustNumbers = toml::from_str(&text("navicust.toml")?).map_err(|e| format!("navicust.toml: {e}"))?;
        Ok(Compat { chips, chip_keys, assets, text: text_file, rules, stages, games, kinds, actions, records, panels, navicust })
    }

    /// A save's NaviCust part (its id: 4 x the program's number + the
    /// variant): the program's key and the color's place in its definition's
    /// colors; none for the empty part 0.
    pub fn navicust_part(&self, id: u8) -> Result<Option<(&str, u8)>, String> {
        if id == 0 {
            return Ok(None);
        }
        let number = id >> 2;
        let key = self
            .navicust
            .programs
            .iter()
            .find(|(_, n)| **n == number)
            .map(|(k, _)| k.as_str())
            .ok_or_else(|| format!("navicust.toml has no program {number} (part id {id:#04x})"))?;
        let color = self.navicust.variants[key]
            .iter()
            .position(|&v| v == id & 3)
            .ok_or_else(|| format!("{key} has no color in variant {} (part id {id:#04x})", id & 3))?;
        Ok(Some((key, color as u8)))
    }

    /// A chip's id (`cannon`) by its number.
    pub fn chip_key(&self, id: u16) -> Option<&str> {
        self.chip_keys.get(&id).map(String::as_str)
    }

    /// [`Compat::chip_key`], owned.
    pub fn chip(&self, id: u16) -> Option<String> {
        self.chip_key(id).map(String::from)
    }

    /// A chip's entry by its id.
    pub fn chip_entry(&self, key: &str) -> Option<&ChipEntry> {
        self.chips.get(key)
    }

    /// The engine's panel type of EXE4's panel type `n` (none: content has no
    /// type for it yet).
    pub fn panel_type(&self, n: u8) -> Result<Option<PanelType>, String> {
        let p = self.panels.get(&n).ok_or_else(|| format!("panels.toml has no type {n}"))?;
        Ok(p.engine.as_deref().and_then(engine_panel))
    }

    /// The sprites' names by (category, index).
    pub fn sprite_names(&self) -> BTreeMap<(u8, u8), String> {
        self.assets.sprites.iter().filter_map(|(name, id)| Some((parse_sprite(id)?, name.clone()))).collect()
    }

    /// The stage whose layout and actor list a settings record of a
    /// console of `version` and region names: its id.
    pub fn stage(&self, layout: u8, actor_list: u32, version: Version, japanese: bool) -> Option<String> {
        let shift = self.games.of(version, japanese).map_or(0, |g| g.settings);
        let actor_list = actor_list.wrapping_sub(shift as u32);
        self.stages.iter().find(|(_, e)| e.layout == layout && e.actor_list == actor_list).map(|(k, _)| k.clone())
    }

    /// The navi of a navi number (NaviStats +0x23): its id, if records.toml
    /// has it.
    pub fn navi_key(&self, n: u8) -> Option<&str> {
        self.records.navis.iter().find(|&(_, &v)| v == n).map(|(k, _)| k.as_str())
    }

    /// A navi's number (NaviStats +0x23), by its id.
    pub fn navi_number(&self, key: &str) -> Option<u8> {
        self.records.navis.get(key).copied()
    }

    /// The weapon of a routine number (NaviStats +0x09, +0x0A: 0x0800CA7C's
    /// table): its id (Err: a number records.toml lacks).
    pub fn weapon(&self, n: u8) -> Result<String, String> {
        self.records.weapons.iter().find(|(_, v)| v.contains(&n)).map(|(k, _)| k.clone()).ok_or_else(|| format!("weapon routine {n:#04x}"))
    }

    /// The form of a soul number (NaviStats +0x24: 0 the base form): its id.
    pub fn form(&self, number: u8) -> Option<&str> {
        self.records.forms.iter().find(|&(_, &n)| n == number).map(|(k, _)| k.as_str())
    }

    /// A form's soul number by its id.
    pub fn form_number(&self, key: &str) -> Option<u8> {
        self.records.forms.get(key).copied()
    }

    /// The aura of a number (NaviStats +0x21; None: 0, none).
    pub fn barrier(&self, n: u8) -> Result<Option<String>, String> {
        if n == 0 {
            return Ok(None);
        }
        self.records.barriers.iter().find(|&(_, &v)| v == n).map(|(k, _)| Some(k.clone())).ok_or_else(|| format!("aura {n}"))
    }

    /// A status's id (`paralyze-90`) by a hit's status byte.
    pub fn status(&self, byte: u8) -> Option<String> {
        self.rules.statuses.iter().find(|&(_, &n)| n == byte).map(|(k, _)| k.clone())
    }

    /// An object kind's pool and EXE4's index of it, by the kind's key.
    pub fn kind(&self, key: &str) -> Option<(Pool, u8)> {
        let e = self.kinds.get(key)?;
        Some((parse_pool(&e.pool)?, e.index))
    }

    /// The original's action number for object `r`'s CurAction (+9), as the
    /// recordings have it: any object's but a navi's its own byte; a navi's
    /// NaviAction as EXE4 numbers it: the framework's states by EXE4's state
    /// table (`EXE4_STATES`), the engine's actions, content's and the
    /// chips' by key (actions.toml).
    pub fn navi_action(&self, b: &nettai_battle::Battle, r: nettai_battle::object::ObjectRef) -> Result<u8, String> {
        use nettai_battle::kinds::player::{NaviAction, navi_action};
        if b.objects.get(r).actor.is_none() {
            return Ok(b.objects.get(r).action);
        }
        let action = navi_action(b, r);
        if let Some(n) = EXE4_STATES.iter().position(|&s| s == action) {
            return Ok(n as u8);
        }
        let key = match action {
            NaviAction::Engine(e) => e.key(),
            NaviAction::Content(h) => b.content.defs.action(h).key.as_str(),
            state => return Err(format!("EXE4 has no state {state:?}")),
        };
        self.actions.get(key).copied().ok_or_else(|| format!("actions.toml has no {key:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_compat_reads() {
        let c = Compat::exe4();
        assert_eq!(c.chip_key(1), Some("cannon"));
        assert_eq!(c.chip_entry("bluemoon").map(|e| (e.id, e.version)), Some((0x135, Some(Version::BlueMoon))));
        assert_eq!(c.sprite_names().get(&(0, 0)).map(String::as_str), Some("megaman"));
        // The US fonts draw 0x00 to 0x6F; the Japanese encoding's second page
        // runs from 0xE4.
        assert_eq!((c.text.first_control, c.text.glyphs.len()), (0xE4, 0x70));
        assert_eq!(c.text.jp.glyphs.len(), 0xE4);
        assert!(!c.text.jp.dialogue_glyphs.is_empty());
        // Twelve panel types: 5 metal, 10 SandRing's pitfall, 11 the Hole
        // chip's hole.
        assert_eq!(c.panels.len(), 12);
        assert_eq!(c.panel_type(8), Ok(Some(PanelType::Lava)));
        assert_eq!((c.panel_type(5), c.panel_type(10), c.panel_type(11)), (Ok(Some(PanelType::Metal)), Ok(Some(PanelType::Pitfall)), Ok(Some(PanelType::Hole))));
        assert_eq!(c.status(0x10).as_deref(), Some("paralyze-90"));
        assert_eq!((c.navi_key(0), c.form(0), c.weapon(0).as_deref()), (Some("megaman"), Some("base"), Ok("megaman/buster")));
        assert_eq!(c.kind("engine/player"), Some((Pool::Actor, 0)));
    }

    /// A settings record names its stage by its layout and actor list, at
    /// its ROM's addresses: the lab's netbattle-2 (record 1) on Red Sun's US
    /// console, and on Blue Moon's (its records 0xC further).
    #[test]
    fn a_settings_record_names_its_stage() {
        let c = Compat::exe4();
        assert_eq!(c.stage(0x00, 0x080F_C5F2, Version::RedSun, false).as_deref(), Some("netbattle-2"));
        assert_eq!(c.stage(0x00, 0x080F_C5FE, Version::BlueMoon, false).as_deref(), Some("netbattle-2"));
        assert_eq!(c.stage(0x70, 0x080F_C616, Version::RedSun, false).as_deref(), Some("netbattle-60"));
        assert_eq!(c.stage(0x00, 0x080F_C5F2, Version::BlueMoon, false), None);
    }

    #[test]
    fn a_compat_folder_reads_from_disk_as_built_in() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/exe4/compat");
        assert_eq!(&Compat::read(&dir).unwrap(), Compat::exe4());
    }
}
