# The app (`nettai`)

**Status (2026-10-06): a prototype.** The user asked for a game UI on desktop, the web and mobile, with no
webviews, and chose Slint ("it should be very polished though"). `crates/nettai` is that app. It runs on the
desktop (built and measured on macOS). §6 records what the web build needs and §7 what mobile would take.

nettai is a second host of nettai-frontend, beside nettai-demo, which stays the developer's tool and the match
editor. It wraps the battle in the screens a player sees:

- a title screen, with a battle playing itself on its monitor;
- Play: the game, a random match or a match file, and a preview of who fights in which arenas, against the
  stand-in;
- the battle;
- the netplay lobby: rooms through nettai-rtc;
- the replays: every set played is recorded;
- settings.

Every screen is navigated alike by the keyboard, a gamepad, the mouse and touch. The app's own strings are in
catalogs, one for each language; the content's names come from its locales.

    NETTAI_PACKS=<packs> cargo run --release -p nettai

The packs are found as nettai-demo finds them (`$NETTAI_PACKS`, else `data`). Sets are recorded to
`$NETTAI_REPLAYS` (else `replays`), and match files are listed from `$NETTAI_MATCHES` (else `matches`). For
netplay, `$NETTAI_SIGNAL` names the signaling server (§4). Some variables are for development:

| Variable | What it does |
|---|---|
| `NETTAI_LANG=<code>` | The first language shown, instead of the system's. |
| `NETTAI_PLAY=<game>` | Starts straight into a random set of the game. |
| `NETTAI_NETPLAY=<game>:<make\|join>:<CODE>` | Starts straight into that room, ready. |
| `NETTAI_PLAY_STATS`, `NETTAI_KEY_PROBE` | The latency figures of §5. |
| `NETTAI_PHYSICAL_PIXELS` | Presents the picture at the display's density. |
| `NETTAI_NO_TEXTURE` | Shows the picture as a new image each time, not in the texture (§1). |
| `NETTAI_RENDERER=femtovg\|software` | Picks Slint's renderer. |
| `NETTAI_TOUR=<folder>` | Walks every screen in each language, at a desktop's size and at a phone's, and writes each to a PNG (`NETTAI_TOUR_LANGS=en,ja,pseudo` picks the languages). |

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
- Esc pauses, and Tab shows the next language;
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
- **Play.** No game is assumed: the player chooses one. The arenas are each round's field, played alone for 24
  ticks and drawn (`arenas.rs`).
- **The battle.** The pause has resume, start over, language and quit. The result has the score, the replay
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
  the picture's density and the player's name. The settings aren't saved yet.

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

The lobby meets the other player in a room of nettai-rtc's signaling server, by the room's code:

- **Making a room** enters a new one at once. Its code (six letters and digits, none that read as another) is
  shown in large type and copied to the clipboard on confirm.
- **Joining** enters the room as soon as its six letters are typed.

The first in the room hosts. The library's lobby and handshake run over the link (`netplay::Agreeing`, the
datagrams of `nettai_frontend::lobby`):

1. Each player proposes the game, a triple battle with its places left to the seed, and a random side of it.
2. Readiness is each player's toggle, and each sees the other's. Changing the game clears both.
3. Once both are ready on the same settings, the match is agreed and the battle is a `NetPlayer` over the link
   (`netplay::Framed`: the link as the library's `Channel`, frames told from the lobby's datagrams by their first
   byte, as nettai-demo's connection does). It is recorded.

The battle shows the connection's figures (ping, present delay, rollbacks) and "reconnecting (N s)" while the link
is down (`NetStatus::reconnecting`). Netplay can't pause: the pause's panel says so and holds no buttons.

The signaling server isn't deployed. Run it locally:

    cd signaling && npx wrangler dev        # http://127.0.0.1:8787
    NETTAI_SIGNAL=ws://127.0.0.1:8787 cargo run --release -p nettai

Without `$NETTAI_SIGNAL` the lobby says there is none. Two windows on one machine meet with
`NETTAI_NETPLAY=exe6:make:ROOM42` and `NETTAI_NETPLAY=exe6:join:ROOM42`.

What's missing:

- The players' names don't cross yet: the lobby's protocol carries none. A name could ride on the signaling
  server's welcome or in the lobby's message.
- The direct links (`Link::host`, `Link::join`) aren't offered on the screen.
- The present delay can't be set there.

## 5. Latency and smoothness

Measured as nettai-demo measures (docs/frontend.md §7). `NETTAI_KEY_PROBE` presses the right arrow every 300 to
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

What was tried, on `wasm32-unknown-unknown` (installed). nettai-luau needs WASI SDK 34 (crates/nettai-luau/README.md):

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
