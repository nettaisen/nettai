//! Identities: what an object is taken for (`define.identity`, docs/design/
//! content-model-v2.md §3.2). The original keys all of it by an object's
//! NameID: the actor record (`byte_80182C4`), a navi sprite's attach
//! points (`sub_8018810`), a field object's look (`byte_8021220`), and
//! what the ruleset tests NameID ranges for. A navi's and a form's
//! identity is nested in its definition; a field object's is its kind's.
//! An object with none is what the original's NameID 0 is: a virus.

use bn6_content_api::{FormHandle, NaviHandle};

use super::{AttachPoint, NaviRecord, SpriteId};
use crate::actor::ActorType;

/// What the ruleset takes an identity for: the original's NameID ranges.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IdentityClass {
    /// A virus (NameIDs up to 0xBA), and anything with no identity.
    Virus,
    /// A field object (0xCD to 0xFF): a rock, a statue, a mine.
    FieldObject,
    /// A navi that is no player's (0x100 to 0x19F but the Cybeasts).
    Navi,
    /// The Cybeasts (0x173 to 0x178, 0x179 to 0x17E).
    Gregar,
    Falzar,
    /// MegaMan in his base form (0x1A0).
    MegaMan,
    /// A link navi (0x1A1 to 0x1AB).
    LinkNavi,
    /// MegaMan's forms: a Cross (0x1AC to 0x1B5), Beast Out (0x1B6,
    /// 0x1B7), a Cross in Beast Out (0x1B8 to 0x1C1), Beast Over (0x1C2,
    /// 0x1C3).
    Cross,
    Beast,
    CrossBeast,
    BeastOver,
}

impl IdentityClass {
    /// The classes by the names definitions give them.
    pub const NAMES: [(&'static str, IdentityClass); 11] = [
        ("virus", IdentityClass::Virus),
        ("field_object", IdentityClass::FieldObject),
        ("navi", IdentityClass::Navi),
        ("gregar", IdentityClass::Gregar),
        ("falzar", IdentityClass::Falzar),
        ("megaman", IdentityClass::MegaMan),
        ("link_navi", IdentityClass::LinkNavi),
        ("cross", IdentityClass::Cross),
        ("beast", IdentityClass::Beast),
        ("cross_beast", IdentityClass::CrossBeast),
        ("beast_over", IdentityClass::BeastOver),
    ];

    pub fn from_name(name: &str) -> Option<IdentityClass> {
        Self::NAMES.iter().find(|(n, _)| *n == name).map(|&(_, c)| c)
    }

    /// A player's navi: MegaMan in any form, or a link navi (the
    /// original's 0x1A0 to 0x1C3).
    pub fn is_player(self) -> bool {
        use IdentityClass::*;
        matches!(self, MegaMan | LinkNavi | Cross | Beast | CrossBeast | BeastOver)
    }

    /// One of MegaMan's forms (past the link navis: 0x1AC and up).
    pub fn is_form(self) -> bool {
        use IdentityClass::*;
        matches!(self, Cross | Beast | CrossBeast | BeastOver)
    }

    /// Any navi (the original's 0x100 to 0x1C3).
    pub fn is_navi(self) -> bool {
        self.is_player() || matches!(self, IdentityClass::Navi | IdentityClass::Gregar | IdentityClass::Falzar)
    }

    /// A Cybeast (0x173 to 0x17E).
    pub fn is_cybeast(self) -> bool {
        matches!(self, IdentityClass::Gregar | IdentityClass::Falzar)
    }
}

/// A field object's look (`byte_8021220`): what something that takes its
/// place shows (DustMan's junk, an absorbed obstacle).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldLook {
    /// None: nothing to show (`sub_800F26C`'s "no look").
    pub sprite: Option<SpriteId>,
    pub anim: u8,
    pub palette: u8,
    /// Drawn with a ground shadow.
    pub shadow: bool,
    /// What wears the look keeps its own flip (the time bombs').
    pub keeps_flip: bool,
}

/// The navi or form an identity is nested in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IdentityOwner {
    Navi(NaviHandle),
    Form(FormHandle),
}

/// An identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Identity {
    pub key: String,
    pub class: IdentityClass,
    /// The actor record (`sub_80182B4`).
    pub record: NaviRecord,
    /// Where things attach to a navi's sprite (`sub_8018810`), by attach
    /// point. A field object has none of its own: every point of one is
    /// (0, 7).
    pub attach_points: Vec<AttachPoint>,
    /// A field object's look.
    pub look: Option<FieldLook>,
    /// The obstacle-absorbing action and ColArmy take it (not a mine).
    pub absorbable: bool,
    /// It can be swallowed or left as junk (`sub_800F486`: not a mine, not
    /// BodyGrd's striker).
    pub scrap: bool,
    /// Whose it is, for a navi's or a form's.
    pub owner: Option<IdentityOwner>,
}

