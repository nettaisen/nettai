use super::reading::{self, Frame, Stream};
use super::*;

/// nettai's time bases: a frame of the GBA's, and its sound's samples.
const FRAME_TIME: (u32, u32) = (4389, 262_144);
const RATE: u32 = 32_768;

/// A picture of `width` by `height` whose bytes say which frame it is.
fn picture(width: u32, height: u32, n: usize) -> Vec<u8> {
    (0..width as usize * height as usize * 3).map(|i| (i * 7 + n * 31) as u8).collect()
}

/// Frame `n`'s sound: 549 samples, or 546 every eighth frame from the
/// third (4389 over eight frames, the sound driver's count, here running a
/// little ahead of the picture and then behind it), each sample its own
/// number.
fn sound(n: usize) -> Vec<[f32; 2]> {
    let before: usize = (0..n).map(|k| if k % 8 == 2 { 546 } else { 549 }).sum();
    let count = if n % 8 == 2 { 546 } else { 549 };
    (before..before + count).map(|i| [i as f32, -(i as f32) / 2.0]).collect()
}

/// A stream of `frames` frames of a `width` by `height` picture, with
/// sound or without.
fn written(width: u32, height: u32, frames: usize, with_sound: bool) -> Vec<u8> {
    let video = Video { width, height, frame_time: FRAME_TIME };
    let mut w = Writer::new(Vec::new(), video, with_sound.then_some(Audio { sample_rate: RATE, tracks: 1 })).unwrap();
    for n in 0..frames {
        let heard = if with_sound { sound(n) } else { Vec::new() };
        w.frame(&picture(width, height, n), &[&heard]).unwrap();
    }
    assert_eq!(w.frames(), frames as u64);
    let bytes = w.bytes();
    let out = w.finish().unwrap();
    assert!(out.len() as u64 > bytes);
    out
}

/// A stream reads back by the specification's syntax: the two streams as
/// their headers state them (the picture `RGB[24]` on the frame's exact
/// time base with square pixels, the sound `PFD[32]` on one of a sample),
/// every frame's bytes and pts, a syncpoint before each, in time order,
/// the headers three times, every checksum held. And the same frames give
/// the same bytes.
#[test]
fn a_stream_reads_back() {
    let bytes = written(12, 8, 40, true);
    assert_eq!(bytes, written(12, 8, 40, true), "the same frames, other bytes");
    let file = reading::read(&bytes).unwrap();
    assert_eq!((file.version, file.max_distance), (3, 32_768));
    let picture_stream =
        Stream { class: 0, fourcc: b"RGB\x18".to_vec(), time_base: (4389, 262_144), msb_pts_shift: 0, size: (12, 8), sample_shape: (1, 1), sample_rate: (0, 0), channels: 0 };
    let sound_stream =
        Stream { class: 1, fourcc: b"PFD\x20".to_vec(), time_base: (1, 32_768), msb_pts_shift: 0, size: (0, 0), sample_shape: (0, 0), sample_rate: (32_768, 1), channels: 2 };
    assert_eq!(file.streams, [picture_stream, sound_stream]);
    assert_eq!((file.frames.len(), file.syncpoints.len(), file.header_sets.len()), (80, 80, 3));
    assert_eq!(file.header_sets[0], FILE_ID.len());
    // The index is the last packet, behind the headers, and a reader finds
    // it from the end: its length is twelve bytes from there.
    let index = file.index.expect("an index");
    let length = u64::from_be_bytes(bytes[bytes.len() - 12..bytes.len() - 4].try_into().unwrap());
    assert_eq!(bytes.len() as u64 - length, index as u64);
    assert_eq!(bytes[index..index + 8], INDEX_STARTCODE.to_be_bytes());
    let mut samples = 0;
    for n in 0..40 {
        let heard = sound(n);
        let raw: Vec<u8> = heard.iter().flat_map(|s| [s[0].to_le_bytes(), s[1].to_le_bytes()]).flatten().collect();
        let shown = Frame { stream: 0, pts: n as u64, data: picture(12, 8, n) };
        let played = Frame { stream: 1, pts: samples, data: raw };
        // The sound first where it starts no later than its picture: sample
        // `samples` against frame `n`, an eighth of 4389 samples each.
        let pair = if samples * 8 <= n as u64 * 4389 { [played, shown] } else { [shown, played] };
        assert!(file.frames[2 * n..2 * n + 2] == pair, "frame {n}");
        samples += heard.len() as u64;
    }
    // (Both orders came up: the sound runs a little behind, then catches up.)
    let first: Vec<usize> = file.frames.chunks(2).map(|pair| pair[0].stream).collect();
    assert!(first.contains(&0) && first.contains(&1), "{first:?}");
}

