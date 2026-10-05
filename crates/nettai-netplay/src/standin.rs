//! A battle whose buttons are the whole input (the engine simulates both
//! custom screens), with the one event left from outside the simulation,
//! the link session closing at the end of the round, derived inside the
//! game: what live netplay plays (a frontend's), and synthetic matches;
//! and, for those, a seeded button masher and a netbattle setup.

use crate::rng::SplitMix64;
use crate::world::Game;
use nettai_battle::battle::top;
use nettai_battle::console::ConsoleSetup;
use nettai_battle::custom::{BattleFolder, FolderChip, PlayerSetup};
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::input::keys;
use nettai_battle::setup::{BattleSettings, NaviStats, RoundSetup, SetScore};
use nettai_battle::{Battle, PlayerTick, TickEvents, TickInput};

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

    fn step(&mut self, inputs: [&u16; 2]) {
        let input = self.tick_input([*inputs[0], *inputs[1]]);
        self.battle.step(&input);
    }

    fn battle(&self) -> &Battle {
        &self.battle
    }

    fn battle_mut(&mut self) -> &mut Battle {
        &mut self.battle
    }
}

/// A battle folder of these chips of `content` (key, code) over and over,
/// in this order (not shuffled).
pub fn folder(content: &Content, chips: &[(&str, u8)]) -> BattleFolder {
    let mut f = BattleFolder::empty();
    for (slot, &(key, code)) in f.chips.iter_mut().zip(chips.iter().cycle()) {
        let chip = content.defs.chip_by_key(key).unwrap_or_else(|| panic!("the content defines no chip {key:?}"));
        *slot = Some(FolderChip::new(chip, ChipCode(code)));
    }
    f
}

/// A MegaMan (base form) with `hp` HP, on `content`: his fresh stats
/// (`NaviStats::fresh`: what his game's rules and his own definition
/// state, as a new save's block has them), with what the stand-in changes:
///
/// - **his HP**, the match's;
/// - **his variant**, 10: a battle's start sets it to his base HP in
///   hundreds (`sub_800A2F8`), and it picks his move lag. The stand-in's
///   is a MegaMan of 1000 base HP's whatever HP the match gives him, so
///   matches of any HP move alike;
/// - **FloatShoes and AirShoes**, so what a masher's steps do doesn't go by
///   the stage's panels: none slides, cracks under or drops him.
///
/// (His stats' version byte stays the fresh block's, none: a battle's
/// start gives a block its side's, and no stand-in match reads it.)
pub fn megaman(content: &Content, hp: u16) -> NaviStats {
    // MegaMan: the content's navi that changes form.
    let megaman = content.form_changing_navi().unwrap_or_else(|| panic!("the content has no navi that changes form"));
    let fresh = NaviStats::fresh(megaman, content)
        .unwrap_or_else(|| panic!("{} has no fresh stats (its definition's `fresh`)", content.defs.navi(megaman).key));
    NaviStats { max_base_hp: hp, hp, max_hp: hp, navi_variant: 10, float_shoes: true, air_shoes: true, ..fresh }
}

/// A one-round netbattle between two MegaMen on `content`'s stage `stage`
/// (its key), simulated from side 0's perspective, with these battle
/// folders. The players are of [`VERSION`], stated where the content's
/// rules take a version (a round assumes none), with the Crosses and Beast
/// Out locked (the rest of EXE6's systems' setups zero: nothing unlocked);
/// the players' buttons
/// reach the fight at once (no link delay). Each player's console RNG is
/// derived from the seed.
pub fn netbattle(content: &Content, stage: &str, hp: u16, seed: u32, folders: [BattleFolder; 2]) -> RoundSetup {
    let player = |f: BattleFolder, side: u32| {
        let mut p = player_setup(f, seed, side);
        p.set_fact(content, "version", &[nettai_battle::rules::Fact::Name(VERSION)]).expect("the stand-in's players' version");
        p
    };
    let [a, b] = folders;
    RoundSetup {
        content: content.hash(),
        settings: BattleSettings::on(content, content.stage_by_key(stage)),
        navi_stats: [megaman(content, hp), megaman(content, hp)],
        rng: seed,
        local_side: 0,
        score: SetScore::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        players: [player(a, 0), player(b, 1)],
        link_delay: 0,
    }
}

/// The version the stand-in's players are of (the test content plays by
/// EXE6's beast system, and EXE6's content by its own: each takes one).
pub const VERSION: &str = "falzar";

/// A stand-in player's setup but for what the content's rules take.
fn player_setup(f: BattleFolder, seed: u32, side: u32) -> PlayerSetup {
    PlayerSetup {
        folder: f,
        joypad_phase: 0,
        navi_level: None,
        sp_times: Default::default(),
        console: ConsoleSetup { rng: seed.rotate_left(16) ^ side.wrapping_mul(0x9E37_79B9), ..ConsoleSetup::default() },
        rules: Vec::new(),
        patch_cards: Default::default(),
        navicust: None,
        tactics: Default::default(),
    }
}

/// Random button mashing, seeded: held buttons change every few frames
/// (directions, A for chips and the custom screen, B to shoot and charge,
/// L/R). START (pause) is never pressed.
#[derive(Clone, Debug)]
pub struct Masher {
    rng: SplitMix64,
    held: u16,
    /// Also press B: the buster, and held, the charged shot.
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
