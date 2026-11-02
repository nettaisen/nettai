//! A battle whose tick words (`PlayerTick::word`: the buttons, a party
//! switch request and a custom screen's choice) are the whole input (the
//! engine simulates both custom screens on the choices), with the one event
//! left from outside the simulation, the link session closing at the end of
//! the round, derived inside the game: what live netplay plays (a
//! frontend's), and synthetic matches; and, for those, a seeded masher and a
//! netbattle setup.

use crate::rng::SplitMix64;
use crate::world::Game;
use nettai_battle::battle::{mode, top};
use nettai_battle::console::ConsoleSetup;
use nettai_battle::custom::screen::MAX_SELECTIONS;
use nettai_battle::custom::{BattleFolder, FolderChip, Phase, PlayerSetup, SlotKind, SlotState};
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::input::keys;
use nettai_battle::setup::{BattleSettings, NaviStats, RoundSetup, SetScore};
use nettai_battle::{Battle, PlayerTick, ScreenChoice, TickEvents, TickInput};

/// A battle stepped on the players' buttons alone.
#[derive(Clone, Debug)]
pub struct StandInBattle {
    pub battle: Battle,
}

impl StandInBattle {
    pub fn new(battle: Battle) -> StandInBattle {
        StandInBattle { battle }
    }

    /// The engine input for this tick, from the tick words and the battle
    /// as it stands before the tick.
    pub fn tick_input(&mut self, words: [u32; 2]) -> TickInput {
        tick_input(&self.battle, words)
    }
}

/// The engine input for `battle`'s next tick from the two players' tick
/// words (by side: `PlayerTick::word`, their buttons, switch request and
/// custom screen's choice)
/// and the battle as it stands before the tick: the one place that derives
/// it, for whoever steps a battle on tick words alone (a netplay peer,
/// offline play).
pub fn tick_input(battle: &Battle, words: [u32; 2]) -> TickInput {
    let r = &battle.round;
    // The end state asks the link session to close; it closes at once.
    let events = TickEvents { link_closed: r.top == top::END && r.mode == 0 && r.sub == 4 && r.init == 4, ..TickEvents::default() };
    let player = |side: usize| PlayerTick::from_word(words[side]);
    TickInput { players: [player(0), player(1)], events }
}

impl Game for StandInBattle {
    type Input = u32;

    fn step(&mut self, inputs: [&u32; 2]) {
        let input = self.tick_input([*inputs[0], *inputs[1]]);
        self.battle.step(&input);
    }

    fn battle(&self) -> &Battle {
        &self.battle
    }

    fn battle_mut(&mut self) -> &mut Battle {
        &mut self.battle
    }

