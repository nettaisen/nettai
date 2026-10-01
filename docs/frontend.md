# The rendering frontend (`bn6-frontend`)

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

    cargo run -p bn6-extract -- content <rom> data/content/bn6

(`data/content/` is gitignored.) The frontend loads the pack at start-up
from `--pack <dir>`, else `$BN6_PACK`, else `data/content/bn6`, straight
from its files: the content with the pack's asset index into the engine's
`Content`, the graphics through bn6-content's importer, and, when a window
opens, the sound. `BN6_LOAD_TIMES=1` prints how long each part took.

The graphics load into the types of the `bn6-assets` crate, decoded
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

What the HUD shows of the content comes from the content: a chip's name is
its definition's, spelled with the font's glyphs (`Hud::glyphs`); its icon
is the pack's image under the chip's key; whether its damage shows is its
definition's flag. The emotion window shows the face the navi's form names
for its emotion (`mugshot`, `FormData::mugshot`), or a link navi's own
(`NaviData::mugshot`); mugshot numbers from `bn6_assets::NAVI_MUGSHOTS`
are the link navis' faces, with their Full Synchro palettes.

## 2. Running

    cargo run -p bn6-frontend -- <trace.jsonl>              # watch a trace
    cargo run -p bn6-frontend -- --play                     # play live
    cargo run -p bn6-frontend -- <trace.jsonl> --headless 150,300,600 --out <dir>
    cargo run -p bn6-frontend -- <trace.jsonl> --audit      # what is missing?
    cargo run -p bn6-frontend -- --play --pack <dir>        # another pack

Options: `--pack <dir>` names the content pack and `--content <dir>` the
battle content (see above), `--mute` turns the sound off, `--round N`
starts a trace at round N (later rounds follow when a round's input runs
out), `--scale N` sets the window scale (default 4), `--paused` starts
paused, `--seed N` fixes a live battle's RNG seed, `--png-scale N` scales
headless output, `--quit-after N` closes the window after N ticks.

Keys: arrows move, Z = A, X = B, A = L, S = R, Enter = START,
Backspace = SELECT; Space pauses, `.` steps one frame while paused, `-` and
`=` change speed (1/8x to 16x of 59.73 Hz), F5 restarts the round, H toggles
the status line, Esc quits.

**Trace playback** runs at the original's 59.73 frames per second until the
input ends or the engine hits something it doesn't implement yet. Then it
stops, shows the reason on screen and prints it; the first difference from
the trace's recorded state is printed too. Frame numbers are the trace's.

**Live play**: you are the left navi; the right one stands still. The round
is the recorded matches' netbattle on the pack's BN6 content
(`driver::bn6_live_setup`: their field and a
1000-HP MegaMan per side, each with a folder of GunDelSols, Geddon,
Invisibl and EraseMan, shuffled from the seed). The custom screen is the
engine's (docs/engine/custom-screen.md), shown as text for now: the dealt
chips in the grid's order (`>` the cursor, `+` picked, `-` greyed), OK and
Beast Out, the picks and the Cross window. The keys are the game's (A
picks, B takes back, START goes to OK, UP from the top row opens the Cross
window, R describes, SELECT hides). The right navi's screen picks its first
chip and presses OK. As in the original's netbattles, the fight gets your
buttons 4 ticks late (the link). F5 starts over.

**Headless mode** renders the listed frames (`a,b,c-d`; trace frame
numbers, or tick numbers in live play) to `frame_NNNNN.png`. It exits
non-zero if some frames couldn't be rendered (the engine stopped first).
With `--objects` it also lists every rendered frame's objects as the
renderer sees them: kind, screen position, sprite, animation and frame, and
look (palette, shadow, flips, white, shader, hidden parts, whether it is
drawn at all), and what its console shows of its chips.

**Audit mode** (`--audit`) draws every frame of the trace and plays every
sound cue into nothing, without a window, and lists what they named that
the pack or the content doesn't have, each with the frames it happened on:
a sprite that isn't in the pack, an animation or palette the sprite doesn't
have, a chip without an icon or with a name the font can't spell, a banner
without glyphs, a text line, a song. It exits 1 if there was any. It is the
quick check after a content or loader change: nothing is silently skipped.

