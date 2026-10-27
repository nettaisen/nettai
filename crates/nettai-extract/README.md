# nettai-extract

A library that extracts a game's battle assets (its graphics, its sound and the index of their names) from the
player's Battle Network Legacy Collection ROMs, in memory, by the game's extraction recipe
(docs/design/extraction-recipes.md): Luau in the game's content repository (`<game>/extract/`, with the code the
games share in exelib's `extract/`), whose `extract/init.luau` returns the ROMs it reads (`roms`, by their header
codes) and the function that reads them (`extract`). The recipe reads the ROMs through the primitives here
(`src/recipe/`: ROM reads, LZ77, 4bpp tiles, palettes, map entries, sprite archives, GFX animations, the dialogue
font) and returns the asset model. The host here finishes it: the pictures' letters (`letters`), the element icons'
frame, the hand-drawn frames (each game's `extract/shapes/` in its content and exelib's, `shapes::Drawings`), the
sound bank and the songs each version has its own of (`SongVersions`), and the asset index. The crate names no
game: EXE4, EXE5 and EXE6 are their content's. It has no command of its own, and writes nothing: a frontend
extracts a game's assets each time it loads the game (nettai-frontend's `game`), and `nettai-content check <game>`
holds them to the game's content (docs/tools.md).

```rust,no_run
use nettai_extract::{RomSource, extract};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let roms = RomSource::from_env()?; // $NETTAI_ROMS, else the Legacy Collection where Steam installed it
let assets = extract(std::path::Path::new("content"), "exe6", &roms)?;

let graphics = &assets.graphics; // a nettai_assets::Bundle
let sound = &assets.sound; // an m4a::SoundBank
let index = &assets.index; // every asset by name, with its number in the two
# Ok(())
# }
```

## The ROMs

A game's ROMs are every one its recipe declares (a complete Legacy Collection's), in any order, each known by the
game code in its header (`header_code`, from its first `HEADER_LEN` bytes). The games' recipes declare these today:

| Game | Base US | Other US | Base Japanese | Other Japanese |
| --- | --- | --- | --- | --- |
| EXE4 | B4WE (Red Sun) | B4BE (Blue Moon) | B4WJ (Red Sun) | B4BJ (Blue Moon) |
| EXE5 | BRBE (ProtoMan) | BRKE (Colonel) | BRBJ (Blues) | BRKJ (Colonel) |
| EXE6 | BR6E (Falzar) | BR5E (Gregar) | BR6J (Falzar) | BR5J (Gregar) |

So the assets have both languages' words: English from the US ROMs, Japanese from the Japanese ones.

`RomSource` is where they are read from (`source.rs`, ported from Tango's `bnlc.rs`):

- `RomSource::Collection(Install)`: the Legacy Collection installed, its ROMs the `.srl` entries of the per-game
  `exe/data/exe*.dat` archives, found by their headers wherever in an archive they are. `Install::find` looks in a
  folder the player chose (the install's own, its `exe` or its `exe/data`), then through Steam (`steamlocate`, the
  volume's app id 1798020).
- `RomSource::Files`: ROM files, a folder's (any names: each file's header says which ROM it is).
- `RomSource::Copies`: the copies a browser keeps of a collection chosen once (nettai's `games`): each ROM's archive
  entry as it was, compressed, in a zip of its own named by its game code (`copy_of`, `RomSource::keep_copies`),
  stowed in the page's storage and read in as a game wants it.

`RomSource::from_env` is how the tools, the tests and development name them: `$NETTAI_ROMS`, a folder that is the
collection's install or holds its ROM files (`RomSource::at`), else the collection where Steam installed it; none
found is an error that says how to name them. `RomSource::find` matches headers against a recipe's codes, reading
each ROM only as far as its header; `RomSource::read` reads a recipe's ROMs whole into a `RomSet`, the game's alone
(another game's ROM or a duplicate is refused, a set short of one is refused before the recipe runs).

## The steps

`extract(content, game, &roms)` is `Recipe::of_content`, `RomSource::read` and `Recipe::extract` in one call:

- `Recipe::of_content(content, game)`: the game's recipe, read whole from a content directory (a bundle's, or the
  repository's content/) and held in memory; `Recipe::roms` the codes it declares.
- `Recipe::extract(&set)` gives an `Extraction`: the graphics, the sound, the songs each version has its own of
  (`sound_versions`), the asset index, and the warnings: what the passes noticed that isn't wrong enough to fail
  it (the letters, the element icons, the frames). `Extraction::unnamed` lists the sprites, songs and backgrounds
  the assets have under a number (`sprite-cc-ii`, `sound-nnn`) because compat names none of them.
- `Recipe::extract_reporting` is the same, telling a callback how far it is (0 to 1); `Recipe::extract_recording_reads`
  gives every range of each ROM the extraction read, with the recipe's file and line of each read (the
  verification's read map).
- `Extraction::timing_differences(names, sprites, numbers)` compares the content's sprite definitions with the
  ROMs' timing: each frame's duration, the loop on the last frame, the flags and the part offsets a sprite states.
- `songs_played_otherwise(a, b)`: the songs two releases' banks play otherwise (the verification's cross-ROM sweep).

An asset the names name that the recipe can't find is an error naming it; there are no placeholders. Extraction
needs no working-directory convention, prints no diagnostics and doesn't exit the host process; a recipe's error,
a primitive's (an address past the ROM), the codec's (a field the model doesn't have) or a ROM that can't be found
or read comes back as an `Error`.

## Tests

`cargo test -p nettai-extract` runs the synthetic tests (the primitives, the codec, the declarations, the sources
with made-up ROMs and archives, the refusals: CI has no ROMs). The ignored tests in `tests/extract.rs` run on the
original ROMs, `RomSource::from_env`'s, for the games `NETTAI_EXTRACT_GAMES` names (`exe6`, `exe5,exe6`; else every
game of the repository's content that has a recipe):

- `full_sets_pass_the_content_audit` loads each game with its assets extracted (as a frontend does), its timing
  the ROMs', and makes every lookup a frontend makes (`nettai-tool --audit-content`'s);
- `an_extraction_reports_how_far_it_is`: the progress a bar shows goes from 0 to 1 and never back.

```sh
NETTAI_ROMS=~/Documents/Tango/roms NETTAI_EXTRACT_GAMES=exe6 \
cargo test --release -p nettai-extract --test extract -- --ignored --nocapture
```

`cargo run -p nettai-extract --example shapes_check -- exe6 [OUT] [--recolor]` checks the hand-drawn frames against
the pictures extracted from the ROMs (`$NETTAI_ROMS`), and writes each picture beside its drawing in `OUT`.