/// Two sound tracks and the info: each track a stream of its own on the
/// sound's time base, holding its own samples; its frames in time order
/// with the picture's, the first track's before the second's where they
/// start together; and the info packets in every set of headers: each
/// track's title, the first the default, and each chapter on the picture's
/// time base. The same frames and info give the same bytes.
#[test]
fn two_tracks_and_the_info_read_back() {
    let video = Video { width: 4, height: 2, frame_time: FRAME_TIME };
    let info = Info {
        titles: vec!["Lan".into(), "Chaud".into()],
        chapters: vec![Chapter { start: 0, frames: 30, title: "Round 1".into() }, Chapter { start: 30, frames: 10, title: "ラウンド 2".into() }],
    };
    let right = |n: usize| -> Vec<[f32; 2]> { sound(n).iter().map(|s| [-s[0], s[1] * 3.0]).collect() };
    let write = || {
        let mut w = Writer::with_info(Vec::new(), video, Some(Audio { sample_rate: RATE, tracks: 2 }), &info).unwrap();
        for n in 0..40 {
            w.frame(&picture(4, 2, n), &[&sound(n), &right(n)]).unwrap();
        }
        assert_eq!((w.samples(0), w.samples(1), w.samples(2)), (w.samples(1), (0..40).map(|n| sound(n).len() as u64).sum(), 0));
        w.finish().unwrap()
    };
    let bytes = write();
    assert_eq!(bytes, write(), "the same frames, other bytes");
    let file = reading::read(&bytes).unwrap();
    assert_eq!((file.streams.len(), file.header_sets.len(), file.frames.len()), (3, 3, 120));
    assert!(file.streams[1] == file.streams[2] && file.streams[1].fourcc == b"PFD\x20" && file.streams[1].time_base == (1, 32_768));
    let raw = |heard: Vec<[f32; 2]>| -> Vec<u8> { heard.iter().flat_map(|s| [s[0].to_le_bytes(), s[1].to_le_bytes()]).flatten().collect() };
    let mut samples = 0;
    for (n, tick) in file.frames.chunks(3).enumerate() {
        let shown = Frame { stream: 0, pts: n as u64, data: picture(4, 2, n) };
        let (left, other) = (Frame { stream: 1, pts: samples, data: raw(sound(n)) }, Frame { stream: 2, pts: samples, data: raw(right(n)) });
        let order = if samples * 8 <= n as u64 * 4389 { [left, other, shown] } else { [shown, left, other] };
        assert!(tick == order, "tick {n}");
        samples += sound(n).len() as u64;
    }
    let fields = |pairs: &[(&str, &str)]| -> Vec<(String, String)> { pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect() };
    let info = |stream_id_plus1, chapter_id, start, length, pairs: &[(&str, &str)]| reading::Info { stream_id_plus1, chapter_id, start, length, fields: fields(pairs) };
    assert_eq!(
        file.info,
        [
            info(2, 0, 0, 0, &[("Title", "Lan"), ("handler_name", "Lan"), ("Disposition", "default")]),
            info(3, 0, 0, 0, &[("Title", "Chaud"), ("handler_name", "Chaud")]),
            // (A chapter's start is a `t`: the frame times the two time
            // bases, plus the picture's index, 0.)
            info(0, 1, 0, 30, &[("Title", "Round 1")]),
            info(0, 2, 60, 10, &[("Title", "ラウンド 2")]),
        ]
    );
    // A track without a title says only that it is the default; a stream
    // without sound has no track's info.
    let untitled = Writer::with_info(Vec::new(), video, Some(Audio { sample_rate: RATE, tracks: 1 }), &Info::default()).unwrap().finish().unwrap();
    assert_eq!(reading::read(&untitled).unwrap().info, [info(2, 0, 0, 0, &[("Disposition", "default")])]);
    let silent = Writer::with_info(Vec::new(), video, None, &Info { titles: Vec::new(), chapters: vec![Chapter { start: 0, frames: 1, title: "Round 1".into() }] });
    assert_eq!(reading::read(&silent.unwrap().finish().unwrap()).unwrap().info, [info(0, 1, 0, 1, &[("Title", "Round 1")])]);
}

/// A stream without sound is the picture's alone: one stream, one time
/// base, a frame after each syncpoint.
#[test]
fn a_stream_without_sound_is_the_pictures_alone() {
    let file = reading::read(&written(6, 4, 9, false)).unwrap();
    assert_eq!((file.streams.len(), file.streams[0].class, file.frames.len(), file.header_sets.len()), (1, 0, 9, 3));
    assert!(file.frames.iter().enumerate().all(|(n, f)| f.stream == 0 && f.pts == n as u64 && f.data == picture(6, 4, n)));
}

