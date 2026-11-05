# The custom screen by actions: a visual cursor (proposal, 2026-11-01)

The user asked "what if the custom screen had mouse support?", turned down steering the simulated cursor with
synthesized button presses ("that's kind of gross"), and proposed the other way around: "should it be the other way
around where the cursor on the custom screen is purely visual?" This note says what that takes, from an audit of the
simulation, the input path and formats, the renderer and verification (the audits' notes with file:line for every
claim are in nettai-verify's docs/custom-screen-actions/).

**Status (2026-11-01): checkpoints A and B are built** (keys and pads through the layer; mouse and touch, §5),
and the pointer's own take-backs (§2: any pick taken back, a Cross un-chosen with the picks kept, a held chip
released). Since then the tick carries the choices alone, no gestures, and the mouse and a finger are one path
(§3, §5).
The layer is `custom::layer::Layer` in nettai-battle (with `custom::Lockstep` for both sides in lockstep), a type of
its own that `Battle` never holds: the code moved there uses the screen's crate-private parts (its slots'
neighbors and lists, the chip checks, the chatbox), and every crate that needs it (the frontend, the tools, the
compat crates, the renderer) already depends on nettai-battle. docs/engine/custom-screen.md §0, §3, §9 describe it
as built.

## 0. Summary

- **The idea holds.** Inside the simulation the custom screen's cursor is only ever the target of A: it draws from
  no RNG, feeds no timer that runs into the fight, and nothing shared reads where it is. Each console's screen is its
  own until OK (docs/engine/custom-screen.md §0), and the battle needs only what was chosen and when.
- **The battle keeps the decisions**: the deal, the picks and their order, the rules' buttons (Beast Out, a soul,
  a re-deal, a scrap, Arm Change, the capsules), a Cross chosen, OK and its tick, and every window whose effect lands
  inside an animation (Beast Out's reorder at its tick 53, a soul's trade at its step 20, EXE4's SHUFFLE landing at
  count 16), which stay busy phases of the simulation as now.
- **A player's tick carries an action** beside their buttons while the screen is open ("pick slot 7", "take back",
  "choose form 2", OK). The screen in the simulation stops reading buttons.
- **A local screen layer, per player and outside the simulation**, holds the cursor, the chip window's preview,
  the dark chips' hover shading, the Cross list's cursor, and R's description, L's run message and SELECT's hiding.
  Keys and the pad drive it with the same ported cursor logic as today, so playing with keys looks and feels as now;
  mouse and touch send actions straight from what they hit. The cursor moves at once, without the input delay.
- **It is a minor version (0.21)**: the tick word, the netplay wire, the replay format and the content API change.
  0.20 replays won't play in 0.21 (the no-compat rule).
- **Verification keeps its reach**: the original's recorded button traces go through the same local layer, per side
  and in lockstep, to become actions; frame comparisons draw the layer's cursor where mGBA draws it.

## 1. What stays in the battle and what moves out

| State | Today | Proposed |
|---|---|---|
| The deal, the slots, the picks and their order, the column, the hand built at OK | simulation | simulation |
| The rules' buttons and their uses (Beast Out, souls, UNITE, re-deal, scrap, Arm Change, capsules) | simulation, pressed by A on them | simulation, pressed by an action |
| Windows whose effect lands mid-animation (EXE6 Beast Out 70 ticks, the BeastOut chip 85, a Cross chosen 34; EXE5/EXE4 souls and capsules; EXE5 Arm Change; EXE4 SHUFFLE) | simulation | simulation (busy phases; no action is taken while one runs) |
| Beast Out's camera shake (two RNG1 draws a tick for 40 ticks; the stream later re-deals and shuffles the next round's folder) | simulation | simulation |
| OK's tick, the slide-out, the 50 result words, the time limit's clock and time-up | simulation | simulation |
| The cursor and its blink, where it starts (the first dark chip), START's jump to OK | simulation (hashed) | local layer |
| The joypad's auto-repeat and its per-console phase (`Side::joypad`, `PlayerSetup::joypad_phase`) | simulation (hashed) | local layer (and the trace converter) |
| The chip window's hovered chip, EXE4's element sprite | battle state the renderer reads | local layer |
| The dark chips' hover shading (`look.fade` when it shades) | battle look (not hashed, but read: §8.1) | local layer; the decision fades stay in the battle, hashed |
| R's description, L's run message, SELECT's hiding, their chatboxes and timing | simulation phases (they delay the next key) | local layer (they delay the next action, from there) |
| EXE6's Cross list: opening on UP, its cursor (`window_cursor`, the rules' hashed state), R on an entry, closing on B | rules' state and keys | local layer; the choice is an action |
| Screen sounds (cursor, refusal, shade, picks) | the simulation's cues | the layer's for what it does, the battle's for decisions |

