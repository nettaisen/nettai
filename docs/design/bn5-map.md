# BN5 against BN6: the routine map

What of BN6's battle code BN5 has, routine by routine, and what differs. The evidence base for the rules design
([multi-game.md](multi-game.md), the rules-into-Luau design) and for BN5's oracle, extractor and port. The
user, 2026-10-02: "start implementing bn5. it should be composable with bn6 content".

The map is made by the verification workspace's `tools/bn5/bmap.py` from the ROMs alone: there is no BN5
disassembly or symbol map (none on this machine, none known). BN6's routines are those of the BN6 disassembly
(US Falzar, BR6E), named and sized by its symbols; the engine's port and docs/engine cite them by these names.
BN5's are found by discovery in each BN5 ROM. Addresses below are US Team ProtoMan's (BRBE) unless said otherwise.
How to read a pair: `bmap.py diff NAME` prints a BN6 routine beside its BN5 counterpart.

## 0. Summary

- **About two thirds of BN6's battle code is in BN5 in some form, and two fifths of it verbatim.** Of BN6's 4,567
  battle routines in scope (the audit's, completeness.md: ported, documented, folded, presentation, link,
  infrastructure; out of scope left out), BN5 has
  - 1,821 (40%) **the same** modulo relocation (only call targets and address literals differ),
  - 272 (6%) the same instructions with **other constants or field offsets**,
  - 996 (22%) **similar** (a counterpart with instructions inserted, removed or changed; ratio ≥ 0.5),
  - 246 (5%) a counterpart that **differs** (found by where the callers and tables put it; ratio below 0.5),
  - 1,232 (27%) **absent**.

  By size the shares are 27%, 6%, 31%, 8% and 29%: the shared routines are the smaller ones (accessors, the
  object system, collision and status helpers), the changed ones the larger (handlers, the custom screen).
- **The engine core is shared.** The object record (0xD8 bytes for actors and attacks, 0xC8 for effects) has every
  field at the same offset, the chip record is the same 0x2C bytes with the same fields, the object system and
  its update list are the same code, and so are most of collision, the hit kernel's helpers, the panel grid and
  the flow machines. What moved is RAM (every structure, by its own amount) and the **actor pool: 16 slots in
  BN5, 32 in BN6** (the attack and effect pools are 32 in both).
- **NaviStats** is 0x60 bytes in BN5 (BN6 0x64), and the fields both games' shared code names are mostly at the
  same offsets (§3.3).
- **The custom screen's core is BN5's** (the slot layout, the keys, what can be picked, OK's hand, the deal, the
  dark chip's cursor and fade), while BN6's Cross window, Beast Out button, its result and its hand-size rule
  are absent; BN5 has about 6 KB of its own custom-screen code (§4).
- **The turn-start transformation sequencer is BN5's** (the same code), so Soul Unison rides the mechanism
  BN6's Crosses and Beast Out use; and BN6's dead "Cross change" (a navi switch) is BN5 code, the same (§5).
- **BN5's own battle code** (routines with no BN6 counterpart that a battle reaches) is about 1,400 routines,
  88 KB, in some 450 blocks (§6).
- **Team ProtoMan and Team Colonel** differ in about 22 battle routines; their code is otherwise the same, moved
  (mostly by 0xE8) (§7).
- **Tango plays BN5 netbattles as Team Battles** (the Battle Chip Gate's row), not as plain NetBattles (§8); the
  user chose Team Battles first.
- **BN5 is traced** (§10): oracle-trace, the chip lab and difftest know BN5's two US ROMs, and a first Team Battle
  of the original is recorded. **bn5-extract** writes a BN5 pack (§11) that says its game, so it loads beside
  BN6's under its own names (§9).

## 1. Method

**BN6's routines:** the recompiler's inventory (13,337 routines), sized by the disassembly's symbols, each
decoded as Thumb up to where its own data starts (a jump table or literal it loads the address of).

**BN5's routines:** discovery from the boot's pointers, every BL whose target starts with a `push {.., lr}`,
every code pointer stored in the ROM whose target starts with a push, then to a fixed point every BL target,
code pointer and table of code pointers a routine loads. On BN6 the same discovery finds 12,703 of the
inventory's 13,331 Thumb routines (95%) and 96% of the audit's battle routines, so BN5's list is about as
complete.

**Comparison:** each instruction is split into its shape (opcode and registers) and its immediates; a BL is one
instruction, a pc-relative load carries its literal's value, classed as an address (ROM, EWRAM, IWRAM; it moves
between builds) or a constant.

| Status | Meaning |
|---|---|
| same | identical instructions; constants equal; call targets and address literals may differ |
| consts | identical shapes; some immediates or constant literals differ (a field offset, an id, a count) |
| similar | a counterpart whose shapes align with difflib's ratio ≥ 0.5 |
| differs | a counterpart with a ratio below 0.5 |
| absent | no counterpart found |

**Finding counterparts,** in order (the `via` column of the table):

1. `exact`: BN6's bytes, with BL pairs and address literals masked, at exactly one place of the BN5 ROM (or its
   IWRAM code); `exact?` when several, the one nearest its neighbours' move.
