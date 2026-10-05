//! Content-declared state: each object kind and action declares a schema
//! of named, typed fields; the core stores the values as a
//! [`ContentState`] next to the object (or in the actor's attack state)
//! and snapshots them with the rest of the battle.
//!
//! Writes go through the field's type: integers wrap to the field's width
//! (as the game's `strb`/`strh`/`str` do), and a value of the wrong kind is
//! an error. So a script cannot put a fraction, a table or a string into
//! battle state, and a `u16` timer written with -1 reads back as 0xFFFF.
//!
//! A state is a fixed-size block of bytes ([`MAX_BYTES`]); the schema
//! decides where each field lives in it. That layout is private to this
//! module: content and engine code read and write fields by name (or by
//! the schema's field index), never by offset.

use std::fmt;

use crate::data::{Data, Key};
use crate::registry::Registry;
use crate::assets::AssetKind;
use crate::types::{ObjectRef, Pool, Vec3};

/// Bytes of state one kind or action may declare. (The game gives an
/// object 0x1C to 0x2C bytes of scratch and an action about 0x40; this
/// covers every kind ported so far with room to spare, and keeps a state a
/// small `Copy` value.)
pub const MAX_BYTES: usize = 64;

/// Most elements an array field may have.
pub const MAX_ARRAY: usize = 64;

/// The type of a state field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    /// A fixed number of elements of a scalar type (`"u8[18]"`): read and
    /// written element by element.
    Array(Box<FieldType>, u8),
    /// A definition of a registry, or none (`"kind"`, `"record"`,
    /// `"record:bomb-variant"`): docs/design/content-model-v2.md §3.4. A
    /// record field may name the records' type.
    Ref(Registry, Option<String>),
    /// An asset of a kind, or none (`"sprite"`, `"sound"`): §3.4.
    Asset(AssetKind),
}

