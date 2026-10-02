//! M4A songs as Standard MIDI files, in mid2agb's conventions, plus a TOML
//! sidecar for what belongs to the song rather than its tracks.
//!
//! The MIDI file (format 1, 24 ticks a quarter note = the M4A tick):
//!
//! - Track 0 is the conductor: the song's name, a 4/4 time signature (for
//!   editors only), the tempo, and the loop as markers `[` and `]`.
//! - M4A track n is MIDI track n + 1, on channel n. Its commands, in their
//!   order on each tick:
//!
//! | M4A | MIDI |
//! |---|---|
//! | notes (N01..N96 with gate, TIE .. EOT) | note on / note off; a note of 96 ticks or less is a gated note, a longer one a tie |
//! | VOICE | program change |
//! | VOL, PAN, MOD | CC 7, 10, 1 |
//! | BEND | pitch bend (the high 7 bits) |
//! | BENDR, LFOS, MODT, TUNE, LFODL | CC 20, 21, 22, 24, 26 (mid2agb) |
//! | PRIO | CC 33 (mid2agb) |
//! | XCMD echo volume / length | CC 30 = 8 or 9, then CC 29 = value (mid2agb) |
//! | MEMACC (set, add, sub; from memory) | CC 13 = operation 0..=5, CC 14 = address, then CC 12 = operand (mid2agb) |
//! | TEMPO (BPM / 2) | tempo meta event (in the conductor when it opens its tick on track 0) |
//! | KEYSH | CC 102 = shift + 64 (extension) |
//! | a tie of 96 ticks or less, a gated note over 96 | CC 103 = 1 or 2 just before the note (extension) |
//! | EOT with no tie of its key sounding | CC 104 = key (extension) |
//! | FINE earlier than a note's end | marker `fine` (extension) |
//!
//! A track ends (FINE) at its end-of-track event. A song loops from `[` to
//! `]`: every track repeats that span. Note offs after `]` only end notes
//! that cross the loop's end (a tie released early in the next pass).
//!
//! The sidecar holds the song header (player, priority, reverb,
//! voicegroup), the track count, and a stamp of the MIDI file as exported.

use crate::midi::{Event as MidiEvent, Message, MidiTrack, Smf};
use crate::report::Report;
use crate::timeline::{self, Ending, Event, Timeline};
use m4a::bank::{Command, MemOp, PlayerId, Song, Track, VoicegroupId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// Ticks a quarter note.
pub const TICKS_PER_BEAT: u16 = 24;
/// Gated notes are at most this long unless marked.
const LONGEST_GATED: u32 = 96;

const CC_MOD: u8 = 1;
const CC_MEMACC: u8 = 12;
const CC_MEMACC_OP: u8 = 13;
const CC_MEMACC_ADDRESS: u8 = 14;
const CC_MEMACC_ALT: u8 = 16;
const CC_VOLUME: u8 = 7;
const CC_PAN: u8 = 10;
const CC_BEND_RANGE: u8 = 20;
const CC_LFO_SPEED: u8 = 21;
const CC_MOD_TYPE: u8 = 22;
const CC_TUNE: u8 = 24;
const CC_LFO_DELAY: u8 = 26;
const CC_XCMD_VALUE: u8 = 29;
const CC_XCMD: u8 = 30;
const CC_XCMD_VALUE_ALT: u8 = 31;
const CC_PRIORITY: u8 = 33;
const CC_PRIORITY_ALT: u8 = 39;
/// Extensions (undefined controllers in General MIDI; mid2agb ignores them).
pub const CC_KEY_SHIFT: u8 = 102;
pub const CC_NOTE_FORM: u8 = 103;
pub const CC_END_TIE: u8 = 104;
const FORM_TIE: u8 = 1;
const FORM_GATED: u8 = 2;
const XCMD_ECHO_VOLUME: u8 = 8;
const XCMD_ECHO_LENGTH: u8 = 9;

/// The sidecar of a song.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SongDoc {
    /// The song-table entry it is.
    pub id: u16,
    pub midi: String,
    pub player: u8,
    pub priority: u8,
    /// The reverb level it sets (absent: it leaves the mixer's reverb).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverb: Option<u8>,
    pub voicegroup: String,
    pub tracks: usize,
    /// FNV-1a of the MIDI file as exported.
    pub midi_stamp: String,
    /// What the MIDI file held when exported. Only for checking: the
    /// importer warns when an edit lost the loop or every command of a
    /// kind (what editors that don't know M4A's controllers do).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exported: Option<Exported>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    #[serde(default, rename = "loop", skip_serializing_if = "Option::is_none")]
    pub loop_ticks: Option<[u32; 2]>,
    /// Where each track ends (FINE), for songs that don't loop.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ends: Vec<u32>,
    /// Commands by kind, over all tracks.
    pub commands: BTreeMap<String, usize>,
}

