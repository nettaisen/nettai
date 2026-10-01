//! What the ruleset needs from content by role (docs/design/
//! content-model-v2.md §7.4): `define.roles { ... }` in `rules/roles.luau`,
//! once. The ruleset starts and recognizes the actions and kinds it needs
//! through these, never by the original's numbers or by content's keys.
//!
//! A role names a definition, or, while what it needs is still a v1
//! registration, that registration through the transitional `legacy`
//! marker (`{ legacy = { action = 0x49 } }`, `{ legacy = { kind = "a-v1-kind" } }`;
//! counted by the ratchet). A legacy action number nothing implements yet
//! leaves the role unported: starting it is "not implemented yet", as the
//! number was. A role content hasn't filled is an error where the ruleset
//! needs it.

use std::collections::BTreeMap;

use bn6_content_api::{
    ActionHandle, ChipHandle, CollisionHandle, EffectHandle, FnId, KindHandle, LockonHandle, RegionHandle, SparkHandle,
    StatusHandle,
};

use super::{BannerId, SpriteId};
use crate::sound::SoundId;

/// The actions the ruleset starts or recognizes by role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ActionRole {
    /// AntiDmg's counter (`sub_801056A`, `sub_80105F2`; the original's 0x47).
    AntiDamageCounter,
    /// AntiSwrd's counter (0x48).
    AntiSwordCounter,
    /// BodyGrd's counter (0x4B).
    BodyGuardCounter,
    /// The charged shot a navi's request starts from idle without its
    /// weapon's setup (`sub_8010312`'s request 0x20; 0x16, MegaMan's
    /// charged shot).
    ForcedChargedShot,
    /// The strike the navi's request 0x80000 starts from idle (0x49, the
    /// machine swords' strike by the attack's variant).
    StunStrike,
    /// What a Cross navi protected at the battle's end runs (0x4D).
    CrossProtect,
    /// The turn L or R starts where turning is enabled (0x3B).
    Turn,
    /// A Cross navi's knock-out (0x4C).
    CrossDeath,
    /// A Cross navi's volley (0x30).
    Volley,
    /// The charged sword (SlashCross's charged slash, 0x41), which the
    /// Beast rush recognizes for its lock-on mode.
    ChargedSword,
    /// The Beast forms' charged claw (0x52), which the Beast rush and chip
    /// use recognize.
    BeastClaw,
    /// DustCross Beast's scatter (0x50), during which the ruleset doesn't
    /// ground a MegaMan navi.
    DustBeastScatter,
}

impl ActionRole {
    pub const ALL: [ActionRole; 12] = [
        ActionRole::AntiDamageCounter,
        ActionRole::AntiSwordCounter,
        ActionRole::BodyGuardCounter,
        ActionRole::ForcedChargedShot,
        ActionRole::StunStrike,
        ActionRole::CrossProtect,
        ActionRole::Turn,
        ActionRole::CrossDeath,
        ActionRole::Volley,
        ActionRole::ChargedSword,
        ActionRole::BeastClaw,
        ActionRole::DustBeastScatter,
    ];

    /// Its name in `rules/roles.luau`'s `actions`.
    pub fn name(self) -> &'static str {
        match self {
            ActionRole::AntiDamageCounter => "anti_damage_counter",
            ActionRole::AntiSwordCounter => "anti_sword_counter",
            ActionRole::BodyGuardCounter => "body_guard_counter",
            ActionRole::ForcedChargedShot => "forced_charged_shot",
            ActionRole::StunStrike => "stun_strike",
            ActionRole::CrossProtect => "cross_protect",
            ActionRole::Turn => "turn",
            ActionRole::CrossDeath => "cross_death",
            ActionRole::Volley => "volley",
            ActionRole::ChargedSword => "charged_sword",
            ActionRole::BeastClaw => "beast_claw",
            ActionRole::DustBeastScatter => "dust_beast_scatter",
        }
    }

    pub fn named(name: &str) -> Option<ActionRole> {
        ActionRole::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// The object kinds the ruleset spawns by role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KindRole {
    /// What an obstacle absorbed flies off as (`obstacle.fly_to_absorber`).
    AbsorbedObstacle,
    /// GroundCross's charged chips' falling rocks (`sub_8012CB2`).
    FallingRock,
    /// The NaviCust supports' dimming controller (Rush, Beat, Tango: the
    /// original's effect object #0x79, `sub_80E8FE0`).
    Support,
    /// AntiRecv's counterattack, the dimming controller a heal meets when
    /// the other side has AntiRecv armed (the original's effect object
    /// #0x2C, `sub_80E3728`).
    AntiRecovery,
}

