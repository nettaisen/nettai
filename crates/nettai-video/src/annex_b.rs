//! H.264's Annex B (each NAL unit after a start code, as Android's encoder makes it) made the MP4's form (each after
//! its length).

/// The NAL units of Annex B bytes, each after a start code (`00 00 01`, or `00 00 00 01`), without the zero bytes
/// that may follow one (a unit ends in a stop bit, never a zero byte).
pub(crate) fn units(bytes: &[u8]) -> Vec<&[u8]> {
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

/// An access unit's Annex B made the MP4's: each NAL unit after its length (four bytes), the parameter sets and
/// access unit delimiters left out (the track's avcC has the parameter sets).
pub(crate) fn lengths(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 16);
    for unit in units(bytes) {
        if matches!(unit[0] & 0x1F, 7..=9) {
            continue;
        }
        out.extend_from_slice(&(unit.len() as u32).to_be_bytes());
        out.extend_from_slice(unit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{lengths, units};

    /// Annex B's units, after three- and four-byte start codes and with zero bytes after them, are made the MP4's
    /// lengths, the parameter sets and delimiters left out.
    #[test]
    fn annex_b_is_made_lengths() {
        let annex_b = [0, 0, 0, 1, 0x09, 0xF0, 0, 0, 0, 1, 0x67, 1, 2, 0, 0, 1, 0x68, 3, 0, 0, 0, 0, 1, 0x65, 4, 0, 5, 6, 0, 0, 1, 0x41, 7];
        assert_eq!(units(&annex_b), vec![&[0x09, 0xF0][..], &[0x67, 1, 2], &[0x68, 3], &[0x65, 4, 0, 5, 6], &[0x41, 7]]);
        assert_eq!(lengths(&annex_b), vec![0, 0, 0, 5, 0x65, 4, 0, 5, 6, 0, 0, 0, 2, 0x41, 7]);
    }
}
