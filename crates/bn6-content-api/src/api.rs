//! [`CoreApi`]: what content can see and do. Every operation is generic
//! (objects, sprites, collision, the attack in progress, sounds); none
//! knows a particular chip.
//!
//! Engine-owned fields are named by enums ([`ObjectField`],
//! [`SpriteField`], ...). Each has a name (what scripts write), a type
//! (which conversions and wrapping apply) and whether content may write
//! it. Rust content uses the typed accessors of [`ObjectFields`] and
//! friends instead of the enums.

use std::fmt;

use crate::state::{ContentState, FieldType, TypeError, Value};
use crate::types::{ObjectRef, PanelPos, Pool, SpriteId, Vec3};

/// An object's lifecycle state (the game's 0/4/8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lifecycle {
    /// Spawned; its first update sets it up.
    Init,
    /// Running.
    Update,
    /// Ending; its next update frees it.
    Destroy,
}

impl Lifecycle {
    pub fn name(self) -> &'static str {
        match self {
            Lifecycle::Init => "init",
            Lifecycle::Update => "update",
            Lifecycle::Destroy => "destroy",
        }
    }

    pub fn from_name(s: &str) -> Option<Lifecycle> {
        [Lifecycle::Init, Lifecycle::Update, Lifecycle::Destroy].into_iter().find(|l| l.name() == s)
    }
}

/// How a sprite's shadow (the first part of each frame) is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Shadow {
    #[default]
    Hidden,
    Ground,
    WithSprite,
}

