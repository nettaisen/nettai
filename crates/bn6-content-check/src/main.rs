//! `bn6-content-check <pack dir>`: type-check a Luau content pack against
//! its core.d.luau and lint it. Exits non-zero on problems.

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "content/bn6".to_string());
    match bn6_content_check::check_pack(std::path::Path::new(&dir)) {
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
