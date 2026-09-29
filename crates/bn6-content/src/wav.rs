//! WAV files for Direct Sound samples: mono PCM with a `smpl` chunk that
//! holds the loop and the unity key, as samplers and wav2agb read it.

/// A mono sample as read from or written to a WAV file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wav {
    pub rate: u32,
    /// Bits per sample of the file (8 or 16).
    pub bits: u16,
    /// Signed 16-bit samples (8-bit files are widened by 256).
    pub samples: Vec<i16>,
    /// The sampler loop: first sample and last sample (inclusive).
    pub loop_points: Option<(u32, u32)>,
    /// The MIDI key the recording plays at its own rate.
    pub unity_key: Option<u8>,
}

impl Wav {
    /// 8-bit samples (the GBA's signed PCM) at `rate` Hz.
    pub fn from_pcm8(data: &[i8], rate: u32, loop_start: Option<u32>) -> Wav {
        Wav {
            rate,
            bits: 8,
            samples: data.iter().map(|&x| (x as i16) << 8).collect(),
            loop_points: loop_start.map(|s| (s, data.len() as u32 - 1)),
            unity_key: Some(60),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut fmt = Vec::new();
        let block = (self.bits / 8) as u32;
        fmt.extend_from_slice(&1u16.to_le_bytes()); // PCM
        fmt.extend_from_slice(&1u16.to_le_bytes()); // mono
        fmt.extend_from_slice(&self.rate.to_le_bytes());
        fmt.extend_from_slice(&(self.rate * block).to_le_bytes());
        fmt.extend_from_slice(&(block as u16).to_le_bytes());
        fmt.extend_from_slice(&self.bits.to_le_bytes());
        let data: Vec<u8> = match self.bits {
            8 => self.samples.iter().map(|&s| ((s >> 8) as i8 as u8) ^ 0x80).collect(),
            _ => self.samples.iter().flat_map(|s| s.to_le_bytes()).collect(),
        };
        let mut chunks: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"fmt ", fmt), (b"data", data)];
        if self.loop_points.is_some() || self.unity_key.is_some() {
            let mut s = Vec::new();
            let period = (1_000_000_000u64 / self.rate.max(1) as u64) as u32;
            for v in [0, 0, period, self.unity_key.unwrap_or(60) as u32, 0, 0, 0] {
                s.extend_from_slice(&v.to_le_bytes());
            }
            s.extend_from_slice(&(self.loop_points.is_some() as u32).to_le_bytes());
            s.extend_from_slice(&0u32.to_le_bytes());
            if let Some((start, end)) = self.loop_points {
                for v in [0, 0, start, end, 0, 0] {
                    s.extend_from_slice(&v.to_le_bytes());
                }
            }
            chunks.push((b"smpl", s));
        }
        let mut body = b"WAVE".to_vec();
        for (id, c) in chunks {
            body.extend_from_slice(id);
            body.extend_from_slice(&(c.len() as u32).to_le_bytes());
            let odd = c.len() % 2 == 1;
            body.extend(c);
            if odd {
                body.push(0);
            }
        }
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend(body);
        out
    }

    pub fn from_bytes(b: &[u8]) -> Result<Wav, String> {
        if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
            return Err("not a WAV file".into());
        }
        let u16at = |d: &[u8], o: usize| u16::from_le_bytes([d[o], d[o + 1]]);
        let u32at = |d: &[u8], o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap());
        let mut at = 12;
        let (mut fmt, mut data, mut smpl) = (None, None, None);
        while at + 8 <= b.len() {
            let id = &b[at..at + 4];
            let len = u32at(b, at + 4) as usize;
            let body = b.get(at + 8..at + 8 + len).ok_or("truncated chunk")?;
            match id {
                b"fmt " => fmt = Some(body),
                b"data" => data = Some(body),
                b"smpl" => smpl = Some(body),
                _ => {}
            }
            at += 8 + len + len % 2;
        }
        let fmt = fmt.ok_or("no fmt chunk")?;
        let data = data.ok_or("no data chunk")?;
        let (format, channels, rate, bits) = (u16at(fmt, 0), u16at(fmt, 2), u32at(fmt, 4), u16at(fmt, 14));
        let format = if format == 0xFFFE && fmt.len() >= 26 { u16at(fmt, 24) } else { format };
        if format != 1 {
            return Err(format!("sample format {format} (only integer PCM: re-save as 8- or 16-bit PCM)"));
        }
        if channels != 1 {
            return Err(format!("{channels} channels: the GBA plays mono samples; mix it down to mono"));
        }
        let samples = match bits {
            8 => data.iter().map(|&x| ((x ^ 0x80) as i8 as i16) << 8).collect(),
            16 => data.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes([c[0], c[1]])).collect(),
            24 => data.as_chunks::<3>().0.iter().map(|c| i16::from_le_bytes([c[1], c[2]])).collect(),
            _ => return Err(format!("{bits}-bit samples (use 8 or 16)")),
        };
        let (mut loop_points, mut unity_key) = (None, None);
        if let Some(s) = smpl.filter(|s| s.len() >= 36) {
            unity_key = Some(u32at(s, 12).min(127) as u8);
            if u32at(s, 28) > 0 && s.len() >= 60 {
                loop_points = Some((u32at(s, 44), u32at(s, 48)));
            }
        }
        Ok(Wav { rate, bits, samples, loop_points, unity_key })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_with_loop() {
        let data: Vec<i8> = (0..100).map(|i| (i * 3 - 128) as i8).collect();
        let w = Wav::from_pcm8(&data, 13379, Some(40));
        let back = Wav::from_bytes(&w.to_bytes()).unwrap();
        assert_eq!(back, w);
        assert_eq!(back.loop_points, Some((40, 99)));
        let pcm: Vec<i8> = back.samples.iter().map(|&s| (s >> 8) as i8).collect();
        assert_eq!(pcm, data);
    }
}
