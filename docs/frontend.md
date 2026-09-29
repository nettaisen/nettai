# The rendering frontend (`bn6-frontend`)

A desktop app that runs a battle through the native engine and draws it the
way the original does, in its 240x160 frame scaled up by an integer factor.
It draws from engine state: the field's panels, each object's sprite,
animation frame and look, HP, the custom gauge and the banners. Nothing
emulates the GBA's video hardware; the frontend composes tile layers and
sprite parts itself with the original's ordering and blending rules.

It can replay a golden trace (the recorded inputs of a real match) or be
played live from the keyboard.

## 1. Assets

Graphics come from your own ROM (US Falzar, `MEGAMAN6_FXXBR6E`) and are
never checked in. Extract them once:

    cargo run -p bn6-extract -- assets <rom> assets

This writes `assets/bn6-assets.bin` (about 4 MB; `/assets` is gitignored).
The frontend loads it at start-up from `--assets <dir-or-file>`, else
`$BN6_ASSETS`, else `./assets`. The bundle's types live in the `bn6-assets`
crate; it holds, decoded (tiles as palette indices, palettes as BGR555):

- **Sprites**: every battle sprite the engine can load (categories
  0x00..=0x14 of `SpritePointersList`), per animation frame its tileset,
  palette set and part list (the frame's OAM composition: tile, offset,
  size, flips, palette offset). The first part of every frame is the
  shadow.
- **Field**: the panel tiles (`dword_86DDBA0`), panel palettes and their
  cycling animations (`off_800C1DC`, with the start timers
  `sub_800BF88` sets), the 5x3 panel blocks by type/owner/row
  (`byte_86DFA98`), highlight blocks and front edges.
- **Backgrounds** by id (`off_8080F98`): tiles, tile map, palette, scroll
  speed (`off_8080E34`) and tile/palette animations (`off_8081220`).
- **HUD**: HP box and digits, gauge tiles and frame, the 8x16 font and chip
  names, chip icons, the opponent's HP digits, mugshots and count boxes,
  banner layouts and glyphs.

## 2. Running

    cargo run -p bn6-frontend -- <trace.jsonl>            # watch a trace
    cargo run -p bn6-frontend -- --play                   # play live
    cargo run -p bn6-frontend -- <trace.jsonl> --headless 150,300,600 --out <dir>

