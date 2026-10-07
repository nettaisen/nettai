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
  [rennet](https://github.com/tangobattle/rennet) over a WebRTC data channel (or any datagram channel), with a
  simulated lossy network.
- `nettai-rtc`: netplay's transport, WebRTC data channels (native on [rtc](https://github.com/webrtc-rs/rtc), the
  browser's on wasm32), met in a room of the signaling server or directly, made again when they drop.
- `nettai-render`: draws a battle into frames: the stage, the objects, the HUD, the custom screen and the text, from
  engine state and the packs' graphics; no window, sound or network.
- `nettai-frontend`: plays battles for a host app to show, as a library with no window, audio device or command
  line in it: it loads a game (its content, graphics, strings and sound), and its player runs a set live or over the
  network from the buttons, by the clock or a tick at a time, and gives the host the picture (`nettai-render`'s) and
  the sound as samples ([docs/frontend.md](docs/frontend.md) §7, with the host loop).
- `nettai`: the app (Slint, a prototype): a title, offline play against the stand-in, the build creator, the
  netplay lobby (rooms through `nettai-rtc`, or a direct link) and the replays around the battle, navigated by keys,
  a gamepad or touch, in each language its catalogs have ([docs/app.md](docs/app.md)).
- `nettai-tools`: `nettai-tool`, the command line with no window: headless frames (a trace's, a match file's, a
  replay's), the content and trace audits, the replay of recorded matches, match setups and replays checked
  ([docs/tools.md](docs/tools.md)).
- `nettai-match`: match files and their sides (a build is one), everything a round needs by content key, checked;
  live play's random pick.
- `nettai-extract`: shared EXE5/EXE6 asset extraction library and CLI, including placeholders for missing ROMs.
- `exe6-compat`: EXE6's original numbers for the content (`content/exe6/compat`): the codecs of the game's setup
  records, and the trace harness. The engine never depends on it.

## Getting started

You need Rust with edition 2024. Extract assets from your own US or Japanese EXE5/EXE6 ROMs into a new or empty
pack directory (the directories below are gitignored). ROMs can be supplied in any order:

    cargo run --release -p nettai-extract -- exe6 data/exe6 <falzar-us> <gregar-us> <falzar-jp> <gregar-jp>
    cargo run --release -p nettai-extract -- exe5 data/exe5 <protoman-us> <colonel-us> <protoman-jp> <colonel-jp>

Any subset works, including a single ROM or no ROMs. Available sources supply their assets; unavailable graphics
become checkerboard placeholders and missing songs become silence. The pack includes `extraction.txt` with missing
sources and generated assets. Add `--content content` to check the game's definitions against the written pack.

Applications can embed the same extractor without launching a process or writing files:

```rust
use nettai_extract::{extract, Game, RomSet};

let mut roms = RomSet::default();
roms.insert(rom_bytes)?; // Vec<u8>, identified by the ROM header
let assets = extract(Game::Exe6, &roms)?;
// Use assets.graphics and assets.sound directly, or serialize the pack:
let files = assets.files()?; // Vec<(relative_path, bytes)>
```

The [extractor documentation](crates/nettai-extract/README.md) describes source selection, diagnostics and the API.

The app and the command line find the packs in `data` (or the directory `$NETTAI_PACKS` names), each by the
game it says, with no options: an EXE5 pack written there (`nettai-extract exe5`) sits beside EXE6's. You play one game
at a time, EXE6 or EXE5: a build and a match file name their game and a trace states its own (there is no default
game), and the battle is that game's content on its pack. Then run the app:

    cargo run --release -p nettai

nettai ([docs/app.md](docs/app.md)) is the player's program, navigated by the keyboard, a gamepad, the mouse or
touch: Play (a game, your build or a random side, against a stand-in that stands still, or a match file from its
matches folder), the builds and their creator (below), the netplay lobby, the replays of every set played, and the
settings, kept between runs. In the battle the arrows move, Z is A, X is B, A is L, S is R, Enter is START and
Backspace is SELECT, and Esc pauses. A match file sets the game, its rounds (one `[[round]]` each: the set is as
many rounds, three for the original's best of three), each side's navi, folder, version, forms, patch cards,
NaviCust and what its save brings to the stats (the rules build the rest); its optional `seed` sets the battle's
RNG, and without one the seed comes from the clock ([docs/frontend.md](docs/frontend.md) §6).

The picture keeps whole pixels at any window size. Chip names, telops, the chatbox's descriptions and messages and
the HUD's text lines are drawn with a vector font at the window's resolution, over the pixel art: Murecho (Latin
and Japanese), bundled under the SIL Open Font License in `crates/nettai-render/fonts`. Settings' battle text draws
them in the game's own fonts instead, exactly as the original does
([text-rendering.md](docs/design/text-rendering.md) §9).

To play another player, both choose the same game in the lobby and meet in a room of the signaling server by its
code (the first in hosts, the left navi), through NATs with a public STUN server; or one hosts and the other joins
directly, over a LAN, or over the Internet with the host's UDP port 47474 forwarded to the host's machine (nothing
is exchanged before the connection, so nothing is authenticated). The signaling server is `signaling/`, a
Cloudflare Worker; `npx wrangler dev` there runs one locally, and `npx wrangler deploy` puts it on your own
Cloudflare account:

    cd signaling && npx wrangler dev                            # http://127.0.0.1:8787
    NETTAI_SIGNAL=ws://127.0.0.1:8787 cargo run --release -p nettai

Both need the same engine, game and content pack (the handshake checks, and says what differs). Each brings their
build (or a random side), and the two agree the settings in the lobby; each commits to its half of the seed and
its side before either reveals them, and the battle's RNG comes from both halves. Both play with rollback: inputs
go out every frame, the other player's are predicted until they arrive, and the battle is simulated again when a
prediction was wrong. The battle shows the round trip, the present delay and the rollbacks
([docs/frontend.md](docs/frontend.md) §2, [rollback.md](docs/design/rollback.md) §4). A connection that drops is
made again, and the battle goes on where it was (it says it is reconnecting meanwhile); after 30 seconds the match
ends. The netplay tests play netbattles between two rollback sessions over a simulated network at several
latencies, with loss, duplication and reordering, and over WebRTC on loopback (through an outage), and check that
both peers stay in step:

    cargo test -p nettai-netplay -p nettai-rtc -p nettai-frontend

Every set played, alone or over the network, is recorded to the app's replays folder, and Replays watches one again
from either console; a replay plays only on the engine and content it was made with
([frontend.md](docs/frontend.md) §8).

The command line, `nettai-tool` ([docs/tools.md](docs/tools.md)), has no window: it renders chosen frames to PNGs,
audits the content and the traces, says a match's setup and checks a replay. `--pack DIR` names a pack elsewhere,
in place of the found one of its game, and `--help` lists the options:

    cargo run --release -p nettai-tools -- --match match.toml --headless 1-120 --out <dir>   # render frames to PNG
    cargo run --release -p nettai-tools -- <trace.jsonl> --headless 150,300 --out <dir>      # a recorded match's
    cargo run --release -p nettai-tools -- --replay set.ntrp                                 # does it reproduce?

A trace is the recorded inputs of a real match; the traces live with the verification workspace (below).

These checks need no ROM:

    cargo test --workspace                             # the engine on its own test content
    cargo run -p nettai-content-check                  # every content module type-checks; the lints

## Editing Luau in Zed

Install the [Luau extension](https://github.com/4teapo/zed-luau), open the repository root, and generate the
editor's type definitions once:

    python3 .zed/generate-luau-definitions.py

The project settings load the generated `.zed/luau-globals.d.luau`, use the new type solver, and select the
standard Luau platform. The engine and pack declarations must be loaded together because they reference each
other; passing each `.d.luau` to the server separately leaves unresolved types. The generated file is gitignored.
The sources use `declare extern type`, which current luau-lsp and the project's content checker both understand.
The launch arguments load only the combined file; `types.definitionFiles` also lists the source declarations so
the server recognizes them as definitions when opened, rather than reporting syntax errors for ordinary scripts.

After editing a source `.d.luau`, run the Zed task **Luau: regenerate editor definitions** (or the command above),
then **editor: restart language server** from the command palette with a Luau file open. This also applies the
configuration if the server was already running. The existing `content/.luaurc` supplies the pack require aliases.
`nettai-content-check` remains the check for each pack's permitted dependencies and types; the editor loads all
packs' declarations into one shared environment. The editor also follows imports and can report cross-module
type errors that the checker, which types `require` as `any`, does not catch. A module with a same-named folder
belongs in that folder's `init.luau`; luau-lsp resolves the folder before a sibling `.luau` file.

To check every module with import resolution, regenerate the definitions and run the language server's CLI
(using `luau-lsp` on your PATH, or the full path to Zed's downloaded binary):

    luau-lsp analyze --platform=standard --flag:LuauSolverV2=true \
      --definitions=.zed/luau-globals.d.luau --ignore='**/*.d.luau' content

## Builds

A build is a player's side of one game, named and kept in the app's data folder ([docs/app.md](docs/app.md) §8):
the navi, the folder, the patch cards and NaviCust, and what the game's rules take of a side besides (its facts:
EXE6's version, Crosses and Beast Out, EXE5's karma and souls), each by its name in the game, written as a match
file writes a side ([docs/frontend.md](docs/frontend.md) §6). A side states no stats: the game's rules build them
as the round is set up.

The build creator (nettai's Builds) makes and edits them, checking them against the content as you go. It lays a
build out from the game's rules' setup alone, a tab for each kind of fact, and names no game's feature: the navi
and the facts of one value, with the stats the round starts it with; the folder, with the chips the rules allow,
their pictures and codes, and the limits counted live; a checklist of faces (EXE6's Crosses, EXE5's souls); the
NaviCust's board, placed with the keys, a pad or the mouse; the patch cards with their effects; EXE5's auto battle
data. FROM A SAVE… reads a whole side from a save of the build's game (a .sav, or a raw save image as Tango's
netplay templates hold), or the auto battle data alone; a save of another game is refused. Every build plays at the
most its game allows: what a save brings to the stats, the navi code's level, the SP navi times and EXE5's auto
battle records stay at the rules' defaults, and an import says which of the save's it set so. Builds are chosen in
Play and in the lobby.

A whole match file (both sides and the rounds) is written by hand, or by `nettai-tool --match FILE --save-match OUT`
(the match with its seed), and Play plays one from the app's matches folder.

## Docs

- [`docs/design`](docs/design): how the engine and its content are built: the content model
  ([content-model-v2.md](docs/design/content-model-v2.md)), how to write content
  ([content-migration.md](docs/design/content-migration.md)), scripting, the content pack, rollback, the NaviCust
  ([navicust.md](docs/design/navicust.md)), what other games would need
  ([multi-game.md](docs/design/multi-game.md)), and where Rust ends and a game's Luau begins
  ([rust-and-luau.md](docs/design/rust-and-luau.md)).
- [`docs/engine`](docs/engine): the original game's battle routines, specified from the disassembly.
- [`docs/frontend.md`](docs/frontend.md): the frontend, and match files (§6).
- [`docs/app.md`](docs/app.md): the app, its screens, netplay and the build creator.
- [`docs/tools.md`](docs/tools.md): the command line, its frames and audits.

## Verification

The engine is verified against the original game in a separate workspace, `bn6battle-verify`: golden traces of
real matches and a chip lab of recorded scenarios, compared frame by frame with the engine. Nothing in this
repository depends on it.
