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
    ActionHandle, ChipHandle, ContentError, Data, Definition, Definitions, FnId, FnSource, FormHandle, Hook, KindHandle,
    NaviHandle, Pool, RecordHandle, Registry, Schema, StageHandle, StateId, WeaponHandle,
};

use super::{
    ChipData, ChipId, Content, DIMMING_CHIP_ACTION,
    FormData, INSTANT_CHIP_ACTION, NAVI_CHIP_ACTION, NaviData,
};
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
    /// by: the engine's own kinds' and the pack's `object.toml`s'. A kind
    /// content defines has none.
    pub slot: Option<(Pool, u8)>,
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
    /// The weapon routine numbers that name it: the routines a weapon
    /// content defines takes with its transitional `legacy = { routines }`
    /// marker (the pack's forms and the ruleset still name weapons by
    /// number; a weapon's alias routines with the same charge times are one
    /// weapon), the test content's `WeaponData`, or the number of a routine
    /// nothing implements. Registration by number finds it by any of them.
    pub routines: Vec<u8>,
    /// Its first routine number: what the ruleset's numeric logic asks
    /// until phase C (none for a weapon content defines without routines).
    pub number: Option<u8>,
    /// Ticks to a full charge by Charge stat, for a weapon content defines
    /// (the pack's routines' are the charge table's, by number).
    pub charge_ticks: Vec<u16>,
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
}

/// A chip: the pack's record (registration by number, keyed `v1/chip-036`)
/// or what content defines.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChipDef {
    pub key: String,
    /// Its record. A definition's has no `id`: the engine never learns the
    /// original's number for what content defines.
    pub record: ChipData,
    /// How its definition uses it; none for the pack's records, which are
    /// used by their action number and subtype.
    pub usage: Option<ChipUsage>,
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
    /// Its place in the pack's settings table (the test content's); none
    /// for a stage content defines.
    pub number: Option<u8>,
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

/// What the ruleset needs from content by role (docs/design/
/// content-model-v2.md §7.4): `define.roles { ... }`, once. A role content
/// hasn't filled yet is `None` until the BN6 content fills every one (then
/// an unfilled role is a load error).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Roles {
    pub actions: RoleActions,
}

/// The actions the ruleset starts by role.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RoleActions {
    /// AntiDmg's counter (`sub_801056A`, `sub_80105F2`).
    pub anti_damage_counter: Option<ActionHandle>,
    /// AntiSwrd's counter.
    pub anti_sword_counter: Option<ActionHandle>,
    /// BodyGrd's counter.
    pub body_guard_counter: Option<ActionHandle>,
    /// The charged shot a navi's request starts from idle without its
    /// weapon's setup (`sub_8010312`'s request 0x20; the original's action
    /// 0x16, MegaMan's charged shot).
    pub forced_charged_shot: Option<ActionHandle>,
}

impl RoleActions {
    /// The roles by name, as `rules/roles.luau` names them.
    const NAMES: [&str; 4] = ["anti_damage_counter", "anti_sword_counter", "body_guard_counter", "forced_charged_shot"];

    fn slot(&mut self, name: &str) -> Option<&mut Option<ActionHandle>> {
        match name {
            "anti_damage_counter" => Some(&mut self.anti_damage_counter),
            "anti_sword_counter" => Some(&mut self.anti_sword_counter),
            "body_guard_counter" => Some(&mut self.body_guard_counter),
            "forced_charged_shot" => Some(&mut self.forced_charged_shot),
            _ => None,
        }
    }
}

impl Roles {
    /// An action role, or a panic naming it when content hasn't filled it.
    pub fn action(role: Option<ActionHandle>, name: &str) -> ActionHandle {
        role.unwrap_or_else(|| panic!("the role actions.{name} is not filled (define.roles in rules/roles.luau)"))
    }
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
    /// Registration by number: hooks by table and number (the bridge).
    pub hooks: BTreeMap<Hook, FnId>,
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

