//! The replay file: everything a set's battles are played from, and
//! nothing a replay of it makes again. It runs no engine: nettai-frontend
//! records one as a set is played and plays one back; this crate writes
//! and reads the file.
//!
//! A replay holds one set, all its rounds: the match it is played from
//! (nettai-match's binary, against the content the head names), and each
//! tick's buttons, both players' by side. The simulation is a function of
//! those alone, so playing them back makes the set again, round by round
//! (the next round is made from the round before and the match). Nothing
//! it makes is kept (who won a round or the set, the score), but for two
//! marks, the ticks a round and the set ended on, which a reader may go by
//! without playing, and the battle's digest every so often, which a
//! playback compares to tell it reproduces. A replay plays only on the
//! engine and the content it was made with: the head names them, and a
//! player refuses another's ([`Head::require`]).
//!
//! ```text
//! file   := "NTRP", layout (a byte: 1), head, info, match, input
//! head   := "HEAD", length (u32), engine version (string), game (string),
//!           content hash (u64), the first battle's digest before any tick (u64)
//! info   := "INFO", length (u32), when (u64: unix seconds), the side that
//!           recorded (a byte: 0, 1), the kind of match (a byte: 0 offline,
//!           1 online), open setups (a byte: 0 no, 1 yes), the players'
//!           names by side (two strings)
//! match  := "MTCH", length (u32), the match in nettai-match's binary
//! input  := "TICK", then a record per tick to the end of the file
//! record := control (a byte), then what its bits announce, in their order:
//!           bit 0: side 0's buttons (u16) follow, bit 1: side 1's,
//!           bit 2: the battle's digest after the tick (u64) follows,
//!           bit 3: a round ended on the tick (a digest always comes with it),
//!           bit 4: the set ended on it (a round did too; nothing comes after);
//!           bits 5 to 7 are 0
//! string := length (LEB128), UTF-8
//! ```
//!
//! Numbers are little-endian. Buttons are the GBA's (the low ten bits), and
//! a side's are written when they change: before the first tick nothing is
//! held, and they carry across a round's end. A writer flushes each record
//! as it is written, so a file cut short (a crash) holds every tick before
//! the cut, and a reader says it was cut ([`End::Cut`]).
//!
//! **What a replay says of each side** is its player's name (the info) and
//! its setup (the match: the navi, the folder and whatever else its game's
//! rules take, each a number or a definition's handle). The setup is what
//! the simulation is made from, so it stays. A build's name is the
//! player's own business and is in neither: the info has no place for one,
//! and a match in binary holds no text at all. A viewer that should not
//! show the setups either is told so by the info ([`Info::open_setups`]:
//! where they weren't open).
//!
//! **The layout.** This is layout 1. A file of another layout is refused
//! as one from another version of nettai ([`ReadError::OtherLayout`]): a
//! replay plays only on the engine and content it was made with, so such a
//! file would not play here whatever its layout.

use std::fmt;
use std::io::{self, Write};

/// The file's first bytes.
pub const MAGIC: &[u8; 4] = b"NTRP";

/// The layout this crate writes and reads.
pub const LAYOUT: u8 = 1;

/// What a replay is played on: the engine, the game and its content, and
/// the battle they make before any tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Head {
    /// The engine's version.
    pub engine: String,
    /// The game (its pack's id: `exe6`).
    pub game: String,
    /// The content's hash (`nettai_battle::Content::hash`).
    pub content: u64,
    /// The digest of the set's first battle before any tick: a player
    /// compares it before playing, so a match that makes another battle on
    /// this engine is told from a simulation that goes another way later.
    pub start: u64,
}

