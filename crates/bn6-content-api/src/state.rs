//! Content-declared state: each object kind and action declares a schema
//! of named, typed fields; the core stores the values as a
//! [`ContentState`] next to the object (or in the actor's attack state)
//! and snapshots them with the rest of the battle.
//!
//! Writes go through the field's type: integers wrap to the field's width
//! (as the game's `strb`/`strh`/`str` do), and a value of the wrong kind is
//! an error. So a script cannot put a fraction, a table or a string into
//! battle state, and a `u16` timer written with -1 reads back as 0xFFFF.

use std::fmt;

use crate::types::{ObjectRef, Vec3};

/// Most fields one kind's state may declare. (The game gives an object
/// 0x1C to 0x2C bytes of scratch; eight typed fields cover every kind
/// ported so far.)
pub const MAX_FIELDS: usize = 8;

/// The type of a state field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldType {
    Bool,
    U8,
    U16,
    U32,
    I8,
    I16,
    I32,
    /// A handle to another object, or none.
    Object,
    /// A 16.16 fixed-point vector.
    Vec3,
    /// One of the named variants (stored as its index).
    Enum(Vec<String>),
    /// A byte, or none (engine fields only, e.g. a sprite's alpha).
    OptionalU8,
}

impl FieldType {
    /// A scalar type by name: `bool`, `u8`, `u16`, `u32`, `i8`, `i16`,
    /// `i32`, `object`, `vec3`.
    pub fn scalar(name: &str) -> Option<FieldType> {
        Some(match name {
            "bool" => FieldType::Bool,
            "u8" => FieldType::U8,
            "u16" => FieldType::U16,
            "u32" => FieldType::U32,
            "i8" => FieldType::I8,
            "i16" => FieldType::I16,
            "i32" => FieldType::I32,
            "object" => FieldType::Object,
            "vec3" => FieldType::Vec3,
            _ => return None,
        })
    }

    /// The value a fresh state holds (all zero, like the game's scratch).
    pub fn zero(&self) -> FieldValue {
        match self {
            FieldType::Bool => FieldValue::Bool(false),
            FieldType::U8 => FieldValue::U8(0),
            FieldType::U16 => FieldValue::U16(0),
            FieldType::U32 => FieldValue::U32(0),
            FieldType::I8 => FieldValue::I8(0),
            FieldType::I16 => FieldValue::I16(0),
            FieldType::I32 => FieldValue::I32(0),
            FieldType::Object => FieldValue::Object(None),
            FieldType::Vec3 => FieldValue::Vec3(Vec3::default()),
            FieldType::Enum(_) => FieldValue::Enum(0),
            FieldType::OptionalU8 => FieldValue::OptionalU8(None),
        }
    }

    /// Convert `v` for storing in a field of this type: integers wrap to
    /// the width; anything else must match.
    pub fn store(&self, v: Value) -> Result<FieldValue, TypeError> {
        Ok(match (self, v) {
            (FieldType::Bool, Value::Bool(b)) => FieldValue::Bool(b),
            (FieldType::U8, Value::Int(i)) => FieldValue::U8(i as u8),
            (FieldType::U16, Value::Int(i)) => FieldValue::U16(i as u16),
            (FieldType::U32, Value::Int(i)) => FieldValue::U32(i as u32),
            (FieldType::I8, Value::Int(i)) => FieldValue::I8(i as i8),
            (FieldType::I16, Value::Int(i)) => FieldValue::I16(i as i16),
            (FieldType::I32, Value::Int(i)) => FieldValue::I32(i as i32),
            (FieldType::Object, Value::Object(o)) => FieldValue::Object(Some(o)),
            (FieldType::Object, Value::Nil) => FieldValue::Object(None),
            (FieldType::Vec3, Value::Vec3(p)) => FieldValue::Vec3(p),
            (FieldType::Enum(names), Value::Int(i)) if (0..names.len() as i64).contains(&i) => {
                FieldValue::Enum(i as u8)
            }
            (FieldType::OptionalU8, Value::Int(i)) => FieldValue::OptionalU8(Some(i as u8)),
            (FieldType::OptionalU8, Value::Nil) => FieldValue::OptionalU8(None),
            (ty, v) => return Err(TypeError { expected: ty.clone(), got: v }),
        })
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FieldType::Bool => f.write_str("bool"),
            FieldType::U8 => f.write_str("u8"),
            FieldType::U16 => f.write_str("u16"),
            FieldType::U32 => f.write_str("u32"),
            FieldType::I8 => f.write_str("i8"),
            FieldType::I16 => f.write_str("i16"),
            FieldType::I32 => f.write_str("i32"),
            FieldType::Object => f.write_str("object"),
            FieldType::Vec3 => f.write_str("vec3"),
            FieldType::Enum(names) => write!(f, "enum {}", names.join(" | ")),
            FieldType::OptionalU8 => f.write_str("u8?"),
        }
    }
}