fn kind(c: &Command) -> &'static str {
    match c {
        Command::Note { gate: 0, .. } => "tie",
        Command::Note { .. } => "note",
        Command::EndTie { .. } => "end_tie",
        Command::Voice(_) => "voice",
        Command::Volume(_) => "volume",
        Command::Pan(_) => "pan",
        Command::Bend(_) => "bend",
        Command::BendRange(_) => "bend_range",
        Command::LfoSpeed(_) => "lfo_speed",
        Command::LfoDelay(_) => "lfo_delay",
        Command::Modulation(_) => "mod",
        Command::ModulationType(_) => "mod_type",
        Command::Tune(_) => "tune",
        Command::Priority(_) => "priority",
        Command::KeyShift(_) => "key_shift",
        Command::Tempo(_) => "tempo",
        Command::EchoVolume(_) => "echo_volume",
        Command::EchoLength(_) => "echo_length",
        Command::MemAcc { .. } => "memacc",
        _ => "other",
    }
}

fn summary(tls: &[Timeline]) -> Exported {
    let mut commands = BTreeMap::new();
    for t in tls {
        for e in &t.events {
            *commands.entry(kind(&e.command).to_string()).or_default() += 1;
        }
    }
    let loop_ticks = tls.iter().find_map(|t| t.loop_ticks()).map(|(s, e)| [s, e]);
    let ends = if loop_ticks.is_some() { Vec::new() } else { tls.iter().map(Timeline::end_tick).collect() };
    Exported { loop_ticks, ends, commands }
}

// ---- Export ------------------------------------------------------------------

