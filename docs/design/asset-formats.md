# The game's assets: extracted from its ROMs, in memory

A battle's graphics and sound are a game's **assets**: extracted from the game's ROMs by its content's recipe as
the game loads (`nettai_extract`, [extraction-recipes.md](extraction-recipes.md)), in memory, into typed values a
frontend draws and plays: nettai-assets' `Bundle` (the sprites, the field, the backgrounds, the HUD and the custom
screen), m4a's `SoundBank`, and the index of their names (`AssetIndex`). Nothing of them is written to disk. The
battle data (chips, navis, rules, the sprites' timing: what the engine runs on) is the content's, described in
[content-pack.md](content-pack.md).

Until 2026-10-27 the assets were kept as a **pack**: a folder of open-format files (indexed PNGs with JSON, Tiled
maps, an Aseprite view, MIDI songs with TOML sidecars, WAV samples) that `nettai-extract` wrote and every frontend
loaded. The pack and its formats are gone (docs/app.md §17). This document now describes what the extraction makes:
the model and the choices behind it, and the one input of extraction that isn't a ROM, the frames drawn by hand as
shapes (§4, "Frames as shapes").

## 0. Summary

| Asset | In memory | From |
|---|---|---|
| Sprites: tiles, palettes, frame layouts | `SpriteSheet`: tile sets, OAM parts by layout, palette sets, each animation's frames | the ROMs' sprite archives |
| Sprites: timing | the content's (`asset.sprite`: content-model-v2.md §6.3); the ROMs' kept beside, for the check | the content; the ROMs |
| Field panels | `Field`: tiles, the panel blocks by the game's panel type names, edges, highlights, cycling palettes | the ROMs |
| Backgrounds | `Background`: tiles, map, palette, scroll, animations | the ROMs |
| HUD | `Hud`: fonts, digits, chip icons, mugshots, banners, the chatbox, its layout, its letters, its shapes | the ROMs; the drawings |
| Custom screen | `CustomScreen`: the window's maps and tiles, chip pictures, elements, emblems, buttons, its letters, its shapes | the ROMs; the drawings |
| Another language's lettering | `HudLettering`, `CustomLettering` (`Bundle::for_language`) | the other release's ROMs |
| Sound | `SoundBank`: songs, voicegroups, key maps, PSG waves, samples, mixer, players | the ROMs |
| A version's own songs | `nettai_extract::SongVersions` | the other version's ROM |
| Names | `AssetIndex`: every asset by kind and name, with the game's number for it | the content's compat/assets.toml |

## 1. Principles

- **The ROMs are the input.** A game's assets are extracted from every ROM of the game (a complete Legacy
  Collection's: no partial sets), each block read through the game's own pointer to it. The one other input is the
  recipe's own files, its drawings among them (§4, "Frames as shapes").
- **Extraction explains.** What it notices that isn't wrong enough to fail it is said (`Extraction::warnings`: a
  picture with no drawing, a drawing far off its picture, a set of digits with no face); what is wrong fails it, with
  the reason.
- **Checked against the content.** The content names every asset it uses and times every sprite; as a game loads,
  its assets are checked against it (`nettai_content::load::asset_numbers`: every asset named is there, each sprite
  with as many animations and frames), and `nettai-content check <game>` holds them to it besides: the content's
  timing against the ROMs', the strings' backgrounds against the assets'.
- **Game-independent model.** The model knows GBA-shaped data (4bpp tiles, 16-color palettes, OAM parts, tile
  maps, M4A songs and voicegroups) and no game's rule; what a game lays out otherwise is in its layouts (`HudLayout`,
  `CustomLayout`), which its recipe fills.

## 2. Names

