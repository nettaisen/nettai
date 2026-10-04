//! The custom screen (BN6 ruleset): between turns each player deals chips
//! from their folder, picks some with their joypad (and maybe Beast Out or
//! a Cross), and presses OK; the hand is built and sent over the link, and
//! the fight resumes once both players' results are in.
//!
//! In the original each console runs only its own player's screen and
//! receives the other's result over the link. Here both screens run inside
//! the simulation, each driven by its player's joypad, and the link is
//! simulated too: a result arrives a fixed number of ticks after it is
//! sent, so the fight resumes on the same tick as in the original. See
//! docs/engine/custom-screen.md.

pub mod builder;
pub mod chatbox;
pub mod folder;
pub mod library;
pub mod look;
pub mod screen;

pub use folder::{BattleFolder, FolderChip, SavedFolder};
pub use library::{GameLibrary, Library};
pub use look::{DarkHover, Drawn, ScreenLook};
pub use screen::{ButtonCell, ButtonPlace, Phase, PlayerView, Request, RoundMemory, Screen, Slot, SlotKind, SlotState};

use crate::battle::{Battle, CustomResult, battle_flags};
use crate::console::{Console, ConsoleSetup};
use nettai_content_api::ChipHandle;
use crate::hand::ChipHand;
use crate::input::Joypad;
use crate::kinds::player::Emotion;
use crate::setup::{NaviStats, effects};
use crate::transform::TransformRequest;
use builder::{ClassCounts, Pick, ProgramAdvancesUsed};
use screen::SPECIAL_SLOT;

/// Which game a player plays: it decides their Crosses and Beast form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameVersion {
    Gregar,
    #[default]
    Falzar,
}

/// What a BN5 save has of Soul Unison: the soul button (event flag 0), the
/// souls (bit n: soul n's flag, 0x08024BF0's table) and Chaos Unison
/// (event flag 0x236); and the turns its NaviCust adds to a soul (NaviStats
/// +0x32, signed: SoulT+1's 1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SoulUnlocks {
    pub button: bool,
    pub owned: u16,
    pub chaos: bool,
    pub turn_bonus: i8,
}

/// The highest level of a navi code (`sub_8121198`: a navi's 15 codes).
pub const MAX_NAVI_LEVEL: u8 = 14;

/// What a player brings to a round that only their own console knows in
/// the original: the battle folder (shuffled at the round's init), what
/// their save holds that the battle reads, and what their ruleset's
/// systems take (BN6's: the game version and what the save unlocks on the
/// custom screen, in its systems' setup blocks).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerSetup {
    /// The shuffled battle folder. None only when checking against a
    /// recording that lacks this player's folder: their screen is then
    /// not simulated, and the recording supplies what it sends
    /// (`TickEvents::recorded`).
    pub folder: Option<BattleFolder>,
    /// BN5's Soul Unison: what the save has of it (none in BN6; until BN5's
    /// soul button is a system's).
    pub souls: SoulUnlocks,
    /// The joypad's auto-repeat beat (0-4) on the round's first tick; each
    /// console counts its own.
    pub joypad_phase: u8,
    /// The save's bug frags (a dark chip spends one), which the init
    /// exchange shares.
    pub bug_frags: u32,
    /// The level of the navi code the save received (0 to
    /// [`MAX_NAVI_LEVEL`]; event flag 0x163 set), which the init exchange
    /// shares (`sub_800B144`, `dword_203CFA0`): a link navi's chip bonus,
    /// and what BN6's rules read of the code (the custom screen's seal on
    /// Beast Out and the Cross window, MegaMan's level gains). None: no
    /// code received (0xFF), MegaMan only: a link navi exists through its
    /// code (docs/engine/link-navis.md).
    pub navi_level: Option<u8>,
    /// How fast the save deleted each SP navi (`byte_203EB00`: the save's
    /// 0x020018C0, through the init exchange): the SP navi chips' damage
    /// goes by it.
    pub sp_times: crate::setup::SpTimes,
    /// What the player's console brings besides: its RNG (RNG1), which
    /// ChpShufl's re-deal draws from (`crate::console`). In netplay it is
    /// part of the setup the peers exchange.
    pub console: ConsoleSetup,
    /// The rules the player plays by (docs/design/rules-in-luau.md §2.3);
    /// none: the content's stock ruleset.
    pub ruleset: Option<nettai_content_api::RulesetHandle>,
    /// What the player brings for each system of their ruleset (its
    /// `setup` fields), in the ruleset's order; none given: each system's
    /// defaults (`setup_defaults`, the rest zero; `PlayerSetup::set_rule`
    /// writes one by name).
    pub rules: Vec<nettai_content_api::ContentState>,
    /// The patch cards the player has installed (`crate::patch_cards`):
    /// their ruleset's rules apply them (BN6's patch-cards system).
    pub patch_cards: crate::patch_cards::PatchCards,
    /// The player's NaviCust (`crate::navicust`), which their ruleset's
    /// rules compile into the navi's stats as the round is set up (BN6's
    /// navicust system); none: the stats are the setup's as they are (a
    /// recording's, which the original's NaviCust has already made).
    pub navicust: Option<crate::navicust::NaviCust>,
    /// The player's tactics (BN5's computer-navi data, `crate::tactics`),
    /// which a computer navi on the other side plays; none: empty. (A
    /// recording's; match files and netplay don't carry them yet.)
    pub tactics: crate::tactics::Tactics,
}

