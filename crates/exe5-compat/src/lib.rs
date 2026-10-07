//! Compatibility with EXE5's original games (Team ProtoMan and Team Colonel,
//! US and Japanese): their numbers for EXE5's content (content/exe5/compat:
//! compat is per game), and what reads them.
//!
//! - [`Compat`]: the tables, content key to the original's numbers (chip
//!   ids, the chips' uses by number, damage formulas, the version
//!   differences) and EXE5's panel types. [`Compat::exe5`] is this
//!   repository's, built in.
//! - [`codec`]: EXE5's records in the engine's terms: the 0x60-byte
//!   NaviStats with EXE5's light/dark value, the panels, the chip blocks.
//! - [`save`]: an EXE5 save file, and what a player's setup takes of it
//!   (the light/dark value, the souls it has).
//! - `trace` (feature `trace`): the chip lab's EXE5 recordings, read,
//!   decoded and replayed (docs/design/exe5-map.md §15.5).
//!
//! Keys: the tables are keyed by EXE5's ids, local to the game (`cannon`), as
//! content writes them (docs/design/content-model-v2.md §4.0), and hand
//! them out as they are.
//!
//! The engine never reads any of it (a test guards it). docs/design/
//! exe5-map.md §13 lists what of EXE5's records has no engine counterpart.

pub mod codec;
pub mod import;
pub mod save;
#[cfg(feature = "trace")]
pub mod trace;

use nettai_battle::field::PanelType;
use nettai_content_api::Pool;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// EXE5's game (content/exe5). Compat's tables are keyed by its ids, local
/// to the game (`cannon`; docs/design/content-model-v2.md §4.0).
pub const ROOT: &str = "exe5";

/// An EXE5 game's version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    Protoman,
    Colonel,
}

impl Version {
    /// The event flag of soul `number` (1 to 12) in this version's save, if
    /// it can have it (0x08024BF0, the soul button's table: Team
    /// ProtoMan's souls 1 to 6 flags 2 to 7, Team Colonel's 7 to 12 flags 8
    /// to 0x0D; the other version's 0xFF, never offered).
    pub fn soul_flag(self, number: u8) -> Option<u16> {
        match (self, number) {
            (Version::Protoman, 1..=6) | (Version::Colonel, 7..=12) => Some(number as u16 + 1),
            _ => None,
        }
    }
}

/// EXE5's object pools: how many slots each has (exe5-map.md §3.1). The
/// actors' is half EXE6's (32); the engine's `object::SLOTS` is one number
/// for every pool, which EXE5 needs per pool.
pub fn pool_slots(pool: Pool) -> usize {
    match pool {
        Pool::Actor => 16,
        Pool::Attack => 32,
        Pool::Effect => 32,
    }
}

/// The pool of an object type number as the traces print it (1, 3, 4).
pub fn pool_of_type(t: u8) -> Option<Pool> {
    match t {
        1 => Some(Pool::Actor),
        3 => Some(Pool::Attack),
        4 => Some(Pool::Effect),
        _ => None,
    }
}

/// What Team Colonel's record of a chip has where it differs from Team
/// ProtoMan's (compat's `colonel`): the flags (the version Gigas' library
/// bit) and the extra flags (the navi chips' +0x16).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionRecord {
    #[serde(default)]
    pub flags: Option<Vec<String>>,
    #[serde(default)]
    pub extra_flags: Option<Vec<u8>>,
}

/// A chip: its id, its use by number (the action and subtype its record
/// names), its damage formula's number (a damage of 1000 and up, less
/// 1000), and Team Colonel's differences.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipEntry {
    pub id: u16,
    pub action: u8,
    pub subtype: u8,
    #[serde(default)]
    pub damage_formula: Option<u16>,
    #[serde(default)]
    pub colonel: Option<VersionRecord>,
    /// An e-Reader card's chip (LeadRaid, ChaosLrd): the save slot its
    /// name, description and picture palette are in (exe5-map.md §6.4).
    #[serde(default)]
    pub save_slot: Option<u8>,
}

