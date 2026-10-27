//! The define phase (docs/design/content-model-v2.md §7.3): a game's top
//! module returns its root, one table whose sections hold what a match
//! names, keyed by id (the core's: `chips`, `navis`, `forms`, `stages`; and
//! the game's own collections, any other key: EXE6's `patch_cards`,
//! `navicust_programs`, whose entries are data, `Registry::Entry`, keyed
//! `<collection>/<id>`), and its `rules`. Loading has no side effect: the
//! loader walks from the root, through tables and through what functions
//! capture, and what it reaches is what the game has. Everything else is a
//! table only code reaches (a kind a chip's action spawns, an effect a
//! kind shows), which says what it is by a tag constructor (`new.kind {
//! ... }`, `new.record("bomb-variant", { ... })`: a metatable, no registry,
//! no id). When the walk is done, the `state` tables of kinds, actions and
//! the rules become schemas, and everything is read back as plain data (the
//! canonical tree, [`Definitions`]).
//!
//! Keys (§2.2), local to the game:
//!
//! - a root section's: its id there; a collection's entry's,
//!   `<collection>/<id>`; the rules', `rules` (`RULESET_KEY`);
//! - a tagged table's: its `id`, where it states one (what a compat map
//!   names it by); else, where a module's result holds it through plain
//!   tables, the module's name and the fields (`rules/collision/attack`,
//!   `exelib:regions/area/0`: a shared table, by where it is written); else
//!   its path from the nearest definition that holds it, by that
//!   definition's key (`minibomb/action`, `minibomb/action/args/thrower`;
//!   through a function, the name it captures the value by:
//!   `bomb/update/EXPLOSION`). Paths are breadth first (from the root, and
//!   from the modules' results in name order), a table's fields in key
//!   order: the shortest, then the first.
//!
//! A load's other entries (a test's: modules alone, or a game held in
//! memory's modules besides its top module) are walked loose: each
//! module's result, its key path starting at the module's name, unless the
//! result's keys are sections, a root.
//!
//! Tag constructors work only while modules load; afterwards every table is
//! frozen and they fail.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::c_void;
use std::rc::Rc;

use nettai_content_api::{AssetKind, AssetNames, Data, DataKey, DefinedAssets, Definition, Definitions, Registry, SpriteDef, keys, valid_key};
use mlua::{Lua, Table, Value as LuaValue};

/// What a tag constructor says of a table: its type, and where it was
/// made (for messages). Not a definition: the walk decides what exists.
pub(crate) struct Tag {
    pub registry: Registry,
    pub record_type: Option<String>,
    pub module: String,
    /// (Held, so a table nothing reaches isn't collected and its address
    /// reused while loading goes on.)
    _table: Table,
}

/// The tags made while content loads, by table.
#[derive(Default)]
pub(crate) struct Tags {
    /// Tag constructors work only while this is set.
    pub open: bool,
    by_table: HashMap<Ptr, Tag>,
}

impl Tags {
    /// Tags for a load that is starting.
    pub fn open() -> Tags {
        Tags { open: true, ..Default::default() }
    }
}

/// The registries a root's sections hold, by the section's name
/// (`packs::SECTIONS`' names): what a match names, by id. (A game's own
/// collections, its root's other keys, hold entries: `Registry::Entry`.)
pub const SECTIONS: [(&str, Registry); 4] =
    [("chips", Registry::Chip), ("navis", Registry::Navi), ("forms", Registry::Form), ("stages", Registry::Stage)];

/// The root's field the game's rules are.
pub const RULES_SECTION: &str = nettai_content_api::packs::RULES;

/// The registries whose tables only code reaches, tagged by
/// `new.<registry>`.
pub const TAGGED: [Registry; 10] = [
    Registry::Weapon,
    Registry::Kind,
    Registry::Action,
    Registry::Effect,
    Registry::Spark,
    Registry::Region,
    Registry::Collision,
    Registry::Status,
    Registry::Identity,
    Registry::Record,
];

/// What the define phase read, with the runtime's tables.
pub(crate) struct Defined {
    pub definitions: Definitions,
    /// Each definition's table, in `definitions.defs`' order.
    pub tables: Vec<Table>,
}

type Ptr = *const c_void;

/// Asset values (docs/design/content-model-v2.md §6.3): one frozen table
/// per asset, `{ name = "bomb" }` with its kind's metatable, made when
/// content first names the asset (a sprite: where its module defines it).
/// The binding finds an asset by its table's identity, as it finds a
/// definition. The handles are made once loading is done ([`AssetTables::finish`]):
/// each kind's names in byte order, whatever order the modules named them in.
pub(crate) struct AssetTables {
    metas: Vec<(AssetKind, Table)>,
    /// Each asset content has named, by kind and name.
    named: BTreeMap<(AssetKind, String), Table>,
    /// The sprites content defines, by name: the module that defines each,
    /// and its definition.
    sprites: BTreeMap<String, (String, SpriteDef)>,
    /// The names, once loading is done: their places are the handles.
    pub names: AssetNames,
    pub by_ptr: HashMap<Ptr, (AssetKind, u16)>,
    by_handle: HashMap<(AssetKind, u16), Table>,
}