impl Default for PlayerSetup {
    fn default() -> PlayerSetup {
        PlayerSetup {
            folder: Some(BattleFolder::empty()),
            souls: SoulUnlocks::default(),
            joypad_phase: 0,
            bug_frags: 0,
            navi_level: None,
            sp_times: Default::default(),
            console: ConsoleSetup::default(),
            ruleset: None,
            rules: Vec::new(),
            patch_cards: Default::default(),
            navicust: None,
            tactics: Default::default(),
        }
    }
}

/// A result on its way over the link.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sent {
    pub result: CustomResult,
    /// The tick it went out, and the tick its last word arrives.
    pub sent_at: u32,
    pub arrives: u32,
}

/// What a recording says about a player whose screen isn't simulated (see
/// `PlayerSetup::folder`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Recorded {
    /// Their custom screen's status bit as they send it this tick.
    pub in_custom: bool,
    /// Their result, arrived this tick.
    pub result: Option<Box<CustomResult>>,
}

/// One player's custom-screen state through a round.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Side {
    /// BN5's Soul Unison (the setup's).
    pub souls: SoulUnlocks,
    /// The joypad the screen reads: the player's buttons this tick, not
    /// delayed by the link like the fight's.
    pub joypad: Joypad,
    pub folder: Option<BattleFolder>,
    pub round: RoundMemory,
    pub program_advances: ProgramAdvancesUsed,
    /// Chips sent this round, by class.
    pub class_uses: ClassCounts,
    /// The screen, from its opening until the next one opens.
    pub screen: Option<Screen>,
    /// The status bit the player's console sends: its custom screen is
    /// open (BattleState+0x11 bit 2).
    pub in_custom: bool,
    /// The hand and transformation built at OK, sent after the window
    /// slides out (None: no chips picked).
    pub built: Option<(Option<ChipHand>, TransformRequest)>,
    pub sent: Option<Sent>,
    /// The emotion the screen reads of its navi (its context's), which its
    /// systems' hooks read too (`custom.player`), set while one runs.
    pub emotion: Emotion,
}

impl Side {
    pub fn new(setup: &PlayerSetup) -> Side {
        Side {
            souls: setup.souls,
            joypad: Joypad::new(setup.joypad_phase),
            folder: setup.folder,
            round: RoundMemory::default(),
            program_advances: ProgramAdvancesUsed::default(),
            class_uses: ClassCounts::default(),
            screen: None,
            in_custom: false,
            built: None,
            sent: None,
            emotion: Emotion::Normal,
        }
    }

    /// The screen is simulated (the folder is known).
    pub fn simulated(&self) -> bool {
        self.folder.is_some()
    }
}

/// Both players' custom screens.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomScreens {
    pub sides: [Side; 2],
    /// Ticks since the screen opened.
    pub ticks: u32,
    /// Both results are in; the screen closes next tick.
    pub committed: bool,
}

impl CustomScreens {
    pub fn new(players: &[PlayerSetup; 2]) -> CustomScreens {
        CustomScreens { sides: [Side::new(&players[0]), Side::new(&players[1])], ticks: 0, committed: false }
    }
}

/// What a player's screen reads of the battle: the game data, their navi,
/// and a few battle-wide facts.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub library: &'a dyn Library,
    /// The player's navi stats.
    pub stats: NaviStats,
    pub emotion: Emotion,
    /// The screen's number in the round (1 = first).
    pub turn: u8,
    /// Battle flag 0x40 (per-player gauges; never set in netbattles).
    pub per_player_gauges: bool,
    /// Battle effects 0x200000 (random battles).
    pub random_battle: bool,
    /// A netbattle's last turns (`sub_800A97A`; presentation).
    pub late_turns: bool,
    /// The tick, and the link's latency: a result sent now arrives
    /// `50 + link_delay` ticks later (50 words, one a tick).
    pub now: u32,
    pub link_delay: u8,
}

