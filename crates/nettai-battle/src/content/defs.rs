//! What the content defines (docs/design/content-model-v2.md): the
//! registries the engine reads, with their handles, and the functions the
//! runtime binds.
//!
//! A registry holds two sorts of entries:
//!
//! - the engine's own (its object kinds, keyed `engine/...`);
//! - content's definitions (`define.kind { ... }`), keyed by their keys.
//!
//! Each registry's keys are sorted byte-wise; an entry's handle is its
//! place.
//!
//! The engine never learns the original's numbers for what content
//! defines: an object records its kind's handle, a navi its content
//! action's, and the validator (`exe6-compat`) maps them to the original's
//! numbers with compat (docs/design/content-model-v2.md §7.3).

use std::collections::BTreeMap;

use nettai_content_api::SpriteId;

use nettai_content_api::{
    ActionHandle, ChipHandle, ContentError, Data, Definition, Definitions, FnId, FnSource, FormHandle, KindHandle,
    NaviHandle, Pool, RecordHandle, Registry, Schema, StageHandle, StateId, SystemHandle, SystemHook,
    WeaponHandle,
};

use super::{
    ChipData, Content,
    FormData, NaviData,
};
use super::views::{ButtonView, PlayerFact, ViewFields, WindowView};
use super::roles::{
    ActionRole, BannerRole, ChipRole, CollisionRole, EffectRole, HookRole, KindRole, MusicRole, RegionRole,
    Roles, SoundRole, SparkRole, SpriteRole, StatusRole,
};
use crate::kinds::{ENGINE_KINDS, EngineKind};

/// Who implements an object kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KindImpl {
    /// The engine's own Rust.
    Engine(EngineKind),
    /// Content: its `update`.
    Script { update: FnId },
}

/// An object kind.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KindDef {
    pub key: String,
    /// Its pool: update order and slot limits.
    pub pool: Pool,
    pub implementation: KindImpl,
    /// A content kind's state layout.
    pub schema: StateId,
    /// What places it when a stage names it (`kind.place`).
    pub place: Option<FnId>,
    /// What the leaving of a navi chip's navi one of its objects brought
    /// does to it (`kind.navi_left`; EXE5's DethPhnx): the navi calls
    /// `navi_chip.navi_left` with the object.
    pub navi_left: Option<FnId>,
}

/// A navi action content implements.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionDef {
    pub key: String,
    pub update: FnId,
    /// Its state layout. Actions of one layout continue each other's
    /// attack state, as the original's actions share theirs.
    pub schema: StateId,
}

/// A weapon: what a button's weapon does.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WeaponDef {
    pub key: String,
    /// `setup(navi) -> action` (`off_80117D4`'s routine); none for an
    /// A-charge that is its chip (`charged_chip`), which nothing starts as
    /// a weapon.
    pub setup: Option<FnId>,
    /// Ticks to a full charge by Charge stat (past the original's five,
    /// what the game reads on into).
    pub charge_ticks: Vec<u16>,
    /// The instant effect its action (the instant chips' action) runs: a
    /// chip's (the arm chips' weapons'), or one no chip has (TenguCross's
    /// wind).
    pub instant: Option<FnId>,
    /// The navi waits 8 ticks after the instant effect (the original's
    /// effect 0x14, which no chip has: TenguCross's wind).
    pub instant_waits: bool,
    /// As a charged shot: it stays when the form's own would replace it
    /// (`sub_800FFAA`: a chip's weapon), and its attack is of the kind
    /// that runs through a dimming.
    pub sticky: bool,
    /// As a buster: it fires while B is held.
    pub held: bool,
    /// As a buster: the buster it gives way to while the charged shot is a
    /// sticky one (`sub_800FFAA`).
    pub plain: Option<WeaponHandle>,
    /// As an A-charge: what the charged chip is, where it isn't this
    /// weapon's own attack.
    pub charged_chip: Option<ChargedChip>,
}

/// What a charged chip is under an A-charge weapon that has no attack of
/// its own (the original's `nullsub_44` entries, which `sub_800FB54` tells
/// apart by number before it would call them).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChargedChip {
    /// The chip itself, with the charge's bonus.
    Bonus,
    /// The chip itself after GroundCross's falling rocks (`sub_8012CB2`).
    RockBarrage,
}

/// How a chip is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChipUsage {
    /// A navi action: the chip's attack.
    Action(ActionHandle),
    /// A cut-in chip: its controller's spawner, `(user, spec) -> controller`.
    Dimming(FnId),
    /// A navi chip: its navi's spawner, `(user, controller, spec) -> navi`.
    Navi(FnId),
    /// An instant chip: its effect, `(user, spec)`.
    Instant(FnId),
}

/// A chip content defines. (The engine never learns the original's number
/// for one: compat has it by key.)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChipDef {
    pub key: String,
    pub record: ChipData,
    /// How it is used: its definition's `action`, `dimming`, `navi` or
    /// `instant`.
    pub usage: ChipUsage,
    /// `setup(navi)`: what the chip itself does to the attack as its use
    /// is prepared (EXE5's 0x0800EC68, the routine its record's +0x1F
    /// names: a team navi's own chip's attack variables), after the hand's
    /// bonuses and before the side's rules check the chip.
    pub setup: Option<FnId>,
    /// What its record names, by handle.
    pub links: ChipLinks,
}

/// What a chip's record names by key or name, resolved: the ruleset reads
/// these.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChipLinks {
    /// An SP navi chip's slot among the setup's SP deletion times
    /// (`DamageFormula::SpNavi::slot`, by the rules' `sp_slots`).
    pub sp_slot: Option<u8>,
    /// The navi whose own chip this is (a link navi's chip: offered on the
    /// custom screen once a round, never charged, dropped from the hand
    /// when a round ends).
    pub own_chip_of: Option<NaviHandle>,
    /// Its place among the Program Advances' results (those chips with a
    /// recipe, in handle order): the bit a formed one takes in a player's
    /// record of the round.
    pub advance: Option<u8>,
}

/// A navi (`define.navi`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NaviDef {
    pub key: String,
    pub record: NaviData,
    /// A link navi's own chip (`NaviData::own_chip`), by handle.
    pub own_chip: Option<(ChipHandle, super::ChipCode)>,
    /// `tick(navi)`: its own part of the per-tick pipeline, for a navi
    /// that doesn't change form (one that does runs its form's `tick`):
    /// the original's table by AI index (`off_80EA93C`; EXE5's 0x080EB1E8,
    /// where GyroMan's entry watches the panel under him).
    pub tick: Option<FnId>,
    /// `idle(navi)`: its own idle, before the common one, which doesn't
    /// run on a tick it starts an action (EXE5's table by control mode and
    /// AI index, 0x080EB068: GyroMan's entry starts his take-off and his
    /// landing; every entry of EXE6's `JumpTable80EA7B0` is the common
    /// idle).
    pub idle: Option<FnId>,
    /// `post_init(navi)`: its post-init hook (`sub_800F378`'s table by AI
    /// index, `off_80EAA04`; EXE5's 0x080EB2A8), as a player's init ends
    /// and after a navi switch: EXE6's DustMan's two objects of battle
    /// mode 9, EXE5's ToadMan's dive.
    pub post_init: Option<FnId>,
}

/// The record type of a lock-on mode (`define.record("lockon", ...)`).
pub const LOCKON_RECORD: &str = "lockon";

/// One of MegaMan's forms (`define.form`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FormDef {
    pub key: String,
    pub record: FormData,
    /// `reset(navi)`: what else the status reset does in it, after its
    /// `status_reset` (EXE5's souls' routines, 0x08011B92: SearchSoul's
    /// reveal, ColonelSoul's, TomahawkSoul's grass).
    pub reset: Option<FnId>,
    /// `put_on(navi)`: the form's put-on routine (`sub_8011268`'s by the
    /// form; EXE5's table 0x0800F038, by soul), in place of the default, its
    /// identity's parts (`Navi:put_on_form_parts`). A base form's also runs
    /// as a player's init ends, in place of its navi's init hook
    /// (`sub_8010DD0`: EXE5's MegaMan's record names the base form's
    /// routine, 0x0800EE1C, Hub Style's shade). EXE5's NumberSoul's puts on
    /// its layer (0x0800F07C).
    pub put_on: Option<FnId>,
    /// `take_off(navi)`: the form's take-off routine (`sub_8011384`'s; EXE5's
    /// table 0x0800F09C), in place of the default, taking down what its
    /// identity's death hook does (`Navi:take_off_form_parts`). EXE5's
    /// NumberSoul's takes its layer off (0x0800F0DE).
    pub take_off: Option<FnId>,
    /// `tick(navi)`: the form's own part of the per-form tick (EXE5's
    /// MegaMan's, 0x080F04CE: GyroSoul's propeller by the priming).
    pub tick: Option<FnId>,
}

/// A stage (`define.stage`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StageDef {
    pub key: String,
    pub record: super::StageData,
}

/// A status effect (`define.status`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StatusDef {
    pub key: String,
    pub effect: super::StatusEffect,
}

/// A Beast Out lock-on mode: a record of type "lockon" (`LOCKON_RECORD`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LockonDef {
    /// Its record (a record of type "lockon").
    pub record: RecordHandle,
    pub key: String,
    pub mode: super::LockonMode,
}

/// A collision type content defines (`define.collision`): what an object is
/// or what it hits, as the flag words for side 0 and side 1
/// (`sub_801A0BA`). A reacts to B when A's target flags meet B's self
/// flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollisionTypeDef {
    pub flags: [u32; 2],
    /// The offset the original's lookup of this type's row leaves in a
    /// register (the row's index times 8), which a damage word's bug code
    /// takes as its high byte (`sub_801A00E`): a quirk materialized where
    /// it is used (docs/design/content-model-v2.md §3.3).
    pub row_offset: u16,
}


/// A system's extension of a registry's definitions
/// (docs/design/rules-in-luau.md §7.5, S7): a field its game's definitions
/// may carry for it (EXE6's dark-chips system's `hp_bug` on a chip), of a
/// type. The engine checks it as the content is defined and reads none of
/// it: Luau reads it on the definition, tools through [`Defs::extension`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Extension {
    pub registry: Registry,
    pub field: String,
    pub ty: ExtensionType,
}

/// An extension field's type: a state field's (`"u8"`, `"chip"`, a list of
/// variants), or a table of such fields.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExtensionType {
    Value(nettai_content_api::FieldType),
    Table(Vec<(String, ExtensionType)>),
}

impl ExtensionType {
    /// The type `v` declares (`at`: where, for errors).
    fn read(v: &Data, at: &str) -> Result<ExtensionType, String> {
        use nettai_content_api::FieldType;
        match v {
            Data::Str(name) => FieldType::scalar(name).map(ExtensionType::Value).ok_or_else(|| format!("{at}: no type is named {name:?}")),
            Data::List(variants) if !variants.is_empty() => {
                let names = variants
                    .iter()
                    .map(|v| v.str().map(str::to_string).ok_or_else(|| format!("{at}: variants are names")))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ExtensionType::Value(FieldType::Enum(names)))
            }
            Data::Map(fields) => Ok(ExtensionType::Table(
                fields.iter().map(|(k, v)| Ok((k.to_string(), ExtensionType::read(v, &format!("{at}.{k}"))?))).collect::<Result<_, String>>()?,
            )),
            _ => Err(format!("{at}: a type name, a list of variants or a table of fields")),
        }
    }

    /// Whether `v` (a definition's, none allowed) is of this type.
    fn check(&self, v: &Data, at: &str) -> Result<(), String> {
        use nettai_content_api::FieldType as T;
        let int = |n: i64, lo: i64, hi: i64| (lo..=hi).contains(&n);
        let ok = match (self, v) {
            (_, Data::Nil) => true,
            (ExtensionType::Table(fields), Data::Map(entries)) => {
                for (k, x) in entries {
                    let k = k.to_string();
                    let Some((_, t)) = fields.iter().find(|(f, _)| *f == k) else {
                        let known: Vec<&str> = fields.iter().map(|(f, _)| f.as_str()).collect();
                        return Err(format!("{at}: `{k}` is none of its fields ({})", known.join(", ")));
                    };
                    t.check(x, &format!("{at}.{k}"))?;
                }
                true
            }
            (ExtensionType::Value(T::Bool), Data::Bool(_)) => true,
            (ExtensionType::Value(T::U8), Data::Int(n)) => int(*n, 0, 0xFF),
            (ExtensionType::Value(T::U16), Data::Int(n)) => int(*n, 0, 0xFFFF),
            (ExtensionType::Value(T::U32), Data::Int(n)) => int(*n, 0, 0xFFFF_FFFF),
            (ExtensionType::Value(T::I8), Data::Int(n)) => int(*n, -0x80, 0x7F),
            (ExtensionType::Value(T::I16), Data::Int(n)) => int(*n, -0x8000, 0x7FFF),
            (ExtensionType::Value(T::I32), Data::Int(n)) => int(*n, -0x8000_0000, 0x7FFF_FFFF),
            (ExtensionType::Value(T::Enum(names)), Data::Str(name)) => names.contains(name),
            (ExtensionType::Value(T::Ref(r, _)), Data::Ref(r2, _)) => r == r2,
            (ExtensionType::Value(T::Asset(k)), Data::Asset(k2, _)) => k == k2,
            (ExtensionType::Value(T::Array(elem, n)), Data::List(items)) => {
                for (i, x) in items.iter().enumerate() {
                    ExtensionType::Value((**elem).clone()).check(x, &format!("{at}[{}]", i + 1))?;
                }
                items.len() <= *n as usize
            }
            _ => false,
        };
        if ok { Ok(()) } else { Err(format!("{at} is {v:?}, not {}", self.describe())) }
    }

    fn describe(&self) -> String {
        match self {
            ExtensionType::Value(t) => t.to_string(),
            ExtensionType::Table(fields) => {
                let f: Vec<String> = fields.iter().map(|(k, t)| format!("{k}: {}", t.describe())).collect();
                format!("{{ {} }}", f.join(", "))
            }
        }
    }
}