/// An EXE5 panel type: content's name for it (its rules' `panels.numbers`,
/// by the same number) and the flag word the game gives it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelEntry {
    pub name: String,
    pub flags: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelsFile {
    types: BTreeMap<String, PanelEntry>,
}

/// The original's numbers of EXE5's rule definitions (rules.toml).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleNumbers {
    /// Statuses (0x0801CEC4): a hit's status byte, by key.
    #[serde(default)]
    pub statuses: BTreeMap<String, u8>,
}

/// An object kind (kinds.toml): EXE5's pool and index of it, and the
/// position bytes the comparison skips (as exe6-compat's).
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
    /// Its panel bytes are register garbage nothing reads (Django's
    /// lights, spawned with the last spawned object's address in a
    /// register): the comparison skips them.
    #[serde(default)]
    pub scratch_panel: bool,
    /// It keeps its spawner's address in its Z (its low half as Z's
    /// fraction: ChaosLrd's navi) as it moves, and so does what wears its Z
    /// (its form overlay): on a console of another ROM its Z is as far from
    /// the content's as the addresses are (games.toml's
    /// `spawner_z_fractions`), whatever it is.
    #[serde(default)]
    pub z_moves_from_spawner: bool,
    /// It falls toward height 0 for this many ticks (its timer counts them
    /// down), and once more as it bursts, from a Z whose fraction is its
    /// spawner's address's low half, its velocity that Z over the ticks
    /// (ChaosLrd's strike, 0x080D6948): on a console of another ROM it falls
    /// from that ROM's (games.toml's `spawner_z_fractions`), and so another
    /// way.
    #[serde(default)]
    pub z_falls_from_spawner: Option<i32>,
    /// What the recordings show as its collision's status is garbage: its
    /// +0x54 holds no collision but a RAM address of its own (ShadowMan's
    /// three, LeadRaid's Colonel), which the recorder reads through as
    /// one. The comparison skips it.
    #[serde(default)]
    pub scratch_status: bool,
}

/// Where a ROM other than Team ProtoMan's US one has what compat names by
/// that ROM's addresses (games.toml, one section a ROM).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameAddresses {
    /// How far its stage actor lists (a battle settings record's bytes
    /// 12..16) are from Team ProtoMan's US ROM's.
    pub actor_lists: i32,
    /// Kinds spawned with a code address as their Z (a register their
    /// spawner left), whole or its low half as Z's fraction: by kind key,
    /// Team ProtoMan US's address (the content's) and this ROM's (in the
    /// same 64 KiB: their low halves are as far apart as they are).
    #[serde(default)]
    pub spawner_z_fractions: BTreeMap<String, [u32; 2]>,
    /// Kinds spawned with a code address's low byte as their panel X, and
    /// as their panel Y: by kind key, Team ProtoMan US's (the content's) and
    /// this ROM's.
    #[serde(default)]
    pub panel_xs: BTreeMap<String, [u8; 2]>,
    #[serde(default)]
    pub panel_ys: BTreeMap<String, [u8; 2]>,
}

/// games.toml: the other three ROMs' [`GameAddresses`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Games {
    pub colonel: GameAddresses,
    #[serde(rename = "jp-protoman")]
    pub jp_protoman: GameAddresses,
    #[serde(rename = "jp-colonel")]
    pub jp_colonel: GameAddresses,
}

impl Games {
    /// The addresses of a version's ROM of a region (none: Team ProtoMan's
    /// US one, compat's own).
    pub fn of(&self, version: Version, japanese: bool) -> Option<&GameAddresses> {
        match (version, japanese) {
            (Version::Protoman, false) => None,
            (Version::Colonel, false) => Some(&self.colonel),
            (Version::Protoman, true) => Some(&self.jp_protoman),
            (Version::Colonel, true) => Some(&self.jp_colonel),
        }
    }

