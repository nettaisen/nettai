# nettai-extract

One Rust library and binary extracts EXE4, EXE5 and EXE6 battle assets from the player's Battle Network Legacy
Collection ROMs into a pack. A game whose content declares an extraction recipe (its manifest's `[extract]`,
docs/design/extraction-recipes.md) is extracted by that recipe, Luau in the game's content repository
(`<game>/extract/`, with the code the games share in exelib's `extract/`), which reads the ROMs through the
primitives here (`src/recipe/`: ROM reads, LZ77, 4bpp tiles, palettes, map entries, sprite archives, GFX
animations, the dialogue font) and returns the asset model: EXE6's and EXE5's now. EXE4, whose recipe isn't written
yet, is still extracted by the Rust module `exe4`, which its recipe will replace. Both paths share what makes the
pack: the check that every named asset is there, the pictures' letters, the element icons' frame, the hand-drawn
frames (each game's `extract/shapes/` in its content and exelib's, `shapes::Drawings`), the sound bank, the files
and their read-back check.

```sh
cargo run --release -p nettai-extract -- exe6 new-pack exe6f_rom_f_e.srl exe6_rom_e.srl exe6f_rom_f.srl exe6_rom.srl
```

The output goes before the ROM paths. Headers identify ROMs in any order; unsupported, truncated and duplicate
images are errors. The supported originals are 8 MiB, the four of each game a complete Legacy Collection holds:

| Game | Base US | Other US | Base Japanese | Other Japanese |
| --- | --- | --- | --- | --- |
| EXE4 | B4WE (Red Sun) | B4BE (Blue Moon) | B4WJ (Red Sun) | B4BJ (Blue Moon) |
| EXE5 | BRBE (ProtoMan) | BRKE (Colonel) | BRBJ (Blues) | BRKJ (Colonel) |
| EXE6 | BR6E (Falzar) | BR5E (Gregar) | BR6J (Falzar) | BR5J (Gregar) |

A recipe needs every ROM its game declares: a set short of one is refused before anything is read, and an asset
the names name that the recipe can't find is an error naming it. There are no placeholders on that path. (The Rust
path still makes a pack of fewer ROMs, with placeholders, until its game moves.) A pack has both languages' words:
English from the US ROMs, Japanese from the Japanese ones.

Every pack includes `extraction.txt`: what the passes found (the letters, the element icons, the frames) and the
sprites and songs written under a number (`sprite-cc-ii`, `sound-nnn`) because compat names none of them, which
`Extraction::unnamed` gives too. No ROM bytes are included in that report.

The CLI verifies that graphics, the asset index and sound structure read back. Optional `--content <directory>`
names the content the names, the recipe and the drawings are read from (the repository's content/ by default) and
also loads battle definitions against the pack. `--reads <file>` writes every range of the ROMs the extraction
read, with the recipe's file and line of each read. Output directories must be new or empty, so a later
extraction cannot keep stale files. Move the previous pack aside before regenerating it.

## Embedding

```rust,no_run
use nettai_extract::{extract, Game, RomSet, Tables};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let tables = Tables::of_content(Game::Exe6, std::path::Path::new("content"))?;
let mut roms = RomSet::default();
for rom in ["exe6f_rom_f_e.srl", "exe6_rom_e.srl", "exe6f_rom_f.srl", "exe6_rom.srl"] {
    roms.insert(std::fs::read(rom)?)?;
}
let assets = extract(&tables, &roms)?;

// Typed data for direct use:
let graphics = &assets.graphics;
let sound = &assets.sound;
let versions = &assets.sound_versions;

// Open-format files for an application's storage, with no filesystem writes:
let files = assets.files()?;

// Or write and verify a pack on disk:
assets.write(std::path::Path::new("new-pack"))?;
# Ok(())
# }
```

`RomSet` owns the supplied buffers and may hold several games'. `Tables` are what a game's content says of its
pack: its compat tables, its drawings and its recipe (`Recipe` runs one directly). Extraction needs no
working-directory convention, prints no diagnostics and doesn't exit the host process; a recipe's error, a
primitive's (an address past the ROM) or the codec's (a field the model doesn't have) comes back as an `Error`.
The caller chooses how to show diagnostics and where to store assets. `files()` leaves overwrite policy to the host;
`write()` provides the checked filesystem path.

Run `cargo test -p nettai-extract` for the synthetic tests (the primitives, the codec, the declaration, the
fingerprint and the refusals: CI has no ROMs). The ignored tests run on the original ROMs, found in a folder by
their headers (`NETTAI_EXTRACT_ROM_DIR`, any file names) and filtered by `NETTAI_EXTRACT_GAMES`:

- `tests/recipes.rs` extracts each game's full set by its recipe and by the Rust in one build, in memory, and
  compares the files byte for byte, the warnings and the ROM reads (as ranges per ROM), with each path's time; with
  `NETTAI_EXTRACT_BASELINE`, each path also against the packs the verification workspace recorded before the
  recipes (`data/extract-baseline/<commit>/`, its `tools/extract-baseline.sh`);
- `full_sets_pass_the_content_audit` loads each game's pack with its content and makes every lookup a frontend
  makes (`nettai-tool --audit-content`'s);
- `full_sets_preserve_reference_graphics` compares decoded graphics with existing full packs
  (`NETTAI_EXTRACT_REFERENCE_PACKS`).

```sh
NETTAI_EXTRACT_ROM_DIR=~/Documents/Tango/roms NETTAI_EXTRACT_GAMES=exe6 \
NETTAI_EXTRACT_BASELINE=../bn6battle-verify/data/extract-baseline/fac5b936a \
cargo test --release -p nettai-extract --test recipes -- --ignored --nocapture
```
