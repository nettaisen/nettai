//! Standard MIDI files: the events M4A songs use, read and written with
//! absolute times. Formats 0 and 1 read; format 1 is written.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    NoteOn { channel: u8, key: u8, velocity: u8 },
    NoteOff { channel: u8, key: u8, velocity: u8 },
    Control { channel: u8, controller: u8, value: u8 },
    Program { channel: u8, program: u8 },
    /// 14-bit, 0x2000 = center.
    PitchBend { channel: u8, value: u16 },
    /// Microseconds per quarter note.
    Tempo(u32),
    Marker(String),
    TrackName(String),
    Text(String),
    TimeSignature([u8; 4]),
    /// Anything else (kept when read, not interpreted).
    Other(Vec<u8>),
}

impl Message {
    pub fn channel(&self) -> Option<u8> {
        match *self {
            Message::NoteOn { channel, .. }
            | Message::NoteOff { channel, .. }
            | Message::Control { channel, .. }
            | Message::Program { channel, .. }
            | Message::PitchBend { channel, .. } => Some(channel),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub tick: u32,
    pub message: Message,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiTrack {
    /// In file order (ticks never decrease).
    pub events: Vec<Event>,
    /// The end-of-track event's time.
    pub end: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Smf {
    pub format: u16,
    /// Ticks per quarter note.
    pub division: u16,
    pub tracks: Vec<MidiTrack>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiError(pub String);

impl fmt::Display for MidiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MidiError {}

fn vlq(mut n: u32, out: &mut Vec<u8>) {
    let mut buf = [0u8; 5];
    let mut i = 4;
    buf[i] = (n & 0x7F) as u8;
    n >>= 7;
    while n > 0 {
        i -= 1;
        buf[i] = 0x80 | (n & 0x7F) as u8;
        n >>= 7;
    }
    out.extend_from_slice(&buf[i..]);
}

fn meta(kind: u8, data: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(&[0xFF, kind]);
    vlq(data.len() as u32, out);
    out.extend_from_slice(data);
}

impl Smf {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = b"MThd".to_vec();
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&self.format.to_be_bytes());
        out.extend_from_slice(&(self.tracks.len() as u16).to_be_bytes());
        out.extend_from_slice(&self.division.to_be_bytes());
        for t in &self.tracks {
            let mut d = Vec::new();
            let mut now = 0;
            for e in &t.events {
                vlq(e.tick - now, &mut d);
                now = e.tick;
                match &e.message {
                    Message::NoteOn { channel, key, velocity } => d.extend_from_slice(&[0x90 | channel, *key, *velocity]),
                    Message::NoteOff { channel, key, velocity } => d.extend_from_slice(&[0x80 | channel, *key, *velocity]),
                    Message::Control { channel, controller, value } => {
                        d.extend_from_slice(&[0xB0 | channel, *controller, *value])
                    }
                    Message::Program { channel, program } => d.extend_from_slice(&[0xC0 | channel, *program]),
                    Message::PitchBend { channel, value } => {
                        d.extend_from_slice(&[0xE0 | channel, (value & 0x7F) as u8, (value >> 7) as u8])
                    }
                    Message::Tempo(us) => meta(0x51, &us.to_be_bytes()[1..], &mut d),
                    Message::Marker(s) => meta(0x06, s.as_bytes(), &mut d),
                    Message::TrackName(s) => meta(0x03, s.as_bytes(), &mut d),
                    Message::Text(s) => meta(0x01, s.as_bytes(), &mut d),
                    Message::TimeSignature(b) => meta(0x58, b, &mut d),
                    Message::Other(raw) => d.extend_from_slice(raw),
                }
            }
            vlq(t.end.saturating_sub(now), &mut d);
            meta(0x2F, &[], &mut d);
            out.extend_from_slice(b"MTrk");
            out.extend_from_slice(&(d.len() as u32).to_be_bytes());
            out.extend(d);
        }
        out
    }

    pub fn from_bytes(data: &[u8]) -> Result<Smf, MidiError> {
        let err = |s: &str| MidiError(s.to_string());
        if data.len() < 14 || &data[0..4] != b"MThd" {
            return Err(err("not a MIDI file"));
        }
        let be16 = |o: usize| u16::from_be_bytes([data[o], data[o + 1]]);
        let hlen = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
        let format = be16(8);
        let ntracks = be16(10);
        let division = be16(12);
        if division & 0x8000 != 0 {
            return Err(err("SMPTE time division; save the file with ticks per quarter note"));
        }
        if format > 1 {
            return Err(err("MIDI format 2 (independent sequences) isn't supported"));
        }
        let mut at = 8 + hlen;
        let mut tracks = Vec::new();
        while tracks.len() < ntracks as usize && at + 8 <= data.len() {
            let id = &data[at..at + 4];
            let len = u32::from_be_bytes(data[at + 4..at + 8].try_into().unwrap()) as usize;
            let body = data.get(at + 8..at + 8 + len).ok_or_else(|| err("truncated track"))?;
            at += 8 + len;
            if id != b"MTrk" {
                continue;
            }
            tracks.push(read_track(body)?);
        }
        Ok(Smf { format, division, tracks })
    }
}

fn read_track(b: &[u8]) -> Result<MidiTrack, MidiError> {
    let err = |s: &str| MidiError(s.to_string());
    let mut at = 0;
    let mut tick = 0u32;
    let mut running: Option<u8> = None;
    let mut events = Vec::new();
    let byte = |at: &mut usize| -> Result<u8, MidiError> {
        let v = *b.get(*at).ok_or_else(|| MidiError("truncated event".into()))?;
        *at += 1;
        Ok(v)
    };
    let read_vlq = |at: &mut usize| -> Result<u32, MidiError> {
        let mut n = 0u32;
        for _ in 0..4 {
            let v = *b.get(*at).ok_or_else(|| MidiError("truncated number".into()))?;
            *at += 1;
            n = (n << 7) | (v & 0x7F) as u32;
            if v & 0x80 == 0 {
                return Ok(n);
            }
        }
        Err(MidiError("bad variable-length number".into()))
    };
    while at < b.len() {
        tick += read_vlq(&mut at)?;
        let mut status = byte(&mut at)?;
        if status < 0x80 {
            status = running.ok_or_else(|| err("data byte without a status"))?;
            at -= 1;
        }
        let message = match status {
            0xFF => {
                let kind = byte(&mut at)?;
                let len = read_vlq(&mut at)? as usize;
                let d = b.get(at..at + len).ok_or_else(|| err("truncated meta event"))?;
                at += len;
                let text = || String::from_utf8_lossy(d).into_owned();
                match kind {
                    0x2F => return Ok(MidiTrack { events, end: tick }),
                    0x51 if len == 3 => Message::Tempo(u32::from_be_bytes([0, d[0], d[1], d[2]])),
                    0x06 => Message::Marker(text()),
                    0x03 => Message::TrackName(text()),
                    0x01 => Message::Text(text()),
                    0x58 if len == 4 => Message::TimeSignature(d.try_into().unwrap()),
                    _ => {
                        let mut raw = vec![0xFF, kind];
                        vlq(len as u32, &mut raw);
                        raw.extend_from_slice(d);
                        Message::Other(raw)
                    }
                }
            }
            0xF0 | 0xF7 => {
                let len = read_vlq(&mut at)? as usize;
                let d = b.get(at..at + len).ok_or_else(|| err("truncated sysex"))?;
                at += len;
                let mut raw = vec![status];
                vlq(len as u32, &mut raw);
                raw.extend_from_slice(d);
                Message::Other(raw)
            }
            _ => {
                running = Some(status);
                let channel = status & 0x0F;
                match status & 0xF0 {
                    0x80 => Message::NoteOff { channel, key: byte(&mut at)?, velocity: byte(&mut at)? },
                    0x90 => {
                        let (key, velocity) = (byte(&mut at)?, byte(&mut at)?);
                        if velocity == 0 {
                            Message::NoteOff { channel, key, velocity: 0 }
                        } else {
                            Message::NoteOn { channel, key, velocity }
                        }
                    }
                    0xB0 => Message::Control { channel, controller: byte(&mut at)?, value: byte(&mut at)? },
                    0xC0 => Message::Program { channel, program: byte(&mut at)? },
                    0xE0 => {
                        let (lo, hi) = (byte(&mut at)?, byte(&mut at)?);
                        Message::PitchBend { channel, value: lo as u16 | (hi as u16) << 7 }
                    }
                    0xA0 => Message::Other(vec![status, byte(&mut at)?, byte(&mut at)?]),
                    0xD0 => Message::Other(vec![status, byte(&mut at)?]),
                    _ => return Err(err("unknown status byte")),
                }
            }
        };
        events.push(Event { tick, message });
    }
    // No end-of-track event: the track ends at its last event.
    Ok(MidiTrack { events, end: tick })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let smf = Smf {
            format: 1,
            division: 24,
            tracks: vec![
                MidiTrack { events: vec![Event { tick: 0, message: Message::Tempo(400_000) }], end: 96 },
                MidiTrack {
                    events: vec![
                        Event { tick: 0, message: Message::Program { channel: 3, program: 7 } },
                        Event { tick: 0, message: Message::NoteOn { channel: 3, key: 60, velocity: 100 } },
                        Event { tick: 200, message: Message::NoteOff { channel: 3, key: 60, velocity: 0 } },
                        Event { tick: 300, message: Message::PitchBend { channel: 3, value: 0x2000 } },
                        Event { tick: 300, message: Message::Marker("[".into()) },
                    ],
                    end: 400,
                },
            ],
        };
        assert_eq!(Smf::from_bytes(&smf.to_bytes()).unwrap(), smf);
    }

    #[test]
    fn running_status_and_zero_velocity_note_on() {
        // MThd, then a track: note on 60, running-status note on 60 vel 0.
        let mut f = b"MThd\0\0\0\x06\0\0\0\x01\0\x60".to_vec();
        let body = [0x00, 0x90, 60, 90, 0x10, 60, 0, 0x00, 0xFF, 0x2F, 0x00];
        f.extend_from_slice(b"MTrk");
        f.extend_from_slice(&(body.len() as u32).to_be_bytes());
        f.extend_from_slice(&body);
        let smf = Smf::from_bytes(&f).unwrap();
        assert_eq!(smf.tracks[0].events[1], Event { tick: 16, message: Message::NoteOff { channel: 0, key: 60, velocity: 0 } });
        assert_eq!(smf.tracks[0].end, 16);
    }
}
