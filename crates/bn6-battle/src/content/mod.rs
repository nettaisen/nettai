//! Game content: the typed data a battle runs on.
//!
//! [`Content`] holds everything the simulation reads that isn't rules
//! code: chips, navis and forms, the ruleset's tables (collision types,
//! panel rules, battle settings, status effects...), object data (rocks,
//! attachments, overlays), the effect and region registries, and every
//! sprite's animation timing. It never changes during a battle and is
//! shared between battles and their snapshots through an `Arc`:
//!
//! ```ignore
//! let (content, _report) = bn6_content::pack::load_battle(pack)?;
//! let content = Arc::new(content);
//! let setup = RoundSetup { content: content.hash(), ..setup };
//! let battle = Battle::new(setup, content.clone());
//! let chip = battle.content.chip(0x11);
//! ```
//!
//! See docs/design/content-pack.md for the pack's files and this API.
//!
//! The engine does no file IO: a loader outside it (bn6-content) reads a
//! content pack into this model, and tests build small content sets in
//! code. BN6's content comes only from a pack extracted from the user's
//! ROM (`bn6-extract content`).
//!
//! Content has an identity, [`Content::hash`], which a round's setup
//! carries (`RoundSetup::content`) so that netplay peers can check they
//! run the same content. The content itself is not part of a snapshot or
//! of the state digest.
//!
//! Ids are the original's numbers (chip ids, NameIDs, row numbers the
//! state and traces observe); the tables here are dense and indexed by
//! them.

mod chips;
mod custom;
mod defs;
mod flags;
mod navis;
mod objects;
mod roles;
mod rules;
mod scripts;
mod sprites;
#[cfg(any(test, feature = "test-content"))]
pub mod testing;

pub use chips::*;
pub use custom::*;
pub use defs::*;
pub use navis::*;
pub use objects::*;
pub use roles::*;
pub use rules::*;
pub use scripts::*;
pub use sprites::*;

use bn6_content_api::{ChipHandle, FormHandle, NaviHandle, StageHandle, WeaponHandle};
use serde::{Deserialize, Serialize};

/// An attack's primary element (a chip's element, a navi's or form's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Element {
    #[default]
    Null = 0,
    Fire = 1,
    Aqua = 2,
    Elec = 3,
    Wood = 4,
}

/// Secondary-element bits: an attack's extra elements, or what a navi is
/// weak to. In a content file, a list of names (`["sword"]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SecondaryElements(pub u8);

impl SecondaryElements {
    pub const BREAK: u8 = 0x10;
    pub const WIND: u8 = 0x20;
    pub const CURSOR: u8 = 0x40;
    pub const SWORD: u8 = 0x80;
    pub(crate) const NAMES: &[(u32, &str)] =
        &[(0x10, "break"), (0x20, "wind"), (0x40, "cursor"), (0x80, "sword")];
}

flags::serde_flags!(SecondaryElements, u8);

/// A panel relative to another, `dx` toward the facing side. In a
/// content file, `[dx, dy]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PanelOffset {
    pub dx: i8,
    pub dy: i8,
}

impl Serialize for PanelOffset {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        [self.dx, self.dy].serialize(s)
    }
}

impl<'de> Deserialize<'de> for PanelOffset {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<PanelOffset, D::Error> {
        let [dx, dy] = <[i8; 2]>::deserialize(d)?;
        Ok(PanelOffset { dx, dy })
    }
}

/// A test on a panel's flags word (`field::pflags`): all of `require`
/// set and none of `forbid`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelCondition {
    pub require: u32,
    pub forbid: u32,
}

/// A hit region content defines (`define.region`): panels around the
/// anchor, or the whole field's panels that meet a condition.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Region {
    Panels(Vec<PanelOffset>),
    Field(PanelCondition),
}

/// A HUD banner (the game's UI banner id).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BannerId(pub u8);