/// What a player's custom screen asks its side's systems
/// (docs/design/rules-in-luau.md §4.4: their `custom` hooks). The battle
/// answers ([`crate::battle::Battle`]'s side extras); a screen without one
/// (the screen's own tests) gets no answers.
pub trait Extras {
    /// `custom.hand_size(side)`: how many chips the screen deals, as it
    /// opens; none: the framework's rule.
    fn hand_size(&mut self) -> Option<u8>;
    /// The buttons on the screen `screen` (as it opens), in the order the
    /// side's systems are listed: each that its `shown` says is.
    fn buttons(&mut self, screen: &Screen) -> Vec<screen::ButtonPlace>;
    /// A button's `state` (at the open and after each pick), if it has one.
    fn button_state(&mut self, screen: &Screen, button: crate::content::ButtonHandle) -> Option<SlotState>;
    /// A button's `pressed` (A on it), which may change the screen.
    fn button_pressed(&mut self, screen: &mut Screen, folder: &mut BattleFolder, button: crate::content::ButtonHandle);
    /// A button's `taken_back` (B took its pick back), if it has one.
    fn button_taken_back(&mut self, screen: &mut Screen, button: crate::content::ButtonHandle);
    /// `custom.deal(side)`: the screen deals, the folder not yet closed
    /// up, on the side's console (its RNG1).
    fn dealing(&mut self, screen: &mut Screen, folder: &mut BattleFolder, console: &mut Console);
    /// `custom.open(side)`: the screen opens.
    fn opened(&mut self, screen: &mut Screen);
    /// `custom.confirmed(side)`: OK built the hand.
    fn confirmed(&mut self, screen: &mut Screen, folder: &mut BattleFolder);
    /// `custom.chip_picked(side, chip)` and `custom.chip_taken_back(side,
    /// chip)`: a chip of the hand picked, or its pick taken back.
    fn chip_picked(&mut self, screen: &mut Screen, folder: &mut BattleFolder, chip: ChipHandle);
    fn chip_taken_back(&mut self, screen: &mut Screen, chip: ChipHandle);
    /// A window's `update` (a tick of it, on the screen's joypad): whether
    /// it stays up.
    fn window_update(
        &mut self,
        screen: &mut Screen,
        folder: &mut BattleFolder,
        console: &mut Console,
        joy: &Joypad,
        window: crate::content::WindowHandle,
    ) -> bool;
    /// `custom.keys(side)`: choosing, a tick with keys: whether a system
    /// took them.
    fn keys(&mut self, screen: &mut Screen, folder: &mut BattleFolder, joy: &Joypad) -> bool;
    /// `custom.take_back(side)`: B with nothing picked: whether a system
    /// took something back.
    fn take_back(&mut self, screen: &mut Screen, folder: &BattleFolder) -> bool;
}

/// No systems: every question unanswered.
pub struct NoExtras;

impl Extras for NoExtras {
    fn hand_size(&mut self) -> Option<u8> {
        None
    }

    fn buttons(&mut self, _: &Screen) -> Vec<screen::ButtonPlace> {
        Vec::new()
    }

    fn button_state(&mut self, _: &Screen, _: crate::content::ButtonHandle) -> Option<SlotState> {
        None
    }

    fn button_pressed(&mut self, _: &mut Screen, _: &mut BattleFolder, _: crate::content::ButtonHandle) {}

    fn button_taken_back(&mut self, _: &mut Screen, _: crate::content::ButtonHandle) {}

    fn dealing(&mut self, _: &mut Screen, _: &mut BattleFolder, _: &mut Console) {}
    fn opened(&mut self, _: &mut Screen) {}

    fn confirmed(&mut self, _: &mut Screen, _: &mut BattleFolder) {}

    fn chip_picked(&mut self, _: &mut Screen, _: &mut BattleFolder, _: ChipHandle) {}

    fn chip_taken_back(&mut self, _: &mut Screen, _: ChipHandle) {}

    fn window_update(&mut self, _: &mut Screen, _: &mut BattleFolder, _: &mut Console, _: &Joypad, _: crate::content::WindowHandle) -> bool {
        false
    }