impl KindRole {
    pub const ALL: [KindRole; 4] = [
        KindRole::AbsorbedObstacle,
        KindRole::FallingRock,
        KindRole::Support,
        KindRole::AntiRecovery,
    ];

    /// Its name in `rules/roles.luau`'s `kinds`.
    pub fn name(self) -> &'static str {
        match self {
            KindRole::AbsorbedObstacle => "absorbed_obstacle",
            KindRole::FallingRock => "falling_rock",
            KindRole::Support => "support",
            KindRole::AntiRecovery => "anti_recovery",
        }
    }

    pub fn named(name: &str) -> Option<KindRole> {
        KindRole::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// The Beast Out lock-on modes the ruleset names by role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LockonRole {
    /// The Beast forms' charged claw's (`sub_80EAF1A`), whatever chip the
    /// attack carries.
    BeastClaw,
}

impl LockonRole {
    pub const ALL: [LockonRole; 1] = [LockonRole::BeastClaw];

    /// Its name in `rules/roles.luau`'s `lockon`.
    pub fn name(self) -> &'static str {
        match self {
            LockonRole::BeastClaw => "beast_claw",
        }
    }

    pub fn named(name: &str) -> Option<LockonRole> {
        LockonRole::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// The status effects the ruleset inflicts itself, by role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StatusRole {
    /// What a damage word's paralysis bit (0x4000) makes its hits carry
    /// (`sub_8019F44`).
    DamageWordParalysis,
    /// What a counter hit lands instead of a flinch (`sub_800EB26`).
    CounterParalysis,
    /// What an aqua hit gives a body standing on ice (`sub_801A0E0`'s
    /// freeze).
    IceFreeze,
    /// The NaviCust on-hit bug's statuses (`sub_8013F1E`, bug levels 1 and
    /// 2).
    HitBugBlind,
    HitBugConfuse,
}

impl StatusRole {
    pub const ALL: [StatusRole; 5] = [
        StatusRole::DamageWordParalysis,
        StatusRole::CounterParalysis,
        StatusRole::IceFreeze,
        StatusRole::HitBugBlind,
        StatusRole::HitBugConfuse,
    ];

    /// Its name in `rules/roles.luau`'s `statuses`.
    pub fn name(self) -> &'static str {
        match self {
            StatusRole::DamageWordParalysis => "damage_word_paralysis",
            StatusRole::CounterParalysis => "counter_paralysis",
            StatusRole::IceFreeze => "ice_freeze",
            StatusRole::HitBugBlind => "hit_bug_blind",
            StatusRole::HitBugConfuse => "hit_bug_confuse",
        }
    }

    pub fn named(name: &str) -> Option<StatusRole> {
        StatusRole::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// A group of roles that name definitions of one registry, or assets of
/// one kind: the roles, each with its name in `rules/roles.luau`'s group.
macro_rules! definition_roles {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident = $key:literal,)* }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name {
            $($(#[$vdoc])* $variant,)*
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant,)*];

            /// Its name in its group of `rules/roles.luau`.
            pub fn name(self) -> &'static str {
                match self {
                    $($name::$variant => $key,)*
                }
            }

            pub fn named(name: &str) -> Option<$name> {
                $name::ALL.iter().copied().find(|r| r.name() == name)
            }
        }
    };
}