/// The identity of a content set: a stable hash of all of it. A round's
/// setup carries it; two peers whose setups agree run the same content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ContentHash(pub u64);

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Everything the simulation reads besides its own state and code. See
/// the module docs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Content {
    /// Chips by chip id (0..=0x19A).
    pub chips: Vec<ChipData>,
    /// Navis by [`Navi`](crate::setup::Navi) number (MegaMan is 0).
    pub navis: Vec<NaviData>,
    /// MegaMan's forms by [`Form`](crate::setup::Form) number (0 is the
    /// base form).
    pub forms: Vec<FormData>,
    /// The ruleset's tables.
    pub rules: Rules,
    /// Object kinds' data.
    pub objects: ObjectData,
    /// Generic one-shot effects by effect id (effect object #0's first
    /// parameter).
    pub effects: Vec<EffectSprite>,
    /// Hit sparks by hit-effect id.
    pub sparks: Vec<EffectSprite>,
    /// Hit-region shapes by region number (below 0x80): the panels a hit
    /// covers, relative to its panel.
    pub regions: Vec<Vec<PanelOffset>>,
    /// Panel layouts by layout number (the battle settings' `layout`).
    pub panel_layouts: Vec<PanelLayout>,
    /// Every sprite's animation timing.
    pub animations: Animations,
    /// The assets content can name (`asset.sprite("bomb")`): the loader
    /// fills it from the pack's asset names (docs/design/
    /// content-model-v2.md §6.3).
    pub assets: bn6_content_api::AssetNames,
    /// MegaMan's weapon routines that scripts implement (see `scripts`).
    pub weapons: Vec<WeaponData>,
    /// The pack's scripts (see `scripts`).
    pub scripts: Scripts,
    /// What the content defines: the registries and their handles, and the
    /// functions the scripts implement (see `defs`). Made from the rest by
    /// [`Content::define`].
    pub defs: Defs,
}

impl Content {
    /// Run the define phase over the scripts and build the registries
    /// (docs/design/content-model-v2.md §7.3). A battle needs defined
    /// content; loaders call this once the data and scripts are in.
    pub fn define(&mut self) -> Result<(), bn6_content_api::ContentError> {
        let definitions = if self.scripts.modules.is_empty() {
            Default::default()
        } else {
            let data = crate::behavior::script_data(self);
            let (definitions, compiled) =
                bn6_luau::define(&self.scripts.pack(), &data, &self.assets, bn6_luau::Options::default())?;
            self.scripts.compiled = CompiledModules(compiled);
            definitions
        };
        self.defs = Defs::build(self, definitions)?;
        Ok(())
    }

    /// The content, defined (see [`Content::define`]); panics on a content
    /// error.
    pub fn defined(mut self) -> Content {
        self.define().unwrap_or_else(|e| panic!("content error: {e}"));
        self
    }

    /// The content's identity (a stable hash of all of it; computing it
    /// walks everything, so callers keep the value).
    pub fn hash(&self) -> ContentHash {
        ContentHash(crate::digest::stable_hash(self))
    }

    /// A chip's record.
    pub fn chip(&self, h: ChipHandle) -> &ChipData {
        &self.defs.chip(h).record
    }

    /// The original's number of a chip the pack numbers (the ruleset's
    /// numeric logic asks it until phase C); none for a chip content
    /// defines.
    pub fn chip_number(&self, h: ChipHandle) -> Option<ChipId> {
        self.defs.chip(h).record.id
    }

    /// The pack's chip with this number (registration by number).
    pub fn chip_numbered(&self, id: ChipId) -> Option<ChipHandle> {
        self.defs.chip_numbered(id)
    }

    /// The record a chip field names: the chip's, or for none (a zeroed
    /// field) the pack's chip 0's, which is what the game reads.
    pub fn chip_field(&self, h: Option<ChipHandle>) -> &ChipData {
        let h = h.or_else(|| self.chip_numbered(0)).expect("the pack's chip 0 (a zeroed chip field reads it)");
        self.chip(h)
    }

