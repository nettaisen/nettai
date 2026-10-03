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
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle};
use crate::hand::ChipHand;
use crate::input::Joypad;
use crate::kinds::player::Emotion;
use crate::content::FormKind;
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

/// What a player's save unlocks on the custom screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Unlocks {
    pub version: GameVersion,
    /// The Crosses owned (Gregar's event flags 0xE2-0xE6, Falzar's
    /// 0xE7-0xEB), by Cross number.
    pub crosses: [bool; screen::CROSSES],
    /// Beast Out is unlocked (event flag 0xE0).
    pub beast_out: bool,
    /// Event flag 0x163, which marks a link navi operated (raised and
    /// lowered with the navi): no Beast Out button, and the Cross window
    /// then needs the navi to be MegaMan (instead of battle flag 0x40
    /// clear).
    pub beast_out_sealed: bool,
    /// The Crosses the setup names for the Cross window, in place of the
    /// version's that `crosses` owns: nettai's extension, which the
    /// original has no way to say (any Crosses, of either game;
    /// docs/engine/custom-screen.md §4.1). None: the original's.
    pub cross_list: Option<CrossList>,
}

impl Unlocks {
    /// Every Cross and Beast Out, as in a finished game.
    pub fn everything(version: GameVersion) -> Unlocks {
        Unlocks { version, crosses: [true; screen::CROSSES], beast_out: true, beast_out_sealed: false, cross_list: None }
    }

    /// The Cross in place `place` of the player's Crosses, the places the
    /// Cross window's entries and the round's record of Crosses used go
    /// by: the setup's list's entry, else the version's Cross with that
    /// number (none: the content has no such Cross).
    pub fn cross_at(&self, library: &dyn Library, navi: NaviHandle, place: u8) -> Option<FormHandle> {
        match &self.cross_list {
            Some(list) => list.get(place),
            None => library.cross_form(navi, self.version, place),
        }
    }

    /// Whether the player has the Cross in place `place`: the save owns
    /// it, or the setup's list names one there.
    pub fn owns_cross(&self, place: u8) -> bool {
        match &self.cross_list {
            Some(list) => list.get(place).is_some(),
            None => self.crosses.get(place as usize).copied().unwrap_or(false),
        }
    }

    /// The Beast form Beast Out takes a navi in `form` to (`sub_802937A`,
    /// `sub_802A040`): when `tired`, Beast Over (of `beast_game`'s game);
    /// from the base form the version's Beast Out; from a Cross that
    /// Cross's form in Beast Out (with a setup's Cross list, whichever
    /// game the Cross is from: HeatCross's Beast for a Falzar player in
    /// HeatCross, §4.1).
    pub fn beast_form(&self, library: &dyn Library, navi: NaviHandle, form: FormHandle, tired: bool) -> Option<FormHandle> {
        if tired {
            library.beast_over_form(navi, self.beast_game(library, form))
        } else if library.form_kind(form) == FormKind::Base {
            library.beast_out_form(navi, self.version)
        } else {
            library.form_in_beast_out(form)
        }
    }

    /// The game of the Beast a navi in `form` goes into, or is in: the
    /// player's version, except that with a setup's Cross list a form of
    /// the other game (one of its Crosses, or a Beast form of one) is that
    /// game's (§4.1). Beast Over and the custom screen's Beast Out roar
    /// follow it, and a frontend draws the Beast Out button and pictures
    /// of its game.
    pub fn beast_game(&self, library: &dyn Library, form: FormHandle) -> GameVersion {
        match library.form_game(form) {
            Some(game) if self.cross_list.is_some() && library.form_kind(form) != FormKind::Base => game,
            _ => self.version,
        }
    }
}

/// The Crosses a setup names for a player's Cross window
/// (`Unlocks::cross_list`): up to five forms, each a Cross, which the
/// window offers in this order (those not used this round, and not the
/// navi's starting form).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CrossList {
    forms: [Option<FormHandle>; screen::CROSSES],
}

