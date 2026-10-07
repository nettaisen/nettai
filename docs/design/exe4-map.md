# EXE4 against EXE5 and EXE6: the routine map

What of EXE5's and EXE6's battle code EXE4 (Battle Network 4: Red Sun and Blue Moon) has, routine by routine, and
what is its own: the evidence base for EXE4's port, as [exe5-map.md](exe5-map.md) was for EXE5's. The user, through
the coordinator, 2026-10-06: port EXE4 "the way EXE5 was ported, frame-exact against recordings".

As for EXE5, the map is made from the ROMs alone (there is no EXE4 disassembly or symbol map): the verification
workspace's `tools/exe5/bmap.py --to B4WE` maps EXE6's inventory (the disassembly's routines, named and sized by its
symbols) into an EXE4 ROM, and `tools/exe4/emap.py` maps EXE5's routines (found by discovery, named by their EXE6
counterparts where EXE5's own map has them) into it the same way, so that what EXE4 shares with EXE5 alone shows
too. Addresses are US Red Sun's (B4WE) unless said otherwise; EXE5's are US Team ProtoMan's (BRBE). Where a fact
comes from: **R** read in the ROMs for this map, **T** Tango's BN4 game support (tango-gamesupport-bn4,
-bn4-dataview), **K** known of the game and not yet read, **?** open.

## 0. Summary

- **EXE4 is an older engine than EXE5, further from it than EXE5 is from EXE6.** Of EXE6's 4,567 battle routines in
  scope EXE4 has 17% **the same** (EXE5 40%), 5% the same with **other constants** (EXE5 6%), 18% **similar** (EXE5
  22%), 7% a counterpart that **differs** (EXE5 5%) and 53% **absent** (EXE5 27%). Of EXE5's routines with an EXE6
  counterpart in a battle area, EXE4 has 25% the same, 7% consts, 19% similar, 6% differs and 43% absent. EXE4's port
  is closer to a new port than EXE5's was: the shared part is the engine's core, not its systems.
- **The core is shared** (R): the RNGs (`GetRNG1`, `GetRNG2` and their positive forms the same code), the object
  system (spawning, the update list, freeing: 15 of its 26 routines the same, 6 close, 5 absent), the object record
  (0xD8 for actors and attacks, 0xC8 for effects; every field the shared code reads at EXE6's offset but one byte,
  §3.1), the chip record (0x2C bytes, the same getter, the table at 0x080197EC; the class at +0x08, not EXE6's +0x07,
  §3.2), and the custom screen's dispatcher (`sub_8026A28`, the same code) over EXE4's own state handlers.
