//! What the content defines (docs/design/content-model-v2.md): the
//! registries the engine reads, with their handles, and the functions the
//! runtime binds.
//!
//! A registry holds two sorts of entries:
//!
//! - the engine's own (its object kinds, keyed `engine/...`);
//! - content's definitions (the root's, and `new.kind { ... }`'s), keyed by their keys.
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
    ActionHandle, ChipHandle, ContentError, Data, Definition, Definitions, EntryHandle, FnId, FnSource, FormHandle, KindHandle,
    NaviHandle, Pool, RecordHandle, Registry, Schema, StageHandle, StateId, RulesHook,
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

/// What a chip's record names by key or name, resolved: the rules read
/// these.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChipLinks {
    /// The navi whose own chip this is (a link navi's chip: offered on the
    /// custom screen once a round, never charged, dropped from the hand
    /// when a round ends).
    pub own_chip_of: Option<NaviHandle>,
    /// Its place among the Program Advances' results (those chips with a
    /// recipe, in handle order): the bit a formed one takes in a player's
    /// record of the round.
    pub advance: Option<u8>,
}

/// A navi (the root's `navis`).
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
    /// Its functions of its side, which the round's setup asks for the side
    /// it is the navi of (`Battle::given`).
    pub given: NaviGivenFns,
}

/// A navi's functions of its side (`HookCall::Given`): what its level gives
/// it, as its game's rules read the level (EXE6's link navis', by their
/// navi code's level: content/exe6/rules/by_level.luau).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviGivenFns {
    /// `chip_bonus.damage`: its bonus on its family's chips
    /// (`NaviData::chip_bonus`; nil none).
    pub chip_bonus: Option<FnId>,
    /// `charged_chips.when`: whether its `charged_chips` charge (without
    /// it, always).
    pub charges: Option<FnId>,
    /// `fire_charge`: how far its A charge builds up the next Fire chip's
    /// damage (`sub_80F0608`; nil none).
    pub fire_charge: Option<FnId>,
}

/// The record type of a lock-on mode (`new.record("lockon", ...)`).
pub const LOCKON_RECORD: &str = "lockon";

/// One of MegaMan's forms (the root's `forms`).
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

/// A stage (the root's `stages`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StageDef {
    pub key: String,
    pub record: super::StageData,
}

/// A status effect (`new.status`).
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

/// A collision type content defines (`new.collision`): what an object is
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


