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
    /// The HP the entry gives it, if it states one.
    pub hp: Option<u16>,
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
    /// the kind brought (EXE5's DethPhnx: the last navi chip's) is done. Its
    /// result is unused.
    NaviLeft { controller: ObjectRef },
    /// A role hook the rules call with a navi (the roles' `hooks`):
    /// its result is unused.
    RoleNavi { navi: ObjectRef },
    /// A form's hook the engine calls with the navi in it (its `reset`:
    /// what else the status reset does in the form, EXE5's souls'), and a
    /// chip's `setup`, called with the navi whose use of it is prepared.
    /// Its result is unused.
    FormNavi { navi: ObjectRef },
    /// `encased(obstacle, ice, class)` (`sub_801813A`'s end): put what an
    /// obstacle encased in ice (`ice`) or a bubble becomes on its panel; its
    /// field-object registry class (none: it wasn't registered). Its result
    /// is unused.
    RoleEncased { obstacle: ObjectRef, ice: bool, class: Option<u8> },
    /// A definition's function of a side, which the round's setup asks
    /// once for each side after its rules' `round_setup` (a chip's
    /// `damage` that goes by the side, a navi's `chip_bonus.damage`,
    /// `charged_chips.when` and `fire_charge`: what the side's level gives,
    /// as its game's rules read it): called with the side, then the chip
    /// whose damage it is (nil for a navi's); its result is a number, a
    /// flag or nil, which the battle keeps for the round
    /// (`Battle::given`).
    Given { side: u8, chip: Option<crate::ChipHandle> },
    /// A hook of the game's rules for side `side` (docs/design/
    /// rules-in-luau.md §4.1): while it runs, `rules.state()` is the rules'
    /// state of that side. It is called with the side, then the navi, the
    /// chip and the weapon the hook is about, where it has them (nil in
    /// between). Its result is the hook's.
    Rules {
        side: u8,
        hook: RulesHook,
        navi: Option<ObjectRef>,
        chip: Option<crate::ChipHandle>,
        weapon: Option<crate::WeaponHandle>,
    },
    /// A hook of a panel type's (its rules' `panels.types.<name>.<hook>`):
    /// what the type does where the engine meets it, with the body it
    /// meets there (docs/design/rules-in-luau.md, "Panels into Luau").
    Panel(PanelCall),
}

/// Which hook of a panel type's is called: where the engine meets the type,
/// in the original's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelHook {
    /// `burn(body, player)`: at the start of a navi's hit intake (EXE5's
    /// 0x080178EC, EXE4's 0x08013128 for a player, 0x0801309E for another
    /// body), and an object's `burn_on_panel`: the body stands on the type.
    /// `player`: the body is a player's navi. Its result is unused.
    Burn,
    /// `stand(body)`: each tick of the fight, in a navi's intake where the
    /// standing effects run (`sub_801A186`), not paused or dimmed, while
    /// the body has a region: the body stands on the type. (Without the
    /// hook the body's standing count goes to 0.) Its result is unused.
    Stand,
    /// `rest(body)`: a navi at rest on the type (`sub_801A36A`'s first
    /// test: not dragged or moving, its slide cooldown not just out): true
    /// says it handled the body, and the move's end waits (EXE6's roads).
    Rest,
    /// `move_end(body)`: a navi's move ended on the type (`sub_801A36A`
    /// consumed its move-complete flag, or its slide cooldown just ran
    /// out). Its result is unused.
    MoveEnd,
    /// `slide(body, how)`: a slide (a navi's: `sub_8016730`), a drag
    /// (`sub_80178D4`'s arrival) or an obstacle's slide reaches the type:
    /// "on" (one panel further), "carry" (a navi's slide is carried the
    /// type's way: roads, EXE5's magnet), "stop", or nil (as any panel).
    Slide,
    /// `hit(body, element)`: in the hit kernel, an object that isn't a body
    /// (an attack, of `element`) touches a body standing on the type, the
    /// body not invulnerable and the object's hit flags' bit 0 clear (EXE6's
    /// ice: an aqua attack freezes the body and the panel turns normal). Its
    /// result is unused.
    Hit,
    /// `tick(x, y)`: each panel update (`sub_800C380`), for a panel of the
    /// type that isn't missing, broken or cracked (EXE6's volcano erupts,
    /// EXE4's pitfall crumbles); a type without one that expires counts down
    /// its `expires`. Its result is unused.
    Tick,
    /// `changed(x, y)`: the panel became the type (`_object_setPanelType`,
    /// EXE4's 0x08009DC4: the pitfall counts at once). Its result is unused.
    Changed,
    /// `start(x, y)`: the round's field starts with the panel of the type
    /// (EXE4's 0x08009120: a stage's pitfall waits, armed). Its result is
    /// unused.
    Start,
}

