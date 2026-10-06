# The frontend (`nettai-render`, `nettai-frontend` and `nettai-demo`)

A desktop program, `nettai-demo`, that runs a battle through the native engine and draws it the
way the original does, in its 240x160 frame scaled up by the largest whole
factor the window has room for. It draws from engine state: the field's
panels, each object's sprite, animation frame and look, HP, the custom
gauge, banners and the flow's screen fades. Nothing emulates the GBA's
video hardware; the frontend composes tile layers and sprite parts itself
with the original's ordering and blending rules. The strings that come
from content or vary (chip names, the telop, the chatbox, the HUD's lines)
are drawn by default with a vector font at the window's resolution over
the scaled frame (§3, "Text"); `--text original` draws them in the game's
own fonts, exactly as the original does.

It replays a golden trace (the recorded inputs of a real match) or is
played live from the keyboard, and can render chosen frames to PNG.

Three crates make it up: the drawing, the playing, and the program around
them. The commands in this document are the program's (`nettai-demo`).

- **nettai-render** draws a battle into frames, and nothing else: it
  composes the layers (`compose`), draws the stage and the field
  (`stage`), the objects (`objects`), the HUD with its banners and telops
  (`hud`), the custom screen (`custom`) and the chatbox (`chatbox`); the
  strings in the game's fonts or as the text layer's items (`fonts`,
  `textlayer`, `strings`) and the vector font that draws those
  (`vfont`); each pack's graphics (`packs`); the lookups a frame makes of
  the packs and the content (`lookups`), each noted and checked once a run
  (`audit`); `Renderer`, which draws a frame, and `present`, which scales
  one to an output of any size, or writes it as a PNG; and a chip's
  pictures on their own (`pictures`, which the editor shows). It has no
  window, sound, network, netplay or command line.
- **nettai-frontend** plays a battle for a host to show, as a library: it
  has no window, no audio device and no command line, and nothing in it
  prints or exits (§7). It loads a game (`game`: the packs found, the
  content, the graphics and strings in a language, the text's font, the
  sound, each step a `Result`); its player (`player`) owns the session
  (`session`), the renderer and the audio, and gives a host the picture and
  the sound as it is driven; what drives a session is a driver (`driver`:
  live play of a set from the GBA button mask; `netplay`: another player,
  over a channel the host provides: it has no socket). It draws the
  battle's picture and nothing over it: it has no text of its own.
- **nettai-demo** is the desktop program, a host of that library: the
  window and the keys, a loop over the player (`app`); the command line
  (`main`); the audio device the player's samples go to, and the audio's
  own lookups (`sound_lookups`); headless output (`headless`) and the audits
  (`content_audit`, `headless::audit_traces`); the replay of the
  original's recordings (`trace`, the one place that depends on the compat
  crates); and netplay's transport, the UDP socket and the handshake
  (`net`).

## 1. The content pack

The battle content the engine runs on is this repository's content/ (its
packs: the game packs content/exe6 and content/exe5 and the support pack
content/exelib, docs/design/content-model-v2.md §4.0; `--content <dir>` or
`$NETTAI_CONTENT` for another content directory). What the
frontend shows and plays comes from a content pack made from your own ROMs
(the US Falzar and Gregar, `MEGAMAN6_FXXBR6E` and `MEGAMAN6_GXXBR5E`, and the
Japanese Falzar and Gregar, `ROCKEXE6_RXXBR6J` and `ROCKEXE6_GXXBR5J`, which
have what the US release cut), never checked in: the graphics and the
sound, in open formats (indexed PNG, JSON, Tiled maps, MIDI, TOML, WAV; see
`docs/design/content-pack.md` and `docs/design/asset-formats.md`), by the
names the content gives them. Extract it once:

    cargo run -p nettai-extract -- exe6 data/exe6 <falzar-us> <gregar-us> <falzar-jp> <gregar-jp>

ROMs can be in any order; any missing source is replaced with generated placeholders.
The output directory must be new or empty. (`data/` is gitignored.) **You play one game**, EXE6 or EXE5: a match
is of one game (§6), which a match file names (`game = "exe6"`) and a
trace states (its setup line's `"game"`: one that states none is refused;
nothing takes a recording for a game it doesn't name). There is no default
game: the frontend requires a match file or a trace. The battle is that
game's content, drawn and heard from its pack: there is no mixing of games,
no other game's chip, navi or field art. The frontend (and the editor)
finds at start-up
**every pack in the packs directory**, `$NETTAI_PACKS`, else
`data`: each folder with a pack manifest, by the game the manifest
says (`nettai_content::pack::find`); `nettai-extract exe5` writes EXE5's into
`data/exe5` beside it. `--pack <dir>` names a pack elsewhere, in
place of the found one of its game, and can be given again for another
game's. Two packs of one game in the directory are an error. A pack is of
the format's one version (each file says it): an older one is refused, with
a note to extract it again. Each pack loads straight from its files: its asset
index into the engine's `Content`, the graphics through nettai-content's
importer, and, when a window opens, the sound. `NETTAI_LOAD_TIMES=1` prints
how long each part took. Each sprite, banner, mugshot, background and sound
comes from its own pack, and the HUD, the custom screen and the field from
the match's game's (see "Field and background" below).

The content goes with the packs (`nettai_content::pack::load_game`;
docs/design/content-model-v2.md §4.0): `--content <dir>` names the content
directory (so does `$NETTAI_CONTENT`); without it, this repository's
content/. A match plays one game: the frontend loads that game's pack (its
top module, init.luau, and what it requires, with the support packs it
depends on, exelib) and its asset pack, nothing else; `nettai_content::pack::games`
lists the games it can offer, each with its asset pack or the command that
writes one. A game's chips without a use yet (a port's unwritten chips)
aren't required by its init.luau, so they don't load. Every loader does the same (the
editor, the tools' `load_battle`, the EXE5 replays, the static audit), and
the content then is what loaded: its hash, which netplay's handshake
compares, covers exactly that. A game the content must load whose pack
isn't found is an error naming the extract command. (Netplay's handshake
compares the content, the packs' asset names among it: two players play
with the same packs, or give `--content` and `--pack` alike.)

The graphics load into the types of the `nettai-assets` crate, decoded
(tiles as palette indices, colors as BGR555):

- **Sprites**: every battle sprite (categories 0x00..=0x14 of
  `SpritePointersList`), per animation frame its tileset, palette set and
  part list (the frame's OAM composition: tile, offset, size, flips,
  palette offset). The first part of every frame is the shadow. A palette
  set is every palette of the sprite's palette block (MegaMan's has 43:
  an object picks one by number, and his Crosses pick past the first 16).
- **Field**: the panel tiles (`dword_86DDBA0`), panel palettes and their
  cycling animations (`off_800C1DC`, with the start timers `sub_800BF88`
  sets), the 5x3 panel blocks by type, owner and row (`byte_86DFA98`),
  highlight blocks and front edges.
- **Backgrounds** by id (`off_8080F98`): tiles, tile map, palette, scroll
  speed (`off_8080E34`) and tile/palette animations (`off_8081220`).
- **HUD**: the HP box and its digits, gauge tiles and frame, the 8x16 font
  with what each glyph draws, chip icons by chip, the HP digits shown under
  objects, the emotion window's faces and count boxes, the link navis'
  faces (each mugshot under its name), the HUD's text lines, banner layouts
  and glyphs, "Cstmzing...", "PAUSE", the warning marker's arrow.
- **The custom screen** (`graphics/custom`): the window's tiles, maps and
  patches, each chip's picture (`chip-art/<chip>.png`; for a chip whose
  palette no ROM holds, the Gregar and Falzar chips, its definition's
  `art_palette` colors it) and the buttons',
  chip codes, element icons and their colors, damage digits, the slots'
  codes and buttons, the cursor, the navis' emblems, the Regular chip's
  frame.
- **The chatbox** (in `graphics/hud`, and its portraits as sprites): the
  dialogue font with its advances, the box's tiles and maps, the key-wait
  arrow, the text's palette; each run message's speaker's portrait, the
  true face from whichever ROM has it.

What the HUD shows of the content comes from the content: a chip's name is
its definition's, spelled with the font's glyphs (`Hud::glyphs`), or in the
font text mode drawn with the bundled font (§3, "Text"); its icon
is the pack's image under the chip's key; whether its damage shows is its
definition's flag. The emotion window shows the face the navi's form names
for its emotion (`mugshot`, `FormData::mugshot`), or a link navi's own
(`NaviData::mugshot`); mugshot numbers from `nettai_assets::NAVI_MUGSHOTS`
are the link navis' faces, with their Full Synchro palettes.

### The battle backgrounds, by area

A background is named for the area whose maps draw it, in the game's own words. The link is the ROM's. A battle
draws the background its settings state (their byte 4), or with 0xFF there its map's, which the game looks up by
map group and number; the same number is the net map's own backdrop.

| | The routine | Its table (pointers by map group from 0x80, each a byte array by map number) | A real-world map's |
|---|---|---|---|
| EXE6 | `sub_8081308` | 0x0808139C (`pt_808139C`) | 0x07 |
| EXE5 | 0x0808CA78 | 0x0808CAAC | 0x06 |

A map's name is the one the menu shows: EXE6's `sub_811F290` and EXE5's 0x08128B50 turn the group and number
into a name's number in the names' text archive (EXE6's at 0x086CB360, EXE5's at 0x086E2B6C). A background's name
is that name in lower case with hyphens, a possessive's apostrophe dropped (`lans-hp`), and a word the menu's
twelve characters cut short spelled as the game's dialogue spells it (`robot-control-comp` for RobCtrlComp).