impl AssetTables {
    pub fn new(lua: &Lua) -> mlua::Result<AssetTables> {
        let mut metas = Vec::new();
        for kind in AssetKind::ALL {
            let meta = lua.create_table()?;
            meta.raw_set("__metatable", format!("{kind} asset"))?;
            meta.raw_set("__asset", kind.name())?;
            meta.set_readonly(true);
            metas.push((kind, meta));
        }
        Ok(AssetTables {
            metas,
            named: BTreeMap::new(),
            sprites: BTreeMap::new(),
            names: AssetNames::default(),
            by_ptr: HashMap::new(),
            by_handle: HashMap::new(),
        })
    }

    /// The value of `kind` asset `name`, made the first time content names it.
    fn named(&mut self, lua: &Lua, kind: AssetKind, name: &str) -> mlua::Result<Table> {
        if let Some(t) = self.named.get(&(kind, name.to_string())) {
            return Ok(t.clone());
        }
        let t = lua.create_table()?;
        t.raw_set("name", name)?;
        let meta = self.metas.iter().find(|(k, _)| *k == kind).map(|(_, m)| m.clone()).expect("every kind");
        t.set_metatable(Some(meta))?;
        t.set_readonly(true);
        self.named.insert((kind, name.to_string()), t.clone());
        Ok(t)
    }

    /// Sprite `name` as module `module` defines it: its value. A sprite is
    /// defined once.
    fn define_sprite(&mut self, lua: &Lua, module: &str, name: &str, def: SpriteDef) -> mlua::Result<Table> {
        if let Some((other, _)) = self.sprites.get(name) {
            return Err(mlua::Error::runtime(format!(
                "{module}: sprite {name:?} is defined twice: in {}.luau and in {}.luau (a sprite is defined once, by its module: require it)",
                keys::module_path(other),
                keys::module_path(module)
            )));
        }
        self.sprites.insert(name.to_string(), (module.to_string(), def));
        self.named(lua, AssetKind::Sprite, name)
    }

    /// Loading is done: every name content gave, each kind's in byte order,
    /// are the handles.
    pub fn finish(&mut self) {
        self.names = AssetNames::of(self.named.keys().cloned());
        for ((kind, name), t) in &self.named {
            let h = self.names.handle(*kind, name).expect("a name just interned");
            self.by_ptr.insert(t.to_pointer(), (*kind, h));
            self.by_handle.insert((*kind, h), t.clone());
        }
    }

    /// What the define phase made of the assets: their names and the
    /// sprites' definitions, by handle. (Every sprite named is defined: a
    /// sprite is named only by its definition.)
    pub fn defined(&self) -> DefinedAssets {
        let sprites = self.names.sprites.iter().map(|n| self.sprites[n].1.clone()).collect();
        DefinedAssets { names: self.names.clone(), sprites }
    }

    /// The value of asset `h` of `kind`.
    pub fn value(&self, kind: AssetKind, h: u16) -> mlua::Result<Table> {
        self.by_handle.get(&(kind, h)).cloned().ok_or_else(|| mlua::Error::runtime(format!("no {kind} has handle {h}")))
    }

    /// The asset `v` is, if it is one.
    pub fn asset(&self, v: &LuaValue) -> Option<(AssetKind, u16)> {
        match v {
            LuaValue::Table(t) => self.by_ptr.get(&t.to_pointer()).copied(),
            _ => None,
        }
    }
}

