//! The names a game defines (docs/design/scripting.md §2.1): its panel
//! types, chip families, emotions, versions, its rules' custom-screen
//! buttons and windows, and its collections, each kind a value table
//! content names them by (`PanelType.cracked`, `Family.sword`,
//! `Window.cross_chosen`), as `Element` is for the elements. A name is a
//! value of its own, opaque to content, compared with `==`; each has one
//! value per VM, made when first asked for. `v.name` reads its name (a
//! table keyed by names looks it up by it).
//!
//! What a table holds is the game's: the engine says it after the define
//! phase (`BindPlan::names`), from what the game's rules, builds and root
//! say it has. The define phase, which runs before anything says so, hands
//! out a value for any member read and keeps what was read
//! ([`read`]), which the engine holds to the game's names when they are
//! known; a runtime's tables hold the game's alone ([`seal`]), and reading
//! one they lack is an error.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use mlua::{AnyUserData, Lua, MetaMethod, UserData, UserDataFields, UserDataMethods, Value as LuaValue};
use nettai_content_api::{GameNames, NameKind};

/// A name a game defines.
#[derive(Clone)]
pub(crate) struct GameName {
    pub kind: NameKind,
    pub name: Rc<str>,
}

impl UserData for GameName {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.name.to_string()));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            Ok(other.borrow::<GameName>().is_ok_and(|o| o.kind == this.kind && o.name == this.name))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(format!("{}.{}", this.kind.table(), this.name)));
    }
}

/// The VM's names: the values made so far, what the define phase read
/// (with where it was first read), and the game's, once known.
#[derive(Default)]
struct Names {
    values: RefCell<HashMap<(NameKind, String), AnyUserData>>,
    read: RefCell<BTreeMap<(NameKind, String), String>>,
    sealed: RefCell<Option<GameNames>>,
}

fn names(lua: &Lua) -> mlua::AppDataRef<'_, Names> {
    if lua.app_data_ref::<Names>().is_none() {
        lua.set_app_data(Names::default());
    }
    lua.app_data_ref::<Names>().expect("set above")
}

/// The value of name `name` of `kind` (the engine's: a name it gives is
/// the game's).
pub(crate) fn value(lua: &Lua, kind: NameKind, name: &str) -> mlua::Result<AnyUserData> {
    let n = names(lua);
    if let Some(v) = n.values.borrow().get(&(kind, name.to_string())) {
        return Ok(v.clone());
    }
    let v = lua.create_userdata(GameName { kind, name: name.into() })?;
    n.values.borrow_mut().insert((kind, name.to_string()), v.clone());
    Ok(v)
}

/// A member of a value table read: the game's (an error if it isn't one,
/// once the game's are known), or any while the define phase runs, kept.
fn member(lua: &Lua, kind: NameKind, name: &str) -> mlua::Result<AnyUserData> {
    let n = names(lua);
    match &*n.sealed.borrow() {
        Some(game) => {
            if !game.has(kind, name) {
                return Err(unknown(kind, name, game));
            }
        }
        None => {
            n.read.borrow_mut().entry((kind, name.to_string())).or_insert_with(|| reader(lua));
        }
    }
    drop(n);
    value(lua, kind, name)
}

/// Where content read a member: the first Luau function's line up the
/// stack (the table's `__index` is the VM's call).
fn reader(lua: &Lua) -> String {
    for level in 1..8 {
        let at = lua.inspect_stack(level, |d| {
            let source = d.source().source.map(|s| s.trim_start_matches('@').to_string())?;
            (!source.starts_with('=')).then(|| format!("{}:{}", source.trim_end_matches(".luau"), d.current_line().unwrap_or(0)))
        });
        match at {
            Some(Some(at)) => return at,
            Some(None) => continue,
            None => break,
        }
    }
    "content".to_string()
}

fn unknown(kind: NameKind, name: &str, game: &GameNames) -> mlua::Error {
    let names = game.of(kind);
    mlua::Error::runtime(format!(
        "{}.{name}: none of the game's {}s ({})",
        kind.table(),
        kind.what(),
        if names.is_empty() { "it has none".to_string() } else { names.join(", ") }
    ))
}

/// The name `v` is, if it is one of `kind`.
pub(crate) fn of(v: &LuaValue) -> Option<(NameKind, Rc<str>)> {
    match v {
        LuaValue::UserData(ud) => ud.borrow::<GameName>().ok().map(|n| (n.kind, n.name.clone())),
        _ => None,
    }
}

/// The name of an argument that is one of `kind`'s (`PanelType.cracked`).
pub(crate) fn arg(v: &LuaValue, kind: NameKind, what: &str) -> mlua::Result<Rc<str>> {
    match of(v) {
        Some((k, name)) if k == kind => Ok(name),
        Some((k, name)) => Err(mlua::Error::runtime(format!("{what}: expected a {} (`{}.x`), got {}.{name}", kind.table(), kind.table(), k.table()))),
        None => Err(mlua::Error::runtime(format!(
            "{what}: expected a {} (`{}.x`), got {}",
            kind.table(),
            kind.table(),
            match v {
                LuaValue::UserData(ud) => ud.type_name().map(|s| s.to_string_lossy()).unwrap_or_default(),
                v => v.type_name().to_string(),
            }
        ))),
    }
}

/// Install a value table per kind (`PanelType`, `Family`, ...): what a
/// member read gives is [`member`]'s.
pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    for kind in NameKind::ALL {
        let t = lua.create_table()?;
        let meta = lua.create_table()?;
        meta.raw_set(
            "__index",
            lua.create_function(move |lua, (_, k): (LuaValue, LuaValue)| match &k {
                LuaValue::String(s) => member(lua, kind, &s.to_str()?),
                k => Err(mlua::Error::runtime(format!("{}: a member is named, not {}", kind.table(), k.type_name()))),
            })?,
        )?;
        meta.raw_set("__metatable", kind.table())?;
        t.set_metatable(Some(meta))?;
        g.set(kind.table(), t)?;
    }
    Ok(())
}

/// What the define phase read of the value tables: each name with where
/// it was first read.
pub(crate) fn read(lua: &Lua) -> Vec<(NameKind, String, String)> {
    names(lua).read.borrow().iter().map(|((k, n), at)| (*k, n.clone(), at.clone())).collect()
}

/// The names each read member must be among: the first read that isn't
/// one of the game's, as an error.
pub fn check(read: &[(NameKind, String, String)], game: &GameNames) -> Result<(), String> {
    match read.iter().find(|(k, n, _)| !game.has(*k, n)) {
        Some((k, n, at)) => Err(format!("{at}: {}", unknown(*k, n, game))),
        None => Ok(()),
    }
}

/// Hold the value tables to the game's names: each holds its members (a
/// read finds them at once), and reading one it lacks is an error. What
/// the load read must be among them.
pub(crate) fn seal(lua: &Lua, game: &GameNames) -> mlua::Result<()> {
    check(&read(lua), game).map_err(mlua::Error::runtime)?;
    *names(lua).sealed.borrow_mut() = Some(game.clone());
    let g = lua.globals();
    for kind in NameKind::ALL {
        let t: mlua::Table = g.get(kind.table())?;
        let readonly = t.is_readonly();
        t.set_readonly(false);
        for name in game.of(kind) {
            t.raw_set(name.as_str(), value(lua, kind, name)?)?;
        }
        t.set_readonly(readonly);
    }
    Ok(())
}
