//! The recording replay: a golden trace's recorded inputs drive the battle,
//! and each tick is compared with what the original recorded. This is the
//! program's boundary with the compat crates: a recording is the original's
//! own numbers, which each game's compat crate reads, so the recording's
//! game picks the reader here.

use exe6_compat::Compat;
use exe6_compat::trace::{self, Frame, Round};
use nettai_battle::content::Content;
use nettai_battle::Battle;
use nettai_frontend::driver::{Driver, FeedShift, Step};
use std::sync::Arc;

/// Replays one round of a golden trace.
pub struct TracePlayer {
    round: Round,
    /// The content the trace's battle runs on (EXE6's).
    content: Arc<Content>,
    /// The original's numbers for it, which the comparison reads.
    compat: &'static Compat,
    /// Indices of the frames the engine simulates.
    frames: Vec<usize>,
    pos: usize,
    pub round_number: usize,
}

impl TracePlayer {
    pub fn new(round: Round, round_number: usize, content: Arc<Content>) -> TracePlayer {
        let start = round.setup.frame;
        let frames = round
            .frames
            .iter()
            .enumerate()
            .filter(|(_, f)| f.frame >= start)
            .take_while(|(_, f)| f.state[0] == 4 || f.state[0] == 8)
            .map(|(i, _)| i)
            .collect();
        TracePlayer { compat: Compat::exe6_for(&content), round, content, frames, pos: 0, round_number }
    }

    /// Every round of a trace file, on `content`.
    pub fn load(path: &std::path::Path, content: &Arc<Content>) -> std::io::Result<Vec<TracePlayer>> {
        Ok(trace::rounds(path)?.into_iter().enumerate().map(|(i, r)| TracePlayer::new(r, i + 1, content.clone())).collect())
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// The trace frame numbers this round covers.
    pub fn frame_range(&self) -> Option<(u32, u32)> {
        let f = |i: usize| self.round.frames[self.frames[i]].frame;
        (!self.frames.is_empty()).then(|| (f(0), f(self.frames.len() - 1)))
    }

    fn current(&self) -> Option<&Frame> {
        self.pos.checked_sub(1).and_then(|p| self.frames.get(p)).map(|&i| &self.round.frames[i])
    }
}

impl Driver for TracePlayer {
    fn start(&mut self) -> Battle {
        self.pos = 0;
        self.round.start(self.content.clone(), self.compat)
    }

    fn next(&mut self, _b: &Battle, _keys: u16) -> Option<Step> {
        let &i = self.frames.get(self.pos)?;
        let f = &self.round.frames[i];
        // The frame before it too: the link's events are read from it (the
        // session closing).
        let mut window = Vec::with_capacity(2);
        if let Some(&h) = self.pos.checked_sub(1).and_then(|p| self.frames.get(p)) {
            window.push(&self.round.frames[h]);
        }
        let at = window.len();
        window.push(f);
        let (input, events) = self.round.tick_inputs(at, &window);
        self.pos += 1;
        Some(Step { input, events, frame: Some(f.frame) })
    }

    fn check(&self, b: &Battle) -> Vec<String> {
        // (As the replays compare: what the original's link made differ is
        // the original's, `trace::compare_at`.)
        self.current().map(|f| trace::compare_at(b, &self.round, &[f], 0, self.compat)).unwrap_or_default()
    }

    fn frame_range(&self) -> Option<(u32, u32)> {
        TracePlayer::frame_range(self)
    }

    fn feed_shift(&self) -> FeedShift {
        let round = &self.round;
        self.current().map_or_else(FeedShift::default, |f| feed_shift(f.frame, round.local_ok(f.frame), round.first_local_ok(), round.link_delay()))
    }

