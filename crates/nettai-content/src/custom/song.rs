//! Custom songs: a standard MIDI file made into the M4A driver's song, as the GBA's own mid2agb makes one, played on
//! the instruments of a song the ROMs have.

use m4a::bank::{Command, Envelope, Sample, SampleId, Song, Track, Voice, VoiceKind};
use nettai_content_api::CustomInstrument;
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// The driver's ticks a beat (a quarter note): it counts `tempo` (half BPM) twice a frame against 150, 60 frames a
/// second.
const TICKS_PER_BEAT: u64 = 24;

/// The longest note one note command holds, in ticks (N96 and its gate's three ticks more); a longer one is a tie
/// and its end.
const LONGEST_NOTE: u32 = 99;

/// The longest wait one wait command holds (W96).
const LONGEST_WAIT: u32 = 96;

/// What a channel does at a tick, in the order a tick's are written: a tie's end, then a setting, then a note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    EndTie(u8),
    Set,
    Note,
}

/// The song in MIDI file `bytes`, on the instruments, player, priority and reverb of the ROMs' song `like`, whose
/// player plays at most `max_tracks` tracks: each channel with a note one track, in the channels' order, the first
/// with the tempo.
pub(super) fn song(bytes: &[u8], like: &Song, max_tracks: usize) -> Result<Song, String> {
    let smf = Smf::parse(bytes).map_err(|e| format!("not a standard MIDI file ({e})"))?;
    let Timing::Metrical(division) = smf.header.timing else {
        return Err("a MIDI file timed in frames of a time code, not in ticks a beat".to_string());
    };
    let division = u64::from(division.as_int().max(1));
    let tick = |t: u64| ((t * TICKS_PER_BEAT + division / 2) / division) as u32;
    // Every event at its time in the file's ticks, in the file's order.
    let mut tempos: Vec<(u64, u8)> = Vec::new();
    let (mut loop_start, mut loop_end) = (None, None);
    let mut channels: BTreeMap<u8, Vec<(u64, MidiMessage)>> = BTreeMap::new();
    for track in &smf.tracks {
        let mut t = 0u64;
        for e in track {
            t += u64::from(e.delta.as_int());
            match e.kind {
                TrackEventKind::Midi { channel, message } => channels.entry(channel.as_int()).or_default().push((t, message)),
                TrackEventKind::Meta(MetaMessage::Tempo(us)) => {
                    // (Half the beats a minute, as TEMPO takes it.)
                    let half_bpm = (30_000_000.0 / f64::from(us.as_int().max(1))).round().clamp(1.0, 255.0) as u8;
                    tempos.push((t, half_bpm));
                }
                TrackEventKind::Meta(MetaMessage::Marker(text) | MetaMessage::Text(text) | MetaMessage::CuePoint(text)) => {
                    match std::str::from_utf8(text).map(str::trim) {
                        Ok("[" | "loopStart") => loop_start = Some(t),
                        Ok("]" | "loopEnd") => loop_end = Some(t),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    let looped = match (loop_start.map(tick), loop_end.map(tick)) {
        (Some(start), Some(end)) if start < end => Some((start, end)),
        (None, None) => None,
        (Some(start), Some(end)) => return Err(format!("its loop ends (\"]\" at tick {end}) before it starts (\"[\" at tick {start})")),
        (Some(_), None) => return Err("its loop starts (\"[\") and never ends (\"]\")".to_string()),
        (None, Some(_)) => return Err("its loop ends (\"]\") and never starts (\"[\")".to_string()),
    };
    let cut = looped.map(|(_, end)| end);
    // Each channel's steps at the driver's ticks.
    let mut steps: Vec<Vec<(u32, Step, Command)>> = Vec::new();
    let mut end = 0u32;
    for (_, events) in channels {
        let mut out: Vec<(u32, Step, Command)> = Vec::new();
        let mut held: HashMap<u8, VecDeque<(u64, u8)>> = HashMap::new();
        let mut notes: Vec<(u32, u32, u8, u8)> = Vec::new();
        // (The registered parameter data entry sets: bend range is RPN 0.)
        let mut rpn = (127u8, 127u8);
        // (The extended command controller 30 picks, which 29 and 31 set, as mid2agb's: 8 the pseudo-echo's
        // volume, 9 its length; the channel's own.)
        let mut extended = 0u8;
        let last = events.last().map_or(0, |e| e.0);
        for &(t, message) in &events {
            let at = tick(t);
            let set = match message {
                MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                    held.entry(key.as_int()).or_default().push_back((t, vel.as_int()));
                    None
                }
                MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                    if let Some((from, vel)) = held.get_mut(&key.as_int()).and_then(VecDeque::pop_front) {
                        notes.push((tick(from), at, key.as_int(), vel));
                    }
                    None
                }
                MidiMessage::ProgramChange { program } => Some(Command::Voice(program.as_int())),
                MidiMessage::PitchBend { bend } => Some(Command::Bend((bend.0.as_int() >> 7) as u8)),
                MidiMessage::Controller { controller, value } => {
                    let v = value.as_int();
                    match controller.as_int() {
                        1 => Some(Command::Modulation(v)),
                        7 => Some(Command::Volume(v)),
                        10 => Some(Command::Pan(v)),
                        20 => Some(Command::BendRange(v)),
                        21 => Some(Command::LfoSpeed(v)),
                        22 => Some(Command::ModulationType(v)),
                        24 => Some(Command::Tune(v)),
                        26 => Some(Command::LfoDelay(v)),
                        101 => {
                            rpn.0 = v;
                            None
                        }
                        100 => {
                            rpn.1 = v;
                            None
                        }
                        6 if rpn == (0, 0) => Some(Command::BendRange(v)),
                        30 => {
                            extended = v;
                            None
                        }
                        29 | 31 => match extended {
                            8 => Some(Command::EchoVolume(v)),
                            9 => Some(Command::EchoLength(v)),
                            _ => None,
                        },
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(c) = set {
                out.push((at, Step::Set, c));
            }
        }
        // (A note never released ends with the channel's last event.)
        for (key, queue) in held {
            for (from, vel) in queue {
                notes.push((tick(from), tick(last), key, vel));
            }
        }
        if notes.is_empty() {
            continue;
        }
        for (from, to, key, velocity) in notes {
            let to = cut.map_or(to, |c| to.min(c)).max(from + 1);
            if cut.is_some_and(|c| from >= c) {
                continue;
            }
            end = end.max(to);
            let gate = to - from;
            if gate <= LONGEST_NOTE {
                out.push((from, Step::Note, Command::Note { gate: gate as u8, key: Some(key), velocity: Some(velocity) }));
            } else {
                out.push((from, Step::Note, Command::Note { gate: 0, key: Some(key), velocity: Some(velocity) }));
                out.push((to, Step::EndTie(key), Command::EndTie { key: Some(key) }));
            }
        }
        end = end.max(out.iter().map(|s| s.0).max().unwrap_or(0));
        steps.push(out);
    }
    if steps.is_empty() {
        return Err("no notes".to_string());
    }
    if steps.len() > max_tracks {
        return Err(format!("{} channels with notes, and its player plays {max_tracks} tracks", steps.len()));
    }
    for (t, tempo) in tempos {
        steps[0].push((tick(t), Step::Set, Command::Tempo(tempo)));
    }
    let end = cut.unwrap_or(end);
    let tracks = steps
        .into_iter()
        .map(|mut s| {
            // (Stable: a tick's settings stay in the file's order, the tempo after the channel's.)
            s.sort_by_key(|&(at, step, _)| (at, step));
            Track { commands: commands(s.into_iter().filter(|&(at, ..)| at <= end).collect(), end, looped.map(|(start, _)| start)) }
        })
        .collect();
    Ok(Song { player: like.player, priority: like.priority, reverb: like.reverb, voicegroup: like.voicegroup, tracks })
}

/// The sample of instrument `def`, the WAV recording `bytes`: its channels mixed to one, as the driver's signed 8-bit
/// samples (each sample's top 8 bits, or a float's scaled), at the rate that plays its key `def.key` at its pitch.
pub(super) fn sample(bytes: &[u8], def: &CustomInstrument) -> Result<Sample, String> {
    let mut wav = hound::WavReader::new(bytes).map_err(|e| format!("not a WAV recording ({e})"))?;
    let spec = wav.spec();
    let channels = usize::from(spec.channels.max(1));
    let values: Vec<f64> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let full = f64::from(1u32 << (spec.bits_per_sample.clamp(1, 32) - 1));
            wav.samples::<i32>().map(|v| v.map(|v| f64::from(v) / full)).collect::<Result<_, _>>()
        }
        hound::SampleFormat::Float => wav.samples::<f32>().map(|v| v.map(f64::from)).collect::<Result<_, _>>(),
    }
    .map_err(|e| format!("a WAV recording that doesn't read ({e})"))?;
    let data: Vec<i8> =
        values.chunks(channels).map(|c| (c.iter().sum::<f64>() / c.len() as f64 * 128.0).round().clamp(-128.0, 127.0) as i8).collect();
    if data.is_empty() {
        return Err("a WAV recording with no samples".to_string());
    }
    if let Some(l) = def.loop_start.filter(|&l| l as usize >= data.len()) {
        return Err(format!("its loop starts at sample {l}, and it has {}", data.len()));
    }
    // (The driver's rate is a sample's at key 60, in 1/1024 Hz.)
    let rate = f64::from(spec.sample_rate) * 2f64.powf((60.0 - f64::from(def.key)) / 12.0) * 1024.0;
    let tail = Sample::usual_tail(&data, def.loop_start);
    Ok(Sample { rate: rate.round().clamp(1.0, f64::from(u32::MAX)) as u32, loop_start: def.loop_start, data, tail })
}

/// A Direct Sound voice of sample `sample`, its envelope `def`'s.
pub(super) fn voice(sample: SampleId, def: &CustomInstrument) -> Voice {
    let [attack, decay, sustain, release] = def.envelope;
    Voice { kind: VoiceKind::DirectSound { sample, fixed: false }, key: 60, pan: None, length: 0, envelope: Envelope { attack, decay, sustain, release } }
}

/// A track's commands: its steps (in order) with waits between them to `end`, then a jump back to where it was at
/// tick `loop_start`, or with none its end.
fn commands(steps: Vec<(u32, Step, Command)>, end: u32, loop_start: Option<u32>) -> Vec<Command> {
    let mut out = Vec::new();
    let mut now = 0u32;
    let mut back = None;
    let mut wait_to = |to: u32, out: &mut Vec<Command>, back: &mut Option<usize>| {
        while now < to {
            // (The loop's start is a place in the track: a wait that passes it stops there.)
            let stop = match loop_start {
                Some(s) if back.is_none() && s > now && s < to => s,
                _ => to,
            };
            while now < stop {
                let n = (stop - now).min(LONGEST_WAIT);
                out.push(Command::Wait(n as u8));
                now += n;
            }
            if loop_start == Some(now) && back.is_none() {
                *back = Some(out.len());
            }
        }
    };
    if loop_start == Some(0) {
        back = Some(0);
    }
    for (at, _, c) in steps {
        wait_to(at, &mut out, &mut back);
        out.push(c);
    }
    wait_to(end, &mut out, &mut back);
    match back {
        Some(i) => out.push(Command::Goto(i as u32)),
        None => out.push(Command::Fine),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use midly::num::{u4, u7, u15, u28};

    /// A WAV recording at `rate` Hz of `channels` channels, its samples 16-bit.
    pub(crate) fn wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let spec = hound::WavSpec { channels, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut out = std::io::Cursor::new(Vec::new());
        let mut w = hound::WavWriter::new(&mut out, spec).unwrap();
        for &v in samples {
            w.write_sample(v).unwrap();
        }
        w.finalize().unwrap();
        out.into_inner()
    }

    /// A recording is the driver's sample: its channels mixed, 8 bits, at the rate that plays its key at its pitch
    /// (a recording of A4, key 69, at 22050 Hz plays at key 60 nine semitones lower).
    #[test]
    fn a_recording_is_a_sample() {
        let def = CustomInstrument { key: 69, loop_start: Some(1), ..Default::default() };
        let s = sample(&wav(22050, 2, &[0x4000, 0x2000, -0x8000, -0x8000, 0x7FFF, 0x7FFF]), &def).unwrap();
        assert_eq!(s.data, [48, -128, 127]);
        assert_eq!((s.loop_start, s.tail), (Some(1), -128));
        assert_eq!(s.rate, (22050.0 * 1024.0 * 2f64.powf(-9.0 / 12.0)).round() as u32);
        let e = sample(&wav(8000, 1, &[1, 2]), &CustomInstrument { loop_start: Some(2), ..Default::default() }).unwrap_err();
        assert_eq!(e, "its loop starts at sample 2, and it has 2");
        assert!(sample(b"RIFF", &def).unwrap_err().starts_with("not a WAV recording"));
        let v = voice(SampleId(9), &CustomInstrument { envelope: [200, 10, 128, 250], ..Default::default() });
        assert_eq!((v.kind, v.envelope.release), (VoiceKind::DirectSound { sample: SampleId(9), fixed: false }, 250));
    }
    use midly::{Format, Header, TrackEvent};

    /// A MIDI file of 96 ticks a beat: its tracks' events, each (delta, event).
    fn midi(tracks: Vec<Vec<(u32, TrackEventKind<'static>)>>) -> Vec<u8> {
        let mut smf = Smf::new(Header::new(Format::Parallel, Timing::Metrical(u15::new(96))));
        for t in tracks {
            let mut events: Vec<TrackEvent> = t.into_iter().map(|(d, kind)| TrackEvent { delta: u28::new(d), kind }).collect();
            events.push(TrackEvent { delta: u28::new(0), kind: TrackEventKind::Meta(MetaMessage::EndOfTrack) });
            smf.tracks.push(events);
        }
        let mut out = Vec::new();
        smf.write_std(&mut out).unwrap();
        out
    }

    fn on(ch: u8, key: u8, vel: u8) -> TrackEventKind<'static> {
        TrackEventKind::Midi { channel: u4::new(ch), message: MidiMessage::NoteOn { key: u7::new(key), vel: u7::new(vel) } }
    }
    fn off(ch: u8, key: u8) -> TrackEventKind<'static> {
        TrackEventKind::Midi { channel: u4::new(ch), message: MidiMessage::NoteOff { key: u7::new(key), vel: u7::new(0) } }
    }
    fn cc(ch: u8, controller: u8, value: u8) -> TrackEventKind<'static> {
        TrackEventKind::Midi { channel: u4::new(ch), message: MidiMessage::Controller { controller: u7::new(controller), value: u7::new(value) } }
    }
    fn program(ch: u8, p: u8) -> TrackEventKind<'static> {
        TrackEventKind::Midi { channel: u4::new(ch), message: MidiMessage::ProgramChange { program: u7::new(p) } }
    }
    fn marker(text: &'static str) -> TrackEventKind<'static> {
        TrackEventKind::Meta(MetaMessage::Marker(text.as_bytes()))
    }

    fn like() -> Song {
        Song { player: m4a::PlayerId(0), priority: 3, reverb: Some(40), voicegroup: m4a::bank::VoicegroupId(7), tracks: Vec::new() }
    }

    fn note(gate: u8, key: u8, velocity: u8) -> Command {
        Command::Note { gate, key: Some(key), velocity: Some(velocity) }
    }

    /// Each channel is a track at 24 ticks a beat: its settings, its notes (a chord's at one tick), waits between;
    /// the first track has the tempo; the song takes its instruments, player, priority and reverb from the ROMs'.
    #[test]
    fn a_midi_file_is_the_drivers_song() {
        let bytes = midi(vec![
            vec![(0, TrackEventKind::Meta(MetaMessage::Tempo(midly::num::u24::new(500_000))))],
            vec![(0, program(0, 3)), (0, cc(0, 7, 100)), (0, on(0, 60, 90)), (0, on(0, 64, 80)), (96, off(0, 60)), (0, off(0, 64)), (48, on(0, 67, 70)), (12, off(0, 67))],
            vec![(0, cc(1, 10, 20)), (192, on(1, 36, 127)), (24, off(1, 36))],
        ]);
        let s = song(&bytes, &like(), 16).unwrap();
        assert_eq!((s.player, s.priority, s.reverb, s.voicegroup), (m4a::PlayerId(0), 3, Some(40), m4a::bank::VoicegroupId(7)));
        assert_eq!(
            s.tracks[0].commands,
            [Command::Voice(3), Command::Volume(100), Command::Tempo(60), note(24, 60, 90), note(24, 64, 80), Command::Wait(36), note(3, 67, 70), Command::Wait(18), Command::Fine]
        );
        assert_eq!(s.tracks[1].commands, [Command::Pan(20), Command::Wait(48), note(6, 36, 127), Command::Wait(6), Command::Fine]);
    }

    /// Controller 30 picks an extended command and 29 or 31 sets it, as mid2agb reads them: 8 the pseudo-echo's
    /// volume, 9 its length; another is passed by, and a channel's pick is its own.
    #[test]
    fn the_extended_commands_set_the_echo() {
        let bytes = midi(vec![vec![
            (0, cc(0, 30, 8)),
            (0, cc(0, 29, 40)),
            (0, cc(0, 30, 9)),
            (0, cc(0, 31, 12)),
            (0, cc(0, 30, 5)),
            (0, cc(0, 29, 1)),
            (0, cc(1, 29, 30)),
            (0, on(0, 60, 64)),
            (0, on(1, 62, 64)),
            (24, off(0, 60)),
            (0, off(1, 62)),
        ]]);
        let s = song(&bytes, &like(), 16).unwrap();
        assert_eq!(s.tracks[0].commands, [Command::EchoVolume(40), Command::EchoLength(12), note(6, 60, 64), Command::Wait(6), Command::Fine]);
        assert_eq!(s.tracks[1].commands, [note(6, 62, 64), Command::Wait(6), Command::Fine]);
    }

    /// A note longer than one note command holds is a tie and its end; a wait longer than one command holds is
    /// several.
    #[test]
    fn a_long_note_is_a_tie() {
        let s = song(&midi(vec![vec![(0, on(2, 50, 100)), (96 * 5, off(2, 50))]]), &like(), 16).unwrap();
        assert_eq!(s.tracks[0].commands, [note(0, 50, 100), Command::Wait(96), Command::Wait(24), Command::EndTie { key: Some(50) }, Command::Fine]);
    }

    /// The markers "[" and "]" are where it loops from and back to: its tracks jump back there at the loop's end,
    /// a note past the end is cut there, and what follows the end is left out.
    #[test]
    fn markers_loop_it() {
        let bytes = midi(vec![
            vec![(24, marker("[")), (96 * 2, marker("]"))],
            vec![(0, cc(0, 7, 90)), (0, on(0, 60, 64)), (48, off(0, 60)), (0, on(0, 62, 64)), (192, off(0, 62)), (0, on(0, 64, 64)), (24, off(0, 64))],
        ]);
        let s = song(&bytes, &like(), 16).unwrap();
        // The loop is ticks 6 to 54: its start, after the first wait, is command 3; the second note (ticks 12 to 60)
        // cut to end there, the third (from 60) left out.
        assert_eq!(
            s.tracks[0].commands,
            [Command::Volume(90), note(12, 60, 64), Command::Wait(6), Command::Wait(6), note(42, 62, 64), Command::Wait(42), Command::Goto(3)]
        );
        let e = song(&midi(vec![vec![(0, marker("[")), (0, on(0, 60, 64)), (24, off(0, 60))]]), &like(), 16).unwrap_err();
        assert_eq!(e, "its loop starts (\"[\") and never ends (\"]\")");
    }

    /// What a MIDI file can't be is said: not one, no notes, more channels than its player's tracks.
    #[test]
    fn a_song_says_what_its_file_isnt() {
        assert!(song(b"MThd", &like(), 16).unwrap_err().starts_with("not a standard MIDI file"));
        assert_eq!(song(&midi(vec![vec![(0, cc(0, 7, 1))]]), &like(), 16).unwrap_err(), "no notes");
        let three = midi(vec![(0..3).flat_map(|ch| [(0, on(ch, 60, 1)), (1, off(ch, 60))]).collect()]);
        assert_eq!(song(&three, &like(), 2).unwrap_err(), "3 channels with notes, and its player plays 2 tracks");
    }
}
