//! [`ContentHost`]: a runtime holding content (Luau). The engine plans what
//! the runtime binds from its content ([`BindPlan`]: the functions it will
//! call, each a function slot of a definition, and the state layouts); the runtime loads the pack's modules, checks it reads the same
//! definitions the content was made from, and the engine calls the
//! functions by [`FnId`].

use std::fmt;

use crate::api::CoreApi;
use crate::definitions::Definitions;
use crate::registry::Registry;
use crate::assets::AssetNames;
use crate::state::{Schema, StateId, Value};
use crate::types::{ObjectRef, PanelPos};

/// Where a function content implements is: a function slot of a
/// definition (`update` of kind `bomb`, `dimming` of chip `areagrab`;
/// docs/design/content-model-v2.md §3.10). `path` is the field's place in
/// the spec, dot-separated.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FnSource {
    pub registry: Registry,
    pub key: String,
    pub path: String,
}

impl FnSource {
    /// A definition's slot.
    pub fn slot(registry: Registry, key: &str, path: &str) -> FnSource {
        FnSource { registry, key: key.to_string(), path: path.to_string() }
    }
}

impl fmt::Display for FnSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}'s {}", self.registry, self.key, self.path)
    }
}

/// An index into [`BindPlan::functions`]: a function the engine calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FnId(pub u32);

/// What a runtime loads, as the engine planned it from its content: the
/// functions it will call, the state layouts, and the definitions the
/// runtime's define phase must read back. Nothing in the engine or the
/// runtime names a particular kind, action or module.
#[derive(Clone, Debug, Default)]
pub struct BindPlan {
    /// The functions the engine calls, by [`FnId`].
    pub functions: Vec<FnSource>,
    /// Content state layouts, by [`StateId`].
    pub schemas: Vec<Schema>,
    /// What the define phase read when the content was made; a runtime must
    /// read the same.
    pub definitions: Definitions,
    /// The handle of each definition, in `definitions.defs`' order (a
    /// registry's handles also number the engine's own entries, so they are
    /// not the definitions' positions).
    pub handles: Vec<u16>,
    /// The registries' entries that are no definition (the engine's own),
    /// by registry, handle and key: a script reaches them as stand-in
    /// values (`me.kind` of one of the engine's objects).
    pub entries: Vec<(Registry, u16, String)>,
    /// The assets content can name (`asset.sprite("bomb")`).
    pub assets: AssetNames,
}

/// What a runtime loaded that the binding reads: the state layouts.
#[derive(Clone, Debug, Default)]
pub struct Manifest {
    pub schemas: Vec<Schema>,
}

impl Manifest {
    pub fn schema(&self, id: StateId) -> &Schema {
        &self.schemas[id.0 as usize]
    }
}

/// What a dimming chip's controller is spawned with (the user's attack:
/// the registers `sub_80EBD9C` passes to `off_802CCB4[subtype]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DimmingChipSpec {
    /// The attack's element byte (primary | secondary bits).
    pub element: u8,
    /// The damage word: damage | hit parameter << 16.
    pub damage: u32,
    /// The chip (none: an attack without one), and the Atk+ / cross bonus
    /// the telop shows with it.
    pub chip: Option<crate::ChipHandle>,
    pub bonus: u16,
}

/// What a navi chip's navi is spawned with (the registers
/// `sub_80E1880` passes to `off_802CD5C[subtype]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NaviChipSpec {
    /// Where the navi appears (the user's panel when the chip was used).
    pub panel: PanelPos,
    pub element: u8,
    /// The damage word with the bonus added.
    pub damage: u32,
}

/// What an instant chip's effect runs with (the registers `sub_80EC39C`
/// passes to `off_80EC3F0[subtype]`: the user's panel and Z, and its
/// attack).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InstantChipSpec {
    /// The user's panel.
    pub panel: PanelPos,
    /// The attack's element byte.
    pub element: u8,
    /// The user's Z, 16.16.
    pub z: i32,
    /// The damage word (damage | hit parameter << 16) plus the Atk+ /
    /// cross bonus's low byte.
    pub damage: u32,
}

