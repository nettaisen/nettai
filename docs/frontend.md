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

## 1. Graphics

The graphics come from your own ROM (US Falzar, `MEGAMAN6_FXXBR6E`) and are
never checked in. Extract them once:

    cargo run -p bn6-extract -- graphics <rom> data/graphics

This writes `data/graphics/bn6-assets.bin` (about 5 MB; `data/graphics/`
is gitignored). The frontend loads it at start-up from
`--graphics <dir-or-file>`, else `$BN6_GRAPHICS`, else `data/graphics`.
(`bn6-extract assets <rom> <bank>` writes the sound bank; see bn6-audio.)

The bundle's types live in the `bn6-assets` crate. It holds, decoded
(tiles as palette indices, colours as BGR555):

- **Sprites**: every battle sprite (categories 0x00..=0x14 of
  `SpritePointersList`), per animation frame its tileset, palette set and
  part list (the frame's OAM composition: tile, offset, size, flips,
  palette offset). The first part of every frame is the shadow.
- **Field**: the panel tiles (`dword_86DDBA0`), panel palettes and their
  cycling animations (`off_800C1DC`, with the start timers `sub_800BF88`
  sets), the 5x3 panel blocks by type, owner and row (`byte_86DFA98`),
  highlight blocks and front edges.
- **Backgrounds** by id (`off_8080F98`): tiles, tile map, palette, scroll
  speed (`off_8080E34`) and tile/palette animations (`off_8081220`).
- **HUD**: the HP box and its digits, gauge tiles and frame, the 8x16 font
  and chip names, chip icons, the opponent's HP digits, mugshots and count
  boxes, banner layouts and glyphs, "Cstmzing...".

## 2. Running

    cargo run -p bn6-frontend -- <trace.jsonl>              # watch a trace
    cargo run -p bn6-frontend -- --play                     # play live
    cargo run -p bn6-frontend -- <trace.jsonl> --headless 150,300,600 --out <dir>
    cargo run -p bn6-frontend -- --play --sound <bank>      # with sound