impl Head {
    /// The head of a replay's file, whether or not the rest of it reads:
    /// what a list of replays goes by to tell which game a file it can't
    /// play is of. None if the bytes aren't a replay, are of another
    /// layout, or end inside the head.
    pub fn read(bytes: &[u8]) -> Option<Head> {
        if bytes.get(..4) != Some(MAGIC.as_slice()) || bytes.get(4) != Some(&LAYOUT) {
            return None;
        }
        let mut b = Bytes { rest: &bytes[5..], what: "start" };
        let mut h = b.section(b"HEAD", "head").ok()?;
        let head = Head { engine: h.string().ok()?, game: h.string().ok()?, content: h.u64().ok()?, start: h.u64().ok()? };
        h.done().ok()?;
        Some(head)
    }

    /// That a replay of this head plays on `engine`'s build of `game`'s
    /// `content`: or what differs.
    pub fn require(&self, engine: &str, game: &str, content: u64) -> Result<(), Mismatch> {
        if self.game != game {
            return Err(Mismatch::Game { replay: self.game.clone(), here: game.to_string() });
        }
        if self.content != content {
            return Err(Mismatch::Content { replay: self.content, here: content });
        }
        if self.engine != engine {
            return Err(Mismatch::Engine { replay: self.engine.clone(), here: engine.to_string() });
        }
        Ok(())
    }
}

/// Why a replay doesn't play here: what its head names that isn't what it
/// is asked to play on ([`Head::require`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mismatch {
    /// It is a replay of another game.
    Game { replay: String, here: String },
    /// It was made on other content (the hashes).
    Content { replay: u64, here: u64 },
    /// It was made on another engine (the versions).
    Engine { replay: String, here: String },
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mismatch::Game { replay, here } => write!(f, "the replay is of {replay}, not {here}"),
            Mismatch::Content { replay, here } => write!(f, "the replay was made on other content (its hash {replay:016x}, this one's {here:016x})"),
            Mismatch::Engine { replay, here } => write!(f, "the replay was made on nettai {replay}, this is {here}"),
        }
    }
}

impl std::error::Error for Mismatch {}

/// The kind of match a replay is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MatchKind {
    /// Played on one machine: no other player over a network.
    Offline,
    /// Played against another player over the network.
    Online,
}

impl MatchKind {
    /// The kind's byte in the info.
    fn byte(self) -> u8 {
        match self {
            MatchKind::Offline => 0,
            MatchKind::Online => 1,
        }
    }

    fn of_byte(b: u8) -> Option<MatchKind> {
        match b {
            0 => Some(MatchKind::Offline),
            1 => Some(MatchKind::Online),
            _ => None,
        }
    }
}

/// What a replay says about itself besides: when, who recorded it, what
/// kind of match it was and whether its setups were open, the players'
/// names. (Nothing of a player's build but what the match holds of it: no
/// build's name.)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Info {
    /// When the set started, in seconds since 1970 (UTC).
    pub when: u64,
    /// The side whose player recorded it (0 or 1): the one a viewer is
    /// shown by default.
    pub side: u8,
    /// The kind of match it was.
    pub kind: MatchKind,
    /// The match was played with open setups: each player could be shown
    /// the other's, and so may a viewer of the replay. Where they weren't,
    /// neither player was shown the other's, and a viewer isn't shown
    /// them either. (The match still holds both setups: the simulation is
    /// made from them.)
    pub open_setups: bool,
    /// The players' names by side; empty where none is known.
    pub names: [String; 2],
}

impl Info {
    /// The info of a match of `kind` recorded from `side`, with open
    /// setups, at no stated time, by players without names: for the rest
    /// to be filled in.
    pub fn of(kind: MatchKind, side: u8) -> Info {
        Info { when: 0, side, kind, open_setups: true, names: Default::default() }
    }
}

/// One tick: both players' buttons by side, and what a recording marks of
/// it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tick {
    pub buttons: [u16; 2],
    /// The battle's digest after the tick, where it was taken.
    pub digest: Option<u64>,
    /// A round of the set ended on this tick (its digest is taken).
    pub round_ended: bool,
    /// The set ended on this tick (a round did too).
    pub set_ended: bool,
}

