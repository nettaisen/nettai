# The command line (`nettai-tools`)

`nettai-tool` is nettai's command line with no window, for the developer and the verification: a battle's chosen
frames written to PNGs (a golden trace's, a match file's, a replay's), the content and trace audits, a match's setup
said, and a replay checked. It is a host of nettai-frontend (docs/frontend.md §7) as nettai is; a battle is played in
a window by nettai (docs/app.md).

    cargo run -p nettai-tools -- <trace.jsonl> --headless 150,300,600 --out <dir>
    cargo run -p nettai-tools -- --match match.toml --headless 1-120 --out <dir> --keys 100:a
    cargo run -p nettai-tools -- --replay set.ntrp --headless 1-600 --side right --out <dir>
    cargo run -p nettai-tools -- --match match.toml --show-folders            # the setup, said
    cargo run -p nettai-tools -- --match match.toml --save-match played.toml  # keep the seed played
    cargo run -p nettai-tools -- --replay set.ntrp                            # does it reproduce?
    cargo run -p nettai-tools -- --match match.toml --audit-content           # what is missing?
    cargo run -p nettai-tools -- --audit <trace.jsonl>...                     # and in these traces?

The battles are of one game, EXE6 or EXE5: a match file names its game, a replay and a trace state their own. The
game is played on its content folder and the support folders it uses, drawn and heard from its pack (written by
`nettai-extract <exe5|exe6> <pack-dir> [ROM ...]`), found in the packs directory, `$NETTAI_PACKS`, else `data`, each
pack by the game it says. `cargo run -p nettai-tools -- --help` lists every option. (Its options are the retired
nettai-demo's for the same, so the verification's scripts took it as they were.)

## Options

- `--pack <dir>` names a content pack elsewhere and `--content <dir>` the battle content (default
  `$NETTAI_CONTENT`, else this repository's `content/`).
- `--mute` turns the audits' sound off (headless rendering never plays any).
- `--round N` starts a trace at round N (later rounds follow when a round's input runs out).
- `--png-scale N` scales headless output (the text layer is drawn at that scale too).
- `--text font|original` chooses how strings are drawn (default `font`: the names, the telop, the chatbox and
  the HUD's lines in a vector font over the scaled frame; `original` in the game's own fonts into the frame, as
  the original does, which the frame comparison uses), and `--font <file>` puts another TrueType or OpenType
  font in the bundled one's place.
- `--lang en|ja` the language of the battle's words (default `en`; docs/frontend.md §3, "Languages").
- `--match FILE` is a match file (docs/frontend.md §6): its setup said on the terminal (the folders with
  `--show-folders`), its frames rendered with `--headless`, and with `--audit-content` the game whose content is
  audited. Its seed sets the battle's RNG (else the clock's). `--save-match FILE` writes the match with its seed,
  to play again; `--record FILE` writes the set played headless as a replay (docs/frontend.md §8).
- `--replay FILE` plays a replay to its end and checks it against the digests it kept: what happened, and
  whether it reproduces (it exits 1 where it doesn't); with `--headless F`, its ticks F are rendered, from the
  console `--side left|right` (default: the side that recorded it).

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
  (an object drawn with it), `background:NAME`, `chip:KEY` (its picture in the chip window),
  `chip-window-at-close` (the HUD's chip name before the navi's first decision); for a frame comparison that
  knows what a console shows otherwise there.
- `--keys` gives your buttons by tick in a match file's set (`headless::KeyScript`): for instance
  `--keys 160-161:up,215:a,260:start,266:a` opens the first screen's Cross window (a direction acts on a hold's
  second tick), chooses its first Cross and presses OK. The buttons are `a b l r up down left right start
  select`; the right navi is the stand-in, standing still and picking its first chip.

## The audits

The audits list what the drawing code and the audio look up that the packs or the content don't have: a sprite
that isn't in its pack, an animation or palette the sprite doesn't have, a chip without an icon or a picture or
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
  emotion, every Cross's name and description; every custom-screen button's look (a button the pack has none for
  is drawn as nothing, unless it shows a chip), on a console of each of the pack's versions; and every asset of
  the loaded packs (each sprite with every animation, its frames and their own palettes; each song, banner,
  background, mugshot), the HUD's text lines, the custom screen, the chatbox. It checks the field too: each loaded
  game's pack must draw the panel types its game names; then, in an arena of each game, every panel type a loaded
  game names and both highlights are drawn as the stage draws them, and a tinted one is said as a note. It takes a
  second or two, and a lookup by the wrong key fails it for every chip, not only for those a trace shows (its
  test: `a_lookup_by_the_wrong_key_fails_for_every_chip`). A string a language's table lacks shows in the
  content's own, by design: it is said, not counted. A language the content has strings in and its pack no
  lettering for is a problem (a console in it can't be shown): both games' packs have their Japanese. It audits
  the match file's one game (`--match FILE`, which may say nothing but `game = "exe6"`); the match's sides aren't
  played or validated.
- `--audit <trace.jsonl>...` runs traces, several at a time (`--jobs N`, default one a core), and makes the
  lookups their frames and sound cues make, without drawing: no stage, no composing, no sound synthesis
  (`Renderer::set_lookups_only`). It catches what the content can't say beforehand: the palette an object picks,
  a telop of a chip the engine wasn't told. Each problem is listed with its trace and the frame it was first seen
  on. `--draw` draws every frame and plays every cue into nothing besides (some ten times slower). `--lookups
  FILE` writes each trace's distinct lookups, by name (and with `--audit-content`, the static audit's), for the
  verification's trace cover: the few golden traces that, with the static audit, make every lookup all of them
  make.

`NETTAI_LOAD_TIMES` prints how long each part of the game took to load.