impl PanelHook {
    /// Every hook, in order.
    pub const ALL: [PanelHook; 9] = [
        PanelHook::Burn,
        PanelHook::Stand,
        PanelHook::Rest,
        PanelHook::MoveEnd,
        PanelHook::Slide,
        PanelHook::Hit,
        PanelHook::Tick,
        PanelHook::Changed,
        PanelHook::Start,
    ];

    /// The hook's name in a panel type's table.
    pub fn name(self) -> &'static str {
        match self {
            PanelHook::Burn => "burn",
            PanelHook::Stand => "stand",
            PanelHook::Rest => "rest",
            PanelHook::MoveEnd => "move_end",
            PanelHook::Slide => "slide",
            PanelHook::Hit => "hit",
            PanelHook::Tick => "tick",
            PanelHook::Changed => "changed",
            PanelHook::Start => "start",
        }
    }
}

/// A panel hook's call, with its arguments: the body it meets, or the
/// panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelCall {
    Burn { body: ObjectRef, player: bool },
    Stand { body: ObjectRef },
    Rest { body: ObjectRef },
    MoveEnd { body: ObjectRef },
    Slide { body: ObjectRef, how: SlideHow },
    Hit { body: ObjectRef, element: u8 },
    Tick { x: u8, y: u8 },
    Changed { x: u8, y: u8 },
    Start { x: u8, y: u8 },
}

impl PanelCall {
    /// The hook it calls.
    pub fn hook(self) -> PanelHook {
        match self {
            PanelCall::Burn { .. } => PanelHook::Burn,
            PanelCall::Stand { .. } => PanelHook::Stand,
            PanelCall::Rest { .. } => PanelHook::Rest,
            PanelCall::MoveEnd { .. } => PanelHook::MoveEnd,
            PanelCall::Slide { .. } => PanelHook::Slide,
            PanelCall::Hit { .. } => PanelHook::Hit,
            PanelCall::Tick { .. } => PanelHook::Tick,
            PanelCall::Changed { .. } => PanelHook::Changed,
            PanelCall::Start { .. } => PanelHook::Start,
        }
    }
}

/// What reaches a panel in a panel type's `slide`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlideHow {
    /// A navi's slide (a push's, ice's, a road's).
    Slide,
    /// A navi's drag.
    Drag,
    /// An obstacle's slide.
    Obstacle,
}

impl SlideHow {
    pub fn name(self) -> &'static str {
        match self {
            SlideHow::Slide => "slide",
            SlideHow::Drag => "drag",
            SlideHow::Obstacle => "obstacle",
        }
    }
}

/// A panel type's `slide`'s answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlideAnswer {
    /// One panel further.
    On,
    /// Carried the type's way (a navi's slide).
    Carry,
    Stop,
}

impl SlideAnswer {
    /// Every answer, by the number a hook's result carries (1 up).
    pub const ALL: [SlideAnswer; 3] = [SlideAnswer::On, SlideAnswer::Carry, SlideAnswer::Stop];

    pub fn name(self) -> &'static str {
        match self {
            SlideAnswer::On => "on",
            SlideAnswer::Carry => "carry",
            SlideAnswer::Stop => "stop",
        }
    }
}