- **The pools** (R, the pool table at 0x08003444 the same layout as EXE5's): **8 actor slots** (EXE5 16, EXE6 32) at
  0x0203B180 (Tango's "unit" records agree, T), 32 attack slots at 0x0203C080, 32 effects (0xC8) at 0x02037D10.
- **The stat block** (NaviStats) is **0x40 bytes** a side (R: the accessors at 0x0800D67E to 0x0800D6B6 multiply by
  0x40), at 0x0203BEC0 in battle (EXE5 0x60, EXE6 0x64). Its fields are EXE4's own layout, the same kinds as EXE6's
  at other offsets (§3.3); BN4's Mod Card effect ids are those offsets.
- **EXE4's own systems** (R: absent or differing in both maps; K for what they are): the custom screen's states, Double
  Soul (no counterpart of EXE5's soul button, its hand builder or the turn-start transformation sequencer EXE5 and
  EXE6 share), the dark chips offered in battle, the emotions (EXE5's emotion routine and mood setter have no
  counterpart), the NaviCust compile (EXE5's and EXE6's shared compile is absent), the Mod Cards (patch cards: 134 by
  Tango's count, six slots in the save), the battle flow (EXE6's `battle_8007800` similar 0.50 at 0x08006B14).
- **Red Sun and Blue Moon** are the same code, moved by 0 to 0x14 bytes (R: 4,504 routines paired in both, 4,261 of
  them moved by 0, 4, 8 or 0xC); the Japanese ROMs move more (−0x110 to +0x3C, the text code). Twelve souls, six a
  version (T, K).
- **Tango plays BN4** with netbattle presets (T): twelve raw saves (light with HP 1000 or 999, dark with HP 997, each
  version and region) and its own hooks (the round's start, the round result, the comm menu, the set's end), which the
  oracle and the chip lab can reuse as they did EXE5's.

## 1. ROMs and method

| Code | File | Version, region | IWRAM code (R) |
|---|---|---|---|
| B4WE | exe4_rom_e.srl | Red Sun, US | 0x17D0 bytes from 0x08212700 to 0x03005800 (the boot's pool at 0x080001F0) |
| B4BE | exe4b_rom_b_e.srl | Blue Moon, US | the same |
| B4WJ | exe4_rom.srl | Red Sun, Japan | 0x17CC bytes, from and to the same |
| B4BJ | exe4b_rom_b.srl | Blue Moon, Japan | 0x17CC bytes, from and to the same |

(EXE5's and EXE6's boots copy their IWRAM code from a pool at 0x08000208; EXE4's is at 0x080001F0, which
`tools/exe5/gba.py` now finds.) Saves on disk: exe4b_rom_b_e.sav (Blue Moon US, 32 KB); exe4b_rom_b.sav is empty.

**The maps.** `bmap.py --to B4WE` (and B4BE, B4WJ, B4BJ) writes target/exe4/bmap-<CODE>.tsv, exe5only-<CODE>.tsv
(EXE4 routines with no EXE6 counterpart) and, with `ram`, ram-<CODE>.tsv. `tools/exe4/emap.py` writes
target/exe4/emap-BRBE-B4WE.tsv (per EXE5 routine: its EXE4 counterpart, status, ratio, via, its EXE6 name and area,
and the EXE6 routine's status in EXE4) and, with `ram` and `romdata`, the RAM and ROM data the paired routines load,
EXE5's address against EXE4's. The statuses and the ways of finding a counterpart are exe5-map.md §1's.

**How sure.** `same` and `consts` are facts about the code. `similar` and `differs` are pairings, and for EXE4 they
are much less reliable than for EXE5: where EXE4's routine differs from both, the `order` and `table` steps pair what
is near or at the same index. A check: the custom screen's three state handlers (the dispatcher's table, 0x0801E100)
are EXE4's 0x0801E110, 0x0801E210 and 0x0801E180, where emap's `order` pairs EXE5's with 0x0801E640, 0x08020FE4 and
0x08020E08. Read a pair before citing it (`bmap.py --to B4WE diff NAME`).

## 2. By area

EXE6's audit areas, in-scope routines, EXE6 → EXE4 (`bmap.py --to B4WE`):

| Area | Routines | same | consts | similar | differs | absent |
|---|---|---|---|---|---|---|
| object kinds, chips, navis (asm31) | 2,682 | 393 | 106 | 487 | 210 | 1,486 |
| actors, collision, status, HUD | 821 | 57 | 64 | 162 | 64 | 474 |
| custom screen, gauge, camera | 367 | 56 | 11 | 25 | 10 | 265 |
| elsewhere, run in a netbattle | 222 | 125 | 16 | 37 | 7 | 37 |
| battle flow | 180 | 36 | 7 | 48 | 16 | 73 |
| battle objects, panels (object.s) | 137 | 42 | 4 | 37 | 12 | 42 |
| link layer | 79 | 18 | 4 | 18 | 1 | 38 |
| IWRAM routines | 37 | 12 | 4 | 9 | 2 | 10 |
| object system | 26 | 15 | 1 | 4 | 1 | 5 |
| link status, battle settings | 16 | 4 | 1 | 4 | 0 | 7 |

EXE5's routines → EXE4 (`emap.py`), by their EXE6 counterpart's area:

| Area | EXE5 routines | same | consts | similar | differs | absent |
|---|---|---|---|---|---|---|
| object kinds, chips, navis | 2,772 | 636 | 161 | 549 | 167 | 1,259 |
| actors, collision, status, HUD | 758 | 71 | 91 | 173 | 75 | 348 |
| custom screen, gauge, camera | 552 | 185 | 33 | 48 | 17 | 269 |
| battle flow | 230 | 51 | 11 | 57 | 10 | 101 |
| elsewhere, run in a netbattle | 208 | 131 | 17 | 27 | 9 | 24 |
| battle objects, panels | 149 | 44 | 6 | 36 | 13 | 50 |
| link layer | 102 | 35 | 10 | 29 | 5 | 23 |
| IWRAM routines | 56 | 26 | 2 | 9 | 3 | 16 |
| link status, battle settings | 28 | 11 | 6 | 5 | 0 | 6 |
| object system | 26 | 18 | 0 | 5 | 1 | 2 |

EXE5's own routines (no EXE6 counterpart): 13,932 discovered, 316 the same in EXE4, 12,942 absent (the battle
ones that matter are named in §5 to §8).

The object kinds are where most of a port's work is: about a quarter of EXE5's are EXE4's verbatim (636), a fifth
close (similar), half not found. The actors, collision and status area, which EXE5 shared with EXE6 at 44% the same,
shares 9% with EXE4: EXE4's actor code (the navi, its actions, the hit kernel's callers) is largely its own.

## 3. Records and RAM

### 3.1 The object record and the pools

The record sizes are EXE5's and EXE6's (0xD8 for actors and attacks, 0xC8 for effects). The pool table (0x08003444,
16 bytes a pool, read by EXE4's `InitializeStructsOfObjectType`, 0x0800340C):

| Type | Base | Slots | Size | EXE5 | EXE6 |
|---|---|---|---|---|---|
| 0, the overworld player | 0x0200A610 | 1 | 0xA8 | | |
| 1, actors | 0x0203B180 | **8** | 0xD8 | 16 at 0x0203B200 | 32 at 0x0203A9B0 |
| 2, overworld NPCs | 0x02006140 | 16 | 0xD8 | | |
| 3, attacks | 0x0203C080 | 32 | 0xD8 | 32 at 0x0203CA40 | 32 at 0x0203CFE0 |
| 4, effects | 0x02037D10 | 32 | 0xC8 | 32 at 0x02036F00 | 32 at 0x02036870 |
| 5, overworld map objects | 0x0200B820 | 32 | 0xB8 | | |

**The fields** (`fields.py --to B4WE`: each load and store off r5, the object, at the same place in EXE6's routine
and its EXE4 counterpart): every offset from +0x00 to +0x7C is EXE6's in most reads, but one. **+0x0E** (a byte) is
EXE4's **+0x17** in 51 of 62 reads (EXE5 keeps it at +0x0E). One read each of a halfword moves +0x1E to +0x28, to
confirm. Tango's unit record (T) agrees on the panel at +0x12, the destination at +0x14, the alliance at +0x16 and
HP and max HP at +0x24 and +0x26. So the engine's object record and its accessors can serve EXE4 as they are, with
the one byte's place a game's.

### 3.2 The chip record

The getter at 0x080190D0 is EXE5's and EXE6's (`getChip8021DA8`, the same code): 0x2C bytes a record, the table at
0x080197EC (US; 0x0801972C in the Japanese ROMs, T). 350 entries (T). Tango's reading (T): codes +0x00, the attack
element +0x04, rarity +0x05, **MB +0x06, element +0x07, class +0x08** (EXE5's and EXE6's order is element, class,
MB), the dark flag in +0x09 bit 5, the counter +0x0A, family and subfamily +0x0B/+0x0C, a dark/soul usage byte
+0x0D, the parameters +0x10, the delay +0x14, karma +0x15, the library flags +0x16, the sort keys +0x18 and +0x1C,
power +0x1A, the chip gate byte +0x1E, the icon, image and palette +0x20 to +0x28. The ROM agrees (R): `fields.py
--to B4WE after getChip8021DA8` reads EXE6's class (+0x07) at EXE4's +0x08 in all six paired reads and every other
field seen (+0x00 to +0x05, +0x09, +0x0A, +0x0C, +0x10, +0x14, +0x16, +0x18, +0x1A, +0x1C, +0x20, +0x24, +0x28) at
EXE6's offset; Cannon, HiCannon and M-Cannon's +0x06 are 8, 24 and 40, their MB. So the chip codec's byte order is
a game's fact at those three bytes, and EXE4's record has no lock-on byte or dark chip id (multi-game.md §2.4).

### 3.3 NaviStats

0x40 bytes a side, two blocks at 0x0203BEC0 and 0x0203BF00 in battle (R). Eight more 0x40-byte blocks hang off a
pointer at the toolkit's +0x78 (R: 0x0800D67E walks eight; whose they are: ?). The layout is EXE4's own.
`fields.py --to B4WE navistats` (the field each call to a NaviStats accessor names, EXE6's against EXE4's; one to
three calls each, so to confirm):

| EXE6 | EXE4 (calls) | EXE6's field (exe6-compat's codec) | BN4's Mod Card effect id (T, multi-game.md §2.4) |
|---|---|---|---|
| +0x0A | +0x12 (3) | the custom level (chips dealt) | 0x12, the custom screen's chips |
| +0x0F | +0x1F (1) | read by the starting mood (`sub_8013892`; not modeled) | 0x1F, Full Synchro at the start |
| +0x06 | +0x21 (1) | the first barrier, read by the starting mood | 0x21, an aura |
| +0x07 | +0x0A (1) | the back special (B+Left) | 0x0A, the B charge (0x0C is B+Left) |
| +0x39 | +0x09 (1) | the A charge weapon | 0x09, the B button |
| +0x29 | +0x23 (6), +0x24 (3) | the navi | |
| +0x2C | +0x23 (2), +0x24 (1), +0x00 (1) | the form | 0x24, a soul |
| +0x0E | +0x00 (2), +0x27 (1) | the mood | |
| +0x02 | +0x06 (1) | rapid | |
| +0x18, +0x19, +0x31 | +0x0E, +0x0F, +0x24 (1 each) | the HP drain, custom drain and processing bugs | |
| +0x16, +0x22, +0x26 | +0x00, +0x10, +0x19 (1 each) | the hit-status bug, the sun, chip drops | |
| +0x64 (the second block) | +0x40 (1) | | |

So the Mod Card effect ids are NaviStats offsets (the custom level's +0x12 by three calls and Full Synchro's +0x1F
agree; the rest one call each, some likely mispaired), as EXE6's bug codes are into its block: a card's effect sets
the field of that offset. Where HP and max HP are in the block (?); the save's max HP is at 0x21CA (T).

### 3.4 RAM

Everything moved again. Read (R) or confirmed by Tango (T):

| What | EXE6 | EXE5 | EXE4 | How |
|---|---|---|---|---|
| the toolkit (r10) | 0x020093B0 | 0x0200A440 | **0x02009E70** | R: the boot copies 0x40 bytes from 0x080060E8 (EXE5 0x3C from 0x08006020) |
| BattleState | 0x02034880 | 0x02034A90 | **0x02035810** | R: the toolkit's +0x18 (as in EXE5); the round result's wins and losses at +0x18/+0x19 (0x08007124) |
| RNG1 | 0x02001120 | 0x02001C94 | 0x020015D4 | R, T |
| RNG2 | 0x020013F0 | 0x02001D40 | 0x02001790 | R, T |
| the joypad | 0x0200A270 | 0x0200AF50 | 0x0200A700 | R: the toolkit's +0x04 |
| the object update list | 0x02009380 | 0x0200A410 | 0x02009E40 | R |
| the sound queue | 0x0200A490 | 0x0200B170 | 0x0200A800 | R |
| the chip blocks (hands) | 0x020349C0 | 0x02034E20 | 0x02035CB0 | R, T |
| the input records | 0x02036820 | 0x02036E90 | 0x02037C40 | R |
| the panels | 0x02039AE0 | 0x0203A100 | 0x0203AC70 | R (EXE6's map; the entry size ?) |
| the fighting machine | 0x0203CA70 | 0x0203C5D0 | 0x0203BCB0 | R |
| NaviStats | 0x0203CE00 | 0x0203C880 | 0x0203BEC0 | R |
| the link struct | 0x0203F7D8 | 0x0203F244 | 0x0203F6D4 | R |
| the custom screen's state | 0x020364C0 | 0x02036B10 | 0x02036440 | R; T's "custom flags" |
| the actor pool | 0x0203A9B0 | 0x0203B200 | 0x0203B180 | R, T |

To find for the oracle (§13): the battle folder, the banner, the custom gauge and the HUD's task mask, the pause flag,
the exchanged transform records (if EXE4 has the records at all), the panel entry's size and the sound queue's
entries.

## 4. The battle flow

EXE6's battle flow area: 36 of 180 routines the same, 48 similar, 73 absent; the round's loop (`battle_8007800`) is
similar 0.50 at 0x08006B14. Tango's hooks (T, the same addresses in both US ROMs) locate EXE4's own: the round
start's return (0x08006710), the round result (the KO's win and loss at 0x08007130 and 0x08007144, after the code
counts the win in BattleState+0x18 or the loss in +0x19, R; the time-out judge's win, loss and draw at 0x080073DA,
0x080073EE and 0x080073F4), the battle start's music call (0x080074BC) and the set's end (0x08004F68). A netbattle is
a single battle or a triple battle, which the game chains inside its battle mode (T). The flow's states, the banners
and the turn's gauge are to read routine by routine against EXE5's (§13).

## 5. The custom screen

**Shared:** the dispatcher `sub_8026A28` (0x0801E0EC, the same code over the state byte at 0x02036440), and some of the
screen's helpers (185 of EXE5's 552 custom-screen routines the same, many of them the drawing).

