use super::*;

fn head() -> Head {
    Head { engine: "0.1.0".into(), game: "exe6".into(), content: 0x0123_4567_89AB_CDEF, start: 42 }
}

fn info() -> Info {
    Info { when: 1_790_000_000, side: 1, kind: MatchKind::Online, open_setups: false, names: ["Lan".into(), String::new()] }
}

/// A set of two rounds: input that changes now and then, a digest
/// every 60 ticks and at each round's end.
fn ticks() -> Vec<Tick> {
    let mut out: Vec<Tick> = (0..300u32)
        .map(|i| Tick {
            inputs: [if i % 20 < 5 { 0x001 } else { 0 }, if i % 33 < 3 { 0x220 } else if i % 50 == 7 { 0x600 } else { 0x200 }],
            digest: (i % 60 == 59).then_some(i as u64 * 7),
            ..Tick::default()
        })
        .collect();
    out[149].round_ended = true;
    out[149].digest = Some(1);
    out[299].round_ended = true;
    out[299].set_ended = true;
    out[299].digest = Some(2);
    out
}

fn written_with(info: &Info, ticks: &[Tick]) -> Vec<u8> {
    let mut w = Writer::new(Vec::new(), &head(), b"the match", info).unwrap();
    for t in ticks {
        w.tick(t).unwrap();
    }
    w.into_inner()
}

fn written(ticks: &[Tick]) -> Vec<u8> {
    written_with(&info(), ticks)
}

/// Why bytes don't read, as a reader is told.
fn refused(bytes: &[u8]) -> String {
    Replay::read(bytes).unwrap_err().to_string()
}

/// A replay reads back as it was written, each round where its mark is; a
/// tick that changes nothing is one byte.
#[test]
fn a_replay_reads_back() {
    let ticks = ticks();
    let bytes = written(&ticks);
    let r = Replay::read(&bytes).unwrap();
    assert_eq!((r.head, r.match_bytes.as_slice(), r.info), (head(), b"the match".as_slice(), info()));
    assert_eq!((r.ticks, r.end), (ticks.clone(), End::Set));
    let r = Replay::read(&bytes).unwrap();
    assert_eq!(r.rounds().iter().map(|t| t.len()).collect::<Vec<_>>(), [150, 150]);
    assert_eq!(r.round_starts(), [0, 150]);
    let header = bytes.len() - written(&[]).len();
    assert!(header < 300 * 3, "{header} bytes of input");
    // (The head, the info, the match, the input: a reader that wants who
    // played when reads no match.)
    let at = |tag: &[u8]| bytes.windows(4).position(|w| w == tag).unwrap();
    assert!(at(b"HEAD") < at(b"INFO") && at(b"INFO") < at(b"MTCH") && at(b"MTCH") < at(b"TICK"));
}

/// A round starts on the tick after a round's end, but for a mark on the
/// input's last tick: a set cut after its second round's end has two
/// rounds, not a third of no ticks.
#[test]
fn a_round_starts_after_each_rounds_end() {
    let mut cut = ticks();
    cut[299].set_ended = false;
    let r = Replay::read(&written(&cut)).unwrap();
    assert_eq!((r.end, r.round_starts()), (End::Open, vec![0, 150]));
    let r = Replay::read(&written(&cut[..200])).unwrap();
    assert_eq!(r.round_starts(), [0, 150]);
    let r = Replay::read(&written(&[])).unwrap();
    assert_eq!(r.round_starts(), [0]);
}

