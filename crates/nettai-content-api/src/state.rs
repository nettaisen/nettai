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

use std::fmt;

use crate::data::{Data, Key};
use crate::registry::Registry;
use crate::assets::AssetKind;
use crate::types::{ObjectRef, Pool, Vec3};

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
}

impl FieldType {
    /// A type by name: `bool`, `u8`, `u16`, `u32`, `i8`, `i16`, `i32`,
    /// `u8?` (a byte or none: a setup's fact that may be stated as none, as
    /// a navi code's level), `object`, `vec3`, or an array of one of the
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
            FieldType::Ref(r, _) => FieldValue::Ref(u16::from_le_bytes([b[0], b[1]]).checked_sub(1).map(|h| (*r, h))),
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
        self.offsets[i]
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
