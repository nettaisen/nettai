//! A frame's RGB made 4:2:0 YUV for a phone's encoder, which takes no RGB: NV12 (a plane of luma, then one of the
//! two chroma samples interleaved), as the desktop's ffmpeg makes it of the same frames (`ffmpeg`'s
//! `scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area`): BT.709's matrix, the video
//! range (luma 16 to 235, chroma 16 to 240), and each chroma sample the mean of the two by two pixels it covers.
//! Both phones' encoders are given these bytes, so the picture they code is the same; the desktop's ffmpeg makes
//! its own of the same RGB, to within one of these (`tests`).

/// BT.709's luma weights, and its chroma's scale (`2 (1 - Kb)`, `2 (1 - Kr)`).
const KR: f32 = 0.2126;
const KB: f32 = 0.0722;
const KG: f32 = 1.0 - KR - KB;

/// The NV12 of a `width` by `height` frame's RGB (both even, as every video's are: a multiple of 240 by 160):
/// `width * height` luma bytes, then `width * height / 2` of chroma, Cb and Cr in turn, written into `out` (made
/// that long).
pub(crate) fn nv12(rgb: &[u8], width: usize, height: usize, out: &mut Vec<u8>) {
    assert!(width % 2 == 0 && height % 2 == 0, "a 4:2:0 picture's sides are even");
    assert_eq!(rgb.len(), width * height * 3, "a frame's pixels are its width by its height");
    out.resize(width * height * 3 / 2, 0);
    let (luma, chroma) = out.split_at_mut(width * height);
    for (y, px) in luma.iter_mut().zip(rgb.chunks_exact(3)) {
        let (r, g, b) = (px[0] as f32, px[1] as f32, px[2] as f32);
        *y = video_range(16.0 + 219.0 / 255.0 * (KR * r + KG * g + KB * b));
    }
    let row = width * 3;
    for (cy, pair) in chroma.chunks_exact_mut(width).enumerate() {
        let (top, bottom) = (&rgb[2 * cy * row..][..row], &rgb[(2 * cy + 1) * row..][..row]);
        for (cx, cbcr) in pair.chunks_exact_mut(2).enumerate() {
            let at = cx * 6;
            let mean = |c: usize| (top[at + c] as f32 + top[at + 3 + c] as f32 + bottom[at + c] as f32 + bottom[at + 3 + c] as f32) / 4.0;
            let (r, g, b) = (mean(0), mean(1), mean(2));
            let y = KR * r + KG * g + KB * b;
            cbcr[0] = video_range(128.0 + 224.0 / 255.0 * (b - y) / (2.0 * (1.0 - KB)));
            cbcr[1] = video_range(128.0 + 224.0 / 255.0 * (r - y) / (2.0 * (1.0 - KR)));
        }
    }
}

/// A sample rounded to the nearest byte.
fn video_range(v: f32) -> u8 {
    (v + 0.5).clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::nv12;

    /// The ends of the range: black, white and the primaries, as BT.709's video range has them.
    #[test]
    fn the_ends_are_the_video_ranges() {
        let mut out = Vec::new();
        let px = |rgb: [u8; 3]| -> (u8, u8, u8) {
            let mut out = Vec::new();
            nv12(&rgb.repeat(4), 2, 2, &mut out);
            (out[0], out[4], out[5])
        };
        assert_eq!(px([0, 0, 0]), (16, 128, 128));
        assert_eq!(px([255, 255, 255]), (235, 128, 128));
        assert_eq!(px([255, 0, 0]), (63, 102, 240));
        assert_eq!(px([0, 0, 255]), (32, 240, 118));
        // (The chroma of two by two different pixels is their mean's.)
        nv12(&[[255, 0, 0], [0, 0, 255], [255, 0, 0], [0, 0, 255]].concat(), 2, 2, &mut out);
        assert_eq!(out.len(), 6);
        assert_eq!(&out[4..], &[171, 179]);
    }

    /// The NV12 is ffmpeg's, as the desktop's encoder makes it of the same frame, to within one (ffmpeg installed;
    /// `cargo test -p nettai-video -- --ignored`): the luma and the chroma of a frame of every kind of edge.
    #[test]
    #[ignore]
    fn the_nv12_is_ffmpegs() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let (width, height) = (64usize, 32usize);
        // (Pixel art at twice the frame: two by two blocks of colors, and a ramp.)
        let rgb: Vec<u8> = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let (bx, by) = (x / 2, y / 2);
                let n = (bx * 37 + by * 91) as u8;
                [n.wrapping_mul(7), n.wrapping_mul(13).wrapping_add(x as u8), (by * 8) as u8]
            })
            .collect();
        let mut ours = Vec::new();
        nv12(&rgb, width, height, &mut ours);
        let size = format!("{width}x{height}");
        let filter = "scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format=nv12";
        let mut child = Command::new("ffmpeg")
            .args(["-v", "error", "-f", "rawvideo", "-pixel_format", "rgb24", "-video_size", &size, "-i", "pipe:0", "-vf", filter, "-f", "rawvideo", "pipe:1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("ffmpeg");
        child.stdin.take().unwrap().write_all(&rgb).unwrap();
        let theirs = child.wait_with_output().unwrap().stdout;
        assert_eq!(theirs.len(), ours.len());
        let worst = ours.iter().zip(&theirs).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        assert!(worst <= 1, "the NV12 is {worst} off ffmpeg's");
    }
}
