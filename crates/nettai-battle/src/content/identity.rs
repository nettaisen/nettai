//! Identities: what an object is taken for (`new.identity`, docs/design/
//! content-model-v2.md §3.2). The original keys all of it by an object's
//! NameID: the actor record (`byte_80182C4`), a navi sprite's attach
//! points (`sub_8018810`), a field object's look (`byte_8021220`), and
//! what the rules test NameID ranges for. A navi's and a form's
//! identity is nested in its definition; a field object's is its kind's.
//! An object with none is what the original's NameID 0 is: a virus.

use nettai_content_api::{FormHandle, NaviHandle};

use super::{AttachPoint, NaviRecord, SpriteId};
use crate::actor::ActorType;

/// What the rules take an identity for: the original's NameID ranges.
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

/// A body overlay (`byte_80C4320`, `off_80C42D4`): a second sprite over a
/// navi's body, drawn in front of it in some of its animations and one
/// pixel further back in the others.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BodyPart {
    pub sprite: SpriteId,
    /// By the wearer's animation: drawn in front of it. (Past the list,
    /// the original reads on into the tables' pointers.)
    pub in_front: Vec<bool>,
    /// It keeps its own palette (0) rather than its wearer's.
    pub own_palette: bool,
    /// Added to the wearer's animation.
    pub anim_offset: u8,
}

/// What an identity's object wears: what its actor record's init hook
/// puts on (`off_8010E0C`), and its death hook takes off (`off_801105C`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Parts {
    /// A body overlay (`sub_80C44A8`), in the object's second related slot.
    Body(BodyPart),
    /// Two of them (`sub_8010FD8`): the second in the object's second
    /// overlay slot.
    Bodies(BodyPart, BodyPart),
    /// An overlay worn only while standing (`sub_80C41D8`).
    Idle { sprite: SpriteId },
    /// A beast's head (`sub_8011366`, `sub_8011352`): its palette, or
    /// none for the one that follows the side's mood.
    BeastHead { sprite: SpriteId, palette: Option<u8> },
    /// A routine of its own, `own(wearer)`: the init hook of a record that
    /// wears an object of the content's own kind, which the routine puts in
    /// the wearer's second related slot itself (EXE5's NumberMan's face,
    /// 0x0800EE6E: NumberSoul's layer).
    Own(nettai_content_api::FnId),
}

/// Which of its object's hooks touch what the object wears (the overlay
/// in its second related slot), by the actor record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct OverlayHooks {
    /// The death hook takes it off (`off_801105C`; a form's, `sub_8011384`).
    pub death: bool,
    /// An animation change restarts it (`off_8011470`).
    pub refresh: bool,
    /// A flinch restarts it (`off_80EAB94`).
    pub flinch: bool,
    /// The flinch's hook tests for what the navi wears before it restarts
    /// it (EXE5's team navis' hooks; EXE6's link navis' don't, and without
    /// the overlay jump into another routine).
    pub flinch_checked: bool,
    /// A drag restarts it (`off_80EABF8`).
    pub drag: bool,
    /// A flinch, a drag or a paralysis lets go of it where an attack holds
    /// it shown or hidden: one routine in those three tables (EXE5's
    /// KnightMan's, 0x080FBA20: `sub_80C4510` on his ball and chain;
    /// nothing without it). EXE6's paralysis has no per-form hook.
    pub lets_go: bool,
}

/// The size of the ice block that fits an identity's object
/// (`byte_80E9C30`, `byte_80E9C4E`: the block's animation).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IceSize {
    #[default]
    Small = 0,
    Medium = 1,
    Large = 2,
}

