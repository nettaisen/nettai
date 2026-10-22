use super::*;
use nettai_battle::content::Content;
use nettai_frontend::Player;
use nettai_frontend::game::GraphicsMode;
use nettai_frontend::replay::ReplayPlayer;
use nettai_frontend::testing::{audio, graphics, recorded};
use std::sync::Arc;

/// A viewer of `replay` from `view`'s side, on graphics without a pack and
/// a sound of every cue.
pub(crate) fn viewer(content: &Arc<Content>, replay: &Replay, view: u8) -> Viewer {
    let renderer = graphics(content, 0x7C1F).renderer(GraphicsMode::Original, None);
    Viewer::with(content, replay, Some(view), renderer, None, Some(audio(content))).unwrap()
}

fn go_on(_: Progress) -> ControlFlow<()> {
    ControlFlow::Continue(())
}

/// A video's options: `part` from the viewer's side, the picture `scale`
/// times the frame, the titles the library's own.
pub(crate) fn options(scale: u32, sides: Sides, part: &Part) -> Options {
    Options { scale, view: None, sides, part: part.clone(), titles: Titles::default() }
}

/// What an encoder is given of a video, either platform's (what [`Mp4`]
/// hands it as it comes): the picture, the sound and the info it is
/// started with, then each frame's pixels and each sound track's samples
/// with it.
pub(crate) struct Given {
    pub video: Video,
    pub audio: Option<Audio>,
    pub info: Info,
    pub frames: Vec<(Vec<u8>, Vec<Vec<[f32; 2]>>)>,
}

impl Given {
    /// The pictures in order.
    pub fn pictures(&self) -> Vec<&[u8]> {
        self.frames.iter().map(|(rgb, _)| &rgb[..]).collect()
    }

    /// Sound track `track`'s samples, as one run.
    pub fn track(&self, track: usize) -> Vec<[f32; 2]> {
        self.frames.iter().flat_map(|(_, sound)| sound[track].iter().copied()).collect()
    }
}

impl Frames for Given {
    fn frame(&mut self, rgb: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        self.frames.push((rgb, sound.iter().map(|s| s.to_vec()).collect()));
        Ok(Vec::new())
    }
}

/// What the replay `viewer` plays gives an encoder, as `options` say.
pub(crate) fn given(viewer: &mut Viewer, options: &Options, progress: OnProgress) -> Result<Given, Error> {
    play(viewer, options, progress, |video, audio, info| Ok(Given { video, audio, info: info.clone(), frames: Vec::new() }))
}

/// `part` of `replay` from `view`'s side, as an encoder is given it, the
/// picture `scale` times the frame.
fn given_of(content: &Arc<Content>, replay: &Replay, view: u8, scale: u32, part: &Part) -> Given {
    given(&mut viewer(content, replay, view), &options(scale, Sides::One, part), &mut go_on).unwrap()
}

/// `sound`, a run of the consoles' samples, made 48 kHz as one run.
fn at_48_khz(sound: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut r = Resampler::new(nettai_audio::SAMPLE_RATE);
    let mut made = r.push(sound);
    made.extend(r.finish());
    made
}

/// What a player playing `replay` straight presents and plays, a tick at
/// a time: each tick's picture at `scale` times the frame as RGB bytes,
/// and its samples. (A [`Player`]: nothing of the viewer or its seeking.)
fn played(content: &Arc<Content>, replay: &Replay, view: u8, scale: usize) -> Vec<(Vec<u8>, Vec<[f32; 2]>)> {
    let driver = ReplayPlayer::new(content, replay, Some(view)).unwrap();
    let renderer = graphics(content, 0x7C1F).renderer(GraphicsMode::Original, None);
    let mut p = Player::with(renderer, None, Some(audio(content)), Box::new(driver));
    let (w, h) = (WIDTH * scale, HEIGHT * scale);
    let mut buffer = vec![0u32; w * h];
    let mut out = Vec::new();
    while p.tick(0) {
        p.present(&mut buffer, w, h);
        let rgb = buffer.iter().flat_map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]).collect();
        let mut sound = Vec::new();
        p.take_samples(&mut sound);
        out.push((rgb, sound));
    }
    out
}

