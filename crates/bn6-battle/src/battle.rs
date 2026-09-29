//! The battle: round state, the per-tick order of operations, and the flow
//! state machines (intro, banner, custom screen, fighting, results, end).
//! See docs/engine/battle-flow.md.

use crate::actor::{ActorId, Actors};
use crate::collision::Collision;
use crate::field::Field;
use crate::hand::ChipHand;
use crate::hud::{Banner, BannerStatus, CustomGauge};
use crate::input::{InputRecord, PlayerTick, keys};
use crate::object::{ObjectRef, Objects};
use crate::rng::Rng;
use crate::data::BannerId;
use crate::setup::{BattleSettings, Form, Navi, NaviStats, RoundSetup, SetScore, effects};
use crate::transform::{TransformRequest, TransformSequencer};
use crate::sound::{SoundCue, SoundId};

/// Battle flag bits.
pub mod battle_flags {
    /// The fight has started (collision is live).
    pub const FIGHTING: u16 = 0x01;
    /// The custom gauge is full.
    pub const GAUGE_FULL: u16 = 0x02;
    /// Time stop (time-freeze chips).
    pub const TIME_STOP: u16 = 0x04;
    /// A player asked to open the custom screen.
    pub const CUSTOM_REQUESTED: u16 = 0x10;
    /// Per-player custom gauges and chip counters (`sub_802E112`). Never set
    /// in netbattles (battle mode 1) or random battles.
    pub const PER_PLAYER_GAUGES: u16 = 0x40;
}

/// Top-level battle states (the game's jump-table offsets).
pub mod top {
    pub const INIT: u8 = 0x0;
    pub const RUNNING: u8 = 0x4;
    pub const END: u8 = 0x8;
}

/// Battle-mode handler states (netbattle).
pub mod mode {
    pub const INTRO: u8 = 0x00;
    pub const BANNER: u8 = 0x04;
    pub const CUSTOM: u8 = 0x08;
    pub const FIGHTING: u8 = 0x0C;
    pub const FADE_OUT: u8 = 0x14;
}

/// Fighting-phase states.
pub mod fight {
    pub const SETUP: u8 = 0x00;
    pub const START_BANNER: u8 = 0x04;
    pub const FIGHTING: u8 = 0x08;
    pub const WIN: u8 = 0x0C;
    pub const LOSE: u8 = 0x10;
    pub const DRAW: u8 = 0x14;
    pub const JUDGE: u8 = 0x18;
    pub const PAUSE: u8 = 0x1C;
    pub const CUSTOM_REVERT: u8 = 0x20;
    pub const CUSTOM_SEQUENCE: u8 = 0x24;
}

/// Round-level state (the game's BattleState).
#[derive(Clone, Debug, Default)]
pub struct RoundState {
    pub top: u8,
    pub mode: u8,
    pub sub: u8,
    pub init: u8,
    /// Actors counted per side (drops when an actor is removed).
    pub actor_count: [u8; 2],
    pub layout: u8,
    /// Custom screens opened so far (the turn number).
    pub turn: u8,
    pub name_counts: [u8; 2],
    pub running: u8,
    pub time_up: u8,
    pub local_side: u8,
    /// Cycles mod 20 and mod 180 (grass healing).
    pub cycle20: u8,
    pub cycle180: u8,
    pub mode_copy: u8,
    pub winner: u8,
    /// This side's status bits (bit 2: in the custom screen).
    pub status: u8,
    /// Alive navis per side.
    pub alive: [u8; 2],
    /// Both players' status bits as received.
    pub remote_status: [u8; 2],
    pub has_regular: u8,
    pub wins: u8,
    pub losses: u8,
    pub round: u8,
    pub max_combo: u8,
    pub combo: u8,
    pub combo_window: u8,
    pub busting_level: u8,
    pub result: u8,
    /// Low-HP music latch (`sub_8009158`).
    pub low_hp_music: bool,
    /// Small countdown used by banners and the end state.
    pub delay: i16,
    pub flags: u16,
    /// The local navi's HP when the battle ended (`sub_800FAE0`).
    pub exit_hp: u16,
    pub escape: u16,
    /// Ticks of fighting (capped).
    pub battle_time: u32,
    pub has_tags: u8,
    pub tag_index: u8,
    /// Navi name ids per side (four each).
    pub name_ids: [[u16; 4]; 2],
    /// Intro progress bits (0x10 fade started, 0x01 fade done, 0x02 all
    /// navis in); 0x04/0x08 chips enabled per side.
    pub intro_bits: u8,
    pub frames: u32,
    pub ticks: u32,
    /// Alive actors, four slots per side.
    pub alive_actors: [[Option<ObjectRef>; 4]; 2],
    /// The actor list as spawned.
    pub spawned_actors: [[Option<ObjectRef>; 4]; 2],
}

/// The fighting-phase machine.
#[derive(Clone, Debug, Default)]
pub struct FightMachine {
    pub state: u8,
    pub sub: u8,
    pub init: u8,
    /// 0 none, 1 local win, 2 local loss, 3 draw, 6 open the custom screen.
    pub result: u8,
    pub pausing_player: u8,
    pub timer: i16,
    pub turn_timer: u16,
}

/// Screen fade progress (only its duration matters to the simulation).
#[derive(Clone, Copy, Debug, Default)]
pub struct Fade {
    pub remaining: u8,
}

impl Fade {
    /// Fades take this many ticks at the speed netbattles use.
    pub const TICKS: u8 = 17;

    pub fn start(&mut self) {
        self.remaining = Self::TICKS;
    }
    pub fn active(&self) -> bool {
        self.remaining > 0
    }
    /// Screen fades step once per frame, outside the battle tick.
    pub fn step(&mut self) {
        self.remaining = self.remaining.saturating_sub(1);
    }
}

