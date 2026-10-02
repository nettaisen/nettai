# The rendering frontend (`nettai-frontend`)

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

## 1. The content pack

The battle content the engine runs on is this repository's content/bn6 (its
definitions; `--content <dir>` or `$BN6_CONTENT` for another). What the
frontend shows and plays comes from a content pack made from your own ROMs
(the US Falzar and Gregar, `MEGAMAN6_FXXBR6E` and `MEGAMAN6_GXXBR5E`, and the
Japanese Falzar and Gregar, `ROCKEXE6_RXXBR6J` and `ROCKEXE6_GXXBR5J`, which
have what the US release cut), never checked in: the graphics and the
sound, in open formats (indexed PNG, JSON, Tiled maps, MIDI, TOML, WAV; see
`docs/design/content-pack.md` and `docs/design/asset-formats.md`), by the
names the content gives them. Extract it once:

    cargo run -p bn6-extract -- content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> data/content/bn6

(`data/content/` is gitignored.) The frontend loads the pack at start-up
from `--pack <dir>`, else `$BN6_PACK`, else `data/content/bn6`, straight
from its files: the content with the pack's asset index into the engine's
`Content`, the graphics through nettai-content's importer, and, when a window
opens, the sound. `NETTAI_LOAD_TIMES=1` prints how long each part took.

The graphics load into the types of the `nettai-assets` crate, decoded
(tiles as palette indices, colours as BGR555):

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
  `art_palette` colours it) and the buttons',
  chip codes, element icons and their colours, damage digits, the slots'
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
    cargo run -p nettai-frontend -- <trace.jsonl> --headless 150,300,600 --out <dir>
    cargo run -p nettai-frontend -- <trace.jsonl> --audit      # what is missing?
    cargo run -p nettai-frontend -- --play --pack <dir>        # another pack
    cargo run -p nettai-frontend -- --play --host 7777         # netplay: host...
    cargo run -p nettai-frontend -- --play --join 192.0.2.10:7777   # ...and join

Options: `--pack <dir>` names the content pack and `--content <dir>` the
battle content (see above), `--mute` turns the sound off, `--round N`
starts a trace at round N (later rounds follow when a round's input runs
out), `--scale N` sets the window's first size (default 4 times 240x160;
the window can be resized, and the picture keeps whole pixels, centred on
black), `--paused` starts paused, `--png-scale N` scales headless output
(the text layer is drawn at that scale too), `--quit-after N` closes the
window after N ticks (with `NETTAI_WINDOW_SHOT=<file>` set, the window's
last picture is written there as a PNG), `--text font|original` chooses
how strings are drawn (default `font`; the frame comparison uses
`original`), `--font <file>` puts another TrueType or OpenType font in the
bundled one's place, `--lang en|ja` the language of the battle's words
(default `en`; §3, "Languages"). For live play, `--seed N` gives the seed its
setup and battle are drawn from (default: from the clock; each start
prints it), `--stage NAME` forces a link battle stage by its key
(`netbattle-1` to `netbattle-96`), `--show-folders` prints both folders,
`--cards` and `--their-cards` install your and the right navi's patch cards
(the Japanese games', docs/engine/patch-cards.md: names comma-separated in
the order they apply, `-name` switched off, e.g. `canodumb,-shadow`),
and with `--headless`, `--keys` holds buttons on given ticks (below).

Keys: arrows move, Z = A, X = B, A = L, S = R, Enter = START,
Backspace = SELECT; Space pauses, `.` steps one frame while paused, `-` and
`=` change speed (1/8x to 16x of 59.73 Hz), F5 restarts the round (none of
these in netplay), H toggles the status line, Esc quits.

**Trace playback** runs at the original's 59.73 frames per second until the
input ends or the engine hits something it doesn't implement yet. Then it
stops, shows the reason on screen and prints it; the first difference from
the trace's recorded state is printed too. Frame numbers are the trace's.

**Live play**: you are the left navi; the right one stands still. The round
is a netbattle on the pack's BN6 content between two 1000-HP MegaMen with
no NaviCust programs (so roads carry them and holes stop them;
`driver::live_navi`), set up at random from the seed
(`driver::bn6_live_setup`, which prints what it drew):