Options: `--round N` starts a trace at round N (later rounds follow when a
round's input runs out), `--scale N` sets the window scale (default 4),
`--paused` starts paused, `--seed N` fixes a live battle's RNG seed,
`--png-scale N` scales headless output.

Keys: arrows move, X = A, Z = B, A = L, S = R, Enter = START,
Backspace = SELECT; Space pauses, `.` steps one frame while paused, `-` and
`=` change speed (1/8x to 16x of 59.73 Hz), F5 restarts the round, H toggles
the status line, Esc quits.

Trace playback runs at the original's 59.73 frames per second until the
input ends or the engine hits something it doesn't implement yet. Then it
stops, shows the reason on screen and prints it; the first difference from
the trace's recorded state is printed too. The frame numbers are the
trace's.

**Live play**: you are the left navi; the right one stands still. The round
uses a built-in netbattle setup (the recorded matches' field and a
1000-HP MegaMan per side). The custom screen is a stand-in: press A to
close it (no chips are chosen yet). How far a live battle gets depends on
what the engine implements; when it stops, F5 starts over.

**Headless mode** renders the listed frames (`a,b,c-d`; trace frame numbers,
or tick numbers in live play) to `frame_NNNNN.png` files. It exits non-zero
if some frames couldn't be rendered (the engine stopped first).

**Audio** plugs in as a `TickHook` (`bn6_frontend::session`): a hook added
to `Session::hooks` sees the battle after every tick, which is where an
audio output hands the tick's sound cues over and advances.

## 3. What is drawn, and how

Layers, back to front: the backdrop colour, the background (priority 3),
the field (priority 2), sprites of priority 2 (objects), the HUD layer
(priority 1), sprites of priority 0 (banners).

**Objects** (`objects.rs`) follow the original's render passes
(`sub_8003E18`/`sub_8004218`/`sub_8004510` and `sub_30061E8`,
`sub_3006440`, `sub_3006920`):

- pools in order actors, attacks, effects; objects of a pool in update
  order; only objects flagged visible whose sprite animates;
- projection of 16.16 positions: screen x = x - camera x + 120 (mirrored
  for the right-hand player), ground y = y - camera y + 80, sprite y =
  ground y - (z - camera z);
- per part: flip-adjusted offsets, the original's culling, hardware
  position wrapping, the palette chosen by the object plus the frame's
  first part;
- the first part is the shadow: hidden, drawn on the ground one layer back
  (`sprite_hasShadow`), or drawn with the sprite (`sprite_noShadow`);
- order: parts go into depth buckets (ground y + 0x40) and come out
  deepest first, most recent first, so later and lower objects are in
  front; at most 128 parts;
- white flash, alpha and mosaic (mGBA's sampling) as the object asks.

**Field** (`stage.rs`): each panel's block by displayed type and owner
(from the viewer's side), highlights, missing panels, front edges, and the
cycling panel palettes. **Background**: scroll and tile animations by frame.

**HUD** (`hud.rs`): the HP box with its rolling number and colours, the
custom gauge (fill, full animation), the next chip's name and damage, the
mugshot and count, the opponent's HP number under the navi (rolling,
coloured), the chip icons over the navis, and the banners with their
squash-and-stretch. The rolling numbers are presentation state the engine
doesn't keep; `Renderer::observe` follows them tick by tick.

**Screen fades**: the intro fades in from white (from black after the first
battle of a set), one frame behind the engine's fade counter as on the
original.

### Engine state the frontend reads that the engine didn't have

Small presentation-only additions to the engine:

- `object::sprite::Look` on every `Sprite` (palette, flips, shadow mode,
  white flash, colour shader, alpha, mosaic, priority, hidden parts), reset
  by `Sprite::load` and set where the ported behaviors call the game's
  `sprite_*` routines: effects, sparks, rock debris, rocks, and the player
  navi (sprite load, entry fade-in, deletion fade-out, hit flash).
- `hud::Banner::id`, the banner showing.
- `Field::clear_highlights`, called at the start of each tick: highlights
  last one frame (the original's field renderer clears them after drawing).

## 4. Verification

Headless frames were compared pixel for pixel with the original running in
mGBA (the oracle's `--shots` option in the separate verification workspace
captures the screen showing each battle frame). On the vanilla PvP test
match, over the 1103 frames the engine currently simulates (round 1 frames
72..=622, round 2 frames 1224..=1775):

- **433 frames are pixel-exact**, the HUD included: the intro fade and the
  opponent's mosaic fade-in, the round banner, the fight with its gauge,
  HP numbers, chip icons and chip name, e.g. frames 72-207, 150, 500, 600,
  620 and 1224-1359.
- The rest differ only by what isn't drawn yet: the custom screen itself
  (frames 208-384 and 1360-1636), the "Cstmzing..." message shown while
  the opponent is still choosing (386-417, 450-481, 514-526 and part of
  1638-1773), and in round 2 the mugshot's emotion (1637-1775: the navi's
  anger isn't modelled).

The comparison needs the ROM and a recorded match, so it lives outside this
repository. The frontend's own tests (`cargo test -p bn6-frontend`) use a
small synthetic asset set and a live battle built in code.

## 5. Known gaps

- The custom screen (chip selection UI) and the "Cstmzing..." message; live
  play closes the custom screen with no chips.
- Mugshot emotions beyond normal, Full Synchro, worn out and the forms'
  (anger, the emotion-change blink); the navi palette choice
  (`sub_80100EC`: Full Synchro, element and cross palettes, other navis) is
  not modelled, so navis draw with palette 0.
- Camera shake (not modelled by the engine); colour shaders (the field
  exists, nothing sets it and the frontend ignores it); affine sprites
  other than the banners; the per-part palette override of `sub_3006440`;
  sprite flag bits 0x20/0x40 of the original's sprite block.
- The full-gauge animation's phase and the round-end fade to black are from
  the code, not verified; dynamic text banners (kinds 3 and 4) aren't drawn.
- The chip name's "x2" suffix and the chip icon position for navis other
  than MegaMan are unverified.
- Background scroll timing: the first round of a set starts one frame
  earlier than later rounds (measured; the cause wasn't traced).
