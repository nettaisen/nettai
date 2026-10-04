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
- **The action map** (§14). Of BN5's 330 chips:
  - 122 descend from the same chip's BN6 code: 21 identical where the labs ran, 8 with constants only, 93 changed.
  - 101 are built like another BN6 chip's code.
  - 17 descend from code BN6 keeps but no BN6 chip uses.
  - 90 are BN5's own, LeadRaid and ChaosLrd among them. LeadRaid is BN6's TwinLdrs's ancestor (the same actor
    kind and handler), reworked.

  Three shared routines differ for many chips:
  - The dimming chips and action 0x1A leave the action on the use frame in BN5.
  - The navi chips' framework is the same code with renumbered chips.
  - The swords pick their slash by soul in BN5, by form in BN6.

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
   IWRAM code); `exact?` when several, the one nearest its neighbors' move.
2. `shape`: the same shapes at exactly one discovered BN5 routine.
3. `call`: a BL (or a code pointer) at the same place of a matched pair (strong when the pair is the same code).
4. `table`: the same index of a table of code pointers both load (object kinds, state machines, handler tables,
   tables of tables).
5. `order`: between two matched neighbors, the unmatched routines of both gaps paired in order by similarity.

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
and their neighbors') step their state machine in +0x0B where BN6 uses +0x0A (63 places, 8 of them in
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
the shoes and armors +0x1B to +0x23, the form +0x29, HP +0x40/+0x42) are the same offsets. Seen moved, one call
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
- the slot layout's neighbor fill `sub_8027F42`, the deal's compaction `sub_802945A`: same;
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
and can't use the navi chips (0xDD–0x118) but the DS ones; a light MegaMan can't use the dark chips or the DS navi
chips. Each such chip fizzles: the navi enters action 0x1A for a frame and a puff of smoke appears. Which branch
decides is not read yet.

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

### 6.1 Light and dark MegaMan

BN5's light and dark MegaMan is one number, NaviStats +0x44 (a halfword, read through 0x08012882): the save's
light/dark meter. Tango's finished Team ProtoMan save has 500, its finished Team Colonel save 0, Tango's light
netplay templates 1000. Three rules read it, with two thresholds:

- **The chips a MegaMan may use** (0x08010118, BN5's own; it passes any navi but MegaMan, NaviStats +0x29 ≠ 0). A
  chip record's byte +0x15 (BN6's library sub-index, read only by the menus) is the chip's **`megaman`** field:
  0 either, 1 light only (meter ≥ 470), 2 dark only (meter < 470). Light only: the navi chips and their SP ones,
  GunDelSol, the Barriers, HolyPanl, BugFix, Snctuary, Otenko, JustcOne, MetrKnuk, HolyDrem, BigHook. Dark only:
  the DS navi chips, Static, Muramasa, Anubis, BlakWing, BugCurse, BugCharg. The same in both versions. The dark
  chips (flag 0x20) need a dark MegaMan as well, by their flag. A use the field refuses fizzles: the navi enters
  action 0x1A for a frame and a puff of smoke appears, nothing else.
- **A dark MegaMan on a holy panel turns it Normal** (0x08017136, BN5's own): when the panel under the object is
  holy (BN5's type 9) and its side's meter is ≤ 499, the panel is set Normal (the panel setter). It runs **every
  tick**, from the object's per-tick intake update (the counterpart of BN6's `sub_801AC6C`, 0x080178EC, and of its
  five siblings for the other object kinds, `sub_801A9B8` to `sub_801ABB8`), so it takes the panel the first tick
  of the fight a dark MegaMan stands on one, and any holy panel he steps onto; a light MegaMan's never. The
  threshold is 500, not the chips' 470: a meter of 470 to 499 uses light chips and still clears holy panels.
- **The Soul Unison button** is not on a dark MegaMan's screen (observed; the check is not read yet).

**In nettai** this is a BN5 system ("light and dark"): the meter is the side's state (from the save, by the
side's setup), the chips' rule is a hook on chip use, the holy panels' rule a per-tick hook on the side's navi, and
the chip field is a BN5 extension on chip definitions, `extends = { chip = { megaman = "light"|"dark"|nil } }`
(a system's extension, rules-in-luau.md §7.5), nil meaning either. A BN6 chip has none. A BN6 MegaMan in a mixed battle has no
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
  toward the user, until it runs off the field (0x080BD96C). A panel counts when its type is BN5's holy (9). For
  each one found, 10 ticks later it shows an effect on it, **sets it Normal** (the panel setter, which refuses only
  a missing panel) and fires another shot. With none left it waits 60 ticks and ends.
- **The shots** (T3 kind 0x83, 0x080D6A60): bullets from the actor along **the user's row**, each stopping on the
  first thing it hits; 50 damage each (plus the modifiers). A target standing on a holy panel would halve it as any
  hit; in these recordings each target's panel was Normal by the time the shots came.
- **So:** the damage is 50 × (1 + the holy panels on the field), every holy panel ends Normal, and with no holy
  panel it is one shot of 50. An opponent out of the user's row takes nothing, though the panels still go.
- **Recorded** (Team ProtoMan's console, both sides): the default stage's holy middle row (side 0: 5 panels, as
  the dark Team Colonel turned its own Normal, 6 shots, 300; side 1 on bn5-team-light: 6 panels, 7 shots, 350);
  a stage without holy panels (one shot, 50); a HolyPanl first (it makes the panel ahead of the user holy: two
  shots, 100); the opponent a row up (no damage, all the panels Normal).
- **The first side-1 recording's no damage** was not the panels: it ran on bn5-team, where Team Colonel's MegaMan is
  dark, and HolyDrem fizzled before any actor existed.
- **In nettai:** the scan reads the panel type by what it is, the engine's `PanelType::Holy` (BN5's 9 and BN6's 5
  are both that), and the absorbing writes `PanelType::Normal`. So in a mixed battle it counts every holy panel
  whoever made it: a BN6 HolyPanl's, a BN6 holy stage's, its own side's and the opponent's. Nothing in it is BN5's
  alone about the panels: no BN5-only panel state, no owner or timer read, only the type and the setter BN6 has
  too. What a BN6 field wouldn't provide is the BN5 rules around it: the light MegaMan it needs (§6.1), and the
  dark MegaMan turning holy panels Normal under him every tick (§6.1), which changes the count when a dark
  MegaMan stands on one.

### 6.3 Chips that need a set-up (read for the chip lab)

The chip lab's templates stand the opponent where each chip lands (verification workspace, tools/chiplab/
reach_bn5.py); a few chips hit nobody standing anywhere, and their code says what they need:

- **WavePit, RedWave, MudWave** (action 0x1A, sub-type 0x13, 0x080DAF18; the chip's parameter picks the panel):
  for each row, from the user's end of the field toward the other, the first panel with the kind's flag starts a
  wave along that row: a sea panel (flag 0x20000) for WavePit, a lava panel (0x1000) for RedWave, both then set
  Normal; for MudWave a panel without the standable flag 0x10 (a hole), left as it is. No such panel in a row, no
  wave there; none on the field, the chip does nothing.
- **The mode chips** (CannMode to DrilMode, FinalGun; action 0x1A, sub-type 0x0C, 0x0802D62A): they set the
  user's side's buster mode (the per-side block of BN6's flag-0x40 gauges, `sub_802E070`: its +0x0B the chip's
  parameter, +0x2E 480 ticks, 360 in flag-0x40 mode). The damage comes from the buster afterwards.
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
  the navi chips' action (0x41, BN6's 0x1B, the same code) with subtypes 0x15 and 0x1D: the summon table
  (`off_802CD5C`'s counterpart, 0x080298C4) spawns actor kinds 0x20 and 0x51. Their pictures (record +0x24: 0x0872F8E8,
  0x0872FE28) and icons (+0x20) are in the ROM. ChaosLrd's summon has a name in the battle name list (entry 0xE6,
  "Chaos Lord", JP ロードオブカオス); LeadRaid's none.
- **What the save has:** their names (the ROM's name entries are the text command `FF 00 n`), their descriptions
  (`FF 01 n`) and their pictures' palettes (record +0x28 points into EWRAM): slot n's name is a one-entry text archive
  at save +0x1D14 + 0x18 n, its description one at +0x1374 + 0x64 n, its palette 16 colors at +0x1660 + 0x20 n (the
  save is EWRAM from 0x02000000, so those are its addresses once loaded). LeadRaid is slot 0, ChaosLrd slot 1.
- **How the game gets them:** an e-Reader card read over the link: the card dispatcher (0x0812F3BC) passes the card's
  kind to 0x0812F8F0, which notes the card in the save's obfuscated bytes (BN6's `encryption_8007004`, index 0x1020
  plus the slot; what reads that is not read), gives the chip once (`GetChipCountOfCode`, `GiveChips`: LeadRaid in L,
  ChaosLrd in X) and decompresses the card's name and description into the slot and copies its palette. A new game
  clears the slots (`sub_8021D36`, the same code as BN6's): the palettes zeroed (a black picture), the names and
  descriptions "????". Owning the chip is the pack's count, as any chip's; its text and colors are the slot's.
- **The saves:** Tango's eight raw netplay saves (both versions, light and dark, US and JP) hold both chips' slots,
  the same in all (the US's "LeadRaid" "ProtoMan & Colonel together!" and "ChaosLrd" "Hatred formed into Bass", the
  JP's リーダーズレイド and ロードオブカオス), and so do the four GBA saves in Tango's saves folder (masked). BN5 DS has
  no such block: its save doesn't hold them, and its ROM (bn5.nds) has both palettes (0x00B7EF40, 0x00B7EF60) and
  ChaosLrd's name, so the DS game's two chips are its own.
- **BN6's card buffer is the same mechanism**, which BN6 inherited: the same slot sizes (a 0x18-byte name archive, a
  0x64-byte description archive, a 0x20-byte palette), the same text commands, the same clearing routine; at other
  addresses (BN6: names 0x02001180, descriptions 0x020007D0, palettes 0x02000AF0) and filled by a gift over the link
  rather than an e-Reader card (docs/engine/jp-differences.md).
- **In content/bn5** they are content-given chips as BN6's gift chips are: their records from the ROM, their strings
  in the locales (English from Tango's US saves, Japanese from its JP saves), their palettes the definitions'
  `art_palette`, their slot compat's `save_slot`; gen_content.py's check reads them from the raw saves.
- **Recorded:** the chip lab's chips/0x137-leadraid and 0x138-chaoslrd (hit, adjacent, miss, side1). LeadRaid hits
  twice for 200 wherever the opponent stands (its row or not); ChaosLrd once for 500 wherever it stands. Both are
  either MegaMan's (+0x15 0).

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
  custom screen with Soul Unison, and its navi switch (from a gate's navi chip) never happens (§5). **A plain
  NetBattle is recorded too** (the chip lab's `bn5-netbattle` base: Tango's primer, then the gate's flag lowered
  and the comm menu's root cursor put on NetBattle; mode 2, a triple NetBattle). Its battle runs from the other
  link applet (0x081359C4) with the same BattleState and settings records; its custom screen is the shared one too.
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
- **Formats:** `nettai-content/hud` and `nettai-content/custom` were BN6's layouts (multi-game.md §1.6). BN5's
  HUD and custom screen are written in them with optional fields where BN5 lays them out otherwise (§11: the
  faces' own boxes, the window's layout, the buttons by name), which a BN6 pack writes none of.

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
  m4a bank (308 songs, 105 samples), the 368 chips' pictures and icons (and the invalid chip's, 0x185, past them),
  the HUD's 8x16 font, and the dialogue font (442 glyphs; BN5's advance table is a word a glyph, BN6's a byte).
- **The HUD** (bn5-extract's `hud.rs`): the layer from tile 0x180 and the gauge from 0x202 (BN6 0x1A0, 0x222), the
  HP box, the gauge's frame and "L or R", the enemy digits, the chip icons' palette, the banners (49) with their
  digits, "Cstmzing...", "PAUSE", the HUD's text lines, the warning marker, the chatbox (its 20 tiles from 0x2EB,
  its maps and arrow) and the emotion window's faces: BN5's 16 pictures a version (0-4 MegaMan by emotion, the
  order of BN6's table, 0x0801AFB4; 5-10 the version's souls; 11-15 dark MegaMan), 0x180 bytes each, a face of
  MegaMan's or dark MegaMan's with its own 2x2 box (`mugshot_boxes`), a soul's beside the turns left (`counts`,
  0x0801985C: 10 - n), 22 palettes (11 more for Chaos Unison); Team Colonel's from its own ROM (0x087417FC...).
- **The custom screen** (`custom.rs`): BN6's window and patch list but for the special slot, a 3x2 button; the
  window's parts at BN5's tile numbers (`layout`: the name 0x59, the picture 0x69, the code 0x93, the element
  0x95, the digits 0x99, the slots 0x9F, the column's icons 0xE1 and cells 0x47, the turn limit 0x4B, the name
  bar 0x1B6), a hidden slot filled with tile 2, the cursor over OK at (0x58, 0x70) and over the button at (0x58,
  0x88) with their corners (0x08024714, 0x08024744); the 13 element icons in BN5's family order, put in the
  engine's; the re-deal and scrap buttons; the soul button (`buttons`: its states' tiles 0x086FBB64, its picture
  0x087322E8 with Soul Unison's and Chaos Unison's palettes, the souls' 2x2 icons 0x08749FB8, 14 with Chaos's, in
  sprite palette 13, 0x0874AAB8); the emblems by version (13, with 8 palettes).
- **Versions:** 12 chips (0x12D–0x136, 0x139, 0x13A: the version navi chips) are drawn differently by each
  version's ROM; their pictures and icons are in the pack twice, `CHIP-protoman` and `CHIP-colonel`, with their
  `version`, and a console shows its own version's (the frontend's `Packs::chip_art`).
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
  - **Languages** (BN6's shape, text-rendering.md §10): with a BN5 content root, its strings go in
    `content/bn5/locales/en.toml` (from the US ROMs) and `ja.toml` (from the Japanese ROMs), keyed by definition
    key; and bn5-extract writes the Japanese ROMs' lettering beside the US's as nettai-assets' `HudLettering` and
    `CustomLettering` (the fonts in the Japanese encoding, the HUD's lines, the banners and pictures with words in
    them), as bn6-extract's `lettering` does. The strings exist; the lettering doesn't yet: the pack is the US
    ROMs' lettering alone, so a BN5 console in Japanese (`--lang ja --text original`) draws the Japanese names in
    the US font (the static audit says so, not counted).
- **Names:** placeholders (`sprite-0c-2d`, `sound-10e`, `chip-12d`; a glyph's number in brackets) until a BN5 content
  root names them in its compat, as BN6's does.
- **Shared decoding:** the sprite archive and GFX-animation decoders are BN6's format and code; bn5-extract has its
  own copy, which belongs in one shared decoder with bn6-extract's when that is next reworked.
- nettai-content's HUD reader accepts a pack without the count box (`no_count_box`: BN5 has none).

## 12. Reproducing

In the verification workspace, with the ROMs in `$BN6_ROMS` and the BN6 disassembly's symbols in `$BN6F`:

    tools/bn5/bmap.py [--to BRKE] [ram]    # target/bn5/bmap-<CODE>.tsv, ram-<CODE>.tsv, the summary
    tools/bn5/bmap.py diff NAME...         # a BN6 routine beside its counterpart
    tools/bn5/fields.py [r5 | navistats | after NAME]
    tools/bn5/only.py                      # BN5's own battle code
    tools/bn5/versions.py [BRBE BRKE]      # the two versions

`target/audit/classes.tsv` (the audit, `tools/audit/audit.py`) gives the areas and classes; without it the map is
the same, unclassified.

## 13. BN5's root and compat (as built, before R)

What BN5 verification and content need that doesn't depend on the roots (rules-in-luau.md R), shaped to R1's
format.

- **content/bn5** (written by the verification workspace's tools/bn5/gen_content.py from all four ROMs; its `check`
  mode compares them again): `root.toml` (name "bn5", assets "bn5"; it requires BN6's root since the port, §15.1); `compat/chips.toml` (328 chips by
  key: id, action and subtype; `damage_formula` for the 46 whose damage is a formula, 1000 and up; `colonel` for the
  48 whose Team Colonel record differs: the version Gigas' library flag, the navi chips' +0x16);
  `compat/panels.toml` (BN5's 11 panel types, their flag words and the engine's type each is);
  `chips/<key>/chip.luau` (each chip's common record and its `megaman` extension, §6.1; no use yet; since §15.12 a
  series' ported chips share chips/<series>/chips.luau, and the generator finds each definition by its id); and
  `locales/en.toml`, `ja.toml` (names and descriptions from the US and the Japanese ROMs). The records are Team
  ProtoMan's: the Japanese ROMs' differ only in the library's sort keys (+0x18), Team Colonel's only where compat
  says, and both versions' strings are the same. Keys follow BN6's: the US name in lower case, `+` and spaces as
  `-`, a navi chip's SP or DS split off (`gyroman-sp`); the second CannBall (the mode chip, 0x11A) is
  `cannball-mode`. Rewriting keeps a ported chip file's code: only the record's fields are rewritten.
- **bn5-compat** (the engine never depends on it): `Compat::bn5()` (content/bn5/compat built in) and `read`; keys
  unqualified inside, qualified `bn5:<key>` at its boundary (`qualify`, `strip`, `Compat::chip`); BN5's pool sizes;
  the codec (the 0x60-byte NaviStats with the light/dark value and its two thresholds, the panels by compat's
  table, the chip blocks); and, with the `trace` feature, the chip lab's recordings read and decoded. The
  verification workspace's trace-tests `bn5_lab` decodes every recording in data/traces/lab-bn5 (1,371 of them,
  1,134,927 frames).

**Waiting on R** (and the port): the loader reading content/bn5 (its manifest, qualified keys, the locales by
root); the chips' handles (the codec gives ids and qualified keys); a use for each chip (`define.chip` asks for
one, so the generated records don't load until the port writes them, or the loader takes data-only chips); the
`megaman` field as the light-and-dark system's extension (S7's `extends`); the type check of content/bn5 (BN6's
core.d.luau has no `megaman`); BN5's kinds (no kinds.toml: objects decode to their pool and BN5's kind number);
BN5's stages (the settings record stays raw).

