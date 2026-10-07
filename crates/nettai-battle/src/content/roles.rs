//! What the rules need from content by role (docs/design/
//! content-model-v2.md §7.4): the rules' `roles`, a plain table
//! (`rules/roles.luau`), once. The rules start and recognizes the actions and kinds it needs
//! through these, never by the original's numbers or by content's keys.
//!
//! A role names a definition. A role content hasn't filled is an error
//! where the rules need it.

use std::collections::BTreeMap;

use nettai_content_api::{
    ActionHandle, ChipHandle, CollisionHandle, EffectHandle, FnId, KindHandle, RegionHandle, SparkHandle,
    StatusHandle,
};

use super::{BannerId, SpriteId};
use crate::sound::SoundId;

/// The actions the rules start or recognizes by role.
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
    /// machine swords' strike).
    StunStrike,
    /// What a navi switched in (the navi switch) and protected at the
    /// battle's end runs (0x4D).
    SwitchProtect,
    /// The turn L or R starts where turning is enabled (0x3B).
    Turn,
    /// A switched-in navi's knock-out (0x4C): it falls back to the navi
    /// it replaced.
    SwitchKnockout,
    /// What the navi's volley request starts (the original's action 0x30,
    /// on whatever the attack's parameter bytes hold). No EXE6 routine
    /// raises the request, and EXE6's content leaves the role unfilled;
    /// EXE5's loss of HP raises it for a dark MegaMan's last stand
    /// (0x0802C16C), EXE5's action 0x30.
    Volley,
    /// An action during which the navi that changes form isn't grounded
    /// (EXE6's DustCross Beast's scatter, 0x50).
    Ungrounded,
    /// An action during which an invulnerable navi doesn't glow
    /// (`sub_8016860`: EXE6's ChargeCross's tackle, 0x56).
    Glowless,
    /// The wrapper an attack runs inside while its `wrapped` is 1
    /// (`sub_801B9E6`): EXE6's Beast Out rush (`sub_80EAD9C`). Unfilled, the
    /// attack runs as it is.
    Wrapper,
    /// A player's deletion, where the rules have their own (EXE4's action 2,
    /// 0x08010850): the navi's deletion action runs it. Unfilled, the
    /// engine's (EXE6's `sub_80173F4`, EXE5's alike).
    Deletion,
}

impl ActionRole {
    pub const ALL: [ActionRole; 13] = [
        ActionRole::AntiDamageCounter,
        ActionRole::AntiSwordCounter,
        ActionRole::BodyGuardCounter,
        ActionRole::ForcedChargedShot,
        ActionRole::StunStrike,
        ActionRole::SwitchProtect,
        ActionRole::Turn,
        ActionRole::SwitchKnockout,
        ActionRole::Volley,
        ActionRole::Ungrounded,
        ActionRole::Glowless,
        ActionRole::Wrapper,
        ActionRole::Deletion,
    ];

    /// Its name in `rules/roles.luau`'s `actions`.
    pub fn name(self) -> &'static str {
        match self {
            ActionRole::AntiDamageCounter => "anti_damage_counter",
            ActionRole::AntiSwordCounter => "anti_sword_counter",
            ActionRole::BodyGuardCounter => "body_guard_counter",
            ActionRole::ForcedChargedShot => "forced_charged_shot",
            ActionRole::StunStrike => "stun_strike",
            ActionRole::SwitchProtect => "switch_protect",
            ActionRole::Turn => "turn",
            ActionRole::SwitchKnockout => "switch_knockout",
            ActionRole::Volley => "volley",
            ActionRole::Ungrounded => "ungrounded",
            ActionRole::Glowless => "glowless",
            ActionRole::Wrapper => "wrapper",
            ActionRole::Deletion => "deletion",
        }
    }

    pub fn named(name: &str) -> Option<ActionRole> {
        ActionRole::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// The object kinds the rules spawn by role.
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
    /// What the player whose AI index is 10 spawns after its init in battle
    /// mode 9 (`off_80EAA04`'s entry): an attack object (the original's
    /// #0xD2, `sub_80DFD74`) and an actor object (#0x28, `sub_80C02A6`).
    Mode9Attack,
    Mode9Actor,
    /// The ripple over a body under the sea's surface (EXE5's effect object
    /// #0x3E, 0x080E4B64), which the navi's status tick keeps
    /// (0x0800DEB2): the engine gives it the body as its first related.
    DiveRipple,
    /// The charge glow a navi's charge brings, where the rules say a charge
    /// brings its own (`effects.charge_glow = with_charge`: EXE4's effect
    /// object 5, 0x080E215C, which the navi's tail spawns, 0x0800BD88): the
    /// engine gives it the navi as its first related and its side, links it
    /// (`charge_glow`) and shows it (its state's `shown`).
    ChargeGlow,
}