    fn keys(&mut self, _: &mut Screen, _: &mut BattleFolder, _: &Joypad) -> bool {
        false
    }

    fn take_back(&mut self, _: &mut Screen, _: &BattleFolder) -> bool {
        false
    }
}

/// Ticks a result takes to send: the link carries one of its words a tick
/// (`sub_801FF18`), BN6's 50 (a game's own: `Library::result_words`).
pub const SEND_TICKS: u32 = 50;

impl Side {
    fn view<'a>(&'a self, ctx: &'a Context, regular_pending: bool) -> PlayerView<'a> {
        PlayerView {
            library: ctx.library,
            stats: &ctx.stats,
            emotion: ctx.emotion,
            souls: &self.souls,
            class_uses: &self.class_uses,
            round: &self.round,
            regular_pending,
            per_player_gauges: ctx.per_player_gauges,
            random_battle: ctx.random_battle,
            late_turns: ctx.late_turns,
        }
    }

    /// The custom screen opens (`sub_8026840`) on the player's console:
    /// the status bit goes up and the player's screen deals. The round's
    /// first screen forgets the previous round's Crosses and Beast Out.
    pub fn open(&mut self, ctx: &Context, console: &mut Console) {
        self.open_with(ctx, console, &mut NoExtras);
    }

    /// [`Side::open`], asking the side's systems.
    pub fn open_with(&mut self, ctx: &Context, console: &mut Console, extras: &mut dyn Extras) {
        self.emotion = ctx.emotion;
        self.in_custom = true;
        self.built = None;
        self.sent = None;
        if ctx.turn == 1 {
            self.round = RoundMemory::default();
        }
        let Some(mut folder) = self.folder else { return };
        let regular = folder.regular_pending;
        // (Palette 11 keeps the last chip window's element colors from
        // screen to screen.)
        let last_chip = self.screen.and_then(|s| s.look.chip_window.last_chip).filter(|_| ctx.turn != 1);
        let mut screen = Screen::open(&mut folder, &self.view(ctx, regular), ctx.turn, console, extras);
        if screen.look.chip_window.last_chip.is_none() {
            screen.look.chip_window.last_chip = last_chip;
        }
        // sub_802A646: once the tag pair is among the chips a screen can
        // deal, a re-deal no longer keeps it apart (BattleState+0x44).
        if console.tag_pair.is_some_and(|t| t < screen.hand_size) {
            console.tag_pair = None;
        }
        self.folder = Some(folder);
        self.screen = Some(screen);
    }

    /// One tick of the player's screen on their joypad and console.
    /// `damage`: a chip's damage for this player now (`sub_80109A4`), for
    /// the hand built at OK.
    pub fn tick(&mut self, ctx: &Context, console: &mut Console, damage: impl Fn(ChipHandle) -> u16) -> Option<Request> {
        self.tick_with(ctx, console, damage, &mut NoExtras)
    }

    /// [`Side::tick`], asking the side's systems.
    pub fn tick_with(
        &mut self,
        ctx: &Context,
        console: &mut Console,
        damage: impl Fn(ChipHandle) -> u16,
        extras: &mut dyn Extras,
    ) -> Option<Request> {
        self.emotion = ctx.emotion;
        let (Some(mut screen), Some(mut folder)) = (self.screen, self.folder) else { return None };
        let request = screen.tick(&self.joypad, &self.view(ctx, folder.regular_pending), &mut folder, console, extras);
        match request {
            Some(Request::Confirm) => self.confirm(ctx, &mut screen, &mut folder, console, damage, extras),
            Some(Request::Send) => {
                // sub_8026DC4's first tick: sub_802A4FC counts the classes,
                // sub_800B3A2 sends the hand, the navi's stats as they are
                // now, and the transformation.
                let (hand, transform) = self.built.clone().expect("a hand was built at OK");
                if let Some(h) = &hand {
                    builder::count_classes(h, &mut self.class_uses, ctx.library);
                }
                let result = CustomResult { hand, navi_stats: ctx.stats, transform };
                self.sent = Some(Sent { result, sent_at: ctx.now, arrives: ctx.now + ctx.library.result_words() + ctx.link_delay as u32 });
            }
            None => {}
        }
        if matches!(screen.phase, Phase::Closing { tick: 1 }) {
            // The slide-out's first tick clears the status bit.
            self.in_custom = false;
        }
        self.screen = Some(screen);
        self.folder = Some(folder);
        request
    }

