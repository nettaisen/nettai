//! What the codecs made ([`Coded`]) made the MP4's tracks and samples ([`crate::mp4`]), in one place for every
//! platform:
//!
//! - **The picture** is a sample a frame, its H.264 made the MP4's form (`h264::sample`: each unit after its length,
//!   the parameter sets left out), at the frame's count on the tick's time base (the time scale the tick's
//!   denominator, a frame its numerator), exact: no codec reorders its frames, so each is decoded when it is shown.
//!   A key frame is one with an IDR slice. Its description is H.264's of the codec's first SPS and PPS (said apart,
//!   or in its first key frame), tagged BT.709 in the video range, as every codec is given it (`yuv`).
//! - **Each sound track** is a sample a coded frame, at the count of the samples before it: AAC's 1024 a frame,
//!   Opus's 960 (20 ms). An AAC track's description is its AudioSpecificConfig, and it is shown from past its
//!   encoder's priming (an edit list); Opus says its own lookahead (its pre-skip).
//! - **The tracks** are said once every one is described; the samples made before then wait in the muxer.

use crate::codecs::{Audio, Coded, Sound, Video};
use crate::mp4::{Event, Kind, Packet, Track};
use crate::{Error, aac, codecs, h264};

/// The samples a coded frame holds: AAC's, and Opus's at the 20 ms the browser's encoder makes.
const AAC_FRAME: u32 = 1024;
const OPUS_FRAME: u32 = 960;

/// The codecs' output being made the MP4's.
pub(crate) struct Assembler {
    video: Video,
    rate: u32,
    /// The picture's first SPS and PPS, and its frames so far.
    sets: Option<(Vec<u8>, Vec<u8>)>,
    frames: u64,
    sounds: Vec<SoundTrack>,
    said: bool,
}

/// A sound track: its coding once said, its frames so far, and those that came before its coding.
#[derive(Default)]
struct SoundTrack {
    coding: Option<Sound>,
    frames: u64,
    early: Vec<Vec<u8>>,
}

impl SoundTrack {
    fn packet(&mut self, track: usize, data: Vec<u8>, frame: u32) -> Packet {
        let time = self.frames * frame as u64;
        self.frames += 1;
        Packet { track: 1 + track, data, time, duration: frame, key: true }
    }
}

impl Assembler {
    pub(crate) fn new(video: Video, audio: Option<Audio>) -> Assembler {
        let tracks = audio.map_or(0, |a| a.tracks);
        let rate = audio.map_or(crate::resample::RATE, |a| a.sample_rate);
        Assembler { video, rate, sets: None, frames: 0, sounds: (0..tracks).map(|_| SoundTrack::default()).collect(), said: false }
    }

    /// What the codecs made, as the MP4's events: each frame a sample, and the tracks once every one is described.
    pub(crate) fn take(&mut self, coded: Vec<Coded>) -> Result<Vec<Event>, Error> {
        let mut events = Vec::new();
        for c in coded {
            match c {
                Coded::Sets(h264) => self.found_sets(&h264::units(&h264)),
                Coded::Frame(h264) => {
                    let units = h264::units(&h264);
                    self.found_sets(&units);
                    let frame = self.video.frame_time.0;
                    let time = self.frames * frame as u64;
                    self.frames += 1;
                    events.push(Event::Packet(Packet { track: 0, data: h264::sample(&units), time, duration: frame, key: h264::is_key(&units) }));
                }
                Coded::Coding(track, sound) => {
                    let s = self.sound(track)?;
                    if s.coding.is_some() {
                        return Err(Error::Encoder(format!("sound track {track}'s coding said twice")));
                    }
                    let frame = frame_of(&sound);
                    s.coding = Some(sound);
                    for data in std::mem::take(&mut s.early) {
                        events.push(Event::Packet(s.packet(track, data, frame)));
                    }
                }
                Coded::Sound(track, data) => {
                    let s = self.sound(track)?;
                    match s.coding.as_ref().map(frame_of) {
                        Some(frame) => events.push(Event::Packet(s.packet(track, data, frame))),
                        None => s.early.push(data),
                    }
                }
            }
        }
        if !self.said
            && let Some(tracks) = self.tracks()?
        {
            self.said = true;
            events.push(Event::Tracks(tracks));
        }
        Ok(events)
    }