The fight's input record keeps taking the held buttons every tick, the screen's included
(`battle.rs` 1118-1121): a START, L or R held across the screen's close must not read as a new press on the first
fight tick. So buttons keep flowing in the word while the screen is open; the screen ignores them.

## 2. The actions

Common to the three games; each is taken only while the screen is choosing (no window busy, not hidden), and the
simulation checks it as A on that slot is checked today (a refused one plays the refusal, as now):

| Action | Today's keys | What the battle does |
|---|---|---|
| `pick(slot)`, slot 0-9 | directions, then A | picks a chip that is selectable, with fewer than 5 picks (EXE4 refuses an invalid chip) |
| `press(slot)`, slot 8-11 a rules' button | directions, then A | runs the button's `pressed` hook (Beast Out, a soul, a re-deal, a scrap, Arm Change, a capsule) |
| `ok` | A on OK (START only moves the cursor) | OK, with nothing picked too |
| `take_back` | B | undoes in today's order: an Arm Change hold, the rules' own take-back with nothing picked (EXE6's Cross), a soul trade, the last pick (detaching a capsule) |
| `choose_form(entry)`, entry 0-4 | UP opens the list, directions, A | EXE6: chooses a Cross from the offered list (MegaMan, no Beast Out picked, no Cross yet, the entry not marked) and plays its 34-tick window |
| `unpick(place)`, place 0-4 | none (a click on the pick) | takes back the pick in that place as B takes back the last, with what it carries; the picks after it move up a place |
| `unchoose` | none (a click on the Cross chosen) | un-chooses the rules' form (EXE6's Cross) as B with nothing picked does, the picks kept; refused where none is chosen |
| `release(slot)`, slot 0-11 | none (a click on the button) | puts back the chip a rules' button holds (EXE5's Arm Change) |
| `switch(member)` | (already in the word: `PlayerTick::switch_to`) | EXE5 DS's party switch, unchanged |

Per game: EXE6 uses all but `release` and `switch`; EXE5 has the soul (slot 11), ColonelSoul's Arm Change (and
`release`), MeddySoul's two capsules, SearchSoul's re-deal (three uses) and the DS switch; EXE4 has UNITE (slot 11),
SearchSoul's SHUFFLE (three uses), and its offered dark chips and JunkSoul's chips as ordinary `pick`s. 49 values in
all (§3).

**The pointer's take-backs** (the user, trying the screen by pointer: "unselecting a cross shouldn't remove all
chips" and "you should also allow removing chips from the middle of the queue"). The game's B undoes only the last
pick, and with nothing picked the Cross; before these, a click on a pick sent B down to it, one a tick, and a click
on the chosen Cross every pick and then the Cross. Keys stay the game's: B is `take_back`, and only the pointer
sends `unpick`, `unchoose` and `release`.

- **`unpick(place)`** takes back that pick and what it carries, as B does the last's: a capsule attached to it
  (EXE5's MeddySoul) comes off, a button picked alone takes its face back (EXE6's Beast Out: its `taken_back`), a
  chip's `chip_taken_back` runs (EXE4's offered dark chips' flags, EXE6's BeastOut chip's face). The picks after it
  move up with their column icons as the screen copied them, and keep what they carry; the column's last cell is
  emptied, as B empties it. Of the last pick it is B's take-back: the same state, sounds and hooks.