2. `shape`: the same shapes at exactly one discovered BN5 routine.
3. `call`: a BL (or a code pointer) at the same place of a matched pair (strong when the pair is the same code).
4. `table`: the same index of a table of code pointers both load (object kinds, state machines, handler tables,
   tables of tables).
5. `order`: between two matched neighbours, the unmatched routines of both gaps paired in order by similarity.

Each BN5 routine is the counterpart of at most one BN6 routine (the strongest claim keeps it). Weak claims (a
table entry, an order pairing) need a ratio of 0.35. Steps 3 to 5 repeat to a fixed point.

**How sure.** `same` and `consts` are exact facts about the code. `similar` and `differs` are pairings: right
when found by `call` from a same caller, plausible by `table` or `order`. Run on Team Colonel, the map pairs
99.7% of BN6's routines with the counterpart Team ProtoMan's map gives, moved by the version's move. A
spot-check of pairs read side by side (battle_8007800, sub_8009338, sub_800F09E, the custom screen's) found no
wrong pairing; a few BN6-only routines next to a BN5 routine of the same shape were mis-paired before the
one-to-one rule.

## 2. By area

The audit's areas, in-scope routines only:

| Area | Routines | same | consts | similar | differs | absent |
|---|---|---|---|---|---|---|
| object kinds, chips, navis (asm31) | 2,682 | 864 | 144 | 583 | 198 | 893 |
| actors, collision, status, HUD | 821 | 361 | 53 | 215 | 32 | 160 |
| custom screen, gauge, camera | 367 | 161 | 23 | 81 | 10 | 92 |
| elsewhere, run in a netbattle | 222 | 172 | 8 | 29 | 0 | 13 |
| battle flow | 180 | 100 | 13 | 50 | 1 | 16 |
| battle objects, panels (object.s) | 137 | 86 | 24 | 17 | 5 | 5 |
| link layer | 79 | 32 | 3 | 11 | 0 | 33 |
| IWRAM routines | 37 | 16 | 1 | 5 | 0 | 15 |
| object system | 26 | 21 | 2 | 0 | 0 | 3 |
| link status, battle settings | 16 | 8 | 1 | 5 | 0 | 2 |

By the audit's class: of the 3,401 **ported** routines, 1,164 are the same in BN5, 194 consts, 814 similar, 227
differ and 1,002 are absent. Out of scope (viruses, navi AI, other battle modes) 23% are the same and 53% absent.

The absent object-kind routines are mostly BN6's own chips, navis and forms (the Crosses, the Beast forms, the
link navis' chips, BN6's new chips); many more of BN5's chips are BN6's predecessors with other numbers, found as
similar.

## 3. Records and RAM

### 3.1 The object record

`fields.py` (r5, every audited routine): for every load or store through r5 (the object, in BN6's object code)
at the same place of a BN6 routine and its counterpart, the offset each uses. Of 11,209 such accesses 97.4% use
the same offset, and every field from +0x00 to +0x7C is mostly at its BN6 offset. The one pattern among the
rest: the navi chips' summoned navis (BN6's routines from `sub_80B8F30` to `sub_80BA0D8`, BlastMan's, TenguMan's
and their neighbours') step their state machine in +0x0B where BN6 uses +0x0A (63 places, 8 of them in
otherwise identical code). The record sizes are BN6's: 0xD8 for actors and attacks, 0xC8 for effects (the pool
table of `InitializeStructsOfObjectType`, the same code in both).

