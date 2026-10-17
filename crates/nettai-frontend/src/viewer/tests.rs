use super::*;
use crate::Player;
use crate::game::GraphicsMode;
use crate::player::FRAME_RATE;
use crate::testing::{audio, graphics, recorded};

/// What playing a replay straight through shows and makes, tick by tick,
/// from one side: after `t` ticks (0: before any), the battle's digest,
/// the picture, the HUD's state and the sound the tick made. It is played
/// by a [`Player`], which has no seeking in it.
struct Straight {
    digests: Vec<u64>,
    pictures: Vec<Vec<u16>>,
    huds: Vec<String>,
    sounds: Vec<Vec<[f32; 2]>>,
    result: Option<BattleResult>,
}

fn straight(content: &Arc<Content>, replay: &Replay, view: u8) -> Straight {
    let driver = ReplayPlayer::new(content, replay, Some(view)).unwrap();
    let renderer = graphics(content, 0x001F).renderer(GraphicsMode::Original, None);
    let mut p = Player::with(renderer, None, Some(audio(content)), Box::new(driver));
    let mut s = Straight { digests: Vec::new(), pictures: Vec::new(), huds: Vec::new(), sounds: Vec::new(), result: None };
    loop {
        let mut sound = Vec::new();
        p.take_samples(&mut sound);
        s.digests.push(p.battle().digest());
        s.pictures.push(p.frame().pixels);
        s.huds.push(format!("{:?}", p.renderer().hud_state));
        s.sounds.push(sound);
        if !p.tick(0) {
            break;
        }
    }
    assert_eq!(p.diverged(), None);
    s.result = p.result();
    s
}

fn viewer(content: &Arc<Content>, replay: &Replay, view: u8) -> Viewer {
    let renderer = graphics(content, 0x001F).renderer(GraphicsMode::Original, None);
    Viewer::with(content, replay, Some(view), renderer, None, Some(audio(content))).unwrap()
}

/// That `v` is as playing straight to where it is leaves a player: the
/// battle, the picture and the HUD's state.
#[track_caller]
fn same(v: &mut Viewer, s: &Straight, at: u32, what: &str) {
    assert_eq!(v.position(), at, "{what}");
    let t = at as usize;
    assert_eq!(v.battle().digest(), s.digests[t], "{what}: the battle at tick {t}");
    assert!(v.frame().pixels == s.pictures[t], "{what}: the picture at tick {t}");
    assert_eq!(format!("{:?}", v.renderer().hud_state), s.huds[t], "{what}: the HUD at tick {t}");
}

/// Play `v` on `ticks` ticks (to the end at most), each as straight
/// playback has it, its sound sample for sample.
#[track_caller]
fn follow(v: &mut Viewer, s: &Straight, ticks: u32, what: &str) {
    let mut sound = Vec::new();
    v.take_samples(&mut sound);
    assert!(sound.is_empty(), "{what}: sound left over from before");
    for _ in 0..ticks {
        let before = v.position();
        if !v.step_forward() {
            assert!(v.at_end(), "{what}: stopped at tick {before}");
            break;
        }
        same(v, s, before + 1, what);
        sound.clear();
        v.take_samples(&mut sound);
        assert!(sound == s.sounds[before as usize + 1], "{what}: the sound of tick {before}");
    }
}

