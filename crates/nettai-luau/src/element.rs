//! `Element`: an element byte as content holds it (docs/design/scripting.md
//! §2.1): an attack's primary element and its secondary elements, the byte
//! objects, navis' attacks and stats and collision data keep (the primary
//! element in the low four bits, the secondary elements' bits above). A
//! value of its own, opaque to content: made by names (`Element.new("fire",
//! { sword = true })`, `Element.fire`) and read by names (`e.primary`,
//! `e.secondary.sword`); compared with `==`. Each byte has one value per VM
//! (and one view of its secondary elements), made when first asked for, so
//! reading one allocates nothing after.
//!
//! The byte itself (`e.byte`, `Element.from_byte(n)`) is for the places the
//! original holds an element as a number: where its registers carry the
//! byte into a coordinate (a spawn's position), or a number held for
//! something else into the element (a side, a coordinate's low byte), and
//! the bytes the original's data holds (a NaviCust bug code's argument, a
//! patch card's block).
//!
//! The define phase reads a value back as the byte it is (`Data::Int`).

use std::cell::RefCell;

use mlua::{AnyUserData, Lua, MetaMethod, UserData, UserDataFields, UserDataMethods, Value as LuaValue};

/// The primary elements, by their number (the byte's low four bits): the
/// five a chip or a navi has, and the sixth, the drain (the damage that
/// passes barriers, holy panels and Undershirt: poison, the field's drain).
pub const PRIMARY: [&str; 6] = ["null", "fire", "aqua", "elec", "wood", "drain"];

/// The secondary elements, by their bit.
pub const SECONDARY: [(&str, u8); 4] = [("break", 0x10), ("wind", 0x20), ("cursor", 0x40), ("sword", 0x80)];

/// An element byte.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Element(pub u8);

impl Element {
    /// The primary element's name (none: a number past the drain, which
    /// only a byte written from something else holds).
    pub fn primary(self) -> Option<&'static str> {
        PRIMARY.get((self.0 & 0xF) as usize).copied()
    }

    fn describe(self) -> String {
        let mut s = self.primary().map_or_else(|| format!("primary {}", self.0 & 0xF), str::to_string);
        for (name, bit) in SECONDARY {
            if self.0 & bit != 0 {
                s.push('+');
                s.push_str(name);
            }
        }
        s
    }
}

impl UserData for Element {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("primary", |_, this| Ok(this.primary()));
        fields.add_field_method_get("secondary", |lua, this| secondary_value(lua, this.0));
        fields.add_field_method_get("byte", |_, this| Ok(this.0));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            Ok(other.borrow::<Element>().is_ok_and(|o| o.0 == this.0))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(format!("Element({})", this.describe())));
    }
}

/// The secondary elements of a byte, each a property (`e.secondary.sword`,
/// `c.damage_elements.wind`; read-only).
#[derive(Clone, Copy)]
pub(crate) struct SecondaryElements(pub u8);

impl UserData for SecondaryElements {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for (name, bit) in SECONDARY {
            fields.add_field_method_get(name, move |_, this| Ok(this.0 & bit != 0));
        }
    }
}

/// A secondary element's bit, by its name.
pub(crate) fn secondary(name: &str) -> mlua::Result<u8> {
    SECONDARY.iter().find(|(n, _)| *n == name).map(|&(_, bit)| bit).ok_or_else(|| {
        mlua::Error::runtime(format!("{name:?} is not a secondary element (break, wind, cursor, sword)"))
    })
}

/// The bits of a table of secondary elements by name, each true or false
/// (`{ sword = true }`): an unknown name or a value not a boolean is an error.
pub(crate) fn secondary_set(t: &mlua::Table, what: &str) -> mlua::Result<u8> {
    let mut b = 0;
    for pair in t.pairs::<LuaValue, LuaValue>() {
        let (k, v) = pair?;
        let name = match &k {
            LuaValue::String(s) => s.to_str()?.to_string(),
            k => return Err(mlua::Error::runtime(format!("{what}: a secondary element is named, not {}", k.type_name()))),
        };
        let bit = secondary(&name).map_err(|e| mlua::Error::runtime(format!("{what}: {e}")))?;
        match v {
            LuaValue::Boolean(true) => b |= bit,
            LuaValue::Boolean(false) => {}
            v => return Err(mlua::Error::runtime(format!("{what}: `{name}` is true or false, not {}", v.type_name()))),
        }
    }
    Ok(b)
}