**The pools.** Actors (type 1): **16 slots** in BN5 at 0x0203B200 (BN6 32 at 0x0203A9B0). Attacks (type 3): 32 at
0x0203CA40 (BN6 0x0203CFE0). Effects (type 4): 32 at 0x02036F00 (BN6 0x02036870). The engine's `object::SLOTS`
is one number for all three pools; BN5 needs it per pool.

### 3.2 The chip record

The getter (`getChip8021DA8`) is the same code: 0x2C bytes a record, the table at 0x0801E214 (BN6 0x08021DA8).
`fields.py after getChip8021DA8`: every field the shared code reads after it (+0x00 to +0x28: codes, elements,
class, MB, flags at +0x09, the counter, family and subfamily at +0x07/+0x08, the parameters, the delay, power at
+0x1A, the sort keys and the image) is at the same offset. Tango's reading of the record agrees (multi-game.md
§2.2).

### 3.3 NaviStats

0x60 bytes a side in BN5 (the two blocks at 0x0203C880 and 0x0203C8E0; BN6 0x64 at 0x0203CE00 and 0x0203CE64).
`fields.py navistats` pairs the field each call to a NaviStats accessor names. +0x01 to +0x3E (the buster, the
NaviCust programs' bytes, the custom level +0x0A, the folders +0x0B/+0x0C, the supports' +0x0D, the mood +0x0E,
the shoes and armours +0x1B to +0x23, the form +0x29, HP +0x40/+0x42) are the same offsets. Seen moved, one call
each (to be confirmed): +0x04 → +0x39 and +0x39 → +0x04 (a swap), +0x17 → +0x2C, +0x21 → +0x44, the patch-card
block +0x56 to +0x5B → +0x2D to +0x30 and +0x40, +0x5F → +0x34, +0x63 → +0x54. Four of 46 calls reading the form
+0x29 read +0x2C in BN5 (`sub_800F09E`, the charged chip by form: BN6's forms 1–4 and 8 are BN5's souls there).

### 3.4 RAM

Everything moved, each structure by its own amount (`bmap.py ram`: the RAM addresses the same routines load,
and the tables of RAM pointers they load, the toolkit's among them; 567 addresses). The ones the trace format
reads:

| BN6 | Name | BN5 | Move |
|---|---|---|---|
| 0x02034880 | BattleState | 0x02034A90 | +0x210 |
| 0x02001120 | RNG1 | 0x02001C94 | +0xB74 |
| 0x020013F0 | RNG2 | 0x02001D40 | +0x950 |
| 0x020093B0 | eToolkit | 0x0200A440 | +0x1090 |
| 0x02009380 | the object update list (start, sentinel 0x02009AB0) | 0x0200A410 (0x0200ABA0) | +0x1090, +0x10F0 |
| 0x0200A270 | eJoypad | 0x0200AF50 | +0xCE0 |
| 0x0200A490 | the sound queue | 0x0200B170 | +0xCE0 |
| 0x020349C0 | the chip blocks (hands) | 0x02034E20 | +0x460 |
| 0x02036820 | the input records | 0x02036E90 | +0x670 |
| 0x02036840 | the banner | 0x02036ED0 | +0x690 |
| 0x020367F0 | the transformation sequencer | 0x02036E60 | +0x670 |
| 0x02039AE0 | the panels | 0x0203A100 | +0x620 |
| 0x0203CA70 | the fighting machine | 0x0203C5D0 | −0x4A0 |
| 0x0203CDB0 | the battle folder | 0x0203C830 | −0x580 |
| 0x0203CE00 | NaviStats (two) | 0x0203C880 | −0x580 |
| 0x0203F7D8 | the link struct | 0x0203F244 | −0x594 |
| 0x020352A0 | the custom gauge (rate at +2) | 0x02035700 | +0x460 |
| 0x020352C0 | the HUD's task mask | 0x02035720 | +0x460 |
| 0x02001B8A | the pause flag (toolkit +0x3C, +0xA) | 0x02002949 (+9) | |
| 0x0203F558, 0x0203F658 | the exchanged transform records | 0x0203EFC8, 0x0203F0C8 | −0x590 |
| 0x020364C0, 0x020365C0 | the custom screen's state and slots | 0x02036B10, 0x02036C10 | +0x650 |

BattleState, RNG1, RNG2, the hands and actor 0 agree with Tango's BN5 support, and every row was checked on
running consoles (§10): the gauge fills to 0x4000, the pause flag is set through the turn's banners as BN6's is,
the transform records hold the requesting navi at +8. Three layouts differ besides the moves:

- **The panels are 0x24 bytes** (BN6 0x20): BN5's panel getter (0x0800BD1C, ROM code; BN6's is IWRAM code)
  multiplies by 0x24. The type and owner are at +2 and +3, as in BN6.
