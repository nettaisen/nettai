//! Content-declared state: each object kind and action declares a schema
//! of named, typed fields; the core stores the values in a [`StateArena`]
//! by the object's slot (or the actor's, for its attack state) and
//! snapshots them with the rest of the battle.
//!
//! Writes go through the field's type: integers wrap to the field's width
//! (as the game's `strb`/`strh`/`str` do), and a value of the wrong kind is
//! an error. So a script cannot put a fraction, a table or a string into
//! battle state, and a `u16` timer written with -1 reads back as 0xFFFF.
//!
//! Every state is a block of its schema's own size, whatever that is: an
//! object's or an action's in its pool's arena ([`StateArena`], read and
//! written through a [`StateRef`] or [`StateMut`]), a game's rules' state
//! of a side and a player's setup of them on their own ([`Block`]). The
//! schema decides where each field lives in a block. That layout is
//! private to this module: content and engine code read and write fields
//! by name (or by the schema's field index), never by offset.
//!
//! A field may be a record of fields of its own (`{ chip = "chip", code =
//! "code" }`) or a bounded list of a type (`schema.list(T, n)`: how many it
//! holds, then room for `n`), nested as deep as a schema says. A part of a
//! block is reached through its [`Place`]: a field's, a record's field's,
//! an element's.

use std::fmt;

use crate::data::{Data, Key};
use crate::registry::Registry;
use crate::assets::AssetKind;
use crate::types::{ObjectRef, Pool, Vec3};

/// Most elements an array field may have.
pub const MAX_ARRAY: usize = 64;

/// Most elements a list may hold (`schema.list(T, n)`).
pub const MAX_LIST: usize = u16::MAX as usize;

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
    /// A byte, or none (a sprite's alpha; a setup's level, `u8?`).
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
    /// A chip code (`"code"`): a letter A to Z or `*`, or none.
    Code,
    /// A record: fields of their own, in their names' order (a table of
    /// field names to types).
    Record(Box<Schema>),
    /// A list of up to `n` elements of a type, and how many it holds
    /// (`schema.list(T, n)`): read and written element by element, its
    /// length its own.
    List(Box<FieldType>, u16),
}

impl FieldType {
    /// A type by name: `bool`, `u8`, `u16`, `u32`, `i8`, `i16`, `i32`,
    /// `u8?` (a byte or none: a setup's fact that may be stated as none, as
    /// a navi code's level), `object`, `vec3`, `code` (a chip code), a
    /// registry's or an asset kind's name, or an array of one of the
    /// scalars, `"u8[18]"`.
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
            "u8?" => FieldType::OptionalU8,
            "u16" => FieldType::U16,
            "u32" => FieldType::U32,
            "i8" => FieldType::I8,
            "i16" => FieldType::I16,
            "i32" => FieldType::I32,
            "object" => FieldType::Object,
            "vec3" => FieldType::Vec3,
            "code" => FieldType::Code,
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
            FieldType::Bool | FieldType::U8 | FieldType::I8 | FieldType::Object | FieldType::Enum(_) | FieldType::Code => 1,
            FieldType::U16 | FieldType::I16 | FieldType::OptionalU8 | FieldType::Ref(..) | FieldType::Asset(_) => 2,
            FieldType::U32 | FieldType::I32 => 4,
            FieldType::Vec3 => 12,
            FieldType::Array(elem, n) => elem.size() * *n as usize,
            FieldType::Record(fields) => fields.size(),
            FieldType::List(elem, n) => list_count_size(*n) + elem.size() * *n as usize,
        }
    }

    /// Whether a value of this type is one value (not a record, a list or
    /// an array, whose parts are read and written each by its place).
    pub fn is_scalar(&self) -> bool {
        !matches!(self, FieldType::Array(..) | FieldType::Record(_) | FieldType::List(..))
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
            FieldType::Code => FieldValue::Code(None),
            FieldType::Record(_) | FieldType::List(..) => FieldValue::Nested,
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
            (FieldType::Code, Value::Code(c)) if is_code(c) => FieldValue::Code(Some(c)),
            (FieldType::Code, Value::Nil) => FieldValue::Code(None),
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
            // (The letter itself; 0 is none.)
            FieldValue::Code(c) => out[0] = c.unwrap_or(0),
            // (A record's or a list's parts are written each by its place.)
            FieldValue::Nested => {}
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
            FieldType::Ref(r, _) => FieldValue::Ref(u16::from_le_bytes([b[0], b[1]]).checked_sub(1).map(|h| (*r, h))),
            FieldType::Asset(k) => FieldValue::Asset(*k, u16::from_le_bytes([b[0], b[1]]).checked_sub(1)),
            FieldType::Code => FieldValue::Code((b[0] != 0).then_some(b[0])),
            FieldType::Record(_) | FieldType::List(..) => FieldValue::Nested,
        }
    }
}

/// Whether `c` is a chip code: a letter A to Z, or `*`.
pub fn is_code(c: u8) -> bool {
    c.is_ascii_uppercase() || c == b'*'
}

/// The bytes a list of up to `n` elements counts them in.
fn list_count_size(n: u16) -> usize {
    if n <= u8::MAX as u16 { 1 } else { 2 }
}

/// The key a list's declaration (`schema.list(T, n)`) marks its table with.
pub const LIST_MARK: &str = "__list";

impl FieldType {
    /// A field's type as data: a type name, a list of variant names (an
    /// enum), a table of fields (a record), or a list's declaration.
    pub fn from_data(d: &Data) -> Result<FieldType, String> {
        Ok(match d {
            Data::Str(t) => FieldType::scalar(t).ok_or_else(|| format!("unknown type {t:?}"))?,
            Data::List(variants) => FieldType::Enum(
                variants.iter().map(|v| v.str().map(str::to_string).ok_or_else(|| "variants are names".to_string())).collect::<Result<_, _>>()?,
            ),
            Data::Map(_) if *d.field(LIST_MARK) == Data::Bool(true) => {
                let max = d.field("max").int().ok_or("a list's `max` is a whole number")?;
                if !(1..=MAX_LIST as i64).contains(&max) {
                    return Err(format!("a list holds 1 to {MAX_LIST} elements, not {max}"));
                }
                let of = FieldType::from_data(d.field("of")).map_err(|e| format!("a list's elements: {e}"))?;
                FieldType::List(Box::new(of), max as u16)
            }
            Data::Map(_) => FieldType::Record(Box::new(Schema::from_data(d)?)),
            _ => return Err("needs a type name, a list of variants, a table of fields or a list (`schema.list`)".into()),
        })
    }
}

