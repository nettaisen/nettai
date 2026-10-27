# Extraction recipes: each game says how its assets are extracted

The design for moving nettai-extract's per-game knowledge (where each asset is in each ROM, which ROM it is read
from, how the HUD and the custom screen are put together) out of the app's Rust and into Luau in each game's content
repo, beside the content that uses the assets. The app keeps the Rust that is the machine: ROM access, the GBA's and
the series' binary formats and the asset model (and, until 2026-10-27, the pack writer). Status: decided 2026-10-26
(§8); built: the baseline (step 0), the host (step 1), the recipes of EXE6 (step 2), EXE5 (step 3) and EXE4 (step
4), the removal of the Rust recipe path (step 5) and the verification workspace's tools (step 6); §9 says what
building changed. Since 2026-10-27 there are no packs: a game's assets are extracted from its ROMs by its recipe in
memory, each time the game loads, and nothing is written (§5; §9, "No packs"). What this document says of packs,
their fingerprint and their files is how it was built before that.

Decided (2026-10-26):

- the extraction recipes move into Luau in each game's repo (`nettaisen/exe4`, `exe5`, `exe6`), so content that needs
  a new asset says how to extract it, in the same bundle;
- the app keeps Rust primitives the recipes call;
- this is designed and built before any app release downloads content: it fixes what a bundle contains
  ([content-deployment.md](content-deployment.md) §6, step 4);
- assets are extracted from a complete Battle Network Legacy Collection only, every ROM of the game, on the desktop
  and in the browser; there are no placeholders (§8, decisions 5 and 7).

The rule it follows: Rust is the machine, Luau is the game, and Rust names no game feature
([rust-and-luau.md](rust-and-luau.md), [core-content-boundary.md](core-content-boundary.md)).

Related: [content-deployment.md](content-deployment.md) (bundles, the store),
[asset-formats.md](asset-formats.md) (the model the assets are extracted into), [content-model-v2.md](content-model-v2.md)
(packs, compat), [scripting.md](scripting.md) (the Luau runtime), [text-rendering.md](text-rendering.md) §12–13 (the
letters and the hand-drawn frames read as the assets are extracted), crates/nettai-extract/README.md (the extractor
as it is).

Words:

- a **recipe** is a game's extraction in Luau: `extract/` in the game's repo, with the shared part in exelib's
  `extract/` (§4);
- a **primitive** is a Rust function a recipe calls (§3);
- the **host** is nettai-extract after the change: it loads a bundle's recipe, gives it the ROMs and the primitives,
  takes back the asset model, and runs the fixed steps that finish the assets (§3.10; until 2026-10-27, that made a
  pack);
- the **model** is the assets in typed form: `nettai_assets::Bundle` (the graphics: sprites, field,
  backgrounds, HUD, custom screen), the sound bank with its versions, and the asset names (`AssetNames`);
- a game's **ROMs** are its four: two versions, US and Japanese, as a Legacy Collection install holds them. They are
  always all given (§8, decision 5).

## 0. Summary

- **A recipe returns the model; the host finishes it.** A recipe reads the ROMs through primitives and returns one
  table: the graphics in the model's shape, the asset names, and which ROMs its sound is read from. Everything after
  that is the host's and unchanged: the check that every named asset is there, the letters, the element icons'
  frame, the hand-drawn frames, the sound bank, the asset index (and, while there were packs, the files and the
  read-back check: `extract_inner` and `Extraction::files`, from `placeholders::complete` on). The pack's encoders
  didn't move, which is what made byte-identical output reachable.
- **The line.** Rust keeps the GBA's formats (ROM reads, LZ77, 4bpp tiles, 15-bit palettes, BG map entries, OBJ
  shapes, the M4A sound driver) and the series' bulk formats that decode straight into the model's own types (sprite
  archives, GFX animation lists), the per-pixel work, the model's codec and the writer. Luau gets every address, every
  count, which ROM each block comes from (versions, regions, cut chips), every layout constant, the assembly of the
  HUD and the custom screen, the record-sized formats (banners, patch lists, text lines, palette animation lists,
  background descriptors, cursor tables), the asset names, and the special cases (DustMan's orphaned portrait, the
  e-Reader cards' pictures, the gift chips' palettes).
- **A complete Legacy Collection, and nothing else.** A recipe declares its game's ROMs and they are all required: an
  install missing one is an error the player sees, on the desktop and in the browser alike, not a smaller pack. A
  recipe may assume every ROM is there. Today's handling of partial sets and its placeholders (about 1,300 lines of
  Rust) isn't ported; it is deleted as each part moves (§7). An asset a recipe can't find in the full set is an
  extraction error naming it.
- **Names stay data.** compat/assets.toml and the tables it is joined with stay where they are, written by gen-content
  and checked against the ROMs; the recipe reads them (§4.3) and builds `AssetNames` itself. The compat crates leave
  the extractor.
- **A recipe declares** its ROMs' header codes in the table its `extract/init.luau` returns, beside the function that
  reads them (§4.2); the host reads them without ROMs. The app finds a Legacy Collection's ROMs by the declared
  codes. The manifest says only what the pack is.
- **It runs in a VM of its own** (§4.4): nettai-luau's sandbox and bytecode check, with the extraction libraries in
  place of the battle's, one budget for the whole run, on the extraction thread, the same on the desktop and in the
  browser. Recipes are the one place Luau holds the original's addresses and numbers: they are the extractor, the
  boundary, and nothing in the battle content can require them nor they it (§2.4).
- **No fingerprint now.** While there were packs, a pack was named by a digest of the pack formats, the engine's line
  and the bytes of every file the recipe can read (§5.2), computed from the bundle without ROMs. Since 2026-10-27 a
  game's assets are extracted at each load by the recipe of the content it plays, so nothing names them and nothing
  is extracted again for a change: the next load runs the new recipe.
- **Primitives change only with a line** (a minor version), as the simulation does, and a bundle is published for one
  line: there is no separate primitives version (§5.3).
- **The content hash is untouched by the move.** It covers no recipe (§5.5), and since 2026-10-27 no pack either: the
  asset names and the sprites' timing are the content's own, so a bundle's hash is the bundle's alone.
- **Publishing reads no ROM**: the content hash is the content's alone (§5.5, §5.7).
- **Verification.** Both paths live in one build while a game moves: the old Rust recipe and the new Luau one extract
  the game's full set, and their files, `extraction.txt` and ROM reads must be identical (§6), with a baseline of the
  three games' full-set packs recorded before anything changes. Only then is the game's Rust deleted.
- **Migration**, one game at a time (§7): the host, the primitives and exelib's shared recipe code first; then EXE6,
  EXE5, EXE4; then `Game`, placeholders.rs, the compat crates' place in the extractor and the old fingerprint go, and
  each game is published for the line with its recipe. About 2,100 lines of Rust added (250 of them tests) and 5,900
  removed (450 of them tests); about 3,550 lines of Luau and 330 of declarations.

## 1. Extraction today

### 1.1 The pipeline

`nettai_extract::extract_reporting(game, roms, progress)` (crates/nettai-extract/src/lib.rs) runs `extract_inner`
under `catch_unwind`:

1. `asset_names(game)`: the names, from the game's compat tables (`exe*/names.rs`, through the exe*-compat crates'
   `include_str!`).
2. `exeN::graphics::bundle(&Roms, &names)`: the graphics, in the model's typed form. This is the per-game code.
3. `exeN::graphics::panel_names()`, then `placeholders::complete`: a checkerboard for every named sprite, background,
   mugshot, chip picture, chip icon and emblem no ROM given holds, and the field if none.
4. The languages: with US and Japanese ROMs both, as built; Japanese alone,
   `in_language("ja").without_language("en")`; no Japanese ROM, `without_language("ja")`.
5. `letters::read`, `elements::split`, `shapes::read`: the passes the enhanced graphics need (the
   pictures' letters, the element icons apart from their frame, the hand-drawn frames checked against the pictures).
6. The sound: `m4a::rom::extract` of the first ROM given in `Game::codes` order; for EXE5 with both US ROMs,
   `sound::colonel_songs` brings Team Colonel's own songs in as a version; `placeholders::songs` fills the songs no ROM
   gave; `SoundBank::validate`.

`Extraction::files` then writes the manifest, `export_graphics`, `export_sound_versions`, the asset index
(`AssetNames::index`) and `extraction.txt`; `write_reporting` writes them and reads them back. `fingerprint(game)`
digests `pack::FORMATS` and the names (FNV-1a, 64 bits).

Any subset of the four ROMs makes a pack today, the empty one included (the README's tables). With all four, nothing
is a placeholder: the full-set packs in the main checkout and in the verification workspace (data/exe4–6) have no
`placeholder:` line, no missing ROM and no silenced song in their `extraction.txt`.

### 1.2 Where the per-game knowledge is

| | EXE4 | EXE5 | EXE6 |
|---|---|---|---|
| Rust (`src/exeN/`) | 1,458 lines: rom, names, graphics, hud, custom, cards | 1,516: rom, names, graphics, hud, custom, lettering | 1,504: names, graphics, hud, custom, gregar, jp, lettering |
| Hand-drawn frames (`shapes/exeN/`: drawings and `.same` links) | 59 files | 73 | 87 |
| Compat tables read | assets, chips (`chip_keys`, `version`), panels, text (`first_control`, `jp`) | assets (`sprite_names`, `background_names`), chips (`chip_keys`), records (`navis`), panels, text | assets, chips (`id`), navis (`navi`), text |

Every block is stated by a `Place`: the address, in each of the four ROMs, of the game's own pointer to it (a
routine's literal, a load list's entry), or the block's own address where nothing points at it (`Place::stated`).
What the per-game modules hold, by kind:

- **Places and sizes**: some 270 `Place`s (155 pointers at one address a release, 111 at one of their own in each
  ROM, 2 stated), the byte counts beside them, record strides (`0x2C` a chip).
- **Source choice**: `Roms::any`, `us`, `jp`, `of(version)`, `preferring`, `us_of`, `jp_of`, and the tables that pick
  among them: EXE6's `gregar::RIGHT_IN_GREGAR`, `OWN_FACES`, `SPRITES`, `chip_source`, `icon_source`, `jp::SPRITES`,
  `CHIP_PICTURES`, `VERSIONED`; EXE5's `VERSION_CHIPS`, `OWN_CHIPS`; EXE4's `version_of` (compat's `version`).
- **Layout the frontend draws by**: `CustomLayout`, `CursorPlace`s, `WindowLabel`s, `HudLayout`, `TelopTiles`,
  `ButtonPlace`, `FAMILY_BYTES`, the chatbox's first tile.
- **Assembly**: `hud`, `drawn`, `worded`, `custom`, `version_pictures`, `row_button`, `soul_button`, the lettering's
  `hud` and `custom`: which block fills which field of `Hud` and `CustomScreen`, and the reorderings ("PAUSE" as five
  glyphs, the banner digits with the filler as glyph 10, EXE4's UNITE button with the window's fill tile six times).
- **Record formats**: `decode::banner_at` (EXE5's and EXE6's banners) and EXE4's `hud::banner`, `decode::patches`, the
  text lines (`hud::texts`, three copies), `decode::palette_anims` and EXE4's hand-listed `PANEL_PALETTE_ANIMS`,
  `background_picture_by` with its `BackgroundDescriptor`, EXE5's and EXE4's `cursor_corners`, EXE4's
  `window_emblem` steps.
- **Special cases**: EXE6's `DUSTMAN_PORTRAIT`, `GIFT_PALETTE`, the BeastOut chip's picture; EXE5's background
  numbers that must be one picture (an `assert!` against compat); EXE4's e-Reader cards (`cards.rs`: two pictures as
  hex, icons after chip 0x136's), its judge banner's kind, its Japanese chatbox text place.
- **Absent ROMs**: each block's `is_present` branch with its placeholder (55, 37 and 46 lines in EXE4, EXE5 and
  EXE6), `BANNER_RECORDS` (the banners' kinds for a HUD of placeholders, about 20 lines a game), the fallbacks of the
  source choice above, EXE4's placeholder font and its screen without a ROM: about 110 lines a game.

### 1.3 The shared part

`rom.rs` (ROM reads, LZ77, the read recorder, `Place`, `Source`, `Roms`), `decode.rs` (`speeding_scroll`,
`gfx_anims`, `tiles`, `banner_at`, `patches`, the field, palette animations, map entries, background pictures),
`sprite.rs` (archives, sheets, portraits), `sound.rs` (`colonel_songs`, `played_otherwise`), `placeholders.rs`
(401 lines), `elements.rs`, `shapes/mod.rs` with `build.rs` (the hand-drawn SVGs compiled in), `cli.rs`, `lib.rs`:
about 3,400 lines. Part of it is per-game in all but name (§2.3).

## 2. The split

### 2.1 Where the line runs

Three tests, in order:

1. **Does it name a game, a version, a region, an address or a count?** Then it is the recipe's. The `Game` enum, the
   header codes, `Version`, every `Place`, every table of which ROM a chip or a face comes from.
2. **Is it a format the GBA defines, or a format of the series that decodes straight into one of the model's own
   types?** Then it is a primitive, named for the format, with every address and count a parameter. 4bpp tiles,
   palettes, map entries, LZ77, the M4A driver; sprite archives (`SpriteSheet`), GFX animation lists (`GfxAnim`), the
   dialogue font's nibble rows (`DialogueFont`).
3. **Is the work per byte or per pixel, or per record?** Per pixel stays in Rust, however game-shaped (a tile's flips
   in a GFX animation frame). Per record goes to Luau, even when all three games share it: a banner record is a head
   word and twenty pointers, a patch list six bytes a patch, a text line a few dozen bytes; Luau reads them through
   `rom:u32` and passes the addresses to the bulk primitives.

What the frontend draws by stays Rust's: the model (`nettai_assets`), the pack formats (`nettai_content::pack`, until
the packs went on 2026-10-27), and the passes that read the model for the enhanced graphics (`letters::read`,
`elements::split`, the shape checks), which state of themselves that nothing in them is a game's (letters.rs:
"Nothing here is a game's: every look is the game's pictures'"). The recipe fills the model; it doesn't change it. A field the model lacks is a model change in
Rust first, as now.

