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

Any subset, including none, produces a pack, and a pack has every asset one of the ROMs
given holds: what the four ROMs have alike is read from whichever is there, a version's
own pictures from a ROM of that version, each release's words from a ROM of that release,
and the US ROM's wherever both releases have their own. Each block is stated by where the
game's own pointer to it is (a routine's literal, a load list's entry), so a ROM is read
the way its own code reaches the block. A placeholder is left only for what no ROM given
holds: unavailable graphics become checkerboards (black/magenta for assets with their own
palettes), and unavailable named songs become valid silent MIDI tracks.

A pack has the words of the releases its ROMs are of: English with a US ROM, Japanese with
a Japanese one, its own language the first of them (`hud.json`'s `language`, the other's
under `languages`). A pack of one release's ROMs has no lettering of the other's, rather
than placeholders for it; a frontend set to that language draws the pack's own pictures
with words, and the strings in the language chosen (docs/frontend.md, "Languages").

Placeholders preserve asset names and references but cannot reproduce missing animation
timing or the original appearance. They keep the game's shape where a frontend looks for
it: a banner is the kind its ROM record says (one with a number, one that holds until
removed, a telop's, the judge's) whatever its picture, and EXE4's HUD and screen are laid
out as its code lays them out (its telops, its own emblem, cursor colors and element
sprite).

### What is a placeholder, by the ROMs given

