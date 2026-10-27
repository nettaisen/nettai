# The command line (`nettai-tools`)

`nettai-tool` is nettai's command line with no window, for the developer and the verification: a battle's chosen
frames written to PNGs (a golden trace's, a match file's, a replay's), a replay written as video, the content and
trace audits, a match's setup said, and a replay checked. It is a host of nettai-frontend (docs/frontend.md §7) as nettai is; a battle is played in
a window by nettai (docs/app.md).

    cargo run -p nettai-tools -- <trace.jsonl> --headless 150,300,600 --out <dir>
    cargo run -p nettai-tools -- --match match.toml --headless 1-120 --out <dir> --keys 100:a
    cargo run -p nettai-tools -- --replay set.nettaireplay --headless 1-600 --side right --out <dir>
    cargo run -p nettai-tools -- --match match.toml --show-folders            # the setup, said
    cargo run -p nettai-tools -- --match match.toml --save-match played.toml  # keep the seed played
    cargo run -p nettai-tools -- --replay set.nettaireplay                            # does it reproduce?
    cargo run -p nettai-tools -- --replay set.nettaireplay --mp4 set.mp4              # the replay as video
    cargo run -p nettai-tools -- --replay set.nettaireplay --mp4 round2.mp4 --round 2 --scale 4
    cargo run -p nettai-tools -- --match match.toml --audit-content           # what is missing?
    cargo run -p nettai-tools -- --audit <trace.jsonl>...                     # and in these traces?
    cargo run -p nettai-content -- check exe6                                 # the content, and its assets

The battles are of one game, EXE6, EXE5 or EXE4: a match file names its game, a replay and a trace state their own.
The game is played on its content folder and the support folders it uses, drawn and heard from its assets,
extracted from its ROMs by its recipe as it loads, in memory (docs/frontend.md §1). The ROMs are `$NETTAI_ROMS`'s, a
folder that is the Legacy Collection's install or holds its ROM files (any names: `~/Documents/Tango/roms` serves
as it is), else the collection where Steam installed it (`nettai_extract::RomSource::from_env`); none found is an
error that says how to name them. `cargo run -p nettai-tools -- --help` lists every option. (Its options are the
retired nettai-demo's for the same, so the verification's scripts took it as they were.)

## Options

