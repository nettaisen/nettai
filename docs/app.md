# The app (`nettai` and `nettai-ui`)

**Status (2026-10-12): the redesigned app's foundation.** The user asked for the app's UI to be rethought and
redesigned entirely, in Slint, and approved a design: [design/app-ui.md](design/app-ui.md) is that design (what
the user asked for, every decision, the look, the rules every screen follows, each screen). This document is the
app as it is built: how the battle is hosted in a Slint window (§1, §5), how the app is put together and how a
screen is added or filled in (§2), and the parts every screen shares (§3, §9 to §14).

What exists: the design system, focus and navigation, the keyboard, mouse, touch and gamepads, scaling and the
upright-phone switch, the languages, the errors and the log, Loading, the main menu with its reel and the title
music, Settings, the two cards before a battle, and the battle's host with its menu overlay. Practice is built
(§15), and so are Play, the netplay UI (§4), Builds (§8) and Replays (§16: the list, the viewer with its
transport, the export): each is one Slint file and one Rust module (§2).

Builds is filled in (§8): the editor with its five tabs behind its tab interface: Navi, Folder, NaviCust, Patch cards and, in a game whose setup takes
it, Auto battle. The builds themselves (a new build, a save's import, the chosen one's overview, its menu of DUPLICATE,
RENAME and DELETE) are Play's (§4), and so is the way into Practice (docs/design/app-ui.md §2.50). The overview, a build's row
and a version's tag are components any screen uses.

nettai is the player's host of nettai-frontend (docs/frontend.md: the battle it plays and draws); nettai-tools is
the command line with no window, for frames, audits and checks (docs/tools.md).

    cargo run --release -p nettai
    NETTAI_PACKS=<packs> cargo run --release -p nettai

The app extracts each game's pack itself, the first time the game is loaded, from the player's Battle Network Legacy
Collection Vol. 2 (Steam's), into its data folder's `packs/`, and again when another version of nettai made it
(§17). With
`$NETTAI_PACKS` set, the packs are that folder's, found as nettai-tool finds them (each pack by the game it says;
written by `nettai-extract <exe4|exe5|exe6> <pack-dir> [ROM ...]`), and nothing is looked for or extracted: for
development and the screenshot pass. The game's content (its rules and data in Luau, docs/design/content-model-v2.md
§4.0) is the `content/` folder beside the program, where a packaged app ships it; a build run from the repository
finds none there and reads the repository's `content/`; `$NETTAI_CONTENT` names another. (A macOS app bundle
ships it in its `Resources/`; a packaged build has no fallback to the repository's, §19.) A release build on Windows
opens no console window (`windows_subsystem`); a development build keeps one, for what it prints. What the player
makes is kept in the app's
data folder: `$NETTAI_DATA`, else the system's (`~/Library/Application Support/nettai` on macOS,
`$XDG_DATA_HOME/nettai` or `~/.local/share/nettai` on Linux, `%APPDATA%\nettai` on Windows). There, builds are in
`builds/<game>/` (`$NETTAI_BUILDS`), sets are recorded to `replays/` (`$NETTAI_REPLAYS`), the settings are
`settings.toml` (`$NETTAI_SETTINGS`), what playing replays out gave is `replays-played.toml` beside it (§16), and
the log is `nettai.log` (`$NETTAI_LOG`). For netplay, the signaling server is `wss://matchmaking.nettai.n1gp.net`, or the one `$NETTAI_SIGNAL` names (§4). Some
variables are for development:

