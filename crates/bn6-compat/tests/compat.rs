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
    assert_eq!(built_in.weapon_key(0x2E), Some("megaman/buster"));
    assert_eq!(built_in.actions["engine/move"], 0x10);
}