/// A player's custom-screen result, as exchanged when the screen closes.
#[derive(Clone, Debug)]
pub struct CustomResult {
    /// The chosen hand (None = no chips chosen: the previous hand stays).
    pub hand: Option<ChipHand>,
    pub navi_stats: NaviStats,
    /// Cross/Beast transformation request.
    pub transform: TransformRequest,
}

/// Events from outside the simulation that happen on a tick.
#[derive(Clone, Debug, Default)]
pub struct TickEvents {
    /// The local player confirmed their custom-screen selection.
    pub local_confirm: bool,
    /// Both players' custom-screen results arrived.
    pub exchange: Option<Box<[CustomResult; 2]>>,
    /// After the round, the link session the end state asked to close has
    /// closed.
    pub link_closed: bool,
}

pub struct Battle {
    pub setup: RoundSetup,
    pub stats: [NaviStats; 2],
    pub rng: Rng,
    pub round: RoundState,
    pub fight: FightMachine,
    pub gauge: CustomGauge,
    pub banner: Banner,
    pub paused: bool,
    pub inputs: [InputRecord; 2],
    pub hands: [ChipHand; 2],
    /// Both players' transformation requests from the last custom screen.
    pub transform_requests: [TransformRequest; 2],
    /// The requests as this turn started (`unk_203A980`): what each navi
    /// changes into.
    pub turn_transforms: [TransformRequest; 2],
    /// The transformation sequencer run at the start of each turn.
    pub transform_seq: TransformSequencer,
    /// Per side: the navi went Beast Out this battle (`byte_203EAE0` +2,
    /// read after the battle: a navi that did not gets a turn back).
    pub beast_out_used: [bool; 2],
    /// Per side: the navi crossed this battle (`byte_203EAE0` +0xB,
    /// read after the battle for the busting level).
    pub changed_form: [bool; 2],
    pub objects: Objects,
    pub actors: Actors,
    pub collision: Collision,
    pub field: Field,
    pub fade: Fade,
    /// Navis waiting to fade in at the intro.
    pub fadein_queue: [Option<ObjectRef>; 8],
    /// Per-side damage-carry records (`dword_203CFB0`).
    pub damage_carry: [DamageCarry; 2],
    /// Custom-screen UI progress on this side (local presentation that the
    /// simulation observes in a few places).
    pub custom_ui: CustomUi,
    /// Per-side extra battle state (`sub_802E070`), used by the battle-flag
    /// 0x40 mode.
    pub sides: [SideState; 2],
    /// Per-side statistics counters (`byte_203EAE0`, `sub_800AB46`).
    pub side_stats: [[u8; 16]; 2],
    /// Per-side registry of defensive chips and their linked objects
    /// (0x10 bytes per side at 0x02036720).
    pub linked: [LinkedRecord; 2],
    /// Per side: its time freeze (`byte_203CF00`).
    pub freeze: [crate::time_freeze::FreezeRecord; 2],
    /// Sound calls made this tick (output only; see `sound`).
    sound: Vec<SoundCue>,
    /// How the round ended, once the end state is through.
    outcome: Option<RoundEnd>,
}

/// How a round ended (`sub_8007CA0`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoundEnd {
    /// The set goes on. The next round starts with its own init (link
    /// sync, the navi stats and RNG exchange) from these settings and the
    /// score so far.
    NextRound { settings: BattleSettings, score: SetScore },
    /// The battle is over.
    Over(BattleResult),
}

/// The battle's result from the local side's perspective (BattleState
/// +0x1F, which the game hands back to the menu that started the battle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleResult {
    Won = 1,
    Lost = 2,
    Drawn = 3,
}

/// Where a set stands after a round (`sub_800AF50`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SetStanding {
    Undecided,
    Decided(BattleResult),
}

/// A side's extra battle state (0x1D0 bytes at `sub_802E070(side)`); only
/// the fields the engine reads are modeled (the rest are listed in
/// docs/engine/field-names.md). All zero outside the battle flag 0x40
/// mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SideState {
    pub active: u8,
    pub panel_x: u8,
    /// A per-side gauge (a SELECT special needs 0x1500; counters add it).
    pub gauge: u16,
    pub select_special: u8,
    pub cross_special: u8,
}

/// A side's defensive-chip record: the chip and the object that
/// implements it. (The game's record also keeps two values from the
/// registering chip and its owner; nothing ported registers one, see
/// docs/engine/field-names.md.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinkedRecord {
    pub chip: u16,
    pub object: Option<ObjectRef>,
}

impl Battle {
    /// `sub_800AB46`: bump a statistics counter (saturating at 0xFF).
    pub fn bump_side_stat(&mut self, side: u8, index: usize, n: u8) {
        let v = &mut self.side_stats[side as usize][index];
        *v = v.saturating_add(n);
    }

    /// `sub_802CEA6`: clear a side's defensive-chip record, telling its
    /// object to end (Param2 = 1).
    pub fn clear_linked(&mut self, side: u8) {
        let rec = std::mem::take(&mut self.linked[side as usize]);
        if let Some(o) = rec.object {
            self.objects.get_mut(o).params[1] = 1;
        }
    }

    /// `battle_networkInvert`: whether `alliance` is not the local side.
    pub fn is_remote(&self, alliance: u8) -> bool {
        alliance ^ self.round.local_side != 0
    }