- **The sound queue's entries start at +8** (BN6 +0xC); the m4a players are BN6's moved 0xBE0 on, and BN5's queue
  has no conditional-music request (BN6's `sub_8000822`).
- **The actor pool** (§3.1): 16 slots.

**IWRAM code:** BN5 copies 0x1C58 bytes from 0x081C7A00 to **0x03005C00** (BN6: 0x1ED4 bytes from 0x081D6000 to
0x03005B00).

## 4. The custom screen

**Shared** (BN6 routine: BN5 status):

- the per-console screen and its state machine, `sub_8026A28`: same (state block 0x02036B10, BN6 0x020364C0);
- the slot layout's neighbour fill `sub_8027F42`, the deal's compaction `sub_802945A`: same;
- the keys `sub_8028B74`: similar 0.89; what can be picked `sub_8028E32`: 0.94; OK builds the hand
  `sub_8029110`: 0.99 (127 instructions each); the opening's slide `sub_8026B04`: 0.95; the sub-screens
  `sub_8029688`, `sub_8029788`, `sub_802983C`: 0.72–0.84;
- BN6's DustCross scrap `sub_8027406` and ChpShufl re-deal's helpers `sub_80271F8`, `sub_802721C`,
  `sub_802723A`: the same code in BN5 (presumably the sacrifice and re-deal machinery BN5's own screens use,
  which BN6 reused);
- the **dark chip** on the screen: the cursor starting on it `sub_802806C` the same, the hover's fade and music
  ramp `sub_802A2B0` 0.92. This is the code unverified.md lists as unreachable in BN6 (no US chip has the dark
  flag): it is BN5's.
- the folder shuffle `sub_800A570`: similar 0.74 (BN6 adds the Tag chips).

**BN6's own:** the OK result's form (`sub_8029344`, `sub_802937A`), the Cross window and its list (`sub_802A220`,
`sub_80275EC`, `sub_80280E0`), the hand-size rule with ChargeCross and NumbrOpn (`sub_802A40C`; BN5's
`sub_802A49C` counterpart is similar 0.62), the opening `sub_8026840` and the round's custom memory: absent.

**BN5's own:** the custom screen's entry (`sub_8009338`, similar 0.66) runs BN6's screen (`sub_8026A28`) unless
the battle flag 0x40 mode is on (`sub_800A8F8`, the same code), in which case it runs BN5's own screen (0x08025EF2,
in a 2.9 KB block from 0x08025E4E, beside another of 3.2 KB from 0x080269A0). BN6's flag-0x40 mode is the "Cross
change mode", never in a netbattle. **Tango's Team Battles don't set it** (the battle flags, battle state +0x32, read 0
in the traces): their custom screen is the shared one, and the per-player gauges and SELECT special of the flag-0x40
mode don't run either.

