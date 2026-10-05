# nettai

A Mega Man Battle Network battle engine in Rust. Its first game is Mega Man Battle Network 6 (US Falzar), whose
battles are reimplemented frame for frame from the original, routine by routine from the disassembly. Other
Battle Network games are meant to follow on the same core ([multi-game.md](docs/design/multi-game.md) surveys
what that takes).

The engine is a content-independent core with EXE6's ruleset on it. The game's content (chips, navis and their
forms, weapons, stages, the rule tables) is Luau in [`content/`](content), committed here and named by
keys rather than the original's numbers, in packs: the game packs content/exe6 and content/exe5, each with a manifest
listing its definitions, and the support pack content/exelib that both use. The graphics and sound are not in this
repository: you extract them
from a ROM you own into a content pack.

A battle is deterministic and cloneable, and the players' inputs are its only input, which is what rollback
netplay needs.

## Crates

- `nettai-battle`: the engine: the simulation core, EXE6's ruleset, and the content model it runs on.
- `nettai-content-api`: the contract between the core and content: the API scripts call, definitions, handles.
- `nettai-luau`: the Luau runtime for content (sandboxed scripts that keep no state of their own).
- `nettai-content`: loads the content's game packs (by their manifests) and the asset packs into the engine; the
  asset packs' open formats.
