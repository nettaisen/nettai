//! A NUT stream read back, for the tests: written from the specification's
//! syntax on its own (the main header's frame code table decoded as the
//! specification's loop decodes one, a frame's header read by its code's
//! flags), and checking what the specification requires of a stream as it
//! goes. It reads what the writer writes and refuses the rest.

use super::{FILE_ID, crc32};

/// A stream of a NUT file, by its header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stream {
    /// 0 video, 1 audio.
    pub class: u64,
    pub fourcc: Vec<u8>,
    /// Its time base, a fraction of a second.
    pub time_base: (u64, u64),
    pub msb_pts_shift: u64,
    /// A picture's width and height, and its samples' shape.
    pub size: (u64, u64),
    pub sample_shape: (u64, u64),
    /// A sound's rate (a fraction) and channels.
    pub sample_rate: (u64, u64),
    pub channels: u64,
}

/// A frame of a NUT file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub stream: usize,
    pub pts: u64,
    pub data: Vec<u8>,
}

/// A NUT file, read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub version: u64,
    pub max_distance: u64,
    pub streams: Vec<Stream>,
    /// Every frame, in the file's order.
    pub frames: Vec<Frame>,
    /// How many times the headers are in the file, and where each set
    /// starts.
    pub header_sets: Vec<usize>,
    /// Where each syncpoint starts.
    pub syncpoints: Vec<usize>,
    /// Where the index starts, if the file has one (it is then the file's
    /// last packet, and says what the frames read say).
    pub index: Option<usize>,
}

/// What a frame code stands for (the main header's table).
#[derive(Clone, Copy, Debug, Default)]
struct Code {
    flags: u64,
    stream: u64,
    size_mul: u64,
    size_lsb: u64,
    pts_delta: i64,
    reserved: u64,
}

const FLAG_KEY: u64 = 1 << 0;
const FLAG_CODED_PTS: u64 = 1 << 3;
const FLAG_STREAM_ID: u64 = 1 << 4;
const FLAG_SIZE_MSB: u64 = 1 << 5;
const FLAG_CHECKSUM: u64 = 1 << 6;
const FLAG_RESERVED: u64 = 1 << 7;
const FLAG_HEADER_IDX: u64 = 1 << 10;
const FLAG_MATCH_TIME: u64 = 1 << 11;
const FLAG_CODED: u64 = 1 << 12;
const FLAG_INVALID: u64 = 1 << 13;

const fn startcode(letter: u8, rest: u64) -> u64 {
    rest + (((b'N' as u64) << 8 | letter as u64) << 48)
}
const MAIN: u64 = startcode(b'M', 0x7A56_1F5F_04AD);
const STREAM: u64 = startcode(b'S', 0x1140_5BF2_F9DB);
const SYNCPOINT: u64 = startcode(b'K', 0xE4AD_EECA_4569);
const INDEX: u64 = startcode(b'X', 0xDD67_2F23_E64E);
const INFO: u64 = startcode(b'I', 0xAB68_B596_BA78);

/// Bytes being read, and where.
struct Bytes<'a> {
    all: &'a [u8],
    at: usize,
}

impl Bytes<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let b = self.all.get(self.at..self.at + n).ok_or_else(|| format!("the file ends {n} bytes short at {}", self.at))?;
        self.at += n;
        Ok(b)
    }

    fn v(&mut self) -> Result<u64, String> {
        let mut value = 0u64;
        loop {
            let b = self.take(1)?[0];
            value = value << 7 | (b & 0x7F) as u64;
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
    }

    fn s(&mut self) -> Result<i64, String> {
        let temp = self.v()? + 1;
        Ok(if temp & 1 != 0 { -((temp >> 1) as i64) } else { (temp >> 1) as i64 })
    }

    fn vb(&mut self) -> Result<Vec<u8>, String> {
        let n = self.v()? as usize;
        Ok(self.take(n)?.to_vec())
    }
}