Counted 2026-10-12 from packs of each set of ROMs against the pack of all four and the pack of none
(bn6battle-verify's `tools/cross-rom/census.py`, which tells a placeholder by its picture). A cell is the
placeholders of a kind's assets; `-` is none. "Recolored" is a picture read from a ROM and drawn in the
placeholder's colors: the right shapes in magenta and black. A placeholder picture in a real palette counts as a
placeholder, and so does one with a placeholder part (EXE4's UNITE button without its label); a real picture in
another real palette (another version's, another release's, a sheet's palette rows past its archive's own) doesn't.
"Words in" is the languages a pack has words in: a pack of one release's ROMs has that release's alone, its
pictures with words compared with the full pack's in that language, and no placeholders for the other's (a
frontend set to it draws the pack's own). The HUD's navi box is "recolored" in a pack without the first link
navi's version only in its file, which shows it in that navi's colors: its tiles are real, and it is drawn in
each navi's own. `every_source_subset` holds each game's sixteen packs to the ROMs' own bytes: a placeholder
stands only where no ROM given holds the picture, and a pack states the languages its ROMs are of.

**EXE4**

| | both JP | Red Sun JP | Blue Moon JP | both US | Red Sun US | Blue Moon US |
|---|---|---|---|---|---|---|
| words in | Japanese | Japanese | Japanese | English | English | English |
| HUD pieces | - | - | - | - | - | - |
| custom screen pieces | - | - | - | - | - | - |
| banners | - | - | - | - | - | - |
| fonts | - | - | - | - | - | - |
| chip pictures | - | - | - | - | - | - |
| chip icons | - | - | - | - | - | - |
| sprites | - | - | - | - | - | - |
| portraits | - | - | - | - | - | - |
| backgrounds | - | - | - | - | - | - |
| field | - | - | - | - | - | - |
| mugshots | - | 6 of 22 | 6 of 22 | - | 6 of 22 | 6 of 22 |
| songs | - | - | - | - | - | - |

**EXE5**

| | both JP | Team of Blues JP | Team of Colonel JP | both US | Team ProtoMan US | Team Colonel US |
|---|---|---|---|---|---|---|
| words in | Japanese | Japanese | Japanese | English | English | English |
| HUD pieces | - | - | 1 recolored | - | - | 1 recolored |
| custom screen pieces | - | - | - | - | - | - |
| banners | - | - | - | - | - | - |
| fonts | - | - | - | - | - | - |
| chip pictures | - | 12 of 396 | 12 of 396 | - | 12 of 396 | 12 of 396 |
| chip icons | - | 6 of 396 | 6 of 396 | - | 6 of 396 | 6 of 396 |
| sprites | - | - | - | - | - | - |
| portraits | - | 4 of 13 | 5 of 13 | - | 4 of 13 | 5 of 13 |
| backgrounds | - | - | - | - | - | - |
| field | - | - | - | - | - | - |
| mugshots | - | 18 of 56 | 18 of 56 | - | 18 of 56 | 18 of 56 |
| emblems | - | 6 of 13 | 6 of 13 | - | 6 of 13 | 6 of 13 |
| songs | - | - | - | - | - | - |

**EXE6**

| | both JP | Falzar JP | Gregar JP | both US | Falzar US | Gregar US |
|---|---|---|---|---|---|---|
| words in | Japanese | Japanese | Japanese | English | English | English |
| HUD pieces | - | - | 1 recolored | - | - | 1 recolored |
| custom screen pieces | - | 3 of 25 | 3 of 25 | - | 3 of 25 | 3 of 25 |
| banners | - | - | - | - | - | - |
| fonts | - | - | - | - | - | - |
| chip pictures | - | 6 of 411 | 6 of 411 | 15 of 411 | 20 of 411 | 20 of 411 |
| chip icons | - | 5 of 411 | 6 of 411 | - | 5 of 411 | 6 of 411 |
| sprites | - | 1 of 298 | 1 of 298 | 6 of 298 | 7 of 298 | 7 of 298 |
| portraits | - | 5 of 12 | 4 of 12 | - | 5 of 12 | 4 of 12 |
| backgrounds | - | - | - | - | - | - |
| field | - | - | - | - | - | - |
| mugshots | - | 23 of 52 | 23 of 52 | - | 23 of 52 | 23 of 52 |
| emblems | - | 5 of 12 | 5 of 12 | - | 5 of 12 | 5 of 12 |
| songs | - | - | - | - | - | - |

What a player sees, and whether the ROMs they have hold what is missing.

**EXE6**, where nothing a ROM given holds is missing:

- **Only Japanese ROMs.** A whole battle in Japanese. The pack has no English words, which no Japanese ROM
  has (the 45 banners, the two fonts, the gauge's word, the notice of the other player customizing, the chip
  window's five worded pictures and the Crosses' names): set to English, its pictures with words are the
  Japanese ones, and the strings (chip names, descriptions) English in the frontend's font. Three sheets, an
  icon and a picture are the Japanese ROMs' own where the releases differ: HeatMan's flames, the Falzar
  feather and the flame, BassAnly's icon, and Gregar's Beast Out picture.
- **One version's ROM alone**, of either release. Everything but the other version's own: its five Giga chips'
  pictures and icons, its five link navis' portraits, faces and emblems, its Crosses' and its Beast's eighteen
  faces, its Beast Out button and picture, and its Cybeast's sprite. Its Crosses' names are the release's words
  of the version given. A Japanese ROM also lacks the other version's chip of the two the US release cut,
  Gregar or Falzar. A Gregar ROM has no icon for the Falzar chip (its record's is a placeholder that reads
  "13A"), and gives the BeastOut chip's picture as it has it, its own Beast's. And a Gregar ROM does have one
  of Falzar's navis' portraits, DustMan's: the archive is in it with no entry of the portraits' table pointing
  at it, so the extractor states where.
- **Only US ROMs.** The fifteen chip pictures and six sprites the US release cut. No Japanese words: set to
  Japanese, the pictures with words are the English ones and the strings Japanese.

**EXE5**, likewise:

- **Only Japanese ROMs.** A whole battle in Japanese. The pack has no English words (the 47 banners, the two
  fonts, the notice of the other player customizing, the chip window's four worded pictures and the soul
  button's label): set to English, the Japanese ones, the strings English. (The gauge and Arm Change's picture
  have no words in EXE5: any ROM gives them.) Yai's HP background, the battle mode 1 label and the hit's
  loudness are the Japanese ROMs' own, where the releases differ.
- **One version's ROM alone**, of either release. Everything but the other team's own: its six version chips'
  pictures and icons and its six navis' chips' pictures, four or five of its navis' portraits, its souls' and
  its navis' eighteen faces, its six emblems, and its soul icons' outline color. A Team Colonel ROM gives
  MegaMan's five dark faces in its own colors (four differ from Team ProtoMan's in two or three colors) and
  Colonel's and NumberMan's portraits as it has them.
- **Only US ROMs.** No Japanese words: set to Japanese, the English pictures with words, the strings Japanese.

**EXE4**, likewise:

- **Only Japanese ROMs.** A whole battle in Japanese. The pack has no English words (the 41 banners, the two
  fonts, "BUSY...", the chip window's three worded pictures and the UNITE button's label): set to English, the
  Japanese ones, the strings English. (The gauge with "CUSTOM", UNITE's picture and the banners' digits and
  palette are the same in every ROM.)
- **One version's ROM alone**, of either release. Everything but the other version's six souls' faces. (Every
  ROM has every chip's picture and icon. Four chips' icons aren't the same in a Blue Moon ROM as in a Red Sun
  one: a pack of Blue Moon ROMs has Blue Moon's.)
- **Only US ROMs.** No Japanese words: set to Japanese, the English pictures with words, the strings Japanese.
- In no ROM: the two e-Reader cards' chips' pictures, which the extractor has of its own.

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