impl Shadow {
    pub const NAMES: [&'static str; 3] = ["hidden", "ground", "with_sprite"];
    pub const ALL: [Shadow; 3] = [Shadow::Hidden, Shadow::Ground, Shadow::WithSprite];
}

macro_rules! named_fields {
    (
        $(#[$m:meta])*
        pub enum $name:ident {
            $( $(#[$vm:meta])* $var:ident = $str:literal, $ty:expr, $rw:ident; )*
        }
    ) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name { $( $(#[$vm])* $var, )* }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$var,)*];

            /// The field's name in content.
            pub fn name(self) -> &'static str {
                match self { $($name::$var => $str,)* }
            }

            pub fn from_name(s: &str) -> Option<$name> {
                match s { $($str => Some($name::$var),)* _ => None }
            }

            /// The field's type.
            pub fn ty(self) -> FieldType {
                use FieldType::*;
                match self { $($name::$var => $ty,)* }
            }

            /// Whether content may write the field.
            pub fn writable(self) -> bool {
                match self { $($name::$var => named_fields!(@rw $rw),)* }
            }
        }
    };
    (@rw rw) => { true };
    (@rw ro) => { false };
}

named_fields! {
    /// The fields every object has.
    pub enum ObjectField {
        /// Which behavior within the pool.
        Index = "index", U8, ro;
        /// The current action and phase; their meaning is the kind's.
        Action = "action", U8, rw;
        Phase = "phase", U8, rw;
        PhaseInit = "phase_init", U8, rw;
        PanelX = "panel_x", U8, rw;
        PanelY = "panel_y", U8, rw;
        /// 0 = left side, 1 = right side.
        Alliance = "alliance", U8, rw;
        /// Facing is `alliance ^ flip`.
        Flip = "flip", U8, rw;
        /// The requested animation, and the one the sprite has.
        Anim = "anim", U8, rw;
        AnimLoaded = "anim_loaded", U8, rw;
        Element = "element", U8, rw;
        Timer = "timer", U16, rw;
        Timer2 = "timer2", U16, rw;
        Hp = "hp", U16, rw;
        MaxHp = "max_hp", U16, rw;
        /// Attack power plus flag bits.
        Damage = "damage", U16, rw;
        Stamina = "stamina", U16, rw;
        NameId = "name_id", U16, ro;
        Pos = "pos", Vec3, rw;
        Vel = "vel", Vec3, rw;
        Related1 = "related1", Object, rw;
        Related2 = "related2", Object, rw;
        /// Drawn this frame.
        Visible = "visible", Bool, rw;
        /// Keeps updating while the battle is paused.
        RunWhilePaused = "run_while_paused", Bool, rw;
        /// Keeps updating during time stop.
        RunInTimeStop = "run_in_time_stop", Bool, rw;
        /// The sprite doesn't animate.
        NoSpriteUpdate = "no_sprite_update", Bool, rw;
    }
}

named_fields! {
    /// An actor's (navi's) fields, including the attack in progress.
    pub enum ActorField {
        /// The sprite overlay attached for the current attack.
        Overlay = "overlay", Object, rw;
        /// The running action's phase (0, 4, 8...) and whether its entry
        /// ran.
        AttackStep = "step", U8, rw;
        AttackStepInit = "step_init", U8, rw;
        /// The attack's variant (a chip's level or subtype).
        AttackVariant = "variant", U8, ro;
        /// The chip being used.
        AttackChip = "chip", U16, ro;
    }
}

named_fields! {
    /// How an object's sprite is drawn.
    pub enum SpriteField {
        Palette = "palette", U8, rw;
        HFlip = "hflip", Bool, rw;
        VFlip = "vflip", Bool, rw;
        Shadow = "shadow", Enum(Shadow::NAMES.iter().map(|s| s.to_string()).collect()), rw;
        /// Drawn solid white (a hit flash).
        White = "white", Bool, rw;
        ColorShader = "color_shader", U16, rw;
        /// Blended at alpha/16, or not blended (nil).
        Alpha = "alpha", OptionalU8, rw;
    }
}

named_fields! {
    /// An object's collision registration.
    pub enum CollisionField {
        /// Region shape.
        Region = "region", U8, rw;
        /// Hit spark effect (0xFF = none).
        HitEffect = "hit_effect", U8, rw;
        StatusBase = "status_base", U8, rw;
        /// Bug code (low byte) and argument (high byte).
        Bugs = "bugs", U16, rw;
        /// What the last resolution hit.
        HitFlags = "hit_flags", U32, ro;
    }
}

named_fields! {
    /// A navi's stats that content reads.
    pub enum NaviStat {
        /// Fighting in the sun.
        Sun = "sun", Bool, ro;
        Form = "form", U8, ro;
        Navi = "navi", U8, ro;
    }
}

/// Status flags an object with collision data carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusFlag {
    Guard,
    Invisible,
    Invulnerable,
    Moving,
    Flashing,
    Flinching,
    Paralyzed,
    Sliding,
    Frozen,
    SuperArmor,
    MoveComplete,
    /// An action is in use (attacks set it while they run).
    UsingAction,
    Bubbled,
}

impl StatusFlag {
    pub const ALL: [StatusFlag; 13] = [
        StatusFlag::Guard,
        StatusFlag::Invisible,
        StatusFlag::Invulnerable,
        StatusFlag::Moving,
        StatusFlag::Flashing,
        StatusFlag::Flinching,
        StatusFlag::Paralyzed,
        StatusFlag::Sliding,
        StatusFlag::Frozen,
        StatusFlag::SuperArmor,
        StatusFlag::MoveComplete,
        StatusFlag::UsingAction,
        StatusFlag::Bubbled,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StatusFlag::Guard => "guard",
            StatusFlag::Invisible => "invisible",
            StatusFlag::Invulnerable => "invulnerable",
            StatusFlag::Moving => "moving",
            StatusFlag::Flashing => "flashing",
            StatusFlag::Flinching => "flinching",
            StatusFlag::Paralyzed => "paralyzed",
            StatusFlag::Sliding => "sliding",
            StatusFlag::Frozen => "frozen",
            StatusFlag::SuperArmor => "super_armor",
            StatusFlag::MoveComplete => "move_complete",
            StatusFlag::UsingAction => "using_action",
            StatusFlag::Bubbled => "bubbled",
        }
    }

    pub fn from_name(s: &str) -> Option<StatusFlag> {
        StatusFlag::ALL.into_iter().find(|f| f.name() == s)
    }
}

/// Why an API call failed. Content errors are bugs in the content: the
/// engine stops the battle, the same way on every machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiError {
    Type {
        field: &'static str,
        error: TypeError,
    },
    ReadOnly(&'static str),
    /// The object has no actor data (it isn't a navi).
    NoActor(ObjectRef),
    /// The object has no collision data.
    NoCollision(ObjectRef),
    /// The object has no content state (its kind isn't content).
    NoState(ObjectRef),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::Type { field, error } => write!(f, "field `{field}`: {error}"),
            ApiError::ReadOnly(field) => write!(f, "field `{field}` is read-only"),
            ApiError::NoActor(o) => write!(f, "{o:?} has no actor data"),
            ApiError::NoCollision(o) => write!(f, "{o:?} has no collision data"),
            ApiError::NoState(o) => write!(f, "{o:?} has no content state"),
        }
    }
}

