//! [`CoreApi`]: what content can see and do. Every operation is generic
//! (objects, sprites, collision, panels, the navi framework, the ruleset's
//! services); none knows a particular chip.
//!
//! Engine-owned fields are named by enums ([`ObjectField`],
//! [`ActorField`], [`SpriteField`], ...). Each has a name (what scripts
//! write), a type (which conversions and wrapping apply) and whether
//! content may write it. Flags and timers are named too
//! ([`StatusFlag`], [`RequestFlag`], [`Key`] ...): content never handles
//! the engine's bit values or offsets.

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
    /// The fourth state (the game's 0xC) of kinds that have one after
    /// `Destroy`: for them `Destroy` is a last running phase, and this one
    /// frees them.
    Finish,
}

impl Lifecycle {
    pub fn name(self) -> &'static str {
        match self {
            Lifecycle::Init => "init",
            Lifecycle::Update => "update",
            Lifecycle::Destroy => "destroy",
            Lifecycle::Finish => "finish",
        }
    }

    pub fn from_name(s: &str) -> Option<Lifecycle> {
        [Lifecycle::Init, Lifecycle::Update, Lifecycle::Destroy, Lifecycle::Finish].into_iter().find(|l| l.name() == s)
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

/// Panel types by name, in the engine's order (the type number is the
/// index).
pub const PANEL_TYPES: [&str; 13] = [
    "missing",
    "broken",
    "normal",
    "cracked",
    "poison",
    "holy",
    "grass",
    "ice",
    "volcano",
    "road_up",
    "road_down",
    "road_left",
    "road_right",
];

/// Actor types by name (an actor record's type).
pub const ACTOR_TYPES: [&str; 3] = ["virus", "navi", "player"];

fn enum_type(names: &[&str]) -> FieldType {
    FieldType::Enum(names.iter().map(|s| s.to_string()).collect())
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
                #[allow(unused_imports)]
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
        /// The panel a move is going to.
        FuturePanelX = "future_panel_x", U8, rw;
        FuturePanelY = "future_panel_y", U8, rw;
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
        /// Attack power plus flag bits (the damage word's low half).
        Damage = "damage", U16, rw;
        /// The damage word's high half (the counter byte in its low byte).
        Stamina = "stamina", U16, rw;
        NameId = "name_id", U16, rw;
        /// Holds an object's sprite (and more, by kind: `PreventAnim`).
        PreventAnim = "prevent_anim", U8, rw;
        Pos = "pos", Vec3, rw;
        Vel = "vel", Vec3, rw;
        Related1 = "related1", Object, rw;
        Related2 = "related2", Object, rw;
        /// The slot is in use.
        Active = "active", Bool, ro;
        /// Drawn this frame.
        Visible = "visible", Bool, rw;
        /// Keeps updating while the battle is paused.
        RunWhilePaused = "run_while_paused", Bool, rw;
        /// Keeps updating while the battle is dimmed.
        RunWhileDimmed = "run_while_dimmed", Bool, rw;
        /// The sprite doesn't animate.
        NoSpriteUpdate = "no_sprite_update", Bool, rw;
    }
}

