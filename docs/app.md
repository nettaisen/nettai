# The app (`nettai`)

**Status (2026-10-06): a prototype.** The user asked for a game UI on desktop, the web and mobile, with no
webviews, and chose Slint ("it should be very polished though"). `crates/nettai` is that app. It runs on the
desktop (built and measured on macOS). §6 records what the web build needs and §7 what mobile would take.

nettai is the player's host of nettai-frontend (docs/frontend.md: the battle it plays and draws); nettai-tools is
the command line with no window, for frames, audits and checks (docs/tools.md). nettai-demo, the first desktop
host, is retired (§10). nettai wraps the battle in the screens a player sees:

- a title screen, with a battle playing itself on its monitor;
- Play: the game, a random match or a match file, your build, and a preview of who fights in which arenas,
  against the stand-in;
- the battle;
- the builds: a player's sides, game by game, made in the build creator (§8);
- the netplay lobby: rooms through nettai-rtc, or a direct link;
- the replays: every set played is recorded;
- settings, kept between runs (§9).

Every screen is navigated alike by the keyboard, a gamepad, the mouse and touch. The app's own strings are in
catalogs, one for each language; the content's names come from its locales.

    NETTAI_PACKS=<packs> cargo run --release -p nettai

The packs are found as nettai-tool finds them (`$NETTAI_PACKS`, else `data`, each pack by the game it says;
written by `nettai-extract <exe5|exe6> <pack-dir> [ROM ...]`). What the player makes is kept in
the app's data folder: `$NETTAI_DATA`, else the system's (`~/Library/Application Support/nettai` on macOS,
`$XDG_DATA_HOME/nettai` or `~/.local/share/nettai` on Linux, `%APPDATA%\nettai` on Windows). There, builds are
in `builds/<game>/` (`$NETTAI_BUILDS`), sets are recorded to `replays/` (`$NETTAI_REPLAYS`), match files are
listed from `matches/` (`$NETTAI_MATCHES`), and the settings are `settings.toml` (`$NETTAI_SETTINGS`). For
netplay, `$NETTAI_SIGNAL` names the signaling server (§4). Some variables are for development:

| Variable | What it does |
|---|---|
| `NETTAI_LANG=<code>` | The first language shown, instead of the settings' or the system's. |
| `NETTAI_PLAY=<game>` | Starts straight into a random set of the game. |
| `NETTAI_NETPLAY=<game>:<make\|join>:<CODE>`, `<game>:host:`, `<game>:direct:<HOST:PORT>` | Starts straight into that room (or direct link), ready. |
| `NETTAI_PLAY_STATS`, `NETTAI_KEY_PROBE` | The latency figures of §5. |
| `NETTAI_PHYSICAL_PIXELS` | Presents the picture at the display's density. |
| `NETTAI_NO_TEXTURE` | Shows the picture as a new image each time, not in the texture (§1). |
| `NETTAI_RENDERER=femtovg\|software` | Picks Slint's renderer. |
| `NETTAI_TOUR=<folder>` | Walks every screen in each language, at a desktop's size and at a phone's, and writes each to a PNG (`NETTAI_TOUR_LANGS=en,ja,pseudo` picks the languages; `NETTAI_TOUR_ONLY=builds` walks the builds, Play's build and the direct lobby alone, of `NETTAI_TOUR_GAME=<game>`). Nothing it changes is kept in the settings. |

## 1. The battle in a Slint window

**The clock is the window's rendering.** The rendering notifier's `BeforeRendering` runs the app's frame
(`App::frame`) before Slint draws the frame:

1. It polls the gamepad and steps the wipe between screens.
2. It runs the battle's ticks due, with the buttons held.
3. If a tick ran, or the space changed, it presents a new picture, which that same frame draws.
4. It queues the sound.

`AfterRendering` notes that the frame was drawn and asks for the next one, so the app runs at the display's rate:
120 frames a second on this Mac's display. This is nettai-demo's lesson ("the widget is the clock") carried over:
a frame shows the picture presented for it, not the previous frame's.

Slint on Apple's systems draws its frames from a display link with no window event before them. Its Skia
renderer on Metal never calls `BeforeRendering`, so the clock needs femtovg, on OpenGL here and WebGL on the web.

**The picture is one texture, written in place** (`gl.rs`), nettai-demo's other lesson. When femtovg's
`RenderingSetup` hands over its OpenGL, the app makes one texture there, the picture's size, and shows it as a
borrowed texture (`BorrowedOpenGLTextureBuilder`), set as the `picture` property once. Each new picture is
written into it with `glTexSubImage2D`, and nothing is allocated for it:

- The presented buffer goes as it is. Its 0x00RRGGBB pixels are B, G, R and an unused byte in memory, which
  OpenGL takes as BGRA, so only the unused byte is made opaque.
- The texture is made again only when the picture's size changes.

Without that OpenGL (the software renderer, the web, or `NETTAI_NO_TEXTURE`), each picture is a new
`SharedPixelBuffer` image. That was the first prototype's only way, and it was the lag the user saw (§5).

**Its size** is the frame's largest whole multiple that fits the space, counted in display pixels
(`stage::fit`). It is presented at the window's density (the scale over the display's factor, rounded up), or at
the display's density with `NETTAI_PHYSICAL_PIXELS`, and drawn with `image-rendering: pixelated` at exactly
`240 × scale` display pixels.

