# The frontend (`nettai-frontend` and `nettai-render`)

A desktop app that runs a battle through the native engine and draws it the
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

Two crates make it up:

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
- **nettai-frontend** is the app around it: the window and the keys
  (`app`, with the status line's small font, `text`); the sessions
  (`session`) and what drives them (`driver`: a trace, live play, a match
  file; `netplay`); the sound, a `TickHook` to nettai-audio, and the
  audio's own lookups (`sound_lookups`); the command line (`main`);
  headless output (`headless`) and the audits (`content_audit`,
  `headless::audit_traces`).

## 1. The content pack

The battle content the engine runs on is this repository's content/ (its
packs: the game packs content/bn6 and content/bn5 and the support pack
content/exelib, docs/design/content-model-v2.md §4.0; `--content <dir>` or
`$NETTAI_CONTENT` for another content directory). What the
frontend shows and plays comes from a content pack made from your own ROMs
(the US Falzar and Gregar, `MEGAMAN6_FXXBR6E` and `MEGAMAN6_GXXBR5E`, and the
Japanese Falzar and Gregar, `ROCKEXE6_RXXBR6J` and `ROCKEXE6_GXXBR5J`, which
have what the US release cut), never checked in: the graphics and the
sound, in open formats (indexed PNG, JSON, Tiled maps, MIDI, TOML, WAV; see
`docs/design/content-pack.md` and `docs/design/asset-formats.md`), by the
names the content gives them. Extract it once:

    cargo run -p bn6-extract -- content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> data/content/bn6

