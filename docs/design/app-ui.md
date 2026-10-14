# The nettai app's UI

**Status (2026-10-12): approved design, implementation started.** The user asked for the app's UI to be rethought and
redesigned entirely ("throw away everything you have right now because it's so janky and not great. do keep using
slint though"), reviewed mockups over a day, and then said "throw out the old nettai and start implementing". This
document is that design. docs/app.md is the app as it is built: its foundation is (the design system, navigation,
input, Settings, the battle's host), and Play, Practice, Builds and Replays are to be filled in on it.

The mockups are the picture of every screen: https://claude.ai/artifact/5Kg4TfQhZeMXB5C45rXM2Z, a copy of which is
`bn6battle-verify/docs/nettai-ui/mockups.html` (not in this repository: it embeds chip icons and art taken from a
pack). Its CSS is the reference for sizes, colors and spacing, in units where the window is 1280 by 800; this
document gives the same numbers. Where the two differ, this document is right.

## 1. What the user asked for

In their words (2026-10-12), in order:

- "make sure all the sizing doesn't randomly shift as well as text changes. make sure all the keyboard navigation
  works smoothly, especially when going through input boxes and stuff. make sure all the ui elements don't clip off
  textboxes etc and the ui element spacing is comfortable"
- top level: PLAY (the netplay UI), TRAINING, BUILDS, REPLAYS; less emphasized: settings, exit
- a game selector in the top right (4/5/6); the screen shows a demo reel on the right
- "play title screen music depending on game, use the game's title screen music"
- "saves should be preset to": Red Sun or Blue Moon, Team ProtoMan or Team Colonel, Gregar or Falzar
- editable for a build: the navi, the chip folder, and for MegaMan the NaviCust, the patch cards and the auto battle
  data
- i18n support
- "in battle pause should be replaced with nettai's pause menu"
- "use https://github.com/tangobattle/gamepad-facade"
- "disable L to run in battle"
- "replays should allow seeking and export"
- "signaling server should assign invite code instead of the client generating one"
- "don't show any internal error strings to the user. if there are internal errors that must be surfaced as user
  errors, they should be replaced with enums and then go through i18n. they should not be overly technical errors"
- "when waiting for an opponent in netplay mode, start a sparring session in training mode"
- "don't have random build support, you must have a build. when creating a build, fill folders with the default
  chips for an empty save for the game"
- "chip restrictions for folders should be based on the rules in tango (e.g. version specific gigas)"

## 2. What the user decided in review

Each is settled unless the user changes it.

1. **The game selector** is the game's name between two arrows that steps through the three games and wraps, not a
   segmented control. A game with no pack is skipped.
2. **Versions are listed in this order everywhere:** Red Sun, Blue Moon; Team ProtoMan, Team Colonel; Gregar, Falzar.
3. **Rounds in netplay are chosen in the lobby.**
4. **Training is like Street Fighter's:** a dummy configured from a menu while in training. The word on screen is
   TRAINING, never "sparring". (It is PRACTICE since item 28.)
5. **Direct connect stays** (host and port, no server).
6. **Input delay is configurable, 4 by default.**
7. **The look is modern and techy**, not retro. A first, pixel-and-bevel "Battle Network" skin was rejected as "too
   retro".
8. **Little explanatory text.** Subtitles such as "Battle someone online" under a menu item are "claudeslop" and
   are gone. Stripping every line was too far ("a little more explanatory text than that"). So: a line only where
   it says something its label can't (what a setting changes, what happens next, how to fix an error). No subtitles
   on menu items or buttons.
9. **The build overview shows the compiled result:** every stat as the NaviCust and patch cards leave it.
10. **The NaviCust:** EXE4 and EXE5 draw only their 5x5 board (they have no out-of-bounds cells); EXE6 draws its 7x7
    with the out-of-bounds ring and the command line clearly marked. Out of bounds "is fine, it's not an error", and
    "bugs can sometimes be good in a build", so neither is styled as a warning. A bug is shown by its name and its
    level, whoever brings it, the NaviCust or a patch card (the build overview lists both; the NaviCust tab only
    the NaviCust's own, item 26). A bug's name is a full name spelled out from the game's
    own patch card effect name for the same bug ("MoveBug" is Movement Bug, "BatHPbug" is Battle HP Bug, "CustDmg"
    is Custom Damage), not the abbreviation and not a word of ours; a card's own lines stay the game's strings. A
    card's bug that writes what no level of the bug does is shown with its amount in the level's place (Custom
    Damage 50; Buster Blank 4 of 16; Movement Grass 2 of 8), and a card's drawback that is no bug of its own kind
    (Super Armor off) stays the card's line. **Every bug, everywhere, is purple.** A plus part is a cell with a cross that reaches its edges, with no
    border around the cell.
11. **Chips show their element** with the game's own element icon, at the right of the row, just left of the code.
    Chip icons and chip art are the pack's own.
12. **The chip library has no filters** by element or class: "it should all be done by sorting". Sort keys: library
    order, alphabetical, code, MB, element, attack power. There is no class column; class is "coloring something
    subtle" (the chip's name: Mega blue, Giga pink).
13. **The folder editor** is two panes at the build overview's spacing: the folder in a blue-bordered panel, the
    library an open list. Denser layouts with empty areas ("there is so much spacing") and a three-pane one with
    large art ("too much going on") were both rejected. **That framed-loadout, open-library look is the pattern** for
    patch cards and the other editors.
14. **REG is reddish pink** as in the game; tag chips are supported and marked **TAG1** and **TAG2**.
15. **A loading screen** loads assets before a battle starts, "to avoid stuttering".
16. **Builds are private.** "the lobby shouldn't show build name or which game, since that's private information to
    the player"; "the build name should not really be visible across the wire or in replays or anything". (The
    lobby's protocol sends none of a build's name, version or navi label.)
17. **During a match both builds can be viewed**, as in Tango; and **the lobby has a "closed builds" setting**.
    The lobby's match settings also have **a toggle for patch cards, on or off** ("for netbattle settings you should
    also have a toggle for patch cards on/off").
18. **Connection quality is shown during play.**
19. **Patch cards are shown comprehensively**, have no image area ("patch cards don't have images"), have no on/off
    switch ("nettai doesn't support turning them off"), and a card that doesn't fit says so in the error red
    ("Needs 24 MB"), "like how errors in the folder editor get surfaced".
20. **EXE4's patch card slots are named 0A to 0F**, and not every card fits every slot: each card states the one
    slot it belongs in (`slot = cards.SLOT_0A` to `SLOT_0F` in its definition).
21. **Phone (upright) layouts are in scope.**
22. **A game is called by its own name in the language shown** ("in english, EXE4/EXE5/EXE6 should be called
    BN4/BN5/BN6"): BN4, BN5 and BN6 in English, EXE4, EXE5 and EXE6 in Japanese, in the header's selector and in
    every message that names a game. Each game states its name in its locales (the text table `game`, `name`);
    the app reads it from the content, so a game without its pack is named too.
23. **Each game has a look of its own** (2026-10-13): "each selected game in nettai should also have a different
    visual identity". From the mockup of three (`bn6battle-verify/docs/nettai-ui/game-identities.html`) the user
    took BN4's comet ("i like bn4c"), BN5's liberation map and BN6's look as it was. Switching the game changes the
    colors, the glow and the backdrop's pattern (§4); the layout, the sizes, the keys, the motion and the type stay
    the same. The rest of what they said of it:
    - *One typeface for every game.* A Latin face per game (Stick for BN4, Saira Condensed for BN5) was tried and
      dropped: "stick latin font looks ridiculous for bn4. you should use the same latin and japanese typefaces
      everywhere instead of changing them up per game". Latin display text is Chakra Petch in every game.
    - *Japanese in a calm gothic:* "you should find fonts that work in japanese as well, since there needs to be
      japanese support"; of the first candidates, "these japanese fonts look horrible. they should be a little less
      playful"; and of the six in the mockup, M PLUS 1. Japanese display text is M PLUS 1 at 800 in every game;
      body text stays Murecho.
    - *The wordmark is one color:* "don't color nettai separately like that, same color for the whole word".
    - *The backdrop's pattern as strong as the mockup:* "backdrop pattern looks fine".
    - *A change of game sweeps:* "switching games should the new game's pattern sweeping across the window". The
      new game's ground comes in from the side of the arrow pressed, behind a moving edge (§4, "Motion").
    - Each game's look is stated in its own content, beside its versions' colors and its name; the app names no
      game, and a game whose content states no look is drawn in the app's own (BN6's).
24. **The packs come from the player's Legacy Collection** (2026-10-13): "you should detect bnlc vol2 and extract
    all assets from there on startup if they are not extracted yet (and if nettai version changes, reextract
    assets). there is bnlc detection code in tango you can steal". The app finds Battle Network Legacy Collection
    Vol. 2 through Steam, as Tango does, or in a folder the player chose, and extracts each game's pack from it into
    its own data folder as it starts: when the pack is missing, or another version of nettai made it. A game is
    playable as its pack lands. Without the collection, a game with no pack says so, with a way to choose the
    collection's folder. (docs/app.md §17.)
25. **An About screen** (2026-10-13): "add an about screen with license and dependency info etc like for slint".
    About is a minor item of the main menu, beside Settings: the app's version and license, Slint's "Made with
    Slint" widget (which Slint's royalty-free license asks for in an About screen reached from the top menu), and
    every font and package the app is built from with its license's texts (§6, docs/app.md §18).
26. **The NaviCust's bugs are its own, and a program is placed as often as wanted** (2026-10-13): "navicust
    shouldn't display patch card bugs. also you should allow a navicust part to be placed multiple times". The
    NaviCust tab's Bugs (on a phone, its Bugs line) list the bugs the NaviCust brings, the build's without its
    patch cards, as its Effects already were; a card's bugs stay on the Patch cards tab (its lines, Together) and
    in the build overview, which lists every bug. Taking a program from the list, by Enter, a click or a drag out
    of its row, always takes a new copy in hand, however many copies are placed (before, it took a placed
    program's last copy off); Del (or a right click) on its row takes the last copy off, and a placed piece is
    taken off as before. A row with more than one copy placed says how many ("Placed ×2"). No game's rules limit
    copies: the checks (EXE5's and EXE6's `@exelib/navicust/compile`'s `check`, EXE4's rules/navicust `validate`)
    ask only for a color the program comes in, a turn, the board and no overlap, and of EXE5's and EXE6's copies of
    one program in one color, that all or none are compressed. What the copies do is the games' own compile: on
    the command line only the rightmost of an exclusive group works in EXE5 and EXE6 (two SuprArmrs are one), and
    in EXE4 a program in one color works once there; plus parts add up.
27. **The auto battle data's mega, giga and program advance lists hold their class alone** (2026-10-13): "also fix
    the abd editor to only allow mega chips in mega, giga chips in giga, and PA in program advance". It is a rule
    of the games' content (each such list's `own_class` in @exelib/auto_battle/places, which the rules' `validate`
    holds a place to), and the editor reads it: a place of one of those lists is offered that class's chips, the
    Program Advances among them. The first and standard lists are as they were, a chip of any class (the user named
    three lists). (§6, docs/app.md §8.)
28. **Training is called Practice** (2026-10-13): "Training should be called Practice actually, and you should be
    able to go to builds screen from the practice screen somehow while having the relevant build displayed". The
    word on screen is PRACTICE wherever the player meets it: the main menu, the screen and its title, its menu over
    the battle, START PRACTICE, the tag beside a waiting invite and STOP PRACTICE; in Japanese 練習, where
    トレーニング was. The code is named for it too ("training.rs and TrainingView should be renamed practice but
    nettai-training can stay as it is"): the AI's crate keeps its name. **Practice opens a build in Builds:**
    beside each build's select on Practice's entry is EDIT (or E, or the pad's north button), which opens that
    build in Builds' editor; Back there (Esc, the pad's B, the header's ‹ PRACTICE) comes straight back to
    Practice, which shows the build as the edit left it. CREATE A BUILD opens Builds' list the same way, Back
    going back to Practice. (§6, docs/app.md §8 and §15.)
29. **The battle's music is the player's pick** (2026-10-13): "you should also allow for selectable battle themes
    (bn4 virus battle, bn4 boss battle, bn4 tournament battle, bn4 vs duo, bn5 virus battle, bn5 boss battle, bn5
    vs nebula gray, bn6 virus battle, bn6 boss battle, bn6 vs cybeast, bn6 crossover battle)". Any theme plays in
    any game's battle (a BN6 battle can play BN4's tournament battle theme); the first choice, "Game's own", is
    what the battle's game plays without one. A first draft put it in Settings' Audio group; the user: "settings
    is a weird place to put it". Asked where, between a shared setting of the match, each player's own beside the
    arena, and a row of the menu over the battle, they chose **"Beside the arena, each their own"**: a Music row
    in Practice's entry beside Arena and Background (a Your music in Play's lobby too, until the user: "don't allow
    bgm select in lobby"): each player picks and hears their own, nothing of it is sent. The pick is one,
    remembered between runs, and Practice's sessions play it; a match, a replay and its video play the game's own
    ("why does the music setting carry over from practice mode into pvp mode? huh?", "for pvp it should use the
    game's default for now"), and the practice battle beside a waiting invite leaves the menus' music on ("in the
    practice mode while the lobby is waiting, don't switch the music"). The themes and their names are the games' content (each game's rules'
    `battle_themes`, named in its locales), shown after the game's name as the language says it ("BN4 Tournament
    Battle", "EXE4 トーナメントバトル"). (§6, docs/app.md §1, "The battle's music".) Then: "in the practice
    screen, if the bgm choice changes from game default you should start playing the bgm": Practice's entry plays
    the pick in place of the title music while it shows (once the row has stayed on it a moment, so a run of steps
    starts one song), the title music again for "Game's own" and elsewhere; and "also you should be able to change
    the bgm in the practice menu": the practice menu's Battle rows have Music, the same pick, which switches the
    battle's music at once.
30. **A build is 構築 in Japanese** (2026-10-13): "build should be called "構築" in japanese". Every Japanese string
    that said ビルド says 構築 (構築, 新規構築, 構築を編集, 相手の構築, この構築は使えません, and the rest).
31. **The backgrounds are one pool** (2026-10-13): "is it possible to use e.g. exe4 backgrounds in exe5? as in if
    they all came from the same pool of backgrounds?", then "sure do that". Every game's backgrounds are one pool,
    any of which a battle of any game is drawn on, each named by its game and its own name ("BN4 ACDC Area",
    "EXE4 秋原エリア"). Practice's Background row offers the whole pool after Random, and a round's random
    background (Practice's Random, every round of Play) is drawn from it. Asked whether the background stays the
    match's, both players seeing the round's and a replay keeping it, or becomes each player's own as the music
    is, they chose **"Shared, as now"**: the lobby agrees it with the round and a replay records it. The rules,
    the chips and the field stay the arena game's. (§6, docs/app.md §1, "The battle's background".)
32. **A dialog's text field is ready to type in** (2026-10-13): "you should auto focus the primary text field on
    dialogs that pop up". A dialog that asks for text (a build's name, new or renamed; a direct connection's
    address) opens with the field edited and its text selected.
33. **A search starts its list from the top** (2026-10-13): "when a search box is changed, it should reset
    scroll". Each change to a search field shows its list from its first row.
34. **First to, not rounds** (2026-10-13): "instead of "rounds" you should have "first to" then you should be able
    to select 1, 2, 3, 4 etc". The lobby's setting is First to (先取), a stepper of the round wins that take the
    set, 1 to 10, the number alone (in Japanese "2本先取"). First to 1 is the original's single battle and first to
    2 its triple battle, played exactly as before; a set first to `n` is `2n − 1` rounds at most, decided as the
    original decides its triple battle (a drawn round is one of them). Ten is the top: the longest set fighting
    games' players commonly play ("10先"), and its 19 rounds are each numbered by the games' own BATTLE n START!
    banner in its two digits. (§6, docs/app.md §4.)
35. **Practice's arena preview moves** (2026-10-13): "you should also animate the background in the practice setup
    screen", then "you should also run the field a little so it shows stuff like wind fields and panel animations"
    and "you should disable the custom gauge in the arena preview". The preview on Practice's entry is the
    session's first round run into its fight with nobody pressing anything: its background animates and scrolls,
    its panels animate and its field does what it does (wind blows, roads and volcanoes run), the navis stand,
    and it never shows the custom screen, a banner or a round's end. The HUD has no custom gauge. (§6, docs/app.md
    §15.)
36. **A scrolling page is gone through before what is under it** (2026-10-13): "it's impossible to scroll the
    practice menu without selecting the button on the bottom first, it should scroll the entire length before
    moving to buttons underneath". And the practice beside a waiting invite has no Music row: "in the waiting for
    opponent screen, it really shouldn't let you change the music because it does nothing" (its music is the
    menus', §2.29).

## 3. Chosen where the user gave no answer

These were put to the user as questions with a stated default and got no reply before "start implementing". They
are the defaults; each is easy to reverse and is listed for review.

| Question | Chosen |
|---|---|
| Typefaces | Headings, buttons and labels in Chakra Petch; all other text in Murecho; codes and numbers in M PLUS 1 Code. (Japanese headings were Murecho's until the user chose M PLUS 1 for them: §2.23) |
| A game's look, where the user said nothing (§2.23) | On a change of game the panels' and the text's colors cross-fade over the sweep's time (they don't change at the moving edge, which would mean drawing every screen twice); no art from the packs in the header |
| A build's version | Fixed when the build is made, as a save's is ("It can't be changed later") |
| Netplay rounds | A single battle or best of 3, a shared setting either player can change (since §2.34: First to, 1 to 10) |
| The opponent's navi in the lobby | Hidden, with the build's name and version |
| What "export" makes | An MP4. A `nettai-video` crate writes a NUT stream (raw frames and PCM) that the app pipes to ffmpeg; the app never saves the `.nut` (the user: "don't allow raw .nut export") |
| The demo reel | The player's latest replays of the selected game; with none, the two versions' starter builds on autopilot |
| The dummy while waiting online | A copy of the player's own build, standing still, with the practice menu there to change it |
| Practice's options | The menu first, with what the dummy can already do; then refilling HP, the always-full custom gauge, and recording, in that order; folder-order draws and a second pad last. (Recording went later: "get rid of recording support, it's so complicated".) |
| Practice and replays | Practice is not recorded |
| Text entry on a pad | The invite code has a six-box picker a pad can use; names, addresses and searches need a keyboard |
| Closed builds and replays | A replay of a closed-builds match hides the builds in the viewer too |
| Smaller NaviCust boards | Not offered: every build has the full board |
| Light or dark MegaMan (EXE4, EXE5) | Kept from the old creator, which the mockups left out: a Light or Dark choice on the Navi tab, the two save presets (the light/dark value and the base HP together) |

## 4. The look

A modern network terminal: a dark ground under a faint pattern, thin lines, one cut corner on panels and buttons, a
bracket cursor, one bright color for the main action, red and blue for the two sides. Each game has its own colors
and pattern (§2.23): BN6's is the night-blue, cyan and amber over hexagons the app was first drawn in, BN5's
field-ops lime and gold over an isometric grid (its liberation missions' map), BN4's ice-blue and magenta over a
starfield with a comet's streak (Duo's comet).

**Units.** Everything below is in units of a 1280 by 800 window (16:10). The window scales the whole layout from
that reference; the battle picture alone is always a whole multiple of 240 by 160.

**Colors.** The ones that change with the game:

| Role | BN4 | BN5 | BN6 |
|---|---|---|---|
| ground, top to bottom | `#09071a` to `#020106` | `#11150f` to `#040603` | `#0a1330` to `#050916` |
| the glow at its top right | `rgba(159,240,255,0.16)` | `rgba(200,255,90,0.16)` | `rgba(40,110,255,0.28)` |
| panel fill | `#100d22` | `#141a12` | `#0c1836` |
| raised (a button, a pressed row) | `#1b1636` | `#1d2619` | `#13254d` |
| line (every border) | `#3d3574` | `#3f5435` | `#27508f` |
| inset (a field, a value box) | `#06040e` | `#080b06` | `#040813` |
| text; muted; dim | `#f1efff`; `#aea8d8`; `#655f94` | `#f0f5e8`; `#a6b597`; `#617056` | `#eaf3ff`; `#8ea8d4`; `#55698f` |
| the cursor, the active pane's title, the chosen row's bar | `#9ff0ff` | `#c8ff5a` | `#66e0ff` |
| the main action, its text | `#ff4fd8`, `#1a0016` | `#ffcc33`, `#171200` | `#ffb62e`, `#1b1100` |
| the backdrop's pattern | a starfield: three lattices of dots (white at 0.22 and 0.14, `#9ff0ff` at 0.20) and a streak across the window at 118 degrees from `#9ff0ff` into the main action's magenta | an isometric grid: 1-unit lines of `#c8ff5a` at 0.055, 46 apart at 30 degrees each way, over a faint wash of it low on the window | hexagons stroked in `#62b4ff` at 0.078 |

The colors made from them follow: a raised control's hover is the raised fill a third brighter; the header's and the
footer's bar is the ground's foot darker at 0.72 (`rgba(3,6,15,0.72)` in BN6), a dialog's shade the same at 0.8;
the cursor's glow, the chosen row's tint and a hovered row's are the cursor's color at 0.4, 0.1 and 0.05; the
out-of-bounds hatching is the muted text at 0.2 with its edge at 0.6. The rest are the same in every game, and each
stays readable on all three grounds:

| Role | Value |
|---|---|
| player one, player two | `#ff5d50`, `#4a93ff` |
| good; an error or a reason something can't be done | `#45e0a0`; `#ff9272` |
| a bug, anywhere | `#b497ff` |
| a Mega chip's name; a Giga chip's name | `#6cc4ff`; `#ff7eb6` |
| REG; TAG1 and TAG2 | `#ff5c82`; `#7fe36c` |
| a chip's code | `#ffb62e` |
| the command line's empty cells | `#0e3550` |

(Version tags are each version's own color, which the game's content states.)

**Type**, the same in every game. Chakra Petch 500/600/700 for titles, buttons, tabs, labels (labels are 11,
uppercase, tracked 0.18em); in Japanese these are M PLUS 1 at 800, tracked nearly not; Murecho 400 to 900 for text,
in either language; M PLUS 1 Code 500/700 for codes, numbers and times. All four are under the SIL Open Font License
and are bundled. Sizes: 11, 12, 13, 14 (the base), 16, 20, 28; the main menu's items 36. The wordmark "nettai" is
Chakra Petch 700 in the text's color, one color for the whole word.

**Shapes.** Panels, dialogs and buttons are rectangles with a 1 unit line and the bottom right corner cut at 45
degrees (12 on a panel, 18 on a dialog or a menu, 9 on a button), the line following the cut. A panel has a short
accent in the cursor's color (cyan in BN6), 72 by 2, at its top left. Nothing is slanted and nothing is rounded.

**The cursor** is four corner brackets (each arm 10 long, 2 thick, in the cursor's color with a soft glow) drawn
over the layout, 5 outside the focused control. It is one object that slides from control to control; it takes no
space, and nothing under it changes size. It shows only when a keyboard or a pad is the last thing used. A row
a list marks keeps its mark, the chosen row's bar and tint, when the pointer hides the cursor: a library's
focused row, and in the build editor the build's row being filled (a folder's entry, a patch card or slot, an
auto battle place) while the focus is on it or in the library beside it, so a row clicked shows as chosen (the
user: "it is impossible to tell now what is selected with the mouse on the left hand side").

**Motion.** The user: "you should have way more ui animations so it feels like an actual game". Everything
moves, briefly, and nothing waits on it.

- *It never changes a size.* Motion is transforms, opacity, clipping and overlays; rule 2 of §5 stands.
- *It never blocks input.* A press acts at once; a transition under way is cut short or redirected; the cursor
  never trails the focus by more than its own travel.
- *It is short.* 90 to 160 ms for a control, 180 to 260 for a panel or a screen, up to 500 for the two set pieces
  (entering a battle, a result). Things arriving ease out; things leaving ease in. One table of durations and
  curves in the theme; no screen invents its own.
- *It can be turned down.* Video has "Reduce motion" (on by default where the system asks for it): everything
  becomes a cut or a short fade. The screenshot pass runs with motion settled.

What moves:

| Where | What |
|---|---|
| the field | the pattern drifts slowly and a soft pulse crosses it now and then; it rushes as a battle is entered |
| a change of game | the new game's ground (its gradient, glow and pattern) sweeps across the window from the side of the arrow pressed (200), a lit edge and a soft band of the new cursor's color leading it, while the panels' and the text's colors cross-fade over the same time; a second press mid-sweep sweeps the next game in over both at once |
| the cursor | it glides between controls, its brackets breathe 2 in and out while idle, and they snap inward on Enter |
| a screen | leaving, the body slides 16 and fades (120); arriving, the title wipes in behind its cyan bar and the panels come one after another, 30 apart, each rising 12 as it fades in (200); Back runs the other way |
| the main menu | at start the logo, the four words and the reel assemble in turn; the chosen word's bar grows from the left and its tint sweeps in; changing the game slides the name out and in the arrow's way and crossfades the reel and the music |
| a panel | its cyan accent draws from the left as it arrives |
| a button | a press flashes its fill and dips it 1; the main action has a slow sheen |
| a stepper, a tab | the value slides out and in within its box, the way pressed; a tab's underline slides and the body follows that way |
| a list | the chosen row's bar and tint slide to the next row; scrolling is smooth, never a jump |
| the folder | a chip put in flies from its library row to its slot, which flashes; a count that changes ticks to its number; a refusal shakes the row 4, twice, and flashes its reason |
| the NaviCust | a piece picked up lifts (to 1.06, with a shadow), turns as a quarter turn, and drops with a pulse along its cells; the command line's arrows pulse; a bug that appears pulses purple once |
| a dialog, a menu | the scrim fades (120) as the panel grows from 96% (160); the practice and match menus slide in from the left |
| loading | a bar fills smoothly under a moving highlight; before a battle the two cards slide in from their sides and "VS" stamps, and when both are full the screen wipes into the battle |
| the lobby | the opponent's card slides in as they join; READY stamps with a flash |
| an error | a field's message fades in; a failure's dialog shakes once |

Every move of the cursor, confirm, back and refusal also has the selected game's own menu sound, as the old app
had.

**Metrics.** Page margins 32. The header is 64 tall and the footer 44, the same on every menu screen. A control is
40 tall. A list row is 44, or 26 with a 2 gap where chips or cards are listed (the build overview's spacing, which
the editors share). Gaps are 8, 12, 16, 24.

**The header** holds, from the left: a Back button naming where it goes (`‹ MENU`, `‹ BUILDS`), the screen's title
with a cyan bar before it, then at the right the game selector (§2.1). Where the game can't change (inside a
build, the lobby, a replay), the game's name stays in the same place without its arrows.

**The footer** holds key hints at the left (key caps or pad buttons, by what was last used) and one short status at
the right. On a phone there is no footer.

## 5. Rules every screen follows

These answer the user's first paragraph (§1), and a screenshot pass checks them.

1. **One grid.** The metrics above, everywhere.
2. **Nothing resizes with its content.** Every value has a box of fixed width; text too long is cut with an
   ellipsis; digits are fixed-width; lines for messages, hints and errors are there when empty.
3. **Little text, and none that repeats a label** (§2.8).
4. **Sized for translation.** Buttons and labels are sized for the longest language. A pseudo-locale 40% longer than
   English runs through every screen in the screenshot pass, which fails on clipped or overlapping text.
5. **The cursor takes no space** (§4).
6. **One focus, always.** Arrows move to the neighbor in that direction; Tab and Shift+Tab move in reading order;
   **Enter activates and Esc goes back** on the keyboard, whatever is bound; the keys bound to the GBA's L and R
   are the game (or a screen's tabs) before and after. On a pad the buttons bound to the GBA's A and B confirm and
   go back (south and west until rebound). The user first asked for the GBA's A and B everywhere ("i feel like the
   GBA A and B buttons should be the universal confirm/cancel buttons everywhere, instead of enter+esc"), tried
   it, and took the keyboard's half back: "actually z and x for navigation is not very good, enter + esc was
   fine". A screen opens with focus on its main control and remembers where it was.
7. **Text fields don't trap.** The arrows walk past a field without changing it. Enter or a typed letter starts
   editing, and so does the pad's A. While editing, Left and Right move the caret, Enter confirms, Esc restores
   the old text, and Up, Down or Tab confirm and move on; the pad's A confirms and its B restores. The footer's
   hints are the field's own while it is edited. The field's border and caret light while it is edited.
8. **Text never touches a border.** A field clips its text at its inner padding (12) and scrolls to keep the caret
   in view.
9. **Lists.** Rows have one height; the focused row is scrolled into view; the scrollbar's space is reserved.
10. **Pads work the same way**, through gamepad-facade, by the pad's own bindings (A confirms, B goes back), with
    the hints switching to the pad's buttons as bound.
11. **One frame for what the build holds.** The folder, the patch cards, the NaviCust board and the build overview
    sit in a blue-bordered panel; the list to pick from beside it has no frame; the side the cursor is in has its
    title lit.
12. **Errors are a fixed list** (§8.4). A reason something can't be done is shown in the error red at the end of its
    row or in the field's reserved line; a failure that ends what the player was doing is a dialog.

## 6. The screens

Each is in the mockups under the same name.

### Loading
- **Starting up:** the logo, one bar 480 wide, "Loading BN6" and a percentage. The game last used loads before the
  menu appears; the others load behind it, and switching to one not ready shows the bar again.
- **A pack being extracted** (§2.24): the same bar, "Extracting BN6" and a percentage, before "Loading BN6". The
  game last used is extracted first; the others behind the menu, one after the other, and switching to one not
  extracted yet shows its bar.
- **No game, and no Legacy Collection:** the logo, the error's line ("nettai needs Battle Network Legacy Collection
  Vol. 2. Install it from Steam, or choose its folder.") and CHOOSE FOLDER, which opens the system's folder
  dialog. Where a game can be played, the same error is a dialog with CHOOSE FOLDER and OK.
- **Before a battle** (online, practice, a replay): the two players' cards facing each other, each with its own bar.
  Everything the match can use is loaded before its first frame: both navis and their forms, every chip in both
  folders, the arena, the music and the sounds. Online the match starts when both bars are full. The opponent's card
  carries their name alone (§2.16).

### Main menu
- Left: PLAY, PRACTICE, BUILDS, REPLAYS, each the word alone, 36, in a column 416 wide, rows 72 tall. Below a line:
  Settings, About and Exit, 16, muted.
- Right: the demo reel, the battle picture at three times (720 by 480), with no caption under it ("don't show the
  caption text").
- The selected game's title music plays across the menu screens and fades out as a battle starts.

### Play
- Left: the player's builds of the game, an open list (rows 52: the name, then "MegaMan · ready" or what is wrong,
  and the version's tag). With none: one button, Create a build.
- Right: the build overview (below), framed.
- Below, one row: CREATE INVITE (the main action) | the code field and JOIN, with a reserved space beside for a
  message | DIRECT CONNECT at the far right.
- **Direct connection** is a dialog: Host (this machine's address with Copy, and HOST; "Send this address to your
  opponent.") and Join (an address field and CONNECT).
- **Waiting for an opponent:** a practice battle starts at once (the picture at three times, tagged PRACTICE), with
  a panel beside it: the invite code, large, in fixed-width type, grouped in threes; COPY CODE; "Waiting for
  opponent"; "The lobby opens when they join."; CANCEL. Keys go to the battle; Esc or Start opens the practice menu
  with Copy invite code and Cancel invite added.
- **The lobby:** the two players' cards (yours shows your build; theirs shows their name and whether they are
  ready, nothing else), then one row of settings: First to (a stepper of the round wins that take the set, 1 to
  10, 2 at first: §2.34), Patch cards
  (on, off), Closed builds (off, on), Input delay (a stepper, yours alone, default 4, remembered), the
  connection's bars and ping (no music: "don't allow bgm select in lobby"; the battle's music is picked in
  Practice, §2.29); then READY, CHANGE BUILD, LEAVE; then a reserved status line. First to, Patch cards and Closed
  builds are shared: either player changes them, both see it, and both ready marks clear; the opponent never sees
  Input delay. With Patch
  cards off, neither player's cards apply in the match: each build plays as it is without them, and a build that
  breaks a rule without its cards (a sixth Mega chip a card allowed, say) can't ready up and says why.

### Practice
- **Entry:** five rows, Your build, Dummy's build, Arena and Background (the user: "in training mode can you
  also add arena + background select?": the game's arenas and the background pool, every game's battle
  backgrounds (§2.31), Random first, a Random background drawn from the pool; "the arena names are meaningless
  so you should just have like a pager widget that goes through them. background should also be a arrow button
  pager thing": each an arrow pager, the arena by its place, "12 / 96", the background by its game and its
  name, "BN4 ACDC Area"), and Music (the battle's music, §2.29: a row laid out as Arena's, stepped by its arrows, Enter
  opening its list; a theme plays here in place of the title music once
  the row rests on it a moment), all remembered, and START PRACTICE; beside them the two builds facing each other
  and the arena. One line: "Set up the dummy from the menu once you're in."
- **The arena preview moves** (§2.35): the round the session opens on, run into its fight with nothing pressed
  while the entry shows it, its background animating and scrolling and its field doing what it does (wind, roads,
  volcanoes, panels animating), the navis standing, with no custom gauge; never a custom screen, a banner or a
  round's end. A change of arena or background shows the new one from its start. The screenshot pass holds it
  as the round's opening ends.
- **EDIT** (§2.28) beside each build's select, a button of its own (Tab and Shift+Tab reach it, Up and Down go
  from one to the other), opens that build in Builds' editor; so do E and the pad's north button, on the dummy's
  row or EDIT the dummy's build and anywhere else the player's (the footer's "Edit build"). Back from the editor
  comes straight back to the entry, the two builds still chosen and shown as the edit left them (one that can't
  be used now says why in the reserved line). A build gone from Builds is missed as any is: the first that can
  be used is chosen. With no build, CREATE A BUILD opens Builds' list, whose Back goes back to Practice.
- **The practice menu** (Start or Esc in a practice battle): a panel on the left, the battle behind it in the game's own pause
  ("music continues, stuff still animates"), without the game's PAUSED banner, with the dummy
  in view. Rows are a label and a value between arrows, changed at once by Left and Right:
  - Dummy: Action (stands still, moves around, fights, a second pad), with its Level (easy, normal, hard: how well
    it fights) under it while it is Fights. Its custom screen goes with its Action. No recording: "get rid of
    recording support, it's so complicated". The user: "the dummy menu is so confusing and has so many redundant options and
    it's confusing to navigate. you should make it clearer and less redundant".
  - Battle: Arena and Background (as on the entry; an arena changed starts the set over on it, a background
    changed only redraws: "changing the background shouldn't change the entire arena either in trianing
    mode"), Music (the entry's pick, §2.29: the battle's music switched at once), Your HP and Dummy's HP
    (normal, refills), Custom gauge (normal, always full), Chip draw (shuffled, folder order)
  - Display: Input display
  - RESUME, RESTART, CHANGE BUILDS, QUIT TO MENU. Restart is also R or the pad's top button.

### Builds
- **The list:** NEW BUILD and IMPORT SAVE… over the builds; the selected build's overview beside it, with EDIT,
  DUPLICATE, RENAME and, apart, DELETE (asks once, Cancel focused).
- **Opened from Practice** (§2.28): its EDIT opens the build in the editor, whose Back (‹ PRACTICE in the header)
  goes straight back to Practice; its CREATE A BUILD opens the list, whose Back goes back to Practice too.
- **The build overview** (also Play's, and each player's panel in a match): the build's name, navi and version;
  Stats (HP, Attack, Rapid, Charge, Custom, Mega chips, Giga chips, Regular memory, each with what the NaviCust and
  patch cards added beside it in cyan); Abilities; Bugs, each by name and level, purple; then the folder's 30 chips
  in two columns of 26-tall rows: number, icon, name, REG or TAG1 or TAG2, element, code.
- **New build:** a name and the version (two choices, in §2.2's order); "Sets the Crosses and Giga chips. It can't
  be changed later." The folder starts as a new save's of that game. There are no random builds anywhere.
- **The editor's** header has the build's name, its version, and "✓ Ready to play" or how many problems; its tabs
  are NAVI, FOLDER, NAVICUST, PATCH CARDS and, in EXE4 and EXE5, AUTO BATTLE (the pad's shoulders or A and S switch
  them). With a navi other than MegaMan the last three stay in place, greyed, and keep their contents: the build's file
  keeps MegaMan's NaviCust, cards and auto battle data aside, outside the side it hands to a match (the rules
  refuse a NaviCust on a navi that compiles none), and puts them back when MegaMan is chosen again.
- **Navi:** the game's navis as a grid of cards; the chosen one's HP and element beside it (no charge shot: no game's content names a navi's, and the
  user chose to leave it out); "NaviCust
  and patch cards are MegaMan only." In EXE4 and EXE5, MegaMan's panel also has Light or Dark, a stepper, with the face the
  round starts him with.
- **Folder** (§2.11 to §2.14): a strip across the top describes the chip under the cursor from either pane (its
  art, name and code, the game's description, element, class, MB, attack, how many are in the folder). Below, left:
  the framed folder, its header holding Chips 30/30, Mega, Giga, Reg and Tag memory, then the overview's own 2 by
  15 block. Right: LIBRARY, a search field, Sort and its direction, then rows of one chip in one code (icon, name,
  a note, MB, attack, element, code), 15 showing. Enter on a slot goes to the library; Enter on a row puts it in
  the slot and returns to the next slot. R sets the Regular chip, T tags. A chip that can't be added stays, greyed,
  with the reason in red ("Gregar only", "Giga limit reached", "4 copies at most"); the rules are Tango's.
- **NaviCust** (§2.10, §2.26): a strip across the top describes the program under the cursor, in the game's words
  (the user: "you should show ncp descriptions"); the board framed at the left with a key under it; PROGRAMS, an
  open list, in the middle (each with its color, a cross if a plus part, and Placed, Placed ×2 where more than one
  copy is, In hand or Out of bounds); Effects and Bugs at the right, what the NaviCust does and the bugs it brings
  (not a patch card's). The command line is marked three ways: a line above and below its whole row that runs over
  placed pieces, an arrow at each end outside the grid, and a tint in its empty cells. EXE6's board has a white
  outline at its edge, a hatched ring outside it, and no cells at the corners. Arrows move the piece in hand, A and
  S turn it, C compresses it (in place, turned as it is: "rotating the ncp on compression is so confusing"), Enter (the
  pad's A) places it, Esc (the pad's B) puts it away into the list, as the game's B does: so a placed program is taken off by picking it
  up and putting it away, or with Del ("there seems to be no way to remove the ncp from the board once placed").
  Enter on a program's row takes a new copy in hand whether or not copies are placed, and Del there takes the
  last copy off (§2.26).
- **Patch cards** (§2.19, §2.20): the folder's shape. EXE5 and EXE6: the installed cards in order in the framed
  panel (number, name, MB, then every line the card writes, Parameter lines then Ability lines, bug lines purple),
  Cards and MB used in its header, and "Together", what they add up to, at its foot; the library's rows show the
  same lines, and the strip across the top lays the card under the cursor out in its two groups. EXE4: six slots
  named 0A to 0F, each with its card or Empty. A card belongs in one slot, which its library row names, and Enter
  puts it there, in place of the card that slot held. A two-part card says where its other half must be; a card
  shows its effect and the bugs it adds to or clears.
- **Auto battle** (EXE4, EXE5): the same shape, its places grouped under their lists in the framed panel. The
  library offers what the place being filled can hold (§2.27): a Mega place the Mega chips, the Giga place the Giga
  chips, the Program Advance place the Program Advances (which no folder holds), any other place the chips a
  folder can hold. A Program Advance has no picture and no element, and none is shown ("program advances don't
  have elements or chip art, so don't show those"). A chip of another class in one of those three lists (a build or save made by hand) is a
  problem of the build, shown as any other: its place's row in the error red, and the strip says why ("Cannon
  can't be in the Mega list").

### Replays
- **The list:** All, Online (and Training only if training is ever recorded) and a search by opponent; rows of the
  date, the opponent, the score, the length; beside it the selected replay with WATCH, EXPORT VIDEO, SHOW FILE and,
  apart, DELETE. No build names (§2.16).
- **The viewer:** the picture at three times over a transport: the time, a bar to click or drag with a mark where
  each round begins, the length; then to the round's start, a frame back, play or pause, a frame on, the next
  round; Speed (½, 1, 2, 4); View (P1, P2); EXPORT VIDEO. Space plays, Left and Right seek 5 seconds, comma and
  period step a frame, the brackets go a round.
- **Export:** a dialog of one size from start to finish: Size (even multiples of 240 by 160: 2, 4 and 6 times, 4
  first), View, Part, Save as, the progress bar and its line, CANCEL and EXPORT. The sizes are even because an MP4
  keeps its color at half resolution, and at an odd multiple every pixel edge fringes (measured: 25 dB at 3 times
  against 40 and 44 at 2 and 4). The export is an MP4 and nothing else: the user, "don't allow raw .nut export".
  (The NUT stream is how `nettai-video` hands the pictures and sound to ffmpeg, never a file the app saves.)

### In an online match
- The picture fills the window, with no header or footer.
- **Connection quality:** four bars and the round-trip time in a corner for the whole match, amber then red as the
  link worsens; "Reconnecting 12 s" while it is down.
- **Both builds:** holding Tab (Select on a pad) shows the player's build on their side and the opponent's on
  theirs, each the build overview without a name; letting go hides them. With Closed builds on, only the player's
  own.
- **The match menu** (Start or Esc; the game's own pause never appears): BACK TO MATCH, YOUR BUILD, THEIR BUILD
  (not with Closed builds), CONTROLS, SETTINGS, LEAVE MATCH. "The match keeps running", and it does.

### After a match
The mockups show no screen for this; it is decided here.
- **Online:** the result stays on the picture (who won and the rounds' score, in the game's own way), then both
  players are back in the lobby on the same link, neither ready, with the same settings. Either can change build,
  play again or leave. Another match with the same opponent needs no new invite code.
- **LEAVE MATCH** in the match menu ends the match for both players and puts both back in the lobby, as a
  match's end does. Leaving the opponent altogether is the lobby's LEAVE.
- **Practice** has no end: the practice menu's RESTART and QUIT TO MENU are the ways out of a set.
- **A replay** stops at its end with the transport still there.

### Settings
One scrolling page, a list at the left jumping to a group: General (Name, "Shown to your opponent."; Language),
Video (Picture size, Full screen, Battle text: sharp or original, Reduce motion), Audio (Music, Sound effects), Online (Input
delay, "In frames. Lower feels faster; higher rolls back less."), Controls (every action with its key and its pad
button, each rebindable), About.

### About
The app's name, its version and its license; Slint's "Made with Slint" widget; and MADE WITH, a list 440 wide of
the fonts the app bundles and every package built into it (each row its name, its version, and its license under
them), the chosen one's license files beside it in a panel, scrolling (§2.25). On a phone the list is the page and
a license opens as a sheet from the bottom.

### On a phone
One column; pickers and the chip library open as sheets from the bottom; everything tapped is at least 44 tall; no
key hints, and the cursor only with a pad or keyboard connected; in battle the pad is drawn under the picture and
START and MENU open the same menus. Held sideways, a phone uses the desktop layout.

## 7. Errors the player can see

A fixed enum, each with a catalog string; the raw error goes to the log only. The first set:

| When | What the player reads |
|---|---|
| the signaling server can't be reached | Can't reach the server. Check your connection. |
| no match has that invite code | No match with that code. |
| the match already has two players | That match is full. |
| the two apps differ | Your opponent has a different version of nettai. You both need the latest. |
| a build breaks a rule | This build can't be used: 2 Giga chips, limit 1. |
| the link dropped for good | Connection lost. The match has ended. |
| a replay is from another version | This replay is from a different version and can't be played. |
| the server needs a newer app | This version of nettai is too old for the server. Get the latest. |
| the opponent chose another game | Your opponent is on BN5. Pick the same game. |
| the opponent's build breaks a rule | Your opponent's build can't be used. |
| the opponent left the lobby or the match | Your opponent left. |
| the opponent never answered | No answer from your opponent. |
| a link couldn't be made | Couldn't connect to your opponent. |
| a direct address is malformed | That isn't an address. Use host:port. |
| the port to host on is taken | That port is already in use. |
| a save is another game's | Not a BN6 save. |
| a game's pack is there and can't be loaded | BN6's data is out of date. Extract it again. |
| a game has no pack, and no Legacy Collection is found | nettai needs Battle Network Legacy Collection Vol. 2. Install it from Steam, or choose its folder. |
| a game's pack couldn't be extracted | BN6's data couldn't be extracted. Details are in the log file. |
| anything else | Something went wrong. Details are in the log file. |

A replay that fails as it is opened, or whose engine stops as it plays, is its line's dialog over the replays list
(the viewer is left for it); the replay's box there shows its own still or nothing, never another battle's picture
(the main menu's reel was what the user saw under the error).

## 8. Behind the screens

1. **Gamepads** through `https://github.com/tangobattle/gamepad-facade` (a git dependency); gilrs goes.
2. **Invite codes** are issued by the signaling server (`signaling/`), which the client asks for one; the client
   never makes one up. Six characters of an alphabet without look-alikes, shown grouped in threes.
3. **In battle**, Start opens nettai's menu and the game's own pause never runs; L never runs away.
4. **Errors** as §7.
5. **Languages:** every string, errors included, through the catalogs (English and Japanese now, and the
   pseudo-locale).
6. **Title music** from each game's own data.
7. **New builds** start with the game's new-save folder; folder rules follow Tango's, including the Giga chips of
   one version. The build screens ask nettai-match's `build` for all of it, and it asks the game's rules: the
   versions in §2.2's order and a new build (`build::Builds`, from the game's rules/builds data), a build checked
   with its problems typed (`build::Checked`, `Wrong`: a screen words each kind, with its numbers, in its
   language), the folder editor's limits and each library row's reason (`folders::View`, read per row), the
   Regular and tag marks (`build::mark`, `folders::mark_problems`), the overview (`Checked::overview`: each stat
   with what was added, what gives the abilities, the bugs by name with a level or an amount, `build::Extent`,
   whoever brings them, the patch cards' MB) and a chip's
   row with its sort keys (`build::ChipInfo`, `build::sorted`). Light or Dark is a choice of the game's builds data
   (`Builds::choices_of`, `preset_of`, `take_preset`; `settle` for a build from a file or a save; none for a navi
   but MegaMan), with the face the round starts him with (`Checked::face`); EXE4's slots are `Builds::slots`
   (`Slots::own` and `name`, `held`, `put`). docs/design/rules-in-luau.md's last section says how the rules
   answer.
8. **Replays:** seeking, by keeping snapshots; export through `nettai-video` and ffmpeg; no build names in the file.
9. **The lobby's protocol** stops sending a build's name, version and navi; carries the rounds and Closed builds as
   shared settings; and holds the start until both sides have loaded.
10. **Random builds** are removed from Play, Practice and the lobby.
11. **Practice's** new options (§3) need new support in the battle engine.
12. **The screenshot pass** (the old `NETTAI_TOUR`) walks every screen in each language and size and is compared
    with the mockups.
13. **The build stays cheap.** "try not make the slint build really expensive, because the current nettai binary
    takes forever to build". The app's clean and incremental build times, and the time after touching one `.slint`
    file, are measured and reported with each step, and a choice that costs build time needs a reason from this
    document.