- One background of an area's numbered parts has the area's name once (`central-area`).
- One background that another place draws too has its own area's name, the other place in the comment beside it
  in compat/assets.toml (`robot-control-comp` is PavilonComp4's too; EXE5's `undernet` is NebulaArea6's too).
- The two that dozens of unrelated comps share have no one area. `comp` is the default the game's table falls
  back to: the comps without a backdrop of their own, every real-world map, and what a link battle's settings
  state. `comp-alt` is the same picture in its second palette, which some comps draw instead. In EXE6 those are
  the six comps of the second of the two maps the small comps are built on (the 26 others are on the first:
  `initMapTilesState_803037c`'s descriptors and `decompressCoordEventData_8030aa4`'s collision data, one of each
  per set). In EXE5 every such comp draws the same map tiles, and each of the two collision maps (the
  SquirrelCmps', the others') holds comps of both: nothing else in the data sets the eleven apart.
- Two backgrounds of one area are told apart by a part's number (`end-area2`) or a story state
  (`mr-weather-comp-storm`).
- EXE5 has two that no menu name gives: the VisionBursts' maps, which the menu leaves blank and their own
  dialogue names (`visionburst`), and one no map draws, named for the battles whose settings state it
  (`nebulagray`).

"Named from" says which of these a name is, in bold where it isn't a menu name alone: those carry a judgment
(`comp`, `comp-alt`, `visionburst` and `nebulagray` the most).
"Was" is the look-name it had before.

The tables are the verification workspace's (tools/backgrounds): `areas.py` reads the areas from the ROMs,
`names.tsv` holds the names, and `rename.py` puts a changed name into the content, the generators and these
tables.

**EXE6** has 21. A link battle draws one of `sub_81209DC`'s table (`byte_8120A20`), which its `link_pick` rules
state as `backgrounds`.

<!-- backgrounds:exe6 -->
| Number | Name | The maps that draw it (the menu's names; map group:numbers) | Named from | Also | Was (its picture) |
|---|---|---|---|---|---|
| 0x00 | `lans-hp` | Lan's HP (0x88:0) | the menu | a link battle | `honeycomb` |
| 0x01 | `acdc-hp` | ACDC HP (0x88:1) | the menu | a link battle (2 of 21) | `statues` |
| 0x03 | `aquarium-hp` | Aquarium HP (0x88:3) | the menu | a link battle | `seals` |
| 0x04 | `sky-hp` | Sky HP (0x88:6) | the menu | a link battle | `clouds` |
| 0x05 | `green-hp` | Green HP (0x88:5) | the menu | a link battle | `sprouts` |
| 0x06 | `robot-control-comp` | RobCtrlComp1, 2 (0x80:0, 1); PavilonComp4 (0x85:3) | **the menu, spelled out as the dialogue has it** | a link battle | `calendar-checkers` |
| 0x07 | `comp` | Stg6Dungeon1, 2, 3 (0x84:0, 1, 2); Extra (0x88:2, 4, 7, 8); RoboDogComp (0x8c:0); Class6-1Comp (0x8c:2); Class6-2Comp (0x8c:3); Class1-1Comp (0x8c:4); Class1-2Comp (0x8c:5); BathroomComp (0x8c:6); ElevatorComp (0x8c:7); FshStkSpComp (0x8c:8); SecurCamComp (0x8c:9); Fan Comp (0x8c:11); AirCndtrComp (0x8c:12); Heater Comp (0x8c:13); Shower Comp (0x8c:14); PunshChrComp (0x8d:2); WaterMchComp (0x8d:3); Symbol Comp (0x8d:4); Monitor Comp (0x8d:5); PopcrnShpCmp (0x8d:6); TeachrRmComp (0x8d:7); ObservtnComp (0x8d:9); OxygnTnkComp (0x8d:10); PrcplOfcComp (0x8d:11); Mascot Comp (0x8d:12); StfToySpComp (0x8d:13); DogHouseComp (0x8d:14); GuidPanlComp (0x8d:15); the default: every real-world map | **no one area: the default, which the comps without their own draw** | 192 battle settings state it; a link battle | `calendar-mint` |
| 0x08 | `comp-alt` | Extra (0x88:9); Lab'sComp1 (0x8c:1); BookComp (0x8c:10); HeliportComp (0x8c:15); Lab'sComp2 (0x8d:0); VendngMcComp (0x8d:1); Pipe Comp (0x8d:8) | **no one area: the default's picture in its second palette** | 1 battle settings state it (0x080aee70); a link battle | `calendar-lavender` |
| 0x09 | `central-area` | CentralArea1, 2, 3 (0x90:0, 1, 2) | the menu | a link battle | `calendar-navy` |
| 0x0a | `aquarium-comp` | AquarumComp1, 2, 3 (0x81:0, 1, 2); PavilonComp1 (0x85:0) | **the menu, spelled out as the dialogue has it** | a link battle | `calendar-blue` |
| 0x0b | `seaside-area` | SeasideArea1, 2, 3 (0x91:0, 1, 2) | the menu | a link battle | `calendar-cyan` |
| 0x0c | `judgetree-comp` | JdgTreeComp1, 2, 3 (0x82:0, 1, 2); PavilonComp2 (0x85:1) | **the menu, spelled out as the dialogue has it** | a link battle | `trees` |
| 0x0d | `green-area` | Green Area1, 2 (0x92:0, 1) | the menu | a link battle | `calendar-green` |
| 0x0e | `sky-area` | Sky Area1, 2 (0x94:0, 1) | the menu | a link battle | `calendar-bright-blue` |
| 0x0f | `undernet` | Undernet1, 2, 3 (0x95:0, 2, 3); UndernetZero (0x95:1) | the menu | a link battle | `code` |
| 0x10 | `mr-weather-comp` | MrWeathrCmp1, 2, 3 (0x83:0, 1, 2); PavilonComp3 (0x85:2) | **the menu, spelled out as the dialogue has it** | a link battle | `globes` |
| 0x11 | `underground` | Underground1, 2 (0x93:0, 1); unnamed maps (0x93:2) | the menu | a link battle (2 of 21) | `code-2` |
| 0x12 | `copybot-comp` | CopyBotComp (0x85:4) | the menu |  | `swirls` |
| 0x13 | `acdc-area` | ACDC Area (0x94:2) | the menu | a link battle (2 of 21) | `calendar-purple` |
| 0x14 | `graveyard` | Graveyard1 (0x96:0); Graveyard (0x96:1); ImmortalArea (0x96:2) | the menu |  | `storm-clouds` |
| 0x15 | `mr-weather-comp-storm` | MrWeathrCmp1 (0x83:0) until event flag 0x0be1 is set; MrWeathrCmp2 (0x83:1) until event flag 0x0be2 is set; MrWeathrCmp3 (0x83:2) until event flag 0x0be3 is set; PavilonComp3 (0x85:2) until event flag 0x0fd2 is set | **the menu, and the story state** |  | `globes-2` |
<!-- /backgrounds:exe6 -->

Mr.Weather's comps are the one place the story changes it (`sub_8081308`'s `word_8081368`: a map and an event
flag): each draws 0x15 until its typhoon virus is beaten, then 0x10.

**EXE5** has 29. A link battle draws one of 0x00 to 0x1A (0x08129F2C's table at 0x08129F6C, its `link_pick`
rules' `backgrounds`).

<!-- backgrounds:exe5 -->
| Number | Name | The maps that draw it (the menu's names; map group:numbers) | Named from | Also | Was (its picture) |
|---|---|---|---|---|---|
| 0x00 | `main-comp` | MainComp1, 2 (0x80:0, 1) | the menu | a link battle | `binary` (green 01s and dotted lines on black) |
| 0x01 | `drill-comp` | DrillComp1, 2, 3, 4 (0x81:0, 1, 2, 3) | the menu | a link battle | `drills` (drills boring through layers of earth) |
| 0x02 | `lans-hp` | Lan's HP (0x88:0) | the menu | a link battle | `soccer-balls` (soccer balls) |
| 0x03 | `mayls-hp` | Mayl's HP (0x88:1) | the menu | a link battle | `rabbits-and-apples` (rabbits and apples with hearts, on pink) |
| 0x04 | `dexs-hp` | Dex's HP (0x88:2) | the menu | a link battle | `faces-and-crosses` (faces and crosses) |
| 0x05 | `yais-hp` | Yai's HP (0x88:3) | the menu | a link battle | `goldfish` (goldfish (the Japanese ROMs': bubbles in the dark)) |
| 0x06 | `comp` | DoghouseComp (0x8c:0); KitchenComp (0x8c:2); ElctLockComp (0x8c:3); EngineComp (0x8c:7); ViewComp (0x8c:8); ChipMkrComp (0x8c:9); AirFilterCmp (0x8c:10); ArmorComp (0x8c:11); HelmetComp (0x8c:12); KatanaComp (0x8c:13); FurnaceComp (0x8c:15); ElevatorComp (0x8d:0); CraneComp (0x8d:1); TreeComp (0x8d:2); OldComp (0x8d:3); Dad'sComp (0x8d:4); SculptureCmp (0x8d:5); TerminalComp (0x8d:6); NetBattleCmp (0x8d:7); WineCaseComp (0x8d:8); DumplingComp (0x8d:9); ExpServComp (0x8d:10); WindGodComp (0x8d:11); PipeComp (0x8d:12); SquirrelCmp1, 3, 4, 5, 6, 8 (0x8e:0, 2, 3, 4, 5, 7); SquirelCmp10, 11, 13, 15, 16 (0x8e:9, 10, 12, 14, 15); the default: every real-world map | **no one area: the default, which the comps without their own draw** | 108 battle settings state it; a link battle | `calendar-squares` (yellow turning rings) |
| 0x07 | `comp-alt` | OldTrmnlComp (0x8c:1); RadarComp (0x8c:4); AirConComp (0x8c:5); ScrewComp (0x8c:6); ServerComp (0x8c:14); MessageComp (0x8d:13); SquirrelCmp2, 7, 9 (0x8e:1, 6, 8); SquirelCmp12, 14 (0x8e:11, 13) | **no one area: the default's picture in its second palette** | 1 battle settings state it (0x08113bf0); a link battle | `rings-blue` (0x06's turning rings in light blue) |
| 0x08 | `acdc-area` | ACDC Area1, 2 (0x90:0, 1) | the menu | a link battle | `calendar-blue` (blue diamonds rising on black) |
| 0x09 | `oran-area` | Oran Area1, 2 (0x91:0, 1) | the menu | a link battle | `calendar-gold` (red diamonds rising on black) |
| 0x0a | `scilab-area` | SciLab1, 2, 4 (0x92:0, 1, 2) | **the menu, spelled out as the dialogue has it** | a link battle | `ones-and-zeros` (slanted 01s and 10s on bright blue) |
| 0x0b | `visionburst` | unnamed maps (0x8a:0, 0x8a:1, 0x8a:2) | **the dialogue: the menu names none** | a link battle | `grid` (a green grid on black, warping) |
| 0x0c | `acdc-area3` | ACDC Area3 (0x86:0) | the menu | a link battle | `calendar-blue-2` (0x08's again) |
| 0x0d | `oran-area3` | Oran Area3 (0x86:1) | the menu | a link battle | `calendar-gold-2` (0x09's again) |
| 0x0e | `scilab-area3` | SciLab3 (0x86:2) | the menu | a link battle | `ones-and-zeros-2` (0x0a's again) |
| 0x0f | `end-area` | End Area1, 3, 4 (0x93:0, 1, 2) | the menu | a link battle | `numbers-3` (red and green diamonds on dark red) |
| 0x10 | `ship-comp` | ShipComp1, 1, 2, 3 (0x82:0, 1, 2, 3) | the menu | a link battle | `ship-wheels` (ship's wheels on blue) |
| 0x11 | `scilab-hp` | SciLab HP (0x88:4) | the menu | a link battle | `swirls` (swirls) |
| 0x12 | `end-area2` | End Area2 (0x86:3) | the menu | a link battle | `numbers` (0x0f's again) |
| 0x13 | `gargcastle-hp` | GargCastleHP (0x88:5) | the menu | a link battle | `lions` (pale lion masks on cream, one lit at a time) |
| 0x14 | `end-area5` | End Area5 (0x86:4) | the menu | a link battle | `numbers-2` (0x0f's again) |
| 0x15 | `soulserv-comp` | SoulServComp (0x84:4) | the menu | a link battle | `crests` (dark red crests on black) |
| 0x16 | `gargoyle-comp` | GargoylComp1, 2, 2, 3 (0x83:0, 1, 2, 3) | **the menu, spelled out as the dialogue has it** | a link battle | `shuriken` (shuriken turning on purple) |
| 0x17 | `factory-comp` | FactoryComp1, 2, 3, 4 (0x84:0, 1, 2, 3) | the menu | a link battle | `microchips` (gray chips with a magenta core, coming apart) |
| 0x18 | `undernet` | Undernet1, 2, 3 (0x94:0, 1, 2); NebulaArea6 (0x94:5) | the menu | a link battle | `glyphs-2` (brown speckles) |
| 0x19 | `nebula-area` | NebulaArea2, 4 (0x94:3, 4) | the menu | a link battle | `glyphs-blue` (0x18's speckles in dark blue) |
| 0x1a | `undernet4` | Undernet4 (0x86:5) | the menu | a link battle | `glyphs` (0x18's again) |
| 0x1b | `nebulagray` | none | **the battles that state it: no map draws it** | 2 battle settings state it (0x081140e0, 0x08114270) | unnamed (not looked at) |
| 0x1c | `nebula-area1` | NebulaArea1, 3, 5 (0x86:6, 7, 8) | the menu |  | unnamed (0x19's again) |
<!-- /backgrounds:exe5 -->

- A liberation's map (group 0x86) draws its area's picture again: the same load data as the area's other parts
  (0x0C as 0x08, 0x0D as 0x09, 0x0E as 0x0A, 0x12 and 0x14 as 0x0F, 0x1A as 0x18, 0x1C as 0x19).
- While its liberation runs, such a map draws another picture. The loader (0x0808C2C8) takes the load data from a
  second table (0x0808C500, where the usual one is 0x0808C578) when the map is the running mission's (0x08051982)
  and the battle isn't a link battle; the two tables differ for those seven numbers alone. The pack has the usual
  table's pictures, which are a link battle's.
- 0x1B and 0x1C are not among a link battle's, and no recording has shown them: their drawing isn't compared.

## 2. Running

    cargo run -p nettai-demo -- <trace.jsonl>              # watch a trace
    cargo run -p nettai-demo -- --match match.toml           # play a match file (§6)
    cargo run -p nettai-demo -- --match match.toml --show-folders
    cargo run -p nettai-demo -- --match match.toml --save-match played.toml   # keep the seed played
    cargo run -p nettai-demo -- <trace.jsonl> --headless 150,300,600 --out <dir>
    cargo run -p nettai-demo -- --match match.toml --audit-content   # what is missing?
    cargo run -p nettai-demo -- --audit <trace.jsonl>...   # and in these traces?
    cargo run -p nettai-demo -- --match match.toml --pack <dir>        # a pack elsewhere
    cargo run -p nettai-demo -- --match match.toml --host 7777         # netplay: host...
    cargo run -p nettai-demo -- --match match.toml --join 192.0.2.10:7777   # ...and join
    cargo run -p nettai-demo -- --match match.toml --record set.ntrp   # record the set (§8)
    cargo run -p nettai-demo -- --replay set.ntrp --side right   # watch it again, the right navi's console

Options: `--pack <dir>` names a content pack elsewhere and `--content <dir>`
the battle content (see above), `--mute` turns the sound off, `--round N`
starts a trace at round N (later rounds follow when a round's input runs
out), `--scale N` sets the window's first size to play in (default 4 times
240x160; the window can be resized, and the picture keeps whole pixels,
centered on black), `--paused` starts paused, `--png-scale N` scales headless output
(the text layer is drawn at that scale too), `--quit-after N` closes the
window after N ticks (with `NETTAI_WINDOW_SHOT=<file>` set, the window's
last picture is written there as a PNG; with `NETTAI_PLAY_STATS` set, what
each picture costs to show is printed every two seconds), `--text font|original` chooses
how strings are drawn (default `font`; the frame comparison uses
`original`), `--font <file>` puts another TrueType or OpenType font in the
bundled one's place, `--lang en|ja` the language of the battle's words
(default `en`; §3, "Languages"). `--match FILE` plays a match file (§6: you
are its left side). The file sets the game, its rounds, both sides and the optional
`seed` for the battle's RNG (default: from the clock; each start prints it).
Edit those settings in the file or with the editor (nettai-demo with nothing
to play, or `--edit FILE`), which plays the match in the same window; its
Random button creates a random setup. `--show-folders` prints both folders, and with
`--headless`, `--keys` holds buttons on given ticks (below). `--save-match FILE`
writes the match played, the file's setup or the one netplay agreed, with its
seed, as a match file. `--record FILE` writes a replay of the set played (alone,
`--host` or `--join`), and `--replay FILE` plays one, shown from the side that
recorded it or `--side left|right`; with `--headless F` it renders the set's
ticks F (§8).

Keys: arrows move, Z = A, X = B, A = L, S = R, Enter = START,
Backspace = SELECT; Space pauses, `.` steps one frame while paused, `-` and
`=` change speed (1/8x to 16x of 59.73 Hz), F5 starts over (a recording's
round; live play's set, from its first round) (none of these in netplay),
Tab shows the next of the content's languages (the
battle goes on: the language is the drawing's alone), Esc stops (back to the
editor when the editor's Play started it, else the window closes). Nothing is
written over the battle's picture: the window's title says where playback
is, its speed, a pause, why it stopped, and in netplay the connection's
figures and the result.

**Trace playback** runs at the original's 59.73 frames per second until the
input ends or the engine hits something it doesn't implement yet. Then it
stops, with the reason in the window's title and printed; the first difference from
the trace's recorded state is printed too. Frame numbers are the trace's.

**Live play** (`--match FILE`): you are the left navi; the right one stands
still. The match file sets up the battle (§6), including its game, rounds,
each side's navi, folder, forms, patch cards, NaviCust and stats. Use the
editor to create, randomize or edit a match before playing it.

Live play is a set of the rounds the match lists, as a link battle is (the
original's triple battle: three, best of three; a match may list any number
from 1 to 99): when a round ends the
next one starts, on the next round's place, with the score carried and each
folder shuffled again by its console's RNG, and when the set is decided play
stops with the result in the window's title and printed (`the match is over: you won`). A set
of `n` rounds is decided once one side can no longer be caught, or after its
last round, drawn if the sides are level (`sub_800AF50`, with `n` for three: an
even number of rounds may end drawn, as three may on a drawn round); each
round opens with the ROUND banner and its number, in the banner's digits. How a
set goes on is `nettai_match::Set`'s to say, for live play and netplay alike
(`Set::after`: the next round's battle from the round that ended, or the
result); the session swaps the battle and tells the presentation to start
over, and the frames go on being numbered from the set's first tick.

- **The field**: each round's stage and background, as the match states
  them; a part it leaves (or a round left empty) picked from the seed as a
  link battle picks them, from the game's rule section `link_pick` (`sub_81209DC`: its
  `stages`, the stage for each index of the original's pick, EXE6's 96 link
  battle stages each once; its `backgrounds`, EXE6's `byte_8120A20`). Each
  round's stage and background are drawn whether stated or not, so a part a
  match states moves no other's pick, and a random match states the three
  rounds a match that states nothing would be picked; the first round's stage is one
  of the first `first_round_stages` (the count the comm menu passes for a
  triple battle's practice: EXE6's all 96, EXE5's the first 68). gen-content
  checks the lists and the count against the ROM, and verify's `link_pick`
  test checks them against every recording's rounds. (EXE5's are its own: 96
  indices over 82 stages, the first twelve records twice and twelve stages
  never, as its 0x08129F2C picks; and 27 backgrounds.)
- **A folder for each player**: 30 chips that keep EXE6's folder rules
  (`nettai_match::folders`: the folder editor's, `sub_8135080` with `sub_8135500`: copies
  of a chip by its MB, five up to 19 MB down to one from 50; Mega and Giga
  chips within the fresh navi's levels; a code each chip comes in; a
  Regular chip within the fresh navi's Regular memory, if a chip fits it; no
  tag chips in a draw), from
  the chips the chip pack lists (Standard, Mega and Giga, not the dark
  chips, and not the five the US game has no routine for). The codes lean to
  two the folder favors, and `*`. Each console shuffles its folder from
  the seed at the round's init, as before.
- **Five Crosses for each Cross window**, picked from MegaMan's ten, both
  games' (the setup's list of the player's Crosses; one of both games is
  nettai's: docs/engine/custom-screen.md §4.1). A Cross of the other game is its own
  form, buster, charged shot, element and face; Beast Out from it is its
  own Beast form (HeatCross Beast, of Gregar's Beast, for a Falzar player).
- **Each player's game**, Falzar or Gregar: their Beast (Beast Out from
  the base form, Beast Over), their console's own pictures and Beast Out
  roar, and the navi's game (NaviStats+0x20, which MstrCros reads).

The draw is the frontend's, made before the battle; the battle is then a
function of its setup and the buttons, as rollback needs. The same seed
gives the same setup.

The custom screen is the engine's (docs/engine/custom-screen.md), drawn, and
also shown as text: the dealt chips in the grid's order (`>` the cursor,
`+` picked, `-` grayed), OK and Beast Out, the picks and the Cross window's
Crosses by name. The keys are the game's (A picks, B takes back, START goes
to OK, UP from the top row opens the Cross window, R describes, SELECT
hides). The right navi's screen picks its first chip and presses OK. As in
the original's netbattles, the fight gets your buttons 4 ticks late (the
link). F5 starts over with the same setup.

**Netplay** (`--match FILE --host PORT` or `--match FILE --join ADDR:PORT`) plays another
player over the network, with rollback (docs/design/rollback.md §4; the
frontend's side is `netplay`):

- **Hosting**: `--host 7777` listens on UDP port 7777 of every IPv4
  interface and waits for a player (`--wait SECONDS`, default 300), the
  window open and blank meanwhile, its title saying what it waits for (Esc
  gives up). On a LAN
  the other player joins this machine's address; over the Internet, forward
  the UDP port on the host's router to the host's machine, and the other
  player joins the router's public address. The host is the left navi
  (side 0).
- **Joining**: `--join 192.0.2.10:7777` (a name works too) reaches the host
  and waits for its answer (`--wait`, default 30). The joiner is the right
  navi (side 1), and sees the battle from its side: its navi on the left of
  the field, mirrored as the original's second console shows it, its own
  custom screen, HUD and sounds.
- **The handshake** checks that both players run the same netplay protocol,
  the same engine, the same game (a match is of one: "can't play: the other
  side plays exe5, this one exe6: a match is of one game, both sides playing
  it") and the same content (`Content::hash`: the definitions, scripts and
  rule tables, and what the battle reads of the pack, the asset names and
  the animations' timing), and refuses a mismatch on both sides with what
  differs ("can't play: the other side plays other content (its hash ...,
  this one's ...)"). Each player then brings their own side of the match (an
  offer, in nettai-match's binary against the content both play,
  `nettai_match::binary`: the side's facts in the order of the game's rules'
  setup, each definition by its handle, which the content hash pins): a
  match file's left side (`--match`), its facts all (folder, version, forms,
  patch cards...);
  the other player's is checked against the content as a match
  file's side is (`nettai_match::check_side`, §6). Both play by the game's
  rules (a game has one ruleset). The language (`--lang`) is each player's own. The players
  agree the match's settings, its game and its rounds (each round's stage and
  background stated, or left to the seed), in a symmetric lobby
  (`nettai_frontend::lobby`): each proposes and readies, any change by either
  clears both readies, and the settings are agreed when both are ready on the
  same. The program proposes each player's match file's settings: two files
  that state other rounds stop it, saying what differs ("the rounds: 3 here,
  5 there", "round 2's stage: ..."). Then each commits (a Hello: SHA-256 of
  the agreed settings' hash and what it will reveal) before either reveals
  its half of the seed and its side, and each checks the other's Reveal
  against its commitment; the battle's RNG comes from both halves, and each
  part the rounds leave is picked from it alike on both. Both print what was
  agreed, and `--save-match` writes it; a recording keeps every round's place.
- **Playing**: the match is a set of the rounds it lists; its rounds follow one
  another (the folders shuffled again by each console's RNG). Every frame the
  frontend sends your buttons and shows the frame its rollback session
  presents: by default the newest (`--present-delay N`, default 0: the
  frame N ticks behind your newest input, so that fewer corrections show
  and your own input shows that much later; `[` and `]` change it during
  the match; it is yours alone, never sent, and the other player picks
  theirs). The other player's input is predicted until it arrives, and the
  frame is simulated again when a prediction was wrong. A cue played on a wrong
  prediction is stopped or taken back (rollback.md §3.2). There is no pause,
  speed change or restart (F5) in netplay.
- **The window's title** shows the connection's figures (the library's
  `Player::net_status`, values the program words itself): `ping` (the round
  trip, in milliseconds), `loss` (the share of the other player's datagrams
  that were lost), `present delay` (the present delay), `rollback` (the last
  rollback's depth, then the deepest and how many in all) and `waits`
  (frames held for clock sync or the stall guard).
- **The end**: when the set is over the result shows in the title and
  the window stays open; Esc leaves, and tells the other player. If the
  other player leaves, nothing arrives from them for 10 seconds, or their
  input falls more than the rollback horizon behind, the match stops with
  the reason in the title and printed.

The stand-in bot (the right navi standing still, picking its first chip)
stays for playing alone.

**Headless mode** renders the listed frames (`a,b,c-d`; trace frame
numbers, or tick numbers in live play) to `frame_NNNNN.png`. It exits
non-zero if some frames couldn't be rendered (the engine stopped first).
With `--objects` it also lists every rendered frame's objects as the
renderer sees them: kind, screen position, sprite, animation and frame, and
look (palette, shadow, flips, white, shader, hidden parts, whether it is
drawn at all), and what its console shows of its chips; and its text items
(the words, the role, the box, the depth key, how many of the box's pixels
something in front covers, the squash, the fades). In live play
`--keys` gives your buttons by tick (`headless::KeyScript`): for instance
`--keys 160-161:up,215:a,260:start,266:a` opens the first screen's Cross
window (a direction acts on a hold's second tick), chooses its first Cross
and presses OK.

**The audits** list what the drawing code and the audio look up that the
packs or the content don't have: a sprite that isn't in its pack, an
animation or palette the sprite doesn't have, a chip without an icon or a
picture or with a name the font can't spell, a face, an emblem, a banner
without glyphs, a telop's banner that is no telop's, a text line, a song.
Each exits 1 if there was any; drawing itself skips what it can't find,
so nothing else notices. Every such lookup goes through one module
(nettai-render's `lookups.rs`; the audio's, a cue's song, nettai-demo's
`sound_lookups.rs`), which notes it (`audit::Lookup`) and checks it once a
run, so both audits make the lookups a frame makes, through the same
functions:

- `--audit-content` (`content_audit.rs`) makes every lookup for everything
  the content defines, in every language it has strings in: every chip's
  icon, picture, name, its window's class,
  element and code pictures, its description in the dialogue font; every
  navi's face, emblem, name and no-running message
  with its portrait (a navi a side can start that has no message, or none
  to say it, is a problem where the game's roles fill the message's sound:
  the custom screen opens no box for it, so L would do nothing); every
  form's face for each emotion, every Cross's name
  and description; every custom-screen button's look (a button the pack
  has none for is drawn as nothing, unless it shows a chip), on a console
  of each of the pack's versions; and every asset of the loaded packs (each sprite with
  every animation, its frames and their own palettes; each song, banner,
  background, mugshot), the HUD's text lines, the custom screen, the
  chatbox. It checks the field too: each loaded game's pack must draw
  the panel types its game names; then, in an arena of each game, every
  panel type a loaded game names and both highlights are drawn as the
  stage draws them, and a tinted one is said as a note. It takes a
  second or two, and a lookup by the wrong key fails it for every chip,
  not only for those a trace shows (its test:
  `a_lookup_by_the_wrong_key_fails_for_every_chip`). A string a
  language's table lacks shows in the content's own, by design: it is said,
  not counted. A language the content has strings in and its pack no
  lettering for is a problem (a console in it can't be shown): both games'
  packs have their Japanese. It audits the
  match file's one game (`--match FILE`); the match's sides aren't played or validated.
- `--audit <trace.jsonl>...` runs traces, several at a time (`--jobs N`,
  default one a core), and makes the lookups their frames and sound cues
  make, without drawing: no stage, no composing, no sound synthesis
  (`Renderer::set_lookups_only`). It catches what the content can't say
  beforehand: the palette an object picks, a telop of a chip the engine
  wasn't told. Each problem is listed with its trace and the frame it was
  first seen on. `--draw` draws every frame and plays every cue into nothing
  besides (the audit as it was before: some ten times slower). `--lookups
  FILE` writes each trace's distinct lookups, by name (and with
  `--audit-content`, the static audit's), for the verification's trace
  cover: the few golden traces that, with the static audit, make every
  lookup all of them make.

**Sound**: the player renders each tick's sound cues with the pack's sound
(nettai-audio's `BattleAudio`) and the window plays the samples through the
audio device (`nettai_audio::Output`), unless `--mute`; headless rendering
never plays sound. In netplay the player renders the cue actions of each
frame (plays, and cancels of cues played on a wrong prediction).

## 3. What is drawn, and how

The drawing is nettai-render's; the modules this section names are its.

Layers, back to front: the backdrop color, the background (priority 3),
the field (priority 2), sprites of priority 2, the HUD layer (priority 1),
sprites of priority 0 (banners), and BG0 (priority 0: the custom screen's
enemy names).

**Objects** (`objects.rs`) follow the original's render passes
(`sub_8003E18`/`sub_8004218`/`sub_8004510`, `sub_30061E8`, `sub_3006440`,
`sub_3006920`):

- pools in order actors, attacks, effects; objects of a pool in update
  order; only objects flagged visible whose sprite animates;
- projection of 16.16 positions: screen x = x - camera x + 120 (x negated
  first for the right-hand player), ground y = y - camera y + 80, sprite y =
  ground y - (z - camera z). The camera is the local console's shake
  (`Console::camera`): it moves the sprites and the field layer together,
  by whole pixels;
- per part: flip-adjusted offsets, the original's culling, hardware
  position wrapping, the palette the object chose plus the frame's first
  part's offset, the color shader;
- the first part is the shadow: hidden, drawn on the ground one layer back
  (`sprite_hasShadow`), or drawn with the sprite (`sprite_noShadow`). Many
  sprites' first part is no shadow but part of the picture, so a kind that
  leaves it hidden (the state after loading) draws incomplete, or nothing.
  A sprite with a ground shadow that the game marks with its flag 0x20
  (`sprite_setField0x3Bit5`: `Look::under_objects`) has every part drawn as
  the shadow is, one layer back and in the first bucket, at the sprite's
  height: a mark on the floor, under every object (EXE5's immobilized mark,
  its ripple over a dived navi, DethPhnx's fire);
- order: parts go into depth buckets (ground y + 0x40) and come out
  deepest first, most recent first, so later and lower objects are in
  front; at most 128 parts;
- white flash, alpha and mosaic (sampled as mGBA does);
- an effect the game spawns without a position (the second explosion of a
  deletion: its X and Y are memory addresses, far off the screen) isn't
  drawn.

**Field and background** (`stage.rs`): each panel's block by displayed type
and owner (from the viewer's side), highlights, missing panels, front
edges, the cycling panel palettes; the background's scroll and tile
animations. The field is the match's game's pack's: a match is of one game,
so its field draws its panels. `FieldArt` says, for each panel type and
highlight, whether the field draws it (field.json's `panel_types`); one it
doesn't is the owner's normal panel,
tinted halfway to magenta, never a hole and never another pack's. A tinted
panel is said in the audits, not counted.

**HUD** (`hud.rs`), by the original's HUD tasks:

- the HP box with its rolling number and colors, and the custom gauge
  (fill, the full gauge's animation, whose phase carries over from the
  last "Cstmzing..." wait);
- the emotion window: the face the navi's form names for its emotion,
  with the count beside it, or a link navi's own face (in its second
  palette in Full Synchro); the 12-tick blink back to the previous face
  when one of the base form's faces changes (white instead, on a change to
  Full Synchro); the flicker of a bugged navi's window; the form chosen on
  the custom screen while the screens are still open, and the window gone
  from the tick the fight resumes until the transformation is over; shown
  through the damage judge;
- the next chip's name, damage and bonus (the hand's and the navi's own:
  `kinds::player::next_chip_bonus`) at the bottom left, while the navi can
  use it (`Battle::chip_hud_for`);
- the chip icons over the local navi (and over both in a battle that is no
  netbattle), among the field's sprites by depth; the opponent's
  defensive chip as "????";
- the HP numbers under objects, in the console's four places
  (`Battle::hp_numbers`): the opponent's navi's, LilBoiler's damage taken
  (rolling, colored);
- the warning markers (`Battle::warnings`): the arrow over the custom
  gauge, or over a place on the field, blinking with the console's frame
  counter;
- banners with their squash and stretch; a telop (the chip's name, its
  damage and bonus, "x2"; "????" for a trap's, to the opponent) on its
  user's half of the screen (`Battle::telop_for`); the chip the other
  player just used, named for a second (`Battle::used_chip_for`);
- the damage judge's numbers, rolling and real;
- the HUD's text lines: the turn timer's seconds and "TIME UP!" from a
  netbattle's 15th turn, "COUNTER HIT!" (`Battle::message`);
- "Cstmzing..." while the opponent is still choosing, "PAUSE".

The rolling numbers, the blink and the gauge's animation phase are
presentation state the engine doesn't keep; `Renderer::observe` follows
them tick by tick. The HUD's pieces placed over an object use the HUD's own
projection (`sub_800362C`): on the right-hand player's console it mirrors
after the camera, so a shake moves them against the sprites there.

**Fades** (`compose.rs`, `render.rs`): the intro fades in from white (from
black after the first battle of a set), the round's end to black. A dimming
darkens the stage (background and field: the palettes 0-8) and leaves the
sprites and the HUD's layer. The transformation sequencer fades every tile
layer to black while navis change form (sprites keep their colors). A
palette flash takes the transformation's palette transform: variant 0
(`sub_80E10C0`) whitens the stage's palettes on the frames its counter has
bit 2 clear and leaves the HUD's colors, variant 1 (`sub_80E114C`, the
FlashBomb's) whitens the stage, the HUD and the sprites every frame;
neither shows while the battle holds it (`objects::palette_flash`).

**The custom screen** (`custom.rs`): the local player's screen as the
original draws it on its console (`sub_8026A28` and its states), from the
engine's `Screen` and its presentation state (`Screen::look`,
docs/engine/custom-screen.md §9) and the pack's `graphics/custom`:

- the window on the HUD layer: the original's 15x20 map (with or without
  the Cross tab) and its patches, composed from the tile numbers the map
  names: the blocks the battle loads at fixed places (the frame from tile
  1, the picked column's cells, the last turns' block) and what the screen
  copies in as it runs; it slides in and out a column or two a tick under
  the layer's scroll, and SELECT takes it off;
- the chip window: the chip's name (8 cells of the 8x16 font, in the
  window's colors), its picture and palette, the window's colors by its
  class (a dark chip's dark: no EXE6 chip is one), its code, its element's icon and colors, its damage ("???" for
  a chip with the trait `hides_damage_as_a` as an A: the original compares the whole chip word, number and code,
  with Muramasa's number, so a Muramasa M shows its damage); for OK, Beast Out and the buttons their pictures;
- the slots (each dealt chip's icon and code, grayed or picked by its
  palette; the empty slots; the Beast Out, re-deal and scrap buttons) and
  the picked column's icons and cells;
- sprites (layer 1, bucket 0, each in front of the last, as the raw OAM
  list `sub_8009FF8` fills): the cursor's four corners in its two frames,
  the navi's emblem over the column (a 32x32 affine sprite: it spins after a
  pick), the Regular chip's frame;
- the enemy names on BG0 over their bar on the HUD layer, on a round's
  first screen;
- the Cross window: its opening steps, its map with the Crosses' names (the
  one under the cursor in its own look, palette 10 the Cross's), its
  cursor; a Cross's choice whitens everything and puts the Cross's face in
  the emotion window;
- the Program Advance animation (the window out): the picks' names and
  codes a column right of the layer's scroll, the recipe's in the blinking
  palette 10 and taken off, the Program Advance's in their place; the
  stage and the objects fade a quarter of the way. The original hides
  the code of a chip numbered 0x160 or more (0x08027BC6 in EXE5,
  `sub_802B80C` in EXE6), and no recipe of either game uses such a chip,
  so every pick shows its code;
- the scrap and the re-deal: the column losing the scrapped picks, the
  slots dealt again, the emblem and the Regular chip's frame throughout;
- a button's look is the pack's, by the name its content registers the
  button under (`lookups::button`; asset-formats.md, "Buttons"): its tiles
  by set and which set a state shows, the cursor over it, its picture in
  the chip window, its uses left and the chip it holds; its tiles a set are
  its cells' (the content's `cells`), and a special slot with no button
  shows the hidden set of the button its content registers there, if its
  look has one (EXE6's Beast Out's). The renderer has no look of its own,
  and names no button: what it draws of one besides its look goes by the
  button's `view` (`ButtonView`: one that offers a form shows the offered
  form's icon in the column and its picture in its second palette for the
  alternate offer, EXE5's soul button; one whose picture is a chip's too
  gives the BeastOut chip its picture, EXE6's Beast Out button);
- the navi's emblem is the pack's under the navi's key (`lookups::emblem`),
  its palette sprite palette 11 (the cursor's and the Regular chip's
  frame's too); a navi with none shows none;
- a console's own pictures by its version (`Versioned`: a Gregar console's
  Beast, the pack's `-gregar` assets); a Cross's name and
  colors in the Cross window are its own version's (`custom::cross_picture`:
  the pack's pictures of the form's `version`, numbered by the Cross's
  place among that version's Crosses in its navi's list,
  `forms.<version>.crosses`, which is the original's order;
  for the form in the entry's place, `custom::cross_at`: the player's Cross
  list's entry, else their version's Cross of that number, read as the
  facts the engine names, `Battle::fact` and `PlayerFact`), so a Gregar
  Cross shows Gregar's name in any window, and a window a setup's Cross
  list mixes shows each game's own; the Beast Out button, its picture in
  the chip window and the BeastOut chip's picture are of the Beast the
  navi goes into (`custom::beast_pictures`: the player's version's from
  the base form, and a form's own version's from a Cross), so a
  Falzar player in HeatCross sees Gregar's;
- what the screen does to the rest: the HP box and the mugshot move right
  with the window and the field and the sprites 15 pixels down (the
  camera), the gauge and the HUD's "????" stay off until the local result
  is sent, Beast Out's
  fade darkens the stage, the HUD layer and the objects (sprite palettes
  0-10) half way, the camera's jitter moves the HUD layer in Beast Out's
  states, and the emotion window shows the Beast form chosen. A dark
  chip's hover (never seen in EXE6) darkens the stage and the objects on the
  first fade record and the HUD layer and the screen's sprites on the
  second.

The screen's sounds are the engine's cues for its player
(docs/engine/audio.md §1): they play through the audio crate like the
battle's.

**The chatbox** (`chatbox.rs`) the custom screen runs (R: a chip's or a
Cross's description; L: the no-running message), as the original's
chatbox draws each tick: the box's map on BG0 at its opening step (row 12,
the message box or the narrower description box); the text as the line
buffer's three rows of six sprites at (51, 108), its lines 14 rows apart
(the third row of sprites is 32x8: a third line's descenders are cut, as in
the original; in the font mode each line fits the open box's inside,
`text_room`); the speaker's portrait, a sprite whose animations are its
faces, stepped by its updates and tinted while it fades; the key-wait
arrow. The text is the content's strings (the chip's or the Cross's
`description`, the navi's `run_message`, in the player's language), how
far it has printed and the rest the engine's chatbox
(docs/engine/custom-screen.md §3.5).

**An EXE5 console.** In an EXE5 match the HUD, the custom screen and the
chatbox are EXE5's pack's: its HP box, gauge, fonts, banners, emotion
window, window and chatbox, by the pack's own tile numbers and layout
(docs/design/asset-formats.md §4, "Another game's HUD and custom screen").
What an EXE5 console does otherwise, by data, not by game:

- the emotion window: the faces EXE5's forms name bring their own box
  (MegaMan's five, Team Colonel's), and a soul's face shows the soul's
  turns left beside it (`Battle::form_turns`: the turns the rules keep for
  the button that offers a form);
- the custom screen: the special slot's button is drawn by the pack's look
  of its name (Soul Unison's, its picture in the chip
  window in Chaos Unison's palette for Chaos, as a button whose view is
  `form_offer`), the cursor over OK and over
  the button where the pack puts them; the soul choice (the window whose
  view is `offer_flight`, EXE5's state 9) flies the soul's icon
  up onto the column's first cell under its flash, and the cell keeps it
  (`Battle::offer` and `Battle::offer_flight`: the offer, a soul's form, and
  the window's step, typed reads of what the view shows of the rules'
  state; the icon is the soul's place
  among its navi's souls, `soul_place`: a pack's soul icons are in the order
  the navi lists its souls, the original's soul numbers'), in the palette of
  the soul's own version on any console (Team Colonel's icons have another
  outline color, the pack's `icon_versions`; the soul's version is its
  place's among the navi's souls, a version's after another's:
  `soul_palette_row`; §5);
- what a soul adds to the screen, drawn by what the engine's screen says,
  not by soul (verified frame for frame, exe5.txt's custom/capsules,
  custom/capsule-frame, custom/arm-change and five souls/ scenarios):
  - a button that shows a chip (`Slot::face`: MeddySoul's capsules) is a
    chip's slot: the chip's icon (the empty icon once used), a blank code,
    gray while unavailable, the chip cursor; its chip window is the chip's
    name and picture alone, over the frame colors the last chip slot left
    (`ChipWindow::framed`: a capsule after a Mega chip keeps the Mega
    frame); R describes the chip; its mix (the window whose view is
    `chip_flight`, `Battle::chip_flight`) flies the chip's icon as the
    soul's choice flies the soul's;
  - the button look `arm_change` (ColonelSoul's Arm Change: the pack's
    look of that name, its second tile set for unavailable
    alone) with the chip it holds as a sprite over it in the HUD's icon
    palette, on the ticks the engine draws it (`Drawn::held`: while
    choosing, through a soul's choice, and in the blink's last 20 ticks
    when the column hides the icon); the column cell the chip left stays
    drawn for the tick it leaves (`ScreenLook::column_kept`);
  - the re-deal button's uses left, a digit in the chip window's damage
    cells, where the pack's look says (`uses_digit`: EXE5's Shuffle;
    EXE6's ChpShufl shows none);
- its game's flow (rules `flow`, read of the console's own game): the
  custom screen's close starts the chip window (`chip_window_at_close`:
  EXE6's as its Japanese games' do, on every console; EXE5's), the intro
  fades in from black (`intro_from_black`);
- its game's custom screen (rules `custom_screen`): the emblem as SELECT's
  hidden window comes back, on the tick of the key and the next
  (`emblem_at_window_return`: EXE6's screen draws it on both, EXE5's on
  neither; the engine's screen draws it or not, `ScreenLook::drawn`); the
  chatbox's commands after a character (`chatbox_commands_wait_for_text`:
  EXE5's key-wait arrow comes up, and its speaker's mouth closes at a
  line's end, two ticks sooner than EXE6's) and the characters that move
  the speaker's mouth (`talking_characters`), both in the engine's
  chatbox, whose look the frontend draws;
- a version's own chip (its five Giga chips, DethPhnx or Phoenix) shows
  its own version's ROM's icon and picture on either console (§5; the
  console's version is `Renderer::console_version`, which an EXE5 recording
  names; live play, the pack's first version).

The frame comparison against EXE5's consoles (verification's
tools/frontend-compare/exe5.txt, chiplab's library-exe5) and what still
differs are in §4.

**Text** (docs/design/text-rendering.md §9). Every string goes through
`fonts.rs`: the 8x16 font's (`cell_glyphs`, `cell_text`, `draw_cell_text`,
`cell_glyph`, `layer_text`, `layer_line`: the HUD's lines, chip names,
telops, the enemy names, the Program Advance's names, "VS") and the
dialogue font's (`dialogue_glyphs`, `dialogue_text`, which composes a line
as the original's line buffer does: each glyph OR'd in at the pen, cut
past its advance but never before eight pixels). In the original text
mode that is all. In the font mode (the default) each of those strings
that the font has every character of becomes a text item instead
(`textlayer.rs`): the words, in the box the original's glyphs take, with
the face and shadow colors of the palette the original draws them in;
the glyphs are left out of the frame (a telop's glyph parts become blank
parts, as many, so the sprite limit is unchanged). `Renderer::render`
returns a `Frame`: the picture, per pixel the depth key of what won it
(`compose_with_depth`: priority, sprite or layer, the sprite's place in
the hardware order), and the items, each with its own key (its layer's,
or its sprite parts') and its fades (its layer's or the sprites', then
the screen's). `present.rs` scales the picture and draws the items over
it at the output's resolution (`vfont.rs`: swash, the bundled font), each
pixel only where nothing in front of the item's layer or sprite won the
frame pixel under it: banners, the mugshot or the chatbox's arrow cover
text as they do in the original, and a telop's squash is a transform of
its text. A string the font lacks a character of is drawn in the game's
font, whole.

**Languages** (docs/design/text-rendering.md §10). `--lang ja` shows
the battle's text as the Japanese games do, in either text mode: the
pack's Japanese lettering (the Japanese ROMs' 8x16 and dialogue fonts in
their encoding, the HUD's lines in their glyphs, the banners whose words
differ, カスタム中…, the gauge's "L or R", the chip window's pictures for
OK, the re-deal and scrap, the Cross window's names) swapped in for the
English (`Bundle::in_language`), and the content's strings from
each game pack's `locales/ja.toml` (chip names and descriptions, the
Crosses' descriptions, the navis' names and no-running messages;
`strings.rs`, `DisplayText`: a string the table lacks is the content's
own, `locales/en.toml`'s, which `--audit` lists). Only what is shown
changes: the battle reads the shape of the content's own strings (a
description's lines, a message's characters per line), and a translation
prints in step with it (a description's whole lines in the proportion of
the own lines printed, a message's characters in the proportion of the own
characters), so two players of different languages play one battle. The
program's own text (what it prints: folder listings, a match's
description) is the content's own strings.

The bundled font is Murecho (`crates/nettai-render/fonts/murecho`, SIL
Open Font License 1.1, its license beside it): Latin, kana and some 2,300
kanji, weight 700 for the 8x16 font's strings and 300 for the chatbox's.
A string too wide for its box is squeezed (to 85%, 70% with kana or kanji)
and then made smaller; it never leaves its box, and its layout never
reaches the simulation.

### What the engine gives the frontend

Presentation outputs: the simulation reads none of them (`digest.rs` lists
the ones the state digest leaves out).

- `object::sprite::Look` on every `Sprite` (palette, flips, shadow mode,
  white flash, color shader, alpha, mosaic, priority, hidden parts),
  reset by `Sprite::load` and set where the behaviors call the game's
  `sprite_*` routines; the player navi's palette (`navi_palette`: no-charge,
  the Cross palettes, a link navi's, Beast Over's glow) and status shaders
  (anger, the paralysis and immobilized blinks, the invulnerable glow,
  frozen, the counter-able blink) every tick.
- `hud::Banner::id`, the banner showing, and `Banner::telop`, what a telop
  says; `Object::telop_chip`, what a dimming's controller will name
  (`Battle::telop_for` is the viewer's side of it: a trap's name is hidden
  from the opponent).
- `Battle::used_chips`, `Battle::chip_hud` (whether a console shows its
  navi's icons and chip window), `Battle::message`, `Battle::warnings`,
  `Battle::hp_numbers` (each console's HP numbers by place: what asked for
  one, from where, and the HP it starts rolling from).
- `FormData::mugshot` and `NaviData::mugshot`, the faces the definitions
  name.
- `Console::camera.jitter`, this tick's shake, and the emotion window's
  flicker (`Console::emotion_window`).
- `Field::clear_highlights` at the start of each tick: highlights last one
  frame (the original's field renderer clears them after drawing).
- The sound cues (docs/engine/audio.md).

A telop names its chip from what the engine was told when the dimming
started: a chip's use, or, for a dimming content starts itself (a trap
springing, a statue punishing, VDoll's curse), the `telop` it passes to
`dimming.start`. One that passes none shows no name (`--audit` lists
it).

## 4. Verification

Headless frames are compared pixel for pixel with screenshots of the
original running under emulation, one per battle frame; the frames where
the custom screen is up are counted apart. The comparison runs the
frontend with `--text original` (the verification workspace's scripts pass
it): in that mode every frame of the comparison's scenarios is the same,
byte for byte, as before the text layer existed.

**The vanilla PvP test match** (round 1 frames 72..=1145, round 2 frames
1224..=2554): **all 2404 frames the engine simulates are pixel-exact**, the
HUD and both custom screens included (the second with Beast Out): the intro
fades and the opponent's mosaic fade-in, round and turn banners, movement,
GunDelSol with its name and icons, the deletion, the win banner and the
fade out, and in round 2 Beast Out with its overlay, afterimages, lock-on,
camera shake and screen dim.

**A second match**, three rounds traced on the right-hand player's console
(so the field is drawn mirrored), with Crosses, rock cubes, ice and grass
panels, traps and Invisibl: of the 6397 frames outside the custom screen
that have screenshots, **all 6397 are pixel-exact**, every row of them, and
so are all 13,278 of its custom-screen frames, with the Cross windows, the
DustCross scrap, a description's chatbox and the screens' openings and
closings.

**Chip-lab scenarios**: 132 scenarios, a few of every family
(shot, sword, thrown, placed and dimming chips, navi chips, the link
navis, traps, supports, stages with their objects, forms, Beast Over, the
flow: knockouts, the damage judge, pause, a counter hit, a lost Full
Synchro), each recorded with a screenshot per battle frame. Of 123,433
frames outside the custom screen, **all 123,433 are pixel-exact**, and so
is every scenario on every frame, its 21,900 custom-screen frames included
(the Cross window, the Program Advance animation, Beast Out): the telops of dimmings content starts
itself, LilBoiler's HP number, the warning arrows, the faces. No frame
panics, and the audit names nothing missing.

**The custom screen's own scenarios** (42: the re-deal, the scrap, the
keys, the Cross window, Beast Out, invalid chips, modifiers and Program
Advances, a link navi's own chip, the chatbox, six on a Gregar console):
all 36,329 frames outside the custom screen are pixel-exact, and so are all
11,167 of its own. **The chatbox's** (26 more: every link navi's run
message, A tapped, B held, waited out; the Giga chips' descriptions, a
Cross's, an invalid chip's; four on a Gregar console): all 8,567 frames
outside the custom screen and 4,157 of its 4,177 are pixel-exact; the 20
others are the Gregar chip's description on a Falzar console (§5).

**Every link-battle stage** (96 scenarios: the 47 stages the content
defines, each traced on the left-hand player's console and on the
right-hand one's, where the field is drawn mirrored, plus the one
background no other recording shows): both navis walk their panels
without NaviCust programs, so the roads carry them each way, ice slides,
cracked panels break and come back, volcanoes erupt, poison hurts; the
road stages run until their roads blink out and turn normal. All 101,424
frames outside the custom screen are pixel-exact, the road arrows'
palette cycling, their direction on either console and their last-second
blink included, and so are its 7,872 custom-screen frames and the 85
other stage scenarios (boulders, statues, ice blocks, the panel-changing
chips: 73,929 frames).

Roads don't carry a navi with FloatShoe (or a submerged one): the
original's `sub_801A400` tests its object flags 0x24 first. A navi set up
with that program sees the roads drawn and animated, standing still on
them.

**On Japanese consoles** (traced on two JP Falzar consoles, and a JP Gregar
pair for the Gregar chip): Otenko placed, blessing, timing out, broken,
absorbed and thrown by DustCross, taken and thrown as junk by DustMan;
GunDelEX, HackJack's Count and his lances, Django and his bike, DblBeast,
the Gregar and Falzar chips with their beasts, CrosOver's Django and his
gun. Every frame is pixel-exact outside what a Japanese console writes in
Japanese (chip names, telops, the gauge's message, the custom screen's
labels), the chips' pictures included; the Gregar chip's on a Falzar
console is the known difference above.

**On EXE5 consoles** (verification's tools/frontend-compare/exe5.txt: 16 of
chiplab's library-exe5 scenarios, Team ProtoMan against Team Colonel,
traced on the Team ProtoMan console and once on the Team Colonel one,
Soul and Chaos Unison and the auto-battling navi's Chaos among them, compared
with `--text original`, EXE5's pack loaded beside EXE6's): of 7,964 frames,
7,914 are pixel-exact, the navis included, and 12 scenarios wholly: the
HUD, the emotion window (a dark chip user's flicker too, now that the
console's RNG1 is the engine's: the dark chip offer, exe5-map.md §15.3
item 13), the custom screen with its picks, Soul Unison's choice and a
dark chip's hover, its close (the hand's name and icons on the tick the
results are in), the chatbox, the banners, the mercy flash and a
deletion's result. What still differs:

- the UNITE button for a soul not offered yet (HeatSoul for AntiFire, a
  pick in custom/picks) is gray where EXE5's is lit;
- a soul's buster shot's flame is whiter for a few frames (Soul and Chaos
  Unison, the auto-battling navi's Chaos);
- an explosion's colors in two frames of AntiFire's.

The comparison needs the ROM, so it lives outside this repository, with the
lists of scenarios. The frontend's own tests (`cargo test -p nettai-render
-p nettai-frontend -p nettai-demo`) use a small synthetic asset set and a live battle
built in code, and the bundled font for the text layer (its layout and the
depth test; not its pixels, which are floating-point arithmetic).

The recorders take each picture at the traced console's own VBlank, as
its main loop leaves `main_awaitFrame`. Screenshots of the right-hand
player's console taken as the emulated pair's tick ended instead (both
recorders' before) have the previous picture in their last rows: the tick
ends at the first console's VBlank, when the second console's video is
still some seven scanlines from the picture's end (152-159), and on two
frames of the second match (7498, 25885) a whole picture off. The
comparison leaves rows 152-159 out for those (no `vblank` file beside
them).

## 5. Known gaps

- **Deliberate: Gregar's true faces on a Falzar console.** The emotion
  window shows every form's and link navi's own face, Gregar's from the
  Gregar ROM, on either console (the user's choice). The original Falzar
  console has no faces for Gregar's Crosses, Beast and link navis and shows
  the Falzar counterpart's instead (HeatCross as SpoutCross); a frame that
  shows a Gregar form or navi on a Falzar console differs there on purpose
  (the sample's HeatMan, SlashMan and ChargeMan scenarios show one on every
  frame). The headless frontend lists such places with each frame
  (`known.tsv` beside the frames: frame, rectangle, why), and the
  comparison leaves them out and counts those frames apart, not as
  regressions.

- **Deliberate: the other game's link navis' portraits.** The run
  message's portrait is the speaker's true face on either console; each
  US ROM has a black placeholder for the other game's link navis (a Falzar
  console shows HeatMan's message beside a black box). Listed with each
  frame as known, as the faces are; a Gregar console's Falzar faces in the
  emotion window are known differences the same way.
- **Deliberate: the other version's link navis' emblems.** The custom
  screen shows every navi's own emblem on either console (the same choice
  as the faces and the version chips' art). A ROM has emblems for MegaMan,
  ProtoMan and its own version's five link navis, and its table gives the
  other version's five the same pictures in their own colors (`sub_802812C`:
  a Falzar console's HeatMan has SpoutMan's emblem in HeatMan's palette).
  The pack has each navi's own, from the ROM that has it, so a frame of the
  custom screen with a link navi of the other version on the console
  differs at the emblem on purpose; it is listed as known (`known.tsv`: the
  emblem sprite's 32x32). EXE5's ROMs are the same (MegaMan's and their own
  team's six navis', 0x08023F68), for the team navis the content has.
- **Deliberate: one presentation, the Japanese games' where the releases
  differ** (the user's: "nettai should not have any region specific code").
  The pack has the Japanese ROMs' art where the US ROMs have a placeholder
  (six sprites, eleven chips' pictures: asset-formats.md §4), the content
  draws Otenko's statue and CrosOver's gun with the Japanese games' sprites
  (0C-49, 0C-0F) where the US games draw others, and EXE6's custom screen
  starts the chip window as it closes (`chip_window_at_close`), on every
  console. A US console's original shows its placeholder (a purple picture,
  a dot) or its other sprite, and its chip window waits for the navi's
  first decision; an EXE5 Japanese console's background 0x05 is the bubbles
  without the goldfish. No part of the frontend knows a console's region:
  the verification's frame comparison does (its table, consoles.tsv, by
  game, region and version), and asks the headless frontend to mark where
  it draws those things (`--mark sprite:NAME,background:NAME,chip:KEY,
  chip-window-at-close`; `marks.tsv` beside the frames: frame, rectangle,
  what), which it counts as known on that console's recordings, with the
  table's margin around each (a sprite's 48 pixels: the US's sprite there
  may reach past the Japanese one).
- **Deliberate: a version's own chips on the other version's console.**
  Each ROM draws its own version's chips and has their art again at the
  other version's counterparts: EXE6's five Giga chips a version (Bass and
  BassAnly, BigHook and MetrKnuk, DeltaRay and CrossDiv, ColForce and
  HubBatc, BugRSwrd and BgDthThd share one picture, palette and icon in a
  ROM); EXE5's five a version and its phoenix (Team ProtoMan's ROM draws
  MetrKnuk and CrossDiv as HolyDrem, OmegaRkt and BugCharg as BigHook,
  BassAnly as Bass, Phoenix as DethPhnx; Team Colonel's the other way).
  The pack has each chip's picture and icon once, from its own version's
  ROM (the one whose library lists it), marked with that version
  (`ChipArt::version`), and the frontend and the editor show it on either
  console (the user's choice: a version's chips show that version's art
  always). A console of the other version shows the counterpart's there:
  the chip window's picture, the icons in the custom screen's slots and
  column, and the icons over the navi are listed as known
  (`lookups::other_versions_icon`).
- **Deliberate: a soul's flying icon on the other version's console.** As a
  Soul Unison is chosen the soul's icon flies onto the first cell in the
  soul button's icons' palette, of which each EXE5 ROM has its own (Team
  Colonel's outline is another color) and in which it draws every soul's
  icon. The frontend draws each soul's in the soul's own version's on either
  console, as a version's chips show that version's art (the user's choice):
  so nothing of an EXE5 match needs a console's version to draw it. A soul's
  version is its place's among MegaMan's souls (`forms.souls` lists Team
  ProtoMan's six, then Team Colonel's: `custom::soul_palette_row`); no
  field restates it. A console of the other version than the soul's shows
  the icon in its own outline: listed as known
  (`custom::OTHER_VERSIONS_SOUL`), as is a MeddySoul capsule's icon, which
  flies in the same palette.
- **Deliberate: the Gregar and Falzar chips' pictures.** Each Japanese ROM
  has one picture for both chips, its own beast; the pack has each chip's
  own (Gregar's from the Japanese Gregar ROM, Falzar's from the Japanese
  Falzar ROM), on either console. A Japanese Falzar console's Gregar chip
  (and a Gregar console's Falzar chip) is a known difference, the
  verification's table's as the other region's.
- **Deliberate: the Gregar and Falzar chips' descriptions.** Their scripts
  copy the text from a buffer the console keeps (`FF 01 01`, the same
  buffer for both), which holds the console's own Giga chip's: a Falzar
  console shows "Falzar's ruinous tornado!" for the Gregar chip too. The
  frontend shows each chip's own (its definition's, the user's text), so
  the Gregar chip's description differs on a Falzar console (and the
  Falzar chip's, presumably, on a Gregar console). Not listed as known: no
  content says which chips copy that buffer.
- An EXE5 console in Japanese: the EXE5 pack has the US ROMs' lettering
  alone (docs/design/exe5-map.md §11), so `--lang ja --text original`
  draws its Japanese names in the US font (the font mode draws them); the
  static audit says so, not counted.
- Live play shows the custom screen as text.
- One blend register for every semi-transparent sprite. `sprite_setAlpha`
  (EXE6's 0x08002C7A, EXE5's 0x08002AE6) writes the sprite's alpha straight
  into BLDALPHA, which the whole screen shares: on the original every
  semi-transparent sprite on a frame is blended by the alpha written last
  that tick (the last such sprite's object to update), whatever its own. The
  renderer blends each sprite by its own alpha (`compose::blend`, with the
  register's five-bit weights). The two differ only on a frame that shows
  two semi-transparent sprites whose alphas differ; no recording compared
  so far has one (the frame comparisons would show it as one of the two
  sprites too faint or too strong for as long as both are up).
- An alpha of exactly 255. The compositor marks a sprite pixel that isn't
  blended with the byte 0xFF, so a sprite whose alpha is 255 is drawn
  opaque; the hardware adds it to what is behind in full (both weights
  over 16). No object sets an alpha of 255.
- Affine (rotated or scaled) object sprites (`sprite_makeScalable`: no kind
  in the engine or the content uses one yet; compose draws affine parts, the
  custom screen's emblem is one), the per-part palette override
  of `sub_3006440`, and the original's sprite block bits 0x20/0x40.
- The HUD's other text lines: the multiple deletions of virus battles,
  "SHUFFLE!" and "PENALTY!!" (the custom screen's).
- Not drawn, as only viruses' code or no netbattle reaches them: an HP
  number at a fixed place (`sub_801DCFC`) or moved (`sub_801DCCC`), the
  training viruses' (NameIDs 0x49..=0x4E: moved 32 pixels left), and a
  warning marker within 16 pixels left of or above the screen, for which
  the original writes a garbled sprite.
- The background scroll starts one frame earlier in the first round of a
  set than in later ones (measured).

## 6. Match files

A match file is everything a round needs, chosen before the battle: the
game (which is its rules: a game has one ruleset, so a match names none),
its rounds (each a `[[round]]` table: the place it is fought on), and each side's navi, folder, SP deletion times, patch cards and
NaviCust, and what its game's rules take of it besides (its facts: what the
save brings, the base HP, the Regular memory and the navi's level; EXE6's
version, Crosses and Beast Out; EXE5's karma and souls), in TOML. A side
states no stats: the rules build them. **A match is of one game**, named
once at the file's top: everything else is a name in that game's namespace
(`cannon`, `megaman`, `netbattle-43`), looked up there alone
(`nettai_match::ids`), so a match can't name another game's chip, navi,
soul, patch card or stage: a name the game hasn't is said as any unknown
name is ("left: folder entry 3: no chip \"darkthnd\" in exe6"), whether
another game has it or not. `--match FILE` plays one (you are its left
side), `--save-match FILE` writes the match played, and the editor (nettai-demo
with nothing to play, or `--edit FILE`) makes and edits them (README.md, "The
match editor"). The crate `nettai-match`
reads, checks and writes them, and builds the round (`Match::round`); the
editor's random pick is a match too (`nettai_match::pick::live`), so a random
setup written out and played again is the same battle (the frontend's test
`a_saved_match_plays_the_same_battle` compares the digest every tick).

```toml
game = "exe6"                               # the match's game and its rules: everything below is its
seed = 42                                  # optional: the setup's and battle's seed

[[round]]                                  # the set's rounds, a table each, in order: as many as the set has
stage = "netbattle-43"                     # (1 to 99; three: the original's triple battle). A round's stage (a
background = "lans-hp"                     # link battle stage of the game's) and background (of its pack); a part
                                           # left out is picked from the seed, as the game picks a link battle's
[[round]]
stage = "netbattle-12"

[[round]]                                  # (picked from the seed: stage and background)

[left]                                     # you (side 0); then [right]: the side's facts (see below)
navi = "megaman"                           # the navi (stated: a side states its own)
version = "gregar"                         # EXE6's: version, gregar or falzar (stated: none is assumed), crosses, up
crosses = ["heatcross", "spoutcross"]      # to five of either version (else none), beast_out, else unlocked (the
beast_out = false                          # save's flag 0xE0), bug_frags, else 0
bug_frags = 0
hp = 1000                                  # what the save brings to the stats: MegaMan's base HP (else 100), the
reg_up = 50                                # Regular memory (else the fresh stats' 4), the sun (else none); the rules
sun = true                                 # build the rest (below)
level = 0                                  # the navi code's level, 0-14: a link navi's always, MegaMan's optional
regular_chip = 4                           # the Regular chip's entry, counting from 0 (else none)
tag_chips = [5, 6]                         # the tag chips' entries (else none; EXE6's)
navicust_expansions = 2                    # MegaMan's NaviCust's board, 0 (4x4) to 2 (5x5, the default)
folder = [                                 # up to 30 entries, { chip, code } ({} empty: a folder being made)
    { chip = "cannon", code = "A" },
    { chip = "cannon", code = "A" },
    { chip = "airshot", code = "*" },
]
patch_cards = ["canodumb", "shadow"]       # the installed cards, in the order they apply (each applies)
sp_times = [                               # how long the save took to delete each SP navi, by its SP chip,
    { chip = "heatman-sp", frames = 741 },  # in frames (60 a second; else every SP navi in no time)
]
navicust_programs = [                      # MegaMan's NaviCust's programs, in the save's order; x, y the center
    { program = "suprarmr", color = "red", x = 2, y = 3, rotation = 0 },    # on the 7x7 grid; quarter turns
    { program = "undersht", color = "white", x = 5, y = 3, rotation = 1 },
    { program = "hp-100", color = "pink", x = 3, y = 1, rotation = 0, compressed = true },
]
```

**A side's facts.** What a side brings that its game's rules take is the
game's own to say: its rules declare a `setup` (content/exe6/rules/init.luau:
`setup = { version = { "gregar", "falzar" }, crosses = "form[5]", ... }`;
content/exe5/rules/init.luau: `setup = { karma = "u16", ... }`), and every field of that setup is a fact a side of that
game may state, as a key of its table by the field's name. `nettai-match`
names none of them (`nettai_match::facts`): a side holds its facts as that
setup block, the round's setup hands the engine the block as it is, and a
game that declares another fact has it in its match files, its
descriptions and the editor without a line of Rust.

- A fact's value is its field's type's: a flag `true` or `false`; a whole
  number in the type's range; an enum's variant by the name the rules give
  it (`version = "falzar"`); a definition by its name in the game
  (`"heatcross"`); a chip code by its letter (`"A"`, `"*"`); a list as a
  TOML array, no longer than it holds, the entries past the last given
  empty (`souls = []`: none); a record as a table of its fields, a field
  left out holding nothing (false, none, zero), and a list of records an
  array of tables, a line each, written after the side's other facts.
- Every part of a side is a fact (step c3 of the rules-in-Luau work): its
  navi, its folder, its Regular and tag chips, its patch cards, its
  NaviCust. The engine knows the navi and the folder parts by role (the
  round's stats are the navi's, its battle folder is dealt from the folder);
  what the rest are is the game's rules'.
- A fact a file leaves out is what the rules say a side that says nothing
  has (`setup_defaults`: EXE6's Beast Out; EXE5's
  every soul, both unisons and a fresh save's karma, 500), else zero. A
  file is written with only the facts that differ from that.
- **The facts' order is their names'**: a setup's layout keeps its
  fields sorted by name, whatever order its `setup` tables write them in
  (and whichever part declares each). A file writes its facts in that
  order, `describe` says them in it, and a round's setup holds the block
  in it. So renaming a setup field can move it within the block: anything that lays a block out by position
  (a binary form of a match, docs/design/match-binary.md when it comes)
  changes with it, and the content hash (which covers every key) says so.
- An enum without a default is **required**: nothing fills one in.
  EXE6's `version`, `falzar` or `gregar` (its Beast, its pictures and its
  navi's version byte), is one. The engine itself starts no round whose
  player's setup leaves it unstated, and a file without one is refused
  with what is missing ("left: no version: a side of exe6 states its own
  (gregar or falzar); none is assumed"). A new match's sides have none
  until they are given theirs: the editor shows nothing chosen; a random
  match picks each side's from its seed and writes it.
- A list a side leaves out is its default, else empty: EXE6's `crosses`
  left out is no Crosses (nothing is read as "its version's own five";
  the editor's "Its version's own" states those, and a random match the
  five it picked).
- What is refused (`nettai_match::facts::check`, and the file's reader): a
  key the rules' setup doesn't declare ("left: no field \"karm\" (a side of
  exe5 takes chaos_unison, hp, karma, level, reg_up, soul_unison, souls)"); a value that isn't
  the field's type's ("karma: 70000 is past a u16 (0 to 65535)", "version:
  no \"azure\" (gregar or falzar)"); a name the game hasn't; a definition
  twice in a list, or an empty entry before one (a list is filled from
  the front); and, for the one list the engine knows by its role (its
  form list, EXE6's `crosses`), a form that is none of the side's navi's
  own lists. What a value means is the rules' alone: no range is checked
  beyond the type's.
- Facts known by role (`PlayerFact`): the version, Beast Out and the form
  list, which the battle and its frontend read; the level and MegaMan's base
  HP, which tools read (the checks, the editor). Where a tool needs one it
  asks by the role: a navi's version byte in its stats, the forms a random
  match's form list is picked from, the level's range.

EXE6's facts: `version` (required); `crosses` (none unless stated), up to five
Crosses of either version for the Cross window, in its order (a save's
are those of its version's five it owns, which the original keeps as
five flags and a save import writes as the list: `[]` for a save that
owns none); `beast_out`; `bug_frags`. EXE5's: `karma` (the
light/dark value; dark under 470), `souls` (up to sixteen, either
version's; every soul unless said), `soul_unison` and `chaos_unison` (the
save's event flags 0 and 0x236). An EXE5 match has no
version: Team ProtoMan and Team Colonel play alike (its rules take none: a
side may hold either version's souls and chips), so a `version` in an EXE5
match is refused as any key its rules don't declare, and its sides bring
none to a battle (their navis' version byte is 0, as an EXE5 recording's).
A version is still read at the edges that have one: a save's, a recording's
console's.

An EXE5 match (`game = "exe5"`) names EXE5's navis, chips, patch cards and
NaviCust programs, and its sides may say besides:

```toml
[left]
karma = 100                                # a fact: the light/dark value (default 500; dark under 470)
souls = ["protosoul", "colonelsoul"]       # a fact: the souls it has, EXE5's, either version (default: every soul)

auto_battle_places = [                    # what a navi in auto battle plays from the side's save: the block's 42
    {}, {}, {},                            # places, to the last that isn't empty (else none): a chip, a pattern
    { chip = "sword" }, { chip = "sword" },   # record by number (1 to 8), a 0 or {} (empty)
    { pattern = 1 },
]
auto_battle_records = [                    # its eight records (else every record zeros: nothing learned)
    { dx = -2, chips = [{ chip = "sword" }, { chip = "wideswrd" }, {}, {}, {}], score = 10 },
]
```

**A side states no stats.** A round starts from each navi's fresh stats
(`NaviStats::fresh`, `init_8013B64`: what a new save gives the navi, by its
game's `fresh_stats` rules and its own definition), and its game's rules
build the rest as the round is set up (`round_setup`), in the order their code says:

- **the save module** (content/exe6/rules/save, content/exe5/rules/save),
  first: what the save brings that nothing derives, its facts, `hp` (the
  base HP of the navi that compiles a NaviCust: MegaMan's, which HP
  Memories raise; default 100), `reg_up` (the Regular memory; default the
  fresh stats' 4) and, in EXE6, `sun` (default none; EXE5's block has no
  sun), and MegaMan's variant (+0x2B, his base HP in hundreds); a link
  navi's stats at its `level`, as the PET's reload gives them
  (docs/engine/link-navis.md: the base HP of the cleared game and the
  level's HP, buster levels, custom and Mega levels and abilities); an EXE5
  team navi's HP, the story's at its level;
- the NaviCust's compile (MegaMan's: the maximum HP the base and the HP
  programs, the abilities, levels, weapons and bugs) and MegaMan's navi
  code level's gains;
- the patch cards.

The navi's version byte (+0x20) is the version the side states, which the
battle's start sets. Another navi than the one that compiles a NaviCust
takes its HP from its level, so the checks refuse a side of one that
states `hp`. The editor's stats pane shows the stats a round starts with,
and nothing of them is edited: a test that needs an odd stat pokes the
built `RoundSetup`. (A recording's stats are its console's block:
exe6-compat and exe5-compat state the save's facts from it, start the sides
the rules build, a link or team navi or a compiled MegaMan, from their fresh
stats, and the replay compares what the rules built with the block.)

**The emotion window's glitch** (the save's event flag 0x1720, 0x1723
with patch cards; EXE5's 0x10C1 and 0x10C4: MegaMan's window flickers) is
no key of a match and no field of a setup: the game's rules make it as the
round is set up. The NaviCust's compile sets it when a bug applies, the
patch cards' routine from the stats they leave, and for a setup with no
NaviCust (a recording's, whose stats are as a compile left them) it is set
when the stats carry a NaviCust bug
(content/exelib/navicust/compile.luau). BugFix clears it.

**The navi code's level** (`level`) is the level of the navi code the
save received (docs/engine/link-navis.md), 0 to 14: a fact of EXE6's save
part (`level = "u8?"`, a number or none). The engine knows no level: EXE6's
rules read it (`exe6.navi_level`, its API module): the seal below, the
reload, MegaMan's gains, and what a level gives a link navi in battle (its
chip bonus, its charged chips, ChargeMan's Fire charge: rules/by_level.luau,
the navi's functions of its side, which the round's setup asks once,
`Battle::given`). Tools find the fact by its role (`PlayerFact::Level`). A link navi exists only
through its code, so it always has one, and a side of one states it:
nothing fills one in ("right: ProtoMan has no level (0 to 14): a link navi
exists only through its navi code"); tools state it (the editor 0 when the
navi is picked, a random match what it picked, an import the save's). At
0 its stats are its reload's at level 0 and its chip bonus is level 0's.
MegaMan without `level` has **none** (no code received, 0xFF in the
battle); with one he was received from a navi code: his level's gains go
over his NaviCust, and, as the game's event flag 0x163 does, his custom
screen has no Beast Out button and his Cross window stays his even with a
gauge for each player. The checks refuse a level past 14 and a link navi
without one; a file is written with the level whenever the side has one.
A navi takes a level where its definition says what one gives it
(`levels`: EXE6's MegaMan and link navis; `story`: EXE5's team navis); the
checks refuse a level for any other (EXE5's MegaMan), and one past the
navi's last.

**An EXE5 team navi's level** (`level`, 0 to 6, the rules' fact; docs/design/exe5-map.md
§15.16) is the level its attacks' damage goes by: the count of the save's
story flags, which the battle's init exchange sends. A side that operates
one (`navi = "protoman"`: any of the twelve, of either version) always
states a level (nothing fills one in: "right: ProtoMan has no level (0 to
6)"; tools state 0 for a new side), and its stats are the navi's fresh stats
with the HP the story gives at that level (the navi's `story`, which EXE5's
save module reads: a level below 6 is the story's progress, and at 6 the
story is taken as done). A side states no HP of its own for one: the checks
refuse a level past 6, and an `hp`. A team navi has no NaviCust, patch
cards or souls: they are MegaMan's. EXE5's MegaMan takes no level.

**The SP deletion times** (`sp_times`) are a fact of the match's
rules like any other (`sp_times`, a list of `{ chip, frames }` records: the
engine knows it as `PlayerFact::SpTimes`), an entry each SP navi chip, by
the chip's name, the frames the save took to delete its navi. A side that
states none has the rules' default, every SP chip of its game in no time;
one that states some has those alone (a chip it leaves out: in no time). The
editor shows each as the game does, `mm:ss.cc` rounded down to the
hundredth (`sub_8000D84`), and a time typed there is the fewest frames that
show as it. The SP navi chips' damage goes by them (rules/sp_chips.luau,
`sub_8010AE4`). A fact that is a record is a table of its fields, and a list
of records an array of tables, as above.

**A save** (the editor's "Import from save…", `Match::import_save`, into a
match of the save's game: a save of the other game makes a new match of its
game first; `nettai_match::import::exe6` and `::exe5`) gives a whole side.
Of EXE6, a .sav as an emulator keeps it or a raw image as Tango's netplay
templates hold, read by `exe6_compat::save` (a Japanese image's region from
the US's 0x414C sits 0x40 earlier): the navi it operates; its version,
Beast Out and the Crosses it owns (its `crosses`: those of its version's
five its flags own, in the Cross numbers' order); the navi code's level (a
link navi keeps the side's when the save has no code); its equipped folder
with its Regular and tag chips (the operated navi's block's); MegaMan's
NaviCust (its ExpMemry's board and its programs as placed, compressed by
their event flags) and patch cards (those switched on; a link navi has
neither); what it brings to the stats (`hp`, MegaMan's base HP; `reg_up`
and `sun`, the operated navi's block's); its BugFrags; and the SP times.
A test in the verification workspace imports Tango's four EXE6 and eight
EXE5 raw saves (the match's checks accept each, and its round starts), and
holds a side imported from a save the chip lab recorded on to the
recording's round: the stats it starts with, the folder's chips and the
facts both state.

**The NaviCust** (`navicust_expansions` and `navicust_programs`,
docs/design/navicust.md) is the programs placed on MegaMan's grid, by name
and color name (one of the program's `colors`), and its board; without
them, none on the largest board. The game's
`navicust` part makes the stats it gives (the maximum HP, the abilities,
levels, weapons and bugs) from the programs as the round is set up, over
what the save brings (`hp`, `reg_up`, `sun`). The editor's NaviCust pane (its
own grid, docs/design/navicust.md) places the programs on the board as the
game does, and shows what they make.

**The karma** (`karma`, `nettai_match::facts`) is EXE5's light/dark value
(NaviStats +0x44), 0 to 1000; without it, **500**, a fresh save's
(0x08010C00): light for the chips, the starting mood 0x80, no holy panels
cleared. Under 470 a dark MegaMan (mood 0, the dark face and palette, dark
chips usable in a link battle, no soul button); 499 or under clears holy
panels; under 500 he starts worried; 1000 the brightest (mood 190, Tango's
light templates). Like EXE6's `version`, `crosses` and `beast_out` (S6c's
facts), the round's setup writes it into the rules' setup, where they declare
the field (`PlayerSetup::set_fact`): EXE5's light and dark module's
`karma`. A game whose rules take none refuses one other than 500.
Hub Style (NaviStats +0x4C, which EXE5's patch card 111 sets) waits for
EXE5's patch cards. A netplay offer carries the karma and the souls,
by their names in the game, and a round's setup and the battle's digest hold
them, so both peers start alike.

**The souls** (`souls`, `nettai_match::facts`) are the souls the side has,
EXE5's Soul Unison, by name: those the custom screen's soul button may
offer. Without `souls`, every soul of the game (both versions'); with a
list, those; an empty list, none (the button never lit). The original's soul
button (0x08024B28) offers the soul of the last chip's family when the save
has it: each version's table (0x08024BF0) gives Team ProtoMan's souls 1 to 6
the event flags 2 to 7 and Team Colonel's 7 to 12 the flags 8 to 0x0D, the
other version's none, and a dark chip's Chaos Unison needs flag 0x236 too.
The engine ports that check on the souls owned: the round's setup writes
the side's into the rules' setup field `souls` (`set_fact`), and the
battle reads them as the save's flags (a soul is its form, named by its id;
the original's number for it is compat's, exe5-compat's `form_number`). A side may have
any of the game's souls, of either version (nettai's extension, as a Cross
list may name either version's), and a soul whose family the folder never
holds never comes up. Only a game whose rules take `souls` takes a list
(the checks refuse one elsewhere, and a form that is no soul).

**Soul Unison and Chaos Unison** (`soul_unison`, `chaos_unison`) are the
save's event flags 0 and 0x236: the soul button at all, and a dark chip's
Chaos Unison. Both are on unless a side says (`soul_unison = false`), as a
finished save has them; the round's setup writes them into the souls
part's setup (its defaults, on, for a setup that says nothing). A game
whose rules take neither refuses one off. The netplay offer carries them.

**An EXE5 save** (the editor's "Import from save…", `Match::import_save`,
which reads a save that isn't EXE6's as EXE5's: a .sav, or a raw save image as
Tango's netplay templates hold, read by `exe5_compat::save`) makes the match
EXE5's and gives the navi it operates, its equipped folder with its Regular
chip, MegaMan's NaviCust (the board of its ExpMemry and the programs as
placed) and patch cards (those switched on; a team navi has neither), his
base HP and the operated navi's Regular memory, its karma, the souls its
version's flags give (the forms compat names for those numbers), its Soul
Unison and Chaos Unison, and its auto battle data (the block at
save +0x554C, place for place and record for record, chips by their
numbers' names). What a match can't state of a block is left empty and
said: a chip number the game has no chip for, and an entry for a pattern
past the eighth. The editor's Auto battle pane takes that data alone from
a save ("From a save…", `nettai_match::auto_battle_of_save`), and edits
it: the 42 places in their six lists, the eight records, and the game's
chips to put in them (README.md, "The match editor"). A side that operates a team navi takes
the save's level (its story flags' count), whose HP the story gives (EXE5's
save module), and, where the save's version has the navi, the light/dark
value of the navi's own block.

**The auto battle data** (`auto_battle_places` and `auto_battle_records`,
two facts of EXE5's rules: content/exe5/rules/auto_battle/block.luau,
docs/design/exe5-map.md §15.9) is what a navi in auto battle plays from the
side's save: the Dark MegaMan that the side's
failed Chaos Unison brings, and the side's own navi under DarkInvs. EXE5
learns it from its player (the chips they use most, and the runs of chips
they use from one place) and keeps it in the save as a block of 42 places
and eight pattern records. A battle reads nearly all of that block, so a
side states its places in order (those past the list empty) and its
records (those past the list blank, 0xFF throughout: `auto_battle_records =
[]` is a save's that has never finished a battle). A side that says nothing
has what the game's battle end writes of a player it has learned nothing
of: every place empty and every record zeros; the navi in auto battle only
fires its buster between rests. A random match's sides state the data below.

The 42 places, in the six lists the game's battle end writes them in (the
lists are the rules', for what they say of a place):

| list | places | what the game writes there |
|---|---|---|
| `first` | 1 to 3 | three standard chips of a second count, which its code never raises: empty in the US saves seen |
| `standard` | 4 to 27 | its player's sixteen most used standard chips: the two most used four times each, the next two twice, the rest once |
| `mega` | 28 to 32 | the five most used mega chips |
| `giga` | 33 | the most used giga chip |
| `patterns` | 34 to 41 | the pattern records that have a score, by number, from the first |
| `program_advance` | 42 | the most used program advance |

An entry is `{ chip = "..." }`; `{ pattern = n }`, pattern record n (the
first of `auto_battle_records` is 1); `{ zero = true }`, a place holding 0
(no chip, but no empty place either); or `{}`, an empty place. The lists
are named for what the game writes there, but any entry may stand in any
place, as in the save (a save made by hand can have a giga chip among the
patterns' places).

One kind of chip is refused in a place: a chip the original can't play in
auto battle (a team navi's own chip, such as StepSwrd, and the chips
past the library, FtrSword to PnkCapsl). The AI gets in place for a chip by
the chip's positioning class, and theirs, 255, is past the game's table of
classes: the original crashes there, and the engine raises. The game's own
writer never puts one among the 42 places (it counts library chips only),
so only a block made by hand holds one, and the check says where: "place 29
of the auto battle data (`mega`, entry 2) holds StepSwrd: the original
can't play it in auto battle (...)". Which chips those are is the content's
to say (EXE5's rules' `validate`, from its own data's classes), so a navi's
own chip is covered as it lands. A pattern record may hold one (the game
writes any chip used in a run there, and a record never plays).

`auto_battle_records` is the pattern records in their order, each a table:
`dx` and `dy`, where the navi in auto battle stands from its target (`dx`
columns toward its enemies, so -2 is two columns short of the target; `dy`
rows down the screen); `chips`, its five chip places, each `{ chip = "..." }`,
`{ zero = true }` or `{}` (the chips it uses there in a row, to the first
empty place); and `score`, how the game's learning ranks the pattern (a new
pattern's is 10; one seen again in a battle gains 5, the others lose 1). A
field left out is 0. A record the game's learning never filled is zeros
(five `{ zero = true }`, all else 0); one nothing has written is `dx = -1,
dy = -1`, five `{}` and `score = 4294967295`.

The file refuses what the game can't hold: more places or records than the
block has, a place from the target or a score that doesn't fit its bytes, a
chip the game hasn't, a field a place or a record hasn't; and either fact
for a game without auto battle (EXE6). EXE5's rules' `validate` refuses a
pattern number past 8 and a place that is two things at once.

**Why all of it is stated.** The navi in auto battle plays the entries in order,
the played one going last: three times in four the first, otherwise it steps
into an enemy's row and fires its buster (three shots).

- *The places.* The order it starts with is the console's send of the data
  as the round is set up (0x0802C7BE, EXE5's rules' `send`): three swaps
  among places 1 to 3, 39 swaps among the other 39 (each swap two places
  drawn at random), then the entries packed to the front, here from the
  battle's RNG before its first tick (84 draws a side; a recording's places,
  `auto_battle_sent`, are as its console sent them, and draw none). That is no even shuffle: a place is in none of the 39 swaps about
  one time in eight, so the entry in place 4 leads what is sent about six
  times as often as another, and an empty place between two entries changes
  what a seed sends. A chip in several places is played that much more
  often.
- *A 0* is packed with the entries, not away with the empty places: it can
  come first, where a decision treats it as nothing to play (a miss, then a
  buster run).
- *The records.* The game reads a pattern's chips to the first empty chip
  place with nothing else to end them, so from a record whose five places
  are all filled it reads on: into the record's score, then the next
  record's place and chips, each as a chip's number. So a record's score,
  the record after it (named by an entry or not) and the records' order all
  show, to a navi that plays a pattern. None of the games' navis does: the
  game's test of the pattern's place (0x0802BC48) reads what its move-lag
  routine left behind, not the place, and always fails for them, so a
  pattern entry only ever costs the navi in auto battle a turn (it steps to a
  panel of its own area) and its record's contents don't play. The records
  are stated as the save has them all the same.

**What the facts leave out of the save's block**: the count at +0x54, which
the send writes, and the block's last eight bytes, which nothing reads.
`nettai_match`'s tests hold a block read into a side and written back, by
itself and through a match file, to the same places and records, over
blocks of each awkward shape.

A random match (`nettai_match::pick`, the editor's Random) states none: its
sides have the rules' default, what the game's battle end writes of a player
it has learned nothing of (empty places, zeroed records). The user: "i don't
think you need learning right? since the battles are one-off": the data
comes into a match from a match file, a save (the import) or a recording
(compat), never from the game's learning. A netplay offer carries the data.

**The checks** (`nettai_match::check`) run when a file loads, when a netplay
offer arrives (the same `check_side`), and live in the editor; each problem
is said with where it is:

- the game is one the content has, and every name names a definition of
  the game's (a stage, a background of its pack, a navi, a
  Cross, a patch card, a chip, a NaviCust program, a soul, a weapon, a
  record, a form), and every stat is in range; a match made in memory
  holding another game's (no file or offer can) is refused the same way
  ("right: a navi exe5 hasn't");
- it lists 1 to 99 rounds (a file that lists none is refused: list one
  `[[round]]` per round, an empty one picked from the seed); each stage
  stated is one a match of the game may name (its rules'
  `link_pick.match_stages`: EXE6's and EXE5's link battle stages, each a
  settings record with the link effect that isn't the random battle's;
  `link_battle_stages`);
- what the game's rules say of the side (their `validate` hook,
  `Battle::validate`, once the round is set up; nettai-match reports what it
  says and knows none of it). EXE6's and EXE5's say:
- the folder keeps the game's rules (EXE6's are rules/folder/init.luau:
  30 chips, so a folder being made, with empty entries, is no folder yet;
  copies by MB, each chip in one of its codes, at most three dark
  chips, chips the chip pack lists, the Regular chip within the Regular
  memory, the tag chips two other entries of 60 MB together at most). The
  Mega, Giga and Regular limits are the navi's stats as the round starts them
  (after the rules' `round_setup`: the NaviCust's and the patch cards' folder
  limits, as the original's folder editor and link battle check read the
  reloaded stats). Rust only asks and reports what the rules say, so
  another game's folder rules are its own Luau: EXE5's
  (content/exe5/rules/folder/init.luau, its folder editor's) are four copies
  of a Standard chip and one of a Mega, Giga or dark chip, the Mega and Giga
  levels, at most three dark chips, the chips its pack lists, the Regular
  chip within the Regular memory, and no tag chips. A folder holds the
  game's chips alone (the rules' pool is the game's). Live play's random
  folders are drawn from the rules' pool and kept only
  when their `folder_check` hook accepts them (`nettai_match::folders`);
- a NaviCust's programs only for MegaMan; its board one of the game's;
  every program in one of its colors, turned 0 to 3 quarters, fitting the
  board, over no other, the copies of one program in one color all
  compressed or all not (the save keeps one flag for them);
- a base HP (`hp`) only for the navi that compiles a NaviCust: any
  other's HP is its level's; the navi's level within its navi code's or its
  story's;
- patch cards each installed once, their MB together at most 80 (EXE6's
  menu adds none past 80 MB, `0x08141868`);
- and of nettai-match's own, generic: a side's navi stated, and its facts
  its game's rules' (`nettai_match::facts::check`, above):
  each one the rules require stated (EXE6's version and its Crosses), every
  definition the game's and once in its list with no gap before it, and
  the form list (EXE6's Crosses) forms of the navi's own lists (none for a
  navi that doesn't change form);
- karma 0 to 1000, and other than 500 only with rules that take it;
- the round starts (`Battle::new` doesn't stop).

**Netplay with a match file** (`--match FILE --host PORT` or
`--join`): the file's left side is what you bring, wherever netplay puts
you, and the host's file's rounds are the match's (the joiner's are sent,
and must be as many: the handshake stops on two numbers of rounds). The
battle's RNG still comes from both players' halves of the seed.

## 7. Embedding

nettai-frontend is a library a larger app depends on to play battles in its
own window: it has no window toolkit, no audio device and no command line in
it (`cargo tree -p nettai-frontend` names neither iced nor cpal), and
nothing in it prints or exits. Every failure is a value. The host owns the
window, the keys, the sound output and what it tells its user; nettai-demo is
one such host (its window, `window.rs`, an iced program whose frame is the
loop below; the editor is in the same window), and
`crates/nettai-frontend/examples/embed.rs` another, with no window at all.

**How nettai-demo shows the picture.** Each frame (the display's rate) it
advances the player; when a tick ran, the window's
size or the language changed, it presents into a buffer of the window's size
in logical pixels (as the earlier minifb window had it; a HiDPI display scales
it up), and the window shows that picture (`picture.rs`). On iced's GPU
renderer (wgpu) it is one texture of the window's size, written in place with
each new picture (`queue.write_texture`) and drawn with the nearest texel; on
its software renderer (tiny-skia, the fallback without a GPU) an image. (An
image widget given a new `image::Handle` each tick flickered: iced_wgpu
uploads a raster image of 2 MiB or more, the picture from 960x640 up, on a
thread of its own and draws nothing for it until that is done. Of 60
consecutive captures of a window at 960x640 playing a match, 47 showed no
picture, and 22 of 40 playing a recording; at 720x480, under the 2 MiB, none.
With the texture: none of 60, of 40, and of 30 on tiny-skia.) Memory stays
flat: a minute's play held 169 to 171 MB, and with the texture 14 seconds
held 166 to 171 MB. On a Retina display at 960x640 points tiny-skia drew 30
frames a second, the software rendering of every physical pixel being the
cost, while wgpu shows up to 120 (the display's rate), a picture presented
each tick in about 2 ms (5.7 ms when each was converted for an image).
`NETTAI_PHYSICAL_PIXELS` presents at the display's own density instead
(sharper text; four times the pixels on a Retina display: about 22 ms a
picture, 43 frames a second). `NETTAI_PLAY_STATS` prints, every two
seconds, the frames shown, the pictures presented and their cost, how long
after it was presented each picture was drawn, and for the keys pressed the
time to the tick that saw each and to that tick's picture drawn;
`NETTAI_KEY_PROBE` presses the right arrow every 300 to 400 ms (a key event
as the window gets one, at no particular point between frames) to measure
those.

**The input's latency.** The picture's widget is the play's clock and
keyboard too (`picture::surface`): it hears each redraw before that frame is
drawn, so the battle advances and the frame shows the picture it presented,
and it hears key events as they come to the window. Before, a
`window::frames` subscription heard each redraw only after it was drawn
(iced_winit broadcasts the redraw to subscriptions after drawing), so a
picture showed a frame later, and the keys came through a subscription too.
With the key probe, on a machine loaded by other builds (load average 17 to
22, 80 to 95 frames a second), from a key's event to its tick's picture
drawn took 30 ms on average then (19 to the tick that saw it) and 14 ms now
(6 to the tick); a picture is drawn 5 to 6 ms after it was presented (8 to
10 before), most of it waiting for the swap chain to free a frame. What is
left: the wait for the next tick (the battle's 59.73 a second; a tick due
just after a frame runs on the next, as with minifb), presenting the
picture (about 2 ms), that swap chain wait, and the display's next refresh
(at 120 Hz, 4 ms on average). `ICED_PRESENT_MODE=immediate` (iced's own
setting) measured no better than the default (vsync) and may tear.
(Hardware key events skip a subscription's hop now, which the probe, a
subscription itself, doesn't measure.)

**Loading a game** is one call, `game::load(name, &Options)`: the packs found
(in `Options::packs_dir`, by default where the program looks), the game's
content, its graphics and strings in the language asked for, the text's font
and, unless `sound` is off, its sound. It gives a `Loaded` or a `LoadError`
that says which step failed (`Failed::Packs`, `Content`, `NoPack`, `Part`,
`Language`, `Font`) and carries what the loaders reported up to there; a
`Loaded` carries their warnings too (`Loaded::report`). The steps are public
one by one for a host that wants them apart (`Found::find`, `Game::load`,
`Game::graphics`, `font`, `Game::sound`), as the program does, which loads no
sound for frames it only writes.

**What is played** is a driver's: `LivePlayer` plays a set of a match
(`nettai_match::Set::of(&content, &m, seed)`: a match file's, or a random
pick's, `nettai_match::pick::live`) from the local player's buttons, round
after round to the set's end; `netplay::NetPlayer` plays one against another
player over a channel the host provides (`netplay::Channel`: it sends the
frames and takes those that came, never waiting; the library has no socket).
The lobby and the handshake that agree the match are the library's,
without IO (`lobby::Lobby`: datagrams in, datagrams out), and so is the set
played from the agreement (`netplay::netplay_setup`); the transport is the
host's. The program's is nettai-demo's `net`: UDP, and the lobby the window
polls each frame, so it stays responsive while it waits. A host may bring a
driver of its own (`Driver`): the program's replays the original's
recordings.

**The player** (`Player`) is what the host drives. It owns the session, the
renderer, the font mode's text renderer and the battle's audio:

    use nettai_frontend::{Player, driver::LivePlayer, game};

    let game = game::load("exe6", &game::Options::default())?;         // a LoadError says what failed
    let m = nettai_match::pick::live(&game.game.content, "exe6", seed, None)?;
    let set = nettai_match::Set::of(&game.game.content, &m, seed);
    let mut player = Player::new(&game, Box::new(LivePlayer::new(set)));

    let mut last = Instant::now();
    let mut samples = Vec::new();
    while window.is_open() {
        // The keys are the host's: the GBA's buttons held, as a mask (nettai_battle::input::keys).
        let buttons = window.gba_buttons();
        // The time that passed: the player keeps the original's 59.7275 Hz clock and runs the ticks due.
        let now = Instant::now();
        player.advance(now - last, buttons);
        last = now;
        // The picture, into the host's own pixels (0x00RRGGBB) at any size: the battle's, nothing over it.
        player.present(window.pixels(), window.width(), window.height());
        // The sound of those ticks, as stereo samples at 32768 Hz, for the host's own output.
        samples.clear();
        player.take_samples(&mut samples);
        speaker.queue(&samples);
        if let Some(why) = player.stopped() {
            // The engine stopped, or the set is over (player.finished(), player.result()).
        }
    }

- `advance(elapsed, buttons)` runs the ticks due for the time that passed
  (at most a quarter of a second's, so a stalled host doesn't catch up) at
  the speed set, and keeps the part of a tick left over. A host that paces
  itself calls `tick(buttons)` instead, once per frame of its own clock; the
  embedding example does.
- `frame()` is the battle's 240x160 picture with nothing over it;
  `present(buffer, width, height)` scales it by the largest whole factor that
  fits and draws the font mode's text (the game's own) at the buffer's
  resolution. Neither has anything of the library's over it: the library
  composes no text and draws none.
- What a host may want to say is values, to show however it likes or not
  at all: `position()` (where playback is, in a few words), `paused()` and
  `speed()`, `stopped()` (why, once it has: the engine stopped, the input
  ran out, the set is over, the other player left), `finished()`,
  `result()` (a set's, for the local player; a netplay match's as soon as
  it is over), `diverged()` (the first difference from a recording) and
  `net_status()` (a netplay connection's figures: ping, loss, present delay,
  the last and deepest rollback and their count, frames waited).
  nettai-demo puts them in its window's title and prints a stop and a
  difference.
- `set_paused`, `slower`, `faster` and `restart` are the controls; each
  returns false and does nothing while the battle runs in real time with
  another player (`real_time()`: netplay). `set_present_delay(ticks)` is
  netplay's: the frame shown from the next frame on is that many ticks
  behind the player's newest input (false outside netplay).
- `take_samples` gives the sound of the ticks run since the last call (about
  549 samples a tick); a player made without sound gives none.
- `play(driver)` goes on with another driver in the same picture and sound (a
  recording's next round).
- `set_language(&graphics)` shows the battle in another language from the
  next frame: the host loads the game's graphics in each language it offers
  (`Game::graphics(lang)`) and keeps them. Only the drawing changes (the
  pack's lettering, the content's strings); the battle doesn't know its
  language, and the HUD's rolling numbers and timers carry on.
  `examples/language.rs` checks it on a game's
  real graphics: after a change, every picture is the one a player shown in
  that language from the start gives.

The player owns everything it needs: the graphics it draws from are shared
with the `Loaded` they came from (`Arc`), so a host keeps a player wherever
it keeps its state, for as long as it likes.

**What the library leaves to the host:** the panic hook that keeps an engine
stop off stderr (`session::quiet_engine_panics`: the session reports the
stop either way; the hook is the whole process's, so the host decides);
resampling the sound to its device's rate; where a match comes from.

**Its dependencies** are the engine's crates alone, with nettai-audio taken
without its `playback` feature. The two compat crates are in its tree through
nettai-match's save importers (a host that imports saves needs them); the
library itself reads no recording and no save.

**The proof that it embeds** is the example: it plays a seeded random match
with scripted buttons a tick at a time, with no window, and checks every
frame it takes against the PNG the program writes headless for the same seed
and buttons, byte for byte, and its samples against a second rendition from
the battle's cues. The verification workspace runs it against both games
(`tools/embed-against.sh`).

## 8. Replays

A replay is one set, all its rounds, as it was played: the match and every
tick's buttons, both players'. The simulation is a function of those alone, so
playing them again makes the same set, round by round; nothing it makes is
kept (who won, the score) but the ticks a round and the set ended on, and the
battle's digest now and then, to tell a replay that reproduces from one that
doesn't. A replay plays only on the engine and the content it was made with;
it names them, and another build or pack refuses it with what differs.

**Recording.** `nettai-demo --match FILE --record set.ntrp` (alone, or with
`--host`/`--join`) writes the set as it is played; in the library,
`Player::record(Recorder::new(sink, &content, &m, &info)?)` before the first
tick. Live play writes each tick as it runs; netplay each as it settles
(confirmed, never a prediction), so both players' replays of a match hold the
same match and ticks, and differ only in who recorded. Each tick is flushed as
it is written: a crash leaves every tick before it, and the file reads as cut
short. A round of netplay ends on the tick it ended on, which both peers know
however late they see it settle (rollback.md §4.6, "Rounds"), so the replay's
marks are the simulation's.

**Playing back.** `nettai-demo --replay set.ntrp` (`--side left|right`: the
console shown; default the recorder's; `--headless F` renders the set's ticks
F); in the library, `ReplayPlayer::new(&content, &replay, side)?` is a driver
like any other, and `replay::play_out(&content, &replay)` plays one to its end
with no picture or sound (the ticks, each round's winner, the set's result,
how the file ends, and the first difference). In a release build it plays
about 200,000 ticks a second: a 10.4-minute EXE6 set of three rounds (37,187
ticks, one side mashing) in 0.18 s, about 0.05 s for three minutes of play
(`cargo test --release -p nettai-frontend play_out_timing -- --ignored
--nocapture`); a host listing replays can afford to play each one out. A digest or a round's or the
set's end that differs from the recording is reported as a difference
(`Player::diverged`), with its tick; the simulation is side 0's on every
console, so a replay shown from side 1 compares the same digests.

**The file** (nettai-replay, which holds the format alone and runs no engine):

```text
file   := "NTRP", layout (a byte: 1), head, info, match, input
head   := "HEAD", length (u32), engine version (string), game (string),
          content hash (u64), the first battle's digest before any tick (u64)
info   := "INFO", length (u32), when (u64: unix seconds), the side that
          recorded (a byte), the players' names by side (two strings)
match  := "MTCH", length (u32), the match in nettai-match's binary
input  := "TICK", then a record per tick to the end of the file
record := control (a byte), then what its bits announce, in their order:
          bit 0: side 0's buttons (u16) follow, bit 1: side 1's,
          bit 2: the battle's digest after the tick (u64) follows,
          bit 3: a round ended on the tick (a digest always comes with it),
          bit 4: the set ended on it (a round did too; nothing comes after);
          bits 5 to 7 are 0
string := length (LEB128), UTF-8
```

Numbers are little-endian; a side's buttons are written when they change
(nothing held before the first tick, carried across a round's end); a digest
is kept every 60 ticks of the set and at each round's end (the reader takes
any). A tick that changes nothing is one byte: a three-minute round is about
15 KB. The layout byte is the container's; what a match holds is pinned by the
content hash, nothing else.

**The match** (nettai-match's `binary`, which the netplay offer uses too) is
read against the content the head names: definitions by their handles (the
pack's index, the same on every load of content of that hash), a background by
its number in the pack, nothing by name:

```text
place := stage (u16: its handle), background (u16: its number in the pack)
round := what the round states (a byte: 1 its stage, 2 its background, 3 both, 0 neither),
         then its stage's handle (u16) if stated, its background's number (u16) if stated
side  := the side's setup of the game's rules in its compact form (each fact in the
         setup's order at its width; a record's fields in order; a list as its
         count and only the entries it holds)
match := seed (u32), the rounds' count (u8), every round's place (as the seed picks
         those the match leaves), side 0, side 1
offer := the rounds' count (u8), each round as the player states it (round), then the side
```

A side's bytes are its game's rules' (a game's facts change no other game's),
a place's the engine's. A replay keeps every round's place, whatever the match
left to its seed. A random EXE6 side's offer with three stated rounds is
about 210 bytes, EXE5's about 540 (its auto battle data included); a whole
match 410 and 1,060. A reader refuses bytes that end early or run on, a handle
past the content's, a background the pack hasn't and a value no fact may hold,
saying where; the match's checks then judge what it read, as for a match file.
