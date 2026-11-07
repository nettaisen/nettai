//! The choices the engine knows as content holds them (docs/design/
//! scripting.md §2.1, [`nettai_content_api::choice`]): a global table per
//! kind (`DragStep`), its members the kind's values (`DragStep.slide`).
//! A value is compared with `==` and passed whole; its `name` is its
//! member's (as the game's names' values have theirs), and `tostring` shows
//! its table and member. Each value is made once per VM, when
//! the tables are, so two values are equal exactly when they are the same
//! value (no `__eq`: Luau compares them as it does any two userdata, with
//! no call). Reading a member a table hasn't is an error, and the tables are
//! read-only.
//!
//! The define phase reads a value back as [`Data::Choice`].

use mlua::{AnyUserData, Lua, MetaMethod, UserData, UserDataMethods, Value as LuaValue};
use nettai_content_api::{Choice, ChoiceKind, Data};

/// A choice's value.
#[derive(Clone, Copy)]
pub(crate) struct Value(pub Choice);

impl UserData for Value {
    // (Its name, as the game's names' values have theirs: a message's, a
    // table keyed by names'.)
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.0.member()));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(this.0.to_string()));
    }
}

/// The values made, by kind and member, and each kind's table.
struct Values(Vec<Vec<AnyUserData>>, Vec<(*const std::ffi::c_void, ChoiceKind)>);

/// The kind whose members are `names`, in their order: the kind a field of
/// named variants holds (an engine field's, or a state's field declared with
/// the kind's table, `source = ChargeSource`). A state's own list of names
/// is never one (the define phase refuses a list a kind's table states).
pub(crate) fn kind_of(names: &[String]) -> Option<ChoiceKind> {
    ChoiceKind::ALL
        .iter()
        .copied()
        .find(|k| k.members().len() == names.len() && k.members().iter().zip(names).all(|(a, b)| a == b))
}

/// The kind whose table `v` is (`ChargeSource`), if it is one.
pub(crate) fn table_kind(lua: &Lua, v: &LuaValue) -> Option<ChoiceKind> {
    let LuaValue::Table(t) = v else { return None };
    let values = lua.app_data_ref::<Values>()?;
    let p = t.to_pointer();
    values.1.iter().find(|(q, _)| *q == p).map(|&(_, k)| k)
}

/// The value of `c`.
pub(crate) fn value(lua: &Lua, c: Choice) -> mlua::Result<AnyUserData> {
    let values = lua.app_data_ref::<Values>().ok_or_else(|| mlua::Error::runtime("the choice tables are not made"))?;
    Ok(values.0[c.kind as usize][c.index as usize].clone())
}

/// The value of member `index` of `kind`.
pub(crate) fn value_of(lua: &Lua, kind: ChoiceKind, index: u8) -> mlua::Result<LuaValue> {
    Ok(LuaValue::UserData(value(lua, Choice { kind, index })?))
}

/// The value of the member of `kind` the engine names `name`.
pub(crate) fn named(lua: &Lua, kind: ChoiceKind, name: &str) -> mlua::Result<LuaValue> {
    let c = Choice::of(kind, name).ok_or_else(|| mlua::Error::runtime(format!("{} has no member {name:?}", kind.name())))?;
    Ok(LuaValue::UserData(value(lua, c)?))
}

/// The choice `v` is, if it is one.
pub(crate) fn of(v: &LuaValue) -> Option<Choice> {
    match v {
        LuaValue::UserData(ud) => ud.borrow::<Value>().ok().map(|c| c.0),
        _ => None,
    }
}

/// What `v` is, for a message.
fn describe(v: &LuaValue) -> String {
    match (v, of(v)) {
        (_, Some(c)) => c.to_string(),
        (LuaValue::String(s), _) => format!("the string {:?}", s.to_string_lossy()),
        (LuaValue::UserData(ud), _) => ud.type_name().map(|s| s.to_string_lossy()).unwrap_or_default(),
        (v, _) => v.type_name().to_string(),
    }
}

