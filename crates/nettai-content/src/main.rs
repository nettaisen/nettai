//! `nettai-content`: check a game's content, and publish it.
//!
//!     nettai-content check <game> [--content DIR]
//!     nettai-content publish <game> [--dry-run]
//!     nettai-content lock [game ...]
//!
//! A game's assets against its content (extracted from its ROMs) are checked by
//! `nettai-tool --audit-content` (nettai-tools): nettai-extract reads this
//! crate, so this command can't extract.

mod publish;

use nettai_content::index;
use nettai_content::report::{Level, Report};
use std::path::PathBuf;

struct Args {
    command: String,
    /// The game.
    game: String,
    /// The content root whose definitions `check` loads.
    content: PathBuf,
}

const USAGE: &str = "usage:
  nettai-content check <game> [--content DIR]         define the game's content (default: $NETTAI_CONTENT,
                                                   else a content/ beside the program, else this
                                                   repository's) and check its definitions, strings,
                                                   library order and look (its assets against it:
                                                   nettai-tool --match FILE --audit-content)
";

fn parse() -> Result<Args, String> {
    let usage = || format!("{USAGE}{}\n{}", publish::USAGE, publish::LOCK_USAGE);
    let mut it = std::env::args().skip(1);
    let command = it.next().ok_or_else(usage)?;
    let game = it.next().ok_or_else(usage)?;
    let mut a = Args { command, game, content: index::content() };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--content" => a.content = it.next().ok_or("--content needs a directory")?.into(),
            f => return Err(format!("unknown argument {f}\n{}", usage())),
        }
    }
    Ok(a)
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

fn print_report(r: &Report, verbose: bool) {
    for i in &r.issues {
        if verbose || i.level != Level::Note {
            eprintln!("{i}");
        }
    }
    let notes = r.count(Level::Note);
    eprintln!("{} errors, {} warnings, {notes} notes", r.count(Level::Error), r.count(Level::Warning));
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("publish") {
        publish::run(std::env::args().skip(2)).unwrap_or_else(|e| fail(e));
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("lock") {
        publish::lock(std::env::args().skip(2)).unwrap_or_else(|e| fail(e));
        return;
    }
    let a = parse().unwrap_or_else(|e| fail(e));
    match a.command.as_str() {
        "check" => {
            let mut r = Report::default();
            let game = &a.game;
            // The define phase: every module of the game loads, and what the
            // modules define fits together.
            let t = std::time::Instant::now();
            match nettai_content::load::load_battle(&a.content, game) {
                Ok((c, loaded)) => {
                    r.issues.extend(loaded.issues);
                    r.note(
                        a.content.display().to_string(),
                        format!(
                            "{} modules define {} definitions ({:.1?})",
                            c.scripts.modules.len(),
                            c.defs.definitions.defs.len(),
                            t.elapsed()
                        ),
                    );
                    nettai_content::lint::definitions(&c, &mut r);
                    // The strings tables, against the definitions.
                    nettai_content::locale::check_games(&a.content, &c, None, &mut r);
                    // The library order, against the definitions.
                    nettai_content::library::check_games(&a.content, &c, &mut r);
                    // The look the app draws the game's menus in.
                    nettai_content::look::check_games(&a.content, &c, &mut r);
                }
                Err(failed) => r.issues.extend(failed.issues),
            }
            print_report(&r, true);
            if r.has_errors() {
                std::process::exit(1);
            }
        }
        _ => fail(format!("{USAGE}{}\n{}", publish::USAGE, publish::LOCK_USAGE)),
    }
}
