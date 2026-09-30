//! Synthetic matches: a battle whose custom screen is a stand-in driven
//! by the players' buttons, so that the buttons are the whole input, and
//! a seeded button masher.
//!
//! The stand-in exists until the engine simulates the custom screen. It
//! derives the custom screen's part of the engine input (`in_custom`, the
//! local confirmation, the exchange) and the link closing at the end of
//! the round from the battle's state and the buttons, inside the
//! simulated game, so it rolls back with it.

use crate::Game;
use crate::bn6::HasBattle;
use crate::rng::SplitMix64;
use bn6_battle::battle::{mode, top};
use bn6_battle::data::{self, ChipId};
use bn6_battle::hand::ChipHand;
use bn6_battle::input::keys;
use bn6_battle::setup::{BattleSettings, Form, GaugeSpeed, Navi, NaviCustBugs, NaviStats, NaviWeapons, RoundSetup, SetScore, SupportNavis};
use bn6_battle::transform::TransformRequest;
use bn6_battle::{Battle, CustomResult, PlayerTick, TickEvents, TickInput};

/// How the stand-in custom screen behaves.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rules {
    /// The hand each side gets at every custom screen.
    pub hands: [ChipHand; 2],
    /// Ticks the screen is open before A confirms.
    pub min_ticks: u32,
    /// A player who hasn't confirmed by then confirms anyway.
    pub max_ticks: u32,
    /// L confirms with a Beast Out request (a MegaMan not in beast form).
    pub beast_out: bool,
}

/// One custom screen's progress.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
struct Screen {
    /// Ticks since it opened.
    ticks: u32,
    /// Per side: the tick the player confirmed, and whether with Beast Out.
    confirmed: [Option<(u32, bool)>; 2],
    /// The results went out.
    sent: bool,
}

/// Ticks between the local player's confirmation and the results
/// arriving: the engine turns the custom gauge back on 11 ticks after the
/// local confirmation, which must happen before the fight resumes.
pub const EXCHANGE_DELAY: u32 = 12;

/// A battle with the stand-in custom screen; its input is each player's
/// held buttons.
#[derive(Clone, Debug)]
pub struct StandInBattle {
    pub battle: Battle,
    pub rules: Rules,
    screen: Screen,
    previous: [u16; 2],
}

impl StandInBattle {
    pub fn new(battle: Battle, rules: Rules) -> StandInBattle {
        StandInBattle { battle, rules, screen: Screen::default(), previous: [0; 2] }
    }

    /// The engine input for this tick, from the buttons and the battle as
    /// it stands before the tick.
    pub fn tick_input(&mut self, buttons: [u16; 2]) -> TickInput {
        let b = &self.battle;
        let pressed = [buttons[0] & !self.previous[0], buttons[1] & !self.previous[1]];
        self.previous = buttons;
        let mut events = TickEvents::default();
        let open = b.round.top == top::RUNNING && b.round.mode == mode::CUSTOM;
        if !open {
            self.screen = Screen::default();
        } else {
            let local = b.round.local_side as usize;
            let s = &mut self.screen;
            s.ticks += 1;
            for side in 0..2 {
                if s.confirmed[side].is_some() || s.ticks < self.rules.min_ticks {
                    continue;
                }
                let stats = &b.stats[side];
                let beast = self.rules.beast_out
                    && pressed[side] & keys::L != 0
                    && stats.navi == Navi::MEGAMAN
                    && !stats.form.is_beast()
                    && stats.beast_out_counter != 0;
                if pressed[side] & keys::A != 0 || beast || s.ticks >= self.rules.max_ticks {
                    s.confirmed[side] = Some((s.ticks, beast));
                    events.local_confirm |= side == local;
                }
            }
            if let [Some((local_at, _)), Some(_)] = [s.confirmed[local], s.confirmed[local ^ 1]]
                && !s.sent
                && s.ticks >= local_at + EXCHANGE_DELAY
            {
                let result = |side: usize| {
                    let beast = s.confirmed[side].is_some_and(|c| c.1);
                    let form = beast.then_some(Form::FALZAR_BEAST);
                    CustomResult {
                        hand: Some(self.rules.hands[side].clone()),
                        navi_stats: b.stats[side],
                        transform: TransformRequest { form, ..TransformRequest::NONE },
                    }
                };
                events.exchange = Some(Box::new([result(0), result(1)]));
                s.sent = true;
            }
        }
        // The end state asks the link session to close; it closes at once.
        let r = &b.round;
        events.link_closed = r.top == top::END && r.mode == 0 && r.sub == 4 && r.init == 4;
        let player = |side: usize| PlayerTick {
            // The screen takes the buttons while it's open.
            held: if open { 0 } else { buttons[side] & 0x3FF },
            in_custom: open && self.screen.confirmed[side].is_none(),
        };
        TickInput { players: [player(0), player(1)], events }
    }
}