/// A system of a game's rules (docs/design/rules-in-luau.md §2.2): its state
/// per side, its player setup, its hooks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SystemDef {
    pub key: String,
    /// The layout of its state of a side, and of its player setup.
    pub state: StateId,
    pub setup: StateId,
    /// Its player setup when the player's setup says nothing of a field:
    /// its `setup_defaults` (EXE5's light/dark value a fresh save's 500; an
    /// array's, a list: EXE6's Crosses owned, every one),
    /// zero elsewhere, but an enum, which has no default unless
    /// `setup_defaults` gives it one: it is left unstated
    /// (`ContentState::unstate`: EXE6's player's version, gregar or falzar),
    /// and a round doesn't start until the player's setup states it
    /// ([`SystemDef::setup_block`], `SideRules::for_player`).
    pub setup_default: nettai_content_api::ContentState,
    /// The layout of its state of each navi no player controls that it
    /// drives (its `controller`: the navi object's own state), if it
    /// drives any.
    pub navi_state: Option<StateId>,
    /// Its hooks, in [`SystemHook::ALL`]'s order.
    hooks: Vec<Option<FnId>>,
    /// Its own actions, which reach its state.
    pub actions: Vec<ActionHandle>,
    /// Its custom-screen buttons and windows.
    pub buttons: Vec<ButtonHandle>,
    pub windows: Vec<WindowHandle>,
    /// The fields of its state a frontend reads for its windows' and
    /// buttons' views.
    pub views: ViewFields,
    /// Its extensions of its game's definitions (`extends`).
    pub extends: Vec<Extension>,
    /// The chips its rules can't play of a player's auto battle data, each with why
    /// (`unplayable_in_auto_battle`: EXE5's auto battle's chips whose positioning
    /// class the original has no routine for): for tools (a match's check;
    /// the engine reads none, and its rules raise when one is played).
    pub unplayable_in_auto_battle: Vec<(ChipHandle, String)>,
}

impl SystemDef {
    /// A player's setup block for it when the player gives none: its
    /// defaults (`setup_defaults`), the rest zero, an enum without a
    /// default unstated.
    pub fn setup_block(&self) -> nettai_content_api::ContentState {
        self.setup_default
    }

    /// Its function for `hook`, if it has one.
    pub fn hook(&self, hook: SystemHook) -> Option<FnId> {
        let i = SystemHook::ALL.iter().position(|&h| h == hook).expect("every hook is listed");
        self.hooks[i]
    }
}

/// A custom-screen button of a system's (docs/design/rules-in-luau.md
/// §4.4): where it sits, and its functions, which run as its system's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ButtonDef {
    pub system: SystemHandle,
    /// Its name in the system's `buttons` (the key a pack has its look
    /// under).
    pub name: String,
    /// What a frontend draws of it besides its look (`view`), if it says.
    pub view: Option<ButtonView>,
    /// Its first slot, and how many it takes (1 or 2).
    pub slot: u8,
    pub cells: u8,
    /// Uses it has on a screen (the machinery it starts counts them).
    pub uses: u8,
    /// Its first cell's right neighbor and its last cell's left one, if
    /// not the layout's.
    pub right: Option<u8>,
    pub left: Option<u8>,
    pub shown: FnId,
    pub state: Option<FnId>,
    pub pressed: FnId,
    pub taken_back: Option<FnId>,
    /// The chip it shows (`chip`), if it shows one.
    pub chip: Option<FnId>,
}

/// A custom-screen window of a system's (docs/design/rules-in-luau.md
/// §4.4): its update, run as its system's each tick it is up.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowDef {
    pub system: SystemHandle,
    /// Its name in the system's `windows` (what `custom.open_window` opens
    /// it by).
    pub name: String,
    /// What a frontend draws while it is up (`view`), if it says.
    pub view: Option<WindowView>,
    pub update: FnId,
}

/// A window, by its place in [`Defs::windows`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowHandle(pub u16);

/// A button, by its place in [`Defs::buttons`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ButtonHandle(pub u16);

/// A game's rules (docs/design/rules-in-luau.md §2.2): its ruleset's
/// systems, in the order the framework calls them. A game has one ruleset
/// (the user: "there should only be one ruleset per game"), which every
/// match of it plays by.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RulesetDef {
    pub systems: Vec<SystemHandle>,
}

/// Most systems a ruleset may list.
pub const MAX_SYSTEMS: usize = 16;

/// A record only content reads: the engine keeps its handle and type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordDef {
    pub key: String,
    pub record_type: String,
}

/// A patch card (`define.patch_card`; BN4's, EXE5's and EXE6's Modification
/// Cards, docs/engine/patch-cards.md): what every game's card is. A game's
/// rules give its effects their meaning (EXE6's patch-cards system applies a
/// player's cards as the round is set up); the engine keeps the card's
/// capacity cost and its effects' kinds, and the effects' own fields stay
/// the definition's data, which the rules read. Its name is the locales'.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PatchCardDef {
    pub key: String,
    /// Its capacity cost (EXE6's MB): what the installed cards' limit counts.
    pub mb: u8,
    /// Its effects in the card's order.
    pub effects: Vec<PatchCardEffect>,
}

/// An effect of a patch card: its kind (a game's rules say what it does)
/// and whether the card shows it as a bug (a menu's red text; no game's
/// application reads it).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PatchCardEffect {
    pub kind: String,
    pub bug: bool,
}

/// A NaviCust program (`define.navicust_program`; BN4's, EXE5's and EXE6's
/// Navi Customizer parts, docs/design/navicust.md): what every game's
/// program is. The engine keeps what a NaviCust's board needs of it, its
/// colors and shapes and whether it is a plus part; the rest (EXE6's: what it
/// does, which bug it brings, which programs it excludes) is the
/// definition's data, which the game's rules read. Its name is the
/// locales'.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NaviCustProgramDef {
    pub key: String,
    /// The colors it comes in (EXE6's `white`, `yellow`, `pink`, `red`,
    /// `blue`, `green`), in its variants' order: a placed program's color
    /// is an index into them.
    pub colors: Vec<String>,
    /// A plus part (EXE6: one that belongs off the command line).
    pub plus: bool,
    /// Its shape, centered on the grid's middle cell, and compressed (none:
    /// it doesn't compress).
    pub shape: crate::navicust::Shape,
    pub compressed: Option<crate::navicust::Shape>,
}

impl NaviCustProgramDef {
    /// Its shape as placed: compressed or not, turned.
    pub fn placed_shape(&self, compressed: bool, rotation: u8) -> crate::navicust::Shape {
        let s = if compressed { self.compressed.as_ref().unwrap_or(&self.shape) } else { &self.shape };
        crate::navicust::rotate(s, rotation)
    }
}

/// A shape from a definition: seven rows of seven cells, `#` a cell it
/// covers and `.` one it doesn't.
fn read_shape(d: &Data) -> Result<crate::navicust::Shape, String> {
    use crate::navicust::SIZE;
    let Data::List(rows) = d else { return Err(format!("a shape is {SIZE} rows of {SIZE} cells (strings of `#` and `.`)")) };
    if rows.len() != SIZE {
        return Err(format!("a shape is {SIZE} rows, not {}", rows.len()));
    }
    let mut shape = [[false; SIZE]; SIZE];
    for (y, row) in rows.iter().enumerate() {
        let Some(row) = row.str() else { return Err(format!("row {} is not a string", y + 1)) };
        if row.len() != SIZE || !row.bytes().all(|b| b == b'#' || b == b'.') {
            return Err(format!("row {} is not {SIZE} cells of `#` and `.`: {row:?}", y + 1));
        }
        for (x, b) in row.bytes().enumerate() {
            shape[y][x] = b == b'#';
        }
    }
    if !shape.iter().flatten().any(|&c| c) {
        return Err("a shape covers no cell".into());
    }
    Ok(shape)
}

/// A content state layout.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SchemaDef {
    pub key: String,
    pub schema: Schema,
}

/// The key of the layout of content that declares no state.
pub const NO_STATE: &str = "";

/// What the content defines. Built by [`Content::define`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Defs {
    /// Whether the content was defined (a battle needs it).
    pub defined: bool,
    /// What the define phase read.
    pub definitions: Definitions,
    /// Each definition's handle, in `definitions.defs`' order.
    pub handles: Vec<u16>,
    pub kinds: Vec<KindDef>,
    pub actions: Vec<ActionDef>,
    pub weapons: Vec<WeaponDef>,
    pub chips: Vec<ChipDef>,
    /// The chips whose damage is a formula (read from the battle), in
    /// handle order.
    pub formula_chips: Vec<ChipHandle>,
    pub navis: Vec<NaviDef>,
    pub forms: Vec<FormDef>,
    pub stages: Vec<StageDef>,
    /// The Beast Out lock-on modes, by handle.
    pub lockons: Vec<LockonDef>,
    /// The identities, by handle (`define.identity`, in key order).
    pub identities: Vec<super::Identity>,
    /// The status effects, by handle.
    pub statuses: Vec<StatusDef>,
    pub records: Vec<RecordDef>,
    /// The patch cards, by handle.
    pub patch_cards: Vec<PatchCardDef>,
    /// The NaviCust programs, by handle.
    pub navicust_programs: Vec<NaviCustProgramDef>,
    /// One-shot effects' and hit sparks' looks (`define.effect`,
    /// `define.spark`), by handle.
    pub effects: Vec<super::EffectSprite>,
    pub sparks: Vec<super::EffectSprite>,
    /// Hit regions (`define.region`), by handle.
    pub regions: Vec<super::Region>,
    /// Collision types (`define.collision`), by handle.
    pub collisions: Vec<CollisionTypeDef>,
    /// What the game needs from content by role (its ruleset's `roles`;
    /// none given, none filled).
    pub roles: Roles,
    /// The systems (docs/design/rules-in-luau.md), by handle.
    pub systems: Vec<SystemDef>,
    /// The systems' custom-screen buttons and windows.
    pub buttons: Vec<ButtonDef>,
    pub windows: Vec<WindowDef>,
    /// The game's ruleset (none: content without one, whose sides have no
    /// systems).
    ruleset: Option<RulesetDef>,
    /// Where each fact a player brings is ([`PlayerFact::ALL`]'s order): the
    /// place among the ruleset's systems of the first whose setup has the
    /// field, and the field.
    facts: Vec<Option<(u8, u16)>>,
    /// Each action's system, if it is one's, by action handle.
    action_owner: Vec<Option<SystemHandle>>,
    /// Whether a form names the action as its change, by action handle.
    change_actions: Vec<bool>,
    /// The Program Advances, in the order they are tried (each chip holds
    /// the recipes that make it; `sub_8029520`).
    pub program_advances: Vec<super::ProgramAdvance>,
    /// State layouts by [`StateId`].
    pub schemas: Vec<SchemaDef>,
    /// The functions the runtime binds, by [`FnId`].
    pub functions: Vec<FnSource>,
    kind_keys: BTreeMap<String, KindHandle>,
    /// The engine's kinds, in [`ENGINE_KINDS`]' order.
    engine: Vec<KindHandle>,
    /// The game's base form: what its navis are in before they change form
    /// (rules-in-luau.md P1 item 12: EXE5's MegaMan's is EXE5's own); None
    /// for a game that defines none.
    pub base_form: Option<FormHandle>,
    /// Keys by registry, for the codecs.
    chip_keys: BTreeMap<String, ChipHandle>,
    weapon_keys: BTreeMap<String, WeaponHandle>,
    /// The game the content is (its game pack's id, `Content::game`); ""
    /// for content without packs (a test's modules alone).
    pub game: String,
}

impl Defs {
    /// Definition `key` of `registry`'s value of extension field `field`
    /// (its system's, `SystemDef::extends`), if it carries it: for tools
    /// (the engine reads none).
    pub fn extension(&self, registry: Registry, key: &str, field: &str) -> Option<&Data> {
        let d = self.definitions.get(registry, key)?;
        Some(d.spec.field(field)).filter(|v| !matches!(v, Data::Nil))
    }