/// A primary element's number, by its name.
pub(crate) fn primary(name: &str) -> mlua::Result<u8> {
    PRIMARY.iter().position(|n| *n == name).map(|i| i as u8).ok_or_else(|| {
        mlua::Error::runtime(format!("{name:?} is not an element (null, fire, aqua, elec, wood, drain)"))
    })
}

/// A primary element given by name (`c:element_damage("fire")`).
pub(crate) fn primary_arg(v: &LuaValue, what: &str) -> mlua::Result<u8> {
    match v {
        LuaValue::String(s) => primary(&s.to_str()?).map_err(|e| mlua::Error::runtime(format!("{what}: {e}"))),
        v => Err(mlua::Error::runtime(format!("{what}: an element is named (\"fire\"), not {}", v.type_name()))),
    }
}

/// The values made so far, one per byte: the elements, then their
/// secondary elements' views.
#[derive(Default)]
struct Interned(RefCell<Vec<Option<AnyUserData>>>);

/// The value kept at `at` (512 places), made by `make` the first time.
fn interned(lua: &Lua, at: usize, make: impl FnOnce() -> mlua::Result<AnyUserData>) -> mlua::Result<AnyUserData> {
    if lua.app_data_ref::<Interned>().is_none() {
        lua.set_app_data(Interned::default());
    }
    let interned = lua.app_data_ref::<Interned>().expect("set above");
    if let Some(Some(ud)) = interned.0.borrow().get(at) {
        return Ok(ud.clone());
    }
    let ud = make()?;
    let mut kept = interned.0.borrow_mut();
    if kept.is_empty() {
        kept.resize(512, None);
    }
    kept[at] = Some(ud.clone());
    Ok(ud)
}

/// The value of byte `b`: the one made before, if any.
pub(crate) fn value(lua: &Lua, b: u8) -> mlua::Result<AnyUserData> {
    interned(lua, b as usize, || lua.create_userdata(Element(b)))
}

/// The view of the secondary elements of byte `b` (its high four bits).
pub(crate) fn secondary_value(lua: &Lua, b: u8) -> mlua::Result<AnyUserData> {
    let b = b & 0xF0;
    interned(lua, 256 + b as usize, || lua.create_userdata(SecondaryElements(b)))
}

/// The element `v` is, if it is one.
pub(crate) fn of(v: &LuaValue) -> Option<u8> {
    match v {
        LuaValue::UserData(ud) => ud.borrow::<Element>().ok().map(|e| e.0),
        _ => None,
    }
}

/// The byte of an element argument.
pub(crate) fn arg(v: &LuaValue, what: &str) -> mlua::Result<u8> {
    of(v).ok_or_else(|| {
        mlua::Error::runtime(format!(
            "{what}: expected an Element (`Element.new(\"fire\")`, `Element.fire`), got {}",
            match v {
                LuaValue::UserData(ud) => ud.type_name().map(|s| s.to_string_lossy()).unwrap_or_default(),
                v => v.type_name().to_string(),
            }
        ))
    })
}

/// The `Element` library: `new`, `from_byte` and the primary elements alone.
pub(crate) fn lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    t.set(
        "new",
        lua.create_function(|lua, (p, s): (mlua::LuaString, Option<mlua::Table>)| {
            let mut b = primary(&p.to_str()?)?;
            if let Some(s) = s {
                b |= secondary_set(&s, "Element.new's secondary elements")?;
            }
            value(lua, b)
        })?,
    )?;
    t.set(
        "from_byte",
        lua.create_function(|lua, n: LuaValue| {
            let n = match n {
                LuaValue::Integer(i) => i64::from(i),
                LuaValue::Number(f) if f.fract() == 0.0 => f as i64,
                v => return Err(mlua::Error::runtime(format!("Element.from_byte: expected a byte, got {}", v.type_name()))),
            };
            let b = u8::try_from(n).map_err(|_| mlua::Error::runtime(format!("Element.from_byte: {n} is not a byte (0 to 255)")))?;
            value(lua, b)
        })?,
    )?;
    for (i, name) in PRIMARY.iter().enumerate() {
        t.set(*name, value(lua, i as u8)?)?;
    }
    // (A member it hasn't is an error, as the choices' tables' is; it is
    // read-only.)
    crate::choice::seal(lua, &t, "Element")?;
    Ok(t)
}
