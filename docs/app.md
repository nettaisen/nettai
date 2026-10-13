# The app (`nettai` and `nettai-ui`)

**Status (2026-10-12): the redesigned app's foundation.** The user asked for the app's UI to be rethought and
redesigned entirely, in Slint, and approved a design: [design/app-ui.md](design/app-ui.md) is that design (what
the user asked for, every decision, the look, the rules every screen follows, each screen). This document is the
app as it is built: how the battle is hosted in a Slint window (§1, §5), how the app is put together and how a
screen is added or filled in (§2), and the parts every screen shares (§3, §9 to §14).

What exists: the design system, focus and navigation, the keyboard, mouse, touch and gamepads, scaling and the
upright-phone switch, the languages, the errors and the log, Loading, the main menu with its reel and the title
music, Settings, the two cards before a battle, and the battle's host with its menu overlay. Training is built
(§15), and so are Play, the netplay UI (§4), Builds (§8) and Replays (§16: the list, the viewer with its
transport, the export): each is one Slint file and one Rust module (§2).

Builds is filled in (§8): the list with a build's overview, a new build, a save's import, and the editor with
its five tabs behind its tab interface: Navi, Folder, NaviCust, Patch cards and, in a game whose setup takes
it, Auto battle. The overview, a build's row and a version's tag are components any screen uses.

nettai is the player's host of nettai-frontend (docs/frontend.md: the battle it plays and draws); nettai-tools is
the command line with no window, for frames, audits and checks (docs/tools.md).

    NETTAI_PACKS=<packs> cargo run --release -p nettai

The packs are found as nettai-tool finds them (`$NETTAI_PACKS`, else `data`, each pack by the game it says;
written by `nettai-extract <exe4|exe5|exe6> <pack-dir> [ROM ...]`). What the player makes is kept in the app's
data folder: `$NETTAI_DATA`, else the system's (`~/Library/Application Support/nettai` on macOS,
`$XDG_DATA_HOME/nettai` or `~/.local/share/nettai` on Linux, `%APPDATA%\nettai` on Windows). There, builds are in
`builds/<game>/` (`$NETTAI_BUILDS`), sets are recorded to `replays/` (`$NETTAI_REPLAYS`), the settings are
`settings.toml` (`$NETTAI_SETTINGS`), what playing replays out gave is `replays-played.toml` beside it (§16), and
the log is `nettai.log` (`$NETTAI_LOG`). For netplay, `$NETTAI_SIGNAL` names the signaling server (§4). Some
variables are for development:

| Variable | What it does |
|---|---|
| `NETTAI_LANG=<code>` | The first language shown, instead of the settings' or the system's. |
| `NETTAI_TRAINING=<game>` | Starts straight into a training session of the game: new builds of its first two versions, the other a dummy that stands still. |
| `NETTAI_REPLAY=<file>` | Starts straight into watching the replay, its game selected; `NETTAI_REPLAY_SPEED=half\|1\|2\|4` at that speed. |
| `NETTAI_FFMPEG=<program>` | The `ffmpeg` a video is made with, instead of the one found (§16). |
| `NETTAI_PLAY_STATS`, `NETTAI_KEY_PROBE` | The latency figures of §5, in a battle, on the main menu and in a replay's viewer; with the first, the replays' listing's and play-outs' times, and where each seek of the viewer landed (§16). |
| `NETTAI_VIEWER_PROBE=<seconds>:<step>,...` | Drives a replay's viewer as a player would (§5): `click:<part>` (the bar pressed and let go `part` of the way along), `drag:<from>:<to>` (pressed, moved over a second, let go), `speed:<half\|1\|2\|4>`. The pointer's are the window's own events. |
| `NETTAI_WINDOW_PROBE=<minimize\|hide>:<after>:<for>` | Minimizes or hides the window `after` seconds from the start, for `for` seconds (§1, "The clock when the window isn't drawn"). |
| `NETTAI_NO_TIMER_CLOCK` | Leaves the timer's clock out, to see what the window's frames do alone. |
| `NETTAI_PHYSICAL_PIXELS` | Presents the picture at the display's density. |
| `NETTAI_NO_TEXTURE` | Shows the picture as a new image each time, not in the texture (§1). |
| `NETTAI_TOUR=<folder>` | The screenshot pass (§13); `NETTAI_TOUR_MOTION=1` its motion probe; `NETTAI_TOUR_ONLY=replays` one screen's steps alone. A build with the `tour` feature's alone (`--features tour`): a player's has none of it. |

## 1. The battle in a Slint window

**The clock is the window's rendering.** The rendering notifier's `BeforeRendering` runs the app's frame
(`App::frame`, shell.rs) before Slint draws the frame:

1. It fits the layout to the window (§9), reads the gamepads, follows the games as they load, and gives each
   screen its frame (`Screen::frame`: a link's datagrams are fed here).
2. It makes a battle ready, a few steps a frame, while the two cards show; or runs the battle's ticks due, with
   the buttons held.
3. If a tick ran, or the picture's room changed, it presents a new picture, which that same frame draws.
4. It gives each screen the frame's end (`Screen::frame_end`: what the ticks made to send goes out), queues the
   sound, and keeps the one focus (§11).

`AfterRendering` notes that the frame was drawn and asks for the next one, so the app runs at the display's rate:
120 frames a second on this Mac's display. A frame shows the picture presented for it, not the previous frame's.

**The app keeps its time while something runs** (`awake.rs`). macOS gives an app that isn't in front, or whose
window is covered, the lowest priority and delays its timers (App Nap); on a busy machine it then gets no time at
all. The screenshot pass found it: covered by another window while builds loaded the machine (load average 170),
it stopped mid-walk, in the middle of a frame, and never went on. So while a battle is loading or running, while
a screen says it has something that must go on (`Shell::stay_awake`: a link to another player), and during the
screenshot pass, the app holds a Foundation activity (user-initiated, latency-critical, the display kept on: a
player on a pad presses no key). It was checked by the system's list of them (`pmset -g assertions` names the
app's, "a battle is running", during a training battle), not by covering the window under load again. Windows
and Linux have nothing held.

**The clock when the window isn't drawn.** The system draws no frames for a window that is minimized or hidden,
and the clock is the drawing: a battle in one stood still for as long, and a link would have gone unpolled,
which the other player sees as a stalled peer. So a timer (16 ms, `main.rs`) runs the app's frame itself
(`App::idle`) when the window hasn't drawn one for 50 ms and something must go on: a battle, the cards before
one, or a screen's link (`Shell::stay_awake`). Such a frame is the whole frame but its picture (`Shell::show`
presents nothing: there is no frame to show it in, and the window's OpenGL isn't the thread's), so the battle
ticks by the clock, the sound plays, and `Screen::frame` and `Screen::frame_end` run, which is where a link is
fed and polled. Each asks the window for a redraw, so the window's own frames take over again when it shows.
While the window draws, the timer does nothing.

Measured on macOS (2026-10-12): a training battle with the window taken away for 8 seconds, by
`NETTAI_WINDOW_PROBE` for the first two rows and by another program's window for the last two (load average 165
to 318):

| The window | Without the timer | With it |
|---|---|---|
| minimized | no frame, no tick, for the 8 seconds | 59 frames a second run by the timer, 59.3 to 60.1 ticks a second |
| hidden | no frame, no tick | 59 frames a second by the timer, 59.5 to 60.0 ticks a second |
| covered by another program's window the size of the screen (the system says "covered") | the window keeps drawing: 120 frames a second, 59.75 ticks | the same; the timer runs nothing |
| behind another program's full-screen window (the system says "covered") | the same: 120 frames, 59.75 ticks | the same |

Minimizing and restoring each stop the app for about 0.4 seconds (the system's animation, on the window's
thread): the ticks of that time are lost, as with any stall that long. With the window in view the latency
figures are the same with the timer and without (§5: 119.5 to 120.0 frames a second, a key to its picture 10.4
ms against 10.6, measured back to back). A link's polling rides the same frames, and was measured between two
apps since (§4, "The window away under a link"). Not tested: the app's own window moved to another desktop
(nothing moves one without the user), and other systems.

Slint on Apple's systems draws its frames from a display link with no window event before them. Its Skia renderer
on Metal never calls `BeforeRendering`, so the clock needs femtovg, on OpenGL here and WebGL on the web; the app
asks for it by name and is built with no other renderer (§14).

**The picture is one texture, written in place** (`gl.rs`). When femtovg's `RenderingSetup` hands over its OpenGL,
the app makes one texture there, the picture's size, and shows it as a borrowed texture
(`BorrowedOpenGLTextureBuilder`), set as the UI's `Shell.picture` once. Each new picture is written into it with
`glTexSubImage2D`, and nothing is allocated for it:

- The presented buffer goes as it is. Its 0x00RRGGBB pixels are B, G, R and an unused byte in memory, which
  OpenGL takes as BGRA, so only the unused byte is made opaque.
- The texture is made again only when the picture's size changes.

Without that OpenGL (`NETTAI_NO_TEXTURE`, the web), each picture is a new `SharedPixelBuffer` image. That was the
first prototype's only way, and it was the lag the user saw (§5).

There is one texture and it holds one picture: the battle's, the main menu's reel's, a replay's viewer's, or the
still of the replay selected in the list. Each
is a `Surface` (`stage.rs`: the buffer something presents itself into, how it fits, whether it is out of date),
and the one last shown is the one in the texture; another shown after it presents its own picture anew.

**Where the picture goes** is a `Picture` in the screen's Slint (shell.slint): a box that says where it is and
how large (`Shell.picture-room-*`). The app draws the picture in the box's middle, at the frame's largest whole
multiple that fits, counted in the display's pixels (`stage::fit`, held to the settings' picture size), and moved
onto the display's pixels: the layout is scaled by a fraction (§9), so the box's corner needn't fall on one, and
a picture drawn across them wouldn't show each of the game's pixels as one square. The box keeps its size
whatever the multiple, so nothing beside it moves. In a battle the box is the window; on the main menu it is the
reel's 720 by 480.

It is presented at the window's density (the scale over the display's factor, rounded up), or at the display's
density with `NETTAI_PHYSICAL_PIXELS`, and drawn with `image-rendering: pixelated` at exactly `240 × scale`
display pixels. The density need not divide the scale. Each display pixel samples the presented pixel its center
falls in, and that pixel always belongs to the right frame pixel, so every frame pixel is the same square
(`stage::tests::every_frame_pixel_is_a_square_at_any_density`). Only the text layer, drawn at the density, is
scaled by a fraction.

At 1280x800 points on a Retina display, the battle's picture is 10x (2400 by 1600 of the display's pixels),
presented at 5x: 1200x800 pixels, presented and written into the texture in about 3.5 ms in a dev build (§5).
Presented at the display's density, a picture cost the first app 6.4 ms and held it to 60 frames a second.

Nothing else is drawn under a battle that fills the window: the ground and its hexagons are left out (they cost
1.4 ms a frame there: §5).