/// What a navi no player controls is (an identity of actor type navi,
/// spawned mid-battle: EXE5's Dark MegaMan): the original's enemy structs
/// by NameID (`enemy_getStruct1`, `enemy_getStruct2`: EXE5's 0x0800D138,
/// 0x0800D160) and the constants of its AI index's post-init hook.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActorBody {
    /// Its sprite (struct 1 bytes 0 and 1), and whether it has a shadow
    /// (byte 5).
    pub sprite: SpriteId,
    pub shadow: bool,
    /// Struct 1 byte 2 (AIData+3): the multiplier of its palette by
    /// version (`sub_800F334`).
    pub palette_scale: u8,
    /// Its palette once in (its post-init hook's: EXE5's Dark MegaMan's 1,
    /// 0x08104270); none: `sub_800F334`'s.
    pub palette: Option<u8>,
    /// Struct 2: its HP (the low 12 bits) and element (the high 4).
    pub hp: u16,
    pub element: u8,
    /// Struct 2 +4: what its body deals (BattleObject+0x2C).
    pub body_damage: u16,
    /// Its body's collision: what it is, what it reacts to and its hit
    /// modifier (`object_setupCollisionData`'s, which struct 2 byte 3 and
    /// the NameID pick: EXE5's 0x08013C5A).
    pub collision: (nettai_content_api::CollisionHandle, nettai_content_api::CollisionHandle, u8),
    /// The flag-1 bits it starts with (struct 2 byte 3's: superarmor,
    /// airshoe, floatshoe, affected by ice, untouchable; and its hook's).
    pub flags: u32,
    /// Its HP number never shows (EXE5's NameID 0x18D: 0x0801339C,
    /// 0x08104270).
    pub hides_hp: bool,
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
    /// EXE5's Poltergeist takes it (0x080E8CA0, 0x080E8D1C: not EXE5's
    /// NameIDs 0xDA, 0xD3, 0xD2, 0xE5, 0xE4 and 0xE7).
    pub throwable: bool,
    /// What its object wears.
    pub parts: Option<Parts>,
    /// Which hooks touch what its object wears.
    pub overlay_hooks: OverlayHooks,
    /// The Full Synchro aura's animation around it (`sub_80C4C52`); none:
    /// it gets no aura (`sub_80139C4`).
    pub aura_anim: Option<u8>,
    /// The anger tick passes over it (EXE5's 0x08011A14 tests the actor
    /// record's AI index for 23: the records of NameIDs 0x18E to 0x190, a
    /// player's type, which ShadowMan's SplitUp shadow has and no navi a
    /// player operates). A body of the player's kind that takes no part in
    /// the emotions states it.
    pub never_angers: bool,
    /// The ice block that fits it.
    pub ice: IceSize,
    /// Whose it is, for a navi's or a form's.
    pub owner: Option<IdentityOwner>,
    /// What it is as a navi no player controls (actor type navi).
    pub body: Option<ActorBody>,
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
            throwable: true,
            parts: None,
            overlay_hooks: OverlayHooks::default(),
            aura_anim: None,
            never_angers: false,
            ice: IceSize::Small,
            owner: None,
            body: None,
        })
    }
}