/// The rules' extension of a registry's definitions
/// (docs/design/rules-in-luau.md §7.5, S7): a field its game's definitions
/// may carry for it (EXE6's rules/dark_chips's `hp_bug` on a chip), of a
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
    /// The type `v` declares (`at`: where, for errors), the game's
    /// collections `collections` (an entry of one: its name).
    fn read(v: &Data, at: &str, collections: &[&str]) -> Result<ExtensionType, String> {
        use nettai_content_api::FieldType;
        match v {
            Data::Str(name) => FieldType::scalar(name)
                .filter(|ty| unknown_collection(ty, collections).is_none())
                .map(ExtensionType::Value)
                .ok_or_else(|| format!("{at}: no type is named {name:?}")),
            Data::List(variants) if !variants.is_empty() => {
                let names = variants
                    .iter()
                    .map(|v| v.str().map(str::to_string).ok_or_else(|| format!("{at}: variants are names")))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ExtensionType::Value(FieldType::Enum(names)))
            }
            Data::Map(fields) => Ok(ExtensionType::Table(
                fields
                    .iter()
                    .map(|(k, v)| Ok((k.to_string(), ExtensionType::read(v, &format!("{at}.{k}"), collections)?)))
                    .collect::<Result<_, String>>()?,
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

/// A game's rules (docs/design/rules-in-luau.md §2.5), its one definition of
/// them (its root's `rules`, rules/init.luau): their state of a side,
/// a player's setup of them (the side's facts), and their hooks, plain
/// functions that call the game's modules as their code says. A game has
/// one, which every match of it plays by.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RulesDef {
    /// The layout of their state of a side, and of a player's setup.
    pub state: StateId,
    pub setup: StateId,
    /// Its player setup when the player's setup says nothing of a field:
    /// its `setup_defaults` (EXE5's light/dark value a fresh save's 500; an
    /// array's, a list: EXE5's souls, every one),
    /// zero elsewhere (a list: empty), but an enum, which has no default unless
    /// `setup_defaults` gives it one: it is left unstated
    /// (`Block::unstate`: EXE6's player's version, gregar or falzar),
    /// and a round doesn't start until the player's setup states it
    /// ([`RulesDef::setup_block`], `SideRules::for_player`).
    pub setup_default: nettai_content_api::Block,
    /// The layout of its state of each navi no player controls that it
    /// drives (its `controller`: the navi object's own state), if it
    /// drives any.
    pub navi_state: Option<StateId>,
    /// Its hooks, in [`RulesHook::ALL`]'s order.
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
    /// The functions of its `panes` (what a tool shows of a side's setup,
    /// pane by pane; the editor's), by their paths there
    /// (`panes.2.fields.1.board`): the engine calls one only for a tool
    /// (`Battle::call_pane`).
    pub pane_functions: BTreeMap<String, FnId>,
}

impl RulesDef {
    /// The function at `path` of its `panes`, if one is there.
    pub fn pane_function(&self, path: &str) -> Option<FnId> {
        self.pane_functions.get(path).copied()
    }

    /// A player's setup block of them when the player gives none: their
    /// defaults (`setup_defaults`), the rest zero, an enum without a
    /// default unstated.
    pub fn setup_block(&self) -> nettai_content_api::Block {
        self.setup_default.clone()
    }

    /// Its function for `hook`, if it has one.
    pub fn hook(&self, hook: RulesHook) -> Option<FnId> {
        let i = RulesHook::ALL.iter().position(|&h| h == hook).expect("every hook is listed");
        self.hooks[i]
    }
}

/// A custom-screen button of the rules' (docs/design/rules-in-luau.md
/// §4.4): where it sits, and its functions, which run as the rules' calls.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ButtonDef {
    /// Its name in the rules' `buttons` (the key a pack has its look
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

/// A custom-screen window of the rules' (docs/design/rules-in-luau.md
/// §4.4): its update, run as the rules' call each tick it is up.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowDef {
    /// Its name in the rules' `windows` (what `custom.open_window` opens
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

/// A record only content reads: the engine keeps its handle and type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordDef {
    pub key: String,
    pub record_type: String,
}

/// An entry of one of the game's collections (`Registry::Entry`: a table its
/// root holds by id under a key the core doesn't know, EXE6's `patch_cards`
/// and `navicust_programs`): data only content reads. The engine keeps its
/// key (`<collection>/<id>`) and its collection; a match names it by its id,
/// the locales and the library order by its key there.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EntryDef {
    pub key: String,
    pub collection: String,
}

impl EntryDef {
    /// Its id in its collection (`canodumb`).
    pub fn id(&self) -> &str {
        nettai_content_api::entry_parts(&self.key).1
    }
}