    fn position(&self) -> String {
        match self.current() {
            Some(f) => format!("round {} frame {}", self.round_number, f.frame),
            None => format!("round {} start", self.round_number),
        }
    }
}

/// Every round of a trace file, on `content`, each as a driver, with its
/// round's number: the recording is of the game its setup line states
/// ([`trace_game`]), which is `content`'s; an EXE6 recording's rounds are
/// [`TracePlayer`]s, an EXE5 one's [`Exe5TracePlayer`]s, an EXE4 one's
/// [`Exe4TracePlayer`]s. A recording that
/// states no game, another game than the content's, or a game no player
/// here replays is refused.
///
/// This is the frontend's boundary with the compat crates: a recording is
/// the original's own numbers, which each game's compat crate reads, so the
/// recording's game picks the reader here (and nowhere else does this crate
/// ask which game a content is).
pub fn trace_rounds(path: &std::path::Path, content: &Arc<Content>) -> Result<Vec<(usize, Box<dyn Driver>)>, String> {
    let game = trace_game(path)?;
    if game != content.game() {
        return Err(format!("an {game} recording, and the content loaded is {}'s", content.game()));
    }
    match game.as_str() {
        exe6_compat::ROOT => {
            let rounds = TracePlayer::load(path, content).map_err(|e| e.to_string())?;
            Ok(rounds.into_iter().map(|r| (r.round_number, Box::new(r) as Box<dyn Driver>)).collect())
        }
        exe5_compat::ROOT => {
            let rounds = Exe5TracePlayer::load(path, content)?;
            Ok(rounds.into_iter().map(|r| (r.round_number, Box::new(r) as Box<dyn Driver>)).collect())
        }
        exe4_compat::ROOT => {
            let rounds = Exe4TracePlayer::load(path, content)?;
            Ok(rounds.into_iter().map(|r| (r.round_number, Box::new(r) as Box<dyn Driver>)).collect())
        }
        other => Err(format!("a recording of {other}: no game this frontend replays recordings of")),
    }
}

/// The game a recording is of: the one its first setup line states
/// (`"game"`). A recording without a setup line, or whose setup states no
/// game, is an error: nothing here takes a recording for a game it doesn't
/// name.
pub fn trace_game(path: &std::path::Path) -> Result<String, String> {
    use std::io::BufRead;
    #[derive(serde::Deserialize)]
    struct Line {
        setup: Stated,
    }
    #[derive(serde::Deserialize)]
    struct Stated {
        game: Option<String>,
    }
    let file = std::io::BufReader::new(std::fs::File::open(path).map_err(|e| e.to_string())?);
    for line in file.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if !line.starts_with("{\"setup\"") {
            continue;
        }
        let stated: Line = serde_json::from_str(&line).map_err(|e| format!("its setup line: {e}"))?;
        return stated.setup.game.ok_or_else(|| exe6_compat::trace::NO_GAME.to_string());
    }
    Err("it has no setup line".into())
}

/// A driver's [`FeedShift`] on frame `frame` from its harness's OK ticks: the
/// recording console's side's OK on the screen open then (`local_ok`), its
/// first of the round (`first_local_ok`), and the cable's delay.
fn feed_shift(frame: u32, ok: Option<u32>, first_ok: Option<u32>, delay: u8) -> FeedShift {
    let d = delay as u32;
    FeedShift {
        held: ok.is_some_and(|ok| (ok..ok + d).contains(&frame)),
        late: if ok.is_some_and(|ok| frame >= ok + d) { d } else { 0 },
        hud_late: if first_ok.is_some_and(|ok| frame >= ok + d) { d } else { 0 },
    }
}

// ---- EXE5's recordings -------------------------------------------------------

/// Replays one round of an EXE5 recording (the chip lab's EXE5 library, read
/// by exe5-compat): its setup on EXE5's content, then each battle frame's
/// buttons.
pub struct Exe5TracePlayer {
    round: exe5_compat::trace::Round,
    content: Arc<Content>,
    compat: &'static exe5_compat::Compat,
    /// Indices of the frames the engine simulates.
    frames: Vec<usize>,
    pos: usize,
    pub round_number: usize,
    /// The traced console's version (its setup line's).
    version: &'static str,
}

