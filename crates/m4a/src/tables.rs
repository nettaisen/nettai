//! The driver's own constant tables (the same in every game that links the
//! M4A library) and the key-to-frequency conversions built on them.

/// Wait lengths of W00..W96 (and note lengths of N01..N96): `gClockTable`.
pub const CLOCK: [u8; 49] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 28, 30, 32, 36, 40, 42,
    44, 48, 52, 54, 56, 60, 64, 66, 68, 72, 76, 78, 80, 84, 88, 90, 92, 96,
];

/// `gFreqTable`: 2^31 times the twelve semitones' ratios, the top octave
/// of [`midi_key_to_freq`]'s scale.
const FREQ: [u32; 12] = [
    0x8000_0000,
    0x879C_7C97,
    0x8FAC_D61E,
    0x9837_F052,
    0xA145_17CC,
    0xAADC_0848,
    0xB504_F334,
    0xBFC8_86BB,
    0xCB2F_F52A,
    0xD744_FCCB,
    0xE411_F03A,
    0xF1A1_BF39,
];

/// `gCgbFreqTable`: the PSG frequency register minus 2048 for the twelve
/// semitones of the lowest octave (key 36 up).
const CGB_FREQ: [i16; 12] = [-2004, -1891, -1785, -1685, -1591, -1501, -1417, -1337, -1262, -1192, -1125, -1062];

/// `gNoiseTable`: NR43 (clock shift, width, divisor) for keys 21 and up.
pub const NOISE: [u8; 60] = [
    0xD7, 0xD6, 0xD5, 0xD4, 0xC7, 0xC6, 0xC5, 0xC4, 0xB7, 0xB6, 0xB5, 0xB4, 0xA7, 0xA6, 0xA5, 0xA4, 0x97, 0x96, 0x95,
    0x94, 0x87, 0x86, 0x85, 0x84, 0x77, 0x76, 0x75, 0x74, 0x67, 0x66, 0x65, 0x64, 0x57, 0x56, 0x55, 0x54, 0x47, 0x46,
    0x45, 0x44, 0x37, 0x36, 0x35, 0x34, 0x27, 0x26, 0x25, 0x24, 0x17, 0x16, 0x15, 0x14, 0x07, 0x06, 0x05, 0x04, 0x03,
    0x02, 0x01, 0x00,
];

/// `gCgb3Vol`: NR32 (the wave channel's output level) for envelope levels
/// 0..=15: off, 25%, 50%, 75% (bit 7: the GBA's 75%) and 100%.
pub const WAVE_VOLUME: [u8; 16] = [0, 0, 0x60, 0x60, 0x60, 0x60, 0x40, 0x40, 0x40, 0x40, 0x80, 0x80, 0x80, 0x80, 0x20, 0x20];

/// `umul3232H32`: the high word of a 32 by 32 bit product.
fn mul_high(a: u32, b: u32) -> u32 {
    ((a as u64 * b as u64) >> 32) as u32
}

/// `gScaleTable` and `gFreqTable`: key `k`'s step, 2^31 / 2^((178 - k) / 12)
/// roughly.
fn scale(key: u32) -> u32 {
    FREQ[(key % 12) as usize] >> (14 - key / 12)
}

/// `MidiKeyToFreq`: a Direct Sound note's playback rate for a sample of
/// `rate` (Hz in 1/1024, at key 60), at `key` plus `fine`/256 of a
/// semitone. The mixer steps through the sample by this times its
/// `divFreq` each output sample.
pub fn midi_key_to_freq(rate: u32, key: u8, fine: u8) -> u32 {
    let (key, fine) = if key > 178 { (178, 0xFF) } else { (key as u32, fine) };
    let low = scale(key);
    let high = scale(key + 1);
    mul_high(rate, low.wrapping_add(mul_high(high.wrapping_sub(low), (fine as u32) << 24)))
}

/// `MidiKeyToCgbFreq`: a PSG note's frequency register for PSG channel
/// `channel` (1..=4) at `key` plus `fine`/256 of a semitone (NR43 for the
/// noise channel).
pub fn midi_key_to_cgb_freq(channel: usize, key: u8, fine: u8) -> u32 {
    if channel == 4 {
        let k = if key <= 20 { 0 } else { (key - 21).min(59) };
        return NOISE[k as usize] as u32;
    }
    let (k, fine) = if key <= 35 {
        (0, 0)
    } else if key - 36 > 130 {
        (130, 255)
    } else {
        ((key - 36) as u32, fine)
    };
    let step = |k: u32| (CGB_FREQ[(k % 12) as usize] as i32) >> (k / 12);
    let (low, high) = (step(k), step(k + 1));
    (low + ((fine as i32 * (high - low)) >> 8) + 2048) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables_follow_their_rules() {
        // gScaleTable's octave rows: 0xE0..=0xEB, 0xD0.., down to 0x00..
        assert_eq!(scale(60), FREQ[0] >> 9);
        // C2 (key 36) on the PSG: 131072 / (2048 - 44) Hz = 65.4 Hz.
        assert_eq!(midi_key_to_cgb_freq(1, 36, 0), 44);
        assert_eq!(midi_key_to_cgb_freq(1, 48, 0), 2048 - 1002);
        assert_eq!(midi_key_to_cgb_freq(4, 10, 0), 0xD7);
        // A sample at its own rate at key 60.
        let rate = 13379 * 1024;
        assert_eq!(midi_key_to_freq(rate, 60, 0), mul_high(rate, FREQ[0] >> 9));
        assert!(midi_key_to_freq(rate, 72, 0) > midi_key_to_freq(rate, 71, 255));
    }
}