/// A seek lands in exactly the state playing straight there reaches, and
/// goes on from it as straight playback does: forward past what was
/// reached, backward, across a round's end both ways, onto a round's first
/// tick, to the end and back from it, and onto and beside a kept state.
/// The battle's digest, the picture, the HUD's state and every sample of
/// the sound that follows are straight playback's, from either console.
/// Both games.
#[test]
fn a_seek_lands_where_playing_straight_gets() {
    for (content, game, views) in
        [(nettai_match::testing::exe6_content(), "exe6", &[0u8, 1][..]), (nettai_match::testing::exe5_content(), "exe5", &[1u8][..])]
    {
        let (replay, ticks) = recorded(&content, game, 7);
        let length = ticks as u32;
        let second = replay.round_starts()[1];
        assert!(second > KEYFRAME_EVERY + 60 && length > second + KEYFRAME_EVERY, "{game}: rounds of {second} and {} ticks", length - second);
        for &view in views {
            let s = straight(&content, &replay, view);
            assert_eq!(s.digests.len() as u32, length + 1, "{game}");
            assert!(s.sounds.iter().flatten().any(|x| x[0] != 0.0), "{game}: the set makes no sound to tell by");
            let mut v = viewer(&content, &replay, view);
            same(&mut v, &s, 0, "at the start");
            let seeks = [
                (length / 3, "forward, past what was reached"),
                (7, "backward"),
                (second + 55, "forward, across a round's end"),
                (second - 3, "backward, across a round's end"),
                (second, "onto a round's first tick"),
                (second + 1, "a tick into a round"),
                (length, "to the end"),
                (length - 1, "back from the end"),
                (0, "to the start"),
                (KEYFRAME_EVERY + 1, "past a kept state"),
                (KEYFRAME_EVERY, "onto a kept state"),
                (KEYFRAME_EVERY - 1, "before a kept state"),
                (length + 500, "past the end"),
            ];
            for (to, what) in seeks {
                let what = format!("{game} side {view}, a seek {what}");
                v.seek(to);
                same(&mut v, &s, to.min(length), &what);
                follow(&mut v, &s, 40, &what);
            }
            assert_eq!(v.diverged(), None, "{game}");
            // A frame back, again and again, across a kept state and a
            // round's start; then a frame on.
            v.seek(second + 2);
            for back in 1..=5 {
                v.step_back();
                same(&mut v, &s, second + 2 - back, "a frame back");
            }
            follow(&mut v, &s, 8, "frames on after frames back");
            // The states kept: tick 0's, every KEYFRAME_EVERYth, each
            // round's first.
            let mut kept: Vec<u32> = (0..=length).filter(|t| t % KEYFRAME_EVERY == 0 || *t == second).collect();
            kept.dedup();
            assert_eq!(v.keyframes.iter().map(|k| k.tick).collect::<Vec<_>>(), kept, "{game}");
            assert_eq!((v.indexed(), v.index(1)), (length, true), "{game}");
            // Frames back through more than a whole stretch between two kept
            // states: each is straight playback's. The seek there and the
            // frame back into each earlier stretch play the stretch and
            // leave a state every NEAR_EVERYth tick of it; every other
            // frame back goes on from the nearest of those, fewer than
            // NEAR_EVERY ticks before. Those states are of one stretch at
            // a time.
            let from = length / KEYFRAME_EVERY * KEYFRAME_EVERY - 2;
            let frames = KEYFRAME_EVERY + 40;
            v.seek(from);
            let (mut long, mut all) = (0, 0);
            for back in 1..=frames {
                let before = v.sought();
                v.step_back();
                let played = (v.sought() - before) as u32;
                let to = from - back;
                same(&mut v, &s, to, "a frame back");
                assert!(played < KEYFRAME_EVERY, "{game}: a frame back to tick {to} played {played} ticks");
                long += (played >= NEAR_EVERY) as u32;
                all += played;
                let one_stretch = v.near.iter().all(|n| n.tick / KEYFRAME_EVERY == to / KEYFRAME_EVERY && n.tick % NEAR_EVERY == 0);
                assert!(one_stretch && v.near.len() as u32 <= KEYFRAME_EVERY / NEAR_EVERY, "{game}: at tick {to}");
            }
            // (Two stretches were stepped back into: at most two long ones.)
            assert!(long <= 2 && all < 2 * KEYFRAME_EVERY + frames * NEAR_EVERY, "{game}: {long} long frames back, {all} ticks played for {frames}");
            follow(&mut v, &s, 20, "frames on after many frames back");
        }
    }
}

