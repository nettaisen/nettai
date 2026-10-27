//! Check the hand-drawn frames (each game's `extract/shapes/` in its content, and its support packs') against the
//! pictures extracted from the game's ROMs (`$NETTAI_ROMS`): what extraction says of them, and with a folder given, each
//! picture written there beside its drawing (`nettai_extract::shapes`, `NETTAI_SHAPES_SOURCES`). With `--recolor`,
//! each of the game's own hand-drawn files has its fills made its palette entries' colors in the extracted palettes
//! (for viewing the files). The drawings are the content's at `--content DIR`, else the repository's content/
//! ($NETTAI_CONTENT).
//!
//!     cargo run -p nettai-extract --example shapes_check -- exe6 [OUT] [--recolor] [--content DIR]

fn main() {
    let mut args: Vec<String> = Vec::new();
    let (mut recolor, mut content) = (false, None);
    let mut given = std::env::args().skip(1);
    while let Some(a) = given.next() {
        match a.as_str() {
            "--recolor" => recolor = true,
            "--content" => content = Some(std::path::PathBuf::from(given.next().expect("--content needs a directory"))),
            _ => args.push(a),
        }
    }
    let content = content.unwrap_or_else(nettai_content::index::content);
    let Some(game) = args.first() else { panic!("usage: shapes_check <game> [out] [--recolor] [--content DIR]") };
    if let Some(out) = args.get(1) {
        // SAFETY: set before any thread starts.
        unsafe { std::env::set_var("NETTAI_SHAPES_SOURCES", out) };
    }
    let roms = nettai_extract::RomSource::from_env().unwrap_or_else(|e| panic!("{e}"));
    let extracted = nettai_extract::extract(&content, game, &roms).unwrap_or_else(|e| panic!("{e}"));
    if recolor {
        let drawings = nettai_extract::shapes::Drawings::of_content(&content, game).unwrap_or_else(|e| panic!("{e}"));
        let root = content.join(game).join(nettai_extract::shapes::SHAPES);
        for (name, text) in drawings.own_svgs() {
            let colored = nettai_assets::shapes::recolor(text, &|pp, c| nettai_extract::shapes::view(&extracted.graphics, name, pp, c));
            std::fs::write(root.join(format!("{name}.svg")), colored).unwrap();
        }
    }
    // (What reading the frames said as the game's ROMs were extracted.)
    for p in extracted.warnings.iter().filter_map(|w| w.strip_prefix("frames: ")) {
        println!("{p}");
    }
}
