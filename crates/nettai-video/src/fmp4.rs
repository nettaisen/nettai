//! A video's MP4, written as it is made: fragmented, so each fragment goes to its writer (a file, the browser's
//! file being saved) as it is ready and nothing of the video is kept. Its tracks are the encoder's ([`crate::encode`]): the picture, and a sound track a console
//! shown, titled with its player's name, the first the one played (the others its alternates); its chapters are the
//! rounds (Nero's `chpl`, which ffmpeg, VLC and mpv read).
//!
//! The file is the init segment (`ftyp` and a `moov` of the tracks, their samples none) and then a `moof` and an
//! `mdat` a fragment, each fragment about two seconds from a key frame of the picture, every track's packets since
//! the last in it. The init segment is written with the picture's first packet: a picture whose first frame is shown
//! later than it is decoded (B-frames) has an edit list that shows it from its start.

use crate::Error;
use crate::encode::{Event, Kind as TrackKind, Packet, Track};
use std::io::Write;

/// How long a fragment is, at least, in seconds (it ends at a key frame of the picture past that).
const FRAGMENT_SECONDS: f64 = 2.0;

/// The movie's time scale (its header's, and its tracks' edit lists').
pub const MOVIE_TIMESCALE: u32 = 1000;

/// A chapter: its title and its start, in seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct Chapter {
    pub title: String,
    pub start: f64,
}

/// The MP4 being written to `W`.
pub struct Muxer<W: Write> {
    to: W,
    /// The sound tracks' titles, in their order.
    titles: Vec<String>,
    chapters: Vec<Chapter>,
    tracks: Option<Vec<Track>>,
    /// Whether the init segment is written.
    began: bool,
    /// Packets that came before the tracks were said.
    early: Vec<Packet>,
    /// Each track's packets since the last fragment.
    pending: Vec<Vec<Packet>>,
    sequence: u32,
}

impl<W: Write> Muxer<W> {
    /// An MP4 written to `to`, its sound tracks titled `titles`, its chapters `chapters`.
    pub fn new(to: W, titles: Vec<String>, chapters: Vec<Chapter>) -> Muxer<W> {
        Muxer { to, titles, chapters, tracks: None, began: false, early: Vec::new(), pending: Vec::new(), sequence: 0 }
    }

    /// What the encoder made.
    pub fn event(&mut self, event: Event) -> Result<(), Error> {
        match event {
            Event::Tracks(tracks) => {
                if self.tracks.is_some() {
                    return Err(Error::Encoder("the encoder said its tracks twice".into()));
                }
                self.pending = vec![Vec::new(); tracks.len()];
                self.tracks = Some(tracks);
                for packet in std::mem::take(&mut self.early) {
                    self.packet(packet)?;
                }
                Ok(())
            }
            Event::Packet(packet) if self.tracks.is_none() => {
                self.early.push(packet);
                Ok(())
            }
            Event::Packet(packet) => self.packet(packet),
        }
    }

    /// The video is whole: its last fragment written, and its writer given back.
    pub fn finish(mut self) -> Result<W, Error> {
        if self.tracks.is_none() || !self.began {
            return Err(Error::Encoder("the encoder made no picture".into()));
        }
        self.fragment()?;
        self.to.flush().map_err(Error::Write)?;
        Ok(self.to)
    }

    fn write(&mut self, bytes: Vec<u8>) -> Result<(), Error> {
        self.to.write_all(&bytes).map_err(Error::Write)
    }

    fn packet(&mut self, packet: Packet) -> Result<(), Error> {
        let tracks = self.tracks.as_ref().expect("said");
        let Some(track) = tracks.get(packet.track) else {
            return Err(Error::Encoder(format!("a packet of track {} of {}", packet.track, tracks.len())));
        };
        if !self.began && matches!(track.kind, TrackKind::Video { .. }) {
            // (The picture shown from its first frame: past what it is decoded ahead of it, if anything.)
            let shown = packet.time as i64 + packet.offset as i64;
            let mut tracks = tracks.clone();
            if shown > 0 && tracks[packet.track].edits.is_none() {
                let entry = mp4_atom::ElstEntry { segment_duration: 0, media_time: Some(shown as u64), media_rate: mp4_atom::FixedPoint::new(1, 0) };
                tracks[packet.track].edits = Some(mp4_atom::Edts { elst: Some(mp4_atom::Elst { entries: vec![entry] }) });
            }
            let init = self.init(&tracks)?;
            self.write(init)?;
            self.tracks = Some(tracks);
            self.began = true;
        }
        let track = &self.tracks.as_ref().expect("said")[packet.track];
        // (A fragment ends where the picture has a key frame past its length.)
        if matches!(track.kind, TrackKind::Video { .. }) && packet.key {
            let pending = &self.pending[packet.track];
            let length = pending.last().zip(pending.first()).map_or(0, |(last, first)| last.time + last.duration as u64 - first.time);
            if length as f64 >= FRAGMENT_SECONDS * track.timescale as f64 {
                self.fragment()?;
            }
        }
        self.pending[packet.track].push(packet);
        Ok(())
    }