impl Game for StandInBattle {
    type Input = u16;

    fn advance(&mut self, inputs: &[u16; 2]) {
        let input = self.tick_input(*inputs);
        self.battle.step(&input);
    }

    fn digest(&self) -> u64 {
        let StandInBattle { battle, rules, screen, previous } = self;
        bn6_battle::digest::stable_hash(&(battle, rules, screen, previous))
    }

    fn is_over(&self) -> bool {
        self.battle.round_end().is_some()
    }

    fn blank_input() -> u16 {
        0
    }
}

impl HasBattle for StandInBattle {
    fn battle(&self) -> &Battle {
        &self.battle
    }
}

/// A hand of chips with their codes, as a custom screen hands it over:
/// in use order, with the chip data's damage.
pub fn hand(chips: &[(ChipId, u8)]) -> ChipHand {
    let mut h = ChipHand::empty();
    for (i, &(id, code)) in chips.iter().enumerate().take(5) {
        let d = data::chip(id).damage;
        assert!(d < 1000, "chip {id:#x} has a damage formula; give it a plain damage");
        h.ids[i] = id;
        h.damage[i] = d;
        h.selection[i] = (code as u16) << 9 | id;
    }
    h
}

/// A MegaMan (base form, Falzar Beast Out available) with `hp` HP.
pub fn megaman(hp: u16) -> NaviStats {
    NaviStats {
        attack: 0,
        rapid: 0,
        charge: 0,
        first_barrier: 0,
        gauge_speed: GaugeSpeed::Normal,
        reg_up: 50,
        custom_level: 5,
        mega_level: 5,
        giga_level: 1,
        support: Some(SupportNavis::default()),
        mood: 0x80,
        element: 0,
        starting_form: Form::NONE,
        float_shoes: true,
        air_shoes: true,
        undershirt: false,
        super_armor: false,
        beast_out_counter: 3,
        sun: false,
        navi: Navi::MEGAMAN,
        navi_variant: 10,
        form: Form::NONE,
        folder_reg: [0xFF; 2],
        max_base_hp: hp,
        hp,
        max_hp: hp,
        chip_recovery: 0,
        folder_tags: [[0xFF; 2]; 2],
        weapons: NaviWeapons {
            buster: 0,
            charge_shot: 1,
            back_special: 0xFF,
            a_charge: 0xFF,
            mode9_a: 0,
            buster_shot: 0,
            charge_shot_kind: 0,
        },
        bugs: NaviCustBugs { panel_trail_kind: 0xFF, ..NaviCustBugs::default() },
    }
}

/// A one-round netbattle between two MegaMen on the plain field
/// (battle settings 0), simulated from side 0's perspective.
pub fn netbattle(hp: u16, seed: u32) -> RoundSetup {
    RoundSetup {
        settings: BattleSettings { ..data::BATTLE_SETTINGS[0] },
        navi_stats: [megaman(hp), megaman(hp)],
        rng: seed,
        local_side: 0,
        score: SetScore::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        sp_times: Default::default(),
    }
}

/// Random button mashing, seeded: held buttons change every few frames
/// (directions, A for chips, B to shoot and charge, L/R for the custom
/// screen). START (pause) is never pressed.
#[derive(Clone, Debug)]
pub struct Masher {
    rng: SplitMix64,
    held: u16,
    /// Also press B (the buster; not implemented in the engine yet).
    pub buster: bool,
}

impl Masher {
    pub fn new(seed: u64) -> Masher {
        Masher { rng: SplitMix64::new(seed), held: 0, buster: false }
    }

    /// The next frame's held buttons.
    pub fn buttons(&mut self) -> u16 {
        let r = &mut self.rng;
        if r.chance(1, 4) {
            const DIRS: [u16; 5] = [0, keys::UP, keys::DOWN, keys::LEFT, keys::RIGHT];
            let mut held = DIRS[r.below(5) as usize];
            if r.chance(1, 3) {
                held |= keys::A;
            }
            if self.buster && r.chance(2, 5) {
                held |= keys::B;
            }
            if r.chance(1, 12) {
                held |= if r.chance(1, 2) { keys::L } else { keys::R };
            }
            self.held = held;
        }
        self.held
    }
}
