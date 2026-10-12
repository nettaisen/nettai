# nettai-extract

One Rust library and binary extracts EXE4, EXE5 and EXE6 battle assets. ROM identification,
LZ77, sprite/portrait archives, field and background formats, background animations,
banners, custom-screen patch lists, placeholders, and pack export/verification are shared.
The `exe4`, `exe5` and `exe6` modules retain game-specific addresses, layouts and source selection.
EXE4's has its sprites, chip pictures and icons, fonts and HUD text lines in both languages, the
field, the HUD with its banners, faces and chatbox, the custom screen with its Japanese words, and
sound; its `extraction.txt` lists the sprites and sounds without names yet.

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
timing or the original appearance. They keep the game's shape where a frontend looks for
it: a banner is the kind its ROM record says (one with a number, one that holds until
removed, a telop's, the judge's) whatever its picture, and EXE4's screen has its own
emblem, cursor colors and element sprite. An EXE4 US ROM that isn't Red Sun's gives its fonts,
text lines and banners; the HUD's other art is read at Red Sun US's addresses alone.

### What is a placeholder, by the ROMs given

Counted 2026-10-12 from packs of each set of ROMs against the pack of all four and the pack of none
(bn6battle-verify's `tools/cross-rom/census.py`, which tells a placeholder by its picture). A cell is the
placeholders of a kind's assets; `-` is none. "Recolored" is a picture read from a ROM and drawn with a palette
that is a placeholder: the right shapes in magenta and black. The Japanese banners that a full pack has as the
base's own words, and a sheet whose palette rows past its archive's own differ with the ROM, count as real.

**EXE4**

| | both JP | Red Sun JP | Blue Moon JP | both US | Red Sun US | Blue Moon US |
|---|---|---|---|---|---|---|
| HUD pieces | 12 of 12 | 12 of 12 | 12 of 12 | - | - | 10 of 12 |
| custom screen pieces | 19 of 21 | 19 of 21 | 19 of 21 | - | - | 19 of 21 |
| banners (the US words) | 41 of 41 | 41 of 41 | 41 of 41 | - | - | - |
| fonts (the US ones) | 2 of 2 | 2 of 2 | 2 of 2 | - | - | 1 recolored |
| Japanese lettering | 1 of 50, 1 recolored | 1 of 50, 1 recolored | 1 of 50, 1 recolored | 50 of 50 | 50 of 50 | 50 of 50 |
| chip pictures | - | - | - | - | - | - |
| chip icons | one gray off | one gray off | one gray off | - | - | one gray off |
| sprites, portraits | - | - | - | - | - | - |
| backgrounds | - | - | - | - | - | - |
| field | 1 of 1 | 1 of 1 | 1 of 1 | - | - | 1 of 1 |
| mugshots | 17 of 17 | 17 of 17 | 17 of 17 | - | 6 of 22 | 17 of 17 |
| songs | - | - | - | - | - | - |

**EXE5**

| | both JP | Team of Blues JP | Team of Colonel JP | both US | Team ProtoMan US | Team Colonel US |
|---|---|---|---|---|---|---|
| HUD pieces | 12 of 12 | 12 of 12 | 12 of 12 | - | - | 12 of 12 |
| custom screen pieces | 20 of 22 | 20 of 22 | 20 of 22 | - | - | 20 of 22 |
| banners (the US words) | 47 of 47 | 47 of 47 | 47 of 47 | - | - | 47 of 47 |
| fonts (the US ones) | 2 of 2 | 2 of 2 | 2 of 2 | - | - | 2 of 2 |
| Japanese lettering | 3 recolored | 3 recolored | 3 recolored | 58 of 58 | 58 of 58 | 58 of 58 |
| chip pictures | - | 12 of 396 | 12 of 396 | - | 12 of 396 | 12 of 396 |
| chip icons | 396 recolored | 6 of 396, 390 recolored | 6 of 396, 390 recolored | - | 6 of 396 | 6 of 396, 390 recolored |
| sprites | - | - | - | - | - | - |
| portraits | - | 4 of 13 | 5 of 13 | - | 4 of 13 | 5 of 13 |
| backgrounds | - | - | 22 of 22 | - | - | 22 of 22 |
| field | 1 of 1 | 1 of 1 | 1 of 1 | - | - | 1 of 1 |
| mugshots | 56 of 56 | 56 of 56 | 56 of 56 | - | 18 of 56 | 44 of 56 |
| emblems | 13 of 13 | 13 of 13 | 13 of 13 | - | 6 of 13 | 7 of 13 |
| songs | - | - | - | - | - | - |

**EXE6**

| | both JP | Falzar JP | Gregar JP | both US | Falzar US | Gregar US |
|---|---|---|---|---|---|---|
| HUD pieces | 12 of 12 | 12 of 12 | 12 of 12 | - | - | 12 of 12 |
| custom screen pieces | 23 of 25 | 24 of 25 | 24 of 25 | - | 3 of 25 | 22 of 25 |
| banners (the US words) | 45 of 45 | 45 of 45 | 45 of 45 | - | - | 45 of 45 |
| fonts (the US ones) | 2 of 2 | 2 of 2 | 2 of 2 | - | - | 2 of 2 |
| Japanese lettering | 4 recolored | 1 of 56, 3 recolored | 55 of 56 | 54 of 56 | 55 of 56 | 55 of 56 |
| chip pictures | - | 6 of 411 | 6 of 411 | 15 of 411 | 20 of 411 | 20 of 411 |
| chip icons | 411 recolored | 5 of 411, 406 recolored | 5 of 411, 405 recolored | - | 5 of 411 | 5 of 411, 405 recolored |
| sprites | - | - | - | 6 of 298 | 6 of 298 | 6 of 298 |
| portraits | - | 5 of 12 | 5 of 12 | - | 5 of 12 | 5 of 12 |
| backgrounds | 21 of 21 | 21 of 21 | 21 of 21 | - | - | 21 of 21 |
| field | 1 of 1 | 1 of 1 | 1 of 1 | - | - | 1 of 1 |
| mugshots | 52 of 52 | 52 of 52 | 52 of 52 | - | 23 of 52 | 29 of 52 |
| emblems | 12 of 12 | 12 of 12 | 12 of 12 | - | 5 of 12 | 7 of 12 |
| songs | - | - | - | - | - | - |

What a player sees, and whether the ROMs they have hold what is missing:

- **Only Japanese ROMs.** A battle is drawn with real navis, attacks and chip pictures, and sounds right, over
  checkerboards: the field's panels, the whole HUD (the HP boxes, the gauge, the digits, the emotion window and
  its faces, the chatbox), the custom screen (its window, slots, codes, cursor, buttons, emblems), and in EXE6 the
  backgrounds. The chips' icons and the Japanese words are their true shapes in magenta and black, as their
  palettes are the HUD's (EXE4's icons keep their colors but for one gray). **The Japanese ROMs hold all of
  it**: the same blocks, at other addresses (the verification's cross-ROM sweep found each one's bytes there).
  The extractor reads them at the base US ROM's addresses alone. Only the US words (the banners, the two US
  fonts) aren't in a Japanese ROM, and a Japanese player's language has its own.