impl std::error::Error for ApiError {}

pub type ApiResult<T> = Result<T, ApiError>;

/// The engine, as content sees it. Implemented by the battle engine;
/// called by content (native or through a script runtime).
pub trait CoreApi {
    // ---- The battle ------------------------------------------------------

    fn is_time_stop(&self) -> bool;
    fn is_paused(&self) -> bool;
    /// Report a sound effect (output only; nothing in the simulation reads
    /// it).
    fn play_sound(&mut self, sound: u16);
    fn navi_stat(&self, side: u8, stat: NaviStat) -> Value;
    fn panel_valid(&self, p: PanelPos) -> bool;
    /// The center of a panel, 16.16 (x, y).
    fn panel_center(&self, p: PanelPos) -> (i32, i32);

    // ---- Objects -----------------------------------------------------------

    /// Spawn an object: it takes the pool's lowest free slot and runs later
    /// this tick, right after the object that spawned it. None if the
    /// pool is full. A content kind starts with its zeroed state.
    fn spawn(&mut self, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef>;
    /// Free the slot now (the object stops running).
    fn free(&mut self, o: ObjectRef);
    /// The usual end: release panel reservations and collision, then free.
    fn destroy(&mut self, o: ObjectRef);
    fn lifecycle(&self, o: ObjectRef) -> Lifecycle;
    /// Enter a lifecycle state, from its first action and phase.
    fn set_lifecycle(&mut self, o: ObjectRef, l: Lifecycle);
    /// Spawn parameter `n` (0..4).
    fn param(&self, o: ObjectRef, n: usize) -> u8;
    fn get(&self, o: ObjectRef, f: ObjectField) -> Value;
    fn set(&mut self, o: ObjectRef, f: ObjectField, v: Value) -> ApiResult<()>;
    /// +1 facing right, -1 facing left.
    fn facing(&self, o: ObjectRef) -> i32;
    /// Request animation `anim`, restarting it even if it is current.
    fn set_animation(&mut self, o: ObjectRef, anim: u8);
    /// Load a newly requested animation and step the sprite, unless paused
    /// or stopped.
    fn update_sprite(&mut self, o: ObjectRef);
    /// The object's sprite attach point `n`, in pixels, facing its way.
    fn attach_point(&self, o: ObjectRef, n: u8) -> (i32, i32);
    /// The object's content state (None for kinds the engine implements).
    fn state(&self, o: ObjectRef) -> Option<&ContentState>;
    fn state_mut(&mut self, o: ObjectRef) -> Option<&mut ContentState>;

    // ---- Actors and the attack in progress -------------------------------------

    fn actor_get(&self, o: ObjectRef, f: ActorField) -> ApiResult<Value>;
    fn actor_set(&mut self, o: ObjectRef, f: ActorField, v: Value) -> ApiResult<()>;
    /// The running content action's state, zeroed when a different action
    /// last used it.
    fn action_state_mut(&mut self, o: ObjectRef) -> ApiResult<&mut ContentState>;
    fn status(&self, o: ObjectRef, flag: StatusFlag) -> ApiResult<bool>;
    fn set_status(&mut self, o: ObjectRef, flag: StatusFlag, on: bool) -> ApiResult<()>;
    /// Open the attack's counter window.
    fn open_counter_window(&mut self, o: ObjectRef);
    /// The reactive-defense abort attacks check after each phase.
    fn check_reactive_abort(&mut self, o: ObjectRef);
    /// End the attack: back to the idle action with animation 0.
    fn exit_attack(&mut self, o: ObjectRef);

    // ---- Sprites -------------------------------------------------------------

    fn sprite_load(&mut self, o: ObjectRef, id: SpriteId);
    /// Start animation `anim` from its first frame.
    fn sprite_set_animation(&mut self, o: ObjectRef, anim: u8);
    /// Advance the animation one tick.
    fn sprite_step(&mut self, o: ObjectRef);
    fn sprite_get(&self, o: ObjectRef, f: SpriteField) -> Value;
    fn sprite_set(&mut self, o: ObjectRef, f: SpriteField, v: Value) -> ApiResult<()>;

    // ---- Collision -------------------------------------------------------------

    /// Give the object a collision slot; false if none is free.
    fn create_collision(&mut self, o: ObjectRef) -> bool;
    /// Set up the registration from the object: its side, panel, element
    /// and damage, and what it is and reacts to (collision type indices).
    fn setup_collision(&mut self, o: ObjectRef, self_type: u8, target_type: u8, hit_mod: u8);
    fn collision_get(&self, o: ObjectRef, f: CollisionField) -> ApiResult<Value>;
    fn collision_set(&mut self, o: ObjectRef, f: CollisionField, v: Value) -> ApiResult<()>;
    /// Register on the region's panels (clearing the last results).
    fn present_collision(&mut self, o: ObjectRef);
    /// Unregister, resolving hits against whatever is registered there.
    fn remove_collision(&mut self, o: ObjectRef);
    fn free_collision(&mut self, o: ObjectRef);
    /// Show the hit spark of a registration that just hit something.
    fn hit_spark(&mut self, o: ObjectRef);
}

/// Conversion out of a [`Value`] for typed accessors.
pub trait FromValue: Sized {
    fn from_value(v: Value) -> Self;
}

/// Conversion into a [`Value`] for typed accessors.
pub trait IntoValue {
    fn into_value(self) -> Value;
}

macro_rules! int_value {
    ($($t:ty),*) => {$(
        impl FromValue for $t {
            fn from_value(v: Value) -> $t {
                v.int().unwrap_or_else(|| panic!("expected an integer, got {v:?}")) as $t
            }
        }
        impl IntoValue for $t {
            fn into_value(self) -> Value {
                Value::Int(self as i64)
            }
        }
    )*};
}
int_value!(u8, u16, u32, i8, i16, i32);

impl FromValue for bool {
    fn from_value(v: Value) -> bool {
        match v {
            Value::Bool(b) => b,
            v => panic!("expected a bool, got {v:?}"),
        }
    }
}

impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}

