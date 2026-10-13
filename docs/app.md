# The app (`nettai` and `nettai-ui`)

**Status (2026-10-12): the redesigned app's foundation.** The user asked for the app's UI to be rethought and
redesigned entirely, in Slint, and approved a design: [design/app-ui.md](design/app-ui.md) is that design (what
the user asked for, every decision, the look, the rules every screen follows, each screen). This document is the
app as it is built: how the battle is hosted in a Slint window (§1, §5), how the app is put together and how a
screen is added or filled in (§2), and the parts every screen shares (§3, §9 to §14).

What exists: the design system, focus and navigation, the keyboard, mouse, touch and gamepads, scaling and the
upright-phone switch, the languages, the errors and the log, Loading, the main menu with its reel and the title
music, Settings, the two cards before a battle, and the battle's host with its menu overlay. Training is built
(§15), and so is Replays (§16: the list, the viewer with its transport, the export). Play and Builds are
registered and reachable, each with its header and Back and an empty body: each is one Slint file and one Rust
module, to be filled in without editing a shared file (§2).

Builds is filled in (§8): the list with a build's overview, a new build, a save's import, and the editor with
its Navi and Folder tabs; its NaviCust, patch card and auto battle tabs are empty, behind the editor's tab
interface. The overview, a build's row and a version's tag are components any screen uses.

nettai is the player's host of nettai-frontend (docs/frontend.md: the battle it plays and draws); nettai-tools is
the command line with no window, for frames, audits and checks (docs/tools.md).

    NETTAI_PACKS=<packs> cargo run --release -p nettai

The packs are found as nettai-tool finds them (`$NETTAI_PACKS`, else `data`, each pack by the game it says;
written by `nettai-extract <exe4|exe5|exe6> <pack-dir> [ROM ...]`). What the player makes is kept in the app's
data folder: `$NETTAI_DATA`, else the system's (`~/Library/Application Support/nettai` on macOS,
`$XDG_DATA_HOME/nettai` or `~/.local/share/nettai` on Linux, `%APPDATA%\nettai` on Windows). There, builds are in
`builds/<game>/` (`$NETTAI_BUILDS`), sets are recorded to `replays/` (`$NETTAI_REPLAYS`), the settings are
`settings.toml` (`$NETTAI_SETTINGS`) and the log is `nettai.log` (`$NETTAI_LOG`). For netplay, `$NETTAI_SIGNAL`
names the signaling server (§4). Some variables are for development:

| Variable | What it does |
|---|---|
| `NETTAI_LANG=<code>` | The first language shown, instead of the settings' or the system's. |
| `NETTAI_TRAINING=<game>` | Starts straight into a training session of the game: new builds of its first two versions, the other a dummy that stands still. |
| `NETTAI_REPLAY=<file>` | Starts straight into watching the replay, its game selected; `NETTAI_REPLAY_SPEED=half\|1\|2\|4` at that speed. |
| `NETTAI_FFMPEG=<program>` | The `ffmpeg` a video is made with, instead of the one found (§16). |
| `NETTAI_PLAY_STATS`, `NETTAI_KEY_PROBE` | The latency figures of §5, in a battle, on the main menu and in a replay's viewer. |
| `NETTAI_WINDOW_PROBE=<minimize\|hide>:<after>:<for>` | Minimizes or hides the window `after` seconds from the start, for `for` seconds (§1, "The clock when the window isn't drawn"). |
| `NETTAI_NO_TIMER_CLOCK` | Leaves the timer's clock out, to see what the window's frames do alone. |
| `NETTAI_PHYSICAL_PIXELS` | Presents the picture at the display's density. |
| `NETTAI_NO_TEXTURE` | Shows the picture as a new image each time, not in the texture (§1). |
| `NETTAI_TOUR=<folder>` | The screenshot pass (§13); `NETTAI_TOUR_MOTION=1` its motion probe; `NETTAI_TOUR_ONLY=replays` one screen's steps alone. |

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
ms against 10.6, measured back to back). Not tested: the app's own window moved to another desktop (nothing
moves one without the user), a link's polling itself (no screen has one yet: it rides the same frames), and
other systems.

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
- in training, R (or F5) starts the set over, unless the player bound it to a button.

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
dialog of the shell's own: "EXE6's data is out of date. Extract it again." (`UserError::GameOutdated`; what the
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
                      lobby.rs netplay.rs builds/                       kept for the screens to come (§4, §8)
                      screens/<name>.rs