/// Which hook of the rules is called (docs/design/rules-in-luau.md §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RulesHook {
    /// `round_setup(side)`: once per side as the round is set up
    /// (`Battle::new`), before anything reads the side's navi stats: the
    /// rules may change them (EXE6's patch cards). Its result is unused.
    RoundSetup,
    /// `round_start(side)`: once per side, after the navis spawn. Its result
    /// is unused.
    RoundStart,
    /// `turn_opened(side)`: the turn-start sequencer's check begins
    /// (`sub_801486C`'s entry; EXE5's 0x08011DDC), each side in turn, before
    /// either side's `turn_check` or change of form: a side's request for
    /// the turn is read (EXE5's ColonelSoul takes its arm chip, 0x080124AE).
    /// Its result is unused.
    TurnOpened,
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
    /// `custom_result(side)`: both results are in and each hand with chips
    /// is installed (`sub_800B3D8`), the tick before the fight resumes, each
    /// side in turn: EXE4's dark chips cost their user as its hand takes
    /// them (0x0801EA1E). Its result is unused.
    CustomResult,
    /// `custom_closed(side)`: both results are in and the fight resumes
    /// (`sub_8009338`).
    CustomClosed,
    /// `navi_intake(side, navi)`: each tick of the fight, in the navi's
    /// intake (`sub_801AC6C`) after the standing effects: EXE5's light and
    /// dark part clears the holy panel a dark MegaMan stands on
    /// (0x08017136). Its result is unused.
    NaviIntake,
    /// `chip_check(side, navi, chip)`: a chip's use is prepared (the end of
    /// `sub_80127C0`, where EXE5's 0x080100E6 checks it): nil lets it be
    /// used; a chip is what the navi uses instead (EXE5's light and dark
    /// part refuses a chip its MegaMan may not use: 0x08010118). The
    /// first part that answers decides.
    ChipCheck,
    /// `chip_cost(side, navi, chip)`: earlier in a chip use's preparation,
    /// before the hand bonus's panel is spent (EXE5's 0x08010030: a dark
    /// chip's use, which may fizzle, and the chip's cost, 0x0802C934). It
    /// answers as `chip_check` does; a chip in the use's place skips the
    /// rest of the preparation, `chip_check` too.
    ChipCost,
    /// `chip_substitute(side, navi, chip)`: a chip's use is prepared, before
    /// its record is loaded (`sub_80127C0`'s `sub_8010D58`): a chip is what
    /// the navi uses in its place, with its own record, damage and bonus
    /// (EXE6's dark chips' substitute, with no bug frag left); nil, the chip.
    /// The first part that answers decides.
    ChipSubstitute,
    /// `chip_prepared(side, navi, chip)`: a chip's use was prepared
    /// (`sub_80127C0`, on each of its paths: a use, the wrapper's chain, the
    /// counter cut-in's), its substitute taken: `chip` the chip it uses.
    /// EXE6's dark chips worsen the user's HP bug (`sub_800B79A`). Its result
    /// is unused.
    ChipPrepared,
    /// `chip_used(side, navi, chip, weapon)`: a chip's use started
    /// (`sub_800FB54`, its action set): `chip` the chip it reads (the
    /// zeroed chip for the empty hand), `weapon` the form's weapon run
    /// instead of it (a charged use), else nil. EXE6's rules/beast decides
    /// whether it runs inside the rush (the attack's `wrapped`). Its result
    /// is unused.
    ChipUsed,
    /// `controller(side, navi)`: each tick of the idle action of a navi whose
    /// form is `controlled` (EXE6's Beast Over: `sub_802D322`), in place of
    /// the player's decisions: "nothing", "chip" (a chip's use started),
    /// "buster" (the buster is to fire) or "moved" (a step started); the
    /// framework carries it out as idle does. The first part that
    /// answers decides.
    Controller,
    /// `takeover_requested(side, navi)`: idle finds the navi's takeover
    /// request (`sub_802E4E4`; EXE6's DarkInvs asks for its Cross
    /// special): the rules start it (`battle.take_over`). Its result is
    /// unused.
    TakeoverRequested,
    /// `takeover(side, navi)`: each tick of the idle action of a navi whose
    /// side's takeover runs (`sub_802D4C6`, the Cross special), in place
    /// of the player's decisions: what it did, as `controller` answers,
    /// or "own_chip" (an attack of its own started: the attack's chip). The
    /// rules end it (`battle.end_takeover`). The first part that
    /// answers decides.
    Takeover,
    /// `countered(side, victim)`: side `side`'s navi landed a counter on
    /// `victim` (`sub_801A200`): EXE6's rules/emotion gives Full Synchro
    /// unless the victim's mood is held. Its result is unused.
    Countered,
    /// `navi_tick(side, navi)`: each unpaused tick, after the navi's input
    /// (`sub_8013DA0`'s place), for a navi the rules asked it for (its
    /// `ticked`): EXE6's NaviCust emotion-swing bug. Its result is unused.
    NaviTick,
    /// `starting_mood(side)`: the mood the side's navi starts the round with
    /// (`sub_8015C2C`'s 0x80, set where `sub_8013892` sets it): EXE5's light
    /// and dark part's by the light/dark value (0x0801283A). The first
    /// part that answers decides; none, 0x80.
    StartingMood,
    /// `navi_palette(side, navi)`: each tick, the sprite palette of the
    /// side's navi of a player's kind (presentation; `sub_80100EC`'s
    /// `sub_801002C`, EXE5's 0x0800DD94: rules/light_dark's). The
    /// first part that answers decides; none, the framework's (EXE6's).
    NaviPalette,
    /// `charge_released(side, navi)`: a full B charge is released while the
    /// rules' own B charge is armed (`b_charge_time`; EXE5's Chaos Unison,
    /// 0x0801086E): the rules note what it does (EXE5's: by its cycle's
    /// window), which idle has them start (`release_taken`). Its result is
    /// unused.
    ChargeReleased,
    /// `release_taken(side, navi)`: idle takes that release (EXE5's
    /// 0x080F034E and 0x080F0382), the navi out of idle: the rules start
    /// what it does (EXE5's chaos weapon, or its failure). Its result is
    /// unused.
    ReleaseTaken,
    /// `hit_bug(side, navi)`: any navi (a player's or not) takes the bugs
    /// its hit's code brings every navi (EXE6's `sub_801A6B4` and
    /// `sub_801A720`: the HP bug's level up, and paralysis and blindness),
    /// before its status and damage; asked only when a hit brought a code.
    /// Its result is unused.
    HitBug,
    /// `bug_mark(side, navi)`: whether the navi's hit's bug shows the HP
    /// bug's marker (`sub_801A4A6`), asked as its hits' effects show and
    /// only when a hit brought a code.
    BugMark,
    /// `navi_flinched(side, navi)`: a player's navi's hits ask it to flinch
    /// or be dragged (its requests 0x104: `sub_8013F1E`, EXE5's
    /// 0x0801156E), before the hit's status takes: the NaviCust's hit bug
    /// (the `hit_status` stat), behind the game's gate (EXE6's: a hit,
    /// damage, once a hit sequence), puts its status there or raises the HP
    /// bug. Its result is unused.
    NaviFlinched,
    /// `navi_bug(side, navi)`: the navi takes its hit's NaviCust bug
    /// (`sub_80139F6`, EXE5's 0x0801103E: the collision's `inflicted_bugs`,
    /// its code and argument): the game's table of its codes, which writes
    /// the stat a code names, and answers "edited" (its abilities and form
    /// flags come back), nil (nothing of the stats changed), or "spared"
    /// (nothing at all: EXE5's for hit flag 0x400 on a light/dark value of
    /// 1000 or more, and a drain that wouldn't rise); but "spared", its
    /// weapons are reloaded after.
    NaviBug,
    /// `hp_emptied(side, navi)`: a loss of HP brought the side's navi (an
    /// object with actor data) to 0, by a game whose HP loss asks
    /// (`status.hp_loss = "gauge"`: EXE5's `object_subtractHP`,
    /// 0x0800C6E0, which calls 0x0802C16C): the rules may hold it (EXE5's
    /// last stand: its HP back to 1, and the volley asked for), and answer
    /// true: the hit shows (white, its sounds: EXE5's
    /// `applyDamageToPlayer`, 0x080185A2, by the register the check
    /// leaves). None answering true: it doesn't.
    HpEmptied,
    /// `obstacle_reaction(side, obstacle)`: an obstacle's reaction (after
    /// the damage and the crushing hits) leaves it standing, outside the
    /// dimming and past its first action, while a side is armed
    /// (`obstacle.arm_conversion`): the rules may turn it into something
    /// else and break it (its HP and max HP to 0: EXE5's ColonelSoul army,
    /// 0x080CAB02 from its four obstacle reactions). `side` is the
    /// obstacle's. Its result is unused.
    ObstacleReaction,
    /// `custom.hand_size(side)`: how many chips the side's custom screen
    /// deals (`sub_802A40C`; EXE5's `sub_802A49C`), asked as it opens. The
    /// first part that answers decides; none answering, the framework's
    /// rule (the custom level and the hand-shrink bug).
    CustomHandSize,
    /// `custom.deal(side)`: the side's custom screen deals, before
    /// `custom.open`: the folder as the last screen left it (the chips used
    /// still leave their places empty: `custom.folder`), with the
    /// framework's hand size (`custom.hand_size`); EXE5's custom screen
    /// opening (0x08022C5C) offers a worried or dark MegaMan a dark chip
    /// there (0x08025114, rules/light_dark's). Its result is
    /// unused.
    CustomDeal,
    /// `custom.open(side)`: the side's custom screen opens, before its hand
    /// size and its layout (EXE6's: ChargeCross's screens counted, the
    /// round's first screen forgetting the round's Beast Out). Its result
    /// is unused.
    CustomOpen,
    /// `custom.chip_picked(side, chip)`: a chip of the hand was picked (EXE6's
    /// BeastOut chip starts its animation, `sub_802A00C`). Its result is
    /// unused.
    CustomChipPicked,
    /// `custom.chip_taken_back(side, chip)`: B took a chip's pick back
    /// (EXE6's BeastOut chip: its face goes, `sub_802A0EC`). Its result is
    /// unused.
    CustomChipTakenBack,
    /// `custom.confirmed(side)`: OK was pressed and the hand is built (the
    /// round's Beast Out or Cross noted). Its result is unused.
    CustomConfirmed,
    /// `custom.keys(side)`: choosing chips, on a tick a key repeats or is
    /// pressed, before the screen's own keys (`custom.pressed`,
    /// `custom.repeated`): whether the rules took the tick's keys (EXE6's
    /// UP opening the Cross window, `sub_8028B74`). The first part that
    /// answers true takes them.
    CustomKeys,
    /// `custom.take_back(side)`: B with nothing picked (`sub_8029032`):
    /// whether the rules took something back (EXE6's Cross chosen). The
    /// first part that answers true did; none, and B is refused.
    CustomTakeBack,
    /// A window's `update(side)`, each tick it is up: whether it
    /// stays up (`custom.window_tick` counts its ticks from 1).
    WindowUpdate,
    /// A button's `taken_back(side)`: B took its pick back (EXE6's Beast
    /// Out: its face goes).
    ButtonTakenBack,
    /// A custom-screen button's `shown(side)` (§4.4), as the
    /// screen opens: whether it is on the screen. (Not in `hooks`: each
    /// button names its own functions.)
    ButtonShown,
    /// A button's `state(side)`, at the open and after each pick unless it
    /// is selected: "selectable" or "unavailable". A button without one is
    /// selectable at the open and left as it is.
    ButtonState,
    /// A button's `pressed(side)`: A on it.
    ButtonPressed,
    /// A button's `chip(side)`, as the screen lays it out: the chip it
    /// shows, if any (EXE5's capsules: the chip window shows it, R describes
    /// it).
    ButtonChip,
    /// `form_reverted(side, navi)`: the framework reverts the navi to its
    /// base form, its form not yet changed (`sub_80158CC`): EXE6's spends a
    /// Beast Out and exhausts a Beast Over. Its result is unused.
    FormReverted,
    /// `folder_check(side)`: a tool asks whether a folder keeps the side's
    /// game's folder rules (`Battle::check_folder`, not the simulation):
    /// the folder is `battle.checked_folder()`, and each rule it breaks is
    /// `battle.folder_problem(rule, text)`. Its result is unused.
    FolderCheck,
    /// `validate(side)`: a tool asks what is wrong with side `side`'s
    /// setup (`Battle::validate`, not the simulation), once the round is
    /// set up (the stats its rules built): a list of problems, each a
    /// sentence or `{ text, field?, entry? }` (the setup field it is of, and
    /// the entry of a list field, from 1). Nothing: none.
    Validate,
}