/// What a stage places, as the kind's `place` reads it (the original's
/// spawner for the actor list's entry, `off_80073A0[type]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlaceSpec {
    pub panel: PanelPos,
    /// The entry's side (the one navis take; field objects take their
    /// panel's).
    pub side: u8,
    /// Which of the kind's variants the stage names (a record of the
    /// kind's: a rock's).
    pub variant: Option<crate::RecordHandle>,
    /// The entry's raw argument (what a spawner that ignores it leaves in
    /// a register: the Guardian statue's).
    pub argument: u8,
}

/// A call of a hook, with its arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookCall {
    /// `setup(navi)`: returns the action it starts (an action definition).
    Weapon { navi: ObjectRef },
    /// `dimming_chip(user, spec)`: returns the controller, or nil.
    DimmingChip { user: ObjectRef, spec: DimmingChipSpec },
    /// `navi_chip(user, controller, spec)`: returns the navi, or nil. The
    /// navi calls `navi_chip.navi_left(controller)` when it is done.
    NaviChip { user: ObjectRef, controller: ObjectRef, spec: NaviChipSpec },
    /// `instant_chip(user, spec)`: its result is unused.
    InstantChip { user: ObjectRef, spec: InstantChipSpec },
    /// A kind's `place(spec)`: returns what it placed, or nil.
    Place { spec: PlaceSpec },
    /// A kind's `navi_left(controller)`: the navi chip's navi an object of
    /// the kind brought (BN5's DethPhnx: the last navi chip's) is done. Its
    /// result is unused.
    NaviLeft { controller: ObjectRef },
    /// A role hook the ruleset calls with a navi (`define.roles`'
    /// `hooks`): its result is unused.
    RoleNavi { navi: ObjectRef },
    /// A form's hook the engine calls with the navi in it (its `reset`:
    /// what else the status reset does in the form, BN5's souls'). Its
    /// result is unused.
    FormNavi { navi: ObjectRef },
    /// `encased(obstacle, ice, class)` (`sub_801813A`'s end): put what an
    /// obstacle encased in ice (`ice`) or a bubble becomes on its panel; its
    /// field-object registry class (none: it wasn't registered). Its result
    /// is unused.
    RoleEncased { obstacle: ObjectRef, ice: bool, class: Option<u8> },
    /// A hook of a system of side `side`'s ruleset (docs/design/
    /// rules-in-luau.md §4.1), the system in place `slot` of the ruleset's
    /// list: while it runs, `system.state()` is that system's state of that
    /// side. It is called with the side, then the navi, the chip and the
    /// weapon the hook is about, where it has them (nil in between). Its
    /// result is the hook's.
    System {
        side: u8,
        slot: u8,
        hook: SystemHook,
        navi: Option<ObjectRef>,
        chip: Option<crate::ChipHandle>,
        weapon: Option<crate::WeaponHandle>,
    },
}

