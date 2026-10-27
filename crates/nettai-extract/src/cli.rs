//! Command-line adapter; the extraction API itself has no process or I/O policy.
use crate::{Error, Game, RomSet, Tables, extract, extract_recording_reads, songs_played_otherwise};
use std::{ffi::OsString, path::PathBuf, process::ExitCode};

const USAGE: &str = "usage: nettai-extract <exe4|exe5|exe6> <pack-dir> [ROM ...] [--content <dir>] [--reads <file>]\n\
ROMs may be supplied in any order. A game whose content declares an extraction recipe\n\
needs every ROM it declares; the others make placeholders for the ROMs missing.\n\
The output must be a new or empty directory. The names the assets are written under, and\n\
the recipe, are the content's: --content's, else the repository's content/ ($NETTAI_CONTENT);\n\
--content also checks the battle definitions load with the pack.\n\
--reads writes every range of the ROMs the extraction read, in order (tab-separated: the\n\
ROM's code, the address, the length, and `data`, `open` (a span a parser took what it\n\
needed of) or the ROM address the word read holds; with a recipe, where in it the read\n\
was made): for comparing ROMs where it looks.\n\
\n\
       nettai-extract songs <ROM> <ROM>\n\
The numbers of the songs the two ROMs' sound banks play otherwise, or only one has.";

struct Args {
    game: Game,
    output: PathBuf,
    roms: Vec<PathBuf>,
    content: Option<PathBuf>,
    reads: Option<PathBuf>,
}
fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, Error> {
    let mut args = args.into_iter();
    let game = match args.next().as_deref().and_then(|a| a.to_str()) {
        Some("exe4") => Game::Exe4,
        Some("exe5") => Game::Exe5,
        Some("exe6") => Game::Exe6,
        _ => return Err(Error(USAGE.into())),
    };
    let output = args
        .next()
        .filter(|a| !a.to_string_lossy().starts_with("--"))
        .ok_or_else(|| Error(USAGE.into()))?
        .into();
    let mut result = Args {
        game,
        output,
        roms: Vec::new(),
        content: None,
        reads: None,
    };
    while let Some(arg) = args.next() {
        if arg == "--content" {
            if result.content.is_some() {
                return Err(Error("--content specified twice".into()));
            }
            result.content = Some(
                args.next()
                    .filter(|v| !v.to_string_lossy().starts_with("--"))
                    .ok_or_else(|| Error("--content needs a directory".into()))?
                    .into(),
            );
        } else if arg == "--reads" {
            if result.reads.is_some() {
                return Err(Error("--reads specified twice".into()));
            }
            result.reads = Some(
                args.next()
                    .filter(|v| !v.to_string_lossy().starts_with("--"))
                    .ok_or_else(|| Error("--reads needs a file".into()))?
                    .into(),
            );
        } else if arg.to_string_lossy().starts_with("--") {
            return Err(Error(format!(
                "unknown option {}\n{USAGE}",
                arg.to_string_lossy()
            )));
        } else {
            result.roms.push(arg.into());
        }
    }
    Ok(result)
}

fn execute(args: Args) -> Result<(), Error> {
    let content = args.content.clone().unwrap_or_else(nettai_content::index::content);
    let tables = Tables::of_content(args.game, &content)?;
    let mut roms = RomSet::default();
    for path in args.roms {
        let bytes = std::fs::read(&path).map_err(|e| Error(format!("{}: {e}", path.display())))?;
        // A typo in the selected game must not silently produce a placeholder pack.
        let code = bytes
            .get(0xAC..0xB0)
            .and_then(|b| std::str::from_utf8(b).ok());
        if code.is_some_and(|c| !args.game.codes().contains(&c)) {
            return Err(Error(format!(
                "{} is not a {} ROM ({code:?})",
                path.display(),
                args.game.id()
            )));
        }
        roms.insert(bytes)
            .map_err(|e| Error(format!("{}: {e}", path.display())))?;
    }
    let assets = match &args.reads {
        Some(path) => {
            let (assets, reads) = extract_recording_reads(&tables, &roms)?;
            let mut out = String::new();
            for r in &reads {
                let kind = match (r.pointer, r.open) {
                    (Some(p), _) => format!("{p:08X}"),
                    (None, true) => "open".into(),
                    (None, false) => "data".into(),
                };
                match &r.at {
                    Some(at) => out += &format!("{}\t{:08X}\t{:X}\t{kind}\t{at}\n", r.rom, r.address, r.len),
                    None => out += &format!("{}\t{:08X}\t{:X}\t{kind}\n", r.rom, r.address, r.len),
                }
            }
            std::fs::write(path, out).map_err(|e| Error(format!("{}: {e}", path.display())))?;
            assets
        }
        None => extract(&tables, &roms)?,
    };
    for warning in &assets.warnings {
        eprintln!("{warning}");
    }
    assets.write(&args.output)?;
    if let Some(content) = args.content {
        nettai_content::pack::load_battle(&content, &args.output)
            .map_err(|r| Error(format!("content does not load with the pack:\n{r}")))?;
    }
    eprintln!(
        "wrote {}: {} sprites, {} songs; {} missing ROMs, {} named placeholders (see extraction.txt)",
        args.output.display(),
        assets.graphics.sprites.len(),
        assets.sound.songs.iter().flatten().count(),
        assets.missing_roms.len(),
        assets.placeholders.len()
    );
    Ok(())
}

pub fn run() -> ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args.len() == 3 && args[0] == "songs" {
        let read = |p: &OsString| std::fs::read(p).map_err(|e| Error(format!("{}: {e}", PathBuf::from(p).display())));
        return match read(&args[1]).and_then(|a| Ok((a, read(&args[2])?))).and_then(|(a, b)| songs_played_otherwise(&a, &b)) {
            Ok(songs) => {
                println!("{}", songs.iter().map(|s| format!("{s:#05x}")).collect::<Vec<_>>().join(" "));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    match parse(args).and_then(execute) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(s: &[&str]) -> Result<Args, Error> {
        parse(s.iter().map(OsString::from))
    }
    #[test]
    fn partial_and_empty_sets() {
        assert!(args(&["exe5", "pack"]).unwrap().roms.is_empty());
        assert_eq!(args(&["exe4", "pack"]).unwrap().game, Game::Exe4);
        let a = args(&[
            "exe6",
            "pack",
            "gregar.gba",
            "--content",
            "defs",
            "falzar.gba",
        ])
        .unwrap();
        assert_eq!(
            a.roms,
            [PathBuf::from("gregar.gba"), PathBuf::from("falzar.gba")]
        );
        assert_eq!(a.content, Some(PathBuf::from("defs")));
    }
    #[test]
    fn invalid_arguments() {
        for a in [
            vec![],
            vec!["exe3", "pack"],
            vec!["exe5"],
            vec!["exe6", "pack", "--content"],
            vec!["exe6", "pack", "--unknown"],
        ] {
            assert!(args(&a).is_err());
        }
    }
}
