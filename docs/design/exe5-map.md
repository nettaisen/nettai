# EXE5 against EXE6: the routine map

What of EXE6's battle code EXE5 has, routine by routine, and what differs. The evidence base for the rules design
([multi-game.md](multi-game.md), the rules-into-Luau design) and for EXE5's oracle, extractor and port. The
user, 2026-10-02: "start implementing bn5. it should be composable with bn6 content".

The map is made by the verification workspace's `tools/exe5/bmap.py` from the ROMs alone: there is no EXE5
disassembly or symbol map (none on this machine, none known). EXE6's routines are those of the EXE6 disassembly
(US Falzar, BR6E), named and sized by its symbols; the engine's port and docs/engine cite them by these names.
EXE5's are found by discovery in each EXE5 ROM. Addresses below are US Team ProtoMan's (BRBE) unless said otherwise.
How to read a pair: `bmap.py diff NAME` prints an EXE6 routine beside its EXE5 counterpart.

## 0. Summary

- **About two thirds of EXE6's battle code is in EXE5 in some form, and two fifths of it verbatim.** Of EXE6's 4,567
  battle routines in scope (the audit's, completeness.md: ported, documented, folded, presentation, link,
  infrastructure; out of scope left out), EXE5 has
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
  EXE5, 32 in EXE6** (the attack and effect pools are 32 in both).
- **NaviStats** is 0x60 bytes in EXE5 (EXE6 0x64), and the fields both games' shared code names are mostly at the
  same offsets (§3.3).
- **The custom screen's core is EXE5's** (the slot layout, the keys, what can be picked, OK's hand, the deal, the
  dark chip's cursor and fade), while EXE6's Cross window, Beast Out button, its result and its hand-size rule
  are absent; EXE5 has about 6 KB of its own custom-screen code (§4).
- **The turn-start transformation sequencer is EXE5's** (the same code), so Soul Unison rides the mechanism
  EXE6's Crosses and Beast Out use; and EXE6's dead "Cross change" (a navi switch) is EXE5 code, the same (§5).
- **EXE5's own battle code** (routines with no EXE6 counterpart that a battle reaches) is about 1,400 routines,
  88 KB, in some 450 blocks (§6).
- **Team ProtoMan and Team Colonel** differ in about 22 battle routines; their code is otherwise the same, moved
  (mostly by 0xE8) (§7).
- **Tango plays EXE5 netbattles as Team Battles** (the Battle Chip Gate's row), not as plain NetBattles (§8); the
  user chose Team Battles first.
- **EXE5 is traced** (§10): oracle-trace, the chip lab and difftest know EXE5's two US ROMs, and a first Team Battle
  of the original is recorded. **exe5-extract** writes an EXE5 pack (§11) that says its game, so it loads beside
  EXE6's under its own names (§9).
- **The action map** (§14). Of EXE5's 330 chips:
  - 122 descend from the same chip's EXE6 code: 21 identical where the labs ran, 8 with constants only, 93 changed.
  - 101 are built like another EXE6 chip's code.
  - 17 descend from code EXE6 keeps but no EXE6 chip uses.
  - 90 are EXE5's own, LeadRaid and ChaosLrd among them. LeadRaid is EXE6's TwinLdrs's ancestor (the same actor
    kind and handler), reworked.

  Three shared routines differ for many chips:
  - The dimming chips and action 0x1A leave the action on the use frame in EXE5.
  - The navi chips' framework is the same code with renumbered chips.
  - The swords pick their slash by soul in EXE5, by form in EXE6.

## 1. Method

**EXE6's routines:** the recompiler's inventory (13,337 routines), sized by the disassembly's symbols, each
decoded as Thumb up to where its own data starts (a jump table or literal it loads the address of).

**EXE5's routines:** discovery from the boot's pointers, every BL whose target starts with a `push {.., lr}`,
every code pointer stored in the ROM whose target starts with a push, then to a fixed point every BL target,
code pointer and table of code pointers a routine loads. On EXE6 the same discovery finds 12,703 of the
inventory's 13,331 Thumb routines (95%) and 96% of the audit's battle routines, so EXE5's list is about as
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

1. `exact`: EXE6's bytes, with BL pairs and address literals masked, at exactly one place of the EXE5 ROM (or its
   IWRAM code); `exact?` when several, the one nearest its neighbors' move.
2. `shape`: the same shapes at exactly one discovered EXE5 routine.
3. `call`: a BL (or a code pointer) at the same place of a matched pair (strong when the pair is the same code).
4. `table`: the same index of a table of code pointers both load (object kinds, state machines, handler tables,
   tables of tables).
5. `order`: between two matched neighbors, the unmatched routines of both gaps paired in order by similarity.

Each EXE5 routine is the counterpart of at most one EXE6 routine (the strongest claim keeps it). Weak claims (a
table entry, an order pairing) need a ratio of 0.35. Steps 3 to 5 repeat to a fixed point.

**How sure.** `same` and `consts` are exact facts about the code. `similar` and `differs` are pairings: right
when found by `call` from a same caller, plausible by `table` or `order`. Run on Team Colonel, the map pairs
99.7% of EXE6's routines with the counterpart Team ProtoMan's map gives, moved by the version's move. A
spot-check of pairs read side by side (battle_8007800, sub_8009338, sub_800F09E, the custom screen's) found no
wrong pairing; a few EXE6-only routines next to an EXE5 routine of the same shape were mis-paired before the
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

By the audit's class: of the 3,401 **ported** routines, 1,164 are the same in EXE5, 194 consts, 814 similar, 227
differ and 1,002 are absent. Out of scope (viruses, navi AI, other battle modes) 23% are the same and 53% absent.

The absent object-kind routines are mostly EXE6's own chips, navis and forms (the Crosses, the Beast forms, the
link navis' chips, EXE6's new chips); many more of EXE5's chips are EXE6's predecessors with other numbers, found as
similar.

## 3. Records and RAM

### 3.1 The object record

`fields.py` (r5, every audited routine): for every load or store through r5 (the object, in EXE6's object code)
at the same place of an EXE6 routine and its counterpart, the offset each uses. Of 11,209 such accesses 97.4% use
the same offset, and every field from +0x00 to +0x7C is mostly at its EXE6 offset. The one pattern among the
rest: the navi chips' summoned navis (EXE6's routines from `sub_80B8F30` to `sub_80BA0D8`, BlastMan's, TenguMan's
and their neighbors') step their state machine in +0x0B where EXE6 uses +0x0A (63 places, 8 of them in
otherwise identical code). The record sizes are EXE6's: 0xD8 for actors and attacks, 0xC8 for effects (the pool
table of `InitializeStructsOfObjectType`, the same code in both).

**The pools.** Actors (type 1): **16 slots** in EXE5 at 0x0203B200 (EXE6 32 at 0x0203A9B0). Attacks (type 3): 32 at
0x0203CA40 (EXE6 0x0203CFE0). Effects (type 4): 32 at 0x02036F00 (EXE6 0x02036870). The engine's `object::SLOTS`
is one number for all three pools; EXE5 needs it per pool.

### 3.2 The chip record

The getter (`getChip8021DA8`) is the same code: 0x2C bytes a record, the table at 0x0801E214 (EXE6 0x08021DA8).
`fields.py after getChip8021DA8`: every field the shared code reads after it (+0x00 to +0x28: codes, elements,
class, MB, flags at +0x09, the counter, family and subfamily at +0x07/+0x08, the parameters, the delay, power at
+0x1A, the sort keys and the image) is at the same offset. Tango's reading of the record agrees (multi-game.md
§2.2).

### 3.3 NaviStats

0x60 bytes a side in EXE5 (the two blocks at 0x0203C880 and 0x0203C8E0; EXE6 0x64 at 0x0203CE00 and 0x0203CE64).
`fields.py navistats` pairs the field each call to a NaviStats accessor names. +0x01 to +0x3E (the buster, the
NaviCust programs' bytes, the custom level +0x0A, the folders +0x0B/+0x0C, the supports' +0x0D, the mood +0x0E,
the shoes and armors +0x1B to +0x23, the form +0x29, HP +0x40/+0x42) are the same offsets. Seen moved, one call
each (to be confirmed): +0x04 → +0x39 and +0x39 → +0x04 (a swap), +0x17 → +0x2C, +0x21 → +0x44, the patch-card
block +0x56 to +0x5B → +0x2D to +0x30 and +0x40, +0x5F → +0x34, +0x63 → +0x54. Four of 46 calls reading the form
+0x29 read +0x2C in EXE5 (`sub_800F09E`, the charged chip by form: EXE6's forms 1–4 and 8 are EXE5's souls there).

### 3.4 RAM

Everything moved, each structure by its own amount (`bmap.py ram`: the RAM addresses the same routines load,
and the tables of RAM pointers they load, the toolkit's among them; 567 addresses). The ones the trace format
reads:

| EXE6 | Name | EXE5 | Move |
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

BattleState, RNG1, RNG2, the hands and actor 0 agree with Tango's EXE5 support, and every row was checked on
running consoles (§10): the gauge fills to 0x4000, the pause flag is set through the turn's banners as EXE6's is,
the transform records hold the requesting navi at +8. Three layouts differ besides the moves:

- **The panels are 0x24 bytes** (EXE6 0x20): EXE5's panel getter (0x0800BD1C, ROM code; EXE6's is IWRAM code)
  multiplies by 0x24. The type and owner are at +2 and +3, as in EXE6.
- **The sound queue's entries start at +8** (EXE6 +0xC); the m4a players are EXE6's moved 0xBE0 on, and EXE5's queue
  has no conditional-music request (EXE6's `sub_8000822`).
- **The actor pool** (§3.1): 16 slots.

**IWRAM code:** EXE5 copies 0x1C58 bytes from 0x081C7A00 to **0x03005C00** (EXE6: 0x1ED4 bytes from 0x081D6000 to
0x03005B00).

## 4. The custom screen

**Shared** (EXE6 routine: EXE5 status):

- the per-console screen and its state machine, `sub_8026A28`: same (state block 0x02036B10, EXE6 0x020364C0);
- the slot layout's neighbor fill `sub_8027F42`, the deal's compaction `sub_802945A`: same;
- the keys `sub_8028B74`: similar 0.89; what can be picked `sub_8028E32`: 0.94; OK builds the hand
  `sub_8029110`: 0.99 (127 instructions each); the opening's slide `sub_8026B04`: 0.95; the sub-screens
  `sub_8029688`, `sub_8029788`, `sub_802983C`: 0.72–0.84;
- EXE6's DustCross scrap `sub_8027406` and ChpShufl re-deal's helpers `sub_80271F8`, `sub_802721C`,
  `sub_802723A`: the same code in EXE5 (presumably the sacrifice and re-deal machinery EXE5's own screens use,
  which EXE6 reused);
- the **dark chip** on the screen: the cursor starting on it `sub_802806C` the same, the hover's fade and music
  ramp `sub_802A2B0` 0.92. This is the code unverified.md lists as unreachable in EXE6 (no US chip has the dark
  flag): it is EXE5's.
- the folder shuffle `sub_800A570`: similar 0.74 (EXE6 adds the Tag chips).

**EXE6's own:** the OK result's form (`sub_8029344`, `sub_802937A`), the Cross window and its list (`sub_802A220`,
`sub_80275EC`, `sub_80280E0`), the hand-size rule with ChargeCross and NumbrOpn (`sub_802A40C`; EXE5's
`sub_802A49C` counterpart is similar 0.62), the opening `sub_8026840` and the round's custom memory: absent.

**EXE5's own:** the custom screen's entry (`sub_8009338`, similar 0.66) runs EXE6's screen (`sub_8026A28`) unless
battle flag 0x40 is on (`sub_800A8F8`, the same code): EXE5's operation battle, which its setter (0x0802D590) turns on
when the battle mode isn't 1 and the navi's stats' +0x2A is set (both navis in auto battle, each side its own gauge).
Then it runs EXE5's own screen, the Tactics screen (0x08025EF2, in a 2.9 KB block from 0x08025E4E, beside another of
3.2 KB from 0x080269A0). EXE6's flag 0x40 is another mode, its chip gate battle (each side its own gauge, which pays
for the chips slotted into its chip gate), never in a netbattle without gates. **Tango's Team Battles don't set it**
(the battle flags, battle state +0x32, read 0 in the traces): their custom screen is the shared one, and the operation
battle's own gauges and SELECT special don't run either.

**Soul Unison on the shared screen** (read in Team ProtoMan's code, driven in the chip lab, §10): Beast Out's
button under OK (slot 11, kind 2) is EXE5's Soul Unison button. After every pick and take-back (`sub_8028E32`'s
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
and can't use the navi chips (0xDD–0x118) but the DS ones; a light MegaMan can't use the dark chips or the DS navi
chips. Each such chip fizzles: the navi enters action 0x1A for a frame and a puff of smoke appears. Which branch
decides is not read yet.

## 5. Transformations: Soul Unison, the navi switch

- **The turn-start sequencer** (battle-flow.md §3.4.1): `sub_801483C`, `sub_80148CC`, `sub_8014944`,
  `sub_8014A00`, `sub_80147E4` (copying the exchanged transform records) and `sub_801482C` are the same in EXE5;
  `sub_801486C` similar 0.93, the fades `sub_80148EC` and `sub_801498E` 0.76 and 0.69. So EXE5's form change
  (Soul Unison) goes through the same transform record and the same fade-change-fade sequence as EXE6's Crosses
  and Beast Out. EXE6's Beast Out end check `sub_80159C6` is similar 0.57 in EXE5: the place a soul's turns run
  out, presumably.
- **The navi switch:** EXE6's Cross change (the pause handler's action 0x1C, `sub_802D714`, `sub_802D738`,
  `sub_802D7A0`, `sub_802D8F0`, the fall-back `sub_802DD2A`, `sub_802D926`, `sub_802D9B0`) is all in EXE5, the same
  or 0.97–1.00 similar. In EXE6 no custom screen sends it (custom-screen.md §6). **In EXE5 the custom screen does,
  from the Battle Chip Gate:** on every tick of the screen's state 4, 0x080259A0 asks the gate for an inserted navi
  chip (0x0812A074; in a link battle through the link, 0x08143CB8); a navi other than the current one (a set of
  rules: not used this battle, `0x02034E10`+14, and others) moves the screen to its state 0x40 (0x08023840), which
  sets the screen's navi (+0x10); the hand builder then writes the transform record's +4 (`sub_802DCD8`'s
  counterpart, EXE6's has no caller) and the pause handler switches. **Tango never inserts a gate chip** (its primer
  raises the gate-present flag and nothing else), so in Tango's Team Battles the navi switch is unreachable, as in
  EXE6. A chip lab scenario would need the gate's answer faked (the gate's RAM, not read yet).
