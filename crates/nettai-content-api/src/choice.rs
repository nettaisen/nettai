//! The choices the engine knows, as content holds them (docs/design/
//! scripting.md §2.1): each a table of values by name (`DragStep.slide`,
//! `ChipClass.mega`), a value opaque to content and compared with `==`.
//! A field, an argument or a definition's field that takes a choice takes
//! its value and refuses its name as a string.
//!
//! A choice is its kind and its member's place in the kind's list. Where
//! the engine keeps the choice as a byte (an object's drag step, a fade),
//! the list is the engine's own list of names in its order, so the place
//! is the byte; elsewhere the binding hands the engine the member's name,
//! the very name the engine took before.
//!
//! The define phase reads a choice in a definition back as
//! [`Data::Choice`](crate::Data::Choice); a loader takes it where it takes
//! the choice and refuses anything else.

use crate::api::{
    ACTOR_TYPES, AttackKind, BARRIER_BEHAVIORS, BARRIER_STATES, BUFFERED_MOVES, BlinkOut, CHARGE_LEVELS, CHARGE_SOURCES,
    DRAG_STEPS, DamageTaken, FightCycle, GAUGE_SPEEDS, Lifecycle, MOVE_DIRECTIONS, OVERLAY_STEPPINGS, ObstacleClass,
    ObstacleCrush, ObstacleHold, ObstaclePush, ObstacleRemoval, PRIMARY_ELEMENTS, PaletteFlashVariant, SLIDE_STEPS,
    SLIDE_TYPES, ScreenFade, Shadow, SideSpecial, SpecialBonus, StepDirection, TakeDamageMode,
};
use crate::host::{SlideAnswer, SlideHow};
use crate::types::Pool;