### 2.2 Every module and function, and its fate

"Recipe" is the game's `extract/`; "exelib" is exelib's `extract/`, the code the three games share (§4.1); "primitive"
is §3; "host" is nettai-extract's fixed steps (§3.10); "deleted" is code only a partial set runs, which isn't ported
(§2.3).

**lib.rs**

| Item | Fate |
|---|---|
| `Game`, `ALL`, `named`, `id`, `codes` | gone: the game is the bundle's `game`; the codes are the recipe's ROM declarations (§4.2) |
| `HEADER_LEN`, `recognize` | host, game-neutral: a header's code is matched against the installed bundles' declarations |
| `fingerprint`, `asset_names` | host: the new fingerprint (§5.2); the names are the recipe's |
| `RomSet::insert` (known code, 8 MiB, no duplicate) | host: the code from the recipe's `roms`, no size (a ROM is told by its header's code); an incomplete set is refused (§4.2) |
| `Extraction` | host; `game` becomes the game's id; `missing_roms` and `placeholders` deleted |
| `extract_recording_reads`, `RomRead` | host: the ROM handles record their reads as `Rom` does now |
| `songs_played_otherwise` | host, for the verification's cross-ROM sweep; no recipe calls it |
| `extract_reporting`'s `catch_unwind` | host; a recipe's error becomes `Error` with the Luau traceback |
| `JAPANESE` and the language trimming | deleted: a full set's pack has both languages, as built |
| `extract_inner` | recipe through step 2 and the sound's ROMs (§1.1); host the rest, in the same order |
| `Extraction::unnamed`, `files`, `write`, `write_reporting` | host, unchanged |

**cli.rs**: host. `nettai-extract <content-dir> <pack-dir> ROM...`, the game from the content directory (a game
pack's folder, or an unpacked bundle's one game with a recipe), every declared ROM required; `--reads` and `songs`
unchanged.

**rom.rs**

| Item | Fate |
|---|---|
| `Rom::u8`, `u16`, `u32`, `bytes`, `from`, `contains`, `lz77`, `note` | primitive: the `Rom` handle (§3.2) |
| `record_reads`, `Read`, `ReadKind` | host |
| `Place::pointer`, `pointer_in_each`, `stated`, `Source::at` | exelib: `place.luau` |
| `Rom::is_present`, `Source`, `Roms` (`first`, `any`, `us`, `jp`, `of`, `preferring`, `us_of`, `jp_of`), `Version` | deleted: with every ROM there, each choice is one ROM, which the recipe names (§2.3) |

**decode.rs**

| Item | Fate |
|---|---|
| `speeding_scroll` | exelib: `scroll.luau` (§2.3) |
| `gfx_anims`, `gfx_anim` | primitive `gfx.anims` |
| `tiles` | primitive `gba.tiles`; its placeholder branch deleted |
| `banner_at` | exelib: `banners.luau`; EXE4's record (`exe4::hud::banner`) in EXE4's recipe |
| `patches` | exelib: `patches.luau` |
| `FieldAddresses`, `field`, `field_with`, `palette_anims` | exelib: `field.luau`, over `rom:lz77`, `gba.tiles`, `gba.palettes`, `gba.map` |
| `map_entries` | primitive `gba.map` |
| `BackgroundDescriptor`, `background_picture`, `background_picture_by` | exelib: `backgrounds.luau`, over the same primitives |

**sprite.rs**: `archive`, `sheet`, `portrait_sheet` are the primitives `sprite.archive`, `sprite.sheet`,
`sprite.portrait`; `part_list` and `part_size` go with them.

**sound.rs**: `colonel_songs` becomes the host's song versions, with the version names the recipe gives (§3.8);
`played_otherwise` stays as above.

**placeholders.rs**: deleted whole (§2.3). Of `complete`, the host keeps the two sorts it makes (each language's
buttons by name, the sprites by number); a named asset the model lacks becomes an error (§3.10).

**elements.rs** `split`: host. **shapes/mod.rs** (`Shaper`, `read`, `hud_shapes`, `custom_shapes`, `map_drawings`,
`Views`): host, reading the drawings from the bundle (§2.3). **build.rs**: gone. **examples/shapes_check.rs**: host
tool, given a content directory.

