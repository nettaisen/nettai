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
//! action's, and the validator (`bn6-compat`) maps them to the original's
//! numbers with compat (docs/design/content-model-v2.md §7.3).

use std::collections::BTreeMap;

use nettai_content_api::SpriteId;

use nettai_content_api::{
    ActionHandle, ChipHandle, ContentError, Data, Definition, Definitions, FnId, FnSource, FormHandle, KindHandle,
    NaviHandle, Pool, RecordHandle, Registry, RulesetHandle, Schema, StageHandle, StateId, SystemHandle, SystemHook,
    WeaponHandle, keys,
};

use super::{
    ChipData, Content,
    FormData, NaviData,
};
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
    /// does to it (`kind.navi_left`; BN5's DethPhnx): the navi calls
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
    /// What its record names, by handle.
    pub links: ChipLinks,
}

/// What a chip's record names by key or name, resolved: the ruleset reads
/// these.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChipLinks {
    /// A dark chip's substitute (`ChipData::dark_substitute`).
    pub dark_substitute: Option<ChipHandle>,
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
}

/// One of MegaMan's forms (`define.form`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FormDef {
    pub key: String,
    pub record: FormData,
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

/// A Beast Out lock-on mode (`define.lockon`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LockonDef {
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


/// A system of a game's rules (docs/design/rules-in-luau.md §2.2): its state
/// per side, its player setup, its hooks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SystemDef {
    pub key: String,
    /// The layout of its state of a side, and of its player setup.
    pub state: StateId,
    pub setup: StateId,
    /// Its hooks, in [`SystemHook::ALL`]'s order.
    hooks: Vec<Option<FnId>>,
    /// Its own actions, which reach its state.
    pub actions: Vec<ActionHandle>,
    /// Its custom-screen buttons and windows.
    pub buttons: Vec<ButtonHandle>,
    pub windows: Vec<WindowHandle>,
}

impl SystemDef {
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
    /// Its name in the system's `buttons` (what the frontend draws it by).
    pub name: String,
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
}

/// A custom-screen window of a system's (docs/design/rules-in-luau.md
/// §4.4): its update, run as its system's each tick it is up.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowDef {
    pub system: SystemHandle,
    /// Its name in the system's `windows` (what the frontend draws it by).
    pub name: String,
    pub update: FnId,
}

/// A window, by its place in [`Defs::windows`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowHandle(pub u16);

/// A button, by its place in [`Defs::buttons`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ButtonHandle(pub u16);

/// A player's rules (docs/design/rules-in-luau.md §2.2): its systems, in
/// the order the framework calls them, and its game, whose data (roles,
/// rule sections) is the side's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RulesetDef {
    pub key: String,
    /// The game's own rules: what a player has unless their setup names
    /// another.
    pub stock: bool,
    pub systems: Vec<SystemHandle>,
    /// The ruleset it was made from (a mix's `base`), if any.
    pub base: Option<RulesetHandle>,
    /// Its game (§2.3): the root its `game` names, else its base's game,
    /// else its own root (which must then be a game's: one with a stock
    /// ruleset).
    pub game: RootId,
    /// Its own rule sections (P1 item 8: a mix's, over its game's for the
    /// sides that play by it; only the sections about a side), by id.
    pub sections: Vec<String>,
}

/// A game, by its place among the content's games (`Defs::roots`, by
/// name: a definition's game is its id's prefix, docs/design/
/// rules-in-luau.md, the flat namespace). A game's data, its roles and rule
/// sections, is its own (§2.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RootId(pub u8);

impl RootId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Most systems a ruleset may list.
pub const MAX_SYSTEMS: usize = 16;

/// A record only content reads: the engine keeps its handle and type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordDef {
    pub key: String,
    pub record_type: String,
}

