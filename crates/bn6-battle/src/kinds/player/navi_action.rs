//! What a navi runs, its CurAction (docs/design/content-model-v2.md §7.2):
//! one of the framework's states, one of the ruleset's own actions, or a
//! content action, by handle. None has a number here: the framework's
//! states are in the original's order (state-machine values the traces
//! observe), and the ruleset's actions and content's have keys, by which
//! the validator numbers them (compat actions.toml).

use bn6_content_api::ActionHandle;

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
    /// An action content defines.
    Content(ActionHandle),
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
    pub const ALL: [EngineAction; 6] = [
        EngineAction::Move,
        EngineAction::DimmingChip,
        EngineAction::NaviChip,
        EngineAction::InstantChip,
        EngineAction::FormChange,
        EngineAction::CrossSpecial,
    ];

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

    /// The framework state the original numbers `n` (a lifecycle state's
    /// first action is one).
    pub fn state(n: u8) -> Option<NaviAction> {
        STATES.get(n as usize).copied()
    }

    /// Whether it is an attack (the original's CurAction 0x10 and up): not
    /// one of the framework's states.
    pub fn is_attack(self) -> bool {
        matches!(self, NaviAction::Engine(_) | NaviAction::Content(_))
    }
}