    /// Whether `h` is the pack's chip number `id`.
    pub fn is_chip(&self, h: ChipHandle, id: ChipId) -> bool {
        self.chip_number(h) == Some(id)
    }

    /// A navi's data.
    pub fn navi(&self, h: NaviHandle) -> &NaviData {
        &self.defs.navi(h).record
    }

    /// A navi's number (the ruleset's numeric logic asks it until phase C).
    pub fn navi_number(&self, h: NaviHandle) -> crate::setup::Navi {
        crate::setup::Navi(self.navi(h).id)
    }

    /// The pack's navi with this number.
    pub fn navi_numbered(&self, navi: crate::setup::Navi) -> NaviHandle {
        self.defs.navi_numbered(navi).unwrap_or_else(|| panic!("navi {} is not in the content", navi.0))
    }

    /// One of MegaMan's forms.
    pub fn form(&self, h: FormHandle) -> &FormData {
        &self.defs.form(h).record
    }

    /// A form's number (the ruleset's numeric logic asks it until phase C).
    pub fn form_number(&self, h: FormHandle) -> crate::setup::Form {
        crate::setup::Form(self.form(h).id)
    }

    /// The pack's form with this number's data.
    pub fn form_data(&self, form: crate::setup::Form) -> &FormData {
        self.form(self.form_numbered(form))
    }

    /// The pack's navi with this number's data.
    pub fn navi_data(&self, navi: crate::setup::Navi) -> &NaviData {
        self.navi(self.navi_numbered(navi))
    }

    /// The pack's form with this number.
    pub fn form_numbered(&self, form: crate::setup::Form) -> FormHandle {
        self.defs.form_numbered(form).unwrap_or_else(|| panic!("form {:#x} is not in the content", form.0))
    }

    /// A weapon's routine number (the ruleset's numeric logic asks it until
    /// phase C); none for a weapon content defines.
    pub fn weapon_number(&self, h: WeaponHandle) -> Option<u8> {
        self.defs.weapon(h).number
    }

    /// The weapon a routine number names.
    pub fn weapon_numbered(&self, routine: u8) -> WeaponHandle {
        self.defs.weapon_numbered(routine).unwrap_or_else(|| panic!("weapon routine {routine:#04x} is not in the content"))
    }

    /// A stage's battle settings record.
    pub fn stage(&self, h: StageHandle) -> &crate::setup::StageSettings {
        &self.defs.stage(h).record
    }

    /// The pack's stage at this place in the settings table.
    pub fn stage_numbered(&self, index: u8) -> StageHandle {
        self.defs.stage_numbered(index).unwrap_or_else(|| panic!("battle settings {index:#x} are not in the content"))
    }

    /// The navi or form that has a player NameID, and its name record.
    pub fn name(&self, name_id: u16) -> &NameData {
        self.navis
            .iter()
            .filter_map(|n| n.name_record.as_ref())
            .chain(self.forms.iter().filter_map(|f| f.name_record.as_ref()))
            .find(|n| n.id == name_id)
            .unwrap_or_else(|| panic!("NameID {name_id:#x} is not a player navi"))
    }

    /// The actor record of a NameID (a player's from its name record,
    /// any other from the rules' table).
    pub fn navi_record(&self, name_id: u16) -> NaviRecord {
        self.navis
            .iter()
            .filter_map(|n| n.name_record.as_ref())
            .chain(self.forms.iter().filter_map(|f| f.name_record.as_ref()))
            .find(|n| n.id == name_id)
            .map(NameData::record)
            .or_else(|| self.rules.actor_records.get(name_id as usize).copied())
            .unwrap_or_else(|| panic!("NameID {name_id:#x} has no actor record in the content"))
    }

    /// A player NameID's sprite attach point `index`.
    pub fn attach_point(&self, name_id: u16, index: usize) -> AttachPoint {
        self.name(name_id).attach_points[index]
    }