impl Exe5TracePlayer {
    /// Every round of an EXE5 recording, on `content`: each round's setup
    /// must be one the content defines (exe5-compat's `Round::needs`).
    pub fn load(path: &std::path::Path, content: &Arc<Content>) -> Result<Vec<Exe5TracePlayer>, String> {
        let compat = exe5_compat::Compat::exe5();
        let mut out = Vec::new();
        for (i, round) in exe5_compat::trace::rounds(path)?.into_iter().enumerate() {
            round.round_setup(content, compat).map_err(|e| format!("round {}: {e}", i + 1))?;
            let d = exe5_compat::trace::decode_setup(&round.setup)?;
            let local = d.battle_state[0x0D] as usize & 1;
            let version = match d.versions[local] {
                exe5_compat::trace::Version::Protoman => "protoman",
                exe5_compat::trace::Version::Colonel => "colonel",
            };
            let start = round.setup.frame;
            let frames = round
                .frames
                .iter()
                .enumerate()
                .filter(|(_, f)| f.frame >= start)
                .take_while(|(_, f)| f.state[0] == 4 || f.state[0] == 8)
                .map(|(i, _)| i)
                .collect();
            out.push(Exe5TracePlayer { round, content: content.clone(), compat, frames, pos: 0, round_number: i + 1, version });
        }
        Ok(out)
    }

    fn current(&self) -> Option<&exe5_compat::trace::Frame> {
        self.pos.checked_sub(1).and_then(|p| self.frames.get(p)).map(|&i| &self.round.frames[i])
    }
}

impl Driver for Exe5TracePlayer {
    fn start(&mut self) -> Battle {
        self.pos = 0;
        // (`load` saw the setup define.)
        self.round.start(self.content.clone(), self.compat).unwrap_or_else(|e| panic!("round {}: {e}", self.round_number))
    }

    fn next(&mut self, _b: &Battle, _keys: u16) -> Option<Step> {
        let &i = self.frames.get(self.pos)?;
        // The frame before too: the link's session closes on the tick the
        // end state moves on.
        let mut window = Vec::with_capacity(2);
        if let Some(&h) = self.pos.checked_sub(1).and_then(|p| self.frames.get(p)) {
            window.push(&self.round.frames[h]);
        }
        let at = window.len();
        window.push(&self.round.frames[i]);
        let (input, events) = self.round.tick_inputs(at, &window);
        self.pos += 1;
        Some(Step { input, events, frame: Some(self.round.frames[i].frame) })
    }

    fn check(&self, b: &Battle) -> Vec<String> {
        self.current().map(|f| exe5_compat::trace::compare_at(b, &self.round, &[f], 0, self.compat)).unwrap_or_default()
    }

    fn console_version(&self) -> Option<&'static str> {
        Some(self.version)
    }

    fn frame_range(&self) -> Option<(u32, u32)> {
        let f = |i: usize| self.round.frames[self.frames[i]].frame;
        (!self.frames.is_empty()).then(|| (f(0), f(self.frames.len() - 1)))
    }

    fn feed_shift(&self) -> FeedShift {
        let round = &self.round;
        self.current().map_or_else(FeedShift::default, |f| feed_shift(f.frame, round.local_ok(f.frame), round.first_local_ok(), round.link_delay()))
    }

    fn position(&self) -> String {
        match self.current() {
            Some(f) => format!("round {} frame {}", self.round_number, f.frame),
            None => format!("round {} start", self.round_number),
        }
    }
}

// ---- EXE4's recordings -------------------------------------------------------

/// Replays one round of an EXE4 recording (the chip lab's EXE4 library, read
/// by exe4-compat): its setup on EXE4's content, then each battle frame's
/// buttons.
pub struct Exe4TracePlayer {
    round: exe4_compat::trace::Round,
    content: Arc<Content>,
    compat: &'static exe4_compat::Compat,
    /// Indices of the frames the engine simulates.
    frames: Vec<usize>,
    pos: usize,
    pub round_number: usize,
    /// The recording console's version (its setup line's).
    version: &'static str,
}

impl Exe4TracePlayer {
    /// Every round of an EXE4 recording, on `content`: each round's setup
    /// must be one the content defines (exe4-compat's `Round::needs`).
    pub fn load(path: &std::path::Path, content: &Arc<Content>) -> Result<Vec<Exe4TracePlayer>, String> {
        let compat = exe4_compat::Compat::exe4();
        let mut out = Vec::new();
        for (i, round) in exe4_compat::trace::rounds(path)?.into_iter().enumerate() {
            round.round_setup(content, compat).map_err(|e| format!("round {}: {e}", i + 1))?;
            let d = exe4_compat::trace::decode_setup(&round.setup)?;
            let version = d.traced_rom().0.name();
            let start = round.setup.frame;
            let frames = round
                .frames
                .iter()
                .enumerate()
                .filter(|(_, f)| f.frame >= start)
                .take_while(|(_, f)| f.state[0] == 4 || f.state[0] == 8)
                .map(|(i, _)| i)
                .collect();
            out.push(Exe4TracePlayer { round, content: content.clone(), compat, frames, pos: 0, round_number: i + 1, version });
        }
        Ok(out)
    }