    /// OK (`sub_8028D3A`): build the hand (`sub_8029110`), take the picked
    /// chips out of the folder (`sub_80293F8`: each one also moves the
    /// console's tag pair index down by one, while it has one), and turn
    /// Beast Out or the Cross into a transformation (`sub_8029344`,
    /// `sub_802937A`).
    fn confirm(
        &mut self,
        ctx: &Context,
        screen: &mut Screen,
        folder: &mut BattleFolder,
        console: &mut Console,
        damage: impl Fn(ChipHandle) -> u16,
        extras: &mut dyn Extras,
    ) {
        let view = self.view(ctx, folder.regular_pending);
        let picks: Vec<Pick> = screen
            .selection()
            .iter()
            .filter_map(|&slot| {
                let chip = screen.chip_in(slot, folder)?;
                let regular = matches!(screen.slots[slot as usize].kind, SlotKind::Chip { regular: true, .. });
                Some(Pick { chip: screen::checked(chip, &view), regular })
            })
            .collect();
        // (The family of the chip given up for BN5's soul, as the screen
        // checked it.)
        let soul_family = screen.chip_in(screen.soul.given_up, folder).map(|c| ctx.library.chip(screen::checked(c, &view).id).family);
        let mut pa_used = self.program_advances;
        let built = builder::build(&picks, ctx.turn, &mut pa_used, ctx.library, ctx.per_player_gauges, damage);
        self.program_advances = pa_used;
        for p in &picks {
            // A link navi's own chip is spent for the round (the bit of
            // its navi).
            if let Some(navi) = ctx.library.own_chip_of(p.chip.id) {
                self.round.navi_chips_used |= 1 << navi.0;
            }
        }
        let navi = ctx.stats.navi;
        let mut transform = TransformRequest::NONE;
        // The form a system's pick holds (BN6's Beast Out or Cross), and
        // what its systems note of the round (BN6's: Beast Out or the Cross
        // used).
        if screen.form.is_some() {
            transform.form = screen.form;
        }
        extras.confirmed(screen, folder);
        if screen.slots[SPECIAL_SLOT as usize].kind == SlotKind::Soul && screen.selection().contains(&SPECIAL_SLOT) {
            // 0x08024FF6: BN5's soul, for 3 turns and the NaviCust's bonus
            // (at most 9; under 0, 1), Chaos Unison for 1; the soul is used
            // this round.
            let soul = screen.soul;
            transform.form = soul_family.and_then(|f| ctx.library.soul_for_family(navi, f)).map(|(_, f)| f);
            let turns = 3 + self.souls.turn_bonus as i32;
            transform.turns = if soul.chaos {
                1
            } else if turns > 9 {
                9
            } else if turns < 0 {
                1
            } else {
                turns as u8
            };
            transform.chaos = soul.chaos;
            self.round.souls_used |= if soul.chaos { 1 << (16 + soul.number) } else { 1 << soul.number };
        }
        for &slot in screen.selection() {
            // (0x08025088: the soul's place takes the chip given up for it
            // out of the folder.)
            let slot = if screen.slots[slot as usize].kind == SlotKind::Soul { screen.soul.given_up } else { slot };
            if let SlotKind::Chip { index, regular } = screen.slots[slot as usize].kind {
                folder.take(index as usize);
                if regular {
                    folder.regular_pending = false;
                }
                // The folder closes up by one at the next opening: the
                // pair's index follows (a byte; a pair at the hand's end
                // can reach 0, which reads as no pair).
                if let Some(t) = &mut console.tag_pair {
                    *t = t.wrapping_sub(1);
                }
            }
        }
        screen.program_advance = built.program_advance;
        let hand = (!screen.selection().is_empty()).then_some(built.hand);
        self.built = Some((hand, transform));
    }
}