    pub fn system(&self, h: SystemHandle) -> &SystemDef {
        &self.systems[h.index()]
    }

    /// Why the game's rules can't play `chip` of a player's auto battle data, if
    /// they can't (the first of its ruleset's systems that says:
    /// `SystemDef::unplayable_in_auto_battle`): for tools.
    pub fn unplayable_in_auto_battle(&self, chip: ChipHandle) -> Option<&str> {
        self.ruleset_systems().iter().find_map(|&s| self.system(s).unplayable_in_auto_battle.iter().find(|(c, _)| *c == chip).map(|(_, why)| why.as_str()))
    }

    pub fn button(&self, h: ButtonHandle) -> &ButtonDef {
        &self.buttons[h.0 as usize]
    }

    pub fn window(&self, h: WindowHandle) -> &WindowDef {
        &self.windows[h.0 as usize]
    }

    /// The system an action is one of, if any (its state is that system's).
    pub fn action_owner(&self, h: ActionHandle) -> Option<SystemHandle> {
        self.action_owner.get(h.index()).copied().flatten()
    }

    /// Whether a form names the action as the one that changes a navi into
    /// it.
    pub fn is_change_action(&self, h: ActionHandle) -> bool {
        self.change_actions.get(h.index()).copied().unwrap_or(false)
    }

    /// The game's ruleset, if it has one (docs/design/rules-in-luau.md
    /// §2.3: a game has one, which every match of it plays by).
    pub fn ruleset(&self) -> Option<&RulesetDef> {
        self.ruleset.as_ref()
    }

    /// The game's ruleset's systems, in its order (none without a ruleset).
    pub fn ruleset_systems(&self) -> &[SystemHandle] {
        self.ruleset.as_ref().map_or(&[], |r| &r.systems)
    }

    /// Where fact `fact` is: the place among the ruleset's systems of the
    /// one that keeps it, and the field of its setup. None: no system of
    /// the game's does.
    pub fn fact_field(&self, fact: PlayerFact) -> Option<(usize, usize)> {
        let k = PlayerFact::ALL.iter().position(|&f| f == fact).expect("every fact is listed");
        self.facts.get(k).copied().flatten().map(|(slot, field)| (slot as usize, field as usize))
    }

    /// The game's roles.
    pub fn roles(&self) -> &Roles {
        &self.roles
    }

    /// The kind with this key.
    pub fn kind_by_key(&self, key: &str) -> Option<KindHandle> {
        self.kind_keys.get(key).copied()
    }

    pub fn kind(&self, h: KindHandle) -> &KindDef {
        &self.kinds[h.index()]
    }

    /// The engine's kind `kind`.
    pub fn engine(&self, kind: EngineKind) -> KindHandle {
        let i = ENGINE_KINDS.iter().position(|e| e.0 == kind).expect("every engine kind is listed");
        self.engine[i]
    }

    /// Which of the engine's kinds `h` is, if it is one.
    pub fn engine_kind(&self, h: KindHandle) -> Option<EngineKind> {
        match self.kinds.get(h.index())?.implementation {
            KindImpl::Engine(k) => Some(k),
            KindImpl::Script { .. } => None,
        }
    }

    pub fn action(&self, h: ActionHandle) -> &ActionDef {
        &self.actions[h.index()]
    }

    /// The action with this key.
    pub fn action_by_key(&self, key: &str) -> Option<ActionHandle> {
        self.actions.binary_search_by(|a| a.key.as_str().cmp(key)).ok().map(|i| ActionHandle(i as u16))
    }

    /// The chip with this key.
    pub fn chip_by_key(&self, key: &str) -> Option<ChipHandle> {
        self.chip_keys.get(key).copied()
    }

    pub fn chip(&self, h: ChipHandle) -> &ChipDef {
        &self.chips[h.index()]
    }

    pub fn navi(&self, h: NaviHandle) -> &NaviDef {
        &self.navis[h.index()]
    }

    /// The navi with this key.
    pub fn navi_by_key(&self, key: &str) -> Option<NaviHandle> {
        self.navis.binary_search_by(|n| n.key.as_str().cmp(key)).ok().map(|i| NaviHandle(i as u16))
    }

    pub fn form(&self, h: FormHandle) -> &FormDef {
        &self.forms[h.index()]
    }

    /// The form with this key.
    pub fn form_by_key(&self, key: &str) -> Option<FormHandle> {
        self.forms.binary_search_by(|f| f.key.as_str().cmp(key)).ok().map(|i| FormHandle(i as u16))
    }

    pub fn stage(&self, h: StageHandle) -> &StageDef {
        &self.stages[h.index()]
    }

    /// The stage with this key.
    pub fn stage_by_key(&self, key: &str) -> Option<StageHandle> {
        self.stages.binary_search_by(|s| s.key.as_str().cmp(key)).ok().map(|i| StageHandle(i as u16))
    }

    /// The status effect with this key.
    pub fn status_by_key(&self, key: &str) -> Option<nettai_content_api::StatusHandle> {
        self.statuses.binary_search_by(|s| s.key.as_str().cmp(key)).ok().map(|i| nettai_content_api::StatusHandle(i as u16))
    }

    /// The identity with this key.
    pub fn identity_by_key(&self, key: &str) -> Option<nettai_content_api::IdentityHandle> {
        self.identities.binary_search_by(|i| i.key.as_str().cmp(key)).ok().map(|i| nettai_content_api::IdentityHandle(i as u16))
    }

    /// The lock-on mode with this key.
    pub fn lockon_by_key(&self, key: &str) -> Option<RecordHandle> {
        self.lockons.iter().find(|l| l.key == key).map(|l| l.record)
    }

    pub fn weapon(&self, h: WeaponHandle) -> &WeaponDef {
        &self.weapons[h.index()]
    }

    /// The weapon with this key.
    pub fn weapon_by_key(&self, key: &str) -> Option<WeaponHandle> {
        self.weapon_keys.get(key).copied()
    }

    /// The patch card with this key.
    pub fn patch_card_by_key(&self, key: &str) -> Option<nettai_content_api::PatchCardHandle> {
        self.patch_cards.binary_search_by(|c| c.key.as_str().cmp(key)).ok().map(|i| nettai_content_api::PatchCardHandle(i as u16))
    }

    pub fn patch_card(&self, h: nettai_content_api::PatchCardHandle) -> &PatchCardDef {
        &self.patch_cards[h.index()]
    }

    /// The NaviCust program with this key.
    pub fn navicust_program_by_key(&self, key: &str) -> Option<nettai_content_api::NaviCustProgramHandle> {
        self.navicust_programs.binary_search_by(|p| p.key.as_str().cmp(key)).ok().map(|i| nettai_content_api::NaviCustProgramHandle(i as u16))
    }

    pub fn navicust_program(&self, h: nettai_content_api::NaviCustProgramHandle) -> &NaviCustProgramDef {
        &self.navicust_programs[h.index()]
    }

    /// A record's handle by key.
    pub fn record(&self, key: &str) -> Option<RecordHandle> {
        self.records.binary_search_by(|r| r.key.as_str().cmp(key)).ok().map(|i| RecordHandle(i as u16))
    }

    /// The layout with this key.
    pub fn schema_named(&self, key: &str) -> Option<StateId> {
        self.schemas.binary_search_by(|s| s.key.as_str().cmp(key)).ok().map(|i| StateId(i as u16))
    }

    pub fn schema(&self, id: StateId) -> &Schema {
        &self.schemas[id.0 as usize].schema
    }
}

/// The builder's view of one registry while it is filled.
struct Entries<T> {
    registry: Registry,
    entries: Vec<(String, T, String)>,
}

impl<T> Entries<T> {
    fn new(registry: Registry) -> Entries<T> {
        Entries { registry, entries: Vec::new() }
    }

    /// Add an entry; `whose` says where it comes from, for messages.
    fn add(&mut self, key: String, v: T, whose: String) {
        self.entries.push((key, v, whose));
    }

    /// The entries in key order (their handles); two with one key are an
    /// error.
    fn sorted(mut self) -> Result<Vec<(String, T)>, ContentError> {
        self.entries.sort_by(|a, b| a.0.cmp(&b.0));
        for w in self.entries.windows(2) {
            if w[0].0 == w[1].0 {
                return Err(ContentError::new(format!(
                    "{} {:?} is both {} and {}",
                    self.registry, w[0].0, w[0].2, w[1].2
                )));
            }
        }
        Ok(self.entries.into_iter().map(|(k, v, _)| (k, v)).collect())
    }
}

/// An identity a navi or form names is that navi's or form's: one owner
/// each, of a player's class.
fn claim_identity(
    identities: &mut [super::Identity],
    h: nettai_content_api::IdentityHandle,
    owner: super::IdentityOwner,
    whose: &str,
) -> Result<(), ContentError> {
    let id = &mut identities[h.index()];
    if let Some(other) = id.owner {
        return Err(ContentError::new(format!("identity {} is both {other:?}'s and {whose}'s", id.key)));
    }
    if !id.class.is_player() {
        return Err(ContentError::new(format!(
            "identity {} ({whose}'s) is of class {:?}: a navi's or a form's is a player's (megaman, link_navi, cross, beast, cross_beast, beast_over)",
            id.key, id.class
        )));
    }
    id.owner = Some(owner);
    Ok(())
}

/// Functions, each once.
#[derive(Default)]
struct Functions {
    list: Vec<FnSource>,
    ids: BTreeMap<FnSource, FnId>,
}

impl Functions {
    fn id(&mut self, f: FnSource) -> FnId {
        if let Some(&id) = self.ids.get(&f) {
            return id;
        }
        let id = FnId(self.list.len() as u32);
        self.ids.insert(f.clone(), id);
        self.list.push(f);
        id
    }
}

/// A byte field of a definition (0 when absent).
fn byte(d: &Definition, field: &str) -> Result<u8, ContentError> {
    let what = |e: String| ContentError::new(format!("{}.luau: {} {}: {e}", d.module, d.registry, d.key));
    match d.spec.field(field) {
        Data::Nil => Ok(0),
        Data::Int(i) => u8::try_from(*i).map_err(|_| what(format!("`{field}` {i} is not a byte"))),
        _ => Err(what(format!("`{field}` is not a number"))),
    }
}

/// A definition's function slot at `path`, which must hold a function.
fn slot(d: &Definition, path: &str) -> Result<FnSource, ContentError> {
    match d.spec.field(path) {
        Data::Function => Ok(FnSource::slot(d.registry, &d.key, path)),
        Data::Nil => Err(ContentError::new(format!("{}.luau: {} {} needs `{path}`", d.module, d.registry, d.key))),
        _ => Err(ContentError::new(format!("{}.luau: {} {}'s `{path}` is not a function", d.module, d.registry, d.key))),
    }
}

/// A definition holds no display text: its strings (a name, a description,
/// a message) are the content root's `locales/<lang>.toml`, by its key
/// (`strings`), and a field that gives them is refused.
pub(crate) fn no_display_text(d: &Definition) -> Result<(), ContentError> {
    for field in ["name", "description", "description_lines"] {
        if !d.spec.field(field).is_nil() {
            return Err(ContentError::new(format!(
                "{}.luau: {} {}'s `{field}`: a definition holds no display text (nor counts of it): it is the content root's locales/<lang>.toml, by the key {:?}",
                d.module, d.registry, d.key, d.key
            )));
        }
    }
    Ok(())
}

/// The fields the game's systems extend its definitions of `registry` with
/// (`extends`), read off the systems' definitions before they are built:
/// what a record's reader leaves to them. (Building the systems checks
/// them.)
fn extended_fields(definitions: &Definitions, registry: Registry) -> Vec<String> {
    let mut fields = Vec::new();
    for s in definitions.of(Registry::System) {
        if let Data::Map(own) = s.spec.field("extends").field(registry.name()) {
            fields.extend(own.iter().map(|(k, _)| k.to_string()));
        }
    }
    fields
}

