//! The engine never reads compat (docs/design/content-model-v2.md §6.4):
//! the original's numbers for what content defines are the validator's
//! (bn6-compat), which maps the engine's identities to them. In the engine
//! and the crates it runs content through, the word appears only in
//! comments.

use std::path::Path;

/// Every `.rs` file under `dir`.
fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_engine_names_compat_only_in_comments() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for c in ["nettai-battle", "nettai-content-api", "nettai-luau"] {
        sources(&crates.join(c).join("src"), &mut files);
        let manifest = std::fs::read_to_string(crates.join(c).join("Cargo.toml")).expect("a manifest");
        assert!(!manifest.to_lowercase().contains("compat"), "{c}'s Cargo.toml names compat");
    }
    assert!(files.len() > 50, "{} sources", files.len());
    let mut found = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(&f).expect("a source");
        for (i, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if code.to_lowercase().contains("compat") {
                found.push(format!("{}:{}: {}", f.display(), i + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "the engine names compat outside comments:\n{}", found.join("\n"));
}
