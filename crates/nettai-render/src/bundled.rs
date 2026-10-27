//! The bundled fonts' big files, which the program holds packed: Noto Sans CJK's cuts (`vfont`), Fusion Pixel's
//! (`pixeltext`) and M PLUS 1 Code (the app's code face, which the app registers: this crate draws nothing with
//! it). The build script (build.rs) packs each group of them into one zstd frame, about a third of their size;
//! the first time one of a group's files is wanted the whole group is unpacked into memory, once, and kept for
//! the program's life, and what draws with a file borrows its bytes from there. The files are the fonts' own,
//! byte for byte (the tests compare them with the folder's).

use crate::vfont::Cut;
use std::sync::{Mutex, OnceLock, PoisonError};

include!(concat!(env!("OUT_DIR"), "/sizes.rs"));

/// Noto Sans CJK's JP, SC and TC (fonts/noto-sans-cjk).
static NOTO_SANS_CJK: Group = Group::new(include_bytes!(concat!(env!("OUT_DIR"), "/noto-sans-cjk.zst")), &NOTO_SANS_CJK_SIZES);
/// Fusion Pixel 12px's Simplified and Traditional cuts, then its bold build's (fonts/fusion-pixel,
/// fonts/fusion-bold-pixel).
static FUSION_PIXEL: Group = Group::new(include_bytes!(concat!(env!("OUT_DIR"), "/fusion-pixel.zst")), &FUSION_PIXEL_SIZES);
/// M PLUS 1 Code (fonts/mplus1code).
static M_PLUS_1_CODE: Group = Group::new(include_bytes!(concat!(env!("OUT_DIR"), "/mplus1code.zst")), &M_PLUS_1_CODE_SIZES);

/// Noto Sans CJK's file of `cut` (Google Fonts' variable cut, weights 100 to 900).
pub fn noto_sans_cjk(cut: Cut) -> &'static [u8] {
    NOTO_SANS_CJK.file(cut as usize)
}

/// Fusion Pixel 12px's file, or its bold build's, in Simplified Chinese's forms or Traditional's.
pub fn fusion_pixel(bold: bool, traditional: bool) -> &'static [u8] {
    FUSION_PIXEL.file(2 * usize::from(bold) + usize::from(traditional))
}

/// M PLUS 1 Code's file (the variable font, weights 100 to 700).
pub fn m_plus_1_code() -> &'static [u8] {
    M_PLUS_1_CODE.file(0)
}

/// Files packed into one frame, and where they are unpacked.
struct Group {
    packed: &'static [u8],
    sizes: &'static [usize],
    unpacked: OnceLock<Box<[u8]>>,
    /// Held while one thread unpacks, taken as the browser's page may take it (`nettai_platform::thread::lock`:
    /// its main thread may not wait).
    unpacking: Mutex<()>,
}

impl Group {
    const fn new(packed: &'static [u8], sizes: &'static [usize]) -> Group {
        Group { packed, sizes, unpacked: OnceLock::new(), unpacking: Mutex::new(()) }
    }

    /// Its `k`th file.
    fn file(&'static self, k: usize) -> &'static [u8] {
        let all = self.all();
        let start = self.sizes[..k].iter().sum::<usize>();
        &all[start..start + self.sizes[k]]
    }

    /// Its files, one after another: unpacked the first time.
    fn all(&'static self) -> &'static [u8] {
        if let Some(all) = self.unpacked.get() {
            return all;
        }
        let _alone = nettai_platform::thread::lock(&self.unpacking).unwrap_or_else(PoisonError::into_inner);
        // (Another thread may have unpacked it while this one waited.)
        self.unpacked.get_or_init(|| {
            let size = self.sizes.iter().sum::<usize>();
            let mut all = vec![0; size].into_boxed_slice();
            let unpacked = ruzstd::decoding::FrameDecoder::new().decode_all(self.packed, &mut all).expect("the bundled fonts unpack");
            assert_eq!(unpacked, size, "the bundled fonts unpack whole");
            all
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts").join(path);
        std::fs::read(&path).unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()))
    }

    /// Every file unpacks to the font's own, byte for byte, by the name it is asked for by: what draws with it
    /// draws what it drew when the file itself was built into the program.
    #[test]
    fn the_files_unpack_to_the_fonts_own() {
        for (cut, name) in [(Cut::Japanese, "JP"), (Cut::Simplified, "SC"), (Cut::Traditional, "TC")] {
            assert!(noto_sans_cjk(cut) == file(&format!("noto-sans-cjk/NotoSans{name}[wght].ttf")), "Noto Sans {name}");
        }
        for (bold, build) in [(false, "fusion-pixel"), (true, "fusion-bold-pixel")] {
            for (traditional, forms) in [(false, "zh_hans"), (true, "zh_tw")] {
                assert!(fusion_pixel(bold, traditional) == file(&format!("{build}/{build}-12px-monospaced-{forms}.otf")), "{build} {forms}");
            }
        }
        assert!(m_plus_1_code() == file("mplus1code/MPLUS1Code-VariableFont_wght.ttf"));
    }

    /// A group is unpacked once: its files are the same memory each time they are asked for.
    #[test]
    fn a_group_unpacks_once() {
        let first = m_plus_1_code().as_ptr();
        let threads: Vec<_> = (0..4).map(|_| std::thread::spawn(|| m_plus_1_code().as_ptr() as usize)).collect();
        for t in threads {
            assert_eq!(t.join().unwrap(), first as usize);
        }
    }
}