- **Only the other version's US ROM** (Blue Moon, Team Colonel, Gregar). The same checkerboards, with EXE5's and
  EXE6's backgrounds and US words too. **The ROM holds all of it**; the extractor has the base version's
  addresses alone. EXE4 already reads Blue Moon US's fonts, text lines and banners (the font in the HUD's
  placeholder colors).
- **Only the base US ROM** (Red Sun, Team ProtoMan, Falzar). Complete but for what the ROM doesn't have: the other
  version's own art and the Japanese ROMs'.
  - EXE4: Blue Moon's six souls' faces.
  - EXE5: Team Colonel's six chips (pictures and icons) and its six navis' chips' pictures, four of its navis'
    portraits, eighteen faces and six emblems.
  - EXE6: Gregar's five Giga chips (pictures and icons), five link navis' portraits, 23 faces and five emblems, its
    Cross names and Beast Out button; and the fifteen chip pictures and six sprites only the Japanese ROMs have.
  - In all three, the Japanese lettering.
- **Both US ROMs.** Complete but for the Japanese lettering, and in EXE6 the Japanese ROMs' fifteen chip pictures
  and six sprites. (Where the releases' art differs, every pack with a US ROM has the US art: EXE5's goldfish
  and quieter hit, EXE6's HeatMan flames. A pack of Japanese ROMs alone has theirs.)
- **Three gaps of the same kind in the Japanese sets**, each an address the extractor lacks: EXE6's Japanese
  lettering is read from Falzar JP alone, so Gregar JP by itself has checkerboard words; EXE5's backgrounds are
  read from Team of Blues JP (or Team ProtoMan US) alone, so Team of Colonel JP by itself has none; EXE4 without
  Red Sun US takes the chips' icon palette by another path, one gray a step off.

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

Run `cargo test -p nettai-extract` for synthetic tests: among them, a pack of no ROMs of
each game passes the content audit (every lookup a frontend makes of everything the
content defines, `nettai-tool --audit-content`'s) and has the banner kinds the ROMs
state. The ignored `every_source_subset` test checks all 48 game/source combinations,
their graphics and sound round trips, battle-definition loading, the content audit and
the banners' kinds (about nine minutes with `--release`).
To run it locally, set `NETTAI_EXTRACT_ROM_DIR` to a directory of original ROMs named by
the codes above, for example `BR6E.gba`:

```sh
NETTAI_EXTRACT_ROM_DIR=/path/to/roms cargo test --release -p nettai-extract --test extract every_source_subset -- --ignored
```

The ignored `full_sets_preserve_reference_graphics` test also compares decoded graphics
against existing full packs. Set `NETTAI_EXTRACT_REFERENCE_PACKS` to their parent directory
alongside `NETTAI_EXTRACT_ROM_DIR` to run it.