/// A sprite's definition, as content writes it (`asset.sprite(name,
/// animations, { parts = ... })`, core.d.luau's `SpriteAnimation` and
/// `SpriteParts`): its animations, each its frames' durations in ticks (1
/// to 255) with `loop = true` where it loops; and the parts content reads,
/// by part number, each animation's frames' `{ x, y }`, as many as the
/// animations' frames.
fn sprite_def(at: &str, animations: &LuaValue, extra: &LuaValue) -> Result<SpriteDef, String> {
    let int = |v: &LuaValue, what: &str, lo: i64, hi: i64| -> Result<i64, String> {
        let n = match v {
            LuaValue::Integer(i) => i64::from(*i),
            LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => *n as i64,
            v => return Err(format!("{at}: {what} is a {}, not an integer", v.type_name())),
        };
        if !(lo..=hi).contains(&n) {
            return Err(format!("{at}: {what} is {n}, not {lo} to {hi}"));
        }
        Ok(n)
    };
    let LuaValue::Table(list) = animations else {
        return Err(format!(
            "{at}: asset.sprite(name, animations) takes the sprite's animations (a list, each its frames' durations), not {} (a sprite is defined once, by its module: require it)",
            animations.type_name()
        ));
    };
    let mut def = SpriteDef::default();
    for (k, v) in fields(list, at)? {
        if k != DataKey::Int(def.animations.len() as i64 + 1) {
            return Err(format!("{at}: the animations are a list (1, 2, ...), not keyed {k}"));
        }
        let what = format!("animation {}", def.animations.len());
        let LuaValue::Table(anim) = v else { return Err(format!("{at}: {what} is a {}, not a table of durations", v.type_name())) };
        let mut a = nettai_content_api::SpriteAnimation::default();
        for (k, v) in fields(&anim, at)? {
            match k {
                DataKey::Int(i) if i == a.frames.len() as i64 + 1 => {
                    a.frames.push(int(&v, &format!("{what}'s frame {}", a.frames.len()), 1, 255)? as u8)
                }
                DataKey::Str(s) if s == "loop" => match v {
                    LuaValue::Boolean(b) => a.looping = b,
                    v => return Err(format!("{at}: {what}'s `loop` is a {}, not a boolean", v.type_name())),
                },
                k => return Err(format!("{at}: {what} holds its frames' durations (1, 2, ...) and `loop`, not {k}")),
            }
        }
        if a.frames.is_empty() {
            return Err(format!("{at}: {what} has no frame"));
        }
        def.animations.push(a);
    }
    match extra {
        LuaValue::Nil => {}
        LuaValue::Table(t) => {
            for (k, v) in fields(t, at)? {
                if k != DataKey::Str("parts".into()) {
                    return Err(format!("{at}: a sprite's third argument holds its `parts`, not {k}"));
                }
                let LuaValue::Table(parts) = v else { return Err(format!("{at}: `parts` is a {}, not a table", v.type_name())) };
                for (k, v) in fields(&parts, at)? {
                    let DataKey::Int(n) = k else { return Err(format!("{at}: `parts` is keyed by part number, not {k}")) };
                    let n = int(&LuaValue::Integer(n as _), "a part's number", 0, 31)? as u8;
                    let what = format!("part {n}");
                    let LuaValue::Table(by_anim) = v else { return Err(format!("{at}: {what} is a {}, not a table", v.type_name())) };
                    let mut offsets = Vec::new();
                    for (i, (k, v)) in fields(&by_anim, at)?.into_iter().enumerate() {
                        let frames = def.animations.get(i).map_or(0, |a| a.frames.len());
                        let what = format!("{what}, animation {i}");
                        let LuaValue::Table(by_frame) = v else { return Err(format!("{at}: {what} is a {}, not a table", v.type_name())) };
                        if k != DataKey::Int(i as i64 + 1) {
                            return Err(format!("{at}: {what}: a part's offsets are a list by animation"));
                        }
                        let mut anim = Vec::new();
                        for (j, (k, v)) in fields(&by_frame, at)?.into_iter().enumerate() {
                            let LuaValue::Table(xy) = v else { return Err(format!("{at}: {what}, frame {j} is a {}, not {{ x, y }}", v.type_name())) };
                            let xy = fields(&xy, at)?;
                            if k != DataKey::Int(j as i64 + 1) || xy.len() != 2 || xy[0].0 != DataKey::Int(1) || xy[1].0 != DataKey::Int(2) {
                                return Err(format!("{at}: {what}, frame {j}: an offset is {{ x, y }}, in a list by frame"));
                            }
                            let c = |v: &LuaValue| int(v, &format!("{what}, frame {j}'s offset"), -128, 127).map(|n| n as i8);
                            anim.push((c(&xy[0].1)?, c(&xy[1].1)?));
                        }
                        if anim.len() != frames {
                            return Err(format!("{at}: {what} has {} offsets, and the animation {frames} frames", anim.len()));
                        }
                        offsets.push(anim);
                    }
                    if offsets.len() != def.animations.len() {
                        return Err(format!("{at}: {what} has offsets for {} animations, and the sprite has {}", offsets.len(), def.animations.len()));
                    }
                    def.parts.insert(n, offsets);
                }
            }
        }
        v => return Err(format!("{at}: a sprite's third argument is a table (`{{ parts = ... }}`), not {}", v.type_name())),
    }
    Ok(def)
}

/// Install `asset`: a resolver per asset kind, working while content
/// loads. A sprite is defined where its module makes it
/// (`asset.sprite("bomb", animations)`); any other asset is named
/// (`asset.sound("throw")`).
pub(crate) fn install_assets(
    lua: &Lua,
    tables: &Rc<RefCell<AssetTables>>,
    module: Rc<dyn Fn() -> Option<String>>,
) -> mlua::Result<()> {
    let asset = lua.create_table()?;
    for kind in AssetKind::ALL {
        let tables = Rc::downgrade(tables);
        let module = module.clone();

        let f = lua.create_function(move |lua, (name, animations, extra): (LuaValue, LuaValue, LuaValue)| {
            let what = format!("asset.{kind}");
            let at = module().ok_or_else(|| mlua::Error::runtime(format!("{what}: assets are named while content loads")))?;
            let tables = tables.upgrade().ok_or_else(|| mlua::Error::runtime(format!("{what}: content has loaded")))?;
            let LuaValue::String(name) = name else {
                return Err(mlua::Error::runtime(format!("{at}: {what} takes a name, not {}", name.type_name())));
            };
            let name = name.to_str()?.to_string();
            if kind == AssetKind::Sprite {
                let def = sprite_def(&format!("{at}: sprite {name:?}"), &animations, &extra).map_err(mlua::Error::runtime)?;
                return tables.borrow_mut().define_sprite(lua, &at, &name, def);
            }
            if !animations.is_nil() || !extra.is_nil() {
                return Err(mlua::Error::runtime(format!("{at}: {what} takes a name alone")));
            }
            tables.borrow_mut().named(lua, kind, &name)
        })?;
        asset.raw_set(kind.name(), f)?;
    }
    asset.set_readonly(true);
    lua.globals().set("asset", asset)?;
    Ok(())
}

