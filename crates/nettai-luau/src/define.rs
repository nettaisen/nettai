//! The define phase (docs/design/content-model-v2.md §7.3): while a pack's
//! modules load, `define.<registry>(spec)` records definitions; when every
//! module has loaded, the definitions get their keys, the `state` tables of
//! kinds, actions and modules become schemas, and everything is read back as
//! plain data (the canonical tree, [`Definitions`]).
//!
//! Keys (§2.2), each qualified with the root of the module that made the
//! definition (`bn6:minibomb`; docs/design/rules-in-luau.md §7.2):
//!
//! - an explicit `id` (required for the registries named from outside
//!   content, [`Registry::keyed`]);
//! - else, for a definition made while the same module loads as a keyed
//!   definition that holds it, the owner's key and the field path
//!   (`minibomb/action`, `minibomb/action/args/thrown`); owners are walked in
//!   registry and key order, their fields in key order;
//! - else `<module>#<n>`, its place among the module's definitions.
//!
//! Definers work only while modules load; afterwards every definition is
//! frozen and the definers fail.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::c_void;
use std::rc::Rc;

use nettai_content_api::{AssetKind, AssetNames, Data, DataKey, Definition, Definitions, Registry, keys, valid_key};
use mlua::{Lua, Table, Value as LuaValue};

/// A definition as a definer recorded it.
pub(crate) struct Made {
    pub registry: Registry,
    /// The module loading when it was made.
    pub module: String,
    /// Its place among that module's definitions, from 1.
    pub ordinal: u32,
    pub record_type: Option<String>,
    pub table: Table,
}

/// The define phase's running record.
#[derive(Default)]
pub(crate) struct Collector {
    /// Definers work only while this is set.
    pub open: bool,
    pub made: Vec<Made>,
    ordinals: HashMap<String, u32>,
}

impl Collector {
    /// A collector for a define phase that is starting.
    pub fn open() -> Collector {
        Collector { open: true, ..Default::default() }
    }
}

/// What the define phase read, with the runtime's tables.
pub(crate) struct Defined {
    pub definitions: Definitions,
    /// Each definition's table, in `definitions.defs`' order.
    pub tables: Vec<Table>,
}

type Ptr = *const c_void;

/// Asset values (docs/design/content-model-v2.md §6.3): one frozen table
/// per asset, `{ name = "bomb" }` with its kind's metatable, made when
/// content first names the asset. The binding finds an asset by its table's
/// identity, as it finds a definition.
pub(crate) struct AssetTables {
    pub names: AssetNames,
    metas: Vec<(AssetKind, Table)>,
    pub by_ptr: HashMap<Ptr, (AssetKind, u16)>,
    by_handle: HashMap<(AssetKind, u16), Table>,
}

impl AssetTables {
    pub fn new(lua: &Lua, names: AssetNames) -> mlua::Result<AssetTables> {
        let mut metas = Vec::new();
        for kind in AssetKind::ALL {
            let meta = lua.create_table()?;
            meta.raw_set("__metatable", format!("{kind} asset"))?;
            meta.raw_set("__asset", kind.name())?;
            meta.set_readonly(true);
            metas.push((kind, meta));
        }
        Ok(AssetTables { names, metas, by_ptr: HashMap::new(), by_handle: HashMap::new() })
    }

    /// The value of asset `h` of `kind`.
    pub fn value(&mut self, lua: &Lua, kind: AssetKind, h: u16) -> mlua::Result<Table> {
        if let Some(t) = self.by_handle.get(&(kind, h)) {
            return Ok(t.clone());
        }
        let name = self.names.names(kind).get(h as usize).map(|s| s.to_string()).ok_or_else(|| {
            mlua::Error::runtime(format!("no {kind} has handle {h}"))
        })?;
        let t = lua.create_table()?;
        t.raw_set("name", name)?;
        let meta = self.metas.iter().find(|(k, _)| *k == kind).map(|(_, m)| m.clone()).expect("every kind");
        t.set_metatable(Some(meta))?;
        t.set_readonly(true);
        self.by_ptr.insert(t.to_pointer(), (kind, h));
        self.by_handle.insert((kind, h), t.clone());
        Ok(t)
    }