/// Which hook of a system is called (docs/design/rules-in-luau.md §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemHook {
    /// `round_setup(side)`: once per side as the round is set up
    /// (`Battle::new`), before anything reads the side's navi stats: a
    /// system may change them (BN6's patch cards). Its result is unused.
    RoundSetup,
    /// `round_start(side)`: once per side, after the navis spawn. Its result
    /// is unused.
    RoundStart,
    /// `turn_check(side)`: the turn-start sequencer's check of a side that
    /// isn't changing form (`sub_801486C`), or that asks for a navi switch:
    /// a form whose time ran out asks to revert.
    TurnCheck,
    /// `turn_started(side)`: the turn starts, after the sequencer
    /// (`sub_800840C`'s end): a turn in a form is spent.
    TurnStarted,
    /// `custom_requested(side)`: a custom screen was asked for in the
    /// middle of the fight (`sub_8008452`), before the reversions.
    CustomRequested,
    /// `custom_closed(side)`: both results are in and the fight resumes
    /// (`sub_8009338`).
    CustomClosed,
    /// `navi_intake(side, navi)`: each tick of the fight, in the navi's
    /// intake (`sub_801AC6C`) after the standing effects: BN5's light and
    /// dark system clears the holy panel a dark MegaMan stands on
    /// (0x08017136). Its result is unused.
    NaviIntake,
    /// `chip_check(side, navi, chip)`: a chip's use is prepared (the end of
    /// `sub_80127C0`, where BN5's 0x080100E6 checks it): nil lets it be
    /// used; a chip is what the navi uses instead (BN5's light and dark
    /// system refuses a chip its MegaMan may not use: 0x08010118). The
    /// first system that answers decides.
    ChipCheck,
    /// `chip_cost(side, navi, chip)`: earlier in a chip use's preparation,
    /// before the hand bonus's panel is spent (BN5's 0x08010030: a dark
    /// chip's use, which may fizzle, and the chip's cost, 0x0802C934). It
    /// answers as `chip_check` does; a chip in the use's place skips the
    /// rest of the preparation, `chip_check` too.
    ChipCost,
    /// `chip_substitute(side, navi, chip)`: a chip's use is prepared, before
    /// its record is loaded (`sub_80127C0`'s `sub_8010D58`): a chip is what
    /// the navi uses in its place, with its own record, damage and bonus
    /// (BN6's dark chips' substitute, with no bug frag left); nil, the chip.
    /// The first system that answers decides.
    ChipSubstitute,
    /// `chip_prepared(side, navi, chip)`: a chip's use was prepared
    /// (`sub_80127C0`, on each of its paths: a use, the wrapper's chain, the
    /// counter cut-in's), its substitute taken: `chip` the chip it uses.
    /// BN6's dark chips worsen the user's HP bug (`sub_800B79A`). Its result
    /// is unused.
    ChipPrepared,
    /// `chip_used(side, navi, chip, weapon)`: a chip's use started
    /// (`sub_800FB54`, its action set): `chip` the chip it reads (the
    /// zeroed chip for the empty hand), `weapon` the form's weapon run
    /// instead of it (a charged use), else nil. BN6's beast system decides
    /// whether it runs inside the rush (the attack's `wrapped`). Its result
    /// is unused.
    ChipUsed,
    /// `controller(side, navi)`: each tick of the idle action of a navi whose
    /// form is `controlled` (BN6's Beast Over: `sub_802D322`), in place of
    /// the player's decisions: "nothing", "chip" (a chip's use started),
    /// "buster" (the buster is to fire) or "moved" (a step started); the
    /// framework carries it out as idle does. The first system that
    /// answers decides.
    Controller,
    /// `takeover_requested(side, navi)`: idle finds the navi's takeover
    /// request (`sub_802E4E4`; BN6's DarkInvs asks for its Cross
    /// special): a system starts it (`battle.take_over`). Its result is
    /// unused.
    TakeoverRequested,
    /// `takeover(side, navi)`: each tick of the idle action of a navi whose
    /// side's takeover runs (`sub_802D4C6`, the Cross special), in place
    /// of the player's decisions: what it did, as `controller` answers,
    /// or "own_chip" (an attack of its own started: the attack's chip). The
    /// system ends it (`battle.end_takeover`). The first system that
    /// answers decides.
    Takeover,
    /// `countered(side, victim)`: side `side`'s navi landed a counter on
    /// `victim` (`sub_801A200`): BN6's emotion system gives Full Synchro
    /// unless the victim's mood is held. Its result is unused.
    Countered,
    /// `navi_tick(side, navi)`: each unpaused tick, after the navi's input
    /// (`sub_8013DA0`'s place), for a navi a system asked it for (its
    /// `ticked`): BN6's NaviCust emotion-swing bug. Its result is unused.
    NaviTick,
    /// `starting_mood(side)`: the mood the side's navi starts the round with
    /// (`sub_8015C2C`'s 0x80, set where `sub_8013892` sets it): BN5's light
    /// and dark system's by the light/dark value (0x0801283A). The first
    /// system that answers decides; none, 0x80.
    StartingMood,
    /// `navi_palette(side, navi)`: each tick, the sprite palette of the
    /// side's navi of a player's kind (presentation; `sub_80100EC`'s
    /// `sub_801002C`, BN5's 0x0800DD94: its light and dark system's). The
    /// first system that answers decides; none, the framework's (BN6's).
    NaviPalette,
    /// `navi_bug(side, navi)`: before the navi takes its hit's NaviCust bug
    /// (`sub_80139F6`, BN5's 0x0801103E): a system may change the bug
    /// (the collision's `inflicted_bugs`: BN5's light and dark system turns
    /// its codes 0xFD and 0xFC into an HP drain or none), or answer true:
    /// the bug and the weapons' reload are skipped (BN5's, for hit flag
    /// 0x400 on a light/dark value of 1000 or more). The first system that
    /// answers true decides.
    NaviBug,
    /// `custom.hand_size(side)`: how many chips the side's custom screen
    /// deals (`sub_802A40C`; BN5's `sub_802A49C`), asked as it opens. The
    /// first system that answers decides; none answering, the framework's
    /// rule (the custom level, NumbrOpn and the hand-shrink bug).
    CustomHandSize,
    /// `custom.open(side)`: the side's custom screen opens, before its hand
    /// size and its layout (BN6's: ChargeCross's screens counted, the
    /// round's first screen forgetting the round's Beast Out). Its result
    /// is unused.
    CustomOpen,
    /// `custom.chip_picked(side, chip)`: a chip of the hand was picked (BN6's
    /// BeastOut chip starts its animation, `sub_802A00C`). Its result is
    /// unused.
    CustomChipPicked,
    /// `custom.chip_taken_back(side, chip)`: B took a chip's pick back
    /// (BN6's BeastOut chip: its face goes, `sub_802A0EC`). Its result is
    /// unused.
    CustomChipTakenBack,
    /// `custom.confirmed(side)`: OK was pressed and the hand is built (the
    /// round's Beast Out or Cross noted). Its result is unused.
    CustomConfirmed,
    /// `custom.keys(side)`: choosing chips, on a tick a key repeats or is
    /// pressed, before the screen's own keys (`custom.pressed`,
    /// `custom.repeated`): whether the system took the tick's keys (BN6's
    /// UP opening the Cross window, `sub_8028B74`). The first system that
    /// answers true takes them.
    CustomKeys,
    /// `custom.take_back(side)`: B with nothing picked (`sub_8029032`):
    /// whether the system took something back (BN6's Cross chosen). The
    /// first system that answers true did; none, and B is refused.
    CustomTakeBack,
    /// A system's window's `update(side)`, each tick it is up: whether it
    /// stays up (`custom.window_tick` counts its ticks from 1).
    WindowUpdate,
    /// A button's `taken_back(side)`: B took its pick back (BN6's Beast
    /// Out: its face goes).
    ButtonTakenBack,
    /// A system's custom-screen button's `shown(side)` (§4.4), as the
    /// screen opens: whether it is on the screen. (Not in `hooks`: each
    /// button names its own functions.)
    ButtonShown,
    /// A button's `state(side)`, at the open and after each pick unless it
    /// is selected: "selectable" or "unavailable". A button without one is
    /// selectable at the open and left as it is.
    ButtonState,
    /// A button's `pressed(side)`: A on it.
    ButtonPressed,
    /// `form_reverted(side, navi)`: the framework reverts the navi to its
    /// base form, its form not yet changed (`sub_80158CC`): BN6's spends a
    /// Beast Out and exhausts a Beast Over. Its result is unused.
    FormReverted,
    /// `folder_check(side)`: a tool asks whether a folder keeps the side's
    /// game's folder rules (`Battle::check_folder`, not the simulation):
    /// the folder is `battle.checked_folder()`, and each rule it breaks is
    /// `battle.folder_problem(rule, text)`. Its result is unused.
    FolderCheck,
}

