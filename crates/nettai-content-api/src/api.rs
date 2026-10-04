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

use crate::registry::{ChipHandle, Registry};
use crate::state::{ContentState, FieldType, StateId, TypeError, Value};
use crate::types::{ObjectRef, PanelPos, SpriteId, Vec3};

/// What a navi runs (`CoreApi::navi_action`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NaviAction {
    /// A content action, by handle.
    Content(u16),
    /// One of the ruleset's own states or actions: `"entry"`,
    /// `"take_control"`, `"deletion"`, `"flinch"`, `"paralysis"`, `"drag"`,
    /// `"freeze"`, `"bubble"`, `"idle"`, `"move"`, `"dimming_chip"`,
    /// `"navi_chip"`, `"instant_chip"`, `"form_change"`.
    Engine(&'static str),
}

/// Where a spawned object goes in the update list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnAt {
    /// Right after the object updating (it runs later this tick).
    AfterCurrent,
    /// `sub_80033E4`: at the head (it first runs next tick, before
    /// everything else).
    First,
    /// `sub_8003374` / `sub_800333C`: at the end (it runs after everything
    /// else this tick).
    End,
}

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
pub const PANEL_TYPES: [&str; 16] = [
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
    "metal",
    "lava",
    "sea",
];

/// Actor types by name (an actor record's type).
pub const ACTOR_TYPES: [&str; 3] = ["virus", "navi", "player"];
/// A navi's barrier as content sees it (CollisionData+0x06): none, up (any
/// kind, its HP whatever it is), or popped (0x10: blown away by wind).
pub const BARRIER_STATES: [&str; 3] = ["none", "up", "popped"];

/// The custom gauge's speeds (NaviStats+0x08), by their byte.
pub const GAUGE_SPEEDS: [&str; 3] = ["normal", "fast", "slow"];

/// How a barrier behaves once raised (`sub_801A802` by the barrier byte):
/// worn down and timed out (`plain`: Barrier, Barr100, Barr200, LifeAur,
/// the auras), back with 1 HP 240 ticks after it is worn down and broken
/// by elec (`bubble`: BblWrap), or regenerating up to 200 HP (`regenerating`).
pub const BARRIER_BEHAVIORS: [&str; 3] = ["plain", "bubble", "regenerating"];

/// `sub_801A7CC`'s row: what a barrier sets in the collision data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BarrierSpec {
    /// Index into [`BARRIER_BEHAVIORS`].
    pub behavior: u8,
    /// Its HP (+0x16), the damage a hit must reach to wear it (+0x17), its
    /// timer (+0x1A; 0xFFFF: none), and the element that breaks it (+0x14;
    /// 0: none).
    pub hp: u8,
    pub threshold: u8,
    pub timer: u16,
    pub weak_element: u8,
}

/// A navi's drag reaction steps by name (BattleObject+0x0D: the game's 0,
/// 4, 8).
pub const DRAG_STEPS: [&str; 3] = ["start", "slide", "recover"];

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
        /// What it is taken for (none: a virus).
        Identity = "identity", Ref(Registry::Identity, None), rw;
        /// Holds an object's sprite (and more, by kind: `PreventAnim`).
        PreventAnim = "prevent_anim", U8, rw;
        /// BattleObject +0x1A: a player's chips left in its hand as the HUD
        /// shows them; another actor's own count (BN5's Jealousy reads it).
        ChipsHeld = "chips_held", U8, ro;
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
        /// A navi's drag reaction step (BattleObject+0x0D).
        DragStep = "drag_step", enum_type(&DRAG_STEPS), rw;
        /// It holds a panel reservation (released when it is destroyed).
        HoldsReservation = "holds_reservation", Bool, rw;
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
        /// The chip being used, as its definition (none for weapons): what
        /// a weapon that uses the chip asks for the chip's own parts, and
        /// what an action that becomes another chip's sets.
        AttackChip = "attack_chip", Ref(Registry::Chip, None), rw;
        /// The attack's element byte (primary | secondary bits).
        AttackElement = "attack_element", U8, rw;
        /// The attack's damage (with its flag bits).
        AttackDamage = "attack_damage", U16, rw;
        /// The attack's hit parameter (its hitbox's counter byte).
        HitParam = "hit_param", U16, rw;
        /// The attack's variant (AIAttackVars+3), as the anti-damage
        /// counters read it.
        AttackVariant = "attack_variant", U8, rw;
        Charged = "charged", U8, rw;
        /// Lockout the attack applies when it ends (kind 2: the chip
        /// lockout; kind 3: the B+Back cooldown).
        AttackLockout = "attack_lockout", U8, rw;
        /// The Atk+ / cross bonus.
        Extra = "extra", U16, rw;
        SpecialSource = "special_source", U8, rw;
        /// Which `set_attack` kind started the action (0..5).
        AttackKind = "attack_kind", U8, ro;
        /// 1 while the action runs inside the side's wrapper (`sub_801B9E6`
        /// runs the role `actions.wrapper` instead: BN6's Beast Out rush).
        Wrapped = "wrapped", U8, rw;
        /// The wrapper's state starts over (`sub_801011A` clears it with the
        /// attack's links); the wrapper clears it once it has.
        WrapperFresh = "wrapper_fresh", Bool, rw;
        /// The controller's state starts over (a form's `berserk` effect,
        /// `sub_802D310`); the controller clears it once it has.
        ControllerFresh = "controller_fresh", Bool, rw;
        /// Its mood is held (`sub_8015BEC`): held tired or exhausted. What
        /// another side's rules read (BN6's counter, `sub_801A200`).
        MoodHeld = "mood_held", Bool, ro;
        /// The side's systems' `navi_tick` runs for it each tick.
        Ticked = "ticked", Bool, rw;
        /// Exhausted for the rest of the battle (BN6's after Beast Over):
        /// worn out, the mood can't change, 1 HP lost per tick (never the
        /// last), no Full Synchro or anger.
        Exhausted = "exhausted", Bool, rw;
        /// +0x2C: an object a step or the wrapper turns to face in the panel
        /// patterns 0x23, 0x31 and 0x33 (only the unused `sub_80116F6` sets
        /// one).
        FaceTarget = "face_target", Object, ro;
        /// The lock-on mode the attack's own action asks the Beast Out rush
        /// for (the charged sword's, by its slash); none: the chip's.
        RushLockon = "rush_lockon", Ref(Registry::Lockon, None), rw;
        /// A per-action word some actions keep (a move's direction change,
        /// a thrown obstacle).
        Marker = "marker", U32, rw;
        /// The absorbed obstacle a throw carries (the marker word, as
        /// DustCross's throws keep it): its look (an absorbed-look record)
        /// and animation.
        ThrownLook = "thrown_look", Ref(Registry::Record, Some("absorbed-look".into())), rw;
        ThrownAnim = "thrown_anim", U8, rw;
        /// The attack's count (AIAttackVars+0x12): what an action counts
        /// (its shots, swings, slashes, a hold's ticks, the recovery after
        /// a shot), a word every action shares and none clears, so an
        /// action that reads it before writing it reads what an earlier one
        /// left.
        AttackCount = "attack_count", U16, rw;
        ActorType = "actor_type", enum_type(&ACTOR_TYPES), ro;
        /// Form or AI variant.
        AiIndex = "ai_index", U8, ro;
        /// The target marker (BN6's Beast Out lock-on marker).
        TargetMarker = "target_marker", Object, rw;
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
        /// The weapons the navi's buttons run (none: no weapon; the
        /// buster's and charged shot's writable: chips change them).
        BusterWeapon = "buster_weapon", Ref(Registry::Weapon, None), rw;
        ChargeShotWeapon = "charge_shot_weapon", Ref(Registry::Weapon, None), rw;
        BackSpecialWeapon = "back_special_weapon", Ref(Registry::Weapon, None), ro;
        /// AIData+0x32: the Beast Out counter is spent (BugFix sets it by
        /// the navi's counter, `sub_8014446` / `sub_801443C`).
        Tired = "tired", Bool, rw;
        /// AIData+0x60: the barrier's visual (effect #7).
        BarrierVisual = "barrier_visual", Object, rw;
        /// BN5's AIData+0x3C: DarkPlus's tint (0 none), which picks the
        /// navi's status shader.
        PlusTint = "plus_tint", U16, rw;
        /// BN5's Chaos Unison charge: armed (AIData+0x12; the chaos
        /// change arms it, a weapons' load and the failure's end disarm
        /// it) and its level (AIData+0x6C: releases that succeeded, at
        /// most 4).
        ChaosArmed = "chaos_armed", Bool, rw;
        ChaosLevel = "chaos_level", U8, rw;
        /// BN5's no-charge drive (DarkInvs): its ticks left (AIData+0x36;
        /// counted down in the intake while the navi has the no-charge
        /// state, asking for the stun strike at 0; 0xFFFF holds), and the
        /// computer-navi AI driving it (AIData+0xF0, which VarSwrd's pick
        /// reads).
        NoChargeTimer = "no_charge_timer", U16, rw;
        ComputerDriven = "computer_driven", Bool, rw;
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
        /// The region its hits cover (a region definition), or none.
        Region = "region", Ref(Registry::Region, None), rw;
        /// The panel the region is anchored on (the game's CollisionData
        /// PanelX/PanelY; `update_collision_panels` copies the object's).
        PanelX = "panel_x", U8, rw;
        PanelY = "panel_y", U8, rw;
        /// The spark its hits show (a spark definition), or none.
        HitEffect = "hit_effect", Ref(Registry::Spark, None), rw;
        /// The primary element its hits carry (CollisionData+0x02; setup
        /// takes the object's element's low nibble), and the secondary
        /// elements (+0x19: its high nibble), which `sub_8019F8C` sets
        /// again from an element byte.
        Element = "element", U8, rw;
        SecondaryElement = "secondary_element", U8, rw;
        /// The status its hits carry (a status definition), or none.
        StatusBase = "status_base", Ref(Registry::Status, None), rw;
        /// Bug code (low byte) and argument (high byte).
        Bugs = "bugs", U16, rw;
        /// The bug code and argument the last resolution's hits brought
        /// (CollisionData+0x9C, +0x9D), which the navi takes.
        InflictedBugs = "inflicted_bugs", U16, rw;
        HitModBase = "hit_mod_base", U8, rw;
        /// The damage it deals (the object's damage at setup).
        SelfDamage = "self_damage", U16, rw;
        /// The counter byte (CollisionData+0x07: bits 0-6 counter
        /// strength, bit 7 can't counter), which setup takes from the
        /// damage word's high half.
        CounterByte = "counter_byte", U8, rw;
        /// The ticks left of its counter window (CollisionData+0x0D): a
        /// hit with a counter byte counters it while they run.
        CounterTimer = "counter_timer", U8, ro;
        /// What the last resolution hit (writable: some kinds clear it).
        HitFlags = "hit_flags", U32, rw;
        /// The damage taken this window.
        FinalDamage = "final_damage", U16, ro;
        /// Which way the region's anchor last moved (0 none, 1 up, 2 down,
        /// 3 back, 4 forward, 5 other).
        Direction = "direction", U8, ro;
        /// The directions (1 << the hitter's flip) a guard blocked hits
        /// from this window (CollisionData+0x03).
        GuardDirs = "guard_dirs", U8, ro;
        /// The secondary elements (sword 0x80, cursor 0x40, wind 0x20,
        /// break 0x10) of what hit it this window.
        DamageElements = "damage_elements", U8, ro;
        /// The hit modifiers of what hit it this window (+0x0E,
        /// HitModifierFinal): 0x3C the pushes, 0x40 a pushing hit.
        HitModFinal = "hit_mod_final", U8, ro;
        /// Its barrier ([`BARRIER_STATES`]), the barrier's HP byte, and the
        /// hit modifier that popped it (+0x15).
        Barrier = "barrier", enum_type(&BARRIER_STATES), ro;
        BarrierHp = "barrier_hp", U8, ro;
        BarrierPopHitMod = "barrier_pop_hit_mod", U8, ro;
    }
}