/// A replay written whole is, frame for frame, what a player playing it
/// straight presents, one picture a tick at the scale asked for on the
/// tick's exact time base; and its sound is what the player plays, made
/// 48 kHz in one run, every frame's in its place: the picture's length to
/// within a sample. From either console; and the two consoles' videos
/// differ.
#[test]
fn a_replay_is_written_as_it_is_shown_and_heard() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let mut sides = Vec::new();
    for (view, scale) in [(0u8, 2u32), (1, 1)] {
        let given = given_of(&content, &replay, view, scale, &Part::Whole);
        let straight = played(&content, &replay, view, scale as usize);
        assert_eq!(straight.len() as u64, ticks);
        assert_eq!(given.video, Video { width: 240 * scale, height: 160 * scale, frame_time: (4389, 262_144) });
        assert_eq!(given.audio, Some(Audio { sample_rate: 48_000, tracks: 1 }));
        let (pictures, heard) = (given.pictures(), given.track(0));
        assert_eq!(pictures.len() as u64, ticks, "side {view}: a picture a tick");
        for (tick, (shown, (presented, _))) in pictures.iter().zip(&straight).enumerate() {
            assert!(shown == presented, "side {view}: the picture of tick {tick}");
        }
        let made: Vec<[f32; 2]> = straight.iter().flat_map(|(_, sound)| sound.iter().copied()).collect();
        assert!(heard == at_48_khz(&made), "side {view}: {} samples for {}", heard.len(), made.len());
        // The pictures aren't all one, nor the sound silence; and the
        // sound is the picture's length to within a sample.
        assert!(pictures.iter().any(|p| p != &pictures[0]) && heard.iter().any(|s| s[0] != 0.0), "side {view}: nothing to tell by");
        let length = ticks as f64 * 4389.0 / 262_144.0 * 48_000.0;
        assert!((heard.len() as f64 - length).abs() < 2.0, "side {view}: {} samples for {ticks} ticks ({length})", heard.len());
        // Each frame's sound is about its tick's: the first's a filter's
        // half short, the last's with the rest.
        let runs: Vec<usize> = given.frames.iter().map(|(_, sound)| sound[0].len()).collect();
        assert!(runs[1..runs.len() - 1].iter().all(|&n| (802..=805).contains(&n)), "side {view}: {runs:?}");
        assert!((690..802).contains(&runs[0]) && (805..920).contains(&runs[runs.len() - 1]), "side {view}: {runs:?}");
        sides.push(given.frames);
    }
    assert!(sides[0] != sides[1]);
}

/// Both sides side by side: a picture twice as wide, each row the left
/// console's row then the right one's, each half that console's own video
/// frame for frame; and two sound tracks, the first the left console's
/// sound sample for sample and the second the right one's, which differ.
/// (The test's graphics are flat: the two halves differ on a game's packs.)
#[test]
fn both_sides_are_each_sides_video_side_by_side() {
    let content = nettai_match::testing::exe6_content();
    let (replay, _) = recorded(&content, "exe6", 7);
    let part = Part::Ticks(250..400);
    for (left, scale) in [(0u8, 2usize), (1, 1)] {
        let both = given(&mut viewer(&content, &replay, left), &options(scale as u32, Sides::SideBySide, &part), &mut go_on).unwrap();
        assert_eq!((both.video.width, both.video.height), (480 * scale as u32, 160 * scale as u32));
        assert_eq!(both.audio, Some(Audio { sample_rate: 48_000, tracks: 2 }), "left {left}: a picture and two tracks");
        let (own, other) = (given_of(&content, &replay, left, scale as u32, &part), given_of(&content, &replay, left ^ 1, scale as u32, &part));
        let (pictures, own_pictures, other_pictures) = (both.pictures(), own.pictures(), other.pictures());
        assert_eq!(pictures.len(), 150);
        let row = 240 * scale * 3;
        for (tick, ((picture, l), r)) in pictures.iter().zip(&own_pictures).zip(&other_pictures).enumerate() {
            for (y, (l, r)) in l.chunks_exact(row).zip(r.chunks_exact(row)).enumerate() {
                let at = y * row * 2;
                assert!(&picture[at..at + row] == l && &picture[at + row..at + 2 * row] == r, "left {left}: tick {tick}'s row {y}");
            }
        }
        assert!(both.track(0) == own.track(0), "left {left}: the first track isn't the left console's");
        assert!(both.track(1) == other.track(0), "left {left}: the second track isn't the right console's");
        assert!(own.track(0) != other.track(0), "left {left}: the two consoles' sounds are one");
    }
}