/// A chip definition's record (docs/design/content-model-v2.md §3.1): the
/// fields the engine reads, with the lock-on mode by the handle `r` gives
/// it, and the chips it names (its Program Advance recipes' ingredients, a
/// dark chip's substitute) by key: the registry resolves those to handles
/// once every chip has one (`ChipDef::links`).
pub(crate) fn chip_record(d: &Definition, r: &super::reader::SpecReader) -> Result<ChipData, ContentError> {
    use serde_json::{Map, Value as Json};
    let what = |e: String| ContentError::new(format!("{}.luau: chip {}: {e}", d.module, d.key));
    let spec = &d.spec;
    let json = |field: &str| -> Result<Json, ContentError> { r.json(spec.field(field), &format!("chip {}.{field}", d.key)).map_err(what) };
    let mut o = Map::new();
    no_display_text(d)?;
    // (The custom screen draws the chip's picture with it.)
    match json("art_palette")? {
        Json::Null => {}
        Json::Array(colors) if colors.len() == 16 && colors.iter().all(|c| c.as_u64().is_some_and(|c| c < 0x8000)) => {
            o.insert("art_palette".into(), Json::Array(colors));
        }
        other => return Err(what(format!("`art_palette` is {other}: 16 BGR555 colors (below 0x8000)"))),
    }
    let defaults: [(&str, Json); 13] = [
        ("codes", Json::Array(Vec::new())),
        ("element", "null".into()),
        ("family", "null".into()),
        ("class", "standard".into()),
        ("rarity", 0.into()),
        ("mb", 0.into()),
        ("flags", Json::Array(Vec::new())),
        ("extra_flags", Json::Array(Vec::new())),
        ("hit_param", 0.into()),
        ("lockout", 0.into()),
        ("damage", 0.into()),
        ("slot_in_limit", 0.into()),
        ("modifier", Json::Null),
    ];
    for (field, default) in defaults {
        let v = json(field)?;
        o.insert(field.into(), if v.is_null() { default } else { v });
    }
    // `damage`: a number, or a formula (`{ formula = "hp_lost" }`).
    match o["damage"].take() {
        formula @ Json::Object(_) => {
            o.insert("formula".into(), formula);
            o.insert("damage".into(), 0.into());
        }
        Json::Number(n) if n.as_i64().is_some_and(|v| (0..1000).contains(&v)) => {
            o.insert("damage".into(), Json::Number(n));
        }
        other => return Err(what(format!("`damage` is {other}: a number below 1000, or a formula"))),
    }
    // What the ruleset asks of this chip: its traits and the trap it is.
    // (A system's own fields, EXE6's dark chips' cost and substitute and its
    // Beast rush's lock-on, are its extension: SystemDef::extends.)
    for field in ["traits", "trap"] {
        let v = json(field)?;
        if !v.is_null() {
            o.insert(field.into(), v);
        }
    }
    let library = spec.field("library");
    for (field, from) in [("library_number", "number"), ("library_index", "index"), ("sort_key", "sort")] {
        o.insert(field.into(), library.field(from).int().unwrap_or(0).into());
    }
    o.insert("program_advance".into(), json("program_advances")?);
    if o["program_advance"].is_null() {
        o.insert("program_advance".into(), Json::Array(Vec::new()));
    }
    serde_json::from_value(Json::Object(o)).map_err(|e| what(e.to_string()))
}

/// The game's ruleset (`define.ruleset { ... }`, its rules/init.luau),
/// which holds its rule sections and its roles; none for content without
/// one. (A game has one: the define phase refuses two.)
pub(crate) fn ruleset(definitions: &Definitions) -> Option<&Definition> {
    definitions.of(Registry::Ruleset).first()
}