**The keys** come from winit's window events through Slint's winit hook (`on_winit_window_event`), before Slint
sees them. They are read by physical key (Z and X sit where they are whatever the layout) and timestamped as they
arrive. While the battle has the keys, they don't reach the menus. The battle's buttons are the player's bindings
(Settings' Controls, kept in the settings; `input::Bindings`). The defaults:

- the arrows move; Z is A, X is B, A is L, S is R; Enter is START, Backspace is SELECT;
- Esc opens nettai's menu;
- in training, R (or F5) starts the set over, unless the player bound it to a button;
- in an online match, Tab held shows both builds (§4): no button can be bound to it.

**The gamepads** are read through [gamepad-facade](https://github.com/tangobattle/gamepad-facade) (SDL3's
gamepads behind a small API; the user's choice) and polled each frame (`input::Pad`). Every pad plays the one
player:

- the buttons are bound as the keys are; the defaults are the south button A, the west button B, the shoulders L
  and R, Start and Back;
- the left stick moves whatever is bound;
- the Guide button opens nettai's menu, and in training the north button starts over.

In the menus the same presses navigate (§11), and the key hints switch to the pad's buttons once it is used.

**A terminate request closes the app** (`quit.rs`: the system's SIGTERM, and SIGINT, a terminal's Ctrl+C). It
did nothing before: the gamepads' library starts SDL, which takes both signals and turns each into an event of
its own queue, and the library hands on a pad's events alone, so only a kill ended the app. The app takes the
two signals itself once SDL has started (the last to take them has them): its handler writes a byte to a pipe,
and a thread waiting on the pipe ends the window's event loop on the window's thread, so `main` returns as it
does when the window's close button is pressed, and what the app holds is dropped in order (a recording's file
flushed, a link closed; the settings are written as each changes). A second request while it closes is the
system's own again. Run on macOS: the app ended with status 0 in 0.3 to 0.4 seconds, from the main menu and
from a training battle, on SIGTERM and on SIGINT. Windows has no such signals and nothing is taken there.

**START never pauses the game.** In a fight the library reports START and doesn't act on it
(`Player::take_pause_requests`: docs/frontend.md §2), and what it opens is nettai's menu (`Shell::open_menu`):
the battle's owner's own, the training menu or the match menu (§2). A battle that can stop is paused under the
menu; an online match keeps running, as the menu says.

**The sound** is nettai-audio's `Output`, fed one stream at the battle's rate (`sound.rs`):

- The menus' sounds are the selected game's own custom screen sounds, through its rules' sound roles
  (`custom_cursor`, `custom_pick`, `custom_back`, `refused`): the app names no sound of a pack. Every move of the
  cursor, confirm, back and refusal has one.
- The title music is the selected game's too: the sound its pack names `title-screen`, on a driver of its own. It
  plays across the menu screens, fades in, fades out as a battle starts, and crossfades with the next game's as
  the game changes. Each game's content names its own (`content/<game>/compat/assets.toml`): the song its start
  screen's first state sets, song 1 in EXE6 and EXE5 and song 0 in EXE4, read from the routine in all twelve
  ROMs. A pack that names no such sound has none: one extracted before the names were given, until it is
  extracted again.
- While a battle runs, both are ticked with the battle's ticks and their samples added to the battle's, sample
  for sample; with no battle running they run on their own clock at the same rate. (At first the menus' sound ran
  on its own clock throughout, beside the battle's, at twice the rate the device plays. The queue then sat at
  `Output`'s cap of 200 ms: every sound was late by that much, and dropped samples skipped: §5.)
- Settings' Music is the level of the title music and of the battle's music; Sound effects is the level of the
  menus' sounds and of the battle's effects. The battle's sound is one stream, mixed by the game's own driver
  (its Direct Sound channels into one eight-bit buffer, its four PSG channels shared), so the two are set apart
  inside the driver: it mixes each note at a level the host gives its music player (`m4a::Driver::set_level`;
  what sounds, and what is cut for what, stays the game's), and the music is one player's. The app gives a
  battle the two levels against the louder of them (`Sound::battle_levels`, `Player::set_sound_levels`; a
  replay's viewer takes them the same way and keeps them across seeks) and plays its samples at the louder, so
  only the difference between the two is turned down in eight bits. At the full level the driver's output is
  unchanged to the sample. Measured in a training battle of each game: with Music at 0 the stream is silent
  once the opening effects end; at 50 the music's peak is half (0.30 to 0.39 against 0.6 to 0.7).

**A game is ready in two stretches**, and the loading screen's bar covers both (`games.rs`; docs/frontend.md §9):

1. Everything read from disk (the content, the pack's graphics and sound, the text's font, and the strings and
   lettering of the language shown) is read on a thread of the game's own, a part at a time (the library's
   `game::Loading`), so the window stays live. This is four fifths of the bar.
2. The content's Luau runtime is loaded on the window's thread, the one that runs the battles and checks the
   builds (`preload::runtime`): about a second with the window still. It is done the moment the selected game's
   parts are in, two frames after the bar has shown them, and never later: a link nobody polls for three seconds
   reads as dropped, so it must be over before any lobby exists.

The game last used loads before the menu appears. The others' parts are read behind the menu; a game takes its
runtime when the player steps to it, behind the bar again.

**A game whose pack is there and can't be loaded** (extracted for another version of the game's content: the
content names a sprite the pack hasn't) is passed over, and the player is told once, by the game's name, in a
dialog of the shell's own: "BN6's data is out of date. Extract it again." (`UserError::GameOutdated`; what the
loader said is in the log). The other games load and play; if it was the game the app opened on, the next one
is. It is told as its load fails, at start-up, but never over another dialog or a battle: then once that is
gone (`Games::take_failed`, asked each frame by `App::follow_games`). A game with no pack at all is passed over
without a word, as ever. Another language's graphics are loaded on a thread too
(`Games::load_language`) when the language changes, never on the window's thread where they are first asked for;
the screens and the main menu's reel are shown again in the language when they arrive.

**Before a battle** the two cards show while the match is made ready (`battle.rs`, the library's
`preload::Preload`): the round's first battle, its first picture at the size it will be shown, the sound's
drivers, and each side's navi, forms and chips. Once a game is loaded this is a few milliseconds, so the bars
fill at once; they show the truth, and the cards stay up half a second so they can be read. A battle made ready
loads no runtime and reads nothing while it plays, and the app holds itself to that: when a battle ends it asks
the library what was loaded late (`Player::late`) and writes any runtime or read to the log.

**What is missing.** The app looks the title music up by its asset name, `title-screen`, where the menus'
sounds go through the rules' sound roles: a role for it would be the content's to add.

## 2. The shell and the screens

Two crates. **nettai-ui** is the Slint alone: `ui/` compiled by `slint-build`, the fonts and the translations
bundled. **nettai** is everything else. They are apart so that an edit to the app's Rust never compiles the UI
again, and so that the UI can be built without optimization (§14).

    crates/nettai-ui/ui/theme.slint       the colors, the type, the metrics, the durations (§10)
                        motion.slint      what every animation is made of
                        nav.slint         a focusable control; what a control tells the app (§11)
                        widgets.slint     the components
                        shell.slint       the header, the footer, a page, the ground, the cursor, dialogs
                        app.slint         the window: the one screen shown, what opens over it
                        screens/<name>.slint
               lang/<code>/<part>.po      the translations (§3)
    crates/nettai/src/main.rs             the window, its clock, its events
                      shell.rs            `Shell` (what a screen is handed) and `App` (who carries what happens)
                      nav.rs input.rs     the focus; the keys and the pads
                      battle.rs stage.rs gl.rs sound.rs games.rs      the battle's host (§1)
                      awake.rs                                          the app's time kept (§1)
                      settings.rs lang.rs paths.rs errors.rs tour.rs  §9, §3, §12, §13
                      tour/<name>.rs                                    a screen's steps of the screenshot pass
                      replays.rs                                        the replays folder, read on a thread (§16)
                      builds/                                           what any screen shows of a build (§8)
                      screens/<name>.rs
                      screens/play/session.rs tour.rs                 Play's link and lobby; its part of §13

**A screen is one Slint file and one Rust module** (which may have modules of its own under it:
`screens/replays/`). Menu, Settings, Training, Play, Builds and Replays are built. `ui/screens/play.slint`
holds:

- `PlayView`, a global: the screen's data, which its Rust module sets (and may read back). A struct or an enum
  one of its properties uses reaches Rust by that use; nothing is added to app.slint for it.
- `PlayScreen`, a component that `inherits Page` (shell.slint): the header with its title and where Back goes,
  the footer's key hints, and the screen's own parts in the body.
- `PlayOverlay`: what the screen opens over itself (its dialogs, on a phone its sheets), over the whole window.
  (A screen that opens nothing has none, and no `if` for one in app.slint: Training.)
- For Training and Play, the menu over their battle (`TrainingMenu`, `MatchMenu`, each a `MenuOverlay`); for
  Play also `MatchPeek`, what shows over an online match with no menu open.

One thing to know in all of them: a part placed in a `Page`, a `MenuOverlay` or a `DialogFrame` is laid out in
the area the component gives it, but `parent` there is Slint's lexical parent, the whole component. Its parts
measure from what the component says instead: a page's `body-width` and `body-height`, a menu's `inner-width`
and `inner-height`, a dialog's `inner-width` (`width: root.body-width`, not `parent.width`).

`src/screens/play.rs` holds a struct that implements `Screen` (screens/mod.rs). A screen never handles keys and
wires no callbacks. Its controls have names (`nav: "ready"`, and `nav-index` where there are several), and
whatever the player used, a key, a pad or a click, arrives as one of a few calls with the name:

| `Screen` | When |
|---|---|
| `show` | The screen is shown, or something it shows changed under it (the selected game, the language, the window turned upright): set the view from the state. The one method a screen must write. |
| `hide` | Another screen is shown in its place. |
| `activate(id, index)` | Enter, a click or a tap on a control. |
| `step(id, index, by)` | Left or Right on a stepper, a select or a switch, or a click on its arrow. |
| `choose(id, index, choice)` | A choice made outright: one side of a switch clicked, a slider set, a row of a select's list. |
| `edit(id, index, text, done)` | A field's text as it is typed, then as it is left. The screen keeps what it wants of it and sets its view's property, which the field then shows. |
| `focused(id, index)` | The focus moved to a control (a list's row: the overview beside it follows). |
| `key`, `button` | A key or a pad's button the screen may want for itself, before the menus read it (a key being bound; R in training). |
| `action` | Previous and next (the shoulders, the keys bound to L and R), delete, the pad's north button. The shell changes the game with the first two unless the screen takes them. |
| `scroll` | Up, Down, Page Up, Page Down, Home, End (keys, D-pad, stick), before the focus moves: for a page of the screen's own that scrolls with no control in it to move to (Play's build sheets in the match's menu). |
| `back` | Esc or Back: true if the screen went back within itself; false and the shell goes to the menu. |
| `answered(tag, choice)` | A dialog the screen asked was answered; or something it asked of another screen was (`Shell::tell`: a row it added to the training menu was chosen). |
| `dropped(path)` | A file was dropped on the window. |
| `battle(event)` | A battle the screen started began, ended (with its result) or stopped (with its reason); or the replay it asked for is ready to watch. |
| `frame`, `frame_end` | Every frame, shown or not: before the battle's ticks, and after them. |

**What a screen asks of the shell** (`Shell`, shell.rs):

- *Navigation:* `go(screen)`; `refresh()` to be shown again; `nav.focus(id, index)`; `nav.enter(view, main)` for
  a view of its own that replaces the rest (a lobby, an editor's tab: the focus is remembered by the view's
  name, and the screen calls it again in `show`); `nav.open(prefix, key)` and `nav.close(prefix)` around a dialog
  of its own, whose controls' names begin with the prefix (while it is open the keys reach nothing else).
- *The selected game:* `game()`, `ready()` (the game loaded: its content, graphics and sound), `lock_game(true)`
  where the game can't change (the name stays in the header without its arrows), `select_game`, `step_game`.
- *Dialogs:* `ask(Dialog)` for a question (a title, a message, buttons; the answer comes to `answered`);
  `pick(id, index, options, chosen)` to open a select's choices as a list; and for a dialog with controls of its
  own, a `DialogFrame` in the screen's overlay.
- *Errors:* `fail(UserError)` for a failure that ends what the player was doing, `fail_with` for one with the
  screen's own button; `UserError::line` for a field's reserved line (§12).
- *A battle:* `start_battle(BattleStart)`: who owns it, which menu Start opens, the set and what plays it, the
  two cards, whether it fills the window, and for an online match a `gate` that is told this side's fraction
  each frame and answers the other side's (the lobby's `set_loading` and `their_loading`: the battle starts when
  both are 1), with `Play::Make` for a driver that can be made only as the battle begins (the match's, on the
  channel its lobby gives once both sides have loaded: §4). Then `open_menu`, `close_menu`, `end_battle`, and
  `battle` for the player itself
  (`battle.stage.player`: the dummy's choices) and `battle.restart()` (a battle that can stop, from its start:
  it runs again, and its owner will hear of its next end). `loading_of(owner)` says whose battle the cards are
  making ready. The shell owns no link and no lobby, and decides nothing when a battle ends: the owner is told
  the result or the reason and says what is next.
- *Another screen:* `tell(screen, tag, choice)` has that screen hear `answered(tag, choice)` once the caller
  has returned (how Training tells the screen that added a row to its menu).
- *A replay:* `start_battle` with `Play::Watch(replay, side)` makes its viewer ready behind the same cards and
  hands it over (`BattleEvent::Watching`); the screen shows it in a `Picture` of its own with
  `show_surface(surface, present)` and plays its sound with `play_samples`, having given the viewer the Music
  and Sound effects levels (`viewer.set_sound_levels` with `sound.battle_levels()`: §1, "The sound").
- *The rest:* `settings` (and `settings.remembered`, a map for what a screen remembers by its own keys, so it
  adds no field) with `save_settings()`; `set_name`; `lang`; `sound.play(Cue)`; `copy(text)`;
  `choose_file`, `choose_save`; `recorder` for an offline set's replay; `fly` (§10); `stay_awake`, set while
  the screen has a link open: the app isn't napped, and its frames run by a timer when the window isn't drawn
  (§1).

**A training session, from any screen** (`crate::screens::training`, §15). A training session is a battle
against a dummy with the training menu behind Start and Esc; Training starts one from its entry, and another
screen may start one beside something of its own (Play, while an invite waits for its opponent). Three calls:

    use crate::screens::training::{self, Build, Start};

    // Start one. `Start::new(game, you, dummy)` is Training's own (the two cards, the picture filling the
    // window); `Start::beside(game, build, from, tag, rows)` is one a screen shows in a `Picture` of its own:
    // the dummy a copy of the player's build, standing still, no cards, and `rows` at the top of the training
    // menu. The fields are public: `opponent`, `seed`, `full`, `cards`.
    let rows = vec![copy_invite_code, cancel_invite];           // their texts, in the language shown
    training::start(shell, Start::beside(&game, Build { name, side }, ScreenId::Play, "invite", rows))?;

    // A row it added was chosen: the screen hears it as a dialog's answer, by its tag and the row's place.
    fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) { /* "invite": 0 copy, 1 cancel */ }

    // End it from outside (the opponent joined). `training::running(shell)` says whether one is on.
    training::end(shell);

- `start` gives `Err(UserError)` if the game isn't ready or a build can't be played (the detail is in the log);
  both builds must be ready (`nettai_match::build::Checked::ready`). A session already on ends first.
- The session's battle is always Training's (`ScreenId::Training` owns it), so Training's module restarts a set
  that ended, hears the menu (`menu.*`) and the restart key whatever screen is shown, and the starter writes
  none of that. The starter's own controls stay its own: a control is heard by the battle's owner only if it is
  the battle's (its name begins `menu.`) or the battle fills the window (`App::hearer_of`).
- A session started from outside has no CHANGE BUILDS and no QUIT TO MENU: its starter's rows are the ways out.
  Choosing an added row tells the starter and leaves the menu as it is (the starter closes it with
  `shell.close_menu()` if it wants, or ends the session). It keeps nothing of the dummy's setup.
- One shown in the starter's own `Picture` (`full` off) lasts while that screen is shown: it ends by itself if
  another screen is gone to, and when another battle starts in its place. The keys and a gamepad are the
  battle's there as anywhere; the phone's pad is drawn by the battle's own screen, which only a battle that
  fills the window has, so on a phone such a session has no pad unless its screen fills the window with it.
- A dialog asked while it runs (`Shell::ask`) is answered to the battle's owner, as ever: Training passes the
  answer on to the starter (`answered`, by the dialog's tag).

**Filling in a screen** touches three files: `ui/screens/<name>.slint`, `src/screens/<name>.rs` and
`lang/ja/<name>.po`. **Adding a screen** is those three and a few lines in three shared files: the `Screen` enum
(shell.slint); its import, its view's export and its two `if`s (app.slint); its module, `ScreenId` and `all()`
(screens/mod.rs). Its steps in the screenshot pass go in `tour::walk` (§13).

## 3. Languages

**Every string of the app's own goes through Slint's `@tr`**, written in English, with a context where one word
means two things. A string Rust needs (a dialog's button, a card's role) is a property or a function of the
screen's view in the same catalog (`out property <string> you: @tr("YOU");`). No string from a library's error is
ever shown (§12).

**A language is a folder**, `crates/nettai-ui/lang/<code>/`, of gettext catalogs: one file for each part of the
app (`shell.po` for what every screen shares, then one a screen), so two screens' translators don't meet in one
file. `build.rs` merges a language's files into the one catalog Slint bundles into the binary, and says at build
time which strings a language has no translation for and which of its translations no string asks for any more.
Each language names itself: its translation of `msgctxt "language"` / `msgid "English"`, which Settings lists.

It also makes a **pseudo-locale**, `pseudo`: every `@tr` string found in the .slint files, lengthened (each
vowel doubled, the whole bracketed: about 40% longer) and its letters accented. A layout that can't take a longer
text shows it, and so does a string that doesn't go through `@tr`. It is made from the sources, so no string is
missing from it, and no template has to be kept in step. It is a testing device, and no player's: the
screenshot pass walks it, `NETTAI_LANG=pseudo` shows it, and Settings' list offers it only while it is the
language shown; a settings file that names it doesn't bring it back (`lang::offered`, `lang::initial`).

The first language is the settings', else the system's (`sys-locale`, by its primary subtag), else English;
`$NETTAI_LANG` overrides it. Settings changes it while the app runs: the app's strings switch at once
(`slint::select_bundled_translation`), and the content's names and the battle's own drawing switch once each
game's graphics in the language are loaded, on a thread (§1). A language the content has no table for shows the
content's own names.

**A game's name is its own, in the language shown**: BN4, BN5 and BN6 in English, EXE4, EXE5 and EXE6 in
Japanese (docs/design/app-ui.md §2.22). Each game states it in its locales, in the text table `game` its manifest
declares (`"name" = "BN6"`), and `nettai_content::locale::game_name(dir, game, lang)` reads it from the content
alone (a language with no table, the pseudo-locale, has the game's own language's), so a game with no pack, or
one whose pack fails to load, is named too. The shell reads every game's at start-up and when the language
changes (`Shell::name_games`), and `Shell::game_name(id)` gives it to whoever names a game: the header's
selector, the loading screen, "Not a BN6 save", "This navi isn't BN6's", a lobby's or a replay's other game, a
pack that is out of date. The header's name changes with the language at once. A game whose locales can't be
read at all is shown by its id in capitals (EXE6); Rust and Slint name no game. `locale::tests::
each_game_is_named_in_its_languages` holds the three games to their names in both languages.

**Adding a language** is adding its folder: copy `lang/ja/` to `lang/<code>/`, translate each `msgstr`, and
build; the build lists what is left. `msgfmt --check -o /dev/null` validates a file.

**Typography.** Headings, buttons and labels are Chakra Petch, which has Latin alone; a language set in another
script (`lang::cjk`: Chinese, Japanese, Korean) has them in Murecho, with the labels' tracking nearly none.
Murecho covers Latin and Japanese. For a script it lacks, Slint falls back to the system's fonts on the desktop;
the web has none and a phone's may lack one, so such a language needs its font bundled (imported in theme.slint
beside the others).

## 4. Netplay

Play is the netplay UI (docs/design/app-ui.md §6: "Play", "In an online match", "After a match"):
`ui/screens/play.slint` and `src/screens/play.rs`, with the link and the lobby in `screens/play/session.rs`.

**The screen has three parts**, one shown at a time (`PlayView.part`), and two things over a match:

- **The builds.** The player's builds of the selected game as a list, as the Builds screen lists them
  (`builds::shelf::load`, its `BuildRow`s: each read from its file and checked by the game's rules; one that
  can't be played says how many problems it has, in the error red), the chosen one's overview beside it (the
  wide sheet, §8), and one row under them: CREATE INVITE, the code's field with JOIN
  and a space for a message, DIRECT CONNECT. The chosen build is the one under the list's cursor, remembered by
  game. With no build of the game there is one button in the list's place, which goes to Builds, and the row
  is unavailable. The field keeps what is typed as a code: capitals, a code's six characters, shown in threes,
  whatever it was typed with (`code_typed`, `code_shown`). A pad can't type: its confirm on the field opens the
  code as six boxes (docs/design/app-ui.md §3), where Left and Right go between the boxes, Up and Down through
  a box's characters (the alphabet codes are written in), and a character typed goes into the box and on to
  the next. (A box is a list to the app's focus, its rows all in one place: `CodeBox`, `Nav::set_row`.) DIRECT
  CONNECT opens a dialog (a `DialogFrame` in `PlayOverlay`, its scope `direct.`): this machine's address with
  COPY, and HOST; an address's field, and CONNECT.
- **Waiting** for the other player: the invite's code (a direct host's address), large, in fixed-width type,
  with COPY CODE and CANCEL, and a training session beside it in a `Picture` of the screen's own. The session
  is Training's (`training::start` with `Start::beside`, §2: the player's build against a copy of it that
  stands still; Training owns its battle, starts its sets over and hears its menu), with Play's rows at the
  top of the training menu: COPY INVITE CODE (COPY ADDRESS for a direct host), which copies and closes the
  menu, and CANCEL INVITE (STOP HOSTING), each heard as `Screen::answered` by the tag Play gave. The keys are
  the battle's, so the cursor is on none of the panel's buttons (the pointer clicks them), and Esc or Start
  opens the training menu. The session ends when the other player comes (`training::end`) or the wait is
  given up.
  *On a phone* a battle has its pad only where it fills the window (§2), and a battle to watch is no training:
  so there the session isn't started with the wait. The invite shows with a TRAINING button under it; the
  session then fills the window, pad and MENU under the picture as in any battle, and its menu has a third
  row of Play's, STOP TRAINING, which goes back to the invite. The other player coming ends it for the lobby
  as on the desktop.
- **The lobby**, for as long as the two players stay together: the two cards (this player's says their build;
  the other's says their name and whether they are ready, and nothing else: the lobby's protocol carries
  nothing of a build), one row of settings (Rounds, Patch cards and Closed builds are the match's: either
  player changes them, both see it, and both ready marks clear; Input delay is the player's own, and is the
  Settings' value, kept as it is changed) with the connection's bars and round trip at its end, READY, CHANGE
  BUILD (a list of the builds, in a dialog; nothing of the new one is said to the other player) and LEAVE, and
  a line that is there whether or not it says anything: who is waited for, "Reconnecting N s" while the link is
  down, or why the match before ended. In the row each setting's name is over its control (`LobbyFacts`): with
  the names beside them, as the mockup has three, five cells don't fit the window in a language a third longer
  than English. On a phone each is a line, its name at the left. Left and Right change a setting, so the row
  can't be walked with them: Tab does, and in the lobby so do the shoulders and the keys bound to L and R
  (`Screen::action`). Patch cards is the lobby's own setting (`Lobby::set_patch_cards`, on at first; off,
  neither player's cards apply and each build plays without them: docs/frontend.md §2). The lobby checks this
  player's build as the settings would play it whenever either changes (`Lobby::problems`, typed); while it
  has a problem READY is refused (`Lobby::set_ready` says so, and the mark is the lobby's answer) and the line
  says what is wrong with it, in the error red. So CHANGE BUILD takes any build whose
  file reads, and the lobby is what says one can't be played.
- **The match menu** (`MatchMenu`: Start or Esc in a match, which goes on behind it): BACK TO MATCH, YOUR BUILD,
  THEIR BUILD (not with closed builds), CONTROLS (what is bound to each button, to read), SETTINGS (the input
  delay, which changes at once, `Player::set_input_delay`; the two levels of the sound), LEAVE MATCH.
- **Both builds** (`MatchPeek`, which app.slint shows over an online match): while Tab is held (`Keys::tab`), or
  the pad's button bound to SELECT, the player's build on their side and the other player's on theirs; with
  closed builds the player's own alone. The match takes its keys meanwhile.

Where a build is shown whole, it is the Builds screen's overview (`BuildOverview`, filled by
`builds::overview::sheet`, §8): beside the list, the wide sheet of the build as it is (made again only when
the build chosen or the language changes); a player's panel in a match, on its YOUR BUILD and THEIR BUILD
pages and while Tab is held, the narrow sheet of the agreement's side, which is the build as the match plays
it (without its patch cards where those were off), under the player's name. The menu's page is shorter than
the sheet at the reference size, so it scrolls: by the wheel and a drag, and by Up and Down (a step of two
chips' rows), Page Up and Down, Home and End, the pad's too, which the screen takes before the focus would
move (`Screen::scroll`: the page has no control in it to move to, Back is under it). On a phone a sheet is a column taller than the window: the menu's page scrolls it, and with Tab held each
of the two is half the window, its top (the navi, its stats, abilities and bugs). This player's card in the
lobby says the build's name, navi and version as the settings would play it (`Lobby::side_as_played`).

**The link and the lobby** are one `Session` (`session.rs`), and the screen runs the library's one loop over it
(docs/design/rollback.md §4.7, "What a host does") in its two frame halves, whatever is on screen:

- `Screen::frame`, before the battle's ticks: `Session::feed` gives every datagram the link has to the lobby, a
  match's frames too, and tells the lobby how the link is. Before the link is open, it keeps the link up.
- `Screen::frame_end`, after them: this side's loading is said (`set_loading`, `set_loaded`); `Session::poll`
  says how the lobby stands, which moves the screen on; `Session::send` sends what the lobby has to say and the
  frame the match's player made this frame.

A link is made by `Session::open`: an invite (`Link::create_room`: the server picks the code, and the wait
begins when it has said it), a code typed (`Link::join_room`), a direct host (`Link::host`, UDP port 47474) or
a direct host's address (`Link::join`). Making a room's link resolves the STUN server's name, and joining an
address resolves the host's: either may wait on the system's resolver, so the link is made on a thread of its
own (`Session::open` says at once only what needs no lookup: no server named, a code that can't be one, a port
that can't be listened on). Meanwhile the space beside JOIN, or the dialog's under CONNECT, says the link is
being made, and the loop goes on; a host's name that doesn't resolve is that address's error, and a
signaling server that can't be reached is the server's. A STUN server whose name doesn't resolve is left out,
as the link library has it (a link on one network needs none), not a failure. The lobby is made the moment the link is open, in the link's role, with
a timeout of 45 seconds (longer than the link's 30 of making a dropped connection again), and the training
battle ends. From the moment a link is asked for until it is dropped (the wait, the lobby, the loading, a
match and its result) the screen holds `Shell::stay_awake`: the app is kept awake, and the timer runs its
frames while the window isn't drawn (§1), so the link is fed and polled with the window minimized or hidden.
Nothing in the two halves needs a frame drawn. And nothing is read or checked on the window's thread while
there is a link: the builds are listed before, and a match is made ready a few milliseconds a frame.

**A match.** When the lobby says a match is agreed (`Status::Loading`), Play starts the battle
(`Shell::start_battle`): the two cards, this player's with their build and the other's with their name alone;
the agreement's set, for this player's side; and a `gate`, through which the shell says how far this machine's
preload is each frame and hears how far the other is (`Lobby::their_loading`). The match's driver can't be made
yet: the lobby gives the match's channel only once both sides have loaded. So the battle's `play` is
`Play::Make`, a driver made as the battle begins: when the lobby says `Status::Start`, Play takes
`Lobby::channel`, makes the `NetPlayer` on it with the player's input delay, and lets the gate through. If the
other player takes their ready back in the moment the match was agreed, the lobby says so and the cards go.
The match is recorded from its first tick by its agreement (`Recorder::of_agreement`: an online match, from
this player's side, by the names the lobby had, closed builds where they were agreed), to a new file in the
replays folder.

**After a match** both players are back in the lobby on the same link (`Play::reopen`: the battle ended, which
drops the match's player, then `Lobby::reopen`), nobody ready, the settings as they were:

- *Its set is over.* The game shows the result itself, in its own way, before the set is over for the engine
  (the winner's banner, then the picture fades out); half a second after that, the lobby.
- *LEAVE MATCH*, or the other player leaving it (`StopReason::OpponentLeft`, or the other's lobby heard again
  while this player's set isn't over, `Lobby::their_reopened`), or the other's player going silent
  (`ConnectionLost`), or something internal: the lobby's line says which, as its kind's sentence.
- The lobby's LEAVE and another screen leave the other player (`Lobby::leave`, then the link is dropped): their
  lobby ends, told. Esc or Back in the lobby asks first ("Leave the lobby?", CANCEL focused, as a deletion
  asks): a key pressed by mistake shouldn't end the link for both.

**What a player is told** when it goes wrong is the typed kind's sentence (§12), where it belongs:

- *Before the two have met*, the problem is the control's that asked: in the space beside JOIN (the server
  can't be reached, no match has that code, the match is full, the build can't be used), or in the direct
  connection's dialog (that isn't an address, the port is taken, no connection could be made). While a link is
  being made the same space says so.
- *In the lobby*, a build the settings can't play is the lobby's line (above): it doesn't end the lobby.
- *Once they have met*, a link that gives up (`Link::recv`'s error) or a lobby that ends (`Status::Failed`:
  another version, another game, the other player left, no answer; a build refused only where the other
  program revealed one anyway) ends what the player was doing: the shell's dialog, with BACK TO PLAY, over
  the builds again.

**What it adds to the shell**, all small: `battle::Play::Make` (above); `Keys::tab`; `Nav::set_row` (the row
the focus comes to a list at, where that is the screen's to say); `Button`'s `natural-width` (for a button
given a width only on a phone); and the screenshot pass's hooks (§13). And three things of the shell's that a
link between two players showed wrong, mended: the corner's box in a match is wide enough for "Reconnecting 30
s" in the language shown (it was cut to "Reconnecting 2…"); a dialog's title takes the lines it needs (a
failure's title is a sentence, and the pass found "Your opponent left" cut on a phone in the pseudo-locale;
on the desktop "Your opponent's build can't be used" just fits its one line); `Message`'s text is as wide as
its line (it sat in the line's middle whatever its alignment).

**What is wrong with a build** is said in the Builds screen's words for its kind (`builds::words`): beside JOIN
before a link (`words::first_problem`, as `UserError::BuildUnusable`'s reason), and in the lobby's line
(`words::problem` of the first of `Lobby::problems`, which names the part of the build it is about): "This build can't be used: 28 chips, a folder has 30."

**Not done.** CONTROLS in the match menu shows the bindings and doesn't change them.

**What has run, and what was only written** (2026-10-12). Between two players in one process over real links
(the screenshot pass's second player, §13), in English, Japanese and the pseudo-locale at both sizes:

- an invite made, and joined by its code; a code no match has, refused by the server; a direct host joined at
  its address; the other player's invite joined by its code, typed in small letters;
- the wait's training session: started beside the invite (on a phone, by its button, filling the window), its
  menu's COPY INVITE CODE, STOP TRAINING (a phone) and CANCEL INVITE each chosen, and the session ended by the
  other player coming;
- the lobby's rounds, patch cards and closed builds changed by one player and taken by the other; the input
  delay changed; another build chosen in the lobby that the settings can't play (READY refused, the line
  saying why), and the first again;
- the link dropped for six seconds in the lobby and in a match, and made again;
- a match agreed, loaded with the other side held at 74%, and played to its end (the game's own "WIN" banner,
  then the lobby again, nobody ready); a second match on the same link with patch cards off and closed builds,
  left by the other player; a match as the joiner, on the right navi's console, left by LEAVE MATCH;
- the other player bringing a build that breaks a rule (its lobby won't make it ready, and this player waits,
  ready); the other player leaving the lobby; this player leaving it.

The three matches of a pass, recorded by their agreements (a set played to its end, a match with patch cards
off the other player left, a match this one left as the joiner), were played back by nettai-tool
(`--replay`): each reproduces. A
recording holds the two players' names as the lobby had them, and no build's. Only written, never run: a pad (its SELECT held, its shoulders in the lobby; the code's boxes were drawn and walked
by the pass, opened as a pad's confirm opens them); the signaling server's other refusals (a full room,
another version); a lobby's other failures (another version, game or content, this player's build refused, no
answer); a link that gives up for good (30 seconds down); a match whose player goes silent for good; two
machines (two apps on one machine have met: below).

**The window away under a link** (2026-10-12, macOS; two apps on one machine with a direct link between them,
§13's two-app check; load average about 140). Each met the other through Play's own controls (one hosting, one
typing its address), they readied up and played, the host left the match and then the lobby, and the other
saw each. The host's window was minimized for eight seconds and then hidden for eight, in the lobby and again
in the match, and the other app wrote what its link saw, a second at a time:

| The host's window | What the other player's app saw of it |
|---|---|
| in the lobby, minimized | the lobby's messages 100 ms apart throughout (the longest silence 116 ms), but for 0.62 s as the window went and 0.59 s as it came back |
| in the lobby, hidden | the longest silence 139 ms |
| in a match, minimized | 60 or 61 ticks a second and no silence over 34 ms, but for 0.61 s as it went (44 ticks that second) and 0.55 s as it came back (47) |
| in a match, hidden | 59 to 61 ticks a second, no silence over 50 ms |

The link was never down, for either. The host ran 60 or 61 frames a second by the timer while its window was
away, none of them drawn. The half second as a window minimizes and comes back is the system's animation on
the window's thread (§1); hiding has none. The same run with the host's timer left out
(`NETTAI_NO_TIMER_CLOCK`) is what the check is for: the other app heard nothing for 8.1 to 8.8 seconds each
time, its link read as down from the third second, and its match stood still (0 ticks for six or seven
seconds); the link and the match both picked up when the window came back.

The signaling server isn't deployed (with none named, CREATE INVITE and JOIN say the server can't be reached,
and DIRECT CONNECT works). Run it locally:

    cd signaling && npx wrangler dev        # http://127.0.0.1:8787
    NETTAI_SIGNAL=ws://127.0.0.1:8787 cargo run --release -p nettai

A room's link finds its way through NATs with nettai-rtc's default STUN server and checks the certificates its
descriptions name; a direct link authenticates nobody, as plain UDP doesn't. Over the Internet, a direct host
forwards UDP port 47474 on its router to its machine. No TURN server can be given, so two players whose NATs
need a relay can't meet (nettai-rtc's `Config` takes one in its `ice_servers`).

## 5. Latency and smoothness

Measured as nettai-demo measured (docs/frontend.md §7, its lessons). `NETTAI_KEY_PROBE` presses the right arrow
every 300 to 400 ms, through the event loop as a key's event arrives, at no particular point between frames.
`NETTAI_PLAY_STATS` prints, every two seconds:

- frames a second, and the frame time's median, 95th and 99th percentiles and worst ("long": over half again the
  median, a refresh missed);
- ticks a second (the original's 59.73);
- each picture's cost (presented and written), and how long after it the frame was drawn;
- for each press, the time to the tick that saw it and to that tick's picture drawn;
- the most sound queued for the device (the sound's latency);
- each second, how much of its level the title music is at, its samples' peak, and the whole stream's (what no
  screenshot shows: on EXE6's main menu the music is at 1.00 with a peak of 0.76 to 0.88 and the stream 0.8 of
  that, the Music setting's 80; in a battle the music is at 0.00 and the stream is the battle's).

The figures below are the first app's, of 2026-10-06, on this Mac (120 Hz display), with a 1280x800-point window
(a 9x picture presented at 5x), while other builds loaded the machine (load average 12 to 56). Each is the range
over 2-second windows. The redesigned app hosts the battle the same way (the same clock, texture and key path;
`stage.rs` and `gl.rs` are as they were but for the picture's state now being a `Surface`), and its own figures
are at this section's end.

**The user found the first prototype laggy.** There were three causes:

- A new image for each picture: a buffer allocated and converted, and a texture made and dropped.
- In the dev profile (opt-level 1), frames with a new picture missed the 120 Hz refresh. The picture's frames came
  unevenly (a missed refresh, then two quick ones), which is stutter.
- The sound queued 200 ms deep (§1, "The sound").

The persistent texture and the single sound stream fixed them:

| nettai, `dev` | before | after |
|---|---|---|
| frames a second | 90 to 101 | 118 to 120 |
| frame time p95 / p99 | 15.7 to 16.6 / 16.7 to 16.8 ms | 8.5 to 9.5 / 8.7 to 12.1 ms |
| long frames in 2 s | 29 to 57 | 0 to 2 |
| ticks a second | 59.2 to 60.0 | 59.4 to 60.0 |
| a picture (presented, written) | 5.0 to 5.3 ms, then 3.1 to 3.4 ms of femtovg's upload to the frame drawn | 3.2 to 3.5 ms, then 0.45 to 0.63 ms to the frame drawn |
| key to its picture drawn (mean) | 11 to 13 ms | 9.3 to 15.4 ms (mean 11.9) |
| sound queued | 201 ms | 51 to 62 ms |

| nettai, `release` | before | after |
|---|---|---|
| frames a second | 101 to 117 | 120.0 |
| frame time p95 / p99 | 8.4 to 14.8 / 8.6 to 21.1 ms | 8.4 to 8.5 / 8.5 to 8.9 ms |
| long frames in 2 s | 5 to 39 | 0 |
| a picture, then to the frame drawn | 2.1 to 2.5 ms, then 2.8 to 3.2 ms | 2.4 to 2.9 ms, then 0.45 to 0.66 ms |
| key to its picture drawn (mean) | 7 to 12 ms | 8.3 to 14.2 ms (mean 11.4) |
| sound queued | (the same 200 ms) | 37 to 74 ms |

nettai-demo, measured the same way on a replay of the same build, at its 960x640 window:

| nettai-demo | `dev` | `release` |
|---|---|---|
| frames a second | 65 to 78 | 78 to 85 |
| pictures a second | 36 to 42 | 41 to 46 |
| key to its picture drawn (mean) | 7.5 to 19.1 ms (13.4) | 7.2 to 26.6 ms (15.4) |

The demo, under the same load, shows fewer than the battle's 60 pictures a second.

The key-to-picture time is mostly the wait for the next tick. The battle ticks 59.73 times a second, so a press at
a random moment waits 8.4 ms on average (8.1 ms measured) before a tick sees it. After the tick come:

- presenting and writing the picture (about 3 ms);
- femtovg's frame (0.5 ms);
- the display's next refresh (4 ms on average at 120 Hz).

The means vary from window to window with where the presses fall between ticks: four or five presses a window.

**The redesigned app** (2026-10-12, the same Mac and window, `NETTAI_TRAINING=exe6` with both variables):

| nettai, `dev` | In a training battle | On the main menu, its reel playing |
|---|---|---|
| frames a second | 119.9 to 120.1 | 120.0 |
| frame time p95 / p99 | 8.37 to 8.59 / 8.44 to 12.48 ms | 8.38 to 8.58 / 8.41 to 11.78 ms |
| long frames in 2 s | 0 to 2 | 0 to 1 |
| ticks a second | 59.5 to 60.0 | 59.3 to 60.0 |
| a picture (presented, written), then to the frame drawn | 3.45 to 4.13 ms (1200x800), then 0.28 to 0.31 ms | 1.5 to 1.6 ms (720x480) |
| key to its picture drawn (mean) | 9.0 to 15.2 ms (12.4) | |
| sound queued | 57 to 58 ms | |

Each is the range over fifteen 2-second windows, with motion on, while other agents' builds loaded the machine
(load average 82 to 105 on its ten cores). The release profile wasn't measured.

Measuring found two things, both the ground's hexagons, a path stroked across the whole window:

- **They were drawn under the battle,** which covers them: a frame was drawn 1.6 to 1.8 ms after its picture, not
  0.3, and 2 to 12 frames in 2 seconds missed their refresh. The ground isn't drawn under a battle that fills
  the window (app.slint).
- **They were stroked anew each frame on the menus,** since they drift: the main menu ran at 107 to 119 frames a
  second, with up to 24 long frames in 2 seconds (on a machine less loaded than for the table: 26 to 53). They
  are drawn once into a layer (`cache-rendering-hint`), and the layer is what moves; with it a frame costs 0.2 ms
  more than with no ground at all.

**With Training built** (§15; 2026-10-12, the same Mac, window and variables; the build before it and the build
with it run one after the other, twice over, 44 seconds each; load average 139 to 162):

| nettai, `dev`, a training battle | Before Training | With it | With it, the input display on |
|---|---|---|---|
| frames a second | 105.5 to 120.0 | 115.1 to 120.0 | 116.3 to 120.1 |
| frame time p95 / p99 | 8.39 to 16.63 / 8.61 to 16.81 ms | 8.44 to 12.51 / 8.99 to 18.16 ms | 8.42 to 10.56 / 8.69 to 13.66 ms |
| long frames in 2 s | 0 to 28 | 0 to 11 | 0 to 6 |
| ticks a second | 59.3 to 60.0 | 59.2 to 60.0 | 59.3 to 60.0 |
| a picture (presented, written), then to the frame drawn | 3.57 to 4.01 ms, then 0.27 to 0.46 ms | 3.57 to 3.90 ms, then 0.27 to 0.43 ms | 3.55 to 3.90 ms, then 0.31 to 0.40 ms |
| key to its picture drawn (the two runs' means) | 7.2 to 13.2 ms (10.5, 11.1) | 8.0 to 14.6 ms (10.6, 10.6) | 8.7 to 17.4 ms (11.2, 12.6) |
| sound queued | 50 to 58 ms | 51 to 58 ms | 45 to 63 ms |

Each is the range over fifteen 2-second windows (fourteen in one) of each of two runs. A training session puts
nothing on the frame's path but a few comparisons: whether the session's battle is still the one on, the round's
number, and, with the input display on, the two sides' buttons, told to the UI only when one changes.

Measuring found one thing: **the input display, drawn each frame,** cost 0.3 ms a frame (a frame was drawn
0.56 to 0.64 ms after its picture, not 0.28 to 0.37: forty boxes and twenty letters). It is drawn into a layer
(`cache-rendering-hint`) and drawn anew only in a frame a button changes in; the table's last column is with
that. (The spread of the key's time between the columns is where the probe's presses fell between ticks, four
or five a window; a frame that shows a press does draw the display anew.)

**Watching a replay** (§16; 2026-10-12, the same Mac and 1280 by 800 window, `NETTAI_REPLAY=<file>` with
`NETTAI_PLAY_STATS`, the build at 700c62646: an eight-minute set of EXE6's, 28,718 ticks in three rounds, its
picture 720 by 480; load average 243 to 278). The viewer indexes the replay ahead from the moment it shows, so
each speed has two columns, while it does and once it is done:

| nettai, `dev`, a replay's viewer | 1x, indexing | 1x, indexed | 4x, indexing | 4x, indexed |
|---|---|---|---|---|
| frames a second | 115.0 to 120.0 | 120.0 | 117.5 to 120.0 | 118.8 to 120.0 |
| frame time p95 / p99 | 8.52 to 10.07 / 8.95 to 19.62 ms | 8.37 to 8.59 / 8.50 to 12.43 ms | 8.46 to 10.11 / 9.23 to 12.91 ms | 8.51 to 9.12 / 9.12 to 12.92 ms |
| long frames in 2 s | 0 to 8 | 0 to 2 | 0 to 4 | 0 to 3 |
| ticks a second | 59.4 to 60.0 | 59.5 to 60.0 | 238.5 to 239.1 | 238.5 to 239.0 |
| the ticks of a frame that ran some | 0.08 to 0.18 ms | 0.20 to 0.32 ms | 0.19 to 0.34 ms | 0.25 to 0.37 ms |
| pictures a second, and each one (presented, written) | 60; 1.68 to 2.13 ms | 60; 1.66 to 1.80 ms | 116 to 120; 1.73 to 1.91 ms | 116 to 120; 1.72 to 1.94 ms |
| a slice of the index, and the worst | 1.40 to 1.46 ms (3.3 to 7.1) | | 1.37 to 1.45 ms (1.8 to 7.4) | |

Each is the range over eleven to sixteen 2-second windows of one run, about 50 seconds each. What the viewer
holds: the window's 120 frames a second at both speeds, and the replay's own speed (59.75 ticks a second, and
four times that). At 1x a picture is presented in every other frame, at 4x in every frame (two ticks to each);
a frame that does all of it, while the replay is being indexed, spends about 3.5 ms of its 8.3 (the ticks, the
picture, a slice). The whole set was indexed in 21 to 22 seconds at either speed, after which a seek anywhere
plays under 240 ticks. The one run whose windows went wrong (from 120 to 96 frames, pictures of 1200 by 800) was
someone using the window as it was measured: its replay jumped from tick 840 to 8,640, 3,120, 13,200 and 23,520
(clicks on the bar) and the window was made larger; it was run again. With both builds shown over the picture
(§16) the viewer wasn't measured.

**At half and twice the speed, seeking, dragging** (2026-10-13, the same Mac and window, the build at 9c4e7e6a1,
the same set; load average 204 to 253; `NETTAI_REPLAY_SPEED` for the speeds, `NETTAI_VIEWER_PROBE` for the
pointer, whose presses, moves and releases are put to the window as the system's are):

| nettai, `dev`, a replay's viewer | ½x, indexing | ½x, indexed | 2x, indexing | 2x, indexed |
|---|---|---|---|---|
| frames a second | 117.0 to 120.1 | 108.1 to 120.0 | 112.8 to 119.5 | 117.6 to 120.1 |
| frame time p95 / p99 | 8.40 to 10.59 / 8.42 to 16.25 ms | 8.38 to 14.35 / 8.40 to 16.71 ms | 8.95 to 12.87 / 10.15 to 18.37 ms | 8.41 to 9.95 / 8.46 to 13.84 ms |
| long frames in 2 s | 0 to 4 | 0 to 20 | 1 to 12 | 0 to 6 |
| ticks a second | 29.5 to 30.0 | 29.9 to 30.0 | 119.1 to 119.8 | 119.3 to 119.6 |
| pictures a second, and each one | 30; 1.51 to 1.74 ms | 30; 1.64 to 1.73 ms | 113 to 120; 1.57 to 1.85 ms | 113 to 120; 1.75 to 1.85 ms |

(The ½x run's one bad window, 108 frames a second with 20 long, is three from its end; its ticks and pictures
cost what they did in the others, and the windows around it ran 115 to 119: the machine's load.)

- *The sound at each speed* is a tape's: each two seconds of the replay (120 of its ticks) gave 263,340
  samples at ½x, 131,670 at 1x, 65,835 at 2x and about 32,918 at 4x, so the device is handed about 65,550 a
  second at every speed (both channels, at nettai's 32,768 a second), as the ticks a second say.
- *A seek far ahead while the index is made:* 5 seconds in (tick 298, indexed to 7,099), a click at 90% of the
  bar asked for tick 25,846. The bar showed it at once, the picture waited (the seek past the index), the index
  went on at the idle budget (3.2 to 5.0 ms a slice, about 5,900 ticks a second) and the seek landed on tick
  25,846, the one asked for, 3.17 seconds later. The window drew 119.5 and 119.7 frames a second meanwhile, p99
  at most 10.9 ms.
- *A drag:* pressed at 20% of the bar (tick 5,743), moved to 50% in twenty steps over a second and let go: the
  picture followed in the states kept every 240 ticks (5,520, 6,000, 6,480 ... 14,160), each the one at or
  before the pointer, and the replay landed on tick 14,359, half of 28,718, where it was let go.
- *Settings' levels across a seek:* the replay watched four times at Music and Sound effects 100 and 100, 0 and 100,
  100 and 0, 0 and 0, each clicked to 60% of the bar 25 seconds after it showed. Each landed on tick 17,230. After
  it, as before it, 0 and 0 was silence sample for sample, and in each stretch the two alone added up (as squares)
  to the two together within 0.5% (0.26022 and 0.25920, 0.20090 and 0.20022, 0.17623 and 0.17674, 0.18402 and
  0.18456).

## 6. The web

What was tried with the first app, on `wasm32-unknown-unknown` (installed). nettai-luau needs WASI SDK 34
(crates/nettai-luau/README.md; fetched from https://github.com/WebAssembly/wasi-sdk/releases/tag/wasi-sdk-34,
`wasi-sdk-34.0-arm64-macos.tar.gz` on an Apple machine, and `WASI_SDK_PATH` set to where it is unpacked):

- **The engine type-checks.** With the SDK and `-Zbuild-std=std,panic_unwind`, nettai-frontend type-checks for
  the target, and with it nettai-battle, nettai-luau, nettai-content, nettai-render, nettai-match,
  nettai-netplay, nettai-replay and nettai-audio. So do nettai-rtc and nettai-extract's library.
- **The app doesn't build yet; winit's web backend is the blocker.** Luau's errors are C++ exceptions, so the
  target is built with `panic=unwind` (`.cargo/config.toml`). Under `panic=unwind` with its `std` feature,
  wasm-bindgen requires every `Closure` to be `UnwindSafe`, and winit 0.30's web backend (Slint's) has closures
  that aren't (`Rc<RefCell<…>>` captures).
  - Ways through:
    - patch winit (wrap its closures in `AssertUnwindSafe`; about a dozen sites);
    - wait for a winit or Slint that does;
    - run Luau out of the UI's module, which the battle's synchronous calls into Luau rule out.
- **What it needs once it builds:**
  - **Time and threads.** `std::time::Instant` panics on this target, so the app's clock must use `web-time`
    (nettai-rtc does already). `std::thread::spawn` panics too: a game's parts and a language's graphics must be
    read in steps on the event loop (the library's `game::Loading` steps), or in a worker.
  - **Files.** The content (Luau modules) must be embedded or fetched; nettai-luau already takes modules from the
    host (`Pack::new`/`Pack::of`), but nettai-content's loaders read paths. The packs can't be served (they come
    from the user's ROMs). The user brings them: a dropped pack folder kept in IndexedDB, or their ROMs extracted
    in the browser (nettai-extract's library builds for wasm32, and `extract` works in memory). Replays go to
    IndexedDB, with download and upload.
  - **Sound.** cpal's `wasm-bindgen` feature (WebAudio), started on the first click as browsers require.
  - **Gamepads.** gamepad-facade's web backend is the browser's Gamepad API; the app's side is the same.
  - **Netplay** is nettai-rtc's web backend (the browser's `RTCPeerConnection`).
  - **Rendering.** Slint's femtovg on WebGL2, which calls the rendering notifier, so the clock carries over.
- **IME on the web.** Slint's canvas can't host the browser's IME, so a text field gets a small HTML `<input>`
  laid over the canvas while it is edited: the field's edit (§11) hands the host its rectangle and its text; the
  host places an absolutely positioned `<input>` there, in the field's face and size, its border and background
  transparent, so the Slint field stays the one that's seen; its `input` and `compositionend` events hand the
  value back; Enter, Tab, Esc, the arrows up and down, and blur end it. On the desktop nothing is laid over and
  the Slint field takes the keys. The overlay isn't built.

## 7. Android and iOS

Not built. What it would take:

- **Slint:**
  - Android is supported (`backend-android-activity-06`, built with `cargo apk` or xbuild and the NDK).
  - iOS is supported through winit (an Xcode project around the static library, as Slint's iOS template has it).
  - The femtovg renderer runs on Android (GLES), and the clock carries over. iOS renders with Skia on Metal,
    which doesn't call the rendering notifier (§1): the clock would move to a display link of the app's own
    or a timer.
- **The layout** is there: a window taller than wide is the phone's (§9), with the main menu and Settings laid
  out for it, and the battle's pad drawn under the picture.
- **The engine:** Luau's C++ builds with the platforms' clang (`cc`); nothing in the engine is
  platform-specific.
- **Files:**
  - The content embedded in the app.
  - The pack imported by the user (Android's Storage Access Framework, iOS's document picker) into the app's
    storage. Or the ROMs imported and extracted on the device, as on the web.
  - Replays in the app's storage, shared with the system's share sheet.
- **Sound:** cpal plays through AAudio/Oboe on Android and CoreAudio on iOS.
- **Input:**
  - Touch is the on-screen pad (`TouchPad`, battle.slint; §11, "The on-screen pad"): it takes the window's
    touches itself, every finger its own. No touch screen has run it.
  - Gamepads: gamepad-facade's native backend is SDL3, which has both platforms' controllers.
- **Text:** the system IME works through winit on Android and iOS.
- **Netplay:** nettai-rtc's native backend (the `rtc` crate over UDP) runs on both.
- **Lifecycle:** suspend and resume. Netplay can't tick in the background, so a suspended match is a dropped
  link, which reconnects within its 30 s or ends.

## 8. Builds

A **build** is a player's side of one game, named and kept: a TOML file of its own,
`builds/<game>/<name>.toml` in the data folder (`builds/store.rs`). It names its game and the build, then states
the side under `[side]` as a match file states one (`nettai_match::file::side_toml`, read back by
`resolve_side`):

    game = "exe6"
    name = "Tengu rush"

    [side]
    navi = "tenguman"
    version = "falzar"
    folder = [ ... ]

    [aside]
    navicust_programs = [ ... ]
    patch_cards = [ ... ]

- **The name is the file's alone.** `[side]` is what a match, a lobby and a replay are given, and it holds no
  name (docs/design/app-ui.md §2.16).
- **A build made new** is `Builds::new_build(content, version)`: the game's own navi, that version, the folder a
  new save of the game starts with. There are no random builds.
- **What is kept aside.** A NaviCust, patch cards and auto battle data are the game's own navi's alone, the one
  that changes form. While a build's navi is another, the facts of the editor's tabs that are that navi's
  (`screens::builds::editor::navi_only_fields`) are kept under `[aside]`, outside the side (the rules refuse a
  NaviCust on a navi that compiles none), and put back when the game's own navi is chosen again
  (`Build::set_navi`, `Build::settle_aside`).
- **A build from a save** (`builds/import.rs`): each game's compat crate reads its own saves; then the save's
  version with all a version sets (`Builds::take_version`), each choice on the preset the save is on
  (`Builds::settle`, which says which it took, typed: the list says "Imported as Dark MegaMan") and the facts
  no build states at the rules' defaults (`Builds::as_built`). A file that is no save of the game is the typed
  "Not a BN6 save"; what the readers say in words goes to the log.

What the screens ask of the game is nettai-match's `build` (docs/frontend.md §6): the game's versions in the
order every list shows them, a build checked with its problems typed (`Checked`, `nettai_match::Wrong`), its
overview, the folder's view and the chips' rows and sorts. The game's builds data
(content/<game>/rules/builds) states each version's color beside it, and the game's strings name what a build is
made of (the text table `builds`: `version.<id>`, `<choice>.<preset>`, and `versions`, what a version sets).
Rust and Slint name no version, and hold no color of one.

**A navi's charge shot** isn't on the Navi tab: none of the three games' content names one (a navi's
locale entry has its name and a run message; EXE6's `patch_card_effects` names the kinds of charge shot a patch
card gives MegaMan, which aren't a navi's own). The panel shows the navi's HP and its element; a charge shot's
line needs the content to name each navi's.

**What any screen uses** (`crates/nettai/src/builds/`, with no screen's code; `ui/build.slint`):

| What | The UI's | The app's |
|---|---|---|
| A game's kit: its builds data, its pictures, its library's order, made once | | `builds::kit(shell)` (`Kit`) |
| A version's tag, in the version's own color | `VersionTag`, drawn by a `Pill` | `builds::version_tag(shell, game, ready, side)`, `Kit::version_tag` |
| The build overview | `BuildOverview { build: <BuildSheet>; }` | `builds::overview::sheet(shell, kit, label, name, side)` |
| A game's builds for a list | `BuildRow { build: <BuildEntry>; nav: "builds"; nav-index: at; }` under a `ListView` of 52-tall rows (the row holds its own touch) | `builds::shelf::load(shell, kit)` (`Shelved`: the file, the build, whether it is ready, its row) |
| A chip's row | `ChipRow { chip: <ChipCell>; number: 1; }` | `builds::overview::chip_cell` |
| What is wrong with a build, in the player's words | the catalog's `BuildWords` | `builds::words::problem(shell, game, ready, &Problem)`: the kind's sentence with its numbers and names, about the part of the build its fact is (`words::wrong(.., &Wrong)` for a kind alone: the general sentence). `words::first_problem(shell, kit, side)` checks a side and words its first problem, for `UserError::BuildUnusable`. `builds::words::Words` (`problem`, `refusal`, `status`) where many are worded at once |

**A problem's sentence** has no full stop and is whole by itself: "2 Giga chips, limit 1", "Gregar is Gregar
only", "The Regular chip's slot is empty". It stands alone on a line of its own (the folder editor's strip) and
after a colon in another's sentence, which ends it (Training's reserved line: "Your build can't be used: 31
chips, a folder has 30."; the shell's "This build can't be used: {}."). `problem` words any
`nettai_match::check::Problem`, whichever screen is shown; its kinds are matched with no catch-all, so a kind
added to nettai-match doesn't compile until it has its sentence.

**It says what it is about.** A problem names the fact it is about (`Problem::field`, the setup field's name),
and `words::part_of` says which part of a build that is (the UI's `BuildPart`): the navi, its level and the
folder by the engine's roles of their facts, the version by the facts the game's versions set and the forms
they bring (souls, Crosses), else the editor's tab whose facts include it (the NaviCust, the patch cards, the
auto battle data). A kind that would otherwise say "something" says the part: "No navi chosen", "No version
chosen", "This navi isn't EXE6's", "Something in the patch cards is there twice", "This navi can't use the
NaviCust". A problem that names no fact, or one with no part, keeps the general sentence ("Something isn't
chosen"). `words::tests::a_problem_is_about_its_part` holds the parts to the facts (a side with no navi, a new
EXE6 side with no version); the sentences are the catalog's, seen in Training's reserved line ("The dummy's
build can't be used: No version chosen.").

**The overview on another screen.** A screen's view has a property of the model, its Slint places the
component, and its module fills the model from a side:

    // ui/screens/play.slint
    import { BuildSheet, BuildOverview } from "../build.slint";
    export global PlayView { in property <BuildSheet> build; }
    BuildOverview { x: 424px; width: 792px; height: self.sheet-height; build: PlayView.build; }

    // src/screens/play.rs
    let Some(kit) = crate::builds::kit(shell) else { return };
    view.set_build(crate::builds::overview::sheet(shell, &kit, "", &build.name, &build.side));

The component is framed and takes the size it is given. At 620 wide or more it is the wide sheet, 540 tall: the
face the round starts with, the name over the navi, the version's tag; Stats, each with what the NaviCust and
the patch cards added beside it in cyan; Abilities; Bugs, each by its full name and its level or amount, purple;
and the folder's 30 chips in two columns of 26-tall rows. Narrower, it is the narrow sheet, 650 tall, which a
player's panel in a match is (430 wide: `label` is YOU or OPPONENT, `name` the player's, `side` 1 or 2 for the
player's color along its top): the stats in two columns, abilities and bugs a line each, the folder in two.
Narrower than 400, a phone's, it is one column throughout, 1158 tall, for a page that scrolls: two columns
leave a chip's name 39 wide there. `sheet-height` is the height of the one it is at its width; which it is
depends on the width alone, never on the build. Every number is the compiled one (`Checked::overview`).

**The editor's tabs** (`screens/builds/editor.rs`). The editor is a frame: the header's name, version and
"✓ Ready to play" (or how many problems), the tab bar, and the tab chosen. **A tab is one Slint file and one
Rust module**, as a screen is for the shell:

- `ui/screens/builds/<tab>.slint` has `<Tab>View`, a global the module sets (with `hints`, the footer's key
  hints while the tab shows, and `status`, its one short status), and `<Tab>Tab`, a component the frame places
  under the tab bar, the page's body wide. What a tab opens over the window is a component of its own, named in
  `BuildsOverlay` (the Folder's, the Patch cards' and the Auto battle tab's sheets on a phone).
- `src/screens/builds/<tab>.rs` has a struct that implements `editor::Tab`: the calls a `Screen` gets (`show`,
  `activate`, `step`, `choose`, `edit`, `focused`, `key`, `button`, `action`, `back`, `answered`), each handed
  the shell and the `Draft`.

| `Draft` | What it is |
|---|---|
| `kit` | The game's builds data, pictures and library order |
| `build`, `side()` | The build, and the side a match is handed |
| `checked` | The build checked (`nettai_match::build::Checked`): its problems, its round's stats, the folder's view, the navi's first face. Made again at each change |
| `edit(|content, side| -> bool)` | **How a tab changes the build.** The closure says whether it did; the draft is checked again at once |
| `choose_navi(navi)` | The navi changed, with what is kept aside moved |
| `with_words(shell, |words| ...)` | The catalogs' and the game's words in the language shown |

A tab neither saves nor checks. When its call returns, the frame keeps the build's file if the draft changed and
shows the header and the tabs' marks anew.

A tab also says which of the build's facts are its own (`Tab::fields`, by their setup fields' names). From that
alone the frame knows three things: a problem with one of them marks the tab's word in the bar; a tab none of
whose facts the game's setup takes isn't in the bar (auto battle data: EXE4's and EXE5's); and the facts of a
tab that is the game's own navi's alone (`Tab::own_navi_only`) are kept aside while the navi is another, the tab
staying in its place, greyed. A tab names every fact its view can edit, and a game may take only some of them
(EXE4's NaviCust has its programs and no board to expand): the tab is there for any, and what is kept aside is
those the game takes. `editor`'s test names each game's tabs and facts.

The tab bar is as wide as the page and the tabs may not be: five don't fit a phone in English, nor four in a
longer language. `TabBar` is then a window on its tabs, moved to keep the chosen one in it, and an end with
more tabs past it fades.

**The Folder tab's header** has the folder's counts beside its title where the language's words leave them the
room, and on a line of their own under it where they don't (the header keeps its height; which it is depends
on the language alone, and the pass is told if the counts don't fit even so).

The five tabs are built. **Adding a tab** is its two files, `lang/ja/builds.po` and a few lines: its place in
`EditorTab` and its `if` (builds.slint's `EditorBody`, whose one clock brings in whichever tab is chosen), its
view's export (builds.slint and app.slint), its word in `BuildsView.tab-names`, its module in `editor::all()`,
and its sheet's `if` in `BuildsOverlay` if it opens one on a phone.

**The NaviCust tab** (`navicust`; docs/design/app-ui.md §2.10). What it draws is the game's data, read by
`builds::grid` (the boards and the command line from the game's rules/navicust/board module as it loaded, each
program's shape, compressed shape, colors and plus mark from its definition; a board by the setup's
`navicust_expansions` where the game takes it, else its one: EXE4's): no game is named.

- *The board*, framed at the left: the square of the engine's 7x7 grid any of the game's boards has a cell on,
  so EXE6's 7x7 (cells 50) and EXE5's and EXE4's 5x5 (cells 64; the board column is EXE6's width in every game,
  so the columns don't move with the game). A cell of the board is inset with a line; one of the frame (EXE6's
  out-of-bounds ring) hatched with a dashed edge; none at all where the board has none (EXE6's corners); the
  board's own cells outlined in white where a ring is round them. A placed piece's cells are its color, with no
  border; a plus part's crossed edge to edge in the board's dark; a piece over the ring hatched over its color.
  The command line is a line over and under its row, drawn over what is placed on it and running past the grid
  to the frame's sides, where an arrow points in at each end (they pulse, `animation-tick`, with motion on), and
  its empty cells are tinted. Under the board, its key (Board, Command line, Plus part, and Out of bounds where
  there is a ring), on one line or two by the language's words alone, and a reserved line for why a piece can't
  go down.
- *Programs*, the open list in the middle: each program in each of its colors (a row a color, as the folder's
  library has a row a code), in the library's order, with its color, its cross if a plus part, and where it is:
  Placed, In hand, or Out of bounds (a placed copy over the ring: no error, and nothing red).
- *Effects and Bugs*, two panels at the right. The effects are what the NaviCust alone does: the build's
  overview without its patch cards (`Checked::of` a second time, the cards' fact emptied), each number it
  changes ("HP +500") and each program at work that gives an ability. The bugs are the whole build's, whoever
  brings them (the draft's overview), each by its full name and its level or amount, purple. A bug that wasn't
  there before the last change pulses purple once.
- *The keys.* The cursor goes over the board cell by cell (each cell is a control, `cell`, its place in the
  drawn grid). Enter on a placed piece picks it up by that cell; Enter on a program's row takes it in hand, its
  middle on the board's (a copy already placed in that color says whether it is compressed). While a piece is
  in hand it is where the cursor is: the arrows (and a pad's stick) move it a cell at a time, the keys bound to
  L and R (A and S) and the pad's shoulders turn it a quarter about the cell held (the tab's `key` and `button`
  take them before the frame changes the tab), C or the pad's north button compresses it, Enter puts it down
  where it fits, and Esc puts it back where it was (one from the list goes back to it). One that doesn't fit is
  refused with the rules' own kind (`GridData::misfit`: `Wrong::OffBoard` or `Wrong::Overlap`) in the reserved
  line, and shakes. Del takes off the piece under the cursor, or a program's last copy from its row, or drops
  the piece in hand. The footer's hints are the hand's while a piece is held. Leaving the tab with a piece in
  hand puts it back.
- *Its motion* (§4's NaviCust row): a piece taken up lifts to 1.06 with a shadow and a white outline along its
  edge (`lifted`), glides from cell to cell as it is moved (and is where it is taken up at once: `glide`), turns
  as a quarter turn about the cell held (`turned`, `turn-way`), shakes where it is refused, and drops with a
  flash along its cells one after another (`dropped`).

**The Patch cards tab** (`patch_cards`; §2.19, §2.20): the folder's shape. A card's number, name, MB, slot and
lines are the game's data: its number its place in the library's order (which lists the cards by their
numbers), its MB the field the rules' budget names (`Overview::budget`), its slot `Slots::own` and `name`, its
lines `builds::cards` (each effect's line from the strings' `patch_card_effects`, a bug's marked by the effect's
`bug`, the card screen's group by its `group`, an effect that holds effects of its own followed by theirs, and a
line's names put in as the screen shows them: `cards::named_by`, a slot by its name, a card by its number).

- *The strip* lays the card under the cursor out: its number, name, MB or slot, why it can't go in (in the
  error red: the typed `EntryRefusal`, worded by `Words::entry_refusal`: "Needs 24 MB"; of one of the build's,
  the rules' problem with it), and its lines in its two groups under their names in the game's strings
  (`patch_card_groups`: Parameter, Ability); where a game's cards have no groups (EXE4's), what it does and the
  bugs it adds to or clears.
- *The build's cards*, framed: in their order, a row a card (number, name, MB, then every line it writes on one
  line, a bar between its groups, bugs purple, fading where they run past the row), and an Empty row after them
  while the list has room; in a game whose cards go in slots, each slot by its name with its card or Empty. Cards
  and MB used (or Slots filled) in the header, ticking as they change, and Together at the foot: what the cards
  add up to, the build's overview without its NaviCust (the numbers they change, what they give, their bugs).
- *The library*, open: every card of the game with its lines, a search by name, and a sort (number, name, MB;
  in a game with slots, number, name, slot). A card that can't go in is greyed with why: Installed, or in the
  error red what it needs. No pictures (a card has none) and no on or off (nettai plays every card in).
- *The keys.* Enter on one of the build's rows goes to the library (on a phone, its sheet); Enter on a card puts
  it in (at the list's end; in a game with slots, in the slot it belongs in, in place of what was there), and the
  cursor stays in the library to put in another. Del takes the card under the cursor out; `[` and `]` (the pad's
  north button: up) move it, as the order matters. A card put in flies from its row to the build's, which
  flashes; one refused shakes.

**The Auto battle tab** (`auto_battle`), in a game whose setup takes the places: the folder's shape again. The
places and their lists are the game's (`builds::auto`: its rules/auto_battle/block's `LISTS`, each named in its
strings' `auto_battle_lists`; a block that states no lists, EXE4's, has its 42 places as one, untitled). A
build states the places alone (its pattern records are the rules' default), so a place holds a chip or nothing.
The strip describes the chip under the cursor; the places are framed under their lists' titles in two columns,
scrolled (a `ScrollPage`, `places.`); the library is a row a chip of those a folder of the build can hold, each
saying in how many places it is. Enter on a place goes to the library, Enter on a chip puts it in and goes on to
the next place (it flies there, the place flashes), Del empties a place. A chip the rules refuse in a place
(their `validate`: one its navi can't play) isn't put: its row shakes and the strip says why, worded for the
part (`Words::problem`: "{chip} can't be played in auto battle").

**On a phone** each of the three is one column: the NaviCust's board over its key, the effects and the bugs a line
each, then the programs; the patch cards' counts, their cards and Together, with the library as a sheet from the
bottom (`PatchCardsOverlay`, with UP, DOWN and REMOVE, or EMPTY); the auto battle's places, one column, with
the library as a sheet (`AutoBattleOverlay`, with REMOVE). The strips are left out, as the folder's is.

`crates/nettai/src/builds/` keeps the parts they share with no screen's code: the NaviCust's grid (`grid`),
the patch cards' lines (`cards`), the auto battle data (`auto`), a list's edits (`edit`) and the library's order
(`order`). nettai-match's `build` is the one source of what a game says of its builds.

## 9. Settings, scaling and the phone

**The settings** are `settings.toml` in the data folder (`settings.rs`), written as each changes and read as the
app starts: the player's name (their account's on this machine until they choose one), the language, the game
last used, the picture's size (the largest that fits, or at most 1 to 8 times), full screen, Reduce motion (as
the system asks until the player says), the battle's text (sharp or original), the levels of the music and the
sound effects, the input delay (4), the keys and the pad's buttons bound (those that differ from the defaults),
and what screens remember by their own keys (`remembered`). A setting the file leaves out is the app's default.

The Settings screen is one scrolling page with a list at its left that jumps to a group: General, Video, Audio,
Online, Controls, About. In Controls, Enter on a key's or a pad button's box asks for the next key or button; one
bound to another action swaps with it, so every action keeps one.

**Scaling.** Every length in the UI is in units of the 1280 by 800 reference window, and the app sets the
window's scale so that one of Slint's `px` is one unit (`Shell::fit_window`): a unit is a 1280th of the window's
width or an 800th of its height, whichever is less. So a .slint file multiplies by no scale, text is drawn at the
display's resolution at any size, and only the battle's picture is counted in the display's own pixels (§1).

**The phone.** A window less than four fifths as wide as it is tall is an upright phone's: a unit is a 390th of
its width, and `Theme.phone` is set, which the theme's metrics and the screens read (the margins 16, the header
56 with a smaller title, no footer and no key hints; the main menu in one column under the reel; Settings without
its list, a label on two lines where it needs them and each note under its control; a select's choices as a sheet
from the bottom; a menu's buttons one over the other; the battle's pad under the picture). Held sideways, a phone
has the desktop's layout.

On a phone the battle's picture has the window's whole width, at its top, with no gutter (`BattleScreen`), and
so has Training's arena: the picture is a whole multiple of the game's 240 (§1), and the gutters' 32 can be the
difference between one multiple and the next (a window 780 of the display's pixels wide takes three times, 720;
716 took two).

## 10. The design system

docs/design/app-ui.md §4 is the look; this is where it lives.

**The theme** (theme.slint): every color, the three faces (Chakra Petch 500, 600 and 700, M PLUS 1 Code and
Murecho, all under the SIL Open Font License, each with its license beside it in `fonts/`), the metrics, and the
durations. A screen uses these and writes no color, size or duration of its own.

**The components** (widgets.slint, shell.slint):

| Component | What it is |
|---|---|
| `Cut`, `PanelBack`, `Panel` | The rectangle with one line and its bottom right corner cut (9 on a button, 12 on a panel, 18 on a dialog or a menu); a panel's back with the cyan accent, or a player's color; a panel with something in it |
| `Button` | Default, the main action (amber), ghost, unavailable (`enabled` off: the focus passes it by); `danger`, `big` |
| `Field` | A text field (§11) |
| `Select` | A value box whose choices step with Left and Right and open as a list (`Shell::pick`: at most ten rows at once, fewer where the window has less room by the select; more scroll, and it opens with the chosen one in view); `focusable` off where its row is what is focused |
| `Stepper` | A value between two arrows, in a box of a set width; `numeric`, `locked`; `focusable` off where its row is what is focused |
| `Seg` | One of two |
| `Slider` | A level, with its number in a box of its own |
| `TabBar` | A row of tabs, the chosen one's underline sliding; a window on them where they are wider than it, an end with more past it fading |
| `Row`, `RowBack`, `Marker`, `RowTouch`, `ListView` | A list row (44); a row's own back; the mark of the chosen row, which slides; a row's clicks; a list of any length, of which only the rows in view exist |
| `MenuItem`, `OptionRow` | A menu's item; a label and a value between arrows |
| `ScrollPage` | A page that scrolls its controls into view |
| `Label`, `Note`, `Message`, `Value`, `Mono`, `Heading`, `Pill`, `GroupTitle`, `Rule` | The text styles; a reserved line for a message; a tag; a group's title over its line |
| `Bar`, `Bars` | A progress bar; the connection's four bars |
| `Page`, `Header`, `Footer`, `Hint` | A menu screen; its header (Back, the title, the game) and footer (key hints, one status) |
| `Picture` | Where the battle's picture goes (§1) |
| `DialogFrame`, `MenuOverlay` | A dialog of a screen's own; a menu over the battle |
| `Backdrop`, `Cursor` | The ground with its hexagons; the bracket cursor |

**The rules** (docs/design/app-ui.md §5) are built into them: a value has a box of a set width and is cut with an
ellipsis; a line for a message is there when empty; digits are M PLUS 1 Code's, one width; the cursor is drawn
over the layout; a field's text is clipped at its padding.

**Motion** (docs/design/app-ui.md §4, "Motion"). The theme has the one table of durations (`quick` 110 ms and
`brisk` 150 for a control, `settle` 200 for a panel or a screen arriving, `leave` 120, `set-piece` 420, and the
`stagger`, 30), and the components move by themselves, so a screen built from them moves without its author
writing an animation:

- the cursor glides between controls, breathes while idle and snaps inward on Enter;
- a button's press flashes its fill and dips it 1, and the main action has a slow sheen;
- a stepper's value slides in its box the way it was stepped; a switch's mark and a tab bar's underline slide;
- a list's chosen mark slides to the next row, and a scroll the app asks for is smooth;
- a panel's accent draws in; a progress bar fills under a moving highlight; a message fades in;
- a screen's body slides and fades as it leaves, and the next one's title wipes in; a dialog grows in as its
  shade fades in, and a failure's shakes once; a menu over the battle slides in from the left;
- the two cards before a battle slide in from their sides, VS stamps, the hexagons rush, and the screen wipes
  into the battle; the hexagons drift otherwise, and a soft pulse crosses them now and then.

What a screen does write is *which of its parts arrive, in what order*: each part goes in an `Arrive` with its
place in the order (the main menu's words, then what is under them, then the reel), and rises as it fades in, 30
ms after the one before. And its own set pieces, from the same pieces (motion.slint):

| Piece | What it is |
|---|---|
| `Tween` | A clock that runs once from 0 to 1, from its creation and each time its `seq` changes: `t` eased out, `linear` the time alone |
| `Pulse` | A clock that falls from 1 to 0 each time its `seq` changes |
| `Arrive` | A part of a screen arriving: rising, or from a side (`from-x`: a tab's body following its underline) |
| `Flash` | A flash over what it covers (a slot a chip landed in, READY stamping) |
| `Shake` | `dx` swinging to each side twice and coming to rest (a refusal): added to the `x` of what shakes |
| `Count` | A number that ticks to its value when it changes |
| `Shell::fly(from, to, image, text)` | Something flying over the layout from one control to another (a chip from its library row to its slot) |

Four rules hold for all of it:

1. *Motion never changes a size.* It moves, fades or covers what the layout has already placed. The motion probe
   (§13) holds every control to its size at rest while things move.
2. *It never blocks input.* The focus moves in the same call as the press, whatever is still sliding. A screen
   that is leaving is gone the moment something is pressed, and a press that comes before the next screen's
   controls exist is done as soon as they do (`App::act`).
3. *It is short:* the theme's durations, and no others.
4. *It can be turned down.* With `Theme.motion` off every clock is at its end and nothing moves by itself:
   Settings' Reduce motion (as the system asks, until the player says: macOS's and GNOME's settings are read),
   and the screenshot pass.

**A row that is focused as a whole** (the design's rows are: the cursor is around the label and its control
together) has a `Focusable` of its own under its control, named as the control is, and the control's own is
turned off (`focusable: false` on a `Stepper` or a `Select`): the arrows' clicks still step it, a click anywhere
else on the row activates it, and the list `Shell::pick` opens is as wide as the row. Training's rows are so
(`BuildOption` and `MenuRow` in training.slint); `OptionRow` here is the other way, the stepper alone focused.

**What has run.** Everything the built screens use is in the screenshot pass and the motion probe (§13). The
components no built screen uses yet (`TabBar`, `ListView`, `Panel`, `OptionRow`, `Count`, `Flash`, `Shake`, and
a `DialogFrame` of a screen's own with a `Field` and its scope) were drawn and walked once, in both, at the
desktop's size and a phone's, in a harness that isn't in the tree (2026-10-12); it found the tab bar running off
a phone's edge, which is fixed and now said by the pass. Play has since put `ListView`, `Flash` and a
`DialogFrame` of its own with a `Field` in the pass, and an online match's `gate` runs there between two
players (§4, §13). Replays has put a `DialogFrame` of a screen's own with its scope, a `Field` outside a
scrolling page, a view of a screen's own (`nav.enter`), a replay's viewer through `Play::Watch` and the build
overview over it in the pass, and its viewer's `set_sound_levels` was checked by its samples (§16). Never run:
`Shell::fly`, `Shell::recorder`, a dropped file, a real gamepad, an IME, a touch screen (the pad's fingers are
put through the same entry by the screenshot pass).
Training and Play were the first to show a `Message`, whose text sat in the middle of its line (a text with no
width of its own is as wide as its words, centered): it is as wide as its line now, and at its left unless the
screen says otherwise.

Three things about how it is made. A clock reads the frame's time (`animation-tick()`) only while it runs, and
stops when it is done, so a screen at rest evaluates nothing each frame but what moves by itself. What moves
without stopping is drawn once and moved as a picture: the ground's hexagons are a cached layer (§5). And the
cursor's glide is the app's (`Nav::settle`: a share of the way to the focused control each frame), not a Slint
animation: that starts over each time its target moves, and the cursor crawled behind a screen arriving or a page
scrolling.

## 11. Focus and navigation

docs/design/app-ui.md §5, rules 5 to 10. There is one focus, and the app holds it (`nav.rs`).

**A control is focusable by holding a `Focusable`** (nav.slint; every control of widgets.slint does), which
tells the app where it is in the window and takes its clicks. From those rectangles:

- **The arrows** go to the neighbor in their direction: a control straight ahead (one that overlaps the focused
  one across the direction) before a nearer one off to the side, and of two equally near the one that starts
  where the focused one does (Settings, not Exit, under REPLAYS). On a stepper, a select, a switch or a slider,
  Left and Right change the value instead.
- **Tab and Shift+Tab** go through the same controls in reading order, around from the last to the first.
- **Enter** activates; **Esc** goes back: out of a list, a dialog or a menu, else within the screen
  (`Screen::back`), else to the main menu.
- A key held repeats a move; a choice is pressed once.

So no screen writes key handling, and a control added to a screen is reachable by being there. A screen opens
with the focus on its main control (`Screen::main`) and remembers where it was; a dialog or a menu is a scope,
and the focus goes back where it was when it closes.

**The cursor** is one object, four brackets drawn over the layout 5 outside the focused control, which glides
from control to control. It shows only while a keyboard or a pad is the last thing used; the pointer hides it.

**Text fields don't trap** (rule 7): the arrows walk past a field without changing it. Enter or a typed letter
starts editing (the letter is the edit's first); while editing, the keys are the field's own, but Esc restores
the text it had, Enter confirms, and Up, Down or Tab confirm and move on. The field's border and caret light
while it is edited, and its text is clipped at its padding and scrolled to keep the caret in view (rule 8). The
edit is Slint's `TextInput`, so the desktop's IME composes into it. The screen is told each change and sets the
text the field shows, so a name is cut to its length and a code put in capitals as they are typed.

**Lists and pages** (rule 9). A `ListView` is one stop: the focus is on one of its rows, Up and Down are its
rows' until its ends, Page Up and Down, Home and End move by a page and to the ends, and the focused row is
scrolled into view. A `ScrollPage`'s controls are scrolled into view as the focus reaches them (its last one
with the page's end, so a note under it shows), and those out of view are passed by when the focus comes from
outside it. The scrollbar's room is reserved whether or not it shows. A select's choices (`PickerLayer`) are a
`ListView` of a set height: it is as tall as its rows up to ten (and to the room the window has under the
select, or over it), keeps that height, and scrolls for more, the chosen row in view as it opens.

**Pads** (rule 10): the D-pad or the stick moves, the south button is Enter, the east one Esc, the shoulders the
game before and after (or a screen's tabs), Start is Enter in a menu. The footer's hints show key caps or the
pad's buttons, by which was last used: a face button by its place in the four, since its letter depends on the
pad's maker.

**The mouse and touch**: a click or a tap focuses and activates in one; everything tapped on a phone is at least
44 tall.

**The on-screen pad** (a phone, in battle: `input::TouchPad`). Slint's `TouchArea` follows one pointer, and a
battle needs a direction and a button held together, so the pad takes the window's touch events itself, before
Slint (`App::touched`, from the same winit hook as the keys). Each button says where it is as the UI lays it
out, and is drawn held when the app says so. A finger that comes down on a button is the pad's until it lifts:
it holds the button under it, and the next one as it slides onto it. The four directions read as one control:
anywhere in their square the direction is the one the finger is farthest toward from the middle, so a thumb
needn't land on a key, and in the very middle it holds none. A finger that comes down anywhere else is Slint's,
as a pointer (MENU, a dialog). The mouse still comes through the buttons' `TouchArea`, one pointer. Unit tests
hold it to that, and the screenshot pass puts two fingers down at once on a phone's battle (Left and A: both
held and drawn held, then one lifted).

## 12. Errors and the log

docs/design/app-ui.md §7. No string from a library's error is ever shown. What a player can be told is one of
`UserError`'s kinds (`errors.rs`), each a sentence of the catalogs (the UI's `Errors`: a whole line, and a
dialog's title and message):

- a failure that ends what the player was doing is a dialog (`Shell::fail`, which shakes once);
- a problem with one field is that field's reserved line (`UserError::line`, in a `Message`); a reason something
  can't be done is at the end of its row, in the error red.

What the code said goes to the log alone (`errors::log`: `nettai.log` in the data folder, a line a failure with
its time and what was being done). gamepad-facade's warnings go there too.

The libraries' errors are typed the same way, a kind for the player and a detail in English for the log
(docs/design/rollback.md §4.9): `UserError::of_link` (nettai-rtc's `ErrorKind`), `of_lobby` (the lobby's
`Reason`), `of_stop` (a match's `StopReason`), `of_replay_read` and `of_refusal` (a replay that doesn't read, or
isn't played here) and `of_export` (nettai-video's) each write the detail and give the kind's sentence; a test
holds every kind to one. Replays shows the first two at the end of a replay's row and beside the list, and the
last in its export dialog's reserved line (§16). What breaks a build's rules is nettai-match's `Wrong`, which the
Builds screen words in its own catalog and hands to `UserError::BuildUnusable` as the reason. A game's pack that
can't be loaded is `UserError::GameOutdated`, with the game's name as shown (§1).

## 13. The screenshot pass

`NETTAI_TOUR=<folder>` (`tour.rs`), in a build with the `tour` feature (`cargo run -p nettai --features tour`;
a player's build has none of the pass, nor the link library's signaling server Play's second player meets the
first through, and says so if asked for it; the tests compile it all the same): the app walks its own screens, in each language bundled (English, Japanese,
the pseudo-locale) and at a desktop's size (1280 by 800) and an upright phone's (390 by 800), and writes the
window as each shows to a PNG. `NETTAI_TOUR_LANGS`, `NETTAI_TOUR_SIZES` and `NETTAI_TOUR_GAME` pick the
languages, the sizes and the game. The app is held still for it: nothing moves, there is no sound device, the
reel is stopped on one picture, the settings are the defaults and are never written, and the pointer is ignored.

The pictures are for comparing with the mockups by eye. Two things the pass checks itself, writes to
`report.txt`, and fails on (the process ends with status 1):

- **Nothing shifts.** Some shots come in pairs a step apart: the focus moved to the next control (the main
  menu, Settings), or a value changed under it (the input delay from 4 to 10, the picture's size). The two
  pictures may differ only inside the controls the step touched and the cursor around them; a pixel that differs
  anywhere else is something that moved.
- **Two fingers hold two buttons.** On a phone's battle, a finger on Left and one on A, through the entry the
  window's touches come by: both are held, and A alone once the other lifts.
- **Nothing is cut.** Every label, button, heading and tab says when its text doesn't fit its box (the UI's
  `Audit`, on for each shot), and the footer when its hints don't leave the status its room. The
  pseudo-locale makes every string about 40% longer, so a layout sized for English alone shows up there.

**The motion probe** (`NETTAI_TOUR_MOTION=1`) is the pass with things moving. A still picture can't show an
animation, so it takes a burst of shots through each (a screen leaving and the next arriving, the cursor's
glide, a value sliding, a press, a list opening, a page scrolling, a dialog, the cards and the wipe into a
battle, a menu over it), each named for the milliseconds since it began. It checks and fails on two things: that
in every shot each control is the size it has at rest, and that a press acts at once (two moves pressed in the
same instant as a screen change land on the new screen's controls, and a direction held down a scrolling page
moves the focus at every repeat).

A screen's steps are a file of its own, `src/tour/<name>.rs` (a child module of `tour`, so it has the pass's
own `write` and `compare`) or the screen's own (`screens/play/tour.rs`), with a function for the pass and one
for the motion probe, each called from `tour::walk` and `tour::motion_walk` with one line
(`training::steps(&mut add, lang, size)`; Play's take `walk`'s `add`, a step and the time it is given first,
and its `shot`, a shot's name and its place in a pair). A pair's second shot may be given more to differ in
than the two cursors (`training.rs`'s `Differs::Builds`: where the entry shows the builds chosen; `replays.rs`'s
`Also::Playback`: the picture, the bar and its two times), found from where the screen's controls are. A step that must wait for something that comes by itself says so
(`Tour::wait_until`: the step after it runs once it has come, or after a minute). Where the shots after it
depend on its having come, the walk says when it didn't (`Tour::never_came`, which the report lists and the
pass fails on: Play's `reach`), so that a picture of the wrong thing isn't taken for the right one.
`NETTAI_TOUR_ONLY=replays` walks Replays' steps alone (it has none for the motion probe).

The pass has nothing of the player's to go by: its settings are the defaults, and Training and Play list a new
build of each of the game's versions (and one that can't be used) in place of the player's builds. The builds the Builds
screen lists are the pass's own too, in `<folder>/builds` (`paths::keep_builds_in`), made there through the
screen as a player makes them, and Replays lists replays of the pass's own.

**Builds' steps** (`tour/builds.rs`; `steps` is handed `walk`'s `shot`, `motion` the probe's `burst`): the list
with none and with three builds (one named in Japanese, one with a name too long for its row), a new build's
dialog with the focus a control on, a file that is no save dropped on the list, a build's rename and the
question before it is deleted, the Navi tab with the focus a card on, with another navi chosen and with the
other preset of the game's choice, and the Folder tab (an entry, the library, its controls a focus move apart,
the library by MB so that what can't go in says why, a chip the rules refuse asked for, a chip put in, a search,
the Regular chip and two tag chips marked, a mark the rules refuse with its reason, a chip taken out, the
sort's list), then the tab after it. A game may have one navi or thirteen, a choice or none, four tabs or five:
a step names the control it is for and does nothing where the game has none. The probe's bursts are the
editor's body arriving as the tab changes, a chip the rules refuse (its row shakes) and a chip put in (it flies
to its entry, the entry flashes, a count that changed ticks).

**The NaviCust's, Patch cards' and Auto battle tabs' steps** (`tour/build_tabs.rs`, a build of the pass's own
made through the screen): the NaviCust's list a focus move apart (a pair), a program taken in hand, moved and
turned, put down on the command line, a plus part beside it, another program put off the command line (its bug
appears), one put over another (refused, and why), and a plus part on EXE6's ring (out of bounds; in a game
without a ring the shot is of the piece in hand); the patch cards' library, its search and sort a focus move
apart (a pair), three cards put in, the library by MB with a card the MB left can't take refused, one of the
build's moved up and taken out, the sort's list; the auto battle's places a focus move apart (a pair), three
filled from the library, one emptied, the places scrolled to the last list. Where a piece goes is found from
what the view shows of the board (`put`: the first cell where the piece in hand would be off the command line,
on the ring, or over another), so the walk is the same whatever the game's shapes. A tab the game hasn't is
passed by (`to_tab` sets `Tour::passing`: the steps after it do nothing and take no picture until the next tab):
EXE6's walk has no auto battle pictures. The probe's bursts: a piece taken up, moved, turned, put down, put off
the command line (a bug pulses), refused; a card put in and one refused; a chip put in a place.

`NETTAI_TOUR_ONLY=<part,...>` walks those parts alone: `shell` (loading, the menu, the screens as they open,
Settings, a dialog, a battle), `training`, `builds`, `build-tabs`, and in the probe `play`; `play` and `replays`
walk Play's and Replays' steps alone.

**A picture's name says what it is of, and the pass doesn't check that**: it checks what moves between a pair
and what is cut. Two things went unseen that way, and each now has its check.

- *A picture of another tab under a tab's name.* EXE4's editor had no NaviCust tab (a mistake of the editor's),
  and the walk's "builds-navicust" was a clean picture of PATCH CARDS. After a picture whose name says a tab,
  Builds' steps ask the view which tab is shown (`on_tab`) and report a picture that is of another, counted
  against it as a cut text is.
- *A game that doesn't load is passed over, and the pass goes on with another.* The shell selects the next
  game that can be played, and the pass then walks that one and ends well: `NETTAI_TOUR_GAME=exe6` with a pack
  the content doesn't load (a sprite the content names and the pack hasn't) gives a clean report of EXE4's
  screens. The log says so (`loading exe6: can't load ...`) and the pictures' headers name the game shown; the
  report doesn't. Read one before trusting a run.

**Replays' steps.** While the pass runs the replays folder is `<folder>/replays`, and the first time through
three sets of the selected game are played by themselves there, recorded, and written again under other names,
days and kinds of match (fifteen files), with three that aren't played beside them (another layout's, other
content's, no replay at all). The steps: the list; the actions beside it (a pair: the focus a step along); a
search; the Online filter; a file that doesn't read; the delete question, and then a delete carried out, of a
file made for it; the export's dialog as it opens (a pair), as an export under way looks and as a failed one
does (both set on the view: no export runs); the two cards; the viewer held a little way into the second round
(a pair a focus move apart, and a pair a second of playback apart); five seconds on; the other console; both
builds over it; the export from the viewer; a closed-builds match's viewer (BUILDS unavailable); and on a phone
the selected replay's sheet (a pair). A shot that follows a change of the selected replay waits for that
replay's still. `replays.txt` in the pass's folder says whether the deleted
file went and the list followed; that is said, not one of the pass's own checks. `NETTAI_TOUR_EXPORT=1` adds,
once (in English at the desktop's size), a real export through `ffmpeg` (the first replay's last round, at
twice the frame, into the pass's folder) and a second one stopped once some of it is written; `export.txt`
says how each ended and what was in the folder.

**Play's steps have a second player** (`screens/play/tour.rs`): the lobby, the loading, a match and what ends
them take two. The second is in the same process: the link library's own signaling server (nettai-rtc's
`testing`, on loopback), and a peer with a link, a lobby and a match's player of its own, run as the library's
tests seat two players. The app's side is the Play screen itself, driven through its controls; the peer does
what the steps tell it (ready, load to 74% and hold, drop its network for six seconds, leave the match, leave).
So the pass runs the whole way between two players over real links, each time it is run: an invite, a wrong
code, a direct connection, the lobby's shared settings, the two bars, a match to its result (the app's player
shoots; the other's build has 1 HP), a second match with closed builds, both ways of leaving. Four things are
the pass's own, so that its pictures are the same from run to run: the builds listed are new builds of the
game's versions (never the player's files), the lobby shows a round trip of 42 ms (what two players of one
process measure moves about), the two lobbies draw their nonces from numbers of the pass's (a match's seed
picks its arenas, and an arena with a rock between the two navis kept the first pass that met one from its
result for over a minute), and a match isn't recorded unless `$NETTAI_REPLAYS` says where.
`NETTAI_TOUR_ONLY=play` walks Play's steps alone.
The motion probe has the lobby's two set pieces, the other player's card sliding in and a ready mark stamping.

**Two apps on one machine** (`tour::apart`): `NETTAI_TOUR_ONLY=play-away-host` in one app and `play-away-join`
in another, each with a data folder of its own. The first hosts directly on a loopback port both know and the
second joins it there, each through Play's own controls; they ready up and play. In the lobby, and again in
the match, the host's window is minimized for eight seconds and then hidden for eight (the calls
`NETTAI_WINDOW_PROBE` makes, made by the walk where they are wanted). Each app writes what its link saw to its
log a second at a time (`play watch`): the frames it ran and how many of them the window drew, the datagrams
the other's link brought, the longest it brought none for, the frames in which its link was down, and the
match's ticks. The joiner's lines say whether the other side saw the host stall; with `NETTAI_NO_TIMER_CLOCK`
on the host they show what a stall looks like. No shot is taken.

## 14. Build time

The user: "you should also try not make the slint build really expensive, because the current nettai binary takes
forever to build for some reason".

**Why the first app built slowly** (measured 2026-10-12, dev profile, on this ten-core Mac while other agents'
builds loaded it: load average 170 to 360, so the wall times are several times an idle machine's and the CPU
times are the ones to compare; sccache off):

| The first app | Wall | CPU |
|---|---|---|
| a clean build of everything (`cargo build -p nettai`) | 27 min | 18 min |
| of which the `nettai` binary's own unit | 10.5 min | |
| after one line of Rust changed | 48 s | 9 s |
| after one line of a .slint file changed | 4 min 50 s | 82 s |
| the same .slint edit with the crate at opt-level 0 | 1 min 51 s | 15 s |

`slint-build` made one Rust module of 7.4 MB (74,688 lines) from 6,939 lines of Slint, and it was part of the
`nettai` binary's crate. Of the .slint edit's 280 s in rustc, 223 were LLVM (67 its passes, 158 thin LTO over the
crate's units), 21 the translation to LLVM's IR, and about 25 everything before it (expansion, checking); the
Slint compiler in the build script was 8 s, and linking 2. Three things made it so:

1. **The size of what Slint generated.** The UI was built from containers (a component with others placed
   inside it), which Slint copies into each of their users, and from 96 `if`s and 42 `for`s, each of which is a
   component of its own with its own item tree and vtable: 4.5 of the 7.4 MB were those. A conditional with
   nothing in it is about 35 KB of Rust.
2. **It was optimized.** The workspace's dev profile is opt-level 1 (the battle needs it: §5), and the generated
   module was compiled at it: five times the CPU of opt-level 0, for code whose work at run time is done by the
   Slint library.
3. **It was the app's own crate.** Any edit to a .slint file compiled all of it again with the app's 9,300
   lines of Rust; any edit to the app's Rust expanded and type-checked the 7.4 MB again (the 48 s).

**What the redesigned app does about each:**

1. **nettai-ui is a crate of its own**, the generated module and nothing else, so an edit to the app's Rust
   doesn't touch it, and an edit to a .slint file compiles it alone and then links.
2. **It is built without optimization or debug info** in the dev profile (`[profile.dev.package.nettai-ui]` in
   the workspace's Cargo.toml). The release profile optimizes it as everything else.
3. **The components are written to stay small in the generated code**, and a screen's author keeps it so by four
   habits (widgets.slint's and nav.slint's comments say the same):
   - *Leaves, not containers.* A component used with nothing inside it is compiled once however often it is
     used; one used with children inside is copied into each user. So a row's label and its control are two
     components side by side, not one around the other (Settings' rows), and a panel's back is a leaf with the
     panel's parts beside it.
   - *Nothing on a component's own root that forces the copy:* `visible`, `opacity`, `clip`, a drop shadow or a
     size in percent on the root element of a component makes Slint copy it into every user. They go on an
     element inside it, or on the instance.
   - *`visible`, not `if`, for a small part* shown or hidden; an `if` or a `for` only where a part is truly
     there or not (a screen, a dialog, a list's rows). One row component under a `for`, not conditionals inside
     the row.
   - *A long list is a `ListView`*, whose rows exist only in view.
4. **One renderer and no more features than are used:** femtovg alone (the clock needs it: §1), without the
   software renderer and the accessibility backend the first app also built. No standard widgets are imported.
   The fonts are embedded as files (5.6 MB in all, 3.9 of it M PLUS 1 Code with its Japanese), which costs the
   linker nothing measurable.
5. **The build script writes a file only when what it says changed** (the merged catalogs, which Slint has cargo
   watch), and gives what it wrote a time long past. Rewritten on every run, they made every build find the UI
   changed and compile it again: 14 seconds for a build with nothing to do. Written once but dated by the run,
   they made the build after every clean one do the same.

**What it costs now** (measured 2026-10-12, dev profile, sccache off, load average 100 to 150):

| The redesigned app | Wall | CPU |
|---|---|---|
| a clean build of nettai-ui and nettai, their dependencies built | 47 s | 27 s |
| of which nettai-ui (its build script 3 s, the script's run 2.7 s, the generated module 19 s) | 25 s | |
| of which the `nettai` binary | 20 s | |
| nothing changed | 1.1 s | 0.3 s |
| after one line of Rust changed | 3.4 to 3.7 s | 2.0 s |
| after one line of a .slint file changed (a value) | 8 to 12 s | 4.9 s |
| after a .slint edit that adds or changes components (this work's own edits, with Rust edits) | 14 to 32 s | 20 to 24 s |

Measured again after the same day's follow-ups (the pad's touches, the timer's clock, the sound's levels; load
average 54 to 113): a clean build of the two crates 45 to 68 s of wall time and 28 to 29 s of CPU, the build
after it 1.9 s; one line of Rust 7.6 s (2.3 s of CPU); one line of a .slint file 26 to 28 s (5.3 to 6.1 s of
CPU).

The generated module is 3.3 MB (28,204 lines, 102 components) from 4,316 lines of Slint with 18 `if`s and 6
`for`s. The first app's whole clean build wasn't run again for the new one: the engine's crates, most of it, are
the same. One dependency is new and slow: gamepad-facade's SDL3 is built from its source with cmake, about three
and a half minutes of a clean build on the loaded machine (gilrs, which it replaces, took 23 seconds), once.

**With Play** (2026-10-12; one script before the screen was written and after, on the branch it was written
on, before Training and Builds were merged into it: dev profile, sccache off, the two crates cleaned with their
dependencies built; the load average was 270 to 330 before and 200 to 230 after, so the CPU times are the ones
to compare):

| | Before Play: wall, CPU | With Play: wall, CPU |
|---|---|---|
| a clean build of nettai-ui and nettai | 113 s, 28.2 s | 184 s, 45.5 s |
| nothing changed | 23 s, 3.7 s | 3.2 s, 0.4 s |
| after one line of Rust changed | 9.5 s, 1.7 s | 7.4 s, 1.8 s |
| after one value of a .slint file changed | 43 to 48 s, 5.5 s | 33 to 52 s, 7.1 to 7.3 s |

The generated module went from 3.32 MB (102 components) to 5.14 MB (131), which is what the clean build and
the .slint edit pay for: Play is 1,500 of the UI's 5,800 lines of Slint, with 12 of its 31 conditional
elements and 4 of its 10 `for`s. It is the screen with the most on it (three parts, three dialogs, a menu of
five pages and the two builds over a match), and it was 5.35 MB before its conditionals were merged: one
conditional a part, with what differs on a phone placed and shown by `visible` inside it, where the first
draft had one for each thing that differs. An edit to the app's Rust costs what it did.

**What trades against it.** Motion (§10) added 0.43 MB of generated Rust (2.76 to 3.19 MB: the clocks and the
offsets they drive), which is in the figures above. The accessibility backend is off: a screen reader sees
nothing of the app. And with one renderer, a machine without OpenGL can't run it.

**What a screen costs: Training** (§15; measured 2026-10-12 with one script, back to back, the tree before it
and the tree with it; sccache off; CPU is user and system time together, and the one to compare: the wall
times follow the machine's load, 129 to 206 before and 166 to 197 after):

| | Before Training | With Training |
|---|---|---|
| a clean build of nettai-ui and nettai | 77 s, 33.4 s of CPU | 162 s, 40.6 s of CPU |
| nothing changed | 2.0 to 2.6 s, 0.7 s of CPU | 2.6 to 3.6 s, 0.6 to 0.9 s of CPU |
| after one line of Rust changed (a constant of battle.rs, and put back) | 8.8 to 11.1 s, 5.8 to 6.0 s of CPU | 17.4 to 20.3 s, 6.9 to 7.1 s of CPU |
| after one line of menu.slint changed (a value, and put back) | 20.7 to 31.9 s, 6.5 to 7.5 s of CPU | 38.6 to 40.0 s, 8.2 to 8.3 s of CPU |
| after one line of training.slint changed (a value, and put back) | | 33.7 to 43.8 s, 7.9 to 8.1 s of CPU |
| the generated module | 3.33 MB, 28,204 lines | 3.88 MB, 31,925 lines |
| the Slint | 4,316 lines, 18 `if`s, 6 `for`s | 4,972 lines, 17 `if`s, 7 `for`s |

Training's 656 lines of Slint are 0.55 MB of generated Rust: the entry 0.24 MB (the main menu's screen is 0.23),
the training menu and its rows 0.22, the input display 0.09, the entry's cards and rows 0.07, less the 0.17 its
stub was. What kept it there: the entry's parts are shown and hidden, not `if`s; the menu's rows are one
component under one `for`; the input display is in the window throughout and shown when it is on, where an `if`
was 46 KB more; Training has no overlay (an empty one under its `if` was 22 KB); a card places its three texts
itself, where two layouts were 9 KB more; the display's arrows are the text face's, not paths (11 KB).

The follow-ups after it (the phone's picture edge to edge, a select's list scrolling, a terminate request, a
game that can't be loaded told; measured the same way, load average 260 to 336) cost 0.06 MB of generated Rust
(3.88 to 3.94 MB: the select's list as a `ListView`) and nothing measurable in time: a clean build 35.5 s of CPU
before and 36.7 after, one line of Rust 6.1 and 6.5 to 6.8, one line of a .slint file 7.0 to 7.1 and 7.3 to 7.6.
A training battle's figures (§5) are the same before and after them: 107.6 to 120.0 frames a second and 117.6 to
120.0, a key to its picture 11.9 and 11.1 ms against 11.0 and 10.2 (two runs each, back to back).

**What a screen costs: Builds** (§8; measured 2026-10-12 with one script, back to back: the tree before it,
nettai-ui at 6648374c7, and with it, ui-builds at 0821e2b1c; sccache off; CPU is user and system time
together, the one to compare: load average 207 to 345 throughout):

| | Before Builds | With Builds |
|---|---|---|
| a clean build of nettai-ui and nettai | 138 s, 35.6 s of CPU | 169 s, 56.5 s of CPU |
| nothing changed | 2.7 to 3.9 s, 0.6 to 0.8 s of CPU | 2.5 to 3.7 s, 0.6 to 0.7 s of CPU |
| after one line of Rust changed (a constant of battle.rs, and put back) | 18.2 to 18.5 s, 6.4 to 6.9 s of CPU | 15.8 to 17.3 s, 9.6 s of CPU |
| after one line of menu.slint changed (a value, and put back) | 38.3 to 40.5 s, 7.1 to 7.3 s of CPU | 39.2 to 41.6 s, 10.3 to 10.5 s of CPU |
| after one line of builds.slint, or of build.slint (the overview) | | 20.4 to 40.6 s, 10.2 to 10.8 s of CPU |
| the generated module | 3.88 MB, 31,930 lines | 6.14 MB, 51,052 lines (6.08 MB after the three fixes below) |
| the Slint | 4,975 lines, 17 `if`s, 7 `for`s | 7,125 lines, 31 `if`s, 18 `for`s |

Builds' 2,150 lines of Slint are 2.2 MB of generated Rust, four times Training's for three times its Slint
(by component, after the fixes below): the Folder tab 0.72 MB (the tab 0.12 and its `if` 0.04, the library pane
0.09 and its rows 0.1, a phone's list 0.1 and sheet 0.11, the slots 0.07), the list, the overview beside it and
a phone's page of one build 0.44 (each `if` page 0.07 to 0.12: an `if` around a `ScrollPage` or a `ListView` has
all of it copied in), the overview and its rows 0.3, the Navi tab 0.17, the new build's dialog 0.17, the tab bar
and the editor's frame with its five tabs' `if`s 0.2, the views and the catalog's words 0.09. A Rust edit costs 3 s
more CPU than before: the app's crate has the screen's 5,000 lines of Rust. Three things were made smaller after
measuring: a build's row and a phone's folder row hold their own `RowTouch` (a row given one as a child was
copied into its list: 64 and 88 KB, now 46 and 36), and the editor's five tabs are under one `EditorBody` with
one clock, where an `Arrive` around each tab was copied into each tab's `if` (27 KB an empty tab, now 16).
What stays large: a `ListView` and a `ScrollPage` are containers, copied into each use (40 to 50 KB each, five
in Builds), and each `if` page holds the parts it lays out.

**What a screen costs: Replays** (§16; measured 2026-10-12 with one script, back to back: the tree before it,
nettai-ui at b566b2000, and with it, ui-replays at 700c62646; sccache off; CPU is user and system time together,
the one to compare: load average 198 to 352 throughout):

| | Before Replays | With Replays |
|---|---|---|
| a clean build of nettai-ui and nettai | 170 s, 56.7 s of CPU | 316 s, 71.4 s of CPU |
| nothing changed | 2.3 to 3.1 s, 0.5 to 0.8 s of CPU | 3.3 to 4.6 s, 0.5 to 0.7 s of CPU |
| after one line of Rust changed (a line added to main.rs, and taken away) | 8.5 to 11.0 s, 2.9 to 3.1 s of CPU | 10.3 to 13.1 s, 3.0 to 3.2 s of CPU |
| after one line of menu.slint changed (a value, and put back) | 37 to 51 s, 10.9 to 11.4 s of CPU | 64 to 73 s, 12.5 to 12.9 s of CPU |
| after one line of replays.slint changed (a value, and put back) | | 43 to 92 s, 12.3 to 12.9 s of CPU |
| the generated module | 6.19 MB, 51,366 lines | 7.62 MB, 61,733 lines |
| the Slint | 6,211 lines, 29 `if`s, 12 `for`s | 7,462 lines, 35 `if`s, 14 `for`s |

Replays' 1,246 lines of Slint (the stub was 22) are 1.42 MB of generated Rust: the viewer 0.50 (its transport
0.16 and its page's `if` 0.14, the page 0.06, the transport's buttons, choices and bar 0.14), the list and the panel
beside it 0.44 (the page 0.11 and its `if` 0.06, a row 0.07 and its `for` 0.02, the panel 0.05 and its `if` 0.05,
the facts 0.08), the export's dialog 0.2 (its `if` 0.05), both builds over the viewer 0.12 (half of it their
`if`), a phone's sheet 0.11, the view 0.05. A .slint edit costs 1.6 s more CPU than before, about 14%. Measured
before the builds over the viewer and the merge of Builds (against nettai-ui at 6648374c7, load average 176 to
320), Replays was 1.33 MB (3.88 to 5.21) and a .slint edit 7.1 to 7.5 s of CPU before it, 8.9 to 9.1 with it.
One thing was made smaller after measuring: the desktop's transport and the phone's were two components under an
`if` each, and are one whose other shape's parts are hidden (`visible`, and passed by the focus). That was only
62 KB less, not the 0.21 MB the two `if`s seemed to be: most of a conditional is the item tree of what it holds,
and the viewer's page's `if` describes the transport's items now (44 KB before, 133 after). What stays: each `if`
here is a part that is truly there or not (the list's page and the viewer's, each with a `Picture`, which two of
can't be at once; the panel beside the list, whose `Picture` a phone's sheet has instead; the sheet, the dialog,
the builds over the viewer), and each costs about as much as its own component again.

## 15. Training

docs/design/app-ui.md §6, "Training": you against a dummy set up from a menu while in training. It is
`src/screens/training.rs` and `ui/screens/training.slint`.

**The entry.** Two rows, Your build and Dummy's build, and START TRAINING at the column's foot; beside them (on a
phone, under them) the two builds as cards over the arena.

- Each row is the player's builds of the selected game (`builds::store::list`), stepped with Left and Right or
  opened as a list with Enter. Both are remembered by game between runs (the settings' `remembered`:
  `training.build.<game>` and `training.dummy.<game>`, by the build's file); with nothing remembered both are
  the first build that can be used.
- A card says its build's name, its version's tag and its navi.
- The arena is the session's first round as its opening ends, the two builds' navis on it: a battle of its own
  held at that tick, in a `Picture` at three times (§1), so it is whole pixels as every picture of the battle
  is. (The window has one texture: the entry's box shows it only once the arena is what is in it, from the
  second frame the window draws of it.) It is made once the builds chosen have stayed 160 ms (making a round holds the window about 20 ms: a
  step through the builds doesn't make one each time), and what the session will be picked from is drawn as the
  entry lists a game's builds and after each session, so the arena shown is the one START TRAINING opens on.
- With no build of the game there is one button, CREATE A BUILD, which goes to Builds.
- A build is checked as the lobby will check one (`nettai_match::build::Checked`), once for what its file
  says. One that can't be used says so in the reserved line ("Your build can't be used.", "The dummy's build
  can't be used."), START TRAINING is unavailable, no arena is drawn, and the screen opens with the focus on
  the row to change. *The rule it breaks isn't said yet:*
  `Wrong`'s kinds are the Builds screen's to word (§12), and `training::rule` is where that wording goes, as
  the line's own `{}`; meanwhile each problem is in the log.

**A session** (its interface is §2's "A training session, from any screen"). `training::start` checks the
match of the two builds (`nettai_match::check::problems`), shows the two cards while the library's `Preload`
makes it ready, and plays the set with `LivePlayer::against(set, opponent, seed)`. It isn't recorded.

- It has no end: when the set is decided the shell says `Finished` and Training starts the battle over
  (`Battle::restart`), as RESTART does, and R or F5 on the keyboard and the pad's north button without opening
  the menu (unless the player bound them to a button).
- The session is the shell's (`Shell::training`), and gone with its battle however that went: ended by
  `training::end`, by a battle another screen started in its place, or, shown in another screen's `Picture`,
  by that screen being left.

**The training menu** (Start or Esc; the battle is paused behind it, the dummy in view beside the panel). Its
rows are a model (`TrainingView.rows`: one row component under one `for`), made from one list,
`training::ROWS`:

| Row | Values | What it sets |
|---|---|---|
| Dummy: Action | Stands still, Moves around, Attacks at random, Plays your recording | the library's `Opponent::in_fight`, from the next tick |
| Dummy: Custom screen | Picks no chips, Picks the first it can | `Opponent::on_custom_screen` |
| Dummy: Recording | None, or how long it is ("2.4 s") | chosen (Enter, Left or Right), it records one: below |
| Battle: Your HP, Dummy's HP | Normal, Refills | the library's `Training::refills`, by side: the navi's HP is full again a second and a half after it last lost any (docs/frontend.md §2) |
| Battle: Custom gauge | Normal, Always full | `Training::full_gauge`: the custom gauge is full whenever it runs |
| Battle: Chip draw | Shuffled, Folder order | `Training::chip_draw`: both sides' custom screens deal from their folders' tops, in the folders' own order |
| Display: Input display | Off, On | the app's own drawing, below |

The Battle rows are the library's training options (`Player::set_training`, set at once on the battle behind
the menu): host-side options a training session sets between ticks, applied to the battle's own typed state
(a navi's HP, the custom gauge, the battle folder the custom screen deals from) before each tick, never a rule
of the simulation. With them at their normal values the battle is
the set's as it was before they existed, tick for tick by digest (the library's test of it, docs/frontend.md
§2), and every online match and replay is untouched: only live play has them.

A row is focused as a whole and changed at once by Left and Right (Enter steps it on), around from its last
value to its first. Under them RESUME, RESTART, CHANGE BUILDS (the entry again, with another arena) and QUIT TO
MENU. The menu opens where the focus last was in it. The dummy's setup, the battle's options and the input
display are remembered between runs (`training.action`, `training.custom-screen`, `training.your-hp`,
`training.dummys-hp`, `training.custom-gauge`, `training.chip-draw`, `training.inputs`); a session another screen started begins as that screen says, every
battle option at its normal value, and keeps nothing of the dummy's or the battle's.

**The dummy's recording** (the library's `LivePlayer::record_opponent`, docs/frontend.md §2). Choosing the
Recording row closes the menu, and from the battle as it stands the player's buttons drive the dummy, from
wherever it is (the player's own navi stands still; on the custom screen the player's buttons are their own
screen's as ever, and the dummy's screen does what its Custom screen row says). RECORDING shows over the
battle meanwhile (in the dummy's color, with the input display's layer: `TrainingInputs`). Each tick the fight
runs (not the turn's banner, the custom screen, the game's pause) its buttons are kept, up to a minute. Start
ends it (or the menu opening, or starting over, or the minute): the dummy's Action becomes Plays your
recording, and it plays the buttons over and over from wherever it stands, while the player has their navi
back. A new recording replaces it. Nothing of it is kept between sessions, and Plays your recording isn't
remembered as the Action (with nothing recorded the dummy stands still).

*Adding a row*: a value of `TrainingOption` and its words in training.slint (`label`, `value`), its place in
`ROWS`, and what it does in `values`, `value` and `Training::step_option`. A group's title is a row too.

**The input display** (`TrainingInputs`) is the app's own: two rows of ten buttons (the directions, A, B, L, R,
SELECT, START), the player's over the dummy's in their sides' colors, each lit while it is held. It reads what
each side's custom screen reads (`Battle::custom.sides[n].joypad.held`: this tick's buttons, not the fight's
delayed ones) after the frame's ticks, and tells the UI only when they change. It is in the picture's top right
corner, under the eighth the battle's own text takes there; on a phone, under MENU. It is drawn into a layer
and drawn anew only when a button changes (§5: drawn each frame it cost 0.3 ms a frame).

**On a phone** the entry is one column (the rows, the line, the cards side by side, the arena, START TRAINING at
the foot), the menu's rows have their label over their arrows and scroll (a `ScrollPage`, `menu.rows`, which on
a desktop holds them all) and its buttons are one over the other, and in
battle the line beside MENU says "Training · Round 1" (`Shell.battle-note`, which a battle's owner sets).

**What isn't there.**

- The rule a build breaks, in the reserved line (above).
- A choice of arena: it is picked from the seed, as a match's is.
- A real key press: the screenshot pass drives the screens through the same calls a key becomes (`App::act`)
  and holds the battle's buttons directly; the window's own key events were not sent to it.

## 16. Replays

docs/design/app-ui.md §6, "Replays". The screen is `ui/screens/replays.slint` and `src/screens/replays.rs` (with
`replays/viewer.rs` and `replays/export.rs`); the folder's reading is `src/replays.rs`. The library under all of
it is docs/frontend.md §8: the file, the viewer and its seeking, the video.

**Nothing of the folder is read on the window's thread** (`crate::replays::Library`). The screen has a thread of
its own, started when it first asks something, and takes what the thread found at its next frame:

- *The listing.* Each time the screen shows, the replays folder is listed again (a match just recorded is
  there): every `.ntrp` file is read and what it says of itself kept (its head, its info, its length and its
  rounds' starts; not its ticks). A file as it was at the last listing (its size and time) isn't read again.
- *The score.* A replay holds no result, so each replay of the selected game is played out there (the library's
  `play_out`: no picture, no sound), newest first, and its row gets its score as it is known. What it gave is
  kept between runs (`replays::cache`), in `replays-played.toml` beside the settings (the pass's in its folder),
  never in a replay: each outcome by the hash of the file's bytes (the engine's stable hash) and of the content
  it was played on, all of one engine. A kept one is told at once, so only what is new or changed is played
  out. What is kept of files no longer in the folder is dropped as the cache is written (whole or not at all,
  when the thread has nothing to do); a cache that can't be read is told to the log and made again. A replay
  that isn't played here isn't kept: that is found before any of it is played. Listing the pass's folder (18
  files, fifteen of EXE6's to play out: 77 minutes of sets) took 6 to 20 ms; with nothing kept, their outcomes
  took 29.6 seconds more, and with all of them kept, 28 to 35 ms in all (load average 217 to 243;
  `NETTAI_PLAY_STATS` says both).
- *The still.* The selected replay's picture is a frame the thread draws a moment into its first fight, shown
  through the screen's `Picture` as any picture is (§1): a whole multiple of the frame, the text at the
  window's resolution. The box is empty until the selected replay's own still has come: a still is never shown
  beside another replay than its own.

**A replay that can't be played is a row, not a failure.** A file that doesn't read (`ReadError`), a replay
whose head names another engine or other content, one the library refuses as it is played out (`Refused`) and
one that plays another battle than it recorded each show their sentence in the row, in the error red, in
place of the opponent, the score and the length (`UserError::of_replay_read`, `of_refusal`: §12), and beside
the list with WATCH and EXPORT VIDEO unavailable; SHOW FILE and DELETE stay. Each is written to the log once.
A file that doesn't read is listed with the replays of the game its head names, where the head reads
(`nettai_replay::Head::read`: a damaged file, or one of layout 1, whose head is the same), and with every game's
only where even that doesn't.

**The list** is the selected game's replays, newest first: the date, the opponent (the other player's name;
`P1` or `P2` where the replay has none), the score as the recorder's (`W 2–1`; the rounds alone where the set
wasn't finished) and the length, each in a column of its own width. The search is by opponent. The filter
between All and Online is there only while the game has replays of both kinds. Enter on a row watches it, E
exports it, Delete asks once (Cancel focused) and removes the file. On a phone a tap on a row opens the
selected replay as a sheet from the bottom, with the four actions one under the other. No build's name is
anywhere: a replay has none.

**The viewer** (`Watching`) is the library's `Viewer`, which the shell makes ready behind the two cards
(`Play::Watch`: §2) and hands over. The cards say who played and nothing of what they brought. Each frame the
screen runs the ticks due, presents the picture into the window's texture if one ran, and hands the sound on:

- *The transport:* the time, the bar (a click or a drag; a mark where each later round begins), the length;
  to the round's start, a frame back, play or pause, a frame on, the next round; the speed (½, 1, 2, 4), the
  side whose console is shown, EXPORT VIDEO. The focus starts on the bar, where Left and Right go five seconds
  (it is a control whose arrows change it, as a slider is); the speeds and the sides are each a control, so the
  arrows walk through the row and past it. Space plays and pauses, comma and period step a frame (and pause),
  the brackets (and the shoulders) go a round, Enter on the bar plays and pauses, Esc goes back to the list.
  At its end a replay stops with the transport there; play starts it again.
- *No seek stalls the window.* From the moment it shows, the replay is indexed ahead a slice a frame
  (`Viewer::index`), each slice as many ticks as the last one's time says fit a budget: 1.5 ms while the
  replay plays, 5 ms while nothing does. A seek to what is indexed is done at once (it plays under 240 ticks).
  A seek past it waits: the bar shows where it is going, and it lands when the index has reached it.
- *A drag moves the picture in steps:* while the bar is held the picture is the state the viewer keeps at or
  before the pointer (one every four seconds, which costs no playing), and it lands exactly where the bar is
  let go.
- *The sound* goes to the shell in whole ticks' worth (`Shell::play_samples`), because the menus' sounds are
  ticked with what a screen plays: the viewer's own while it plays, at any speed, and while it doesn't,
  silence at the original's rate, so a move of the cursor is heard over a paused replay and the title music
  isn't. It is at Settings' levels (`viewer.set_sound_levels(shell.sound.battle_levels())`): Music is the
  replay's music too. Checked by the viewer's own samples (`NETTAI_PLAY_STATS` says their loudness for each two
  seconds of the replay, by its ticks): 45 seconds of one replay at Music and Sound effects 100 and 100, 0 and
  100, 100 and 0, 0 and 0. Both at 0 is silence, sample for sample; Music at 0 leaves the effects' sound in
  every stretch (rms 0.04 to 0.16, by bursts), Sound effects at 0 the music's (0.14 to 0.19, steady); and in
  each stretch the two together are within about 1% of what the two alone add up to (as squares). Nobody
  listened to it.
- *While the window isn't drawn* (minimized, hidden: §1's timer runs the frame), a replay stands still: it asks
  for no time and plays no sound, and goes on where it was when the window shows again. Checked with
  `NETTAI_WINDOW_PROBE=hide:20:12`: the timer ran one frame, then nothing for the 12 seconds, and the replay went
  on from the tick it was at, its stretches as loud as without the hiding. The app is kept awake
  (`Shell::stay_awake`) while a replay plays in a window that is drawn, and while a video is being made.
- *The builds.* Where the match's builds weren't closed, BUILDS (the transport's sixth button, after the five;
  B, or Select on a pad) shows both over the picture's sides, as a match shows them: each the narrow sheet of
  `BuildOverview` (§8), the console shown's player's at the left in red, the other's at the right in blue (the
  colors of their sides of the picture), YOU over the name of the side that recorded it and OPPONENT over the
  other's, each name the player's (a replay holds no build's). They are a scope of their own: each sheet is a
  control, and a press or a click on them, BUILDS, B, Select or Esc puts them away; the replay goes on under
  them, and its keys still work. On a phone they are one under the other in a page that scrolls (each sheet is
  then one column, 1158 tall). A closed-builds match's sides aren't kept at all (`Info::closed_builds`): its
  BUILDS can't be pressed.
- *On a phone* the picture has the window's whole width, as the battle's has (§9): a window 780 of the display's
  pixels wide shows three times the frame, where the page's gutters left room for two. The transport is under
  it, with its buttons across, then Speed and View as steppers and EXPORT VIDEO. One component is both shapes
  (the other shape's parts hidden and passed by), since two under an `if` each cost more (§14).

**The export** (`Export`) is a dialog of one size from its start to its end: Size (2, 4 and 6 times the frame,
4 chosen), View, Part (the whole match, or a round), Save as, the bar and its line, CANCEL and EXPORT. EXPORT
runs nettai-video's `export_mp4` on a thread of its own, with a copy of what the window plays from (the game's
graphics in the language shown, the text as the settings have it, its sound); the thread says how far it is
and is told to stop through what the two share, which the dialog reads each frame. CANCEL (or Esc) stops an
export under way and leaves the dialog open; with none under way it closes it. The line says the round and
the percentage, then "Saved.", or what went wrong in the player's words (`UserError::of_export`: ffmpeg not
found, the file not written, anything else with the log named). The app keeps awake while one runs (§1).

- *The file* is an MP4 and nothing else: the NUT stream is how the pictures reach ffmpeg, never a file. It is
  named for the day, the opponent and the part, in the folder last exported to (at first the system's videos
  folder's `nettai`), with a number after it where that name is taken; Save as asks for another place in the
  system's dialog. It takes its name only once it is whole: stopped or failed, nothing is left (the library's
  `mp4::Encoder`).
- *ffmpeg* is the one on the path, or where a package manager puts it (`/opt/homebrew/bin`, `/usr/local/bin`,
  `/opt/local/bin`: an app started from the desktop has a short path), or `$NETTAI_FFMPEG`.

**The main menu's reel** takes its replays from `crate::replays::Reel`: the selected game's latest five, found
and read on a thread as the menu shows, then given one after another, around. One that doesn't play here is
passed over and not given again; with none, the reel is the set made for it (§1).

**What has run, and what hasn't.** The screenshot pass walks all of it but the export's real run (§13), in the three
languages at both sizes; the real export, a stopped one and one with no ffmpeg ran once each through
`NETTAI_TOUR_EXPORT` (on EXE6: the first replay's last round at twice the frame, 8,539 frames, 143 seconds of H.264
and AAC, 19.4 MB; the stopped one's `.part` file was there while it ran and gone after it; with `NETTAI_FFMPEG`
naming no program the line said "Video export needs ffmpeg. Install it, then try again." and nothing was written).
The viewer was watched playing by `NETTAI_REPLAY` with `NETTAI_PLAY_STATS` (§5), at ½, 1, 2 and 4 times, and driven
by `NETTAI_VIEWER_PROBE`: a seek past the index, a drag of the bar by the window's pointer events, and Settings'
levels across a seek (§5). What Play's code records was listed and watched too: Play's steps alone with
`$NETTAI_REPLAYS` set recorded three matches (one to its result, W 1–0 in 18 seconds; one with closed builds; one
left), and Replays' steps with them in the folder listed them first, watched the newest, showed both builds of it
(its recorder played as P2, whose console is shown, so YOU was at the left) and none of the closed-builds one's.
Never run: a real pointer or a pad (the probe's are the window's events, not the system's), SHOW FILE, Save as's
system dialog, the main menu's reel playing a replay (its order is a unit test), the builds with a real key, and
anything on a real phone.