impl FromValue for Option<ObjectRef> {
    fn from_value(v: Value) -> Option<ObjectRef> {
        match v {
            Value::Nil => None,
            Value::Object(o) => Some(o),
            v => panic!("expected an object, got {v:?}"),
        }
    }
}

impl IntoValue for Option<ObjectRef> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl FromValue for Vec3 {
    fn from_value(v: Value) -> Vec3 {
        match v {
            Value::Vec3(p) => p,
            v => panic!("expected a vec3, got {v:?}"),
        }
    }
}

impl IntoValue for Vec3 {
    fn into_value(self) -> Value {
        Value::Vec3(self)
    }
}

impl FromValue for Option<u8> {
    fn from_value(v: Value) -> Option<u8> {
        match v {
            Value::Nil => None,
            v => Some(u8::from_value(v)),
        }
    }
}

impl IntoValue for Option<u8> {
    fn into_value(self) -> Value {
        self.map_or(Value::Nil, |v| Value::Int(v as i64))
    }
}

macro_rules! typed_accessors {
    ($trait:ident, $field:ident, $get:ident, $set:ident; $($var:ident => $getter:ident / $setter:ident : $t:ty;)*) => {
        /// Typed accessors (for Rust content) over the generic field calls.
        pub trait $trait: CoreApi {
            $(
                fn $getter(&self, o: ObjectRef) -> $t {
                    <$t>::from_value(typed_accessors!(@unwrap self.$get(o, $field::$var)))
                }
                fn $setter(&mut self, o: ObjectRef, v: $t) {
                    self.$set(o, $field::$var, v.into_value()).unwrap_or_else(|e| panic!("{e}"))
                }
            )*
        }
        impl<A: CoreApi + ?Sized> $trait for A {}
    };
    (@unwrap $e:expr) => { ApiResultOrValue::value($e) };
}

