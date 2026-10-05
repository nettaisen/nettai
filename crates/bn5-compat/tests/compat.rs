//! BN5's compat stays out of the engine, reads, and decodes BN5's records.
//! (Every recording of the verification workspace's lab-bn5 decodes: its
//! trace tests.)

use bn5_compat::Compat;
use bn5_compat::codec::{self, LightDark};
use nettai_battle::field::PanelType;

/// The engine can't depend on compat (content-model-v2.md §6.4).
#[test]
fn the_engine_does_not_depend_on_compat() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../nettai-battle/Cargo.toml")).unwrap();
    assert!(!manifest.contains("bn5-compat"), "nettai-battle's Cargo.toml names bn5-compat");
}

/// BN5's compat reads, built in and from its folder, the same.
#[test]
fn bn5_compat_reads() {
    let built_in = Compat::bn5();
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn5/compat"));
    assert_eq!(&Compat::read(dir).unwrap(), built_in);
    assert_eq!(built_in.chips["cannon"].id, 0x001);
    assert_eq!(built_in.chip_key(0x133), Some("holydrem"));
    assert_eq!(built_in.chip(0x133).as_deref(), Some("holydrem"));
    assert_eq!(built_in.chip_entry("holydrem").map(|c| c.id), Some(0x133));
    assert_eq!(built_in.chip_entry("bn6:holydrem"), None); // (written in full)
    let phoenix = &built_in.chips["phoenix"];
    assert_eq!(phoenix.colonel.as_ref().and_then(|c| c.flags.as_deref()).map(|f| f.contains(&"library".to_string())), Some(true));
    assert_eq!(built_in.chips["custswrd"].damage_formula, Some(45));
    // The e-Reader cards' chips: their strings and palettes are the save's.
    assert_eq!((built_in.chips["leadraid"].id, built_in.chips["leadraid"].save_slot), (0x137, Some(0)));
    assert_eq!((built_in.chips["chaoslrd"].id, built_in.chips["chaoslrd"].save_slot), (0x138, Some(1)));
    // BN5's holy panel is its type 9, the engine's Holy; metal, lava and
    // sea are BN5's own types (docs/design/bn5-map.md §15.3 item 1).
    assert_eq!(built_in.panel_type(9), Ok(Some(PanelType::Holy)));
    assert_eq!(built_in.panel_type(8), Ok(Some(PanelType::Lava)));
    assert_eq!(built_in.panel_type(5), Ok(Some(PanelType::Metal)));
    assert_eq!(built_in.panel_type(10), Ok(Some(PanelType::Sea)));
    assert!(built_in.panel_type(11).is_err());
    // The assets' names: BN6's where the asset or its place is BN6's.
    assert_eq!(built_in.assets.sounds.get("own-hit"), Some(&0x6B));
    assert_eq!(built_in.assets.sounds.get("winner-1"), Some(&0x1F));
    assert_eq!(built_in.assets.banners.get("program-advance"), Some(&0x24));
    assert_eq!(built_in.sprite_names().get(&(0x0C, 0x02)).map(String::as_str), Some("bomb"));
    // The statuses' bytes.
    assert_eq!(built_in.status(0x10).as_deref(), Some("paralyze-90"));
    assert_eq!(built_in.status(0x32).as_deref(), Some("blind-1200"));
}

/// Team ProtoMan's NaviStats block as the chip lab's team-plain recording
/// has it (its HP set to 200, which the save's patch cards take 150 off).
const TEAM_PROTOMAN: &str = "
    08 00 00 00 00 01 01 ff 00 32 07 06 03 00 00 00
    00 00 ff 00 00 00 00 00 00 00 00 00 00 00 00 00
    00 01 01 00 00 00 00 1f 00 00 00 02 00 00 07 ff
    ff 00 00 00 00 00 00 00 00 ff 01 00 00 00 c8 00
    32 00 32 00 f4 01 01 00 00 00 0a 00 00 00 00 00
    00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00";

fn bytes(hex: &str) -> [u8; 0x60] {
    let h: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    let v: Vec<u8> = (0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap()).collect();
    v.try_into().expect("0x60 bytes")
}

#[test]
fn navi_stats_decode() {
    let s = codec::navi_stats(&bytes(TEAM_PROTOMAN)).unwrap();
    assert_eq!((s.attack, s.custom_level, s.mega_level, s.giga_level), (0, 7, 6, 3));
    assert_eq!((s.max_base_hp, s.hp, s.max_hp), (200, 50, 50));
    assert_eq!(s.first_barrier, 1);
    assert_eq!(s.navi, 0);
    assert_eq!(s.light_dark, LightDark(500));
    // 500 uses light chips and keeps holy panels; 0 the other way.
    assert!(s.light_dark.may_use(1) && !s.light_dark.may_use(2) && !s.light_dark.clears_holy());
    assert!(!LightDark(0).may_use(1) && LightDark(0).may_use(2) && LightDark(0).clears_holy());
    // Between the two thresholds: light chips, and holy panels cleared.
    assert!(LightDark(480).may_use(1) && LightDark(480).clears_holy());
}