definition_roles! {
    /// The one-shot effects the ruleset shows itself, by role (`effects`).
    EffectRole {
        /// A navi deleted (`sub_8010820`'s two explosions), a cross merged
        /// into MegaMan, a navi arriving mid-battle or back from a Cross
        /// special, a form put on or taken off.
        Deletion = "deletion",
        /// HP recovered: a heal's sparkle (`sub_800E2FC`), a recovery
        /// chip's, the undershirt's credits', a navi appearing mid-battle.
        Recovery = "recovery",
        /// The flash where a side cut in (`sub_800B8EE`).
        CutInFlash = "cut_in_flash",
        /// The mark over a navi whose trap sprang (`sub_800ABC6`: AntiNavi,
        /// AntiRecv).
        TrapMark = "trap_mark",
        /// What an obstacle encased in ice or a bubble flickers with
        /// (`sub_8018186`).
        Encased = "encased",
        /// A Cross or Beast Out put on (`sub_8015166`).
        FormChange = "form_change",
        /// Beast Over's beast (`sub_80151D4`), Gregar's and Falzar's.
        BeastOverGregar = "beast_over_gregar",
        BeastOverFalzar = "beast_over_falzar",
        /// Beast Over's blast, after its beast.
        BeastOverBlast = "beast_over_blast",
        /// One of the bursts around a navi going Beast Over
        /// (`sub_80E7D0C`).
        BeastOverBurst = "beast_over_burst",
    }
}

definition_roles! {
    /// The hit sparks the ruleset shows itself, by role (`sparks`).
    SparkRole {
        /// What a new collision registration's hits show until its owner
        /// says otherwise (the original's zeroed hit-effect byte).
        Plain = "plain",
        /// A blocked hit's (`object_spawnHiteffect`).
        Guard = "guard",
        /// An eruption's hits' (a volcano panel's, `sub_80E1DA0`).
        Eruption = "eruption",
        /// A thrown obstacle's landing's (`sub_8018002`).
        ThrownObstacle = "thrown_obstacle",
        /// A navi's programs uninstalled (`sub_80140EE`).
        Uninstall = "uninstall",
    }
}

definition_roles! {
    /// The hit regions the ruleset registers itself, by role (`regions`).
    RegionRole {
        /// The registration's own panel: what `object_setupCollisionData`
        /// gives every registration.
        Anchor = "anchor",
    }
}

definition_roles! {
    /// The collision types the ruleset registers itself, by role
    /// (`collision`).
    CollisionRole {
        /// A navi's body, and its body when it floats (`sub_8010BD8`:
        /// FloatShoes, the forms that float).
        Navi = "navi",
        FloatingNavi = "floating_navi",
        /// What a navi's body reacts to.
        NaviTarget = "navi_target",
        /// An eruption (a volcano panel's), and what it reaches.
        Eruption = "eruption",
        EruptionTarget = "eruption_target",
        /// A thrown obstacle's landing, and what it reaches.
        ThrownObstacle = "thrown_obstacle",
        ThrownObstacleTarget = "thrown_obstacle_target",
    }
}