/// Where a part of a block is: its type and its offset. A schema's field's
/// ([`Schema::place`]), a record's field's ([`Place::field`]), an array's
/// or a list's element's ([`Place::elem`]).
#[derive(Clone, Copy, Debug)]
pub struct Place<'s> {
    ty: &'s FieldType,
    at: usize,
}

impl<'s> Place<'s> {
    pub fn ty(self) -> &'s FieldType {
        self.ty
    }

    /// A record's fields' schema (none: not a record).
    pub fn record(self) -> Option<&'s Schema> {
        match self.ty {
            FieldType::Record(fields) => Some(fields),
            _ => None,
        }
    }

    /// A record's field `name`.
    pub fn field(self, name: &str) -> Option<Place<'s>> {
        let fields = self.record()?;
        let i = fields.index_of(name)?;
        Some(self.field_at(i))
    }

    /// A record's field `i` (by its schema's index).
    pub fn field_at(self, i: usize) -> Place<'s> {
        let fields = self.record().expect("a record");
        Place { ty: &fields.field(i).ty, at: self.at + fields.at(i) }
    }

    /// Element `k` of an array or of a list (within its room: whether it is
    /// one the list holds is its length's).
    pub fn elem(self, k: usize) -> Option<Place<'s>> {
        match self.ty {
            FieldType::Array(elem, n) => (k < *n as usize).then(|| Place { ty: elem, at: self.at + k * elem.size() }),
            FieldType::List(elem, n) => {
                (k < *n as usize).then(|| Place { ty: elem, at: self.at + list_count_size(*n) + k * elem.size() })
            }
            _ => None,
        }
    }

    /// How many elements an array has, or a list has room for.
    pub fn capacity(self) -> Option<usize> {
        match self.ty {
            FieldType::Array(_, n) => Some(*n as usize),
            FieldType::List(_, n) => Some(*n as usize),
            _ => None,
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
            FieldType::Code => f.write_str("code"),
            FieldType::Record(fields) => {
                f.write_str("{ ")?;
                for (i, d) in fields.fields().iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}: {}", d.name, d.ty)?;
                }
                f.write_str(" }")
            }
            FieldType::List(elem, n) => write!(f, "list of {n} {elem}"),
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
    /// A chip code: its letter (`b'A'` to `b'Z'`, or `b'*'`).
    Code(u8),
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
    /// A chip code's letter, or none.
    Code(Option<u8>),
    /// A record or a list: read its parts by their places.
    Nested,
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
            FieldValue::Code(c) => c.map_or(Value::Nil, Value::Code),
            FieldValue::Nested => Value::Nil,
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
    offsets: Vec<usize>,
    /// The bytes its fields take.
    size: usize,
}

impl Schema {
    /// A schema from its fields; names must be unique identifiers. Its size
    /// is what its fields take, whatever that is.
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
            offsets.push(at);
            at += f.ty.size();
        }
        Ok(Schema { fields, offsets, size: at })
    }

    /// The bytes its fields take.
    pub fn size(&self) -> usize {
        self.size
    }

    /// A schema from a `state` table as data: field name to a type name
    /// (`"u16"`, `"u8[18]"`, `"object"`, `"code"`), to a list of variant
    /// names (an enum), to a table of fields (a record) or to a list
    /// (`schema.list(T, n)`, a table of [`LIST_MARK`], `of` and `max`).
    /// Fields are stored in name order.
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
            let ty = FieldType::from_data(v).map_err(|e| format!("state field `{name}`: {e}"))?;
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
        self.offsets[i]
    }

    /// Where field `i` is in a block of this schema.
    pub fn place(&self, i: usize) -> Place<'_> {
        Place { ty: &self.fields[i].ty, at: self.offsets[i] }
    }

    /// The field named `name`: one of this schema's, else one of its
    /// records', at any depth ([`FieldPath::DEPTH`]), the one field of the
    /// name there is (several: an error naming their paths). For a reader
    /// that knows a field by its name wherever the content keeps it (the
    /// rules' views).
    pub fn find(&self, name: &str) -> Result<Option<FieldPath>, String> {
        fn walk(schema: &Schema, name: &str, at: FieldPath, out: &mut Vec<FieldPath>) {
            for (i, f) in schema.fields.iter().enumerate() {
                let Some(here) = at.then(i) else { continue };
                if f.name == name {
                    out.push(here);
                }
                if let FieldType::Record(fields) = &f.ty {
                    walk(fields, name, here, out);
                }
            }
        }
        let mut found = Vec::new();
        walk(self, name, FieldPath::default(), &mut found);
        match found[..] {
            [] => Ok(None),
            [one] => Ok(Some(one)),
            _ => Err(format!(
                "`{name}` is {}",
                found.iter().map(|p| format!("`{}`", self.path_name(*p))).collect::<Vec<_>>().join(" and ")
            )),
        }
    }

    /// Where the field at `path` is in a block of this schema.
    pub fn place_of(&self, path: FieldPath) -> Place<'_> {
        let steps = &path.steps[..path.len as usize];
        let mut place = self.place(steps[0] as usize);
        for &i in &steps[1..] {
            place = place.field_at(i as usize);
        }
        place
    }

    /// The field at `path` as a message names it: `screen.mix_step`.
    pub fn path_name(&self, path: FieldPath) -> String {
        let mut schema = self;
        let mut out = Vec::new();
        for &i in &path.steps[..path.len as usize] {
            let f = schema.field(i as usize);
            out.push(f.name.clone());
            if let FieldType::Record(fields) = &f.ty {
                schema = fields;
            }
        }
        out.join(".")
    }
}