/// A song as MIDI bytes and its sidecar (with `voicegroup` the name of its
/// voicegroup file).
pub fn export(song: &Song, id: u16, title: &str, voicegroup: &str, midi_file: &str) -> Result<(Vec<u8>, SongDoc), String> {
    let mut tls = Vec::new();
    for (i, t) in song.tracks.iter().enumerate() {
        tls.push(timeline::linearize(t).map_err(|e| format!("track {i}: {e}"))?);
    }
    let song_loop = align_loops(&mut tls)?;
    let smf = to_smf(&tls, song_loop, title)?;
    let bytes = smf.to_bytes();
    // The file must read back as the same timelines.
    let (back, report) = timelines_from_smf(&smf, tls.len());
    if back.as_ref() != Some(&tls) {
        let why = report.issues.iter().map(|i| i.message.clone()).collect::<Vec<_>>().join("; ");
        return Err(format!("the MIDI mapping doesn't read back the same (a case it can't express yet) {why}"));
    }
    let doc = SongDoc {
        id,
        midi: midi_file.into(),
        player: song.player.0,
        priority: song.priority,
        reverb: song.reverb,
        voicegroup: voicegroup.into(),
        tracks: song.tracks.len(),
        midi_stamp: crate::report::stamp(&bytes),
        exported: Some(summary(&tls)),
    };
    Ok((bytes, doc))
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Give every looping track the song's loop: from the latest track's
/// earliest whole-tick start (rounded up to an eighth note), over the
/// common period.
pub fn align_loops(tls: &mut [Timeline]) -> Result<Option<(u32, u32)>, String> {
    let froms: Vec<Option<(u32, u32)>> = tls.iter().map(|t| t.loop_from()).collect();
    if froms.iter().all(Option::is_none) {
        return Ok(None);
    }
    if froms.iter().any(Option::is_none) {
        return Err("some tracks loop and others end; one loop per song is supported".into());
    }
    let start = froms.iter().flatten().map(|f| f.0).max().unwrap().div_ceil(12) * 12;
    let period = froms.iter().flatten().map(|f| f.1).fold(1, |a, p| a / gcd(a, p) * p);
    if period > 1 << 20 {
        return Err(format!("the tracks' loops only line up every {period} ticks"));
    }
    for t in tls.iter_mut() {
        *t = t.with_loop(start, period);
    }
    Ok(Some((start, start + period)))
}

fn tempo_us(bpm_half: u8) -> Result<u32, String> {
    let us = (30_000_000.0 / bpm_half as f64).round() as u32;
    if bpm_half < 2 {
        return Err(format!("tempo {} BPM is slower than MIDI can say", 2 * bpm_half as u32));
    }
    Ok(us)
}

fn tempo_from_us(us: u32) -> (u8, bool) {
    let v = (30_000_000.0 / us.max(1) as f64).round();
    let exact = (tempo_us(v.clamp(2.0, 255.0) as u8).ok() == Some(us)) && (2.0..=255.0).contains(&v);
    (v.clamp(1.0, 255.0) as u8, exact)
}

/// Whether a tempo event at `i` on track 0 opens its tick (only KEYSH
/// before it there), so it can live in the conductor.
fn tempo_opens_tick(t: &Timeline, i: usize) -> bool {
    let tick = t.events[i].tick;
    t.events[..i].iter().rev().take_while(|e| e.tick == tick).all(|e| matches!(e.command, Command::KeyShift(_)))
}

fn to_smf(tls: &[Timeline], song_loop: Option<(u32, u32)>, title: &str) -> Result<Smf, String> {
    let mut conductor = vec![
        MidiEvent { tick: 0, message: Message::TrackName(title.into()) },
        MidiEvent { tick: 0, message: Message::TimeSignature([4, 2, 24, 8]) },
    ];
    let mut tracks = Vec::new();
    let mut song_end = 0;
    for (n, t) in tls.iter().enumerate() {
        let ch = n as u8;
        if n >= 16 {
            return Err("more than 16 tracks".into());
        }
        // (tick, group, order, message): group 0 are note offs of gated
        // notes, which go first on their tick.
        let mut out: Vec<(u32, u8, usize, Message)> = Vec::new();
        let mut push = |tick: u32, group: u8, order: usize, m: Message| out.push((tick, group, order, m));
        // Pair ties with their end-ties: the newest open tie of the key.
        let mut open: HashMap<u8, Vec<usize>> = HashMap::new();
        // tie -> (the tick it ends, whether that is after the loop's end).
        let mut tie_end: HashMap<usize, (u32, bool)> = HashMap::new();
        let mut orphan_eots = Vec::new();
        for (i, e) in t.events.iter().enumerate() {
            match e.command {
                Command::Note { gate: 0, key: Some(k), .. } => {
                    let v = open.entry(k).or_default();
                    if !v.is_empty() {
                        return Err(format!("track {n}: two ties on key {k} at once at tick {}", e.tick));
                    }
                    v.push(i);
                }
                Command::EndTie { key: Some(k) } => match open.get_mut(&k).and_then(|v| v.pop()) {
                    Some(tie) => {
                        tie_end.insert(tie, (e.tick, false));
                    }
                    None => orphan_eots.push(i),
                },
                _ => {}
            }
        }
        // Ties of the loop still open at its end end in the next pass, at
        // the loop's first end-tie of their key.
        if let Ending::Loop { start, start_tick, end_tick } = t.ending {
            for (&k, ties) in &open {
                for &tie in ties.iter().filter(|&&tie| tie >= start) {
                    if let Some(e) = t.events[start..].iter().find(|e| e.command == Command::EndTie { key: Some(k) }) {
                        tie_end.insert(tie, (e.tick + (end_tick - start_tick), true));
                    }
                }
            }
        }
        let mut last_off = 0;
        for (i, e) in t.events.iter().enumerate() {
            let tick = e.tick;
            let cc = |controller: u8, value: u8| Message::Control { channel: ch, controller, value };
            let small = |what: &str, v: u8| -> Result<u8, String> {
                if v > 127 { Err(format!("track {n}: {what} {v} at tick {tick} doesn't fit MIDI's 0..=127")) } else { Ok(v) }
            };
            match e.command {
                Command::Note { gate, key: Some(key), velocity: Some(velocity) } => {
                    if velocity == 0 {
                        return Err(format!("track {n}: a note with velocity 0 at tick {tick}"));
                    }
                    let form = if gate == 0 {
                        match tie_end.get(&i) {
                            Some(&(end, _)) if end - tick <= LONGEST_GATED => Some(FORM_TIE),
                            _ => None,
                        }
                    } else if gate as u32 > LONGEST_GATED {
                        Some(FORM_GATED)
                    } else {
                        None
                    };
                    if let Some(f) = form {
                        push(tick, 1, 2 * i, cc(CC_NOTE_FORM, f));
                    }
                    push(tick, 1, 2 * i + 1, Message::NoteOn { channel: ch, key, velocity: small("key", velocity)? });
                    if gate > 0 {
                        push(tick + gate as u32, 0, i, Message::NoteOff { channel: ch, key, velocity: 0 });
                        last_off = last_off.max(tick + gate as u32);
                    } else if let Some(&(end, true)) = tie_end.get(&i) {
                        // Ends after the loop's end.
                        push(end, 2, i, Message::NoteOff { channel: ch, key, velocity: 0 });
                        last_off = last_off.max(end);
                    }
                }
                Command::EndTie { key: Some(key) } => {
                    if orphan_eots.contains(&i) {
                        push(tick, 1, 2 * i, cc(CC_END_TIE, key));
                    } else {
                        push(tick, 1, 2 * i, Message::NoteOff { channel: ch, key, velocity: 0 });
                    }
                }
                Command::Tempo(v) if n == 0 && tempo_opens_tick(t, i) => {
                    conductor.push(MidiEvent { tick, message: Message::Tempo(tempo_us(v)?) });
                }
                Command::Tempo(v) => push(tick, 1, 2 * i, Message::Tempo(tempo_us(v)?)),
                Command::Voice(v) => push(tick, 1, 2 * i, Message::Program { channel: ch, program: small("voice", v)? }),
                Command::Volume(v) => push(tick, 1, 2 * i, cc(CC_VOLUME, small("volume", v)?)),
                Command::Pan(v) => push(tick, 1, 2 * i, cc(CC_PAN, small("pan", v)?)),
                Command::Modulation(v) => push(tick, 1, 2 * i, cc(CC_MOD, small("mod", v)?)),
                Command::BendRange(v) => push(tick, 1, 2 * i, cc(CC_BEND_RANGE, small("bend range", v)?)),
                Command::LfoSpeed(v) => push(tick, 1, 2 * i, cc(CC_LFO_SPEED, small("LFO speed", v)?)),
                Command::ModulationType(v) => push(tick, 1, 2 * i, cc(CC_MOD_TYPE, small("mod type", v)?)),
                Command::Tune(v) => push(tick, 1, 2 * i, cc(CC_TUNE, small("tune", v)?)),
                Command::LfoDelay(v) => push(tick, 1, 2 * i, cc(CC_LFO_DELAY, small("LFO delay", v)?)),
                Command::Priority(v) => push(tick, 1, 2 * i, cc(CC_PRIORITY, small("priority", v)?)),
                Command::Bend(v) => {
                    push(tick, 1, 2 * i, Message::PitchBend { channel: ch, value: (small("bend", v)? as u16) << 7 })
                }
                Command::KeyShift(v) => {
                    if !(-64..=63).contains(&v) {
                        return Err(format!("track {n}: key shift {v} at tick {tick} is out of range"));
                    }
                    push(tick, 1, 2 * i, cc(CC_KEY_SHIFT, (v as i32 + 64) as u8));
                }
                Command::EchoVolume(v) | Command::EchoLength(v) => {
                    let which = if matches!(e.command, Command::EchoVolume(_)) { XCMD_ECHO_VOLUME } else { XCMD_ECHO_LENGTH };
                    push(tick, 1, 2 * i, cc(CC_XCMD, which));
                    push(tick, 1, 2 * i + 1, cc(CC_XCMD_VALUE, small("echo", v)?));
                }
                Command::MemAcc { op, address, operand } => {
                    let code = match op {
                        MemOp::Set => 0,
                        MemOp::Add => 1,
                        MemOp::Sub => 2,
                        MemOp::SetFromMemory => 3,
                        MemOp::AddFromMemory => 4,
                        MemOp::SubFromMemory => 5,
                        MemOp::JumpIf { .. } => {
                            return Err(format!("track {n}: a conditional MEMACC at tick {tick} can't be a MIDI event"));
                        }
                    };
                    push(tick, 1, 2 * i, cc(CC_MEMACC_OP, code));
                    push(tick, 1, 2 * i, cc(CC_MEMACC_ADDRESS, small("memory address", address)?));
                    push(tick, 1, 2 * i + 1, cc(CC_MEMACC, small("memory operand", operand)?));
                }
                c => return Err(format!("track {n}: unexpected command {c:?} in a timeline")),
            }
        }
        let end = match t.ending {
            Ending::Fine { tick } => {
                if last_off > tick {
                    push(tick, 3, 0, Message::Marker("fine".into()));
                }
                tick.max(last_off)
            }
            Ending::Loop { end_tick, .. } => end_tick.max(last_off),
        };
        song_end = song_end.max(end);
        out.sort_by_key(|e| (e.0, e.1, e.2));
        let mut events = vec![MidiEvent { tick: 0, message: Message::TrackName(format!("Track {n}")) }];
        events.extend(out.into_iter().map(|(tick, _, _, message)| MidiEvent { tick, message }));
        tracks.push(MidiTrack { events, end });
    }
    if let Some((start, end)) = song_loop {
        conductor.push(MidiEvent { tick: start, message: Message::Marker("[".into()) });
        conductor.push(MidiEvent { tick: end, message: Message::Marker("]".into()) });
    }
    conductor.sort_by_key(|e| e.tick);
    let mut all = vec![MidiTrack { events: conductor, end: song_end }];
    all.extend(tracks);
    Ok(Smf { format: 1, division: TICKS_PER_BEAT, tracks: all })
}

// ---- Import ------------------------------------------------------------------

/// Read a song back from its MIDI bytes and sidecar. `voicegroup` is the
/// sidecar's voicegroup resolved to an id.
pub fn import(
    bytes: &[u8],
    doc: &SongDoc,
    voicegroup: VoicegroupId,
    file: &str,
    report: &mut Report,
) -> Option<Song> {
    if crate::report::stamp(bytes) != doc.midi_stamp {
        report.note(file, "edited since export");
    }
    let smf = match Smf::from_bytes(bytes) {
        Ok(s) => s,
        Err(e) => {
            report.error(file, e.to_string());
            return None;
        }
    };
    let (tls, r) = timelines_from_smf(&smf, doc.tracks);
    for i in r.issues {
        report.push(i.level, file, i.message);
    }
    let tls = tls?;
    if let Some(then) = &doc.exported {
        let now = summary(&tls);
        if then.loop_ticks.is_some() && now.loop_ticks.is_none() {
            report.warn(file, "the loop markers `[` and `]` are gone: the song now plays once and stops");
        }
        if !then.ends.is_empty() && now.loop_ticks.is_none() && then.ends != now.ends {
            report.warn(
                file,
                format!(
                    "the tracks now end (FINE) at ticks {:?}, not {:?}: an editor that pads or merges tracks moves the end, \
                     which cuts notes at another time",
                    now.ends, then.ends
                ),
            );
        }
        for (k, &n) in &then.commands {
            if n > 0 && now.commands.get(k).copied().unwrap_or(0) == 0 {
                report.warn(
                    file,
                    format!("all {n} {k} commands are gone; the editor probably dropped M4A's controllers (see the MIDI mapping)"),
                );
            }
        }
    }
    Some(Song {
        player: PlayerId(doc.player),
        priority: doc.priority,
        reverb: doc.reverb,
        voicegroup,
        tracks: tls.iter().map(timeline::encode).collect::<Vec<Track>>(),
    })
}

/// A MIDI position for messages: bar.beat.tick at 4/4.
fn at(tick: u32) -> String {
    format!("{}.{}.{} (tick {tick})", tick / 96 + 1, tick % 96 / 24 + 1, tick % 24)
}

/// The timelines of `n` M4A tracks from a MIDI file.
pub fn timelines_from_smf(smf: &Smf, n: usize) -> (Option<Vec<Timeline>>, Report) {
    let mut report = Report::default();
    let f = "";
    let div = smf.division as u32;
    if div == 0 || !div.is_multiple_of(TICKS_PER_BEAT as u32) && !(TICKS_PER_BEAT as u32).is_multiple_of(div) {
        report.error(f, format!("{div} ticks a quarter note doesn't convert to M4A's 24 exactly; use a multiple of 24"));
        return (None, report);
    }
    let mut off_grid = 0;
    let mut conv = |t: u32| -> u32 {
        let x = t as u64 * TICKS_PER_BEAT as u64;
        if !x.is_multiple_of(div as u64) {
            off_grid += 1;
        }
        (x / div as u64) as u32
    };
    // Conductor data and each channel's events, with their track's end.
    let mut tempos: Vec<(u32, Message)> = Vec::new();
    let mut global_loop: (Option<u32>, Option<u32>) = (None, None);
    let mut channels: BTreeMap<u8, Vec<(u32, Message)>> = BTreeMap::new();
    let mut ends: HashMap<u8, u32> = HashMap::new();
    let mut local_loops: HashMap<u8, (Option<u32>, Option<u32>)> = HashMap::new();
    let mut fines: HashMap<u8, u32> = HashMap::new();
    for track in &smf.tracks {
        let track_channels: Vec<u8> = {
            let mut c: Vec<u8> = track.events.iter().filter_map(|e| e.message.channel()).collect();
            c.sort();
            c.dedup();
            c
        };
        let end = conv(track.end);
        for &c in &track_channels {
            let e = ends.entry(c).or_insert(0);
            *e = (*e).max(end);
        }
        let own = track_channels.first().copied().filter(|_| track_channels.len() == 1);
        for e in &track.events {
            let tick = conv(e.tick);
            match &e.message {
                Message::Marker(m) | Message::Text(m) => {
                    let m = m.trim();
                    let slot = match own {
                        Some(c) => local_loops.entry(c).or_default(),
                        None => &mut global_loop,
                    };
                    match m {
                        "[" | "loopStart" => slot.0 = Some(tick),
                        "]" | "loopEnd" => slot.1 = Some(tick),
                        "fine" => {
                            if let Some(c) = own {
                                fines.insert(c, tick);
                            }
                        }
                        _ => {}
                    }
                }
                Message::Tempo(_) if own.is_none() => tempos.push((tick, e.message.clone())),
                Message::Tempo(_) => channels.entry(own.unwrap()).or_default().push((tick, e.message.clone())),
                m => {
                    if let Some(c) = m.channel() {
                        channels.entry(c).or_default().push((tick, m.clone()));
                    }
                }
            }
        }
    }
    if off_grid > 0 {
        report.error(f, format!("{off_grid} events fall between M4A's 24 ticks a beat; quantize them to 1/96 notes"));
        return (None, report);
    }
    if let Some(&c) = channels.keys().find(|&&c| c as usize >= n) {
        report.error(f, format!("channel {} has events but the song has {n} tracks (channels 1..={n})", c + 1));
        return (None, report);
    }
    // Conductor tempos go to track 0, after the KEYSH that open their tick
    // (and the conductor tempos placed there before them).
    if !tempos.is_empty() {
        let ch0 = channels.entry(0).or_default();
        let mut placed: HashMap<u32, usize> = HashMap::new();
        for (tick, m) in tempos {
            let mut i = ch0.partition_point(|(t, _)| *t < tick);
            while i < ch0.len()
                && ch0[i].0 == tick
                && matches!(ch0[i].1, Message::Control { controller: CC_KEY_SHIFT, .. })
            {
                i += 1;
            }
            let k = placed.entry(tick).or_default();
            ch0.insert(i + *k, (tick, m));
            *k += 1;
        }
    }
    let song_end = ends.values().copied().max().unwrap_or(0);
    let mut out = Vec::new();
    for c in 0..n as u8 {
        let events = channels.remove(&c).unwrap_or_default();
        let lp = local_loops.get(&c).copied().filter(|l| l.0.is_some() || l.1.is_some()).unwrap_or(global_loop);
        let span = match lp {
            (Some(s), Some(e)) if e > s => Some((s, e)),
            (None, None) => None,
            _ => {
                report.error(f, "the loop needs both a `[` and a later `]` marker");
                return (None, report);
            }
        };
        let end = ends.get(&c).copied().unwrap_or(song_end);
        match track_timeline(c, &events, span, fines.get(&c).copied().unwrap_or(end), &mut report) {
            Some(t) => out.push(t),
            None => return (None, report),
        }
    }
    (Some(out), report)
}

fn track_timeline(
    c: u8,
    events: &[(u32, Message)],
    span: Option<(u32, u32)>,
    fine: u32,
    report: &mut Report,
) -> Option<Timeline> {
    let f = "";
    let mut out: Vec<Event> = Vec::new();
    // Open notes by key, oldest first: (index in `out`, start, form hint).
    let mut open: HashMap<u8, VecDeque<(usize, u32, Option<u8>)>> = HashMap::new();
    let mut form: Option<u8> = None;
    let mut xcmd: Option<u8> = None;
    // MEMACC's operation and address, until the controller that runs it.
    let (mut memacc_op, mut memacc_address) = (0u8, 0u8);
    let wrap = span.map(|s| s.1);
    let mut bend_rounded = 0;
    for (tick, m) in events {
        let tick = *tick;
        let in_wrap = wrap.is_some_and(|w| tick >= w);
        if in_wrap && !matches!(m, Message::NoteOff { .. }) {
            report.error(f, format!("track {c}: an event at {} is after the loop's end `]` and never plays", at(tick)));
            return None;
        }
        macro_rules! push {
            ($c:expr) => {
                out.push(Event { tick, command: $c })
            };
        }
        match *m {
            Message::NoteOn { key, velocity, .. } => {
                let i = out.len();
                push!(Command::Note { gate: 0, key: Some(key), velocity: Some(velocity) });
                open.entry(key).or_default().push_back((i, tick, form.take()));
            }
            Message::NoteOff { key, .. } => {
                let Some((i, start, hint)) = open.get_mut(&key).and_then(|q| q.pop_front()) else {
                    report.note(f, format!("track {c}: a note off for key {key} at {} with no note on; ignored", at(tick)));
                    continue;
                };
                let len = tick - start;
                let tie = match hint {
                    Some(FORM_TIE) => true,
                    Some(FORM_GATED) => len == 0 || len > 255,
                    _ => len == 0 || len > LONGEST_GATED,
                };
                if hint == Some(FORM_GATED) && tie {
                    report.warn(f, format!("track {c}: the note at {} is marked gated but lasts {len} ticks (1..=255)", at(start)));
                }
                if tie {
                    if !in_wrap {
                        push!(Command::EndTie { key: Some(key) });
                    }
                } else if let Command::Note { gate, .. } = &mut out[i].command {
                    *gate = len as u8;
                }
            }
            Message::Control { controller, value, .. } => {
                let command = match controller {
                    CC_VOLUME => Command::Volume(value),
                    CC_PAN => Command::Pan(value),
                    CC_MOD => Command::Modulation(value),
                    CC_BEND_RANGE => Command::BendRange(value),
                    CC_LFO_SPEED => Command::LfoSpeed(value),
                    CC_MOD_TYPE => Command::ModulationType(value),
                    CC_TUNE => Command::Tune(value),
                    CC_LFO_DELAY => Command::LfoDelay(value),
                    CC_PRIORITY | CC_PRIORITY_ALT => Command::Priority(value),
                    CC_KEY_SHIFT => Command::KeyShift(value as i8 - 64),
                    CC_END_TIE => Command::EndTie { key: Some(value) },
                    CC_NOTE_FORM => {
                        form = Some(value);
                        continue;
                    }
                    CC_MEMACC_OP => {
                        memacc_op = value;
                        continue;
                    }
                    CC_MEMACC_ADDRESS => {
                        memacc_address = value;
                        continue;
                    }
                    CC_MEMACC | CC_MEMACC_ALT => {
                        let op = match memacc_op {
                            0 => MemOp::Set,
                            1 => MemOp::Add,
                            2 => MemOp::Sub,
                            3 => MemOp::SetFromMemory,
                            4 => MemOp::AddFromMemory,
                            5 => MemOp::SubFromMemory,
                            other => {
                                report.warn(
                                    f,
                                    format!("track {c}: MEMACC operation {other} at {} isn't one MIDI can say (0..=5); skipped", at(tick)),
                                );
                                continue;
                            }
                        };
                        Command::MemAcc { op, address: memacc_address, operand: value }
                    }
                    CC_XCMD => {
                        xcmd = Some(value);
                        continue;
                    }
                    CC_XCMD_VALUE | CC_XCMD_VALUE_ALT => match xcmd {
                        Some(XCMD_ECHO_VOLUME) => Command::EchoVolume(value),
                        Some(XCMD_ECHO_LENGTH) => Command::EchoLength(value),
                        other => {
                            report.warn(f, format!("track {c}: extended command {other:?} at {} isn't supported; skipped", at(tick)));
                            continue;
                        }
                    },
                    other => {
                        report.note(f, format!("track {c}: controller {other} at {} means nothing to M4A; skipped", at(tick)));
                        continue;
                    }
                };
                push!(command);
            }
            Message::Program { program, .. } => push!(Command::Voice(program)),
            Message::PitchBend { value, .. } => {
                if value & 0x7F != 0 {
                    bend_rounded += 1;
                }
                push!(Command::Bend(((value + 0x40) >> 7).min(127) as u8));
            }
            Message::Tempo(us) => {
                let (v, exact) = tempo_from_us(us);
                if !exact {
                    report.warn(
                        f,
                        format!(
                            "tempo {:.2} BPM at {} isn't an even whole BPM; M4A plays {} BPM",
                            60_000_000.0 / us as f64,
                            at(tick),
                            2 * v as u32
                        ),
                    );
                }
                push!(Command::Tempo(v));
            }
            _ => {}
        }
    }
    if bend_rounded > 0 {
        report.warn(f, format!("track {c}: {bend_rounded} pitch bends used MIDI's fine bits; M4A bends in 128 steps"));
    }
    // Notes never released are ties without an end.
    let ending = match span {
        Some((start_tick, end_tick)) => {
            let start = out.partition_point(|e| e.tick < start_tick);
            Ending::Loop { start, start_tick, end_tick }
        }
        None => Ending::Fine { tick: fine },
    };
    Some(Timeline { events: out, ending })
}