/// The kind of match and open setups are the info's, each read back as
/// written; a byte neither holds is refused.
#[test]
fn the_info_says_the_kind_of_match_and_open_setups() {
    for (kind, open_setups) in [(MatchKind::Offline, true), (MatchKind::Online, true), (MatchKind::Online, false)] {
        let info = Info { kind, open_setups, ..info() };
        let r = Replay::read(&written_with(&info, &ticks())).unwrap();
        assert_eq!(r.info, info);
    }
    let blank = Info { when: 0, side: 1, kind: MatchKind::Offline, open_setups: true, names: Default::default() };
    assert_eq!(Info::of(MatchKind::Offline, 1), blank);
    // The info's body: when (8 bytes), the side, the kind, open setups.
    let bytes = written(&[]);
    let body = bytes.windows(4).position(|w| w == b"INFO").unwrap() + 8;
    assert_eq!(bytes[body + 8..body + 11], [1, 1, 0]);
    let mut kind = bytes.clone();
    kind[body + 9] = 2;
    assert_eq!(refused(&kind), "the info's kind of match is 2, not 0 (offline) or 1 (online)");
    let mut open = bytes.clone();
    open[body + 10] = 2;
    assert_eq!(refused(&open), "the info's open setups is 2, not 0 or 1");
}

/// All a replay says of a side is its player's name and the match's bytes:
/// the head and the info are their listed fields and nothing more, so
/// there is no place a build's name could be.
#[test]
fn a_replay_holds_no_more_of_a_side_than_its_name_and_setup() {
    let bytes = written(&[]);
    let string = |s: &str| 1 + s.len();
    let (h, i) = (head(), info());
    let head_len = string(&h.engine) + string(&h.game) + 8 + 8;
    let info_len = 8 + 1 + 1 + 1 + string(&i.names[0]) + string(&i.names[1]);
    let section = |tag: &[u8], at: usize| -> usize {
        assert_eq!(&bytes[at..at + 4], tag);
        u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize
    };
    let mut at = 5;
    assert_eq!(section(b"HEAD", at), head_len);
    at += 8 + head_len;
    assert_eq!(section(b"INFO", at), info_len);
    at += 8 + info_len;
    assert_eq!(section(b"MTCH", at), b"the match".len());
    at += 8 + b"the match".len();
    assert_eq!(&bytes[at..], b"TICK");
}

/// A file cut short: every whole tick before the cut, and the cut said;
/// one that stops between ticks is open; the head cut short is damaged.
#[test]
fn a_cut_file_reads_to_its_cut() {
    let ticks = ticks();
    let bytes = written(&ticks);
    let start = written(&[]).len();
    // (Tick 59's record: its control, then its digest.)
    let at = written(&ticks[..59]).len();
    let r = Replay::read(&bytes[..at + 3]).unwrap();
    assert_eq!((r.ticks.len(), r.end), (59, End::Cut));
    let r = Replay::read(&bytes[..at]).unwrap();
    assert_eq!((r.ticks.len(), r.end), (59, End::Open));
    assert_eq!(Replay::read(&bytes[..start - 1]).unwrap_err(), ReadError::Damaged("the file ends inside its input".into()));
    assert_eq!(refused(&bytes[..20]), "the file ends inside its head");
    assert_eq!(refused(&bytes[..4]), "the file ends inside its start");
}

/// What isn't a replay, a replay of another layout (another version of
/// nettai's) and what no writer writes are each refused as what they are,
/// saying what.
#[test]
fn a_file_that_doesnt_read_is_refused_as_what_it_is() {
    let bytes = written(&ticks());
    let start = written(&[]).len();
    assert_eq!(Replay::read(b"PK\x03\x04").unwrap_err(), ReadError::NotAReplay);
    assert_eq!(refused(b"PK\x03\x04"), "not a replay (no \"NTRP\" at its start)");
    assert_eq!(Replay::read(b"").unwrap_err(), ReadError::NotAReplay);
    for other in [0, 2] {
        let mut layout = bytes.clone();
        layout[4] = other;
        assert_eq!(Replay::read(&layout).unwrap_err(), ReadError::OtherLayout { layout: other });
        assert_eq!(refused(&layout), format!("a replay of layout {other}; this reads layout 1"));
    }
    let mut bits = bytes.clone();
    bits[start] |= 0x20;
    assert_eq!(Replay::read(&bits).unwrap_err(), ReadError::Damaged("tick 0: its record's bits 0x23 are none a writer writes".into()));
    let mut more = bytes.clone();
    more.push(0);
    assert_eq!(refused(&more), "1 bytes after the set's end");
    let mut tag = bytes.clone();
    tag[5..9].copy_from_slice(b"HEAP");
    assert_eq!(refused(&tag), "no head where it goes (\"HEAP\" there)");
    // The first tick's record: side 0's input, then side 1's, past a tick
    // word's bits.
    let mut held = bytes.clone();
    held[start + 3..start + 5].copy_from_slice(&0x1000u16.to_le_bytes());
    assert_eq!(refused(&held), "tick 0: side 1's input 0x1000 is past a tick word's bits");
}