macro_rules! choices {
    ($($(#[$m:meta])* $kind:ident = $members:expr;)*) => {
        /// A choice's kind: the table content reads its values from.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum ChoiceKind {
            $($(#[$m])* $kind,)*
        }

        impl ChoiceKind {
            pub const ALL: &'static [ChoiceKind] = &[$(ChoiceKind::$kind,)*];

            /// Its table's name (`DragStep`).
            pub fn name(self) -> &'static str {
                match self {
                    $(ChoiceKind::$kind => stringify!($kind),)*
                }
            }

            /// Its members' names, in their order (the engine's byte where
            /// it keeps the choice as one).
            pub fn members(self) -> &'static [&'static str] {
                match self {
                    $(ChoiceKind::$kind => $members,)*
                }
            }
        }
    };
}

choices! {
    /// An object's lifecycle state ([`Lifecycle`]).
    Lifecycle = &Lifecycle::NAMES;
    /// An object's pool.
    Pool = &Pool::NAMES;
    /// How a sprite's shadow part is drawn ([`Shadow`]).
    Shadow = &Shadow::NAMES;
    /// An actor record's type.
    ActorType = &ACTOR_TYPES;
    /// A side's special in progress.
    SideSpecial = SideSpecial::NAMES;
    /// A screen fade content starts (`battle.screen_fade`).
    ScreenFade = ScreenFade::NAMES;
    /// A navi's drag reaction step.
    DragStep = &DRAG_STEPS;
    /// A navi's slide type.
    SlideType = &SLIDE_TYPES;
    /// Where a navi's slide is.
    SlideStep = &SLIDE_STEPS;
    /// What reaches a panel in a panel type's `slide`.
    SlideHow = &SlideHow::NAMES;
    /// A panel type's `slide`'s answer.
    SlideAnswer = &SlideAnswer::NAMES;
    /// The slot an action starts in.
    AttackKind = AttackKind::NAMES;
    /// A step's direction.
    StepDirection = StepDirection::NAMES;
    /// A buffered auto-step's direction.
    BufferedMove = &BUFFERED_MOVES;
    /// Which way a collision region's anchor last moved.
    MoveDirection = &MOVE_DIRECTIONS;
    /// A navi's charge.
    ChargeLevel = &CHARGE_LEVELS;
    /// What a navi charges with.
    ChargeSource = &CHARGE_SOURCES;
    /// `take_damage`'s mode.
    TakeDamageMode = TakeDamageMode::NAMES;
    /// What `take_damage` answers.
    DamageTaken = DamageTaken::NAMES;
    /// The field-object registry's class an obstacle takes.
    ObstacleClass = ObstacleClass::NAMES;
    /// A screen palette flash's variant.
    PaletteFlashVariant = PaletteFlashVariant::NAMES;
    /// The battle's counts of fight ticks.
    FightCycle = FightCycle::NAMES;
    /// The bonuses a side stores for its special chip.
    SpecialBonus = SpecialBonus::NAMES;
    /// How an obstacle takes a hit that would crush it.
    ObstacleCrush = ObstacleCrush::NAMES;
    /// When a dimming holds an obstacle.
    ObstacleHold = ObstacleHold::NAMES;
    /// How an obstacle takes a push.
    ObstaclePush = ObstaclePush::NAMES;
    /// How an obstacle left the field.
    ObstacleRemoval = ObstacleRemoval::NAMES;
    /// Where an obstacle's blink-out is.
    BlinkOut = BlinkOut::NAMES;
    /// How a form's overlay steps.
    FormOverlayStepping = &OVERLAY_STEPPINGS;
    /// A barrier's state.
    BarrierState = &BARRIER_STATES;
    /// How a barrier behaves.
    BarrierBehavior = &BARRIER_BEHAVIORS;
    /// How fast a navi's custom gauge fills.
    GaugeSpeed = &GAUGE_SPEEDS;
    /// A sound of the custom screen's own (`custom.play`).
    ScreenSound = &["pick", "refused", "back", "cancel", "cursor", "description", "program_advance_part", "program_advance", "redeal"];
    /// A fade of the custom screen's (`custom.fade`).
    CustomFade = &["half_out", "half_out_back", "end_to_white", "intro_from_white", "flash", "flash_back"];
    /// A custom-screen button's state.
    ButtonState = &["selectable", "unavailable", "selected"];
    /// How a custom-screen button of the rules' is drawn.
    ButtonView = &["form_offer", "chip_picture"];
    /// How a custom-screen window of the rules' is drawn.
    WindowView = &["form_chosen", "offer_flight", "chip_flight"];
    /// What a controller or a takeover did with the side's navi this tick.
    ControlOutcome = &["nothing", "chip", "buster", "moved", "own_chip"];
    /// What the rules did with a hit's NaviCust bug.
    BugOutcome = &["edited", "spared"];
    /// The engine's own actions, as `navi_action` names them.
    EngineAction = &[
        "entry", "take_control", "deletion", "flinch", "paralysis", "drag", "freeze", "bubble", "idle", "move",
        "dimming_chip", "hand_off_chip", "instant_chip", "form_change", "navi_switch", "switch_knockout",
    ];
    /// A chip's class.
    ChipClass = &["standard", "mega", "giga", "special", "program_advance"];
    /// The trap a chip is.
    TrapKind = &["anti_damage", "anti_sword", "body_guard", "anti_navi", "anti_recovery"];
    /// How a chip's damage is worked out when it isn't a number (the tag of
    /// its `damage` table).
    Formula = &["opponent_hp", "gauge", "gauge_level", "hp_lost", "hp_last_digits", "half_opponent_max_hp", "navi_level", "count"];
    /// What a count formula counts that is no table: the turns before this
    /// one.
    Counted = &["turns_before"];
    /// What a form's charged chip is.
    ChargedChip = &["bonus", "rock_barrage"];
    /// A status's timer.
    StatusKind = &["paralyze", "confuse", "blind", "immobilize", "freeze", "bubble"];
    /// What an identity is.
    IdentityClass = &["player", "field_object", "virus", "navi"];
    /// How large a navi's ice is.
    IceSize = &["small", "medium", "large"];
    /// What an afterimage follows.
    Tether = &["form", "attack"];
    /// A setup problem's kind.
    SetupProblemKind = &[
        "folder_size", "chip_not_allowed", "chip_code", "copies", "mega", "giga", "dark", "regular_memory", "tag_memory",
        "regular_entry", "tag_entries", "tag_regular", "no_tags", "range", "not_taken", "missing", "twice", "budget",
        "off_board", "overlap", "slot", "invalid",
    ];
    /// The roles the engine knows a field by (a view's field's role is the
    /// view and the field, `form_list.offered`: its member is
    /// `form_list_offered`).
    SchemaRole = &[
        "navi", "folder", "regular_chip", "tag_chips", "version", "form_list", "level", "hp", "window_count",
        "form_list.offered", "form_list.count", "form_list.marked", "form_list.chosen", "form_list.chosen_place",
        "form_offer.form", "form_offer.alternate", "form_offer.turns", "form_offer.turn_limit", "offer_flight.step",
        "offer_flight.count", "chip_flight.step", "chip_flight.count", "chip_flight.button",
    ];
    /// What a stage places that is no kind: the side's navi.
    Place = &["navi"];
    /// A charged chip's damage bonus that is no number: the chip's damage
    /// again (the byte 0xFF).
    ChargedBonus = &["again"];
    /// A navi's mood that is no number: Full Synchro (the byte 0xFF).
    Mood = &["full_synchro"];
    /// A count that is no number: it holds (0xFFFF).
    Timer = &["holds"];
    /// A primary element (an element's low four bits, a damage slot): the
    /// number is the member's place.
    PrimaryElement = &PRIMARY_ELEMENTS;
}

/// A choice: its kind, and its member's place in the kind's list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Choice {
    pub kind: ChoiceKind,
    pub index: u8,
}

