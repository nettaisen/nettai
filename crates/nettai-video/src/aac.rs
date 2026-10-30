//! AAC as the codecs make it, made the MP4's: its AudioSpecificConfig (what the track's description holds), and
//! ADTS (ffmpeg's raw AAC: each frame after a header that says its config and its length) read into raw frames.

/// The sample rates AAC's configs number (ISO 14496-3's sampling frequency index), up to the ones used.
#[cfg_attr(not(any(target_os = "ios", test)), allow(dead_code))]
const RATES: [u32; 12] = [96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000];

/// AAC-LC's audio object type.
#[cfg_attr(not(any(target_os = "ios", test)), allow(dead_code))]
pub(crate) const LC: u8 = 2;

/// An AudioSpecificConfig's fields: the audio object type, the sample rate's index, the channels' configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    pub object: u8,
    pub rate: u8,
    pub channels: u8,
}

impl Config {
    /// AAC-LC at `rate` samples a second, `channels` of them (a rate AAC has): what iOS's converter is asked for.
    #[cfg_attr(not(any(target_os = "ios", test)), allow(dead_code))]
    pub(crate) fn lc(rate: u32, channels: u8) -> Config {
        let index = RATES.iter().position(|&r| r == rate).expect("a rate AAC has") as u8;
        Config { object: LC, rate: index, channels }
    }

    /// The config's two bytes: five bits of object type, four of rate, four of channels, three of zero (as the
    /// desktop's ADTS and iOS's converter say it).
    #[cfg_attr(not(any(desktop, target_os = "ios", test)), allow(dead_code))]
    pub(crate) fn bytes(self) -> Vec<u8> {
        let bits = (self.object as u16) << 11 | (self.rate as u16) << 7 | (self.channels as u16) << 3;
        bits.to_be_bytes().to_vec()
    }

    /// The fields of an AudioSpecificConfig's first two bytes (none: shorter).
    pub(crate) fn of(bytes: &[u8]) -> Option<Config> {
        let [a, b, ..] = *bytes else { return None };
        Some(Config { object: a >> 3, rate: (a & 7) << 1 | b >> 7, channels: (b >> 3) & 0xF })
    }
}

/// A stream of ADTS as it comes (ffmpeg's), read into raw AAC frames and the config its headers say.
#[cfg(any(desktop, test))]
#[derive(Default)]
pub(crate) struct Adts {
    held: Vec<u8>,
    /// The config the first header said.
    pub config: Option<Config>,
}

#[cfg(any(desktop, test))]
impl Adts {
    /// `bytes` more of the stream: the frames it completed, their headers left off. An error: what isn't ADTS.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
        self.held.extend_from_slice(bytes);
        let mut out = Vec::new();
        let mut at = 0;
        while self.held.len() - at >= 7 {
            let h = &self.held[at..at + 7];
            if h[0] != 0xFF || h[1] & 0xF6 != 0xF0 {
                return Err(format!("ADTS out of step at byte {at} of what was held"));
            }
            // (No CRC: the header is 7 bytes; with one, 9.)
            let header = if h[1] & 1 == 1 { 7 } else { 9 };
            let length = ((h[3] as usize & 3) << 11) | (h[4] as usize) << 3 | (h[5] as usize) >> 5;
            if length < header {
                return Err(format!("an ADTS frame of {length} bytes"));
            }
            if self.held.len() - at < length {
                break;
            }
            self.config.get_or_insert(Config { object: (h[2] >> 6) + 1, rate: (h[2] >> 2) & 0xF, channels: (h[2] & 1) << 2 | h[3] >> 6 });
            out.push(self.held[at + header..at + length].to_vec());
            at += length;
        }
        self.held.drain(..at);
        Ok(out)
    }

    /// The stream ended: whether a part of a frame is left over (it ended in mid-frame).
    pub(crate) fn torn(&self) -> bool {
        !self.held.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AAC-LC at 48 kHz in stereo is the config every codec makes (0x1190), read back as it was made.
    #[test]
    fn the_config_is_read_as_made() {
        let lc = Config::lc(48_000, 2);
        assert_eq!(lc.bytes(), [0x11, 0x90]);
        assert_eq!(Config::of(&lc.bytes()), Some(lc));
        assert_eq!(Config::of(&[0x11]), None);
    }

    /// ADTS is read into its frames and its config, however its bytes come.
    #[test]
    fn adts_is_read_into_frames() {
        // (AAC-LC, 48 kHz, stereo, no CRC: a header for a frame of `payload` bytes.)
        let frame = |payload: &[u8]| -> Vec<u8> {
            let length = payload.len() + 7;
            let header = [0xFF, 0xF1, 0x4C, 0x80 | (length >> 11) as u8, (length >> 3) as u8, ((length & 7) << 5) as u8 | 0x1F, 0xFC];
            [&header[..], payload].concat()
        };
        let whole = [frame(&[1, 2, 3]), frame(&[4]), frame(&[5, 6])].concat();
        for step in [1, 4, whole.len()] {
            let mut adts = Adts::default();
            let made: Vec<Vec<u8>> = whole.chunks(step).flat_map(|c| adts.push(c).unwrap()).collect();
            assert_eq!(made, [vec![1, 2, 3], vec![4], vec![5, 6]], "{step} bytes at a time");
            assert_eq!(adts.config, Some(Config::lc(48_000, 2)));
            assert!(!adts.torn());
        }
        assert!(Adts::default().push(&[0, 1, 2, 3, 4, 5, 6, 7]).is_err());
    }
}