/// Both consoles on one screen: a picture the frame's width, each what a
/// viewer shown so presents at the tick, and one sound track, both
/// consoles' as that viewer plays it (titled with both players), which
/// isn't the left console's alone.
#[test]
fn both_on_one_screen_is_the_viewers_both() {
    let content = nettai_match::testing::exe6_content();
    let (replay, _) = recorded(&content, "exe6", 7);
    let part = Part::Ticks(250..400);
    let left = 1u8;
    let titles = Titles { rounds: Vec::new(), sides: ["Lan".into(), "Chaud".into()] };
    let options = Options { titles, ..options(2, Sides::Both, &part) };
    let one = given(&mut viewer(&content, &replay, left), &options, &mut go_on).unwrap();
    assert_eq!((one.video.width, one.video.height), (480, 320));
    assert_eq!(one.audio.map(|a| a.tracks), Some(1), "a picture and one track");
    assert_eq!(one.info.titles, ["Chaud & Lan"]);
    // What a viewer shown so presents and plays, tick by tick.
    let mut v = viewer(&content, &replay, left);
    v.set_both(true);
    v.seek(250);
    let (w, h) = (WIDTH * 2, HEIGHT * 2);
    let mut buffer = vec![0u32; w * h];
    let mut heard = Vec::new();
    for picture in one.pictures() {
        assert!(v.step_forward());
        v.present(&mut buffer, w, h);
        let rgb: Vec<u8> = buffer.iter().flat_map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]).collect();
        assert!(rgb == picture, "the picture of tick {}", v.position());
        v.take_samples(&mut heard);
    }
    assert!(one.track(0) == at_48_khz(&heard), "the track isn't both consoles' sound");
    assert!(one.track(0) != given_of(&content, &replay, left, 2, &part).track(0), "both consoles' sound is the left console's alone");
}

/// A video's chapters: a round each, from the frame its picture begins on
/// (the tick before its first: the frame showing the position the viewer
/// calls the round's first) to the next one's; a part's first frame its
/// round's; a round whose picture begins on a part's last frame none. Titled
/// as given, "Round N" where nothing is.
#[test]
fn a_chapter_is_a_round_from_its_first_picture() {
    let rounds = [0, 150, 300];
    let titles = Titles { rounds: vec!["ラウンド 1".into(), " ".into()], sides: Default::default() };
    let of = |ticks: Range<u32>| -> Vec<(u64, u64, String)> { chapters(&rounds, &ticks, &titles).into_iter().map(|c| (c.start, c.frames, c.title)).collect() };
    let chapter = |start, frames, title: &str| (start, frames, title.to_string());
    assert_eq!(of(0..400), [chapter(0, 149, "ラウンド 1"), chapter(149, 150, "Round 2"), chapter(299, 101, "Round 3")]);
    // A round alone: its part's last frame shows the next round's first picture, and starts no chapter.
    assert_eq!(of(150..300), [chapter(0, 150, "Round 2")]);
    assert_eq!(of(0..150), [chapter(0, 150, "ラウンド 1")]);
    // From the frame before a round's first picture, and from that picture.
    assert_eq!(of(148..152), [chapter(0, 1, "ラウンド 1"), chapter(1, 3, "Round 2")]);
    assert_eq!(of(149..152), [chapter(0, 3, "Round 2")]);
    assert_eq!(of(399..400), [chapter(0, 1, "Round 3")]);
    assert_eq!(of(298..300), [chapter(0, 2, "Round 2")]);
    // On a replay: each chapter starts on the frame that shows the round's
    // first position, the one before it showing the round before's.
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let v = viewer(&content, &replay, 0);
    let made = chapters(v.rounds(), &(0..ticks as u32), &Titles::default());
    assert_eq!(made.len(), 2);
    for (n, c) in made.iter().enumerate() {
        // (Frame f is the picture of position f + 1.)
        let shows = c.start as u32 + 1;
        assert_eq!((v.round_at(shows), c.title.as_str()), (n as u32 + 1, ["Round 1", "Round 2"][n]));
        assert!(n == 0 || v.round_at(shows - 1) == n as u32, "chapter {n} starts late");
    }
}

