//! Compat stays out of the engine, and BN6's reads.

use bn6_battle::object::Pool;
use bn6_compat::Compat;

/// The engine can't depend on compat (docs/design/content-model-v2.md
/// §6.4): its manifest names no bn6-compat.
#[test]
fn the_engine_does_not_depend_on_compat() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../bn6-battle/Cargo.toml")).unwrap();
    assert!(!manifest.contains("bn6-compat"), "bn6-battle's Cargo.toml names bn6-compat");
}

/// What content defines has no number in the engine: an object records its
/// kind's handle and a navi its content action's. Compat gives them the
/// original's numbers (docs/design/content-model-v2.md §7.3), and the
/// engine's own kinds' too (their keys are `engine/...`).
#[test]
fn compat_numbers_what_the_engine_runs_by_handle() {
    use bn6_battle::content::testing;
    use bn6_battle::kinds::player::{NaviAction, navi_action};
    use bn6_content_api::CoreApi;

    let content = std::sync::Arc::new(testing::with_test_pack());
    let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::stats(1000));
    setup.content = content.hash();
    let mut b = bn6_battle::Battle::new(setup, content);
    b.spawn_actors();
    let player = b.player(0).unwrap();
    let ticker = bn6_battle::behavior::spawn_kind(&mut b, "test/ticker", Default::default()).unwrap();
    let shot = b.content.defs.actions.iter().position(|a| a.key == "test/tick-shot/shot").unwrap() as u16;
    b.set_content_attack(player, shot, 1).unwrap();
    assert_eq!(navi_action(&b, player), NaviAction::Content(bn6_content_api::ActionHandle(shot)));

    let mut compat = Compat::default();
    // Without entries, compat says what it lacks.
    assert_eq!(compat.object_slot(&b, ticker), Err("kinds.toml has no \"test/ticker\"".into()));
    assert_eq!(compat.navi_action(&b, player), Err("actions.toml has no \"test/tick-shot/shot\"".into()));
    compat.kinds.insert(
        "test/ticker".into(),
        bn6_compat::KindEntry { pool: "effect".into(), index: 0xF0, ..Default::default() },
    );
    compat.actions.insert("test/tick-shot/shot".into(), 0x11);
    assert_eq!(compat.object_slot(&b, ticker), Ok((Pool::Effect, 0xF0)));
    assert_eq!(compat.navi_action(&b, player), Ok(0x11));
    // The engine's own kinds have their slots in compat too.
    assert_eq!(compat.object_slot(&b, player), Err("kinds.toml has no \"engine/player\"".into()));
    assert_eq!(Compat::bn6().object_slot(&b, player), Ok((Pool::Actor, 0)));
    // A kind in the wrong pool is compat's mistake.
    compat.kinds.get_mut("test/ticker").unwrap().pool = "attack".into();
    assert!(compat.object_slot(&b, ticker).is_err());
}

/// BN6's compat reads, built in and from its folder, the same.
#[test]
fn bn6_compat_reads() {
    let built_in = Compat::bn6();
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6/compat"));
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
