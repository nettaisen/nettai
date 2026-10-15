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
    also have a toggle for patch cards on/off"). (Patch cards are a ruleset's since §2.59, allowed or not one by
    one; Closed builds stays the lobby's own setting.)
18. **Connection quality is shown during play.** Clicking the corner opens the link panel (2026-10-14: "you should
    allow clicking the ping indicator in pvp to bring up a tango-style link quality panel thing").
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
      playful"; and of the six in the mockup, M PLUS 1. Japanese display text was M PLUS 1 at 800 in every game
      (Corporate Logo since §2.51);
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
    About was a minor item of the main menu, beside Settings (alone under the line since §2.50; Settings'
    subscreen since §2.53): the app's version and license, Slint's "Made with Slint" widget (which Slint's
    royalty-free license asks for in an About screen reached from the top menu), and every font and package the
    app is built from with its license's texts (§6, docs/app.md §18).
26. **The NaviCust's bugs are its own, and a program is placed as often as wanted** (2026-10-13): "navicust
    shouldn't display patch card bugs. also you should allow a navicust part to be placed multiple times". The
    NaviCust tab's Bugs (on a phone, its Bugs line) list the bugs the NaviCust brings, the build's without its
    patch cards, as its Effects already were; a card's bugs stay on the Patch cards tab (its lines, Together) and
    in the build overview, which lists every bug. Taking a program from the list, by Enter or a click on its row
    (since §2.38 no drag), always takes a new copy in hand, however many copies are placed (before, it took a placed
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
    going back to Practice. (§6, docs/app.md §8 and §15.) (Since §2.50 Practice is entered from PLAY with a
    build, and CREATE A BUILD is gone. Since §2.57 there is no entry: the practice menu's EDIT opens the dummy's
    build.)
29. **The battle's music is the player's pick** (2026-10-13): "you should also allow for selectable battle themes
    (bn4 virus battle, bn4 boss battle, bn4 tournament battle, bn4 vs duo, bn5 virus battle, bn5 boss battle, bn5
    vs nebula gray, bn6 virus battle, bn6 boss battle, bn6 vs cybeast, bn6 crossover battle)". Any theme plays in
    any game's battle (a BN6 battle can play BN4's tournament battle theme); the first choice, "Game's own", is
    what the battle's game plays without one. A first draft put it in Settings' Audio group; the user: "settings
    is a weird place to put it". Asked where, between a shared setting of the match, each player's own beside the
    arena, and a row of the menu over the battle, they chose **"Beside the arena, each their own"**: a Music row
    in Practice's entry beside Arena and Background (a Your music in Play's lobby too, until the user: "don't allow
    bgm select in lobby"): each player picks and hears their own, nothing of it is sent. The pick is one,
    remembered between runs (since §2.40, kept for one visit to Practice), and Practice's sessions play it; a
    match, a replay and its video play the game's own ("why does the music setting carry over from practice mode into pvp mode? huh?", "for pvp it should use the
    game's default for now"), and the practice battle beside a waiting invite leaves the menus' music on ("in the
    practice mode while the lobby is waiting, don't switch the music"). The themes and their names are the games' content (each game's rules'
    `battle_themes`, named in its locales), shown after the game's name as the language says it ("BN4 Tournament
    Battle", "EXE4 トーナメントバトル"). (§6, docs/app.md §1, "The battle's music".) Then: "in the practice
    screen, if the bgm choice changes from game default you should start playing the bgm": Practice's entry plays
    the pick in place of the title music while it shows (once the row has stayed on it a moment, so a run of steps
    starts one song), the title music again for "Game's own" and elsewhere (since §2.40 "Game's own" plays the
    game's own battle music there); and "also you should be able to change the bgm in the practice menu": the practice menu's Battle rows have Music, the same pick, which switches the
    battle's music at once. (Since §2.57 there is no entry: the pick is the practice menu's, and the battle's music
    starts with the battle.)
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
    and it never shows the custom screen, a banner or a round's end. The HUD has no custom gauge. (Gone with the
    entry: §2.57.) (§6, docs/app.md §15.)
36. **A scrolling page is gone through before what is under it** (2026-10-13): "it's impossible to scroll the
    practice menu without selecting the button on the bottom first, it should scroll the entire length before
    moving to buttons underneath". And the practice beside a waiting invite has no Music row: "in the waiting for
    opponent screen, it really shouldn't let you change the music because it does nothing" (its music is the
    menus', §2.29).
37. **A click on the marked row removes, and a row is dragged to its place** (2026-10-13): "also actually clicking on
    an already selected chip or patch card or whatever should remove it, instead of right clicking. also you should
    be able to drag and drop to reorder". In the editor's own lists (the folder's entries, the patch cards and
    EXE4's slots, the auto battle places), a click on the row that is marked takes out what it holds, as Del does;
    a click on another marks it, as before; a right click did nothing there then (since §2.38 it removes). A double click removes nothing
    a single click wouldn't ("fast double click should remove" for a while, then "remove double clicking to
    remove"): on a row that isn't marked it only marks it. A row pressed and moved is dragged: it lifts under the pointer, the rows between slide to leave a gap
    where it would land, the list scrolls near its edges, and let go it moves there (off the list, it goes back). A
    folder's entry takes its Regular and tag marks with it; a patch card moves as `[` and `]` move it (EXE4's slots,
    each card's own, aren't dragged); an auto battle place's chip moves within its list, or trades with another
    list's place, refused with a shake where the rules hold that list to another class. The NaviCust keeps its right
    click, and a phone's tap still opens the library's sheet; on a phone a touch held still a moment drags a row, and
    one that moves at once scrolls. (§4, §6, docs/app.md §8.)
38. **Removing with the mouse: back where it came from, by a right click, and the NaviCust by clicks** (2026-10-13).
    Asked how to make removing with the mouse alike across the tabs, the user chose the option "drag it back to where
    it came from": "ok do 1". Things come out of the library (chips, patch cards, auto battle chips) or the NaviCust's
    program list, so taken back there they are removed. A row of the build's (a folder's entry, a patch card, an auto
    battle place's chip) dragged onto the library beside its list and let go there is taken out, as Del takes it;
    let go anywhere else off its list it goes back, nothing removed. While it is over the library it shows so,
    plainly and with no new button or badge (an on-strip REMOVE button was found ugly): its line turns the error
    color and it fades back, and the library fades under a wash and an edge in the error color with DROP TO REMOVE
    across its middle; off the library it looks as a drag does again. Then of the NaviCust: "dragging navicust piece
    is confusing because usually it's click to drop". Its board carries a piece by clicks alone: a click on a placed
    piece or a program's row takes it up, it follows the pointer with no button held, and a click puts it down; a
    press that moves before its release is the same click, and nothing follows the held button. A click on the
    program list with a piece in hand puts it away (the list shows the same quiet look while the pointer is over
    it, CLICK TO PUT AWAY, and the piece fades); a click anywhere else off the board puts it back where it was (a new
    copy goes away, having had no place). Where letting go off the board put a piece away before, only the list does
    now. And of the right click: "sure right click should remove everywhere". On the folder's entries, the patch
    cards (and EXE4's slots) and the auto battle places a right click takes out what the row holds, marked or not,
    the focus coming to it; on a library row that is in the build (a patch card that is in, the chip the place
    holds) it takes that out; the NaviCust's is as it was (it puts the piece in hand away, or takes off the piece
    under the pointer). On a phone the library is a sheet, not beside the list, so dragging onto it doesn't arise:
    the sheet's REMOVE stays the way. Keys, the pad and the libraries' clicks are as they were.
    Later: "remove double clicking to remove", and of a click on the marked row removing, "remove that too": a
    click only marks a row; a right click, Del or a drag onto the library takes its thing out. (§4, §6, docs/app.md §8.)
39. **From the library into the list, by a drag; EXE4's patch cards too** (2026-10-14): "you should allow drag and
    dropping from the library into a slot", and "you should also allow drag and dropping for exe4 patch cards". A
    library's row (a chip of a code, a patch card, an auto battle chip) pressed and moved is dragged: it stays where it
    is, a ghost of it follows the pointer, and over the build's list the row it would go in shows so, in the look of
    a row dragged within the list; let go there, it goes in, as a click or Enter would put it, refused as they are (the
    row shakes and the strip says why); anywhere else, nothing. A folder's chip takes the place of the entry's, a patch
    card goes in at that place (the others shifting on), an auto battle chip takes the place of the place's. A press
    that doesn't move is a click, as ever. EXE4's cards each belong in one slot, so a slot's card can't move to
    another: dragged onto the library it is taken out (its slot emptied), and a library's card let go anywhere over the
    slots goes in its own. On a phone the library is a sheet over the list, and its rows aren't dragged. (§6, docs/app.md
    §8.)

40. **Practice starts fresh, plays the battle's music through, and leaves by EXIT** (2026-10-14). The user: "going
    into practice should reset all the arena settings, there's no reason for it to remember the last settings";
    "the practice mode menu should also play the battle music and when it transitions it should play through the
    transition and not restart"; and "you should also get rid of "quit to menu" and "change builds" should just
    be "exit"". Arena, Background and Music are no longer remembered: each visit to Practice (from its entry being
    shown from elsewhere until a screen other than Practice and Builds is shown; Builds through EDIT, and CREATE A
    BUILD until §2.50, is part of it) starts on Random, Random and the game's own, and keeps what the player sets through the
    entry, its sessions and back. The entry plays the music its session will start (the game's own battle music,
    not the title music, or the theme picked), and the battle takes that song over where it is: no fade, no
    restart through the cards and the battle's start. The practice menu's buttons are RESUME, RESTART and EXIT
    (CHANGE BUILDS renamed, doing what it did: the entry again; 終了 in Japanese); QUIT TO MENU is gone. (Since
    §2.57 there is no entry: EXIT goes back to PLAY, and the battle's music starts with the battle.) (§6,
    docs/app.md §1, "The battle's music", and §15.)

41. **A build's version can be switched** (2026-10-14): "you should allow switching a save between game versions
    actually" (a save here is a build, made new or from a save). The version is switched on the Navi tab, near what it
    decides; the NEW BUILD dialog no longer says it can't be changed later. A switch makes what the version sets the
    new version's (EXE6's version and its MegaMan's Crosses; EXE4's and EXE5's souls and the Giga chips a save can
    have) and keeps everything else; what no longer fits, the other version's Giga chips in the folder, stays and is
    the build's problem as any rule's break is. Then: "you should show the crosses/souls as well to indicate which are
    available for the version". Asked "should those be under the right side under megaman if he's selected?", yes:
    the switch is in the Navi tab's panel of the build's navi, and under it, while MegaMan is the build's navi, the
    faces of that version's Crosses (EXE6) or souls (EXE4, EXE5), changing with it; the NEW BUILD dialog keeps its
    plain choice, with no faces. And: "well i think the version is still required for non-megaman because of folder
    restrictions right?": so the switch is there for every navi, the faces for MegaMan alone. (§6, docs/app.md §8.)
    Where the switch is went to §2.42.