**What has no engine counterpart yet:**

- *Chips:* `megaman` (+0x15); the families recovery and invisible (the engine's `ChipFamily` has neither; BN5's
  obstacle family is BN6's summon); BN5's damage formulas (its own table: compat keeps the row); +0x16 (BN6's extra
  flags byte: BN5's bits unread, written as numbers) and +0x17 (BN6's lock-on mode: BN5's byte, mostly 0x10,
  unread); a version's own record (Team Colonel's differences).
- *Panels:* metal (type 5: BN6's road flag, its look a plate, its behavior unread) and sea (type 10, flag
  0x20000); the panel record's 0x24 bytes (the lava and sea timers the panel setter starts, at +0x10 and +0x14).
- *Pools:* the actors' 16 slots (the engine's `object::SLOTS` is one number, 32).
- *NaviStats:* the light/dark value (+0x44); the weapon bytes' BN5 meaning (+0x04, +0x05, +0x07, +0x39: BN6's
  buster and +0x39 swap places in the one call each that pairs them); and the bytes whose BN5 meaning isn't read:
  +0x00, +0x0F, +0x11 to +0x1A and +0x24 (paired with BN6's NaviCust bug bytes at the same offsets, BN5's bugs not
  checked), +0x1E to +0x22, +0x25 to +0x28, +0x2A, +0x2D to +0x38 (BN6's folder bytes; BN5's patch-card routine
  writes there), +0x3A to +0x3D, +0x46 to +0x5F. BN6's fields BN5 has elsewhere or not at all: the starting form
  (BN6 +0x17, BN5 +0x2C), the Tag chips, ChpShufl and NumbrOpn, the Beast Out counter, the version byte, the sun,
  the hand-shrink bug.
- *Setup:* the BattleSettings record (BN5's stages by their own numbers), BattleState (0xF0 bytes; the traces carry
  it raw), the versions and regions (both decoded).

## 14. The action map: BN5's chips against BN6's code

For each of BN5's 330 chips, the BN6 code its use descends from and how the two differ, from the disassembly: what
a port can take from BN6's Luau (`require("@bn6/...")` with BN5's assets and records) and what it must write. Made
by the verification workspace's `tools/bn5/actions.py`; the machine-readable map is its `tools/bn5/chip-actions.tsv`
(a row per chip: the roots in both games, the BN6 chips and module, class, relation, every differing region and
constant with its routine and offset, the data tables that differ, the kinds paired, the labs' verdicts),
`chip-library.tsv` (the shared routines, §14.3) and `chip-systematic.tsv`. `actions.py --diff KEY` prints a chip's
differing routines side by side, marked with what ran in the labs.

### 14.1 Method

- **Roots.** A chip's use runs its action's handler (BN5's action table 0x080EB42C, BN6's `JumpTable80EAC60`; the
  games number actions apart: BN5's Thunder is 0x20, BN6's 0x1F) and, where the handler dispatches by the chip's
  subtype, the subtype's routine: the dimming chips' spawners (BN6 `off_802CCB4`, BN5 0x080297B8), the navi chips'
  summons (`off_802CD5C`, 0x080298C4), and any code-pointer table the handler, what it calls or its states index
  with the attack's subtype byte.
- **Candidates.** The BN6 handler likest BN5's and its table's likest entry, and the routine of the BN6 chip of the
  same name, if there is one. Names only propose; code decides. "Likest" weighs the code's ratio (exact
  instruction forms) together with the ratio of what it calls, loads (state tables) and spawns at the same places,
  four levels down, each weighted by size, so a dispatcher or a spawner stub weighs little against the states and
  objects it reaches. Object kinds pair where they are spawned (`movs r0, #kind` then a spawner), because the games
  number kinds apart.
