//! `nettai-content-check [content dir] [--pack ID ...]`: type-check the
//! content (content/, its packs) against their declarations, lint it, and
//! check what its manifests and requires name. With `--pack`, only the
//! packs named (`nettai-content publish` checks a game and its support
//! packs so). Exits non-zero on problems.

fn main() {
    let mut dir = None;
    let mut only = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pack" => only.push(args.next().unwrap_or_else(|| fail("--pack needs a pack's id"))),
            a if a.starts_with("--") => fail(&format!("unknown option {a}\nusage: nettai-content-check [content dir] [--pack ID ...]")),
            _ if dir.is_some() => fail("one content directory"),
            _ => dir = Some(arg),
        }
    }
    let dir = dir.unwrap_or_else(|| "content".to_string());
    let path = std::path::Path::new(&dir);
    let checked = if only.is_empty() {
        nettai_content_check::check_content(path)
    } else {
        nettai_content_check::check_packs(path, &only.iter().map(String::as_str).collect::<Vec<_>>())
    };
    match checked {
        Ok((n, problems)) if problems.is_empty() => println!("{dir}: {n} modules type-check"),
        Ok((_, problems)) => {
            for p in &problems {
                eprintln!("{p}");
            }
            std::process::exit(1);
        }
        Err(e) => fail(&e),
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(2);
}