impl CrossList {
    /// The list of `forms`, at most the window's five.
    pub fn new(forms: &[FormHandle]) -> CrossList {
        assert!(forms.len() <= screen::CROSSES, "a Cross window offers at most {} Crosses, not {}", screen::CROSSES, forms.len());
        let mut list = CrossList::default();
        for (slot, &f) in list.forms.iter_mut().zip(forms) {
            *slot = Some(f);
        }
        list
    }

    /// The Cross in place `place`.
    pub fn get(&self, place: u8) -> Option<FormHandle> {
        self.forms.get(place as usize).copied().flatten()
    }

    /// The Crosses, in order.
    pub fn forms(&self) -> impl Iterator<Item = FormHandle> + '_ {
        self.forms.iter().flatten().copied()
    }
}

/// What a player brings to a round that only their own console knows in
/// the original: the battle folder (shuffled at the round's init) and
/// what their save unlocks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerSetup {
    /// The shuffled battle folder. None only when checking against a
    /// recording that lacks this player's folder: their screen is then
    /// not simulated, and the recording supplies what it sends
    /// (`TickEvents::recorded`).
    pub folder: Option<BattleFolder>,
    pub unlocks: Unlocks,
    /// The joypad's auto-repeat beat (0-4) on the round's first tick; each
    /// console counts its own.
    pub joypad_phase: u8,
    /// The save's bug frags (a dark chip spends one) and the link navi's
    /// level (its chip bonus), which the init exchange shares.
    pub bug_frags: u32,
    pub navi_level: u8,
    /// What the player's console brings besides: its RNG (RNG1), which
    /// ChpShufl's re-deal draws from (`crate::console`). In netplay it is
    /// part of the setup the peers exchange.
    pub console: ConsoleSetup,
    /// The rules the player plays by (docs/design/rules-in-luau.md §2.3);
    /// none: the content's stock ruleset.
    pub ruleset: Option<nettai_content_api::RulesetHandle>,
    /// What the player brings for each system of their ruleset (its
    /// `setup` fields), in the ruleset's order; none given: all zero
    /// (`PlayerSetup::set_rule` writes one by name).
    pub rules: Vec<nettai_content_api::ContentState>,
    /// The patch cards the player has installed (`crate::patch_cards`):
    /// their ruleset's rules apply them (BN6's patch-cards system).
    pub patch_cards: crate::patch_cards::PatchCards,
    /// The player's NaviCust (`crate::navicust`), which their ruleset's
    /// rules compile into the navi's stats as the round is set up (BN6's
    /// navicust system); none: the stats are the setup's as they are (a
    /// recording's, which the original's NaviCust has already made).
    pub navicust: Option<crate::navicust::NaviCust>,
}

