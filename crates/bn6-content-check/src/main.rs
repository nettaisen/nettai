//! `bn6-content-check <pack dir>`: type-check a Luau content pack against
//! its core.d.luau and lint it. Exits non-zero on problems.
//!
//! `bn6-content-check --deprecated <pack dir>`: each module's uses of the
//! deprecated numeric API, as `path count` lines (the ratchet's format,
//! tests/deprecated.txt), with `--list` each use.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let dir = args.iter().find(|a| !a.starts_with("--")).cloned().unwrap_or_else(|| "content/bn6".to_string());
    if flag("--deprecated") {
        let uses = bn6_content_check::deprecated_uses(std::path::Path::new(&dir)).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2)
        });
        for (path, list) in &uses {
            println!("{path} {}", list.len());
            if flag("--list") {
                for d in list {
                    println!("    line {}: {} (use {})", d.line, d.what, d.instead);
                }
            }
        }
        return;
    }
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
