//! What the content defines (docs/design/content-model-v2.md): the
//! registries the engine reads, with their handles, and the functions the
//! runtime binds.
//!
//! A registry holds three sorts of entries while the migration runs:
//!
//! - the engine's own (its object kinds, keyed `engine/...`);
//! - content registered by number from the pack's data (an object folder's
//!   `[kind]`, a chip's `script`, a `weapon.toml`), keyed from that data
//!   (the kind's folder name; `v1/action-12`, `v1/weapon-02`);
//! - content's definitions (`define.kind { ... }`), keyed by their keys.
//!
//! Each registry's keys are sorted byte-wise; an entry's handle is its
//! place. Registration by number still reaches entries by the original's
//! numbers (an object slot, an action number, a weapon routine, a hook by
//! subtype): the *bridge*, which the migration's last step deletes.

use std::collections::BTreeMap;

use bn6_content_api::{
    ActionHandle, ChipHandle, ContentError, Data, Definition, Definitions, FnId, FnSource, Hook, KindHandle, Pool,
    RecordHandle, Registry, Schema, StateId, WeaponHandle,
};

use super::{ChipId, Content, DIMMING_CHIP_ACTION, INSTANT_CHIP_ACTION, NAVI_CHIP_ACTION};
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
    /// The object slot (pool and index) it fills, for registration by
    /// number and the traces (the bridge).
    pub slot: Option<(Pool, u8)>,
    /// Its position is register garbage until its init places it (the trace
    /// comparison skips it).
    pub scratch_position: bool,
    /// The fraction of its Z is register garbage (likewise).
    pub scratch_z_fraction: bool,
}

/// A navi action content implements.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionDef {
    pub key: String,
    pub update: FnId,
    /// Its state layout. Actions of one layout continue each other's
    /// attack state, as the original's actions share theirs.
    pub schema: StateId,
    /// Its action number (the navi's CurAction, which the traces compare;
    /// registration by number reaches it by it): the bridge.
    pub number: Option<u8>,
}

/// A weapon: what a button's weapon does.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WeaponDef {
    pub key: String,
    pub name: String,
    /// `setup(navi) -> action`.
    pub setup: FnId,
    /// The weapon routine numbers that name it (the bridge).
    pub ids: Vec<u8>,
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

