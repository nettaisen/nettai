use super::*;
use crate::sim::{fight_runs, navi_hp, step_ticks};
use nettai_battle::battle::mode;
use nettai_battle::custom::Layer;
use nettai_battle::hand::ChipHand;
use nettai_battle::input::Keys;
use nettai_battle::{Battle, PlayerTick, Rng};
use nettai_frontend::driver::{Ai, Driver, InFight, LivePlayer, OnCustomScreen, Opponent, bot_tick};
use nettai_match::Set;
use nettai_netplay::standin::Masher;

/// Helper: produce one `PlayerTick` from `ai` (direct `ScreenChoice` on custom screen, buttons in fight).
fn ai_tick(ai: &mut Fighter, b: &Battle, side: u8) -> PlayerTick {
    if b.round.mode == mode::CUSTOM {
        PlayerTick::default().with_choice(ai.custom_screen(b, side))
    } else {
        PlayerTick::buttons(ai.fight(b, side, Keys::empty()))
    }
}

/// Helper: run a `LivePlayer` battle with `ai` on side 1 and `p0` on side 0 for up to `max_ticks`.
fn run_live(
    content: &std::sync::Arc<nettai_battle::Content>,
    game: &str,
    seed: u32,
    ai: Box<dyn Ai>,
    max_ticks: u32,
    mut p0: impl FnMut(&Battle, u32) -> PlayerTick,
) -> (Battle, u32, u32) {
    let m = nettai_match::pick::live(content, game, seed, None).unwrap();
    let set = Set::of(content, &m, seed);
    let opponent = Opponent { in_fight: InFight::Ai, on_custom_screen: OnCustomScreen::Ai };
    let mut driver = LivePlayer::against(set, opponent, seed as u64);
    driver.set_ai(Some(ai));
    let mut b = driver.start();
    let mut fight_ticks = 0u32;
    let mut screen_ticks = 0u32;
    for t in 0..max_ticks {
        if b.round_end().is_some() || b.is_stopped() {
            break;
        }
        if b.round.mode == mode::CUSTOM {
            screen_ticks += 1;
        } else if fight_runs(&b) {
            fight_ticks += 1;
        }
        let input = p0(&b, t);
        let step = driver.next(&b, input).unwrap();
        let ti = nettai_battle::TickInput { players: step.input, events: step.events };
        b.step(&ti);
        driver.follow(&b);
    }
    (b, fight_ticks, screen_ticks)
}

/// Across EXE6, EXE5, and EXE4, Hard AI closes the custom screen quickly, fights actively,
/// and takes significant HP away from a passive opponent.
#[test]
fn plays_and_damages_across_all_three_games() {
    for (game, content) in [
        ("exe6", nettai_match::testing::exe6_content()),
        ("exe5", nettai_match::testing::exe5_content()),
        ("exe4", nettai_match::testing::exe4_content()),
    ] {
        let (b, fight_ticks, screen_ticks) = run_live(
            &content,
            game,
            11,
            Box::new(Fighter::inline(Level::Hard, 42)),
            2400,
            |b, t| if b.round.mode == mode::CUSTOM { bot_tick(b, 0, t) } else { PlayerTick::default() },
        );
        assert!(!b.is_stopped(), "{game}: engine stopped");
        assert!(fight_ticks > 200, "{game}: spent too few ticks fighting ({fight_ticks}, screen={screen_ticks})");
        let hp = navi_hp(&b, 0);
        let max_hp = sim::navi_max_hp(&b, 0);
        let my_p = sim::navi_panel(&b, 1);
        let en_p = sim::navi_panel(&b, 0);
        let hand_keys: Vec<_> = b.hands[1].ids.iter().flatten().map(|c| content.defs.chip(*c).key.as_str()).collect();
        let actor_info = sim::navi_ref(&b, 1).and_then(|r| b.objects.get(r).actor).map(|a| {
            let ad = b.actors.get(a);
            (ad.charge_level, ad.charge_source, ad.charge_shot.is_some(), ad.navi_action, ad.status)
        });
        let form_key = content.defs.form(b.stats[1].form).key.as_str();
        let p52 = b.field.panel(5, 2).map(|p| (p.alliance, p.flags, p.kind));
        assert!(
            hp < max_hp,
            "{game}: AI dealt 0 damage in {fight_ticks} fight ticks (hp={hp}/{max_hp}, form={form_key}, my={my_p:?}, en={en_p:?}, p52={p52:?}, hand={hand_keys:?}, cursor={}, actor={actor_info:?})",
            b.hands[1].cursor
        );
    }
}

/// Every difficulty level (Easy, Normal, Hard) plays cleanly and deals damage.
#[test]
fn all_three_levels_fight_and_damage() {
    let content = nettai_match::testing::exe6_content();
    for level in Level::ALL {
        let (b, fight_ticks, _) = run_live(
            &content,
            "exe6",
            17,
            Box::new(Fighter::inline(level, 99)),
            2400,
            |b, t| if b.round.mode == mode::CUSTOM { bot_tick(b, 0, t) } else { PlayerTick::default() },
        );
        assert!(!b.is_stopped(), "{level:?}: engine stopped");
        assert!(fight_ticks > 100, "{level:?}: never reached fight");
        let hp = navi_hp(&b, 0);
        let max_hp = sim::navi_max_hp(&b, 0);
        assert!(hp < max_hp, "{level:?}: dealt 0 damage ({hp}/{max_hp})");
    }
}

/// The AI with its background worker thread (`Fighter::new`) works cleanly on a real battle.
#[test]
fn background_worker_plays_and_damages() {
    let content = nettai_match::testing::exe6_content();
    let (b, fight_ticks, _) = run_live(
        &content,
        "exe6",
        7,
        Box::new(Fighter::new(Level::Hard, 123)),
        2400,
        |b, t| if b.round.mode == mode::CUSTOM { bot_tick(b, 0, t) } else { PlayerTick::default() },
    );
    assert!(!b.is_stopped());
    assert!(fight_ticks > 100);
    assert!(navi_hp(&b, 0) < sim::navi_max_hp(&b, 0));
}

/// Determinism and `reset()`: replaying from the same seed (or after `reset()`) emits
/// the exact same buttons on every tick.
#[test]
fn deterministic_and_resettable() {
    let content = nettai_match::testing::exe6_content();
    let m = nettai_match::pick::live(&content, "exe6", 5, None).unwrap();

    let run = |ai: &mut Fighter| -> Vec<PlayerTick> {
        let set = Set::of(&content, &m, 5);
        let mut b = set.start();
        let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
        let mut out = Vec::new();
        for t in 0..600u32 {
            let p0 = if b.round.mode == mode::CUSTOM { bot_tick(&b, 0, t) } else { PlayerTick::default() };
            let p1 = ai_tick(ai, &b, 1);
            out.push(p1);
            let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
            assert!(step_ticks(&mut b, ticks));
            for l in &mut layers {
                l.follow(&b);
            }
        }
        out
    };

    let mut ai = Fighter::inline(Level::Normal, 777);
    let first = run(&mut ai);
    ai.reset();
    let second = run(&mut ai);
    assert_eq!(first, second, "reset must replay identically");
}