/// The member `v` is of `kind`: an error when it is anything else (a
/// string with its name among them).
pub(crate) fn arg(v: &LuaValue, kind: ChoiceKind, what: &str) -> mlua::Result<Choice> {
    match of(v) {
        Some(c) if c.kind == kind => Ok(c),
        _ => Err(expected(v, kind, what)),
    }
}

/// [`arg`], none when `v` is nil.
pub(crate) fn opt_arg(v: &LuaValue, kind: ChoiceKind, what: &str) -> mlua::Result<Option<Choice>> {
    if v.is_nil() { Ok(None) } else { arg(v, kind, what).map(Some) }
}

/// The error of `v` given where a `kind` is wanted.
pub(crate) fn expected(v: &LuaValue, kind: ChoiceKind, what: &str) -> mlua::Error {
    let k = kind.name();
    let first = Choice { kind, index: 0 }.member();
    mlua::Error::runtime(format!("{what}: expected a {k} (`{k}.{first}`), got {}", describe(v)))
}

/// The engine's name of the member `v` is of `kind`.
pub(crate) fn name(v: &LuaValue, kind: ChoiceKind, what: &str) -> mlua::Result<&'static str> {
    arg(v, kind, what).map(Choice::name)
}

/// The engine's value the member `v` is of `kind` parses to (`parse` its
/// name: the engine's own `from_name`).
pub(crate) fn parsed<T>(v: &LuaValue, kind: ChoiceKind, what: &str, parse: impl Fn(&str) -> Option<T>) -> mlua::Result<T> {
    let c = arg(v, kind, what)?;
    parse(c.name()).ok_or_else(|| mlua::Error::runtime(format!("{what}: {c} has no engine value")))
}

/// A choice as a definition's data.
pub(crate) fn data(v: &LuaValue) -> Option<Data> {
    of(v).map(Data::Choice)
}

/// Make `t`, the table `name` of values, read-only, and reading a member it
/// hasn't an error.
pub(crate) fn seal(lua: &Lua, t: &mlua::Table, name: &'static str) -> mlua::Result<()> {
    let meta = lua.create_table()?;
    meta.raw_set(
        "__index",
        lua.create_function(move |_, (_, k): (LuaValue, LuaValue)| -> mlua::Result<()> {
            Err(mlua::Error::runtime(format!("{name} has no member {}", match &k {
                LuaValue::String(s) => format!("{:?}", s.to_string_lossy()),
                k => k.type_name().to_string(),
            })))
        })?,
    )?;
    meta.raw_set("__metatable", name)?;
    meta.set_readonly(true);
    t.set_metatable(Some(meta))?;
    t.set_readonly(true);
    Ok(())
}

/// Make every kind's table, each a global (`DragStep`), and the values.
pub(crate) fn install(lua: &Lua, g: &mlua::Table) -> mlua::Result<()> {
    let mut values = Vec::with_capacity(ChoiceKind::ALL.len());
    let mut tables = Vec::with_capacity(ChoiceKind::ALL.len());
    for (i, &kind) in ChoiceKind::ALL.iter().enumerate() {
        debug_assert_eq!(kind as usize, i);
        let t = lua.create_table()?;
        let mut of_kind = Vec::with_capacity(kind.members().len());
        for index in 0..kind.members().len() {
            let c = Choice { kind, index: index as u8 };
            let ud = lua.create_userdata(Value(c))?;
            t.raw_set(c.member(), ud.clone())?;
            of_kind.push(ud);
        }
        seal(lua, &t, kind.name())?;
        values.push(of_kind);
        tables.push((kind.name(), t));
    }
    let ptrs = tables.iter().zip(ChoiceKind::ALL).map(|((_, t), &k)| (t.to_pointer(), k)).collect();
    lua.set_app_data(Values(values, ptrs));
    for (name, t) in tables {
        g.set(name, t)?;
    }
    Ok(())
}