    /// The asset `v` is, if it is one.
    pub fn asset(&self, v: &LuaValue) -> Option<(AssetKind, u16)> {
        match v {
            LuaValue::Table(t) => self.by_ptr.get(&t.to_pointer()).copied(),
            _ => None,
        }
    }
}

/// Install `asset`: a resolver per asset kind (`asset.sprite("bomb")`),
/// working while content loads. An unknown name is an error naming the
/// module.
pub(crate) fn install_assets(
    lua: &Lua,
    tables: &Rc<RefCell<AssetTables>>,
    module: Rc<dyn Fn() -> Option<String>>,
) -> mlua::Result<()> {
    let asset = lua.create_table()?;
    for kind in AssetKind::ALL {
        let tables = Rc::downgrade(tables);
        let module = module.clone();

        let f = lua.create_function(move |lua, name: LuaValue| {
            let what = format!("asset.{kind}");
            let at = module().ok_or_else(|| mlua::Error::runtime(format!("{what}: assets are named while content loads")))?;
            let tables = tables.upgrade().ok_or_else(|| mlua::Error::runtime(format!("{what}: content has loaded")))?;
            let LuaValue::String(name) = name else {
                return Err(mlua::Error::runtime(format!("{at}: {what} takes a name, not {}", name.type_name())));
            };
            let name = name.to_str()?.to_string();
            // (Written in full: `bn6:bomb`.)
            let qualified = crate::Pack::asset_name(&name).map_err(|e| mlua::Error::runtime(format!("{at}: {what}: {e}")))?;
            let h = tables.borrow().names.handle(kind, &qualified).ok_or_else(|| {
                mlua::Error::runtime(format!("{at}: no {kind} is named {name:?} (the pack {})", keys::root_of(&qualified).unwrap_or("")))
            })?;
            tables.borrow_mut().value(lua, kind, h)
        })?;
        asset.raw_set(kind.name(), f)?;
    }
    asset.set_readonly(true);
    lua.globals().set("asset", asset)?;
    Ok(())
}

/// Install `define`: a definer per registry, recording into `collector`.
/// `module` names the module loading at the moment of a call (None outside
/// loading).
pub(crate) fn install(
    lua: &Lua,
    collector: &Rc<RefCell<Collector>>,
    module: Rc<dyn Fn() -> Option<String>>,
) -> mlua::Result<()> {
    let define = lua.create_table()?;
    for registry in Registry::DEFINED {
        let meta = lua.create_table()?;
        meta.raw_set("__registry", registry.name())?;
        meta.raw_set("__metatable", format!("{} definition", registry.name()))?;
        let collector = Rc::downgrade(collector);
        let module = module.clone();
        let record = move |record_type: Option<String>, spec: LuaValue| -> mlua::Result<Table> {
            let what = format!("define.{}", registry.name());
            let collector = collector.upgrade().filter(|c| c.borrow().open).ok_or_else(|| {
                mlua::Error::runtime(format!("{what}: definitions are made while content loads"))
            })?;
            let at = module().ok_or_else(|| mlua::Error::runtime(format!("{what}: called outside a module")))?;
            let LuaValue::Table(table) = spec else {
                return Err(mlua::Error::runtime(format!("{at}: {what} takes a table, not {}", spec.type_name())));
            };
            if table.metatable().is_some() {
                return Err(mlua::Error::runtime(format!(
                    "{at}: {what}'s spec already has a metatable (is it defined twice?)"
                )));
            }
            // (The migration's `legacy` markers carried the original's
            // numbers. A definition says what it is by name; the numbers
            // are the validator's, outside the engine.)
            if !table.raw_get::<LuaValue>("legacy")?.is_nil() {
                return Err(mlua::Error::runtime(format!(
                    "{at}: {what} takes no `legacy` field: a definition is named by its key, not by the original's numbers"
                )));
            }
            table.set_metatable(Some(meta.clone()))?;
            let mut c = collector.borrow_mut();
            let n = c.ordinals.entry(at.clone()).or_default();
            *n += 1;
            let ordinal = *n;
            c.made.push(Made { registry, module: at, ordinal, record_type, table: table.clone() });
            Ok(table)
        };
        let f = if matches!(registry, Registry::Record | Registry::Rules) {
            // `define.record(type, spec)`, `define.rules(section, spec)`:
            // the string is the record's type, or the section's name (its
            // key).
            let what = if registry == Registry::Record { "define.record(type, spec): the type" } else { "define.rules(section, spec): the section" };
            lua.create_function(move |_, (name, spec): (LuaValue, LuaValue)| {
                let LuaValue::String(t) = name else {
                    return Err(mlua::Error::runtime(format!("{what} is a string")));
                };
                let t = t.to_str()?.to_string();
                record(Some(t), spec)
            })?
        } else {
            lua.create_function(move |_, spec: LuaValue| record(None, spec))?
        };
        define.raw_set(registry.name(), f)?;
    }
    define.set_readonly(true);
    lua.globals().set("define", define)?;
    Ok(())
}