/// A field by its place in a schema's records: a field's index, then its
/// record's field's, as deep as [`FieldPath::DEPTH`] (`Schema::find`,
/// `Schema::place_of`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FieldPath {
    steps: [u16; FieldPath::DEPTH],
    len: u8,
}

impl FieldPath {
    /// The most records a path goes through, and one.
    pub const DEPTH: usize = 4;

    /// A schema's field `i`, at the top.
    pub fn top(i: usize) -> FieldPath {
        FieldPath::default().then(i).expect("a path's first step")
    }

    /// This path's record's field `i` (none: past [`FieldPath::DEPTH`]).
    fn then(self, i: usize) -> Option<FieldPath> {
        let mut out = self;
        *out.steps.get_mut(out.len as usize)? = i as u16;
        out.len += 1;
        Some(out)
    }
}

/// The field codec of a state's bytes (`self.bytes`), to read: what
/// [`StateRef`], [`StateMut`] and [`Block`] read by.
macro_rules! codec_read {
    () => {
    /// Field `i` of `schema` (for an array, its first element).
    pub fn get(&self, schema: &Schema, i: usize) -> FieldValue {
        schema.field(i).ty.decode(&self.bytes[schema.at(i)..])
    }

    /// Whether field `i` is stated: an enum holds one of its variants (true
    /// of any other field).
    pub fn stated(&self, schema: &Schema, i: usize) -> bool {
        match (&schema.field(i).ty, self.get(schema, i)) {
            (FieldType::Enum(names), FieldValue::Enum(v)) => (v as usize) < names.len(),
            _ => true,
        }
    }

    /// Element `k` of array field `i` (None past its end).
    pub fn get_elem(&self, schema: &Schema, i: usize, k: usize) -> Option<FieldValue> {
        let FieldType::Array(elem, n) = &schema.field(i).ty else { return None };
        (k < *n as usize).then(|| elem.decode(&self.bytes[schema.at(i) + k * elem.size()..]))
    }

    /// The value at `place` (a record or a list: [`FieldValue::Nested`],
    /// whose parts have places of their own).
    pub fn get_at(&self, place: Place) -> FieldValue {
        place.ty.decode(&self.bytes[place.at..])
    }

    /// How many elements the list at `place` holds (an array: all of them).
    pub fn len_at(&self, place: Place) -> Option<usize> {
        match place.ty {
            FieldType::Array(_, n) => Some(*n as usize),
            FieldType::List(_, n) => Some(match list_count_size(*n) {
                1 => self.bytes[place.at] as usize,
                _ => u16::from_le_bytes([self.bytes[place.at], self.bytes[place.at + 1]]) as usize,
            }),
            _ => None,
        }
    }
    };
}

/// The field codec of a state's bytes, to write: what [`StateMut`] and
/// [`Block`] write by.
macro_rules! codec_write {
    () => {
    /// Leave field `i` stated by nobody, if it is an enum: it holds no
    /// variant ([`ENUM_UNSTATED`]) until something states one. For a
    /// player's setup, whose enums have no default (a choice among named
    /// things is none of the engine's to make): a round doesn't start with
    /// one unstated. Any other field is left as it is (a list left out is
    /// empty).
    pub fn unstate(&mut self, schema: &Schema, i: usize) {
        if let FieldType::Enum(_) = &schema.field(i).ty {
            self.bytes[schema.at(i)] = ENUM_UNSTATED;
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

    /// Store `v` in element `k` of array field `i`.
    pub fn set_elem(&mut self, schema: &Schema, i: usize, k: usize, v: Value) -> Result<(), String> {
        let FieldType::Array(elem, n) = &schema.field(i).ty else {
            return Err(format!("field `{}` is not an array", schema.field(i).name));
        };
        if k >= *n as usize {
            return Err(format!("index {} is past the end of `{}` ({n} elements)", k + 1, schema.field(i).name));
        }
        let stored = elem.store(v).map_err(|e| e.to_string())?;
        elem.encode(stored, &mut self.bytes[schema.at(i) + k * elem.size()..]);
        Ok(())
    }

    /// Store `v` at `place`, converted by its type (one value: not a
    /// record's, a list's or an array's whole).
    pub fn set_at(&mut self, place: Place, v: Value) -> Result<(), TypeError> {
        if !place.ty.is_scalar() {
            return Err(TypeError { expected: place.ty.clone(), got: v });
        }
        let stored = place.ty.store(v)?;
        place.ty.encode(stored, &mut self.bytes[place.at..]);
        Ok(())
    }

    /// Zero the part at `place`, whole (a record's every field, a list's
    /// length and elements): as a fresh state has it.
    pub fn clear_at(&mut self, place: Place) {
        self.bytes[place.at..place.at + place.ty.size()].fill(0);
    }

    /// Make the list at `place` hold `n` elements: the ones past `n` are
    /// zeroed (so a list's bytes are what its elements say), the ones it
    /// gains zero.
    pub fn set_len_at(&mut self, place: Place, n: usize) -> Result<(), String> {
        let FieldType::List(elem, max) = place.ty else { return Err(format!("a {} is not a list", place.ty)) };
        if n > *max as usize {
            return Err(format!("a list of {max} holds no {n}"));
        }
        let count = list_count_size(*max);
        let items = place.at + count;
        self.bytes[items + n * elem.size()..items + *max as usize * elem.size()].fill(0);
        match count {
            1 => self.bytes[place.at] = n as u8,
            _ => self.bytes[place.at..place.at + 2].copy_from_slice(&(n as u16).to_le_bytes()),
        }
        Ok(())
    }
    };
}

/// What an enum field holds that nothing has stated
/// ([`Block::unstate`]): no variant's index.
pub const ENUM_UNSTATED: u8 = 0xFF;

/// Which schema a state follows (an index into the content's
/// [`crate::Manifest`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StateId(pub u16);

/// The states of a pool of slots (an object pool's, or the actors' attack
/// states): each slot's block, of the size of the layout it last took,
/// packed in slot order in one run of bytes. A copy of the arena (a
/// snapshot's) carries no unused bytes, and no slot that never took a
/// state has any.
///
/// A slot's block is replaced only when the slot takes a state
/// ([`StateArena::reset`]: zeroed, of the new layout's size, the blocks
/// above it moved by the difference). Freeing an object leaves its block as
/// it is, so its state stays readable until the slot is taken again, as the
/// game's memory does. The bytes follow from what each slot last took, so
/// two battles that ran the same ticks have the same arena, and the
/// arena's `Hash` (the digest's) covers every block and its slot.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateArena<const SLOTS: usize> {
    bytes: Vec<u8>,
    /// Each slot's block's size, in slot order (where a block starts is the
    /// sum of the sizes below it).
    sizes: [u32; SLOTS],
}

impl<const SLOTS: usize> Default for StateArena<SLOTS> {
    fn default() -> StateArena<SLOTS> {
        StateArena { bytes: Vec::new(), sizes: [0; SLOTS] }
    }
}

impl<const SLOTS: usize> StateArena<SLOTS> {
    /// Where `slot`'s block is in the bytes.
    fn span(&self, slot: usize) -> std::ops::Range<usize> {
        let at: usize = self.sizes[..slot].iter().map(|&n| n as usize).sum();
        at..at + self.sizes[slot] as usize
    }

    /// Give `slot` a zeroed block of `size` bytes in place of the one it
    /// had (an object's spawn, a state of another layout; `size` 0: none).
    /// Every field type's zero value is all zero bytes.
    pub fn reset(&mut self, slot: usize, size: usize) {
        let span = self.span(slot);
        self.bytes.splice(span, std::iter::repeat_n(0, size));
        self.sizes[slot] = u32::try_from(size).expect("a state's size fits in 32 bits");
    }

    /// The size of `slot`'s block.
    pub fn size_of(&self, slot: usize) -> usize {
        self.sizes[slot] as usize
    }

    /// The bytes every block takes.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }

    /// `slot`'s block as a state of layout `id`, to read. (Its schema is
    /// the one the block was sized for.)
    pub fn state(&self, slot: usize, id: StateId) -> StateRef<'_> {
        StateRef { id, bytes: &self.bytes[self.span(slot)] }
    }

    /// `slot`'s block as a state of layout `id`, to read and write.
    pub fn state_mut(&mut self, slot: usize, id: StateId) -> StateMut<'_> {
        let span = self.span(slot);
        StateMut { id, bytes: &mut self.bytes[span] }
    }
}