**EXE4's own:** the three state handlers (0x0801E110, 0x0801E210, 0x0801E180; the first, R, is no shape of EXE5's: it
tests the screen's +0x06, asks 0x0801FEE4 for a cursor position and reads that slot's byte at the screen's +0x21 +
2 × (row × 5 + column)) and what they run;
none of EXE5's custom-screen logic that matters for a netbattle is found: what can be picked (`sub_8028E32`), OK's
hand builder (`sub_8029110`), the keys (`sub_8028B74`), the dark chip's cursor (`sub_802806C`), the soul button's
offer (0x08024B28) and its family table (0x08024BE0): all absent in both maps.

**Double Soul** (K, T): sacrificing a chip of a soul's kind on the custom screen; twelve souls, six a version (Red Sun:
Roll, Guts, Wind, Search, Fire, Thunder; Blue Moon: Proto, Number, Metal, Junk, Aqua, Wood), three turns, once a
soul a battle (?). **Dark chips** (T, K): never in a folder, offered only in battle (T, multi-game.md §2.4), when
MegaMan is worried (?); a chip with the dark flag (+0x09 bit 5). How the offer is drawn, which slot it takes and what
using one does (the dark state, no Double Soul afterwards: ?) are EXE4's own code, to read.

## 6. Transformations

