//! Check the hand-drawn frames (`shapes/`) against a pack's pictures again,
//! without extracting it again: what extraction says of them, and with a
//! folder given, each picture written there beside its drawing
//! (`nettai_extract::shapes`, `NETTAI_SHAPES_SOURCES`). With `--recolor`,
//! each hand-drawn file's fills are made its palette entries' colors in the
//! pack's palettes first (for viewing the files).
//!
//!     cargo run -p nettai-extract --example shapes_check -- exe6 data/exe6 [OUT] [--recolor]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).filter(|a| a != "--recolor").collect();
    let recolor = std::env::args().any(|a| a == "--recolor");
    let (Some(game), Some(pack)) = (args.first(), args.get(1)) else { panic!("usage: shapes_check <game> <pack> [out] [--recolor]") };
    if let Some(out) = args.get(2) {
        // SAFETY: set before any thread starts.
        unsafe { std::env::set_var("NETTAI_SHAPES_SOURCES", out) };
    }
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
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("shapes").join(game);
        let mut stack = vec![root.clone()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "svg") {
                    let name = p.strip_prefix(&root).unwrap().to_string_lossy().trim_end_matches(".svg").to_string();
                    let text = std::fs::read_to_string(&p).unwrap();
                    let colored = nettai_content::shapes::recolor(&text, &|pp, c| nettai_extract::shapes::view(&b, &name, pp, c));
                    std::fs::write(&p, colored).unwrap();
                }
            }
        }
    }
    for p in nettai_extract::shapes::read(&mut b, game, &banner) {
        println!("{p}");
    }
}
