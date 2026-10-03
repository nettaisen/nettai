# The custom screen (BN6 US Falzar, link PvP)

Between turns each player deals chips from their folder, picks some with their joypad (and maybe Beast Out or a
Cross), and presses OK. Their hand is built and sent over the link, and the fight resumes once both players'
results are in. This document is the spec for the port's custom screen, `nettai-battle/src/custom` (ruleset layer),
and says what is verified and how.

- Chip data, the chip block (the hand) and how chips are used in the fight: [`chips.md`](chips.md) §1-§2. §5 below
  supersedes chips.md §2.2-§2.4 where they differ.
- The battle's modes and the link packet: [`battle-flow.md`](battle-flow.md) §3.3, §6, §7.
- Rollback netplay: [`../design/rollback.md`](../design/rollback.md) §8.1.

**Sources.**
- The original's code (US Falzar), read statically.
- The two golden traces (machgun: Falzar vs Falzar, 2 rounds, 2 screens; soundmod: Gregar vs Falzar, 3 rounds,
  18 screens, Crosses, Beast Outs, DustCross scraps).
- Per-frame dumps of **both** consoles' custom-screen state in both replays, taken under emulation (the control
  block, the 12 slots, the joypad, the folder, the hand being built, the transform record). Every rule marked
  **[dumps]** was checked against every screen and every key press of both players there.
- **[code]**: read from the code; no recording exercises it.

**Battle mode.** Netbattles are battle mode 0 (BattleSettings+3; effects 0xE8C, link bit 8 set). The code's many
`GetBattleMode() == 1` branches are not taken, and tutorials (`byte_80269BC[mode]`) are off.

---

## 0. The model

In the original each console runs only its own player's screen (`sub_8026A28`, state block `0x020364C0`, slots
at `0x020365C0`), reads its own joypad, and sends its result over the link. The port runs **both players'
screens in the simulation**, each driven by its player's buttons, so that the whole battle is a function of the
setup and both players' buttons (rollback netplay):

| Original (per console) | Port |
|---|---|
| The local screen reads `eJoypad` (the undelayed joypad); the fight reads the link's input records, which lag it by the link queue (4 frames) | `PlayerTick::held` is the player's buttons this tick. Each player's `Side::joypad` reads them at once; the fight gets them through `link::Link`, `RoundSetup::link_delay` ticks later (4 in the recordings) |
| The status byte BS+0x11 bit 2 (custom screen open), sent in the link packet; received as BS+0x14/0x15 | `Side::in_custom` per player, carried by the link like the buttons; `RoundState::remote_status` is what arrives |
| The hand, NaviStats and transform record sent in 50 link words; committed when both magic words are in | `Side::sent` (the result and the tick its last word arrives); the fight resumes when both have arrived |
| The folder shuffle at the round's init with the console's own RNG1 | `BattleFolder::shuffled` with the RNG it's given; `RoundSetup::players[p].folder` is the shuffled folder |
| Each console's RNG1, which ChpShufl's re-deal draws from | `Battle::consoles[p]` (`crate::console`): each player's console RNG, seeded from `PlayerSetup::console` and advanced as that console's is (§8) |
| Save data: owned Crosses, Beast Out unlocked, game version | `custom::Unlocks` in `RoundSetup::players` |

Nothing in the custom screen depends on which side is "local": which screen a frontend draws is presentation.
`TickEvents` carries only `link_closed` (the end of the round) and, for checking against recordings that lack a
player's folder, that player's recorded results (`TickEvents::recorded`, §7).

**RNG.** The custom screen never draws from RNG2 **[dumps]**. It draws from its console's RNG1 only for ChpShufl's
re-deal (§3.7); the other RNG1 draws seen during screens are the emotion window's flicker (`sub_801CC94`) and the
camera's shakes (Beast Out's), which the port simulates only for their draws (§8). The folder shuffle at the
round's init is RNG1 too (§1).

**Game data.** The screen reads chip records, the Program Advances, the link navis' own chips and its slot
layout through `custom::Library`, which the battle's `Content` implements (tests use `TestLibrary`, made-up chips
on the hand-authored test content's layout). In a content pack: the slot grid and its scan lists are a rule,
`Rules::custom_screen` (`rules/custom-screen.toml`); each Program Advance recipe sits with the chip it makes
(`[[program_advance]]` in that chip's `chip.toml`, with an explicit `order`, the original's table order), and
`Content::program_advances()` puts them back in order; a link navi's own chip is `NaviData::own_chip`
(`[own_chip]` in its `navi.toml`); a modifier chip says what it does, `ChipData::modifier` (§5).

## 1. The folder and its shuffle

`eBattleFolder` (0x0203CDB0) holds 30 chips as `code << 9 | id`, one per console. It is built once per round
at init (`sub_80079F0` → `sub_800A3E4`, the frame the init sub-state goes to 8):

1. Lay out the save's folder (NaviStats+0x2D picks it): the Regular chip (NaviStats+0x2E + folder) first, the
   two tag chips (+0x56 + 2·folder) last, the rest in order. Battle mode 1 has neither.
2. `sub_800A570` shuffles it with **RNG1** (`GetPositiveSignedRNG1`):
   - split out the Giga chips (class 2), in order;
   - `sub_8000D12`: n swaps of two entries picked as `rng % n` (two draws per swap) over the other chips,
     skipping the Regular chip at the front and the tags at the back;
   - the same over the Gigas;
   - insert each Giga at `10 + rng % (n − 12)` (battle mode 1: `8 + rng % 11`) when that is within the list;
   - with tags: `t = rng % 19 + 1`, swap the tags with entries t and t+1.

   A plain 30-chip folder takes 60 draws; the recordings' folders took 55, 60, 61 and 63.
3. BattleState+0x17 = 1 if there is a Regular chip (`BattleFolder::regular_pending`); +0x44 = 1 with tags and
   +0x45 = where the shuffle put the pair (`BattleFolder::shuffled_with_tag_pair`, `ConsoleSetup::tag_pair`). Only
   ChpShufl's re-deal reads them (§3.7); each opening clears +0x44 once +0x45 is below its hand size
   (`sub_802A646`). While +0x44 is set, OK takes one off +0x45 for each chip it takes out of the folder
   (`sub_80293F8`, §5), so the index follows the pair as the folder closes up (the picks are all before it).
   DustCross's scrap (§3.6) doesn't.

