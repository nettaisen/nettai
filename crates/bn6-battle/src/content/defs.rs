//! What the content defines (docs/design/content-model-v2.md): the
//! registries the engine reads, with their handles, and the functions the
//! runtime binds.
//!
//! A registry holds three sorts of entries while the migration runs:
//!
//! - the engine's own (its object kinds, keyed `engine/...`);
//! - content registered by number from the pack's data (an object folder's
//!   `[kind]`, a chip's `script`, a `weapon.toml`), keyed from that data
//!   (the kind's folder name; `v1/action-12`, `v1/weapon-02`); and the
//!   pack's records content doesn't define yet: its chips, navis, forms
//!   and stages (`v1/chip-036`, `v1/navi-01`, `v1/form-0c`, `v1/stage-11`)
//!   and every weapon routine number (`v1/weapon-29`);
//! - content's definitions (`define.kind { ... }`), keyed by their keys.
//!
//! Each registry's keys are sorted byte-wise; an entry's handle is its
//! place. Registration by number still reaches its entries by the numbers
//! its own data gives (an object slot, an action number, a weapon routine,
//! a hook by subtype), until the migration's last step deletes it.
//!
//! The engine never learns the original's numbers for what content
//! defines: an object records its kind's handle, a navi its content
//! action's, and the validator (`bn6-compat`) maps them to the original's
//! numbers with compat (docs/design/content-model-v2.md §7.3).

use std::collections::BTreeMap;

use bn6_content_api::{
    ActionHandle, ChipHandle, ContentError, Data, Definition, Definitions, FnId, FnSource, FormHandle, KindHandle,
    NaviHandle, Pool, RecordHandle, Registry, Schema, StageHandle, StateId, WeaponHandle,
};

use super::{
    ChipClass, ChipCode, ChipData, ChipFamily, ChipFlags, ChipId, ChipModifier, Content, DIMMING_CHIP_ACTION, Element,
    ExtraChipFlags, FormData, INSTANT_CHIP_ACTION, NAVI_CHIP_ACTION, NaviData,
};
use super::roles::{ActionRole, HookRole, KindRole, RoleAction, RoleKind, Roles};
use crate::setup::{Form, Navi, StageSettings};
use crate::kinds::{ENGINE_KINDS, EngineKind};

/// Who implements an object kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KindImpl {
    /// The engine's own Rust.
    Engine(EngineKind),
    /// Content: its `update`.
    Script { update: FnId },
}

/// An object kind.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KindDef {
    pub key: String,
    /// Its pool: update order and slot limits.
    pub pool: Pool,
    pub implementation: KindImpl,
    /// A content kind's state layout.
    pub schema: StateId,
    /// The object slot (pool and index) registration by number reaches it
    /// by: the pack's `object.toml`s'. The engine's kinds and the kinds
    /// content defines have none (the validator has their slots by key).
    pub slot: Option<(Pool, u8)>,
    /// What places it when a stage's actor list names it (`kind.place`, or
    /// a v1 module's `actor_list_entry`).
    pub place: Option<FnId>,
}

/// A navi action content implements.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionDef {
    pub key: String,
    pub update: FnId,
    /// Its state layout. Actions of one layout continue each other's
    /// attack state, as the original's actions share theirs.
    pub schema: StateId,
    /// The action number registration by number gives it (a chip record's
    /// or a `weapon.toml`'s): the navi's CurAction while it runs. An action
    /// content defines has none; the navi's CurAction is then
    /// [`crate::kinds::player::CONTENT_ACTION`].
    pub number: Option<u8>,
}

/// A weapon: what a button's weapon does.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WeaponDef {
    pub key: String,
    pub name: String,
    /// `setup(navi) -> action`; none for a routine number nothing
    /// implements yet.
    pub setup: Option<FnId>,
    /// The weapon routine numbers that name it: a `weapon.toml`'s, the
    /// number of a routine nothing implements, or the routines a weapon
    /// content defines takes with its transitional `legacy = { routines }`
    /// marker (the pack's forms and the ruleset still name weapons by
    /// number). Registration by number finds it by any of them.
    pub routines: Vec<u8>,
    /// Its first routine number: what the ruleset's numeric logic asks
    /// until phase C (none for a weapon content defines without routines).
    pub number: Option<u8>,
    /// Ticks to a full charge by Charge stat, for a weapon content defines
    /// (the pack's routines' are the charge table's, by number).
    pub charge_ticks: Vec<u16>,
    /// The instant effect its action (the instant chips' action) runs: a
    /// weapon that names one no chip has (TenguCross's wind).
    pub instant: Option<FnId>,
}

/// How a chip is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChipUsage {
    /// A navi action: the chip's attack.
    Action(ActionHandle),
    /// A cut-in chip: its controller's spawner, `(user, spec) -> controller`.
    Dimming(FnId),
    /// A navi chip: its navi's spawner, `(user, controller, spec) -> navi`.
    Navi(FnId),
    /// An instant chip: its effect, `(user, spec)`.
    Instant(FnId),
    /// A pack record whose action or subtype nothing implements yet: using
    /// it is "not implemented yet" where the original runs it.
    Unported(Unported),
}

/// What a pack record names that nothing implements yet (the numbers are
/// for the error).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unported {
    /// A navi action.
    Action(u8),
    /// A dimming chip's controller, by subtype (`off_802CCB4`).
    Dimming(u8),
    /// A navi chip's navi, by subtype (`off_802CD5C`).
    Navi(u8),
    /// An instant chip's effect, by subtype (`off_80EC3F0`).
    Instant(u8),
}

/// A chip: the pack's record (registration by number, keyed `v1/chip-036`)
/// or what content defines.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChipDef {
    pub key: String,
    /// Its record. A definition's has no `id`: the engine never learns the
    /// original's number for what content defines.
    pub record: ChipData,
    /// How it is used: a definition's own use, or a pack record's, which
    /// registration by number resolves from its action and subtype.
    pub usage: ChipUsage,
}

/// A navi: the pack's record (keyed `v1/navi-01`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NaviDef {
    pub key: String,
    pub record: NaviData,
}

/// One of MegaMan's forms: the pack's record (keyed `v1/form-0c`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FormDef {
    pub key: String,
    pub record: FormData,
}

/// A stage: the pack's battle settings record (keyed `v1/stage-11` by its
/// place in the settings table).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StageDef {
    pub key: String,
    pub record: StageSettings,
    /// Its place in the pack's settings table.
    pub number: u8,
}

/// A collision type content defines (`define.collision`): what an object is
/// or what it hits, as the flag words for each side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollisionTypeDef {
    pub flags: [u32; 2],
    /// The offset the original's lookup of this type's row leaves in a
    /// register (the row's index times 8), which a damage word's bug code
    /// takes as its high byte (`sub_801A00E`): a quirk materialized where
    /// it is used (docs/design/content-model-v2.md §3.3).
    pub row_offset: u16,
}


/// A record only content reads: the engine keeps its handle and type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordDef {
    pub key: String,
    pub record_type: String,
}

/// A content state layout.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SchemaDef {
    pub key: String,
    pub schema: Schema,
}