**Soul Unison on the shared screen** (read in Team ProtoMan's code, driven in the chip lab, §10): Beast Out's
button under OK (slot 11, kind 2) is BN5's Soul Unison button. After every pick and take-back (`sub_8028E32`'s
counterpart calls 0x08024B28 first) it is offered when the **last pick** is a chip whose family (record +6) is a
soul's (the family table at 0x08024BE0: twelve souls, a family each), the save has that soul (an event flag per
soul in the version's table at 0x08024BF0: Team ProtoMan has souls 1–6, Team Colonel 7–12), and the soul wasn't
used this battle (a bit per soul in 0x02034E10). A **dark chip** (record +9 flag 0x20) of the family offers the
soul's **Chaos Unison** instead, if event flag 0x236 is set (its own bit, +16). A to the button (sub-state 0x24,
Beast Out's) gives the last pick up for the soul; the hand builder (`sub_8029110`'s counterpart, 0x08024DAC) then
writes the transform record (`sub_8015952`: the soul, Chaos or not, the turns: 3 plus a NaviStats-derived bonus,
1 to 9; a Chaos Unison 1) and marks the soul used. The pick count stays. The souls by family: 1 sword (6), 2 wind
(10), 3 cursor (8), 4 fire (0), 5 elec (2), 6 recovery (4); 7 obstacle (9), 8 invisible (7), 9 plus (5), 10 wood
(3), 11 break (11), 12 aqua (1) (by the lab's pictures: 1 is ProtoSoul, 10 TomahawkSoul).

**Light and dark MegaMan** (observed in the lab): a dark MegaMan has no Soul Unison button (slot 11 isn't kind 2)
and can't use the SP navi chips; a light MegaMan can't use the dark chips or the DS navi chips (each fizzles: the
navi enters action 0x1A for a frame and a puff of smoke appears). Which branch decides is not read yet.

## 5. Transformations: Soul Unison, the navi switch

- **The turn-start sequencer** (battle-flow.md §3.4.1): `sub_801483C`, `sub_80148CC`, `sub_8014944`,
  `sub_8014A00`, `sub_80147E4` (copying the exchanged transform records) and `sub_801482C` are the same in BN5;
  `sub_801486C` similar 0.93, the fades `sub_80148EC` and `sub_801498E` 0.76 and 0.69. So BN5's form change
  (Soul Unison) goes through the same transform record and the same fade-change-fade sequence as BN6's Crosses
  and Beast Out. BN6's Beast Out end check `sub_80159C6` is similar 0.57 in BN5: the place a soul's turns run
  out, presumably.
- **The navi switch:** BN6's Cross change (the pause handler's action 0x1C, `sub_802D714`, `sub_802D738`,
  `sub_802D7A0`, `sub_802D8F0`, the fall-back `sub_802DD2A`, `sub_802D926`, `sub_802D9B0`) is all in BN5, the same
  or 0.97–1.00 similar. In BN6 no custom screen sends it (custom-screen.md §6). **In BN5 the custom screen does,
  from the Battle Chip Gate:** on every tick of the screen's state 4, 0x080259A0 asks the gate for an inserted navi
  chip (0x0812A074; in a link battle through the link, 0x08143CB8); a navi other than the current one (a set of
  rules: not used this battle, `0x02034E10`+14, and others) moves the screen to its state 0x40 (0x08023840), which
  sets the screen's navi (+0x10); the hand builder then writes the transform record's +4 (`sub_802DCD8`'s
  counterpart, BN6's has no caller) and the pause handler switches. **Tango never inserts a gate chip** (its primer
  raises the gate-present flag and nothing else), so in Tango's Team Battles the navi switch is unreachable, as in
  BN6. A chip lab scenario would need the gate's answer faked (the gate's RAM, not read yet).
- **The form record:** a navi's form data (the charged chips by form `sub_800F09E`, similar 0.50) reads NaviStats
  +0x2C in BN5 where BN6 reads +0x29, with other forms and families (BN5's souls).

## 6. BN5's own battle code

`only.py`: from the BN5 counterparts of BN6's in-scope routines, the calls, code pointers and tables into BN5
routines with no BN6 counterpart: 1,369 routines, about 88 KB, in 455 blocks. The largest, by where the shared
code reaches them:

| BN5 block | Size | Reached from (BN6 names) |
|---|---|---|
| 0x08029BEC–0x0802AE62 | 4.7 KB, 79 routines | the navi's actions (via `sub_80175B8`'s tables) |
| 0x080269A0–0x08027634 | 3.2 KB, 55 | the custom screen's flag-0x40 branch (`sub_8009338`) |
| 0x08025E4E–0x08026984 | 2.9 KB, 54 | the same, and `sub_8027FDC`, `sub_8009158` |
| 0x0802C012–0x0802CA0C | 2.6 KB, 34 | damage and HP (`applyDamageToPlayer_801ba12`, `object_subtractHP`), the flow (`sub_80071D4`, `sub_8007CA0`) |
| 0x0802F0D6–0x0802F850, 0x0802B4AC–0x0802BA12, 0x0802F934–0x0802FC4E | 1.9, 1.4, 0.8 KB | the navi's actions |
| 0x080F0530–0x080F0A68, 0x080F1DE2–0x080F21EE | 1.3, 1.0 KB | the navi's actions and `sub_80EA484` (an object kind) |
| 0x08136F38– and four more near 0x0813A000 | 2 KB each | the comm menu (`SubMenuControl`): out of the battle |