    /// The buttons (and the microphone's level) carry on; a switch request
    /// and a custom screen's choice happen once (each is a tick's, as the
    /// link closing is: carried on, a choice would be taken again, or
    /// refused with its sound, on every tick predicted).
    fn predict(last: &u32) -> u32 {
        PlayerTick::from_word(*last).carried_on().word()
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
/// rules take a version (a round assumes none), with no Crosses (a list a
/// round assumes none of either: an empty one) and Beast Out locked, stated
/// where the rules take them (a setup that says nothing of Beast Out has
/// the rules' own default, a finished save's: unlocked); the players' buttons
/// reach the fight at once (no link delay). Each player's console RNG is
/// derived from the seed.
pub fn netbattle(content: &Content, stage: &str, hp: u16, seed: u32, folders: [BattleFolder; 2]) -> RoundSetup {
    use nettai_battle::rules::Fact;
    let player = |f: BattleFolder, side: u32| {
        let mut p = player_setup(f, seed, side);
        p.set_fact(content, "version", &[Fact::Name(VERSION)]).expect("the stand-in's players' version");
        // (No Crosses: the empty list.)
        p.set_fact(content, "crosses", &[]).expect("the stand-in's players' Crosses");
        p.set_fact(content, "beast_out", &[Fact::Value(nettai_content_api::Value::Bool(false))]).expect("the stand-in's players' Beast Out");
        // (No NaviCust: the stats are the stand-in's as they are, where
        // the rules take one.)
        p.set_fact(content, "navicust_expansions", &[Fact::Value(nettai_content_api::Value::Nil)]).expect("the stand-in's players' NaviCust");
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
        host_keys: nettai_battle::setup::HostKeys::NONE,
        custom_screen_limit: None,
        players: [player(a, 0), player(b, 1)],
    }
}

/// The version the stand-in's players are of (the test content plays by
/// EXE6's rules/beast, and EXE6's content by its own: each takes one).
pub const VERSION: &str = "falzar";

/// A stand-in player's setup but for what the content's rules take.
fn player_setup(f: BattleFolder, seed: u32, side: u32) -> PlayerSetup {
    PlayerSetup {
        folder: f,
        joypad_phase: 0,
        console: ConsoleSetup { rng: seed.rotate_left(16) ^ side.wrapping_mul(0x9E37_79B9), ..ConsoleSetup::default() },
        rules: None,
    }
}

/// Random button mashing, seeded: held buttons change every few frames
/// (directions, A for chips, B to shoot and charge, L/R). START (pause) is
/// never pressed. On the custom screen, which takes choices and no keys
/// ([`Masher::choice`]): now and then a chip that can be picked, and OK
/// once it has chosen a while.
#[derive(Clone, Debug)]
pub struct Masher {
    rng: SplitMix64,
    held: u16,
    /// Also press B: the buster, and held, the charged shot (and on the
    /// custom screen, now and then, B: a pick taken back).
    pub buster: bool,
    /// Ticks the side's custom screen has chosen on so far.
    choosing: u32,
}

/// Ticks a masher's custom screen chooses at least before it presses OK.
const CHOOSING_AT_LEAST: u32 = 30;

impl Masher {
    pub fn new(seed: u64) -> Masher {
        Masher { rng: SplitMix64::new(seed), held: 0, buster: false, choosing: 0 }
    }

    /// The next frame's input for side `side` on `b` (the battle as it
    /// stands before the frame): the held buttons ([`Masher::buttons`]) and
    /// the custom screen's choice ([`Masher::choice`]).
    pub fn input(&mut self, b: &Battle, side: u8) -> PlayerTick {
        let held = self.buttons();
        PlayerTick::buttons(held).with_choice(self.choice(b, side))
    }

    /// The next frame's choice on side `side`'s custom screen, while it
    /// chooses on `b`: one tick in six, a chip that can be picked (one of
    /// those there, at random) while there is room; from the screen's 30th
    /// choosing tick, one in eight, OK; with the buster, one in twenty, a
    /// pick taken back. None while the screen doesn't choose.
    pub fn choice(&mut self, b: &Battle, side: u8) -> Option<ScreenChoice> {
        let screen = b.custom.sides[side as usize & 1].screen.as_ref().filter(|s| b.round.mode == mode::CUSTOM && s.phase == Phase::Choosing);
        let Some(screen) = screen else {
            self.choosing = 0;
            return None;
        };
        self.choosing += 1;
        let r = &mut self.rng;
        if self.choosing >= CHOOSING_AT_LEAST && r.chance(1, 8) {
            return Some(ScreenChoice::Ok);
        }
        if self.buster && r.chance(1, 20) {
            return Some(ScreenChoice::TakeBack);
        }
        if r.chance(1, 6) && (screen.selected as usize) < MAX_SELECTIONS {
            let pickable: Vec<u8> = (0..screen.slots.len() as u8)
                .filter(|&s| {
                    let slot = &screen.slots[s as usize];
                    matches!(slot.kind, SlotKind::Chip { .. } | SlotKind::Offered(_)) && slot.state == SlotState::Selectable
                })
                .collect();
            if !pickable.is_empty() {
                return Some(ScreenChoice::Pick(pickable[r.below(pickable.len() as u64) as usize]));
            }
        }
        None
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

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_battle::content::testing;

    /// A tick word predicted is its buttons and level carried on, without
    /// its switch request or its choice.
    #[test]
    fn a_choice_and_a_switch_are_not_predicted() {
        let t = PlayerTick { held: keys::A | keys::UP, switch_to: Some(1), mic_level: 9, choice: Some(ScreenChoice::Pick(3)) };
        let predicted = PlayerTick::from_word(<StandInBattle as Game>::predict(&t.word()));
        assert_eq!(predicted, PlayerTick { switch_to: None, choice: None, ..t });
    }

    /// Two mashers get a stand-in netbattle through its first custom
    /// screens: picks, then OK, and the fight goes on.
    #[test]
    fn mashers_choose_through_the_custom_screen() {
        let c = testing::content();
        let f = || folder(&c, &[(testing::SUN_GUN_3, 0), (testing::SUN_GUN_1, 0)]);
        let mut b = Battle::new(netbattle(&c, testing::LINK_BATTLE, 300, 0x1357, [f(), f()]), c.clone());
        let mut mashers = [Masher::new(1), Masher::new(2)];
        let (mut picks, mut oks, mut fought) = (0, 0, 0);
        for _ in 0..3000 {
            let words = [0u8, 1].map(|side| mashers[side as usize].input(&b, side).word());
            for w in words {
                match PlayerTick::from_word(w).choice {
                    Some(ScreenChoice::Pick(_)) => picks += 1,
                    Some(ScreenChoice::Ok) => oks += 1,
                    _ => {}
                }
            }
            let input = tick_input(&b, words);
            b.step(&input);
            fought += (b.round.mode == mode::FIGHTING) as u32;
        }
        assert!(picks > 0 && oks >= 2 && fought > 1000, "{picks} picks, {oks} OKs, {fought} ticks fought");
    }
}