    /// An object's Z as a console of `rom` (its version and whether it is
    /// Japanese) has it: a Z that is a spawner's address of Team ProtoMan
    /// US's (`spawner_z_fractions`, which the content keeps), or whose
    /// fraction is that address's low half (what is spawned at such an
    /// object: ChaosLrd's strike), has that ROM's.
    pub fn z(&self, rom: (Version, bool), z: i32) -> i32 {
        let Some(g) = self.of(rom.0, rom.1) else { return z };
        let addresses = || g.spawner_z_fractions.values();
        if let Some([_, other]) = addresses().find(|[us, _]| *us == z as u32) {
            return *other as i32;
        }
        let fraction = z as u32 & 0xFFFF;
        match addresses().find(|[us, _]| us & 0xFFFF == fraction) {
            Some([_, other]) => ((z as u32 & !0xFFFF) | (other & 0xFFFF)) as i32,
            None => z,
        }
    }

    /// What a console of `rom` adds to the Z of a `kind` object falling
    /// from its spawner's address in `ticks` ticks (`KindEntry::
    /// z_falls_from_spawner`), at `z` with `timer` ticks left: the content's
    /// fall starts from Team ProtoMan US's address, the console's from its
    /// own. (Its starting height's whole part is the one whose fall reaches
    /// `z` after that many ticks.) As exe6-compat's `drop_z_offset`, with no
    /// gravity.
    pub fn fall_offset(&self, rom: (Version, bool), kind: &str, z: i32, timer: u16, ticks: i32) -> i32 {
        let Some(&[us, other]) = self.of(rom.0, rom.1).and_then(|g| g.spawner_z_fractions.get(kind)) else { return 0 };
        // (The ticks it has fallen: and on the tick after its last, the
        // timer past 0, it falls once more as it bursts.)
        let t = ticks - timer as i16 as i32;
        if !(1..=ticks + 1).contains(&t) {
            return 0;
        }
        // (Its velocity, the BIOS division truncated toward zero.)
        let at = |start: i32| start.wrapping_add(t.wrapping_mul(start.wrapping_neg() / ticks));
        let start = |whole: i32, address: u32| (whole << 16) | (address & 0xFFFF) as i32;
        (-0x100..0x100).find(|&whole| at(start(whole, us)) == z).map_or(0, |whole| at(start(whole, other)).wrapping_sub(z))
    }

    /// How far a `kind` object's Z is on a console of `rom` from the
    /// content's, for a kind that keeps its spawner's address in Z (whole,
    /// or its low half as Z's fraction) as it moves (`KindEntry::
    /// z_moves_from_spawner`): as far as that ROM's address is from Team
    /// ProtoMan US's.
    pub fn z_offset(&self, rom: (Version, bool), kind: &str) -> i32 {
        match self.of(rom.0, rom.1).and_then(|g| g.spawner_z_fractions.get(kind)) {
            Some(&[us, other]) => other.wrapping_sub(us) as i32,
            None => 0,
        }
    }

    /// A `kind` object's panel as a console of `rom` has it: a panel X or Y
    /// that is a code address's low byte (`panel_xs`, `panel_ys`: the
    /// content keeps Team ProtoMan US's) is that ROM's.
    pub fn panel(&self, rom: (Version, bool), kind: &str, [x, y]: [u8; 2]) -> [u8; 2] {
        let Some(g) = self.of(rom.0, rom.1) else { return [x, y] };
        let map = |table: &BTreeMap<String, [u8; 2]>, v: u8| match table.get(kind) {
            Some(&[us, other]) if v == us => other,
            _ => v,
        };
        [map(&g.panel_xs, x), map(&g.panel_ys, y)]
    }
}

/// A netbattle stage (stages.toml): the settings records that are it, its
/// panel layout's number and its actor list's address (Team ProtoMan's US
/// ROM's: games.toml has the others').
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageEntry {
    pub settings: Vec<u8>,
    pub layout: u8,
    pub actor_list: u32,
}