- **The walk.** From a candidate's roots, both games' code in step. Each routine is decoded by flow. Each pair is
  compared as §1 compares: same, consts, similar or differs. Coverage limits the walk:
  - BN6's chip lab records block coverage of its chips' runs, and BN5's now does too, from a list of block leaders
    (`actions.py --blocks`, chiplab's `CHIPLAB_COV_BN5`).
  - The walk stops at a pair that neither lab ran.
  - A table indexed by a parameter pairs the entry BN5's lab ran with the likest entry BN6's ran.
  - The BN6 chips whose labs stand for BN6 are the chip's namesake in the family, else the family's chips whose
    labs ran the most of the walk.
- **The choice.** The candidate whose walk is likest where both labs ran wins. BN5 code that ran unpaired counts as
  unlike. Under 0.6 (0.45 when the family has the chip's name) the chip is BN5's own.
- **What differs, where it ran.** A pair's differences are regions (instructions inserted, removed or changed, and
  constants). A region counts only where one of the labs ran it; `off_path` counts the rest.
  - The library's routines are listed apart (§14.3): those that walks from two BN6 roots reach, and those BN6 calls
    from eight places or more.
  - Systematic constants are listed apart too: alike in three routines or more (collision data +0x70 is BN5's
    +0x68 in 17 routines; a value 8 is 6 in 9).
  - Relocated addresses aren't differences. Data tables the code reads (by address) are compared bytewise and
    listed as `data`. They are often subtype-indexed, so the chip's own entry may still be the same (Cannon's
    shot-parameter table differs only past BN5's three cannons).

| class | meaning |
|---|---|
| identical | its own code is BN6's (calls, addresses and object kind numbers aside) |
| identical-run | it differs only where neither lab ran (Cannon: GigaCannon's branch of the shot) |
| constants | where it ran, only constants differ: sounds, sprites, a call's value, field offsets |
| changed | where it ran, instructions differ (each region with what BN5 adds and drops) |
| bn5-only | no BN6 code is as like as 0.6 |

| relation | meaning |
|---|---|
| same chip | the BN6 ancestors include the chip's name |
| built like | the nearest BN6 code is another chip's: a renamed one (WideSht1–3 and WideSht, AirHoc and AirHocky, Geddon1–3 and Geddon, DrilArm1–3 and DrilArm), a variant (DarkWide on WideSht, Guard on Reflector), or a navi chip on another navi's template |
| BN6 code, no chip | it descends from code BN6 keeps but no BN6 chip uses (BN5 leftovers in BN6's tables) |

- **The labs' timelines.** BN5's lab recordings are compared with the ancestor's BN6 recordings (hit, miss,
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
- **Checked.** The coverage traps don't change a run. All 1,253 chip recordings of the BN5 lab, recorded with the
  traps and without, are byte-identical. One trap that did change them, on a byte table at an odd address the
  discovery took for a routine, is why an odd literal is only followed to a `push`.

### 14.2 Results

| relation | identical | identical-run | constants | changed | bn5-only | chips |
|---|---|---|---|---|---|---|
| same chip | 14 | 7 | 8 | 93 | | 122 |
| built like | | | 1 | 100 | | 101 |
| BN6 code, no chip | 7 | | | 10 | | 17 |
| bn5-only | | | | | 90 | 90 |

Labs: 199 chips have recordings in both games. Of the 122 same chips, 20 match in every category, 28 in all but
effects' frames, 38 in hits and objects (most of these are dimming chips, §14.3), and 28 differ.

- **Reusable as they are (identical, identical-run):** MiniBomb, EnergBom, MegEnBom; Cannon, HiCannon, M-Cannon;
  PanlGrab, AreaGrab; PnlRetrn, HolyPanl, Snctuary; Invisibl; AntiNavi, AntiDmg, AntiSwrd, AntiRecv; GrabBnsh;
  SloGauge, FstGauge; FullCust; Meteors. For example, MiniBomb's hit is the same in both labs at every event: the
  use, the throw at +1, the bomb at +10, the hit for 50 at +50, the explosion's effect from +49 to +72.
- **Constants only:** AirShot (the arm's value 0x13 → 0x08), Silence, Discord, Timpani (a sprite 0xA → 0x8),
  Mine and GrabRvng (an effect's number), ProtoMan and ProtomnSP (an effect and a sprite).
- **Changed but the same in the labs (22, 16 of them same chips):**
  - same chips: Recov10 to Recov300 and DrkRecov (BN5 also adds the use's +0x0A to NaviStats +0x0E), TankCan1–3,
    Tornado, Static, BugBomb, Navi+20;
  - built like: Guard1–3 (Reflector), Atk+10, Atk+30, DarkTorn.

  The differences there are code the labs ran with no effect on what they record.
- **Different in the labs (28 same chips).** Damage alone doesn't count: HiCannon's 100 is BN5's 80, from the
  record. The causes:
  - the code: Thunder's ball; the Vulcans' last bullets; GunDelSol's actor, which leaves at +19 in BN5 and +80
    in BN6;
  - the labs' set-ups: AirSpin's and AreaGrab's miss is a hit in BN5, and BN6's user loses 1 HP at a time with
    the dark chips, Bass, DeltaRay and BigHook;
  - chance: Meteors.
- **BN5-only (90),** by content key:
  - navi chips: napalmmn, magnetmn, meddy, shadoman, knightmn, larkman, gridman, django (with their -sp and -ds);
  - the e-Reader chips (§14.5): leadraid, chaoslrd;
  - chips BN6 dropped: mrkcan1–3, pulsar1–3, spshake1–3, quake1–3, cannball, slasher, moonbld1–3,
    redfrut1–3, skully1–3, crakout, dublcrak, tripcrak, aqwhirl1–3, sidebub1–3, elcreel1–3, cusvolt1–3,
    crsshld1–3, wavepit, redwave, mudwave, woodnos1–3, hotbody1–3, cacdanc1–3, phoenix, dethphnx;
  - dark and bug chips: jealousy, poltrgst, bugcurse, bugcharg, holydrem, magnum, rainyday, elemrage, copydmg.

### 14.3 What differs for many chips (the library)

32 of the 90 shared routines differ where the labs ran (`chip-library.tsv`).

- **The dimming handler** (action 0x15, BN6 `sub_80EBD9C`, BN5 0x080EC318, 68 chips): BN6 spawns the dimming
  object, stays in the action state and leaves it (`object_exitAttackState`) on its next update, which is after the
  dimming; BN5 spawns it and leaves on the same frame. So in BN5 the user stands during the dimming (the labs'
  `navi`: BN6's user leaves the action at +129 to +164, BN5's at +1).
- **The object handler of action 0x1A** (BN6 `sub_80EC39C`, BN5 0x080EC6F6: Boomer, Lance, FireHit, BusterUp, the
  AtkPlus chips, FullCust, JustCone, the mode chips and others; 32 chips): BN6 calls the subtype's routine once,
  keeps the action 8 more frames for subtype 20 and then leaves; BN5 calls it and leaves on the same frame.
- **The navi chips' framework** (`sub_80E192C`, `sub_80E18F8`, `sub_80E1854`, 45 chips): the same code with the
  games' numbers where it singles chips out: Roll's subtype (BN6 0, BN5 0x19), BigHook's (0x17, 0x1A), AntiRecv's
  id (0xBD, 0x93). `sub_800BA8A` (the navi telop) checks AntiNavi by id (0xBA, 0x90). The navi attacks' helpers
  `sub_80E292C`, `sub_80E28C8` single out navi subtypes BN5 numbers two higher (BN6 6, 7, 8, 10, 11, 12), and BN6's
  `sub_80E28C8` adds, for side 1, a swap of subtypes 17 and 18. `sub_80E376C` (a navi chip's leaving, 4 chips)
  takes the chip's HP cost as BN6 does and also subtracts its +0x2E from NaviStats +0x0E (to at least 1, when not 0)
  through 0x08012820; Recov adds to the same byte through 0x08012802 (§14.2). BN6's counterpart of the subtracting
  one is `sub_8015C12`.
- **The swords** (`sub_80EBAE8`, `sub_80EBB34`, 14 chips): NaviStats +0x2C picks the slash and a count. In BN6 its
  values 11 to 23 (its forms) give slashes 12 or 13, and the count comes from a subtype table. In BN5 souls 1, 7
  and 8 give slashes 13, 14 and 15; the count is 3, or 10 with soul 1 and 50 with soul 7.
- **The shot object** (BN6 attack kind 0, `sub_80C4E7C`, `sub_80C4F02`: Cannon, AirShot, GigaCannon; 7 chips): BN6's
  has variants BN5's doesn't (12: its own handling when the shot leaves the field or hits; 34 and 36: they set the
  panel's type), and the crack and break variants are BN5's 24 and 25 (BN6 21 and 22).
- **The bombs' throw** (`sub_80EB644`, 14 chips): BN6 passes a subtype-dependent parameter in the thrown object's
  top byte (3 for subtype 15, the use's +0x0C × 3 for subtype 14); BN5 passes none.
- **The hit-effect spawner** (`sub_80E33FA`, 26 chips): BN6 sets bit 2 of the spawned effect's flags; BN5 doesn't.
- **The Vulcans' bullet** (`sub_80C6964` to `sub_80C6AB8`, 8 chips): other animation numbers and collision values,
  and BN6 sets a status effect (`object_setCollisionStatusEffect1`) that BN5's doesn't.
- **Bass and BassAnly's shared actor helpers** (`sub_80C4550` to `sub_80C468C`): BN6 loads animations, palettes and
  a check BN5's don't.

### 14.4 The map

One row per BN6 routine that BN5's chips descend from. The columns:

- the BN6 chips and module (content/bn6);
- the BN5 chips, marked `=` (the same chip) or `~` (built like it);
- their classes, counted: I identical, Ir identical-run, C constants, X changed;
- the labs' verdicts;
- where the chips' own code differs, by BN6 routine.

chip-actions.tsv has every region; `actions.py --markdown` writes this table.

| BN6 routine (chips; module) | BN5 chips | code | lab | what differs where it ran |
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
| `sub_802E1BE` (no BN6 chip) | voltz1, voltz2, voltz3, cannmode, cannball-mode, swrdmode, yoyomode, drilmode, finalgun | X3 I6 | none 9 | code: `sub_802E1BE`; library: `sub_80EC39C` |
| `sub_80D2596` (lance; chips/lance) | =lance, ~drklance | X2 | differs 2 | code: `sub_80D2514`; constants: value ×2, sprite ×2; library: `sub_80EC39C` |
| `sub_80EC02A` (yoyo, greatyo; chips/greatyo, chips/yoyo) | =yoyo, =greatyo | X2 | effects 1 none 1 | code: `sub_80CE932`, `sub_80E8612`, `sub_80CEA42`, `sub_80CE9A4` (+5 more); constants: value ×10, branch ×5, literal ×2 |
| `sub_80E3128` (wind, fan; chips/wind) | =wind, =fan | X2 | navi 2 | code: `sub_80CD310`, `sub_80CD414`, `sub_80CD236`; constants: sprite ×2; library: `sub_80EBD9C` |
| `sub_80E46B6` (rockcube, icecube; chips/rockcube) | ~boybomb1, ~boybomb2, ~boybomb3, =rockcube, ~omegarkt | X5 | differs 4 navi 1 | code: `sub_80E4678`, `sub_80CFB2C`; constants: value ×4, branch ×2, offset ×2, literal ×1, sprite ×1; library: `sub_80EBD9C` |
| `sub_80ED13E` (rflectr1, rflectr2, rflectr3; chips/rflectr) | ~guard1, ~guard2, ~guard3 | X3 | same 3 | code: `sub_80ED154`; constants: branch ×3; library: `object_genericDestroy`, `sub_80C4FFE` |
| `sub_80E64E8` (colorpt, dblpoint; chips/colorpt) | ~metagel, =colorpt, =dblpoint, ~blakwing | X4 | differs 2 navi 2 | code: `object_dimScreen`, `sub_80E667C`; constants: offset ×2; library: `sub_80EBD9C` |
| `sub_80E59C6` (snake; chips/snake) | =snake | X1 | differs 1 | code: `sub_80D2C40`, `sub_80D3048`, `sub_80D2EDC`; constants: value ×4, branch ×2, offset ×1, literal ×1; library: `sub_80EBD9C` |
| `sub_80E7600` (circgun; chips/circgun) | =circgun, ~darkcirc, ~piledrvr | X3 | differs 2 none 1 | code: `sub_80E75AC`, `sub_80D65FC`, `sub_80D6580`, `sub_80D677C` (+1 more); constants: sprite ×6, offset ×6, branch ×5, value ×4; library: `sub_80EBD9C` |
| `sub_80E349E` (mine; chips/mine) | =mine, ~bodygrd | C1 X1 | navi 1 none 1 | code: `sub_80E3470`; constants: value ×2, offset ×1, branch ×1; library: `sub_80EBD9C` |
| `sub_80E5A64` (no BN6 chip) | astroid1, astroid2, astroid3, darkmetr, boxer1, boxer2, boxer3 | X7 | none 7 | code: `sub_80E5A22`, `sub_80E5A08`, `sub_80E5A64`; constants: value ×15, offset ×8, branch ×3; library: `sub_80CF3DC`, `sub_80EC39C` |
| `sub_80EC844` (recov10, recov30, recov50, recov80, recov120, recov150, …; chips/drkrecov, chips/recov) | =recov10, =recov30, =recov50, =recov80, =recov120, =recov150, =recov200, =recov300, =drkrecov | X9 | same 9 | code: `sub_80EC844`; constants: branch ×9 |
| `sub_8010820` (busterup; chips/busterup) | =busterup | X1 | effects 1 | code: `sub_8010820`; constants: value ×1; library: `sub_80EC39C` |
| `sub_80E07E0` (panlgrab, areagrab; chips/areagrab, chips/panlgrab) | =panlgrab, =areagrab | I2 | navi 1 differs 1 | library: `sub_80EBD9C` |
| `sub_80E2D76` (grabbnsh, grabrvng; chips/grabbnsh) | =grabrvng, =grabbnsh | C1 Ir1 | navi 2 | constants: value ×1; library: `sub_80EBD9C` |
| `sub_80E24B8` (slogauge, fstgauge; chips/fstgauge, chips/slogauge) | =slogauge, =fstgauge | Ir2 | navi 2 | library: `sub_80EBD9C` |
| `sub_80E2B5A` (pnlretrn, holypanl, snctuary, comingrd, goingrd; chips/comingrd, chips/goingrd, chips/holypanl, chips/pnlretrn, chips/snctuary) | =pnlretrn, =holypanl, =snctuary, ~elempowr | I3 X1 | navi 4 | code: `sub_80E2B2C`, `sub_80E2B5A`; constants: value ×1, offset ×1, branch ×1; library: `sub_80E28C8`, `sub_80E292C`, `sub_80EBD9C` |
| `sub_80E2566` (geddon, prpcapsl, pnkcapsl, healball, magpanl, beastout-dimming-1, …; chips/geddon) | ~geddon1, ~geddon2, ~geddon3 | X3 | navi 2 differs 1 | code: `sub_80E2528`; library: `sub_80EBD9C` |
| `sub_80E2F24` (no BN6 chip) | blinder | I1 | none 1 | library: `sub_80EBD9C` |
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

Both summon through the navi chips' action (§6.4), the same handler and framework as BN6's navi chips
(`sub_80EC350`, `sub_80E192C`; §14.3).

**LeadRaid → BN6's TwinLdrs (0x15C).** LeadRaid (0x137, summon entry 0x15) spawns actor kind 0x20. BN6's TwinLdrs
(summon entry 20, `sub_80BD9A2`) spawns kind 0x20 too, whose handler `sub_80BD388` is LeadRaid's kind 0x20 handler
(0x080BE1C8) verbatim. The routine map pairs LeadRaid's other routines with TwinLdrs's (`sub_80BD3AC`,
`sub_80BD478`, `sub_80BD644` and the rest). It is TwinLdrs's ancestor, reworked:

- **BN5: two actors, one strike each.**
  - Kind 0x20 is ProtoMan: sprite 0x1030801, offset (10, 6) pixels ahead of the user.
  - It spawns its partner, kind 0x22, Colonel (0x080BED4C): sprite 0x1030807, offset (10, 6) pixels behind.
  - Each appears with sound 0x94. ProtoMan spawns an effect (effect kind 0x85, 0x080EA198) at frame 10.
  - At frame 55 each picks a target: the first panel ahead in its own row holding an opponent (0x080BE4F0); failing
    that, the first column with one in any row (BN6's `sub_80BE434`'s counterpart).
  - Each moves to the panel before its target and strikes once (0x080BE364):
    - animation 5, the slash actor (kind 5), sound 0xB0;
    - at frame 12, a hit region `0x0705FF04` on the panel ahead, the hit effect (variant 0x16), a shake (3, 10);
    - the two hand over through +0x60 (`sub_80BE734`).
  - The lab: two hits of 200 wherever the opponent stands.
- **BN6: one kind for both.** Kind 0x20 serves both leaders, its +4 choosing ProtoMan (0) or Colonel (1) from a
  table of sprites and offsets (0x080BD464). It appears with a white flash (`sub_80BD4DC`).
  - ProtoMan slashes up to twelve times (counter +6). He searches every row for a target and moves to each in turn
    (`sub_80BD5A8`). Each slash puts hit region `0x0405FF04` on the panel ahead, the hit effect (variant 0x27)
    and a shake (1, 10).
  - Colonel's finishing strike (`sub_80BD8BC`): two hit regions (`0x0405FF12`, `0x0405FF13`), effects 0x36 and
    0x37, sound 0xC7, a shake (3, 35).
  - A screen effect (`sub_80BDB04`: effects 0x4E and 0x4F, sounds 0x71 and 0x72).
- **BN6 keeps BN5's Colonel actor** (kind 0x22, `sub_80BE4D8`) and its spawner (`sub_80BE6D8`), called only from
  `sub_8114FB8`, outside the chips.

So a port writes LeadRaid's own module. TwinLdrs (content/bn6/chips/twinldrs) shares the navi-chip framework, the
target search's shape and the slash actor (kind 5, `sub_80B8E30`), not the behavior. The map's walk finds TwinLdrs
0.19 alike; there is no TwinLdrs lab to compare with.

**ChaosLrd: BN5's own.** ChaosLrd (0x138, summon entry 0x1D) spawns actor kind 0x51. Its code is BN5's: the
routine map pairs none of its states with BN6 code, and no BN6 summon is as like as 0.5. BN6's kind 0x51 is
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

The helpers are BN6's: the aura's spawner and the burst's are at the same places as BN6's leftover asteroid code
(`sub_80E5A64`, BN5's Asteroid chips', §14.4). BN6's BassAnly (kind 0x50, the next kind) shares nothing with it
beyond the framework. A port writes ChaosLrd's module from the BN5 code.

### 14.6 For the port

- **identical, identical-run:** BN6's module (`require("@bn6/chips/<module>")`) with BN5's record (content/bn5),
  assets and sprites.
- **constants:** the same, the listed constants as parameters (sprite numbers come with BN5's assets; effect numbers
  map through BN5's kinds).
- **changed:** start from BN6's module and apply the regions chip-actions.tsv lists. The labs say which matter: the
  22 changed chips whose timelines match differ in code the labs didn't see change anything.
- **The library's differences (§14.3)** are the engine's or a shared module's, once: the dimming handler's and
  action 0x1A's leaving the action on the use frame, the navi chips' numbers, the swords' souls, the shot's and the
  bombs' variants.
- **bn5-only and BN6 code, no chip:** new modules, from the BN5 code. For the latter BN6 keeps the code (Voltz, the
  mode chips, FinalGun, the Asteroid chips, DarkMetr, Boxer, Blinder), so the BN6 disassembly reads for BN5 there.
- **built like:** the nearest BN6 module as a template only (a navi chip's template, not its moves).

## 15. The BN5 port

BN5 as a second game beside BN6 (rules-in-luau.md R2: the arena's game supplies the field, the hit tables and the
flow's banners and music; each side's game its navi, sounds and custom screen). In order: BN5's battle data
(§15.1 to §15.4), the replay harness in the verification workspace (§15.5), the chips (§15.6).

### 15.1 BN5's battle data (as built)

content/bn5/rules is written once by the verification workspace's `tools/bn5/gen_rules.py` (through
`gen_content.py write`, which leaves a written rules file alone; `check` notes where one differs since), from Team
ProtoMan's ROM; Team Colonel's tables are the same. Each file names its sources.

| File | What | BN5's against BN6's |
|---|---|---|
| ruleset.luau | the stock ruleset `bn5:bn5` | no systems yet (Soul Unison, Chaos Unison, BN5's emotions, the Team Battle come as systems) |
| roles.luau | roles naming BN5's assets, its collision types, the anchor region | the rest unfilled (below) |
| pools.luau | 16 actors, 32 attacks, 32 effects | BN6's 32 each |
| elements.luau | weakness (0x08016900) | BN6's; no family adds an element (BN5 stores a chip's element as its record has it) |
| reactions.luau | push rows (0x0800CA24), ice slides (0x0800C988), the bubble's bob | five push rows read another way (§15.3); no bubble |
| math.luau | sine (0x08005CD0) | BN6's |
| buster.luau | recovery by Rapid and open panels (0x0801CEA4); the empty hand's chip | BN6's |
| banners.luau | holding banners: program-advance, hit-damage-judge, program-advance-empty | BN6's three; the 49 records (0x0801B810) match BN6's first 0x5D |
| custom-screen.luau | the slot grid and the neighbor fix-up's lists | BN6's |
| status.luau | 64 statuses: paralysis, confusion, blindness, immobilization by level (0x0801CEC4); the HP bug's periods | no freeze, no bubble; the timers 2 bytes further in the collision record |
| collision.luau | 77 of BN6's collision types by their rows (0x0801636C) | 80 rows to BN6's 89; no 0x80 self bit; row 0x3D gives its sides' bits |
| panels.luau | the 11 panel types (unregistered data, §15.2) | metal and sea; no roads |

content/bn5/compat gains `assets.toml` (§15.4) and `rules.toml` (the statuses' bytes, 0x10 to 0x4F); bn5-compat
reads both (`Compat::assets`, `sprite_names`, `status`), and bn5-extract names the pack's assets from them. The
root requires BN6's (`requires = ["bn6"]`): BN5's definitions use BN6's where BN5's code is BN6's
(`require("@bn6/...")`). nettai-content's lint test `bn5s_rules_are_its_games` loads BN5's rules beside BN6's (its
chips left out until they have uses) and checks they are BN5's game's: 16 actors, BN6's weakness.

**Roles not filled** (a battle stops at the first it needs, naming it): every action, kind, chip, status,
lock-on, effect, spark and hook role (the port's BN5 definitions); the sprites but the charge glow (BN5's status
visuals are BN5's sprites, named when the port draws them); the freeze and bubble sounds and sprites (BN5 has
neither status); the eruption's (BN5's lava doesn't erupt); BN6's own (the A charge's glow, Full Synchro's aura,
the Cross and Beast Out sounds, the custom screen's Cross window, re-deal and scrap).

### 15.2 The field

BN5's 11 panel types (0x0800BCF0): missing, broken, normal, cracked, poison (BN6's flags), 5 (flag word 0x10210,
BN6's road panels', a plate's look: compat calls it metal), grass, ice, lava (BN6's volcano flags), holy, sea
(0x30010). The trail sounds (0x080113A0) are BN6's for the shared types, `immobilizer` for type 5, `sand-worm`'s
song for sea. Start grid and front edges, step and dash-step rules are BN6's; BN5 has no either-side step rule.
What the panels do (BN6's `sub_800C380`, BN5's 0x0800A998, and the routines named):

- a broken panel mends after 600 ticks in every battle (BN6: 0x1E0 in battle mode 1), blinking its last 60;
  cracked panels break as BN6's;
- lava and sea panels turn normal after 960 ticks (their own timers, +0x10 and +0x14 of BN5's 0x24-byte panel
  record), blinking their last 60; nothing erupts (no volcano counter);
- lava (0x08016D80, 0x08016E18, BN5's own): a grounded body, not of fire, not floating and not flagged 0x88000206,
  takes 50 (shifted by its weakness to fire) as a hit (flags 3 at +0x0F, +0x18, +0x19), unless flagged 9; the
  panel turns normal and a burst shows; a navi whose soul byte (NaviStats +0x2C) is 4 standing on lava with a chip
  whose +0x09 has bit 2 gains 10 on it, and the lava goes (0x08012602);
- sea: drains a fire body as poison drains any (0x08016C7E, BN6's `sub_801A186`); at a move's end on sea, a body
  not floating, not aqua and without bit 0x20 of its AI record's flags (+0x48, `sub_801032C`) stops for 20 ticks
  (`sub_800EB18`'s timer, BN5's at +0x24 of the collision record) with a splash (effect 99) (0x0801715E); a body
  on sea with that bit has its collision record's +0x2C set to 0xFFFF, else 0 (0x08017030);
- type 5: at a move's end, a slide (type 3) unless the body has slid within the cooldown (+0x38) or is a navi
  whose soul byte is 5 (0x08017216, BN6's road start `sub_801A400`); BN5's type-3 slide tries four steps in an
  order the move's direction picks (0x0800C8A8's branch, the tables at 0x0800C920 and 0x0800C9C0), not a road's
  fixed direction: after a move forward, down first (then forward, up, back); after a move up, forward first;
  after a move down, back first. Recorded (the chip lab's `stages/panel5-row/slide-*`, the middle row's inner
  four metal): forward from (1,2) onto (2,2) slides down to (2,3); down from (2,1) onto (2,2) slides back to
  (1,2); up from (2,3) onto (2,2) slides forward to (3,2), and from there (metal again) down to (3,3). A slide
  step up or down covers the 24 pixels in 3 ticks (8 a tick);
- conversions (0x08016D14, BN6's `sub_3007708` moved out of IWRAM): fire on grass, aqua on lava, element 4 on
  type 5 turn the panel normal (BN6's roads take element 4 too);
- battle effect 0x1000 (single player only) hands panel runs over (0x0800AE92, BN5's own).

### 15.3 What the engine has no slot for (proposals)

The smallest engine additions BN5's data and rules need, for the rules agent (none made here):

1. **Panel types.** `PanelType` gains BN5's three: `Metal` (type 5), `Lava` (type 8: not BN6's volcano, whose
   flags and sound it shares but not its behavior) and `Sea` (type 10); the `panels` section names the types its
   game has, not exactly all of the engine's (BN6's names its 13, BN5's its 11). Their behavior as section data
   where it is numbers: per type `expires` (ticks to normal: BN5's lava and sea 960; BN6's roads their road
   timer) and the broken panel's mend time per game (BN5 600 always); the rest as code the types select (lava's
   burn, sea's drain and stop, type 5's slide), keyed by the panel type, not the game. bn5-compat then maps 5, 8
   and 10 to them (today: none, volcano, none).
2. **Push.** BN5's push reads the first of bits 2 to 5 of the hit modifier's first byte (+0x18), else of its
   second (+0x19) with the direction reversed (0x0800C9D8), five rows; BN6's reads bits 2 to 5 and adds 5 for 0x80
   (`sub_800E548`). A choice in the `reactions` section (`push = { rows, reading = "bn5" }`), the engine reading
   both ways.
3. **Slide type 3** reads the panel's road direction in BN6 and the move direction with fallbacks in BN5 (§15.2):
   the same choice, or type 5's code in item 1.
4. **Statuses BN5 lacks.** The engine's aqua-on-ice freeze and the encased-obstacle hook ask the arena's roles
   `statuses.ice_freeze` and `hooks.encased`: BN5's arena must answer "none" without a panic (optional roles, or
   the reactions section saying the game has no freeze).
5. **Chip families** recovery and invisible (§13): `ChipFamily` gains them; BN5's chips name them, and the
   systems that count families (AntiRecv's) read them.
6. **NaviStats +0x0E** (BN5's mood byte, §13): a recovery adds its +0x0A to it (0x08012802, cap 254, not from 0 or
   0xFF); a navi chip's leaving subtracts its +0x2E (0x08012820, floor 1). A field of BN5's navi record and a
   hook on recovery and on a navi chip's leaving (or the emotion system's, when BN5's comes).
7. **The flow** (§15.4's table): a `flow` section for its numbers (the result waits), and the arena's game
   choosing the flow's code where BN5's differs (the state machine's structure, below).
8. **Mixes with their own sections** (rules-in-luau.md R2): Soul Unison's custom-screen layout (the shared custom
   screen's soul row) is a mix's; needed when the Team Battle's custom screen is ported.
9. **Collision words:** BN6's chips required from BN5 register BN6's collision types (with the 0x80 self bit,
   rows past BN5's 0x50): fine while every target word lacks 0x80, which the port checks per row it uses (BN6's
   `pull` and `probe` rows test it; BN5's chips pass BN5's types where BN6's modules take them).
10. **Leaving the action on the use frame** (§14.3): BN5's dimming handler (action 0x15) and action 0x1A's object
   handler leave the action on the frame they run; BN6's on the next update after the dimming, and 8 frames
   later for subtype 20. A choice per game in the engine's chip use (a rules section's flag), read by every
   dimming and instant chip.
11. **AntiNavi's sparkle** (`sub_800ABC6`, BN5's 0x080093A2, dimming.rs's `SPARKLE_DY`, `SPARKLE_Z`): BN5's sits
   on the panel's center 16 pixels up, BN6's 16 pixels down the field and 32 up. Numbers for a rules section (or
   the trap mark role's offset).
12. **The hit test** (built: the reactions section's `hit_test = "bn5"`): BN5's (0x0801691C, ROM code where BN6's
   `sub_3007218` is IWRAM's) counts a bubbled body (flag1 0x80000000) as submerged, elec reaching either; has no
   FloatShoe test (BN6's: flag1 0x20 meets only the 0x80 self bit), nor its raw channel (0x08017494, BN6's
   `sub_3007692`); breaks a guard with 0x1002 always (BN6's 0x0002 but with 0x4000) and turns aside what lacks
   0x0C004000 (BN6's 0x0C005000); and counts elec on its sea once more, as fire on grass (0x08016AF6).

### 15.4 The flow and the assets' names

**The flow.** Of the BN6 flow routines the engine cites (battle.rs, hud.rs, dimming.rs), 73 are BN5's the same
code, 6 the same but for constants, 34 similar, 1 differs and 8 absent. What differs, by the BN6 routine:

- `sub_800825A` (the result): a special battle's result wait 65 ticks (BN6 94); the normal win's 102 as BN6's;
  `sub_80081A4` plays BN5's own winner songs at the same numbers (0x19 special, 0x1F).
- `sub_80080D2` (fighting): no Cross-special check (BN6 +0x3A, `sub_800AAD6` absent) and no per-player gauge
  decrement (0x2900) on a custom request; BN5 calls 0x08025ED0 there.
- `sub_8008452`, `sub_8008492`: after the reversions BN5 opens the custom screen (result 6) directly: no
  transformation sequencer re-run (BN6's state 0x24 is absent), and no mode-5 test.
- `sub_800840C`, `sub_8009158`, `sub_8009338`, `sub_80102AC`, `sub_8013FD0`, `sub_8007EB8`: BN5 adds tests of
  battle flag 0x40 (`sub_800A8F8`, set in link battles) around single-player steps; `sub_8009338` calls the
  custom screen's 0x08022C5C/0x08022D70 there (BN6's `sub_8026840` is absent).
- `sub_8007CA0` (round end): BN5 writes the light/dark value back (NaviStats +0x44, under battle effect 0x800)
  and three BN5 counters (0x0801289C, 0x0801288E, 0x0801299E); constants not read yet at 0x080070CE (5, BN6 1)
  and 0x080070EA (16 and 217, BN6 23 and 51).
- `sub_8017AB4` (a side's dimming): 65 where BN6 tests 27, 120 for 128 (0x08014574, 0x080145CA), not read yet.
- `sub_80107D4` (the navi's tick): BN5 counts down four timers at +0x3C and an invulnerability timer at +0x16,
  and runs the lava-chip boost (0x08012602).
- `sub_800A1D0`, `sub_800A244`, `sub_801C840`, `sub_802E112`, `sub_802F068`: BN6's Beast/Cross and battle-mode-6
  tests BN5 hasn't; `sub_802E4E4` (SELECT's special) differs in its flags (BN5 0x200000, BN6 0x20000000).
- `sub_800BA8A`, `sub_800BDD0`, `sub_800BE2C` (the telop and AntiNavi): AntiNavi is chip 144 (BN6 186): content's
  keys cover it.
- `sub_800B3D8`, `sub_802D7A0`, `sub_802D9B0`: record sizes (96 for 100, 129 for 161), not timings.

**The assets' names** (content/bn5/compat/assets.toml, `gen_content.py`'s `asset_names`): BN6's name for what is
BN6's. A song whose data is BN6's song of the same number (227 of BN5's, sound effects and jingles); BN5's own
music by the place BN5's code starts it where BN6's same code starts a named one (virus-battle 0x15, loser 0x1A,
game-over 0x1B, transmission 0x0F); a sound BN5's identical code plays where BN6's plays a named one; the songs
read at their places (own-hit 0x6B and hit 0x6D in `applyDamageToPlayer_801ba12`'s counterpart, winner-0 0x19
and winner-1 0x1F in `sub_80081A4`'s); a battle sprite whose archive is a BN6 sprite's, or that identical code
loads where BN6's loads a named one (48); the banners whose records are BN6's (24 named). 149 sounds, 48
sprites, 24 banners in all; the rest keep their placeholders, which content may not use.

### 15.5 The replay harness (as built)

bn5-compat's `trace` (feature `trace`) replays a BN5 recording as bn6-compat's does a BN6 one: `rounds` splits a
recording at its setup lines; `Round::needs` lists what its setup needs that the content doesn't define (every
chip of its folders and hands by compat key, each side's navi by BN5's number, a soul, the stage by its settings
bytes, the weapons, programs and first barrier its NaviStats name); `Round::round_setup` builds the engine's
`RoundSetup` once nothing is missing (§15.7: the stage and background by the settings record, both NaviStats,
the folders, the RNGs, the set's score, both players on BN5's stock rules) and `Round::start` the battle;
`run_round` ticks each battle frame with the recorded inputs and compares (`compare`: the state machine, ticks,
the simulation RNG, pause, the gauge, the panels by BN5's numbers, the objects by pool, BN5's kind number
(compat kinds.toml), panel, side, HP and position, a position the kind leaves as register garbage skipped as
bn6-compat skips it), stopping at the setup, a panic or the first difference.

The verification workspace's `trace-tests` runs it over the BN5 lab (`--test bn5_replay`, ignored: a report) on
BN5's root (which requires BN6's) and both games' packs (R3a), BN5's chips without a use left out
(`trace_tests::bn5_content`), and writes replay-summary.md beside the recordings: each recording's stage (read,
decoded, setup, replay, matched), its frames matched, what stopped it, and what the recordings need most. First run
(2026-10-02, the 1,376 recordings, 946,555 battle frames): every one stops at its setup, needing BN5's MegaMan
(`bn5:megaman`), a stage (the lab's is one settings record but for 64 recordings) and its chips (Boomer and Cannon
are in every recording's folder or hand, the lab's filler). With the chips of §15.6, Boomer remains in every
recording and Recov10 in 108.

With BN5's MegaMan, its stages and Boomer (§15.7): 115 recordings replay, 1,261 stop at their setup; 25,170 of the
946,555 battle frames match. Every replay matches its first 219 battle frames (the intro, the first turn, the
custom screen until both results are in) and differs at frame 361, the tick both are in: BN5's custom screen mode
(BN6's `sub_8009338`, BN5's 0x08007F50) runs the Team Battle's own routine (0x08025EF2, in place of BN6's
`sub_8026A28`), which ends the screen on that tick (mode 0xC), where BN6's ends it on the next (the engine's
`CustomScreens::committed`); BN5's also doesn't set the navis' AIData +0x0F as BN6's does. A choice per game in the
engine's custom screen (§15.3 item 13).
What the setups need most: Recov10 (108: BN5's recovery family, §15.3 item 5), WideSht1 (18), BlkBomb and Thunder
(15 each), HolyDrem (12), Sword (11), then the chips each scenario tests.

With the chips of §15.6's third batch, BN5's metal, lava and sea stages and the rules work's P1a (its custom screen
end, item 13; the families; the panel types), on 2026-10-02 (1,380 recordings: the lab's and four metal slides):
252 replay, 1,128 stop at their setup; 24 match every frame and 79,701 of the 948,097 battle frames match. 191 of
the 228 that differ stop at frame 426, the fight's first tick, on the panels: Team Colonel's MegaMan is dark (his
save's light/dark value 0) and stands on the default stage's holy middle row, which BN5 turns Normal under him
(§6.1, item 14). The rest: a spark a tick short (18, item 16), the camera shake's draws (7, item 15), a custom
screen opened a tick late (12, item 18). With those four built in a scratch copy of the engine (not committed: the
rules work's), 228 recordings match every frame and 115,999 frames match; the 24 that still differ are HolyDrem's
(item 18), DrkRecov's (its dark chip cost, unread: §6), the souls' (not ported) and the metal slides' speed
(item 17).

### 15.6 The chips (in progress)

From the action map (§14.6), first the chips whose code is BN6's (identical, identical-run), then those that
differ in constants only. Each is BN5's chip file (its record, generated) with a use from BN6's modules
(`require("@bn6/...")`), BN5's collision types where BN6's modules take them; content/bn5/objects/projectile
holds BN5's projectile variants (BN6's rows with BN5's collision). Until R3, the modules draw and time from BN6's
pack (the assets are BN6's where the names are BN6's, §15.4).

**Ported** (26): Cannon, HiCannon, M-Cannon (BN6's cannon, BN5's shot variant: the variant rows are the same);
MiniBomb, EnergBom, MegEnBom (BN6's throw and bomb, the energy burst BN6's object); PanlGrab, AreaGrab; GrabBnsh
and GrabRvng (BN6's controller; GrabRvng's hand effect is BN5's 0x29, the same as BN6's 0x3B); SloGauge,
FstGauge; PnlRetrn, HolyPanl, Snctuary; AntiNavi, AntiSwrd, AntiRecv (BN6's traps; the roles of their counters
and mark are BN6's, §15.1); FullCust (its fill is BN5's: with each side's own gauge, the side's goes full where
BN6 adds a third). The roles `actions.anti_damage_counter`, `anti_sword_counter`, `kinds.anti_recovery` and
`effects.trap_mark` (BN5's effect 43 is BN6's 0x46) are filled for the traps. Then, with BN6's modules taking
BN5's numbers (the user's decision, 2026-10-02: BN6's chip modules may change where BN5 can't reuse them as they
are, the smallest change, BN6 staying the same): Silence, Discord and Timpani (BN6's instrument with BN5's 100 HP,
play and pause ticks, sprite and actor records; the effects BN6's with BN5's statuses and collision types);
AirShot (BN6's AirShot with BN5's shooter look and shot, row 4 without the wind element); ProtoMan (BN6's navi
with BN5's sprite, sword animation, slash effect 0x33 and spawner); Boomer (BN6's boomerang with BN5's variant:
half BN6's speeds, no grass, BN5's sprite).

**Third batch** (2026-10-02): WideSht1 to WideSht3 (BN6's wide shot with BN5's shooter, the wave at the floor and
BN5's wave sprite; WideSht2 and 3 fire wave kinds 4 and 5, their records' parameter), BlkBomb (BN6's bomb with
BN5's leaving, 0x080CE594, its intake BN6's `sub_801ADFA`'s where BN6's is `sub_801AD12`, its spark and
collision), Thunder (BN6's ball with BN5's look: sprites 10-11 and 0c-53, animation 0, collision row 0x0A, hit
modifier 0, a 12-panel ball's row 0x2B, one sound), Sword, WideSwrd and LongSwrd (BN6's slash with BN5's parts,
lib/swords: the blade, the effects 0x16 to 0x18 and BN5's slash type), Recov10 to Recov300 and DrkRecov (BN6's
heal, the mood the record's parameter, only when no AntiRecv turned the heal), HolyDrem (its own: BN6's BurnSqr
controller's code, a stand-in that fires a shot along the row and one more for each holy panel it turns Normal,
and the shot, attack object 0x83), Invisibl, AntiDmg and Mine (BN6's, now the engine has the invisible family).
Fixed: Boomer's variant is the record's row 5 (BN6's Boomer's speeds), not row 0. BN5's own hit sparks
(lib/sparks: BN6's rows with BN5's sprites, whose animations are a frame longer) for its roles, shots and chips;
the status visuals' sprites, a lava burn's spark and a sea splash (effect 0x63) in its roles; AntiSwrd's counter
throws BN5's sonic boom (at the floor, palette 9 going through, else by the whole element byte, 0x080D0E64).
Kinds.toml gives the BN5 numbers of every kind the ported chips spawn, BN6's included. bn5-extract writes BN5's
banners (49, the same layout as BN6's: 0x0801B810), their digits and palette, and "Cstmzing..." (the audit's 24
problems). Waiting in this batch: DrkRecov's dark chip cost (HP bug: BN5's own code, unread, §6), HolyDrem's light
MegaMan (§6.1), the swing's call BN6 stubs out (0x080E9FD2, battle flag 0x40: never in a netbattle).

**Shared code (common-shared, 2026-10-03; the user's direction: shared BN5/BN6 behavior in content/common).** The
BN6 modules the lists below opened for BN5 now live in content/common as makers that take a game's look (none holds a
game's ids or assets; rules-in-luau.md §7.2), BN6's modules at their old paths their BN6 wrappers, BN5's its own
definitions made with them: the regions, panels, slot, element, trajectory (over a game's sine table), dimming and
its stand-in, the recovery heal, the attachment, the buster's parts, the projectile and its firing, the cannons,
AirShot, the bombs' throw and bomb, the energy burst, BlkBomb, the panel bursts, the rising bubble, the panel
changer and the panel chips', trap chips', gauge chips', grab chips', GrabBnsh's, Invisibl's, Mine's, AntiRecv's,
CircGun's, Meteors', TimeBom's and the instruments' controllers with their objects (the grab shot, the hand, the
mine, the gun and shot, the falling meteor and its marker, the countdown bomb, the instrument and its effects),
AntiDmg's counter and shuriken, the swords' parts and slash, the sonic boom, AntiSwrd's counter, the wide shot and
wave, the bullet and the vulcans, the tornado and its blow, Thunder's ball and shot, FireHit's fist, ElemTrap's trap
and counterattack, Lance's lance, DrilArm's drill, ProtoMan, the boomerang, the plus chips' sparkle, the Spreaders'
action, the shower's aim, and MegaMan's buster, blank and charged shots. BN5 makes its own kinds of each (compat's
kinds.toml names them `bn5:...`); a BN5 look names BN6's assets where BN6's module did, as before. BN5 still uses
these BN6 definitions as its own: BN6's effects (lib/effects: the explosions, puffs and flashes BN6's modules
showed), BN6's collision rows that keep the 0x80 self bit where BN5's own rows drop it (thrown, curse, thrown-slash,
attack, slash: BN5's grab shot, energy burst, GrabBnsh's hand, AntiDmg's shuriken, ProtoMan's slash and the
projectile's burst; where BN5's row equals BN6's, BN5's own), BN6's plain shot as the fallback of BN5's forced
charged shot, and BN6's barriers and their visual (lib/barriers: waits on bn5-chips-b's content/common/barriers).
The 0x80 bit and the fallback are as BN6's modules had them; whether BN5's rows are the right ones is the chips'
porters' to check.

The lists below say how each was opened (each the smallest change that lets BN5 reuse it; BN6's behavior the same,
its full set run on the batch):

- lib/instruments/instrument.luau (and types.d.luau's `Instrument`): optional `hp` and `sprite`. BN5's
  instruments have 100 HP and their own sprite, where the library had 60 and BN6's sprite as constants.
- lib/instruments/effects.luau, new: Discord's, Timpani's and Silence's effects, taking their status and
  collision types. They were local functions of BN6's chip files, which BN5's chips can't reach, and BN5's give
  BN5's statuses.
- lib/airshot.luau, new (and `AirShotSpec`): AirShot's action, taking the shooter and the shot. It was the BN6
  chip file's own; BN5's AirShot fires another variant.
- chips/protoman/navi.luau (and `ProtoManLook`): his look (`protoman.look`, `summon_with`), BN6's as
  `protoman.bn6`. BN5's ProtoMan has its own sprite, sword animation, slash effect and spawner address, which
  were constants.
- chips/boomer/boomerang.luau: a variant's optional `sprite`. BN5's boomerang is its own sprite.
- navis/megaman/weapons/buster/weapon.luau: `buster.pick` exported (was local). BN5's buster picks blank and
  charged shots by BN6's code but sets up its own shot.

Third batch:

- chips/widesht/action.luau and wave.luau (and `WaveVariant`): `widesht.action` exported (was local) with an
  optional shooter and wave height; a wave variant's optional `sprite`. BN5's shooter is another attachment row,
  its wave at the floor and its own sprite.
- chips/blkbomb/bomb.luau: `black_bomb.make_kind(id, look)` (the destroyed action, the spark, its and its burst's
  collision types, the push mode), `finish` exported, the thrower taking a kind. BN5's bomb leaves the field
  otherwise, takes hits through another routine and has BN5's spark.
- chips/thunder/ball.luau and shoot.luau (and `ThunderBallLook`): `ball.make_kind(id, look)` (sprites,
  animations, collision types, hit modifier, spark, sounds), the spawn and the action taking a kind. BN5's ball
  differs in each of those.
- chips/boomer/boomerang.luau: a variant's optional collision type and spark. BN5's are its own.
- lib/swords/sonic_boom.luau: `sonic_boom.make_kind(id, look)` (sprite, height, palettes, collision type), the
  throw taking a kind; chips/antiswrd/counter.luau: `action_with` taking the booms' kind. BN5's AntiSwrd counter
  throws BN5's boom.
- chips/recov/heal.luau: the mood only when the heal's result says no AntiRecv turned it (BN5's 0x080EC484 tests
  it; BN6 passes no mood).

**Waiting:**

- *On BN5's dark chip costs* (§6): DrkRecov's use leaves its user losing 1 HP every 10 ticks (BN6's
  `sub_800B79A`, the dark chips' HP bug, is absent in BN5; BN5's own is unread).
- *On BN5's damage formulas* (§13): ProtoMn SP and DS (BN5's records name formula rows 2 and 24, which BN5's
  table, not BN6's SP times, gives); their use is ProtoMan's once it is.
- *On the engine* (§15.3 items 10 and 11): every ported dimming chip and FullCust leave the action as BN6 does
  until the engine reads BN5's choice; AntiNavi's sparkle sits where BN6's does.
- *Without a BN6 chip* (§14.4, `bn6 code, no chip`): Blinder, the mode chips, FinalGun: new modules from the
  shared code.

**Fifth batch** (2026-10-02, bn5-port-5): BusterUp, Attck+10 and +30 (lib/plus), FireHit1 to 3, Vulcan1 to 3,
Tornado, AntiFire/Aqua/Elec/Wood, CrakOut and its family, the dark chips DarkThnd, DrkSword, DarkTorn, DarkWide,
DarkCirc (CircGun's kinds of BN5's look, its variant period 2, 7 shots, look 1), DrkLance (with Lance: BN6's
lance of BN5's look, hit modifier 0x10, Param1 1's row 0x25 and bug 0x16), DarkMetr (with Meteors: the falling
meteor's rows, 2's hit modifier 0, 3 cracking, 4 DarkMetr's breaking with bug 0x19; BN5's shower with its dark
parameter), DarkDril (with DrilArm1 to 3: BN6's drills of BN5's look, no wait outside flag 0x40, DarkDril's 60
ticks and bug 0xFA), DrkSonic (with Fanfare and BN5's own instruments: row 4, its effect paralyzing each enemy
where it stands), DarkPlus (its damage the next damaging chip's Atk+ bonus, the dark tint AIData+0x3C); the dark
chips' rule and costs (rules/light-dark: `battle.set_side_stat`, the mood, `battle.no_dark_chips`); TimeBom1 to 3
and TimeBom+'s use (BN6's TimeBom with BN5's placement: a random row's frontmost enemy panel, 0x080E3420; the
bombs' identities of AI index 0x21, BN5's field objects' index: actor records 0xD8 to 0xE1 all have it, and nothing
of BN5's reads it but the record).

BN6's modules changed for BN5 in the fifth batch, each a kind of its look (BN6 the same): chips/firehit/fist,
objects/bullet, chips/vulcan/action, chips/tornado, chips/elemtrap (trap and strike), chips/widesht/wave (its
trail), chips/timebom (countdown.make_kind, controller.make), chips/circgun (shot, gun, controller), chips/lance
(make_kind, instant_of), chips/meteors (falling_meteor.make_kind with its rows by Param1; the controller moved to
controller.luau), lib/instant/meteor_shower (`pick`, `aim` exported; BN6's three-drop row 2), chips/drilarm/drill
(make_kind returning its spawner), lib/instruments (instrument.make_kind, instruments.make, `effect_period`),
lib/swords (a slash's `blade_anim`; parts.hold's anim).

**Waiting:** LarkMan, GridMan and their SP and DS (BN5's own navis, §14.4: the action map's `bn5-only`), DarkInvs
(BN5's own, 0x080E2338), the mode chips, Program Advances' recipes (TimeBom+'s among them), the flag-0x40 mode's
effect 0x83 (FireHit's warning, the swords' swing, the meteors, DrilArm's start: never in a netbattle).

**Chips 0x000 to 0x06F** (2026-10-03, bn5-chips-a): WideBlde, LongBlde and CustSwrd (the shared slash with BN5's
parts and its effects 0x19, 0x1A and 0x28; CustSwrd's damage the custom gauge's, formula 45); AirHoc (the shared
puck and flick, content/common/airhocky, of BN5's look); Static (BN5's tornado blow, its tornadoes paralyzing by
the bug level: none, 90, 120, 150 ticks); Spreader (the shared Spreaders' action with BN5's gun and bullet row 3:
BN5's flash 0x21 and sound); GunDelS1 to 3 (the shared sun beam, content/common/gundels; BN5's GunDelSol,
chips/gundels/gundels); BugBomb (the shared BugBomb, content/common/bugbomb, with BN5's bugs: either HP drain plus 2 or
the emotion swings); Katana1 to 3 (chips/katana/katana); MrkCan1 to 3 (chips/mrkcan/mrkcan: the sweeping sight, effect
0x44, and the cannon at its panel); Pulsar1 to 3 and SpShake1 to 3 (lib/armshot, and lib/arm: BN5's buster arm,
0x080EBABE; the pulse, attack 0x6A, and the shake wave, 0x68); Skully1 to 3 (chips/skully/skully, attack 0x88);
Astroid1 to 3 (chips/meteors: instant effect 17, 6, 8 and 10 meteors); Snake (the shared snake and holes' scan,
content/common/snake: BN5's nest sends three snakes at a time with a flag each, its snakes wait 48 ticks and
strike as wood); YoYo (the shared throw, content/common/yoyo; BN5's yoyo, chips/yoyo/yoyo, attack 0x52, GreatYo's
modes too); Slasher (BN5's own action 0x29: while A is held, the wide slash at an enemy navi's column); CircGun
(the shared CircGun of DarkCirc's look, 4 shots); TankCan1 to 3 (the shared action and shell,
content/common/tankcan; BN5's shell, chips/tankcan/tankcan); WindRack (BN5's own action: BN6's swing without the
gusts). Then LifeSync (BN5's controller, effect object 0x5C: no sync in a boss-ranked battle either, the HP
capped by each one's max HP in turn, flag 0x40's hit of 50); MoonBld1 to 3 (BN5's own action 0x53, chips/moonbld/moonbld:
BN6's spin with the Katanas' step, lib/stepsword, marked moving while it runs); CrakBom, ParaBom and ResetBom (their
bomb, chips/crakbom/crakbom, attack object 0x24: BN6's code no BN6 chip throws; it hits the column where it lands);
Quake1 to 3 (BN5's own Quake bomb, chips/quake/quake, attack object 0x51: a weight that drops on the panel three ahead
and hits its level's region); IceSeed, SeaSeed, GrasSeed and LavaSeed (the shared seed with BN5's look,
lib/bombs/seed, attack object 0x4A); CannBall (chips/cannball/cannball, attack object 0x35: it breaks the panel it lands on);
Geyser (chips/geyser/geyser, attack objects 0x42 and 0x43: BN6's BlkBomb code reworked, a splash of 10 on solid ground, in
a hole a geyser whose water hits the eight panels around in a shuffled order); MetaGel (its controller, effect
object 0x21, and gel, attack object 0x45: BN6's code no BN6 chip drops; a gel on each row's panel just ahead of the
user's area, taking it); Magnum (BN5's own: its controller, effect object 0x30, and gunner, actor object 2, the
shared stand-in's drawing, a sight stepping across the enemy's columns until A or 180 ticks, then the column shot
and broken; its unused modes 1 and 2 too); VarSwrd (BN5's action 0x2A: BN6's sequences and BN5's picks, ProtoSoul
and ShadowSoul waiting while A is up) with its hidden picks as chips, FtrSword (0x172: the shared slash over three
panels) and SonicBom (0x173: BN5's action 0x2B, the sonic boom swing with the Katanas' step), whose records
gen_content.py writes (its USED_CHIPS). The chip lab has VarSwrd's five commands and Magnum's A press besides the
generated scenarios (chiplab's library-bn5, by hand). Each matches every frame of its lab recordings. The batch's
shared modules
(content/common, as above): airhocky/puck and flick, gundels/beam, bugbomb/bomb, snake/snake, yoyo/throw,
tankcan/action and shell, lifesync/marker, moonbld/blade and bombs/seed, each BN6's at its old path its BN6 wrapper
(BN6 the same). BN5's Cannon, HiCannon and M-Cannon draw BN5's cannon (0c-01) and sound; CircGun and DarkCirc share
one set of kinds (chips/circgun/circgun).

BN5's hit intake (0x080178EC) takes a hit's NaviCust bug (0x0801103E) before the HP bug drains (0x0800DFEC), where
BN6's `sub_801AC6C` drains first, and a drain bug's argument goes by its flags (bit 4 adds its low four bits, bit 5
subtracts them, else the level rises to them): the status section's `bugs_before_drain` and `drain_bug_flags`
(MoonBld's bug 0x18 drains a tick sooner; BugBomb's codes are its own, argument 0x12). The rest of 0x0801103E is
BN6's `sub_80139F6` but for codes BN5's chips here don't give (0xFD and 0xFC: a drain of 1 on conditions; no 0xF8
or 0xF5, which set their bytes as any other code): not ported.

Not shown by the labs: Static's bug levels 1 to 3; GunDelSol's held A; Katana's and MoonBld's charged step;
Slasher's request 0x80000 (`actions.stun_strike`, BN5's action 0x49, unfilled) and its other console's chip name
(`sub_801EB18`); lib/arm's NaviStats +0x4C and AIData +0x12 (read as 0); the kinds 4 and up of CrakBom's bomb (no
chip throws them); battle flag 0x40's effect object 0x83 (0x080E9FD2, 0x080E9FA4: CrakBom's and Quake's bombs; never
in a netbattle); Geyser's geyser (no recording throws it into a hole); a computer-controlled navi's VarSwrd pick (its
tactics' byte, 0x0802D4E2 +0x12: no battle has one; only the story navis' routines set AIData +0xF0, 0x0802C110, so
Chaos Unison's Dark MegaMan takes the joypad path and gets a Sword). **Waiting:** Wind, Fan, RockCube, BoyBomb1 to 3,
RedFrut1 to 3, Voltz1 to 3 and VDoll (on BN5's field obstacles).

### 15.7 BN5's MegaMan, stages and roles (as built)

- **MegaMan** (content/bn5/navis/megaman, `bn5:megaman`): BN5's navi 0, NameID 0x180, from BN5's tables (his
  element, buster bonus 1, move lag 4, banners, actor record, the 30 attach points of BN5's 0x3C-byte rows), his
  buster and charged shot BN6's shot actions with BN5's setups (weapons/: the damage Attack plus the navi's bonus,
  no worn-out rule or cap; a program drawn on every shot, on half the draws). His forms are his souls (§15.8),
  with his own base form `bn5:base` (P1c): his battle sprite is the form's, 00-00 by soul (0x0800DA3A: category 0
  by the soul where NaviStats +0x29 is 0; 08-00, his navi sprite, has 31 one-frame animations). Changing form, he
  takes BN6's MegaMan branches (NaviStats +0x29 0 in BN5's code too: the anger, the bugs' stripped programs, the
  move lag), but for BN6's per-form tick (`sub_80F0608`), which BN5's table (0x080EB1E8) hasn't: the status
  section's `form_tick` (BN5 false). His mercy flash blinks in the other phase (0x080137B6 hides him while the
  flash timer's bit 1 is clear, BN6's `sub_8016934` while it is set): the status section's `flash_hides_on_clear`
  (BN5 true). The engine asks a player's identity for a Full Synchro aura animation, which
  BN5 has differently (BN6's `sub_80C4C52` is absent): 0, BN6's rule, until BN5's emotions.
- **Stages** (content/bn5/stages/netbattle.luau, compat stages.toml): a stage per distinct record of BN5's
  netbattle settings list (0x0811AF4C, 95 records), its layout (0x0800BD6C) and its actor list; the lab's
  settings (written to RAM by the Team Battle with its own background and effects) match the list's by layout,
  actor list, music, mode and panel pattern. Those with obstacles (actor types 3, 8, 9: BN5's boulder, rock and
  statue aren't ported) are listed as waiting; those with metal, sea or lava panels are stages since the rules
  work's P1a (68 stages). The backgrounds are named by their look (no BN6 background has their tiles).
- **Panels** are a registered section now: BN5's types, BN6's roads and either-side step rule (BN5 has neither;
  the section must name the engine's 13 types; nothing of BN5's reaches them).
- **Roles** BN5 shares with BN6: the sparks (BN5's 0 to 0xD are BN6's rows), the deletion, recovery and cut-in
  effects (BN5's 3, 6 and 0x1E are BN6's), the statuses (by BN5's bytes), the forced charged shot, the first
  barrier's hook.
- **Compat** gains records.toml (weapons by routine number, projectile rows, barriers) and kinds.toml (BN5's
  kind numbers, as the replays meet them).

More for §15.3:

12. **A base form per game**: the engine refuses two base forms in one content; BN5's MegaMan, whose souls are
   forms, needs his own (or the base form to be a navi's).
13. **The custom screen's end**: BN5's result is 49 words on the link (0x08009A5E: its NaviStats are 0x60 bytes,
   BN6's 0x64; `sub_800B3A2` sends 50), so it is in a tick sooner than BN6's would be; BN5's Team Battle screen
   (0x08025EF2) takes the results as BN6's does (0x080266FA: the hands installed, the HUD's wait task off and its
   icons and chip window on) and closes the tick after (its state 8, 0x08025FEC). Built as the flow section's
   `result_words` (BN6 50, BN5 49, of the sending side's game); an earlier `custom_closes_with_results` (the screen
   closing on the tick both results are in) had the close on the right tick but the results a tick late. BN5's
   screen sets no AIData +0x0F on the navis (BN6's `sub_8009338` does): BN6's beast system's `custom_closed`.
   **The dark chip offer** (0x08025114, from the screen's opening 0x08022C5C, after its hand size 0x08025BE4 and
   before the folder closes up 0x080250E6): a worried or dark MegaMan (emotions 1 and 5) gets one of his folder's
   dark chips (0xBB-0xC6) moved to the place after the hand unless the first is dealt: the first taken, each later
   one on an RNG1 draw's low bit, DrkRecov first under a quarter of his HP. Built in BN5's light and dark system's
   `custom.deal` (the custom screen's deal hook, with `custom.folder`, `custom.swap_folder`, `custom.hand_size`);
   its RNG1 draws were what the dark chip recordings' consoles were ahead by.
Items 14 to 18 are built (rules-in-luau.md, "P1b"); 19 and the dark chips' costs wait for a replay that needs
them.

14. **A dark MegaMan clears the holy panel under him** (§6.1; 0x08017136, from all six intake updates, BN6's
   `sub_801A9B8` to `sub_801AC6C`): every tick, an object on a holy panel whose side's light/dark value is 499 or
   less turns it Normal. Team Colonel's MegaMan is dark in the lab (bn5-compat's `LightDark` reads it from the
   setup's NaviStats +0x44), and stands on the default stage's holy row: every replay on it differs at the fight's
   first tick until the engine has the side's value and the rule (a hook in the intake, or the light-and-dark
   system's).
15. **The camera shake draws from the battle's RNG** (0x08030D78, BN6's `camera_doShakeEffect_80301e8`): two
   GetRNG2 draws each shaking tick where BN6 draws from GetRNG1, one channel (BN6's two), and no shake while the
   time is stopped; BN5's shakes change the simulation's RNG (a BlkBomb's landing: 15 ticks of draws).
16. **The hit spark's first tick** (0x080E0870, BN6's `sub_80E0864`): BN5's spark doesn't step its sprite at its
   init, so it lasts a tick longer (spark.rs).
17. **The metal slide's speed** (§15.2): a step up or down covers its 24 pixels in 3 ticks (8 a tick); the
   engine's slide moves 6. Its order by the move's direction is the tables', as recorded. A navi's slide and drag
   both go 8 a tick in depth in BN5 (0x0801361E, 0x080143A8). A slide arriving on metal goes on as on BN6's roads
   (0x08013564), and one arriving on sea ends.
18. **A custom screen asked for in the fight** (L or R with a full gauge) opens a tick after BN5's does: BN5's
   state 0x20 (0x08007774) opens the screen itself once the reversions are done, where BN6 goes through state 0x24
   first. The recordings that dig for a chip over several turns (HolyDrem's, the chip lab's `dig`) met it.
19. **BN5's obstacle framework** (0x08018404, BN6's `sub_801B750`): outside the dimming, an obstacle not in its
   first action on a solid panel tests a word (+0x5C of the toolkit's +0x18) against 0x20 or 0x10 by its panel's
   side, then the panels beside it, and may spawn attack object 0x30 there (0x080CAB02, 0x080CAAE2: ColonelSoul's
   army, read in §15.11); BlkBomb's idle and return actions are 7 and 6 (BN6's 9 and 8), its table without frozen
   and bubbled. Not met in the replays yet.

### 15.8 Soul Unison (as built, in progress)

- **The soul button** (the engine's custom screen, BN5's layout: slot 11, `SlotKind::Soul`, 0x08023C54,
  0x08024B28, 0x08024972): lit for the last pick's family when the navi has a soul of it (a form naming its
  `soul = { number, family }`), the save has the soul (`SoulUnlocks`: bn5-compat gives
  a finished save's six of the version and Chaos Unison) and it isn't used this round (Soul Unison and Chaos Unison
  apart; a dark chip's is Chaos Unison). Pressed: BN5's state 9 (`Phase::SoulChosen`: fades 0x34 and 0x30), the
  soul first in the selection in place of the chip given up. At OK the transform record asks for the soul's form,
  3 turns and the NaviCust's bonus (NaviStats +0x32, at most 9) or Chaos Unison's 1 (0x08024FF6;
  `TransformRequest::turns`, `chaos`; the netplay protocol's version 2); the chip given up leaves the folder.
- **The change** (rules/souls/change.luau, the souls' `change`, BN5's 0x08011F74, run by the shared turn-start
  sequencer): onto the future panel, a flash, his moves stopped (`stop_moving`); the soul's image (objects/
  soul-image, actor 0x2A, its navi's sprite 08-xx) spirals in, blinks and fades; MegaMan emerges in the soul (the
  old form's end hook and the new one's start hook, `put_on/take_off_form_overlay`; TomahawkSoul's shake), the
  status reset; Chaos Unison's 11 ticks arm the chaos charge (AIData +0x12, not yet modeled). The souls system
  (rules/souls/system.luau) keeps the soul's turns (AIData +0x0F), counts them down at a turn's start (0x0801248C)
  and asks for the revert when they run out (0x0801246C); the revert is the souls' own (`FormData::revert`, the
  pause handler's: rules/souls/revert.luau, 0x080121D8: back to `bn5:base` with no state saved).
- **ProtoSoul** (soul 1, sword): sprite 00-01, weapons from 0x0801CA1C's row 1: the buster, the charged slash
  (routine 3: WideSwrd's slash, 80 + 10 × (Attack + 1), counter byte 0x94), Sword chips charged with A (routine
  5: any Sword chip but a dimming or dark one, `charged_chips`' `plain`, 0x0801090A; doubled, 0x080103D0). BN5's
  blade animation goes by the soul (0x080EC038: ProtoSoul 13, ColonelSoul 14, ShadowSoul 15; `blade_anim`).
  souls/01-sword/unison matches every frame; its B+Back shield is routine 4 (BN5's guard as the Reflect program's).
- **The other eleven** follow the pattern: a form file each under navis/megaman/forms (sprite 00-0n, image sprite
  08-0n, family, weapons from 0x0801CA1C: +5 the A-charge's routine, +6 the buster's, +7 the charged shot's, +8
  B+Back's, +0x11 Chaos Unison's, a dark chip by routines 0x30 to 0x3A: weapons/chaos), the element by soul
  (0x0800E634: NapalmSoul Fire, MagnetSoul Elec, TomahawkSoul Wood, ToadSoul Aqua), the charge table's rows
  (0x0801CA6C, ten bytes a routine). What a soul wears is its identity's `parts`, a row of BN5's body overlays
  (lib/body_overlays: 0x080C35DC; GyroSoul's propeller, row 2); the soul's image wears its navi's (0x0800EDBC:
  GyroMan's propeller, NapalmMan's cannon, Colonel's cape, KnightMan's ball and chain). BN5's MegaMan's hooks
  restart what he wears after an animation change, a flinch or a drag by having it reload its animation
  (0x080C374E: the reactions section's `overlay_restart = "reload"`, where BN6's steps it at once).
  - **The status reset by soul** (0x08011B92; a NaviCust change's, 0x08011CBC): GyroSoul's FloatShoes, floating
    body and AirShoes and ShadowSoul's FloatShoes and floating body are the form's `status_reset`; the rest is the
    form's `reset` hook (`FormDef::reset`, called after the flags): SearchSoul's reveal of the other side's
    invisible navis (effect 0x8F), TomahawkSoul's grass, ColonelSoul's arming of its side's obstacles.
  - **The chip use by soul** (0x0800FF48), by form data: `priming` (GyroSoul: a Wind chip primes it, AIData +0x0D,
    0x080102D2; primed, the next damaging Wind or Null chip is doubled, and neither Full Synchro nor anger doubles
    meanwhile, 0x0801026C), `grass_doubles` (TomahawkSoul's Wood chips on grass, which the use turns normal,
    0x0801032A), `front_guard` (KnightSoul's 50 invulnerable ticks for a damaging chip used with the panel ahead
    not its side's, 0x08010392), `charged_action` (NapalmSoul's charged Fire chips start action 0x4B, a napalm bomb
    with the chip's damage: 0x08010442), `charged_chips` and `charged_bonus` (0x0801090A, 0x080103D0: Proto Sword,
    Knight Break, Magnet Elec, Toad Aqua and Napalm Fire doubled; Shadow Sword without a bonus), `move_lag`
    (ShadowSoul's 0).
  - **Built:** GyroSoul (routine 9, action 0x3C: a tornado, attack object 0x1E, along the three panels ahead),
    SearchSoul (8, 0x3B: five shots at the nearest enemy navi's panel, 0x08012E50), NapalmSoul (0x19, 0x44: three
    fire bullets, rows 0x11 and 0x12), MagnetSoul (0x15, 0x42: a paralyzing field, attack object 0x75, on the
    panel ahead and a pull over the six panels ahead) and ColonelSoul (0x14, 0x43: the screen divide on the first
    enemy ahead): their unison recordings match every frame. Not yet: MagnetSoul's B+Back (0x25: instant effect 10,
    immobilizers ahead), ColonelSoul's obstacles (with BN5's obstacle chips), and souls 6 and 8 to 12.
- **Chaos Unison** waits on the engine: its charge (AIData +0x11's weapon, the routine's charge row by the chaos
  level AIData +0x6C, the cycle 0x080105F8 of 0x08010650's rows, the release's requests 0x8000 and 0x10000, the
  idle's start of the chaos weapon or of action 0x39) and, on a failed release, action 0x39 spawns the Dark MegaMan
  (actor record 0x18D: a navi of AI index 0x16, 500 HP, on a random panel for the other side) that runs BN5's
  computer navi AI (0x0802B4AC, AIData +0xF0): a second navi on a side, driven by an AI, which the engine hasn't.
  Eleven of the twelve chaos recordings fail the charge.

### 15.9 A second navi on a side: Chaos Unison's Dark MegaMan (design)

**What BN5 does.** A Chaos Unison charge released off its window starts action 0x39 (0x080EE63C): the battle dims,
MegaMan flashes, and on its second step's eighth tick a navi appears for the *other* side (0x080EE6BC: a random
solid, empty panel of that side's area, 0x08010226 as BN6's `sub_80129F4`; BN5's generic actor spawn 0x08006AAE,
BN6's `sub_80076A0`): actor record 0x18D (actor type navi, AI index 0x16), 500 HP and its element from the
record's enemy structs (0x0800D138, 0x0800D160), its Param2 1 and AIData +2 1. Then MegaMan reverts. The navi:

- is an actor of the player's kind (object kind 0 with its own AIData and collision), on the side's actor list
  (0x08006B86: four slots a side) but not counted (AIData +2): neither its spawn nor its deletion (Param2 1 skips
  0x08008AFC) changes the side's counts, so it neither ends the round nor keeps it going; the round still ends with
  the counted navi (the side's player);
- reads its side's NaviStats (BN5 reads them by alliance: the opponent player's Attack, Charge and the like) and
  none of the side's input: the pad is copied to the side's player alone, and its idle (0x080EAFE0) dispatches by
  the side's input mode and the record's AI index to BN5's computer-navi AI (0x080EB068[0x16], 0x080F1D48,
  0x0802B4AC);
- the AI (about 3.6 KB, 0x0802B4AC to 0x0802C438; its state AIData +0xF0 to +0xFF) moves, fires its weapon routine
  0x3E and uses chips from a list per side (0x02034C20 + side × 0xE0: up to 42 chip ids, a count at +0x54, sixteen-
  byte records from +0x58), its own, not the side's hand;
- is targeted as any navi: hits by its collision, and the searches over the side's actor list (a meteor's target,
  a lock-on) see it; it is deleted as any navi (its deletion leaves the slot to its destroy);
- goes with the round's objects at the round's end (the next round spawns only the stage's actors).

**The engine (proposal).** BN6 never has a second navi on a side, so none of this runs for it; BN6 stays byte for
byte the same.

1. *Spawn:* `battle.spawn_navi(spec)` (0x08006AAE): an actor of the player's kind with `spec.identity`'s actor
   record (its type and AI index), `spec.hp`, on (`spec.x`, `spec.y`) for `spec.side`, Param2 and `not_counted`
   1: on the side's alive actor list, not counted. The engine's actor bookkeeping is BN6's already (the four
   slots, `actor_count`, `alive`, `not_counted`, `spawned_actors`), as are the deletion's Param2 test and the
   destroy's freeing of a not-counted actor.
2. *Who the player is:* the side's player stays `battle.player(side)` (the spawned list's first slot): the pad,
   the hand, the custom screen, the HUD's chip window and emotion window follow it alone, as now. Nothing in the
   custom screen or the HUD changes (the rules agent's S6 doesn't meet this).
3. *Its decisions:* idle asks the side's systems' `controller` hook (S3) for a navi that isn't the side's player,
   as it does for a `controlled` form: "nothing", "chip", "buster", "moved", carried out as idle does. BN5's
   computer-navi AI is BN5 content: a system in BN5's stock ruleset (rules/computer-navi) whose controller answers
   for its AI index 0x16; its per-side list is the system's side state, its per-navi state (AIData +0xF0, sixteen
   bytes) an actor state the system declares (`actor_state`), allocated with the actor (several Dark MegaMen can
   stand on one side: each Chaos Unison's failure brings one). A side whose ruleset hasn't the system leaves such
   a navi standing (a mixed battle's BN6 side); the souls system brings BN5's AI into a mix that needs it.
4. *Its chips and weapons:* `navi:start_chip_attack(chip)` (S4) for the list's chips, the weapon routines by
   number as the content's weapons (0x3E), its stats the side's (`battle.navi(side)`, as BN5 reads them).
5. *HP, deletion, targeting, the round's end:* the navi's own HP and collision; its deletion as any navi's, not
   counted; the round's end the counted navis' (unchanged).
6. *Rollback and netplay:* the spawn, the AI's states and the list are battle state (snapshotted and digested);
   no input reaches the navi; the protocol doesn't change.

**Then:** the chaos charge itself (AIData +0x12 armed, +0x6C its level, +0x1C/+0x1F the cycle by 0x08010650's
rows, the release's requests and idle's starts: Rust, by the side's game's rules, BN6 never arming it), the
failure action 0x39 (Luau), the shade (actor 0x2B), and the success's chaos weapon (AIData +0x11's routine:
ProtoSoul's DrkSword).

**As built (bn5-port-6, 2026-10-03).** All of the above, verified on every frame of the BN5 lab's `chaos-ai/`
scenarios (a failed Chaos Unison with side 0's computer-navi data poked into its save: none, Cannon, mixed classes,
chips walked up to, patterns, traps; 10,225 frames, each through Dark MegaMan's twelve seconds and his leave) and
`souls/01-sword/chaos`:

- *The charge* (Rust: armed, the cycle, the releases) and ProtoSoul's chaos weapon (routine 6: DrkSword loaded as
  the attack, its charge row by the chaos level: navis/megaman/forms/protosoul/chaos).
- *The failure* (action 0x39: rules/souls/chaos, the role `chaos_failure`): the dim, the white flashes, the fade,
  the dismissal of the Dark MegaMen across (0x08104284) and the new one on a random solid empty panel of the other
  side's area, then the revert to the base form.
- *Dark MegaMan* (navis/dark-megaman: NameID 0x18D's record, enemy structs, collision and post-init hook) and the
  system that drives him (rules/computer-navi/system: BN5's computer navis, a system of BN5's stock ruleset): his
  idle (twelve seconds from his first, then his leave, the navi type's action 7), his tick (the time running down
  outside pauses and dimming, his last three seconds blinking, the battle's end ending it).
- *The AI* (rules/computer-navi/ai, 0x0802BA14): its decisions, the buster runs (his buster, weapon routine 0x3E,
  is attack 0x16: three shots, rules/computer-navi/buster), the patterns, the reposition, a chip's play; the
  pressure picks as written (the front one calls 0x081BC8AC, data: an error; the hole one reads the AI's own
  side's tactics; their counters stay 0); getting in place for a chip by its positioning class
  (rules/computer-navi/place: all 33 classes of 0x08029B3C, and the panel searches they share, ./panels).
- *The tactics* (nettai_battle::tactics): the recordings' exchanged blocks (bn5-compat); a match file's
  `[side.tactics]`, sent as the console sends them (0x0802C7BE); the netplay offer (protocol version 3).
- BN5's shots raise BN5's own arm (lib/arm, 0x080EBABE; lib/buster's), the arm a computer navi of AI index 0x16
  raises.

New APIs: a collision's `counter_timer`, `battle.gauge_damage`, a side's sword pick (`battle.sword_pick`,
`set_sword_pick`: `sub_802E070`+0x12, which the AI draws for VarSwrd and NeoVari and BN5's VarSwrd, when ported,
reads for a navi no buttons drive), the identity spec's `body`. Read as constant: NaviStats +0x2A (class 28's test;
its BN5 meaning unread, 0 in every setup).

### 15.10 BN5's emotions (as built)

BN5's emotion is its own routine (0x0801270C → 0x08012740; in battle mode 1, 0x080127C0: Full Synchro or normal),
which the engine runs for a side whose rules say so (the status section's `emotions = "bn5"`):

| BN5's | When | The engine's | Face (0x0801AFB4) |
|---|---|---|---|
| 4 | in a soul (NaviStats +0x2C) | normal (the soul's own face; nothing doubles or ends) | the soul's |
| 3 | anger (AIData +0x34) | angry | 1 |
| 5 | a mood of 0 (a dark MegaMan's) | worn out | 4, the dark face |
| 2 | mood 0xFF | Full Synchro | 3 |
| 0 | a mood of 65 or more | normal | 0 |
| 1 | a mood under 65 | **worried** (new) | 2 |

- **The mood setter** (0x080127D6, BN6's `sub_8015BEC`) leaves a mood of 0 as it is (BN6's leaves a held one): a dark
  MegaMan never reaches Full Synchro, and anger's 0x80 doesn't lift him.
- **The anger tick** (0x08011A14) is BN6's but passes over AI index 23, and ends anger on BN5's 5 alone (BN6's 5 and
  1: worried doesn't).
- **The starting mood** (0x08010EC8's, where BN6's `sub_8013892` sets 0x80): by the light/dark value (0x0801283A):
  under 470 0, under 500 64, from 1000 190, else value / 20 + 103 (500 gives 0x80, so a light MegaMan's is BN6's).
  Battle effect 0x20000 holds the value at 500 (0x08010EDC). The hook `starting_mood`, which the light and dark
  system answers.
- **Full Synchro on a counter** (0x08016FDC, BN6's `sub_801A200`): the counterer, in no soul, to 0xFF through the
  setter (rules/emotion, `bn5:emotion`). The aura (0x0801100C, 0x080C45E0) is BN6's, for an AI index up to 12 in
  BN5's emotion 2.
- **The palette** (0x0800DD94, BN6's `sub_801002C`; presentation): the hook `navi_palette`, the light and dark
  system's. Dark MegaMan (a computer navi of AI index 0x16 or 0x17) 1, another computer navi 0; MegaMan in AI index
  23 2; unable to charge 1; in a soul 0, or 2 with the Chaos Unison charge armed; else by the mood: 0xFF 4, 0 2 (dark)
  or 3 (light), else the value's tier (0x0800DE5C: from 1000 4, from 500 0, from 470 3, else 2). Then Hub Style's
  `hub_style * 5 + 20`, else the element's `* 5` (none in a soul). A link navi's (0x0800DA98, by BN5's navi numbers)
  is left to the framework: BN5's content has no link navi.
- **Hub Style** (NaviStats +0x4C): set out of battle by the patch cards' routine (0x08138214, which follows the
  NaviCust's compile) when patch card 111 is installed and on (0x08137A58); 0 in every recording. It adds 11 to the face (0x0801AF8E: pictures 11-15, the base
  form's second set of faces, `mugshot.variant`, shown by `battle.set_face_variant`) and moves the palette. The
  light and dark system's setup carries it (`hub_style`, bn5-compat from +0x4C).
- The faces' names follow: `megaman-worried` (2), `megaman-dark` (4), `megaman-hub*` (11-15; they were named as
  dark faces).
- **The emotion-swing bug** (0x080113F8, BN6's `sub_8013DA0`; rules/emotion): while the side's NaviStats +0x24 is
  set, out of battle flag 0x40's mode and out of a soul, every 60 ticks the anger ends and the mood goes back to 0x80
  (0x08011A94), then the emotion swings to one of 0x0801147C's sixteen (seven normal, seven worried, one angry, one
  Full Synchro) but the one it last swung to (every entry of it taken out), drawn from RNG2: angry asks for anger (the
  request, no mood test), any other sets 0x0801148C's mood (0x99, 0x3F, 0, 0xFF) through the setter. It runs from the
  tick of every navi of the player's kind (0x080EAD6A; a computer navi's, 0x080F224C, has none), each with its own
  counters (AIData +0x3A, +0x0B), and a curse (BugCurse) can set the bug mid-round, so every player's navi is ticked.
- **The face in Chaos Unison** (0x08019704): while the chaos charge is armed (0x080125F6, AIData +0x12) the window's
  picture is drawn in its palette 11 on, the pack's *soul*-chaos faces (the soul form's `mugshot.variant`); the
  palette is read as the window draws, so a blink back to the soul's face after the failure's revert shows it plain.
- **Dark MegaMan's appearance** (BN6's `sub_80164A0`, the shared mid-battle appearance): white, fading over its 30
  ticks (the color shader gray at the second timer's level), which the engine now draws.
  BugCurse's four recordings match through it.
- **The soul break** (0x080122C8, BN6's `sub_8015766`; the status section's `form_break = "bn5"`): a dark chip used in
  a soul (0x08010070) sets the weakness request, which breaks any form (BN6's only a Cross or a Beast) to the base
  form: BN6's Cross break without animation 2 and the overlay's refresh, the overlay's kept stepping, the collision
  region's removal and return, and the flags 0x80110000 and statuses 0x200800 it clears. (In a netbattle a light
  MegaMan's dark chip fizzles first and a dark one has no Soul Unison: no recording reaches it.)

- **The light/dark bug codes** (0x0801103E, the navi's hit NaviCust bug; the hook `navi_bug`, the light and dark
  system's): a hit with hit flag 0x400 brings nothing to a value of 1000 or more (not even the weapons' reload); code
  0xFD is an HP drain of level 1 (code 0x18, argument 1, through the drain's flags rule) on a dark MegaMan (the value's
  tier 2), code 0xFC the same from 500; else neither is anything. Django's hits bring both: his recordings match.

**ProtoSoul's B+Back** (weapon routine 4, 0x0800F634; navis/megaman/forms/protosoul/back): BN5's guard (action 0x1F,
lib/guard) as the NaviCust Reflect program's (subtype 4): 20 ticks (the params word 0x114's first byte), the Reflect
program's look (its second byte: row 1), row 9's (a look of its own) with the Chaos Unison charge armed (0x914), 50
damage, the B+Back cooldown 40. A guarded hit from the front fires the charged shot's projectile back:
`souls/01-sword/back` (hand-written) matches on every frame, side 1's buster shot reflected for 50.

Seen against mGBA (tools/frontend-compare, unmasked): chips/0x0bc-drksword/hit 1 → 286 of 314 frames exact,
souls/01-sword/unison 19 → 598 of 704 (the dark opponent's palette and face). What still differs there: the emotion
window blinking out after a dark chip (456-537, two frames in four), the custom screen's face box, and the hit
navi left undrawn after Cannon's hit (452-553; before this work too).

### 15.11 bn5-chips-b: chips 0x070–0x0DC, 0x119–0x12C, 0x137–0x138, 0x13B–0x15D, Guard1–3 (as built)

**Program Advances.** A player's formed Program Advances are a `u64` (`ProgramAdvancesUsed`; content may define 64
Program Advances: BN6's 30 and BN5's 30 load together). BN5's full table (0x08027FC8) is 21 recipes and then the
netbattles' table (0x0802801C); only a battle with each side keeping its own gauge (battle flag 0x40, a Liberation
Mission's) tries those 21: a recipe's `per_player_gauges_only = true`, which `find_program_advance` skips unless
the battle's `per_player_gauges` is set. A recipe's `order` is its index in the full table. bn6battle-verify's
tools/bn5/recipes.py writes BN5's recipes (and the flag) into the Program Advances' chip files, naming only chips
that have a use; rerun it as chips get theirs.

**Rules.** The `effects` section's `retype = "bn5"`: BN5's retype (0x08016B9E, BN6's `sub_801A9E8`'s counterpart)
sets the self type only (no dimmed bit), leaves the target type and writes the target's `row_offset` plus four
times the side into the next word. A hit's modifier goes into its side's slot by its flip (`hit_mod_by_side`).

**Obstacles and stages.** The rock and its debris (content/common/rock), the boulder (content/common/boulder) are
makers BN6's chips/rockcube and objects/boulder wrap (same APIs). BN5's rock (attack object 0x59, rows by
variant), its debris (effect 0x38) and boulder (attack 0x6E) are in content/bn5/objects; the stage statue (the
Guardian's, @common/guardian/statue) takes its stage damage word. The 25 netbattle stages that waited on them are
in content/bn5/stages/netbattle.luau and compat/stages.toml (64 stage recordings match). The engine's obstacle
service gains `obstacle.throw` (`sub_800F6AC`: the request `sub_8018002` serves; nothing in BN6 makes it) and
`obstacle.throwable` (an identity's `throwable`, default true; BN5's mine sets false: Poltergeist's 0x080E8CA0
skips BN5's NameIDs 0xDA, 0xD3, 0xD2, 0xE5, 0xE4 and 0xE7). A thrown obstacle's landing (`sub_80180EC`, BN5's
0x08014AB4: r4 = 0x06050001, Param2 0) shows the plain spark: the role `sparks.thrown_obstacle` is `plain` in both
games (BN6's said `charged`, read from the wrong byte; BN6 never throws one).

Item 19 (§15.7), read: it is ColonelSoul's army. All four of BN5's obstacle reactions (0x08018000, 0x08018168,
0x080182D4 and 0x08018404, BN6's `sub_801B394`, `sub_801B4D4`, `sub_801B610` and `sub_801B750`) add one step after
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
0x080F8418 (an entry of 0x080F24A0, a computer navi's) sets the bit and the words too. BN5's obstacle flag word
moves bits too (removed 0x10000, encased 0x6000; BN6's 0x8000 and 0x3000): the engine's names keep BN6's, which
nothing outside reads.

As built: the engine keeps each side's army (`kinds::obstacle::Soldiers`: armed, the two words; in the snapshot and
the digest), which content arms and reads with `obstacle.arm_soldiers(side, sword, gun)`, `obstacle.disarm_soldiers
(side)` and `obstacle.soldiers(side)`. ColonelSoul's status reset arms it (navis/megaman/forms/colonelsoul: its
`reset`), and the engine's status resets disarm it first, as BN5's do (0x08011918 in `sub_8014216`'s counterpart,
0x08011B3C in `sub_80144C0`'s and `sub_80144CA`'s; BN6 has nothing to disarm). The step runs in `obstacle.react`
for an obstacle whose own game's rules have `effects.obstacle_soldiers` (BN5's), spawning the role
`kinds.obstacle_soldier` (objects/soldier, `bn5:colonel-soldier`; its sprite 10-20 is `colonel-soldier`) with its
state `gun` set; the soldier's element byte is what the search left in r2 (0x0800BD1D's low byte for the sword's,
the body mask's for the gun's). The chip lab's souls/07-obstacle/soldiers-gun and soldiers-sword record it.

**Shared modules moved to content/common** (makers taking a game's look; BN6's modules wrap them with the same
APIs): anubis, guardian, otenko, justcone, batcan, colorpt, geddon (controller, quake), barriers (visual,
controller), rflectr, rock (rock, debris), boulder, bugfix (glow, controller), h-burst (action, burst), bodygrd
(striker, shuriken).

**Engine.** The damage formula `gauge_level` (BN5's 73 to 75, CusVolt's: `base` plus 100 by the custom gauge's level,
none when full); `battle.gauge_full` (battle flag 2) and `battle.drain_custom_gauge` (`sub_801DFD0`, CusVolt's drain
outside link battles); ColonelSoul's army (above): `obstacle.arm_soldiers`, `disarm_soldiers` and `soldiers`, the rule
`effects.obstacle_soldiers`, the role `kinds.obstacle_soldier`.

**Chips.** In the range, the branch's 135 chips match all their recordings, and with bn5-navichips' AirSpin1–3,
AqWhirl1–3, Z-Saver, NumbrBl and NeoVari 144 do (525 of the range's 548 recordings, after merging main on
2026-10-03; the 23 left are CopyDmg's, DarkInvs', Jealousy's, LeadRaid's, ChaosLrd's and PileDrvr's, bn5-navichips'
now). The last ones: BugFix; LCrsShld, LStepSwd, LCounter (the Liberation chips: BN5's
controller, effect 0x8A, gives the side five uses of the ability, which only a Liberation Mission's specials read,
nothing a netbattle reads); Poltrgst (BN5's own: controller effect 0x72, stand-in actor 0x58, poltergeist effect
0x73); Navi+20; GunDelEX; InfVulc1–3, LifeSrd, PoisPhar, TimeBom+; GreatYo (controller effect 0x70: the leader and
two followers); PitHoky (the puck's row 3, chips/airhoc/puck); SuprSpr1–3 (wave kinds 6 to 8, chips/widesht/variants);
GigaCan1–3 (projectile row 0x0C: hit modifier 0x49, the blast spark; BN5's projectile has none of BN6's row-0x0C
bursts; the third afterimage on NaviStats +0x4C reads 0, as lib/arm's); H-Burst (BN5's shot: a probe of row 0, its
explosions effect row 0x3C; its bursts are 8, the shot's table 0x080DA514 read at the record's word 0x103, past its
four bytes); the instant Program Advances Boxer1–3 (effect 21: a boxer, effect 0x64, punching FireHit's fists down
the rows), ShakPar1–3 (effect 22: a shaker, effect 0x66, sending paralyzing SpShake waves from the back column) and
CacDanc1–3 (effect 23: a dancer, effect 0x65, a field object dropping cactuses, attack 0xB1); HotBody1–3 (action
0x58: a fire, effect 0x57, spreading flames, attack 0x9D, to the enemies around the last ones; its position stays
the spawner's registers, as its copy of the navi's is stored at address 0x34); CusVolt1–3 (action 0x27: a beam,
attack 0xB8, following the navi); BodyGrd (BN5's sends the striker out at once from its controller, effect 0x6D,
where BN6's is a trap); ElemPowr (its controller, effect 0x7F: 10 Atk+ a panel of the type the user stands on,
those panels back to normal through the panel changer's rows 15 to 19); RainyDay (its controller, effect 0x75, and
cloud, attack 0x97: a hit over the first enemy navi ahead for each sea panel of the user's side, which turns
normal); ElemRage (action 0x56 and flame, attack 0x98: flames sent on ahead, of the element of the panel the user
stands on, spreading and paralyzing; the attach point read unflipped, `sub_8018842`); WildBird (LarkMan with Param4 1,
the summon table's entry 23: no command, his swoop's variant 6 with row 6's speed and turn, 25 more turn ticks and
its eleven turn animations by side, 0x080DDC44); BlakWing (its controller, effect 0x54, flock, attack 0x7E, perches,
effect 0x55, and wings, attack 0x7F: BN6's leftover code); the navi Program Advances CsmoPris (BN5's own CosmoMan,
actor 0x26, and comets, attack 0x74), Football (GridMan, actor 0x25, and balls, attack 0x95, BN6's leftover code)
and BigNoise (ShadeMan, actor 0x1B, and his noise, attack 0x04: BN5's own, not BN6's flame), each in its chip's
folder with the kinds it owns.

**Waiting** (since 2026-10-03 bn5-navichips', what is found of them so far given with them). AirSpin1–3 (BN6's AirSpin top with BN5's changes: random targets, its own panel setting, its hit's
self type 4) and AqWhirl1–3 (BN5's own, attack object 0x5D), both on BN6's AirSpin action, to move to
content/common; PileDrvr (its controller, effect 0x6F, piles, attack 0x99, and their charge, attack 0x9A: AirSpin's
top reworked, so with AirSpin); CopyDmg (BN5's action 0x24, the buster arm and a spawn by subtype, with BN6's mark,
attack 0x28: to share); NumbrBl (BN5's own NumberMan stand-in with BN6's balls), NeoVari, Z-Saver (BN6 has them: to
share, with BN5's changes); DarkInvs (BN5's own: the user's navi on the computer-navi AI for 600 ticks, bn5-port-6's
system); Jealousy (BN6's leftover code, BG transfers; it counts the other side's hand, which the engine has no call
for yet); LeadRaid and ChaosLrd (actors 0x20 and 0x22, and 0x51: §14.5).

**The obstacle chips** (from chips-a's range, 2026-10-03). RedFrut1–3 (action 0x1A's instant effect 15,
0x080D818C, BN5's own: chips/redfrut/fruit): a fruit (attack object 0x8D, NameID 0xE7) drops on a random free panel
but the back columns (they are reserved while it looks, 0x080D8280) and hops: 4 ticks coming, 40 shown, 4 going,
then onto a random free panel of the other rows (or its own, with none), as many times as the record's second
parameter (5), and gone after the last going. A hit breaks it (1 HP); the side whose attacks, objects or bodies broke
it, alone, gets its gift (the record's first parameter, also its palette) on a random alive navi of its side
(0x080D82BA; while it has none, the fruit waits): 300 HP back (AntiRecv turning it) and 50 more mood (0x080127E8: to
254, a mood of 0 left), invulnerability for 420 ticks, or Full Synchro (a player's navi out of battle flag 0x40's mode
whose BN5 emotion is one of the first four; 0x080127D6's setter). BoyBomb1–3 (the dimming handler's subtype 58:
BN6's leftover controller, effect object 0x67, and bomb, attack object 0x3E, which no BN6 chip spawns, with BN5's
numbers: chips/boybomb/bomb): the bomb (NameID 0xD4, 150 HP) on the free panel ahead fades in for 16 ticks, then on a
panel it may stand on (0x080CD2BC, by its side) counts down 60 ticks blinking and blows up over the 3x3 around it
(no spark, against navis, hit modifier 3); anywhere else it breaks. Its Param1 0 (50 HP, the enemy area's blast, a
holder's record) is an AI's (0x08108274), no chip's. The controller's position is the dimming handler's registers
(the user's row, the element and the hook's own address).

**BN5's obstacle pushes** (the obstacle framework, by the obstacle's own game's `push_reading`): BN5 keeps a
collision's hit flags only by the other collision's flip (+0x6C, +0x70: the hit registration 0x080169C8 to
0x08016A68; the engine's `hit_flags_by_flip` beside the union it reads as BN5's +0x68); its push on any hit
(0x08017AD8, from 0x08017A78) reads them: one side's hits by unflipped hitters alone mark the unflipped hitters'
modifier byte (+0x18), by flipped ones the other (+0x19), and the final modifier. Its push vector (0x0800D4B0) takes
the pusher as side 0 when side 0's hits alone pushed (none when both did), else side 1, and the first of bits 2 to 5
of +0x18, else of +0x19 reversed, with a fifth row of nothing (BN6's reads past its table there). Its slide
(0x08014894) keeps no bounds, and both its pushes go 8 pixels a tick in depth (0x08014730: the reactions section's
`slide_speed`). The chip lab's chips/0x055-boybomb1/pushed and airshot record a buster's and an AirShot's
knockback; RedFrut's broken and eaten recordings (side 0's buster, side 1's after side 0's Cannon) its gifts.

**BN5's Full Synchro aura and guard** (found with RedFrut3, then built). The aura is actor object 0x5E (0x080C45E0):
the role `sprites.full_synchro_aura` is BN5's sprite 14-16 (`full-synchro-aura`), and the engine's aura, by the
side's `emotions` rule, takes BN5's ways: its sprite steps while paused (0x080C45E0), but once the fight is on it
stops running while paused (it clears its header's run-while-paused bit, 0x080C4648), it keeps the animation it
started with (the navi's actor record's AI index, 0x0800D1C0), and it hides while its navi is bubbled too
(0x80000004); like BN6's it frees itself as Full Synchro ends. Its spawner (0x0801100C) allows AI indexes to 12. The
chip lab's chips/0x044-redfrut3/broken, chips/0x004-airshot/counter (a counter hit's Full Synchro, 0x08016FDC) and
counter-paused (a pause through it) record it. BN5's guard (0x080169B8, part of BN5's hit test, 0x0801691C: the
reactions section's `hit_test`, which bn5-port-6 made meanwhile) breaks on a hit of types 0x1002 whatever its 0x4000
(BN6's: 0x0002, or 0x1002 with 0x4000) and marks a guarded direction unless the hit has 0x0C004000 (BN6's
0x0C005000). No recording can show it: BN5's own types with 0x1000 all have 0x4000, so only another game's attack in
a BN5 arena breaks a guard that BN6's would hold; and the direction mask differs only for 0x1000, which BN5's guard
never holds. A unit test (`collision::tests`) shows the first under that rule.

### 15.12 The content's layout (as built)

content/bn5 is laid out as content/bn6 is (content-model-v2.md §4.1; the user, 2026-10-03: "you should consolidate
the chips together where appropriate and move colocate objects with those chips, where appropriate like what bn6
does"). The verification workspace's `tools/bn5/layout.py <checkout>` computes the layout from the content and
moves it there (git mv, every `require` rewired, no id changed); its `tools/bn5/gen_content.py` finds each chip's
definition by its id wherever it is. The move changed no recording's replay (§15.5's report the same, recording by
recording).

- **Series files.** A series' chips are one module, chips/<series>/chips.luau: each chip a `local` with its own
  comment above it, the requires once at the top, the module returning the chips by key
  (`require("../cannon/chips").hicannon`), as BN6's chips/cannon/chips.luau. The series are BN6's where BN6 has the
  same chips (barrier, batcan, cannon, colorpt with DblPoint, energbom with MegEnBom, firehit, grabbnsh with
  GrabRvng, gundels, recov, tankcan, timebom with TimeBom+, tornado with Static, vulcan with SuprVulc; wind with
  Fan and gigacan once they have uses); each navi chip with its SP and DS (blizman, cloudman, colonel, cosmoman,
  django, gridman, gyroman, knightmn, larkman, magnetmn, meddy, napalmmn, numbrman, protoman, roll, serchman,
  shademan, shadoman, tmhwkman, toadman); numbered levels (astroid, cactbal, crsshld, drilarm, elcreel, geddon,
  guard, infvulc, katana, moonbld, mrkcan, pulsar, quake, sidebub, skully, spshake, widesht, woodnos; airspin,
  aqwhirl, boxer and the rest as they are ported); a Program Advance whose ingredients are one series'; and BN5's
  own two: crakout (CrakOut, DublCrak, TripCrak, as BN6's CrakShot series) and cannmode (the Liberation Missions'
  mode chips: CannMode, CannBall's, SwrdMode, YoYoMode, DrilMode). A chip in no series keeps
  chips/<key>/chip.luau.
- **A chip without a use keeps its own folder** until the port gives it one: the loader leaves it out by its
  folder, with every chip folder that requires one of its modules (content-model-v2.md §7.3), so it can't be in a
  file with chips that play. The next run of layout.py takes it into its series.
- **Kinds with their owners** (§4.1's rules 1 to 5). A kind one chip or series uses is in its folder
  (chips/vulcan/vulcan, chips/timebom/timebom, chips/widesht/wave and variants, chips/yoyo/yoyo). One with a
  natural owner and borrowers is the owner's: the dark chips borrow their light chip's (DarkDril
  chips/drilarm/drill, DarkThnd chips/thunder/ball, DarkTorn chips/tornado/tornado, DrkLance chips/lance/lance,
  DarkCirc chips/circgun/circgun, DarkWide chips/widesht/wave), the InfVulcs chips/vulcan/vulcan, PoisPhar
  chips/anubis/anubis, ParaBom and ResetBom chips/crakbom/crakbom, MudWave and RedWave chips/wavepit/wavepit. A
  family's is in its lib/ folder, its builder with it: lib/bombs (bombs, seed), lib/guard (guard, the Guard chips'
  shock wave), lib/traps (traps, the Anti traps' anti_trap), lib/instruments (instrument), lib/navi-chips (the
  navi chips' throw marker). The soul system's are rules/souls' (shade, image).
- **objects/** keeps what several families share (attachment, bullet, flying-shot, panel-bursts, panel-changer,
  projectile with its variants, rising-bubble), and six modules whose `define.kind` keys name no owner, which
  can't move without new ids (the content check keys a kind in an owner's folder under the owner): capsule
  (Meddy's), dice (NumberMan's), gyro-bomb, napalm-bomb, crack (CrakOut's) and meteors (the shower and its
  marker). Re-keyed (`bn5:meddy/capsule`, ..., with compat's kinds.toml), the next run moves them.

| | before | after |
|---|---|---|
| modules | 535 | 423 |
| chips/ folders | 333 (a chip each) | 221: 53 series' (165 chips), 168 chips' own (68 without a use yet) |
| objects/ folders | 53 | 13 |
| lib/ | 18 modules | 15 modules, 5 family folders |

**A branch from before the layout:** merge main and take main's moves; where main merged a chip you changed into a
series, the merge reports your chips/<key>/chip.luau as modified and deleted: keep yours (`git add` it); where
main only repointed `require` lines you also changed, keep yours. Commit the merge, then run
`tools/bn5/layout.py <checkout>` from the verification workspace's main: it takes your version of the chip into
its series file, moves your new chips and objects to their places, and repoints your requires of old paths (a
chip's by its id, another module's by git's renames). Then build and test as usual. A series' new kinds are keyed
under its folder (`bn5:<series>/...`), and a chip gets its series file's place by gaining a use. (Tried on
bn5-chips-b at 3c524c21: the merge stopped at four chip files and one block of requires, as above; the run then took
26 chips into 10 series files, moved 7 new kinds and repointed 22 old requires, and the branch's §15.5 report
stayed the same, recording by recording: 1,250 matching, 871,358 frames.)

### 15.13 BN5's NaviCust (as built)

BN5's compile (0x0813FA10; 0x0813F97C runs it, then the patch cards, 0x08138214) is BN6's routine for routine, so the
two share it: content/common/navicust/compile.luau, each game's navicust system passing its board, its bugs and its
quirks (`NaviCustGame`). BN6's compile is unchanged (trace-tests' navicust: Tango's four saves and the lab's 1274
NaviCusts). BN5's, content/bn5/rules/navicust:

- **The board** (rules/navicust/board.luau, the section `bn5:navicust`): 5x5 cells, no expansions and no frame, any
  cell of it a neighbor's; the engine's 7x7 grid with BN5's board in its middle, the command line BN5's row 2 (the
  engine's 3). A part's 5x5 shapes (the part table's +8, +0xC) sit in the middle of the definition's 7x7.
- **The programs** (content/bn5/navicust/, 47; compat/navicust.toml by number and colored variant), written by the
  verification workspace's tools/bn5/gen_navicust.py from the part table (BRBE 0x0813D540, BRKE 0x0813D628, BRBJ
  0x0813D0CC, BRKJ 0x0813D1B4; 16 bytes a part id, program n's four variants at 4n..4n+3: +0 the exclusive group,
  +1 a plus part, +3 the color, +4 the bug group). Unlike BN6's a program's colored variants needn't come first
  (SprArmr is only its fourth), so compat lists each program's colored variants; the Japanese ROMs' MegFldr1 comes in
  pink too (part 17). The effects are the handlers of 0x0813FB44 (the shared constructors, @common/navicust/effects;
  Shield, Reflect and AntiDmg are MegaMan's B+Back weapons 0x1F, 0x61 and 0x21, navis/megaman/weapons/guards.luau;
  SoulT+1 adds a soul turn, NaviStats +0x32, the stat `soul_turn_bonus`, at most 6).
- **The counts** (0x0813F310): BN6's but for HubBatc (program 27), which counts its own bug once more (0x0813F5A8),
  and no frame. Five colors bring the status bug, six the stronger one.
- **The reset** (0x08133DBC) keeps the mood, the light/dark value, +0x21, +0x22, the base HP, the soul, the folder and
  its Regular chips, the Regular memory and the HP; the rest is the navi's fresh stats (0x08010C00 under MegaMan's row
  of 0x0801D55F).
- **The bugs** (0x08140008, its table 0x08140054 by group and level): steps astray (+0x31), the emotion swing
  (+0x24), the panel trail (+0x12 = 3, +0x13 = 2, 4 or 8), the custom screen's damage (+0x54: 20, 40, 80), encounters
  (+0x28), drops (+0x26), the buster (+0x14 = 4, +0x15 = 2), no supports (+0x0D = 0xFF), the HP bug (+0x16 = 3), Hub's
  (0x08140248: the maximum the base and half the HP programs, the HP no more than it), the statuses (+0x1A = 9, 10).
  BugStop (+0x1F) stops them (0x0813FA48; the patch cards can stop or keep them, 0x08137A30, which come with BN5's
  patch cards). No emotion window glitch: its flag, 0x10C1, is read outside battle only.
- **The HP** (0x0803C13C): in the real world the maximum again and the HP with it; in the cyberworld (the save's
  area, 0x02002944, from 0x80, or event flag 0x10B2) nothing, so the HP is the save's and the maximum the effects'
  (0x0803C1CC), Hub's halving standing. The navicust system's setup `cyberworld` says which; Tango's finished Team
  ProtoMan light save is in the cyberworld (area 0x8C, HP 850 of 1000).
- **Compression** is an event flag (0x1EC0 + the part id, which 0x0813EEFC tests), not the list's +5 (the editor's
  mark): every finished save on hand has the flags of every program that compresses set.
- The save's list: 25 parts of 8 bytes at 0x02004D6C (+0 the part id, +2 the column, +3 the row, +4 the quarter
  turns), the grid at 0x02004D48; bn5-compat's `trace::navicust` reads it.

bn5-compat now reads the bug bytes the compile writes (+0x12 to +0x16, +0x1A, +0x24, +0x26, +0x28, +0x31, +0x54) at
BN6's offsets into the engine's stats, which a replay's setup carries.

**Checked:** trace-tests' bn5_navicust compiles Tango's BN5 saves (the finished ones in ~/Documents/Tango/saves and
the netplay templates; the three with patch cards wait for BN5's patch cards) and every side of the BN5 lab's
scenarios against its recording's setup, in every byte the engine's compile can write (59 NaviCusts of 164 programs:
library-bn5/navicust-compile, the finished saves' own NaviCusts with their cards taken off, both sides of each). The
programs in battle (library-bn5/navicust, 31 scenarios) replay on every frame but Rush's, which waits for BN5's
supports (the role `kinds.support`).

What the programs in battle brought into the engine:

- BN5's charged shot (0x080EC9F0) waits only in the battle flag 0x40 mode, marking the panel in front; else it fires
  on its first tick (the shared charged shot's `waits`).
- BN5's hit test (0x0801691C, §15.3 item 12): a FloatShoe body is hit (BN6's needs the hitter's 0x80 self bit, which
  BN5's collision types lack), and its raw channel (0x08017494) has no FloatShoe test either, so a barrier under
  FloatShoe wears.