/// The video says its tracks' titles and its chapters: each track the
/// player of its console as the titles give it (else the replay's name,
/// else "Player N"), the first the default; the chapters as `chapters`
/// makes them, titled in the host's words.
#[test]
fn the_video_says_its_tracks_and_chapters() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let titles = Titles { rounds: vec!["ラウンド 1".into(), "ラウンド 2".into()], sides: ["Lan".into(), String::new()] };
    let second = replay.round_starts()[1];
    let part = Part::Ticks(second - 20..second + 10);
    let options = Options { titles: titles.clone(), ..options(1, Sides::SideBySide, &part) };
    let given = given(&mut viewer(&content, &replay, 1), &options, &mut go_on).unwrap();
    // (The left console is side 1's, whose player the replay names nobody.)
    let unnamed = if replay.info.names[1].trim().is_empty() { "Player 2".to_string() } else { replay.info.names[1].trim().to_string() };
    assert_eq!(given.info.titles, [unnamed, "Lan".to_string()]);
    let chapter = |start, frames, title: &str| Chapter { start, frames, title: title.into() };
    assert_eq!(given.info.chapters, [chapter(0, 19, "ラウンド 1"), chapter(19, 11, "ラウンド 2")]);
    assert!(ticks > second as u64 + 10);
}

/// The same replay gives the encoder the same frames and samples, from a
/// viewer made anew each time; and a viewer that has been about the replay
/// first (sought, played, shown from the other side) gives them too.
#[test]
fn the_same_replay_gives_the_same_frames() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let first = given_of(&content, &replay, 0, 1, &Part::Whole).frames;
    assert!(first == given_of(&content, &replay, 0, 1, &Part::Whole).frames, "another run, other frames");
    let mut v = viewer(&content, &replay, 1);
    v.seek(ticks as u32 / 2);
    for _ in 0..30 {
        v.step_forward();
    }
    v.set_view(0);
    v.set_speed(Speed::Double);
    let after = given(&mut v, &options(1, Sides::One, &Part::Whole), &mut go_on).unwrap().frames;
    assert!(first == after, "a viewer that had been about the replay gives other frames");
    assert!(v.paused() && v.at_end() && v.speed() == Speed::Normal);
}

/// A part is the whole video's frames for its ticks, and its sound the
/// consoles' for those ticks made 48 kHz: a round alone, and a range of
/// ticks from the middle of one into the next, start where playing
/// straight there gets (the sound in mid-note included) and stop at their
/// last tick.
#[test]
fn a_part_is_the_whole_videos_for_its_ticks() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let (length, second) = (ticks as u32, replay.round_starts()[1]);
    let straight = played(&content, &replay, 0, 1);
    // (A tick in the first round's later half that the sound sounds
    // through: a part from it starts in mid-note.)
    let sounds = |tick: usize| straight[tick].1.iter().any(|s| s[0] != 0.0);
    let sounding = (second as usize / 2..second as usize - 60).find(|&t| sounds(t - 1) && sounds(t) && sounds(t + 1)).expect("a tick in mid-sound") as u32;
    for (part, range) in [
        (Part::Round(1), 0..second),
        (Part::Round(2), second..length),
        (Part::Ticks(second - 100..second + 150), second - 100..second + 150),
        (Part::Ticks(sounding..sounding + 50), sounding..sounding + 50),
        (Part::Ticks(length - 1..length), length - 1..length),
    ] {
        assert_eq!(part.ticks(&replay.round_starts(), length).unwrap(), range, "{part:?}");
        let given = given_of(&content, &replay, 0, 1, &part);
        let of_part = &straight[range.start as usize..range.end as usize];
        assert_eq!(given.frames.len(), of_part.len(), "{part:?}");
        assert!(given.pictures().iter().zip(of_part).all(|(shown, (presented, _))| shown == presented), "{part:?}: its pictures");
        let made: Vec<[f32; 2]> = of_part.iter().flat_map(|(_, sound)| sound.iter().copied()).collect();
        assert!(given.track(0) == at_48_khz(&made), "{part:?}: its sound");
    }
}