definition_roles! {
    /// The sounds the ruleset plays itself, by role (`sounds`): assets
    /// (`asset.sound`).
    SoundRole {
        /// A panel cracks or breaks.
        PanelCrack = "panel_crack",
        /// A panel turns to poison (`sub_800C9CE`).
        PanelPoison = "panel_poison",
        /// A player's own navi is hit (what its player hears), and the
        /// custom-HP bug's damage (`sub_8013FD0`).
        OwnHit = "own_hit",
        /// A navi is hit (what the other player hears), and an object
        /// hurt without flinching (`sub_801A29A` mode 2).
        Hit = "hit",
        /// An object takes damage (`sub_801A29A` mode 0), an obstacle hit.
        Damage = "damage",
        /// A blocked hit (`object_spawnHiteffect`), a trap catching one.
        Guard = "guard",
        /// A counter hit ("COUNTER HIT!").
        CounterHit = "counter_hit",
        /// A navi deleted (`sub_8010820`).
        Deleted = "deleted",
        /// HP recovered (`sub_800E2FC`).
        Recovery = "recovery",
        /// A chip's damage bonus as it is used (a charged chip's, Full
        /// Synchro's, anger's).
        DamageBonus = "damage_bonus",
        /// The battle pauses and resumes.
        Pause = "pause",
        /// The custom gauge fills.
        GaugeFull = "gauge_full",
        /// The low-HP alarm, every 45 ticks to the navi's player.
        LowHp = "low_hp",
        /// A side cuts in; a trap springs (`sub_800ABC6`).
        CutIn = "cut_in",
        /// A telop (the chip's name) shows.
        Telop = "telop",
        /// The buster starts charging, and is fully charged.
        BusterCharge = "buster_charge",
        BusterCharged = "buster_charged",
        /// A navi frozen, bubbled, and its bubble popped.
        Freeze = "freeze",
        Bubble = "bubble",
        BubblePop = "bubble_pop",
        /// The confusion's stars, every 60 ticks.
        Confusion = "confusion",
        /// A navi turns invisible (`sub_8010474`).
        Invisible = "invisible",
        /// A navi shows again: invisibility or a dive ended, a navi
        /// entering.
        Appear = "appear",
        /// A navi arrives mid-battle.
        Arrive = "arrive",
        /// Something goes: a defensive chip a cursor hit cancels, a navi's
        /// programs uninstalled, a Cross navi's special, a form taken off.
        Fade = "fade",
        /// A thrown obstacle is lifted, and flies.
        ObstacleLift = "obstacle_lift",
        ObstacleThrow = "obstacle_throw",
        /// A cross merges into MegaMan.
        CrossMerge = "cross_merge",
        /// A form change starts.
        FormChange = "form_change",
        /// A Cross is put on (two sounds at once).
        CrossChange = "cross_change",
        CrossChangeChime = "cross_change_chime",
        /// A Beast Out is put on.
        BeastOut = "beast_out",
        /// The beast's roar as a Beast form starts: Gregar's and Falzar's.
        GregarRoar = "gregar_roar",
        FalzarRoar = "falzar_roar",
        /// Beast Over's rumbles, and each of its bursts.
        BeastOverRumble = "beast_over_rumble",
        BeastOverBurst = "beast_over_burst",
        /// A Cross navi starts its special's chip.
        CrossSpecial = "cross_special",
        /// SELECT pressed with too little gauge: its player hears that it
        /// can't.
        Refused = "refused",
    }
}

definition_roles! {
    /// The music the ruleset starts itself, by role (`music`): sound
    /// assets. (A stage's music is its own.)
    MusicRole {
        /// A link battle's, whatever its stage's.
        LinkBattle = "link_battle",
        /// The winner's, in special battles and otherwise; the loser's.
        WinnerSpecial = "winner_special",
        Winner = "winner",
        Loser = "loser",
    }
}

definition_roles! {
    /// The sprites of the engine's own kinds, by role (`sprites`): assets
    /// (`asset.sprite`).
    SpriteRole {
        /// The charge glow of a B charge, and of an A charge
        /// (`sub_80E0F2E`).
        ChargeGlow = "charge_glow",
        ChargeGlowA = "charge_glow_a",
        /// The Full Synchro aura.
        FullSynchroAura = "full_synchro_aura",
        /// The status visuals over a navi: confusion's stars, blindness,
        /// and the immobilized one (a row the status routine never spawns).
        Confusion = "confusion",
        Blindness = "blindness",
        Immobilized = "immobilized",
        /// The ice block around a frozen navi, and the bubble around a
        /// bubbled one.
        Ice = "ice",
        Bubble = "bubble",
        /// The mark over a navi a hit told something about ("!!", the HP
        /// bug's).
        HitMarker = "hit_marker",
        /// A volcano panel's eruption.
        Eruption = "eruption",
        /// The Beast forms' lock-on marker.
        LockonMarker = "lockon_marker",
    }
}

definition_roles! {
    /// The banners the ruleset shows itself, by role (`banners`): assets
    /// (`asset.banner`). (The win and lose banners are the navi's.)
    BannerRole {
        /// A round's start (the battle's number).
        RoundStart = "round_start",
        /// A turn's start, and the final turns'.
        TurnStart = "turn_start",
        FinalTurn = "final_turn",
        /// A draw.
        Draw = "draw",
        /// The damage judge's.
        Judge = "judge",
        /// A telop on the local player's side, and on the other's.
        Telop = "telop",
        TelopRemote = "telop_remote",
        /// A Program Advance, and a selection that makes none.
        ProgramAdvance = "program_advance",
        ProgramAdvanceEmpty = "program_advance_empty",
    }
}