named_fields! {
    /// A side's navi stats that content reads (and the few it changes).
    pub enum NaviStat {
        /// Fighting in the sun.
        Sun = "sun", Bool, ro;
        /// The form and the navi, as their definitions.
        /// Writable: a game's form change puts the navi in its form.
        Form = "form", Ref(Registry::Form, None), rw;
        Navi = "navi", Ref(Registry::Navi, None), ro;
        NaviVariant = "navi_variant", U8, ro;
        /// The base form's element (patch cards write it).
        Element = "element", U8, rw;
        /// The buster's levels (writable: chips raise them).
        Attack = "attack", U8, rw;
        Rapid = "rapid", U8, rw;
        Charge = "charge", U8, rw;
        /// Writable: BN5's dark chips set it (0x080127D6).
        Mood = "mood", U8, rw;
        /// The Beast Out turns left (writable: BN6's beast system spends
        /// them).
        BeastOutCounter = "beast_out_counter", U8, rw;
        /// The form the navi started the battle in.
        StartingForm = "starting_form", Ref(Registry::Form, None), ro;
        /// The navi's game: 0 Gregar, 1 Falzar.
        Version = "version", U8, ro;
        /// The Regular chip's MB at most (+0x09, RegUp's).
        RegularMemory = "regular_memory", U8, ro;
        MaxBaseHp = "max_base_hp", U16, ro;
        /// The NaviCust's heal on chip use.
        ChipRecovery = "chip_recovery", U16, rw;
        /// NaviCust weapon stats: the buster shot's and the charged
        /// shot's programs, each the projectile variant a shot is on a
        /// lucky draw (none: no program).
        BusterShot = "buster_shot", Ref(Registry::Record, Some("projectile-variant".into())), rw;
        ChargeShotKind = "charge_shot_kind", Ref(Registry::Record, Some("projectile-variant".into())), rw;
        /// The damage a B+Back special takes from the navi's stats
        /// (NaviStats+0x48).
        BackSpecialDamage = "back_special_damage", U16, ro;
        /// NaviCust bugs: buster blanks and buster charged shots (of 16).
        BusterBlanks = "buster_blanks", U8, rw;
        BusterCharged = "buster_charged", U8, rw;
        /// NaviCust bugs: the HP drain in the fight and while the custom
        /// screen is open (levels), and what a step leaves behind.
        HpDrain = "hp_drain", U8, rw;
        CustomDrain = "custom_drain", U8, rw;
        PanelTrail = "panel_trail", U8, rw;
        // Written by the navi-changing dimming chips (off_802CCB4[38]).
        /// The custom screen's size.
        CustomLevel = "custom_level", U8, rw;
        /// NaviCust bug: the hand shrinks from this turn on (0 none).
        HandShrinkTurn = "hand_shrink_turn", U8, rw;
        /// The weapons the navi starts rounds with: the charged shot, the
        /// B+Back special (none: no weapon).
        ChargeShotWeapon = "charge_shot_weapon", Ref(Registry::Weapon, None), rw;
        BackSpecialWeapon = "back_special_weapon", Ref(Registry::Weapon, None), rw;
        /// NaviCust: FloatShoes, AirShoes, UnderShirt.
        FloatShoes = "float_shoes", Bool, rw;
        AirShoes = "air_shoes", Bool, rw;
        Undershirt = "undershirt", Bool, rw;
        /// BN5's Hub Style (+0x4C), which its patch card 111 sets: 1 Team
        /// ProtoMan's, 2 Team Colonel's; 0 none.
        HubStyle = "hub_style", U8, rw;
        // Written by the patch cards at the round's start (rules/
        // patch-cards.luau), with the writable ones above.
        /// HP when the round starts, and its maximum (+0x40, +0x42).
        Hp = "hp", U16, rw;
        MaxHp = "max_hp", U16, rw;
        /// The folder's Mega and Giga chip limits (+0x0B, +0x0C).
        MegaLevel = "mega_level", U8, rw;
        GigaLevel = "giga_level", U8, rw;
        /// NaviCust SuperArmor and StatusGuard (+0x23, +0x52).
        SuperArmor = "super_armor", Bool, rw;
        StatusGuard = "status_guard", Bool, rw;
        /// The B button's weapon (+0x04; none: no weapon).
        BusterWeapon = "buster_weapon", Ref(Registry::Weapon, None), rw;
        /// The barrier the navi enters with (+0x06; none: no barrier).
        FirstBarrier = "first_barrier", Ref(Registry::Record, Some("barrier".into())), rw;
        /// The custom gauge's speed (+0x08, [`GAUGE_SPEEDS`]).
        Gauge = "gauge", enum_type(&GAUGE_SPEEDS), rw;
        /// The supports (+0x0D's bits). With the NaviCust's support bug
        /// (the byte 0xFF) there are none, and setting one leaves it so.
        Rush = "rush", Bool, rw;
        Beat = "beat", Bool, rw;
        Tango = "tango", Bool, rw;
        /// The support bug (+0x0D 0xFF): no supports, and none can be set.
        /// Writable: a game's NaviCust rules bring it.
        SupportBug = "support_bug", Bool, rw;
        /// NaviCust bugs: steps go astray (+0x31, 1), a step's panel trail
        /// level (+0x13), the status a hit gives (+0x16), the custom
        /// screen's damage (+0x54), the emotion swings (+0x24), the battle
        /// start's (+0x1A).
        StepBug = "step_bug", U8, rw;
        PanelTrailLevel = "panel_trail_level", U8, rw;
        HitStatus = "hit_status", U8, rw;
        CustomDamage = "custom_damage", U16, rw;
        EmotionBug = "emotion_bug", U8, rw;
        BattleStartBug = "battle_start_bug", U8, rw;
        /// NaviCust ChpShufl and NumbrOpn (+0x60, +0x61).
        ChipShuffle = "chip_shuffle", Bool, rw;
        NumberOpen = "number_open", Bool, rw;
        /// What the NaviCust does to chip drops (+0x26: 1 its collector
        /// bug, bit 2 Collect) and random encounters (+0x28: 1 its
        /// encounter bug): no netbattle reads them; the patch cards' bug
        /// count does. Writable: a game's NaviCust rules set them.
        ChipDrops = "chip_drops", U8, rw;
        Encounters = "encounters", U8, rw;
        /// `sub_800FE52`: how many kinds of NaviCust bug the navi has
        /// (astray steps, a panel trail, buster blanks, a hit status,
        /// custom-screen damage, emotion swings, the two HP drains, a
        /// battle-start hook, a shrinking hand).
        BugKinds = "bug_kinds", U8, ro;
        /// BN5's NaviStats +0x32: the turns a soul lasts longer (SoulT+1's
        /// 1), which the custom screen adds to a soul's three (signed).
        /// Writable: BN5's NaviCust rules set it.
        SoulTurnBonus = "soul_turn_bonus", I8, rw;
    }
}

named_fields! {
    /// Battle-wide state content reads.
    pub enum BattleInfo {
        /// A link (net) battle.
        Link = "link", Bool, ro;
        /// A battle against a ranked boss (battle effect 1: LifeSync does
        /// nothing in one, `sub_80E72C8`).
        BossRank = "boss_rank", Bool, ro;
        /// A battle whose dark chips fizzle (battle effect 0x100000: BN5's
        /// dark chip rule, 0x0801003C).
        NoDarkChips = "no_dark_chips", Bool, ro;
        /// A battle that holds BN5's light/dark value at 500 (battle effect
        /// 0x20000, 0x08010EDC).
        LightDarkHeld = "light_dark_held", Bool, ro;
        Mode = "mode", U8, ro;
        PanelPattern = "panel_pattern", U8, ro;
        /// Every navi is in (the intro's bit 2).
        NavisIn = "navis_in", Bool, ro;
        /// Presentation only: the side the simulation's perspective is.
        LocalSide = "local_side", U8, ro;
        /// Custom screens opened so far.
        Turn = "turn", U8, ro;
        /// Battle flag 0x40: each player has a custom gauge (`sub_800A8F8`;
        /// not in netbattles).
        PerPlayerGauges = "per_player_gauges", Bool, ro;
        /// Battle flag 1: the fight is on (collision is live).
        Fighting = "fighting", Bool, ro;
        /// Battle flag 2: the custom gauge is full.
        GaugeFull = "gauge_full", Bool, ro;
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
        // Dimming chip subtype 20 (ElemTrap's trap).
        /// Hit even while the battle is dimmed.
        HitWhileDimmed = "hit_while_dimmed",
        // VDoll's doll.
        /// A field object being carried to be thrown (0x04000000).
        Carried = "carried",
        /// Untouchable (0x08000000): no hit reaches it and it hits nothing
        /// (Falzar Beast Over's form, BN5's DarkInvs drive).
        Untouchable = "untouchable",
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
        NaviSwitch = "navi_switch",
        SwitchKnockout = "switch_knockout",
        Mode9A = "mode9_a",
        /// A system's takeover of the side's navi (BN6's Cross special).
        Takeover = "takeover",
        Volley = "volley",
        WeaknessHit = "weakness_hit",
        /// The slide request (the collision's flag2 0x10, not an action
        /// request).
        Slide = "slide",
        /// The anger request (the collision's flag2 0x200): the navi gets
        /// angry at its next status update, unless its mood is held
        /// (`sub_8014326`).
        Anger = "anger",
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
        ChangingCross = "switching_navi",
        CrossKnockout = "switch_knockout",
        Crossed = "switched",
        Volley = "volley",
        Uninterruptible = "uninterruptible",
        CrossBreaking = "cross_breaking",
        FormChangeSpriteHeld = "form_change_sprite_held",
        HeatTrap = "heat_trap",
        /// Gone from the field while its navi chip's navi acts
        /// (`sub_80E1352` sets it, `sub_80E13DC` clears it).
        Vanished = "vanished",
        /// It dives: a panel that submerges (BN5's sea) submerges it and
        /// doesn't hold it at a move's end (BN5's ToadSoul, 0x08011CB2).
        /// The status reset ends it.
        Dives = "dives",
    }
}

/// A folder a tool asks a side's rules to check (`folder_check`): whose
/// side it is, its chips in order (handles and codes), its Regular and tag
/// chips (entries of `chips`), and whether it is all of a folder (else the
/// chips so far, as a random folder is drawn: the rules about a whole
/// folder wait).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedFolder {
    pub side: u8,
    pub chips: Vec<(u16, u8)>,
    pub regular: Option<u8>,
    pub tags: Option<(u8, u8)>,
    pub complete: bool,
}

