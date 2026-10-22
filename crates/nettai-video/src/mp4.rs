//! A video's MP4, written as it is made: its samples go to its writer (a file; in a browser, the file being made in
//! the page's own storage) a chunk at a time as they come, and of each only its size and times are kept. Once every
//! sample is written, the head that goes before them is made ([`Made`]): the `ftyp`, the `moov` that indexes every
//! sample, and the `mdat`'s header. The host puts the head first in the file, so a player reads the index before
//! the samples: it knows the video's length and seeks at once, and plays a download from its start. It is no
//! fragmented MP4: every player and editor takes it.
//!
//! Its tracks are the encoder's ([`crate::encode`]): the picture, and a sound track a console shown, titled with its
//! player's name, the first the one played (the others its alternates); its chapters are the rounds (Nero's
//! `chpl`, which ffmpeg, VLC and mpv read).
//!
//! The samples are written in chunks of about two seconds from a key frame of the picture: every track's samples
//! since the last chunk, one track's after another's. Each sample lasts what the encoder says, from its track's
//! first. A picture whose first frame is shown later than it is decoded (B-frames) has an edit list that shows it
//! from its start.

use crate::Error;
use crate::encode::{Event, Kind as TrackKind, Packet, Track};
use std::io::Write;

/// How long a chunk is, at least, in seconds (it ends at a key frame of the picture past that).
const CHUNK_SECONDS: f64 = 2.0;

/// The movie's time scale (its header's, and its tracks' edit lists').
pub const MOVIE_TIMESCALE: u32 = 1000;

/// The `mdat`'s header: its size in 64 bits (the samples may pass 4 GB).
const MDAT_HEADER: u64 = 16;

/// A chapter: its title and its start, in seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct Chapter {
    pub title: String,
    pub start: f64,
}

/// An MP4 made: its samples went to `body` as they were made, and `head` goes before them in the file.
pub struct Made<W> {
    pub head: Vec<u8>,
    pub body: W,
}

/// The MP4 being written to `W`.
pub struct Muxer<W: Write> {
    to: W,
    /// The sound tracks' titles, in their order.
    titles: Vec<String>,
    chapters: Vec<Chapter>,
    tracks: Option<Vec<Track>>,
    /// Whether the picture's first packet has come.
    began: bool,
    /// Packets that came before the tracks were said.
    early: Vec<Packet>,
    /// Each track's packets since the last chunk.
    pending: Vec<Vec<Packet>>,
    /// Each track's samples written.
    index: Vec<Index>,
    /// How many bytes of samples are written.
    written: u64,
}

/// A track's samples as they are written: what its index says of them.
#[derive(Default)]
struct Index {
    /// The first one's decoding time, in the track's time scale.
    start: Option<u64>,
    /// How long the ones so far last, together.
    decoded: u64,
    /// Where the last of them to be shown ends, from the first's decoding.
    end: u64,
    /// Each one's size in bytes.
    sizes: Vec<u32>,
    /// Their durations, in runs of one: how many, and the duration.
    durations: Vec<(u32, u32)>,
    /// How much later than it is decoded each is shown, in runs of one: how many, and by how much.
    offsets: Vec<(u32, i32)>,
    /// The ones that start on their own (key frames), counted from 1.
    keys: Vec<u32>,
    /// Each chunk: its first byte, from the samples' start, and how many samples it has.
    chunks: Vec<(u64, u32)>,
}

impl Index {
    /// `packets`, a chunk of them at byte `at` of the samples.
    fn add(&mut self, at: u64, packets: &[Packet]) {
        self.chunks.push((at, packets.len() as u32));
        for p in packets {
            self.start.get_or_insert(p.time);
            self.end = self.end.max((self.decoded as i64 + p.offset as i64 + p.duration as i64).max(0) as u64);
            self.decoded += p.duration as u64;
            self.sizes.push(p.data.len() as u32);
            run(&mut self.durations, p.duration);
            run(&mut self.offsets, p.offset);
            if p.key {
                self.keys.push(self.sizes.len() as u32);
            }
        }
    }
}

/// `value` added to `runs`: one more of the last run, if it is of it.
fn run<T: PartialEq + Copy>(runs: &mut Vec<(u32, T)>, value: T) {
    match runs.last_mut() {
        Some((count, last)) if *last == value => *count += 1,
        _ => runs.push((1, value)),
    }
}

