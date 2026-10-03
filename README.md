# nettai

A Mega Man Battle Network battle engine in Rust. Its first game is Mega Man Battle Network 6 (US Falzar), whose
battles are reimplemented frame for frame from the original, routine by routine from the disassembly. Other
Battle Network games are meant to follow on the same core ([multi-game.md](docs/design/multi-game.md) surveys
what that takes).

The engine is a content-independent core with BN6's ruleset on it. The game's content (chips, navis and their
forms, weapons, stages, the rule tables) is Luau in [`content/bn6`](content/bn6), committed here and named by
keys rather than the original's numbers. The graphics and sound are not in this repository: you extract them
from a ROM you own into a content pack.

A battle is deterministic and cloneable, and the players' inputs are its only input, which is what rollback
netplay needs.

## Crates

- `nettai-battle`: the engine: the simulation core, BN6's ruleset, and the content model it runs on.
- `nettai-content-api`: the contract between the core and content: the API scripts call, definitions, handles.
- `nettai-luau`: the Luau runtime for content (sandboxed scripts that keep no state of their own).
- `nettai-content`: loads a content root and an asset pack into the engine; the pack's open formats.
- `nettai-content-check`: type-checks a content root against the engine's API (`content/nettai/core.d.luau`) and
  the root's own declarations, and lints it.
- `nettai-assets`: the battle graphics in typed form, for drawing.
- `nettai-audio`: plays the engine's sound cues through the M4A driver.
- `m4a`: the GBA's M4A (Sappy) sound driver.
- `nettai-netplay`: rollback netplay on [getgud](https://github.com/tangobattle/getgud), its inputs carried by
  [rennet](https://github.com/tangobattle/rennet) over UDP (or any datagram channel), with a simulated lossy network.
- `nettai-frontend`: a desktop app that draws battles: it replays recorded matches or plays live.
- `nettai-match`: match files, everything a round needs by content key, checked; live play's random draw.
- `nettai-editor`: a desktop app that edits match files and plays them with the frontend.
- `bn6-extract`: extracts BN6's graphics and sound from the four ROMs into a content pack.
- `bn6-compat`: BN6's original numbers for the content (`content/bn6/compat`): the codecs of the game's setup
  records, and the trace harness. The engine never depends on it.

## Getting started

You need Rust with edition 2024, and four Mega Man Battle Network 6 ROMs of your own: the US Cybeast Falzar
(`MEGAMAN6_FXXBR6E`) and Cybeast Gregar (`MEGAMAN6_GXXBR5E`), and the Japanese Rockman EXE 6 Dennoujuu Falzar
(`ROCKEXE6_RXXBR6J`) and Dennoujuu Gregar (`ROCKEXE6_GXXBR5J`), which have what the US release cut. Extract a
content pack from them, in that order, into `data/content/bn6` (the directory is gitignored):

    cargo run --release -p bn6-extract -- content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> data/content/bn6

The frontend and the editor load every pack in `data/content` (or the directory `$NETTAI_PACKS` names), each by the
game it says, with no options: a BN5 pack written there (`bn5-extract content`) loads beside BN6's, and BN5's
content root with it once it loads. `--pack DIR` names a pack elsewhere, in place of the found one of its game.

Then run the frontend:

    cargo run --release -p nettai-frontend -- --play                                # play live
    cargo run --release -p nettai-frontend -- <trace.jsonl>                         # replay a recorded match
    cargo run --release -p nettai-frontend -- --play --headless 1-120 --out <dir>   # render frames to PNG

In live play alone you are the left navi, and the right one is a stand-in that stands still. Each start draws its setup from a seed, which
it prints: a link battle stage and background, each side's game (Falzar's or Gregar's Beast), a legal random folder
for each side, and five Crosses of both games in each Cross window. `--seed N` replays a setup, `--stage NAME` forces the stage (`netbattle-1` to `netbattle-96`)
and `--show-folders` prints the folders. Keys: the arrows move, Z is A, X is B, A is L,
S is R, Enter is START and Backspace is SELECT; Space pauses, `.` steps a frame while paused, `-` and `=` change
the speed, F5 restarts the round, H toggles the status line and Esc quits. `--help` lists the options, and
[docs/frontend.md](docs/frontend.md) has the rest. A trace is the recorded inputs of a real match; the traces
live with the verification workspace (below).

The window can be resized; the picture keeps whole pixels. Chip names, telops, the chatbox's descriptions and
messages and the HUD's text lines are drawn with a vector font at the window's resolution, over the pixel art:
Murecho (Latin and Japanese), bundled under the SIL Open Font License in `crates/nettai-frontend/fonts`.
`--text original` draws them in the game's own fonts instead, exactly as the original does, and `--font <file>`
uses another TrueType or OpenType font ([text-rendering.md](docs/design/text-rendering.md) §9).

To play another player, one hosts and the other joins, over a LAN, or over the Internet with the host's UDP port
forwarded to the host's machine:

    cargo run --release -p nettai-frontend -- --play --host 7777                    # host, the left navi
    cargo run --release -p nettai-frontend -- --play --join 192.0.2.10:7777         # join, the right navi

Both need the same engine and the same content pack (the handshake checks, and says what differs). Each brings their
own folder, game and Crosses, drawn from their own `--seed`, and their patch cards (`--cards`); the host's `--stage`
picks the stage. Both play with rollback: inputs go out every frame, the other player's are predicted until they
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

A match file sets up a round: the arena (stage and background), and each side's ruleset, navi, game, folder,
Crosses, patch cards, NaviCust and stats, by content key ([docs/frontend.md](docs/frontend.md) §6). The editor makes
and edits them, checking them against the content as you go, and plays them:

    cargo build --release -p nettai-frontend -p nettai-editor
    cargo run --release -p nettai-editor -- [match.toml]
    cargo run --release -p nettai-frontend -- --match match.toml     # what Play runs

Its panes show only what the side's ruleset has (Crosses with the forms system, patch cards with the patch-cards
system, the NaviCust with the navicust system): the arena; each side's ruleset, navi and game, with the stats the
round starts the navi with; the folder (the chips the side's folder rules allow, with their pictures, searchable, a
code puts a chip in the selected entry; the Regular and tag chips; the copies and the Mega, Giga, Regular and tag
limits live, as the game's folder rules count them); the Crosses; the patch cards (MB used of 80); the NaviCust (the
board as the side's game draws it, with its frame and command line, edited with the mouse as Tango's is: drag a
program's colour swatch onto the grid, or press a placed program to pick it up and drag it; while held it shows
where it would land, lit if it fits and red if not; the wheel or R turns it, C compresses it, right-click, Delete
or a drag off the grid takes it off, Esc puts it back; right-clicking a placed program turns it; the stats it
compiles to show beside it, and the stats-and-bugs block set directly is the pane's other view); every stat. The problems with the match show at the bottom as you edit. Play saves the match and
runs `nettai-frontend --match` (the one beside the editor's program, or `--frontend PATH`). Random draws a match as
live play does, and `nettai-frontend --play --save-match FILE` writes live play's draw out to edit. `--lang ja` (or
the language list) names the chips, navis, Crosses and patch cards in Japanese. The editor loads the content and
every pack in `data/content` as the frontend does (each chip's pictures from its own game's pack), and Play hands
the frontend the same: `--content` and `--pack` are the frontend's, and only what you give is passed on.

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
