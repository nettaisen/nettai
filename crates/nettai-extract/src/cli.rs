//! Command-line adapter; the extraction API itself has no process or I/O policy.
use crate::{Error, Recipe, songs_played_otherwise};
use nettai_content_api::packs::{self, PackKind};
use std::{ffi::OsString, path::Path, path::PathBuf, process::ExitCode};

const USAGE: &str = "usage: nettai-extract <game> <pack-dir> ROM... [--reads <file>] [--check]\n\
<game> is a game's pack folder (content/exe6) or an unpacked bundle of one game: its\n\
recipe (extract/init.luau) is run on its ROMs, every one it states (a complete Legacy\n\
Collection's), given in any order. The output must be a new or empty directory.\n\
--check also checks the pack has what the content names, its sprites as the content animates them.\n\
--reads writes every range of the ROMs the extraction read, in order (tab-separated: the\n\
ROM's code, the address, the length, `data`, `open` (a span a parser took what it needed\n\
of) or the ROM address the word read holds, and where in the recipe the read was made):\n\
for comparing ROMs where it looks.\n\
\n\
       nettai-extract songs <ROM> <ROM>\n\
The numbers of the songs the two ROMs' sound banks play otherwise, or only one has.";

struct Args {
    game: PathBuf,
    output: PathBuf,
    roms: Vec<PathBuf>,
    reads: Option<PathBuf>,
    check: bool,
}

fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, Error> {
    let mut args = args.into_iter();
    let positional = |args: &mut dyn Iterator<Item = OsString>| args.next().filter(|a| !a.to_string_lossy().starts_with("--")).ok_or_else(|| Error(USAGE.into()));
    let game = positional(&mut args)?.into();
    let output = positional(&mut args)?.into();
    let mut result = Args { game, output, roms: Vec::new(), reads: None, check: false };
    while let Some(arg) = args.next() {
        if arg == "--reads" {
            if result.reads.is_some() {
                return Err(Error("--reads specified twice".into()));
            }
            result.reads = Some(args.next().filter(|v| !v.to_string_lossy().starts_with("--")).ok_or_else(|| Error("--reads needs a file".into()))?.into());
        } else if arg == "--check" {
            result.check = true;
        } else if arg.to_string_lossy().starts_with("--") {
            return Err(Error(format!("unknown option {}\n{USAGE}", arg.to_string_lossy())));
        } else {
            result.roms.push(arg.into());
        }
    }
    Ok(result)
}

/// The content directory and the game of `at`: a game pack's folder (its manifest in it), or a content directory
/// (an unpacked bundle) whose one game pack has a recipe.
fn game_of(at: &Path) -> Result<(PathBuf, String), Error> {
    let said = |e: &dyn std::fmt::Display| Error(format!("{}: {e}", at.display()));
    let at = std::fs::canonicalize(at).map_err(|e| said(&e))?;
    if at.join(packs::MANIFEST).is_file() {
        let (Some(content), Some(id)) = (at.parent(), at.file_name().and_then(|n| n.to_str())) else {
            return Err(said(&"no pack's folder"));
        };
        return Ok((content.to_path_buf(), id.to_string()));
    }
    let all = packs::packs(&at).map_err(|e| said(&e))?;
    let games: Vec<String> = all.into_iter().filter(|p| p.kind == PackKind::Game && packs::has_recipe(&at, &p.id)).map(|p| p.id).collect();
    match games.as_slice() {
        [game] => Ok((at, game.clone())),
        [] => Err(said(&"no game pack in it has an extraction recipe")),
        _ => Err(said(&format_args!("it holds the games {}: give one's folder ({})", games.join(", "), at.join(&games[0]).display()))),
    }
}

fn execute(args: Args) -> Result<(), Error> {
    let (content, game) = game_of(&args.game)?;
    let recipe = Recipe::of_content(&content, &game)?;
    let mut roms = recipe.rom_set();
    for path in &args.roms {
        let bytes = std::fs::read(path).map_err(|e| Error(format!("{}: {e}", path.display())))?;
        roms.insert(bytes).map_err(|e| Error(format!("{}: {e}", path.display())))?;
    }
    let assets = match &args.reads {
        Some(path) => {
            let (assets, reads) = recipe.extract_recording_reads(&roms)?;
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
        None => recipe.extract(&roms)?,
    };
    for warning in &assets.warnings {
        eprintln!("{warning}");
    }
    assets.write(&args.output)?;
    if args.check {
        let (loaded, _) = nettai_content::pack::load_battle(&content, &game).map_err(|r| Error(format!("the content does not load:\n{r}")))?;
        nettai_content::pack::pack_assets(&loaded, &args.output).map_err(|r| Error(format!("the pack can't draw the content:\n{r}")))?;
    }
    eprintln!(
        "wrote {}: {} sprites, {} songs (see extraction.txt)",
        args.output.display(),
        assets.graphics.sprites.len(),
        assets.sound.songs.iter().flatten().count()
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
    fn the_game_the_pack_and_the_roms_in_any_order() {
        let a = args(&["content/exe6", "pack", "gregar.gba", "--reads", "reads.tsv", "falzar.gba", "--check"]).unwrap();
        assert_eq!((a.game, a.output), (PathBuf::from("content/exe6"), PathBuf::from("pack")));
        assert_eq!(a.roms, [PathBuf::from("gregar.gba"), PathBuf::from("falzar.gba")]);
        assert_eq!((a.reads, a.check), (Some(PathBuf::from("reads.tsv")), true));
    }

    #[test]
    fn invalid_arguments() {
        for a in [vec![], vec!["content/exe6"], vec!["content/exe6", "--reads"], vec!["content/exe6", "pack", "--reads"], vec!["content/exe6", "pack", "--unknown"]] {
            assert!(args(&a).is_err(), "{a:?}");
        }
    }

    /// The game is a pack's folder's, or a content directory's one game with a recipe; a content directory of
    /// several is refused, naming them.
    #[test]
    fn the_game_is_its_folders() {
        let dir = tempfile::tempdir().unwrap();
        let pack = |id: &str, text: &str, recipe: bool| {
            std::fs::create_dir_all(dir.path().join(id).join(packs::EXTRACT)).unwrap();
            std::fs::write(dir.path().join(id).join(packs::MANIFEST), text).unwrap();
            if recipe {
                std::fs::write(dir.path().join(id).join(packs::EXTRACT).join("init.luau"), "return { roms = { \"BR6E\" } }\n").unwrap();
            }
        };
        pack("lib", "id = \"lib\"\nkind = \"support\"\n", false);
        pack("g", "id = \"g\"\nkind = \"game\"\ndepends = [\"lib\"]\n", true);
        pack("f", "id = \"f\"\nkind = \"game\"\n", false);
        let content = std::fs::canonicalize(dir.path()).unwrap();
        assert_eq!(game_of(&dir.path().join("g")).unwrap(), (content.clone(), "g".to_string()));
        assert_eq!(game_of(dir.path()).unwrap(), (content.clone(), "g".to_string()), "f has no recipe");
        pack("h", "id = \"h\"\nkind = \"game\"\n", true);
        let e = game_of(dir.path()).unwrap_err().to_string();
        assert!(e.contains("the games g, h"), "{e}");
        assert!(game_of(&dir.path().join("missing")).is_err());
    }
}