The index (`AssetIndex`, made by the extraction: `nettai_extract`'s names) lists every sprite, sound, banner,
background and mugshot by name, with the game's number for it (a sprite's category and index, a sound's song-table
number, a banner's, background's or mugshot's id): every name the game's compat/assets.toml gives, and the other
assets under placeholders (`sprite-0c-2d`, `sound-10e`), which content may not use (`Extraction::unnamed` lists
them). Content names its assets (`asset.sprite("bomb")`), never their numbers; a frontend finds each handle's
number in the index as the game loads (`AssetNumbers`).

## 3. Sprites

### 3.1 The data

A battle sprite has animations; each frame names a tile set (a block of 8x8 tiles the game copies to video memory
for that frame), a palette set, a layout (the hardware sprites, "parts", the frame is built from) and a duration
with flag bits. EXE6's battle sprites: 298 sprites, 3,176 animations, 8,329 frames each with its own layout, 2,005
tile sets (1,147 of them shared by several frames), 56,033 tiles, 35,645 parts in the 12 hardware sizes, one palette
set per sprite (16 rows of 16 colors, of which the object picks one with `sprite_setPalette`; a part's palette offset
is added).

### 3.2 Parts, not flattened frames

Flattening each frame into one picture loses data: in 4,086 layouts parts overlap, and in 3,335 frames (40%) the
overlapping parts have different opaque pixels (an arm over a body). A flattened frame keeps only the front pixel,
while the hidden ones show when a part is hidden (`Look::hidden_parts`), culled, or when the first part (the shadow)
is drawn on the ground apart from the body. Tiles are also shared: 813 layouts reuse a tile within a frame, and most
tile sets serve several frames. Parts are therefore kept exactly (`SpriteSheet`).

### 3.3 Timing

The battle is timed by the content: each sprite's animations, each frame's duration and whether it loops, are its
module's (`asset.sprite`, content-model-v2.md §6.3). A frame drawn is looked up by the battle's animation and frame,
so the sheet has the frames each animation draws and no timing. The extraction keeps the ROMs' timing beside the
sheets, for one use: `Extraction::timing_differences` compares the content's with it (each frame's duration, the
loop on the last frame, flags the content can't state, the parts a sprite states), and `nettai-content check <game>`
fails on any difference.

## 4. Field, backgrounds, HUD and the custom screen

Tiles are 8x8 and 4bpp, a tile block a run of them as the game loads it to video memory (tile number n at place n,
from the block's first tile number); palettes are 16 BGR555 colors (widened as `v << 3 | v >> 2`, the renderer's
own conversion); a map entry is a tile, its flips and its palette.

**Field** (`Field`): its tiles with the panel palettes; the panel types it draws, by its game's names (its rules'
`panels.numbers`: `"normal"`, `"cracked"`, ..., `"magnet"`, `"lava"`, `"sea"`), in the order of their blocks, the
game's (EXE6's 13, EXE5's 11, EXE4's 12; a panel type the field doesn't draw is drawn tinted:
docs/design/rules-in-luau.md §7.4); six blocks for each (5x3 map entries, by owner and row); the two front edges;
one highlight or two (EXE6 has two; EXE5 draws one block for both); the cycling panel palettes.