/// Either console is shown, and the view changes while the replay plays:
/// from the change on, the picture, the HUD's state and the sound are
/// those of a viewer shown from that side all along, and the battle is
/// the same one. At the end each console is told its own result.
#[test]
fn the_view_changes_to_the_other_consoles_exactly() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let length = ticks as u32;
    let sides = [straight(&content, &replay, 0), straight(&content, &replay, 1)];
    assert!(sides[0].sounds != sides[1].sounds, "the two consoles hear alike: nothing to tell a change by");
    assert!(sides[0].huds != sides[1].huds, "the two consoles' HUDs go alike");
    let mut v = viewer(&content, &replay, 0);
    assert_eq!(v.view(), 0);
    follow(&mut v, &sides[0], 300, "side 0 from the start");
    let mut view = 0;
    for at in [300, 340, length / 2, length - 100] {
        v.seek(at);
        view ^= 1;
        assert!(v.set_view(view) && v.view() == view);
        let what = format!("changed to side {view} at tick {at}");
        same(&mut v, &sides[view as usize], at, &what);
        follow(&mut v, &sides[view as usize], 40, &what);
        // A seek after the change is that side's too.
        v.seek(at - 20);
        follow(&mut v, &sides[view as usize], 30, &format!("{what}, then back"));
    }
    assert!(!v.set_view(2) && v.view() == view);
    // The end, and each console's result there.
    v.seek(length);
    assert!(v.at_end() && !v.step_forward() && v.stopped().is_none());
    for side in [0, 1, 0] {
        v.set_view(side);
        assert_eq!(v.result(), sides[side as usize].result);
        same(&mut v, &sides[side as usize], length, "at the end");
    }
    assert_eq!((sides[0].result, sides[1].result), (Some(BattleResult::Won), Some(BattleResult::Lost)));
    // Back from the end there is no result, and the replay plays again.
    v.seek(length - 10);
    assert_eq!(v.result(), None);
    follow(&mut v, &sides[0], 10, "the last ticks again");
    assert_eq!(v.result(), Some(BattleResult::Won));
}

/// The other console's picture, presented beside the one shown, is what a
/// viewer shown from that side presents at the same tick; and presenting it
/// changes nothing of the shown console's picture, its HUD or what follows.
/// (The test's graphics are flat and the same both ways, so the two
/// consoles' pictures are told apart by their HUDs' states here; on a
/// game's packs, by the pictures: docs/frontend.md §8, "Video".)
#[test]
fn the_other_console_is_presented_as_its_own_viewer_presents_it() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let length = ticks as u32;
    let s = straight(&content, &replay, 0);
    let (mut v, mut other) = (viewer(&content, &replay, 0), viewer(&content, &replay, 1));
    let (w, h) = (nettai_render::compose::WIDTH * 2, nettai_render::compose::HEIGHT * 2);
    let (mut inset, mut own) = (vec![0u32; w * h], vec![0u32; w * h]);
    let mut huds_differ = false;
    for at in [0, 1, 120, 300, length / 2, length - 30, length] {
        v.seek(at);
        other.seek(at);
        v.present_view(1, &mut inset, w, h);
        other.present(&mut own, w, h);
        assert!(inset == own, "side 1's picture beside side 0's at tick {at}");
        huds_differ |= format!("{:?}", v.renderer().hud_state) != format!("{:?}", other.renderer().hud_state);
        same(&mut v, &s, at, &format!("side 0 after side 1 was presented at tick {at}"));
        follow(&mut v, &s, 20, &format!("side 0 on from tick {at}"));
    }
    assert!(huds_differ, "the two consoles' HUDs are alike: nothing was told apart");
}

/// Each console's sound is had at once, sample for sample as straight
/// playback from that side makes it: the shown one's by `take_samples` or
/// `take_samples_of` alike, the other's by `take_samples_of`, from the
/// start and after a seek; and the two differ.
#[test]
fn each_consoles_sound_is_its_own_viewers() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let sides = [straight(&content, &replay, 0), straight(&content, &replay, 1)];
    let mut v = viewer(&content, &replay, 0);
    let mut differ = false;
    for (from, by) in [(0, 200), (ticks as u32 / 2, 120)] {
        v.seek(from);
        for t in from..from + by {
            assert!(v.step_forward());
            let (mut shown, mut own, mut other) = (Vec::new(), Vec::new(), Vec::new());
            v.take_samples_of(1, &mut other);
            v.take_samples_of(0, &mut own);
            v.take_samples(&mut shown);
            assert!(own == sides[0].sounds[t as usize + 1] && shown.is_empty(), "side 0's sound of tick {t}");
            assert!(other == sides[1].sounds[t as usize + 1], "side 1's sound of tick {t}");
            differ |= own != other;
        }
    }
    assert!(differ, "the two consoles' sounds are alike: nothing was told apart");
    // (A side a match hasn't gives nothing.)
    let mut none = Vec::new();
    v.take_samples_of(2, &mut none);
    assert!(none.is_empty());
}