impl Battle {
    /// What side `side`'s screen reads of the battle (and its game data,
    /// the battle's content).
    fn custom_context<'a>(&self, side: u8, library: &'a dyn Library) -> Context<'a> {
        Context { emotion: crate::kinds::player::emotion(self, side), ..self.custom_context_without_emotion(side, library) }
    }

    /// [`Battle::custom_context`] with the emotion left normal, for a
    /// caller that has the screen's own.
    fn custom_context_without_emotion<'a>(&self, side: u8, library: &'a dyn Library) -> Context<'a> {
        Context {
            library,
            stats: self.stats[side as usize],
            emotion: Emotion::Normal,
            turn: self.round.turn,
            per_player_gauges: self.round.flags & battle_flags::PER_PLAYER_GAUGES != 0,
            random_battle: self.setup.settings.effects & effects::RANDOM != 0,
            late_turns: self.late_turns(),
            now: self.round.ticks,
            link_delay: self.link.delay,
        }
    }

    /// The custom screen opens (`sub_8009338`'s first tick, `sub_8026840`).
    pub(crate) fn open_custom_screens(&mut self) {
        // Shared: the turn count, the gauge.
        self.gauge.value = 0;
        self.clear_flags(battle_flags::GAUGE_FULL | battle_flags::CUSTOM_REQUESTED);
        self.gauge.enabled = false;
        // sub_801DACC(0x30172): the chips' icons and window go.
        self.chip_hud = Default::default();
        self.round.turn += 1;
        self.custom.ticks = 0;
        self.custom.committed = false;
        let content = self.content.clone();
        for side in 0..2u8 {
            let library = library::GameLibrary { content: &content, game: self.games.sides[side as usize], ruleset: self.games.rulesets[side as usize] };
            let ctx = self.custom_context(side, &library);
            let mut s = self.custom.sides[side as usize].clone();
            let mut console = self.consoles[side as usize];
            s.open_with(&ctx, &mut console, &mut SideExtras { b: self, side, emotion: ctx.emotion });
            self.custom.sides[side as usize] = s;
            self.consoles[side as usize] = console;
        }
    }

    /// One tick of both players' screens after the opening tick, then the
    /// link: the fight resumes once both results are in.
    pub(crate) fn tick_custom_screens(&mut self, recorded: &[Option<Recorded>; 2]) {
        self.custom.ticks += 1;
        if self.custom.ticks == 10 && self.round.turn != 1 {
            // The window has slid in: the NaviCust custom-HP bug bites
            // (not on the round's first screen).
            for side in 0..2 {
                self.custom_hp_bug(side);
            }
        }
        for side in 0..2u8 {
            if let Some(r) = &recorded[side as usize] {
                let s = &mut self.custom.sides[side as usize];
                if let Some(result) = &r.result {
                    let now = self.round.ticks;
                    s.sent = Some(Sent { result: (**result).clone(), sent_at: now, arrives: now });
                }
                continue;
            }
            let content = self.content.clone();
            let library = library::GameLibrary { content: &content, game: self.games.sides[side as usize], ruleset: self.games.rulesets[side as usize] };
            let ctx = self.custom_context(side, &library);
            let mut s = self.custom.sides[side as usize].clone();
            let mut console = self.consoles[side as usize];
            // (A chip's damage now, for the hand built at OK: a formula's
            // reads the battle, so those are read before the extras borrow
            // it.)
            let formulas: Vec<(ChipHandle, u16)> = content
                .defs
                .formula_chips
                .iter()
                .map(|&id| (id, crate::hand::chip_damage(self, Some(id), side)))
                .collect();
            let damage = |id: ChipHandle| match formulas.iter().find(|(c, _)| *c == id) {
                Some(&(_, d)) => d,
                None => content.chip(id).damage,
            };
            let request = s.tick_with(&ctx, &mut console, damage, &mut SideExtras { b: self, side, emotion: ctx.emotion });
            // The screen's sounds, which only its player hears.
            if let Some(screen) = &s.screen {
                for sound in screen.look.drawn.sounds() {
                    self.sound_for(side, sound.role());
                }
                if let Some((music, screen)) = screen.look.drawn.volume {
                    self.play_sound_for(side, crate::sound::SoundCue::ScreenVolume { music, screen });
                }
            }
            self.custom.sides[side as usize] = s;
            self.consoles[side as usize] = console;
            if request == Some(Request::Send) {
                // sub_8027D78 on the sending tick.
                self.restart_gauge();
            }
        }
        let now = self.round.ticks;
        if self.custom.sides.iter().all(|s| s.sent.as_ref().is_some_and(|x| x.arrives <= now)) {
            let results = self.custom.sides.each_ref().map(|s| s.sent.as_ref().expect("sent").result.clone());
            self.install_exchange(results);
            self.custom.committed = true;
            // loc_8026E14: the consoles' chip icons are back (sub_801DA48(2)).
            for hud in &mut self.chip_hud {
                hud.icons = true;
            }
        }
    }

    /// `sub_8027D78`: the gauge starts over (and runs, unless it is the
    /// 15th screen or later).
    pub(crate) fn restart_gauge(&mut self) {
        self.gauge.value = 0;
        self.clear_flags(battle_flags::GAUGE_FULL | battle_flags::CUSTOM_REQUESTED);
        if !self.late_turns() {
            self.gauge.enabled = true;
        }
    }
}