/// The key of the layout of content that declares no state.
pub const NO_STATE: &str = "";

/// What the content defines. Built by [`Content::define`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Defs {
    /// Whether the content was defined (a battle needs it).
    pub defined: bool,
    /// What the define phase read.
    pub definitions: Definitions,
    /// Each definition's handle, in `definitions.defs`' order.
    pub handles: Vec<u16>,
    pub kinds: Vec<KindDef>,
    pub actions: Vec<ActionDef>,
    pub weapons: Vec<WeaponDef>,
    pub chips: Vec<ChipDef>,
    pub navis: Vec<NaviDef>,
    pub forms: Vec<FormDef>,
    pub stages: Vec<StageDef>,
    pub records: Vec<RecordDef>,
    /// One-shot effects' and hit sparks' looks content defines, by handle.
    /// Each has the engine's number after the pack data's
    /// (`Content::effect`, `Content::spark`): see [`Defs::number`].
    pub effects: Vec<super::EffectSprite>,
    pub sparks: Vec<super::EffectSprite>,
    /// Hit regions content defines, by handle, with the engine's number for
    /// each (a shape's after the pack data's shapes, a whole-field region's
    /// after its field regions, from 0x80).
    pub regions: Vec<(super::Region, u8)>,
    /// Collision types content defines, by handle: the engine's number for
    /// each comes after the pack data's (`Content::collision_type`).
    pub collisions: Vec<CollisionTypeDef>,
    /// What the ruleset needs from content by role (`define.roles`).
    pub roles: Roles,
    /// The engine's numbers for the first effect, spark and collision type
    /// content defines.
    effect_base: u8,
    spark_base: u8,
    collision_base: u8,
    /// State layouts by [`StateId`].
    pub schemas: Vec<SchemaDef>,
    /// The functions the runtime binds, by [`FnId`].
    pub functions: Vec<FnSource>,
    kind_keys: BTreeMap<String, KindHandle>,
    /// The engine's kinds, in [`ENGINE_KINDS`]' order.
    engine: Vec<KindHandle>,
    /// Kinds by object slot: `pool * 256 + index` (registration by number).
    kind_slots: Vec<Option<KindHandle>>,
    /// Actions by number (registration by number).
    action_numbers: Vec<Option<ActionHandle>>,
    /// Weapons by routine number (registration by number).
    weapon_ids: Vec<Option<WeaponHandle>>,
    /// The pack's records by their numbers (registration by number).
    chip_numbers: BTreeMap<ChipId, ChipHandle>,
    navi_numbers: Vec<Option<NaviHandle>>,
    form_numbers: Vec<Option<FormHandle>>,
    stage_numbers: Vec<Option<StageHandle>>,
    /// Keys by registry, for the codecs.
    chip_keys: BTreeMap<String, ChipHandle>,
    weapon_keys: BTreeMap<String, WeaponHandle>,
}

fn pool_index(pool: Pool) -> usize {
    Pool::ALL.iter().position(|&p| p == pool).expect("a pool")
}

impl Defs {
    /// The kind with this key.
    pub fn kind_by_key(&self, key: &str) -> Option<KindHandle> {
        self.kind_keys.get(key).copied()
    }

    pub fn kind(&self, h: KindHandle) -> &KindDef {
        &self.kinds[h.index()]
    }

    /// The kind registration by number puts in an object slot.
    pub fn kind_at(&self, pool: Pool, index: u8) -> Option<KindHandle> {
        self.kind_slots.get(pool_index(pool) * 256 + index as usize).copied().flatten()
    }

    /// The engine's kind `kind`.
    pub fn engine(&self, kind: EngineKind) -> KindHandle {
        let i = ENGINE_KINDS.iter().position(|e| e.0 == kind).expect("every engine kind is listed");
        self.engine[i]
    }

    /// Which of the engine's kinds `h` is, if it is one.
    pub fn engine_kind(&self, h: KindHandle) -> Option<EngineKind> {
        match self.kinds.get(h.index())?.implementation {
            KindImpl::Engine(k) => Some(k),
            KindImpl::Script { .. } => None,
        }
    }

    pub fn action(&self, h: ActionHandle) -> &ActionDef {
        &self.actions[h.index()]
    }

    /// The action registered by number.
    pub fn action_numbered(&self, number: u8) -> Option<ActionHandle> {
        self.action_numbers.get(number as usize).copied().flatten()
    }

    /// The action with this key.
    pub fn action_by_key(&self, key: &str) -> Option<ActionHandle> {
        self.actions.binary_search_by(|a| a.key.as_str().cmp(key)).ok().map(|i| ActionHandle(i as u16))
    }

    /// The weapon of a weapon routine number.
    pub fn weapon_numbered(&self, id: u8) -> Option<WeaponHandle> {
        self.weapon_ids.get(id as usize).copied().flatten()
    }

    /// The pack's chip record with this id (registration by number).
    pub fn chip_numbered(&self, id: ChipId) -> Option<ChipHandle> {
        self.chip_numbers.get(&id).copied()
    }

    /// The chip with this key.
    pub fn chip_by_key(&self, key: &str) -> Option<ChipHandle> {
        self.chip_keys.get(key).copied()
    }

    pub fn chip(&self, h: ChipHandle) -> &ChipDef {
        &self.chips[h.index()]
    }

    /// The pack's navi with this number.
    pub fn navi_numbered(&self, navi: Navi) -> Option<NaviHandle> {
        self.navi_numbers.get(navi.index()).copied().flatten()
    }

    pub fn navi(&self, h: NaviHandle) -> &NaviDef {
        &self.navis[h.index()]
    }

    /// The navi with this key.
    pub fn navi_by_key(&self, key: &str) -> Option<NaviHandle> {
        self.navis.binary_search_by(|n| n.key.as_str().cmp(key)).ok().map(|i| NaviHandle(i as u16))
    }

    /// The pack's form with this number.
    pub fn form_numbered(&self, form: Form) -> Option<FormHandle> {
        self.form_numbers.get(form.index()).copied().flatten()
    }

    pub fn form(&self, h: FormHandle) -> &FormDef {
        &self.forms[h.index()]
    }

    /// The form with this key.
    pub fn form_by_key(&self, key: &str) -> Option<FormHandle> {
        self.forms.binary_search_by(|f| f.key.as_str().cmp(key)).ok().map(|i| FormHandle(i as u16))
    }

    /// The pack's stage at this place in the settings table.
    pub fn stage_numbered(&self, index: u8) -> Option<StageHandle> {
        self.stage_numbers.get(index as usize).copied().flatten()
    }

    pub fn stage(&self, h: StageHandle) -> &StageDef {
        &self.stages[h.index()]
    }

    /// The stage with this key.
    pub fn stage_by_key(&self, key: &str) -> Option<StageHandle> {
        self.stages.binary_search_by(|s| s.key.as_str().cmp(key)).ok().map(|i| StageHandle(i as u16))
    }

    pub fn weapon(&self, h: WeaponHandle) -> &WeaponDef {
        &self.weapons[h.index()]
    }