/// A packet's body behind its startcode (already taken, from `at`): its
/// forward pointer followed, and its checksums held (the body's, and past
/// a forward pointer of 4096 the header's own).
fn packet<'a>(b: &mut Bytes<'a>, at: usize, what: &str) -> Result<Bytes<'a>, String> {
    let forward = b.v()? as usize;
    if forward > 4096 {
        let header = crc32(&b.all[at..b.at]);
        let sum = u32::from_be_bytes(b.take(4)?.try_into().unwrap());
        if header != sum {
            return Err(format!("{what}: its header's checksum is {sum:08x}, its bytes' {header:08x}"));
        }
    }
    let start = b.at;
    let body = b.take(forward.checked_sub(4).ok_or_else(|| format!("{what}: a forward pointer of {forward}"))?)?.to_vec();
    let sum = u32::from_be_bytes(b.take(4)?.try_into().unwrap());
    if crc32(&body) != sum {
        return Err(format!("{what}: its checksum is {sum:08x}, its body's {:08x}", crc32(&body)));
    }
    Ok(Bytes { all: &b.all[..start + body.len()], at: start })
}

/// That a packet's body was read to its end (a writer writes no reserved
/// bytes).
fn done(b: &Bytes, what: &str) -> Result<(), String> {
    if b.at == b.all.len() { Ok(()) } else { Err(format!("{what}: {} bytes after its fields", b.all.len() - b.at)) }
}

/// Whether `a` units of the time base `x` are later than `b` of `y`.
fn later(a: u64, x: (u64, u64), b: u64, y: (u64, u64)) -> bool {
    a as u128 * x.0 as u128 * y.1 as u128 > b as u128 * y.0 as u128 * x.1 as u128
}