/// How a replay's input ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// With the set's end: the whole set.
    Set,
    /// Between two ticks, before the set's end: the recording stopped
    /// there (the engine stopped, a player left).
    Open,
    /// Inside a tick's record: the file was cut short (its writer stopped
    /// mid-write). The ticks before the cut are whole.
    Cut,
}

/// A replay, read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replay {
    pub head: Head,
    /// The match, in nettai-match's binary.
    pub match_bytes: Vec<u8>,
    pub info: Info,
    pub ticks: Vec<Tick>,
    pub end: End,
}

const BUTTONS_0: u8 = 1 << 0;
const BUTTONS_1: u8 = 1 << 1;
const DIGEST: u8 = 1 << 2;
const ROUND_ENDED: u8 = 1 << 3;
const SET_ENDED: u8 = 1 << 4;
const KNOWN: u8 = BUTTONS_0 | BUTTONS_1 | DIGEST | ROUND_ENDED | SET_ENDED;

/// The buttons a tick may hold: the GBA's ten.
pub const BUTTON_MASK: u16 = 0x3FF;

// ---- Writing ------------------------------------------------------------------

/// Writes a replay as its set is played: the head, the info and the match
/// at once, then a record a tick, each flushed as it is written.
pub struct Writer<W: Write> {
    out: W,
    /// The buttons held after the last tick written, by side.
    buttons: [u16; 2],
    /// The set's end was written: nothing comes after it.
    ended: bool,
}

impl<W: Write> Writer<W> {
    /// A replay of `match_bytes` (nettai-match's binary) on `out`, from its
    /// head and info; written and flushed before the first tick.
    pub fn new(mut out: W, head: &Head, match_bytes: &[u8], info: &Info) -> io::Result<Writer<W>> {
        let mut b = MAGIC.to_vec();
        b.push(LAYOUT);
        let mut h = Vec::new();
        string(&mut h, &head.engine);
        string(&mut h, &head.game);
        h.extend_from_slice(&head.content.to_le_bytes());
        h.extend_from_slice(&head.start.to_le_bytes());
        section(&mut b, b"HEAD", &h);
        let mut i = info.when.to_le_bytes().to_vec();
        i.push(info.side);
        i.push(info.kind.byte());
        i.push(info.open_setups as u8);
        for name in &info.names {
            string(&mut i, name);
        }
        section(&mut b, b"INFO", &i);
        section(&mut b, b"MTCH", match_bytes);
        b.extend_from_slice(b"TICK");
        out.write_all(&b)?;
        out.flush()?;
        Ok(Writer { out, buttons: [0; 2], ended: false })
    }

    /// The next tick, written and flushed. Refused (nothing written): a
    /// tick after the set's end, a round's end without its digest, the
    /// set's end without a round's, buttons past the GBA's ten.
    pub fn tick(&mut self, t: &Tick) -> io::Result<()> {
        let invalid = |why: &str| Err(io::Error::new(io::ErrorKind::InvalidInput, why.to_string()));
        if self.ended {
            return invalid("a tick after the set's end");
        }
        if t.round_ended && t.digest.is_none() {
            return invalid("a round's end without its digest");
        }
        if t.set_ended && !t.round_ended {
            return invalid("the set's end without a round's");
        }
        if t.buttons.iter().any(|&b| b & !BUTTON_MASK != 0) {
            return invalid("buttons past the GBA's ten");
        }
        let mut control = 0;
        let mut rest = Vec::with_capacity(12);
        for side in 0..2 {
            if t.buttons[side] != self.buttons[side] {
                control |= [BUTTONS_0, BUTTONS_1][side];
                rest.extend_from_slice(&t.buttons[side].to_le_bytes());
            }
        }
        if let Some(d) = t.digest {
            control |= DIGEST;
            rest.extend_from_slice(&d.to_le_bytes());
        }
        control |= if t.round_ended { ROUND_ENDED } else { 0 } | if t.set_ended { SET_ENDED } else { 0 };
        let mut record = vec![control];
        record.extend_from_slice(&rest);
        self.out.write_all(&record)?;
        self.out.flush()?;
        self.buttons = t.buttons;
        self.ended = t.set_ended;
        Ok(())
    }

