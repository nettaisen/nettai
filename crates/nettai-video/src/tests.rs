use super::*;
use crate::nut::reading;
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

/// `part` of `replay` from `view`'s side as a NUT stream, the picture
/// `scale` times the frame.
fn exported(content: &Arc<Content>, replay: &Replay, view: u8, scale: u32, part: &Part) -> Vec<u8> {
    write(&mut viewer(content, replay, view), &options(scale, Sides::One, part), Vec::new(), &mut go_on).unwrap()
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

/// A stream's pictures in order, and its (first track's) sound as one run
/// of samples.
fn pictures_and_sound(file: &reading::File) -> (Vec<&[u8]>, Vec<[f32; 2]>) {
    let pictures = file.frames.iter().filter(|f| f.stream == 0).map(|f| &f.data[..]).collect();
    (pictures, track(file, 1))
}

/// The samples of a stream's stream `stream` (a sound track's), as one run.
fn track(file: &reading::File, stream: usize) -> Vec<[f32; 2]> {
    file.frames
        .iter()
        .filter(|f| f.stream == stream)
        .flat_map(|f| f.data.chunks_exact(8))
        .map(|s| [f32::from_le_bytes(s[..4].try_into().unwrap()), f32::from_le_bytes(s[4..].try_into().unwrap())])
        .collect()
}

/// A replay written whole is, frame for frame and sample for sample, what
/// a player playing it straight presents and plays: one picture a tick at
/// the scale asked for, on the tick's exact time base, and each tick's
/// sound beside it, in a stream that reads back by NUT's syntax. From
/// either console; and the two consoles' videos differ.
#[test]
fn a_replay_is_written_as_it_is_shown_and_heard() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let mut sides = Vec::new();
    for (view, scale) in [(0u8, 2u32), (1, 1)] {
        let bytes = exported(&content, &replay, view, scale, &Part::Whole);
        let file = reading::read(&bytes).unwrap_or_else(|e| panic!("side {view}: {e}"));
        let straight = played(&content, &replay, view, scale as usize);
        assert_eq!(straight.len() as u64, ticks);
        let (picture, sound) = (&file.streams[0], &file.streams[1]);
        assert_eq!((picture.size, picture.time_base, &picture.fourcc[..]), ((240 * scale as u64, 160 * scale as u64), (4389, 262_144), &b"RGB\x18"[..]));
        assert_eq!((sound.sample_rate, sound.channels, sound.time_base, &sound.fourcc[..]), ((32_768, 1), 2, (1, 32_768), &b"PFD\x20"[..]));
        let (pictures, heard) = pictures_and_sound(&file);
        assert_eq!(pictures.len() as u64, ticks, "side {view}: a picture a tick");
        for (tick, (shown, (presented, _))) in pictures.iter().zip(&straight).enumerate() {
            assert!(shown == presented, "side {view}: the picture of tick {tick}");
        }
        let made: Vec<[f32; 2]> = straight.iter().flat_map(|(_, sound)| sound.iter().copied()).collect();
        assert!(heard == made, "side {view}: {} samples for {}", heard.len(), made.len());
        // The pictures aren't all one, nor the sound silence; and the
        // sound is the picture's length to within a sample a tick.
        assert!(pictures.iter().any(|p| p != &pictures[0]) && heard.iter().any(|s| s[0] != 0.0), "side {view}: nothing to tell by");
        assert!((heard.len() as f64 - ticks as f64 * 4389.0 / 8.0).abs() < 2.0, "side {view}: {} samples for {ticks} ticks", heard.len());
        // Each picture's pts is its number; each sound frame's, the
        // samples before it.
        let mut samples = 0;
        for (n, f) in file.frames.iter().filter(|f| f.stream == 0).enumerate() {
            assert_eq!(f.pts, n as u64);
        }
        for f in file.frames.iter().filter(|f| f.stream == 1) {
            assert_eq!(f.pts, samples);
            samples += f.data.len() as u64 / 8;
        }
        sides.push(bytes);
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
        let both = write(&mut viewer(&content, &replay, left), &options(scale as u32, Sides::Both, &part), Vec::new(), &mut go_on).unwrap();
        let both = reading::read(&both).unwrap();
        assert_eq!(both.streams[0].size, (480 * scale as u64, 160 * scale as u64));
        assert_eq!((both.streams.len(), both.streams[1].class, both.streams[2].class), (3, 1, 1), "left {left}: a picture and two tracks");
        let alone = |view| reading::read(&exported(&content, &replay, view, scale as u32, &part)).unwrap();
        let (own, other) = (alone(left), alone(left ^ 1));
        let ((pictures, sound), (own_pictures, own_sound), (other_pictures, other_sound)) = (pictures_and_sound(&both), pictures_and_sound(&own), pictures_and_sound(&other));
        assert_eq!(pictures.len(), 150);
        let row = 240 * scale * 3;
        for (tick, ((picture, l), r)) in pictures.iter().zip(&own_pictures).zip(&other_pictures).enumerate() {
            for (y, (l, r)) in l.chunks_exact(row).zip(r.chunks_exact(row)).enumerate() {
                let at = y * row * 2;
                assert!(&picture[at..at + row] == l && &picture[at + row..at + 2 * row] == r, "left {left}: tick {tick}'s row {y}");
            }
        }
        assert!(sound == own_sound, "left {left}: the first track isn't the left console's");
        assert!(track(&both, 2) == other_sound, "left {left}: the second track isn't the right console's");
        assert!(own_sound != other_sound, "left {left}: the two consoles' sounds are one");
    }
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

/// The stream says its tracks' titles and its chapters: each track the
/// player of its console as the titles give it (else the replay's name,
/// else "Player N"), the first the default; the chapters as `chapters`
/// makes them, titled in the host's words.
#[test]
fn the_stream_says_its_tracks_and_chapters() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let titles = Titles { rounds: vec!["ラウンド 1".into(), "ラウンド 2".into()], sides: ["Lan".into(), String::new()] };
    let second = replay.round_starts()[1];
    let part = Part::Ticks(second - 20..second + 10);
    let options = Options { titles: titles.clone(), ..options(1, Sides::Both, &part) };
    let file = reading::read(&write(&mut viewer(&content, &replay, 1), &options, Vec::new(), &mut go_on).unwrap()).unwrap();
    let named = |stream: u64| file.info.iter().find(|i| i.stream_id_plus1 == stream + 1).map(|i| i.fields.clone()).unwrap();
    let field = |name: &str, value: &str| (name.to_string(), value.to_string());
    // (The left console is side 1's, whose player the replay names nobody.)
    let unnamed = if replay.info.names[1].trim().is_empty() { "Player 2".to_string() } else { replay.info.names[1].trim().to_string() };
    assert_eq!(named(1), [field("Title", &unnamed), field("handler_name", &unnamed), field("Disposition", "default")]);
    assert_eq!(named(2), [field("Title", "Lan"), field("handler_name", "Lan")]);
    let chapters: Vec<(i64, u64, u64, String)> = file.info.iter().filter(|i| i.chapter_id != 0).map(|i| (i.chapter_id, i.start / 2, i.length, i.fields[0].1.clone())).collect();
    assert_eq!(chapters, [(1, 0, 19, "ラウンド 1".to_string()), (2, 19, 11, "ラウンド 2".to_string())]);
    assert!(ticks > second as u64 + 10);
}