/// The transport: the length and the rounds are known as the replay opens;
/// the clock runs the ticks due at each speed and none while paused or at
/// the end; five seconds on and back, a round on and back, a frame on and
/// back, and a time sought, each land on the tick they say.
#[test]
fn the_transport_goes_where_it_says() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let length = ticks as u32;
    let mut v = viewer(&content, &replay, 0);
    let second = replay.round_starts()[1];
    assert_eq!((v.length(), v.rounds(), v.end()), (length, &[0, second][..], End::Set));
    assert_eq!((v.info(), v.info().open_setups), (&replay.info, true));
    assert_eq!(v.duration(), time_of(ticks));
    assert!((v.duration().as_secs_f64() - ticks as f64 / FRAME_RATE).abs() < 1e-3);
    assert_eq!((v.position(), v.time(), v.round(), v.at_end(), v.paused(), v.speed()), (0, Duration::ZERO, 1, false, false, Speed::Normal));
    assert_eq!((v.round_at(second - 1), v.round_at(second), v.round_at(length)), (1, 2, 2));
    // The clock, at each speed.
    let time = |ticks: f64| Duration::from_secs_f64(ticks / FRAME_RATE);
    assert_eq!(v.advance(time(10.4)), 10);
    assert_eq!(v.advance(time(0.7)), 1, "the part of a tick left over counts");
    for (speed, ran) in [(Speed::Half, 5), (Speed::Double, 20), (Speed::Quadruple, 40), (Speed::Normal, 10)] {
        v.set_speed(speed);
        assert_eq!((v.speed(), v.advance(time(10.05))), (speed, ran), "{speed:?}");
    }
    assert_eq!(v.position(), 86);
    v.set_paused(true);
    assert_eq!((v.paused(), v.advance(time(30.0)), v.position()), (true, 0, 86));
    assert!(v.step_forward(), "a frame on while paused");
    v.step_back();
    v.step_back();
    assert_eq!(v.position(), 85);
    // Five seconds are 299 ticks (298.6); the start and the end stop it.
    v.skip(5);
    assert_eq!(v.position(), 384);
    v.skip(-5);
    v.skip(-5);
    assert_eq!(v.position(), 0);
    v.skip(100_000);
    assert_eq!((v.position(), v.at_end()), (length, true));
    v.set_paused(false);
    assert_eq!(v.advance(time(10.0)), 0, "nothing plays at the end");
    // A time sought: the tick nearest to it.
    let middle = length / 2 + 1;
    v.seek_time(time_of(middle as u64));
    assert_eq!((v.position(), v.time()), (middle, time_of(middle as u64)));
    v.seek_time(Duration::from_secs_f64(100.4 / FRAME_RATE));
    assert_eq!(v.position(), 100);
    v.seek_time(Duration::from_secs(100_000));
    assert_eq!(v.position(), length);
    // Rounds: paused, the round's start, and from it the round before's.
    v.set_paused(true);
    v.seek(second + 30);
    v.previous_round();
    assert_eq!((v.position(), v.round()), (second, 2));
    v.previous_round();
    assert_eq!((v.position(), v.round()), (0, 1));
    v.previous_round();
    assert_eq!(v.position(), 0);
    assert!(v.next_round());
    assert_eq!(v.position(), second);
    assert!(!v.next_round(), "the last round has none after it");
    assert_eq!(v.position(), second);
    // Playing, a round's first second counts as its start.
    v.set_paused(false);
    v.seek(second + ROUND_START_GRACE);
    v.previous_round();
    assert_eq!(v.position(), 0);
    v.seek(second + ROUND_START_GRACE + 1);
    v.previous_round();
    assert_eq!(v.position(), second);
    // A replay cut short plays to its cut and has no result; one of
    // another game isn't opened.
    let mut cut = replay.clone();
    cut.ticks.truncate(second as usize + 50);
    cut.end = End::Cut;
    let mut v = viewer(&content, &cut, 0);
    assert_eq!((v.length(), v.rounds(), v.end()), (second + 50, &[0, second][..], End::Cut));
    v.seek(u32::MAX);
    assert!(!v.step_forward());
    assert_eq!((v.position(), v.at_end(), v.result(), v.stopped()), (second + 50, true, None, None));
    let five = nettai_match::testing::exe5_content();
    let renderer = graphics(&five, 0x001F).renderer(GraphicsMode::Original, None);
    let refused = Viewer::with(&five, &replay, None, renderer, None, None).err().unwrap();
    assert_eq!(refused, Refused::OtherGame { replay: "exe6".into(), here: "exe5".into() });
}

