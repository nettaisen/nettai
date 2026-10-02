//! Game content: the typed data a battle runs on.
//!
//! [`Content`] holds everything the simulation reads that isn't rules
//! code: chips, navis and forms, stages, the ruleset's tables (collision
//! types, panel rules, status effects...), the identities (what an object
//! is taken for and wears), the registries of what the content defines,
//! and every sprite's animation timing. It never changes during a battle and is
//! shared between battles and their snapshots through an `Arc`:
//!
//! ```ignore
//! let (content, _report) = nettai_content::pack::load_battle(pack)?;
//! let content = Arc::new(content);
//! let setup = RoundSetup { content: content.hash(), ..setup };
//! let battle = Battle::new(setup, content.clone());
//! let chip = battle.content.chip(hand[0]);
//! ```
//!
//! See docs/design/content-pack.md for the pack's files and this API.
//!
//! The engine does no file IO: a loader outside it (nettai-content) reads a
//! content root (BN6's is the committed content/bn6) and an asset root
//! (extracted from the user's ROM by `bn6-extract content`) into this
//! model, and tests build small content sets in code.
//!
//! Content has an identity, [`Content::hash`], which a round's setup
//! carries (`RoundSetup::content`) so that netplay peers can check they
//! run the same content. The content itself is not part of a snapshot or
//! of the state digest.
//!
//! What the content defines is by handle: each key interns to a dense
//! handle at load ([`Defs`]). The engine has no navi, form or other
//! content by the original's numbers (docs/design/content-model-v2.md
//! §3.2, §12): compat's numbers are bn6-compat's.

mod chips;
mod custom;
mod defs;
mod flags;
mod identity;
mod navis;
mod reader;
mod roles;
mod rules;
mod scripts;
mod sections;
mod sprites;
mod stages;
pub mod strings;
#[cfg(any(test, feature = "test-content"))]
pub mod testing;

pub use chips::*;
pub use custom::*;
pub use defs::*;
pub use identity::{BodyPart, FieldLook, IceSize, Identity, IdentityClass, IdentityOwner, OverlayHooks, Parts};
pub use navis::*;
pub use roles::*;
pub use rules::*;
pub use scripts::*;
pub use sprites::*;
pub use stages::*;

use nettai_content_api::{ChipHandle, FormHandle, NaviHandle, StageHandle, WeaponHandle};
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

/// A banner asset as the pack identifies it (for BN6's pack, the game's UI
/// banner id). The engine names none itself: what the ruleset shows it
/// gets by role (`Roles::banner`) or from a definition (a navi's win and
/// lose banners, the banners that hold).
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
    /// The ruleset's tables.
    pub rules: Rules,
    /// Every sprite's animation timing.
    pub animations: Animations,
    /// The assets content can name (`asset.sprite("bomb")`): the loader
    /// fills it from the pack's asset names (docs/design/
    /// content-model-v2.md §6.3).
    pub assets: nettai_content_api::AssetNames,
    /// The pack's scripts (see `scripts`).
    pub scripts: Scripts,
    /// What the content defines: the registries and their handles, and the
    /// functions the scripts implement (see `defs`). Made from the rest by
    /// [`Content::define`].
    pub defs: Defs,
    /// The content's own language's strings (`strings`: a content root's
    /// `locales/`), which the define phase counts the chatbox's timing
    /// from; presentation besides, left out of the hash.
    pub strings: strings::Strings,
}

impl Content {
    /// Run the define phase over the scripts and build the registries
    /// (docs/design/content-model-v2.md §7.3). A battle needs defined
    /// content; loaders call this once the data and scripts are in.
    pub fn define(&mut self) -> Result<(), nettai_content_api::ContentError> {
        if self.scripts.modules.is_empty() {
            let definitions = Default::default();
            sections::build(self, &definitions)?;
            self.defs = Defs::build(self, definitions)?;
            return Ok(());
        }
        let (definitions, compiled) = nettai_luau::define(&self.scripts.pack(), &self.assets, nettai_luau::Options::default())?;
        self.scripts.compiled = CompiledModules(compiled);
        // The rule sections into the ruleset's typed tables.
        sections::build(self, &definitions)?;
        self.defs = Defs::build(self, definitions)?;
        self.count_strings();
        Ok(())
    }

