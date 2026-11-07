//! `schema`: the types a state's declaration gives its fields
//! (docs/design/scripting.md §2.4): a kind's or an action's `state`, the
//! rules' `state`, `setup`, `navi_state` and `stats`, and the rules'
//! extensions (`extends`). Each type is a value of its own, opaque to
//! content: a scalar (`schema.u16`), a reference (`schema.chip`), an asset
//! (`schema.sprite`), an array (`schema.array(schema.u8, 18)`), a byte or
//! none (`schema.optional(schema.u8)`), an entry of a collection
//! (`schema.entry("patch_cards")`), a record type (declared once,
//! `schema.record_type("bomb-variant")`, which `new.record` takes too), a
//! list (`schema.list(T, n)`) and a field the engine knows by a role
//! (`schema.role(role, T)`). A list of variant names (an enum) and a table
//! of fields (a record) are plain tables; a string is no type.
//!
//! The define phase reads a value back as data ([`Data::Type`]), which
//! `Schema::from_data` makes the layout of.

use mlua::{Lua, MetaMethod, UserData, UserDataMethods, Value as LuaValue};
use nettai_content_api::{Data, FieldType, TypeData};

/// A `schema` value: a field's type.
#[derive(Clone)]
pub(crate) enum SchemaType {
    /// A type of its own (a scalar, a reference, an asset, an array, a
    /// record type).
    Is(FieldType),
    /// A list of up to `max` elements of `of` (a type, a list of variant
    /// names or a table of fields).
    List { of: LuaValue, max: u16 },
    /// A field of type `of` that the engine knows by `role`.
    Role { role: String, of: LuaValue },
}

impl UserData for SchemaType {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(this.describe()));
    }
}

impl SchemaType {
    /// What a message calls it.
    pub(crate) fn describe(&self) -> String {
        match self {
            SchemaType::Is(ty) => match ty.record_type() {
                Some(t) => format!("record type {t}"),
                None => format!("type {ty}"),
            },
            SchemaType::List { max, .. } => format!("list of {max}"),
            SchemaType::Role { role, .. } => format!("role {role}"),
        }
    }

    /// The record type this is (`schema.record_type`'s value), if it is one.
    pub(crate) fn record_type(&self) -> Option<&str> {
        match self {
            SchemaType::Is(ty) => ty.record_type(),
            _ => None,
        }
    }

    /// This type as a definition's data (`Data::Type`), `data` reading what a
    /// list or a role holds (a type, a list of variants, a table of fields).
    pub(crate) fn data(&self, mut data: impl FnMut(&LuaValue) -> Result<Data, String>) -> Result<Data, String> {
        Ok(Data::Type(match self {
            SchemaType::Is(ty) => TypeData::Is(ty.clone()),
            SchemaType::List { of, max } => TypeData::List(Box::new(data(of)?), *max),
            SchemaType::Role { role, of } => TypeData::Role(role.clone(), Box::new(data(of)?)),
        }))
    }
}

/// The schema type `v` is, if it is one.
pub(crate) fn of(v: &LuaValue) -> Option<mlua::UserDataRef<SchemaType>> {
    match v {
        LuaValue::UserData(ud) => ud.borrow::<SchemaType>().ok(),
        _ => None,
    }
}

/// What `v` is, for a message that expected a type.
fn not_a_type(v: &LuaValue) -> String {
    match v {
        LuaValue::String(s) => format!("the string {:?} (a type is a `schema` value: `schema.u8`, `schema.chip`)", s.to_string_lossy()),
        v => format!("a {}", v.type_name()),
    }
}

/// `what`'s type, `v`: a type of its own (`schema.u8`), for an array's
/// element and an optional field.
fn own_type(v: &LuaValue, what: &str) -> mlua::Result<FieldType> {
    match of(v).as_deref() {
        Some(SchemaType::Is(ty)) => Ok(ty.clone()),
        Some(other) => Err(mlua::Error::runtime(format!("{what} is a type of one value, not a {}", other.describe()))),
        None => Err(mlua::Error::runtime(format!("{what} is a type (`schema.u8`), not {}", not_a_type(v)))),
    }
}