/// The headers are written again once the stream passes 8 MiB, at the
/// next frame, and at the end: a stream of game-sized frames long enough
/// has them in the middle, a short one twice at the end.
#[test]
fn the_headers_repeat_past_eight_mebibytes_and_at_the_end() {
    let bytes = written(240, 160, 80, true);
    let file = reading::read(&bytes).unwrap();
    assert_eq!((file.frames.len(), file.header_sets.len()), (160, 3));
    let a_tick = 240 * 160 * 3 + 549 * 8 + 200;
    let (middle, end, index) = (file.header_sets[1], file.header_sets[2], file.index.unwrap());
    assert!((1 << 23..(1 << 23) + a_tick).contains(&middle), "the middle headers are at {middle}");
    assert!(end > middle && index - end < 200, "the last headers are at {end}, the index at {index}");
    // A short stream: at the start, and twice at its end, before the index.
    let short = written(240, 160, 2, true);
    let file = reading::read(&short).unwrap();
    assert_eq!(file.header_sets.len(), 3);
    assert_eq!(file.header_sets[2] - file.header_sets[1], file.index.unwrap() - file.header_sets[2]);
}

/// A long stream's index is past the 4096 bytes a packet may be without a
/// checksum of its header's own, and has one; it reads back as what the
/// frames are (the reader checks every place and pts it states). A stream
/// of no frames has no index.
#[test]
fn a_long_streams_index_reads_back() {
    let bytes = written(12, 8, 1200, true);
    let file = reading::read(&bytes).unwrap();
    let index = file.index.unwrap();
    assert_eq!((file.frames.len(), file.syncpoints.len()), (2400, 2400));
    assert!(bytes.len() - index > 4096 + 16, "an index of {} bytes", bytes.len() - index);
    let mut header = bytes.clone();
    header[index + 9] ^= 1;
    assert!(reading::read(&header).unwrap_err().contains("the index: its header's checksum"));
    let empty = written(12, 8, 0, true);
    assert_eq!(reading::read(&empty).unwrap().index, None);
}

/// What isn't a stream is refused, saying what: a picture of no size, a
/// frame time that isn't a fraction in lowest terms, pixels that aren't a
/// frame's, sound where the stream has none, and a sound that runs ahead
/// of its picture.
#[test]
fn what_isnt_a_stream_is_refused() {
    let video = Video { width: 4, height: 2, frame_time: FRAME_TIME };
    let refused = |video, audio| Writer::new(Vec::new(), video, audio).err().map(|e| e.to_string());
    assert_eq!(refused(Video { width: 0, ..video }, None).unwrap(), "a picture of 0 by 2 pixels");
    assert_eq!(refused(Video { frame_time: (2, 4), ..video }, None).unwrap(), "a frame time of 2/4 s: a fraction in lowest terms, each part under 2^31");
    assert_eq!(refused(Video { frame_time: (1, 0), ..video }, None).unwrap(), "a frame time of 1/0 s: a fraction in lowest terms, each part under 2^31");
    assert_eq!(refused(video, Some(Audio { sample_rate: 0, tracks: 1 })).unwrap(), "a sample rate of 0 Hz");
    assert_eq!(refused(Video { frame_time: (1, 60), ..video }, Some(Audio { sample_rate: 60, tracks: 1 })).unwrap(), "the sound's time base is the picture's (1/60 s)");
    assert_eq!(refused(video, Some(Audio { sample_rate: RATE, tracks: 0 })).unwrap(), "0 sound tracks (1 to 16)");
    assert_eq!(refused(video, Some(Audio { sample_rate: RATE, tracks: 17 })).unwrap(), "17 sound tracks (1 to 16)");
    assert_eq!(refused(video, None), None);
    let titled = Info { titles: vec!["A".into()], chapters: Vec::new() };
    let refused_titles = Writer::with_info(Vec::new(), video, None, &titled).err().map(|e| e.to_string());
    assert_eq!(refused_titles.unwrap(), "titles for 1 sound tracks of a stream of 0");
    let mut w = Writer::new(Vec::new(), video, None).unwrap();
    let before = w.bytes();
    assert_eq!(w.frame(&[0; 23], &[]).unwrap_err().to_string(), "23 bytes of picture; a frame of 4 by 2 is 24");
    assert_eq!(w.frame(&[0; 24], &[&[[0.0; 2]]]).unwrap_err().to_string(), "sound for a stream that has none");
    assert_eq!((w.bytes(), w.frames()), (before, 0), "nothing was written");
    let mut w = Writer::new(Vec::new(), video, Some(Audio { sample_rate: RATE, tracks: 2 })).unwrap();
    let before = w.bytes();
    assert_eq!(w.frame(&[0; 24], &[&[[0.0; 2]]]).unwrap_err().to_string(), "sound of 1 tracks for a stream of 2");
    assert_eq!((w.bytes(), w.frames()), (before, 0), "nothing was written");
    // Far more sound than a frame's: it is cut into frames of 4096 samples
    // around its picture, and the next picture would come before the last
    // of it.
    let mut w = Writer::new(Vec::new(), video, Some(Audio { sample_rate: RATE, tracks: 1 })).unwrap();
    w.frame(&[0; 24], &[&[[0.5; 2]; 5000][..]]).unwrap();
    assert_eq!((w.frames(), w.samples(0)), (1, 5000));
    assert_eq!(w.frame(&[0; 24], &[&[[0.5; 2]; 549][..]]).unwrap_err().to_string(), "frame 1 of the picture would come before sample 4096 of the sound, already written");
    let file = reading::read(&w.finish().unwrap()).unwrap();
    let order: Vec<(usize, u64, usize)> = file.frames.iter().map(|f| (f.stream, f.pts, f.data.len())).collect();
    assert_eq!(order, [(1, 0, 4096 * 8), (0, 0, 24), (1, 4096, 904 * 8)]);
}