/// The collection a type (at any depth) holds an entry of that the game's
/// root hasn't (`collections`): a type name that is neither one of the
/// core's nor a collection's.
fn unknown_collection(ty: &nettai_content_api::FieldType, collections: &[&str]) -> Option<String> {
    use nettai_content_api::FieldType;
    match ty {
        FieldType::Ref(Registry::Entry, Some(c)) => (!collections.contains(&c.as_str())).then(|| c.clone()),
        FieldType::Array(elem, _) | FieldType::List(elem, _) => unknown_collection(elem, collections),
        FieldType::Record(fields) => fields.fields().iter().find_map(|f| unknown_collection(&f.ty, collections)),
        _ => None,
    }
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
    /// The identities, by handle (`new.identity`, in key order).
    pub identities: Vec<super::Identity>,
    /// The status effects, by handle.
    pub statuses: Vec<StatusDef>,
    pub records: Vec<RecordDef>,
    /// The entries of the game's collections, by handle (key order: a
    /// collection's together).
    pub entries: Vec<EntryDef>,
    /// One-shot effects' and hit sparks' looks (`new.effect`,
    /// `new.spark`), by handle.
    pub effects: Vec<super::EffectSprite>,
    pub sparks: Vec<super::EffectSprite>,
    /// Hit regions (`new.region`), by handle.
    pub regions: Vec<super::Region>,
    /// Collision types (`new.collision`), by handle.
    pub collisions: Vec<CollisionTypeDef>,
    /// What the game needs from content by role (its rules' `roles`;
    /// none given, none filled).
    pub roles: Roles,
    /// The rules' custom-screen buttons and windows.
    pub buttons: Vec<ButtonDef>,
    pub windows: Vec<WindowDef>,
    /// The game's rules (none: content without them, whose sides have no
    /// rules' state).
    rules: Option<RulesDef>,
    /// Where each fact a player brings is ([`PlayerFact::ALL`]'s order):
    /// the field of the rules' setup.
    facts: Vec<Option<u16>>,
    /// Whether an action is the rules' (it reaches their state), by action
    /// handle.
    rules_actions: Vec<bool>,
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

    pub fn button(&self, h: ButtonHandle) -> &ButtonDef {
        &self.buttons[h.0 as usize]
    }

    pub fn window(&self, h: WindowHandle) -> &WindowDef {
        &self.windows[h.0 as usize]
    }

    /// Whether an action is the rules' (it reaches their state).
    pub fn is_rules_action(&self, h: ActionHandle) -> bool {
        self.rules_actions.get(h.index()).copied().unwrap_or(false)
    }

    /// Whether a form names the action as the one that changes a navi into
    /// it.
    pub fn is_change_action(&self, h: ActionHandle) -> bool {
        self.change_actions.get(h.index()).copied().unwrap_or(false)
    }

    /// The game's rules, if it has them (docs/design/rules-in-luau.md §2.3:
    /// a game has one definition of them, which every match of it plays
    /// by).
    pub fn rules(&self) -> Option<&RulesDef> {
        self.rules.as_ref()
    }

    /// Where fact `fact` is: the field of the rules' setup. None: the
    /// game's rules take no such fact.
    pub fn fact_field(&self, fact: PlayerFact) -> Option<usize> {
        let k = PlayerFact::ALL.iter().position(|&f| f == fact).expect("every fact is listed");
        self.facts.get(k).copied().flatten().map(|field| field as usize)
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

    /// The entry with this key (`patch_cards/canodumb`).
    pub fn entry_by_key(&self, key: &str) -> Option<EntryHandle> {
        self.entries.binary_search_by(|e| e.key.as_str().cmp(key)).ok().map(|i| EntryHandle(i as u16))
    }

    /// Entry `id` of collection `collection`.
    pub fn entry_in(&self, collection: &str, id: &str) -> Option<EntryHandle> {
        self.entry_by_key(&nettai_content_api::entry_key(collection, id))
    }

    pub fn entry(&self, h: EntryHandle) -> &EntryDef {
        &self.entries[h.index()]
    }

    /// The entries of collection `collection`, in key order (none: the
    /// game's root holds no such collection).
    pub fn entries_of(&self, collection: &str) -> Vec<EntryHandle> {
        (0..self.entries.len() as u16).map(EntryHandle).filter(|&h| self.entry(h).collection == collection).collect()
    }

    /// The game's collections, by name, in order.
    pub fn collections(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.entries.iter().map(|e| e.collection.as_str()).collect();
        out.dedup();
        out
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

/// Every function of `d`'s data `data` (at `path` in it), registered, by
/// its path (a list's items from 1, as in Luau).
fn functions_under(d: &Definition, data: &Data, path: &str, functions: &mut Functions, out: &mut BTreeMap<String, FnId>) {
    match data {
        Data::Function => {
            out.insert(path.to_string(), functions.id(FnSource::slot(d.registry, &d.key, path)));
        }
        Data::List(items) => {
            for (i, x) in items.iter().enumerate() {
                functions_under(d, x, &format!("{path}.{}", i + 1), functions, out);
            }
        }
        Data::Map(entries) => {
            for (k, x) in entries {
                functions_under(d, x, &format!("{path}.{k}"), functions, out);
            }
        }
        _ => {}
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

/// The fields the game's rules extend its definitions of `registry` with
/// (`extends`), read off the rules' definition before it is built: what a
/// record's reader leaves to them. (Building the rules checks them.)
fn extended_fields(definitions: &Definitions, registry: Registry) -> Vec<String> {
    let mut fields = Vec::new();
    if let Some(r) = rules_definition(definitions)
        && let Data::Map(own) = r.spec.field("extends").field(registry.name())
    {
        fields.extend(own.iter().map(|(k, _)| k.to_string()));
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
        // (A damage a function of the side gives is the chip's function:
        // `Defs::build` takes it.)
        if field == "damage" && matches!(spec.field(field), Data::Function) {
            o.insert(field.into(), default);
            continue;
        }
        let v = json(field)?;
        o.insert(field.into(), if v.is_null() { default } else { v });
    }
    // `damage`: a number, or a formula (`{ formula = "hp_lost" }`), or a
    // function of the side.
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
    // What the rules ask of this chip: its traits and the trap it is.
    // (The rules' own fields, EXE6's dark chips' cost and substitute and its
    // Beast rush's lock-on, are its extension: SystemDef::extends.)
    for field in ["traits", "trap"] {
        let v = json(field)?;
        if !v.is_null() {
            o.insert(field.into(), v);
        }
    }
    o.insert("program_advance".into(), json("program_advances")?);
    if o["program_advance"].is_null() {
        o.insert("program_advance".into(), Json::Array(Vec::new()));
    }
    serde_json::from_value(Json::Object(o)).map_err(|e| what(e.to_string()))
}

/// The game's rules (its root's `rules`, rules/init.luau),
/// which holds its rule sections and its roles; none for content without
/// one. (A game has one: the define phase refuses two.)
pub(crate) fn rules_definition(definitions: &Definitions) -> Option<&Definition> {
    definitions.of(Registry::Rules).first()
}

/// The rules' `roles = { actions = { ... }, kinds = { ... } }`
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
    let what = |e: String| ContentError::new(format!("{}.luau: rules: roles: {e}", d.module));
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
                        what(format!("the rules have no role actions.{name} (it has {})", names.join(", ")))
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
                        what(format!("the rules have no role kinds.{name} (it has {})", names.join(", ")))
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
                        what(format!("the rules have no role hooks.{name} (it has {})", names.join(", ")))
                    })?;
                    if !matches!(v, Data::Function) {
                        return Err(what(format!("hooks.{name} is not a function")));
                    }
                    roles.hooks.insert(role, functions.id(FnSource::slot(Registry::Rules, &d.key, &format!("roles.hooks.{name}"))));
                }
                "chips" => {
                    let names: Vec<&str> = ChipRole::ALL.iter().map(|r| r.name()).collect();
                    let role = ChipRole::named(&name).ok_or_else(|| {
                        what(format!("the rules have no role chips.{name} (it has {})", names.join(", ")))
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
                        what(format!("the rules have no role statuses.{name} (it has {})", names.join(", ")))
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
                        "the rules have no role group `{group}` (it has actions, kinds, hooks, chips, lockon, statuses, effects, sparks, regions, collision, sounds, music, sprites, banners)"
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
        format!("the rules have no role {group}.{name} (it has {})", names.collect::<Vec<_>>().join(", "))
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
        // (The game's collections, which a field may hold an entry of.)
        let collections: Vec<&str> = {
            let mut c: Vec<&str> = definitions.of(Registry::Entry).iter().filter_map(|d| d.record_type.as_deref()).collect();
            c.dedup();
            c
        };
        for d in definitions.of(Registry::Schema) {
            let schema = Schema::from_data(&d.spec)
                .map_err(|e| ContentError::new(format!("{}.luau: state {}: {e}", d.module, d.key)))?;
            for f in schema.fields() {
                if let Some(c) = unknown_collection(&f.ty, &collections) {
                    return Err(ContentError::new(format!(
                        "{}.luau: state {}: `{}`: no type is named {c:?} (nor is it a collection of the game's root)",
                        d.module, d.key, f.name
                    )));
                }
            }
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
            let mut record = chip_record(d, &reader)?;
            if matches!(d.spec.field("damage"), Data::Function) {
                record.formula = Some(super::DamageFormula::Given(functions.id(slot(d, "damage")?)));
            }
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
        // (their recipes' ingredients), a dark chip's substitute.
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
                    "{}.luau: {} {}: `identity` is {other:?}, not an identity (new.identity {{ ... }})",
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
            let (tick, idle, post_init, fire_charge) = (hook("tick")?, hook("idle")?, hook("post_init")?, hook("fire_charge")?);
            if record.forms.is_some() && tick.is_some() {
                return Err(what(d, "a navi that changes form has no `tick` of its own: its forms' `tick` run".into()));
            }
            let given = NaviGivenFns {
                chip_bonus: match d.spec.field("chip_bonus").field("damage") {
                    Data::Function => Some(functions.id(FnSource::slot(d.registry, &d.key, "chip_bonus.damage"))),
                    _ if record.chip_bonus.is_some() => return Err(what(d, "`chip_bonus.damage` is a function of the side".into())),
                    _ => None,
                },
                charges: match d.spec.field("charged_chips").field("when") {
                    Data::Nil => None,
                    Data::Function => Some(functions.id(FnSource::slot(d.registry, &d.key, "charged_chips.when"))),
                    _ => return Err(what(d, "`charged_chips.when` is a function of the side".into())),
                },
                fire_charge,
            };
            navis.push(NaviDef { key: d.key.clone(), record, own_chip, tick, idle, post_init, given });
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

        // The roles: the game's, its rules' `roles` (docs/design/
        // content-model-v2.md §7.4).
        let roles = match rules_definition(&definitions) {
            Some(d) => read_roles(d, &definitions, &content.assets, &actions, &kinds, &chips, &statuses, &mut functions)?,
            None => Roles::default(),
        };

        // The game's rules (one definition: their state, setup, hooks,
        // buttons and windows, actions and extensions, beside their rule
        // sections and roles).
        let mut rules = None;
        let mut buttons: Vec<ButtonDef> = Vec::new();
        let mut windows: Vec<WindowDef> = Vec::new();
        if let Some(d) = rules_definition(&definitions) {
            let what = |e: &str| ContentError::new(format!("{}.luau: rules: {e}", d.module));
            const FIELDS: [&str; 12] =
                ["state", "setup", "setup_defaults", "navi_state", "hooks", "custom", "buttons", "windows", "actions", "extends", "roles", "panes"];
            if let Data::Map(entries) = &d.spec {
                for (k, _) in entries {
                    let nettai_content_api::DataKey::Str(f) = k else {
                        return Err(what(&format!("`{k}` is no field of the rules")));
                    };
                    if !FIELDS.contains(&f.as_str()) && !super::sections::SECTIONS.contains(&f.as_str()) {
                        return Err(what(&format!(
                            "`{f}` is no field of the rules ({}; their rule sections {})",
                            FIELDS.join(", "),
                            super::sections::SECTIONS.join(", ")
                        )));
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
            let mut hooks = vec![None; RulesHook::ALL.len()];
            match d.spec.field("hooks") {
                Data::Nil => {}
                Data::Map(entries) => {
                    for (k, v) in entries {
                        let name = k.to_string();
                        let Some(i) = RulesHook::ALL.iter().position(|h| h.name() == name && !name.contains('.')) else {
                            let known: Vec<&str> = RulesHook::ALL.iter().map(|h| h.name()).filter(|n| !n.contains('.')).collect();
                            return Err(what(&format!("no hook is named `{name}` (the hooks: {})", known.join(", "))));
                        };
                        if !matches!(v, Data::Function) {
                            return Err(what(&format!("hook `{name}` is not a function")));
                        }
                        hooks[i] = Some(functions.id(FnSource::slot(d.registry, &d.key, &format!("hooks.{name}"))));
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
                        let Some(i) = RulesHook::ALL.iter().position(|h| h.name() == name) else {
                            let known: Vec<&str> =
                                RulesHook::ALL.iter().filter_map(|h| h.name().strip_prefix("custom.")).collect();
                            return Err(what(&format!("no custom-screen hook is named `{k}` (the hooks: {})", known.join(", "))));
                        };
                        if !matches!(v, Data::Function) {
                            return Err(what(&format!("custom-screen hook `{k}` is not a function")));
                        }
                        hooks[i] = Some(functions.id(FnSource::slot(d.registry, &d.key, &name)));
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
                    return Err(what("`actions` lists action definitions (new.action { ... })"));
                };
                let h = actions.binary_search_by(|a| a.key.as_str().cmp(key)).expect("a defined action");
                system_actions.push(ActionHandle(h as u16));
            }
            // Their custom-screen buttons, by name.
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
                                Data::Function => Ok(Some(functions.id(FnSource::slot(d.registry, &d.key, &format!("buttons.{name}.{f}"))))),
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
                        buttons.push(ButtonDef { name: name.clone(), view, slot, cells, uses, right, left, shown, state, pressed, taken_back, chip });
                    }
                }
                _ => return Err(what("`buttons` is a table of buttons by name")),
            }
            // (A navi's state, as an object's.)
            let navi_state = match d.spec.field("navi_state") {
                Data::Nil => None,
                _ => Some(layout("navi_state")?),
            };
            let controller = RulesHook::ALL.iter().position(|&h| h == RulesHook::Controller).expect("listed");
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
                        let update = functions.id(FnSource::slot(d.registry, &d.key, &format!("windows.{name}.update")));
                        own_windows.push(WindowHandle(windows.len() as u16));
                        windows.push(WindowDef { name, view, update });
                    }
                }
                _ => return Err(what("`windows` is a table of windows by name")),
            }
            // Its panes' functions, by their paths (a tool's to call).
            let mut pane_functions = BTreeMap::new();
            functions_under(d, d.spec.field("panes"), "panes", &mut functions, &mut pane_functions);
            // Its extensions of definitions: by registry, each field's type.
            let mut extends = Vec::new();
            match d.spec.field("extends") {
                Data::Nil => {}
                Data::Map(registries) => {
                    for (k, fields) in registries {
                        let name = k.to_string();
                        let Some(registry) = Registry::from_name(&name).filter(|r| matches!(r, Registry::Chip | Registry::Form | Registry::Navi))
                        else {
                            return Err(what(&format!("`extends.{name}`: the rules extend chip, form or navi definitions")));
                        };
                        let Data::Map(fields) = fields else { return Err(what(&format!("`extends.{name}` is a table of fields")) ) };
                        for (f, ty) in fields {
                            let field = f.to_string();
                            let ty = ExtensionType::read(ty, &format!("extends.{name}.{field}"), &collections).map_err(|e| what(&e))?;
                            extends.push(Extension { registry, field, ty });
                        }
                    }
                }
                _ => return Err(what("`extends` is a table of fields by registry (chip, form, navi)")),
            }
            // Its setup's defaults: a value of a field of its setup each
            // (an enum's by name), which it must hold as given.
            let setup = layout("setup")?;
            let mut setup_default = nettai_content_api::Block::new(setup, &schemas[setup.0 as usize].schema);
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
                    // A value at a place: an array's or a list's elements
                    // from the first (an array's rest stay zero; a list
                    // holds what is given), a record's fields by name.
                    fn write(
                        block: &mut nettai_content_api::Block,
                        place: nettai_content_api::Place,
                        v: &Data,
                        value_of: &dyn Fn(&Data, &nettai_content_api::FieldType) -> Result<nettai_content_api::Value, String>,
                    ) -> Result<(), String> {
                        use nettai_content_api::FieldType;
                        match (v, place.ty()) {
                            (Data::List(items), FieldType::Array(..) | FieldType::List(..)) => {
                                let n = place.capacity().expect("an array or a list");
                                if items.len() > n {
                                    return Err(format!("{} values, and the field holds {n}", items.len()));
                                }
                                if let FieldType::List(..) = place.ty() {
                                    block.set_len_at(place, items.len())?;
                                }
                                for (k, item) in items.iter().enumerate() {
                                    write(block, place.elem(k).expect("within the field's room"), item, value_of).map_err(|e| format!("[{}]: {e}", k + 1))?;
                                }
                                Ok(())
                            }
                            (Data::Map(entries), FieldType::Record(fields)) => {
                                for (k, v) in entries {
                                    let name = k.to_string();
                                    let at = place.field(&name).ok_or_else(|| format!("the record has no field `{name}` ({})", fields.fields().iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", ")))?;
                                    write(block, at, v, value_of).map_err(|e| format!(".{name}: {e}"))?;
                                }
                                Ok(())
                            }
                            (v, ty) if ty.is_scalar() => {
                                let value = value_of(v, ty)?;
                                block.set_at(place, value).map_err(|e| e.to_string())?;
                                if block.get_at(place).load() != value {
                                    return Err(format!("{value:?} doesn't fit the field"));
                                }
                                Ok(())
                            }
                            (other, ty) => Err(format!("{other:?} is no value of a {ty} field")),
                        }
                    }
                    for (k, v) in entries {
                        let name = k.to_string();
                        let at = |e: String| what(&format!("setup_defaults.{name}: {e}"));
                        let i = schema.index_of(&name).ok_or_else(|| at("the setup has no such field".into()))?;
                        write(&mut setup_default, schema.place(i), v, &value_of).map_err(|e| {
                            // (An element's or a field's error names its place.)
                            if e.starts_with('[') || e.starts_with('.') {
                                what(&format!("setup_defaults.{name}{e}"))
                            } else {
                                at(e)
                            }
                        })?;
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
            rules = Some(RulesDef {
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
                pane_functions,
            });
        }
        // The rules' extensions: the definitions that carry a field carry it
        // of its type; and the rules' actions.
        let mut rules_actions = vec![false; actions.len()];
        if let Some(r) = &rules {
            let module = rules_definition(&definitions).map_or("", |d| d.module.as_str());
            for (i, e) in r.extends.iter().enumerate() {
                if r.extends[..i].iter().any(|x| x.registry == e.registry && x.field == e.field) {
                    return Err(ContentError::new(format!("{module}.luau: rules: they extend {} definitions with `{}` twice", e.registry, e.field)));
                }
                for d in definitions.of(e.registry) {
                    e.ty.check(d.spec.field(&e.field), &format!("{} {}.{}", e.registry, d.key, e.field))
                        .map_err(|m| ContentError::new(format!("{}.luau: {m} (the rules' extension)", d.module)))?;
                }
            }
            for &a in &r.actions {
                rules_actions[a.index()] = true;
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
        // What a player brings that a frontend and tools read by the
        // engine's name for it: the rules' setup field of the name, of the
        // fact's type.
        let mut facts = vec![None; PlayerFact::ALL.len()];
        if let Some(r) = &rules {
            let schema = &schemas[r.setup.0 as usize].schema;
            for (k, fact) in PlayerFact::ALL.iter().enumerate() {
                let Some(i) = schema.index_of(fact.name()) else { continue };
                if let Err(want) = fact.fits(&schema.field(i).ty) {
                    let module = rules_definition(&definitions).map_or("", |d| d.module.as_str());
                    return Err(ContentError::new(format!(
                        "{module}.luau: rules: their setup field `{}` is the fact a player brings by that name, {want}: it is {:?}",
                        fact.name(),
                        schema.field(i).ty
                    )));
                }
                facts[k] = Some(i as u16);
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

        // The entries of the game's collections: data only content reads.
        let mut entries = Vec::new();
        for d in definitions.of(Registry::Entry) {
            no_display_text(d)?;
            let collection = d.record_type.clone().expect("an entry's record type is its collection");
            entries.push(EntryDef { key: d.key.clone(), collection });
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
            entries,
            effects,
            sparks,
            regions,
            collisions,
            roles,
            buttons,
            windows,
            rules,
            facts,
            rules_actions,
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
        defs.functions = functions.list;
        Ok(defs)
    }
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
        // (Its own top module, whose root holds its collections by their
        // names: a held game's is made by folder.)
        let top = nettai_content_api::packs::top_module("exe6");
        c.scripts.modules.insert(top, std::fs::read_to_string(format!("{dir}/init.luau")).expect("content/exe6/init.luau"));
        c.assets = crate::content::testing::asset_names_for(&c.scripts);
        assert!(c.scripts.modules.len() > 200, "{} modules", c.scripts.modules.len());
        c.define().unwrap_or_else(|e| panic!("content/exe6: {e}"));
        // Every chip is a definition with its own use.
        assert!(c.defs.chips.len() > 300, "{} chips", c.defs.chips.len());
        // Effects, sparks, regions and collision types are definitions
        // the engine holds by handle.
        assert!(c.defs.effects.len() > 100, "{} effects defined", c.defs.effects.len());
        assert!(!c.defs.sparks.is_empty() && !c.defs.regions.is_empty() && c.defs.collisions.len() >= 88);
    }
}