impl FieldType {
    /// A type by name: `bool`, `u8`, `u16`, `u32`, `i8`, `i16`, `i32`,
    /// `object`, `vec3`, or an array of one of the scalars, `"u8[18]"`.
    pub fn scalar(name: &str) -> Option<FieldType> {
        if let Some((elem, len)) = name.strip_suffix(']').and_then(|s| s.split_once('[')) {
            let elem = FieldType::scalar(elem)?;
            let len: usize = len.parse().ok()?;
            if matches!(elem, FieldType::Array(..) | FieldType::Vec3) || !(1..=MAX_ARRAY).contains(&len) {
                return None;
            }
            return Some(FieldType::Array(Box::new(elem), len as u8));
        }
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
            _ if name.starts_with("record:") => {
                let t = &name["record:".len()..];
                return (!t.is_empty()).then(|| FieldType::Ref(Registry::Record, Some(t.to_string())));
            }
            _ if Registry::from_name(name).is_some_and(|r| !matches!(r, Registry::Schema)) => {
                FieldType::Ref(Registry::from_name(name).expect("a registry"), None)
            }
            _ if AssetKind::from_name(name).is_some() => FieldType::Asset(AssetKind::from_name(name).expect("an asset kind")),
            _ => return None,
        })
    }

    /// Bytes a value of this type takes in a state.
    pub fn size(&self) -> usize {
        match self {
            FieldType::Bool | FieldType::U8 | FieldType::I8 | FieldType::Object | FieldType::Enum(_) => 1,
            FieldType::U16 | FieldType::I16 | FieldType::OptionalU8 | FieldType::Ref(..) | FieldType::Asset(_) => 2,
            FieldType::U32 | FieldType::I32 => 4,
            FieldType::Vec3 => 12,
            FieldType::Array(elem, n) => elem.size() * *n as usize,
        }
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
            FieldType::Array(elem, _) => elem.zero(),
            FieldType::Ref(..) => FieldValue::Ref(None),
            FieldType::Asset(kind) => FieldValue::Asset(*kind, None),
        }
    }

    /// Convert `v` for storing in a field of this type: integers wrap to
    /// the width; anything else must match. (For an array, the type of
    /// one element.)
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
            (FieldType::Array(elem, _), v) => return elem.store(v),
            (FieldType::Ref(r, _), Value::Def(d, h)) if *r == d => FieldValue::Ref(Some((d, h))),
            (FieldType::Ref(..), Value::Nil) => FieldValue::Ref(None),
            (FieldType::Asset(k), Value::Asset(a, h)) if *k == a => FieldValue::Asset(a, Some(h)),
            (FieldType::Asset(k), Value::Nil) => FieldValue::Asset(*k, None),
            (ty, v) => return Err(TypeError { expected: ty.clone(), got: v }),
        })
    }

    /// Write a stored value (of this type) at the start of `out`.
    fn encode(&self, v: FieldValue, out: &mut [u8]) {
        match v {
            FieldValue::Bool(b) => out[0] = b as u8,
            FieldValue::U8(x) => out[0] = x,
            FieldValue::I8(x) => out[0] = x as u8,
            FieldValue::U16(x) => out[..2].copy_from_slice(&x.to_le_bytes()),
            FieldValue::I16(x) => out[..2].copy_from_slice(&x.to_le_bytes()),
            FieldValue::U32(x) => out[..4].copy_from_slice(&x.to_le_bytes()),
            FieldValue::I32(x) => out[..4].copy_from_slice(&x.to_le_bytes()),
            FieldValue::Object(o) => out[0] = o.map_or(0, |o| 0x80 | (o.pool as u8) << 5 | (o.slot & 0x1F)),
            FieldValue::Vec3(p) => {
                out[..4].copy_from_slice(&p.x.to_le_bytes());
                out[4..8].copy_from_slice(&p.y.to_le_bytes());
                out[8..12].copy_from_slice(&p.z.to_le_bytes());
            }
            FieldValue::Enum(i) => out[0] = i,
            FieldValue::OptionalU8(v) => {
                out[0] = v.is_some() as u8;
                out[1] = v.unwrap_or(0);
            }
            // The handle plus one; 0 is none.
            FieldValue::Ref(d) => out[..2].copy_from_slice(&d.map_or(0, |(_, h)| h.wrapping_add(1)).to_le_bytes()),
            FieldValue::Asset(_, h) => out[..2].copy_from_slice(&h.map_or(0, |h| h.wrapping_add(1)).to_le_bytes()),
        }
    }

    /// Read a value of this type (an array's element type) from the start
    /// of `b`.
    fn decode(&self, b: &[u8]) -> FieldValue {
        let i32_at = |k: usize| i32::from_le_bytes([b[k], b[k + 1], b[k + 2], b[k + 3]]);
        match self {
            FieldType::Bool => FieldValue::Bool(b[0] != 0),
            FieldType::U8 => FieldValue::U8(b[0]),
            FieldType::I8 => FieldValue::I8(b[0] as i8),
            FieldType::U16 => FieldValue::U16(u16::from_le_bytes([b[0], b[1]])),
            FieldType::I16 => FieldValue::I16(i16::from_le_bytes([b[0], b[1]])),
            FieldType::U32 => FieldValue::U32(i32_at(0) as u32),
            FieldType::I32 => FieldValue::I32(i32_at(0)),
            FieldType::Object => FieldValue::Object((b[0] & 0x80 != 0).then(|| ObjectRef {
                pool: Pool::ALL[((b[0] >> 5) & 3) as usize],
                slot: b[0] & 0x1F,
            })),
            FieldType::Vec3 => FieldValue::Vec3(Vec3 { x: i32_at(0), y: i32_at(4), z: i32_at(8) }),
            FieldType::Enum(_) => FieldValue::Enum(b[0]),
            FieldType::OptionalU8 => FieldValue::OptionalU8((b[0] != 0).then_some(b[1])),
            FieldType::Array(elem, _) => elem.decode(b),
            // (A list nothing has stated reads as holding nothing:
            // `LIST_UNSTATED` is no definition's.)
            FieldType::Ref(r, _) => {
                FieldValue::Ref(Some(u16::from_le_bytes([b[0], b[1]])).filter(|&v| v != LIST_UNSTATED).and_then(|v| v.checked_sub(1)).map(|h| (*r, h)))
            }
            FieldType::Asset(k) => FieldValue::Asset(*k, u16::from_le_bytes([b[0], b[1]]).checked_sub(1)),
        }
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
            FieldType::Array(elem, n) => write!(f, "{elem}[{n}]"),
            FieldType::Ref(r, None) => write!(f, "{r}"),
            FieldType::Ref(r, Some(t)) => write!(f, "{r}:{t}"),
            FieldType::Asset(k) => write!(f, "{k}"),
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
    /// A definition, by registry and handle (docs/design/content-model-v2.md
    /// §2): what a weapon's `setup` returns, a reference field's value.
    Def(Registry, u16),
    /// An asset, by kind and handle (§6.3).
    Asset(AssetKind, u16),
}