/// A program on a side's NaviCust, as content reads it
/// (`battle.navicust`): the program's handle, its color (an index into the
/// definition's `colors`), its center on the 7x7 grid, its quarter turns
/// clockwise, and whether it is compressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedProgram {
    pub program: u16,
    pub color: u8,
    pub x: u8,
    pub y: u8,
    pub rotation: u8,
    pub compressed: bool,
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
        /// BN5's mood under 65.
        Worried = "worried",
    }
}

named_flags! {
    /// A HUD part a chip's effect hides and shows (`sub_801DACC` and
    /// `sub_801DA48` with its draw task).
    pub enum HudPart {
        /// The custom gauge (draw task 4).
        Gauge = "gauge",
        /// The emotion window (draw task 14).
        EmotionWindow = "emotion_window",
        /// The battle flag 0x40 mode's gauge, drawn by its levels (draw
        /// task 17).
        LevelGauge = "level_gauge",
    }
}

/// What a side's custom screen reads of its player (`custom.player`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomPlayer {
    /// The emotion the screen reads of its navi.
    pub emotion: Emotion,
    pub random_battle: bool,
}

named_flags! {
    /// A side's special in progress (battle flag 0x40 mode;
    /// `sub_802E4B8`): the SELECT special, or a system's takeover of the
    /// side's navi (BN6's Cross special).
    pub enum SideSpecial {
        None = "none",
        Select = "select",
        Takeover = "takeover",
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
        /// The Gregar and Falzar chips' controllers' first action, in
        /// place of the dim (the Japanese ROMs' 0x080EDD0C and
        /// 0x080EDED0): darken the field to black, then the next action.
        FadeToBlack = "fade_to_black",
        /// `sub_800BCF6`: back from black (to the dim, if the other side's
        /// dimming still runs), then end.
        FadeFromBlack = "fade_from_black",
        /// `object_timefreezeEnd`: the controller's state 8; ends the
        /// dimming once both sides are done, and frees the controller.
        Finish = "finish",
    }
}

/// How a form overlay's sprite steps once every navi is in (its Param3):
/// `object_updateSprite` (and not while dimmed), `object_updateSpriteTimestop`,
/// or `sub_801BCD0` (paused or not).
pub const OVERLAY_STEPPINGS: [&str; 3] = ["normal", "while_dimmed", "always"];

named_flags! {
    /// A shared entry of an obstacle's action table (the obstacle
    /// framework, `kinds/obstacle.rs`): what an obstacle kind's table runs
    /// for the actions that aren't its own.
    pub enum ObstacleAction {
        /// `sub_80165B8`: back to idle.
        ReturnToIdle = "return_to_idle",
        /// `sub_8017E26`: pushed up to six panels, not into bodies or other
        /// obstacles (and, pulled back, not into the puller's area).
        Slide = "slide",
        /// `sub_80166AE`, `sub_8016B02`, `sub_8016B36`, `sub_8016B72`: the
        /// actor hit reactions (they need actor data).
        Flinch = "flinch",
        Paralyzed = "paralyzed",
        Frozen = "frozen",
        Bubbled = "bubbled",
        // Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework:
        /// `sub_8017CC0`: knocked back as far as the hit says, onto free
        /// panels it reserves on the way.
        KnockedBack = "knocked_back",
    }
}

named_flags! {
    /// What an obstacle's `obstacle.react` does with a body's, another
    /// obstacle's or a breaking hit's touch (the dispatchers differ there).
    pub enum ObstacleCrush {
        /// `sub_801B394`: the HP drops to 0.
        Breaks = "breaks",
        /// `sub_801B4D4`: destroyed, the HP as it is.
        Destroys = "destroys",
        // Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework:
        /// `sub_801B610`: bodies don't break it; obstacles and breaking
        /// hits drop the HP to 0.
        SparesBodies = "spares_bodies",
        /// `sub_801B878` while its object's ExtraVars+4 is set (LilBoiler
        /// erupting): such a touch is as any hit.
        Ignores = "ignores",
    }
}

// Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework:
named_flags! {
    /// When `obstacle.react` holds an obstacle still while dimmed.
    pub enum ObstacleHold {
        /// `sub_801B394` and the others: once it has appeared (its
        /// appearing action runs on while dimmed).
        AfterAppearing = "after_appearing",
        /// `sub_801B750`: always.
        Always = "always",
    }
}

named_flags! {
    /// What a pushing hit (hit modifier 0x40) does in `obstacle.take_hits`.
    pub enum ObstaclePush {
        /// `sub_801AD9E`: pushes it, and its damage is forgotten.
        ForgetsDamage = "forgets_damage",
        /// `sub_801AD12`: pushes it, and the damage still lands.
        KeepsDamage = "keeps_damage",
        /// `sub_801ADFA`: as `keeps_damage`, and any hit from one side (but
        /// hits of type 0x1000) pushes it a panel away.
        AnyHit = "any_hit",
        /// `sub_801AD6A`: it is never pushed.
        Ignored = "ignored",
    }
}

named_flags! {
    /// Who placed a side's wind (`sub_80E541A`'s third argument).
    pub enum WindSource {
        /// Wind and Fan's fan (attack object #0x48).
        Obstacle = "obstacle",
        /// A navi's own wind (effect object #0x41, `sub_80E532C`), which a
        /// fan's can't replace.
        Navi = "navi",
    }
}

named_flags! {
    /// How an obstacle is leaving the field (its collision `f2` word).
    pub enum ObstacleRemoval {
        /// Broken: its HP ran out, its time is up, the battle is over.
        Broken = "broken",
        /// A chip removed it.
        Removed = "removed",
        /// A chip made it blink out.
        Vanished = "vanished",
        /// A navi absorbed it.
        Absorbed = "absorbed",
    }
}

named_flags! {
    /// What other objects ask of a field object (an obstacle: a rock, a
    /// cube, a whirlwind...), in its collision's second flags word.
    pub enum ObstacleFlag {
        /// Break it at its next reaction.
        Destroy = "destroy",
        /// A hit made it flinch.
        Flinch = "flinch",
        /// A hit pushes it.
        Pushed = "pushed",
        /// Picked up to be thrown.
        Thrown = "thrown",
        /// Encased in ice or a bubble.
        Encased = "encased",
        /// A chip removes it.
        Removed = "removed",
        /// It blinks out.
        Vanish = "vanish",
        /// A side absorbs it (either side, side 0, side 1).
        Absorbed = "absorbed",
        AbsorbedBy0 = "absorbed_by_0",
        AbsorbedBy1 = "absorbed_by_1",
    }
}

named_flags! {
    /// `sub_800F8CE`: how a removed obstacle's blink-out is going.
    pub enum BlinkOut {
        /// Not blinking out (removed some other way).
        No = "no",
        Blinking = "blinking",
        Done = "done",
    }
}

named_flags! {
    /// What a chip asks of an obstacle (`sub_800F884`, `sub_800F898`,
    /// `sub_800F8B0`).
    pub enum ObstacleRequest {
        Remove = "remove",
        Vanish = "vanish",
        /// Absorbed by the requester's side's navi.
        Absorb = "absorb",
    }
}

/// A one-tick hit region (attack object #3), as `object_spawnCollisionRegion`
/// takes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HitboxSpec {
    pub panel: PanelPos,
    pub element: u8,
    pub z: i32,
    /// The region it covers and the spark its hits show, or none.
    pub region: Option<crate::RegionHandle>,
    pub hit_effect: Option<crate::SparkHandle>,
    /// Its collision types: what it reaches, what it is.
    pub target: crate::CollisionHandle,
    pub self_type: crate::CollisionHandle,
    /// The damage word (damage | flag bits) and the counter byte.
    pub damage: u16,
    pub stamina: u16,
    pub hit_mod: u8,
    pub status: Option<crate::StatusHandle>,
    pub bug: u8,
    pub bug_arg: u8,
}

/// An afterimage (effect object #0x28, as `sub_80E33FA` spawns it): a copy
/// of a sprite that stays behind, blinking unless `steady`, for `lifetime`
/// ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AfterimageSpec {
    /// A sprite of its own, or None: a copy of the owner's (Param1 0xFF),
    /// with its NameID and form overlay.
    pub sprite: Option<SpriteId>,
    pub anim: u8,
    /// The game's flip value (bit 0 horizontal, bit 1 vertical).
    pub flip: u8,
    pub lifetime: u16,
    pub color_shader: u16,
    pub palette: u8,
    pub shadow: Shadow,
    pub steady: bool,
    /// It ends early: 1 when its side leaves the forms whose afterimages
    /// stay (BN6's Beast forms), 2 when its owner's action drops below
    /// 0x10 (0 never).
    pub tether: u8,
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

/// An entry of a player's tactics (BN5's computer-navi data) as content
/// reads it: a chip, a pattern (its place among the patterns, from 0), or
/// nothing (the halfword 0 or an empty place, 0xFFFF).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TacticEntry {
    Chip(crate::ChipHandle),
    Pattern(u8),
    /// The halfword 0 (chip 0: a block no save filled).
    Nothing,
    /// An empty place (0xFFFF).
    Empty,
}