/// The ruleset's `roles = { actions = { ... }, kinds = { ... } }`
/// (content::roles; a plain table, rules/roles.luau): each role a
/// definition.
fn read_roles(
    d: &Definition,
    definitions: &Definitions,
    assets: &nettai_content_api::AssetNames,
    actions: &[ActionDef],
    kinds: &[KindDef],
    chips: &[ChipDef],
    statuses: &[StatusDef],
    functions: &mut Functions,
) -> Result<Roles, ContentError> {
    let what = |e: String| ContentError::new(format!("{}.luau: ruleset: roles: {e}", d.module));
    let groups: &[(nettai_content_api::DataKey, Data)] = match d.spec.field("roles") {
        Data::Nil => &[],
        Data::Map(groups) => groups,
        _ => return Err(what("a table of role groups".into())),
    };
    let mut roles = Roles::default();
    for (group, entries) in groups {
        let group = group.to_string();
        let Data::Map(entries) = entries else { return Err(what(format!("`{group}` is a table"))) };
        for (name, v) in entries {
            let name = name.to_string();
            match group.as_str() {
                "actions" => {
                    let names: Vec<&str> = ActionRole::ALL.iter().map(|r| r.name()).collect();
                    let role = ActionRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role actions.{name} (it has {})", names.join(", ")))
                    })?;
                    let full = format!("actions.{name}");
                    let Data::Ref(Registry::Action, key) = v else {
                        return Err(what(format!("{full} is not an action")));
                    };
                    let target = ActionHandle(actions.iter().position(|a| &a.key == key).expect("a defined action") as u16);
                    roles.actions.insert(role, target);
                }
                "kinds" => {
                    let names: Vec<&str> = KindRole::ALL.iter().map(|r| r.name()).collect();
                    let role = KindRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role kinds.{name} (it has {})", names.join(", ")))
                    })?;
                    let full = format!("kinds.{name}");
                    let Data::Ref(Registry::Kind, key) = v else {
                        return Err(what(format!("{full} is not a kind")));
                    };
                    let target = KindHandle(kinds.iter().position(|k| &k.key == key).expect("a defined kind") as u16);
                    roles.kinds.insert(role, target);
                }
                "hooks" => {
                    let names: Vec<&str> = HookRole::ALL.iter().map(|r| r.name()).collect();
                    let role = HookRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role hooks.{name} (it has {})", names.join(", ")))
                    })?;
                    if !matches!(v, Data::Function) {
                        return Err(what(format!("hooks.{name} is not a function")));
                    }
                    roles.hooks.insert(role, functions.id(FnSource::slot(Registry::Ruleset, &d.key, &format!("roles.hooks.{name}"))));
                }
                "chips" => {
                    let names: Vec<&str> = ChipRole::ALL.iter().map(|r| r.name()).collect();
                    let role = ChipRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role chips.{name} (it has {})", names.join(", ")))
                    })?;
                    let full = format!("chips.{name}");
                    let Data::Ref(Registry::Chip, key) = v else {
                        return Err(what(format!("{full} is not a chip")));
                    };
                    let i = chips
                        .binary_search_by(|c| c.key.as_str().cmp(&key))
                        .map_err(|_| what(format!("{full} names the chip {key:?}, which the content doesn't have")))?;
                    roles.chips.insert(role, ChipHandle(i as u16));
                }
                "statuses" => {
                    let names: Vec<&str> = StatusRole::ALL.iter().map(|r| r.name()).collect();
                    let role = StatusRole::named(&name).ok_or_else(|| {
                        what(format!("the ruleset has no role statuses.{name} (it has {})", names.join(", ")))
                    })?;
                    let Data::Ref(Registry::Status, key) = v else {
                        return Err(what(format!("statuses.{name} is not a status")));
                    };
                    let h = statuses.iter().position(|s| &s.key == key).expect("a defined status");
                    roles.statuses.insert(role, nettai_content_api::StatusHandle(h as u16));
                }
                "effects" => {
                    let role = definition_role(&group, &name, EffectRole::named, EffectRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let h = definition_handle(definitions, Registry::Effect, &group, &name, v).map_err(&what)?;
                    roles.effects.insert(role, nettai_content_api::EffectHandle(h));
                }
                "sparks" => {
                    let role = definition_role(&group, &name, SparkRole::named, SparkRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let h = definition_handle(definitions, Registry::Spark, &group, &name, v).map_err(&what)?;
                    roles.sparks.insert(role, nettai_content_api::SparkHandle(h));
                }
                "regions" => {
                    let role = definition_role(&group, &name, RegionRole::named, RegionRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let h = definition_handle(definitions, Registry::Region, &group, &name, v).map_err(&what)?;
                    roles.regions.insert(role, nettai_content_api::RegionHandle(h));
                }
                "collision" => {
                    let role =
                        definition_role(&group, &name, CollisionRole::named, CollisionRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let h = definition_handle(definitions, Registry::Collision, &group, &name, v).map_err(&what)?;
                    roles.collisions.insert(role, nettai_content_api::CollisionHandle(h));
                }
                "sounds" => {
                    let role = definition_role(&group, &name, SoundRole::named, SoundRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let id = role_asset(assets, nettai_content_api::AssetKind::Sound, &group, &name, v).map_err(&what)?;
                    roles.sounds.insert(role, crate::sound::SoundId(id));
                }
                "music" => {
                    let role = definition_role(&group, &name, MusicRole::named, MusicRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let id = role_asset(assets, nettai_content_api::AssetKind::Sound, &group, &name, v).map_err(&what)?;
                    roles.music.insert(role, crate::sound::SoundId(id));
                }
                "sprites" => {
                    let role = definition_role(&group, &name, SpriteRole::named, SpriteRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let sprite = match v {
                        Data::Asset(nettai_content_api::AssetKind::Sprite, asset) => assets
                            .handle(nettai_content_api::AssetKind::Sprite, asset)
                            .map(SpriteId)
                            .ok_or_else(|| what(format!("sprites.{name}: the packs have no sprite {asset:?}")))?,
                        _ => return Err(what(format!("sprites.{name} is not a sprite asset (asset.sprite(...))"))),
                    };
                    roles.sprites.insert(role, sprite);
                }
                "banners" => {
                    let role = definition_role(&group, &name, BannerRole::named, BannerRole::ALL.iter().map(|r| r.name())).map_err(&what)?;
                    let id = role_asset(assets, nettai_content_api::AssetKind::Banner, &group, &name, v).map_err(&what)?;
                    roles.banners.insert(role, super::BannerId(id));
                }
                _ => {
                    return Err(what(format!(
                        "the ruleset has no role group `{group}` (it has actions, kinds, hooks, chips, lockon, statuses, effects, sparks, regions, collision, sounds, music, sprites, banners)"
                    )));
                }
            }
        }
    }
    Ok(roles)
}

/// The role `group.name`, of a group whose roles name definitions.
fn definition_role<R>(
    group: &str,
    name: &str,
    named: impl Fn(&str) -> Option<R>,
    names: impl Iterator<Item = &'static str>,
) -> Result<R, String> {
    named(name).ok_or_else(|| {
        format!("the ruleset has no role {group}.{name} (it has {})", names.collect::<Vec<_>>().join(", "))
    })
}

/// The asset of `kind` the role `group.name` names, as the engine
/// identifies it (the pack's id for it).
fn role_asset(
    assets: &nettai_content_api::AssetNames,
    kind: nettai_content_api::AssetKind,
    group: &str,
    name: &str,
    v: &Data,
) -> Result<u16, String> {
    match v {
        Data::Asset(k, asset) if *k == kind => {
            assets.handle(kind, asset).ok_or_else(|| format!("{group}.{name}: the packs have no {kind} {asset:?}"))
        }
        _ => Err(format!("{group}.{name} is not a {kind} asset (asset.{kind}(...))")),
    }
}

/// The handle of the `registry` definition the role `group.name` names: its
/// place in the registry's key order.
fn definition_handle(definitions: &Definitions, registry: Registry, group: &str, name: &str, v: &Data) -> Result<u16, String> {
    match v {
        Data::Ref(r, key) if *r == registry => {
            Ok(definitions.of(registry).iter().position(|d| &d.key == key).expect("a defined entry") as u16)
        }
        _ => Err(format!("{group}.{name} is not a {registry} definition")),
    }
}

impl Defs {
    /// What `content` (its data's registrations and the engine's own) and
    /// `definitions` (what its modules define) make.
    pub fn build(content: &Content, definitions: Definitions) -> Result<Defs, ContentError> {
        let reader = super::reader::SpecReader::new(&content.assets, &definitions);
        // The game (content without packs is one game of no name).
        let game = content.game().to_string();
        let mut functions = Functions::default();

        // Layouts: the definitions' and modules' state tables, and the
        // empty one.
        let mut schemas = vec![SchemaDef { key: NO_STATE.to_string(), schema: Schema::default() }];
        for d in definitions.of(Registry::Schema) {
            let schema = Schema::from_data(&d.spec)
                .map_err(|e| ContentError::new(format!("{}.luau: state {}: {e}", d.module, d.key)))?;
            schemas.push(SchemaDef { key: d.key.clone(), schema });
        }
        schemas.sort_by(|a, b| a.key.cmp(&b.key));
        let schema_id = |key: &str| -> StateId {
            StateId(schemas.binary_search_by(|s| s.key.as_str().cmp(key)).expect("a defined schema") as u16)
        };
        let state_of = |d: &Definition| -> Result<StateId, ContentError> {
            match d.spec.field("state") {
                Data::Nil => Ok(schema_id(NO_STATE)),
                Data::Ref(Registry::Schema, key) => Ok(schema_id(key)),
                _ => Err(ContentError::new(format!("{}.luau: {} {}'s `state` is not a table", d.module, d.registry, d.key))),
            }
        };

        // Object kinds.
        let mut kinds = Entries::new(Registry::Kind);
        for (kind, key, pool) in ENGINE_KINDS {
            let def = KindDef {
                key: key.to_string(),
                pool,
                implementation: KindImpl::Engine(kind),
                schema: schema_id(NO_STATE),
                place: None,
                navi_left: None,
            };
            kinds.add(key.to_string(), def, "the engine's".into());
        }
        for d in definitions.of(Registry::Kind) {
            let pool = d
                .spec
                .field("pool")
                .str()
                .and_then(Pool::from_name)
                .ok_or_else(|| ContentError::new(format!("{}.luau: kind {} needs a `pool` (actor, attack or effect)", d.module, d.key)))?;
            let place = match d.spec.field("place") {
                Data::Nil => None,
                _ => Some(functions.id(slot(d, "place")?)),
            };
            let navi_left = match d.spec.field("navi_left") {
                Data::Nil => None,
                _ => Some(functions.id(slot(d, "navi_left")?)),
            };
            let def = KindDef {
                key: d.key.clone(),
                pool,
                implementation: KindImpl::Script { update: functions.id(slot(d, "update")?) },
                schema: state_of(d)?,
                place,
                navi_left,
            };
            kinds.add(d.key.clone(), def, format!("defined in {}.luau", d.module));
        }
        let kinds: Vec<KindDef> = kinds.sorted()?.into_iter().map(|(_, k)| k).collect();

        // The actions content defines.
        let mut actions = Entries::new(Registry::Action);
        for d in definitions.of(Registry::Action) {
            let def =
                ActionDef { key: d.key.clone(), update: functions.id(slot(d, "update")?), schema: state_of(d)? };
            actions.add(d.key.clone(), def, format!("defined in {}.luau", d.module));
        }
        let actions: Vec<ActionDef> = actions.sorted()?.into_iter().map(|(_, a)| a).collect();

        // Weapons: what content defines. A weapon names another by key
        // (`plain`), so their handles come first.
        let mut weapon_defs: Vec<&Definition> = definitions.of(Registry::Weapon).iter().collect();
        weapon_defs.sort_by(|a, b| a.key.cmp(&b.key));
        for w in weapon_defs.windows(2) {
            if w[0].key == w[1].key {
                return Err(ContentError::new(format!(
                    "weapon {:?} is defined in both {}.luau and {}.luau",
                    w[0].key, w[0].module, w[1].module
                )));
            }
        }
        let weapon_handle = |key: &str| -> Option<WeaponHandle> {
            weapon_defs.binary_search_by(|d| d.key.as_str().cmp(key)).ok().map(|i| WeaponHandle(i as u16))
        };
        let mut weapons = Vec::with_capacity(weapon_defs.len());
        for d in &weapon_defs {
            let what = |e: String| ContentError::new(format!("{}.luau: weapon {}: {e}", d.module, d.key));
            no_display_text(d)?;
            let charge_ticks: Vec<u16> = match d.spec.field("charge_ticks") {
                Data::List(items) => items
                    .iter()
                    .map(|t| match t {
                        Data::Int(i) if (0..=0xFFFF).contains(i) => Ok(*i as u16),
                        other => Err(what(format!("`charge_ticks` holds {other:?}, not a tick count"))),
                    })
                    .collect::<Result<_, _>>()?,
                other => return Err(what(format!("`charge_ticks` is {other:?}, not a list"))),
            };
            let flag = |field: &str| -> Result<bool, ContentError> {
                match d.spec.field(field) {
                    Data::Nil => Ok(false),
                    Data::Bool(b) => Ok(*b),
                    other => Err(what(format!("`{field}` is {other:?}, not true or false"))),
                }
            };
            let setup = match d.spec.field("setup") {
                Data::Nil => None,
                _ => Some(functions.id(slot(d, "setup")?)),
            };
            // Its own instant effect (`instant = function(user, spec)`),
            // which the instant chips' action runs.
            let instant = match d.spec.field("instant") {
                Data::Nil => None,
                _ => Some(functions.id(slot(d, "instant")?)),
            };
            let plain = match d.spec.field("plain") {
                Data::Nil => None,
                Data::Ref(Registry::Weapon, key) => {
                    Some(weapon_handle(key).ok_or_else(|| what(format!("`plain` names {key:?}, which is not a weapon")))?)
                }
                other => return Err(what(format!("`plain` is {other:?}, not a weapon"))),
            };
            let charged_chip = match d.spec.field("charged_chip") {
                Data::Nil => None,
                Data::Str(s) if s == "bonus" => Some(ChargedChip::Bonus),
                Data::Str(s) if s == "rock_barrage" => Some(ChargedChip::RockBarrage),
                other => return Err(what(format!("`charged_chip` is {other:?}, not \"bonus\" or \"rock_barrage\""))),
            };
            if charged_chip.is_some() && setup.is_some() {
                return Err(what("a weapon with a `setup` is its own attack: it has no `charged_chip`".into()));
            }
            if charged_chip.is_none() && setup.is_none() {
                return Err(what("a weapon needs a `setup` (or, an A-charge that is its chip, `charged_chip`): nothing could start it".into()));
            }
            weapons.push(WeaponDef {
                key: d.key.clone(),
                setup,
                charge_ticks,
                instant,
                instant_waits: flag("instant_waits")?,
                sticky: flag("sticky")?,
                held: flag("held")?,
                plain,
                charged_chip,
            });
        }
        // A navi's or form's weapons, by slot.
        let read_weapons = |d: &Definition| -> Result<super::FormWeapons, ContentError> {
            let what = |e: String| ContentError::new(format!("{}.luau: {} {}: {e}", d.module, d.registry, d.key));
            let mut out = super::FormWeapons::default();
            let entries = match d.spec.field("weapons") {
                Data::Nil => return Ok(out),
                Data::Map(entries) => entries,
                other => return Err(what(format!("`weapons` is {other:?}, not weapons by slot"))),
            };
            for (slot, v) in entries {
                let name = slot.to_string();
                let Some(at) = out.slot_mut(&name) else {
                    return Err(what(format!("`weapons` has no slot {name:?} (it has {})", super::FormWeapons::SLOTS.join(", "))));
                };
                *at = match v {
                    Data::Ref(Registry::Weapon, key) => {
                        Some(weapon_handle(key).ok_or_else(|| what(format!("weapons.{name} names {key:?}, which is not a weapon")))?)
                    }
                    other => return Err(what(format!("weapons.{name} is {other:?}, not a weapon"))),
                };
            }
            Ok(out)
        };

        // Chips content defines.
        let action_handle = |key: &str| -> Option<ActionHandle> {
            actions.binary_search_by(|a| a.key.as_str().cmp(key)).ok().map(|i| ActionHandle(i as u16))
        };
        let mut chips = Entries::new(Registry::Chip);
        // The chips content defines: each with exactly one use, its
        // `action`, `dimming`, `navi` or `instant`.
        for d in definitions.of(Registry::Chip) {
            let mut usages = Vec::new();
            match d.spec.field("action") {
                Data::Nil => {}
                Data::Ref(Registry::Action, key) => usages.push(ChipUsage::Action(action_handle(key).expect("a defined action"))),
                _ => return Err(ContentError::new(format!("{}.luau: chip {}'s `action` is not an action", d.module, d.key))),
            }
            for (field, usage) in [
                ("dimming", ChipUsage::Dimming as fn(FnId) -> ChipUsage),
                ("navi", ChipUsage::Navi),
                ("instant", ChipUsage::Instant),
            ] {
                if !d.spec.field(field).is_nil() {
                    usages.push(usage(functions.id(slot(d, field)?)));
                }
            }
            let [usage] = usages[..] else {
                return Err(ContentError::new(format!(
                    "{}.luau: chip {} needs exactly one of `action`, `dimming`, `navi` and `instant`",
                    d.module, d.key
                )));
            };
            let record = chip_record(d, &reader)?;
            let setup = match d.spec.field("setup") {
                Data::Nil => None,
                _ => Some(functions.id(slot(d, "setup")?)),
            };
            chips.add(
                d.key.clone(),
                ChipDef { key: d.key.clone(), record, usage, setup, links: ChipLinks::default() },
                format!("defined in {}.luau", d.module),
            );
        }
        let mut chips: Vec<ChipDef> = chips.sorted()?.into_iter().map(|(_, c)| c).collect();

        // What the chips' records name, by handle: the Program Advances
        // (their recipes' ingredients), a dark chip's substitute, an SP
        // navi chip's slot.
        let chip_handle = |key: &str, whose: &str| -> Result<ChipHandle, ContentError> {
            chips
                .binary_search_by(|c| c.key.as_str().cmp(key))
                .map(|i| ChipHandle(i as u16))
                .map_err(|_| ContentError::new(format!("{whose} names the chip {key:?}, which the content doesn't have")))
        };
        let mut advances: Vec<(u8, super::ProgramAdvance)> = Vec::new();
        let mut links = Vec::with_capacity(chips.len());
        let mut results = 0u8;
        for (i, c) in chips.iter().enumerate() {
            let whose = format!("chip {}", c.key);
            let mut l = ChipLinks::default();
            if let Some(super::DamageFormula::SpNavi { slot, .. }) = &c.record.formula {
                let rules = content.rules();
                let n = rules.sp_slots.iter().position(|s| s == slot).ok_or_else(|| {
                    ContentError::new(format!("{whose}'s damage is by the SP navi {slot:?}, which the rules' sp_slots don't list"))
                })?;
                l.sp_slot = Some(n as u8);
            }
            for r in &c.record.program_advances {
                let at = format!("{whose}'s recipe");
                let recipe = match &r.recipe {
                    super::PaRecipe::CodeRun { chip, count } => super::Recipe::CodeRun { chip: chip_handle(chip, &at)?, count: *count },
                    super::PaRecipe::Sequence(keys) => {
                        super::Recipe::Sequence(keys.iter().map(|k| chip_handle(k, &at)).collect::<Result<_, _>>()?)
                    }
                };
                let operation_battle_only = r.operation_battle_only;
                advances.push((r.order, super::ProgramAdvance { result: ChipHandle(i as u16), recipe, operation_battle_only }));
            }
            if !c.record.program_advances.is_empty() {
                // (A player's record of the round's formed ones is 64 bits:
                // EXE6's 30 and EXE5's 30 fit.)
                if results >= 64 {
                    return Err(ContentError::new(format!("{whose}: more than 64 chips are Program Advances")));
                }
                l.advance = Some(results);
                results += 1;
            }
            links.push(l);
        }
        for (c, l) in chips.iter_mut().zip(links) {
            c.links = l;
        }
        advances.sort_by_key(|(order, _)| *order);
        let program_advances: Vec<super::ProgramAdvance> = advances.into_iter().map(|(_, pa)| pa).collect();
        let chip_handle = |key: &str, whose: &str| -> Result<ChipHandle, ContentError> {
            chips
                .binary_search_by(|c| c.key.as_str().cmp(key))
                .map(|i| ChipHandle(i as u16))
                .map_err(|_| ContentError::new(format!("{whose} names the chip {key:?}, which the content doesn't have")))
        };

        // Navis, forms and identities: the definitions, in key order (a
        // definition's handle is its place among its registry's).
        // The identities, by key (the definitions' order). A navi's and a
        // form's name theirs: each knows whose it is.
        let mut identities = Vec::new();
        for d in definitions.of(Registry::Identity) {
            // (Its `parts.own` is a function: the identity's only one.)
            let own_part = match d.spec.field("parts") {
                parts @ Data::Map(_) => match parts.field("own") {
                    Data::Nil => None,
                    Data::Function => Some(functions.id(FnSource::slot(Registry::Identity, &d.key, "parts.own"))),
                    other => {
                        return Err(ContentError::new(format!("{}.luau: identity {}: `parts.own` is {other:?}, not a function", d.module, d.key)));
                    }
                },
                _ => None,
            };
            identities.push(super::identity::read(d, &content.assets, &definitions, own_part)?);
        }
        if identities.windows(2).any(|w| w[0].key >= w[1].key) {
            return Err(ContentError::new("the identities are not in key order (the define phase sorts each registry)"));
        }
        let identity_of = |d: &Definition, identities: &[super::Identity]| -> Result<Option<nettai_content_api::IdentityHandle>, ContentError> {
            match d.spec.field("identity") {
                Data::Nil => Ok(None),
                Data::Ref(Registry::Identity, key) => {
                    let i = identities.binary_search_by(|i| i.key.as_str().cmp(key)).expect("a defined identity");
                    Ok(Some(nettai_content_api::IdentityHandle(i as u16)))
                }
                other => Err(ContentError::new(format!(
                    "{}.luau: {} {}: `identity` is {other:?}, not an identity (define.identity {{ ... }})",
                    d.module, d.registry, d.key
                ))),
            }
        };
        let in_order = |registry: Registry| -> Result<(), ContentError> {
            if definitions.of(registry).windows(2).any(|w| w[0].key >= w[1].key) {
                return Err(ContentError::new(format!("the {registry}s are not in key order (the define phase sorts each registry)")));
            }
            Ok(())
        };
        in_order(Registry::Navi)?;
        in_order(Registry::Form)?;
        let handle_of = |registry: Registry, key: &str| -> u16 {
            definitions.of(registry).binary_search_by(|d| d.key.as_str().cmp(key)).expect("a defined entry") as u16
        };
        let what = |d: &Definition, e: String| ContentError::new(format!("{}.luau: {} {}: {e}", d.module, d.registry, d.key));
        let form_ref = |d: &Definition, v: &Data, field: &str| -> Result<Option<FormHandle>, ContentError> {
            match v {
                Data::Nil => Ok(None),
                Data::Ref(Registry::Form, key) => Ok(Some(FormHandle(handle_of(Registry::Form, key)))),
                other => Err(what(d, format!("`{field}` is {other:?}, not a form"))),
            }
        };
        let mut navis = Vec::new();
        for d in definitions.of(Registry::Navi) {
            let mut record = super::navis::read_navi(d, &reader)?;
            record.weapons = read_weapons(d)?;
            record.fresh = super::navis::read_fresh(d, |key| {
                definitions.of(Registry::Record).iter().position(|r| r.key == key).map(|i| RecordHandle(i as u16))
            })?;
            record.cross_hp = super::navis::read_cross_hp(d)?;
            record.levels = super::navis::read_levels(d, &weapon_handle)?;
            record.story = super::navis::read_story(d)?;
            record.identity = identity_of(d, &identities)?;
            record.forms = match d.spec.field("forms") {
                Data::Nil => None,
                forms @ Data::Map(_) => {
                    // The forms a form list offers, by version: each set of
                    // the table that lists them (`crosses`; the rest of a set
                    // is its game's: EXE6's Beast Out and Beast Over).
                    let mut by_version = Vec::new();
                    if let Data::Map(sets) = forms {
                        for (version, set) in sets {
                            let Data::List(items) = set.field("crosses") else { continue };
                            let version = version.to_string();
                            let listed = items
                                .iter()
                                .map(|v| form_ref(d, v, &format!("forms.{version}.crosses")).map(|f| f.expect("a form")))
                                .collect::<Result<Vec<_>, _>>()?;
                            by_version.push((version, listed));
                        }
                    }
                    let souls = match forms.field("souls") {
                        Data::Nil => Vec::new(),
                        Data::List(items) => items
                            .iter()
                            .map(|v| form_ref(d, v, "forms.souls").map(|f| f.expect("a form")))
                            .collect::<Result<_, _>>()?,
                        Data::Map(m) if m.is_empty() => Vec::new(),
                        other => return Err(what(d, format!("forms.souls is {other:?}, not a list of forms"))),
                    };
                    Some(super::NaviForms { souls, by_version })
                }
                other => return Err(what(d, format!("`forms` is {other:?}, not the forms by game"))),
            };
            let own_chip = match &record.own_chip {
                Some(c) => Some((chip_handle(&c.chip, &format!("navi {}'s own chip", d.key))?, c.code)),
                None => None,
            };
            let mut hook = |field: &str| -> Result<Option<FnId>, ContentError> {
                Ok(match d.spec.field(field) {
                    Data::Nil => None,
                    _ => Some(functions.id(slot(d, field)?)),
                })
            };
            let (tick, idle, post_init) = (hook("tick")?, hook("idle")?, hook("post_init")?);
            if record.forms.is_some() && tick.is_some() {
                return Err(what(d, "a navi that changes form has no `tick` of its own: its forms' `tick` run".into()));
            }
            navis.push(NaviDef { key: d.key.clone(), record, own_chip, tick, idle, post_init });
        }
        // (A round's record of the link navis' chips used is 32 bits.)
        if navis.len() > 32 {
            return Err(ContentError::new("more than 32 navis"));
        }
        // (Each navi's own chip knows its navi, and its identity whose it
        // is.)
        for (i, n) in navis.iter().enumerate() {
            if let Some((chip, _)) = n.own_chip {
                chips[chip.index()].links.own_chip_of = Some(NaviHandle(i as u16));
            }
            if let Some(h) = n.record.identity {
                claim_identity(&mut identities, h, super::IdentityOwner::Navi(NaviHandle(i as u16)), &n.key)?;
            }
        }
        let mut forms = Vec::new();
        for d in definitions.of(Registry::Form) {
            let extended = extended_fields(&definitions, Registry::Form);
            let mut record = super::navis::read_form(d, &reader, &extended.iter().map(String::as_str).collect::<Vec<_>>())?;
            record.weapons = read_weapons(d)?;
            record.identity = identity_of(d, &identities)?;
            record.breaks_to = form_ref(d, d.spec.field("breaks_to"), "breaks_to")?;
            record.change = match d.spec.field("change") {
                Data::Nil => None,
                Data::Ref(Registry::Action, key) => {
                    Some(ActionHandle(actions.binary_search_by(|a| a.key.as_str().cmp(key)).expect("a defined action") as u16))
                }
                other => return Err(what(d, format!("`change` is {other:?}, not an action"))),
            };
            record.revert = match d.spec.field("revert") {
                Data::Nil => None,
                Data::Ref(Registry::Action, key) => {
                    Some(ActionHandle(actions.binary_search_by(|a| a.key.as_str().cmp(key)).expect("a defined action") as u16))
                }
                other => return Err(what(d, format!("`revert` is {other:?}, not an action"))),
            };
            record.charged_action = match d.spec.field("charged_action") {
                Data::Nil => None,
                Data::Ref(Registry::Action, key) => {
                    Some(ActionHandle(actions.binary_search_by(|a| a.key.as_str().cmp(key)).expect("a defined action") as u16))
                }
                other => return Err(what(d, format!("`charged_action` is {other:?}, not an action"))),
            };
            if record.base && record.revert.is_some() {
                return Err(what(d, "a base form names no action that reverts a navi out of it (`revert`)".into()));
            }
            if !record.base && record.change.is_none() {
                return Err(what(d, "a form other than the base form names the action that changes a navi into it (`change`)".into()));
            }
            let mut hook = |field: &str| -> Result<Option<FnId>, ContentError> {
                Ok(match d.spec.field(field) {
                    Data::Nil => None,
                    _ => Some(functions.id(slot(d, field)?)),
                })
            };
            let (reset, put_on, take_off, tick) = (hook("reset")?, hook("put_on")?, hook("take_off")?, hook("tick")?);
            forms.push(FormDef { key: d.key.clone(), record, reset, put_on, take_off, tick });
        }
        for (i, f) in forms.iter().enumerate() {
            if let Some(h) = f.record.identity {
                claim_identity(&mut identities, h, super::IdentityOwner::Form(FormHandle(i as u16)), &f.key)?;
            }
        }
        // The game's base form: what its navis are in before they change
        // form; one.
        let mut base_form: Option<FormHandle> = None;
        for (i, f) in forms.iter().enumerate().filter(|(_, f)| f.record.base) {
            if let Some(other) = base_form {
                return Err(ContentError::new(format!(
                    "two forms of {game} are base forms ({} and {})",
                    forms[other.index()].key,
                    f.key
                )));
            }
            base_form = Some(FormHandle(i as u16));
        }
        // (What a game's forms and its navis' sets say of each other is its
        // systems': EXE6's are checked by exe6-compat's tests.)
        for f in &forms {
            if f.record.glow.as_ref().is_some_and(|g| g.is_empty()) {
                return Err(ContentError::new(format!("form {}'s `glow` has no shaders", f.key)));
            }
        }
        for n in &navis {
            if n.record.forms.is_none() {
                continue;
            }
            if base_form.is_none() {
                return Err(ContentError::new(format!("navi {} changes form, and no form of its game is the base form", n.key)));
            }
        }
        // Stages: what they place names kinds and their variant records.
        let mut stages = Entries::new(Registry::Stage);
        for d in definitions.of(Registry::Stage) {
            let record = super::stages::read(
                d,
                &content.assets,
                |key| kinds.iter().position(|k| k.key == key).map(|i| KindHandle(i as u16)),
                |key| definitions.of(Registry::Record).iter().position(|r| r.key == key).map(|i| RecordHandle(i as u16)),
            )?;
            for e in &record.actors {
                if let super::Place::Kind(k) = e.place
                    && kinds[k.index()].place.is_none()
                {
                    return Err(ContentError::new(format!(
                        "{}.luau: stage {} places the kind {}, which has no `place`",
                        d.module,
                        d.key,
                        kinds[k.index()].key
                    )));
                }
            }
            stages.add(d.key.clone(), StageDef { key: d.key.clone(), record }, format!("defined in {}.luau", d.module));
        }
        let stages: Vec<StageDef> = stages.sorted()?.into_iter().map(|(_, s)| s).collect();

        // The status effects, by key (the definitions' order).
        let mut statuses = Vec::new();
        for d in definitions.of(Registry::Status) {
            let what = |e: String| ContentError::new(format!("{}.luau: status {}: {e}", d.module, d.key));
            let mut spec = d.spec.clone();
            if let Data::Map(entries) = &mut spec {
                entries.retain(|(k, _)| !matches!(k, nettai_content_api::DataKey::Str(s) if s == "id"));
            }
            let effect: super::StatusEffect = reader.read(&spec, &d.key).map_err(what)?;
            statuses.push(StatusDef { key: d.key.clone(), effect });
        }
        if statuses.windows(2).any(|w| w[0].key >= w[1].key) {
            return Err(ContentError::new("the statuses are not in key order (the define phase sorts each registry)"));
        }

        // Effects, sparks, regions and collision types, by handle.
        let look = |d: &Definition| -> Result<super::EffectSprite, ContentError> {
            let what = |e: String| ContentError::new(format!("{}.luau: {} {}: {e}", d.module, d.registry, d.key));
            let sprite = match d.spec.field("sprite") {
                Data::Asset(nettai_content_api::AssetKind::Sprite, name) => {
                    SpriteId(content.assets.handle(nettai_content_api::AssetKind::Sprite, name).ok_or_else(|| what(format!("the packs have no sprite {name:?}")))?)
                }
                _ => return Err(what("needs a `sprite` (asset.sprite(...))".into())),
            };
            Ok(super::EffectSprite { sprite, anim: byte(d, "anim")?, palette: byte(d, "palette")? })
        };
        let effects = definitions.of(Registry::Effect).iter().map(look).collect::<Result<Vec<_>, _>>()?;
        let sparks = definitions.of(Registry::Spark).iter().map(look).collect::<Result<Vec<_>, _>>()?;
        let mut regions: Vec<super::Region> = Vec::new();
        for d in definitions.of(Registry::Region) {
            let what = |e: &str| ContentError::new(format!("{}.luau: region {}: {e}", d.module, d.key));
            regions.push(match (d.spec.field("panels"), d.spec.field("field")) {
                (Data::List(items), Data::Nil) => {
                    let mut panels = Vec::new();
                    for p in items {
                        let (Some(dx), Some(dy)) = (p.item(1).int(), p.item(2).int()) else {
                            return Err(what("each of `panels` is { dx, dy }"));
                        };
                        panels.push(super::PanelOffset { dx: dx as i8, dy: dy as i8 });
                    }
                    super::Region::Panels(panels)
                }
                (Data::Nil, Data::Map(_)) => {
                    let word = |k: &str| d.spec.field("field").field(k).int().unwrap_or(0) as u32;
                    super::Region::Field(super::PanelCondition { require: word("require"), forbid: word("forbid") })
                }
                _ => return Err(what("needs exactly one of `panels` and `field`")),
            });
        }
        let mut collisions = Vec::new();
        for d in definitions.of(Registry::Collision) {
            let word = |k: &str| -> Result<u32, ContentError> {
                d.spec.field(k).int().map(|i| i as u32).ok_or_else(|| {
                    ContentError::new(format!("{}.luau: collision {} needs `{k}` (a flag word)", d.module, d.key))
                })
            };
            let row_offset = d.spec.field("row_offset").int().unwrap_or(0) as u16;
            collisions.push(CollisionTypeDef { flags: [word("side0")?, word("side1")?], row_offset });
        }

        // The roles: the game's, its ruleset's `roles` (docs/design/
        // content-model-v2.md §7.4).
        let roles = match ruleset(&definitions) {
            Some(d) => read_roles(d, &definitions, &content.assets, &actions, &kinds, &chips, &statuses, &mut functions)?,
            None => Roles::default(),
        };

        // The systems and the ruleset.
        let mut systems = Vec::new();
        // (Each system's `unplayable_in_auto_battle`, by chip id, with its module
        // and key: in the systems' order.)
        let mut unplayable_in_auto_battle: Vec<(String, String, Vec<(String, String)>)> = Vec::new();
        let mut buttons: Vec<ButtonDef> = Vec::new();
        let mut windows: Vec<WindowDef> = Vec::new();
        for d in definitions.of(Registry::System) {
            let what = |e: &str| ContentError::new(format!("{}.luau: system {}: {e}", d.module, d.key));
            if let Data::Map(entries) = &d.spec {
                for (k, _) in entries {
                    if !matches!(k, nettai_content_api::DataKey::Str(f) if ["id", "state", "setup", "setup_defaults", "navi_state", "hooks", "custom", "buttons", "windows", "actions", "extends", "unplayable_in_auto_battle"].contains(&f.as_str())) {
                        return Err(what(&format!("`{k}` is no field of a system (id, state, setup, setup_defaults, navi_state, hooks, custom, buttons, windows, actions, extends, unplayable_in_auto_battle)")));
                    }
                }
            }
            let layout = |field: &str| -> Result<StateId, ContentError> {
                match d.spec.field(field) {
                    Data::Nil => Ok(schema_id(NO_STATE)),
                    Data::Ref(Registry::Schema, key) => Ok(schema_id(key)),
                    _ => Err(what(&format!("`{field}` is not a table of fields"))),
                }
            };
            let mut hooks = vec![None; SystemHook::ALL.len()];
            match d.spec.field("hooks") {
                Data::Nil => {}
                Data::Map(entries) => {
                    for (k, v) in entries {
                        let name = k.to_string();
                        let Some(i) = SystemHook::ALL.iter().position(|h| h.name() == name && !name.contains('.')) else {
                            let known: Vec<&str> = SystemHook::ALL.iter().map(|h| h.name()).filter(|n| !n.contains('.')).collect();
                            return Err(what(&format!("no hook is named `{name}` (the hooks: {})", known.join(", "))));
                        };
                        if !matches!(v, Data::Function) {
                            return Err(what(&format!("hook `{name}` is not a function")));
                        }
                        hooks[i] = Some(functions.id(FnSource::slot(Registry::System, &d.key, &format!("hooks.{name}"))));
                    }
                }
                _ => return Err(what("`hooks` is a table of functions by hook name")),
            }
            // Its custom screen's (docs/design/rules-in-luau.md §4.4): the
            // hooks named `custom.<name>`.
            match d.spec.field("custom") {
                Data::Nil => {}
                Data::Map(entries) => {
                    for (k, v) in entries {
                        let name = format!("custom.{k}");
                        let Some(i) = SystemHook::ALL.iter().position(|h| h.name() == name) else {
                            let known: Vec<&str> =
                                SystemHook::ALL.iter().filter_map(|h| h.name().strip_prefix("custom.")).collect();
                            return Err(what(&format!("no custom-screen hook is named `{k}` (the hooks: {})", known.join(", "))));
                        };
                        if !matches!(v, Data::Function) {
                            return Err(what(&format!("custom-screen hook `{k}` is not a function")));
                        }
                        hooks[i] = Some(functions.id(FnSource::slot(Registry::System, &d.key, &name)));
                    }
                }
                _ => return Err(what("`custom` is a table of functions by custom-screen hook name")),
            }
            let own: &[Data] = match d.spec.field("actions") {
                Data::Nil => &[],
                Data::List(items) => items,
                Data::Map(m) if m.is_empty() => &[],
                _ => return Err(what("`actions` is a list of actions")),
            };
            let mut system_actions = Vec::new();
            for v in own {
                let Data::Ref(Registry::Action, key) = v else {
                    return Err(what("`actions` lists action definitions (define.action { ... })"));
                };
                let h = actions.binary_search_by(|a| a.key.as_str().cmp(key)).expect("a defined action");
                system_actions.push(ActionHandle(h as u16));
            }
            // Its custom-screen buttons, by name.
            let system = SystemHandle(systems.len() as u16);
            let mut own_buttons = Vec::new();
            match d.spec.field("buttons") {
                Data::Nil => {}
                Data::Map(entries) => {
                    for (k, spec) in entries {
                        let name = k.to_string();
                        let at = |e: &str| what(&format!("button `{name}`: {e}"));
                        let Data::Map(fields) = spec else { return Err(at("a table of its place and functions")) };
                        for (f, _) in fields {
                            let f = f.to_string();
                            if !["slot", "cells", "uses", "right", "left", "view", "shown", "state", "pressed", "taken_back", "chip"].contains(&f.as_str()) {
                                return Err(at(&format!("`{f}` is no field of a button (slot, cells, uses, right, left, view, shown, state, pressed, taken_back, chip)")));
                            }
                        }
                        let view = match spec.field("view") {
                            Data::Nil => None,
                            Data::Str(v) => Some(ButtonView::named(v).ok_or_else(|| {
                                let known: Vec<&str> = ButtonView::ALL.iter().map(|v| v.name()).collect();
                                at(&format!("`view` is {v:?}: a button's view is one of {}", known.join(", ")))
                            })?),
                            _ => return Err(at("`view` is a view's name")),
                        };
                        let byte = |f: &str| -> Result<Option<u8>, ContentError> {
                            match spec.field(f) {
                                Data::Nil => Ok(None),
                                v => v.int().filter(|n| (0..=255).contains(n)).map(|n| Some(n as u8)).ok_or_else(|| at(&format!("`{f}` is a number"))),
                            }
                        };
                        let mut func = |f: &str, needed: bool| -> Result<Option<FnId>, ContentError> {
                            match spec.field(f) {
                                Data::Function => Ok(Some(functions.id(FnSource::slot(Registry::System, &d.key, &format!("buttons.{name}.{f}"))))),
                                Data::Nil if !needed => Ok(None),
                                _ => Err(at(&format!("`{f}` is a function"))),
                            }
                        };
                        let shown = func("shown", true)?.expect("needed");
                        let state = func("state", false)?;
                        let pressed = func("pressed", true)?.expect("needed");
                        let taken_back = func("taken_back", false)?;
                        let chip = func("chip", false)?;
                        let slot = byte("slot")?.ok_or_else(|| at("`slot` is missing"))?;
                        let cells = byte("cells")?.unwrap_or(1);
                        if !(1..=2).contains(&cells) {
                            return Err(at("`cells` is 1 or 2"));
                        }
                        let (uses, right, left) = (byte("uses")?.unwrap_or(0), byte("right")?, byte("left")?);
                        own_buttons.push(ButtonHandle((buttons.len()) as u16));
                        buttons.push(ButtonDef { system, name: name.clone(), view, slot, cells, uses, right, left, shown, state, pressed, taken_back, chip });
                    }
                }
                _ => return Err(what("`buttons` is a table of buttons by name")),
            }
            let navi_state = match d.spec.field("navi_state") {
                Data::Nil => None,
                _ => Some(layout("navi_state")?),
            };
            let controller = SystemHook::ALL.iter().position(|&h| h == SystemHook::Controller).expect("listed");
            if navi_state.is_some() && hooks[controller].is_none() {
                return Err(what("a `navi_state` is the state of the navis its `controller` drives: it has no `controller`"));
            }
            // Its custom-screen windows, by name.
            let mut own_windows = Vec::new();
            match d.spec.field("windows") {
                Data::Nil => {}
                Data::Map(entries) => {
                    for (k, spec) in entries {
                        let name = k.to_string();
                        let Data::Map(fields) = spec else {
                            return Err(what(&format!("window `{name}`: a table with its `update`")));
                        };
                        if let Some((f, _)) = fields.iter().find(|(f, _)| !["update", "view"].contains(&f.to_string().as_str())) {
                            return Err(what(&format!("window `{name}`: `{f}` is no field of a window (view, update)")));
                        }
                        let view = match spec.field("view") {
                            Data::Nil => None,
                            Data::Str(v) => Some(WindowView::named(v).ok_or_else(|| {
                                let known: Vec<&str> = WindowView::ALL.iter().map(|v| v.name()).collect();
                                what(&format!("window `{name}`: `view` is {v:?}: a window's view is one of {}", known.join(", ")))
                            })?),
                            _ => return Err(what(&format!("window `{name}`: `view` is a view's name"))),
                        };
                        if !matches!(spec.field("update"), Data::Function) {
                            return Err(what(&format!("window `{name}`: `update` is a function")));
                        }
                        let update = functions.id(FnSource::slot(Registry::System, &d.key, &format!("windows.{name}.update")));
                        own_windows.push(WindowHandle(windows.len() as u16));
                        windows.push(WindowDef { system, name, view, update });
                    }
                }
                _ => return Err(what("`windows` is a table of windows by name")),
            }
            // Its extensions of definitions: by registry, each field's type.
            let mut extends = Vec::new();
            match d.spec.field("extends") {
                Data::Nil => {}
                Data::Map(registries) => {
                    for (k, fields) in registries {
                        let name = k.to_string();
                        let Some(registry) = Registry::from_name(&name).filter(|r| matches!(r, Registry::Chip | Registry::Form | Registry::Navi))
                        else {
                            return Err(what(&format!("`extends.{name}`: a system extends chip, form or navi definitions")));
                        };
                        let Data::Map(fields) = fields else { return Err(what(&format!("`extends.{name}` is a table of fields")) ) };
                        for (f, ty) in fields {
                            let field = f.to_string();
                            let ty = ExtensionType::read(ty, &format!("extends.{name}.{field}")).map_err(|e| what(&e))?;
                            extends.push(Extension { registry, field, ty });
                        }
                    }
                }
                _ => return Err(what("`extends` is a table of fields by registry (chip, form, navi)")),
            }
            // The chips it can't play of a player's auto battle data, by id, each
            // with why (the ids are resolved once every chip is known).
            let mut unplayable = Vec::new();
            match d.spec.field("unplayable_in_auto_battle") {
                Data::Nil => {}
                Data::Map(entries) => {
                    for (k, why) in entries {
                        let (nettai_content_api::DataKey::Str(id), Data::Str(why)) = (k, why) else {
                            return Err(what("`unplayable_in_auto_battle` is a table of sentences by chip id"));
                        };
                        unplayable.push((id.clone(), why.clone()));
                    }
                }
                _ => return Err(what("`unplayable_in_auto_battle` is a table of sentences by chip id")),
            }
            unplayable.sort();
            unplayable_in_auto_battle.push((d.module.clone(), d.key.clone(), unplayable));
            // Its setup's defaults: a value of a field of its setup each
            // (an enum's by name), which it must hold as given.
            let setup = layout("setup")?;
            let mut setup_default = nettai_content_api::ContentState::new(setup);
            // (An enum has no default but one given below: unstated.)
            for i in 0..schemas[setup.0 as usize].schema.fields().len() {
                setup_default.unstate(&schemas[setup.0 as usize].schema, i);
            }
            match d.spec.field("setup_defaults") {
                Data::Nil => {}
                Data::Map(entries) => {
                    let schema = &schemas[setup.0 as usize].schema;
                    // One value of a field (an array's element): a flag,
                    // a number, an enum's name, a definition by its id.
                    let value_of = |v: &Data, ty: &nettai_content_api::FieldType| -> Result<nettai_content_api::Value, String> {
                        use nettai_content_api::{FieldType, Value};
                        Ok(match (v, ty) {
                            (Data::Bool(b), _) => Value::Bool(*b),
                            (Data::Int(n), _) => Value::Int(*n),
                            (Data::Str(n), FieldType::Enum(names)) => {
                                Value::Int(names.iter().position(|x| x == n).ok_or_else(|| format!("no variant {n:?}"))? as i64)
                            }
                            (Data::Str(key) | Data::Ref(_, key), FieldType::Ref(registry, _))
                                if !matches!(v, Data::Ref(r, _) if r != registry) =>
                            {
                                let place = definitions.of(*registry).binary_search_by(|d| d.key.as_str().cmp(key));
                                Value::Def(*registry, place.map_err(|_| format!("the content has no {registry} {key:?}"))? as u16)
                            }
                            (other, ty) => return Err(format!("{other:?} is no value of a {ty:?} field")),
                        })
                    };
                    for (k, v) in entries {
                        let name = k.to_string();
                        let at = |e: String| what(&format!("setup_defaults.{name}: {e}"));
                        let i = schema.index_of(&name).ok_or_else(|| at("the setup has no such field".into()))?;
                        match (v, &schema.field(i).ty) {
                            // An array's, a list: its elements from the
                            // first (the rest stay zero, none).
                            (Data::List(items), nettai_content_api::FieldType::Array(elem, n)) => {
                                if items.len() > *n as usize {
                                    return Err(at(format!("{} values, and the field holds {n}", items.len())));
                                }
                                for (place, item) in items.iter().enumerate() {
                                    let value = value_of(item, elem).map_err(&at)?;
                                    setup_default.set_elem(schema, i, place, value).map_err(&at)?;
                                    if setup_default.get_elem(schema, i, place).map(|x| x.load()) != Some(value) {
                                        return Err(at(format!("{value:?} doesn't fit the field's elements")));
                                    }
                                }
                            }
                            (v, ty) => {
                                let value = value_of(v, ty).map_err(&at)?;
                                setup_default.set(schema, i, value).map_err(|e| at(e.to_string()))?;
                                if setup_default.get(schema, i).load() != value {
                                    return Err(at(format!("{value:?} doesn't fit the field")));
                                }
                            }
                        }
                    }
                }
                _ => return Err(what("`setup_defaults` is a table of the setup's fields")),
            }
            // What its windows' and buttons' views show of its state: the
            // fields, each by the name the view gives it.
            let state = layout("state")?;
            let views = ViewFields::of(
                &schemas[state.0 as usize].schema,
                own_windows.iter().filter_map(|h| windows[h.0 as usize].view.map(|v| (windows[h.0 as usize].name.as_str(), v))),
                own_buttons.iter().filter_map(|h| buttons[h.0 as usize].view.map(|v| (buttons[h.0 as usize].name.as_str(), v))),
            )
            .map_err(|e| what(&e))?;
            systems.push(SystemDef {
                key: d.key.clone(),
                state,
                setup,
                setup_default,
                navi_state,
                hooks,
                actions: system_actions,
                buttons: own_buttons,
                windows: own_windows,
                views,
                extends,
                unplayable_in_auto_battle: Vec::new(),
            });
        }
        // The extensions: one system of the game owns a field of a registry,
        // and the definitions that carry it carry it of its type.
        for (i, s) in systems.iter().enumerate() {
            for e in &s.extends {
                if let Some(other) = systems[..i].iter().find(|o| o.extends.iter().any(|x| x.registry == e.registry && x.field == e.field)) {
                    return Err(ContentError::new(format!(
                        "systems {} and {} both extend {} definitions with `{}`",
                        other.key,
                        s.key,
                        e.registry,
                        e.field
                    )));
                }
                for d in definitions.of(e.registry) {
                    e.ty.check(d.spec.field(&e.field), &format!("{} {}.{}", e.registry, d.key, e.field))
                        .map_err(|m| ContentError::new(format!("{}.luau: {m} (system {}'s extension)", d.module, s.key)))?;
                }
            }
        }
        // Each action's system, if it is one's.
        let mut action_owner: Vec<Option<SystemHandle>> = vec![None; actions.len()];
        for (i, s) in systems.iter().enumerate() {
            for &a in &s.actions {
                if let Some(other) = action_owner[a.index()] {
                    return Err(ContentError::new(format!(
                        "action {} is both system {}'s and system {}'s",
                        actions[a.index()].key,
                        systems[other.index()].key,
                        s.key
                    )));
                }
                action_owner[a.index()] = Some(SystemHandle(i as u16));
            }
        }
        // The actions forms name as their change (and their revert):
        // unpaused, they are the instant chips' action (the original's
        // CurAction 0x1C, EXE5's 0x1A).
        let mut change_actions = vec![false; actions.len()];
        for f in &forms {
            for a in [f.record.change, f.record.revert].into_iter().flatten() {
                change_actions[a.index()] = true;
            }
        }
        let ruleset = read_ruleset(&definitions)?;
        // What a player brings that a frontend reads by the engine's name
        // for it: the first of the ruleset's systems whose setup has a
        // field of the name; every system's field of the name must be of
        // the fact's type (a setup writes the fact into each).
        let mut facts = vec![None; PlayerFact::ALL.len()];
        for (k, fact) in PlayerFact::ALL.iter().enumerate() {
            let listed = ruleset.iter().flat_map(|r| r.systems.iter().enumerate());
            for (slot, h) in listed {
                let s = &systems[h.index()];
                let schema = &schemas[s.setup.0 as usize].schema;
                let Some(i) = schema.index_of(fact.name()) else { continue };
                if let Err(want) = fact.fits(&schema.field(i).ty) {
                    let module = &definitions.of(Registry::System)[h.index()].module;
                    return Err(ContentError::new(format!(
                        "{module}.luau: system {}: its setup field `{}` is the fact a player brings by that name, {want}: it is {:?}",
                        s.key,
                        fact.name(),
                        schema.field(i).ty
                    )));
                }
                facts[k].get_or_insert((slot as u8, i as u16));
            }
        }

        let records: Vec<RecordDef> = definitions
            .of(Registry::Record)
            .iter()
            .map(|d| RecordDef { key: d.key.clone(), record_type: d.record_type.clone().unwrap_or_default() })
            .collect();
        // The lock-on modes (`ho_8026554`'s, which `navi:lockon_panel` reads):
        // the records of type "lockon", by handle.
        let mut lockons = Vec::new();
        for (i, d) in definitions.of(Registry::Record).iter().enumerate().filter(|(_, d)| d.record_type.as_deref() == Some(LOCKON_RECORD)) {
            let what = |e: String| ContentError::new(format!("{}.luau: lockon record {}: {e}", d.module, d.key));
            let mut spec = d.spec.clone();
            if let Data::Map(entries) = &mut spec {
                entries.retain(|(k, _)| !matches!(k, nettai_content_api::DataKey::Str(s) if s == "id"));
            }
            let mode: super::LockonMode = reader.read(&spec, &d.key).map_err(what)?;
            lockons.push(LockonDef { record: RecordHandle(i as u16), key: d.key.clone(), mode });
        }

        // The patch cards: their capacity cost and their effects' kinds
        // (what the effects do is a game's rules').
        let mut patch_cards = Vec::new();
        for d in definitions.of(Registry::PatchCard) {
            let what = |e: &str| ContentError::new(format!("{}.luau: patch_card {}: {e}", d.module, d.key));
            no_display_text(d)?;
            let mb = d.spec.field("mb").int().filter(|mb| (0..=0xFF).contains(mb)).ok_or_else(|| what("needs `mb` (0-255)"))?;
            let Data::List(list) = d.spec.field("effects") else {
                return Err(what("needs `effects`, a list of effects (a table each, with its `kind`)"));
            };
            let mut effects = Vec::with_capacity(list.len());
            for (i, e) in list.iter().enumerate() {
                let Data::Str(kind) = e.field("kind") else {
                    return Err(what(&format!("effect {} has no `kind` (a string)", i + 1)));
                };
                let bug = match e.field("bug") {
                    Data::Nil => false,
                    Data::Bool(b) => *b,
                    _ => return Err(what(&format!("effect {}'s `bug` is not a boolean", i + 1))),
                };
                effects.push(PatchCardEffect { kind: kind.clone(), bug });
            }
            patch_cards.push(PatchCardDef { key: d.key.clone(), mb: mb as u8, effects });
        }

        // The NaviCust programs: their colors and shapes (what they do is a
        // game's rules').
        let mut navicust_programs = Vec::new();
        for d in definitions.of(Registry::NaviCustProgram) {
            let what = |e: &str| ContentError::new(format!("{}.luau: navicust_program {}: {e}", d.module, d.key));
            no_display_text(d)?;
            let Data::List(list) = d.spec.field("colors") else {
                return Err(what("needs `colors`, the colors it comes in (strings), in its variants' order"));
            };
            let colors: Vec<String> = list.iter().map(|c| c.str().map(str::to_string)).collect::<Option<_>>().ok_or_else(|| what("a color is a string"))?;
            if colors.is_empty() || colors.len() > 0xFF {
                return Err(what("comes in 1 to 255 colors"));
            }
            let plus = match d.spec.field("plus") {
                Data::Nil => false,
                Data::Bool(b) => *b,
                _ => return Err(what("`plus` is a boolean")),
            };
            let shape = read_shape(d.spec.field("shape")).map_err(|e| what(&format!("shape: {e}")))?;
            let compressed = match d.spec.field("compressed") {
                Data::Nil => None,
                c => Some(read_shape(c).map_err(|e| what(&format!("compressed: {e}")))?),
            };
            navicust_programs.push(NaviCustProgramDef { key: d.key.clone(), colors, plus, shape, compressed });
        }

        // Each definition's handle.
        fn position<'a>(mut keys: impl Iterator<Item = &'a String>, key: &str) -> u16 {
            keys.position(|k| k == key).expect("an entry") as u16
        }
        let mut handles = Vec::with_capacity(definitions.defs.len());
        let mut in_registry = 0u16;
        let mut last = None;
        for d in &definitions.defs {
            if last != Some(d.registry) {
                (last, in_registry) = (Some(d.registry), 0);
            }
            handles.push(match d.registry {
                Registry::Kind => position(kinds.iter().map(|k| &k.key), &d.key),
                Registry::Action => position(actions.iter().map(|a| &a.key), &d.key),
                Registry::Weapon => position(weapons.iter().map(|w| &w.key), &d.key),
                Registry::Chip => position(chips.iter().map(|c| &c.key), &d.key),
                Registry::Schema => schema_id(&d.key).0,
                _ => in_registry,
            });
            in_registry += 1;
        }

        // Lookups by key.
        let kind_keys: BTreeMap<String, KindHandle> =
            kinds.iter().enumerate().map(|(i, k)| (k.key.clone(), KindHandle(i as u16))).collect();
        let engine = ENGINE_KINDS.iter().map(|e| kind_keys[e.1]).collect();
        let formula_chips =
            chips.iter().enumerate().filter(|(_, c)| c.record.formula.is_some()).map(|(i, _)| ChipHandle(i as u16)).collect();
        let mut defs = Defs {
            defined: true,
            formula_chips,
            definitions,
            handles,
            kind_keys,
            engine,
            base_form,
            chip_keys: BTreeMap::new(),
            weapon_keys: BTreeMap::new(),
            game: game.clone(),
            kinds: Vec::new(),
            actions,
            weapons,
            chips,
            navis,
            forms,
            stages,
            lockons,
            identities,
            statuses,
            records,
            patch_cards,
            navicust_programs,
            effects,
            sparks,
            regions,
            collisions,
            roles,
            systems,
            buttons,
            windows,
            ruleset,
            facts,
            action_owner,
            change_actions,
            program_advances,
            schemas,
            functions: Vec::new(),
        };
        defs.kinds = kinds;
        for (i, w) in defs.weapons.iter().enumerate() {
            defs.weapon_keys.insert(w.key.clone(), WeaponHandle(i as u16));
        }
        for (i, c) in defs.chips.iter().enumerate() {
            defs.chip_keys.insert(c.key.clone(), ChipHandle(i as u16));
        }
        // (A system's `unplayable_in_auto_battle` names chips of its game.)
        for (system, (module, key, unplayable)) in defs.systems.iter_mut().zip(unplayable_in_auto_battle) {
            for (id, why) in unplayable {
                let chip = defs.chip_keys.get(&id).copied().ok_or_else(|| {
                    ContentError::new(format!("{module}.luau: system {key}: `unplayable_in_auto_battle` names {id}, which is no chip of the game"))
                })?;
                system.unplayable_in_auto_battle.push((chip, why));
            }
        }
        defs.functions = functions.list;
        Ok(defs)
    }
}

/// The game's ruleset (docs/design/rules-in-luau.md §2.2): its systems, in
/// order. A game has one, with no name and no variants.
fn read_ruleset(definitions: &Definitions) -> Result<Option<RulesetDef>, ContentError> {
    let Some(d) = ruleset(definitions) else { return Ok(None) };
    let systems = definitions.of(Registry::System);
    let what = |e: &str| ContentError::new(format!("{}.luau: ruleset: {e}", d.module));
    if let Data::Map(entries) = &d.spec {
        for (k, _) in entries {
            let nettai_content_api::DataKey::Str(f) = k else {
                return Err(what(&format!("`{k}` is no field of a ruleset (systems, roles and the rule sections)")));
            };
            match f.as_str() {
                "systems" | "roles" => {}
                "stock" => return Err(what("`stock`: a game has one ruleset, which is its rules; it says nothing of being the stock one")),
                "base" | "add" | "remove" => {
                    return Err(what(&format!("`{f}`: a game has one ruleset, and no variants of it; it lists its `systems`")));
                }
                f if super::sections::SECTIONS.contains(&f) => {}
                f => {
                    return Err(what(&format!(
                        "`{f}` is no field of a ruleset (systems, roles; its rule sections {})",
                        super::sections::SECTIONS.join(", ")
                    )));
                }
            }
        }
    }
    let items: &[Data] = match d.spec.field("systems") {
        Data::Nil => &[],
        Data::List(items) => items,
        Data::Map(m) if m.is_empty() => &[],
        _ => return Err(what("`systems` is a list of systems")),
    };
    let mut out: Vec<SystemHandle> = Vec::new();
    for v in items {
        let Data::Ref(Registry::System, key) = v else {
            return Err(what("`systems` lists system definitions (define.system { ... })"));
        };
        let h = SystemHandle(systems.iter().position(|s| &s.key == key).expect("a defined system") as u16);
        if out.contains(&h) {
            return Err(what(&format!("`systems` lists system {key} twice")));
        }
        out.push(h);
    }
    if out.len() > MAX_SYSTEMS {
        return Err(what(&format!("{} systems; a ruleset has at most {MAX_SYSTEMS}", out.len())));
    }
    Ok(Some(RulesetDef { systems: out }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every module of the EXE6 content (content/exe6) loads in the define
    /// phase: it passes the bytecode check, runs, and what it defines reads
    /// back as data.
    #[test]
    fn every_exe6_module_loads_in_the_define_phase() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/exe6");
        let mut c = Content::default();
        c.scripts = crate::content::Scripts::dir("exe6", crate::content::testing::modules_under(dir));
        // (With a manifest for EXE6 and the support pack its modules
        // require, content/exelib.)
        crate::content::testing::add_shared(&mut c.scripts);
        crate::content::testing::add_index(&mut c.scripts, "exe6");
        c.assets = crate::content::testing::asset_names_for(&c.scripts);
        assert!(c.scripts.modules.len() > 200, "{} modules", c.scripts.modules.len());
        c.define().unwrap_or_else(|e| panic!("content/exe6: {e}"));
        // Every chip is a definition with its own use.
        assert!(c.defs.chips.len() > 300, "{} chips", c.defs.chips.len());
        // Effects, sparks, regions and collision types are definitions
        // the engine holds by handle.
        assert!(c.defs.effects.len() > 100, "{} effects defined", c.defs.effects.len());
        assert!(!c.defs.sparks.is_empty() && !c.defs.regions.is_empty() && c.defs.collisions.len() >= 89);
    }
}