- **A soul given for a chip** (EXE5's souls, EXE4's Double Soul: the soul first in the column, the chip out of the
  picks) frees the code the chip held, so a later pick may not go with it. Taken back from before later picks, the
  chip goes back in the soul's place where it goes with the picks left (the availability's own test,
  `screen::Together::goes_with`); where it doesn't, the soul and the chip both leave, the chip back in the hand
  (the rules hear the soul's `taken_back` and the chip's `chip_taken_back`, as two Bs would say them). The soul as
  the only pick: as B, the chip back as the pick.
- **A held chip** (`custom.hold_last_pick`, EXE5's Arm Change) is no pick: it leaves the picks and their codes for
  the button, and B puts it back only with the picks as they were then, before taking back the last pick. So
  `unpick` never puts it back (where B on the last would put it back first, `unpick` of the last takes back the last
  pick), and a pick taken back before its place moves its place down one, so that B still puts it back once the
  picks after it are gone. **`release(slot)`**, a click on the button holding it, puts it back at any time: with the
  picks as they were, as B; else as a pick in the place it left (the picks from there moving down) where there is
  room and it goes with them, else back to the hand.
- **`unchoose`** asks the rules what B with nothing picked asks (`custom.unchoose_form`), now with picks too. No
  pick depends on the Cross: Beast Out is refused while a Cross is chosen and a Cross while Beast Out is picked
  (`custom.form_taken`), the BeastOut chip goes with any code, and the slots' availability reads no form. EXE6's
  Luau gives the emotion window the BeastOut chip's face again where it is picked (a Cross chosen after it covered
  it).

## 3. The tick word and the formats