impl<W: Write> Muxer<W> {
    /// An MP4 written to `to`, its sound tracks titled `titles`, its chapters `chapters`.
    pub fn new(to: W, titles: Vec<String>, chapters: Vec<Chapter>) -> Muxer<W> {
        Muxer { to, titles, chapters, tracks: None, began: false, early: Vec::new(), pending: Vec::new(), index: Vec::new(), written: 0 }
    }

    /// What the encoder made.
    pub fn event(&mut self, event: Event) -> Result<(), Error> {
        match event {
            Event::Tracks(tracks) => {
                if self.tracks.is_some() {
                    return Err(Error::Encoder("the encoder said its tracks twice".into()));
                }
                self.pending = vec![Vec::new(); tracks.len()];
                self.index = tracks.iter().map(|_| Index::default()).collect();
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

    /// Every sample is made: the last chunk written, and the head to go before them made.
    pub fn finish(mut self) -> Result<Made<W>, Error> {
        if self.tracks.is_none() || !self.began {
            return Err(Error::Encoder("the encoder made no picture".into()));
        }
        self.chunk()?;
        self.to.flush().map_err(Error::Write)?;
        // (The chunks' places count from the file's start, past the head, whose length doesn't depend on them.)
        let length = self.head(0)?.len() as u64;
        let head = self.head(length)?;
        debug_assert_eq!(head.len() as u64, length);
        Ok(Made { head, body: self.to })
    }

    fn packet(&mut self, packet: Packet) -> Result<(), Error> {
        let tracks = self.tracks.as_mut().expect("said");
        let Some(track) = tracks.get_mut(packet.track) else {
            return Err(Error::Encoder(format!("a packet of track {} of {}", packet.track, tracks.len())));
        };
        let (video, timescale) = (matches!(track.kind, TrackKind::Video { .. }), track.timescale);
        if !self.began && video {
            // (The picture shown from its first frame: past what it is decoded ahead of it, if anything.)
            let shown = packet.time as i64 + packet.offset as i64;
            if shown > 0 && track.edits.is_none() {
                let entry = mp4_atom::ElstEntry { segment_duration: 0, media_time: Some(shown as u64), media_rate: mp4_atom::FixedPoint::new(1, 0) };
                track.edits = Some(mp4_atom::Edts { elst: Some(mp4_atom::Elst { entries: vec![entry] }) });
            }
            self.began = true;
        }
        // (A chunk ends where the picture has a key frame past its length.)
        if video && packet.key {
            let pending = &self.pending[packet.track];
            let length = pending.last().zip(pending.first()).map_or(0, |(last, first)| last.time + last.duration as u64 - first.time);
            if length as f64 >= CHUNK_SECONDS * timescale as f64 {
                self.chunk()?;
            }
        }
        self.pending[packet.track].push(packet);
        Ok(())
    }

    /// The packets since the last chunk written: each track's run of them after the one before it, and where
    /// each is kept.
    fn chunk(&mut self) -> Result<(), Error> {
        let count = self.pending.len();
        let pending = std::mem::replace(&mut self.pending, vec![Vec::new(); count]);
        let mut data = Vec::with_capacity(pending.iter().flatten().map(|p| p.data.len()).sum());
        for (index, packets) in self.index.iter_mut().zip(&pending).filter(|(_, packets)| !packets.is_empty()) {
            index.add(self.written + data.len() as u64, packets);
            for p in packets {
                data.extend_from_slice(&p.data);
            }
        }
        if data.is_empty() {
            return Ok(());
        }
        self.written += data.len() as u64;
        self.to.write_all(&data).map_err(Error::Write)
    }

    /// The head, its samples from byte `at` of the file: `ftyp`, the `moov` of the tracks (each sample's size,
    /// times and place), and the `mdat`'s header.
    fn head(&self, at: u64) -> Result<Vec<u8>, Error> {
        use mp4_atom::*;
        let tracks = self.tracks.as_ref().expect("said");
        let mut brands: Vec<FourCC> = vec![b"isom".into(), b"iso2".into()];
        if tracks.iter().any(|t| matches!(t.codec, Codec::Avc1(_))) {
            brands.push(b"avc1".into());
        }
        brands.push(b"mp41".into());
        let ftyp = Ftyp { major_brand: b"isom".into(), minor_version: 0x200, compatible_brands: brands };
        let mut sounds = 0;
        let mut longest = 0;
        let mut trak = Vec::new();
        for (i, (track, index)) in tracks.iter().zip(&self.index).enumerate() {
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
            let edits = edits(track, index);
            let shown = match &edits {
                Some(edts) => edts.elst.iter().flat_map(|l| &l.entries).map(|e| e.segment_duration).sum(),
                None => in_movie(index.end, track.timescale),
            };
            longest = longest.max(shown);
            let mut chunks: Vec<StscEntry> = Vec::new();
            for (n, &(_, samples)) in index.chunks.iter().enumerate() {
                if chunks.last().is_none_or(|last| last.samples_per_chunk != samples) {
                    chunks.push(StscEntry { first_chunk: n as u32 + 1, samples_per_chunk: samples, sample_description_index: 1 });
                }
            }
            let stbl = Stbl {
                stsd: Stsd { codecs: vec![track.codec.clone()] },
                stts: Stts { entries: index.durations.iter().map(|&(sample_count, sample_delta)| SttsEntry { sample_count, sample_delta }).collect() },
                // (Shown as decoded: no offsets.)
                ctts: index.offsets.iter().any(|&(_, offset)| offset != 0).then(|| Ctts {
                    entries: index.offsets.iter().map(|&(sample_count, offset)| CttsEntry { sample_count, sample_offset: offset as i64 }).collect(),
                }),
                // (Every sample a key one: none listed.)
                stss: (index.keys.len() < index.sizes.len()).then(|| Stss { entries: index.keys.clone() }),
                stsc: Stsc { entries: chunks },
                stsz: Stsz { samples: StszSamples::Different { sizes: index.sizes.clone() } },
                co64: Some(Co64 { entries: index.chunks.iter().map(|&(from, _)| at + from).collect() }),
                ..Default::default()
            };
            trak.push(Trak {
                tkhd: Tkhd {
                    track_id: i as u32 + 1,
                    duration: shown,
                    // (The first sound track is played; the others are its alternates.)
                    enabled: video || sounds == 1,
                    in_movie: true,
                    alternate_group: if video { 0 } else { 1 },
                    volume: if video { 0.into() } else { FixedPoint::new(1, 0) },
                    width: FixedPoint::new(width as u16, 0),
                    height: FixedPoint::new(height as u16, 0),
                    ..Default::default()
                },
                edts: edits,
                mdia: Mdia {
                    mdhd: Mdhd { timescale: track.timescale, duration: index.decoded, language: "und".into(), ..Default::default() },
                    hdlr: Hdlr { handler: if video { b"vide".into() } else { b"soun".into() }, name },
                    minf: Minf {
                        vmhd: video.then(Vmhd::default),
                        smhd: (!video).then(Smhd::default),
                        dinf: Dinf { dref: Dref { urls: vec![Url { location: String::new() }] } },
                        stbl,
                        ..Default::default()
                    },
                },
                ..Default::default()
            });
        }
        let moov = Moov {
            mvhd: Mvhd { timescale: MOVIE_TIMESCALE, duration: longest, rate: FixedPoint::new(1, 0), volume: FixedPoint::new(1, 0), next_track_id: tracks.len() as u32 + 1, ..Default::default() },
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
        out.extend_from_slice(&1u32.to_be_bytes());
        out.extend_from_slice(b"mdat");
        out.extend_from_slice(&(MDAT_HEADER + self.written).to_be_bytes());
        Ok(out)
    }
}

/// What of a track is shown, as its index says it: the encoder's edit list (or the muxer's, past the picture's
/// reordering), its times from the track's first sample (which the index decodes at 0) and each edit that runs to
/// the end (the encoder not knowing where that is) given its length. A track whose first sample comes after 0 and
/// that says nothing of what is shown starts that much in, by an empty edit.
fn edits(track: &Track, index: &Index) -> Option<mp4_atom::Edts> {
    use mp4_atom::{Edts, Elst, ElstEntry, FixedPoint};
    let start = index.start.unwrap_or(0);
    let said = track.edits.as_ref().and_then(|e| e.elst.as_ref()).map(|l| l.entries.clone()).unwrap_or_default();
    let entries = if said.is_empty() {
        if start == 0 {
            return None;
        }
        let rate = FixedPoint::new(1, 0);
        vec![
            ElstEntry { segment_duration: in_movie(start, track.timescale), media_time: None, media_rate: rate },
            ElstEntry { segment_duration: in_movie(index.end, track.timescale), media_time: Some(0), media_rate: rate },
        ]
    } else {
        said.into_iter()
            .map(|mut e| {
                if let Some(from) = &mut e.media_time {
                    *from = from.saturating_sub(start);
                    if e.segment_duration == 0 {
                        e.segment_duration = in_movie(index.end.saturating_sub(*from), track.timescale);
                    }
                }
                e
            })
            .collect()
    };
    Some(Edts { elst: Some(Elst { entries }) })
}

/// `time` in a time scale of `timescale` a second, in the movie's.
fn in_movie(time: u64, timescale: u32) -> u64 {
    ((time as u128 * MOVIE_TIMESCALE as u128 + timescale as u128 / 2) / timescale.max(1) as u128) as u64
}

fn atom_error(e: mp4_atom::Error) -> Error {
    Error::Encoder(format!("the MP4: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::Kind;
    use mp4_atom::{Decode, Moov};

    /// A track of `kind` in time scale `timescale`, its codec Opus's (the muxer takes any description as it is).
    fn track(kind: Kind, timescale: u32) -> Track {
        use mp4_atom::*;
        let audio = Audio { data_reference_index: 1, channel_count: 2, sample_size: 16, sample_rate: FixedPoint::new(48000, 0) };
        let codec = Codec::Opus(Opus { audio, dops: Dops { output_channel_count: 2, pre_skip: 312, input_sample_rate: 48000, output_gain: 0 }, btrt: None });
        Track { kind, timescale, codec, edits: None }
    }

    /// The `moov` of a head.
    fn moov_of(head: &[u8]) -> Moov {
        let ftyp = u32::from_be_bytes(head[..4].try_into().unwrap()) as usize;
        Moov::decode(&mut &head[ftyp..]).unwrap()
    }

    fn packet(track: usize, time: u64, offset: i32, duration: u32, key: bool, byte: u8) -> Event {
        Event::Packet(Packet { track, data: vec![byte; 3 + track], time, offset, duration, key })
    }

    /// The file a muxer makes: its head, then its samples.
    fn whole(made: Made<Vec<u8>>) -> Vec<u8> {
        let mut file = made.head;
        file.extend(made.body);
        file
    }

    /// The top-level atoms of `file`: each one's kind and its bytes.
    fn atoms(file: &[u8]) -> Vec<([u8; 4], &[u8])> {
        let mut out = Vec::new();
        let mut at = 0;
        while at < file.len() {
            let size = u32::from_be_bytes(file[at..at + 4].try_into().unwrap()) as usize;
            let kind = file[at + 4..at + 8].try_into().unwrap();
            let size = if size == 1 { u64::from_be_bytes(file[at + 8..at + 16].try_into().unwrap()) as usize } else { size };
            out.push((kind, &file[at..at + size]));
            at += size;
        }
        out
    }

    /// The head is the index of every sample, before them: no fragments; each sample's bytes are where its chunk's
    /// offset and the sizes before it in its chunk say; the durations are the encoder's; the picture's reordering
    /// is past by its edit list, given the track's length.
    #[test]
    fn the_index_comes_first_and_finds_every_sample() {
        let mut muxer = Muxer::new(Vec::new(), vec!["Lan".into()], vec![Chapter { title: "Round 1".into(), start: 0.0 }]);
        // (Picture at 60 a second in a scale of 60, its first shown 2 frames late; sound at 48 kHz, 1024 a packet.)
        muxer.event(Event::Tracks(vec![track(Kind::Video { width: 480, height: 320 }, 60), track(Kind::Audio, 48000)])).unwrap();
        let mut sent = Vec::new();
        for frame in 0..300u64 {
            let event = packet(0, frame, 2, 1, frame % 120 == 0, frame as u8);
            sent.push(event.clone());
            muxer.event(event).unwrap();
            if frame % 3 == 0 {
                let event = packet(1, frame / 3 * 1024, 0, 1024, true, 0xA0 ^ frame as u8);
                sent.push(event.clone());
                muxer.event(event).unwrap();
            }
        }
        let file = whole(muxer.finish().unwrap());
        let kinds: Vec<_> = atoms(&file).iter().map(|(k, _)| *k).collect();
        assert_eq!(kinds, [*b"ftyp", *b"moov", *b"mdat"]);
        let moov = Moov::decode(&mut &atoms(&file)[1].1[..]).unwrap();
        assert!(moov.mvex.is_none());
        // (300 frames from 2 in: 5 s. Sound: 100 packets of 1024, 2.133 s.)
        assert_eq!(moov.mvhd.duration, 5000);
        assert_eq!(moov.udta.unwrap().chpl.unwrap().entries.len(), 1);
        for (t, trak) in moov.trak.iter().enumerate() {
            let stbl = &trak.mdia.minf.stbl;
            let want: Vec<&Packet> = sent.iter().filter_map(|e| if let Event::Packet(p) = e { (p.track == t).then_some(p) } else { None }).collect();
            let sizes = match &stbl.stsz.samples {
                mp4_atom::StszSamples::Different { sizes } => sizes.clone(),
                other => panic!("{other:?}"),
            };
            assert_eq!(sizes.len(), want.len());
            // Each sample's bytes, found as a player finds them.
            let offsets = &stbl.co64.as_ref().unwrap().entries;
            let mut sample = 0;
            for (chunk, &offset) in offsets.iter().enumerate() {
                let per = stbl.stsc.entries.iter().rev().find(|e| e.first_chunk as usize <= chunk + 1).unwrap().samples_per_chunk;
                let mut at = offset as usize;
                for _ in 0..per {
                    assert_eq!(&file[at..at + sizes[sample] as usize], &want[sample].data[..], "track {t}, sample {sample}");
                    at += sizes[sample] as usize;
                    sample += 1;
                }
            }
            assert_eq!(sample, want.len());
            let durations: u64 = stbl.stts.entries.iter().map(|e| e.sample_count as u64 * e.sample_delta as u64).sum();
            assert_eq!(durations, want.iter().map(|p| p.duration as u64).sum::<u64>());
            assert_eq!(trak.mdia.mdhd.duration, durations);
            if t == 0 {
                assert_eq!(stbl.stss.as_ref().unwrap().entries, [1, 121, 241]);
                assert_eq!(stbl.ctts.as_ref().unwrap().entries.len(), 1);
                let edit = &trak.edts.as_ref().unwrap().elst.as_ref().unwrap().entries;
                assert_eq!((edit.len(), edit[0].media_time, edit[0].segment_duration), (1, Some(2), 5000));
                assert_eq!(trak.tkhd.duration, 5000);
            } else {
                assert!(stbl.stss.is_none() && stbl.ctts.is_none() && trak.edts.is_none());
                assert_eq!(trak.tkhd.duration, 2133);
            }
        }
    }

    /// A track that starts after 0 starts that much into the movie, by an empty edit.
    #[test]
    fn a_late_track_starts_late() {
        let mut muxer = Muxer::new(Vec::new(), vec![], vec![]);
        muxer.event(Event::Tracks(vec![track(Kind::Video { width: 2, height: 2 }, 60), track(Kind::Audio, 48000)])).unwrap();
        muxer.event(packet(0, 0, 0, 1, true, 1)).unwrap();
        muxer.event(packet(1, 4800, 0, 1024, true, 2)).unwrap();
        let made = muxer.finish().unwrap();
        let moov = moov_of(&made.head);
        assert!(moov.trak[0].edts.is_none());
        let edit = &moov.trak[1].edts.as_ref().unwrap().elst.as_ref().unwrap().entries;
        assert_eq!(edit.iter().map(|e| (e.media_time, e.segment_duration)).collect::<Vec<_>>(), [(None, 100), (Some(0), 21)]);
    }

    /// No picture, no file.
    #[test]
    fn no_picture_is_refused() {
        let mut muxer = Muxer::new(Vec::new(), vec![], vec![]);
        muxer.event(Event::Tracks(vec![track(Kind::Video { width: 2, height: 2 }, 60), track(Kind::Audio, 48000)])).unwrap();
        muxer.event(packet(1, 0, 0, 1024, true, 2)).unwrap();
        assert!(muxer.finish().is_err());
    }
}