/// A writer refuses what a reader would.
#[test]
fn a_writer_refuses_what_isnt_a_set() {
    let mut w = Writer::new(Vec::new(), &head(), b"", &info()).unwrap();
    let end = Tick { round_ended: true, ..Tick::default() };
    assert!(w.tick(&end).is_err());
    assert!(w.tick(&Tick { set_ended: true, digest: Some(0), ..Tick::default() }).is_err());
    assert!(w.tick(&Tick { inputs: [0x1000, 0], ..Tick::default() }).is_err());
    w.tick(&Tick { set_ended: true, digest: Some(9), ..end }).unwrap();
    assert!(w.ended());
    assert!(w.tick(&Tick::default()).is_err());
}

/// The head names what a replay plays on, and what differs is said as what
/// it is.
#[test]
fn the_head_says_what_it_plays_on() {
    let h = head();
    assert_eq!(h.require("0.1.0", "exe6", h.content), Ok(()));
    let game = h.require("0.1.0", "exe5", h.content).unwrap_err();
    assert_eq!(game, Mismatch::Game { replay: "exe6".into(), here: "exe5".into() });
    assert_eq!(game.to_string(), "the replay is of exe6, not exe5");
    let content = h.require("0.1.0", "exe6", 1).unwrap_err();
    assert_eq!(content, Mismatch::Content { replay: h.content, here: 1 });
    assert_eq!(content.to_string(), "the replay was made on other content (its hash 0123456789abcdef, this one's 0000000000000001)");
    let engine = h.require("0.2.0", "exe6", h.content).unwrap_err();
    assert_eq!(engine, Mismatch::Engine { replay: "0.1.0".into(), here: "0.2.0".into() });
    assert_eq!(engine.to_string(), "the replay was made on nettai 0.1.0, this is 0.2.0");
    // (A patch release plays it.)
    assert_eq!(h.require("0.1.7", "exe6", h.content), Ok(()));
    assert_eq!((line("0.7.3"), line("1.2.0-rc.1"), line("1.2")), ("0.7", "1.2", "1.2"));
}

/// A file's head reads by itself, whatever is wrong after it: a file cut
/// inside its info, or one whose ticks no writer writes, still says its
/// game. Bytes that aren't a replay, another layout's, and a file that
/// ends inside its head say nothing.
#[test]
fn a_head_reads_whatever_follows_it() {
    let bytes = written(&ticks());
    assert_eq!(Head::read(&bytes), Some(head()));
    let info_at = bytes.windows(4).position(|w| w == b"INFO").unwrap();
    let cut = &bytes[..info_at + 6];
    assert!(Replay::read(cut).is_err());
    assert_eq!(Head::read(cut), Some(head()));
    let mut wrong = written(&[]);
    wrong.push(0xE0);
    assert!(Replay::read(&wrong).is_err());
    assert_eq!(Head::read(&wrong), Some(head()));
    let mut other = bytes.clone();
    other[4] = 2;
    assert_eq!(Head::read(&other), None);
    assert_eq!(Head::read(b"not a replay"), None);
    assert_eq!(Head::read(&bytes[..info_at - 3]), None);
}