named_fields! {
    /// An actor's (a navi's) fields: the attack in progress (the game's
    /// AIAttackVars) and the actor data (AIData) content uses.
    pub enum ActorField {
        /// The sprite overlay attached for the current attack.
        Overlay = "overlay", Object, rw;
        /// The running action's phase (0, 4, 8...) and whether its entry
        /// ran.
        Step = "step", U8, rw;
        StepInit = "step_init", U8, rw;
        /// The attack's variant (a chip's subtype, a weapon's variant).
        Variant = "variant", U8, rw;
        /// The chip being used (0 for weapons).
        Chip = "chip", U16, rw;
        /// The attack's element byte (primary | secondary bits).
        AttackElement = "attack_element", U8, rw;
        /// The attack's damage (with its flag bits).
        AttackDamage = "attack_damage", U16, rw;
        /// The attack's hit parameter (its hitbox's counter byte).
        HitParam = "hit_param", U16, rw;
        Charged = "charged", U8, rw;
        /// Lockout the attack applies when it ends (kind 2: the chip
        /// lockout; kind 3: the B+Back cooldown).
        AttackLockout = "attack_lockout", U8, rw;
        /// The Atk+ / cross bonus.
        Extra = "extra", U16, rw;
        SpecialSource = "special_source", U8, rw;
        /// Which `set_attack` kind started the action (0..5).
        AttackKind = "attack_kind", U8, ro;
        /// 1 while the action runs inside a form's action wrapper.
        BeastLockon = "beast_lockon", U8, rw;
        /// A per-action word some actions keep (a move's direction change,
        /// a thrown obstacle).
        Marker = "marker", U32, rw;
        ActorType = "actor_type", enum_type(&ACTOR_TYPES), ro;
        /// Form or AI variant.
        AiIndex = "ai_index", U8, ro;
        /// The Beast Out lock-on marker.
        LockonMarker = "lockon_marker", Object, rw;
        /// The charge glow.
        ChargeGlow = "charge_glow", Object, rw;
        /// The Full Synchro aura.
        FullSynchroAura = "full_synchro_aura", Object, rw;
        /// The charge: 0 none, 1 charging, 2 full; its source (0 none, 1
        /// A, 2 B); its counter.
        ChargeLevel = "charge_level", U8, ro;
        ChargeSource = "charge_source", U8, ro;
        ChargeCounter = "charge_counter", U8, ro;
        /// A buffered auto-step's direction (0 none).
        BufferedMove = "buffered_move", U8, rw;
        /// Ticks before the next chip can be used.
        ChipLockout = "chip_lockout", U8, rw;
        /// Ticks before the B+Back special can be input again.
        BackSpecialCooldown = "back_special_cooldown", U8, rw;
        /// The weapon routines the navi's buttons run (0xFF = none).
        BusterRoutine = "buster_routine", U8, ro;
        ChargeShotRoutine = "charge_shot_routine", U8, ro;
        BackSpecialRoutine = "back_special_routine", U8, ro;
        AChargeRoutine = "a_charge_routine", U8, ro;
        AltAChargeRoutine = "alt_a_charge_routine", U8, ro;
        Mode9ARoutine = "mode9_a_routine", U8, ro;
    }
}

named_fields! {
    /// How an object's sprite is drawn, and where its animation is.
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
        /// Mosaic blocks of n + 1 pixels, or none (nil).
        Mosaic = "mosaic", OptionalU8, rw;
        /// Priority against the background layers (0 = frontmost).
        Priority = "priority", U8, rw;
        /// Parts not drawn: bit 31 - i hides part i.
        HiddenParts = "hidden_parts", U32, rw;
        /// The animation the sprite plays.
        Animation = "animation", U8, ro;
        /// The current frame's flags (0x80 last, 0x40 loop), only on the
        /// tick the frame ends.
        FrameFlags = "frame_flags", U8, ro;
        /// The animation has played to its end.
        Finished = "finished", Bool, ro;
    }
}

named_fields! {
    /// An object's collision registration.
    pub enum CollisionField {
        /// Region shape.
        Region = "region", U8, rw;
        /// The panel the region is anchored on (the game's CollisionData
        /// PanelX/PanelY; `update_collision_panels` copies the object's).
        PanelX = "panel_x", U8, rw;
        PanelY = "panel_y", U8, rw;
        /// Hit spark effect (0xFF = none).
        HitEffect = "hit_effect", U8, rw;
        StatusBase = "status_base", U8, rw;
        /// Bug code (low byte) and argument (high byte).
        Bugs = "bugs", U16, rw;
        HitModBase = "hit_mod_base", U8, rw;
        /// The damage it deals (the object's damage at setup).
        SelfDamage = "self_damage", U16, rw;
        /// What the last resolution hit.
        HitFlags = "hit_flags", U32, ro;
        /// The damage taken this window.
        FinalDamage = "final_damage", U16, ro;
    }
}

named_fields! {
    /// A side's navi stats that content reads.
    pub enum NaviStat {
        /// Fighting in the sun.
        Sun = "sun", Bool, ro;
        Form = "form", U8, ro;
        Navi = "navi", U8, ro;
        NaviVariant = "navi_variant", U8, ro;
        /// The base form's element.
        Element = "element", U8, ro;
        Attack = "attack", U8, ro;
        Rapid = "rapid", U8, ro;
        Charge = "charge", U8, ro;
        Mood = "mood", U8, ro;
        BeastOutCounter = "beast_out_counter", U8, ro;
        MaxBaseHp = "max_base_hp", U16, ro;
        /// The NaviCust's heal on chip use.
        ChipRecovery = "chip_recovery", U16, ro;
        /// NaviCust weapon stats: the buster's spread shot, the charged
        /// shot's kind.
        BusterShot = "buster_shot", U8, ro;
        ChargeShotKind = "charge_shot_kind", U8, ro;
        /// NaviCust bugs: buster blanks and buster charged shots (of 16).
        BusterBlanks = "buster_blanks", U8, ro;
        BusterCharged = "buster_charged", U8, ro;
        /// The form is a Beast form, Beast Over.
        Beast = "beast", Bool, ro;
        BeastOver = "beast_over", Bool, ro;
    }
}

