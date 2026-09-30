//! Synthetic matches: a battle whose buttons are the whole input (the
//! engine simulates both custom screens), with the one event left from
//! outside the simulation, the link session closing at the end of the
//! round, derived inside the game; and a seeded button masher.

use crate::Game;
use crate::bn6::HasBattle;
use crate::rng::SplitMix64;
use bn6_battle::battle::top;
use bn6_battle::custom::{BattleFolder, FolderChip, GameVersion, PlayerSetup, Unlocks};
use bn6_battle::content::{ChipCode, ChipId, Content};
use bn6_battle::input::keys;
use bn6_battle::setup::{Form, GaugeSpeed, Navi, NaviCustBugs, NaviStats, NaviWeapons, RoundSetup, SetScore, SupportNavis};
use bn6_battle::{Battle, PlayerTick, TickEvents, TickInput};

/// A battle stepped on the players' buttons alone.
#[derive(Clone, Debug)]
pub struct StandInBattle {
    pub battle: Battle,
}

impl StandInBattle {
    pub fn new(battle: Battle) -> StandInBattle {
        StandInBattle { battle }
    }

    /// The engine input for this tick, from the buttons and the battle as
    /// it stands before the tick.
    pub fn tick_input(&mut self, buttons: [u16; 2]) -> TickInput {
        let r = &self.battle.round;
        // The end state asks the link session to close; it closes at once.
        let events = TickEvents { link_closed: r.top == top::END && r.mode == 0 && r.sub == 4 && r.init == 4, ..TickEvents::default() };
        let player = |side: usize| PlayerTick { held: buttons[side] & 0x3FF };
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
        self.battle.digest()
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

/// A battle folder of these chips (id, code) over and over, in this
/// order (not shuffled).
pub fn folder(chips: &[(ChipId, u8)]) -> BattleFolder {
    let mut f = BattleFolder::empty();
    for (slot, &(id, code)) in f.chips.iter_mut().zip(chips.iter().cycle()) {
        *slot = Some(FolderChip::new(id, ChipCode(code)));
    }
    f
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
        folder: 0,
        folder_reg: [0xFF; 2],
        max_base_hp: hp,
        hp,
        max_hp: hp,
        chip_recovery: 0,
        folder_tags: [[0xFF; 2]; 2],
        chip_shuffle: false,
        number_open: false,
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

/// A one-round netbattle between two MegaMen on `content`'s battle
/// settings 0, simulated from side 0's perspective, with these battle
/// folders. The Crosses and Beast Out are locked; the players' buttons
/// reach the fight at once (no link delay).
pub fn netbattle(content: &Content, hp: u16, seed: u32, folders: [BattleFolder; 2]) -> RoundSetup {
    let player = |f: BattleFolder| PlayerSetup {
        folder: Some(f),
        unlocks: Unlocks { crosses: [false; 5], beast_out: false, ..Unlocks::everything(GameVersion::Falzar) },
        joypad_phase: 0,
        bug_frags: 0,
        navi_level: 0,
    };
    let [a, b] = folders;
    RoundSetup {
        content: content.hash(),
        settings: content.rules.stages.settings(0),
        navi_stats: [megaman(hp), megaman(hp)],
        rng: seed,
        local_side: 0,
        score: SetScore::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        sp_times: Default::default(),
        players: [player(a), player(b)],
        link_delay: 0,
    }
}

/// Random button mashing, seeded: held buttons change every few frames
/// (directions, A for chips and the custom screen, B to shoot and charge,
/// L/R). START (pause) is never pressed.
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