impl Value {
    pub fn int(self) -> Option<i64> {
        if let Value::Int(i) = self { Some(i) } else { None }
    }

    /// The definition this value is, if it is one.
    pub fn def(self) -> Option<(Registry, u16)> {
        if let Value::Def(r, h) = self { Some((r, h)) } else { None }
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
    /// A definition (registry and handle), or none.
    Ref(Option<(Registry, u16)>),
    /// An asset of a kind (its handle), or none.
    Asset(AssetKind, Option<u16>),
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
            FieldValue::Ref(d) => d.map_or(Value::Nil, |(r, h)| Value::Def(r, h)),
            FieldValue::Asset(k, h) => h.map_or(Value::Nil, |h| Value::Asset(k, h)),
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FieldDef {
    pub name: String,
    pub ty: FieldType,
}

/// The fields a kind's state declares, in storage order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Schema {
    fields: Vec<FieldDef>,
    /// Where each field starts in a state's bytes.
    offsets: Vec<u8>,
}

impl Schema {
    /// A schema from its fields; names must be unique identifiers and the
    /// fields must fit in [`MAX_BYTES`].
    pub fn new(fields: Vec<FieldDef>) -> Result<Schema, String> {
        let mut offsets = Vec::with_capacity(fields.len());
        let mut at = 0usize;
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
            offsets.push(at as u8);
            at += f.ty.size();
        }
        if at > MAX_BYTES {
            return Err(format!("the state's fields take {at} bytes; at most {MAX_BYTES} are allowed"));
        }
        Ok(Schema { fields, offsets })
    }

    /// A schema from a `state` table as data: field name to a type name
    /// (`"u16"`, `"u8[18]"`, `"object"`) or to a list of variant names (an
    /// enum). Fields are stored in name order.
    pub fn from_data(d: &Data) -> Result<Schema, String> {
        let Data::Map(entries) = d else {
            return match d {
                Data::List(l) if l.is_empty() => Schema::new(Vec::new()),
                _ => Err("a state is a table of field names to types".into()),
            };
        };
        let mut fields = Vec::with_capacity(entries.len());
        for (k, v) in entries {
            let Key::Str(name) = k else {
                return Err(format!("state field names are strings, not {k}"));
            };
            let ty = match v {
                Data::Str(t) => {
                    FieldType::scalar(t).ok_or_else(|| format!("state field `{name}` has unknown type {t:?}"))?
                }
                Data::List(variants) => FieldType::Enum(
                    variants
                        .iter()
                        .map(|v| v.str().map(str::to_string).ok_or_else(|| format!("state field `{name}`: variants are names")))
                        .collect::<Result<_, _>>()?,
                ),
                _ => return Err(format!("state field `{name}` needs a type name or a list of variants")),
            };
            fields.push(FieldDef { name: name.clone(), ty });
        }
        fields.sort_by(|a, b| a.name.cmp(&b.name));
        Schema::new(fields)
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

    fn at(&self, i: usize) -> usize {
        self.offsets[i] as usize
    }
}

/// What an enum field holds that nothing has stated
/// ([`ContentState::unstate`]): no variant's index.
pub const ENUM_UNSTATED: u8 = 0xFF;

/// What the first element of a list of definitions holds when nothing has
/// stated the list ([`ContentState::unstate`]): no definition's stored
/// value (a registry has fewer than 0xFFFE definitions), and not "none".
/// A stated list never holds it.
pub const LIST_UNSTATED: u16 = 0xFFFF;

/// Which schema a [`ContentState`] follows (an index into the content's
/// [`crate::Manifest`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StateId(pub u16);

/// The stored state of one object or action: the values of its schema's
/// fields, in a fixed block of bytes. A plain `Copy` value, so snapshots
/// copy it like any other engine state and the digest hashes it.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentState {
    id: StateId,
    bytes: [u8; MAX_BYTES],
}