**exe6/**

| Module | Fate |
|---|---|
| `mod.rs` (`Version`, `name`) | recipe: its versions' names (`falzar`, `gregar`) as constants |
| `names.rs` | recipe: `names.luau` (§4.3) |
| `graphics.rs`: `bundle`, `sprites`, `portraits`, `field`, `panel_names`, `backgrounds`, `SCROLLERS` | recipe: `init.luau`, `sprites.luau`, `stage.luau`, over exelib's makers |
| `hud.rs`: every `Place`, `texts`, `palette`, `map`, `banner`, `mugshot`, `navi_mugshot`, `hud`, `drawn`, `worded`, `dialogue_font`, `chatbox`, `chip_icons` | recipe: `hud.luau`; `texts`, `chatbox`, `banner` through exelib; `BANNER_RECORDS` deleted |
| `custom.rs`: every `Place`, `LAYOUT`, the cursors, the button names, `version_pictures`, `row_button`, `emblems`, `picture`, `custom`, `labels`, `drawn`, `chip_art` | recipe: `custom.luau` |
| `gregar.rs` | recipe: `versions.luau` |
| `jp.rs` | recipe: `jp.luau` |
| `lettering.rs` | recipe: `lettering.luau` |

**exe5/**: the same shape. `rom.rs` (`Version`), `names.rs`, `graphics.rs` (with `VERSION_CHIPS`, `OWN_CHIPS`,
`chip_ids` and the background numbers' check), `hud.rs` (the faces with their boxes, the souls in Chaos Unison's
palettes, `glyph_name`), `custom.rs` (`FAMILY_BYTES`, `cursor_corners`, `soul_button`, Arm Change), `lettering.rs`:
all recipe, less each game's absent-ROM code.

**exe4/**: the same. `graphics.rs` (`version_of`, `panel_names`, `texts` with compat's `first_control`, the
hand-listed panel palette animations, the stated scrollers, its `DESCRIPTOR`), `hud.rs` (its banner record, the judge's
kind, the telops, `HudLayout`), `custom.rs` (`window_emblem`, the UNITE fill, `SHUFFLE_SETS`, `lettering`), `cards.rs`:
all recipe; `placeholder_font` and the screen without a ROM deleted.

### 2.3 What resists the split

- **Partial sets and placeholders.** Today's extractor makes a pack of any subset of the four ROMs, filling what the
  ROMs given don't hold with checkerboards (§1.2, "Absent ROMs"; placeholders.rs). A complete Legacy Collection is
  now required (§8, decision 5), and with all four ROMs today's packs have no placeholder (§1.1). So none of it is
  ported:
  - each `is_present` branch and each `placeholders::` call runs only without a ROM; `BANNER_RECORDS` serves only a
    HUD of placeholders; EXE4's `placeholder_font` and its screen without a ROM likewise;
  - every source choice is one ROM: `any()` and `us()` are the base version's US ROM, `jp()` its Japanese one,
    `of(v)`, `preferring(v)` and `us_of(v)` version `v`'s US ROM, `jp_of(v)` its Japanese one. The recipe names it
    (`set.BR5E`);
  - `placeholders::hud` and `placeholders::custom`, the skeletons every game's HUD and screen start from, hold EXE6's
    layout numbers (tile 0x1A0, the gauge at 0x222, palettes 13 and 9, a 136-tile window), but a full set's pack keeps
    none of their fields: each game's `drawn` and `worded` (EXE4's screen, built whole) set every one. The recipe
    starts from the model's defaults;
  - `placeholders::complete` adds nothing to a full set's pack, but makes two sorts the pack's bytes depend on (each
    language's buttons by name, the sprites by number): the host keeps them. A named asset the recipe doesn't give is
    an error naming it (`exe6: sprite/megaman is named in compat/assets.toml and no ROM gave it`), which the harness
    and the content audit catch (§6.4).

  What no ROM holds is no placeholder either: EXE4's two e-Reader chips' pictures are recipe data (below), and the
  gift chips' palettes are black, their definitions' `art_palette` coloring them, as now.
- **`speeding_scroll` reads a routine's code.** It matches a BG1 scroll callback's Thumb instructions and the
  battle-flag test (flag 0x40, BattleState+0x5C) of EXE5's 0x080019EC and EXE4's scroller +0xB8: that is the series'
  code, not a format. It moves to exelib as Luau over `rom:u16` (about 40 lines; it runs on fewer than thirty
  callbacks a game).
- **The banner record differs by game.** EXE5's and EXE6's (`banner_at`: a head word, twenty glyph pointers that
  repeat the filler, a number place) and EXE4's (a halfword place, a kind byte, twenty pointers with no filler; the
  judge's kind overridden to 4). A Rust primitive would have to know both. Both are Luau: exelib's maker for the
  first, EXE4's recipe for its own; the glyphs are `gba.tiles` reads.
- **The GFX animation list has an EXE4-only command** (0xC, the palette transform, `decode::gfx_anim`). It stays in
  the primitive: the command byte in the data selects it, not the game, and its frames are per-pixel work (a tile's
  flips).
- **The hand-drawn frames are game content compiled into the app**: 121 drawings and 98 `.same` links, 72 of them
  across games (EXE4's and EXE5's banners' bars to EXE6's `bar-NNN.svg`, EXE5's `mugshot-box-frame` to EXE4's). With
  one repo a game, a game's bundle holds its own pack and exelib only. The 33 drawings another game links to move to
  exelib's `extract/shapes/` (§8, decision 2); each game's own stay in its `extract/shapes/`; a `.same` file names a
  path in the bundle (`exelib/extract/shapes/banners/bar-104.svg`), and a moved drawing its own game names (all but
  the eleven bars, which only links reach) gets a link in its game's folder: 22 more, 120 in all. The host reads
  them from the bundle (`shapes::Drawings`, `Shaper::file`), not from `drawn::DRAWINGS`, on both paths: so every
  game's drawings move in step 1 (§9). `NETTAI_SHAPES_SOURCES` and `shapes_check` work as now, given a content
  directory (`--content`, the repository's content/ by default).
- **The e-Reader cards' pictures** (EXE4's `cards.rs`: Duo and PrixPowr, 0x540 bytes each as hex) are the one input
  besides the drawings that no ROM holds: embedded data. They move to EXE4's recipe as they are (`extract/cards.luau`,
  hex strings, decoded by `gba.tiles` over `bytes.hex`; §8, decision 6).
- **Asserts.** EXE5's background check (`assert!` that the numbers compat makes one background are one picture) and
  the lettering's banner-kind check (`assert_eq!` between a Japanese and a US banner) become `error()` in the recipe:
  extraction fails with the message, as it does now through `catch_unwind`.
- **The compat crates validate as they parse** (exe4-compat's text table: no more glyphs than the bytes below
  `first_control`). What the recipe reads raw, the recipe checks, with the same messages, at every extraction.
- **EXE5's song versions** (`colonel_songs`) name `protoman` and `colonel`. The merge itself is the machine's (voice
  groups matched or added); the recipe names the versions and which ROM is which.
- **EXE6's panel names are a Rust list** (`exe6::graphics::panel_names`), EXE5's and EXE4's compat's
  (`panel_name(n)`). All three become the recipe's (EXE6's list in its recipe, the others from panels.toml), for the
  field's panel types.
- **The order of the reads.** `--reads` (`extract_recording_reads`) lists the ROM ranges in the order the extraction
  read them, and the verification workspace's cross-ROM sweep consumes it. A recipe ported statement by statement
  reads in the same order (Rust evaluates a struct literal's fields as written, Luau a table constructor's likewise);
  the verification compares the reads as ranges per ROM after merging (§6.3), so an order the port can't keep
  doesn't fail it.

### 2.4 The original's numbers in Luau

content-model-v2.md §6.4 says no Luau API returns an original number, and core.d.luau's header that nothing is
named by one; compat stays TOML "which only the tools outside the engine read". Recipes are full of the original's
numbers: ROM addresses, chip numbers, sprite categories. They are the exception, for the reason compat is: the
extractor is the boundary. What keeps them there:

- **Their own VM** (§4.4), with the extraction libraries and none of the battle's (`asset`, `new`, `rules`,
  `battle`...). A recipe can't define content or reach a battle.
- **Their own tree.** A recipe module lives under `extract/` and requires only `extract/` modules (its own pack's and
  its support packs'); the runner refuses any other path. nettai-content-check refuses a content module that requires
  one under `extract/` (§4.5). The content's require walk never reaches them, so they are in no `Content` and no
  content hash, and never in the battle's VM.
- **What crosses** is the asset model and the names, the same as the Rust extractor hands over now: content still names
  assets by name (`asset.sprite("bomb")`), resolved through the pack's index.

content-model-v2.md §6.4 and core.d.luau's header gain the exception in step 1 (§7).

## 3. The primitives

### 3.1 Their shape

- **Global libraries and a function** in the recipe's VM (`gba`, `bytes`, `sprite`, `gfx`, `font`; `data`), installed
  as the battle's are (`lib_fn!` closures in a table, crates/nettai-luau/src/bind.rs).
- **Handles for bulk data**: `Rom`, `Bytes`, `Tiles`, `Map`, `Sheet`, `GfxAnim`, `DialogueFont` are userdata over
  Rust values (`impl UserData`, `enable_namecall`, as `Object` and `Vec3` are). Their bytes never enter Luau: an 8 MiB
  ROM is an `Arc<[u8]>` the handles share. A recipe moves handles around and puts them in the model; it reads single
  words through them.
- **Plain values for what is small**: a palette is an array of sixteen numbers, a map entry `{tile, hflip, vflip,
  palette}`, a layout a table of numbers.
- **Errors, not panics.** Every primitive checks its bounds and raises a Luau error naming the address
  (`Rom:u32: 0x08800004 is past the ROM`). Today a decoder's slice panics and `catch_unwind` turns it into an error on
  the desktop; the browser build is `panic=abort`, where it kills the page. A Luau error unwinds there too.
- **Numbers** are GBA addresses (0x08000000 on) and plain integers; a recipe computes with `bit32` and `//` as content
  does.
- **No placeholders.** No primitive makes a stand-in picture, palette, font or song (§2.3).

The declarations are content/nettai/extract.d.luau, beside core.d.luau, checked against by `extract/` modules only
(§4.5). The excerpts below are its substance.

### 3.2 ROMs

```luau
declare extern type Rom with
    code: string          -- "BR6E": as the recipe's `roms` states it (§4.2)
    slot: number          -- its place in `roms`, 1 to 4
    function u8(self, at: number): number
    function u16(self, at: number): number
    function u32(self, at: number): number
    function holds(self, at: number): boolean          -- a ROM address inside this ROM (`Rom::contains`)
    function bytes(self, at: number, len: number): Bytes
    function span(self, at: number, max: number): Bytes -- up to `max` bytes, an open read (`Rom::from`)
    function lz77(self, at: number): Bytes?             -- a BIOS LZ77 (0x10) block, nil if it isn't one
end
```

The recipe gets one argument, the set: each declared code to its ROM (`set.BR6E`), every one present (§4.2). Reads
are recorded as `Rom::note` records them now (a word that holds a ROM address is a pointer read; contiguous data reads
merge; `span` is open), for `--reads`; the recorder also notes the recipe's file and line of each read (mlua's stack
inspection, only while recording), a fifth column the cross-ROM sweep can use in place of parsing Rust constants
(§6.6).

`Place` is exelib's (`place.luau`): `place.pointer(us, jp)`, `place.pointer_in_each({...})`, `place.stated({...})`,
and `place.at(rom, p)`, which takes the address for the ROM's slot (a `pointer`'s first for slots 1 and 2, its second
for 3 and 4, as `Place::pointer` has them) and reads the pointer word there through `rom:u32`. They keep their Rust
spelling (`place.pointer(0x0801_ECC0, 0x0801_F0E4)`: Luau takes `_` in numbers), so the tools that read them change
their pattern, not their method.

### 3.3 Bytes and decompression

```luau
declare extern type Bytes with
    function len(self): number
    function u8(self, at: number): number      -- offsets from 0
    function u16(self, at: number): number
    function u32(self, at: number): number
    function sub(self, from: number, len: number): Bytes
end
declare bytes: { hex: (text: string) -> Bytes } -- EXE4's card pictures (§2.3)
```

`rom:lz77` is the only decompression the three games use (`Rom::lz77`, unchanged, bounds-checked as now).

### 3.4 Tiles, palettes, maps

```luau
declare extern type Tiles with
    function count(self): number
    function pick(self, tiles: {number}): Tiles      -- these tiles, in this order (from 0)
    function sub(self, first: number, count: number): Tiles
end
declare extern type Map with
    function count(self): number
    function entry(self, i: number): MapEntry         -- from 0
end
declare gba: {
    tiles: (from: Rom | Bytes, at: number, len: number) -> Tiles,      -- 4bpp, `len` bytes
    join: (parts: {Tiles}) -> Tiles,
    palette: (from: Rom | Bytes, at: number) -> Palette,               -- 16 colors as stored
    palettes: (from: Rom | Bytes, at: number, count: number) -> {Palette},
    map: (from: Rom | Bytes, at: number, count: number, first_tile: number?) -> Map,
}
export type Palette = {number}
export type MapEntry = { tile: number, hflip: boolean, vflip: boolean, palette: number }
```

- `gba.tiles` is `Tiles::from_4bpp`; `pick`, `sub` and `join` are what the assembly does with `Tiles::push` today
  ("PAUSE" as five glyphs: `pick({0, 4, 1, 5, 2, 6, 3, 7, 8, 9})`; the banner digits with the filler's two tiles
  joined after).
- `gba.palette` gives the colors as stored. Whether bit 15 is dropped is the recipe's (exelib's `colors.masked`):
  the games' code differs (EXE6's HUD reads palettes as stored and its custom screen drops bit 15; EXE5's HUD,
  custom screen and chip pictures drop it, as EXE4's HUD and custom screen do; the three games' field and backgrounds
  keep it, exelib's makers reading them as stored), and a pack keeps what the Rust kept.
- `gba.map` is `MapEntry::from_gba` over `count` halfwords; `first_tile` is taken off each entry's tile, wrapping in
  16 bits (the chatbox's maps, which count from BG0's tile 0x2E4 in EXE6).

### 3.5 Sprite archives

```luau
declare extern type Sheet with end   -- placed in the model as it is
declare sprite: {
    -- a sprite list's entry: with bit 31, LZ77 (its first word, the size, dropped); else an open read of 0x80000
    archive: (rom: Rom, entry: number) -> Bytes?,
    sheet: (archive: Bytes, category: number, index: number) -> Sheet?,
    portrait: (archive: Bytes, category: number, index: number) -> Sheet?,
}
```

`sprite::archive`, `sheet` and `portrait_sheet`, unchanged. The walk of the sprite list (the categories, where each
ends, which slots a version or a release fills) is exelib's and the recipe's. A sheet's frames' durations, flags and
parts reach the content hash (§5.5).

### 3.6 GFX animations

```luau
declare extern type GfxAnim with end
declare gfx: { anims: (rom: Rom, list: number, palette_buffer: number) -> {GfxAnim} }
```

`decode::gfx_anims`, unchanged: a background's animation list, palette frames, tile frames (each tile flipped as its
word says) and the palette transform (§2.3). The scroll callbacks' steps and `speeding_scroll` are exelib's.

### 3.7 Fonts and glyphs

```luau
declare extern type DialogueFont with end
declare font: {
    -- `count` glyphs of 0x60 bytes (16x12, the left pixel in each byte's low nibble) at `glyphs`, their
    -- advances at `advances` (`advance_bytes` apart: 1 in EXE6, 4 in EXE5 and EXE4, the low byte read),
    -- glyph k drawing chars[k]
    dialogue: (rom: Rom, glyphs: number, advances: number, count: number, advance_bytes: number,
        chars: {string}) -> DialogueFont,
}
```

The 8x16 font is `gba.tiles` (two tiles a glyph) with the recipe's characters beside it (`font_chars`). The HUD's
text lines are Luau (a table of halfword offsets, then each line's glyphs up to its end mark, below the first control
byte): the end mark and the limit are each game's (0xE6 and 0xE0 in EXE6 and EXE5, 0xE5 and compat's `first_control`
in EXE4).

### 3.8 Sound

A recipe doesn't read the sound bank; it says which ROM the host reads it from, and which ROMs are versions of it:

```luau
sound = { from = set.BRBE, base_version = "protoman", versions = { { name = "colonel", from = set.BRKE } } }
```

The host reads each with `m4a::rom::extract` (the driver is found by its code, `find_song_num_start`: the machine's),
brings each version's songs that play otherwise into the base bank (`sound::colonel_songs`, renamed
`song_versions`, the names the recipe's) and validates. A song that fails to read is an extraction error naming it
(no full set has one today); a song-table slot the game leaves empty stays empty (EXE6's stop-music). `songs` and
`played_otherwise` stay the CLI's and the cross-ROM sweep's.

### 3.9 Data the recipe reads

```luau
declare data: (path: string) -> any   -- a file of the game's compat/ (`compat/assets.toml`), TOML parsed to tables
```

### 3.10 What a recipe returns, and what the host does with it

The recipe (`extract/init.luau`) returns its table (§4.2), whose `extract` is a function from the set to one table:

```luau
local recipe: Recipe = {
    roms = { "BR6E", "BR5E", "BR6J", "BR5J" },
    extract = function(set: Set): Extracted
        local names = require("@self/names")()
        return {
            names = names, -- AssetNames: sprites, songs, backgrounds, mugshots, banners, chips, navis, glyphs
            graphics = {
                sprites = sprites(set, names),
                field = field(set.BR6E),
                backgrounds = backgrounds(set.BR6E),
                hud = hud(set, names),
                custom = custom(set, names),
            },
            sound = { from = set.BR6E },
        }
    end,
}

return recipe
```

**The codec** (nettai-extract's `recipe/model.rs`) reads the table into `nettai_assets::Bundle`, `AssetNames` and
the sound plan. The tables use the model's own field names: an unknown field, or a value of the wrong kind, is an
error naming its path (`graphics.hud.chatbox.boxes[2][1]: expected Map, got table`), as the rule sections'
`SpecReader` does; a field left out takes the model's default, as the Rust's `..Default::default()` does now. Lists
are arrays; a list with holes (`Vec<Option<Background>>`) is a table keyed by the game's number from 0 with its length
(`{ n = 22, [0x00] = ..., [0x05] = ... }`); an enum is its variant's name (`sets = "other"`); a list of named things
(`Vec<(String, T)>`: buttons, languages, a version's own) is `{ { name = ..., value = ... } }`, and `Versioned` is
`{ base_version, base, versions = { { name = ..., value = ... } } }`; a tuple is an array (a place `{ x, y }`, a face
`{ tiles, palette }`), but for the names' other languages' glyphs, `{ glyphs, dialogue_glyphs }` as `AssetNames`
names them; a `Map` handle stands for a list of map entries. Keyed maps go into `BTreeMap`s, so the order Luau iterates
them in never reaches a file. It is hand-written (the model has no serde and holds handles); about 800 lines.
`plain_data` doesn't fit: it takes no userdata and goes 16 deep.

**The host then runs the steps `extract_inner` runs after the per-game code, in the same order**, less the two that
only a partial set needs (the placeholders and the language trimming):

1. the check that takes `placeholders::complete`'s place: each language's buttons sorted by name and the sprites by
   number, as `complete` sorts them; every asset the names name present in the model (a sprite, a background, a
   mugshot, a chip's picture and icon, a navi's emblem, the field), else an error naming the first missing; progress
   0.6;
2. `letters::read`, `elements::split`, `shapes::read` with the drawings from the bundle (§2.3), each line prefixed as
   now;
3. the sound (§3.8); progress 0.95; `SoundBank::validate`; 1.0;
4. `Extraction`: the assets, in memory, with the warnings (while there were packs, `files`, `write_reporting` and the
   read-back wrote and checked one).

### 3.11 Performance

Before the recipes, a game's graphics took 0.02–0.04 s of an extraction's 1.1–1.8 s on the desktop (2–3 s in a
browser worker); encoding, writing and reading back a pack were the rest. (Since 2026-10-27 nothing is encoded or
written: a game's extraction by its recipe is about a tenth of a second, and reading its four ROMs out of the
archives about as long, in a release build; docs/app.md §17.) The recipe adds:

- **the VM and the modules**: some 1,000 lines a game plus 500 of exelib's, compiled once, a few milliseconds;
- **the calls**: a few tens of thousands a game (411 chip records read three words each, twice; the sprite lists'
  1,500 entries; 47 banners of 22 words in two languages; the text lines' bytes; the places' pointer words), at the
  50–180 ns a library call, method or handle costs (scripting.md §7): under 10 ms;
- **the codec**: about ten thousand table fields, a few milliseconds.

Everything per pixel or per byte stays in Rust: LZ77, the 4bpp unpacking, a tile's flips, a sprite archive's frames
and parts, the dialogue font's nibbles, the M4A bank, the letters, the encoders. The budget: recipe and codec under
0.1 s a game on the desktop and 0.3 s in the browser, under a tenth of an extraction; the verification harness times
both paths (§6.3). Over it, the fix is a coarser primitive, not a faster binding.

### 3.12 Primitives and the line

There is no primitives version. A primitive is added, or what one returns changes, or the codec takes a field it
didn't, only with a new minor version of the engine (a new line), and a bundle is published for one line, so every
app that runs a bundle's recipe has the primitives it was written for. (The fingerprint digested the line while there
were packs: §5.2.)

## 4. Recipes in a game's repo

### 4.1 Layout

```text
exe6/
  manifest.toml        what the pack is (id, kind, depends), nothing of the recipe
  compat/              as now; assets.toml, chips.toml, navis.toml, text.toml are read by the recipe
  extract/
    init.luau          the recipe: its ROMs (§4.2), and the set in, the model out (§3.10)
    names.luau         AssetNames from compat/ (§4.3)
    sprites.luau       the sprite list, the portraits, DustMan's (graphics.rs, gregar.rs, jp.rs)
    stage.luau         the field, the panel names, the backgrounds and their scrollers
    hud.luau           hud.rs
    custom.luau        custom.rs
    versions.luau      gregar.rs: what is a version's own
    jp.luau            jp.rs: what the US release cut
    lettering.luau     lettering.rs
    shapes/            EXE6's own drawings (banners/, hud/, custom/)
exelib/
  extract/
    place.luau  colors.luau  scroll.luau  banners.luau  patches.luau  texts.luau
    field.luau  backgrounds.luau  sprites.luau  chatbox.luau  buttons.luau  names.luau
    types.d.luau       the types the makers take (Place, FieldAddresses, ChatboxPlaces...)
    shapes/            the drawings two games or more share (§2.3)
exe4/extract/cards.luau  the e-Reader cards' pictures
```

One file a Rust module, so the port is read side by side. exelib's are makers that take a game's places and sizes
(the "shared maker" form content already uses): `chatbox.of(rom, CHATBOX)` where `CHATBOX` is the game's table of
places, tile count and first tile. A game that differs writes its own (EXE4's banner record); one that differs in a
count passes it (EXE4's chatbox, one kind of box, which its recipe shows for both kinds: §9, step 4).

### 4.2 The declaration

The table the recipe's `extract/init.luau` returns, `roms` beside the function (§3.10), and nothing else:

```luau
local recipe: Recipe = {
    -- Every ROM of a complete Legacy Collection, each required, in the order its places read an address by (a
    -- ROM's slot): Falzar's US ROM, Gregar's, then their Japanese ones.
    roms = { "BR6E", "BR5E", "BR6J", "BR5J" },
    extract = function(set: Set): Extracted
        -- ...
    end,
}

return recipe
```

- **`roms`**: the header codes (0xAC), four letters and digits each, none twice. Every one is required: the host
  refuses a set without one before the function runs, and the app tells the player the Legacy Collection's files are
  incomplete (§5.6). The app matches a header against every installed bundle's codes; `RomSet::insert` refuses a code
  the recipe doesn't state and a duplicate. A ROM is told by its code alone: there is no size, and an image cut
  short fails where the recipe reads past its end, naming the ROM. Two games may not declare one code (the app
  refuses the later bundle).
- **The order** is the one `Game::codes` had, which exelib's places read an address by (a ROM's `slot`, §3.2).
  Regions and versions aren't declared: a recipe names its ROMs by code (`set.BR5E` is the Gregar US ROM) and holds
  its versions' names (`falzar`, `gregar`) as the constants the pack writes them under.
- **Read without ROMs.** `Recipe::of_content` loads the recipe's modules in its VM (§4.4), with the primitives and
  `data` but no set, and reads the table (`model::declaration`): an unknown field, a code that isn't one, a code
  twice, `extract` that isn't a function are errors naming the field (`the recipe's table.roms[2]: BR6E twice`). So
  everything that needs the codes before a ROM is read has them: the app's Legacy Collection lookup
  (`bnlc::Install::roms`), `RomSet`'s checks, the CLI, the harness. Loading runs only the modules' top level, which
  defines functions and constants; the extraction loads them again in a VM of its own and calls `extract`.
- **What it reads**: with `data`, any file of the game's own `compat/`, by its path in the pack
  (`compat/assets.toml`). None needs naming (the fingerprint covered all of them while there were packs, §5.2); a
  path outside it is refused (and the content check refuses a literal one, §4.5).
- `PackManifest` (`deny_unknown_fields`) is the pack's `id`, `kind` and `depends` alone: a manifest with `[extract]`
  is refused. exelib's `extract/` is reached through the games' recipes.

### 4.3 The names

compat/assets.toml and the tables beside it stay TOML, and the recipe reads them (`names.luau`; §8, decision 1):

- they are written by gen-content (the verification workspace) and checked by it against the ROMs, and gen-content,
  the trace tests and the curation tools read them (they would read Luau badly);
- the content checker validates asset names against assets.toml without a ROM (content-model-v2.md §9.1);
- they are compat: the boundary's tables, where the original's numbers live.

What moves is the knowledge that is code: that a chip's icon is named by its key from chips.toml's `id` (EXE6) or
`chip_keys` (EXE4, EXE5), a navi's emblem by navis.toml's or records.toml's, that EXE5's backgrounds of several numbers
are the first's, that the Japanese ROMs' glyphs are text.toml's `[jp]`. And the checks the compat crates make as they
parse (no more glyphs than the bytes below `first_control`; one number under one name) move with it, with their
messages.

The two stay in step because they are in one repo: EXE5's and EXE4's mugshot numbers are the order the extractor
gives the faces (assets.toml says so, "by the number the extractor gives them"); a recipe that orders them otherwise
and an assets.toml that names them so are one commit of the game's repo.

The content-app branch's `Tables` (`Tables::of_content`, the compat crates' `of_content`) carry the names and the
recipes' compat reads at run time; recipes replace it, and with it the extractor's last use of the compat crates.
The crates stay for save import, traces and gen-content.

### 4.4 How it runs

- **A VM of its own**, made by a new public entry of nettai-luau for tool Luau that isn't battle content
  (`nettai_luau::tool::open(packs, entry, options, install)`): `sandbox::new_vm` (string, table, bit32 and the exact
  math), `sandbox::compiler`, `verify::check` (no global assignment, no writes to module-level locals), modules
  frozen as they load (`deep_freeze`), `require` resolved by `packs::find` and limited to `extract/` trees of the pack
  and its support packs (`@exelib/extract/place`), and the caller's libraries installed (`install`: nettai-extract's
  primitives). None of `asset`, `new`, `rules`, `battle`.
- **One budget for the run**, not a call's: the battle's million checks a call would stop a recipe that walks 411
  chips twice. A whole-run count large enough that only a runaway loop meets it ("the recipe ran past its budget"),
  so a broken recipe fails rather than hangs the extraction thread.
- **On the extraction thread.** `mlua::Lua` isn't `Send`: the VM is made where the extraction runs (the app's
  "extracting" thread; a Web Worker in the browser), as the define phase's is on the loading thread.
- **The same on the desktop and in the browser**: nettai-luau already builds Luau for wasm32 (WASI SDK, Wasm
  exceptions), and nettai-extract already links it through nettai-content.
- **Deterministic**: no clock, no randomness, no I/O but `data`; the codec sorts what Luau iterates.
- **Errors**: a recipe's `error`, a primitive's, or the codec's comes back as `nettai_extract::Error` with the Luau
  traceback (`exe6:extract/hud:212`). The app shows it as it shows an extraction's error now ("couldn't be extracted",
  the details in the log).

### 4.5 Checking

- **Types.** content/nettai/extract.d.luau declares the primitives; nettai-content-check checks `extract/` modules
  against it and exelib's `extract/` declarations, and the content's modules against core.d.luau and the packs'
  `types.d.luau`, each set apart (`packs::declarations` today gives every module every declaration, which would show
  the battle's API to recipes and the recipes' to content).
- **Lints**: a content module may not require under `extract/`, nor an `extract/` module outside it; a literal
  `data` path is a file of the game's `compat/`.
- **Runs need the ROMs.** A recipe runs only on its full set: in the verification harness (§6.3), in the full-set
  content audit (§6.4), in `nettai-content check <game>` and in the app. CI has no ROMs: there, the recipes are
  type-checked and the primitives, the codec and the declaration are unit-tested on synthetic data. (Today's no-ROM
  tests, `empty_sets_*`, extract placeholder packs, which no longer exist.) nettai-content-check couldn't run a recipe
  anyway: it can't link nettai-luau (the two Luau builds' symbols collide).

### 4.6 A recipe, in part

EXE6's HUD's `drawn`, as `hud.luau` would have it:

```luau
local place = require("@exelib/extract/place")
local chatbox = require("@exelib/extract/chatbox")
local at = place.at

local HUD_TILES = place.pointer(0x0801_ECC0, 0x0801_F0E4)
local TIMES_GLYPH = place.pointer(0x0801_ECD8, 0x0801_F0FC)
local TWO_GLYPH = place.pointer(0x0801_ECE4, 0x0801_F108)
local HUD_PALETTES = place.pointer(0x0801_DF80, 0x0801_E3A4)
local PAUSE = place.pointer(0x0801_E188, 0x0801_E5AC)
-- ...

-- `h` with what the four ROMs have alike, read from the Falzar US ROM.
local function drawn(rom, h)
    h.tiles = gba.join({
        gba.tiles(rom, at(rom, HUD_TILES), 0x640),
        gba.tiles(rom, at(rom, TIMES_GLYPH), 0x40),
        gba.tiles(rom, at(rom, TWO_GLYPH), 0x40),
    })
    h.first_tile = 0x1A0
    h.gauge_first_tile = 0x222
    h.hp_palettes = gba.palettes(rom, at(rom, HUD_PALETTES), 3)
    h.gauge_palette = gba.palette(rom, at(rom, HUD_PALETTES))
    -- ...
    -- "PAUSE" as five glyphs: the 32x16 sprite's columns (its tiles go row by row), then the 8x16 sprite.
    h.pause = gba.tiles(rom, at(rom, PAUSE), 0x140):pick({ 0, 4, 1, 5, 2, 6, 3, 7, 8, 9 })
    h.chatbox = chatbox.of(rom, CHATBOX)
    return h
end
```

## 5. The bundle and the assets

As built 2026-10-27, after the packs went (§9, "No packs"). Until then the app extracted a game's pack once, into
its data folder, named by the recipe's fingerprint, and extracted it again when the fingerprint changed; what follows
says what replaced each part.

### 5.1 What a bundle carries

Nothing new in its form: a bundle holds every file its packs' repos track (bundle.rs `pack_files`), so `extract/` and
`compat/` are in it already. What a game's bundle must have to be extracted is its recipe, `extract/init.luau`
and what it requires, and the `compat/` files it reads.

### 5.2 No fingerprint

A game's assets are extracted from its ROMs by the recipe of the content it plays each time the game loads, in
memory, and nothing of them is kept: there is nothing to name and nothing to extract again. A recipe changed in a
new bundle is run the next time its game loads. (The fingerprint, `Recipe::fingerprint`, was SHA-256's first 16 hex
digits of the pack formats, the engine's line, the game's id and every recipe input, path and bytes; it named the
pack's folder, `packs/<game>-<fingerprint>/`, and its stamp. It went with the packs, and so did `pack::FORMATS`.)

### 5.3 Primitives and the line

What a primitive does, and what the codec takes, is the line's: it changes only with a minor version, where the
simulation's changes too (§3.12). Bundles are already published for a line (content-deployment.md), and the app
picks its line's newest bundle and finds a replay's content by its content hash within the line, as before: no
index entry says more, and no app of a line meets a recipe its primitives can't run. What a primitive returns where
the content hash looked (a sprite's frames and parts, the asset index) was simulation surface for the same reason,
until the content took them over (§5.5).

### 5.4 When the app extracts

As each game loads (nettai-frontend's `game::Loading`, its `Assets` part, on the game's loading thread): its ROMs
read out of the collection's archives into memory, its recipe run on them, and what it extracted checked against the
content (`nettai_content::load::asset_numbers`), about two tenths of a second a game. Another game's backgrounds (the
background pool's) are extracted, by that game's own recipe, the first time a round or a picture wants one, on a
thread of their own, once a run (docs/frontend.md §1). An incomplete Legacy Collection extracts nothing: the player
sees why (§5.6). The packs folder an earlier app kept (`packs/` in the data folder) is deleted once, as the app
starts (docs/app.md §17).

### 5.5 The content hash and the assets

As built 2026-10-27: `Content::hash` covers no asset. The asset handles are the names the content's modules give and
each sprite's timing is its module's definition (content-model-v2.md §6.3), so the hash is the content's alone: a
battle, the training pool and a bundle's publishing read no ROM, and the assets are only what a frontend draws and
plays with, checked against the content as a game loads (`nettai_content::load::asset_numbers`: every asset the
content names, each sprite with the content's animations and frames). The content's timing is held to the ROMs' by
`nettai-content check <game>` (`Extraction::timing_differences`: each frame's duration, the loop on the last frame,
the flags and the part offsets a sprite states; each difference an error), and a frontend notes a difference as it
loads (`Game::timing_differences`). What follows is how it was before.

`Content::hash` covered the pack's asset index (`assets.toml`: names to numbers) and its sprites' animations (each
frame's duration and flags, the parts' places and sizes: `timing::load` of `animations.json` and `sprite.json`), and
through them what the define phase resolved. Not pixels, palettes, sound, icons, emblems, glyphs or `extraction.txt`;
not compat (tools' data, never loaded as content); not `extract/` (outside the require walk).

So, while the hash covered the pack:

- **A recipe moved the content hash only through the pack**: by naming an asset otherwise, or decoding a sprite's
  timing or parts otherwise. A bundle with its recipe and a byte-identical pack had the content hash the content
  before it had, and replays and the lobby saw no change.
- **The primitives that fed it were simulation surface**: `sprite.sheet`, `sprite.portrait` and the index's making.
  Their output changes only with a line (minor version), as the simulation's does (docs/design/rollback.md).
- **Every pack came from a full set**, so two apps of a line with the same bundle computed the same hash.

### 5.6 Finding the ROMs

nettai-extract's `source.rs` (the app's `bnlc.rs` until 2026-10-27, ported from Tango's) finds the Legacy Collection
(Steam's app 1798020, or a folder chosen), opens its `exe?*.dat` archives and reads each `.srl` entry's header:
`RomSource::Collection`. `RomSource::Files` is a folder of ROM files, and `RomSource::from_env` the tools' and
development's choice (`$NETTAI_ROMS`, else Steam's). It knows no game: `RomSource::find` matches each header's code
against a recipe's declarations (`Recipe::roms`), and `Rom::code` is a `String`. A game whose declared ROMs aren't all
there can't be played: the player sees that the Legacy Collection's files are incomplete (`UserError::
IncompleteCollection`, its words in the four locales), the log names the missing codes, and the dialog to choose the
folder again offers itself as for no Legacy Collection. A browser keeps copies of the ROMs of a folder chosen once
(`RomSource::Copies`: each ROM's archive entry as it was, compressed, in a zip of its own named by its game code,
stowed in the page's storage and read in as a game wants it; docs/app.md §6), so a later visit needs no folder.

### 5.7 Publishing

`nettai-content publish` hashes the bundle's content alone (step 4, content-deployment.md §4): no ROM is read and no
asset extracted. (Before 2026-10-27 it hashed with a pack, `--packs DIR`; the plan was to extract that pack from the
bundle's recipe and the publisher's collection, which the content's taking over the asset names and timing made
needless.) A recipe is held to its game's content by `nettai-content check <game>`, which extracts the game's assets
and checks them (docs/tools.md).

## 6. Verification

(This is how the move was verified while there were packs. Since 2026-10-27 the harness and the baseline's digests
are gone with the packs (§9, "No packs"); a recipe is held to its content by `nettai-content check <game>` and the
full-set content audit.)

### 6.1 The bar

For each game, the Luau recipe's pack of the full set is the Rust recipe's, byte for byte: the same paths, the same
bytes, `extraction.txt` included. And the ROM reads are the same ranges. A pack whose bytes are the same draws the
same, so no frame comparison is needed for the move; any later recipe change that draws something new is
frame-compared as any drawing change is. No partial set is compared: none is supported.

### 6.2 A baseline first

Nothing pins a pack today: the packs in the verification workspace (data/exe4–6) and the main checkout's data/ were
written by hand at different commits, and no test compares bytes across runs. Before step 1:

- `nettai-extract` at the commit step 1 starts from (with its content submodules' commits) extracts each game's full
  set: three packs;
- each pack's files' SHA-256, path by path, and its `--reads` log go to the verification workspace
  (`data/extract-baseline/<commit>/<game>.sha256`, `.reads`): digests and addresses, no ROM bytes;
- the three packs are kept whole too, for `diff -r` and the tools that read packs.

The baseline catches what the dual path can't: a change to the host both paths share (the drawings read from the
bundle instead of compiled in, the check in place of `placeholders::complete`).

### 6.3 Both paths in one build

While a game moves, nettai-extract has both: the Rust recipe behind `Game`, the Luau one through `Recipe`. An ignored
test (`tests/recipes.rs`) extracts the game's full set both ways, in memory (`Extraction::files`, no writes), and
compares:

- the file lists, then each file's bytes (the first difference reported with its path and, for a text file, its first
  differing line);
- the warnings (`extraction.txt` is a file);
- the reads (`extract_recording_reads`), as ranges per ROM after merging adjacent ones;
- the time each path takes (§3.11).

It takes a folder of ROMs and recognizes them by their headers (`NETTAI_EXTRACT_ROM_DIR`, any file names), so the
ROMs where the tools keep them (~/Documents/Tango/roms) serve as they are, and filters by game
(`NETTAI_EXTRACT_GAMES`). One game's run is a few seconds a path, so it runs at every step of a game's port.

### 6.4 The other checks, on the new path

- **A full-set content audit** (ignored, it needs the ROMs): each game's pack loads with its content and passes every
  lookup a frontend makes (`nettai-tool --audit-content`'s), as `empty_sets_pass_the_content_audit` does today for a
  pack of no ROMs. With no placeholders, an asset content names that the recipe didn't extract fails the extraction
  (§3.10), and one the names don't list fails the audit.
- `full_sets_preserve_reference_graphics`, and EXE4 added to it.
- `nettai-content publish --dry-run` for each game: the content hash is the one the same content has with today's
  pack.
- The fingerprint: the same bundle gives the same one on two runs and two machines; a byte changed in any input
  changes it; a change outside the inputs doesn't (unit tests over a synthetic bundle). The refusals: an unknown code,
  a truncated image, a duplicate, an incomplete set.
- **What goes**: `every_source_subset` (the sixteen sets a game, with its ROM-provenance census: `Held`, `pictures`,
  `unreached`), `empty_sets_make_loadable_packs_and_language_variants`, `an_empty_set_makes_an_exe4_pack`,
  `empty_sets_pass_the_content_audit`, `banner_kinds` (the kinds a HUD of placeholders had to keep), and `in_no_rom`
  (the e-Reader pictures are the recipe's data, read like any other block): about 450 lines of tests/extract.rs.

### 6.5 After the Rust is gone

The baseline stayed the reference while there were packs: the harness compared the recipe path with the recorded
digests, so a later refactor of a recipe, of exelib's code or of a primitive was held to the packs the Rust made until
a change meant to alter them recorded a new baseline (with the commit that did). The packs went on 2026-10-27, and
the harness with them.

### 6.6 The verification workspace

- `tools/cross-rom/sweep.py` and `words.py` read places from the extractor's Rust (`Place::pointer(...)` in
  `src/<game>/*.rs`): they read `extract/*.luau` (`place.pointer(...)`), or the `--reads` log's new column (§3.2). The
  sweep stays useful: it is how a recipe's author finds which ROM holds a block as the game shows it.
- `tools/cross-rom/census.py` and `checkerboards.py` classify placeholders in packs and frames of partial sets: they
  retire with them.
- `tools/exe4/gen_content.py`'s `check_cards` reads Duo's and PrixPowr's hex from `exe4/cards.rs`: it reads
  `content/exe4/extract/cards.luau`.
- `tools/shapes/redraw/build.sh` names `crates/nettai-extract/shapes`: the drawings are the packs' `extract/shapes/`.
- Comments that point at the Rust (`exe4/hud.rs`'s and `exe5/hud.rs`'s mugshot numbering in the two `gen_content.py`,
  `FAMILY_BYTES` in gen_rules.py and the generated elements.luau, tools/jp/README.md's `src/exe6/jp.rs`) point at the
  recipes.
- The briefs' pack command (`cargo run --release -p nettai-extract -- exe6 <dir> <ROMs>`) becomes
  `-- content/exe6 <dir> <ROMs>`, with all four ROMs.
- gen-content, the trace tests and the sound tests use the compat crates, which stay;
  `the_packs_asset_names_are_compats` holds the recipe's index to assets.toml as it holds the Rust's.

## 7. Migration

Each step builds and passes the quick checks (type-check, the touched tests, the content check); the harness runs at
each step of a game's port. The content-app branch lands first: recipes replace its `Tables`, and keep its pack
folders, its stamp and its store.

**Partial sets and placeholders go as they move.** None of today's code for them is ported. It goes in three places:

- in step 1, the tests built on partial sets (about 450 lines) and the README's tables of what each set holds (about
  120 lines of text);
- in steps 2 to 4, each game's absent-ROM branches, fallbacks and `BANNER_RECORDS` (about 110 lines a game), with
  the game's Rust;
- in step 5, placeholders.rs (401 lines, for a 40-line check), rom.rs's `Source`, `Roms` and `is_present` (86), and
  lib.rs's missing ROMs, no-ROM sound and language trimming (about 60).

About 1,300 lines of Rust in all.

| Step | What | Size |
|---|---|---|
| 0 | The baseline (§6.2) | a script in the verification workspace; three extractions. Done |
| 1 | The host: `nettai_luau::tool`, the primitives, the codec, the declaration (`PackManifest::extract`, every ROM required), `Recipe`, the fingerprint, the drawings read from a content directory, `tests/recipes.rs` and the full-set audit; the partial-set tests and the README's tables deleted; extract.d.luau; content-check's split declarations and lints; exelib's `extract/` (places, colors, scroll, banners, patches, texts, field, backgrounds, sprite list, chatbox, buttons, names) and the shared drawings; every game's drawings to its `extract/shapes/`, build.rs and the crate's `shapes/` deleted (§9); content-model-v2.md §6.4 and core.d.luau's header | Rust +2,000 (codec 800, primitives 450, runner 300, declaration and fingerprint 200, tests 250), −450 (tests); Luau +500 exelib, +330 declarations; 33 drawings to exelib, 98 links renamed, 22 added. Done |
| 2 | EXE6's recipe, its manifest's `[extract]`; the harness equal; then `src/exe6/` deleted | Luau +1,050; Rust −1,500. Done: Luau +1,180, Rust −1,470 |
| 3 | EXE5's, likewise | Luau +1,000; Rust −1,500. Done: Luau +1,340, Rust −1,640 |
| 4 | EXE4's, with `cards.luau` | Luau +1,000; Rust −1,450. Done: Luau +1,410 (84 lines of them the cards' hex), Rust −1,490 |
| 5 | The rest of the Rust recipe path: `Game` and `codes`, the compat dependencies, `asset_names`, the old fingerprint, rom.rs's source choice and `Place`, decode.rs's moved parts, placeholders.rs whole, lib.rs's partial-set handling; `bnlc` by declarations and its incomplete-collection error (+40, its words in four locales); the stamp without the app version; the catalog by primitives; the CLI by content directory; the README | Rust −1,000, +80. Done: Rust −1,660, +720 (the extractor −1,460, +440; the app +260, −170); no primitives version (§9) |
| 6 | The verification workspace (§6.6) | about 150 lines of Python and shell changed, two tools retired. Done: about 440 lines added and 640 removed in 30 files, census.py and checkerboards.py retired |
| 7 | Each game published for the line (below) | three releases |

**The compat crates** stay: the app's save import, the trace tools and gen-content use them. What only the extractor
used goes in step 5: `sprite_names`, `background_names`, the content-app branch's `of_content` and `builtin` feature
if nothing else needs them.

**Publishing.** Every 0.11 content release (EXE4's r1 and r2, EXE5's r1, EXE6's r1) has been deleted, tags too, and
the sites serve empty indexes. No released app has downloaded content, and none will until step 7. Step 7 publishes
r1 of each game for the line from the commits with the recipes; the app release that first downloads content ships
those (`release.yml`).

## 8. Decisions

The questions this design left open, with the answers (2026-10-26).

1. **The asset names stay TOML, read by the recipe** (§4.3). gen-content writes and checks them against the ROMs, the
   content check validates asset names by them, the tools read them, and they are compat, where the original's
   numbers are kept. The recipe holds the code that joins them.
2. **The code and the drawings the games share live in exelib's `extract/`** (§4.1): about 500 lines and 33 drawings
   the three games use alike (EXE4's and EXE5's banners' bars are EXE6's drawings), as makers that take a game's
   values. An exelib change re-extracts every game (its `extract/` is in each one's fingerprint).
3. **Primitives change only with a minor version; there is no separate primitives version** (§3.12, §5.3). A bundle
   is published for one line, and every app of the line runs its recipe; the fingerprint digests the line. (Decided
   first as a primitives version a patch release could raise; revised the same day, §9 step 5.)
4. **Publishing extracts the pack it hashes with** (§5.7), from the bundle's own recipe and the publisher's Legacy
   Collection, never a pack left in `data/<game>`. (Superseded 2026-10-27: the content hash is the content's alone,
   and publishing reads no ROM.)
5. **Only a complete Legacy Collection install is supported**, every ROM of the game (both versions, US and
   Japanese), on the desktop and in the browser alike. A recipe may assume its full set; every declared ROM is
   required; a missing one is an error the player sees ("the Legacy Collection's files are incomplete", §5.6), not a
   smaller pack. The partial-set machinery goes (§2.3, §7), verification compares each game's full set only (three
   packs, §6), and the content hash no longer depends on which ROMs were given (§5.5).
6. **EXE4's e-Reader card pictures are hex in `cards.luau`** (§2.3), as `cards.rs` has them, byte for byte. An indexed
   PNG would need a PNG-reading primitive for two pictures; it can follow as a content change.
7. **No placeholders at all.** With a complete Legacy Collection nothing needs one: the only assets no ROM holds are
   EXE4's two e-Reader pictures, which are embedded data. There are no placeholder primitives and no placeholder
   skeletons; placeholders.rs and the checkerboard are deleted (§7); an asset a recipe can't find in the full set is
   an extraction error naming it, which the harness and the content audit catch (§3.10, §6.4).
8. **The recipe declares its ROMs; the manifest says what the pack is** (§4.2; decided after step 6). Content is
   returned tables, not TOML declarations: `extract/init.luau` returns the ROMs' codes beside its function, and the
   manifest keeps `id`, `kind` and `depends` alone. `size` goes (a ROM is told by its header's code), and so does
   `reads` (the fingerprint covers the game's whole `compat/`). The manifest's `text` goes too: a top-level table of a
   game's locales that is no collection of its root is a text table.

## 9. As built

What building showed, step by step, where it differs from the sections above or says what they leave open.

**Step 0** (engine fac5b936a). The verification workspace's `tools/extract-baseline.sh <engine checkout> [ROM
folder] [game...]` builds the checkout's nettai-extract, finds each game's four ROMs in the folder by their headers,
extracts each full set, and writes `data/extract-baseline/<commit>/` in the workspace's main checkout (one baseline
for every worktree; ignored by git): each pack whole, `<game>.sha256` (each file's digest, path by path),
`<game>.reads` (the `--reads` log) and `commits` (the engine's and the content submodules'). It refuses a checkout
with changes. The baseline of fac5b936a: EXE4 3,076 files, EXE5 3,040, EXE6 3,413.

**Step 1.**

- *The drawings moved whole.* The host reads the drawings from the bundle on both paths (`Tables` carries a game's
  `shapes::Drawings` for the Rust one), as §6.2's baseline foresaw, so the crate's `shapes/` would have been dead
  the moment the host changed: every game's drawings went to its `extract/shapes/` in step 1 (EXE4's 31 and 28
  links, EXE5's 26 and 47, EXE6's 31 and 45, exelib's 33), and build.rs and the crate's `shapes/` were deleted
  then, not in steps 2 to 5. The harness showed all three games' Rust packs equal to the baseline after the move.
- *`Tables` until step 5.* The app's `Tables` became a struct: the game's compat (`Compat`, still the save import's),
  its drawings, and its `Recipe` when its manifest declares `[extract]`. `extract`, `extract_reporting` and
  `fingerprint` run the recipe for such a game, so the app extracts each game the way its content says with no
  change but the import's `match tables.compat()`; step 5 takes the compat crates out of it. `Extraction::game` is
  the game's id. `extract_in_rust` and `has_rust_extraction` (hidden) let the harness run the Rust path of a game
  that has a recipe.
- *The recipe's VM* is `nettai_luau::tool::open(modules, packs, entry, tree, options, install)`, its budget one
  count of 2,000,000,000 interrupt checks for the run; `nettai_luau` re-exports `mlua` for the primitives.
- *The reads.* `RomRead::at` (the recipe's `file:line`, from the Luau stack, only while recording) is `--reads`'s
  fifth column on the recipe path; the Rust path's lines keep four. One recording runs at a time.
- *The check* (§3.10, step 1) says `exe6: sprite/megaman is named in the asset names and no ROM gave it`, and
  `the recipe gave no field`.
- *The fingerprint* digests each part length-prefixed (the formats' `Debug` text, the engine's line since step 5,
  the game, the declaration's fields, then each input's path and bytes), hidden files left out of the inputs.
- *Declarations.* extract.d.luau declares the model's types too, every field optional (the codec defaults what is
  left out), so a recipe's misspelled field is a type error before it is a run-time one; exelib's makers' types
  are `exelib/extract/types.d.luau`. `packs::declarations` leaves out `nettai/extract.d.luau` and every `.d.luau`
  under an `extract/`; `packs::extract_declarations` is those.
- *Lints.* A content module requiring an `extract/` module and an `extract/` module requiring outside them are
  refused by the content check's reach pass; a recipe's literal `data("...")` must be in its manifest's `reads`,
  and a support pack's recipe code calls no `data` (the game passes it what it reads). The content lints
  (placeholder asset names, untyped table constants, owners' folders) don't apply to recipe modules.
- *exelib's names.luau* holds what the three games' names share: assets.toml's "cc-ii" sprites, the inversion of a
  table of name to number, the compat crates' one-number-one-name check with their messages, a key's own part.
- *The harness* is `tests/recipes.rs`'s `the_recipes_make_the_rust_extractions_packs` (ignored): each game's full
  set by its recipe and by the Rust, each path timed, and with `NETTAI_EXTRACT_BASELINE` (a baseline's folder)
  each path's files against the baseline's digests and its reads against the baseline's, as ranges per ROM; it
  says too whether the two paths' reads come in the same order. Its helpers (the ROMs by header from
  `NETTAI_EXTRACT_ROM_DIR`, the games of `NETTAI_EXTRACT_GAMES`) are `tests/common`.
- *Tests gone* (§6.4): `every_source_subset` with its census, `empty_sets_make_loadable_packs_and_language_variants`,
  `an_empty_set_makes_an_exe4_pack`, `empty_sets_pass_the_content_audit`, `banner_kinds`; the empty-set halves of
  `the_names_are_the_contents` and `rejects_unknown_truncated_and_duplicate_roms`. `an_extraction_reports_how_far_it_is`
  runs on a full set now (ignored), and `full_sets_pass_the_content_audit` (ignored) loads each game's full-set pack
  with its content, a battle's definitions and the audit's lookups.

**Step 2.** EXE6's recipe is content/exe6/extract/ (init, names, hud, custom, lettering, sprites, stage, versions,
jp: about 1,200 lines with the comments the Rust had), ported statement by statement. Its pack is the Rust's byte
for byte, `extraction.txt` included, and the baseline's (3,413 files); it reads the same ranges, and in the same
order but for the dialogue font's advances pointer, which `font.dialogue` reads before the glyphs where the Rust read
it after (twice: the English font and the Japanese). Where a ROM is chosen, the choice is the full set's (§2.3):
BR6E for what the four have alike and Falzar's own, BR5E for Gregar's own, BR6J for the Japanese words and what the
US release cut, BR5J for Gregar's Japanese form names and Gregar chip's picture. In a release build the recipe and
the codec take 0.04 to 0.06 s; the whole extraction in memory 0.11 to 0.12 s, the Rust's 0.06 to 0.07 s.

- `Tables::of_content` refuses a game this nettai extracts only by a recipe (`has_rust_extraction`: EXE6 now) whose
  content declares none; `Roms::us_of` and `jp_of`, which only EXE6's Rust used, went with src/exe6/.
- The reads log's fifth column is the innermost line of the game's own modules (where exelib's `place.at` reads a
  pointer for EXE6, the line of EXE6's that named the place), else the innermost line.
- The full-set content audit passes every asset lookup for EXE6's pack; it reports only the zh-Hans and zh-Hant
  lettering problems (a font without a character of a translated word, a picture's line count) that the Rust-made
  EXE4 and EXE5 packs report too, which are the content's and its fonts', not the extraction's. A battle's
  definitions load against the pack, and a headless match plays on it.

**Step 3.** EXE5's recipe is content/exe5/extract/ (init, names, versions, chips, hud, custom, lettering, sprites,
stage: about 1,340 lines with the comments the Rust had), ported statement by statement. graphics.rs became three
modules: the sprite list and the portraits (sprites), the field and the backgrounds (stage), the chip records
(chips); rom.rs's `Version` and graphics.rs's `VERSION_CHIPS` and `OWN_CHIPS` are versions. Its manifest declares
BRBE, BRKE, BRBJ and BRKJ, and reads assets.toml, chips.toml, panels.toml, records.toml and text.toml. Its pack is the
Rust's byte for byte, `extraction.txt` included, and the baseline's (3,040 files); it reads the same ranges, and in
the same order but for reads the Rust made twice (Team Colonel's soul icon palette, read for the comparison and again
for the list; the backgrounds' three table pointers, read again for the check of the numbers one background shares).
In a full set, BRBE holds what the four have alike, Team ProtoMan's own and the English words; BRKE Team Colonel's own
(its version chips' pictures and icons, its navis' own chips' pictures, its souls' faces, its navis' faces and
emblems, the soul icons' palette where it differs, its songs); BRBJ the Japanese words (Team of Colonel's are the
same). Nothing is read of BRKJ, as the Rust read nothing of it with BRBJ there; it is declared and required as every
ROM is (§8, decision 5). In a release build the recipe and the codec take 0.04 to 0.045 s; the whole extraction in
memory 0.09 to 0.1 s, the Rust's 0.067 to 0.07 s.

- exelib served as built: `chatbox.of` with `masked` (EXE5's chatbox drops bit 15), `field.read` (EXE5's panel
  palette animations are in EXE6's form), `backgrounds.picture` with `DIRECT`, and `scroll.speeding`, whose first
  user EXE5 is (NebulaGray's background, 0x1B). No primitive changed.
- EXE5's HUD and custom screen start from the model's defaults (§2.3): with every ROM there its `drawn` and `worded`
  set every field the placeholder skeletons did. An emotion-window face that brings no box (a soul's) has empty tiles
  (`gba.join({})`).
- The recipe checks what exe5-compat checks of the tables it reads, with its messages: one chip a number, a
  background's numbers (at least one, none another background's), a sprite's "cc-ii", the panel types by number;
  the background numbers' `assert!` is an `error()`.
- What went with src/exe5/: `sound::colonel_songs` (the host's `own_songs` under the recipe's version names does it),
  extract_inner's EXE5 sound branch, and of decode.rs `banner_at`, `field`, `palette_anims`, `background_picture`,
  `BackgroundDescriptor::DIRECT` and `FieldAddresses`' `palette_anims` and `palette_buffer`, which only EXE5 still
  used (EXE4's field gives its own palette animations, its backgrounds their own descriptor). `has_rust_extraction`
  is EXE4 alone.
- The full-set content audit passes every asset lookup for EXE5's pack and reports only the zh-Hans and zh-Hant
  lettering problems step 2 names (six: the chip window's pictures' words); the pack is the baseline's byte for byte,
  so they are the Rust-made pack's too. A battle's definitions load against the pack, and a headless match plays on
  it.
- A comment that still points at the Rust is in a generated file (compat/assets.toml's mugshot numbering, "its
  hud.rs"): it changes with its generator in the verification workspace's step (§6.6).

**Step 4.** EXE4's recipe is content/exe4/extract/ (init, names, versions, sprites, stage, hud, custom, lettering,
cards: about 1,400 lines with the comments the Rust had, 84 of them the e-Reader pictures' hex), ported statement by
statement. Its pack is the Rust's byte for byte, `extraction.txt` included, and the baseline's (3,076 files); it reads
the same ranges, fewer times and in another order where it reads less: the Rust read each chip's record and picture
twice (for the icons, then for the pictures) and the window map's pointer once an entry, and exelib's chatbox maker
reads the box's maps after its tiles and palette. Where a ROM is chosen, the choice is the full set's (§2.3): B4WE for
what the four have alike, the English words and Red Sun's own; B4BE for Blue Moon's own (its giga chips' pictures and
icons, its souls' faces); B4WJ for the Japanese words. In a release build the recipe and the codec take 0.03 to
0.04 s; the whole extraction in memory 0.07 to 0.08 s, the Rust's 0.04 to 0.05 s.

- *exelib unchanged.* EXE4's banner record (a halfword place, a kind byte the judge's banner doesn't read, twenty
  glyph pointers) is its hud.luau's. Its chatbox is exelib's maker with one kind (`kinds = 1`, its palettes masked),
  which the recipe gives for both: EXE4 shows a chip's description in the message box. The field and the backgrounds
  are exelib's makers, with EXE4's six hand-listed panel palette animations (`field.with`) and its descriptor (`{ gfx
  = 0, dest = 8, map = 0x0C, palette = 0x18 }`); `scroll.speeding` reads background 0x17's speeding scroll.
- *Bit 15.* EXE4's palettes drop it on the HUD, the custom screen and the panels' cycling palettes (its own list);
  the field's eight and the backgrounds' are read as stored by exelib's makers, as the Rust's shared decoders read
  them for EXE5 too. §3.4 said EXE5's and EXE4's drop it everywhere; it now says where (exelib's colors.luau still
  says everywhere).
- *The compat checks* keep exe4-compat's messages, in EXE4's names.luau and stage.luau (`chips.toml: <later> and
  <earlier> are both chip 0x13a`, the sounds' likewise; more glyphs than the bytes below `first_control`; panels.toml's
  types by number): exelib's `names.unique` says exe6-compat's (`<earlier> and <later> are both 0x13A`). A chip's
  `version` other than `redsun` or `bluemoon` is refused, as the crate's parse refused it.
- *Chips.* EXE4's pack has the pictures and icons of the chips compat names, as the Rust's did (EXE6's has all 411).
  The e-Reader cards' pictures are two long strings in cards.luau, a tile a line, under `bytes.hex`; their icons are
  read after chip 0x136's as before, and their records, which point at RAM, aren't read. The verification
  workspace's `check_cards` reads the pictures there (§6.6, done with this step).
- *Deleted:* src/exe4/ (1,458 lines, cards.rs among them) and `Place::stated`, which only EXE4's Rust still used;
  lib.rs's EXE4 arms are unreachable, and `Tables::of_content` refuses EXE4 content that declares no recipe, as it
  does EXE6's.
- *Checked:* the full-set content audit passes every asset lookup for EXE4's pack; a battle's definitions load
  against it and a headless match plays on it. The audit reports only the zh-Hans and zh-Hant lettering problems of
  step 2's note (five: in zh-Hans the Display font without a character of three pictures' words, and in both OK's
  picture with five lines of letters for four of words). EXE5's Rust and EXE6's recipe still make the baseline's
  packs.

**Step 5.** The Rust recipe path is gone; every game is extracted by its recipe alone, and each pack is the
baseline's byte for byte (the harness, now the recipes against the baseline only: §6.5).

- *Removed:* lib.rs's `Game` (`ALL`, `named`, `id`, `codes`), `Compat`, `Tables` (with `names`, `panel_names`,
  `read_by_number`), the old fingerprint, `extract_inner`, `extract_in_rust*`, `has_rust_extraction`, `JAPANESE`
  and the free `extract`, `extract_reporting`, `extract_recording_reads` and `fingerprint` (the `Recipe` methods
  are the API), `Extraction`'s `missing_roms` and `placeholders`; rom.rs's `Place`, `Source`, `Version`, `Roms`
  and `Rom::is_present`; decode.rs but the GFX animation lists, which are gfx.rs; placeholders.rs whole; the
  compat crates as the extractor's dependencies, and compat's `sprite_names` and `background_names`, which only
  the Rust extraction called. In all, Rust −1,660 and +720: the extractor −1,460 and +440 (its tests −160, +110),
  the app +260 and −170.
- *The API.* `Recipe` is the entry (`of_content`, `roms`, `rom_set`, `extract`, `extract_reporting`,
  `extract_recording_reads`, `fingerprint`, `drawings`). A `RomSet` is made by `Recipe::rom_set` and refuses a code
  the recipe doesn't declare, a size other than the declared one, and a duplicate; `Recipe::extract` refuses a set
  short of a ROM. `header_code` (the four letters and digits at 0xAC, from the first `HEADER_LEN` bytes) took
  `recognize`'s place: which game a code is of is the declarations' to say.
- *No primitives version* (§8, decision 3, revised): primitives change only with a minor version, and a bundle is
  published for one line, so `[extract]`'s `primitives`, `PRIMITIVES` and the refusal of a recipe that needs newer
  ones went (the three games' manifests with them), and nothing filters the catalog or an index by primitives. The
  fingerprint digests the engine's line in their place: nettai-extract takes the workspace's version, so a minor
  bump changes every fingerprint and a patch release none.
- *The CLI* is `nettai-extract <game> <pack-dir> ROM... [--reads FILE] [--check]`: `<game>` is a game's pack folder
  (content/exe6), or an unpacked bundle whose one game pack declares a recipe; `--content` became `--check` (the
  battle definitions of the content the game is in, loaded against the pack).
- *The app.* `bnlc` knows no game: `Install::roms(codes)` gives a recipe's ROMs in its order, or the codes no
  archive has, and `bnlc::Rom.code` is a `String`. A game's own pack is its content's recipe's
  (`packs/<game>-<fingerprint>`); a recipe that declares a ROM a game listed before it declares is refused (two
  games can't have one ROM). A collection found or chosen that lacks some of a wanted game's ROMs makes it no
  extractable game: the log names the codes, and the player sees `UserError::IncompleteCollection` ("The Legacy
  Collection's files are incomplete: some of BN6's are missing. Have Steam verify the collection's files, then
  choose its folder again.", in English, Japanese, Simplified and Traditional Chinese), a dialog with CHOOSE FOLDER
  where a game can be played, else the loading screen's line with the same button, on the desktop and in the
  browser alike. The stamp is `fingerprint = "..."` alone. The catalog's games are all the catalog's (its
  `Game::named` filter went), the app picking its line's newest bundle of each as before. The save import's compat
  tables are the app's own (`builds::import::Compat`, read by the compat crates' `of_content`), no longer the
  extractor's.
- *The checks the compat crates made* stay the recipes' with the crates' wording, now that no compat crate reads the
  tables on the way: a duplicate chip number says `chips.toml: hicannon and cannon are both chip 0x001` in EXE4 as
  exe4-compat did, the later key first.
- *exelib* took what EXE4's and EXE5's recipes had twice: `buttons.corners` (the cursor's corner tables, with the
  signed byte), `glyphs.chars` (a font's glyphs past the encoding's characters by their number in brackets), and a
  formatter for `names.unique`, by which EXE4 and EXE5 keep their compat crates' wording. EXE5's `same_list` and
  `same_background` stay EXE5's: no other game compares backgrounds. colors.luau's header says where each game
  drops bit 15 (§3.4). Every game's depends.lock follows exelib.
- *Checked:* the harness on the three games (EXE4 3,076 files, EXE5 3,040, EXE6 3,413: the baseline's, and its
  reads); the full-set content audit's asset lookups; the workspace's type check with no warning; the touched
  crates' tests; the content check; a wasm32 type check of nettai-web.

**Step 6.** The verification workspace reads the recipes (§6.6):

- tools/cross-rom: `sweep.py READS FROM TO --recipe <pack>` takes the ROM order from the manifest's `[extract] roms`
  and the places from `extract/*.luau` and exelib's (`pointer`, `pointer_in_each`, `stated`), lists each range that
  differs under its place with the recipe lines that read it (the reads log's fifth column), and works from a full
  set's reads (each ROM read where the recipe chose it); `words.py` lists, for a recipe file, each place's block in
  the four ROMs and the words that point at it. census.py and checkerboards.py are retired with the placeholders.
- tools/shapes/redraw writes the drawings into the packs' `extract/shapes/` (a `.same` link's target where there is
  one); tools/extract-baseline.sh and the briefs' pack commands use the CLI's new form.
- The comments that pointed at the Rust point at the recipes (the two gen_content.py's mugshot numbering, gen_rules'
  `FAMILY_BYTES`, tools/jp); EXE5's generated content followed (comments only; EXE4's already pointed at its
  recipe). tools/content/index.py, which writes the manifests, kept no `[extract]`: it keeps a manifest's tables now.
- gen_content's checks pass: EXE4's and EXE5's against the ROMs, EXE5's auto battle data and rules, EXE6's
  gen-content.

**The declaration in the recipe** (§8, decision 8):

- *The table.* A recipe's `extract/init.luau` returns `{ roms = { ... }, extract = function(set) ... end }`, typed
  `Recipe` in extract.d.luau, and nothing else; each game's comment on its ROMs' order moved from the manifest to
  `roms`. `PackManifest` is `id`, `kind` and `depends`: `Extract` went, and a manifest with `[extract]` or `text` is
  refused (an unknown field). A game has a recipe when it has `extract/init.luau` (`packs::has_recipe`), which the
  CLI and the harness use to find the games.
- *Read without ROMs.* `Recipe::of_content` reads the inputs, opens the recipe's VM (the primitives and `data`, no
  set), loads `extract/init.luau` and reads its table (`model::declaration`): `the recipe's table.roms[2]: BR6E twice`,
  `the recipe's table.extract: expected a function, got string`, an unknown field. The run opens a VM of its own, as
  before, and calls `extract`. The app's Legacy Collection lookup, `RomSet`, the CLI and the harness take the codes
  from `Recipe::roms` as they did.
- *No size.* `RomSet::insert` refuses a code the recipe doesn't state and a duplicate; an image cut short fails
  where the recipe reads past it (`Rom:u32: 4 bytes at 0x08021ab0 are past BR6E (4096 bytes)`, the app's test of a
  failed extraction).
- *No `reads`.* `data` reads any file of the game's `compat/` (`compat/assets.toml`), and the content check refuses a
  literal path that is none. The fingerprint digests the formats, the line, the game and every file under the game's
  `extract/` and `compat/` and its support packs' `extract/`, path and bytes; the manifest is no input. Every
  fingerprint changed once, and no pack: the harness gives the baseline's three packs byte for byte (EXE4 3,076
  files, EXE5 3,040, EXE6 3,413) and its reads, and the CLI's EXE6 pack has the baseline's digests.
- *No `text`.* `locale::text_tables` infers a game's text tables: the top-level tables of text, in any of its
  languages, that are no collection of its root. The check refuses a table of entries that is no collection and a
  collection's table of text, and holds the own language to every text table another language of the game has (in
  place of every declared one). A misspelt text table is no longer refused there: a lookup that finds no table is
  the full content audit's to report. tools/exe5/gen_content.py keeps the committed locales' tables it doesn't write
  (the twelve EXE5's manifest listed), and gen-content's locale check infers them as the engine does.
- *The content hash moved, for the manifests alone.* `Scripts`' hash covers the packs' manifests, which held `text`
  and `[extract]`, so the three games' hashes changed: EXE4 c2d5ff351c08488e to 8870df0294578597, EXE5
  c54ce9defd43ae05 to f13b1322a84e4639, EXE6 69ae86083e34366a to 6e6a185a834ba1d9. They are the hashes the content
  before the change gives when each manifest is hashed as `id`, `kind` and `depends` alone: nothing else moved.
- *The verification workspace.* tools/content/index.py writes a manifest's header comment, `id`, `kind` and
  `depends`, nothing else; tools/cross-rom's sweep.py takes the ROM order from the recipe's `roms`. exelib's
  place.luau points at the recipe's `roms`, and every game's depends.lock follows exelib.

**No packs** (2026-10-27). The user, asked whether the app should read the ROMs at each start instead of keeping
packs: "no packs at all", everywhere. A game's assets are extracted from its ROMs by its recipe as it loads, in
memory, and nothing of them is written:

- *The format went* with everything that read or wrote it: nettai-content's pack, sprite, Aseprite, image, Tiled,
  HUD, custom screen, stage, song (MIDI), sound (WAV), timeline and verify modules, the report's file stamps, its
  `verify` and `aseprite-export`/`-import` commands; `Extraction::files`, `write` and `write_reporting`, and
  `pack::FORMATS`; the fingerprint (§5.2) and the stamp; the `nettai-extract` command (the crate is a library alone:
  `--reads` is `Recipe::extract_recording_reads`, `songs` `songs_played_otherwise`); and the tests that held a
  recipe to packs: the harness (`tests/recipes.rs`, against the baseline's digests: §6.3, §6.5) and
  `full_sets_preserve_reference_graphics`. The letters, the drawings' SVG reading and the asset names stayed, the
  letters and the names in nettai-extract and the SVG in nettai-assets' `shapes`.
- *What checks a recipe now* is the content: `nettai-content check <game>` extracts the game's assets and holds them
  to it (every asset it names, each sprite's animations and frames, the sprites' timing against the ROMs', the
  strings' backgrounds), and the full-set content audit (`full_sets_pass_the_content_audit`, ignored) makes every
  lookup a frontend makes. A recipe change that draws something new is frame-compared, as any drawing change is.
- *Where the ROMs are* is nettai-extract's `source.rs` (`RomSource`, §5.6): `$NETTAI_ROMS` for the tools, the tests
  and development (it replaced `NETTAI_EXTRACT_ROM_DIR` and `$NETTAI_PACKS`), the collection for the app, and in a
  browser the copies it keeps of a collection chosen once.
- *Checked* (2026-10-27, the twelve ROMs): each game loads with its assets extracted, the content times its 175,
  229 and 240 sprites as the ROMs do, and a battle of each draws its frames and plays its sound the same as before
  the packs went.