EXE5's and EXE6's turn-start transformation sequencer (`sub_801483C`, `sub_80148CC`, `sub_8014944`, `sub_8014A00`,
the transform records' copy `sub_80147E4`) and the transform record's writer (`sub_8015952`) have no counterpart in
EXE4 in either map. So Double Soul's change is EXE4's own path: the engine's turn-start sequencer (the rules'
transformations) can carry it, but its order of events is to read from EXE4's code, not to assume from EXE5's.

## 7. Emotions

EXE5's emotion routine (0x0801270C), its link-battle form (0x080127C0), the mood setter (0x080127D6), the anger tick
(0x08011A14) and the starting mood (0x08010EC8) have no counterpart in EXE4 by EXE5's map (EXE6's map pairs EXE6's
`sub_8015B54`, `sub_8014326` and `sub_8013892` with EXE4 routines only as similar, unread). EXE5's light/dark mood
helper (0x0801283A) is EXE4's 0x0800F56A with other constants; Full Synchro on a counter (EXE6's `sub_801A200`) is
similar 0.65 at 0x080131E4. EXE4's emotions (K): normal, Full Synchro, angry, worried, the dark state; their triggers
and faces are its own table, to read.

## 8. NaviCust and Mod Cards

The NaviCust compile EXE5 and EXE6 share (`sub_813C458`) and its placement checks have no counterpart: EXE4's compile
is its own. 47 programs, four color variants each (188 parts, T; the part table at 0x0804563C in Red Sun US, 0x08045644
in Blue Moon US). The save's NaviCust: its list at 0x4564 and the 5x5 grid at 0x4540 (T). **Mod Cards** (EXE4's patch
cards): 134 by Tango's count, six slots in the save (0x464C on, 0x4653 off, T), whose effects set NaviStats bytes by
number (§3.3); the cards' routine is to find (EXE5's 0x08138214 has no counterpart).

