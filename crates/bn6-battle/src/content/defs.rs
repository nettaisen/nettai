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
    ChipClass, ChipCode, ChipData, ChipFamily, ChipFlags, ChipId, ChipModifier, Content, DIMMING_CHIP_ACTION, Element,
    ExtraChipFlags, FormData, INSTANT_CHIP_ACTION, NAVI_CHIP_ACTION, NaviData,
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
    /// The weapon routine number registration by number gives it (a
    /// `weapon.toml`'s, or the number of a routine nothing implements): the
    /// ruleset's numeric logic asks it until phase C. A weapon content
    /// defines has none.
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
    /// Its place in the pack's settings table.
    pub number: u8,
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
    pub effects: Vec<super::EffectSprite>,
    pub sparks: Vec<super::EffectSprite>,
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
/// fields the engine reads. Damage formulas, Program Advance recipes, dark
/// chips' substitutes and lock-on modes by definition come with the v2 API
/// (steps 4 and 10); a definition that gives one is refused.
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
    let library = spec.field("library");
    let lib_int = |field: &str| library.field(field).int().unwrap_or(0);
    Ok(ChipData {
        id: None,
        name: name_of("name")?.unwrap_or_else(|| d.key.clone()),
        codes,
        element,
        rarity: int("rarity", 0xFF)? as u8,
        family,
        class,
        mb: int("mb", 0xFF)? as u8,
        flags: ChipFlags(flags("flags", ChipFlags::NAMES)?),
        hit_param: int("hit_param", 0xFF)? as u8,
        action: 0,
        subtype: 0,
        beast_lockon,
        params: [0; 4],
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

        // Weapons.
        let mut weapons = Entries::new(Registry::Weapon);
        for w in &content.weapons {
            let whose = format!("weapon routine {:#04x} ({})", w.id, w.name);
            let setup = export(&definitions, &w.script, "setup", &whose)?;
            let def = WeaponDef {
                key: format!("v1/weapon-{:02x}", w.id),
                name: w.name.clone(),
                setup: Some(functions.id(setup)),
                number: Some(w.id),
                charge_ticks: Vec::new(),
            };
            weapons.add(def.key.clone(), def, whose);
        }
        // Every other routine number a navi's stats may name: a weapon
        // nothing implements yet (using it is the ruleset's error).
        for n in 0..=0xFEu8 {
            if !content.weapons.iter().any(|w| w.id == n) {
                let def = WeaponDef {
                    key: format!("v1/weapon-{n:02x}"),
                    name: String::new(),
                    setup: None,
                    number: Some(n),
                    charge_ticks: Vec::new(),
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
            let def = WeaponDef { key: d.key.clone(), name, setup, number: None, charge_ticks };
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
            chips.add(key.clone(), ChipDef { key, record: c.clone(), usage: None }, "the pack's chip record".into());
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
            chips.add(d.key.clone(), ChipDef { key: d.key.clone(), record, usage: Some(usage) }, format!("defined in {}.luau", d.module));
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

        let look = |d: &Definition| -> Result<super::EffectSprite, ContentError> {
            let what = |e: String| ContentError::new(format!("{}.luau: {} {}: {e}", d.module, d.registry, d.key));
            let sprite = d.spec.field("sprite").str().ok_or_else(|| what("needs a `sprite`".into()))?;
            let byte = |field: &str| -> Result<u8, ContentError> {
                match d.spec.field(field) {
                    Data::Nil => Ok(0),
                    Data::Int(i) => u8::try_from(*i).map_err(|_| what(format!("`{field}` {i} is not a byte"))),
                    _ => Err(what(format!("`{field}` is not a number"))),
                }
            };
            Ok(super::EffectSprite { sprite: sprite.parse().map_err(what)?, anim: byte("anim")?, palette: byte("palette")? })
        };
        let effects = definitions.of(Registry::Effect).iter().map(look).collect::<Result<Vec<_>, _>>()?;
        let sparks = definitions.of(Registry::Spark).iter().map(look).collect::<Result<Vec<_>, _>>()?;

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
            if let Some(id) = w.number {
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
            defs.stage_numbers[st.number as usize] = Some(StageHandle(i as u16));
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
        assert!(c.scripts.modules.len() > 200, "{} modules", c.scripts.modules.len());
        c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
        // The modules that return a table with a `state` give its layout.
        let states = c.defs.definitions.modules.iter().filter(|m| m.state.is_some()).count();
        assert!(states > 100, "{states} module states");
    }
}