/// A patch card (`define.patch_card`; BN4's, BN5's and BN6's Modification
/// Cards, docs/engine/patch-cards.md): what every game's card is. A game's
/// rules give its effects their meaning (BN6's patch-cards system applies a
/// player's cards as the round is set up); the engine keeps the card's
/// capacity cost and its effects' kinds, and the effects' own fields stay
/// the definition's data, which the rules read. Its name is the locales'.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PatchCardDef {
    pub key: String,
    /// Its capacity cost (BN6's MB): what the installed cards' limit counts.
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

/// A NaviCust program (`define.navicust_program`; BN4's, BN5's and BN6's
/// Navi Customizer parts, docs/design/navicust.md): what every game's
/// program is. The engine keeps what a NaviCust's board needs of it, its
/// colors and shapes and whether it is a plus part; the rest (BN6's: what it
/// does, which bug it brings, which programs it excludes) is the
/// definition's data, which the game's rules read. Its name is the
/// locales'.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NaviCustProgramDef {
    pub key: String,
    /// The colors it comes in (BN6's `white`, `yellow`, `pink`, `red`,
    /// `blue`, `green`), in its variants' order: a placed program's color
    /// is an index into them.
    pub colors: Vec<String>,
    /// A plus part (BN6: one that belongs off the command line).
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
    /// What each root's game needs from content by role (`define.roles`),
    /// by [`RootId`] (a root that defines none has none filled).
    pub roles: Vec<Roles>,
    /// The systems and rulesets (docs/design/rules-in-luau.md), by handle.
    pub systems: Vec<SystemDef>,
    /// The systems' custom-screen buttons and windows.
    pub buttons: Vec<ButtonDef>,
    pub windows: Vec<WindowDef>,
    pub rulesets: Vec<RulesetDef>,
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
    /// Each game's base form, by `RootId`: what its navis are in before
    /// they change form (rules-in-luau.md P1 item 12: BN5's MegaMan's is
    /// BN5's own); None for a game that defines none.
    pub base_forms: Vec<Option<FormHandle>>,
    /// Keys by registry, for the codecs.
    chip_keys: BTreeMap<String, ChipHandle>,
    weapon_keys: BTreeMap<String, WeaponHandle>,
    /// The roots the content came from, by name, its own first
    /// (`Scripts::roots`): what an unqualified key is looked up in.
    pub roots: Vec<String>,
}

