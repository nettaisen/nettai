//! EXE5's NaviCust bugs a hit inflicts (0x0801103E: content/exe5/rules/
//! navicust/bugs with @exelib/navicust/bugs, docs/design/exe5-map.md): a
//! drain's argument goes by its flags, a code names EXE5's own stats by
//! their bytes, and any other code's byte is written past the side's
//! block too (0x08011142: NaviStats, 0x60 a side, at 0x0203C880): side
//! 0's codes 0x60 to 0xBF are side 1's bytes, and the rest (0xF5 and 0xF8
//! among them, which EXE5 has no bug of its own for) RAM no stat holds.

use nettai_battle::Battle;
use nettai_battle::kinds::player::take_navi_bug;
use nettai_battle::setup::NaviStats;
use nettai_content_api::FieldValue;
use nettai_match::testing::exe5_content;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// A live match's round of EXE5, its navis in (their collision data made).
fn battle() -> Battle {
    let content = exe5_content();
    let m = nettai_match::pick::live(&content, "exe5", 3, None).unwrap();
    let mut b = Battle::new(m.round(&content, 3), content.clone());
    for _ in 0..600 {
        if b.player(0).is_some_and(|r| b.objects.get(r).collision.is_some()) {
            return b;
        }
        b.tick(&Default::default(), Default::default());
    }
    panic!("side 0's navi has no collision data after 600 ticks");
}

/// Both sides' stats after side `side`'s navi takes `code` with `arg`, or
/// none where the rules refused it.
fn bugged_by(b: &Battle, side: u8, code: u8, arg: u8) -> Option<[NaviStats; 2]> {
    let mut b = b.clone();
    catch_unwind(AssertUnwindSafe(move || {
        take_navi_bug(&mut b, side, u16::from_le_bytes([code, arg]));
        b.stats
    }))
    .ok()
}

/// Side 0's stats after its navi takes `code` with `arg`.
fn bugged(b: &Battle, code: u8, arg: u8) -> Option<NaviStats> {
    bugged_by(b, 0, code, arg).map(|s| s[0])
}

#[test]
fn drains_go_by_their_flags() {
    let mut b = battle();
    b.stats[0].bugs.hp_drain = 3;
    let drain = |arg: u8| bugged(&b, 0x18, arg).unwrap().bugs.hp_drain;
    // Bit 4 adds the low four bits (to at most 7), bit 5 takes them away (to
    // at least 0), else the level rises to them.
    assert_eq!(drain(0x12), 5);
    assert_eq!(drain(0x16), 7);
    assert_eq!(drain(0x21), 2);
    assert_eq!(drain(0x25), 0);
    assert_eq!(drain(0x05), 5);
    // (A level that wouldn't rise is left.)
    assert_eq!(drain(0x02), 3);
}

#[test]
fn codes_name_exe5s_own_stats() {
    let b = battle();
    let content = exe5_content();
    let stat = |s: &NaviStats, name: &str| s.game_stat(&content, name);
    let s = bugged(&b, 0x22, 2).unwrap();
    assert_eq!(stat(&s, "sun"), Some(FieldValue::Bool(true)));
    let s = bugged(&b, 0x32, 0xFE).unwrap();
    assert_eq!(stat(&s, "soul_turn_bonus"), Some(FieldValue::I8(-2)));
    let s = bugged(&b, 0x4C, 1).unwrap();
    assert_eq!(stat(&s, "hub_style"), Some(FieldValue::U8(1)));
}

#[test]
fn codes_past_the_block_are_written_past_it() {
    let b = battle();
    // Side 0's code 0x62 is side 1's byte 2, its Rapid.
    let s = bugged_by(&b, 0, 0x62, 0x2A).unwrap();
    assert_eq!((s[0], s[1].rapid), (b.stats[0], 0x2A));
    assert_eq!(s[1], nettai_battle::setup::NaviStats { rapid: 0x2A, ..b.stats[1] });
    // Side 1's, and side 0's from 0xC0 (0xF5 and 0xF8 among them), are RAM
    // past both blocks: no stat changes.
    for (side, code) in [(1, 0x62), (0, 0xC0), (0, 0xF5), (0, 0xF8), (1, 0xF8)] {
        assert_eq!(bugged_by(&b, side, code, 7), Some(b.stats), "side {side}, code {code:#x}");
    }
}