/// Fairness: the AI ignores `theirs` (player's live buttons), opponent's hidden hand,
/// and the real battle/console RNG.
#[test]
fn fairness_ignores_theirs_rng_and_enemy_hand() {
    let content = nettai_match::testing::exe6_content();
    let m = nettai_match::pick::live(&content, "exe6", 13, None).unwrap();
    let set = Set::of(&content, &m, 13);
    let mut b = set.start();
    let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];

    // Advance past intro + first custom screen into the fight.
    let mut ai_a = Fighter::inline(Level::Hard, 555);
    let mut ai_b = Fighter::inline(Level::Hard, 555);
    let mut fight_checked = 0;

    for t in 0..900u32 {
        let p0 = if b.round.mode == mode::CUSTOM { bot_tick(&b, 0, t) } else { PlayerTick::default() };
        let t_a = ai_tick(&mut ai_a, &b, 1);

        // Build a scrambled copy with different `theirs`, RNG, and opponent hidden hand.
        let t_b = if b.round.mode == mode::CUSTOM {
            PlayerTick::default().with_choice(ai_b.custom_screen(&b, 1))
        } else {
            let mut scrambled = b.clone();
            scrambled.rng = Rng::new(0xDEAD_BEEF ^ t);
            scrambled.consoles[0].rng = Rng::new(0xCAFE_BABE ^ t);
            scrambled.consoles[1].rng = Rng::new(0x1337_C0DE ^ t);
            if !scrambled.is_dimmed() {
                scrambled.hands[0] = ChipHand::empty(&content);
            }
            fight_checked += 1;
            PlayerTick::buttons(ai_b.fight(&scrambled, 1, Keys::A | Keys::B | Keys::UP | Keys::RIGHT))
        };

        assert_eq!(t_a, t_b, "tick {t}: AI read hidden state or `theirs`");

        let ticks = [layers[0].input(p0, &b), layers[1].input(t_a, &b)];
        assert!(step_ticks(&mut b, ticks));
        for l in &mut layers {
            l.follow(&b);
        }
    }
    assert!(fight_checked > 200, "should have checked many fight ticks ({fight_checked})");
}

/// Hard AI beats a random masher decisively across multiple seeds on EXE6.
#[test]
fn hard_ai_beats_masher() {
    let content = nettai_match::testing::exe6_content();
    let mut ai_damage_total = 0i64;
    let mut masher_damage_total = 0i64;
    for seed in [3u32, 11, 29] {
        let mut masher = Masher::new(seed as u64 ^ 0x1234);
        masher.buster = true;
        let (b, _, _) = run_live(&content, "exe6", seed, Box::new(Fighter::inline(Level::Hard, seed as u64)), 3600, |b, t| {
            if b.round.mode == mode::CUSTOM {
                bot_tick(b, 0, t)
            } else {
                PlayerTick::buttons(masher.buttons() & !(Keys::L | Keys::R))
            }
        });
        let p0_lost = (sim::navi_max_hp(&b, 0) - navi_hp(&b, 0)) as i64;
        let p1_lost = (sim::navi_max_hp(&b, 1) - navi_hp(&b, 1)) as i64;
        ai_damage_total += p0_lost;
        masher_damage_total += p1_lost;
    }
    assert!(
        ai_damage_total > masher_damage_total,
        "Hard AI should out-damage masher (AI dealt {ai_damage_total}, masher dealt {masher_damage_total})"
    );
}

/// Custom screen picks multiple chips and never leaves an unfolded modifier chip in hand.
#[test]
fn custom_screen_picks_well_and_never_stalls() {
    for (game, content) in [
        ("exe6", nettai_match::testing::exe6_content()),
        ("exe5", nettai_match::testing::exe5_content()),
        ("exe4", nettai_match::testing::exe4_content()),
    ] {
        let m = nettai_match::pick::live(&content, game, 21, None).unwrap();
        let set = Set::of(&content, &m, 21);
        let mut b = set.start();
        let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
        let mut ai = Fighter::inline(Level::Hard, 21);
        let mut screen_ticks = 0u32;
        let mut left_screen = false;

        for t in 0..1200u32 {
            let p0 = if b.round.mode == mode::CUSTOM { bot_tick(&b, 0, t) } else { PlayerTick::default() };
            let p1 = if b.round.mode == mode::CUSTOM {
                screen_ticks += 1;
                PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
            } else {
                if screen_ticks > 0 {
                    left_screen = true;
                    break;
                }
                PlayerTick::buttons(ai.fight(&b, 1, Keys::empty()))
            };
            let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
            assert!(step_ticks(&mut b, ticks));
            for l in &mut layers {
                l.follow(&b);
            }
        }
        assert!(left_screen, "{game}: AI stalled on custom screen ({screen_ticks} ticks)");
        assert!(screen_ticks < 400, "{game}: custom screen took too long ({screen_ticks} ticks)");
        let hand = &b.hands[1];
        assert!(hand.remaining() >= 1, "{game}: AI picked no chips");
        for id in hand.ids.iter().flatten() {
            assert!(
                content.chip(*id).modifier.is_none(),
                "{game}: AI left an unfolded modifier chip in hand: {}",
                content.defs.chip(*id).key
            );
        }
    }
}

/// Custom screen planner detects and forms a Program Advance when its ingredients are dealt!
#[test]
fn custom_screen_forms_program_advance() {
    use nettai_battle::content::{ChipCode, Recipe};
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    // Pick the first 3-chip Sequence Program Advance from content (no hardcoded chip names!).
    let pa = content
        .program_advances()
        .iter()
        .find(|p| !p.operation_battle_only && matches!(&p.recipe, Recipe::Sequence(v) if v.len() == 3))
        .expect("a 3-chip Sequence PA in exe6");
    let Recipe::Sequence(ref recipe_ids) = pa.recipe else { unreachable!() };
    let filler = content.defs.chips.iter().enumerate().find(|(_, d)| d.record.damage > 0 && d.record.codes.contains(&ChipCode(0))).map(|(i, _)| nettai_content_api::ChipHandle(i as u16)).unwrap();

    // Put the PA ingredients in common code `*` right in the folder (repeated so they're always dealt).
    let mut chips = [FolderChip::new(filler, ChipCode::ASTERISK); 30];
    for i in 0..30 {
        chips[i] = FolderChip::new(recipe_ids[i % 3], ChipCode::ASTERISK);
    }
    let folder = SavedFolder { chips, regular: None, tags: None };
    let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
    let setup = live_setup(&content, BattleSettings::on(&content, stage), [folder, folder], "gregar", 7);
    let mut b = Battle::new(setup, content.clone());
    let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
    let mut ai = Fighter::inline(Level::Hard, 7);

    let mut saw_custom = false;
    for t in 0..1200u32 {
        let p0 = if b.round.mode == mode::CUSTOM { bot_tick(&b, 0, t) } else { PlayerTick::default() };
        let p1 = if b.round.mode == mode::CUSTOM {
            saw_custom = true;
            PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
        } else {
            if saw_custom {
                break;
            }
            PlayerTick::buttons(ai.fight(&b, 1, Keys::empty()))
        };
        let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
        assert!(step_ticks(&mut b, ticks));
        for l in &mut layers {
            l.follow(&b);
        }
    }
    assert!(saw_custom);
    assert!(
        b.hands[1].ids.iter().flatten().any(|&c| c == pa.result),
        "expected Program Advance {} in hand, got {:?}",
        content.defs.chip(pa.result).key,
        b.hands[1].ids.iter().flatten().map(|c| &content.defs.chip(*c).key).collect::<Vec<_>>()
    );
}

