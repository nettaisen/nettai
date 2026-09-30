//! A synthetic netbattle for tests and benchmarks that need whole rounds
//! without golden traces, on the hand-authored test content
//! (`content::testing`): two navis in the sun whose folders hold only a
//! level-3 GunDelSol chip (code A), picking four at each custom screen,
//! stepping about and firing. It is recorded as an input tape (both
//! players' inputs and the link events for every tick), the way a replay
//! is, so rollback runs and benchmarks can play it back exactly.

use crate::battle::{Battle, TickEvents, mode};
use crate::behavior::Behaviors;
use crate::content::{ChipCode, ChipId, Content, testing};
use crate::custom::screen::{OK_SLOT, Phase, SlotKind, SlotState};
use crate::custom::{BattleFolder, FolderChip, GameVersion, PlayerSetup, Unlocks};
use crate::input::{PlayerTick, keys};
use crate::setup::{NaviStats, NaviWeapons, RoundSetup, SetScore};
use std::sync::Arc;

/// One tick of a tape.
#[derive(Clone, Debug)]
pub struct Tick {
    pub input: [PlayerTick; 2],
    pub events: TickEvents,
}

/// A plain MegaMan with 1000 HP, fighting in the sun.
fn megaman() -> NaviStats {
    NaviStats {
        hp: 1000,
        max_hp: 1000,
        max_base_hp: 1000,
        mood: 0x80,
        custom_level: 5,
        mega_level: 5,
        giga_level: 1,
        sun: true,
        weapons: NaviWeapons {
            buster: 0,
            charge_shot: 1,
            back_special: 0xFF,
            a_charge: 0xFF,
            mode9_a: 0xFF,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// The test content the duel is fought on.
pub fn content() -> Arc<Content> {
    testing::content()
}

/// The round: a link battle with the usual two-navi list (side 1 first),
/// on a field of plain panels (no roads or ice to slide on).
pub fn setup() -> RoundSetup {
    let content = testing::build();
    let mut folder = BattleFolder::empty();
    folder.chips = [Some(FolderChip::new(testing::SUN_GUN_3, ChipCode(0))); 30];
    let player = PlayerSetup {
        folder: Some(folder),
        unlocks: Unlocks { crosses: [false; 5], beast_out: false, ..Unlocks::everything(GameVersion::Falzar) },
        joypad_phase: 0,
    };
    RoundSetup {
        content: content.hash(),
        settings: content.rules.stages.settings(testing::LINK_BATTLE),
        navi_stats: [megaman(); 2],
        rng: 0x1234_5678,
        local_side: 0,
        score: SetScore::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        sp_times: Default::default(),
        players: [player; 2],
        link_delay: 0,
    }
}

/// The same round with both folders holding `chips` (in turn, all code
/// A) instead.
pub fn setup_with(chips: &[ChipId]) -> RoundSetup {
    let mut s = setup();
    let mut folder = BattleFolder::empty();
    for (slot, &id) in folder.chips.iter_mut().zip(chips.iter().cycle()) {
        *slot = Some(FolderChip::new(id, ChipCode(0)));
    }
    for p in &mut s.players {
        p.folder = Some(folder);
    }
    s
}

/// A player's buttons on the custom screen: A on the chip under the
/// cursor, RIGHT to the next one, until four are picked; then START and A
/// on OK. A and START are released between presses.
fn custom_buttons(b: &Battle, side: usize, last: u16) -> u16 {
    let Some(screen) = b.custom.sides[side].screen.as_ref().filter(|s| s.phase == Phase::Choosing) else { return 0 };
    let here = &screen.slots[screen.cursor as usize];
    let press = |k: u16| if last & (keys::A | keys::START) != 0 { 0 } else { k };
    let pickable = screen.slots.iter().any(|x| matches!(x.kind, SlotKind::Chip { .. }) && x.state == SlotState::Selectable);
    if screen.selected < 4 && pickable {
        if matches!(here.kind, SlotKind::Chip { .. }) && here.state == SlotState::Selectable {
            press(keys::A)
        } else {
            keys::RIGHT
        }
    } else if screen.cursor != OK_SLOT {
        press(keys::START)
    } else {
        press(keys::A)
    }
}

/// A small deterministic generator for the players' inputs.
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }
}

/// Record `ticks` ticks of the duel (a tape plays back the same with any
/// runtime options).
pub fn record(ticks: usize) -> Vec<Tick> {
    record_seeded(ticks, 7)
}

/// The same with the players' steps and shots drawn from another seed:
/// another duel. (A tape drives the custom screens too, so it plays back
/// only from the start of the round it was recorded on.)
pub fn record_seeded(ticks: usize, seed: u32) -> Vec<Tick> {
    record_on(setup(), ticks, seed)
}

/// The duel's players on another round (`setup_with`), with the steps and
/// shots from `seed`.
pub fn record_on(setup: RoundSetup, ticks: usize, seed: u32) -> Vec<Tick> {
    let mut b = Battle::new(setup, content());
    let mut rng = Lcg(seed);
    let mut tape = Vec::with_capacity(ticks);
    // Per side: a held direction and how long to keep it.
    let mut held = [0u16; 2];
    let mut hold_for = [0u32; 2];
    let mut last = [0u16; 2];
    for _ in 0..ticks {
        let events = TickEvents::default();
        let in_custom = b.round.mode == mode::CUSTOM;
        let mut input = [PlayerTick::default(), PlayerTick::default()];
        for side in 0..2 {
            if hold_for[side] == 0 {
                let r = rng.next();
                // Side 0 first steps into range (two panels from side 1),
                // then both step up and down and fire.
                let x = b.player(side as u8).map_or(0, |p| b.objects.get(p).panel.x);
                held[side] = match r % 16 {
                    _ if side == 0 && x == 2 => keys::RIGHT,
                    0..=3 => keys::UP,
                    4..=7 => keys::DOWN,
                    8 | 9 => keys::A,
                    _ => 0,
                };
                hold_for[side] = 2 + (r >> 4) % 12;
            }
            hold_for[side] -= 1;
            let buttons = if in_custom { custom_buttons(&b, side, last[side]) } else { held[side] };
            input[side] = PlayerTick { held: buttons };
            last[side] = buttons;
        }
        b.tick(&input, events.clone());
        tape.push(Tick { input, events });
    }
    tape
}

/// Play a tape from the start of the round with `behaviors`.
pub fn play(tape: &[Tick], behaviors: Behaviors) -> Battle {
    let mut b = Battle::with_behaviors(setup(), content(), behaviors);
    for t in tape {
        b.tick(&t.input, t.events.clone());
    }
    b
}
