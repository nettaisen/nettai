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
pub mod folder;
pub mod library;
pub mod screen;

pub use folder::{BattleFolder, FolderChip, SavedFolder};
pub use library::Library;
pub use screen::{Phase, PlayerView, Request, RoundMemory, Screen, Slot, SlotKind, SlotState};

use crate::battle::{Battle, CustomResult, battle_flags};
use crate::content::ChipId;
use crate::hand::ChipHand;
use crate::input::Joypad;
use crate::kinds::player::Emotion;
use crate::setup::{Form, NaviStats, effects};
use crate::transform::TransformRequest;
use builder::{ClassCounts, Pick, ProgramAdvancesUsed};
use screen::SPECIAL_SLOT;

/// Which game a player plays: it decides their Crosses and Beast form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GameVersion {
    Gregar,
    #[default]
    Falzar,
}

/// What a player's save unlocks on the custom screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Unlocks {
    pub version: GameVersion,
    /// The Crosses owned (event flags 0xE7-0xEB), by Cross number.
    pub crosses: [bool; screen::CROSSES],
    /// Beast Out is unlocked (event flag 0xE0).
    pub beast_out: bool,
    /// Event flag 0x163 (its story meaning is unknown): no Beast Out
    /// button, and the Cross window then needs the navi to be MegaMan
    /// (instead of battle flag 0x40 clear).
    pub beast_out_sealed: bool,
}

impl Unlocks {
    /// Every Cross and Beast Out, as in a finished game.
    pub fn everything(version: GameVersion) -> Unlocks {
        Unlocks { version, crosses: [true; screen::CROSSES], beast_out: true, beast_out_sealed: false }
    }
}

/// What a player brings to a round that only their own console knows in
/// the original: the battle folder (shuffled at the round's init) and
/// what their save unlocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
}

impl Default for PlayerSetup {
    fn default() -> PlayerSetup {
        PlayerSetup {
            folder: Some(BattleFolder::empty()),
            unlocks: Unlocks::everything(GameVersion::Falzar),
            joypad_phase: 0,
            bug_frags: 0,
            navi_level: 0,
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
    /// The tick, and the link's latency: a result sent now arrives
    /// `50 + link_delay` ticks later (50 words, one a tick).
    pub now: u32,
    pub link_delay: u8,
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
        }
    }

    /// The custom screen opens (`sub_8026840`): the status bit goes up
    /// and the player's screen deals. The round's first screen forgets the
    /// previous round's Crosses and Beast Out.
    pub fn open(&mut self, ctx: &Context) {
        self.in_custom = true;
        self.built = None;
        self.sent = None;
        if ctx.turn == 1 {
            self.round = RoundMemory::default();
        }
        let Some(mut folder) = self.folder else { return };
        let mut round = self.round;
        let regular = folder.regular_pending;
        let screen = Screen::open(&mut folder, &self.view(ctx, regular), ctx.turn, &mut round);
        self.round = round;
        self.folder = Some(folder);
        self.screen = Some(screen);
    }

    /// One tick of the player's screen on their joypad. `damage`: a
    /// chip's damage for this player now (`sub_80109A4`), for the hand
    /// built at OK.
    pub fn tick(&mut self, ctx: &Context, damage: impl Fn(ChipId) -> u16) -> Option<Request> {
        let (Some(mut screen), Some(mut folder)) = (self.screen, self.folder) else { return None };
        let request = screen.tick(&self.joypad, &self.view(ctx, folder.regular_pending), &mut folder);
        match request {
            Some(Request::Confirm) => self.confirm(ctx, &mut screen, &mut folder, damage),
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
    /// chips out of the folder (`sub_80293F8`), and turn Beast Out or the
    /// Cross into a transformation (`sub_8029344`, `sub_802937A`).
    fn confirm(&mut self, ctx: &Context, screen: &mut Screen, folder: &mut BattleFolder, damage: impl Fn(ChipId) -> u16) {
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
            if p.chip.id >= 0x190 {
                self.round.navi_chips_used |= 1 << (p.chip.id - 0x18F);
            }
        }
        let form = ctx.stats.form;
        let version = self.unlocks.version;
        let mut transform = TransformRequest::NONE;
        if screen.selection().contains(&SPECIAL_SLOT) {
            transform.form = Some(if ctx.emotion == Emotion::Tired {
                version.beast_over()
            } else if form == Form::NONE {
                version.beast_out()
            } else {
                form.with_beast()
            });
            self.round.beast_out_used = true;
        }
        if let Some(cross) = screen.crosses.chosen {
            let f = version.cross_form(cross);
            transform.form = Some(if form.is_beast() { f.with_beast() } else { f });
            self.round.crosses_used[cross as usize] = true;
        }
        for &slot in screen.selection() {
            if let SlotKind::Chip { index, regular } = screen.slots[slot as usize].kind {
                folder.take(index as usize);
                if regular {
                    folder.regular_pending = false;
                }
            }
        }
        screen.program_advance = built.program_advance.map(|(_, chips)| chips);
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
        self.round.turn += 1;
        self.custom.ticks = 0;
        self.custom.committed = false;
        let content = self.content.clone();
        for side in 0..2u8 {
            let ctx = self.custom_context(side, &*content);
            self.custom.sides[side as usize].open(&ctx);
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
            let ctx = self.custom_context(side, &*content);
            let mut s = self.custom.sides[side as usize].clone();
            let request = s.tick(&ctx, |id| crate::hand::chip_damage(self, id, side));
            self.custom.sides[side as usize] = s;
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

#[cfg(test)]
mod tests;