    /// Every frame is made: what is still waiting (a sound track never described) is an error, as is a picture
    /// never described.
    pub(crate) fn finish(&self) -> Result<(), Error> {
        if self.sets.is_none() {
            return Err(Error::Encoder("the picture's codec said no parameter sets".into()));
        }
        match self.sounds.iter().position(|s| s.coding.is_none()) {
            Some(track) => Err(Error::Encoder(format!("sound track {track}'s codec said no coding"))),
            None => Ok(()),
        }
    }

    fn sound(&mut self, track: usize) -> Result<&mut SoundTrack, Error> {
        let tracks = self.sounds.len();
        self.sounds.get_mut(track).ok_or_else(|| Error::Encoder(format!("sound track {track} of {tracks}")))
    }

    fn found_sets(&mut self, units: &[&[u8]]) {
        if self.sets.is_none()
            && let (Some(sps), Some(pps)) = h264::sets(units)
        {
            self.sets = Some((sps.to_vec(), pps.to_vec()));
        }
    }

    /// The tracks, once every one is described: the picture's, then each sound track's.
    fn tracks(&self) -> Result<Option<Vec<Track>>, Error> {
        use mp4_atom::esds::{DecoderConfig, DecoderSpecific, EsDescriptor};
        use mp4_atom::{Avc1, Avcc, AvccExt, Codec, Colr, Dops, Edts, Elst, ElstEntry, Esds, FixedPoint, Mp4a, Opus, Visual};
        let Some((sps, pps)) = &self.sets else { return Ok(None) };
        if self.sounds.iter().any(|s| s.coding.is_none()) {
            return Ok(None);
        }
        let Video { width, height, frame_time: (_, timescale) } = self.video;
        let mut avcc = Avcc::new(sps, pps).map_err(|e| Error::Encoder(format!("the picture's parameter sets: {e}")))?;
        // (High's profiles say their chroma and bit depths: every picture here is 4:2:0 in 8 bits.)
        if matches!(avcc.avc_profile_indication, 100 | 110 | 122 | 144) {
            avcc.ext = Some(AvccExt::default());
        }
        let visual = Visual { data_reference_index: 1, width: width as u16, height: height as u16, ..Default::default() };
        // (BT.709 in the video range, as every codec is given the picture and its SPS says it.)
        let colr = Colr::Nclx { colour_primaries: 1, transfer_characteristics: 1, matrix_coefficients: 1, full_range_flag: false };
        let codec = Codec::Avc1(Avc1 { visual, avcc, colr: Some(colr), ..Default::default() });
        let mut tracks = vec![Track { kind: Kind::Video { width, height }, timescale, codec, edits: None }];
        let audio = || mp4_atom::Audio { data_reference_index: 1, channel_count: 2, sample_size: 16, sample_rate: FixedPoint::new(self.rate as u16, 0) };
        for (t, sound) in self.sounds.iter().enumerate() {
            let track = match sound.coding.as_ref().expect("described") {
                Sound::Aac { config, priming } => {
                    let fields = aac::Config::of(config).ok_or_else(|| Error::Encoder(format!("sound track {t}'s AudioSpecificConfig is {} bytes", config.len())))?;
                    let esds = Esds {
                        es_desc: EsDescriptor {
                            es_id: t as u16 + 2,
                            dec_config: DecoderConfig {
                                object_type_indication: 0x40,
                                stream_type: 5,
                                max_bitrate: codecs::SOUND_BITRATE,
                                avg_bitrate: codecs::SOUND_BITRATE,
                                dec_specific: Some(DecoderSpecific { profile: fields.object, freq_index: fields.rate, chan_conf: fields.channels, raw: config.clone() }),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    };
                    // (Shown from past the priming to its end: the muxer gives the edit its length.)
                    let entry = ElstEntry { segment_duration: 0, media_time: Some(*priming as u64), media_rate: FixedPoint::new(1, 0) };
                    Track {
                        kind: Kind::Audio,
                        timescale: self.rate,
                        codec: Codec::Mp4a(Mp4a { audio: audio(), esds, btrt: None, taic: None }),
                        edits: (*priming > 0).then(|| Edts { elst: Some(Elst { entries: vec![entry] }) }),
                    }
                }
                Sound::Opus { pre_skip } => {
                    let dops = Dops { output_channel_count: 2, pre_skip: *pre_skip, input_sample_rate: self.rate, output_gain: 0 };
                    Track { kind: Kind::Audio, timescale: self.rate, codec: Codec::Opus(Opus { audio: audio(), dops, btrt: None }), edits: None }
                }
            };
            tracks.push(track);
        }
        Ok(Some(tracks))
    }
}

/// The samples a coded frame of `sound` holds.
fn frame_of(sound: &Sound) -> u32 {
    match sound {
        Sound::Aac { .. } => AAC_FRAME,
        Sound::Opus { .. } => OPUS_FRAME,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::H264;

    /// The picture's frames are samples at their counts, key where they have an IDR slice, described by the first
    /// sets; a sound track's frames wait for its coding and are then samples of 1024, shown from past the priming;
    /// the tracks are said once all are described.
    #[test]
    fn the_codecs_output_is_made_the_mp4s() {
        let video = Video { width: 480, height: 320, frame_time: (4389, 262_144) };
        let mut a = Assembler::new(video, Some(Audio { sample_rate: 48_000, tracks: 1 }));
        // (High profile: an SPS of profile 100.)
        let key = [&[0, 0, 0, 1, 0x09, 0xF0][..], &[0, 0, 1, 0x67, 100, 0, 30, 1], &[0, 0, 1, 0x68, 2], &[0, 0, 1, 0x65, 7]].concat();
        let other = [&[0, 0, 0, 1, 0x09, 0xF0][..], &[0, 0, 1, 0x41, 8]].concat();
        let made = a.take(vec![Coded::Sound(0, vec![1]), Coded::Frame(H264::AnnexB(key)), Coded::Frame(H264::AnnexB(other))]).unwrap();
        let packets: Vec<&Packet> = made.iter().filter_map(|e| if let Event::Packet(p) = e { Some(p) } else { None }).collect();
        assert_eq!(packets.len(), 2, "the sound waits for its coding");
        assert_eq!((packets[0].time, packets[0].duration, packets[0].key, &packets[0].data[..]), (0, 4389, true, &[0, 0, 0, 2, 0x65, 7][..]));
        assert_eq!((packets[1].time, packets[1].key), (4389, false));
        assert!(!made.iter().any(|e| matches!(e, Event::Tracks(_))), "a track isn't described yet");
        let config = aac::Config::lc(48_000, 2).bytes();
        let made = a.take(vec![Coded::Coding(0, Sound::Aac { config, priming: 2112 }), Coded::Sound(0, vec![2])]).unwrap();
        let Some(Event::Tracks(tracks)) = made.last() else { panic!("the tracks are said") };
        let sound: Vec<(u64, u32, u8)> = made.iter().filter_map(|e| if let Event::Packet(p) = e { Some((p.time, p.duration, p.data[0])) } else { None }).collect();
        assert_eq!(sound, [(0, 1024, 1), (1024, 1024, 2)]);
        let mp4_atom::Codec::Avc1(avc1) = &tracks[0].codec else { panic!("H.264") };
        assert_eq!((avc1.avcc.avc_profile_indication, avc1.avcc.ext.is_some()), (100, true));
        assert_eq!((tracks[0].timescale, tracks[1].timescale), (262_144, 48_000));
        let edit = tracks[1].edits.as_ref().and_then(|e| e.elst.as_ref()).map(|l| l.entries[0].media_time);
        assert_eq!(edit, Some(Some(2112)));
        a.finish().unwrap();
    }

    /// Codecs that never describe a track are said to have made nothing whole.
    #[test]
    fn a_track_never_described_is_an_error() {
        let video = Video { width: 2, height: 2, frame_time: (1, 60) };
        let a = Assembler::new(video, Some(Audio { sample_rate: 48_000, tracks: 1 }));
        assert!(a.finish().is_err());
    }
}