/// What EXE5's NaviStats and its save name by number (records.toml): navis
/// by navi number, weapons by routine number, projectile variants by row,
/// barriers by type, MegaMan's forms by form number (+0x2C: 0 his base
/// form, 1 to 12 his souls), the SP chips whose damage goes by a deletion
/// time by the save's slot of it, each by its key.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordNumbers {
    #[serde(default)]
    pub navis: BTreeMap<String, u8>,
    #[serde(default)]
    pub weapons: BTreeMap<String, Vec<u8>>,
    #[serde(default)]
    pub projectile_variants: BTreeMap<String, u8>,
    #[serde(default)]
    pub barriers: BTreeMap<String, u8>,
    #[serde(default)]
    pub forms: BTreeMap<String, u8>,
    #[serde(default)]
    pub sp_slots: BTreeMap<String, u8>,
}

/// EXE5's NaviCust programs (navicust.toml): each program's number (a part
/// id's high bits) and its colored variants (a part id's low bits) in the
/// order of its definition's colors.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviCustNumbers {
    #[serde(default)]
    pub programs: BTreeMap<String, u8>,
    #[serde(default)]
    pub variants: BTreeMap<String, Vec<u8>>,
}

/// EXE5's patch cards (patch-cards.toml): each card's number, the number a
/// save's card list holds, by key; card 111 (Bass-Cross MegaMan) each
/// team's own, by version.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatchCardNumbers {
    pub cards: BTreeMap<String, u8>,
    pub protoman: BTreeMap<String, u8>,
    pub colonel: BTreeMap<String, u8>,
}

impl PatchCardNumbers {
    /// patch-cards.toml: the cards by key at the top, the versions' own in
    /// their tables (`[by-version.protoman]`, `[by-version.colonel]`). (An
    /// empty file: none.)
    fn parse(text: &str) -> Result<PatchCardNumbers, String> {
        let table: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
        let mut out = PatchCardNumbers::default();
        let number = |k: &str, v: &toml::Value| -> Result<u8, String> {
            v.as_integer().and_then(|n| u8::try_from(n).ok()).ok_or_else(|| format!("{k} is {v}, not a card number"))
        };
        for (k, v) in &table {
            match (k.as_str(), v) {
                ("by-version", toml::Value::Table(versions)) => {
                    for (version, t) in versions {
                        let map = match version.as_str() {
                            "protoman" => &mut out.protoman,
                            "colonel" => &mut out.colonel,
                            _ => return Err(format!("by-version.{version}: EXE5's versions are protoman and colonel")),
                        };
                        let toml::Value::Table(t) = t else {
                            return Err(format!("by-version.{version} is {t}, not a table of cards"));
                        };
                        for (k, v) in t {
                            map.insert(k.clone(), number(k, v)?);
                        }
                    }
                }
                _ => {
                    out.cards.insert(k.clone(), number(k, v)?);
                }
            }
        }
        Ok(out)
    }
}

/// EXE5's asset names (assets.toml): the names exe5-extract writes its
/// assets under, by EXE6's names for what is EXE6's. Sprites as "cc-ii" (the
/// category's byte offset in the sprite list and the index), sounds by the
/// song table's numbers, banners by banner id, backgrounds by number.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetNames {
    #[serde(default)]
    pub sprites: BTreeMap<String, String>,
    #[serde(default)]
    pub sounds: BTreeMap<String, u16>,
    #[serde(default)]
    pub banners: BTreeMap<String, u8>,
    /// A background's number, or the numbers that are one background: a
    /// liberation's map's own number has its area's load data, scroll and
    /// animations (`acdc-area = [0x08, 0x0c]`). The pack has the first
    /// number's picture under the name.
    #[serde(default)]
    pub backgrounds: BTreeMap<String, Numbers>,
    /// The emotion window's faces by the number exe5-extract gives them.
    #[serde(default)]
    pub mugshots: BTreeMap<String, u8>,
}

