# The frontend (`nettai-render` and `nettai-frontend`)

The library that runs a battle through the native engine and draws it the
way the original does, in its 240x160 frame scaled up by the largest whole
factor its host has room for. It draws from engine state: the field's
panels, each object's sprite, animation frame and look, HP, the custom
gauge, banners and the flow's screen fades. Nothing emulates the GBA's
video hardware; the frontend composes tile layers and sprite parts itself
with the original's ordering and blending rules. The strings that come
from content or vary (chip names, the telop, the chatbox, the HUD's lines)
are drawn by default with a vector font at the host's resolution over
the scaled frame (§3, "Text"); `--text original` draws them in the game's
own fonts, exactly as the original does.

It plays a set live from a player's buttons, or against another player over
the network, replays a golden trace (the recorded inputs of a real match) or
a replay (which it seeks about in, and writes as video), and draws chosen
frames to PNG. Its hosts are nettai, the app
(docs/app.md), and nettai-tools, the command line with no window
(docs/tools.md); the commands in this document are nettai-tool's.

Two crates make it up: the drawing and the playing. (Loading a game, and what
a match waits for before its first tick: §9.)

- **nettai-render** draws a battle into frames, and nothing else: it
  composes the layers (`compose`), draws the stage and the field
  (`stage`), the objects (`objects`), the HUD with its banners and telops
  (`hud`), the custom screen (`custom`) and the chatbox (`chatbox`); the
  strings in the game's fonts or as the text layer's items (`fonts`,
  `textlayer`, `strings`) and the vector font that draws those
  (`vfont`); the game's graphics, with other games' backgrounds beside
  them (`graphics`); the lookups a frame makes of the graphics and the
  content (`lookups`), each noted and checked once a run
  (`audit`); `Renderer`, which draws a frame, and `present`, which scales
  one to an output of any size, or writes it as a PNG; and a chip's
  pictures on their own (`pictures`, which the build creator shows). It has
  no window, sound, network, netplay or command line.
- **nettai-frontend** plays a battle for a host to show, as a library: it
  has no window, no audio device and no command line, and nothing in it
  prints or exits (§7). It loads a game (`game`: the content, its assets
  extracted from its ROMs, the graphics and strings in a language, the
  text's font, the sound, each step a `Result`); its player (`player`) owns the session
  (`session`), the renderer and the audio, and gives a host the picture and
  the sound as it is driven; what drives a session is a driver (`driver`:
  live play of a set from the GBA button mask; `netplay`: another player,
  over a channel the host provides: it has no socket; `replay`: a replay).
  A replay has a player of its own besides (`viewer`), which seeks (§8).
  It draws the battle's picture and nothing over it: it has no text of its
  own.

A third crate is over them: **nettai-video** writes a replay as video, an
MP4 encoded by ffmpeg on the desktop and by the browser's own encoder in a
browser (§8, "Video").

## 1. The content and the ROMs

The battle content the engine runs on is this repository's content/ (its
packs: the game packs content/exe6, content/exe5 and content/exe4 and the
support pack content/exelib, docs/design/content-model-v2.md §4.0; `--content
<dir>` or `$NETTAI_CONTENT` for another content directory). What the
frontend shows and plays is extracted from your own ROMs as the game loads,
in memory, by the game's recipe in its content (`nettai_extract`;
docs/design/extraction-recipes.md): the graphics and the sound, by the
names the content gives them. Nothing of them is written anywhere, and
nothing is checked in. A game's ROMs are every one of the collection's for
it (EXE6's: the US Falzar and Gregar, `MEGAMAN6_FXXBR6E` and
`MEGAMAN6_GXXBR5E`, and the Japanese Falzar and Gregar, `ROCKEXE6_RXXBR6J`
and `ROCKEXE6_GXXBR5J`, which have what the US release cut): a complete
Legacy Collection's. They are where `nettai_extract::RomSource` finds them:
the tools read `$NETTAI_ROMS`, a folder that is the Legacy Collection's
install or holds the twelve ROM files (any names: each file's header says
which ROM it is), else the collection where Steam installed it
(`RomSource::from_env`); the app finds the collection itself (docs/app.md
§17).

**You play one game**, EXE6, EXE5 or EXE4: a match is of one game (§6),
which a match file names (`game = "exe6"`) and a trace states (its setup
line's `"game"`: one that states none is refused; nothing takes a
recording for a game it doesn't name). There is no default game: the
frontend requires a build, a match file, a replay or a trace. The battle is
that game's content, drawn and heard from its own assets: there is no
mixing of games, no other game's chip, navi or field art (a round may be
drawn on another game's background: "The battle backgrounds" below).

`nettai_frontend::game` loads a game (`load`, or `Loading` a part at a
time: §9): `--content <dir>` names the content directory (so does
`$NETTAI_CONTENT`); without it, a content/ beside the program (a packaged
app's), else this repository's content/. A match plays one game: the
frontend loads that game's pack (its top module, init.luau, and what it
requires, with the support packs it depends on, exelib), then extracts its
assets from its ROMs (`Game::load`, `Options::roms`: none, `$NETTAI_ROMS`'s
or Steam's), nothing else; `nettai_content::index::games` lists the games
the content has. A game's chips without a use yet (a port's unwritten
chips) aren't required by its init.luau, so they don't load. Every loader
does the same (the editor, the tools' `load_battle`, the EXE5 replays, the
static audit), and the content then is what loaded: its hash, which
netplay's handshake compares, covers exactly that. The battle is the
content's alone: the assets it names, their handles and its sprites' timing
are its own (docs/design/content-model-v2.md §6.3), so a headless host
(`nettai_content::load::load_battle`: the tools' battles, the training pool)
reads no ROM, and the hash is the same whatever draws it. A frontend checks
the assets against the content as it loads the game
(`nettai_content::load::asset_numbers`): they have every asset the content
names, and each sprite the content defines with as many animations, each of
as many frames (drawing takes the graphics' frame by the simulation's
animation and frame; the content's timing times the battle), else the load
fails naming the sprite or the asset; what the game numbers each asset is
then what it draws and plays by (`AssetNumbers`). Where the content times a
sprite otherwise than the ROMs do, the game says so
(`Game::timing_differences`; `nettai-content check <game>` fails on it:
docs/tools.md). A game whose ROMs can't be found or read can't load
(`Failed::Assets`, the reason in its report). `NETTAI_LOAD_TIMES=1` prints
how long each part took.

The graphics are extracted into the types of the `nettai-assets` crate
(tiles as palette indices, colors as BGR555), and the sound into an
`m4a::SoundBank`:

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
  speed (`off_8080E34`), or a scroll that speeds up (`speeding`, read from
  the callback's code: EXE5's 0x080019EC, EXE4's the scrollers' +0xB8, §3),
  and tile/palette animations (`off_8081220`).
- **HUD**: the HP box and its digits, gauge tiles and frame, the 8x16 font
  with what each glyph draws, chip icons by chip, the HP digits shown under
  objects, the emotion window's faces and count boxes, the link navis'
  faces (each mugshot under its name), the HUD's text lines, banner layouts
  and glyphs, "Cstmzing...", "PAUSE", the warning marker's arrow.
- **The custom screen**: the window's tiles, maps and
  patches, each chip's picture (under the chip's key; for a chip whose
  palette no ROM holds, the Gregar and Falzar chips, its definition's
  `art_palette` colors it) and the buttons',
  chip codes, element icons and their colors, damage digits, the slots'
  codes and buttons, the cursor, the navis' emblems, the Regular chip's
  frame.
- **The chatbox** (in the HUD's graphics, and its portraits as sprites): the
  dialogue font with its advances, the box's tiles and maps, the key-wait
  arrow, the text's palette; each run message's speaker's portrait, the
  true face from whichever ROM has it.

What the HUD shows of the content comes from the content: a chip's name is
its definition's, spelled with the font's glyphs (`Hud::glyphs`), or in the
enhanced mode drawn with the bundled font (§3, "Text"); its icon
is the graphics' image under the chip's key; whether its damage shows is its
definition's flag. The emotion window shows the face the navi's form names
for its emotion (`mugshot`, `FormData::mugshot`), or a link navi's own
(`NaviData::mugshot`); mugshot numbers from `nettai_assets::NAVI_MUGSHOTS`
are the link navis' faces, with their Full Synchro palettes. The box beside
the face shows its side's count (the stat of the role `window_count`) where
the side's rules say so for the round (`battle.set_hud("window_count", ...)`: EXE6's
Beast Out turns, `sub_801D814`), else the empty box.

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
- Two backgrounds of one area are told apart by a story state (`mr-weather-comp-storm`), or by a part's number
  where they differ.
- Numbers whose load data, scroll callbacks and animations are the same are one background (EXE5's liberation
  maps' numbers and their areas': `acdc-area` is 0x08 and 0x0C), named for its area: one key, one picture
  extracted (the first number's), and compat/assets.toml lists every number (`acdc-area = [0x08, 0x0c]`). EXE6 and
  EXE4 have none such (verify's `areas.py --check` compares the numbers a name lists in the ROM).
- EXE5 has two that no menu name gives: the VisionBursts' maps, which the menu leaves blank and their own
  dialogue names (`visionburst`), and one no map draws, named for the battles whose settings state it
  (`nebulagray`).

"Named from" says which of these a name is, in bold where it isn't a menu name alone: those carry a judgment
(`comp`, `comp-alt`, `visionburst` and `nebulagray` the most).
"Was" is the look-name it had before.

The tables are the verification workspace's (tools/backgrounds): `areas.py` reads the areas from the ROMs,
`names.tsv` holds the names, and `rename.py` puts a changed name into the content, the generators and these
tables.

**The background pool.** The backgrounds of every game form one pool a battle of any game may be drawn on
(docs/app.md §1, "The battle's background"): `Content::background_pool`, each game's own language's
`[backgrounds]` keys (below), read with any game's content. A round's background (`BackgroundId`) is a handle
over the game's own, at their handles, then the pool's others; the renderer draws another game's from that
game's own backgrounds, extracted from its own ROMs by its own content's recipe
(`nettai_render::graphics::OtherBackgrounds`: an `OtherGame` a game). They are extracted the first time something
draws one, on a thread of their own, and kept for the run, shared by every game loaded, so a game is extracted once
(`nettai_frontend::game::others`; a game's own are given as it loads): a `Preload` asks for the other games' its
set's rounds are drawn on as it is made and waits for them before the first picture (§9), a replay's `Viewer`
before its first frame, and a still or a thumbnail on its own thread; a page's main thread never waits (it asks
`backgrounds_ready` between its frames). A round on one that isn't there (a game whose ROMs aren't found, or not
extracted yet) is drawn on its stage's own (`Graphics::round_background`). Lobbies and replays agree on the pool by
its names alone: no pixels are needed to compare or record it. Every background of
the three games draws from tile 1 below the field's tiles (163 on) in background palette 0 alone, so any draws
in any game's battle.

**The names a player sees** (the app's arenas: `Names::background`, the game's name before) are each game's locales' `[backgrounds]`,
by asset name, in English and Japanese: the area's name as the game's menu shows it, from the US ROM and from the
Japanese ROM (its first map's, the part number dropped where the key drops it: ACDC Area, 秋原エリア; EXE4's
ToyRoboComp4 keeps it), as the menu writes it, a word its twelve characters cut short too (RobCtrlComp,
GargoylComp: the user's choice, though a key spells such a word out); where no menu names it, the game's other
words (EXE5's VisionBurst, パストビジョン, from its dialogue; NebulaGray, Duo and DarkSoul from the enemies' and
souls' names; EXE4's venues' real-world map names, Stadium, AirStadium, Colosseum); and where the game has no name
at all, a translation (`comp`: Comp, 電脳; `comp-alt`: Comp (Alt), 電脳 (別色); EXE6's storm: MrWeathrCmp
(Typhoon), ウェザーくんの電脳 (台風)). names.tsv holds them, EXE5's generator writes its locales' from it, and
`areas.py --check` holds names.tsv to the ROMs' menus and every game's locales to names.tsv.

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

**EXE5** has 29 numbers and 22 backgrounds. A link battle draws one of 0x00 to 0x1A (0x08129F2C's table at
0x08129F6C, its `link_pick` rules' `backgrounds`: a merged background there as often as its numbers are).

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
| 0x0c | `acdc-area` | ACDC Area3 (0x86:0) | **the menu: 0x08's area, the same load data** | a link battle | `calendar-blue-2` (0x08's again) |
| 0x0d | `oran-area` | Oran Area3 (0x86:1) | **the menu: 0x09's area, the same load data** | a link battle | `calendar-gold-2` (0x09's again) |
| 0x0e | `scilab-area` | SciLab3 (0x86:2) | **the menu: 0x0a's area, the same load data** | a link battle | `ones-and-zeros-2` (0x0a's again) |
| 0x0f | `end-area` | End Area1, 3, 4 (0x93:0, 1, 2) | the menu | a link battle | `numbers-3` (red and green diamonds on dark red) |
| 0x10 | `ship-comp` | ShipComp1, 1, 2, 3 (0x82:0, 1, 2, 3) | the menu | a link battle | `ship-wheels` (ship's wheels on blue) |
| 0x11 | `scilab-hp` | SciLab HP (0x88:4) | the menu | a link battle | `swirls` (swirls) |
| 0x12 | `end-area` | End Area2 (0x86:3) | **the menu: 0x0f's area, the same load data** | a link battle | `numbers` (0x0f's again) |
| 0x13 | `gargcastle-hp` | GargCastleHP (0x88:5) | the menu | a link battle | `lions` (pale lion masks on cream, one lit at a time) |
| 0x14 | `end-area` | End Area5 (0x86:4) | **the menu: 0x0f's area, the same load data** | a link battle | `numbers-2` (0x0f's again) |
| 0x15 | `soulserv-comp` | SoulServComp (0x84:4) | the menu | a link battle | `crests` (dark red crests on black) |
| 0x16 | `gargoyle-comp` | GargoylComp1, 2, 2, 3 (0x83:0, 1, 2, 3) | **the menu, spelled out as the dialogue has it** | a link battle | `shuriken` (shuriken turning on purple) |
| 0x17 | `factory-comp` | FactoryComp1, 2, 3, 4 (0x84:0, 1, 2, 3) | the menu | a link battle | `microchips` (gray chips with a magenta core, coming apart) |
| 0x18 | `undernet` | Undernet1, 2, 3 (0x94:0, 1, 2); NebulaArea6 (0x94:5) | the menu | a link battle | `glyphs-2` (brown speckles) |
| 0x19 | `nebula-area` | NebulaArea2, 4 (0x94:3, 4) | the menu | a link battle | `glyphs-blue` (0x18's speckles in dark blue) |
| 0x1a | `undernet` | Undernet4 (0x86:5) | **the menu: 0x18's area, the same load data** | a link battle | `glyphs` (0x18's again) |
| 0x1b | `nebulagray` | none | **the battles that state it: no map draws it** | 2 battle settings state it (0x081140e0, 0x08114270) | unnamed (not looked at) |
| 0x1c | `nebula-area` | NebulaArea1, 3, 5 (0x86:6, 7, 8) | **the menu: 0x19's area, the same load data** |  | unnamed (0x19's again) |
<!-- /backgrounds:exe5 -->

- A liberation's map (group 0x86) draws its area's picture again: the same load data, scroll callbacks and
  animations as the area's other parts (0x0C as 0x08, 0x0D as 0x09, 0x0E as 0x0A, 0x12 and 0x14 as 0x0F, 0x1A as
  0x18, 0x1C as 0x19), so its number is its area's background (`acdc-area`, `oran-area`, `scilab-area`,
  `end-area`, `undernet`, `nebula-area`). The extractor checks that they are the same and keeps the first's.
- While its liberation runs, such a map draws another picture. The loader (0x0808C2C8) takes the load data from a
  second table (0x0808C500, where the usual one is 0x0808C578) when the map is the running mission's (0x08051982)
  and the battle isn't a link battle; the two tables differ for those seven numbers alone. The extraction takes the
  usual table's pictures, which are a link battle's.
- 0x1B and 0x1C are not among a link battle's (0x1C's picture is 0x19's, which a link battle draws). The chip
  lab draws 0x1B by patching the link pick's table on both consoles (backgrounds/0x1b, 0x1b-turns): its picture
  speeds down from the battle's first frame (§3, "Field and background").