/// Hard AI beats an active Shooter opponent (which walks to our row, uses chips, and shoots) by dodging and landing attacks.
#[test]
fn hard_ai_beats_shooter() {
    let content = nettai_match::testing::exe6_content();
    let mut ai_dealt = 0i64;
    let mut shooter_dealt = 0i64;
    for seed in [5u32, 19] {
        let mut last = Keys::empty();
        let mut held_ticks = 0u16;
        let (b, _, _) = run_live(&content, "exe6", seed, Box::new(Fighter::inline(Level::Hard, seed as u64)), 3600, |b, t| {
            if b.round.mode == mode::CUSTOM {
                last = Keys::empty();
                return bot_tick(b, 0, t);
            }
            let (Some(at), Some(them)) = (sim::navi_panel(b, 0), sim::navi_panel(b, 1)) else {
                return PlayerTick::default();
            };
            let pressed = if !last.is_empty() {
                held_ticks += 1;
                if last.intersects(Keys::UP | Keys::DOWN) && held_ticks < 10 && at.y != them.y {
                    last
                } else {
                    Keys::empty()
                }
            } else {
                held_ticks = 0;
                if at.y != them.y {
                    if them.y < at.y { Keys::UP } else { Keys::DOWN }
                } else if b.hands[0].remaining() > 0 {
                    Keys::A
                } else if t % 12 < 2 {
                    Keys::B
                } else {
                    Keys::empty()
                }
            };
            last = pressed;
            PlayerTick::buttons(pressed)
        });
        ai_dealt += (sim::navi_max_hp(&b, 0) - navi_hp(&b, 0)) as i64;
        shooter_dealt += (sim::navi_max_hp(&b, 1) - navi_hp(&b, 1)) as i64;
    }
    assert!(
        ai_dealt > shooter_dealt,
        "Hard AI should beat Shooter (AI dealt {ai_dealt}, Shooter dealt {shooter_dealt})"
    );
}

/// Hard AI outperforms Easy AI in head-to-head mirrored battles.
#[test]
fn hard_beats_easy_head_to_head() {
    let content = nettai_match::testing::exe6_content();
    let mut hard_net = 0i64;
    for seed in [7u32, 23] {
        let mut m = nettai_match::pick::live(&content, "exe6", seed, None).unwrap();
        // Mirror builds so only AI play decides.
        m.sides[1] = m.sides[0].clone();
        for swap in [false, true] {
            let set = Set::of(&content, &m, seed);
            let mut b = set.start();
            let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
            let (mut a0, mut a1) = if swap {
                (Fighter::inline(Level::Hard, seed as u64 + 1), Fighter::inline(Level::Easy, seed as u64 + 2))
            } else {
                (Fighter::inline(Level::Easy, seed as u64 + 1), Fighter::inline(Level::Hard, seed as u64 + 2))
            };
            for _ in 0..3600u32 {
                if b.round_end().is_some() || b.is_stopped() {
                    break;
                }
                let t0 = ai_tick(&mut a0, &b, 0);
                let t1 = ai_tick(&mut a1, &b, 1);
                let ticks = [layers[0].input(t0, &b), layers[1].input(t1, &b)];
                assert!(step_ticks(&mut b, ticks));
                for l in &mut layers {
                    l.follow(&b);
                }
            }
            let (hard_s, easy_s) = if swap { (0u8, 1u8) } else { (1u8, 0u8) };
            let hard_dealt = (sim::navi_max_hp(&b, easy_s) - navi_hp(&b, easy_s)) as i64;
            let easy_dealt = (sim::navi_max_hp(&b, hard_s) - navi_hp(&b, hard_s)) as i64;
            hard_net += hard_dealt - easy_dealt;
        }
    }
    assert!(hard_net > 0, "Hard AI should outperform Easy AI across mirrored deals (net HP diff: {hard_net})");
}

/// Interactive dimming aiming: AI casts EraseMan, Magnum, and ElementMan, tracks their moving aim cursors
/// via `Battle::dimmed_key_reads`, presses A on the exact hitting window (without timing out), and lands damage!
#[test]
fn interactive_dimming_aims_and_lands_eraseman_and_magnum() {
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    let filler = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "cannon")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("cannon");

    for chip_key in ["eraseman", "magnum", "elementman"] {
        let chip_id = content
            .defs
            .chips
            .iter()
            .enumerate()
            .find(|(_, d)| d.key == chip_key)
            .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
            .unwrap_or_else(|| panic!("missing chip {chip_key}"));
        let code = content.chip(chip_id).codes.first().copied().unwrap_or(ChipCode::ASTERISK);

        // Legal folder with the tested interactive dimming chip set as Regular (guaranteed dealt turn 1!).
        let mut chips = [FolderChip::new(filler, ChipCode::ASTERISK); 30];
        chips[0] = FolderChip::new(chip_id, code);
        let folder = SavedFolder { chips, regular: Some(0), tags: None };
        let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
        let setup = live_setup(&content, BattleSettings::on(&content, stage), [folder, folder], "gregar", 13);
        let mut b = Battle::new(setup, content.clone());
        let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
        let mut ai = Fighter::inline(Level::Hard, 13);

        let mut dimmed_ticks = 0u32;
        let mut spent = false;
        let mut hp_after_dim = None;

        for t in 0..1600u32 {
            if b.round_end().is_some() || b.is_stopped() {
                break;
            }
            let p0 = if b.round.mode == mode::CUSTOM { bot_tick(&b, 0, t) } else { PlayerTick::default() };
            let p1 = if b.round.mode == mode::CUSTOM {
                PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
            } else {
                if b.is_dimmed() {
                    dimmed_ticks += 1;
                }
                PlayerTick::buttons(ai.fight(&b, 1, Keys::empty()))
            };
            let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
            assert!(step_ticks(&mut b, ticks));
            for l in &mut layers {
                l.follow(&b);
            }
            if b.hands[1].cursor > 0 && b.is_dimmed() {
                spent = true;
            }
            if spent && !b.is_dimmed() && dimmed_ticks > 0 {
                hp_after_dim = Some(navi_hp(&b, 0));
                break;
            }
        }

        let max_hp = sim::navi_max_hp(&b, 0);
        let dealt = max_hp.saturating_sub(hp_after_dim.unwrap_or_else(|| navi_hp(&b, 0)));
        let my_p = sim::navi_panel(&b, 1);
        let en_p = sim::navi_panel(&b, 0);
        assert!(spent, "{chip_key}: AI never cast interactive dimming chip (my={my_p:?}, en={en_p:?})");
        assert!(
            dimmed_ticks < 450,
            "{chip_key}: AI waited too long / timed out during dimming ({dimmed_ticks} dimmed ticks, dealt={dealt}, my={my_p:?}, en={en_p:?})"
        );
        assert!(
            dealt >= 100,
            "{chip_key}: AI failed to aim and land interactive dimming chip (dealt={dealt}, dimmed_ticks={dimmed_ticks}, my={my_p:?}, en={en_p:?})"
        );
    }
}

