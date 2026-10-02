# The rendering frontend (`nettai-frontend`)

A desktop app that runs a battle through the native engine and draws it the
way the original does, in its 240x160 frame scaled up by an integer factor.
It draws from engine state: the field's panels, each object's sprite,
animation frame and look, HP, the custom gauge, banners and the flow's
screen fades. Nothing emulates the GBA's video hardware; the frontend
composes tile layers and sprite parts itself with the original's ordering
and blending rules.

It replays a golden trace (the recorded inputs of a real match) or is
played live from the keyboard, and can render chosen frames to PNG.

## 1. The content pack

The battle content the engine runs on is this repository's content/bn6 (its
definitions; `--content <dir>` or `$BN6_CONTENT` for another). What the
frontend shows and plays comes from a content pack made from your own ROM
(US Falzar, `MEGAMAN6_FXXBR6E`), never checked in: the graphics and the
sound, in open formats (indexed PNG, JSON, Tiled maps, MIDI, TOML, WAV; see
`docs/design/content-pack.md` and `docs/design/asset-formats.md`), by the
names the content gives them. Extract it once:

    cargo run -p bn6-extract -- content <falzar-rom> <gregar-rom> data/content/bn6

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
  patches, each chip's picture (`chip-art/<chip>.png`) and the buttons',
  chip codes, element icons and their colours, damage digits, the slots'
  codes and buttons, the cursor, the navis' emblems, the Regular chip's
  frame. A pack extracted before it loads without them (with a warning),
  and the screen isn't drawn.

What the HUD shows of the content comes from the content: a chip's name is
its definition's, spelled with the font's glyphs (`Hud::glyphs`); its icon
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

Options: `--pack <dir>` names the content pack and `--content <dir>` the
battle content (see above), `--mute` turns the sound off, `--round N`
starts a trace at round N (later rounds follow when a round's input runs
out), `--scale N` sets the window scale (default 4), `--paused` starts
paused, `--png-scale N` scales headless output, `--quit-after N` closes
the window after N ticks. For live play, `--seed N` gives the seed its
setup and battle are drawn from (default: from the clock; each start
prints it), `--stage NAME` forces a link battle stage by its key
(`netbattle-1` to `netbattle-96`), `--show-folders` prints both folders,
and with `--headless`, `--keys` holds buttons on given ticks (below).

Keys: arrows move, Z = A, X = B, A = L, S = R, Enter = START,
Backspace = SELECT; Space pauses, `.` steps one frame while paused, `-` and
`=` change speed (1/8x to 16x of 59.73 Hz), F5 restarts the round, H toggles
the status line, Esc quits.

**Trace playback** runs at the original's 59.73 frames per second until the
input ends or the engine hits something it doesn't implement yet. Then it
stops, shows the reason on screen and prints it; the first difference from
the trace's recorded state is printed too. Frame numbers are the trace's.

**Live play**: you are the left navi; the right one stands still. The round
is a netbattle on the pack's BN6 content between two 1000-HP MegaMen of
Falzar, set up at random from the seed (`driver::bn6_live_setup`, which
prints what it drew):

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
  form, buster, charged shot, element and face; Beast Out from it is
  Falzar's Beast.

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

**Headless mode** renders the listed frames (`a,b,c-d`; trace frame
numbers, or tick numbers in live play) to `frame_NNNNN.png`. It exits
non-zero if some frames couldn't be rendered (the engine stopped first).
With `--objects` it also lists every rendered frame's objects as the
renderer sees them: kind, screen position, sprite, animation and frame, and
look (palette, shadow, flips, white, shader, hidden parts, whether it is
drawn at all), and what its console shows of its chips. In live play
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
the pack's sound, unless `--mute`; headless rendering never plays sound.
Other per-tick consumers can plug in the same way, as a `TickHook`
(`nettai_frontend::session`), which the window runs after every tick.

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
  list mixes shows each game's own;
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

The text it draws goes through `fonts.rs` (the cell-text helper for the
8x16 font), so that a later font-rendering step can change what is behind
it (docs/design/text-rendering.md).

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
the custom screen is up are counted apart.

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
that have screenshots, **all 6397 are pixel-exact**, every row of them. Of
its 13,278 custom-screen frames 13,266 are, with the Cross windows, the
DustCross scrap and the screens' openings and closings; the 12 that aren't
show a description's chatbox.

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

**The custom screen's own scenarios** (36: the re-deal, the scrap, the
keys, the Cross window, Beast Out, invalid chips, modifiers and Program
Advances, a link navi's own chip, the chatbox): all 32,434 frames outside
the custom screen are pixel-exact, and 9,593 of its 9,977, all but the
frames that show the chatbox (descriptions, the run message), which isn't
drawn yet.

The comparison needs the ROM, so it lives outside this repository, with the
lists of scenarios. The frontend's own tests (`cargo test -p nettai-frontend`)
use a small synthetic asset set and a live battle built in code.

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

- The custom screen's chatbox (descriptions, the run message) isn't drawn
  yet; live play also shows the screen as text.
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