/// A value crossing the content API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    Nil,
    Bool(bool),
    /// Any integer; fields wrap it to their width.
    Int(i64),
    Object(ObjectRef),
    Vec3(Vec3),
}

impl Value {
    pub fn int(self) -> Option<i64> {
        if let Value::Int(i) = self { Some(i) } else { None }
    }

    pub fn object(self) -> Option<ObjectRef> {
        if let Value::Object(o) = self { Some(o) } else { None }
    }
}

impl From<Option<ObjectRef>> for Value {
    fn from(o: Option<ObjectRef>) -> Value {
        o.map_or(Value::Nil, Value::Object)
    }
}

/// A stored field value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldValue {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    I8(i8),
    I16(i16),
    I32(i32),
    Object(Option<ObjectRef>),
    Vec3(Vec3),
    Enum(u8),
    OptionalU8(Option<u8>),
}

impl FieldValue {
    pub fn load(self) -> Value {
        match self {
            FieldValue::Bool(b) => Value::Bool(b),
            FieldValue::U8(v) => Value::Int(v as i64),
            FieldValue::U16(v) => Value::Int(v as i64),
            FieldValue::U32(v) => Value::Int(v as i64),
            FieldValue::I8(v) => Value::Int(v as i64),
            FieldValue::I16(v) => Value::Int(v as i64),
            FieldValue::I32(v) => Value::Int(v as i64),
            FieldValue::Object(o) => o.into(),
            FieldValue::Vec3(p) => Value::Vec3(p),
            FieldValue::Enum(i) => Value::Int(i as i64),
            FieldValue::OptionalU8(v) => v.map_or(Value::Nil, |v| Value::Int(v as i64)),
        }
    }
}

/// A value of the wrong kind for its field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeError {
    pub expected: FieldType,
    pub got: Value,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "expected {}, got {:?}", self.expected, self.got)
    }
}

impl std::error::Error for TypeError {}

/// One declared field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDef {
    pub name: String,
    pub ty: FieldType,
}

/// The fields a kind's state declares, in storage order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schema {
    fields: Vec<FieldDef>,
}

impl Schema {
    /// A schema from its fields; names must be unique identifiers and
    /// there can be at most [`MAX_FIELDS`].
    pub fn new(fields: Vec<FieldDef>) -> Result<Schema, String> {
        if fields.len() > MAX_FIELDS {
            return Err(format!("{} state fields; at most {MAX_FIELDS} are allowed", fields.len()));
        }
        for (i, f) in fields.iter().enumerate() {
            let ident = f.name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && f.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !ident {
                return Err(format!("state field name {:?} is not an identifier", f.name));
            }
            if fields[..i].iter().any(|g| g.name == f.name) {
                return Err(format!("state field {:?} is declared twice", f.name));
            }
            if let FieldType::Enum(names) = &f.ty
                && (names.is_empty() || names.len() > 256)
            {
                return Err(format!("state field {:?}: an enum needs 1 to 256 variants", f.name));
            }
        }
        Ok(Schema { fields })
    }

    pub fn fields(&self) -> &[FieldDef] {
        &self.fields
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }

    pub fn field(&self, i: usize) -> &FieldDef {
        &self.fields[i]
    }
}

/// Which schema a [`ContentState`] follows (an index into the content's
/// [`crate::Manifest`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StateId(pub u16);

/// The stored state of one object or action: the values of its schema's
/// fields. A plain `Copy` value, so snapshots copy it like any other
/// engine state.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentState {
    id: StateId,
    len: u8,
    values: [FieldValue; MAX_FIELDS],
}

impl ContentState {
    /// A zeroed state for `schema`.
    pub fn new(id: StateId, schema: &Schema) -> ContentState {
        let mut values = [FieldValue::Bool(false); MAX_FIELDS];
        for (v, f) in values.iter_mut().zip(schema.fields()) {
            *v = f.ty.zero();
        }
        ContentState { id, len: schema.fields().len() as u8, values }
    }

    pub fn id(&self) -> StateId {
        self.id
    }

    pub fn values(&self) -> &[FieldValue] {
        &self.values[..self.len as usize]
    }

    pub fn get(&self, i: usize) -> FieldValue {
        self.values()[i]
    }

    /// Store `v` in field `i`, converted by the field's type.
    pub fn set(&mut self, schema: &Schema, i: usize, v: Value) -> Result<(), TypeError> {
        self.values[i] = schema.field(i).ty.store(v)?;
        Ok(())
    }

    /// Replace field `i` with a stored value of the same type (for typed
    /// Rust views, which convert exactly).
    pub fn replace(&mut self, i: usize, v: FieldValue) {
        let old = &mut self.values[i];
        assert_eq!(std::mem::discriminant(old), std::mem::discriminant(&v), "field {i} changes type");
        *old = v;
    }
}