- `--content <dir>` names the battle content (default `$NETTAI_CONTENT`, else a `content/` beside the program,
  else this repository's `content/`); the game's recipe is read from it too.
- `--mute` turns the audits' sound off (headless rendering never plays any), and leaves a video without sound.
- `--round N` starts a trace at round N (later rounds follow when a round's input runs out); of a replay's video
  it is the round shown alone.
- `--png-scale N` scales headless output (the text layer is drawn at that scale too).
- `--text enhanced|original` chooses how the battle is drawn (default `enhanced`, also spelled `font`: the
  names, the telop, the chatbox and the HUD's lines in a vector font over the scaled frame, the background's
  glide, the gauge's smooth fill, the banners' squash and the custom screen's slide of the field at the output's resolution, docs/frontend.md §3, "Enhanced and original"; `original` as the original does, the
  text in the game's own fonts into the frame, which the frame comparison uses), and `--font <file>` puts
  another TrueType or OpenType font in the bundled one's place.
- `--lang en|ja` the language of the battle's words (default `en`; docs/frontend.md §3, "Languages").
- `--match FILE` is a match file (docs/frontend.md §6): its setup said on the terminal (the folders with
  `--show-folders`), its frames rendered with `--headless`, and with `--audit-content` the game whose content is
  audited. Its seed sets the battle's RNG (else the clock's). `--save-match FILE` writes the match with its seed,
  to play again; `--record FILE` writes the set played headless as a replay (docs/frontend.md §8).
- `--replay FILE` plays a replay to its end and checks it against the digests it kept: what happened, and
  whether it reproduces (it exits 1 where it doesn't); with `--headless F`, its ticks F are rendered, from the
  console `--side left|right` (default: the side that recorded it), or both players on one screen with
  `--side both` (the side that recorded it on the left; `both-left`, `both-right`: that side's console on the
  left): the field as the left console draws it, each player's own pieces on its own side, the right player's
  mirrored into the right half (docs/frontend.md §3, "Both on one screen"); with `--mp4 OUT` it is written as
  video ("A replay as video", below).

A trace is rendered or audited: it is no longer watched in a window.

## Headless frames

`--headless` renders the listed frames (`a,b,c-d`; trace frame numbers, or tick numbers in live play and in a
replay) to `frame_NNNNN.png` in `--out` (default `.`). It exits non-zero if some frames couldn't be rendered (the
engine stopped first). A trace's rounds play at the original's pace, its recorded inputs driving the battle, each
tick compared with what the original recorded; the first difference is printed.

- `--objects` lists every rendered frame's objects as the renderer sees them: kind, screen position, sprite,
  animation and frame, and look (palette, shadow, flips, white, shader, hidden parts, whether it is drawn at all),
  and what its console shows of its chips; and its text items (the words, the role, the box, the depth key, how
  many of the box's pixels something in front covers, the squash, the fades).
- `--mark M,...` writes where each of these is drawn, by frame, to `marks.tsv` beside the frames: `sprite:NAME`
  (an object drawn with it), `background:NAME`, `chip:KEY` (its picture in the chip window), `chip-icon:KEY` (its
  icon: over its navi, in the custom screen's slots and its picked chips' column), `chip-window-at-close` (the
  HUD's chip name before the navi's first decision); for a frame comparison that knows what a console shows
  otherwise there.
- `--keys` gives your buttons by tick in a match file's set (`headless::KeyScript`): for instance
  `--keys 160-161:up,215:a,260:start,266:a` opens the first screen's Cross window (a direction acts on a hold's
  second tick), chooses its first Cross and presses OK. The buttons are `a b l r up down left right start
  select`; the right navi is the stand-in, standing still and picking its first chip. `--opponent moves` has it
  move about at random and `--opponent attacks` press at random what a player fights with
  (`driver::InFight::MovesAround`, `AttacksAtRandom`; `still` is the default); on its custom screen it still picks
  its first chip.

## A replay as video

`--replay FILE --mp4 OUT` writes the replay as an MP4, as the app's export makes one (nettai-video,
docs/frontend.md §8, "Video"): one frame a tick, the picture a viewer playing the replay straight is shown (the
text included) and the sound the tick made, made 48 kHz; H.264 at CRF 16 in 4:2:0 and AAC, encoded by `ffmpeg`
(`$NETTAI_FFMPEG`, else one beside nettai-tool, else one on the path: nettai-video's `ffmpeg::find_ffmpeg`), which
is given the frames and samples the browser's encoder would be; the pixels aren't scaled by ffmpeg, nor the sound
resampled. Each round is a chapter ("Round N") and each sound track is titled with its player's name. `OUT` is
there only once it is whole; a failure says whether ffmpeg wasn't found, ffmpeg failed (with what it said) or the
file couldn't be written. Everything the tool says goes to standard error (a line every tenth of the way, then
what it wrote). (`--nut`, the raw stream ffmpeg is given, is gone: the user, "can you remove nut support then?")

- `--scale N` makes the picture N times the frame's 240x160 (1 to 16; default 1). The scaling is the nearest
  pixel's, and the enhanced mode's text is drawn at that size, as `--png-scale` does for a PNG. For an MP4, an even N
  keeps every pixel's edge clean in 4:2:0; an odd one leaves a fringe of mixed color at each (docs/frontend.md §8).
- `--side left|right` is the console shown (default: the side that recorded the replay); `--side both` (or
  `both-left`, `both-right`) shows both players on one screen, as `--headless` draws it, its sound one track: the
  left console's with the right side's own effects over it (nettai-video's `Sides::Both`, docs/frontend.md §8).
- `--both-sides` shows both consoles side by side in a picture twice as wide (480·N by 160·N): `--side`'s at the
  left, the other's at the right, each byte for byte its own video's, from one playing of the replay; each
  console's sound is a track of its own, the left one's first and the default (nettai-video's `Sides::SideBySide`,
  docs/frontend.md §8).
- `--round N` shows that round alone, and `--ticks A-B` the frames A to B alone, numbered as `--headless` numbers
  them (the pictures after the set's ticks A to B: `--ticks 600-719` is the 120 frames `--headless 600-719` writes
  as PNGs, and is byte for byte those pictures). A part starts as playing straight to it leaves the battle, its
  sound in mid-note included.
- `--mute` leaves the sound out (an MP4 of the picture alone); `--text` and `--lang` are the picture's, as for
  headless frames.

Either way each round is a chapter, "Round 1", "Round 2"…, from the frame its picture begins on, and each sound
track is titled with its player's name as the replay has it (else "Player 1" or "Player 2"): in English here;
the app's export says them in its language.

The same replay and options give the same bytes. It exits 1 if the replay doesn't reproduce, as `--replay FILE`
alone does.

## The audits

The audits list what the drawing code and the audio look up that the game's assets or the content don't have: a
sprite that isn't in the graphics, an animation or palette the sprite doesn't have, a chip without an icon or a picture or
with a name the font can't spell, a face, an emblem, a banner without glyphs, a telop's banner that is no telop's,
a text line, a song. Each exits 1 if there was any; drawing itself skips what it can't find, so nothing else
notices. Every such lookup goes through one module (nettai-render's `lookups.rs`; the audio's, a cue's song,
nettai-tools' `sound_lookups.rs`), which notes it (`audit::Lookup`) and checks it once a run, so both audits make
the lookups a frame makes, through the same functions:

- `--audit-content` (`content_audit.rs`) makes every lookup for everything the content defines, in every language
  it has strings in: every chip's icon, picture, name, its window's class, element and code pictures, its
  description in the dialogue font; every navi's face, emblem, name and no-running message with its portrait (a
  navi a side can start that has no message, or none to say it, is a problem where the game's roles fill the
  message's sound: the custom screen opens no box for it, so L would do nothing); every form's face for each
  emotion, every Cross's name and description; every custom-screen button's look (a button the graphics have none
  for is drawn as nothing, unless it shows a chip), on a console of each of the graphics' versions; and every
  asset of the game (each sprite with every animation, its frames and their own palettes; each song, banner,
  background, mugshot), the HUD's text lines, the custom screen, the chatbox. It checks the field too: the game's
  field must draw the panel types its game names; then every panel type the game names and both highlights are
  drawn as the stage draws them, and a tinted one is said as a note. It takes a
  second or two, and a lookup by the wrong key fails it for every chip, not only for those a trace shows (its
  test: `a_lookup_by_the_wrong_key_fails_for_every_chip`). A string a language's table lacks shows in the
  content's own, by design: it is said, not counted. A language the content has strings in and the graphics no
  lettering for is said as a note: they are drawn in the graphics' own lettering, and the strings in the
  frontend's font (Chinese). It audits the match file's one game (`--match FILE`, which may say nothing but `game =
  "exe6"`); the match's sides aren't played or validated. What the assets hold of the content (every asset it
  names, each sprite's frames, the sprites' timing against the ROMs', the strings' backgrounds) is `nettai-content
  check`'s ("The content check", below).
- `--audit <trace.jsonl>...` runs traces, several at a time (`--jobs N`, default one a core), and makes the
  lookups their frames and sound cues make, without drawing: no stage, no composing, no sound synthesis
  (`Renderer::set_lookups_only`). It catches what the content can't say beforehand: the palette an object picks,
  a telop of a chip the engine wasn't told. Each problem is listed with its trace and the frame it was first seen
  on. `--draw` draws every frame and plays every cue into nothing besides (some ten times slower). `--lookups
  FILE` writes each trace's distinct lookups, by name (and with `--audit-content`, the static audit's), for the
  verification's trace cover: the few golden traces that, with the static audit, make every lookup all of them
  make.

`NETTAI_LOAD_TIMES` prints how long each part of the game took to load.

## The content check

`nettai-content check <game> [--content DIR] [--roms DIR]` (the nettai-content crate's command) defines the
game's content and checks its definitions (the lints), its strings, its library order and its look; then it
extracts the game's assets from its ROMs by its recipe (`--roms DIR`: the Legacy Collection's folder or one of ROM
files; default `$NETTAI_ROMS`, else the collection where Steam installed it) and holds them to the content: every
asset the content names is there and each sprite it defines has as many animations, each of as many frames
(`nettai_content::load::asset_numbers`: else the game doesn't load), the content times its sprites as the ROMs time
them (`Extraction::timing_differences`: each frame's duration, the loop on the last frame, the flags and the part
offsets a sprite states; each difference an error), and the strings' backgrounds are the assets'. ROMs that can't
be found or extracted are an error. What the extraction noticed is said as notes. It exits 1 if there was an
error. (Its `publish` and `lock` commands publish a game's content: docs/design/content-deployment.md.)

## The extractor, from a program

nettai-extract is a library, with no command of its own (crates/nettai-extract/README.md): nettai uses it to
extract each game's assets from the player's Legacy Collection as the game loads (docs/app.md §17), nettai-tools
through nettai-frontend, and nettai-content's check directly. It reads ROMs from wherever a program names them
(`RomSource`: the Legacy Collection installed, a folder of ROM files, or the copies a browser keeps), into memory,
and writes nothing. What a program needs besides `extract(content, game, &roms)`:

- `header_code(header)`: the game code a ROM is of, from its first `HEADER_LEN` (176) bytes alone, so a program can
  match the ROMs it finds against the codes each game's recipe declares (`Recipe::roms`) before it reads one whole.
  `RomSource::find` does so.
- `Recipe::of_content`, `RomSource::read` and `Recipe::extract`, the steps of `extract`; `Recipe::extract_reporting`,
  the same telling a callback how far it is (0 to 1), for a bar (`an_extraction_reports_how_far_it_is`); and
  `Recipe::extract_recording_reads`, every range of each ROM the extraction read (the verification's read map).
- `Extraction::timing_differences`, the content's sprite timing against the ROMs' (the content check's).
- `songs_played_otherwise(a, b)`: the songs two releases' banks play otherwise (the verification's cross-ROM sweep).
