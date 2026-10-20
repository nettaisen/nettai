//! The hand-drawn frames (`shapes/`) compiled in: `OUT_DIR/drawings.rs`
//! lists each file under `shapes/` by its path from there, with its text.

use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else if p.extension().is_some_and(|x| x == "svg" || x == "same") {
            out.push(p);
        }
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("shapes");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut found = Vec::new();
    files(&root, &mut found);
    found.sort();
    let mut code = String::from("/// The hand-drawn files, by path from `shapes/`.\npub static DRAWINGS: &[(&str, &str)] = &[\n");
    for p in &found {
        println!("cargo:rerun-if-changed={}", p.display());
        let rel = p.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        code += &format!("    ({rel:?}, include_str!({:?})),\n", p.display().to_string());
    }
    code += "];\n";
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("drawings.rs");
    std::fs::write(out, code).unwrap();
}
