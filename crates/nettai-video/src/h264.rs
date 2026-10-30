//! H.264 as the codecs make it, made the MP4's: its NAL units (after start codes, Annex B, or after their lengths),
//! each frame's as an MP4 sample (each unit after its length in four bytes, the parameter sets and access unit
//! delimiters left out: they are the track's), and the parameter sets the track is described by.

use crate::codecs::H264;

/// A NAL unit's type, by its first byte.
fn kind(unit: &[u8]) -> u8 {
    unit.first().map_or(0, |b| b & 0x1F)
}

/// The types: a key frame's slice (IDR), a sequence parameter set, a picture parameter set, an access unit
/// delimiter.
const IDR: u8 = 5;
const SPS: u8 = 7;
const PPS: u8 = 8;
const DELIMITER: u8 = 9;

/// The NAL units of H.264 in either form.
pub(crate) fn units(h264: &H264) -> Vec<&[u8]> {
    match h264 {
        H264::AnnexB(bytes) => annex_b(bytes),
        H264::Lengths(bytes) => {
            let mut out = Vec::new();
            let mut rest = &bytes[..];
            while let Some((length, after)) = rest.split_first_chunk::<4>() {
                let length = (u32::from_be_bytes(*length) as usize).min(after.len());
                out.push(&after[..length]);
                rest = &after[length..];
            }
            out
        }
    }
}

/// The NAL units of Annex B bytes, each after a start code (`00 00 01`, or `00 00 00 01`), without the zero bytes
/// that may follow one (a unit ends in a stop bit, never a zero byte).
fn annex_b(bytes: &[u8]) -> Vec<&[u8]> {
    let starts: Vec<usize> = (0..bytes.len().saturating_sub(2)).filter(|&i| bytes[i..i + 3] == [0, 0, 1]).collect();
    starts
        .iter()
        .enumerate()
        .map(|(n, &code)| {
            let unit = &bytes[code + 3..starts.get(n + 1).copied().unwrap_or(bytes.len())];
            let end = unit.iter().rposition(|&b| b != 0).map_or(0, |last| last + 1);
            &unit[..end]
        })
        .filter(|u| !u.is_empty())
        .collect()
}

/// A frame's access unit made an MP4 sample: each NAL unit after its length (four bytes), the parameter sets and
/// delimiters left out.
pub(crate) fn sample(units: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(units.iter().map(|u| u.len() + 4).sum());
    for unit in units.iter().filter(|u| !matches!(kind(u), SPS | PPS | DELIMITER)) {
        out.extend_from_slice(&(unit.len() as u32).to_be_bytes());
        out.extend_from_slice(unit);
    }
    out
}

/// The first SPS and the first PPS among `units`.
pub(crate) fn sets<'a>(units: &[&'a [u8]]) -> (Option<&'a [u8]>, Option<&'a [u8]>) {
    (units.iter().copied().find(|u| kind(u) == SPS), units.iter().copied().find(|u| kind(u) == PPS))
}

/// Whether an access unit's units are a key frame's (an IDR slice).
pub(crate) fn is_key(units: &[&[u8]]) -> bool {
    units.iter().any(|u| kind(u) == IDR)
}

/// A stream of Annex B as it comes (ffmpeg's raw H.264, a frame's access unit after another's, each beginning with
/// a delimiter: x264's `aud`), cut into access units.
#[cfg(any(desktop, test))]
#[derive(Default)]
pub(crate) struct Stream {
    held: Vec<u8>,
}

#[cfg(any(desktop, test))]
impl Stream {
    /// `bytes` more of the stream: the access units it ended (each but the last, which the next delimiter ends).
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        // (From a little before what came, as a start code may have begun in what was held.)
        let from = self.held.len().saturating_sub(4);
        self.held.extend_from_slice(bytes);
        let mut out = Vec::new();
        let mut at = from;
        while let Some(found) = self.held[at..].windows(4).position(|w| w[..3] == [0, 0, 1] && w[3] & 0x1F == DELIMITER) {
            let start = at + found;
            // (The start code's leading zero, `00 00 00 01`, is the delimiter's.)
            let start = if start > 0 && self.held[start - 1] == 0 { start - 1 } else { start };
            if start > 0 {
                out.push(self.held.drain(..start).collect());
                at = 4;
            } else {
                at = start + 4;
            }
        }
        out
    }

    /// The stream ended: the access unit it ends, if any.
    pub(crate) fn end(&mut self) -> Option<Vec<u8>> {
        (!self.held.is_empty()).then(|| std::mem::take(&mut self.held))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Annex B's units, after three- and four-byte start codes and with zero bytes after them, are made an MP4
    /// sample's lengths, the parameter sets and delimiters left out; and the sets and a key frame are found.
    #[test]
    fn annex_b_is_made_a_sample() {
        let annex_b = vec![0, 0, 0, 1, 0x09, 0xF0, 0, 0, 0, 1, 0x67, 1, 2, 0, 0, 1, 0x68, 3, 0, 0, 0, 0, 1, 0x65, 4, 0, 5, 6, 0, 0, 1, 0x41, 7];
        let h264 = H264::AnnexB(annex_b);
        let units = units(&h264);
        assert_eq!(units, vec![&[0x09, 0xF0][..], &[0x67, 1, 2], &[0x68, 3], &[0x65, 4, 0, 5, 6], &[0x41, 7]]);
        assert_eq!(sample(&units), vec![0, 0, 0, 5, 0x65, 4, 0, 5, 6, 0, 0, 0, 2, 0x41, 7]);
        assert_eq!(sets(&units), (Some(&[0x67, 1, 2][..]), Some(&[0x68, 3][..])));
        assert!(is_key(&units) && !is_key(&units[4..]));
        // The same units after their lengths.
        let lengths = H264::Lengths(vec![0, 0, 0, 3, 0x67, 1, 2, 0, 0, 0, 2, 0x41, 7]);
        assert_eq!(super::units(&lengths), vec![&[0x67, 1, 2][..], &[0x41, 7]]);
    }

    /// A raw stream is cut into access units at each delimiter, however its bytes come: one at a time, or all at
    /// once.
    #[test]
    fn a_stream_is_cut_at_its_delimiters() {
        let au = |n: u8, key: bool| -> Vec<u8> { [&[0, 0, 0, 1, 0x09, 0xF0][..], &[0, 0, 1, if key { 0x65 } else { 0x41 }, n, 0, n]].concat() };
        let whole: Vec<u8> = [au(1, true), au(2, false), au(3, false)].concat();
        for step in [1, 3, whole.len()] {
            let mut stream = Stream::default();
            let mut made: Vec<Vec<u8>> = whole.chunks(step).flat_map(|c| stream.push(c)).collect();
            made.extend(stream.end());
            assert_eq!(made, [au(1, true), au(2, false), au(3, false)], "{step} bytes at a time");
        }
    }
}