named_fields! {
    /// Battle-wide state content reads.
    pub enum BattleInfo {
        /// A link (net) battle.
        Link = "link", Bool, ro;
        Mode = "mode", U8, ro;
        PanelPattern = "panel_pattern", U8, ro;
        /// Every navi is in (the intro's bit 2).
        NavisIn = "navis_in", Bool, ro;
        /// Presentation only: the side the simulation's perspective is.
        LocalSide = "local_side", U8, ro;
    }
}

macro_rules! named_flags {
    ($(#[$m:meta])* pub enum $name:ident { $($(#[$vm:meta])* $var:ident = $str:literal,)* }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name { $($(#[$vm])* $var,)* }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$var,)*];

            pub fn name(self) -> &'static str {
                match self { $($name::$var => $str,)* }
            }

            pub fn from_name(s: &str) -> Option<$name> {
                match s { $($str => Some($name::$var),)* _ => None }
            }
        }
    };
}

named_flags! {
    /// Status flags an object with collision data carries.
    pub enum StatusFlag {
        Guard = "guard",
        Invisible = "invisible",
        Submerged = "submerged",
        Invulnerable = "invulnerable",
        AirShoe = "air_shoe",
        FloatShoe = "float_shoe",
        Moving = "moving",
        Dead = "dead",
        Flashing = "flashing",
        Flinching = "flinching",
        Paralyzed = "paralyzed",
        Sliding = "sliding",
        Blind = "blind",
        Immobilized = "immobilized",
        Confused = "confused",
        Frozen = "frozen",
        SuperArmor = "super_armor",
        Undershirt = "undershirt",
        MoveComplete = "move_complete",
        Drag = "drag",
        Anger = "anger",
        /// An action is in use (attacks set it while they run).
        UsingAction = "using_action",
        AffectedByIce = "affected_by_ice",
        Bubbled = "bubbled",
    }
}

named_flags! {
    /// Status timers an object with collision data carries.
    pub enum StatusTimer {
        Paralyze = "paralyze",
        Confuse = "confuse",
        Blind = "blind",
        Immobilize = "immobilize",
        /// Flashing (and invisible while it runs, with the invisible flag).
        Flash = "flash",
        Submerged = "submerged",
        Invulnerable = "invulnerable",
        Freeze = "freeze",
        Bubble = "bubble",
    }
}

named_flags! {
    /// A navi's action requests (from input and from reactions).
    pub enum RequestFlag {
        Buster = "buster",
        ChargedShot = "charged_shot",
        Chip = "chip",
        ChargedChip = "charged_chip",
        BackSpecial = "back_special",
        ForcedChargedShot = "forced_charged_shot",
        RevertForm = "revert_form",
        AntiDamageTriggered = "anti_damage_triggered",
        AntiSwordTriggered = "anti_sword_triggered",
        CutIn = "cut_in",
        TurnL = "turn_l",
        TurnR = "turn_r",
        FormChange = "form_change",
        BodyGuardTriggered = "body_guard_triggered",
        AltChip = "alt_chip",
        AHeld = "a_held",
        BHeld = "b_held",
        StunStrike = "stun_strike",
        SelectSpecial = "select_special",
        CrossChange = "cross_change",
        CrossDeath = "cross_death",
        Mode9A = "mode9_a",
        CrossSpecial = "cross_special",
        Volley = "volley",
        WeaknessHit = "weakness_hit",
    }
}

named_flags! {
    /// A navi's state bits.
    pub enum NaviState {
        Controllable = "controllable",
        ChipInProgress = "chip_in_progress",
        FormChange = "form_change",
        RevertingForm = "reverting_form",
        NoCharge = "no_charge",
        CanTurn = "can_turn",
        TrapArmed = "trap_armed",
        ChangingCross = "changing_cross",
        CrossKnockout = "cross_knockout",
        Crossed = "crossed",
        Volley = "volley",
        Uninterruptible = "uninterruptible",
        CrossBreaking = "cross_breaking",
        FormChangeSpriteHeld = "form_change_sprite_held",
        HeatTrap = "heat_trap",
    }
}