**A screen is one Slint file and one Rust module** (which may have modules of its own under it:
`screens/replays/`). Menu, Settings, Training and Replays are built; Play and Builds are their headers.
`ui/screens/play.slint` holds:

- `PlayView`, a global: the screen's data, which its Rust module sets (and may read back). A struct or an enum
  one of its properties uses reaches Rust by that use; nothing is added to app.slint for it.
- `PlayScreen`, a component that `inherits Page` (shell.slint): the header with its title and where Back goes,
  the footer's key hints, and the screen's own parts in the body.
- `PlayOverlay`: what the screen opens over itself (its dialogs, on a phone its sheets), over the whole window.
  (A screen that opens nothing has none, and no `if` for one in app.slint: Training.)
- For Training and Play, the menu over their battle (`TrainingMenu`, `MatchMenu`, each a `MenuOverlay`).

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
  both are 1). Then `open_menu`, `close_menu`, `end_battle`, and `battle` for the player itself
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

**Adding a language** is adding its folder: copy `lang/ja/` to `lang/<code>/`, translate each `msgstr`, and
build; the build lists what is left. `msgfmt --check -o /dev/null` validates a file.

**Typography.** Headings, buttons and labels are Chakra Petch, which has Latin alone; a language set in another
script (`lang::cjk`: Chinese, Japanese, Korean) has them in Murecho, with the labels' tracking nearly none.
Murecho covers Latin and Japanese. For a script it lacks, Slint falls back to the system's fonts on the desktop;
the web has none and a phone's may lack one, so such a language needs its font bundled (imported in theme.slint
beside the others).

## 4. Netplay

The Play screen isn't built. What is there for it:

- **The link** (`lobby.rs`, `LobbyState`): made in a room of nettai-rtc's signaling server (a code typed, or a
  new room, whose code the server picks) or directly (an address, or this machine hosting on UDP port 47474).
  It is the session alone: it shows nothing and reads no screen.
- **The agreeing** over it (`netplay.rs`: `Agreeing`, and `Framed`, the link taken for one match's channel) is
  the first app's way, kept compiling against the library. The library's `Lobby` now lives for as long as the
  two players stay together and carries their matches' frames (docs/frontend.md §2, "After a match"): the host
  runs one loop from the link opening until it is dropped, whatever is on screen, and the match's channel is
  `lobby.channel()`. `Screen::frame` and `Screen::frame_end` are that loop's two halves for the Play screen: feed
  every datagram to the lobby; then, after the battle's ticks, poll it and send what it has to send.
- **What a player is told** when it goes wrong: each layer's error is a kind and a detail, and
  `UserError::of_link`, `of_lobby` and `of_stop` give the kind's sentence and write the detail to the log (§12).
- **The connection's quality** in a match: the battle's host shows the library's four bars and the round trip
  in a corner, and "Reconnecting N s" while the link is down.
- **The input delay**: Settings keeps the player's (Online, 4 by default, 0 to 10: the library's range), for the
  lobby to start with (`NetOptions::input_delay`).

The signaling server isn't deployed. Run it locally:

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
  "Not an EXE6 save"; what the readers say in words goes to the log.

What the screens ask of the game is nettai-match's `build` (docs/frontend.md §6): the game's versions in the
order every list shows them, a build checked with its problems typed (`Checked`, `nettai_match::Wrong`), its
overview, the folder's view and the chips' rows and sorts. The game's builds data
(content/<game>/rules/builds) states each version's color beside it, and the game's strings name what a build is
made of (the text table `builds`: `version.<id>`, `<choice>.<preset>`, and `versions`, what a version sets).
Rust and Slint name no version, and hold no color of one.

