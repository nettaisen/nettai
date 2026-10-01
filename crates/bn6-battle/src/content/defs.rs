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
    ChipData, ChipId, Content, DIMMING_CHIP_ACTION,
    FormData, INSTANT_CHIP_ACTION, NAVI_CHIP_ACTION, NaviData,
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
    /// Each has the engine's number (`Content::effect`, `Content::spark`):
    /// the numbered table's row with the same look, else one after the
    /// table's. See [`Defs::number`].
    pub effects: Vec<super::EffectSprite>,
    pub sparks: Vec<super::EffectSprite>,
    /// Hit regions content defines, by handle, with the engine's number for
    /// each: the numbered table's region of the same shape or condition,
    /// else one after the table's (a shape's after its shapes, a
    /// whole-field region's after its field regions, from 0x80).
    pub regions: Vec<(super::Region, u8)>,
    /// Collision types content defines, by handle. The engine's number for
    /// each (`Content::collision_type`) is its row in the numbered table
    /// (the row its `row_offset` names, with the same flags), else one
    /// after the table's.
    pub collisions: Vec<CollisionTypeDef>,
    /// What the ruleset needs from content by role (`define.roles`).
    pub roles: Roles,
    /// The engine's number for each effect, spark and collision type
    /// content defines, by handle; and those past the numbered tables, by
    /// number from the table's length (the same look or type defined twice
    /// is one number).
    effect_numbers: Vec<u8>,
    spark_numbers: Vec<u8>,
    collision_numbers: Vec<u8>,
    effect_extra: Vec<super::EffectSprite>,
    spark_extra: Vec<super::EffectSprite>,
    collision_extra: Vec<CollisionTypeDef>,
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
    /// byte (an effect, a spark, a region, a collision type): the numbered
    /// table's entry that is the same thing (the content's own table by
    /// number, which v1 modules and the ruleset still read), else its own
    /// after the table's. The engine learns no number from it: the table is
    /// content's, and a definition only shares its entry.
    pub fn number(&self, registry: Registry, h: u16) -> Option<u8> {
        let i = h as usize;
        match registry {
            Registry::Effect => self.effect_numbers.get(i).copied(),
            Registry::Spark => self.spark_numbers.get(i).copied(),
            Registry::Region => self.regions.get(i).map(|&(_, n)| n),
            Registry::Collision => self.collision_numbers.get(i).copied(),
            _ => None,
        }
    }

    /// A defined effect, spark or collision type past the numbered table,
    /// by the engine's number.
    pub(crate) fn effect_numbered(&self, n: u8) -> Option<super::EffectSprite> {
        n.checked_sub(self.effect_base).and_then(|i| self.effect_extra.get(i as usize)).copied()
    }

    pub(crate) fn spark_numbered(&self, n: u8) -> Option<super::EffectSprite> {
        n.checked_sub(self.spark_base).and_then(|i| self.spark_extra.get(i as usize)).copied()
    }

    pub(crate) fn collision_numbered(&self, n: u8) -> Option<CollisionTypeDef> {
        n.checked_sub(self.collision_base).and_then(|i| self.collision_extra.get(i as usize)).copied()
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
    // (The custom screen reads its lines: none given counts as three.)
    match json("description")? {
        Json::Null => {}
        text @ Json::String(_) => {
            o.insert("description".into(), text);
        }
        other => return Err(what(format!("`description` is {other}, not text"))),
    }
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
                        let Data::Int(n @ 0..=0xFF) = n else {
                            return Err(what(format!("{full}'s legacy action is {n:?}, not an action number")));
                        };
                        match actions.iter().position(|a| a.number == Some(n as u8)) {
                            Some(i) => RoleAction::Action(ActionHandle(i as u16)),
                            None => RoleAction::Unported(n as u8),
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
            let instant = match (&w.script, w.instant_chip) {
                (Some(script), Some(_)) => Some(functions.id(export(&definitions, script, "instant_chip", &whose)?)),
                _ => None,
            };
            let def = WeaponDef {
                key: key.clone(),
                name: w.name.clone(),
                setup,
                number: w.routines.first().copied(),
                routines: w.routines.clone(),
                charge_ticks: w.charge_ticks.clone(),
                instant,
            };
            weapons.add(key.clone(), def, whose);
        }
        for w in content.weapons.iter().filter(|w| !claimed.contains_key(&w.id)) {
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
                    instant: None,
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
            let def = WeaponDef { key: d.key.clone(), name, setup, routines, number, charge_ticks, instant: None };
            weapons.add(d.key.clone(), def, format!("defined in {}.luau", d.module));
        }
        let weapons: Vec<WeaponDef> = weapons.sorted()?.into_iter().map(|(_, w)| w).collect();

        // Chips content defines.
        let action_handle = |key: &str| -> Option<ActionHandle> {
            actions.binary_search_by(|a| a.key.as_str().cmp(key)).ok().map(|i| ActionHandle(i as u16))
        };
        let mut chips = Entries::new(Registry::Chip);
        // A chip definition's own use: exactly one of its `action`,
        // `dimming`, `navi` and `instant` (none: `Ok(None)`).
        let own_usage = |d: &Definition, functions: &mut Functions| -> Result<Option<ChipUsage>, ContentError> {
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
            match usages[..] {
                [] => Ok(None),
                [usage] => Ok(Some(usage)),
                _ => Err(ContentError::new(format!(
                    "{}.luau: chip {} needs exactly one of `action`, `dimming`, `navi` and `instant`",
                    d.module, d.key
                ))),
            }
        };
        // The pack's chips: those a definition gives by number (its legacy
        // marker) under its key, the others' records (the test content's)
        // under a transitional key. A numbered definition's use is its own
        // where it has one, else what registration by number resolves from
        // the record's action and subtype (a behaviour still a v1 module).
        let chip_keys: BTreeMap<ChipId, &str> =
            legacy.chips.iter().filter_map(|(k, c)| Some((c.id?, k.as_str()))).collect();
        for c in &content.chips {
            let n = c.id.unwrap_or_default();
            let (key, whose, own) = match chip_keys.get(&n) {
                Some(k) => {
                    let d = definitions.get(Registry::Chip, k).expect("a numbered chip's definition");
                    (k.to_string(), format!("defined in {}.luau", d.module), own_usage(d, &mut functions)?)
                }
                None => (format!("v1/chip-{n:03x}"), "the pack's chip record".to_string(), None),
            };
            // Registration by number: the record's action, or for the
            // ruleset's generic chip actions its subtype's registration. A
            // record whose module exports an action runs that instead, as
            // a chip definition's `action` does.
            let by_subtype = |table: &BTreeMap<u8, FnId>, f: fn(FnId) -> ChipUsage, unported: fn(u8) -> Unported| {
                table.get(&c.subtype).map_or(ChipUsage::Unported(unported(c.subtype)), |&h| f(h))
            };
            let exported = match (&own, &c.script) {
                (None, Some(module)) => record_action(&definitions, module, &key)?,
                _ => None,
            };
            let usage = match (own, exported, c.action) {
                (Some(usage), _, _) => usage,
                (None, Some(a), _) => ChipUsage::Action(action_handle(a).expect("a defined action")),
                (None, None, DIMMING_CHIP_ACTION) => by_subtype(&dimming_hooks, ChipUsage::Dimming, Unported::Dimming),
                (None, None, NAVI_CHIP_ACTION) => by_subtype(&navi_hooks, ChipUsage::Navi, Unported::Navi),
                (None, None, INSTANT_CHIP_ACTION) => by_subtype(&instant_hooks, ChipUsage::Instant, Unported::Instant),
                (None, None, n) => match actions.iter().position(|a| a.number == Some(n)) {
                    Some(i) => ChipUsage::Action(ActionHandle(i as u16)),
                    None => ChipUsage::Unported(Unported::Action(n)),
                },
            };
            chips.add(key.clone(), ChipDef { key, record: c.clone(), usage }, whose);
        }
        for d in definitions.of(Registry::Chip) {
            if legacy.chips.contains_key(&d.key) {
                continue;
            }
            let Some(usage) = own_usage(d, &mut functions)? else {
                return Err(ContentError::new(format!(
                    "{}.luau: chip {} needs exactly one of `action`, `dimming`, `navi` and `instant` (or, for a behaviour still a v1 module, a legacy marker with its `number`)",
                    d.module, d.key
                )));
            };
            let record = chip_record(d, &resolver)?;
            chips.add(d.key.clone(), ChipDef { key: d.key.clone(), record, usage }, format!("defined in {}.luau", d.module));
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
        // The engine's numbers for what content defines: the numbered
        // table's entry that is the same thing, else a number after the
        // table's (one per distinct thing).
        fn numbers<T: PartialEq + Copy>(
            table_len: usize,
            defined: &[T],
            in_table: impl Fn(&T) -> Option<usize>,
            what: &str,
        ) -> Result<(u8, Vec<u8>, Vec<T>), ContentError> {
            let mut extra: Vec<T> = Vec::new();
            let mut out = Vec::with_capacity(defined.len());
            for x in defined {
                let n = match in_table(x) {
                    Some(i) => i,
                    None => match extra.iter().position(|e| e == x) {
                        Some(i) => table_len + i,
                        None => {
                            extra.push(*x);
                            table_len + extra.len() - 1
                        }
                    },
                };
                out.push(u8::try_from(n).map_err(|_| {
                    ContentError::new(format!(
                        "too many {what}: {table_len} in the numbered table and more than {} others defined",
                        0x100usize.saturating_sub(table_len)
                    ))
                })?);
            }
            Ok((table_len.min(0xFF) as u8, out, extra))
        }
        let (effect_base, effect_numbers, effect_extra) =
            numbers(content.effects.len(), &effects, |e| content.effects.iter().position(|t| t == e), "effects")?;
        let (spark_base, spark_numbers, spark_extra) =
            numbers(content.sparks.len(), &sparks, |e| content.sparks.iter().position(|t| t == e), "hit sparks")?;
        let (mut shapes, mut fields) = (content.regions.len(), content.rules.field_regions.len());
        let mut regions: Vec<(super::Region, u8)> = Vec::new();
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
                    // The numbered table's region of this shape (region
                    // 0 is none), else one a definition before it made,
                    // else a new one.
                    let region = super::Region::Panels(panels);
                    let super::Region::Panels(panels) = &region else { unreachable!() };
                    let number = match content.regions.iter().skip(1).position(|r| r == panels) {
                        Some(i) => i + 1,
                        None => match regions.iter().find(|(r, _)| *r == region) {
                            Some(&(_, n)) => n as usize,
                            None => {
                                shapes += 1;
                                shapes - 1
                            }
                        },
                    };
                    (region, number)
                }
                (Data::Nil, Data::Map(_)) => {
                    let word = |k: &str| d.spec.field("field").field(k).int().unwrap_or(0) as u32;
                    let condition = super::PanelCondition { require: word("require"), forbid: word("forbid") };
                    let region = super::Region::Field(condition);
                    let number = match content.rules.field_regions.iter().position(|c| *c == condition) {
                        Some(i) => 0x80 + i,
                        None => match regions.iter().find(|(r, _)| *r == region) {
                            Some(&(_, n)) => n as usize,
                            None => {
                                fields += 1;
                                0x80 + fields - 1
                            }
                        },
                    };
                    (region, number)
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
        // A collision type is its row of the numbered table when its
        // `row_offset` names one with its flags.
        let rows = &content.rules.collision_types;
        let (collision_base, collision_numbers, collision_extra) = numbers(
            rows.len(),
            &collisions,
            |c| {
                let row = (c.row_offset / 8) as usize;
                (c.row_offset % 8 == 0 && rows.get(row) == Some(&c.flags)).then_some(row)
            },
            "collision types",
        )?;

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
            effect_numbers,
            spark_numbers,
            collision_numbers,
            effect_extra,
            spark_extra,
            collision_extra,
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
            if let Some(n) = st.number {
                defs.stage_numbers[n as usize] = Some(StageHandle(i as u16));
            }
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
        // The modules that return a table with a `state` (the v1 modules
        // registration by number runs: fewer as families convert) give its
        // layout.
        let states = c.defs.definitions.modules.iter().filter(|m| m.state.is_some()).count();
        assert!(states > 0, "{states} module states");
        // The numbered tables are the definitions' (step 5): what the
        // modules define of them shares their entries, so the engine's
        // byte holds them all.
        assert!(c.effects.len() > 100 && c.defs.effects.len() > c.effects.len(), "{} effects defined", c.defs.effects.len());
        numbers_give_the_definitions_back(&c);
    }

    /// The engine's number for each effect, spark, region and collision
    /// type content defines reads back as what was defined; one the
    /// numbered table holds is that entry, another comes after the table.
    fn numbers_give_the_definitions_back(c: &Content) {
        let d = &c.defs;
        for (h, look) in d.effects.iter().enumerate() {
            let n = d.number(Registry::Effect, h as u16).expect("an effect's number");
            assert_eq!(c.effect(n), *look, "effect {h}");
            assert_eq!(c.effects.iter().position(|e| e == look).unwrap_or(n as usize), n as usize, "effect {h}");
        }
        for (h, look) in d.sparks.iter().enumerate() {
            let n = d.number(Registry::Spark, h as u16).expect("a spark's number");
            assert_eq!(c.spark(n), *look, "spark {h}");
            assert_eq!(c.sparks.iter().position(|e| e == look).unwrap_or(n as usize), n as usize, "spark {h}");
        }
        for (h, t) in d.collisions.iter().enumerate() {
            let n = d.number(Registry::Collision, h as u16).expect("a collision type's number");
            for side in 0..2 {
                assert_eq!(c.collision_type(n, side), (t.flags[side as usize], t.row_offset), "collision type {h}");
            }
        }
        for (h, (region, n)) in d.regions.iter().enumerate() {
            assert_eq!(d.number(Registry::Region, h as u16), Some(*n));
            match region {
                crate::content::Region::Panels(p) => assert_eq!(c.region(*n), &p[..], "region {h}"),
                crate::content::Region::Field(f) => assert_eq!(c.field_region(*n), *f, "region {h}"),
            }
        }
    }

    #[test]
    fn a_definition_shares_the_numbered_tables_entry_it_is() {
        // The test content's tables are its own; the modules it loads
        // define some of the same looks, regions and collision types.
        let c = crate::content::testing::with_test_pack();
        assert!(!c.defs.effects.is_empty() && !c.defs.collisions.is_empty() && !c.defs.regions.is_empty());
        numbers_give_the_definitions_back(&c);
        // The same look defined twice is one number.
        for (i, a) in c.defs.effects.iter().enumerate() {
            for (j, b) in c.defs.effects.iter().enumerate() {
                if a == b {
                    assert_eq!(c.defs.number(Registry::Effect, i as u16), c.defs.number(Registry::Effect, j as u16));
                }
            }
        }
    }
}
