//! What the content defines (docs/design/content-model-v2.md): the
//! registries the engine reads, with their handles, and the functions the
//! runtime binds.
//!
//! A registry holds three sorts of entries while the migration runs:
//!
//! - the engine's own (its object kinds, keyed `engine/...`);
//! - content registered by number from the pack's data (an object folder's
//!   `[kind]`), keyed from that data (the kind's folder name); and the
//!   pack's records content doesn't define yet: its navis and forms
//!   (`v1/navi-01`, `v1/form-0c`);
//! - content's definitions (`define.kind { ... }`), keyed by their keys.
//!   Chips and weapons are only these: nothing names one by number.
//!
//! Each registry's keys are sorted byte-wise; an entry's handle is its
//! place. Registration by number still reaches its entries by the numbers
//! its own data gives (an object slot), until the migration's last step
//! deletes it.
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
    ChipData, Content,
    FormData, NaviData,
};
use super::roles::{
    ActionRole, BannerRole, ChipRole, CollisionRole, EffectRole, HookRole, KindRole, LockonRole, MusicRole, RegionRole,
    RoleAction, RoleKind, Roles, SoundRole, SparkRole, SpriteRole, StatusRole,
};
use crate::setup::{Form, Navi};
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
    /// What places it when a stage names it (`kind.place`).
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
    /// The action number registration by number gives it (a
    /// `weapon.toml`'s): the navi's CurAction while it runs. An action
    /// content defines has none; the navi's CurAction is then
    /// [`crate::kinds::player::CONTENT_ACTION`].
    pub number: Option<u8>,
}

/// A weapon: what a button's weapon does.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WeaponDef {
    pub key: String,
    pub name: String,
    /// `setup(navi) -> action` (`off_80117D4`'s routine); none for a weapon
    /// nothing can start: an A-charge that is its chip (`charged_chip`), or
    /// one nothing implements yet.
    pub setup: Option<FnId>,
    /// Ticks to a full charge by Charge stat (past the original's five,
    /// what the game reads on into).
    pub charge_ticks: Vec<u16>,
    /// The instant effect its action (the instant chips' action) runs: a
    /// chip's (the arm chips' weapons'), or one no chip has (TenguCross's
    /// wind).
    pub instant: Option<FnId>,
    /// The navi waits 8 ticks after the instant effect (the original's
    /// effect 0x14, which no chip has: TenguCross's wind).
    pub instant_waits: bool,
    /// As a charged shot: it stays when the form's own would replace it
    /// (`sub_800FFAA`: a chip's weapon), and its attack is of the kind
    /// that runs through a dimming.
    pub sticky: bool,
    /// As a buster: it fires while B is held.
    pub held: bool,
    /// As a buster: the buster it gives way to while the charged shot is a
    /// sticky one (`sub_800FFAA`).
    pub plain: Option<WeaponHandle>,
    /// As an A-charge: what the charged chip is, where it isn't this
    /// weapon's own attack.
    pub charged_chip: Option<ChargedChip>,
}

/// What a charged chip is under an A-charge weapon that has no attack of
/// its own (the original's `nullsub_44` entries, which `sub_800FB54` tells
/// apart by number before it would call them).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChargedChip {
    /// The chip itself, with the charge's bonus.
    Bonus,
    /// The chip itself after GroundCross's falling rocks (`sub_8012CB2`).
    RockBarrage,
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

/// A chip content defines. (The engine never learns the original's number
/// for one: compat has it by key.)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChipDef {
    pub key: String,
    pub record: ChipData,
    /// How it is used: its definition's `action`, `dimming`, `navi` or
    /// `instant`.
    pub usage: ChipUsage,
    /// What its record names, by handle.
    pub links: ChipLinks,
}

/// What a chip's record names by key or name, resolved: the ruleset reads
/// these.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChipLinks {
    /// A dark chip's substitute (`ChipData::dark_substitute`).
    pub dark_substitute: Option<ChipHandle>,
    /// An SP navi chip's slot among the setup's SP deletion times
    /// (`DamageFormula::SpNavi::slot`, by the rules' `sp_slots`).
    pub sp_slot: Option<u8>,
    /// The navi whose own chip this is (a link navi's chip: offered on the
    /// custom screen once a round, never charged, dropped from the hand
    /// when a round ends).
    pub own_chip_of: Option<NaviHandle>,
    /// Its place among the Program Advances' results (those chips with a
    /// recipe, in handle order): the bit a formed one takes in a player's
    /// record of the round.
    pub advance: Option<u8>,
}

/// A navi: the pack's record (keyed `v1/navi-01`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NaviDef {
    pub key: String,
    pub record: NaviData,
    /// A link navi's own chip (`NaviData::own_chip`), by handle.
    pub own_chip: Option<(ChipHandle, super::ChipCode)>,
}