Chaos Unison (the dark chip held by the soul, the charged shot's timing) and the dark chips' battle costs are
expected in the navi's-actions and damage blocks; they are not read yet. The tables are
`target/bn5/only-<CODE>.tsv` in the verification workspace, per routine with where it is reached from.

## 7. Team ProtoMan and Team Colonel

`versions.py` (fmap.py's method between the two BN5 ROMs): the battle code is the same, moved (mostly +0xE8,
also +0x4, +0x8, −0x8). The battle routines that differ: the chip use's `chip_800AEE8` and `sub_800AF34`, the
HUD's `sub_801CC34` and `sub_801E574`–`sub_801E6A8` (the emotion window: each version's soul faces, presumably),
`sub_80133EC`, two link routines (`sub_803F894`, `sub_803F8C4`), seven object kinds (`sub_80C6C14`,
`sub_80C9C5C`, `sub_80CBD32`, `sub_80D795C`, `sub_80DAA48`, `sub_80DB6D4`, `sub_80DDC30`: each version's souls'
moves, presumably), the save's encryption pair and one IWRAM routine. RAM is the same in both (Tango uses one
EWRAM table for all four BN5 ROMs). As with Falzar and Gregar, what each version owns (its souls) is content and
setup, and its few routines are version branches.

## 8. Facts for the next steps

- **Hooks** (Team ProtoMan): the battle's frame routine (BN6 `battle_8007800`) at 0x08006C10, the same code but
  for its callees (similar 0.82 only because its state table follows it); after its prologue 0x08006C1A; the
  main loop's frame start 0x080002D4 (as in BN6); the m4a calls: SongNumStart 0x0814D634, MPlayAllStop
  0x0814D768, TempoControl 0x0814E698, PitchControl 0x0814E724, VolumeControl 0x0814E6BC, FadeOut 0x0814D58C,
  SongNumStop 0x0814D700, ImmInit 0x0814D82C, FadeIn 0x0814D800 (all the same code). **Two** link applets call
  the frame routine: 0x081359C4 (the plain NetBattle's, BN6 `sub_812B698`'s counterpart by shape) and 0x0813B4DC
  (the Team Battle's: Tango's Team Battles return there).
- **Tango and Team Battles:** since 2026-08-05 Tango's BN5 primer raises the Battle Chip Gate flag and confirms
  the comm menu's Team Battle row (チームバトル), so every Tango BN5 match is a Team Battle (mode bytes 4–7), with
  Patch Cards on. **The user's decision (2026-10-02): Team Battle first**, as Tango plays it; a plain NetBattle
  later. The Team Battle is the Battle Chip Gate's mode (two consoles, each with a gate); Tango's runs the shared
  custom screen with Soul Unison, and its navi switch (from a gate's navi chip) never happens (§5). A plain
  NetBattle would add the comm menu's first row in the primer and the other link applet.
- **Replays:** no BN5 replay in Tango's current format. Three of 2022 in the oldest format (0x10) and six in
  format 0x11 (made with the bn5_gate patch), the same kind of savestate-started rounds the 2022 BN6 replays are;
  two BN5 DS replays (another platform, out of scope).
- **Saves:** the .sav files beside the US ROMs are blank; Tango's saves folder has finished US saves (Team
  ProtoMan light, Team Colonel dark), which the chip lab uses, and Tango carries light and dark templates for all
  four BN5 ROMs.
- **The Japanese ROMs** (BRBJ, BRKJ) are not mapped yet (`bmap.py --to BRBJ` would).

## 9. Asset packs that load together (agreed)