## 9. The link exchange

EXE6's link layer: 18 of 79 routines the same in EXE4, 38 absent (EXE5's: 35 of 102 the same). The link struct is at
0x0203F6D4. Tango (T) drives EXE4's own exchange: the vs prompt's confirm connects, runs the ROM's settings generator
off the seeded RNGs and transmits it; the battle's link session is the battle's init's. What the exchange carries
(the navi stats, the folders, the custom screen's results) and how the transform records travel, if they exist, are
to read.

## 10. Content tables

| What | Where (B4WE) | Count | Source |
|---|---|---|---|
| chips | the record table 0x080197EC (0x2C each); names 0x0804FB74, descriptions 0x0801FDE0 (pointers) | 350 (Red Sun legal: 1–186, 201–280, and its version's navi chips) | R, T |
| NaviCust programs | parts 0x0804563C; names 0x0804FB84, descriptions 0x0803E63C | 47 programs, 188 parts | T |
| Mod Cards | | 134 | T |
| souls | (the custom screen's code, to read) | 6 a version | K, T |
| navis | MegaMan in a netbattle (K) | | |
| stages | (the settings generator's tables, to read) | | |

## 11. Versions and regions

Red Sun and Blue Moon US: 4,504 routines paired in both maps; 2,377 moved by 0xC, 1,095 by 0, 514 by 4, 275 by 8,
232 by 0x14: the same code, moved by what the versions hold differently (which, routine by routine: ?). Red Sun
Japan against US: moves of −0x110, +0x3C, −0x20, −0xAC, −0x12C and −0xC0, and an IWRAM block 4 bytes shorter
(0x17CC). The RAM is the same in all four (T: one EWRAM layout for the four). The chip table is at 0x080197EC in both
US ROMs and 0x0801972C in both Japanese ones; the NaviCust part tables and the name pointers move by 8 to 0xC between
the versions (T).

## 12. Saves

Tango's BN4 save support (T): the image is 0x73D2 bytes; the mask word at 0x1554, the shift word at 0x1550 (a region
from 0x2130 to 0x5E20 moves by it, up to 0x1FC, a multiple of 4), the checksum at 0x21E8, the game's name at 0x2208;
the equipped folder at 0x2132, the Regular chip at 0x214C; the NaviCust and Mod Cards (§8). Tango ships twelve raw
netbattle saves (tango-gamesupport-bn4/src/saves): light with HP 1000 or 999 and dark with HP 997, for each version and
region; with the .sav on disk (Blue Moon US), the chip lab's bases and the save import's tests have what EXE5's had.

## 13. What to share, what is EXE4's own, and the next steps

**Shared through the engine and exelib** (the same code in EXE4, by the maps): the object system and pools (with
EXE4's slot counts per pool), the object record's fields (but +0x0E's byte), the RNGs, the chip record and getter (but
the three bytes +0x06 to +0x08), the panel grid's helpers (object.s: 42 of 137 the same), the custom screen's
dispatcher and part of its drawing, a quarter of the object kinds verbatim and a fifth close (EXE5's chips and their
objects in exelib are the starting point for EXE4's: a chip whose EXE5 code is EXE4's the same is shared as it is,
one whose code is similar is shared with EXE4's parameters, as EXE5 did with EXE6's).

**EXE4's own:** the actor and its actions (most of the actors area), the custom screen's states and its rules, Double
Soul and its transformation, the dark chips offered in battle, the emotions, the NaviCust compile and the Mod Cards,
the stat block (0x40 bytes, the same kinds of fields at other offsets), the battle flow's handlers, the link
exchange.

**For the next steps:**

- **Extraction** (`nettai-extract exe4`): the chip records, names and descriptions, the NaviCust parts, the sprites
  (the object sprite table, to find as EXE5's was), the sounds (the m4a song table), both languages.
- **exe4-compat:** the numbers (chips by id, navis, souls), the recording decode (the oracle's layout, §3.4, and the
  setup line), the save import (§12).
- **The oracle:** an EXE4 layout (the addresses of §3.4 and those still to find), its hooks (Tango's round start and
  round result, the frame routine), the four ROMs in the chip lab with Tango's twelve saves as bases.
- **Open:** the NaviStats fields beyond those §3.3 pairs (HP among them); what the object record's +0x17 byte is;
  the custom screen's states and Double Soul's offer; the dark chip offer; the emotion function; the turn-start order
  of a transformation; the panel entry's size and types (BN4 numbers holy 9, metal 5: multi-game.md §2.4); the link
  exchange's contents.