/// One number, or several that are one thing (`0x08`, `[0x08, 0x0c]`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Numbers {
    One(u8),
    Many(Vec<u8>),
}

impl Numbers {
    /// Every number, the first first.
    pub fn all(&self) -> &[u8] {
        match self {
            Numbers::One(n) => std::slice::from_ref(n),
            Numbers::Many(v) => v,
        }
    }
}

/// EXE5's text encodings (text.toml): what each byte below `first_control`
/// draws, the 8x16 font's glyphs then the dialogue font's past them (as
/// EXE6's text.toml), the US ROMs' and the Japanese ROMs'.
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

/// EXE5's compat tables (content/exe5/compat).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Compat {
    /// chips.toml: by key.
    pub chips: BTreeMap<String, ChipEntry>,
    /// panels.toml: by EXE5's panel type number.
    pub panels: BTreeMap<u8, PanelEntry>,
    /// The chips' keys by id.
    pub chip_keys: BTreeMap<u16, String>,
    /// assets.toml: the assets' names.
    pub assets: AssetNames,
    /// rules.toml: the rule definitions' numbers.
    pub rules: RuleNumbers,
    /// stages.toml: the netbattle stages, by key.
    pub stages: BTreeMap<String, StageEntry>,
    /// games.toml: where the other ROMs have what compat addresses.
    pub games: Games,
    /// records.toml: what NaviStats name by number.
    pub records: RecordNumbers,
    /// kinds.toml: the object kinds' numbers, by key.
    pub kinds: BTreeMap<String, KindEntry>,
    /// text.toml: the text encodings.
    pub text: Text,
    /// navicust.toml: the NaviCust programs' numbers.
    pub navicust: NaviCustNumbers,
    /// patch-cards.toml: the patch cards' numbers.
    pub patch_cards: PatchCardNumbers,
    /// actions.toml: the navi action numbers chips.toml doesn't give (the
    /// engine's own actions, content's actions of no chip), by key.
    pub actions: BTreeMap<String, u8>,
}

/// EXE5's navi states, by their CurAction (its player's state table,
/// 0x080EAE08): EXE6's order without its freeze and bubble (EXE6's 6 and 7),
/// so that idle is 6 (EXE6's 8).
const EXE5_STATES: [nettai_battle::kinds::player::NaviAction; 7] = {
    use nettai_battle::kinds::player::NaviAction::*;
    [Entry, TakeControl, Deletion, Flinch, Paralysis, Drag, Idle]
};

/// The files of a compat folder.
pub const FILES: [&str; 12] = [
    "actions.toml",
    "games.toml",
    "chips.toml",
    "panels.toml",
    "assets.toml",
    "rules.toml",
    "stages.toml",
    "records.toml",
    "kinds.toml",
    "text.toml",
    "navicust.toml",
    "patch-cards.toml",
];

/// This repository's compat (content/exe5/compat), built in.
const EXE5: [(&str, &str); 12] = [
    ("actions.toml", include_str!("../../../content/exe5/compat/actions.toml")),
    ("games.toml", include_str!("../../../content/exe5/compat/games.toml")),
    ("patch-cards.toml", include_str!("../../../content/exe5/compat/patch-cards.toml")),
    ("navicust.toml", include_str!("../../../content/exe5/compat/navicust.toml")),
    ("text.toml", include_str!("../../../content/exe5/compat/text.toml")),
    ("kinds.toml", include_str!("../../../content/exe5/compat/kinds.toml")),
    ("stages.toml", include_str!("../../../content/exe5/compat/stages.toml")),
    ("records.toml", include_str!("../../../content/exe5/compat/records.toml")),
    ("chips.toml", include_str!("../../../content/exe5/compat/chips.toml")),
    ("panels.toml", include_str!("../../../content/exe5/compat/panels.toml")),
    ("assets.toml", include_str!("../../../content/exe5/compat/assets.toml")),
    ("rules.toml", include_str!("../../../content/exe5/compat/rules.toml")),
];