impl SystemHook {
    /// The hook's name in a system's `hooks` table.
    pub fn name(self) -> &'static str {
        match self {
            SystemHook::RoundSetup => "round_setup",
            SystemHook::RoundStart => "round_start",
            SystemHook::TurnCheck => "turn_check",
            SystemHook::TurnStarted => "turn_started",
            SystemHook::CustomRequested => "custom_requested",
            SystemHook::CustomClosed => "custom_closed",
            SystemHook::FolderCheck => "folder_check",
            SystemHook::NaviIntake => "navi_intake",
            SystemHook::ChipCheck => "chip_check",
            SystemHook::ChipCost => "chip_cost",
            SystemHook::ChipSubstitute => "chip_substitute",
            SystemHook::ChipPrepared => "chip_prepared",
            SystemHook::ChipUsed => "chip_used",
            SystemHook::Controller => "controller",
            SystemHook::FormReverted => "form_reverted",
            SystemHook::TakeoverRequested => "takeover_requested",
            SystemHook::Countered => "countered",
            SystemHook::NaviTick => "navi_tick",
            SystemHook::StartingMood => "starting_mood",
            SystemHook::NaviPalette => "navi_palette",
            SystemHook::NaviBug => "navi_bug",
            SystemHook::CustomHandSize => "custom.hand_size",
            SystemHook::CustomOpen => "custom.open",
            SystemHook::CustomConfirmed => "custom.confirmed",
            SystemHook::CustomKeys => "custom.keys",
            SystemHook::CustomTakeBack => "custom.take_back",
            SystemHook::CustomChipPicked => "custom.chip_picked",
            SystemHook::CustomChipTakenBack => "custom.chip_taken_back",
            SystemHook::WindowUpdate => "window.update",
            SystemHook::ButtonTakenBack => "button.taken_back",
            SystemHook::ButtonShown => "button.shown",
            SystemHook::ButtonState => "button.state",
            SystemHook::ButtonPressed => "button.pressed",
            SystemHook::Takeover => "takeover",
        }
    }

    pub const ALL: [SystemHook; 34] = [
        SystemHook::RoundSetup,
        SystemHook::RoundStart,
        SystemHook::TurnCheck,
        SystemHook::TurnStarted,
        SystemHook::CustomRequested,
        SystemHook::CustomClosed,
        SystemHook::FolderCheck,
        SystemHook::NaviIntake,
        SystemHook::ChipCheck,
        SystemHook::ChipCost,
        SystemHook::ChipSubstitute,
        SystemHook::ChipPrepared,
        SystemHook::ChipUsed,
        SystemHook::Controller,
        SystemHook::FormReverted,
        SystemHook::TakeoverRequested,
        SystemHook::Takeover,
        SystemHook::Countered,
        SystemHook::NaviTick,
        SystemHook::StartingMood,
        SystemHook::NaviPalette,
        SystemHook::NaviBug,
        SystemHook::CustomHandSize,
        SystemHook::ButtonShown,
        SystemHook::ButtonState,
        SystemHook::ButtonPressed,
        SystemHook::CustomOpen,
        SystemHook::CustomConfirmed,
        SystemHook::WindowUpdate,
        SystemHook::ButtonTakenBack,
        SystemHook::CustomChipPicked,
        SystemHook::CustomChipTakenBack,
        SystemHook::CustomKeys,
        SystemHook::CustomTakeBack,
    ];
}