/// One of MegaMan's forms: the pack's record (keyed `v1/form-0c`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FormDef {
    pub key: String,
    pub record: FormData,
}

/// A stage (`define.stage`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StageDef {
    pub key: String,
    pub record: super::StageData,
}

/// A status effect (`define.status`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StatusDef {
    pub key: String,
    pub effect: super::StatusEffect,
}

/// A Beast Out lock-on mode (`define.lockon`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LockonDef {
    pub key: String,
    pub mode: super::LockonMode,
}

/// A collision type content defines (`define.collision`): what an object is
/// or what it hits, as the flag words for side 0 and side 1
/// (`sub_801A0BA`). A reacts to B when A's target flags meet B's self
/// flags.
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
    /// The Beast Out lock-on modes, by handle.
    pub lockons: Vec<LockonDef>,
    /// The identities, by handle (`define.identity`, in key order).
    pub identities: Vec<super::Identity>,
    /// The status effects, by handle.
    pub statuses: Vec<StatusDef>,
    pub records: Vec<RecordDef>,
    /// One-shot effects' and hit sparks' looks (`define.effect`,
    /// `define.spark`), by handle.
    pub effects: Vec<super::EffectSprite>,
    pub sparks: Vec<super::EffectSprite>,
    /// Hit regions (`define.region`), by handle.
    pub regions: Vec<super::Region>,
    /// Collision types (`define.collision`), by handle.
    pub collisions: Vec<CollisionTypeDef>,
    /// What the ruleset needs from content by role (`define.roles`).
    pub roles: Roles,
    /// The Program Advances, in the order they are tried (each chip holds
    /// the recipes that make it; `sub_8029520`).
    pub program_advances: Vec<super::ProgramAdvance>,
    /// The Cross special's chips by row (`Rules::cross_special`), each with
    /// the chip whose damage it strikes with, if another's.
    pub cross_special: Vec<Vec<(ChipHandle, Option<ChipHandle>)>>,
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
    /// The pack's navis and forms by their numbers (registration by
    /// number).
    navi_numbers: Vec<Option<NaviHandle>>,
    form_numbers: Vec<Option<FormHandle>>,
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

    pub fn stage(&self, h: StageHandle) -> &StageDef {
        &self.stages[h.index()]
    }

    /// The stage with this key.
    pub fn stage_by_key(&self, key: &str) -> Option<StageHandle> {
        self.stages.binary_search_by(|s| s.key.as_str().cmp(key)).ok().map(|i| StageHandle(i as u16))
    }

    /// The status effect with this key.
    pub fn status_by_key(&self, key: &str) -> Option<bn6_content_api::StatusHandle> {
        self.statuses.binary_search_by(|s| s.key.as_str().cmp(key)).ok().map(|i| bn6_content_api::StatusHandle(i as u16))
    }

    /// The identity with this key.
    pub fn identity_by_key(&self, key: &str) -> Option<bn6_content_api::IdentityHandle> {
        self.identities.binary_search_by(|i| i.key.as_str().cmp(key)).ok().map(|i| bn6_content_api::IdentityHandle(i as u16))
    }

    /// The lock-on mode with this key.
    pub fn lockon_by_key(&self, key: &str) -> Option<bn6_content_api::LockonHandle> {
        self.lockons.binary_search_by(|l| l.key.as_str().cmp(key)).ok().map(|i| bn6_content_api::LockonHandle(i as u16))
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

/// An identity a navi or form names is that navi's or form's: one owner
/// each, of a player's class.
fn claim_identity(
    identities: &mut [super::Identity],
    h: bn6_content_api::IdentityHandle,
    owner: super::IdentityOwner,
    whose: &str,
) -> Result<(), ContentError> {
    let id = &mut identities[h.index()];
    if let Some(other) = id.owner {
        return Err(ContentError::new(format!("identity {} is both {other:?}'s and {whose}'s", id.key)));
    }
    if !id.class.is_player() {
        return Err(ContentError::new(format!(
            "identity {} ({whose}'s) is of class {:?}: a navi's or a form's is a player's (megaman, link_navi, cross, beast, cross_beast, beast_over)",
            id.key, id.class
        )));
    }
    id.owner = Some(owner);
    Ok(())
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
/// fields the engine reads, with the lock-on mode by the handle `r` gives
/// it, and the chips it names (its Program Advance recipes' ingredients, a
/// dark chip's substitute) by key: the registry resolves those to handles
/// once every chip has one (`ChipDef::links`).
pub(crate) fn chip_record(d: &Definition, r: &super::legacy::Resolver) -> Result<ChipData, ContentError> {
    use serde_json::{Map, Value as Json};
    let what = |e: String| ContentError::new(format!("{}.luau: chip {}: {e}", d.module, d.key));
    let spec = &d.spec;
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
    // `damage`: a number, or a formula (`{ formula = "hp_lost" }`).
    match o["damage"].take() {
        formula @ Json::Object(_) => {
            o.insert("formula".into(), formula);
            o.insert("damage".into(), 0.into());
        }
        Json::Number(n) if n.as_i64().is_some_and(|v| (0..1000).contains(&v)) => {
            o.insert("damage".into(), Json::Number(n));
        }
        other => return Err(what(format!("`damage` is {other}: a number below 1000, or a formula"))),
    }
    // What the ruleset asks of this chip: its traits, the trap it is, a
    // dark chip's cost and its substitute (a chip, by key here).
    for field in ["traits", "trap", "hp_bug", "dark_substitute"] {
        let v = json(field)?;
        if !v.is_null() {
            o.insert(field.into(), v);
        }
    }
    let library = spec.field("library");
    for (field, from) in [("library_number", "number"), ("library_index", "index"), ("sort_key", "sort")] {
        o.insert(field.into(), library.field(from).int().unwrap_or(0).into());
    }
    let beast = spec.field("beast");
    o.insert("beast_lockon".into(), Json::Bool(!beast.is_nil() && !matches!(beast.field("rush"), Data::Bool(false))));
    // (A lock-on mode's number is its handle: `Resolver::new`.)
    let mode = match beast.field("lockon") {
        Data::Nil => Json::Null,
        v @ Data::Ref(Registry::Lockon, _) => r.json(v, &format!("chip {}.beast.lockon", d.key)).map_err(what)?,
        _ => return Err(what("`beast.lockon` is a lock-on mode (rules/lockon)".into())),
    };
    o.insert("lockon_mode".into(), mode);
    o.insert("program_advance".into(), json("program_advances")?);
    if o["program_advance"].is_null() {
        o.insert("program_advance".into(), Json::Array(Vec::new()));
    }
    // (A chip is its definition: it has no number, subtype or parameter
    // bytes for a marker to give.)
    if !spec.field("legacy").is_nil() {
        return Err(what("a chip takes no `legacy` marker: its use is its `action`, `dimming`, `navi` or `instant`".into()));
    }
    serde_json::from_value(Json::Object(o)).map_err(|e| what(e.to_string()))
}

/// `define.roles { actions = { ... }, kinds = { ... } }` (content::roles):
/// each role a definition, or a v1 registration through its `legacy`
/// marker (an action by number, a kind by key).
fn read_roles(
    d: &Definition,
    definitions: &Definitions,
    assets: &bn6_content_api::AssetNames,
    actions: &[ActionDef],
    kinds: &[KindDef],
    chips: &[ChipDef],
    lockons: &[LockonDef],
    statuses: &[StatusDef],
    functions: &mut Functions,
) -> Result<Roles, ContentError> {
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
                "chips" => {
                    let names: Vec<&str> = ChipRole::ALL.iter().map(|r| r.name()).collect();
                    let role = ChipRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role chips.{name} (it has {})", names.join(", ")))
                    })?;
                    let full = format!("chips.{name}");
                    let Data::Ref(Registry::Chip, key) = v else {
                        return Err(what(format!("{full} is not a chip")));
                    };
                    let i = chips
                        .binary_search_by(|c| c.key.as_str().cmp(&key))
                        .map_err(|_| what(format!("{full} names the chip {key:?}, which the content doesn't have")))?;
                    roles.chips.insert(role, ChipHandle(i as u16));
                }
                "lockon" => {
                    let names: Vec<&str> = LockonRole::ALL.iter().map(|r| r.name()).collect();
                    let role = LockonRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role lockon.{name} (it has {})", names.join(", ")))
                    })?;
                    let Data::Ref(Registry::Lockon, key) = v else {
                        return Err(what(format!("lockon.{name} is not a lock-on mode")));
                    };
                    let h = lockons.iter().position(|l| &l.key == key).expect("a defined lock-on mode");
                    roles.lockons.insert(role, bn6_content_api::LockonHandle(h as u16));
                }
                "statuses" => {
                    let names: Vec<&str> = StatusRole::ALL.iter().map(|r| r.name()).collect();
                    let role = StatusRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role statuses.{name} (it has {})", names.join(", ")))
                    })?;
                    let Data::Ref(Registry::Status, key) = v else {
                        return Err(what(format!("statuses.{name} is not a status")));
                    };
                    let h = statuses.iter().position(|s| &s.key == key).expect("a defined status");
                    roles.statuses.insert(role, bn6_content_api::StatusHandle(h as u16));
                }
                "effects" => {
                    let role = definition_role(&group, &name, EffectRole::named, EffectRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let h = definition_handle(definitions, Registry::Effect, &group, &name, v).map_err(&what)?;
                    roles.effects.insert(role, bn6_content_api::EffectHandle(h));
                }
                "sparks" => {
                    let role = definition_role(&group, &name, SparkRole::named, SparkRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let h = definition_handle(definitions, Registry::Spark, &group, &name, v).map_err(&what)?;
                    roles.sparks.insert(role, bn6_content_api::SparkHandle(h));
                }
                "regions" => {
                    let role = definition_role(&group, &name, RegionRole::named, RegionRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let h = definition_handle(definitions, Registry::Region, &group, &name, v).map_err(&what)?;
                    roles.regions.insert(role, bn6_content_api::RegionHandle(h));
                }
                "collision" => {
                    let role =
                        definition_role(&group, &name, CollisionRole::named, CollisionRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let h = definition_handle(definitions, Registry::Collision, &group, &name, v).map_err(&what)?;
                    roles.collisions.insert(role, bn6_content_api::CollisionHandle(h));
                }
                "sounds" => {
                    let role = definition_role(&group, &name, SoundRole::named, SoundRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let id = role_asset(assets, bn6_content_api::AssetKind::Sound, &group, &name, v).map_err(&what)?;
                    roles.sounds.insert(role, crate::sound::SoundId(id));
                }
                "music" => {
                    let role = definition_role(&group, &name, MusicRole::named, MusicRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let id = role_asset(assets, bn6_content_api::AssetKind::Sound, &group, &name, v).map_err(&what)?;
                    roles.music.insert(role, crate::sound::SoundId(id));
                }
                "sprites" => {
                    let role = definition_role(&group, &name, SpriteRole::named, SpriteRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let sprite = match v {
                        Data::Asset(bn6_content_api::AssetKind::Sprite, asset) => assets
                            .sprites
                            .get(asset)
                            .copied()
                            .ok_or_else(|| what(format!("sprites.{name}: the pack has no sprite {asset:?}")))?,
                        _ => return Err(what(format!("sprites.{name} is not a sprite asset (asset.sprite(...))"))),
                    };
                    roles.sprites.insert(role, sprite);
                }
                "banners" => {
                    let role = definition_role(&group, &name, BannerRole::named, BannerRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let id = role_asset(assets, bn6_content_api::AssetKind::Banner, &group, &name, v).map_err(&what)?;
                    roles.banners.insert(role, super::BannerId(id as u8));
                }
                _ => {
                    return Err(what(format!(
                        "the ruleset has no role group `{group}` (it has actions, kinds, hooks, chips, lockon, statuses, effects, sparks, regions, collision, sounds, music, sprites, banners)"
                    )));
                }
            }
        }
    }
    Ok(roles)
}

/// The role `group.name`, of a group whose roles name definitions.
fn definition_role<R>(
    group: &str,
    name: &str,
    named: impl Fn(&str) -> Option<R>,
    names: impl Iterator<Item = &'static str>,
) -> Result<R, String> {
    named(name).ok_or_else(|| {
        format!("the ruleset has no role {group}.{name} (it has {})", names.collect::<Vec<_>>().join(", "))
    })
}

/// The asset of `kind` the role `group.name` names, as the engine
/// identifies it (the pack's id for it).
fn role_asset(
    assets: &bn6_content_api::AssetNames,
    kind: bn6_content_api::AssetKind,
    group: &str,
    name: &str,
    v: &Data,
) -> Result<u16, String> {
    match v {
        Data::Asset(k, asset) if *k == kind => assets
            .handle(kind, asset)
            .and_then(|h| assets.number(kind, h))
            .ok_or_else(|| format!("{group}.{name}: the pack has no {kind} {asset:?}")),
        _ => Err(format!("{group}.{name} is not a {kind} asset (asset.{kind}(...))")),
    }
}

/// The handle of the `registry` definition the role `group.name` names: its
/// place in the registry's key order.
fn definition_handle(definitions: &Definitions, registry: Registry, group: &str, name: &str, v: &Data) -> Result<u16, String> {
    match v {
        Data::Ref(r, key) if *r == registry => {
            Ok(definitions.of(registry).iter().position(|d| &d.key == key).expect("a defined entry") as u16)
        }
        _ => Err(format!("{group}.{name} is not a {registry} definition")),
    }
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
        for k in &content.objects.kinds {
            let whose = format!("objects/{} (object.toml)", k.name);
            let update = export(&definitions, &k.script, "update", &whose)?;
            let def = KindDef {
                key: k.name.clone(),
                pool: k.pool,
                implementation: KindImpl::Script { update: functions.id(update) },
                schema: module_state(&k.script),
                slot: Some((k.pool, k.index)),
                place: None,
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

        // The actions content defines. (No action is registered by number:
        // every chip's and weapon's is a definition.)
        let mut actions = Entries::new(Registry::Action);
        for d in definitions.of(Registry::Action) {
            let def =
                ActionDef { key: d.key.clone(), update: functions.id(slot(d, "update")?), schema: state_of(d)?, number: None };
            actions.add(d.key.clone(), def, format!("defined in {}.luau", d.module));
        }
        let actions: Vec<ActionDef> = actions.sorted()?.into_iter().map(|(_, a)| a).collect();

        // Weapons: what content defines. A weapon names another by key
        // (`plain`), so their handles come first.
        let mut weapon_defs: Vec<&Definition> = definitions.of(Registry::Weapon).iter().collect();
        weapon_defs.sort_by(|a, b| a.key.cmp(&b.key));
        for w in weapon_defs.windows(2) {
            if w[0].key == w[1].key {
                return Err(ContentError::new(format!(
                    "weapon {:?} is defined in both {}.luau and {}.luau",
                    w[0].key, w[0].module, w[1].module
                )));
            }
        }
        let weapon_handle = |key: &str| -> Option<WeaponHandle> {
            weapon_defs.binary_search_by(|d| d.key.as_str().cmp(key)).ok().map(|i| WeaponHandle(i as u16))
        };
        let mut weapons = Vec::with_capacity(weapon_defs.len());
        for d in &weapon_defs {
            let what = |e: String| ContentError::new(format!("{}.luau: weapon {}: {e}", d.module, d.key));
            let name = d.spec.field("name").str().unwrap_or(&d.key).to_string();
            let charge_ticks: Vec<u16> = match d.spec.field("charge_ticks") {
                Data::List(items) => items
                    .iter()
                    .map(|t| match t {
                        Data::Int(i) if (0..=0xFFFF).contains(i) => Ok(*i as u16),
                        other => Err(what(format!("`charge_ticks` holds {other:?}, not a tick count"))),
                    })
                    .collect::<Result<_, _>>()?,
                other => return Err(what(format!("`charge_ticks` is {other:?}, not a list"))),
            };
            let flag = |field: &str| -> Result<bool, ContentError> {
                match d.spec.field(field) {
                    Data::Nil => Ok(false),
                    Data::Bool(b) => Ok(*b),
                    other => Err(what(format!("`{field}` is {other:?}, not true or false"))),
                }
            };
            let setup = match d.spec.field("setup") {
                Data::Nil => None,
                _ => Some(functions.id(slot(d, "setup")?)),
            };
            // Its own instant effect (`instant = function(user, spec)`),
            // which the instant chips' action runs.
            let instant = match d.spec.field("instant") {
                Data::Nil => None,
                _ => Some(functions.id(slot(d, "instant")?)),
            };
            let plain = match d.spec.field("plain") {
                Data::Nil => None,
                Data::Ref(Registry::Weapon, key) => {
                    Some(weapon_handle(key).ok_or_else(|| what(format!("`plain` names {key:?}, which is not a weapon")))?)
                }
                other => return Err(what(format!("`plain` is {other:?}, not a weapon"))),
            };
            let charged_chip = match d.spec.field("charged_chip") {
                Data::Nil => None,
                Data::Str(s) if s == "bonus" => Some(ChargedChip::Bonus),
                Data::Str(s) if s == "rock_barrage" => Some(ChargedChip::RockBarrage),
                other => return Err(what(format!("`charged_chip` is {other:?}, not \"bonus\" or \"rock_barrage\""))),
            };
            if charged_chip.is_some() && setup.is_some() {
                return Err(what("a weapon with a `setup` is its own attack: it has no `charged_chip`".into()));
            }
            weapons.push(WeaponDef {
                key: d.key.clone(),
                name,
                setup,
                charge_ticks,
                instant,
                instant_waits: flag("instant_waits")?,
                sticky: flag("sticky")?,
                held: flag("held")?,
                plain,
                charged_chip,
            });
        }
        // A navi's or form's weapons, by slot.
        let read_weapons = |d: &Definition| -> Result<super::FormWeapons, ContentError> {
            let what = |e: String| ContentError::new(format!("{}.luau: {} {}: {e}", d.module, d.registry, d.key));
            let mut out = super::FormWeapons::default();
            let entries = match d.spec.field("weapons") {
                Data::Nil => return Ok(out),
                Data::Map(entries) => entries,
                other => return Err(what(format!("`weapons` is {other:?}, not weapons by slot"))),
            };
            for (slot, v) in entries {
                let name = slot.to_string();
                let Some(at) = out.slot_mut(&name) else {
                    return Err(what(format!("`weapons` has no slot {name:?} (it has {})", super::FormWeapons::SLOTS.join(", "))));
                };
                *at = match v {
                    Data::Ref(Registry::Weapon, key) => {
                        Some(weapon_handle(key).ok_or_else(|| what(format!("weapons.{name} names {key:?}, which is not a weapon")))?)
                    }
                    other => return Err(what(format!("weapons.{name} is {other:?}, not a weapon"))),
                };
            }
            Ok(out)
        };

        // Chips content defines.
        let action_handle = |key: &str| -> Option<ActionHandle> {
            actions.binary_search_by(|a| a.key.as_str().cmp(key)).ok().map(|i| ActionHandle(i as u16))
        };
        let mut chips = Entries::new(Registry::Chip);
        // The chips content defines: each with exactly one use, its
        // `action`, `dimming`, `navi` or `instant`.
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
            let record = chip_record(d, &resolver)?;
            chips.add(
                d.key.clone(),
                ChipDef { key: d.key.clone(), record, usage, links: ChipLinks::default() },
                format!("defined in {}.luau", d.module),
            );
        }
        let mut chips: Vec<ChipDef> = chips.sorted()?.into_iter().map(|(_, c)| c).collect();

        // What the chips' records name, by handle: the Program Advances
        // (their recipes' ingredients), a dark chip's substitute, an SP
        // navi chip's slot.
        let chip_handle = |key: &str, whose: &str| -> Result<ChipHandle, ContentError> {
            chips
                .binary_search_by(|c| c.key.as_str().cmp(key))
                .map(|i| ChipHandle(i as u16))
                .map_err(|_| ContentError::new(format!("{whose} names the chip {key:?}, which the content doesn't have")))
        };
        let mut advances: Vec<(u8, super::ProgramAdvance)> = Vec::new();
        let mut links = Vec::with_capacity(chips.len());
        let mut results = 0u8;
        for (i, c) in chips.iter().enumerate() {
            let whose = format!("chip {}", c.key);
            let mut l = ChipLinks::default();
            if let Some(sub) = &c.record.dark_substitute {
                l.dark_substitute = Some(chip_handle(sub, &format!("{whose}'s dark_substitute"))?);
            }
            if let Some(super::DamageFormula::SpNavi { slot, .. }) = &c.record.formula {
                let n = content.rules.sp_slots.iter().position(|s| s == slot).ok_or_else(|| {
                    ContentError::new(format!("{whose}'s damage is by the SP navi {slot:?}, which the rules' sp_slots don't list"))
                })?;
                l.sp_slot = Some(n as u8);
            }
            for r in &c.record.program_advances {
                let at = format!("{whose}'s recipe");
                let recipe = match &r.recipe {
                    super::PaRecipe::CodeRun { chip, count } => super::Recipe::CodeRun { chip: chip_handle(chip, &at)?, count: *count },
                    super::PaRecipe::Sequence(keys) => {
                        super::Recipe::Sequence(keys.iter().map(|k| chip_handle(k, &at)).collect::<Result<_, _>>()?)
                    }
                };
                advances.push((r.order, super::ProgramAdvance { result: ChipHandle(i as u16), recipe }));
            }
            if !c.record.program_advances.is_empty() {
                // (A player's record of the round's formed ones is 32 bits.)
                if results >= 32 {
                    return Err(ContentError::new(format!("{whose}: more than 32 chips are Program Advances")));
                }
                l.advance = Some(results);
                results += 1;
            }
            links.push(l);
        }
        for (c, l) in chips.iter_mut().zip(links) {
            c.links = l;
        }
        advances.sort_by_key(|(order, _)| *order);
        let program_advances: Vec<super::ProgramAdvance> = advances.into_iter().map(|(_, pa)| pa).collect();
        let chip_handle = |key: &str, whose: &str| -> Result<ChipHandle, ContentError> {
            chips
                .binary_search_by(|c| c.key.as_str().cmp(key))
                .map(|i| ChipHandle(i as u16))
                .map_err(|_| ContentError::new(format!("{whose} names the chip {key:?}, which the content doesn't have")))
        };
        let mut cross_special = Vec::with_capacity(content.rules.cross_special.len());
        for row in &content.rules.cross_special {
            let at = "the Cross special's chips (rules cross-special)";
            let mut out = Vec::with_capacity(row.len());
            for c in row {
                let damage_of = match &c.damage_of {
                    Some(k) => Some(chip_handle(k, at)?),
                    None => None,
                };
                out.push((chip_handle(&c.chip, at)?, damage_of));
            }
            cross_special.push(out);
        }

        // The pack's navis, forms and stages.
        let key_of = |defined: &BTreeMap<String, u8>, n: u8, v1: String| -> String {
            defined.iter().find(|(_, m)| **m == n).map_or(v1, |(k, _)| k.clone())
        };
        // The identities, by key (the definitions' order). A navi's and a
        // form's name theirs: each knows whose it is.
        let mut identities = Vec::new();
        for d in definitions.of(Registry::Identity) {
            identities.push(super::identity::read(d, &content.assets)?);
        }
        if identities.windows(2).any(|w| w[0].key >= w[1].key) {
            return Err(ContentError::new("the identities are not in key order (the define phase sorts each registry)"));
        }
        let identity_of = |d: &Definition, identities: &[super::Identity]| -> Result<Option<bn6_content_api::IdentityHandle>, ContentError> {
            match d.spec.field("identity") {
                Data::Nil => Ok(None),
                Data::Ref(Registry::Identity, key) => {
                    let i = identities.binary_search_by(|i| i.key.as_str().cmp(key)).expect("a defined identity");
                    Ok(Some(bn6_content_api::IdentityHandle(i as u16)))
                }
                other => Err(ContentError::new(format!(
                    "{}.luau: {} {}: `identity` is {other:?}, not an identity (define.identity {{ ... }})",
                    d.module, d.registry, d.key
                ))),
            }
        };
        let navi_keys: BTreeMap<String, u8> = legacy.navis.iter().map(|(k, n)| (k.clone(), n.id)).collect();
        let form_keys: BTreeMap<String, u8> = legacy.forms.iter().map(|(k, f)| (k.clone(), f.id)).collect();
        let mut navis = Entries::new(Registry::Navi);
        for n in &content.navis {
            let key = key_of(&navi_keys, n.id, format!("v1/navi-{:02x}", n.id));
            let mut record = n.clone();
            // What its definition names by handle, and its fresh stats.
            if let Some(d) = definitions.get(Registry::Navi, &key) {
                record.weapons = read_weapons(d)?;
                record.fresh = super::navis::read_fresh(d)?;
                record.cross_hp = super::navis::read_cross_hp(d)?;
                record.identity = identity_of(d, &identities)?;
            }
            let own_chip = match &n.own_chip {
                Some(c) => Some((chip_handle(&c.chip, &format!("navi {key}'s own chip"))?, c.code)),
                None => None,
            };
            navis.add(key.clone(), NaviDef { key, record, own_chip }, "the pack's navi".into());
        }
        let navis: Vec<NaviDef> = navis.sorted()?.into_iter().map(|(_, n)| n).collect();
        // (Each navi's own chip knows its navi, and its identity whose it
        // is.)
        for (i, n) in navis.iter().enumerate() {
            if let Some((chip, _)) = n.own_chip {
                chips[chip.index()].links.own_chip_of = Some(NaviHandle(i as u16));
            }
            if let Some(h) = n.record.identity {
                claim_identity(&mut identities, h, super::IdentityOwner::Navi(NaviHandle(i as u16)), &n.key)?;
            }
        }
        let mut forms = Entries::new(Registry::Form);
        for f in &content.forms {
            let key = key_of(&form_keys, f.id, format!("v1/form-{:02x}", f.id));
            let mut record = f.clone();
            if let Some(d) = definitions.get(Registry::Form, &key) {
                record.weapons = read_weapons(d)?;
                record.identity = identity_of(d, &identities)?;
            }
            forms.add(key.clone(), FormDef { key, record }, "the pack's form".into());
        }
        let forms: Vec<FormDef> = forms.sorted()?.into_iter().map(|(_, f)| f).collect();
        for (i, f) in forms.iter().enumerate() {
            if let Some(h) = f.record.identity {
                claim_identity(&mut identities, h, super::IdentityOwner::Form(FormHandle(i as u16)), &f.key)?;
            }
        }
        // Stages: what they place names kinds and their variant records.
        let mut stages = Entries::new(Registry::Stage);
        for d in definitions.of(Registry::Stage) {
            let record = super::stages::read(
                d,
                &content.assets,
                |key| kinds.iter().position(|k| k.key == key).map(|i| KindHandle(i as u16)),
                |key| definitions.of(Registry::Record).iter().position(|r| r.key == key).map(|i| RecordHandle(i as u16)),
            )?;
            for e in &record.actors {
                if let super::Place::Kind(k) = e.place
                    && kinds[k.index()].place.is_none()
                {
                    return Err(ContentError::new(format!(
                        "{}.luau: stage {} places the kind {}, which has no `place`",
                        d.module,
                        d.key,
                        kinds[k.index()].key
                    )));
                }
            }
            stages.add(d.key.clone(), StageDef { key: d.key.clone(), record }, format!("defined in {}.luau", d.module));
        }
        let stages: Vec<StageDef> = stages.sorted()?.into_iter().map(|(_, s)| s).collect();

        // The lock-on modes, by key (the definitions' order).
        let mut lockons = Vec::new();
        for d in definitions.of(Registry::Lockon) {
            let what = |e: String| ContentError::new(format!("{}.luau: lockon {}: {e}", d.module, d.key));
            let mut spec = d.spec.clone();
            if let Data::Map(entries) = &mut spec {
                entries.retain(|(k, _)| !matches!(k, bn6_content_api::DataKey::Str(s) if s == "id"));
            }
            let mode: super::LockonMode = resolver.read(&spec, &d.key).map_err(what)?;
            lockons.push(LockonDef { key: d.key.clone(), mode });
        }
        if lockons.windows(2).any(|w| w[0].key >= w[1].key) {
            return Err(ContentError::new("the lock-on modes are not in key order (the define phase sorts each registry)"));
        }

        // The status effects, by key (the definitions' order).
        let mut statuses = Vec::new();
        for d in definitions.of(Registry::Status) {
            let what = |e: String| ContentError::new(format!("{}.luau: status {}: {e}", d.module, d.key));
            let mut spec = d.spec.clone();
            if let Data::Map(entries) = &mut spec {
                entries.retain(|(k, _)| !matches!(k, bn6_content_api::DataKey::Str(s) if s == "id"));
            }
            let effect: super::StatusEffect = resolver.read(&spec, &d.key).map_err(what)?;
            statuses.push(StatusDef { key: d.key.clone(), effect });
        }
        if statuses.windows(2).any(|w| w[0].key >= w[1].key) {
            return Err(ContentError::new("the statuses are not in key order (the define phase sorts each registry)"));
        }

        // Effects, sparks, regions and collision types, by handle.
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
        let mut regions: Vec<super::Region> = Vec::new();
        for d in definitions.of(Registry::Region) {
            let what = |e: &str| ContentError::new(format!("{}.luau: region {}: {e}", d.module, d.key));
            regions.push(match (d.spec.field("panels"), d.spec.field("field")) {
                (Data::List(items), Data::Nil) => {
                    let mut panels = Vec::new();
                    for p in items {
                        let (Some(dx), Some(dy)) = (p.item(1).int(), p.item(2).int()) else {
                            return Err(what("each of `panels` is { dx, dy }"));
                        };
                        panels.push(super::PanelOffset { dx: dx as i8, dy: dy as i8 });
                    }
                    super::Region::Panels(panels)
                }
                (Data::Nil, Data::Map(_)) => {
                    let word = |k: &str| d.spec.field("field").field(k).int().unwrap_or(0) as u32;
                    super::Region::Field(super::PanelCondition { require: word("require"), forbid: word("forbid") })
                }
                _ => return Err(what("needs exactly one of `panels` and `field`")),
            });
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

        // The roles.
        let mut roles = Roles::default();
        if let [d] = definitions.of(Registry::Roles) {
            roles = read_roles(d, &definitions, &content.assets, &actions, &kinds, &chips, &lockons, &statuses, &mut functions)?;
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
            navi_numbers: Vec::new(),
            form_numbers: Vec::new(),
            chip_keys: BTreeMap::new(),
            weapon_keys: BTreeMap::new(),
            kinds: Vec::new(),
            actions,
            weapons,
            chips,
            navis,
            forms,
            stages,
            lockons,
            identities,
            statuses,
            records,
            effects,
            sparks,
            regions,
            collisions,
            roles,
            program_advances,
            cross_special,
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
        }
        for (i, c) in defs.chips.iter().enumerate() {
            defs.chip_keys.insert(c.key.clone(), ChipHandle(i as u16));
        }
        defs.navi_numbers = vec![None; 256];
        for (i, n) in defs.navis.iter().enumerate() {
            defs.navi_numbers[n.record.id as usize] = Some(NaviHandle(i as u16));
        }
        defs.form_numbers = vec![None; 256];
        for (i, f) in defs.forms.iter().enumerate() {
            defs.form_numbers[f.record.id as usize] = Some(FormHandle(i as u16));
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
        // Every chip is a definition with its own use: none is a record
        // registration by number runs.
        assert!(c.defs.chips.len() > 300, "{} chips", c.defs.chips.len());
        // Effects, sparks, regions and collision types are definitions
        // the engine holds by handle.
        assert!(c.defs.effects.len() > 100, "{} effects defined", c.defs.effects.len());
        assert!(!c.defs.sparks.is_empty() && !c.defs.regions.is_empty() && c.defs.collisions.len() >= 89);
    }
}