    /// The function a hook by number runs: a weapon routine's `setup`, or
    /// what the pack's data registers for a subtype or an entry type.
    pub fn hook(&self, hook: Hook) -> Option<FnId> {
        match hook {
            Hook::Weapon(n) => self.weapon_numbered(n).and_then(|w| self.weapons[w.index()].setup),
            _ => self.hooks.get(&hook).copied(),
        }
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

/// A chip definition's record (docs/design/content-model-v2.md §3.1): the
/// fields the engine reads, with the lock-on mode and the Program Advance
/// recipes' chips by the numbers `r` gives them. Damage formulas and dark
/// chips' substitutes by definition come with the v2 API (a chip still read
/// by number gives them in its legacy marker); a definition that gives one
/// is refused.
///
/// The transitional `legacy = { subtype, params }` marker gives the record
/// the original's subtype and parameter bytes, for what still reads them
/// of a chip besides its own action (SlashCross's charged slash reads a
/// sword's); a chip whose behaviour is still a v1 module gives its number,
/// action and module there too (`content::legacy` reads those).
pub(crate) fn chip_record(d: &Definition, r: &super::legacy::Resolver) -> Result<ChipData, ContentError> {
    use serde_json::{Map, Value as Json};
    let what = |e: String| ContentError::new(format!("{}.luau: chip {}: {e}", d.module, d.key));
    let spec = &d.spec;
    if !spec.field("dark_substitute").is_nil() {
        return Err(what("`dark_substitute` in a definition comes with the v2 API".into()));
    }
    let json = |field: &str| -> Result<Json, ContentError> { r.json(spec.field(field), &format!("chip {}.{field}", d.key)).map_err(what) };
    let mut o = Map::new();
    o.insert("name".into(), Json::String(spec.field("name").str().unwrap_or(&d.key).to_string()));
    let defaults: [(&str, Json); 13] = [
        ("codes", Json::Array(Vec::new())),
        ("element", "null".into()),
        ("family", "null".into()),
        ("class", "standard".into()),
        ("rarity", 0.into()),
        ("mb", 0.into()),
        ("flags", Json::Array(Vec::new())),
        ("extra_flags", Json::Array(Vec::new())),
        ("hit_param", 0.into()),
        ("lockout", 0.into()),
        ("damage", 0.into()),
        ("slot_in_limit", 0.into()),
        ("modifier", Json::Null),
    ];
    for (field, default) in defaults {
        let v = json(field)?;
        o.insert(field.into(), if v.is_null() { default } else { v });
    }
    if o["damage"].as_i64().is_some_and(|v| v >= 1000) {
        return Err(what("`damage` is a number below 1000 (formulas come with the v2 API)".into()));
    }
    let library = spec.field("library");
    for (field, from) in [("library_number", "number"), ("library_index", "index"), ("sort_key", "sort")] {
        o.insert(field.into(), library.field(from).int().unwrap_or(0).into());
    }
    let beast = spec.field("beast");
    o.insert("beast_lockon".into(), Json::Bool(!beast.is_nil() && !matches!(beast.field("rush"), Data::Bool(false))));
    let mode = match beast.field("lockon") {
        Data::Nil => 0,
        v => r.json(v, &format!("chip {}.beast.lockon", d.key)).map_err(what)?.as_i64().unwrap_or(0),
    };
    o.insert("lockon_mode".into(), mode.into());
    o.insert("program_advance".into(), json("program_advances")?);
    if o["program_advance"].is_null() {
        o.insert("program_advance".into(), Json::Array(Vec::new()));
    }
    let (subtype, params) = legacy_bytes(spec).map_err(what)?;
    o.insert("action".into(), 0.into());
    o.insert("subtype".into(), subtype.into());
    o.insert("params".into(), Json::Array(params.iter().map(|&p| p.into()).collect()));
    serde_json::from_value(Json::Object(o)).map_err(|e| what(e.to_string()))
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
    pub fn build(content: &Content, definitions: Definitions, legacy: &super::legacy::Legacy) -> Result<Defs, ContentError> {
        let resolver = super::legacy::Resolver::new(&content.assets, &definitions);
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
        for (kind, key, pool, index) in ENGINE_KINDS {
            let def = KindDef {
                key: key.to_string(),
                pool,
                implementation: KindImpl::Engine(kind),
                schema: schema_id(NO_STATE),
                slot: Some((pool, index)),
            };
            kinds.add(key.to_string(), def, "the engine's".into());
        }
        let mut hooks: BTreeMap<Hook, (FnSource, String)> = BTreeMap::new();
        let mut add_hook = |hook: Hook, f: FnSource, whose: String| -> Result<(), ContentError> {
            match hooks.get(&hook) {
                Some((g, first)) if *g != f => {
                    Err(ContentError::new(format!("{whose} implements {hook} with {f}, but {first} with {g}")))
                }
                Some(_) => Ok(()),
                None => {
                    hooks.insert(hook, (f, whose));
                    Ok(())
                }
            }
        };
        for k in &content.objects.kinds {
            let whose = format!("objects/{} (object.toml)", k.name);
            let update = export(&definitions, &k.script, "update", &whose)?;
            let def = KindDef {
                key: k.name.clone(),
                pool: k.pool,
                implementation: KindImpl::Script { update: functions.id(update) },
                schema: module_state(&k.script),
                slot: Some((k.pool, k.index)),
            };
            kinds.add(k.name.clone(), def, whose.clone());
            if let Some(entry) = k.actor_list_entry {
                add_hook(Hook::ActorListEntry(entry), export(&definitions, &k.script, "actor_list_entry", &whose)?, whose)?;
            }
        }
        for d in definitions.of(Registry::Kind) {
            let pool = d
                .spec
                .field("pool")
                .str()
                .and_then(Pool::from_name)
                .ok_or_else(|| ContentError::new(format!("{}.luau: kind {} needs a `pool` (actor, attack or effect)", d.module, d.key)))?;
            let def = KindDef {
                key: d.key.clone(),
                pool,
                implementation: KindImpl::Script { update: functions.id(slot(d, "update")?) },
                schema: state_of(d)?,
                slot: None,
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
            match c.action {
                DIMMING_CHIP_ACTION => {
                    let f = export(&definitions, module, "dimming_chip", &whose)?;
                    add_hook(Hook::DimmingChip(c.subtype), f, whose)?;
                }
                NAVI_CHIP_ACTION => {
                    let f = export(&definitions, module, "navi_chip", &whose)?;
                    add_hook(Hook::NaviChip(c.subtype), f, whose)?;
                }
                INSTANT_CHIP_ACTION => {
                    let f = export(&definitions, module, "instant_chip", &whose)?;
                    add_hook(Hook::InstantChip(c.subtype), f, whose)?;
                }
                action => add_numbered(action, module, whose)?,
            }
        }
        for w in &content.weapons {
            let whose = format!("weapon routine {:#04x} ({})", w.id, w.name);
            if let Some(subtype) = w.instant_chip {
                let f = export(&definitions, &w.script, "instant_chip", &whose)?;
                add_hook(Hook::InstantChip(subtype), f, whose.clone())?;
            }
            if let Some(action) = w.action {
                add_numbered(action, &w.script, whose)?;
            }
        }
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

        // Weapons: those a definition gives by routine number (its legacy
        // setup marker: every routine number it is, the v1 module that
        // implements it), the test content's by routine, the other routine
        // numbers (a weapon nothing implements yet, with the charge times
        // the content gives them), and the ones content defines.
        let mut weapons = Entries::new(Registry::Weapon);
        let mut claimed: BTreeMap<u8, &str> = BTreeMap::new();
        for (key, w) in &legacy.weapons {
            let whose = format!("weapon {key}");
            for &n in &w.routines {
                if let Some(first) = claimed.insert(n, key) {
                    return Err(ContentError::new(format!("weapons {first} and {key} are both routine {n:#04x}")));
                }
            }
            let setup = match &w.script {
                Some(script) => Some(functions.id(export(&definitions, script, "setup", &whose)?)),
                None => None,
            };
            let def = WeaponDef {
                key: key.clone(),
                name: w.name.clone(),
                setup,
                number: w.routines.first().copied(),
                routines: w.routines.clone(),
                charge_ticks: w.charge_ticks.clone(),
            };
            weapons.add(key.clone(), def, whose);
        }
        for w in content.weapons.iter().filter(|w| !claimed.contains_key(&w.id)) {
            let whose = format!("weapon routine {:#04x} ({})", w.id, w.name);
            let setup = export(&definitions, &w.script, "setup", &whose)?;
            let def = WeaponDef {
                key: format!("v1/weapon-{:02x}", w.id),
                name: w.name.clone(),
                setup: Some(functions.id(setup)),
                routines: vec![w.id],
                number: Some(w.id),
                charge_ticks: Vec::new(),
            };
            weapons.add(def.key.clone(), def, whose);
        }
        // The routines the other definitions take (their `legacy` markers).
        let mut defined_routines = Vec::new();
        for d in definitions.of(Registry::Weapon) {
            if !legacy.weapons.contains_key(&d.key) {
                defined_routines.extend(weapon_routines(d)?);
            }
        }
        // Every other routine number a navi's stats may name: a weapon
        // nothing implements yet (using it is the ruleset's error), with the
        // charge times the content gives its routine.
        for n in 0..=0xFEu8 {
            if !claimed.contains_key(&n) && !content.weapons.iter().any(|w| w.id == n) && !defined_routines.contains(&n) {
                let def = WeaponDef {
                    key: format!("v1/weapon-{n:02x}"),
                    name: String::new(),
                    setup: None,
                    routines: vec![n],
                    number: Some(n),
                    charge_ticks: legacy.routine_charges.get(&n).cloned().unwrap_or_default(),
                };
                weapons.add(def.key.clone(), def, "a weapon routine number".into());
            }
        }
        for d in definitions.of(Registry::Weapon) {
            if legacy.weapons.contains_key(&d.key) {
                continue;
            }
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
            let def = WeaponDef { key: d.key.clone(), name, setup, routines, number, charge_ticks };
            weapons.add(d.key.clone(), def, format!("defined in {}.luau", d.module));
        }
        let weapons: Vec<WeaponDef> = weapons.sorted()?.into_iter().map(|(_, w)| w).collect();

        // Chips content defines.
        let action_handle = |key: &str| -> Option<ActionHandle> {
            actions.binary_search_by(|a| a.key.as_str().cmp(key)).ok().map(|i| ActionHandle(i as u16))
        };
        let mut chips = Entries::new(Registry::Chip);
        // The pack's chips: those a definition gives by number (its legacy
        // action marker) under its key, the others' records (the test
        // content's) under a transitional key.
        let chip_keys: BTreeMap<ChipId, &str> =
            legacy.chips.iter().filter_map(|(k, c)| Some((c.id?, k.as_str()))).collect();
        for c in &content.chips {
            let n = c.id.unwrap_or_default();
            let (key, whose) = match chip_keys.get(&n) {
                Some(k) => (k.to_string(), "the pack's chip, defined by number".to_string()),
                None => (format!("v1/chip-{n:03x}"), "the pack's chip record".to_string()),
            };
            chips.add(key.clone(), ChipDef { key, record: c.clone(), usage: None }, whose);
        }
        for d in definitions.of(Registry::Chip) {
            if legacy.chips.contains_key(&d.key) {
                continue;
            }
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
            let record = chip_record(d, &resolver)?;
            chips.add(d.key.clone(), ChipDef { key: d.key.clone(), record, usage: Some(usage) }, format!("defined in {}.luau", d.module));
        }
        let chips: Vec<ChipDef> = chips.sorted()?.into_iter().map(|(_, c)| c).collect();

        // The pack's navis, forms and stages.
        let key_of = |defined: &BTreeMap<String, u8>, n: u8, v1: String| -> String {
            defined.iter().find(|(_, m)| **m == n).map_or(v1, |(k, _)| k.clone())
        };
        let navi_keys: BTreeMap<String, u8> = legacy.navis.iter().map(|(k, n)| (k.clone(), n.id)).collect();
        let form_keys: BTreeMap<String, u8> = legacy.forms.iter().map(|(k, f)| (k.clone(), f.id)).collect();
        let mut navis = Entries::new(Registry::Navi);
        for n in &content.navis {
            let key = key_of(&navi_keys, n.id, format!("v1/navi-{:02x}", n.id));
            navis.add(key.clone(), NaviDef { key, record: n.clone() }, "the pack's navi".into());
        }
        let navis: Vec<NaviDef> = navis.sorted()?.into_iter().map(|(_, n)| n).collect();
        let mut forms = Entries::new(Registry::Form);
        for f in &content.forms {
            let key = key_of(&form_keys, f.id, format!("v1/form-{:02x}", f.id));
            forms.add(key.clone(), FormDef { key, record: f.clone() }, "the pack's form".into());
        }
        let forms: Vec<FormDef> = forms.sorted()?.into_iter().map(|(_, f)| f).collect();
        let mut stages = Entries::new(Registry::Stage);
        for (i, st) in content.rules.stages.settings.iter().enumerate() {
            let key = format!("v1/stage-{i:02x}");
            stages.add(key.clone(), StageDef { key, record: *st, number: Some(i as u8) }, "the pack's battle settings".into());
        }
        for (key, (st, number)) in &legacy.stages {
            stages.add(key.clone(), StageDef { key: key.clone(), record: *st, number: *number }, "a stage".into());
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
            let what = |e: String| ContentError::new(format!("{}.luau: roles: {e}", d.module));
            let Data::Map(groups) = &d.spec else { return Err(what("a table of role groups".into())) };
            for (group, entries) in groups {
                if group.to_string() != "actions" {
                    return Err(what(format!("the ruleset has no role group `{group}`")));
                }
                let Data::Map(entries) = entries else { return Err(what("`actions` is a table".into())) };
                for (name, v) in entries {
                    let name = name.to_string();
                    let slot = roles.actions.slot(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role actions.{name} (it has {})", RoleActions::NAMES.join(", ")))
                    })?;
                    let Data::Ref(Registry::Action, key) = v else {
                        return Err(what(format!("actions.{name} is not an action")));
                    };
                    *slot = Some(ActionHandle(
                        actions.binary_search_by(|a| a.key.as_str().cmp(key)).expect("a defined action") as u16,
                    ));
                }
            }
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
            hooks: BTreeMap::new(),
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
            if c.usage.is_none()
                && let Some(id) = c.record.id
            {
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
            if let Some(n) = st.number {
                defs.stage_numbers[n as usize] = Some(StageHandle(i as u16));
            }
        }
        defs.hooks = hooks.into_iter().map(|(hook, (f, _))| (hook, functions.id(f))).collect();
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
        // The modules that return a table with a `state` give its layout.
        let states = c.defs.definitions.modules.iter().filter(|m| m.state.is_some()).count();
        assert!(states > 100, "{states} module states");
    }
}