Verified **[dumps]** on all 10 shuffles (both consoles, every round of both replays): same folder, same RNG1 after.
The unit tests in `custom/folder.rs` replay four of them.

The folder only changes at three moments afterwards: each opening compacts it (§2), OK takes the picked chips out
(§5), and DustCross's scrap moves chips to its end (§3.6).

## 2. Opening and dealing

The mode handler's first custom tick (`sub_8009338` → `sub_8026840`), tick **O**:

- shared: the turn counter BS+7 += 1; the gauge is zeroed and flags 2 and 0x10 cleared; the gauge task stops;
- per player: status bit 2 set; round memory cleared on the round's first screen; the screen deals.

Ticks O+1 … O+10: the window slides in (`sub_8026B04`). On O+10, except on the round's first screen, the NaviCust
custom-HP bug hits both navis (`sub_8013FD0`, never lethal). Input is read from **O+11**; keys pressed during the
opening are lost **[dumps]**.

**Hand size** (`sub_802A49C`, `sub_802A40C`), per player:

```
charge = (form is ChargeCross (5) or its Beast form (0x11)) ? min(charge + 1, 3) : 0   // per round
n = CustomLevel (NaviStats+0x0A) + charge;  extra = 0
if n > 8: extra = n − 8; n = 8
if form is not DustCross (0x0A, 0x16) and NumbrOpn (NaviStats+0x61 == 1): n = 10; extra = charge
bug = NaviStats+0x63
if bug and turn >= bug: extra −= turn − bug + 1; if extra < 0: n = max(n + extra, 2)
```

The forms are literal (ChargeCross is Gregar's, DustCross Falzar's). Checked on every opening **[dumps]** (e.g.
6, 5, 4, 3, 2 under the custom bug; 7 and 8 in ChargeCross).

**Dealing.** `sub_802945A` compacts the folder (chips first, holes last); the first `min(chips left, n)` are
dealt, slot i showing folder entry i. Chips dealt but not picked stay where they are, so they are dealt again
first next turn.

**Slots.** 12 slots: 0-4 the top row, 5-9 the bottom row, 10 = OK (right end of the top row), 11 = the button under
OK. The grid starts from `dword_802A7CC` (`Rules::custom_screen`); dealt chips fill slots 0, 1, …; slot 11 is the
Beast Out button when the player has it (§4); slots 8/9 are DustCross's scrap button (form 0x0A or 0x16) or
ChpShufl's re-deal button; a link navi's own chip goes in slot 9 once a round. Then `sub_8027F42` points every
neighbor that is an empty slot at the next slot present along fixed scan lists (`CustomScreenLayout::left_scan_*`,
`right_scan_*`): with 5 chips, LEFT from slot 0 wraps to OK, RIGHT from OK to slot 0, OK and slot 11 are each
other's UP/DOWN, and the chips have no UP/DOWN. The cursor starts on the first slot present (slot 0 when a chip was
dealt). All neighbor bytes of all openings match **[dumps]**.

## 3. Choosing

### 3.1 The joypad

