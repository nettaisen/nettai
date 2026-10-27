//! Check the hand-drawn frames (each game's `extract/shapes/` in its content, and its support packs') against a
//! pack's pictures again, without extracting it again: what extraction says of them, and with a folder given, each
//! picture written there beside its drawing (`nettai_extract::shapes`, `NETTAI_SHAPES_SOURCES`). With
//! `--recolor`, each of the game's own hand-drawn files has its fills made its palette entries' colors in the pack's
//! palettes first (for viewing the files). The drawings are the content's at `--content DIR`, else the repository's
//! content/ ($NETTAI_CONTENT).
//!
//!     cargo run -p nettai-extract --example shapes_check -- exe6 data/exe6 [OUT] [--recolor] [--content DIR]

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
    let (Some(game), Some(pack)) = (args.first(), args.get(1)) else {
        panic!("usage: shapes_check <game> <pack> [out] [--recolor] [--content DIR]")
    };
    if let Some(out) = args.get(2) {
        // SAFETY: set before any thread starts.
        unsafe { std::env::set_var("NETTAI_SHAPES_SOURCES", out) };
    }
    let drawings = nettai_extract::shapes::Drawings::of_content(&content, game).unwrap_or_else(|e| panic!("{e}"));
    let (mut b, _) = nettai_content::pack::load_graphics(std::path::Path::new(pack)).unwrap_or_else(|r| panic!("{r}"));
    // (The banners' names: their pictures' in the pack's hud.json.)
    let mut report = nettai_content::report::Report::default();
    let dir = std::path::Path::new(pack).join("graphics/hud");
    let doc: nettai_content::hud::HudDoc = nettai_content::sprite::read_json(&dir.join("hud.json"), "hud.json", &mut report).expect("hud.json");
    let names: Vec<String> = doc
        .banners
        .iter()
        .enumerate()
        .map(|(i, b)| b.image.as_ref().map_or(format!("banner-{i}"), |im| im.file.trim_start_matches("banners/").trim_end_matches(".png").to_string()))
        .collect();
    let banner = |i: usize| names.get(i).cloned().unwrap_or_default();
    if recolor {
        let root = content.join(game).join(nettai_extract::shapes::SHAPES);
        for (name, text) in drawings.own_svgs() {
            let colored = nettai_content::shapes::recolor(text, &|pp, c| nettai_extract::shapes::view(&b, name, pp, c));
            std::fs::write(root.join(format!("{name}.svg")), colored).unwrap();
        }
    }
    for p in nettai_extract::shapes::read(&mut b, &drawings, &banner) {
        println!("{p}");
    }
}