#[test]
fn chip_hands_decode() {
    // Boxer1's Program Advance as the lab recorded its hand: Boxer1 (0x146)
    // and a FireHit1 (0x097), picked as FireHit1 O, P, Q.
    let mut b = [0u8; 0x50];
    for (i, v) in [0x146u16, 0xFFFF, 0x097, 0xFFFF, 0xFFFF, 0xFFFF].iter().enumerate() {
        b[2 + 2 * i..4 + 2 * i].copy_from_slice(&v.to_le_bytes());
    }
    for (i, v) in [0x1C97u16, 0x1E97, 0x2097, 0xFFFF, 0xFFFF, 0xFFFF].iter().enumerate() {
        b[0x32 + 2 * i..0x34 + 2 * i].copy_from_slice(&v.to_le_bytes());
    }
    let h = codec::chip_hand(Compat::bn5(), &b).unwrap();
    assert_eq!(h.ids[0].as_ref().and_then(|c| c.key.as_deref()), Some("boxer1"));
    assert!(h.ids[1].is_none());
    let (chip, code) = h.selection[1].clone().unwrap();
    assert_eq!((chip.key.as_deref(), code), (Some("firehit1"), 15));
}

#[cfg(feature = "trace")]
#[test]
fn a_frame_line_decodes() {
    use bn5_compat::trace::{self, Line};
    let panels: Vec<String> = (0..18).map(|i| format!("[{}, {}]", if i / 6 == 1 { 9 } else { 2 }, (i % 6 >= 3) as u8)).collect();
    let block = "00".repeat(0x50);
    let line = format!(
        r#"{{"frame": 142, "state": [4, 0, 0, 4], "frames": 142, "ticks": 1, "link": 2, "rng1": 1, "rng2": 2, "bs": "", "fight": "", "gauge": 0, "gauge_rate": 32, "paused": 1, "hud_tasks": 0, "banner": "", "input": [[0, 0, 0], [0, 0, 0]], "objects": [{{"type": 1, "index": 0, "flags": 23, "params": 0, "state": [4, 6, 0, 4], "panel": [2, 2], "alliance": 0, "flip": 0, "hp": 850, "max_hp": 850, "pos": [0, 0, 0], "timer": 0, "anim": 0, "status": 0}}], "panels": [{}], "chip_blocks": ["{block}", "{block}"]}}"#,
        panels.join(", ")
    );
    let Line::Frame(f) = trace::parse_line(&line).unwrap() else { panic!("a frame line") };
    let d = trace::decode_frame(Compat::bn5(), &f).unwrap();
    assert_eq!(d.panels[6].kind, Some(PanelType::Holy));
    assert_eq!(d.panels[3].alliance, 1);
    assert_eq!(d.objects[0].pool, nettai_content_api::Pool::Actor);
}

/// A round's setup names what the content lacks (docs/design/bn5-map.md
/// §15.5), and its replay stops there: on content without BN5's (the
/// engine's test content), its chips, its navi and its stage.
#[cfg(feature = "trace")]
#[test]
fn a_rounds_setup_names_what_the_content_lacks() {
    use bn5_compat::trace::{self, Line, Stop};
    let stats: String = TEAM_PROTOMAN.split_whitespace().collect();
    // Cannon A (0x001, code 0) and a zeroed field (no chip).
    let folder = "0100".to_string() + "0000" + &"ffff".repeat(38);
    let line = format!(
        r#"{{"setup": {{"frame": 10, "game": "bn5", "settings_ptr": 0, "settings": "{}", "navi_stats": ["{stats}", "{stats}"], "folder": "{folder}", "battle_state": "{}", "rng1": 1, "rng2": 2, "game_versions": ["protoman", "colonel"], "game_regions": ["us", "us"], "folders": ["{folder}", "{folder}"], "joypad_phases": [0, 0], "frame_counter": 11}}}}"#,
        "00".repeat(0x10),
        "00".repeat(0xF0)
    );
    let Line::Setup(setup) = trace::parse_line(&line).unwrap() else { panic!("a setup line") };
    let round = trace::Round { setup: *setup, exchanges: Vec::new(), frames: Vec::new() };
    assert_eq!(round.chip_ids().unwrap(), [0x001]);
    let content = std::sync::Arc::new(nettai_battle::content::testing::build());
    let needs = round.needs(&content, Compat::bn5()).unwrap();
    assert_eq!(needs[0], "chip cannon (0x001)");
    // (Its navi's name, `megaman`, the test content has: a name is its
    // game's, and a round runs on its own game's content.)
    assert!(needs.iter().any(|n| n.starts_with("the stage")), "{needs:?}");
    assert_eq!(needs.last().map(String::as_str), Some("BN5's pack"));
    let replay = trace::run_round(&round, &content, Compat::bn5());
    assert!(matches!(replay.stopped, Some(Stop::Setup(ref e)) if e.contains("chip cannon")), "{:?}", replay.stopped);
}