/// A Rust type a typed state field can have.
pub trait StateField: Sized {
    fn field_type() -> FieldType;
    fn from_field(v: FieldValue) -> Self;
    fn to_field(&self) -> FieldValue;
}

macro_rules! state_field {
    ($($t:ty => $ty:ident),*) => {$(
        impl StateField for $t {
            fn field_type() -> FieldType {
                FieldType::$ty
            }
            fn from_field(v: FieldValue) -> $t {
                match v {
                    FieldValue::$ty(x) => x,
                    v => panic!("expected {}, got {v:?}", stringify!($ty)),
                }
            }
            fn to_field(&self) -> FieldValue {
                FieldValue::$ty(*self)
            }
        }
    )*};
}
state_field!(bool => Bool, u8 => U8, u16 => U16, u32 => U32, i8 => I8, i16 => I16, i32 => I32, Vec3 => Vec3);

impl StateField for Option<ObjectRef> {
    fn field_type() -> FieldType {
        FieldType::Object
    }
    fn from_field(v: FieldValue) -> Option<ObjectRef> {
        match v {
            FieldValue::Object(x) => x,
            v => panic!("expected an object, got {v:?}"),
        }
    }
    fn to_field(&self) -> FieldValue {
        FieldValue::Object(*self)
    }
}

/// A Rust struct viewing a [`ContentState`]: Rust content declares its
/// state with [`content_state!`](crate::content_state) and copies it in and
/// out.
pub trait TypedState: Sized {
    fn schema() -> Schema;
    fn load(s: &ContentState) -> Self;
    fn store(&self, s: &mut ContentState);
}

/// Declare a Rust content kind's state: a plain struct whose fields become
/// the kind's schema (in order), with [`TypedState`] to copy it from and to
/// the engine's [`ContentState`].
#[macro_export]
macro_rules! content_state {
    ($(#[$m:meta])* $vis:vis struct $name:ident { $($(#[$fm:meta])* $f:ident : $t:ty),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        $vis struct $name { $($(#[$fm])* pub $f: $t,)* }

        impl $crate::state::TypedState for $name {
            fn schema() -> $crate::Schema {
                $crate::Schema::new(vec![$($crate::FieldDef {
                    name: stringify!($f).to_string(),
                    ty: <$t as $crate::state::StateField>::field_type(),
                },)*]).expect("a valid state schema")
            }
            #[allow(unused_assignments)]
            fn load(s: &$crate::ContentState) -> Self {
                let mut i = 0;
                $name { $($f: { let v = <$t as $crate::state::StateField>::from_field(s.get(i)); i += 1; v },)* }
            }
            #[allow(unused_assignments)]
            fn store(&self, s: &mut $crate::ContentState) {
                let mut i = 0;
                $( s.replace(i, $crate::state::StateField::to_field(&self.$f)); i += 1; )*
            }
        }
    };
}

impl fmt::Debug for ContentState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ContentState").field("id", &self.id).field("values", &self.values()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Schema {
        Schema::new(vec![
            FieldDef { name: "timer".into(), ty: FieldType::U16 },
            FieldDef { name: "lift".into(), ty: FieldType::I8 },
            FieldDef { name: "slot".into(), ty: FieldType::Enum(vec!["overlay".into(), "related".into()]) },
            FieldDef { name: "owner".into(), ty: FieldType::Object },
        ])
        .unwrap()
    }

    #[test]
    fn integer_fields_wrap_to_their_width() {
        let s = schema();
        let mut st = ContentState::new(StateId(0), &s);
        st.set(&s, 0, Value::Int(-1)).unwrap();
        assert_eq!(st.get(0), FieldValue::U16(0xFFFF));
        st.set(&s, 1, Value::Int(0x1F8)).unwrap();
        assert_eq!(st.get(1).load(), Value::Int(-8));
    }

    #[test]
    fn wrong_kinds_and_bad_enum_values_are_errors() {
        let s = schema();
        let mut st = ContentState::new(StateId(0), &s);
        assert!(st.set(&s, 0, Value::Bool(true)).is_err());
        assert!(st.set(&s, 2, Value::Int(2)).is_err());
        assert!(st.set(&s, 3, Value::Int(1)).is_err());
        st.set(&s, 3, Value::Nil).unwrap();
        assert_eq!(st.get(3).load(), Value::Nil);
    }

    #[test]
    fn schemas_reject_duplicates_and_bad_names() {
        let f = |n: &str| FieldDef { name: n.into(), ty: FieldType::U8 };
        assert!(Schema::new(vec![f("a"), f("a")]).is_err());
        assert!(Schema::new(vec![f("1a")]).is_err());
        assert!(Schema::new((0..9).map(|i| f(&format!("f{i}"))).collect()).is_err());
    }
}
