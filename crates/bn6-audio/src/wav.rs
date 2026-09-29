//! Writing rendered audio as a 16-bit stereo WAV file.

use std::io::Write;

/// Stereo samples (+-1.0 full scale) as a 16-bit PCM WAV file's bytes.
pub fn to_bytes(samples: &[[f32; 2]], rate: u32) -> Vec<u8> {
    let data_len = samples.len() as u32 * 4;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&2u16.to_le_bytes()); // channels
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 4).to_le_bytes()); // bytes per second
    out.extend_from_slice(&4u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        for &x in s {
            out.extend_from_slice(&((x.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
        }
    }
    out
}

/// Write a WAV file.
pub fn write(path: impl AsRef<std::path::Path>, samples: &[[f32; 2]], rate: u32) -> std::io::Result<()> {
    std::fs::File::create(path)?.write_all(&to_bytes(samples, rate))
}