impl ContentState {
    /// A zeroed state for the schema `id`. (Every field type's zero value
    /// is all zero bytes.)
    pub fn new(id: StateId) -> ContentState {
        ContentState { id, bytes: [0; MAX_BYTES] }
    }

    pub fn id(&self) -> StateId {
        self.id
    }

    /// Field `i` of `schema` (for an array, its first element).
    pub fn get(&self, schema: &Schema, i: usize) -> FieldValue {
        schema.field(i).ty.decode(&self.bytes[schema.at(i)..])
    }

    /// Leave field `i` stated by nobody, if it is an enum or a list of
    /// definitions (an array of them): an enum holds no variant
    /// ([`ENUM_UNSTATED`]) and a list is marked as no list at all
    /// ([`LIST_UNSTATED`]; it reads as empty) until something states one.
    /// For a player's setup, whose enums and lists of definitions have no
    /// default (a choice among named things is none of the engine's to
    /// make, and an empty list is a statement: no Crosses): a round doesn't
    /// start with one unstated. Any other field is left as it is.
    pub fn unstate(&mut self, schema: &Schema, i: usize) {
        let at = schema.at(i);
        match &schema.field(i).ty {
            FieldType::Enum(_) => self.bytes[at] = ENUM_UNSTATED,
            ty @ FieldType::Array(elem, _) if matches!(**elem, FieldType::Ref(..)) => {
                self.bytes[at..at + ty.size()].fill(0);
                self.bytes[at..at + 2].copy_from_slice(&LIST_UNSTATED.to_le_bytes());
            }
            _ => {}
        }
    }

    /// Whether field `i` is stated: an enum holds one of its variants, a
    /// list of definitions isn't marked unstated (true of any other field).
    pub fn stated(&self, schema: &Schema, i: usize) -> bool {
        let at = schema.at(i);
        match (&schema.field(i).ty, self.get(schema, i)) {
            (FieldType::Enum(names), FieldValue::Enum(v)) => (v as usize) < names.len(),
            (FieldType::Array(elem, _), _) if matches!(**elem, FieldType::Ref(..)) => {
                u16::from_le_bytes([self.bytes[at], self.bytes[at + 1]]) != LIST_UNSTATED
            }
            _ => true,
        }
    }

    /// Store `v` in field `i`, converted by the field's type.
    pub fn set(&mut self, schema: &Schema, i: usize, v: Value) -> Result<(), TypeError> {
        let ty = &schema.field(i).ty;
        if let FieldType::Array(..) = ty {
            return Err(TypeError { expected: ty.clone(), got: v });
        }
        let stored = ty.store(v)?;
        ty.encode(stored, &mut self.bytes[schema.at(i)..]);
        Ok(())
    }

    /// Element `k` of array field `i` (None past its end).
    pub fn get_elem(&self, schema: &Schema, i: usize, k: usize) -> Option<FieldValue> {
        let FieldType::Array(elem, n) = &schema.field(i).ty else { return None };
        (k < *n as usize).then(|| elem.decode(&self.bytes[schema.at(i) + k * elem.size()..]))
    }

    /// Store `v` in element `k` of array field `i`.
    pub fn set_elem(&mut self, schema: &Schema, i: usize, k: usize, v: Value) -> Result<(), String> {
        let FieldType::Array(elem, n) = &schema.field(i).ty else {
            return Err(format!("field `{}` is not an array", schema.field(i).name));
        };
        if k >= *n as usize {
            return Err(format!("index {} is past the end of `{}` ({n} elements)", k + 1, schema.field(i).name));
        }
        let stored = elem.store(v).map_err(|e| e.to_string())?;
        // (Writing an element states a list nothing had stated: it is then
        // the list of what is written, the rest empty.)
        if !self.stated(schema, i) {
            self.bytes[schema.at(i)..schema.at(i) + 2].fill(0);
        }
        elem.encode(stored, &mut self.bytes[schema.at(i) + k * elem.size()..]);
        Ok(())
    }
}