(`data/content/` is gitignored.) **You play one game**, BN6 or BN5: a match
is of one game (§6), which a match file names (`game = "bn6"`), else
`--game GAME` (default `bn6`); with a trace, `--game`'s. The battle is that
game's content, drawn and heard from its pack: there is no mixing of games,
no other game's chip, navi or field art. The frontend (and the editor)
finds at start-up
**every pack in the packs directory**, `$NETTAI_PACKS`, else
`data/content`: each folder with a pack manifest, by the game the manifest
says (`nettai_content::pack::find`); `bn5-extract content` writes BN5's into
`data/content/bn5` beside it. `--pack <dir>` names a pack elsewhere, in
place of the found one of its game, and can be given again for another
game's; `$BN6_PACK`, the BN6 pack's directory, still works the same way
(deprecated: the verification's tools set it). Two packs of one game in the
directory are an error. Each pack loads straight from its files: its asset
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
editor, the tools' `load_battle`, the BN5 replays, the static audit), and
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
  frame. A pack extracted before it loads without them (with a warning),
  and the screen isn't drawn.
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

## 2. Running

    cargo run -p nettai-frontend -- <trace.jsonl>              # watch a trace
    cargo run -p nettai-frontend -- --play                     # play live
    cargo run -p nettai-frontend -- --play --seed 42 --stage netbattle-43 --show-folders
    cargo run -p nettai-frontend -- --match match.toml           # play a match file (§6)
    cargo run -p nettai-frontend -- --play --seed 42 --save-match match.toml   # keep the draw
    cargo run -p nettai-frontend -- <trace.jsonl> --headless 150,300,600 --out <dir>
    cargo run -p nettai-frontend -- --audit-content            # what is missing?
    cargo run -p nettai-frontend -- --audit <trace.jsonl>...   # and in these traces?
    cargo run -p nettai-frontend -- --play --pack <dir>        # a pack elsewhere
    cargo run -p nettai-frontend -- --play --host 7777         # netplay: host...
    cargo run -p nettai-frontend -- --play --join 192.0.2.10:7777   # ...and join

Options: `--pack <dir>` names a content pack elsewhere and `--content <dir>`
the battle content (see above), `--mute` turns the sound off, `--round N`
starts a trace at round N (later rounds follow when a round's input runs
out), `--scale N` sets the window's first size (default 4 times 240x160;
the window can be resized, and the picture keeps whole pixels, centered on
black), `--paused` starts paused, `--png-scale N` scales headless output
(the text layer is drawn at that scale too), `--quit-after N` closes the
window after N ticks (with `NETTAI_WINDOW_SHOT=<file>` set, the window's
last picture is written there as a PNG), `--text font|original` chooses
how strings are drawn (default `font`; the frame comparison uses
`original`), `--font <file>` puts another TrueType or OpenType font in the
bundled one's place, `--lang en|ja` the language of the battle's words
(default `en`; §3, "Languages"). For live play, `--seed N` gives the seed its
setup and battle are drawn from (default: from the clock; each start
prints it), `--game GAME` plays BN6 (default) or BN5, `--stage NAME`
forces a link battle stage by its name in the game (`netbattle-1` to
`netbattle-96`), `--show-folders` prints both folders,
`--cards` and `--their-cards` install your and the right navi's patch cards
(the Japanese games', docs/engine/patch-cards.md: names comma-separated in
the order they apply, `-name` switched off, e.g. `canodumb,-shadow`),
and with `--headless`, `--keys` holds buttons on given ticks (below).
`--match FILE` plays a match file instead of a random draw (§6: you are its
left side; it names the game, the stage and the patch cards, so `--game`,
`--stage` and `--cards` don't go with it; `--seed` overrides its seed), and
`--save-match FILE` writes the match played, live play's draw or the one
netplay agreed, with its seed, as a match file.

Keys: arrows move, Z = A, X = B, A = L, S = R, Enter = START,
Backspace = SELECT; Space pauses, `.` steps one frame while paused, `-` and
`=` change speed (1/8x to 16x of 59.73 Hz), F5 restarts the round (none of
these in netplay), H toggles the status line, Esc quits.

**Trace playback** runs at the original's 59.73 frames per second until the
input ends or the engine hits something it doesn't implement yet. Then it
stops, shows the reason on screen and prints it; the first difference from
the trace's recorded state is printed too. Frame numbers are the trace's.

**Live play**: you are the left navi; the right one stands still. The round
is a netbattle on BN6's content between two MegaMen at their fresh stats
(100 HP, as a new match's in the editor: `nettai_match::Side::fresh`) with
no NaviCust programs (so roads carry them and holes stop them), set up at
random from the seed (`nettai_match::draw::live`, which prints what it
drew), unless a match file sets it up (`--match`, §6). (`--game bn5` draws a plain BN5 match instead:
BN5's stock rules, a stage of its link battles, and on each side BN5's
MegaMan at his fresh stats with a folder its rules accept.)

- **The field**: one of the 96 link battle stages the content defines (the
  settings records a link battle draws from, `sub_81209DC`: the stages with
  the link effect and not the random battle's), with a background drawn as
  a link battle draws one (`byte_8120A20`). The set's later rounds get
  theirs the same way.
- **A folder for each player**: 30 chips that keep BN6's folder rules
  (`nettai_match::folders`: the folder editor's, `sub_8135080` with `sub_8135500`: copies
  of a chip by its MB, five up to 19 MB down to one from 50; Mega and Giga
  chips within the fresh navi's levels; a code each chip comes in; a
  Regular chip within the fresh navi's Regular memory, if a chip fits it; no
  tag chips in a draw), from
  the chips the chip pack lists (Standard, Mega and Giga, not the dark
  chips, and not the five the US game has no routine for). The codes lean to
  two the folder favors, and `*`. Each console shuffles its folder from
  the seed at the round's init, as before.
- **Five Crosses for each Cross window**, drawn from MegaMan's ten, both
  games' (the setup's Cross list, nettai's extension:
  docs/engine/custom-screen.md §4.1). A Cross of the other game is its own
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

**Netplay** (`--play --host PORT` or `--play --join ADDR:PORT`) plays another
player over the network, with rollback (docs/design/rollback.md §4; the
frontend's side is `netplay`):

- **Hosting**: `--host 7777` listens on UDP port 7777 of every IPv4
  interface and waits for a player (`--wait SECONDS`, default 300). On a LAN
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
  side plays bn5, this one bn6: a match is of one game, both sides playing
  it") and the same content (`Content::hash`: the definitions, scripts and
  rule tables, and what the battle reads of the pack, the asset names and
  the animations' timing), and refuses a mismatch on both sides with what
  differs ("can't play: the other side plays other content (its hash ...,
  this one's ...)"). Each player then brings their own side of the match (an
  offer, by name in the game as a match file names things: protocol version
  6): a match file's left side and ruleset (`--match`), or a folder, a
  version and five Crosses drawn from their own `--seed` (as live play draws
  a player's) with their patch cards (`--cards`), by the game's stock rules;
  the other player's is checked against the content as a match file's side
  is (`nettai_match::check_side`, §6), and both must bring the same ruleset
  (a match has one). The language (`--lang`) is each player's own. The field
  is the host's: its match file's arena, else drawn from both players'
  halves of the seed (on the host's `--stage`, if it names one); the
  battle's RNG comes from the halves of the seed. Both print what was
  agreed, and `--save-match` writes it.
- **Playing**: the match is a best-of-three set; its rounds follow one
  another (the folders shuffled again by each console's RNG). Every frame the
  frontend sends your buttons and shows the frame its rollback session
  presents: your input shows after the input delay (`--delay N`, default 2
  frames), the other player's is predicted until it arrives, and the frame
  is simulated again when a prediction was wrong. A cue played on a wrong
  prediction is stopped or taken back (rollback.md §3.2). There is no pause,
  speed change or restart (F5) in netplay.
- **The status line** (H toggles it) shows `PING` (the round trip, in
  milliseconds), `LOSS` (the share of the other player's datagrams that
  were lost), `DELAY` (the input delay), `ROLLBACK` (the last rollback's
  depth), `MAX` (the deepest, and in brackets how many) and `WAIT` (frames
  held for clock sync or the stall guard).
- **The end**: when the set is over the result shows on the status line and
  the window stays open; Esc leaves, and tells the other player. If the
  other player leaves, nothing arrives from them for 10 seconds, or their
  input falls more than the rollback horizon behind, the match stops with
  the reason on screen.

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
(nettai-render's `lookups.rs`; the audio's, a cue's song, nettai-frontend's
`sound_lookups.rs`), which notes it (`audit::Lookup`) and checks it once a
run, so both audits make the lookups a frame makes, through the same
functions:

- `--audit-content` (`content_audit.rs`) makes every lookup for everything
  the content defines, in every language it has strings in: every chip's
  icon, picture, name, Program Advance name and code, its window's class,
  element and code pictures, its description in the dialogue font; every
  navi's face, emblem on either game's console, name and no-running message
  with its portrait; every form's face for each emotion, every Cross's name
  and description; and every asset of the loaded packs (each sprite with
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
  not counted.
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

**Sound**: the window plays each tick's sound cues through nettai-audio, with
the pack's sound, unless `--mute`; headless rendering never plays sound. In
netplay it plays the cue actions of each frame (plays, and cancels of cues
played on a wrong prediction). Other per-tick consumers can plug in the same
way, as a `TickHook` (`nettai_frontend::session`), which the window runs
after every step with the session.

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
  leaves it hidden (the state after loading) draws incomplete, or nothing;
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
doesn't (a pack extracted before it had it) is the owner's normal panel,
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
  class (a dark chip's dark: no BN6 chip is one), its code, its element's icon and colors, its damage ("???" for
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
  stage and the objects fade a quarter of the way;
- the scrap and the re-deal: the column losing the scrapped picks, the
  slots dealt again, the emblem and the Regular chip's frame throughout;
- a console's own pictures by its version (`Versioned`: a Gregar console's
  Beast and emblem, the pack's `-gregar` assets); a Cross's name and
  colors in the Cross window are its own game's (`custom::cross_picture`,
  for the form in the entry's place, `bn6_compat::Unlocks::cross_at` over
  the cross system's setup), so a Gregar
  Cross shows Gregar's name in any window, and a window a setup's Cross
  list mixes shows each game's own; the Beast Out button, its picture in
  the chip window and the BeastOut chip's picture are of the Beast the
  navi goes into (`custom::beast_pictures`, `bn6_compat::Unlocks::beast_game`), so a
  Falzar player in HeatCross sees Gregar's;
- what the screen does to the rest: the HP box and the mugshot move right
  with the window and the field and the sprites 15 pixels down (the
  camera), the gauge and the HUD's "????" stay off until the local result
  is sent, Beast Out's
  fade darkens the stage, the HUD layer and the objects (sprite palettes
  0-10) half way, the camera's jitter moves the HUD layer in Beast Out's
  states, and the emotion window shows the Beast form chosen. A dark
  chip's hover (never seen in BN6) darkens the stage and the objects on the
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

**A BN5 console.** In a BN5 match the HUD, the custom screen and the
chatbox are BN5's pack's: its HP box, gauge, fonts, banners, emotion
window, window and chatbox, by the pack's own tile numbers and layout
(docs/design/asset-formats.md §4, "Another game's HUD and custom screen").
What a BN5 console does otherwise, by data, not by game:

- the emotion window: the faces BN5's forms name bring their own box
  (MegaMan's five, Team Colonel's), and a soul's face shows the soul's
  turns left beside it (the souls system's `turns`, read by name);
- the custom screen: the special slot's button is the one the pack names
  for the system's button (`soul`: Soul Unison's, its picture in the chip
  window in Chaos Unison's palette for Chaos), the cursor over OK and over
  the button where the pack's layout puts them; the soul choice (the
  souls system's window `soul_unison`, BN5's state 9) flies the soul's icon
  up onto the column's first cell under its flash, and the cell keeps it
  (nettai-render's `SoulOffer`: the offer and the window's step, read of the
  system's state by name);
- its game's flow (rules `flow`, read of the console's own game): the
  custom screen's close starts the chip window as a Japanese BN6 console's
  does (`chip_window_at_close`), the intro fades in from black
  (`intro_from_black`);
- a chip each version draws its own way shows the console's version's
  icon and picture, and the emblem is the console's version's
  (`Renderer::console_version`, which a BN5 recording names; live play,
  the pack's first version).

The frame comparison against BN5's consoles (verification's
tools/frontend-compare/bn5.txt, chiplab's library-bn5) and what still
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
frontend's own text (live play's status line, folder listings) is the
content's own strings.

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
labels) and a Japanese console's HUD timings (the next chip's name shown
from the fight's first frame), the chips' pictures included; the Gregar
chip's on a Falzar console is the known difference above.

**On BN5 consoles** (verification's tools/frontend-compare/bn5.txt: 16 of
chiplab's library-bn5 scenarios, Team ProtoMan against Team Colonel,
traced on the Team ProtoMan console and once on the Team Colonel one,
Soul and Chaos Unison and the computer navi's Chaos among them, compared
with `--text original`, BN5's pack loaded beside BN6's): of 7,964 frames,
7,914 are pixel-exact, the navis included, and 12 scenarios wholly: the
HUD, the emotion window (a dark chip user's flicker too, now that the
console's RNG1 is the engine's: the dark chip offer, bn5-map.md §15.3
item 13), the custom screen with its picks, Soul Unison's choice and a
dark chip's hover, its close (the hand's name and icons on the tick the
results are in), the chatbox, the banners, the mercy flash and a
deletion's result. What still differs:

- the UNITE button for a soul not offered yet (HeatSoul for AntiFire, a
  pick in custom/picks) is gray where BN5's is lit;
- a soul's buster shot's flame is whiter for a few frames (Soul and Chaos
  Unison, the computer navi's Chaos);
- an explosion's colors in two frames of AntiFire's.

The comparison needs the ROM, so it lives outside this repository, with the
lists of scenarios. The frontend's own tests (`cargo test -p nettai-render
-p nettai-frontend`) use a small synthetic asset set and a live battle
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
- **Deliberate: what the US release cut, on a US console.** The pack has
  the Japanese ROMs' art where the US ROMs have a placeholder (six sprites,
  eleven chips' pictures: asset-formats.md §4), and the content draws Otenko's
  statue and CrosOver's gun with the Japanese games' sprites (0C-49, 0C-0F)
  where the US games draw others; a US console's original shows its
  placeholder (a purple picture, a dot) or its other sprite. The headless
  frontend lists such places as known (`known.tsv`): a chip's picture from
  another region's ROMs, and around an object drawn with a sprite of another
  region's ROMs, 48 pixels around it (`objects::OTHER_REGION_MARGIN`: the
  US's sprite there may reach past the Japanese one). The console's region is
  the trace's (`game_regions`; US without it).
- **Deliberate: the Gregar and Falzar chips' pictures.** Each Japanese ROM
  has one picture for both chips, its own beast; the pack has each chip's
  own (Gregar's from the Japanese Gregar ROM, Falzar's from the Japanese
  Falzar ROM), on either console. A Japanese Falzar console's Gregar chip
  (and a Gregar console's Falzar chip) is a known difference.
- **Deliberate: the Gregar and Falzar chips' descriptions.** Their scripts
  copy the text from a buffer the console keeps (`FF 01 01`, the same
  buffer for both), which holds the console's own Giga chip's: a Falzar
  console shows "Falzar's ruinous tornado!" for the Gregar chip too. The
  frontend shows each chip's own (its definition's, the user's text), so
  the Gregar chip's description differs on a Falzar console (and the
  Falzar chip's, presumably, on a Gregar console). Not listed as known: no
  content says which chips copy that buffer.
- A BN5 console in Japanese: the BN5 pack has the US ROMs' lettering
  alone (docs/design/bn5-map.md §11), so `--lang ja --text original`
  draws its Japanese names in the US font (the font mode draws them); the
  static audit says so, not counted.
- Live play shows the custom screen as text.
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
game, the ruleset both sides play by, the arena, and each side's navi,
version, navi code level, stats, folder, Crosses, Beast Out, SP deletion
times, patch cards and NaviCust, in TOML. **A match is of one game**, named
once at the file's top: everything else is a name in that game's namespace
(`cannon`, `megaman`, `netbattle-43`), looked up there alone
(`nettai_match::ids`), so a match can't name another game's chip, navi,
soul, patch card or stage: a name the game hasn't is said as any unknown
name is ("left: folder entry 3: no chip \"darkthnd\" in bn6"), whether
another game has it or not. `--match FILE` plays one (you are its left
side), `--save-match FILE` writes the match played, and nettai-editor makes
and edits them (README.md, "The match editor"). The crate `nettai-match`
reads, checks and writes them, and builds the round (`Match::round`); live
play's random draw is a match too (`nettai_match::draw::live`), so a drawn
setup written out and played again is the same battle (the frontend's test
`a_saved_match_plays_the_same_battle` compares the digest every tick).

```toml
game = "bn6"                               # the match's game: everything below is its
ruleset = "stock"                          # optional: both sides' rules, one of the game's (else its stock)
seed = 42                                  # optional: the setup's and battle's seed

[arena]
stage = "netbattle-43"                     # a link battle stage of the game's
background = "honeycomb"                   # optional: else the stage's own
later = [                                  # optional: the set's later rounds (else the first's)
    { stage = "netbattle-12", background = "code" },
    { stage = "netbattle-7" },
]

[left]                                     # you (side 0); then [right]
navi = "megaman"
version = "gregar"                         # optional: falzar (default) or gregar
crosses = ["heatcross", "spoutcross"]      # optional: else the version's own five
beast_out = false                          # optional: else Beast Out is unlocked (the save's flag 0xE0)
cards = [{ card = "canodumb" }, { card = "shadow", on = false }]
level = 0                                  # optional: the navi code's level, 0-14 (see below)
bug_frags = 0                              # optional
folder = [                                 # 30 entries, [chip, code] ([] empty: a folder being made)
    ["cannon", "A"],
    ["cannon", "A"],
    ["airshot", "*"],
]
regular = 4                                # optional: the Regular chip's entry, counting from 0
tags = [5, 6]                              # optional: the tag chips' entries

[left.sp_times]                            # optional: how fast the save deleted each SP navi
"sp/heatman" = "00:12.34"                  # mm:ss.cc, by the rules' slot (else the fastest, 00:00.00)

[left.stats]                               # optional: what differs from the navi's fresh stats (a link navi's at its level)
hp = 1000
regular_memory = 50
sun = true

[left.navicust]                            # optional: MegaMan's NaviCust, compiled into his stats
expansions = 2                             # optional: the board, 0 (4x4) to 2 (5x5, the default)
programs = [                               # in the save's order; x, y the center on the 7x7 grid
    { program = "suprarmr", color = "red", x = 2, y = 3 },
    { program = "undersht", color = "white", x = 5, y = 3, rotation = 1 },   # quarter turns
    { program = "hp-100", color = "pink", x = 3, y = 1, compressed = true },
]
```

A BN5 match (`game = "bn5"`: its stock rules take no version and have no
Crosses) names BN5's navis, chips, patch cards and NaviCust programs, and
its sides may say besides:

```toml
[left]
karma = 100                                # optional: the light/dark value, 0 to 1000 (default 500; dark under 470)
souls = ["protosoul", "colonelsoul"]       # optional: the souls it has, BN5's, either version (none: every soul)

[left.tactics]                             # optional: BN5's computer-navi data (none: empty)
entries = ["cannon", "pattern 1", "nothing", "empty"]   # up to 42, in the save's places
patterns = [{ dx = 1, dy = 0, chips = ["sword", "wideswrd"] }]   # up to 8, each up to 6 chips
```

**The stats block** (`nettai_match::stats`) sets the navi's stats by name
over its fresh stats (`NaviStats::fresh`, `init_8013B64`: what a new save
gives the navi), of the side's version; a link navi's over its stats at its
`level`, as the PET's reload gives them (`nettai_match::link_navis`,
docs/engine/link-navis.md: the base HP of the cleared game and the level's
HP, buster levels, custom and Mega levels and abilities): `hp` (the base HP, which also sets the
maximum and the HP the round starts with; `max_hp` and `current_hp` set
those apart), `attack`, `rapid`, `charge`, `custom_level`, `mega_level`,
`giga_level`, `regular_memory`, `mood`, `element`, `beast_out_counter`,
`sun`, the NaviCust's abilities (`super_armor`, `float_shoes`, `air_shoes`,
`undershirt`, `status_guard`, `first_barrier`, `gauge`, `supports`,
`chip_recovery`, `chip_shuffle`, `number_open`), the weapons (`buster`,
`charged_shot`, `back_special`, `a_charge`, `mode9_a`) and shot programs
(`buster_shot`, `charged_shot_program`) by name or `none`, the forms, and the
NaviCust's bugs (`step_bug`, `panel_trail`, `panel_trail_level`,
`buster_blanks`, `buster_charged`, `hit_status`, `hp_drain`,
`custom_drain`, `battle_start_bug`, `emotion_bug`, `starting_damage`,
`custom_damage`, `hand_shrink_turn`, ...): every stat a round starts from,
so a written block gives back the same stats. Writing a match, only the
fields that differ are written.

**The emotion window's glitch** (the save's event flag 0x1720, 0x1723
with patch cards; BN5's 0x10C1 and 0x10C4: MegaMan's window flickers) is
no key of a match and no field of a setup: the game's rules make it as the
round is set up. The NaviCust's compile sets it when a bug applies, the
patch cards' routine from the stats they leave, and for a side with no
NaviCust (its stats set directly, or a recording's, which are as a compile
left them) it is set when the stats carry a NaviCust bug
(content/exelib/navicust/compile.luau). BugFix clears it.

**The navi code's level** (`level`) is the level of the navi code the
save received (docs/engine/link-navis.md), 0 to 14. A link navi exists only
through its code, so it always has one: without `level` it is **0**, its
stats are its reload's at level 0 and its chip bonus is level 0's. MegaMan
without `level` has **none** (no code received, 0xFF in the battle); with
one he was received from a navi code: his level's gains go over his
NaviCust, and, as the game's event flag 0x163 does, his custom screen has
no Beast Out button and his Cross window stays his even with a gauge for
each player. The checks refuse a level past 14 and a link navi without one;
a file written leaves out the navi's default (a link navi's 0, MegaMan's
none).

**The SP deletion times** (`[left.sp_times]`) are by the SP navi slots of
the match's rules (BN6's `sp/heatman` to `sp/colonel`, rules/sp-chips.luau),
each `mm:ss.cc`; a slot left out is the fastest. The game keeps frames and
shows them as a time rounded down to the hundredth (`sub_8000D84`): a
written time is the fewest frames that show as it, so a time the game shows
reads back as itself. The SP navi chips' damage goes by them
(`sub_8010AE4`).

**A save** (the editor's "Import from save…", `Match::import_save`, into a
match of the save's game: a save of the other game makes a new match of its
game first) of BN6, a .sav as an emulator keeps it, read by
`bn6_compat::save`, gives a side its version, Beast Out and the Crosses it
owns (as a Cross list, unless it owns all
five), the navi code's level (a link navi keeps its own when the save has no
code) and the SP times; its folder, NaviCust, patch cards and stats are not
read yet.

**The NaviCust** (`[left.navicust]`, docs/design/navicust.md) is the
programs placed on MegaMan's grid, by name and color name (a program's
`colors`). With one, the stats block is the save's stats before the NaviCust:
only what a save keeps through the NaviCust's reload (`hp`, `regular_memory`,
`mood`, `beast_out_counter`, `sun`, `form` and the folder fields), since the
ruleset's `navicust` system makes the rest (the abilities, levels, weapons and
bugs) from the programs as the round is set up. Without one, the stats block
is the stats as they are, NaviCust included, as a recording's are. The
editor's NaviCust pane places the programs on the board as the game does.

**The karma** (`karma`, `nettai_match::facts`) is BN5's light/dark value
(NaviStats +0x44), 0 to 1000; without it, **500**, a fresh save's
(0x08010C00): light for the chips, the starting mood 0x80, no holy panels
cleared. Under 470 a dark MegaMan (mood 0, the dark face and palette, dark
chips usable in a link battle, no soul button); 499 or under clears holy
panels; under 500 he starts worried; 1000 the brightest (mood 190, Tango's
light templates). Like BN6's `version`, `crosses` and `beast_out` (S6c's
facts), the round's setup writes it into whichever of the rules' systems
declares the setup field (`PlayerSetup::set_fact`): BN5's light and dark
system's `karma`. A ruleset that takes none refuses one other than 500.
Hub Style (NaviStats +0x4C, which BN5's patch card 111 sets) waits for
BN5's patch cards. A netplay offer carries the karma and the souls
(protocol version 5), and a round's setup and the battle's digest hold
them, so both peers start alike (protocol version 6 names them in the
game).

**The souls** (`souls`, `nettai_match::facts`) are the souls the side has,
BN5's Soul Unison, by name: those the custom screen's soul button may
offer. Without `souls`, every soul of the game (both versions'); with a
list, those; an empty list, none (the button never lit). The original's soul
button (0x08024B28) offers the soul of the last chip's family when the save
has it: each version's table (0x08024BF0) gives Team ProtoMan's souls 1 to 6
the event flags 2 to 7 and Team Colonel's 7 to 12 the flags 8 to 0x0D, the
other version's none, and a dark chip's Chaos Unison needs flag 0x236 too.
The engine ports that check on the souls owned: the round's setup writes
the side's into the souls system's setup field `souls` (`set_fact`), and the
battle reads them as the save's flags, by each soul's number. A side may have
any of the game's souls, of either version (nettai's extension, as a Cross
list may name either version's), and a soul whose family the folder never
holds never comes up. Only a ruleset whose systems take `souls` takes a list
(the checks refuse one elsewhere, and a form that is no soul).

**Soul Unison and Chaos Unison** (`soul_unison`, `chaos_unison`) are the
save's event flags 0 and 0x236: the soul button at all, and a dark chip's
Chaos Unison. Both are on unless a side says (`soul_unison = false`), as a
finished save has them; the round's setup writes them into the souls
system's setup (its defaults, on, for a setup that says nothing). A ruleset
that takes neither refuses one off. The netplay offer carries them
(protocol version 7).

**A BN5 save** (the editor's "Import from save…", `Match::import_save`,
which reads a save that isn't BN6's as BN5's: a .sav, or a raw save image as
Tango's netplay templates hold, read by `bn5_compat::save`) makes the match
BN5's and gives its karma, the souls its version's flags give (BN5's
souls of those numbers), and its Soul Unison and Chaos Unison.

**The tactics** (`[left.tactics]`, nettai_battle::tactics, docs/design/bn5-map.md
§15.9) are BN5's computer-navi data, the block a BN5 save keeps for its
player: what a computer navi across from them plays, BN5's Dark MegaMan,
whom a failed Chaos Unison brings. The entries are in the save's places,
each a chip's name, `pattern N` (one of the patterns, from 1), `nothing` (a
save's 0) or `empty` (an empty place); a pattern is a place by the target
(`dx` columns toward the computer navi's enemies, `dy` rows) and the chips
used there. As the round is set up each side's are sent as the console sends
them (0x0802C7BE: the first three places and the next 39 shuffled, packed to
the front), from a stream of the side's own from the seed. With none, Dark
MegaMan only steps into an enemy's row and fires his buster (three shots)
between rests. A netplay offer carries them (protocol version 3).

**The checks** (`nettai_match::check`) run when a file loads, when a netplay
offer arrives (the same `check_side`), and live in the editor; each problem
is said with where it is:

- the game is one the content has, and every name names a definition of
  the game's (a stage, a background of its pack, a ruleset, a navi, a
  Cross, a patch card, a chip, a NaviCust program, a soul, a weapon, a
  record, a form), and every stat is in range; a match made in memory
  holding another game's (no file or offer can) is refused the same way
  ("right: a navi bn5 hasn't");
- the arena's stages are the game's link battle stages (`link_battle_stages`);
- the folder keeps the game's rules, which the match's ruleset checks: the
  ruleset's systems' `folder_check` hooks (BN6's are rules/folder/init.luau:
  30 chips, so a folder being made, with empty entries, is no folder yet;
  copies by MB, each chip in one of its codes, at most three dark
  chips, chips the chip pack lists, the Regular chip within the Regular
  memory, the tag chips two other entries of 60 MB together at most). The
  Mega, Giga and Regular limits are the navi's stats as the round starts them
  (after the rules' `round_setup`: the NaviCust's and the patch cards' folder
  limits, as the original's folder editor and link battle check read the
  reloaded stats). Rust only asks (`Battle::check_folder`) and reports what
  the hooks say, so another game's folder rules are its own Luau: BN5's
  (content/bn5/rules/folder/init.luau, its folder editor's) are four copies
  of a Standard chip and one of a Mega, Giga or dark chip, the Mega and Giga
  levels, at most three dark chips, the chips its pack lists, the Regular
  chip within the Regular memory, and no tag chips. A folder holds the
  game's chips alone (the rules' pool is the game's). Live play's random
  folders are drawn from the rules' pool and kept only
  when the hooks accept them (`nettai_match::folders`);
- a NaviCust only with a ruleset that has the navicust system, and only for
  MegaMan; every program fits the board, none overlaps another, the copies of
  one program in one color are all compressed or all not (the save keeps
  one flag for them), and the stats block holds only what a save keeps;
- a Cross list only with a ruleset that has the forms system, of the navi's
  Crosses (a navi that changes form), at most five, none twice;
- a soul list only with a ruleset that has the souls system, each a soul
  of the game's (a form of kind `soul`), none twice, of either version;
- karma 0 to 1000, and other than 500 only with rules that take it;
- patch cards only with a ruleset that has the patch-cards system, each
  installed once, at most 32, their MB together at most 80 (BN6's menu adds
  none past 80 MB, `0x08141868`);
- the round starts (`Battle::new` doesn't stop).

**Netplay with a match file** (`--play --host PORT --match FILE` or
`--join`): the file's left side is what you bring, wherever netplay puts
you, and the host's file's arena is the match's (the joiner's is not
sent). The battle's RNG still comes from both players' halves of the seed.