    /// The weapon with this key.
    pub fn weapon_by_key(&self, key: &str) -> Option<WeaponHandle> {
        self.weapon_keys.get(key).copied()
    }


    /// A record's handle by key.
    pub fn record(&self, key: &str) -> Option<RecordHandle> {
        self.records.binary_search_by(|r| r.key.as_str().cmp(key)).ok().map(|i| RecordHandle(i as u16))
    }

    /// The engine's number for a definition the ruleset still stores as a
    /// byte (an effect, a spark, a region, a collision type): its own,
    /// after the pack data's, never an original number.
    pub fn number(&self, registry: Registry, h: u16) -> Option<u8> {
        let i = h as usize;
        match registry {
            Registry::Effect => (i < self.effects.len()).then(|| self.effect_base + h as u8),
            Registry::Spark => (i < self.sparks.len()).then(|| self.spark_base + h as u8),
            Registry::Region => self.regions.get(i).map(|&(_, n)| n),
            Registry::Collision => (i < self.collisions.len()).then(|| self.collision_base + h as u8),
            _ => None,
        }
    }

    /// A defined effect, spark or collision type by the engine's number.
    pub(crate) fn effect_numbered(&self, n: u8) -> Option<super::EffectSprite> {
        n.checked_sub(self.effect_base).and_then(|i| self.effects.get(i as usize)).copied()
    }

    pub(crate) fn spark_numbered(&self, n: u8) -> Option<super::EffectSprite> {
        n.checked_sub(self.spark_base).and_then(|i| self.sparks.get(i as usize)).copied()
    }

    pub(crate) fn collision_numbered(&self, n: u8) -> Option<CollisionTypeDef> {
        n.checked_sub(self.collision_base).and_then(|i| self.collisions.get(i as usize)).copied()
    }

    /// A defined region by the engine's number.
    pub(crate) fn region_numbered(&self, n: u8) -> Option<&super::Region> {
        self.regions.iter().find(|(_, m)| *m == n).map(|(r, _)| r)
    }

    /// The layout with this key.
    pub fn schema_named(&self, key: &str) -> Option<StateId> {
        self.schemas.binary_search_by(|s| s.key.as_str().cmp(key)).ok().map(|i| StateId(i as u16))
    }

    pub fn schema(&self, id: StateId) -> &Schema {
        &self.schemas[id.0 as usize].schema
    }
}

/// The builder's view of one registry while it is filled.
struct Entries<T> {
    registry: Registry,
    entries: Vec<(String, T, String)>,
}

impl<T> Entries<T> {
    fn new(registry: Registry) -> Entries<T> {
        Entries { registry, entries: Vec::new() }
    }

    /// Add an entry; `whose` says where it comes from, for messages.
    fn add(&mut self, key: String, v: T, whose: String) {
        self.entries.push((key, v, whose));
    }

    /// The entries in key order (their handles); two with one key are an
    /// error.
    fn sorted(mut self) -> Result<Vec<(String, T)>, ContentError> {
        self.entries.sort_by(|a, b| a.0.cmp(&b.0));
        for w in self.entries.windows(2) {
            if w[0].0 == w[1].0 {
                return Err(ContentError::new(format!(
                    "{} {:?} is both {} and {}",
                    self.registry, w[0].0, w[0].2, w[1].2
                )));
            }
        }
        Ok(self.entries.into_iter().map(|(k, v, _)| (k, v)).collect())
    }
}

/// Functions, each once.
#[derive(Default)]
struct Functions {
    list: Vec<FnSource>,
    ids: BTreeMap<FnSource, FnId>,
}

impl Functions {
    fn id(&mut self, f: FnSource) -> FnId {
        if let Some(&id) = self.ids.get(&f) {
            return id;
        }
        let id = FnId(self.list.len() as u32);
        self.ids.insert(f.clone(), id);
        self.list.push(f);
        id
    }
}

/// A byte field of a definition (0 when absent).
fn byte(d: &Definition, field: &str) -> Result<u8, ContentError> {
    let what = |e: String| ContentError::new(format!("{}.luau: {} {}: {e}", d.module, d.registry, d.key));
    match d.spec.field(field) {
        Data::Nil => Ok(0),
        Data::Int(i) => u8::try_from(*i).map_err(|_| what(format!("`{field}` {i} is not a byte"))),
        _ => Err(what(format!("`{field}` is not a number"))),
    }
}

/// A definition's function slot at `path`, which must hold a function.
fn slot(d: &Definition, path: &str) -> Result<FnSource, ContentError> {
    match d.spec.field(path) {
        Data::Function => Ok(FnSource::slot(d.registry, &d.key, path)),
        Data::Nil => Err(ContentError::new(format!("{}.luau: {} {} needs `{path}`", d.module, d.registry, d.key))),
        _ => Err(ContentError::new(format!("{}.luau: {} {}'s `{path}` is not a function", d.module, d.registry, d.key))),
    }
}

/// A module's exported function (registration by module).
fn export(definitions: &Definitions, module: &str, name: &str, whose: &str) -> Result<FnSource, ContentError> {
    let m = definitions
        .module(module)
        .ok_or_else(|| ContentError::new(format!("{whose} names the script {module}.luau, which isn't in the pack")))?;
    if !m.functions.iter().any(|f| f == name) {
        return Err(ContentError::new(format!("{whose}: {module}.luau doesn't export the function `{name}`")));
    }
    Ok(FnSource::export(module, name))
}

/// The action a chip record's module exports as `action`, which the record
/// runs as its own (a chip definition's `action`), whatever its action
/// number names: the link navis' chips, whose number (0x0A) is the user's
/// own action table's (docs/design/content-model-v2.md §12, "A record's
/// action by its module"). None for a module registered by number (its
/// `update` and `state`); a module can't be both.
fn record_action<'d>(definitions: &'d Definitions, module: &str, whose: &str) -> Result<Option<&'d str>, ContentError> {
    let Some(m) = definitions.module(module) else { return Ok(None) };
    let Some(action) = &m.action else { return Ok(None) };
    if m.functions.iter().any(|f| f == "update") {
        return Err(ContentError::new(format!(
            "{whose}: {module}.luau exports both an action ({action}) and `update`: a chip runs one"
        )));
    }
    Ok(Some(action))
}

