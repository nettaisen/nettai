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
- `nettai-frontend`: plays battles for a host app to show, as a library with no window, audio device or command
  line in it: it loads a game (its content, graphics, strings and sound), and its player runs a set live or over the
  network from the buttons, by the clock or a tick at a time, and gives the host the picture (`nettai-render`'s) and
  the sound as samples ([docs/frontend.md](docs/frontend.md) §7, with the host loop).
- `nettai-demo`: the desktop program over `nettai-frontend`: a window and its keys, the command line, headless
  frames, the audits, the replay of recorded matches, and the sound through the audio device.
- `nettai-match`: match files, everything a round needs by content key, checked; the editor's random pick.
- `nettai-demo-editor`: a desktop app that edits match files and plays them with `nettai-demo`.
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

The frontend and the editor find the packs in `data` (or the directory `$NETTAI_PACKS` names), each by the
game it says, with no options: an EXE5 pack written there (`nettai-extract exe5`) sits beside EXE6's. You play one game
at a time, EXE6 or EXE5: a match file names its game and a trace states its own (there is no default game), and the battle is that
game's content on its pack. `--pack DIR` names a pack elsewhere, in place of the found one of its game.

Create a match file with the match editor below, then run the frontend:

    cargo run --release -p nettai-demo -- --match match.toml                              # play live
    cargo run --release -p nettai-demo -- <trace.jsonl>                                    # replay a recorded match
    cargo run --release -p nettai-demo -- --match match.toml --headless 1-120 --out <dir>  # render frames to PNG

In live play alone you are the left navi, and the right one is a stand-in that stands still. The match file sets
the game, arena, each side's navi, folder, version, forms, patch cards, NaviCust and what its save brings to the
stats (the rules build the rest). Its optional `seed` sets the
battle's RNG; without one, the seed comes from the clock. Each start prints it, and `--show-folders` prints the
folders. The editor's Random button creates a random setup you can save and play. Keys: the arrows move, Z is A, X is B, A is L,
S is R, Enter is START and Backspace is SELECT; Space pauses, `.` steps a frame while paused, `-` and `=` change
the speed, F5 restarts the round and Esc quits. `--help` lists the options, and
[docs/frontend.md](docs/frontend.md) has the rest. A trace is the recorded inputs of a real match; the traces
live with the verification workspace (below).

The window can be resized; the picture keeps whole pixels. Chip names, telops, the chatbox's descriptions and
messages and the HUD's text lines are drawn with a vector font at the window's resolution, over the pixel art:
Murecho (Latin and Japanese), bundled under the SIL Open Font License in `crates/nettai-render/fonts`.
`--text original` draws them in the game's own fonts instead, exactly as the original does, and `--font <file>`
uses another TrueType or OpenType font ([text-rendering.md](docs/design/text-rendering.md) §9).

To play another player, one hosts and the other joins, over a LAN, or over the Internet with the host's UDP port
forwarded to the host's machine:

    cargo run --release -p nettai-demo -- --match match.toml --host 7777                   # host, the left navi
    cargo run --release -p nettai-demo -- --match match.toml --join 192.0.2.10:7777        # join, the right navi

Both need the same engine, game and content pack (the handshake checks, and says what differs). Each brings the
left side of their own match file; the host's file supplies the arena. The battle's RNG comes from both players'
randomly generated seed halves. Both play with rollback: inputs go out every frame, the other player's are predicted until they
arrive, and the battle is simulated again when a prediction was wrong. The window's title shows the round trip, the
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
and each side's navi, folder, patch cards and NaviCust, and what the game's rules take of a side besides (its
facts: EXE6's version and Crosses, EXE5's karma and souls, and what a save brings to the stats, the base HP and the
Regular memory), each by its name in the game. A side states no stats: the game's rules build them as the round is
set up
(`cannon`, `megaman`; [docs/frontend.md](docs/frontend.md) §6). The editor makes and edits them, checking them
against the content as you go, and plays them:

    cargo build --release -p nettai-demo -p nettai-demo-editor
    cargo run --release -p nettai-demo-editor -- [--game GAME] [match.toml]
    cargo run --release -p nettai-demo -- --match match.toml     # what Play runs

A match's game is chosen before anything else, and none is preselected: an opened file is of the game it names,
and for a new match (the editor started without a file, or New) the editor asks which game first (`--game` answers
on the command line). The arena pane shows the game: everything below it is that game's, and there is no way to pick another
game's navi, chip, soul or patch card. Changing the game there makes a new match of it (the sides start over). A game is
its rules (it has one ruleset), whose systems the pane lists. The panes show only what
those rules have (patch cards with the patch-cards system, the NaviCust with the navicust system, and each fact
the rules' systems declare of a side): the arena's stages and backgrounds (the game's); each side's navi
(the game's), with the stats the round starts the navi with (what the rules build: a link navi's from its level, a
team navi's HP from its story; MegaMan's optional navi code level); the SP navi deletion times; Import from save;
the folder (the game's chips the rules
allow, with their pictures from the game's pack, searchable; a code puts a chip in the selected entry; the Regular
and tag chips; the copies and the Mega, Giga, Regular and tag limits live, as the game's folder rules count them:
EXE6's folder editor's, or EXE5's, content/exe5/rules/folder); the lists the rules take of a side (EXE6's
Crosses, EXE5's souls); the patch cards (the game's; MB used of 80);
the NaviCust (the board as the game draws it, with its frame and command line, edited with the mouse as Tango's is:
drag a program's color swatch onto the grid, or press a placed program to pick it up and drag it; while held it
shows where it would land, lit if it fits and red if not; the wheel or R turns it, C compresses it, right-click,
Delete or a drag off the grid takes it off, Esc puts it back; right-clicking a placed program turns it; the stats it
compiles to show beside it); every stat as the round starts it (shown, not edited). The problems with the match show
at the bottom as you edit. Play saves the
match and runs `nettai-demo --match` (the one beside the editor's program, or `--frontend PATH`). A new match
(the editor started without a file, or New) is an empty one of the game the editor asks for: its stock rules, its
first link battle stage, and on each side its MegaMan at his fresh stats with an empty folder, its facts the rules'
defaults (an EXE6 side has no version and no Crosses stated: its navi pane asks its version, falzar or gregar, with
nothing chosen, since neither is assumed, and choosing one states that version's five Crosses; an EXE5 match has
neither to state), no patch cards and no NaviCust programs (the problems list
says the folders aren't whole and the versions aren't chosen until they are).
Random picks a match of the game to save or play, and `nettai-demo --match match.toml --save-match FILE` writes
the match played with its seed (or the one netplay agreed) out to edit. `--lang ja` (or the language list) names the chips, navis, Crosses and patch cards in
Japanese. The editor loads the match's game's content and pack as the frontend does (a game's chips with no use yet
left out, with the frontend's warning), and Play hands the frontend the same: `--content` and `--pack` are the
frontend's, and only what you give is passed on.

**A side's facts** (what its game's rules take of it: docs/frontend.md §6) get their controls from the rules'
own declarations, by each setup field's type, and the editor names none of them: a flag is a checkbox (EXE6's
Beast out, EXE5's Soul unison), a number a field to type it in with the rules' default as its placeholder (EXE5's
Karma, EXE6's Bug frags), an enum a list of the variants the rules name (EXE6's Version, nothing chosen until the
side states one), all on the navi pane under "What the rules take" (EXE6's Hp, Reg up and Sun among them); a
list of definitions has a pane of its own with a checkbox for each one
it may hold (EXE6's Crosses: the side's navi's ten of both versions, five at most, kept in the window's
order; EXE5's Souls: the twelve the rules' default lists, each with its face). Each has a Default button where the
rules give it a default and it isn't that. What the rules require and assume nothing of (EXE6's version, and its
Crosses, where an empty list means none) starts unchosen, and the editor fills in what it can: choosing the version
states that version's own five Crosses, to edit from there ("None" and "Its version's own" on the pane).
A game that declares another fact has its control here with no change to the editor.

An EXE5 match's Auto battle pane (shown where the game's rules have auto
battle: not for EXE6) is what a navi in auto battle plays from the side's save, the Dark MegaMan its failed Chaos Unison
brings and its own navi under DarkInvs, whole (docs/frontend.md §6): on the left the save's 42 places in the six lists
the game writes them in, each a chip, a pattern's number, a 0 or empty; in the middle its eight pattern records, each
where the navi stands from its target, its five chip places and its score; on the right the game's chips, searched
(those the game writes in the selected place's list, or every chip), where "put" fills the selected place and moves on
to the next as the folder pane does. An entry the game wouldn't write where it is has a quiet note, no error. "From the
folder" fills it in as the game would have written it for a player who used each chip of the side's folder once; "From
a save…" takes it alone from an EXE5 save; "Nothing learned" is what the game writes of a player it has learned nothing
of (a new match's), and "No data" a save that never finished a battle (the file then states none). A side's own fields
show only when the rules and its navi take them: the navi
code's level with a navi whose definition has `levels` (not EXE5's MegaMan), the SP times with rules that have SP
navi slots (each named by the game's SP navi chip).
"Import from save…" reads an EXE6 .sav (its version, Beast Out and the Crosses it owns, the navi code's level and the
SP times) or an EXE5 one (a .sav or a raw save image: its karma, its souls, its NaviCust board's expansions and its
auto battle data) into a match of the save's game: a save of the other game makes a new match of its game first.

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
