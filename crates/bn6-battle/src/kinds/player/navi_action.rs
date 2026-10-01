//! What a navi runs, its CurAction (docs/design/content-model-v2.md §7.2):
//! one of the framework's states, one of the ruleset's own actions, or a
//! content action, by handle. The framework's states keep the original's
//! numbers (state-machine values the traces observe); the ruleset's actions
//! and content's have keys, by which the validator numbers them (compat
//! actions.toml).

use bn6_content_api::ActionHandle;

use crate::content::Defs;

/// What a navi runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NaviAction {
    /// Entering the battle (`sub_80EA6D4`).
    #[default]
    Entry,
    /// Handing control over once it stands.
    TakeControl,
    Deletion,
    Flinch,
    Paralysis,
    Drag,
    Freeze,
    Bubble,
    /// Waiting for input (`sub_80EA734`).
    Idle,
    /// One of the ruleset's own actions.
    Engine(EngineAction),
    /// An action content implements (a definition, or a v1 registration by
    /// number).
    Content(ActionHandle),
    /// An action number nothing implements yet (a pack record's, or the
    /// numeric API's): running it is "not implemented yet".
    Unported(u8),
}

/// A navi's saved lifecycle position (`obj+0x5C`): its state, action and
/// phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviWord {
    pub state: u8,
    pub action: NaviAction,
    pub phase: u8,
    pub phase_init: u8,
}

/// The ruleset's own navi actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EngineAction {
    /// A step (`movement`).
    Move,
    /// A dimming chip's use (`dimming_chip`).
    DimmingChip,
    /// A navi chip's use (`navi_chip`).
    NaviChip,
    /// An instant chip's use (`instant`).
    InstantChip,
    /// A form change, a revert or a Cross change while paused
    /// (`transform`, `cross_change`; the original's CurAction is the
    /// instant chips' then).
    FormChange,
    /// The Cross special's controller (`cross_special`).
    CrossSpecial,
}

impl EngineAction {
    /// Its key, by which the validator numbers it.
    pub fn key(self) -> &'static str {
        match self {
            EngineAction::Move => "engine/move",
            EngineAction::DimmingChip => "engine/dimming-chip",
            EngineAction::NaviChip => "engine/navi-chip",
            EngineAction::InstantChip => "engine/instant-chip",
            EngineAction::FormChange => "engine/form-change",
            EngineAction::CrossSpecial => "engine/cross-special",
        }
    }

    /// Its number in registration by number and the numeric API
    /// (transitional: content model v2 step 13 removes both).
    pub fn number(self) -> u8 {
        use super::actions::{cross_special, dimming_chip, instant, movement, navi_chip};
        match self {
            EngineAction::Move => movement::ACTION,
            EngineAction::DimmingChip => dimming_chip::ACTION,
            EngineAction::NaviChip => navi_chip::ACTION,
            EngineAction::InstantChip | EngineAction::FormChange => instant::ACTION,
            EngineAction::CrossSpecial => cross_special::ACTION,
        }
    }
}

/// The framework's states, in the original's numbering.
const STATES: [NaviAction; 9] = [
    NaviAction::Entry,
    NaviAction::TakeControl,
    NaviAction::Deletion,
    NaviAction::Flinch,
    NaviAction::Paralysis,
    NaviAction::Drag,
    NaviAction::Freeze,
    NaviAction::Bubble,
    NaviAction::Idle,
];

impl NaviAction {
    /// The framework state's number (the original's CurAction), for a
    /// state.
    pub fn state_number(self) -> Option<u8> {
        STATES.iter().position(|&s| s == self).map(|n| n as u8)
    }

    /// Whether it is an attack (the original's CurAction 0x10 and up):
    /// not one of the framework's states, nor a link navi's own action
    /// past idle.
    pub fn is_attack(self, defs: &Defs) -> bool {
        match self {
            NaviAction::Engine(_) => true,
            NaviAction::Content(h) => defs.action(h).number.is_none_or(|n| n >= 0x10),
            NaviAction::Unported(n) => n >= 0x10,
            _ => false,
        }
    }

    /// The action registration by number and the numeric API mean by
    /// number `n` (transitional: content model v2 step 13 removes both).
    pub fn numbered(defs: &Defs, n: u8) -> NaviAction {
        use EngineAction as E;
        if let Some(&s) = STATES.get(n as usize) {
            return s;
        }
        if let Some(h) = defs.action_numbered(n) {
            return NaviAction::Content(h);
        }
        match [E::Move, E::DimmingChip, E::NaviChip, E::InstantChip, E::CrossSpecial].into_iter().find(|e| e.number() == n) {
            Some(e) => NaviAction::Engine(e),
            None => NaviAction::Unported(n),
        }
    }

    /// Its number in registration by number and the numeric API: a
    /// content action's registration's, else [`super::CONTENT_ACTION`]
    /// (transitional: content model v2 step 13 removes both).
    pub fn number(self, defs: &Defs) -> u8 {
        match self {
            NaviAction::Engine(e) => e.number(),
            NaviAction::Content(h) => defs.action(h).number.unwrap_or(super::CONTENT_ACTION),
            NaviAction::Unported(n) => n,
            state => state.state_number().expect("a framework state"),
        }
    }
}