| Variable | What it does |
|---|---|
| `NETTAI_LANG=<code>` | The first language shown, instead of the settings' or the system's. |
| `NETTAI_PRACTICE=<game>` | Starts straight into a practice session of the game: new builds of its first two versions, the other a dummy that stands still, the Battle options (§15) as Practice last left them. |
| `NETTAI_REPLAY=<file>` | Starts straight into watching the replay, its game selected (once: a game that can't be played here, the shell says why and the replay is let go); `NETTAI_REPLAY_SPEED=half\|1\|2\|4` at that speed. |
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
app's, "a battle is running", during a practice battle), not by covering the window under load again. Windows
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

Measured on macOS (2026-10-12): a practice battle with the window taken away for 8 seconds, by
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

**The picture is shown only in a frame in which something was shown in it** (`stage::Showing`: the UI's
`Shell.picture-shown`, which every `Picture` follows). The texture keeps what was presented last, and every
`Picture` would show it: a screen with nothing of its own to present showed whatever had been there, at its
size and place. The user saw it ("if a replay rendering errors it ends up showing the demo video below"): Replays
with a replay that can't be played selected, come from the main menu, had the menu's reel in its box, spilling
past it over the facts under it, the row's error in the middle of it; a replay that failed as it was opened had
it under the error's dialog. Now each frame starts with nothing shown; a battle, the reel, a replay's still or its
viewer shown in the frame (`Shell::show`, `Shell::show_surface`) shows the picture, and a change of what the
window shows (another screen, a battle ended) takes it away in the frame it comes, whatever was shown before it
in that frame. A screen with nothing of its own shows its `Picture`'s ground. While a screen leaves (§10)
nothing presents (the room is still the leaving screen's) and the leaving one keeps what it showed: the menu's
reel goes with the menu, and a battle that ended took its picture away at once, so a screen leaving after it
shows none of it. And no picture is drawn past its `Picture`'s box (the box clips it), whatever size it was
presented at for the room before. (`stage::tests::the_picture_is_shown_only_in_a_frame_something_was_shown_in`; the screenshot
pass's `replays-unreadable` and `replays-watch-failed`, §16, each of which showed the reel before; a screen
leaving is the motion probe's, §13, whose `screen-to-settings` has the menu leave with its reel and
`screen-back-to-menu` has it come back with one, and whose `battle-entered` ends on the battle's picture.)

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

The density has two limits, since the CPU draws what is presented every picture (the user saw the enhanced
mode stutter on a 4K Windows display, where the picture was presented at 9 to 13 times the frame): the enhanced
mode's is at most six (`stage::MOST_DENSITY`, 1440 by 960, about a 1080p display's picture; but with
`NETTAI_PHYSICAL_PIXELS`), the window scaling the rest; and the original mode's is 1, the frame's own 240 by 160,
since it has nothing finer than the frame's pixels to draw: the window's nearest-pixel scaling makes the same
picture as the CPU's did.

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
- in a practice battle, R (or F5) starts the set over, unless the player bound it to a button;
- in an online match, Tab held shows both builds (§4): no button can be bound to it.

**The gamepads** are read through [gamepad-facade](https://github.com/tangobattle/gamepad-facade) (SDL3's
gamepads behind a small API; the user's choice) and polled each frame (`input::Pad`). Every pad plays the one
player:

- the buttons are bound as the keys are; the defaults are the south button A, the west button B, the shoulders L
  and R, Start and Back;
- in a practice battle, with the dummy's Action "A second pad", the second pad used plays the dummy instead (§15);
- the left stick moves whatever is bound;
- the Guide button opens nettai's menu, and in a practice battle the north button starts over.

In the menus the same presses navigate (§11), and the key hints switch to the pad's buttons once it is used.

**A terminate request closes the app** (`quit.rs`: the system's SIGTERM, and SIGINT, a terminal's Ctrl+C). It
did nothing before: the gamepads' library starts SDL, which takes both signals and turns each into an event of
its own queue, and the library hands on a pad's events alone, so only a kill ended the app. The app takes the
two signals itself once SDL has started (the last to take them has them): its handler writes a byte to a pipe,
and a thread waiting on the pipe ends the window's event loop on the window's thread, so `main` returns as it
does when the window's close button is pressed, and what the app holds is dropped in order (a recording's file
flushed, a link closed; the settings are written as each changes). A second request while it closes is the
system's own again. Run on macOS: the app ended with status 0 in 0.3 to 0.4 seconds, from the main menu and
from a practice battle, on SIGTERM and on SIGINT. Windows has no such signals and nothing is taken there.

**START never pauses the game.** In a fight the library reports START and doesn't act on it
(`Player::take_pause_requests`: docs/frontend.md §2), and what it opens is nettai's menu (`Shell::open_menu`):
the battle's owner's own, the practice menu or the match menu (§2). A battle that can stop is paused under the
menu; an online match keeps running, as the menu says. A practice battle is held under its menu, its music
going on and its scenery moving as the game's pause moves it (§15).

**The sound** is nettai-audio's `Output`, fed one stream at the battle's rate (`sound.rs`):

- The menus' sounds are the selected game's own custom screen sounds, through its rules' sound roles
  (`custom_cursor`, `custom_pick`, `custom_back`, `refused`): the app names no sound of a pack. Every move of the
  cursor, confirm, back and refusal has one. Entering a battle (the jack-in, §10) plays the game's own encounter
  sound, the role `encounter`: what every battle's start plays in the original as the screen breaks into it, a
  random encounter's too, song 0x78 in all three games (EXE6's `SOUND_BATTLE_START`, played by
  `gameState_8005BC8`, the battle's game state's entry; EXE5's same routine, 0x080054F6, and its same song;
  EXE4's 0x08005490, 0x080054EE and 0x08005520, which its walking encounter and its scripts enter by). It is a
  sound effect of 0.7 s (one note) on an effects player of its own, so it plays over the battle's music, which
  starts at once. No rule plays it; the packs name it `encounter` (`compat/assets.toml`). (EXE5's and EXE6's song
  0x0F, `transmission`, was looked at first and isn't used: the user, "it shouldn't be the transmission sound, it
  should be the encounter sound".)
- The title music is the selected game's too: the sound its pack names `title-screen`, on a driver of its own. It
  plays across the menu screens, fades in, fades out as a battle starts, and crossfades with the next game's as
  the game changes. Each game's content names its own (`content/<game>/compat/assets.toml`): the song its start
  screen's first state sets, song 1 in EXE6 and EXE5 and song 0 in EXE4, read from the routine in all twelve
  ROMs. A pack that names no such sound has none: one extracted before the names were given, until it is
  extracted again. (Practice's entry played the battle's music in its place, and handed it to the battle and back,
  until the entry went: docs/design/app-ui.md §2.57.)
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
  unchanged to the sample. Measured in a practice battle of each game: with Music at 0 the stream is silent
  once the opening effects end; at 50 the music's peak is half (0.30 to 0.39 against 0.6 to 0.7).

**The battle's music** (`music.rs`; docs/design/app-ui.md §2.29). The user: "you should also allow for
selectable battle themes (bn4 virus battle, bn4 boss battle, bn4 tournament battle, bn4 vs duo, bn5 virus battle,
bn5 boss battle, bn5 vs nebula gray, bn6 virus battle, bn6 boss battle, bn6 vs cybeast, bn6 crossover battle)".

- *What plays without a pick* (the game's own) is what the engine's round start asks for (`Battle::mode_intro`):
  every match is a link battle, so the rules' `music.link_battle` (EXE6 and EXE5: `virus-battle`, song 0x15), and
  in EXE4 a single battle's `link_battle_single` (`virus-battle`, 0x14) or a set's `link_battle` (`boss-battle`,
  0x15), as its battle start picks by the battle's type. The stage's own music is a battle's outside a link (none
  the app plays). The round's end stops it (`StopMusic`), and the result plays the winner's or the loser's music.
- *The themes* are each game's content: its rules' `battle_themes`, songs of its pack in the order the list shows
  them, named in its locales' text table `battle_themes` by the song's asset name (docs/design/content-model-v2.md
  §3.8). Each is the music the original's battle settings records start for a kind of battle (their music byte,
  found by the records' shape in each ROM, the same in every ROM of a game):

  | Theme | Song | The records that start it |
  |---|---|---|
  | BN4 Virus Battle | EXE4 0x14 `virus-battle` | 369 (the virus battles; a single netbattle's too) |
  | BN4 Boss Battle | EXE4 0x15 `boss-battle` | 52: the navis' later versions in the net, the DarkSoul's (a triple battle's too) |
  | BN4 Tournament Battle | EXE4 0x17 `tournament-battle` | 24, every battle at the three venues (backgrounds 0x18 to 0x1A) |
  | BN4 vs Duo | EXE4 0x16 `final-battle` | Duo's two (background 0x17, `duo`) |
  | BN5 Virus Battle | EXE5 0x15 `virus-battle` | 957 |
  | BN5 Boss Battle | EXE5 0x16 `boss-battle` | 141, the navis' |
  | BN5 vs Nebula Gray | EXE5 0x17 `final-battle` | NebulaGray's two (0x081140E0, 0x08114270) |
  | BN6 Virus Battle | EXE6 0x15 `virus-battle` | 201 of `BattleSettingsList0` |
  | BN6 Boss Battle | EXE6 0x16 `boss-battle` | 65, the navis' |
  | BN6 vs Cybeast | EXE6 0x17 `final-battle` | the two against a Cybeast (NameIDs 0x179 and 0x17B in Falzar) |
  | BN6 Crossover Battle | EXE6 0x22 `crossover-battle` | one (record 0); the disassembly's name for the song |

  The names shown are the game's name in the language, then the theme's ("BN4 Tournament Battle", "EXE4
  トーナメントバトル"). The app names no song and no game: it lists what the packs' contents state.
- *The pick* is the player's own and one: the practice menu over the battle (§15) shows and changes it (Play's
  lobby has none: the user, "don't allow bgm select in lobby"). It is kept for one visit to Practice (`practice::Visit`, `Shell::battle_music`; §15) and each visit starts on "Game's own" (the user, "going
  into practice should reset all the arena settings, there's no reason for it to remember the last settings";
  docs/design/app-ui.md §2.40): nothing of it is in the settings. The list is "Game's own", then the themes of every game whose parts are read
  (`Games::read`: its content and its sound; no runtime is needed), the games in their order; a game whose pack
  isn't there offers none, and a pick of one is played as the game's own until its game is read.
- *Which battles play it* (`BattleStart::music`, `BattleMusic`): Practice's own sessions play the pick (the
  user: "why does the music setting carry over from practice mode into pvp mode? huh?"); an online match, a replay
  watched and a replay's video (§16) play the game's own ("for pvp it should use the game's default for now");
  and the practice battle beside a waiting invite plays none of its own: the menus' music goes on under it, its
  battle music silent ("in the practice mode while the lobby is waiting, don't switch the music").
- *How it plays.* A battle that plays the pick has it from `Shell::battle_music` through `music::sound`. The song plays in
  place of the battle's own music (the content's `battle_music`: the link battle's music roles and the stages'
  own), with its own game's bank on a driver of its own (`nettai_audio::with_theme`: its own voice groups and
  samples, its own mixer; one of the battle's own game is played on the game's own driver, as its music is). The
  battle stays its game's, one game a match: its effects and its result's music are its own pack's, and the
  result's music stops the theme as it would the game's. The battle, its cues, a replay and the wire are the same
  whatever plays: nothing of the pick is sent, and each player of an online match hears their own.
- *What the engine does to the music applies to the theme*, being the music: the round's end stops it, a new
  round starts it again, the result's music replaces it; BN6's low HP (`Pinch`: a semitone up and tempo 0x11A
  while the console's navi is at a quarter of its HP or less) is the battle's rule, so a BN6 battle plays any
  theme so and a BN4 or BN5 battle none (their rules have no low-HP music); the custom screen's hover over a dark
  chip turns the music player down and back (`Volume`, set in every pack's driver as one driver's would be).
  Rollback takes a theme's change back as the game's own: a result's music played on a wrong prediction stops,
  and the theme starts again on its own driver.
- *A difference from the original:* a theme of another game has a mixer of its own, so the battle's effects don't
  take its channels as they take the game's own music's.
- *The battle's music starts with the battle* (docs/design/app-ui.md §2.57): a practice session's battle starts
  its music at its round's start as any battle does, the title music having faded as the battle is made ready,
  and Play's title music comes back after. (Practice's entry played the session's music before the battle and the
  battle took it over, and back at the entry the entry took the battle's over, §2.29, §2.40, §2.43 and §2.45; the
  entry went, and with it the menus' preview of a battle's music, `Sound::set_preview` and the battle's sound
  carried across, nettai-frontend's `Player::carry_sound` and `take_sound`, nettai-audio's
  `BattleAudio::go_on_with` and `keep_music_alone`.)
- *After that, the battle's as ever:* a round's end stops the music (`StopMusic`) and the next round starts it
  from its beginning; RESTART, and a set that ends and starts over, start the set again, whose round start finds
  the music still playing (on through) or stopped after a round's end (from its beginning).
- *The practice menu changes it while the battle runs* (the user: "also you should be able to change the bgm in
  the practice menu"): its Stage rows (Battle rows until docs/design/app-ui.md §2.56) have Music, the same pick. A change switches the battle's music at once
  (`music::retheme`, `Player::set_sound_parts`, nettai-audio's `BattleAudio::set_songs`): where the battle's own
  music is playing, the new song starts from its beginning on its own pack's driver (the one before stops, that
  song alone: the effects play on), with BN6's low-HP speed-up kept on where it is on; where the result's music is
  playing, or the round's end has stopped the music, nothing changes then, and the next round starts with the new
  song. A pack the battle hadn't is added at Settings' levels. The battle, its cues and its rollback are the same.
  (A volume the custom screen's hover over a dark chip set before the change isn't carried to a pack the change
  adds; the next one the screen sets is every pack's.)

**The battle's background** (`backgrounds.rs`; docs/design/app-ui.md §2.31). The user: "is it possible to use
e.g. exe4 backgrounds in exe5? as in if they all came from the same pool of backgrounds?", then "sure do that";
and of whether it stays the match's or becomes each player's own, "Shared, as now".

- *The pool* is every background of every game the content has, each by its game and its name in its game's
  pack (`Content::background_pool`: EXE4's 27, EXE5's 22, EXE6's 21, the games in their order and each game's
  by name). It is read from the content alone, each game's own language's `[backgrounds]` (which names every
  background its pack has, and none it hasn't: the content check holds it to that), so it is the same for every
  player whatever packs they have. No tick reads a background, so it is left out of the content's hash (as the
  strings are) and a game's content is the same whatever other games stand beside it; two peers compare their
  pools on their own (below).
- *Its names*: the game's name in the language shown, then the background's from that game's locales ("BN4 ACDC
  Area", "EXE4 秋原エリア"), read from the content alone at start-up and when the language changes.
- *A round's background* is a handle over the round's backgrounds, the game's own first at the handles they
  always had, then the pool's others (`BackgroundId`); a match names another game's with its game,
  `exe4:acdc-area` (nettai-match's `ids::background`), and its own by its name, as before. Practice's Background
  row offers the whole pool (§15), and a background left to the seed is drawn from the pool, each as likely
  (`BackgroundPick::Pool`): Practice's Random and every round of Play's matches (`screens::play::settings`).
  Where the original's own pick is asked for, it stays the game's rule (`BackgroundPick::Game`: its rules'
  `link_pick.backgrounds`, EXE6's `byte_8120A20`): the library's lobby and match files unless they say
  otherwise, `pick::live`'s random matches, the lab's replays and traces (their backgrounds stated) and the
  engine's tests. Either draws one number for each round's background, so the arenas are the same whichever.
- *Shared*: the lobby's settings carry what a background left to the seed is drawn from and a round's
  background by name; the seed (from both nonces) picks it alike on both peers; the lobby's compatibility
  fields carry the pool's identity beside the content's hash, and two peers whose pools differ are refused as
  other content are (`Reason::DifferentContent`). A replay keeps each round's background by its game and its
  name (docs/frontend.md §8).
- *Drawing it*: the renderer draws another game's background from that game's own pack (`OtherBackgrounds`,
  read with the game's graphics from the other packs found: a few kilobytes each). In all three games a
  background's tiles start at tile 1 and end below the field's (163 on), and its map, its palette and every
  palette animation and shift use background palette 0 alone (the field's are 1 to 8): so another game's
  draws in any battle as in its own, its scroll and its animations its own data's, and the battle's dimming
  (palettes 0 to 8 a quarter down) and palette flash (the stage's nine palettes white, before or after the fades
  as the arena game's rules order it) take it as they take the game's own. A round on another game's
  background whose pack the player hasn't is drawn on its stage's own background, the one the arena game
  itself would draw (`Packs::round_background`). Looked at: an EXE5 battle on EXE4's ACDC Area (its tile
  animation over frames, EXE4's own battle beside it, the same picture and colors) through Colonel's dimming
  and flash, and with EXE4's pack taken away (EXE5's own `comp`); an EXE6 battle on EXE5's
  NebulaGray (its speeding scroll and palette animation) through EXE6's Colonel.

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
gone (`Games::take_notice`, asked each frame by `App::follow_games`). A game with no pack at all is passed over;
with the app's own packs, that is said once, with the way to the Legacy Collection's folder (§17). Before a game
loads, its pack may be extracted: the bar is that first, "EXTRACTING BN6" (§17). Another language's graphics are
loaded on a thread too
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
                      bnlc.rs packs.rs                                  the packs, from the Legacy Collection (§17)
               build.rs                                                 the software it is made of, for About (§18)
                      builds/                                           what any screen shows of a build (§8)
                      screens/<name>.rs
                      screens/play/session.rs tour.rs                 Play's link and lobby; its part of §13
                      screens/play/list.rs                           what Play does with the builds (§4)

**A screen is one Slint file and one Rust module** (which may have modules of its own under it:
`screens/replays/`, `screens/settings/`). Menu, Settings (with About, its subscreen: §18), Practice, Play, Builds
and Replays are built. `ui/screens/play.slint`
holds:

- `PlayView`, a global: the screen's data, which its Rust module sets (and may read back). A struct or an enum
  one of its properties uses reaches Rust by that use; nothing is added to app.slint for it.
- `PlayScreen`, a component that `inherits Page` (shell.slint): the header with its title and where Back goes,
  the footer's key hints, and the screen's own parts in the body.
- `PlayOverlay`: what the screen opens over itself (its dialogs, on a phone its sheets), over the whole window.
  (A screen that opens nothing has none, and no `if` for one in app.slint: Practice.)
- For Practice and Play, the menu over their battle (`PracticeMenu`, `MatchMenu`, each a `MenuOverlay`); for
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
| `activate(id, index)` | Enter (the pad's A), a click or a tap on a control. |
| `step(id, index, by)` | Left or Right on a stepper, a select or a switch, or a click on its arrow. |
| `choose(id, index, choice)` | A choice made outright: one side of a switch clicked, a slider set, a row of a select's list. |
| `edit(id, index, text, done)` | A field's text as it is typed, then as it is left. The screen keeps what it wants of it and sets its view's property, which the field then shows. |
| `focused(id, index)` | The focus moved to a control (a list's row: the overview beside it follows). |
| `key`, `button` | A key or a pad's button the screen may want for itself, before the menus read it (a key being bound; R in a practice battle). |
| `action` | Previous and next (the shoulders, the keys bound to L and R), delete, the pad's north button. The shell changes the game with the first two unless the screen takes them. |
| `scroll` | Up, Down, Page Up, Page Down, Home, End (keys, D-pad, stick), before the focus moves: for a page of the screen's own that scrolls with no control in it to move to (Play's build sheets in the match's menu). |
| `back` | Esc or Back: true if the screen went back within itself; false and the shell goes to the menu. |
| `answered(tag, choice)` | A dialog the screen asked was answered; or something it asked of another screen was (`Shell::tell`: a row it added to the practice menu was chosen). |
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
  has returned (how Practice tells the screen that added a row to its menu). With `go`, it asks another screen
  for something: `screens::builds::open(shell, from, at)` goes to Builds and tells it which screen opened it and
  which build to open in its editor (§8); `screens::practice::open(shell, game, build)` has Practice go straight
  into a practice battle on a build (§15).
- *A replay:* `start_battle` with `Play::Watch(replay, side)` makes its viewer ready behind the same cards and
  hands it over (`BattleEvent::Watching`); the screen shows it in a `Picture` of its own with
  `show_surface(surface, present)` and plays its sound with `play_samples`, having given the viewer the Music
  and Sound effects levels (`viewer.set_sound_levels` with `sound.battle_levels()`: §1, "The sound").
- *The rest:* `settings` (and `settings.remembered`, a map for what a screen remembers by its own keys, so it
  adds no field) with `save_settings()`; `set_name`; `lang`; `sound.play(Cue)`; `copy(text)`;
  `choose_file`, `choose_save`; `recorder` for an offline set's replay; `fly` (§10); `stay_awake`, set while
  the screen has a link open: the app isn't napped, and its frames run by a timer when the window isn't drawn
  (§1).

**A practice session, from any screen** (`crate::screens::practice`, §15). A practice session is a battle
against a dummy with the practice menu behind Start and Esc; Practice starts one from its entry, and another
screen may start one beside something of its own (Play, while an invite waits for its opponent). Three calls:

    use crate::screens::practice::{self, Build, Start};

    // Start one. `Start::new(game, you, dummy)` is Practice's own (the two cards, the picture filling the
    // window); `Start::beside(game, build, from, tag, rows)` is one a screen shows in a `Picture` of its own:
    // the dummy a copy of the player's build, standing still, no cards, and `rows` at the top of the practice
    // menu. The fields are public: `opponent`, `seed`, `full`, `cards`.
    let rows = vec![copy_invite_code, cancel_invite];           // their texts, in the language shown
    practice::start(shell, Start::beside(&game, Build { name, side }, ScreenId::Play, "invite", rows))?;

    // A row it added was chosen: the screen hears it as a dialog's answer, by its tag and the row's place.
    fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) { /* "invite": 0 copy, 1 cancel */ }

    // End it from outside (the opponent joined). `practice::running(shell)` says whether one is on.
    practice::end(shell);

- `start` gives `Err(UserError)` if the game isn't ready or a build can't be played (the detail is in the log);
  both builds must be ready (`nettai_match::build::Checked::ready`). A session already on ends first.
- The session's battle is always Practice's (`ScreenId::Practice` owns it), so Practice's module restarts a set
  that ended, hears the menu (`menu.*`) and the restart key whatever screen is shown, and the starter writes
  none of that. The starter's own controls stay its own: a control is heard by the battle's owner only if it is
  the battle's (its name begins `menu.`) or the battle fills the window (`App::hearer_of`).
- A session started from outside has no EXIT: its starter's rows are the ways out.
  Choosing an added row tells the starter and leaves the menu as it is (the starter closes it with
  `shell.close_menu()` if it wants, or ends the session). Its menu is otherwise the usual one (the user:
  "practice menu in the matchmaking waiting screen should be the same as the usual practice menu"): `start`
  gives it the game's builds that can be played for the Dummy's build, the dummy on the player's own at first;
  it has no Music (its music is the menus') and no EDIT (the editor would leave the starter's screen, and Play
  gives up its link as it is left: `shown_rows`). What its menu sets is kept for the starter's wait, never for a
  visit to Practice: `practice::stop` ends the session for a while keeping it (a phone's STOP PRACTICE), the
  next session the same screen starts begins on it, and `practice::end` (the wait over) forgets it.
- One shown in the starter's own `Picture` (`full` off) lasts while that screen is shown: it ends by itself if
  another screen is gone to, and when another battle starts in its place. The keys and a gamepad are the
  battle's there as anywhere; the phone's pad is drawn by the battle's own screen, which only a battle that
  fills the window has, so on a phone such a session has no pad unless its screen fills the window with it.
- A dialog asked while it runs (`Shell::ask`) is answered to the battle's owner, as ever: Practice passes the
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
script (`lang::cjk`: Chinese, Japanese, Korean) has them in Corporate Logo at 700 (the user's choice: docs/design/app-ui.md
§2.51), with the labels' tracking nearly none, in every game. Body text is Murecho in either. Corporate Logo and
Murecho cover Latin and Japanese. For a script they lack, Slint falls back to the system's fonts on the desktop; the web has
none and a phone's may lack one, so such a language needs its font bundled (imported in theme.slint beside the
others).

## 4. Netplay

Play is the netplay UI (docs/design/app-ui.md §6: "Play", "In an online match", "After a match"):
`ui/screens/play.slint` and `src/screens/play.rs`, with the link and the lobby in `screens/play/session.rs`.

**The screen has three parts**, one shown at a time (`PlayView.part`), and two things over a match:

- **The builds** (docs/design/app-ui.md §2.50: Play and the Builds list made one; §2.52: in three zones; §2.57:
  a build and the ways to play it). + NEW BUILD over the player's builds of the selected game as a list
  (`play::shelved`: `builds::shelf::load`, its `BuildRow`s, each read from its file and checked by the game's
  rules; one that can't be played says how many problems it has, in the error red); the chosen one's overview
  beside it (the wide sheet, §8) with PRACTICE, EDIT and ⋯ in its header (`BuildOverview.tools`, the room it
  leaves them); and under the overview, one row of the app's controls (40 tall): at its left edge HOST
  (`invite`, the main action's look while there is a build: an invite, as CREATE INVITE was), then the code's
  field (`code`, 208 wide, "Invite code"; Enter in it joins, as JOIN does, and Enter in the direct connection's
  address connects: `Screen::submitted`) and JOIN (`join`), and at the overview's right edge DIRECT CONNECT
  (`direct`, a regular button), the direct connection's dialog (§2.57: the user found the field taking a code
  or an address worse). The reserved line is under the row, and the list and the overview grow down to it (564
  at the reference window). The chosen build is the one under the list's cursor, remembered by game; the
  overview follows it. What is done with the builds is `screens/play/list.rs`: + NEW BUILD and RENAME open the
  name's dialog (`NameDialog` in `PlayOverlay`, its scope `name.`), a new build being the game's own
  (`Builds::new_build`); the new build's dialog's FROM A SAVE FILE… (`name.save`) closes it and makes a build of
  a save chosen in the system's dialog, as a save dropped on the window does (`builds::import`), saying quietly in
  the reserved line what it was put on where that changed it ("Imported as Dark MegaMan"); ⋯ (`more`) and a right
  click on a row (`row-menu`) open the build's menu (`Shell::pick_menu`, its choice heard as `more`'s): DUPLICATE,
  RENAME and DELETE, or DELETE alone for a build whose file doesn't read; DELETE (and Delete on the list) asks
  once, Cancel focused; EDIT opens the build in Builds' editor (`screens::builds::open(shell, ScreenId::Play,
  at)`), whose Back comes back here with it still chosen; PRACTICE goes straight into a practice battle on it
  (`screens::practice::open`, by the build's name), whose EXIT comes back here too (§15). With no build of the
  game + NEW BUILD is the one main action (HOST takes the normal look), the focus is on it, PRACTICE, EDIT and ⋯
  aren't there, and HOST and JOIN say "Make a build first." when pressed (`UserError::NoBuild`); a build that
  can't be played, why. Right from the list goes to PRACTICE and Tab goes in reading order (`Screen::route`,
  §11). On a phone the list is one box of the chosen build, which opens the builds as a sheet, PRACTICE, EDIT
  and ⋯ in a row under it, then HOST the column's width (52 tall), the field and JOIN under it, DIRECT CONNECT
  the column's width under them, and no overview. The field keeps what is typed as a code: capitals, a code's
  six characters, shown in threes, whatever it was typed with (`code_typed`, `code_shown`). A pad can't type:
  its confirm on the field opens the code as six boxes (docs/design/app-ui.md §3), where Left and Right go
  between the boxes, Up and Down through a box's characters (the alphabet codes are written in), and a
  character typed goes into the box and on to the next. (A box is a list to the app's focus, its rows all in
  one place: `CodeBox`, `Nav::set_row`.) DIRECT CONNECT opens a dialog (a `DialogFrame` in `PlayOverlay`, its
  scope `direct.`, opened with the address edited): this machine's address with COPY, and HOST (this machine
  listening on `session::DIRECT_PORT`); an address's field, and CONNECT, which takes a host or an IP and its
  port, an IPv6 one in brackets (`play::direct_address`; anything else `UserError::BadAddress`, said under
  the field).
- **Waiting** for the other player: the invite's code (a direct host's address, with "Send this address to your
  opponent." under it), large, in fixed-width type, copied to the clipboard as the wait starts (COPY CODE
  flashing as though pressed; not in the screenshot pass), with COPY CODE (COPY ADDRESS) and CANCEL, and a practice session beside it in a `Picture` of the screen's own. The session
  is Practice's (`practice::start` with `Start::beside`, §2: the player's build against a copy of it that
  stands still; Practice owns its battle, starts its sets over and hears its menu; the usual practice menu but
  for Music and EDIT, its choices lasting for the wait), with CANCEL INVITE (STOP HOSTING for a direct host) in
  EXIT's place (`Start::exit`, heard as `Screen::answered(tag, practice::EXIT)`) and, on a phone, STOP PRACTICE
  as the one row at the menu's top (`Start::rows`, heard by its place). The keys are
  the battle's, so the cursor is on none of the panel's buttons (the pointer clicks them), and Esc or Start
  opens the practice menu. The session ends when the other player comes (`practice::end`) or the wait is
  given up.
  *On a phone* a battle has its pad only where it fills the window (§2), and a battle to watch is no practice:
  so there the session isn't started with the wait. The invite shows with a PRACTICE button under it; the
  session then fills the window, pad and MENU under the picture as in any battle, and its menu has a third
  row of Play's, STOP PRACTICE, which goes back to the invite (`practice::stop`: PRACTICE again starts on what
  its menu was left at). The other player coming ends it for the lobby as on the desktop.
- **The lobby**, for as long as the two players stay together: the two cards (this player's says their build;
  the other's says their name and whether they are ready, and nothing else: the lobby's protocol carries
  nothing of a build), one row of settings (First to, Ruleset and Closed builds are the match's: both see
  them, and a change clears both ready marks; either player changes First to and Closed builds, the host alone
  the Ruleset; Input delay is the player's own, and is the
  Settings' value, kept as it is changed; no music: the battle's music is picked in Practice, §1; no
  background either: every round's is drawn from the background pool by the match's seed, alike for both
  players, §1, "The battle's background") with the
  connection's bars and round trip at its end, READY, CHANGE
  BUILD (a list of the builds, in a dialog; nothing of the new one is said to the other player) and LEAVE, and
  a line that is there whether or not it says anything: who is waited for, "Reconnecting N s" while the link is
  down, or why the match before ended. In the row each setting's name is over its control (`LobbyFacts`): with
  the names beside them, as the mockup has three, six cells don't fit the window in a language a third longer
  than English. On a phone each is a line, its name at the left. Left and Right change a setting, so the row
  can't be walked with them: Tab does, and in the lobby so do the shoulders and the keys bound to L and R
  (`Screen::action`). **The Ruleset** (docs/design/app-ui.md §2.59; it took Patch cards' place) is the host's
  select of the game's presets and then the player's own rulesets by name (`LobbyFacts`' `lobby.ruleset`,
  `rulesets::pickable`; Left and Right step it, Enter opens it as a list; `Lobby::set_ruleset`), with EDIT
  (`lobby.rules`), which opens the ruleset's page over the lobby (§8, "Rulesets"; the lobby and its link go on
  under it, and a change made there is the match's at once). The joiner's select can't be changed; it shows the
  host's ruleset by the preset's name where its rules are a preset's, else as "Host's ruleset" (the game's strings'
  `[rulesets] theirs`): a ruleset's name stays with its player, and the lobby carries its rules alone (the user:
  "ruleset name shouldn't travel to the opponent"; `ruleset::Name::Theirs`, `Rulesets::named_here`). Its VIEW opens
  the page to read (its DUPLICATE makes the player's own copy, which no longer follows), and the page follows the host's changes as they come (the user: "show live edits of host's ruleset"; `Page::follow`). A host's lobby starts on the ruleset their
  build is made for (`Shelved::made_for`; else `rulesets::chosen_last`), and the one the host picks is remembered as
  the player's last choice (what a new build is made for: §8). The lobby checks this
  player's build under the ruleset whenever either changes (`Lobby::problems`, typed); while it
  has a problem READY is refused (`Lobby::set_ready` says so, and the mark is the lobby's answer) and the line
  says what is wrong with it, in the error red. So CHANGE BUILD takes any build whose
  file reads, and the lobby is what says one can't be played. **First to** (the user: "instead of "rounds" you
  should have "first to" then you should be able to select 1, 2, 3, 4 etc") is a stepper of the round wins that
  take the set, 1 to 10 (`nettai_match::MAX_FIRST_TO`), around from 10 to 1 and back, Enter a step on: 1 is the
  original's single battle, 2 its triple battle (the first lobby's, until a match is played: then the last
  match's, remembered as `play.first-to`). The number alone in English; in Japanese 先取, each value "2本先取". A
  set first to `n` is `2n − 1` rounds at most, decided as the original decides its triple battle: once a side
  can no longer be caught, else after its last round by the wins (drawn if level), a drawn round being one of
  the rounds (docs/frontend.md §2). Ten is the top: the longest set fighting games' players commonly play (the
  FGC's "first to ten", "10先"), 19 rounds at most, each numbered by the round's banner in its two digits
  ("BATTLE 10 START!": the games' own banner, digits and all, in every game). The lobby carries the number
  (`Settings`, its byte in the lobby's message and in the Hello).
- **The match menu** (`MatchMenu`: Start or Esc in a match, which goes on behind it): BACK TO MATCH, YOUR BUILD,
  THEIR BUILD (not with closed builds), CONTROLS (what is bound to each button, to read), SETTINGS (the input
  delay, which changes at once, `Player::set_input_delay`; the two levels of the sound), LEAVE MATCH.
- **The link panel** (`LinkPanel` in battle.slint; its figures `battle/link.rs`): a click on the connection's
  corner opens it over the corner (`Shell.connection-open`, which the corner alone toggles: "telemetry window should not
  dismiss on click outside, it should only be dismissed if the ping indicator is clicked again"; the app clears it
  as a battle starts and ends). The figures are the player's `net_status`; the
  app samples its round trip ten times a second for the whole match and, while the panel is open, sets
  `Shell.link` at each sample: the figures, and the last ten seconds as two paths (the graph's line and the area
  under it).
- **Both builds** (`MatchPeek`, which app.slint shows over an online match): while Tab is held (`Keys::tab`), or
  the pad's button bound to SELECT, the player's build on their side and the other player's on theirs; with
  closed builds the player's own alone. The match takes its keys meanwhile.

Where a build is shown whole, it is the build overview (`BuildOverview`, filled by
`builds::overview::sheet`, §8): beside the list, the wide sheet of the build as it is (made again only when
the build chosen or the language changes); a player's panel in a match, on its YOUR BUILD and THEIR BUILD
pages and while Tab is held, the narrow sheet of the agreement's side, which is the build as the match plays
it (without its patch cards where those were off), under the player's name. The menu's page is shorter than
the sheet at the reference size, so it scrolls: by the wheel and a drag, and by Up and Down (a step of two
chips' rows), Page Up and Down, Home and End, the pad's too, which the screen takes before the focus would
move (`Screen::scroll`: the page has no control in it to move to, Back is under it). On a phone a sheet is a column taller than the window: the menu's page scrolls it, and with Tab held each
of the two is half the window, its top (the navi, its stats, abilities and bugs). This player's card in the
lobby says the build's name, navi and version (`Lobby::side`).

**The link and the lobby** are one `Session` (`session.rs`), and the screen runs the library's one loop over it
(docs/design/rollback.md §4.7, "What a host does") in its two frame halves, whatever is on screen:

- `Screen::frame`, before the battle's ticks: `Session::feed` gives every datagram the link has to the lobby, a
  match's frames too, and tells the lobby how the link is. Before the link is open, it keeps the link up.
- `Screen::frame_end`, after them: this side's loading is said (`set_loading`, `set_loaded`); `Session::poll`
  says how the lobby stands, which moves the screen on; `Session::send` sends what the lobby has to say and the
  frame the match's player made this frame.

A link is made by `Session::open`: an invite (`Link::create_room`: the server picks the code, and the wait
begins when it has said it), a code typed (`Link::join_room`), a direct host (`Link::host`, UDP port 47474) or
a direct host's address (`Link::join`). Joining an address resolves the host's name, which may wait on the
system's resolver, so that link is made on a thread of its own; the others are made at once (a room's link
waits on nothing: its STUN and TURN servers come with the server's welcome, and the link resolves their names
on a thread of its own). `Session::open` says at once what needs no lookup: no server named, a code that can't
be one, a port that can't be listened on. Meanwhile the space beside JOIN, or the dialog's under CONNECT, says
the link is being made, and the loop goes on; a host's name that doesn't resolve is that address's error, and a
signaling server that can't be reached is the server's. A STUN or TURN server whose name doesn't resolve is left out,
as the link library has it (a link on one network needs none), not a failure. The lobby is made the moment the link is open, in the link's role, with
a timeout of 45 seconds (longer than the link's 30 of making a dropped connection again), and the practice
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
  (the winner's banner, then the picture fades out). Then nettai's result is over the picture (`MatchResult`,
  docs/design/app-ui.md §2.49; `Play::show_result`): won, lost or drawn, the round wins each side has
  (`Battle::wins_for`, the round's count by side: the same on both peers, as it is the shared simulation's) and
  the match's time (the frames its player ran, `Player::ticks`, at the game's rate), which the frontend already
  has: nothing is added to the battle or the content for it. While it shows it has the keys
  (`Shell::cover_battle`, its controls' names beginning `result.`): CONTINUE (Enter, the pad's A; Esc and B too)
  settles it where it still moves, and pressed again goes on; it goes on by itself after six seconds
  (`RESULT_STAYS`). Its sounds are the app's: the UI counts the numbers' ticks (`PlayView.result-ticks`) and says
  when the last is done (`result-done`), and Play plays the game's cursor sound for the ticks, at most one each
  45 ms, and its confirm sound for the last. Each player goes on alone: the other's lobby hears this one back
  (`Lobby::their_reopened`) and goes on showing its result, and nobody has left (a player dropped after its set
  is over on the lobby's channel says nothing of leaving: `NetPlayer`'s `Drop`). A set that ended some other way
  (no result nettai has words for) shows none, and the lobby comes half a second on, as before.
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

**What is wrong with a build** is said in the builds' words for its kind (`builds::words`): beside JOIN
before a link (`words::first_problem`, as `UserError::BuildUnusable`'s reason), and in the lobby's line
(`words::problem` of the first of `Lobby::problems`, which names the part of the build it is about): "This build can't be used: 28 chips, a folder has 30."

**Not done.** CONTROLS in the match menu shows the bindings and doesn't change them.

**What has run, and what was only written** (2026-10-12). Between two players in one process over real links
(the screenshot pass's second player, §13), in English, Japanese and the pseudo-locale at both sizes:

- an invite made, and joined by its code; a code no match has, refused by the server; a direct host joined at
  its address; the other player's invite joined by its code, typed in small letters;
- the wait's practice session: started beside the invite (on a phone, by its button, filling the window), its
  menu's STOP PRACTICE (a phone) chosen, and the session ended by the
  other player coming;
- the lobby's First to (stepped from 2 to 3, `play-lobby-first-to` and `-3`, a pair, then back to 1 for the
  match played), the host's Ruleset stepped from Masters class to Senior class (`play-lobby-ruleset`, `-senior`) and closed
  builds changed, each taken by the other player; the input
  delay changed; another build chosen in the lobby that the settings can't play (READY refused, the line
  saying why), and the first again;
- the link dropped for six seconds in the lobby and in a match, and made again;
- a match agreed, loaded with the other side held at 74%, and played to its end (the game's own "WIN" banner,
  then the lobby again, nobody ready); a second match on the same link under Mixed and with closed builds,
  left by the other player; a match as the joiner, on the right navi's console, left by LEAVE MATCH;
- the other player bringing a build that breaks a rule (its lobby won't make it ready, and this player waits,
  ready); the other player leaving the lobby; this player leaving it.

The three matches of a pass, recorded by their agreements (a set played to its end, a match with closed builds
the other player left, a match this one left as the joiner), were played back by nettai-tool
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

The signaling server is `wss://matchmaking.nettai.n1gp.net` (`DEFAULT_SIGNAL` in `session.rs`; the user:
"default signaling server should be matchmaking.nettai.n1gp.net"), `signaling/`'s Worker once its owner has
deployed it there (rollback.md §4.8). While it can't be reached, HOST and JOIN say so, and DIRECT CONNECT's
dialog works. `$NETTAI_SIGNAL` names another instead, such as one run locally:

    cd signaling && npx wrangler dev        # http://127.0.0.1:8787
    NETTAI_SIGNAL=ws://127.0.0.1:8787 cargo run --release -p nettai

A room's link finds its way through NATs on the STUN and TURN servers the signaling server's welcome gives
(Cloudflare's STUN server, and its TURN service's with credentials of a few hours, when the Worker has its TURN
key; the STUN server alone when it hasn't, and then two players whose NATs need a relay can't meet), and checks
the certificates its descriptions name. A direct link asks no STUN or TURN server (it says no candidate to
anyone) and authenticates nobody, as plain UDP doesn't. Over the Internet, a direct host forwards UDP port
47474 on its router to its machine.

## 5. Latency and smoothness

**Slow frames in a battle are logged** (`shell.rs`, `SlowFrames`): a frame drawn more than 40 ms after the one
before (and less than a second: past that the window wasn't drawing at all) writes a line to the log with how long
the app's part of it took (the battle's ticks and picture, the screens), how long the window's drawing took, and
whether the custom screen was up; forty at most a run. The user asked "why does entering custom for the first time
stutter sometimes?", and the battle's own work on a custom screen's first frames, timed tick by tick in a release
build (simulation, picture with its text, sound; EXE6, three seeds, a 1200x800 picture), was 2 to 6 ms a frame:
whatever stutters there is the window's drawing or the app around the battle, which the log tells apart.

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

**The redesigned app** (2026-10-12, the same Mac and window, `NETTAI_PRACTICE=exe6` with both variables):

| nettai, `dev` | In a practice battle | On the main menu, its reel playing |
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

**With Practice built** (§15; 2026-10-12, the same Mac, window and variables; the build before it and the build
with it run one after the other, twice over, 44 seconds each; load average 139 to 162):

| nettai, `dev`, a practice battle | Before Practice | With it | With it, the input display on |
|---|---|---|---|
| frames a second | 105.5 to 120.0 | 115.1 to 120.0 | 116.3 to 120.1 |
| frame time p95 / p99 | 8.39 to 16.63 / 8.61 to 16.81 ms | 8.44 to 12.51 / 8.99 to 18.16 ms | 8.42 to 10.56 / 8.69 to 13.66 ms |
| long frames in 2 s | 0 to 28 | 0 to 11 | 0 to 6 |
| ticks a second | 59.3 to 60.0 | 59.2 to 60.0 | 59.3 to 60.0 |
| a picture (presented, written), then to the frame drawn | 3.57 to 4.01 ms, then 0.27 to 0.46 ms | 3.57 to 3.90 ms, then 0.27 to 0.43 ms | 3.55 to 3.90 ms, then 0.31 to 0.40 ms |
| key to its picture drawn (the two runs' means) | 7.2 to 13.2 ms (10.5, 11.1) | 8.0 to 14.6 ms (10.6, 10.6) | 8.7 to 17.4 ms (11.2, 12.6) |
| sound queued | 50 to 58 ms | 51 to 58 ms | 45 to 63 ms |

Each is the range over fifteen 2-second windows (fourteen in one) of each of two runs. A practice session puts
nothing on the frame's path but a few comparisons: whether the session's battle is still the one on, the round's
number, and, with the input display on, the two sides' buttons, told to the UI only when one changes.

Measuring found one thing: **the input display, drawn each frame,** cost 0.3 ms a frame (a frame was drawn
0.56 to 0.64 ms after its picture, not 0.28 to 0.37: forty boxes and twenty letters). It is drawn into a layer
(`cache-rendering-hint`) and drawn anew only in a frame a button changes in; the table's last column is with
that. (The spread of the key's time between the columns is where the probe's presses fell between ticks, four
or five a window; a frame that shows a press does draw the display anew.)

**Watching a replay** (§16; 2026-10-12, the same Mac and 1280 by 800 window, `NETTAI_REPLAY=<file>` with
`NETTAI_PLAY_STATS`, the build at 7bebca494: an eight-minute set of EXE6's, 28,718 ticks in three rounds, its
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

**At half and twice the speed, seeking, dragging** (2026-10-13, the same Mac and window, the build at 24d3df7da,
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

**With the practice menu open** (2026-10-13, the same Mac, window and variables, the menu opened 22 seconds in
with the battle in its fight; two runs each, back to back, load average 109 to 161): before, the battle held
still under the menu; then, in the game's own pause (since, held again with its scenery moving: §15).

| nettai, `dev`, a practice battle | Menu closed | Menu open, before | Menu open, in the game's pause |
|---|---|---|---|
| frames a second | 99.8 to 120.1 | 119.0 to 120.1 | 117.0 to 120.0 |
| frame time p95 / p99 | 8.42 to 17.48 / 9.19 to 22.60 ms | 8.40 to 9.03 / 9.13 to 13.49 ms | 8.68 to 10.48 / 9.73 to 16.67 ms |
| long frames in 2 s | 0 to 39 | 0 to 3 | 0 to 6 |
| ticks a second | 59.5 to 60.0 | 0 | 59.5 to 60.0 |
| a picture, then to the frame drawn | 3.13 to 3.87 ms, then 0.28 to 0.41 ms | none | 3.11 to 3.34 ms, then 0.78 to 0.90 ms |
| key to its picture drawn (the runs' means) | 7.7 to 10.3 ms | | |
| sound queued | 48 to 56 ms | 41 to 53 ms | 56 to 58 ms |
| the stream's peak, by second | 0.48 to 0.61 | 0.0 after the first second | 0.36 to 0.67 |

Under the menu the battle now costs what it costs running: a picture a tick, and the menu's panel and shade
drawn over it (the frame is drawn 0.8 ms after its picture, against 0.3 with nothing over it). The keys go to the
menu, so no press is timed then.

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
  - **Netplay** is nettai-rtc on the web (the browser's `RTCPeerConnection`, through datachannel-facade).
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
- **Netplay:** nettai-rtc natively is libdatachannel (C++ and Mbed TLS through datachannel-facade, built with
  CMake as SDL is), which builds for both platforms; not tried here.
- **Lifecycle:** suspend and resume. Netplay can't tick in the background, so a suspended match is a dropped
  link, which reconnects within its 30 s or ends.

## 8. Builds

A **build** is a player's side of one game, named and kept: a TOML file of its own,
`builds/<game>/<name>.toml` in the data folder (`builds/store.rs`). It names its game, the build and the ruleset it
is made for, then states the side under `[side]` as a match file states one (`nettai_match::file::side_toml`, read
back by `resolve_side`):

    game = "exe6"
    name = "Tengu rush"
    ruleset = "preset:masters"

    [side]
    navi = "tenguman"
    cybeast = "falzar-beast"
    folder = [ ... ]

    [aside]
    navicust_programs = [ ... ]
    patch_cards = [ ... ]

- **The name is the file's alone.** `[side]` is what a match, a lobby and a replay are given, and it holds no
  name (docs/design/app-ui.md §2.16).
- **The ruleset it is made for** (docs/design/app-ui.md §2.73; the user: "the build in the menu should indicate the
  ruleset it's for") is the file's alone too: `Build::ruleset`, a preset's id or the player's own ruleset's name by
  kind (`preset:<id>`, `own:<name>`: `rulesets::name_text`, `name_of_text`). A file that names none (written before;
  a serde default, no conversion) is made for the game's first preset. Outside a lobby the build is checked under it
  (`rulesets::made_for`, among `rulesets::pickable`); one it names that is no longer there (the player's own,
  deleted) is `MadeFor::missing`, shown with "(missing)", the build checked under the game's first preset. The
  player's own renamed on its page is renamed in each build made for it, their files written again
  (`rulesets::rename`, `store::rename_ruleset`; the page's `Outcome::Renamed`). A new build (NEW BUILD, an import)
  is made for the ruleset the player last chose (`rulesets::chosen_last`), DUPLICATE keeps its source's
  (`Build::made_for`, `named`).
- **The Builds screen is the editor alone** (docs/design/app-ui.md §2.50): the builds themselves, + NEW BUILD
  (and its FROM A SAVE FILE…), the overview, the build's menu of DUPLICATE, RENAME and DELETE, are Play's (§4),
  and the main menu has no BUILDS.
- **Opened by another screen** (`screens::builds::open(shell, from, at)`; docs/design/app-ui.md §2.28): Builds is
  shown and told by `Shell::tell` which screen opened it (its tag is `opened-by.` and the screen's name,
  `screens::opened_tag`) and the build to open (its place in `store::list`'s order, which every list of them has).
  Back goes straight back to that screen, which leaving the editor closes, the build kept; the header's Back says
  where (‹ PLAY, or ‹ PRACTICE: `BuildsView.to-practice`). Play's EDIT and Practice's open it so (§4, §15). A build
  that no longer reads as one is refused, and the screen goes straight back.
- **A build made new** is `Builds::new_build(content, group)`: the game's own navi, the forms its facts offer of
  that version (docs/design/versions.md §2.2: EXE6's five Crosses and its Cybeast, EXE4's and EXE5's six souls), the
  folder a new save of the game starts with. There are no random builds. The NEW BUILD dialog offers the game's
  versions (its builds data's `groups`, each its tag in its color: `Kit::version_tags`) as what a new build starts
  from, its note the game's words ("Starts with the version's Crosses and Cybeast.") and "They can be changed later
  in the editor."
- **A build has no version** (docs/design/versions.md §2.1, §3.3; the user: "the builds don't have version labels
  either, they're kind of pointless since they correspond to ruleset rules"). It states what it has: its forms, its
  folder (Giga chips of both versions included: no game's folder rules hold a build to one version, the ruleset's
  Version exclusives setting does, §8's Rulesets), its cards (a Bass-Cross MegaMan card's variant its own). No list, sheet,
  card or header shows a version's tag; the header's tag after the title is a page's own (a ruleset's PRESET).
  A build file written before (its `version`, its cards as names) is no build now: the list shows it as one that
  can't be read (docs/design/versions.md §3.2: old builds are dropped, not converted).
- **The Navi tab's selector** (docs/design/versions.md §2.1; the user: "you probably need to have some kind of soul
  and cross and cybeast selector in the editor now right?"; its look as docs/design/app-ui.md §2.61 has it). Beside
  the navi's face its facts, and under them each fact of forms the build chooses (`Builds::form_choices`,
  nettai-match's: the setup's one form that is the side's version, and its arrays of forms), named as the game's
  rulesets name it ("BEAST OUT", "CROSSES 5/5", "SOULS 6/12": the content's `category.<fact>`), its forms in the
  battle's order, both versions' (`facts::offered_forms`: the navi that changes form's sets hold them under the
  fact's name, or for the engine's form list every set's list; sorted by the groups, each version's under a head of
  its name in its color, those of none last: the app places each row, `column`, `row` and `heads`), each a row of the editor's
  lists (`Theme.list-row`, 26 tall, a phone's 40; one column, a phone's two): a check mark where the build has it,
  its face, its name (the locales' `[forms]`: the ROMs' names, EXE6's Beast forms GBeast and FBeast by their
  NameIDs), and at the right its version's mark (`VersionMark`, in build.slint: `Kit::version_tag_of`). EXE6's Beast
  form is at the head, one always chosen, for every navi (MstrCros plays its version's Crosses); the lists are the
  game's own navi's alone. What is wrong with a fact is said by its heading, in the error red, where problems are
  about it (`Draft::facts_of`: the one a problem names, and for a mix of versions, which names none, each fact
  holding one of its things); the chosen forms it is about have their names in the error red (one the build's
  ruleset doesn't allow, one of a mix of versions, the entry a problem names), and a form the ruleset holds
  back has "Not allowed" at its row's end in the error red (`NaviForm.banned`): `activate("form", i)` refuses to put it
  in (the refusal's sound; one the build has is taken out as ever), `forms-all` passes it by and counts it left out,
  and a navi the ruleset holds back (`NaviCard.banned`, dimmed) isn't chosen (design §2.68). The tab has no strip (design §2.66: the user, "i don't
  like having an info thing on the navi view, it's redundant"): the sentences are said in the note's place under the
  selector, in the note's stead (`NaviView.fault`, `Navi::show_fault`): what the build's problems have against the
  form under the pointer or the cursor (`Tab::hovered`, `Tab::focused`), the ruleset's among them (a form's own, then
  its fact's about no form of it), else against the navi (and the build's of no part a tab shows), as
  `Words::summary` says them; the note where neither has any. The place is as tall as the note and a line at least
  whether it says anything or not, so nothing moves when a problem appears; a phone's the same. The same facts flag
  the tab bar's words (`Editor::tabs_of`), so a mix of versions marks the tabs that hold its things (a Beast form's
  the Navi tab, a Giga chip's the Folder tab, a card's variant's the Patch cards tab), every thing of each version
  the build holds has its name red (`Wrong::Mixed`'s `items`: none is taken for the build's own version), and the
  strip of each of them, or the note's place, says it. Enter, a click or a tap is `activate("form", i)`:
  `Draft::choose_form` (`Builds::choose_form`) chooses it, or in a list takes it out, or puts it in where the list
  has room (the Cross window's five, the soul button's twelve: the setup's `form[5]` and `form[12]`), in the
  battle's order; a full list refuses one more. A list (not the one form always chosen) has ALL and NONE (design
  §2.69): two small words at the right end of each version's head where its forms are grouped by version, a group of
  none's head too, else of the fact's heading (`NaviForm.set`, `NaviFormGroup.set`: the index of `Navi::sets`, the
  fact and the group's forms), each a control of its own (`forms-all`, `forms-none`), dim where it would do nothing
  (`all`, `none`). `activate("forms-all", i)` is `Draft::choose_forms` (`Builds::choose_forms`, one change): the
  group's forms put in in order while the list has room, the rest left out and refused (the refusal's sound; the Pick
  sound where all went in); `forms-none` takes the group's out.
- **The overview's forms** (docs/design/app-ui.md §2.62): `BuildSheet::forms`, each fact of forms the build
  chooses in the setup's order and each in the build's own (the battle's), a face and its version's tag (a bar in
  its color under the face); none for a navi that doesn't change form. The wide sheet's abilities and bugs are a
  line fewer for each row of six (`overview::FORMS_TO_A_ROW`).
- **A version's own things are marked** in the editor's rows, by the same `VersionMark` (the version's name in its
  color, framed, 16 tall; a square of the color where the row has no room for the name): the folder's entries and
  the library's rows of a Giga chip (and the auto battle data's), the selector's forms, and a card's variants (the
  Patch cards tab's rows show the card's own name with its variant's mark; the strip says "Bass-Cross MegaMan
  (Gregar)"). Membership is the groups' (`Rulesets::group_of`: a definition's own `version`, a card's variant), so
  any game the content gives groups has the marks; the overview and Play's sheets show none.
- **What is kept aside.** A NaviCust, patch cards and auto battle data are the game's own navi's alone, the one
  that changes form. While a build's navi is another, the facts of the editor's tabs that are that navi's
  (`screens::builds::editor::navi_only_fields`) are kept under `[aside]`, outside the side (the rules refuse a
  NaviCust on a navi that compiles none), and put back when the game's own navi is chosen again
  (`Build::set_navi`, `Build::settle_aside`).
- **A build from a save** (`builds/import.rs`): each game's compat crate reads its own saves, its version giving
  the facts it holds (docs/design/versions.md §2.4: the souls or Crosses it owns, EXE6's Cybeast, a Bass-Cross
  MegaMan card's variant); then each choice on the preset the save is on
  (`Builds::settle`, which says which it took, typed: the list says "Imported as Dark MegaMan") and the facts
  no build states at the rules' defaults (`Builds::as_built`). A file that is no save of the game is the typed
  "Not a BN6 save"; what the readers say in words goes to the log.

**Rulesets** (docs/design/app-ui.md §2.59; `crates/nettai/src/rulesets/`). What may be played: the game's
presets (its rules/rulesets: Masters class, the cartridges' own rules, first; Senior class, without patch cards;
Unrestricted, the versions mixed) and the player's own, each a TOML file
of its own, `rulesets/<game>/<name>.toml` in the data folder (`rulesets/store.rs`, as builds are kept):

    game = "exe6"
    name = "Tournament"
    versions = "one_per_build"

    [banned]
    folder = ["bass", "deltaray"]
    patch_cards = ["starman"]

A category is named by the fact of a build it holds back (the folder's chips, the navi, `navicust_programs`,
`patch_cards`, `crosses` or `souls`, EXE4's and EXE5's `auto_battle_places`: the auto battle data's chips, held
back apart from the folder's), and its items by their names in the game; what a file doesn't name is
allowed. Closed builds is no ruleset's: it is the lobby's own setting.
- **The ruleset a build is made for** (§8, Builds; docs/design/app-ui.md §2.73, taking back §2.59's ruleset a
  player plays by) is what Play's list and Practice check each build under (`builds::shelf`: `Own`, `Shelved::made_for`;
  docs/design/versions.md §4), and the list's row names it (`BuildEntry.ruleset`, `ruleset-missing`: build.slint's
  `RulesetNote` at the status line's right end). In a lobby Play checks them again under the lobby's, the host's
  (`Play::check_builds`: CHANGE BUILD's list, whose rows still name their own); joining asks only the game's rules of
  a build (`Shelved::playable`), the lobby checking it under the host's.
- **The ruleset the player last chose** is one for each game, remembered under `ruleset.<game>` in the settings
  (`rulesets::chosen_last`, `remember`: `preset:<id>` or `own:<name>`; the game's first preset where none is, or the
  one remembered is gone): set by the editor's select and a lobby host's pick, it is only what a new build is made
  for (and what a lobby starts on where no build says).
- **The build editor's ruleset** is the build's: at the tab bar's right end, RULESET, its select (`ruleset`: the
  presets, then the player's own; `Select.note` says "(missing)" after one no longer there) and EDIT
  (`ruleset-edit`); on a phone a row at the top of the Navi tab, between the navi's panel and the grid. Picking one
  makes the build for it, kept at once (`Editor::make_for`); a copy made on its page (DUPLICATE) or its rename is
  the build's too; deleted, the build still names it, missing (`Draft::find_ruleset`). The build is checked under it
  (`Draft::ruleset` is a `MadeFor`; `Draft::checked` is `Checked::under`), so what it doesn't
  allow is among the build's problems: what it has against the build (the problems `Wrong::of_ruleset` says are its,
  an item it doesn't allow and a mix of versions) is said in the strip of each item it is about (the Navi tab's
  note's place), with the game's rules' (`Draft::line`), and by nothing by the select (design §2.66; a phone's pane
  headers say the game's rules' alone). What it holds back isn't added (design §2.68; the user, "okay actually rulesets
  should prevent adding unallowed ncps/chips/patch cards/etc", after adding it anyway was tried): each tab asks it
  (`Draft::allows`, `allows_item`) where something would go in and refuses as the game's rules' refusals are, the row
  shaking where rows shake, the refusal's sound, "Not allowed" at the strip's top right (`BuildWords.not-allowed-here`):
  the folder's `put_in` (a click, Enter, a drag in), the NaviCust's `take` (a row, one of its colors), the patch
  cards' `refuse` (put in, dropped in, a variant chosen), the auto battle's `put_in` (by its own category,
  `auto_battle_places`), the Navi tab's navi and forms. Before that, it is marked unavailable: the folder's, the
  patch cards' and the auto battle data's library rows greyed (`refused`) with "Not allowed" in the error red, a
  NaviCust program's row where none of it is placed dimmed (`ProgramState::NotAllowed`), a form's row and a navi's card
  (dimmed) with "Not allowed" in the error red. The folder's view stays the game's rules' alone (no `View::hold_to`).
  A build that holds it all the same (made under another ruleset, or the ruleset changed) has it as a problem about
  every entry that holds it (`Wrong::NotAllowed`'s `entries`, `Problem::at`: each copy of a chip, each placed
  program, each auto battle place), its name in the error red; taking it out is never refused.
- **A ruleset's page** (`rulesets/page.rs`, `ui/ruleset.slint`'s `RulesetPage` and `RulesetView`): app.slint
  shows it in the place of the screen that opened it (the lobby's Play, the editor's Builds), which goes on
  unseen under it; its controls are `rules.`-named, a scope of the focus's, and the screen hands them to it
  (`Page::activate`, `step`, `choose`, `edit`, `focused`, `answered` for DELETE's question) and hears what it did
  (`Outcome::Changed`: the ruleset shown changed and was saved, or is a copy; `Renamed`, the player's own named
  anew, the builds made for it renamed with it; `Deleted`). The categories (a list;
  a phone's select), the chosen one's items with a check each (Enter, a click or a tap), ALLOW ALL, ALLOW NONE,
  the version's quick choices (`rules.set`: each group with items in the category, then All; their rows changed
  in place, so a press on one keeps its focus), the search; the name (saved when confirmed, numbered where
  another of the player's own has it), Version exclusives (ONE VERSION | MIX VERSIONS, design §2.65: the file's
  `versions`, `one_per_build` or `mixed`), DUPLICATE (named after the ruleset, numbered past the presets'
  names and the player's own) and DELETE (asked once). Each change is saved at once. A preset, or the host's
  ruleset a joiner views, changes nothing, and the reserved line says why.

What the screens ask of the game is nettai-match's `build` (docs/frontend.md §6): the game's versions as groups
in the order every list shows them, the forms a build chooses, a build checked with its problems typed (`Checked`,
`nettai_match::Wrong`), its overview, the folder's view and the chips' rows and sorts. The game's builds data
(content/<game>/rules/builds' `groups`) states each version's color beside it, and the game's strings name what a
build is made of (the text table `builds`: `version.<id>`, `<choice>.<preset>`, and `versions`, what a new build of
a version starts with). Rust and Slint name no version, and hold no color of one.

**A navi's charge shot** isn't on the Navi tab: none of the three games' content names one (a navi's
locale entry has its name and a run message; EXE6's `patch_card_effects` names the kinds of charge shot a patch
card gives MegaMan, which aren't a navi's own). The panel shows the navi's HP and its element; a charge shot's
line needs the content to name each navi's.

**What any screen uses** (`crates/nettai/src/builds/`, with no screen's code; `ui/build.slint`):

| What | The UI's | The app's |
|---|---|---|
| A game's kit: its builds data, its pictures, its library's order, made once | | `builds::kit(shell)` (`Kit`) |
| A version's tag, in the version's own color (a new build's starting group's; a build has none) | `VersionTag`, drawn by a `Pill` | `Kit::version_tags` |
| The build overview | `BuildOverview { build: <BuildSheet>; }` | `builds::overview::sheet(shell, kit, label, name, side)` |
| A game's builds for a list | `BuildRow { build: <BuildEntry>; nav: "builds"; nav-index: at; }` under a `ListView` of 52-tall rows (the row holds its own touch) | `builds::shelf::load(shell, kit)` (`Shelved`: the file, the build, whether it is ready, its row) |
| A chip's row | `ChipRow { chip: <ChipCell>; number: 1; }` | `builds::overview::chip_cell` |
| What is wrong with a build, in the player's words | the catalog's `BuildWords` | `builds::words::Words::problem(&Problem)`: the kind's sentence with its numbers and names, about the part of the build its fact is (`words::wrong(.., &Wrong)` for a kind alone: the general sentence). `words::summary(shell, kit, problems)` words a build's problems as one line, the first and " · +n more" (`Words::summary`, design §2.66), for `UserError::BuildUnusable`; `words::problems(shell, game, ready, &[Problem])` the same for the lobby's line. `builds::words::Words` (`problem`, `summary`, `refusal`, `status`) where many are worded at once |

**A problem's sentence** has no full stop and is whole by itself: "2 Giga chips, limit 1", "Mixes Gregar and
Falzar" (the versions alone: every thing of them the build holds is marked), "The Regular chip's slot is empty". It stands alone on a line of its own (the folder editor's strip) and
after a colon in another's sentence, which ends it (Practice's reserved line: "Your build can't be used: 31
chips, a folder has 30."; the shell's "This build can't be used: {}."). `problem` words any
`nettai_match::check::Problem`, whichever screen is shown; its kinds are matched with no catch-all, so a kind
added to nettai-match doesn't compile until it has its sentence.

**It says what it is about.** A problem names the fact it is about (`Problem::field`, the setup field's name),
and `words::part_of` says which part of a build that is (the UI's `BuildPart`): the navi, its level and the
folder by the engine's roles of their facts, the forms by the facts of forms a build chooses (souls, Crosses, the
Cybeast), else the editor's tab whose facts include it (the NaviCust, the patch cards, the auto battle data). A
kind that would otherwise say "something" says the part: "No navi chosen", "No form chosen", "This navi isn't
EXE6's", "Something in the patch cards is there twice", "This navi can't use the NaviCust". A problem that names
no fact, or one with no part, keeps the general sentence ("Something isn't chosen").
`words::tests::a_problem_is_about_its_part` holds the parts to the facts (a side with no navi, a new EXE6 side with
no Cybeast); the sentences are the catalog's, seen in Practice's reserved line ("The dummy's build can't be used:
No form chosen.").

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
face the round starts with, the name over the navi; Stats, each with what the NaviCust and
the patch cards added beside it in cyan, signed; after the eight, any stat the game's rules give besides
(`Overview::stats`, from the rules' `overview`'s `stats`: each by its name in the game's `stats` strings, its value
and what the build added; the row's `StatLine.label`), EXE5's Soul time for a navi with souls, the turns a Soul
Unison lasts as the battle gives them ("Soul time 6 +3": SoulT+1 and MagicMan's card); Abilities, each by what
gives it; Bugs, each by its full name and its level or amount, purple;
and the folder's 30 chips in two columns of 26-tall rows. Narrower, it is the narrow sheet, 650 tall, which a
player's panel in a match is (430 wide: `label` is YOU or OPPONENT, `name` the player's, `side` 1 or 2 for the
player's color along its top): the stats in two columns, abilities and bugs a line each, the folder in two.
Narrower than 400, a phone's, it is one column throughout, 1158 tall, for a page that scrolls: two columns
leave a chip's name 39 wide there. A stat after the eight is a row of its own: on the wide sheet under Regular
memory, the lines under it one fewer (eight where nine were: `overview::LINES`); on the narrow sheet under the two
columns at the left (a second one beside it), the sheet 22 taller (672); on a phone's the ninth row (1180).
`sheet-height` is the height of the one it is at its width; which it is depends on the width alone, never on the
build, and its height on the width and the stats after the eight. Every number is the compiled one
(`Checked::overview`).

**The editor's tabs** (`screens/builds/editor.rs`). The editor is a frame: the header's name (nothing of the build's
problems: each tab says them where they are, design §2.66), the tab bar with the ruleset's select, and the tab
chosen. A tab's strip says at its top right, where nothing was refused, what the build's problems have against the
item it describes, the ruleset's among them (`Draft::line`: `ChipDetail.fault`, `CardDetail.fault`,
`NavicustView.fault`; `Refusal`'s `problem`; the Navi tab, which has no strip, in its note's place: `NaviView.fault`):
the item's own, those of a whole part it counts in
(the folder's size in an empty entry, a class's limit in each of its chips, the cards' MB in each card; the count in
the error red too), and those of its part that are about no item and no count, in every item of the pane
(`Draft::pane_problems`). An item a problem is about has its name red on its row. The top right is the strip's
own: a name on its line ends at the strip's middle, where it begins, and the Patch cards strip's two groups of lines
end a column before it (the strip's right of the card's name is three columns as wide). A phone, which has no strip,
says the game's rules' problems with a tab's facts in its pane's header (`Draft::pane_line`; not the ruleset's; the
Navi tab's note's place as a desktop's). **A tab is
one Slint file and one Rust module**, as a screen is for the shell:

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
| `checked` | The build checked (`nettai_match::build::Checked`): its problems, its round's stats, the folder's view, the navi's first face, of its round set up with no folder dealt (`check::set_up`: a folder being made is checked as it is, with no stand-in chips). Made again at each change |
| `edit(|content, side| -> bool)` | **How a tab changes the build.** The closure says whether it did; the draft is checked again at once |
| `choose_navi(navi)` | The navi changed, with what is kept aside moved |
| `choose_form(fact, form)` | A form of the Navi tab's selector chosen, or taken out of a list (`Builds::choose_form`) |
| `choose_forms(fact, forms, on)` | ALL or NONE of a group of the selector's forms, as one change (`Builds::choose_forms`): whether it changed and how many were left out for want of room |
| `allows(fact, h)`, `allows_item(fact, item)` | Whether the build's ruleset allows a definition, or an item (a card's variant) |
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
on the language alone, and the pass is told if the counts don't fit even so). CLEAR is at the title's line's right
end, the counts ending before it on that line; its word counts in the room the one line needs.

**CLEAR** (docs/design/app-ui.md §2.71): the Folder, NaviCust, Patch cards and Auto battle tabs each have one small
word (`WordButton`, a control of its own, `clear`) at the right end of a pane's header: the folder's, the cards',
the board's key's first line (PROGRAMS' title line where the game draws no board), the places'; a phone's at the
right end of the counts' box's first line, the board's key's, the places' problem line. `activate("clear", 0)`, or X on the tab (`editor::clear_key`: not on a text field, not a key
the menus take; the NaviCust's with no piece in hand), asks first (`editor::ask_clear`: `Shell::ask`, the tag
`editor::CLEAR`, the tab's `clear-question` and CANCEL focused, CLEAR in the error red), and is refused where the
part holds nothing (the word is then dim and disabled: the folder's chips, the cards, `NavicustView.clearable`, the
places filled). The answer is the tab's (`Tab::answered`, `editor::cleared`): one `Draft::edit` each, the folder
`Folder::EMPTY` (its marks with it), the NaviCust `grid::Edit::Clear` (every placed piece; the piece in hand
dropped; the board kept), the cards `edit::ListEdit::Clear` (EXE4's slots too, one list of records), the places
`auto::Edit::Clear`; the cursor goes back to the tab's list.

The five tabs are built. **Adding a tab** is its two files, `lang/ja/builds.po` and a few lines: its place in
`EditorTab` and its `if` (builds.slint's `EditorBody`, whose one clock brings in whichever tab is chosen), its
view's export (builds.slint and app.slint), its word in `BuildsView.tab-names`, its module in `editor::all()`,
and its sheet's `if` in `BuildsOverlay` if it opens one on a phone.

**The NaviCust tab** (`navicust`; docs/design/app-ui.md §2.10, §2.26). What it draws is the game's data, read by
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
- *Programs*, the open list in the middle: each program once (design §2.64: its colors are its definition's,
  `GridData::colors`), in the library's order, in the color chosen for it, its cross if a plus part; where it has
  more than one color, its colors as small boxes at the row's right edge (`ProgramRow.colors`, the chosen one,
  `ProgramRow.color`, outlined; one the ruleset doesn't allow ringed in the error red), the state's column ending
  where the list's most colors start (`NavicustView.most-colors`), so the states line up. Left and Right on the
  row (`Tab::route`: past the first or the last the focus moves on) choose the color alone; a click or a tap on a
  box (`choose("color")`) chooses it alone, as Left and Right do (design §2.72; a click on the row takes it); it
  starts at the color the build has placed first (`Navicust::color`). Where that color's part is, before the
  boxes: Placed (with how many copies, Placed ×2, where more than one is: `ProgramRow.copies`), In hand, Not
  allowed, or Out of bounds (a placed copy over the ring: no error, and nothing red).
- *Effects and Bugs*, two panels at the right: what the NaviCust alone comes to, the build's overview without
  its patch cards (`own_overview`: `Side::without_patch_cards`, and `Checked::of` a second time where the build
  has cards). The effects are each number it changes ("HP +500"; the rules' own stats among them, EXE5's
  "Soul time +1" where SoulT+1 is at work) and each program at work that gives an ability; the bugs are those it
  brings, each by its full name and its level or amount, purple. A patch card's
  bugs aren't listed here (the user: "navicust shouldn't display patch card bugs"): they are the Patch cards
  tab's (the card's lines, Together) and the build overview's, which has every bug the build has. A bug that
  wasn't there before the last change pulses purple once.
- *The strip* across the top, as the folder's: the program under the cursor (the piece in hand, the piece on
  the cell the cursor is on, or the list's row in its color chosen; with no piece in hand, the row under the
  pointer: design §2.64), where it compresses into another shape its compressed shape then its whole one, over
  COMPRESSED and UNCOMPRESSED (design §2.65; a program of one shape alone, that one with no word), each `boxed`
  (the shape's cells' box, drawn in its color), its name beside its color, the game's description of it (the locales'
  `[navicust_programs]` descriptions, the ROMs' own text, on one line) and its colors. A phone has no strip.
- *The keys.* The cursor goes over the board cell by cell (each cell is a control, `cell`, its place in the
  drawn grid). Confirm on a placed piece picks it up by that cell; Confirm on a program's row takes a new copy of
  it in hand, its middle on the board's, however many copies are placed already (the user: "you should allow a
  navicust part to be placed multiple times"; no game's rules limit copies, §2.26 of the design; it comes
  compressed where the program compresses, the user: "navicust pieces should default to compressed in the
  editor"; a copy already placed in that color says whether it is compressed, as a save compresses them all
  alike). While a piece
  is in hand it is where the cursor is: the arrows (and a pad's stick) move it a cell at a time, the keys bound
  to L and R (A and S) and the pad's shoulders turn it a quarter about the cell held (the tab's `key` and
  `button` take them before the frame changes the tab), C or the pad's north button compresses it in place (its
  other shape turned as it is, about the same middle, held by the same cell where it still has it, else by the
  nearest, the cursor going there: `Edit::Compress`), Confirm puts it down where it fits, and Back puts it away
  into the list, as the game's B does. So a placed program is taken off by picking it up and putting it away;
  Del does it at once (the piece under the cursor, or a program's last copy from its row, or the piece in hand).
  One that doesn't fit is refused with the rules' own kind
  (`GridData::misfit`: `Wrong::OffBoard` or `Wrong::Overlap`) in the reserved line, and shakes; the piece in hand
  is outlined white where it would fit and red, its cells washed red, where it wouldn't (`held-fits`). The
  footer's hints follow the cursor: the hand's while a piece is held (Back: Put away), Del's Take off while the
  cursor is on a placed piece, Confirm's Take and Del's Take off on a placed program's row, and Take on another
  (`on-piece`, `in-list`). Leaving the tab with a piece picked up off the board puts it back where it was.
- *The mouse carries a piece by clicks* (the user: "dragging navicust piece is confusing because usually it's click
  to drop"; §2.38 of the design). The board, the list's rows and the tab under them tell the tab what the pointer
  does (`Nav.choose("drag", phase, where)`: where is the window's point, which the tab turns into the cell under it
  from where the cells are laid out, or finds on the list by the list's view, `editor::over`). A click on a placed
  piece picks it up; a click on a program's row takes a new copy of it in hand, as Confirm there does. The piece in
  hand follows the pointer over the board with no button held, cell by cell, and stays at the last cell it was over
  when the pointer leaves the board; the wheel turns it. The next click on the board puts it down where it fits (or
  it shakes and stays in hand). A click on the list (a row, or the room under the rows) puts it away into the list,
  as Back does ("drag it back to where it came from"); while the pointer is over the list with a piece in hand the
  list shows so, as a library does under a row dragged there (`over-list`: the list fades back under a wash and an
  edge in the error color and the line CLICK TO PUT AWAY, `RemoveTarget`; the piece on the board fades to 0.45). A
  click anywhere else off the board (beside the cells, the effects, the key) puts it back where it was; a new copy,
  which had no place, goes away. Each click acts as the button goes down: a press that moves before it is let go is
  the same click, and nothing follows the held button (let go, the piece comes under the pointer). A right click puts
  the piece in hand away into the list however it was taken, by a click or the keys (off the board, never back where
  it was), and with nothing in hand takes off the piece under the pointer, or a program's last copy from its row. A
  key moves the piece in hand from where it is, and the pointer's next move takes it back under the pointer. (The
  footer's hints are the keys' and the pad's; the mouse has none. The list doesn't scroll by a mouse's drag,
  `ListView`'s `mouse-pan`; the wheel scrolls it.)
- *Its motion* (§4's NaviCust row): a piece taken up lifts to 1.06 with a shadow and a white outline along its
  edge (`lifted`), glides from cell to cell as it is moved (and is where it is taken up at once: `glide`), turns
  as a quarter turn about the cell held (`turned`, `turn-way`), shakes where it is refused, and drops with a
  flash along its cells one after another (`dropped`).

**The Patch cards tab** (`patch_cards`; §2.19, §2.20): the folder's shape. A card's number, name, MB, slot and
lines are the game's data: its number its own, as the game's compat tables state it (`builds::import::card_number`,
each compat crate's `patch_card_number`: the number the card is printed with and a save holds; the library's
order lists the cards by number, one card a number), its MB the field the rules' budget names
(`Overview::budget`), its slot `Slots::own` and `name`, its lines `builds::cards` (each effect's line from the
strings' `patch_card_effects`, a bug's marked by the effect's `bug`, the card screen's group by its `group`, an
effect that holds effects of its own followed by theirs, and a line's names put in as the screen shows them:
`cards::named_by`, a slot by its name, a card by its number). An effect that says its `version` is that variant's
alone (`cards::of_version`, by the card's variant): the Bass-Cross MegaMan card, one card whose effects each
version's ROM gives its own (§2.48), has a variant of each version (docs/design/versions.md §2.1, §3.1: the
versions its effects name, `ruleset::variants_of`), which a build chooses for the card as the NaviCust's program's
color is chosen. The library lists the card once for each variant, its name with the variant's ("Bass-Cross
MegaMan (Gregar)", `BuildWords.card-variant`) and its lines that variant's; putting one in installs the card of
that variant (a record of the card and its `variant`: `edit::list`, `edit::variant`), and putting the other in
while it is chooses the other in its place. The build's list and the strip show the installed card's variant's
lines. A ruleset can hold one variant back (its item `bass-cross-megaman/gregar`): that row is "Not allowed".

- *The strip* lays the card under the cursor (or, design §2.64, under the pointer) out: its number, name, MB or
  slot, why it can't go in (in the
  error red: the typed `EntryRefusal`, worded by `Words::entry_refusal`: "Needs 24 MB"; of one of the build's,
  the rules' problem with it), and its lines in its two groups under their names in the game's strings
  (`patch_card_groups`: Parameter, Ability); where a game's cards have no groups (EXE4's), what it does and the
  bugs it adds to or clears.
- *The build's cards*, framed: in their order, a row a card as the folder's (number, name, its version's mark, MB;
  its lines are the strip's: design §2.64), and an Empty row after them
  while the list has room; in a game whose cards go in slots, each slot by its name with its card or Empty. Cards
  and MB used (or Slots filled) in the header, ticking as they change, and Together at the foot: what the cards
  add up to, the build's overview without its NaviCust (the numbers they change, what they give, their bugs).
- *The library*, open: every card of the game, its rows as the build's, a search by name, and a sort (number, name, MB;
  in a game with slots, number, name, slot). A card that is in says Installed; one that can't go in is greyed
  with why, in the error red what it needs. No pictures (a card has none) and no on or off (nettai plays every
  card in).
- *The keys.* Enter on one of the build's rows goes to the library (on a phone, its sheet); Enter on a card puts
  it in (at the list's end; in a game with slots, in the slot it belongs in, in place of what was there), and the
  cursor stays in the library to put in another; Enter, Del or a click on a card that is in takes it out again
  (in a game with slots, empties its slot), its hint Remove (`on-installed`). Del takes the build's card under
  the cursor out; `[` and `]` (the pad's north button: up) move it, as the order matters. A card put in flies
  from its row to the build's, which flashes; one taken out flies back; one refused shakes.

**The Auto battle tab** (`auto_battle`), in a game whose setup takes the places: the folder's shape again. The
places and their lists are the game's (`builds::auto`: its rules/auto_battle/block's `LISTS`, each named in its
strings' `auto_battle_lists`: EXE4's and EXE5's alike, as their writers fill them; a block that states no lists
would have its 42 places as one, untitled). A build states the places alone (its pattern records are the rules'
default, nothing learned), so a place holds a chip or nothing, and the pattern list's places aren't shown (the
game puts none there but a learned pattern's: a match is sent them empty, a valid block); the status counts the
places shown. The strip describes the chip under the cursor; the places are framed under their lists' titles in
two columns, scrolled (a `ScrollPage`, `places.`); the library is a row a chip, each saying in how many places it
is, of what the place being filled can hold. **The mega, giga and program advance lists hold their class alone**
(the user, 2026-10-13: "only allow mega chips in mega, giga chips in giga, and PA in program advance"): the rules
say so, each such list's `own_class` in @exelib/auto_battle/places' `LISTS`, which their `validate` holds a place to
and the tab reads (`Holds::Chips`). For a place of such a list the library is the game's chips of that class
(`auto::of_class`: the Mega chips, the Giga chips, the Program Advances, which no folder holds); for a place of
another, the chips a folder of the build can hold. The library changes with the place the cursor is on, from its
first row; a Program Advance's class in the strip is Program Advance, and it shows no picture and no element
(it has neither, the user: "program advances don't have elements or chip art, so don't show those": the
pictures' `art` and `chip_element` give a Program Advance none; the strip's facts come up to where its element
would be). Enter on a place goes to the library, Enter
on a chip puts it in and goes on to the next place shown (it flies there, the place flashes), Enter, Del or a
click on the chip the place holds takes it out again (it flies back; its hint Remove, `on-placed`), Del empties a
place. A chip the rules refuse in a place (their `validate`: one its navi can't play, or one of another class in
a list held to one) isn't put: its row shakes and the strip says why, worded for the part (`Words::problem`:
"{chip} can't be played in auto battle"; in a list held to another class, "{chip} can't be in the {list} list",
the list's name as the game's strings have it). A build that holds such a chip already (a build file or a save
made by hand) has the problem as any other: Play's list counts it, the tab is flagged, the place's name is in
the error red (`flagged`, as a folder's entry), and the strip's top right says why of the place it describes where
nothing was refused (design §2.66; a phone's line over the places, `AutoBattleView.problems`).

**A click chooses** one of the build's rows (a folder's entry, a patch card or an EXE4 slot, an auto battle place)
as Enter does, but where the library is beside it the focus stays on what was clicked (the editor's `activate`
brings it back from the library the tab's Enter went on to), so the next key acts on it (R, T, Del, the arrows)
and a click on a library row puts what it holds there; a phone's click opens the library's sheet, as Enter.
**What was clicked is marked.** The pointer hides the cursor, so the build's row the tab is on (the entry, card
or place being filled: the views' `marked`) has the chosen row's bar and tint while the focus is on it or in the
library, the cursor shown or not. It had them only while the focus was in the library, the cursor's brackets
alone marking it otherwise, and with the focus kept on a clicked row nothing did (the user: "it is impossible to
tell now what is selected with the mouse on the left hand side in patch card/chip/abd etc"). A phone's folder,
a `ListView` of its own, marks it with the list's `chosen`. The pass checks it as the window has it
(`build_tabs::clicks`: a click, then R, T, Down and Del in the folder, Up on a patch card and a click on another
that holds a card and Del (in EXE4, whose slots may be empty, the second slot that holds one), Del on a place;
and a shot of each tab's first click with the pointer gone on to another row, §13).

**A click removes nothing**: it marks the row, as above, however often it comes. (For a while a click on the marked
row removed what it held, then a double click did too; the user then: "remove double clicking to remove", and of the
click on the marked row, "remove that too".) What takes a row's thing out with the mouse is a right click, below, or
dragging it back onto the library. A click on a library row is as before (a patch card that is in, or the chip a place holds, is
taken out by it). **A right click removes everywhere** (the user: "sure right click should remove everywhere"): on
one of the build's rows it takes out what the row holds, as Del does, whether or not the row is marked, the focus
coming to it (it doesn't mark it first: the press brings the focus, then `Nav.choose("remove", ...)`, which the
editor hands the tab as `Action::Delete`); on a library row that is in the build (a patch card that is in, the chip
the place holds) it takes that out; on the folder's library it does nothing (a chip of the library is in no one
entry). The touches say which rows it removes from (`Focusable`'s and `RowTouch`'s `removable`, told to
`Drag.pointer`). The NaviCust's right click is as it was: it puts away the piece in hand or takes off the one under
the pointer. A phone's tap opens the library's sheet as before, which has REMOVE: a tap isn't a removal there. The
keys and the pad are unchanged.

**The pointer on a row describes it** (design §2.64: "show chip/patch card/ncp details on list item hover"). A row
given `describes` (`Focusable`'s and `RowTouch`'s; the NaviCust's program rows their own touch's) says when the
pointer comes onto it and leaves it, and when its list scrolls another row under the pointer: `Nav.hover(id, index,
on)`, the app's `NavInput::Hover`, handed to the screen as `Screen::hovered` and by the editor to the tab. Each tab
keeps the row hovered beside the one it describes (`editor::hover`: a row left that isn't the one hovered changes
nothing, the next row's coming may be told first), and its strip shows the hovered row while there is one, else the
focused one; the focus doesn't move. A move of the focus (a key, a click) drops the hover, so the strip follows
whatever was used last; a coming onto a row while the keys were used last (`Nav::pointing` false: a row the keys
scrolled under a still pointer) isn't told at all, nor anything on a phone. A refusal's reason belongs to the focused
row, shown with it; the row a drag was let go over (`Registry::at`, the editor's `rest`) isn't a hover until the
pointer goes on to another, so a refused drop's reason stays in the strip.

**A row dragged moves** (the user: "also you should be able to drag and drop to reorder"). A press on one of the
build's rows that moves 6 from where it went down (`Drag.threshold`, either way: the folder's second column is
reached sideways) is a drag, not a click: the row lifts under the pointer (raised, 1.03 larger, a shadow and a line
in the cursor's color, `LiftedRow`), the rows between where it was and where it would land slide over a row to leave
a gap there, outlined in the cursor's color (`DropGap`; the slide is `Theme.quick`, a cut with motion off), and the
numbers the folder's entries and the places show are those they would have. Let go, it lands there with the flash a
chip put in has; let go off the list (more than 16 past its rows), it goes back, and nothing changes.

**Dragged back onto the library, it is removed** (the user, of making removing with the mouse alike in every tab,
chose "drag it back to where it came from": "ok do 1"). A row of the build's let go over the library beside its list
(its search, its sort and its rows, or within 16 of them: `editor::over_library`) is taken out as Del takes it: the
folder's entry emptied, the patch card taken out, the place emptied. While it is over the library the drag shows so
(`Drag.removing`, which the app sets as it says where the row would land: `editor::show_remove`): the lifted row's
line turns the error color and it fades to 0.6, and the library fades back under a wash and an edge in the error
color with DROP TO REMOVE on a band across its middle (`RemoveTarget`, folder.slint; each fades in and out over
`Theme.quick`); back off the library, the row looks as a drag does again. Anywhere else off the list it still goes
back. On a phone the library is a
sheet, shut while a row is dragged, so nothing is over it: a drag there only reorders, and the sheet's REMOVE stays
the way to take something out (`over_library` is false on a phone).

**A library's row dragged into the list goes in there** (the user: "you should allow drag and dropping from the
library into a slot", and "you should also allow drag and dropping for exe4 patch cards"). A press on a library's row
that moves 6 is a drag, as a row of the build's is; a press that doesn't move is still a click, which puts what the
row holds in the marked row, as before. The library's row stays where it is, and a ghost of it follows the pointer
(the lifted row's look in the shape of the build's rows, at 0.7 until it is over a row it would go in; held no
farther in than its middle, as a library's row is wider). Over the build's list the row it would go in shows so, in
the look of a row dragged within it; let go there, it goes in, by the rules a click or Enter goes by: a chip the
rules refuse there, or a card the MB or the list's room can't take, isn't put in, its library row shakes and the
strip says why ("Falzar only", "Needs 24 MB", "MrkCan1 can't be in the Mega list"), and nothing else changes. Let go
anywhere else (the library among it), nothing happens. Put in, it flies from its row as a click's does, and the row it
went in flashes and is the marked one (the folder's entry and the auto battle place with the focus on them, as after
a move; a patch card with the focus left in the library, as after a click).

- *The folder:* the chip goes in the entry under the pointer, in place of what it held, none shifting (`DropGap`
  under that entry). Its reason is asked of the folder without that entry, as a click's is of the folder without the
  marked one (`Folder::fill`); refused, the marked entry stays as it was. A refused click's row now says why in the
  strip too (it said so in the row's note alone).
- *The patch cards:* the card goes in at the place under the pointer, those from there on shifting down to leave it
  a gap (`ListEdit::Insert`); over the Empty row after them, at their end. One that is in already moves there, as a
  drag of it in the list would.
- *EXE4's slots.* A card is the game's for one slot (its definition's `slot`, which `build_check` holds a build to:
  content/exe4/rules/patch_cards), so it can be in no other, and a slot's card has nowhere to move to within the
  list. What a drag can mean there is coming in and going out: a slot's card dragged onto the library empties its
  slot, as the list's cards are taken out; over the slots it goes back to its own (its slot is outlined), and let go
  there nothing changes. A library's card let go anywhere over the slots goes in its own slot, in place of what it
  held, as a click puts it (its slot outlined while it is held there: `Drag.swap`, none shifting); one that is in
  already stays.
- *The auto battle places:* the chip goes in the place under the pointer, in place of what it held, none shifting,
  held to what that place may hold (`AutoBattle::fill`). The library is still the one for the place being filled, so
  a chip of another class dragged into a Mega, Giga or Program Advance place is refused there.
- *How it is told.* The library's rows are given its list's name (`drag: "lib.rows"`), so `Drag.list` says a drag
  is one of them; the editor tells the tab it as `drag-in-over` and `drag-in-drop` (`editor::from_library`), `index`
  the library's row. The tab answers with `editor::show_landing`, which also names the list it lands in
  (`Drag.into`, the list that scrolls near its edges: a row of the build's, its own); the views' drag globals
  (`FolderDrag`, `CardDrag`, `PlaceDrag`) show it as `filling`. The ghost is the tab's: the folder's
  `FolderView.ghost`, the auto battle's `lifted`, the patch cards' the library's own row.
- *On a phone* the library is a sheet over the list, shut once something is put in, so a drag from it has nowhere to
  land in view: its rows aren't dragged there, and a tap puts in as before.

- *How it is told.* The UI's `Drag` global (nav.slint) holds the drag (the list, the row, the pointer, where in the
  row it went down); the row's touch, given its list's name (`drag`), feeds it. Each move asks the app where the row
  would land (`Nav.choose("drag-over", from, at)`, `at` the window's point); the tab works it out from where its rows
  are as the focus's registry has them (`editor::nearest`, `editor::list_row`, with `Registry::view` and
  `Registry::row_at`), as they are laid out still, the shifting being the rows' insides alone, and says so in
  `Drag.to` (and `Drag.swap`). Let go, `Nav.choose("drag-drop", from, at)` makes the edit.
- *The folder:* an entry to another entry, those between shifting toward where it was, its Regular and tag marks
  going with their chips (`FolderEdit::Move`, `edit::moved`); it is then the marked entry.
- *The patch cards:* a card to another place in the list, by the edit `[` and `]` make (`ListEdit::Move(from, to)`,
  which the keys make a place up or down); past the last card, it is the last. An EXE4 slot's card goes back to its
  own slot (each card's, which the editor already holds to), or onto the library, below.
- *The auto battle places:* a place's chip to another place of its list, those between shifting (`Edit::Move`); to a
  place of another list, the two places trade what they hold (`Edit::Swap`), the target outlined and nothing
  shifting (`Drag.swap`). A chip the rules refuse where it would go isn't moved, as one from the library isn't: the
  place dragged shakes and the strip says why ("MrkCan1 can't be in the Mega list").
- *The rules:* a move is shown as any edit is (the folder's entries and the patch cards are never refused one: what
  the rules have against the result is the row's problem, as ever); only the auto battle's, as above, where the
  editor already refuses that chip in that place.
- *Scrolling.* The lists the build's rows are in don't scroll by a mouse's drag (`ListView`'s and `ScrollPage`'s
  `mouse-pan` off, as the NaviCust's programs), so a drag on a row is the row's; the wheel scrolls them. The libraries
  beside them are alike on a desktop: their rows are dragged into the list, so a mouse's drag doesn't pan them
  (`mouse-pan: Theme.phone`; a phone's sheet keeps it, its rows not dragged). While a row is dragged near the top or
  bottom edge of the list it lands in (`Drag.into`: within 32, or up to 96 past it), the list scrolls that way
  (`Scroller`'s timer, 8 a frame) and asks again where the row would land; the library a row is dragged from doesn't. A touch's drag still scrolls a phone's list, as Slint's
  `Flickable` takes a touch that moves in its first half second; one held still for that half second is the row's,
  and drags it (the Folder's, the Patch cards' and the Auto battle's lists on a phone alike; a list with nothing to
  scroll gives a touch's drag to the row at once).
- *The pass* (`build_tabs::drags`, §13): a double click on a folder entry and one more click on it, marked (its
  chip left in, `tabs-folder-clicked-twice`); an entry dragged three rows down, shot held there and after; an entry dragged onto the
  library, shot held there (`tabs-folder-drag-remove`, the remove look) and after (`tabs-folder-drag-removed`: emptied,
  the next entry untouched); a right click on an entry not marked (taken out, the focus on it); the library's first
  chip that can go in dragged onto the emptied entry, shot held there (`tabs-folder-drag-in`: its ghost, the entry
  outlined) and after (`tabs-folder-dragged-in`: in that entry, the others as they were, the focus on it), and one
  the rules refuse dragged there too (`tabs-folder-drag-in-refused`: nothing changed, the strip says why); a patch
  card dragged above the one before it, and the first card onto the card library (`tabs-cards-drag-remove`: taken
  out, the others closing up; in EXE4 the first slot that holds a card emptied, `-removed`); a library's card that
  isn't in dragged onto the first card (`tabs-cards-drag-in`: a gap there, the cards from there on shifted down;
  `tabs-cards-dragged-in`: it is the first, the others after it; in EXE4 its own slot outlined, and then holding it);
  an auto battle place's chip dragged two places back
  in its list, then into another list (traded), then down past the page's foot into the Mega list (it scrolls there;
  refused), then the first place holding a chip onto the library (`tabs-auto-drag-remove`, `-removed`: that place
  emptied, the others as they were), and a library's chip dragged onto the second place (`tabs-auto-drag-in`,
  `tabs-auto-dragged-in`: it holds it, the others as they were, the focus on it). On a desktop the NaviCust's clicks (`build_tabs::carries`): a placed piece clicked
  up and carried over the list (`tabs-navicust-over-list`, the remove look), a click there (`tabs-navicust-clicked-away`:
  a program fewer placed); another clicked up and carried off the board onto the effects (`tabs-navicust-carried-off`),
  a click there (`tabs-navicust-put-back`: back where it was, the board as it was). On a phone, the folder's drag, with
  the mouse (the pass has no touches to give).

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
the system asks until the player says), the battle's graphics (`enhanced`, else original: docs/frontend.md §3), the levels of the music and the
sound effects, the input delay (4), the folder the player chose the Legacy Collection in (`legacy_collection`,
where Steam doesn't list it: §17), the keys and the pad's buttons bound (those that differ from the defaults),
and what screens remember by their own keys (`remembered`: what a screen chose last, such as Practice's builds and
its dummy's setup and options, §15, Play's build and First to, Builds' tab, a video's folder). A setting the file leaves out is the app's default. The battle's
music isn't one of Settings' rows (the user: "settings is a weird place to put it"): it is picked beside the
arena, for one visit to Practice, and kept nowhere (§1, "The battle's music"); nor are Practice's Arena and
Background (§15).

The Settings screen is one scrolling page with a list at its left that jumps to a group: General, Video, Audio,
Online, Controls, About. In Controls, Enter on a key's or a pad button's box asks for the next key or button; one
bound to another action swaps with it, so every action keeps one. In About, SHOW LICENSES opens About in the
page's place (§18).

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
so has Practice's arena: the picture is a whole multiple of the game's 240 (§1), and the gutters' 32 can be the
difference between one multiple and the next (a window 780 of the display's pixels wide takes three times, 720;
716 took two).

## 10. The design system

docs/design/app-ui.md §4 is the look; this is where it lives.

**The theme** (theme.slint): every color, the four faces (Chakra Petch 500, 600 and 700, Corporate Logo 700
for Japanese display text, M PLUS 1 Code and Murecho, all under the SIL Open Font License, each with its license
beside it in `fonts/`), the metrics, and the durations. A screen uses these and writes no color, size or duration
of its own: a display text's weight is the theme's too (`display-bold`, `display-semibold`: 700 and 600 in Chakra
Petch, 700 for both in Corporate Logo, which is bundled at that weight alone).

**A game's look** (docs/design/app-ui.md §2.23, §4). Each game states its own in its content, beside its versions'
colors (rules/builds) and its name (its locales): `content/<game>/look.toml`, which `nettai_content::look` reads.
It holds what the app draws and nothing more: the ground's two colors and its glow (a color and a strength, the
opacity it is drawn at), the panel, raised, line and inset colors, the text's three, the cursor, the main action and
the text on it, and the backdrop's pattern (`[pattern]`: its `kind`, `color` and `strength`). It is TOML, read from
the content alone as the game's name is, because the menus must be in the selected game's look before the game's
content is loaded (a Luau table would wait for its runtime); the battle reads none of it, and `nettai-content
check` holds each to its fields. A game whose content states none is drawn in the app's own look, the theme's
defaults (BN6's), and one whose file doesn't read is too, with the reason in the log.

- *The theme* is the selected game's look (`Theme.look`, a `Look`), and the colors every screen uses are made from
  it: the twelve stated, and those that follow from them (a raised control's hover, the header's bar, a dialog's
  shade, the cursor's glow and tints, the out-of-bounds hatching, a piece's marks). The other tokens (the players'
  colors, good and bad, a bug's purple, Mega and Giga, REG, TAG, a chip's code, the command line) are the same in
  every game. Nothing in Rust or Slint names a game.
- *The patterns* are the app's, by what they are (`PatternKind`: hexagons, an isometric grid, a starfield with a
  streak), each drawn in the look's pattern color at its strength, with the kind's other marks at fixed fractions of
  it (a starfield's fainter dots and its streak into the main action's color, a grid's wash). Their paths are
  drawn by the app for the window's size, once for a size (`look::patterns`), each over the window and one of its
  tiles more each way, and each ground draws its path once into a layer that drifts by less than a tile, as the
  hexagons always were (§5): a ground at rest costs what the hexagons did.
- *A change of game* (`look::Looks`). The new game's ground (its gradient, glow and pattern) is pushed onto
  `Shell.grounds`, the last drawn on top; each ground (`GroundLayer`) covers the window from the side it came in
  from as its own clock runs (from its creation, for the theme's `settle`), clipped at its edge, with a lit line and
  a soft band of its cursor's color leading it; once the last has swept, those under it are dropped. Its side is
  the arrow pressed (the stepper's left arrow or L: from the left; the right one or R: from the right; a game
  selected otherwise, a replay's or a build's, from the right). Over the same time the theme's colors cross-fade
  from those shown (`Theme.was`) to the new look, by `Theme.fade`, which the app moves a frame at a time and eases
  as the sweep is. The panels and the text cross-fade rather than change at the edge: that would mean drawing every
  screen twice. A second press mid-sweep starts at once from what is shown: a third ground sweeps over the two, and
  the colors fade on from where they were (four grounds at the most: a fifth drops the oldest). Nothing waits on
  it, nothing changes a size, and with motion off the new look is there at once. The motion probe watches it both
  ways and twice over (§13).
- *Its clock starts at the frame after the press*, not at the press: the ground is pushed, and the colors' fade
  started, in the app's frame (`Looks::frame`, before Slint draws it, with that frame's time). The probe's first
  run, with the ground pushed at the press, showed the sweep 90% across in the first frame drawn after it.
- *The menu's reel waits for the sweep.* The window's first frame after a change of game begins 5 to 8 ms after
  the press, but its work was 40 to 57 ms (the motion probe's "the press to its first frame, and its work"): all
  of it the main menu making the new game's reel, its match, battle and player (23 to 33 ms, on the window's
  thread, where the content's runtime is), and in the screenshot pass the reel played on to the tick it is held
  at (12 to 29 ms more). The press, the sound's change and the looks cost nothing measurable. So after a change
  of game the menu holds its next reel until the sweep has crossed and 60 ms more (`Menu::hold`; none with motion
  off), the old reel's last picture standing under the sweep, and the new one fades in as a reel always does. The
  first frame's work is 0.1 to 0.3 ms now, and the sweep shows from it: 39, 82 and 98% across at 79, 137 and 197 ms
  after the press, done by 258, where before the first frame to show it was at 144 ms, 45% across. (The shots are
  60 ms apart at the least: taking one holds the window that long.) The cost moved to the frame that makes the
  reel, after the sweep; the battle's own frames are untouched.
- *What it costs*: 0.14 MB of generated Rust (10.21 to 10.35 MB: the ground's component 0.08, under its `for`, and
  the rest the theme's colors read where they were constants), and the bundled faces 1.8 MB more (M PLUS 1 at 800,
  with its Japanese; since replaced by Corporate Logo at 700, 2.8 MB).

**The components** (widgets.slint, shell.slint):

| Component | What it is |
|---|---|
| `Cut`, `PanelBack`, `Panel` | The rectangle with one line and its bottom right corner cut (9 on a button, 12 on a panel, 18 on a dialog or a menu); a panel's back with the cyan accent, or a player's color; a panel with something in it |
| `Button` | Default, the main action (amber), ghost, text (no fill and no line, its words muted until hovered or focused), unavailable (`enabled` off: the focus passes it by); `danger`, `big` |
| `Field` | A text field (§11); `code`, a code's (fixed-width type, larger, tracked; what goes in it, while it is empty, in the body face) |
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
| `Backdrop`, `Cursor` | The ground, in the selected game's look, with its pattern (the grounds of a change of game sweeping in over it), behind everything, a battle that fills the window too (the user: "the backdrop should be behind the game at all times"); the bracket cursor |

**The rules** (docs/design/app-ui.md §5) are built into them: a value has a box of a set width and is cut with an
ellipsis (a stepper's is drawn smaller to fit first, down to 11 px, then on two lines at 11 px, and only past
that cut, which the screenshot pass reports: the `Stepper`'s own check); a line for a message is there when empty; digits are M PLUS 1 Code's, one width; the cursor is drawn
over the layout; a field's text is clipped at its padding.

**Motion** (docs/design/app-ui.md §4, "Motion"). The theme has the one table of durations (`quick` 110 ms and
`brisk` 150 for a control, `settle` 200 for a panel or a screen arriving, `leave` 120, `set-piece` 480, the
`stagger`, 30, and `calm-fade`, 160, what Reduce motion keeps of the jack-in), and the components move by
themselves, so a screen built from them moves without its author writing an animation:

- the cursor glides between controls, breathes while idle and snaps inward on Enter;
- a button's press flashes its fill and dips it 1, and the main action has a slow sheen;
- a stepper's value slides in its box the way it was stepped; a switch's mark and a tab bar's underline slide;
- a list's chosen mark slides to the next row, and a scroll the app asks for is smooth;
- a panel's accent draws in; a progress bar fills under a moving highlight; a message fades in;
- a screen's body slides and fades as it leaves, and the next one's title wipes in; a dialog grows in as its
  shade fades in, and a failure's shakes once; a menu over the battle slides in from the left;
- the two cards before a battle slide in from their sides, VS stamps, and the ground's pattern rushes; the
  pattern drifts otherwise, and a soft pulse crosses it now and then;
- a battle that had its cards jacks in as it begins (`JackIn` in shell.slint, `Shell::jack_in`; online,
  practice and a replay's viewer alike): over its first frames, for the set piece's time, the ground covers it
  as the field rushes again, and a swipe takes it off from the left: eight strips, each 0.045 of the set piece
  after the one above, each strip's edge lit in the cursor's color with a 140-wide band of it trailing over the
  battle; the game's encounter sound plays (§1). The battle starts when it
  would have and runs under it: it is drawn over the battle and takes no input, and nothing of it reaches the
  simulation, so an online match's two sides start in lockstep as before. With Reduce motion the ground fades over
  `calm-fade` (a `Pulse` that is `calm` runs with motion off), with no swipe; in the screenshot pass
  (`Theme.still`) there is nothing of it;
- an online match's result (§4): the winner's half floods in from its edge (`settle`), WIN or LOSE stamps in
  (`brisk`, `quick` after), then the score and the time count up (`Count`), ending with the set piece;
- a change of game sweeps the new game's ground across the window from the side of the arrow pressed as the
  colors cross-fade (above).

What a screen does write is *which of its parts arrive, in what order*: each part goes in an `Arrive` with its
place in the order (the main menu's words, then the reel), and rises as it fades in, 30
ms after the one before. And its own set pieces, from the same pieces (motion.slint):

| Piece | What it is |
|---|---|
| `Tween` | A clock that runs once from 0 to 1, from its creation and each time its `seq` changes: `t` eased out, `linear` the time alone |
| `Pulse` | A clock that falls from 1 to 0 each time its `seq` changes |
| `Arrive` | A part of a screen arriving: rising, or from a side (`from-x`: a tab's body following its underline) |
| `Flash` | A flash over what it covers (a slot a chip landed in, READY stamping) |
| `Shake` | `dx` swinging to each side twice and coming to rest (a refusal): added to the `x` of what shakes |
| `Count` | A number that ticks to its value when it changes; or counts up from 0 as it is made (`from-zero`, after its `delay`, over its `duration`: a result's numbers, one after another), a time shown from hundredths (`clock`), saying each tick (`ticked`) and when it is at its value (`done`) |
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
   and the screenshot pass. The one thing Reduce motion keeps is the jack-in's short fade (a `Pulse` given
   `calm`), which the screenshot pass doesn't (`Theme.still`). A set piece a press settles (the result) is put at
   its end by its clocks' `settled`. In the battle, with Battle graphics Enhanced, it also takes off the
   background's glide, the gauge's smooth fill and the banners' squash at the window's resolution (the
   renderer's `reduce_motion`, set as a battle, the menu's reel or a video starts; a replay's still never has
   them), and keeps the sharp text (docs/frontend.md §3, "Enhanced and original").

**A row that is focused as a whole** (the design's rows are: the cursor is around the label and its control
together) has a `Focusable` of its own under its control, named as the control is, and the control's own is
turned off (`focusable: false` on a `Stepper` or a `Select`): the arrows' clicks still step it, a click anywhere
else on the row activates it, and the list `Shell::pick` opens is as wide as the row. Practice's rows are so
(`BuildOption` and `MenuRow` in practice.slint); `OptionRow` here is the other way, the stepper alone focused.

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
Practice and Play were the first to show a `Message`, whose text sat in the middle of its line (a text with no
width of its own is as wide as its words, centered): it is as wide as its line now, and at its left unless the
screen says otherwise.

Three things about how it is made. A clock reads the frame's time (`animation-tick()`) only while it runs, and
stops when it is done, so a screen at rest evaluates nothing each frame but what moves by itself. What moves
without stopping is drawn once and moved as a picture: the ground's pattern is a cached layer (§5). And the
cursor's glide is the app's (`Nav::settle`: a share of the way to the focused control each frame), not a Slint
animation: that starts over each time its target moves, and the cursor crawled behind a screen arriving or a page
scrolling.

## 11. Focus and navigation

docs/design/app-ui.md §5, rules 5 to 10. There is one focus, and the app holds it (`nav.rs`).

**A control is focusable by holding a `Focusable`** (nav.slint; every control of widgets.slint does), which
tells the app where it is in the window and takes its clicks. From those rectangles:

- **The arrows** go to the neighbor in their direction: a control straight ahead (one that overlaps the focused
  one across the direction) before a nearer one off to the side, and of two equally near the one that starts
  where the focused one does. On a stepper, a select, a switch or a slider,
  Left and Right change the value instead.
- **Tab and Shift+Tab** go through the same controls in reading order, around from the last to the first.
- **A screen may route a move itself** where the places don't say it (`Screen::route`, asked before the nav for
  an arrow or Tab while nothing is open over the screen): Play's Right from its list goes past the overview,
  which takes no focus, to EDIT (`Nav::focus_first`), and its Tab goes zone by zone (`Nav::tab_in`, the
  controls it names first in its order, the rest in reading order after them).
- **Enter** (and Num Enter, Space) activates; **Esc** goes back: out of a list, a dialog or a menu (a battle's
  menu too, which Esc opened), else within the screen (`Screen::back`), else to the main menu. These are the
  menus' fixed keys (`input::menu_action`, with the arrows, Tab and Shift+Tab, Delete and Backspace, Page Up and
  Down, Home and End): a binding can't take them, and the keys bound to the GBA's A and B (Z and X) are nothing
  to the menus. The keys bound to L and R are previous and next. (The user had asked for the GBA's A and B
  everywhere, "i feel like the GBA A and B buttons should be the universal confirm/cancel buttons everywhere,
  instead of enter+esc"; tried, the keyboard went back: "actually z and x for navigation is not very good,
  enter + esc was fine". The pad kept its half.) Settings, asking for a key to bind, takes every key but Esc,
  which stops the question.
- A key held repeats a move; a choice is pressed once.

So no screen writes key handling, and a control added to a screen is reachable by being there. A screen opens
with the focus on its main control (`Screen::main`) and remembers where it was; a dialog or a menu is a scope,
and the focus goes back where it was when it closes.

**The cursor** is one object, four brackets drawn over the layout 5 outside the focused control, which glides
from control to control. It shows only while a keyboard or a pad is the last thing used; the pointer hides it.
Where a list marks its row, the mark stays when it hides: a library's focused row, and the build's row the
editor's tab is on, have the chosen row's bar and tint whether the cursor shows or not (§8, "A click chooses").

**Text fields don't trap** (rule 7): the arrows walk past a field without changing it. Enter or a typed letter
starts editing (the letter is the edit's first), and so does the pad's A; while editing, the keys are the
field's own, but Esc restores the text it had, Enter finishes it, and Up, Down or Tab finish it and move on;
the pad's A finishes and its B restores. A dialog whose first control is a text field (a build's name, a direct
connection's address) opens with it edited, its text all selected, so what is typed replaces it
(`Nav::open_editing`; the user: "you should auto focus the primary text field on dialogs that pop up"); Esc then
ends the edit and leaves the dialog open, and a second Esc closes it. Up and Down in a scrolling page (the
practice menu's rows, Settings) go through the page's controls, scrolled into view, before they leave it for a
control beside or under it, however near that one is on screen (`Registry::next`; the user: "it's impossible to
scroll the practice menu without selecting the button on the bottom first, it should scroll the entire length
before moving to buttons underneath"). A search field's list (the libraries' of
the folder, the patch cards and the auto battle data; the replays') goes back to its top as the search changes,
sliding there as a reveal does, and the focus enters it at its first row next (`Nav::list_to_top`; the user:
"when a search box is changed, it should reset scroll"). While a field is edited the footer's hints are the field's own
("Enter Done", "Esc Cancel", the `Page`'s `field-hints`). The field's border and caret light
while it is edited, and its text is clipped at its padding and scrolled to keep the caret in view (rule 8). The
edit is Slint's `TextInput`, so the desktop's IME composes into it. The screen is told each change and sets the
text the field shows, so a name is cut to its length and a code put in capitals as they are typed.

**Lists and pages** (rule 9). A `ListView` is one stop: the focus is on one of its rows, Up and Down are its
rows' until its ends, Page Up and Down, Home and End move by a page and to the ends, and the focused row is
scrolled into view. A `ScrollPage`'s controls are scrolled into view as the focus reaches them (its last one
with the page's end, so a note under it shows), and those out of view are passed by when the focus comes from
outside it. The scrollbar's room is reserved whether or not it shows. A select's choices (`PickerLayer`) are a
`ListView` of a set height: it is as tall as its rows up to ten (and to the room the window has under the
select, or over it), keeps that height, and scrolls for more, the chosen row in view as it opens. A menu of
actions is the same list (`Shell::pick_menu`, `Shell.picker-menu`: Play's build menu, from its ⋯ or a right
click on a build's row, `RowTouch.menu`): 200 wide with its right edge at what opened it, its rows in the
buttons' type, the cursor on the first, and those from `Shell.picker-apart` on under a line in the error color
(DELETE).

**Pads** (rule 10) follow the pad's bindings in the same way (`Bindings::pad_action`): the D-pad or the stick
moves, the button bound to A confirms and B's goes back (the south and west buttons until rebound; south and
east stand in where A or B is bound to a direction or shares a button), the buttons bound to L and R are
previous and next, Start's is the menu (closing a battle's, nothing elsewhere), the face button that is neither
A's nor B's on the east or west deletes (east until rebound), and north is a screen's second action. The
footer's hints show key caps or the pad's buttons, by which was last used: a face button by its place in the
four, since its letter depends on the pad's maker. They go through the shell's properties (`Shell.key-a` and
`key-b`, which are Enter and Esc; `pad-a`, `pad-b`, `pad-delete`, the pad's as bound, which
`Shell::show_bindings` sets), so a screen's hints say confirm and back by what does them; the Controls in
Settings say "A · Pad confirms" and "B · Pad goes back".

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
last in its export dialog's reserved line (§16); a replay that fails as it is opened or as it plays is a dialog
over the list (§16), with the replay's own still or nothing in its box (§1). What breaks a build's rules is
nettai-match's `Wrong`, which the builds' words (`builds::words`) say in their own catalog and hands to `UserError::BuildUnusable` as the reason. A game's pack that
can't be loaded is `UserError::GameOutdated`, with the game's name as shown (§1). With the app's own packs (§17),
no Legacy Collection to extract a missing pack from is `UserError::NoLegacyCollection`, whose dialog has the way
to the collection's folder, and a pack that couldn't be extracted is `UserError::ExtractFailed`, by the game's
name.

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

A pass ended before its walk is done (a terminate request from outside, SIGTERM or SIGINT, or its window closed)
writes its report as it stands, under a line that says why it ended and after which shot, and fails too: it
doesn't end as one that walked through (`tour::ended`).

**The motion probe** (`NETTAI_TOUR_MOTION=1`) is the pass with things moving. A still picture can't show an
animation, so it takes a burst of shots through each (a screen leaving and the next arriving, the cursor's
glide, a change of game sweeping in from the right, from the left and twice over, a value sliding, a press, a
list opening, a page scrolling, a dialog, the cards and the jack-in into a battle, a menu over it), each named for
the milliseconds since it began. (Before the change of game's bursts it steps once around the games, waiting for
each to be ready, so that no game's runtime loads, the window still, inside a burst.) It checks and fails on two
things: that in every shot each control is the size it has at rest, and that a press acts at once (two moves
pressed in the same instant as a screen change land on the new screen's controls, and a direction held down a
scrolling page moves the focus at every repeat). And it says, for each burst, how soon the window's first frame
after the press began and how long the app's work for that frame was (`App::first_frame_after`): a press that
holds the window shows there, though the shots, each of which holds it some 60 ms itself, can't.

A screen's steps are a file of its own, `src/tour/<name>.rs` (a child module of `tour`, so it has the pass's
own `write` and `compare`) or the screen's own (`screens/play/tour.rs`), with a function for the pass and one
for the motion probe, each called from `tour::walk` and `tour::motion_walk` with one line
(`practice::steps(&mut add, lang, size)`; Play's take `walk`'s `add`, a step and the time it is given first,
and its `shot`, a shot's name and its place in a pair). A pair's second shot may be given more to differ in
than the two cursors (`practice.rs`'s `Differs::Picture`: the game's picture where the practice menu leaves it in view, as the game moves its
backgrounds through its pause, each box given exactly so that none of the picture under the panel is let
differ; `replays.rs`'s `Also::Playback`: the picture, the bar and its two times; `Tour::may_differ`, for the
next pair of `walk`'s `shot`: the NaviCust's strip, which shows the program under the cursor), found from where
the screen's controls are. A step that must wait for something that comes by itself says so
(`Tour::wait_until`: the step after it runs once it has come, or after a minute). Where the shots after it
depend on its having come, the walk says when it didn't (`Tour::never_came`, which the report lists and the
pass fails on: Play's `reach`), so that a picture of the wrong thing isn't taken for the right one.
`NETTAI_TOUR_ONLY=replays` walks Replays' steps alone (it has none for the motion probe).

The pass has nothing of the player's to go by: its settings are the defaults, and Play lists builds of its own
(and one that can't be used) in place of the player's builds, which Practice offers as Play lists them. The builds Play lists
outside its own walk are the pass's own too, in `<folder>/builds` (`paths::keep_builds_in`), made there through
Play as a player makes them, and Replays lists replays of the pass's own.

**Practice's steps** (`tour/practice.rs`) walk Play's PRACTICE (the two cards, then the battle with the practice
menu open over it as it starts, the focus on RESUME, a step checking both), the menu's Dummy's build (a value
apart, the set started over; its list; a list of 24 names, scrolled), the battle, and the practice menu (each
Battle row with its value changed, a pair apart; the Arena's and Background's grids and Music's list, and a
background chosen from its grid, a step checking the list closed and the place moved on one), and drive the
options in the battle as a player would, the buttons held as the shell holds them: the dummy shot with Your HP
and Dummy's HP on Refills, a shot after the hits and one after a rest; the custom gauge full, L, and the custom
screen dealing in folder order; the dummy's Action Fights with its Level under it. Then EXIT, a step checking
Play is shown; and the way to Builds' editor and back: with the pass's new builds kept as files in its own
folder (Practice lists them as a player's: `Shared::tour_files`), PRACTICE on the first, the Dummy's build
stepped to the second, its EDIT focused, the editor it opens on that build (‹ PRACTICE), and Back, the practice
battle again with the menu open and the dummy still on that build; a step before each picture checks the walk
is where the picture says, and the report says so if not. `practice.txt` in the pass's folder says what no picture does at
each: both navis' HP, the gauge, the options, and each side's deal beside the top of its folder as saved; that
is said, not one of the pass's own checks. The pass needs the screen unlocked: a window on a
locked screen draws no frames, and the practice battle never starts (`practice.txt` says "no battle").

**The builds' steps** (`tour/builds.rs`; `steps` is handed `walk`'s `shot`, `motion` the probe's `burst`):
Play's builds with none and with three (one named in Japanese, one with a name too long for its row), a new build's
dialog with the focus a control on, a file that is no save dropped on the list, EDIT and ⋯ a focus move apart,
the build's menu open (`builds-menu`), a build's rename and the question before it is deleted, each chosen from the
menu (the same steps on a phone, whose page has the same buttons), the editor opened by EDIT: the Navi tab with the focus a card on, with another navi chosen and with the
other preset of the game's choice, and the Folder tab (an entry, the library, its controls a focus move apart,
the library by MB so that what can't go in says why, a chip the rules refuse asked for, a chip put in, a search,
the Regular chip and two tag chips marked, a mark the rules refuse with its reason, a chip taken out
(`builds-folder-short`: the empty entry's strip says the folder's size, Chips in the error red), the entry of the
other version's Giga chip put in before (`builds-folder-mixed`: its name red with every thing of either version
the build holds, its strip saying the mix), the
sort's list), then the tab after it; last, the selector of forms (`switched`: a build of the game's first version
made beside the walk's; `builds-forms-focus`, its first form focused, ready, the note under the selector;
`builds-forms-mixed`, the other version's first form of the first fact chosen, every form of either version the
build has in the error red and the note's place saying the mix the ruleset has against the form focused;
`builds-forms-back`, the first version's again, ready, the note again; a step after each checks the focus and the
build's state are as the picture's name says); then a ruleset's page
(`rulesets`): from the Navi tab EDIT by the build's ruleset (`rulesets-preset`: Masters class, read alone,
the reserved line saying so), DUPLICATE (`rulesets-own`), its chips (`rulesets-chips`, chosen by the category's
select's choice: a phone has no list of them), the first chip held back and the first version's quick choice
(`rulesets-chips-held`), and Back, the Folder tab marking what the copy holds back (`rulesets-folder-marked`); the
game's first preset is picked again after, so the next language's walk is as this one. A game may have one navi or thirteen, a choice or none, four tabs or five:
a step names the control it is for and does nothing where the game has none. The probe's bursts are the
editor's body arriving as the tab changes, a chip the rules refuse (its row shakes) and a chip put in (it flies
to its entry, the entry flashes, a count that changed ticks).

**The NaviCust's, Patch cards' and Auto battle tabs' steps** (`tour/build_tabs.rs`, a build of the pass's own
made through the screen): the NaviCust's list a focus move apart (a pair), a program taken in hand, moved and
turned, put down on the command line, a plus part beside it, another program put off the command line (its bug
appears), one put over another (refused, and why), a plus part on EXE6's ring (out of bounds; in a game
without a ring the shot is of the piece in hand), and a placed program under the cursor (Del's hint), picked up
(In hand) and put away into the list with Back (no longer placed), and a placed plus part's row under the cursor
(Confirm's Take, Del's Take off), Confirm on it (a second copy in hand), that copy put down where it fits
(`tabs-navicust-twice`: two copies on the board, the row's Placed ×2) and Del on the row (the last copy off); the
patch cards' library, its search and sort a focus move apart (a pair), three cards put in, the cursor on one that
is in (Confirm's Remove) and Confirm taking it out, the library by MB with a card the MB left can't take refused,
one of the build's moved up and taken out, the sort's list; a card that brings a bug, put in where the build's
cards bring none yet (`bug_card`), shot on the Patch cards tab (`tabs-cards-bug`: its bug in the strip and in
Together) and on the NaviCust tab (`tabs-navicust-card-bug`: not in the NaviCust's Bugs, §8); the auto battle's
places a focus move apart (a pair), three filled from the library, one emptied, the places scrolled to the last
list, the library of a place in each list held to one class (`tabs-auto-mega`, `tabs-auto-giga`,
`tabs-auto-advance`: its Mega chips, its Giga chips, its Program Advances) and a Program Advance put in its place
(`tabs-auto-advance-put`); on a desktop, a click on one of the build's rows in each tab, shot with the pointer gone
on to another row (`tabs-folder-clicked`,
`tabs-cards-clicked`, `tabs-auto-clicked`: the cursor hidden, the row clicked marked, the other lit by the
pointer), and the keys after the clicks (`clicks`); last, a new build made beside the walk's with a chip of
another class in each list held to one (`wrong_classes`: `tabs-auto-wrong-list`, Play's list counting its
problems; `tabs-auto-wrong`, its tab flagged, the places' names red, the strip saying
why), then two builds of their own with a problem on another tab (design §2.66): the walk's build with its first
program placed again over itself (`overlap`: `tabs-navicust-overlap`, the program's name red and the strip saying it), and a new build of the first version
given the other version's variant of a card (`card_mix`: `tabs-cards-mixed`, the card's name red and its strip
saying the mix); on a
desktop, the pointer on a row
describing it in the strip, the focus where it was (`hovers`: `tabs-hover-chip`, `tabs-hover-program`,
`tabs-hover-card`, `tabs-hover-installed`, a step after each checking the strip is the row's and the focus
unmoved; the pass's pointer moves are the window's, the pointer marked the last thing used), and a program's colors
(`program_colors`: `tabs-navicust-colors`, Right choosing the second, `tabs-navicust-color-chosen`, and Enter taking
that color's part, `tabs-navicust-color-held`). Where a piece goes is found from
what the view shows of the board (`put`: the first cell where the piece in hand would be off the command line,
on the ring, over another, or anywhere on the board itself), so the walk is the same whatever the game's shapes. A tab the game hasn't is
passed by (`to_tab` sets `Tour::passing`: the steps after it do nothing and take no picture until the next tab):
EXE6's walk has no auto battle pictures. The probe's bursts: a piece taken up, moved, turned, compressed and
decompressed (turned as it was, in place), the command line's arrows pulsing by themselves, a piece put down, put
off the command line (a bug pulses), refused; the mouse carrying pieces by clicks (the window's pointer events, by
timers, as Replays' bar is dragged: a press moved with the button held and let go, then a click putting it down,
`navicust-press-move`; a click, moves and a key, `navicust-click-move`; a program's row clicked and the copy clicked
down; a click over another piece, then off the board; a right click; a click off the board, and one on the list,
`navicust-click-list`, the list's remove look fading in; the wheel); a folder entry dragged onto the library
(`folder-drag-remove`) and a right click on another; a card put in and one refused; a chip put in a place.

`NETTAI_TOUR_ONLY=<part,...>` walks those parts alone: `shell` (loading, the menu, the screens as they open,
Settings, a dialog, a battle; in the probe, a change of game too), `packs` (a pack being extracted, and the Legacy
Collection not found: the loading screen's line and button, and its dialog), `practice`, `builds`, `build-tabs`,
`play` and `replays`; `play` or `replays` named alone walks Play's or Replays' steps alone. (The pass extracts
nothing: its packs are `$NETTAI_PACKS`'s, and the extraction's screens are set on the view.)

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
search; the Online filter; a file that doesn't read, selected as the screen arrives from the main menu (its box
empty, nothing of the menu's reel in it: §1); the delete question, and then a delete carried out, of a
file made for it; the export's dialog as it opens (a pair), as an export under way looks and as a failed one
does (both set on the view: no export runs); the two cards; the viewer held a little way into the second round
(a pair a focus move apart, and a pair a second of playback apart); five seconds on; the other console; both
builds over it; the export from the viewer; a closed-builds match's viewer (BUILDS unavailable); a replay that
fails as it is watched (`replays-watch-failed`: a copy of the first, damaged after the list read it, its size
and time kept so that the list still offers it, watched as the screen arrives from the menu: the error's dialog
over the list, the replay's box empty; the copy is removed after); and on a phone the selected replay's sheet (a
pair). A shot that follows a change of the selected replay waits for that
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

**The showreel** (`tour/reel.rs`): `NETTAI_REEL=<out.mp4>` with the pass's folder, in a build with the `tour`
feature, walks a scripted demo of the app with everything moving and sounding and writes it to an MP4 (H.264 and
AAC through `ffmpeg`, `$NETTAI_FFMPEG` else the one on the path), so that the reel can be made again as the app
changes:

```
cargo build -p nettai --release --features tour
NETTAI_PACKS=data NETTAI_TOUR=/tmp/reel NETTAI_REEL=/tmp/reel/nettai-showreel.mp4 target/release/nettai
```

It shows, in about 90 seconds: the main menu with its demo battle and the title music, a change of game to EXE5,
EXE4 and back to EXE6 (the ground sweeping in, each game's music and demo battle); Play's PRACTICE, the cards,
the practice menu open as the battle starts, the arena stepped there and its Music stepped to a battle theme,
which plays, the dummy set to Fights at Hard, and a fight in which the player's side
presses by itself (the custom screen picked through as the bots pick, chips used, a move, a charged shot, the
custom screen opened once the gauge is full); Play's builds, the editor's Folder tab (a chip put in from the
library), the NaviCust (two programs taken and put down) and the patch cards (three put in), then EXE5's builds
and the Auto battle tab (three places filled); Play, an invite with a practice session beside it, the second
player joining by its code (`screens/play/tour.rs`'s `reel`), both ready, the cards and a few seconds of their
match; Replays, the list, a replay watched, a seek further on and twice the speed; About, from Settings.

- **Time is the reel's.** The app runs a frame at a time: each frame captured moves its clock on by exactly
  1/60 s whatever the capture costs, so the video plays smoothly at 60 frames a second however slowly it was
  made (a release build makes it in a few minutes). Slint's clock (its animations and timers, among them the
  walk's steps) is the reel's through the window's backend, wrapped (the winit backend, built as `main.rs`'s
  selection builds it, its `duration_since_start` the reel's); the app's (a frame's time, a screen's change, a
  change of game's sweep, the cards' time up, a pick's delay) through `crate::clock::now`, which is the system's
  but in the reel. Only the frames the reel captures run the app's frame: the window's own draws show them, and
  the timer that runs frames for a window that isn't drawn is off.
- **The sound is what the device would play**: `Sound::frame` hands the reel the samples it queues, at the
  device's volume (the menus' sounds and music, a battle's, a replay's), and each video frame is written with
  1/60 s of them. The sound is written 640 samples (20 ms) behind the picture, a battle tick's and a little, so
  that a frame in which no tick ran doesn't leave a gap.
- **What is only waited for is cut out.** The walk is off the air while the games load, its builds and replays
  are made and the Replays screen plays them out, and while the other player's link is undone and the editor
  is left: the clock runs, nothing is written, and the video cuts from before to after.
- **The size.** The window is 1280 by 800 units at the display's pixels (2560 by 1600 at a scale of two),
  scaled to 1920 by 1200 (Lanczos) and encoded at CRF 21: some 35 MB for the reel.

The reel is the pass, held still as it is but for motion (on, as the probe's) and the battles (the menu's demo
battle plays, and a replay's viewer: `Shell::reel`): the settings are the defaults and never written, the builds
and replays are the pass's own (`<folder>/builds`, `<folder>/replays`), the demo battles' seeds are fixed, and
the pointer is ignored. `reel.txt` in the folder says what was written: the frames and their size, the samples,
how many were silence for want of any heard, the frames run off the air. The process ends with status 1 if
ffmpeg failed.

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
3. **The Slint is written to stay small in what Slint generates**, and a screen's author keeps it so by these
   rules, measured in the size pass below (widgets.slint's comments say the same):
   - *A component is shared only where sharing pays.* Slint compiles a component as one of its own, shared by
     its uses, unless a use has children: then it copies it into that use. Shared, a component costs its own
     functions (its item geometry, accessibility and tree functions: a few KB however little is in it) and each
     use about 2 KB of calls into them; copied, it costs what is in it again at each use. So a component used
     in one place is copied in: its use is given a bare child, which draws nothing and takes no input,

         if !root.covered && Shell.screen == Screen.menu: MenuScreen {
             Rectangle { }  // (Copied in: docs/app.md §14.)
         }

     (one compiled as its own cost 16 to 23 KB more). So is a small one, two or three elements, used in a few
     places (a line, a row's mark, the cursor's corners). One with more in it used in several places stays
     shared: the header copied into its seven pages would be 550 KB more, a library pane into its two uses 130
     KB, a field into its eight 320 KB. A component whose root is a text, a path or a touch area can't be given a
     child, and one with no `inherits` can't either (`Tween`); those stay shared. Where it isn't plain which is
     smaller, measure (below).
   - *A container holds what must be inside it, and places a leaf for the rest.* A component with something put
     inside it is copied into each use, so it keeps only what its children must be inside of: ScrollPage and
     ListView are a flickable and the box the rows go in, and what they do besides (the focus's registration,
     the reveal, a list's rows in view and its chosen mark, the scrollbar) is a `Scroller` they place, compiled
     once. A use is 16 KB a page and 18 KB a list beside its rows (they were 28 and 41).
   - *Nothing on a component's root that forces a copy it doesn't need:* `visible`, `opacity`, `clip`, a drop
     shadow or a size in percent on the root element of a component makes Slint copy it into every use. They go
     on an element inside it, or on the use.
   - *An element that draws nothing says where it is.* One the source leaves `x` and `y` off is put in the
     middle of its parent by two bindings at every use (and their expressions again in the user's geometry): the
     clocks state `x: 0; y: 0` (a use was 3.4 KB, and is 2.1; their 87 uses were 120 KB of that).
   - *`visible`, not `if`, for a small part* shown or hidden; an `if` or a `for` only where a part is truly
     there or not (a screen, a dialog, a list's rows), and one for the parts shown on one condition (a screen and
     its overlay). One around a rectangle is 16 KB; an element given `visible` is a clip element more, 2.4 KB.
   - *A long list is a `ListView`*, whose rows exist only in view.
   - *A part written twice is one component* (the three libraries' heads were three copies).

   What each thing costs in the generated Rust, a use (bn6battle-verify's tools/slint-size/cost.py, which adds
   ten of it to the main menu's screen):

   | | |
   |---|---|
   | a rectangle, placed by constants / by bindings | 1.0 / 1.8 KB |
   | `visible` on it, `opacity` on it | 2.4, 1.2 KB more |
   | a text | 3.6 KB |
   | a use of a shared component, for its calls alone (a clock) | 2.1 KB |
   | a `Label`, a `Button` | 7.2, 9.4 KB |
   | an `if` around a rectangle | 15.7 KB |
   | a `ListView` beside its rows; a `ScrollPage` | 17.7, 15.5 KB |
4. **One renderer and no more features than are used:** femtovg alone (the clock needs it: §1), without the
   software renderer and the accessibility backend the first app also built. No standard widgets are imported.
   The fonts are embedded as files (7.4 MB in all: 3.9 of it M PLUS 1 Code and 1.8 M PLUS 1 at 800, each with
   its Japanese), which costs the linker nothing measurable.
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
on, before Practice and Builds were merged into it: dev profile, sccache off, the two crates cleaned with their
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

**What a screen costs: Practice** (§15; measured 2026-10-12 with one script, back to back, the tree before it
and the tree with it; sccache off; CPU is user and system time together, and the one to compare: the wall
times follow the machine's load, 129 to 206 before and 166 to 197 after):

| | Before Practice | With Practice |
|---|---|---|
| a clean build of nettai-ui and nettai | 77 s, 33.4 s of CPU | 162 s, 40.6 s of CPU |
| nothing changed | 2.0 to 2.6 s, 0.7 s of CPU | 2.6 to 3.6 s, 0.6 to 0.9 s of CPU |
| after one line of Rust changed (a constant of battle.rs, and put back) | 8.8 to 11.1 s, 5.8 to 6.0 s of CPU | 17.4 to 20.3 s, 6.9 to 7.1 s of CPU |
| after one line of menu.slint changed (a value, and put back) | 20.7 to 31.9 s, 6.5 to 7.5 s of CPU | 38.6 to 40.0 s, 8.2 to 8.3 s of CPU |
| after one line of practice.slint changed (a value, and put back) | | 33.7 to 43.8 s, 7.9 to 8.1 s of CPU |
| the generated module | 3.33 MB, 28,204 lines | 3.88 MB, 31,925 lines |
| the Slint | 4,316 lines, 18 `if`s, 6 `for`s | 4,972 lines, 17 `if`s, 7 `for`s |

Practice's 656 lines of Slint are 0.55 MB of generated Rust: the entry 0.24 MB (the main menu's screen is 0.23),
the practice menu and its rows 0.22, the input display 0.09, the entry's cards and rows 0.07, less the 0.17 its
stub was. What kept it there: the entry's parts are shown and hidden, not `if`s; the menu's rows are one
component under one `for`; the input display is in the window throughout and shown when it is on, where an `if`
was 46 KB more; Practice has no overlay (an empty one under its `if` was 22 KB); a card places its three texts
itself, where two layouts were 9 KB more; the display's arrows are the text face's, not paths (11 KB). (The entry
went with docs/design/app-ui.md §2.57: Practice is now the menu and the input display alone.)

The follow-ups after it (the phone's picture edge to edge, a select's list scrolling, a terminate request, a
game that can't be loaded told; measured the same way, load average 260 to 336) cost 0.06 MB of generated Rust
(3.88 to 3.94 MB: the select's list as a `ListView`) and nothing measurable in time: a clean build 35.5 s of CPU
before and 36.7 after, one line of Rust 6.1 and 6.5 to 6.8, one line of a .slint file 7.0 to 7.1 and 7.3 to 7.6.
A practice battle's figures (§5) are the same before and after them: 107.6 to 120.0 frames a second and 117.6 to
120.0, a key to its picture 11.9 and 11.1 ms against 11.0 and 10.2 (two runs each, back to back).

**What a screen costs: Builds** (§8; measured 2026-10-12 with one script, back to back: the tree before it,
nettai-ui at aeaad46ce, and with it, ui-builds at 5a4e5ffd1; sccache off; CPU is user and system time
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

Builds' 2,150 lines of Slint are 2.2 MB of generated Rust, four times Practice's for three times its Slint
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
in Builds; 16 to 18 since the size pass, below), and each `if` page holds the parts it lays out.

**What a screen costs: Replays** (§16; measured 2026-10-12 with one script, back to back: the tree before it,
nettai-ui at a8087d1d4, and with it, ui-replays at 7bebca494; sccache off; CPU is user and system time together,
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
before the builds over the viewer and the merge of Builds (against nettai-ui at aeaad46ce, load average 176 to
320), Replays was 1.33 MB (3.88 to 5.21) and a .slint edit 7.1 to 7.5 s of CPU before it, 8.9 to 9.1 with it.
One thing was made smaller after measuring: the desktop's transport and the phone's were two components under an
`if` each, and are one whose other shape's parts are hidden (`visible`, and passed by the focus). That was only
62 KB less, not the 0.21 MB the two `if`s seemed to be: most of a conditional is the item tree of what it holds,
and the viewer's page's `if` describes the transport's items now (44 KB before, 133 after). What stays: each `if`
here is a part that is truly there or not (the list's page and the viewer's, each with a `Picture`, which two of
can't be at once; the panel beside the list, whose `Picture` a phone's sheet has instead; the sheet, the dialog,
the builds over the viewer), and each costs about as much as its own component again.

**The size pass** (2026-10-13). With the four screens built the generated module was 11.64 MB (93,741 lines;
main at a4eceae77), past the first app's 7.4. It was brought down to 10.24 MB (83,010 lines) without changing
what is drawn or how anything behaves, by the rules above; sizes, not times, are what it was judged by (the
user: "stop measuring build times that stuff takes forever for no reason"), each from the UI's build script run
on the Slint alone. Each step, measured against the tree before it:

| Step | Less |
|---|---|
| the clocks state their place | 0.12 MB |
| `Scroller`: a page's and a list's machinery in a leaf, the list's mark with it | 0.25 MB |
| a screen and its overlay one conditional of the window | 0.04 MB |
| the 65 components used in one place copied in | 0.81 MB |
| the small components used in a few places copied in | 0.12 MB |
| the three libraries' heads one component | 0.07 MB |

What the 1.40 MB less is, by the code Slint writes:

| | Before | After |
|---|---|---|
| components of their own; conditionals and repeaters | 161; 96 | 89; 93 |
| the items the item trees list | 3,729 | 3,824 |
| bindings and each component's setup | 5.68 MB | 5.17 MB |
| the item trees (each conditional's and repeater's, of all it holds) | 2.16 MB | 1.91 MB |
| the per-item functions (geometry, accessibility, layout) | 2.62 MB | 2.18 MB |
| a shared component's plumbing | 0.55 MB | 0.39 MB |
| the rest (the globals, the window) | 0.62 MB | 0.57 MB |

What was looked at and left: the header and the footer once in the window, not in each of the seven pages
(about 0.2 MB: each screen's own header is made with it, which is what wipes its title in and registers its
Back, and its title and hints are the screen's bindings); Settings' column as positions stated, not a layout
(about 0.06 MB, with the layout's own arithmetic to match to the pixel); the text styles (`Label`, `Value`,
`Mono`, `Note`) copied in by an `opacity` on their root (a Label so is 0.13 MB more, a Value 0.01 less); the
accessibility Slint writes though its backend is off (its functions are 0.83 MB, nearly all of it each item
tree's own share whatever is in it: every text, field and image given no role is 0.03 MB less); the whole UI
copied in (Slint's `SLINT_INLINING`: 22.9 MB).

Practice's and the battle's files were left to last while other branches changed them. With those merged (main
at 14bfaf42b, 10.29 MB: Practice's options had added 0.05 MB) their three components used in one place were
copied in (the menu's row, the connection's mark, the touch pad): 10.26 MB, 83,075 lines. Their components used
in a few places are smaller shared: copied in, the pad's keys would be 0.22 MB more, the input display's keys
0.06, a build's card and option 0.02 and 0.01, an input row 0.02.

The games' looks (§10; main at dc55c1752, 10.21 MB, 82,825 lines, after the main menu's caption went) are 0.14
MB more: 10.35 MB, 84,686 lines. The ground's component under its `for` is 0.08 MB (the three pattern paths of
its layer 0.02, of which a starfield uses all three and the others one); the theme's colors, which were constants
Slint wrote into each use, are read from the theme now, about 0.05 MB over their uses; the theme's own code 0.015
more. The display texts' weights from the theme cost nothing measurable (0.004 MB less than constants).

The tools that measured it are bn6battle-verify's tools/slint-size: the module's size from the build script
alone, its breakdown by kind of code and by component, what one component's code is, a construct's cost, and
the components used in one place (given the child, or measured copied in).

## 15. Practice

docs/design/app-ui.md §6, "Practice": you against a dummy set up from a menu while practicing. It is
`src/screens/practice.rs` and `ui/screens/practice.slint` (the practice menu and the input display).

**No screen of its own** (docs/design/app-ui.md §2.57: the user, "i feel like the practice button is in a weird
place", then the whole play/build/practice flow rethought, "sure do that"). Practice's entry is gone (its rows,
the two builds facing each other, its running arena preview, START PRACTICE, its EDIT and its music). Play's
PRACTICE checks the chosen build as HOST does (`Play::side`: "Make a build first.", or why it can't be played, in
Play's reserved line) and calls `practice::open(shell, game, build)`, which starts a visit with that build (by its
name) and goes to `ScreenId::Practice`. That screen has no page (`Screen.practice` shows nothing in app.slint):
shown, it starts a session of its own (`Practice::start_own`) whose battle fills the window, after the two cards
and the jack-in as any battle's. Back (before the battle has the keys) and the menu's EXIT go to Play, whose
build is still chosen; leaving Practice ends its session (`Practice::hide`). A session that can't start (the
build gone, or no longer one that can be played) goes back to Play with the failure's dialog.

- **What it starts on** (`Practice::start_own`): the player's build the visit's (Play's chosen one; shown from
  elsewhere, the screenshot pass, the one Play remembers, `play::remembered`, else the first that can be
  played), the dummy on the visit's dummy's build (at first the player's own), the place, the music and the
  menu's choices the visit's (at first Random, Random, "Game's own", Stands still, Normal, every option normal,
  the input display off). The builds are read as Play lists them (`play::shelved`), and those that can be played
  are what the menu's Dummy's build offers (`practice::Pick`: a build and its place among the game's files).
- **The menu opens as it starts** (the user: "when practice mode starts you should also bring up the menu
  immediately"): the session's `Start::menu` has `Practice::battle` open the practice menu at `Started`, over the
  battle held (below), its focus on RESUME; the battle steps through its opening under it and waits
  on its custom screen. RESUME starts the fight. The session beside a waiting invite has no `menu` and starts as
  before, and RESTART doesn't open it.
- **The battle's music** starts with the battle, as any battle that plays the pick (§1, "The battle's music"):
  nothing plays it before, and the title music comes back on Play after.

**A visit** (`practice::Visit`, in `Shell::practice`): from Play's PRACTICE (`Visit::new`) until a screen other
than Practice and Builds is shown. Builds is part of it: the menu's EDIT goes there and its Back comes back
(`Practice::away` keeps the visit through Practice's leaving for it; any other screen shown ends it,
`Visit::shown`, as Practice leaving for one does). The player's build, the dummy's build, Arena, Background,
Music and the menu's choices are the visit's: each visit starts on the defaults (the user: "going into practice
should reset all the arena settings, there's no reason for it to remember the last settings":
docs/design/app-ui.md §2.40; "yes for the practice options, they should all reset to default", "you should also
not remember the last dummy build either": §2.54), and through it every session, and the session started again
back from the editor, keeps what the player set. None of it is in the settings.

- **The Dummy's build** (docs/design/app-ui.md §2.57) is the menu's first row, a `Select` of the game's builds
  that can be played, by name, the player's own at first: Left and Right step it, Enter, a click or a tap opens
  its list. A change starts the set over with the dummy on it, as Arena's does (`restart_set`: the session's
  match's side 1 the new build, its set made again from the session's seed and played from its next start, the
  menu staying open; what the new build draws that the battle wasn't made ready for is loaded as it is first
  drawn, as for a new arena). The menu's line under its title says the two builds' names.
- **EDIT** under it, a button in the controls' column (`PracticeOption::EditDummy`), opens the dummy's build in
  Builds' editor (`screens::builds::open(shell, ScreenId::Practice, at)`, by its place among the game's files;
  the editor's Back says ‹ PRACTICE). Practice is left (its session ends; `away` keeps the visit), and Back from
  the editor shows Practice again, which starts a session on the visit's choices with the builds as the edit
  left them, the menu open (chosen in docs/design/app-ui.md §2.57: an edit changes a build the battle was made of,
  so the battle starts again, as a change of the Dummy's build starts the set over). A dummy's build that can't
  be played after the edit is offered no more, and the dummy is on the player's build; where that is the one
  that can't be played, the session doesn't start, and Play shows with the failure's dialog. A build with no
  file (the screenshot pass's own) is refused.
- Arena and Background (the user: "in training mode can you also add arena + background select?") are where
  every round of the session is fought (`practice::Places`, from the content alone): Random first, a
  place picked from the seed each round as a match that names none has it (what practice did before; a
  background from the whole background pool, §1, "The battle's background"), then the game's arenas, the link
  battle stages a match may name (`nettai_match::link_battle_stages`, the rules' `link_pick.match_stages`:
  EXE6's 96, EXE5's 94, EXE4's 96) each arena once (below), and the background pool, every game's battle
  backgrounds (`nettai_match::ids::pool_backgrounds`: 70, BN4's, then BN5's, then BN6's, in any game's practice),
  each in its keys' natural order (a run of digits by its number: `netbattle-2` before `netbattle-10`). Arena was
  an arrow pager (the user: "the arena names are meaningless so you should just have like a pager widget that goes
  through them") and Background a dropdown (docs/design/app-ui.md §2.43); since §2.56 ("arena selector should be
  like a dropdown that shows previews of all arenas. background dropdown should be similar and show preview of all
  backgrounds") both are a pictured `Select` in the menu (`pictured`: the chosen one's picture, fifteen sevenths of
  the box's height less 12 wide, before its name; Random a question mark), and Enter, a click or a tap opens the
  grid of every choice's picture (`Shell::pick_grid`, below); Left and Right still step it, around from the last
  to Random. Each is the visit's (above): Random as a visit starts, kept through it, a choice of another game's
  Random (a match names a background by the name it writes, the game's own by its name in the pack, another
  game's with its game, `exe4:acdc-area`).
  - The two are apart in the games' own data: a link battle picks its stage and its background each on its
    own (`link_pick`: EXE6's and EXE5's `PosRNG1` then `PosRNG2`, EXE4's two `PosRNG2` picks), and a stage's
    own background is shown only where the rules list none to pick from. So either is chosen with any of the
    other, and nothing ties them.
  - Each arena is listed once (the user: "don't show duplicate arenas in the practice mode selector, just
    show the first of any duplicates", docs/design/app-ui.md §2.47). The games' link battle stages hold an
    arena more than once, as records that differ in nothing but the order their actor lists place in: EXE6's
    each arena twice (netbattle-1 and 2, 3 and 4, ...) or four times (1, 2, 89 and 90; 89 and 90 the same as 1
    and 2 to the byte), one record placing side 0's navi first and the other side 1's, EXE5's and EXE4's
    alike. `Places::of` groups them from the content as it reads it (`practice::same_arena`, no list kept):
    two records are one arena when everything their `StageData` holds is the same, the layout, the panel
    pattern, the music and background they state, the mode and the battle number, and their actor lists hold
    the same entries, in any order; the effects aren't compared (EXE6's netbattle-95 and 96, and EXE5's 93 and
    94, differ only in them, the second lacking the first's 0x800, a bit nothing reads, and the user: "they
    should count as duplicates too"). Each group shows as its first record in the list's natural order, so
    EXE6's 96 are 47 arenas, EXE5's 94 are 47 and EXE4's 96 are 48 (the select's "n / 47" in EXE6). A
    round that names a later record (a session another screen starts on it) shows as its arena, which plays
    its first (nettai's `each_arena_is_listed_once`).
  - The actor list's order is the order the round's setup places the navis and objects in, so the order they
    are updated in: an arena chosen always plays its first record, so it always has the same side first (side
    0's navi in the three games). Random is the rules' pick over every record, as a match's, so a Random round
    keeps the original's coin flip of which side goes first. Play's matches and their random pick are as
    they were.
  - Their words: an arena is said by its place in the list, "12 / 48" (a stage is its key in the content,
    and the user found the keys' names meaningless; its cell in the grid says the same, under its picture); a
    background by its game's name and what that game's locales call it in the language (`[backgrounds]`, by the
    area it comes from: `backgrounds::name`), "BN4 ACDC Area". Every background of the three games had its name
    in English and in Japanese already.
  - A background chosen keeps the arena: with Arena Random the seed picks each round's arena whether the
    round names its background or not (nettai's `a_background_chosen_keeps_the_arenas`).
- **The grid** (`Shell::pick_grid`, the shell's `PickerLayer` with `picker-grid`; `Look::Grid`): every choice
  as a cell, its picture 120 by 56 (seven fifteenths of the cell's width: the field's band, which a display of
  two pixels a unit shows pixel for pixel) over its name on two lines at most, Random first with a question mark. It
  opens under the select (over it where there is more room), five cells to a row, 8 apart, in 8 of padding with
  the scrollbar's 12 at the right (656 wide), showing as many rows as there are up to five and as fit
  (`Grid::rows`); on a phone a sheet from the bottom, three to a row sharing its width, three fifths of the window
  at most. It is a page (`pick.grid`) of cells each focused (`pick.grid.cell`, by the choice's place): the arrows
  go to the cell beside, Up and Down through the rows scrolled out of view too, the cell scrolled into view; the
  cursor brackets the cell; Enter, A or a click chooses (the screen hears `choose` as from a list), Esc or B
  closes. The chosen cell is outlined in the cursor's color over its tint. It drops in as the lists do, and
  with Reduce motion is there at once. Its sizes are `GRID_*` in shell.rs, which `PickerLayer` lays out by.
- **The places' pictures** (`screens/practice/thumbs.rs`, `Thumbs`, in `Shared`): an arena's is a round on its
  first record between two MegaMen (a new build of the game's first version, on both sides: never the player's
  builds, a save or the dummy; the user: "you should just render the arena and megaman only without his hp",
  "you should show the opponent too as megaman on the other side. it should always be megaman as well, don't use
  the training dummy or saves"), played to its opening's end (at most 300 ticks, `thumbs::OPENING`), and drawn by
  nettai-render's `still::field`: the field's panels, the two navis, each with its own shadow, and the objects
  the arena places as its round starts (the user: "objects don't seem to be rendered on stage previews"), from
  the local side, and nothing else (no background, no HUD, no other object; clear where nothing is, the cell's
  inset showing). The arena's objects are those of the kinds its stage's actor list places beside the navis
  (`thumbs::drawn`, from the content: `StageData::actors`' `Place::Kind` entries, which `Battle::spawn_actors`
  places through each kind's `place`), drawn as the round has them by their own sprites, the battle's path:
  EXE6's cubes, ice and Guardian statues and its boulders, EXE5's the same, EXE4's boulders, gears and flags (its
  windy arenas' wind is unseen, and the flags' HP is HUD, left out). A background's is its first frame
  (`still::background`). Each is the battle picture's rows 40 to 152, 240 by 112: the tallest thing on the field
  whole, EXE4's flags on the back row reaching up to row 40 and its field's front edge down to row 151 (until the
  objects were drawn the band was rows 56 to 152, 240 by 96, which cut those flags, the cubes on EXE6's back row
  and the navis of the arenas that start them there). A game's are made on a thread of their own
  (`Thumbs::want`) when a session of it starts: the backgrounds first (they need no runtime), then the content's
  runtime loaded on that thread, then each arena; in a dev build about 20 ms, 0.25 s and 0.3 s for EXE6's 47, so
  they are there well before the cards are done, and opening a grid never waits (a cell not made yet is empty and
  fills in as its picture comes, `Thumbs::poll` each frame, which tells the selects their pictures too). They are
  the same whatever the player's builds, and kept for the session by game (the pool's backgrounds once for all
  three): nothing is written to disk. The screenshot pass makes them on the window's thread at once.
- **Music** is the battle's music the player picked (§1, "The battle's music"), the one pick Practice's sessions
  play, the visit's ("Game's own" as a visit starts): a dropdown as Background's, its list "Game's own" then the
  themes, stepped by Left and Right around from the last theme to "Game's own", and Enter, a click or a tap opens
  its list.
- A build is checked as the lobby will check one (`nettai_match::build::Checked`, through Play's list), and the
  match of the two (`nettai_match::check::problems`) as a session starts.

**A session** (its interface is §2's "A practice session, from any screen"). `practice::start` checks the
match of the two builds (`nettai_match::check::problems`), shows the two cards while the library's `Preload`
makes it ready, and plays the set with `LivePlayer::against(set, opponent, seed)`. It isn't recorded.

- **Its menu holds it, wherever it opens.** The user, of the menu as it was (the game's own pause mid-fight,
  the battle held still elsewhere): "pause during custom screen vs not custom screen is so janky now"; of three
  ways put to them they chose to always hold the battle itself and keep drawing the background's animation and
  scroll and the panels' animation on a clock of their own while the menu is up, so it looks like the game's
  pause on every screen (docs/design/app-ui.md §2.57, "One hold, everywhere"). Before that the user had asked
  for the game's pause ("the training pause should actually just use the game's pause behavior: music
  continues, stuff still animates. but don't show the PAUSED banner in the middle of the screen"), and it held
  the fight alone. `battle/hold.rs` does it now:
  - A practice set's rounds have the host answer START, as every match's (`HostKeys::pause`:
    `practice::set_of`): START in the fight is reported (`Player::take_pause_requests`) and opens the menu, and
    the game's own pause never runs (so there is no PAUSED banner to leave out).
  - Under the menu (opened by Esc, the pad's Guide, MENU or START), mid-fight, on the custom screen, in a
    dimming, at a round's end or in a round's opening, the battle is held where it stands (`Player::set_held`):
    no tick steps it, so nothing of it changes (neither side's custom screen, its cursor, the chips chosen, its
    HP bug's drain; the fight, EXE4's HP bug with it, which its own pause drains; a dimming's chip; a round's
    end), no button reaches it and the dummy's presses reach nothing (`Player::hold_opponent`). Its sound goes
    on: the audio driver plays on a tick's worth each tick due, the music and what was sounding going on, no
    cue new.
  - **Its scenery moves on** as the game's pause moves it: each held tick the renderer moves the stage's clock a
    frame on (`Renderer::scenery_frame`), the background's animation frames and its scroll (Enhanced's glide
    with it) and the panels' own animations (their palette cycles), and the picture is
    drawn anew. That is what the game's own pause moves mid-fight (its tick goes on counting the round's frames
    and ticks, which the background and the panels are drawn on; the panels' own state, a crack's timer, waits).
    The navis and the effects stand: the game's pause runs only the few objects marked to (a charge's glow, an
    aura's shimmer, afterimages), which are the battle's own and stand with it.
  - The frames the scenery moved are the renderer's alone (`Renderer::stage_clock`: the battle's clock and
    them), kept after the hold, so as the menu closes the background goes on from where the hold left it, a
    frame a tick, with no jump back; a new round or a start over forgets them. Nothing of them is in the
    battle, its snapshot or its digest, and a battle never held (every online match, every replay) draws byte
    for byte as before. The menu closed, the battle goes on from the tick it was held at.
  - **A battle started with the menu open** (Practice's own sessions, and an arena or the Dummy's build changed
    under the menu, which starts the set over) steps through its round's opening (its fade in from white, its
    navis' arrival and its banner) with no buttons, and is held as its custom screen begins, before the screen
    comes in (`Phase::Opening`): at a round's first tick the field is black and no music plays yet, and the menu
    is there to set the dummy and the stage up with them in view. Nothing in the opening is the fight's: no
    gauge fills, no HP changes. In EXE6 the opening is 136 ticks (2.3 s). The menu opened in an opening already
    under way holds it there, as anywhere.
  - Practice isn't recorded, so no replay holds a hold. The tests: nettai-frontend's
    `held_the_battle_stands_and_its_scenery_moves_on` (held, the digest and the ticks the same and the
    background drawn anew; after, digest for digest the battle never held, the scenery a frame a tick from the
    hold's; held for no tick, and started over, byte for byte), nettai's
    `a_battle_held_under_the_menu_on_the_custom_screen_and_mid_fight`, `held_in_a_dimming_and_at_a_rounds_end`,
    `a_practice_opened_with_its_menu_is_held_from_its_custom_screen` and `under_the_menu_mid_fight_the_hp_bug_waits`
    (EXE6 and EXE4).
- It has no end: when the set is decided the shell says `Finished` and Practice starts the battle over
  (`Battle::restart`), as RESTART does, and R or F5 on the keyboard and the pad's north button without opening
  the menu (unless the player bound them to a button).
- The session is the shell's (`Shell::practice`), and gone with its battle however that went: ended by
  `practice::end`, by a battle another screen started in its place, or, shown in another screen's `Picture`,
  by that screen being left.

**The practice menu** (Start or Esc; the battle held behind it, its music going on and its scenery moving, the
dummy in view beside the panel). Its
rows are a model (`PracticeView.rows`: one row component under one `for`), made from one list,
`practice::ROWS`:

| Row | Values | What it sets |
|---|---|---|
| Dummy: Dummy's build (a dropdown; Practice's own sessions) | the game's builds that can be played, the player's own first chosen | the set starts over with the dummy on it, the menu staying open: above |
| Dummy: EDIT (a button; Practice's own sessions) | | the dummy's build in Builds' editor, and back: above |
| Dummy: Action | Stands still, Moves around, Fights, A second pad | what the dummy does (the library's `Opponent`), from the next tick: below |
| Dummy: Level (under Fights alone) | Easy, Normal, Hard | how well it fights (nettai-training's `Level`) |
| Stage: Arena (a pictured select) | Random, then "1 / 47" and on, each arena once | the set starts over on the new arena, as RESTART starts it, the menu staying open: below |
| Stage: Background (a pictured select) | Random, then the pool's by game and name | the battle goes on where it is, drawn on the new background: below |
| Stage: Music (a dropdown) | Game's own, then the battle themes (§1, "The battle's music") | the pick, the battle's music switched at once (not beside a waiting invite) |
| Options: Your HP, Dummy's HP | Normal, Refills | the library's `Training::refills`, by side: the navi's HP is full again a second and a half after it last lost any (docs/frontend.md §2) |
| Options: Custom gauge | Normal, Always full | `Training::full_gauge`: the custom gauge is full whenever it runs |
| Options: Chip draw | Shuffled, Folder order | `Training::chip_draw`: both sides' custom screens deal from their folders' tops, in the folders' own order |
| Options: Input display | Off, On | the app's own drawing, below |

**Its rows' look** (`MenuRow`): bare, a group's title over each group, the hover tint on a row. Its controls are
the size the entry's were (docs/design/app-ui.md §2.44, the user's "option 1"): the steppers, the selects and
EDIT 40 tall (36 on a phone), in one column, the value's box 196 and its arrows' room 72, right-aligned with 8
to spare, so the selects and the steppers line up; a row is 52 tall (it was 44, its steppers 32 and its selects
32 in a narrower column). EDIT's row is the control's height and 12 (52; 48 on a phone), the button in the
controls' column, a ghost button with its word alone. On a phone a row is 64, its label over its control (24, then the control at 26, as wide
as the panel). The menu scrolls a little more for it, and Up and Down go through the whole page first, as before.

Arena changes the field, which a round sets as it starts, so a change starts the set over on the new arena:
the session's match is put on it, its set made again from the session's seed, and the live driver plays that
set from its next start (the library's `LivePlayer::play_set`) as the battle starts over (`Battle::restart`,
RESTART's). The menu stays open, and the battle behind it steps through the round's opening with no buttons
and is held as its custom screen begins, as above (nettai's
`under_the_menu_a_set_starts_over_on_another_place`). The dummy's setup, its recording and the options stay.

Background changes the background alone (the user: "changing the background shouldn't change the entire
arena either in trianing mode"). A round's background is in its setup (`RoundSetup::settings.background`, and
`later_stages` for the rounds after), and the battle's digest covers the setup, but no tick reads it: the
renderer alone does. So the battle goes on where it is, the arena, the HP, the dummy and the round as they
were: the set made again on the new background (its arenas the same, above) is put in the playing one's place,
and the battle's setup says its backgrounds (the library's `Player::set_backgrounds`), drawn from the next
frame, the rounds after and a RESTART on them too. Stepped on from there, the battle is the one without the
change tick for tick, its digest the same but for the background its setup says, another game's as one of the
game's own (nettai's `on_another_background_the_battle_goes_on`); practice is neither online nor recorded, so no digest of it is
compared with another's.

Music changes the battle's music the player picked, the visit's one pick (§1, "The
battle's music"): Left and Right step it, Enter opens its list, and the battle's music switches at once, nothing
else of it changing (nettai's `a_battle_changes_its_music_as_it_runs`).

The Dummy's build, Arena, Background and Music are dropdowns (docs/design/app-ui.md §2.43, §2.56, §2.57): the
row's value is the `Select` box (Arena's and Background's with the chosen one's picture), and Enter, a click or a
tap opens its list (Arena's and Background's the grid of pictures; the shell's, by the row; open at the value
chosen, a long one scrolling; on a phone the sheet); a choice from it changes the row as a step to it would, at
once. Left and Right still step them. The list is the battle's owner's, as the menu is (`Shell::pick`: a menu's
list over a battle is heard by the battle's owner, Practice, whatever screen started the session).

The groups are DUMMY, STAGE and OPTIONS (ダミー, ステージ, オプション; docs/design/app-ui.md §2.56: "having an
"arena" field under "arena" is so weird"): no group is named for one of its rows and none has one row; the input
display, a group of its own before, is the last of OPTIONS.

The Options rows but the input display are the library's training options (`Player::set_training`, set at once on the battle behind
the menu): host-side options a practice session sets between ticks, applied to the battle's own typed state
(a navi's HP, the custom gauge, the battle folder the custom screen deals from) before each tick, never a rule
of the simulation. With them at their normal values the battle is
the set's as it was before they existed, tick for tick by digest (the library's test of it, docs/frontend.md
§2), and every online match and replay is untouched: only live play has them. They act on the battle as it
is held behind the menu, and change nothing of the hold: a refill shows there (the HP rolls back up under the
menu), the gauge's value is full there and the engine flags it full once the fight runs again, the folder is
put in order there.

A row is focused as a whole and changed at once by Left and Right (Enter steps it on, or opens the list of
a dropdown's), around from its last value to its first. Under them RESUME and RESTART side by side, and under
those EXIT, as wide as the two (on a phone, the three one over the other): back to Play (`menu.exit`;
`practice::MENU_BUTTONS`). The user (docs/design/app-ui.md §2.40): "you should also get rid of "quit to menu"
and "change builds" should just be "exit"": CHANGE BUILDS became EXIT, and QUIT TO MENU went; since §2.57 there
is no entry, and EXIT goes to Play. In Japanese EXIT is 終了, as a fighting game's training ends
("トレーニング終了"). Esc and the pad's Start and back button close the menu as RESUME does, as before; R, F5 and the pad's north
button restart. A session another screen started (Play's, beside a waiting invite) has that screen's EXIT, if
it gives one, worded its way: CANCEL INVITE (hosting directly, STOP HOSTING), and on a phone STOP PRACTICE as
the one row at the top. The
menu opens on RESUME every time (the user: "when opening the pause menu in training, focus should always default
to the resume button"). The dummy's setup, the battle's options and the input display are
the visit's (`practice::Visit`), as the place and the dummy's build are: each visit starts on their defaults
(Stands still, Normal, every option normal, the input display off, the dummy on the player's own build; the
user: "yes for the practice options, they should all reset to default", "you should also not remember the last
dummy build either"), and the player's build is Play's chosen one; a session another screen started begins as
that screen says, the dummy on the player's own build, every battle option at its normal value and the place
Random, and keeps what its menu sets for that screen's wait alone (§4, "Waiting"), not for a visit.

**The dummy's rows.** One row says what the dummy does, its Action, and its Level comes under it while it is
Fights (`shown_rows`; `lay_out_rows` puts the row in or takes it out as the Action changes, the rows above it,
the Action's own and the focus with it, staying where they are). Its custom screen goes with its Action
(`dummy`): the AI's for one that fights, a second pad's own, else the first chip it can. The user (2026-10-13):
"the dummy menu is so confusing and has so many redundant options and it's confusing to navigate. you should
make it clearer and less redundant": before, Level, Custom screen and Recording were rows of their own whatever
the Action, and Attacks at random was a weaker Fights; they went. Then, of the dummy's recording (the player
drove the dummy, and it played their buttons back): "get rid of recording support, it's so complicated"; it
went, from the menu and from the library.

**A second pad.** With the dummy's Action "A second pad", the second gamepad used (the pads are known in the
order each was first pressed, `input::Pad`) is the dummy's: what it holds is the dummy's buttons, in the fight
and on its custom screen (the library's `LivePlayer::set_second_pad`, each frame before the ticks; its START
reaches the battle on the custom screen alone, the menu being the player's), and its
presses are neither the player's nor the menus' (`Pad::apart`). With one pad or none the dummy stands still and
its custom screen picks the first chip it can. Any other Action gives the pads back to the player.

**The dummy that fights** (Action Fights). Its AI is nettai-training's `Fighter` (the crate's docs: how well
each level plays, measured), in the fight and on its custom screen, where Normal and Hard pick the chips worth
most first (each tried in a copy of the fight; Easy, every chip it can in the cursor's order); it is
plugged into the session's live play (`LivePlayer::set_ai`) while the Action is Fights, at the Level chosen, and
taken out otherwise (`practice::plug`); another Level plugs in a new one. It presses buttons as a player does: each few
ticks it tries short plans in copies of the battle, on threads of its own (three at most, fewer on a machine
with fewer cores to spare), and acts on its decision a reaction time later (Easy's half a second, Normal's about a
quarter, Hard's a tenth); a decision not ready by then is waited for (measured at 60 ticks a second: none was
late). On the web it thinks on the battle's own thread.

*Adding a row*: a value of `PracticeOption` and its words in practice.slint (`label`, `value`; a row whose
values are the content's names has them as its text, as Arena's), its place in `ROWS`, and what it does in
`values`, `value` and `Practice::change_option`. A group's title is a row too.

**The input display** (`PracticeInputs`) is the app's own: two rows of ten buttons (the directions, A, B, L, R,
SELECT, START), the player's over the dummy's in their sides' colors, each lit while it is held. It reads what
each side's custom screen reads (`Battle::custom.sides[n].joypad.held`: this tick's buttons, not the fight's
delayed ones) after the frame's ticks, and tells the UI only when they change. It is in the picture's top right
corner, under the eighth the battle's own text takes there; on a phone, under MENU. It is drawn into a layer
and drawn anew only when a button changes (§5: drawn each frame it cost 0.3 ms a frame). Under the menu it shows
nothing held: the battle takes no button then, and the dummy's presses go nowhere.

**On a phone** the menu's rows have their label over their arrows and scroll (a `ScrollPage`, `menu.rows`, which
on a desktop holds them all), EDIT is the panel's width, the buttons are one over the other, and in battle the
line beside MENU says "Practice · Round 1" (`Shell.battle-note`, which a battle's owner sets).

**What isn't there.**

- A real key press: the screenshot pass drives the screens through the same calls a key becomes (`App::act`)
  and holds the battle's buttons directly; the window's own key events were not sent to it.

## 16. Replays

docs/design/app-ui.md §6, "Replays". The screen is `ui/screens/replays.slint` and `src/screens/replays.rs` (with
`replays/viewer.rs` and `replays/export.rs`); the folder's reading is `src/replays.rs`. The library under all of
it is docs/frontend.md §8: the file, the viewer and its seeking, the video. A replay keeps each round's
background by its game and its name (any game's: §1, "The battle's background"), and a viewer draws it as the
match did, or its stage's own where the player hasn't that game's pack. The file is of layout 1
(`nettai_replay::LAYOUT`); one of another layout is refused as another version's.

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
(`nettai_replay::Head::read`: a damaged file), and with every game's
only where even that doesn't.

**The list** is the selected game's replays, newest first: the date, the opponent (the other player's name;
`P1` or `P2` where the replay has none), the score as the recorder's (`W 2–1`, `W 10–9` in a set first to ten,
which the score's column holds; the rounds alone where the set wasn't finished) and the length, each in a column of its own width. The search is by opponent. The filter
between All and Online is there only while the game has replays of both kinds. Enter on a row watches it (a click chooses it
and no more: WATCH starts it), E exports it, Delete asks once (Cancel focused) and removes the file. On a phone a tap on a row opens the
selected replay as a sheet from the bottom, with the four actions one under the other. No build's name is
anywhere: a replay has none.

**A set of any length.** A replay keeps the match it was played from, and with it the round wins its set was first
to (the match in nettai-match's binary: a byte, then every round's place: docs/frontend.md §8); its rounds are as
many as were played, one to nineteen, each with its mark. The score, the bar's marks, a round's part to export and
the viewer's round steps go by the rounds the file marks, whatever their number.

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
- *What fails, and what shows under it.* One that fails as it is opened is the error's dialog over the list
  (`Shell::fail`): its file doesn't read now (it changed since the list read it), the library refuses it, the
  engine stops making its first battle (`watch` and the cards' last step each start its set inside the library's
  `in_engine`: a failure, not a crash), or the cards' making ready stops (`UserError::Other`). One whose engine
  stops as it plays (as it plays, at a frame stepped to, or where a seek or a drag took it) ends there: the
  viewer is left for the list and "Something went wrong" is over it, the detail (the tick, and the panic's words)
  in the log as `watching <file>`, as Practice's battle that stops is (§15). Under each, and beside a replay
  that can't be played at all (a row: above), the replay's box holds its own still or nothing: never the menu's
  reel nor another replay's still, which until the user saw it was what the box showed, spilling past it
  (§1). The screenshot pass shoots two (§13: `replays-unreadable`, `replays-watch-failed`); the engine's stop
  as it plays was seen once, through a panic forced at a tick of the second round in a build made for it and
  not kept: the viewer went back to the list with the dialog over it and the replay's own still in its box.
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
- *What else may be over the picture* (the user: "add support for input view and picture in picture in replay
  viewer"), each off at first, toggled by its transport button after BUILDS (lit while it is on) or a key, and
  kept from one replay to the next while the app runs (not saved: the viewer saves no setting of its own):
  - *The input display* (I): Practice's (§15; one component, `ui/inputs.slint`'s `InputDisplay`, which Practice
    places too), in the picture's top right corner as Practice's is, the console shown's player's buttons in
    red over the other's in blue (the colors of their sides of the picture). The buttons are the battle's own
    (`custom.sides[side].joypad.held`, as Practice reads them), so they are the tick shown's wherever the replay
    is: playing, paused, stepped, sought or dragged.
  - *The other console's picture* (P: a picture in the picture), in the shown one's bottom right corner, four of
    the game's pixels in, a line around it: a whole multiple of the frame about a third of the picture's width
    (the picture's scale over three, rounded, at least one: twice the frame where the picture is six times it),
    on the display's pixels. It is presented from the same tick as the picture (the library's
    `Viewer::present_view`: the replay is played once, the other console drawn from the same battle with its
    own HUD state), whenever the picture is, at the window's density as the picture is (`stage::fit`: in the
    original mode the frame itself, 115 KB), into a second texture of its own (`Shell::inset_gl`, written in
    place as the picture's is; `Shell::show_inset`): the window's one picture and its `Showing` are untouched.
    So it keeps step with every seek, step, drag and pause, and costs a second frame drawn per tick shown.
  Neither has a footer hint: with them the pseudo-locale's hints no longer left the status its room.
- *On a phone* the picture has the window's whole width, as the battle's has (§9): a window 780 of the display's
  pixels wide shows three times the frame, where the page's gutters left room for two. The transport is under
  it, with its eight buttons across (each 40 wide, as on a desktop), then Speed and View as steppers and EXPORT
  VIDEO. One component is both shapes
  (the other shape's parts hidden and passed by), since two under an `if` each cost more (§14).

**The export** (`Export`) is a dialog of one size from its start to its end: Size (2, 4 and 6 times the frame,
4 chosen), View, Sides (One, or Both: the user, "add support for two sided side by side export"), Part (the
whole match, or a round), Save as, the bar and its line, CANCEL and EXPORT. With Both, the video is both
consoles side by side, the View's at the left and the other's at the right, twice as wide (the Size says the
video's pixels: 1920 × 640 at four times), from one playing of the replay (nettai-video's `Sides::Both`,
docs/frontend.md §8), and the file's name ends "both sides". Its sound is two tracks, not a mix (the user:
"for 2 sided replays, audio track 1 should be left player audio, audio track 2 should be right player
audio"): the first is what the left console played, the default a player starts with, the second what the
right one played, which a player's audio track menu switches to. Each track is titled with its player's name
as the replay has it (else P1 or P2, in the app's words), as the MP4 track's name and its handler's. A mix
would be the music and the shared effects twice over, and at the set's end the winner's music over the
loser's. With One the sound is the shown console's, one track titled the same way.

Every export has **chapters**, a round each (the user: "video exports should have chapter info for
rounds"): "Round 1", "Round 2"… in the app's language (`ReplaysView.chapter`, "ラウンド 1" in Japanese), each
from the frame its round's picture begins on (the tick that ends a round shows the next one at its start) to
the next one's; a round's part has its round's chapter alone. They reach the MP4 the way the tracks' titles
do, in the NUT stream's info packets, which ffmpeg keeps (nettai-video's `chapters` and `nut::Info`,
docs/frontend.md §8): the MP4 has them both ways players read chapters, as a chapter text track (QuickTime's)
and as Nero's `chpl` box. The bundled ffmpeg needed nothing added for either. EXPORT
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
- *ffmpeg* is `$NETTAI_FFMPEG`, else the one beside the program, which every package bundles (§19), else one on
  the path or where a package manager puts it (`/opt/homebrew/bin`, `/usr/local/bin`, `/opt/local/bin`: an app
  started from the desktop has a short path). The path's relative folders (`.`, an empty entry) are passed by, so
  an `ffmpeg` in the working directory is never run (nettai-video's `mp4::find_ffmpeg`, docs/frontend.md §8).

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

The input display and the other console's picture were shot by the pass (`viewer-overlays`, on together over
the other console, I by its key and P by its button; English at a desktop's size, Japanese and the pseudo-locale at both)
and looked at: the inset is the other side's console at twice the frame in a picture six times it. The dialog
with Both is `viewer-export-both`. Not run: either while a replay plays in a window (the pass holds the viewer
still), so neither's cost per frame was measured. The two-sided video ran once through `nettai-tool --both-sides`
on one of the user's EXE4 replays (copied out; 7,237 ticks): 1,800 frames as an MP4 at twice the frame,
960 × 320, H.264 and AAC by Homebrew's ffmpeg, a frame of it each side's own custom screen; and 120 frames as
NUT streams, both sides and each alone, decoded by ffmpeg: each half byte for byte that side's own video, the
halves different in every frame, the sound the left side's sample for sample (before it was a track a side).

The tracks and the chapters ran through `nettai-tool --mp4` with the bundled ffmpeg (ffmpeg-9.0.2-1's
macos-arm64 binary, by `NETTAI_FFMPEG`) on EXE6's packs, a two-round set of 962 ticks recorded headless:
two-sided with the right side at the left, ffprobe read H.264, then AAC "Chaud" (the default), then AAC
"Lan", then the chapter text track, and the chapters "Round 1" (0 to 7.986 s) and "Round 2" (7.986 to
16.106 s: frame 477, the second round's first picture); one-sided, one AAC track "Lan" and the same
chapters. Decoded, the two-sided file's first track was sample for sample the right side's own one-sided
export's and its second the left side's, the two different (the difference's RMS 0.18, over most samples).
Not tried: a player's menus themselves (QuickTime's, VLC's, mpv's).

## 17. The packs, from the Legacy Collection

The user: "you should detect bnlc vol2 and extract all assets from there on startup if they are not extracted yet
(and if nettai version changes, reextract assets). there is bnlc detection code in tango you can steal".

**Where the packs are** (`packs.rs`). In the data folder's `packs/`, a folder a game (`packs/exe6`), which the app
makes itself. `$NETTAI_PACKS` wins: when it is set the packs are that folder's, by the game each says, and nothing
is looked for or extracted (development, and the screenshot pass, which never extracts: it keeps nothing of its
own in the player's data folder).

**Finding the collection** (`bnlc.rs`, ported from Tango's `tango-library/src/bnlc.rs`, its `locate_app_dir`,
`rom_archives` and `scan_rom_archive`, Vol. 2 alone, with the crates it uses: `steamlocate` and `zip`). The
folder the player chose is looked in first (the settings' `legacy_collection`: the install's own folder, its
`exe` or its `exe/data`, whichever was chosen), then Steam, for the app id of Vol. 2, 1798020. `steamlocate`
finds Steam on Windows by the registry's `InstallPath` (`HKLM\SOFTWARE\Wow6432Node\Valve\Steam`, else
`SOFTWARE\Valve\Steam`), on macOS at `~/Library/Application Support/Steam`, and on Linux in each of Flatpak's,
the usual (`$XDG_DATA_HOME/Steam`, `~/.steam/steam`, `~/.steam/root`, `~/.steam/debian-installation`) and Snap's
folders that has a `steamapps` folder; then each library Steam lists (`steamapps/libraryfolders.vdf`) for the
app's manifest, whose folder is `<library>/steamapps/common/<installdir>`. (Tango's lock has steamlocate 2.0.1,
which on Linux takes the first of those that exists; this is 2.1.1, and every Steam found is looked in.) The
collection is a Windows game: Linux's Steam installs it with Proton in its own libraries, where it is found;
macOS's doesn't install it, and a Windows Steam in a compatibility layer (CrossOver, Whisky) is a folder to
choose. On this Mac Steam has no collection, so only the chosen folder has run.

The ROMs are the `.srl` entries of the per-game archives `exe/data/exe*.dat` (zip files; the shared `exe.dat`
holds the collection's own art and is passed over), wherever in an archive they are. Each is read only as far as
its header, and the extractor says whose it is (`nettai_extract::recognize`: the app names no game); a second
copy of one game code is passed over. Steam isn't asked when no pack is wanted.

**When a pack is extracted.** When the game is first loaded (the user: "you should lazily extract games on first
load"; before, every game wanted was extracted as the app started), for each of the content's games whose pack is
missing, or was made by another build: each pack is stamped (`stamp.toml` in it) with what made it,

    nettai = "0.1.0"
    assets = "1f3eed2364479361"

the app's version, and the extractor's fingerprint of the game's asset names (`nettai_extract::fingerprint`: a
digest of the versions of the pack's formats, `nettai_content::pack::FORMATS`, each 1, and of the names the
pack's assets are written under, from the content's compat tables: `assets.toml`, a chip's icon by its id, a
navi's emblem by its key, the fonts' glyphs). A pack whose stamp isn't this build's, or that has none (one
`nettai-extract` wrote), is extracted again. The version is
the user's word; it doesn't change between builds while it stays 0.1.0, and two builds of it differ in their
packs above all where the content names an asset the other's pack hasn't (a sprite added, a song named): the pack
loads and the content can't find it, which is what `UserError::GameOutdated` told before. The fingerprint is
that. A change to how an asset is decoded that names nothing new doesn't change it: that is the version's to say.
(A digest of the extractor's own source would catch that too, and would extract every pack again at nearly every
commit.)

The battle themes (§1, "The battle's music") were such a change: every pack held every song already, the
unnamed ones under their numbers (`sound-016`), which content may not use; naming EXE4's 0x16 and 0x17 and EXE5's
0x16 and 0x17, and EXE4's 0x14 and 0x15 by their use (`virus-battle`, `boss-battle`, were `netbattle-single` and
`netbattle`), changed BN4's and BN5's fingerprints, so the app extracts those two again (BN6's names were all
there). A packs folder of `nettai-extract`'s (`$NETTAI_PACKS`) is the developer's to write again:
`nettai-extract exe4 <dir> <exe4 ROMs>` and `nettai-extract exe5 <dir> <exe5 ROMs>`, into a new folder, then
put in place.

**How.** On one thread, the games one after the other, the one the app opens on first; stepping to one still
waiting moves it to the front. For each, its ROMs are read out of their archives into memory (none is written to
disk) and handed to the extractor's library as bytes (`RomSet::insert`, `extract_reporting`), which writes the
pack into a new folder, `packs/.new/<game>` (`Extraction::write_reporting`, which reads it back), and the stamp is
written beside it. Then it is put in place by a rename: the old pack is moved aside to `packs/.old/<game>` the
moment before, and deleted after. A crash leaves the old pack or none, never half of one; what it leaves is
tidied as the app starts (a pack moved aside and not replaced is put back, the rest goes). The extractor's rule
stands: its inputs are ROMs alone, and a pack is whole from whichever of the game's ROMs hold each asset. The pack's
files are the extractor's, as `nettai-extract` writes them from the same ROMs (checked: below); the stamp is the
one file more. Each game's end goes to the log, with how long each stretch took.

**What the player sees.** The loading screen's bar is the extraction's first, "EXTRACTING BN6" and its
percentage, then the load's, "LOADING BN6". As the app starts, the collection is found and each game it has the
ROMs of can be stepped to (`games::State::Extractable`), but only the game opened on is extracted: the one last
played, or the first time BN6 ("you should default to bn6 as the first game though"; `Games::first_choice`). The
others are extracted when the player first steps to one, its bar the extraction's (the user: "it doesn't correctly
show that it's extracting each game" of extracting them all at once, behind a bar that showed only the game
shown). Stepping to a second while one is extracted waits for the first to land.

**When there is nothing to extract from.** A game with no pack, and no collection found, is
`UserError::NoLegacyCollection`: "nettai needs Battle Network Legacy Collection Vol. 2. Install it from Steam, or
choose its folder." (in Japanese by the collection's Japanese name, ロックマンエグゼ アドバンスドコレクション
Vol.2). Where some game can be played, it is a dialog, once, with CHOOSE FOLDER and OK; where none can, the loading
screen shows the line and CHOOSE FOLDER. Either opens the system's folder dialog: a folder that holds the
collection is kept in the settings, and the games it has can be extracted from it, each as it is first loaded; one that doesn't says the error
again, and nothing is kept. A game whose pack another build made is loaded as it is while there is nothing to
make it again from; if it doesn't load, that is the same error. A failure while reading the archives or
extracting is `UserError::ExtractFailed` ("BN6's data couldn't be extracted. Details are in the log file."), the
old pack kept where there is one; what went wrong is in the log alone.

**How long it takes** (2026-10-13, a release build, each game's four ROMs, from the archives of the folder below,
as the app's log said them; load average 17 to 35; the game the app opened on first):

| Game | Its ROMs read out of the archives | Extracted | Its pack written (encoded, written, read back) | In all |
|---|---|---|---|---|
| BN6 | 0.18 s | 0.04 s | 1.55 s | 1.77 s |
| BN4 | 0.12 s | 0.02 s | 1.00 s | 1.14 s |
| BN5 | 0.11 s | 0.04 s | 0.91 s | 1.06 s |

So a first run waits about two seconds for the game it opens on, and the three are there in four. The writing's
own split is about 40, 30 and 30 of 100 (its files encoded, written, read back: the bar is weighted so). The
command, `nettai-extract` in a release build with the same ROMs from their files, takes 1.0 s for BN4 and BN5
and 1.1 s for BN6, at its peak 100 MB of memory (BN5's 210 MB).

**What has run.** End to end, on this Mac: a folder shaped like an install (`exe/data/exe4.dat`, `exe5.dat` and
`exe6.dat`, zip archives of each game's four ROMs at their root, as the collection names them, `exe4_rom_e.srl`
and the rest; an `exe.dat` beside them), chosen as the player would by the settings' `legacy_collection`, with the
data folder an empty one of its own. The app found the folder, extracted BN6 first (the settings' game), then BN4
and BN5, with no load failing in the log, and ended on a terminate request with status 0; each of the three
packs is byte for byte the one `nettai-extract` writes from the same ROMs' files (`diff -r`: the stamp is the one
file more). In tests (`bnlc.rs`, `packs.rs`, `games.rs`, each with archives and packs of its own in temporary
folders, the ROMs fake bytes where the code only moves them): an install found from
any of its three folders; the ROMs found by their headers wherever in an archive they are, other entries and a
second copy passed over, an archive that isn't one passed over, each ROM read back whole; a pack current by its
stamp, stale by another's or none; a pack replaced only once whole, a failed one leaving the old; what a crash
leaves tidied; a failed extraction leaving the old pack; the extraction's thread taking the games in turn, the one
asked for first, and starting again for more; a game playable as its pack lands, a failure told once and its old
pack played. The screenshot pass has the two screens (`extracting`, `no-legacy-collection`) and the dialog
(`no-legacy-collection-dialog`), set on the view (`NETTAI_TOUR_ONLY=packs`): walked in English and Japanese at
the desktop's size, nothing cut and the pictures looked at; not yet at a phone's, nor in the pseudo-locale. **What waits on a real install:** Steam finding it (on Windows,
and on Linux under Proton); the real archives' layout and compression (Tango reads them as zip archives with
`.srl` entries wherever they are, as this does; the app reads deflate and stored entries, and another method is an
`ExtractFailed` with the method in the log); the real folder's name, for the chosen folder (any of its three
folders is taken).

## 18. About

The user: "add an about screen with license and dependency info etc like for slint".

**Where it is.** About is Settings' subscreen (docs/design/app-ui.md §2.53; it was the main menu's small item
under the line until the user: "the about screen should be moved as a subscreen of the settings dialog and
removed from the main menu"). Settings' About group, the last in its list, has a row, Made with, whose SHOW
LICENSES (`set.about`) shows About in the page's place: `SettingsScreen` is the page or About's
(`SettingsView.about`), as Replays is its list or its viewer, and the phone's sheet is Settings' overlay. To the
focus it is a view of its own (`settings.about`, `Nav::enter`). Settings' module passes it what it hears while it
is open (`screens/settings/about.rs`: `About::open`, `close`, and a screen's `show`, `activate`, `focused`,
`scroll` and `back`). Its title is ABOUT and its Back ‹ SETTINGS.

**What it shows.** At its left: the app's name, its version and its license (the workspace's `license = "MPL-2.0"`, the Mozilla Public License 2.0: LICENSE at the
repository's root); Slint's own `AboutSlint` widget, the "Made with Slint" logo with Slint's version and address;
and under MADE WITH, a list of the software the app is made of: the four fonts it bundles, then every package built into it, by name, each row its name, its version and its
license as it states it (an SPDX expression). Beside the list, in a panel, the chosen one's name, version, license
and repository, and its license files as its package ships them, each file's name over its text in the monospaced
face, scrolling. The footer counts them ("394 packages · 4 fonts" today). On a phone the list is the page, and a
tap on a row opens its license as a sheet from the bottom, with CLOSE.

**Why Slint's widget.** Slint is used under its royalty-free license (`LicenseRef-Slint-Royalty-free-2.0`, one
of the three its packages offer), whose attribution clause asks an app to show `AboutSlint` "in an 'About' screen
or dialog that is accessible from the top level menu of the Application". This is that screen. (The same clause
asks for the "Made with Slint" badge on a public webpage, preferably where the app is downloaded: that is the
project's page's to do, not the app's.) The widget is Slint's, drawn in its own type; its logo is the one for a
dark scheme (`Palette.color-scheme`, set by the screen: the app uses no other standard widget).

**Where the list comes from** (`crates/nettai/build.rs`). At build time, the packages `cargo tree -p nettai`
lists for the target being built, with the features a build of the app alone gives them (`-e normal`: not the
tools that build or test it; `cargo metadata` would unify the whole workspace's features and list packages the app
never builds), and the sources of a library a build script compiles into it (a `*-src` package: SDL's, Luau's).
`cargo metadata` says where each one's files are; its license files are those named LICENSE, LICENCE, COPYING,
COPYRIGHT, NOTICE or UNLICENSE (in its folder or a folder of its own, and in that folder's `deps`: the libraries a
vendored library vendors, as libdatachannel's ICE, SCTP and logging in libdatachannel-sys), everything in a
`LICENSES` folder, and the file its manifest names. The workspace's own crates are not listed. The fonts and their
OFL files are named in the build script. What it writes (`licenses.rs`, in the build's output) holds each text
once, as many packages ship the same one: 220 texts for 328 entries on macOS (2026-10-14, once the `rtc` crates
had given way to libdatachannel), about 0.8 MB in the binary. It is written again when Cargo.lock, the app's
manifest or a font's license changes. The same list is written as a text file beside it (`licenses.txt`),
which a package ships as THIRD-PARTY-NOTICES.txt (§19).

24 of the packages ship no license file in what they publish (the objc2 family, the lyon ones, libc, taffy,
gamepad-facade and some others): their row and panel name their license, and the panel says the package comes
with none.

**The keys.** Up and Down go through the list, and the license beside it follows the focus; Enter (A) goes to the
license, where Up and Down scroll it a step, Page Up and Page Down a page, Home and End to its ends
(`About::scroll`, as a build's page in the match's menu scrolls), and Back (or Left) returns to the list; Back
from the list goes back to the settings, the focus on SHOW LICENSES. A click on a row chooses it and leaves the
focus there; the wheel or a drag scrolls the license wherever the focus is. L and R do nothing here: the header
offers no game. Opened again, About has the focus on the row it was on, the list scrolled to it (a list made anew
starts at its top, and a remembered focus alone doesn't scroll it). Settings left while it is open (the shell
going elsewhere) closes it.

**What has run.** Its tests (`screens/settings/about.rs`): the fonts are listed with their OFL texts, Slint is
listed with its royalty-free license among its files, no workspace crate is and nothing only the workspace's
other members build is (skia); a license's pieces are its files' names over their paragraphs. The screenshot pass
has `settings-about` (the About group, SHOW LICENSES focused), `about` (opened by it: the list, the first font's
license beside it), `about-license` (Slint's license reached with the keys and paged down; on a phone its sheet),
`about-back` (Back to the settings, on SHOW LICENSES) and `about-again` (opened again: Slint's row focused and in
view). Before About moved into Settings they were walked in English and Japanese at both sizes, nothing cut, and
the pictures looked at; since, the steps are type-checked and not yet walked.

## 19. Packaging

The user: "start working on github actions workflows to make an installer for windows and i dunno dmg for macos
and appimage for linux".

A package is the program, the content (`content/`, as the repository has it), the license (LICENSE), the
third-party notices (THIRD-PARTY-NOTICES.txt, build.rs's `licenses.txt`, §18), and `ffmpeg` beside the program
with its notice (ffmpeg-NOTICE.txt; below). No ROM and no extracted asset is in
any of them, ever: the app extracts each game's pack from the player's own Legacy Collection into its data folder
(§17). Each platform's script is in `packaging/`, runs on that platform alone, and writes to
`target/package/<platform>/`:

- **Windows** (`packaging/windows/package.ps1`, `nettai.iss`): an Inno Setup installer,
  `nettai-<version>-windows-x64-setup.exe`, for the player alone by default (`%LOCALAPPDATA%\Programs\nettai`, no
  administrator; it offers everyone), with a Start menu shortcut, a desktop one if asked and an uninstaller that
  leaves the data folder; and the same app as a .zip. Needs Inno Setup 6.
- **macOS** (`packaging/macos/package.sh`, `Info.plist`): `nettai.app` (bundle id `io.github.bigfarts.nettai`;
  the content in `Contents/Resources/`, where the content lookup finds it) in `nettai-<version>-macos-<arch>.dmg`,
  beside a link to /Applications. `NETTAI_UNIVERSAL=1` builds arm64 and x86_64 and joins them with lipo; else the
  host's alone. Signed ad hoc unless `MACOS_SIGN_IDENTITY` names a Developer ID; with `APPLE_ID`, `APPLE_TEAM_ID`
  and `APPLE_APP_PASSWORD` too, the image is notarized.
- **Linux** (`packaging/linux/package.sh`, `AppRun`, `nettai.desktop`): `nettai-<version>-linux-x86_64.AppImage`,
  made by appimagetool with the type 2 runtime, both downloaded at pinned versions and checked against their
  digests. The program needs the glibc it was built against or a later one, so it is built on an old distribution.

**The bundled ffmpeg.** The user: "i think you also need like the equivalent of
https://github.com/tangobattle/ffmpeg-build for encoding and bundle ffmpeg with it. it should support whatever
nettai needs and nothing more so the build is minimal", then "make sure to bundle it with the binary", then "i want
you to build it with a separate workflow in https://github.com/nettaisen/ffmpeg-build so you don't have to
build ffmpeg everty time". That repository's workflow, run by hand, builds a static `ffmpeg` per platform from
`--disable-everything` with only what the export's command runs (the NUT demuxer, the raw video and float PCM
decoders, the pipe and file protocols, swscale's `scale` and `format`, `setparams`, swresample's `aresample` with
`aformat`, `null` and `anull`, libx264 and the native AAC encoder, the MP4 muxer: its README traces each to the
command), smoke-tests it with that command on streams nettai-video's own NUT writer makes, decodes the MP4s with a
full ffmpeg, and publishes a release `ffmpeg-<version>-<revision>`. `packaging/ffmpeg.sha256` pins the release a
package takes by its tag and each file by its SHA-256; the scripts download them once into
`target/package/ffmpeg/<tag>/` and check them (or bundle the ffmpeg `NETTAI_BUNDLE_FFMPEG`, `-Ffmpeg` on Windows,
names). Where it goes: `ffmpeg.exe` beside `nettai.exe` (the installer and the .zip), `Contents/MacOS/ffmpeg` in
the app (the release's arm64 and x86_64 programs joined by lipo for the universal app, the host's alone otherwise;
signed before the app, as the app is), `usr/lib/nettai/ffmpeg` in the AppImage (beside the program, which is there
too). The app looks beside itself first (§16).

The binary is GPL: FFmpeg built with `--enable-gpl` and linked with x264 (GPL-2.0-or-later) is licensed
GPL-2.0-or-later as a whole. Each package ships `ffmpeg-NOTICE.txt` from the release (beside THIRD-PARTY-NOTICES.txt:
beside the program on Windows, in `Contents/Resources` on macOS, in `usr/share/doc/nettai` in the AppImage): what
the program is built from, its license, the license texts, and where its complete source is, which is the release
itself: the FFmpeg and x264 source tarballs it was built from and the build scripts at the commit that built it.

The icons are made from `docs/logo/nettai-icon.svg` by `packaging/icons.sh` (rsvg-convert and ImageMagick):
PNGs, the Windows .ico, and macOS's icon set, the artwork on Apple's grid (824 of 1024, centered) so that it is no
larger than other apps in the Dock; nothing raster is committed. The Windows program takes its icon from the .ico
`$NETTAI_WINDOWS_ICON` names (build.rs, through winresource); without it, it has none. The window's icon is the
SVG itself (`app.slint`).

A package's build sets `NETTAI_PACKAGED`: its content lookup then has no fallback to the repository's `content/`
(`nettai_content::index::content`), so that the program names no folder of the machine it was built on and reads
only the content it ships. The version is `$NETTAI_VERSION` (`-Version` on Windows), else the app crate's.

Besides Rust, a build needs CMake (SDL's, and libdatachannel's: nettai-rtc's WebRTC, C++ with Mbed TLS, its
sources submodules of the facade's repository, which Cargo fetches with it) and libclang (bindgen, for
libdatachannel's C API): Xcode's on macOS, the runner's LLVM on Windows, `libclang-dev` on Linux (the workflow
installs it).

The workflow (`.github/workflows/release.yml`) runs on a tag `v*` and by hand. The icons are made once, on Linux;
then Windows (`windows-latest`), macOS (`macos-latest`, universal) and Linux (`ubuntu-22.04`) each package and
upload theirs as the run's artifacts, the version the tag's without its `v`. On a tag, a draft release is made with
all of them, never published by the workflow. Signing is optional and skipped without its secrets:
`WINDOWS_CERTIFICATE` (a .pfx, base64) and `WINDOWS_CERTIFICATE_PASSWORD`; `MACOS_CERTIFICATE` (a Developer ID
Application .p12, base64) and `MACOS_CERTIFICATE_PASSWORD`, and for notarizing `APPLE_ID`, `APPLE_TEAM_ID` and
`APPLE_APP_PASSWORD`.