The density need not divide the scale. Each display pixel samples the presented pixel its center falls in, and
that pixel always belongs to the right frame pixel, so every frame pixel is the same square
(`stage::tests::every_frame_pixel_is_a_square_at_any_density`). Only the text layer, drawn at the density, is
scaled by a fraction.

At 1280x800 points on a Retina display, a 9x picture is presented at 5x: 1200x800 pixels, presented and
written into the texture in 2.4 to 2.9 ms. Before this, odd scales were presented at the display's density, which
cost 6.4 ms a picture and held the app to 60 frames a second.

**The keys** come from winit's window events through Slint's winit hook (`on_winit_window_event`), before Slint's
focus handling sees them. They are read by physical key (Z and X sit where they are whatever the layout) and
timestamped as they arrive. While the battle has the keys, they don't reach the menus. The map is nettai-demo's:

- the arrows move; Z is A, X is B, A is L, S is R; Enter is START, Backspace is SELECT;
- Esc pauses (Tab does nothing in the battle: in the menus it is the next item, Shift+Tab the one before);
- in a replay, Space pauses, `-` and `=` change the speed, and `.` steps a frame.

**The gamepad** is read through gilrs and polled each frame:

- the south button is A, the west (or east) button B, the shoulders L and R, and Start and Select are theirs;
- the D-pad and the left stick move;
- the Mode (Guide) button pauses.

In the menus the same presses navigate, and the key hints switch to the pad's buttons once it is used.

**The sound** is nettai-audio's `Output`. The menus' sounds are the loaded game's own custom screen sounds,
through its rules' sound roles (`custom_cursor`, `custom_pick`, `custom_back`, `refused`): the app names no sound
of a pack. A second `BattleAudio` plays them:

- While a battle runs, it is ticked with the battle's ticks and its samples are added to the battle's, sample
  for sample, so the device gets one stream at the battle's rate.
- With no battle running, it runs on its own clock at the same rate.

At first it ran on its own clock throughout, and its samples went to the device beside the battle's, at twice the
rate the device plays. The queue then sat at `Output`'s cap of 200 ms: every sound was late by that much, and
dropped samples skipped (§5).

**Loading** a game (`game::load`) runs on a thread of its own. The app starts loading every game with a pack at
once, and the result comes back through Slint's event loop. Replays are played out on threads too, to show who
won and whether each plays back as it was played.

## 2. The screens

The visual direction is "cyber arena", built from Slint's shapes alone (no standard widgets):

- **Backdrop:** night blues behind the net's grid, which drifts.
- **Shapes:** slanted panels and bars (`Slant`, a path).
- **Color:** cyan for player one and focus, magenta for player two and the versus, gold for the main action and
  wins.
- **Type:** Murecho throughout, with its black weight for display.
- **Motion:** focus bars slide out, and screens change behind a diagonal wipe in the players' colors.

Each screen keeps its own cursor and takes navigation actions (`NavAction`: up, down, left, right, confirm, back,
previous, next) from the window's keys, the gamepad, or a click or tap, which focuses and activates.

