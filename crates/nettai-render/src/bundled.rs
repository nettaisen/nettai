//! The bundled fonts' big files, which the repository holds as WOFF2: Noto Sans CJK's cuts (`vfont`), Fusion
//! Pixel's (`pixeltext`) and M PLUS 1 Code (the app's code face, which the app registers: this crate draws nothing
//! with it). The program holds the WOFF2 files as they are, about a third of the fonts' size; the first time a font
//! is wanted its file is unpacked into memory, once, and kept for the program's life, and what draws with the font
//! borrows its bytes from there.

use crate::vfont::Cut;
use std::sync::{Mutex, OnceLock, PoisonError};

/// Noto Sans CJK's JP, SC and TC, by `Cut`'s order (fonts/noto-sans-cjk).
static NOTO_SANS_CJK: [Packed; 3] = [
    Packed::new(include_bytes!("../fonts/noto-sans-cjk/NotoSansJP[wght].woff2")),
    Packed::new(include_bytes!("../fonts/noto-sans-cjk/NotoSansSC[wght].woff2")),
    Packed::new(include_bytes!("../fonts/noto-sans-cjk/NotoSansTC[wght].woff2")),
];
/// Fusion Pixel 12px's Simplified and Traditional cuts, then its bold build's (fonts/fusion-pixel,
/// fonts/fusion-bold-pixel).
static FUSION_PIXEL: [Packed; 4] = [
    Packed::new(include_bytes!("../fonts/fusion-pixel/fusion-pixel-12px-monospaced-zh_hans.woff2")),
    Packed::new(include_bytes!("../fonts/fusion-pixel/fusion-pixel-12px-monospaced-zh_tw.woff2")),
    Packed::new(include_bytes!("../fonts/fusion-bold-pixel/fusion-bold-pixel-12px-monospaced-zh_hans.woff2")),
    Packed::new(include_bytes!("../fonts/fusion-bold-pixel/fusion-bold-pixel-12px-monospaced-zh_tw.woff2")),
];
/// M PLUS 1 Code (fonts/mplus1code).
static M_PLUS_1_CODE: Packed = Packed::new(include_bytes!("../fonts/mplus1code/MPLUS1Code-VariableFont_wght.woff2"));

/// Noto Sans CJK's font of `cut` (Google Fonts' variable cut, weights 100 to 900).
pub fn noto_sans_cjk(cut: Cut) -> &'static [u8] {
    NOTO_SANS_CJK[cut as usize].font()
}

/// Fusion Pixel 12px's font, or its bold build's, in Simplified Chinese's forms or Traditional's.
pub fn fusion_pixel(bold: bool, traditional: bool) -> &'static [u8] {
    FUSION_PIXEL[2 * usize::from(bold) + usize::from(traditional)].font()
}

/// M PLUS 1 Code's font (the variable font, weights 100 to 700).
pub fn m_plus_1_code() -> &'static [u8] {
    M_PLUS_1_CODE.font()
}

/// A WOFF2 file, and the font it unpacks to.
struct Packed {
    woff2: &'static [u8],
    unpacked: OnceLock<Box<[u8]>>,
    /// Held while one thread unpacks, taken as the browser's page may take it (`nettai_threads::lock`:
    /// its main thread may not wait).
    unpacking: Mutex<()>,
}

impl Packed {
    const fn new(woff2: &'static [u8]) -> Packed {
        Packed { woff2, unpacked: OnceLock::new(), unpacking: Mutex::new(()) }
    }

    /// The font: unpacked the first time.
    fn font(&'static self) -> &'static [u8] {
        if let Some(font) = self.unpacked.get() {
            return font;
        }
        let _alone = nettai_threads::lock(&self.unpacking).unwrap_or_else(PoisonError::into_inner);
        // (Another thread may have unpacked it while this one waited.)
        self.unpacked.get_or_init(|| wuff::decompress_woff2(self.woff2).expect("the bundled fonts unpack").into_boxed_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The family a font names itself by (its name table's family, English).
    fn family(font: &[u8]) -> String {
        let font = swash::FontRef::from_index(font, 0).expect("the font parses");
        font.localized_strings().find_by_id(swash::StringId::Family, Some("en")).expect("the font names its family").chars().collect()
    }

    /// Every file unpacks to a font that parses, the one it is asked for by.
    #[test]
    fn each_file_unpacks_to_its_font() {
        for (cut, name) in [(Cut::Japanese, "JP"), (Cut::Simplified, "SC"), (Cut::Traditional, "TC")] {
            assert!(family(noto_sans_cjk(cut)).starts_with(&format!("Noto Sans {name}")), "Noto Sans {name}");
        }
        for (bold, build) in [(false, "Fusion Pixel"), (true, "Fusion Bold Pixel")] {
            for (traditional, forms) in [(false, "zh-Hans"), (true, "zh-TW")] {
                assert_eq!(family(fusion_pixel(bold, traditional)), format!("{build} 12px Mono {forms}"));
            }
        }
        assert_eq!(family(m_plus_1_code()), "M PLUS 1 Code");
    }

    /// A font is unpacked once: its bytes are the same memory each time it is asked for.
    #[test]
    fn a_font_unpacks_once() {
        let first = m_plus_1_code().as_ptr();
        let threads: Vec<_> = (0..4).map(|_| std::thread::spawn(|| m_plus_1_code().as_ptr() as usize)).collect();
        for t in threads {
            assert_eq!(t.join().unwrap(), first as usize);
        }
    }
}