/// A sprite's "cc-ii".
fn parse_sprite(id: &str) -> Option<(u8, u8)> {
    let (c, i) = id.split_once('-')?;
    Some((u8::from_str_radix(c, 16).ok()?, u8::from_str_radix(i, 16).ok()?))
}

impl Compat {
    /// EXE5's compat, as this repository's content/exe5/compat has it.
    pub fn exe5() -> &'static Compat {
        static EXE5_COMPAT: std::sync::OnceLock<Compat> = std::sync::OnceLock::new();
        EXE5_COMPAT.get_or_init(|| {
            Compat::parse(|file| Ok(EXE5.iter().find(|(f, _)| *f == file).map(|(_, t)| t.to_string()).unwrap_or_default()))
                .unwrap_or_else(|e| panic!("content/exe5/compat: {e}"))
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
        let panels: PanelsFile = toml::from_str(&text("panels.toml")?).map_err(|e| format!("panels.toml: {e}"))?;
        let mut chip_keys = BTreeMap::new();
        for (k, c) in &chips {
            if let Some(other) = chip_keys.insert(c.id, k.clone()) {
                return Err(format!("chips.toml: {k} and {other} are both chip {:#05x}", c.id));
            }
        }
        let mut by_number = BTreeMap::new();
        for (n, p) in panels.types {
            let n: u8 = n.parse().map_err(|_| format!("panels.toml: type {n:?} isn't a number"))?;
            by_number.insert(n, p);
        }
        let assets: AssetNames = toml::from_str(&text("assets.toml")?).map_err(|e| format!("assets.toml: {e}"))?;
        let mut background_numbers = BTreeMap::new();
        for (name, numbers) in &assets.backgrounds {
            if numbers.all().is_empty() {
                return Err(format!("assets.toml: background {name} has no number"));
            }
            for &n in numbers.all() {
                if let Some(other) = background_numbers.insert(n, name) {
                    return Err(format!("assets.toml: backgrounds {name} and {other} are both {n:#04x}"));
                }
            }
        }
        for (name, id) in &assets.sprites {
            parse_sprite(id).ok_or_else(|| format!("assets.toml: sprite {name} is {id:?}, not \"cc-ii\""))?;
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
        let records: RecordNumbers = toml::from_str(&text("records.toml")?).map_err(|e| format!("records.toml: {e}"))?;
        let mut forms = BTreeMap::new();
        for (k, n) in &records.forms {
            if let Some(other) = forms.insert(*n, k) {
                return Err(format!("records.toml: forms {k} and {other} are both {n}"));
            }
        }
        let mut sp_slots = BTreeMap::new();
        for (k, n) in &records.sp_slots {
            if *n as usize >= save::SP_SLOTS {
                return Err(format!("records.toml: sp_slots {k} is {n}, past the save's {} slots", save::SP_SLOTS));
            }
            if let Some(other) = sp_slots.insert(*n, k) {
                return Err(format!("records.toml: sp_slots {k} and {other} are both {n}"));
            }
        }
        let kinds: BTreeMap<String, KindEntry> = toml::from_str(&text("kinds.toml")?).map_err(|e| format!("kinds.toml: {e}"))?;
        let navicust: NaviCustNumbers = toml::from_str(&text("navicust.toml")?).map_err(|e| format!("navicust.toml: {e}"))?;
        for (k, n) in &navicust.programs {
            if !navicust.variants.contains_key(k) {
                return Err(format!("navicust.toml: program {k} ({n}) has no variants"));
            }
        }
        // (A compat folder from before the patch cards has none.)
        let patch_cards = PatchCardNumbers::parse(&text("patch-cards.toml").unwrap_or_default()).map_err(|e| format!("patch-cards.toml: {e}"))?;
        // (A compat folder from before the action numbers has none.)
        let actions_file = text("actions.toml").unwrap_or_default();
        let actions: BTreeMap<String, u8> =
            if actions_file.is_empty() { BTreeMap::new() } else { toml::from_str(&actions_file).map_err(|e| format!("actions.toml: {e}"))? };
        // (A compat folder from before the encodings has none.)
        let text_file = text("text.toml").unwrap_or_default();
        let text: Text = if text_file.is_empty() { Text::default() } else { toml::from_str(&text_file).map_err(|e| format!("text.toml: {e}"))? };
        Ok(Compat { chips, panels: by_number, chip_keys, assets, rules, stages, games, records, kinds, text, navicust, patch_cards, actions })
    }

    /// The NaviCust program a part id names (its number, `id >> 2`) and its
    /// color (the variant, `id & 3`, as its place among the program's
    /// colored variants: its definition's color there); none for 0, no part.
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

    /// The patch card of a save's card number in `version` (its own card
    /// 111 first): its key.
    pub fn patch_card(&self, number: u8, version: Version) -> Result<&str, String> {
        let own = match version {
            Version::Protoman => &self.patch_cards.protoman,
            Version::Colonel => &self.patch_cards.colonel,
        };
        own.iter()
            .chain(&self.patch_cards.cards)
            .find(|(_, n)| **n == number)
            .map(|(k, _)| k.as_str())
            .ok_or_else(|| format!("patch-cards.toml has no card {number} in {version:?}"))
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

    /// The stage whose layout and actor list a settings record names: its
    /// id.
    pub fn stage(&self, layout: u8, actor_list: u32, version: Version, japanese: bool) -> Option<String> {
        let shift = self.games.of(version, japanese).map_or(0, |g| g.actor_lists);
        let actor_list = actor_list.wrapping_sub(shift as u32);
        self.stages.iter().find(|(_, e)| e.layout == layout && e.actor_list == actor_list).map(|(k, _)| k.clone())
    }

    /// The navi of a navi number (NaviStats +0x29): its id, if records.toml
    /// has it (the navis the content has).
    pub fn navi_key(&self, n: u8) -> Option<&str> {
        self.records.navis.iter().find(|&(_, &v)| v == n).map(|(k, _)| k.as_str())
    }

    /// A navi's number (NaviStats +0x29), by its id.
    pub fn navi_number(&self, key: &str) -> Option<u8> {
        self.records.navis.get(key).copied()
    }

    /// The weapon of a routine number: its id (None: 0xFF, no weapon; Err:
    /// a number records.toml lacks).
    pub fn weapon(&self, n: u8) -> Result<Option<String>, String> {
        if n == 0xFF {
            return Ok(None);
        }
        self.records
            .weapons
            .iter()
            .find(|(_, v)| v.contains(&n))
            .map(|(k, _)| Some(k.clone()))
            .ok_or_else(|| format!("weapon routine {n:#04x}"))
    }

    /// The projectile variant a navi's shot program names (NaviStats +0x4D,
    /// the buster shot's, and +0x4F, the charged shot's: the projectile
    /// table's row); none for the byte 0, no program (the table's first
    /// row is the plain shot, which a shot with no program is), as
    /// exe6-compat reads EXE6's.
    pub fn shot_program(&self, n: u8) -> Result<Option<String>, String> {
        if n == 0 {
            return Ok(None);
        }
        self.records.projectile_variants.iter().find(|&(_, &v)| v == n).map(|(k, _)| Some(k.clone())).ok_or_else(|| format!("projectile row {n:#04x}"))
    }

    /// The barrier of a type (None: 0, none).
    pub fn barrier(&self, n: u8) -> Result<Option<String>, String> {
        if n == 0 {
            return Ok(None);
        }
        self.records.barriers.iter().find(|&(_, &v)| v == n).map(|(k, _)| Some(k.clone())).ok_or_else(|| format!("barrier type {n}"))
    }

    /// A form's number (NaviStats +0x2C: a soul's 1 to 12, the base form's
    /// 0) by its id; none for a form the original hasn't.
    pub fn form_number(&self, key: &str) -> Option<u8> {
        self.records.forms.get(key).copied()
    }

    /// The form of a form number: its id (`protosoul` for soul 1).
    pub fn form(&self, number: u8) -> Option<&str> {
        self.records.forms.iter().find(|&(_, &n)| n == number).map(|(k, _)| k.as_str())
    }

    /// A status's id (`paralyze-90`) by a hit's status byte.
    pub fn status(&self, byte: u8) -> Option<String> {
        self.rules.statuses.iter().find(|&(_, &n)| n == byte).map(|(k, _)| k.clone())
    }

    /// The name of background number `n` (a settings record's byte 4).
    pub fn background(&self, n: u8) -> Option<&str> {
        self.assets.backgrounds.iter().find(|(_, numbers)| numbers.all().contains(&n)).map(|(k, _)| k.as_str())
    }

    /// The backgrounds' names by the number the pack has each under (its
    /// first).
    pub fn background_names(&self) -> BTreeMap<u8, String> {
        self.assets.backgrounds.iter().map(|(k, numbers)| (numbers.all()[0], k.clone())).collect()
    }

    /// The sprites' names by (category, index).
    pub fn sprite_names(&self) -> BTreeMap<(u8, u8), String> {
        self.assets.sprites.iter().filter_map(|(name, id)| Some((parse_sprite(id)?, name.clone()))).collect()
    }

    /// The original's action number for object `r`'s CurAction (+9), as the
    /// traces record it: any object's but a navi's its own byte; a navi's
    /// NaviAction as EXE5 numbers it: the framework's states by EXE5's state
    /// table (`EXE5_STATES`), a chip's action by its record (chips.toml's
    /// `action`: the action of the chip whose use it is), the rules'
    /// actions and content's others by key (actions.toml).
    pub fn navi_action(&self, b: &nettai_battle::Battle, r: nettai_battle::object::ObjectRef) -> Result<u8, String> {
        use nettai_battle::content::ChipUsage;
        use nettai_battle::kinds::player::{NaviAction, navi_action};
        if b.objects.get(r).actor.is_none() {
            return Ok(b.objects.get(r).action);
        }
        let action = navi_action(b, r);
        if let Some(n) = EXE5_STATES.iter().position(|&s| s == action) {
            return Ok(n as u8);
        }
        let key = match action {
            NaviAction::Engine(e) => e.key(),
            NaviAction::Content(h) => {
                let key = b.content.defs.action(h).key.as_str();
                if let Some(&n) = self.actions.get(key) {
                    return Ok(n);
                }
                let of_chip = self.chips.iter().find(|(k, _)| {
                    b.content.defs.chip_by_key(k).is_some_and(|c| matches!(b.content.defs.chip(c).usage, ChipUsage::Action(a) if a == h))
                });
                if let Some((_, entry)) = of_chip {
                    return Ok(entry.action);
                }
                key
            }
            state => return Err(format!("EXE5 has no state {state:?}")),
        };
        self.actions.get(key).copied().ok_or_else(|| format!("actions.toml has no {key:?}"))
    }

    /// The panel type EXE5 numbers `n`: the same number in content's rules,
    /// which name it as panels.toml does.
    pub fn panel_type(&self, n: u8) -> Result<PanelType, String> {
        self.panels.get(&n).map(|_| PanelType(n)).ok_or_else(|| format!("panels.toml has no type {n}"))
    }

    /// The name content gives EXE5's panel type `n`.
    pub fn panel_name(&self, n: u8) -> Result<&str, String> {
        self.panels.get(&n).map(|p| p.name.as_str()).ok_or_else(|| format!("panels.toml has no type {n}"))
    }
}