The user wants BN5 composable with BN6 content. The rules design (rules-in-luau.md §7) and this proposal agree,
and the user confirmed (2026-10-02): every pack and content root declares its game (`game = "bn5"`); the loader
qualifies names as `<game>:<key>` (`bn5:cannon`) when roots load together; inside a root and its compat, keys and
asset names stay unqualified; version suffixes `-protoman`/`-colonel` and the region as a field; a root manifest
has `name` (its namespace) and `assets` (whose pack it resolves in, its own game by default). **No shared content
library:** every game exports its own content, even where it overlaps with BN6's (`bn5:cannon` and `bn6:cannon`
are separate), so BN5's assets are named for BN5 alone. For the packs:

- **A pack says its game.** The manifest (content.toml) gains `game = "bn5"` (BN6's packs `game = "bn6"`), the
  name a root's `assets` refers to. A loader given several packs keys them by it, and refuses two of one game.
- **Names inside a pack stay unqualified**, as BN6's are now (`graphics/sprites/<name>`, assets.toml's names): a
  pack is one game's, so its own names can't collide. BN5's names are BN5's own, curated like BN6's
  (compat/assets.toml in content/bn5), with placeholders for the rest.
- **Qualified when loaded together:** the engine's asset handles cover every loaded pack, keyed by
  `<game>:<name>` (`bn5:bomb` and `bn6:bomb` are different sprites), interned like definition keys. Inside a root,
  `asset.sprite("bomb")` means its own pack's; another game's asset is named qualified (`asset.sprite("bn6:bomb")`),
  allowed for the roots in its `requires`. A sprite's identity gains its pack (`SpriteId`), as §7.4 says.