/// What a game's modules have and a support pack's don't: the playing
/// game's context, which reaches a support pack only as its callers'
/// arguments (docs/design/content-model-v2.md §4.0). `asset` names the
/// game's asset pack; `rules` reaches the game's rules' state, of the side
/// their running call is for.
pub(crate) const GAME_CONTEXT: [&str; 2] = ["asset", "rules"];

/// The environments modules run in: a game pack's modules', and a support
/// pack's, which lacks [`GAME_CONTEXT`] (a module's closures keep its
/// environment, so a support pack's function lacks it whoever calls it).
/// Both are read-only copies of the globals, which hold neither `new` nor
/// the game's context: Luau resolves a module's globals against the
/// VM's globals when it loads, so what one environment lacks must be
/// missing there too.
pub(crate) struct Environments {
    pub game: Table,
    pub support: Table,
}

/// The module of the content function calling (stack level 1), for
/// messages and the API's guards.
pub(crate) fn caller(lua: &Lua) -> String {
    lua.inspect_stack(1, |d| d.source().source.map(|s| s.trim_start_matches('@').trim_end_matches(".luau").to_string()))
        .flatten()
        .unwrap_or_else(|| "a support pack's module".to_string())
}

/// Take `new` and the game's context out of the globals, into the
/// game's and the support packs' environments.
pub(crate) fn environments(lua: &Lua) -> mlua::Result<Environments> {
    let globals = lua.globals();
    let mut own = Vec::new();
    for name in ["new"].into_iter().chain(GAME_CONTEXT) {
        let v: LuaValue = globals.raw_get(name)?;
        if let LuaValue::Table(t) = &v {
            t.set_readonly(true);
        }
        own.push((name, v));
        globals.raw_set(name, LuaValue::Nil)?;
    }
    let copy = || -> mlua::Result<Table> {
        let env = lua.create_table()?;
        for pair in globals.pairs::<LuaValue, LuaValue>() {
            let (k, v) = pair?;
            env.raw_set(k, v)?;
        }
        Ok(env)
    };
    let game = copy()?;
    let support = copy()?;
    for (name, v) in own {
        game.raw_set(name, v.clone())?;
        if !GAME_CONTEXT.contains(&name) {
            support.raw_set(name, v)?;
        }
    }
    // (A support pack's module reaching for the game's context fails,
    // naming itself.)
    let meta = lua.create_table()?;
    meta.raw_set(
        "__index",
        lua.create_function(|lua, (_, k): (LuaValue, LuaValue)| -> mlua::Result<LuaValue> {
            if let LuaValue::String(s) = &k
                && let Ok(name) = s.to_str()
                && GAME_CONTEXT.contains(&&*name)
            {
                return Err(mlua::Error::runtime(format!(
                    "{}: a support pack has no `{}`: the game's context reaches a support pack only as its callers' arguments (a maker takes its look from the game's module)",
                    caller(lua),
                    &*name
                )));
            }
            Ok(LuaValue::Nil)
        })?,
    )?;
    meta.set_readonly(true);
    support.set_metatable(Some(meta))?;
    for env in [&game, &support] {
        env.set_readonly(true);
        env.set_safeenv(true);
    }
    Ok(Environments { game, support })
}