named_flags! {
    /// A button.
    pub enum Key {
        A = "a",
        B = "b",
        Select = "select",
        Start = "start",
        Right = "right",
        Left = "left",
        Up = "up",
        Down = "down",
        R = "r",
        L = "l",
    }
}

named_flags! {
    /// Which of a navi's button records: the normal one, or the one kept
    /// while the battle is dimmed.
    pub enum Pad {
        Held = "held",
        Pressed = "pressed",
        Released = "released",
        DimmedHeld = "dimmed_held",
        DimmedPressed = "dimmed_pressed",
        DimmedReleased = "dimmed_released",
    }
}

named_flags! {
    /// A navi's emotion, as its mugshot shows it (`sub_8015B54`).
    pub enum Emotion {
        Normal = "normal",
        /// The Beast Out counter is spent.
        Tired = "tired",
        FullSynchro = "full_synchro",
        Angry = "angry",
        /// Mood 0, or exhausted after Beast Over.
        WornOut = "worn_out",
    }
}

named_flags! {
    /// A step of a dimming controller, run by the dimming service
    /// (`dimming.rs`): a dimming chip's controller calls them from its
    /// update, in the order its routine does.
    pub enum DimmingStep {
        /// `object_timefreezeBegin`: the controller's init; starts the
        /// dimming if its side started it.
        Begin = "begin",
        /// `object_dimScreen`: dim the screen, then the next action.
        DimScreen = "dim_screen",
        /// `object_drawChipName`: the telop, the wait for a cut-in by
        /// the other side, then the effect (the next action).
        ShowTelop = "show_telop",
        /// `sub_800BBA8`: a hidden chip's telop (the trap chips).
        ShowHiddenTelop = "show_hidden_telop",
        /// `sub_800BDB2`: a navi chip's AntiNavi check.
        CheckAntiNavi = "check_anti_navi",
        /// `sub_800BA8A`: a navi chip's telop.
        ShowNaviTelop = "show_navi_telop",
        /// `object_undimScreen`: brighten the screen, then end.
        UndimScreen = "undim_screen",
        /// `object_timefreezeEnd`: the controller's state 8; ends the
        /// dimming once both sides are done, and frees the controller.
        Finish = "finish",
    }
}

/// A one-tick hit region (attack object #3), as `object_spawnCollisionRegion`
/// takes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HitboxSpec {
    pub panel: PanelPos,
    pub element: u8,
    pub z: i32,
    /// Region shape.
    pub region: u8,
    pub hit_effect: u8,
    /// Collision type indices.
    pub target: u8,
    pub self_type: u8,
    /// The damage word (damage | flag bits) and the counter byte.
    pub damage: u16,
    pub stamina: u16,
    pub hit_mod: u8,
    pub status: u8,
    pub bug: u8,
    pub bug_arg: u8,
}

/// A panel as content sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelInfo {
    /// Index into [`PANEL_TYPES`].
    pub kind: u8,
    /// Its owner now, and at the start of the round.
    pub alliance: u8,
    pub home: u8,
}

/// A field column's stolen-area bookkeeping as content sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnInfo {
    /// Whose home the column is.
    pub home: u8,
    /// Ticks before a stolen panel of it comes back.
    pub timer: u16,
}

/// A side's defensive-chip record (the linked registry).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinkedChip {
    pub chip: u16,
    pub bonus: u16,
    /// The damage word.
    pub damage: u32,
    /// The navi that used the chip; its deletion clears the record.
    pub owner: Option<ObjectRef>,
    /// The object that implements it, if any.
    pub object: Option<ObjectRef>,
}

/// A side's damage-carry record (`dword_203CFB0`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DamageCarryInfo {
    /// The side's largest damage this tick, and last tick.
    pub this_tick: u16,
    pub previous: u16,
    /// The object that set the record, and the target that takes last
    /// tick's damage on top of its own.
    pub source: Option<ObjectRef>,
    pub target: Option<ObjectRef>,
}

