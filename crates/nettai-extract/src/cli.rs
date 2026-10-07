//! Command-line adapter; the extraction API itself has no process or I/O policy.
use crate::{Error, Game, RomSet, extract};
use std::{ffi::OsString, path::PathBuf, process::ExitCode};

const USAGE: &str = "usage: nettai-extract <exe4|exe5|exe6> <pack-dir> [ROM ...] [--content <dir>]\n\
ROMs may be supplied in any order. Missing ROMs generate placeholders.\n\
The output must be a new or empty directory. --content also checks battle definitions.";

struct Args {
    game: Game,
    output: PathBuf,
    roms: Vec<PathBuf>,
    content: Option<PathBuf>,
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
    let assets = extract(args.game, &roms)?;
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