impl Default for PlayerSetup {
    fn default() -> PlayerSetup {
        PlayerSetup {
            folder: Some(BattleFolder::empty()),
            unlocks: Unlocks::everything(GameVersion::Falzar),
            joypad_phase: 0,
            bug_frags: 0,
            navi_level: 0,
            console: ConsoleSetup::default(),
            ruleset: None,
            rules: Vec::new(),
            patch_cards: Default::default(),
            navicust: None,
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
    pub unlocks: Unlocks,
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
}

impl Side {
    pub fn new(setup: &PlayerSetup) -> Side {
        Side {
            unlocks: setup.unlocks,
            joypad: Joypad::new(setup.joypad_phase),
            folder: setup.folder,
            round: RoundMemory::default(),
            program_advances: ProgramAdvancesUsed::default(),
            class_uses: ClassCounts::default(),
            screen: None,
            in_custom: false,
            built: None,
            sent: None,
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
}

/// Ticks a result takes to send: the link carries one of its 50 words a
/// tick (`sub_801FF18`).
pub const SEND_TICKS: u32 = 50;

impl Side {
    fn view<'a>(&'a self, ctx: &'a Context, regular_pending: bool) -> PlayerView<'a> {
        PlayerView {
            library: ctx.library,
            stats: &ctx.stats,
            emotion: ctx.emotion,
            unlocks: &self.unlocks,
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
        let mut screen = Screen::open(&mut folder, &self.view(ctx, regular), ctx.turn, extras);
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
        let (Some(mut screen), Some(mut folder)) = (self.screen, self.folder) else { return None };
        let request = screen.tick(&self.joypad, &self.view(ctx, folder.regular_pending), &mut folder, console, extras);
        match request {
            Some(Request::Confirm) => self.confirm(ctx, &mut screen, &mut folder, console, damage),
            Some(Request::Send) => {
                // sub_8026DC4's first tick: sub_802A4FC counts the classes,
                // sub_800B3A2 sends the hand, the navi's stats as they are
                // now, and the transformation.
                let (hand, transform) = self.built.clone().expect("a hand was built at OK");
                if let Some(h) = &hand {
                    builder::count_classes(h, &mut self.class_uses, ctx.library);
                }
                let result = CustomResult { hand, navi_stats: ctx.stats, transform };
                self.sent = Some(Sent { result, sent_at: ctx.now, arrives: ctx.now + SEND_TICKS + ctx.link_delay as u32 });
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
        let mut pa_used = self.program_advances;
        let built = builder::build(&picks, ctx.turn, &mut pa_used, ctx.library, damage);
        self.program_advances = pa_used;
        for p in &picks {
            // A link navi's own chip is spent for the round (the bit of
            // its navi).
            if let Some(navi) = ctx.library.own_chip_of(p.chip.id) {
                self.round.navi_chips_used |= 1 << navi.0;
            }
        }
        // (The original's forms by number: the game's Beast Over, Beast
        // Out, or the Cross's form 0xC past it; a Cross by its number, in
        // Beast Out its form 0xC past it.)
        let (navi, form) = (ctx.stats.navi, ctx.stats.form);
        let kind = ctx.library.form_kind(form);
        let mut transform = TransformRequest::NONE;
        if screen.selection().contains(&SPECIAL_SLOT) {
            transform.form = self.unlocks.beast_form(ctx.library, navi, form, ctx.emotion == Emotion::Tired);
            self.round.beast_out_used = true;
        }
        if let Some(cross) = screen.crosses.chosen {
            let f = self.unlocks.cross_at(ctx.library, navi, cross);
            transform.form = if kind.is_beast() { f.and_then(|f| ctx.library.form_in_beast_out(f)) } else { f };
            self.round.crosses_used[cross as usize] = true;
        }
        for &slot in screen.selection() {
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
        Context {
            library,
            stats: self.stats[side as usize],
            emotion: crate::kinds::player::emotion(self, side),
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
            s.open_with(&ctx, &mut console, &mut SideExtras { b: self, side });
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
            let request = s.tick_with(&ctx, &mut console, damage, &mut SideExtras { b: self, side });
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
    pub fn custom_extras(&mut self, side: u8) -> impl Extras + '_ {
        SideExtras { b: self, side }
    }
}

/// A side's custom screen's extras: its systems' `custom` hooks.
struct SideExtras<'b> {
    b: &'b mut Battle,
    side: u8,
}

impl SideExtras<'_> {
    /// Run `f` with the side's screen (and folder) back in the battle, for
    /// `custom.*` to reach, and take them back after.
    fn with_screen<R>(&mut self, screen: &mut Screen, folder: Option<&mut BattleFolder>, f: impl FnOnce(&mut Battle) -> R) -> R {
        let side = self.side as usize & 1;
        let (old_screen, old_folder) = (self.b.custom.sides[side].screen, self.b.custom.sides[side].folder);
        self.b.custom.sides[side].screen = Some(*screen);
        if let Some(f) = &folder {
            self.b.custom.sides[side].folder = Some(**f);
        }
        let r = f(self.b);
        *screen = self.b.custom.sides[side].screen.expect("the screen stays");
        if let Some(folder) = folder {
            *folder = self.b.custom.sides[side].folder.expect("the folder stays");
        }
        self.b.custom.sides[side].screen = old_screen;
        self.b.custom.sides[side].folder = old_folder;
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
}

#[cfg(test)]
mod tests;
