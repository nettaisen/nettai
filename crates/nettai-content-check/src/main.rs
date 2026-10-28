//! `nettai-content-check [content dir] [--pack ID ...]`: type-check the
//! content (content/, its packs) against their declarations, lint it,
//! check what its manifests and requires name, and check each game's lock
//! (depends.lock) against the support packs checked out. With `--pack`,
//! only the packs named (`nettai-content publish` checks a game and its
//! support packs so). Exits non-zero on problems.

#![forbid(unsafe_code)]

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
    let named: Vec<&str> = only.iter().map(String::as_str).collect();
    let named = (!named.is_empty()).then_some(named.as_slice());
    let checked = match named {
        None => nettai_content_check::check_content(path),
        Some(named) => nettai_content_check::check_packs(path, named),
    };
    let checked = checked.and_then(|(n, mut problems)| {
        let (locks, notes) = nettai_content_check::locks(path, named)?;
        for note in notes {
            eprintln!("note: {note}");
        }
        problems.extend(locks);
        Ok((n, problems))
    });
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