/// The same replay gives the same bytes, from a viewer made anew each
/// time; and a viewer that has been about the replay first (sought,
/// played, shown from the other side) writes them too.
#[test]
fn the_same_replay_gives_the_same_bytes() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let first = exported(&content, &replay, 0, 1, &Part::Whole);
    assert!(first == exported(&content, &replay, 0, 1, &Part::Whole), "another run, other bytes");
    let mut v = viewer(&content, &replay, 1);
    v.seek(ticks as u32 / 2);
    for _ in 0..30 {
        v.step_forward();
    }
    v.set_view(0);
    v.set_speed(Speed::Double);
    let after = write(&mut v, &options(1, Sides::One, &Part::Whole), Vec::new(), &mut go_on).unwrap();
    assert!(first == after, "a viewer that had been about the replay writes other bytes");
    assert!(v.paused() && v.at_end() && v.speed() == Speed::Normal);
}

/// A part is the whole video's frames and samples for its ticks: a round
/// alone, and a range of ticks from the middle of one into the next,
/// start where playing straight there gets (the sound in mid-note
/// included) and stop at their last tick; their pts count from the part's
/// start.
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
        let file = reading::read(&exported(&content, &replay, 0, 1, &part)).unwrap_or_else(|e| panic!("{part:?}: {e}"));
        let (pictures, heard) = pictures_and_sound(&file);
        let of_part = &straight[range.start as usize..range.end as usize];
        assert_eq!(pictures.len(), of_part.len(), "{part:?}");
        assert!(pictures.iter().zip(of_part).all(|(shown, (presented, _))| shown == presented), "{part:?}: its pictures");
        let made: Vec<[f32; 2]> = of_part.iter().flat_map(|(_, sound)| sound.iter().copied()).collect();
        assert!(heard == made, "{part:?}: its sound");
        assert_eq!(file.frames.iter().find(|f| f.stream == 0).map(|f| f.pts), Some(0), "{part:?}");
        assert_eq!(file.frames.iter().find(|f| f.stream == 1).map(|f| f.pts), Some(0), "{part:?}");
    }
}

/// The progress is told before the first frame and after each, with the
/// round being written; an answer to stop cancels the video where it is.
/// A viewer without sound writes the picture's stream alone.
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
    write(&mut viewer(&content, &replay, 0), &options(1, Sides::One, &Part::Whole), Vec::new(), &mut tell).unwrap();
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
    write(&mut viewer(&content, &replay, 0), &options(1, Sides::One, &Part::Ticks(length - 10..length)), Vec::new(), &mut tell).unwrap();
    let before = told.iter().filter(|p| p.written == 0).count();
    assert!(before > 1 && told.len() == before + 10, "{before} of {}", told.len());
    assert!(told.iter().all(|p| p.of == 10 && p.round == 2));
    // Canceled after its twentieth frame.
    let mut frames = 0;
    let mut stop_at_twenty = |p: Progress| {
        frames = p.written;
        if p.written == 20 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    };
    let canceled = write(&mut viewer(&content, &replay, 0), &options(1, Sides::One, &Part::Whole), Vec::new(), &mut stop_at_twenty);
    assert!(matches!(canceled, Err(Error::Canceled)) && frames == 20);
    // Without sound.
    let renderer = graphics(&content, 0x7C1F).renderer(GraphicsMode::Original, None);
    let mut silent = Viewer::with(&content, &replay, Some(0), renderer, None, None).unwrap();
    let file = reading::read(&write(&mut silent, &options(1, Sides::One, &Part::Round(1)), Vec::new(), &mut go_on).unwrap()).unwrap();
    assert_eq!((file.streams.len(), file.frames.len() as u32), (1, second));
}

/// What can't be made is refused before anything is written, saying what.
#[test]
fn what_cant_be_made_is_refused() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let length = ticks as u32;
    let mut v = viewer(&content, &replay, 0);
    let mut refused = |scale, part: Part| {
        let mut out = Vec::new();
        let e = write(&mut v, &options(scale, Sides::One, &part), &mut out, &mut go_on).unwrap_err();
        assert!(out.is_empty(), "{part:?}: something was written");
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