/// The functions the ruleset calls by role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HookRole {
    /// `(navi)`: the NaviCust FirstBarrier's barrier and its visual
    /// (`sub_8013892`: `sub_801A7CC` with the navi stat, which the game's
    /// program sets to 1, and `sub_80E0D98`).
    FirstBarrier,
    /// `(obstacle, ice, class)`: what an obstacle encased in ice or a
    /// bubble becomes (`sub_801813A`'s end: an ice block, `sub_80CFBC4`
    /// variant 3 in its registry class; or the bubble, attack #0xA3
    /// `sub_80D99EC`).
    Encased,
}

impl HookRole {
    pub const ALL: [HookRole; 2] = [HookRole::FirstBarrier, HookRole::Encased];

    /// Its name in `rules/roles.luau`'s `hooks`.
    pub fn name(self) -> &'static str {
        match self {
            HookRole::FirstBarrier => "first_barrier",
            HookRole::Encased => "encased",
        }
    }

    pub fn named(name: &str) -> Option<HookRole> {
        HookRole::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// The chips the ruleset names by role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChipRole {
    /// What a zeroed chip field reads: the chip the original's chip 0 is
    /// (a hand's empty selection, the attack's cleared chip, a
    /// non-player's carried chip, a side's special chip never set).
    Zeroed,
    /// The Beast Out chip the custom screen offers and recognizes
    /// (`sub_802A00C`).
    BeastOut,
    /// What an illegal pick counts as in a selection (`getChipID_802A54E`).
    Invalid,
    /// The chips the NaviCust supports' telops name (`sub_80E90FE`).
    Rush,
    Beat,
    Tango,
}

impl ChipRole {
    pub const ALL: [ChipRole; 6] =
        [ChipRole::Zeroed, ChipRole::BeastOut, ChipRole::Invalid, ChipRole::Rush, ChipRole::Beat, ChipRole::Tango];

    /// Its name in `rules/roles.luau`'s `chips`.
    pub fn name(self) -> &'static str {
        match self {
            ChipRole::Zeroed => "zeroed",
            ChipRole::BeastOut => "beast_out",
            ChipRole::Invalid => "invalid",
            ChipRole::Rush => "rush",
            ChipRole::Beat => "beat",
            ChipRole::Tango => "tango",
        }
    }

    pub fn named(name: &str) -> Option<ChipRole> {
        ChipRole::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// What an action role names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoleAction {
    /// A content action (a definition, or a v1 registration by number).
    Action(ActionHandle),
    /// A legacy action number nothing implements yet.
    Unported(u8),
}

/// What a kind role names.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RoleKind {
    Kind(KindHandle),
    /// A legacy kind key the content doesn't have (a content set without
    /// that v1 kind; the lint reports it).
    Missing(String),
}

/// The roles content filled.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Roles {
    pub actions: BTreeMap<ActionRole, RoleAction>,
    pub kinds: BTreeMap<KindRole, RoleKind>,
    pub hooks: BTreeMap<HookRole, FnId>,
    pub chips: BTreeMap<ChipRole, ChipHandle>,
    pub lockons: BTreeMap<LockonRole, LockonHandle>,
    pub statuses: BTreeMap<StatusRole, StatusHandle>,
    pub effects: BTreeMap<EffectRole, EffectHandle>,
    pub sparks: BTreeMap<SparkRole, SparkHandle>,
    pub regions: BTreeMap<RegionRole, RegionHandle>,
    pub collisions: BTreeMap<CollisionRole, CollisionHandle>,
    pub sounds: BTreeMap<SoundRole, SoundId>,
    pub music: BTreeMap<MusicRole, SoundId>,
    pub banners: BTreeMap<BannerRole, BannerId>,
    pub sprites: BTreeMap<SpriteRole, SpriteId>,
}

impl Roles {
    /// The action of `role`, if content filled it with one.
    pub fn try_action(&self, role: ActionRole) -> Option<ActionHandle> {
        match self.actions.get(&role)? {
            RoleAction::Action(h) => Some(*h),
            RoleAction::Unported(_) => None,
        }
    }