    /// What the battle reads of the content's own strings, into the
    /// records: the descriptions' lines, the no-running messages'
    /// characters per line and which of them move the speaker's mouth
    /// (`strings`).
    fn count_strings(&mut self) {
        let w = &self.strings;
        for d in &mut self.defs.chips {
            let description = w.chip(&d.key).and_then(|c| c.description.as_deref());
            d.record.description_lines = strings::description_lines(description);
        }
        for d in &mut self.defs.forms {
            let description = w.form(&d.key).and_then(|f| f.description.as_deref());
            d.record.description_lines = strings::description_lines(description);
        }
        for d in &mut self.defs.navis {
            let message = w.navi(&d.key).and_then(|n| n.run_message.as_deref()).unwrap_or("");
            let m = &mut d.record.run_message;
            (m.counts, m.talking) = if message.is_empty() { (Vec::new(), [0; 3]) } else { (strings::message_counts(message), strings::talking(message)) };
        }
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

    /// The first navi that changes form (MegaMan), if the content has one
    /// (stand-in setups).
    pub fn form_changing_navi(&self) -> Option<NaviHandle> {
        self.defs.navis.iter().position(|n| n.record.changes_form()).map(|i| NaviHandle(i as u16))
    }

    /// The navi with this key (setups by name, tools and tests).
    pub fn navi_by_key(&self, key: &str) -> NaviHandle {
        self.defs.navi_by_key(key).unwrap_or_else(|| panic!("navi {key:?} is not in the content"))
    }

    /// One of MegaMan's forms.
    pub fn form(&self, h: FormHandle) -> &FormData {
        &self.defs.form(h).record
    }

    /// The form with this key (setups by name, tools and tests).
    pub fn form_by_key(&self, key: &str) -> FormHandle {
        self.defs.form_by_key(key).unwrap_or_else(|| panic!("form {key:?} is not in the content"))
    }

    /// The base form: what a navi that has not changed form is in (a link
    /// navi always).
    pub fn base_form(&self) -> FormHandle {
        self.defs.base_form.unwrap_or_else(|| panic!("the content has no base form"))
    }

    /// The identity of a navi in a form: the form's, or in the base form
    /// (which has none of its own) the navi's.
    pub fn form_identity(&self, navi: NaviHandle, form: FormHandle) -> Option<nettai_content_api::IdentityHandle> {
        self.form(form).identity.or(self.navi(navi).identity)
    }

    /// The battle sprite of a navi in a form (`sub_800FC9E`): its form's
    /// if it changes form, else its own.
    pub fn navi_sprite(&self, navi: NaviHandle, form: FormHandle) -> SpriteId {
        let n = self.navi(navi);
        if n.changes_form() { self.form(form).sprite } else { n.sprite }
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
    pub fn status(&self, h: nettai_content_api::StatusHandle) -> StatusEffect {
        self.defs.statuses[h.index()].effect
    }

    /// A Beast Out lock-on mode.
    pub fn lockon(&self, h: nettai_content_api::LockonHandle) -> &LockonMode {
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

    /// An identity; what an object without one is taken for, for none.
    pub fn identity(&self, h: Option<nettai_content_api::IdentityHandle>) -> &Identity {
        match h {
            Some(h) => &self.defs.identities[h.index()],
            None => Identity::none(),
        }
    }

    /// The actor record of an identity (`sub_80182B4`).
    pub fn navi_record(&self, h: Option<nettai_content_api::IdentityHandle>) -> NaviRecord {
        self.identity(h).record
    }

    /// An identity's sprite attach point `index` (`sub_8018810`): a navi's;
    /// every point of a field object is (0, 7), which the caller handles.
    pub fn attach_point(&self, h: Option<nettai_content_api::IdentityHandle>, index: usize) -> AttachPoint {
        let id = self.identity(h);
        *id.attach_points.get(index).unwrap_or_else(|| {
            panic!("identity {:?} has no attach point {index} (sub_8018810 reads another actor's table)", id.key)
        })
    }

    /// The identity with this key (tools and tests).
    pub fn identity_by_key(&self, key: &str) -> nettai_content_api::IdentityHandle {
        self.defs.identity_by_key(key).unwrap_or_else(|| panic!("identity {key:?} is not in the content"))
    }

    /// An animation's frames (empty when the sprite has no animation
    /// data: the sprite then behaves like a single held frame).
    pub fn animation(&self, sprite: SpriteId, anim: u8) -> &[AnimFrame] {
        self.animations.get(sprite, anim)
    }

    /// A one-shot effect's look.
    pub fn effect(&self, h: nettai_content_api::EffectHandle) -> EffectSprite {
        self.defs.effects[h.index()]
    }

    /// A hit spark's look.
    pub fn spark(&self, h: nettai_content_api::SparkHandle) -> EffectSprite {
        self.defs.sparks[h.index()]
    }

    /// A hit region.
    pub fn region(&self, h: nettai_content_api::RegionHandle) -> &Region {
        &self.defs.regions[h.index()]
    }

    /// The panels around its anchor a hit region covers; none for no
    /// region and for a whole-field one (which has no offsets: the
    /// original's shape lookup doesn't know them).
    pub fn region_offsets(&self, region: Option<nettai_content_api::RegionHandle>) -> &[PanelOffset] {
        match region.map(|h| self.region(h)) {
            Some(Region::Panels(p)) => p,
            _ => &[],
        }
    }

    /// A collision type's flag word for `alliance`'s side, and the offset
    /// the original's lookup of its row leaves in a register (a bug code's
    /// high byte, `sub_801A00E`).
    pub fn collision_type(&self, h: nettai_content_api::CollisionHandle, alliance: u8) -> (u32, u16) {
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
}