`eJoypad` (`main_static_80003E4`, once per frame): held, pressed (down now, not last frame), and **repeat**: a
held button repeats on the second frame of a hold, then from its 17th frame on every fifth frame, on the frames a
console-wide counter (0-4, +1 a frame) reads 0. That counter is each console's own: `PlayerSetup::joypad_phase`
(the recordings' consoles read the frame number mod 5) **[dumps]**. `input::Joypad`.

### 3.2 Keys (`sub_8028B74`, state 4)

One action per tick, in this order:

1. UP/DOWN (repeat): UP from a top-row chip or OK opens the Cross window (§4) when the navi is MegaMan, Beast
   Out isn't picked and a Cross is offered; otherwise the slot's vertical neighbor.
2. LEFT (repeat), then RIGHT (repeat): the neighbor. A missing neighbor still uses up the key.
3. A (pressed): the slot's action (§3.3).
4. B: take back the last pick (§3.4).
5. START: the cursor goes to OK (it doesn't press it).
6. SELECT: hide the window (§3.5).
7. R on a chip: its description (§3.5).
8. L: "no time to run away!" (§3.5).

Cursor movement never skips grayed or picked slots. Verified on every state-4 tick of both players in both
replays (37.5k ticks: 692 moves, 174 picks, 49 take-backs, 88 Cross windows, 40 OKs, 10 scraps…) **[dumps]**.

### 3.3 A

| Slot | Action |
|---|---|
| A chip | If selectable and fewer than 5 picks: pick it (`sub_8028CCC`). Chip 0x13F ("BeastOut" as a folder chip) goes to state 0x44 (§3.5) |
| OK | Build the hand (§5) and slide out (§6); works with nothing picked |
| Beast Out | If selectable and fewer than 5 picks: pick it (it counts as a pick) and play its animation (§4) |
| Scrap (slots 8/9) | If usable: scrap (§3.6) |
| Re-deal (8/9) | If usable: ChpShufl's re-deal (§3.7) |

### 3.4 What can be picked (`sub_8028E32`)

After every pick and take-back the chip slots are grayed or not (`screen::update_availability`). The picked
chips' codes (ignoring `*`) and ids are summarized first:

```
for each picked chip c (as checked below; chip 0x13F skipped):
    codes 0x1B/0x1C: special = code; skip
    code != '*': code_lock = first code, or "mixed" once two differ
    same_chip = first id, or "mixed" once two differ
a chip slot not picked is selectable when:
    fewer than 5 picks, and
    id 0x13F; or (code 0x1B/0x1C and no other special code picked); or id == same_chip;
    or code_lock is none; or code_lock isn't mixed and (the chip's code is '*' or == code_lock)
```

So: one code plus any `*` chips, or several copies of one chip in any codes. Beast Out is selectable with fewer
than 5 picks, no Cross chosen, and the navi able to (§4).

**Invalid chips** (`getChipID_802A54E`, `sub_800B022`): a chip counts as chip 0x185 with code 0x1B when it is a
Mega or Giga chip and the round's count of that class (`dword_20367E0`, §5.4) is **greater** than NaviStats
MegaLevel/GigaLevel, or when its code isn't one of the chip's codes (not checked for code 0x1B or ids ≥ 0x19B).
The original also fails it on its anti-tamper check, which legitimate play never trips. Never seen in the
recordings (the counts stayed within the limits).

**B** (`sub_8029032`): take back the last pick (Beast Out included); with none picked, un-choose the Cross. The
cursor doesn't move.

### 3.5 Sub-screens and their timing

T is the tick that took the key; "input from" is the first tick the grid reads keys again.

| Sub-state | Entered by | Input from |
|---|---|---|
| 0x0C hide (`sub_8026D06`) | SELECT at T | T+1 hides; any key pressed at P ≥ T+2 brings it back; input from P+2 (that key is used up) **[dumps]** |
| 0x18 chip description (`sub_8026E4C`) | R at T on a chip | the chatbox takes any key from T+5 plus the lines of the chip's text (T+8 for most chips, T+7 for the Recov chips, SloGauge and FstGauge, T+6 for the invalid chip), or B held for 11 ticks from then; after a key at P, input from P+6 **[lab: custom/description-*]** |
| 0x1C run message (`sub_8026E98`) | L at T | the chatbox starts at T+1; it takes A or B from T+80 for MegaMan (by the message's lines; from T+21 at the earliest when A is pressed or B held as it prints); after a key at P, input from P+9 **[lab: custom/run-message*]** |
| 0x4C Cross window opening | UP at T | window from T+13 |
| 0x50 Cross window closing | B or START in the window at T | T+7 |
| 0x5C Cross chosen | A in the window at T | T+35 **[dumps, 15 cases]** |
| 0x48 Beast Out picked | A on Beast Out at T | T+71 **[dumps, 5 cases]** |
| 0x44 BeastOut chip picked (`sub_80275EC`) | A on chip 0x13F at T | as 0x48 but its fade starts 16 ticks later; the chip moves to the front of the selection at T+68 (the Beast Out flag and button stay); T+86 **[lab]** |
| 0x38 DustCross scrap | A on the scrap button at T | T+4+25k for k chips scrapped **[dumps, 13 cases]** |
| 0x28 ChpShufl re-deal (`sub_80271F8`) | A on the re-deal button at T | T+34 **[lab: navicust/chpshufl-redeal*]** |

**The chatbox** (`custom::chatbox`). Both sub-screens wait on the original's chatbox, which runs a text script
once a frame after the screen (`chatbox_onUpdate`) and clears its flag (`eFlags2009F38` 0x80) when the script ends.
The port runs what those scripts' commands do to the timing:

- **The box**: a description's opens at once (`E8 06`); the message's (`E8 00`) takes a tick and three steps, and
  then waits for the portrait. Closing (`E6`) takes three steps and a tick, after the portrait is gone.
- **The portrait** (`F5`, the message's): it fades in while the box is fully open (its tint 0x18C6, one 0x421 a
  tick: seven ticks) and out before the box closes (0x842 a tick until a channel reaches 6: three ticks).
- **Text**: at print speed 0 (`F1 00 00`, the descriptions') a whole line a tick; at the default speed 2 a
  character every other tick. A line break (`E9`) ends the tick's printing, so each line of a description costs a
  tick. Once the box has run four ticks without waiting on a command, B held or A pressed prints all the rest at
  once (`chatbox_8040154`).
- **The key wait** (`E7`): five ticks of delay, then A or B pressed (`E7 00`, the message) or any key (`E7 01`, the
  descriptions), or B held for an eleventh tick (the held ticks needn't be in a row).

A chip's description is its `description` in the content's strings (locales/en.toml; its lines apart by `\n`):
the engine reads how many lines, which the define phase counts into the record (`description_lines`); the
invalid chip's (one line) is shown for an invalid chip. A Cross's is its form's `description` (the Cross window's
script, `TextScriptChipDesc86EF4D4`'s by the form's number less one); every one has three lines, and a form
without one counts as three. The message is the operated navi's `run_message` in the strings: the define phase
counts its characters in each line (`counts`, which time it: MegaMan's is 19 and 12) and which of them move the
speaker's mouth (`talking`) into the record, whose `portrait` (a sprite) the definition gives; each link navi has
its own script (`TextScriptBattleRunDialog`'s script 3 sends it there). gen-content checks the strings and the
counted records against the ROM. (The text itself is presentation, out of the hash: docs/design/text-rendering.md
§10.)

**What the chatbox shows** is presentation, kept beside its timing in `ChatboxLook` (left out of the state digest,
as `ScreenLook` is) and read through `Chatbox::box_step`, `shows_contents` and `look`, from which the frontend
draws it (docs/frontend.md §3):

- the box's map at its opening step, while it's drawn (`chatbox_CopyBackgroundTiles_8040344`); the text and the
  portrait only while it's fully open (or not drawn at all);
- the text's sprites hold the line buffer as last copied to them (`sub_30070B4`): every tick they're shown,
  except that once the key wait has run they keep what they have until a tick prints all at once
  (`+0x3D`). So the end, which clears the buffer, leaves the message's text up while the portrait fades out
  after A, and blanks it at once after held B (the end then runs on a tick that prints all at once);
- the portrait (`chatbox_8040B8C`): drawn with the tint before the tick's step of its fade, added to each
  color channel (`sub_3005F34`); its face (`+0x1F0`..`+0x1F3`): still as `F5` loads it, talking from a character
  that talks (`chatbox_8040C44`: letters and digits), idle after one that doesn't and after every command or
  character printed all at once (`chatbox_8040C9C`); the sprite takes the face when the talking changes, and its
  animation steps once a tick it's drawn;
- the key-wait arrow (`chatbox_804082C`), from the tick the wait first runs until the key: its frame by a step
  counted only while it shows (`byte_80408A4`: six ticks each of three frames, then from the second step).

**Verified** in the lab: `custom/description-arm-001-5..9` and `-09a-5..9` (Cannon, three lines, takes A from
R+8; Recov10, two lines, from R+7), `description-invalid-4..6` (from R+6), `description-cross-7`, `-8` (a Cross's,
from R+8, back to the Cross window), `description-b-held-12`, `-30`, `description-keys`; `run-message`, `-b`,
`-wait`, `-taps-0`, `-taps-1` (A on every other frame, from either parity), `-b-held`, and the link navis'
`run-message-navi-1` to `-11` with their `-wait` (every link navi's message).

Three chips' descriptions aren't in their scripts: DblBeast's, Gregar's and Falzar's scripts copy their text from
the console's memory (`FF 01 nn`, `chatbox_FF_copytext`: 0x40 bytes of a buffer the game keeps, run as script and
returned from at its end), so gen-content finds no text for them in the ROM. Their definitions carry the text the
game shows from that buffer, as the user gave it (2026-10-01): DblBeast "Ferocious / beast / power!", Gregar
"Gregar's / breath / attack!", Falzar "Falzar's / ruinous / tornado!". Each is three lines, which is what the battle
reads of a description, so the timing is unchanged: `description-arm-137-5..9` and
`description-arm-139-7`, `-8` (the Falzar chip, dug out over two turns) and `description-arm-138-7`, `-8` (the
Gregar chip, which copies the same buffer) take A from R+8. Not reached: what `sub_802A220` closes a description for (it answers 0xFF in a netbattle with
MegaMan).

### 3.6 DustCross's scrap (`sub_8027406`)

In DustCross (form 0x0A, or its Beast form 0x16) slots 8/9 are one scrap button, usable once a screen when the
last pick is a chip. Every 25 ticks (from T+2) it takes the last picked chip out of the folder (the Regular chip's
clears BattleState+0x17, as OK taking it does: the next screens deal no Regular chip, while this screen's front
slot keeps its Regular bit); when the last pick
isn't a chip (or none is left), the folder is compacted, the scrapped chips are put in its first holes (so at its
end, in pick order), the dealt slots show the chips now at the front, and the button is used up. **[dumps]**

### 3.7 ChpShufl's re-deal (`sub_80271F8`, state 0x28)

With ChpShufl (NaviCust, NaviStats+0x60 = 1; MegaMan, battle modes 0, 5, 8, 0xA, 0xB, not DustCross) slots 8/9 are
one re-deal button (`sub_80280E0`), usable once a screen (its uses are 1 − +0x16, the screen's re-deals so far,
which each opening zeroes). A on it while it is selectable (`sub_8028DD6`; sound 0x182) enters state 0x28:

- **First tick** (`sub_802721C`): +0x40 = 0; `sub_8029788` shuffles the chips it re-deals into a new order (below)
  with the console's RNG1; the button's state = selected (in use); availability (`sub_8028E32`).
- **Then every tick** (`sub_802723A`): +0x40 += 1; every 4th: on the 8th (+0x40 = 32) `sub_802983C` writes the new
  order into the folder, +0x16 += 1, the button's uses − 1 (state selectable if any are left, else grayed), and the
  grid takes keys again (state 4); before that, `sub_8029688` shows the chips shuffled once more: the same chips,
  shuffled in the folder itself with RNG1 (seven times; the final order doesn't depend on them, the RNG does).
  Either way availability again, and sound 0x113.

**Which chips** (the three routines walk the folder the same way): over the first hand-size slots (+6), each chip
slot is the next folder entry, re-dealt unless it is picked or the Regular chip (slot +4 bit 0); then as many more
entries as the screen had chips beyond the hand size (+5 − +6), skipping the tag pair (two entries at +0x45 while
+0x44 is set) where the walk meets it. Each shuffle is `sub_8000D12` over them all, n swaps of two entries (two
draws each) for n chips (the table that would shuffle the dealt part apart, `byte_80298C8`, is all zeros).

A quirk comes with the walk: with NumbrOpn's ten chips the button covers slots 8 and 9, so the walk counts eight
dealt entries and leaves the folder's last two out. The pair's index is kept up as OK takes chips
out (§1), so the walk skips the pair on later screens too; a pair in the hand is dealt again like any chips (+0x44 is
clear by then). Where the tag pair straddles the walk's end the original runs on past the folder (a buffer overrun);
the port stops at the folder's end. A netbattle can't get there: the chips left less the pair's index never change
from what the shuffle made them (OK takes one off both for every chip, the scrap changes neither), at least 10. With
NumbrOpn the walk would count eight dealt entries, but NumbrOpn and ChpShufl can't both be installed (docs/engine/
unverified.md).

Port: `Phase::Redealing`, `Screen::redeal` (custom/screen.rs); the index's upkeep is in `Side::confirm`
(custom/mod.rs). **Verified** on three chip-lab scenarios, every frame of each: `navicust/chpshufl-redeal` (side 0
re-deals on three screens running, with a Regular chip it never picks and the tag pair beyond the hand, picking two,
three and five of the re-dealt chips: the second and third re-deals only match with the index kept up),
`navicust/chpshufl-redeal-tags-dealt` (the pair in the first hand is taken apart; a chip picked before the re-deal
stays) and `navicust/chpshufl-redeal-shaken` (the second screen, after the other side's Beast Out shook both cameras
for 60 ticks). The console's modeled RNG1 keeps step with the recordings' throughout (it leaves them for a frame at a
time, or for as long as a camera shake lasts, where the trace's sample already has the next frame's draws: the
emotion window's flicker, the re-deal's shows, the shake). The custom screens' check on their own (§7) doesn't
simulate the draws outside the screens; it takes the recording console's RNG1 from the trace on every frame, so
all twelve player-screens of the three scenarios match there too. Unit tests: custom/tests.rs.

## 4. Beast Out and Crosses

**Who gets what** (per player; the original reads the local save):

- The Beast Out button (slot 11) exists for MegaMan with Beast Out unlocked (event flag 0xE0) and event flag 0x163
  clear (0x163 marks a link navi operated: it is raised and lowered with the navi, so MegaMan never has it). It is
  selectable unless the navi is worn out (emotion 5), already in a Beast form (form ≥ 0x0B), or tired (emotion 1,
  the Beast Out counter spent) without having gone Beast Out this round.
- The Cross window offers the version's five Crosses that the save owns (Gregar's event flags 0xE2-0xE6, Falzar's
  0xE7-0xEB), not used this round, and not the navi's starting form; none when worn out.
- Recordings carry each console's flag bytes (the setup's `unlock_flags`); older ones, recorded with finished
  saves, read as everything unlocked. The lab's `custom/one-cross-owned` and `custom/no-beast-out` are saves with
  fewer.

**Keys.** In the window: UP/DOWN (repeat) move with wrap-around, A chooses (once), B closes, START closes with the
grid cursor on OK, R describes. With a Cross chosen, UP/DOWN/A are ignored; B on the grid with nothing picked
un-chooses it. Beast Out goes first in the pick list once its animation ends, so B takes it back last.

**Result** (at OK, `sub_8029344`, `sub_802937A`):

| Choice | Form sent |
|---|---|
| Beast Out, tired | Beast Over (Gregar 0x17, Falzar 0x18) |
| Beast Out from the base form | Beast Out (Gregar 0x0B, Falzar 0x0C) |
| Beast Out in a Cross | the Cross's Beast form (Cross + 0x0C) |
| Cross i | Gregar 1 + i, Falzar 6 + i; + 0x0C when in a Beast form |

The round remembers Beast Out and each Cross used (`dword_20349A0`, `custom::RoundMemory`). No NaviStats change
here: the form changes at the turn's start (battle-flow.md §3.4.1). The transform record's +3 (a turn count) is
never read in battle and is not modeled; its +4 ("Cross change") is always 0xFF (its only writer is dead code).
All 20 recorded transformations match **[dumps]** (Crosses 2, 5, 6, 7, 0x0A, Beast Out 0x0B, 0x0C, 0x11).

### 4.1 nettai's extension: a setup's Cross list

**Not the original's.** The original's window offers only its version's five Crosses that the save owns. A
nettai setup can name the Crosses instead: `Unlocks::cross_list` (`custom::CrossList`), up to five forms, of
either game, in the order the window lists them. nettai-frontend's live play uses it to offer five of all ten
Crosses (docs/frontend.md §2). Without a list (every recording, the chip lab, the netplay stand-in) nothing below
applies and the screen is the original's.

- **Places.** The window's entries, the Cross chosen and the round's record of Crosses used go by a Cross's place
  among the player's Crosses (`CrossWindow::offered`, `RoundMemory::crosses_used`): the version's Cross number
  without a list, the place in the list with one (`Unlocks::cross_at`, `owns_cross`). Everything else is as above:
  a Cross used this round and the navi's starting form aren't offered, A chooses, B takes it back, the face is the
  Cross's, OK sends the Cross's form (its form in Beast Out when the navi is in a Beast form).
- **What a list offers.** Its entries that are Crosses (a form of kind `cross`; anything else is never offered),
  and, while the navi is in a Beast form, only the Crosses whose Beast it is (that Beast's game's: their forms in
  Beast Out are of that Beast).
- **Beast Out from a Cross** takes the navi to that Cross's form in Beast Out, whichever game the Cross is from
  (`Unlocks::beast_form`): a Falzar player in HeatCross becomes HeatCross Beast, of Gregar's Beast. Beast Out from
  the base form is the player's own game's Beast.
- **The Beast's game** (`Unlocks::beast_game`) is the game of the Beast the navi goes into or is in: the player's
  game, but a form of the other game's (one of its Crosses, or one of its Beast forms) is that game's. Beast Over
  (Beast Out when tired) is that game's, and so is the screen's Beast Out roar (§9); a frontend draws the Beast Out
  button, its picture in the chip window and the BeastOut chip's picture of that game. The Beast form itself
  brings its own buster, charged shot, Beast rush and face.
- **The Cross itself** is its form definition, whatever the player's game: its sprite and palette, element and
  weakness, buster and charged shot, chip bonuses, face, and what a weakness hit breaks it to. A frontend draws its
  name in the window from that Cross's own game's pictures (docs/frontend.md §3).
- **R** describes the Cross under the cursor by its form (`CrossWindow::hovered`, through `Unlocks::cross_at`):
  the chatbox's lines (the form's `description_lines`) and the text a frontend shows are that Cross's own. (Until
  2026-10-02 both took the version's Cross in the hovered place, so a list mixing both games showed the
  descriptions of the player's own game's Crosses in order.)

## 5. OK: the hand (`sub_8029110`)

Built on the OK tick from the picks in order, with each chip as checked (§3.4):

1. For each picked chip: an entry (id, damage from `sub_80109A4` for this player now, the Regular-chip bit); the
   raw pick (`code << 9 | id`) goes in the hand's `selection`. Beast Out adds no entry. Navi chips (ids ≥ 0x190)
   are noted for the round.
2. **Program Advance** (`sub_8029520`): for each start position (at least 3 chips left), the 43 recipes of
   `off_802BCB0` in order (`Content::program_advances()`):
   - a sequence recipe: those ids in that order, any codes;
   - a code-run recipe: 3 of one chip with codes going up by one in pick order; one `*` stands in for the code
     it needs (`[A,B,C]`, `[A,*,C]`, `[*,B,C]`, `[A,B,*]` match; `[*,A,B]`, `[A,A,A]` don't).

   The first match not formed yet this round is formed (once per round per player, `dword_203CA48`); the recipe's
   entries become the Program Advance (damage recomputed; Regular if any part was). The veto `sub_8029328` can't
   fire (no slot carries the flags it tests).
3. **Modifiers** (`sub_8029224`; `ChipData::modifier`, below): from the second entry on, a modifier right after a
   chip it applies to folds into that chip and leaves the hand; chained ones stack.

   | Modifier | `modifier` | Applies to | Effect |
   |---|---|---|---|
   | Atk+10 (0xC0), Atk+30 (0xC3) | `attack_plus` | damaging chips (flag 2) | attack bonus += its damage |
   | Navi+20 (0xC1) | `navi_plus` | navi chips (flag 4) | attack bonus += 20 |
   | WhiCapsl (0xB8) | `paralyze` | damaging chips | paralyzes (modifier bit 2) |
   | Uninstll (0xB9) | `uninstall` | damaging chips, not dimming chips | uninstalls (bit 4) |

4. The hand: ids, damage, attack bonus, charge bonus 0, `selection` (the raw picks, not changed by steps 2-3),
   turn = BS+7 − 1, modifier bits (bit 1 = the Regular chip). With nothing picked, nothing is sent and the
   player's hand stays; with only Beast Out picked, an empty hand is sent and replaces it.
5. The picked chips leave the folder (`sub_80293F8`): each entry becomes a hole (the next opening closes them
   up); the Regular chip's clears BattleState+0x17; and while the folder has its tag pair (+0x44), each takes one
   off the pair's index (+0x45, a byte: §1, §3.7).

**5.4 Class counts** (`sub_802A4FC`, on the sending tick): each raw pick's class (standard, mega, giga; invalid
chips don't count) is added to the player's counts for the round; the invalid-chip rule reads them from the next
screen on.

All 40 sent hands (every screen, both players; Regular chips and invalid-free) match **[dumps]**. The chip lab
records the rest (`custom/pa-*`, `custom/modifier-*`, and a recording of every recipe under `pa/`), and every one
matches: a code run with a `*` first, in the middle or last forms, `[*,A,B]`, `[A,*,*]` and `[A,A,A]` don't; a
sequence recipe out of order doesn't; a recipe between two other chips forms from the second pick; a recipe picked
again on the next screen stays three chips (once a round); a modifier after a Program Advance folds onto it;
Atk+10 twice, Atk+10 with WhiCapsl and Atk+30, and Navi+20 with Atk+30 on a navi chip all fold; a modifier picked
first, or after a chip it doesn't apply to (Atk+10 after Invisibl, Navi+20 after AirShot, Uninstll after Roll, which
dims: `custom/modifier-uninstll-dimming`), stays a chip of its own.
A recipe with the Regular chip as a part carries its bit (`custom/pa-regular`), and one made of the tag pair and
the Regular chip forms from the first deal (`pa-tags`). The folder's upkeep (step 5) is in `custom/folder-odd-picks`
(picks from the middle of the hand, screen after screen) and `folder-runs-out` (30 chips, five a screen, then two
screens with none).

## 6. Closing, sending, and the exchange

For a player who presses OK on tick **C**:

| Tick | |
|---|---|
| C | hand built, folder entries taken out, transformation noted |
| C+1 … C+10 | slide-out (`sub_8026BF4`); C+1 clears the status bit |
| C+11 = **P** | send (`sub_8026DC4`'s first tick): the hand, the player's NaviStats as they are now, the transformation; class counts; the gauge restarts (unless the 15th screen or later) |
| P+1 … P+50 | the 50 words go out (last the magic word) |
| **A = P + 50 + delay** | the result has arrived (on both consoles, the sender's own included) |

With a Program Advance, its animation (`sub_8026DB0`) runs after the slide-out and **P = C + 177 + 8n** for n
picked chips (the lab's recordings with three, four and five picks match).

The fight resumes when both results are in: on **K = max(A₀, A₁)** both hands (with chips) and both NaviStats
are installed (`sub_800B3D8`), the transformations wait for the turn's start; on K+1 the gauge restarts again,
both navis' Beast Out check delay is set to 1 and the mode goes to fighting. The status bit's clearing reaches
both consoles on C+6.

All 20 screens fit this with no exception **[dumps, both consoles]**:

| Screen (opened) | OK (P0 / P1) | Fight resumes |
|---|---|---|
| machgun 208 | 375 / 462 | 528 |
| machgun 1360 | 1627 / 1709 | 1775 |
| soundmod 208 | 2938 / 1094 | 3004 |
| soundmod 5045 | 6163 / 6655 | 6721 |
| soundmod 8183 | 9030 / 10000 | 10066 |
| soundmod 11626 | 13219 / 13022 | 13285 |
| soundmod 15284 | 16986 / 16117 | 17052 |
| soundmod 19152 | 19913 / 20540 | 20606 |
| soundmod 22248 | 25507 / 24145 | 25573 |
| soundmod 26118 | 27834 / 27432 | 27900 |
| soundmod 29686 | 30173 / 30729 | 30795 |
| soundmod 32200 | 32883 / 33175 | 33241 |
| soundmod 34561 | 36212 / 35233 | 36278 |
| soundmod 37259 | 38047 / 39003 | 39069 |
| soundmod 40561 | 41296 / 42434 | 42500 |
| soundmod 42658 | 43603 / 43865 | 43931 |
| soundmod 45254 | 46691 / 45906 | 46757 |
| soundmod 49191 | 50193 / 50520 | 50586 |
| soundmod 52435 | 53560 / 53739 | 53805 |
| soundmod 55823 | 56508 / 56671 | 56737 |

(OK ticks are the players' own presses: the recorded input records show them 4 frames later.)

## 7. Verification

- **Unit tests** (in this repository, with made-up chips): the shuffle on four recorded folders, the joypad's
  repeat, the builder (Program Advances, modifiers, class counts), and scripted screens (dealing and layout, the
  timeline from opening to sending, the selection rules, invalid chips, Beast Out and a Cross for both games,
  DustCross's scrap, hand sizes, SELECT), and a setup's Cross list (§4.1: either game's Crosses offered and
  chosen, Beast Out from the other game's Cross with its roar and Beast Over, the window in a Beast form, entries
  that aren't Crosses). nettai-frontend's `a_falzar_player_plays_a_gregar_cross` plays one through on content/bn6:
  HeatCross chosen by a Falzar player, its form, element, buster and charged flame, then Beast Out from it into
  HeatCross Beast with that form's weapons.
- **Golden traces** (`trace::run_round`, verification workspace): the traces' recorded buttons drive the
  engine; each player's custom screen runs when the trace has their folder (setup `folders`), otherwise that
  player's results come from the recording (`TickEvents::recorded`). Each frame compares, besides the rest of the
  battle, both hands and the received status bits. With the recording console's folder only (the current traces)
  and with both consoles' folders added, every frame up to the existing floors matches (machgun 1074 and 1331;
  soundmod 4513, 6284 and 2566), also under rollback at latencies 0-10.
- **Every screen on its own** (`trace::check_custom_screens`): the custom screens alone, over the whole trace
  (the fight isn't simulated; each screen reads its navi's stats from the trace). With both consoles' folders all
  40 player-screens match: the OK tick, the hand as installed, the transformation, the status bit's clearing, and
  the tick the fight resumes. The check reads emotions from the mood only, so a tired navi isn't seen, and
  doesn't check damage from formulas. The recording console's RNG1 (a re-deal's) is the trace's, frame by frame;
  the other console's only has the draws the screens and the main loop make.
- The recorded traces carry only the recording console's folder. `folders`, `joypad_phases` and `game_versions`
  in setup lines come from recording both consoles.
- Matches recorded by Tango's first netplay engine (2022) ran each console alone, with no link cable: each tick
  the console found both players' packets in its receive buffers a tick after they were built. Their traces have
  the recording console only (no `folders`: the other player's screens come from the trace) and a `link_delay`
  of 1 in the setup; every round of them matches at that delay (`RoundSetup::link_delay`), and none at the
  cable's 4.

## 8. Not ported or not verified

- **Each console's RNG1** (ChpShufl's re-deal, §3.7). The re-deal draws from its console's own RNG1, which the
  original never shares over the link. During a battle it advances once a frame (the main loop's `GetRNG1`, after
  the battle's), twice a tick while the console's camera shakes (`camera_doShakeEffect_80301e8`), once when the
  emotion window's timer runs out on a bugged navi (`sub_801CC94`), and by the re-deal itself. All of that is a
  function of the round's start, the shared simulation and the player's own buttons, so the port simulates it
  exactly, per player, in `Battle::consoles` (`crate::console`, part of the snapshot and the digest):
  - the seed is the console's RNG1 on the round's first battle frame, after its folder shuffle
    (`PlayerSetup::console`, with the tag pair's place and the save's emotion window glitch). In netplay it is part
    of the setup the peers share; the frontend's and the netplay stand-in's setups carry on from their folder
    shuffles' RNG. A set's next round needs the console RNG as its init left it (§8.3 of rollback.md);
  - the camera: two channels (`camera_initShakeEffect_80302a8`'s primary, `sub_80302B6`'s secondary), the primary
    first unless the battle is paused without dimming while player 0's status byte (BattleState+0x14) has neither
    bit 0 nor 2 (`sub_80269D0`); run each running tick after the objects. The shared simulation starts shakes on
    both consoles (`Battle::shake_camera`, content's `battle.shake_camera`; the form changes and GroundCross's rock
    barrage in Rust); the custom screen's Beast Out (tick 2 of state 0x48, tick 17 of 0x44) on its own. The jitter
    (`CameraShake::jitter`) is there for a frontend to draw;
  - the emotion window (HUD task bit 14): started by the intro's HUD setup (`sub_800927C` → `sub_801E5F8`, first
    check 120 ticks on), stopped by the win, loss and draw states; every 20 ticks it checks the console's own navi,
    and flickers once or twice (the draw) when the navi has a NaviCust bug (`sub_800FE52`) or, for MegaMan, when the
    save's event flag 0x1720 is set (`ConsoleSetup::emotion_window_glitch`; BugFix clears it on both consoles,
    `Battle::clear_emotion_window_glitch`, from BugFix's controller).

  **Fidelity.** Measured against the recording console's RNG1 column (a scratch probe; the trace comparison doesn't
  check RNG1): exact on every frame the engine reproduces of machgun, soundmod (all three rounds up to their
  floors) and 3618 of the chip lab's 3621 rounds. The limits:
  - link stalls: frames on which a console waits for the link still draw (the main loop) but don't tick. They are
    the console's own network timing and can't be reproduced; the port's link never stalls, and nor did the
    recordings (their frames and ticks stay a constant apart through each round);
  - the save's event flag 0x1720 is in the setups of traces recorded since the coverage push
    (`emotion_window_glitches`, both consoles'); bn6-compat reads it as clear in older ones. The NaviCust sets it
    at load when a bug's routine ran (`sub_813CBCC`), also for the bugs the navi's stats don't show in battle (the
    support bug, the result bug): the lab's `navicust/bug-support` has it set and keeps step (the three support
    scenarios had it, unintended, until their parts were moved off the grid's outer ring);
  - the other console's RNG1 (and tag pair) isn't recorded either: bn6-compat gives it 0 (and none), which only a
    re-deal on that player's screen would read;
  - shakes are the content's to start: it calls `battle.shake_camera` where the original calls
    `camera_initShakeEffect_80302a8`. The viruses' are missing with the viruses, which aren't a netbattle's;
  - the run-away check (`sub_8026F1A`, one RNG1 draw on the answer) isn't ported: it runs only with battle effects
    0x20, which a netbattle doesn't have (L gives the run message there, §3.5; the lab's `custom/run-message*`).
- **Chip 0x13F picked as a chip** (state 0x44, `Phase::BeastOutChipChosen`): the chip lab's BeastOut scenarios match.
- **Tag chips**: laid out and shuffled; only ChpShufl's re-deal reads the tag pair (§3.7).
- **Link navis** (NaviStats+0x29 ≠ 0): their own chip in slot 9 is picked in the chip lab's `navis/` scenarios, whose
  custom screens match.
- The builder's stale-register write on the first fold (chips.md §2.4) is not reproduced.
- Battle mode 1 paths, tutorials, escape, the Beast Link Gate (state 0x40).
- NaviStats +0x0A (CustomLevel) and +0x63 change during fights in soundmod (6878, 10341, 36618, 54134); the hand
  size follows them, but the fight engine doesn't model those changes yet.

## 9. Presentation: what the screen shows

The simulation above is all the battle needs. What each console draws of its own screen is presentation: the
frontend draws the local player's (nettai-render `custom`, docs/frontend.md §3), from the `Screen` and from
`Screen::look` (`custom/look.rs`), the part of the original's control block at `0x020364C0` and of its VRAM that
the screen's drawing reads. The state digest leaves `look` out, like a sprite's `Look`; nothing the simulation
reads depends on it.

- **The frame counter** (`+0x40`). While the window slides in (`sub_8026B04`) it is the window's offset, 0x78 less
  12 a tick; sliding out (`sub_8026BF4`), 12 more a tick. While the chips are chosen (`sub_8026CCC`) it counts the
  ticks, from 0 when the window is in, and the cursor (`sub_8028820`) shows its second frame on ticks with bit 3
  set. Beast Out's states (`sub_802770C`, `sub_80275EC`) zero it and count their own timers with it, and choosing
  goes on from there.
- **The emblem's spin** (`+0xF`): a pick (`sub_8028CCC`) and Beast Out (`sub_8027738`) set it to 1; each tick that
  draws the emblem (`sub_8029C08`) steps it, through 0x14, nudging the sprite and setting its affine matrix from
  `byte_8029CAC` (an angle and a scale; the matrix stays as last set). The emblem isn't drawn while the window is
  further out than 0x67.
- **What the window's tiles hold.** The original copies tiles into VRAM when something changes and not otherwise,
  so the look keeps what it copied: the chip window's last draw (`sub_8028476`: the slot under the cursor, the
  picks then, and the last chip drawn, whose element's colors palette 11 keeps from screen to screen within a
  round); the slots as last drawn (`sub_8028250`, on opening and after every pick or take-back, so the chips OK
  takes out of the folder stay drawn while the window slides out); the picked column's icons (`sub_80281D4`: a
  pick's chip as checked, Beast Out's the BeastOut chip's, and after a Beast Out the picks again in their new
  order, unchecked).
- **The Regular chip's frame** (`sub_802899C`): drawn on choosing ticks while the folder has its Regular chip, its
  tiles changed on ticks whose counter is a multiple of 8.
- **The last turns' block** ("FINAL TURN", `sub_8029D34`): in a netbattle's 15th turn on, off 4 frames of 32;
  hiding or sliding out takes it off.
- **The face** the emotion window shows: Beast Out's Beast form from Beast Out's 53rd tick (`sub_802A040`; Beast
  Over's for a tired navi), the BeastOut chip's from its 68th; taking it back (`sub_802A0EC`) takes the face back.
- **The screen fade** the screen runs on its console: Beast Out's modes 0x64 (half way to black, background
  palettes 0-13 and sprite palettes 0-10) and 0x60 back, at 8 a frame. The original runs them on the console's
  one fade record; the port keeps them per screen, apart from the battle's `Fade`.
- **The window's map variant**: with the Cross tab while MegaMan has a Cross he owns and hasn't used this round
  (`sub_8029EC8`, at the opening).
- **The sub-screens' counters and sprites.** The window's frame counter (`+0x40`) is the sub-screens' timer too, and
  the look follows it through them: the Cross window's opening counts its ticks (its map steps every 3, the
  frontend's), the window (`sub_802794A`) and its closing count from 0, a Cross's choice counts 16 ticks and holds
  at 0 through its fades, the scrap counts from 24 (`sub_8027434`), the re-deal from 0. Each of these states draws
  the emblem and the Regular chip's frame every tick (`sub_80279C8`, `sub_8027406`, `sub_80271F8`, `sub_8027A58`),
  the Cross window its own cursor (`sub_80289E4`); closing the Cross window, putting a Cross on and the re-deal's
  end draw the chip window again (`sub_8028476`). A Cross's choice whitens the screen on the screen's fade (mode 4
  then 0, at 0x20 a frame) and, once white, shows the Cross's face (`sub_802A088`: its Beast form's in Beast Out).
  The Cross window's names and palette 10 are the frontend's, from the screen's Cross window.
- **The scrap**: each chip scrapped takes its icon off the picked column (`sub_80281D4`) and draws the chip window
  again; the slots' tiles keep the look they were last drawn with (`look.slot_picked`: a picked chip's slot shows
  the empty icon until the slots are drawn again, on the scrap's last tick).
- **The Program Advance animation**: its own counter (`word_2036660`+0xC, every tick, from 0 when the names begin)
  steps the names' colors through three sets every 16 ticks (`sub_802B9E4`: background palette 10's first four,
  `byte_802BA48`), and the screen fades a quarter of the way for it (0x14, and 0x10 back, at 8 a frame). The names
  themselves (the picks', the recipe's taken off, the Program Advance's in their place) are the frontend's, from the
  animation's step and timer, the Program Advance formed and the hand built at OK.
- **The sounds** a tick made (`Drawn::sounds`, `ScreenSound`), in the order its states call `PlaySoundEffect`;
  the battle plays each by its role for the screen's player only (docs/engine/audio.md §1 lists them). One is by
  the console's version: Beast Out's first sound (`sub_802774C`, the BeastOut chip's `sub_8027624`) is 0x193 on
  Falzar and 0x191 on Gregar (`custom_beast_out_falzar`, `_gregar`). With a setup's Cross list it is by the Beast's
  game (§4.1: a Falzar player in HeatCross roars Gregar's).

What isn't in the look derives from the screen: the window's place (the phase's ticks), the slots' kinds and
states, the cursor, the picks, and from the battle the camera's jitter, which in Beast Out's states, the Cross
window's, the scrap's and the re-deal's moves the HUD layer too (`sub_80269E2`, `sub_8030158`).

**Dark chips.** A chip whose record has bit 0x20 of its flags (the `dark` flag of chip definitions; the dark chips
of the earlier games) gets three things on the screen, all ported though no chip record of the US ROM has the bit
(unverified.md lists them as unreachable):

- **The cursor's start** (`sub_802806C`, the last step of the opening's layout): a cursor on the first slot goes to
  the first chip slot (0-9, the link navi's chip's too) whose chip, as the class limits count it (`sub_802A53C`: an
  over-limit Mega or Giga chip is the invalid chip; the code isn't checked), has the flag. This is simulation
  state: the cursor decides the picks.
- **The window's frame**: the chip window's frame palette is by the chip's class (`byte_86E587C`: standard, Mega,
  Giga), a class past Giga's the standard one, and a dark chip of the first three classes the fourth, dark one.
- **The hover** (`sub_802A2B0`, after every tick's state; `+0x12` its state, `+0x13` its step): while the screen
  chooses chips or shows a chip's description (states 4 and 0x18, not a Cross's) and the cursor's slot holds a
  chip that, as it counts in a selection (`getChipID_802A54E`), has the flag (`sub_802A394`), the screen
  darkens: fade 0x54 (background palettes 0-8, sprite palettes 0-9, to 0x50) on the console's first fade record
  and 0x5C (background palettes 9-13, the window; sprite palettes 10-13, the screen's sprites; to 0x30) on its
  second (`loc_8006274`), both at 0xA a frame. Each tick after, until the second record's fade is done, the music
  (player 31) and the screen's player (22) change volume a step: the music down `byte_802A3F4` (0x100, 0xE0,
  0xC0, 0xA0, 0x80, 0x80), the screen's player up `byte_802A400` (the same, reversed); 5 steps, the fade taking
  5 frames. When the cursor leaves (or the state changes), fades 0x50 and 0x58 take both back and the volumes go
  the other way, 6 steps (a fade toward clear holds its first frame). The volumes are cues
  (`SoundCue::ScreenVolume`) for the screen's player; the closing's `sub_802A3CC` sets both to 0x100 again
  (`RestoreVolume`), and `sub_80062EC` clears both fade records. A counter at `+0x14` the routine steps changes
  nothing. The look keeps the hover (`ScreenLook::dark`) and the second record (`window_fade`).