/// Indexing ahead plays the replay on in slices without moving the viewer:
/// after each slice it is where and as it was, and what it goes on to play
/// is straight playback's, sound and all. Once the whole replay is indexed
/// a seek anywhere plays fewer ticks than lie between two kept states.
#[test]
fn indexing_ahead_leaves_the_viewer_as_it_was() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let length = ticks as u32;
    let s = straight(&content, &replay, 0);
    let mut v = viewer(&content, &replay, 0);
    follow(&mut v, &s, 100, "before indexing");
    let mut slices = 0;
    // (A slice shorter than the kept states are apart still gets on.)
    while !v.index(97) {
        slices += 1;
        assert!(slices < 1000, "indexing doesn't end (at {})", v.indexed());
        assert_eq!(v.indexed(), (97 * slices).max(100).min(length));
        same(&mut v, &s, 100 + slices - 1, "after a slice");
        follow(&mut v, &s, 1, "between slices");
    }
    assert!(slices > 5 && v.indexed() == length && v.frontier.is_none());
    let at = v.position();
    follow(&mut v, &s, 50, "after indexing");
    assert_eq!(v.position(), at + 50);
    // Every kept state is there, and a seek goes on from the nearest.
    let mut kept: Vec<u32> = (0..=length).filter(|t| t % KEYFRAME_EVERY == 0 || replay.round_starts().contains(t)).collect();
    kept.dedup();
    assert_eq!(v.keyframes.iter().map(|k| k.tick).collect::<Vec<_>>(), kept);
    assert_eq!(v.kept(), kept.len());
    for to in [length - 5, 3, length / 2] {
        v.seek(to);
        same(&mut v, &s, to, "a seek in an indexed replay");
        follow(&mut v, &s, 20, "a seek in an indexed replay");
    }
    assert_eq!(v.diverged(), None);
    // Without sound a viewer seeks the same, and gives none.
    let renderer = graphics(&content, 0x001F).renderer(GraphicsMode::Original, None);
    let mut silent = Viewer::with(&content, &replay, Some(0), renderer, None, None).unwrap();
    silent.seek(length / 2);
    same(&mut silent, &s, length / 2, "a silent viewer");
    assert!(silent.step_forward());
    let mut sound = Vec::new();
    silent.take_samples(&mut sound);
    assert!(sound.is_empty());
}

/// The sound follows the speed as a tape's does: at half speed every
/// sample comes twice, at double and four times every two and four come as
/// their mean, so the host gets as much sound a second at any speed; and a
/// seek leaves none of what was made before it.
#[test]
fn the_sound_follows_the_speed() {
    let content = nettai_match::testing::exe6_content();
    let (replay, _) = recorded(&content, "exe6", 7);
    let s = straight(&content, &replay, 0);
    // (Somewhere the set sounds, for 20 ticks: at half speed they are under
    // the second of sound a viewer keeps for its host.)
    let from = (1..s.sounds.len() - 20).find(|&t| s.sounds[t..t + 20].iter().flatten().filter(|x| x[0] != 0.0).count() > 2000).expect("20 ticks of sound") as u32;
    let made: Vec<[f32; 2]> = s.sounds[from as usize + 1..from as usize + 21].concat();
    for speed in Speed::ALL {
        let mut v = viewer(&content, &replay, 0);
        v.seek(from);
        v.set_speed(speed);
        let mut heard = Vec::new();
        for _ in 0..20 {
            assert!(v.step_forward());
        }
        v.take_samples(&mut heard);
        let want: Vec<[f32; 2]> = match speed {
            Speed::Normal => made.clone(),
            Speed::Half => made.iter().flat_map(|&x| [x, x]).collect(),
            Speed::Double | Speed::Quadruple => {
                let n = if speed == Speed::Double { 2 } else { 4 };
                made.chunks_exact(n).map(|g| [g.iter().map(|x| x[0]).sum::<f32>() / n as f32, g.iter().map(|x| x[1]).sum::<f32>() / n as f32]).collect()
            }
        };
        assert!(heard == want, "{speed:?}: {} samples for {}", heard.len(), want.len());
        // (As much sound a second: 20 ticks' at the speed.)
        let a_second = heard.len() as f64 * speed.factor() / made.len() as f64;
        assert!((a_second - 1.0).abs() < 0.01, "{speed:?}: {a_second}");
        // A seek drops what the host hasn't taken.
        assert!(v.step_forward());
        v.seek(from);
        heard.clear();
        v.take_samples(&mut heard);
        assert!(heard.is_empty(), "{speed:?}");
    }
}