/// One state in a [`StateArena`], to read: its layout and its block.
#[derive(Clone, Copy, Debug)]
pub struct StateRef<'a> {
    id: StateId,
    bytes: &'a [u8],
}

impl StateRef<'_> {
    pub fn id(&self) -> StateId {
        self.id
    }

    codec_read!();
}

/// One state in a [`StateArena`], to read and write: its layout and its
/// block.
#[derive(Debug)]
pub struct StateMut<'a> {
    id: StateId,
    bytes: &'a mut [u8],
}

impl StateMut<'_> {
    pub fn id(&self) -> StateId {
        self.id
    }

    codec_read!();
    codec_write!();
}

/// A game's rules' state of a side, or a player's setup of them: the
/// values of a schema's fields in a block of the schema's own size. Cloned with the battle (the setup is the round's,
/// read-only in battle) and digested with it.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Block {
    id: StateId,
    bytes: Vec<u8>,
}

impl Block {
    /// A zeroed block of `schema` (`id`'s).
    pub fn new(id: StateId, schema: &Schema) -> Block {
        Block { id, bytes: vec![0; schema.size()] }
    }

    pub fn id(&self) -> StateId {
        self.id
    }

    codec_read!();
    codec_write!();
}

// ---- A block's compact form ------------------------------------------------
//
// A block's values and nothing else, for a block to travel or be kept (a
// match's side, nettai-match's binary): each field of the schema in its
// order, a scalar at its width as the block keeps it (little-endian; a
// definition or an asset its handle plus one, 0 for none; an enum its
// variant's index, or [`ENUM_UNSTATED`]; a code its letter, 0 for none; a
// `u8?` a present byte then the value), an array's every element, a
// record's fields in their order, and a list its count (a byte, two for a
// list of room for more than 255) and then only the elements it holds. No
// names, no tags: the schema says what comes. A reader with the same
// schema reads it back to the same block.

impl Block {
    /// The block's compact form, appended to `out`.
    pub fn write_compact(&self, schema: &Schema, out: &mut Vec<u8>) {
        for i in 0..schema.fields().len() {
            self.write_compact_at(schema.place(i), out);
        }
    }

    fn write_compact_at(&self, place: Place, out: &mut Vec<u8>) {
        match place.ty {
            FieldType::Record(fields) => {
                for i in 0..fields.fields().len() {
                    self.write_compact_at(place.field_at(i), out);
                }
            }
            FieldType::List(_, max) => {
                let n = self.len_at(place).expect("a list");
                let count = list_count_size(*max);
                out.extend_from_slice(&self.bytes[place.at..place.at + count]);
                for k in 0..n {
                    self.write_compact_at(place.elem(k).expect("an element the list holds"), out);
                }
            }
            // (An array's elements are scalars, one after another.)
            ty => out.extend_from_slice(&self.bytes[place.at..place.at + ty.size()]),
        }
    }

