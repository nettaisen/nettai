//! The bundled fonts' big files, packed (`bundled`): each group of files below one zstd frame in `OUT_DIR`,
//! which the program unpacks once at run time. A group's files are packed together, at zstd's strongest level
//! short of its ultra levels and with its long-distance matching over all of them, because they share most of
//! their glyphs: Noto Sans CJK's three cuts take 14.3 MB so against 19.7 MB packed one by one, Fusion Pixel's
//! four 1.8 MB against 3.1 MB. Also written: `sizes.rs`, each group's files' sizes in their order here.

use std::fmt::Write as _;
use std::path::PathBuf;

/// Each group: the name of its packed file and of its sizes' constant, and its files, from the crate's folder,
/// in the order `bundled` reads them by.
const GROUPS: [(&str, &str, &[&str]); 3] = [
    // By `vfont::Cut`'s order.
    (
        "noto-sans-cjk",
        "NOTO_SANS_CJK",
        &["fonts/noto-sans-cjk/NotoSansJP[wght].ttf", "fonts/noto-sans-cjk/NotoSansSC[wght].ttf", "fonts/noto-sans-cjk/NotoSansTC[wght].ttf"],
    ),
    // The regular build's Simplified and Traditional cuts, then the bold build's.
    (
        "fusion-pixel",
        "FUSION_PIXEL",
        &[
            "fonts/fusion-pixel/fusion-pixel-12px-monospaced-zh_hans.otf",
            "fonts/fusion-pixel/fusion-pixel-12px-monospaced-zh_tw.otf",
            "fonts/fusion-bold-pixel/fusion-bold-pixel-12px-monospaced-zh_hans.otf",
            "fonts/fusion-bold-pixel/fusion-bold-pixel-12px-monospaced-zh_tw.otf",
        ],
    ),
    ("mplus1code", "M_PLUS_1_CODE", &["fonts/mplus1code/MPLUS1Code-VariableFont_wght.ttf"]),
];

/// zstd's strongest level without its ultra levels (which win a hundredth more).
const LEVEL: i32 = 19;

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets it"));
    println!("cargo:rerun-if-changed=build.rs");
    // (The groups at once, each on a thread of its own: Noto Sans CJK's takes the longest.)
    let sizes: Vec<Vec<usize>> = std::thread::scope(|s| {
        let packing: Vec<_> = GROUPS
            .iter()
            .map(|&(name, _, files)| {
                let (manifest, out) = (&manifest, &out);
                s.spawn(move || {
                    let mut all = Vec::new();
                    let mut sizes = Vec::new();
                    for file in files {
                        let path = manifest.join(file);
                        println!("cargo:rerun-if-changed={}", path.display());
                        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()));
                        sizes.push(bytes.len());
                        all.extend(bytes);
                    }
                    std::fs::write(out.join(format!("{name}.zst")), pack(&all)).expect("the build's folder is writable");
                    sizes
                })
            })
            .collect();
        packing.into_iter().map(|p| p.join().expect("a group packs")).collect()
    });
    let mut rs = String::from("// Written by build.rs: each packed group's files' sizes, in their order there.\n");
    for ((_, constant, _), sizes) in GROUPS.iter().zip(&sizes) {
        writeln!(rs, "const {constant}_SIZES: [usize; {}] = {sizes:?};", sizes.len()).unwrap();
    }
    std::fs::write(out.join("sizes.rs"), rs).expect("the build's folder is writable");
}

/// `data` as one zstd frame that states its size, matched over its whole length (the window is cut to the
/// data's size: the decoder never keeps more).
fn pack(data: &[u8]) -> Vec<u8> {
    use zstd::zstd_safe::CParameter;
    let mut packer = zstd::bulk::Compressor::new(LEVEL).expect("zstd starts");
    packer.set_parameter(CParameter::EnableLongDistanceMatching(true)).expect("zstd takes long matching");
    packer.set_parameter(CParameter::WindowLog(27)).expect("zstd takes the window");
    packer.compress(data).expect("zstd packs")
}