impl Battle {
    /// Side `side`'s custom-screen extras (its systems' `custom` hooks),
    /// for a screen run outside the battle's own loop (bn6-compat's check
    /// of the traces' screens, which sets the side's stats and the turn
    /// first).
    pub fn custom_extras(&mut self, side: u8, emotion: Emotion) -> impl Extras + '_ {
        SideExtras { b: self, side, emotion }
    }
}

/// A side's custom screen's extras: its systems' `custom` hooks.
struct SideExtras<'b> {
    b: &'b mut Battle,
    side: u8,
    /// The emotion the screen reads (its context's).
    emotion: Emotion,
}

impl SideExtras<'_> {
    /// Run `f` with the side's screen (and folder) back in the battle, for
    /// `custom.*` to reach, and take them back after.
    fn with_screen<R>(&mut self, screen: &mut Screen, folder: Option<&mut BattleFolder>, f: impl FnOnce(&mut Battle) -> R) -> R {
        self.with_screen_console(screen, folder, None, None, f)
    }

    /// [`SideExtras::with_screen`], with the side's console (its camera)
    /// and the screen's joypad (`custom.pressed`; a checker ticks a side of
    /// its own) back in the battle too.
    fn with_screen_console<R>(
        &mut self,
        screen: &mut Screen,
        folder: Option<&mut BattleFolder>,
        console: Option<&mut Console>,
        joy: Option<&Joypad>,
        f: impl FnOnce(&mut Battle) -> R,
    ) -> R {
        let side = self.side as usize & 1;
        let (old_screen, old_folder, old_console, old_emotion, old_joypad) = (
            self.b.custom.sides[side].screen,
            self.b.custom.sides[side].folder,
            self.b.consoles[side],
            self.b.custom.sides[side].emotion,
            self.b.custom.sides[side].joypad,
        );
        self.b.custom.sides[side].screen = Some(*screen);
        self.b.custom.sides[side].emotion = self.emotion;
        if let Some(j) = joy {
            self.b.custom.sides[side].joypad = *j;
        }
        if let Some(f) = &folder {
            self.b.custom.sides[side].folder = Some(**f);
        }
        if let Some(c) = &console {
            self.b.consoles[side] = **c;
        }
        let r = f(self.b);
        *screen = self.b.custom.sides[side].screen.expect("the screen stays");
        if let Some(folder) = folder {
            *folder = self.b.custom.sides[side].folder.expect("the folder stays");
        }
        if let Some(console) = console {
            *console = self.b.consoles[side];
        }
        self.b.custom.sides[side].screen = old_screen;
        self.b.custom.sides[side].folder = old_folder;
        self.b.consoles[side] = old_console;
        self.b.custom.sides[side].emotion = old_emotion;
        self.b.custom.sides[side].joypad = old_joypad;
        r
    }
}