- **Text fields** are Slint's `TextInput`, so the desktop's IME composes into them (winit's IME events; Slint
  shows the preedit). Up, down and Esc still leave a field: the window's `FocusScope` captures them while one
  is being edited.
- **Responsive layout:** the window sets `Theme.compact` (small) and `Theme.portrait` (held upright) as its size
  changes. Layouts follow these imperatively-set values, never the window's own size, so they don't size the
  window.
- **On a phone held upright:** the battle draws an on-screen pad under the picture, Play puts FIGHT at the foot,
  and the lobby shows the two players in a line.

The screens:

- **Title.** The demo battle is a random set of a loaded game, with the stand-in's custom screen pressed on
  both sides and the left navi fighting.
- **Play.** No game is assumed: the player chooses one. YOUR BUILD puts one of their builds of it in place of
  their side (a build that can't play says why). The arenas are each round's field, played alone for 24 ticks
  and drawn (`arenas.rs`).
- **Builds** and the build creator: §8.
- **The battle.** The pause has resume, start over and quit (the language is Settings' alone). The result has the score, the replay
  file it was recorded to, rematch and menu. A replay has its transport, and netplay its connection line and
  "reconnecting (N s)".
- **The lobby.** §4.
- **Replays.** Newest first, each row with:
  - the date in local time;
  - the navis, by the content's names;
  - the round pips;
  - the length;
  - whether it reproduces: ✓, or the difference;
  - watch from either console.
- **Settings.** The language, the volume, the menu sounds, the battle's text (crisp, or the game's own),
  the picture's density and the player's name, kept between runs (§9).

## 3. Languages

**Every string of the app's own goes through Slint's `@tr`**, with a context where one word means two things
(`"heading" => "PLAY"`, `"menu" => "PLAY"`) and plurals where a count shows (`"{n} round" | "{n} rounds" % n`).

The few strings the Rust side shows are functions of a global in the same catalog (`Strings`: "Random", "Stand-in").
Numbers, times and dates are formatted by the catalog's patterns (`format.slint`): a translation orders and words
them its own way (`{1}/{2}/{0} {3}:{4}` in English, `{0}年{1}月{2}日 {3}:{4}` in Japanese). Words the library
reports (why a game can't load, why a replay doesn't play) are its own English, shown under a translated heading.

**The catalogs** are gettext files: `crates/nettai/lang/nettai.pot` (the template) and one
`lang/<language>/LC_MESSAGES/nettai.po` for each language. `build.rs` bundles every catalog it finds there into
the binary (`slint_build`'s bundled translations), so the web and mobile need no files beside it.

It also makes a **pseudo-locale**, `pseudo`, from the template: every string lengthened (each vowel doubled, the
whole bracketed) and its letters accented. A layout that can't take a longer text shows it, and so does a string
that doesn't go through the catalog. It is in the language list and in the tour.

Each catalog names its own language: its translation of `"language" => "English"`. Settings lists the languages
by those names. The first language is the system's (`sys-locale`, by its primary subtag), else English;
`$NETTAI_LANG` overrides it. Settings changes it while the app runs:

- the app's strings switch with `slint::select_bundled_translation`;
- the content's names switch through its locales (`Game::graphics(lang)`, kept once loaded): the navis and chips
  in the preview, the replays and the battle's banners;
- the battle's own drawing switches too (`Player::set_language`).

A language the content has no table for shows the content's own names. Stage and background names have no display
names in the content, so they are shown by their names in it.

**Adding a language** is adding its catalog. With gettext's tools and Slint's extractor
(`cargo install slint-tr-extractor`):

    cd crates/nettai
    slint-tr-extractor --no-default-translation-context --package-name nettai -o lang/nettai.pot ui/*.slint
    mkdir -p lang/fr/LC_MESSAGES
    msginit --no-translator -l fr -i lang/nettai.pot -o lang/fr/LC_MESSAGES/nettai.po   # a new language
    msgmerge -U lang/ja/LC_MESSAGES/nettai.po lang/nettai.pot                              # a language after the UI changes

Translate the `msgstr`s, including `msgctxt "language"` / `msgid "English"` (the language's own name), and
build. The extractor runs without a default context, as the build compiles (`DefaultTranslationContext::None`).
`msgfmt --check -o /dev/null` validates a catalog. If the content has the language too, add its table
(`content/<game>/locales/<language>.toml`).

**Typography.** Tracked capitals (labels, buttons) are spaced for Latin and nearly solid for scripts set without
spaces (`lang::tracking`: Chinese, Japanese, Korean). Murecho covers Latin and Japanese (kana and kanji). For a
script it lacks, Slint falls back to the system's fonts on the desktop. The web has no system fonts, and a phone's
may lack one, so such a language needs its font bundled: import a Noto face for the script in `theme.slint`
beside Murecho, and fontique falls back to it by coverage.

## 4. Netplay

The lobby meets the other player in a room of nettai-rtc's signaling server, by the room's code, or over a
direct link:

- **Making a room** enters a new one at once. Its code (six letters and digits, none that read as another) is
  shown in large type and copied to the clipboard on confirm.
- **Joining** enters the room as soon as its six letters are typed.
- **Direct**: with no address typed, this player hosts on UDP port 47474 (`Link::host`), and the screen shows
  the address to give the other player (this machine's on its network; click to copy). An address typed
  (`host:port`) is joined once it is confirmed (`Link::join`). No signaling server is needed.

In a room, the first in it hosts. The library's lobby and handshake run over the link (`netplay::Agreeing`, the
datagrams of `nettai_frontend::lobby`):

1. Each player proposes the game, a triple battle with its places left to the seed, and their side: the build
   they chose (§8), else a random side of the game.
2. Each player's name goes with their lobby state (protocol version 4): it is shown on the other's card and kept
   in the replay's names, never taken for who they are; its control characters are dropped and it is cut to 16
   characters (`lobby::player_name`).
3. Readiness is each player's toggle, and each sees the other's. Changing the game or the build clears both.
4. Once both are ready on the same settings, the match is agreed and the battle is a `NetPlayer` over the link
   (`netplay::Framed`: the link as the library's `Channel`, frames told from the lobby's datagrams by their first
   byte, as nettai-demo's connection did). It is recorded, with both names.

The battle shows the connection's figures (ping, present delay, rollbacks) and "reconnecting (N s)" while the link
is down (`NetStatus::reconnecting`). Netplay can't pause: the pause's panel says so and holds no buttons.

The signaling server isn't deployed. Run it locally:

    cd signaling && npx wrangler dev        # http://127.0.0.1:8787
    NETTAI_SIGNAL=ws://127.0.0.1:8787 cargo run --release -p nettai

Without `$NETTAI_SIGNAL` the lobby says there is none (a direct link still works). Two windows on one machine
meet with `NETTAI_NETPLAY=exe6:make:ROOM42` and `NETTAI_NETPLAY=exe6:join:ROOM42`, or directly with
`NETTAI_NETPLAY=exe6:host:` and `NETTAI_NETPLAY=exe6:direct:127.0.0.1:47474`.

A room's link finds its way through NATs with nettai-rtc's default STUN server (`stun:stun.l.google.com:19302`)
and checks the certificates its descriptions name; a direct link authenticates nobody, as plain UDP doesn't. Over
the Internet, the host forwards UDP port 47474 on its router to its machine, and the other player joins the
router's public address. What the library does when the link drops, and what the handshake checks, is
docs/frontend.md §2's.

What's missing: the present delay can't be set on the screen (the library's `Player::set_present_delay`), and no
TURN server can be given, so two players whose NATs need a relay can't meet (nettai-rtc's `Config` takes
one, an `IceServer` with its credentials, in its `ice_servers`; Cloudflare's TURN service hands out short-lived
credentials from its API).

## 5. Latency and smoothness

Measured as nettai-demo measured (docs/frontend.md §7, its lessons). `NETTAI_KEY_PROBE` presses the right arrow every 300 to
400 ms, through the event loop as a key's event arrives, at no particular point between frames.
`NETTAI_PLAY_STATS` prints, every two seconds:

- frames a second, and the frame time's median, 95th and 99th percentiles and worst ("long": over half again the
  median, a refresh missed);
- ticks a second (the original's 59.73);
- each picture's cost (presented and written), and how long after it the frame was drawn;
- for each press, the time to the tick that saw it and to that tick's picture drawn;
- the most sound queued for the device (the sound's latency).

All figures are from this Mac (120 Hz display), with a 1280x800-point window (a 9x picture presented at 5x),
while other builds loaded the machine (load average 12 to 56). Each is the range over 2-second windows.

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

## 6. The web

What was tried, on `wasm32-unknown-unknown` (installed). nettai-luau needs WASI SDK 34
(crates/nettai-luau/README.md; fetched from https://github.com/WebAssembly/wasi-sdk/releases/tag/wasi-sdk-34,
`wasi-sdk-34.0-arm64-macos.tar.gz` on an Apple machine, and `WASI_SDK_PATH` set to where it is unpacked):

- **The engine type-checks.** With the SDK and `-Zbuild-std=std,panic_unwind`, nettai-frontend type-checks for
  the target, and with it nettai-battle, nettai-luau, nettai-content, nettai-render, nettai-match,
  nettai-netplay, nettai-replay and nettai-audio. So do nettai-rtc, nettai-extract's library and gilrs.
  - nettai-luau had stopped building there: three places took mlua's integer as `i64`, and it is `i32` on wasm32.
    That is fixed on this branch.
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
    (nettai-rtc does already). `std::thread::spawn` panics too: loading a game and playing out replays must run in
    steps on the event loop, or in a worker.
  - **Files.** The content (Luau modules) must be embedded or fetched; nettai-luau already takes modules from the
    host (`Pack::new`/`Pack::of`), but nettai-content's loaders read paths. The packs can't be served (they come
    from the user's ROMs). The user brings them: a dropped pack folder kept in IndexedDB, or their ROMs extracted
    in the browser (nettai-extract's library builds for wasm32, and `extract` works in memory). Replays go to
    IndexedDB, with download and upload.
  - **Sound.** cpal's `wasm-bindgen` feature (WebAudio), started on the first click as browsers require.
  - **Netplay** is nettai-rtc's web backend (the browser's `RTCPeerConnection`).
  - **Rendering.** Slint's femtovg on WebGL2, which calls the rendering notifier, so the clock carries over.
- **IME on the web.** Slint's canvas can't host the browser's IME, so the two name fields (the player's name, the
  room code) get a small HTML `<input>` laid over the canvas, used only while one is edited:
  1. The field's `edit()` calls the host (a `callback edit-text(rect, text)`) with its rectangle in window
     coordinates and its text. On the web, the host places an absolutely positioned `<input>` over the canvas
     there, in Murecho at the field's size, with its border and background transparent, so the Slint field
     stays the one that's seen. It sets the input's value and focuses it.
  2. The browser's IME composes in the input. Its `input` and `compositionend` events hand the value back
     (`ui.set_name`), and the Slint field shows it; the preedit is the input's own, drawn by the browser over the
     field.
  3. Enter, Tab, Esc, the arrows up and down, and blur end it: the input is hidden, the canvas refocused, and the
     same navigation action given to the screen.
  - On the desktop the host's `edit-text` does nothing and the Slint field takes the keys as now. The
    prototype's fields have the hooks this needs (`Field::edit`, `leave`, `Input.editing`); the overlay itself
    isn't built.

## 7. Android and iOS

Not built. What it would take:

- **Slint:**
  - Android is supported (`backend-android-activity-06`, built with `cargo apk` or xbuild and the NDK).
  - iOS is supported through winit (an Xcode project around the static library, as Slint's iOS template has it).
  - The femtovg renderer runs on Android (GLES), and the clock carries over. iOS renders with Skia on Metal,
    which doesn't call the rendering notifier (§1): the clock would move to a display link of the app's own
    or a timer.
- **The engine:** Luau's C++ builds with the platforms' clang (`cc`); nothing in the engine is
  platform-specific.
- **Files:**
  - The content embedded in the app.
  - The pack imported by the user (Android's Storage Access Framework, iOS's document picker) into the app's
    storage. Or the ROMs imported and extracted on the device, as on the web.
  - Replays in the app's storage, shared with the system's share sheet.
- **Sound:** cpal plays through AAudio/Oboe on Android and CoreAudio on iOS.
- **Input:**
  - Touch is the on-screen pad, already drawn when the window is upright. It needs multi-touch: Slint's
    `TouchArea` follows one pointer, so the pad would take the window's touch events itself.
  - Gamepads: gilrs has no Android or iOS backend. On Android winit delivers a pad's buttons as key events (the
    keycodes need mapping); on iOS it would take GameController.framework.
- **Text:** the system IME works through winit on Android and iOS.
- **Netplay:** nettai-rtc's native backend (the `rtc` crate over UDP) runs on both.
- **Lifecycle:** suspend and resume. Netplay can't tick in the background, so a suspended match is a dropped
  link, which reconnects within its 30 s or ends.

## 8. Builds and the build creator

A **build** is a player's side of one game, named and kept: a TOML file of its own, `builds/<game>/<name>.toml`
in the data folder. It names its game and the build, then states the side under `[side]` as a match file states
one (`nettai_match::file::side_toml`, read back by `resolve_side`: the same names, by the creator or by hand):

    game = "exe6"
    name = "Falzar heat"

    [side]
    navi = "megaman"
    version = "falzar"
    folder = [ ... ]

The **Builds** screen lists a game's builds (L and R go through the games), each with its navi's face, its folder's
count and whether it can play (or what the rules say first), above NEW BUILD and FROM A SAVE…. A save can also be
dropped on the window. Builds are chosen in Play and in the lobby (§4).

**The creator** (`crate::builds`) lays a build out from its game's rules' setup schema alone (`builds::layout`;
the rules declare no views), a tab for each kind of fact:

- **NAVI:** the build's name, the navi (by its face), and each fact of one value: a flag, a number, an enum's
  variants (EXE6's version, Beast Out, bug frags), or a fact's presets (EXE5's light or dark MegaMan, with the face
  the round starts him with: below). Then FROM A SAVE…,
  DUPLICATE and DELETE (confirmed twice). Beside them, what the round starts the navi with (HP, the buster, the
  custom screen, the Mega and Giga limits, the Regular memory, the abilities and the NaviCust's bugs) and every
  problem the rules see.
- **FOLDER** (the engine's folder facts): its 30 entries with their codes and the Regular and tag marks; the
  counts against the limits the round's stats give; a chip's picture as the chip window shows it. The browser
  offers the chips the side's rules let a folder hold (`folders::pool`), in the library's order, by class and
  searched; a chip's code puts it in the entry, and the next entry is chosen.
- **A few definitions** (`form[5]`, `form[16]`: EXE6's Crosses, EXE5's souls): a checklist of faces, of what
  `facts::offered` offers, up to the list's room, with NONE, DEFAULT and ITS VERSION'S OWN. Choosing a version
  states its own Crosses where none are stated.
- **The placement grid** (the NaviCust: `builds::grid`, chosen by its data's names and shape): the board by its
  size, its frame and its command line, the pieces in their colors and plus marks, outlined where the rules say
  something of one. A piece is taken from the list in one of its colors and held over the board, lit where it
  fits and red where it doesn't. Keys and the pad: arrows move it, L and R turn it, C (Y) compresses it, Enter (A)
  puts it down, Esc (B) puts it back; Enter on a placed piece picks it up, Delete (X) takes it off. The mouse
  holds it under the pointer: a click puts it down or picks one up, the wheel turns it, a right click turns a
  placed piece or takes off the one held.
- **A list of definitions** (the patch cards): its entries in order, each with its card's Parameter and Ability
  lines and its bugs (`builds::cards`, the strings' `patch_card_effects`), the MB in all; added from the
  collection's browser, moved and taken out.
- **The auto battle data** (EXE5's, `builds::auto`): its 42 places in the game's lists, each a chip, a program
  advance, a 0 or empty; the browser offers what the game writes in the place's list, or any class. FROM A SAVE…
  takes a save's places alone.
- A list of `{ <definition>, frames }` is a time each (`mm:ss.cc`), and any other list is shown as it stands; no
  game's setup has one a player states now.

Every edit is saved at once and the build checked again: nettai-match's `check::problems`, the build on both
sides of a match of its game, each problem beside its field and entry (a tab with problems is marked).

**What a build never states.** Every build in the app plays at the most its game allows, and states only what a
player chooses. The facts kept at the rules' defaults (`layout::at_defaults`) are those of the engine's level and
base HP roles, and the ones the app lists by game in `crates/nettai/builds.toml`: what a save brings to the
navi's stats (HP, the Regular memory, the sun, the level), the SP navi times, EXE5's auto battle records and its
Chaos Unison (always on: the user, "chaos unison should always be enabled in a build"). The creator neither shows
nor edits them. A navi that must have a level has its last. A build made new, read from a file or made from a save
is set to them (`layout::as_built`), and a place of the auto battle data that pointed at a record is emptied; a
save's import says which facts were set to the defaults.

**Presets** (`builds::presets`, the same file's `presets`): a fact a build states only as one of a few presets,
each of which sets several facts together. EXE5's are its light and dark MegaMan, as Tango's EXE5 save templates
have them, the only two (the user: "there's only 2 karma save presets in bn5, and they also end up setting
megaman's base hp"): LIGHT, the light/dark value 1000 and the base HP 1000, and DARK, 0 and 997. The value's row
steps between them and shows the face the round starts MegaMan with (`screen::Face`: the round's first tick, the
dark one's his dark face); the base HP is no row of its own. A team navi's base HP stays the rules' default (its
HP is its story's: `edit::without_forms`). A build whose value is another (made new, read from a file, made from
a save) takes the preset on its side of the game's line, 470 (content/exe5/rules/light_dark's `LIGHT_CHIPS`): a
new build's 500 is light's, and a save's import says which it took ("karma 100 is dark's: the dark preset (hp 997,
karma 0)").

**From a save** (`builds::import`): each game's compat crate reads its own saves (`exe6_compat::import`,
`exe5_compat::import`); a save of another game is refused.

## 9. Settings

`settings.toml` in the data folder keeps the language, the volume, the menu sounds, the battle's text, the
picture's density and the player's name, written as each changes and read as the app starts
(`crate::settings`). `$NETTAI_LANG` still names the first language; the tour keeps nothing.

## 10. Retiring nettai-demo

What nettai-demo does, and where each part goes:

| nettai-demo | Goes to |
|---|---|
| Live play against the stand-in, `--match` files, `--record` | nettai: Play (match files, builds) |
| Netplay (rooms, `--host`/`--join`) | nettai: the lobby (rooms, direct links) |
| Replays (`--replay`, watching) | nettai: Replays |
| The match editor (iced) | nettai: the build creator (a match file is two sides; Play opens match files) |
| `--headless` frames (`--png-scale`, `--keys`, `--objects`, `--mark`), headless `--replay` | nettai-tools |
| The trace replay of the original's recordings (the compat `trace` feature) | nettai-tools |
| `--audit`, `--audit-content` | nettai-tools |
| `--show-folders`, `--save-match`, setup dumps | nettai-tools |

nettai-tools is a small CLI crate (binary `nettai-tool`, no window toolkit; docs/tools.md) holding nettai-demo's
headless modules as they were (`headless`, `trace`, `content_audit`, `sound_lookups`), with the same flags, so
verify's scripts (lab-compare.sh, lab-batch.sh, identity.sh, compare.sh, play-headless.sh, embed-against.sh,
audit-against.sh, and the merge checks) switched from `nettai-demo …` to `nettai-tool …` (built with
`-p nettai-tools`) mechanically. Its frames, marks and audits are nettai-demo's, byte for byte. Without frames,
`--match FILE` says the setup (`--show-folders`, `--save-match`) and `--replay FILE` plays the replay to its end
and says whether it reproduces. A trace is rendered or audited: the user wanted no trace viewer ("i don't think you
really need a trace viewer").

The order was: the build creator; nettai-tools; verify's scripts and the merge checks switched; then, on
2026-10-06, nettai-demo deleted from the workspace with iced and what only it pulled in (`Cargo.lock`: 824
packages, then 738), and docs/frontend.md's program sections moved here and to docs/tools.md.