    /// `sub_800AA1A`: add `r` to the intro fade-in queue (unless present).
    pub fn fadein_enqueue(&mut self, r: ObjectRef) -> bool {
        for slot in self.fadein_queue.iter_mut() {
            match slot {
                Some(o) if *o == r => return false,
                None => {
                    *slot = Some(r);
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    /// `sub_800AA06`: whether `r` is first in the fade-in queue.
    pub fn fadein_is_head(&self, r: ObjectRef) -> bool {
        self.fadein_queue[0] == Some(r)
    }

    /// `sub_800AA40`: remove `r` from the fade-in queue, then compact it
    /// (`sub_800AA64`, which only looks at the first seven entries).
    pub fn fadein_dequeue(&mut self, r: ObjectRef) {
        for slot in self.fadein_queue.iter_mut() {
            if *slot == Some(r) {
                *slot = None;
            }
        }
        let kept: Vec<ObjectRef> = self.fadein_queue[..7].iter().flatten().copied().collect();
        for (i, slot) in self.fadein_queue[..7].iter_mut().enumerate() {
            *slot = kept.get(i).copied();
        }
    }
}

/// A side's damage-carry record: damage this tick and last tick, and the
/// objects it tracks.
#[derive(Clone, Copy, Debug, Default)]
pub struct DamageCarry {
    pub this_tick: u16,
    pub previous: u16,
    pub source: Option<ObjectRef>,
    pub target: Option<ObjectRef>,
}

/// Local custom-screen UI progress.
#[derive(Clone, Debug, Default)]
pub struct CustomUi {
    /// Results that arrived this tick, installed by the custom screen.
    pub pending: Option<Box<[CustomResult; 2]>>,
    /// Ticks since the screen opened.
    pub ticks: u32,
    /// Ticks since the local player confirmed.
    pub since_confirm: Option<u32>,
    /// The exchange was installed; the screen closes next tick.
    pub installed: bool,
}

impl Battle {
    /// Start a round: the state the game is in when its init finishes and
    /// the first battle tick is about to run.
    pub fn new(setup: RoundSetup) -> Battle {
        let score = setup.score;
        let mut b = Battle {
            stats: setup.navi_stats,
            rng: Rng::new(setup.rng),
            round: RoundState {
                running: 1,
                max_combo: score.max_combo.max(1),
                wins: score.wins,
                losses: score.losses,
                round: score.round,
                layout: setup.settings.layout,
                mode_copy: setup.settings.mode,
                local_side: setup.local_side,
                intro_bits: 0x0C,
                low_hp_music: setup.low_hp_music_latched,
                top: top::RUNNING,
                ..RoundState::default()
            },
            fight: FightMachine::default(),
            gauge: CustomGauge::new(),
            banner: Banner::default(),
            paused: false,
            inputs: [InputRecord::default(); 2],
            hands: [ChipHand::empty(), ChipHand::empty()],
            transform_requests: [TransformRequest::NONE; 2],
            turn_transforms: [TransformRequest::NONE; 2],
            transform_seq: TransformSequencer::default(),
            beast_out_used: [false; 2],
            changed_form: [false; 2],
            objects: Objects::new(),
            actors: Actors::default(),
            collision: Collision::new(),
            field: Field::new(setup.settings.layout, setup.settings.panel_pattern, setup.settings.mode),
            fade: Fade::default(),
            fadein_queue: [None; 8],
            damage_carry: [DamageCarry::default(); 2],
            custom_ui: CustomUi::default(),
            sides: [SideState::default(); 2],
            side_stats: [[0; 16]; 2],
            linked: [LinkedRecord::default(); 2],
            freeze: Default::default(),
            sound: Vec::new(),
            outcome: None,
            setup,
        };
        // Init's last steps: refresh every panel, then one unpaused panel
        // update.
        b.field.refresh_all(&b.collision);
        b.tick_panels();
        b
    }

    pub fn is_time_stop(&self) -> bool {
        self.round.flags & battle_flags::TIME_STOP != 0
    }

    pub fn set_flags(&mut self, f: u16) {
        self.round.flags |= f;
    }

    pub fn clear_flags(&mut self, f: u16) {
        self.round.flags &= !f;
    }

    /// `battle_isBattleOver`: a side has no navis left, or time is up.
    pub fn is_battle_over(&self) -> bool {
        self.round.alive[0] == 0 || self.round.alive[1] == 0 || self.round.time_up != 0
    }

    /// `battle_isBattleOver` as seven callers use it: they test the CPU
    /// flag the function leaves, which reads "over" only for time-up. A KO
    /// looks like "not over" to them.
    pub fn is_battle_over_flag_quirk(&self) -> bool {
        self.round.time_up != 0 && self.round.alive[0] != 0 && self.round.alive[1] != 0
    }

    /// The player navi of a side (`sub_80103BC`).
    pub fn player(&self, side: u8) -> Option<ObjectRef> {
        let r = self.round.spawned_actors[side as usize][0]?;
        let a = self.objects.get(r).actor?;
        (self.actors.get(a).actor_type == crate::actor::ActorType::Player).then_some(r)
    }

    pub fn player_actor(&self, side: u8) -> Option<ActorId> {
        self.player(side).and_then(|r| self.objects.get(r).actor)
    }

    /// Report a sound call of the original (output only).
    pub fn play_sound(&mut self, cue: impl Into<SoundCue>) {
        self.sound.push(cue.into());
    }

    /// The sound calls of the last tick, in the order the game makes them.
    pub fn sound_cues(&self) -> &[SoundCue] {
        &self.sound
    }

    /// One battle tick (one frame of the running battle).
    pub fn tick(&mut self, input: &[PlayerTick; 2], events: TickEvents) {
        self.sound.clear();
        match self.round.top {
            top::RUNNING => self.tick_running(input, events),
            top::END => self.tick_end(&events),
            _ => {}
        }
        self.round.frames = self.round.frames.wrapping_add(1);
        self.fade.step();
    }

    fn tick_running(&mut self, input: &[PlayerTick; 2], events: TickEvents) {
        // Apply both players' packets.
        for (p, t) in input.iter().enumerate() {
            self.inputs[p].update(t.held | keys::PRESENT);
            self.round.remote_status[p] = if t.in_custom { 4 } else { 0 };
        }
        if events.local_confirm {
            self.custom_ui.since_confirm = Some(0);
        }
        if let Some(results) = events.exchange {
            self.custom_ui.pending = Some(results);
        }

        self.run_mode_handler();
        self.run_objects();
        if !self.paused && !self.is_time_stop() {
            self.tick_panels();
        }
        self.update_player_hands();
        self.run_hud_tasks();
        self.update_linked_registry();
        self.refresh_variable_damage();
        if !self.paused {
            if !self.is_time_stop() {
                self.round.cycle20 = (self.round.cycle20 + 1) % 20;
                self.round.cycle180 = (self.round.cycle180 + 1) % 180;
            }
            self.shift_damage_carry();
        }
        self.custom_hp_drain(0);
        if self.setup.settings.effects & effects::LINK != 0 {
            self.custom_hp_drain(1);
        }
        self.round.ticks = self.round.ticks.wrapping_add(1);
    }

    /// Top state 8 (`sub_8007B80`): 11 ticks of objects still running,
    /// then close the link session (mode 0, `sub_8007B9C`). Once it has
    /// closed (mode 4) the round is over (`sub_8007CA0`).
    fn tick_end(&mut self, events: &TickEvents) {
        if self.round.mode != 0 {
            if self.outcome.is_none() {
                self.finish_round();
            }
            return;
        }
        match self.round.sub {
            0 => {
                // sub_8007BD0
                if self.round.init == 0 {
                    self.round.delay = 10;
                    self.round.init = 4;
                } else {
                    self.round.delay -= 1;
                    if self.round.delay < 0 {
                        self.round.sub = 4;
                        self.round.init = 0;
                    }
                }
            }
            _ => {
                // sub_8007C14: ask the link to close, then wait for it.
                if self.round.init == 0 {
                    self.round.init = 4;
                } else if events.link_closed {
                    self.round.mode = 4;
                    self.round.sub = 0;
                    self.round.init = 0;
                }
            }
        }
        self.run_objects();
        if !self.paused && !self.is_time_stop() {
            self.tick_panels();
        }
    }

    /// How the round ended, once the end state is through.
    pub fn round_end(&self) -> Option<&RoundEnd> {
        self.outcome.as_ref()
    }

    /// `sub_8007CA0`: chain the set's next round, or end the battle.
    fn finish_round(&mut self) {
        self.play_sound(SoundCue::StopMusic);
        let code = self.round.result & 0xF;
        if matches!(code, 5 | 9 | 0xA) {
            panic!("ending a battle with result code {code} (sub_8007CA0) is not implemented yet");
        }
        if self.setup.settings.effects & effects::SET != 0 {
            match self.set_standing() {
                SetStanding::Undecided => return self.chain_next_round(),
                // setTwoStructs_800A840
                SetStanding::Decided(r) => self.round.result = r as u8,
            }
        }
        let result = match self.round.result & 0xF {
            1 => BattleResult::Won,
            2 => BattleResult::Lost,
            3 => BattleResult::Drawn,
            c => panic!("battle result code {c}"),
        };
        // (A win also counts toward a save-data statistic, dword_2000B30.)
        // sub_800FAE0: the local navi's HP, read from its object although
        // the fade-out freed it.
        if let Some(r) = self.player(self.round.local_side) {
            self.round.exit_hp = self.objects.get(r).hp;
        }
        // The rest updates the PET navi and rewards outside the battle and
        // hands the result back (loc_8007E38).
        self.paused = false;
        self.round.running = 0;
        self.outcome = Some(RoundEnd::Over(result));
    }

    /// `sub_800AF50`: a best-of-three set is decided once one side can no
    /// longer be caught, or after the third round.
    fn set_standing(&self) -> SetStanding {
        let r = &self.round;
        let left = 3 - r.round as i32;
        let (wins, losses) = (r.wins as i32, r.losses as i32);
        if wins > losses + left {
            SetStanding::Decided(BattleResult::Won)
        } else if losses > wins + left {
            SetStanding::Decided(BattleResult::Lost)
        } else if r.round >= 3 {
            SetStanding::Decided(BattleResult::Drawn)
        } else {
            SetStanding::Undecided
        }
    }

    /// The set goes on (`battleSettings_802D2B2`, `loc_8007204`): the next
    /// round is fought on the stage drawn for it and starts over with its
    /// init, keeping the score.
    fn chain_next_round(&mut self) {
        let stage = self.setup.later_stages[self.round.round as usize - 1];
        let settings = self.setup.next_settings(stage);
        let r = &self.round;
        let score = SetScore { wins: r.wins, losses: r.losses, round: r.round, max_combo: r.max_combo };
        self.round.top = top::INIT;
        self.round.mode = 0;
        self.round.sub = 0;
        self.round.init = 0;
        self.outcome = Some(RoundEnd::NextRound { settings, score });
    }

    /// Run every object's update in list order, with pause/time-stop gating.
    pub fn run_objects(&mut self) {
        let mut cur = self.objects.loop_first();
        while let Some(r) = cur {
            let f = self.objects.get(r).flags;
            let mut run = true;
            if self.paused && f & crate::object::flags::RUN_WHILE_PAUSED == 0 {
                run = false;
            }
            if run && self.is_time_stop() && f & crate::object::flags::RUN_IN_TIME_STOP == 0 {
                run = false;
            }
            if run {
                crate::kinds::update(self, r);
            }
            cur = self.objects.loop_next();
        }
    }

    // ---- Battle-mode handler -------------------------------------------

    fn run_mode_handler(&mut self) {
        match self.round.mode {
            mode::INTRO => self.mode_intro(),
            mode::BANNER => self.mode_banner(),
            mode::CUSTOM => self.mode_custom(),
            mode::FIGHTING => self.mode_fighting(),
            mode::FADE_OUT => self.mode_fade_out(),
            m => panic!("battle mode state {m:#x} not supported in netbattles"),
        }
        self.low_hp_music();
    }

    fn mode_intro(&mut self) {
        if self.round.init == 0 {
            let s = &self.setup.settings;
            if s.effects & effects::SET == 0 {
                self.round.round = s.battle_number;
            } else {
                self.round.round += 1;
            }
            crate::kinds::intro::spawn(self);
            self.spawn_actors();
            // Reward-chip pick: draws once; netbattle navis have no rewards.
            self.rng.next_positive();
            self.paused = true;
            self.gauge.rate = CustomGauge::rate_for(self.stats[0].gauge_speed, self.stats[1].gauge_speed);
            let link = self.setup.settings.effects & effects::LINK != 0;
            let music = if link { SoundId::VIRUS_BATTLE } else { SoundId(self.setup.settings.music as u16) };
            if music != SoundId::NO_MUSIC {
                self.play_sound(SoundCue::Music(music));
            }
            self.round.init = 4;
            return;
        }
        if self.round.sub == 0 {
            self.round.sub = 4;
        }
        if self.round.intro_bits & 0x02 != 0 {
            self.enter_mode(mode::BANNER);
        }
    }

    fn enter_mode(&mut self, m: u8) {
        self.round.mode = m;
        self.round.sub = 0;
        self.round.init = 0;
    }

    /// `sub_8007368`: spawn the settings' actor list. Only navis join the
    /// alive/actor bookkeeping; rocks and other field objects don't.
    pub fn spawn_actors(&mut self) {
        use crate::setup::ActorKind;
        for entry in self.setup.settings.actors {
            match entry.kind {
                ActorKind::Navi => {}
                ActorKind::Rock { variant } => {
                    crate::kinds::rock::spawn_at_start(self, entry.x, entry.y, variant);
                    continue;
                }
                k => panic!("actor list entries of kind {k:?} are not implemented yet"),
            }
            let r = crate::kinds::player::spawn(self, entry);
            let side = entry.alliance as usize;
            if let Some(r) = r {
                let counted = self.objects.get(r).actor.map(|a| self.actors.get(a).not_counted != 1).unwrap_or(true);
                if let Some(slot) = self.round.alive_actors[side].iter_mut().find(|s| s.is_none()) {
                    *slot = Some(r);
                }
                if counted {
                    self.round.actor_count[side] += 1;
                }
                let n = self.round.name_counts[side] as usize;
                if n < 4 {
                    self.round.name_ids[side][n] = self.objects.get(r).name_id;
                }
                self.round.name_counts[side] += 1;
            }
        }
        self.round.alive = self.round.actor_count;
        self.round.spawned_actors = self.round.alive_actors;
    }

    fn mode_banner(&mut self) {
        match self.round.sub {
            0 => {
                if self.round.round == 0 {
                    self.enter_mode(mode::CUSTOM);
                    return;
                }
                if self.round.init == 0 {
                    self.round.delay = 10;
                    self.round.init = 4;
                    return;
                }
                self.round.delay -= 1;
                if self.round.delay < 0 {
                    self.round.sub = 4;
                    self.round.init = 0;
                }
            }
            4 => {
                if self.round.init == 0 {
                    self.banner.start(BannerId(0x30));
                    self.round.init = 4;
                } else if self.banner.status() == BannerStatus::Done {
                    self.round.sub = 8;
                    self.round.init = 0;
                }
            }
            _ => {
                if self.round.init == 0 {
                    self.round.delay = 10;
                    self.round.init = 4;
                    return;
                }
                self.round.delay -= 1;
                if self.round.delay < 0 {
                    self.enter_mode(mode::CUSTOM);
                }
            }
        }
    }

    // ---- Custom screen ---------------------------------------------------

    fn mode_custom(&mut self) {
        if self.round.init == 0 {
            self.round.status |= 4;
            self.round.init = 1;
            self.open_custom();
        }
        let first_screen = self.round.turn == 1;
        let ui = &mut self.custom_ui;
        ui.ticks += 1;
        // Slide-in completes 10 ticks in: the NaviCust custom-HP bug bites
        // (except on the first screen of the battle).
        if ui.ticks == 11 && !first_screen {
            for side in 0..2 {
                self.custom_hp_bug(side);
            }
        }
        let ui = &mut self.custom_ui;
        if let Some(t) = ui.since_confirm.as_mut() {
            let n = *t;
            *t += 1;
            match n {
                1 => self.round.status &= !4,
                11 => self.gauge.enabled = true,
                _ => {}
            }
        }
        if self.custom_ui.installed {
            self.play_sound(SoundCue::RestoreVolume);
            for side in 0..2 {
                if let Some(a) = self.player_actor(side) {
                    self.actors.get_mut(a).beast_out_check_delay = 1;
                }
            }
            self.custom_ui = CustomUi::default();
            self.enter_mode(mode::FIGHTING);
            return;
        }
        if let Some(results) = self.custom_ui.pending.take() {
            self.install_exchange(*results);
        }
    }

    fn open_custom(&mut self) {
        self.gauge.value = 0;
        self.clear_flags(battle_flags::GAUGE_FULL | battle_flags::CUSTOM_REQUESTED);
        self.gauge.enabled = false;
        self.round.turn += 1;
        self.custom_ui = CustomUi::default();
    }

    fn install_exchange(&mut self, results: [CustomResult; 2]) {
        for (side, r) in results.into_iter().enumerate() {
            if let Some(h) = r.hand {
                self.hands[side] = h;
            }
            self.stats[side] = r.navi_stats;
            self.transform_requests[side] = r.transform;
        }
        self.custom_ui.installed = true;
    }

    /// `sub_8013FD0`: custom-HP bug damage at custom-screen open. Never kills.
    fn custom_hp_bug(&mut self, side: u8) {
        let v = self.stats[side as usize].bugs.custom_damage;
        if v == 0 {
            return;
        }
        if let Some(r) = self.player(side) {
            let hp = self.objects.get(r).hp;
            let d = v.min(hp.saturating_sub(1));
            crate::kinds::subtract_hp(self, r, d);
            self.play_sound(SoundId(0x6B));
        }
    }

    // ---- Fighting ----------------------------------------------------------

    fn mode_fighting(&mut self) {
        if self.round.init == 0 {
            self.fight = FightMachine::default();
            self.round.init = 4;
        }
        self.run_fight_machine();
        let r = self.fight.result;
        match r {
            0 => {}
            6 => self.enter_mode(mode::CUSTOM),
            _ => {
                match r {
                    1 | 2 => {
                        self.round.result = (self.round.result & 0xF0) | r;
                        self.round.busting_level = self.busting_level();
                    }
                    _ => {}
                }
                self.enter_mode(mode::FADE_OUT);
            }
        }
    }

    fn run_fight_machine(&mut self) {
        match self.fight.state {
            fight::SETUP => self.fight_setup(),
            fight::START_BANNER => self.fight_start_banner(),
            fight::FIGHTING => self.fight_fighting(),
            fight::WIN | fight::LOSE => self.fight_result(),
            s => panic!("fighting state {s:#x} not implemented yet"),
        }
    }

    /// Fighting state 0 (`sub_800840C`): run the transformation sequencer
    /// twice, then count down Beast Out.
    fn fight_setup(&mut self) {
        if self.fight.init == 0 {
            self.start_transform_sequencer();
            self.fight.init = 4;
        }
        if self.step_transform_sequencer() {
            return;
        }
        if self.fight.sub == 0 {
            self.transform_seq.restart();
            self.fight.sub = 4;
            return;
        }
        for side in 0..2u8 {
            if self.player(side).is_some() {
                self.count_down_beast_out(side);
            }
        }
        self.fight.state = fight::START_BANNER;
        self.fight.sub = 0;
        self.fight.init = 0;
    }

    /// `sub_8015A38`: a turn in Beast Out uses up one of MegaMan's turns,
    /// unless he started the battle in Beast Out.
    fn count_down_beast_out(&mut self, side: u8) {
        let s = &mut self.stats[side as usize];
        let started_beast = matches!(s.starting_form, Form::GREGAR_BEAST | Form::FALZAR_BEAST);
        if s.navi == Navi::MEGAMAN && !started_beast && s.form.is_beast() && s.beast_out_counter != 0 {
            s.beast_out_counter -= 1;
        }
    }

    fn apply_actor_inputs(&mut self) {
        for side in 0..2u8 {
            let Some(a) = self.player_actor(side) else { continue };
            let over = self.is_battle_over();
            let form = self.stats[side as usize].form;
            let held = self.inputs[side as usize].held;
            let timestop = self.is_time_stop();
            let ad = self.actors.get_mut(a);
            if over {
                ad.pad = Default::default();
                continue;
            }
            if form.is_beast_over() {
                continue;
            }
            ad.pad.update(held);
            if !timestop {
                ad.timestop_pad = Default::default();
            } else {
                ad.timestop_pad.update(held);
            }
        }
    }

    fn fight_start_banner(&mut self) {
        self.apply_actor_inputs();
        if self.fight.init == 0 {
            self.fight.timer = 0x1E;
            self.fight.init = 4;
            if self.late_turns() {
                self.fight.turn_timer = 0xA5 * 4 - 1;
                self.banner.start(BannerId(0x10));
            } else if self.setup.settings.effects & effects::LINK != 0 {
                self.banner.start(BannerId(0x0C));
            }
        }
        if self.banner.status() == BannerStatus::Done {
            self.fight.state = fight::FIGHTING;
            self.fight.sub = 0;
            self.fight.init = 0;
        }
    }

    /// From the 15th custom screen on, netbattles use a turn timer and the
    /// gauge stops arming.
    pub fn late_turns(&self) -> bool {
        self.setup.settings.effects & effects::LINK != 0 && self.round.turn >= 15
    }

    fn fight_fighting(&mut self) {
        self.apply_actor_inputs();
        self.paused = false;
        self.set_flags(battle_flags::FIGHTING);
        self.update_combo();
        self.update_battle_time();
        if self.late_turns() {
            self.update_turn_timer();
        }
        match self.round_result() {
            1 => {
                if self.round.escape != 0 {
                    panic!("escape is not a netbattle outcome");
                }
                self.round.wins += 1;
                self.fight.state = fight::WIN;
                return;
            }
            2 => {
                self.round.losses += 1;
                self.fight.state = fight::LOSE;
                return;
            }
            7 => {
                self.fight.state = fight::JUDGE;
                return;
            }
            _ => {}
        }
        if let Some(p) = self.pause_request() {
            self.fight.pausing_player = p;
            self.paused = true;
            self.fight.state = fight::PAUSE;
            return;
        }
        if self.custom_open_requested() {
            self.paused = true;
            self.fight.state = fight::CUSTOM_REVERT;
        }
    }

    /// `sub_800A152`: the round's result from the local side's perspective.
    fn round_result(&self) -> u8 {
        if self.is_time_stop() {
            return 0;
        }
        let local = self.round.local_side;
        if self.round.actor_count[0] == 0 {
            return if local == 0 { 2 } else { 1 };
        }
        if self.round.actor_count[1] == 0 {
            return if local == 0 { 1 } else { 2 };
        }
        if self.round.time_up != 0 {
            return 7;
        }
        0
    }

    /// `sub_800A046`: a player pressed START.
    fn pause_request(&self) -> Option<u8> {
        if self.is_battle_over() || self.is_time_stop() {
            return None;
        }
        (0..2u8).find(|&p| self.inputs[p as usize].pressed & keys::START != 0)
    }

    /// `sub_800A1D0`.
    fn custom_open_requested(&self) -> bool {
        if self.is_time_stop() || self.is_battle_over() {
            return false;
        }
        let berserk = |s: &NaviStats| s.form.is_beast_over();
        ((berserk(&self.stats[0]) || berserk(&self.stats[1])) && self.round.flags & battle_flags::GAUGE_FULL != 0)
            || self.round.flags & battle_flags::CUSTOM_REQUESTED != 0
    }

    fn update_combo(&mut self) {
        let r = &mut self.round;
        if r.combo > r.max_combo {
            r.max_combo = r.combo;
        }
        if r.combo_window != 0 {
            r.combo_window -= 1;
        } else {
            r.combo = 0;
        }
    }

    fn update_battle_time(&mut self) {
        if self.is_time_stop() || self.paused || self.round.flags & battle_flags::FIGHTING == 0 {
            return;
        }
        if self.is_battle_over_flag_quirk() {
            return;
        }
        if self.round.battle_time < 0x8C9F {
            self.round.battle_time += 1;
        }
    }

    fn update_turn_timer(&mut self) {
        if self.paused || self.is_time_stop() {
            return;
        }
        if self.is_battle_over() {
            return;
        }
        if self.fight.turn_timer >= 0x3C {
            self.fight.turn_timer -= 1;
        } else {
            self.round.time_up = 1;
        }
    }

    fn fight_result(&mut self) {
        if self.fight.init == 0 {
            self.gauge.enabled = false;
            let win = self.fight.state == fight::WIN;
            if win {
                let special = self.setup.settings.effects & 2 != 0;
                self.play_sound(SoundCue::Music(if special { SoundId::WINNER_SPECIAL } else { SoundId::WINNER }));
            } else if self.setup.settings.effects & effects::LINK != 0 {
                self.play_sound(SoundCue::Music(SoundId::LOSER));
            }
            self.round.winner = if win { self.round.local_side } else { self.round.local_side ^ 1 };
            self.fight.init = 4;
            self.fight.timer = 0x66;
            // Netbattle win/lose banners come from per-navi tables.
            let navi = self.stats[self.round.local_side as usize].navi.index();
            let id = if win { crate::data::WIN_BANNERS[navi] } else { crate::data::LOSE_BANNERS[navi] };
            self.banner.start(id);
        }
        self.fight.timer -= 1;
        if self.banner.status() == BannerStatus::Done && self.fight.timer <= 0 {
            self.fight.result = if self.fight.state == fight::WIN { 1 } else { 2 };
        }
    }

    fn busting_level(&self) -> u8 {
        crate::kinds::busting_level(self)
    }

    // ---- End of round ------------------------------------------------------

    fn mode_fade_out(&mut self) {
        if self.round.init == 0 {
            self.fade.start();
            self.fade.remaining = 16;
            self.round.init = 4;
            return;
        }
        if !self.fade.active() {
            self.play_sound(SoundCue::StopMusic);
            self.objects.free_all();
            self.round.top = top::END;
            self.round.mode = 0;
            self.round.sub = 0;
            self.round.init = 0;
        }
    }

    // ---- Per-tick helpers ----------------------------------------------------

    /// `sub_800FDC0`: expose each player's hand on their navi.
    fn update_player_hands(&mut self) {
        for side in 0..2 {
            for slot in 0..4 {
                let Some(r) = self.round.alive_actors[side][slot] else { continue };
                let Some(a) = self.objects.get(r).actor else { continue };
                if self.actors.get(a).actor_type != crate::actor::ActorType::Player {
                    continue;
                }
                let alliance = self.objects.get(r).alliance as usize;
                let hand = &self.hands[alliance];
                let (held, next) = (hand.remaining(), hand.next_chip());
                let o = self.objects.get_mut(r);
                o.chips_held = held;
                o.chip = next.unwrap_or(0xFFFF);
            }
        }
    }

    fn run_hud_tasks(&mut self) {
        if self.gauge.enabled {
            self.fill_gauge();
        }
        self.banner.tick();
    }

    /// `sub_801C470`.
    fn fill_gauge(&mut self) {
        if self.paused || self.is_time_stop() || self.round.flags & battle_flags::GAUGE_FULL != 0 {
            return;
        }
        let v = self.gauge.value.wrapping_add(self.gauge.rate);
        self.gauge.value = v;
        if v >= CustomGauge::FULL {
            self.gauge.value = CustomGauge::FULL;
            if !self.late_turns() {
                self.set_flags(battle_flags::GAUGE_FULL);
                self.play_sound(SoundId(0x8F));
            }
        }
    }

    fn update_linked_registry(&mut self) {
        crate::kinds::update_linked_registry(self);
    }

    /// `chip_800AEE8`: chips flagged for it recompute their damage every tick.
    fn refresh_variable_damage(&mut self) {
        for side in 0..2u8 {
            crate::hand::refresh_variable_damage(self, side);
        }
    }

    fn shift_damage_carry(&mut self) {
        crate::kinds::shift_damage_carry(self);
    }

    /// `sub_80102AC`: the NaviCust HP-drain bug, active only while that
    /// player's custom screen is open.
    fn custom_hp_drain(&mut self, side: u8) {
        if self.round.remote_status[side as usize] & 5 == 0 {
            return;
        }
        const PERIOD: [u8; 8] = [0, 40, 30, 20, 10, 5, 3, 2];
        let period = PERIOD[(self.stats[side as usize].bugs.custom_drain & 7) as usize];
        if period == 0 {
            return;
        }
        let Some(r) = self.player(side) else { return };
        if self.objects.get(r).hp <= 1 {
            return;
        }
        let Some(a) = self.objects.get(r).actor else { return };
        let ad = self.actors.get_mut(a);
        ad.drain_counter += 1;
        if ad.drain_counter >= period {
            ad.drain_counter = 0;
            crate::kinds::subtract_hp(self, r, 1);
        }
    }

    /// `sub_8009158`: the low-HP music switch (sound only, but it keeps a
    /// latch in the round state).
    fn low_hp_music(&mut self) {
        if self.setup.settings.effects & effects::LINK == 0 {
            return;
        }
        let Some(r) = self.player(self.round.local_side) else { return };
        let o = self.objects.get(r);
        let low = o.hp <= o.max_hp / 4;
        if low && !self.round.low_hp_music {
            self.round.low_hp_music = true;
            self.play_sound(SoundCue::Pinch(true));
        } else if !low && self.round.low_hp_music {
            self.round.low_hp_music = false;
            self.play_sound(SoundCue::Pinch(false));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::BATTLE_SETTINGS;
    use crate::setup::Stage;

    /// A best-of-three netbattle round about to leave its end state, with
    /// the set standing at `wins`-`losses` after `round` rounds.
    fn ending(wins: u8, losses: u8, round: u8) -> Battle {
        let stats = NaviStats { hp: 1000, max_hp: 1000, max_base_hp: 1000, ..NaviStats::default() };
        let mut b = Battle::new(RoundSetup {
            settings: BattleSettings { effects: 0xE8C, ..BATTLE_SETTINGS[0x0B] },
            navi_stats: [stats; 2],
            rng: 1,
            local_side: 0,
            score: SetScore::default(),
            later_stages: [Stage { settings: 0x11, background: 3 }, Stage { settings: 0x46, background: 0x13 }],
            low_hp_music_latched: false,
        });
        let r = &mut b.round;
        (r.top, r.mode, r.sub, r.init) = (top::END, 4, 0, 0);
        (r.wins, r.losses, r.round, r.max_combo) = (wins, losses, round, 1);
        r.result = if wins > losses { 1 } else { 2 };
        b.paused = true;
        b
    }

    fn tick(b: &mut Battle) {
        b.tick(&[PlayerTick::default(), PlayerTick::default()], TickEvents::default());
    }

    #[test]
    fn an_undecided_set_chains_its_next_round_on_the_drawn_stage() {
        let mut b = ending(1, 0, 1);
        tick(&mut b);
        let Some(RoundEnd::NextRound { settings, score }) = b.round_end() else { panic!("{:?}", b.round_end()) };
        // The drawn table entry, with this round's effects and the drawn
        // background.
        assert_eq!(*settings, BattleSettings { effects: 0xE8C, background: 3, ..BATTLE_SETTINGS[0x11] });
        assert_eq!(*score, SetScore { wins: 1, losses: 0, round: 1, max_combo: 1 });
        assert_eq!(b.round.top, top::INIT);
        assert_eq!(b.sound_cues(), [SoundCue::StopMusic]);
    }

    #[test]
    fn a_decided_set_ends_the_battle() {
        let mut b = ending(2, 0, 2);
        tick(&mut b);
        assert_eq!(b.round_end(), Some(&RoundEnd::Over(BattleResult::Won)));
        assert_eq!((b.round.top, b.round.mode, b.round.running, b.paused), (top::END, 4, 0, false));
        tick(&mut b);
        assert_eq!(b.sound_cues(), [], "the battle ends once");

        let mut b = ending(1, 2, 3);
        tick(&mut b);
        assert_eq!(b.round_end(), Some(&RoundEnd::Over(BattleResult::Lost)));
        let mut b = ending(1, 1, 3);
        b.round.result = 1;
        tick(&mut b);
        assert_eq!((b.round_end(), b.round.result), (Some(&RoundEnd::Over(BattleResult::Drawn)), 3));
    }

    #[test]
    fn a_latched_low_hp_switch_plays_no_pinch_cue_on_the_first_tick() {
        let stats = NaviStats { hp: 500, max_hp: 500, max_base_hp: 500, ..NaviStats::default() };
        let mut b = Battle::new(RoundSetup {
            settings: BattleSettings { effects: effects::LINK, ..BATTLE_SETTINGS[0] },
            navi_stats: [stats; 2],
            rng: 1,
            local_side: 0,
            score: SetScore::default(),
            later_stages: Default::default(),
            low_hp_music_latched: true,
        });
        tick(&mut b);
        assert_eq!(b.sound_cues(), [SoundCue::Music(SoundId::VIRUS_BATTLE)]);
        tick(&mut b);
        assert_eq!(b.sound_cues(), [SoundCue::Pinch(false)]);
    }
}
