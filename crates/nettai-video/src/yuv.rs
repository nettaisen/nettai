//! A video's pictures made 4:2:0 YUV, the one input every platform's codecs are given: NV12 (a plane of luma, then
//! one of the two chroma samples interleaved), BT.709's matrix, the video range (luma 16 to 235, chroma 16 to 240),
//! and each chroma sample the mean of the two by two pixels it covers, as ffmpeg's scaler makes it
//! (`scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area`: the same to within one,
//! `tests`). It is made once, on the threads that present the frames (`presenters`), of the pictures as the renderer
//! makes them.

/// BT.709's luma weights, and its chroma's scale (`2 (1 - Kb)`, `2 (1 - Kr)`).
const KR: f32 = 0.2126;
const KB: f32 = 0.0722;
const KG: f32 = 1.0 - KR - KB;

/// The NV12 of `pictures` side by side, each `width` by `height` (both even, as every video's are: a multiple of 240
/// by 160) of pixels 0x00RRGGBB (as `nettai_render::present` makes them): the luma, a byte a pixel, then the chroma,
/// Cb and Cr in turn a byte each for each two by two pixels, written into `out` (made that long). A side's width is
/// even, so no chroma sample covers two sides.
pub(crate) fn nv12(pictures: &[&[u32]], width: usize, height: usize, out: &mut Vec<u8>) {
    assert!(width % 2 == 0 && height % 2 == 0, "a 4:2:0 picture's sides are even");
    assert!(pictures.iter().all(|p| p.len() == width * height), "a picture's pixels are its width by its height");
    let across = width * pictures.len();
    out.resize(across * height * 3 / 2, 0);
    let (luma, chroma) = out.split_at_mut(across * height);
    let rgb = |c: u32| ((c >> 16 & 0xFF) as f32, (c >> 8 & 0xFF) as f32, (c & 0xFF) as f32);
    for (side, picture) in pictures.iter().enumerate() {
        for (y, row) in picture.chunks_exact(width).enumerate() {
            for (to, &c) in luma[y * across + side * width..][..width].iter_mut().zip(row) {
                let (r, g, b) = rgb(c);
                *to = video_range(16.0 + 219.0 / 255.0 * (KR * r + KG * g + KB * b));
            }
        }
        for cy in 0..height / 2 {
            let (top, bottom) = (&picture[2 * cy * width..][..width], &picture[(2 * cy + 1) * width..][..width]);
            for (cx, cbcr) in chroma[cy * across + side * width..][..width].chunks_exact_mut(2).enumerate() {
                let four = [top[2 * cx], top[2 * cx + 1], bottom[2 * cx], bottom[2 * cx + 1]].map(rgb);
                let mean = |f: fn(&(f32, f32, f32)) -> f32| four.iter().map(f).sum::<f32>() / 4.0;
                let (r, g, b) = (mean(|p| p.0), mean(|p| p.1), mean(|p| p.2));
                let y = KR * r + KG * g + KB * b;
                cbcr[0] = video_range(128.0 + 224.0 / 255.0 * (b - y) / (2.0 * (1.0 - KB)));
                cbcr[1] = video_range(128.0 + 224.0 / 255.0 * (r - y) / (2.0 * (1.0 - KR)));
            }
        }
    }
}

/// RGB bytes as the renderer's pixels (0x00RRGGBB), for a picture made of them (the tests').
#[cfg(test)]
pub(crate) fn pixels(rgb: &[u8]) -> Vec<u32> {
    rgb.chunks_exact(3).map(|p| (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32).collect()
}

/// A sample rounded to the nearest byte.
fn video_range(v: f32) -> u8 {
    (v + 0.5).clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::pixels;

    fn nv12(rgb: &[u8], width: usize, height: usize, out: &mut Vec<u8>) {
        super::nv12(&[&pixels(rgb)], width, height, out);
    }

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

    /// The NV12 is ffmpeg's scaler's of the same frame, to within one (ffmpeg installed; `cargo test -p nettai-video
    /// -- --ignored`): the luma and the chroma of a frame of every kind of edge.
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

    /// Pictures side by side are each one's NV12, row by row in each plane: the left's columns, then the right's.
    #[test]
    fn side_by_side_is_each_ones_rows() {
        let (w, h) = (4usize, 2usize);
        let left: Vec<u32> = (0..w * h).map(|n| 0x10_20_30 * n as u32).collect();
        let right: Vec<u32> = (0..w * h).map(|n| 0xFF_00_80 ^ n as u32).collect();
        let (mut both, mut l, mut r) = (Vec::new(), Vec::new(), Vec::new());
        super::nv12(&[&left, &right], w, h, &mut both);
        super::nv12(&[&left], w, h, &mut l);
        super::nv12(&[&right], w, h, &mut r);
        let rows = |plane: &[u8], width: usize| plane.chunks_exact(width).map(<[u8]>::to_vec).collect::<Vec<_>>();
        let (lr, rr, br) = (rows(&l, w), rows(&r, w), rows(&both, 2 * w));
        // (Luma's two rows, then chroma's one: each `w` bytes a side.)
        for (n, row) in br.iter().enumerate() {
            assert_eq!(row, &[lr[n].clone(), rr[n].clone()].concat(), "row {n}");
        }
    }
}