impl Extras for SideExtras<'_> {
    fn hand_size(&mut self) -> Option<u8> {
        self.b.systems_custom_hand_size(self.side)
    }

    fn buttons(&mut self, screen: &Screen) -> Vec<screen::ButtonPlace> {
        let content = self.b.content.clone();
        let mut screen = *screen;
        let side = self.side;
        let mut out = Vec::new();
        for button in self.b.side_buttons(side) {
            let shown = self.with_screen(&mut screen, None, |b| b.call_button(side, button, nettai_content_api::SystemHook::ButtonShown));
            if shown == nettai_content_api::Value::Bool(true) {
                let d = content.defs.button(button);
                out.push(screen::ButtonPlace { button, slot: d.slot, cells: d.cells, uses: d.uses, right: d.right, left: d.left });
            }
        }
        out
    }

    fn button_state(&mut self, screen: &Screen, button: crate::content::ButtonHandle) -> Option<SlotState> {
        self.b.content.defs.button(button).state?;
        let mut screen = *screen;
        let side = self.side;
        match self.with_screen(&mut screen, None, |b| b.call_button(side, button, nettai_content_api::SystemHook::ButtonState)) {
            nettai_content_api::Value::Int(0) => Some(SlotState::Selectable),
            nettai_content_api::Value::Int(_) => Some(SlotState::Unavailable),
            _ => None,
        }
    }

    fn button_pressed(&mut self, screen: &mut Screen, folder: &mut BattleFolder, button: crate::content::ButtonHandle) {
        let side = self.side;
        self.with_screen(screen, Some(folder), |b| b.call_button(side, button, nettai_content_api::SystemHook::ButtonPressed));
    }

    fn button_taken_back(&mut self, screen: &mut Screen, button: crate::content::ButtonHandle) {
        if self.b.content.defs.button(button).taken_back.is_none() {
            return;
        }
        let side = self.side;
        self.with_screen(screen, None, |b| b.call_button(side, button, nettai_content_api::SystemHook::ButtonTakenBack));
    }

    fn dealing(&mut self, screen: &mut Screen, folder: &mut BattleFolder, console: &mut Console) {
        let side = self.side;
        self.with_screen_console(screen, Some(folder), Some(console), None, |b| {
            b.systems_call_custom(side, nettai_content_api::SystemHook::CustomDeal)
        });
    }

    fn opened(&mut self, screen: &mut Screen) {
        let side = self.side;
        self.with_screen(screen, None, |b| b.systems_call_custom(side, nettai_content_api::SystemHook::CustomOpen));
    }

    fn confirmed(&mut self, screen: &mut Screen, folder: &mut BattleFolder) {
        let side = self.side;
        self.with_screen(screen, Some(folder), |b| b.systems_call_custom(side, nettai_content_api::SystemHook::CustomConfirmed));
    }

    fn chip_picked(&mut self, screen: &mut Screen, folder: &mut BattleFolder, chip: ChipHandle) {
        let side = self.side;
        self.with_screen(screen, Some(folder), |b| b.systems_call_custom_chip(side, nettai_content_api::SystemHook::CustomChipPicked, chip));
    }

    fn chip_taken_back(&mut self, screen: &mut Screen, chip: ChipHandle) {
        let side = self.side;
        self.with_screen(screen, None, |b| b.systems_call_custom_chip(side, nettai_content_api::SystemHook::CustomChipTakenBack, chip));
    }

    fn window_update(
        &mut self,
        screen: &mut Screen,
        folder: &mut BattleFolder,
        console: &mut Console,
        joy: &Joypad,
        window: crate::content::WindowHandle,
    ) -> bool {
        let side = self.side;
        self.with_screen_console(screen, Some(folder), Some(console), Some(joy), |b| b.call_window(side, window))
            == nettai_content_api::Value::Bool(true)
    }

    fn keys(&mut self, screen: &mut Screen, folder: &mut BattleFolder, joy: &Joypad) -> bool {
        let side = self.side;
        self.with_screen_console(screen, Some(folder), None, Some(joy), |b| b.systems_ask_custom(side, nettai_content_api::SystemHook::CustomKeys))
    }

    fn take_back(&mut self, screen: &mut Screen, folder: &BattleFolder) -> bool {
        let side = self.side;
        let mut folder = *folder;
        self.with_screen(screen, Some(&mut folder), |b| b.systems_ask_custom(side, nettai_content_api::SystemHook::CustomTakeBack))
    }
}

impl Battle {
    /// Run `f` on side `side`'s custom screen as a `custom.*` call does from
    /// a custom hook: the screen and the folder out of the side (back after),
    /// the side's view as the screen reads it, its console, and the side's
    /// extras for what the screen asks in turn. None when no screen is open.
    pub(crate) fn with_custom_screen<R>(
        &mut self,
        side: u8,
        f: impl FnOnce(&mut Screen, &PlayerView, &mut BattleFolder, &mut Console, &mut dyn Extras) -> R,
    ) -> Option<R> {
        let i = side as usize & 1;
        let (Some(mut screen), Some(mut folder)) = (self.custom.sides[i].screen, self.custom.sides[i].folder) else { return None };
        let copy = self.custom.sides[i].clone();
        let content = self.content.clone();
        let library = library::GameLibrary { content: &content, game: self.games.sides[i], ruleset: self.games.rulesets[i] };
        // (The emotion the screen reads, its own: a battle that checks the
        // traces' screens alone has no navi to ask.)
        let emotion = copy.emotion;
        let ctx = Context { emotion, ..self.custom_context_without_emotion(side, &library) };
        let view = copy.view(&ctx, folder.regular_pending);
        let mut console = self.consoles[i];
        let r = f(&mut screen, &view, &mut folder, &mut console, &mut SideExtras { b: self, side, emotion });
        self.custom.sides[i].screen = Some(screen);
        self.custom.sides[i].folder = Some(folder);
        self.consoles[i] = console;
        Some(r)
    }
}

#[cfg(test)]
mod tests;