impl RulesHook {
    /// The hook's name in the rules' `hooks` table.
    pub fn name(self) -> &'static str {
        match self {
            RulesHook::RoundSetup => "round_setup",
            RulesHook::RoundStart => "round_start",
            RulesHook::TurnOpened => "turn_opened",
            RulesHook::TurnCheck => "turn_check",
            RulesHook::TurnStarted => "turn_started",
            RulesHook::CustomRequested => "custom_requested",
            RulesHook::CustomResult => "custom_result",
            RulesHook::CustomClosed => "custom_closed",
            RulesHook::FolderCheck => "folder_check",
            RulesHook::Validate => "validate",
            RulesHook::NaviIntake => "navi_intake",
            RulesHook::ChipCheck => "chip_check",
            RulesHook::ChipCost => "chip_cost",
            RulesHook::ChipSubstitute => "chip_substitute",
            RulesHook::ChipPrepared => "chip_prepared",
            RulesHook::ChipUsed => "chip_used",
            RulesHook::Controller => "controller",
            RulesHook::FormReverted => "form_reverted",
            RulesHook::TakeoverRequested => "takeover_requested",
            RulesHook::Countered => "countered",
            RulesHook::NaviTick => "navi_tick",
            RulesHook::StartingMood => "starting_mood",
            RulesHook::NaviPalette => "navi_palette",
            RulesHook::NaviBug => "navi_bug",
            RulesHook::HitBug => "hit_bug",
            RulesHook::ChargeReleased => "charge_released",
            RulesHook::ReleaseTaken => "release_taken",
            RulesHook::BugMark => "bug_mark",
            RulesHook::NaviFlinched => "navi_flinched",
            RulesHook::HpEmptied => "hp_emptied",
            RulesHook::ObstacleReaction => "obstacle_reaction",
            RulesHook::CustomHandSize => "custom.hand_size",
            RulesHook::CustomDeal => "custom.deal",
            RulesHook::CustomOpen => "custom.open",
            RulesHook::CustomConfirmed => "custom.confirmed",
            RulesHook::CustomKeys => "custom.keys",
            RulesHook::CustomTakeBack => "custom.take_back",
            RulesHook::CustomChipPicked => "custom.chip_picked",
            RulesHook::CustomChipTakenBack => "custom.chip_taken_back",
            RulesHook::WindowUpdate => "window.update",
            RulesHook::ButtonTakenBack => "button.taken_back",
            RulesHook::ButtonShown => "button.shown",
            RulesHook::ButtonState => "button.state",
            RulesHook::ButtonPressed => "button.pressed",
            RulesHook::ButtonChip => "button.chip",
            RulesHook::Takeover => "takeover",
        }
    }

    pub const ALL: [RulesHook; 46] = [
        RulesHook::RoundSetup,
        RulesHook::RoundStart,
        RulesHook::TurnOpened,
        RulesHook::TurnCheck,
        RulesHook::TurnStarted,
        RulesHook::CustomRequested,
        RulesHook::CustomResult,
        RulesHook::CustomClosed,
        RulesHook::FolderCheck,
        RulesHook::NaviIntake,
        RulesHook::ChipCheck,
        RulesHook::ChipCost,
        RulesHook::ChipSubstitute,
        RulesHook::ChipPrepared,
        RulesHook::ChipUsed,
        RulesHook::Controller,
        RulesHook::FormReverted,
        RulesHook::TakeoverRequested,
        RulesHook::Takeover,
        RulesHook::Countered,
        RulesHook::NaviTick,
        RulesHook::StartingMood,
        RulesHook::NaviPalette,
        RulesHook::NaviBug,
        RulesHook::HitBug,
        RulesHook::ChargeReleased,
        RulesHook::ReleaseTaken,
        RulesHook::BugMark,
        RulesHook::NaviFlinched,
        RulesHook::HpEmptied,
        RulesHook::ObstacleReaction,
        RulesHook::CustomHandSize,
        RulesHook::ButtonShown,
        RulesHook::ButtonState,
        RulesHook::ButtonPressed,
        RulesHook::ButtonChip,
        RulesHook::CustomOpen,
        RulesHook::CustomDeal,
        RulesHook::CustomConfirmed,
        RulesHook::WindowUpdate,
        RulesHook::ButtonTakenBack,
        RulesHook::CustomChipPicked,
        RulesHook::CustomChipTakenBack,
        RulesHook::CustomKeys,
        RulesHook::CustomTakeBack,
        RulesHook::Validate,
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
    /// `rules`: the side whose rules' state the action reaches, for an
    /// action of the rules' (the rules' `actions`,
    /// docs/design/rules-in-luau.md §5.3).
    fn update_action(
        &self,
        api: &mut dyn CoreApi,
        f: FnId,
        me: ObjectRef,
        state: StateId,
        rules: Option<u8>,
    ) -> Result<(), ContentError>;
    /// Call function `f` for a hook.
    fn call_hook(&self, api: &mut dyn CoreApi, f: FnId, call: HookCall) -> Result<Value, ContentError>;
    /// What module `module` (by its name: a game pack's module by its pack
    /// and path, `exe6:rules/navicust/board`) returned as the content
    /// loaded, as plain data: for a tool reading a game's data (the
    /// editor's NaviCust board). Nothing is called, and a function in it is
    /// left out. None: no module of that name was loaded.
    fn module_data(&self, _module: &str) -> Option<Result<crate::Data, ContentError>> {
        None
    }
}