impl KindRole {
    pub const ALL: [KindRole; 8] = [
        KindRole::AbsorbedObstacle,
        KindRole::FallingRock,
        KindRole::Support,
        KindRole::AntiRecovery,
        KindRole::Mode9Attack,
        KindRole::Mode9Actor,
        KindRole::DiveRipple,
        KindRole::ChargeGlow,
    ];

    /// Its name in `rules/roles.luau`'s `kinds`.
    pub fn name(self) -> &'static str {
        match self {
            KindRole::AbsorbedObstacle => "absorbed_obstacle",
            KindRole::FallingRock => "falling_rock",
            KindRole::Mode9Attack => "mode9_attack",
            KindRole::Mode9Actor => "mode9_actor",
            KindRole::Support => "support",
            KindRole::AntiRecovery => "anti_recovery",
            KindRole::DiveRipple => "dive_ripple",
            KindRole::ChargeGlow => "charge_glow",
        }
    }

    pub fn named(name: &str) -> Option<KindRole> {
        KindRole::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// The status effects the rules inflict themselves, by role (in a rule
/// section, by its name: `damage_word_paralysis`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusRole {
    /// What a damage word's paralysis bit (0x4000) makes its hits carry
    /// (`sub_8019F44`).
    DamageWordParalysis,
    /// What EXE5's damage words' bits 0x2000 and 0x1000 make their hits
    /// carry (0x080165EC: status bytes 0x20 and 0x30; the rule
    /// `effects.damage_word`).
    DamageWordConfusion,
    DamageWordBlindness,
    /// What a counter hit lands instead of a flinch (`sub_800EB26`).
    CounterParalysis,
}

impl StatusRole {
    pub const ALL: [StatusRole; 4] = [
        StatusRole::DamageWordParalysis,
        StatusRole::DamageWordConfusion,
        StatusRole::DamageWordBlindness,
        StatusRole::CounterParalysis,
    ];

    /// Its name in `rules/roles.luau`'s `statuses`.
    pub fn name(self) -> &'static str {
        match self {
            StatusRole::DamageWordParalysis => "damage_word_paralysis",
            StatusRole::DamageWordConfusion => "damage_word_confusion",
            StatusRole::DamageWordBlindness => "damage_word_blindness",
            StatusRole::CounterParalysis => "counter_paralysis",
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
    /// The one-shot effects the rules show themselves, by role (`effects`).
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
        /// One of the burst's (`sub_80E7D0C`; EXE6's around a navi going
        /// Beast Over).
        Burst = "burst",
    }
}

definition_roles! {
    /// The hit sparks the rules show themselves, by role (`sparks`).
    SparkRole {
        /// What a new collision registration's hits show until its owner
        /// says otherwise (the original's zeroed hit-effect byte).
        Plain = "plain",
        /// A blocked hit's (`object_spawnHiteffect`).
        Guard = "guard",
        /// A thrown obstacle's landing's (`sub_8018002`).
        ThrownObstacle = "thrown_obstacle",
    }
}

definition_roles! {
    /// The hit regions the rules register themselves, by role (`regions`).
    RegionRole {
        /// The registration's own panel: what `object_setupCollisionData`
        /// gives every registration.
        Anchor = "anchor",
    }
}

definition_roles! {
    /// The collision types the rules register themselves, by role
    /// (`collision`).
    CollisionRole {
        /// A navi's body, and its body when it floats (`sub_8010BD8`:
        /// FloatShoes, the forms that float).
        Navi = "navi",
        FloatingNavi = "floating_navi",
        /// What a navi's body reacts to.
        NaviTarget = "navi_target",
        /// A thrown obstacle's landing, and what it reaches.
        ThrownObstacle = "thrown_obstacle",
        ThrownObstacleTarget = "thrown_obstacle_target",
    }
}

