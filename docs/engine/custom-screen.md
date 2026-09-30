# The custom screen (BN6 US Falzar, link PvP)

Between turns each player deals chips from their folder, picks some with their joypad (and maybe Beast Out or a
Cross), and presses OK. Their hand is built and sent over the link, and the fight resumes once both players'
results are in. This document is the spec for the port's custom screen, `bn6-battle/src/custom` (ruleset layer),
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
| Save data: owned Crosses, Beast Out unlocked, game version | `custom::Unlocks` in `RoundSetup::players` |

Nothing in the custom screen depends on which side is "local": which screen a frontend draws is presentation.
`TickEvents` carries only `link_closed` (the end of the round) and, for checking against recordings that lack a
player's folder, that player's recorded results (`TickEvents::recorded`, §7).

**RNG.** The custom screen draws from neither RNG stream **[dumps]**: RNG2 never moves during a screen, and the
RNG1 draws seen during screens are presentation (the emotion window's bug flashes, `sub_801CC94`, and the Beast
Out camera shake). The only RNG in this area is the folder shuffle at the round's init (RNG1, §1), and ChpShufl
(RNG1, not ported, §8).

**Game data.** The screen reads chip records, the Program Advances and the link navis' own chips through
`custom::Library` (today `BuiltIn`, the engine's tables). The slot grid and its scan lists are layout tables in
`data::custom`.

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
3. BattleState+0x17 = 1 if there is a Regular chip (`BattleFolder::regular_pending`); +0x44/+0x45 note the tag
   pair (only ChpShufl reads them; not ported).

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

**Slots.** 12 slots: 0-4 the top row, 5-9 the bottom row, 10 = OK (right end of the top row), 11 = the button
under OK. The grid starts from `dword_802A7CC` (`data::custom::SLOT_TEMPLATE`); dealt chips fill slots 0, 1, …;
slot 11 is the Beast Out button when the player has it (§4); slots 8/9 are DustCross's scrap button (form 0x0A or
0x16) or ChpShufl's re-deal button; a link navi's own chip goes in slot 9 once a round. Then `sub_8027F42` points
every neighbour that is an empty slot at the next slot present along fixed scan lists
(`data::custom::LEFT_SCAN_*`, `RIGHT_SCAN_*`): with 5 chips, LEFT from slot 0 wraps to OK, RIGHT from OK to slot
0, OK and slot 11 are each other's UP/DOWN, and the chips have no UP/DOWN. The cursor starts on the first slot
present (slot 0 when a chip was dealt). All neighbour bytes of all openings match **[dumps]**.

## 3. Choosing

### 3.1 The joypad