42. **The version is switched by the header's tag** (2026-10-14). The user, of §2.41's stepper on the Navi tab:
    "the game version switcher is in a weird place", and given the choices, "header please": the editor's header's
    version tag (GREGAR, in the version's color) is the switch. A click or a tap takes the next version; it is a
    control in the focus order, which Left and Right step and Enter steps on, its text and color changing with the
    version; it keeps a tag's look, with a small arrow at each side lit on hover and focus, Back's hover look, and
    on a phone a touch 44 tall. While it is focused the footer says Switch version and Next version. The Navi tab's
    stepper is gone; its panel keeps the faces of the version's Crosses or souls under MegaMan, changing as the tag
    does. A switch is §2.41's, its problems shown as they were (the Folder tab marked, the header's count). Only
    the editor's tag is a switch: Play's (and the Builds list's, until §2.50) and Practice's stay plain. The NEW
    BUILD dialog's note says "It can be switched later in the editor." (§6, docs/app.md §8.) Its look went to §2.44.

43. **Practice's music plays on back at the entry; Background and Music are dropdowns** (2026-10-14). The user:
    "also exiting practice mode back into the practice menu should continue the music" (the practice menu here is
    Practice's entry, which EXIT returns to). The other way of §2.40's: EXIT, or the session's battle stopping,
    hands the battle's song to the entry where it is, with no restart and no fade, at the music's level; the
    battle's sound effects don't come with it, and the title music doesn't come in between. Where the battle's
    music had stopped (a round's end) or the result's winner or loser music plays, the entry's starts from its
    beginning as before. A Music changed in the practice menu during the battle is the entry's pick too, so the
    entry plays what the battle plays. Then: "background and music selectors should both be dropdowns like they
    are in the practice screen": on the entry and in the practice menu, Background and Music are the build rows'
    select (Enter, a click or a tap opens the list, open at the value chosen and scrolling where it is long, a
    sheet on a phone; choosing closes it; Left and Right still step it); Arena stays a pager, as asked before
    (its places are numbers). (Arena became a dropdown of pictures, and Background with it: §2.56. The music played
    on at the entry went with it: §2.57.) (§6, docs/app.md §1, "The battle's music", and §15.)

44. **The practice menu's controls are the entry's size; the version tag opens a list** (2026-10-14). Asked why
    the dropdowns differ (the entry's 40 tall in 56-tall framed rows, the practice menu's 32 tall in 44-tall bare
    rows, its steppers 32 too), the user chose "option 1": the menu uses the entry's sizes. Its steppers and its
    Background and Music dropdowns are 40 tall (36 on a phone, as the entry's), one column wide so they line up,
    its rows 52 tall to fit; it stays bare, with its group titles, and scrolls a little more. Of §2.42's tag: "i
    just don't like how the version changer looks it feels weird and out of place". It stays in the header and
    reads as a tag that opens a choice: the other headers' tag (the same pill, color, size and place after the
    title) with a small down caret inside it in place of the side arrows, the hover tint inside it, and the
    cursor's brackets with room around it; a click, a tap, Enter or the pad's A opens the game's versions as the
    selects' list under it, each its own colored tag, open at the build's; choosing one closes it and switches.
    Left and Right still step it without opening, as any select's; the footer says Choose version for Enter. Only
    the editor's tag has it. (§6, docs/app.md §8 and §15.)
46. **Team Colonel's tag is green, and BN6 shows its Beast Out first** (2026-10-14): "team colonel should be green
    instead of blue. you should also show the beast form as the first cross in bn6". EXE5's Team Colonel is drawn
    in green; under MegaMan, a BN6 version's faces start with its Beast Out (the builds rules' `shown_first`), then
    its Crosses.
45. **A game changed on Practice's entry keeps the same song playing** (2026-10-14). The user: "switching game in
    practice mode shouldn't restart the music if it's the same music". Where the entry's song after the change of
    game (the header's arrows, or L and R) is the same one, the same pack's same song (a theme picked: BN4 Virus
    Battle plays the same song in every game), it goes on where it is, with no restart and no crossfade, and the
    session's battle of the new game takes it over at START as before. Where it differs (Game's own: BN6's own,
    then BN5's), it crossfades as before. The pick stays what the player set across the change (§2.40: it is the
    visit's). (Gone with the entry: §2.57.) (docs/app.md §1, "The battle's music", and §15.)
47. **Practice's Arena lists each arena once** (2026-10-14): "don't show duplicate arenas in the practice mode
    selector, just show the first of any duplicates". The link battle stages that differ only in the order their
    actor lists place in (which side is updated first) are one arena, shown as the first of them; the grouping is
    read from the content, and Random still picks among all of them. Of the two pairs that differ only in their
    effects, by a bit nothing reads (EXE6's netbattle-95 and 96, EXE5's 93 and 94): "they should count as
    duplicates too", so the effects aren't compared. (docs/app.md §15.)
48. **Bass-Cross MegaMan is one card whose effects are the build's version's** (2026-10-14). The user: "you should
    only allow the bass-cross megaman patch card for the given version and it should switch automatically depending
    on game version, e.g. team protoman can't pick team colonel's. it looks like bn6 is also missing gregar's
    variant of bass-cross megaman." Then: "well i guess it's actually just the same card, but with different effects
    depending on version of the game". So it is: in each game one card, one number (EXE6's 117, EXE5's 111), one
    name and MB, with each version's ROM giving its own effects (EXE6's Gregar ROMs HP+200, SprArmr and B↓Sbustr
    where Falzar's have HP+20%, FlotShoe and B↓HelzR; EXE5's Team ProtoMan and Team Colonel the same pairs, and each
    its Hub Style). An effect only one version's card has says its `version` in the content, as a chip says its
    own; a build of either version holds the card with nothing wrong, and has the effects of its version. The
    library, the build's list and the strip show the build's version's lines, which change with the header's tag,
    the card staying where it is. EXE5's sides state their version (its team) as EXE6's do. And of how the content
    first said it (a card's effects as a list or a table of lists by version): "i don't like this typing, it's
    weird and confusing. can you make it more consistent?", so a card's effects are always one list, a version's
    own effect marked by its `version`. (docs/app.md §8.)
49. **A battle jacks in, and an online match ends on a result** (2026-10-14). The user: "can you make the ui more
    fun?". Of four ideas put to them they chose two: a jack-in set piece going into a battle, and a big result
    screen after a match.
    - *The jack-in* replaces the wipe from the two cards into the battle, for every way into one (online,
      practice, a replay), and is a set piece of the theme's (480 ms). It never delays the battle: the battle
      starts when it would have, and the set piece is drawn over its first frames; online both sides start in
      lockstep as before, and nothing of it touches the simulation. Streaks in the selected game's cursor color
      rush in toward the middle over the ground (the backdrop's rush, which runs again), a bright flash, and the
      battle shows; a telop stamps over it. Its first wording named the player's navi; the user changed it:
      "instead of JACK IN! {navi}, EXECUTE! it should be BATTLE ROUTINE, SET! EXECUTE!". So it is
      "BATTLE ROUTINE, SET! EXECUTE!" in English; in Japanese "バトルオペレーション、セット！イン！", the Japanese
      games' own call for the moment (the operator's "バトルオペレーション、セット！" and the navi's "イン！"),
      which was chosen here: the user gave the English alone. Then: "BATTLE ROUTINE, SET! EXECUTE! should
      appear in sequence rather than all at once", and then "battle routine, set! and execute! should be two
      separate telops". So they are two telops, one after the other in the same place, each its own band: the
      call's band draws out from the middle as it stamps in and draws back as it fades, then the answer's comes
      the same way, on a clock of their own (the theme's `telop`, 1200 ms: the call's from 2% to 47% of it, the
      answer's from 50% to its end), the answer over the battle's first frames. Then the user took the telops
      out: "get rid of the battle routine set execute telop it's too corny. just have like a more exciting
      transition or something". So there is no text: at the flash the ground splits along the middle, its two
      halves thrown apart (eased in, faster and faster) with lit edges at the split, and a diamond shockwave
      (the app's shapes are cut, never round), a second fainter one just behind it, bursts out from the middle
      over the battle as it shows, its line thinning and fading. That went too: "the diamond fade is too
      exciting, maybe just like an interesting swipe". So it is a swipe: the ground covers the battle and is
      taken off from the left in eight strips across the window, each a little after the one above (so the
      swipe's edge is a staircase), each strip's edge lit in the cursor's color with a soft band trailing it over
      the battle. Its sound: "it shouldn't be the transmission sound,
      it should be the encounter sound". It is the game's own encounter sound, the one every battle's start plays
      in the original as the screen breaks into it (song 0x78 in all three games: EXE6's `SOUND_BATTLE_START` of
      `gameState_8005BC8`, EXE5's 0x080054F6, EXE4's 0x08005490 and its two siblings), a sound effect of 0.7 s on
      an effects player, so it plays over the battle's music, which starts at once. The packs name it
      `encounter`, and each game's content names it as the role `sounds.encounter`; Rust names no sound. With
      Reduce motion on it is a short fade (the theme's `calm-fade`, 160 ms) of the ground, with no
      swipe.
    - *The result*: when an online match's set is over, the game's own end plays as before, and then nettai's
      result shows over the picture before the lobby. The winner's half of the window floods with their side's
      color, sweeping in from its edge (the player's own half, on the left, in player one's `#ff5d50` as
      everywhere else; the other's in player two's `#4a93ff`; on a phone the halves are the top and the bottom); a
      big WIN or LOSE (DRAW for a drawn set), from this player's side, stamps in, centered on their half, in the
      display face; then the rounds' score (2–1) and the match's total time (1:23.45) count up one after another
      (`Count`), in M PLUS 1 Code, each number in a box of one width. Each tick has the game's own menu cursor
      sound and the last number its confirm sound. Its facts are the frontend's: the round wins each side has
      (the battle's count, by side) and the frames the match's player ran; no state or content field is added for
      it. Nothing waits on it: CONTINUE, Enter or the pad's A (and Esc or B) settles it at once if it still moves,
      and pressed again goes on to the lobby; it goes on by itself after six seconds. Each player goes to the
      lobby when they go on, and neither waits on the other's result. Practice has no end and a replay stops at
      its end with its transport, so neither has a result: it is the online match's alone. With Reduce motion on
      it is a cut, everything at its end. (§4, §6, docs/app.md §1, §4 and §10.)
50. **PLAY holds the builds and Practice; the main menu is four big words** (2026-10-14). The user: "merge play
    and builds together". Then: "i feel like practice could also move into play, but then the main menu feels
    really sparse :(", and of the options put to them they chose PRACTICE in PLAY's row and a menu of fewer,
    larger words; then "maybe EXIT should be a large menu item as well".
    - *PLAY is one screen* of what Play and the Builds list were: NEW BUILD and IMPORT SAVE… over the player's
      builds of the game (Play's open list), the chosen build's overview beside them with EDIT, DUPLICATE, RENAME
      and, apart, DELETE (as the list had them), and under them Play's row. With no build NEW BUILD (the main action
      then) and IMPORT SAVE… are the way in, and the row's controls say in its reserved line why they can't be used
      ("Make a build first."). The editor opened from PLAY has ‹ PLAY as its Back, which comes back with the build
      still chosen. There is no Builds list any more, and BUILDS left the main menu. (Since §2.52: + NEW BUILD
      alone, a save started from its dialog, and DUPLICATE, RENAME and DELETE in the build's menu.)
    - *PRACTICE is in PLAY's row,* beside CREATE INVITE (which stays the main action): it opens Practice's entry
      with Your build the build chosen in PLAY, the entry otherwise as it was (Dummy's build, Arena, Background,
      Music, START PRACTICE, EDIT); the entry's Back is ‹ PLAY, coming back with the same build chosen. With no
      build it says why in the reserved line, as CREATE INVITE does. The practice menu, EXIT back to the entry and
      the music carried over are as they were. Practice's CREATE A BUILD (§2.28) is gone: the entry is entered with
      a build, and one that finds no build of the game goes back to PLAY. (Since §2.57 PRACTICE goes straight
      into a practice battle, and HOST is CREATE INVITE.)
    - *The main menu* is PLAY, REPLAYS, SETTINGS and EXIT, large, as a fighting game's title screen has them
      (SETTINGS came up from the small row, and EXIT with it), and About alone small under the line (since §2.53
      in Settings, and the line gone with it). The words are
      48 on rows 96 (36 and 72 before), the type's largest step, in the same 416 column, the four as tall as the
      reel beside them; on a phone 32 on rows 68.
    (§4, §6, docs/app.md §2, §4, §8 and §15.)
51. **Japanese display text in Corporate Logo** (2026-10-14). The user: "the japanese font should also be a
    little more stylistically like the english font". A sheet of the app's own strings (the menu's words, the
    telop's two calls, the labels at 11, a button) set in Chakra Petch, M PLUS 1 at 800 and five candidates under
    licenses that let it be bundled, and the user chose Corporate Logo ver3 Bold (LOGOTYPE.JP; SIL Open Font
    License 1.1): its kana are squared and cut straight, as Chakra Petch's letters are, its kanji are Source Han
    Sans's, and it covers both JIS levels. It replaces M PLUS 1 for every Japanese title, button, tab and label, in every game, at 700 (the one weight bundled, for the bold and the semibold alike); the tracking
    stays nearly none. Body text stays Murecho; codes and numbers M PLUS 1 Code.
52. **PLAY in three zones; the build's actions in a menu; one NEW BUILD** (2026-10-14). The user, of §2.50's
    screen: "the play menu feels so crowded now". Of the options put to them they chose two, "Build actions in a
    menu" and "One NEW BUILD button" (not "Online behind one button" nor "Smaller overview"), and added: "you
    shold rethink the layout to be more coherent i guess it's kind of crowded now".
    - *The build's actions in a menu.* Beside the overview only EDIT stays a button; DUPLICATE, RENAME and DELETE
      are in the build's menu, which ⋯ (a small button beside EDIT) opens as the selects' list opens (the
      cursor in it, Enter or a click choosing, Esc or B closing it; a phone's sheet), and which a right click on
      a build's row opens at the row. DELETE is last, under a line, in the error color, and asks once as before
      (Cancel focused); Del on the list still asks it.
    - *One NEW BUILD.* Over the list only + NEW BUILD. Its dialog has, under its buttons, FROM A SAVE FILE…: the
      system's file dialog, and a build made of the save as IMPORT SAVE… made one (named for its file, "Imported
      as Dark MegaMan" in the reserved line where that was so). IMPORT SAVE… is gone; a save dropped on the
      window is still made a build.
    - *Three zones, read in order*: the builds (+ NEW BUILD, the list), the chosen build (its overview, with EDIT
      and ⋯ in its header), what is done with it (PRACTICE; ONLINE, one group under a small label, CREATE
      INVITE its one main action, the code and JOIN, DIRECT CONNECT a text button), the reserved line under
      them. The online controls all stay on the screen. Tab goes zone by zone, and Right from the list goes to
      EDIT past the overview. Chosen here, the user saying no more: the two columns kept (the list 400 wide, the
      overview its 792 by 540), the band at their foot aligned to them, EDIT and ⋯ in the overview's header so
      that no row of buttons sits under it, CREATE INVITE in the normal look while there is no build (+ NEW BUILD
      is the one main action then), and the save's build named for its file whatever the dialog's name field
      says.
    (The band became three tiles, PRACTICE, HOST and JOIN, and then one row of controls, HOST, JOIN and DIRECT
    CONNECT, PRACTICE beside EDIT: §2.57.) (§6, docs/app.md §4.)
53. **About is in Settings** (2026-10-14). The user: "the about screen should be moved as a subscreen of the
    settings dialog and removed from the main menu".
    - *Settings' About group,* the last in the list at its left as before, has under Version and Your files a row
      of its own, Made with, whose SHOW LICENSES opens About in the page's place: its title ABOUT, its Back
      ‹ SETTINGS, and all it showed as it was (the app's name, its version and license, Slint's "Made with Slint",
      MADE WITH and the chosen one's license beside it scrolling, a phone's sheet), with its keys, its pad and its
      pointer as before. Back from its list comes back to the page, the focus on SHOW LICENSES; opened again, it
      is on the row it was on. Slint's widget is still reached from the top menu, by SETTINGS.
    - *The main menu* is PLAY, REPLAYS, SETTINGS and EXIT alone, the four big words with nothing under them and
      no line, centered in the body's height (on a phone under the reel, as before).
    - Chosen here, the user saying no more: a subscreen opened from a row, not About's parts laid in the group
      itself, as the list and the license beside it are a page of their own, each scrolling, that can't scroll
      inside the settings' page; and the row's label Made with, over the list it opens.
    (§6, docs/app.md §2, §9 and §18.)
54. **Practice keeps nothing between visits** (2026-10-14). Asked what is remembered between runs, the user: "yes
    for the practice options, they should all reset to default", and "you should also not remember the last
    dummy build either". The practice menu's rows (the dummy's Action and Level, both HP rows, the custom gauge,
    the chip draw, the input display) and the Dummy's build are kept for one visit, as Arena, Background and
    Music are (§2.40), and each visit starts on their defaults: Stands still, Normal, every option normal, the
    input display off, the dummy on the player's own build. Your build stays PLAY's chosen one. In the same
    exchange the build editor stopped reopening the tab last used: "don't remember the tab" (§6, Builds).
55. **The battle's HUD comes in, in the enhanced graphics** (2026-10-14). After the jack-in, the user: "can you do
    more exciting things when a battle starts like animate all the game ui in?"
    - *The game's own HUD*, drawn by the renderer, not the app: at each round's start its pieces come in one
      after another, each within 600 ms of its first showing: the HP box slides in from the left edge as the
      intro's fade clears, the emotion window after it; the HP numbers over the opponents pop in (a squash and
      stretch, then a hop); as the round's first chips are sent the custom gauge drops in from the top and a
      light sweeps its bar; as they arrive the chip's name slides in at the bottom left and the chip icons pop
      in. Motion only, in whole frame pixels, nothing of new color or art; the custom screen is the game's own.
      Every game, each with the pieces it has.
    - *The background glides.* Then the user: "can you also animate the battle background smoothly as well?" A
      background that drifts by parts of a pixel a frame (most areas' half a pixel across and a quarter up or
      down) is shown where the game's scroll counter is, at the window's resolution, not at the whole pixels
      it steps by every second or fourth frame; its tile and palette animations stay the game's.
    - *One mode.* Then the user: "you should only enable the ui improvements if sharp rendering is enabled. you
      should call it something other than sharp font rendering as well since it does more than that now".
      Settings' Video row is **Battle graphics: Enhanced / Original** (it was Battle text: Sharp / Original):
      Enhanced is the sharp text and the background's glide; Reduce motion takes the motion off and keeps the
      text.
      There is no row of its own for the motion.
    - *The HUD's intro is gone.* Looking at frame strips of it, the user: "the custom screen should slide
      smoothly too. why does the custom bar do a weird animation like that?" (the custom gauge's light sweeping
      its bar read as the empty gauge filling and vanishing), then "don't animate extra slide ins that weren't
      present in the original". The whole intro is removed: the HP box's and the emotion window's slides, the
      HP numbers' pops, the gauge's drop and sweep, the chip name's slide and the chip icons' pops. Enhanced
      smooths only motion the original itself has, over the same frames, and adds none; nothing comes in with
      an animation the game doesn't have.
    - *The custom screen keeps the game's slide.* Every game's window slides in and out 12 pixels a tick over 10
      ticks, the field 1.5 pixels a tick down with it. An eased curve over the same ticks was tried, then the
      user: "you should keep the original motion". A slide already in whole pixels every frame along its own
      path can't be drawn smoother at 60 frames a second without changing its curve, so Enhanced moves it as
      the game does. The same holds for anything else that moves so: Enhanced changes only what is drawn
      coarser than its own motion (a part of a pixel or a scale the frame rounds), never the curve or the
      timing.
    - *The gauge fills smoothly.* Then the user: "can you fill the custom gauge smoothly as well?" The game draws
      the fill to its counter's whole pixels (128 of its 0x4000 a pixel), a pixel every 4 ticks at the normal
      speed; Enhanced draws its end where the counter is, at the window's resolution. Its tiles, colors and
      full flashing stay the game's.
    - *Banners and telops at the window's resolution.* Then the user: "what about banners and telops and stuff?
      can they be animated more smoothly?" Their squash and stretch is the hardware's exact scale, which the
      240x160 frame shows by picking whole rows of their glyphs; Enhanced draws the same scale on the same
      ticks at the window's resolution, and so the custom screen's emblem as it spins. Their curves and timing
      stay the game's. Everything else that moves either moves in whole pixels along its own path, steps
      through pictures, or is a palette fade, and stays as the game draws it (docs/frontend.md §3, the
      survey's table).
    - *In-between places.* Then the user: "custom screen slide should be smoothed. shaking should also be
      smoothed if possible. smooth in-between positions to whole pixels". The window's own slide is 12 pixels
      a tick, whole; but the field and every sprite on it move with it 1.5 pixels a tick, which the game
      rounds to steps of 2, 1, 2, 1. Enhanced draws them where they truly are, at the window's resolution:
      the field half a pixel up on the odd ticks, each sprite at its true place, on the same path and ticks.
      The camera's shake has no in-between places: each tick's jitter is whole pixels by design (-2 to 1
      pixels each way at the common strength), so it stays the game's; nothing is made up between its
      frames. Then, told the navis and the attacks on the field round their in-between places too, the user:
      "you should also do". What moves by parts of a pixel (thrown bombs and rocks, arcs, a few shots and
      dashes) is drawn where it truly is; a semi-transparent or mosaic sprite stays at its whole pixels.
    - *Presentation alone.* The motion goes by the battle's own ticks, never the clock, and is in no state the
      simulation keeps: a rollback, a replay and a video draw it alike. Original draws every frame exactly as
      before; every verification path draws Original (docs/frontend.md §3, "Enhanced and original").
    (§4 "Motion", §6 Settings, docs/app.md §9.)
56. **Arena and Background open as grids of pictures; the practice menu's groups are renamed** (2026-10-14). The
    user: "arena selector should be like a dropdown that shows previews of all arenas. background dropdown should
    be similar and show preview of all backgrounds. the practice menu and the practice start screen should use the
    same widgets".
    - *One widget.* Arena and Background, on Practice's entry and in the practice menu, are the same select (the
      pager of §6 is gone): collapsed, the chosen one's picture small before its name (Random a question mark),
      the dropdown's caret; Left and Right still step it without opening. Enter, the pad's A, a click or a tap
      opens every choice as a grid, Random first: each cell its picture (120 by 56) over its name on two lines at
      most, five to a row under the select (over it where there is more room), up to five rows shown, the rest
      scrolling; arrows go a cell, the cursor brackets the cell, Enter or a click chooses, Esc or B closes. The
      chosen cell is outlined in the cursor's color over its tint. On a phone the grid is a sheet from the bottom,
      three to a row, the cells sharing its width. It opens as the other lists do (Reduce motion: at once).
      Music stays the plain dropdown it was, in both places.
    - *The pictures.* An arena's is its field, the panels alone with MegaMan on each side's starting panel (the
      user: "for arena thumbnails, you shouldn't render with the background, you should just render the arena and
      megaman only without his hp", then "you should show the opponent too as megaman on the other side. it should
      always be megaman as well, don't use the training dummy or saves"): no background, no HUD, nothing else of
      the round's but the objects the arena itself places as it starts (then the user: "objects don't seem to be
      rendered on stage previews"): the rocks, cubes, ice, boulders, statues, gears and flags its stage's list
      places, each on its panel as the round has it, drawn as a battle draws them; never anything from a build. A
      background's is its first frame. Both are the battle picture's band that holds the field and the tallest
      thing on it, 240 by 112 (rows 40 to 152: it was 240 by 96, which cut EXE4's flags and the cubes on the
      back row). The entry's preview, a running round, showed the objects already. The
      arenas have no names in the content, so a cell says its place, "12 / 47", as the select does; a background
      is named by its game and its area.
    - *Made as Practice opens.* The renderer draws them from the packs, a game's on a thread of its own as the
      entry shows (or a session starts), kept for the session; nothing is written to disk. A grid opened before
      they are all there shows each as it comes.
    - *The menu's groups.* Then the user: "the practice menu groupings are weird. you should rethink them. having
      an "arena" field under "arena" is so weird". The groups are DUMMY (Action, Level), STAGE (Arena, Background,
      Music) and OPTIONS (Your HP, Dummy's HP, Custom gauge, Chip draw, Input display); in Japanese ダミー,
      ステージ, オプション. No group is named for one of its rows, and none has one row (the input display's
      DISPLAY group is folded into OPTIONS). The entry has no groups.
    (Since §2.57 there is no entry: the grids are the practice menu's, and DUMMY starts with Dummy's build and its
    EDIT.) (§6 Practice, docs/app.md §15.)
57. **PLAY is a build and three ways to play it; PRACTICE goes straight into battle** (2026-10-14). The user, of
    §2.52's screen: "i feel like the practice button is in a weird place", then "hmm what about more holistically
    what do you think it should look like. the whole play/build/practice flow". Of the flow put to them (pick a
    build, then what to do with it): "sure do that. maybe there's too much explanatory text in your mockup but
    sure". And then: "when practice mode starts you should also bring up the menu immediately".
    - *PLAY.* The builds (+ NEW BUILD, the list) and the chosen build's overview with EDIT and ⋯ in its header
      stay as §2.52 had them; under them are three tiles of one size, the three ways to play the chosen build:
      PRACTICE, HOST and JOIN. HOST is what CREATE INVITE was (an invite, its code from the server, the wait).
      JOIN holds the code's field and its go button, and under its word "Direct connect…", a small link to the
      direct connection's dialog: DIRECT CONNECT is no longer a button of its own, and the ONLINE label is gone.
      The tiles carry their words alone (the mockup's lines under them, "Fight a dummy" and "Get a code to
      share", were too much); the reserved line stays under them, for no build or a bad code.
    - *HOST is the main action* (chosen here, the user saying no more): an online match is what PLAY is for, and
      HOST is the way that starts one by itself (JOIN needs a code someone sent, PRACTICE is alone); Enter on a
      build's row already went there. With no build, + NEW BUILD is the one main action, as before.
    - *PRACTICE goes straight into battle.* Practice's entry is gone: PRACTICE starts a practice battle at once,
      the build chosen in PLAY the player's, the dummy on the same build, Arena Random, Background Random, Music
      the game's own and every option of the menu at its default; and the practice menu is open over the battle
      as it starts, the battle held, the focus on RESUME, so the dummy and the stage can be set before
      fighting. RESUME starts the fight. The practice battle beside a waiting invite starts as before, with no
      menu, and RESTART doesn't open it.
    - *Everything is changed from the menu.* Its DUMMY group starts with Dummy's build, a dropdown of the game's
      builds that can be played, the player's own at first; a change starts the set over with the dummy on it,
      as an arena's does. Under it, EDIT opens the dummy's build in Builds' editor (‹ PRACTICE), and Back there
      comes back to the practice battle with the menu open (chosen here: the edit changed a build the battle was
      made of, so the battle starts again on the visit's choices with the build as the edit left it, as a change
      of the Dummy's build starts the set over). The menu's EXIT goes back to PLAY, the build still chosen.
    - *A visit* (§2.40, §2.54) is the time from PRACTICE until PLAY is shown again, the trip to the editor
      included.
    - *The music* starts with the battle as any battle's does: nothing plays the battle's music before it, and
      nothing carries it back to a screen after.
    - *What it supersedes:* Practice's entry and all of it: its rows, the two builds facing each other, START
      PRACTICE; §2.28's EDIT beside each build's select and Back to Practice; §2.29's entry playing the pick;
      §2.35's moving arena preview; §2.40's entry music played through into the battle, and EXIT back to the
      entry; §2.43's music played on back at the entry; §2.45's same song kept across a change of game on the
      entry; §2.50's PRACTICE opening the entry; §2.52's band (PRACTICE, the ONLINE group with CREATE INVITE, the
      code and JOIN, DIRECT CONNECT as text); and the entry's half of §2.56's one widget. The Arena and
      Background grids and their pictures (§2.56) stay, in the practice menu.
    - *The menu at the start holds the battle* (the user: "does the training menu open at the start not
      correctly pause the game?", and then "pause at start doesn't seem to stop custom hp drain"). Opened as the
      battle starts, the menu went where START can't pause the game, and there the battle stepped on with no
      buttons: through the round's opening, then waiting on the custom screen, where the NaviCust's custom HP
      bug went on draining. Now wherever the game's pause can't hold the battle (its custom screen, a dimming, a
      round's end) the menu holds it where it stands, nothing of it changing, its music going on; RESUME goes on
      from that tick. At the start the round's opening (the fade in, the navis' arrival, the banner: 2.3 s in
      EXE6) still plays under the menu, since at its first tick the field is black and no music plays, and the
      battle is held as its custom screen begins, before the screen comes in, the dummy and the stage in view
      (chosen here). The same after EDIT's return, a Dummy's build or an arena changed, and RESTART's start over
      under the menu. Mid-fight START was the game's pause as before (until the next: one hold everywhere): in
      EXE6 and EXE5 the NaviCust's HP bug waited through it; EXE4's drained through its pause, as the game's
      does. (docs/app.md §15.)
    - *One hold, everywhere.* The user: "pause during custom screen vs not custom screen is so janky now".
      Mid-fight the menu had the game's own pause, the background and the panels moving on; on the custom
      screen, in a dimming or at a round's end it held the battle, everything still. Of three ways put to them
      they chose: always hold the battle itself, and keep drawing the background's animation and scroll and the
      panels' animation on a clock of their own while the menu is up, so it looks like the game's pause on every
      screen. So wherever the menu opens (mid-fight, the custom screen, a dimming, a round's end, a round's
      opening) the battle is held where it stands: no tick, no button, no cue, nothing of it changing (EXE4's HP
      bug, which its own pause drains, waits too), its music going on. The scenery moves on a frame a tick on the
      picture's own clock, from where it was: the background's animation frames and its scroll (Enhanced's
      glide with it), and the panels' own animations (their palette cycles), which is what the game's pause
      moves; the navis and the effects stand, as the game's pause has them (the few effects its pause lets run, a charge's glow or an aura's shimmer, are the battle's own
      objects, and stand with it). RESUME goes on from the held tick and the scenery from where the hold left
      it: the frames it moved stay added, so nothing jumps back; they are the picture's alone, never the
      battle's, its snapshot's or its digest's, and a battle never held draws as before. START in the fight is
      the host's again, as in a match: it opens the menu, and the game's own pause never runs (no PAUSED banner
      to leave out). A battle started with the menu open still steps through its round's opening, as above, and
      is held as its custom screen begins. Practice isn't recorded, so no replay has a hold in it. An online
      match is untouched: its menu says "the match keeps running". (§6 Practice, docs/app.md §15.)
    - *The waiting practice's menu is the usual one.* The user: "practice menu in the matchmaking waiting screen
      should be the same as the usual practice menu". The practice battle beside a waiting invite (or a direct
      host's wait) has the practice menu's groups, rows and widgets, and its hold: DUMMY (Dummy's build, a
      dropdown of the game's builds that can be played, the player's own at first; Action; Level), STAGE (Arena,
      Background) and OPTIONS, all of them. Its way out is CANCEL INVITE (a direct host's STOP HOSTING) in EXIT's
      place under RESUME and RESTART, and there is no COPY INVITE CODE: the code was copied as the wait began, and
      the panel has COPY CODE (the user: "the stop hosting button should be in the place of the exit button on the
      regular practice menu. get rid of copy code"); on a phone STOP PRACTICE is the one row at the top, back to the
      panel the session covers. (The two rows at the top were taken out for the waiting panel's own buttons, "get rid
      of COPY INVITE CODE and CANCEL INVITE since it's already available on the host screen", and put back, "the
      practice menu should still have copy code and cancel match in the lobby waiting because otherwise there's no
      way to do it with keyboard": while the practice battle has the keys, the panel's buttons are the mouse's
      alone.) Two rows stay out (chosen here): Music, since its music is the menus' (§2.29, the user: "in
      the practice mode while the lobby is waiting, don't switch the music", and #36's "it really shouldn't let
      you change the music because it does nothing"), so the row would change nothing heard; and EDIT, since the
      editor is another screen and Play gives up its link as it is left (a link nobody polls reads as dropped),
      so it would cancel the invite or the direct host's listening. What the menu sets there lasts for that
      wait, a phone's STOP PRACTICE and its PRACTICE again included, and is forgotten when the wait ends; it
      never carries into a visit to Practice. (Superseding §6's "no Dummy's build or EDIT" beside a waiting
      invite and the session that "keeps nothing of the dummy's setup".) (§6 Play and Practice, docs/app.md §4
      and §15.)
    - *HOST and JOIN are the band; PRACTICE is the build's.* The user, of the three tiles: "the visual style is
      so ugly for practice/host/join", and then "well maybe for the play ui host/join should be the primary
      items and then practice like some kind of secondary thing". PRACTICE is a regular button in the chosen
      build's header, before EDIT and ⋯ (something done with the build), and on a phone in a row with them under
      the build's box. (The header was chosen over the band's left: it holds PRACTICE, EDIT and ⋯ beside the
      name and the version's tag in the pseudo-locale.)
    - *No tiles: one row of the app's controls.* The tiles were restyled as panels 56 tall (HOST's main action
      on its line, accent and word), and the user, of that: "the JOIN and HOST buttons are so big omg". The band
      is one row of normal controls at the controls' height (40), under the overview at its left edge: HOST, the
      main action's button as every main button is (its amber, its sheen), at its natural width; then, 24 on,
      JOIN's field and JOIN, a regular button with its word. No panel and no label over it. (HOST's ▾ and the
      field's 464 for an address went with the next two decisions.) The reserved line stays under the
      row. The list and the overview have the room back: they grow down to 16 over the row (564 tall), the
      overview no shorter than its 540. On a phone HOST is the column's width and 52 tall, as a phone's main
      buttons are, and the field and JOIN are a row under it.
    - *Direct connect was in HOST and JOIN* (superseded by the next). The user: "direct connect is in a weird
      place"; of the ways put to
      them they chose folding it into HOST and JOIN, with no control of its own. JOIN's field takes an invite's
      code or a direct host's address ("Code or address"): an address (a host or an IP and its port, an IPv6 one
      in brackets) joins that host directly, other text is a code; text with an address's marks that isn't one
      (no port) says so in the reserved line ("That isn't an address. Use host:port."). HOST is a split button:
      its ▾, a control of its own at the button's right edge, opens a short list, "Host with a code" (HOST's
      own) and "Host directly", which hosts on this machine's port and waits as an invite does, the panel
      showing this machine's address with COPY ADDRESS and "Send this address to your opponent.". The direct
      connection's dialog is gone, and "Direct connect…" with it. (The port stays what it was: 47474, never a
      setting.)
    - *Direct connect is separate again.* The user, of the field taking either: "can you make the direct connect
      separate again? the box with the combined code and address sucks". JOIN's field takes an invite's code
      alone again ("Invite code" in it while empty; what is typed kept as a code's six characters, capitals, in
      threes; a code no match has says so in the reserved line), 208 wide. HOST is a plain main button: its ▾
      and its list ("Host with a code", "Host directly") are gone. The direct connection's dialog is back as it
      was before: Host (this machine's address with COPY, and HOST; "Send this address to your opponent.") and
      Join (an address field and CONNECT; an address that isn't one says so under it). Its way in is DIRECT
      CONNECT, a regular button (40 tall, the regular kind, not text), at the row's right end, its right edge
      the overview's (chosen here: the place the user had not called weird, §2.52's, as a button and not
      text). On a phone it is a row of its own under the field and JOIN, the column's width as the phone's
      other secondary buttons (+ NEW BUILD) are. A direct host still waits in the waiting panel, its address
      shown with COPY ADDRESS and "Send this address to your opponent.".
    - *What these supersede:* the three tiles, PRACTICE's among them, and their restyle as panels; JOIN's link
      (§2.52's DIRECT CONNECT text button before it); HOST's ▾ and JOIN's field taking an address.
    (§6 Play and Practice, docs/app.md §1, §4 and §15.)
58. **Soul time is a stat** (2026-10-14). EXE5's overview listed a Soul Unison's turns under Abilities as
    "Soul Time 5", and the user: "instead of "soul time 5" shouldn't it be "soul time +2"?", then "shouldn't soul
    time go into stats?". It is a row of Stats after Regular memory, Soul time (ソウルタイム), for an EXE5 build
    whose navi has souls: its value is the turns a Soul Unison lasts with the build, as the battle gives them
    (base 3, at most 9, at least 1), and beside it in cyan the build's change, signed ("+2", "−1"), as every stat
    has what was added; nothing where it is 0. SoulT+1 with MagicMan's card reads "Soul time 6 +3". The Abilities
    line is gone. The row is the game's: its rules' overview gives the stats a game has beyond the common eight,
    each by its name in the game's strings, and the app shows whatever rows they give. Chosen here, the user
    saying no more: the wide sheet has no 22 to spare under Regular memory, so the row takes it from the lines
    under it (the abilities and bugs a line fewer, nine to eight); the narrow sheet's two columns of four have no
    room for a ninth, so it is a row of its own under them at the left, the sheet 22 taller (a third column would
    leave a stat's label 40% longer no room); a phone's sheet has it as its ninth row. (§6 Builds, docs/app.md §8.)
59. **Rulesets in place of Patch cards** (2026-10-15). The user: "instead of just closed setup and
    patch cards on/off there should be an entire ruleset configuration thing like: - closed setups - allowed
    navicust pieces - allowed patch cards - allowed chips - allowed navis - allowed sets of souls/crosses (should be
    able to represent game)". Asked how, they chose: for souls and Crosses both a version's set as a quick choice
    (EXE6: Gregar's Crosses, Falzar's, or both; EXE5: Team ProtoMan's souls, Team Colonel's, or both; a version's
    set is exactly what that cartridge has) and a toggle for each soul or Cross under them; named rulesets the
    host saves and picks in the lobby, beside built-in presets of each game ("Standard" allowing everything), the
    lobby showing the ruleset chosen and the joiner seeing it read-only; and the build editor marking what a
    ruleset bans, the player picking a ruleset there (remembered by game) so that they can build to it, and the
    lobby's line saying what a build breaks under the agreed ruleset ("banned chip: X" style). What follows is
    chosen here, where they said no more.
    - *A ruleset* is what may be played: a name, Versions (below) and, for each of the game's ruleset
      categories, the items it doesn't allow. Closed builds isn't a ruleset's (the user, as rulesets were built:
      "i think closed setup should not be part of the ruleset actually"): it stays the lobby's own setting, as it
      was, and no ruleset file or page has it. A category is one of what
      a build is made of that the game lets a ruleset hold back: Navis (EXE5, EXE6), Chips, NaviCust programs,
      Patch cards, and Crosses (EXE6) or Souls (EXE4, EXE5). The game's content says which categories it has
      and what it offers ready-made; nothing in the app names one (docs/design/rules-in-luau.md, "Rulesets").
      *Versions* is a ruleset's setting too (docs/design/versions.md §2.3): one per build (each build is one
      cartridge's: what it holds of a version's own things, its souls or Crosses and its Giga chips, all one
      version's) or mixed (the allowlists alone say what may be played). The versions are the game's *groups*
      (docs/design/versions.md §2.2), read from what a build holds, never from its version: a category's items that
      are a version's own have that version as a quick choice on the page (Gregar, Falzar, All), and a build of
      two versions' things under one per build is a problem ("Mixes Gregar and Falzar: TenguCross, Bass").
    - *Presets* are each game's, in its order. Standard, the cartridges' own rules (the user, of rulesets once the
      versions were decided: "you should have a standard ruleset for each game that enforces version-specific
      stuff"): everything allowed, Versions one per build, both versions allowed; it is a new lobby's,
      and a player's until they pick another. Then Mixed (everything allowed, Versions mixed), one for each version
      (named for it: "Gregar" allows Gregar's own things alone, one per build), and No patch cards (Standard with no
      patch card allowed). **Since renamed, by the community's names** (2026-10-15; the user: "no patch card rules is
      known as "senior class". patch card rules is known as "masters class". mixed rules is known as "unrestricted".
      get rid of version only rules"): Masters class (was Standard; the first), Senior class (was No patch cards),
      Unrestricted (was Mixed); the presets of one version alone are gone (the page's quick choices still make one).
      A preset can't be changed; DUPLICATE makes the player's own copy of it. *The player's
      own* are named by them and kept as files, one per ruleset, by game (docs/app.md §8), as builds are.
    - *What a ruleset does to a build:* a build that holds something its ruleset doesn't allow can't be played
      under it. Each thing is a problem of the build as a rule's break is, one for each item ("Bass isn't
      allowed", "TenguCross isn't allowed"): the lobby's line says the first and READY is refused, the build
      editor's header counts them, and the item is marked where the editor shows it. Nothing is taken out of a
      build to play it. **So Patch cards off is gone, and it worked otherwise**: with it off each build was played
      without its cards; now a ruleset that allows no patch cards (Senior class, once No patch cards) refuses a build that
      holds any, and the player takes them out, or brings another build. A ruleset that allows some cards and not
      others reads the same way, as the user's "allowed patch cards" asks.
    - *A build's souls and Crosses* were its version's, all of them, until docs/design/versions.md §2.1 was
      built: now the Navi tab's selector chooses them one by one (§2.60), and a ruleset that holds back one soul or
      Cross refuses only a build that holds it.
    - *The lobby's row* is First to, Ruleset, Closed builds, Input delay and Ping: Ruleset takes Patch cards'
      place. The host's is a select of the game's presets and then the player's own by name (Left and Right step
      it, Enter opens it as a list), with EDIT after it, which opens the ruleset's page. The joiner's shows the
      host's ruleset by its name, can't be stepped or opened, and has VIEW after it, which opens the page
      read-only. A ruleset chosen, or the one chosen changed on its page, clears both ready marks as any shared
      setting does. Each player checks their own build against it, and their peer checks the other's. A host's
      lobby starts on the ruleset that player plays by (below).
    - *The ruleset a player plays by* is one choice for each game, remembered: the one the lobby starts on when
      they host, the one the build editor marks builds against, and the one Play's list and Practice check builds
      under (a build it doesn't allow is "2 problems" there, and PRACTICE and HOST say its first; in a lobby the
      list CHANGE BUILD opens is checked under the host's; JOIN asks only the game's rules, as the lobby it joins
      checks the build under the host's). The build editor's is a select at the tab
      bar's right end ("Ruleset", then the select, and EDIT), on every tab; a phone has it on the Navi tab, a row
      between the navi's panel and the grid. Picked in either place it is the other's too (a joiner's lobby shows the host's and changes nothing).
      The game's first preset (Standard) until one is picked, or when the one picked is deleted.
    - *What the editor marks:* a navi's card it doesn't allow (a red line under its name, "Not allowed"); a chip
      in the folder's library (its row's reason, as another version's Giga chip's: "Not allowed", in the error
      red, and it can't be added) and a folder's entry of one (the entry's red, as any entry's problem); a NaviCust
      program's row in PROGRAMS (Not allowed, in the error red where Placed is); a patch card's library row and an
      installed card (in the error red); a soul's or Cross's face on the Navi tab (a red ring). The header's count has them.
    - *The ruleset's page* covers the screen it was opened from (the lobby, or the build editor), which goes on
      under it (the lobby's link with it); its Back (‹ LOBBY, ‹ EDITOR) or Esc comes back to the control it was
      opened from. Header: the ruleset's name, and PRESET as its tag for a preset. Body, in its units: a row of
      the Name field (360; disabled for a preset, and for a ruleset viewed), Versions (ONE PER BUILD, MIXED) and, at the right, DUPLICATE and DELETE (the player's own alone; it asks once). Under it from 64 the categories at the
      left (an open list 300 wide, rows 52: the category's name, and how much it allows, "All allowed", "None
      allowed" or "12 of 380 allowed"), and the chosen one's items at the right: a head of ALLOW ALL, ALLOW
      NONE, for a category with a version's own items the quick choices (a segmented choice of each version
      and All, the one lit whose own items alone are allowed of the versions'), and a search field; then the items as the build editor's
      lists show them (chips with their icon in the library's order; navis, souls and Crosses with their faces;
      programs and cards by name), each row a check mark, allowed or not, toggled by Enter, a click or a tap.
      Each change is saved at once, as a build's are. A preset's page, or a ruleset viewed, changes nothing (its
      rows' marks still show) and says so in the reserved line ("A preset can't be changed. Duplicate it to make
      your own."), but DUPLICATE works there: a viewed ruleset duplicated is the joiner's own copy. On a phone
      one column: the name, Versions, the buttons, the category as a select, then its head and items.
    (§6 Play and Builds; docs/app.md §4 and §8; docs/design/rules-in-luau.md, "Rulesets".)

60. **A build has no version; the editor chooses its forms** (2026-10-15; docs/design/versions.md, built). The
    user, of the versions' design: "1. bass cross variant on the card 2. drop them, ignore backward compatibility 3.
    no label 4. one per build", then "the builds don't have version labels either, they're kind of pointless since
    they correspond to ruleset rules", and "you probably need to have some kind of soul and cross and cybeast
    selector in the editor now right?".
    - *No version label anywhere:* the editor's header's tag and its list of versions (§2.42, §2.44) are gone, and
      so are the tags of the Builds list's and Play's rows, the build sheet, Play's card and the versus cards. A
      build that fits one version is the ruleset's to say (Versions one per build: §2.59's "Mixes Gregar and
      Falzar: …"). The header's tag after a title stays a page's own (a ruleset's PRESET).
    - *The Navi tab's selector*, under the navi's facts where §2.41's faces were: each fact of forms the build
      chooses, by the game's rulesets' name for it and how many it has of its room ("CROSSES 5/5"), its faces
      twice their size in the battle's order (rows since §2.61), both versions' (EXE6's Cybeast at the head, Gregar's then Falzar's,
      one always chosen, for every navi; then the ten Crosses, up to the Cross window's five; EXE4's and EXE5's
      twelve souls, up to the soul button's twelve: the lists are the game's own navi's alone). The build's are
      lit with a line in the cursor's color under them, the others dim; one the ruleset the player plays by
      doesn't allow has a ring in the error red. Enter, a click or a tap chooses one (the Cybeast), or puts a
      soul or Cross in (in the battle's order, where the list has room) or takes it out.
    - *Bass-Cross MegaMan's variant is the card's:* the Patch cards library lists the card once for each variant
      ("Bass-Cross MegaMan (Gregar)", "(Falzar)"; EXE5's teams'), each with its variant's lines, as the NaviCust's
      list has a program once for each color. Putting one in installs the card of that variant; putting the other
      in while it is chooses the other in its place (the card is in once). The installed card's row and the
      strip show its variant's lines. A ruleset's page lists the card's variants as items of their own, so a
      ruleset can hold one back.
    - *NEW BUILD* still offers the game's versions as tags in their colors: what the build starts from, its note
      the game's words ("Starts with the version's Crosses and Cybeast.") and "They can be changed later in the
      editor."
    - *Old builds are dropped:* a build file that states a version is no build now; the list shows it as one
      that can't be read.
    (§6 Builds; docs/app.md §8.) §2.41, §2.42, §2.44 and §2.46 (the version's switch and faces) and §2.48's
    effects by the build's version are gone with it.

61. **The editor's versions, reviewed** (2026-10-15). The user, of the forms' selector (§2.60): "instead of
    "Cybeasts" in bn6 it should really be just "Beast form"", "you should indicate in the editor version
    exclusives", "there's missing translation strings for souls and beast forms", "the rows in the editor are too
    tall, they should match the height of the regular editor", "also the version selector in new build is really
    tall for no reason" and "also if there's a problem with e.g. crosses or souls there's nowhere in the ui that
    shows it".
    - *Beast form:* EXE6's fact of the Cybeast is shown as "Beast form" (its heading, the ruleset category, the new
      build's note), the Japanese 獣化, the games' word; the field stays `cybeast`. The forms are named by the
      ROMs' names for them: GBeast and FBeast (Gビースト, Fビースト), EXE4's souls their chips' (AquaSoul,
      アクアソウル), EXE6's Crosses in Japanese the game's (キラークロス, アクアクロス).
    - *Rows:* the selector's forms are rows of the editor's lists, 26 tall (a phone's 40, the lists' own), a check
      mark, the face at its size, the name and the version's mark; the navi's facts sit beside its face to give
      them the room.
    - *Version marks:* everything one version's own has its version's tag after its name or at its row's end,
      small, in the version's color: a Giga chip's entry and library row, a form's row, a card's variant (the row
      then shows the card's own name).
    - *Problems:* a fact's problem is said by its heading in the error red, and its rows are red; a mix of versions
      marks each tab that holds its things.
    - *NEW BUILD's versions* are a control's height (40), as the dialogs' other controls.
    - *Grouped by version* (the user: "you should also group souls/crosses by version i think"): each fact's rows
      are its forms by version, in the groups' order (the battle's order within each), each version's under a
      small head of its name in its color; forms of no version last, under a head's room with no name. The focus
      goes through them as drawn.
    (§6 Builds; docs/app.md §8.)

62. **The overview shows the build's forms** (2026-10-15). The user: "is there a way to fit selected crosses and
    souls and beast form into the stats view for a build?", then "yes add the forms row". Under the header (the
    face, the name, the navi) a row of the forms the build has, their faces at their size (32 by 16) 6 apart, each
    with its version's mark under it (a bar 2 tall in the version's color, so a build that mixes them shows it):
    EXE6's Beast form first, then its Crosses; EXE4's and EXE5's souls; in the build's own order (the order the
    battle lists them in: the Cross window's, the soul button's, which the editor keeps grouped by version). A
    navi that doesn't change form has no row. The wide sheet has it over the stats' column, six to a row, and
    takes its room as Soul time did (§2.58): each row a line fewer of the abilities and bugs. The narrow sheet
    has it over its stats, six to a row, and is taller by it; a phone's as many to a row as its width takes.
    (§6 Builds; docs/app.md §8.)

63. **The Navi tab's panel, reviewed** (2026-10-15). The user: "the light/dark switcher should be above the souls.
    the souls/crosses layout is also very cramped. for other navis, the spacing for the description text is not
    important since stuff has moved around. the light/dark switcher also takes up too much space."
    - *Choices first, and small:* under the navi's facts come the build's choices, then the selector. Each choice
      is a segmented control of its presets in capitals (LIGHT | DARK), as the app's two-way one (Versions' ONE PER
      BUILD | MIXED) is drawn: each segment as wide as its word (44 at least), the build's tinted and underlined in
      the cursor's color, the control as tall as the editor's rows (26; a phone's 44, to be tapped) and as wide as
      its segments, where the stepper was the panel's width and 36 tall. Left and Right step it (none past its
      ends), Enter takes the next (of two, the other), a click or a tap on a segment takes its preset. A navi
      with none of a choice has nothing in its place.
    - *Room in the selector:* the panel's padding stays 16 all round (the editor's strips' and the NaviCust's
      effects'); the choices, each fact of forms and the note are 20 apart; each version's head has 8 over it (from
      its fact's heading, or the group before) and 4 under it. The rows stay the lists' (`Theme.list-row`, 2
      apart). Where each of two versions has one form (EXE6's Beast form: GBeast, FBeast), the two groups are side
      by side, a column each under its head: one row, which gives EXE6's Crosses the room the gaps take (stacked,
      EXE6's selector ran past the panel's foot).
    - *The note* ("NaviCust and patch cards are MegaMan only.") follows whatever is over it, 20 after it, as tall
      as its words: nothing is reserved for it, nor for a choice the navi doesn't have. (Read so: with another
      navi the panel kept an empty stepper's 44 and the note's box of 60 from the old layout, so the note sat apart
      under nothing; now it sits right under the navi's facts.)
    (§6 Builds.)

64. **The editor's rows: names first, details on hover, a program once** (2026-10-15). The user, of the Patch cards
    tab: "the name isn't very visible on the patch cards view. i think you should get rid of the effects list inline
    with the list in the patch cards view. then also show chip/patch card/ncp details on list item hover"; then
    "deduppe navicust program parts in the list and then have you select by color".
    - *A patch card's row* is the folder's: its number, its name (12, a phone's 14), its version's mark, its MB (EXE4:
      its slot's box before it, or its slot at the library row's end), the name in the error red where the rules
      have something against it; a list row's height (26, a phone's 40), in the build's list and the library alike.
      A card's lines are the strip's alone: its card screen's two groups, bugs purple.
    - *Hover drives the strip* (chosen over a card floating by the row: every tab already has a strip that describes
      the focused row, so the hover reuses it whole, nothing new to draw or to keep from covering the rows beside it,
      and a flat panel that changes in place is the HUD's way): while the pointer is on a row of the folder, the
      chip library, the NaviCust's list, the patch cards, their library, or the auto battle's places and library, the
      strip describes that row; the keyboard's focus and the chosen row's mark stay where they were. The pointer gone
      off the rows, or a key that moves the focus, and the strip is the focused row's again. A refusal's reason is the
      focused row's: the strip keeps it, and the row a drag was let go over isn't a hover until the pointer goes on.
      With a piece in hand the NaviCust's strip stays on it. A phone has no hover: a tap is as it was.
    - *What the strip shows* (the details asked for): a chip, its picture, name, code, description, element, class,
      MB, attack and how many the folder holds (as it was; a row is one chip in one code, so its code is that one); a
      program, its shape and, where it compresses into another, its compressed shape, each a small grid in its color
      over its word (SHAPE, COMPRESSED; 形, 圧縮時), its name in its color, the game's description, and its colors
      (Colors, カラー) with the chosen one outlined; a card, its number, name, MB or slot, and its lines, bugs purple.
      No game's data has a description of a patch card (the cards' screens list their effects), so a card has none.
    - *A program once:* the NaviCust's list has each program once, in its color chosen; where it has more than one
      color, its row shows them as small boxes before its state, the chosen one outlined (the footer: ←→ Color). Left
      and Right on the row (the pad's too) choose the one before or after, and past the first or the last the focus
      moves on as ever; a click or a tap on a box chooses it. Enter (a click on the row) takes that color's part. The
      choice starts at the color of the copy the build has placed first, else the program's first. The row's state
      (Placed ×2, In hand, Out of bounds), Del's last copy and the strip are the chosen color's part's. A ruleset holds
      a program back whole (its items are programs, not their colors): each of its boxes has the error red's ring, and
      the row reads Not allowed.
    (§6 Builds; docs/app.md §13.)

65. **The editor's wording and places, reviewed** (2026-10-15). The user, of the build editor: "selecting the color
    in the ncp editor should also pick up the ncp from the palette. the error message should go to the left of the
    color", "isntead of shape/compressed it should be compressed/uncompressed (in that order)", "error messages for
    beast form/crosses ends up shifting layout" and "versions: one per build is weird wording, you should make it
    more understandable"; then "instead of "beast form" it should say "beast out"".
    - *A color's box takes its part:* a click or a tap on one of a program's color boxes chooses that color and
      takes its part in hand, as Enter on the row (or a click on it) does: the piece goes onto the board under the
      cursor. Left and Right still choose the color alone (chosen so): taking a part puts the cursor on the board,
      so a key that took one would leave no way to step on to the next color, and Enter is a key away.
    - *The row's state before its colors:* Placed, Placed ×2, In hand, Not allowed or Out of bounds comes before the
      color boxes, which are at the row's right edge. The state's column ends where the list's most colors begin
      (EXE6's three boxes), so the states line up whatever each row's colors; a row's name runs up to its state.
    - *COMPRESSED, then UNCOMPRESSED:* the strip shows a program that compresses as its compressed shape first,
      over COMPRESSED (圧縮), then its whole shape, over UNCOMPRESSED (非圧縮; no game's text names either, and the
      app's Compress is 圧縮 already). A program of one shape shows that one, first, with no word under it: it is
      neither ("there's only one form of it"). The shapes' boxes are 120 wide, which UNCOMPRESSED needs.
    - *The forms' problems move nothing:* a fact's heading and the rows a problem is about are still in the error
      red, and the sentences are no longer a wrapped line under the heading (it pushed the rows down). They are in
      the note's place under the selector, which for a navi that chooses forms is two lines tall whether it says
      anything or not, and while the forms have problems says them instead of the note, whole, each once (a mix of
      versions is each of its facts'), a line each; past two, the first and "+n more". Chosen over a place of its own
      under the note: EXE6's selector (Beast Out and ten Crosses) leaves the panel room for two lines under it at a
      desktop's 800, not for the note and two more; over shorter words by the heading or the whole on hover: EXE5's
      "Mixes Team ProtoMan and Team Colonel: ColSoul" doesn't fit by its heading, and a keyboard or a phone has no
      hover. The note is said again once the problems are gone. (§2.66 moved them, and then back: the place says, on
      one line, the problems of the form under the cursor or the pointer, else of the navi; it is as tall as the note
      and a line at least.)
    - *Version exclusives:* the ruleset's setting is VERSION EXCLUSIVES, ONE VERSION | MIX VERSIONS (バージョン限定:
      1バージョンのみ | 混在可), where it was VERSIONS, ONE PER BUILD | MIXED: whether a build may hold both versions'
      own things (Gregar's and Falzar's Crosses and Giga chips) or keeps to one version's, as a cartridge does. Chosen
      over a MIX VERSIONS toggle of OFF and ON: both segments say what they do. On a phone the word is over its
      control, which its width wouldn't hold beside it (the rows under it 24 lower). The files keep `versions =
      "one_per_build"` or `"mixed"`; the lobby shows the ruleset by name alone, and the problem's sentence ("Mixes
      Gregar and Falzar", §2.66) names no setting, so neither changes.
    - *Beast Out:* EXE6's fact of the Beast form is shown as "Beast Out" (BEAST OUT on the selector, the ruleset's
      category, NEW BUILD's note "Starts with the version's Crosses and Beast Out."), the game's name for it; the
      Japanese stays 獣化, the Japanese game's, and the field stays `cybeast`.
    (§6 Builds; docs/app.md §8, §13; docs/design/versions.md §2.3.)

66. **A build's problems, one system, said where they are** (2026-10-15). The user, of the build editor: "why isn't
    the build error for a pane shown in the same location on all tabs? e.g. top right like folder", then "the build
    problem stuff is so not unified". That made them one system said in the header (below, *What was tried*); of that,
    "i feel like errors should be inline? also the drop down is so ugly", then "and maybe ruleset errors should be
    displayed with the ruleset and not like that", and of the header's "✓ Ready to play" and "n problems" left after
    it, "do you really need "ready to play" if the problems are indicated elsewhere already?". The sentences were then
    put at the end of the rows they were about, and of that: "putting the error inline in the list makes it
    unreadable". Earlier, of the Navi tab: "if there's a problem with e.g. crosses or souls there's nowhere in the ui
    that shows it" and "error messages for beast form/crosses ends up shifting layout". Of the panes' headers and the
    ruleset's line that followed (*What was tried*): "you should show the error for an item on the top right of the
    info box for everywhere it is relevant, including ruleset violations. you should get rid of the ruleset violation
    message at the top and rely on the violation issue per info box. but it doesn't really have a good place to put
    errors for crosses/souls/beast outs", then "the message on the top right of the folder pane is redundant too if
    the error is going to be moved into the info box", and "panel header doesn't need to show ruleset violation". Of
    the Navi tab's strip and a mix of versions that followed (*What was tried*): "i don't like having an info thing on
    the navi view, it's redundant. also the error on the patch cards view runs int othe existing text. also guessing
    which versio nthe user intends is confusing, if mixing and matching occurs and is not permitted you should just
    mark everything that's mixed and matched as an error".
    - *Where each is said now:*

      | Place | What it shows | How |
      |---|---|---|
      | Editor header | nothing of the build's problems | the build's name and the game alone |
      | Tab bar | a tab with a problem (`Editor::tabs_of`) | the problem's mark, a red square 5 by 5, at its word's upper right |
      | By the ruleset's select | nothing | RULESET, its select and EDIT alone |
      | Strips (every tab's but the Navi tab's, which has none) | at the top right, where nothing was refused, what the build's problems have against the item described, the ruleset's among them: the item's own (a chip's code, a program over another, a card held back, a Giga chip or a card variant of a mix of versions); those of a whole part it counts in (an empty folder entry: the folder's size; a chip: its copies, its class's limit, the dark chips', the tag memory of a tag chip; a card: the cards' MB); then its part's that are about no item and no count, in every item of its pane (a fact a build made by hand states wrong: the fallback) | `Refusal`'s `problem` (`ProblemNote`): the mark and the words, the first and " · +n more"; a refusal ("Needs 70 MB") in its place, red with no mark |
      | Navi tab's fault line | a line under the navi's facts (HP, element), kept whether it says anything or not, in the same place whatever the selector holds (§2.70; a phone's too): of the form under the cursor or the pointer where it has problems, else of the navi where it has, else the note | a form's own problems (the ruleset holds it back, a mix of versions), then its fact's about no form; else the navi's (the ruleset holds it back, its level, its HP) and the build's of no part a tab shows: `ProblemNote` at the left, the mark and the words, the first and " · +n more", in the note's stead |
      | Rows | an item a problem is about, the ruleset's too | its name in the error red, nothing more; "Not allowed" in the error red at the end of a library's row (greyed), an unplaced program's row (dimmed) or a form's row the ruleset holds back, at the top of a navi's card (dimmed): it can't be added (§2.68) |
      | Counts | the folder's Chips, Mega, Giga, Reg and Tag, the cards' MB, past their limit (Chips: not the folder's size) | in the error red, as before |
      | Fact headings (Navi tab) | a fact of forms with a problem | its heading and count in the error red |
      | A phone's panes (no strip) | the game's rules' problems with the tab's facts, not the ruleset's | the header's line as before: a folder's last line of its counts' box, PROGRAMS' line, the cards' after their counts, a line over the places (the Navi tab: its note's place, as a desktop's) |
      | Library rows | a chip or card the game's rules keep out, or the ruleset (§2.68) | greyed, why at its end ("Not allowed"), red, no mark |
      | Play's list, phone's build box | how many | "MegaMan · 2 problems", red |
      | Play's line under HOST and JOIN, the lobby, Practice | "This build can't be used: <the first> · +n more." | as before: the mark and the line; the lobby's reserved line; Practice's dialog |

    - *Every kind has a place:* each kind the content's rules or nettai-match say of a part a tab shows lands in the
      strip of an item (the Navi tab's fault line), or has a count, or both: the folder's ChipNotAllowed, ChipCode, RegularMemory (with Reg),
      NotAllowed, Mixed on their entry; Copies, Class (with Mega and Giga), Dark, TagMemory (with Tag), TagEntries,
      TagRegular on the entries they count; FolderSize (with Chips) and RegularEntry (the Regular chip an empty entry)
      on the empty entries; the NaviCust's Overlap, OffBoard, Range and Invalid (a turn, a color) and NotAllowed on the
      program; the cards' Twice, Range, Slot (EXE4), NotAllowed and Mixed on the card, Budget (with MB) on every card;
      the auto battle data's ChipNotAllowed, Range, Invalid and NotAllowed on the place; a form's NotAllowed and Mixed on the form;
      the navi's NotAllowed, a level's or the HP's (NotTaken, Range, Missing), and what is of no fact (Internal) on the
      navi. Only these have neither: what nettai-match's facts check says of a fact a build made by hand states wrong
      (Foreign, Unstated, Invalid, Twice, NotTaken of a form list), a NaviCust on a navi that compiles none (NotTaken,
      which the editor's builds never have: it keeps them aside) or a board size past the game's (Range), and a
      Regular chip past the folder (RegularEntry). Each is said in every item's strip of its pane after the item's
      own (`unplaced`), and a fact of forms' in every form of it.
    - *Which are the ruleset's:* `Wrong::of_ruleset` (nettai-match), the two kinds `Checked::under` adds to what the
      game's rules say (`Wrong::NotAllowed`, `Wrong::Mixed`), which no game's rules say; every other kind is the game's
      rules'. A strip says both (`Draft::line`), and so does the Navi tab's fault line, a phone's too; a phone's
      other panes say the game's rules' alone (`Draft::pane_line`); the tab bar marks both.
    - *The strip's top right:* the place a refusal has, empty otherwise, so nothing moves when a problem appears, on
      every tab the same; a long sentence is cut there with an ellipsis. The item's problems are said there whether or
      not they are said elsewhere (`Draft::unsaid`, which kept the strip from saying again what its pane's header said
      first, is gone with the headers' lines). Nothing else of the strip is under it (the user: "the error on the
      patch cards view runs int othe existing text"): a name on its line ends where the top right begins (the strip's
      middle), and the Patch cards strip's right of the card's name is three columns as wide, its two groups of
      lines then the top right's, so the groups' lines (ABILITY's "FlotShoe", "TriBustr") end before it whatever the
      language or the window's width. Chosen over a line of its own for the problem, which the strip's 104 hasn't
      room for under a group's four lines, and keeps the place every tab's top right has.
    - *The Navi tab: no strip, the note's place* (the user: "i don't like having an info thing on the navi view, it's
      redundant"): the tab is its grid of navi cards (140 tall) and the panel of the build's navi beside it, from the
      page's top, as before the strip. What the build's problems have against the form under the cursor or the
      pointer (its own, then its fact's about no form), else against the navi (and the build's of no part a tab
      shows), the ruleset's among them, is said in the place of the note under the selector ("NaviCust and patch
      cards are MegaMan only.", §2.65's place for the forms' problems), in its stead: the problem's mark, the first and
      " · +n more", on one line cut with an ellipsis. The place is as tall as the note and a line at least whether it
      says anything or not, so nothing moves when one appears; the note is said again where nothing has a problem.
      The same on a phone, whose fact headings and navi's name have no line of problems now.
    - *A mix of versions, every part of it* (the user: "guessing which versio nthe user intends is confusing, if
      mixing and matching occurs and is not permitted you should just mark everything that's mixed and matched as an
      error"): under a ruleset of one version per build, a build that holds things of more than one version
      (`Wrong::Mixed`) has the problem about every one of them (`items`: all of each version's it holds, in the
      groups' order: its Beast Out, its Crosses or souls, its Giga chips, its cards' variants), none taken for the
      build's own: each has its name red and says the mix where it is described. The sentence names the versions alone,
      "Mixes Gregar and Falzar" (グレイガとファルザーが混在しています), no thing of them as the one at fault. It was
      the things of all versions but the one the build held most of (the first of a tie), "Mixes Gregar and Falzar:
      FBeast", the Gregar Crosses beside it unmarked.
    - *The folder's header* is its title and counts on one line where the language leaves them room, as before the
      headers said problems (the English EXE6's does), else the counts under the title.
    - *A problem, and a refusal:* both in the one error red (`Theme.bad`). A problem (the rules or the ruleset have
      something against the build as it stands) carries the problem's mark before its words, wherever it is said; a
      refusal (why what was asked wasn't done, or can't be) has none, and is said by what was asked of, with the row's
      shake and the refusal's sound: the strip's top right, a library row's end.
    - *One look:* `ProblemNote` (widgets.slint: `Message` with the mark, right-aligned) for every problem the editor
      says, a strip's (`Refusal`'s `problem`), the Navi tab's note's place (at its left) and a phone's panes';
      `ProblemMark` for the tab bar's and the lists';
      `Refusal` for a strip's refusal. One wording: `Words::problem`'s sentences, several in a place as
      `Words::summary` says them.
    - *What was tried* (2026-10-15, the same day): the header's top right said the shown tab's problems, the first and
      " · +n more", and opened them all as a list; it replaced a strip that said the focused item's problem (so a tab
      with problems and the cursor elsewhere said nothing), a NaviCust that said none, the Navi tab's two lines under its
      selector, and three marks for an item (a ring, a red row, a red name), now one, the red name. Then the header's
      line and list went, with "✓ Ready to play": each pane's header said the game's rules' problems with its part on
      its title's line (the folder's header two lines in every language for it), a fact of forms' on its heading's
      line, the navi's on its name's line, and the strip the described item's game's rules' problem where its pane's
      header didn't say it first; the ruleset's select said what the ruleset had against the build, beside it on every
      tab (a phone's on a line kept under it). Those lines are a phone's alone now, without the ruleset's. Then the
      Navi tab had a strip as every tab (a form's face, name, version's mark, a Cross's description, element and fact,
      else the build's navi's), across the grid's side with the cards under it shorter where four rows didn't fit
      (EXE5's), the problems at its top right; a navi's card focused described the build's navi. It went for the note's
      place (above).
    (§5 rule 13; §6 Builds, Play; §7.)
67. **The ruleset's page moves as a screen does** (2026-10-15). The user: "why is there no animation for opening the
    ruleset editor?" and "also shouldn't the version names be capitalized in the ruleset editor?"
    - *Opening and closing:* the page's parts arrive as a screen's (`Arrive`, one after another: the row, the
      categories, the category's head and items); closing it, the lobby or the build editor under it comes back from
      above, fading in (`UnderPage` in app.slint), where both had been shown and hidden at once.
    - *The quick choices in capitals:* a category's version choices read GREGAR, FALZAR, ALL, as ALLOW ALL and ALLOW
      NONE beside them and as a version's tag everywhere.
68. **What a ruleset holds back can't be added; a build that holds it is in violation** (2026-10-15). The user, of
    the build editor: "if something is not allowed by the ruleset, it should still be allowed to be added but it should
    be in violation. also it doesn't seem to color all disallowed chips as red, just hte first one. also you should
    disallow chips and program advances as well, and they should apply to auto battle". Then of the auto battle part:
    "you should instead have a separate abd rules page" (an auto battle rules page); of the program advances, "stop
    the program advance from forming during the battle instead", and then "actually nvm don't ban program advances".
    Adding it anyway was tried, and taken back the same day: "okay actually rulesets should prevent adding unallowed
    ncps/chips/patch cards/etc".
    - *What the ruleset holds back is refused where it would be added*, as the game's rules' refusals are: a folder's
      chip (from the library, or dragged in), a NaviCust program taken from the list (a click on one of its colors
      too), a patch card or a card's variant (put in, dropped in, or chosen in place of the variant in), an auto
      battle place's chip (by the places' own category; put in or dragged in), a navi chosen, a form put in (ALL puts
      in only those allowed: the rest are left out, as for want of room, with the refusal's sound). The row shakes
      where the tab's rows shake, the refusal's sound plays, and the strip's top right says "Not allowed" (the Navi
      tab, which has no strip: the sound, and its row or card already says so). Before it is tried, its library row
      (the folder's, the patch cards', the auto battle data's) is greyed with "Not allowed" at its end in the error
      red, as a row the game's rules refuse; its NaviCust row (where none of it is placed) is dimmed with "Not allowed"
      in the error red; its form's row says so in the error red; its navi's card is dimmed with "Not allowed" at its
      top in the error red. The folder's view stays the game's rules' alone (`View::hold_to` isn't back): the tab
      asks the ruleset (`Draft::allows`).
    - *A build may still hold it* (made under another ruleset, or the ruleset changed since): each is the build's
      problem (`Wrong::NotAllowed`), its name in the error red on its row, the strip's top right (the Navi tab's
      note's place) saying "Bass isn't allowed", the tab's word marked. Taking it out is never refused (Del, NONE, a
      card that is in, the build's navi chosen again).
    - *Every copy is marked:* a chip held back in the folder was one problem at the entry that first held it, so the
      first copy alone was red (`ruleset::named_by` kept an item's first entry). The problem is still one for each
      item (one sentence, one count), and is now about every entry that holds it (`Wrong::NotAllowed`'s `entries`,
      `Problem::at`): each copy of a chip, each placed copy of a program, each auto battle place. A mix of versions
      was already about every copy (`Wrong::Mixed` names the things, and a tab marks each that holds one).
    - *Auto battle is a page of its own:* EXE4's and EXE5's rulesets have an AUTO BATTLE category (オートバトル), by
      the setup's fact `auto_battle_places`, after the souls: the chips a folder lists and the Program Advances
      besides (the category's `classes`), which the auto battle data's program advance list holds and no folder
      does. It is held back apart from the folder's Chips: a chip one allows the other may not, and the presets allow
      everything there. A place holding one it holds back is red, and the strip says it, as the folder's entries.
    - *Program advances are not held back:* neither as a build's problem nor in the battle (both were asked, then
      dropped by the user). A ruleset still never changes how a round plays.
    (§6 Builds; docs/app.md §8; docs/design/rules-in-luau.md, "Rulesets".)

69. **ALL and NONE of the forms** (2026-10-15). The user, of the Navi tab's selector: "the souls/crosses should be
    able to select all/none".
    - *Where:* a list of forms (EXE6's Crosses, EXE4's and EXE5's souls) has two small words, ALL and NONE (すべて,
      なし), in its heads' letters at the right end of each version's head, a hairline between: per version, as the
      cartridges' builds are one version's (GREGAR ALL is Gregar's five Crosses); a group of none after the
      versions' has them on its head's room. A list that shows no versions has them at its heading's right end. A
      fact of one form always chosen (EXE6's Beast Out) has none. The head's name is cut short of them; nothing
      moves, the words are on lines that were there (a phone's touch is 6 more over and under the line, in the room
      over and under the head).
    - *What they do:* ALL puts the group's forms in, in the list's order, while the list has room ("CROSSES 5/5"),
      kept in the battle's order; the rest are left out, with the refusal's sound (the tab's refusal: a full list
      refuses one more), and so are those the ruleset holds back (§2.68). NONE takes the group's forms out. Each is one change (`Builds::choose_forms`; the editor has
      no undo). A word that would do nothing (ALL with the group in or the list full, NONE with none of it in) is
      dim, and refused if taken.
    - *Controls of their own:* each word is a control the arrows reach as any (`forms-all`, `forms-none`), Enter (the
      pad's A) takes it; the footer's Choose says so. Chosen over a segmented pair: they are actions, not a state.
    (§6 Builds; docs/app.md §8.)

70. **The Navi tab's problems on a line of their own** (2026-10-15). The user, of them in the note's place under the
    selector: "the error message placement on the navi tab is janky as hell", then of where: a line kept right
    under the navi's facts (HP, element), and "make sure the layout doesn't shift though". The line is there whether
    it says anything or not and the selector starts under it at the same place, so neither moves when a problem
    appears or goes; its place doesn't hang on how long the selector is. What it says is as before (the form under
    the cursor or the pointer's problems, else the navi's); the note under the selector is the note again, always.
    (§5 rule 13; §6 Builds.)

71. **CLEAR** (2026-10-15). The user: "you should add clear for navicust/folder/patch cards/abd" (abd: auto battle).
    - *Where:* one small word, CLEAR (クリア), in the ALL and NONE words' letters (§2.69; `WordButton`), at the right
      end of a pane's header on each of the four tabs: the folder's (after the counts, its word where they would end;
      the counts on their own line under the title and CLEAR where the language leaves the one line no room, as
      before), the patch cards' (after the counts), PROGRAMS' title line (its word where the list's rows end) and the
      auto battle places' (its title's line). A phone's: at the right end of the folder's and the cards' counts' box's
      first line, of PROGRAMS' line and of the auto battle places' problem line. Nothing moves for it: it is on a line
      that was there, the header's title or problem line cut short of it.
    - *What it does:* empties that part of the build in one change: every folder entry, its Regular and tag marks
      with it; every placed program (the board kept; a piece in hand dropped); every patch card (EXE4: every slot);
      every auto battle place. The editor has no undo, so it asks first, as a build's DELETE does: "Clear the
      folder?", "Clear the NaviCust?", "Clear the patch cards?", "Clear the auto battle data?", CANCEL (focused) and
      CLEAR in the error red. Dim, and passed by, where the part holds nothing.
    - *Keys:* a control the arrows reach (`clear`), Enter (the pad's A) takes it; X takes it from anywhere on the
      tab (not while typing a search, nor with a NaviCust piece in hand), and the footer says "X Clear" (the pad
      reaches it by the arrows: no button of its own).
    (§6 Builds; docs/app.md §8.)

## 3. Chosen where the user gave no answer

These were put to the user as questions with a stated default and got no reply before "start implementing". They
are the defaults; each is easy to reverse and is listed for review.

| Question | Chosen |
|---|---|
| Typefaces | Headings, buttons and labels in Chakra Petch; all other text in Murecho; codes and numbers in M PLUS 1 Code. (Japanese headings were Murecho's until the user chose M PLUS 1 for them: §2.23) |
| A game's look, where the user said nothing (§2.23) | On a change of game the panels' and the text's colors cross-fade over the sweep's time (they don't change at the moving edge, which would mean drawing every screen twice); no art from the packs in the header |
| A build's version | Fixed when the build is made, as a save's is ("It can't be changed later"). Reversed by §2.41: it is switched in the editor (by the header's tag: §2.42) |
| Netplay rounds | A single battle or best of 3, a shared setting either player can change (since §2.34: First to, 1 to 10) |
| The opponent's navi in the lobby | Hidden, with the build's name and version |
| What "export" makes | An MP4. A `nettai-video` crate writes a NUT stream (raw frames and PCM) that the app pipes to ffmpeg; the app never saves the `.nut` (the user: "don't allow raw .nut export") |
| The demo reel | The player's latest replays of the selected game; with none, the two versions' starter builds on autopilot |
| The dummy while waiting online | A copy of the player's own build, standing still, with the practice menu there to change it (since §2.57 the usual menu, its Dummy's build among them) |
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
| the cursor, the active pane's title, the chosen row's bar | `#b8a6ff` | `#c8ff5a` | `#66e0ff` |
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
uppercase, tracked 0.18em); in Japanese these are Corporate Logo at 700 (§2.51), tracked nearly not; Murecho 400 to 900 for text,
in either language; M PLUS 1 Code 500/700 for codes, numbers and times. All four are under the SIL Open Font License
and are bundled. Sizes: 11, 12, 13, 14 (the base), 16, 20, 28; the main menu's words 48 (§2.50). The wordmark "nettai" is
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
  becomes a cut or a short fade (the jack-in is the one fade: §2.49). The screenshot pass runs with motion
  settled.

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
| a build's row dragged | it lifts under the pointer (to 1.03, raised, with a shadow and a line in the cursor's color), the rows between slide a row over to leave a gap where it would land, outlined (`quick`), and let go it lands with the flash of a chip put in; refused, it shakes; over the library (where let go it is taken out), its line turns `#ff9272` and it fades to 0.6, and the library fades in under a wash of the ground at 0.7, an edge and a faint wash of `#ff9272`, and DROP TO REMOVE on a band across its middle (`quick`); the NaviCust's list does the same under a piece carried over it (CLICK TO PUT AWAY), the piece fading to 0.45 |
| the NaviCust | a piece picked up lifts (to 1.06, with a shadow), turns as a quarter turn, and drops with a pulse along its cells; the command line's arrows pulse; a bug that appears pulses purple once |
| a dialog, a menu | the scrim fades (120) as the panel grows from 96% (160); the practice and match menus slide in from the left |
| loading | a bar fills smoothly under a moving highlight; before a battle the two cards slide in from their sides and "VS" stamps |
| entering a battle (the jack-in, §2.49) | when both bars are full the battle begins under the set piece (480): the ground covers it and the field rushes, then a swipe takes the ground off from the left in eight strips, each a little after the one above, each strip's edge lit with a band of the cursor's color trailing it; the game's own encounter sound plays. With Reduce motion the ground fades (160) |
| the battle's background (§2.55) | with Battle graphics Enhanced, a background that drifts by parts of a pixel a frame glides at the window's resolution instead of stepping a whole pixel every few frames. With Reduce motion, or Original, it steps as the game's does |
| the banners and telops, in the battle (§2.55) | with Battle graphics Enhanced, their squash and stretch (and the custom screen's emblem as it spins) is drawn at the window's resolution, the game's own scale on each tick, instead of whole rows of the frame. With Reduce motion, or Original, as the game draws them |
| the custom screen's slide, in the battle (§2.55) | with Battle graphics Enhanced, the field and the sprites on it, which move down 1.5 pixels a tick with the window, are drawn where they truly are at the window's resolution instead of in steps of 2, 1, 2, 1 pixels; the window itself slides as the game's. With Reduce motion, or Original, they step as the game's do |
| what moves on the field, in the battle (§2.55) | with Battle graphics Enhanced, a sprite whose place is between whole pixels (a thrown bomb's arc) is drawn there at the window's resolution instead of in uneven whole-pixel steps; its pixel art is the game's. With Reduce motion, or Original, it steps as the game's does |
| the custom gauge's fill, in the battle (§2.55) | with Battle graphics Enhanced, the fill's end is where the gauge's counter is, at the window's resolution, instead of stepping a pixel every 2, 4 or 8 ticks. With Reduce motion, or Original, it steps as the game's does |
| a match's result (§2.49) | over the picture, the winner's half floods with their side's color from its edge (200), WIN or LOSE stamps in on the player's half, then the rounds' score and the match's time count up one after another, the cursor's sound at each tick and the confirm sound at the last, all within the set piece's 480; a press settles it |
| the lobby | the opponent's card slides in as they join; READY stamps with a flash |
| an error | a field's message fades in; a failure's dialog shakes once |

Every move of the cursor, confirm, back and refusal also has the selected game's own menu sound, as the old app
had; entering a battle has the game's own encounter sound.

**Metrics.** Page margins 32. The header is 64 tall and the footer 44, the same on every menu screen. A control is
40 tall. A list row is 44, or 26 with a 2 gap where chips or cards are listed (the build overview's spacing, which
the editors share). Gaps are 8, 12, 16, 24.

**The header** holds, from the left: a Back button naming where it goes (`‹ MENU`, `‹ PLAY`), the screen's title
with a cyan bar before it, then at the right the game selector (§2.1). Where the game can't change (inside a
build, the lobby, a replay), the game's name stays in the same place without its arrows. The main menu's header
has the logo in Back's and the title's place: the app's icon, then "nettai" (the user: "maybe no logotype in the
header, maybe logo then \"nettai\""). The icon (`docs/logo/nettai-icon.svg`) is a retrowave sun setting over the
battle field, three rows of six panels in perspective, red on the left and blue on the right ("maybe the retrowave
sun with the perspective battle field looks the best actually"), without the characters ("don't bother with putting
the characters on it, i feel like the tropical feel already engenders it"). It is drawn as the app is ("i feel like
the visual style of the logo looks out of place" of a first, glossy one: a rounded tile, gradients, a glow): flat,
the action's amber for the sun and its stripes and the horizon, the field's columns and rows in the two sides' red
and blue over their tints (made solid, so it reads on any background), the seam in the text's white; no tile, no
gradient, no glow. It is the same mark everywhere: the header, the loading screen and the installers' icons. The
logotype, 熱帯 in the same amber as tall as the t of nettai beside it (`docs/logo/nettai-logotype.svg`, from Corporate Logo and Chakra Petch
outlined), is for the README, the installers and a site, not the app's screens.

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
13. **A build's problems are one system, said where they are** (§2.66). What the build's problems have against an
    item, the ruleset's with the game's rules', at the top right of the strip that describes it (every tab's but the
    Navi tab's, which has none and says them on a line kept under the navi's facts (§2.70): the form under the cursor or
    the pointer's, else the navi's): its own, those of a whole it counts in (whose count is in the error red too), and
    those of its part that are about no item; after the problem's mark (the tab bar's red square), the first and " ·
    +n more", in a place reserved for it (nothing moves when one appears), which nothing else of the strip runs under.
    The editor's header, the ruleset's select and a pane's header say nothing of them (a phone, which has no strip,
    says the game's rules' in its panes' headers, and the Navi tab's fault line as a desktop's), and a row nothing
    but its name in the error red where a problem is about its item. A mix of versions a ruleset doesn't allow is
    about every thing of each version the build holds, each marked, and its sentence names the versions alone. A refusal (why what was asked wasn't done) has no
    mark and is said by what was asked of: a strip's top right, a library row's end.

## 6. The screens

Each is in the mockups under the same name.

### Loading
- **Starting up:** the logo (the icon, 160 square, 112 on a phone, over "nettai": "you should put it prominently on
  the loading screen"), one bar 480 wide, "Loading BN6" and a percentage. The game last used loads before the
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
  carries their name alone (§2.16). The battle then begins under the jack-in (§2.49): the set piece is drawn over
  its first frames, and nothing waits on it.

### Main menu
- Left: PLAY, REPLAYS, SETTINGS and EXIT (§2.50), each the word alone, 48 (`Theme.menu-word`, tracked 0.075em), in
  a column 416 wide, rows 96 tall 8 apart, centered in the body's height, about as tall as the reel beside
  them; nothing under them (About is in Settings, §2.53). The longest word the languages have (the
  pseudo-locale's REPLAYS) fits the column's 384 for text. On a phone the words are 32 on rows 68, under the reel. Practice is
  a button of Play's and the builds are Play's (§2.50, §2.57).
- Right: the demo reel, the battle picture at three times (720 by 480), with no caption under it ("don't show the
  caption text").
- The selected game's title music plays across the menu screens and fades out as a battle starts.

### Play
The builds and the ways to play one, on one screen (§2.50), built around "pick a build, then what to do with it"
(§2.57): the builds, the chosen build (PRACTICE, EDIT and ⋯ in its header), and a row of controls for playing it
online, read in that order. In the body's units (1216 by 644 at the reference window):
- **The builds**, the left column, 400 wide: + NEW BUILD at the top (as wide as its words; the main action while
  the game has no build), and under it from 52 the player's builds of the game, an open list down to 564 (rows
  52: the name, then "MegaMan · ready" or what is wrong, and the version's tag at the row's right: tried after
  the name, the user took it back, "move it back to the right, it's so cramped"). The row the cursor is on is the
  build chosen, remembered by game. A save dropped on the window is made a build ("Imported as Dark MegaMan"
  quietly in the reserved line where the save's preset changed what it held). Delete on the list asks as the
  menu's DELETE does; a right click on a row opens the build's menu there.
- **The chosen build**, the right column, from 424, 792 wide: its overview (Builds, below; the wide sheet, 564
  tall here, 540 at least), framed, with PRACTICE, EDIT and ⋯ (40 by 40) in its header at the right, 16 in from the frame and 24
  down, 8 apart (PRACTICE and EDIT regular buttons, as wide as their words); the version's tag follows the
  build's name, 12 after it (the user: "the version indiactor should be next to the build name in the play
  screen i think"). ⋯ opens **the build's menu**, the selects' list (§2.43) 200 wide
  under it, its right edge at the button's (over it where there is no room under; a phone's sheet): DUPLICATE,
  RENAME and DELETE, in the buttons' type, DELETE under a line in the error color. The cursor starts on the
  first row; Enter, a click or a tap chooses, Esc or the pad's B closes it. DELETE asks once, Cancel focused. A
  build whose file doesn't read has an empty frame, PRACTICE and EDIT unavailable and DELETE alone in its menu;
  with no build, none of the three is there.
- **The ways to play it online** (§2.57), one row of the app's controls at the body's foot, under the overview
  (from 424 to its right edge), at 580, 40 tall, no panel and no label over it:
  - **HOST**, the main action's button while there is a build (an online match is what PLAY is for, and HOST is
    the way that starts one by itself; with no build it has the normal look and + NEW BUILD is the one main
    action): the main button's amber and its sheen, as wide as its word. HOST makes an invite, its code from
    the server, and the wait (what CREATE INVITE was).
  - 24 after it **JOIN's field** (208 wide; "Invite code" in it while it is empty) and 8 after that **JOIN**, a
    regular button. The field takes an invite's code: what is typed is kept as its six characters, capitals, in
    threes (a hyphen or a space typed or not); a seventh character typed doesn't go in (the user: "why does
    the invite code allow more than 6 characters?").
  - **DIRECT CONNECT**, a text button (no fill, no line, its words muted) as wide as its words, at the row's
    right end, its right edge the overview's (the user: "direct connect should be deemphasized"). It opens the direct connection's dialog (below).
  Under them at 626 the reserved line (18 tall, as wide as the overview, 12). Pressed with no build, PRACTICE,
  HOST and JOIN say there "Make a build first." (§7); with a build that can't be played, why; a code no match
  has, "No match with that code.". The list and the overview grow down to 16 over the row: both end at 564 (the
  overview's wide sheet is 540 at least).
- **+ NEW BUILD** opens its dialog: a name and the version, CANCEL and CREATE BUILD, and under them, below a line,
  FROM A SAVE FILE…, which closes it and opens the system's file dialog for a save; the build is made of the save
  and named for its file (Builds, below).
- **PRACTICE** starts a practice battle with the chosen build as the player's; the practice menu's EXIT comes
  back with it still chosen.
- **EDIT** opens the chosen build in Builds' editor, whose Back (‹ PLAY) comes back with the build still chosen.
- The focus opens on the chosen build in the list (with none, on + NEW BUILD), and Enter there goes to HOST. The
  arrows walk by where things are: Down from + NEW BUILD into the list and from its last row to HOST, Right from
  any row of the list to PRACTICE (past the overview, which takes no focus; to EDIT or ⋯ where it can't be used),
  along the header PRACTICE, EDIT, ⋯, Down from them to the row; along the row Right goes HOST, the field, JOIN,
  DIRECT CONNECT. Tab goes in reading order: + NEW BUILD, the list, PRACTICE, EDIT, ⋯, HOST, the field, JOIN,
  DIRECT CONNECT. A control that isn't there is passed by.
- **On a phone** one column in the same order (358 wide): + NEW BUILD the column's width; at 52 the chosen build
  in a box, 72 tall (Left and Right step it, Enter opens the builds as a sheet); at 136 PRACTICE (as wide as its
  word, 120 at least), EDIT (the rest) and ⋯ (40) in a row, 8 apart; at 188 HOST the column's width and 52 tall,
  as a phone's main buttons are; at 252 the field (what JOIN leaves) and JOIN, 8 apart; at 304 DIRECT CONNECT
  the column's width; at 356 the reserved line (two lines, 40). The overview isn't shown there (the editor shows
  the build whole).
- **Direct connection** is a dialog (§2.57): Host (this machine's address with COPY, and HOST: this machine
  listens on its port, 47474, and waits as for an invite; "Send this address to your opponent.", or why it
  can't host) and Join (an address field, a host or an IP with its port, an IPv6 one in brackets, and CONNECT;
  an address that isn't one says so under it); CANCEL. It opens with the address field edited (§2.32).
- **Waiting for an opponent:** a practice battle starts at once (the picture at three times, tagged PRACTICE), with
  a panel beside it: the invite code, large, in fixed-width type, grouped in threes, copied to the clipboard as it
  comes ("you should automatically copy the invite code when hosting starts"; a direct host's address too); COPY CODE; "Waiting for
  opponent"; "The lobby opens when they join."; CANCEL. Hosting directly, the panel has YOUR ADDRESS, this
  machine's address and port, COPY ADDRESS, and "Send this address to your opponent." in the last line's place.
  Keys go to the battle; Esc or Start opens the practice menu, the usual one (§2.57, "The waiting practice's menu
  is the usual one"), its rows the same but Music and EDIT, with Copy invite code and Cancel invite (Copy
  address and Stop hosting) at its top in place of EXIT; what it sets lasts for the wait. (It starts as it
  always did, with no menu over it: §2.57.)
- **The lobby:** the two players' cards (yours shows your build; theirs shows their name and whether they are
  ready, nothing else), then one row of settings: First to (a stepper of the round wins that take the set, 1 to
  10, 2 at first: §2.34), Ruleset (§2.59: the host's a select with EDIT, the joiner's the host's ruleset by
  name with VIEW), Closed builds (off, on), Input delay (a stepper, yours alone, default 4, remembered), the
  connection's bars and ping (no music: "don't allow bgm select in lobby"; the battle's music is picked in
  Practice, §2.29); then READY, CHANGE BUILD, LEAVE; then a reserved status line. First to, the Ruleset and
  Closed builds are shared: both see them, and a change clears both ready marks; either player changes First to
  and Closed builds, the host alone the Ruleset; the opponent never sees Input delay. A build that holds something the ruleset doesn't allow can't
  ready up and the line says the first thing ("Bass isn't allowed"); nothing is taken out of a build to play it
  (Patch cards off, which played each build without its cards, is gone: §2.59).

### Practice
- **No screen of its own** (§2.57): PLAY's PRACTICE goes straight into a practice battle, the two cards and the
  jack-in first as for any battle. It starts on the defaults: the player's build PLAY's chosen one, the dummy on
  the same build, Arena Random, Background Random, Music the game's own, and the menu's options at their
  defaults (Stands still, Normal, every option normal, the input display off). The practice menu is open over
  the battle as it starts, the focus on RESUME (the user: "when practice mode starts you should also bring up
  the menu immediately"), so the dummy and the stage can be set before fighting: the round's opening plays
  under it, and the battle is held as its custom screen begins, nothing of it changing but its music (§2.57);
  RESUME goes on from there. (The practice battle beside a waiting invite starts as before, with no menu over it, and
  RESTART doesn't open it.)
- **A visit** (§2.40, §2.54) is the time from PRACTICE until PLAY is shown again, the trip to the editor
  included: what the menu sets lasts through it, and each visit starts on the defaults. Nothing is remembered
  between visits.
- **The battle's music** starts with the battle, as any battle's: the music picked (§2.29), the game's own at
  first. Nothing plays it before the battle, and PLAY's title music comes back after.
- **The practice menu** (Start or Esc in a practice battle, and open as one of Practice's own starts), its focus
  on RESUME each time it opens ("focus should always default to the resume button"): a panel on the left, the
  battle behind it held where it stands wherever it opens, and looking as the game's own pause does ("music
  continues, stuff still animates"): its music going on, its background and its panels' animations moving on,
  no PAUSED banner, the dummy in view (§2.57, "One hold, everywhere"). Rows are a label and a value between arrows, changed at once by Left and
  Right; bare rows under group titles, their steppers, dropdowns and EDIT 40 tall (36 on a phone), in one column
  so they line up, the rows 52 tall (§2.44: "option 1"; on a phone the label over the control):
  - Dummy: Dummy's build (§2.57: a dropdown of the game's builds that can be played, the player's own at first;
    a change starts the set over with the dummy on it, the menu staying open, as an arena's does), and under it
    EDIT, a button in the controls' column that opens the dummy's build in Builds' editor (its Back ‹ PRACTICE);
    Back there comes back to the practice battle, started again on the visit's choices with the build as the
    edit left it, the menu open over it. Then Action (stands still, moves around, fights, a second pad), with its
    Level (easy, normal, hard: how well it fights) under it while it is Fights. Its custom screen goes with its
    Action. No recording: "get rid of recording support, it's so complicated". The user: "the dummy menu is so
    confusing and has so many redundant options and it's confusing to navigate. you should make it clearer and
    less redundant". Beside a waiting invite the dummy is on the player's own build at first and the Dummy's
    build offers the same builds; there is no EDIT (§2.57: the editor would leave the wait).
  - Stage: Arena (the game's arenas, by place, "12 / 47") and Background (the background pool, every game's
    battle backgrounds, §2.31, by game and name, "BN4 ACDC Area"), each Random first, a Random background drawn
    from the pool; each a pictured select whose grid opens on Enter, a click or a tap (§2.56); an arena changed
    starts the set over on it, a background changed only redraws ("changing the background shouldn't change the
    entire arena either in trianing mode"). Music (the battle's music, §2.29, a dropdown: the battle's music
    switched at once; not beside a waiting invite, whose music is the menus'). A group of its own: "you should
    split the practice menu settings into arena and battle so the arena settings aren't mixed together"; named
    STAGE since §2.56, "having an "arena" field under "arena" is so weird".
  - Options: Your HP and Dummy's HP (normal, refills), Custom gauge (normal, always full), Chip draw (shuffled,
    folder order), Input display (a group of its own, Display, until §2.56)
  - RESUME, RESTART, EXIT (§2.40; since §2.57 back to PLAY, the build still chosen; no QUIT TO MENU). Restart is
    also R or the pad's top button.

### Builds
A build's editor, and what any screen shows of a build. The builds themselves are Play's (§2.50, §2.52): + NEW
BUILD (with FROM A SAVE FILE… in its dialog), the chosen build's overview with EDIT and ⋯, the build's menu of
DUPLICATE, RENAME and DELETE.
- **Opened by Play or Practice:** Play's EDIT, and the practice menu's EDIT under Dummy's build, open the build in
  the editor, whose Back (‹ PLAY or ‹ PRACTICE in the header) goes straight back to the screen that opened it
  (§2.28; §2.57: Practice's battle starts again, the menu open).
- **The build overview** (also Play's, and each player's panel in a match): the build's name and navi; the forms it
  has, a row of their faces with their versions' marks (§2.62); Stats (HP, Attack, Rapid, Charge, Custom, Mega chips, Giga chips, Regular memory, each with what the NaviCust and
  patch cards added beside it in cyan; then any stat the game's rules give besides, by its name in the game's
  strings: EXE5's Soul time, the turns a Soul Unison lasts, for a navi with souls, §2.58); Abilities; Bugs, each by
  name and level, purple; then the folder's 30 chips in two columns of 26-tall rows: number, icon, name, REG or
  TAG1 or TAG2, element, code.
- **New build** (Play's + NEW BUILD; RENAME is the same dialog with the name alone): a name and the version (two choices, in §2.2's order, their names alone); "Sets the Crosses and
  Giga chips. It can be switched later in the editor." (§2.41, §2.42.) The folder starts as a new save's of that game.
  There are no random builds anywhere. Under its CANCEL and CREATE BUILD, below a line, FROM A SAVE FILE… (a ghost
  button) closes it and opens the system's file dialog: the save's build is named for its file, its version and
  everything else the save's (§2.52).
- **The editor's** header has the build's name and nothing of its problems (§2.66): they are said where they are.
  A tab with problems has the problem's mark by its word. What the build's problems have against an item (a folder's
  entry, a form, a placed program, a patch card, an auto battle place, the navi), the ruleset's (an item it doesn't
  allow, a mix of versions: every thing of each version the build holds) with the game's rules', is said at the top
  right of the tab's strip while it describes the item, where nothing was refused (the Navi tab, which has no strip:
  on its fault line under the navi's facts, §2.70): the item's own; a whole's it counts in ("29 chips, a folder has 30" in an empty
  entry, a class's limit in each chip of the class, the cards' MB in each card), whose count is in the error red too;
  and its part's that are about no item. The item's name is red on its row. Each is the problem's mark and the error
  red's words, the first and " · +n more", in a place that is there when it says nothing. The ruleset's select and the
  panes' headers say nothing of them; a phone, which has no strip, says the game's rules' in its panes' headers (the
  Navi tab's fault line as a desktop's). No version's tag: a build has none (§2.60). Its tabs
  are NAVI, FOLDER, NAVICUST, PATCH CARDS and, in EXE4 and EXE5, AUTO BATTLE (the pad's shoulders or A and S switch
  them); the editor opens on NAVI every time, whichever tab was last used (the user: "don't remember the tab"). With a navi other than MegaMan the last three stay in place, greyed, and keep their contents: the build's file
  keeps MegaMan's NaviCust, cards and auto battle data aside, outside the side it hands to a match (the rules
  refuse a NaviCust on a navi that compiles none), and puts them back when MegaMan is chosen again.
- **Navi:** the game's navis as a grid of cards (140 tall); the chosen one's HP and element beside it (no charge
  shot: no game's content names a navi's, and the user chose to leave it out), the face the round starts it with. No
  strip (§2.66: the user, "i don't like having an info thing on the navi view, it's redundant"). Under them, one
  after the other (§2.63): in EXE4 and EXE5 MegaMan's Light or Dark, a segmented control (LIGHT | DARK) as tall as
  the editor's rows; the selector of the build's forms (§2.60, §2.61): EXE6's Beast Out (BEAST OUT: GBeast and
  FBeast side by side under their versions' heads) and Crosses, EXE4's and EXE5's souls, each fact's heading with its
  count (in the error red where it has a problem) over its forms' rows grouped by version; and the note,
  "NaviCust and patch cards are MegaMan only.", as tall as its words. Nothing there moves when a
  problem appears (§2.65, §2.66): a fact's heading in the error red, a chosen form's name a problem is about in it
  (in a mix of versions, every form of each version the build has), "Not allowed" in the error red at the end of a
  form's row the ruleset holds back (it isn't put in, and ALL leaves it out; one the build has can be taken out,
  §2.68; a navi it holds back is dimmed, "Not allowed" at its card's top, and isn't chosen), and on a line kept
  under the navi's facts (HP, element; §2.70), in the same place whatever the selector holds, what the build's
  problems have against the form under the cursor or the pointer, else against the navi, the ruleset's among them
  (the problem's mark, the first and " ·
  +n more"). A list of forms (Crosses, souls; not the Beast Out, one always chosen) has ALL and NONE at the right end
  of each version's head, or of its heading where it shows no versions (§2.69): ALL puts that group's forms in, in
  order, while the list has room, and refuses the rest; NONE takes them out; each one change, dim where it would do
  nothing. On a phone the panel is over the grid, as tall as what it shows, the selector's rows two to a row, the
  fault line as a desktop's, and the ruleset's select between the panel and the grid.
- **Folder** (§2.11 to §2.14): a strip across the top describes the chip under the cursor from either pane (its
  art, name and code, the game's description, element, class, MB, attack, how many are in the folder). Below, left:
  the framed folder, its header its title and Chips 30/30, Mega, Giga, Reg and Tag memory (on a line under the title
  where the language leaves them no room beside it; one past its limit in the error red) and CLEAR at its right end,
  which empties every entry and its marks, asked first (X; §2.71); then the overview's own 2
  by 15 block (a phone's counts' box keeps its last line for what the game's rules have against the folder). Right: LIBRARY, a search field, Sort and its direction, then rows of one chip in one code (icon, name,
  a note, MB, attack, element, code), 15 showing. The pointer on an entry or a row shows its chip in the strip
  (§2.64). Enter on a slot goes to the library; Enter on a row puts it in
  the slot and returns to the next slot. R sets the Regular chip, T tags. A chip that can't be added stays, greyed,
  with the reason in red ("Gregar only", "Giga limit reached", "4 copies at most"); the rules are Tango's. One the
  ruleset holds back is refused too (§2.68): its row greyed, "Not allowed" at its end; a build that holds it all the
  same has every entry of it as its problem, its name red. A mark the
  rules refuse is said at the strip's top right (§2.66), and where nothing was, what the build's problems have
  against the entry described (an empty one: the folder's size; a version's Giga chip in a build that mixes versions
  under one version per build: the mix, of which every such chip, form and card variant has its name red); an entry a
  problem is about has its name in the error red.
- **NaviCust** (§2.10, §2.26, §2.64): a strip across the top describes the program under the cursor (or the
  pointer, on a row of the list): its compressed shape then its whole one over COMPRESSED and UNCOMPRESSED (a
  program of one shape, that one with no word; §2.65), its name in its color, the game's words (the user: "you
  should show ncp descriptions") and its colors; the board framed at the left with a key under it; PROGRAMS, an
  open list, in the middle, each program once (in its color chosen, a cross if a plus part; its chosen color's part
  Placed, Placed ×2 where more than one copy is, In hand, Out of bounds or, where none is placed, Not allowed in the
  error red, the row dimmed (the ruleset holds it back: it isn't taken, the strip's top right saying so; a copy a
  build has placed all the same is red, §2.68); then at the row's right edge
  its colors as small boxes where it has more than one, Left and Right choosing, a click or a tap choosing and taking
  that color's part in hand; its name in the error red where a placed copy has a problem, which the strip's top right
  says of the program it describes, §2.66; a phone's PROGRAMS' line), and CLEAR at the right end of PROGRAMS' line,
  which takes every placed program off the board, asked first (X; §2.71); Effects and Bugs at
  the right, what the NaviCust does and the bugs it brings
  (not a patch card's). The command line is marked three ways: a line above and below its whole row that runs over
  placed pieces, an arrow at each end outside the grid, and a tint in its empty cells. EXE6's board has a white
  outline at its edge, a hatched ring outside it, and no cells at the corners. Arrows move the piece in hand, A and
  S turn it, C compresses or uncompresses it (in place, turned as it is: "rotating the ncp on compression is so
  confusing"; a program taken from the list comes compressed where it compresses: "navicust pieces should default
  to compressed in the editor"; its hint says Uncompress while the piece is compressed: "compress key on the
  navicust view should say uncompress if uncompress is possible"; a program of one shape has no C and no hint:
  "if the piece is the same compressed and uncompresse then it can't compress/uncompress, there's only one form of
  it"), Enter (the pad's A) places it (where it doesn't fit, the strip's top right says why: §2.66; a phone's line
  under the key), Esc (the pad's B) puts it away into the list, as the game's B does: so a placed program is taken off by picking it
  up and putting it away, or with Del ("there seems to be no way to remove the ncp from the board once placed").
  Enter on a program's row takes a new copy in hand whether or not copies are placed, and Del there takes the
  last copy off (§2.26). The mouse carries a piece by clicks (§2.38): a click takes up a placed piece or a new copy
  from a program's row, the piece follows the pointer with no button held, and a click puts it down; a click on the
  list puts it away (the list shows CLICK TO PUT AWAY while the pointer is over it), a click anywhere else off the
  board puts it back where it was; a right click puts it away, or takes off the piece under the pointer.
- **Patch cards** (§2.19, §2.20, §2.64): the folder's shape. EXE5 and EXE6: the installed cards in order in the
  framed panel, rows as the folder's (number, name, its version's mark, MB), Cards and MB used in its header (a phone's
  with what the game's rules have against the cards after them) and CLEAR at its right end, which takes every card
  out (EXE4: empties every slot), asked first (X; §2.71), and
  "Together", what they add up to, at its foot; the library's rows the same, with their note ("Not allowed" in the
  error red, greyed, where the ruleset holds the card or its variant back: it isn't put in nor chosen, §2.68); the
  strip across the
  top lays the card under the cursor (or the pointer) out: its lines in its two groups, Parameter then Ability, bug
  lines purple, and at its top right why a library's card can't go in ("Needs 70 MB") or wasn't put in (§2.66); an
  installed card a problem is about (a version's variant in a build that mixes versions, one held back) has its name
  in the error red, and what the build's problems have against it (the mix, the cards' MB past the list's, MB in the
  error red) is said, where nothing was refused, at the strip's top right of the card it describes (a phone's after
  the counts). The strip's right of the card's name is three columns as wide: the two groups, then the top right's,
  so a group's lines never run under it. EXE4:
  six slots
  named 0A to 0F, each with its card or Empty. A card belongs in one slot, which its library row names, and Enter
  puts it there, in place of the card that slot held; so does a drag of it onto the slots, and a slot's card dragged
  onto the library is taken out (it can move to no other slot). A two-part card says where its other half must be; a card
  shows its effect and the bugs it adds to or clears.
- **The mouse on the build's lists** (§2.37, §2.38, §2.64): the pointer on a row shows what it holds in the strip,
  the focus staying where it is; a click marks a row (a folder's entry, a patch card or slot, an
  auto battle place) and removes nothing; a right click takes out what it holds; a row
  is dragged to another place (a folder's entry with its marks, a patch card in the list's order, a place's chip in
  its list or traded with another list's); dragged back onto the library and let go there, what it holds is taken
  out (the library shows DROP TO REMOVE while it is over it); let go anywhere else off the list it goes back. A right
  click on a library row that is in the build takes it out. A library's row dragged onto the list goes in where it is
  let go (§2.39): a chip in that entry or place in place of what it held, a card at that place, the others shifting
  on (EXE4: in its own slot, wherever over the slots), refused as a click is; elsewhere, nothing. On a phone, a tap
  opens the library's sheet (its REMOVE takes out) and a held touch drags, to reorder only; the sheet's rows aren't
  dragged.
- **Auto battle** (EXE4, EXE5): the same shape, its places grouped under their lists in the framed panel, CLEAR at
  its header's right end emptying every place, asked first (X; §2.71; a phone's at the right end of its problem line). The
  library offers what the place being filled can hold (§2.27): a Mega place the Mega chips, the Giga place the Giga
  chips, the Program Advance place the Program Advances (which no folder holds), any other place the chips a
  folder can hold. A Program Advance has no picture and no element, and none is shown ("program advances don't
  have elements or chip art, so don't show those"). A chip of another class in one of those three lists (a build or save made by hand) is a
  problem of the build, shown as any other: its place's name in the error red, and the strip's top right says why of
  the place it describes ("Cannon can't be in the Mega list"; §2.66; a phone's, a line of its own over the places).
  The ruleset's AUTO BATTLE category holds chips back here apart from the folder's Chips (§2.68): a library row of one
  is greyed and says "Not allowed", and it isn't put in (its row shakes, the strip says so); a build that holds one
  all the same has each place of it red, the strip saying it.

### Replays
- **The list:** All, Online (and Training only if training is ever recorded) and a search by opponent; rows of the
  date, the opponent, the score, the length; beside it the selected replay with WATCH, EXPORT VIDEO, SHOW FILE and,
  apart, DELETE. No build names (§2.16). A click chooses a replay and doesn't start it (the user: "clicking on a replay
  shouldn't automatically start it"); WATCH, Enter or the pad's A does.
- **The viewer:** the picture at three times over a transport: the time, a bar to click or drag with a mark where
  each round begins, the length; then to the round's start, a frame back, play or pause, a frame on, the next
  round; Speed (½, 1, 2, 4); View (P1, P2); EXPORT VIDEO. Space plays, Left and Right seek 5 seconds, comma and
  period step a frame, the brackets go a round. After BUILDS, two toggles, each off at first (the user: "add
  support for input view and picture in picture in replay viewer"): the input display (I), Practice's, both
  players' buttons in the picture's top right corner; and the other console's picture (P), small in the bottom
  right corner of the one shown.
- **Export:** a dialog of one size from start to finish: Size (even multiples of 240 by 160: 2, 4 and 6 times, 4
  first), View, Sides (One, or Both side by side, the View's at the left: the user, "add support for two
  sided side by side export"; with Both a sound track a side, the left's first: "for 2 sided replays, audio track 1
  should be left player audio, audio track 2 should be right player audio"), Part, Save as, the progress bar and its line, CANCEL and EXPORT. The sizes are even because an MP4
  keeps its color at half resolution, and at an odd multiple every pixel edge fringes (measured: 25 dB at 3 times
  against 40 and 44 at 2 and 4). The export is an MP4 and nothing else: the user, "don't allow raw .nut export".
  (The NUT stream is how `nettai-video` hands the pictures and sound to ffmpeg, never a file the app saves.) Every
  export has a chapter a round, "Round 1", "Round 2"… in the app's language: the user, "video exports should have
  chapter info for rounds".

### In an online match
- The picture fills the window, with no header or footer.
- **Connection quality:** four bars and the round-trip time in a corner for the whole match, amber then red as the
  link worsens; "Reconnecting 12 s" while it is down.
- **The link panel** (§18): clicking or tapping the corner opens a panel over it, modeled on Tango's PvP telemetry,
  live and refreshed ten times a second: the bars again, a graph of the round trip over the last ten seconds (a gap
  where the link was down), then two columns: the link (round trip, jitter, loss, reconnects, clock sync's skew,
  the lead) and the rollback (input delay, rollbacks, the last one's depth, the deepest, frames waited), and under
  them the packets and KB sent and received. A figure that says the link is doing badly is amber, then red, as the
  bars. Clicking the corner again closes it, and nothing else does ("telemetry window should not dismiss on click
  outside, it should only be dismissed if the ping indicator is clicked again"); it takes no keys (the match has them,
  and Tab and Esc keep their jobs), and the match keeps running under it. On a phone it sits as near the top as it
  has to.
- **Both builds:** holding Tab (Select on a pad) shows the player's build on their side and the opponent's on
  theirs, each the build overview without a name; letting go hides them. With Closed builds on, only the player's
  own.
- **The match menu** (Start or Esc; the game's own pause never appears): BACK TO MATCH, YOUR BUILD, THEIR BUILD
  (not with Closed builds), CONTROLS, SETTINGS, LEAVE MATCH. "The match keeps running", and it does.

### After a match
The mockups show no screen for this; it is decided here.
- **Online:** the game's own end plays on the picture (who won, in the game's own way), then nettai's result
  over it (§2.49): the winner's half flooded with their color, WIN or LOSE on the player's half, the rounds'
  score and the match's time. CONTINUE (Enter, the pad's A) settles it where it still moves and goes on; it goes
  on by itself after six seconds. Each player is then back in the lobby on the same link, neither ready, with
  the same settings, whether or not the other is still looking at their result. Either can change build, play
  again or leave. Another match with the same opponent needs no new invite code.
- **LEAVE MATCH** in the match menu ends the match for both players and puts both back in the lobby, as a
  match's end does. Leaving the opponent altogether is the lobby's LEAVE.
- **Practice** has no end: the practice menu's RESTART and EXIT are the ways out of a set.
- **A replay** stops at its end with the transport still there.

### Settings
One scrolling page, a list at the left jumping to a group: General (Name, "Shown to your opponent."; Language),
Video (Picture size, Full screen, Battle graphics: enhanced or original, Reduce motion), Audio (Music, Sound effects), Online (Input
delay, "In frames. Lower feels faster; higher rolls back less."), Controls (every action with its key and its pad
button, each rebindable), About (Version; Your files, SHOW FOLDER; Made with, SHOW LICENSES, which opens About).

### About
Settings' subscreen (§2.53): SHOW LICENSES in Settings' About group shows it in the page's place, its title
ABOUT and its Back ‹ SETTINGS, which comes back to the page on SHOW LICENSES. The app's name, its version and its
license; Slint's "Made with Slint" widget; and MADE WITH, a list 440 wide of the fonts the app bundles and every
package built into it (each row its name, its version, and its license under them), the chosen one's license
files beside it in a panel, scrolling (§2.25). On a phone the list is the page and a license opens as a sheet from
the bottom.

### On a phone
One column; pickers and the chip library open as sheets from the bottom; everything tapped is at least 44 tall; no
key hints, and the cursor only with a pad or keyboard connected; in battle the pad is drawn under the picture and
START and MENU open the same menus. Held sideways, a phone uses the desktop layout.

## 7. Errors the player can see

A fixed enum, each with a catalog string; the raw error goes to the log only. The first set:

| When | What the player reads |
|---|---|
| the signaling server can't be reached | Can't reach the server. Check your connection. |
| no match has the invite code in JOIN's field | No match with that code. |
| the match already has two players | That match is full. |
| the two apps differ | Your opponent has a different version of nettai. You both need the latest. |
| a build breaks a rule | This build can't be used: 2 Giga chips, limit 1. (Several: the first, then " · +1 more", §2.66.) |
| there is no build to play with | Make a build first. |
| the link dropped for good | Connection lost. The match has ended. |
| a replay is from another version | This replay is from a different version and can't be played. |
| the server needs a newer app | This version of nettai is too old for the server. Get the latest. |
| the opponent chose another game | Your opponent is on BN5. Pick the same game. |
| the opponent's build breaks a rule | Your opponent's build can't be used. |
| the opponent left the lobby or the match | Your opponent left. |
| the opponent never answered | No answer from your opponent. |
| a link couldn't be made | Couldn't connect to your opponent. |
| a direct connection's address is malformed (no port) | That isn't an address. Use host:port. |
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
   with what was added, and the rules' own after them by name, `build::NamedStat` (EXE5's soul time), what gives
   the abilities, the bugs by name with a level or an amount, `build::Extent`,
   whoever brings them, the patch cards' MB) and a chip's
   row with its sort keys (`build::ChipInfo`, `build::sorted`). Light or Dark is a choice of the game's builds data
   (`Builds::choices_of`, `preset_of`, `take_preset`; `settle` for a build from a file or a save; none for a navi
   but MegaMan), with the face the round starts him with (`Checked::face`); EXE4's slots are `Builds::slots`
   (`Slots::own` and `name`, `held`, `put`). docs/design/rules-in-luau.md's last section says how the rules
   answer.
8. **Replays:** seeking, by keeping snapshots; export through `nettai-video` and ffmpeg; no build names in the file.
9. **The lobby's protocol** stops sending a build's name, version and navi; carries the rounds, the Ruleset (by
   names: §2.59) and Closed builds as shared settings; and holds the start until both sides have loaded.
10. **Random builds** are removed from Play, Practice and the lobby.
11. **Practice's** new options (§3) need new support in the battle engine.
12. **The screenshot pass** (the old `NETTAI_TOUR`) walks every screen in each language and size and is compared
    with the mockups.
13. **The build stays cheap.** "try not make the slint build really expensive, because the current nettai binary
    takes forever to build". The app's clean and incremental build times, and the time after touching one `.slint`
    file, are measured and reported with each step, and a choice that costs build time needs a reason from this
    document.