/// Install `new`: a tag constructor per registry only code reaches
/// ([`TAGGED`]), which marks a table as what it is (a metatable) and
/// returns it. `module` names the module loading at the moment of a call
/// (None outside loading).
pub(crate) fn install(lua: &Lua, tags: &Rc<RefCell<Tags>>, module: Rc<dyn Fn() -> Option<String>>) -> mlua::Result<()> {
    let new = lua.create_table()?;
    for registry in TAGGED {
        let meta = lua.create_table()?;
        meta.raw_set("__metatable", format!("{} definition", registry.name()))?;
        let tags = Rc::downgrade(tags);
        let module = module.clone();
        let tag = move |record_type: Option<String>, spec: LuaValue| -> mlua::Result<Table> {
            let what = format!("new.{}", registry.name());
            let tags = tags.upgrade().filter(|t| t.borrow().open).ok_or_else(|| {
                mlua::Error::runtime(format!("{what}: tables are tagged while content loads"))
            })?;
            let at = module().ok_or_else(|| mlua::Error::runtime(format!("{what}: called outside a module")))?;
            let LuaValue::Table(table) = spec else {
                return Err(mlua::Error::runtime(format!("{at}: {what} takes a table, not {}", spec.type_name())));
            };
            if table.metatable().is_some() {
                return Err(mlua::Error::runtime(format!("{at}: {what}'s table is tagged already (twice?)")));
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
            tags.borrow_mut().by_table.insert(table.to_pointer(), Tag { registry, record_type, module: at, _table: table.clone() });
            Ok(table)
        };
        let f = if registry == Registry::Record {
            // `new.record(type, table)`: the string is the record's type.
            let what = "new.record(type, table): the type";
            lua.create_function(move |_, (name, spec): (LuaValue, LuaValue)| {
                let LuaValue::String(t) = name else {
                    return Err(mlua::Error::runtime(format!("{what} is a string")));
                };
                let t = t.to_str()?.to_string();
                tag(Some(t), spec)
            })?
        } else {
            lua.create_function(move |_, spec: LuaValue| tag(None, spec))?
        };
        new.raw_set(registry.name(), f)?;
    }
    new.set_readonly(true);
    lua.globals().set("new", new)?;
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
            LuaValue::Integer(i) => DataKey::Int(i64::from(i)),
            LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9e15 => DataKey::Int(n as i64),
            k => return Err(format!("{at}: a table key is a {}, not a string or an integer", k.type_name())),
        };
        out.push((k, v));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// A table's fields with a string, an integer or a boolean key, in key order
/// ([`fields`]'s), and its other entries (a key that is a table or a
/// function), as (key, value).
#[allow(clippy::type_complexity)]
fn all_fields(t: &Table, at: &str) -> Result<(Vec<(DataKey, LuaValue)>, Vec<(LuaValue, LuaValue)>), String> {
    let mut named = Vec::new();
    let mut other = Vec::new();
    for pair in t.clone().pairs::<LuaValue, LuaValue>() {
        let (k, v) = pair.map_err(|e| format!("{at}: {e}"))?;
        match k {
            LuaValue::String(s) => {
                named.push((DataKey::Str(s.to_str().map_err(|e| format!("{at}: a key isn't UTF-8 ({e})"))?.to_string()), v))
            }
            LuaValue::Integer(i) => named.push((DataKey::Int(i64::from(i)), v)),
            LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9e15 => named.push((DataKey::Int(n as i64), v)),
            // (A path names a boolean key by its word: `PUFFS/true`.)
            LuaValue::Boolean(b) => named.push((DataKey::Str(b.to_string()), v)),
            k => other.push((k, v)),
        }
    }
    named.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((named, other))
}

/// A definition the walk found.
struct Found {
    registry: Registry,
    key: String,
    module: String,
    record_type: Option<String>,
    table: Table,
}

/// The values a Lua function captures, each with the name it captures it
/// by (its upvalues; a Rust function has none).
fn captured(lua: &Lua, f: &mlua::Function) -> Vec<(String, LuaValue)> {
    // (Pushed as name, value pairs above the function.)
    let pushed: mlua::Result<mlua::MultiValue> = unsafe {
        lua.exec_raw(f.clone(), |state| {
            if mlua::ffi::lua_iscfunction(state, 1) != 0 {
                return;
            }
            let mut n = 1;
            loop {
                let name = mlua::ffi::lua_getupvalue(state, 1, n);
                if name.is_null() {
                    break;
                }
                let name = std::ffi::CStr::from_ptr(name).to_bytes().to_vec();
                // (Name below value.)
                mlua::ffi::lua_pushlstring(state, name.as_ptr() as *const _, name.len());
                mlua::ffi::lua_insert(state, -2);
                n += 1;
            }
        })
    };
    let Ok(values) = pushed else { return Vec::new() };
    let mut out = Vec::new();
    let mut it = values.into_iter().skip(1);
    let mut n = 0;
    while let (Some(name), Some(value)) = (it.next(), it.next()) {
        n += 1;
        let name = match name {
            LuaValue::String(s) if !s.as_bytes().is_empty() => s.to_string_lossy(),
            _ => format!("^{n}"),
        };
        out.push((name, value));
    }
    out
}

/// Where each table a module returns was written, by table: the module
/// whose result it is, else one whose result holds it as a field; of
/// several (a section's init merges its modules' tables), the one whose
/// result holds the fewest, then the longest path (the module that wrote it, not the init
/// that gathers it). For messages and what a definition says of itself.
fn written_in(modules: &BTreeMap<String, LuaValue>) -> HashMap<Ptr, String> {
    let mut best: HashMap<Ptr, (usize, &String)> = HashMap::new();
    for (path, v) in modules {
        let LuaValue::Table(m) = v else { continue };
        let fields: Vec<Ptr> = m
            .clone()
            .pairs::<LuaValue, LuaValue>()
            .filter_map(|pair| match pair {
                Ok((_, LuaValue::Table(x))) => Some(x.to_pointer()),
                _ => None,
            })
            .collect();
        // (A module's own result first: 0 fields between.)
        let n = fields.len() + 1;
        for (p, n) in std::iter::once((m.to_pointer(), 0)).chain(fields.into_iter().map(|p| (p, n))) {
            let better = best.get(&p).is_none_or(|&(m, q)| n < m || (n == m && path.len() > q.len()));
            if better {
                best.insert(p, (n, path));
            }
        }
    }
    best.into_iter().map(|(p, (_, path))| (p, path.clone())).collect()
}

/// Finish the define phase: walk from the root (or the loose modules'
/// results, `entries`), keys, schemas, the canonical tree.
pub(crate) fn finish(
    lua: &Lua,
    tags: &RefCell<Tags>,
    assets: &AssetTables,
    games: &HashSet<String>,
    entries: &[(String, LuaValue)],
    modules: &BTreeMap<String, LuaValue>,
) -> Result<Defined, String> {
    let mut t = tags.borrow_mut();
    t.open = false;
    let tags = std::mem::take(&mut t.by_table);
    drop(t);

    let mut found: Vec<Found> = Vec::new();
    let mut by_ptr: HashMap<Ptr, usize> = HashMap::new();
    let written = written_in(modules);
    // What the walk starts from: each root's definitions, then each loose
    // module's result, each with its key path.
    // (`opaque`: reached through a key that is no string or integer, a
    // set's table, say, which no path names.)
    let mut queue: std::collections::VecDeque<(LuaValue, String, bool)> = Default::default();
    let mut roots = 0;
    for (module, value) in entries {
        let LuaValue::Table(root) = value else { continue };
        let named = |k: &str| SECTIONS.iter().any(|(s, _)| *s == k) || k == RULES_SECTION;
        let entries_ = fields(root, module)?;
        // (A game's top module is its root; any other entry is a module
        // walked loose, unless what it returns is a root: a test's.)
        let top = keys::root_of(module).is_some_and(|p| games.contains(p)) && keys::local(module) == nettai_content_api::packs::INIT;
        // (A top module's result is its game's root, whose other keys are
        // its collections; a loose module's is a root where every key is a
        // section's or the rules'.)
        let is_root = top || (!entries_.is_empty() && entries_.iter().all(|(k, _)| matches!(k, DataKey::Str(s) if named(s))));
        if !is_root {
            queue.push_back((value.clone(), module_key(module, games), false));
            continue;
        }
        if let Some((k, _)) = entries_.iter().find(|(k, _)| !matches!(k, DataKey::Str(_))) {
            return Err(format!("{}.luau: a game's root's keys are names, not {k}", keys::module_path(module)));
        }
        roots += 1;
        // The core's sections, then the game's own collections (the root's
        // other keys, but its rules), each entry keyed `<collection>/<id>`.
        let collections: Vec<String> = entries_
            .iter()
            .filter_map(|(k, _)| match k {
                DataKey::Str(s) if !SECTIONS.iter().any(|(c, _)| c == s) && s != RULES_SECTION => Some(s.clone()),
                _ => None,
            })
            .collect();
        for c in &collections {
            if !nettai_content_api::is_collection_name(c) {
                return Err(format!("{}.luau: `{c}` names no collection (lowercase words joined by _)", keys::module_path(module)));
            }
        }
        let parts = SECTIONS.iter().map(|&(s, r)| (s, r, false)).chain(collections.iter().map(|c| (c.as_str(), Registry::Entry, true)));
        for (section, registry, collection) in parts {
            match root.raw_get::<LuaValue>(section).map_err(|e| e.to_string())? {
                LuaValue::Nil => {}
                LuaValue::Table(defs) => {
                    for (id, v) in fields(&defs, section)? {
                        let at = format!("{}.luau: {section}.{id}", keys::module_path(module));
                        let DataKey::Str(id) = id else { return Err(format!("{at}: a section's keys are ids")) };
                        if !valid_key(&id) {
                            return Err(format!("{at}: {id:?} is not a valid id (lowercase words in -, joined with /)"));
                        }
                        let LuaValue::Table(table) = v else {
                            return Err(format!("{at}: a {} is a table, not {}", registry.name(), v.type_name()));
                        };
                        if let Some(tag) = tags.get(&table.to_pointer()) {
                            return Err(format!("{at}: a {} (new.{}), not a {}", tag.registry.name(), tag.registry.name(), registry.name()));
                        }
                        if !table.raw_get::<LuaValue>("id").map_err(|e| format!("{at}: {e}"))?.is_nil() {
                            return Err(format!("{at}: a {}'s id is its key in `{section}`: it names none of its own", registry.name()));
                        }
                        if let Some(&j) = by_ptr.get(&table.to_pointer()) {
                            return Err(format!("{at}: the same table as {} {}", found[j].registry.name(), found[j].key));
                        }
                        by_ptr.insert(table.to_pointer(), found.len());
                        let module = written.get(&table.to_pointer()).cloned().unwrap_or_else(|| module.clone());
                        let (key, record_type) =
                            if collection { (nettai_content_api::entry_key(section, &id), Some(section.to_string())) } else { (id.clone(), None) };
                        found.push(Found { registry, key: key.clone(), module, record_type, table: table.clone() });
                        queue.push_back((LuaValue::Table(table), key, false));
                    }
                }
                v => {
                    let what = if collection { "entries".to_string() } else { format!("{}s", registry.name()) };
                    return Err(format!("{}.luau: `{section}` is a table of {what} by id, not {}", keys::module_path(module), v.type_name()));
                }
            }
        }
        match root.raw_get::<LuaValue>(RULES_SECTION).map_err(|e| e.to_string())? {
            LuaValue::Nil => {}
            LuaValue::Table(rules) => {
                if roots > 1 || found.iter().any(|f| f.registry == Registry::Rules) {
                    return Err("a game has one rules definition: two roots give one".to_string());
                }
                by_ptr.insert(rules.to_pointer(), found.len());
                let at = written.get(&rules.to_pointer()).cloned().unwrap_or_else(|| module.clone());
                found.push(Found {
                    registry: Registry::Rules,
                    key: nettai_content_api::RULESET_KEY.to_string(),
                    module: at,
                    record_type: None,
                    table: rules.clone(),
                });
                queue.push_back((LuaValue::Table(rules), RULES_SECTION.to_string(), false));
            }
            v => return Err(format!("{}.luau: `{RULES_SECTION}` is a table, not {}", keys::module_path(module), v.type_name())),
        }
    }
    let reaches = |x: &LuaValue| matches!(x, LuaValue::Table(_) | LuaValue::Function(_));
    // Names from the modules: each module's result, and what it holds
    // through plain tables (not into a definition), by the module's name and
    // the fields (`rules/collision/attack`, `exelib:regions/area/0`), the
    // shortest then the first: a shared table is named where it is written,
    // whatever reaches it first.
    let mut by_module: HashMap<Ptr, String> = HashMap::new();
    let mut named: std::collections::VecDeque<(LuaValue, String)> =
        modules.iter().filter(|(_, v)| reaches(v)).map(|(m, v)| (v.clone(), module_key(m, games))).collect();
    while let Some((v, path)) = named.pop_front() {
        let p = v.to_pointer();
        if by_module.contains_key(&p) {
            continue;
        }
        by_module.insert(p, path.clone());
        if let LuaValue::Table(t) = &v
            && !by_ptr.contains_key(&p)
            && !tags.contains_key(&p)
        {
            for (k, x) in all_fields(t, &path)?.0 {
                if reaches(&x) {
                    named.push_back((x, format!("{path}/{k}")));
                }
            }
        }
    }
    // The walk, breadth first (the shortest path first; within a table its
    // fields in key order, then its metatable; within a function what it
    // captures in order).
    let mut seen: HashSet<Ptr> = HashSet::new();
    let mut unwritten = Vec::new();
    while let Some((v, path, opaque)) = queue.pop_front() {
        // (Named by its module where one holds it.)
        let (path, opaque) = match by_module.get(&v.to_pointer()) {
            Some(name) => (name.clone(), false),
            None => (path, opaque),
        };
        match &v {
            LuaValue::Table(table) => {
                let p = table.to_pointer();
                if !seen.insert(p) || assets.by_ptr.contains_key(&p) {
                    continue;
                }
                if !by_ptr.contains_key(&p)
                    && let Some(tag) = tags.get(&p)
                {
                    let what = format!("{}.luau: new.{}", keys::module_path(&tag.module), tag.registry.name());
                    let key = match table.raw_get::<LuaValue>("id").map_err(|e| format!("{what}: {e}"))? {
                        LuaValue::Nil if opaque => {
                            return Err(format!(
                                "{what}: reached only through a table's key that is no string or integer ({path}/...), which no key can name: an `id`, or a field it is reached by"
                            ));
                        }
                        LuaValue::Nil => path.clone(),
                        LuaValue::String(id) => {
                            let id = id.to_str().map_err(|e| format!("{what}: {e}"))?.to_string();
                            if !valid_key(&id) {
                                unwritten.push(format!("{what}: {id:?} is not a valid id (lowercase words in -, joined with /: \"eraseman/mark\")"));
                            }
                            id
                        }
                        v => return Err(format!("{what}: `id` is a {}, not a string", v.type_name())),
                    };
                    by_ptr.insert(p, found.len());
                    found.push(Found {
                        registry: tag.registry,
                        key,
                        module: tag.module.clone(),
                        record_type: tag.record_type.clone(),
                        table: table.clone(),
                    });
                }
                // What a definition holds is named from its key
                // (`minibomb/action`), whatever path reached it.
                let (path, opaque) = match by_ptr.get(&p) {
                    Some(&i) => (found[i].key.clone(), false),
                    None => (path, opaque),
                };
                let (named, other) = all_fields(table, &path)?;
                for (k, x) in named {
                    if reaches(&x) {
                        queue.push_back((x, format!("{path}/{k}"), opaque));
                    }
                }
                for (k, x) in other {
                    for x in [k, x] {
                        if reaches(&x) {
                            queue.push_back((x, format!("{path}/[...]"), true));
                        }
                    }
                }
                if let Some(meta) = table.metatable() {
                    queue.push_back((LuaValue::Table(meta), format!("{path}/(metatable)"), opaque));
                }
            }
            LuaValue::Function(f) => {
                if !seen.insert(f.to_pointer()) {
                    continue;
                }
                for (name, x) in captured(lua, f) {
                    if reaches(&x) {
                        queue.push_back((x, format!("{path}/{name}"), opaque));
                    }
                }
            }
            _ => {}
        }
    }
    if !unwritten.is_empty() {
        return Err(unwritten.join("\n"));
    }
    let mut by_key: BTreeMap<(Registry, &str), usize> = BTreeMap::new();
    for (i, f) in found.iter().enumerate() {
        if let Some(&j) = by_key.get(&(f.registry, f.key.as_str())) {
            return Err(format!(
                "{} {:?} is two tables: one made in {}.luau, one in {}.luau",
                f.registry.name(),
                f.key,
                keys::module_path(&found[j].module),
                keys::module_path(&f.module)
            ));
        }
        by_key.insert((f.registry, f.key.as_str()), i);
    }
    // A root's definition reads its id, its key there (`chip.id`), which no
    // table of the content's writes: a view, not a field of its data.
    for f in &found {
        if SECTIONS.iter().any(|&(_, r)| r == f.registry) || f.registry == Registry::Entry {
            let id = lua.create_table().map_err(|e| e.to_string())?;
            // (An entry's id is its key's last part: `canodumb` of
            // `patch_cards/canodumb`.)
            let own = if f.registry == Registry::Entry { nettai_content_api::entry_parts(&f.key).1 } else { f.key.as_str() };
            id.raw_set("id", own).map_err(|e| e.to_string())?;
            let meta = lua.create_table().map_err(|e| e.to_string())?;
            id.set_readonly(true);
            meta.raw_set("__index", id).map_err(|e| e.to_string())?;
            meta.raw_set("__metatable", format!("{} definition", f.registry.name())).map_err(|e| e.to_string())?;
            meta.set_readonly(true);
            if f.table.metatable().is_some() {
                return Err(format!("{} {}: a table with a metatable of its own", f.registry.name(), f.key));
            }
            // (Its module's result was frozen as it loaded.)
            f.table.set_readonly(false);
            f.table.set_metatable(Some(meta)).map_err(|e| format!("{} {}: {e}", f.registry.name(), f.key))?;
            f.table.set_readonly(true);
        }
    }

    // Schemas: the `state` tables of kinds, actions and the game's rules,
    // and the rules' `setup` and `navi_state` tables.
    let mut schema_of: HashMap<Ptr, usize> = HashMap::new();
    let mut schemas: Vec<(String, String, Table)> = Vec::new();
    let mut claim = |t: Table, key: String, module: &str, schemas: &mut Vec<(String, String, Table)>| {
        if by_ptr.contains_key(&t.to_pointer()) {
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
            Registry::Rules => &["state", "setup", "navi_state", "stats"],
            _ => continue,
        };
        for &field in tables {
            if let Ok(LuaValue::Table(state)) = found[i].table.raw_get::<LuaValue>(field) {
                claim(state, format!("{}:{key}/{field}", registry.name()), &found[i].module, &mut schemas);
            }
        }
    }
    let schema_keys: HashMap<Ptr, String> = schema_of.iter().map(|(&p, &i)| (p, schemas[i].0.clone())).collect();
    let def_keys: HashMap<Ptr, (Registry, String)> =
        found.iter().map(|f| (f.table.to_pointer(), (f.registry, f.key.clone()))).collect();

    // The canonical tree.
    let refs = Refs { defs: &def_keys, schemas: &schema_keys, assets };
    let mut defs = Vec::with_capacity(found.len() + schemas.len());
    let mut tables = Vec::with_capacity(found.len() + schemas.len());
    for (&(registry, key), &i) in &by_key {
        let f = &found[i];
        let at = format!("{} {key}", registry.name());
        let spec = refs.data(&LuaValue::Table(f.table.clone()), &at, true, &mut Vec::new())?;
        defs.push(Definition {
            registry,
            key: key.to_string(),
            module: f.module.clone(),
            record_type: f.record_type.clone().filter(|_| matches!(registry, Registry::Record | Registry::Entry)),
            spec,
        });
        tables.push(f.table.clone());
    }
    let mut schema_order: Vec<usize> = (0..schemas.len()).collect();
    schema_order.sort_by(|&a, &b| schemas[a].0.cmp(&schemas[b].0));
    // (A schema's records are its own, whole: a table another schema is too
    // is a record of it, not a reference: the rules' state holds the AI's
    // fields that are a driven navi's `navi_state`.)
    let no_schemas = HashMap::new();
    let inline = Refs { defs: &def_keys, schemas: &no_schemas, assets };
    for i in schema_order {
        let (key, module, t) = &schemas[i];
        let at = format!("schema {key}");
        let spec = inline.data(&LuaValue::Table(t.clone()), &at, true, &mut Vec::new())?;
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

/// The key path a loose module's result starts at: its name
/// ([`anonymous_base`]), a folder's init by its folder (`objects/boulder`).
fn module_key(module: &str, games: &HashSet<String>) -> String {
    let base = anonymous_base(module, games);
    base.strip_suffix("/init").unwrap_or(base).to_string()
}

/// A module's name: a game pack's module's path in the pack
/// (`chips/cannon/init`), a support pack's module's name (`exelib:regions`).
/// `games`, the game packs (none: every module is a game's, a test's
/// modules alone).
fn anonymous_base<'m>(module: &'m str, games: &HashSet<String>) -> &'m str {
    match keys::root_of(module) {
        Some(pack) if games.is_empty() || games.contains(pack) => keys::local(module),
        _ => module,
    }
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
            LuaValue::Integer(i) => Data::Int(i64::from(*i)),
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
                    let name = self.assets.names.name(kind, h).expect("an interned asset").to_string();
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