/// A NUT file from its bytes, or what is wrong with it.
pub fn read(bytes: &[u8]) -> Result<File, String> {
    let mut b = Bytes { all: bytes, at: 0 };
    if b.take(FILE_ID.len()).ok() != Some(FILE_ID) {
        return Err("no file id string at the start".into());
    }
    let mut file = File { version: 0, max_distance: 0, streams: Vec::new(), frames: Vec::new(), header_sets: Vec::new(), syncpoints: Vec::new(), index: None };
    // For each syncpoint, the first frame of each stream behind it (before
    // the next syncpoint): its pts.
    let mut behind: Vec<Vec<Option<u64>>> = Vec::new();
    let mut codes = [Code::default(); 256];
    let mut time_bases: Vec<(u64, u64)> = Vec::new();
    // The first set of headers' bytes: every later set is the same.
    let mut first_set: Option<Vec<u8>> = None;
    // A frame may follow only a syncpoint or a frame; the last syncpoint's
    // place and time; the syncpoint before each stream's last frame; the
    // last frame's stream and pts.
    let mut after_headers = true;
    let mut last_syncpoint: Option<(usize, u64, usize)> = None;
    let mut syncpoint_before: Vec<Option<usize>> = Vec::new();
    let mut last_frame: Option<(usize, u64)> = None;
    let mut ended_with_headers = false;
    while b.at < bytes.len() {
        let at = b.at;
        if file.index.is_some() {
            return Err(format!("{} bytes after the index", bytes.len() - at));
        }
        if bytes[at] != b'N' {
            // A frame.
            if after_headers {
                return Err(format!("a frame at {at} with no syncpoint since the headers"));
            }
            let (sync_at, ..) = last_syncpoint.ok_or_else(|| format!("a frame at {at} before any syncpoint"))?;
            if at as u64 > sync_at as u64 + file.max_distance {
                return Err(format!("a frame starts at {at}, more than max_distance past the syncpoint at {sync_at}"));
            }
            let code = codes[b.take(1)?[0] as usize];
            let mut flags = code.flags;
            if flags & FLAG_INVALID != 0 {
                return Err(format!("the frame at {at} has an invalid code {}", bytes[at]));
            }
            if flags & FLAG_CODED != 0 {
                flags ^= b.v()?;
            }
            let stream = if flags & FLAG_STREAM_ID != 0 { b.v()? } else { code.stream } as usize;
            let info = file.streams.get(stream).ok_or_else(|| format!("the frame at {at} is of stream {stream}"))?;
            if flags & FLAG_CODED_PTS == 0 {
                return Err(format!("the frame at {at} codes no pts (a pts_delta of {})", code.pts_delta));
            }
            let coded = b.v()?;
            let pts = coded.checked_sub(1 << info.msb_pts_shift).ok_or_else(|| format!("the frame at {at} codes its pts's low bits alone"))?;
            let size = code.size_lsb + if flags & FLAG_SIZE_MSB != 0 { code.size_mul * b.v()? } else { 0 };
            if flags & FLAG_MATCH_TIME != 0 {
                b.s()?;
            }
            if flags & FLAG_HEADER_IDX != 0 && b.v()? != 0 {
                return Err(format!("the frame at {at} names an elision header"));
            }
            let reserved = if flags & FLAG_RESERVED != 0 { b.v()? } else { code.reserved };
            for _ in 0..reserved {
                b.v()?;
            }
            if flags & FLAG_CHECKSUM != 0 {
                let header = crc32(&bytes[at..b.at]);
                let sum = u32::from_be_bytes(b.take(4)?.try_into().unwrap());
                if header != sum {
                    return Err(format!("the frame at {at}: its header's checksum is {sum:08x}, its bytes' {header:08x}"));
                }
            } else if size > 2 * file.max_distance {
                return Err(format!("the frame at {at} is {size} bytes, over twice max_distance, with no checksum"));
            }
            if flags & FLAG_KEY == 0 {
                return Err(format!("the frame at {at} is no keyframe"));
            }
            // In time order across the streams, and no earlier than the
            // last syncpoint's time.
            let base = info.time_base;
            if let Some((s, p)) = last_frame
                && later(p, file.streams[s].time_base, pts, base)
            {
                return Err(format!("the frame at {at} (stream {stream}, pts {pts}) is before the one before it (stream {s}, pts {p})"));
            }
            let (_, sync_pts, sync_base) = last_syncpoint.expect("a syncpoint");
            if later(sync_pts, time_bases[sync_base], pts, base) {
                return Err(format!("the frame at {at} (pts {pts}) is before its syncpoint's time"));
            }
            let data = b.take(size as usize)?.to_vec();
            let first = &mut behind.last_mut().expect("a syncpoint")[stream];
            *first = first.or(Some(pts));
            syncpoint_before[stream] = Some(sync_at);
            last_frame = Some((stream, pts));
            file.frames.push(Frame { stream, pts, data });
            ended_with_headers = false;
            continue;
        }
        let code = u64::from_be_bytes(b.take(8)?.try_into().unwrap());
        match code {
            MAIN => {
                file.header_sets.push(at);
                let mut p = packet(&mut b, at, "the main header")?;
                // What a set of headers is, to compare with the first.
                let mut set_end = b.at;
                let (version, streams, max_distance) = (p.v()?, p.v()? as usize, p.v()?);
                if version != 3 {
                    return Err(format!("version {version}"));
                }
                let mut bases = Vec::new();
                for _ in 0..p.v()? {
                    let (num, den) = (p.v()?, p.v()?);
                    if num == 0 || den == 0 || num >= 1 << 31 || den >= 1 << 31 || bases.contains(&(num, den)) {
                        return Err(format!("a time base of {num}/{den}"));
                    }
                    bases.push((num, den));
                }
                // The frame code table, as the specification's loop reads it.
                let (mut tmp_pts, mut tmp_mul, mut tmp_stream) = (0i64, 1u64, 0u64);
                let mut i = 0usize;
                let mut table = [Code::default(); 256];
                while i < 256 {
                    let tmp_flag = p.v()?;
                    let fields = p.v()?;
                    if fields > 0 {
                        tmp_pts = p.s()?;
                    }
                    if fields > 1 {
                        tmp_mul = p.v()?;
                    }
                    if fields > 2 {
                        tmp_stream = p.v()?;
                    }
                    let tmp_size = if fields > 3 { p.v()? } else { 0 };
                    let tmp_res = if fields > 4 { p.v()? } else { 0 };
                    let count = if fields > 5 { p.v()? } else { tmp_mul.saturating_sub(tmp_size) };
                    if fields > 6 {
                        p.s()?;
                    }
                    if fields > 7 && p.v()? != 0 {
                        return Err("a frame code with an elision header".into());
                    }
                    for _ in 8..fields {
                        p.v()?;
                    }
                    if count == 0 || tmp_stream as usize >= streams {
                        return Err(format!("frame codes from {i}: a count of {count}, stream {tmp_stream}"));
                    }
                    let mut j = 0;
                    while j < count && i < 256 {
                        if i == b'N' as usize {
                            table[i].flags = FLAG_INVALID;
                            i += 1;
                            continue;
                        }
                        table[i] = Code { flags: tmp_flag, stream: tmp_stream, size_mul: tmp_mul, size_lsb: tmp_size + j, pts_delta: tmp_pts, reserved: tmp_res };
                        i += 1;
                        j += 1;
                    }
                    if j < count {
                        return Err(format!("the frame codes run {} past 255", count - j));
                    }
                }
                if table[0].flags & FLAG_INVALID == 0 || table[255].flags & FLAG_INVALID == 0 {
                    return Err("frame codes 0 and 255 should be invalid".into());
                }
                if p.v()? != 0 {
                    return Err("elision headers".into());
                }
                if p.v()? != 0 {
                    return Err("main flags".into());
                }
                done(&p, "the main header")?;
                // The stream headers, in order.
                let mut read_streams = Vec::new();
                for id in 0..streams {
                    if b.take(8).ok() != Some(&STREAM.to_be_bytes()[..]) {
                        return Err(format!("no header of stream {id} after the main header at {at}"));
                    }
                    let what = format!("stream {id}'s header");
                    let stream_at = b.at - 8;
                    let mut p = packet(&mut b, stream_at, &what)?;
                    set_end = b.at;
                    if p.v()? as usize != id {
                        return Err(format!("{what}: another stream's id"));
                    }
                    let (class, fourcc) = (p.v()?, p.vb()?);
                    let base = *bases.get(p.v()? as usize).ok_or_else(|| format!("{what}: no such time base"))?;
                    let msb_pts_shift = p.v()?;
                    let (_max_pts_distance, decode_delay, _flags, extra) = (p.v()?, p.v()?, p.v()?, p.vb()?);
                    if msb_pts_shift >= 16 || decode_delay != 0 || !extra.is_empty() || !matches!(fourcc.len(), 2 | 4) {
                        return Err(format!("{what}: msb_pts_shift {msb_pts_shift}, decode_delay {decode_delay}, {} bytes of codec data, a fourcc of {}", extra.len(), fourcc.len()));
                    }
                    let mut stream =
                        Stream { class, fourcc, time_base: base, msb_pts_shift, size: (0, 0), sample_shape: (0, 0), sample_rate: (0, 0), channels: 0 };
                    match class {
                        0 => {
                            stream.size = (p.v()?, p.v()?);
                            stream.sample_shape = (p.v()?, p.v()?);
                            p.v()?; // colorspace_type
                            if stream.size.0 == 0 || stream.size.1 == 0 || (stream.sample_shape.0 == 0) != (stream.sample_shape.1 == 0) {
                                return Err(format!("{what}: a picture of {:?}, samples of {:?}", stream.size, stream.sample_shape));
                            }
                        }
                        1 => {
                            stream.sample_rate = (p.v()?, p.v()?);
                            stream.channels = p.v()?;
                            if stream.sample_rate.0 == 0 || stream.sample_rate.1 == 0 || stream.channels == 0 {
                                return Err(format!("{what}: a rate of {:?}, {} channels", stream.sample_rate, stream.channels));
                            }
                        }
                        other => return Err(format!("{what}: stream class {other}")),
                    }
                    done(&p, &what)?;
                    read_streams.push(stream);
                }
                let set = bytes[at..set_end].to_vec();
                match &first_set {
                    None => {
                        first_set = Some(set);
                        (file.version, file.max_distance, file.streams) = (version, max_distance, read_streams);
                        (codes, time_bases) = (table, bases);
                        syncpoint_before = vec![None; streams];
                    }
                    Some(first) if *first != set => return Err(format!("the headers at {at} aren't the first ones")),
                    Some(_) => {}
                }
                after_headers = true;
                ended_with_headers = true;
            }
            SYNCPOINT => {
                if first_set.is_none() {
                    return Err(format!("a syncpoint at {at} before the headers"));
                }
                let mut p = packet(&mut b, at, "a syncpoint")?;
                let t = p.v()?;
                let (pts, base) = (t / time_bases.len() as u64, (t % time_bases.len() as u64) as usize);
                let back = p.v()? as usize * 16;
                done(&p, "a syncpoint")?;
                // The way back: to the nearest syncpoint with a frame of
                // every stream that has one after it (to itself before
                // any), named to within fifteen bytes before it.
                let to = syncpoint_before.iter().flatten().min().copied().unwrap_or(at);
                let pointed = at.checked_sub(back + 15);
                if !pointed.is_none_or(|p| p <= to) || at.checked_sub(back).is_none_or(|upper| upper < to) {
                    return Err(format!("the syncpoint at {at} points {back} back; the one it should name is at {to}"));
                }
                // Its time: no earlier than any frame before it.
                if let Some((s, p)) = last_frame
                    && later(p, file.streams[s].time_base, pts, time_bases[base])
                {
                    return Err(format!("the syncpoint at {at} is at a time before the frame before it"));
                }
                last_syncpoint = Some((at, pts, base));
                file.syncpoints.push(at);
                behind.push(vec![None; file.streams.len()]);
                after_headers = false;
                ended_with_headers = false;
            }
            STREAM => return Err(format!("a stream header at {at} outside a set of headers")),
            INDEX => {
                if !ended_with_headers {
                    return Err(format!("the index at {at} doesn't follow the headers"));
                }
                let mut p = packet(&mut b, at, "the index")?;
                // The latest pts, and every syncpoint's place.
                let t = p.v()?;
                let max = ((t % time_bases.len() as u64) as usize, t / time_bases.len() as u64);
                if last_frame.is_none_or(|(s, p)| (time_bases[max.0], max.1) != (file.streams[s].time_base, p)) {
                    return Err(format!("the index's last pts is time base {}'s {}; the last frame's is {last_frame:?}", max.0, max.1));
                }
                let count = p.v()? as usize;
                let mut place = 0;
                let mut places = Vec::new();
                for _ in 0..count {
                    place += p.v()?;
                    places.push(place);
                }
                let read: Vec<u64> = file.syncpoints.iter().map(|&s| s as u64 >> 4).collect();
                if places != read {
                    return Err(format!("the index places {count} syncpoints; the file has {} (or elsewhere)", read.len()));
                }
                // By stream: which syncpoints have a keyframe of it before
                // them, as the specification's loop reads it, and those
                // keyframes' pts.
                for stream in 0..file.streams.len() {
                    let mut has = vec![false; count + 1];
                    let mut last_pts: i64 = -1;
                    let mut j = 0;
                    while j < count {
                        let mut x = p.v()?;
                        let run = x & 1 != 0;
                        x >>= 1;
                        let mut n = j;
                        let mut set = |n: &mut usize, flag: bool| -> Result<(), String> {
                            *has.get_mut(*n).ok_or("the index's flags run past its syncpoints")? = flag;
                            *n += 1;
                            Ok(())
                        };
                        if run {
                            let flag = x & 1 != 0;
                            for _ in 0..x >> 1 {
                                set(&mut n, flag)?;
                            }
                            set(&mut n, !flag)?;
                        } else {
                            if x <= 1 {
                                return Err("the index: a number of no flags".into());
                            }
                            while x != 1 {
                                set(&mut n, x & 1 != 0)?;
                                x >>= 1;
                            }
                        }
                        while j < n && j < count {
                            // (The keyframe before syncpoint j is behind
                            // syncpoint j - 1.)
                            let frame = j.checked_sub(1).and_then(|k| behind[k][stream]);
                            if has[j] != frame.is_some() {
                                return Err(format!("the index: stream {stream} has {} keyframe before syncpoint {j}", if has[j] { "a" } else { "no" }));
                            }
                            if let Some(pts) = frame {
                                let a = p.v()? as i64;
                                if a == 0 || last_pts + a != pts as i64 {
                                    return Err(format!("the index: stream {stream}'s keyframe before syncpoint {j} is at {}, not {pts}", last_pts + a));
                                }
                                last_pts += a;
                            }
                            j += 1;
                        }
                    }
                }
                // Its own length, startcode to checksum, as its last field.
                let length = u64::from_be_bytes(p.take(8)?.try_into().unwrap());
                done(&p, "the index")?;
                if length != (b.at - at) as u64 {
                    return Err(format!("the index says it is {length} bytes; it is {}", b.at - at));
                }
                file.index = Some(at);
            }
            INFO => return Err(format!("an info packet at {at}")),
            other => return Err(format!("a startcode {other:016x} at {at}")),
        }
    }
    if !ended_with_headers {
        return Err("the file doesn't end with the headers".into());
    }
    if file.index.is_none() && !file.frames.is_empty() {
        return Err("no index after the last headers".into());
    }
    Ok(file)
}