    /// The set's end was written.
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// The sink, once done with.
    pub fn into_inner(self) -> W {
        self.out
    }
}

fn string(out: &mut Vec<u8>, s: &str) {
    let mut n = s.len();
    loop {
        let byte = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
    out.extend_from_slice(s.as_bytes());
}

fn section(out: &mut Vec<u8>, tag: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(tag);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
}

// ---- Reading ------------------------------------------------------------------

/// Bytes being read, with where they are for a message.
struct Bytes<'a> {
    rest: &'a [u8],
    what: &'static str,
}

impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if n > self.rest.len() {
            return Err(format!("the file ends inside its {}", self.what));
        }
        let (b, rest) = self.rest.split_at(n);
        self.rest = rest;
        Ok(b)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("two bytes")))
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("four bytes")))
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("eight bytes")))
    }

    fn string(&mut self) -> Result<String, String> {
        let mut n: u64 = 0;
        for shift in (0..).step_by(7) {
            if shift > 28 {
                return Err(format!("a string's length in its {} is past any", self.what));
            }
            let b = self.u8()?;
            n |= ((b & 0x7F) as u64) << shift;
            if b & 0x80 == 0 {
                break;
            }
        }
        let b = self.take(n as usize)?;
        String::from_utf8(b.to_vec()).map_err(|_| format!("a string in its {} isn't UTF-8", self.what))
    }

    /// Section `tag`'s body, all of it.
    fn section(&mut self, tag: &[u8; 4], what: &'static str) -> Result<Bytes<'a>, String> {
        self.what = what;
        let got = self.take(4)?;
        if got != tag {
            return Err(format!("no {what} where it goes (\"{}\" there)", String::from_utf8_lossy(got)));
        }
        let n = self.u32()? as usize;
        Ok(Bytes { rest: self.take(n)?, what })
    }

    /// That nothing is left of a section.
    fn done(self) -> Result<(), String> {
        match self.rest.len() {
            0 => Ok(()),
            n => Err(format!("{n} bytes after the end of its {}", self.what)),
        }
    }
}