/// EXE5 AI triggers Soul Unison via FormOffer, still selects compatible chips after the trade, and deals damage.
#[test]
fn exe5_soul_unison_transforms_and_fights() {
    let content = nettai_match::testing::exe5_content();
    let (b, fight_ticks, _) = run_live(
        &content,
        "exe5",
        11,
        Box::new(Fighter::inline(Level::Hard, 42)),
        2400,
        |b, t| if b.round.mode == mode::CUSTOM { bot_tick(b, 0, t) } else { PlayerTick::default() },
    );
    assert!(fight_ticks > 200);
    assert!(!b.form(1).base, "expected EXE5 AI to transform via Soul Unison");
    assert!(navi_hp(&b, 0) < sim::navi_max_hp(&b, 0), "expected EXE5 Soul Unison AI to deal damage");
}

/// Persistent ReachTable + Step→Fire: AI learns melee Sword reach, walks into front column, and lands WideSword.
#[test]
fn reach_table_and_step_fire_lands_close_range_sword() {
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    let ws = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "wide_sword")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("wide_sword");
    let invis = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "invisible")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("invisible");
    let ws_code = content.chip(ws).codes.first().copied().unwrap_or(ChipCode::ASTERISK);

    let mut chips = [FolderChip::new(invis, ChipCode::ASTERISK); 30];
    chips[0] = FolderChip::new(ws, ws_code);
    let folder = SavedFolder { chips, regular: Some(0), tags: None };
    let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
    let setup = live_setup(&content, BattleSettings::on(&content, stage), [folder, folder], "gregar", 19);
    let mut b = Battle::new(setup, content.clone());
    let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
    let mut ai = Fighter::inline(Level::Hard, 19);

    let mut fight_t = 0u32;
    let mut sword_used = false;
    for t in 0..1600u32 {
        if b.round_end().is_some() || b.is_stopped() {
            break;
        }
        let p0 = if b.round.mode == mode::CUSTOM {
            bot_tick(&b, 0, t)
        } else if sim::fight_runs(&b) {
            fight_t += 1;
            if sim::navi_panel(&b, 0).is_some_and(|p| p.x < 3) && fight_t % 3 == 1 {
                PlayerTick::buttons(Keys::RIGHT)
            } else {
                PlayerTick::default()
            }
        } else {
            PlayerTick::default()
        };
        let p1 = ai_tick(&mut ai, &b, 1);
        let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
        assert!(step_ticks(&mut b, ticks));
        for l in &mut layers {
            l.follow(&b);
        }
        if b.hands[1].cursor > 0 && sim::navi_max_hp(&b, 0).saturating_sub(navi_hp(&b, 0)) >= 80 {
            sword_used = true;
            break;
        }
    }
    let dealt = sim::navi_max_hp(&b, 0).saturating_sub(navi_hp(&b, 0));
    assert!(sword_used && dealt >= 80, "expected AI to walk in and land WideSword (used={sword_used}, dealt={dealt})");
}

/// ElementMan element selection during dimming: against an ElecCross opponent (weak to Wood = 2x + Cross break),
/// the AI skips Fire (100 dmg), Aqua (miss), and Elec (100 dmg), waits for Wood, and presses A on Wood for 200 dmg!
#[test]
fn elementman_selects_weakness_element_during_dimming() {
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::input::ScreenChoice;
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    let filler = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "cannon")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("cannon");
    let em = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "elementman")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("elementman");
    let code = content.chip(em).codes.first().copied().unwrap_or(ChipCode::ASTERISK);

    let mut chips = [FolderChip::new(filler, ChipCode::ASTERISK); 30];
    chips[0] = FolderChip::new(em, code);
    let folder = SavedFolder { chips, regular: Some(0), tags: None };
    let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
    let setup = live_setup(&content, BattleSettings::on(&content, stage), [folder, folder], "gregar", 13);
    let mut b = Battle::new(setup, content.clone());
    let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
    let mut ai = Fighter::inline(Level::Hard, 13);

    let mut p0_chose_form = false;
    let mut saw_elec_cross = false;
    let mut dimmed_ticks = 0u32;
    let mut spent = false;
    let mut hp_after_dim = None;

    for _ in 0..1600u32 {
        if b.round_end().is_some() || b.is_stopped() {
            break;
        }
        let p0 = if b.round.mode == mode::CUSTOM {
            // Side 0 picks ElecCross (index 1 in Gregar form_list), then OK!
            let choosing = b.custom.sides[0].screen.as_ref().is_some_and(|s| s.phase == nettai_battle::custom::screen::Phase::Choosing);
            if choosing {
                if !p0_chose_form {
                    p0_chose_form = true;
                    PlayerTick::default().with_choice(Some(ScreenChoice::ChooseForm(1)))
                } else {
                    PlayerTick::default().with_choice(Some(ScreenChoice::Ok))
                }
            } else {
                PlayerTick::default()
            }
        } else {
            PlayerTick::default()
        };
        let p1 = if b.round.mode == mode::CUSTOM {
            PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
        } else {
            if !saw_elec_cross && content.defs.form(b.stats[0].form).key == "elec_cross" {
                saw_elec_cross = true;
            }
            if b.is_dimmed() {
                dimmed_ticks += 1;
            }
            PlayerTick::buttons(ai.fight(&b, 1, Keys::empty()))
        };
        let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
        assert!(step_ticks(&mut b, ticks));
        for l in &mut layers {
            l.follow(&b);
        }
        if b.hands[1].cursor > 0 && b.is_dimmed() {
            spent = true;
        }
        if spent && !b.is_dimmed() && dimmed_ticks > 0 {
            hp_after_dim = Some(navi_hp(&b, 0));
            // Let any post-dim Cross-break revert animation finish.
            for _ in 0..30 {
                let ticks = [layers[0].input(PlayerTick::default(), &b), layers[1].input(PlayerTick::default(), &b)];
                assert!(step_ticks(&mut b, ticks));
                for l in &mut layers {
                    l.follow(&b);
                }
            }
            break;
        }
    }

    assert!(saw_elec_cross, "test setup: side 0 should have transformed into elec_cross");
    assert!(spent, "AI never cast ElementMan");
    let dealt = sim::navi_max_hp(&b, 0).saturating_sub(hp_after_dim.unwrap_or_else(|| navi_hp(&b, 0)));
    assert!(
        dealt >= 200,
        "expected ElementMan to skip Fire/Aqua/Elec (100 dmg) and pick Wood (200 dmg 2x weakness vs ElecCross), got dealt={dealt} in {dimmed_ticks} dimmed ticks (form0={})",
        content.defs.form(b.stats[0].form).key
    );
    assert!(b.form(0).base, "expected Wood hit from ElementMan to break opponent's ElecCross back to base form");
}