/// A chip definition's record (docs/design/content-model-v2.md §3.1): the
/// fields the engine reads. Damage formulas, Program Advance recipes, dark
/// chips' substitutes and lock-on modes by definition come with the v2 API
/// (steps 4 and 10); a definition that gives one is refused.
///
/// The transitional `legacy = { subtype, params }` marker gives the record
/// the original's subtype and parameter bytes, for what still reads them
/// of a chip besides its own action (no BN6 chip needs it since
/// SlashCross's charged slash asks a sword's slash for its own); its
/// `action` and `script` (a behaviour still a v1 module) come with step 5.
fn chip_record(d: &Definition) -> Result<ChipData, ContentError> {
    let what = |e: String| ContentError::new(format!("{}.luau: chip {}: {e}", d.module, d.key));
    let spec = &d.spec;
    let int = |field: &str, max: i64| -> Result<i64, ContentError> {
        match spec.field(field) {
            Data::Nil => Ok(0),
            Data::Int(i) if (0..=max).contains(i) => Ok(*i),
            other => Err(what(format!("`{field}` is {other:?}, not a number from 0 to {max}"))),
        }
    };
    let name_of = |field: &str| -> Result<Option<String>, ContentError> {
        match spec.field(field) {
            Data::Nil => Ok(None),
            Data::Str(s) => Ok(Some(s.clone())),
            other => Err(what(format!("`{field}` is {other:?}, not a name"))),
        }
    };
    fn named<T: serde::de::DeserializeOwned>(name: &str) -> Option<T> {
        serde_json::from_value(serde_json::Value::String(name.to_string())).ok()
    }
    let enum_field = |field: &str| -> Result<Option<String>, ContentError> { name_of(field) };
    let element = match enum_field("element")? {
        None => Element::Null,
        Some(n) => named(&n).ok_or_else(|| what(format!("`element` {n:?} is not an element")))?,
    };
    let family = match enum_field("family")? {
        None => ChipFamily::Null,
        Some(n) => named(&n).ok_or_else(|| what(format!("`family` {n:?} is not a chip family")))?,
    };
    let class = match enum_field("class")? {
        None => ChipClass::Standard,
        Some(n) => named(&n).ok_or_else(|| what(format!("`class` {n:?} is not a chip class")))?,
    };
    let modifier: Option<ChipModifier> = match enum_field("modifier")? {
        None => None,
        Some(n) => Some(named(&n).ok_or_else(|| what(format!("`modifier` {n:?} is not a modifier")))?),
    };
    let flags = |field: &str, names: &[(u32, &str)]| -> Result<u8, ContentError> {
        let mut bits = 0u8;
        match spec.field(field) {
            Data::Nil => {}
            Data::List(items) => {
                for item in items {
                    let n = item.str().ok_or_else(|| what(format!("`{field}` holds {item:?}, not a flag's name")))?;
                    let (bit, _) = names.iter().find(|(_, f)| *f == n).ok_or_else(|| what(format!("`{field}`: no flag {n:?}")))?;
                    bits |= *bit as u8;
                }
            }
            other => Err(what(format!("`{field}` is {other:?}, not a list of flags")))?,
        }
        Ok(bits)
    };
    let codes = match spec.field("codes") {
        Data::Nil => Vec::new(),
        Data::List(items) => items
            .iter()
            .map(|c| {
                c.str()
                    .and_then(|s| {
                        let mut chars = s.chars();
                        match (chars.next().and_then(ChipCode::from_letter), chars.next()) {
                            (Some(code), None) => Some(code),
                            _ => None,
                        }
                    })
                    .ok_or_else(|| what(format!("`codes` holds {c:?}, not a code (A-Z or *)")))
            })
            .collect::<Result<Vec<_>, _>>()?,
        other => return Err(what(format!("`codes` is {other:?}, not a list"))),
    };
    for field in ["program_advances", "dark_substitute"] {
        if !spec.field(field).is_nil() {
            return Err(what(format!("`{field}` in a definition comes with the v2 API")));
        }
    }
    let damage = match spec.field("damage") {
        Data::Nil => 0,
        Data::Int(i) if (0..1000).contains(i) => *i as u16,
        other => return Err(what(format!("`damage` is {other:?}: a number below 1000 (formulas come with the v2 API)"))),
    };
    let (beast_lockon, lockon_mode) = match spec.field("beast") {
        Data::Nil => (false, 0),
        beast => {
            let rush = !matches!(beast.field("rush"), Data::Bool(false));
            let mode = match beast.field("lockon") {
                Data::Nil => 0,
                Data::Int(i) if (0..=0xFF).contains(i) => *i as u8,
                other => return Err(what(format!("`beast.lockon` is {other:?}: a lock-on mode's number until the v2 API"))),
            };
            (rush, mode)
        }
    };
    let (subtype, params) = legacy_bytes(spec).map_err(|e| what(e))?;
    let library = spec.field("library");
    let lib_int = |field: &str| library.field(field).int().unwrap_or(0);
    Ok(ChipData {
        id: None,
        name: name_of("name")?.unwrap_or_else(|| d.key.clone()),
        description: name_of("description")?,
        codes,
        element,
        rarity: int("rarity", 0xFF)? as u8,
        family,
        class,
        mb: int("mb", 0xFF)? as u8,
        flags: ChipFlags(flags("flags", ChipFlags::NAMES)?),
        hit_param: int("hit_param", 0xFF)? as u8,
        action: 0,
        subtype,
        beast_lockon,
        params,
        lockout: int("lockout", 0xFF)? as u8,
        extra_flags: ExtraChipFlags(flags("extra_flags", ExtraChipFlags::NAMES)?),
        lockon_mode,
        damage,
        library_number: lib_int("number") as u16,
        library_index: lib_int("index") as u8,
        sort_key: lib_int("sort") as u16,
        slot_in_limit: int("slot_in_limit", 0xFF)? as u8,
        dark_substitute: None,
        sp_damage: None,
        navi_damage: None,
        modifier,
        program_advances: Vec::new(),
        gun_del_sol: None,
        recovery: None,
        sword: None,
        script: None,
    })
}