    /// An animation's frames (empty when the sprite has no animation
    /// data: the sprite then behaves like a single held frame).
    pub fn animation(&self, sprite: SpriteId, anim: u8) -> &[AnimFrame] {
        self.animations.get(sprite, anim)
    }

    /// A generic effect's look: the pack data's, else one content defines
    /// (by the engine's number for it, `Defs::number`).
    pub fn effect(&self, id: u8) -> EffectSprite {
        self.effects
            .get(id as usize)
            .copied()
            .or_else(|| self.defs.effect_numbered(id))
            .unwrap_or_else(|| panic!("effect {id:#x} is not in the content"))
    }

    /// A hit spark's look (likewise).
    pub fn spark(&self, id: u8) -> EffectSprite {
        self.sparks
            .get(id as usize)
            .copied()
            .or_else(|| self.defs.spark_numbered(id))
            .unwrap_or_else(|| panic!("hit spark {id:#x} is not in the content"))
    }

    /// A hit-region shape (empty for regions the content doesn't have).
    pub fn region(&self, region: u8) -> &[PanelOffset] {
        match self.regions.get(region as usize) {
            Some(r) => r,
            None => match self.defs.region_numbered(region) {
                Some(Region::Panels(p)) => p,
                _ => &[],
            },
        }
    }

    /// A whole-field region's panel condition (region 0x80 and up).
    pub fn field_region(&self, region: u8) -> PanelCondition {
        let i = (region & 0x7F) as usize;
        match self.rules.field_regions.get(i) {
            Some(&c) => c,
            None => match self.defs.region_numbered(region) {
                Some(&Region::Field(c)) => c,
                _ => panic!("field region {region:#x} is not in the content"),
            },
        }
    }

    /// Collision type `index`'s flag word for `alliance`'s side, and the
    /// offset the original's lookup of its row leaves in a register (a bug
    /// code's high byte, `sub_801A00E`).
    pub fn collision_type(&self, index: u8, alliance: u8) -> (u32, u16) {
        match self.rules.collision_types.get(index as usize) {
            Some(t) => (t[alliance as usize & 1], index as u16 * 8),
            None => match self.defs.collision_numbered(index) {
                Some(t) => (t.flags[alliance as usize & 1], t.row_offset),
                None => panic!("collision type {index:#x} is not in the content"),
            },
        }
    }

    /// A panel layout.
    pub fn panel_layout(&self, layout: u8) -> &PanelLayout {
        self.panel_layouts.get(layout as usize).unwrap_or_else(|| panic!("panel layout {layout:#x} is not in the content"))
    }

    /// The Program Advances, in the order they are tried (each chip holds
    /// the recipes that make it).
    pub fn program_advances(&self) -> Vec<ProgramAdvance> {
        let mut v: Vec<(u8, ProgramAdvance)> = self
            .chips
            .iter()
            .filter_map(|c| Some((self.chip_numbered(c.id?)?, c)))
            .flat_map(|(h, c)| c.program_advances.iter().map(move |r| (r.order, h, &r.recipe)))
            .map(|(order, result, recipe)| {
                let chip = |id: ChipId| self.chip_numbered(id).unwrap_or_else(|| panic!("a Program Advance names chip {id:#x}"));
                let recipe = match recipe {
                    PaRecipe::CodeRun { chip: c, count } => Recipe::CodeRun { chip: chip(*c), count: *count },
                    PaRecipe::Sequence(ids) => Recipe::Sequence(ids.iter().map(|&c| chip(c)).collect()),
                };
                (order, ProgramAdvance { result, recipe })
            })
            .collect();
        v.sort_by_key(|(order, _)| *order);
        v.into_iter().map(|(_, pa)| pa).collect()
    }

    /// A link navi's own chip (none for MegaMan).
    pub fn navi_chip(&self, navi: NaviHandle) -> Option<CodedChip> {
        self.navi(navi).own_chip
    }

    /// An attachment kind (the attachment object's first parameter).
    pub fn attachment(&self, kind: u8) -> &AttachmentKind {
        self.objects.attachment(kind)
    }
}