/// With `with_open_custom(false)` (used in practice mode), the AI never presses L or R to open the custom screen
/// even after the custom gauge stays full for hundreds of ticks.
#[test]
fn open_custom_disabled_never_opens_custom_screen() {
    let content = nettai_match::testing::exe6_content();
    let m = nettai_match::pick::live(&content, "exe6", 11, None).unwrap();
    let set = Set::of(&content, &m, 11);
    let mut b = set.start();
    let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
    let mut ai = Fighter::inline(Level::Hard, 11).with_open_custom(false);

    let mut left_first_custom = false;
    let mut full_gauge_ticks = 0u32;
    for t in 0..2000u32 {
        if b.round_end().is_some() || b.is_stopped() {
            break;
        }
        let p0 = if b.round.mode == mode::CUSTOM { bot_tick(&b, 0, t) } else { PlayerTick::default() };
        let p1 = if b.round.mode == mode::CUSTOM {
            assert!(!left_first_custom, "AI opened a second custom screen when open_custom=false!");
            PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
        } else {
            if !left_first_custom && sim::fight_runs(&b) {
                left_first_custom = true;
            }
            if sim::may_open(&b) {
                full_gauge_ticks += 1;
            }
            let k = ai.fight(&b, 1, Keys::empty());
            assert!(!k.intersects(Keys::L | Keys::R), "AI pressed L/R ({k:?}) with open_custom=false");
            PlayerTick::buttons(k)
        };
        let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
        assert!(step_ticks(&mut b, ticks));
        for l in &mut layers {
            l.follow(&b);
        }
    }
    assert!(full_gauge_ticks >= 200, "expected custom gauge to stay full without AI opening it ({full_gauge_ticks})");
}

/// Repositioning to aimable locations before casting dimming chips:
/// With opponent camped on `(1, 1)` (unreachable from `(5, 2)` by both EraseMan and JudgeMan!),
/// the AI probes aimable panels, walks into range (`y=1` for EraseMan, `(4, 1)` for JudgeMan), casts,
/// aims, and lands the full hit!
#[test]
fn eraseman_and_judgeman_reposition_to_aimable_panel() {
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    let filler = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "cannon")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("cannon");

    for chip_key in ["eraseman", "judgeman"] {
        let chip_id = content
            .defs
            .chips
            .iter()
            .enumerate()
            .find(|(_, d)| d.key == chip_key)
            .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
            .unwrap_or_else(|| panic!("missing chip {chip_key}"));
        let code = content.chip(chip_id).codes.first().copied().unwrap_or(ChipCode::ASTERISK);

        let mut chips = [FolderChip::new(filler, ChipCode::ASTERISK); 30];
        chips[0] = FolderChip::new(chip_id, code);
        let folder = SavedFolder { chips, regular: Some(0), tags: None };
        let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
        let setup = live_setup(&content, BattleSettings::on(&content, stage), [folder, folder], "gregar", 13);
        let mut b = Battle::new(setup, content.clone());
        let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
        let mut ai = Fighter::inline(Level::Hard, 13);

        // Move opponent (side 0) to (1, 1) right when fighting starts, BEFORE giving AI its turn to cast.
        let mut fight_t = 0u32;
        let mut spent_dim = false;
        let mut hp_after_dim = None;
        let mut dim_ticks = 0u32;

        for t in 0..2000u32 {
            if b.round_end().is_some() || b.is_stopped() {
                break;
            }
            let p0 = if b.round.mode == mode::CUSTOM {
                bot_tick(&b, 0, t)
            } else if sim::fight_runs(&b) {
                fight_t += 1;
                let en = sim::navi_panel(&b, 0);
                if en.is_some_and(|p| p.y > 1) && fight_t % 3 == 1 {
                    PlayerTick::buttons(Keys::UP)
                } else if en.is_some_and(|p| p.x > 1) && fight_t % 3 == 1 {
                    PlayerTick::buttons(Keys::LEFT)
                } else {
                    PlayerTick::default()
                }
            } else {
                PlayerTick::default()
            };
            let p1 = if b.round.mode == mode::CUSTOM {
                PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
            } else if sim::fight_runs(&b) && fight_t < 24 {
                // Let side 0 settle at (1, 1) first so the test strictly tests repositioning from (5, 2) -> aimable panel!
                PlayerTick::default()
            } else {
                if b.is_dimmed() {
                    dim_ticks += 1;
                }
                PlayerTick::buttons(ai.fight(&b, 1, Keys::empty()))
            };
            let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
            assert!(step_ticks(&mut b, ticks));
            for l in &mut layers {
                l.follow(&b);
            }
            if b.hands[1].cursor > 0 && b.is_dimmed() {
                spent_dim = true;
            }
            if spent_dim && !b.is_dimmed() && dim_ticks > 0 {
                hp_after_dim = Some(navi_hp(&b, 0));
                break;
            }
        }

        let dealt = sim::navi_max_hp(&b, 0).saturating_sub(hp_after_dim.unwrap_or_else(|| navi_hp(&b, 0)));
        let my_p = sim::navi_panel(&b, 1);
        let en_p = sim::navi_panel(&b, 0);
        assert!(
            spent_dim && dealt >= 100,
            "{chip_key}: expected AI to reposition to hit enemy at {en_p:?} (spent={spent_dim}, dealt={dealt}, my={my_p:?})"
        );
    }
}

/// ElementMan positioning + element selection: against a SpoutCross opponent at `(1, 2)` (4 columns away,
/// out of ElementMan Elec's 3-column strike!), the AI steps forward to `(4, 2)` (`dx = 3`), casts ElementMan,
/// selects Elec, deals 200 dmg (2x weakness!), and breaks SpoutCross!
#[test]
fn elementman_steps_into_weakness_range_for_cross_break() {
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::input::ScreenChoice;
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    let filler = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "cannon")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("cannon");
    let em = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "elementman")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("elementman");
    let code = content.chip(em).codes.first().copied().unwrap_or(ChipCode::ASTERISK);

    let mut chips = [FolderChip::new(filler, ChipCode::ASTERISK); 30];
    chips[0] = FolderChip::new(em, code);
    let folder = SavedFolder { chips, regular: Some(0), tags: None };
    let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
    // Falzar form_list[0] is aqua_cross (SpoutCross, weak to Elec = 2x!).
    let setup = live_setup(&content, BattleSettings::on(&content, stage), [folder, folder], "falzar", 13);
    let mut b = Battle::new(setup, content.clone());
    let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
    let mut ai = Fighter::inline(Level::Hard, 13);

    let mut p0_chose_form = false;
    let mut fight_t = 0u32;
    let mut spent_dim = false;
    let mut dim_ticks = 0u32;
    let mut hp_after_dim = None;

    for _ in 0..2000u32 {
        if b.round_end().is_some() || b.is_stopped() {
            break;
        }
        let p0 = if b.round.mode == mode::CUSTOM {
            let choosing = b.custom.sides[0].screen.as_ref().is_some_and(|s| s.phase == nettai_battle::custom::screen::Phase::Choosing);
            if choosing {
                if !p0_chose_form {
                    p0_chose_form = true;
                    PlayerTick::default().with_choice(Some(ScreenChoice::ChooseForm(0)))
                } else {
                    PlayerTick::default().with_choice(Some(ScreenChoice::Ok))
                }
            } else {
                PlayerTick::default()
            }
        } else if sim::fight_runs(&b) {
            fight_t += 1;
            let en = sim::navi_panel(&b, 0);
            if en.is_some_and(|p| p.x > 1) && fight_t % 3 == 1 {
                PlayerTick::buttons(Keys::LEFT)
            } else {
                PlayerTick::default()
            }
        } else {
            PlayerTick::default()
        };
        let p1 = if b.round.mode == mode::CUSTOM {
            PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
        } else if sim::fight_runs(&b) && fight_t < 18 {
            PlayerTick::default()
        } else {
            if b.is_dimmed() {
                dim_ticks += 1;
            }
            PlayerTick::buttons(ai.fight(&b, 1, Keys::empty()))
        };
        let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
        assert!(step_ticks(&mut b, ticks));
        for l in &mut layers {
            l.follow(&b);
        }
        if b.hands[1].cursor > 0 && b.is_dimmed() {
            spent_dim = true;
        }
        if spent_dim && !b.is_dimmed() && dim_ticks > 0 {
            hp_after_dim = Some(navi_hp(&b, 0));
            for _ in 0..30 {
                let ticks = [layers[0].input(PlayerTick::default(), &b), layers[1].input(PlayerTick::default(), &b)];
                assert!(step_ticks(&mut b, ticks));
                for l in &mut layers {
                    l.follow(&b);
                }
            }
            break;
        }
    }

    let dealt = sim::navi_max_hp(&b, 0).saturating_sub(hp_after_dim.unwrap_or_else(|| navi_hp(&b, 0)));
    assert!(
        spent_dim && dealt >= 200 && b.form(0).base,
        "expected AI to step to dx=3 and cast ElementMan Elec (200 dmg + SpoutCross break), got spent={spent_dim}, dealt={dealt}, my={:?}, en={:?}, form0={}",
        sim::navi_panel(&b, 1),
        sim::navi_panel(&b, 0),
        content.defs.form(b.stats[0].form).key
    );
}

