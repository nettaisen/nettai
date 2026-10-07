# Asset formats: open, editable, exact

A battle's graphics and sound come from a **content pack**: a folder of
widely supported files that ordinary tools edit, which loads into exactly the
data the frontend and the audio use. This document describes the graphics and
sound formats; the pack's battle data (chips, navis, rules, sprite timing:
what the engine runs on) is described in [content-pack.md](content-pack.md).
A pack is the only form this data takes: `nettai-extract exe6` writes it
from the user's ROM, and everything loads it straight from its files.

The code is the `nettai-content` crate, `nettai-extract exe6`, and pack
loading in `nettai-frontend` (`game`) and the audio examples. Everything below was
checked on the game's full battle graphics and all of its songs; the in-repo
tests use synthetic assets.

## 0. Summary

| Asset | Format | Editors | Fidelity |
|---|---|---|---|
| Battle data (chips, navis, rules...) | TOML by owner ([content-pack.md](content-pack.md)) | text editor | exact (equal to the engine's former compiled tables) |
| Sprites: pixels and palettes | 8-bit indexed PNG part atlas, the palette set as 16 palette rows | Aseprite, GIMP, LibreSprite, Pillow, any indexed editor | byte-exact |
| Sprites: frame layouts | `sprite.json` (OAM parts: first tile, size, offset, flips, palette offset) | text editor | byte-exact |
| Sprites: animation timing | `animations.json` (ticks and flag bits per frame), read without images | text editor | exact (checked against the engine's table) |
| Sprites: whole-frame editing | optional `sprite.aseprite` view: a layer per part, a tag per animation, linked cels for shared tiles | Aseprite | byte-exact after a re-save by Aseprite |
| Field panels | indexed PNG + `field.json` (map entries as `tile:palette[:flip]`) | image editor, text editor | byte-exact |
| Backgrounds | indexed PNG + Tiled map (`map.tmj`) + `background.json` (scroll, animations) | Tiled, image editor | byte-exact, also after a Tiled re-export |
| HUD | indexed PNGs laid out as drawn (glyphs, icons, mugshots) + `hud.json` | image editor | byte-exact |
| Custom screen | indexed PNGs laid out as drawn (the window's tiles, chip pictures, codes, icons) + `custom.json` | image editor, text editor | byte-exact |
| Songs | Standard MIDI file in mid2agb's conventions + TOML sidecar | DAW or MIDI editor | every song plays sample for sample the same |
| Instruments | voicegroups, key maps, PSG waves as TOML | text editor | exact |
| Samples | 8-bit mono WAV with a `smpl` loop chunk + `samples.toml` | Audacity, sox, any audio editor | exact, also after tools drop the loop chunk |

Results (§6): the graphics a pack loads are identical to the graphics
decoded from the ROM; rendered trace frames are pixel-identical; sprite
timing matches the engine's former compiled table for all 3,176 animations;
all 397 songs
have identical timelines and render bit-identical PCM (60 s each, 780 million
stereo samples), and the battle music with all 18 battle effects mixed in is
bit-identical. Nothing in EXE6's data needed an approximation. §7 lists
precisely what a format can't carry and how each case is handled.

## 1. Principles

- **The pack is the only form.** The frontend, the audio and the engine
  load a pack straight from its files; nothing derived from it is stored
  (§9).
- **Open formats first.** Every file is a PNG, JSON, TOML, MIDI, WAV or a
  Tiled or Aseprite document. Where a format can't hold a detail, the detail
  goes in a small text sidecar rather than in a private binary.
- **Exact by construction, then proved.** Export writes a file, and for songs
  immediately reads it back and compares, refusing anything that wouldn't
  return the same. `nettai-extract exe6` reads the asset data back before it
  finishes, and `nettai-content verify` checks a whole pack against another.
- **Imports explain, never guess silently.** An import returns a report of
  errors (the pack can't be built as it is), warnings (it builds, but likely
  not as intended) and notes, each naming the file and what to do.
- **Stamps tell untouched files from edited ones.** Sidecars keep a hash of
  what was exported (a PNG palette's fingerprint, a WAV's or MIDI file's
  bytes, a summary of a song's commands). An untouched file takes the exact
  values from its sidecar; an edited one is read for what it says, and the
  importer points out what an editor probably damaged.
- **Game-independent formats.** The format modules know GBA-shaped data
  (4bpp tiles, 16-color palettes, OAM parts, tile maps, M4A songs and
  voicegroups) and no EXE6 rule. Only the HUD layout and the field's panel
  tables are EXE6-shaped. They can sit under the core that the content sits
  on (see `docs/design/core-content-boundary.md`).

## 2. Pack layout

```text
content.toml                         manifest: format, version, name, what it holds
chips/ navis/ objects/ rules/ registries/   the battle data (content-pack.md)
graphics/
  sprites/NAME/                      one folder per sprite, under its name (compat/assets.toml;
                                     sprite-CC-II without one); sprite.json holds its id
    atlas.png                        part images; palette = the sprite's palette set
    sprite.json                      tile sets (atlas regions) and frame layouts
    animations.json                  timing: ticks and flags per frame
    sprite.aseprite                  optional whole-frame view (§3.5)
  field/
    tiles.png                        panel tiles; palette rows = background palette slots
    field.json                       panel types, blocks, edges, highlights, cycling palettes
  backgrounds/NAME/                   one folder per background (background.json holds its id)
    tiles.png  map.tmj  background.json  anim-K.png
  hud/
    hud.json  layer.png  gauge.png  font.png  enemy-digits.png  counts.png
    banner-digits.png  waiting.png  pause.png  navi-box.png  warning.png
    hidden-icon.png  chip-icons/CHIP.png  mugshots/NAME.png  banners/NAME.png
    dialogue-font.png  chatbox.png  chatbox-arrow.png
  custom/
    custom.json  window.png  column-cells.png  turn-limit.png  name-bar.png
    codes.png  elements.png  digits.png  slot-codes.png  empty-icon.png
    cursor.png  form-list-cursor.png  regular.png
    chip-art/CHIP.png  emblems/NAVI.png  pictures/NAME.png
    buttons/BUTTON.png  pictures/BUTTON.png  buttons/BUTTON-icons.png
    (a button by the name its content registers it under: redeal, scrap, soul)
    form-names-V.png  buttons/BUTTON-V.png  pictures/BUTTON-V.png
    (V: falzar, gregar; a version's own look of a button: beast_out)
    (an EXE5 pack's HUD also: mugshots/NAME-box.png, the box a face brings)
    (and another language's lettering, L: ja)
    hud/font-L.png  dialogue-font-L.png  waiting-L.png  gauge-L.png  banners/NAME-L.png
    custom/pictures/NAME-L.png  form-names-V-L.png
sound/
  sound.toml                         mixer, music players, song table size
  samples.toml                       per sample: file, exact rate, loop start, stamp
  samples/smp-NNN.wav
  waves.toml                         PSG wave shapes
  keymaps.toml                       key splits
  voicegroups/vg-NNN.toml            instruments (drum kits and split groups too)
  songs/NAME.mid                     a song, under its name (sound-XXX without one)
  songs/NAME.toml                    its header (with its song id) and stamps
```

EXE6's pack is 2,759 files, 10.6 MiB (468 of the files, about 2 MiB, are
the battle data). Writing it from the ROM takes about 1 s.

## 3. Sprites

### 3.1 The data

A battle sprite has animations; each frame names a tile set (a block of 8x8
tiles the game copies to video memory for that frame), a palette set, a
layout (the hardware sprites, "parts", the frame is built from) and a duration
with flag bits. EXE6's battle sprites: 298 sprites, 3,176 animations, 8,329
frames each with its own layout, 2,005 tile sets (1,147 of them shared by
several frames), 56,033 tiles, 35,645 parts in the 12 hardware sizes, one
palette set per sprite (16 rows of 16 colors, of which the object picks one
with `sprite_setPalette`; a part's palette offset is added).

### 3.2 Parts, not flattened frames

Flattening each frame into one picture is what artists would like, but it
loses data: in 4,086 layouts parts overlap, and in 3,335 frames (40%) the
overlapping parts have different opaque pixels (an arm over a body). A
flattened frame keeps only the front pixel, while the hidden ones show when a
part is hidden (`Look::hidden_parts`), culled, or when the first part (the
shadow) is drawn on the ground apart from the body. Tiles are also shared:
813 layouts reuse a tile within a frame, and most tile sets serve several
frames. Parts are therefore kept exactly, and whole-frame editing is offered
as a layered view (§3.5) in which each part is a layer.

### 3.3 Files

**`atlas.png`**: every part image once, as an 8-bit indexed PNG. A part image
is the part's tiles as stored (unflipped), `width x height` pixels. Each tile
set starts a new row of the atlas, so a row reads as one pose's pieces.

- The PNG palette (PLTE) is palette set 0: 16 rows of 16 colors, 256
  entries. A pixel's value is `row * 16 + index`: `index` is the GBA color
  index (0 = transparent) and `row` the palette row the part is first drawn
  with, so the atlas shows the right colors. Only `index` is data; the
  importer takes each pixel modulo 16. Entry 0 of every row is marked
  transparent (tRNS).
- Colors are BGR555 widened as `v << 3 | v >> 2`, the renderer's own
  conversion, so every GBA color has exactly one RGB value and back.

**`sprite.json`**:

```json
{
  "format": "nettai-content/sprite", "version": 1,
  "sprite": [0, 0],
  "atlas": "atlas.png",
  "palette_rows": 16,
  "palette_fingerprint": "99011b43f293a39d-085de07e49759465",
  "palette_high_bits": [[15, 3]],
  "tilesets": [
    { "tiles": 30, "regions": [
      {"tile": 0, "size": [32, 16], "at": [0, 0]},
      {"tile": 8, "size": [32, 32], "at": [40, 0]}
    ] }
  ],
  "layouts": [
    [
      {"tile": 0, "size": [32, 16], "offset": [-16, -8]},
      {"tile": 8, "size": [32, 32], "offset": [-16, -40], "flip": "h", "palette": 1}
    ]
  ]
}
```

- `tilesets[t].regions`: which atlas rectangle fills tiles `tile..` of tile
  set `t`, row by row as a sprite of that size reads them. Every tile of a
  tile set is in some region (tiles no part draws get an 8x8 region of their
  own, so nothing is lost; EXE6 has none). If two regions cover the same tile
  they must agree.
- `layouts[l]`: the parts of layout `l`, first part the shadow. `flip` is
  `h`, `v` or `hv`; `palette` is the part's palette offset (omitted when 0).
- `palette_fingerprint`: hashes of the atlas palette as exported, in order and
  sorted, to tell a re-sorted or truncated palette from an edited one.
- `palette_rows` counts the rows the atlas palette holds: an indexed image
  has 256 colors, 16 rows. A palette set with more (MegaMan's has 43: the
  palettes his Crosses use come after his own) lists the rest in
  `more_palette_rows`, each row 16 BGR555 colors in hex. An object picks a
  row by number (`palette`), so the order is the data.
- `palette_high_bits`: palette entries whose BGR555 value has bit 15 set.
  The hardware ignores that bit and a PNG can't hold it. EXE6 has 16,020 such
  entries, all in palette-set rows past the real palettes (the extractor keeps
  16 rows because the game reads whatever follows a palette).
- `extra_palette_sets`: palette sets after the first, as BGR555 hex (none in
  EXE6).

**`animations.json`**: the simulation's data:

```json
{
  "format": "nettai-content/animations", "version": 1,
  "sprite": [0, 0],
  "animations": [
    [ {"ticks": 2, "flags": ["last"], "tileset": 0, "layout": 0} ],
    [ {"ticks": 4, "tileset": 1, "layout": 1},
      {"ticks": 3, "flags": ["last", "loop"], "tileset": 0, "layout": 8} ]
  ]
}
```

- `ticks`: how many ticks the frame shows (1..=255).
- `flags`: the flag byte, spelled out: `"last"` (0x80, the animation ends
  here), `"loop"` (0x40, with `last`: it starts over), and any other bits as
  numbers (`[4]`), which attack code reads as cues. Omitted when 0. Nothing is
  derived: the byte is exactly what the list says. (In EXE6 only the last frame
  of each animation has flags, 0x80 or 0xC0.) The importer warns when an
  animation's last frame lacks `last` or an earlier one has it.
- `tileset`, `layout`, `palettes`: which tile set, layout and palette set the
  frame draws; the simulation ignores them.

Effect lifetimes and chip timings end on these durations and flags, so timing
is kept apart from pixels: `nettai_content::timing::load` reads every
`animations.json` without opening an image (298 files in 10-50 ms), and the
battle data's loader puts it in the engine's `Content::animations`.

### 3.4 Editing

- **Pixels**: open `atlas.png` in any editor that keeps indexed color. Paint
  with the colors of the part's own palette row; only the index within a row
  counts.
- **Palettes and palette swaps**: edit the PNG's palette. Row n is what
  `sprite_setPalette(n)` shows.
- **Layouts and timing**: edit the JSON. A new part needs a new atlas region
  and a layout entry; a new animation needs frames in `animations.json`.

### 3.5 The Aseprite view

`nettai-content aseprite-export <pack> [NAME ...]` writes
`graphics/sprites/NAME/sprite.aseprite`; `aseprite-import` reads it back. It
exists because whole frames are what artists want to see, and Aseprite can
show them without flattening:

- **Frames**: every animation frame in order, drawn whole. **Layers**: one
  per part number (`part 0 (shadow)` at the bottom, then `part 1`, ... in
  drawing order). A part's cel is the part as displayed (flipped as drawn),
  in the palette row of its offset.
- **Tags**: one per animation, `anim NN`, or `anim NN loop` when the last
  frame has the loop flag. The name decides; the tag's repeat count (0 for
  loops, 1 otherwise) only drives Aseprite's preview, because the field is
  from Aseprite 1.3 and older versions and LibreSprite don't keep it. A
  disagreement is reported.
- **Durations**: milliseconds, `round(ticks * 1000 / 59.7275)`. Every tick
  count 1..=3,999 converts to milliseconds and back exactly, and a frame
  duration field reaches 3,914 ticks, so Aseprite's durations hold ticks
  exactly. A duration typed by hand is rounded to the nearest tick, with a
  note.
- **Linked cels = shared tiles**: parts that show the same tiles at the same
  place in several frames are linked cels, so an edit reaches every frame that
  shares them, as it would on the hardware. Shared tiles that can't be linked
  (a different shape or place) must be edited the same way everywhere; the
  importer refuses diverging copies and says how many pixels differ.
- **Palette**: indexed color mode, the 256-color palette = palette set 0.
- The view edits pixels, palettes, durations and loop flags. Part geometry
  stays in `sprite.json`: pixels painted outside a part's rectangle are
  reported and dropped, and adding or removing frames is refused (change
  `animations.json` first).
- The file is written and read by our own code from Aseprite's published
  specification (layers 0x2004, cels 0x2005 with zlib images or links,
  palette 0x2019, tags 0x2018, user data 0x2020); no Aseprite is needed to
  build a pack. Cel user data names each part's tile, size and flips, for the
  artist; the importer doesn't rely on it.

Why a view and not the canonical format: Aseprite is a paid (source-available)
program, LibreSprite follows an older version of the format (no repeat counts
or user data), and the engine's timing loader shouldn't parse Aseprite
files. The atlas and JSON stay the source any
tool can edit; the view is a synced editing workflow.

## 4. Field, backgrounds and HUD

All three store tiles the same way: an indexed PNG in which tile number n sits
at place n (a grid 16 tiles wide; tiles below the first loaded one are blank,
and anything drawn there is reported as ignored), the PNG palette's rows being
the palettes. A `TileImage` entry in the JSON names the file, the layout, the
tile count, which palette rows are data (`palettes: [first, count]`; other rows
only color the image for viewing) and the palette fingerprint.

**Field** (`graphics/field/`): `tiles.png` with palette rows 1..=8 the panel
palettes (row numbers = background palette slots, so a map entry's palette is
the row it shows in). `field.json` holds:

- `panel_types`: the panel types the field draws, by the engine's names
  (`"normal"`, `"cracked"`, ..., `"metal"`, `"lava"`, `"sea"`), in the order
  of their blocks. EXE6's pack lists its 13 in the engine's order, EXE5's its
  11 in EXE5's order. A panel type a field doesn't list is drawn from another
  loaded game's field, or tinted (docs/design/rules-in-luau.md §7.4).
- the panel blocks, six for each listed type (5x3 map entries, the block of
  type `k` (its place in `panel_types`) for an owner and a row at
  `6 * k + 3 * owner + row - 1`): EXE6's 78, EXE5's 66.
- the two front edges;
- one highlight or two (EXE6 has two; EXE5 draws one block for both, which its
  pack writes twice);
- the cycling panel palettes (slot, start timer, frames of `ticks` and 16
  colors).

A map entry is text, `tile:palette` with `:h`, `:v` or `:hv` when flipped;
a color is `#rrggbb`, or `0xNNNN` (raw BGR555) when it has bits RGB can't
hold.

The reader takes the format as the extractors write it today, and nothing older: every file's version is the
format's own (a pack of another is refused, with a note to extract it again), `panel_types`, the custom screen
(with its `layout`, its game's own: no pack leaves it to another game's), the warning marker, the chatbox and the
dialogue font are required, and so are the manifest's `game` and the asset index. (`custom.json`'s version 2 named
the form list window's pictures for it, where 1 named them for EXE6's Cross window: `cross_maps`, `cross-names-V`.)

**Backgrounds** (`graphics/backgrounds/NAME/`): `tiles.png`; `map.tmj`, a Tiled
JSON map (orthogonal, 8x8 tiles, one tile layer, the tileset being
`tiles.png`, gid = tile number + 1, flips as Tiled's flip bits; palettes, when
any cell's isn't 0, as the layer's `palettes` property, a hex digit a cell);
`background.json` (whether it has its own palette, scroll speed in 1/16 pixel,
animations). A tile
animation's frames are `anim-K.png`, one block of tiles a
frame; a palette animation's frames are color lists; a palette shift's
(`brightens` or `darkens`: palettes shown lightened or darkened, per channel,
by a color a frame, EXE4's background 0x09) are one `shift` color each. Tiled's rotation bit is
refused (the GBA only flips), and so are infinite maps and compressed layers,
each with the setting to change.

**HUD** (`graphics/hud/`): one PNG per tile block, laid out as the game draws
it: 8x16 glyphs for the fonts and digits, 2x2 chip icons, 4x2 mugshots, banner
glyph rows. Each palette belongs to one image (the HP box palettes to
`layer.png`, each mugshot's to its own file, and so on); images drawn with
another's palette show it for viewing, and color edits there are ignored with
a note. `hud.json` holds the map entries, what each glyph of the font draws
(`font_chars`: the frontend spells a chip's name, which its definition
gives, with them), each chip icon's chip (its key in the content), the link
navis' mugshots and which one a navi shows, the HUD's text lines as glyph
numbers ("TIME UP!", the turn timer's seconds, "COUNTER HIT!"), banner
layouts and the form emotions.

The HUD holds the chatbox's graphics. `dialogue-font.png`
is the dialogue font: 16x12 cells, 32 a row, palette index 0 clear (its
palette only colors it for viewing: the chatbox draws it with the text's);
`hud.json`'s `dialogue_font` gives each glyph's advance and what it draws
(the 8x16 font's characters for the one-byte glyphs, then the two-byte
codes' kana and kanji, `compat/text.toml`'s `dialogue_glyphs`), so a string
is spelled as the font's. `chatbox.png` is the box's tiles with its
palette, `chatbox-arrow.png` the key-wait arrow's three 16x16 frames with
the palette the text draws with; `hud.json`'s `chatbox` holds the box's
maps (30x8 entries, a row of text each, the tiles counted from the image's
first) by kind (the message box, the description box) and opening step (0
to 3, open).

`font_chars` and the dialogue font's `chars` write the game's marks as
characters (Ⓐ for the A button, U+E002
for the stacked EX: `compat/text.toml`'s, text-rendering.md §10.5), so that
content's strings are spelled in them.

The chatbox's portraits are sprites (`graphics/sprites/NAME/`, category
0x20, `mugshotSpritePtrs`): their animations are the speaker's faces (still,
idle with its blinks, talking), the original's mini-animations of the
portrait's one frame. Each US ROM has a black placeholder in place of the
other game's link navis' portraits; the extractor takes each from the ROM
that has it.

**Custom screen** (`graphics/custom/`): the same scheme.
`window.png` holds the window frame's tiles with the window's four palettes
(by the chip under the cursor's class: standard, Mega, Giga, dark), each
chip's picture (`chip-art/CHIP.png`, 7x6 tiles) its own palette,
`elements.png` a palette row per element whose colors 10-15 are the ones
the element brings, each navi's emblem (`emblems/NAVI.png`, 2x2 tiles,
under the navi's key) its own palette, which is the cursor's and the
Regular chip's frame's too while that navi's console has the screen up,
`form-list-cursor.png`
the form list window's cursor (EXE6's Cross window's: its corner and its
edge, two frames) with sprite palette 14. `custom.json` holds the window's
maps (15x20, without and with the form list's tab) and the form list
window's (`form_list_maps`: three opening steps, then the window with one
to five forms; `form_list_patches`), their patch lists (a block of consecutive tile
numbers at a cell, row or column first, in a palette), the three palettes
no image owns as color lists, and the Program
Advance animation's three sets of name colors; each chip's picture by
its chip's key, with the `version` of a version's own chip's (below);
each navi's emblem by its navi's key; and the buttons
(below). The frontend composes the tile numbers the maps name from
these blocks, as the original's VRAM holds them.

A navi's emblem is its own on any console: the extractor takes each from
the ROM that has it (EXE6's ROMs each have MegaMan's, ProtoMan's and their
own version's five link navis'; EXE5's MegaMan's and their own team's
six), under the key the game's compat gives the navi's number; a navi
without a key has none. The original has no picture for the other
version's navis and shows its own counterpart's in the navi's colors
(docs/frontend.md §5).

**Buttons** (`buttons` in `custom.json`): a button of the rules on the custom
screen is drawn by the pack's look of the name its content registers it
under (EXE6's `redeal`, `scrap` and `beast_out`; EXE5's `soul`, `redeal`
and `arm_change`); the frontend has no look of its own. An entry has the
button's tiles among the slots' (`buttons/BUTTON.png`: `size` tiles a
cell, a slot's 2x3 or the special slot's own; a set its cells', set after
set), which set a state shows (`sets`: `each`, one a state; `other`, the
second for unavailable and picked; `unavailable`, the second for
unavailable alone), the set its slot shows while the button isn't there
(`hidden`: EXE6's Beast Out's fourth; left out, the window's fill), the
cursor over it (`cursor`: its place and each frame's four corners), its
picture in the chip window with its palettes as rows, its own first
(`pictures/BUTTON.png`), whether the chip window shows its uses left
(`uses_digit`: EXE5's Shuffle) and where the chip it holds is drawn
(`held_at`: EXE5's Arm Change). EXE5's `soul` has the souls' 2x2 icons
besides (`buttons/soul-icons.png`, with their sprite palette, which the
soul choice flies onto the column; `icon_versions` names the game versions
whose consoles fly the icon in a palette of their own, the image's rows
after the first: Team Colonel's). A version's own look of a button is in
its version's entry (EXE6's `beast_out`: `buttons/beast_out-V.png`,
`pictures/beast_out-V.png`), and a language's own tiles or picture of one
in its language's (`buttons/soul-ja.png`, `pictures/redeal-ja.png`).

**Another game's HUD and custom screen** (EXE5's, docs/design/exe5-map.md §11)
use the same files, with optional fields where its game lays them out
otherwise; an EXE6 pack writes none of them and is byte-identical to before:

- `hud.json`: the tile numbers count from the HUD layer's and the gauge's
  first tiles (`first_tile`, `gauge_first_tile`: EXE5's 0x180 and 0x202,
  EXE6's 0x1A0 and 0x222). `mugshot_boxes` gives a face that brings its own
  2x2 box beside it (`mugshots/NAME-box.png`: EXE5's MegaMan, dark MegaMan
  and Team Colonel's faces), by mugshot number; `no_count_box` says the
  game has no box without a count (EXE5's souls' faces show their turns
  left: `counts.png`, by count).
- `custom.json`: `layout` (every pack says its game's) puts the
  window's parts at the game's tile numbers (the chip's name, picture,
  code, element and digits, the slots, the column's icons and cells, the
  turn limit, the name bar, the Cross names), says which tile a hidden slot
  is filled with, and where the cursor stands over OK, with its corners.
  `buttons` are the game's own (above: EXE5's special slot is the 3x2 soul
  button, with a palette for Soul Unison and one for Chaos Unison; EXE6's
  the 4x2 Beast Out).
- EXE4's `custom.json` (docs/design/exe4-map.md §14) says what its screen
  draws otherwise, each left out of the others': its layout's
  `detail_blank` (the color a blank code or damage cell is, 7 where
  EXE6's and EXE5's are 8) and `empty_palette` (the empty icon's palette in
  a slot or cell without a chip, 9); `element_sprite` (the element icon
  drawn as a 16x16 sprite at a place and in a palette of its own, not as
  window tiles in palette 11 with `elements.png`'s colors); `cursor_palette`
  (the cursor's and the Regular chip's frame's own sprite palette, not a
  navi emblem's); `window_emblem` (the orb over the picked column drawn on
  the window's map: `window-emblem.png` at `first_tile`, its frames' map
  entries, and the steps of its turn after a pick, a frame or a hold each);
  and a button's `place` ([column, row, first tile, palette]: the UNITE
  button's own cells on the window's map, not among the slots').
- A version's own chip (EXE5's and EXE6's version Giga chips, EXE5's Phoenix
  and DethPhnx), which the other version's ROM draws as its counterpart,
  has its icon and picture once, under the chip's key, from its own
  version's ROM, the picture with that `version`: the frontend shows it on
  either console, and a console of the other version's is a known
  difference there (docs/frontend.md §5).

**Four ROMs, and what differs by version and region.** An EXE6 pack is made
from the two US ROMs and the two Japanese ones (`nettai-extract exe6 <pack>
<falzar-us> <gregar-us> <falzar-jp> <gregar-jp>`, any subset and order,
identified by game code: BR6E, BR5E, BR6J, BR5J): the
US Falzar ROM's data, with what only the US Gregar ROM has right or of its
own, read at the addresses the same code points at there (exe6-extract's
`gregar`), and what the US release cut, from the Japanese ROMs
(exe6-extract's `jp`):

- **What differs by version** is two assets, each named with its version:
  `form-names-falzar` and `form-names-gregar` (nettai-assets
  `Versioned`, whose halves the names find; an asset no version has its
  own of has no suffix). On the custom screen these are the Beast Out
  button, the version's Beast's (`buttons/beast_out-V`, four sets of 4x2:
  selectable, unavailable, battle mode 1's, the hidden slot's), with its
  picture in the chip window (`pictures/beast_out-V`, which the BeastOut
  chip shows too) and the form list window's (the Cross window's)
  names, 9x2 tiles each, the five on the cursor's row then on the others',
  with background palette 10 for the Cross under the cursor
  (`form-names-V`; the layout's `form_names` places them). `custom.json` lists the base's (`own`, its version in
  `base_version`) and the others' (`versions`). A console shows its
  version's, except a Cross's name, which is the Cross's own game's.
- **Gregar's own faces**: the Falzar ROM has none for Gregar's Crosses,
  its Beast and its link navis (its tables show the Falzar counterpart's).
  The pack has the Gregar ROM's as faces of their own (`mugshots/heatcross`
  and the others: numbers 0x17 to 0x28 after the Falzar ROM's emotion
  pictures, 0x86 to 0x8A after its link navis'), and Gregar's forms and
  navis name them: every form shows its true face on either console.
- **Five chips' pictures and icons** the Falzar ROM has wrong (Bass,
  BigHook, DeltaRay, ColForce, BugRSwrd: the other five Giga chips'
  copied in): the pack has the Gregar ROM's, on either console.
- **What the US release cut** (docs/engine/jp-differences.md §4.4, §4.5):
  six sprites the US ROMs fill with a placeholder archive (`count`,
  `django`, `otenko`, `falzar-summon`, `gregar-summon`, `blocking-banner`),
  from the Japanese Falzar ROM, and the pictures of eleven chips the US ROMs
  give a placeholder picture (GunDelEX, Otenko, Count (HackJack) ×3, Django ×3,
  DblBeast, Gregar, Falzar), from the Japanese Falzar ROM but Gregar's,
  from the Japanese Gregar ROM (a Japanese console shows its own beast in
  both chips; the pack has each chip's own). The pack keeps no region: a
  pack has one picture of each thing, the Japanese games' where the
  releases differ, and what a console of the other region shows there is
  the verification's to know (docs/frontend.md §5).
  Three of the pictures' palettes are EWRAM in the original, which a link
  gift (an e-Reader card's) fills and the save keeps: DblBeast's, the
  card's, is also orphaned in the Japanese ROMs, and the pack has it; the
  Gregar and Falzar chips' are in no ROM, and the pack's pictures have a
  black one, which their definitions' `art_palette` replaces when drawn.

**Languages.** The words a battle's pictures and fonts show are the pack's
own language's (`language` in `hud.json`, "en" for a pack from the US ROMs)
and, for another language the pack has (nettai-assets `lettering`), that
language's own files, named with it: `hud.json`'s `languages.ja` holds the
Japanese 8x16 font (`font-ja.png`) and what each glyph draws, the dialogue
font (`dialogue-font-ja.png`, its advances and characters), the HUD's text
lines in its glyphs, the banners whose words differ (`banners/NAME-ja.png`,
each with its place: a longer name starts further left; null for the
others), "Cstmzing..." as wide as its words (`waiting-ja.png`: カスタム中…,
seven tiles where the US's are eight) and the gauge (`gauge-ja.png`, its "L
or R"); `custom.json`'s `languages.ja` the chip window's pictures for OK,
the re-deal and scrap (`pictures/ok-ja.png`...: "chip data transmission"
in Japanese), the form list window's names by version
(`form-names-falzar-ja.png`, `form-names-gregar-ja.png`) and, under
`buttons`, a named button's tiles where they say something
(`buttons/soul-ja.png`: EXE5's soul button, "uni son" for "UNITE"). They
come from the Japanese ROMs (exe6-extract's and exe5-extract's
`lettering`); the fonts' characters from compat/text.toml's `[jp]`, the
Japanese ROMs' encoding. Everything else a battle shows is the same
pictures in a game's four ROMs. A frontend in that
language swaps them in (`Bundle::in_language`). The content's display text
(names, descriptions, messages) is no asset: it is the content root's strings
(`locales/<lang>.toml`, docs/design/text-rendering.md §10).

Tiled was considered for the field's panel blocks too, but a panel tile is
drawn in different palettes for each side, which a Tiled tileset can't show
without duplicating tiles; the text entries are exact and short.

## 5. Sound

### 5.1 Songs: the timeline

An M4A track is a command stream with control flow: GOTO, pattern calls
(PATT/PEND, nested up to three deep), REPT, FINE, and optional note arguments
that repeat the previous key or velocity. MIDI is a timeline. The pivot is
`timeline::linearize`, which plays a track's control flow exactly as the driver
does (the pattern stack, the shared repeat counter, the depth limit) and
records every other command at its tick, with keys and velocities made
explicit, until the track ends or its state (position, call stack, repeat
counter, last key, last velocity) repeats. That point is the loop. Two tracks
with equal timelines are indistinguishable to the driver; `encode` turns a
timeline back into a straight-line track.

A loop is then moved to a whole tick: `Timeline::loop_from` finds the earliest
tick from which everything repeats with the loop's period, and `with_loop`
re-cuts the timeline there. A song's tracks all get one loop: from the latest
track's earliest start (rounded up to an eighth note) over the common period.
In EXE6, 26 of the 32 looping songs have tracks whose repeats settle at
different points (a note that takes its key from the pass before), and every
song has one period.

### 5.2 Songs: the MIDI mapping

Format 1, 24 ticks a quarter note (one MIDI tick = one M4A tick). Track 0 is
the conductor: the song's name, a 4/4 time signature (for editors only), the
tempo and the loop markers. M4A track n is MIDI track n + 1 on channel n; its
commands keep their order on each tick.

| M4A | MIDI |
|---|---|
| N01..N96 (gate), TIE .. EOT | note on / note off; 96 ticks or less = gated note, longer = tie (mid2agb's rule) |
| VOICE | program change |
| VOL, PAN, MOD | CC 7, 10, 1 |
| BEND | pitch bend, the high 7 bits |
| BENDR, LFOS, MODT, TUNE, LFODL | CC 20, 21, 22, 24, 26 (mid2agb) |
| PRIO | CC 33 (mid2agb; 39 also read) |
| XCMD 8, 9 (pseudo-echo volume, length) | CC 30 = 8 or 9, then CC 29 = value (mid2agb; 31 also read) |
| TEMPO (BPM / 2) | tempo meta event; in the conductor when it opens its tick on track 0, else on its own track |
| the loop | markers `[` and `]` in the conductor (mid2agb; `loopStart`, `loopEnd` also read) |
| FINE | the track's end-of-track event |

What MIDI can't say, and how it is said (extensions use controllers General
MIDI leaves undefined; mid2agb ignores them):

| Case | Carried by | In EXE6 |
|---|---|---|
| KEYSH (key shift; sounding notes follow it) | CC 102 = shift + 64 | 602 (one KEYSH 0 per track, mid2agb's boilerplate) |
| a tie of 96 ticks or less; a gated note over 96 | CC 103 = 1 or 2 just before the note | none |
| EOT with no tie of its key sounding | CC 104 = key | none |
| FINE before a note's end (the note is cut) | marker `fine` on the track | none |
| a tie that ends in the loop's next pass | its note off after `]` (note offs after `]` only end notes that cross the loop's end) | 6 tracks |
| same-tick order (VOICE, PRIO, LFODL or echo before or after a note differ) | the order of events in the track | always controls before notes |
| where TEMPO sits among track 0's commands | conductor tempo goes after the tick's KEYSH, else it stays on its track | all 397 open tick 0 after KEYSH |
| song header: player, priority, reverb, voicegroup | sidecar | |

The sidecar (`NAME.toml`):

```toml
midi = "song-015.mid"
player = 31
priority = 20
reverb = 40
voicegroup = "vg-001"
tracks = 6
midi_stamp = "2a4ef49c07410705"

[exported]
loop = [204, 2508]

[exported.commands]
bend = 31
key_shift = 6
note = 1476
pan = 112
voice = 26
# ...
```

`[exported]` is only for checking: when an edit loses the loop markers, moves
a track's end, or drops every command of a kind, the importer warns (§8).

Export writes the file and reads it straight back; a song whose file wouldn't
read back as the same timelines isn't written (none in EXE6). The importer
pairs note on and note off first in, first out per key, as most MIDI software
does, and accepts any division that is a multiple of 24 (DAWs save at 480 or
960); events between the 24-a-beat grid are refused with their positions.

**mid2agb compatibility.** The controller meanings, the loop markers, the
tie rule and the tempo conversion are mid2agb's (pret's decompilation tool),
so the files build with it: pret's mid2agb reads the exported battle theme and
writes a looping six-track song with every controller (it re-compresses
repeats into PATT/PEND on its own). Differences: mid2agb quantizes
velocities through its table (ours are exact), rounds gates unless given `-E`,
re-sorts events on a tick by type, and ignores the extension controllers
(harmless for EXE6, which uses only KEYSH 0).

**Compared with the arranger's song document.** bnmusic's arranger stores a
song as a Protocol Buffers `.song` file: per track, notes (tick, key with
KEYSH folded in, velocity, gate, voice) and control changes, one loop tick for
all tracks, and optional "breaks" for byte-identical re-encoding. It is
lossless for what it models, but it is our own binary format (readable only
through its schema), it fixes the order of controls on a tick (controls
before notes, VOICE derived from notes), folds KEYSH into keys (though a
KEYSH change also moves sounding notes), and its MIDI export is documented as
lossy. The MIDI mapping here is exact on the driver's own terms (timelines,
PCM), keeps same-tick order and KEYSH as commands, and puts the song in a
format every sequencer opens. What the arranger has that this doesn't: section
markers, titles and meter for editors (MIDI markers and time signatures could
carry them), and byte-identical re-encoding of the original command stream
(§7).

### 5.3 Instruments and samples

**Voicegroups** (`voicegroups/vg-NNN.toml`), one inline table per voice,
voice n = program n:

```toml
voices = [
  { type = "square1", duty = 2, key = 60, attack = 0, decay = 0, sustain = 15, release = 0 }, # 0
  { type = "direct_sound_fixed", sample = "smp-000", key = 60, attack = 255, decay = 0, sustain = 255, release = 0 }, # 1
  { type = "drums", kit = "vg-004", key = 60, attack = 0, decay = 0, sustain = 0, release = 0 }, # 2
]
```

Types: `direct_sound` (a sample, pitched by key), `direct_sound_fixed` (played
at the mixing rate), `square1`, `square2` (`duty` 0..=3), `wave` (a PSG
wave), `noise` (`narrow`: the 7-bit LFSR), `drums` (`kit`: a voicegroup whose
voice k plays key k at its own `key` and `pan`), `split` (`group` and
`keymap`), `silent`. Envelopes are the driver's bytes: Direct Sound steps
per frame, PSG frames per level. This is the pret decompilations'
`voice_*` macro set, as data.

**Key maps** (`keymaps.toml`): `km-NNN = [[first key, last key, voice], ...]`.
**Waves** (`waves.toml`): `wave-NNN = "0123456789abcdeffedcba9876543210"`, 32
steps of 0..=15.

**Samples** (`samples/smp-NNN.wav`): 8-bit unsigned mono PCM (the GBA's signed
PCM shifted by 128, exact), with a `smpl` chunk: unity key 60 and the loop
(start, and the last sample as end, since M4A loops to the end). The WAV
header's sample rate is whole Hz; the GBA's rate is Hz in 1/1024 steps, so
`samples.toml` keeps the exact value:

```toml
[samples.smp-000]
file = "samples/smp-000.wav"
rate_hz = 10512.0
loop_start = 1234
stamp = "3d93ae4c4b7462a4"
```

An untouched WAV (stamp matches) takes rate and loop from `samples.toml`. An
edited one is read for what it says: its data (16-bit detail is rounded to 8
bits, with a warning), its rate (if the header's whole-Hz rate is the rounded
exact rate, the exact one is kept; otherwise the new rate counts, with a
warning), its loop from `smpl` (samples after the loop end are cut, since M4A
can't play them), or, if an editor dropped `smpl`, the loop start from
`samples.toml` with a warning. A unity key other than 60 converts the rate.
Stereo and floating-point files are refused with what to do.

**Mixer and players** (`sound.toml`): mixing rate, Direct Sound channels,
master volume, reverb, and each music player's track limit, priority rule and
track order.

### 5.4 SF2 and SFZ: listening only

SoundFont 2 and SFZ can't reproduce the driver, confirming the premise:

- **Envelopes.** M4A's Direct Sound envelope is integer arithmetic per frame:
  attack adds a constant up to 255, decay and release multiply by `d/256` and
  truncate (so they reach 0 in finite time, faster than an exponential near
  the bottom), sustain is a level. SF2's volume envelope is linear in
  amplitude for attack and linear in decibels for decay and release, in
  timecents; the shapes differ and the truncation can't be expressed. PSG
  envelopes step through 16 levels every n frames; SF2 and SFZ are smooth.
- **Pseudo-echo** (XCMD 8/9) holds a note at a level for n frames after its
  release: no SF2 or SFZ stage does that.
- **Channels and mixing.** Four Direct Sound channels mix at 13,379 Hz into
  an 8-bit buffer with the driver's ring-buffer reverb, then are held up to
  32,768 Hz beside the PSG; notes steal channels by priority across all music
  players. A synthesizer mixes at full precision with its own polyphony,
  reverb and interpolation.
- **Volume and velocity** are linear products in M4A; SF2's default velocity
  curve is concave (a custom modulator can linearize it).
- **PSG voices** (square duty, wave, LFSR noise with the driver's frequency
  table) would have to be sampled.

An SF2 or SFZ export is still useful for auditioning songs in a DAW, and is a
reasonable next step, but only as a listening aid; the pack's TOML and WAV
files are the instruments.

## 6. Fidelity results

On EXE6 (US Falzar), everything exported, imported and compared with the data
it came from, the data decoded from the ROM:

| Check | Result |
|---|---|
| Graphics loaded from the pack | identical to the ROM's (298 sprites, 21 backgrounds, field, HUD) |
| Frontend, 9 trace frames (headless), from the ROM's graphics and from the pack | pixel-identical PNGs |
| Timing loaded from `animations.json` vs the engine's former compiled table | identical: 298 sprites, 3,176 animations, 8,329 frames |
| Battle data loaded from the pack vs the engine's former compiled tables | identical: 5,841 checks over 67 tables ([content-pack.md](content-pack.md) §7) |
| Frontend, all 2,405 frames of the machgun trace, from the pack vs before the engine loaded packs | pixel-identical |
| Aseprite views written and read back | identical sprites |
| Aseprite views re-saved by Aseprite 1.3.2, then read back | identical sprites (all 298) |
| Instruments, samples, key maps, waves, mixer, players | identical |
| Songs as timelines | 397 of 397 identical (186 also command for command) |
| PCM, each song alone, 60 s | 397 of 397 bit-identical, 780,391,634 stereo samples, the 4 battle songs and 18 battle effects included |
| PCM, battle music with every battle effect started over it, pairs at once | bit-identical, 727,476 samples |

A frame drawn from a pack in which one pixel block was painted in Aseprite
(via its scripting interface, then `aseprite-import`) shows the change on
both navis: edits reach the game through the pipeline.

## 7. Lossy spots

None were hit by EXE6's data. What the formats can't carry, precisely:

**Graphics**

- A PNG can't hold BGR555 bit 15: carried in `palette_high_bits`.
- Colors edited to values off the 5-bit grid are rounded (warning).
- A pixel painted with a color from another palette row keeps only its index
  within the row; its tile then uses the part's row (warning when a tile mixes
  rows).
- Two atlas regions, or two Aseprite cels, covering the same tile must agree;
  they can't hold different pixels for one tile (error).
- The Aseprite view can't change part geometry or the number of frames; those
  edits go to the JSON files.
- 3.2: a flattened frame would lose hidden pixels, which is why parts are
  kept.

**Songs** (export refuses these with the reason; none occur in EXE6)

- Command values of 128 and up (VOL, PAN, LFOS, PRIO, ...: bytes in M4A, 7
  bits in MIDI). Would need a high-bit controller.
- Notes with velocity 0 (a MIDI note on with velocity 0 is a note off).
- Two ties on one key at once on a track (MIDI note pairing is ambiguous).
- Songs in which some tracks loop and others end (one loop per song); and
  loops that only line up after 2^20 ticks.
- Tempos under 4 BPM (a MIDI tempo can't be that slow).

**Songs, when edited** (the importer rounds and warns)

- Tempos that aren't a whole even BPM are rounded to M4A's BPM / 2.
- Pitch bends using MIDI's low 7 bits are rounded to M4A's 128 steps.
- Events off the 24-a-beat grid are refused.
- Same-tick order matters for VOICE, PRIO, LFODL and echo before or after a
  note, and for notes competing for channels. Software that re-sorts events
  on a tick can change what plays; nothing in the file tells an intended order
  from a re-sorted one.
- A round trip gives the same timelines, not the same command bytes: patterns
  and repeats are unrolled, keys and velocities written out, and the loop may
  start later than the original's (by up to a pass, when a track's key state
  settles late). The driver can't tell the difference. (186 of 397 songs are
  identical even command for command: the effects, which have no control
  flow.)

**Samples**

- The exact rate (1/1024 Hz) and the loop live in `samples.toml` when an
  editor rewrites the WAV header or drops `smpl`; a resampled file takes its
  new whole-Hz rate.
- 16-bit detail is rounded to 8 bits; stereo is refused.

## 8. Tooling

Tested on EXE6's pack (each tool re-saved every file of its kind in a copy of
the pack, then `nettai-content check` and `verify`):

| Tool | What it did | Result |
|---|---|---|
| Aseprite 1.3.2 | PNG to .aseprite and back, all 366 PNGs | exact |
| Aseprite 1.3.2 | re-saved all 298 sprite views | exact |
| GIMP 3.0.4 (batch export) | keeps the palette; rewrites transparent pixels of other rows to index 0 | exact (only the index within a row counts) |
| Pillow 12.3 | re-save | exact |
| ImageMagick 7.1 (default save) | drops unused palette entries (256 to 24), re-sorts palettes | refused: "the editor dropped entries ... indices have shifted" / "the editor re-sorted it" |
| Tiled 1.11 (`--export-map`) | re-exported all 21 maps; its renderer draws them | exact |
| sox | re-save, 8-bit and 16-bit | drops `smpl` from all 27 looped samples; loop recovered from `samples.toml` with a warning; exact |
| ffmpeg 9 | re-save | same as sox; exact |
| MuseScore 4 | MIDI import and export of the battle theme | drops every M4A controller, KEYSH, all 31 pitch bends and the loop markers, cuts 112 pan changes to 10 and 26 program changes to 10, re-times 1,475 events, saves at 480 per quarter: refused ("events fall between M4A's 24 ticks a beat") |
| pret's mid2agb | builds the exported battle theme | a looping six-track song with every controller |

Which editor for what:

- **Sprites**: Aseprite (the whole-frame view, or the atlas), LibreSprite or
  GIMP (the atlas; keep indexed mode and the palette's order). Tiled has no
  part/OAM notion and isn't suited.
- **Backgrounds**: Tiled for maps, any indexed editor for tiles.
- **Songs**: a DAW or sequencer that keeps all controllers and markers
  (REAPER, Cakewalk, Domino, MidiEditor; untested here, but most DAWs save at
  480 or 960 per quarter and move tempo to the first track, which the importer
  expects). Not notation programs (MuseScore re-times notes and drops
  controllers).
- **Samples**: Audacity or any audio editor. General audio editors drop
  `smpl` loop points (sox and ffmpeg did here; Audacity is known to), and the
  importer restores them from `samples.toml` as long as the length doesn't
  change. Moving a loop needs a sample editor that writes `smpl`, or an edit
  of `loop_start`.
- **Polyphone** and other SoundFont editors: only for an SF2 export for
  listening (§5.4), not for the pack's instruments.
- **Instruments**: a text editor.

What breaks with an ordinary tool, and what the importer says:

| Damage | Detected by | Message |
|---|---|---|
| palette re-sorted | palette fingerprint (same colors, other order) | error: "re-sorted ... undo the save or turn off palette sorting" |
| unused palette entries dropped | fewer entries than the image needs | error: "dropped entries ... re-export" |
| saved as RGB | PNG color type | error: "re-save it in indexed mode with the original palette" |
| colors off the GBA grid | per entry | warning, rounded |
| colors from another palette row | per tile | warning |
| a sprite view converted to RGB | Aseprite color depth | error: "convert it back (Sprite > Color Mode > Indexed)" |
| a Tiled map rotated, infinite, compressed | the map's fields | error naming the setting |
| MIDI off the M4A grid | per event | error with bar.beat.tick positions |
| loop markers lost | `[exported] loop` | warning: "the song now plays once and stops" |
| M4A controllers dropped | `[exported.commands]` | warning per kind: "all N key_shift commands are gone" |
| tracks padded or merged (FINE moved) | `[exported] ends` | warning with old and new ends |
| tempo not a whole even BPM | per event | warning, rounded |
| WAV loop chunk dropped | missing `smpl` | warning, loop from `samples.toml` |
| WAV resampled | header rate | warning, new rate used |
| WAV stereo, float | format | error |
| any file edited | stamps | note: "edited since export" |

`nettai-content check <pack>` runs every import and prints the report.

## 9. Loading

**Straight from the files.** `nettai_content::pack::load_battle(pack)`,
`load_graphics(pack)` and `load_sound(pack)` read a pack's battle data,
graphics and sound through the importers, each with a report of what it
found. Nothing derived is stored: an edit shows up the next time the pack
loads, and there is no cache to go stale.

**What it costs.** EXE6's full pack loads in about 0.75 s in a release
build (three runs: 0.75, 0.76, 0.86 s): the battle data in 45-150 ms (the
sprite timing is 10-100 ms of it), the graphics in 60-110 ms and the sound
in 520-640 ms. Sound dominates: its MIDI songs are parsed and checked
against their sidecars, and its WAV samples read. The frontend loads the
sound only when it plays it (not headless, not with `--mute`). Should the
total ever pass about a second, the sound import is where to look first.

**The engine.** The simulation reads its sprite timing from the pack's
`animations.json` files, loaded with the rest of the battle data into
`Content::animations` (see [content-pack.md](content-pack.md)); nothing is
compiled into the engine.

**From the ROMs.** `nettai-extract exe6 <dir> [ROM ...]` writes a pack in one step from any
subset of the four ROMs, generating placeholders for unavailable assets (the graphics
extraction, the sound extraction, the exporters, then reading it all back),
a few seconds. It is the only extraction.

## 10. Commands

    cargo run -p nettai-extract -- exe6 data/exe6 <falzar-us> <gregar-us> <falzar-jp> <gregar-jp>    # ROMs -> pack
    cargo run -p nettai-tools -- <trace.jsonl> --headless 100 --out <dir>   # a frame, from every pack in data
    cargo run -p nettai-content -- check data/exe6            # lint every file
    cargo run -p nettai-content -- verify data/exe6 <reference-pack> [--seconds N]
    cargo run -p nettai-content -- aseprite-export data/exe6 [NAME ...]
    cargo run -p nettai-content -- aseprite-import data/exe6 [NAME ...]
    cargo run -p nettai-content --example midi_summary -- a.mid b.mid
    cargo run -p nettai-content --example stats -- data/exe6
    cargo run -p nettai-content --example audio_stats -- data/exe6

`verify` compares what two packs load: the battle data record by record,
the graphics part by part, the sprite timing frame by frame, and the sound
as instruments, as song timelines and as rendered PCM (each song for
`--seconds`, default 60, and the battle music with every battle effect).
Use it after a round trip through an editor, against a pack exported
before.

`data/` is ignored by version control: a pack exported from the ROM
holds the game's graphics and recordings and is never committed.

## 11. Open problems

- **Structural edits in the Aseprite view**: adding frames, moving or resizing
  parts, and auto-splitting a new whole-frame drawing into hardware parts (a
  packing problem; the original's own splits come from the data). Today those
  edits go to the JSON files.
- **New content without an original**: an importer path that takes a plain
  flattened frame and chooses parts, for sprites that don't need hidden
  pixels.
- **Songs with some tracks looping and others ending**, command values of
  128 and up, velocity 0 and overlapping same-key ties: each needs a small
  extension (per-track loop markers, a high-bit controller, ...); none occurs
  in EXE6.
- **Same-tick order under DAWs**: a DAW that re-sorts events on a tick can
  change which note gets a channel. A lint could compare against the
  exported order when the notes are otherwise unchanged.
- **DAWs untested**: REAPER, Logic, Cakewalk round trips should be run by
  hand; the importer's expectations (multiples of 24 per quarter, tempo in
  the first track, controllers kept) come from their documentation.
- **The HUD layout and chip names** are EXE6 schema; a game-independent HUD
  would describe its elements as data.
- **SF2/SFZ export** for auditioning songs outside the game.
- **Field panels in Tiled** would need one tile per (tile, palette) pair.