definition_roles! {
    /// The sounds the rules play themselves, by role (`sounds`): assets
    /// (`asset.sound`).
    SoundRole {
        /// A panel cracks or breaks.
        PanelCrack = "panel_crack",
        /// A player's own navi is hit (what its player hears), and the
        /// custom-HP bug's damage (`sub_8013FD0`).
        OwnHit = "own_hit",
        /// A navi is hit (what the other player hears), and an object
        /// hurt without flinching (`sub_801A29A` mode 2).
        Hit = "hit",
        /// A navi in auto battle is hit, what every console hears where a
        /// hit sounds by the navi (`IntakeRules::hit_sound`: EXE4's 0x6D).
        AutoBattleHit = "auto_battle_hit",
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
        /// Each of the burst's (EXE6's around a navi going Beast Over).
        Burst = "burst",
        /// SELECT pressed with too little gauge: its player hears that it
        /// can't (and what can't be picked or taken back on the custom
        /// screen).
        Refused = "refused",
        // The custom screen's (its player hears them).
        /// The custom screen's window starts sliding in.
        CustomOpen = "custom_open",
        /// The custom screen's cursor moves (also in a rules' window).
        CustomCursor = "custom_cursor",
        /// SELECT hides the custom screen's window, and a key brings it back.
        CustomHide = "custom_hide",
        /// The custom screen's hover over a dark chip, every 64 ticks while
        /// it shades the screen (EXE5's; a game that plays none fills none).
        CustomShade = "custom_shade",
        /// A chip or a button's pick (EXE6's Beast Out, the scrap, a
        /// Cross) is picked.
        CustomPick = "custom_pick",
        /// OK is pressed.
        CustomOk = "custom_ok",
        /// A pick is taken back.
        CustomBack = "custom_back",
        /// L: the no-running message.
        CustomRunMessage = "custom_run_message",
        /// R: a description opens, and closes.
        CustomDescription = "custom_description",
        CustomDescriptionClose = "custom_description_close",
        /// A button's pick taken back (EXE6's Beast Out or Cross).
        CustomCancel = "custom_cancel",
        /// ChpShufl's re-deal pressed, and each of its shuffles.
        CustomRedeal = "custom_redeal",
        CustomRedealShuffle = "custom_redeal_shuffle",
        /// DustCross scraps a chip, and is done.
        CustomScrap = "custom_scrap",
        CustomScrapDone = "custom_scrap_done",
        /// The Program Advance animation names a chip of the recipe, and the Program Advance.
        ProgramAdvancePart = "program_advance_part",
        ProgramAdvance = "program_advance",
    }
}

impl SoundRole {
    /// A sound some game hasn't: a game that leaves the role unfilled plays
    /// none there (EXE6's screen plays no hover sound; EXE4's pause, SELECT's
    /// hiding, L's message and a description's close play none: 0x080078F4,
    /// 0x0801E59C, 0x080205C6, 0x080209F8; nor does its telop: 0x08008C2A).
    pub fn optional(self) -> bool {
        matches!(
            self,
            SoundRole::Pause
                | SoundRole::CustomShade
                | SoundRole::CustomHide
                | SoundRole::CustomRunMessage
                | SoundRole::CustomDescriptionClose
                | SoundRole::Telop
        )
    }
}

definition_roles! {
    /// The music the rules start themselves, by role (`music`): sound
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
        /// The target marker (EXE6's Beast forms' lock-on marker).
        TargetMarker = "target_marker",
    }
}