impl Defs {
    pub fn system(&self, h: SystemHandle) -> &SystemDef {
        &self.systems[h.index()]
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

    pub fn ruleset(&self, h: RulesetHandle) -> &RulesetDef {
        &self.rulesets[h.index()]
    }

    /// Game `root`'s stock ruleset, if it has one (docs/design/
    /// rules-in-luau.md §2.3: what a player has unless their setup names
    /// another, the stage's game's).
    pub fn stock_ruleset_of(&self, root: &str) -> Option<RulesetHandle> {
        self.rulesets
            .iter()
            .position(|r| r.stock && keys::root_of(&r.key) == Some(root))
            .map(|i| RulesetHandle(i as u16))
    }

    /// The root named `name`.
    pub fn root_id(&self, name: &str) -> Option<RootId> {
        self.roots.iter().position(|r| r == name).map(|i| RootId(i as u8))
    }

    /// The game a definition's id is of: its prefix's (None for the
    /// engine's own keys, `engine/...`).
    pub fn root_of(&self, key: &str) -> Option<RootId> {
        keys::root_of(key).and_then(|r| self.root_id(r))
    }

    /// Game `root`'s roles.
    pub fn roles(&self, root: RootId) -> &Roles {
        &self.roles[root.index()]
    }

    /// The game of the ruleset a setup names, or else `default`'s (the
    /// stage's game): whose data the side reads.
    pub fn ruleset_game(&self, ruleset: Option<RulesetHandle>, default: RootId) -> RootId {
        ruleset.map_or(default, |r| self.ruleset(r).game)
    }

    /// The ruleset with this key.
    pub fn ruleset_by_key(&self, key: &str) -> Option<RulesetHandle> {
        self.rulesets.iter().position(|r| r.key == key).map(|i| RulesetHandle(i as u16))
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
    pub fn lockon_by_key(&self, key: &str) -> Option<nettai_content_api::LockonHandle> {
        self.lockons.binary_search_by(|l| l.key.as_str().cmp(key)).ok().map(|i| nettai_content_api::LockonHandle(i as u16))
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
    // What the ruleset asks of this chip: its traits, the trap it is, a
    // dark chip's cost and its substitute (a chip, by key here).
    for field in ["traits", "trap", "hp_bug", "dark_substitute"] {
        let v = json(field)?;
        if !v.is_null() {
            o.insert(field.into(), v);
        }
    }
    let library = spec.field("library");
    for (field, from) in [("library_number", "number"), ("library_index", "index"), ("sort_key", "sort")] {
        o.insert(field.into(), library.field(from).int().unwrap_or(0).into());
    }
    let beast = spec.field("beast");
    o.insert("beast_lockon".into(), Json::Bool(!beast.is_nil() && !matches!(beast.field("rush"), Data::Bool(false))));
    // (A lock-on mode reads as its handle: `SpecReader::new`.)
    let mode = match beast.field("lockon") {
        Data::Nil => Json::Null,
        v @ Data::Ref(Registry::Lockon, _) => r.json(v, &format!("chip {}.beast.lockon", d.key)).map_err(what)?,
        _ => return Err(what("`beast.lockon` is a lock-on mode (rules/lockon)".into())),
    };
    o.insert("lockon_mode".into(), mode);
    o.insert("program_advance".into(), json("program_advances")?);
    if o["program_advance"].is_null() {
        o.insert("program_advance".into(), Json::Array(Vec::new()));
    }
    serde_json::from_value(Json::Object(o)).map_err(|e| what(e.to_string()))
}

/// `define.roles { actions = { ... }, kinds = { ... } }` (content::roles):
/// each role a definition.
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
    let what = |e: String| ContentError::new(format!("{}.luau: roles: {e}", d.module));
    let Data::Map(groups) = &d.spec else { return Err(what("a table of role groups".into())) };
    let mut roles = Roles::default();
    for (group, entries) in groups {
        let group = group.to_string();
        // (Its id names its game: `bn6:roles`.)
        if group == "id" {
            continue;
        }
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
                    roles.hooks.insert(role, functions.id(FnSource::slot(Registry::Roles, &d.key, &format!("hooks.{name}"))));
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
        // The games, by name (content without scripts is one game of no
        // name).
        let root_names = Content::game_names(&content.scripts, &definitions);
        let root_of = |key: &str| -> RootId {
            let name = keys::root_of(key).unwrap_or("");
            RootId(root_names.iter().position(|r| r == name).unwrap_or(0) as u8)
        };
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
            chips.add(
                d.key.clone(),
                ChipDef { key: d.key.clone(), record, usage, links: ChipLinks::default() },
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
            if let Some(sub) = &c.record.dark_substitute {
                l.dark_substitute = Some(chip_handle(sub, &format!("{whose}'s dark_substitute"))?);
            }
            if let Some(super::DamageFormula::SpNavi { slot, .. }) = &c.record.formula {
                // (The chip's own game's slots.)
                let rules = content.rules_of(root_of(&c.key));
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
                advances.push((r.order, super::ProgramAdvance { result: ChipHandle(i as u16), recipe }));
            }
            if !c.record.program_advances.is_empty() {
                // (A player's record of the round's formed ones is 32 bits.)
                if results >= 32 {
                    return Err(ContentError::new(format!("{whose}: more than 32 chips are Program Advances")));
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
            identities.push(super::identity::read(d, &content.assets)?);
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
            record.identity = identity_of(d, &identities)?;
            record.forms = match d.spec.field("forms") {
                Data::Nil => None,
                forms @ Data::Map(_) => {
                    let set = |game: &str| -> Result<super::FormSet, ContentError> {
                        let g = forms.field(game);
                        if matches!(g, Data::Nil) {
                            return Ok(super::FormSet::default());
                        }
                        let crosses = match g.field("crosses") {
                            Data::Nil => Vec::new(),
                            Data::List(items) => items
                                .iter()
                                .map(|v| form_ref(d, v, &format!("forms.{game}.crosses")).map(|f| f.expect("a form")))
                                .collect::<Result<_, _>>()?,
                            Data::Map(m) if m.is_empty() => Vec::new(),
                            other => return Err(what(d, format!("forms.{game}.crosses is {other:?}, not a list of forms"))),
                        };
                        Ok(super::FormSet {
                            crosses,
                            beast_out: form_ref(d, g.field("beast_out"), &format!("forms.{game}.beast_out"))?,
                            beast_over: form_ref(d, g.field("beast_over"), &format!("forms.{game}.beast_over"))?,
                        })
                    };
                    let souls = match forms.field("souls") {
                        Data::Nil => Vec::new(),
                        Data::List(items) => items
                            .iter()
                            .map(|v| form_ref(d, v, "forms.souls").map(|f| f.expect("a form")))
                            .collect::<Result<_, _>>()?,
                        Data::Map(m) if m.is_empty() => Vec::new(),
                        other => return Err(what(d, format!("forms.souls is {other:?}, not a list of forms"))),
                    };
                    Some(super::NaviForms { gregar: set("gregar")?, falzar: set("falzar")?, souls })
                }
                other => return Err(what(d, format!("`forms` is {other:?}, not the forms by game"))),
            };
            let own_chip = match &record.own_chip {
                Some(c) => Some((chip_handle(&c.chip, &format!("navi {}'s own chip", d.key))?, c.code)),
                None => None,
            };
            navis.push(NaviDef { key: d.key.clone(), record, own_chip });
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
            let mut record = super::navis::read_form(d, &reader)?;
            record.weapons = read_weapons(d)?;
            record.identity = identity_of(d, &identities)?;
            record.cross_of = match d.spec.field("cross_of") {
                Data::Nil => None,
                Data::Ref(Registry::Navi, key) => Some(NaviHandle(handle_of(Registry::Navi, key))),
                other => return Err(what(d, format!("`cross_of` is {other:?}, not a navi"))),
            };
            record.beast = form_ref(d, d.spec.field("beast"), "beast")?;
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
            if record.kind == super::FormKind::Base && record.revert.is_some() {
                return Err(what(d, "a base form names no action that reverts a navi out of it (`revert`)".into()));
            }
            if record.kind != super::FormKind::Base && record.change.is_none() {
                return Err(what(d, "a form other than the base form names the action that changes a navi into it (`change`)".into()));
            }
            if record.kind.has_cross() != record.cross_of.is_some() {
                return Err(what(d, "a Cross (and one in Beast Out) names the navi it is made with (`cross_of`), and no other form does".into()));
            }
            if (record.kind == super::FormKind::Cross) != record.beast.is_some() {
                return Err(what(d, "a Cross names its form in Beast Out (`beast`), and no other form does".into()));
            }
            forms.push(FormDef { key: d.key.clone(), record });
        }
        for (i, f) in forms.iter().enumerate() {
            if let Some(h) = f.record.identity {
                claim_identity(&mut identities, h, super::IdentityOwner::Form(FormHandle(i as u16)), &f.key)?;
            }
        }
        // Each game's base form: what its navis are in before they change
        // form; one a game.
        let game_of = |key: &str| keys::root_of(key).and_then(|g| root_names.iter().position(|r| r == g));
        let mut base_forms: Vec<Option<FormHandle>> = vec![None; root_names.len()];
        for (i, f) in forms.iter().enumerate().filter(|(_, f)| f.record.kind == super::FormKind::Base) {
            let Some(game) = game_of(&f.key) else {
                return Err(ContentError::new(format!("base form {}'s id names no loaded game", f.key)));
            };
            if let Some(other) = base_forms[game] {
                return Err(ContentError::new(format!(
                    "two forms of {} are base forms ({} and {})",
                    root_names[game],
                    forms[other.index()].key,
                    f.key
                )));
            }
            base_forms[game] = Some(FormHandle(i as u16));
        }
        // What the forms and the navis' sets name is the kind of form they
        // say.
        let kind_of = |h: FormHandle| forms[h.index()].record.kind;
        for f in &forms {
            if f.record.beast.is_some_and(|b| kind_of(b) != super::FormKind::CrossBeast) {
                return Err(ContentError::new(format!("form {}'s `beast` is not a Cross in Beast Out", f.key)));
            }
            if f.record.glow.as_ref().is_some_and(|g| g.is_empty()) {
                return Err(ContentError::new(format!("form {}'s `glow` has no shaders", f.key)));
            }
        }
        for n in &navis {
            let Some(sets) = &n.record.forms else { continue };
            for set in [&sets.gregar, &sets.falzar] {
                let ok = set.crosses.iter().all(|&c| kind_of(c) == super::FormKind::Cross)
                    && set.beast_out.is_none_or(|f| kind_of(f) == super::FormKind::Beast)
                    && set.beast_over.is_none_or(|f| kind_of(f) == super::FormKind::BeastOver);
                if !ok {
                    return Err(ContentError::new(format!(
                        "navi {}'s forms: `crosses` are Crosses, `beast_out` a Beast and `beast_over` a Beast Over",
                        n.key
                    )));
                }
            }
            if game_of(&n.key).and_then(|g| base_forms[g]).is_none() {
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

        // The lock-on modes, by key (the definitions' order).
        let mut lockons = Vec::new();
        for d in definitions.of(Registry::Lockon) {
            let what = |e: String| ContentError::new(format!("{}.luau: lockon {}: {e}", d.module, d.key));
            let mut spec = d.spec.clone();
            if let Data::Map(entries) = &mut spec {
                entries.retain(|(k, _)| !matches!(k, nettai_content_api::DataKey::Str(s) if s == "id"));
            }
            let mode: super::LockonMode = reader.read(&spec, &d.key).map_err(what)?;
            lockons.push(LockonDef { key: d.key.clone(), mode });
        }
        if lockons.windows(2).any(|w| w[0].key >= w[1].key) {
            return Err(ContentError::new("the lock-on modes are not in key order (the define phase sorts each registry)"));
        }

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

        // The roles: each root's (a root defines at most one, `<root>:roles`).
        let mut roles = Vec::with_capacity(root_names.len());
        for name in &root_names {
            let own: Vec<&Definition> =
                definitions.of(Registry::Roles).iter().filter(|d| keys::root_of(&d.key).unwrap_or("") == name.as_str()).collect();
            roles.push(match own[..] {
                [d] => read_roles(d, &definitions, &content.assets, &actions, &kinds, &chips, &statuses, &mut functions)?,
                _ => Roles::default(),
            });
        }

        // The systems and the rulesets.
        let mut systems = Vec::new();
        let mut buttons: Vec<ButtonDef> = Vec::new();
        let mut windows: Vec<WindowDef> = Vec::new();
        for d in definitions.of(Registry::System) {
            let what = |e: &str| ContentError::new(format!("{}.luau: system {}: {e}", d.module, d.key));
            if let Data::Map(entries) = &d.spec {
                for (k, _) in entries {
                    if !matches!(k, nettai_content_api::DataKey::Str(f) if ["id", "state", "setup", "hooks", "custom", "buttons", "windows", "actions"].contains(&f.as_str())) {
                        return Err(what(&format!("`{k}` is no field of a system (id, state, setup, hooks, custom, buttons, windows, actions)")));
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
                            if !["slot", "cells", "uses", "right", "left", "shown", "state", "pressed", "taken_back"].contains(&f.as_str()) {
                                return Err(at(&format!("`{f}` is no field of a button (slot, cells, uses, right, left, shown, state, pressed, taken_back)")));
                            }
                        }
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
                        let slot = byte("slot")?.ok_or_else(|| at("`slot` is missing"))?;
                        let cells = byte("cells")?.unwrap_or(1);
                        if !(1..=2).contains(&cells) {
                            return Err(at("`cells` is 1 or 2"));
                        }
                        let (uses, right, left) = (byte("uses")?.unwrap_or(0), byte("right")?, byte("left")?);
                        own_buttons.push(ButtonHandle((buttons.len()) as u16));
                        buttons.push(ButtonDef { system, name: name.clone(), slot, cells, uses, right, left, shown, state, pressed, taken_back });
                    }
                }
                _ => return Err(what("`buttons` is a table of buttons by name")),
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
                        if let Some((f, _)) = fields.iter().find(|(f, _)| f.to_string() != "update") {
                            return Err(what(&format!("window `{name}`: `{f}` is no field of a window (update)")));
                        }
                        if !matches!(spec.field("update"), Data::Function) {
                            return Err(what(&format!("window `{name}`: `update` is a function")));
                        }
                        let update = functions.id(FnSource::slot(Registry::System, &d.key, &format!("windows.{name}.update")));
                        own_windows.push(WindowHandle(windows.len() as u16));
                        windows.push(WindowDef { system, name, update });
                    }
                }
                _ => return Err(what("`windows` is a table of windows by name")),
            }
            systems.push(SystemDef {
                key: d.key.clone(),
                state: layout("state")?,
                setup: layout("setup")?,
                hooks,
                actions: system_actions,
                buttons: own_buttons,
                windows: own_windows,
            });
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
        // CurAction 0x1C, BN5's 0x1A).
        let mut change_actions = vec![false; actions.len()];
        for f in &forms {
            for a in [f.record.change, f.record.revert].into_iter().flatten() {
                change_actions[a.index()] = true;
            }
        }
        let rulesets = read_rulesets(&definitions, &root_names)?;

        let records: Vec<RecordDef> = definitions
            .of(Registry::Record)
            .iter()
            .map(|d| RecordDef { key: d.key.clone(), record_type: d.record_type.clone().unwrap_or_default() })
            .collect();

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
            base_forms,
            chip_keys: BTreeMap::new(),
            weapon_keys: BTreeMap::new(),
            roots: root_names.clone(),
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
            rulesets,
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
        defs.functions = functions.list;
        Ok(defs)
    }
}

/// The rulesets (docs/design/rules-in-luau.md §2.2): a game's own
/// (`stock`) lists its systems; a ruleset made from another (`base`, a mix)
/// has its base's systems less `remove`, with `add` after them. Each has a
/// game (§2.3): the root its `game` names, else its base's game, else its
/// own root, which must then be a game's (have a stock ruleset).
fn read_rulesets(definitions: &Definitions, root_names: &[String]) -> Result<Vec<RulesetDef>, ContentError> {
    let defs = definitions.of(Registry::Ruleset);
    let systems = definitions.of(Registry::System);
    let handle = |key: &str| defs.iter().position(|r| r.key == key).map(|i| RulesetHandle(i as u16));
    struct Read {
        stock: bool,
        systems: Option<Vec<SystemHandle>>,
        base: Option<RulesetHandle>,
        add: Vec<SystemHandle>,
        remove: Vec<SystemHandle>,
        game: Option<RootId>,
        sections: Vec<String>,
    }
    let mut read = Vec::with_capacity(defs.len());
    for d in defs {
        let what = |e: &str| ContentError::new(format!("{}.luau: ruleset {}: {e}", d.module, d.key));
        const FIELDS: [&str; 8] = ["id", "stock", "systems", "base", "add", "remove", "game", "sections"];
        if let Data::Map(entries) = &d.spec {
            for (k, _) in entries {
                if !matches!(k, nettai_content_api::DataKey::Str(f) if FIELDS.contains(&f.as_str())) {
                    return Err(what(&format!("`{k}` is no field of a ruleset ({})", FIELDS.join(", "))));
                }
            }
        }
        let list = |field: &str| -> Result<Option<Vec<SystemHandle>>, ContentError> {
            let items: &[Data] = match d.spec.field(field) {
                Data::Nil => return Ok(None),
                Data::List(items) => items,
                Data::Map(m) if m.is_empty() => &[],
                _ => return Err(what(&format!("`{field}` is a list of systems"))),
            };
            let mut out = Vec::new();
            for v in items {
                let Data::Ref(Registry::System, key) = v else {
                    return Err(what(&format!("`{field}` lists system definitions (define.system {{ ... }})")));
                };
                let h = SystemHandle(systems.iter().position(|s| &s.key == key).expect("a defined system") as u16);
                if out.contains(&h) {
                    return Err(what(&format!("`{field}` lists system {key} twice")));
                }
                out.push(h);
            }
            Ok(Some(out))
        };
        let stock = match d.spec.field("stock") {
            Data::Nil => false,
            Data::Bool(b) => *b,
            _ => return Err(what("`stock` is true or false")),
        };
        let base = match d.spec.field("base") {
            Data::Nil => None,
            Data::Ref(Registry::Ruleset, key) => Some(handle(key).expect("a defined ruleset")),
            _ => return Err(what("`base` is a ruleset (define.ruleset { ... })")),
        };
        let game = match d.spec.field("game") {
            Data::Nil => None,
            Data::Str(name) => Some(RootId(
                root_names.iter().position(|r| r == name).ok_or_else(|| what(&format!("`game` names {name:?}, which is no loaded root")))? as u8,
            )),
            _ => return Err(what("`game` is a root's name (\"bn6\")")),
        };
        let sections: Vec<String> = match d.spec.field("sections") {
            Data::Nil => Vec::new(),
            Data::List(items) => items
                .iter()
                .map(|v| match v {
                    Data::Ref(Registry::Rules, key) => Ok(key.clone()),
                    _ => Err(what("`sections` lists rule sections (define.rules(...))")),
                })
                .collect::<Result<_, _>>()?,
            Data::Map(m) if m.is_empty() => Vec::new(),
            _ => return Err(what("`sections` is a list of rule sections (define.rules(...))")),
        };
        if stock && !sections.is_empty() {
            return Err(what("a stock ruleset's sections are its game's (define.rules in its folder), not its own"));
        }
        let r = Read {
            stock,
            systems: list("systems")?,
            base,
            add: list("add")?.unwrap_or_default(),
            remove: list("remove")?.unwrap_or_default(),
            game,
            sections,
        };
        match (r.stock, r.base, &r.systems) {
            (true, Some(_), _) => return Err(what("a stock ruleset is a game's own: it has no `base`")),
            (_, Some(_), Some(_)) => return Err(what("a ruleset made from a `base` lists what it changes (`add`, `remove`), not `systems`")),
            (_, None, _) if !r.add.is_empty() || !r.remove.is_empty() => {
                return Err(what("`add` and `remove` change a `base`, and it names none"));
            }
            _ => {}
        }
        read.push(r);
    }
    // Resolve each, its base first (a ruleset can't be its own base).
    fn resolve(
        i: usize,
        read: &[Read],
        defs: &[Definition],
        root_names: &[String],
        systems_of: &[Definition],
        out: &mut Vec<Option<RulesetDef>>,
        visiting: &mut Vec<usize>,
    ) -> Result<(), ContentError> {
        if out[i].is_some() {
            return Ok(());
        }
        let d = &defs[i];
        let what = |e: &str| ContentError::new(format!("{}.luau: ruleset {}: {e}", d.module, d.key));
        if visiting.contains(&i) {
            return Err(what("its `base` leads back to it"));
        }
        visiting.push(i);
        let r = &read[i];
        let own_root = keys::root_of(&d.key).unwrap_or("");
        let own = RootId(root_names.iter().position(|n| n == own_root).unwrap_or(0) as u8);
        let (systems, base_game) = match r.base {
            Some(b) => {
                resolve(b.index(), read, defs, root_names, systems_of, out, visiting)?;
                let base = out[b.index()].as_ref().expect("resolved");
                let mut list: Vec<SystemHandle> = base.systems.clone();
                for h in &r.remove {
                    let Some(at) = list.iter().position(|x| x == h) else {
                        return Err(what(&format!("`remove` names system {}, which its base doesn't have", systems_of[h.index()].key)));
                    };
                    list.remove(at);
                }
                for h in &r.add {
                    if list.contains(h) {
                        return Err(what(&format!("`add` names system {}, which it has already", systems_of[h.index()].key)));
                    }
                    list.push(*h);
                }
                (list, Some(base.game))
            }
            None => (r.systems.clone().unwrap_or_default(), None),
        };
        if systems.len() > MAX_SYSTEMS {
            return Err(what(&format!("{} systems; a ruleset has at most {MAX_SYSTEMS}", systems.len())));
        }
        let own_is_game = read.iter().zip(defs).any(|(o, od)| o.stock && keys::root_of(&od.key).unwrap_or("") == own_root);
        let game = match (r.game, base_game) {
            (Some(g), _) => g,
            (None, Some(g)) => g,
            (None, None) if own_is_game => own,
            (None, None) => {
                return Err(what(&format!("its root ({own_root}) is no game's (it has no stock ruleset): name a `base` or a `game`")));
            }
        };
        visiting.pop();
        out[i] = Some(RulesetDef { key: d.key.clone(), stock: r.stock, systems, base: r.base, game, sections: r.sections.clone() });
        Ok(())
    }
    let mut out: Vec<Option<RulesetDef>> = vec![None; defs.len()];
    for i in 0..defs.len() {
        resolve(i, &read, defs, root_names, systems, &mut out, &mut Vec::new())?;
    }
    let rulesets: Vec<RulesetDef> = out.into_iter().map(|r| r.expect("every ruleset resolved")).collect();
    // A game names a root with a stock ruleset; a root has at most one.
    for (i, name) in root_names.iter().enumerate() {
        let stocks: Vec<&str> =
            rulesets.iter().filter(|r| r.stock && keys::root_of(&r.key).unwrap_or("") == name.as_str()).map(|r| r.key.as_str()).collect();
        if stocks.len() > 1 {
            return Err(ContentError::new(format!(
                "root {name} has {} stock rulesets ({}); a game has one",
                stocks.len(),
                stocks.join(", ")
            )));
        }
        if stocks.is_empty() {
            if let Some(r) = rulesets.iter().find(|r| r.game == RootId(i as u8)) {
                return Err(ContentError::new(format!("ruleset {}'s game is root {name}, which is no game's (no stock ruleset)", r.key)));
            }
        }
    }
    Ok(rulesets)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every module of the BN6 content (content/bn6) loads in the define
    /// phase: it passes the bytecode check, runs, and what it defines reads
    /// back as data.
    #[test]
    fn every_bn6_module_loads_in_the_define_phase() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");
        let mut c = Content::default();
        c.scripts = crate::content::Scripts::root(crate::content::RootManifest::named("bn6"), crate::content::testing::modules_under(dir));
        // (With the shared folder its modules require, content/common.)
        crate::content::testing::add_shared(&mut c.scripts);
        c.assets = crate::content::testing::asset_names_for(&c.scripts);
        assert!(c.scripts.modules.len() > 200, "{} modules", c.scripts.modules.len());
        c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
        // Every chip is a definition with its own use.
        assert!(c.defs.chips.len() > 300, "{} chips", c.defs.chips.len());
        // Effects, sparks, regions and collision types are definitions
        // the engine holds by handle.
        assert!(c.defs.effects.len() > 100, "{} effects defined", c.defs.effects.len());
        assert!(!c.defs.sparks.is_empty() && !c.defs.regions.is_empty() && c.defs.collisions.len() >= 89);
    }
}