- **Variants, as in BN6's packs:** what differs by version is named with the version
  (`-protoman`/`-colonel`, as BN6's `-falzar`/`-gregar`), and what a console of each version shows of its own
  goes in nettai-assets' `Versioned`; what comes from the Japanese ROMs carries the `region` field
  (sprite.json, custom.json), as BN6's do.
- **Sound** is per pack too: BN5's songs and sound effects are BN5's numbers in BN5's m4a bank; a cue names
  `<game>:<song>` once qualified, and the audio loads each pack's bank. Two packs' banks never mix inside one
  m4a player (a song plays with its own pack's instruments).
- **Formats:** `nettai-content/hud` and `nettai-content/custom` are BN6's layouts (multi-game.md §1.6). BN5's
  HUD and custom screen differ (souls, team navis); bn5-extract writes what is shared in those formats and leaves
  the rest to formats the BN5 ruleset will need, rather than stretch BN6's.

## 10. Tracing BN5 (as built)

The verification workspace traces BN5 consoles as it does BN6's, with the same line format:

- **oracle-trace** has the games `TeamProtoMan` (BRBE), `TeamColonel` (BRKE) and the Japanese `JpTeamOfBlues`
  (BRBJ) and `JpTeamOfColonel` (BRKJ), their hooks (§8; both link applets' returns are trapped) and a RAM
  `Layout` per game (§3.4; the Japanese ROMs' RAM is the US ROMs'). BN6's lines are byte-identical to before. A
  BN5 setup line says `"game":"bn5"`, has BN5's 0x60-byte NaviStats blocks, says the regions when a side is
  Japanese, and leaves out what is BN6's alone (SP times, link navi levels, bug frags, event flags, Tag chips).
  The hooks test checks every BN5 hook against BN6's code (masked for what moves, RAM included), Team Colonel's
  against Team ProtoMan's, and each Japanese ROM's against the US ROM of its version.
- **chiplab** runs BN5 consoles from a base of BN5 ROMs and saves (Tango's primer walks into a Team Battle),
  edits BN5 saves (folder, Regular chip, navi, NaviCust, HP) and drives them (`custom CHIP soul` for Soul Unison);
  BN5's navi stands idle in action 6 (BN6 8). BN5's scenarios are a library of their own (generated from the ROM's
  chip, Program Advance and soul tables), recorded apart from BN6's lab: bases with a light and a dark MegaMan on
  either side (§4), every chip from both sides, the Program Advances, each soul's Soul and Chaos Unison.
- **difftest** takes BN5 replays (none exists in Tango's current format yet).
- **The first traces:** a plain Team Battle, Team ProtoMan (traced) against Team Colonel, each picking the first
  chip dealt every turn and shooting until Team ProtoMan's navi is deleted: 3,216 frames, a full round from the
  intro to the deletion; and the same on Japanese consoles (Tango's netplay saves at 60 HP; 2,005 frames). Nothing
  replays them yet.
- Observed on the way: a Team Battle's custom screen keeps its state in the shared screen's block; Patch Cards apply
  in Team Battles (the Team ProtoMan save's take 150 off its max HP); the sound queue and panels differ (§3.4).

## 11. The BN5 pack (as built)

`bn5-extract content <protoman-us> <colonel-us> <protoman-jp> <colonel-jp> <pack-dir>` (the four ROMs by header,
as bn6-extract takes BN6's) writes a pack whose manifest says `game = "bn5"`:

- **What it has:** the 272 battle sprites (the same in both US ROMs), the field (11 panel types, BN6 13; one
  highlight block for both highlights, BN6 two), the 29 battle backgrounds with their scrolling and animations, the
  m4a bank (308 songs, 105 samples), the 368 chips' pictures and icons, the HUD's 8x16 font, and the dialogue font
  (442 glyphs; BN5's advance table is a word a glyph, BN6's a byte).
- **Versions:** 12 chips (0x12D–0x136, 0x139, 0x13A: the version navi chips) are drawn differently by each
  version's ROM; their pictures and icons are in the pack twice, `chip-12d-protoman` and `chip-12d-colonel`, with
  their `version`.
- **Version songs:** Team Colonel has 11 songs of its own at Team ProtoMan's numbers (0x13C–0x142, 0x145, 0x146,
  0x170, 0x171: its navi chips' sounds). They are in the pack beside Team ProtoMan's, each a song file named with
  its version (`sound-13c-protoman`, `sound-13c-colonel`, with `version` in its header and `base_version =
  "protoman"` in sound.toml); the asset index keeps one name a number. They play with the bank's voicegroups:
  Team Colonel's are matched to equal ones of Team ProtoMan's bank (samples, waves and key maps by content,
  drum kits and splits in turn) or added. nettai-content's `sound::SongVersions` holds them; a pack without
  versions (BN6's) names no version anywhere and is byte-identical to before. Which version a console plays is the
  audio's choice by the viewer's console, not built yet.
- **Left out, for now:**
  - The Japanese ROMs' one different sprite (14-17, which has text on it): the US release localized it rather than
    cut it, so the pack keeps the US's, as BN6's does.
  - BN5's HUD and custom-screen layouts (§9).
  - **Languages** (BN6's shape, text-rendering.md §10): with a BN5 content root, its strings go in
    `content/bn5/locales/en.toml` (from the US ROMs) and `ja.toml` (from the Japanese ROMs), keyed by definition
    key; and bn5-extract writes the Japanese ROMs' lettering beside the US's as nettai-assets' `HudLettering` and
    `CustomLettering` (the fonts in the Japanese encoding, the HUD's lines, the banners and pictures with words in
    them), as bn6-extract's `lettering` does. Neither exists yet: the pack is the US ROMs' lettering alone.
- **Names:** placeholders (`sprite-0c-2d`, `sound-10e`, `chip-12d`; a glyph's number in brackets) until a BN5 content
  root names them in its compat, as BN6's does.
- **Shared decoding:** the sprite archive and GFX-animation decoders are BN6's format and code; bn5-extract has its
  own copy, which belongs in one shared decoder with bn6-extract's when that is next reworked.
- nettai-content's HUD reader now accepts a pack without the count box (BN5 has none).

## 12. Reproducing

In the verification workspace, with the ROMs in `$BN6_ROMS` and the BN6 disassembly's symbols in `$BN6F`:

    tools/bn5/bmap.py [--to BRKE] [ram]    # target/bn5/bmap-<CODE>.tsv, ram-<CODE>.tsv, the summary
    tools/bn5/bmap.py diff NAME...         # a BN6 routine beside its counterpart
    tools/bn5/fields.py [r5 | navistats | after NAME]
    tools/bn5/only.py                      # BN5's own battle code
    tools/bn5/versions.py [BRBE BRKE]      # the two versions

`target/audit/classes.tsv` (the audit, `tools/audit/audit.py`) gives the areas and classes; without it the map is
the same, unclassified.
