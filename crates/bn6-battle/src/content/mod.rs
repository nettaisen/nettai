//! Game content: the typed data a battle runs on.
//!
//! [`Content`] holds everything the simulation reads that isn't rules
//! code: chips, navis and forms, stages, the ruleset's tables (collision
//! types, panel rules, status effects...), object data (rocks,
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
pub mod legacy;
mod navis;
mod objects;
mod roles;
mod rules;
mod scripts;
mod sprites;
mod stages;
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
pub use stages::*;

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
    /// Navis by [`Navi`](crate::setup::Navi) number (MegaMan is 0).
    pub navis: Vec<NaviData>,
    /// MegaMan's forms by [`Form`](crate::setup::Form) number (0 is the
    /// base form).
    pub forms: Vec<FormData>,
    /// The ruleset's tables.
    pub rules: Rules,
    /// Object kinds' data.
    pub objects: ObjectData,
    /// Every sprite's animation timing.
    pub animations: Animations,
    /// The assets content can name (`asset.sprite("bomb")`): the loader
    /// fills it from the pack's asset names (docs/design/
    /// content-model-v2.md §6.3).
    pub assets: bn6_content_api::AssetNames,
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
        if self.scripts.modules.is_empty() {
            let definitions = Default::default();
            let legacy = legacy::build(self, &definitions)?;
            self.defs = Defs::build(self, definitions, &legacy)?;
            return Ok(());
        }
        let mut data = crate::behavior::script_data(self);
        for _ in 0..3 {
            let (definitions, compiled) =
                bn6_luau::define(&self.scripts.pack(), &data, &self.assets, bn6_luau::Options::default())?;
            self.scripts.compiled = CompiledModules(compiled);
            // What registration by number reads of the definitions: the
            // tables by number, and the pack's chips, navis, forms, weapons
            // and stages.
            let legacy = legacy::build(self, &definitions)?;
            // v1 modules see those tables as the `data` global, some while
            // they load: when the definitions made them, the modules run
            // again on what they will see at run time (once: the tables
            // come from the definitions alone, so they come out the same).
            let built = crate::behavior::script_data(self);
            if built == data {
                self.defs = Defs::build(self, definitions, &legacy)?;
                return Ok(());
            }
            data = built;
        }
        Err(bn6_content_api::ContentError::new(
            "the tables the definitions build keep changing what the modules define (a module defines by `data` what the tables come from)",
        ))
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

    /// What a chip's record names, by handle.
    pub fn chip_links(&self, h: ChipHandle) -> &ChipLinks {
        &self.defs.chip(h).links
    }

    /// The chip a zeroed chip field reads (`roles.chips.zeroed`: the chip
    /// the original's chip 0 is), if the content has one.
    pub fn zeroed_chip(&self) -> Option<ChipHandle> {
        self.defs.roles.try_chip(ChipRole::Zeroed)
    }

    /// The chip a chip field names: itself, or for none (a zeroed field)
    /// the zeroed chip, which is what the game reads.
    pub fn chip_or_zeroed(&self, h: Option<ChipHandle>) -> ChipHandle {
        h.unwrap_or_else(|| self.defs.roles.chip(ChipRole::Zeroed))
    }

    /// The record a chip field names (see [`Content::chip_or_zeroed`]).
    pub fn chip_field(&self, h: Option<ChipHandle>) -> &ChipData {
        self.chip(self.chip_or_zeroed(h))
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

    /// A weapon.
    pub fn weapon(&self, h: WeaponHandle) -> &WeaponDef {
        self.defs.weapon(h)
    }

    /// The weapon with this key (setups by name, tools and tests).
    pub fn weapon_by_key(&self, key: &str) -> WeaponHandle {
        self.defs.weapon_by_key(key).unwrap_or_else(|| panic!("weapon {key:?} is not in the content"))
    }

    /// A status effect.
    pub fn status(&self, h: bn6_content_api::StatusHandle) -> StatusEffect {
        self.defs.statuses[h.index()].effect
    }

    /// A Beast Out lock-on mode.
    pub fn lockon(&self, h: bn6_content_api::LockonHandle) -> &LockonMode {
        &self.defs.lockons[h.index()].mode
    }

    /// A stage.
    pub fn stage(&self, h: StageHandle) -> &StageData {
        &self.defs.stage(h).record
    }

    /// The stage with this key (setups by name, tools and tests).
    pub fn stage_by_key(&self, key: &str) -> StageHandle {
        self.defs.stage_by_key(key).unwrap_or_else(|| panic!("stage {key:?} is not in the content"))
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

    /// A one-shot effect's look.
    pub fn effect(&self, h: bn6_content_api::EffectHandle) -> EffectSprite {
        self.defs.effects[h.index()]
    }

    /// A hit spark's look.
    pub fn spark(&self, h: bn6_content_api::SparkHandle) -> EffectSprite {
        self.defs.sparks[h.index()]
    }

    /// A hit region.
    pub fn region(&self, h: bn6_content_api::RegionHandle) -> &Region {
        &self.defs.regions[h.index()]
    }

    /// The panels around its anchor a hit region covers; none for no
    /// region and for a whole-field one (which has no offsets: the
    /// original's shape lookup doesn't know them).
    pub fn region_offsets(&self, region: Option<bn6_content_api::RegionHandle>) -> &[PanelOffset] {
        match region.map(|h| self.region(h)) {
            Some(Region::Panels(p)) => p,
            _ => &[],
        }
    }

    /// A collision type's flag word for `alliance`'s side, and the offset
    /// the original's lookup of its row leaves in a register (a bug code's
    /// high byte, `sub_801A00E`).
    pub fn collision_type(&self, h: bn6_content_api::CollisionHandle, alliance: u8) -> (u32, u16) {
        let t = &self.defs.collisions[h.index()];
        (t.flags[alliance as usize & 1], t.row_offset)
    }

    /// The Program Advances, in the order they are tried (each chip holds
    /// the recipes that make it).
    pub fn program_advances(&self) -> &[ProgramAdvance] {
        &self.defs.program_advances
    }

    /// A link navi's own chip (none for MegaMan).
    pub fn navi_chip(&self, navi: NaviHandle) -> Option<(ChipHandle, ChipCode)> {
        self.defs.navi(navi).own_chip
    }

    /// An attachment kind (the attachment object's first parameter).
    pub fn attachment(&self, kind: u8) -> &AttachmentKind {
        self.objects.attachment(kind)
    }
}
