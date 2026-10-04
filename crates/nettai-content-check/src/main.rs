//! `nettai-content-check [content dir]`: type-check the content (content/,
//! its packs) against their declarations, lint it, and check what its
//! manifests and requires name. Exits non-zero on problems.

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "content".to_string());
    let path = std::path::Path::new(&dir);
    let checked = nettai_content_check::check_content(path);
    match checked {
        Ok((n, problems)) if problems.is_empty() => println!("{dir}: {n} modules type-check"),
        Ok((_, problems)) => {
            for p in &problems {
                eprintln!("{p}");
            }
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }
}