/// Lets the accessor macro take getters that return a value or a result.
trait ApiResultOrValue {
    fn value(self) -> Value;
}

impl ApiResultOrValue for Value {
    fn value(self) -> Value {
        self
    }
}

impl ApiResultOrValue for ApiResult<Value> {
    fn value(self) -> Value {
        self.unwrap_or_else(|e| panic!("{e}"))
    }
}

typed_accessors! {
    ObjectFields, ObjectField, get, set;
    Action => action / set_action: u8;
    Phase => phase / set_phase: u8;
    PhaseInit => phase_init / set_phase_init: u8;
    PanelX => panel_x / set_panel_x: u8;
    PanelY => panel_y / set_panel_y: u8;
    Alliance => alliance / set_alliance: u8;
    Flip => flip / set_flip: u8;
    Anim => anim / set_anim: u8;
    AnimLoaded => anim_loaded / set_anim_loaded: u8;
    Element => element / set_element: u8;
    Timer => timer / set_timer: u16;
    Damage => damage / set_damage: u16;
    Stamina => stamina / set_stamina: u16;
    Pos => pos / set_pos: Vec3;
    Related1 => related1 / set_related1: Option<ObjectRef>;
    Related2 => related2 / set_related2: Option<ObjectRef>;
    Visible => visible / set_visible: bool;
    RunWhilePaused => run_while_paused / set_run_while_paused: bool;
    RunInTimeStop => run_in_time_stop / set_run_in_time_stop: bool;
    NoSpriteUpdate => no_sprite_update / set_no_sprite_update: bool;
}

typed_accessors! {
    ActorFields, ActorField, actor_get, actor_set;
    Overlay => overlay / set_overlay: Option<ObjectRef>;
    AttackStep => step / set_step: u8;
    AttackStepInit => step_init / set_step_init: u8;
}

typed_accessors! {
    SpriteFields, SpriteField, sprite_get, sprite_set;
    Palette => palette / set_palette: u8;
    HFlip => hflip / set_hflip: bool;
    VFlip => vflip / set_vflip: bool;
    White => white / set_white: bool;
    ColorShader => color_shader / set_color_shader: u16;
    Alpha => alpha / set_alpha: Option<u8>;
}

typed_accessors! {
    CollisionFields, CollisionField, collision_get, collision_set;
    Region => region / set_region: u8;
    HitEffect => hit_effect / set_hit_effect: u8;
    StatusBase => status_base / set_status_base: u8;
    Bugs => bugs / set_bugs: u16;
}

/// Accessors that don't fit the get/set pattern.
pub trait OtherFields: CoreApi {
    fn index(&self, o: ObjectRef) -> u8 {
        u8::from_value(self.get(o, ObjectField::Index))
    }
    fn name_id(&self, o: ObjectRef) -> u16 {
        u16::from_value(self.get(o, ObjectField::NameId))
    }
    fn variant(&self, o: ObjectRef) -> u8 {
        u8::from_value(self.actor_get(o, ActorField::AttackVariant).unwrap_or_else(|e| panic!("{e}")))
    }
    /// The chip the navi's attack is using.
    fn chip(&self, o: ObjectRef) -> u16 {
        u16::from_value(self.actor_get(o, ActorField::AttackChip).unwrap_or_else(|e| panic!("{e}")))
    }
    fn hit_flags(&self, o: ObjectRef) -> u32 {
        u32::from_value(self.collision_get(o, CollisionField::HitFlags).unwrap_or_else(|e| panic!("{e}")))
    }
    fn set_shadow(&mut self, o: ObjectRef, s: Shadow) {
        let i = Shadow::ALL.iter().position(|&x| x == s).unwrap_or(0);
        self.sprite_set(o, SpriteField::Shadow, Value::Int(i as i64)).unwrap_or_else(|e| panic!("{e}"))
    }
}

impl<A: CoreApi + ?Sized> OtherFields for A {}