- **The word**: `PlayerTick` grows an optional action, `choice: Option<ScreenChoice>`, at bits 20-25 of the word (it
  is 20 bits today: buttons 0-9, the switch 10-11, the microphone 12-19). Buttons and an action, not one or the
  other: a word decided while the presented frame shows the fight lands the input delay later, maybe on the screen,
  so it has to say both. *As built (B2):* the choice is bits 20-24 (its 31 numbers fit five), and the tick carries
  the pointer's gesture too, `gesture: Option<Gesture>`, bits 25-31 (`Gesture::code`: a hover, a click or a
  describe on each of the 24 targets, a click on none; 0 none): the one the player's layer took on the tick it read
  the word's buttons. The battle never reads it. The word fills its 32 bits, so nothing masks its top any more
  (`TICK_WORD` and the replay's `INPUT_MASK` are gone). *As built (the pointer's take-backs, §2):* the choices
  number 49 (`ScreenChoice::CODES`, 50 with none: picks 1-12, presses 13-24, OK 25, B 26, the form list's entries
  27-31, `unpick` 32-36, `unchoose` 37, `release` 38-49), more than five bits hold, and the gestures 74
  (`Gesture::CODES`, 75 with none, `Away` among them) need seven, so bits 20-31 hold the two as one number: the
  gesture's plus 75 times the choice's (at most 3749 of 4096). `PlayerTick::word` and `from_word` alone know it; netplay's prediction drops
  a choice and a gesture through `PlayerTick::carried_on`. A gesture alone stays under 128 (the word under 2^27), a
  choice is past it.
  *As built (choices only, 2026-11-01; the user: "for the mouse support in custom screen i don't actually care
  about all the mouse movement, right? i only really care about the selections/deselections and the mouse movement
  is just noise"):* the gesture is out of the tick (`PlayerTick` has no `gesture`, `Gesture` no numbers), and bits
  20-31 hold the choice's number alone (`ScreenChoice::code`, 0-49). Nothing of the pointer is recorded or sent:
  only what it chose, as keys' choices are. *As built (narrow words; the user: "the custom screen inputs are so
  big, do they have to be this wide?", and "microphone input can be 1 bit: max volume or silent"):* the microphone
  is bit 12 and the choice bits 13-18 (six bits hold its 50 numbers), so the word is 19 bits
  (`PlayerTick::WORD_BITS`), and the replay writes each input as LEB128 (layout 2): a choice is three bytes there
  and on the wire.
- **The wire** (nettai-netplay `protocol.rs`): the event flags move up past the action; a tick with an action is a
  4-byte element instead of 3. Actions are a few dozen a screen, so the mean frame doesn't change. *As built (B2):*
  the flags sit at bit 32 of the head, past the gesture; a tick with a gesture is 4 or 5 bytes, one with flags 5 or
  6. A pointer makes a gesture as it moves onto another target, a click, a describe: a few a second while it moves.
  *As built (§2's take-backs):* a gesture alone is 4 bytes, a choice (the more numbers, the rarer) 5. *As built
  (choices only):* a choice is 4 bytes again, and the pointer adds nothing to the wire; with the narrow words, 3.
- **Prediction**: a missing remote word is predicted as its last buttons and **no action** (as events are dropped
  now), and no gesture (B2: each of the other's gestures is a misprediction too, though the battle doesn't read
  it; the re-simulated ticks come out the same). Each remote action is then one misprediction, and the remote's cursor moves cost none, because they aren't
  input any more. `StandInBattle`'s prediction (`last.clone()`) gets this override; it fixes a smaller thing seen in
  the audit: today a predicted tick repeats the remote's party switch request until the real word arrives.
- **Replays** (nettai-replay): the input mask grows to the action's bits. A replay still has to show both players'
  cursors (the viewer, the both-sides spectator view, video export), so it also records each side's local layer as a
  presentation track: the cursor, the hovered chip, an open description or list, on the ticks they change. Not in
  the digest; the viewer's seek keyframes keep the layer's state as they keep the HUD's. *As built (A):* the track
  is each side's screen delay (`Tick::delays`, control bits 5 and 6, written when it changes), and the viewer runs
  each side's layer again from its words' buttons, the word that landed `delay` ticks on; a netplay peer learns the
  other's delay from a marker in its stream (`protocol::Element::Delay`) and records both. *As built (B2):* the
  words carry the pointer's gestures (above), and the replay's driver gives each side's layer the gesture, the
  buttons and the switch request of the word that landed the side's screen delay later (a gesture given with the
  input is the tick's, ahead of the layer's own queue): the viewer, the both-sides spectator view and video export,
  all on that driver, show a pointer player's cursor, hover, descriptions and Cross list as they were, and the
  viewer's keyframes keep the layers as before. Netplay's screen delay is the present delay **and one**: getgud
  presents a frame's battle the present delay behind the input it decides, and the layer reads the battle shown
  before the frame decides (A recorded the present delay: a replay ran a netplay player's layer a tick early). And
  a netplay layer reads each battle once, on the frame whose word lands the screen delay past it, as a replay runs
  it: while the presentation holds a battle (a round's first frames until the delay's words are in or the other's
  first input comes; a delay made longer) the frame's buttons pass the layer by and a gesture waits in its queue.
  (Before, it read the round's first battle the delay and two times, which also put the key repeat's beat off the
  battle's but at a delay of 4.) What a replay can't give again: the battles a peer never showed (it jumps past them
  once the other's late input comes, or when the delay is made shorter) are read in the replay with the words'
  buttons alone. *As built (choices only):* the words carry no gestures, so the replay's driver gives each side's
  layer the whole word that landed the side's screen delay later, its choice with it, and a layer given a choice
  (a replay's, a peer's, an AI's) puts its cursor on what it names (`Layer::cursor_to_choice`: a pick's or a
  button's slot, OK, the slot a pick taken back goes back to). Keys' screens replay as they were, tick for tick; a
  pointer player's cursor is on each place they chose as they chose it, and between choices it is the keys' alone
  (their presses, descriptions and the Cross list opened by click aren't shown).
- **State and digest**: `custom::Side` loses its joypad, the screen its cursor, the rules' state EXE6's list cursor;
  the decision fades move out of `ScreenLook` into hashed state (§8.1).

## 4. The local screen layer

One per player, beside the renderer's per-console HUD state, outside the simulation, its state never hashed:

- **Keys and the pad** drive it with the code that drives the simulated cursor today, moved, not rewritten: each
  game's slot neighbors and EXE4's per-slot key orders, the auto-repeat with its per-console phase, START to OK, UP
  into EXE6's Cross list from the top row or OK, the first dark chip as the starting place. A when the cursor is on a
  slot sends `pick`/`press`/`ok`; B sends `take_back` (or closes the list).
- **It reads the presented battle**: what is pickable, whether the screen is choosing or busy, and the time left. It
  sends no action while the presented screen is busy, and drops its own description, run message or list when the
  battle closes the screen (time-up): the battle's own time-up stays the authority.
- **R, L and SELECT are the layer's**: the description's box and timing, the run message, the hiding (which moves
  the picture as today). While one is up the layer sends nothing, as the simulated sub-screens read no keys now; the
  time limit counts in the battle regardless.
- **Drawn by the renderer** as a second input to `Renderer::render`: the cursor, the chip window's chip, the shade,
  the description's chatbox, the list's cursor. On a screen of both, each seat draws its side's layer (the replay's
  recorded one for the other side).
- **Sounds**: the cursor, refusal and shade sounds become the layer's cues, played at once for the local player;
  decisions' sounds stay the battle's frame-tagged cues (rollback.md §3.2).

## 5. Mouse and touch

*As built (B).*

- **Hit targets** (nettai-render `custom::targets`, `Renderer::targets` and `Renderer::target_at`): while a side's
  window is at rest, the renderer reports what a pointer finds where, from the places it draws the cursor and those
  parts at, for each seat that shows its own console's screen as it is (not a mirrored seat), in the frame's pixels:
  the window's anchor (top left, in every Fill size) plus the HUD layer's shake. The layer's `Target`s:
  - `Slot(s)`, each slot the cursor goes to: a chip's whole cell edge to edge (16 by 24, its code plate in it), a
    button in the slots' run too (EXE6's ChpShufl and DustCross on slots 8 and 9: each its cell); OK and a button at a
    place of its own (the special slot's Beast Out, EXE5's soul, EXE4's UNITE) the cells its cursor goes about
    (`cursor_cells`, as the upright view's OK box), a two-cell one halved;
  - `Pick(k)`, the column's picks;
  - `FormTab`, the form list's tab: the window label its rules name (`FormListRules::tab`; EXE6's `cross-select`,
    the tab's tiles with its triangle, and the window's header once it is up), where the window's map shows it;
  - `Form(k)`, the form list's entries (the names' 9x2 cells' rows, as wide as the exit label: the window's frame
    either side with them), and only they, the tab and the exit while its window is up, which so span the window;
  - `FormExit`, the form list's exit label while its window is up (B2: `FormListRules::exit`; EXE6's `b-exit`,
    B:EXIT, the window's foot bar where the label's tiles are: the letters, their filler and the end, 88 by 8).
    EXE4 and EXE5 draw no such label (their windows' labels are BATTLE CHIP and CHIP SELECT), so they have none.
  None while the window slides, hides or isn't there.
- **The picture takes the pointer**: in the app a `TouchArea` over the battle's picture, as the replay viewer has,
  enabled while the battle has the keys and the frame has targets (`Player::takes_pointer`), sends each event of the
  mouse's or a finger's (`Shell.picture-pointer`); the app maps it to the frame's pixels through the picture's size
  and gives it to the battle's player (`Player::point`).
- **Gestures** (nettai-frontend `pointer::ScreenPointer`, the layer's `Gesture`): the player hit-tests against the
  last frame drawn and gives the local player's layer a gesture for its next tick (`Layer::point`; a few kept, one a
  tick, the pointer's place replaced by what follows it). The mouse and a finger are one path (the user: "then you
  can unify the touch path with the mouse path"; and, the gestures out of the tick: "well you should still let me
  hover but it doesn't have to go into the replay"); a finger moves only pressed, so it never hovers:
  - over a target (the mouse moved onto it, unpressed; a press on it, the mouse's button or a finger down):
    `Over`, the cursor goes there (its sound, the chip window, the dark chip's shade) with no choice; in the form
    list, its cursor (until a form is chosen, as its keys); over a pick, the cursor goes on the pick's icon in the
    column (below); over another target (the tab), the cursor is on nothing, as away;
  - the mouse off the targets (moved on none, at each move; or off the picture): `Away`, the cursor on nothing
    (the user: "when the mouse cursor isn't hovering over anything on the custom screen the selection should be
    empty"; then "can you blank it", of the chip window; `Layer::away`): no cursor drawn, no dark chip's shade, no
    element icon (EXE4's sprite, drawn by what the window shows, not the cursor's slot), the chip window empty
    (`ChipWindow::empty`: no name, picture, code, element or damage; the frame in the standard colors, the window's
    ground where the picture goes), `Layer::cursor` kept;
    over a slot or a pick it shows again there, and a direction, A or R brings it back to its slot, that press doing
    no more (B is the game's, START goes to OK). It lasts through the screen's closing and the next one's opening
    (a mouse moved in the fight finds the cursor on nothing); a finger never hovers, so never makes it;
  - let go on the target it was pressed on (off it, or slid off and back, nothing; the system taking it, nothing):
    `Click`, what A on it would choose (a pick, a button's press, OK); on a button holding a chip, its `release`
    (§2; A on it is refused, as the game's); on a pick, its `unpick` (§2: that pick alone, the picks after it moving
    up; before, a take-back a tick down to it), the cursor to the slot it goes back to (the user: "when deleting a
    chip from the tray, you should reposition the cursor to where the chip returns"; B, the game's, leaves it where
    it is); on the tab, the form list opens where UP would, from anywhere (the layer's checks but the cursor's
    place; refused: the refusal sound), and again closes it as B does; on an entry, its form chosen (as A); on the
    chosen entry, `unchoose`: the Cross un-chosen, the picks kept (before, every pick was taken back first); on the
    exit label, the list closes as B does (B2), nothing chosen, and so it does on none, off the window (the user:
    "in crossselect, you should probably allow clicks outside cross select (e.g. the chips) to dismiss
    crossselect"; the chips aren't targets while it is up); a click anywhere, on a target or none, while a
    description, the run message or the hidden window is up is a key for it, as A (it closes the description);
  - right-click, or a press held still on its target for `pointer::LONG_PRESS` (30 ticks, half a second; its
    letting go then chooses nothing): `Describe`, R on what it is on: a slot's chip (the cursor goes there), a
    pick's chip (the cursor stays), a form list entry's form.
  L's run message and SELECT's hide have no gesture (the keys keep them).
- **The cursor on a pick** (the user: "when mousing over the stack of chips you should show the cursor there too.
  or does that conflict with how the keyboard works?"). It doesn't conflict: the column is the pointer's place
  alone. The pointer over a pick puts the layer's cursor on it (`Layer::pick_cursor`) beside its slot (`Layer::cursor`
  stays as it was), with a move's sound; the renderer draws the screen's chip cursor about the pick's icon (inside
  the box `custom::targets` reports for `Pick(k)`), and the chip window shows the pick's chip as the pointer over
  its slot would (a button's pick, its picture); no dark chip's shade runs there. Over a slot the cursor moves there
  as before; over another target, or away, it is on nothing (above). Keys while it is on a pick:
  a direction only brings it back to its slot (that press moves it no further; held, it repeats on the slots as
  any hold does); A does what a click there does (`unpick`, §2); R describes the pick; B is the game's B, START
  goes to OK. No key moves the cursor into the column. A pick it takes back (A, a click) puts it on the slot the
  pick goes back to; one taken back from under it otherwise (B's last) leaves it on the place, over the pick that
  moved up there, or, with none, back on its slot. It is the layer's state, the player's view alone (§3: a
  replay shows the cursor on a pick's slot as the pick is taken back).
- **Touch**: the mouse's path (above): a tap on a pick takes it back as a click does.
- **Phones held sideways**: while the local custom screen is open (its first tick to its close), the see-through
  pad's directions, L, R and SELECT, which sit over the window, hide (`Shell.pad-gives-way`) and take no finger
  (`TouchPad::give_way`; a direction held lets go), so taps on the window go on to the picture; A, B, START and MENU
  stay. A slot's cell is 16 by 24 frame pixels, about 6 by 9 mm at a sideways phone's multiple; the tab is one row of
  cells (8 pixels) tall.
- **Phones held upright**: the pad is under the picture and stays as it is; taps on the picture work as above.
- **Where it plays**: Practice and live play (`LivePlayer`) and netplay (`NetPlayer`): `Driver::layer_mut` gives the
  local player's layer, which moves at once (no input delay); its choices go into the player's tick as their keys'
  do. A replay's driver gives none, and the replay viewer and the spectator view take no pointer.
- **The mouse's cursor** (B2, the user after B, then on trying it: "the hidden cursor is so hard to use, maybe it
  should change into a pointer hand if it's mousing over something clickable? that applies for the entire
  application"): the system's cursor stays over the battle's picture, the arrow, and turns a hand over a target of
  the local player's custom screen (`Shell.picture-hand`, from `Player::pointer_on_target`: where the mouse last was,
  `ScreenPointer::at`, against the last frame's targets), as over the app's own controls (docs/design/app-ui.md:
  the hand on what a click takes).
- **Replays**: only the choices go in the player's ticks; a replay puts a pointer player's cursor on each place
  they chose (§3).
- **Tests**: the layer's gestures (nettai-battle `custom::tests`), the recognizer (`pointer::tests`), the pad giving
  way (nettai `input::tests`), and per game at 240x160 and at a Fill size (347x199) a click at the middle of each
  target the renderer reports gives its choice, then two picks, the first's `unpick`, EXE6's tab and Cross entries
  and OK played by pointer, EXE6's B:EXIT closing the list, EXE6's Cross chosen and un-chosen with a pick kept, and a
  long press describing (nettai-tools `tests/custom_pointer.rs`, which needs the ROMs). §2's take-backs: the
  screen's own (nettai-battle `custom::tests`: a pick anywhere with what it carries, `unpick` of the last as B, a
  trade's chip back where it fits or to the hand, a held chip kept through `unpick` and released, the pointer's
  clicks sending them; the cursor on a pick: its place and chip window, a direction back to its slot, A's `unpick`,
  R, the pointer over a slot bringing it back, over another target or away putting it on nothing; the cursor on
  nothing: drawn nowhere, its slot kept through the next screen's opening, a direction, A or R bringing it back)
  and each game's (nettai-match `games::take_backs`: EXE5's soul, capsule and Arm Change,
  EXE6's Beast Out, BeastOut chip and Cross, EXE4's offered dark chips and Double Soul, `unpick` of the last as B
  tick for tick). Choices only: the word's fields (nettai-battle `input::tests`); a layer given another's ticks
  (buttons and choices, as a replay) takes the same choices with its cursor where the pointer's was at each
  (`custom::tests`); a set played by keys in each game, live at an input delay of 3, recorded and played back,
  both sides' layers the same on every tick, and one played by pointer on both sides, the replayed cursors where
  they were live on every tick a choice names a place (nettai-frontend `replay::tests`); a netplay set with one
  pointer player at input delays of 2 and 6 over a link with latency, the other's replay showing the pointer
  player's cursor as they saw it at each such choice (`netplay::tests`); the wire's sizes and prediction
  (nettai-netplay); the pointer's one path and the mouse's hover (`pointer::tests`).

## 6. The content API

The screen's Luau API is written in cursor terms today, and changes:

- `custom.pick`, `custom.cursor_state`, `custom.sacrifice` and `custom.redeal` mean "the button under the cursor":
  they become "the pressed button" (the engine passes the slot to its `pressed` hook). Every game's buttons call them:
  EXE6's scrap, re-deal and Beast Out, EXE5's soul, Arm Change, capsules and Shuffle, EXE4's UNITE.
- `custom.pressed`, `custom.repeated`, `custom.cursor`, `custom.set_cursor` and the rules' `custom.keys` hook go:
  only EXE6's Cross window uses them. It is rewritten as a `choose_form` hook (the choice, its checks, its 34-tick
  window, B's un-choosing) with the list itself the layer's; `form_list.cursor` leaves the rules' state.
- `custom.fading` reads only the decision fades (§8.1). `custom.describe` (a form's R) becomes the layer's.
- *As built (§2's take-backs):* `custom.take_back` (B with nothing picked) is `custom.unchoose_form`, which
  `unchoose` asks too, the picks kept (EXE6's `window.unchoose`). The take-back hooks say which pick:
  `custom.chip_taken_back(side, chip, place)` and a button's `taken_back(side, place)`, the place (from 0) the pick
  left (EXE4's dark chips shift their per-place flags by it). Nothing in Rust names what a button is: `release`
  is a button's held chip released, and EXE5's Arm Change says it is its own.

## 7. Verification

- **Trace replays** (the golden traces, the lab, the 2022 Tango rounds, rollback, mirror, `check_custom_screens`):
  each side's recorded buttons go through that side's local layer, in lockstep with the battle (it reads the presented
  state as the app's does), which turns them into actions on the ticks the original took them. The comparisons check
  outcomes (the OK tick, the hand, the transform, the status bits), unchanged; the converter must reproduce the
  original's timing exactly: the repeat's phase, the cable's held buttons carried into the screen, EXE4's settling
  tick, EXE5 DS's early repeat.
- **Frame comparisons** (compare.sh, lab-compare with custom-screen.txt, the other-side runs, presentation-check)
  render nettai-tool's frames from the same traces: the layer draws its cursor where mGBA draws it, so they stay as
  they are.
- **Sound tests** merge the layer's cues with the battle's in the original's order.
- **Unaffected**: chiplab and difftest, which record the original game itself.

## 8. Risks the audit found

1. **The hover shares a fade with decisions.** The dark chips' hover writes `look.fade`, which the battle waits on:
   EXE4's and EXE5's soul and capsule steps (`custom.fading`) and EXE5 DS's switch. In EXE5, a switch begun within
   about 5 ticks of the cursor landing on a dark chip completes a few ticks later than it would; the outcome is the
   same. Split: the hover's shade becomes the layer's, the decision fades stay in the battle, and are hashed (today
   `ScreenLook` is left out of the digest though the battle reads its fade: a divergence there would go unseen).
2. **The windows' late effects** must stay simulated, or the earliest OK moves and time-up's result changes (time-up
   keeps what a window had noted).
3. **The fight's input record** must keep the held buttons through the screen (§1).
4. **R, L and SELECT leave the simulation**: their waits no longer delay anything the battle sees. The player still
   can't act during them (the layer holds), so nothing a player does changes; the battle just doesn't know.
5. **The training AI** already chooses slots and form entries, but carries them out with a button navigator and
   observes the cursor: its observation spec changes and its networks need retraining.

## 9. Steps

1. **The engine**: the action in `PlayerTick`; the screen consumes actions; the local layer moved out of
   `custom/screen.rs` with its tests; the decision fades split from the hover; the content API (§6) and EXE6's Cross
   window, EXE5's and EXE4's buttons. The trace converter; the golden traces, the lab and the frame comparisons pass
   as before.
2. **The formats**: the wire, prediction, the replay with its presentation track; the frontend's u16 input becomes
   the tick (`Player::choose` beside `ask_switch`), its bots and drivers send actions; nettai-tools' `--keys` takes
   action names as it took `switchN`.
3. **The app**: the layer per player in battles, Practice, replays and the menu's reel; the picture's pointer, hit
   targets, gestures; phones.
4. **Training**: the new observation and actions, then retraining.
5. **0.21**: content republished for the line (the content API changed), then the app.

## 10. The user's answers (2026-11-01)

1. **Replays keep showing both cursors**, by recording each side's layer (§3): "yes". Narrowed later to the
   choices alone (§3, "choices only"): "i only really care about the selections/deselections and the mouse
   movement is just noise".
2. **Hover moves the cursor** and the chip window: "yes". Kept when the gestures left the tick, as the player's
   view alone: "well you should still let me hover but it doesn't have to go into the replay".
3. **R's description by right-click**; L and SELECT get no pointer gesture: "right click to show description. L
   doens't get used in battle anyway, no need to bind select". So `take_back` by pointer is a click on the last
   picked chip (§5).
4. **Phones held sideways**: taps on the window are the screen's, and the d-pad hides while the screen is open:
   "yes and also i guess hide the d pad? or are the chip icons too small to be clicked on phone" (the sizes: §5).