**EXE4** has 27. Its routine is 0x08085BAC (the table at 0x08085BD0; a real-world map's 0x03), the menu's names
0x0802DEB0's (a group's first name number plus the map number, into the archive at 0x086E8770), and a battle's
settings state the background in their byte 5. A link battle draws one of 0x0803AA6C's table (0x0803AAA4: all but
0x0a, 0x11 and 0x17), its `link_pick` rules' `backgrounds`. No map draws five of them, which story battles'
settings state; the map scripts say where those battles are (a map's script starts a cutscene, the cutscene the
battle: docs/design/exe4-map.md §18 item 21). Three are the tournament venues', each named for the one map whose
scripts start its eight battles (`stadium`, `air-stadium`, `colosseum`); two are named for their battles, as EXE5's
`nebulagray` (`duo`, and `darksoul`: LaserMan's possessed navi on CastNBMComp, a map that draws `comp`).

<!-- backgrounds:exe4 -->
| Number | Name | The maps that draw it (the menu's names; map group:numbers) | Named from | Also | Was (its picture) |
|---|---|---|---|---|---|
| 0x00 | `eltwr-comp` | ElTwrComp1, 2 (0x80:0, 1) | the menu | 4 battle settings state it; a link battle | unnamed |
| 0x01 | `toyrobo-comp` | ToyRoboComp1, 2, 3 (0x81:0, 1, 2) | the menu | 4 battle settings state it; a link battle | unnamed |
| 0x02 | `lans-hp` | Lan's HP (0x88:0) | the menu | a link battle | unnamed |
| 0x03 | `comp` | MicrowvComp (0x8c:0); StereoComp (0x8c:1); HotdogComp (0x8c:2); DomeNBMComp (0x8c:3); CyberTopComp (0x8c:4); LCD Comp (0x8c:5); CastNBMComp (0x8c:6); StatueComp (0x8c:7); NupopoComp (0x8c:8); ComputerComp (0x8c:9); ToyComp (0x8c:10); ColoNBMComp (0x8c:11); LionComp (0x8c:12); DogHousComp (0x8c:13); GameComp (0x8c:14); VendMchComp (0x8c:15); CardComp (0x8d:0); WaterComp (0x8d:1); Ticket Comp (0x8d:2); StandComp (0x8d:3); AntComp1, 2, 3, 4 (0x8d:4, 5, 6, 7); BuddhaComp (0x8d:8); GoddessComp (0x8d:9); HeroComp (0x8d:10); CookComp (0x8d:11); WtrGodComp1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 (0x8e:0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15); the default: every real-world map | **no one area: the default, which the comps without their own draw** | 75 battle settings state it; a link battle | unnamed |
| 0x04 | `acdc-area` | ACDC Area1, 2, 3 (0x90:0, 1, 2) | the menu | 6 battle settings state it; a link battle | unnamed |
| 0x05 | `town-area` | Town Area1, 2, 3, 4 (0x91:0, 1, 2, 3) | the menu | 14 battle settings state it; a link battle | unnamed |
| 0x06 | `park-area` | Park Area1, 2, 3 (0x92:0, 1, 2) | the menu | 7 battle settings state it; a link battle | unnamed |
| 0x07 | `yumland-area` | YumLand Area (0x93:0) | the menu | 2 battle settings state it (0x080fad08, 0x080fad14); a link battle | unnamed |
| 0x08 | `undernet` | Undernet1, 2, 3, 4, 5, 6 (0x94:0, 1, 2, 3, 4, 5) | the menu | 42 battle settings state it; a link battle | unnamed |
| 0x09 | `darksoul` | none | **the battles that state it: no map draws it** | 2 battle settings state it (0x080fabb8, 0x080fabc4); a link battle | unnamed |
| 0x0a | `meteor-comp` | MeteorComp1, 2, 3, 4 (0x82:0, 1, 2, 3); ControlArea (0x82:4) | the menu | 2 battle settings state it (0x080fad44, 0x080fad50) | unnamed |
| 0x0b | `mayls-hp` | Mayl's HP (0x88:1) | the menu | a link battle | unnamed |
| 0x0c | `dexs-hp` | Dex's HP (0x88:2) | the menu | a link battle | unnamed |
| 0x0d | `yais-hp` | Yai's HP (0x88:3) | the menu | a link battle | unnamed |
| 0x0e | `hotel-hp` | Hotel HP (0x88:4) | the menu | a link battle | unnamed |
| 0x0f | `castillo-hp` | Castillo HP (0x88:5) | the menu | a link battle | unnamed |
| 0x10 | `jomonelec-hp` | JomonElec HP (0x88:6) | the menu | a link battle | unnamed |
| 0x11 | `black-earth` | BlackEarth1, 2 (0x94:6, 7) | the menu | 7 battle settings state it | unnamed |
| 0x12 | `toyrobo-comp4` | ToyRoboComp4 (0x81:3) | the menu | 6 battle settings state it; a link battle | unnamed |
| 0x13 | `netopia-area` | Netopia Area (0x93:1) | the menu | 5 battle settings state it; a link battle | unnamed |
| 0x14 | `netfrica-area` | NetFricaArea (0x93:2) | the menu | 4 battle settings state it; a link battle | unnamed |
| 0x15 | `sharo-area` | Sharo Area (0x93:3) | the menu | 1 battle settings state it (0x080fad2c); a link battle | unnamed |
| 0x16 | `spacectr-hp` | SpaceCtr HP (0x88:7) | the menu | a link battle | unnamed |
| 0x17 | `duo` | none | **the battles that state it: no map draws it** | 2 battle settings state it (0x080fad5c, 0x080fad68) | unnamed |
| 0x18 | `stadium` | none | **the venue's map: its scripts start the battles that state it** | 8 battle settings state it; a link battle | unnamed |
| 0x19 | `air-stadium` | none | **the venue's map: its scripts start the battles that state it** | 8 battle settings state it; a link battle | unnamed |
| 0x1a | `colosseum` | none | **the venue's map: its scripts start the battles that state it** | 8 battle settings state it; a link battle | unnamed |
<!-- /backgrounds:exe4 -->

## 2. Playing

How a player plays is the hosts': nettai's screens, keys and pad (docs/app.md
§2), and nettai-tool's frames, keys by tick and audits (docs/tools.md). What
the library plays is the same for both.

**Trace playback** runs a golden trace's rounds at the original's 59.73
frames per second, their recorded inputs driving the battle, until the input
ends or the engine hits something it doesn't implement yet; the first
difference from the trace's recorded state is reported (`Player::diverged`).
Frame numbers are the trace's.

**Live play** (a match, docs/app.md's Practice or `nettai-tool --match FILE`):
you are the left navi; the right one is an opponent's (`driver::Opponent`,
its buttons alone, as a player's are). What it does is two choices, each
made alone and changed at any tick of the set (`Player::set_opponent`,
`LivePlayer::set_opponent`; a practice menu's two rows):

- **in the fight** (`InFight`): it stands still (the default), moves around
  at random and never attacks, or attacks at random: it moves, shoots and
  charges, uses its chips and presses L and R (never START);
- **on its custom screen** (`OnCustomScreen`): it picks nothing and presses
  OK, or picks the first chip it can and presses OK (the default).

In the fight a second pad of the host's may also drive it (`InFight::SecondPad`, the host
saying what the pad holds each frame with `LivePlayer::set_second_pad`), in
the fight and on its custom screen; while the host has none, it stands
still.

Or the host's AI plays it (`InFight::Ai` in the fight, `OnCustomScreen::Ai`
on its custom screen): an `Ai` the host gives with `LivePlayer::set_ai`,
asked for the opponent's buttons a tick at a time (`fight`, with what the
local player holds; `custom_screen`) and told when the set starts over
(`reset`). The library has no AI of its own: the app's is nettai-training's
`NetAi` (the practice dummy's Fights). With none given it stands still and
picks the first chip it can.

The dummy opens the custom screen as its presses do (the user, 2026-10-24: "allow the dummy to go into custom screen.
remove that restriction it has", of the rule since 2026-10-16 that only the player opened it): one attacking at
random by its L and R, the AI as it was trained (nettai-training's `NetAi`: a while after the gauge is full, as the
env opens it), a second pad as a person does. One standing still or moving around never presses them.

Its random presses are seeded (from the set's seed in nettai) and run every
tick of the set, taken or not: what it presses on a tick goes by the seed,
the tick, the battle and its choices on that tick, whatever its choices
were before, and a set started over presses the same again. It is a
player's buttons and nothing else (a recording keeps them, so a set whose
opponent was changed on the way replays as it was played). The match sets
up the battle (§6), including its game, rounds, each side's navi, folder,
forms, patch cards, NaviCust and stats.

**A training session's options** (`driver::Training`, `Player::set_training`,
`LivePlayer::set_training`; the practice menu's Battle rows) change live
play's battle between its ticks, as a training mode does. They are a host's
options, not rules of the simulation: the driver applies them to the
battle's own typed state before each tick (`Driver::prepare`, which the
session calls before it asks for the tick's inputs; nothing from any other
driver), and each acts at once when it is set, on the battle as it stands,
paused or not. With all of them off (the default) the hook does nothing and
the battle is the set's, a function of its setup and the buttons: a test
plays a set of each game with them off beside the same set stepped as live
play stepped it before they existed, and the two agree tick for tick by
digest (`driver::tests::a_set_with_the_normal_options_is_the_set_as_before`).
A set with any of them on is no match's: it isn't recorded (`record`
refuses, and `set_training` is refused while a replay is written).

- **Refills**, by side: the navi's HP (its object's `hp`, back to its
  `max_hp`) is full again once a hit is over: `REFILL_REST` ticks (a second
  and a half) after it last lost any. The HUD's number rolls down to what a
  hit left in under a second, so a combo's damage is read before the
  refill rolls it back up, and a refill doesn't land inside a combo. A navi
  at 0 HP is being deleted and is left to it, as is a round already decided:
  a hit (or hits closer together than the rest) of its whole HP still ends
  the round. Turned on with HP missing, it is full at once.
- **The full gauge**: the custom gauge's value is at its full whenever the
  gauge runs (from each custom screen's end, not after the round is
  decided). The engine itself then flags it full and plays its sound, on the
  first tick it fills (`fill_gauge`: none while the battle is paused or
  dimmed, nor in the final turns, where a full gauge isn't flagged), so the
  game's own pause is left as it is and L or R opens the custom screen from
  the turn's start on (about a second, its banner, where without it is ten).
- **Chip draw** (`ChipDraw`), both sides: as each round shuffled the folder
  (the default), or in **folder order**, as the games deal a folder they
  don't shuffle (the battle's effect 0x80 clear, `sub_800A3E4`): from the
  folder's top as it was saved, no Regular chip first and no tag pair, what
  a screen leaves unpicked staying on top for the next. It is the battle
  folder the custom screen deals from (`custom.sides[n].folder`) put in that
  order: what is left of it, at once outside the custom screen (a screen
  being shown keeps its deal, and the folder is put in order as it closes)
  and again as each round starts; set back, what is left is put in the
  round's own shuffle again (with its Regular chip and tag pair if no screen
  has dealt yet).

Live play is a set, as a link battle is: first to `n` round wins, `n` from 1
to 10 (`nettai_match::MAX_FIRST_TO`; the original's single battle is first to
1, its triple battle first to 2), its rounds the `2n − 1` the match lists
(`nettai_match::rounds_for`; a match lists no other count; a match of one
round has the single battle's match type, `nettai_match::match_effects`: no
set, so no round number and no BATTLE 1 START! banner): when a round ends
the next one starts, on the next round's place, with the score carried and
each folder shuffled again by its console's RNG, and when the set is decided
play stops with the result (`the match is over: you won`). A set of `2n − 1`
rounds is decided as the original decides its triple battle (`sub_800AF50`,
whose 3 the engine takes as the set's count): once one side can no longer be
caught, else after its last round, drawn if the sides are level. With no
round drawn that is when a side has `n` wins; a drawn round is one of the
rounds, as in the original, so a set with one can end on fewer wins (first
to 2 after a win and two draws: 1–0) or drawn (first to 3 at 2–2 with a
draw). First to 1 and to 2 play exactly as the original's single and triple
battles (the engine's `a_set_first_to_n_ends_once_its_result_cant_change`
checks every score of first to 2 against the original's own code). Each round
opens with the BATTLE n START! banner, its number in the banner's two digits,
as every game draws it (the games have no other art for a later round: a
fourth is "BATTLE 4 START!", a nineteenth "BATTLE 19 START!"). The HUD shows
no win marks (the original shows none). How a
set goes on is `nettai_match::Set`'s to say, for live play and netplay alike
(`Set::after`: the next round's battle from the round that ended, or the
result); the session swaps the battle and tells the presentation to start
over, and the frames go on being numbered from the set's first tick.

- **The field**: each round's stage and background, as the match states
  them; a part it leaves (or a round left empty) picked from the seed as a
  link battle picks them, from the game's rule section `link_pick` (`sub_81209DC`: its
  `stages`, the stage for each index of the original's pick, EXE6's 96 link
  battle stages each once; its `backgrounds`, EXE6's `byte_8120A20`), or a
  background from the background pool, every game's, each as likely, where the
  match says so (`Match::backgrounds`, `BackgroundPick::Pool`: nettai's
  Practice and Play; the original's pick, `Game`, is a match's otherwise). Each
  round's stage and background are drawn whether stated or not, so a part a
  match states moves no other's pick, and a random match states the three
  rounds a match that states nothing would be picked; the first round's stage is one
  of the first `first_round_stages` (the count the comm menu passes for a
  triple battle's practice: EXE6's all 96, EXE5's the first 68). gen-content
  checks the lists and the count against the ROM, and verify's `link_pick`
  test checks them against every recording's rounds. (EXE5's are its own: 96
  indices over 82 stages, the first twelve records twice and twelve stages
  never, as its 0x08129F2C picks; and 27 backgrounds.)
- **A random folder for each player** (a random match's): 30 chips that keep
  EXE6's folder rules (`nettai_match::folders`: the folder editor's,
  `sub_8135080` with `sub_8135500`: copies of a chip by its MB, five up to 19
  MB down to one from 50; Mega and Giga chips within the fresh navi's levels;
  a code each chip comes in; a Regular chip within the fresh navi's Regular
  memory, if a chip fits it; no tag chips in a draw), from the chips the chip
  pack lists (Standard, Mega and Giga, not the dark chips, and not the five
  the US game has no routine for). The codes lean to two the folder favors,
  and `*`. Each console shuffles its folder from the seed at the round's
  init, as before.
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

The custom screen is the engine's (docs/engine/custom-screen.md), drawn. The
keys are the game's (A picks, B takes back, START goes to OK, UP from the top
row opens the Cross window, R describes, SELECT hides), but for the two a
match's host answers in the battle's place (`nettai_match::HOST_KEYS`, in
every match's setup: `RoundSetup::host_keys`):

- **START in the fight** pauses nothing: the game's own pause never runs.
  The player says who pressed it (`Player::take_pause_requests`, by side),
  for the host to open its own menu and to hold the ticks while it is up
  where it may (`Player::set_paused`: live play and replays; a netplay match
  runs on). It is said exactly where the original pauses (the fight running,
  the battle not over, not dimmed: docs/engine/battle-flow.md §3.5), so
  START on the custom screen is still the cursor's way to OK. (A round
  that leaves the game its START, `HostKeys::pause` off, pauses in the
  game's own pause, as a recording of the original does:
  `Battle::paused_by` says who holds it paused, output only.)
- **L on the custom screen** does nothing: nobody runs from a match, and
  the navi's message that says so isn't shown or heard
  (docs/engine/custom-screen.md §3.5). L and R in the fight ask for the
  custom screen as they did.

A match's custom screen may have a time limit (`Match::custom_screen_time`,
the lobby's Custom timer; nettai's own: docs/engine/custom-screen.md §6.1),
which its rounds' setups carry in ticks (`RoundSetup::custom_screen_limit`).
While the local player's screen counts, the HUD shows the seconds left
(`Battle::custom_time_left`) in the 8x16 font, in the HP box's plain colors
throughout, set as the custom screen's enemy name is (its cells ending at the
screen's right edge, so the same inset), on the emotion window's row, under
the opponent's name (the user: "just put it in a plain box under the name on
the right side right aligned", "align it with the emotion window", "maybe no
box and use the regular font and don't change the color", "it should have the
same offset as the enemy name"). It goes the tick the
screen closes, and a round without a limit (Practice's, a recording's) shows
none.

A recording of the original has neither: its battle pauses itself, and its L
opens the message. As in the original's netbattles, the fight gets your
buttons 4 ticks late (the link). Restarting starts the same setup over.

**Netplay** plays another player over the network, with rollback
(docs/design/rollback.md §4; the frontend's side is `netplay`), on a WebRTC
data channel (nettai-rtc, rollback.md §4.8), which the host makes: it meets
the other player in a room of the signaling server, or directly. What a
host calls, in order:

    // An invite: the server makes the room and picks its code.
    let link = nettai_rtc::Link::create_room(server, Config::default());
    // ... poll it; once link.role() is known, link.code() is the code to show.
    // Or join by a code typed (spaces, hyphens and small letters are fine):
    let link = nettai_rtc::Link::join_room(server, typed, Config::default())?;
    // Or directly (native only): Link::host(port, config)?, Link::join("host:port", config)?.

    // Once link.is_open() (the other player has come): the lobby, in the link's role. Its
    // timeout is longer than the link's 30 seconds of reconnecting. It lasts as long as the
    // two players stay together: through every match they play, and between them.
    let mut lobby = Lobby::new(role, &content, Settings::new(game, 3), side, lobby::entropy(), timeout);

    // Every frame, whatever is on screen (the lobby, the loading, the match, its result):
    while let Some(d) = link.recv()? { lobby.receive(now, d); }   // every datagram, a match's frames too
    lobby.set_link_down(link.down_for().map(|elapsed| Reconnecting { elapsed, gives_up_in: link.gives_up_in() }));
    if let Some(player) = &mut player { player.advance(elapsed, buttons); }   // the match's player, if one is up
    match lobby.poll(now) {
        Status::Pending => { /* show it: settings(), ready(), their_ready(), their_name(), ping(), problems() */ }
        Status::Loading(agreement) => {
            // The match's preload (§9), made once of agreement.set, a step or a few a frame:
            // preload.step(); lobby.set_loading(preload.fraction()); and once preload.done(), lobby.set_loaded().
            // The other player's bar is lobby.their_loading().
        }
        Status::Start(agreement) => {
            // let driver = NetPlayer::new(lobby.channel().unwrap(), agreement.side, agreement.set.clone(), options);
            // player = Some(preload.player(Box::new(driver)));
        }
        Status::Playing => {
            // The match, then its result (player.result()). When the player is done with it, or the
            // match stopped with the link still up (player.stop_reason()): player = None; lobby.reopen();
            // and the next poll says Pending: the lobby again, nobody ready.
        }
        Status::Failed(failure) => { /* say failure.reason; log failure.detail; drop the link */ }
    }
    for d in lobby.outgoing() { link.send(&d)?; }   // the lobby's, and the frame the player just made

- **In a room**, one player asks the signaling server for a room
  (`Link::create_room`): the server picks its code, six characters of
  `ABCDEFGHJKMNPQRSTUVWXYZ23456789` (no `0`, `O`, `1`, `I` or `L`), and says
  it as it lets the player in (`Link::code`). That player hosts (the left
  navi, side 0) and gives the code to the other, who joins by it
  (`Link::join_room`; `nettai_rtc::signal::normalize_code` is what a typed
  code becomes). No program makes a code up, and joining a code no room has
  makes no room: it is refused (`ErrorKind::NoSuchRoom`), as a room with two
  players is (`RoomFull`). The connection finds its
  way through NATs: each side offers its addresses and the one a STUN server
  sees, and relays through a TURN server when no direct path is found, on
  the servers the signaling server's welcome gives (Cloudflare's STUN server,
  and its TURN service's with short-lived credentials; docs/design/rollback.md
  §4.8). The signaling server is `signaling/`, a Cloudflare Worker
  (`npx wrangler dev` there runs it locally, at `ws://127.0.0.1:8787`;
  deploying it, `npx wrangler deploy`, is its owner's step). A room's
  connection checks the certificates its descriptions name.
- **Directly**, one player hosts on a UDP port of every IPv4 interface and
  waits; the other joins its address. On a LAN that is the host's machine's;
  over the Internet, forward the UDP port on the host's router to the host's
  machine, and the other player joins the router's public address. The host
  is the left navi (side 0). Nothing is exchanged before the connection: each
  side makes up the other's description, so direct connect authenticates
  nobody, as plain UDP doesn't. The joiner is the right navi (side 1), and
  sees the battle from its side: its navi on the left of the field, mirrored
  as the original's second console shows it, its own custom screen, HUD and
  sounds.
- **When the connection drops** (its state, its data channel, or nothing
  from the other side for 3 seconds), it is made again, through the room or
  to the same address: the battle stops within half a second (both players
  wait at the stall guard), the host is told how long it has been down and
  how long is left (`NetStatus::reconnecting`: its `elapsed`, and
  `gives_up_in`, which a "reconnecting" countdown shows), and when it is
  back the battle goes on where it was, in the same match (what was lost on
  the way is sent again). After 30 seconds it gives up and the match ends
  (`StopReason::ConnectionLost`).
- **What goes wrong is never a sentence to show.** Each layer gives a kind
  (an enum) for the host to say in its own words, from its catalog, and a
  detail in English for its log: nettai-rtc's `Error { kind, detail }` (the
  server can't be reached, no such room, a full room, a server of another
  version, a bad address, a port that is taken, no connection made, the
  connection lost), the lobby's `Failure { reason, detail }` (another
  version of the program, another game, other content, your build or theirs
  breaking a rule, the other player left, no answer) and the match's
  `StopReason` (finished, the other player left, the connection lost,
  internal). rollback.md §4.9 lists every one, with when.