/// Defense chip discipline & smart dimming cut-in saving:
/// 1. With Invisible in hand against a passive opponent, the AI does NOT waste Invisible unprovoked.
/// 2. Against a blockable dimming Navi chip (ProtoMan), the AI cuts in with its saved Invisible during
///    the telop and takes 0 damage!
/// 3. Against an Invis-piercing dimming Navi chip (JudgeMan — Curse hit!), the AI simulates the pierce
///    and does NOT waste its Invisible!
#[test]
fn saves_invisible_unprovoked_and_cuts_in_against_enemy_dimming_attack() {
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    let invis = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "invisible")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("invisible");

    for (enemy_chip_key, expect_cut_in) in [("protoman", true), ("judgeman", false)] {
        let enemy_chip = content
            .defs
            .chips
            .iter()
            .enumerate()
            .find(|(_, d)| d.key == enemy_chip_key)
            .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
            .unwrap_or_else(|| panic!("missing {enemy_chip_key}"));
        let code = content.chip(enemy_chip).codes.first().copied().unwrap_or(ChipCode::ASTERISK);

        let f0 = [FolderChip::new(enemy_chip, code); 30];
        let f1 = [FolderChip::new(invis, ChipCode::ASTERISK); 30];
        let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
        let setup = live_setup(
            &content,
            BattleSettings::on(&content, stage),
            [
                SavedFolder { chips: f0, regular: Some(0), tags: None },
                SavedFolder { chips: f1, regular: Some(0), tags: None },
            ],
            "gregar",
            13,
        );
        let mut b = Battle::new(setup, content.clone());
        let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
        let mut ai = Fighter::inline(Level::Hard, 13).with_open_custom(false);

        let mut fight_t = 0u32;
        let mut p0_cast = false;
        let mut saw_dim_after_cast = false;
        let mut saw_ai_cut_in = false;
        let mut hp1_before = 0u16;

        for t in 0..1600u32 {
            if b.round_end().is_some() || b.is_stopped() {
                break;
            }
            let p0 = if b.round.mode == mode::CUSTOM {
                bot_tick(&b, 0, t)
            } else if sim::fight_runs(&b) {
                fight_t += 1;
                // First 80 fight ticks: side 0 does nothing -> AI MUST hold its Invisible!
                if fight_t == 80 {
                    assert_eq!(
                        b.hands[1].cursor, 0,
                        "{enemy_chip_key}: AI wasted Invisible unprovoked during the first 80 passive fight ticks!"
                    );
                }
                if fight_t >= 80 && !p0_cast {
                    let (Some(p0_pos), Some(p1_pos)) = (sim::navi_panel(&b, 0), sim::navi_panel(&b, 1)) else {
                        continue;
                    };
                    if p0_pos.y != p1_pos.y {
                        if fight_t % 3 == 1 {
                            PlayerTick::buttons(if p1_pos.y < p0_pos.y { Keys::UP } else { Keys::DOWN })
                        } else {
                            PlayerTick::default()
                        }
                    } else if p0_pos.x < 3 {
                        if fight_t % 3 == 1 {
                            PlayerTick::buttons(Keys::RIGHT)
                        } else {
                            PlayerTick::default()
                        }
                    } else if sim::navi_idle(&b, 0) && sim::navi_idle(&b, 1) {
                        p0_cast = true;
                        hp1_before = navi_hp(&b, 1);
                        PlayerTick::buttons(Keys::A)
                    } else {
                        PlayerTick::default()
                    }
                } else {
                    PlayerTick::default()
                }
            } else {
                PlayerTick::default()
            };
            let p1 = ai_tick(&mut ai, &b, 1);
            let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
            assert!(step_ticks(&mut b, ticks));
            for l in &mut layers {
                l.follow(&b);
            }
            if p0_cast && b.is_dimmed() {
                saw_dim_after_cast = true;
                if b.hands[1].cursor > 0 {
                    saw_ai_cut_in = true;
                }
            }
            if saw_dim_after_cast && !b.is_dimmed() {
                break;
            }
        }

        assert!(saw_dim_after_cast, "{enemy_chip_key}: test setup never saw enemy dimming");
        assert_eq!(
            saw_ai_cut_in, expect_cut_in,
            "{enemy_chip_key}: expected cut_in={expect_cut_in}, got {saw_ai_cut_in} (hp1={hp1_before}->{})",
            navi_hp(&b, 1)
        );
        if expect_cut_in {
            assert_eq!(
                navi_hp(&b, 1),
                hp1_before,
                "{enemy_chip_key}: expected Invisible cut-in to completely negate damage!"
            );
        }
    }
}

