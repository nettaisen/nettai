# The custom screen by actions: a visual cursor (proposal, 2026-11-01)

The user asked "what if the custom screen had mouse support?", turned down steering the simulated cursor with
synthesized button presses ("that's kind of gross"), and proposed the other way around: "should it be the other way
around where the cursor on the custom screen is purely visual?" This note says what that takes, from an audit of the
simulation, the input path and formats, the renderer and verification (the audits' notes with file:line for every
claim are in nettai-verify's docs/custom-screen-actions/). Nothing here is built yet.

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
| `switch(member)` | (already in the word: `PlayerTick::switch_to`) | EXE5 DS's party switch, unchanged |

Per game: EXE6 uses all but `switch`; EXE5 has the soul (slot 11), ColonelSoul's Arm Change, MeddySoul's two
capsules, SearchSoul's re-deal (three uses) and the DS switch; EXE4 has UNITE (slot 11), SearchSoul's SHUFFLE (three
uses), and its offered dark chips and JunkSoul's chips as ordinary `pick`s. About 20 values in all.

## 3. The tick word and the formats

- **The word**: `PlayerTick` grows an optional action, `choice: Option<ScreenChoice>`, at bits 20-25 of the word (it
  is 20 bits today: buttons 0-9, the switch 10-11, the microphone 12-19). Buttons and an action, not one or the
  other: a word decided while the presented frame shows the fight lands the input delay later, maybe on the screen,
  so it has to say both.
- **The wire** (nettai-netplay `protocol.rs`): the event flags move up past the action; a tick with an action is a
  4-byte element instead of 3. Actions are a few dozen a screen, so the mean frame doesn't change.
- **Prediction**: a missing remote word is predicted as its last buttons and **no action** (as events are dropped
  now). Each remote action is then one misprediction, and the remote's cursor moves cost none, because they aren't
  input any more. `StandInBattle`'s prediction (`last.clone()`) gets this override; it fixes a smaller thing seen in
  the audit: today a predicted tick repeats the remote's party switch request until the real word arrives.
- **Replays** (nettai-replay): the input mask grows to the action's bits. A replay still has to show both players'
  cursors (the viewer, the both-sides spectator view, video export), so it also records each side's local layer as a
  presentation track: the cursor, the hovered chip, an open description or list, on the ticks they change. Not in
  the digest; the viewer's seek keyframes keep the layer's state as they keep the HUD's.
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

- **Hit targets**: the renderer reports each target's box from the places it already draws the cursor at (the slot
  cells, OK and the buttons from each game's extraction, the Cross list's entries), in frame pixels. The custom window
  keeps to the top-left anchor in every Fill size, so the boxes are the original's 240x160 places shifted by the
  slide and a shake. The app maps a pointer through the picture's fit (`stage::fit`, `place_picture`).
- **The picture takes pointers**: a `TouchArea` over the battle's picture, as the replay viewer has over its own.
- **Gestures**: a click or tap on a chip, OK or a button sends its action and moves the cursor there; a click on a
  Cross list entry chooses it; hovering moves the cursor (and the chip window) with no action; right-click is
  `take_back`. R's description, L's run message and SELECT's hide have no place on the screen to click: §10.
- **Phones held sideways**: the see-through pad's directions sit over the custom window's left edge and take any
  finger near them first; while the screen is open, a tap on a slot has to win over the pad (§10).

## 6. The content API

The screen's Luau API is written in cursor terms today, and changes:

- `custom.pick`, `custom.cursor_state`, `custom.sacrifice` and `custom.redeal` mean "the button under the cursor":
  they become "the pressed button" (the engine passes the slot to its `pressed` hook). Every game's buttons call them:
  EXE6's scrap, re-deal and Beast Out, EXE5's soul, Arm Change, capsules and Shuffle, EXE4's UNITE.
- `custom.pressed`, `custom.repeated`, `custom.cursor`, `custom.set_cursor` and the rules' `custom.keys` hook go:
  only EXE6's Cross window uses them. It is rewritten as a `choose_form` hook (the choice, its checks, its 34-tick
  window, B's un-choosing) with the list itself the layer's; `form_list.cursor` leaves the rules' state.
- `custom.fading` reads only the decision fades (§8.1). `custom.describe` (a form's R) becomes the layer's.

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

## 10. Questions for the user

1. **Replays keep showing both cursors** by recording each side's layer (§3)? Recommended: yes; without it a replay
   shows picks appearing with no cursor moving.
2. **Hover moves the cursor** (and the chip window shows the chip under the mouse)? Recommended: yes.
3. **R's description, L's run message and SELECT's hide for a mouse or a finger**: hovering a chip long enough
   shows its description, and the run message and the hide stay keys only? Or small buttons beside the window?
4. **Phones held sideways**: while the custom screen is open, taps on the window go to the screen and the pad's
   directions step aside there? Or the pad keeps its place and taps work only where it isn't?