- **The form record:** a navi's form data (the charged chips by form `sub_800F09E`, similar 0.50) reads NaviStats
  +0x2C in EXE5 where EXE6 reads +0x29, with other forms and families (EXE5's souls).

## 6. EXE5's own battle code

`only.py`: from the EXE5 counterparts of EXE6's in-scope routines, the calls, code pointers and tables into EXE5
routines with no EXE6 counterpart: 1,369 routines, about 88 KB, in 455 blocks. The largest, by where the shared
code reaches them:

| EXE5 block | Size | Reached from (EXE6 names) |
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
`target/exe5/only-<CODE>.tsv` in the verification workspace, per routine with where it is reached from.

### 6.1 Light and dark MegaMan

EXE5's light and dark MegaMan is one number, NaviStats +0x44 (a halfword, read through 0x08012882): the save's
light/dark meter. Tango's finished Team ProtoMan save has 500, its finished Team Colonel save 0, Tango's light
netplay templates 1000. Three rules read it, with two thresholds:

- **The chips a MegaMan may use** (0x08010118, EXE5's own; it passes any navi but MegaMan, NaviStats +0x29 ≠ 0). A
  chip record's byte +0x15 (EXE6's library sub-index, read only by the menus) is the chip's **`megaman`** field:
  0 either, 1 light only (meter ≥ 470), 2 dark only (meter < 470). Light only: the navi chips and their SP ones,
  GunDelSol, the Barriers, HolyPanl, BugFix, Snctuary, Otenko, JustcOne, MetrKnuk, HolyDrem, BigHook. Dark only:
  the DS navi chips, Static, Muramasa, Anubis, BlakWing, BugCurse, BugCharg. The same in both versions. The dark
  chips (flag 0x20) need a dark MegaMan as well, by their flag. A use the field refuses fizzles: the navi enters
  action 0x1A for a frame and a puff of smoke appears, nothing else.
- **A dark MegaMan on a holy panel turns it Normal** (0x08017136, EXE5's own): when the panel under the object is
  holy (EXE5's type 9) and its side's meter is ≤ 499, the panel is set Normal (the panel setter). It runs **every
  tick**, from the object's per-tick intake update (the counterpart of EXE6's `sub_801AC6C`, 0x080178EC, and of its
  five siblings for the other object kinds, `sub_801A9B8` to `sub_801ABB8`), so it takes the panel the first tick
  of the fight a dark MegaMan stands on one, and any holy panel he steps onto; a light MegaMan's never. The
  threshold is 500, not the chips' 470: a meter of 470 to 499 uses light chips and still clears holy panels.
- **The Soul Unison button** is not on a dark MegaMan's screen (observed; the check is not read yet).

**In nettai** this is an EXE5 system ("light and dark"): the meter is the side's state (from the save, by the
side's setup), the chips' rule is a hook on chip use, the holy panels' rule a per-tick hook on the side's navi, and
the chip field is an EXE5 extension on chip definitions, `extends = { chip = { megaman = "light"|"dark"|nil } }`
(a system's extension, rules-in-luau.md §7.5), nil meaning either. An EXE6 chip has none. An EXE6 MegaMan in a mixed battle has no
meter; what the system does with him is the rules work's to decide (the simplest: he is neither, and its rules
leave him out).

### 6.2 HolyDrem (0x133)

A Giga chip of both versions (action 0x15, sub-type 0x23, 50 damage), for a light MegaMan only (§6.1). Read in Team
ProtoMan's code and recorded both ways (the chip lab's `chips/0x133-holydrem/`):

- **Its parts:** the dimming spawner (`off_802CCB4`'s counterpart at 0x080297B8, entry 0x23: 0x080E6E78) makes the
  dimming controller (T4 kind 0x59, 0x080E6DFC: the dimming's start and end), whose attack phase (0x080E6E40)
  makes HolyDrem's actor (T1 kind 0x1C, 0x080BD728) on the user's position, with the chip's damage plus the
  modifiers' bonus.
- **The scan:** the actor fires one shot at once. Then it looks for holy panels over **the whole field, both
  areas**: from the column at the far end (x = 6 for side 0, x = 1 for side 1), rows 1 to 3, then the next column
  toward the user, until it runs off the field (0x080BD96C). A panel counts when its type is EXE5's holy (9). For
  each one found, 10 ticks later it shows an effect on it, **sets it Normal** (the panel setter, which refuses only
  a missing panel) and fires another shot. With none left it waits 60 ticks and ends.
- **The shots** (T3 kind 0x83, 0x080D6A60): bullets from the actor along **the user's row**, each stopping on the
  first thing it hits; 50 damage each (plus the modifiers). A target standing on a holy panel would halve it as any
  hit; in these recordings each target's panel was Normal by the time the shots came.
- **So:** the damage is 50 × (1 + the holy panels on the field), every holy panel ends Normal, and with no holy
  panel it is one shot of 50. An opponent out of the user's row takes nothing, though the panels still go.
- **Recorded** (Team ProtoMan's console, both sides): the default stage's holy middle row (side 0: 5 panels, as
  the dark Team Colonel turned its own Normal, 6 shots, 300; side 1 on exe5-team-light: 6 panels, 7 shots, 350);
  a stage without holy panels (one shot, 50); a HolyPanl first (it makes the panel ahead of the user holy: two
  shots, 100); the opponent a row up (no damage, all the panels Normal).
- **The first side-1 recording's no damage** was not the panels: it ran on exe5-team, where Team Colonel's MegaMan is
  dark, and HolyDrem fizzled before any actor existed.
- **In nettai:** the scan reads the panel type by what it is, the engine's `PanelType::Holy` (EXE5's 9 and EXE6's 5
  are both that), and the absorbing writes `PanelType::Normal`. So in a mixed battle it counts every holy panel
  whoever made it: an EXE6 HolyPanl's, an EXE6 holy stage's, its own side's and the opponent's. Nothing in it is EXE5's
  alone about the panels: no EXE5-only panel state, no owner or timer read, only the type and the setter EXE6 has
  too. What an EXE6 field wouldn't provide is the EXE5 rules around it: the light MegaMan it needs (§6.1), and the
  dark MegaMan turning holy panels Normal under him every tick (§6.1), which changes the count when a dark
  MegaMan stands on one.

### 6.3 Chips that need a set-up (read for the chip lab)

The chip lab's templates stand the opponent where each chip lands (verification workspace, tools/chiplab/
reach_exe5.py); a few chips hit nobody standing anywhere, and their code says what they need:

- **WavePit, RedWave, MudWave** (action 0x1A, sub-type 0x13, 0x080DAF18; the chip's parameter picks the panel):
  for each row, from the user's end of the field toward the other, the first panel with the kind's flag starts a
  wave along that row: a sea panel (flag 0x20000) for WavePit, a lava panel (0x1000) for RedWave, both then set
  Normal; for MudWave a panel without the standable flag 0x10 (a hole), left as it is. No such panel in a row, no
  wave there; none on the field, the chip does nothing.
- **The mode chips** (CannMode to DrilMode, FinalGun; action 0x1A, sub-type 0x0C, 0x0802D62A): they set the
  user's side's buster mode (the per-side block of EXE6's flag-0x40 gauges, `sub_802E070`: its +0x0B the chip's
  parameter, +0x2E 480 ticks, 360 in the operation battle). The damage comes from the buster afterwards.
- **Slasher** (action 0x29, 0x080ED328): while A is held, it looks over the whole field for the opponent's navi on
  a panel of the user's alliance (`sub_801273E`'s counterpart, panel flags: the opponent navi's body bit, and the
  panel's alliance); only then it strikes. So it needs the opponent standing on a panel of the user's area (the lab's
  user takes the opponent's panel with a PanlGrab).
- The others need what their kind does (recorded, not read): the time bombs' countdown, BoyBomb's bomb hit by its user's buster, the
  guards' reflection, Snake from holes in the user's area, Mine waiting for a step, the Anti traps (sprung by the
  opponent's chip of their element when it is used; a thrown BlkBomb doesn't spring AntiFire, a FireHit does),
  Muramasa's lost HP, Guardian's punishment, Jealousy's held hand, Poltrgst's obstacles, SerchMan's scope fired
  with A.
- **By chance:** Phoenix's fireballs fall on random panels of columns 2 to 5, counted from the left whichever
  side uses it (from side 1 most fall in its own area), and MetrKnuk's meteors on random panels too; the lab
  stands the opponent on a panel they hit with its RNG (recorded, not read).

### 6.4 The e-Reader cards' chips: LeadRaid and ChaosLrd

Two chips' text and colors are the save's, written when an e-Reader card is read: **LeadRaid (0x137)** and
**ChaosLrd (0x138)**. Read in Team ProtoMan's code; the four ROMs and both versions are the same.

- **What the ROM has:** their records (the chip table, as any chip): LeadRaid a Mega chip, code L, Null, 200 damage,
  MB 99, flags dimming and damage; ChaosLrd a Giga chip, code X, Null, 500 damage, MB 99, the same flags. Both use
  the navi chips' action (0x41, EXE6's 0x1B, the same code) with subtypes 0x15 and 0x1D: the summon table
  (`off_802CD5C`'s counterpart, 0x080298C4) spawns actor kinds 0x20 and 0x51. Their pictures (record +0x24: 0x0872F8E8,
  0x0872FE28) and icons (+0x20) are in the ROM. ChaosLrd's summon has a name in the battle name list (entry 0xE6,
  "Chaos Lord", JP ロードオブカオス); LeadRaid's none.
- **What the save has:** their names (the ROM's name entries are the text command `FF 00 n`), their descriptions
  (`FF 01 n`) and their pictures' palettes (record +0x28 points into EWRAM): slot n's name is a one-entry text archive
  at save +0x1D14 + 0x18 n, its description one at +0x1374 + 0x64 n, its palette 16 colors at +0x1660 + 0x20 n (the
  save is EWRAM from 0x02000000, so those are its addresses once loaded). LeadRaid is slot 0, ChaosLrd slot 1.
- **How the game gets them:** an e-Reader card read over the link: the card dispatcher (0x0812F3BC) passes the card's
  kind to 0x0812F8F0, which notes the card in the save's obfuscated bytes (EXE6's `encryption_8007004`, index 0x1020
  plus the slot; what reads that is not read), gives the chip once (`GetChipCountOfCode`, `GiveChips`: LeadRaid in L,
  ChaosLrd in X) and decompresses the card's name and description into the slot and copies its palette. A new game
  clears the slots (`sub_8021D36`, the same code as EXE6's): the palettes zeroed (a black picture), the names and
  descriptions "????". Owning the chip is the pack's count, as any chip's; its text and colors are the slot's.
- **The saves:** Tango's eight raw netplay saves (both versions, light and dark, US and JP) hold both chips' slots,
  the same in all (the US's "LeadRaid" "ProtoMan & Colonel together!" and "ChaosLrd" "Hatred formed into Bass", the
  JP's リーダーズレイド and ロードオブカオス), and so do the four GBA saves in Tango's saves folder (masked). EXE5 DS has
  no such block: its save doesn't hold them, and its ROM (exe5.nds) has both palettes (0x00B7EF40, 0x00B7EF60) and
  ChaosLrd's name, so the DS game's two chips are its own.
- **EXE6's card buffer is the same mechanism**, which EXE6 inherited: the same slot sizes (a 0x18-byte name archive, a
  0x64-byte description archive, a 0x20-byte palette), the same text commands, the same clearing routine; at other
  addresses (EXE6: names 0x02001180, descriptions 0x020007D0, palettes 0x02000AF0) and filled by a gift over the link
  rather than an e-Reader card (docs/engine/jp-differences.md).
- **In content/exe5** they are content-given chips as EXE6's gift chips are: their records from the ROM, their strings
  in the locales (English from Tango's US saves, Japanese from its JP saves), their palettes the definitions'
  `art_palette`, their slot compat's `save_slot`; gen_content.py's check reads them from the raw saves.
- **Recorded:** the chip lab's chips/0x137-leadraid and 0x138-chaoslrd (hit, adjacent, miss, side1). LeadRaid hits
  twice for 200 wherever the opponent stands (its row or not); ChaosLrd once for 500 wherever it stands. Both are
  either MegaMan's (+0x15 0).

## 7. Team ProtoMan and Team Colonel

`versions.py` (fmap.py's method between the two EXE5 ROMs): the battle code is the same, moved (mostly +0xE8,
also +0x4, +0x8, −0x8). The battle routines that differ: the chip use's `chip_800AEE8` and `sub_800AF34`, the
HUD's `sub_801CC34` and `sub_801E574`–`sub_801E6A8` (the emotion window: each version's soul faces, presumably),
`sub_80133EC`, two link routines (`sub_803F894`, `sub_803F8C4`), seven object kinds (`sub_80C6C14`,
`sub_80C9C5C`, `sub_80CBD32`, `sub_80D795C`, `sub_80DAA48`, `sub_80DB6D4`, `sub_80DDC30`: each version's souls'
moves, presumably), the save's encryption pair and one IWRAM routine. RAM is the same in both (Tango uses one
EWRAM table for all four EXE5 ROMs). As with Falzar and Gregar, what each version owns (its souls) is content and
setup, and its few routines are version branches.

## 8. Facts for the next steps

- **Hooks** (Team ProtoMan): the battle's frame routine (EXE6 `battle_8007800`) at 0x08006C10, the same code but
  for its callees (similar 0.82 only because its state table follows it); after its prologue 0x08006C1A; the
  main loop's frame start 0x080002D4 (as in EXE6); the m4a calls: SongNumStart 0x0814D634, MPlayAllStop
  0x0814D768, TempoControl 0x0814E698, PitchControl 0x0814E724, VolumeControl 0x0814E6BC, FadeOut 0x0814D58C,
  SongNumStop 0x0814D700, ImmInit 0x0814D82C, FadeIn 0x0814D800 (all the same code). **Two** link applets call
  the frame routine: 0x081359C4 (the plain NetBattle's, EXE6 `sub_812B698`'s counterpart by shape) and 0x0813B4DC
  (the Team Battle's: Tango's Team Battles return there).
- **Tango and Team Battles:** since 2026-08-05 Tango's EXE5 primer raises the Battle Chip Gate flag and confirms
  the comm menu's Team Battle row (チームバトル), so every Tango EXE5 match is a Team Battle (mode bytes 4–7), with
  Patch Cards on. **The user's decision (2026-10-02): Team Battle first**, as Tango plays it; a plain NetBattle
  later. The Team Battle is the Battle Chip Gate's mode (two consoles, each with a gate); Tango's runs the shared
  custom screen with Soul Unison, and its navi switch (from a gate's navi chip) never happens (§5). **A plain
  NetBattle is recorded too** (the chip lab's `exe5-netbattle` base: Tango's primer, then the gate's flag lowered
  and the comm menu's root cursor put on NetBattle; mode 2, a triple NetBattle). Its battle runs from the other
  link applet (0x081359C4) with the same BattleState and settings records; its custom screen is the shared one too.
- **Replays:** no EXE5 replay in Tango's current format. Three of 2022 in the oldest format (0x10) and six in
  format 0x11 (made with the exe5_gate patch), the same kind of savestate-started rounds the 2022 EXE6 replays are;
  two EXE5 DS replays (another platform, out of scope).
- **Saves:** the .sav files beside the US ROMs are blank; Tango's saves folder has finished US saves (Team
  ProtoMan light, Team Colonel dark), which the chip lab uses, and Tango carries light and dark templates for all
  four EXE5 ROMs.
- **The Japanese ROMs** (BRBJ, BRKJ) are not mapped yet (`bmap.py --to BRBJ` would).

## 9. Asset packs that load together (agreed)

The user wants EXE5 composable with EXE6 content. The rules design (rules-in-luau.md §7) and this proposal agree,
and the user confirmed (2026-10-02): every pack and content root declares its game (`game = "exe5"`); the loader
qualifies names as `<game>:<key>` (`exe5:cannon`) when roots load together; inside a root and its compat, keys and
asset names stay unqualified; version suffixes `-protoman`/`-colonel` and the region as a field; a root manifest
has `name` (its namespace) and `assets` (whose pack it resolves in, its own game by default). **No shared content
library:** every game exports its own content, even where it overlaps with EXE6's (`exe5:cannon` and `exe6:cannon`
are separate), so EXE5's assets are named for EXE5 alone. For the packs:

- **A pack says its game.** The manifest (content.toml) gains `game = "exe5"` (EXE6's packs `game = "exe6"`), the
  name a root's `assets` refers to. A loader given several packs keys them by it, and refuses two of one game.
- **Names inside a pack stay unqualified**, as EXE6's are now (`graphics/sprites/<name>`, assets.toml's names): a
  pack is one game's, so its own names can't collide. EXE5's names are EXE5's own, curated like EXE6's
  (compat/assets.toml in content/exe5), with placeholders for the rest.
- **Qualified when loaded together:** the engine's asset handles cover every loaded pack, keyed by
  `<game>:<name>` (`exe5:bomb` and `exe6:bomb` are different sprites), interned like definition keys. Inside a root,
  `asset.sprite("bomb")` means its own pack's; another game's asset is named qualified (`asset.sprite("exe6:bomb")`),
  allowed for the roots in its `requires`. A sprite's identity gains its pack (`SpriteId`), as §7.4 says.
- **Variants, as in EXE6's packs:** what differs by version is named with the version
  (`-protoman`/`-colonel`, as EXE6's `-falzar`/`-gregar`), and what a console of each version shows of its own
  goes in nettai-assets' `Versioned`. A pack keeps no region (one picture of each thing; EXE5's are the US ROMs',
  and what a Japanese console shows otherwise, background 0x05, is the verification's to know).
- **Sound** is per pack too: EXE5's songs and sound effects are EXE5's numbers in EXE5's m4a bank; a cue names
  `<game>:<song>` once qualified, and the audio loads each pack's bank. Two packs' banks never mix inside one
  m4a player (a song plays with its own pack's instruments).
- **Formats:** `nettai-content/hud` and `nettai-content/custom` were EXE6's layouts (multi-game.md §1.6). EXE5's
  HUD and custom screen are written in them with optional fields where EXE5 lays them out otherwise (§11: the
  faces' own boxes, the window's layout, the buttons by name), which an EXE6 pack writes none of.

## 10. Tracing EXE5 (as built)

The verification workspace traces EXE5 consoles as it does EXE6's, with the same line format:

- **oracle-trace** has the games `TeamProtoMan` (BRBE), `TeamColonel` (BRKE) and the Japanese `JpTeamOfBlues`
  (BRBJ) and `JpTeamOfColonel` (BRKJ), their hooks (§8; both link applets' returns are trapped) and a RAM
  `Layout` per game (§3.4; the Japanese ROMs' RAM is the US ROMs'). Every setup line says the game it is of, first
  (`"game":"exe6"`, `"game":"exe5"`), and both consoles' regions. An
  EXE5 setup line has EXE5's 0x60-byte NaviStats blocks
  and leaves out what is EXE6's alone (SP times, link navi levels, bug frags, event flags, Tag chips).
  It carries both consoles' NaviCusts (`navicusts`: each save's list, 0x02004D6C, the compression flags' bytes,
  event flags 0x1EC0 to 0x1FBF, whether the console's own compile left the HP, `cyberworld` (which no rule reads:
  §15.13), and the board's memory
  `expansions`, key item 0x61's count) and, when a console has any, their patch cards (`patch_cards`, each list's
  bytes). **exe5-compat replays such a round by compiling**: each MegaMan's recorded stats go back to what EXE5's
  reset leaves (`trace::reset`: the navi's fresh stats, the engine's `NaviStats::fresh`, with what the save keeps),
  and the round is set up with the
  NaviCust (on the board of the recorded `expansions`, or the rules' largest when they have fewer sizes) and the
  cards, as a match is; the emotion window's glitch is then the compile's and the cards', what
  the rules made, checked against the console's recorded flag (`emotion_window_glitches`). Recordings made before
  carry neither field and replay their stats as recorded, their glitch from the bugs in those stats.
  The hooks test checks every EXE5 hook against EXE6's code (masked for what moves, RAM included), Team Colonel's
  against Team ProtoMan's, and each Japanese ROM's against the US ROM of its version.
- **chiplab** runs EXE5 consoles from a base of EXE5 ROMs and saves (Tango's primer walks into a Team Battle),
  edits EXE5 saves (folder, Regular chip, navi, NaviCust, HP) and drives them (`custom CHIP soul` for Soul Unison);
  EXE5's navi stands idle in action 6 (EXE6 8). EXE5's scenarios are a library of their own (generated from the ROM's
  chip, Program Advance and soul tables), recorded apart from EXE6's lab: bases with a light and a dark MegaMan on
  either side (§4), every chip from both sides, the Program Advances, each soul's Soul and Chaos Unison.
- **difftest** takes EXE5 replays (none exists in Tango's current format yet).
- **The first traces:** a plain Team Battle, Team ProtoMan (traced) against Team Colonel, each picking the first
  chip dealt every turn and shooting until Team ProtoMan's navi is deleted: 3,216 frames, a full round from the
  intro to the deletion; and the same on Japanese consoles (Tango's netplay saves at 60 HP; 2,005 frames). Nothing
  replays them yet.
- Observed on the way: a Team Battle's custom screen keeps its state in the shared screen's block; Patch Cards apply
  in Team Battles (the Team ProtoMan save's take 150 off its max HP); the sound queue and panels differ (§3.4).

## 11. The EXE5 pack (as built)

`nettai-extract exe5 <pack-dir> <protoman-us> <colonel-us> <protoman-jp> <colonel-jp>` (any subset, identified by header,
as the same extractor takes EXE6's) writes a pack whose manifest says `game = "exe5"`:

- **What it has:** the 272 battle sprites (the same in both US ROMs), the field (11 panel types, EXE6 13; one
  highlight block for both highlights, EXE6 two), the 29 battle backgrounds with their scrolling and animations, the
  m4a bank (308 songs, 105 samples), the 368 chips' pictures and icons (and the invalid chip's, 0x185, past them),
  the HUD's 8x16 font, and the dialogue font (442 glyphs; EXE5's advance table is a word a glyph, EXE6's a byte).
- **The HUD** (exe5-extract's `hud.rs`): the layer from tile 0x180 and the gauge from 0x202 (EXE6 0x1A0, 0x222), the
  HP box, the gauge's frame and "L or R", the enemy digits, the chip icons' palette, the banners (49) with their
  digits, "Cstmzing...", "PAUSE", the HUD's text lines, the warning marker, the chatbox (its 20 tiles from 0x2EB,
  its maps and arrow) and the emotion window's faces: EXE5's 16 pictures a version (0-4 MegaMan by emotion, the
  order of EXE6's table, 0x0801AFB4; 5-10 the version's souls; 11-15 dark MegaMan), 0x180 bytes each, a face of
  MegaMan's or dark MegaMan's with its own 2x2 box (`mugshot_boxes`), a soul's beside the turns left (`counts`,
  0x0801985C: 10 - n), 22 palettes (11 more for Chaos Unison); Team Colonel's from its own ROM (0x087417FC...).
- **The custom screen** (`custom.rs`): EXE6's window and patch list but for the special slot, a 3x2 button; the
  window's parts at EXE5's tile numbers (`layout`: the name 0x59, the picture 0x69, the code 0x93, the element
  0x95, the digits 0x99, the slots 0x9F, the column's icons 0xE1 and cells 0x47, the turn limit 0x4B, the name
  bar 0x1B6), a hidden slot filled with tile 2, the cursor over OK at (0x58, 0x70) and over the button at (0x58,
  0x88) with their corners (0x08024714, 0x08024744); the 13 element icons in EXE5's family order, put in the
  engine's; the re-deal and scrap buttons; the soul button (the souls module's; `buttons`: its states' tiles
  0x086FBB64, its picture
  0x087322E8 with Soul Unison's and Chaos Unison's palettes, the souls' 2x2 icons 0x08749FB8, 14 with Chaos's, in
  sprite palette 13, 0x0874AAB8); the emblems (each ROM's seven, MegaMan's and its own team's six navis',
  with the thirteen navis' palettes of 8).
- **Versions:** 12 chips (0x12D–0x136, 0x139, 0x13A: each version's five Giga chips, and DethPhnx and Phoenix)
  are drawn differently by each version's ROM: a ROM holds the art of its own (the ones its library lists, the
  record's flag 0x40: Team ProtoMan's Bass, DeltaRay, BugCurse, HolyDrem, BigHook, DethPhnx; Team Colonel's
  MetrKnuk, OmegaRkt, BassAnly, CrossDiv, BugCharg, Phoenix) and has it again at the other's counterparts. The
  pack has each chip's picture and icon once, from its own version's ROM, under the chip's key, the picture with
  its `version`: either console shows it, and a console of the other version's is a known difference there
  (docs/frontend.md §5).
- **Version songs:** none. The two US ROMs play every song alike: the same header and commands, and voices
  that play alike on the keys of each track's notes (exe5-extract's `plays_alike`). Eleven songs (0x13C–0x142,
  0x145, 0x146, 0x170, 0x171: among them Football's, ChaosLrd's and DethPhnx's sounds) had been taken for Team
  Colonel's own: their MIDI is the same, and their voicegroups differ only in voices no note of theirs plays (a
  voicegroup is read 128 voices long, past the voices the game defines for it into the tables and data after
  them, where the two ROMs' bytes, pointers at other addresses in each, read as other silent or noise voices).
  The mechanism stays for a game that has some: a version's own song would be a song file named with
  its version (`sound-13c-colonel`, with `version` in its header and `base_version` in sound.toml), playing
  with the bank's voicegroups (the version's matched to equal ones of the bank, samples, waves and key maps by
  content, or added); nettai-content's `sound::SongVersions` holds them, and a pack without versions names no
  version anywhere.
- **Languages** (EXE6's shape, text-rendering.md §10): the strings are in `content/exe5/locales/en.toml` (from the
  US ROMs) and `ja.toml` (from the Japanese ROMs), keyed by definition key; and exe5-extract writes the Japanese
  ROMs' lettering beside the US's as nettai-assets' `HudLettering` and `CustomLettering` (its `lettering`, as
  exe6-extract's does), so an EXE5 console in Japanese (`--lang ja`) draws as Team of Blues and Team of Colonel do:
  - the 8x16 font and the dialogue font with its advances, in the Japanese encoding (compat/text.toml's `[jp]`);
  - the HUD's text lines;
  - the ten banners whose words differ (ROCKMAN, BLUES, SEARCHMAN and TODOMAN for MegaMan, ProtoMan, SearchMan and
    TomahawkMan, DELETED and WIN!; the Program Advance's katakana, twice), some starting elsewhere, and the
    banners' palette;
  - カスタム中… for "Cstmzing...", seven tiles wide for eight;
  - the chip window's pictures for OK and the re-deal button (チップが えらばれて いません; the shared tiles of
    "chip data transmission" and "BLOCKING!") with their palettes;
  - the soul button's label, "uni son" in two rows where the US ROMs' says "UNITE" (a named button's tiles by
    language: `CustomLettering::buttons`).

  The two Japanese ROMs' lettering is the same (the extractor checks it), and so is the rest of what a netbattle
  draws in all four ROMs: the HUD's and the custom screen's other blocks, the faces and emblems, every chip's icon
  and picture, the field (the verification workspace's tools/exe5/jpassets.py compares them block by block, by the
  literals of the routines the builds share).
- **Left out:**
  - The Japanese ROMs' one different sprite (14-17, the "BLOCK!" label, ブロック! there): the US release localized
    it rather than cut it, so the pack keeps the US's, as EXE6's does; no content draws it.
  - The Japanese ROMs' background 0x05, which is another picture than the US ROMs' (bubbles in the dark for the
    goldfish): no netbattle's stage shows it, and the pack keeps the US's. The extractor says both.
  - "Interval..." and "Strat Change..." (インターバル中…, サクセンヘンコウ中…), which a Team Battle shows where a
    netbattle shows "Cstmzing...": the pack has neither language's.
- **Names:** placeholders (`sprite-0c-2d`, `sound-10e`, `chip-12d`; a glyph's number in brackets) until an EXE5 content
  root names them in its compat, as EXE6's does.
- **Shared decoding:** `nettai-extract` now owns one ROM/LZ77 reader, sprite and portrait decoder, field/background
  decoders, GFX-animation decoder, banner and patch-list decoder, and pack writer for EXE5 and EXE6. Game modules
  retain their address tables and version/region selection. Its library accepts ROM bytes and produces typed assets
  or pack files; the CLI accepts any subset and generates placeholders for unavailable sources.
- nettai-content's HUD reader accepts a pack without the count box (`no_count_box`: EXE5 has none).

## 12. Reproducing

In the verification workspace, with the ROMs in `$EXE6_ROMS` and the EXE6 disassembly's symbols in `$BN6F`:

    tools/exe5/bmap.py [--to BRKE] [ram]    # target/exe5/bmap-<CODE>.tsv, ram-<CODE>.tsv, the summary
    tools/exe5/bmap.py diff NAME...         # an EXE6 routine beside its counterpart
    tools/exe5/fields.py [r5 | navistats | after NAME]
    tools/exe5/only.py                      # EXE5's own battle code
    tools/exe5/versions.py [BRBE BRKE]      # the two versions

`target/audit/classes.tsv` (the audit, `tools/audit/audit.py`) gives the areas and classes; without it the map is
the same, unclassified.

## 13. EXE5's root and compat (as built, before R)

What EXE5 verification and content need that doesn't depend on the roots (rules-in-luau.md R), shaped to R1's
format.

- **content/exe5** (written by the verification workspace's tools/exe5/gen_content.py from all four ROMs; its `check`
  mode compares them again): `root.toml` (name "exe5", assets "exe5"; it requires EXE6's root since the port, §15.1); `compat/chips.toml` (328 chips by
  key: id, action and subtype; `damage_formula` for the 46 whose damage is a formula, 1000 and up; `colonel` for the
  48 whose Team Colonel record differs: the version Gigas' library flag, the navi chips' +0x16);
  `compat/panels.toml` (EXE5's 11 panel types, their flag words and the engine's type each is);
  `chips/<key>/chip.luau` (each chip's common record and its `megaman` extension, §6.1; no use yet; since §15.12 a
  series' ported chips share chips/<series>/chips.luau, and the generator finds each definition by its id); and
  `locales/en.toml`, `ja.toml` (names and descriptions from the US and the Japanese ROMs). The records are Team
  ProtoMan's: the Japanese ROMs' differ only in the library's sort keys (+0x18), Team Colonel's only where compat
  says, and both versions' strings are the same. Keys follow EXE6's: the US name in lower case, `+` and spaces as
  `-`, a navi chip's SP or DS split off (`gyroman-sp`); the second CannBall (the mode chip, 0x11A) is
  `cannball-mode`. Rewriting keeps a ported chip file's code: only the record's fields are rewritten.
- **exe5-compat** (the engine never depends on it): `Compat::exe5()` (content/exe5/compat built in) and `read`; keys
  unqualified inside, qualified `exe5:<key>` at its boundary (`qualify`, `strip`, `Compat::chip`); EXE5's pool sizes;
  the codec (the 0x60-byte NaviStats with the light/dark value and its two thresholds, the panels by compat's
  table, the chip blocks); and, with the `trace` feature, the chip lab's recordings read and decoded. The
  verification workspace's trace-tests `exe5_lab` decodes every recording in data/traces/lab-exe5 (1,371 of them,
  1,134,927 frames).

**Waiting on R** (and the port): the loader reading content/exe5 (its manifest, qualified keys, the locales by
root); the chips' handles (the codec gives ids and qualified keys); a use for each chip (`define.chip` asks for
one, so the generated records don't load until the port writes them, or the loader takes data-only chips); the
`megaman` field as the light-and-dark system's extension (S7's `extends`); the type check of content/exe5 (EXE6's
core.d.luau has no `megaman`); EXE5's kinds (no kinds.toml: objects decode to their pool and EXE5's kind number);
EXE5's stages (the settings record stays raw).

**What has no engine counterpart yet:**

- *Chips:* `megaman` (+0x15); the families recovery and invisible (the engine's `ChipFamily` has neither; EXE5's
  obstacle family is EXE6's summon); EXE5's damage formulas (its own table: compat keeps the row); +0x16 (EXE6's extra
  flags byte: EXE5's bits unread, written as numbers) and +0x17 (EXE6's lock-on mode: EXE5's byte, mostly 0x10,
  unread); a version's own record (Team Colonel's differences).
- *Panels:* metal (type 5: EXE6's road flag, its look a plate, its behavior unread) and sea (type 10, flag
  0x20000); the panel record's 0x24 bytes (the lava and sea timers the panel setter starts, at +0x10 and +0x14).
- *Pools:* the actors' 16 slots (the engine's `object::SLOTS` is one number, 32).
- *NaviStats:* the light/dark value (+0x44); the weapon bytes' EXE5 meaning (+0x04, +0x05, +0x07, +0x39: EXE6's
  buster and +0x39 swap places in the one call each that pairs them); and the bytes whose EXE5 meaning isn't read:
  +0x00, +0x0F, +0x11 to +0x1A and +0x24 (paired with EXE6's NaviCust bug bytes at the same offsets, EXE5's bugs not
  checked), +0x1E to +0x22, +0x25 to +0x28, +0x2A, +0x2D to +0x38 (EXE6's folder bytes; EXE5's patch-card routine
  writes there), +0x3A to +0x3D, +0x46 to +0x5F. EXE6's fields EXE5 has elsewhere or not at all: the starting form
  (EXE6 +0x17, EXE5 +0x2C), the Tag chips, ChpShufl and NumbrOpn, the Beast Out counter, the version byte, the sun,
  the hand-shrink bug.
- *Setup:* the BattleSettings record (EXE5's stages by their own numbers), BattleState (0xF0 bytes; the traces carry
  it raw), the versions and regions (both decoded).

## 14. The action map: EXE5's chips against EXE6's code

For each of EXE5's 330 chips, the EXE6 code its use descends from and how the two differ, from the disassembly: what
a port can take from EXE6's Luau (`require("@exe6/...")` with EXE5's assets and records) and what it must write. Made
by the verification workspace's `tools/exe5/actions.py`; the machine-readable map is its `tools/exe5/chip-actions.tsv`
(a row per chip: the roots in both games, the EXE6 chips and module, class, relation, every differing region and
constant with its routine and offset, the data tables that differ, the kinds paired, the labs' verdicts),
`chip-library.tsv` (the shared routines, §14.3) and `chip-systematic.tsv`. `actions.py --diff KEY` prints a chip's
differing routines side by side, marked with what ran in the labs.

### 14.1 Method

- **Roots.** A chip's use runs its action's handler (EXE5's action table 0x080EB42C, EXE6's `JumpTable80EAC60`; the
  games number actions apart: EXE5's Thunder is 0x20, EXE6's 0x1F) and, where the handler dispatches by the chip's
  subtype, the subtype's routine: the dimming chips' spawners (EXE6 `off_802CCB4`, EXE5 0x080297B8), the navi chips'
  summons (`off_802CD5C`, 0x080298C4), and any code-pointer table the handler, what it calls or its states index
  with the attack's subtype byte.
- **Candidates.** The EXE6 handler likest EXE5's and its table's likest entry, and the routine of the EXE6 chip of the
  same name, if there is one. Names only propose; code decides. "Likest" weighs the code's ratio (exact
  instruction forms) together with the ratio of what it calls, loads (state tables) and spawns at the same places,
  four levels down, each weighted by size, so a dispatcher or a spawner stub weighs little against the states and
  objects it reaches. Object kinds pair where they are spawned (`movs r0, #kind` then a spawner), because the games
  number kinds apart.
- **The walk.** From a candidate's roots, both games' code in step. Each routine is decoded by flow. Each pair is
  compared as §1 compares: same, consts, similar or differs. Coverage limits the walk:
  - EXE6's chip lab records block coverage of its chips' runs, and EXE5's now does too, from a list of block leaders
    (`actions.py --blocks`, chiplab's `CHIPLAB_COV_EXE5`).
  - The walk stops at a pair that neither lab ran.
  - A table indexed by a parameter pairs the entry EXE5's lab ran with the likest entry EXE6's ran.
  - The EXE6 chips whose labs stand for EXE6 are the chip's namesake in the family, else the family's chips whose
    labs ran the most of the walk.
- **The choice.** The candidate whose walk is likest where both labs ran wins. EXE5 code that ran unpaired counts as
  unlike. Under 0.6 (0.45 when the family has the chip's name) the chip is EXE5's own.
- **What differs, where it ran.** A pair's differences are regions (instructions inserted, removed or changed, and
  constants). A region counts only where one of the labs ran it; `off_path` counts the rest.
  - The library's routines are listed apart (§14.3): those that walks from two EXE6 roots reach, and those EXE6 calls
    from eight places or more.
  - Systematic constants are listed apart too: alike in three routines or more (collision data +0x70 is EXE5's
    +0x68 in 17 routines; a value 8 is 6 in 9).
  - Relocated addresses aren't differences. Data tables the code reads (by address) are compared bytewise and
    listed as `data`. They are often subtype-indexed, so the chip's own entry may still be the same (Cannon's
    shot-parameter table differs only past EXE5's three cannons).

| class | meaning |
|---|---|
| identical | its own code is EXE6's (calls, addresses and object kind numbers aside) |
| identical-run | it differs only where neither lab ran (Cannon: GigaCannon's branch of the shot) |
| constants | where it ran, only constants differ: sounds, sprites, a call's value, field offsets |
| changed | where it ran, instructions differ (each region with what EXE5 adds and drops) |
| exe5-only | no EXE6 code is as like as 0.6 |

| relation | meaning |
|---|---|
| same chip | the EXE6 ancestors include the chip's name |
| built like | the nearest EXE6 code is another chip's: a renamed one (WideSht1–3 and WideSht, AirHoc and AirHocky, Geddon1–3 and Geddon, DrilArm1–3 and DrilArm), a variant (DarkWide on WideSht, Guard on Reflector), or a navi chip on another navi's template |
| EXE6 code, no chip | it descends from code EXE6 keeps but no EXE6 chip uses (EXE5 leftovers in EXE6's tables) |

- **The labs' timelines.** EXE5's lab recordings are compared with the ancestor's EXE6 recordings (hit, miss,
  adjacent, side1) frame by frame, starting from the chip's use: the user's navi entering the action's state. Each
  category is compared on its own:
  - hits, by frame and side; their amounts are the records' damage, listed where they differ;
  - actors and attacks spawning and going;
  - the navis' states and the user's animations, with standing and the action renumbered;
  - effects.

  `lab`:
  - **same**: every category matches.
  - **effects**: only effects' frames differ.
  - **navi**: hits and objects match; the navi states don't.
  - **differs**
  - **none**: no recording in one of the games.
- **Checked.** The coverage traps don't change a run. All 1,253 chip recordings of the EXE5 lab, recorded with the
  traps and without, are byte-identical. One trap that did change them, on a byte table at an odd address the
  discovery took for a routine, is why an odd literal is only followed to a `push`.

### 14.2 Results

| relation | identical | identical-run | constants | changed | exe5-only | chips |
|---|---|---|---|---|---|---|
| same chip | 14 | 7 | 8 | 93 | | 122 |
| built like | | | 1 | 100 | | 101 |
| EXE6 code, no chip | 7 | | | 10 | | 17 |
| exe5-only | | | | | 90 | 90 |

Labs: 199 chips have recordings in both games. Of the 122 same chips, 20 match in every category, 28 in all but
effects' frames, 38 in hits and objects (most of these are dimming chips, §14.3), and 28 differ.

- **Reusable as they are (identical, identical-run):** MiniBomb, EnergBom, MegEnBom; Cannon, HiCannon, M-Cannon;
  PanlGrab, AreaGrab; PnlRetrn, HolyPanl, Snctuary; Invisibl; AntiNavi, AntiDmg, AntiSwrd, AntiRecv; GrabBnsh;
  SloGauge, FstGauge; FullCust; Meteors. For example, MiniBomb's hit is the same in both labs at every event: the
  use, the throw at +1, the bomb at +10, the hit for 50 at +50, the explosion's effect from +49 to +72.
- **Constants only:** AirShot (the arm's value 0x13 → 0x08), Silence, Discord, Timpani (a sprite 0xA → 0x8),
  Mine and GrabRvng (an effect's number), ProtoMan and ProtomnSP (an effect and a sprite).
- **Changed but the same in the labs (22, 16 of them same chips):**
  - same chips: Recov10 to Recov300 and DrkRecov (EXE5 also adds the use's +0x0A to NaviStats +0x0E), TankCan1–3,
    Tornado, Static, BugBomb, Navi+20;
  - built like: Guard1–3 (Reflector), Atk+10, Atk+30, DarkTorn.

  The differences there are code the labs ran with no effect on what they record.
- **Different in the labs (28 same chips).** Damage alone doesn't count: HiCannon's 100 is EXE5's 80, from the
  record. The causes:
  - the code: Thunder's ball; the Vulcans' last bullets; GunDelSol's actor, which leaves at +19 in EXE5 and +80
    in EXE6;
  - the labs' set-ups: AirSpin's and AreaGrab's miss is a hit in EXE5, and EXE6's user loses 1 HP at a time with
    the dark chips, Bass, DeltaRay and BigHook;
  - chance: Meteors.
- **EXE5-only (90),** by content key:
  - navi chips: napalmmn, magnetmn, meddy, shadoman, knightmn, larkman, gridman, django (with their -sp and -ds);
  - the e-Reader chips (§14.5): leadraid, chaoslrd;
  - chips EXE6 dropped: mrkcan1–3, pulsar1–3, spshake1–3, quake1–3, cannball, slasher, moonbld1–3,
    redfrut1–3, skully1–3, crakout, dublcrak, tripcrak, aqwhirl1–3, sidebub1–3, elcreel1–3, cusvolt1–3,
    crsshld1–3, wavepit, redwave, mudwave, woodnos1–3, hotbody1–3, cacdanc1–3, phoenix, dethphnx;
  - dark and bug chips: jealousy, poltrgst, bugcurse, bugcharg, holydrem, magnum, rainyday, elemrage, copydmg.

### 14.3 What differs for many chips (the library)

32 of the 90 shared routines differ where the labs ran (`chip-library.tsv`).

- **The dimming handler** (action 0x15, EXE6 `sub_80EBD9C`, EXE5 0x080EC318, 68 chips): EXE6 spawns the dimming
  object, stays in the action state and leaves it (`object_exitAttackState`) on its next update, which is after the
  dimming; EXE5 spawns it and leaves on the same frame. So in EXE5 the user stands during the dimming (the labs'
  `navi`: EXE6's user leaves the action at +129 to +164, EXE5's at +1).
- **The object handler of action 0x1A** (EXE6 `sub_80EC39C`, EXE5 0x080EC6F6: Boomer, Lance, FireHit, BusterUp, the
  AtkPlus chips, FullCust, JustCone, the mode chips and others; 32 chips): EXE6 calls the subtype's routine once,
  keeps the action 8 more frames for subtype 20 and then leaves; EXE5 calls it and leaves on the same frame.
- **The navi chips' framework** (`sub_80E192C`, `sub_80E18F8`, `sub_80E1854`, 45 chips): the same code with the
  games' numbers where it singles chips out: Roll's subtype (EXE6 0, EXE5 0x19), BigHook's (0x17, 0x1A), AntiRecv's
  id (0xBD, 0x93). `sub_800BA8A` (the navi telop) checks AntiNavi by id (0xBA, 0x90). The navi attacks' helpers
  `sub_80E292C`, `sub_80E28C8` single out navi subtypes EXE5 numbers two higher (EXE6 6, 7, 8, 10, 11, 12), and EXE6's
  `sub_80E28C8` adds, for side 1, a swap of subtypes 17 and 18. `sub_80E376C` (a navi chip's leaving, 4 chips)
  takes the chip's HP cost as EXE6 does and also subtracts its +0x2E from NaviStats +0x0E (to at least 1, when not 0)
  through 0x08012820; Recov adds to the same byte through 0x08012802 (§14.2). EXE6's counterpart of the subtracting
  one is `sub_8015C12`.
- **The swords** (`sub_80EBAE8`, `sub_80EBB34`, 14 chips): NaviStats +0x2C picks the slash and a count. In EXE6 its
  values 11 to 23 (its forms) give slashes 12 or 13, and the count comes from a subtype table. In EXE5 souls 1, 7
  and 8 give slashes 13, 14 and 15; the count is 3, or 10 with soul 1 and 50 with soul 7.
- **The shot object** (EXE6 attack kind 0, `sub_80C4E7C`, `sub_80C4F02`: Cannon, AirShot, GigaCannon; 7 chips): EXE6's
  has variants EXE5's doesn't (12: its own handling when the shot leaves the field or hits; 34 and 36: they set the
  panel's type), and the crack and break variants are EXE5's 24 and 25 (EXE6 21 and 22).
- **The bombs' throw** (`sub_80EB644`, 14 chips): EXE6 passes a subtype-dependent parameter in the thrown object's
  top byte (3 for subtype 15, the use's +0x0C × 3 for subtype 14); EXE5 passes none.
- **The hit-effect spawner** (`sub_80E33FA`, 26 chips): EXE6 sets bit 2 of the spawned effect's flags; EXE5 doesn't.
- **The Vulcans' bullet** (`sub_80C6964` to `sub_80C6AB8`, 8 chips): other animation numbers and collision values,
  and EXE6 sets a status effect (`object_setCollisionStatusEffect1`) that EXE5's doesn't.
- **Bass and BassAnly's shared actor helpers** (`sub_80C4550` to `sub_80C468C`): EXE6 loads animations, palettes and
  a check EXE5's don't.

### 14.4 The map

One row per EXE6 routine that EXE5's chips descend from. The columns:

- the EXE6 chips and module (content/exe6);
- the EXE5 chips, marked `=` (the same chip) or `~` (built like it);
- their classes, counted: I identical, Ir identical-run, C constants, X changed;
- the labs' verdicts;
- where the chips' own code differs, by EXE6 routine.

chip-actions.tsv has every region; `actions.py --markdown` writes this table.

| EXE6 routine (chips; module) | EXE5 chips | code | lab | what differs where it ran |
|---|---|---|---|---|
| `sub_80EBC0E` (cannon, hicannon, m-cannon, gigacan1, gigacan2, gigacan3, …; chips/cannon, chips/gigacan) | =cannon, =hicannon, =m-cannon, =gigacan1, =gigacan2, =gigacan3 | Ir3 X3 | effects 3 none 3 | code: `sub_80EBC28`; constants: shift ×3, literal ×3; library: `sub_80C4E7C`, `sub_80C4F02`, `sub_80E33FA` |
| `sub_80EC884` (airshot; chips/airshot) | =airshot | C1 | effects 1 | constants: value ×1; library: `sub_80C4E7C`, `sub_80C4F02` |
| `sub_80ECCB0` (airhocky, pithocky; chips/airhocky) | ~airhoc, ~pithoky | X2 | effects 1 none 1 | code: `sub_80ECCCC`; constants: value ×2, offset ×2, sprite ×2, literal ×1; library: `sub_80E33FA`, `sub_80ECA0C` |
| `sub_80CA4F6` (boomer, hiboomer, m-boomer; chips/boomer) | =boomer | X1 | differs 1 | code: `sub_80CA2D4`, `sub_80CA408`; constants: sprite ×1, value ×1; library: `sub_80EC39C` |
| `sub_80E5EA8` (fanfare, discord, timpani, silence; chips/discord, chips/fanfare, chips/silence, chips/timpani) | =silence, =fanfare, =discord, =timpani, ~drksonic | C3 X2 | navi 5 | code: `sub_80D43E8`, `sub_80D435C`; constants: sprite ×5; library: `object_spawnCollisionRegion`, `sub_80EBD9C` |
| `sub_80ED454` (tornado, static; chips/tornado) | =tornado, =static, ~darktorn | X3 | same 3 | code: `sub_80C9F98`; constants: value ×8 |
| `sub_80ED55C` (widesht, suprspr; chips/widesht) | ~widesht1, ~widesht2, ~widesht3, ~darkwide, ~suprspr1, ~suprspr2, ~suprspr3 | X7 | differs 4 none 3 | code: `sub_80ED5BE`; constants: value ×14, sprite ×7, literal ×4; library: `sub_80E33FA` |
| `sub_80EBF10` (vulcan1, vulcan2, vulcan3, suprvulc; chips/vulcan) | =vulcan1, =vulcan2, =vulcan3, =suprvulc, ~infvulc1, ~infvulc2, ~infvulc3 | X7 | effects 1 differs 3 none 3 | code: `sub_80EBF30`; constants: value ×7; library: `sub_80C6964`, `sub_80C69AC`, `sub_80C6A50` … |
| `sub_80ECBB0` (spreadr1, spreadr2, spreadr3; chips/spreadr) | ~spreader | X1 | effects 1 | code: `sub_80ECBCC`; library: `sub_80C6964`, `sub_80C69AC`, `sub_80C6A50` … |
| `sub_80EC7A6` (thunder, darkthnd; chips/darkthnd, chips/thunder) | =thunder, =darkthnd | X2 | differs 2 | code: `sub_80C940C`; constants: value ×6, sprite ×4, branch ×2 |
| `sub_80CE44E` (grasseed, iceseed, poisseed; chips/grasseed, chips/iceseed, chips/poisseed) | =iceseed, =grasseed, ~lavaseed, ~seaseed | X4 | navi 3 differs 1 | code: `sub_80CE270`; constants: literal ×4, branch ×4, offset ×4; library: `sub_80EB644` |
| `sub_80E7464` (lifesync; chips/lifesync) | =lifesync | X1 | navi 1 | code: `sub_80E72C8`; constants: branch ×1; library: `sub_80EBD9C` |
| `sub_80C5DBC` (minibomb, energbom, megenbom, bigbomb; chips/energbom, chips/minibomb) | =minibomb, =energbom, =megenbom | I3 | same 3 | library: `sub_80EB644` |
| `sub_80EDAE0` (gundels1, gundels2, gundels3, gundelex; chips/gundels) | =gundels1, =gundels2, =gundels3, =gundelex | X4 | differs 4 | code: `sub_80EDB14`; constants: literal ×4, value ×4, offset ×4 |
| `sub_80D49F6` (vdoll; chips/vdoll) | ~crakbom, ~parabom, ~resetbom, =vdoll | X4 | differs 3 effects 1 | code: `sub_80D49F6`, `sub_80D4754`, `sub_80D4870`, `sub_80D4888`; constants: value ×4; library: `sub_80EB644` |
| `sub_80D9FA8` (bugbomb; chips/bugbomb) | =bugbomb | X1 | same 1 | code: `sub_80D9FC2`; constants: value ×5, other ×2, sound ×1, branch ×1, literal ×1; library: `sub_80EB644` |
| `sub_80CD886` (blkbomb; chips/blkbomb) | ~geyser, =blkbomb | X2 | differs 1 effects 1 | code: `sub_80CD50C`, `sub_80CD5F8`, `sub_80CD700`, `sub_80CD886` (+3 more); constants: value ×4, offset ×2, literal ×2, branch ×2; library: `sub_80EB644` |
| `sub_80EB776` (sword, wideswrd, longswrd, wideblde, longblde, fireswrd, …; chips/drksword, chips/lifesrd, chips/longblde, chips/longswrd, chips/muramasa, chips/stepswrd, chips/sword, chips/wideblde, chips/wideswrd) | =sword, =wideswrd, =longswrd, =wideblde, =longblde, ~custswrd, ~katana1, ~katana2, ~katana3, =drksword, =muramasa, ~z-saver, =lifesrd | X13 | effects 6 differs 6 none 1 | code: `sub_80EB862`, `sub_80EB79C`, `sub_80EBB78`, `nullsub_12` (+3 more); constants: offset ×43, value ×18, shift ×12, branch ×6, sound ×1; library: `sub_80E33FA`, `sub_80EBAE8`, `sub_80EBB34` |
| `sub_80EE192` (windrack; chips/windrack) | =windrack | X1 | differs 1 | code: `sub_80EE1AC`; constants: value ×2; library: `sub_80EBAE8` |
| `sub_80EF62E` (varswrd; chips/varswrd) | =varswrd | X1 | effects 1 | code: `sub_80EF6FC`; constants: value ×6, branch ×5, sound ×1 |
| `sub_80ECACA` (tankcan1, tankcan2, tankcan3; chips/tankcan) | =tankcan1, =tankcan2, =tankcan3 | X3 | same 3 | code: `sub_80CB7BC`, `sub_80CB71C`; constants: value ×6, literal ×3 |
| `sub_80ED374` (drilarm; chips/drilarm) | ~drilarm1, ~drilarm2, ~drilarm3, ~darkdril | X4 | differs 4 | code: `sub_80ED3B6`, `sub_80D2B8E`, `sub_80D2AB4`, `sub_80D2B2C` (+1 more); constants: value ×4, branch ×4 |
| `sub_80E3242` (timebom1, timebom2, timebom3, timebom; chips/timebom) | =timebom1, =timebom2, =timebom3, =timebom | X4 | navi 3 none 1 | code: `sub_80E3264`, `sub_80CDA1C`, `sub_80CDB2C`, `sub_80CD9CA` (+2 more); constants: branch ×10, offset ×10, literal ×6, value ×6, sound ×4; library: `sub_80EBD9C` |
| `sub_802E1BE` (no EXE6 chip) | voltz1, voltz2, voltz3, cannmode, cannball-mode, swrdmode, yoyomode, drilmode, finalgun | X3 I6 | none 9 | code: `sub_802E1BE`; library: `sub_80EC39C` |
| `sub_80D2596` (lance; chips/lance) | =lance, ~drklance | X2 | differs 2 | code: `sub_80D2514`; constants: value ×2, sprite ×2; library: `sub_80EC39C` |
| `sub_80EC02A` (yoyo, greatyo; chips/greatyo, chips/yoyo) | =yoyo, =greatyo | X2 | effects 1 none 1 | code: `sub_80CE932`, `sub_80E8612`, `sub_80CEA42`, `sub_80CE9A4` (+5 more); constants: value ×10, branch ×5, literal ×2 |
| `sub_80E3128` (wind, fan; chips/wind) | =wind, =fan | X2 | navi 2 | code: `sub_80CD310`, `sub_80CD414`, `sub_80CD236`; constants: sprite ×2; library: `sub_80EBD9C` |
| `sub_80E46B6` (rockcube, icecube; chips/rockcube) | ~boybomb1, ~boybomb2, ~boybomb3, =rockcube, ~omegarkt | X5 | differs 4 navi 1 | code: `sub_80E4678`, `sub_80CFB2C`; constants: value ×4, branch ×2, offset ×2, literal ×1, sprite ×1; library: `sub_80EBD9C` |
| `sub_80ED13E` (rflectr1, rflectr2, rflectr3; chips/rflectr) | ~guard1, ~guard2, ~guard3 | X3 | same 3 | code: `sub_80ED154`; constants: branch ×3; library: `object_genericDestroy`, `sub_80C4FFE` |
| `sub_80E64E8` (colorpt, dblpoint; chips/colorpt) | ~metagel, =colorpt, =dblpoint, ~blakwing | X4 | differs 2 navi 2 | code: `object_dimScreen`, `sub_80E667C`; constants: offset ×2; library: `sub_80EBD9C` |
| `sub_80E59C6` (snake; chips/snake) | =snake | X1 | differs 1 | code: `sub_80D2C40`, `sub_80D3048`, `sub_80D2EDC`; constants: value ×4, branch ×2, offset ×1, literal ×1; library: `sub_80EBD9C` |
| `sub_80E7600` (circgun; chips/circgun) | =circgun, ~darkcirc, ~piledrvr | X3 | differs 2 none 1 | code: `sub_80E75AC`, `sub_80D65FC`, `sub_80D6580`, `sub_80D677C` (+1 more); constants: sprite ×6, offset ×6, branch ×5, value ×4; library: `sub_80EBD9C` |
| `sub_80E349E` (mine; chips/mine) | =mine, ~bodygrd | C1 X1 | navi 1 none 1 | code: `sub_80E3470`; constants: value ×2, offset ×1, branch ×1; library: `sub_80EBD9C` |
| `sub_80E5A64` (no EXE6 chip) | astroid1, astroid2, astroid3, darkmetr, boxer1, boxer2, boxer3 | X7 | none 7 | code: `sub_80E5A22`, `sub_80E5A08`, `sub_80E5A64`; constants: value ×15, offset ×8, branch ×3; library: `sub_80CF3DC`, `sub_80EC39C` |
| `sub_80EC844` (recov10, recov30, recov50, recov80, recov120, recov150, …; chips/drkrecov, chips/recov) | =recov10, =recov30, =recov50, =recov80, =recov120, =recov150, =recov200, =recov300, =drkrecov | X9 | same 9 | code: `sub_80EC844`; constants: branch ×9 |
| `sub_8010820` (busterup; chips/busterup) | =busterup | X1 | effects 1 | code: `sub_8010820`; constants: value ×1; library: `sub_80EC39C` |
| `sub_80E07E0` (panlgrab, areagrab; chips/areagrab, chips/panlgrab) | =panlgrab, =areagrab | I2 | navi 1 differs 1 | library: `sub_80EBD9C` |
| `sub_80E2D76` (grabbnsh, grabrvng; chips/grabbnsh) | =grabrvng, =grabbnsh | C1 Ir1 | navi 2 | constants: value ×1; library: `sub_80EBD9C` |
| `sub_80E24B8` (slogauge, fstgauge; chips/fstgauge, chips/slogauge) | =slogauge, =fstgauge | Ir2 | navi 2 | library: `sub_80EBD9C` |
| `sub_80E2B5A` (pnlretrn, holypanl, snctuary, comingrd, goingrd; chips/comingrd, chips/goingrd, chips/holypanl, chips/pnlretrn, chips/snctuary) | =pnlretrn, =holypanl, =snctuary, ~elempowr | I3 X1 | navi 4 | code: `sub_80E2B2C`, `sub_80E2B5A`; constants: value ×1, offset ×1, branch ×1; library: `sub_80E28C8`, `sub_80E292C`, `sub_80EBD9C` |
| `sub_80E2566` (geddon, prpcapsl, pnkcapsl, healball, magpanl, beastout-dimming-1, …; chips/geddon) | ~geddon1, ~geddon2, ~geddon3 | X3 | navi 2 differs 1 | code: `sub_80E2528`; library: `sub_80EBD9C` |
| `sub_80E2F24` (no EXE6 chip) | blinder | I1 | none 1 | library: `sub_80EBD9C` |
| `sub_80D8B96` (airspin1, airspin2, airspin3; chips/airspin) | =airspin1, =airspin2, =airspin3 | X3 | differs 3 | code: `sub_80D8ADC`, `sub_80D8908`, `sub_80D8988`, `sub_80D8BD4` (+2 more); constants: literal ×9, branch ×7, offset ×3 |
| `sub_80E7546` (invisibl, whicapsl-invisible; chips/invisibl) | =invisibl, ~nrthwind | I1 X1 | navi 1 differs 1 | code: `sub_80E7518`, `sub_80E7546`; constants: branch ×1, value ×1; library: `sub_80EBD9C` |
| `sub_80E3B50` (barrier, barr100, barr200, bblwrap, lifeaur; chips/barrier, chips/bblwrap, chips/lifeaur) | =bblwrap, =barrier, =barr100, =barr200, =lifeaur | X5 | navi 5 | code: `sub_80E0C74`, `sub_80E0B8C`; constants: offset ×5; library: `sub_80EBD9C` |
| `sub_80E353E` (antinavi, antidmg, antiswrd, antirecv, elemtrap, bodygrd; chips/antidmg, chips/antinavi, chips/antirecv, chips/antiswrd, chips/elemtrap) | ~antifire, ~antiwatr, ~antielec, ~antiwood, =antinavi, =antidmg, =antiswrd, =antirecv | X4 I4 | navi 8 | code: `sub_80CE05E`, `sub_80CE034`, `sub_80CDFA4`; constants: offset ×4; library: `sub_80EBD9C` |
| `sub_8010488` (megabstr, whicapsl, uninstll, atk-10, navi-20, atk-30, …; chips/atk-10, chips/atk-30, chips/darkplus, chips/navi-20, chips/uninstll, chips/whicapsl) | ~attck-10, =navi-20, =darkplus, ~attck-30 | X4 | same 3 differs 1 | code: `sub_8010488`; constants: offset ×4; library: `sub_80EC39C` |
| `sub_80CFE08` (firehit1, firehit2, firehit3; chips/firehit) | =firehit1, =firehit2, =firehit3 | X3 | effects 3 | code: `sub_80CFD80`; library: `sub_80EC39C` |
| `sub_80ED2F8` (bblstar1, bblstar2, bblstar3; chips/bblstar) | ~cactbal1, ~cactbal2, ~cactbal3 | X3 | differs 3 | code: `sub_80ED314`; constants: value ×12, sound ×3, offset ×3; library: `sub_80ECA0C` |
| `sub_80E979C` (puncharm, needlarm, puzzlarm, boomrarm, darkinvs, bugrswrd, …; chips/darkinvs) | =darkinvs | X1 | differs 1 | code: `object_timefreezeBegin`, `sub_80E979C`; constants: offset ×1; library: `sub_80EBD9C` |
| `sub_80E67E6` (guardian; chips/guardian) | =guardian | X1 | navi 1 | code: `sub_80D4EFC`; constants: branch ×6, value ×2, literal ×1; library: `sub_80EBD9C` |
| `sub_80E4164` (anubis, poisphar; chips/anubis, chips/poisphar) | =anubis, =poisphar | X2 | navi 1 none 1 | code: `sub_80CF1DC`, `sub_80CF0F0`; constants: literal ×2, branch ×2; library: `sub_80EBD9C` |
| `sub_80E49A2` (bugfix; chips/bugfix) | =bugfix, ~lcrsshld, ~lstepswd, ~lcounter | X4 | navi 1 differs 3 | code: `sub_80C4848`, `sub_80C4958`, `sub_80E4954`, `sub_80C49A4` (+2 more); constants: value ×17, offset ×6, branch ×3; library: `sub_80E13DC`, `sub_80EBD9C` |
| `sub_800AF34` (fullcust; chips/fullcust) | =fullcust | Ir1 | same 1 | library: `sub_80EC39C` |
| `sub_80E4288` (meteors; chips/meteors) | =meteors | I1 | differs 1 | library: `sub_80CF3DC`, `sub_80EBD9C` |
| `sub_80E7FBA` (numbrbl; chips/numbrbl) | =numbrbl | X1 | differs 1 | code: `sub_800BDB2`; library: `sub_80EBD9C` |
| `sub_80E76D4` (otenko; chips/otenko) | =otenko | X1 | navi 1 | code: `sub_80DB1E0`; constants: literal ×2, value ×1; library: `sub_80EBD9C` |
| `sub_80DB4B4` (justcone; chips/justcone) | =justcone | X1 | effects 1 | code: `sub_80DB4CE`; constants: value ×1, literal ×1; library: `sub_80EC39C` |
| `sub_80EF7E2` (neovari; chips/neovari) | =neovari | X1 | effects 1 | code: `sub_80EF87C`; constants: value ×6, branch ×5, sound ×1 |
| `sub_80C0DD8` (roll, roll2, roll3; chips/roll) | =roll, ~roll-sp, ~roll-ds | X3 | effects 3 | code: `sub_80C0C9A`, `sub_80C0C48`, `sub_80C0A90`, `sub_80C0938` (+2 more); constants: offset ×15, value ×9, sprite ×6, branch ×6, other ×6, shift ×3; library: `sub_800BA8A`, `sub_80E1854`, `sub_80E18F8` … |
| `sub_80BA920` (judgeman, judgemn-ex, judgemn-sp; chips/judgeman) | ~gyroman, ~gyroman-sp, ~gyroman-ds | X3 | differs 3 | code: `sub_80BA7A0`, `sub_80BA84C`, `sub_80BA8F6`, `sub_80E71B8` (+4 more); constants: value ×39, offset ×9, branch ×9, sound ×6, sprite ×3; library: `sub_800BA8A`, `sub_80E1854`, `sub_80E18F8` … |
| `sub_80B84EC` (colonel, colonel-ex, colonel-sp, crossdiv; chips/colonel, chips/crossdiv) | ~serchman, ~serchmn-sp, ~serchmn-ds | X3 | differs 3 | code: `sub_80B83C0`, `sub_80B8230`, `sub_80B8406`, `sub_80B8338` (+1 more); constants: value ×33, offset ×18, branch ×12, sprite ×3; library: `sub_800BA8A`, `sub_80E1854`, `sub_80E18F8` … |
| `sub_80C2A4C` (protoman, protomn-ex, protomn-sp; chips/protoman) | =protoman, =protomn-sp, ~protomn-ds | C3 | effects 3 | constants: value ×3, sprite ×3; library: `sub_800BA8A`, `sub_80E1854`, `sub_80E18F8` … |
| `sub_80B9014` (blastman, blastmn-ex, blastmn-sp; chips/blastman) | ~numbrman, ~numbrmn-sp, ~numbrmn-ds, ~colonel, ~colonel-sp, ~colonel-ds, ~toadman, ~toadman-sp, ~toadman-ds, ~blizman, ~blizman-sp, ~blizman-ds, ~cloudman, ~cloudmn-sp, ~cloudmn-ds, ~cosmoman, ~cosmomn-sp, ~cosmomn-ds, ~crossdiv, ~wildbird, ~football, ~bignoise | X22 | differs 19 none 3 | code: `sub_80B8FEA`, `sub_80B8F8E`, `sub_80B8EC4`, `sub_80B8F30` (+4 more); constants: value ×138, offset ×32, sprite ×19, branch ×16; library: `sub_800BA8A`, `sub_80E1854`, `sub_80E18F8` … |
| `sub_80B999A` (tmhkman, tmhkman-ex, tmhkman-sp; chips/tmhkman) | ~tmhwkman, ~tmhwkmn-sp, ~tmhwkmn-ds | X3 | differs 3 | code: `sub_80B98EC`, `sub_80B97E4`, `sub_80B9848`, `sub_80B999A` (+1 more); constants: value ×30, branch ×9, sprite ×3; library: `sub_800BA8A`, `sub_80E1854`, `sub_80E18F8` … |
| `sub_80EA11C` (bighook, flmhook1, flmhook2, flmhook3; chips/bighook) | ~shademan, ~shademn-sp, ~shademn-ds, =bighook | X4 | differs 4 | code: `sub_80EA11C`, `sub_80DE818`, `sub_80EA0A0`, `sub_80EA05C` (+5 more); constants: offset ×9, value ×3, literal ×2; library: `sub_800BA8A`, `sub_80E1854`, `sub_80E18F8` … |
| `sub_80C3B30` (bass; chips/bass) | =bass | X1 | differs 1 | code: `sub_80C5E00`, `sub_80C5E84`, `sub_80C39BA`, `sub_80C3B30` (+7 more); constants: value ×5, offset ×4, branch ×3, sprite ×2, literal ×1; library: `sub_80C4550`, `sub_80C458C`, `sub_80C461C` … |
| `sub_80C2F96` (deltaray; chips/deltaray) | =deltaray | X1 | differs 1 | code: `sub_80C2D8C`, `sub_80C2CB4`, `sub_80C2C14`, `sub_80C2B8C`; constants: value ×4, branch ×2, literal ×1, sprite ×1; library: `sub_80E1854`, `sub_80E18F8`, `sub_80E192C` |
| `sub_80E8BC0` (metrknuk; chips/metrknuk) | =metrknuk | X1 | differs 1 | code: `sub_80E8B70`, `sub_80E8BE2`, `sub_80DBDA0`, `sub_80DBD10` (+1 more); constants: sprite ×1, value ×1; library: `sub_80EBD9C` |
| `sub_80C3E98` (bassanly; chips/bassanly) | =bassanly | X1 | differs 1 | code: `sub_80C3D32`, `sub_80C3E98`, `sub_80C3D0C`, `sub_80C3E46` (+2 more); constants: value ×4, sprite ×1, sound ×1, literal ×1; library: `sub_80C4550`, `sub_80C458C`, `sub_80C461C` … |
| `sub_80EC0E6` (batcan1, batcan2, batcan3, batcan4; chips/batcan) | =batcan1, =batcan2, =batcan3, =batcan4 | X4 | effects 4 | code: `sub_80EC11C`; constants: value ×4, offset ×4 |
| `sub_80EC44C` (synctrgr; chips/synctrgr) | ~shakpar1, ~shakpar2, ~shakpar3 | X3 | none 3 | code: `sub_80EC44C`; constants: sound ×3; library: `sub_80EC39C` |
| `sub_80ED810` (h-burst; chips/h-burst) | =h-burst | X1 | none 1 | code: `sub_80D8FE4`; constants: value ×3 |
| `sub_80BB7F6` (eraseman, erasemn-ex, erasemn-sp; chips/eraseman) | ~csmopris | X1 | none 1 | code: `sub_80BB710`, `sub_80BB772`, `sub_80BB62C`, `sub_80BB66A` (+4 more); constants: value ×10, offset ×4, branch ×3, sprite ×1; library: `sub_80E1854`, `sub_80E18F8`, `sub_80E192C` … |

### 14.5 LeadRaid's and ChaosLrd's summoned actors

Both summon through the navi chips' action (§6.4), the same handler and framework as EXE6's navi chips
(`sub_80EC350`, `sub_80E192C`; §14.3).

**LeadRaid → EXE6's TwinLdrs (0x15C).** LeadRaid (0x137, summon entry 0x15) spawns actor kind 0x20. EXE6's TwinLdrs
(summon entry 20, `sub_80BD9A2`) spawns kind 0x20 too, whose handler `sub_80BD388` is LeadRaid's kind 0x20 handler
(0x080BE1C8) verbatim. The routine map pairs LeadRaid's other routines with TwinLdrs's (`sub_80BD3AC`,
`sub_80BD478`, `sub_80BD644` and the rest). It is TwinLdrs's ancestor, reworked:

- **EXE5: two actors, one strike each.**
  - Kind 0x20 is ProtoMan: sprite 0x1030801, offset (10, 6) pixels ahead of the user.
  - It spawns its partner, kind 0x22, Colonel (0x080BED4C): sprite 0x1030807, offset (10, 6) pixels behind.
  - Each appears with sound 0x94. ProtoMan spawns an effect (effect kind 0x85, 0x080EA198) at frame 10.
  - At frame 55 each picks a target: the first panel ahead in its own row holding an opponent (0x080BE4F0); failing
    that, the first column with one in any row (EXE6's `sub_80BE434`'s counterpart).
  - Each moves to the panel before its target and strikes once (0x080BE364):
    - animation 5, the slash actor (kind 5), sound 0xB0;
    - at frame 12, a hit region `0x0705FF04` on the panel ahead, the hit effect (variant 0x16), a shake (3, 10);
    - the two hand over through +0x60 (`sub_80BE734`).
  - The lab: two hits of 200 wherever the opponent stands.
- **EXE6: one kind for both.** Kind 0x20 serves both leaders, its +4 choosing ProtoMan (0) or Colonel (1) from a
  table of sprites and offsets (0x080BD464). It appears with a white flash (`sub_80BD4DC`).
  - ProtoMan slashes up to twelve times (counter +6). He searches every row for a target and moves to each in turn
    (`sub_80BD5A8`). Each slash puts hit region `0x0405FF04` on the panel ahead, the hit effect (variant 0x27)
    and a shake (1, 10).
  - Colonel's finishing strike (`sub_80BD8BC`): two hit regions (`0x0405FF12`, `0x0405FF13`), effects 0x36 and
    0x37, sound 0xC7, a shake (3, 35).
  - A screen effect (`sub_80BDB04`: effects 0x4E and 0x4F, sounds 0x71 and 0x72).
- **EXE6 keeps EXE5's Colonel actor** (kind 0x22, `sub_80BE4D8`) and its spawner (`sub_80BE6D8`), called only from
  `sub_8114FB8`, outside the chips.

So a port writes LeadRaid's own module. TwinLdrs (content/exe6/chips/twinldrs) shares the navi-chip framework, the
target search's shape and the slash actor (kind 5, `sub_80B8E30`), not the behavior. The map's walk finds TwinLdrs
0.19 alike; there is no TwinLdrs lab to compare with.

**ChaosLrd: EXE5's own.** ChaosLrd (0x138, summon entry 0x1D) spawns actor kind 0x51. Its code is EXE5's: the
routine map pairs none of its states with EXE6 code, and no EXE6 summon is as like as 0.5. EXE6's kind 0x51 is
`sub_80B81EC`, shared with 0x52, not this.

The actor (0x080C2EB8, states 0x080C2F02, 0x080C3014, 0x080C2EDC) runs as follows:

- **Appearing.**
  1. Sound 0x94.
  2. It zeroes +0x24 of the eight objects listed at `[sl+0x18]+0xA0` (0x080C32A2).
  3. It leaves on a non-zero `GetRandomRelativePanelFiltered` (its arguments from
     `GetAllianceDependentPanelParamArgs`).
  4. It spawns five aura effects (`sub_80E5B62`'s counterpart: effect kind 0x47).
  5. It stands at the user's back column (1 or 6), row 2, eight pixels forward.
- **The strike.**
  1. After 142 frames, a burst of hit objects from a table of offsets (0x080C30C0, through `sub_80E5F78`'s
     counterpart: effect kind 0x4B), then sound 0x107.
  2. Sprite changes and a color fade.
  3. 20 frames into the last phase: animation 14, sound 0x141, and the strike (attack kind 0x82,
     `sub_80D5890`'s counterpart).
  4. 40 frames, then it leaves.
- **The lab:** one hit of 500 wherever the opponent stands.

The helpers are EXE6's: the aura's spawner and the burst's are at the same places as EXE6's leftover asteroid code
(`sub_80E5A64`, EXE5's Asteroid chips', §14.4). EXE6's BassAnly (kind 0x50, the next kind) shares nothing with it
beyond the framework. A port writes ChaosLrd's module from the EXE5 code.

### 14.6 For the port

- **identical, identical-run:** EXE6's module (`require("@exe6/chips/<module>")`) with EXE5's record (content/exe5),
  assets and sprites.
- **constants:** the same, the listed constants as parameters (sprite numbers come with EXE5's assets; effect numbers
  map through EXE5's kinds).
- **changed:** start from EXE6's module and apply the regions chip-actions.tsv lists. The labs say which matter: the
  22 changed chips whose timelines match differ in code the labs didn't see change anything.
- **The library's differences (§14.3)** are the engine's or a shared module's, once: the dimming handler's and
  action 0x1A's leaving the action on the use frame, the navi chips' numbers, the swords' souls, the shot's and the
  bombs' variants.
- **exe5-only and EXE6 code, no chip:** new modules, from the EXE5 code. For the latter EXE6 keeps the code (Voltz, the
  mode chips, FinalGun, the Asteroid chips, DarkMetr, Boxer, Blinder), so the EXE6 disassembly reads for EXE5 there.
- **built like:** the nearest EXE6 module as a template only (a navi chip's template, not its moves).

## 15. The EXE5 port

EXE5 as a second game beside EXE6 (rules-in-luau.md R2: the arena's game supplies the field, the hit tables and the
flow's banners and music; each side's game its navi, sounds and custom screen). In order: EXE5's battle data
(§15.1 to §15.4), the replay harness in the verification workspace (§15.5), the chips (§15.6).

### 15.1 EXE5's battle data (as built)

content/exe5/rules is written once by the verification workspace's `tools/exe5/gen_rules.py` (through
`gen_content.py write`, which leaves a written rules file alone; `check` notes where one differs since), from Team
ProtoMan's ROM; Team Colonel's tables are the same. Each file names its sources.

| File | What | EXE5's against EXE6's |
|---|---|---|
| ruleset.luau | the stock ruleset `exe5:exe5` | no systems yet (Soul Unison, Chaos Unison, EXE5's emotions, the Team Battle come as systems) |
| roles.luau | roles naming EXE5's assets, its collision types, the anchor region | the rest unfilled (below) |
| pools.luau | 16 actors, 32 attacks, 32 effects | EXE6's 32 each |
| elements.luau | weakness (0x08016900) | EXE6's; no family adds an element (EXE5 stores a chip's element as its record has it) |
| reactions.luau | push rows (0x0800CA24), ice slides (0x0800C988), the bubble's bob | five push rows read another way (§15.3); no bubble |
| math.luau | sine (0x08005CD0) | EXE6's |
| buster.luau | recovery by Rapid and open panels (0x0801CEA4); the empty hand's chip | EXE6's |
| banners.luau | holding banners: program-advance, hit-damage-judge, program-advance-empty | EXE6's three; the 49 records (0x0801B810) match EXE6's first 0x5D |
| custom-screen.luau | the slot grid and the neighbor fix-up's lists | EXE6's |
| status.luau | 64 statuses: paralysis, confusion, blindness, immobilization by level (0x0801CEC4); the HP bug's periods | no freeze, no bubble; the timers 2 bytes further in the collision record |
| collision.luau | 77 of EXE6's collision types by their rows (0x0801636C) | 80 rows to EXE6's 89; no 0x80 self bit; row 0x3D gives its sides' bits |
| panels.luau | the 11 panel types (unregistered data, §15.2) | metal and sea; no roads |

content/exe5/compat gains `assets.toml` (§15.4) and `rules.toml` (the statuses' bytes, 0x10 to 0x4F); exe5-compat
reads both (`Compat::assets`, `sprite_names`, `status`), and exe5-extract names the pack's assets from them. The
root requires EXE6's (`requires = ["exe6"]`): EXE5's definitions use EXE6's where EXE5's code is EXE6's
(`require("@exe6/...")`). nettai-content's lint test `exe5s_rules_are_its_games` loads EXE5's rules beside EXE6's (its
chips left out until they have uses) and checks they are EXE5's game's: 16 actors, EXE6's weakness.

**Roles not filled** (a battle stops at the first it needs, naming it): every action, kind, chip, status,
lock-on, effect, spark and hook role (the port's EXE5 definitions); the sprites but the charge glow (EXE5's status
visuals are EXE5's sprites, named when the port draws them); the freeze and bubble sounds and sprites (EXE5 has
neither status); the eruption's (EXE5's lava doesn't erupt); EXE6's own (the A charge's glow, Full Synchro's aura,
the Cross and Beast Out sounds, the custom screen's Cross window, re-deal and scrap).

### 15.2 The field

EXE5's 11 panel types (0x0800BCF0): missing, broken, normal, cracked, poison (EXE6's flags), 5 (flag word 0x10210,
EXE6's road panels', a plate's look: compat calls it metal), grass, ice, lava (EXE6's volcano flags), holy, sea
(0x30010). The trail sounds (0x080113A0) are EXE6's for the shared types, `immobilizer` for type 5, `sand-worm`'s
song for sea. Start grid and front edges, step and dash-step rules are EXE6's; EXE5 has no either-side step rule.
What the panels do (EXE6's `sub_800C380`, EXE5's 0x0800A998, and the routines named):

- a broken panel mends after 600 ticks in every battle (EXE6: 0x1E0 in battle mode 1), blinking its last 60;
  cracked panels break as EXE6's;
- lava and sea panels turn normal after 960 ticks (their own timers, +0x10 and +0x14 of EXE5's 0x24-byte panel
  record), blinking their last 60; nothing erupts (no volcano counter);
- lava (0x08016D80, 0x08016E18, EXE5's own): a grounded body, not of fire, not floating and not flagged 0x88000206,
  takes 50 (shifted by its weakness to fire) as a hit (flags 3 at +0x0F, +0x18, +0x19), unless flagged 9; the
  panel turns normal and a burst shows; a navi whose soul byte (NaviStats +0x2C) is 4 standing on lava with a chip
  whose +0x09 has bit 2 gains 10 on it, and the lava goes (0x08012602);
- sea: drains a fire body as poison drains any (0x08016C7E, EXE6's `sub_801A186`); at a move's end on sea, a body
  not floating, not aqua and without bit 0x20 of its AI record's flags (+0x48, `sub_801032C`) stops for 20 ticks
  (`sub_800EB18`'s timer, EXE5's at +0x24 of the collision record) with a splash (effect 99) (0x0801715E); a body
  on sea with that bit has its collision record's +0x2C set to 0xFFFF, else 0 (0x08017030);
- type 5: at a move's end, a slide (type 3) unless the body has slid within the cooldown (+0x38) or is a navi
  whose soul byte is 5 (0x08017216, EXE6's road start `sub_801A400`); EXE5's type-3 slide tries four steps in an
  order the move's direction picks (0x0800C8A8's branch, the tables at 0x0800C920 and 0x0800C9C0), not a road's
  fixed direction: after a move forward, down first (then forward, up, back); after a move up, forward first;
  after a move down, back first. Recorded (the chip lab's `stages/panel5-row/slide-*`, the middle row's inner
  four metal): forward from (1,2) onto (2,2) slides down to (2,3); down from (2,1) onto (2,2) slides back to
  (1,2); up from (2,3) onto (2,2) slides forward to (3,2), and from there (metal again) down to (3,3). A slide
  step up or down covers the 24 pixels in 3 ticks (8 a tick);
- conversions (0x08016D14, EXE6's `sub_3007708` moved out of IWRAM): fire on grass, aqua on lava, element 4 on
  type 5 turn the panel normal (EXE6's roads take element 4 too);
- battle effect 0x1000 (single player only) hands panel runs over (0x0800AE92, EXE5's own).

### 15.3 What the engine has no slot for (proposals)

The smallest engine additions EXE5's data and rules need, for the rules agent (none made here):

1. **Panel types.** `PanelType` gains EXE5's three: `Metal` (type 5), `Lava` (type 8: not EXE6's volcano, whose
   flags and sound it shares but not its behavior) and `Sea` (type 10); the `panels` section names the types its
   game has, not exactly all of the engine's (EXE6's names its 13, EXE5's its 11). Their behavior as section data
   where it is numbers: per type `expires` (ticks to normal: EXE5's lava and sea 960; EXE6's roads their road
   timer) and the broken panel's mend time per game (EXE5 600 always); the rest as code the types select (lava's
   burn, sea's drain and stop, type 5's slide), keyed by the panel type, not the game. exe5-compat then maps 5, 8
   and 10 to them (today: none, volcano, none).
2. **Push.** EXE5's push reads the first of bits 2 to 5 of the hit modifier's first byte (+0x18), else of its
   second (+0x19) with the direction reversed (0x0800C9D8), five rows; EXE6's reads bits 2 to 5 and adds 5 for 0x80
   (`sub_800E548`). A choice in the `reactions` section (`push_reading = "by_hitter_flip"`), the engine reading
   both ways.
3. **Slide type 3** reads the panel's road direction in EXE6 and the move direction with fallbacks in EXE5 (§15.2):
   the same choice, or type 5's code in item 1.
4. **Statuses EXE5 lacks.** The engine's aqua-on-ice freeze and the encased-obstacle hook ask the arena's roles
   `statuses.ice_freeze` and `hooks.encased`: EXE5's arena must answer "none" without a panic (optional roles, or
   the reactions section saying the game has no freeze).
5. **Chip families** recovery and invisible (§13): `ChipFamily` gains them; EXE5's chips name them, and the
   systems that count families (AntiRecv's) read them.
6. **NaviStats +0x0E** (EXE5's mood byte, §13): a recovery adds its +0x0A to it (0x08012802, cap 254, not from 0 or
   0xFF); a navi chip's leaving subtracts its +0x2E (0x08012820, floor 1). A field of EXE5's navi record and a
   hook on recovery and on a navi chip's leaving (or the emotion module's, when EXE5's comes).
7. **The flow** (§15.4's table): a `flow` section for its numbers (the result waits), and the arena's game
   choosing the flow's code where EXE5's differs (the state machine's structure, below).
8. **Mixes with their own sections** (rules-in-luau.md R2): Soul Unison's custom-screen layout (the shared custom
   screen's soul row) is a mix's; needed when the Team Battle's custom screen is ported.
9. **Collision words:** EXE6's chips required from EXE5 register EXE6's collision types (with the 0x80 self bit,
   rows past EXE5's 0x50): fine while every target word lacks 0x80, which the port checks per row it uses (EXE6's
   `pull` and `probe` rows test it; EXE5's chips pass EXE5's types where EXE6's modules take them).
10. **Leaving the action on the use frame** (§14.3): EXE5's dimming handler (action 0x15) and action 0x1A's object
   handler leave the action on the frame they run; EXE6's on the next update after the dimming, and 8 frames
   later for subtype 20. A choice per game in the engine's chip use (a rules section's flag), read by every
   dimming and instant chip.
11. **AntiNavi's sparkle** (`sub_800ABC6`, EXE5's 0x080093A2): EXE5's sits on the panel's center 16 pixels up, EXE6's
   16 pixels down the field and 32 up (built: the chip-use rules' `anti_navi_sparkle`, by the trap's game, which
   AntiRecv's mark reads too).
12. **The hit test** (built: the reactions section's `hit_test`, a table of its differences): EXE5's (0x0801691C, ROM code where EXE6's
   `sub_3007218` is IWRAM's) counts a bubbled body (flag1 0x80000000) as submerged, elec reaching either; has no
   FloatShoe test (EXE6's: flag1 0x20 meets only the 0x80 self bit), nor its raw channel (0x08017494, EXE6's
   `sub_3007692`); breaks a guard with 0x1002 always (EXE6's 0x0002 but with 0x4000) and turns aside what lacks
   0x0C004000 (EXE6's 0x0C005000); and counts elec on its sea once more, as fire on grass (0x08016AF6).

### 15.4 The flow and the assets' names

**The flow.** Of the EXE6 flow routines the engine cites (battle.rs, hud.rs, dimming.rs), 73 are EXE5's the same
code, 6 the same but for constants, 34 similar, 1 differs and 8 absent. What differs, by the EXE6 routine:

- `sub_800825A` (the result): a special battle's result wait 65 ticks (EXE6 94); the normal win's 102 as EXE6's;
  `sub_80081A4` plays EXE5's own winner songs at the same numbers (0x19 special, 0x1F).
- `sub_80081A4` (the win's banner; EXE5's 0x080074D2). EXE6 shows the winner's navi's banner in every link battle,
  whatever the result (`sub_800A8D4`'s table by NaviStats +0x29); outside one banner 4, "ENEMY DELETED", or 0x14,
  "YOU WIN", for the round's result 7 (the judge's ruling: time up with navis left on both sides). EXE5 shows the
  navi's (0x080090C0's table: MegaMan's 0x38, the team navis' 0x94 to 0xC0) only in a link battle that is an
  operation battle (0x080090E8: battle flag 0x40); a Team Battle's and a NetBattle's win is banner 4, or 0x14 on the
  judge's ruling. The loser's is the same in both (`sub_800825A`, EXE5's 0x0800758A: the navi's, 0x18 "YOU LOSE" on
  the judge's ruling). The flow rule `navi_win_banner` (`link_battle`, `operation_battle`), each game's own, and
  the banner roles `win`, `win_judged` and `lose_judged` (2026-10-05: the engine showed the navi's for every win,
  "MEGAMAN WIN!" on an EXE5 winner's console, and named the judged loss's by a raw number where a handle goes, so
  EXE6's showed "MEGAMAN DELETED"). Both routines' other branches: a win with a boss among side 1's actors (EXE6's
  Cybeasts, NameIDs 0x173 to 0x17E; EXE5's 0x173 to 0x176, 0x08008F6E) has neither music nor banner and holds 102
  ticks; a win in battle modes 4 and 5 (and 8 in EXE6) plays the special battle's song and holds its wait. As
  recorded (library-exe5/flow, each on both consoles): ko-win and ko-win-netbattle 4, judge-win 0x14, judge-lose
  0x18, judge-draw 0x1C, double-ko 8 on side 0's console and 4 on side 1's (the round goes to side 1); an
  operation battle's win, once recorded (below), 0x38. The replays compare the banner's number with the traced console's record (its +1; a telop's
  holds 0) on every frame, in both games.
- The operation battle (battle flag 0x40), as seen and not read: the mode isn't supported, by the user's decision
  of 2026-10-05 (the engine doesn't run it: a replay stops on the round's first frame, frame 162, the gauge 0x1500
  where the engine's is 0). Recorded once (2026-10-04) with a chiplab base whose `operation_battle` raises
  NaviStats +0x2A at 0x0802D590, which no Team Battle sets; the base and its two scenarios
  (flow/operation-win, operation-win-side1) are out of the library since, the recordings kept in the verification
  workspace's data/staged/exe5-operation-battle-2026-10-04.
  - No custom screen opens. The round's start shows "BATTLE 1 START!" (banner 0x30, frames 226 to 284) and the
    turn's start banner 0, "BATTLE START" (frames 386 to 444), where a Team Battle's shows 0x0C; the fight runs from
    frame 446. (The driver's `waitcustom` returns at once and its `fight` never does; five A presses 20 frames apart
    before the fight do nothing seen.)
  - Each console shows its own three chips in a row along the bottom with a cursor over the middle one (side 0,
    Team ProtoMan's save: AirShot, AirSpin1, WindRack; side 1, Team Colonel's: YoYo, BugBomb, Sword: neither the
    folder's first chips nor the save's auto battle data), its navi's HP top left, and the gauge along the top,
    full and red from the start.
  - Both navis stand idle (action 6) until their player presses A: the navi then goes through action 0x47 and its
    chip's own. Side 0's first A deleted side 1's navi at 20 HP (frames 794 to 823); side 1's navi's first chip
    took 10 HP of side 0's navi.
  - The win shows the winner's navi's banner (0x38, "MEGAMAN WIN!", from frame 880 with side 0 the winner:
    0x080090C0's table), the result's wait 102 ticks as a Team Battle's.
- `sub_80080D2` (fighting): no Cross-special check (EXE6 +0x3A, `sub_800AAD6` absent) and no own-gauge
  decrement (0x2900) on a custom request; EXE5 calls 0x08025ED0 there.
- `sub_8008452`, `sub_8008492`: after the reversions EXE5 opens the custom screen (result 6) directly: no
  transformation sequencer re-run (EXE6's state 0x24 is absent), and no mode-5 test.
- `sub_800840C`, `sub_8009158`, `sub_8009338`, `sub_80102AC`, `sub_8013FD0`, `sub_8007EB8`: EXE5 adds tests of
  battle flag 0x40 (`sub_800A8F8`: the operation battle, which EXE5's setter, 0x0802D590, turns on when the battle
  mode isn't 1 and NaviStats +0x2A is set) around single-player steps; `sub_8009338` calls the custom screen's
  0x08022C5C/0x08022D70 there (EXE6's `sub_8026840` is absent).
- `sub_8007CA0` (round end): EXE5 writes the light/dark value back (NaviStats +0x44, under battle effect 0x800)
  after three routines of its own, what a battle leaves in the save (called at 0x08007062 and 0x0800710C, as the
  battle is left): 0x0801289C works out the light/dark value (down where the side took its last stand, its counter
  1, set at 0x080EFEDC, or used a dark chip, counter 2, set by the dark chip rule at 0x0801007C; else it may go up,
  by the stats' +0x20 and +0x21 and the battle's settings, not read through), 0x0801288E stores it, and 0x0801299E
  is the permanent cost of the two: for MegaMan, where either counter
  is set, the base maximum HP (+0x3E) goes down by one, once a battle (a tally at the toolkit's +0x40 block, +0x16,
  stops it at 499), and the maximum is made again from it (0x0803C208: the base, the NaviCust's HP programs and the
  patch cards' HP, 0x0813792C). The first and the third run only under battle effect 0x800 and not in a link battle
  (effect 8: the recorded link battles' effects are 0x0E8C), so a link battle changes neither, and neither is in
  the engine: both write the save after the battle, which nothing here keeps. Constants not read yet at 0x080070CE
  (5, EXE6 1) and 0x080070EA (16 and 217, EXE6 23 and 51).
- `sub_8017AB4` (a side's dimming): 65 where EXE6 tests 27, 120 for 128 (0x08014574, 0x080145CA), not read yet.
- `sub_80107D4` (the navi's tick): EXE5 counts down four timers at +0x3C and an invulnerability timer at +0x16,
  and runs the lava-chip boost (0x08012602).
- `sub_800A1D0`, `sub_800A244`, `sub_801C840`, `sub_802E112`, `sub_802F068`: EXE6's Beast/Cross and battle-mode-6
  tests EXE5 hasn't; `sub_802E4E4` (SELECT's special) differs in its flags (EXE5 0x200000, EXE6 0x20000000).
- `sub_800BA8A`, `sub_800BDD0`, `sub_800BE2C` (the telop and AntiNavi): AntiNavi is chip 144 (EXE6 186): content's
  keys cover it.
- `sub_800B3D8`, `sub_802D7A0`, `sub_802D9B0`: record sizes (96 for 100, 129 for 161), not timings.

**The assets' names** (content/exe5/compat/assets.toml, `gen_content.py`'s `asset_names`): EXE6's name for what is
EXE6's. A song whose data is EXE6's song of the same number (227 of EXE5's, sound effects and jingles); EXE5's own
music by the place EXE5's code starts it where EXE6's same code starts a named one (virus-battle 0x15, loser 0x1A,
game-over 0x1B, transmission 0x0F); a sound EXE5's identical code plays where EXE6's plays a named one; the songs
read at their places (own-hit 0x6B and hit 0x6D in `applyDamageToPlayer_801ba12`'s counterpart, winner-0 0x19
and winner-1 0x1F in `sub_80081A4`'s); a battle sprite whose archive is an EXE6 sprite's, or that identical code
loads where EXE6's loads a named one (48); the banners whose records are EXE6's (24 named). 149 sounds, 48
sprites, 24 banners in all; the rest keep their placeholders, which content may not use.

### 15.5 The replay harness (as built)

exe5-compat's `trace` (feature `trace`) replays an EXE5 recording as exe6-compat's does an EXE6 one: `rounds` splits a
recording at its setup lines; `Round::needs` lists what its setup needs that the content doesn't define (every
chip of its folders and hands by compat key, each side's navi by EXE5's number, a soul, the stage by its settings
bytes, the weapons, programs and first barrier its NaviStats name); `Round::round_setup` builds the engine's
`RoundSetup` once nothing is missing (§15.7: the stage and background by the settings record, both NaviStats,
the folders, the RNGs, the set's score, both players on EXE5's stock rules) and `Round::start` the battle;
`run_round` ticks each battle frame with the recorded inputs and compares (`compare`: the state machine, ticks,
the simulation RNG, pause, the gauge, the panels by EXE5's numbers, the objects by pool, EXE5's kind number
(compat kinds.toml), panel, side, HP and position, a position the kind leaves as register garbage skipped as
exe6-compat skips it), stopping at the setup, a panic or the first difference.

The verification workspace's `trace-tests` runs it over the EXE5 lab (`--test exe5_replay`, ignored: a report) on
EXE5's root (which requires EXE6's) and both games' packs (R3a), EXE5's chips without a use left out
(`trace_tests::exe5_content`), and writes replay-summary.md beside the recordings: each recording's stage (read,
decoded, setup, replay, matched), its frames matched, what stopped it, and what the recordings need most. First run
(2026-10-02, the 1,376 recordings, 946,555 battle frames): every one stops at its setup, needing EXE5's MegaMan
(`exe5:megaman`), a stage (the lab's is one settings record but for 64 recordings) and its chips (Boomer and Cannon
are in every recording's folder or hand, the lab's filler). With the chips of §15.6, Boomer remains in every
recording and Recov10 in 108.

With EXE5's MegaMan, its stages and Boomer (§15.7): 115 recordings replay, 1,261 stop at their setup; 25,170 of the
946,555 battle frames match. Every replay matches its first 219 battle frames (the intro, the first turn, the
custom screen until both results are in) and differs at frame 361, the tick both are in: EXE5's custom screen mode
(EXE6's `sub_8009338`, EXE5's 0x08007F50) runs the Team Battle's own routine (0x08025EF2, in place of EXE6's
`sub_8026A28`), which ends the screen on that tick (mode 0xC), where EXE6's ends it on the next (the engine's
`CustomScreens::committed`); EXE5's also doesn't set the navis' AIData +0x0F as EXE6's does. A choice per game in the
engine's custom screen (§15.3 item 13).
What the setups need most: Recov10 (108: EXE5's recovery family, §15.3 item 5), WideSht1 (18), BlkBomb and Thunder
(15 each), HolyDrem (12), Sword (11), then the chips each scenario tests.

With the chips of §15.6's third batch, EXE5's metal, lava and sea stages and the rules work's P1a (its custom screen
end, item 13; the families; the panel types), on 2026-10-02 (1,380 recordings: the lab's and four metal slides):
252 replay, 1,128 stop at their setup; 24 match every frame and 79,701 of the 948,097 battle frames match. 191 of
the 228 that differ stop at frame 426, the fight's first tick, on the panels: Team Colonel's MegaMan is dark (his
save's light/dark value 0) and stands on the default stage's holy middle row, which EXE5 turns Normal under him
(§6.1, item 14). The rest: a spark a tick short (18, item 16), the camera shake's draws (7, item 15), a custom
screen opened a tick late (12, item 18). With those four built in a scratch copy of the engine (not committed: the
rules work's), 228 recordings match every frame and 115,999 frames match; the 24 that still differ are HolyDrem's
(item 18), DrkRecov's (its dark chip cost, unread: §6), the souls' (not ported) and the metal slides' speed
(item 17).

### 15.6 The chips (in progress)

From the action map (§14.6), first the chips whose code is EXE6's (identical, identical-run), then those that
differ in constants only. Each is EXE5's chip file (its record, generated) with a use from EXE6's modules
(`require("@exe6/...")`), EXE5's collision types where EXE6's modules take them; content/exe5/objects/projectile
holds EXE5's projectile variants (EXE6's rows with EXE5's collision). Until R3, the modules draw and time from EXE6's
pack (the assets are EXE6's where the names are EXE6's, §15.4).

**Ported** (26): Cannon, HiCannon, M-Cannon (EXE6's cannon, EXE5's shot variant: the variant rows are the same);
MiniBomb, EnergBom, MegEnBom (EXE6's throw and bomb, the energy burst EXE6's object); PanlGrab, AreaGrab; GrabBnsh
and GrabRvng (EXE6's controller; GrabRvng's hand effect is EXE5's 0x29, the same as EXE6's 0x3B); SloGauge,
FstGauge; PnlRetrn, HolyPanl, Snctuary; AntiNavi, AntiSwrd, AntiRecv (EXE6's traps; the roles of their counters
and mark are EXE6's, §15.1); FullCust (its fill is EXE5's: with each side's own gauge, the side's goes full where
EXE6 adds a third). The roles `actions.anti_damage_counter`, `anti_sword_counter`, `kinds.anti_recovery` and
`effects.trap_mark` (EXE5's effect 43 is EXE6's 0x46) are filled for the traps. Then, with EXE6's modules taking
EXE5's numbers (the user's decision, 2026-10-02: EXE6's chip modules may change where EXE5 can't reuse them as they
are, the smallest change, EXE6 staying the same): Silence, Discord and Timpani (EXE6's instrument with EXE5's 100 HP,
play and pause ticks, sprite and actor records; the effects EXE6's with EXE5's statuses and collision types);
AirShot (EXE6's AirShot with EXE5's shooter look and shot, row 4 without the wind element); ProtoMan (EXE6's navi
with EXE5's sprite, sword animation, slash effect 0x33 and spawner); Boomer (EXE6's boomerang with EXE5's variant:
half EXE6's speeds, no grass, EXE5's sprite).

**Third batch** (2026-10-02): WideSht1 to WideSht3 (EXE6's wide shot with EXE5's shooter, the wave at the floor and
EXE5's wave sprite; WideSht2 and 3 fire wave kinds 4 and 5, their records' parameter), BlkBomb (EXE6's bomb with
EXE5's leaving, 0x080CE594, its intake EXE6's `sub_801ADFA`'s where EXE6's is `sub_801AD12`, its spark and
collision), Thunder (EXE6's ball with EXE5's look: sprites 10-11 and 0c-53, animation 0, collision row 0x0A, hit
modifier 0, a 12-panel ball's row 0x2B, one sound), Sword, WideSwrd and LongSwrd (EXE6's slash with EXE5's parts,
lib/swords: the blade, the effects 0x16 to 0x18 and EXE5's slash type), Recov10 to Recov300 and DrkRecov (EXE6's
heal, the mood the record's parameter, only when no AntiRecv turned the heal), HolyDrem (its own: EXE6's BurnSqr
controller's code, a stand-in that fires a shot along the row and one more for each holy panel it turns Normal,
and the shot, attack object 0x83), Invisibl, AntiDmg and Mine (EXE6's, now the engine has the invisible family).
Fixed: Boomer's variant is the record's row 5 (EXE6's Boomer's speeds), not row 0. EXE5's own hit sparks
(lib/sparks: EXE6's rows with EXE5's sprites, whose animations are a frame longer) for its roles, shots and chips;
the status visuals' sprites, a lava burn's spark and a sea splash (effect 0x63) in its roles; AntiSwrd's counter
throws EXE5's sonic boom (at the floor, palette 9 going through, else by the whole element byte, 0x080D0E64).
Kinds.toml gives the EXE5 numbers of every kind the ported chips spawn, EXE6's included. exe5-extract writes EXE5's
banners (49, the same layout as EXE6's: 0x0801B810), their digits and palette, and "Cstmzing..." (the audit's 24
problems). Waiting in this batch: DrkRecov's dark chip cost (HP bug: EXE5's own code, unread, §6), HolyDrem's light
MegaMan (§6.1), the swing's call EXE6 stubs out (0x080E9FD2, battle flag 0x40: never in a netbattle).

**Shared code (common-shared, 2026-10-03; the user's direction: shared EXE5/EXE6 behavior in content/exelib).** The
EXE6 modules the lists below opened for EXE5 now live in content/exelib as makers that take a game's look (none holds a
game's ids or assets; rules-in-luau.md §7.2), EXE6's modules at their old paths their EXE6 wrappers, EXE5's its own
definitions made with them: the regions, panels, slot, element, trajectory (over a game's sine table), dimming and
its stand-in, the recovery heal, the attachment, the buster's parts, the projectile and its firing, the cannons,
AirShot, the bombs' throw and bomb, the energy burst, BlkBomb, the panel bursts, the rising bubble, the panel
changer and the panel chips', trap chips', gauge chips', grab chips', GrabBnsh's, Invisibl's, Mine's, AntiRecv's,
CircGun's, Meteors', TimeBom's and the instruments' controllers with their objects (the grab shot, the hand, the
mine, the gun and shot, the falling meteor and its marker, the countdown bomb, the instrument and its effects),
AntiDmg's counter and shuriken, the swords' parts and slash, the sonic boom, AntiSwrd's counter, the wide shot and
wave, the bullet and the vulcans, the tornado and its blow, Thunder's ball and shot, FireHit's fist, ElemTrap's trap
and counterattack, Lance's lance, DrilArm's drill, ProtoMan, the boomerang, the plus chips' sparkle, the Spreaders'
action, the shower's aim, and MegaMan's buster, blank and charged shots. EXE5 makes its own kinds of each (compat's
kinds.toml names them `exe5:...`); an EXE5 look names EXE5's assets (the same sheets and sounds where EXE5's are
EXE6's, by EXE6's names in compat/assets.toml). Since exe5-no-exe6 (2026-10-03) content/exe5 requires nothing of EXE6's
and names none of EXE6's assets: a match loads its own game's pack alone. What it took from EXE6 it now has of its
own: EXE5's effects (lib/effects: EXE5's effect table, 0x080DFC94, whose rows to 0x22 are EXE6's in EXE5's sheets; the
trap mark EXE5's row 0x2B), EXE5's own collision rows where EXE6's kept the 0x80 self bit (thrown, curse,
thrown-slash, attack, slash: the grab shot, the energy burst, GrabBnsh's hand, AntiDmg's shuriken, ProtoMan's slash
and the projectile's burst; no EXE5 target word tests the bit), EXE5's plain shot as the fallback of the forced
charged shot (EXE5's projectile table's row 0), EXE5's navi arm (the attachment table's row 0x2B) as the shared
buster's enemy-navi arm, and the shared code where it is the same (@exelib/instant/side_special, EXE6's
`sub_802E1BE`; @exelib/guardian/strike, the strike back, whose telop names EXE5's own Punisher, chip 0x175, as
VDoll's curse names its Curse, 0x174: both EXE5's SonicBom action of hit modifier 0). The sheets EXE5's pack had no
names for got them by their places (gen_content.py's SPRITES_BY_PLACE): 0c-06 `muzzle-flash`, 0c-07 `aura`, 0c-20
`bubble`, 0c-3d `barrier`, 0c-49 `otenko`, 14-0a `burst` and 14-14 `lightning`.

The lists below say how each was opened (each the smallest change that lets EXE5 reuse it; EXE6's behavior the same,
its full set run on the batch):

- lib/instruments/instrument.luau (and types.d.luau's `Instrument`): optional `hp` and `sprite`. EXE5's
  instruments have 100 HP and their own sprite, where the library had 60 and EXE6's sprite as constants.
- lib/instruments/effects.luau, new: Discord's, Timpani's and Silence's effects, taking their status and
  collision types. They were local functions of EXE6's chip files, which EXE5's chips can't reach, and EXE5's give
  EXE5's statuses.
- lib/airshot.luau, new (and `AirShotSpec`): AirShot's action, taking the shooter and the shot. It was the EXE6
  chip file's own; EXE5's AirShot fires another variant.
- chips/protoman/navi.luau (and `ProtoManLook`): his look (`protoman.look`, `summon_with`), EXE6's as
  `protoman.exe6`. EXE5's ProtoMan has its own sprite, sword animation, slash effect and spawner address, which
  were constants.
- chips/boomer/boomerang.luau: a variant's optional `sprite`. EXE5's boomerang is its own sprite.
- navis/megaman/weapons/buster/init.luau: `buster.pick` exported (was local). EXE5's buster picks blank and
  charged shots by EXE6's code but sets up its own shot.

Third batch:

- chips/widesht/action.luau and wave.luau (and `WaveVariant`): `widesht.action` exported (was local) with an
  optional shooter and wave height; a wave variant's optional `sprite`. EXE5's shooter is another attachment row,
  its wave at the floor and its own sprite.
- chips/blkbomb/bomb.luau: `black_bomb.make_kind(id, look)` (the destroyed action, the spark, its and its burst's
  collision types, the push mode), `finish` exported, the thrower taking a kind. EXE5's bomb leaves the field
  otherwise, takes hits through another routine and has EXE5's spark.
- chips/thunder/ball.luau and shoot.luau (and `ThunderBallLook`): `ball.make_kind(id, look)` (sprites,
  animations, collision types, hit modifier, spark, sounds), the spawn and the action taking a kind. EXE5's ball
  differs in each of those.
- chips/boomer/boomerang.luau: a variant's optional collision type and spark. EXE5's are its own.
- lib/swords/sonic_boom.luau: `sonic_boom.make_kind(id, look)` (sprite, height, palettes, collision type), the
  throw taking a kind; chips/antiswrd/counter.luau: `action_with` taking the booms' kind. EXE5's AntiSwrd counter
  throws EXE5's boom.
- chips/recov/heal.luau: the mood only when the heal's result says no AntiRecv turned it (EXE5's 0x080EC484 tests
  it; EXE6 passes no mood).

**Waiting:**

- *On EXE5's dark chip costs* (§6): DrkRecov's use leaves its user losing 1 HP every 10 ticks (EXE6's
  `sub_800B79A`, the dark chips' HP bug, is absent in EXE5; EXE5's own is unread).
- *On EXE5's damage formulas* (§13): ProtoMn SP and DS (EXE5's records name formula rows 2 and 24, which EXE5's
  table, not EXE6's SP times, gives); their use is ProtoMan's once it is.
- *On the engine* (§15.3 items 10 and 11): every ported dimming chip and FullCust leave the action as EXE6 does
  until the engine reads EXE5's choice; AntiNavi's sparkle sits where EXE6's does.
- *Without an EXE6 chip* (§14.4, `exe6 code, no chip`): Blinder, the mode chips, FinalGun: new modules from the
  shared code.

**Fifth batch** (2026-10-02, exe5-port-5): BusterUp, Attck+10 and +30 (lib/plus), FireHit1 to 3, Vulcan1 to 3,
Tornado, AntiFire/Aqua/Elec/Wood, CrakOut and its family, the dark chips DarkThnd, DrkSword, DarkTorn, DarkWide,
DarkCirc (CircGun's kinds of EXE5's look, its variant period 2, 7 shots, look 1), DrkLance (with Lance: EXE6's
lance of EXE5's look, hit modifier 0x10, Param1 1's row 0x25 and bug 0x16), DarkMetr (with Meteors: the falling
meteor's rows, 2's hit modifier 0, 3 cracking, 4 DarkMetr's breaking with bug 0x19; EXE5's shower with its dark
parameter), DarkDril (with DrilArm1 to 3: EXE6's drills of EXE5's look, no wait outside flag 0x40, DarkDril's 60
ticks and bug 0xFA), DrkSonic (with Fanfare and EXE5's own instruments: row 4, its effect paralyzing each enemy
where it stands), DarkPlus (its damage the next damaging chip's Atk+ bonus, the dark tint AIData+0x3C); the dark
chips' rule and costs (rules/light_dark: `battle.set_side_stat`, the mood, `battle.no_dark_chips`); TimeBom1 to 3
and TimeBom+'s use (EXE6's TimeBom with EXE5's placement: a random row's frontmost enemy panel, 0x080E3420; the
bombs' identities of AI index 0x21, EXE5's field objects' index: actor records 0xD8 to 0xE1 all have it, and nothing
of EXE5's reads it but the record).

EXE6's modules changed for EXE5 in the fifth batch, each a kind of its look (EXE6 the same): chips/firehit/fist,
objects/bullet, chips/vulcan/action, chips/tornado, chips/elemtrap (trap and strike), chips/widesht/wave (its
trail), chips/timebom (countdown.make_kind, controller.make), chips/circgun (shot, gun, controller), chips/lance
(make_kind, instant_of), chips/meteors (falling_meteor.make_kind with its rows by Param1; the controller moved to
controller.luau), lib/instant/meteor_shower (`pick`, `aim` exported; EXE6's three-drop row 2), chips/drilarm/drill
(make_kind returning its spawner), lib/instruments (instrument.make_kind, instruments.make, `effect_period`),
lib/swords (a slash's `blade_anim`; parts.hold's anim).

**Waiting:** LarkMan, GridMan and their SP and DS (EXE5's own navis, §14.4: the action map's `exe5-only`), DarkInvs
(EXE5's own, 0x080E2338), the mode chips, Program Advances' recipes (TimeBom+'s among them), the operation battle's
effect 0x83 (FireHit's warning, the swords' swing, the meteors, DrilArm's start: never in a netbattle).

**Chips 0x000 to 0x06F** (2026-10-03, exe5-chips-a): WideBlde, LongBlde and CustSwrd (the shared slash with EXE5's
parts and its effects 0x19, 0x1A and 0x28; CustSwrd's damage the custom gauge's, formula 45); AirHoc (the shared
puck and flick, content/exelib/airhocky, of EXE5's look); Static (EXE5's tornado blow, its tornadoes paralyzing by
the bug level: none, 90, 120, 150 ticks); Spreader (the shared Spreaders' action with EXE5's gun and bullet row 3:
EXE5's flash 0x21 and sound); GunDelS1 to 3 (the shared sun beam, content/exelib/gundels; EXE5's GunDelSol,
chips/gundels/gundels); BugBomb (the shared BugBomb, content/exelib/bugbomb, with EXE5's bugs: either HP drain plus 2 or
the emotion swings); Katana1 to 3 (chips/katana/katana); MrkCan1 to 3 (chips/mrkcan/mrkcan: the sweeping sight, effect
0x44, and the cannon at its panel); Pulsar1 to 3 and SpShake1 to 3 (lib/armshot, and lib/arm: EXE5's buster arm,
0x080EBABE; the pulse, attack 0x6A, and the shake wave, 0x68); Skully1 to 3 (chips/skully/skully, attack 0x88);
Astroid1 to 3 (chips/meteors: instant effect 17, 6, 8 and 10 meteors); Snake (the shared snake and holes' scan,
content/exelib/snake: EXE5's nest sends three snakes at a time with a flag each, its snakes wait 48 ticks and
strike as wood); YoYo (the shared throw, content/exelib/yoyo; EXE5's yoyo, chips/yoyo/yoyo, attack 0x52, GreatYo's
modes too); Slasher (EXE5's own action 0x29: while A is held, the wide slash at an enemy navi's column); CircGun
(the shared CircGun of DarkCirc's look, 4 shots); TankCan1 to 3 (the shared action and shell,
content/exelib/tankcan; EXE5's shell, chips/tankcan/tankcan); WindRack (EXE5's own action: EXE6's swing without the
gusts). Then LifeSync (EXE5's controller, effect object 0x5C: no sync in a boss-ranked battle either, the HP
capped by each one's max HP in turn, flag 0x40's hit of 50); MoonBld1 to 3 (EXE5's own action 0x53, chips/moonbld/moonbld:
EXE6's spin with the Katanas' step, lib/stepsword, marked moving while it runs); CrakBom, ParaBom and ResetBom (their
bomb, chips/crakbom/crakbom, attack object 0x24: EXE6's code no EXE6 chip throws; it hits the column where it lands);
Quake1 to 3 (EXE5's own Quake bomb, chips/quake/quake, attack object 0x51: a weight that drops on the panel three ahead
and hits its level's region); IceSeed, SeaSeed, GrasSeed and LavaSeed (the shared seed with EXE5's look,
lib/bombs/seed, attack object 0x4A); CannBall (chips/cannball/cannball, attack object 0x35: it breaks the panel it lands on);
Geyser (chips/geyser/geyser, attack objects 0x42 and 0x43: EXE6's BlkBomb code reworked, a splash of 10 on solid ground, in
a hole a geyser whose water hits the eight panels around in a shuffled order); MetaGel (its controller, effect
object 0x21, and gel, attack object 0x45: EXE6's code no EXE6 chip drops; a gel on each row's panel just ahead of the
user's area, taking it); Magnum (EXE5's own: its controller, effect object 0x30, and gunner, actor object 2, the
shared stand-in's drawing, a sight stepping across the enemy's columns until A or 180 ticks, then the column shot
and broken; its unused modes 1 and 2 too); VarSwrd (EXE5's action 0x2A: EXE6's sequences and EXE5's picks, ProtoSoul
and ShadowSoul waiting while A is up) with its hidden picks as chips, FtrSword (0x172: the shared slash over three
panels) and SonicBom (0x173: EXE5's action 0x2B, the sonic boom swing with the Katanas' step), whose records
gen_content.py writes (its USED_CHIPS). The chip lab has VarSwrd's five commands and Magnum's A press besides the
generated scenarios (chiplab's library-exe5, by hand). Each matches every frame of its lab recordings. The batch's
shared modules
(content/exelib, as above): airhocky/puck and flick, gundels/beam, bugbomb/bomb, snake/init, yoyo/throw,
tankcan/action and shell, lifesync/marker, moonbld/blade and bombs/seed, each EXE6's at its old path its EXE6 wrapper
(EXE6 the same). EXE5's Cannon, HiCannon and M-Cannon draw EXE5's cannon (0c-01) and sound; CircGun and DarkCirc share
one set of kinds (chips/circgun/circgun).

EXE5's hit intake (0x080178EC) takes a hit's NaviCust bug (0x0801103E) before the HP bug drains (0x0800DFEC), where
EXE6's `sub_801AC6C` drains first, and a drain bug's argument goes by its flags (bit 4 adds its low four bits, bit 5
subtracts them, else the level rises to them): the status section's `bugs_before_drain` and `drain_bug_flags`
(MoonBld's bug 0x18 drains a tick sooner; BugBomb's codes are its own, argument 0x12). The rest of 0x0801103E is
EXE6's `sub_80139F6` but for codes EXE5's chips here don't give (0xFD and 0xFC: a drain of 1 on conditions; no 0xF8
or 0xF5, which set their bytes as any other code): not ported.

Not shown by the labs: Static's bug levels 1 to 3; GunDelSol's held A; Katana's and MoonBld's charged step;
Slasher's request 0x80000 (`actions.stun_strike`, EXE5's action 0x49: DarkInvs's drive's end, which only that drive's
timer raises) and its other console's chip name (`sub_801EB18`); lib/arm's NaviStats +0x4C and AIData +0x12 (read as
0); the kinds 4 and up of CrakBom's bomb (no chip throws them); battle flag 0x40's effect object 0x83 (0x080E9FD2,
0x080E9FA4: CrakBom's and Quake's bombs; never in a netbattle); Geyser's geyser (no recording throws it into a hole).
(An auto-battling navi's VarSwrd pick, the side's sword pick, is DarkInvs's drive's: 0x0802C110 sets AIData
+0xF0 for the story navis and DarkInvs, never Chaos Unison's Dark MegaMan, who takes the joypad path; the lab's
chips/0x0bd-darkinvs/auto-battle-varswrd shows it.) EXE5's field obstacles' chips (Wind, Fan, RockCube, BoyBomb1 to 3,
RedFrut1 to 3, Voltz1 to 3 and VDoll) are in §15.11.

### 15.7 EXE5's MegaMan, stages and roles (as built)

- **MegaMan** (content/exe5/navis/megaman, `exe5:megaman`): EXE5's navi 0, NameID 0x180, from EXE5's tables (his
  element, buster bonus 1, move lag 4, banners, actor record, the 30 attach points of EXE5's 0x3C-byte rows), his
  buster and charged shot EXE6's shot actions with EXE5's setups (weapons/: the damage Attack plus the navi's bonus,
  no worn-out rule or cap; a program drawn on every shot, on half the draws). His forms are his souls (§15.8),
  with his own base form `exe5:base` (P1c): his battle sprite is the form's, 00-00 by soul (0x0800DA3A: category 0
  by the soul where NaviStats +0x29 is 0; 08-00, his navi sprite, has 31 one-frame animations). Changing form, he
  takes EXE6's MegaMan branches (NaviStats +0x29 0 in EXE5's code too: the anger, the bugs' stripped programs, the
  move lag), but for EXE6's per-form tick (`sub_80F0608`), which EXE5's table (0x080EB1E8) hasn't: the status
  section's `form_tick` (EXE5 false). His mercy flash blinks in the other phase (0x080137B6 hides him while the
  flash timer's bit 1 is clear, EXE6's `sub_8016934` while it is set): the status section's `flash_hides_on_clear`
  (EXE5 true). The engine asks a player's identity for a Full Synchro aura animation, which
  EXE5 has differently (EXE6's `sub_80C4C52` is absent): 0, EXE6's rule, until EXE5's emotions.
- **Stages** (content/exe5/stages.luau, compat stages.toml): a stage per distinct record of EXE5's
  netbattle settings list (0x0811AF4C, 96 records: as many as the link battle's pick reaches, 0x08129F2C; the
  last, record 95, is netbattle-94), its layout (0x0800BD6C) and its actor list; the lab's
  settings (written to RAM by the Team Battle with its own background and effects) match the list's by layout,
  actor list, music, mode and panel pattern. Those with obstacles (actor types 3, 8, 9: EXE5's boulder, rock and
  statue aren't ported) are listed as waiting; those with metal, sea or lava panels are stages since the rules
  work's P1a (68 stages). The backgrounds are named for their areas (docs/frontend.md §1: no EXE6 background has their tiles). The
  actor lists' addresses are Team ProtoMan's US ROM's; the other three ROMs have the same list, each its actor
  lists a constant away (compat/games.toml, as EXE6's: Team Colonel +0xE8, the Japanese Team ProtoMan −0x3E4 and
  Team Colonel −0x2FC), and a recording's settings record is its traced console's (the BattleState's local side's
  version and region): jp-team-plain matches through it. (Checked over the lab on 2026-10-05: every recording whose
  settings match no record by the US addresses, 51 of them, is record 65 or record 1 on a Team Colonel or a Japanese
  console, its actor list at that ROM's address. None is a setup outside the list.)
- **What objects keep of a code address**, as EXE6's (docs/engine/objects-and-player.md): the dimming chips'
  controllers (BoyBomb's, Wind's and Fan's, RockCube's) and ShadowMan's and MagnetMan's navis keep the address their
  spawner was called through as their Z, ChaosLrd his spawner's in Z's fraction (and his strike, thrown from where he
  stands, falls from it: its velocity is that Z over its 15 ticks), MoonBld's blade its phase routine's low byte as its
  panel Y, TomahawkSoul's grass the low bytes of the status reset's soul routine and of its dispatch's table as its
  panel. The content keeps Team ProtoMan US's numbers; compat/games.toml has each other ROM's (`spawner_z_fractions`,
  `panel_xs`, `panel_ys`, which `tools/exe5/gen_rules.py` finds in each ROM by the code around the address), and
  kinds.toml says which kind's Z moves from the address (`z_moves_from_spawner`: ChaosLrd's, and what wears his Z)
  or falls from it (`z_falls_from_spawner`: his strike). The comparison expects the traced console's ROM's. Found by
  the Team Colonel souls recorded on their own console (TomahawkSoul's grass; BoyBomb3's controller, used in
  KnightSoul), and checked on copies of eleven scenarios on the other three ROMs' consoles (2026-10-05: all 33
  whole).
- **Panels** are a registered section now: EXE5's types, EXE6's roads and either-side step rule (EXE5 has neither;
  the section must name the engine's 13 types; nothing of EXE5's reaches them).
- **Roles** EXE5 shares with EXE6: the sparks (EXE5's 0 to 0xD are EXE6's rows), the deletion, recovery and cut-in
  effects (EXE5's 3, 6 and 0x1E are EXE6's), the statuses (by EXE5's bytes), the forced charged shot, the first
  barrier's hook.
- **Compat** gains records.toml (weapons by routine number, projectile rows, barriers) and kinds.toml (EXE5's
  kind numbers, as the replays meet them).

More for §15.3:

12. **A base form per game**: the engine refuses two base forms in one content; EXE5's MegaMan, whose souls are
   forms, needs his own (or the base form to be a navi's).
13. **The custom screen's end**: EXE5's result is 49 words on the link (0x08009A5E: its NaviStats are 0x60 bytes,
   EXE6's 0x64; `sub_800B3A2` sends 50), so it is in a tick sooner than EXE6's would be; EXE5's Team Battle screen
   (0x08025EF2) takes the results as EXE6's does (0x080266FA: the hands installed, the HUD's wait task off and its
   icons and chip window on) and closes the tick after (its state 8, 0x08025FEC). Built as the flow section's
   `result_words` (EXE6 50, EXE5 49, of the sending side's game); an earlier `custom_closes_with_results` (the screen
   closing on the tick both results are in) had the close on the right tick but the results a tick late. EXE5's
   screen sets no AIData +0x0F on the navis (EXE6's `sub_8009338` does): the rules' `custom_closed`.
   **The dark chip offer** (0x08025114, from the screen's opening 0x08022C5C, after its hand size 0x08025BE4 and
   before the folder closes up 0x080250E6): a worried or dark MegaMan (emotions 1 and 5) gets one of his folder's
   dark chips (0xBB-0xC6) moved to the place after the hand unless the first is dealt: the first taken, each later
   one on an RNG1 draw's low bit, DrkRecov first under a quarter of his HP. Built in EXE5's light and dark module's
   `custom.deal` (the custom screen's deal hook, with `custom.folder`, `custom.swap_folder`, `custom.hand_size`);
   its RNG1 draws were what the dark chip recordings' consoles were ahead by.
Items 14 to 18 are built (rules-in-luau.md, "P1b"); 19 and the dark chips' costs wait for a replay that needs
them.

14. **A dark MegaMan clears the holy panel under him** (§6.1; 0x08017136, from all six intake updates, EXE6's
   `sub_801A9B8` to `sub_801AC6C`): every tick, an object on a holy panel whose side's light/dark value is 499 or
   less turns it Normal. Team Colonel's MegaMan is dark in the lab (exe5-compat's `LightDark` reads it from the
   setup's NaviStats +0x44), and stands on the default stage's holy row: every replay on it differs at the fight's
   first tick until the engine has the side's value and the rule (a hook in the intake, or the light-and-dark
   system's).
15. **The camera shake draws from the battle's RNG** (0x08030D78, EXE6's `camera_doShakeEffect_80301e8`): two
   GetRNG2 draws each shaking tick where EXE6 draws from GetRNG1, one channel (EXE6's two), and no shake while the
   time is stopped; EXE5's shakes change the simulation's RNG (a BlkBomb's landing: 15 ticks of draws).
16. **The hit spark's first tick** (0x080E0870, EXE6's `sub_80E0864`): EXE5's spark doesn't step its sprite at its
   init, so it lasts a tick longer (spark.rs). EXE5's frame load (0x03006898, in IWRAM) also leaves the palette
   offset of the frame's first part (the sprite's +5) to the sprite's next step (0x03006948), where EXE6's
   (`sub_3006730`) takes it: a spark's first frame shows in its palette 0 (an elec spark blue-white, not yellow),
   as does any sprite drawn after a frame load and before its next step (the effects section's
   `load_sets_part_palette`, `Look::part_palette`; presentation).
17. **The metal slide's speed** (§15.2): a step up or down covers its 24 pixels in 3 ticks (8 a tick); the
   engine's slide moves 6. Its order by the move's direction is the tables', as recorded. A navi's slide and drag
   both go 8 a tick in depth in EXE5 (0x0801361E, 0x080143A8). A slide arriving on metal goes on as on EXE6's roads
   (0x08013564), and one arriving on sea ends.
18. **A custom screen asked for in the fight** (L or R with a full gauge) opens a tick after EXE5's does: EXE5's
   state 0x20 (0x08007774) opens the screen itself once the reversions are done, where EXE6 goes through state 0x24
   first. The recordings that dig for a chip over several turns (HolyDrem's, the chip lab's `dig`) met it.
19. **EXE5's obstacle framework** (0x08018404, EXE6's `sub_801B750`): outside the dimming, an obstacle not in its
   first action on a solid panel tests a word (+0x5C of the toolkit's +0x18) against 0x20 or 0x10 by its panel's
   side, then the panels beside it, and may spawn attack object 0x30 there (0x080CAB02, 0x080CAAE2: ColonelSoul's
   army, read in §15.11); BlkBomb's idle and return actions are 7 and 6 (EXE6's 9 and 8), its table without frozen
   and bubbled. Not met in the replays yet.

### 15.8 Soul Unison (as built)

- **Where it is.** A soul is a form of MegaMan's, named by its id (`colonelsoul`), never by the original's number.
  - *The soul's own* is in its folder, navis/megaman/forms/<soul>/: the form (init.luau), its weapons, its sprite's
    attach points (attach_points.luau) and what only it does. The form says what the rules ask of a soul: its
    family (`soul = { family }`, the engine's), its buster arm's animation (`buster_arm`), and the souls module's
    fields (the system's `extends`; content/exe5/types.d.luau types them): its navi's image at the change
    (`image`, a `SoulImage`: the sprite, and what the image puts on and takes off), what it adds to the custom
    screen (`custom`, a `SoulCustom`: below), its blade's animation (`blade_anim`), its charged sword's step behind
    the enemy (`steps_behind`), VarSwrd's wait (`var_sword_waits`) and the cameras' shake at its change
    (`emerge_shake`).
  - *The system's* is rules/souls/: the soul button and the soul's choice (custom.luau), the turns (init.luau), the
    change and the revert (change.luau, revert.luau, each defining its action, which every soul names), the image
    and the shade (image.luau, shade.luau), Chaos Unison's failure (chaos.luau). It names no soul: custom.luau
    gathers each soul's `custom` from MegaMan's `forms.souls` and asks the soul the side's navi is in.
  - *The original's number* (NaviStats +0x2C: 1 to 12) is compat's: content/exe5/compat/records.toml's `[forms]`
    (exe5-compat's `Compat::form_number`, `form`), which a recording's setup (each version's six souls, by the
    save's flags) and the save import read. MegaMan lists his souls in that order (navis/megaman/init.luau's
    `forms.souls`): the order of the soul button's icons in a pack, by which the frontend takes a soul's icon
    (nettai-render's `soul_place`; nettai-match's `megamans_souls_are_in_the_order_of_their_numbers` holds the
    list to compat's numbers).
- **The soul button** (the souls module's button `soul`, rules/souls/custom.luau: EXE5's layout's slot 11,
  0x08023C54, 0x08024B28, 0x08024972): shown for MegaMan with souls, outside battle flag 0x40, with the save's
  Soul Unison (event flag 0; nettai's setup lists souls, none for a save without it), unless he is worried or dark
  (in a soul or angry he may);
  lit for the last pick's family when the navi has a soul of it (a form naming its `soul = { family }`),
  the save has the soul (the setup's `souls`, a list of forms: exe5-compat gives a recording's side its version's
  six) and it isn't given this round (the system's `souls_used`, a bit a soul by its place among the navi's souls,
  the original's word 0x02034E10: Soul Unison and Chaos Unison apart; a dark
  chip's is Chaos Unison, which needs the setup's `chaos_unison`, flag 0x236). Pressed: the window `soul_unison`
  (EXE5's state 9: the icon's flight, fades 0x34 and 0x30, then the white), whose white step puts the soul first in
  the selection in place of the chip given up (`custom.trade_last_pick`). At OK (`custom.confirmed`) the transform
  record asks for the soul's form, 3 turns and the bonus (NaviStats +0x32, `soul_turn_bonus`: at most 9, under 0
  one) or Chaos Unison's 1 (0x08024FF6; `custom.set_form`'s turns and Chaos flag, `TransformRequest::turns`,
  `chaos`); the chip given up leaves the folder in the soul's place. B on the soul puts the chip back. (Until
  2026-10-04 this was the custom screen's Rust: `SlotKind::Soul`, `Phase::SoulChosen`, `SoulUnlocks`.)
- **What a soul adds to the screen** (0x08023CF8, by the soul MegaMan is in, when the side's navi is MegaMan, the
  screen's +0x10): its slots 8 and 9, over the hand's last two chips. Each soul's is its own, a module in its
  folder that makes the form's `custom` (a `SoulCustom`: the state fields it keeps, its buttons and windows by
  name, and its `hand_size`, `deal`, `confirmed` and `turn_opened`). rules/souls/custom.luau gathers them from
  MegaMan's souls into the system's buttons, windows and state, and calls a soul's while the side's navi is in that
  soul, with the system's state (its buttons and its `deal` not in battle mode 1, where the routine reads no
  soul). What the souls keep of a screen starts fresh at each deal, every soul's and whatever soul the navi is in:
  the system sets each field of each soul's `custom.state` to what a fresh state holds, by the field's declared
  type, before the soul's deal runs (the original zeroes the screen's record as it opens, 0x08022CA2: out of
  MeddySoul there are no capsules). SearchSoul's Shuffle (kinds 4 and 5) and NumberSoul's hand of ten
  (0x08025BE4) are the two bullets after this one; the two that change a chip are:
  - **MeddySoul's capsules** (kinds 6 and 7; navis/megaman/forms/meddysoul/capsules.luau): two of five capsule chips a screen
    (0x17C YelCapsl, 0x17D BlkCapsl, 0x17E WhiCapsl, 0x17F PrpCapsl, 0x180 PnkCapsl: chips/capsules), one draw of
    the console's RNG1 as the slots are laid out (0x08023D30: bits 1 to 4 and 17 to 20 into a table of sixteen,
    0x08025E60, or 0x08025E80 while MegaMan's HP is under a quarter of its maximum; the system's deal hook, after
    the dark chip's). A capsule is on offer while the last pick is a chip that deals damage with none mixed in
    (0x08024C74); a chip with one offers no soul (0x08024B54). A on it (0x08024A02) runs the screen's state 0x3C
    (0x0802373A: the window `capsule`, the soul's choice's steps with the capsule's icon), whose white step marks
    the pick (`custom.attach_to_last_pick`: the slot's +4 and +5, by 0x08023824) and uses the capsule; B on the
    chip frees it (0x08024D78). The mark goes into the hand's flag byte (+68, `ChipHand::modifiers`), and the
    chip's use reads it (0x08010368, 0x0800FFF6; the chip-use rule `mixed_modifiers`): 0x04 the damage word's
    0x2000 (YelCapsl: confusion), 0x08 its 0x1000 (BlkCapsl: blindness), 0x02 its 0x4000 (WhiCapsl: paralysis),
    0x20 its 0x0800 (PrpCapsl: the HP bug), 0x10 a tenth of the user's maximum HP healed, rounded up (PnkCapsl).
    The capsule chips never reach a hand; a slot shows its capsule as a chip (icon, and in the chip window the name
    and picture alone, 0x08024422), and R describes it. Their records' uses are leftovers (the cannon's action,
    Poltrgst's and RockCube's dimming routines, the supports' controller, routine 50: lib/supports/dimming).
  - **ColonelSoul's Arm Change** (kinds 8 and 9; navis/megaman/forms/colonelsoul/arm_change.luau): on offer while the last pick
    is a standard chip of no family that deals damage and neither dims nor is dark (0x08024C00). A on it
    (0x080249D6) runs the screen's state 0x38 (0x08023694: the window `arm_change`): the chip leaves the picks for
    the button (`custom.hold_last_pick`; five more can be picked) and its icon blinks in the column for 30 ticks.
    B, with the picks as they were then, puts it back (0x08024CFC). At OK it is the turn's arm chip (the transform
    record's +6, 0x080123FC; the system's `arm_chip`) and leaves the folder (0x080250C8, the Regular chip's flag
    untouched). At the turn's start, before either side changes form (0x08011DDC, 0x080124AE: the hook
    `turn_opened`), a MegaMan in ColonelSoul takes it (AIData +0x32, the navi's `weapon_chip`) and his charged
    shot is the weapon routine 0x13 (0x0800F7D8: the chip loaded as the attack and used, the charge table's row
    0x13, 120 ticks at every Charge; forms/colonelsoul/arm) for that turn; without one, the soul's own (0x14). A
    weapons' reload (a change of form, a status reset) puts the soul's own back. The soul change copies the
    record's +6 into AIData +0x32 too (0x08012102), and NumberSoul's image's face copies it for a tick as register
    garbage in its position, where the engine has 0xFFFF whatever the chip (a replay's comparison translates it).
  - The lab's scenarios (souls/06-recovery/capsule-*, ten; souls/07-obstacle/arm-change*, five) match on every
    frame and every sound call: each capsule's effect, the low-HP table, the refusals, B, a soul after a capsule,
    the arm chip fired twice and gone the turn after, B's order through later picks, Arm Change with a soul.
  - On screen (nettai-render `custom`; docs/frontend.md's EXE5 console): a capsule's slot is drawn as a chip's
    (0x08024114: its icon, the empty icon once used; code 0x1B, blank; palette 12 while unavailable, 0x08024200;
    the chip cursor, 0x080246B8), its chip window the name and picture alone (0x08024422: no frame colors, no
    code, element or damage), its mix the soul's choice's sprite with the chip's icon (the chip records' icons
    are the table 0x0874A738 the state loads from). Arm Change's tiles are 0x086FA7CC, the second set for state 1
    alone (0x0802415A), its picture 0x0874F5B8; the chip it holds is a sprite over it (0x080254F4: x 0x45, y 0x84,
    sprite palette 10, the HUD's icons'), drawn by the choosing state, the soul's choice and the blink's hidden
    ticks from 20 down; the blink redraws the column's cell from its second tick (0x08023712), so the cell stays
    filled on the tick the chip leaves. Shuffle's chip window shows its uses left (0x080245F2: a digit at
    0x060093A0), which EXE6's routine reads and doesn't draw. All three states draw the emblem and the Regular
    chip's frame after their step. The soul icon's sprite palette is the version's (0x0874AAB8, Team Colonel's
    0x0874BDBC: color 9, the outline). Verified frame for frame against mGBA on 26 scenarios (every custom-screen
    frame; exe5.txt keeps eight).
  - Not built: the Liberation Missions' team navis' part of the same routine (the screen's +0x10 nonzero: a chip
    pair by navi from 0x08025EA0 in slot 9, 0x08023EFE; the navi switch, state 0x40; battle mode 1's button in
    slot 11). A netbattle's navi is MegaMan.
- **SearchSoul's Shuffle** (the souls module's button `redeal`, the name the frontend draws the pack's re-deal
  button's tiles and picture for; navis/megaman/forms/searchsoul/shuffle.luau; the per-soul slots,
  0x08023CF8, by the soul the emotion routine leaves in r1, so never in battle mode 1): slots 8 and 9 (types 4 and 5),
  its right neighbor the special slot, its left the eighth chip's; three uses a screen (3 less the screen's +0x16,
  which each open zeroes). A on it (0x080249B0, no sound of its own; EXE6's ChpShufl plays one, now its button's
  content's `custom.play("redeal")`) is the framework's re-deal, the screen's state 0x28 (0x08023488, EXE6's
  `sub_80271F8`), a `hop` each of its eight steps (the roles' `custom_redeal_shuffle`). EXE5's keeps some of the hand
  in the hand (0x080253E8's table, 0x080254C8: the custom-screen rules' `redeal_kept`, by how many of the hand it
  deals again: 0, 0, 0, 1, 1, 2, 2, 3, 3): the hand's are shuffled among themselves, then all but the first kept
  with the rest of the folder. Its scenarios: souls/03-cursor/shuffle, shuffle-3, shuffle-picked-1 to 5.
- **NumberSoul's hand of ten** (navis/megaman/forms/numbersoul/hand.luau, through the souls module's
  `custom.hand_size`; 0x08025BE4): MegaMan
  in NumberSoul is dealt ten chips whatever his custom level, in battle modes 0 and 6 (mode 1 takes the same branch, but
  its emotion routine, 0x080127C0, leaves the stats' address in r1, not the soul: the custom level's hand; any other
  mode deals five, which nothing answers yet). Its scenario: souls/09-plus/hand-of-ten.
- **The change** (rules/souls/change.luau, the souls' `change`, EXE5's 0x08011F74, run by the shared turn-start
  sequencer): onto the future panel, a flash, his moves stopped (`stop_moving`); the soul's image (rules/souls/
  image.luau, actor 0x2A, in the look its form's `image` gives: its navi's sprite 08-xx and what it wears) spirals
  in, blinks and fades; MegaMan emerges in the soul (the
  old form's end hook and the new one's start hook, `put_on/take_off_form_overlay`; TomahawkSoul's shake, its form's
  `emerge_shake`), the
  status reset; Chaos Unison's 11 ticks arm the chaos charge (AIData +0x12, not yet modeled). The souls module
  (rules/souls/init.luau) keeps the soul's turns (AIData +0x0F), counts them down at a turn's start (0x0801248C)
  and asks for the revert when they run out (0x0801246C); the revert is the souls' own (`FormData::revert`, the
  pause handler's: rules/souls/revert.luau, 0x080121D8: back to `exe5:base` with no state saved).
- **ProtoSoul** (soul 1, sword): sprite 00-01, weapons from 0x0801CA1C's row 1: the buster, the charged slash
  (routine 3: WideSwrd's slash, 80 + 10 × (Attack + 1), counter byte 0x94), Sword chips charged with A (routine
  5: any Sword chip but a dimming or dark one, `charged_chips`' `plain`, 0x0801090A; doubled, 0x080103D0). EXE5's
  blade animation goes by the soul (0x080EC038: ProtoSoul 13, ColonelSoul 14, ShadowSoul 15; each form's
  `blade_anim`, which lib/swords' `blade_anim` reads).
  souls/01-sword/unison matches every frame; its B+Back shield is routine 4 (EXE5's guard as the Reflect program's).
- **The other eleven** follow the pattern: a form file each under navis/megaman/forms (sprite 00-0n, image sprite
  08-0n, family, weapons from 0x0801CA1C: +5 the A-charge's routine, +6 the buster's, +7 the charged shot's, +8
  B+Back's, +0x11 Chaos Unison's, a dark chip by routines 0x30 to 0x3A: weapons/chaos), the element by soul
  (0x0800E634: NapalmSoul Fire, MagnetSoul Elec, TomahawkSoul Wood, ToadSoul Aqua), the charge table's rows
  (0x0801CA6C, ten bytes a routine). What a soul wears is its identity's `parts`, a row of EXE5's body overlays
  (lib/body_overlays: 0x080C35DC; GyroSoul's propeller, row 2); the soul's image wears its navi's (0x0800EDBC:
  GyroMan's propeller, NapalmMan's cannon, Colonel's cape, KnightMan's ball and chain). EXE5's MegaMan's hooks
  restart what he wears after an animation change, a flinch or a drag by having it reload its animation
  (0x080C374E: the reactions section's `overlay_restart = "reload"`, where EXE6's steps it at once).
  - **The status reset by soul** (0x08011B92; a NaviCust change's, 0x08011CBC): GyroSoul's FloatShoes, floating
    body and AirShoes and ShadowSoul's FloatShoes and floating body are the form's `status_reset`; the rest is the
    form's `reset` hook (`FormDef::reset`, called after the flags): SearchSoul's reveal of the other side's
    invisible navis (effect 0x8F), TomahawkSoul's grass (effect 0x16), ToadSoul's dives (the navi state "dives"),
    ColonelSoul's arming of its side's soldiers (`obstacle.arm_soldiers`, 0x08011C44; §15.11). EXE5's put-on and
    take-off routines are one table each, by soul (0x0800F024's 0x0800F038, 0x0800F088's 0x0800F09C): the base form's
    Hub Style shade (row 0), GyroSoul's propeller (row 2, the default: its identity's parts), NumberSoul's layer
    (row 9, actor 0x54), the rest nothing. A form's `put_on` and `take_off` replace the defaults (its identity's
    parts; what its identity's death hook takes down), which a hook may call (`put_on_form_parts`,
    `take_off_form_parts`).
  - **The chip use by soul** (0x0800FF48), by form data: `priming` (GyroSoul: a Wind chip primes it, AIData +0x0D,
    0x080102D2; primed, the next damaging Wind or Null chip is doubled, and neither Full Synchro nor anger doubles
    meanwhile, 0x0801026C), `grass_doubles` (TomahawkSoul's Wood chips on grass, which the use turns normal,
    0x0801032A), `front_guard` (KnightSoul's 50 invulnerable ticks for a damaging chip used with the panel ahead
    not its side's, 0x08010392), `charged_action` (NapalmSoul's charged Fire chips start action 0x4B, a napalm bomb
    with the chip's damage: 0x08010442), `charged_chips` and `charged_bonus` (0x0801090A, 0x080103D0: Proto Sword,
    Knight Break, Magnet Elec, Toad Aqua and Napalm Fire doubled; Shadow Sword without a bonus), `move_lag`
    (ShadowSoul's 0).
  - **The hand's bonus** (0x0800D0A6, from the hand entry 0x0800D054 and the chip window 0x0800D018): NumberSoul's
    damaging Null chips +10 and NapalmSoul's damaging Fire chips +40 on a use that isn't charged with the A charge
    not full (the form's `chip_bonus`, `uncharged`); in any other form, MegaMan's damaging Aqua chips +30 on sea
    (the navi's `panel_bonus`), the sea under him turning Normal as the use is prepared (0x080100B0), after the
    light and dark module's dark-chip use and cost (the system hook `chip_cost`, 0x08010030) and before its
    light/dark check (`chip_check`, 0x08010118).
  - **The charged swords** (action 0x13 by the attack's charge, 0x080EBD04; lib/swords' `SlashSteps` for the common
    slash): a charged slash steps two panels ahead (charge 1, 0x080125D4), or in ShadowSoul (and by a charge of 2)
    warps behind the enemy, turned round (0x08012538); the afterimages and the step back go by a charge.
  - **The B+Back moves**: ProtoSoul's guard (routine 4, the Reflect program's), MagnetSoul's immobilizers ahead
    (0x25: instant effect 10, a glow, @exelib/instant/immobilizer with EXE5's mark), ShadowSoul's anti-damage stance
    (0x45, action 0x3D: @exelib/navicust/anti-damage, the AntiDmg program's) with the attack's variant 1, so its
    counter throws at the nearest enemy ahead (`attack_variant`, AIAttackVars +3); EXE5's stance counter runs its
    first step at once (0x0800E340: the reactions section's `stance_counter = "at_once"`).
  - **ToadSoul under the sea** (0x0800DF5A, 0x08017030, 0x0800DEB2; by the arena's panel rules' `submerges`): a
    diving body on sea is under the surface (its dive timer, EXE5's CollisionData +0x2C, held), its flag 0x80000000
    on (the bit EXE6's bubble has; EXE5's kernel doubles no elec hit by it, the panel's elec bonus does) unless it uses
    an action, is dragged, flinches or is paralyzed; under (0x80000004) it is hidden, a ripple over it (effect object
    0x3E, objects/dive_ripple, the role `kinds.dive_ripple`; a splash as it starts, row 0x5D). EXE6's submerged
    state (`sub_8010162`, +0x28 in EXE5) stays apart.
  - **GyroSoul's propeller while primed:** MegaMan's per-form tick (0x080EB1E8's row 0, 0x080F04CE) in soul 2 sets
    his body overlay's animation offset (its ExtraVars word, 0x080C451A, which EXE5's overlay adds to its owner's
    animation and reads its depth by: 0x080C365C) to 17 while primed, else 0: the form's `tick` hook and
    `set_overlay_anim_offset`. **KnightSoul's ball** is KnightMan's spawned by 0x080C768A, not KnightMan's
    0x080C76AC that also keeps it running while dimmed (chips/knightmn/ball's `spawn`). **ShadowSoul's Chaos
    Unison** loads DarkInvs (routine 0x31); DarkInvs's stand-in and drive end know MegaMan by his key, his navi's
    module reaching theirs through the soul.
  - **The hit kernel** (0x0801691C, EXE6's IWRAM `sub_3007218`; the reactions section's `hit_test`): no
    FloatShoe test (EXE5's collision types have no 0x80, so EXE6's test would keep every hit off a floating
    ShadowSoul or GyroSoul), the Elec element reaching a submerged or bubbled side (0x80000004), a guard broken by
    types 0x1002 and marked unless the hitter has 0x0C004000.
  - **Built:** all twelve: ProtoSoul, GyroSoul (routine 9, action 0x3C: a tornado, attack object 0x1E, along the
    three panels ahead), SearchSoul (8, 0x3B: five shots at the nearest enemy navi's panel, 0x08012E50), NapalmSoul
    (0x19, 0x44: three fire bullets, rows 0x11 and 0x12), MagnetSoul (0x15, 0x42: a paralyzing field, attack object
    0x75, on the panel ahead and a pull over the six panels ahead), ColonelSoul (0x14, 0x43: the screen divide on
    the first enemy ahead), MeddySoul (0xE, 0x3E: a capsule, mode 0), ShadowSoul (0x17: LongSwrd's slash),
    NumberSoul (7, 0x3A: the dice), TomahawkSoul (0x1A, 0x45), KnightSoul (0x11, 0x40: KnightMan's ball's swing,
    confusing) and ToadSoul (0x2D, 0x50: ToadMan's notes). The souls recordings match every frame but MeddySoul's
    and KnightSoul's unisons (RedFrut1, BoyBomb3: the obstacle chips), ShadowSoul's Chaos Unison (DarkInvs: the
    navi chips') and four Chaos Unisons that stop at the Dark MegaMan's panel.
- **Chaos Unison** waits on the engine: its charge (AIData +0x11's weapon, the routine's charge row by the chaos
  level AIData +0x6C, the cycle 0x080105F8 of 0x08010650's rows, the release's requests 0x8000 and 0x10000, the
  idle's start of the chaos weapon or of action 0x39) and, on a failed release, action 0x39 spawns the Dark MegaMan
  (actor record 0x18D: a navi of AI index 0x16, 500 HP, on a random panel for the other side) that runs EXE5's
  auto battle AI (0x0802B4AC, AIData +0xF0): a second navi on a side, driven by an AI, which the engine hasn't.
  Eleven of the twelve chaos recordings fail the charge.

### 15.9 A second navi on a side: Chaos Unison's Dark MegaMan (design)

**What EXE5 does.** A Chaos Unison charge released off its window starts action 0x39 (0x080EE63C): the battle dims,
MegaMan flashes, and on its second step's eighth tick a navi appears for the *other* side (0x080EE6BC: a random
solid, empty panel of that side's area, 0x08010226 as EXE6's `sub_80129F4`; EXE5's generic actor spawn 0x08006AAE,
EXE6's `sub_80076A0`): actor record 0x18D (actor type navi, AI index 0x16), 500 HP and its element from the
record's enemy structs (0x0800D138, 0x0800D160), its Param2 1 and AIData +2 1. Then MegaMan reverts. The navi:

- is an actor of the player's kind (object kind 0 with its own AIData and collision), on the side's actor list
  (0x08006B86: four slots a side) but not counted (AIData +2): neither its spawn nor its deletion (Param2 1 skips
  0x08008AFC) changes the side's counts, so it neither ends the round nor keeps it going; the round still ends with
  the counted navi (the side's player);
- reads its side's NaviStats (EXE5 reads them by alliance: the opponent player's Attack, Charge and the like) and
  none of the side's input: the pad is copied to the side's player alone, and its idle (0x080EAFE0) dispatches by
  the side's input mode and the record's AI index to EXE5's auto battle AI (0x080EB068[0x16], 0x080F1D48,
  0x0802B4AC);
- the AI (about 3.6 KB, 0x0802B4AC to 0x0802C438; its state AIData +0xF0 to +0xFF) moves, fires its weapon routine
  0x3E and uses chips from a list per side (0x02034C20 + side × 0xE0: up to 42 chip ids, a count at +0x54, sixteen-
  byte records from +0x58), its own, not the side's hand;
- is targeted as any navi: hits by its collision, and the searches over the side's actor list (a meteor's target,
  a lock-on) see it; it is deleted as any navi (its deletion leaves the slot to its destroy);
- goes with the round's objects at the round's end (the next round spawns only the stage's actors).

**The engine (proposal).** EXE6 never has a second navi on a side, so none of this runs for it; EXE6 stays byte for
byte the same.

1. *Spawn:* `battle.spawn_navi(spec)` (0x08006AAE): an actor of the player's kind with `spec.identity`'s actor
   record (its type and AI index), `spec.hp`, on (`spec.x`, `spec.y`) for `spec.side`, Param2 and `not_counted`
   1: on the side's alive actor list, not counted. The engine's actor bookkeeping is EXE6's already (the four
   slots, `actor_count`, `alive`, `not_counted`, `spawned_actors`), as are the deletion's Param2 test and the
   destroy's freeing of a not-counted actor.
2. *Who the player is:* the side's player stays `battle.player(side)` (the spawned list's first slot): the pad,
   the hand, the custom screen, the HUD's chip window and emotion window follow it alone, as now. Nothing in the
   custom screen or the HUD changes (the rules agent's S6 doesn't meet this).
3. *Its decisions:* idle asks the side's systems' `controller` hook (S3) for a navi that isn't the side's player,
   as it does for a `controlled` form: "nothing", "chip", "buster", "moved", carried out as idle does. EXE5's
   auto battle AI is EXE5 content: a system in EXE5's stock ruleset (rules/auto_battle) whose controller answers
   for its AI index 0x16; its per-side list is the system's side state, its per-navi state (AIData +0xF0, sixteen
   bytes) an actor state the system declares (`actor_state`), allocated with the actor (several Dark MegaMen can
   stand on one side: each Chaos Unison's failure brings one). A side whose ruleset hasn't the system leaves such
   a navi standing (a mixed battle's EXE6 side); the souls module brings EXE5's AI into a mix that needs it.
4. *Its chips and weapons:* `navi:start_chip_attack(chip)` (S4) for the list's chips, the weapon routines by
   number as the content's weapons (0x3E), its stats the side's (`battle.navi(side)`, as EXE5 reads them).
5. *HP, deletion, targeting, the round's end:* the navi's own HP and collision; its deletion as any navi's, not
   counted; the round's end the counted navis' (unchanged).
6. *Rollback and netplay:* the spawn, the AI's states and the list are battle state (snapshotted and digested);
   no input reaches the navi; the protocol doesn't change.

**Then:** the chaos charge itself (AIData +0x12 armed, +0x6C its level, +0x1C/+0x1F the cycle by 0x08010650's
rows, the release's requests and idle's starts: Rust, by the side's game's rules, EXE6 never arming it), the
failure action 0x39 (Luau), the shade (actor 0x2B), and the success's chaos weapon (AIData +0x11's routine:
ProtoSoul's DrkSword).

**As built (exe5-port-6, 2026-10-03).** All of the above, verified on every frame of the EXE5 lab's `chaos-ai/`
scenarios (a failed Chaos Unison with side 0's auto battle data poked into its save: none, Cannon, mixed classes,
chips walked up to, patterns, traps; 10,225 frames, each through Dark MegaMan's twelve seconds and his leave) and
`souls/01-sword/chaos`:

- *The charge* (Rust: armed, the cycle, the releases) and ProtoSoul's chaos weapon (routine 6: DrkSword loaded as
  the attack, its charge row by the chaos level: navis/megaman/forms/protosoul/chaos).
- *The failure* (action 0x39: rules/souls/chaos, the role `chaos_failure`): the dim, the white flashes, the fade,
  the dismissal of the Dark MegaMen across (0x08104284) and the new one on a random solid empty panel of the other
  side's area, then the revert to the base form.
- *Dark MegaMan* (navis/dark_megaman: NameID 0x18D's record, enemy structs, collision and post-init hook) and the
  system that drives him (rules/auto_battle/init: EXE5's auto battle, a system of EXE5's stock ruleset): his
  idle (twelve seconds from his first, then his leave, the navi type's action 7), his tick (the time running down
  outside pauses and dimming, his last three seconds blinking, the battle's end ending it).
- *The AI* (rules/auto_battle/ai, 0x0802BA14): its decisions, the buster runs (his buster, weapon routine 0x3E,
  is attack 0x16: three shots, rules/auto_battle/buster), the patterns (never played: below), the reposition, a
  chip's play; the
  pressure picks as written (the front one calls 0x081BC8AC, data: an error; the hole one reads the AI's own
  side's auto battle data; their counters stay 0); getting in place for a chip by its positioning class
  (rules/auto_battle/place: all 33 classes of 0x08029B3C, and the panel searches they share, ./panels).
- *The auto battle data* (nettai_battle::auto_battle): the recordings' exchanged blocks (exe5-compat); a match file's
  `[side.auto_battle]` (below), sent as the console sends it (0x0802C7BE); the netplay offer.
- EXE5's shots raise EXE5's own arm (lib/arm, 0x080EBABE; lib/buster's), the arm a navi in auto battle of AI index 0x16
  raises.

New APIs: a collision's `counter_timer`, `battle.gauge_damage`, a side's sword pick (`battle.sword_pick`,
`set_sword_pick`: `sub_802E070`+0x12, which the AI draws for VarSwrd and NeoVari and EXE5's VarSwrd, when ported,
reads for a navi no buttons drive), the identity spec's `body`. Read as constant: NaviStats +0x2A (class 28's test;
its EXE5 meaning unread, 0 in every setup).

**The auto battle data a save keeps, and how a match states it (2026-10-04).** Read from Team ProtoMan's code and
seven saves (Tango's templates and three played ones):

- *The block* (0xE0 bytes): 42 halfword places (a chip's number; 0x8000 with a pattern's index; 0xFFFF empty), a
  count at +0x54 that only the send writes (a save's is 0xFFFFFFFF), and from +0x58 eight pattern records of 16
  bytes: `dx`, `dy` (signed bytes), five chip places to the first 0xFFFF, and the pattern's score, a word at +12.
  As written the AI reads a pattern's chips to the first 0xFFFF with no other end and takes any other halfword
  for a chip's number (0x0802BCD6, 0x0802C094), so a record with all five places filled would be read on into its
  score (its lower half a chip's number, its upper half chip 0), the next record's `dx` and `dy` as one halfword,
  that record's places, and so through the records to the block's last eight bytes, which nothing writes (0xFF).
  As run, nothing of a pattern is ever read: see *A pattern is never played* below. The engine's type holds the
  record whole all the same (`PatternRecord`: `dx`, `dy`, five places each a chip, 0 or empty, the score; eight
  records in their places; `AutoBattleData::pattern_read` is the read as written), and exe5-compat refuses a block whose
  last eight bytes aren't 0xFF.
  A record the learning never filled is zeros (place 0, 0, five chip places of 0, score 0: the played saves'
  unused records), one nothing has written 0xFF.
- *A pattern is never played* (2026-10-05). The step of a pattern entry (0x0802BC48, Dark MegaMan's family; the
  same code at 0x0802B6DC, the DarkInvs drive's and the last stand's, and at 0x0802B908, a third family's,
  0x0802B800, which the story's navi actors run, 0x080F2036 on: none in a netbattle) works out its place from the
  target, calls the move lag's routine (0x0800E0BE) and then the step test (0x0800CAA0) before it takes the place
  back off the stack (0x0802BC7E to 0x0802BC8E): the test is of what the lag's routine left in r0 and r1, by the
  stats of the navi's side (NaviStats +0x29, the navi its player operates). For MegaMan's (navi 0) that is the lag
  as the column and the lag again as the row (0x0800E0D8: 4 and 4; 0 and 0 in ShadowSoul); for a team navi's (1 to
  12) the lag as the column (4; KnightMan's 10) and, as the row, the address of that navi's row of the lag table
  (0x0801D462 + 11 × its number, 0x0800E0E8: the pointer at 0x0800E0F0), which is no row whatever the lag
  (`pattern_place`: the side's navi's `forms`, as the ruleset asks whether a navi is MegaMan). The field's rows are
  1 to 3 (0x0800B33C), so the test fails for every navi at every place, and the
  entry takes the other branch: a miss (+0xF4), the target's search moved on (+0xFC), a step to a random panel of
  its own area. The step that reads a pattern's chips (+0xF2 = 0x14) is set by the branch that never runs. So a
  pattern entry differs from an empty turn only in being an entry (it is counted, shuffled and turned), and its
  place, chips, score and the record after it show in no battle. Recorded: library-exe5/chaos-ai/pattern-reached (a
  two-chip pattern two columns back from its target in its row, a free panel of Dark MegaMan's own area: he steps
  and punches, 36 steps and 10 buster runs, and plays no chip), pattern-full and pattern-full-zeroed (five chips
  and a score of 3, the records after it 0xFF and zeroed: the same frames). The engine's port had tested the place
  itself and played the pattern from it, which no recording had met: chaos-ai/pattern, the lab's one pattern
  scenario till then, has both its places on side 0's area (dx 1 and 2), where the step fails either way;
  pattern-reached stopped at frame 908 of 1,658 (the miss's draw for the panel) and matches with the test as the
  game has it (`pattern_place`: `can_step(lag, lag)`). The chips' read stays as written behind it
  (`pattern_chips`, by `battle.auto_battle_pattern_read`); a halfword there that is no chip place's chip is an error
  naming the number (content names chips; the chip table's first record, chip 0, a blank record with the plus
  chips' own use and a damage of 1, and its other blank ones have no definition).
- *Who is in auto battle in a netbattle, and whose stats it reads* (2026-10-05). Three ways in: the Dark MegaMan a
  failed Chaos Unison brings (he stands on the other side and reads that side's stats by his alliance: its Attack,
  its move lag, the pattern step's test above), a player under DarkInvs's drive, and a dark MegaMan in his last
  stand. No team navi is itself in auto battle: a dark chip fizzles for any navi but MegaMan and navi 23
  (0x08010056 tests NaviStats +0x29 for 0 and 23, else the use becomes the invalid chip's), so none uses DarkInvs;
  the last stand is the player of AI index 0's alone (0x0802C17A; a team navi's AI index is its number), whatever
  its mood; Chaos Unison is a soul's. But a team navi's stats are read by the Dark MegaMan that comes onto its
  side: recorded as chaos-ai/colonel-side (side 0's Chaos Unison fails against a side that operates Colonel: a
  pattern entry and Cannon in the data; 27 steps, 12 buster runs, Cannon twice, none of the pattern; every frame
  and sound call match). With the games' navis the two reads of the pattern step's row can't be told apart (4 or 0,
  or an address: none a row); the port keeps both as the game has them.
- *A place holding 0* (the halfword 0, chip 0's number; `AutoBattleEntry::Nothing`). The send packs only 0xFFFF away
  (0x0802C7BE), so a 0 is an entry. The decision tests the first place for 0 before it tests for 0xFFFF and takes
  both the same way (0x0802BAC6, 0x0802B55E): a miss and a buster run. So a 0 that comes first stays first, each
  decision that reaches it a miss, until five in a row bring the swap with a random other place (0x0802C0DC); the
  pickers (0x0802BE4E, 0x0802BE9A) read it as chip 0's record, whose byte 14 is 0, and pass over it. No save the
  game wrote has one: a battle's end fills the 42 places with 0xFFFF and then chips' numbers from its standard,
  mega, giga and program advance lists and the pattern entries (chip 0 is class 3, a list it doesn't write).
  Recorded: chaos-ai/zero-first (a 0 first, Cannon among the rest: his buster runs until the swap brings Cannon
  first, once in his twelve seconds) and zero-later (Cannon first, a 0 and Sword among the rest). The engine had
  given the AI `false` for it, which its decision then indexed as a chip.
- *Where it is:* seven blocks at save +0x554C (0x0200554C, the toolkit's +0x78). A battle sends the first
  (0x08009B64: with battle flag 0x40, the operation battle, 0x0802C7A0 builds one from the player's folder instead,
  its 30 halfwords in places 4 to 33). Nothing writes the other six (0x0802C8C2, which copies a block into one by
  its index, has no caller): 0xFFFF throughout in every save, as the first is in a save that has learned nothing.
- *What it is learned from* (all in the save): two tables of a halfword a chip (368 chips each, +0x7340 and
  +0x2340) and 24 pattern records at +0x0000 (16 kept, 8 for the battle's), all zeroed for a new game
  (0x0802C1E0). Only the console's own player is learned from (0x0802C1FC tests the local side). A chip's use adds
  to the first table by its class (record byte 7): a standard or a mega chip 1, or 3 when the caller's third
  argument is over 1; a giga chip or a program advance 1; a special chip nothing. It would add to the second
  table by how the use went (1, 4 or 8), but the weight is tested against a register the chip's test has just
  loaded with 4, so it adds 0 (0x0802C216 to 0x0802C222; the Japanese ROM's code is the same). A chip whose record
  byte 14 has bit 2 also joins a run (0x0802C294: a run is chips used from one column, each within 30 ticks of
  the last); a hit records where it landed for the run's chip (0x0802C3E2, from the hit code 0x08017E7C and
  0x080185F4); a run of two or more that ends (0x0802C2DC: another column, a chip without the bit, or the 30
  ticks, 0x0802C3C4) becomes a pattern, its place the nearer of its first two chips' hits from where the player
  stands, `dx` toward the enemies (eight a battle).
- *The battle's end* (0x0802C540, from 0x0800713C, block 0) sorts each table's chips by class and count
  (0x0814301C: the higher count first, then the higher chip number), takes 1 off every kept pattern's score
  (not under 1), adds 5 to one the battle saw again, sorts the 24 by score (0x0802C820) and writes: places 1 to 3
  the three most counted standard chips of the second table; places 4 to 27 the sixteen most used standard chips
  of the first, by the table 0x0802C790 (4, 4, 2, 2 times, then once each); places 28 to 32 the five most used mega
  chips; place 33 the most used giga chip; places 34 to 41 the patterns with a score (0x0802C892), up to eight;
  place 42 the most used program advance; then the eight highest scored pattern records. A played Team Colonel
  save: 58 chips counted, the block Lance and SideBub3 four times each, Magnum and CrsShld3 twice, twelve more
  standard chips once, five megas, CrossDiv, eight patterns of two chips each, CsmoPris. Places 1 to 3 are empty in every US save seen, as the code says; one
  Japanese save has three chips there and the second table filled like the first, which this reading doesn't
  explain.
- *The send* (0x0802C7BE) makes three swaps among places 1 to 3 and 39 among the other 39 (`sub_8000CDA`: each swap
  two places drawn at random), packs the entries to the front and counts them. That is no even shuffle: a place is
  in none of 39 swaps (38/39)^78 of the time, 13%, so the entry in place 4 leads the sent list about 15% of the
  time where an even shuffle gives 2.6%, and where the empty places are changes what a seed sends. So a battle can
  tell where in the block an entry was.
- *A match states the block whole* (the rules' `auto_battle_places` and `auto_battle_records`, docs/frontend.md §6), as a battle reads nearly
  all of the places: besides their order, a place holding 0 is packed with the entries by the send (only 0xFFFF is
  packed away). Of a pattern a battle shows only that its entry is one (*A pattern is never played*, above: not its
  place, its chips, its score or the record after it); the file carries the records whole as the save has them all
  the same. `[side.auto_battle]` is the 42
  places in the six lists the battle's end writes them in (`first` 1 to 3, `standard` 4 to 27, `mega` 28 to 32,
  `giga` 33, `patterns` 34 to 41, `program_advance` 42), each entry a chip by name, a pattern record's number (1 to
  8), `0` or `{}` (an empty place), any entry in any place as in the block, and `records`, the eight pattern records in order
  (`dx`, `dy`, five chip places of a name, `0` or `{}`, and `score`). Every place and record is stated; a side
  without the section has a block nothing has written (0xFF throughout: a save that never finished a battle), and
  a new match's side what the battle's end writes of nothing learned (empty places, zeroed records: the rules'
  default, block.luau's `nothing_learned`). Left out: the count and the last eight
  bytes. A test holds a block read into a side and written back, by itself and through a match file, to the same
  places and records (blocks as the game writes them; a full pattern before a zeroed record and before a blank one;
  a 0 among the places and in a record; patterns out of the records' order). The save import reads block 0 so
  (`exe5_compat::save::AutoBattleBlock`, the block by number as it is; `exe5_compat::import::auto_battle_of_save`).
  A random match states none (the rules' default: nothing learned), and nothing writes what the game would have
  learned: the battles are one-off (the user, 2026-10-06). A match compiles to the engine's block
  (`AutoBattleData`) place for place and record for record. The learning itself (the tables and the runs during a battle)
  is not ported: nothing of a battle reads it.
- *What a battle's end writes of the records* (0x0802C540's end, read 2026-10-05). The 42 places are built on the
  stack (0xFFFF, then the lists' chips and the pattern entries) and copied to the block's first 0x54 bytes; the
  eight records are copied from the save's 24 learning records (+0x0000), sorted by score (0x0802C820), 0x80 bytes
  to the block's +0x58. A new game zeroes the 24 (0x0802C1E0), and a record with a score of 0 is an unused one
  (0x0802C62A stops at it), so after any finished battle a record the block has no pattern for is zeroed (`dx` 0,
  `dy` 0, five places of 0, score 0: Team Colonel's played save), where a save that has finished no battle keeps
  0xFF there. The count (+0x54) and the last eight bytes (+0xD8) are never written. A learned pattern's unused chip
  places are 0xFFFF (the run's buffer is reset to it, 0x0802C3B2); the buffer's count has no bound and the copy
  into a record takes `dx`, `dy` and five chips (0x0802C3A0: 12 bytes), so a run of five chips or more gives a
  record all of whose places hold a chip.
- *Which chips a battle's end can write among the 42 places* (2026-10-05): those the count tables count, numbers
  under 368 (the writer's loops, 0x0802C584) of record class 0, 1, 2 or 4 (standard, mega, giga, program advance:
  0x0802C1FC counts a use of the local player's by the record's byte 7, a special chip's, class 3, not at all, and
  only a chip whose byte 14 has bit 2; the writer writes those four lists and never the class 3 one it builds).
  Every one of them has a positioning class the AI's table has (byte 13: 0 to 32; checked over the four ROMs'
  tables). The chips of positioning class 255, which the AI can't play (its table has 33 entries and no bound: for
  255 it jumps to the word at 0x08029F64, 0x18809A0C, outside the ROM), are all of record class 3: the blank
  records, the chips past the library (FtrSword to PnkCapsl, the invalid chip) and the team navis' own chips
  (0x186 to 0x1A7: StepSwrd, C-Cannon, T-Swing). So no save the game wrote has one among its places, and only a
  hand-made block can put one there. A pattern record can hold one (a run takes any used chip whose byte 14 has
  bit 2, 0x0802C294: StepSwrd's has), where nothing plays it. rules/auto_battle/data.luau lists every chip with a
  key, these among them (tools/exe5/gen_auto_battle.py, whose `--check` names a chip it lacks).

### 15.10 EXE5's emotions (as built)

EXE5's emotion is its own routine (0x0801270C → 0x08012740; in battle mode 1, 0x080127C0: Full Synchro or normal),
which the engine runs for a side whose rules say so (the status section's `emotion` table: each difference a rule of its own, named for what it does):

| EXE5's | When | The engine's | Face (0x0801AFB4) |
|---|---|---|---|
| 4 | in a soul (NaviStats +0x2C) | normal (the soul's own face; nothing doubles or ends) | the soul's |
| 3 | anger (AIData +0x34) | angry | 1 |
| 5 | a mood of 0 (a dark MegaMan's) | worn out | 4, the dark face |
| 2 | mood 0xFF | Full Synchro | 3 |
| 0 | a mood of 65 or more | normal | 0 |
| 1 | a mood under 65 | **worried** (new) | 2 |

- **The mood setter** (0x080127D6, EXE6's `sub_8015BEC`) leaves a mood of 0 as it is (EXE6's leaves a held one): a dark
  MegaMan never reaches Full Synchro, and anger's 0x80 doesn't lift him.
- **The anger tick** (0x08011A14) is EXE6's but passes over AI index 23, and ends anger on EXE5's 5 alone (EXE6's 5 and
  1: worried doesn't). AI index 23 is no navi a player operates: the actor records of NameIDs 0x18E, 0x18F and 0x190
  have it (a player's type; the table, 0x08014C94 by NameID, ends with them), and 0x18E is ShadowMan's SplitUp shadow
  (navis/shadowman/shadow.luau, a kind of its own in content, which never runs the tick). In the engine it is an
  identity's `never_angers`, which nothing states today.
- **The starting mood** (0x08010EC8's, where EXE6's `sub_8013892` sets 0x80): by the light/dark value (0x0801283A):
  under 470 0, under 500 64, from 1000 190, else value / 20 + 103 (500 gives 0x80, so a light MegaMan's is EXE6's).
  Battle effect 0x20000 holds the value at 500 (0x08010EDC). The hook `starting_mood`, which the light and dark
  part answers.
- **Full Synchro on a counter** (0x08016FDC, EXE6's `sub_801A200`): the counterer, in no soul, to 0xFF through the
  setter (rules/emotion, `exe5:emotion`). The aura (0x0801100C, 0x080C45E0) is EXE6's, for an AI index up to 12 in
  EXE5's emotion 2.
- **The palette** (0x0800DD94, EXE6's `sub_801002C`; presentation): the hook `navi_palette`, the light and dark
  part's. Dark MegaMan (a navi in auto battle of AI index 0x16 or 0x17) 1, another navi in auto battle 0; MegaMan in AI index
  23 2; unable to charge 1; in a soul 0, or 2 with the Chaos Unison charge armed; else by the mood: 0xFF 4, 0 2 (dark)
  or 3 (light), else the value's tier (0x0800DE5C: from 1000 4, from 500 0, from 470 3, else 2). Then Hub Style's
  `hub_style * 5 + 20`, else the element's `* 5` (none in a soul). A link navi's (0x0800DA98, by EXE5's navi numbers)
  is left to the framework: EXE5's content has no link navi.
- **Hub Style** (NaviStats +0x4C): set out of battle by the patch cards' routine (0x08138214, which follows the
  NaviCust's compile) when patch card 111 is installed and on (0x08137A58): 1 by Team ProtoMan's, 2 by Team
  Colonel's (§15.14). It adds 11 to the face (0x0801AF8E: pictures 11-15, the base form's second set of faces,
  `mugshot.variant`, shown by `battle.set_face_variant`) and moves the palette (`hub_style * 5 + 20`). The stat
  `hub_style` carries it (exe5-compat from +0x4C); what else it does is §15.14's.
- The faces' names follow: `megaman-worried` (2), `megaman-dark` (4), `megaman-hub*` (11-15; they were named as
  dark faces).
- **The emotion-swing bug** (0x080113F8, EXE6's `sub_8013DA0`; rules/emotion): while the side's NaviStats +0x24 is
  set, out of battle flag 0x40's mode and out of a soul, every 60 ticks the anger ends and the mood goes back to 0x80
  (0x08011A94), then the emotion swings to one of 0x0801147C's sixteen (seven normal, seven worried, one angry, one
  Full Synchro) but the one it last swung to (every entry of it taken out), drawn from RNG2: angry asks for anger (the
  request, no mood test), any other sets 0x0801148C's mood (0x99, 0x3F, 0, 0xFF) through the setter. It runs from the
  tick of every navi of the player's kind (0x080EAD6A; an auto-battling navi's, 0x080F224C, has none), each with its own
  counters (AIData +0x3A, +0x0B), and a curse (BugCurse) can set the bug mid-round, so every player's navi is ticked.
- **The face in Chaos Unison** (0x08019704): while the chaos charge is armed (0x080125F6, AIData +0x12) the window's
  picture is drawn in its palette 11 on, the pack's *soul*-chaos faces (the soul form's `mugshot.variant`); the
  palette is read as the window draws, so a blink back to the soul's face after the failure's revert shows it plain.
- **Dark MegaMan's appearance** (EXE6's `sub_80164A0`, the shared mid-battle appearance): white, fading over its 30
  ticks (the color shader gray at the second timer's level), which the engine now draws.
  BugCurse's four recordings match through it.
- **The soul break** (0x080122C8, EXE6's `sub_8015766`; the status section's `form_break = "any_form"`): a dark chip used in
  a soul (0x08010070) sets the weakness request, which breaks any form (EXE6's only a Cross or a Beast) to the base
  form: EXE6's Cross break without animation 2 and the overlay's refresh, the overlay's kept stepping, the collision
  region's removal and return, and the flags 0x80110000 and statuses 0x200800 it clears. (In a netbattle a light
  MegaMan's dark chip fizzles first and a dark one has no Soul Unison: no recording reaches it.) **A weakness hit
  doesn't ask for it**: EXE5's status routine (0x08017BF2) calls the hit's mark (0x08017254: effect object 0x6B, EXE6's
  "!!", when the accumulator of the element the navi is weak to took damage, 0x08017284), the counter's bookkeeping and
  the damage, and has no step like EXE6's `sub_801A506`; the request's one setter is the dark chip's. Each game states
  it (the status section's `weakness_hit_breaks_form`): the engine broke any EXE5 navi hit on its weakness (the screen
  dimmed, a team navi reloaded as the base form) until ToadMan's recordings reached one. Recorded: ToadMan under the
  sea and on the plain stage hit by a Thunder, TomahawkMan by a FireHit1, and MegaMan in a soul of each element (the
  lab's souls/12-aqua/weak, 10-wood/weak, 04-fire/weak and 05-elec/weak: ToadSoul by a Thunder, TomahawkSoul by a
  FireHit1, NapalmSoul by a WideSht1, MagnetSoul by a Boomer), ToadSoul also 11 ticks into its charged shot
  (12-aqua/weak-acting: the paralysis cuts the action short): the mark, twice the damage, and each goes on as it
  was, the soul still on.
  - *The damage.* The hit kernel (0x0801691C) shifts the hit's damage left by the element table's answer
    (0x080168F0 over the 28 bytes at 0x08016900: 0 or 1) and adds it to the accumulator of the hitter's element; a
    Fire hit on grass counts once more as Null damage (0x08016AF6). So a Thunder's 40 takes 80 from ToadSoul, a
    WideSht1's 70 takes 140 from NapalmSoul, a Boomer's 60 takes 120 from MagnetSoul, and a FireHit1's 60 takes 180
    from TomahawkSoul, who stands on the grass his status reset grows. The souls' scenarios are on the holes stage
    for this: on exe5-team's own stage the navis stand on its holy middle row, which halves the sum, so the scratch
    recording that first showed the mark (ToadSoul by a Thunder there) lost 40, the chip's own damage, and reads as
    if nothing doubled.
  - *No second weakness.* EXE5 has none of EXE6's secondary weaknesses (a Cross's to sword, wind, cursor or
    breaking hits). Its element routine (0x0800E600) sets the element (the table 0x0800E634 by navi and soul, or
    NaviStats +0x10) and returns, where EXE6's `sub_801086C` goes on to the form's weakness byte (`byte_80108D1`,
    `sub_8019F9E`). Its hit kernel has no call for a secondary weakness (EXE6's `getSecondaryElementWeakness`
    over the receiver's +0x18 and the hitter's +0x19), keeps no secondary element of the hitter (EXE6's +0x19 into
    the receiver's +0x76), adds nothing for a thaw or for elec on a bubble, and stores no multiplier bytes (EXE6's
    +0x74 and +0x75). The mark's routine reads the accumulator of the one element the receiver's element is weak
    to (0x08017284 by element; none for Null). So a soul of no element has no weakness: ProtoSoul hit by a CannBall,
    a breaking chip, takes its 140 with no mark (souls/01-sword/break-hit).
  - *The mark's test* (the status section's `weakness_mark`, a rule each game states). EXE6's routine
    (`sub_801A42E`) tests the multiplier byte its hit kernel stores of each hit (+0x74: the last hit's is left) and
    the final damage: `last_hit_multiplier`. EXE5's (0x08017254) reads the navi's element into the table at
    0x08017284 (`FF 7E 80 82 7C`: no accumulator for Null, Aqua's for Fire, Elec's for Aqua, Wood's for Elec,
    Fire's for Wood: the hit kernel's table read the other way, which is where the engine reads it) and tests that
    accumulator alone: `weak_element_damage`. The two part when a plain hit follows a weakness hit on one tick:
    ToadSoul hit by a Thunder and by a buster shot together, the shot the later of the two (the lab's
    souls/12-aqua/weak-buster: 81 damage), shows the mark in EXE5, where the engine with EXE6's test showed none.
    A barrier or a trap that takes the hit zeroes the five accumulators in both games (0x08017632, 0x08029AD2), so
    no mark there by either test, and a guarded hit never reaches them. Nothing else of EXE5's reads a multiplier
    byte: the engine's two (`exclamation`, `damage_multiplier`) are read by the mark's test and by EXE6's break
    request (`weakness_hit_breaks_form`) alone.

- **The light/dark bug codes** (0x0801103E, the navi's hit NaviCust bug; the hook `navi_bug`, the light and dark
  part's): a hit with hit flag 0x400 brings nothing to a value of 1000 or more (not even the weapons' reload); code
  0xFD is an HP drain of level 1 (code 0x18, argument 1, through the drain's flags rule) on a dark MegaMan (the value's
  tier 2), code 0xFC the same from 500; else neither is anything. Django's hits bring both: his recordings match.

**ProtoSoul's B+Back** (weapon routine 4, 0x0800F634; navis/megaman/forms/protosoul/back): EXE5's guard (action 0x1F,
lib/guard) as the NaviCust Reflect program's (subtype 4): 20 ticks (the params word 0x114's first byte), the Reflect
program's look (its second byte: row 1), row 9's (a look of its own) with the Chaos Unison charge armed (0x914), 50
damage, the B+Back cooldown 40. A guarded hit from the front fires the charged shot's projectile back:
`souls/01-sword/back` (hand-written) matches on every frame, side 1's buster shot reflected for 50.

Seen against mGBA (tools/frontend-compare, unmasked): chips/0x0bc-drksword/hit 1 → 286 of 314 frames exact,
souls/01-sword/unison 19 → 598 of 704 (the dark opponent's palette and face). What still differs there: the emotion
window blinking out after a dark chip (456-537, two frames in four), the custom screen's face box, and the hit
navi left undrawn after Cannon's hit (452-553; before this work too).

### 15.11 exe5-chips-b: chips 0x070–0x0DC, 0x119–0x12C, 0x137–0x138, 0x13B–0x15D, Guard1–3 (as built)

**Program Advances.** A player's formed Program Advances are a `u64` (`ProgramAdvancesUsed`; content may define 64
Program Advances: EXE6's 30 and EXE5's 30 load together). EXE5's full table (0x08027FC8) is 21 recipes and then the
netbattles' table (0x0802801C); only EXE5's operation battle (battle flag 0x40, each side its own gauge: EXE5's setter,
0x0802D590, tests NaviStats +0x2A) tries those 21: a recipe's `operation_battle_only = true`, which
`find_program_advance` skips unless the battle is one (`battle.own_gauges()`, the engine's `OWN_GAUGES`). A recipe's `order` is its index in the full table. bn6battle-verify's
tools/exe5/recipes.py writes EXE5's recipes (and the flag) into the Program Advances' chip files, naming only chips
that have a use; rerun it as chips get theirs.

**Rules.** The `effects` section's `retype = "is_alone"`: EXE5's retype (0x08016B9E, EXE6's `sub_801A9E8`'s counterpart)
sets the self type only (no dimmed bit), leaves the target type and writes the target's `row_offset` plus four
times the side into the next word. A hit's modifier goes into its side's slot by its flip (`hit_mod_by_side`).

**Obstacles and stages.** The rock and its debris (content/exelib/rock), the boulder (content/exelib/boulder) are
makers EXE6's chips/rockcube and objects/boulder wrap (same APIs). EXE5's rock (attack object 0x59, rows by
variant), its debris (effect 0x38) and boulder (attack 0x6E) are in content/exe5/objects; the stage statue (the
Guardian's, @exelib/guardian/statue) takes its stage damage word. The 25 netbattle stages that waited on them are
in content/exe5/stages.luau and compat/stages.toml (64 stage recordings match). The engine's obstacle
service gains `obstacle.throw` (`sub_800F6AC`: the request `sub_8018002` serves; nothing in EXE6 makes it) and
`obstacle.throwable` (an identity's `throwable`, default true; EXE5's mine sets false: Poltergeist's 0x080E8CA0
skips EXE5's NameIDs 0xDA, 0xD3, 0xD2, 0xE5, 0xE4 and 0xE7). A thrown obstacle's landing (`sub_80180EC`, EXE5's
0x08014AB4: r4 = 0x06050001, Param2 0) shows the plain spark: the role `sparks.thrown_obstacle` is `plain` in both
games (EXE6's said `charged`, read from the wrong byte; EXE6 never throws one).

Item 19 (§15.7), read: it is ColonelSoul's army. All four of EXE5's obstacle reactions (0x08018000, 0x08018168,
0x080182D4 and 0x08018404, EXE6's `sub_801B394`, `sub_801B4D4`, `sub_801B610` and `sub_801B750`) add one step after
the damage and the crushing hits (an obstacle they leave standing): outside the dimming and past its first action,
0x080CAB02 asks whether an obstacle on a solid panel (flags 0x10) of side A stands where the other side's
ColonelSoul can use it: BattleState+0x5C bit 0x20 (A = 0) or 0x10 (A = 1), which ColonelSoul's start (0x08011C44,
the souls' table 0x08011BB0 entry 6) sets by its navi's side (0x080CAC1E; 0x080CAC30 clears it, from 0x08011918 and
0x08011B3C). Then for that side S: a body of S's enemy (panel flags 0x04000000 for S = 0, 0x08000000 for S = 1) on
one of the two panels on S's side of the obstacle (toward S's back, 0x080CAB5A, stopping off the field) gives 1;
else one anywhere on S's front of it in the row (0x080CABB0, `object_getFirstPanelInDirectionFiltered`) gives 2.
Either way 0x080CAAE2 spawns attack object 0x30 on the obstacle's panel (alliance S, Param1 the answer less 1,
flipped when Param1 is 0: it faces back toward S's side) and the obstacle's HP and max HP go to 0 (a word store), so
it breaks as any other. Attack 0x30 (0x080CA834) is the soldier: Param1 0 a cannon soldier (anim 1 for 15 ticks,
sound 0xB0, 0x080E9FD2 on the panel ahead; then anim 2 for 24, and 6 ticks in a hit region 0x0705FF02 on the panel
ahead, r6 ColonelSoul's first damage word, r7 3, and effect 23), Param1 1 a machine gunner (anim 4 for 15 ticks;
then anim 5 and three shots 10 ticks apart, 0x080C6D26 with one of four values from 0x080CAA84 by RNG2, sound 0xB9,
the second word); both then blink out over 30 ticks (action 8). The damage words (0x02034000 + 8 × side, 0x080CABF8)
are ColonelSoul's start's: 40 + 10 a buster attack level (`sub_800FE5E`) and 10 + 2 a level, each | 0x00944000.
0x080F8418 (an entry of 0x080F24A0, an auto-battling navi's) sets the bit and the words too. EXE5's obstacle flag word
moves bits too (removed 0x10000, encased 0x6000; EXE6's 0x8000 and 0x3000): the engine's names keep EXE6's, which
nothing outside reads.

As built: the engine keeps each side's army (`kinds::obstacle::Soldiers`: armed, the two words; in the snapshot and
the digest), which content arms and reads with `obstacle.arm_soldiers(side, sword, gun)`, `obstacle.disarm_soldiers
(side)` and `obstacle.soldiers(side)`. ColonelSoul's status reset arms it (navis/megaman/forms/colonelsoul: its
`reset`), and the engine's status resets disarm it first, as EXE5's do (0x08011918 in `sub_8014216`'s counterpart,
0x08011B3C in `sub_80144C0`'s and `sub_80144CA`'s; EXE6 has nothing to disarm). The step runs in `obstacle.react`
for an obstacle whose own game's rules have `effects.obstacle_soldiers` (EXE5's), spawning the role
`kinds.obstacle_soldier` (objects/soldier, `exe5:colonel-soldier`; its sprite 10-20 is `colonel-soldier`) with its
state `gun` set; the soldier's element byte is what the search left in r2 (0x0800BD1D's low byte for the sword's,
the body mask's for the gun's). The chip lab's souls/07-obstacle/soldiers-gun and soldiers-sword record it.

**Shared modules moved to content/exelib** (makers taking a game's look; EXE6's modules wrap them with the same
APIs): anubis, guardian, otenko, justcone, batcan, colorpt, geddon (controller, quake), barriers (visual,
controller), rflectr, rock (rock, debris), boulder, bugfix (glow, controller), h-burst (action, burst), bodygrd
(striker, shuriken).

**Engine.** The damage formula `gauge_level` (EXE5's 73 to 75, CusVolt's: `base` plus 100 by the custom gauge's level,
none when full); `battle.gauge_full` (battle flag 2) and `battle.drain_custom_gauge` (`sub_801DFD0`, CusVolt's drain
outside link battles); ColonelSoul's army (above): `obstacle.arm_soldiers`, `disarm_soldiers` and `soldiers`, the rule
`effects.obstacle_soldiers`, the role `kinds.obstacle_soldier`.

**Chips.** In the range, the branch's 135 chips match all their recordings, and with exe5-navichips' AirSpin1–3,
AqWhirl1–3, Z-Saver, NumbrBl and NeoVari 144 do (525 of the range's 548 recordings, after merging main on
2026-10-03; the 23 left are CopyDmg's, DarkInvs', Jealousy's, LeadRaid's, ChaosLrd's and PileDrvr's, exe5-navichips'
now). The last ones: BugFix; LCrsShld, LStepSwd, LCounter (the Liberation chips: EXE5's
controller, effect 0x8A, gives the side five uses of the ability, which only a Liberation Mission's specials read,
nothing a netbattle reads); Poltrgst (EXE5's own: controller effect 0x72, stand-in actor 0x58, poltergeist effect
0x73); Navi+20; GunDelEX; InfVulc1–3, LifeSrd, PoisPhar, TimeBom+; GreatYo (controller effect 0x70: the leader and
two followers); PitHoky (the puck's row 3, chips/airhoc/puck); SuprSpr1–3 (wave kinds 6 to 8, chips/widesht/variants);
GigaCan1–3 (projectile row 0x0C: hit modifier 0x49, the blast spark; EXE5's projectile has none of EXE6's row-0x0C
bursts; the third afterimage on NaviStats +0x4C reads 0, as lib/arm's); H-Burst (EXE5's shot: a probe of row 0, its
explosions effect row 0x3C; its bursts are 8, the shot's table 0x080DA514 read at the record's word 0x103, past its
four bytes); the instant Program Advances Boxer1–3 (effect 21: a boxer, effect 0x64, punching FireHit's fists down
the rows), ShakPar1–3 (effect 22: a shaker, effect 0x66, sending paralyzing SpShake waves from the back column) and
CacDanc1–3 (effect 23: a dancer, effect 0x65, a field object dropping cactuses, attack 0xB1); HotBody1–3 (action
0x58: a fire, effect 0x57, spreading flames, attack 0x9D, to the enemies around the last ones; its position stays
the spawner's registers, as its copy of the navi's is stored at address 0x34); CusVolt1–3 (action 0x27: a beam,
attack 0xB8, following the navi); BodyGrd (EXE5's sends the striker out at once from its controller, effect 0x6D,
where EXE6's is a trap); ElemPowr (its controller, effect 0x7F: 10 Atk+ a panel of the type the user stands on,
those panels back to normal through the panel changer's rows 15 to 19); RainyDay (its controller, effect 0x75, and
cloud, attack 0x97: a hit over the first enemy navi ahead for each sea panel of the user's side, which turns
normal); ElemRage (action 0x56 and flame, attack 0x98: flames sent on ahead, of the element of the panel the user
stands on, spreading and paralyzing; the attach point read unflipped, `sub_8018842`); WildBird (LarkMan with Param4 1,
the summon table's entry 23: no command, his swoop's variant 6 with row 6's speed and turn, 25 more turn ticks and
its eleven turn animations by side, 0x080DDC44); BlakWing (its controller, effect 0x54, flock, attack 0x7E, perches,
effect 0x55, and wings, attack 0x7F: EXE6's leftover code); the navi Program Advances CsmoPris (EXE5's own CosmoMan,
actor 0x26, and comets, attack 0x74), Football (GridMan, actor 0x25, and balls, attack 0x95, EXE6's leftover code)
and BigNoise (ShadeMan, actor 0x1B, and his noise, attack 0x04: EXE5's own, not EXE6's flame), each in its chip's
folder with the kinds it owns.

**exe5-navichips' fifteen** (2026-10-03, as built; every recording of theirs matches on every frame):

- *Shared with EXE6 in content/exelib* (EXE6's wrappers at their old paths): AirSpin1–3 (common/airspin: the action and
  its top made by a look; EXE5's top attack 0x9B), Z-Saver (common/zsaver), NumbrBl (common/numbrbl's balls and
  controller; EXE5's NumberMan stand-in, actor 0x45, its own), CopyDmg (common/copydmg: EXE5's action 0x24 with the buster
  arm, EXE6's mark, attack 0x28, which in the operation battle hits its panel each tick it marks).
- *EXE5's own, on EXE6's action:* AqWhirl1–3 (the whirlpool, attack 0x5D); NeoVari and its picks CrosSwrd, SprSonic and
  DblDream (VarSwrd's action by its sequences; a navi in auto battle, AIData +0xF0, takes the side's sword pick).
- *EXE6's leftover code no EXE6 chip uses, in EXE5's folders:* Jealousy (effect 0x36: the other side's held chips counted,
  `battle.hand_left` and the objects' `chips_held`; in the operation battle its warning and
  `battle.drain_side_gauge`); PileDrvr (effect 0x6F, piles attack 0x99 and their beams attack 0x9A: a timed beam, not
  AirSpin's top) with its two recipes.
- *EXE5's own:* LeadRaid (ProtoMan actor 0x20, Colonel actor 0x22 striking an X, their charge glow effect 0x85; ProtoMan
  waits on his `prevent_anim`, which Colonel's spawn sets); ChaosLrd (Bass actor 0x51, the dark beast effect 0x47, the
  gathering flames effect 0x4B on the sine table, the chaos strike attack 0x82; its landing's palette flash needs EXE5's
  `effects.palette_flash`: a pause holds either variant, dimming only a modeless one); DarkInvs (effect 0x18, its
  stand-in actor 9; the user driven for 600 ticks, untouchable, by the auto battle AI's other family, 0x0802B4AC, on
  its own side's auto battle data, facing the target searched from a step's column; the status section's `no_charge_drive`: the
  timer at the intake's end, the idle's step asked of the systems' `controller`, the last 180 ticks' gray flicker; its
  end, EXE5's action 0x49, the roles' `stun_strike`, with its dark image, actor 10).

tools/exe5/recipes.py finds chips wherever they are defined (`--check`).

**EXE5's dark MegaMan's last stand** (exe5-navichips, 2026-10-03, as built; its scenarios library-exe5/dark-survival:
holds, antirecv, antirecv-mood, soul, every frame matching). A player MegaMan (AI index 0) of EXE5's emotion 5 (a mood
of 0 out of a soul, unangry; never in battle mode 1) whom a loss of HP brings to 0 holds at 1 HP, once a battle (the
side's statistic 1 marks it spent; NaviStats +0x2A, the operation battle's, rules it out), and asks for the request
0x40000000 (0x0802C16C, from object_subtractHP, 0x0800C6E0, and applyDamageToPlayer, 0x080185A2): the roles' `volley`,
EXE5's action 0x30 (rules/emotion/dark_survival). The battle dims and the screen fades out (the transformation's fade,
`battle.screen_fade`; white, untouchable, his future panel, a flash), the HUD's gauge, HP box and emotion window go (the
HUD part `hp_box`: draw task 7, the box's drawing only, its low-HP alarm sounding on), his dark self comes out and spirals back (actor 0x2E:
rules/souls/shade's code, `shade.make`), he is in auto battle for 720 ticks as DarkInvs's drive does (its end
action 0x49), the HUD comes back (without the gauge in the last turns, `battle.late_turns`) and the screen fades in. The
status section's `hp_loss = "gauge_and_last_stand"` holds EXE5's object_subtractHP (a player's loss drains its side's gauge too: ×128,
in the operation battle 0, 0x555, 0xAAA or 0x2000 by its size, 0x0800C734) and EXE5's applyDamageToPlayer (a hit
shows, white then its sounds, only by the register r1 the check leaves at 0 HP, and one that doesn't show goes
straight to the deletion test, which tries the stand first). AntiRecv's counterattack takes its HP through
`Object:subtract_hp` (EXE6's the same), EXE5's its mood too (0x080E39DC: its damage word's high half, 0x08012820), and
its mark sits where the trap's game puts it (`anti_navi_sparkle`). Also new: the request `drag` (flag2 0x100). Where a
hit landed and which chips a side used are learned for the auto-battling navis' auto battle data (0x0802C294, 0x0802C3C4, 0x0802C3E2:
the battles after; nothing of a battle reads them), not ported. As drawn (2026-10-05, against the original's frames:
tools/frontend-compare's dark-survival/holds, the console's own navi, and holds-side1, the other side's): the
transformation's fades started outside the sequencer are drawn by the fade's own record (the renderer's `layer_fade`:
the tile layers black behind the navis from the fade out to the fade back in), and a shade faces its object's way from
its init (0x0801892E's end, 0x0800C896 and 0x08002DC0, after the sprite's load, which forgets the facing: set at the
spawn it left side 1's dark self, and Chaos Unison's shade, facing right).

**The obstacle chips** (from chips-a's range, 2026-10-03). RedFrut1–3 (action 0x1A's instant effect 15,
0x080D818C, EXE5's own: chips/redfrut/fruit): a fruit (attack object 0x8D, NameID 0xE7) drops on a random free panel
but the back columns (they are reserved while it looks, 0x080D8280) and hops: 4 ticks coming, 40 shown, 4 going,
then onto a random free panel of the other rows (or its own, with none), as many times as the record's second
parameter (5), and gone after the last going. A hit breaks it (1 HP); the side whose attacks, objects or bodies broke
it, alone, gets its gift (the record's first parameter, also its palette) on a random alive navi of its side
(0x080D82BA; while it has none, the fruit waits): 300 HP back (AntiRecv turning it) and 50 more mood (0x080127E8: to
254, a mood of 0 left), invulnerability for 420 ticks, or Full Synchro (a player's navi out of battle flag 0x40's mode
whose EXE5 emotion is one of the first four; 0x080127D6's setter). BoyBomb1–3 (the dimming handler's subtype 58:
EXE6's leftover controller, effect object 0x67, and bomb, attack object 0x3E, which no EXE6 chip spawns, with EXE5's
numbers: chips/boybomb/bomb): the bomb (NameID 0xD4, 150 HP) on the free panel ahead fades in for 16 ticks, then on a
panel it may stand on (0x080CD2BC, by its side) counts down 60 ticks blinking and blows up over the 3x3 around it
(no spark, against navis, hit modifier 3); anywhere else it breaks. Its Param1 0 (50 HP, the enemy area's blast, a
holder's record) is an AI's (0x08108274), no chip's. The controller's position is the dimming handler's registers
(the user's row, the element and the hook's own address).

RockCube (subtype 20: effect object 0x37, 0x080E4664, EXE6's code) and Wind and Fan (subtype 9: effect object 0x25,
0x080E329C; the fan, attack object 0x48, 0x080CE734; its gust, attack object 0x49, 0x080CEA0C) share EXE6's modules
through content/exelib (rockcube/cube, wind/controller, wind/fan: makers taking a game's look, EXE6's wrappers
keeping their APIs; the gust is the patch cards' shared one, gust/init); their controllers stand at the dimming handler's registers as BoyBomb's does. EXE5's
fan: sprite 04-0A (`fan`), NameIDs 0xD6 and 0xD7 (actor records: versions 3 and 4, AI index 0x21; their field-object
looks, 0x0801D6B0, the fan in palettes 0 and 1); it takes hits as EXE6's (0x08017984: the push keeps the damage, after
EXE5's lava burn); leaving (0x080CE90E), any removal (the flag word's 0x10000) is a puff and anything else breaks it,
with no blinking out and no absorbing; Fan's gust (0x080CE96C) starts on the far column alone, none on a row whose
far column has another obstacle (EXE6's `sub_80CD236` walks toward the fan). Its actions are EXE6's numbers (EXE5's
table, 0x080CE810, has six shared entries: rising and blowing are EXE5's 6 and 7). Its gust is EXE5's (`exe5:gust`,
navis/megaman/gust, which the patch cards' Vacuum blows too): sprite 0C-2E, any first parameter pushes weaker (hit
modifier 4, 0x080CEAA2; EXE6's only a pull), no hit stops it (0x080CEB30 tests the hit flags against 0; EXE6's against a
wind stopper's 0x800000), and a side of 2 reads 0x080CEBA4's code. The chip lab's
chips/0x054-fan/far-column (a RockCube pushed by AirShot to the far column), chips/0x054-fan/cosmoman (CosmoMan takes
the fan: the puff) and chips/0x053-wind/broken (the opponent's Cannon breaks it) record the differences.

Voltz1–3 (action 0x1A's instant effect 16, 0x080D88CE; EXE5's own: chips/voltz/voltz; EXE6's attack 0x90 is a virus's,
`sub_80D7068`, unported): a sensor (attack object 0x90, 0x080D86E8; NameID 0xE4, 1 HP, its side's field object of
class 1, sprite 04-17 `voltz`) on the panel three ahead drops from 160 pixels, 4 a tick, its collision on (a thrown
body against navis, hit modifier 3, its element's spark) and its panel reserved for the last 16 pixels; anything its
collision meets ends it at once, and evicted (its HP 0) it leaves a puff. Landed, 10 ticks; then its second
animation, the thunder ball's sound and a shock (attack object 0x92, 0x080D8AF0, sprite 10-11) on each panel beside
it that isn't its side's (up, down, behind, ahead: 0x080D8944), each going on its way to the next such panel after the
record's third parameter (20 ticks) and lasting 5 more; 30 ticks later it blinks out over 30. Its palette is its level
(the record's first parameter) times 4; its drop sound is 0xEA (`voltz-drop`). In battle flag 0x40's mode it marks its
panel as it comes and lands (effect 0x83, lib/navi_chips/marker). The chip lab's chips/0x04e-voltz1/chain (from the
back column: a shock goes on to the back column) and evicted (a second Voltz evicts the first) record the rest.

VDoll (action 0x12's subtype 8: EXE5's throw, holding the bomb, the attachment table's row 4) lobs EXE6's doll
(attack object 0x7A, 0x080D5618; content/exelib/vdoll/doll, a maker taking a game's look) three panels ahead. EXE5's
differs: from the start a body of no side that anything reaches, hit modifier 3 (EXE6's its side's object, hit
modifier 1, until it sets down; EXE5's setting down, its action 6, does nothing); a hit of types 0x0C800002 breaks it
as damage 0 (0x080D56D4: a puff, no curse); its leaving (0x080D57F0) has no blinking out or absorbing, and its curse
starts with a puff 12 pixels up if a chip removed it, else effect 0x24 16 up and sound 0x107 (EXE6's code for that
is unreachable); NameID 0xE2 (version 3, AI index 0x21). EXE5's curse (effect object 0x4E, 0x080E6110, EXE5's own:
chips/vdoll/curse) marks one panel, at random (an RNG2 draw, 0x080E625E), of those the other side's combatants stand
on, keeping the doll's aim when there is none (a column of 0 ends it), where EXE6's marks every combatant; it hits
that panel with EXE5's row 0x17 (a thrown piercing break) and hit modifier 1 (EXE6's the curse row and 3), and its
occupant test (0x080E7356) reads EXE6's table by the side unscaled (side 1: 0x00002000). Its telop names EXE5's Curse
(chip 0x174). The sparkles (effect object 0x11) are EXE6's code (content/exelib/vdoll/sparkles). The chip lab's
chips/0x067-vdoll/cursed records the curse (its user's buster hurts the landed doll). The doll's own actions store
EXE6's numbers until the obstacle framework's per-game numbering (exe5-obstacles-numbering's `obstacle.action_byte`)
lands: EXE5 stores its setting down as 6, so the recordings in which the doll lands differ at its action byte.

**EXE5's obstacle pushes** (the obstacle framework, by the obstacle's own game's `push_reading`): EXE5 keeps a
collision's hit flags only by the other collision's flip (+0x6C, +0x70: the hit registration 0x080169C8 to
0x08016A68; the engine's `hit_flags_by_flip` beside the union it reads as EXE5's +0x68); its push on any hit
(0x08017AD8, from 0x08017A78) reads them: one side's hits by unflipped hitters alone mark the unflipped hitters'
modifier byte (+0x18), by flipped ones the other (+0x19), and the final modifier. Its push vector (0x0800D4B0) takes
the pusher as side 0 when side 0's hits alone pushed (none when both did), else side 1, and the first of bits 2 to 5
of +0x18, else of +0x19 reversed, with a fifth row of nothing (EXE6's reads past its table there). Its slide
(0x08014894) keeps no bounds, and both its pushes go 8 pixels a tick in depth (0x08014730: the reactions section's
`slide_speed`). The chip lab's chips/0x055-boybomb1/pushed and airshot record a buster's and an AirShot's
knockback; RedFrut's broken and eaten recordings (side 0's buster, side 1's after side 0's Cannon) its gifts.

**EXE5's Full Synchro aura and guard** (found with RedFrut3, then built). The aura is actor object 0x5E (0x080C45E0):
the role `sprites.full_synchro_aura` is EXE5's sprite 14-16 (`full-synchro-aura`), and the engine's aura, by the
game's `effects.full_synchro_aura` rules (and the hit test's reading of the flag 0x80000000), takes EXE5's ways: its sprite steps while paused (0x080C45E0), but once the fight is on it
stops running while paused (it clears its header's run-while-paused bit, 0x080C4648), it keeps the animation it
started with (the navi's actor record's AI index, 0x0800D1C0), and it hides while its navi is bubbled too
(0x80000004); like EXE6's it frees itself as Full Synchro ends. Its spawner (0x0801100C) allows AI indexes to 12. The
chip lab's chips/0x044-redfrut3/broken, chips/0x004-airshot/counter (a counter hit's Full Synchro, 0x08016FDC) and
counter-paused (a pause through it) record it. EXE5's guard (0x080169B8, part of EXE5's hit test, 0x0801691C: the
reactions section's `hit_test`, which exe5-port-6 made meanwhile) breaks on a hit of types 0x1002 whatever its 0x4000
(EXE6's: 0x0002, or 0x1002 with 0x4000) and marks a guarded direction unless the hit has 0x0C004000 (EXE6's
0x0C005000). No recording can show it: EXE5's own types with 0x1000 all have 0x4000, so only another game's attack in
an EXE5 arena breaks a guard that EXE6's would hold; and the direction mask differs only for 0x1000, which EXE5's guard
never holds. A unit test (`collision::tests`) shows the first under that rule.

### 15.12 The content's layout (as built)

content/exe5 is laid out as content/exe6 is (content-model-v2.md §4.1; the user, 2026-10-03: "you should consolidate
the chips together where appropriate and move colocate objects with those chips, where appropriate like what bn6
does"). The layout was computed from the content and the modules
moved there (git mv, every `require` rewired, no id changed); the verification workspace's `tools/exe5/gen_content.py` finds each chip's
definition by its id wherever it is. The move changed no recording's replay (§15.5's report the same, recording by
recording).

- **Series files.** A series' chips are one module, chips/<series>/chips.luau: each chip a `local` with its own
  comment above it, the requires once at the top, the module returning the chips by key
  (`require("../cannon/chips").hicannon`), as EXE6's chips/cannon/init.luau. The series are EXE6's where EXE6 has the
  same chips (barrier, batcan, cannon, colorpt with DblPoint, energbom with MegEnBom, firehit, grabbnsh with
  GrabRvng, gundels, recov, tankcan, timebom with TimeBom+, tornado with Static, vulcan with SuprVulc; wind with
  Fan and gigacan once they have uses); each navi chip with its SP and DS (blizman, cloudman, colonel, cosmoman,
  django, gridman, gyroman, knightmn, larkman, magnetmn, meddy, napalmmn, numbrman, protoman, roll, serchman,
  shademan, shadoman, tmhwkman, toadman); numbered levels (astroid, cactbal, crsshld, drilarm, elcreel, geddon,
  guard, infvulc, katana, moonbld, mrkcan, pulsar, quake, sidebub, skully, spshake, widesht, woodnos; airspin,
  aqwhirl, boxer and the rest as they are ported); a Program Advance whose ingredients are one series'; and EXE5's
  own two: crakout (CrakOut, DublCrak, TripCrak, as EXE6's CrakShot series) and cannmode (the Liberation Missions'
  mode chips: CannMode, CannBall's, SwrdMode, YoYoMode, DrilMode). A chip in no series keeps
  chips/<key>/chip.luau.
- **A chip without a use keeps its own folder** until the port gives it one: the loader leaves it out by its
  folder, with every chip folder that requires one of its modules (content-model-v2.md §7.3), so it can't be in a
  file with chips that play. It joins its series when it gets its use.
- **Kinds with their owners** (§4.1's rules 1 to 5). A kind one chip or series uses is in its folder
  (chips/vulcan/vulcan, chips/timebom/timebom, chips/widesht/wave and variants, chips/yoyo/yoyo). One with a
  natural owner and borrowers is the owner's: the dark chips borrow their light chip's (DarkDril
  chips/drilarm/drill, DarkThnd chips/thunder/ball, DarkTorn chips/tornado/tornado, DrkLance chips/lance/lance,
  DarkCirc chips/circgun/circgun, DarkWide chips/widesht/wave), the InfVulcs chips/vulcan/vulcan, PoisPhar
  chips/anubis/anubis, ParaBom and ResetBom chips/crakbom/crakbom, MudWave and RedWave chips/wavepit/wavepit. A
  family's is in its lib/ folder, its builder with it: lib/bombs (bombs, seed), lib/guard (guard, the Guard chips'
  shock wave), lib/traps (traps, the Anti traps' anti_trap), lib/instruments (instrument), lib/navi_chips (the
  navi chips' throw marker). The souls module's are rules/souls' (shade, image); a soul's own are in its form's
  folder (§15.8).
- **objects/** keeps what several families share (attachment, bullet, flying-shot, panel-bursts, panel-changer,
  projectile with its variants, rising-bubble), and six modules whose `define.kind` keys name no owner, which
  can't move without new ids (the content check keys a kind in an owner's folder under the owner): capsule
  (Meddy's), dice (NumberMan's), gyro-bomb, napalm-bomb, crack (CrakOut's) and meteors (the shower and its
  marker). Re-keyed (`exe5:meddy/capsule`, ..., with compat's kinds.toml), the next run moves them.

| | before | after |
|---|---|---|
| modules | 535 | 423 |
| chips/ folders | 333 (a chip each) | 221: 53 series' (165 chips), 168 chips' own (68 without a use yet) |
| objects/ folders | 53 | 13 |
| lib/ | 18 modules | 15 modules, 5 family folders |

### 15.13 EXE5's NaviCust (as built)

EXE5's compile (0x0813FA10; 0x0813F97C runs it, then the patch cards, 0x08138214) is EXE6's routine for routine, so the
two share it: content/exelib/navicust/compile.luau, each game's navicust module passing its board, its bugs and its
quirks (`NaviCustGame`). EXE6's compile is unchanged (trace-tests' navicust: Tango's four saves and the lab's 1274
NaviCusts). EXE5's, content/exe5/rules/navicust:

- **The board** (rules/navicust/board.luau, which the compile reads and the editor's grid reads as data): EXE5's 5x5 grid is the middle of the engine's
  7x7, the command line EXE5's row 2 (the engine's 3), no frame, any cell of it a neighbor's. A part's 5x5 shapes (the
  part table's +8, +0xC) sit in the middle of the definition's 7x7. The board on the grid is the save's ExpMemry's
  (key item 0x61's count, a byte at the save image's 0x3DB0 + 0x61: the toolkit's +0x50, which 0x0803C120 reads):
  4x4 with none, 5x4 with one, 5x5 with two, from the grid's top left, the command line the third row of each. Only
  the NaviCust screen reads the count, as it opens (0x08132928, into its +0x0F): it picks the board's mask (0x0813F138:
  three 15x15 masks, 0x0813EA34, 0x0813EB15 and 0x0813EBF6, a cell at (x + 5, y + 5); the same bytes in the four
  ROMs, their tables at BRBE 0x0813F148, BRKE 0x0813F230, BRBJ 0x0813ECD4 and BRKJ 0x0813EDBC), its background
  (0x08132FB4) and the cursor's bounds (0x0813324C and 0x081333B0: 4 by 4, 5 by 4, 5 by 5). Placing a part asks the
  mask (0x0813F250: every cell the part covers is one of the board's, so nothing juts off it), then the grid
  (0x0813F2A4: over no other part). The compile (0x0813FA10) reads neither the count nor a mask. As built: the
  section's three `boards` by `NaviCust::expansions`, as EXE6's (the match file's `[side.navicust] expansions`, the
  largest when it says none; nettai-match's check of each program against the board; the editor's board picker);
  exe5-compat reads the count (`Save::expansions`) and the EXE5 save import sets it on the side's NaviCust. Every save
  on hand has both ExpMemry.
- **The programs** (content/exe5/navicust/, 47; compat/navicust.toml by number and colored variant), written by the
  verification workspace's tools/exe5/gen_navicust.py from the part table (BRBE 0x0813D540, BRKE 0x0813D628, BRBJ
  0x0813D0CC, BRKJ 0x0813D1B4; 16 bytes a part id, program n's four variants at 4n..4n+3: +0 the exclusive group,
  +1 a plus part, +3 the color, +4 the bug group). Unlike EXE6's a program's colored variants needn't come first
  (SprArmr is only its fourth), so compat lists each program's colored variants; the Japanese ROMs' MegFldr1 comes in
  pink too (part 17). The effects are the handlers of 0x0813FB44 (the shared constructors, @exelib/navicust/effects;
  Shield, Reflect and AntiDmg are MegaMan's B+Back weapons 0x1F, 0x61 and 0x21, navis/megaman/weapons/guards.luau;
  SoulT+1 adds a soul turn, NaviStats +0x32, the stat `soul_turn_bonus`, at most 6).
- **The counts** (0x0813F310): EXE6's but for HubBatc (program 27), which counts its own bug once more (0x0813F5A8),
  and no frame. Five colors bring the status bug, six the stronger one.
- **The reset** (0x08133DBC) keeps the mood, the light/dark value, +0x21, +0x22, the base HP, the soul, the folder and
  its Regular chips, the Regular memory and the HP; the rest is the navi's fresh stats (0x08010C00 under MegaMan's row
  of 0x0801D55F: in content, rules/fresh_stats.luau and his definition's `fresh` and `weapons`).
- **The bugs** (0x08140008, its table 0x08140054 by group and level): steps astray (+0x31), the emotion swing
  (+0x24), the panel trail (+0x12 = 3, +0x13 = 2, 4 or 8), the custom screen's damage (+0x54: 20, 40, 80), encounters
  (+0x28), drops (+0x26), the buster (+0x14 = 4, +0x15 = 2), no supports (+0x0D = 0xFF), the HP bug (+0x16 = 3), Hub's
  (0x08140248: the maximum the base and half the HP programs, the HP no more than it), the statuses (+0x1A = 9, 10).
  BugStop (+0x1F) stops them (0x0813FA48; the patch cards can stop or keep them, 0x08137A30: §15.14). A bug sets
  flag 0x10C1, the emotion window's glitch when the save has no patch cards (§15.14).
- **The HP** (0x0803C13C): in the real world the maximum again and the HP with it; in the cyberworld (the save's
  area, 0x02002944, from 0x80, or event flag 0x10B2) nothing, so the HP is the save's and the maximum the effects'
  (0x0803C1CC), Hub's halving standing. Tango's finished Team ProtoMan light save is in the cyberworld (area 0x8C,
  HP 850 of 1000). **The engine's rules have the real world's ending alone**, and a side states no world: a link
  battle starts every navi at its maximum whatever the block's HP says, and the maximum is the same in both endings
  on every board the game builds (below). What the other ending wrote is in a recording's setup still (the
  console's `cyberworld`, its block's HP), and verify's compile test leaves those bytes out for such a console.
  - *Hub's halving is reached by no board the game builds.* The part table's shapes (16 bytes a part: +8 the shape,
    +0xC the compressed one; the four ROMs agree): HP+50 a straight three, HP+100 a square of four, HP+200 six (two
    over four), HP+300 seven (four over three), HP+400 two by four, HP+500 two by five, each in white, pink and
    yellow, and each one's compressed shape the same 25 bytes as its shape: no HP program compresses. HubBatc is
    thirteen cells (rows of one, three, five, three and one; its compressed shape the same), BugStop a straight
    three, two compressed. HubBatc has no place on the 4x4 or the 5x4 board and one on the 5x5, the middle: the
    whole command line is its own, and the twelve cells left are four corners of three in an L. No HP program fits
    one, turned or not, so the sum the bug halves is zero on every board a save can hold; BugStop fits a corner
    only compressed, off the command line, where it stops nothing. (nettai-match's check refuses a part over
    another, as the original's placing does, 0x0813F2A4; its test `exe5s_hubbatc_shares_a_board_with_no_hp_program`
    walks every placement.) The bug's routine (0x08140248) takes in nothing else: the maximum is the base (+0x3E,
    whole) and half of one halfword, the HP programs' sum (the first of the block the toolkit's +0x68 points to),
    which only the compile's clear (0x0813FA74), the NaviCust screen's (0x0813EEC6) and the six HP programs'
    handlers (0x0813FF9C to 0x0813FFF4) write; the patch cards work on the maximum afterwards.
  - *What the original makes of it, on a board laid by a poke*: HubBatc in the middle (part 108 at (2, 2)) and
    HP+500 in pink over its top rows (part 189 at (2, 1)), off the command line, which the compile takes (it reads
    the list and checks nothing). Both sides, side 0's patch cards taken off; the block as the compile left it,
    and what a link battle's first frame has:

    | Console | Block: base, HP, maximum | The battle starts at |
    |---|---|---|
    | Team ProtoMan light save, in the cyberworld (area 0x8C) | 1000, 850, 1250 | 1250 of 1250 |
    | Team Colonel dark save, in the real world | 997, 1497, 1497 | 1497 of 1497 |
    | Team ProtoMan dark save, in the real world | 997, 1497, 1497 | 1497 of 1497 |
    | Team ProtoMan dark save, event flag 0x10B2 raised (image 0x2C0E, 0x08 to 0x28) | 997, 1247, 1247 | 1247 of 1247 |
    | Team ProtoMan light save, its area byte (image 0x2944) poked to 0 | 1000, 1500, 1500 | 1500 of 1500 |
    | The light save, BugStop besides (part 112, compressed, at (0, 2): over the command line's first two cells) | 1000, 850, 1500 | 1500 of 1500 |
    | The light save, HP+500 alone (a board the game builds) | 1000, 850, 1500 | 1500 of 1500 |
    | The light save, HubBatc alone (a board the game builds) | 1000, 850, 1000 | 1000 of 1000 |

    So the original has the two endings as read here: where the compile leaves the HP, by the area or by the flag
    on one and the same save, the halving stands (the base and 250), and in the real world the maximum is whole (the
    base and 500). A link battle starts each navi at its maximum either way (the engine's `init_hp`, EXE6's
    `sub_80141C8`: the block's HP only in a battle that keeps HP, its effects' bit 4 clear; the recorded link
    battles' effects are 0x0E8C), so the block's HP word, the one other thing the two endings leave different, is
    read by no link battle: on every board the game builds, the two endings start a link battle alike. Replayed
    with each console's fact as recorded, the engine matched all eight recordings on every frame and their blocks
    byte for byte. They are kept, unreplayed, with their scenario files in the verification workspace's
    data/staged/exe5-hubbatc-overlapped-2026-10-05: an engine whose compile has the real-world ending alone can't
    replay the three whose console keeps the halving (1250 twice, 1247). The library has the boards that can be
    built (navicust-compile/hubbatc-hp-apart, hubbatc-bugstop).
- **Compression** is an event flag (0x1EC0 + the part id, which 0x0813EEFC tests), not the list's +5 (the editor's
  mark): every finished save on hand has the flags of every program that compresses set.
- The save's list: 25 parts of 8 bytes at 0x02004D6C (+0 the part id, +2 the column, +3 the row, +4 the quarter
  turns), the grid at 0x02004D48; exe5-compat's `trace::navicust` reads it.

exe5-compat now reads the bug bytes the compile writes (+0x12 to +0x16, +0x1A, +0x24, +0x26, +0x28, +0x31, +0x54) at
EXE6's offsets into the engine's stats, which a replay's setup carries.

**Checked:** trace-tests' exe5_navicust compiles Tango's EXE5 saves (the finished ones in ~/Documents/Tango/saves and
the netplay templates; with their patch cards since §15.14) and every side of the EXE5 lab's
scenarios against its recording's setup, in every byte the engine's compile can write (59 NaviCusts of 164 programs:
library-exe5/navicust-compile, the finished saves' own NaviCusts with their cards taken off, both sides of each). The
programs in battle (library-exe5/navicust, 31 scenarios) replay on every frame but Rush's, which waits for EXE5's
supports (the role `kinds.support`).

What the programs in battle brought into the engine:

- EXE5's charged shot (0x080EC9F0) waits only in the operation battle, marking the panel in front; else it fires
  on its first tick (the shared charged shot's `waits`).
- EXE5's hit test (0x0801691C, §15.3 item 12): a FloatShoe body is hit (EXE6's needs the hitter's 0x80 self bit, which
  EXE5's collision types lack), and its raw channel (0x08017494) has no FloatShoe test either, so a barrier under
  FloatShoe wears.

### 15.14 EXE5's patch cards (as built)

EXE5's patch cards (its Modification Cards, 改造カード) work as EXE6's: after the NaviCust's compile, 0x0813F97C runs
the cards' routine (0x08138214) and then the HP rule. The application is EXE6's, so the two share it
(content/exelib/patch_cards/apply.luau and the effects' constructors, effects.luau; each game's
rules/patch_cards/cards.luau gives its kinds' order, its choices and tables: `PatchCardsGame`). EXE6's is unchanged
(the JP lab's card traces). EXE5's, content/exe5/rules/patch_cards:

- **The cards** (content/exe5/patch_cards/, 112; compat/patch-cards.toml by number, card 111 by version), written by the
  verification workspace's tools/exe5/gen_patch_cards.py (gen_content.py) from the card table (BRBE 0x08138874,
  BRKE 0x0813895C, BRBJ 0x0813842C, BRKJ 0x08138514: u16 offsets by card number, 111 cards, three-byte entries: the
  effect's number, its value and whether the card shows it as a bug; the first the MB, number 0x8B), their names
  from the name archive (LZ77, a four-byte header; the locales' `[patch-cards]`). The four tables are the same but
  for card 111, Bass-Cross MegaMan, each team's own: Team ProtoMan's HP+20, SprArmr, TriBustr, B↓Sbustr; Team
  Colonel's HP+20%, FlotShoe, TriBustr, B↓HelzR; both NormBody and a MegFld-2 bug. Its definitions are
  `bass-cross-megaman-protoman` and `bass-cross-megaman-colonel`; exe5-compat's `trace::patch_cards` reads a save's
  list (0x79A0 the count, 0x79D0 the list: a card a byte, bit 7 switched off) by the save's version.
- **The effects** (the handlers at 0x08137BC8, 0x00 HP+ to 0x89 BugStop: the kinds' order): EXE6's kinds and EXE5's
  own: SoulTm+ and SoulTm- (+0x32, signed, clamped to -2..6: the stat `soul_turn_bonus`), the random encounters'
  element (+0x27, OilBody to Search), SneakRun, Millions, AutoHeal and MegVirus (bytes no battle reads), and Hub
  Style, which card 111 gives switched on (0x08137A58 finds it in the list: no effect number names it). The slots
  are seeded from the stats (0x08137B56); the copy is 0x081382B8's 39 routines, some through tables: the B buttons'
  weapons (0x1D to 0x23: routines 0x62, 0x64, 0x53, 0x57, 0x61, 0x00, 0x65), the buster-shot programs' rows (7,
  0x14, 0xE, 0xD, 8, 0), the charged shots' weapons (0x2A to 0x49, 32 routines), the charged-shot programs' rows
  (0x15, 0x16, 9, 5, 0x17 to 0x1E, 6), the first barriers (types 1, 5, 7, 8, 9, none), the B+Back specials (0x1F,
  0x6A, 0x6B, 0x61, 0x21, none), the gauges, the panel trail's bytes (EXE5's panel numbers: the panels section's
  `numbers`), the hit statuses. BugStop (a card's, at 1) holds back the bug slots and, asked first by the
  NaviCust's compile (0x0813FA48 asks 0x08137A30), stops or keeps the NaviCust's bugs (`NaviCustGame.bug_stop`).
- **The HP** (0x0813F97C, after the cards): in the real world the maximum; in the cyberworld no more than it. The
  rules have the real world's (§15.13).
- **The emotion window's glitch** (0x0801AF14 through 0x0813F650, as the window starts; not in battle modes 1 to
  4): with cards in the save's list (its count, switched on or not) the cards' flag 0x10C4, which the routine sets
  when the stats after the cards have a bug (0x081384D8: the NaviCust's bug bytes, the encounters' and drops' bugs,
  no supports); without, the NaviCust's flag 0x10C1, which its bugs' routine sets when a bug applies (0x08140040).
  Each console reads its own save's. the rules' `glitch`, and the patch cards module with cards; a
  recording's setup carries both consoles' (`emotion_window_glitches`, oracle-trace's `exe5_emotion_window_glitch`).
  No setup gives the engine the flag: the rules make it, from a NaviCust's compile or, for a side without one
  (whose stats are as a compile left them), from the NaviCust bugs in the stats. A recording that carries its
  consoles' NaviCusts and cards (§10's setup line) is replayed by compiling them, and the compile's flag is checked
  against the console's: HubBatc's bug halves the HP programs and writes no bug stat, so only the compile knows it.
  A recording without them has the flag from its stats alone; one whose save had the flag with no bug stat
  (`navicust/hubbatc` as first recorded) would differ from the first flicker on (its console's RNG1). No recording
  of the lab is one since the NaviCust and patch-card scenarios were recorded again with their NaviCusts, so the
  replay names no such difference.
- **The weapons** (navis/megaman/weapons): the routines that load a chip (0x0800FE78: chips.luau, MettGuard's and
  CrsShld's B+Back waiting 40 ticks, Ccann's TankCan1 not cracking), the card Shield (0x62, guards.luau), TriBustr
  (0x65, the buster's routine), ChrgS (0x63, the charged shot without the draw, the program always: its 0, the
  table's plain row, is none), HeatS and BubSht (0x4A, 0x56: the Spreaders' action in its variants 1 and 2, the
  bullet leaving after the count's 10, 0x080EC57C; the bullet's rows 8 and 4), Invis and Vacuum (0x47, 0x4C: the
  instant chips' action with effects 2 and 25, the second TenguCross's wind; EXE5's gust, attack object #0x49, is
  EXE6's code, shared in content/exelib/gust: nothing stops it, an unseen one pushes weaker), FireAm (0x4E, action
  0x57: ElemRage's flames, EXE5's own action; EXE6's 0x57 is another), ZapRng (0x42, action 0x32: a paralyzing ring,
  attack object #0xC4, EXE6's `sub_80C51CC` code, which no EXE6 content spawns), Sbustr (0x68: the rapid buster,
  shared in content/exelib/megaman) and HelzR (0x69: AirSpin's action in its variant 2, the seeking whirlwind,
  shared in content/exelib/airspin/whirl; EXE6's patch card bass-cross-megaman fires it too). Invis's timer takes
  its high byte from the attack parameters' second byte, which the routine leaves as the last attack left it: the
  engine keeps no attack parameters, so 0.
- **Hub Style** (NaviStats +0x4C, the stat `hub_style`: a byte, 1 from Team ProtoMan's card, 2 from Team Colonel's,
  0x081382FC): MegaMan's buster in his base form fires the spread (0x0800F522: the shot's variant 1, row 0x12, its
  own flash, the attachment table's row 0x39), his arm by the value (0x080EBABE: 1 animation 13, 2 the chaos arm),
  his attachments sit a pixel off (0x080B9AB8: the EXE5 attachment kind's `lifted`; EXE5's follow adds the unsigned
  byte where EXE6's subtracts the signed one), his base form wears its shade (body overlay row 5, 0c-58: 0x0800EE1C,
  the form's `put_on`, which also runs as a player's init ends, MegaMan's record's init hook being that routine;
  EXE5's init has no starting form's overlay), GigaCan leaves a third afterimage (0x080EC224), the palette is
  `hub_style * 5 + 20` and the faces move (§15.10), and in a link battle the enemy names show the navi's variant
  name (0x0801AE3A, presentation: with battle effect 8, the other side's NameID 0x180 when its +0x53 is set, else
  0xEA, BCMegaMn, in Hub Style; +0x53 is the reload's, 0x08135968, 1 exactly when 0x08137A58 finds no Hub Style:
  the rules' `round_setup`, `battle.set_name_variant`, the locales' `variant_name`; checked against
  mGBA by verification's library-exe5 custom/hub-name). Battle effect 0x40000 names every entry 0xE6 (ChaosLrd): no
  stage of EXE5's content has it, not built. A deliberate difference: the routine's names start as side 1's actors'
  NameIDs (0x080091A2 reads the battle state's +0x90 list, whichever side the console is), and the link battle's
  overrides above name only a MegaMan enemy. So the original's side-1 console facing a team navi on side 0 (a state
  the game never reaches: a link battle's navis are both MegaMan) names its own navi for the custom screen's first
  frames, where nettai names the enemy's, as a player expects (seen with the chip lab's swapped base, traced on
  side 1; the team navis' from-the-left scenarios are traced on the navi's own console, where the two agree).
- The emotion window's start keeps the glitch outside EXE5's battle modes 1 to 4; the engine's start keeps EXE6's rule
  (no random battles, not modes 1 to 5 and 8), the same for a netbattle.

**Checked:** trace-tests' exe5_navicust applies the cards with the compile: Tango's saves (the three with cards; the
finished Team ProtoMan Light 2's stats are from before its cards: written before a reload applied them, its stats
the NaviCust's alone) and the EXE5 lab's sides with their cards (library-exe5/patch-cards-stats, 18 scenarios: every
card on some side, a later card's choice over an earlier one's, amounts adding up and clamped, BugStop, cards
switched off, Hub Style; 34 sides, 181 cards), in every byte the engine's rules write. The cards in battle
(library-exe5/patch-cards, 61 scenarios: every charged shot, B button and B+Back special a card gives, the programs,
the first barriers, Hub Style on both teams) and the stats scenarios replay on every frame, with RNG1.

The emotion window's glitch came with them: the navicust recordings line-g, bug-support and hubbatc (a bug outside
the counted bytes: supports, Hub) stopped on RNG1 at the first check, re-recorded with the glitches in their setups.
bug-hp's stop was its last frame: a flicker's draw on the frame a recording ends has no frame left to agree, which
exe5-compat's RNG1 comparison now leaves.

The first barriers' visuals on a round's first tick: the other side's navi inits first (its first barrier's visual
asking at once whether the local navi is blind, 0x0800CDF4), before the local navi has collision data, and the game
reads its status word through the null pointer, from the BIOS. EXE6's read gives the object spawn's CpuSet opcode,
0xE3A02004, with the blind bit (the visual hidden for a tick); EXE5's console gives an interrupt's, 0xE55EC002 (read
at 0x0800CE18 with a scratch trap in the chip lab), without it: the visual shows at once. The status section's
`missing_collision_status` (the engine's `Rules::missing_collision_status`, which each game states) says which; the
replays' full object comparison caught it (navicust-compile/bug-colors, patch-cards-stats/bugstop, cards-02, cards-03,
cards-08, clamps-hp: both sides with first barriers).

### 15.15 EXE5's supports (as built)

The NaviCust supports, Rush, Beat and Tango, are EXE6's code in EXE5 (docs/engine/chips.md §2.10), so the two share
them: content/exelib/supports (`controller`, `rush`, `beat`, `tango`, `heal`), each made of a game's look
(`SupportsLook`, `RushLook`, `BeatLook`, `TangoLook`, `TangoHealLook`). EXE6's lib/supports and EXE5's
content/exe5/lib/supports make them; EXE5's roles name the controller (`kinds.support`) and the telops' chips
(`chips.rush`, `beat`, `tango`).

- **Where:** the triggers are EXE6's routines at EXE5's addresses (Tango 0x0800E3B6, from the idle state at
  0x080F02C8; Beat 0x0800E418 and Rush 0x0800E498, from a chip's use at 0x080F0452: the engine's, as EXE6's), each
  passing its telop's chip in r7 (0x17B, 0x17A, 0x179, EXE6's numbers: chips past the library, written by
  gen_content.py's `SUPPORT_CHIPS`, their use the record's, the Cannon's, which nothing starts). The controller is
  effect object #0x74 (0x080E8F50; EXE6's #0x79), its spawner 0x080E906A, its phases EXE6's (0x080E8FB8 to
  0x080E903A) and its spawners' table 0x080E9010. Rush, Beat and Tango are actor objects #0x4B to #0x4D
  (0x080C2214, 0x080C24C8, 0x080C2714; sprites 0c-48, 0c-4b, 0c-4c, named `rush`, `beat`, `tango` by place), the
  heal attack object #0x9F (0x080DA6AC; EXE6's #0xC7; sprite 0c-4d, `tango-heal`). The sounds and the effect rows
  (0x14, 0x15, 6) are EXE6's numbers.
- **What differs:** EXE5's Rush trigger hands the controller no chip (0x0800E498: `movs r4, #0`, where EXE6 passes
  the chip's number shifted into the third and fourth parameters), and the controller's phase 4 (0x080E8FE4)
  doesn't load its parameters into r4 for the spawner; EXE5's bite (0x080C23BE) has no check for the second
  WhiCapsl (0x17E, which EXE5 doesn't have): the opponent's hand always moves on, and without the opponent's navi
  its branch pops what it pushed and moves no hand (EXE6's skips the pop). So EXE5's Rush keeps no chip (his look's
  `spared` is nil, his kind no `eaten` state; the engine sets the controller's `eaten` for every game, which EXE5's
  never hands on). The inits load the sprites without EXE6's `sprite_decompress`. The heal raises EXE5's barrier type
  5 (0x080174DA: EXE5's barrier rows, lib/barriers) and EXE5's barrier visual. EXE5's chips Rush cancels: Invisibl
  (0x085, its record's +0x16 bit 1) alone; EXE5's Invis card weapon doesn't ask Rush (0x0800FC3A), where EXE6's
  patch card invisibility does.

**Checked:** the EXE5 lab's navicust/rush, beat and tango (a NaviCust part each, side 0 hosting: Rush bites the
other side's Invisibl, Beat takes its SuprVulc, Tango heals at a quarter and her barrier takes the next Cannons;
each support once, the second chip going through) and navicust/bug-support, and patch-cards/supports and
supports-side1 (Tora's Tactics, all three supports from the card, on either side), replay on every frame. The
earlier navicust/beat and tango recordings never brought their supports: their folders named chips with codes the
chips don't have (Roll `*`, M-Cannon `*`), which EXE5 makes the invalid chip; the scenarios now name legal codes,
SuprVulc rather than Roll (Roll also wants a light MegaMan), and Tango's navi starts at 80 HP (side 1's M-Cannon
takes only 60 from side 0's navi in these saves, where side 0's takes 120 from side 1's: not looked into). Not
reached: a Giga chip for Beat, Rush without the opponent's navi, a failed spawn.

### 15.16 The team navis (in progress: the framework, and the navis of the table below)

EXE5's players operate thirteen navis: MegaMan (navi 0) and the twelve team navis, six a version in its souls'
order (NaviStats +0x29, and a save's GameState +1, the navi the PET operates): Team ProtoMan's ProtoMan, GyroMan,
SearchMan, NapalmMan, MagnetMan and Meddy (1 to 6), Team Colonel's Colonel, ShadowMan, NumberMan, TomahawkMan,
KnightMan and ToadMan (7 to 12). A save holds seven NaviStats blocks (MegaMan's at save +0x52A8, then its version's
six, 0x60 bytes each: 0x0801165C by navi number). Every table below is the same in the four ROMs (each at its own
address; Team ProtoMan US's are given).

**Where the original lets one be operated.** The story (the Liberation missions and their areas) and the Battle
Chip Gate's operation battle (its menu, 0x0813C2AC, sets the navi with NaviStats +0x2A raised: battle flag 0x40). The
battle's init exchange (0x080098E0) sends the PET navi's block and the team navis' level; a plain NetBattle sets the
PET navi to MegaMan for the battle (0x08135936) and puts it back. **In nettai a side of an EXE5 match may operate
any of the twelve** (a match has no version; a side states its navi), in the netbattle's rules. Not built, and not
planned here: the operation battle with a gate navi (the gate's own HP table, 0x0802FD74), the gate chip's
mid-battle navi switch (§5), and the Liberation battles.

**The no-running message** (L on the custom screen where no one runs: 0x08023190, EXE6's `sub_8026EC8`; built
2026-10-05, after L was found to open an empty box). The screen's run state runs script 3 of the run dialog's
archive (0x08739868; Team Colonel's 0x0873AB24, the Japanese ROMs' 0x0875028C and 0x087516FC), whose first command
(`EF 2F`, thirteen scripts by the player's navi) leaves MegaMan the rest of script 3 and sends each team navi to
its own, 0x17 to 0x22. A message is `F5 00 n` (its speaker's portrait, the sprite list's category 0x20: 0x30
MegaMan, 0x34 ProtoMan, 0x40 to 0x4A the others in the navis' order), `E8 00`, the text, `E7 00`, `E6`: EXE6's
shape, timed by the engine's chatbox with two rules of EXE5's own (docs/engine/custom-screen.md §3.5: a command
runs on the last character's tick; the characters that move the mouth). The key routine (0x080247DC) takes L as
EXE6's does (not in battle mode 1), plays sound 0x7B, and the message is the console's own: the other console
shows nothing of it. In content: each navi's `run_message.portrait`, the words in `locales/en.toml` and `ja.toml`
(`tools/exe5/gen_content.py`, which holds the definitions' portraits to the scripts'). A team's ROM has a
placeholder for the portraits of four or five of the other team's navis; the pack takes each face from the ROM
that has it, and no console shows the other's (a save has a stats block for its own team's navis alone, and
chiplab refuses another). Recorded for MegaMan on both teams' consoles and each team navi on its own, with a copy
on Japanese consoles (the lab's `custom/run-message*`, `navis/<navi>/run-message*`); a Japanese console times the
message by its own words, as EXE6's does, so its frames while the message is up differ from the engine's, which
prints the Japanese in step with the content's English (docs/design/text-rendering.md §10).

**A navi's data** (content/exe5/navis/<navi>/init.luau, a `define.navi` without forms):

| What | Where | Notes |
|---|---|---|
| Battle sprite | 0x0800DA65 by navi (08-NN) | the pack's `protoman` … `toadman` |
| Element | 0x0800E634 | NapalmMan Fire, MagnetMan Elec, TomahawkMan Wood, ToadMan Aqua |
| Buster bonus | 0x0800F5AC | 1; TomahawkMan 3 |
| Move lag | 0x0801D462 (11 bytes a navi) | 4; KnightMan 10 |
| Banners | 0x080090D8 (win, 0x94 + 4 (n − 1)), 0x080090B0 (deleted, 0x60 + 4 (n − 1)) | `<navi>-win`, `<navi>-deleted` |
| Face | 0x08019724 (Team Colonel's 0x0801971C): the version's six pictures, two palettes each | the pack's navi mugshots, `<navi>` (0x80 + n − 1); the second palette in Full Synchro |
| Actor record | 0x08014C94, NameID 0x180 + n | version 0, a player, AI index n |
| Attach points | category 8's table, 0x08015C1C, row n | 30 points |
| Stats row | 0x0801D55F (16 bytes, read by 0x080111AA) | the HP, SuperArmor, FloatShoes, AirShoes, UnderShirt, the Mega and Giga levels, the four weapons, the B+Back special's damage (+0x48). The rest of a fresh block is the game's, the same for every navi (0x08010C00: rules/fresh_stats.luau) |
| Palette step | 0x0801D737 | the sprite's palettes go by it (0x0800DA98: 4 steps in Full Synchro, 1 while it can't charge): 1; NapalmMan 3, MagnetMan, Meddy, Colonel and TomahawkMan 2 |
| Own chip | 0x08025EA0 (a pair of the same chip and code) | below |
| Story HP | 0x0804F960 (20 bytes a navi) | below |

**Weapons.** The stats row names the weapon routines (0x0800F370's): every navi's buster is routine 0 (MegaMan's)
but GyroMan's (0x43) and NapalmMan's (0x44); the charged attacks ProtoMan 0x1E, GyroMan 0x28, SearchMan 0x3F,
MagnetMan 0x26, Meddy 0x3B, Colonel 0x46, ShadowMan 0x24, NumberMan 0x3D, TomahawkMan 0x3C, KnightMan 0x10, ToadMan
0x2E (NapalmMan none); B+Back ProtoMan 0x0B and ShadowMan 0x0D; the A-charge routines of NapalmMan, MagnetMan,
TomahawkMan and ToadMan (0x29 to 0x2C) are empty: their A charge is of their element's chips (0x0801090A: a chip of
their family, neither their own chip nor past 0x190, charges, and a charged one hits twice as hard, 0x080103D0). Each
navi's own actions are its state table's entries from 7 on (0x080EADA8 by navi number; GyroMan has a tick of his
own, 0x080EB1E8's).

**The A charge, in content.** Both routines test the soul (NaviStats +0x2C) and then the navi (+0x29): NapalmMan
(4) Fire, ToadMan (12) Aqua, MagnetMan (5) Elec, TomahawkMan (10) Wood. The chip's tests are the souls': a chip
under 0x190 of that family that is neither a dimming nor a dark chip (its flags' bits 0x01 and 0x20), damaging or
not; no level is read. So each of the four states `charged_chips = { family = ..., damaging = false, plain = true }`
(a navi's rule always says `damaging`: EXE6's link navis say true, with their `when`, from a level) and `charged_bonus =
{ damage = 0xFF }` (the chip's damage again, with the bonus's sound), and its `a_charge` weapon is the charged-chip
routine with its row of the charge table (navis/megaman/weapons/charged_chip's `routine`: 0x29 NapalmMan's, 0x2A
MagnetMan's, 0x2B TomahawkMan's, 0x2C ToadMan's). Where a form and its navi both state a `charged_bonus` the form's
is the one read, as the original tests the soul first; the navi's is read in a form that states none. (NapalmSoul's
charged Fire chips start its bomb's action, 0x08010442, by the soul alone: NapalmMan's are the chips themselves.)

**The level.** A team navi's attacks take their damage from the damage rows (0x0801D74F, seven entries a row,
0x0800EBC4) at the side's level: a word a side (0x0203C870) the init exchange sends, which 0x0800EBE0 counts from the
save's story flags, 0x300 on, up to 6. In content it is the rules' `level`, which the engine knows nothing
of: `exe5.navi_level(side)` (rules/api.luau) through lib/navi_level for a weapon, and `damage =
navi_level.row({ ... })` for an own chip (the chips' damage formulas 50 to 72, each a row: 0x0800EAF8), a function of
the side, which the round's setup asks once for each side (`Battle::given`).

**The own chips** (chips 0x191 to 0x1A6, past the library): StepSwrd B (ProtoMan), Airforce G (GyroMan), Satelity S
(SearchMan), Napalm N (NapalmMan), NSTackle M (MagnetMan), MeddyCap M (Meddy), C-Cannon C (Colonel), SplitUp S
(ShadowMan), NumTrap N (NumberMan), T-Swing T (TomahawkMan), KCrusher K (KnightMan), S-Melody T (ToadMan). The custom
screen offers a team navi its chip in slot 9 until it is picked once a battle (0x08023EFE; the hand builder's
0x08024E20 marks it), picking between its table's two entries, the same chip, with a draw of the console's RNG (the
navi's `own_chip_draws`). A chip's record names a routine of its own (+0x1F: 0x0800EC68's table, 0x0800EC80), which
the chip's use calls once the attack is loaded (0x080100CE): StepSwrd's sets the attack's charged byte, by which the
sword's action steps two panels ahead; KCrusher's, S-Melody's and C-Cannon's set attack variables. In content it is
the chip's `setup(navi)`. A team navi's own attacks carry a counter byte with bit 7 set (StepSwrd's 0x94): they make
no counter hit.

**The story's HP.** A team navi's HP is the story's, not its stats row's: a routine (0x08052E9C) sets every team
navi's base, current and maximum HP from a table (0x0804F960: a row a navi) by the story's progress, the count of
event flags set from 0x300 on (0 to 8), at a new game's start (0x08004C74) and from a map script as the story moves
on; the level above is the same count up to 6. The rows, by navi pair (the two versions' tables are the same with
their own navis): ProtoMan and Colonel 200, 300, 350, 400, 450, 500, 600, 700, 800; MagnetMan and KnightMan 400,
400, 500, 550, 600, 650, 700, 800, 900; GyroMan and ShadowMan 250, 250, 250, 300, 350, 400, 500, 600, 700; NapalmMan
and TomahawkMan 300, 300, 300, 300, 350, 400, 500, 600, 700; SearchMan and NumberMan 300, 300, 300, 300, 300, 350,
450, 550, 650; Meddy and ToadMan 300, 300, 300, 300, 300, 300, 400, 500, 600. In content it is the navi's `story`
(`hp` and `max_level`), which EXE5's save module reads as the round is set up (its HP, current, maximum and base,
at the side's `level`): a level below the last is the progress, and at the last the story is taken as done. A side
of a team navi states its level (a new one's is 0, a tool's statement); a save's import takes the save's level and,
where its version has the navi, the light/dark value of the navi's own block.

**The buster's recovery, found with Meddy.** A buster shot's recovery goes by the open panels ahead of the shooter
(0x0800D97A). EXE5 stops the count at one mask whichever side shoots (0x0F880080: bodies, neutral objects, blockers,
reservations and either side's own bodies), where EXE6's mask goes by the shooter's side and leaves its own bodies
out: a navi's own capsule sitting in its row shortens its next shot's recovery in EXE5. Each game's buster parts state
the mask (the `BusterLook`'s `closed`, lib/buster).

**What else goes by the navi.** No soul button (0x08023C90: the navi has no forms); a dark chip does nothing
(0x08010056) and the light and dark chips' check lets any chip through (0x08010118); the patch cards and the
NaviCust are MegaMan's (their rules pass a navi without forms). The Full Synchro aura's animation is its owner's AI
index (0x080C4608: the identity's `aura_anim`). What a navi holds goes by its AI index: the cannon's, the air
shooter's and the spreader's animations (0x080EC300, 0x080EC9B8, 0x080EC5B8: NapalmMan's, MagnetMan's and
KnightMan's own; the shared actions take each game's table, lib/navi_arms), the buster arm's (lib/arm), the sword's
blade (lib/swords), WindRack's rack and MrkCan's rows. The per-actor hooks (0x0800D30C's five tables, by actor type
and AI index) have entries for GyroMan (three), Colonel (one), KnightMan (three) and ToadMan (one), and the
animation-change hook (0x0800F13C) for GyroMan, NapalmMan, Colonel and KnightMan; GyroMan's move is a variant of its
own (0x0800F234, 0x0802D544).

**As built** (engine): a chip's `setup` hook and the `level` damage formula (since a function of the side the round's
setup asks, `Battle::given`); a navi's `story`, `own_chip_draws` and
`palette_step`; the shared cannon, AirShot and Spreader actions take their holders' animations from their game.
exe5-compat names the navis by number (records.toml's `[navis]`) and reads the level from a recording's setup
(`navi_levels`) and the B+Back special's damage from the stats block (+0x48). exe5-extract takes both versions'
faces, and an own chip's picture from its navi's team's ROM: the two US ROMs hold the twelve pictures alike but under
different palettes, and a console shows its own team's in the palette its ROM has for them (StepSwrd on a Team
ProtoMan console and C-Cannon on a Team Colonel one are both the yellow one). The chip lab operates a team navi by name (`navi = "protoman"`: set in RAM as the init exchange starts, since
a save that operates a team navi doesn't reach the link battle), with `navi_level` (the save's story flags) and `hp`
(the navi's block).

A side that operates a team navi states its level: a round's setup without one doesn't start, and the reader refuses
a recording whose setup has no `navi_levels` for such a side (MegaMan's side reads none). A navi that doesn't change
form is in the game's base form, MegaMan's, but its actor record's hooks are its own: what its identity wears goes on
at its start (`parts`: Colonel's cape), not the base form's put-on routine. `open_counter_window(ticks)` opens a
counter window of a navi's own length (0x0800CCDA; 16 when none is given), and the overlay hook `flinch_checked` is
a flinch hook that tests for what the navi wears first (EXE5's team navis'; EXE6's link navis' don't). The overlay hook
`lets_go` is KnightMan's entry in the flinch, drag and paralysis tables (one routine in the three): as the reaction
starts, what the navi wears is let go where an attack held it shown or hidden. An identity's `parts.own` is a routine
of its own in place of the engine's overlays: NumberMan's record's init hook (0x0800EE6E) puts on NumberSoul's layer,
a content kind, and since the table is by actor record (0x0800ED90) and more than a player's init calls it (the init
of a navi no player controls, the navi switch, and the images and stand-ins that call it by actor type and AI index,
0x0800ED0E's nine callers), the routine is the identity's and not the navi's. EXE5's afterimage is not one of the
callers: it is a sprite alone (effect object 0x28, 0x080E3550: the sprite its spawner gives it, the owner's battle
sprite by 0x0800DA72, with no NameID), where EXE6's copy of its owner (`sub_80E32D8`) takes the NameID and runs the
hook. Each game states which (the effects rule `afterimages_wear_overlays`): an afterimage of KnightMan has no ball
and one of NumberMan no face, Colonel's no cape, GyroMan's no propeller and NapalmMan's no cannon (each navi's
`afterimage` recording: ProtoMan's StepSwrd from its folder). (Those afterimages are recorded from the opponent's console. On the navi's own, a Team
Colonel console, the custom screen draws ProtoMan's StepSwrd in the folder in full color where ours draws the yellow
picture, 11 frames as the screen slides in: a console shows the other team's own chip in the palette its own ROM has
for it, and the pack holds each own chip's picture from its navi's team's ROM. Play can't put the other team's own
chip in a folder, so it is left; the battle's frames there were exact.) A player's init calls the post-init hook and then the record's init
hook in both games (EXE5's 0x0801400E and 0x08014012; EXE6's `sub_80172F0`, where the init hook is the base form's
alone); the init of a navi no player controls and the navi switch have them the other way round, in both games too
(0x08013CF0 then 0x08013D16, EXE6's `sub_8016F56`; 0x0802CC54 then 0x0802CC58).

**A navi's own hooks** (the framework GyroMan and ToadMan need). The original has tables by AI index beside the
state tables: a tick (0x080EB1E8, called each tick after the navi's action, 0x080EAD80: MegaMan's entry is his
souls', GyroMan's his own, the rest empty), an idle by control mode (0x080EB068: GyroMan's entries run before the
common idle, 0x080F0254) and the post-init hook (0x080EB2A8: ToadMan's sets the state bit 0x20, `dives`). A navi
definition states them: `tick`, for a navi that doesn't change form (one that does runs its form's); `idle`, after
which the common idle runs unless it started an action; `post_init`. GyroMan's tick (0x080F09EC) watches the panel
under him: over one that isn't solid it sets the state bit 0x8000 (the navi state `hovering`) and the request
0x10000000; with the bit set on a solid panel, the request 0x20000000; his idle (0x080F0978) answers them with his
own action 9, variant 0 (take off) or 1 (land). Nothing else in the ROM sets or clears the bit or either request,
and no end of an attack or a reaction clears them (their masks are 0x1803F and 0x1843F: the reactions section's
`request_clears`). The engine keeps the two apart from the request word as a navi's own requests (`take_off`,
`land`): the same bits are EXE6's mode-9 A press and takeover. While the bit is set a step sets no animation and
its ends leave the one he has (0x080F01CA; the move's start also sets a byte for AI index 2, 0x0800F234, which the
bit makes redundant).

**The status reset at a player's init.** MegaMan's entry in the post-init table (0x080F04EE) runs the full status
reset (0x08011B20, EXE6's `sub_80144C0`: the engine's `reset_status`) when the side's form isn't the base, and EXE5's
player init has no call of its own; the engine's init resets every navi of both games, as EXE6's init does. No link
battle can tell: a round's init finds every side in the base form (a link battle is one round, its stats fresh from
the save), where the reset of a navi just initialized changes nothing the recordings compare (they match from their
first frame for MegaMan and the team navis alike); a MegaMan starting a round in a soul would be reset once either
way, earlier in the engine's init than in the original's.

**Found with GyroMan.** A panel's flags word holds the panel's type in its low nibble by the game's own number of it
(EXE5's holy is 9, its metal 5): the engine's word holds the game's number too (the panels section's `numbers`), since
Airforce reads the word's low byte. A bomb of the GyroMan chips and GyroSoul's tornado end at the battle's end by
their state byte alone, their action and phase kept (0x080C7D5C, 0x080C8238), as GyroMan's strike does.

| Navi | Weapons | Own chip | Checked |
|---|---|---|---|
| ProtoMan | the charged slash (0x1E, 0x0800F8CE: WideSwrd loaded as the attack, the damage rows' row 0, counter byte 0x94) and the B+Back guard (0x0B, 0x0800F6CC: EXE5's guard as the Reflect program's, the stats' damage, 40 ticks before the next chip) | StepSwrd (action 0x13 as WideSwrd's, row 1, its routine 2) | 43 recordings (navis/protoman): his own chip at levels 0, 3 and 6, from another row, out of reach, blocked, onto a panel the opponent steps to, on a Japanese console and over four custom screens; the charged slash at three levels and let go early; the guard reflecting a buster shot and a Cannon, and too early; AntiSwrd against both; a counter hit and Full Synchro (an AirShot's; StepSwrd's makes none); twelve plain chips, a held A, a dark chip; hits, his deletion, a win; the charged slash, StepSwrd and the guard on the opponent's console. Every frame and every sound call. |
| GyroMan | his buster (0x43, 0x0800FB92): MegaMan's on a solid panel; in the air his own action 10 (0x080F0CAC: no element, counter byte 0x85, his Attack plus one; animation 21 and a sound, and a strike, attack object 0x1C, 0x080C7EA0, on every panel holding an enemy navi's body wherever it is, a burst of dust placed by two draws that hits on the tick it appears and is gone at its first update; 10 ticks). The charged tornado (0x28, 0x0800F9D6: the damage rows' row 2, counter byte 0x8A, no element; his own action 8, 0x080F0B6E: he raises his arm 4 ticks, then GyroSoul's tornado, attack object 0x1E with its second parameter set, lies on the ground along the three panels ahead; 30 ticks; no counter window). No B+Back. In the air (above) his own action 9 takes off and lands (0x080F0C0C: animation 18 for 18 ticks, then 20; animation 3 for 3 ticks, then out of the air), and his charge glow shows 16 pixels higher in animations 18 and 20 (the glow's test of his name, 0x080E0E58: the navi's `charge_glow_lift`). He wears his propeller (body overlay row 0), which his flinch restarts if he wears it and his deletion takes off; his paralysis and drag hooks are empty | Airforce (his own action 7, 0x080F0A74: on a solid panel animation 4, his collision off, 4 ticks; then off the field with his panel reserved, the chips' icons hidden on every console (`battle.show_hud`'s `chip_icons`), his panel column 0 and, for its row, the byte the reservation leaves in a register: his panel's flags' low byte, which holds the game's number of the panel's type. The GyroMan chips' navi, actor object 0x0C by its own spawner 0x080BAE8A, flies his row from his side's back column in his place with a bomb on each enemy panel: no bombing run, standing still while dimmed, its bombs waiting while dimmed too. When it is gone he is back on his panel, animation 3 on a solid one, 4 ticks, and to idle with that animation left; his tick rests meanwhile; row 3, 50 to level 3 and 70 from 4, counter byte 0x8A) | 72 recordings (navis/gyroman): over a hole the opponent's CrakBoms broke he takes off, flies, steps between holes with no step animation, lands on a solid panel and as the hole mends, through a custom screen, and on the opponent's console; from the air his buster's strikes (the opponent elsewhere, ending the round, in a Cannon's windup), his tornado, a Cannon and Airforce; in the air hit (as he steps onto the hole, as he takes off, flying, as he lands), pushed onto a solid panel by an AirShot, paralyzed by a Thunder, and deleted; the tornado at three levels, with the opponent on each of its three panels and a row up, over a hole, held by a dimming (as he raises his arm, and as it blows), let go early, he hit as he raises his arm, ending the round, and in a Cannon's windup (no counter hit); Airforce at three levels, on a Japanese console and over four custom screens, from the top row, with the opponent a row up, after his AreaGrab, held by a dimming (the flight, and a bomb in the air), a Cannon passing under it, his column taken meanwhile (his reserved panel stays his), he hit as he returns, ending the round, in a Cannon's windup, and on the opponent's console; the hover, the tornado and Airforce from the right on the lab's swapped base; a counter hit and Full Synchro (an AirShot's); twelve plain chips, a held A, a dark chip; hits, his deletion, a win; an afterimage of him (a StepSwrd's), which wears no propeller. Every frame and every sound call; 14 of them frame for frame against the original's picture. |
| SearchMan | the charged shot (0x3F, 0x0800FB1A: the damage rows' row 4, counter byte 0x8A, no element; it starts action 0x3B, the one SearchSoul's charged shot starts: the scope marks the enemy navi nearest ahead and five shots follow on its panel, the last flinching). No B+Back. His own action 8 (0x080F0E64: the same five shots at the panel of a target his attack is given) is his attack in an auto battle, which no weapon or chip starts: it comes with the gate navis' operation battle | Satelity (his own action 7, 0x080F0DFC: a dimming chip by his own action, not the dimming chips': he spawns its controller, effect object 0x3F, and starts the dimming himself with no cut-in, and since the cut-in tests the action, 0x0801454C, it cuts in on nothing either. The controller sends out SearchMan's scope as a sight, which its user stops with A or 300 ticks do; then he holds his animation 21 while a satellite, effect 0x61, hangs three panels behind that panel, opens and aims (8, 12 and 10 ticks) and fires: its beam, effect 0x62, cracks the panel and hits it for 60 ticks, and four flares, attack 0xAF, go out to the panels beside it, each hitting its panel until something is hit; row 5, 100 at every level, counter byte 0x94) | 53 recordings (navis/searchman): the charged shot at three levels, at an opponent in another row, as it steps away, let go early, interrupted, ending the round, and in a Cannon's windup (no counter hit); Satelity locked on the opponent's panel (with B, and with B late), on the panel below it (a flare's hit), on the first panel, left to its time limit at three levels and on a Japanese console, after the opponent's AreaGrab (the sight's columns; the satellite off the field's edge, a flare over his own panel), ending the round while dimmed, in a Cannon's windup, and against cut-ins both ways (the opponent's AreaGrab can't cut in on it, it can't cut in on an AreaGrab; an AreaGrab of his own does, and is cut in on); from the right on the lab's swapped base (Satelity twice, the charged shot, his custom screens); a counter hit and Full Synchro; twelve plain chips, a held A, a dark chip; hits, his deletion, a win; the charged shot and Satelity on the opponent's console. Every frame and every sound call; 8 of them frame for frame against the original's picture. |
| NapalmMan | his buster, a machine gun (0x44, 0x0800FBC0: the damage rows' row 6, counter byte 0x82, no element; his own action 7, 0x080F0F6C: animation 14 restarted each shot, on the second tick a bullet, attack object 0x12 in its row 0x13 (the panel it finds and the one behind it, no flinch), from the panel ahead at one of four heights by a draw, with the muzzle flash; five ticks, then three of recovery, which a held direction cuts short into a step with no test of the panel first; when the recovery runs out with B held a player's NapalmMan shoots again from the action's first step, and B held starts nothing from idle. A trap chip's catch takes the action over after its step). No charged attack and no B+Back. His A charge is of Fire chips (above; 120 ticks). He wears his cannon (body overlay row 4), which his deletion takes off; his flinch, paralysis and drag hooks are empty. His own action 9 (0x080F5FA4: two bombs at a target his attack is given) is his attack in an auto battle, which no weapon or chip starts: it comes with the gate navis' operation battle | Napalm (his own action 8, 0x080F1098: the bonus added into the damage, the target the panel four columns ahead in his row, the field's edge column at most; then the steps his auto battle's bombing runs: animation 18 and a counter window of 16 ticks, 21 ticks on animation 19 and a napalm bomb, the NapalmMan chips' attack object 0x3A in its variant 1, from 8 pixels toward the enemy's side and 56 up; 11 ticks, 71 more, and he stands; row 7, 100 to level 4 and 120 from 5, counter byte 0x9E. Its record's element is none and its family Fire: the bomb is Fire by its own spawner) | 58 recordings (navis/napalmman): the buster with B held at two levels, at an opponent ahead, on the next panel and a row up, a step out of its recovery, B and a direction held against the field's edge, he hit meanwhile, through a dimming, a trap chip's catch during it (AntiDmg's counter), ending the round, in a Cannon's windup (no counter hit), and on the opponent's console; Napalm onto an empty panel (its fire spreading) at three levels, onto the opponent, from his back column and his front column (off the field: a puff), at an opponent a row up, he hit as he raises his cannon (a counter hit on him) and after the shot, held in the air by a dimming, ending the round, in a Cannon's windup, on a Japanese console, over four custom screens and on the opponent's console; a Fire chip charged with A (FireHit1 plainly and charged, HotBody1), let go early, he hit as he charges, and the chips that don't charge (a dimming one, a dark one, his own, one of another element); the buster, Napalm and a charged chip from the right on the lab's swapped base; a counter hit and Full Synchro; twelve plain chips, a dark chip; hits, his deletion, a win; an afterimage of him (a StepSwrd's), which wears no cannon. Every frame and every sound call; 14 of them frame for frame against the original's picture. |
| MagnetMan | the magnet missile (0x26, 0x0800F9BA: the damage rows' row 8, counter byte 0x8A, Elec; his own action 7, 0x080F1158: animation 12 and a counter window of 10 ticks, 5 ticks on a missile, attack object 0x63, 0x080D24F0, 20 pixels ahead; 30 ticks. The missile, 16 pixels up, has 20 HP that the other side's attacks take; it waits 15 ticks, flies forward 4 pixels a tick lighting the solid panels under it, and as it passes a panel's center turns once toward a panel of that column holding an enemy navi's body, picked by a draw, to fly up or down 2.4 pixels a tick; a body it hits ends it, as do the field's edge and its HP running out, an explosion; it doesn't move while dimmed). His buster is routine 0, MegaMan's. No B+Back. His A charge is of Elec chips (above; 60 ticks). SuperArmor, FloatShoes and AirShoes. He wears nothing and has no hooks | NSTackle (his own action 8, 0x080F11EE: a one-tick hit of 1 on his own panel, animation 17 for 4 ticks marked as moving; then off the field at panel (10, 10) with his panel reserved and the chips' icons hidden. His target is the first panel ahead in his row holding an enemy navi's body. Two magnets, the MagnetMan chips' attack object 0x67 in its variant 1 (a flinching hit with no immobilizing, going at once, stopping while dimmed), slide 8 pixels a tick: pole 1 from the panel as far past the target as he is short of it, when that panel is on the field and free, and pole 0 from his own, meeting at the target; alone, the one from his panel slides until it hits a navi or leaves the field. He also spawns the magnets' pull, attack object 0x66, in its variant that follows a navi through the enemy MagnetMan's attack: a player's NSTackle is another action, so it is on the panel ahead for one tick. When the magnet from his panel is gone he is back, animation 3 for 4 ticks; row 9, 80 to level 2, 100 at 3 and 4, 120 from 5, counter byte 0x9E) | 61 recordings (navis/magnetman): the missile at three levels, turning up, down and down two rows to the opponent, turning toward an opponent that steps out of its column (it leaves the field), shot down by a Cannon, hit by a buster shot (it flies on), into a RockCube, over broken panels, held by a dimming, let go early, he hit as he aims (a counter hit on him), ending the round, in a Cannon's windup (no counter hit), and on the opponent's console; NSTackle with one magnet at three levels, with two (the opponent two columns ahead, and on the panel ahead), with no target, held by a dimming, a Cannon meeting his magnet, he hit as he gathers himself (his SuperArmor), ending the round, in a Cannon's windup, on a Japanese console, over four custom screens and on the opponent's console; an Elec chip charged with A (SpShake1 plainly and charged, Thunder), let go early, he hit as he charges (the charge holds), and the chips that don't charge (a dimming one, a dark one, his own, one of another element); the missile, NSTackle and a charged chip from the right on the lab's swapped base; a counter hit and Full Synchro; twelve plain chips, a dark chip; hits, his deletion, a win. Every frame and every sound call; 17 of them frame for frame against the original's picture. |
| Meddy | the charged capsule (0x3B, 0x0800FAA2: the damage rows' row 10 with no bonus, counter byte 0x94, no element, a plain capsule's parameters; her own action 7, 0x080F1424, her throw: she swings 8 ticks, throws a capsule, the Meddy chips' attack object 0x0C, at the panel three ahead with the throw's sound, 12 ticks, then 15 standing; no counter window). No B+Back. Her sprite's palettes go by two | MeddyCap (the same action 7 with its record's parameters: a capsule of variant 5, which paralyzes for 150 ticks with hit modifier 0, bursting along its column, not running while dimmed and not halved; row 11) | 51 recordings (navis/meddy): the charged capsule at three levels, onto the opponent, onto an empty panel beside it (the burst along its column), a column short, onto a broken panel (gone), shot while it sits by a Cannon (it bursts where it is) and by buster shots (it holds), held in the air by a dimming, let go early, she hit as she swings, ending the round, and in a Cannon's windup (no counter hit); MeddyCap onto the opponent (paralyzed, and shot meanwhile), onto an empty panel, a column short, shot while it sits, held in the air by a dimming, ending the round, in a Cannon's windup, at three levels, on a Japanese console and over four custom screens; her buster past her own sitting capsule from both sides; MeddyCap, the charged capsule and her custom screens from the right on the lab's swapped base; a counter hit and Full Synchro; twelve plain chips, a held A, a dark chip; hits, her deletion, a win; the charged capsule and MeddyCap on the opponent's console. Every frame and every sound call; 9 of them frame for frame against the original's picture. |
| Colonel | the Screen Divide (0x46, 0x0800FC0C: the damage rows' row 13, counter byte 0x94, its target the first enemy body ahead in his row, else three panels ahead; his own action 7, 0x080F89AE, which ColonelSoul's charged shot is MegaMan's cut of: he stands ready, draws his sword with a 16-tick counter window, and on the 10th tick cuts the target's panel and the two diagonally nearer him). No B+Back. He wears his cape (the body overlays' row 3), taken off at his deletion and restarted by an animation change or a flinch | C-Cannon (his own action 8, 0x080F8B30: the TankCans' cannon raised 8 ticks with a 15-tick counter window, then a TankCan shell from the panel ahead, 16 pixels up, without cracking; row 12, its routine 5 clears the wait) | 42 recordings (navis/colonel, him on side 1): the Screen Divide at three levels, at a panel's reach, on its diagonal, out of reach, let go early, and hit as he draws; C-Cannon at three levels, its blast at the far edge with and without the opponent in it, into a RockCube, into the opponent's start barrier, on a Japanese console, and over four custom screens traced on his own console; AntiSwrd against the Screen Divide; a counter hit and Full Synchro; twelve plain chips, a held A, a dark chip; hits, his deletion, two wins; an afterimage of him (a StepSwrd's), which wears no cape; the Screen Divide on his own console. Every frame and every sound call. |
| TomahawkMan | the thrown tomahawk (0x3C, 0x0800FACA: the damage rows' row 18, counter byte 0x94, no element; his action 7, 0x080F1760, throws attack object 0xB4, 0x080DCB98: four pixels a tick along his row, a slash against navis once a panel; along the middle row it comes back from the first panel with an enemy's body or the last column, along an edge row it crosses the first column with an enemy's body, or the last, and comes back along the other edge row; it waits while dimmed) and the A charge of Wood chips (0x2B, 60 ticks: `charged_chips` and `charged_bonus`, above). SuperArmor and UnderShirt (his stats row) | T-Swing (his action 8, 0x080F17D0: EXE6's TomahawkMan's charged swing with hit modifier 3, Wood, row 19, counter byte 0x9E) | 62 recordings (navis/tomahawkman): the tomahawk along each row, turning at the opponent's panel, its column and the field's end, through a RockCube, across a dimming, two at once, at three levels, deleting the opponent, against AntiSwrd; T-Swing landing at two levels, from the front column, on grass, into a Cannon's windup (no counter hit), against AntiSwrd, with his SuperArmor hit; a charged Lance and Boomer (their damage again), and AntiWood, DrkLance and T-Swing, which don't charge; his deletion past UnderShirt; a FireHit1 on his weakness (the mark, twice the damage, nothing breaks); three from the left (the swapped base), on his own console; the set every team navi gets. Every frame and every sound call. The tools' stats for him are the recordings' in what his definition states (trace-tests' exe5_team_navis). Its picture is compared with the original's (tools/frontend-compare/exe5.txt: the throw through a dimming from the opponent's console, and along an edge row on his own; T-Swing and his deletion were compared once). |
| ShadowMan | the shuriken from above (0x24, 0x0800F98E: the damage rows' row 14, counter byte 0x94, the attack's variant 1 and one throw; his action 9, 0x080F95F0: he vanishes in a puff, his panel kept and nothing reaching him, is back 48 pixels over it and throws AntiDmg's shuriken, attack object 0xC2, from 60 pixels above himself at the nearest enemy body ahead, his own row first) and the B+Back anti-damage stance (0x0D, 0x0800F704: ShadowSoul's with row 14's damage; a caught hit turns into the anti-damage counter, its variant 1). FloatShoes (his stats row) | SplitUp (his action 10, 0x080F998A, over in a tick: his shadow, actor object #4, 0x080B9384, on the panel before the nearest enemy body ahead, which slashes the two panels ahead of it as LongSwrd does and blinks out; row 15, counter byte 0x94; no hit reaches SplitUp's shadow) | 66 recordings (navis/shadowman): the shuriken at three levels, from each row and at each row, from his area's front, across a dimming, twice, deleting the opponent, against AntiSwrd and into a Cannon's windup (no counter hit), with a buster shot and a Cannon passing under him; the stance catching a Cannon at three levels, a buster shot and a Sword, raised too early, and unanswered with its chip wait; SplitUp's shadow before the opponent in each place (a row up, the back column, his own side's front, his own panel, over a hole, on a RockCube's panel), missing an opponent that steps away, across a dimming, deleting the opponent, against AntiSwrd and into a Cannon's windup; his FloatShoes over cracked panels; the three from the left (the swapped base), on his own console; the set every team navi gets. Every frame and every sound call; the tools' stats for him are the recordings'. His picture is compared with the original's (tools/frontend-compare/exe5.txt: the charged attack across a dimming, the stance's catch, his shadow on his own console; 23 scenarios were compared once, 13,788 frames, the three from the left among them). Not ported, being his auto battle's alone: his actions 7 and 8 (0x080F9522: he and his shadows throw flames, attack object #7; 0x080F97AE: he steps in beside his target and slashes), his own move that calls two shadows into the battle state's slots (0x080F9444, 0x080F99EA), and with them a shadow that can be hit and its other orders (navis/shadowman/shadow says which). |
| KnightMan | the wrecking ball's swing (0x10, 0x0800F776: the damage rows' row 20, counter byte 0x9E, the attack's variant 1; his action 9, 0x080FBFF4: two ticks in he swings with a 16-tick counter window and his swing's sound, and KnightMan the navi chip's ball, attack object 0x17, circles him once at 12 a tick, its circle moving with him if he is pushed and waiting while dimmed; 58 ticks). No B+Back. SuperArmor (his stats row) and a move lag of 10. He wears his ball and chain (the body overlays' row 1, in front at every animation), taken off at his deletion and restarted by an animation change; his attacks hide it while its ball is away and hold it so, and a flinch, a drag or a paralysis lets go of that hold (the overlay hook `lets_go`: one routine, 0x080FBA20, his entry in the three tables and nobody else's), so an attack cut short leaves it shown again | KCrusher (his action 8, 0x080FBEB2: a tick in he throws with a 16-tick counter window and the cannon's sound: his iron ball, attack object 0x15, 0x080C724C, leaves 40 pixels ahead of him and 42 up and flies five pixels a tick down his row to the field's end, a breaking hit with hit modifier 3 on everything along it, once a panel, waiting while dimmed; 50 ticks with his arm out, 8 drawing it back; row 21, counter byte 0x9E; its routine 1 gives the ball he wears, no wait and the 50 ticks) | 59 recordings (navis/knightman): the swing at three levels, beside the opponent, on its diagonal, with nobody near, from the top row and from the back corner (its circle off the field), twice, across a dimming, breaking a RockCube, deleting the opponent, hit by buster shots through his SuperArmor, and into a Cannon's windup (no counter hit); the swing cut short by a paralysis (a ParaBom), by a push past his counter window (an AirShot), by a counter hit in it (the circle moving back with him) and by his deletion; KCrusher down his row, at a panel's reach, past an opponent a row up, through a RockCube into the opponent, through a raised Guard1, across a dimming, deleting the opponent, into a Cannon's windup (no counter hit), hit through his SuperArmor, cut short by a paralysis and by a push, and with a Fan pulling him forward as he throws (he slides, the action going on); the two from the left (the swapped base), on his own console; an afterimage of him (ProtoMan's StepSwrd from his folder), which wears no ball; the set every team navi gets. Every frame and every sound call; the tools' stats for him are the recordings'. His picture is compared with the original's (tools/frontend-compare/exe5.txt: the swing across a dimming, a paralysis in it, KCrusher on his own console, his afterimage; all 59 scenarios were compared once, 33,023 frames). His flinch entry isn't reached by a recording: his SuperArmor keeps him from flinching. Not ported, being his auto battle's alone: his action 7 (0x080FBD98: he hurls his ball up and away, attack object #0x14, and it comes down on the other side's panels), and the glow and the dropped guard his two actions start with there (the attack's variant 0). |
| NumberMan | the die's throw (0x3D, 0x0800FAE6: the damage rows' row 16, counter byte 0x94, no element, a 30-tick wait; his action 7, 0x080F15D8: he throws for 22 ticks, and six ticks in, with the throw's sound, the NumberMan chips' die, attack object 5, leaves 24 pixels over his place for the panel three ahead, its face by his own table, 0x080F1664, with 40 HP and hit modifier 3, a body the other side can hit; no counter window, and no status: he can be moved off it). No B+Back. He wears his face: NumberSoul's layer in its row 0, a content kind, put on by his record's init hook (0x0800EE6E: the identity's `parts.own`) and taken down by its death hook | NumTrap (his action 8, 0x080F169C: a dimming of his own as Satelity is, which nothing cuts in on and which cuts in on nothing; its controller, effect object 0x40, 0x080E4D68, looks after 30 ticks at the panel two ahead: a panel of the other side's that can be stood on, with nothing on it, else nothing is set; he holds his animation 12 for 70 ticks with a sound, and 20 ticks in his trap, attack object 0xC3, 0x080DF1E0, is set there: the land mine in its palette 1, shown 41 ticks and then 255 pixels up, out of sight. Armed, it stands still while dimmed, goes with a panel that can't be stood on, and springs under a body of the other side's: a sound, and its press, attack object 0x8C, 0x080D7CE4, over a square of four panels with the trap's in it, drawn among the squares whose top left panel is on the field above the bottom row: it hangs 192 pixels up 6 ticks, falls 32 a tick, shakes the camera and hits the four panels, an attack at navis with hit modifier 3 and the plain spark; row 17) | 66 recordings (navis/numberman): the die at three levels, onto the opponent, onto an empty panel (its burst over nine), from his area's front and back and from the top row, shot by a Cannon (40, its HP) and by buster shots, across a dimming, twice, deleting the opponent, into a Cannon's windup (no counter hit), and with him hit before and after it leaves; NumTrap sprung at three levels, after ten seconds, in each row, on the opponent's start panel, in the first column (behind an AreaGrab), with its press across a dimming, deleting the opponent; not set on his own panel, an occupied one, a RockCube's or a broken one; gone when its panel breaks; walked over by NumberMan himself in a column he took; pressed into the other side's telop and the other side's chip into his (no cut-in either way); an afterimage of him (ProtoMan's StepSwrd from his folder), which wears no face; the three from the left (the swapped base), with the trap in the last column, and a fourth where a die thrown first moves the draw and the press's square starts in the last column: it hangs half off the field; the set every team navi gets. Every frame and every sound call; the tools' stats for him are the recordings'. His picture is compared with the original's (tools/frontend-compare/exe5.txt: his trap set and sprung on his own console, its press across a dimming, his afterimage; all 66 scenarios were compared once, 44,742 frames). |
| ToadMan | the dive (0x2E, 0x0800FA1C: the damage rows' row 22, counter byte 0x94, the attack's variant 0; his action 8, 0x080F19D8: 4 ticks going down as a move, then under with a splash, his own panel kept for him and himself off the field with no hit region for 30 ticks; then he comes up, 0x080F1C20, on the panel before the first body of the other side's, looked for column by column ahead of his own panel and from the top row down in each, where that panel can be stood on and has no body on it: a splash, 8 pixels up for a tick, and his strike, a 60-tick counter window and on its 9th tick a hit on the panel ahead, 16 pixels up, an Aqua attack at navis with hit modifier 3 and the Aqua spark; 13 ticks, 5 more, 4 going down, then back on his own panel, 4 ticks coming up and 20 standing; with nowhere to come up he comes straight back) and the A charge of Aqua chips (0x2C, 60 ticks: `charged_chips` and `charged_bonus`, above). He dives where a panel submerges (his post-init hook, 0x080F199C: the navi state `dives`) | S-Melody (his action 7, 0x080FCA36: he sings with a 10-tick counter window and the melody's sound, and the ToadMan chips' note, attack object 0x81, starts its chain on the panel ahead toward its target, each note 10 ticks and the next 6 ticks in, paralyzing; 10 ticks; row 23; its routine 4 finds the target, 0x080BC92C, and gives no wait and the 10 ticks) | 70 recordings (navis/toadman): the dive at the opponent in his row, a row up, in its back bottom corner and on the panel ahead of his own (he comes up on his own panel), with a RockCube or a broken panel before the opponent (nowhere to come up), with the opponent stepping away, with a Cannon passing while he is under, across a dimming, twice, deleting the opponent, into a Cannon's windup (no counter hit); hit by a buster shot as he strikes (no flinch) and by a Cannon (a counter hit, the dive cut short on the opponent's side) and as he goes down, and deleted on the opponent's side; S-Melody at the opponent in his row, a row up and two rows down, then a dive on the paralyzed opponent, across a dimming, countered as he sings, deleting the opponent, into a Cannon's windup; a charged WideSht1 and SideBub1 (their damage again), let go early, with RainyDay, DarkWide and S-Melody in hand (no charge), with a buster shot (the charge holds) and a Cannon (it is lost) as he charges; on the sea stage, under the sea ahead of him: a buster shot passing over him and a Thunder reaching him, walking through the sea and back, shooting, a Cannon, his dive and S-Melody from under; a Thunder on his weakness on the plain stage (the mark, twice the damage, nothing breaks); the three from the left (the swapped base); the set every team navi gets. Every frame and every sound call; the tools' stats for him are the recordings'. His picture is compared with the original's (tools/frontend-compare/exe5.txt: the dive on his own console, S-Melody toward another row, him under the sea on his own console; all 70 scenarios were compared once, 39,492 frames). Not ported, being his auto battle's alone: the dive's variant 1 (0x080F1CA0: he comes up before a target his attack is given). |