- `nettai-content-check`: type-checks each pack of the content against its declarations (the engine's API,
  `content/nettai/core.d.luau`, then its support packs' and its own), lints it, and checks that its manifests and
  requires name only modules that exist and that a pack requires only itself and the support packs it uses.
- `nettai-assets`: the battle graphics in typed form, for drawing.
- `nettai-audio`: plays the engine's sound cues through the M4A driver.
- `m4a`: the GBA's M4A (Sappy) sound driver.
- `nettai-netplay`: rollback netplay on [getgud](https://github.com/tangobattle/getgud), its inputs carried by
  [rennet](https://github.com/tangobattle/rennet) over UDP (or any datagram channel), with a simulated lossy network.
- `nettai-render`: draws a battle into frames: the stage, the objects, the HUD, the custom screen and the text, from
  engine state and the packs' graphics; no window, sound or network.
- `nettai-frontend`: the desktop app that shows battles drawn by `nettai-render`: it replays recorded matches or
  plays live, alone or over the network, with sound.
- `nettai-match`: match files, everything a round needs by content key, checked; live play's random draw.
- `nettai-editor`: a desktop app that edits match files and plays them with the frontend.
- `exe6-extract`: extracts EXE6's graphics and sound from the four ROMs into a content pack.
- `exe6-compat`: EXE6's original numbers for the content (`content/exe6/compat`): the codecs of the game's setup
  records, and the trace harness. The engine never depends on it.

## Getting started

You need Rust with edition 2024, and four Mega Man Battle Network 6 ROMs of your own: the US Cybeast Falzar
(`MEGAMAN6_FXXBR6E`) and Cybeast Gregar (`MEGAMAN6_GXXBR5E`), and the Japanese Rockman EXE 6 Dennoujuu Falzar
(`ROCKEXE6_RXXBR6J`) and Dennoujuu Gregar (`ROCKEXE6_GXXBR5J`), which have what the US release cut. Extract a
content pack from them, in that order, into `data/content/exe6` (the directory is gitignored):

    cargo run --release -p exe6-extract -- content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> data/content/exe6

The frontend and the editor find the packs in `data/content` (or the directory `$NETTAI_PACKS` names), each by the
game it says, with no options: an EXE5 pack written there (`exe5-extract content`) sits beside EXE6's. You play one game
at a time, EXE6 or EXE5: a match file names its game, else `--game` does (`exe6` by default), and the battle is that
game's content on its pack. `--pack DIR` names a pack elsewhere, in place of the found one of its game.

Then run the frontend:

    cargo run --release -p nettai-frontend -- --play                                # play live
    cargo run --release -p nettai-frontend -- <trace.jsonl>                         # replay a recorded match
    cargo run --release -p nettai-frontend -- --play --headless 1-120 --out <dir>   # render frames to PNG

In live play alone you are the left navi, and the right one is a stand-in that stands still. Each start draws its setup from a seed, which
it prints: a link battle stage and background, each side's version (Falzar's or Gregar's Beast), a legal random
folder for each side, and five Crosses of both versions in each Cross window (`--game exe5`: an EXE5 match, its sides
EXE5's MegaMan with a random folder). `--seed N` replays a setup, `--stage NAME` forces the stage (`netbattle-1` to `netbattle-96`)
and `--show-folders` prints the folders. Keys: the arrows move, Z is A, X is B, A is L,
S is R, Enter is START and Backspace is SELECT; Space pauses, `.` steps a frame while paused, `-` and `=` change
the speed, F5 restarts the round, H toggles the status line and Esc quits. `--help` lists the options, and
[docs/frontend.md](docs/frontend.md) has the rest. A trace is the recorded inputs of a real match; the traces
live with the verification workspace (below).

The window can be resized; the picture keeps whole pixels. Chip names, telops, the chatbox's descriptions and
messages and the HUD's text lines are drawn with a vector font at the window's resolution, over the pixel art:
Murecho (Latin and Japanese), bundled under the SIL Open Font License in `crates/nettai-render/fonts`.
`--text original` draws them in the game's own fonts instead, exactly as the original does, and `--font <file>`
uses another TrueType or OpenType font ([text-rendering.md](docs/design/text-rendering.md) §9).

To play another player, one hosts and the other joins, over a LAN, or over the Internet with the host's UDP port
forwarded to the host's machine:

    cargo run --release -p nettai-frontend -- --play --host 7777                    # host, the left navi
    cargo run --release -p nettai-frontend -- --play --join 192.0.2.10:7777         # join, the right navi

Both need the same engine, game and content pack (the handshake checks, and says what differs). Each brings their own folder, version and Crosses, drawn from their own `--seed`, and their patch cards
(`--cards`); the host's `--stage` picks the stage. Both play with rollback: inputs go out every frame, the other player's are predicted until they
arrive, and the battle is simulated again when a prediction was wrong. The status line shows the round trip, the
loss, the input delay (`--delay N`, default 2), the rollbacks and the frames waited ([docs/frontend.md](docs/frontend.md)
§2, [rollback.md](docs/design/rollback.md) §4). The netplay tests play netbattles between two rollback sessions over a
simulated network at several latencies, with loss, duplication and reordering, and over UDP on loopback, and check that
both peers stay in step:

    cargo test -p nettai-netplay

These checks need no ROM:

    cargo test --workspace                             # the engine on its own test content
    cargo run -p nettai-content-check                  # every content module type-checks; the lints

## The match editor

A match file sets up a round of one game, EXE6 or EXE5: the game (which is its rules), the arena (stage and background),
and each side's navi, version, folder, Crosses, patch cards, NaviCust and stats, each by its name in the game
(`cannon`, `megaman`; [docs/frontend.md](docs/frontend.md) §6). The editor makes and edits them, checking them
against the content as you go, and plays them:

    cargo build --release -p nettai-frontend -p nettai-editor
    cargo run --release -p nettai-editor -- [match.toml]
    cargo run --release -p nettai-frontend -- --match match.toml     # what Play runs

The arena pane picks the game first: everything below it is that game's, and there is no way to pick another
game's navi, chip, soul or patch card. Changing the game makes a new match of it (the sides start over). A game is
its rules (it has one ruleset), whose systems the pane lists. The panes show only what
those rules have (Crosses with the forms system, patch cards with the patch-cards system, the NaviCust with the
navicust system, souls with the souls system): the arena's stages and backgrounds (the game's); each side's navi
(the game's) and version, with the stats the round starts the navi with (a link navi's level fills in the stats
its save gives at that level, as does switching to a link navi; an edited stat says what the level gives; MegaMan's
optional navi code level); the SP navi deletion times; Import from save; the folder (the game's chips the rules
allow, with their pictures from the game's pack, searchable; a code puts a chip in the selected entry; the Regular
and tag chips; the copies and the Mega, Giga, Regular and tag limits live, as the game's folder rules count them:
EXE6's folder editor's, or EXE5's, content/exe5/rules/folder); the Crosses; the patch cards (the game's; MB used of 80);
the NaviCust (the board as the game draws it, with its frame and command line, edited with the mouse as Tango's is:
drag a program's color swatch onto the grid, or press a placed program to pick it up and drag it; while held it
shows where it would land, lit if it fits and red if not; the wheel or R turns it, C compresses it, right-click,
Delete or a drag off the grid takes it off, Esc puts it back; right-clicking a placed program turns it; the stats it
compiles to show beside it, and the stats-and-bugs block set directly is the pane's other view); every stat (its
weapons, records and forms the game's). The problems with the match show at the bottom as you edit. Play saves the
match and runs `nettai-frontend --match` (the one beside the editor's program, or `--frontend PATH`). A new match
(the editor started without a file, or New) is an empty one of the game (EXE6 to start with): its stock rules, its
first link battle stage, and on each side its MegaMan at his fresh stats with an empty folder, the version's own
Crosses, no patch cards and no NaviCust programs (the problems list says the folders aren't whole until they are).
Random draws a match of the game as live play does, and `nettai-frontend --play --save-match FILE` writes live
play's draw out to edit. `--lang ja` (or the language list) names the chips, navis, Crosses and patch cards in
Japanese. The editor loads the match's game's content and pack as the frontend does (a game's chips with no use yet
left out, with the frontend's warning), and Play hands the frontend the same: `--content` and `--pack` are the
frontend's, and only what you give is passed on.

An EXE5 match's navi pane has the side's karma (the light/dark value): a slider from 0 to 1000 with its number,
presets (light 500, very light 1000, dark 0) and what EXE5 makes of it (dark under 470, the starting mood's tiers at
470, 500 and 1000, holy panels cleared at 499 or under). Its Souls pane has every soul of EXE5 by default, or those
checked, each with its face, of either version. A side's own fields show only when the rules and its navi take them
(`nettai_match::facts`): the version (Gregar or Falzar) with rules whose systems take EXE6's `version`, the navi
code's level with a navi whose definition has `levels` (not EXE5's MegaMan), the SP times with rules that have SP
navi slots (each named by the game's SP navi chip).
"Import from save…" reads an EXE6 .sav (its version, Beast Out and the Crosses it owns, the navi code's level and the
SP times) or an EXE5 one (a .sav or a raw save image: its karma, its souls and its NaviCust board's expansions) into a
match of the save's game: a save of the other game makes a new match of its game first.

The editor is an [iced](https://iced.rs) app, drawn in software (tiny-skia), so it needs no GPU backend. On Linux it
needs the usual windowing libraries (X11 or Wayland, and `libxkbcommon`), and its Open and Save As dialogs use
[rfd](https://github.com/PolyMeilex/rfd), which there needs GTK 3 (`libgtk-3-dev` to build) or an XDG desktop portal.
macOS and Windows need nothing more.

## Docs

- [`docs/design`](docs/design): how the engine and its content are built: the content model
  ([content-model-v2.md](docs/design/content-model-v2.md)), how to write content
  ([content-migration.md](docs/design/content-migration.md)), scripting, the content pack, rollback, the NaviCust
  ([navicust.md](docs/design/navicust.md)), and what other games would need
  ([multi-game.md](docs/design/multi-game.md)).
- [`docs/engine`](docs/engine): the original game's battle routines, specified from the disassembly.
- [`docs/frontend.md`](docs/frontend.md): the frontend, and match files (§6).

## Verification

The engine is verified against the original game in a separate workspace, `bn6battle-verify`: golden traces of
real matches and a chip lab of recorded scenarios, compared frame by frame with the engine. Nothing in this
repository depends on it.