/// A NameID's actor record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NaviRecordInfo {
    /// Index into [`ACTOR_TYPES`].
    pub actor_type: u8,
    pub ai_index: u8,
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
    /// No object kind has this name.
    UnknownKind(String),
    /// Anything else, described.
    Other(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::Type { field, error } => write!(f, "field `{field}`: {error}"),
            ApiError::ReadOnly(field) => write!(f, "field `{field}` is read-only"),
            ApiError::NoActor(o) => write!(f, "{o:?} has no actor data"),
            ApiError::NoCollision(o) => write!(f, "{o:?} has no collision data"),
            ApiError::NoState(o) => write!(f, "{o:?} has no content state"),
            ApiError::UnknownKind(name) => write!(f, "no object kind is named {name:?}"),
            ApiError::Other(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for ApiError {}

pub type ApiResult<T> = Result<T, ApiError>;

/// The engine, as content sees it. Implemented by the battle engine;
/// called by content through a script runtime.
///
/// Every call acts at once: an object spawned by content runs later in
/// the same tick, right after its spawner; a hit resolves when the later
/// of two objects removes its collision. Nothing is batched or deferred.
pub trait CoreApi {
    // ---- The battle ------------------------------------------------------

    /// Battle flag 4: the screen is dimmed and only objects that run while
    /// dimmed update.
    fn is_dimmed(&self) -> bool;
    fn is_paused(&self) -> bool;
    /// `battle_isBattleOver`: a side has no navi left, or time is up.
    fn is_battle_over(&self) -> bool;
    /// `battle_isBattleOver` as the routines that read its Z flag see it:
    /// over only once time is up (a KO reads as not over).
    fn is_time_up(&self) -> bool;
    fn battle_info(&self, f: BattleInfo) -> Value;
    /// Report a sound effect both players hear (output only; nothing in
    /// the simulation reads it).
    fn play_sound(&mut self, sound: u16);
    /// Report a sound only `side`'s player hears.
    fn play_sound_for(&mut self, side: u8, sound: u16);
    fn navi_stat(&self, side: u8, stat: NaviStat) -> Value;
    /// A side's emotion (`sub_8015B54`).
    fn emotion(&self, side: u8) -> Emotion;
    /// Set a side's mood, unless its navi's emotion is held (`sub_8015BEC`).
    fn set_mood(&mut self, side: u8, mood: u8);
    /// A side's player navi.
    fn player(&self, side: u8) -> Option<ObjectRef>;
    /// A side's combatants still in, in slot order.
    fn alive_actors(&self, side: u8) -> Vec<ObjectRef>;
    /// `GetRNG2`: one draw of the simulation's RNG.
    fn rng(&mut self) -> u32;
    /// `GetPositiveSignedRNG2`: one draw, bit 31 cleared.
    fn rng_positive(&mut self) -> u32;
    /// `AddRandomVarianceToTwoCoords`: jitter x and z by up to mask/2
    /// pixels (one draw).
    fn jitter(&mut self, mask: u32, pos: Vec3) -> Vec3;
    /// A side's hand: the chip at `i` (0xFFFF = none), and the cursor.
    fn hand_chip(&self, side: u8, i: u8) -> u16;
    fn hand_cursor(&self, side: u8) -> u8;
    /// `sub_800FC7C`: the cursor moves to the next chip.
    fn advance_hand(&mut self, side: u8);
    /// A side's defensive-chip record.
    fn linked(&self, side: u8) -> LinkedChip;
    fn set_linked(&mut self, side: u8, rec: LinkedChip);
    /// `sub_802CEA6`: clear it, telling its object to end.
    fn clear_linked(&mut self, side: u8);
    /// FullCust: the custom gauge is full.
    fn fill_custom_gauge(&mut self);
    /// `sub_800AB46`: bump a side's statistics counter.
    fn bump_side_stat(&mut self, side: u8, index: u8, n: u8);
    /// A player NameID's actor record, if it is one.
    fn navi_record(&self, name_id: u16) -> Option<NaviRecordInfo>;
    /// The damage-carry record of the side that takes the damage
    /// (`dword_203CFB0`; CopyDmg's): what that side took this tick and
    /// last tick, the object that set it, and the target that also takes
    /// last tick's damage.
    fn damage_carry(&self, side: u8) -> DamageCarryInfo;
    fn set_damage_carry(&mut self, side: u8, rec: DamageCarryInfo);

    // ---- Panels -----------------------------------------------------------

    fn panel_valid(&self, p: PanelPos) -> bool;
    /// The center of a panel, 16.16 (x, y).
    fn panel_center(&self, p: PanelPos) -> (i32, i32);
    /// A panel's cached flags word (0 off the field).
    fn panel_flags(&self, p: PanelPos) -> u32;
    /// The flags word has every `require` bit and no `forbid` bit.
    fn panel_check(&self, p: PanelPos, require: u32, forbid: u32) -> bool;
    fn panel_info(&self, p: PanelPos) -> Option<PanelInfo>;
    fn column_info(&self, x: u8) -> ColumnInfo;
    fn set_column_timer(&mut self, x: u8, ticks: u16);
    /// `object_setPanelAlliance`.
    fn set_panel_alliance(&mut self, p: PanelPos, side: u8);
    /// `_object_setPanelType` (an index into [`PANEL_TYPES`]).
    fn set_panel_type(&mut self, p: PanelPos, kind: u8);
    /// `object_crackPanel`: crack a solid panel, or break a cracked,
    /// unoccupied one.
    fn crack_panel(&mut self, p: PanelPos) -> bool;
    /// `object_breakPanel`: break a solid, unoccupied panel, cracked or
    /// not.
    fn break_panel(&mut self, p: PanelPos) -> bool;
    /// `object_isPanelSolid`: the panel is solid (something can stand on
    /// it).
    fn panel_solid(&self, p: PanelPos) -> bool;
    /// `object_highlightPanel` (drawn only).
    fn highlight_panel(&mut self, p: PanelPos);
    /// `object_reservePanel`.
    fn reserve_panel(&mut self, o: ObjectRef, p: PanelPos) -> bool;
    /// `object_removePanelReserve`.
    fn unreserve_panel(&mut self, o: ObjectRef, p: PanelPos) -> bool;
    /// `sub_801BB78`.
    fn release_reservations(&mut self, o: ObjectRef);
    /// `sub_800E618`: may `o` step onto `p`?
    fn can_step(&self, o: ObjectRef, p: PanelPos) -> bool;
    /// `sub_800E680`: could `o` stand on `p`, whichever side owns it?
    fn can_stand_any_side(&self, o: ObjectRef, p: PanelPos) -> bool;

    // ---- Objects -----------------------------------------------------------

    /// Spawn an object: it takes the pool's lowest free slot and runs later
    /// this tick, right after the object that spawned it. None if the
    /// pool is full. A content kind starts with its zeroed state.
    fn spawn(&mut self, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef>;
    /// Spawn the content object kind named `name` (its folder in the pack).
    fn spawn_kind(&mut self, name: &str, pos: Vec3, params: [u8; 4]) -> ApiResult<Option<ObjectRef>>;
    /// `sub_8003374` (attacks) / `sub_800333C` (actors): the same, at the
    /// end of the update list rather than right after the spawner.
    fn spawn_kind_at_end(&mut self, name: &str, pos: Vec3, params: [u8; 4]) -> ApiResult<Option<ObjectRef>>;
    /// Free the slot now (the object stops running).
    fn free(&mut self, o: ObjectRef);
    /// `object_genericDestroy`: release panel reservations and collision,
    /// then free.
    fn destroy(&mut self, o: ObjectRef);
    fn lifecycle(&self, o: ObjectRef) -> Lifecycle;
    /// Enter a lifecycle state, from its first action and phase (the
    /// game's word store of state, action, phase and phase init).
    fn set_lifecycle(&mut self, o: ObjectRef, l: Lifecycle);
    /// Change only the lifecycle state, keeping the action and phase (the
    /// game's byte store).
    fn set_lifecycle_only(&mut self, o: ObjectRef, l: Lifecycle);
    /// Switch to `action` from its first phase.
    fn set_action(&mut self, o: ObjectRef, action: u8);
    /// Spawn parameter `n` (0..4).
    fn param(&self, o: ObjectRef, n: usize) -> u8;
    fn set_param(&mut self, o: ObjectRef, n: usize, v: u8);
    fn get(&self, o: ObjectRef, f: ObjectField) -> Value;
    fn set(&mut self, o: ObjectRef, f: ObjectField, v: Value) -> ApiResult<()>;
    /// +1 facing right, -1 facing left.
    fn facing(&self, o: ObjectRef) -> i32;
    /// `object_setAnimation`: request animation `anim`, restarting it even
    /// if it is current.
    fn set_animation(&mut self, o: ObjectRef, anim: u8);
    /// `object_updateSprite`: load a newly requested animation and step
    /// the sprite, unless paused, dimmed (for objects that don't run while
    /// dimmed) or held.
    fn update_sprite(&mut self, o: ObjectRef);
    /// `object_updateSpriteTimestop`: the same, but only skipped while
    /// paused.
    fn update_sprite_while_dimmed(&mut self, o: ObjectRef);
    /// `sub_801BCD0`: the same, paused or not.
    fn step_sprite(&mut self, o: ObjectRef);
    /// `object_updateSpritePaused`: load a newly requested animation and
    /// step the sprite, paused or not, but not while dimmed (and whatever
    /// `no_sprite_update` says).
    fn update_sprite_while_paused(&mut self, o: ObjectRef);
    /// The object's sprite attach point `n`, in pixels, facing its way.
    fn attach_point(&self, o: ObjectRef, n: u8) -> (i32, i32);
    /// `object_setCoordinatesFromPanels`: x and y from the panel.
    fn set_coordinates_from_panel(&mut self, o: ObjectRef);
    /// `object_setPanelsFromCoordinates`.
    fn set_panel_from_coordinates(&mut self, o: ObjectRef);
    /// `object_updateCollisionPanels`.
    fn update_collision_panels(&mut self, o: ObjectRef);
    /// Onto the destination panel of a move: the panel, the reservation,
    /// the coordinates and the collision.
    fn snap_to_future_panel(&mut self, o: ObjectRef);
    /// The object's content state (None for kinds the engine implements).
    fn state(&self, o: ObjectRef) -> Option<&ContentState>;
    fn state_mut(&mut self, o: ObjectRef) -> Option<&mut ContentState>;
    /// `SpawnT4BattleObjectWithId0`: the one-shot effect `id`.
    fn spawn_effect(&mut self, pos: Vec3, id: u8, flip: u8, palette_add: u8, priority: u8) -> Option<ObjectRef>;
    /// `object_spawnCollisionRegion`: a one-tick hit region spawned by
    /// `owner`.
    fn spawn_hitbox(&mut self, owner: ObjectRef, spec: &HitboxSpec) -> Option<ObjectRef>;
    /// `sub_80E08C4`: hit spark `id` at `pos`.
    fn spawn_spark(&mut self, owner: ObjectRef, pos: Vec3, id: u8) -> Option<ObjectRef>;
    /// `sub_8011044`: what an object with a navi's NameID takes down when
    /// it goes (for most, the overlay in its second related slot).
    fn death_hook(&mut self, o: ObjectRef, name_id: u16);

    // ---- Navis and the attack in progress -------------------------------------

    fn actor_get(&self, o: ObjectRef, f: ActorField) -> ApiResult<Value>;
    fn actor_set(&mut self, o: ObjectRef, f: ActorField, v: Value) -> ApiResult<()>;
    /// The attack's parameter `n` (0..4: the chip's params).
    fn attack_param(&self, o: ObjectRef, n: usize) -> ApiResult<u8>;
    fn set_attack_param(&mut self, o: ObjectRef, n: usize, v: u8) -> ApiResult<()>;
    fn request(&self, o: ObjectRef, f: RequestFlag) -> ApiResult<bool>;
    fn set_request(&mut self, o: ObjectRef, f: RequestFlag, on: bool) -> ApiResult<()>;
    fn navi_state(&self, o: ObjectRef, f: NaviState) -> ApiResult<bool>;
    fn set_navi_state(&mut self, o: ObjectRef, f: NaviState, on: bool) -> ApiResult<()>;
    /// Whether `key` is in the navi's button record `pad`.
    fn key(&self, o: ObjectRef, pad: Pad, key: Key) -> ApiResult<bool>;
    /// The running content action's state, zeroed when a different action
    /// last used it.
    fn action_state_mut(&mut self, o: ObjectRef) -> ApiResult<&mut ContentState>;
    fn status(&self, o: ObjectRef, flag: StatusFlag) -> ApiResult<bool>;
    fn set_status(&mut self, o: ObjectRef, flag: StatusFlag, on: bool) -> ApiResult<()>;
    fn status_timer(&self, o: ObjectRef, t: StatusTimer) -> ApiResult<u16>;
    fn set_status_timer(&mut self, o: ObjectRef, t: StatusTimer, v: u16) -> ApiResult<()>;
    /// `object_setDefaultCounterTime`: open the attack's counter window.
    fn open_counter_window(&mut self, o: ObjectRef);
    /// `sub_801056A`: the reactive-defense abort attacks check after each
    /// phase.
    fn check_reactive_abort(&mut self, o: ObjectRef);
    /// `object_exitAttackState`: back to the idle action with animation 0.
    fn exit_attack(&mut self, o: ObjectRef);
    /// `sub_801171C`: back to the idle action (the animation untouched).
    fn end_attack(&mut self, o: ObjectRef);
    /// `object_setAttack0..5`: start `action`; `kind` records which
    /// (1 buster, 2 chip or charged shot, 3 special, 4 move...).
    fn set_attack(&mut self, o: ObjectRef, action: u8, kind: u8);
    /// `sub_801011A`: clear the attack's link bytes and unfreeze the
    /// lock-on marker.
    fn reset_attack_links(&mut self, o: ObjectRef);
    /// `sub_800FA54`: the held direction (1 up, 2 down, 3 back, 4
    /// forward; 0 none).
    fn held_direction(&self, o: ObjectRef) -> u8;
    /// `sub_800F964`: where a step in `dir` would go, if it can.
    fn step_target(&self, o: ObjectRef, dir: u8) -> Option<PanelPos>;
    /// `sub_80116AE`: start a step toward `dir` from input.
    fn start_move(&mut self, o: ObjectRef, dir: u8);
    /// `object_canMove`: not immobilized, sliding or moving.
    fn can_move(&self, o: ObjectRef) -> bool;
    /// `sub_801265A`: the buster's damage (the attack level, with the
    /// navi's and form's bonus, at most 10; 1 when worn out).
    fn buster_damage(&self, o: ObjectRef) -> u16;
    /// Obstacles the navi absorbed, oldest first: (kind, animation).
    fn absorbed(&self, o: ObjectRef) -> ApiResult<Vec<(u8, u8)>>;
    /// Add one (false when the navi has eight).
    fn push_absorbed(&mut self, o: ObjectRef, kind: u8, anim: u8) -> ApiResult<bool>;
    /// Take the newest.
    fn pop_absorbed(&mut self, o: ObjectRef) -> ApiResult<Option<(u8, u8)>>;

    // ---- Sprites -------------------------------------------------------------

    /// `sprite_load`: load a sprite (its animation and look reset) and let
    /// it animate (`no_sprite_update` off).
    fn sprite_load(&mut self, o: ObjectRef, id: SpriteId);
    /// Start animation `anim` from its first frame.
    fn sprite_set_animation(&mut self, o: ObjectRef, anim: u8);
    /// Advance the animation one tick (no gating).
    fn sprite_step(&mut self, o: ObjectRef);
    fn sprite_get(&self, o: ObjectRef, f: SpriteField) -> Value;
    fn sprite_set(&mut self, o: ObjectRef, f: SpriteField, v: Value) -> ApiResult<()>;

    // ---- Collision -------------------------------------------------------------

    /// Give the object a collision slot; false if none is free.
    fn create_collision(&mut self, o: ObjectRef) -> bool;
    /// Set up the registration from the object: its side, panel, element
    /// and damage, and what it is and reacts to (collision type indices).
    fn setup_collision(&mut self, o: ObjectRef, self_type: u8, target_type: u8, hit_mod: u8);
    /// `sub_801A082`: redo the types (and damage) of a registration.
    fn reset_collision_types(&mut self, o: ObjectRef, self_type: u8, target_type: u8, hit_mod: u8);
    fn collision_get(&self, o: ObjectRef, f: CollisionField) -> ApiResult<Value>;
    fn collision_set(&mut self, o: ObjectRef, f: CollisionField, v: Value) -> ApiResult<()>;
    /// Register on the region's panels (clearing the last results).
    fn present_collision(&mut self, o: ObjectRef);
    /// Unregister, resolving hits against whatever is registered there.
    fn remove_collision(&mut self, o: ObjectRef);
    fn free_collision(&mut self, o: ObjectRef);
    /// `object_spawnCollisionEffect`: the hit spark of a registration that
    /// just hit something (one RNG draw when it shows).
    fn hit_spark(&mut self, o: ObjectRef);

    // ---- Services ------------------------------------------------------------

    /// Run a step of the dimming service for the controller `o`; `chip`
    /// is the chip the controller shows (AntiNavi's steps read it).
    fn dimming(&mut self, o: ObjectRef, step: DimmingStep, chip: u16);
    /// `sub_80E1352`: a navi chip's user vanishes while its navi acts.
    fn hide_user(&mut self, user: ObjectRef);
    /// `sub_80E13DC`: and comes back.
    fn show_user(&mut self, user: ObjectRef);
    /// A navi chip's navi is done: its controller moves on.
    fn navi_chip_left(&mut self, controller: ObjectRef);
}
