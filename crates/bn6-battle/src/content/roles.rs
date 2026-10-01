//! What the ruleset needs from content by role (docs/design/
//! content-model-v2.md §7.4): `define.roles { ... }` in `rules/roles.luau`,
//! once. The ruleset starts and recognizes the actions and kinds it needs
//! through these, never by the original's numbers or by content's keys.
//!
//! A role names a definition, or, while what it needs is still a v1
//! registration, that registration through the transitional `legacy`
//! marker (`{ legacy = { action = 0x49 } }`, `{ legacy = { kind = "rock" } }`;
//! counted by the ratchet). A legacy action number nothing implements yet
//! leaves the role unported: starting it is "not implemented yet", as the
//! number was. A role content hasn't filled is an error where the ruleset
//! needs it.

use std::collections::BTreeMap;

use bn6_content_api::{ActionHandle, FnId, KindHandle};

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
    /// The rock a stage's actor list places (entry type 8; its `place`).
    Rock,
    /// The boulder a stage's actor list places (entry type 3; its
    /// `place`), one of the field's stage objects.
    Boulder,
    /// The Guardian statue a stage's actor list places (entry type 9; its
    /// `place`).
    Statue,
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
    pub const ALL: [KindRole; 7] = [
        KindRole::Rock,
        KindRole::Boulder,
        KindRole::Statue,
        KindRole::AbsorbedObstacle,
        KindRole::FallingRock,
        KindRole::Support,
        KindRole::AntiRecovery,
    ];

    /// Its name in `rules/roles.luau`'s `kinds`.
    pub fn name(self) -> &'static str {
        match self {
            KindRole::Rock => "rock",
            KindRole::Boulder => "boulder",
            KindRole::Statue => "statue",
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

/// The functions the ruleset calls by role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HookRole {
    /// `(navi)`: the NaviCust FirstBarrier's barrier and its visual
    /// (`sub_8013892`: `sub_801A7CC` with the navi stat, which the game's
    /// program sets to 1, and `sub_80E0D98`).
    FirstBarrier,
}

impl HookRole {
    pub const ALL: [HookRole; 1] = [HookRole::FirstBarrier];

    /// Its name in `rules/roles.luau`'s `hooks`.
    pub fn name(self) -> &'static str {
        match self {
            HookRole::FirstBarrier => "first_barrier",
        }
    }

    pub fn named(name: &str) -> Option<HookRole> {
        HookRole::ALL.into_iter().find(|r| r.name() == name)
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
