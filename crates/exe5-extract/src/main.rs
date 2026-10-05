//! Extract EXE5's battle assets from the original ROMs, the US Team ProtoMan
//! (`MEGAMAN5_TP_`, BRBE), the US Team Colonel (`MEGAMAN5_TC_`, BRKE), the
//! Japanese Team of Blues (`ROCKEXE5_TOB`, BRBJ) and Team of Colonel
//! (`ROCKEXE5_TOC`, BRKJ), into a content pack:
//!
//!     exe5-extract content <protoman-us> <colonel-us> <protoman-jp> <colonel-jp> <pack-dir>
//!
//! The pack is EXE5's own (its manifest says `game = "exe5"`): a battle
//! loads one game's pack, so EXE5's assets are named for EXE5 alone
//! (docs/design/exe5-map.md §9). Most of it is the US Team ProtoMan ROM's.
//! A version chip's picture and icon are its own version's ROM's (the other
//! version's ROM has its counterpart's there), under the chip's key; a
//! song a version played otherwise would be in it twice, named `-protoman`
//! and `-colonel` (the two ROMs play every song alike). The Japanese ROMs
//! are checked against the US ones: their battle graphics differ only where
//! text is drawn, so the pack has none of theirs yet.
//!
//! The assets are named as EXE5's content names them (content/exe5/compat/
//! assets.toml, through exe5-compat: EXE6's names where the asset or its place
//! in the code is EXE6's; the chips' icons by chip key); the rest under their
//! placeholders (`sprite-0c-2d`, `sound-10e`), which content may not use.
//! The pack is the games' own data: write it outside version control.

mod content;
mod custom;
mod graphics;
mod hud;
mod rom;
mod sound;
mod sprite;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("content") => content::main(&args[1..]),
        _ => {
            eprintln!("{}", content::USAGE);
            std::process::exit(2);
        }
    }
}
