//! Compat stays out of the engine, and EXE6's reads.

use nettai_battle::object::Pool;
use exe6_compat::Compat;

/// The engine can't depend on compat (docs/design/content-model-v2.md
/// §6.4): its manifest names no exe6-compat.
#[test]
fn the_engine_does_not_depend_on_compat() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../nettai-battle/Cargo.toml")).unwrap();
    assert!(!manifest.contains("exe6-compat"), "nettai-battle's Cargo.toml names exe6-compat");
}

/// What content defines has no number in the engine: an object records its
/// kind's handle and a navi its content action's. Compat gives them the
/// original's numbers (docs/design/content-model-v2.md §7.3), and the
/// engine's own kinds' too (their keys are `engine/...`).
#[test]
fn compat_numbers_what_the_engine_runs_by_handle() {
    use nettai_battle::content::testing;
    use nettai_battle::kinds::player::{NaviAction, navi_action};
    use nettai_content_api::CoreApi;

    let content = std::sync::Arc::new(testing::with_test_pack());
    let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::stats(1000));
    testing::on(&mut setup, &content);
    let mut b = nettai_battle::Battle::new(setup, content);
    b.spawn_actors();
    let player = b.player(0).unwrap();
    let ticker = nettai_battle::behavior::spawn_kind(&mut b, "test/ticker", Default::default()).unwrap();
    let shot = b.content.defs.actions.iter().position(|a| a.key == "test/tick-shot/shot").unwrap() as u16;
    b.set_content_attack(player, shot, 1).unwrap();
    assert_eq!(navi_action(&b, player), NaviAction::Content(nettai_content_api::ActionHandle(shot)));

    let mut compat = Compat::default();
    // Without entries, compat says what it lacks.
    assert_eq!(compat.object_slot(&b, ticker), Err("kinds.toml has no \"test/ticker\"".into()));
    assert_eq!(compat.navi_action(&b, player), Err("actions.toml has no \"test/tick-shot/shot\"".into()));
    compat.kinds.insert(
        "test/ticker".into(),
        exe6_compat::KindEntry { pool: "effect".into(), index: 0xF0, ..Default::default() },
    );
    compat.actions.insert("test/tick-shot/shot".into(), 0x11);
    assert_eq!(compat.object_slot(&b, ticker), Ok((Pool::Effect, 0xF0)));
    assert_eq!(compat.navi_action(&b, player), Ok(0x11));
    // The engine's own kinds have their slots in compat too.
    assert_eq!(compat.object_slot(&b, player), Err("kinds.toml has no \"engine/player\"".into()));
    assert_eq!(Compat::exe6().object_slot(&b, player), Ok((Pool::Actor, 0)));
    // A kind in the wrong pool is compat's mistake.
    compat.kinds.get_mut("test/ticker").unwrap().pool = "attack".into();
    assert!(compat.object_slot(&b, ticker).is_err());
}

/// EXE6's compat reads, built in and from its folder, the same.
#[test]
fn exe6_compat_reads() {
    let built_in = Compat::exe6();
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/exe6/compat"));
    assert_eq!(&Compat::read(dir).unwrap(), built_in);
    assert_eq!(built_in.chips["minibomb"].id, 0x36);
    assert_eq!(built_in.chip_key(0x36), Some("minibomb"));
    assert_eq!(built_in.kind_at(Pool::Attack, 0x08).map(|(k, _)| k), Some("bomb"));
    assert!(built_in.kind_at(Pool::Effect, 0x0A).is_some_and(|(_, e)| e.scratch_position));
    // The buster's alias routines with charge times of their own are
    // weapons of their own.
    assert_eq!(built_in.weapon_key(0x00), Some("megaman/buster"));
    assert_eq!(built_in.weapon_key(0x2F), Some("megaman/buster-2e"));
    assert_eq!(built_in.actions["engine/move"], 0x10);
}

/// Two definitions with one of the original's numbers: reading compat
/// refuses it (several actions may share a number; nothing else).
#[test]
fn a_number_belongs_to_one_definition() {
    let file = |name: &str| std::fs::read_to_string(format!("{}/../../content/exe6/compat/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    for (name, extra, both) in [
        ("chips.toml", "\n[\"cannon-twin\"]\nid = 0x001\naction = 0\nsubtype = 0\n", "are both"),
        ("navis.toml", "\n[\"megaman-twin\"]\nnavi = 0x00\nname_id = 0x7FF\n", "are both"),
        ("kinds.toml", "\n[\"rock-twin\"]\npool = \"attack\"\nindex = 0x59\n", "both fill"),
        ("weapons.toml", "\n\"buster-twin\" = [0x00]\n", "are both"),
    ] {
        let text = file(name) + extra;
        match Compat::exe6_with(name, &text) {
            Err(e) => assert!(e.contains(name) && e.contains(both) && e.contains("-twin"), "{name}: {e}"),
            Ok(_) => panic!("{name}: a second definition of a number read"),
        }
    }
    // (Several actions on one number are the original's own: the chips of
    // one action handler.)
    let c = Compat::exe6();
    assert!(c.actions.values().filter(|&&n| n == c.actions["widesht/action"]).count() > 1);
}