/// A replay of a match whose host answered the pause holds the START a
/// player pressed in a fight, and playing it back reports the press again
/// (a player says so: `Player::take_pause_requests`). A viewer asks nobody:
/// it plays on through the press and keeps no request, playing, seeking
/// over it or indexing through it; its pause is its host's alone.
#[test]
fn a_recorded_start_asks_a_viewer_for_nothing() {
    use nettai_battle::battle::{fight, mode};
    use nettai_battle::input::keys;
    let content = nettai_match::testing::exe6_content();
    // START held through the first three ticks of each round's fight.
    let (mut fought, mut pressed) = (0, Vec::new());
    let (replay, ticks) = crate::testing::recorded_with(&content, "exe6", 7, |b, tick| {
        let fighting = b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING;
        fought = if fighting { fought + 1 } else { 0 };
        if !(1..=3).contains(&fought) {
            return 0;
        }
        pressed.push(tick);
        keys::START
    });
    let length = ticks as u32;
    assert!(pressed.len() >= 6 && pressed.iter().all(|&t| replay.ticks[t as usize].buttons[0] & keys::START != 0), "{pressed:?}");
    // A player playing it straight is asked again, once a press.
    let driver = ReplayPlayer::new(&content, &replay, Some(0)).unwrap();
    let mut p = Player::with(graphics(&content, 0x001F).renderer(GraphicsMode::Original, None), None, None, Box::new(driver));
    let mut asked = Vec::new();
    while p.tick(0) {
        match p.take_pause_requests() {
            [true, false] => asked.push(p.ticks() as u32 - 1),
            other => assert_eq!(other, [false; 2]),
        }
    }
    assert!(asked.len() >= 2 && asked.iter().all(|t| pressed.contains(t)), "asked on {asked:?} of the presses on {pressed:?}");
    // A viewer isn't: played through, sought over, indexed through.
    let s = straight(&content, &replay, 0);
    let mut v = viewer(&content, &replay, 0);
    while v.step_forward() {
        assert_eq!((v.session.pause_requests, v.paused()), ([false; 2], false), "tick {}", v.position());
    }
    same(&mut v, &s, length, "played through a recorded START");
    v.seek(asked[0] - 5);
    follow(&mut v, &s, 20, "through a recorded START");
    v.seek(0);
    v.seek(asked[1] + 10);
    assert_eq!(v.session.pause_requests, [false; 2]);
    same(&mut v, &s, asked[1] + 10, "sought over a recorded START");
    let mut v = viewer(&content, &replay, 0);
    while !v.index(100) {
        assert_eq!(v.session.pause_requests, [false; 2]);
    }
    // The clock plays through one too.
    v.seek(asked[0] - 2);
    let ran = v.advance(Duration::from_secs_f64(10.2 / FRAME_RATE));
    assert_eq!((ran, v.position(), v.paused(), v.session.pause_requests), (10, asked[0] + 8, false, [false; 2]));
}

/// A replay is made ready as any match is: its set (`replay::set_of`) is
/// preloaded, and the viewer the preload gives is the one opened outright,
/// at its start and wherever it is sought to.
#[test]
fn a_preloaded_viewer_is_the_viewer() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let set = crate::replay::set_of(&content, &replay).unwrap();
    let renderer = graphics(&content, 0x001F).renderer(GraphicsMode::Original, None);
    let mut preload = crate::preload::Preload::with(renderer, None, None, &set, 1, (240, 160));
    while preload.step() {}
    assert_eq!(preload.stopped(), None);
    let mut v = preload.viewer(&replay, Some(1)).unwrap();
    let s = straight(&content, &replay, 1);
    assert_eq!((v.view(), v.length(), v.has_sound()), (1, ticks as u32, false));
    same(&mut v, &s, 0, "a preloaded viewer");
    v.seek(ticks as u32 / 2);
    for tick in ticks as u32 / 2..ticks as u32 / 2 + 30 {
        same(&mut v, &s, tick, "a preloaded viewer, sought");
        assert!(v.step_forward());
    }
    // (A replay of another game has no set on this content.)
    let five = nettai_match::testing::exe5_content();
    assert!(matches!(crate::replay::set_of(&five, &replay), Err(Refused::OtherGame { .. })));
}