- **The lobby and the handshake** (`nettai_frontend::lobby`, rollback.md
  §4.7) check that both players run the same netplay protocol, the same
  engine's minor version (`lobby::netplay_line`: "0.7" of 0.7.3; patch
  releases play together, a minor release doesn't), the same game (a match is of one), the same content
  (`Content::hash`: the definitions, scripts and rule tables, the asset
  names and the sprites' animations' timing, all the content's) and
  the same background pool (`Compat::backgrounds`, its identity: the hash
  leaves it out, and a round's background may be any of it), and
  refuse a mismatch on both sides, each with a reason its host can say
  (`Reason::DifferentVersion`, `DifferentGame`, `DifferentContent`). A lobby
  that waits on content (the app's: `Lobby::wait_on_content`) first has its
  host bring other content of the game, or another pool, up to date on both
  sides (docs/design/content-deployment.md §5, "Netplay"). Both
  play by the game's rules (a game has one ruleset). The language is each
  player's own, and so is each player's name, which the lobby carries to be
  shown.
  - **Builds are private.** Until the handshake's Reveal a peer says
    nothing of the build its player brings: not its navi, not its version,
    no label. A build's name is never sent at all. A player may change
    their build in the lobby (`Lobby::set_side`): if they were ready they
    no longer are, and the other sees nothing else.
  - **The settings are the match's**: the round wins its set is first to
    (`Settings::first_to`, 1 to 10, a byte in the lobby's message and in the
    Hello; 1 a single battle, 2 the original's triple battle) and its
    rounds (`Settings::rounds`, `2n − 1` of them: each round's stage and
    background stated, or left to the seed; `backgrounds`, whether a
    background left to the seed is the game's link pick's or the
    background pool's, nettai's Play the pool), the ruleset
    (`ruleset`: `nettai_match::ruleset::Ruleset`, which builds may play)
    and whether the setups are open (`open_setups`: each player may be
    shown the other's build in the match; beside the ruleset and not of
    it: the user, "i think closed setup should not be part of the ruleset
    actually"; off in a new lobby: "closed setups should be open setups
    instead and it should be off by default"), and the custom screen's
    time limit (`custom_screen_time`: 30, 45, 60, 90 or 120 seconds, or
    none; a new lobby's 60, `nettai_match::DEFAULT_CUSTOM_SCREEN_TIME`;
    nettai's own, not the original's: docs/engine/custom-screen.md §6.1).
    **The host alone changes them** (the user: "only the host should be
    able to change rules"): `Lobby::set_first_to`, `set_open_setups`,
    `set_custom_screen_time`, `set_ruleset`, or `propose` for the whole,
    each false for a joiner, whose lobby changes nothing. The joiner's
    lobby takes the host's settings as they come (of a higher stamp, or
    any that differ from its own: as it comes into the lobby too), both
    show the same (`Lobby::settings`), and both ready marks clear. The
    host takes no settings from the joiner: what a joiner's lobby says of
    them only says whether it holds the host's yet, so a joiner's peer
    that says it changed them anyway (one that breaks the protocol) is
    never agreed with and comes back to the host's. They are agreed when
    both players are ready on the same, of the same stamp. A host's lobby
    starts on the settings it is made with (`Lobby::new`), so what the
    host set while waiting for the other player is what the joiner
    takes. A lobby is made with the game's rulesets
    (`Lobby::new(role, &content, rulesets, settings, ...)`,
    `nettai_match::ruleset::Rulesets::of`), which the settings' ruleset is
    read by (on the wire by names: the preset's id or the player's own
    name, one version per build, and what each category holds back by its
    items' names) and checked by.
  - **A build under the ruleset** (docs/design/app-ui.md §2.59): a build
    that holds what the ruleset doesn't allow (a chip, a navi, a NaviCust
    program, a patch card, a soul or a Cross: one problem each,
    `Wrong::NotAllowed`), or two versions' things under one version per
    build (`Wrong::Mixed`), can't be played. Nothing is taken out of a
    build: a match is played of the sides as they are, and a ruleset that
    allows no patch cards (each game's `senior` preset, Senior class) refuses a
    build that holds any, where the lobby's Patch cards off used to play
    each build without its cards.
    - `lobby.problems()` is what is wrong with this player's build as the
      settings stand, typed (`nettai_match::check::Problem`: its `what`
      is a `nettai_match::Wrong`, the kind and its numbers, to word from
      the catalogs as a build screen words them; its `text` is for a log).
      It is checked again whenever the build or the settings change. Empty:
      the build can be played.
    - `lobby.set_ready(true)` says false while there are problems, and
      the mark stays clear: the status line says why, from `problems()`.
    - **To show a build** (the player's own card in the lobby, the
      overview with its numbers): `lobby.side()` is the side, and
      `lobby.settings().checked(&content, &rulesets, &side)` a build's
      check of any side under these settings' ruleset
      (`nettai_match::build::Checked::under`: its `overview`,
      `folder_view` (held to the ruleset), `face` and `problems`, as the
      build screens use them; the same check, so the lobby and a build
      screen with the same ruleset never give two answers of one build).
      In a match and after it, show `agreement.sides`.
  - **The handshake.** Each commits (a Hello: SHA-256 of the agreed
    settings' hash and what it will reveal) before either reveals its half
    of the seed and its side (in nettai-match's binary against the content
    both play, `nettai_match::binary`: the side's facts in the order of the
    game's rules' setup, each definition by its handle, which the content
    hash pins), and each checks the other's Reveal against its commitment
    and the other's side as it checks its own player's (a build's check
    under the agreed ruleset, `Settings::checked`; a side that fails it ends
    both lobbies, `Reason::TheirBuild { problems }` here and
    `Reason::YourBuild { problems }` there). The battle's RNG comes from both
    halves, and each part the rounds leave is picked from it alike on both.
    A recording keeps every round's place.
  - **Open setups** reach the host with the agreement
    (`Agreement::open_setups`): with them the host may show its player
    both builds, without them their own only
    (`agreement.sides[agreement.side]`). Both programs hold both sides
    either way, as the shared simulation needs them. A recording keeps the
    setting: a host makes a netplay match's recorder of the agreement
    (`Recorder::of_agreement`, §8), so the replay's info says whether the
    setups were open, and whoever shows it again hides both where they
    weren't. (The recorder
    records the agreement's match; a ruleset changes no side, so a replay
    needs no word of it to play back.)
  - **Loaded before the first tick.** Agreed, the lobby says
    `Status::Loading` with the agreement, and the host makes the match
    ready: a `preload::Preload` of the agreement's set for this player's
    side (§9), stepped on the thread that will play it. It says how far it
    is (`Lobby::set_loading(preload.fraction())`) and when it is done
    (`set_loaded()`, once `preload.done()`), sees the other's
    (`their_loading()`, 0 to 1: the other player's bar on a loading
    screen), and the lobby says `Status::Start` once both have loaded: no
    tick runs before, and the player is the preload's
    (`preload.player(driver)`), over a `NetPlayer` on the match's channel
    (`Lobby::channel`). With its loading each peer says the digest of the
    match's first round at its start, and two programs that set the agreed
    match up differently stop here (`Reason::DifferentVersion`). (A lobby
    that was loading may say `Pending` again: the other player took their
    ready back in the moment the match was agreed, and both are back in
    the lobby, this player not ready.) The host must keep polling the link
    and the lobby while it loads (load on another thread, or a little a
    frame): a link nobody polls for 3 seconds reads as a dropped
    connection.
  - **Leaving** is said (`Lobby::leave`, then send what `outgoing` has),
    from the lobby, from a match or from its result: the other's lobby
    ends at once with `Reason::OpponentLeft`, and each host drops its link.
- **After a match** the two players are back in the lobby, on the same
  link and in the same room: no new invite. The lobby is the same object,
  and it carried the match (its channel, `Lobby::channel`: the frames a
  host feeds `receive`, and the player's own among `outgoing`'s, each
  marked with its match's seed). Between a match's end and the next lobby
  a host does two things: it **drops the match's player**, and it calls
  **`Lobby::reopen()`**. The next `poll` says `Pending`; the settings are
  as they were, nobody is ready (`ready()`, `their_ready()`), the player
  may bring another build (`set_side`), and READY then agrees a new match,
  of another seed, which is loaded and started as the first was.
  - *When.* Once `player.result()` is some (the set is over) and the
    player has seen the result for as long as the screen shows it; or
    when the match stopped with the link still up: the other player left
    the match (`StopReason::OpponentLeft`), the other's player went
    silent (`ConnectionLost` from the player, the link itself still up)
    or something internal. A link that gave up is the end: its error
    comes from `link.recv()`, not from the lobby.
  - *The other player comes back when they do.* One may be in the lobby
    while the other still looks at the result: the one in the lobby sees
    them not ready, and may ready, change the settings or the build
    meanwhile. The one at the result has `lobby.their_reopened()` to say
    the other is waiting. If it is true while this player's set isn't
    over, the other ended the match on its side, and the host ends it
    here too (drop the player, `reopen()`).
  - *Nothing of the match before reaches the next.* A frame of it that
    comes late is no one's; the last frame a player sent is said again by
    its lobby until the other is back too, since the other may still need
    it to end the match (rollback.md §4.7, item 5).
  - *The loop doesn't change* (above): the host goes on feeding and
    polling the lobby and sending what it has, through the match and its
    result as before and after. A host that shows a result needn't tick
    the player: the lobby keeps the link heard.
- **Playing**: the match is a set of the rounds it lists; its rounds follow one
  another (the folders shuffled again by each console's RNG). Every frame the
  host sends your buttons and shows the frame the rollback session presents.
  The other player's input is predicted until it arrives, and the frame is
  simulated again when a prediction was wrong. A cue played on a wrong
  prediction is stopped or taken back (rollback.md §3.2). There is no pause,
  speed change or restart in netplay: START in the fight is said to the
  host as in any match (`take_pause_requests`, the local player's alone, on
  the tick as it was first simulated), whose menu opens over a match that
  keeps running.
- **The input delay** is each player's own, in ticks:
  `NetOptions::input_delay`, 4 unless the player says
  (`netplay::DEFAULT_INPUT_DELAY`), from 0 to 10 (`netplay::INPUT_DELAY`,
  the range a host offers; a value outside it is brought inside), and
  `Player::set_input_delay(N)` changes it during a match. The frame shown
  is the one N ticks behind your newest input: you see your press N ticks
  later, and the other player's input has N ticks longer to arrive before
  it must be predicted, so fewer corrections show. It is getgud's present
  delay, which is what a rollback fighter calls input delay (rollback.md
  §4.10 has why they are one thing). It is never sent, and the match is the
  same match whatever each player's is.
- **The connection's figures** (`Player::net_status`, values a host words
  itself): the ping (the round trip, in milliseconds), the jitter (how
  unsteady it is), the quality the two make (`connection::Quality`, four
  levels for an indicator of four bars: excellent up to 70 ms of ping plus
  twice the jitter, good up to 135, fair up to 200, poor past that:
  rollback.md §4.11), the loss (the share of the other player's datagrams
  that were lost), the input delay, the last rollback's depth, the deepest
  and how many in all, and the frames held for clock sync or the stall
  guard; for a fuller view (the app's link panel), the latest round trip
  unsmoothed, the datagrams and bytes each way, clock sync's skew, the lead
  (this player's inputs the other's haven't matched yet) and the times the
  connection went down in the match; while the connection is down, how
  long it has been and how long is left. The lobby has the ping and the jitter before the match
  (`Lobby::ping`, and its `quality()`).
- **The end of a match**: when the set is over the result is the host's to
  show (`Player::result`), and the player goes on running until the host
  drops it. If the other player leaves the match
  (`StopReason::OpponentLeft`), nothing arrives from them for 10 seconds
  while the link is up and the set isn't over (`ConnectionLost`), or their
  input falls more than the rollback horizon behind (`Internal`), the
  match stops: `Player::stop_reason` is what to tell the player,
  `Player::stopped` what to log. In each of these the link is still up,
  and the two can go back to the lobby. A link that doesn't come back in
  30 seconds ends everything: that error is the link's
  (`ErrorKind::ConnectionLost`), which the host sees from the link.

**Sound**: the player renders each tick's sound cues with the game's sound,
its bank (`Sound { bank, songs }`; nettai-audio's `BattleAudio`), and the host plays the samples
(nettai's through the audio device, `nettai_audio::Output`; headless
rendering never plays sound). In netplay the player renders the cue actions
of each frame (plays, and cancels of cues played on a wrong prediction).
A host may play another song as the battle's music, a battle theme the
player picked (docs/app.md §1, "The battle's music"): `Sound::parts(content,
theme)` gives the parts a player's audio is made of with the theme's song in
place of the songs the round starts (`Content::battle_music`), and
`Loaded::theme` gives a game's theme with its own game's bank, which plays on
a driver of its own (`nettai_audio::with_theme`: the battle's banks by
`BankId`, a sound's song a `BankSong`, the game's own bank first and a theme's
of another game one more); `Player::set_sound_parts`
changes it while the battle runs (`BattleAudio::set_songs`: the battle's own
music playing starts again as the new song at once, the low-HP switch kept;
the result's music stays, and the next round starts with the new song).
Only the sound changes: the
battle, its cues, a replay and the wire are the same, and the cues' rollback
moves the music back to the theme where a result's music played on a wrong
prediction is taken back.
`Output` takes the samples into a lock-free ring (the device's callback
never waits on the window) and keeps about three ticks queued (50 ms): the
ticks come on the display's frames, a frame's together, and the window's
clock isn't the device's, so the callback plays up to half a percent faster
or slower to hold the queue at that target (dynamic rate control). A window
that stalls (a frame a second late) still runs it
dry: the sound fades out over 3 ms and comes back, faded in, once two ticks
are queued, and what a catching-up window then queues past 200 ms is
skipped back to the target. (Before, the queue was a locked one with no
rate control, and running dry cut the sound to silence until three ticks
were queued again.)

**Headless frames and the audits** are nettai-tool's (docs/tools.md): the
listed frames to PNGs, with the objects, the marks and the keys by tick; and
the content and trace audits of every lookup the drawing code and the audio
make.

## 3. What is drawn, and how

The drawing is nettai-render's; the modules this section names are its.

Layers, back to front: the backdrop color, the background (priority 3),
the field (priority 2), sprites of priority 2, the HUD layer (priority 1),
sprites of priority 0 (banners), and BG0 (priority 0: the custom screen's
enemy names). The HUD layer's priority is the game's (its graphics'
`hud_priority`, BG3's): EXE6's and EXE5's 1, EXE4's 0, so EXE4's HP numbers
(priority 1) are under its HP box and custom screen, as EXE6's (priority 2)
are under its. The custom screen's sprites (cursors, element, emblem, the
chips flying) take the same priority, 1 and 0.

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
- the frame's sprite budget (`objects::SpriteDraw`, the rules'
  `effects.sprite_budget`): before its parts are culled, each visible
  object's sprite reserves its frame's tiles (a frame another sprite
  reserved this frame is shared) and its palette takes a slot (an equal
  palette's is shared; a whitened sprite takes none), in draw order; a
  sprite past the tile cap or the slots isn't drawn (a battle's: 0x2FF and
  10; docs/design/exe4-map.md §18 item 62). `--objects` lists what each
  frame took and left out;
- order: parts go into depth buckets (ground y + 0x40; in a game whose
  rules say so, `effects.sprite_depth`, further by the sprite's priority:
  EXE4's, whose buckets so run out two rows above the screen's bottom)
  and come out deepest first, most recent first, so later and lower
  objects are in front; at most 128 parts; a part past the last bucket
  isn't drawn;
- white flash, alpha and mosaic (sampled as mGBA does);
- an effect the game spawns without a position (the second explosion of a
  deletion: its X and Y are memory addresses, far off the screen) isn't
  drawn (it takes its tiles and its palette's slot all the same).

**Field and background** (`stage.rs`): each panel's block by displayed type
and owner (from the viewer's side), highlights, missing panels, front
edges, the cycling panel palettes; the background's scroll and tile
animations. A speeding scroll (`nettai_assets::Speeding`: a BG1 callback
that takes a step from its counter each frame, to a top, and moves the
picture by the counter's whole pixels) runs on the background's clock, or,
`from_battle`, on the battle's ticks:
- EXE5's nebulagray (0x1B: its callback 0x080019EC) moves the picture only
  while battle flag 0x40 is set (BattleState+0x5C), which the battle's intro
  sets on the battle's first frame (0x080E06A0, where EXE6's sets 0x10) and
  which stays set through the custom screen and the pause: down a pixel a
  frame for 64 frames, then 2, then 3, and 4 a frame from the 193rd on (each
  frame's move its speed rounded up).
- EXE4's duo (0x17: the scrollers' +0xB8, Red Sun US's 0x08001F88) moves it
  from the background's load, left, a pixel a frame for 16 frames, then 2,
  then 3, and 4 a frame from the 49th on.

The chip lab's backgrounds/0x1b and 0x1b-turns (EXE5) and backgrounds/0x17
and 0x17-turns (EXE4) show them, the link pick's table patched to that
background on both consoles. The field is the match's game's: a match is of one game,
so its field draws its panels. `FieldArt` says, for each panel type and
highlight, whether the field draws it (the field's `panel_types`); one it
doesn't is the owner's normal panel,
tinted halfway to magenta, never a hole and never another game's. A tinted
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
  netbattle), among the field's sprites by depth; a side's defensive chip
  as "????" (beside the mugshot, the other side's at the right edge), in
  the HUD's plain colors whatever the HP box's: off while a message shows
  and, where the message ends once the battle is over, from then on
  (`Battle::hud_hidden`), and off from the round's result;
- the HP numbers under objects, in the console's four places
  (`Battle::hp_numbers`): the opponent's navi's, LilBoiler's damage taken
  (rolling, colored);
- the warning markers (`Battle::warnings`): the arrow over the custom
  gauge, or over a place on the field, blinking with the console's frame
  counter;
- banners with their squash and stretch; a telop (the chip's name, its
  damage and bonus, "x2"; "????" for a trap's, to the opponent) on its
  user's half of the screen (`Battle::telops_for`: the banner block's,
  and where a game's telops run apart from its banners the other
  player's on its own block, each with its block's squash; a banner may
  show beside that one); the chip the other
  player just used, named for a second (`Battle::used_chip_for`). Where
  the game loads the banners' and the telops' glyphs into sprite tiles
  they share, and one palette (the HUD's `layout.telop.tiles`: EXE4's),
  each shows what the other loaded last: `HudState` follows the loads as
  the pieces start (`SharedLoads`), and `hud::shared_glyphs` makes up the
  glyphs whose tiles another load took (a deletion banner over the other
  player's used chip turns the name's first two glyphs into pieces of the
  banner, in the banners' colors);
- the damage judge's numbers, rolling and real;
- the turn timer's text and the message's ("COUNTER HIT!") in one line of
  tiles, as the original renders them: the timer's eight glyphs over the
  message's first eight every tick the fight runs from the 15th turn, so a
  message there reads "    1    HIT!" after its first frame (on which the
  timer's row reads the message's start); the enhanced mode draws both whole;
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
docs/engine/custom-screen.md §9) and the game's custom screen graphics:

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
- a button's look is the game's, by the name its content registers the
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
- the navi's emblem is the game's under the navi's key (`lookups::emblem`),
  its palette sprite palette 11 (the cursor's and the Regular chip's
  frame's too); a navi with none shows none;
- a console's own pictures by its version (`Versioned`: a Gregar console's
  Beast, the `-gregar` assets); a form's name and colors in the
  form list's window (EXE6's Cross window) are its own version's
  (`custom::form_name_picture`: the graphics' pictures of the form's
  `version`, numbered by the form's place among that version's forms in
  its navi's list, `forms.<version>.form_list`, which is the original's
  order; the forms offered are the view's, `FormList::forms`: the
  player's form list's entries at the places offered, the setup's field
  of the role `form_list`), so a Gregar
  Cross shows Gregar's name in any window, and a window a setup's Cross
  list mixes shows each game's own; the Beast Out button, its picture in
  the chip window and the BeastOut chip's picture are of the Beast the
  navi goes into (`custom::button_pictures`: the player's version's from
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
Cross's description; L: the no-running message, in a recording of the
original alone: a match's L opens none, §2), as the original's
chatbox draws each tick: the box's map on BG0 at its opening step (row 12,
the message box or the narrower description box); the text as the line
buffer's three rows of six sprites at (51, 108), its lines 14 rows apart
(the third row of sprites is 32x8: a third line's descenders are cut, as in
the original; in the enhanced mode each line fits the open box's inside,
`text_room`); the speaker's portrait, a sprite whose animations are its
faces, stepped by its updates and tinted while it fades; the key-wait
arrow. The text is the content's strings (the chip's or the Cross's
`description`, the navi's `run_message`, in the player's language), how
far it has printed and the rest the engine's chatbox
(docs/engine/custom-screen.md §3.5).

**An EXE5 console.** In an EXE5 match the HUD, the custom screen and the
chatbox are EXE5's: its HP box, gauge, fonts, banners, emotion
window, window and chatbox, by its own tile numbers and layout
(docs/design/asset-formats.md §4, "Another game's HUD and custom screen").
What an EXE5 console does otherwise, by data, not by game:

- the emotion window: the faces EXE5's forms name bring their own box
  (MegaMan's five, Team Colonel's), and a soul's face shows the soul's
  turns left beside it (`Battle::form_turns`: the turns the rules keep for
  the button that offers a form);
- the custom screen: the special slot's button is drawn by the game's look
  of its name (Soul Unison's, its picture in the chip
  window in Chaos Unison's palette for Chaos, as a button whose view is
  `form_offer`), the cursor over OK and over
  the button where the graphics put them; the soul choice (the window whose
  view is `offer_flight`, EXE5's state 9) flies the soul's icon
  up onto the column's first cell under its flash, and the cell keeps it
  (`Battle::offer` and `Battle::offer_flight`: the offer, a soul's form, and
  the window's step, typed reads of what the view shows of the rules'
  state; the icon is the offered form's place
  in the navi's form list that holds it, the alternate's (Chaos Unison's)
  the one after the list's, `custom::offer_icon`: the game's soul icons are in
  the order the navi lists its souls, the original's soul numbers'), in the
  palette of the soul's own version on any console (Team Colonel's icons
  have another outline color, the graphics' `icon_versions`; the form's
  version is its own `version`, which a ruleset's groups read too:
  `icon_palette_row`; §5);
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
  - the button look `arm_change` (ColonelSoul's Arm Change: the game's
    look of that name, its second tile set for unavailable
    alone) with the chip it holds as a sprite over it in the HUD's icon
    palette, on the ticks the engine draws it (`Drawn::held`: while
    choosing, through a soul's choice, and in the blink's last 20 ticks
    when the column hides the icon); the column cell the chip left stays
    drawn for the tick it leaves (`ScreenLook::column_kept`);
  - the re-deal button's uses left, a digit in the chip window's damage
    cells, where the game's look says (`uses_digit`: EXE5's Shuffle;
    EXE6's ChpShufl shows none);
- the intro fades in from black in every game, as EXE5's does (EXE6's and
  EXE4's first battle of a set faded in from white: the user, "remove the
  white flash at the start of battles"). (The custom screen's close starts
  the chip window in every game, as EXE6's Japanese games and EXE5 do:
  nettai-render's `HudState`, no rule.)
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
  names; live play, the graphics' first version).

The frame comparison against EXE5's consoles (verification's
tools/frontend-compare/exe5.txt, chiplab's library-exe5) and what still
differs are in §4.

**Text** (docs/design/text-rendering.md §9). Every string goes through
`fonts.rs`: the 8x16 font's (`cell_glyphs`, `cell_text`, `draw_cell_text`,
`cell_glyph`, `layer_text`, `layer_line`: the HUD's lines, chip names,
telops, the enemy names, the Program Advance's names, "VS") and the
dialogue font's (`dialogue_glyphs`, `dialogue_text`, which composes a line
as the original's line buffer does: each glyph OR'd in at the pen, cut
past its advance but never before eight pixels). In the original
mode that is all. In the enhanced mode (the default) each of those strings
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
the battle's text as the Japanese games do, in either mode: the
graphics' Japanese lettering (the Japanese ROMs' 8x16 and dialogue fonts in
their encoding, the HUD's lines in their glyphs, the banners whose words
differ, カスタム中…, the gauge's "L or R", the chip window's pictures for
OK, the re-deal and scrap, the Cross window's names) swapped in for the
English (`Bundle::for_language`), and the content's strings from
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
description) is the content's own strings. Every game's graphics have both releases'
words, English and Japanese (its recipe reads all four ROMs; the HUD's
`language` and `languages` say so); Chinese, which no game has a font
for, keeps the graphics' own pictures with words, and its strings are drawn
in other fonts: the vector font in the enhanced mode, and in the original
mode pixel fonts into the frame, Fusion Pixel Bold 12px for the 8x16
font's strings and Fusion Pixel 12px for the chatbox's, with the game's
own glyphs for digits, Latin letters and the game's marks
(text-rendering.md §10.3, §15).

The bundled font is Noto Sans CJK (`crates/nettai-render/fonts/noto-sans-cjk`,
SIL Open Font License 1.1, its license beside it), Google Fonts' whole
variable files: one design for every language, in the cut of the language
shown (`vfont::Cut`: Noto Sans JP for English and Japanese, SC and TC for
Chinese), weight 700 for the 8x16 font's strings and 300 for the chatbox's.
Each of the two has one size: kana and kanji as tall as the capitals and
centered on them, as the original's fonts draw them, on the capitals'
baseline. A string too wide for its box is narrowed to it, never made
smaller (text-rendering.md §9.4); it never leaves its box, and its layout
never reaches the simulation. The battle's lettered pictures (the banners,
the HUD's labels, the chip window's help pictures) are drawn in the same
font, bold and narrowed, every number, the custom window's labels and the
chip codes in Bungee, bundled beside it, and EXE6's Cross names in the text
font (text-rendering.md §12, §14). The program holds Noto Sans CJK's files
and the pixel fonts as WOFF2, and unpacks each once, the first time it is
wanted (`bundled`, text-rendering.md §16).

**Enhanced and original** (`GraphicsMode`, `Renderer::set_mode`). A
renderer draws a battle in one of two modes. **Original** draws it exactly
as the original does: the text in the game's own fonts into the frame and
nothing moved that the game holds still. **Enhanced** draws the text with
the vector font (above), the HUD's and the custom screen's frames, buttons
and panels as the game's drawings at the output's resolution, in the palettes
the pictures are drawn in at the moment (text-rendering.md §13), and, unless
`Renderer::reduce_motion` says otherwise, draws smoothly what the original
itself moves; its text and its frames stay either way. It smooths only motion the original has, over the same frames,
and adds none: nothing comes in with an animation the game doesn't have.
(The HUD's intro, which brought the HUD's pieces in at each round's start,
is gone: the user, "don't animate extra slide ins that weren't present in
the original".) A motion the original already makes in whole pixels a
frame along its own path is left as it is: nothing smoother can be drawn at
60 frames a second without changing its curve or its timing. The custom
screen's window is one: every game's slides in and out 12 pixels a tick
over 10 ticks (`sub_8026B04`, `sub_8026BF4`), and Enhanced moves it exactly
so (an eased curve was tried and taken out, the user: "you should keep the
original motion"). What moves with it a part of a pixel a tick, the field,
is drawn where it truly is (the user: "custom screen slide should be
smoothed. shaking should also be smoothed if possible. smooth in-between
positions to whole pixels"); the camera's shake is whole pixels by design,
so it has no in-between places, and stays as the game draws it.

- *The custom screen's slide* (`smooth::Lift`, `objects::project_fine`).
  While the window slides, the camera's Y moves 1.5 pixels a tick (0x18000,
  16.16: 15 pixels over the 10 ticks), and the projection takes its whole
  pixels: the field and every sprite on it step 2, 1, 2, 1 pixels down
  (and up again as it closes), half a pixel low on the odd ticks. The
  window's scroll, the HP box, the emotion window, the emblem and EXE4's
  element icon move 12 pixels a tick: whole pixels, no part. On a tick
  whose camera is between whole pixels Enhanced draws the frame again
  with the field layer a pixel up (the same tiles; the sprites, the HUD
  and the background unchanged) and `present` shows it on the output rows
  whose middle, the camera's half pixel down, falls in the next frame row:
  the field half a pixel up at the output's resolution. Each sprite the
  camera moves (the objects', and the HUD's pieces placed over them: the
  HP numbers, the chip icons, the warning markers on the field) is drawn
  at its true place, the part of a pixel the projection rounds off
  (`project_fine`, `project_hud_fine`) added, as an overlay (below) read
  at the output pixels' middles too, so the navis stand on their panels
  through the slide. The path (15 pixels, linear) and the ticks are the
  game's; at three times the field moves 4 or 5 output pixels a tick, at
  two and four times exactly 3 and 6.
- *The sprites between whole pixels* (`objects::project_fine`,
  `Overlay::fine`). An object's place is 16.16 and the projection takes
  its whole pixels too, so what moves by parts of a pixel a tick steps
  unevenly on the screen; then the user: "you should also do" the navis and
  attacks. Enhanced draws every part of an object's sprite (its shadow
  among them, at the ground's part) and the HUD's pieces placed over it at
  its true place, an overlay as above, the same path and ticks; a part at
  a whole pixel stays in the frame. A part blended or in mosaic stays at
  its whole pixels (the composition mixes it with what is behind), and
  under a blended sprite in front an overlay doesn't show through; nor is
  the hardware's quirk of a transparent sprite pixel raising the priority
  of the one under it followed for overlays. What moves so (measured
  2026-10-14 on a seeded match of each game, 6,000 ticks, the left player
  throwing bombs and using TankCan, Magnum and BigHammer): the thrown
  bombs alone (MiniBomb, BlkBomb, BugBomb: 2.75 to 2.9 pixels a tick
  across, the height a parabola), 121, 137 and 156 sprite-ticks in about
  21,000, 23,000 and 16,000 drawn, and an EXE5 effect at rest a part of a
  pixel off; by the code, anything aimed by a division (`trajectory.in_ticks`,
  `bios_div`: a rock thrown, CannBall, CrakBom, CactBall, CacDanc, Meddy,
  NapalmMan, Django, FlshBom, RlngLog, VDoll, the heal supports, ChargeMan,
  HeatMan, EXE4's Binder and SeekBom), the velocities the content divides
  (RedSun, SunMoon and its meteors, SandWorm, EXE6's MegaMan code), and a
  few speeds (JudgeMan 2.25 pixels a tick, KendoMan's dash 6.67, EXE4's
  gear 0.5). The navis move a panel at a time (40 or 24 pixels), slide 10
  across and 6 or 8 in depth, or warp; the buster, Cannon and AirShot hit
  at once; most shots move 2 to 20 whole pixels a tick; sparks, effects
  and attachments are placed at whole pixels: none of them changes.

- *The custom gauge's fill* (`hud::gauge_sub`). The gauge's counter runs
  to 0x4000 over a bar of 16 cells of 8 pixels, 128 a pixel; it gains 0x20
  a tick (0x40 or 0x10 with a side's gauge speed), and the original draws
  the fill to its whole pixels, a cell's tile filling by eighths (tiles 0
  to 7 of the gauge's, then the filled one): a pixel every 4 ticks (2, 8).
  Enhanced shows the counter's part of a pixel: the frame drawn again with
  the fill a pixel longer (`Frame::nudge`, a `smooth::Nudge`), which
  `present` shows on the output pixels left of that part of each frame
  pixel (at three times, the fill grows an output pixel a tick, three of
  every four ticks). The gauge's tiles, colors, its full
  flashing and "L or R" stay the game's; a full gauge, a hidden one and a
  gauge at a whole pixel have no nudge, and one a pixel short of full shows
  its cells filled, not full, on the nudged pixels.
- *The squashed and turned sprites* (`smooth::Overlay`). The banners'
  and the telops' glyphs are squashed and stretched by the hardware's
  affine scale (`SpritePart::vscale`), and the custom screen's emblem is
  turned and scaled by its matrix as it spins; the frame reads them at its
  own pixels, so a banner a third of its height shows five or six of its
  glyphs' sixteen rows, whole rows picked, and a turned emblem its texels
  re-picked on the frame's grid. Enhanced draws these parts at the output's
  resolution instead: left out of the frame's pixels (their places in the
  sprite order kept), each output pixel reads the part's texture through
  the same scale or matrix the original uses on that tick, colored with
  the same palette and fades, wherever nothing in front of the part won
  the frame pixel (the frame's depth keys, as the text layer's items are
  hidden). The scale and the matrix on every tick are the original's: the
  curves and the timing don't change. An overlay also carries its part's
  true place, the part of a pixel the projection rounds off
  (`Overlay::fine`): a part neither squashed nor turned is one only where
  that is not nothing, and is read a texel a pixel there. (A part that is
  blended or in mosaic stays in the frame at its whole pixels: the
  composition mixes it with what is behind.) Drawn at the frame's own
  size, an overlay is the frame's part, pixel for pixel (its test). A
  squashed or turned part is read at each output pixel's top left corner,
  as the frame reads it at its pixel's, so a whole one drawn larger is the
  frame's part scaled, and on every tick the overlay draws every frame
  pixel the part's sprite draws, its first output pixel in the frame's
  texel (`hud`'s test of EXE4's BATTLE START, whose glyphs' outlines reach
  their last row: read half a frame pixel up, as it once was, the banner
  was drawn that much low and its last row cut by its box).
- *The background's glide* (`Frame::glide`, `present`). A background that
  scrolls by sixteenths of a pixel a frame (its `scroll`: the game's scroll
  counters) shows on the screen at the counter's whole pixels, a step every
  second or fourth frame; the glide shows it where the counter is. The
  renderer draws the background a pixel wider and taller than the frame
  (`Stage::draw_background_wide`: the same tiles, tile animations and
  palettes, faded as the frame's stage is) and the counter's low four bits
  (`Stage::background_sub`); `present` shows that picture, moved on by
  those sixteenths at the output's resolution, on every output pixel whose
  frame pixel the background or the backdrop won (the frame's depth keys),
  and the frame's own pixels elsewhere: what is in front of the background
  keeps its place, and text is drawn over as ever. The frame itself (its
  240x160 pixels, as `Player::frame` gives it) is unchanged. The rates, in
  pixels a frame (the backgrounds' scrolls as extracted): most areas' and
  computers' ½ across and ¼ up or down (-8, -4 in sixteenths), several
  ½ across alone (EXE6's aquarium-comp and judgetree-comp, EXE5's
  drill-comp, EXE4's air-stadium, colosseum, lans-hp and stadium), a few ¼
  up or down alone (EXE6's copybot-comp and robot-control-comp, EXE5's
  main-comp, EXE4's dexs-hp and toyrobo-comp), EXE6's graveyard 1/16
  across; the undernets and a few more hold still. A speeding scroll (EXE5's
  nebulagray, EXE4's duo) moves whole pixels every frame: no glide.

**What moves in the battle's picture** (every game's, surveyed 2026-10-14
for "what about banners and telops and stuff? can they be animated more
smoothly?"). Enhanced changes only what the original draws coarser than
its own motion (a counter's or a position's part of a pixel, a scale or a
turn the frame rounds to its rows or texels); what already moves in whole pixels or
whole steps along its own path, and what steps through pictures or
colors, it leaves as the game draws it.

| What | How the original moves it | Its steps | Enhanced |
|---|---|---|---|
| Banners: the battle's number and START, TURN n START, FINAL TURN, ENEMY DELETED, YOU WIN, YOU LOSE, DRAW, HIT DAMAGE JUDGE, PROGRAM ADVANCE | five 32x16 sprites squashed and stretched about their middle: in from a third of their height to whole, held, out back to a third (`sub_801CE28`; EXE4's 0x08014994). EXE5's and EXE6's 5 ticks in (heights a third, 0.4, a half, two thirds, whole), a bounce to 1.23 and 1.14 times at the hold's first two ticks and its last two, 0x30 ticks held, 5 out; EXE4's 9 in (by 0x40 in the texture's step), 30 held, 9 out, no bounce | the hardware's affine scale is exact; the frame shows it as whole texture rows picked a screen row each (at a third of the height, five or six of sixteen rows) | the same scale on the same ticks, drawn at the window's resolution (`Overlay`) |
| A telop: a dimming chip's name, damage, "+bonus" and "x2" (EXE5's and EXE6's own banners; EXE4's own look on the banner block, 0x0801650C) | as the banners, on the banner block or the other player's | as the banners | the name is the text layer's at the window's resolution already (its squash too); the numbers and signs as the banners |
| The other player's chip (`used_chip_parts`; EXE4's second block shown without sliding) | appears and goes | none | the game's |
| The custom screen's slide: the window | 12 pixels a tick over 10 ticks, in and out, the HP box, the emotion window, the emblem and EXE4's element icon with it | whole pixels every frame along its own line | the game's ("you should keep the original motion") |
| The custom screen's slide: the field and every sprite on it | the camera 1.5 pixels a tick down (0x18000), 15 in all, linear, and back | the camera's whole pixels: steps of 2, 1, 2, 1, half a pixel low on odd ticks | where it truly is, at the window's resolution: the field a lift, the sprites overlays ("smooth in-between positions to whole pixels") |
| The custom screen's emblem as a chip is picked (`sub_8029C08`) | a 32x32 sprite turned and scaled by a matrix for each of its 19 steps (`SPIN`), nudged a pixel or two | the matrix is exact; the frame re-picks its texels on its own grid | the same matrix on the same ticks at the window's resolution (`Overlay`) |
| The custom gauge's fill | its counter, 128 a pixel, 0x10 to 0x40 a tick | whole pixels: a pixel every 2, 4 or 8 ticks | its end where the counter is, at the window's resolution (a nudge) |
| The full gauge's flashing, "L or R", "Cstmzing..." | tile frames (the full cells' four, every 7 ticks; "L or R"'s two, every 8; the text's blink every 32) | pictures | the game's |
| The Program Advance's names | a name every 8 ticks, the recipe's colors cycling | pictures, palettes | the game's |
| The Cross window's opening, the chatbox's (a description, a message) | three and four steps of their maps; the text printed a character at a time, the portrait's animation, the arrow's frames | pictures | the game's |
| HP numbers, the judge's damage, the turn timer, "TIME UP!", "COUNTER HIT!", "PAUSE", the next chip's name, the chip icons, the warning markers | numbers roll by value, lines appear and go, the markers' two frames | none on the screen | the game's |
| The emotion window's flicker and blink | its palette black two ticks of four, left out on two | palettes | the game's |
| What moves on the field by parts of a pixel (thrown bombs and rocks, arcs, a few shots and dashes) | its 16.16 place and velocity | the place's whole pixels: uneven steps (a bomb's 2.9 pixels a tick across in steps of 3 and now and then 2) | where it truly is, at the window's resolution (overlays), but a blended or mosaic part |
| Navis, shots, sparks and effects otherwise | a panel at a time, whole-pixel speeds, placed at whole pixels | whole pixels | the game's |
| The camera's shake (hits, bombs, chips that strike the ground, a soul's or a Cross's change) | a jitter drawn each tick it shakes, by magnitude 0 to 3: -1 to 0, -2 to 1, -4 to 3, -8 to 7 pixels each way (EXE6's `byte_8030284`, EXE5's 0x08030DF4, the same in EXE4's 0x08025FE0: the RNG's bits less a whole-pixel bias) | whole pixels by design: no in-between place | the game's (nothing to smooth: no motion is made up between its frames) |
| An object's shake (a dimmed navi hit, a rock before it is thrown) | a jitter around its whole-pixel origin each tick (`AddRandomVarianceToTwoCoords`) | whole pixels | the game's |
| The fades: the intro's, a dimming's, a transformation's, the custom screen's, a palette flash | palettes toward black or white | sixteenths of the way, as the hardware's palette fade | the game's (a fade, no motion) |
| The background's scroll | its counters, sixteenths of a pixel a frame | whole pixels every second or fourth frame | the glide, at the window's resolution |

The motion is presentation alone and deterministic. The glide's sixteenths
are the background's clock's (the round's frames, as its whole pixels
are) times its rate; the gauge's fill is its counter's; a banner's scale
its step and timer, the emblem's matrix its spin's step, the slide's lift
and the sprites' true places the camera's and the objects' 16.16 places. It reads no clock
and nothing of it is in the simulation, its snapshot or its digest: a
rollback that draws a frame again, a seek, a replay and a video draw it the
same (nettai-frontend's
`the_smooth_motion_is_the_battles_alone_and_moves_nothing_the_original_doesnt`
draws every tick again from a copy of the battle, and finds the frame's own
pixels the original mode's on every tick but under the overlays, which are
left out of them for the output to draw, and the custom screen's slide
lifted on its odd ticks).

Where it is on: the app's battles (live, online, Practice, a replay
watched) and its main menu's reel, with "Battle graphics: Enhanced" (the
app's default; Reduce motion takes the motion off and keeps the text), and
a replay's video the app exports (as its settings say; a replay's still in
the list is drawn at rest); nettai-tools in its default mode (`--text
enhanced`, or `font`: headless frames, `--mp4`); and a host that
loads with the frontend's `Options::default()`. Off: `Renderer::new` and
`Renderer::with_graphics` (Original until told otherwise), so the tests;
`--text original`, which the frame comparisons against mGBA, the lab and
trace replays and the identity checks use; and every frame in the original
mode, byte for byte as before the mode had motion.

### Both on one screen

A frame shows the local side's console's screen (`Renderer::consoles`,
`Consoles::One`, what every console of the game shows), or both players on
one 240x160 screen (`Consoles::Both`: a replay watched from both sides, the
user's "dual perspective replay on a single screen"; the mockups the user
chose are nettai-verify's docs/nettai-ui/spectator). The renderer
follows both consoles' HUDs every tick (`Renderer::hud_state`, the local
console's, and `other_hud`), so either way is drawn from the same state.

The field is drawn once, as the left console draws it (the local side's:
its camera, its panels' colors, its view mirrored or not), and its sprites
are those either player sees (`Battle::visible_to` for either viewer: a
blind player's opponent, a lock-on marker, a charge glow). Each console's
pieces are drawn by `hud::draw` for its seat (`seat::Seat`): the left one's
where its console draws them; the right one's own pieces alone, mirrored
into the right half; and the pieces both consoles show alike (the custom
gauge, the banners but the round's result, the judge's numbers, the turn
timer, "COUNTER HIT!", "PAUSE") once, by the left. Mirroring is of places,
never of what reads: a frame, a box or a face is flipped at its mirrored
place, while words, digits, chip art and icons go to the mirrored place of
the box they are laid in and are laid there upright. A piece tied to an
object goes to the mirror of its place about the object's: an HP number
under one, or a warning marker over a place, where its pixels mirror,
not its box (`Seat::ink_shift`: the HP number's four cells leave a margin
by its digits' count).

- The HP boxes in the top corners, the right's flipped with its digits in
  their cells' mirrored place; the emotion windows under them, the right's
  face and box flipped so the face looks toward the middle, its count
  upright (in Original, the count's box picture flipped with its digits put
  back upright in the mirrored place of their box: the graphics' count letters
  say where they are).
- The next chip's name at the bottom left and, right-aligned, at the bottom
  right; each navi's chip icons over it, as its own console draws its own
  (an EXE5 console shows only its own navi's).
- No HP under a navi (both are in the corners), and of the HP a console
  numbers under other objects, each once.
- A defensive chip's "????" beside its own side's emotion window (the
  right's at its mirrored place), none at the edges.
- Telops on their player's side, as the left console lays them, each named
  as its user's console names it (`Battle::telops_for_both`): a chip hidden
  from the opponent alone is named, one hidden from both stays "????". The
  chip a player just used (`used_chip_for`), each on its user's side: the
  right player's as the left console names it, the left player's as the
  right console names it, at the mirrored place.
- The round's result: each player's own banner (YOU WIN!, YOU LOSE), its
  picture's columns that show centered on its half.
- The warning markers each console shows, once.

On the custom screen (both screens open on one tick, and the fight waits
for both: `seat::Shows`):

- Both players choosing (C1): both windows, the left's as its console draws
  it, the right's mirrored into the right half; the HUD's headers beside
  them (HP boxes, faces, the enemy names, the late turns' block) are
  covered; the time left of the screen that ends first between the two
  windows at the bottom (`hud::time_between`).
- One player's result sent while the other chooses (W2): the chooser's
  console's screen alone, its window and its header and all it draws (the
  other navi's HP under it too), the right's mirrored; the field moved
  down with its window and faded with its screen (`seat::field_side`).
- Between them nothing jumps: the screen that stays up longer
  (`custom::screens_left`) is drawn from the tick the other's window
  starts out, the field seen through its console, and the leaving window
  over it, which uncovers its header as it goes (`Shows::Window` over
  `Shows::Screen`, kept off the leaving window's columns: `Seat::clip`);
  windows that open or close on one tick each draw their console's screen
  in their own half (`Shows::Half`), their headers sliding to and from the
  middle. "Cstmzing…" is the left console's alone. One change is left at
  W2's end: the waiting player's header (HP box, gauge, face) shows on the
  tick the chooser's result is sent, in place of its HP under its navi.
- A mirrored window (`seat::Mirror`) is drawn as its console draws it and
  put in the right half: its frame mirrored pixel by pixel (a drawing
  flipped), and the parts that read (`custom::Window::upright`: the chip
  window with its box, which mirrors as frame about its middle, or in its
  place EXE6's Cross window's bars, each Cross's face and name upright as a
  chip's picture and name are; the hand, the buttons, OK, the labels'
  letters, by the columns the graphics say they take, so that a tab's arrow
  beside them mirrors with the frame (EXE6's CROSSSELECT), the picked
  column's icons, the screen's own emblem, the Program Advance's names, the
  enemy names) moved whole to their mirrored places (a part laid with a
  margin of the frame about it, OK in its cursor's cells, by what it draws:
  `Window::ink`, so that it keeps its margins to the flipped frame); its
  sprites (the
  cursor, the Regular chip's frame, the element, the orb, the form list's
  cursor) moved with the part under them, never flipped (one on no part,
  the orb, where its pixels mirror). A mirrored window
  sliding in or out is drawn whole where it comes to rest and slid as its
  console's slides, the other way, its columns nearest the middle first in
  (`Mirror::slid`), not cut where its console has yet to copy them. Each console's
  HUD layer and BG0 are its own (`Palettes::OtherHud`, `Plane::OtherHud`):
  shaken and faded as its console shakes and fades them, its sprites in its
  console's fade (`SpritePart::fade`).
- A chatbox the custom screen runs spans the screen: the left player's, else
  the right's, as its console draws it.

A replay's viewer shows it so with `Viewer::set_both` (the app's PLAYER
menu's Both; `nettai-tool --side both`; a video's `Sides::Both`), and it is
heard as one sound: the left console's, its music and all, with the right
side's own effects over it, the ones the right console plays that the left
doesn't on that tick (its charge, the hits it hears), each effect once
(the user's "music follows the left console"). Each side's own effects play
on a sound of their own (`viewer::SOUNDS`: each console's and each side's
own effects), and all four run every tick and are kept with each state, so
that a change into or out of Both, of the side at the left, or a seek hears
what playing straight so would.

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
  (`Battle::telops_for` is the viewer's side of it: a trap's name is hidden
  from the opponent). That a block shows a telop is simulation state where
  telops run apart from the banners (the rules' `effects.telops`).
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
with `--text original`, EXE5's assets loaded beside EXE6's): of 7,964 frames,
7,914 are pixel-exact, the navis included, and 12 scenarios wholly: the
HUD, the emotion window (a dark chip user's flicker too, now that the
console's RNG1 is the engine's: the dark chip offer, exe5-map.md §15.3
item 13), the custom screen with its picks, Soul Unison's choice and a
dark chip's hover, its close (the hand's name and icons on the tick the
results are in), the chatbox, the banners, the mercy flash and a
deletion's result. The three differences it found then (the UNITE button
for a soul not offered yet, gray where EXE5's is lit; a soul's buster
shot's flame, whiter for a few frames; an explosion's colors in two frames
of AntiFire's) are gone: the drawing sweep (verification's
tools/frontend-compare/exe5-sweep.tsv) finds custom/picks, every soul's
Soul and Chaos Unison, chaos-ai/cannon and chips/0x08c-antifire/sprung
pixel-exact.

The comparison needs the ROM, so it lives outside this repository, with the
lists of scenarios. The frontend's own tests (`cargo test -p nettai-render
-p nettai-frontend -p nettai-tools`) use a small synthetic asset set and a live battle
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
  The extraction takes each navi's own, from the ROM that has it, so a frame of the
  custom screen with a link navi of the other version on the console
  differs at the emblem on purpose; it is listed as known (`known.tsv`: the
  emblem sprite's 32x32). EXE5's ROMs are the same (MegaMan's and their own
  team's six navis', 0x08023F68), for the team navis the content has.
- **Deliberate: one presentation, whatever the console's region** (the
  user's: "nettai should not have any region specific code").
  The extraction takes the Japanese ROMs' art where the US ROMs have a placeholder
  (six sprites, fifteen chips' pictures: asset-formats.md §4) and the US
  ROMs' where both releases have their own (the user's, 2026-10-12: "you
  should use the US assets where regions differ": a pixel or two of
  BassAnly's icon and Gregar's Beast Out picture; the flames HeatMan
  wears, in his throw; EXE5's background 0x05 and its hit sound), the content
  draws Otenko's statue and CrosOver's gun with the Japanese games' sprites
  (0C-49, 0C-0F) where the US games draw others, and EXE6's custom screen
  starts the chip window as it closes (`HudState`), on every
  console. A US console's original shows its placeholder (a purple picture,
  a dot) or its other sprite, and its chip window waits for the navi's
  first decision; a Japanese console shows its own art where the releases
  differ (EXE5's background 0x05 as bubbles in the dark, HeatMan's flames
  in his throw, the two pictures' pixels); an EXE4 Blue Moon console draws rows 12 and 13 of
  four chips' icons (Blizzard, HeatBrth, ElecShok, WoodPwdr) as a dark bar,
  where the graphics have one icon a chip, Red Sun's. No part of the frontend
  knows a console's region or which of these it shows: the verification's
  frame comparison does (its table, consoles.tsv, by game, region and
  version), and asks the headless frontend to mark where it draws those
  things (`--mark sprite:NAME,sprite:NAME:ANIM,background:NAME,chip:KEY,
  button:NAME,chip-icon:KEY,chip-description:KEY,chip-window-at-close`;
  `marks.tsv`
  beside the frames: frame, rectangle,
  what), which it counts as known on that console's recordings, with the
  table's margin around each (a sprite's 48 pixels: the US's sprite there
  may reach past the Japanese one), or only the part of the mark the
  table names (an icon's two rows, a picture's two pixels). The same
  table has what a console shows of its own whatever its region: EXE6's
  Gregar and Falzar chips' descriptions, which a console prints from its
  memory (its own beast's for both chips), where nettai prints each
  chip's own.
- **Deliberate: a version's own chips on the other version's console.**
  Each ROM draws its own version's chips and has their art again at the
  other version's counterparts: EXE6's five Giga chips a version (Bass and
  BassAnly, BigHook and MetrKnuk, DeltaRay and CrossDiv, ColForce and
  HubBatc, BugRSwrd and BgDthThd share one picture, palette and icon in a
  ROM); EXE5's five a version and its phoenix (Team ProtoMan's ROM draws
  MetrKnuk and CrossDiv as HolyDrem, OmegaRkt and BugCharg as BigHook,
  BassAnly as Bass, Phoenix as DethPhnx; Team Colonel's the other way).
  The graphics have each chip's picture and icon once, from its own version's
  ROM (the one whose library lists it), marked with that version
  (`ChipArt::version`), and the frontend and the build creator show it on either
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
  version is its form's own `version` (Team ProtoMan's six, Team Colonel's six:
  `custom::icon_palette_row`), the field a ruleset's groups read
  (docs/design/versions.md §2.2). A console of the other version than the soul's shows
  the icon in its own outline: listed as known
  (`custom::OTHER_VERSIONS_ICON`), as is a MeddySoul capsule's icon, which
  flies in the same palette.
- **Deliberate: the Gregar and Falzar chips' pictures.** Each Japanese ROM
  has one picture for both chips, its own beast; the graphics have each chip's
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
- An EXE5 console in Japanese: EXE5's graphics have the US ROMs' lettering
  alone (docs/design/exe5-map.md §11), so `--lang ja --text original`
  draws its Japanese names in the US font (the enhanced mode draws them); the
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
another game has it or not. nettai's Practice plays one from its matches folder
(you are its left side; docs/app.md §2), `nettai-tool --match FILE` renders
one or says its setup and `--save-match FILE` writes the match with its seed
(docs/tools.md), and a side of one is what nettai's build creator makes (a
build is a side, written as a match file writes one: docs/app.md §8). The
crate `nettai-match` reads, checks and writes them, and builds the round
(`Match::round`); live play's random pick is a match too
(`nettai_match::pick::live`), so a random setup written out and played again
is the same battle (the frontend's test `a_saved_match_plays_the_same_battle`
compares the digest every tick).

```toml
game = "exe6"                               # the match's game and its rules: everything below is its
seed = 42                                  # optional: the setup's and battle's seed
custom_screen_time = 60                    # optional: the custom screen's time limit in seconds, 30, 45, 60, 90
                                           # or 120 (nettai's own); left out, none, as in the original

[[round]]                                  # the set's rounds, a table each, in order: 2n - 1 for a set first to n
stage = "netbattle-43"                     # (n 1 to 10; one: a single battle; three: the triple battle). A round's stage (a
background = "lans-hp"                     # link battle stage of the game's) and background (of its game); a part
                                           # left out is picked from the seed, as the game picks a link battle's
[[round]]                                  # (or with `backgrounds = "pool"` at the top, a background from every
stage = "netbattle-12"                     # game's: the background pool)
background = "exe4:acdc-area"              # another game's background, the pool's, named with its game

[[round]]                                  # (picked from the seed: stage and background)

[left]                                     # you (side 0); then [right]: the side's facts (see below)
navi = "megaman"                           # the navi (stated: a side states its own)
version = "gregar"                         # EXE6's: version, gregar or falzar (stated: none is assumed), crosses, up
crosses = ["heatcross", "spoutcross"]      # to five of either version (else none), beast_out, else unlocked (the
beast_out = false                          # save's flag 0xE0), bug_frags, else 0
bug_frags = 0
hp = 900                                   # what the save brings to the stats: MegaMan's base HP (else 1000), the
reg_up = 45                                # Regular memory (else 50), the sun (else true): a save at its best; the
sun = false                                # rules build the rest (below)
level = 0                                  # the navi code's level, 0-14 (else the navi's last: a link navi's 14,
                                           # MegaMan's none, no code received)
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
descriptions and the build creator without a line of Rust.

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
  every soul, Chaos Unison and a fresh save's karma, 500), else zero. A
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
  until they are given theirs: the build creator shows none chosen; a random
  match picks each side's from its seed and writes it.
- A list a side leaves out is its default, else empty: EXE6's `crosses`
  left out is no Crosses (nothing is read as "its version's own five";
  the build creator's ITS VERSION'S OWN states those, as choosing a version does where none are stated, and a random match the
  five it picked).
- What is refused (`nettai_match::facts::check`, and the file's reader): a
  key the rules' setup doesn't declare ("left: no field \"karm\" (a side of
  exe5 takes chaos_unison, hp, karma, level, reg_up, souls)"); a value that isn't
  the field's type's ("karma: 70000 is past a u16 (0 to 65535)", "version:
  no \"azure\" (gregar or falzar)"); a name the game hasn't; a definition
  twice in a list, or an empty entry before one (a list is filled from
  the front); and, for the one list the engine knows by its role (its
  form list, EXE6's `crosses`), a form that is none of the side's navi's
  own lists. What a value means is the rules' alone: no range is checked
  beyond the type's.
- Facts known by role (`PlayerFact`): the version and the form list,
  which the battle and its frontend read; the level and MegaMan's base
  HP, which tools read (the checks, the build creator). Where a tool needs one it
  asks by the role: a navi's version byte in its stats, the forms a random
  match's form list is picked from, the level's range.

EXE6's facts: `version` (required); `crosses` (none unless stated), up to five
Crosses of either version for the Cross window, in its order (a save's
are those of its version's five it owns, which the original keeps as
five flags and a save import writes as the list: `[]` for a save that
owns none); `beast_out`; `bug_frags`. EXE5's: `karma` (the
light/dark value; dark under 470), `souls` (up to sixteen, either
version's; every soul unless said; none, no soul button, as a save without
Soul Unison has) and `chaos_unison` (the save's event flag 0x236). An EXE5 match's
sides state their `version` (required: `protoman` or `colonel`, the team):
Team ProtoMan and Team Colonel play alike but for the patch card Bass-Cross
MegaMan (111), whose effects are each team's own (a side may hold either
version's souls and chips all the same). Nothing else reads it in a battle
(their navis' version byte is still 0, as an EXE5 recording's); a save's
import and a recording state it from the save's and the console's team.

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
  Memories raise; default 1000), `reg_up` (the Regular memory; default 50)
  and `sun` (default true: MegaMan's alone, which the overworld writes,
  EXE6's `sub_80355EC` and EXE5's 0x080358DC from `EnterMap`), and
  MegaMan's variant (+0x2B, his base HP in hundreds); a link
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
states `hp`. The build creator's NAVI tab shows the stats a round starts with,
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
through its code, so it always has one: a side of one that states none
has its navi's last, 14 (`nettai_match::play_level`, which
`Side::set_navi` states, and the match file's reader for a side that
leaves it out; an import states the save's). At 0 its stats are its
reload's at level 0 and its chip bonus is level 0's.
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
has a level (a side that states none has its story's last, 6), and its
stats are the navi's fresh stats
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
one that states some has those alone (a chip it leaves out: in no time).
A time reads as the game shows it, `mm:ss.cc` rounded down to the
hundredth (`sub_8000D84`), and one written so is the fewest frames that
show as it (`nettai_match::sp_times`). A build in nettai states none: every
SP navi in no time (docs/app.md §8). The SP navi chips' damage goes by them (rules/sp_chips.luau,
`sub_8010AE4`). A fact that is a record is a table of its fields, and a list
of records an array of tables, as above.

**A save** (the build creator's FROM A SAVE…, nettai's `builds::import`, a
save of the build's game: one of another game is refused; each game's
compat crate imports its own, `exe6_compat::import` and
`exe5_compat::import`) gives a whole side. (A build in nettai then sets
what it never states to the rules' defaults, and says which: docs/app.md
§8.)
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
what the save brings (`hp`, `reg_up`, `sun`). The build creator's NAVICUST
tab (its own grid: docs/app.md §8) places the programs on the board as the
game does, and shows what they make.

**The karma** (`karma`, `nettai_match::facts`) is EXE5's light/dark value
(NaviStats +0x44), 0 to 1000; without it, **500**, a fresh save's
(0x08010C00): light for the chips, the starting mood 0x80, no holy panels
cleared. Under 470 a dark MegaMan (mood 0, the dark face and palette, dark
chips usable in a link battle, no soul button); 499 or under clears holy
panels; under 500 he starts worried; 1000 the brightest (mood 190, Tango's
light templates; a build in nettai is one of Tango's two, light or dark:
docs/app.md §8). Like EXE6's `version`, `crosses` and `beast_out` (S6c's
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
list, those; an empty list, none (no button). The original's soul
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

**Soul Unison** (the save's event flag 0: the soul button at all) is the
souls': the button is there when the side has souls, and a save without
Soul Unison gives none. (The original shows the button for Soul Unison
whatever souls the save has: a save with Soul Unison and no soul shows it
never lit, where nettai shows none.) **Chaos Unison** (`chaos_unison`, the
save's event flag 0x236: a dark chip's Chaos Unison) is on unless a side
says (`chaos_unison = false`), as a finished save has it; the round's
setup writes it into the souls part's setup (its default, on, for a setup
that says nothing). A game whose rules don't take it refuses one off. The
netplay offer carries it.

**An EXE5 save** (the build creator's FROM A SAVE… in an EXE5 build,
`exe5_compat::import`: a .sav, or a raw save image as Tango's netplay
templates hold, read by `exe5_compat::save`) gives the navi it operates, its equipped folder with its Regular
chip, MegaMan's NaviCust (the board of its ExpMemry and the programs as
placed) and patch cards (those switched on; a team navi has neither), his
base HP and the operated navi's Regular memory, its karma, the souls its
version's flags give (the forms compat names for those numbers; none
without Soul Unison), its Chaos Unison, and its auto battle data (the block at
save +0x554C, place for place and record for record, chips by their
numbers' names), and its SP navi deletion times (save +0x2670, a halfword
a slot: each SP chip whose damage goes by one, by the slot compat's
records.toml gives it, in the slots' order; 0xFFFF, a navi never deleted,
stated as it is, which the damage reads as the slowest time, as the game
does). What a match can't state of a block is left empty and
said: a chip number the game has no chip for, and an entry for a pattern
past the eighth. The build creator's AUTO BATTLE tab takes that data alone from
a save (FROM A SAVE…, `exe5_compat::import::auto_battle_of_save`), and edits
its places: the 42 places in their six lists, each a chip, a program advance,
a 0 or empty (a build's records are the rules' default, nothing learned, and
no place points at one: docs/app.md §8). A side that operates a team navi takes
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
are named for what the game writes there. Any entry may stand in a place of
the first, standard and patterns lists, as in the save (a save made by hand
can have a giga chip among the patterns' places); a chip in the mega, giga
or program advance list must be of that class (the list's `own_class` in
@exelib/auto_battle/places' `LISTS`: nettai's rule, docs/design/app-ui.md
§2.27), and a pattern, a 0 or an empty place may stand there too.

A chip of another class there is refused, where the check says: "place 29
of the auto battle data (`mega`, entry 2) holds Cannon, a chip of class
`standard`: the list holds `mega` chips alone". The game's own writer puts
only its player's most used chips of the list's class there.

One kind of chip is refused in any place: a chip the original can't play in
auto battle (a team navi's own chip, such as StepSwrd, and the chips
past the library, FtrSword to PnkCapsl). The AI gets in place for a chip by
the chip's positioning class, and theirs, `unplayable` (the original's 255),
is past the game's table of classes: the original crashes there, and the
engine raises. The game's own
writer never puts one among the 42 places (it counts library chips only),
so only a block made by hand holds one, and the check says where: "place 5
of the auto battle data (`standard`, entry 2) holds StepSwrd: the original
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
pattern number past 8, a place that is two things at once, and a chip of
another class in the mega, giga or program advance list.

**Why all of it is stated.** The navi in auto battle plays the entries in order,
the played one going last: three times in four the first, otherwise it steps
into an enemy's row and fires its buster (three shots).

- *The places.* The order it starts with is the consoles' send of the data
  as the round is set up (0x0802C7BE, EXE5's rules' `send`): places 1 to 3
  shuffled over three rounds and the other 39 over 39, two draws a round,
  then the entries packed to the front, here from the battle's RNG before
  its first tick: 84 draws, the same order for both sides' places, as each
  console shuffles its own with the RNG2 they share (a recording states its
  saves' blocks and the RNG2 the send started from, so the send gives back
  what its consoles sent). Where the entries and the empty places between
  them stand changes what a seed sends. A chip in several places is played
  that much more often.
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

A random match (`nettai_match::pick`, Practice's Random) states none: its
sides have the rules' default, what the game's battle end writes of a player
it has learned nothing of (empty places, zeroed records). The user: "i don't
think you need learning right? since the battles are one-off": the data
comes into a match from a match file, a save (the import) or a recording
(compat), never from the game's learning. A netplay offer carries the data.

**The checks** (`nettai_match::check`) run when a file loads, when a netplay
offer arrives (the same `check_side`), and live in the build creator; each problem
is said with where it is:

- the game is one the content has, and every name names a definition of
  the game's (a stage, a background of its game, a navi, a
  Cross, a patch card, a chip, a NaviCust program, a soul, a weapon, a
  record, a form), and every stat is in range; a match made in memory
  holding another game's (no file or offer can) is refused the same way
  ("right: a navi exe5 hasn't");
- it lists the rounds of a set first to 1 to 10 round wins, `2n − 1` of
  them (1, 3, 5 ... 19; a file that lists none, or another count, is
  refused: list one `[[round]]` per round, an empty one picked from the
  seed); each stage
  stated is one a match of the game may name (its rules'
  `link_pick.match_stages`: EXE6's and EXE5's link battle stages, each a
  settings record with the link effect that isn't the random battle's;
  `link_battle_stages`);
- what the game's rules say of the side (their `validate` hook,
  `Battle::validate`, once the round is set up; nettai-match reports what it
  says and knows none of it). The checks set the round up without dealing
  the folders (`check::set_up`, of `Match::undealt_round`: each player's
  battle folder empty, all else a dealt round's, its stats the same), so a
  folder being made is checked as it is, its gaps unfilled; a round to play
  (`check::start`, `Match::round`) is dealt, and takes a whole folder alone.
  EXE6's and EXE5's say:
- the folder keeps the game's rules (EXE6's are rules/folder/init.luau:
  30 chips, so a folder being made, with empty entries, is no folder yet;
  copies by MB, each chip in one of its codes, the chips a save of the
  side's version can collect (a version's own Giga chips are its alone), the
  Regular chip within the Regular memory, the tag chips two other entries of
  60 MB together at most). The
  Mega, Giga and Regular limits are the navi's stats as the round starts them
  (after the rules' `round_setup`: the NaviCust's and the patch cards' folder
  limits, as the original's folder editor and link battle check read the
  reloaded stats). Rust only asks and reports what the rules say, so
  another game's folder rules are its own Luau: EXE5's
  (content/exe5/rules/folder/init.luau, its folder editor's) are four copies
  of a Standard chip and one of a Mega, Giga or dark chip, the Mega and Giga
  levels, at most three dark chips, the chips a save can collect (either
  version's Giga chips: a ruleset holds a build to one version), the Regular
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

**Netplay** brings a side (nettai's lobby: the build chosen, else a random
side; docs/app.md §4), wherever netplay puts you, and the settings both
propose are the match's (a triple battle with each place left to the seed;
two proposals that differ stop the agreeing, saying what differs). The
battle's RNG comes from both players' halves of the seed.

## 7. Embedding

nettai-frontend is a library a larger app depends on to play battles in its
own window: it has no window toolkit, no audio device and no command line in
it (`cargo tree -p nettai-frontend` names neither a window toolkit nor cpal),
and nothing in it prints or exits. Every failure is a value. The host owns the
window, the keys, the sound output and what it tells its user; nettai is one
such host (a Slint window whose rendering is the loop below: docs/app.md §1),
nettai-tools another (the command line, with no window: docs/tools.md), and
`crates/nettai-frontend/examples/embed.rs` a third, with no window at all.

How nettai shows the picture (a new image each picture, on any renderer), keeps
its clock and measures its input's latency (`NETTAI_PLAY_STATS`,
`NETTAI_KEY_PROBE`) is docs/app.md's, §1 and §5. What the first desktop host
learned on the way (nettai-demo, an iced program, retired 2026-10-06):

**How nettai-demo showed the picture.** Each frame (the display's rate) it
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
time to the tick that saw each and to that tick's picture drawn; the ticks'
time and the frame's work; and the audio output's figures (the device's
buffers, the queue's range against its target, the rate shift, the times it
ran dry and the frames played silent, the skips);
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

**Loading a game** is one call, `game::load(name, &Options)`: the game's
content, its assets extracted from its ROMs (`Options::roms`, a
`nettai_extract::RomSource`: none, `RomSource::from_env`'s), its graphics and
strings in the language asked for, the text's font and, unless `sound` is off,
its sound. It gives a `Loaded` or a `LoadError` that says which step failed
(`Failed::Content`, `Assets`, `Language`, `Font`) and carries what the loaders
reported up to there; a `Loaded` carries their warnings too
(`Loaded::report`). The steps are public one by one for a host that wants them
apart (`Game::load(roms, content, name, pool)`, `Game::graphics`, `font`,
`Game::sound`), as nettai-tool does, which loads no
sound for frames it only writes. `game::Loading` is the same call a part at
a time, with how far it is, for a host's loading bar; and `preload` makes a
match ready before its first tick (the content's runtime on the thread that
runs the battle, which a first battle otherwise waits about a second for),
a step at a time and by side: §9.

**What is played** is a driver's: `LivePlayer` plays a set of a match
(`nettai_match::Set::of(&content, &m, seed)`: a match file's, or a random
pick's, `nettai_match::pick::live`) from the local player's buttons, round
after round to the set's end; `netplay::NetPlayer` plays one against another
player over a channel the host provides (`netplay::Channel`: it sends the
frames and takes those that came, never waiting, and may say how long it has
been down while its transport makes it again: the match waits it out; the
library has no socket).
The lobby and the handshake that agree the match are the library's,
without IO (`lobby::Lobby`: datagrams in, datagrams out), and so is the set
played from the agreement (`netplay::netplay_setup`); the transport is the
host's. nettai's is a WebRTC link (nettai-rtc's `Link`, which another host
can use too: in a room of the signaling server, natively or in a browser, or
directly, natively), and the lobby its frame polls, so it stays responsive
while it waits (nettai's `netplay` and `lobby`: docs/app.md §4). A host may
bring a driver of its own (`Driver`): nettai-tools' replays the original's
recordings.

**The player** (`Player`) is what the host drives. It owns the session, the
renderer, the enhanced mode's text renderer and the battle's audio:

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
  the speed set, and keeps the part of a tick left over. On a display with
  frames to spare (faster than the ticks, on average) a late frame that owes
  two ticks runs one and keeps the other for the next frame, so each tick
  has its picture: on a loaded machine whose 120 Hz display gave the window
  60 to 100 uneven frames a second, 42 to 52 of the battle's 59.73 pictures
  a second were shown before, every one after. A host that paces
  itself calls `tick(buttons)` instead, once per frame of its own clock; the
  embedding example does.
- `frame()` is the battle's 240x160 picture with nothing over it;
  `present(buffer, width, height)` scales it by the largest whole factor that
  fits and draws the enhanced mode's text (the game's own) at the buffer's
  resolution. Neither has anything of the library's over it: the library
  composes no text and draws none.
- What a host may want to say is values, to show however it likes or not
  at all: `position()` (where playback is, in a few words), `paused()` and
  `speed()`, `stop_reason()` (why it stopped, once it has, as a kind to say
  in the host's words: finished, the other player left, the connection
  lost, internal) and `stopped()` (the same in words for a log: the engine
  stopped, the input ran out, the set is over, the other player left),
  `finished()`, `result()` (a set's, for the local player; a netplay
  match's as soon as it is over), `diverged()` (the first difference from a
  recording) and `net_status()` (a netplay connection's figures: ping,
  jitter, quality, loss, input delay, the last and deepest rollback and
  their count, frames waited, and reconnecting with its countdown).
  nettai shows them on its battle screen (the result, a stop, the
  connection's line); nettai-tool prints a stop and a difference.
- `set_paused`, `slower`, `faster` and `restart` are the controls; each
  returns false and does nothing while the battle runs in real time with
  another player (`real_time()`: netplay). `set_input_delay(ticks)` is
  netplay's: the frame shown from the next frame on is that many ticks
  behind the player's newest input (false outside netplay).
- `take_pause_requests()` is the host's cue for its own menu: the players
  who pressed START in the fight since the last call, by side, each where
  the game's own pause was (§2). The battle didn't pause; the host opens
  its menu and, outside real time, calls `set_paused(true)` until it
  closes. (A paused player runs no ticks and makes no sound; the picture
  stays the last tick's.) `set_held(true)` holds the battle where it
  stands but has it look and sound as the game's own pause does: a tick
  due steps nothing of the battle (no state, no cue, no input, `ticks()`
  the same), plays a tick's worth of the audio driver (the music and what
  was sounding going on), and moves the stage's scenery a frame on
  (`Renderer::scenery_frame`: the background's animation and scroll, the
  enhanced mode's glide with it, and the panels' own animations, their
  palette cycles; the sprites stand, as in the game's pause). The frames
  the scenery moved are the renderer's alone, added to the battle's clock
  for drawing (`Renderer::stage_clock`, `Player::stage_clock`) and kept
  after the hold, so nothing jumps back; a new round or a start over
  forgets them. Nothing of them is in the battle, its snapshot or its
  digest, and a battle never held draws byte for byte as before. nettai's
  practice menu holds its battle so wherever it opens. False in real
  time.
- `opponent()` and `set_opponent(Opponent { in_fight, on_custom_screen })`
  are live play's: what the other navi does, read and changed while the
  set runs (§2); none and false for a battle whose other navi is
  somebody's. `hold_opponent(true)` has its presses reach nothing from
  the next tick on (a host's menu over a battle that steps on), and
  `hold_opponent(false)` lets them through again: its choices and its
  random presses go on as they would, so what it presses after is what it
  would have pressed. `training()` and
  `set_training(Training { .. })` are live play's too: a training session's
  options (§2), at once.
- `take_samples` gives the sound of the ticks run since the last call (about
  549 samples a tick); a player made without sound gives none.
- `set_sound_levels(music, effects)` is the host's level for the battle's
  music and for its effects, each 0 to 1. The game's driver mixes the two
  into the one stream, so a host can't turn one down in the samples: the
  driver mixes each note at its player's level (`m4a::Driver::set_level`,
  through `BattleAudio::set_levels`; the music is one player's), and what
  sounds is unchanged. At 1 and 1 the samples are the driver's own. A
  replay's `Viewer` has the same call, and keeps the levels across seeks.
- `late()` counts what the player made on the way that a preload makes
  ahead (§9): runtimes loaded on its thread, parts read from disk, and the
  text's strings and glyphs made for the first time.
- `play(driver)` goes on with another driver in the same picture and sound (a
  recording's next round).
- `set_language(&graphics)` shows the battle in another language from the
  next frame: the host loads the game's graphics in each language it offers
  (`Game::graphics(lang)`) and keeps them. Only the drawing changes (the
  graphics' lettering, the content's strings); the battle doesn't know its
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
without its `playback` feature; no compat crate is in its tree (a host that
imports saves takes each game's compat crate, whose `import` it is, as
nettai does: `builds::import`). The library itself reads no recording and no save.

**The proof that it embeds** is the example: it plays a seeded random match
with scripted buttons a tick at a time, with no window, and checks every
frame it takes against the PNG nettai-tool writes headless for the same seed
and buttons, byte for byte, and its samples against a second rendition from
the battle's cues. The verification workspace runs it against both games
(`tools/embed-against.sh`).

## 8. Replays

A replay is one set, all its rounds, as it was played: the match and every
tick's buttons, both players'. The simulation is a function of those alone, so
playing them again makes the same set, round by round; nothing it makes is
kept (who won, the score) but the ticks a round and the set ended on, and the
battle's digest now and then, to tell a replay that reproduces from one that
doesn't. A replay plays only on the content it was made with and an engine
of the same minor version (`nettai_replay::line`: a patch release plays it);
it names them, and another minor version or content refuses it with what differs.

**Recording.** nettai records the sets it plays to its
replays folder (docs/app.md §2), and `nettai-tool --match FILE --headless F
--record set.nettaireplay` a set played headless; in the library,
`Player::record(Recorder::new(sink, &content, &m, &info)?)` before the first
tick. The `info` says when, which side recorded, the players' names, the
kind of match (`MatchKind::Offline` or `Online`) and whether its setups were
open (`Info::of(kind, side)` is one with the rest left out, its setups
open: an offline set's, both builds the player's to see). **A netplay
match's recorder is made of its agreement**, `Recorder::of_agreement(sink,
&content, &agreement, when)`: the lobby's `Agreement` has the match and its
seed, this peer's side, both names and whether the players agreed open
setups (`Agreement::open_setups`, §2), so the info is all the agreement's and a
host passes the setting on by recording at all. Live play
writes each tick as it runs; netplay each as it settles
(confirmed, never a prediction), so both players' replays of a match hold the
same match and ticks, and differ only in who recorded. Each tick is flushed as
it is written: a crash leaves every tick before it, and the file reads as cut
short. A round of netplay ends on the tick it ended on, which both peers know
however late they see it settle (rollback.md §4.6, "Rounds"), so the replay's
marks are the simulation's.

**What a replay says of each side** is its player's name and its setup: the
navi, the folder and whatever else its game's rules take, each a number or a
definition's handle (the match, below). The setup stays, because the
simulation is made from it. **A build's name is not in a replay**, nor
anything else of a build that the battle doesn't use: the info has no place
for one (nettai-replay's test
`a_replay_holds_no_more_of_a_side_than_its_name_and_setup` pins the head and
the info to their listed fields), and a match in binary holds no text at all
(nettai-match's `no_fact_holds_text`: no fact's type holds a string). Where
the players didn't agree open setups, so neither saw the other's build, the
info says so (`Info::open_setups` false, which a host reads from the replay or
from `Viewer::info()`), and whoever shows the replay hides both setups too
(docs/design/app-ui.md §3): the file still holds them, as it must.

**Playing back.** `nettai-tool --replay set.nettaireplay` plays one out and says
whether it reproduces, and with `--headless F` renders the set's ticks F
(`--side left|right`: the console shown; default the recorder's). In the
library there are three ways, by what is wanted:

- `replay::play_out(&content, &replay)` plays one to its end with no picture
  or sound: the ticks, each round's winner, the set's result, how the file
  ends, and the first difference. In a release build it plays about 200,000
  ticks a second: a 10.4-minute EXE6 set of three rounds (37,187 ticks, one
  side mashing) in 0.18 s, about 0.05 s for three minutes of play (`cargo test
  --release -p nettai-frontend play_out_timing -- --ignored --nocapture`); a
  host listing replays can afford to play each one out.
- `ReplayPlayer::new(&content, &replay, side)?` is a driver like any other,
  for a `Player`: straight playback, start to end.
- `viewer::Viewer::open(&game, &replay, side)?` is a replay's own player,
  with a transport ("Seeking", below): what a Replays screen shows.

A digest or a round's or the set's end that differs from the recording is
reported as a difference (`Player::diverged`, `Viewer::diverged`), with its
tick; the simulation is side 0's on every console, so a replay shown from
side 1 compares the same digests. A console shown hears its own side's
sound (`session::heard`: the charge only its player hears, its own navi's
hit, the music of its own result).

**A replay that isn't played** says why as a type, for a host to say in its
own words (docs/design/app-ui.md §7), with the detail as its `Display`, for a
log:

| | |
|---|---|
| `ReadError::NotAReplay` | the bytes aren't a replay's |
| `ReadError::OtherLayout { layout }` | a replay of another layout: another version of nettai's |
| `ReadError::Damaged(why)` | a replay of this layout that no writer writes (cut inside its head, a value none holds) |
| `Refused::OtherVersion(why)` | made on another engine or other content, or its match makes another first battle here |
| `Refused::OtherGame { replay, here }` | of another game than the content given |
| `Refused::BadMatch(why)` | its match doesn't read or breaks the rules: a damaged or changed file |
| `Refused::NoSide(side)` | asked to be shown from a side a match doesn't have |

`ReadError::OtherLayout` and `Refused::OtherVersion` are "this replay is from
a different version".

### Seeking

`Viewer` plays a replay back for a host to show and hear, as a `Player` does
(`advance(elapsed)`, `present(buffer, width, height)`, `take_samples(&mut
out)`, `set_language`), and has a transport:

| | |
|---|---|
| `length()`, `duration()` | the replay's ticks and its time, known as it opens (the file's) |
| `info()` | what the replay says of itself: the names, the kind of match, open setups |
| `rounds()` | each round's first tick, in order (the file's marks): the bar's marks |
| `position()`, `time()`, `round()`, `at_end()` | where it is |
| `seek(tick)`, `seek_time(time)` | go there (past the end: the end) |
| `skip(seconds)` | five seconds on, or back with -5 (299 ticks) |
| `previous_round()`, `next_round()` | to the round's start (from it, or from its first second while playing, to the round before's); to the next round's |
| `step_forward()`, `step_back()` | a frame on (with its sound), a frame back |
| `set_speed(Speed)` | `Half`, `Normal`, `Double`, `Quadruple`; the sound follows as a tape's does, so the host gets as much sound a second at any speed |
| `set_view(side)` | either console's picture, HUD and sound, switched while it plays |
| `present_view(side, ...)` | either console's picture now, whichever is shown: the other's is drawn from the same tick with its own HUD state (both run every tick), so a host shows both at once without playing twice; nothing shown or heard changes |
| `set_paused(bool)` | pause, or go on |
| `result()`, `stopped()`, `diverged()` | the set's result for the console shown (at the end of a whole set), why the engine stopped short of the end, the first difference from the recording |
| `index(ticks)`, `indexed()` | play ahead without moving (below) |
| `index_part(ticks)`, `take_index(part)` | the index made by another viewer of the replay, on a thread of its own, and taken in (below) |
| `index_battle(ticks)`, `ConsoleSound` | the same made on threads: the battle by a silent viewer, each of its sounds beside it (below) |

**A seek lands in the state straight playback reaches**, all of it: the
battle (its digest is straight playback's), what the drawing follows over
time (the HUD's rolling numbers and timers, for both consoles) and the sound,
to the sample, for both consoles. So after a seek, a frame back or a change
of console, what is shown and heard is what was shown and heard there the
first time: a note that was sounding sounds on from where it was, the music
is where it was in its loop, and nothing of the ticks passed over is heard (no
burst), nor anything of where the viewer came from (no note left hanging).
The simulation is asked nothing a tick doesn't ask: a seek restores a state
the replay passed through and plays the recorded buttons on from it.

A viewer has no pause of the battle's. A match's host answers START in a
fight (§2), so a replay holds the presses and reports each again as it plays
(`Battle::pause_requests`); a viewer drops them, playing, seeking or
indexing, and is paused by its own host alone (`set_paused`).

**How.** As the replay plays, or is sought through, the viewer keeps a copy
of everything it is at every 240th tick (`KEYFRAME_EVERY`: four seconds) and
at each round's first tick: the battle (`Battle::save_state`, the snapshot
rollback uses: docs/design/rollback.md §1.3), each console's HUD state, and
each console's sound (`BattleAudio` is plain state over its shared banks,
and so `Clone`). A seek goes on from the nearest kept state at or before its
target, or from where it is if that is nearer, and plays the ticks between in
silence. Both consoles' HUD states and sounds run every tick, whichever is
shown, so a change of console is a swap. The ticks a seek plays through
leave states of their own every 16th tick (`NEAR_EVERY`), of that one
stretch between two kept states, so a frame back and the next each go on
from a tick or so before: only the first of them into a stretch plays the
stretch. Reaching a tick for the first time
plays every tick up to it; `index(ticks)` does that ahead of time, a slice
a call, leaving the viewer exactly where and as it is (a host calls it a
slice a frame, or all behind its loading screen, until it returns true; or
has the index made on threads of its own, below). A
replay is made ready as any match is besides (§9): `replay::set_of(&content,
&replay)?` is its set for a `Preload`, and `Preload::viewer(&replay, side)?`
the viewer with what was made ready.

**The index made elsewhere.** A host that has threads indexes on one of its
own, so that its own thread spends nothing on it: a second viewer of the
replay, opened there at its start with its sound (made and never heard, so
that each state holds it), plays it through a part at a time
(`index_part(ticks)`: the states it would keep handed over in an
`IndexPart`, not kept, with the tick it reached and whether the replay's end
is in it) and sends each part as it is made; the viewer watched takes them
in, in order (`take_index(part)`: each state kept as its own, one of a tick
it has already dropped, `indexed()` going on to where the part reached). A
part is plain data and `Send` (the battle's snapshot, the HUDs', the sound's
over banks shared by `Arc`). The replay plays the same on either viewer, so
the states are the ones the viewer would make itself, from either side: they
differ only in the side seen from (`RoundSetup::local_side`, which the
digest covers), and a state put back takes the side from the viewer. The
viewer that made them keeps none. Checked by
`an_index_made_elsewhere_is_the_viewers_own` (each state's battle and HUDs
against a viewer that indexed itself, and the seeks after, sound and all,
against straight playback).

**On threads** (the user: "the preload still seems slower than it
should be?"; approving "run each player's sound on its own thread beside
the game's thread"). Half of a tick or more is the sound, and a sound takes
nothing from the battle but the tick's cues: what each of a viewer's sounds
(`SOUNDS`: each console's, and each side's own effects, the ones its
console plays that the other's doesn't that tick, which a screen of both
consoles adds to the left console's sound: "Both on one screen", §3) is
given each tick comes of `sound_cues_for(side)` of the battle that ticked
(the round that ended, on a tick that ended one), then its frame runs
(`BattleAudio::handle`, `tick`). Nothing else touches it as
a replay plays: a round's start, the speed, the side shown and the HUDs
don't, and the host's levels are put on a state as it is put back. So the
sound is made beside the battle. A silent viewer (`Viewer::with` given no
sound) plays the battle on one thread (`index_battle(ticks)`: a
`BattlePart`, the states it would keep without their sound, each tick's cues
for each sound, and after which tick each state was kept); each sound, as a
viewer opened on the game has it at the start (`ConsoleSound::of(&game)`,
or `ConsoleSound::all(audio)` as `Viewer::with` is given it), plays the
part's cues for it on a thread of its own, tick by tick, and puts a copy of
itself into the part at each state (`ConsoleSound::play(&mut part)`, the
parts in order); once all have, `part.into_part()` is the `IndexPart` a
viewer making its own sound makes, for `take_index` as before. The parts go
from one thread to the next as they are made, so the threads run at once,
and the index goes as fast as the slowest of them: the battle's share alone (a silent viewer's) on EXE5's
assets, a console's sound on EXE6's, whose battle is lighter. Checked by
`an_index_made_on_threads_is_the_one_made_on_one` (the threads wired as
a host wires them, from the other side and in parts of another length: each
state's ticks, battle, HUDs and session against `index_part`'s, each
console's sound by the samples it makes over the two seconds after, sample
for sample; and seeks from the states taken in against straight playback,
the picture and the sound included), and on a game's assets by
`examples/seek.rs`, whose seeks are the viewer's that took the states in,
and `examples/index.rs`, which makes a replay's index both ways and checks
each state's battle and both consoles' sound after it. The app does this
(docs/app.md §16).

**What it costs**, measured on EXE6's own assets by `examples/seek.rs` in a
release build, in processor time (the machine was busy with other builds),
on a set of 16,506 ticks (4 minutes 36 seconds, two rounds, both navis
mashing):

- **Memory:** a kept state is 61 KB (the battle's snapshot 32 KB, each
  console's sound 15 KB), and the set keeps 70: 4.3 MB. Five minutes keep
  about 77, 4.7 MB. The states of the stretch last sought through are up to
  14 more, under 1 MB.
- **Indexing the whole set:** 1.19 s, 72 µs a tick, so 1.3 s for five
  minutes. Most of a tick is the two consoles' sound, made and thrown away:
  a viewer without sound takes 17.5 µs a tick (0.29 s for the set). A viewer
  that never indexes pays this once, on its first seek to the end (1.18 s
  measured), or as much of it as lies before wherever it first seeks.
  Playing the set straight with its picture takes 17 s, so the index is a
  fourteenth of that.
- **Indexing on a thread of its own** (2026-10-21, in the `dev` build,
  which is slower: a set of 9,000 ticks, one round): a second viewer's
  parts, made on a thread and taken in as they came, indexed it in 0.88 s
  of the wall, 10,190 ticks a second, as fast as the viewer indexed itself
  in slices (0.88 s of processor time, 98 µs a tick): a part costs nothing
  to send or take in.
- **Indexing on three threads** (2026-10-22, when the viewer had two
  sounds, each console's; it has four since, each side's own effects
  beside them, each on a thread of its own: §3, "Both on one screen"; in a release build, the
  wall's time, the machine busy with other builds): on the user's own
  19:42 EXE5 replay (70,623 ticks, three rounds; `examples/index.rs`),
  3.8 to 4.0 s, 54 to 57 µs a tick, about 18,000 ticks a second, where one
  thread making its own sound took 7.1 to 7.4 s (101 to 104 µs a tick):
  1.8 to 1.9 times as fast, and as fast as the battle's share alone (a
  silent viewer: 3.8 to 4.0 s). (Busier, in four runs more: 3.7 to 6.1 s
  against 7.5 to 10.2 s.) The two indexes were the same: at each of the
  297 states, the same battle, and both consoles' sound over the half
  second after it the same, sample for sample. On EXE6's assets, whose
  battle is lighter (`examples/seek.rs`, a set of 18,000 ticks, both navis
  mashing): 0.46 s, 38,800 ticks a second, where a viewer indexing itself
  took 68.7 µs a tick of processor time; there each console's sound, a
  little longer than the battle, is what the three go at. The 300 seeks and
  480 frames back were then the viewer's that took the states in, each
  checked as below.
- **A seek in what was reached before:** 4.7 ms (the median of 300 seeks of
  every kind; 20 ms at worst: at most 239 ticks are played).
- **A frame back:** 0.5 ms (the median of 480 in a row; 16 ms for the one
  that goes into a new stretch and plays it).

The example also checks what it measures: after each of those seeks, and
after the frames back, the battle's digest, the picture presented at twice
the frame with its text, the HUD's state and the sound of the ticks that
follow are straight playback's, from both consoles and across changes of
console, on the game's real graphics and sound (`cargo run --release -p
nettai-frontend --example seek -- --game exe6 --seed 7`).

Kept states at each round's start make a round's start exact and at once;
a smaller `KEYFRAME_EVERY` buys shorter first seeks into a stretch for more
memory in proportion.

### The file

nettai-replay holds the format alone and runs no engine:

```text
file   := "NTRP", layout (a byte: 1), head, info, match, input
head   := "HEAD", length (u32), engine version (string), game (string),
          content hash (u64), the first battle's digest before any tick (u64)
info   := "INFO", length (u32), when (u64: unix seconds), the side that
          recorded (a byte), the kind of match (a byte: 0 offline, 1 online),
          open setups (a byte: 0 no, 1 yes), the players' names by side
          (two strings)
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
15 KB. The layout byte is the container's (this is layout 1); what a match
holds is pinned by the content hash, nothing else. A file of another layout
is refused as another version's (`ReadError::OtherLayout`). A replay whose
match makes another first battle than it recorded is refused
(`Refused::OtherVersion`), as any is once the engine or the content has moved
on.

**The match** (nettai-match's `binary`, which the netplay offer uses too) is
read against the content the head names: definitions by their handles (the
same on every load of content of that hash), and nothing by
name but a round's background, which may be any game's (the background pool),
by its game and its name in that game's content:

```text
place := stage (u16: its handle), background (string: `<game>:<name>`)
round := what the round states (a byte: 1 its stage, 2 its background, 3 both, 0 neither),
         then its stage's handle (u16) if stated, its background (string) if stated
side  := the side's setup of the game's rules in its compact form (each fact in the
         setup's order at its width; a record's fields in order; a list as its
         count and only the entries it holds)
match := seed (u32), the round wins its set is first to (u8: n, 1 to 10), every one of
         its 2n - 1 rounds' places (as the seed picks those the match leaves), the custom
         screen's time limit (u16: seconds, 0 for none), side 0, side 1
offer := the rounds' count (u8), each round as the player states it (round), then the side
```

A side's bytes are its game's rules' (a game's facts change no other game's),
a place's the engine's. A replay keeps every round's place, whatever the match
left to its seed. A random EXE6 side's offer with three stated rounds is
about 210 bytes, EXE5's about 540 (its auto battle data included); a whole
match 410 and 1,060. A reader refuses bytes that end early or run on, a handle
past the content's, a background neither the game's content nor the pool has and a value no fact may hold,
saying where; the match's checks then judge what it read, as for a match file.

### Video

nettai-video writes a replay as an MP4, the same way on the desktop and in a
browser. It is a crate of its own over the viewer, with no encoder and no
ffmpeg library in it:

- **The playing** (`export(&game, &replay, &options, &encoding, to,
  progress)`, `write_mp4(&mut viewer, ...)`): the replay through a viewer, a
  frame and its sound a tick, the picture made at the video's size on
  threads of their own (`presenters`), the sound made 48 kHz (`resample`).
- **The encoder** (`encode`), the platform's: `ffmpeg` run as a child on the
  desktop (`ffmpeg`), the browser's own in a browser (WebCodecs,
  `webcodecs`). Each is given the same frames and the same samples, and says
  back its tracks and their encoded samples.
- **The MP4** (`mp4::Muxer`): the encoder's samples written to any
  `std::io::Write` a chunk at a time as they come, and the head that indexes
  them made last (`Made`), for the host to put first.

**What a video holds:** one frame of the picture a tick and the sound the tick
made, as a viewer playing the part straight presents and plays them
(`Viewer::present`, text and all; `Viewer::take_samples`), the sound made
48 kHz. The part is the
whole replay, a round (`Part::Round(n)`) or a range of ticks
(`Part::Ticks(a..b)`: what playing from position `a` to position `b` shows),
from either console (`Options::view`), from both on one screen
(`Options::sides`: `Sides::Both`, the frame's size, as a viewer shown so
presents it: §3, "Both on one screen"; one sound track, both consoles'
sound as that viewer plays it, the left console's with the right side's own
effects over it, titled with both players: nettai-video's test
`both_on_one_screen_is_the_viewers_both`), or from both side by side
(`Sides::SideBySide`, a picture twice as wide, 480·N by
160·N, the view's console at the left and the other's at the right, each
half byte for byte that console's own video, from one playing of the
replay: `Viewer::present_view`; nettai-video's test
`both_sides_are_each_sides_video_side_by_side`). Side by side, each console's
sound is a track of its own, the left one's first and the right one's
second, each what that console played
(`Viewer::take_samples_of`: both consoles' sounds run every tick, so one
playing gives both). Not a mix: the two hear one battle, so a mix would be
its music and shared effects twice over, and at the set's end the winner's
music over the loser's; apart, what one console alone hears (its own
charge, its own navi's hit) is in its own track. A width twice an even one
is even, so nothing changes for 4:2:0. A part starts in the state straight
playback reaches there, sound included (the viewer's seek), so a part's
frames and samples are the whole video's for those ticks. The same replay
gives the encoder the same frames and samples (nettai-video's test
`the_same_replay_gives_the_same_frames`, and the same from a viewer that had
been sought about first).

**Titles and chapters** (`Options::titles`, a host's words: the app's in
its language). The first track is the default; each track is titled with
the player whose console it is (`Titles::sides`, else the replay's name,
else "Player 1" or "Player 2"). Each round is a chapter titled
`Titles::rounds`' (else "Round N"), from the frame its picture begins on to
the next one's (`chapters`). That frame is the tick before the round's
first: the tick that ends a round shows the next one at its start, the
position the viewer calls the round's first (what is heard of that tick is
still the round it ended). A part's first frame is its round's, and a
round whose picture begins on a part's last frame starts no chapter (a
round's part ends on the next round's first picture). nettai-video's tests
`a_chapter_is_a_round_from_its_first_picture` and
`the_video_says_its_tracks_and_chapters`. They are the MP4's to write
(`mp4`): each sound track's title as its handler's name, the first track
enabled and the others its alternates, and the chapters as Nero's `chpl`.

**The sound is made 48 kHz here** (`resample`), before either encoder is
given it: the consoles' 32768 Hz is a rate neither AAC nor Opus has, and
done here, ffmpeg and the browser's encoder are given the same samples (the
user: "i want to make the input to the codec layer the same for both
paths"; ffmpeg resampled for itself before, the browser's encoder by a
Catmull-Rom interpolation of its own). Each output sample is 128 input
samples about its place through a Kaiser-windowed sinc (β 9), cut off at
15.9 kHz (0.97 of 16,384): flat to about 15.2 kHz, nothing from 16.6 kHz,
where the image of what is under 16.1 kHz starts. Its places are exact:
output sample `n` is at input sample `n·256/375` (the rates over their
greatest common divisor), one of 375 phases with its weights worked out
once, so the sound keeps step with the picture over any length (its time
base 1/48000 s, the picture's 4389/262144 s: a tick exactly) and the output
is the same however the input is split, sample for sample. Output sample 0
is at input sample 0; the last 64 input samples' worth waits for the input
after it, and the last frame's sound has it, the input ending there
(`Resampler::finish`), so a video's sound is ⌈N·375/256⌉ samples for N of
the consoles'. Measured: tones from 55 Hz to 13.5 kHz are the tone at
48 kHz to 94 dB or better (`a_tone_is_the_same_tone_at_48_khz`); on the
test set's sound, against ffmpeg's resampler with a 1024-tap filter, it is
101 to 111 dB off up to 14 kHz and 45 dB off from 14 to 15.5 kHz, and has as
little above 16.4 kHz (where only images are) as that reference, 70 dB under
the whole; ffmpeg's default resampling, what the desktop had, is 66 dB off
from 10 to 14 kHz and has 30 dB more above 16.4 kHz.

**The picture is scaled here, by whole multiples and the nearest pixel**
(`Options::scale`, 1 to 16), not left at 240x160 for an encoder to scale: in
the enhanced mode the battle's words are drawn at the output's resolution
over the scaled frame, so only a frame presented at its final size has them
sharp.

**Three things at once** (the user: "is replay export slower than it should
be?"). A video was one thread doing everything in turns: play a tick, draw
its frame at the video's size (3.5 ms a frame at four times the frame, most
of it the scaling and the text), write it into ffmpeg's pipe and wait while
ffmpeg took it. Now:

- *The playing thread* plays the battle in order and draws each tick's
  frame at 240x160 (`Viewer::frame_view`, a console each), with its sound,
  which it makes 48 kHz.
- *The presenters* (`presenters`) make each frame's picture at the video's
  size, scaled with the enhanced mode's text over it, and its RGB bytes: a
  whole frame each, on as many threads as the cores but one, at most four
  (two where the system doesn't say: a browser, whose threads are Web
  Workers, `nettai_platform::thread`), each with a text
  renderer like the viewer's (`Viewer::text_renderer`). The frames come
  back in order, a frame a thread and two more under way at most, their
  buffers going round. It is the one way a video's frames are presented,
  with the window's own `present`, on the desktop and in a browser alike:
  each thread says it started, and where they don't all within 20 seconds
  (a browser's load the page's module each), or one stops, no video is
  made (`Error::Threads`), as where the encoder fails.
- *The encoder* is fed from a queue of a few frames: on the desktop ffmpeg's
  input is written from a thread of its own (`ffmpeg::QUEUE`, three), so the
  drawing never waits on the pipe, and its output is read on another; in a
  browser the encoding runs on a thread of its own, on its event loop, where
  WebCodecs answers (eight, a frame waiting while they are full).
- *Canceling* stops it at the next frame written: the playing thread
  lets go of the presenters' queue (each thread ends after the frame it is
  on) and of the encoder, which stops ffmpeg and waits for the writer, whose
  write fails as ffmpeg ends.

The drawing itself is cheaper too, the same pixels: `present` works out what
depends on an output column alone (its frame column, whether it shows the
nudged frame, the glide's column) once a frame and a frame row's colors once
a row, and an output row that shows what the row above it shows is that row
copied (it was a color conversion, two 64-bit divisions and the glide's
lookups an output pixel); `compose_with_depth` finds each pixel's frontmost
layer front to back and stops there, fading only what shows; the text
layer's drawings find each row's frame row once. Each keeps the old way in
its tests and is held to it on random frames
(`the_picture_is_each_pixel_worked_out_on_its_own`,
`a_frame_is_put_together_as_at_each_pixel_alone`), and a text renderer draws
a frame the same whatever it drew before
(`a_renderer_draws_a_frame_the_same_whatever_it_drew_before`), which is
what lets any thread present any frame.

Measured on the user's 19:42 EXE5 replay, its first two minutes (7,200
frames at four times the frame, 960x640, the enhanced mode; release builds,
`nettai-tool --replay R --mp4 OUT --scale 4 --ticks 1-7200`):

| | before | then |
|---|---|---|
| the MP4 | 34.4 s | 20.2 s |
| the frames alone (then `--nut /dev/null`, since gone) | 25.3 s | 4.7 s |
| a frame drawn | 3.5 ms | 0.66 ms |

(Each twice, the two in turns, on a machine other builds were using.) The MP4
then took as long as ffmpeg alone took to encode the same frames from a file
(1,200 of them: 3.2 s against ffmpeg's 3.0; libx264 at `-preset medium`, CRF
16, unchanged), and was byte for byte what it had been.

**On the desktop: ffmpeg** (`ffmpeg`), a child a track (the user: "can we
avoid emitting nut entirely?"). The picture's child is written the frames'
raw RGB on its standard input (`-f rawvideo -pixel_format rgb24`, the
video's size, `-framerate 262144/4389`: a tick's rate, so its times are the
frames' count, exact), and each sound track's child that track's samples
(`-f f32le -sample_rate 48000 -ch_layout stereo`); each says its track back
as a fragmented MP4 on its standard output, read as it comes (its track from
its `moov`, each fragment's samples from its `moof` and `mdat`) and not kept:
the MP4 is the muxer's, which puts the tracks together. Each child reads its
own pipe and nothing else, and its output and what it says are read on
threads of their own, so no child waits on another and a full pipe only
means that child is busy (the user: "well if i feed them separately will it
cause pipe problems?": with one ffmpeg reading every track on pipes of their
own it could, its output interleaving the tracks while the writer waits on
another's full pipe; and those pipes would be more descriptors on Unix and
named pipes on Windows). It was a NUT stream into one ffmpeg until then; that
was no faster (the frames' bytes go to the pipe as they are either way, and
x264 sets the pace) but a container of our own to write, read and test. The
command lines are the module's (`ffmpeg::picture_arguments`,
`ffmpeg::sound_arguments`): H.264 by libx264 (`-preset medium -tune
animation -crf 16 -g 120`), 4:2:0, the colors converted and tagged as BT.709
(the primaries and the transfer by `setparams` on the frames: ffmpeg 9.0's
encoder takes them from the frames, and with `-color_primaries` and
`-color_trc` alone the MP4 said "unknown" for both); AAC at 192 kb/s of the
samples as they come, resampled by nothing. Each child's `moov` waits for its
first fragment (`delay_moov`), so it has the track's edit list: the
picture's past its reordering, and the sound's past AAC's encoder delay, its
first 1024 samples, which a player skips by it. (Before, ffmpeg wrote its
`moov` ahead of any sample, with no edit list, and from 7ad595dff the
desktop's sound played those 1024 samples, 21 ms, late; nettai-video's test
`ffmpeg_makes_the_mp4` holds the decoded sound to the samples given, at no
lag.) The picture isn't scaled by
ffmpeg. What 4:2:0 costs depends on
the size: at an even multiple of the frame every game pixel covers whole
color samples and edges stay clean, at an odd one every edge gets a fringe
of mixed color. Measured on 120 frames of an EXE6 fight (the decoded MP4
against the frames, RGB PSNR): 40 dB at twice the frame and 44 dB at four
times, about 2 Mb/s each; 25 dB at three times, at 4 Mb/s; 38 dB at three
times in 4:4:4 (`Chroma::Full`, which fewer players take). So a host offers
even sizes first.

**Which ffmpeg** is `ffmpeg::find_ffmpeg()`, the one the app, the tour and
`nettai-tool --mp4` run: `$NETTAI_FFMPEG`, else `ffmpeg` beside the running
program (the one every package bundles: docs/app.md §19), else one in an
absolute folder of the path or where a package manager puts it
(`/opt/homebrew/bin`, `/usr/local/bin`, `/opt/local/bin`). A relative folder
of the path (`.`, or an empty entry) is never looked in, so an `ffmpeg` in
the working directory isn't run; found nowhere, it is the bundled one's
place, which fails to start (`NoFfmpeg`). The bundled ffmpeg is a minimal
static build of these commands and nothing more (nettaisen/ffmpeg-build,
from ffmpeg-9.0.2-2: the raw video and float PCM demuxers in place of NUT's,
and pipes alone).

**In a browser: WebCodecs** (`webcodecs`). H.264 at level 4.2 (High, else
Main, else the constrained baseline every encoder has), at a bit rate made
of the rate factor (a fifth of a bit a pixel a frame at CRF 16, at most
40 Mb/s), a key frame every 120 frames; AAC where the browser has it, else
Opus, a track a console shown, of the same 48 kHz samples ffmpeg is given.
An AAC encoder puts samples before the sound (its priming: AAC's blocks
overlap, and an encoder looks ahead), which no AAC packet says and WebCodecs
doesn't tell: Chromium stamps its encoders' packets from 0 (macOS's
AudioToolbox puts 2112 first, Windows' Media Foundation its own). So an
export measures it as it starts, a click through an encoder of the same
settings and a decoder, and each AAC track is shown from past it by its edit
list, as the desktop's tracks are by ffmpeg's (the user: "is aac in the
browser broken then?", then "ok fix it then": before, its sound played that
much late, 44 ms on macOS). Measured in Chrome on macOS: 2112, the same each
time, and a sound encoded and decoded, those 2112 skipped, lines up with
what was encoded at no lag. Opus says its own delay, its pre-skip (312:
Chromium's libopus lookahead, which it leaves in).
The picture's times are a frame's count on a tick's time base, exact; the
sound's are its samples.

**The MP4** (`mp4::Muxer`) is the same from either encoder: the samples go
to the host's writer in chunks of about two seconds from a key frame of the
picture, every track's since the last chunk, and of each only its size,
duration, offset and key flag are kept. Once every sample is written, the
head is made (`Made`): the `ftyp`, a `moov` indexing every sample (its
tracks, their durations, the picture's edit list past its B-frames, a track
starting after 0 delayed by an empty edit) and a 64-bit `mdat` header. The
host puts the head first, so the file tells its length and seeks at once,
and plays a download from its start: no fragmented MP4. On the desktop the
samples go beside the file as `NAME.body.part`, then the head and they as
`NAME.part`, named once whole; in a browser to a file in the page's own
storage, then a download of the head and the file (docs/app.md §6).
Canceled or failed, nothing is left under either name.

**Progress, canceling, errors.** `progress` is called before the first frame
and after each with a `Progress` (the frames written and to write, so a
fraction, and the round being written), and answers
`ControlFlow::Break(())` to cancel. An export plays the replay on the thread
it is called on, so a host calls it on a thread of its own (a loaded game is
`Sync`: the thread borrows what the window plays from). That thread loads the
content's runtime first, if it hasn't one: about a second (§9), with nothing
written yet. `nettai_video::Error`
says what went wrong as a type, with the detail as its `Display`:

| | |
|---|---|
| `NoFfmpeg(e)` | ffmpeg couldn't be started: none bundled or installed, or not where `$NETTAI_FFMPEG` says |
| `FfmpegFailed { code, log }` | ffmpeg ran and failed: its exit code and the last of what it wrote to its standard error |
| `Encoder(said)` | the browser's encoder refused or failed |
| `Disk(e)` | the file couldn't be made there, or the disk is full |
| `Canceled` | the caller stopped it; nothing is left |
| `Replay(refused)`, `NoSuchPart(what)`, `Scale(n)` | the replay isn't played here; a round or range the replay hasn't; a size not 1 to 16 times |
| `Stopped { tick, why }` | the engine stopped before the part's end |
| `Threads(why)` | the threads the frames are presented on didn't start, or one stopped |
| `Write(e)` | the writer's own error |

**Checked.** nettai-video's tests check what the encoder is given (the
frames and samples the playing hands either encoder, kept by the tests in
the encoder's place): a picture a tick, each the one a player playing the
replay straight presents, and the sound what it plays, made 48 kHz as one
run, every frame's sound in its place (the picture's length to within a
sample); a part's the whole's for its ticks; each side, side by side, its
own video; both on one screen the viewer's both; the titles and chapters;
the progress and canceling; what can't be made refused before the encoder
starts. With stand-ins for ffmpeg (shell scripts), each child is written its
own track and nothing else (the frames' pixels, a track's samples as
little-endian floats), and a child that fails, fills the disk, closes its
input or never ends is said or stopped. With ffmpeg itself (9.0.2;
`ffmpeg_makes_the_mp4`, not run by default: it needs `ffmpeg` and
`ffprobe`), the MP4 is `ftyp`, `moov` and `mdat` and no fragments, holds
every frame as H.264, close to the frames, and the sound as AAC at 48 kHz,
decoding to the samples given (to within the millisecond the muxer's edit
lists are in) at no lag; and a two-sided MP4 has a track a side in order,
named, the first the default, the two decoding differently, and the rounds
as chapters from the frame the second's picture begins on.

## 9. Loading, and what a match waits for

The user asked for "a loading screen to load assets before starting the game to avoid stuttering" (2026-10-12;
docs/design/app-ui.md, "Loading"). This is what there is to load, when, what it costs, and how a host shows it.

**A game loads whole, and a match reads nothing.** `game::load` reads everything a battle of the game can use:
its content (every Luau module, defined), its assets extracted from its ROMs into memory (every sprite, the field,
the backgrounds, the HUD and the custom screen, and every song, instrument and sample: §1), the language's strings
and the text's font. Nothing is loaded by asset, so there is nothing to load by match: after the game's load no
tick, picture or sound reads or decodes a file. `nettai_content::load::reads()` counts the parts of the content a
thread has read (a game's content, a language's strings), and it stays where it was through a match. The one thing
a match may still wait for that isn't its game's is another game's background its rounds are drawn on (the
background pool's, §1): those are extracted the first time one is wanted, and a `Preload` waits for them (below).

**`game::Loading`** is `game::load` a part at a time, for a start-up bar: `Loading::new(name, options)`, then
`step()` until it says there is no more, `fraction()` meanwhile, `next()` for what the coming step loads, and
`finish()` for the `Loaded`. The parts and their shares of the whole (`game::Part::share`, estimated, not
measured: the content's times are 2026-10-12's, a release build on a busy machine; a game's extraction, its four
ROMs read and its recipe run, about two tenths of a second, 2026-10-27):

| Part | What | Share |
|---|---|---|
| `Content` | the game's Luau read and its define phase run (EXE6 1.7 to 3.3 s, EXE5 1.8 s, EXE4 1.3 s) | 62 |
| `Assets` | the game's assets extracted from its ROMs and checked against the content (`asset_numbers`, the timing compared) | 30 |
| `Graphics` | the graphics in the language (a copy with its lettering swapped in), the language's strings | 5 |
| `Font` | the text's font (the bundled one is in the program) | 1 |
| `Sound` | the content's songs in the bank (left out, with its share, when a host wants none) | 2 |

Each step runs one part to its end, so the bar moves in five jumps and the content's step is seconds long. A host
with threads steps the loading on one of them and its window stays live (nettai's `games.rs` does, with
`game::load`); a host without (the web: docs/app.md §6) steps it between frames and stalls for the content's
part. Making that part finer means loading the content's modules one at a time, which nettai-luau's loader
doesn't do: it runs the game's top module, which requires the rest.

**What a match's first tick did wait for** was made on the way, the first time it was wanted. Measured on the
same machine (the runtime: each game's, on a thread of its own; the rest: EXE6, three runs of 2,500 ticks with
every picture presented at 720 by 480, the least of the three for each tick, which leaves the machine's load
out):

| What | When it was made | Cost |
|---|---|---|
| The content's runtime on the thread | the first battle made on a thread, or the first build checked there (`check_match`, `pick::live`) | **EXE6 1.2 to 1.4 s, EXE5 0.8 s, EXE4 0.4 s** (EXE6's 0.8 to 2.7 s on the thread that had just loaded the game) |
| The player and the round's first battle | `Player::new` | 15 ms |
| A string's layout and glyphs, in the enhanced mode | the first frame each string shows on (a custom screen opening: the chips' names) | a picture 3.5 ms longer at most, against 0.9 ms a picture on average; 13 to 15 strings and 160 to 330 glyphs in a match's first 3,000 ticks. Since 2026-10-14 (glyphs as distance fields, made once for every size: docs/design/text-rendering.md §9.10) the preload makes the glyphs of both folders' names and descriptions and the navis' names, and a match makes about 1 glyph late |
| A tick | | 80 µs on average, 0.56 ms at most: nothing is made in a tick |

The runtime is the stutter: the worst wait there was, and the only one a player could see. Every thread that
makes or steps a battle keeps a Luau runtime of the content
(`nettai_battle::behavior`: a battle is plain data and the runtime is none of it), and loading one runs every
script of the game again, from the bytecode the define phase kept. The thread that loaded the game doesn't have
one either: the define phase's runtime isn't kept. nettai loads a game on a worker thread (`games.rs`), so its
window's thread has no runtime until a screen first checks a build or makes a battle there, and that is where it
waits, the window stalled (the wait is the runtime's above; it wasn't timed in the app).

**`preload`** takes that wait ahead:

- `preload::runtime(&content)` loads the calling thread's runtime if it hasn't one, and says whether it did. It
  must be called on the thread that will run the battles. It is one call of about a second and can't be stepped;
  a host calls it once a game, behind its start-up bar, the moment the game's `Loading` is done, and no screen
  after that waits for it.
- `Preload::new(&game, &set, viewer, (width, height))` is a match made ready a step at a time: `step()` runs the
  next step and says whether one is left, `fraction()` is how far the whole is, `done()` that it is. Its steps
  are in two kinds of part (`preload::Part`):
  - **`Part::Match`**, the match's own: the thread's runtime (the long step, and a step of no length once the
    thread has it), the round's first battle, its first picture as the viewer's console draws it (the arena, the
    background, both navis, the HUD: every lookup the frame makes, and its text laid out at the host's size),
    the other games' backgrounds its set's rounds are drawn on (`Backgrounds`: asked for as the preload is made,
    each game's extracted on a thread of its own while the rest is made ready, and done once they are; it asks
    and doesn't wait, so a page's main thread can step it), and the sound's drivers;
  - **`Part::Side(0)` and `Part::Side(1)`**, each side's: its navi (face, emblem, name), every form its navi
    lists (its face for each emotion, its name and description where the game lists forms in a window) and each
    chip of its folder in each code it is there in (icon, picture, name, the chip window's colors, description).
    These are looked up in the graphics already in memory, by the lookups a frame draws through
    (nettai-render's `lookups`), so a side's steps take microseconds and need no runtime: what they give is a
    bar a side and `missing()`.

  `step_of(part)` runs one part's next step alone and `fraction_of(part)` is that part's bar, so a loading screen
  with a bar a player fills each from its side's steps; the match's own part is where the time is (EXE6's
  match: 84 steps, 16 ms in all once the thread has its runtime). `missing()`
  lists what the steps looked up that the game's assets don't have (one line each, as the content audit words them),
  for a host to say before the match rather than have a picture missing in it; `stopped()` says why the round
  can't start, if the runtime didn't load or the engine stopped making the first battle.
- `preload.player(driver)` is the `Player`, made with the renderer and the text the steps used (so the first
  picture's text is laid out already, and the glyphs of every name and description of the folders and the navis
  are made: `TextRenderer::prepare`, one field a glyph for every size) and the sound. `Preload::with` takes a player's parts as `Player::with`
  does (a renderer of another language's graphics). The set is the one the driver plays: a match's
  (`Set::of`), a netplay agreement's, a replay's match's (`nettai_match::binary::read_match`).

Online, each peer preloads its own match and the lobby holds the start until both are done (§2, "Loaded before
the first tick"). The lobby says `Status::Loading` with the agreement: the host makes the `Preload` of
`agreement.set` for `agreement.side`, steps it, and tells the lobby how far it is, `Lobby::set_loading(fraction())`
after a step and `Lobby::set_loaded()` once `done()`. A peer's own bar is its `fraction()` (or a bar a side from
`fraction_of`), and the other player's bar is `Lobby::their_loading()`, the other peer's `fraction()` as it said
it. When the lobby says `Status::Start`, the player is `preload.player(driver)` over the `NetPlayer` on the
match's channel (`Lobby::channel`). A second match between the same two players (§2, "After a match") is
preloaded the same way: its sets differ (another seed, perhaps another build), and the thread has its runtime. The link and
the lobby are polled every frame meanwhile, so the runtime's step, which is about a second and can't be stepped,
is best taken before the lobby (`preload::runtime`, behind the game's start-up bar): a link nobody polls for 3
seconds reads as a dropped connection on the other side (docs/design/rollback.md §4.8). nettai-frontend's test
`an_invite_becomes_a_match` (`netplay::tests`) goes this whole way over WebRTC links, and its players load nothing
late.

**Held to it.** `Player::late()` says what a player made on the way since it was made (`preload::Late`):
runtimes loaded on its thread (`Behaviors::loads`), parts read from disk on it (`nettai_content::load::reads`), and its text's
strings laid out and glyphs' distance fields made for the first time. A player a preload made has loaded no
runtime and read nothing however long it plays; one made without has loaded one. The text's counts go up as new
strings show: a string's layout depends on its box, and its strings are an open set (damage numbers), so layouts
aren't made ahead, each a part of a millisecond; the glyphs of the folders' and navis' names and descriptions are
made ahead (a few milliseconds of the preload in English, 10 to 15 in Japanese), and what is left (a glyph of a HUD
line, say) is 8 to 80 µs a glyph (EXE6's match at 5x to 13x: 1 glyph late in 1,500 ticks, against 126 to 180
before).

- nettai-frontend's tests (`preload::tests`, on EXE6's content): `a_preloaded_match_loads_nothing_late` steps a
  preload on a thread without a runtime (one side alone first, then the rest), and plays 1,500 ticks of the set,
  custom screens and fights, with a picture a tick: `late()` is no runtime and no read.
  `a_match_that_was_not_preloaded_loads_its_runtime_late` is the same match without: one runtime, as its player
  is made.
- `cargo run --release -p nettai-frontend --example preload -- --game exe6` does it on a real game's assets: it
  copies the content, loads the copy with `Loading` (its assets extracted from `$NETTAI_ROMS`, printing each part's
  time), preloads a match (printing its steps), **deletes the copy from disk**, and then plays 3,000 ticks with every picture
  presented and every sample taken, beside a player of the same match made without the preload and given the
  same buttons. A file read lazily would be missing; the match plays to its last tick, every picture (its pixels
  and its text) and every tick's sound the same as the other player's, and the example fails unless `late()` is
  no runtime and no read. It then makes the same match's player on a thread that preloaded nothing and prints
  what that waits for (the runtime, from memory: the files are gone). All three games passed it with packs
  (2026-10-12), before the assets were extracted at each load.
- `a_set_on_another_games_background_waits_for_it` (`preload::tests`): a set on another game's background has
  that game's backgrounds extracted once, while it is made ready, and before the preload is done.

**What isn't done, and what it would take.**

- *Extracting by asset* (a match extracting only its navis', chips' and arena's sprites and sounds). The recipe
  would have to extract a sprite or a song on demand (`Bundle` and `SoundBank` are plain data every lookup borrows
  from), and the content would have to say what a chip can summon or play: the assets its modules name
  (`asset.sprite`, at load only) and those of every module they require. With a game's assets extracted in a few
  tenths of a second behind the start-up bar there is nothing to gain yet but memory.
- *The runtime in steps, or off the window's thread.* It is one load of every module. A host can keep the
  window live through it by running its `Player` on a thread of its own (a battle and its snapshots are `Send`;
  the runtime is whichever thread's steps it), which nettai doesn't.
- *One runtime for the loading thread.* A host that loads and plays on one thread (nettai-tool, the web) runs
  the game's scripts twice, once to define the content and once for the runtime: keeping the define phase's
  would save the second.