definition_roles! {
    /// The banners the rules show themselves, by role (`banners`): assets
    /// (`asset.banner`). (A loss's banner is the navi's, and a win's where
    /// the flow's `navi_win_banner` says so.)
    BannerRole {
        /// A round's start (the battle's number).
        RoundStart = "round_start",
        /// A turn's start, and the final turns'.
        TurnStart = "turn_start",
        FinalTurn = "final_turn",
        /// A draw.
        Draw = "draw",
        /// A win's where it isn't the winner's navi's ("ENEMY DELETED"),
        /// and a win and a loss on the judge's ruling ("YOU WIN", "YOU
        /// LOSE": time up with navis left on both sides).
        Win = "win",
        WinJudged = "win_judged",
        LoseJudged = "lose_judged",
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

/// The functions the rules call by role.
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
    /// `(navi) -> Object?`: a navi no player controls is being deleted:
    /// what sparkles over it while it flashes (EXE5's 0x08013DA0: effect
    /// object 0x11, `sub_80E1A6A`, for 90 ticks), which the deletion's end
    /// ends (`sub_80E1A86`).
    NaviDeleted,
}

impl HookRole {
    pub const ALL: [HookRole; 3] = [HookRole::FirstBarrier, HookRole::Encased, HookRole::NaviDeleted];

    /// Its name in `rules/roles.luau`'s `hooks`.
    pub fn name(self) -> &'static str {
        match self {
            HookRole::FirstBarrier => "first_barrier",
            HookRole::Encased => "encased",
            HookRole::NaviDeleted => "navi_deleted",
        }
    }

    pub fn named(name: &str) -> Option<HookRole> {
        HookRole::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// The chips the rules name by role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChipRole {
    /// What a zeroed chip field reads: the chip the original's chip 0 is
    /// (a hand's empty selection, the attack's cleared chip, a
    /// non-player's carried chip, a side's special chip never set).
    Zeroed,
    /// The chip a custom-screen button's pick stands as in the hand (EXE6's
    /// BeastOut, `sub_802A00C`): a frontend draws its picture as the
    /// button's (`ButtonView::ChipPicture`).
    ButtonChip,
    /// What an illegal pick counts as in a selection (`getChipID_802A54E`).
    Invalid,
    /// The chips the NaviCust supports' telops name (`sub_80E90FE`).
    Rush,
    Beat,
    Tango,
}

impl ChipRole {
    pub const ALL: [ChipRole; 6] =
        [ChipRole::Zeroed, ChipRole::ButtonChip, ChipRole::Invalid, ChipRole::Rush, ChipRole::Beat, ChipRole::Tango];

    /// Its name in `rules/roles.luau`'s `chips`.
    pub fn name(self) -> &'static str {
        match self {
            ChipRole::Zeroed => "zeroed",
            ChipRole::ButtonChip => "button_chip",
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

/// The roles content filled.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Roles {
    pub actions: BTreeMap<ActionRole, ActionHandle>,
    pub kinds: BTreeMap<KindRole, KindHandle>,
    pub hooks: BTreeMap<HookRole, FnId>,
    pub chips: BTreeMap<ChipRole, ChipHandle>,
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
        self.actions.get(&role).copied()
    }

    /// The action of `role`; a role content hasn't filled is a panic
    /// naming it.
    pub fn action(&self, role: ActionRole) -> ActionHandle {
        self.try_action(role)
            .unwrap_or_else(|| panic!("the role actions.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    /// The chip of `role`, if content filled it.
    pub fn try_chip(&self, role: ChipRole) -> Option<ChipHandle> {
        self.chips.get(&role).copied()
    }

    /// The chip of `role`; a role content hasn't filled is a panic naming
    /// it.
    pub fn chip(&self, role: ChipRole) -> ChipHandle {
        self.try_chip(role)
            .unwrap_or_else(|| panic!("the role chips.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
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
        self.try_hook(role)
            .unwrap_or_else(|| panic!("the role hooks.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    /// The hook of `role`, if content filled it: an optional role's
    /// absence says the game has no such thing (EXE5's `hooks.encased`:
    /// docs/design/exe5-map.md §15.3 item 4).
    pub fn try_hook(&self, role: HookRole) -> Option<FnId> {
        self.hooks.get(&role).copied()
    }

    /// The status of `role`, if content filled it (EXE5 has no
    /// `statuses.ice_freeze`: no freeze).
    pub fn try_status(&self, role: StatusRole) -> Option<StatusHandle> {
        self.statuses.get(&role).copied()
    }

    /// The status of `role`; a role content hasn't filled is a panic naming
    /// it.
    pub fn status(&self, role: StatusRole) -> StatusHandle {
        self.try_status(role)
            .unwrap_or_else(|| panic!("the role statuses.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    /// The effect of `role`; a role content hasn't filled is a panic naming
    /// it (as for the sparks, regions and collision types below).
    pub fn effect(&self, role: EffectRole) -> EffectHandle {
        *self
            .effects
            .get(&role)
            .unwrap_or_else(|| panic!("the role effects.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    pub fn spark(&self, role: SparkRole) -> SparkHandle {
        *self
            .sparks
            .get(&role)
            .unwrap_or_else(|| panic!("the role sparks.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    pub fn region(&self, role: RegionRole) -> RegionHandle {
        *self
            .regions
            .get(&role)
            .unwrap_or_else(|| panic!("the role regions.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    pub fn collision(&self, role: CollisionRole) -> CollisionHandle {
        *self
            .collisions
            .get(&role)
            .unwrap_or_else(|| panic!("the role collision.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    /// The sound, the music, the sprite and the banner of `role`
    /// (likewise).
    /// The sound of a role a game may leave unfilled.
    pub fn try_sound(&self, role: SoundRole) -> Option<SoundId> {
        self.sounds.get(&role).copied()
    }

    pub fn sound(&self, role: SoundRole) -> SoundId {
        *self
            .sounds
            .get(&role)
            .unwrap_or_else(|| panic!("the role sounds.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    pub fn music(&self, role: MusicRole) -> SoundId {
        *self
            .music
            .get(&role)
            .unwrap_or_else(|| panic!("the role music.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    pub fn sprite(&self, role: SpriteRole) -> SpriteId {
        *self
            .sprites
            .get(&role)
            .unwrap_or_else(|| panic!("the role sprites.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    pub fn try_banner(&self, role: BannerRole) -> Option<BannerId> {
        self.banners.get(&role).copied()
    }

    pub fn banner(&self, role: BannerRole) -> BannerId {
        *self
            .banners
            .get(&role)
            .unwrap_or_else(|| panic!("the role banners.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }

    /// The kind of `role`; a role content hasn't filled is a panic naming
    /// it.
    pub fn kind(&self, role: KindRole) -> KindHandle {
        *self
            .kinds
            .get(&role)
            .unwrap_or_else(|| panic!("the role kinds.{} is not filled (the rules' roles, rules/roles.luau)", role.name()))
    }
}