/// A chip content defines.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChipDef {
    pub key: String,
    pub usage: ChipUsage,
    /// Its chip id, which the engine's hands and records still use (the
    /// bridge).
    pub id: Option<ChipId>,
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
    pub records: Vec<RecordDef>,
    /// State layouts by [`StateId`].
    pub schemas: Vec<SchemaDef>,
    /// The functions the runtime binds, by [`FnId`].
    pub functions: Vec<FnSource>,
    /// Registration by number: hooks by table and number (the bridge).
    pub hooks: BTreeMap<Hook, FnId>,
    kind_keys: BTreeMap<String, KindHandle>,
    /// Kinds by object slot: `pool * 256 + index` (the bridge).
    kind_slots: Vec<Option<KindHandle>>,
    /// Actions by number (the bridge).
    action_numbers: Vec<Option<ActionHandle>>,
    /// Weapons by routine number (the bridge).
    weapon_ids: Vec<Option<WeaponHandle>>,
    /// Chips content defines, by chip id (the bridge).
    chip_ids: BTreeMap<ChipId, ChipHandle>,
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

    /// The kind filling an object slot.
    pub fn kind_at(&self, pool: Pool, index: u8) -> Option<KindHandle> {
        self.kind_slots.get(pool_index(pool) * 256 + index as usize).copied().flatten()
    }

    pub fn action(&self, h: ActionHandle) -> &ActionDef {
        &self.actions[h.index()]
    }

    /// The action registered by number (or bridged to it).
    pub fn action_numbered(&self, number: u8) -> Option<ActionHandle> {
        self.action_numbers.get(number as usize).copied().flatten()
    }

    /// The weapon of a weapon routine number.
    pub fn weapon_numbered(&self, id: u8) -> Option<WeaponHandle> {
        self.weapon_ids.get(id as usize).copied().flatten()
    }

    /// A chip content defines, by its chip id.
    pub fn chip_with_id(&self, id: ChipId) -> Option<&ChipDef> {
        self.chip_ids.get(&id).map(|h| &self.chips[h.index()])
    }

    /// The function a hook by number runs: a weapon routine's `setup`, or
    /// what the pack's data registers for a subtype or an entry type.
    pub fn hook(&self, hook: Hook) -> Option<FnId> {
        match hook {
            Hook::Weapon(n) => self.weapon_numbered(n).map(|w| self.weapons[w.index()].setup),
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

    /// Give the kind `key` an object slot (for registration by number and
    /// the traces): how the bridge reaches a kind content defines.
    pub fn bridge_kind(&mut self, key: &str, slot: (Pool, u8)) -> Result<KindHandle, ContentError> {
        let h = self.kind_by_key(key).ok_or_else(|| ContentError::new(format!("no kind is named {key:?}")))?;
        let i = pool_index(slot.0) * 256 + slot.1 as usize;
        if let Some(other) = self.kind_slots[i]
            && other != h
        {
            return Err(ContentError::new(format!(
                "kinds {} and {key} both fill {} object {:#x}",
                self.kinds[other.index()].key,
                slot.0.name(),
                slot.1
            )));
        }
        self.kinds[h.index()].slot = Some(slot);
        self.kind_slots[i] = Some(h);
        Ok(h)
    }

    /// Give the weapon `key` a weapon routine number (the bridge).
    pub fn bridge_weapon(&mut self, key: &str, id: u8) -> Result<WeaponHandle, ContentError> {
        let i = self.weapons.binary_search_by(|w| w.key.as_str().cmp(key));
        let h = WeaponHandle(i.map_err(|_| ContentError::new(format!("no weapon is named {key:?}")))? as u16);
        if let Some(other) = self.weapon_ids[id as usize]
            && other != h
        {
            return Err(ContentError::new(format!(
                "weapons {} and {key} both are routine {id:#04x}",
                self.weapons[other.index()].key
            )));
        }
        self.weapons[h.index()].ids.push(id);
        self.weapon_ids[id as usize] = Some(h);
        Ok(h)
    }

    /// Give the chip `key` a chip id (the bridge): the engine's record with
    /// that id runs the definition's usage.
    pub fn bridge_chip(&mut self, key: &str, id: ChipId) -> Result<ChipHandle, ContentError> {
        let i = self.chips.binary_search_by(|c| c.key.as_str().cmp(key));
        let h = ChipHandle(i.map_err(|_| ContentError::new(format!("no chip is named {key:?}")))? as u16);
        if let Some(&other) = self.chip_ids.get(&id)
            && other != h
        {
            return Err(ContentError::new(format!("chips {} and {key} both are chip {id:#x}", self.chips[other.index()].key)));
        }
        self.chips[h.index()].id = Some(id);
        self.chip_ids.insert(id, h);
        Ok(h)
    }

    /// Give the action `key` an action number (the bridge).
    pub fn bridge_action(&mut self, key: &str, number: u8) -> Result<ActionHandle, ContentError> {
        let i = self.actions.binary_search_by(|a| a.key.as_str().cmp(key));
        let h = ActionHandle(i.map_err(|_| ContentError::new(format!("no action is named {key:?}")))? as u16);
        self.actions[h.index()].number = Some(number);
        // Several actions can share a number (the chips of one family each
        // compose their own); registration by number reaches the first.
        let slot = &mut self.action_numbers[number as usize];
        slot.get_or_insert(h);
        Ok(h)
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
                scratch_position: false,
                scratch_z_fraction: false,
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
                scratch_position: k.scratch_position,
                scratch_z_fraction: k.scratch_z_fraction,
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
                scratch_position: false,
                scratch_z_fraction: false,
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
            let whose = format!("chip {:#05x} ({})", c.id, c.name);
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
            let def = WeaponDef { key: format!("v1/weapon-{:02x}", w.id), name: w.name.clone(), setup: functions.id(setup), ids: vec![w.id] };
            weapons.add(def.key.clone(), def, whose);
        }
        for d in definitions.of(Registry::Weapon) {
            let name = d.spec.field("name").str().unwrap_or(&d.key).to_string();
            let def = WeaponDef { key: d.key.clone(), name, setup: functions.id(slot(d, "setup")?), ids: Vec::new() };
            weapons.add(d.key.clone(), def, format!("defined in {}.luau", d.module));
        }
        let weapons: Vec<WeaponDef> = weapons.sorted()?.into_iter().map(|(_, w)| w).collect();

        // Chips content defines (their records join the engine's once
        // compat gives them their ids).
        let action_handle = |key: &str| -> Option<ActionHandle> {
            actions.binary_search_by(|a| a.key.as_str().cmp(key)).ok().map(|i| ActionHandle(i as u16))
        };
        let mut chips = Vec::new();
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
            chips.push(ChipDef { key: d.key.clone(), usage, id: None });
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
                Registry::Schema => schema_id(&d.key).0,
                _ => in_registry,
            });
            in_registry += 1;
        }

        // The bridge from numbers.
        let mut defs = Defs {
            defined: true,
            definitions,
            handles,
            kind_keys: kinds.iter().enumerate().map(|(i, k)| (k.key.clone(), KindHandle(i as u16))).collect(),
            kind_slots: vec![None; 3 * 256],
            action_numbers: vec![None; 256],
            weapon_ids: vec![None; 256],
            chip_ids: BTreeMap::new(),
            kinds: Vec::new(),
            actions,
            weapons,
            chips,
            records,
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
            for &id in &w.ids {
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