Options: `--round N` starts a trace at round N (later rounds follow when a
round's input runs out), `--scale N` sets the window scale (default 4),
`--paused` starts paused, `--seed N` fixes a live battle's RNG seed,
`--png-scale N` scales headless output, `--quit-after N` closes the window
after N ticks.

Keys: arrows move, X = A, Z = B, A = L, S = R, Enter = START,
Backspace = SELECT; Space pauses, `.` steps one frame while paused, `-` and
`=` change speed (1/8x to 16x of 59.73 Hz), F5 restarts the round, H toggles
the status line, Esc quits.

**Trace playback** runs at the original's 59.73 frames per second until the
input ends or the engine hits something it doesn't implement yet. Then it
stops, shows the reason on screen and prints it; the first difference from
the trace's recorded state is printed too. Frame numbers are the trace's.

**Live play**: you are the left navi; the right one stands still. The round
uses a built-in netbattle setup (the recorded matches' field and a
1000-HP MegaMan per side). The custom screen is a stand-in: A takes a fixed
hand (GunDelS3 twice, Geddon twice), B the same plus Beast Out. Moving,
chips and Beast Out work; what else works depends on what the engine
implements (the buster doesn't yet). When the engine stops, F5 starts over.

**Headless mode** renders the listed frames (`a,b,c-d`; trace frame
numbers, or tick numbers in live play) to `frame_NNNNN.png`. It exits
non-zero if some frames couldn't be rendered (the engine stopped first).

**Sound**: `--sound <bank>` plays each tick's sound cues through bn6-audio.
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
- projection of 16.16 positions: screen x = x - camera x + 120 (mirrored
  for the right-hand player), ground y = y - camera y + 80, sprite y =
  ground y - (z - camera z);
- per part: flip-adjusted offsets, the original's culling, hardware
  position wrapping, the palette the object chose plus the frame's first
  part's offset, the colour shader;
- the first part is the shadow: hidden, drawn on the ground one layer back
  (`sprite_hasShadow`), or drawn with the sprite (`sprite_noShadow`);
- order: parts go into depth buckets (ground y + 0x40) and come out
  deepest first, most recent first, so later and lower objects are in
  front; at most 128 parts;
- white flash, alpha and mosaic (sampled as mGBA does).

**Field and background** (`stage.rs`): each panel's block by displayed type
and owner (from the viewer's side), highlights, missing panels, front
edges, the cycling panel palettes; the background's scroll and tile
animations.

**HUD** (`hud.rs`): the HP box with its rolling number and colours, the
custom gauge (fill, full animation), the next chip's name and damage, the
mugshot (mood, form, change blink) and count, the opponent's HP number under
the navi (rolling, coloured), the chip icons over the navis, the banners
with their squash and stretch, and "Cstmzing..." while the opponent is
still choosing. The rolling numbers are presentation state the engine
doesn't keep; `Renderer::observe` follows them tick by tick.

**Fades**: the intro fades in from white (from black after the first
battle of a set), the round's end to black, the transformation sequencer
dims the tile layers to black while navis change form (sprites keep their
colours) and the palette flash whitens them.

### What the engine gained for the frontend

Small, presentation-only additions:

- `object::sprite::Look` on every `Sprite` (palette, flips, shadow mode,
  white flash, colour shader, alpha, mosaic, priority, hidden parts),
  reset by `Sprite::load` and set where the ported behaviors call the
  game's `sprite_*` routines: effects, sparks, rock debris, rocks,
  attachments, sun beams, form overlays, lock-on markers, afterimages,
  charge glows, and the player navi (sprite load, palette, entry fade-in,
  deletion fade-out, hit flash, Beast Out emerge).
- `hud::Banner::id`, the banner showing.
- `Field::clear_highlights` at the start of each tick: highlights last one
  frame (the original's field renderer clears them after drawing).
- bn6-extract reads LZ77-compressed sprite archives correctly (they start
  with a size word), so the 41 compressed battle sprites have animation
  data in `sprites_generated.rs` too.

## 4. Verification

Headless frames were compared pixel for pixel with screenshots of the
original running under emulation, one per battle frame. On the vanilla PvP test match, over the
2404 frames the engine simulates (round 1 frames 72..=1145, round 2 frames
1224..=2554), **1649 frames are pixel-exact**, the HUD included: the intro
fades and the opponent's mosaic fade-in, round and turn banners,
movement, GunDelSol, the deletion, the win banner and the fade out, and in
round 2 Beast Out with its overlay, afterimages, lock-on and screen dim.

What differs is what isn't drawn or modelled yet:

| Frames | What |
|---|---|
| 208-384, 1360-1636 | the custom screen itself |
| 1637-1775 | the mugshot previews the Beast Out chosen on the custom screen |
| 939-960, 2348-2368 | a few pixels of the deletion's light rays (rotated sprites) |
| 1802-1861 | the camera shake during Beast Out (the engine has no camera) |
| 1866 | one frame of the beast head's white flash |
| 1970-2004 | the lock-on marker's edge against the navi |
| 2373-2402 | a chip icon the original hides during the beast attack |

A second match, traced on the right-hand player's console (so the field is
drawn mirrored), was compared over round 1's first 2952 frames (up to where
the engine stops at a Cross change). There the field, the ice panels and
rock cubes, the navis, the HP numbers and the gauge are exact; every frame
still differs in two ways: the mugshot previews the Cross chosen on the
custom screen, and the bottom 7 rows show the background scrolled one
frame ahead on frames where its scroll steps. The latter is a timing
effect of the original: on that console its main loop updates the scroll
register while the last rows are being drawn.

The comparison needs the ROM and a recorded match, so it lives outside this
repository. The frontend's own tests (`cargo test -p bn6-frontend`) use a
small synthetic asset set and a live battle built in code.

## 5. Known gaps

- The custom screen (chip selection UI); live play uses a fixed hand.
- The camera (shake) isn't modelled by the engine.
- Affine (rotated or scaled) object sprites, the per-part palette override
  of `sub_3006440`, and the original's sprite block bits 0x20/0x40.
- Navi palettes: Cross forms (`byte_80203EA`), Beast Over and navis other
  than MegaMan draw with the palette they last had.
- Dynamic text banners (kinds 3 and 4), the chip name's "x2" suffix and the
  full gauge's animation phase are unverified or not drawn.
- The form overlay's white flash follows its owner's (measured; the
  overlay object itself doesn't run while the battle is paused).
- The background scroll starts one frame earlier in the first round of a
  set than in later ones (measured).