/// A side's defensive-chip record (the linked registry).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinkedChip {
    /// The chip (a handle of the chip registry); none: no record.
    pub chip: Option<crate::ChipHandle>,
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
    /// Battle flag 4 alone (BN5's chaos failure dims the battle so with
    /// none of a dimming's machinery: 0x080EE66A, 0x080EE76A).
    fn set_dimmed(&mut self, on: bool);
    /// BN5's 0x08006AAE: the navi `identity` (actor type navi, with a
    /// `body`) comes onto `panel` for `side`, brought by `summoner`, driven
    /// by `system` (its `controller` and `navi_state`; of the summoner's
    /// side's ruleset). None when a pool or the side's list of alive actors
    /// is full.
    fn spawn_navi(
        &mut self,
        identity: crate::IdentityHandle,
        panel: PanelPos,
        side: u8,
        summoner: Option<ObjectRef>,
        system: crate::SystemHandle,
    ) -> ApiResult<Option<ObjectRef>>;
    fn is_paused(&self) -> bool;
    /// `battle_isBattleOver`: a side has no navi left, or time is up.
    fn is_battle_over(&self) -> bool;
    /// `battle_isBattleOver` as the routines that read its Z flag see it:
    /// over only once time is up (a KO reads as not over).
    fn is_time_up(&self) -> bool;
    /// `sub_800ED90` and the chip record's flags (bit 1): the chip `user`
    /// would use next deals damage (a player's at its hand's cursor, whose
    /// empty hand reads `Rules::empty_hand`; another object's own chip).
    fn next_chip_damages(&self, user: ObjectRef) -> bool;
    fn battle_info(&self, f: BattleInfo) -> Value;
    /// Report a sound effect both players hear (output only; nothing in
    /// the simulation reads it).
    fn play_sound(&mut self, sound: u16);
    /// Report a sound only `side`'s player hears.
    fn play_sound_for(&mut self, side: u8, sound: u16);
    /// `sub_800AE90`: a warning marker on the HUD this tick (output only),
    /// over the custom gauge or over the place `at` on the field, with
    /// `sound` on every 16th frame of a console's own frame counter; on
    /// `side`'s console only, or on both.
    fn warn(&mut self, sound: u16, at: Option<Vec3>, side: Option<u8>);
    /// `sub_801DC7C(dx, dy)`: every console's HUD numbers `o`'s HP under
    /// it (output only), `dx`, `dy` pixels from where it projects `o`'s
    /// position; `damage`: the damage `o` took instead (its max HP less its
    /// HP), uncentered.
    fn show_hp(&mut self, o: ObjectRef, dx: i8, dy: i8, damage: bool);
    /// `sub_801DD34`: `o`'s HP number goes.
    fn hide_hp(&mut self, o: ObjectRef);
    /// `camera_initShakeEffect_80302a8`: both consoles' cameras shake for
    /// `ticks` ticks at `magnitude` (0-3). Each shaking tick draws from
    /// the consoles' own RNGs, which ChpShufl's re-deal reads.
    fn shake_camera(&mut self, magnitude: u16, ticks: u16);
    /// `sub_80302B6`: the cameras' second shake, `magnitude` (0-3) for
    /// `ticks`.
    fn shake_camera_secondary(&mut self, magnitude: u16, ticks: u16);
    /// `battle_setFlags(0x20)` / `battle_clearFlags(0x20)`: the cameras
    /// shake even while the battle is paused (BN5's TomahawkSoul's change,
    /// 0x08012138; the change's end clears it, 0x080121B6).
    fn set_shake_through_pause(&mut self, on: bool);
    /// `sub_80EA438`: a burst around `navi`'s panel (effect object #0x90,
    /// which runs while paused).
    fn spawn_burst(&mut self, navi: ObjectRef) -> Option<ObjectRef>;
    /// `sub_801DA48` (`shown`) or `sub_801DACC` with a HUD part's draw
    /// task: every console shows or hides it (output only).
    fn show_hud(&mut self, part: HudPart, shown: bool);
    fn navi_stat(&self, side: u8, stat: NaviStat) -> Value;
    /// Change one of a side's navi stats (the writable ones).
    fn set_navi_stat(&mut self, side: u8, stat: NaviStat, v: Value) -> ApiResult<()>;
    /// A side's emotion (`sub_8015B54`).
    fn emotion(&self, side: u8) -> Emotion;
    /// Set a side's mood, unless its navi's emotion is held (`sub_8015BEC`).
    fn set_mood(&mut self, side: u8, mood: u8);
    /// Raise a side's mood by `n`, capped at 254; a mood of 0 or 0xFF
    /// stays (BN5's 0x08012802: its recovery chips, docs/design/bn5-map.md
    /// §15.3 item 6).
    fn gain_mood(&mut self, side: u8, n: u16);
    /// Lower a side's mood by `n`, to 1 at least; a mood of 0 stays
    /// (`sub_8015C12`, BN5's 0x08012820: the hits' loss, BN5's navi chips
    /// leaving).
    fn lose_mood(&mut self, side: u8, n: u16);
    /// Whether `side`'s console starts its emotion window glitching (the
    /// save's NaviCust bug flag; a ruleset's system decides it as the round
    /// is set up, as BN6's patch cards do).
    fn set_emotion_window_glitch(&mut self, side: u8, on: bool);
    /// A side's installed patch cards in their list's order (handles), and
    /// whether each is switched on (the setup's: `PlayerSetup::patch_cards`).
    fn patch_cards(&self, side: u8) -> Vec<(u16, bool)>;
    /// A side's NaviCust (the setup's: `PlayerSetup::navicust`): its
    /// board's expansions and its programs in its list's order; none when
    /// the setup gives none (its stats are already the NaviCust's).
    fn navicust(&self, side: u8) -> Option<(u8, Vec<PlacedProgram>)>;
    /// The folder a tool asks the side's rules to check (`folder_check`),
    /// while it is checked.
    fn checked_folder(&self) -> Option<CheckedFolder>;
    /// A rule the checked folder breaks: the rule's name (`copies`, `mega`,
    /// ... the game's own) and what to say.
    fn folder_problem(&mut self, rule: &str, text: &str);
    /// A side's bug frags in the battle (`sub_800F4A8`).
    fn bug_frags(&self, side: u8) -> u32;
    /// `sub_800F4B2`: a side spends `n` bug frags (the count wraps below
    /// 0, as the original's does: its callers check first).
    fn spend_bug_frags(&mut self, side: u8, n: u32);
    /// `sub_802E4B8`: the side's SELECT special or takeover in progress.
    fn side_special(&self, side: u8) -> SideSpecial;
    /// A system's takeover of side `side`'s navi starts, for `ticks`
    /// (counted down in the navi's stage B, `sub_802E1D8`), or ends: while
    /// it runs, idle asks the side's systems' `takeover` (BN6's Cross
    /// special: SideState +0x54 and +0x30).
    fn take_over(&mut self, side: u8, ticks: u16);
    /// The side's custom screen, in a system's custom hook (its button's
    /// functions; docs/design/rules-in-luau.md §4.4): the refusal sound;
    /// the shared machinery for the button under the cursor (the picked
    /// chips scrapped: BN6's DustCross; the chips not picked dealt again:
    /// BN6's ChpShufl); whether the last pick is a chip; the state of the
    /// button under the cursor ("selectable", "unavailable", "selected").
    fn custom_refuse(&mut self, side: u8) -> ApiResult<()>;
    /// `custom.folder(side)`: the side's battle folder as its screen has
    /// it, its 30 places in order (a used chip's place empty until the
    /// screen deals); `custom.swap_folder(side, a, b)`: two of its places
    /// swapped (0-based here); `custom.hand_size(side)`: how many chips the
    /// screen deals (as it deals, the framework's).
    fn custom_folder(&self, side: u8) -> ApiResult<Vec<Option<crate::ChipHandle>>>;
    fn custom_swap_folder(&mut self, side: u8, a: u8, b: u8) -> ApiResult<()>;
    fn custom_hand_size(&self, side: u8) -> ApiResult<u8>;
    fn custom_sacrifice(&mut self, side: u8) -> ApiResult<()>;
    fn custom_redeal(&mut self, side: u8) -> ApiResult<()>;
    fn custom_last_pick_is_chip(&self, side: u8) -> ApiResult<bool>;
    fn custom_cursor_state(&self, side: u8) -> ApiResult<&'static str>;
    /// More of the side's custom screen, for a system's buttons and windows
    /// (§4.4). `system` is the calling system's place in the side's ruleset,
    /// whose button, window or form a call names.
    fn custom_pick(&mut self, side: u8) -> ApiResult<()>;
    fn custom_play(&mut self, side: u8, sound: &str) -> ApiResult<()>;
    fn custom_set_column_icon(&mut self, side: u8, chip: Option<crate::ChipHandle>) -> ApiResult<()>;
    /// `ticks`: the ticks it has had already (BN6's Cross window, whose
    /// first tick its opening's last runs).
    fn custom_open_window(&mut self, side: u8, system: u8, window: &str, ticks: u16) -> ApiResult<()>;
    fn custom_window_tick(&self, side: u8) -> ApiResult<u16>;
    fn custom_shake(&mut self, side: u8, magnitude: u16, ticks: u16) -> ApiResult<()>;
    fn custom_frame(&self, side: u8) -> ApiResult<u32>;
    fn custom_set_frame(&mut self, side: u8, frame: u32) -> ApiResult<()>;
    fn custom_spin(&mut self, side: u8) -> ApiResult<()>;
    fn custom_fade(&mut self, side: u8, mode: &str, speed: u8) -> ApiResult<()>;
    fn custom_set_face(&mut self, side: u8, form: Option<crate::FormHandle>) -> ApiResult<()>;
    fn custom_pick_first(&mut self, side: u8, icon: Option<crate::ChipHandle>) -> ApiResult<()>;
    fn custom_set_button_state(&mut self, side: u8, system: u8, button: &str, state: &str) -> ApiResult<()>;
    fn custom_update_availability(&mut self, side: u8) -> ApiResult<()>;
    fn custom_draw_emblem(&mut self, side: u8, x: u32) -> ApiResult<()>;
    fn custom_set_form(&mut self, side: u8, system: u8, form: Option<crate::FormHandle>) -> ApiResult<()>;
    fn custom_form_taken(&self, side: u8, system: u8) -> ApiResult<bool>;
    fn custom_full(&self, side: u8) -> ApiResult<bool>;
    fn custom_button_picked(&self, side: u8, system: u8, button: &str) -> ApiResult<bool>;
    /// The screen's cursor (its slot), and moving it; whether a key (its
    /// name: "a", "b", "select", "start", "right", "left", "up", "down",
    /// "r", "l") is pressed this tick, or repeats (`Joypad::repeat`).
    fn custom_cursor(&self, side: u8) -> ApiResult<u8>;
    fn custom_set_cursor(&mut self, side: u8, slot: u8) -> ApiResult<()>;
    fn custom_pressed(&self, side: u8, key: &str) -> ApiResult<bool>;
    fn custom_repeated(&self, side: u8, key: &str) -> ApiResult<bool>;
    /// The screen's drawing, for a system's window: the window's sprites
    /// (`sub_80279C8`: the emblem, the Regular chip's frame and the last
    /// turns' block), the Regular chip's frame alone (`sub_802899C`), the
    /// Cross window's cursor (`sub_80289E4`), the chip window showing the
    /// slot under the cursor (`sub_8028476`), and the window's Cross tab
    /// (`sub_8026840`).
    fn custom_draw_window(&mut self, side: u8) -> ApiResult<()>;
    fn custom_draw_regular(&mut self, side: u8) -> ApiResult<()>;
    fn custom_draw_cross_cursor(&mut self, side: u8) -> ApiResult<()>;
    fn custom_show_chip_window(&mut self, side: u8) -> ApiResult<()>;
    fn custom_set_cross_tab(&mut self, side: u8, on: bool) -> ApiResult<()>;
    /// R in a system's window: `form`'s description (`sub_8026E78`; three
    /// lines without one), back to the window when it closes.
    fn custom_describe(&mut self, side: u8, form: Option<crate::FormHandle>) -> ApiResult<()>;
    /// The systems' buttons' states asked again (`sub_8028F48`).
    fn custom_refresh_buttons(&mut self, side: u8) -> ApiResult<()>;
    /// What the screen reads of its player that isn't a system's setup: the
    /// emotion it reads, and a random battle.
    fn custom_player(&self, side: u8) -> ApiResult<CustomPlayer>;
    /// The level of the navi code side `side`'s save received (0 to 14),
    /// or none (`PlayerSetup::navi_level`): what BN6's rules read of event
    /// flag 0x163.
    fn navi_level(&self, side: u8) -> Option<u8>;
    fn end_takeover(&mut self, side: u8);
    /// Its ticks left.
    fn takeover_ticks(&self, side: u8) -> u16;
    /// A side's player navi.
    fn player(&self, side: u8) -> Option<ObjectRef>;
    /// A side's combatants still in, in slot order.
    fn alive_actors(&self, side: u8) -> Vec<ObjectRef>;
    /// `sub_802EFEE`: the actor `side` tracks in the battle flag 0x40
    /// mode (its side state's +0x44), if any.
    fn tracked(&self, side: u8) -> Option<ObjectRef>;
    /// Slot `i` (from 0, of four) of a side's list of alive actors.
    fn alive_actor_slot(&self, side: u8, i: u8) -> Option<ObjectRef>;
    /// Player `side`'s tactics (BN5's computer-navi data): how many entries
    /// they count, their entry in place `i` (from 0; past the count, an
    /// empty place), and their pattern `i` (from 0): its place from the
    /// target and its chips.
    fn tactic_count(&self, side: u8) -> usize;
    fn tactic(&self, side: u8, i: usize) -> TacticEntry;
    fn tactic_pattern(&self, side: u8, i: usize) -> ApiResult<(i8, i8, Vec<crate::ChipHandle>)>;
    /// The first entry and the entry in place `i` (from 0) change places
    /// (0x0802C0DC's swap).
    fn swap_tactics(&mut self, side: u8, i: usize);
    /// The first entry goes last of the count, the rest move up
    /// (0x0802BF1C).
    fn turn_tactics(&mut self, side: u8) -> ApiResult<()>;
    /// The objects of content kind `kind` (a kind handle) in the update
    /// list, in update order, whatever their lifecycle state (the game's
    /// walks of the list, such as `sub_80C67A4`).
    fn objects_of_kind(&self, kind: u16) -> Vec<ObjectRef>;
    /// `GetRNG2`: one draw of the simulation's RNG.
    fn rng(&mut self) -> u32;
    /// `GetPositiveSignedRNG2`: one draw, bit 31 cleared.
    fn rng_positive(&mut self) -> u32;
    /// `GetPositiveSignedRNG1` on `side`'s console: one draw of that
    /// console's own RNG (RNG1), bit 31 cleared.
    fn console_rng_positive(&mut self, side: u8) -> u32;
    /// `AddRandomVarianceToTwoCoords`: jitter x and z by up to mask/2
    /// pixels (one draw).
    fn jitter(&mut self, mask: u32, pos: Vec3) -> Vec3;
    /// A side's hand: its cursor.
    fn hand_cursor(&self, side: u8) -> u8;
    /// `sub_800FC7C`: the cursor moves to the next chip.
    fn advance_hand(&mut self, side: u8);
    // Subtype 18 (Otenko):
    /// The turn (custom screens opened before, from 0) the chip at `i` of a
    /// side's hand was picked in.
    fn hand_turn(&self, side: u8, i: u8) -> u8;
    /// Add to the Atk+ bonus of the chip at `i` of a side's hand
    /// (wrapping).
    fn add_hand_attack_bonus(&mut self, side: u8, i: u8, n: u16);
    /// BN5's DarkPlus (0x0800E1D6): the Atk+ bonus of a hand's chip at `i`
    /// becomes `n`.
    fn set_hand_attack_bonus(&mut self, side: u8, i: u8, n: u16);
    /// A side's hand has a chip at `i` and it does damage (its record's
    /// flag 0x02, "has_damage").
    fn hand_chip_damages(&self, side: u8, i: u8) -> bool;
    /// BN5's Jealousy (0x080E4596): the chips left in a side's hand, from
    /// its cursor to the first empty slot.
    fn hand_left(&self, side: u8) -> u8;
    /// A side's defensive-chip record.
    fn linked(&self, side: u8) -> LinkedChip;
    fn set_linked(&mut self, side: u8, rec: LinkedChip);
    /// `sub_802CEA6`: clear it, telling its object to end.
    fn clear_linked(&mut self, side: u8);
    /// FullCust: the custom gauge is full.
    fn fill_custom_gauge(&mut self);
    /// `sub_801DFD0` (BN5's CusVolt): the custom gauge drops by `n`, to 0
    /// at least.
    fn drain_custom_gauge(&mut self, n: u16);
    /// `sub_801DF8C`: the custom gauge fills `rate` a tick (full at
    /// 0x4000).
    fn set_gauge_rate(&mut self, rate: u16);
    /// A side's slow and fast gauge timers (`sub_802E070`+0x3C, +0x3A).
    fn set_gauge_speed_ticks(&mut self, side: u8, slow: u16, fast: u16);
    /// `sub_8010B78`: the damage a side's custom gauge gives (its own gauge
    /// in the battle flag 0x40 mode, else the shared one).
    fn gauge_damage(&self, side: u8) -> u16;
    /// A side's sword pick (`sub_802E070`+0x12): the swing a variable sword
    /// makes for a navi no buttons drive (BN5's computer navi draws it,
    /// 0x0802A330).
    fn sword_pick(&self, side: u8) -> u8;
    fn set_sword_pick(&mut self, side: u8, pick: u8);
    /// Presentation: whether side `side`'s emotion window shows its form's
    /// second set of faces (`variant`; BN5's Hub Style).
    fn set_face_variant(&mut self, side: u8, variant: bool);
    /// `sub_802E032`: add to a side's own custom gauge (battle flag 0x40),
    /// up to full.
    fn add_side_gauge(&mut self, side: u8, n: u16);
    /// `sub_802E04E`: take from a side's own custom gauge (battle flag
    /// 0x40), down to empty.
    fn drain_side_gauge(&mut self, side: u8, n: u16);
    /// `sub_8010488`'s special-source branch: add `n` to one of the bonuses
    /// a side stores for its special chip (`index` 0: the Atk+ bonus a
    /// damaging chip spends, +0x36; 1: the Navi+ bonus a navi chip spends,
    /// +0x38). An index past them is an error.
    fn add_special_bonus(&mut self, side: u8, index: u8, n: u16) -> ApiResult<()>;
    /// `sub_800AB46`: bump a side's statistics counter.
    fn bump_side_stat(&mut self, side: u8, index: u8, n: u8);
    /// `sub_800AB2E`: set a side's statistics counter.
    fn set_side_stat(&mut self, side: u8, index: u8, n: u8);
    /// `sub_800AB3A`: a side's statistics counter.
    fn side_stat(&self, side: u8, index: u8) -> u8;
    // Subtype 8 (Wind and Fan):
    /// `sub_80E543C`: a side's wind (BattleState+0xC0) and who placed it.
    fn wind(&self, side: u8) -> (Option<ObjectRef>, WindSource);
    /// `sub_80E541A`: `o` is `side`'s wind; the one there before is
    /// destroyed at once (`sub_80E5410`).
    fn set_wind(&mut self, o: ObjectRef, side: u8, source: WindSource);
    /// `sub_80E544C`: `o` is no side's wind.
    fn clear_wind(&mut self, o: ObjectRef);
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
    /// A side's registered field objects (the obstacles it owns: the
    /// registry's three slots at BattleState+0xA0 + side * 0xC), in slot
    /// order, empty slots left out.
    fn side_field_objects(&self, side: u8) -> Vec<ObjectRef>;
    /// A side's field-object registry slot `i` (from 0, of its three).
    fn field_object_slot(&self, side: u8, i: u8) -> Option<ObjectRef>;
    /// BattleState+0x40: the fight's ticks (capped), `sub_800A704`.
    fn battle_time(&self) -> u32;
    /// Every registered field object (the registry's eight slots at
    /// BattleState+0xA0: each side's three, then the stage's two), in slot
    /// order, empty slots left out.
    fn all_field_objects(&self) -> Vec<ObjectRef>;
    fn column_info(&self, x: u8) -> ColumnInfo;
    fn set_column_timer(&mut self, x: u8, ticks: u16);
    /// `object_setPanelAlliance`.
    fn set_panel_alliance(&mut self, p: PanelPos, side: u8);
    /// `_object_setPanelType` (an index into [`PANEL_TYPES`]).
    fn set_panel_type(&mut self, p: PanelPos, kind: u8);
    /// `object_crackPanel`: crack a solid panel, or break a cracked,
    /// unoccupied one.
    fn crack_panel(&mut self, p: PanelPos) -> bool;
    /// `object_breakPanel_dup2`: break a solid panel, or crack it while
    /// something stands on it; with `sound` in place of the panel crack's
    /// (`object_breakPanelLoud`).
    fn break_panel(&mut self, p: PanelPos, sound: Option<u16>) -> bool;
    /// `object_breakPanel`: break a solid panel nothing stands on (true);
    /// leave any other alone.
    fn break_empty_panel(&mut self, p: PanelPos) -> bool;
    /// `object_isPanelSolid`: the panel is solid (something can stand on
    /// it).
    fn panel_solid(&self, p: PanelPos) -> bool;
    /// `object_highlightPanel` (drawn only).
    fn highlight_panel(&mut self, p: PanelPos);
    /// Replace an object's whole header flag byte (in use 0x01, visible
    /// 0x02, runs while paused 0x04, sprite not animating 0x08, runs while
    /// dimmed 0x10, holds a reservation 0x20; 0x40 and 0x80 nothing reads):
    /// what a spawner's bug leaves (the boulder's, `sub_80D2430`, reads the
    /// byte from the wrong address). Everything else sets flags by name.
    fn set_header_flags(&mut self, o: ObjectRef, flags: u8);
    /// `object_reservePanel`.
    fn reserve_panel(&mut self, o: ObjectRef, p: PanelPos) -> bool;
    /// `object_removePanelReserve`.
    fn unreserve_panel(&mut self, o: ObjectRef, p: PanelPos) -> bool;
    /// `sub_801BB78`.
    fn release_reservations(&mut self, o: ObjectRef);
    /// `sub_800E618`: may `o` step onto `p`?
    fn can_step(&self, o: ObjectRef, p: PanelPos) -> bool;
    /// `sub_800A772`: its side's chips are enabled and its chip lockout
    /// is over.
    fn chips_enabled(&self, o: ObjectRef) -> ApiResult<bool>;
    /// `sub_8010332`: the ticks a step of its ends with.
    fn move_lag(&self, o: ObjectRef) -> ApiResult<u16>;
    /// A navi's links let go as it leaves (BN5's 0x081042E6): its status
    /// visuals (`sub_801A5E2`) and its chips on the HUD (`sub_801DC36`).
    fn drop_links(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// A navi no player controls leaves (BN5's 0x08104306): no HP, the
    /// damage-carry record forgets it, its reservation goes, it leaves its
    /// side's lists and is destroyed.
    fn leave(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// `sub_800E680`: could `o` stand on `p`, whichever side owns it?
    fn can_stand_any_side(&self, o: ObjectRef, p: PanelPos) -> bool;
    // Panel changes (dimming chip subtypes 2, 3, 5, 15 and 27).
    /// `object_panel_setPoison`: a solid panel turns to poison. Whether it
    /// was solid.
    fn poison_panel(&mut self, p: PanelPos) -> bool;
    /// `object_setPanelTypeBlink`: this frame the panel is drawn as type
    /// `kind` (an index into [`PANEL_TYPES`]) of side `side` (drawn only).
    fn blink_panel(&mut self, p: PanelPos, kind: u8, side: u8);
    /// `object_breakPanel_dup1`: break a solid panel, or crack it when
    /// something stands on it; true only when it broke.
    fn shatter_panel(&mut self, p: PanelPos) -> bool;

    // ---- Objects -----------------------------------------------------------

    /// Spawn an object of kind `kind` (a handle of the kind registry): it
    /// takes its pool's lowest free slot and goes into the update list
    /// where `at` says (right after the object that spawned it, it runs
    /// later this tick). Its content state starts zeroed. None if the pool
    /// is full.
    fn spawn_def(&mut self, kind: u16, pos: Vec3, at: SpawnAt) -> ApiResult<Option<ObjectRef>>;
    /// The object's kind (a handle of the kind registry).
    fn object_kind(&self, o: ObjectRef) -> Option<u16>;
    /// What navi `o` runs: a content action (by handle), or one of the
    /// ruleset's own states and actions (by name).
    fn navi_action(&self, o: ObjectRef) -> ApiResult<NaviAction>;
    /// `loc_80126EA`: chip `chip` (a handle) as the navi's attack: its
    /// record's damage, hit parameter, lockout and element, no bonus, not
    /// charged (the weapon routines that fire a chip; a use's
    /// `sub_80126E4` without its count).
    fn load_chip_attack(&mut self, o: ObjectRef, chip: u16) -> ApiResult<()>;
    /// `sub_8010740`: in a link battle, the opponent's support Rush takes
    /// `chip` from the navi `o` if the chip is one it cancels (flags2 bit
    /// 2): the support comes and is spent. Whether it did.
    fn rush_cancels(&mut self, o: ObjectRef, chip: u16) -> ApiResult<bool>;
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
    /// Switch to `action` from its first phase (the object's own action
    /// byte; a navi's action is a definition: `set_content_attack`).
    fn set_action(&mut self, o: ObjectRef, action: u8) -> ApiResult<()>;
    fn get(&self, o: ObjectRef, f: ObjectField) -> ApiResult<Value>;
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
    /// `sub_801BC64`: `update_sprite`'s gating (dimming, holds), but paused
    /// or not.
    fn update_sprite_even_paused(&mut self, o: ObjectRef);
    /// `object_updateSpritePaused`: load a newly requested animation and
    /// step the sprite, paused or not, but not while dimmed (and whatever
    /// `no_sprite_update` says).
    fn update_sprite_while_paused(&mut self, o: ObjectRef);
    /// `sub_801BC24`: load a newly requested animation (without stepping
    /// it), else step the sprite; `update_sprite`'s gating but for holds.
    fn load_or_step_sprite(&mut self, o: ObjectRef);
    /// The object's sprite attach point `n`, in pixels, facing its way.
    fn attach_point(&self, o: ObjectRef, n: u8) -> (i32, i32);
    /// `object_setCoordinatesFromPanels`: x and y from the panel.
    fn set_coordinates_from_panel(&mut self, o: ObjectRef);
    /// `object_setPanelsFromCoordinates`.
    fn set_panel_from_coordinates(&mut self, o: ObjectRef);
    /// `object_updateCollisionPanels`.
    fn update_collision_panels(&mut self, o: ObjectRef);
    /// `sub_80169BE`: show `o` unless the battle is dimmed, then hide it
    /// from a viewer whose navi is blind if it is the other side's (each
    /// console's rule, `sub_800EB6C`, decided for both viewers; the HUD
    /// flash it also drives for a player in an action is presentation).
    fn update_visibility(&mut self, o: ObjectRef);
    /// `sub_800EB6C` on each console: `o` hidden from a viewer whose navi
    /// is blind, if it is the other side's (on top of what each viewer sees
    /// of it now). Presentation: where the original clears an object's
    /// visibility for the local console, the engine decides it per viewer.
    fn hide_from_blind(&mut self, o: ObjectRef);
    /// `to` seen by whoever sees `from` (each viewer's view of it): an
    /// attachment that follows its owner's visibility.
    fn copy_visibility(&mut self, from: ObjectRef, to: ObjectRef);
    /// Onto the destination panel of a move: the panel, the reservation,
    /// the coordinates and the collision.
    fn snap_to_future_panel(&mut self, o: ObjectRef);
    /// The object's content state (None for kinds the engine implements).
    fn state(&self, o: ObjectRef) -> Option<&ContentState>;
    fn state_mut(&mut self, o: ObjectRef) -> Option<&mut ContentState>;
    /// The state of the system in place `slot` of side `side`'s ruleset
    /// (docs/design/rules-in-luau.md §5).
    fn system_state_mut(&mut self, side: u8, slot: u8) -> ApiResult<&mut ContentState>;
    /// The player setup of that system: what the player brought, read-only
    /// in battle.
    fn system_setup(&self, side: u8, slot: u8) -> ApiResult<&ContentState>;
    /// `SpawnT4BattleObjectWithId0`: the one-shot effect `look`.
    fn spawn_effect(&mut self, pos: Vec3, look: crate::EffectHandle, flip: u8, palette_add: u8, priority: u8) -> Option<ObjectRef>;
    /// `sub_801BD3C`: the one-shot effect `look` on each field panel of
    /// hit region `region` around (x, y), turned the way side `side`
    /// faces, at height `z`; a whole-field region's from the bottom right,
    /// on the ground.
    fn spawn_region_effects(&mut self, x: i32, y: i32, region: crate::RegionHandle, side: u8, look: crate::EffectHandle, z: i32);
    /// `object_spawnCollisionRegion`: a one-tick hit region spawned by
    /// `owner`.
    fn spawn_hitbox(&mut self, owner: ObjectRef, spec: &HitboxSpec) -> Option<ObjectRef>;
    /// `sub_80E08C4`: hit spark `look` at `pos`.
    fn spawn_spark(&mut self, owner: ObjectRef, pos: Vec3, look: crate::SparkHandle) -> Option<ObjectRef>;
    /// `sub_80C468C`: a form overlay (actor 0x57) on `owner`: `sprite`,
    /// following the owner's animation plus `anim_offset`; `stepping` 0
    /// normal, 1 while dimmed, 2 always (Param3); a pixel nearer when
    /// `nudged`; wearing the owner's palette when `owner_palette`.
    fn spawn_form_overlay(
        &mut self,
        owner: ObjectRef,
        sprite: SpriteId,
        stepping: u8,
        anim_offset: u8,
        nudged: bool,
        owner_palette: bool,
    ) -> Option<ObjectRef>;
    /// `sub_80E11E0`: a screen palette flash (effect object #0x0A) of
    /// `variant` (0 white or red, 1 white over two layers) for `ticks`,
    /// optionally going on while dimmed or paused.
    fn spawn_palette_flash(&mut self, variant: u8, ticks: u8, while_dimmed: bool, while_paused: bool) -> Option<ObjectRef>;
    /// `sub_80E33FA`: an afterimage of `owner`'s side at `pos` (a sprite of
    /// its own, or a copy of the owner's).
    fn spawn_afterimage(&mut self, owner: ObjectRef, pos: Vec3, spec: &AfterimageSpec) -> Option<ObjectRef>;
    /// `sub_8010DF6`: put on `o` what `identity` wears (its `parts`: the
    /// original's actor record's init hook's routine), kept in `o`'s second
    /// related slot (CircusMan's second one in its second overlay). `arg`
    /// (r2): they step even while paused.
    fn add_parts(&mut self, o: ObjectRef, identity: crate::IdentityHandle, arg: u8);
    /// `sub_8011044`: take them off (at their next update), if the
    /// identity's death hook does.
    fn remove_parts(&mut self, o: ObjectRef, identity: crate::IdentityHandle);
    /// `sub_8010DF6` with `owner`'s NameID record (`sub_800F29C`: its
    /// actor type, AI index and first byte): put on the parts that record
    /// adds, kept in `o`'s related2; with `keep_stepping`, a part that came
    /// steps even while paused and dimmed (its Param3 1, flags 0x14); with
    /// `paused_stepping`, `sub_8010DF6`'s r2 is 1 rather than the record's
    /// first byte (the dimming chips' stand-ins): the parts step even while
    /// paused.
    fn add_parts_of(&mut self, o: ObjectRef, owner: ObjectRef, keep_stepping: bool, paused_stepping: bool);
    /// `sub_8011044` with `owner`'s NameID record: take them off.
    fn remove_parts_of(&mut self, o: ObjectRef, owner: ObjectRef);

    // ---- Navis and the attack in progress -------------------------------------

    fn actor_get(&self, o: ObjectRef, f: ActorField) -> ApiResult<Value>;
    fn actor_set(&mut self, o: ObjectRef, f: ActorField, v: Value) -> ApiResult<()>;
    fn request(&self, o: ObjectRef, f: RequestFlag) -> ApiResult<bool>;
    fn set_request(&mut self, o: ObjectRef, f: RequestFlag, on: bool) -> ApiResult<()>;
    fn navi_state(&self, o: ObjectRef, f: NaviState) -> ApiResult<bool>;
    fn set_navi_state(&mut self, o: ObjectRef, f: NaviState, on: bool) -> ApiResult<()>;
    /// Whether `key` is in the navi's button record `pad`.
    fn key(&self, o: ObjectRef, pad: Pad, key: Key) -> ApiResult<bool>;
    /// The running content action's state, zeroed when an action of another
    /// layout last used it.
    fn action_state_mut(&mut self, o: ObjectRef) -> ApiResult<&mut ContentState>;
    /// The attack state as a state of layout `state` (zeroed unless an
    /// action of that layout last used it): how an action's update sees it,
    /// and how a weapon routine sets up the action it names before the
    /// action starts.
    fn attack_state_for(&mut self, o: ObjectRef, state: StateId) -> ApiResult<&mut ContentState>;
    /// The state layout of a content action (a handle of the action
    /// registry).
    fn action_schema(&self, action: u16) -> ApiResult<StateId>;
    fn status(&self, o: ObjectRef, flag: StatusFlag) -> ApiResult<bool>;
    fn set_status(&mut self, o: ObjectRef, flag: StatusFlag, on: bool) -> ApiResult<()>;
    /// Clear the whole status word (CollisionData ObjectFlags1 = 0).
    fn clear_statuses(&mut self, o: ObjectRef) -> ApiResult<()>;
    fn status_timer(&self, o: ObjectRef, t: StatusTimer) -> ApiResult<u16>;
    fn set_status_timer(&mut self, o: ObjectRef, t: StatusTimer, v: u16) -> ApiResult<()>;
    /// `object_setDefaultCounterTime`: open the attack's counter window.
    fn open_counter_window(&mut self, o: ObjectRef);
    /// `sub_801056A`: the reactive-defense abort attacks check after each
    /// phase.
    fn check_reactive_abort(&mut self, o: ObjectRef);
    /// `sub_80105F2`: a stance's own trap caught a hit (the AntiDmg
    /// program's action 0x5A): its counter starts at the next tick, AntiDmg's
    /// (0x47) or, for a sword hit, AntiSwrd's (0x48), with the stance's damage
    /// word, lockout and variant.
    fn start_stance_counter(&mut self, o: ObjectRef);
    /// `sub_8011450`: restart the navi's form overlay with it after an
    /// animation change.
    fn refresh_form_overlay(&mut self, o: ObjectRef);
    /// `object_exitAttackState`: back to the idle action with animation 0.
    fn exit_attack(&mut self, o: ObjectRef);

    // ---- What a game's rules do to a navi (docs/design/rules-in-luau.md §4.5) ----

    /// The invulnerability ends: its timer and its flag.
    fn clear_invulnerable(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// `sub_800F46C` + `sub_800F2C6`: face the default way under the
    /// standard column patterns, and the sprite with it.
    fn face_default(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// The charge drops: its counters and the hold requests.
    fn reset_charge(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// `sub_80C4C3A` on the navi's aura: the Full Synchro aura goes.
    fn end_full_synchro_aura(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// `sub_80158FA`: movement, reactions and the slower statuses end.
    fn drop_statuses(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// `sub_801A264`: a navi's statuses end: their flags, requests and
    /// timers (not the whole status word, as `clear_statuses`).
    fn end_statuses(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// The navi's overlay (`related2`) keeps stepping through pauses and
    /// dimming (`keep`: its Param3 1 and flags 0x14), or steps like any
    /// object again (its Param3 0). Nothing without an overlay.
    fn overlay_stepping(&mut self, o: ObjectRef, keep: bool);
    /// `sub_8011384(form)`: take off what `form` wore (the base form: what
    /// is there).
    fn take_off_form_overlay(&mut self, o: ObjectRef, form: crate::FormHandle);
    /// `sub_8011268(form)`: put on what `form` wears.
    fn put_on_form_overlay(&mut self, o: ObjectRef, form: crate::FormHandle);
    /// `sub_800FC9E` and a form change's load: the navi's sprite in `form`,
    /// its animation 0 from the start, shadow on the ground, facing its
    /// way, white.
    fn load_form_sprite(&mut self, o: ObjectRef, form: crate::FormHandle) -> ApiResult<()>;
    /// `sub_80144C0`: the full status reset (NaviCust state, hand bonuses,
    /// hit modifier, region, charge, weapons, the form's flags, element,
    /// body damage).
    fn reset_status(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// Anger ends, and the mood is back to 0x80.
    fn end_anger(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// The form the navi's side asked to change into at this turn's start
    /// (none: none, or the base form).
    fn form_change_target(&self, o: ObjectRef) -> Option<crate::FormHandle>;
    /// BN5's Soul Unison: the turns the soul the side asked for lasts, and
    /// whether it is Chaos Unison (the turn's transform record's +3, +1).
    fn form_change_soul(&self, o: ObjectRef) -> (u8, bool);
    /// BN5's soul change's first step (0x08011FAC): the navi stops moving,
    /// flinching, being paralyzed and sliding, and forgets a slide request
    /// and its slide's step (a part of BN6's `sub_80158FA`).
    fn stop_moving(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// `sub_80C4526(overlay, 1)`: an overlay on an image sits in front (an
    /// idle overlay keeps its owner's height).
    fn pin_overlay(&mut self, o: ObjectRef);
    /// `sub_801171C`: back to the idle action (the animation untouched).
    fn end_attack(&mut self, o: ObjectRef);
    /// `object_setAttack0..5`: start the content action `action` (a handle
    /// of the action registry); `kind` records which helper (1 buster, 2
    /// chip or charged shot, 3 special, 4 move...).
    fn set_content_attack(&mut self, o: ObjectRef, action: u16, kind: u8) -> ApiResult<()>;
    /// `sub_801011A`: clear the attack's link bytes and unfreeze the
    /// target marker.
    fn reset_attack_links(&mut self, o: ObjectRef);
    /// `sub_800FA54`: the held direction (1 up, 2 down, 3 back, 4
    /// forward; 0 none).
    fn held_direction(&self, o: ObjectRef) -> u8;
    /// `sub_800F964`: where a step in `dir` would go, if it can.
    fn step_target(&self, o: ObjectRef, dir: u8) -> Option<PanelPos>;
    /// `sub_80116AE`: start a step toward `dir` from input.
    fn start_move(&mut self, o: ObjectRef, dir: u8);
    /// `sub_800EDD0`'s chip: the hand's chip at the cursor (none: the hand
    /// is empty).
    fn next_chip(&self, o: ObjectRef) -> ApiResult<Option<crate::ChipHandle>>;
    /// `sub_800FB54`: use the chip the navi's requests ask for (as idle
    /// does on A): whether its use started.
    fn use_chip(&mut self, o: ObjectRef) -> ApiResult<bool>;
    /// The attack `chip` starts (`sub_80127C0`'s dispatch: its action, or
    /// the dimming, navi or instant chip's), in `set_attack` slot `kind`.
    fn start_chip_attack(&mut self, o: ObjectRef, chip: crate::ChipHandle, kind: u8) -> ApiResult<()>;
    /// `sub_80116AE(5, end_lag, 2)`: a step straight to `target` (column 0:
    /// no step), then `end_lag` ticks; with `face`, BN5's `sub_80116F6`'s
    /// (the object a step turns to face in the panel patterns 0x23, 0x31
    /// and 0x33).
    fn start_move_to(&mut self, o: ObjectRef, target: PanelPos, end_lag: u16, face: Option<ObjectRef>) -> ApiResult<()>;
    /// `sub_80117BA`: weapon `weapon`'s setup, and the action it names
    /// started in `set_attack` slot `kind`.
    fn start_weapon(&mut self, o: ObjectRef, weapon: crate::WeaponHandle, kind: u8) -> ApiResult<()>;
    /// `sub_801B9E6`'s attack, from inside the wrapper (the role
    /// `actions.wrapper`): run the action the navi runs.
    fn run_wrapped(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// `sub_800FC30`: the next chip in the hand starts inside the wrapper
    /// (prepared, its action started, `wrapped` 1): not one with the
    /// `no_chain` trait, a dimming chip, or the empty hand. Whether it did.
    fn chain_next_chip(&mut self, o: ObjectRef) -> ApiResult<bool>;
    /// `sub_8013CC4`: the NaviCust panel-trail bugs on the panel the navi
    /// leaves, `from`.
    fn panel_trail(&mut self, o: ObjectRef, from: PanelPos) -> ApiResult<()>;
    /// `sub_80E1654` (on) and `sub_80E1662` (off): the target marker holds
    /// where it is, looking locked on, or follows its target again.
    fn freeze_target_marker(&mut self, marker: ObjectRef, on: bool) -> ApiResult<()>;
    /// `sub_800F2FC`: turn to face `target`'s column.
    fn face_toward(&mut self, o: ObjectRef, target: ObjectRef) -> ApiResult<()>;
    /// `ho_8026554`: the panel the navi would attack `target` from in
    /// Beast Out lock-on mode `mode` (its own panel for mode 0 or a
    /// target off the field; (0, 0x7F) when no panel fits).
    fn lockon_panel(&self, o: ObjectRef, target: PanelPos, mode: Option<crate::LockonHandle>) -> PanelPos;
    /// `object_canMove`: not immobilized, sliding or moving.
    fn can_move(&self, o: ObjectRef) -> bool;
    /// `sub_800E2FC`: heal `amount` HP with the recovery sparkle and
    /// sound; with `anti_recovery`, an opponent's armed AntiRecv turns it
    /// into damage instead (true when it did).
    fn heal(&mut self, o: ObjectRef, amount: u16, anti_recovery: bool) -> bool;
    /// `sub_801265A`: the buster's damage (the attack level, with the
    /// navi's and form's bonus, at most 10; 1 when worn out).
    fn buster_damage(&self, o: ObjectRef) -> u16;
    /// `sub_80127C0(0)`: fill the attack variables from the chip at the
    /// hand's cursor (its damage, bonuses and modifiers).
    fn prepare_chip(&mut self, o: ObjectRef);
    /// Obstacles the navi absorbed, oldest first: (look, animation), the
    /// look an absorbed-look record's handle.
    fn absorbed(&self, o: ObjectRef) -> ApiResult<Vec<(u16, u8)>>;
    /// Add one (false when the navi has eight).
    fn push_absorbed(&mut self, o: ObjectRef, look: u16, anim: u8) -> ApiResult<bool>;
    /// Take the newest.
    fn pop_absorbed(&mut self, o: ObjectRef) -> ApiResult<Option<(u16, u8)>>;

    // ---- Sprites -------------------------------------------------------------

    /// `sprite_load`: load a sprite (its animation and look reset) and let
    /// it animate (`no_sprite_update` off).
    fn sprite_load(&mut self, o: ObjectRef, id: SpriteId);
    /// `sub_800F29C`, then `sub_800FC9E` or `sub_800F26C`, and
    /// `sprite_load`: load the sprite `like` is drawn with (a player navi's
    /// by its side's navi and form, a field object's by its NameID's
    /// look): a stand-in for its user.
    fn sprite_load_like(&mut self, o: ObjectRef, like: ObjectRef) -> ApiResult<()>;
    /// Load the sprite `owner`'s NameID record gives it: a player's own by
    /// its navi stats (`sub_800FC9E`), another object's its NameID look
    /// (`sub_800F26C`).
    fn sprite_load_look_of(&mut self, o: ObjectRef, owner: ObjectRef);
    /// Start animation `anim` from its first frame.
    fn sprite_set_animation(&mut self, o: ObjectRef, anim: u8);
    /// Advance the animation one tick (no gating).
    fn sprite_step(&mut self, o: ObjectRef);
    /// `sub_800F26C`: whether the look `o`'s NameID gives is `sprite` (an
    /// object's NameID look; a navi's or form's NameID gives its sprite).
    /// An error for a NameID the content has no look for.
    fn name_look_is(&self, o: ObjectRef, sprite: SpriteId) -> ApiResult<bool>;
    /// `sub_80030BA`: where part `n` of the current frame sits, in pixels
    /// from the object, unflipped; (0, 0) when the frame has fewer parts.
    fn sprite_part_offset(&self, o: ObjectRef, n: u8) -> (i32, i32);
    fn sprite_get(&self, o: ObjectRef, f: SpriteField) -> Value;
    fn sprite_set(&mut self, o: ObjectRef, f: SpriteField, v: Value) -> ApiResult<()>;

    // ---- Collision -------------------------------------------------------------

    /// Give the object a collision slot; false if none is free.
    fn create_collision(&mut self, o: ObjectRef) -> bool;
    /// Set up the registration from the object: its side, panel, element
    /// and damage, and what it is and reacts to (collision types).
    fn setup_collision(&mut self, o: ObjectRef, self_type: crate::CollisionHandle, target_type: crate::CollisionHandle, hit_mod: u8);
    /// `sub_801A082`: redo the types (and damage) of a registration.
    fn reset_collision_types(&mut self, o: ObjectRef, self_type: crate::CollisionHandle, target_type: crate::CollisionHandle, hit_mod: u8);
    fn collision_get(&self, o: ObjectRef, f: CollisionField) -> ApiResult<Value>;
    /// The damage taken this window in element `element` (0 null, 1 fire,
    /// 2 aqua, 3 elec, 4 wood, 5 the sixth slot), as totaled.
    fn collision_element_damage(&self, o: ObjectRef, element: u8) -> ApiResult<u16>;
    /// `sub_801A4DC`: the objects whose collisions hit `o`'s this window
    /// (the registrations' parents), in slot order.
    fn collision_hit_by(&self, o: ObjectRef) -> ApiResult<Vec<ObjectRef>>;
    fn collision_set(&mut self, o: ObjectRef, f: CollisionField, v: Value) -> ApiResult<()>;
    /// Register on the region's panels (clearing the last results).
    fn present_collision(&mut self, o: ObjectRef);
    /// Unregister, resolving hits against whatever is registered there.
    fn remove_collision(&mut self, o: ObjectRef);
    fn free_collision(&mut self, o: ObjectRef);
    /// `object_setCollisionPanelsToCurrent`: the registration's panel
    /// becomes the object's (its move direction kept).
    fn set_collision_panel(&mut self, o: ObjectRef);
    /// `object_highlightCurrentCollisionPanels`: highlight the panels the
    /// registration's region covers.
    fn highlight_collision_panels(&mut self, o: ObjectRef);
    /// `object_spawnCollisionEffect`: the hit spark of a registration that
    /// just hit something (one RNG draw when it shows).
    fn hit_spark(&mut self, o: ObjectRef);
    /// `sub_801156A`: an object with HP takes this tick's damage (the sum
    /// of its hits by element): a guard spark if it blocked, then the HP
    /// loss (not in mode 1), and a white flash while hit, with a sound in
    /// modes 0 and 2. -1 when its HP ran out, 1 when hit in another mode,
    /// else 0.
    fn take_damage(&mut self, o: ObjectRef, mode: u8) -> i32;
    /// `sub_801A7CC`: raise a barrier on `o` (its collision data).
    fn raise_barrier(&mut self, o: ObjectRef, spec: BarrierSpec) -> ApiResult<()>;

    // ---- Services ------------------------------------------------------------

    /// Run a step of the dimming service for the controller `o`; `chip`
    /// is the chip the controller shows (AntiNavi's steps read it; none:
    /// the zeroed chip field's).
    fn dimming(&mut self, o: ObjectRef, step: DimmingStep, chip: Option<crate::ChipHandle>);
    /// `sub_800BF16`: `side` starts a dimming with `controller` (None: its
    /// spawn failed), used by `user`; `no_cut_in`: the other side can't cut
    /// in on it. For controllers that aren't a chip's (a trap springing).
    /// `telop`: the chip its telop names (None: a zeroed chip field's, the
    /// roles' `zeroed` chip) and the bonus shown with it (what the
    /// controller's spawn stored at its +0x30 and +0x32; presentation
    /// only).
    fn start_dimming(
        &mut self,
        side: u8,
        no_cut_in: bool,
        controller: Option<ObjectRef>,
        user: ObjectRef,
        telop: Option<(Option<ChipHandle>, u16)>,
    );
    /// `sub_80E1352`: a navi chip's user vanishes while its navi acts.
    fn hide_user(&mut self, user: ObjectRef);
    /// `sub_80E13DC`: and comes back.
    fn show_user(&mut self, user: ObjectRef);
    /// `sub_80E1352(user, 0xF)`: `user` vanishes as with `hide_user`, but
    /// its barrier visual, its confusion and blindness visuals and the HUD
    /// stay (BugFix's glow).
    fn hide_user_sparing(&mut self, user: ObjectRef);
    /// `sub_80E146C`: an actor that isn't a player vanishes (its barrier
    /// visual and its confusion and blindness visuals with it), and
    /// `sub_80E14AC` it is back: BN5's Django's coffin.
    fn hide_actor(&mut self, o: ObjectRef);
    fn show_actor(&mut self, o: ObjectRef);
    /// `sub_80E49C4` (BugFix): a side's NaviCust bugs are fixed: the stats
    /// processing, the panel trail's level, the buster's blanks, the
    /// on-hit status, the custom damage, the emotion, the custom and HP
    /// drains, the battle-start bug and the hand-shrink turn are zeroed.
    fn clear_navicust_bugs(&mut self, side: u8);
    /// `sub_801E658` (BugFix): the save's emotion window glitch is gone
    /// from every console's emotion window, which then flickers (and
    /// draws its console's RNG1) for NaviCust bugs only.
    fn clear_emotion_window_glitch(&mut self);
    /// A navi chip's navi is done: its controller moves on.
    fn navi_chip_left(&mut self, controller: ObjectRef);
    /// The last navi chip used, of either side (`byte_203C960`, BN5's
    /// 0x0203C430): the chip, and the element and the damage word, bonus
    /// included, its navi came with; none since the battle started.
    fn last_navi_chip(&self) -> Option<(ChipHandle, u8, u32)>;
    /// `sub_80E1332`: a navi chip's user warps out (`out`) or back in (the
    /// navi warp, actor 0x2D).
    fn navi_warp(&mut self, user: ObjectRef, out: bool);

    // ---- Obstacles (the obstacle framework) -------------------------------

    /// `setFieldBattleObject_800F614`: register `o` as one of `side`'s
    /// field objects of `class` (0: two a side, 1: one); a third class-0
    /// one (a second class-1) evicts the oldest, whose HP drops to 0.
    fn obstacle_register(&mut self, o: ObjectRef, side: u8, class: u8);
    /// `sub_800F656`: forget `o` in the field-object registry.
    fn obstacle_unregister(&mut self, o: ObjectRef);
    /// `sub_801AD9E` and its variants (`push`): resolve this tick's hits.
    fn obstacle_take_hits(&mut self, o: ObjectRef, push: ObstaclePush) -> ApiResult<()>;
    /// `sub_800F672`: the lifetime; broken when it runs out or the battle
    /// is over, blinking for its last three seconds.
    fn obstacle_tick_lifetime(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// `sub_801B394` and its variants (`crush`, `hold`): damage, removal
    /// and status; the action the kind's own table runs now (None: a status
    /// routine ran).
    fn obstacle_react(&mut self, o: ObjectRef, crush: ObstacleCrush, hold: ObstacleHold) -> ApiResult<Option<u8>>;
    /// Run a shared entry of the obstacle's action table.
    fn obstacle_action(&mut self, o: ObjectRef, a: ObstacleAction) -> ApiResult<()>;
    /// The byte the obstacle's game stores for action `a` of the
    /// framework's numbering (BN6's: the kind's own from 8; BN5's own from
    /// 6, with no frozen or bubbled entries).
    fn obstacle_action_byte(&self, o: ObjectRef, a: u8) -> ApiResult<u8>;
    /// The obstacle's action in the framework's numbering.
    fn obstacle_current_action(&self, o: ObjectRef) -> u8;
    /// How the obstacle is leaving.
    fn obstacle_removal(&self, o: ObjectRef) -> ApiResult<ObstacleRemoval>;
    /// `sub_800F8CE`: blink out for 20 ticks when it vanishes.
    fn obstacle_blink_out(&mut self, o: ObjectRef) -> ApiResult<BlinkOut>;
    /// `sub_800F90E`: absorbed, it flies to the absorbing side's navi as
    /// an absorbed obstacle of look `look` (an absorbed-look record's
    /// handle; the original's obstacle kind), with its animation and
    /// palette.
    fn obstacle_fly_to_absorber(&mut self, o: ObjectRef, look: u16) -> ApiResult<()>;
    /// `sub_802EF5C`: the per-side target tracking some chips keep.
    fn obstacle_release_tracking(&mut self, o: ObjectRef);
    /// Whether one of the field's two stage-object slots is free
    /// (`sub_8007450`'s test).
    fn obstacle_stage_slot_free(&self) -> bool;
    /// `o` takes the first free stage-object slot (`sub_8007450`'s store);
    /// an error if both are taken.
    fn obstacle_enter_stage(&mut self, o: ObjectRef) -> ApiResult<()>;
    /// `o` leaves the stage-object slot it holds.
    fn obstacle_leave_stage(&mut self, o: ObjectRef);
    /// A chip's request of the obstacle `o` (`by`: the requester, whose
    /// side absorbs).
    fn obstacle_request(&mut self, o: ObjectRef, request: ObstacleRequest, by: ObjectRef);
    /// `sub_80EFD74`: `absorber` pulls in every registered field object
    /// (except NameID 0xDA and those already leaving).
    fn obstacle_absorb_all(&mut self, absorber: ObjectRef);
    /// `o` has a collision registration and isn't already leaving the field
    /// (removed by a chip, blinking out or absorbed): the test `sub_80C9EE6`
    /// and `sub_80EFD8C` make before taking an obstacle.
    fn obstacle_present(&self, o: ObjectRef) -> bool;
    /// DustMan's take (`sub_80BC100`): the look the field object `o` would
    /// have thrown back as junk, which its taker keeps: the object's
    /// identity (`sub_800F26C`'s argument); None for the objects DustMan
    /// leaves (`sub_800F486`) and for one with no identity.
    fn junk_look(&self, o: ObjectRef) -> Option<crate::IdentityHandle>;
    /// The r3 the object update loop (`object_800372A`) leaves for the
    /// object updating now: 4 × how many objects of the previous object's
    /// pool it passed before that one this tick. Routines that never set
    /// r3 spawn with it as a position (DustMan's junk).
    fn loop_register(&self) -> u32;
    /// CrosOver's MegaMan (`sub_80BDBC8`): `o` takes its user's identity
    /// when the user is MegaMan (`megaman`) or one of his forms, else
    /// MegaMan's own; that identity's sprite (a player's by its side's navi
    /// and form, `sub_800FC9E`; MegaMan's base sprite for another user's),
    /// with a ground shadow at animation 0 (loaded by the next sprite
    /// update); and its side's form's palette (`byte_80203EA`). True when
    /// it took the user's.
    fn wear_navi_image(&mut self, o: ObjectRef, user: ObjectRef, megaman: crate::NaviHandle) -> ApiResult<bool>;
    /// MstrCros's Crosses (`sub_80BE7BC`) and Darkness's Dark MegaMan
    /// (`sub_80BF710`): `o` takes the identity of `navi` (MegaMan) in
    /// `form` (the form's; the navi's own for his base form), the form's
    /// sprite (`sub_800FC9E(0, form)`) with a ground shadow at animation 0
    /// (loaded by the next sprite update), and the form's palette
    /// (`byte_80203EA`).
    fn wear_form_image(&mut self, o: ObjectRef, navi: crate::NaviHandle, form: crate::FormHandle) -> ApiResult<()>;
    /// `sub_8010DF6` (`on`, its r2 1) or `sub_8011044` by the actor record
    /// of `o`'s identity: the parts the navi image wears.
    fn navi_image_parts(&mut self, o: ObjectRef, on: bool);
    /// `sub_80DBB64`: put the junk look `look` (a field object's identity)
    /// on `o`'s sprite (`sub_800F26C`: the sprite, a shadow if the look has
    /// one, its animation and palette; flipped by `o`'s side unless the
    /// look keeps its own); false when the look shows nothing (the table's
    /// category 0xFF).
    fn wear_junk_look(&mut self, o: ObjectRef, look: crate::IdentityHandle) -> ApiResult<bool>;
    /// `sub_80DC3B2`'s test: a field object by its identity (the
    /// original's NameID word 0xCD to 0xFF, its +0x2A half 0) but those
    /// `sub_800F486` excludes, which BlzrdBal's ball swallows.
    fn obstacle_swallowable(&self, o: ObjectRef) -> bool;
    /// BN5's Poltergeist's test (0x080E8CA0): the object's identity is
    /// `throwable`.
    fn obstacle_throwable(&self, o: ObjectRef) -> bool;
    /// `sub_800F6AC`: ask the field object `o` to be picked up by `side` and
    /// thrown at panel (x, y) after shaking `shake` ticks, hitting with the
    /// damage word `damage` (`sub_8018002` does it).
    fn obstacle_throw(&mut self, o: ObjectRef, side: u8, x: u8, y: u8, shake: u8, damage: u32);
    /// BN5's ColonelSoul army (docs/design/bn5-map.md §15.11): arm side
    /// `side` (BattleState+0x5C bit 0x10 or 0x20, 0x080CAC1E) with its
    /// soldiers' damage words (0x080CABF8): the sword soldier's and the gun
    /// soldier's. While it is armed, an obstacle of a game whose rules have
    /// `effects.obstacle_soldiers`, standing on a panel of the side's enemy,
    /// turns into the side's soldier.
    fn obstacle_arm_soldiers(&mut self, side: u8, sword: u32, gun: u32);
    /// 0x080CAC30: disarm side `side`; its words stay.
    fn obstacle_disarm_soldiers(&mut self, side: u8);
    /// Whether side `side` is armed, and its words (0x080CAC06,
    /// 0x080CAC12): what its soldiers strike with as they read them.
    fn obstacle_soldiers(&self, side: u8) -> (bool, u32, u32);
    // ---- Field objects (obstacles) -------------------------------------------

    /// Whether another object asked `flag` of the field object `o`.
    fn obstacle_flag(&self, o: ObjectRef, flag: ObstacleFlag) -> ApiResult<bool>;
}