**Sound**: the window plays each tick's sound cues through bn6-audio, with
the pack's sound, unless `--mute`; headless rendering never plays sound.
Other per-tick consumers can plug in the same way, as a `TickHook`
(`bn6_frontend::session`), which the window runs after every tick.

## 3. What is drawn, and how

Layers, back to front: the backdrop colour, the background (priority 3),
the field (priority 2), sprites of priority 2, the HUD layer (priority 1),
sprites of priority 0 (banners).

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
layer to black while navis change form (sprites keep their colours) and
the palette flash whitens them.

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
the custom screen is up, which the frontend doesn't draw, are counted
apart.

**The vanilla PvP test match** (round 1 frames 72..=1145, round 2 frames
1224..=2554): of the 2404 frames the engine simulates, **1949 are
pixel-exact**, the HUD included: the intro fades and the opponent's mosaic
fade-in, round and turn banners, movement, GunDelSol with its name and
icons, the deletion, the win banner and the fade out, and in round 2 Beast
Out with its overlay, afterimages, lock-on, camera shake and screen dim.
What differs:

| Frames | What |
|---|---|
| 208-384, 1360-1636 | the custom screen itself |
| 1637 | the mugshot shows the Beast Out chosen on the custom screen a frame before the replay knows it |

**A second match**, three rounds traced on the right-hand player's console
(so the field is drawn mirrored), with Crosses, rock cubes, ice and grass
panels, traps and Invisibl: of the 6397 frames outside the custom screen
that have screenshots, **6395 are pixel-exact** (rows 152-159 left
out, below). What differs: on two frames (7498, 25885) the whole
background is the next frame's, which is how the screenshots were taken
(below), not the game.

**Chip-lab scenarios**: 132 scenarios, a few of every family
(shot, sword, thrown, placed and dimming chips, navi chips, the link
navis, traps, supports, stages with their objects, forms, Beast Over, the
flow: knockouts, the damage judge, pause, a counter hit, a lost Full
Synchro), each recorded with a screenshot per battle frame. Of 123,433
frames outside the custom screen, **all 123,433 are pixel-exact**, and so
is every scenario on every frame: the telops of dimmings content starts
itself, LilBoiler's HP number, the warning arrows, the faces. No frame
panics, and the audit names nothing missing.

The comparison needs the ROM, so it lives outside this repository, with the
list of scenarios. The frontend's own tests (`cargo test -p bn6-frontend`)
use a small synthetic asset set and a live battle built in code.

Screenshots of the right-hand player's console that the recorders took as
the emulated pair's tick ended (the golden traces', and the chip lab's
before its pictures were taken at the console's own VBlank) have the
previous picture in their last rows: the tick ends at the first console's
VBlank, when the second console's video is still some seven scanlines from
the picture's end (152-159). The comparison leaves rows 152-159 out for
those; the chip lab's pictures taken at the VBlank match on every row. The
golden match's two whole-frame differences are most likely the same timing
drifting by a frame (its screenshots are still taken at the tick's end).

## 5. Known gaps

- The custom screen (chip selection UI) isn't drawn, and its own sounds
  aren't cues: live play shows it as text.
- Affine (rotated or scaled) object sprites (`sprite_makeScalable`: no kind
  in the engine or the content uses one yet), the per-part palette override
  of `sub_3006440`, and the original's sprite block bits 0x20/0x40.
- The HUD's other text lines: the multiple deletions of virus battles,
  "SHUFFLE!" and "PENALTY!!" (the custom screen's).
- Not drawn, as only viruses' code or no netbattle reaches them: an HP
  number at a fixed place (`sub_801DCFC`) or moved (`sub_801DCCC`), the
  training viruses' (NameIDs 0x49..=0x4E: moved 32 pixels left), and a
  warning marker within 16 pixels left of or above the screen, for which
  the original writes a garbled sprite.
- During the palette flash (`sub_80E10C0`) the original keeps the HUD
  layer's colours (the HP box and the gauge), where the frontend whitens
  every tile layer (seen in a longer recording of DeltaRay's hit, not yet
  in the sample).
- The background scroll starts one frame earlier in the first round of a
  set than in later ones (measured).