    /// The action of `role`; a role content hasn't filled, or whose action
    /// isn't implemented yet, is a panic naming it.
    pub fn action(&self, role: ActionRole) -> ActionHandle {
        match self.actions.get(&role) {
            Some(RoleAction::Action(h)) => *h,
            Some(RoleAction::Unported(n)) => {
                panic!("player action {n:#x} (the role actions.{}) is not implemented yet", role.name())
            }
            None => panic!("the role actions.{} is not filled (define.roles in rules/roles.luau)", role.name()),
        }
    }

    /// The chip of `role`, if content filled it.
    pub fn try_chip(&self, role: ChipRole) -> Option<ChipHandle> {
        self.chips.get(&role).copied()
    }

    /// The chip of `role`; a role content hasn't filled is a panic naming
    /// it.
    pub fn chip(&self, role: ChipRole) -> ChipHandle {
        self.try_chip(role)
            .unwrap_or_else(|| panic!("the role chips.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    /// Whether `h` is the chip of `role`.
    pub fn is_chip(&self, role: ChipRole, h: ChipHandle) -> bool {
        self.try_chip(role) == Some(h)
    }

    /// Whether `h` is the action of `role`.
    pub fn is_action(&self, role: ActionRole, h: ActionHandle) -> bool {
        self.try_action(role) == Some(h)
    }

    /// The function of `role`; a role content hasn't filled is a panic
    /// naming it.
    pub fn hook(&self, role: HookRole) -> FnId {
        *self
            .hooks
            .get(&role)
            .unwrap_or_else(|| panic!("the role hooks.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    /// The status of `role`; a role content hasn't filled is a panic naming
    /// it.
    pub fn status(&self, role: StatusRole) -> StatusHandle {
        *self
            .statuses
            .get(&role)
            .unwrap_or_else(|| panic!("the role statuses.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    /// The effect of `role`; a role content hasn't filled is a panic naming
    /// it (as for the sparks, regions and collision types below).
    pub fn effect(&self, role: EffectRole) -> EffectHandle {
        *self
            .effects
            .get(&role)
            .unwrap_or_else(|| panic!("the role effects.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    pub fn spark(&self, role: SparkRole) -> SparkHandle {
        *self
            .sparks
            .get(&role)
            .unwrap_or_else(|| panic!("the role sparks.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    pub fn region(&self, role: RegionRole) -> RegionHandle {
        *self
            .regions
            .get(&role)
            .unwrap_or_else(|| panic!("the role regions.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    pub fn collision(&self, role: CollisionRole) -> CollisionHandle {
        *self
            .collisions
            .get(&role)
            .unwrap_or_else(|| panic!("the role collision.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    /// The sound, the music, the sprite and the banner of `role`
    /// (likewise).
    pub fn sound(&self, role: SoundRole) -> SoundId {
        *self
            .sounds
            .get(&role)
            .unwrap_or_else(|| panic!("the role sounds.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    pub fn music(&self, role: MusicRole) -> SoundId {
        *self
            .music
            .get(&role)
            .unwrap_or_else(|| panic!("the role music.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    pub fn sprite(&self, role: SpriteRole) -> SpriteId {
        *self
            .sprites
            .get(&role)
            .unwrap_or_else(|| panic!("the role sprites.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    pub fn banner(&self, role: BannerRole) -> BannerId {
        *self
            .banners
            .get(&role)
            .unwrap_or_else(|| panic!("the role banners.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    /// The lock-on mode of `role`; a role content hasn't filled is a panic
    /// naming it.
    pub fn lockon(&self, role: LockonRole) -> LockonHandle {
        *self
            .lockons
            .get(&role)
            .unwrap_or_else(|| panic!("the role lockon.{} is not filled (define.roles in rules/roles.luau)", role.name()))
    }

    /// The kind of `role`; a role content hasn't filled is a panic naming
    /// it.
    pub fn kind(&self, role: KindRole) -> KindHandle {
        match self.kinds.get(&role) {
            Some(RoleKind::Kind(h)) => *h,
            Some(RoleKind::Missing(key)) => {
                panic!("the role kinds.{} names the kind {key:?}, which the content doesn't have", role.name())
            }
            None => panic!("the role kinds.{} is not filled (define.roles in rules/roles.luau)", role.name()),
        }
    }
}
