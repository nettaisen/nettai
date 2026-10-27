# nettai-extract

One Rust library and binary extracts a game's battle assets from the player's Battle Network Legacy Collection
ROMs into a pack, by the game's extraction recipe (docs/design/extraction-recipes.md): Luau in the game's content
repository (`<game>/extract/`, with the code the games share in exelib's `extract/`), declared in its manifest's
`[extract]`. The recipe reads the ROMs through the primitives here (`src/recipe/`: ROM reads, LZ77, 4bpp tiles,
palettes, map entries, sprite archives, GFX animations, the dialogue font) and returns the asset model. The host
makes the pack from it: the check that every named asset is there, the pictures' letters, the element icons' frame,
the hand-drawn frames (each game's `extract/shapes/` in its content and exelib's, `shapes::Drawings`), the sound
bank, the files and their read-back check. The crate names no game: EXE4, EXE5 and EXE6 are their content's.

```sh
cargo run --release -p nettai-extract -- content/exe6 new-pack exe6f_rom_f_e.srl exe6_rom_e.srl exe6f_rom_f.srl exe6_rom.srl
```

The first argument is the game's pack folder (or an unpacked bundle of one game); the output goes before the ROM
paths. Headers identify ROMs in any order. A recipe needs every ROM its game declares, each of the declared size
(a complete Legacy Collection's): another game's ROM, a truncated image or a duplicate is refused, a set short of
one is refused before the recipe runs, and an asset the names name that the recipe can't find is an error naming
it. There are no placeholders. The games' manifests declare these today, all 8 MiB:

| Game | Base US | Other US | Base Japanese | Other Japanese |
| --- | --- | --- | --- | --- |
| EXE4 | B4WE (Red Sun) | B4BE (Blue Moon) | B4WJ (Red Sun) | B4BJ (Blue Moon) |
| EXE5 | BRBE (ProtoMan) | BRKE (Colonel) | BRBJ (Blues) | BRKJ (Colonel) |
| EXE6 | BR6E (Falzar) | BR5E (Gregar) | BR6J (Falzar) | BR5J (Gregar) |

A pack has both languages' words: English from the US ROMs, Japanese from the Japanese ones.

Every pack includes `extraction.txt`: what the passes found (the letters, the element icons, the frames) and the
sprites and songs written under a number (`sprite-cc-ii`, `sound-nnn`) because compat names none of them, which
`Extraction::unnamed` gives too. No ROM bytes are included in that report.

The CLI verifies that graphics, the asset index and sound structure read back. `--check` also loads the battle
definitions of the content the game is in against the pack. `--reads <file>` writes every range of the ROMs the
extraction read, with the recipe's file and line of each read. Output directories must be new or empty, so a later
extraction cannot keep stale files. Move the previous pack aside before regenerating it.

A pack is named by its recipe's fingerprint (`Recipe::fingerprint`): a digest of the engine's line (its major and
minor version: the primitives change only with one), the pack formats and every file the recipe reads (its
`extract/` and its support packs', and the compat tables its `[extract]` names). The app keeps a pack under it.

## Embedding

```rust,no_run
use nettai_extract::Recipe;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let recipe = Recipe::of_content(std::path::Path::new("content"), "exe6")?;
let mut roms = recipe.rom_set();
for rom in ["exe6f_rom_f_e.srl", "exe6_rom_e.srl", "exe6f_rom_f.srl", "exe6_rom.srl"] {
    roms.insert(std::fs::read(rom)?)?;
}
let assets = recipe.extract(&roms)?;

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

A `Recipe` is read whole from a content directory (a bundle's, or the repository's content/): its inputs are held
in memory, so what it runs on is what its fingerprint digests. `RomSet` owns the supplied buffers, the game's
alone. A host that reads ROMs out of archives can match each one's header against `Recipe::roms` first
(`header_code`, from the first `HEADER_LEN` bytes). Extraction needs no working-directory convention, prints no
diagnostics and doesn't exit the host process; a recipe's error, a primitive's (an address past the ROM) or the
codec's (a field the model doesn't have) comes back as an `Error`. The caller chooses how to show diagnostics and
where to store assets. `files()` leaves overwrite policy to the host; `write()` provides the checked filesystem
path.

Run `cargo test -p nettai-extract` for the synthetic tests (the primitives, the codec, the declaration, the
fingerprint and the refusals: CI has no ROMs). The ignored tests run on the original ROMs, found in a folder by
their headers (`NETTAI_EXTRACT_ROM_DIR`, any file names) and filtered by `NETTAI_EXTRACT_GAMES`:

- `tests/recipes.rs` extracts each game's full set by its recipe, in memory, and compares its files, byte for
  byte, and its ROM reads (as ranges per ROM) with a baseline the verification workspace recorded
  (`NETTAI_EXTRACT_BASELINE`: its `data/extract-baseline/<commit>/`, made by its `tools/extract-baseline.sh`), with
  each game's time. A change meant to alter a pack records a new baseline;
- `full_sets_pass_the_content_audit` loads each game's pack with its content and makes every lookup a frontend
  makes (`nettai-tool --audit-content`'s);
- `full_sets_preserve_reference_graphics` compares decoded graphics with existing full packs
  (`NETTAI_EXTRACT_REFERENCE_PACKS`).

```sh
NETTAI_EXTRACT_ROM_DIR=~/Documents/Tango/roms NETTAI_EXTRACT_GAMES=exe6 \
NETTAI_EXTRACT_BASELINE=../bn6battle-verify/data/extract-baseline/fac5b936a \
cargo test --release -p nettai-extract --test recipes -- --ignored --nocapture
```