/// What a list holds or a role's field is, `v`: a type (not a role: a
/// field's), a list of variant names or a table of fields, which the define
/// phase reads.
fn field_type(v: LuaValue, what: &str) -> mlua::Result<LuaValue> {
    match (&v, of(&v).as_deref()) {
        (_, Some(SchemaType::Role { .. })) => Err(mlua::Error::runtime(format!("{what} is a type, not a role (a field's)"))),
        (_, Some(_)) | (LuaValue::Table(_), None) => Ok(v),
        _ => Err(mlua::Error::runtime(format!(
            "{what} is a type, a list of variant names or a table of fields, not {}",
            not_a_type(&v)
        ))),
    }
}

/// The `schema` library, but `record_type` (the define phase's: a record
/// type is declared once in a load).
pub(crate) fn lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    for (name, ty) in FieldType::named() {
        t.raw_set(name, SchemaType::Is(ty))?;
    }
    let err = |e: String| mlua::Error::runtime(e);
    t.raw_set(
        "array",
        lua.create_function(move |_, (of, n): (LuaValue, LuaValue)| {
            let elem = own_type(&of, "schema.array's element")?;
            let n = crate::bind::int(&n, "schema.array's length")?;
            Ok(SchemaType::Is(FieldType::array(elem, n).map_err(|e| err(format!("schema.array: {e}")))?))
        })?,
    )?;
    t.raw_set(
        "optional",
        lua.create_function(move |_, of: LuaValue| {
            let ty = own_type(&of, "schema.optional's type")?;
            Ok(SchemaType::Is(FieldType::optional(ty).map_err(|e| err(format!("schema.optional: {e}")))?))
        })?,
    )?;
    // A field that holds a choice of the engine's: the kind's values (the
    // byte the member's place is).
    t.raw_set(
        "choice",
        lua.create_function(move |lua, kind: LuaValue| match crate::choice::table_kind(lua, &kind) {
            Some(k) => Ok(SchemaType::Is(FieldType::Enum(k.members().iter().map(|m| m.to_string()).collect()))),
            None => Err(err(format!("schema.choice takes a choice's table (`schema.choice(ChargeSource)`), not {}", not_a_type(&kind)))),
        })?,
    )?;
    t.raw_set(
        "entry",
        lua.create_function(move |_, collection: LuaValue| {
            let c = crate::names::arg(&collection, nettai_content_api::NameKind::Collection, "schema.entry")?;
            Ok(SchemaType::Is(FieldType::entry(&c).map_err(|e| err(format!("schema.entry: {e}")))?))
        })?,
    )?;
    t.raw_set(
        "list",
        lua.create_function(|_, (of, max): (LuaValue, LuaValue)| {
            let max = crate::bind::int(&max, "schema.list's length")?;
            if !(1..=nettai_content_api::MAX_LIST as i64).contains(&max) {
                return Err(mlua::Error::runtime(format!("schema.list holds 1 to {} elements, not {max}", nettai_content_api::MAX_LIST)));
            }
            Ok(SchemaType::List { of: field_type(of, "schema.list's element")?, max: max as u16 })
        })?,
    )?;
    t.raw_set(
        "role",
        lua.create_function(|_, (role, of): (LuaValue, LuaValue)| {
            let role = crate::choice::name(&role, nettai_content_api::ChoiceKind::SchemaRole, "schema.role's role")?;
            Ok(SchemaType::Role { role: role.to_string(), of: field_type(of, "schema.role's field")? })
        })?,
    )?;
    Ok(t)
}

/// A record type's value (`schema.record_type(name)`, `obstacle.absorbed_look`).
pub(crate) fn record_type(name: &str) -> Result<SchemaType, String> {
    FieldType::records(name).map(SchemaType::Is)
}

/// The record type `new.record`'s first argument names.
pub(crate) fn record_type_of(v: &LuaValue) -> Result<String, String> {
    match of(v) {
        Some(t) => t.record_type().map(str::to_string).ok_or_else(|| format!("a record type (`schema.record_type`), not a {}", t.describe())),
        None => Err(format!("a record type (`schema.record_type`), not {}", not_a_type(v))),
    }
}