/// A content error: a bug in the content, or a script breaking the
/// runtime's rules. The engine stops the battle on one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentError {
    pub message: String,
}

impl ContentError {
    pub fn new(message: impl Into<String>) -> ContentError {
        ContentError { message: message.into() }
    }
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ContentError {}

/// A content runtime. Its functions must be deterministic functions of the
/// engine state they read through `api`: the host keeps nothing between
/// calls that affects the battle (so a battle snapshot doesn't include
/// it).
pub trait ContentHost {
    /// A short name for messages ("luau").
    fn runtime(&self) -> &str;
    fn manifest(&self) -> &Manifest;
    /// One tick of object `me`: function `f` is its kind's `update`.
    fn update_object(&self, api: &mut dyn CoreApi, f: FnId, me: ObjectRef) -> Result<(), ContentError>;
    /// One tick of an action for the navi `me`: function `f` is the
    /// action's `update`, and its second argument is the attack state as a
    /// state of layout `state`.
    /// `system`: the side and place in its ruleset of the system the
    /// action is one of (`define.system { actions = ... }`), whose state
    /// the action reaches (docs/design/rules-in-luau.md §5.3).
    fn update_action(
        &self,
        api: &mut dyn CoreApi,
        f: FnId,
        me: ObjectRef,
        state: StateId,
        system: Option<(u8, u8)>,
    ) -> Result<(), ContentError>;
    /// Call function `f` for a hook.
    fn call_hook(&self, api: &mut dyn CoreApi, f: FnId, call: HookCall) -> Result<Value, ContentError>;
}