/// Do not waste attack chips (even slow-moving projectile chips like Thunder / AirHockey) while the opponent
/// is Invisible — hold them until Invisible wears off!
#[test]
fn does_not_waste_attack_chips_on_invisible_opponent() {
    use nettai_battle::collision::StatusFlags;
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    let invis = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "invisible")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("invisible");

    for level in Level::ALL {
        for atk_key in ["thunder", "air_hockey", "cannon"] {
            let atk = content
                .defs
                .chips
                .iter()
                .enumerate()
                .find(|(_, d)| d.key == atk_key)
                .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
                .unwrap_or_else(|| panic!("missing {atk_key}"));
            let code = content.chip(atk).codes.first().copied().unwrap_or(ChipCode::ASTERISK);

            let f0 = [FolderChip::new(invis, ChipCode::ASTERISK); 30];
            let f1 = [FolderChip::new(atk, code); 30];
            let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
            let setup = live_setup(
                &content,
                BattleSettings::on(&content, stage),
                [
                    SavedFolder { chips: f0, regular: Some(0), tags: None },
                    SavedFolder { chips: f1, regular: Some(0), tags: None },
                ],
                "gregar",
                13,
            );
            let mut b = Battle::new(setup, content.clone());
            let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
            let mut ai = Fighter::inline(level, 13).with_open_custom(false);

            let mut p0_used_invis = false;
            let mut invis_ticks = 0u32;
            let mut cursor_when_invis_started = None;

            for t in 0..1200u32 {
                if b.round_end().is_some() || b.is_stopped() {
                    break;
                }
                let p0 = if b.round.mode == mode::CUSTOM {
                    bot_tick(&b, 0, t)
                } else if sim::fight_runs(&b) && !p0_used_invis && sim::navi_idle(&b, 0) {
                    p0_used_invis = true;
                    PlayerTick::buttons(Keys::A)
                } else {
                    PlayerTick::default()
                };
                let p1 = if b.round.mode == mode::CUSTOM {
                    PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
                } else if !p0_used_invis || !sim::navi_status(&b, 0).contains(StatusFlags::INVISIBLE) {
                    // Let side 0 pop Invisible cleanly first so the test measures AI decisions DURING Invisible.
                    PlayerTick::default()
                } else {
                    PlayerTick::buttons(ai.fight(&b, 1, Keys::empty()))
                };
                let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
                assert!(step_ticks(&mut b, ticks));
                for l in &mut layers {
                    l.follow(&b);
                }
                if p0_used_invis && !b.is_dimmed() && sim::navi_status(&b, 0).contains(StatusFlags::INVISIBLE) {
                    let start_c = *cursor_when_invis_started.get_or_insert(b.hands[1].cursor);
                    invis_ticks += 1;
                    // Check ticks 25..250 of Invisible (after Easy's 22-tick reaction delay, well before Invis wears off at 360):
                    // AI must NOT waste its attack chip or fire B-charge/buster shots into an Invisible opponent!
                    if (25..=250).contains(&invis_ticks) {
                        assert_eq!(
                            b.hands[1].cursor, start_c,
                            "{level:?}/{atk_key}: AI wasted attack chip into Invisible opponent at invis_tick={invis_ticks}!"
                        );
                        let action = sim::navi_ref(&b, 1)
                            .and_then(|r| b.objects.get(r).actor)
                            .map(|a| b.actors.get(a).navi_action);
                        if let Some(nettai_battle::kinds::player::NaviAction::Content(h)) = action {
                            panic!(
                                "{level:?}/{atk_key}: AI fired an attack ({:?}, form={}) into Invisible opponent at invis_tick={invis_ticks}!",
                                content.defs.action(h).key,
                                content.defs.form(b.stats[1].form).key
                            );
                        }
                    } else if invis_ticks > 250 {
                        break;
                    }
                }
            }
            assert!(invis_ticks >= 250, "{level:?}/{atk_key}: test setup didn't run enough Invisible ticks ({invis_ticks})");
        }
    }
}

/// Do not cut in with a trap chip (`anti_damage`) against a dimming chip that removes traps (`circle_gun`)!
#[test]
fn does_not_cut_in_with_trap_against_trap_removing_circle_gun() {
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    let cg = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "circle_gun")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("circle_gun");
    let ad = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "anti_damage")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("anti_damage");
    let cg_code = content.chip(cg).codes.first().copied().unwrap_or(ChipCode::ASTERISK);

    let f0 = [FolderChip::new(cg, cg_code); 30];
    let f1 = [FolderChip::new(ad, ChipCode::ASTERISK); 30];
    let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
    let setup = live_setup(
        &content,
        BattleSettings::on(&content, stage),
        [
            SavedFolder { chips: f0, regular: Some(0), tags: None },
            SavedFolder { chips: f1, regular: Some(0), tags: None },
        ],
        "gregar",
        13,
    );
    let mut b = Battle::new(setup, content.clone());
    let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
    let mut ai = Fighter::inline(Level::Hard, 13).with_open_custom(false);

    let mut p0_cast = false;
    let mut saw_dim = false;
    let mut cut_in_with_trap = false;

    for t in 0..1400u32 {
        if b.round_end().is_some() || b.is_stopped() {
            break;
        }
        let p0 = if b.round.mode == mode::CUSTOM {
            bot_tick(&b, 0, t)
        } else if sim::fight_runs(&b) && !p0_cast && sim::navi_idle(&b, 0) {
            p0_cast = true;
            PlayerTick::buttons(Keys::A)
        } else {
            PlayerTick::default()
        };
        let p1 = if b.round.mode == mode::CUSTOM {
            PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
        } else if !p0_cast {
            PlayerTick::default()
        } else {
            PlayerTick::buttons(ai.fight(&b, 1, Keys::empty()))
        };
        let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
        assert!(step_ticks(&mut b, ticks));
        for l in &mut layers {
            l.follow(&b);
        }
        if p0_cast && b.is_dimmed() {
            saw_dim = true;
            if b.hands[1].cursor > 0 {
                cut_in_with_trap = true;
            }
        }
        if saw_dim && !b.is_dimmed() {
            break;
        }
    }

    assert!(saw_dim, "test setup never saw CircGun dimming");
    assert!(!cut_in_with_trap, "AI wasted AntiDmg trap cutting in on CircGun (which removes traps)!");
}

