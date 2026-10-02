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
- `nettai-content-check`: type-checks content against `content/bn6/core.d.luau`, and lints it.
- `nettai-assets`: the battle graphics in typed form, for drawing.
- `nettai-audio`: plays the engine's sound cues through the M4A driver.
- `m4a`: the GBA's M4A (Sappy) sound driver.
- `nettai-netplay`: rollback netplay on [getgud](https://github.com/tangobattle/getgud), with a simulated network.
- `nettai-frontend`: a desktop app that draws battles: it replays recorded matches or plays live.
- `bn6-extract`: extracts BN6's graphics and sound from a ROM into a content pack.
- `bn6-compat`: BN6's original numbers for the content (`content/bn6/compat`): the codecs of the game's setup
  records, and the trace harness. The engine never depends on it.

## Getting started

You need Rust with edition 2024, and the two US Mega Man Battle Network 6 ROMs of your own: Cybeast Falzar
(`MEGAMAN6_FXXBR6E`) and Cybeast Gregar (`MEGAMAN6_GXXBR5E`). Extract a content pack from them into
`data/content/bn6`, where the tools look by default (the directory is gitignored):

    cargo run --release -p bn6-extract -- content <falzar-rom> <gregar-rom> data/content/bn6

Then run the frontend:

    cargo run --release -p nettai-frontend -- --play                                # play live
    cargo run --release -p nettai-frontend -- <trace.jsonl>                         # replay a recorded match
    cargo run --release -p nettai-frontend -- --play --headless 1-120 --out <dir>   # render frames to PNG

In live play you are the left navi, and the right one stands still. Keys: the arrows move, Z is A, X is B, A is L,
S is R, Enter is START and Backspace is SELECT; Space pauses, `.` steps a frame while paused, `-` and `=` change
the speed, F5 restarts the round, H toggles the status line and Esc quits. `--help` lists the options, and
[docs/frontend.md](docs/frontend.md) has the rest. A trace is the recorded inputs of a real match; the traces
live with the verification workspace (below).

Netplay is a library for now, with no online play in the frontend. Its tests play netbattles between two rollback
sessions over simulated links at several latencies, and check that both peers stay in step:

    cargo test -p nettai-netplay

These checks need no ROM:

    cargo test --workspace                             # the engine on its own test content
    cargo run -p nettai-content-check -- content/bn6   # every content module type-checks; the lints

## Docs

- [`docs/design`](docs/design): how the engine and its content are built: the content model
  ([content-model-v2.md](docs/design/content-model-v2.md)), how to write content
  ([content-migration.md](docs/design/content-migration.md)), scripting, the content pack, rollback, and what
  other games would need ([multi-game.md](docs/design/multi-game.md)).
- [`docs/engine`](docs/engine): the original game's battle routines, specified from the disassembly.
- [`docs/frontend.md`](docs/frontend.md): the frontend.

## Verification

The engine is verified against the original game in a separate workspace, `bn6battle-verify`: golden traces of
real matches and a chip lab of recorded scenarios, compared frame by frame with the engine. Nothing in this
repository depends on it.