/// `define.roles { actions = { ... }, kinds = { ... } }` (content::roles):
/// each role a definition, or a v1 registration through its `legacy`
/// marker (an action by number, a kind by key).
fn read_roles(d: &Definition, actions: &[ActionDef], kinds: &[KindDef], functions: &mut Functions) -> Result<Roles, ContentError> {
    let what = |e: String| ContentError::new(format!("{}.luau: roles: {e}", d.module));
    let Data::Map(groups) = &d.spec else { return Err(what("a table of role groups".into())) };
    let legacy = |name: &str, v: &Data, field: &str| -> Result<Option<Data>, ContentError> {
        let Data::Map(_) = v else { return Ok(None) };
        match v.field("legacy").field(field) {
            Data::Nil => Err(what(format!("{name} is a table, but not a legacy marker `{{ legacy = {{ {field} = ... }} }}`"))),
            x => Ok(Some(x.clone())),
        }
    };
    let mut roles = Roles::default();
    for (group, entries) in groups {
        let group = group.to_string();
        let Data::Map(entries) = entries else { return Err(what(format!("`{group}` is a table"))) };
        for (name, v) in entries {
            let name = name.to_string();
            match group.as_str() {
                "actions" => {
                    let names: Vec<&str> = ActionRole::ALL.iter().map(|r| r.name()).collect();
                    let role = ActionRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role actions.{name} (it has {})", names.join(", ")))
                    })?;
                    let full = format!("actions.{name}");
                    let target = if let Data::Ref(Registry::Action, key) = v {
                        RoleAction::Action(ActionHandle(
                            actions.iter().position(|a| &a.key == key).expect("a defined action") as u16,
                        ))
                    } else if let Some(n) = legacy(&full, v, "action")? {
                        match n {
                            // A v1 registration's action, by its number.
                            Data::Int(n @ 0..=0xFF) => match actions.iter().position(|a| a.number == Some(n as u8)) {
                                Some(i) => RoleAction::Action(ActionHandle(i as u16)),
                                None => RoleAction::Unported(n as u8),
                            },
                            // A definition by its key, for a pack whose
                            // modules can't require the one that defines it
                            // (the engine's test pack).
                            Data::Str(key) => match actions.iter().position(|a| a.key == key) {
                                Some(i) => RoleAction::Action(ActionHandle(i as u16)),
                                None => return Err(what(format!("{full}'s legacy action {key:?} is no action's key"))),
                            },
                            n => {
                                return Err(what(format!(
                                    "{full}'s legacy action is {n:?}, not an action number or an action's key"
                                )));
                            }
                        }
                    } else {
                        return Err(what(format!("{full} is not an action")));
                    };
                    roles.actions.insert(role, target);
                }
                "kinds" => {
                    let names: Vec<&str> = KindRole::ALL.iter().map(|r| r.name()).collect();
                    let role = KindRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role kinds.{name} (it has {})", names.join(", ")))
                    })?;
                    let full = format!("kinds.{name}");
                    let key = if let Data::Ref(Registry::Kind, key) = v {
                        key.clone()
                    } else if let Some(k) = legacy(&full, v, "kind")? {
                        let Data::Str(k) = k else {
                            return Err(what(format!("{full}'s legacy kind is {k:?}, not a kind's key")));
                        };
                        k
                    } else {
                        return Err(what(format!("{full} is not a kind")));
                    };
                    let target = match kinds.iter().position(|k| k.key == key) {
                        Some(i) => RoleKind::Kind(KindHandle(i as u16)),
                        None => RoleKind::Missing(key),
                    };
                    roles.kinds.insert(role, target);
                }
                "hooks" => {
                    let names: Vec<&str> = HookRole::ALL.iter().map(|r| r.name()).collect();
                    let role = HookRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role hooks.{name} (it has {})", names.join(", ")))
                    })?;
                    if !matches!(v, Data::Function) {
                        return Err(what(format!("hooks.{name} is not a function")));
                    }
                    roles.hooks.insert(role, functions.id(FnSource::slot(Registry::Roles, &d.key, &format!("hooks.{name}"))));
                }
                _ => return Err(what(format!("the ruleset has no role group `{group}` (it has actions, kinds, hooks)"))),
            }
        }
    }
    Ok(roles)
}

/// A weapon definition's `legacy = { routines = { ... } }` marker: the
/// routine numbers it takes (none without one).
fn weapon_routines(d: &Definition) -> Result<Vec<u8>, ContentError> {
    let what = |e: String| ContentError::new(format!("{}.luau: weapon {}: {e}", d.module, d.key));
    let legacy = d.spec.field("legacy");
    match legacy {
        Data::Nil => return Ok(Vec::new()),
        Data::Map(_) => {}
        other => return Err(what(format!("`legacy` is {other:?}, not a table"))),
    }
    match legacy.field("routines") {
        Data::List(items) if !items.is_empty() => items
            .iter()
            .map(|r| match r {
                Data::Int(i) if (0..0xFF).contains(i) => Ok(*i as u8),
                other => Err(what(format!("`legacy.routines` holds {other:?}, not a routine number below 0xFF"))),
            })
            .collect(),
        other => Err(what(format!("`legacy.routines` is {other:?}, not a list of routine numbers"))),
    }
}

/// A chip definition's `legacy` marker: the subtype and parameter bytes
/// (none: 0).
fn legacy_bytes(spec: &Data) -> Result<(u8, [u8; 4]), String> {
    let legacy = spec.field("legacy");
    match legacy {
        Data::Nil => return Ok((0, [0; 4])),
        Data::Map(_) => {}
        Data::List(l) if l.is_empty() => return Ok((0, [0; 4])),
        other => return Err(format!("`legacy` is {other:?}, not a table")),
    }
    for field in ["action", "script"] {
        if !legacy.field(field).is_nil() {
            return Err(format!("`legacy.{field}` (a behaviour still a v1 module) comes with step 5"));
        }
    }
    let byte = |d: &Data, what: &str| match d {
        Data::Int(i) if (0..=0xFF).contains(i) => Ok(*i as u8),
        other => Err(format!("`legacy.{what}` holds {other:?}, not a byte")),
    };
    let subtype = match legacy.field("subtype") {
        Data::Nil => 0,
        d => byte(d, "subtype")?,
    };
    let mut params = [0; 4];
    match legacy.field("params") {
        Data::Nil => {}
        Data::List(items) if items.len() <= 4 => {
            for (p, item) in params.iter_mut().zip(items) {
                *p = byte(item, "params")?;
            }
        }
        other => return Err(format!("`legacy.params` is {other:?}, not up to four bytes")),
    }
    Ok((subtype, params))
}