impl Choice {
    /// The choice of `kind` named `name` (the engine's name: `form_list.offered`).
    pub fn of(kind: ChoiceKind, name: &str) -> Option<Choice> {
        kind.members().iter().position(|m| *m == name).map(|i| Choice { kind, index: i as u8 })
    }

    /// Its member's name, as the engine names it.
    pub fn name(self) -> &'static str {
        self.kind.members()[self.index as usize]
    }

    /// Its member's name in its table (the engine's, a `.` written `_`).
    pub fn member(self) -> String {
        self.name().replace('.', "_")
    }

    /// The name of a choice of `kind`, if `d` is one.
    pub fn name_in(d: &crate::Data, kind: ChoiceKind) -> Option<&'static str> {
        match d {
            crate::Data::Choice(c) if c.kind == kind => Some(c.name()),
            _ => None,
        }
    }
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.kind.name(), self.member())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state's list of variants that are a kind's members is refused: the
    /// field is declared `schema.choice(Kind)`, and holds the kind's values.
    #[test]
    fn a_kinds_members_as_variants_are_its_table() {
        let names = |n: &[&str]| crate::Data::List(n.iter().map(|s| crate::Data::Str(s.to_string())).collect());
        let e = crate::FieldType::from_data(&names(&["none", "a", "b"])).unwrap_err();
        assert!(e.contains("schema.choice(ChargeSource)"), "{e}");
        assert!(crate::FieldType::from_data(&names(&["a", "b"])).is_ok());
    }

    #[test]
    fn members_are_distinct_and_fit_a_byte() {
        for &k in ChoiceKind::ALL {
            let m = k.members();
            assert!(!m.is_empty() && m.len() <= 256, "{}", k.name());
            for (i, a) in m.iter().enumerate() {
                assert!(!m[..i].contains(a), "{}.{a} twice", k.name());
                assert!(!m[..i].iter().any(|b| b.replace('.', "_") == a.replace('.', "_")), "{}.{a}: two members read alike", k.name());
                assert_eq!(Choice::of(k, a).map(|c| c.name()), Some(*a));
            }
        }
    }
}