    /// The init segment: `ftyp`, and the `moov` of the tracks (no samples: they are the fragments'), their
    /// fragments' defaults, and the chapters.
    fn init(&self, tracks: &[Track]) -> Result<Vec<u8>, Error> {
        use mp4_atom::*;
        let ftyp = Ftyp { major_brand: b"isom".into(), minor_version: 0x200, compatible_brands: vec![b"isom".into(), b"iso6".into(), b"mp41".into()] };
        let mut sounds = 0;
        let trak = tracks
            .iter()
            .enumerate()
            .map(|(i, track)| {
                let video = matches!(track.kind, TrackKind::Video { .. });
                let (width, height) = match track.kind {
                    TrackKind::Video { width, height } => (width, height),
                    TrackKind::Audio => (0, 0),
                };
                let name = if video {
                    "nettai".to_string()
                } else {
                    sounds += 1;
                    self.titles.get(sounds - 1).cloned().unwrap_or_default()
                };
                Trak {
                    tkhd: Tkhd {
                        track_id: i as u32 + 1,
                        // (The first sound track is played; the others are its alternates.)
                        enabled: video || sounds == 1,
                        in_movie: true,
                        alternate_group: if video { 0 } else { 1 },
                        volume: if video { 0.into() } else { FixedPoint::new(1, 0) },
                        width: FixedPoint::new(width as u16, 0),
                        height: FixedPoint::new(height as u16, 0),
                        ..Default::default()
                    },
                    edts: track.edits.clone(),
                    mdia: Mdia {
                        mdhd: Mdhd { timescale: track.timescale, language: "und".into(), ..Default::default() },
                        hdlr: Hdlr { handler: if video { b"vide".into() } else { b"soun".into() }, name },
                        minf: Minf {
                            vmhd: video.then(Vmhd::default),
                            smhd: (!video).then(Smhd::default),
                            dinf: Dinf { dref: Dref { urls: vec![Url { location: String::new() }] } },
                            stbl: Stbl { stsd: Stsd { codecs: vec![track.codec.clone()] }, stco: Some(Stco::default()), ..Default::default() },
                            ..Default::default()
                        },
                    },
                    ..Default::default()
                }
            })
            .collect();
        let moov = Moov {
            mvhd: Mvhd { timescale: MOVIE_TIMESCALE, rate: FixedPoint::new(1, 0), volume: FixedPoint::new(1, 0), next_track_id: tracks.len() as u32 + 1, ..Default::default() },
            mvex: Some(Mvex {
                trex: (1..=tracks.len() as u32).map(|track_id| Trex { track_id, default_sample_description_index: 1, ..Default::default() }).collect(),
                ..Default::default()
            }),
            trak,
            udta: (!self.chapters.is_empty()).then(|| Udta {
                chpl: Some(Chpl {
                    // (In 100 ns units.)
                    entries: self.chapters.iter().map(|c| ChplEntry { start: (c.start * 1e7).round() as u64, title: c.title.clone() }).collect(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut out = Vec::new();
        ftyp.encode(&mut out).map_err(atom_error)?;
        moov.encode(&mut out).map_err(atom_error)?;
        Ok(out)
    }

    /// The packets since the last fragment written as one: a `moof` (each track's run of samples, their times
    /// from its first's) and the `mdat` of their bytes.
    fn fragment(&mut self) -> Result<(), Error> {
        use mp4_atom::*;
        let count = self.pending.len();
        let pending = std::mem::replace(&mut self.pending, vec![Vec::new(); count]);
        if pending.iter().all(Vec::is_empty) {
            return Ok(());
        }
        let tracks = self.tracks.as_ref().expect("said");
        self.sequence += 1;
        let moof = |offsets: &[i32]| Moof {
            mfhd: Mfhd { sequence_number: self.sequence },
            traf: pending
                .iter()
                .enumerate()
                .filter(|(_, packets)| !packets.is_empty())
                .zip(offsets)
                .map(|((i, packets), &offset)| {
                    let video = matches!(tracks[i].kind, TrackKind::Video { .. });
                    Traf {
                        tfhd: Tfhd { track_id: i as u32 + 1, default_base_is_moof: true, ..Default::default() },
                        tfdt: Some(Tfdt { base_media_decode_time: packets[0].time }),
                        trun: vec![Trun {
                            data_offset: Some(offset),
                            entries: packets
                                .iter()
                                .map(|p| TrunEntry {
                                    duration: Some(p.duration),
                                    size: Some(p.data.len() as u32),
                                    // (Sync: depends on no other; else depends on others, and is no sync sample.)
                                    flags: Some(if !video || p.key { 0x0200_0000 } else { 0x0101_0000 }),
                                    // (A picture shown later than it is decoded: B-frames, x264's.)
                                    cts: video.then_some(p.offset),
                                })
                                .collect(),
                        }],
                        ..Default::default()
                    }
                })
                .collect(),
        };
        let runs = pending.iter().filter(|p| !p.is_empty()).count();
        let mut measured = Vec::new();
        moof(&vec![0; runs]).encode(&mut measured).map_err(atom_error)?;
        // (Each run's bytes from the moof's start: past the moof and the mdat's header, after the runs before it.)
        let mut at = measured.len() as i32 + 8;
        let offsets: Vec<i32> = pending
            .iter()
            .filter(|p| !p.is_empty())
            .map(|p| {
                let offset = at;
                at += p.iter().map(|s| s.data.len() as i32).sum::<i32>();
                offset
            })
            .collect();
        let mut out = Vec::new();
        moof(&offsets).encode(&mut out).map_err(atom_error)?;
        let data: Vec<u8> = pending.iter().flatten().flat_map(|p| p.data.iter().copied()).collect();
        Mdat { data }.encode(&mut out).map_err(atom_error)?;
        self.write(out)
    }
}

fn atom_error(e: mp4_atom::Error) -> Error {
    Error::Encoder(format!("the MP4: {e}"))
}