impl fmt::Debug for ContentState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let used = self.bytes.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
        f.debug_struct("ContentState").field("id", &self.id).field("bytes", &&self.bytes[..used]).finish()
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
            FieldDef { name: "offset".into(), ty: FieldType::Vec3 },
            FieldDef { name: "targets".into(), ty: FieldType::scalar("u8[6]").unwrap() },
        ])
        .unwrap()
    }

    #[test]
    fn integer_fields_wrap_to_their_width() {
        let s = schema();
        let mut st = ContentState::new(StateId(0));
        st.set(&s, 0, Value::Int(-1)).unwrap();
        assert_eq!(st.get(&s, 0), FieldValue::U16(0xFFFF));
        st.set(&s, 1, Value::Int(0x1F8)).unwrap();
        assert_eq!(st.get(&s, 1).load(), Value::Int(-8));
    }

    #[test]
    fn fields_read_back_what_was_stored() {
        let s = schema();
        let mut st = ContentState::new(StateId(0));
        let o = ObjectRef { pool: Pool::Effect, slot: 31 };
        st.set(&s, 3, Value::Object(o)).unwrap();
        let p = Vec3 { x: -1, y: 0x12_3456, z: i32::MIN };
        st.set(&s, 4, Value::Vec3(p)).unwrap();
        st.set(&s, 2, Value::Int(1)).unwrap();
        assert_eq!(st.get(&s, 3), FieldValue::Object(Some(o)));
        assert_eq!(st.get(&s, 4), FieldValue::Vec3(p));
        assert_eq!(st.get(&s, 2), FieldValue::Enum(1));
        assert_eq!(st.get(&s, 0), FieldValue::U16(0), "neighbors untouched");
        st.set(&s, 3, Value::Nil).unwrap();
        assert_eq!(st.get(&s, 3), FieldValue::Object(None));
    }

    #[test]
    fn arrays_are_read_and_written_by_element() {
        let s = schema();
        let mut st = ContentState::new(StateId(0));
        st.set_elem(&s, 5, 5, Value::Int(0x1FF)).unwrap();
        assert_eq!(st.get_elem(&s, 5, 5), Some(FieldValue::U8(0xFF)));
        assert_eq!(st.get_elem(&s, 5, 0), Some(FieldValue::U8(0)));
        assert_eq!(st.get_elem(&s, 5, 6), None);
        assert!(st.set_elem(&s, 5, 6, Value::Int(1)).is_err());
        assert!(st.set(&s, 5, Value::Int(1)).is_err(), "a whole array isn't one value");
    }

    #[test]
    fn wrong_kinds_and_bad_enum_values_are_errors() {
        let s = schema();
        let mut st = ContentState::new(StateId(0));
        assert!(st.set(&s, 0, Value::Bool(true)).is_err());
        assert!(st.set(&s, 2, Value::Int(2)).is_err());
        assert!(st.set(&s, 3, Value::Int(1)).is_err());
    }

    #[test]
    fn schemas_reject_duplicates_bad_names_and_oversize() {
        let f = |n: &str, ty: FieldType| FieldDef { name: n.into(), ty };
        assert!(Schema::new(vec![f("a", FieldType::U8), f("a", FieldType::U8)]).is_err());
        assert!(Schema::new(vec![f("1a", FieldType::U8)]).is_err());
        assert!(Schema::new((0..6).map(|i| f(&format!("v{i}"), FieldType::Vec3)).collect()).is_err());
        assert!(Schema::new((0..16).map(|i| f(&format!("f{i}"), FieldType::U32)).collect()).is_ok());
        assert!(FieldType::scalar("vec3[2]").is_none());
        assert!(FieldType::scalar("u8[0]").is_none());
    }
}