**What any screen uses** (`crates/nettai/src/builds/`, with no screen's code; `ui/build.slint`):

| What | The UI's | The app's |
|---|---|---|
| A game's kit: its builds data, its pictures, its library's order, made once | | `builds::kit(shell)` (`Kit`) |
| A version's tag, in the version's own color | `VersionTag`, drawn by a `Pill` | `builds::version_tag(shell, game, ready, side)`, `Kit::version_tag` |
| The build overview | `BuildOverview { build: <BuildSheet>; }` | `builds::overview::sheet(shell, kit, label, name, side)` |
| A game's builds for a list | `BuildRow { build: <BuildEntry>; }` under a `ListView` of 52-tall rows | `builds::shelf::load(shell, kit)` (`Shelved`: the file, the build, whether it is ready, its row) |
| A chip's row | `ChipRow { chip: <ChipCell>; number: 1; }` | `builds::overview::chip_cell` |
| What is wrong with a build, in the player's words | the catalog's `BuildWords` | `builds::words::wrong(shell, game, ready, &Wrong)`: the kind's sentence with its numbers and names. `words::first_problem(shell, kit, side)` checks a side and words its first problem, for `UserError::BuildUnusable`. `builds::words::Words` (`problem`, `refusal`, `status`) where many are worded at once |

**A problem's sentence** has no full stop and is whole by itself: "2 Giga chips, limit 1", "Gregar is Gregar
only", "The Regular chip's slot is empty". It stands alone on a line of its own (the folder editor's strip) and
after a colon in another's sentence, which ends it (Training's reserved line: "Your build can't be used: 31
chips, a folder has 30."; the shell's "This build can't be used: {}."). `wrong` words any `nettai_match::Wrong`,
whichever screen is shown; `Words::wrong` matches the kinds with no catch-all, so a kind added to nettai-match
doesn't compile until it has its sentence.

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
  `BuildsOverlay` (the Folder tab's sheet on a phone).
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

NAVI and FOLDER are built. NAVICUST, PATCH CARDS and AUTO BATTLE are empty tabs behind this interface
(`navicust`, `patch_cards`, `auto_battle`: each names its facts and nothing else). **Filling one in** touches
its two files and `lang/ja/builds.po`. **Adding a tab** is those and a few lines: its place in `EditorTab` and
its `if` (builds.slint), its view's export (builds.slint and app.slint), its word in `BuildsView.tab-names`, and
its module in `editor::all()`.

`crates/nettai/src/builds/` also keeps, from the first app's creator, what the tabs to come take: the NaviCust's
grid (`grid`), the patch cards' lines (`cards`), the auto battle data (`auto`), a list's edits (`edit`) and the
library's order (`order`). Its `presets`, `layout` and `builds.toml` went: nettai-match's `build` is the one
source of what a game says of its builds.

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
a phone's edge, which is fixed and now said by the pass. Replays has since put `ListView`, `PanelBack`, a
`DialogFrame` of a screen's own with its scope, a `Field` outside a scrolling page, a view of a screen's own
(`nav.enter`) and a replay's viewer through `Play::Watch` in the pass, and its viewer's `set_sound_levels` was
checked by its samples (§16). Never run: `Shell::fly`, an online match's `gate`, `Shell::recorder`, a dropped
file, a real gamepad, an IME, a touch screen (the pad's fingers are put through the same entry by the screenshot
pass). Training was the first to show a `Message`, whose text sat in the middle of its line (a text with no
width of its own is as wide as its words, centered): it is at the line's left now.

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

`NETTAI_TOUR=<folder>` (`tour.rs`): the app walks its own screens, in each language bundled (English, Japanese,
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
own `write` and `compare`), with a function for the pass and one for the motion probe, each called from
`tour::walk` and `tour::motion_walk` with one line (`training::steps(&mut add, lang, size)`). A pair's second
shot may be given more to differ in than the two cursors (`training.rs`'s `Differs::Builds`: where the entry
shows the builds chosen; `replays.rs`'s `Also::Playback`: the picture, the bar and its two times), found from
where the screen's controls are. `NETTAI_TOUR_ONLY=replays` walks that screen's steps alone (Replays has none
for the motion probe).

The pass has nothing of the player's to go by: its settings are the defaults, and Training lists a new build of
each of the game's versions (and one that can't be used) in place of the player's builds. The builds the Builds
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
(a pair a focus move apart, and a pair a second of playback apart); five seconds on; the other console; the
export from the viewer; and on a phone the selected replay's sheet (a pair). A shot that follows a change of
the selected replay waits for that replay's still. `replays.txt` in the pass's folder says whether the deleted
file went and the list followed; that is said, not one of the pass's own checks. `NETTAI_TOUR_EXPORT=1` adds,
once (in English at the desktop's size), a real export through `ffmpeg` (the first replay's last round, at
twice the frame, into the pass's folder) and a second one stopped once some of it is written; `export.txt`
says how each ended and what was in the folder.

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
| Dummy: Action | Stands still, Moves around, Attacks at random | the library's `Opponent::in_fight`, from the next tick |
| Dummy: Custom screen | Picks no chips, Picks the first it can | `Opponent::on_custom_screen` |
| Display: Input display | Off, On | the app's own drawing, below |

A row is focused as a whole and changed at once by Left and Right (Enter steps it on), around from its last
value to its first. Under them RESUME, RESTART, CHANGE BUILDS (the entry again, with another arena) and QUIT TO
MENU. The menu opens where the focus last was in it. The dummy's setup and the input display are remembered
between runs (`training.action`, `training.custom-screen`, `training.inputs`); a session another screen started
begins as that screen says and keeps nothing of the dummy's.

*Adding a row* (the design's Your HP, Dummy's HP, Custom gauge, Chip draw, Recording and a second pad wait for
the engine, and aren't shown): a value of `TrainingOption` and its words in training.slint (`label`, `value`),
its place in `ROWS`, and what it does in `values`, `value` and `Training::step_option`. A group's title is a row
too.

**The input display** (`TrainingInputs`) is the app's own: two rows of ten buttons (the directions, A, B, L, R,
SELECT, START), the player's over the dummy's in their sides' colors, each lit while it is held. It reads what
each side's custom screen reads (`Battle::custom.sides[n].joypad.held`: this tick's buttons, not the fight's
delayed ones) after the frame's ticks, and tells the UI only when they change. It is in the picture's top right
corner, under the eighth the battle's own text takes there; on a phone, under MENU. It is drawn into a layer
and drawn anew only when a button changes (§5: drawn each frame it cost 0.3 ms a frame).

**On a phone** the entry is one column (the rows, the line, the cards side by side, the arena, START TRAINING at
the foot), the menu's rows have their label over their arrows and its buttons are one over the other, and in
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
  `play_out`: no picture, no sound), newest first, and its row gets its score as it is known. What was played
  out is kept while the app runs.
- *The still.* The selected replay's picture is a frame the thread draws a moment into its first fight, shown
  through the screen's `Picture` as any picture is (§1): a whole multiple of the frame, the text at the
  window's resolution.

**A replay that can't be played is a row, not a failure.** A file that doesn't read (`ReadError`), a replay
whose head names another engine or other content, one the library refuses as it is played out (`Refused`) and
one that plays another battle than it recorded each show their sentence in the row, in the error red, in
place of the opponent, the score and the length (`UserError::of_replay_read`, `of_refusal`: §12), and beside
the list with WATCH and EXPORT VIDEO unavailable; SHOW FILE and DELETE stay. Each is written to the log once.
A file that doesn't read names no game, so it is listed with every game's.

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
  isn't.
- *The builds.* A replay of a closed-builds match shows none (`Info::closed_builds`, which the view has as
  `builds-open`). Showing both for an open one waits for the Builds screen's overview: its place is marked in
  the viewer's page.

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
