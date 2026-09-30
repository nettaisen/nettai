//! A synthetic netbattle for tests and benchmarks that need whole rounds
//! without golden traces: two MegaMen in the sun, each with four GunDelS3,
//! stepping about and firing. It is recorded as an input tape (both
//! players' inputs and the link events for every tick), the way a replay
//! is, so rollback runs and benchmarks can play it back exactly.

use crate::battle::{Battle, CustomResult, TickEvents, mode};
use crate::behavior::Behaviors;
use crate::hand::ChipHand;
use crate::input::{PlayerTick, keys};
use crate::setup::{BattleSettings, NaviStats, NaviWeapons, RoundSetup, SetScore};
use crate::transform::TransformRequest;

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

/// The round: a link battle with the usual two-navi list, on a field of
/// plain panels (no roads or ice to slide on).
pub fn setup() -> RoundSetup {
    let mut settings = BattleSettings::netbattle_from_bytes(&[
        0xE3, 0x64, 0x15, 0x00, 0x0B, 0x00, 0x38, 0x00, 0x8C, 0x0E, 0x00, 0x00, 0x92, 0x19, 0x0B, 0x08,
    ]);
    settings.layout = 0;
    RoundSetup {
        settings,
        navi_stats: [megaman(); 2],
        rng: 0x1234_5678,
        local_side: 0,
        score: SetScore::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        sp_times: Default::default(),
    }
}

/// Four GunDelS3 (code N), as the custom screen's chip block.
fn hand() -> ChipHand {
    let mut block = [0u8; 0x50];
    for i in 0..6 {
        let (id, selection) = if i < 4 { (0x11u16, 13 << 9 | 0x11u16) } else { (0xFFFF, 0xFFFF) };
        block[0x02 + 2 * i..0x04 + 2 * i].copy_from_slice(&id.to_le_bytes());
        block[0x32 + 2 * i..0x34 + 2 * i].copy_from_slice(&selection.to_le_bytes());
    }
    ChipHand::from_bytes(&block)
}

/// A small deterministic generator for the players' inputs.
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }
}

/// Record `ticks` ticks of the duel (playing it with the built-in kinds;
/// the tape is the same whatever content plays it back).
pub fn record(ticks: usize) -> Vec<Tick> {
    let mut b = Battle::with_behaviors(setup(), Behaviors::builtin());
    let mut rng = Lcg(7);
    let mut tape = Vec::with_capacity(ticks);
    let mut custom_ticks = 0u32;
    // Per side: a held direction and how long to keep it.
    let mut held = [0u16; 2];
    let mut hold_for = [0u32; 2];
    for _ in 0..ticks {
        let mut events = TickEvents::default();
        let in_custom = b.round.mode == mode::CUSTOM;
        if in_custom {
            custom_ticks += 1;
            if custom_ticks == 20 {
                events.local_confirm = true;
            }
            if custom_ticks == 32 {
                let result = |side: usize| CustomResult {
                    hand: Some(hand()),
                    navi_stats: b.stats[side],
                    transform: TransformRequest::NONE,
                };
                events.exchange = Some(Box::new([result(0), result(1)]));
            }
        } else {
            custom_ticks = 0;
        }
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
            input[side] =
                PlayerTick { held: if in_custom { 0 } else { held[side] }, in_custom: in_custom && custom_ticks < 20 };
        }
        b.tick(&input, events.clone());
        tape.push(Tick { input, events });
    }
    tape
}

/// Play a tape from the start of the round with `content`.
pub fn play(tape: &[Tick], content: Behaviors) -> Battle {
    let mut b = Battle::with_behaviors(setup(), content);
    for t in tape {
        b.tick(&t.input, t.events.clone());
    }
    b
}