    /// A block of `schema` (`id`'s) from its compact form at the start of
    /// `bytes`, and how many bytes it took. Refused, with where in the
    /// block and why: bytes that end inside it, a list's count past its
    /// room, a flag that isn't 0 or 1, an enum's index past its variants,
    /// a code that isn't a letter or `*`, a `u8?` that is none with a
    /// value. (A definition's or an asset's handle is the content's to
    /// check.)
    pub fn read_compact(id: StateId, schema: &Schema, bytes: &[u8]) -> Result<(Block, usize), String> {
        let mut block = Block::new(id, schema);
        let mut at = 0;
        for i in 0..schema.fields().len() {
            block.read_compact_at(schema.place(i), &schema.field(i).name, bytes, &mut at)?;
        }
        Ok((block, at))
    }

    fn read_compact_at(&mut self, place: Place, path: &str, bytes: &[u8], at: &mut usize) -> Result<(), String> {
        let take = |at: &mut usize, n: usize| -> Result<std::ops::Range<usize>, String> {
            let r = *at..*at + n;
            if r.end > bytes.len() {
                return Err(format!("{path}: the bytes end inside it"));
            }
            *at = r.end;
            Ok(r)
        };
        match place.ty {
            FieldType::Record(fields) => {
                for i in 0..fields.fields().len() {
                    self.read_compact_at(place.field_at(i), &format!("{path}.{}", fields.field(i).name), bytes, at)?;
                }
            }
            FieldType::List(_, max) => {
                let r = take(at, list_count_size(*max))?;
                let n = match r.len() {
                    1 => bytes[r.start] as usize,
                    _ => u16::from_le_bytes([bytes[r.start], bytes[r.start + 1]]) as usize,
                };
                if n > *max as usize {
                    return Err(format!("{path}: {n} entries, past its room for {max}"));
                }
                self.bytes[place.at..place.at + r.len()].copy_from_slice(&bytes[r]);
                for k in 0..n {
                    self.read_compact_at(place.elem(k).expect("within its room"), &format!("{path}[{}]", k + 1), bytes, at)?;
                }
            }
            FieldType::Array(elem, n) => {
                for k in 0..*n as usize {
                    let r = take(at, elem.size())?;
                    compact_scalar_ok(elem, &bytes[r.clone()]).map_err(|e| format!("{path}[{}]: {e}", k + 1))?;
                    let to = place.at + k * elem.size();
                    self.bytes[to..to + r.len()].copy_from_slice(&bytes[r]);
                }
            }
            ty => {
                let r = take(at, ty.size())?;
                compact_scalar_ok(ty, &bytes[r.clone()]).map_err(|e| format!("{path}: {e}"))?;
                self.bytes[place.at..place.at + r.len()].copy_from_slice(&bytes[r]);
            }
        }
        Ok(())
    }
}

/// Whether `b` is a value of the scalar type `ty` a block may hold (the
/// compact form's reader's check).
fn compact_scalar_ok(ty: &FieldType, b: &[u8]) -> Result<(), String> {
    match ty {
        FieldType::Bool if b[0] > 1 => Err(format!("{} is no flag", b[0])),
        FieldType::Enum(names) if (b[0] as usize) >= names.len() && b[0] != ENUM_UNSTATED => {
            Err(format!("variant {} of {}", b[0], names.len()))
        }
        FieldType::Code if b[0] != 0 && !is_code(b[0]) => Err(format!("{:#04x} is no chip code", b[0])),
        FieldType::OptionalU8 if b[0] > 1 || b[0] == 0 && b[1] != 0 => Err(format!("{:02x} {:02x} is no byte or none", b[0], b[1])),
        FieldType::Object if b[0] != 0 && b[0] & 0x80 == 0 => Err(format!("{:#04x} is no object", b[0])),
        _ => Ok(()),
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let used = self.bytes.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
        f.debug_struct("Block").field("id", &self.id).field("bytes", &&self.bytes[..used]).finish()
    }
}

/// A state's fields, of either kind (an arena's [`StateMut`], a [`Block`]):
/// what code that reads and writes either by its schema takes (the Luau
/// binding's state values).
pub trait Fields {
    fn id(&self) -> StateId;
    fn get(&self, schema: &Schema, i: usize) -> FieldValue;
    fn set(&mut self, schema: &Schema, i: usize, v: Value) -> Result<(), TypeError>;
    fn get_elem(&self, schema: &Schema, i: usize, k: usize) -> Option<FieldValue>;
    fn set_elem(&mut self, schema: &Schema, i: usize, k: usize, v: Value) -> Result<(), String>;
    fn get_at(&self, place: Place) -> FieldValue;
    fn set_at(&mut self, place: Place, v: Value) -> Result<(), TypeError>;
    fn len_at(&self, place: Place) -> Option<usize>;
    fn set_len_at(&mut self, place: Place, n: usize) -> Result<(), String>;
    fn clear_at(&mut self, place: Place);
}