**Backgrounds** (`Background`, by the battle settings' background id): tiles and map; whether it has its own
palette; its scroll, in 1/16 pixel a frame, or a scroll that speeds up (`Speeding`: its speed's step a frame and its
top, from the battle's first frame, EXE5's background 0x1B, else from the background's load, EXE4's 0x17); its
animations: tile frames, palette frames, and palette shifts (palettes shown lightened or darkened by a color a
frame: EXE4's background 0x09).

**HUD** (`Hud`): the HP box and the custom gauge with their palettes; the 8x16 font and what each glyph draws
(`font_chars`: a chip's name is spelled with them, the game's marks as characters, Ⓐ for the A button, U+E002 for
the stacked EX: text-rendering.md §10.5); the digits; each chip's icon under its chip's key; the mugshots and the
link navis' faces; the count boxes; the banners' layouts; the HUD's text lines as glyphs ("TIME UP!", the turn
timer's seconds, "COUNTER HIT!"); "Cstmzing..."; "PAUSE"; the warning marker; the dialogue font, each glyph's advance
and what it draws (the 8x16 font's characters, then the two-byte codes' kana and kanji: `compat/text.toml`'s
`dialogue_glyphs`); and the chatbox's box (its maps by kind and opening step) and key-wait arrow. The chatbox's
portraits are sprites (category 0x20, `mugshotSpritePtrs`): the speaker's faces are their animations. Each US ROM has
a black placeholder in place of the other game's link navis' portraits; the extraction takes each from the ROM that
has it.

**Custom screen** (`CustomScreen`): the window's tiles, maps (15x20, without and with the form list's tab) and
patches, and its palettes by the chip under the cursor's class (standard, Mega, Giga, dark); the form list window's
maps (EXE6's Cross window); each chip's picture (7x6 tiles, with its palette) under its chip's key; the element
icons with each element's colors (below, "Frames as shapes": their shared frame taken apart); each navi's emblem
under the navi's key, whose palette is the cursor's and the Regular chip's frame's too while that navi's console
has the screen up; the codes, digits and slot codes; the cursors; the Program Advance animation's name colors; the
buttons (below). A navi's emblem is its own on any console: the extraction takes each from the ROM that has it
(EXE6's ROMs each have MegaMan's, ProtoMan's and their own version's five link navis'; EXE5's MegaMan's and their
own team's six), under the key the game's compat gives the navi's number. The original has no picture for the other
version's navis and shows its own counterpart's in the navi's colors (docs/frontend.md §5).

**Buttons** (`CustomScreen::buttons`): a button of the rules on the custom screen is drawn by the game's look of the
name its content registers it under (EXE6's `redeal`, `scrap` and `beast_out`; EXE5's `soul`, `redeal` and
`arm_change`); the frontend has no look of its own. A button's look (`ButtonPictures`) has its tiles among the
slots' (a cell's tiles: a slot's 2x3 or the special slot's own; a set its cells', set after set), which set a state
shows (one a state; the second for unavailable and picked; the second for unavailable alone), the set its slot shows
while the button isn't there (EXE6's Beast Out's fourth; none: the window's fill), the cursor over it (its place and
each frame's four corners), its picture in the chip window with its palettes, its own first, whether the chip window
shows its uses left (EXE5's Shuffle) and where the chip it holds is drawn (EXE5's Arm Change). EXE5's `soul` has the
souls' 2x2 icons besides, with their sprite palette, which the soul choice flies onto the column (the game versions
whose consoles fly the icon in a palette of their own: Team Colonel's). A version's own look of a button is its
version's (EXE6's `beast_out`), and a language's own tiles or picture of one its language's (EXE5's soul button,
"uni son" for "UNITE"; the re-deal's picture).

**Letters** (`nettai_assets::letters`: `HudLetters`, `CustomLetters`, `PictureLetters`, and each language's): how
the pictures' letters and digits look, which the enhanced graphics draw their words and numbers in fonts by
(text-rendering.md §12.12). The extraction reads them off the pictures (`nettai_extract::letters::read`); a frontend
reads no picture. Values are palette indices and pixels of the picture: a number set's face box (x, y, width, height
in its 8x16 cells) and paint (its fill down the rows, a rim or a shadow); a word's letters (the fill, the outline and
whether it is heavier right and down, else even, the capitals' top and height, the letters' span of columns); a
banner's (by banner) with its letters' columns either side of its number (one digit, two digits; its bar is a
drawing, "Frames as shapes" below); the gauge's word's cells and how it stands in them; the emotion window's counts'
digit boxes; the chip codes (a glyph's face box and paint, by code); the window's labels' letters and rows; the chip
window's lettered pictures' lines (OK's, OK-picked's, the other slot's, each button's, each version's); the form
list names' face (by look) and the box all of them are centered in.

**Frames as shapes** (`nettai_assets::shapes`: `HudShapes`, `CustomShapes`, a banner's bar; text-rendering.md §13):
the HUD's and the custom screen's frames as filled shapes, which the enhanced graphics draw at the output's
resolution in place of their pictures (a banner with no bar, a logo: the whole picture). Each is an SVG file
(`shapes::from_svg` reads it), its `viewBox` the picture's size in frame pixels, read and drawn by usvg and resvg
(the version Slint builds into the app), so any SVG a drawing needs works: shapes, paths with curves, strokes, groups,
transforms, `use`, clip paths. Its colors are the palette's at the time it is drawn: each shape (or a group round it)
names its fill's palette index as `data-color` and its stroke's as `data-stroke`, and, where the picture is laid in
several palettes (a window map's), the row as `data-palette` on it or round it; a gradient's stops name theirs as
`data-stop-color` (the custom gauge's fill), and a shape it paints names none; every other shape outside `defs` and
clip paths names its paint so (`fill="none"` where it has none). The file's own `fill` and `stroke` are those colors
in the picture's first palette, for viewing only. A `text` element with `data-words` is words written on the
drawing, drawn by the text layer, not as the file's: the string's key in the game's strings (`data-words`), the box
its capitals fill (`data-box`: x y width height; lines one under another share its height), its face's color
(`data-color`), its outline's and shadow's (`data-rim`, `data-shadow`), the outline's width (`data-rim-width`,
pixels; 1 if not given), its slant (`data-slant`) and font (`data-font`); its position, size and contents are for
viewing. What SVG can't say is the drawing's place in the model: which cells of a map a drawing stands in for (a
window map's, the gauge's frame, the chatbox's boxes: its parts, each a drawing and its cells, the first drawing
whose pixels a cell has), and which of a set each is (the gauge's cells and full frames, the cursors, the Regular
chip's frame, each button's sets by name, the chip window's pictures by name, a language's own).

The emotion window's faces are kept without the ring round them (`MugshotFrame`: the ring as a picture, each face's
ring color; none where a face covers its ring and keeps it). The element icons are taken apart at extraction (the
user: "make the extract element icons transparent and borderless"): the frame they share is a picture (each pixel
the index most of the icons share, the rest of it the ground, its most common index) and a drawing (16x16: the ring
and the ground in it, nothing round the ring, checked against the picture without the ground there); each icon is
its pixels that aren't the frame's, clear elsewhere: its symbol, and where an icon draws a pixel of the frame its own
way (EXE6's and EXE5's fourth icon's inner corners, their no-element icon's top right), that pixel, so the original
graphics draw the frame with the icon over it as the game's. The enhanced graphics and the app draw the frame's
drawing with only the symbol's pixels inside the frame's ring over it (its ground there, not joined to its edge by
ground). (EXE5's two icons no chip shows, the Program Advance's and the special family's, leave pixels clear where
the frame has them: over the frame, they aren't the game's.)

The drawings are drawn by hand, not read off the ROM: they live in each game's content (`<game>/extract/shapes/`, a
game's file or a `.same` naming one exelib's `extract/shapes/` holds), and the extraction lays each picture as the
screen lays it and checks the drawing against it (`nettai_extract::shapes`: its size, its colors among the
picture's, its share of the picture's pixels: noted, and called out under 85%). They are the one exception to "the
extractor's inputs are ROMs only": what they say is how the art is meant (a round holder, a rounded corner), which no
ROM holds; they are the recipe's, like its code, the same for every set of ROMs it reads, and a picture with no
drawing is drawn as it is.

**Lettered pictures** are kept once: as the picture with every language's letters taken off (its **base**), and a
layer of its letters over it: each pixel where the picture differs from the base. The letters are its words and their
ink (the user: "it doesn't seem like it's a pure text extraction"), with the few pixels where a panel picture's own
panel departs from the shared one (a body a column wider, a letter's outline drawn in the panel's colors, EXE4's
taller Japanese panel; the user: "fold them into the letters layer"). Another picture of the same base (another
language's, another panel picture's) has its letters as a layer of its own, in that picture's palette. The
extraction settles each picture so (`nettai_extract::letters::settle`): the original mode draws its letters over the
base, and the enhanced mode shows the base, with its words drawn in a font.

The chip window's pictures are bit planes, their palettes picking the plane, so a panel picture's tiles hold the
other pictures' words in index values its palette shows in the panel's own colors. Those pixels take the shared
panel's index (the user: "why do the -own images look like that? can you not remove them?"): extraction checks every
palette the picture is drawn in (its own; a button's others) shows the two alike, and says how many it made so
(EXE5's 2,638, EXE6's 2,647, EXE4's none) and, as a problem, any some palette shows otherwise. So a picture shows the
ROM's colors in every palette it is drawn in. Kept so: the window's tiles (its labels), the codes and the slots'
codes, FINAL TURN, the gauge (CUSTOM, L or R), the counts and the chip window's lettered pictures. A picture with no
letters in any language (a button's art: EXE6's TRASH CHUTE and DUST SHOOT) is each language's own, drawn as it is.

The chip window's **panels** with words (OK's, OK-picked's, the other slot's, the buttons' that are panels, as the
re-deal's; in every language) share one base (the user: "ok/other/redeal actually share the same background"),
recovered from the pictures themselves: each pixel the color most of them show there, of those whose letters are no
nearer than a pixel's gap (an outline in the panel's colors lies beside them); where as many show one color as
another, of all whose letters miss it, then its left neighbor's (EXE4's right edge had taken the Japanese OK's own
taller body from one picture against one); where every picture's letters cover it (8 pixels in EXE5's and EXE6's,
none in EXE4's), its row's nearest such pixel's. A picture's letters are wherever its color differs from the panel's,
its outline and any of its panel that differs with them (EXE5's and EXE6's CHIP DATA TRANSMISSION and BLOCKING!
widen the panel's body a column each side; EXE4's Japanese OK pictures are drawn on a taller one), so the enhanced
mode shows the one panel under every picture's words. The extraction notes how many pixels the panels differ at, by
picture.

The **form list's names** (EXE6's Cross window) are layers: the bar every name of both versions lies on (one name's
9x2 tiles, in every look alike, the yellow bar with its orange streaks, and the frame; the user: "falzar and gregar
should have the same bar"); each name's portrait over it (every look's names, the portrait's own pixels and none
elsewhere, with the names' palettes by look); and the letters over all (another language's its own), with the few
pixels where a name shows the bar otherwise among them (a streak ending a pixel sooner or later, as the English
SPOUT's does where its Japanese art's doesn't: Falzar's English 2, Gregar's English 1 and Japanese 6, each in both
looks; the user chose "fold into letters"). The original mode lays them in that order; the enhanced mode draws a name
in a font over its portrait and the shared bar. The bar is what the names show beside their portraits and letters
(each pixel the value most of them show, every version, look and language; where as many show one value as another,
the streak's: its left neighbor's); a portrait is the pixels joined to the name's left edge through pixels of no
color of the bar's and no letter's; what shows through a letter in the bar's colors is the bar's; a language's
letters are all its pixels that differ from its portraits over the bar: its letters and their outline (one of gray
ヒート's is the ROM's index 1) and those few. The bar's pixels no name shows (behind every portrait, or under letters
or portraits in every name) are filled from their rows; the extraction notes how many.

A window **label**'s letters (BATTLE CHIP, CHIP SELECT, the CROSSSELECT tab and header, B:EXIT) are taken off its
strip as every pixel, from a pixel before its letters' first column to one past their last, that isn't the row's
ground (the color most of the row's other pixels there are): a letter's lighter dot (EXE6's CHIP SELECT's P) goes
with it.

**Another game's HUD and custom screen** (EXE5's, docs/design/exe5-map.md §11; EXE4's, exe4-map.md §14) are the same
model, with the parts its game lays out otherwise in its layouts:

- The HUD: its tile numbers count from the HUD layer's and the gauge's first tiles (EXE5's 0x180 and 0x202, EXE6's
  0x1A0 and 0x222). A face may bring its own 2x2 box beside it (`mugshot_boxes`: EXE5's MegaMan, dark MegaMan and Team
  Colonel's faces), by mugshot number; a game may have no box without a count (EXE5's souls' faces show their turns
  left, by count). Its `layout` (EXE4's) says "PAUSE"'s place, the HP numbers' priority and the HUD layer's (EXE4's
  BG3 is 0, EXE6's 1), a message's place, the damage judge's timing, the chatbox's text and arrows, and the telops'
  own look, with their tiles where the game loads the telops' sprite tiles among the banners' (VRAM tile numbers: a
  banner's first, each place's first name tile, the '×' and '2' pieces'; whether "PAUSE" loads the banners' palette
  into the one they share): EXE4's banners and telops show each other's loads.
- The custom screen: its `layout` (every game's its own) puts the window's parts at the game's tile numbers (the
  chip's name, picture, code, element and digits, the slots, the column's icons and cells, the turn limit, the name
  bar, the Cross names), says which tile a hidden slot is filled with, and where the cursor stands over OK, with its
  corners. Its buttons are the game's own (above: EXE5's special slot is the 3x2 soul button, with a palette for Soul
  Unison and one for Chaos Unison; EXE6's the 4x2 Beast Out). Its labels are the words the window's tiles draw
  (BATTLE CHIP, CHIP SELECT, CROSS SELECT, B:EXIT), each its name (the content's strings say its words: the game's
  `lettering` table, `window.NAME`), its tiles (the first and the last, as the maps number them), the rows of them its
  letters are on and the colors of its letters: the enhanced graphics take the letters off and draw the words in a
  font (text-rendering.md §12).
- EXE4's custom screen draws some things otherwise: a blank code or damage cell's color (7 where EXE6's and EXE5's
  are 8), the "FINAL TURN" block's palette (14 where EXE6's and EXE5's are 9), the empty icon's palette in a slot or
  cell without a chip (9); the element icon drawn as a 16x16 sprite at a place and in a palette of its own, not as
  window tiles in palette 11 with the elements' colors; the cursor's and the Regular chip's frame's own sprite
  palette, not a navi emblem's; the orb over the picked column drawn on the window's map (its tiles, its frames' map
  entries, and the steps of its turn after a pick); and a button's own place on the window's map (the UNITE button's
  cells, not among the slots').
- A version's own chip (EXE5's and EXE6's version Giga chips, EXE5's Phoenix and DethPhnx), which the other version's
  ROM draws as its counterpart, has its icon and picture once, under the chip's key, from its own version's ROM, the
  picture with that version: the frontend shows it on either console, and a console of the other version's is a
  known difference there (docs/frontend.md §5).

**Four ROMs, and what differs by version and region.** EXE6's assets are extracted from the two US ROMs and the two
Japanese ones (all four, in any order, identified by game code: BR6E, BR5E, BR6J, BR5J), by EXE6's recipe
(content/exe6/extract/): what the four have alike from the Falzar US ROM, what a version's ROMs alone have right or of
their own from a ROM of that version (the recipe's `versions.luau`), each release's words from a ROM of that release,
what the US release cut from a Japanese ROM (`jp.luau`), and the US ROM's wherever both releases have their own. Each
block is read through the game's own pointer to it (a routine's literal, a load list's entry: the extractor's
`rom::Place`), which each ROM has where its code is:

- **What differs by version** is held as `Versioned` (the base's, its version, and each other version's own): a
  console shows its version's. On the custom screen these are the Beast Out button, the version's Beast's (four sets
  of 4x2: selectable, unavailable, battle mode 1's, the hidden slot's), with its picture in the chip window (which the
  BeastOut chip shows too), and the form list window's (the Cross window's) names, 9x2 tiles each, the five on the
  cursor's row then on the others', with background palette 10 for the Cross under the cursor. A Cross's name is the
  Cross's own game's on either console.
- **Gregar's own faces**: the Falzar ROM has none for Gregar's Crosses, its Beast and its link navis (its tables show
  the Falzar counterpart's). The assets have the Gregar ROM's as faces of their own (numbers 0x17 to 0x28 after the
  Falzar ROM's emotion pictures, 0x86 to 0x8A after its link navis'), and Gregar's forms and navis name them: every
  form shows its true face on either console.
- **Five chips' pictures and icons** the Falzar ROM has wrong (Bass, BigHook, DeltaRay, ColForce, BugRSwrd: the
  other five Giga chips' copied in): the assets have the Gregar ROM's, on either console.
- **The Cybeasts' sprites**: a ROM has its own version's (Gregar's in slot 08-14, `gregar-cybeast`; Falzar's in
  08-15, `falzar-cybeast`) and a placeholder archive in the other's slot. The assets have each from a ROM of its
  version.
- **What the US release cut** (docs/engine/jp-differences.md §4.4, §4.5): six sprites the US ROMs fill with a
  placeholder archive (`count`, `django`, `otenko`, `falzar-summon`, `gregar-summon`, `blocking-banner`), from a
  Japanese ROM (every other sheet is a US ROM's: the flames HeatMan wears differ in one animation between the
  releases), and the pictures of fifteen chips the US ROMs give a placeholder picture (GunDelEX, Otenko, Count
  (HackJack) ×3, Django ×3, DblBeast, Gregar, Falzar, and BatCan1 to BatCan4, which the US release kept), from a
  Japanese ROM, Gregar's from the Japanese Gregar ROM and Falzar's from the Japanese Falzar one (a Japanese console
  shows its own beast in both chips; the assets have each chip's own). Every other chip's icon and picture are its
  version's US ROM's: the Japanese ROMs' differ in one pixel of the Falzar ROMs' BassAnly icon and two of the Gregar
  ROMs' Beast Out picture (docs/engine/jp-differences.md §4.4). The assets keep no region: one picture of each thing,
  the US games' where both releases have their own and the Japanese games' where the US release has none, and what a
  console of the other region shows there is the verification's to know (docs/frontend.md §5). Three of the
  pictures' palettes are EWRAM in the original, which a link gift (an e-Reader card's) fills and the save keeps:
  DblBeast's, the card's, is also orphaned in the Japanese ROMs, and the extraction has it; the Gregar and Falzar
  chips' are in no ROM, and their pictures have a black one, which their definitions' `art_palette` replaces when
  drawn.

**Languages.** The words a battle's pictures and fonts show are the graphics' own language's (`Hud::language`: "en",
the US release's) and the other language's they have (`Hud::languages`, `CustomScreen::languages`: "ja", from the
Japanese ROMs). Every game's assets have English's and Japanese's: they are extracted from a complete collection.
The other language's lettering is the Japanese 8x16 font and what each glyph draws, the dialogue font (its advances
and characters), the HUD's text lines in its glyphs, the banners whose words differ (each with its place: a longer
name starts further left), "Cstmzing..." as wide as its words (カスタム中…, seven tiles where the US's are eight), the
gauge's "L or R" and, where the language's games place the chatbox's text otherwise (`layout.chatbox_text`), its
place (EXE4's Japanese games', twelve pixels left of the US games' and one up); on the custom screen the chip
window's pictures for OK, the re-deal and scrap ("chip data transmission" in Japanese), the form list window's names
by version and a named button's tiles where they say something (EXE5's soul button, "uni son" for "UNITE"). The
fonts' characters come from compat/text.toml's `[jp]`, the Japanese ROMs' encoding. Everything else a battle shows is
the same pictures in a game's four ROMs. A frontend in that language swaps them in (`Bundle::for_language`); one set
to a language the graphics lack (Chinese) draws their own pictures with words and the content's strings in its own
language (docs/frontend.md, "Languages"). The content's display text (names, descriptions, messages) is no asset: it
is the content's strings (`locales/<lang>.toml`, docs/design/text-rendering.md §10).

## 5. Sound

A game's sound is one `SoundBank` (the `m4a` crate's): the song table, every song as its tracks' commands, the
voicegroups (drum kits and split groups too), the key maps, the PSG waves, the Direct Sound samples (with their
exact rates and loop starts), the mixer's settings and the music players. The `m4a` driver plays it as the game's own
sound driver does: every song renders sample for sample as the original's.

A game's versions may have songs of their own (EXE5's Team Colonel's): what a version's ROM plays otherwise than the
base version's at the same number, or has alone. The extraction brings each into the base's bank as that version's
own, its voicegroups, samples, waves and key maps matched to the bank's or added to it (`nettai_extract::SongVersions`,
made by the extraction's `own_songs`); two songs play alike when their notes play alike, not when their bytes are
equal. No frontend
plays a version's own songs yet: a console plays the bank's.

## 6. Checks

- **As a game loads**: its assets against its content (`nettai_content::load::asset_numbers`): every asset the
  content names, each sprite the content defines with as many animations, each of as many frames. A game whose assets
  aren't so doesn't load.
- **`nettai-content check <game>`**: the same, the content's sprite timing against the ROMs'
  (`Extraction::timing_differences`), and the strings' backgrounds against the assets'.
- **`nettai-tool --match FILE --audit-content`**: every lookup the drawing code and the audio make for everything the
  content defines, in every language (docs/tools.md).
- **Frames and sound**: the verification workspace compares frames drawn from the extracted assets with mGBA's, and
  the battle's sound with the original's (docs/frontend.md).