/// Why bytes don't read as a replay ([`Replay::read`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// They aren't a replay: no "NTRP" at their start.
    NotAReplay,
    /// A replay of another layout than this crate reads: one written by
    /// another version of nettai.
    OtherLayout { layout: u8 },
    /// A replay of this layout that no writer writes: its head, info or
    /// match cut short or running on, a value none of them holds, a
    /// tick's record with unknown bits. What is wrong, for a log.
    Damaged(String),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReadError::NotAReplay => write!(f, "not a replay (no \"NTRP\" at its start)"),
            ReadError::OtherLayout { layout } => write!(f, "a replay of layout {layout}; this reads layout {LAYOUT}"),
            ReadError::Damaged(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for ReadError {}

impl Replay {
    /// A replay from a file's bytes. Refused, saying why: bytes that aren't
    /// a replay, a replay of another layout (another version's), and one
    /// of this layout that is damaged: a head, info or match cut short or
    /// running on, and a tick's record no writer writes (unknown bits, a
    /// round's end without its digest, the set's end without a round's,
    /// buttons past the GBA's, anything after the set's end). A record cut
    /// short ends the input there ([`End::Cut`]).
    pub fn read(bytes: &[u8]) -> Result<Replay, ReadError> {
        if bytes.get(..4) != Some(MAGIC.as_slice()) {
            return Err(ReadError::NotAReplay);
        }
        match bytes.get(4) {
            Some(&LAYOUT) => Replay::read_sections(&bytes[5..]).map_err(ReadError::Damaged),
            Some(&layout) => Err(ReadError::OtherLayout { layout }),
            None => Err(ReadError::Damaged("the file ends inside its start".into())),
        }
    }

    /// A replay of this crate's layout from what follows the layout byte:
    /// or what is wrong with it.
    fn read_sections(bytes: &[u8]) -> Result<Replay, String> {
        let mut b = Bytes { rest: bytes, what: "start" };
        let mut h = b.section(b"HEAD", "head")?;
        let head = Head { engine: h.string()?, game: h.string()?, content: h.u64()?, start: h.u64()? };
        h.done()?;
        let mut i = b.section(b"INFO", "info")?;
        let when = i.u64()?;
        let side = i.u8()?;
        if side > 1 {
            return Err(format!("the info's recording side is {side}, not 0 or 1"));
        }
        let kind = i.u8()?;
        let kind = MatchKind::of_byte(kind).ok_or_else(|| format!("the info's kind of match is {kind}, not 0 (offline) or 1 (online)"))?;
        let open_setups = match i.u8()? {
            0 => false,
            1 => true,
            n => return Err(format!("the info's open setups is {n}, not 0 or 1")),
        };
        let info = Info { when, side, kind, open_setups, names: [i.string()?, i.string()?] };
        i.done()?;
        let match_bytes = b.section(b"MTCH", "match")?.rest.to_vec();
        b.what = "input";
        if b.take(4)? != b"TICK" {
            return Err("no input where it goes".into());
        }
        let mut ticks = Vec::new();
        let mut buttons = [0u16; 2];
        let end = loop {
            if b.rest.is_empty() {
                break End::Open;
            }
            let n = ticks.len();
            let control = b.u8()?;
            if control & !KNOWN != 0 {
                return Err(format!("tick {n}: its record's bits {control:#04x} are none a writer writes"));
            }
            let record = |b: &mut Bytes| -> Result<Option<Tick>, String> {
                let mut t = Tick { buttons, ..Tick::default() };
                for side in 0..2 {
                    if control & [BUTTONS_0, BUTTONS_1][side] != 0 {
                        let Ok(held) = b.u16() else { return Ok(None) };
                        if held & !BUTTON_MASK != 0 {
                            return Err(format!("tick {n}: side {side}'s buttons {held:#06x} are past the GBA's"));
                        }
                        t.buttons[side] = held;
                    }
                }
                if control & DIGEST != 0 {
                    let Ok(d) = b.u64() else { return Ok(None) };
                    t.digest = Some(d);
                }
                t.round_ended = control & ROUND_ENDED != 0;
                t.set_ended = control & SET_ENDED != 0;
                if t.round_ended && t.digest.is_none() {
                    return Err(format!("tick {n}: a round ends without its digest"));
                }
                if t.set_ended && !t.round_ended {
                    return Err(format!("tick {n}: the set ends without a round"));
                }
                Ok(Some(t))
            };
            let Some(t) = record(&mut b)? else { break End::Cut };
            buttons = t.buttons;
            ticks.push(t);
            if t.set_ended {
                if !b.rest.is_empty() {
                    return Err(format!("{} bytes after the set's end", b.rest.len()));
                }
                break End::Set;
            }
        };
        Ok(Replay { head, match_bytes, info, ticks, end })
    }

    /// The ticks of each round, in order, as the marks divide them (the
    /// last round's run to the input's end, ended or not).
    pub fn rounds(&self) -> Vec<&[Tick]> {
        let mut out = Vec::new();
        let mut from = 0;
        for (i, t) in self.ticks.iter().enumerate() {
            if t.round_ended {
                out.push(&self.ticks[from..=i]);
                from = i + 1;
            }
        }
        if from < self.ticks.len() {
            out.push(&self.ticks[from..]);
        }
        out
    }

    /// The tick each round starts on, counted through the set from 0, in
    /// order: the first round's 0, each later one's the tick after a
    /// round's end (a mark on the input's last tick starts none).
    pub fn round_starts(&self) -> Vec<u32> {
        let mut out = vec![0];
        for (i, t) in self.ticks.iter().enumerate() {
            if t.round_ended && i + 1 < self.ticks.len() {
                out.push(i as u32 + 1);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
