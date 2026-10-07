# nettai-extract

One Rust library and binary extracts EXE4, EXE5 and EXE6 battle assets. ROM identification,
LZ77, sprite/portrait archives, field and background formats, background animations,
banners, custom-screen patch lists, placeholders, and pack export/verification are shared.
The `exe4`, `exe5` and `exe6` modules retain game-specific addresses, layouts and source selection.
EXE4's is partial while its port is under way: its sprites, chip pictures and icons, fonts and
HUD text lines in both languages, and sound; the rest are placeholders its `extraction.txt` lists.

```sh
cargo run --release -p nettai-extract -- exe6 new-pack falzar.gba gregar-jp.gba
cargo run --release -p nettai-extract -- exe5 new-placeholder-pack
```

The output goes before the ROM paths. Headers identify ROMs in any order; unsupported,
truncated and duplicate images are errors. The supported originals are 8 MiB:

| Game | Base US | Other US | Base Japanese | Other Japanese |
| --- | --- | --- | --- | --- |
| EXE4 | B4WE (Red Sun) | B4BE (Blue Moon) | B4WJ (Red Sun) | B4BJ (Blue Moon) |
| EXE5 | BRBE (ProtoMan) | BRKE (Colonel) | BRBJ (Blues) | BRKJ (Colonel) |
| EXE6 | BR6E (Falzar) | BR5E (Gregar) | BR6J (Falzar) | BR5J (Gregar) |

Any subset, including none, produces a pack. Extraction selects available ROMs for
shared sprites and portraits, chip media, Japanese lettering and audio. Version-specific
pictures require their own version; unavailable graphics become checkerboards (black/magenta for assets with their own palettes),
and unavailable named songs become valid silent MIDI tracks. The base US ROM provides the
field, backgrounds and general screen layout/art; without it those sections use generated
placeholders even when another version contains similar graphics. EXE6's Japanese HUD
lettering uses Japanese Falzar, and Cross labels use their respective Japanese versions.
Placeholders preserve asset names and references but cannot reproduce missing animation
timing or the original appearance.

Every pack includes `extraction.txt`. The library result exposes `missing_roms`, `warnings`
and `placeholders` (named assets supplied by the completion pass; source-specific UI
placeholders are covered by the missing-ROM diagnostics), and `unnamed()`: the sprites and
songs written under a number (`sprite-cc-ii`, `sound-nnn`) because compat names none of them,
which `extraction.txt` lists too. No source ROM bytes are included
in that report. Supplying all sources retains the original asset formats and identities.

The CLI verifies that graphics, the asset index and sound structure read back. Optional
`--content <directory>` also loads battle definitions against the pack. Output directories
must be new or empty, so extracting a smaller set cannot retain stale files from an older
pack. Move the previous pack aside before regenerating it.

## Embedding

```rust,no_run
use nettai_extract::{extract, Game, RomSet};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut roms = RomSet::default();
roms.insert(std::fs::read("falzar.gba")?)?;
let assets = extract(Game::Exe6, &roms)?;

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

`RomSet` owns the supplied buffers and may hold both games. `extract` uses the game's
built-in compatibility names and needs neither a content directory nor a working-directory
convention. It does not print diagnostics or exit the host process. The ROM decoders assume
original layouts; with unwinding enabled, decoder assertions on corrupt/patched data are
caught and returned as errors (the host's panic hook still applies). The caller chooses how
to show diagnostics and where to store assets. `files()` leaves overwrite policy to the host;
`write()` provides the checked filesystem path.

Run `cargo test -p nettai-extract` for synthetic tests. The ignored `every_source_subset`
test checks all 32 game/source combinations, their graphics and sound round trips and
battle-definition loading. To run it locally, set `NETTAI_EXTRACT_ROM_DIR` to a directory
of original ROMs named by the codes above, for example `BR6E.gba`:

```sh
NETTAI_EXTRACT_ROM_DIR=/path/to/roms cargo test -p nettai-extract --test extract every_source_subset -- --ignored
```

The ignored `full_sets_preserve_reference_graphics` test also compares decoded graphics
against existing full packs. Set `NETTAI_EXTRACT_REFERENCE_PACKS` to their parent directory
alongside `NETTAI_EXTRACT_ROM_DIR` to run it.