/// An identity definition as the engine holds it (its owner is set by the
/// navi or form that names it). `own_part`: the function its `parts.own`
/// is, when it has one.
pub(crate) fn read(
    d: &nettai_content_api::Definition,
    assets: &nettai_content_api::AssetNames,
    definitions: &nettai_content_api::Definitions,
    own_part: Option<nettai_content_api::FnId>,
) -> Result<Identity, nettai_content_api::ContentError> {
    use nettai_content_api::{AssetKind, ContentError, Data};
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
                    Some(SpriteId(assets.handle(AssetKind::Sprite, name).ok_or_else(|| what(format!("the packs have no sprite {name:?}")))?))
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
    let sprite = |v: &Data, field: &str| -> Result<SpriteId, ContentError> {
        match v {
            Data::Asset(AssetKind::Sprite, name) => {
                assets.handle(AssetKind::Sprite, name).map(SpriteId).ok_or_else(|| what(format!("{field}: the packs have no sprite {name:?}")))
            }
            other => Err(what(format!("{field} is {other:?}, not a sprite"))),
        }
    };
    let body = |v: &Data, field: &str| -> Result<BodyPart, ContentError> {
        let anims = match v.field("anims") {
            Data::Int(n) if (0..=256).contains(n) => *n as usize,
            other => return Err(what(format!("{field}.anims is {other:?}, not a count of animations"))),
        };
        let mut in_front = vec![true; anims];
        let ranges = match v.field("behind") {
            Data::Nil => &[][..],
            Data::List(ranges) => ranges.as_slice(),
            other => return Err(what(format!("{field}.behind is {other:?}, not a list of `{{ first, last }}`"))),
        };
        for r in ranges {
            let (first, last) = match r {
                Data::List(r) => match r.as_slice() {
                    [Data::Int(a), Data::Int(b)] if 0 <= *a && a <= b && (*b as usize) < anims => (*a as usize, *b as usize),
                    _ => return Err(what(format!("{field}.behind has a range that is not `{{ first, last }}` within the {anims} animations"))),
                },
                _ => return Err(what(format!("{field}.behind is a list of `{{ first, last }}`"))),
            };
            in_front[first..=last].fill(false);
        }
        Ok(BodyPart {
            sprite: sprite(v.field("sprite"), &format!("{field}.sprite"))?,
            in_front,
            own_palette: flag(v.field("own_palette"), &format!("{field}.own_palette"), false)?,
            anim_offset: byte(v.field("anim_offset"), &format!("{field}.anim_offset"))?,
        })
    };
    let parts = match spec.field("parts") {
        Data::Nil => None,
        p @ Data::Map(_) => {
            let (b, second, idle, head) = (p.field("body"), p.field("second"), p.field("idle"), p.field("beast_head"));
            Some(match (b.is_nil(), second.is_nil(), idle.is_nil(), head.is_nil(), own_part) {
                (false, true, true, true, None) => Parts::Body(body(b, "parts.body")?),
                (false, false, true, true, None) => Parts::Bodies(body(b, "parts.body")?, body(second, "parts.second")?),
                (true, true, false, true, None) => Parts::Idle { sprite: sprite(idle.field("sprite"), "parts.idle.sprite")? },
                (true, true, true, false, None) => Parts::BeastHead {
                    sprite: sprite(head.field("sprite"), "parts.beast_head.sprite")?,
                    palette: match head.field("palette") {
                        Data::Nil => None,
                        v => Some(byte(v, "parts.beast_head.palette")?),
                    },
                },
                (true, true, true, true, Some(f)) => Parts::Own(f),
                _ => return Err(what("`parts` is one of `body` (with a `second`), `idle`, `beast_head` and `own`".into())),
            })
        }
        other => return Err(what(format!("`parts` is {other:?}, not a table"))),
    };
    // The hooks that touch what it wears: by default its death takes its
    // parts off, and an animation change restarts them (not an overlay
    // worn only while standing).
    let overlay_hooks = match spec.field("overlay_hooks") {
        Data::Nil => OverlayHooks {
            death: parts.is_some(),
            refresh: matches!(parts, Some(Parts::Body(_) | Parts::Bodies(..) | Parts::BeastHead { .. })),
            ..OverlayHooks::default()
        },
        Data::List(names) => {
            let mut h = OverlayHooks::default();
            for n in names {
                match n {
                    Data::Str(s) if s == "death" => h.death = true,
                    Data::Str(s) if s == "refresh" => h.refresh = true,
                    Data::Str(s) if s == "flinch" => h.flinch = true,
                    Data::Str(s) if s == "flinch_checked" => (h.flinch, h.flinch_checked) = (true, true),
                    Data::Str(s) if s == "drag" => h.drag = true,
                    Data::Str(s) if s == "lets_go" => h.lets_go = true,
                    other => return Err(what(format!("`overlay_hooks` has {other:?}, not death, refresh, flinch, flinch_checked, drag or lets_go"))),
                }
            }
            h
        }
        // (An empty table reads as a map.)
        Data::Map(m) if m.is_empty() => OverlayHooks::default(),
        other => return Err(what(format!("`overlay_hooks` is {other:?}, not a list of names"))),
    };
    let aura_anim = match spec.field("aura_anim") {
        Data::Nil => None,
        v => Some(byte(v, "aura_anim")?),
    };
    if class.is_player() && aura_anim.is_none() {
        return Err(what("a player's identity says its Full Synchro aura's animation (`aura_anim`)".into()));
    }
    let ice = match spec.field("ice") {
        Data::Nil => IceSize::Small,
        Data::Str(s) if s == "small" => IceSize::Small,
        Data::Str(s) if s == "medium" => IceSize::Medium,
        Data::Str(s) if s == "large" => IceSize::Large,
        other => return Err(what(format!("`ice` is {other:?}, not small, medium or large"))),
    };
    let body = match spec.field("body") {
        Data::Nil => None,
        b @ Data::Map(_) => {
            if actor_type != ActorType::Navi {
                return Err(what("only a navi no player controls (actor type navi) has a `body`".into()));
            }
            let collision = |field: &str| -> Result<nettai_content_api::CollisionHandle, ContentError> {
                match b.field("collision").field(field) {
                    Data::Ref(nettai_content_api::Registry::Collision, key) => definitions
                        .of(nettai_content_api::Registry::Collision)
                        .iter()
                        .position(|c| &c.key == key)
                        .map(|i| nettai_content_api::CollisionHandle(i as u16))
                        .ok_or_else(|| what(format!("body.collision.{field} names {key:?}, which isn't defined"))),
                    other => Err(what(format!("body.collision.{field} is {other:?}, not a collision type"))),
                }
            };
            let word = |field: &str| -> Result<u16, ContentError> {
                match b.field(field) {
                    Data::Int(i) => u16::try_from(*i).map_err(|_| what(format!("body.{field} {i} is not a halfword"))),
                    other => Err(what(format!("body.{field} is {other:?}, not a number"))),
                }
            };
            let hp = word("hp")?;
            if hp > 0xFFF {
                return Err(what(format!("body.hp {hp} is more than its 12 bits hold")));
            }
            const ELEMENTS: [&str; 5] = ["null", "fire", "aqua", "elec", "wood"];
            let element = match b.field("element") {
                Data::Str(s) => {
                    ELEMENTS.iter().position(|n| n == s).ok_or_else(|| what(format!("body.element {s:?} is not an element")))? as u8
                }
                other => return Err(what(format!("body.element is {other:?}, not an element"))),
            };
            let mut flags = 0u32;
            match b.field("flags") {
                Data::Nil => {}
                Data::List(names) => {
                    for n in names {
                        use crate::collision::f1;
                        flags |= match n {
                            Data::Str(s) if s == "superarmor" => f1::SUPERARMOR,
                            Data::Str(s) if s == "airshoe" => f1::AIRSHOE,
                            Data::Str(s) if s == "floatshoe" => f1::FLOATSHOE,
                            Data::Str(s) if s == "affected_by_ice" => f1::AFFECTED_BY_ICE,
                            Data::Str(s) if s == "untouchable" => f1::UNTOUCHABLE,
                            other => {
                                return Err(what(format!(
                                    "body.flags has {other:?}, not superarmor, airshoe, floatshoe, affected_by_ice or untouchable"
                                )));
                            }
                        };
                    }
                }
                other => return Err(what(format!("body.flags is {other:?}, not a list of names"))),
            }
            Some(ActorBody {
                sprite: sprite(b.field("sprite"), "body.sprite")?,
                shadow: flag(b.field("shadow"), "body.shadow", false)?,
                palette_scale: byte(b.field("palette_scale"), "body.palette_scale")?,
                palette: match b.field("palette") {
                    Data::Nil => None,
                    v => Some(byte(v, "body.palette")?),
                },
                hp,
                element,
                body_damage: word("body_damage")?,
                collision: (collision("body")?, collision("target")?, byte(b.field("collision").field("hit_modifier"), "body.collision.hit_modifier")?),
                flags,
                hides_hp: flag(b.field("hides_hp"), "body.hides_hp", false)?,
            })
        }
        other => return Err(what(format!("`body` is {other:?}, not a table"))),
    };
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
        throwable: flag(spec.field("throwable"), "throwable", true)?,
        parts,
        overlay_hooks,
        aura_anim,
        never_angers: flag(spec.field("never_angers"), "never_angers", false)?,
        ice,
        owner: None,
        body,
    })
}