/// The numbers and the checksum are the specification's: a `v` is seven
/// bits a byte from the top, an `s` folds the sign into the low bit, and
/// the checksum is the CRC-32 of generator 0x04C11DB7 from zero, most
/// significant bit first (CRC-32/CKSUM's register before its final
/// inversion: 0x765E7680 inverted, for "123456789").
#[test]
fn the_numbers_and_the_checksum_are_nuts() {
    let v = |value| {
        let mut out = Vec::new();
        put_v(&mut out, value);
        out
    };
    assert_eq!(v(0), [0]);
    assert_eq!(v(127), [0x7F]);
    assert_eq!(v(128), [0x81, 0x00]);
    assert_eq!(v(16_383), [0xFF, 0x7F]);
    assert_eq!(v(115_200), [0x87, 0x84, 0x00]);
    assert_eq!(v(u64::MAX >> 1), [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]);
    let s = |value| {
        let mut out = Vec::new();
        put_s(&mut out, value);
        out
    };
    assert_eq!([s(0), s(1), s(-1), s(2), s(-2)], [[0], [1], [2], [3], [4]]);
    assert_eq!(crc32(b"123456789"), !0x765E_7680);
    assert_eq!(crc32(b""), 0);
    // The startcodes, as the specification gives them.
    assert_eq!(MAIN_STARTCODE.to_be_bytes(), [b'N', b'M', 0x7A, 0x56, 0x1F, 0x5F, 0x04, 0xAD]);
    assert_eq!(STREAM_STARTCODE.to_be_bytes(), [b'N', b'S', 0x11, 0x40, 0x5B, 0xF2, 0xF9, 0xDB]);
    assert_eq!(SYNCPOINT_STARTCODE.to_be_bytes(), [b'N', b'K', 0xE4, 0xAD, 0xEE, 0xCA, 0x45, 0x69]);
    assert_eq!(INFO_STARTCODE.to_be_bytes(), [b'N', b'I', 0xAB, 0x68, 0xB5, 0x96, 0xBA, 0x78]);
}

/// A stream that breaks the specification is told from one that keeps it
/// (the reader the other tests rely on does check): a byte changed in a
/// frame's header, in a syncpoint, in the headers; the headers missing at
/// the end.
#[test]
fn the_reader_refuses_a_stream_that_isnt_one() {
    let bytes = written(12, 8, 4, true);
    let file = reading::read(&bytes).unwrap();
    let change = |at: usize| {
        let mut other = bytes.clone();
        other[at] ^= 1;
        reading::read(&other).unwrap_err()
    };
    let syncpoint = file.syncpoints[2];
    assert!(change(syncpoint + 9).contains("a syncpoint: its checksum"), "{}", change(syncpoint + 9));
    assert!(change(file.header_sets[0] + 10).contains("the main header: its checksum"));
    // (A syncpoint is its startcode, a forward pointer of a byte here, and
    // as many bytes as that says: the frame follows.)
    let frame = syncpoint + 9 + bytes[syncpoint + 8] as usize;
    assert!(change(frame + 1).contains("its header's checksum"), "{}", change(frame + 1));
    assert_eq!(reading::read(&bytes[..file.header_sets[1]]).unwrap_err(), "the file doesn't end with the headers");
    let index = file.index.unwrap();
    assert_eq!(reading::read(&bytes[..index]).unwrap_err(), "no index after the last headers");
    assert!(change(index + 12).contains("the index"), "{}", change(index + 12));
    let mut more = bytes.clone();
    more.push(0);
    assert_eq!(reading::read(&more).unwrap_err(), "1 bytes after the index");
    assert_eq!(reading::read(&bytes[1..]).unwrap_err(), "no file id string at the start");
}