- **The field**: one of the 96 link battle stages the content defines (the
  settings records a link battle draws from, `sub_81209DC`: the stages with
  the link effect and not the random battle's), with a background drawn as
  a link battle draws one (`byte_8120A20`). The set's later rounds get
  theirs the same way.
- **A folder for each player**: 30 chips that keep BN6's folder rules
  (`folders`: the folder editor's, `sub_8135080` with `sub_8135500`: copies
  of a chip by its MB, five up to 19 MB down to one from 50; Mega and Giga
  chips within the navi's levels, 5 and 1; a code each chip comes in; a
  Regular chip within the navi's Regular memory, 50 MB; no tag chips), from
  the chips the chip pack lists (Standard, Mega and Giga, not the dark
  chips, and not the five the US game has no routine for). The codes lean to
  two the folder favours, and `*`. Each console shuffles its folder from
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
`+` picked, `-` greyed), OK and Beast Out, the picks and the Cross window's
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
  the same engine and the same content (`Content::hash`: the definitions,
  scripts and rule tables, and what the battle reads of the pack, the asset
  names and the animations' timing), and refuses a mismatch on both sides with what
  differs ("can't play: the other side plays other content (its hash ...,
  this one's ...)"). Each player then brings their own setup: a folder, a game
  and five Crosses drawn from their own `--seed` (as live play draws a
  player's), and their patch cards (`--cards`); the other player's is
  checked against the content (a legal folder, Crosses of MegaMan's, patch
  cards of the content's). The language (`--lang`) is each player's own. The
  field (stage, background, the set's later stages) and the battle's RNG
  come from both players' halves of the seed; the host's `--stage` forces the
  stage. Both print what was agreed.
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

**Audit mode** (`--audit`) draws every frame of the trace and plays every
sound cue into nothing, without a window, and lists what they named that
the pack or the content doesn't have, each with the frames it happened on:
a sprite that isn't in the pack, an animation or palette the sprite doesn't
have, a chip without an icon or with a name the font can't spell, a banner
without glyphs, a text line, a song. It exits 1 if there was any. It is the
quick check after a content or loader change: nothing is silently skipped.

**Sound**: the window plays each tick's sound cues through nettai-audio, with
the pack's sound, unless `--mute`; headless rendering never plays sound. In
netplay it plays the cue actions of each frame (plays, and cancels of cues
played on a wrong prediction). Other per-tick consumers can plug in the same
way, as a `TickHook` (`nettai_frontend::session`), which the window runs
after every step with the session.

## 3. What is drawn, and how

Layers, back to front: the backdrop colour, the background (priority 3),
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
  part's offset, the colour shader;
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
animations.

**HUD** (`hud.rs`), by the original's HUD tasks:

- the HP box with its rolling number and colours, and the custom gauge
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
  (rolling, coloured);
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
layer to black while navis change form (sprites keep their colours). A
palette flash takes the transformation's palette transform: variant 0
(`sub_80E10C0`) whitens the stage's palettes on the frames its counter has
bit 2 clear and leaves the HUD's colours, variant 1 (`sub_80E114C`, the
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
  window's colours), its picture and palette, the window's colours by its
  class (a dark chip's dark: no BN6 chip is one), its code, its element's icon and colours, its damage ("???" for
  Muramasa); for OK, Beast Out and the buttons their pictures;
- the slots (each dealt chip's icon and code, greyed or picked by its
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
  colours in the Cross window are its own game's (`custom::cross_picture`,
  for the form in the entry's place, `Unlocks::cross_at`), so a Gregar
  Cross shows Gregar's name in any window, and a window a setup's Cross
  list mixes shows each game's own; the Beast Out button, its picture in
  the chip window and the BeastOut chip's picture are of the Beast the
  navi goes into (`custom::beast_pictures`, `Unlocks::beast_game`), so a
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
the face and shadow colours of the palette the original draws them in;
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
English (`Bundle::in_language`), and the content's strings from the
content root's `locales/ja.toml` (chip names and descriptions, the
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

The bundled font is Murecho (`crates/nettai-frontend/fonts/murecho`, SIL
Open Font License 1.1, its licence beside it): Latin, kana and some 2,300
kanji, weight 700 for the 8x16 font's strings and 300 for the chatbox's.
A string too wide for its box is squeezed (to 85%, 70% with kana or kanji)
and then made smaller; it never leaves its box, and its layout never
reaches the simulation.

### What the engine gives the frontend

Presentation outputs: the simulation reads none of them (`digest.rs` lists
the ones the state digest leaves out).

- `object::sprite::Look` on every `Sprite` (palette, flips, shadow mode,
  white flash, colour shader, alpha, mosaic, priority, hidden parts),
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

The comparison needs the ROM, so it lives outside this repository, with the
lists of scenarios. The frontend's own tests (`cargo test -p nettai-frontend`)
use a small synthetic asset set and a live battle built in code, and the
bundled font for the text layer (its layout and the depth test; not its
pixels, which are floating-point arithmetic).

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