impl Identity {
    /// What an object with no identity is: the original's NameID 0, a
    /// virus whose actor record is all zeros.
    pub fn none() -> &'static Identity {
        static NONE: std::sync::OnceLock<Identity> = std::sync::OnceLock::new();
        NONE.get_or_init(|| Identity {
            key: String::new(),
            class: IdentityClass::Virus,
            record: NaviRecord { version: 0, actor_type: ActorType::Virus, ai_index: 0 },
            attach_points: Vec::new(),
            look: None,
            absorbable: true,
            scrap: true,
            owner: None,
        })
    }
}

/// An identity definition as the engine holds it (its owner is set by the
/// navi or form that names it).
pub(crate) fn read(
    d: &bn6_content_api::Definition,
    assets: &bn6_content_api::AssetNames,
) -> Result<Identity, bn6_content_api::ContentError> {
    use bn6_content_api::{AssetKind, ContentError, Data};
    let what = |m: String| ContentError::new(format!("{}.luau: identity {}: {m}", d.module, d.key));
    let spec = &d.spec;
    let class = match spec.field("class") {
        Data::Str(s) => IdentityClass::from_name(s).ok_or_else(|| {
            let names: Vec<&str> = IdentityClass::NAMES.iter().map(|(n, _)| *n).collect();
            what(format!("`class` {s:?} is none of {}", names.join(", ")))
        })?,
        _ => return Err(what("needs a `class`".into())),
    };
    let byte = |v: &Data, field: &str| -> Result<u8, ContentError> {
        match v {
            Data::Nil => Ok(0),
            Data::Int(i) => u8::try_from(*i).map_err(|_| what(format!("`{field}` {i} is not a byte"))),
            other => Err(what(format!("`{field}` is {other:?}, not a number"))),
        }
    };
    let flag = |v: &Data, field: &str, default: bool| -> Result<bool, ContentError> {
        match v {
            Data::Nil => Ok(default),
            Data::Bool(b) => Ok(*b),
            other => Err(what(format!("`{field}` is {other:?}, not true or false"))),
        }
    };
    // The actor type: what its class says, unless it gives one.
    let actor_type = match spec.field("actor_type") {
        Data::Nil if class.is_player() => ActorType::Player,
        Data::Nil if class.is_navi() => ActorType::Navi,
        Data::Nil => ActorType::Virus,
        Data::Str(s) => match s.as_str() {
            "virus" => ActorType::Virus,
            "navi" => ActorType::Navi,
            "player" => ActorType::Player,
            other => return Err(what(format!("`actor_type` {other:?} is not virus, navi or player"))),
        },
        other => return Err(what(format!("`actor_type` is {other:?}, not a name"))),
    };
    let attach_points = match spec.field("attach_points") {
        Data::Nil => Vec::new(),
        Data::List(points) => points
            .iter()
            .map(|p| match p {
                Data::List(xy) => match xy.as_slice() {
                    [Data::Int(x), Data::Int(y)] => match (i8::try_from(*x), i8::try_from(*y)) {
                        (Ok(x), Ok(y)) => Ok(AttachPoint { x, y }),
                        _ => Err(what(format!("the attach point {{ {x}, {y} }} is not in pixels a byte holds"))),
                    },
                    _ => Err(what("an attach point is `{ x, y }`".into())),
                },
                _ => Err(what("an attach point is `{ x, y }`".into())),
            })
            .collect::<Result<_, _>>()?,
        other => return Err(what(format!("`attach_points` is {other:?}, not a list"))),
    };
    let look = match spec.field("look") {
        Data::Nil => None,
        l @ Data::Map(_) => Some(FieldLook {
            sprite: match l.field("sprite") {
                Data::Nil => None,
                Data::Asset(AssetKind::Sprite, name) => {
                    Some(*assets.sprites.get(name).ok_or_else(|| what(format!("the pack has no sprite {name:?}")))?)
                }
                other => return Err(what(format!("look.sprite is {other:?}, not a sprite"))),
            },
            anim: byte(l.field("anim"), "look.anim")?,
            palette: byte(l.field("palette"), "look.palette")?,
            shadow: flag(l.field("shadow"), "look.shadow", false)?,
            keeps_flip: flag(l.field("keeps_flip"), "look.keeps_flip", false)?,
        }),
        other => return Err(what(format!("`look` is {other:?}, not a table"))),
    };
    if look.is_some() && class != IdentityClass::FieldObject {
        return Err(what("only a field object has a `look`".into()));
    }
    Ok(Identity {
        key: d.key.clone(),
        class,
        record: NaviRecord {
            version: byte(spec.field("version"), "version")?,
            actor_type,
            ai_index: byte(spec.field("ai_index"), "ai_index")?,
        },
        attach_points,
        look,
        absorbable: flag(spec.field("absorbable"), "absorbable", true)?,
        scrap: flag(spec.field("scrap"), "scrap", true)?,
        owner: None,
    })
}