macro_rules! fields {
    ($t:ty) => {
        impl Fields for $t {
            fn id(&self) -> StateId {
                <$t>::id(self)
            }
            fn get(&self, schema: &Schema, i: usize) -> FieldValue {
                <$t>::get(self, schema, i)
            }
            fn set(&mut self, schema: &Schema, i: usize, v: Value) -> Result<(), TypeError> {
                <$t>::set(self, schema, i, v)
            }
            fn get_elem(&self, schema: &Schema, i: usize, k: usize) -> Option<FieldValue> {
                <$t>::get_elem(self, schema, i, k)
            }
            fn set_elem(&mut self, schema: &Schema, i: usize, k: usize, v: Value) -> Result<(), String> {
                <$t>::set_elem(self, schema, i, k, v)
            }
            fn get_at(&self, place: Place) -> FieldValue {
                <$t>::get_at(self, place)
            }
            fn set_at(&mut self, place: Place, v: Value) -> Result<(), TypeError> {
                <$t>::set_at(self, place, v)
            }
            fn len_at(&self, place: Place) -> Option<usize> {
                <$t>::len_at(self, place)
            }
            fn set_len_at(&mut self, place: Place, n: usize) -> Result<(), String> {
                <$t>::set_len_at(self, place, n)
            }
            fn clear_at(&mut self, place: Place) {
                <$t>::clear_at(self, place)
            }
        }
    };
}
fields!(StateMut<'_>);
fields!(Block);

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
        let mut st = Block::new(StateId(0), &s);
        st.set(&s, 0, Value::Int(-1)).unwrap();
        assert_eq!(st.get(&s, 0), FieldValue::U16(0xFFFF));
        st.set(&s, 1, Value::Int(0x1F8)).unwrap();
        assert_eq!(st.get(&s, 1).load(), Value::Int(-8));
    }

    #[test]
    fn fields_read_back_what_was_stored() {
        let s = schema();
        let mut st = Block::new(StateId(0), &s);
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

    /// A block's compact form reads back to the same block: a list only
    /// as long as it is, records and arrays whole; what is wrong with bytes
    /// that aren't one is refused, with where.
    #[test]
    fn a_blocks_compact_form_reads_back() {
        let f = |n: &str, ty: FieldType| FieldDef { name: n.into(), ty };
        let entry = Schema::new(vec![f("chip", FieldType::Ref(Registry::Chip, None)), f("code", FieldType::Code)]).unwrap();
        let s = Schema::new(vec![
            f("folder", FieldType::List(Box::new(FieldType::Record(Box::new(entry))), 30)),
            f("flag", FieldType::Bool),
            f("level", FieldType::OptionalU8),
            f("side", FieldType::Enum(vec!["left".into(), "right".into()])),
            f("souls", FieldType::scalar("u16[3]").unwrap()),
        ])
        .unwrap();
        let mut b = Block::new(StateId(4), &s);
        let folder = s.place(s.index_of("folder").unwrap());
        b.set_len_at(folder, 2).unwrap();
        b.set_at(folder.elem(0).unwrap().field("chip").unwrap(), Value::Def(Registry::Chip, 7)).unwrap();
        b.set_at(folder.elem(1).unwrap().field("code").unwrap(), Value::Code(b'*')).unwrap();
        b.set(&s, s.index_of("flag").unwrap(), Value::Bool(true)).unwrap();
        b.set(&s, s.index_of("level").unwrap(), Value::Int(3)).unwrap();
        b.unstate(&s, s.index_of("side").unwrap());
        b.set_elem(&s, s.index_of("souls").unwrap(), 2, Value::Int(0x1234)).unwrap();
        let mut out = Vec::new();
        b.write_compact(&s, &mut out);
        // (The count, two entries of three bytes, the flag, the level, the
        // enum, the array.)
        assert_eq!(out.len(), 1 + 2 * 3 + 1 + 2 + 1 + 6);
        let (back, used) = Block::read_compact(StateId(4), &s, &out).unwrap();
        assert_eq!((back, used), (b.clone(), out.len()));
        // Trailing bytes are the reader's caller's.
        out.push(9);
        assert_eq!(Block::read_compact(StateId(4), &s, &out).unwrap().1, out.len() - 1);
        out.pop();
        let refused = |at: usize, v: u8| {
            let mut bad = out.clone();
            bad[at] = v;
            Block::read_compact(StateId(4), &s, &bad).unwrap_err()
        };
        assert_eq!(refused(0, 31), "folder: 31 entries, past its room for 30");
        assert_eq!(refused(6, b'a'), "folder[2].code: 0x61 is no chip code");
        assert_eq!(refused(7, 2), "flag: 2 is no flag");
        assert_eq!(refused(10, 2), "side: variant 2 of 2");
        assert_eq!(Block::read_compact(StateId(4), &s, &out[..5]).unwrap_err(), "folder[2].chip: the bytes end inside it");
    }

    #[test]
    fn arrays_are_read_and_written_by_element() {
        let s = schema();
        let mut st = Block::new(StateId(0), &s);
        st.set_elem(&s, 5, 5, Value::Int(0x1FF)).unwrap();
        assert_eq!(st.get_elem(&s, 5, 5), Some(FieldValue::U8(0xFF)));
        assert_eq!(st.get_elem(&s, 5, 0), Some(FieldValue::U8(0)));
        assert_eq!(st.get_elem(&s, 5, 6), None);
        assert!(st.set_elem(&s, 5, 6, Value::Int(1)).is_err());
        assert!(st.set(&s, 5, Value::Int(1)).is_err(), "a whole array isn't one value");
    }

    /// Only an enum may be stated by nobody: a list, a definition, a
    /// number stay as they are (zero: an empty list, none).
    #[test]
    fn only_an_enum_may_be_unstated() {
        let f = |n: &str, ty: FieldType| FieldDef { name: n.into(), ty };
        let form = || FieldType::Ref(Registry::Form, None);
        let s = Schema::new(vec![
            f("flags", FieldType::Array(Box::new(FieldType::Bool), 3)),
            f("forms", FieldType::Array(Box::new(form()), 3)),
            f("one", form()),
            f("which", FieldType::Enum(vec!["a".into(), "b".into()])),
        ])
        .unwrap();
        let (forms, which) = (s.index_of("forms").unwrap(), s.index_of("which").unwrap());
        let mut st = Block::new(StateId(0), &s);
        assert!((0..4).all(|i| st.stated(&s, i)), "zeroed: an empty list, the first variant");
        for i in 0..4 {
            st.unstate(&s, i);
        }
        assert!((0..4).all(|i| st.stated(&s, i) == (i != which)));
        assert!((0..3).all(|k| st.get_elem(&s, forms, k) == Some(FieldValue::Ref(None))), "a list left out is empty");
    }

    /// A block is its schema's size, and reads and writes as an arena's
    /// state does.
    #[test]
    fn a_block_is_its_schemas_size() {
        let f = |n: &str, ty: FieldType| FieldDef { name: n.into(), ty };
        let s = Schema::new(vec![f("big", FieldType::scalar("u16[60]").unwrap()), f("last", FieldType::U32)]).unwrap();
        let mut b = Block::new(StateId(0), &s);
        b.set_elem(&s, 0, 59, Value::Int(0x1234)).unwrap();
        b.set(&s, 1, Value::Int(-1)).unwrap();
        assert_eq!((b.get_elem(&s, 0, 59), b.get(&s, 1)), (Some(FieldValue::U16(0x1234)), FieldValue::U32(u32::MAX)));
        assert_eq!(b.get_elem(&s, 0, 0), Some(FieldValue::U16(0)));
    }

    #[test]
    fn wrong_kinds_and_bad_enum_values_are_errors() {
        let s = schema();
        let mut st = Block::new(StateId(0), &s);
        assert!(st.set(&s, 0, Value::Bool(true)).is_err());
        assert!(st.set(&s, 2, Value::Int(2)).is_err());
        assert!(st.set(&s, 3, Value::Int(1)).is_err());
    }

    #[test]
    fn schemas_reject_duplicates_and_bad_names() {
        let f = |n: &str, ty: FieldType| FieldDef { name: n.into(), ty };
        assert!(Schema::new(vec![f("a", FieldType::U8), f("a", FieldType::U8)]).is_err());
        assert!(Schema::new(vec![f("1a", FieldType::U8)]).is_err());
        // (A schema of any size: the user, "drop the block cap".)
        let large = Schema::new((0..400).map(|i| f(&format!("v{i}"), FieldType::Vec3)).collect()).unwrap();
        assert_eq!(large.size(), 4800);
        let mut block = Block::new(StateId(0), &large);
        block.set(&large, 399, Value::Vec3(Vec3 { x: 1, y: 2, z: 3 })).unwrap();
        assert_eq!(block.get(&large, 399).load(), Value::Vec3(Vec3 { x: 1, y: 2, z: 3 }));
        assert!(FieldType::scalar("vec3[2]").is_none());
        assert!(FieldType::scalar("u8[0]").is_none());
    }

    /// docs/design/rust-and-luau.md, step c1: a record's fields and a
    /// list's elements are read and written by their places, a list holds
    /// what its length says (the rest zero), and a code is its letter.
    #[test]
    fn records_lists_and_codes() {
        use crate::data::Key;
        let str_ = |s: &str| Data::Str(s.into());
        let map = |kv: Vec<(&str, Data)>| Data::Map(kv.into_iter().map(|(k, v)| (Key::Str(k.into()), v)).collect());
        let list = |of: Data, max: i64| map(vec![(LIST_MARK, Data::Bool(true)), ("max", Data::Int(max)), ("of", of)]);
        let d = map(vec![
            ("folder", list(map(vec![("code", str_("code")), ("chip", str_("chip"))]), 30)),
            ("tags", list(str_("u8"), 2)),
            ("at", map(vec![("x", str_("u8")), ("y", str_("i8"))])),
            ("many", list(str_("u16"), 300)),
        ]);
        let s = Schema::from_data(&d).unwrap();
        // In name order, within a record too: { chip, code }.
        let names: Vec<&str> = s.fields().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["at", "folder", "many", "tags"]);
        let folder = s.place(s.index_of("folder").unwrap());
        assert_eq!(folder.ty().to_string(), "list of 30 { chip: chip, code: code }");
        // A count byte, then room for 30 of (a chip, 2 bytes; a code, 1).
        assert_eq!(folder.ty().size(), 1 + 30 * 3);
        assert_eq!(s.place(s.index_of("many").unwrap()).ty().size(), 2 + 300 * 2, "a count of two bytes past 255");
        let mut b = Block::new(StateId(0), &s);
        assert_eq!(b.len_at(folder), Some(0));
        b.set_len_at(folder, 2).unwrap();
        let second = folder.elem(1).unwrap();
        b.set_at(second.field("chip").unwrap(), Value::Def(Registry::Chip, 7)).unwrap();
        b.set_at(second.field("code").unwrap(), Value::Code(b'*')).unwrap();
        assert_eq!(b.get_at(second.field("chip").unwrap()), FieldValue::Ref(Some((Registry::Chip, 7))));
        assert_eq!(b.get_at(second.field("code").unwrap()).load(), Value::Code(b'*'));
        assert_eq!(b.get_at(folder.elem(0).unwrap().field("code").unwrap()), FieldValue::Code(None));
        assert!(b.set_at(second.field("code").unwrap(), Value::Code(b'a')).is_err(), "a lowercase letter is no code");
        assert!(b.set_at(folder, Value::Int(1)).is_err(), "a list isn't one value");
        assert_eq!(b.get_at(folder), FieldValue::Nested);
        assert!(folder.elem(30).is_none());
        // Shortened, what it drops is zero: the same block as one never longer.
        b.set_len_at(folder, 1).unwrap();
        let mut fresh = Block::new(StateId(0), &s);
        fresh.set_len_at(folder, 1).unwrap();
        assert_eq!(b, fresh);
        assert!(b.set_len_at(folder, 31).is_err());
        let at = s.place(s.index_of("at").unwrap());
        b.set_at(at.field("y").unwrap(), Value::Int(-2)).unwrap();
        assert_eq!(b.get_at(at.field("y").unwrap()).load(), Value::Int(-2));
        assert_eq!(b.get_at(at.field("x").unwrap()), FieldValue::U8(0));
        assert!(at.field("z").is_none());
        let many = s.place(s.index_of("many").unwrap());
        b.set_len_at(many, 300).unwrap();
        b.set_at(many.elem(299).unwrap(), Value::Int(9)).unwrap();
        assert_eq!((b.len_at(many), b.get_at(many.elem(299).unwrap())), (Some(300), FieldValue::U16(9)));
        // What a declaration may not be.
        assert!(Schema::from_data(&map(vec![("l", list(str_("u8"), 0))])).is_err());
        assert!(Schema::from_data(&map(vec![("l", list(str_("u9"), 2))])).is_err());
        assert!(Schema::from_data(&map(vec![("r", map(vec![("1x", str_("u8"))]))])).is_err());
        // A field by its name, the schema's or a record's; cleared whole.
        let y = s.find("y").unwrap().expect("the record's field");
        assert_eq!((s.path_name(y), s.place_of(y).ty().clone()), ("at.y".to_string(), FieldType::I8));
        assert_eq!(s.place_of(s.find("tags").unwrap().unwrap()).ty().to_string(), "list of 2 u8");
        assert_eq!(s.find("code"), Ok(None), "a list's elements' fields have a place each");
        b.clear_at(at);
        assert_eq!(b.get_at(at.field("y").unwrap()), FieldValue::I8(0));
        let twice = Schema::from_data(&map(vec![("a", map(vec![("n", str_("u8"))])), ("b", map(vec![("n", str_("u8"))]))])).unwrap();
        assert_eq!(twice.find("n"), Err("`n` is `a.n` and `b.n`".to_string()));
    }

    fn arena_schemas() -> (Schema, Schema) {
        let f = |n: &str, ty: FieldType| FieldDef { name: n.into(), ty };
        let small = Schema::new(vec![f("a", FieldType::U16), f("b", FieldType::U8)]).unwrap();
        let large = Schema::new((0..20).map(|i| f(&format!("v{i}"), FieldType::Vec3)).collect()).unwrap();
        (small, large)
    }

    /// Blocks are packed in slot order, each its layout's size; a write to
    /// one leaves the others alone, and a neighbor's new block moves a
    /// block's bytes without changing its values.
    #[test]
    fn an_arena_packs_each_slots_block_in_slot_order() {
        let (small, large) = arena_schemas();
        let (s, l) = (StateId(1), StateId(2));
        let mut a = StateArena::<4>::default();
        a.reset(2, small.size());
        a.state_mut(2, s).set(&small, 0, Value::Int(0x1234)).unwrap();
        a.reset(0, large.size());
        a.state_mut(0, l).set(&large, 19, Value::Vec3(Vec3 { x: 7, y: 8, z: 9 })).unwrap();
        a.reset(3, small.size());
        a.state_mut(3, s).set(&small, 1, Value::Int(5)).unwrap();
        assert_eq!(a.size(), large.size() + 2 * small.size(), "no slot without a state takes bytes");
        assert_eq!((a.size_of(0), a.size_of(1), a.size_of(2)), (large.size(), 0, small.size()));
        assert_eq!(a.state(2, s).get(&small, 0), FieldValue::U16(0x1234), "moved by slot 0's block, kept");
        assert_eq!(a.state(2, s).get(&small, 1), FieldValue::U8(0));
        assert_eq!(a.state(3, s).get(&small, 1), FieldValue::U8(5));
        assert_eq!(a.state(0, l).get(&large, 19).load(), Value::Vec3(Vec3 { x: 7, y: 8, z: 9 }));
        assert_eq!(a.state(0, l).id(), l);
    }

    /// Taking a state replaces the slot's block, zeroed, at the new
    /// layout's size; nothing else does: a freed object's block stays
    /// readable until its slot is taken again.
    #[test]
    fn a_slots_block_is_replaced_only_when_it_takes_a_state() {
        let (small, large) = arena_schemas();
        let (s, l) = (StateId(1), StateId(2));
        let mut a = StateArena::<3>::default();
        a.reset(0, small.size());
        a.state_mut(0, s).set(&small, 0, Value::Int(3)).unwrap();
        a.reset(1, large.size());
        a.state_mut(1, l).set(&large, 0, Value::Vec3(Vec3 { x: 1, y: 1, z: 1 })).unwrap();
        a.reset(2, small.size());
        a.state_mut(2, s).set(&small, 0, Value::Int(4)).unwrap();
        // Slot 1 freed, then taken by the small layout: its block shrinks,
        // zeroed, and slot 2's moves down with its values.
        assert_eq!(a.state(1, l).get(&large, 0).load(), Value::Vec3(Vec3 { x: 1, y: 1, z: 1 }), "left as it was");
        a.reset(1, small.size());
        assert_eq!(a.size(), 3 * small.size());
        assert_eq!(a.state(1, s).get(&small, 0), FieldValue::U16(0));
        assert_eq!((a.state(0, s).get(&small, 0), a.state(2, s).get(&small, 0)), (FieldValue::U16(3), FieldValue::U16(4)));
        // Taken by a kind with no state: no block.
        a.reset(1, 0);
        assert_eq!((a.size_of(1), a.size()), (0, 2 * small.size()));
        assert_eq!(a.state(2, s).get(&small, 0), FieldValue::U16(4));
    }

    /// The bytes follow from what each slot took, in what order: the same
    /// steps give the same arena, and a copy reads as the original.
    #[test]
    fn the_same_steps_give_the_same_arena() {
        let (small, large) = arena_schemas();
        let steps = |a: &mut StateArena<8>| {
            for (slot, big, v) in [(3, true, 1), (0, false, 2), (5, false, 3), (3, false, 4), (1, true, 5), (0, true, 6)] {
                let (schema, id) = if big { (&large, StateId(2)) } else { (&small, StateId(1)) };
                a.reset(slot, schema.size());
                let mut st = a.state_mut(slot, id);
                if big {
                    st.set(schema, 3, Value::Vec3(Vec3 { x: v, y: -v, z: 0 })).unwrap();
                } else {
                    st.set(schema, 0, Value::Int(v as i64)).unwrap();
                }
            }
        };
        let (mut a, mut b) = (StateArena::<8>::default(), StateArena::<8>::default());
        steps(&mut a);
        steps(&mut b);
        assert_eq!(a, b);
        let c = a.clone();
        assert_eq!(c.state(1, StateId(2)).get(&large, 3).load(), Value::Vec3(Vec3 { x: 5, y: -5, z: 0 }));
        assert_eq!(c.state(5, StateId(1)).get(&small, 0), FieldValue::U16(3));
        assert_eq!(c.size(), 2 * large.size() + 2 * small.size());
    }
}