/// A table's fields, in canonical order (integer keys first, then strings),
/// as (key, path segment, value).
fn fields(t: &Table, at: &str) -> Result<Vec<(DataKey, LuaValue)>, String> {
    let mut out = Vec::new();
    for pair in t.clone().pairs::<LuaValue, LuaValue>() {
        let (k, v) = pair.map_err(|e| format!("{at}: {e}"))?;
        let k = match k {
            LuaValue::String(s) => {
                DataKey::Str(s.to_str().map_err(|e| format!("{at}: a key isn't UTF-8 ({e})"))?.to_string())
            }
            LuaValue::Integer(i) => DataKey::Int(i),
            LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9e15 => DataKey::Int(n as i64),
            k => return Err(format!("{at}: a table key is a {}, not a string or an integer", k.type_name())),
        };
        out.push((k, v));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Finish the define phase: keys, schemas, the canonical tree; every
/// definition frozen.
pub(crate) fn finish(
    lua: &Lua,
    collector: &RefCell<Collector>,
    assets: &AssetTables,
) -> Result<Defined, String> {
    let mut c = collector.borrow_mut();
    c.open = false;
    let made = std::mem::take(&mut c.made);
    drop(c);

    let index: HashMap<Ptr, usize> = made.iter().enumerate().map(|(i, m)| (m.table.to_pointer(), i)).collect();
    let mut keys: Vec<Option<String>> = vec![None; made.len()];

    // Explicit ids, written in full (docs/design/rules-in-luau.md, the
    // flat namespace: "in the chip ids directly have bn6:cannon"): the
    // game, then the key (`bn6:cannon`, `bn6:eraseman/mark`); a section's
    // name too (`bn6:panels`), and a game's roles' (`bn6:roles`).
    let full = |what: &str, id: &str, kind: &str, folder: &str| -> Result<String, String> {
        let local = match keys::root_of(id) {
            Some(game) if keys::valid_root_name(game) => keys::local(id),
            _ => return Err(format!("{what}: {kind} {id:?} names no game: write it in full (\"{folder}:{id}\")")),
        };
        if !valid_key(local) {
            return Err(format!("{what}: {id:?} is not a valid {kind} (a game, then lowercase words in -, qualified with /: \"bn6:eraseman/mark\")"));
        }
        Ok(id.to_string())
    };
    // (Every id not in full is reported at once.)
    let mut unwritten = Vec::new();
    for (i, m) in made.iter().enumerate() {
        let what = format!("{}: define.{}", m.module, m.registry.name());
        let folder = keys::root_of(&m.module).unwrap_or("bn6");
        if m.registry == Registry::Rules {
            let name = m.record_type.clone().unwrap_or_default();
            match full(&what, &name, "section name", folder) {
                Ok(k) => keys[i] = Some(k),
                Err(e) => unwritten.push(e),
            }
            continue;
        }
        match m.table.raw_get::<LuaValue>("id").map_err(|e| format!("{what}: {e}"))? {
            LuaValue::Nil if m.registry.keyed() || m.registry == Registry::Roles => return Err(format!("{what} needs an `id`")),
            LuaValue::Nil => {}
            LuaValue::String(s) => {
                let id = s.to_str().map_err(|e| format!("{what}: {e}"))?.to_string();
                match full(&what, &id, "id", folder) {
                    Ok(k) => keys[i] = Some(k),
                    Err(e) => unwritten.push(e),
                }
            }
            v => return Err(format!("{what}: `id` is a {}, not a string", v.type_name())),
        }
    }
    if !unwritten.is_empty() {
        return Err(unwritten.join("\n"));
    }

    // Keys from owners: definitions nested in a keyed one, made while the
    // same module loaded.
    fn walk(
        t: &Table,
        prefix: &str,
        module: &str,
        made: &[Made],
        index: &HashMap<Ptr, usize>,
        keys: &mut [Option<String>],
        seen: &mut HashSet<Ptr>,
    ) -> Result<(), String> {
        if !seen.insert(t.to_pointer()) {
            return Ok(());
        }
        for (k, v) in fields(t, prefix)? {
            let LuaValue::Table(sub) = v else { continue };
            let path = format!("{prefix}/{k}");
            match index.get(&sub.to_pointer()) {
                Some(&j) => {
                    if keys[j].is_none() && made[j].module == module {
                        keys[j] = Some(path.clone());
                        walk(&sub, &path, module, made, index, keys, seen)?;
                    }
                }
                None => walk(&sub, &path, module, made, index, keys, seen)?,
            }
        }
        Ok(())
    }
    let mut owners: Vec<usize> = (0..made.len()).filter(|&i| keys[i].is_some()).collect();
    owners.sort_by(|&a, &b| (made[a].registry, &keys[a]).cmp(&(made[b].registry, &keys[b])));
    let mut seen = HashSet::new();
    for i in owners {
        let key = keys[i].clone().expect("an owner has a key");
        walk(&made[i].table, &key, &made[i].module, &made, &index, &mut keys, &mut seen)?;
    }
    let keys: Vec<String> = keys
        .into_iter()
        .zip(&made)
        .map(|(k, m)| k.unwrap_or_else(|| format!("{}#{}", m.module, m.ordinal)))
        .collect();
    let mut by_key: BTreeMap<(Registry, &str), usize> = BTreeMap::new();
    for (i, m) in made.iter().enumerate() {
        if let Some(&j) = by_key.get(&(m.registry, keys[i].as_str())) {
            return Err(format!(
                "{} {:?} is defined twice: in {}.luau and in {}.luau",
                m.registry.name(),
                keys[i],
                made[j].module,
                m.module
            ));
        }
        by_key.insert((m.registry, keys[i].as_str()), i);
    }

    // Schemas: the `state` tables of kinds, actions and systems, and the
    // `setup` tables of systems.
    let mut schema_of: HashMap<Ptr, usize> = HashMap::new();
    let mut schemas: Vec<(String, String, Table)> = Vec::new();
    let mut claim = |t: Table, key: String, module: &str, schemas: &mut Vec<(String, String, Table)>| {
        if index.contains_key(&t.to_pointer()) {
            return;
        }
        schema_of.entry(t.to_pointer()).or_insert_with(|| {
            schemas.push((key, module.to_string(), t));
            schemas.len() - 1
        });
    };
    for (&(registry, key), &i) in &by_key {
        let tables: &[&str] = match registry {
            Registry::Kind | Registry::Action => &["state"],
            Registry::System => &["state", "setup"],
            _ => continue,
        };
        for &field in tables {
            if let Ok(LuaValue::Table(state)) = made[i].table.raw_get::<LuaValue>(field) {
                claim(state, format!("{}:{key}/{field}", registry.name()), &made[i].module, &mut schemas);
            }
        }
    }
    let schema_keys: HashMap<Ptr, String> =
        schema_of.iter().map(|(&p, &i)| (p, schemas[i].0.clone())).collect();
    let def_keys: HashMap<Ptr, (Registry, String)> =
        made.iter().enumerate().map(|(i, m)| (m.table.to_pointer(), (m.registry, keys[i].clone()))).collect();

    // The canonical tree.
    let refs = Refs { defs: &def_keys, schemas: &schema_keys, assets };
    let mut defs = Vec::with_capacity(made.len() + schemas.len());
    let mut tables = Vec::with_capacity(made.len() + schemas.len());
    for (&(registry, key), &i) in &by_key {
        let m = &made[i];
        let at = format!("{} {key}", registry.name());
        let spec = refs.data(&LuaValue::Table(m.table.clone()), &at, true, &mut Vec::new())?;
        defs.push(Definition {
            registry,
            key: key.to_string(),
            module: m.module.clone(),
            record_type: m.record_type.clone().filter(|_| registry == Registry::Record),
            spec,
        });
        tables.push(m.table.clone());
    }
    let mut schema_order: Vec<usize> = (0..schemas.len()).collect();
    schema_order.sort_by(|&a, &b| schemas[a].0.cmp(&schemas[b].0));
    for i in schema_order {
        let (key, module, t) = &schemas[i];
        let at = format!("schema {key}");
        let spec = refs.data(&LuaValue::Table(t.clone()), &at, true, &mut Vec::new())?;
        defs.push(Definition { registry: Registry::Schema, key: key.clone(), module: module.clone(), record_type: None, spec });
        tables.push(t.clone());
    }
    // Registries sort in declaration order, Schema last: already in order.
    debug_assert!(defs.windows(2).all(|w| (w[0].registry, &w[0].key) < (w[1].registry, &w[1].key)));

    for t in &tables {
        crate::sandbox::deep_freeze(lua, &LuaValue::Table(t.clone())).map_err(|e| e.to_string())?;
    }
    Ok(Defined { definitions: Definitions { defs }, tables })
}

/// How definitions and schemas appear inside other definitions.
struct Refs<'a> {
    defs: &'a HashMap<Ptr, (Registry, String)>,
    schemas: &'a HashMap<Ptr, String>,
    assets: &'a AssetTables,
}

impl Refs<'_> {
    /// A value as data. `top`: the definition's own table (not a reference
    /// to it).
    fn data(&self, v: &LuaValue, at: &str, top: bool, path: &mut Vec<Ptr>) -> Result<Data, String> {
        Ok(match v {
            LuaValue::Nil => Data::Nil,
            LuaValue::Boolean(b) => Data::Bool(*b),
            LuaValue::Integer(i) => Data::Int(*i),
            LuaValue::Number(n) => {
                if n.fract() != 0.0 || !n.is_finite() || n.abs() >= 9_007_199_254_740_992.0 {
                    return Err(format!("{at}: {n} is not an integer"));
                }
                Data::Int(*n as i64)
            }
            LuaValue::String(s) => Data::Str(s.to_str().map_err(|e| format!("{at}: {e}"))?.to_string()),
            LuaValue::Function(_) => Data::Function,
            LuaValue::Table(t) => {
                let p = t.to_pointer();
                if let Some(&(kind, h)) = self.assets.by_ptr.get(&p) {
                    let name = self.assets.names.names(kind)[h as usize].to_string();
                    return Ok(Data::Asset(kind, name));
                }
                if !top {
                    if let Some((registry, key)) = self.defs.get(&p) {
                        return Ok(Data::Ref(*registry, key.clone()));
                    }
                    if let Some(key) = self.schemas.get(&p) {
                        return Ok(Data::Ref(Registry::Schema, key.clone()));
                    }
                }
                if path.contains(&p) {
                    return Err(format!("{at}: a table contains itself"));
                }
                path.push(p);
                let entries = fields(t, at)?;
                let list = entries.iter().enumerate().all(|(i, (k, _))| *k == DataKey::Int(i as i64 + 1));
                let d = if list && !entries.is_empty() {
                    let mut items = Vec::with_capacity(entries.len());
                    for (k, v) in &entries {
                        items.push(self.data(v, &format!("{at}.{k}"), false, path)?);
                    }
                    Data::List(items)
                } else {
                    let mut items = Vec::with_capacity(entries.len());
                    for (k, v) in entries {
                        let d = self.data(&v, &format!("{at}.{k}"), false, path)?;
                        items.push((k, d));
                    }
                    Data::Map(items)
                };
                path.pop();
                d
            }
            v => return Err(format!("{at}: a {} can't be part of a definition", v.type_name())),
        })
    }
}