impl Defs {
    /// What `content` (its data's registrations and the engine's own) and
    /// `definitions` (what its modules define) make.
    pub fn build(content: &Content, definitions: Definitions) -> Result<Defs, ContentError> {
        let mut functions = Functions::default();

        // Layouts: the definitions' and modules' state tables, and the
        // empty one.
        let mut schemas = vec![SchemaDef { key: NO_STATE.to_string(), schema: Schema::default() }];
        for d in definitions.of(Registry::Schema) {
            let schema = Schema::from_data(&d.spec)
                .map_err(|e| ContentError::new(format!("{}.luau: state {}: {e}", d.module, d.key)))?;
            schemas.push(SchemaDef { key: d.key.clone(), schema });
        }
        schemas.sort_by(|a, b| a.key.cmp(&b.key));
        let schema_id = |key: &str| -> StateId {
            StateId(schemas.binary_search_by(|s| s.key.as_str().cmp(key)).expect("a defined schema") as u16)
        };
        let state_of = |d: &Definition| -> Result<StateId, ContentError> {
            match d.spec.field("state") {
                Data::Nil => Ok(schema_id(NO_STATE)),
                Data::Ref(Registry::Schema, key) => Ok(schema_id(key)),
                _ => Err(ContentError::new(format!("{}.luau: {} {}'s `state` is not a table", d.module, d.registry, d.key))),
            }
        };
        let module_state = |module: &str| -> StateId {
            definitions.module(module).and_then(|m| m.state.as_deref()).map_or(schema_id(NO_STATE), &schema_id)
        };

        // Object kinds.
        let mut kinds = Entries::new(Registry::Kind);
        for (kind, key, pool) in ENGINE_KINDS {
            let def = KindDef {
                key: key.to_string(),
                pool,
                implementation: KindImpl::Engine(kind),
                schema: schema_id(NO_STATE),
                slot: None,
                place: None,
            };
            kinds.add(key.to_string(), def, "the engine's".into());
        }
        // Registration by number of the generic chip actions' parts, by
        // subtype: what each pack record resolves its usage from.
        let mut subtype_hooks: BTreeMap<(u8, u8), (FnSource, String)> = BTreeMap::new();
        let mut add_hook = |action: u8, subtype: u8, f: FnSource, whose: String| -> Result<(), ContentError> {
            match subtype_hooks.get(&(action, subtype)) {
                Some((g, first)) if *g != f => Err(ContentError::new(format!(
                    "{whose} implements action {action:#x}'s subtype {subtype} with {f}, but {first} with {g}"
                ))),
                Some(_) => Ok(()),
                None => {
                    subtype_hooks.insert((action, subtype), (f, whose));
                    Ok(())
                }
            }
        };
        for k in &content.objects.kinds {
            let whose = format!("objects/{} (object.toml)", k.name);
            let update = export(&definitions, &k.script, "update", &whose)?;
            // (The entry type `actor_list_entry` names is the stage data's;
            // the ruleset reaches the kind by its role.)
            let place = match k.actor_list_entry {
                Some(_) => Some(functions.id(export(&definitions, &k.script, "actor_list_entry", &whose)?)),
                None => None,
            };
            let def = KindDef {
                key: k.name.clone(),
                pool: k.pool,
                implementation: KindImpl::Script { update: functions.id(update) },
                schema: module_state(&k.script),
                slot: Some((k.pool, k.index)),
                place,
            };
            kinds.add(k.name.clone(), def, whose.clone());
        }
        for d in definitions.of(Registry::Kind) {
            let pool = d
                .spec
                .field("pool")
                .str()
                .and_then(Pool::from_name)
                .ok_or_else(|| ContentError::new(format!("{}.luau: kind {} needs a `pool` (actor, attack or effect)", d.module, d.key)))?;
            let place = match d.spec.field("place") {
                Data::Nil => None,
                _ => Some(functions.id(slot(d, "place")?)),
            };
            let def = KindDef {
                key: d.key.clone(),
                pool,
                implementation: KindImpl::Script { update: functions.id(slot(d, "update")?) },
                schema: state_of(d)?,
                slot: None,
                place,
            };
            kinds.add(d.key.clone(), def, format!("defined in {}.luau", d.module));
        }
        let kinds: Vec<KindDef> = kinds.sorted()?.into_iter().map(|(_, k)| k).collect();

        // Actions registered by number: chips' and weapons'.
        let mut numbered: BTreeMap<u8, (String, String)> = BTreeMap::new();
        let mut add_numbered = |action: u8, module: &str, whose: String| -> Result<(), ContentError> {
            if action < 0x10 {
                return Err(ContentError::new(format!("{whose}: actions below 0x10 are the engine's")));
            }
            match numbered.get(&action) {
                Some((m, first)) if m != module => Err(ContentError::new(format!(
                    "{whose} implements action {action:#x} with {module}.luau, but {first} with {m}.luau"
                ))),
                Some(_) => Ok(()),
                None => {
                    numbered.insert(action, (module.to_string(), whose));
                    Ok(())
                }
            }
        };
        for c in &content.chips {
            let Some(module) = &c.script else { continue };
            let whose = format!("chip {:#05x} ({})", c.id.unwrap_or_default(), c.name);
            if record_action(&definitions, module, &whose)?.is_some() {
                // The chip runs the action its module exports, below.
                continue;
            }
            match c.action {
                DIMMING_CHIP_ACTION => {
                    let f = export(&definitions, module, "dimming_chip", &whose)?;
                    add_hook(c.action, c.subtype, f, whose)?;
                }
                NAVI_CHIP_ACTION => {
                    let f = export(&definitions, module, "navi_chip", &whose)?;
                    add_hook(c.action, c.subtype, f, whose)?;
                }
                INSTANT_CHIP_ACTION => {
                    let f = export(&definitions, module, "instant_chip", &whose)?;
                    add_hook(c.action, c.subtype, f, whose)?;
                }
                action => add_numbered(action, module, whose)?,
            }
        }
        for w in &content.weapons {
            let whose = format!("weapon routine {:#04x} ({})", w.id, w.name);
            if let Some(action) = w.action {
                add_numbered(action, &w.script, whose)?;
            }
        }
        let mut subtype_table = |action: u8| -> BTreeMap<u8, FnId> {
            subtype_hooks.iter().filter(|((a, _), _)| *a == action).map(|(&(_, st), (f, _))| (st, functions.id(f.clone()))).collect()
        };
        let (dimming_hooks, navi_hooks, instant_hooks) =
            (subtype_table(DIMMING_CHIP_ACTION), subtype_table(NAVI_CHIP_ACTION), subtype_table(INSTANT_CHIP_ACTION));
        let mut actions = Entries::new(Registry::Action);
        for (&number, (module, whose)) in &numbered {
            let update = export(&definitions, module, "update", whose)?;
            let def = ActionDef {
                key: format!("v1/action-{number:02x}"),
                update: functions.id(update),
                schema: module_state(module),
                number: Some(number),
            };
            actions.add(def.key.clone(), def, whose.clone());
        }
        for d in definitions.of(Registry::Action) {
            let def =
                ActionDef { key: d.key.clone(), update: functions.id(slot(d, "update")?), schema: state_of(d)?, number: None };
            actions.add(d.key.clone(), def, format!("defined in {}.luau", d.module));
        }
        let actions: Vec<ActionDef> = actions.sorted()?.into_iter().map(|(_, a)| a).collect();

        // Weapons.
        let mut weapons = Entries::new(Registry::Weapon);
        for w in &content.weapons {
            let whose = format!("weapon routine {:#04x} ({})", w.id, w.name);
            let setup = export(&definitions, &w.script, "setup", &whose)?;
            let instant = match w.instant_chip {
                Some(_) => Some(functions.id(export(&definitions, &w.script, "instant_chip", &whose)?)),
                None => None,
            };
            let def = WeaponDef {
                key: format!("v1/weapon-{:02x}", w.id),
                name: w.name.clone(),
                setup: Some(functions.id(setup)),
                routines: vec![w.id],
                number: Some(w.id),
                charge_ticks: Vec::new(),
                instant,
            };
            weapons.add(def.key.clone(), def, whose);
        }
        // The routines the definitions take (their `legacy` markers).
        let mut defined_routines = Vec::new();
        for d in definitions.of(Registry::Weapon) {
            defined_routines.extend(weapon_routines(d)?);
        }
        // Every other routine number a navi's stats may name: a weapon
        // nothing implements yet (using it is the ruleset's error).
        for n in 0..=0xFEu8 {
            if !content.weapons.iter().any(|w| w.id == n) && !defined_routines.contains(&n) {
                let def = WeaponDef {
                    key: format!("v1/weapon-{n:02x}"),
                    name: String::new(),
                    setup: None,
                    routines: vec![n],
                    number: Some(n),
                    charge_ticks: Vec::new(),
                    instant: None,
                };
                weapons.add(def.key.clone(), def, "a weapon routine number".into());
            }
        }
        for d in definitions.of(Registry::Weapon) {
            let name = d.spec.field("name").str().unwrap_or(&d.key).to_string();
            let charge_ticks = match d.spec.field("charge_ticks") {
                Data::Nil => Vec::new(),
                Data::List(items) => items
                    .iter()
                    .map(|t| match t {
                        Data::Int(i) if (0..=0xFFFF).contains(i) => Ok(*i as u16),
                        other => Err(ContentError::new(format!(
                            "{}.luau: weapon {}'s `charge_ticks` holds {other:?}, not a tick count",
                            d.module, d.key
                        ))),
                    })
                    .collect::<Result<_, _>>()?,
                other => {
                    return Err(ContentError::new(format!(
                        "{}.luau: weapon {}'s `charge_ticks` is {other:?}, not a list",
                        d.module, d.key
                    )));
                }
            };
            let setup = Some(functions.id(slot(d, "setup")?));
            let routines = weapon_routines(d)?;
            let number = routines.first().copied();
            // Its own instant effect (`instant = function(user, spec)`),
            // which the instant chips' action runs.
            let instant = match d.spec.field("instant") {
                Data::Nil => None,
                _ => Some(functions.id(slot(d, "instant")?)),
            };
            let def = WeaponDef { key: d.key.clone(), name, setup, routines, number, charge_ticks, instant };
            weapons.add(d.key.clone(), def, format!("defined in {}.luau", d.module));
        }
        let weapons: Vec<WeaponDef> = weapons.sorted()?.into_iter().map(|(_, w)| w).collect();

        // Chips content defines.
        let action_handle = |key: &str| -> Option<ActionHandle> {
            actions.binary_search_by(|a| a.key.as_str().cmp(key)).ok().map(|i| ActionHandle(i as u16))
        };
        let mut chips = Entries::new(Registry::Chip);
        for c in &content.chips {
            let key = format!("v1/chip-{:03x}", c.id.unwrap_or_default());
            // Registration by number: the record's action, or for the
            // ruleset's generic chip actions its subtype's registration. A
            // record whose module exports an action runs that instead, as
            // a chip definition's `action` does.
            let by_subtype = |table: &BTreeMap<u8, FnId>, f: fn(FnId) -> ChipUsage, unported: fn(u8) -> Unported| {
                table.get(&c.subtype).map_or(ChipUsage::Unported(unported(c.subtype)), |&h| f(h))
            };
            let exported = match &c.script {
                Some(module) => record_action(&definitions, module, &key)?,
                None => None,
            };
            let usage = match (exported, c.action) {
                (Some(a), _) => ChipUsage::Action(action_handle(a).expect("a defined action")),
                (None, DIMMING_CHIP_ACTION) => by_subtype(&dimming_hooks, ChipUsage::Dimming, Unported::Dimming),
                (None, NAVI_CHIP_ACTION) => by_subtype(&navi_hooks, ChipUsage::Navi, Unported::Navi),
                (None, INSTANT_CHIP_ACTION) => by_subtype(&instant_hooks, ChipUsage::Instant, Unported::Instant),
                (None, n) => match actions.iter().position(|a| a.number == Some(n)) {
                    Some(i) => ChipUsage::Action(ActionHandle(i as u16)),
                    None => ChipUsage::Unported(Unported::Action(n)),
                },
            };
            chips.add(key.clone(), ChipDef { key, record: c.clone(), usage }, "the pack's chip record".into());
        }
        for d in definitions.of(Registry::Chip) {
            let mut usages = Vec::new();
            match d.spec.field("action") {
                Data::Nil => {}
                Data::Ref(Registry::Action, key) => usages.push(ChipUsage::Action(action_handle(key).expect("a defined action"))),
                _ => return Err(ContentError::new(format!("{}.luau: chip {}'s `action` is not an action", d.module, d.key))),
            }
            for (field, usage) in [
                ("dimming", ChipUsage::Dimming as fn(FnId) -> ChipUsage),
                ("navi", ChipUsage::Navi),
                ("instant", ChipUsage::Instant),
            ] {
                if !d.spec.field(field).is_nil() {
                    usages.push(usage(functions.id(slot(d, field)?)));
                }
            }
            let [usage] = usages[..] else {
                return Err(ContentError::new(format!(
                    "{}.luau: chip {} needs exactly one of `action`, `dimming`, `navi` and `instant`",
                    d.module, d.key
                )));
            };
            let record = chip_record(d)?;
            chips.add(d.key.clone(), ChipDef { key: d.key.clone(), record, usage }, format!("defined in {}.luau", d.module));
        }
        let chips: Vec<ChipDef> = chips.sorted()?.into_iter().map(|(_, c)| c).collect();

        // The pack's navis, forms and stages.
        let mut navis = Entries::new(Registry::Navi);
        for n in &content.navis {
            let key = format!("v1/navi-{:02x}", n.id);
            navis.add(key.clone(), NaviDef { key, record: n.clone() }, "the pack's navi".into());
        }
        let navis: Vec<NaviDef> = navis.sorted()?.into_iter().map(|(_, n)| n).collect();
        let mut forms = Entries::new(Registry::Form);
        for f in &content.forms {
            let key = format!("v1/form-{:02x}", f.id);
            forms.add(key.clone(), FormDef { key, record: f.clone() }, "the pack's form".into());
        }
        let forms: Vec<FormDef> = forms.sorted()?.into_iter().map(|(_, f)| f).collect();
        let mut stages = Entries::new(Registry::Stage);
        for (i, st) in content.rules.stages.settings.iter().enumerate() {
            let key = format!("v1/stage-{i:02x}");
            stages.add(key.clone(), StageDef { key, record: *st, number: i as u8 }, "the pack's battle settings".into());
        }
        let stages: Vec<StageDef> = stages.sorted()?.into_iter().map(|(_, s)| s).collect();

        // Effects, sparks, regions and collision types: each gets the
        // engine's number after the pack data's.
        let look = |d: &Definition| -> Result<super::EffectSprite, ContentError> {
            let what = |e: String| ContentError::new(format!("{}.luau: {} {}: {e}", d.module, d.registry, d.key));
            let sprite = match d.spec.field("sprite") {
                Data::Asset(bn6_content_api::AssetKind::Sprite, name) => content.assets.sprites[name],
                _ => return Err(what("needs a `sprite` (asset.sprite(...))".into())),
            };
            Ok(super::EffectSprite { sprite, anim: byte(d, "anim")?, palette: byte(d, "palette")? })
        };
        let effects = definitions.of(Registry::Effect).iter().map(look).collect::<Result<Vec<_>, _>>()?;
        let sparks = definitions.of(Registry::Spark).iter().map(look).collect::<Result<Vec<_>, _>>()?;
        let base = |len: usize, n: usize, what: &str| -> Result<u8, ContentError> {
            u8::try_from(len)
                .ok()
                .filter(|&b| b as usize + n <= 0x100)
                .ok_or_else(|| ContentError::new(format!("too many {what}: {len} in the pack's data and {n} defined")))
        };
        let effect_base = base(content.effects.len(), effects.len(), "effects")?;
        let spark_base = base(content.sparks.len(), sparks.len(), "hit sparks")?;
        let (mut shapes, mut fields) = (content.regions.len(), content.rules.field_regions.len());
        let mut regions = Vec::new();
        for d in definitions.of(Registry::Region) {
            let what = |e: &str| ContentError::new(format!("{}.luau: region {}: {e}", d.module, d.key));
            let (region, number) = match (d.spec.field("panels"), d.spec.field("field")) {
                (Data::List(items), Data::Nil) => {
                    let mut panels = Vec::new();
                    for p in items {
                        let (Some(dx), Some(dy)) = (p.item(1).int(), p.item(2).int()) else {
                            return Err(what("each of `panels` is { dx, dy }"));
                        };
                        panels.push(super::PanelOffset { dx: dx as i8, dy: dy as i8 });
                    }
                    shapes += 1;
                    (super::Region::Panels(panels), shapes - 1)
                }
                (Data::Nil, Data::Map(_)) => {
                    let word = |k: &str| d.spec.field("field").field(k).int().unwrap_or(0) as u32;
                    fields += 1;
                    (super::Region::Field(super::PanelCondition { require: word("require"), forbid: word("forbid") }), 0x80 + fields - 1)
                }
                _ => return Err(what("needs exactly one of `panels` and `field`")),
            };
            if shapes > 0x80 || fields > 0x80 {
                return Err(what("too many regions for the engine's region byte"));
            }
            regions.push((region, number as u8));
        }
        let mut collisions = Vec::new();
        for d in definitions.of(Registry::Collision) {
            let word = |k: &str| -> Result<u32, ContentError> {
                d.spec.field(k).int().map(|i| i as u32).ok_or_else(|| {
                    ContentError::new(format!("{}.luau: collision {} needs `{k}` (a flag word)", d.module, d.key))
                })
            };
            let row_offset = d.spec.field("row_offset").int().unwrap_or(0) as u16;
            collisions.push(CollisionTypeDef { flags: [word("side0")?, word("side1")?], row_offset });
        }
        let collision_base = base(content.rules.collision_types.len(), collisions.len(), "collision types")?;

        // The roles.
        let mut roles = Roles::default();
        if let [d] = definitions.of(Registry::Roles) {
            roles = read_roles(d, &actions, &kinds, &mut functions)?;
        }

        let records: Vec<RecordDef> = definitions
            .of(Registry::Record)
            .iter()
            .map(|d| RecordDef { key: d.key.clone(), record_type: d.record_type.clone().unwrap_or_default() })
            .collect();

        // Each definition's handle.
        fn position<'a>(mut keys: impl Iterator<Item = &'a String>, key: &str) -> u16 {
            keys.position(|k| k == key).expect("an entry") as u16
        }
        let mut handles = Vec::with_capacity(definitions.defs.len());
        let mut in_registry = 0u16;
        let mut last = None;
        for d in &definitions.defs {
            if last != Some(d.registry) {
                (last, in_registry) = (Some(d.registry), 0);
            }
            handles.push(match d.registry {
                Registry::Kind => position(kinds.iter().map(|k| &k.key), &d.key),
                Registry::Action => position(actions.iter().map(|a| &a.key), &d.key),
                Registry::Weapon => position(weapons.iter().map(|w| &w.key), &d.key),
                Registry::Chip => position(chips.iter().map(|c| &c.key), &d.key),
                Registry::Schema => schema_id(&d.key).0,
                _ => in_registry,
            });
            in_registry += 1;
        }