/// The progress is told before the first frame and after each, with the
/// round being written; an answer to stop cancels the video where it is.
/// A viewer without sound gives the picture alone.
#[test]
fn progress_is_told_and_may_cancel() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let (length, second) = (ticks as u32, replay.round_starts()[1]);
    let mut told = Vec::new();
    let mut tell = |p: Progress| {
        told.push(p);
        ControlFlow::Continue(())
    };
    given(&mut viewer(&content, &replay, 0), &options(1, Sides::One, &Part::Whole), &mut tell).unwrap();
    assert_eq!(told.len() as u32, length + 1);
    assert_eq!(told[0], Progress { written: 0, of: length, round: 1, rounds: 2 });
    assert!(told.iter().enumerate().all(|(n, p)| p.written == n as u32 && p.of == length && p.rounds == 2));
    // (The frame a tick leaves is of the tick's round.)
    assert_eq!((told[second as usize].round, told[second as usize + 1].round, told[length as usize].round), (1, 2, 2));
    assert_eq!((told[0].fraction(), told[length as usize].fraction()), (0.0, 1.0));
    // A part far in: told of while the viewer gets there, nothing written.
    let mut told = Vec::new();
    let mut tell = |p: Progress| {
        told.push(p);
        ControlFlow::Continue(())
    };
    given(&mut viewer(&content, &replay, 0), &options(1, Sides::One, &Part::Ticks(length - 10..length)), &mut tell).unwrap();
    let before = told.iter().filter(|p| p.written == 0).count();
    assert!(before > 1 && told.len() == before + 10, "{before} of {}", told.len());
    assert!(told.iter().all(|p| p.of == 10 && p.round == 2));
    // Canceled after its twentieth frame.
    let mut frames = 0;
    let mut stop_at_twenty = |p: Progress| {
        frames = p.written;
        if p.written == 20 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    };
    let canceled = given(&mut viewer(&content, &replay, 0), &options(1, Sides::One, &Part::Whole), &mut stop_at_twenty);
    assert!(matches!(canceled, Err(Error::Canceled)) && frames == 20);
    // Without sound.
    let renderer = graphics(&content, 0x7C1F).renderer(GraphicsMode::Original, None);
    let mut silent = Viewer::with(&content, &replay, Some(0), renderer, None, None).unwrap();
    let alone = given(&mut silent, &options(1, Sides::One, &Part::Round(1)), &mut go_on).unwrap();
    assert_eq!((alone.audio, alone.frames.len() as u32), (None, second));
    assert!(alone.frames.iter().all(|(_, sound)| sound.is_empty()) && alone.info.titles.is_empty());
}

/// What can't be made is refused before the encoder is started, saying
/// what.
#[test]
fn what_cant_be_made_is_refused() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let length = ticks as u32;
    let mut v = viewer(&content, &replay, 0);
    let mut refused = |scale, part: Part| {
        let mut started = false;
        let e = play(&mut v, &options(scale, Sides::One, &part), &mut go_on, |video, audio, info| {
            started = true;
            Ok(Given { video, audio, info: info.clone(), frames: Vec::new() })
        })
        .err()
        .unwrap();
        assert!(!started, "{part:?}: the encoder was started");
        e.to_string()
    };
    assert_eq!(refused(0, Part::Whole), "a picture 0 times the frame's size (1 to 16)");
    assert_eq!(refused(17, Part::Whole), "a picture 17 times the frame's size (1 to 16)");
    assert_eq!(refused(1, Part::Round(3)), "no round 3 of a replay of 2 rounds");
    assert_eq!(refused(1, Part::Round(0)), "no round 0 of a replay of 2 rounds");
    assert_eq!(refused(1, Part::Ticks(5..5)), format!("no ticks 5 to 5 of a replay of {length} ticks"));
    assert_eq!(refused(1, Part::Ticks(0..length + 1)), format!("no ticks 0 to {} of a replay of {length} ticks", length + 1));
    // A replay of another game isn't opened.
    let five = nettai_match::testing::exe5_content();
    let renderer = graphics(&five, 0x7C1F).renderer(GraphicsMode::Original, None);
    let other = Viewer::with(&five, &replay, None, renderer, None, None).map_err(Error::Replay).err().unwrap();
    assert_eq!(other.to_string(), "the replay isn't played: the replay is of exe6, not exe5");
}
