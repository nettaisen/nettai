//! A hit's NaviCust bug code sets the stat byte it names (`sub_80139F6`:
//! EXE6's table, content/exe6/rules/navicust/bugs with
//! @exelib/navicust/bugs): what EXE6's rules write is what setting that
//! byte of the stats' block decodes to (the codec, which reads the game's
//! records), and they refuse the bytes no stat holds and the content a
//! byte names by the original's number; the drains' codes and the custom
//! screen's damage's add to theirs.

use exe6_compat::Compat;
use exe6_compat::codec::{Ids, navi_stats, navi_stats_bytes};
use nettai_battle::Battle;
use nettai_battle::kinds::player::take_navi_bug;
use nettai_battle::setup::NaviStats;
use nettai_match::testing::exe6_content;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// A live match's round of EXE6, its navis in (their collision data
/// made).
fn battle() -> Battle {
    let content = exe6_content();
    let m = nettai_match::pick::live(&content, "exe6", 3, None).unwrap();
    let mut b = Battle::new(m.round(&content, 3), content.clone());
    for _ in 0..600 {
        if b.player(0).is_some_and(|r| b.objects.get(r).collision.is_some()) {
            return b;
        }
        b.tick(&Default::default(), Default::default());
    }
    panic!("side 0's navi has no collision data after 600 ticks");
}

/// Side 0's stats after its navi takes `code` with `arg`, or none where
/// the rules refused it.
fn bugged(b: &Battle, code: u8, arg: u8) -> Option<NaviStats> {
    let mut b = b.clone();
    catch_unwind(AssertUnwindSafe(move || {
        take_navi_bug(&mut b, 0, u16::from_le_bytes([code, arg]));
        b.stats[0]
    }))
    .ok()
}

#[test]
fn bug_codes_set_the_byte_they_name() {
    let b = battle();
    let content = exe6_content();
    let ids = Ids::new(&content, Compat::exe6_for(&content));
    let base = b.stats[0];
    let raw = navi_stats_bytes(&base, &ids);
    assert_eq!(navi_stats(&raw, &ids), base, "the stats round trip");
    std::panic::set_hook(Box::new(|_| {}));
    let mut problems = Vec::new();
    for offset in 1..0x64u8 {
        // (The drains' codes and the custom screen's damage's add: below.)
        if matches!(offset, 0x18 | 0x19 | 0x54) {
            continue;
        }
        let values: &[u8] = if offset == 0x08 { &[0, 1, 2] } else { &[0, 1, 2, 0x7F, 0xFF] };
        // A byte that names content by the original's number (a navi, a
        // form, a weapon, a shot program) decodes only where the content
        // has it.
        let decoded = |v: u8| {
            catch_unwind(|| {
                let mut r = raw;
                r[offset as usize] = v;
                navi_stats(&r, &ids)
            })
            .ok()
        };
        // (A value that doesn't decode is read as naming content: the byte
        // is held.)
        let held = values.iter().any(|&v| decoded(v).is_none_or(|d| d != base));
        // The content has no numbers for weapons, shot programs, first
        // barriers, forms or navis: a code can only clear those bytes (a
        // form's to the base form), and can't write a navi's.
        let by_number = |v: u8| match offset {
            0x04 | 0x05 | 0x07 | 0x39 | 0x44 => v != 0xFF,
            0x06 | 0x4D | 0x4F | 0x17 | 0x2C => v != 0,
            0x29 => true,
            _ => false,
        };
        for &value in values {
            match (bugged(&b, offset, value), decoded(value)) {
                (Some(s), Some(d)) if held && s != d => problems.push(format!("{offset:#x} = {value:#x}: {s:?}, not {d:?}")),
                (Some(_), _) if !held => problems.push(format!("{offset:#x} holds no stat but is accepted")),
                (Some(_), _) if by_number(value) => problems.push(format!("{offset:#x} = {value:#x} names content by number but is accepted")),
                (None, Some(_)) if held && !by_number(value) => problems.push(format!("{offset:#x} = {value:#x} is held but refused")),
                _ => {}
            }
        }
    }
    let _ = std::panic::take_hook();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn drain_codes_add_their_argument() {
    let mut b = battle();
    b.stats[0].bugs.hp_drain = 2;
    b.stats[0].bugs.custom_drain = 6;
    b.stats[0].bugs.custom_damage = 0x01F0;
    let s = bugged(&b, 0x18, 3).unwrap();
    assert_eq!(s.bugs.hp_drain, 5);
    let s = bugged(&b, 0x19, 3).unwrap();
    assert_eq!(s.bugs.custom_drain, 7, "at most 7");
    // (A byte store of the halfword plus the argument.)
    let s = bugged(&b, 0x54, 0x20).unwrap();
    assert_eq!(s.bugs.custom_damage, 0x0110);
}

#[test]
fn special_codes_are_their_own_bugs() {
    let b = battle();
    let s = bugged(&b, 0xFF, 0).unwrap();
    assert_eq!(s.bugs.buster_blanks, 4);
    for (code, trail) in [(0xFE, (4, 4)), (0xFA, (4, 2)), (0xF9, (4, 1)), (0xF5, (3, 1))] {
        let s = bugged(&b, code, 0).unwrap();
        assert_eq!((s.bugs.panel_trail_kind, s.bugs.panel_trail_level), trail, "{code:#x}");
    }
    let mut armored = b.clone();
    let st = &mut armored.stats[0];
    (st.super_armor, st.float_shoes, st.air_shoes, st.undershirt) = (true, true, true, true);
    let s = bugged(&armored, 0xFB, 0).unwrap();
    assert_eq!((s.super_armor, s.float_shoes, s.air_shoes, s.undershirt), (false, false, false, false));
    let s = bugged(&armored, 0xF8, 0).unwrap();
    assert_eq!((s.super_armor, s.float_shoes, s.air_shoes, s.undershirt), (false, false, false, true), "an uninstall leaves Undershirt");
    // (Codes past the block that aren't EXE6's own bugs do nothing.)
    assert_eq!(bugged(&b, 0x80, 7), Some(b.stats[0]));
}