`eJoypad` (`main_static_80003E4`, once per frame): held, pressed (down now, not last frame), and **repeat**: a
held button repeats on the second frame of a hold, then from its 17th frame on every fifth frame, on the frames a
console-wide counter (0-4, +1 a frame) reads 0. That counter is each console's own: `PlayerSetup::joypad_phase`
(the recordings' consoles read the frame number mod 5) **[dumps]**. `input::Joypad`.

### 3.2 Keys (`sub_8028B74`, state 4)

One action per tick, in this order:

1. UP/DOWN (repeat): UP from a top-row chip or OK opens the Cross window (§4) when the navi is MegaMan, Beast
   Out isn't picked and a Cross is offered; otherwise the slot's vertical neighbour.
2. LEFT (repeat), then RIGHT (repeat): the neighbour. A missing neighbour still uses up the key.
3. A (pressed): the slot's action (§3.3).
4. B: take back the last pick (§3.4).
5. START: the cursor goes to OK (it doesn't press it).
6. SELECT: hide the window (§3.5).
7. R on a chip: its description (§3.5).
8. L: "no time to run away!" (§3.5).

Cursor movement never skips greyed or picked slots. Verified on every state-4 tick of both players in both
replays (37.5k ticks: 692 moves, 174 picks, 49 take-backs, 88 Cross windows, 40 OKs, 10 scraps…) **[dumps]**.

### 3.3 A

| Slot | Action |
|---|---|
| A chip | If selectable and fewer than 5 picks: pick it (`sub_8028CCC`). Chip 0x13F ("BeastOut" as a folder chip) would go to state 0x44 (not ported) |
| OK | Build the hand (§5) and slide out (§6); works with nothing picked |
| Beast Out | If selectable and fewer than 5 picks: pick it (it counts as a pick) and play its animation (§4) |
| Scrap (slots 8/9) | If usable: scrap (§3.6) |
| Re-deal (8/9) | ChpShufl: not ported (§8) |

### 3.4 What can be picked (`sub_8028E32`)

After every pick and take-back the chip slots are greyed or not (`screen::update_availability`). The picked
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
| 0x18 chip description (`sub_8026E4C`) | R at T on a chip | the chatbox takes any key from T+6; after a key at P, input from P+6 **[dumps, 5 cases]** |
| 0x1C run message (`sub_8026E98`) | L at T | A or B once the message has printed; P+6 **[unverified: the printing time is an estimate, 72 ticks]** |
| 0x4C Cross window opening | UP at T | window from T+13 |
| 0x50 Cross window closing | B or START in the window at T | T+7 |
| 0x5C Cross chosen | A in the window at T | T+35 **[dumps, 15 cases]** |
| 0x48 Beast Out picked | A on Beast Out at T | T+71 **[dumps, 5 cases]** |
| 0x38 DustCross scrap | A on the scrap button at T | T+4+25k for k chips scrapped **[dumps, 13 cases]** |

The chip description's chatbox also closes on B held for 10 frames; that is not ported.

### 3.6 DustCross's scrap (`sub_8027406`)

In DustCross (form 0x0A, or its Beast form 0x16) slots 8/9 are one scrap button, usable once a screen when the
last pick is a chip. Every 25 ticks (from T+2) it takes the last picked chip out of the folder; when the last pick
isn't a chip (or none is left), the folder is compacted, the scrapped chips are put in its first holes (so at its
end, in pick order), the dealt slots show the chips now at the front, and the button is used up. **[dumps]**

## 4. Beast Out and Crosses

**Who gets what** (per player; the original reads the local save):

- The Beast Out button (slot 11) exists for MegaMan with Beast Out unlocked (event flag 0xE0) and event flag 0x163
  clear. It is selectable unless the navi is worn out (emotion 5), already in a Beast form (form ≥ 0x0B), or tired
  (emotion 1, the Beast Out counter spent) without having gone Beast Out this round.
- The Cross window offers the version's five Crosses that the save owns (event flags 0xE7-0xEB), not used this
  round, and not the navi's starting form; none when worn out.

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

## 5. OK: the hand (`sub_8029110`)

Built on the OK tick from the picks in order, with each chip as checked (§3.4):

1. For each picked chip: an entry (id, damage from `sub_80109A4` for this player now, the Regular-chip bit); the
   raw pick (`code << 9 | id`) goes in the hand's `selection`. Beast Out adds no entry. Navi chips (ids ≥ 0x190)
   are noted for the round.
2. **Program Advance** (`sub_8029520`): for each start position (at least 3 chips left), the 43 recipes of
   `off_802BCB0` in order (`data::custom::PROGRAM_ADVANCES`):
   - a sequence recipe: those ids in that order, any codes;
   - a code-run recipe: 3 of one chip with codes going up by one in pick order; one `*` stands in for the code
     it needs (`[A,B,C]`, `[A,*,C]`, `[*,B,C]`, `[A,B,*]` match; `[*,A,B]`, `[A,A,A]` don't).

   The first match not formed yet this round is formed (once per round per player, `dword_203CA48`); the recipe's
   entries become the Program Advance (damage recomputed; Regular if any part was). The veto `sub_8029328` can't
   fire (no slot carries the flags it tests).
3. **Modifiers** (`sub_8029224`, `builder::MODIFIERS`): from the second entry on, a modifier right after a chip it
   applies to folds into that chip and leaves the hand; chained ones stack.

   | Modifier | Applies to | Effect |
   |---|---|---|
   | Atk+10 (0xC0), Atk+30 (0xC3) | damaging chips (flag 2) | attack bonus += its damage |
   | Navi+20 (0xC1) | navi chips (flag 4) | attack bonus += 20 |
   | WhiCapsl (0xB8) | damaging chips | paralyzes (modifier bit 2) |
   | Uninstll (0xB9) | damaging chips, not time freezes | uninstalls (bit 4) |

4. The hand: ids, damage, attack bonus, charge bonus 0, `selection` (the raw picks, not changed by steps 2-3),
   turn = BS+7 − 1, modifier bits (bit 1 = the Regular chip). With nothing picked, nothing is sent and the
   player's hand stays; with only Beast Out picked, an empty hand is sent and replaces it.

**5.4 Class counts** (`sub_802A4FC`, on the sending tick): each raw pick's class (standard, mega, giga; invalid
chips don't count) is added to the player's counts for the round; the invalid-chip rule reads them from the next
screen on.

All 40 sent hands (every screen, both players; Regular chips and invalid-free) match **[dumps]**. No recording
forms a Program Advance or folds a modifier: those are **[code]** and covered by unit tests.

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
picked chips **[code, unverified]**.

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
  DustCross's scrap, hand sizes, SELECT).
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
  doesn't check damage from formulas.
- The recorded traces carry only the recording console's folder. `folders`, `joypad_phases` and `game_versions`
  in setup lines come from recording both consoles.

## 8. Not ported or not verified

- **ChpShufl** (NaviCust, NaviStats+0x60): the re-deal button is laid out, pressing it panics (not implemented).
  It shuffles with the console's RNG1, which the simulation doesn't have; a port would give each player an RNG
  stream in the setup.
- **Chip 0x13F picked as a chip** (state 0x44): panics (not implemented); not seen in netbattles.
- **The run message's timing** (L): an estimate; the chatbox's text timing isn't ported.
- **The Program Advance animation's length**: from the code, not a recording.
- **Tag chips**: laid out and shuffled; the tag flag is only read by ChpShufl.
- **Link navis** (NaviStats+0x29 ≠ 0): their own chip in slot 9 is from the code only.
- The builder's stale-register write on the first fold (chips.md §2.4) is not reproduced.
- Battle mode 1 paths, tutorials, escape, the Beast Link Gate (state 0x40).
- NaviStats +0x0A (CustomLevel) and +0x63 change during fights in soundmod (6878, 10341, 36618, 54134); the hand
  size follows them, but the fight engine doesn't model those changes yet.