/// Invis-piercing chips (`gun_del_sol_1` drain & `judgeman` curse): even when the player is Invisible AND
/// out of initial range, the AI probes their reach, repositions into range, and lands damage through Invisible!
#[test]
fn gundelsol_and_judgeman_reposition_and_hit_through_invis() {
    use nettai_battle::collision::StatusFlags;
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    let invis = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "invisible")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("invisible");

    for chip_key in ["gun_del_sol_1", "judgeman"] {
        let chip_id = content
            .defs
            .chips
            .iter()
            .enumerate()
            .find(|(_, d)| d.key == chip_key)
            .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
            .unwrap_or_else(|| panic!("missing {chip_key}"));
        let code = content.chip(chip_id).codes.first().copied().unwrap_or(ChipCode::ASTERISK);

        let f0 = [FolderChip::new(invis, ChipCode::ASTERISK); 30];
        let mut f1 = [FolderChip::new(invis, ChipCode::ASTERISK); 30];
        f1[0] = FolderChip::new(chip_id, code);
        let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
        let setup = live_setup(
            &content,
            BattleSettings::on(&content, stage),
            [
                SavedFolder { chips: f0, regular: Some(0), tags: None },
                SavedFolder { chips: f1, regular: Some(0), tags: None },
            ],
            "gregar",
            13,
        );
        let mut b = Battle::new(setup, content.clone());
        let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
        let mut ai = Fighter::inline(Level::Hard, 13).with_open_custom(false);

        let mut fight_t = 0u32;
        let mut p0_used_invis = false;
        let mut invis_elapsed = 0u32;
        let mut first_dmg_at_invis_t = None;
        let mut hp0_at_invis = None;

        for t in 0..1400u32 {
            if b.round_end().is_some() || b.is_stopped() {
                break;
            }
            let p0 = if b.round.mode == mode::CUSTOM {
                bot_tick(&b, 0, t)
            } else if sim::fight_runs(&b) {
                fight_t += 1;
                // Move side 0 to (2, 1) (out of initial range for both GunDelSol dx=3 and JudgeMan row=2!),
                // then pop Invisible!
                let en = sim::navi_panel(&b, 0);
                if !p0_used_invis && en.is_some_and(|p| p.y > 1) && fight_t % 3 == 1 {
                    PlayerTick::buttons(Keys::UP)
                } else if !p0_used_invis && en.is_some_and(|p| p.y == 1) && sim::navi_idle(&b, 0) {
                    p0_used_invis = true;
                    PlayerTick::buttons(Keys::A)
                } else {
                    PlayerTick::default()
                }
            } else {
                PlayerTick::default()
            };
            // AI is ONLY allowed to input while `p0` is actively Invisible (so every step and chip press
            // must happen strictly while `p0` has `StatusFlags::INVISIBLE`!).
            let p0_invis = sim::navi_status(&b, 0).contains(StatusFlags::INVISIBLE);
            let p1 = if b.round.mode == mode::CUSTOM {
                PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
            } else if !p0_used_invis || !p0_invis {
                PlayerTick::default()
            } else {
                PlayerTick::buttons(ai.fight(&b, 1, Keys::empty()))
            };
            let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
            assert!(step_ticks(&mut b, ticks));
            for l in &mut layers {
                l.follow(&b);
            }
            if p0_used_invis && !b.is_dimmed() {
                if p0_invis {
                    hp0_at_invis.get_or_insert(navi_hp(&b, 0));
                }
                if let Some(start_hp) = hp0_at_invis {
                    invis_elapsed += 1;
                    let dealt = start_hp.saturating_sub(navi_hp(&b, 0));
                    if dealt > 0 && first_dmg_at_invis_t.is_none() {
                        first_dmg_at_invis_t = Some(invis_elapsed);
                    }
                    if dealt >= 80 {
                        break;
                    }
                }
            }
        }

        let dealt = hp0_at_invis.map_or(0, |s| s.saturating_sub(navi_hp(&b, 0)));
        assert!(
            dealt >= 80 && first_dmg_at_invis_t.is_some_and(|t| t < 200),
            "{chip_key}: expected AI to reposition and pierce Invisible early (dealt={dealt}, first_dmg_t={first_dmg_at_invis_t:?}, my={:?}, en={:?}, cur={})",
            sim::navi_panel(&b, 1),
            sim::navi_panel(&b, 0),
            b.hands[1].cursor
        );
    }
}

/// AquaCross passively heals +5% HP when using Water chips, but the AI must NOT dump a Water attack
/// chip (`train_arrow_3`) into empty air while the opponent is Invisible/off-row just for the passive heal!
#[test]
fn aquacross_does_not_dump_water_attack_chip_for_passive_heal() {
    use nettai_battle::collision::StatusFlags;
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::{FolderChip, SavedFolder};
    use nettai_battle::setup::BattleSettings;
    use nettai_frontend::driver::live_setup;

    let content = nettai_match::testing::exe6_content();
    let invis = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "invisible")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("invisible");
    let train_arrow = content
        .defs
        .chips
        .iter()
        .enumerate()
        .find(|(_, d)| d.key == "train_arrow_3")
        .map(|(i, _)| nettai_content_api::ChipHandle(i as u16))
        .expect("train_arrow_3");
    let ta_code = content.chip(train_arrow).codes[0];

    let f0 = [FolderChip::new(invis, ChipCode::ASTERISK); 30];
    let mut f1 = [FolderChip::new(invis, ChipCode::ASTERISK); 30];
    f1[0] = FolderChip::new(train_arrow, ta_code);
    f1[1] = FolderChip::new(train_arrow, ta_code);

    let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
    let setup = live_setup(
        &content,
        BattleSettings::on(&content, stage),
        [
            SavedFolder { chips: f0, regular: Some(0), tags: None },
            SavedFolder { chips: f1, regular: Some(0), tags: None },
        ],
        "falzar",
        13,
    );
    let mut b = Battle::new(setup, content.clone());
    let mut layers = [Layer::new(0, &b), Layer::new(1, &b)];
    let mut ai = Fighter::inline(Level::Hard, 13).with_open_custom(false);

    let mut fight_t = 0u32;
    let mut hurt_ai = false;
    let mut p0_used_invis = false;
    let mut saw_invis_ticks = 0u32;
    let mut wasted_during_invis = false;

    for t in 0..900u32 {
        if b.round_end().is_some() || b.is_stopped() {
            break;
        }
        let p0 = if b.round.mode == mode::CUSTOM {
            bot_tick(&b, 0, t)
        } else if sim::fight_runs(&b) {
            fight_t += 1;
            if !hurt_ai
                && let Some(r1) = sim::navi_ref(&b, 1)
            {
                // Drop AI to 45% HP so AquaCross passive heal is strongly valued by raw heal_weight.
                b.objects.get_mut(r1).hp = 450;
                hurt_ai = true;
            }
            let en = sim::navi_panel(&b, 0);
            if !p0_used_invis && en.is_some_and(|p| p.y > 1) && fight_t % 3 == 1 {
                PlayerTick::buttons(Keys::UP)
            } else if !p0_used_invis && en.is_some_and(|p| p.y == 1) && sim::navi_idle(&b, 0) {
                p0_used_invis = true;
                PlayerTick::buttons(Keys::A)
            } else {
                PlayerTick::default()
            }
        } else {
            PlayerTick::default()
        };

        let p0_invis = sim::navi_status(&b, 0).contains(StatusFlags::INVISIBLE);
        let p1 = if b.round.mode == mode::CUSTOM {
            PlayerTick::default().with_choice(ai.custom_screen(&b, 1))
        } else if !p0_used_invis || !p0_invis {
            PlayerTick::default()
        } else {
            PlayerTick::buttons(ai.fight(&b, 1, Keys::empty()))
        };

        let cur_before = b.hands[1].cursor;
        let ticks = [layers[0].input(p0, &b), layers[1].input(p1, &b)];
        assert!(step_ticks(&mut b, ticks));
        for l in &mut layers {
            l.follow(&b);
        }

        if p0_used_invis && !b.is_dimmed() && sim::navi_status(&b, 0).contains(StatusFlags::INVISIBLE) {
            saw_invis_ticks += 1;
            if b.hands[1].cursor > cur_before {
                wasted_during_invis = true;
            }
            if saw_invis_ticks >= 220 {
                break;
            }
        }
    }

    assert!(saw_invis_ticks >= 200, "test setup never observed enough Invisible ticks ({saw_invis_ticks})");
    assert!(
        !wasted_during_invis,
        "AquaCross AI dumped train_arrow_3 into Invisible/empty air just for passive +5% heal! (form={})",
        content.defs.form(b.stats[1].form).key
    );
}
