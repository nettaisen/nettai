//! `nettai-content-check [content dir]`: type-check the content (every
//! folder of content/, one namespace) against core.d.luau and the folders'
//! own declarations, and lint it; a single folder (content/bn6) checks
//! alone. Exits non-zero on problems.

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "content".to_string());
    let path = std::path::Path::new(&dir);
    let checked = if path.join("nettai").is_dir() {
        nettai_content_check::check_content(path)
    } else {
        nettai_content_check::check_pack(path)
    };
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