        // Registration by number's lookups.
        let kind_keys: BTreeMap<String, KindHandle> =
            kinds.iter().enumerate().map(|(i, k)| (k.key.clone(), KindHandle(i as u16))).collect();
        let engine = ENGINE_KINDS.iter().map(|e| kind_keys[e.1]).collect();
        let mut defs = Defs {
            defined: true,
            definitions,
            handles,
            kind_keys,
            engine,
            kind_slots: vec![None; 3 * 256],
            action_numbers: vec![None; 256],
            weapon_ids: vec![None; 256],
            chip_numbers: BTreeMap::new(),
            navi_numbers: Vec::new(),
            form_numbers: Vec::new(),
            stage_numbers: Vec::new(),
            chip_keys: BTreeMap::new(),
            weapon_keys: BTreeMap::new(),
            kinds: Vec::new(),
            actions,
            weapons,
            chips,
            navis,
            forms,
            stages,
            records,
            effects,
            sparks,
            regions,
            collisions,
            roles,
            effect_base,
            spark_base,
            collision_base,
            schemas,
            functions: Vec::new(),
        };
        for (i, k) in kinds.iter().enumerate() {
            if let Some((pool, index)) = k.slot {
                let at = pool_index(pool) * 256 + index as usize;
                if let Some(other) = defs.kind_slots[at] {
                    return Err(ContentError::new(format!(
                        "kinds {} and {} both fill {} object {index:#x}",
                        kinds[other.index()].key,
                        k.key,
                        pool.name()
                    )));
                }
                defs.kind_slots[at] = Some(KindHandle(i as u16));
            }
        }
        defs.kinds = kinds;
        for (i, a) in defs.actions.iter().enumerate() {
            if let Some(n) = a.number {
                defs.action_numbers[n as usize].get_or_insert(ActionHandle(i as u16));
            }
        }
        for (i, w) in defs.weapons.iter().enumerate() {
            defs.weapon_keys.insert(w.key.clone(), WeaponHandle(i as u16));
            for &id in &w.routines {
                if let Some(other) = defs.weapon_ids[id as usize] {
                    return Err(ContentError::new(format!(
                        "weapons {} and {} both are routine {id:#04x}",
                        defs.weapons[other.index()].key,
                        w.key
                    )));
                }
                defs.weapon_ids[id as usize] = Some(WeaponHandle(i as u16));
            }
        }
        for (i, c) in defs.chips.iter().enumerate() {
            defs.chip_keys.insert(c.key.clone(), ChipHandle(i as u16));
            if let Some(id) = c.record.id {
                defs.chip_numbers.insert(id, ChipHandle(i as u16));
            }
        }
        defs.navi_numbers = vec![None; 256];
        for (i, n) in defs.navis.iter().enumerate() {
            defs.navi_numbers[n.record.id as usize] = Some(NaviHandle(i as u16));
        }
        defs.form_numbers = vec![None; 256];
        for (i, f) in defs.forms.iter().enumerate() {
            defs.form_numbers[f.record.id as usize] = Some(FormHandle(i as u16));
        }
        defs.stage_numbers = vec![None; 256];
        for (i, st) in defs.stages.iter().enumerate() {
            defs.stage_numbers[st.number as usize] = Some(StageHandle(i as u16));
        }
        defs.functions = functions.list;
        Ok(defs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every module of the BN6 content (content/bn6) loads in the define
    /// phase: it passes the bytecode check, runs, and what it defines reads
    /// back as data.
    #[test]
    fn every_bn6_module_loads_in_the_define_phase() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");
        let mut c = Content::default();
        c.scripts.modules = crate::content::testing::modules_under(dir);
        c.assets = crate::content::testing::asset_names_used(&c.scripts.modules);
        assert!(c.scripts.modules.len() > 200, "{} modules", c.scripts.modules.len());
        c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
        // What they define fills the registries (the v1 modules' `state`
        // tables dwindle as they convert, so they aren't counted).
        let (kinds, actions, weapons) = (c.defs.kinds.len(), c.defs.actions.len(), c.defs.weapons.len());
        assert!(kinds > 150 && actions > 100 && weapons > 10, "{kinds} kinds, {actions} actions, {weapons} weapons");
    }
}