    fn current(&self) -> Option<&exe4_compat::trace::Frame> {
        self.pos.checked_sub(1).and_then(|p| self.frames.get(p)).map(|&i| &self.round.frames[i])
    }
}

impl Driver for Exe4TracePlayer {
    fn start(&mut self) -> Battle {
        self.pos = 0;
        // (`load` saw the setup define.)
        self.round.start(self.content.clone(), self.compat).unwrap_or_else(|e| panic!("round {}: {e}", self.round_number))
    }

    fn next(&mut self, _b: &Battle, _keys: u16) -> Option<Step> {
        let &i = self.frames.get(self.pos)?;
        // The frame before too: the link's session closes on the tick the
        // end state moves on.
        let mut window = Vec::with_capacity(2);
        if let Some(&h) = self.pos.checked_sub(1).and_then(|p| self.frames.get(p)) {
            window.push(&self.round.frames[h]);
        }
        let at = window.len();
        window.push(&self.round.frames[i]);
        let (input, events) = self.round.tick_inputs(at, &window);
        self.pos += 1;
        Some(Step { input, events, frame: Some(self.round.frames[i].frame) })
    }

    fn check(&self, b: &Battle) -> Vec<String> {
        self.current().map(|f| exe4_compat::trace::compare_at(b, &self.round, &[f], 0, self.compat)).unwrap_or_default()
    }

    fn console_version(&self) -> Option<&'static str> {
        Some(self.version)
    }

    fn frame_range(&self) -> Option<(u32, u32)> {
        let f = |i: usize| self.round.frames[self.frames[i]].frame;
        (!self.frames.is_empty()).then(|| (f(0), f(self.frames.len() - 1)))
    }

    fn feed_shift(&self) -> FeedShift {
        let round = &self.round;
        self.current().map_or_else(FeedShift::default, |f| feed_shift(f.frame, round.local_ok(f.frame), round.first_local_ok(), round.link_delay()))
    }

    fn position(&self) -> String {
        match self.current() {
            Some(f) => format!("round {} frame {}", self.round_number, f.frame),
            None => format!("round {} start", self.round_number),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A recording is of the game its setup line states, and of no game
    /// when it states none: nothing takes it for EXE6's.
    #[test]
    fn a_recording_is_of_the_game_it_states() {
        let dir = std::env::temp_dir().join(format!("nettai-tools-trace-game-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let write = |name: &str, text: &str| {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            path
        };
        let exchange = "{\"exchange\":{\"frame\":1}}\n";
        let six = write("six.jsonl", &format!("{exchange}{{\"setup\":{{\"game\":\"exe6\",\"frame\":72}}}}\n"));
        assert_eq!(trace_game(&six).as_deref(), Ok("exe6"));
        // (Written by hand, with spaces.)
        let five = write("five.jsonl", "{\"setup\": {\"frame\": 10, \"game\": \"exe5\"}}\n");
        assert_eq!(trace_game(&five).as_deref(), Ok("exe5"));
        let none = write("none.jsonl", "{\"setup\":{\"frame\":72,\"game_versions\":[\"falzar\",\"falzar\"]}}\n");
        assert!(trace_game(&none).unwrap_err().starts_with("a recording that names no game"));
        let empty = write("empty.jsonl", exchange);
        assert!(trace_game(&empty).unwrap_err().contains("no setup line"));
        // A recording of another game than the content's, or of none, plays
        // on no content.
        let content = nettai_match::testing::exe6_content();
        let refused = |path: &std::path::Path| trace_rounds(path, &content).err().expect("refused");
        assert!(refused(&five).contains("an exe5 recording, and the content loaded is exe6's"), "{}", refused(&five));
        assert!(refused(&none).starts_with("a recording that names no game"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
