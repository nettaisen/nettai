# EXE4 against EXE5 and EXE6: the routine map

EXE4 against EXE3 (what of EXE4's code is EXE3's, system by system): [exe4-against-exe3.md](exe4-against-exe3.md).

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
  at other offsets (§3.3); BN4's patch card effect ids are those offsets.
- **EXE4's own systems** (R: absent or differing in both maps; K for what they are): the custom screen's states, Double
  Soul (no counterpart of EXE5's soul button, its hand builder or the turn-start transformation sequencer EXE5 and
  EXE6 share), the dark chips offered in battle, the emotions (EXE5's emotion routine and mood setter have no
  counterpart), the NaviCust compile (EXE5's and EXE6's shared compile is absent), the patch cards (134 by
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

0x40 bytes a block (R). **In battle**, a side's at 0x0203BEC0 + 0x40 side (0x0800D742), and eight more from
0x0203BCC0 (0x0800D74C). **In the save**, eight blocks from the toolkit's +0x78: the save image's 0x4E60 (the save's
pointers, toolkit +0x40 to +0x98, are 0x02002130 + the shift + a table's offsets, 0x080061AC: the routine that shifts
the save, 0x08006128, sets them); MegaMan's is the first (blocks 1 to 7 hold another's defaults). The accessors
(0x0800D756 on: set, get, a byte or a halfword, by side; 0x0800D78A on, MegaMan's save block's: 0x0800D78A is the
"set_effect" Tango's patch cards call, so a card's effect id is the offset it sets). `tools/exe4/navistats.py` lists
every call with a constant offset, by offset, with the caller's EXE6 and EXE5 counterparts: 43 offsets used.

What a block holds, as far as read (R: the defaults a new block gets, 0x0800D6BE; MegaMan's HP, copied from the save's
game state, 0x0800D726; the patch cards' effects, T; the paired reads of `fields.py --to B4WE navistats`, EXE6's offset
in brackets, one to three calls each):

| Offset | Default | What |
|---|---|---|
| +0x00 | 0x99 | the mood, likely (EXE6 +0x0E: two paired reads; the played save's MegaMan has 0xAA) |
| +0x05 | 0 | the buster's attack (patch card 0x05; read by EXE6's buster routines `sub_8011A7E` and kin [+0x01]) |
| +0x06 | 0 | rapid [+0x02] (`sub_800FAAC`) |
| +0x07 | 0 | charge, likely [+0x03] |
| +0x09 | 0 | the B button's shot (patch card 0x09) |
| +0x0A | 1 | the charged shot (patch card 0x0A, "B charge") |
| +0x0C | 0xFF | B+Left (patch card 0x0C; none) |
| +0x0E, +0x0F | 0 | the HP drain and custom drain bugs, likely [+0x18, +0x19] |
| +0x10, +0x11 | 0x20, 4 | ? |
| +0x12 | 5 | the custom level, the chips dealt (patch card 0x12, up to 8) [+0x0A] |
| +0x13, +0x14 | 5, 1 | the Mega and Giga folder limits (patch cards 0x13, 0x14) |
| +0x17 | 0x1F | ? |
| +0x18 | 0 | the supports (patch card 0x18, Triple Supporter) [+0x0D] |
| +0x1B | 0xFF | the panel a step leaves (patch card 0x1B: 1 broken, 3 cracked, 5 metal, 9 holy; none) |
| +0x1F | 0 | Full Synchro at the start (patch card 0x1F) [+0x0F] |
| +0x20 | 1 | ? |
| +0x21 | 0 | the aura at the start (patch card 0x21: 2 Barrier100, 3 Barrier200, 6 LifeAura) [+0x06] |
| +0x23 | 0 | the navi, likely [+0x29: six paired reads] |
| +0x24 | 0 | the soul (patch card 0x24: a battle starts in it, 1 on) [+0x2C] |
| +0x27 | 0 | MegaMan's color (patch card 0x27) |
| +0x28 | 0 | All Guard (patch card 0x28) |
| +0x29 | 0 | fighting in the sun (the overworld sets it by the map, 0x0802A770; the reset keeps it; GunSol reads it) |
| +0x2A | 1 | ? |
| +0x30, +0x32 | 100 | HP and max HP (MegaMan's from the save's 0x2150 and 0x2152) [+0x40, +0x42] |
| +0x34 | 100 | the base max HP (the save's 0x21CA, before the NaviCust's and the patch cards') |
| +0x36 | 500 | the light/dark value (a halfword; Tango's dark save 460, its light saves 1000) [EXE5's +0x44] |

Read since: +0x08 is the buster's blank count (the buster shot, 0x080EB35A: a draw of RNG2, `(GetRNG2() & 15) + 1`, no
greater than it clicks and fires nothing; the engine's `buster_blanks`); +0x09 and +0x0A are the B button's and the
charged weapon's routines (0x0800BD62: copied to the AI data's +0x0D and +0x0F, +0x0F the charge table's row too,
0x0800BD1C; the routines at 0x0800CA7C: 0 the buster, 0x0800CC2E, 1 the charged shot, 0x0800CC54); +0x25 is the move
lag's column (0x0800C208).

The NaviCust's handlers and bugs (§8) write: +0x01 to +0x04 super armor, FloatShoes, AirShoes, Undershirt; +0x0B
BustPack's weapon level (0 to 2, copied to the AI data's +8 at the init, 0x0800D8D2); +0x0D the move bug (0xFF
confused 720 ticks as the round starts, 0x0800D8B0; else its high nibble, in the keys' bits, is held when no direction
is, 0x0800B4DA); +0x15 the encounter bug; +0x16 SneakRun; +0x17's bits 1 to 4 OilBody, Fish, Battery, Jungle; +0x19
Collect (bit 1) and the result bug (1); +0x1C Humor; +0x1D BugStop; +0x1E SoulClen (read by the light/dark value's
update at a round's end, 0x0800F5BC). The rest (+0x22, +0x26, +0x2B) is used and unread yet: the step that ports
what reads it names it. +0x29 is fighting in the sun (the rules' `sun`, as EXE5's +0x22): 1 while the map the
overworld is on is one of 0x0802A7AC's (0x0802A770), which the save's block keeps; GunSol hits harder and shines
brighter in it.

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
| its sentinel | 0x02009AB0 | 0x0200ABA0 | 0x0200A4C0 | R: the object loop, 0x0800314C |
| the sound queue | 0x0200A490 | 0x0200B170 | 0x0200A800 | R: its append, 0x0800073C; entries from **+4** (EXE6 +0xC, EXE5 +8) |
| the chip blocks (hands) | 0x020349C0 | 0x02034E20 | 0x02035CB0 | R, T |
| the input records | 0x02036820 | 0x02036E90 | 0x02037C40 | R |
| the banner | 0x02036840 | 0x02036ED0 | 0x02037CE0 | R: its fields as EXE6's (+0x00, +0x07, +0x08), 0x08016188 |
| the HUD's block (EXE6's 0x02035280) | 0x02035280 | | 0x020363F0 | R: the custom gauge at **+0x24**, its rate +0x26 (EXE6 +0x20, +0x22: 0x080168BE, 0x0801592C), the HUD's task mask at **+0x48** (EXE6 +0x40: 0x080146A8) |
| the battle folder | 0x0203CDB0 | 0x0203C830 | 0x0203BE80 | R: 30 chips, 0x3C bytes (0x08007AE8 copies 60 bytes), NaviStats after it |
| the panels | 0x02039AE0 | 0x0203A100 | 0x0203AC70 | R: 0x20 bytes an entry, as EXE6's (`_object_getPanelDataOffset`, the same code, 0x0800A3D8); the type at **+0** and the owner at +1 (EXE6's +2, +3), seen on a running console |
| the fighting machine | 0x0203CA70 | 0x0203C5D0 | 0x0203BCB0 | R |
| NaviStats | 0x0203CE00 | 0x0203C880 | 0x0203BEC0 | R |
| the link struct | 0x0203F7D8 | 0x0203F244 | 0x0203F6D4 | R; its status at +1 (0x08017B88) |
| the custom screen's state | 0x020364C0 | 0x02036B10 | 0x02036440 | R; T's "custom flags" |
| the actor pool | 0x0203A9B0 | 0x0203B200 | 0x0203B180 | R, T |
| the m4a players | 0x02010690... | +0xBE0 | **+0x1210** | R: the player table 0x08000654, EXE6's moved by one amount |

What the oracle reads besides (R; oracle-trace's `EXE4`):

- **The pause flag** is in the game state the toolkit's **+0x40** points at (EXE6's +0x3C), at its **+9** (EXE6's
  +0xA): the object loop reads it there (0x0800316C). The game state is in the save's region the save's shift moves
  (0x02002130 on, §3.3, §12), so the address is the pointer's, read each time.
- **BattleState** (0x02035810, the toolkit's +0x18) keeps the local side at +0x0D, the settings' pointer at +0x3C
  (written once, 0x08007ACC), the flags at +0x32 (`battle_setFlags`, the same code), the wins and losses at
  +0x18/+0x19 and the Regular chip's flag at +0x17 (written by the folder's builder, 0x08007B44), as EXE6's; its
  +0x08 is read where EXE6's +0x07 is (the battle's mode, likely), and it has fields of its own at +0x44 and +0x64.
  It has **no frame and tick counters** (EXE6's +0x60, +0x64): the frame routine counts nothing.
- **No transform records** are exchanged (EXE6's 0x0203F558 and 0x0203F658: their copy, `sub_800840C`'s, has no
  counterpart in 0x08007064).
- **The joypad's** repeat beat is at +0x13, cycling 0 to 4, as EXE6's (0x080003B0).
- **The save** is the RAM from 0x02000000, the region from 0x2130 to 0x5E20 moved by the shift word at 0x1550
  (§12): the NaviCust's parts at 0x4564 and its grid at 0x4540, the patch cards' slots at 0x464C (on) and 0x4653
  (off), the color bar at 0x190 (outside the region).

**On running consoles** (the chip lab's EXE4 base, Red Sun against Blue Moon, Tango's primer): the hooks trace
every battle frame from the battle's first; BattleState's state byte is 0 before the round (its sub-state counting
the intro: 4, 8, 0xC), then **4** for the round, its sub-state 0 (the entry), 4 (the start's banner), **8** the
custom screen, **0xC** the fight; the fighting machine's first byte is **4** while the fight runs with inputs live
(EXE6's 8); a navi standing idle is in state 4, action **6** (EXE6's 8; EXE5's 6); BattleState's **+0x44** points at
the local player's navi and **+0x48** at the other's (EXE6 keeps alive lists by alliance at +0x80); the custom gauge
(0x02036414) fills from 0 to 0x4000 (0x5A0 a second; the rate field beside it holds 0x20); the panels' types and owners are as the stage shows
(grass at the edges, lava in the middle, each side's). The setup line's NaviStats are the compiled stats: a save's
base HP (0x21CA) and its patch cards reach them (a patch card's +200 max HP, 1000 to 1200), the save's stats block itself
is rebuilt as it loads.

**The hooks** (R; oracle-trace's `RED_SUN_HOOKS` and the others, checked against the four ROMs by its
tests/hooks.rs). The battle runs as the main loop's **subsystem 8**: the frame routine, 0x08006B14 (EXE6's
`battle_8007800`'s counterpart: BattleState's state byte through its table, 0x08006B30: states 0, 4 (the round's
loop, 0x08006CB4), 8, 0xC and 0x10), is the third word of the main loop's subsystem table (0x0800032C), entered by
its dispatch's `mov lr, pc; bx r0` and returning to the loop at **0x08000300**; no link applet calls it (EXE6's and
EXE5's netbattles run it from theirs). The frame starts after the wait at 0x0800036C (0x080002B8). The m4a calls
are those the sound requests queue (their literal pools from 0x0800075C): SongNumStart 0x08112848, MPlayAllStop
0x0811297C, VolumeControl 0x081137F0, FadeOut 0x081127A4, SongNumStop 0x08112914, ImmInit 0x08112A38, FadeIn
0x08112A10; EXE4 queues no tempo request and no conditional music; its pitch control is EXE6's code (0x08113858).
Blue Moon US's are Red Sun's with the sound library 0x14 further; the Japanese ROMs' have the frame routine 0x20
earlier (0x08006AF4) and the sound library 0x194 earlier (`versions.py B4WE B4WJ map`). The round's start and its
result are Tango's primer's traps (§4), which the chip lab takes from tango-gamesupport-bn4 as it does EXE5's.

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

**The screen's block** (0x02036440; R, and seen on running consoles as the chip lab drives it): +0 its state (0, 4
running, 8 done), +1 the running state's sub-state (the table 0x0801E228: 0 opening, **4** the chip selection, 8
closing, 0x18 waiting for the other player), +2 the selection's own (the table 0x0801E430: **4** the cursor on the
chips, **8** on OK, 0x18 a chip's description), the cursor's column and row at **+0x0A** and **+0x0B**, the chips
selected at **+0x12**, and from **+0x20** the slots, a halfword each, row by row, five a row: the chip's byte offset in
the battle folder (0x0203BE80), then its flags (1 dealt, 2 selected, 4 not selectable now: another code or kind than
the picks'). Its selection's keys (0x08020350): A picks, B takes the last back, R describes, START puts the cursor on
OK, RIGHT from a row's last chip too, UP and DOWN change rows. The hand dealt is the battle folder's first chips (the
folder shuffled, a chip taken out of it as the screen closes); the chip blocks (0x02035CB0) take the picks as the
fight starts.
none of EXE5's custom-screen logic that matters for a netbattle is found: what can be picked (`sub_8028E32`), OK's
hand builder (`sub_8029110`), the keys (`sub_8028B74`), the dark chip's cursor (`sub_802806C`), the soul button's
offer (0x08024B28) and its family table (0x08024BE0): all absent in both maps.

**Double Soul** (K, T): sacrificing a chip of a soul's kind on the custom screen; twelve souls, six a version (Red Sun:
Roll, Guts, Wind, Search, Fire, Thunder; Blue Moon: Proto, Number, Metal, Junk, Aqua, Wood), three turns, once a
soul a battle (?). **Dark chips** (T, K): never in a folder, offered only in battle (T, multi-game.md §2.4), when
MegaMan is worried (?); a chip with the dark flag (+0x09 bit 5). How the offer is drawn, which slot it takes and what
using one does (the dark state, no Double Soul afterwards: ?) are EXE4's own code, to read.

**As ported** (group A, the engine's custom/screen.rs on EXE4's rules; §18 item 31). The engine runs one screen for
every game; what EXE4's code does otherwise is its rule section `custom_screen` (written by tools/exe4/gen_rules.py
from the ROM and its code, read routine by routine):

- **The block's states as the engine's phases.** The running state's sub-states: 0 the slide in (`Phase::Opening`,
  10 ticks, the open sound 0x7A on the first), 4 choosing, 8 the slide out (`Closing`), 0xC SELECT's hiding
  (`Hidden`: no sound, no emblem drawn on the return), 0x10 the Program Advance animation (`ProgramAdvance`: EXE6's
  steps, its fade back a step longer by `effects.fade_clear`), 0x18 the send and the wait (`Sending`; 0x14, the escape
  chatbox, no netbattle reaches). The selection's states inside choosing: 0 the settle tick (`Settling`: the state it
  was in put back, no key read, value 1 of the status set), 4 the chips, 8 OK and 0xC the button under it (the
  engine's cursor on slot 10 or 11), 0x10 the soul's animation and 0x14 SearchSoul's shuffle (the rules' windows),
  0x18 R's description (`Description`), 0x1C L's message (`RunMessage`, its script run with the key).
- **The keys and moves** (`slots`): each place reads its own keys in its own order (the chips' 0x08020350, OK's
  0x08020620, the button's 0x08020728) and moves to the first slot of a list that holds something (0x080204C0,
  0x08020518, 0x08020568, 0x0802070C, 0x080207F4). Slots 8 and 9, the dark chips' places, show no frame without a
  chip.
- **Picks** (0x0801F73C): a chip that counts as the invalid chip is refused (`invalid_picks`), the dark chips' code 27
  is one `*` doesn't stand for (`special_codes`). **OK's hand** (0x0801F034): no Program Advance record, no Regular
  mark on it, the entries past the selection cleared, a modifier's mark kept (`program_advances`,
  `modifier_passes_regular`). The hand is the custom level, uncapped (rules/custom: `hand_size`, 0x0801DC8E).
- **The status** (`status_until`, `custom::Side::selecting`): value 4 from the opening (0x08007618) to the send
  (0x0801E986), value 1 from the settle tick (0x08020348) to OK (0x08020652).
- **The gauge**: L or R of either joypad with it full asks for the screen on that tick (flow `custom_request`,
  0x08007A2E); it stays full until each console's send (`gauge_empties_at_open`, `Battle::gauge_for`).
- **The dark chip hover** (0x0801E478, `hover`): only while choosing; an 11-tick ramp with a volume call every other
  tick (players 31 and 9, the table 0x0801E584); its sound 0x100 on the tick after the shade settles and every 61;
  OK clears the fades at once (`fades_clear_at_ok`); the close sets players 9 and 31 back (`restore_players`).
- **Drawing** (presentation): the tick a key leaves the choosing draws what its new state draws, no cursor
  (`cursor_after_leaving`, 0x0801E412). The choosing tick draws the last turns' block and counts its frame before
  its keys (`frame_counts_first`, 0x0801E3DA, 0x0801E3DE): the cursor blinks a frame further on than EXE6's, and the
  Regular chip's frame tests the counted frame for 1 (0x0801EF12), the same tick as EXE6's test for 0. R's
  description, L's message and the rules' windows are states of the choosing (`states_in_choosing`: the selection's
  0x18, 0x1C and 0x10): their ticks draw the last turns' block and count the frame, a window's sprites after its step
  (the soul's choice's icon from the tick it loads, 0x08020DE2), and the tick one goes back draws the state it goes
  back to, its cursor (0x08020A0A, 0x0802095E, 0x0801E412).
- **L's message**: MegaMan's (the archive 0x08749294's entry 3, its words in both locales), EXE6's script shape, its
  portrait `megaman-portrait` (F4 00 40: the mugshot table 0x08028038's 0x40); a character ends the tick's printing
  (`chatbox_character_ends_tick`, 0x0804E1B2: a character every third tick, where EXE6's interpreter goes on and
  counts the next one's delay down on the same tick). Checked against custom/run-message, run-message-b and
  run-message-ok: every frame while it is up, its portrait's fades and faces too.
- **Sounds** (named for their code in gen_content.py's BY_USE): open 0x7A, cursor 0x7D, pick 0x7E, back 0x7F, OK 0x80,
  refused 0x69, description 0x66, the hover 0x100, the Program Advance's parts 0x79 and its result 0x97, the gauge full
  0x81; the hide, the description's close, L's message and the pause play none (optional roles).

Checked against the chip lab's custom/ recordings (every one whose chips the content has matches every frame and
every sound call; with exe4-compat's harness comparing the gauge as the recording console holds it).

**Double Soul's part** (group A, rules/souls/custom; §6 and item 73):

- **The UNITE button** in slot 11 under OK (the selection's state 0xC, its keys 0x08020728), the screen's +0x0C: on the
  screen as it opens (0x0801E0B4) with Double Soul (event flag 0x14: the setup's `double_soul`) unless the navi is
  worried or worn out (emotions 1 and 5), else its place shows the window (the look's `hidden`) and OK's DOWN goes
  nowhere. Its look (0x0801FF14): lit while the last pick offers a soul, gray while it doesn't and once one is given.
- **The offer** (0x0801FF84, the screen's +0x0D, after each pick and take-back): asked only before a soul is given and
  while the navi's soul turns count (AIData +0x18 not 0xFF); the last pick, not the Regular chip, of a family a soul
  of MegaMan's is for (0x08020008: the soul's form's `soul.family`), a soul the save has (its version's flag,
  0x08020018: the setup's `souls`) and not given this round (0x0203BF88, which the battle's start zeroes).
- **A on it** (0x0801F8BC): with a soul on offer OK's sound (0x80) and the soul's choice; else refused (0x69). **The
  soul's choice** (the selection's state 0x10, 0x08020814, the window `double_soul`, EXE5's soul window's steps): the
  soul's chip's icon (chip 0x160 + the soul, 0x0802084E) flies up over the picks (16 ticks, then 8 up 2 pixels a
  tick), the flash (fades 0x34 and 0x30 with sound 0x79), the whitening (fade 4), then white the soul takes the given
  chip's place first in the picks, the button is picked, the screen clears (fade 0, sound 0x97); each step after the
  flash waits for its fade. Taking the soul back (B with it last) puts the chip back (0x0801F8FC).
- **OK** (0x0801F034): the given chip leaves the folder (an offered dark chip counts sent: rules/dark_chips), the soul's
  chip goes first in the hand (the button's `hand_chip`: the form's `chip`, code A), the soul is given this round
  (0x0801F1FE) and is the turn's form (`custom.set_form`).

**The dark chips** (group A; rules/dark_chips, the chips' own modules):

- **The offer** (0x0801DE90, from the screen's opening, 0x0801DD02): in a netbattle (battle type 0x46 and on) to a
  MegaMan worn out (emotion 5, a mood of 0: a dark MegaMan's) alone; the other branches are the story's. Two
  different draws of the console's RNG1 (the low six bits) from a list of 64 (0x0801ECB0, or with the navi's HP at
  half its maximum or less 0x0801ECF0), shown in slots 8 and 9 in code 27 with the flags 0x21: the second only while
  the side has been sent fewer than nine this battle, neither from ten (0x0203F6D0; the draws are drawn all the same).
  The cursor starts on slot 8 (0x0801E110). OK counts the offered chips the hand takes (0x0801EF92).
- **The costs** (0x0801EA1E as each side's hand is installed; the rules' `custom_result` hook): by 0x0801EAF4, each a
  NaviStats byte of the user's through 0x0801EA9A's rules. DrkCanon: the charged shot becomes weapon routine 0x21,
  the taunt (action 0x2A, 0x080ED0FE: its mark, sound 0xC1, and in a netbattle the other side's anger request); the
  weapons reload every fighting tick (0x0800D9E8 from the intake). DrkSword and DarkBomb: the move bug, 0x10 and 0x20.
  DrkVulcn: 720 ticks of confusion (status 0x21). DrkLance: the custom drain (+0x0F: 6, else 4 up to 6, else 3).
  DrkSpred: the panel trail, poison (+0x1B = 4). DrkStage: the custom level one less, down to 2. DrkRecov: the custom
  drain as DrkLance's and the HP drain (+0x0E: 10, else 6 up to 10, else 3). The mood: none (a mood of 0 stays).
- **The chips**: laid out as EXE5's and EXE6's, each series in its folder with its action, the shared ones in lib/, on
  @exelib where EXE4's routine is EXE5's or EXE6's code (its differences as the look's or game's data): lib/cannon
  0x0B (@exelib/cannon), lib/swords 0x0A (@exelib/swords/slash), lib/bombs 0x09 (@exelib/bombs/throw and bomb),
  chips/vulcan/vulcan 0x1F (@exelib/vulcan/action), chips/spreader/spreader 0x1E (@exelib/spreadr/action),
  chips/recov/recov 0x1D (@exelib/recov/heal), chips/lance/lance (@exelib/lance) on lib/spawners 0x20 (EXE4's own,
  emap: absent; lib/plus its variant 4), and the dimming action 0x0C with DrkStage's controller, effect #0x6A, and
  the panel changer, effect #0x1F (@exelib/panel_changer). The records are tools/exe4/gen_content.py's. DrkStage
  is EXE4's first dimming chip: its telop runs on the banner with no banner of its own (item 71).

Checked: the lab's dark/offer, hover-long, hover-hide, hover-describe, hover-then-ok-fast, with-folder-chip, side1,
bluemoon, drksword, darkbomb, drkvulcn, drkspred, drkstage, drkcanon and drkcanon-taunt match every frame; drklance
and drkrecov match through the chip's use and the next turn and stop at the screen after on the custom drain (item
72). Their sound calls match but for the hover's music fades after OK, which the harness's late OK shifts (item 72).

## 6. Transformations

EXE5's and EXE6's turn-start transformation sequencer (`sub_801483C`, `sub_80148CC`, `sub_8014944`, `sub_8014A00`,
the transform records' copy `sub_80147E4`) and the transform record's writer (`sub_8015952`) have no counterpart in
EXE4 in either map. So Double Soul's change is EXE4's own path.

**As ported** (group A, rules/souls; item 73). A soul is a form of MegaMan's, his one list of the twelve in their
numbers' order (compat's numbers, records.toml); the souls a side has are its setup's `souls` (the content has no
version: a save's version is the boundary's, exe4-compat's, which maps it to its souls, and the app's presets name
each version's six); each soul's folder holds its form and its chip
(navis/megaman/forms/<soul>/chip: chips 0x161 to 0x16C), whose use is the soul's own change (rules/souls/change's
maker; compat names them all action 0x0D, their images all effect 0x13).

- **The ask** (0x0800B658, each tick of the intake but a dimmed one: the rules' `navi_intake`): held (flag 0x10000)
  nothing; a chip of the special family (15) next in the hand asks for the change (the engine's form change request;
  its soul, 0x0800B924's chip less 0x160, is the turn's form, which OK gave); in a soul whose turns are over the revert
  (+0x44's 0x40).
- **The change** (action 0x0D, 0x080EBA44), as the fight runs: the status routine starts it (0x08013B14) the first
  unpaused tick after the turn's banner, and runs it whatever else while it holds the navi (0x08013AFC: the engine's
  unpaused form change, a game whose flow has no `sequencer_at_turn_start`). It dims the battle itself; its steps
  are in rules/souls/change: the flash and sound 0x129, the fade 0x44 and the HUD hidden, the image (./image, 60
  ticks, its second flash and sound 0x158 at 10), MegaMan in the soul (sprite 00-01 + the soul, the form; the chip
  icons over him stay by his navi, 0x0800B9C0), the fade 0x40 and the HUD back, the status reset (the soul's turns:
  its form's `soul.turns`, 3 every one; the emotion window counts down the turns left, 0x08016A20), 20 ticks, the
  lockout 2 its start set (0x08013B1C) handed on, the hand on past the soul's chip.
- **The turns** (AIData +0x18, ./turns): each custom screen asked for counts one (0x0800BA20); at 3 the intake asks
  for the revert, which his idle starts (0x080EEBB4: the engine's idle, the form's `revert`): **the revert** (action
  0x11, 0x080EBE90), one tick: sound 0x159, the flash, his base form, the status reset.
- **The statuses' end** the change runs (0x08013218) clears EXE4's four statuses (the reactions' `status_end`), not the
  flag 0x10000 that holds him.

What each soul does besides (its status routine 0x0800E0A0, its charged shot, B+Left, the image's parts, WindSoul's
change object, FireSoul's and NumberSoul's overlays) is item 74.

## 7. Emotions

EXE5's emotion routine (0x0801270C), its link-battle form (0x080127C0), the mood setter (0x080127D6), the anger tick
(0x08011A14) and the starting mood (0x08010EC8) have no counterpart in EXE4 by EXE5's map (EXE6's map pairs EXE6's
`sub_8015B54`, `sub_8014326` and `sub_8013892` with EXE4 routines only as similar, unread). EXE5's light/dark mood
helper (0x0801283A) is EXE4's 0x0800F56A with other constants; Full Synchro on a counter (EXE6's `sub_801A200`) is
similar 0.65 at 0x080131E4. EXE4's emotions (K): normal, Full Synchro, angry, worried, the dark state; their triggers
and faces are its own table, to read.

**As ported** (group A; the status section's `emotion`, rules/light_dark, the roles):

- **The emotion** (0x0800F49C): 0 normal (a mood of 65 or more), 1 worried (under 65), 2 Full Synchro (0xFF), 3 angry
  (AIData +0x2E: anger's ticks), 4 in a soul (NaviStats +0x24 not 0 nor 15, its soul in r1), 5 worn out (a mood of 0:
  the dark MegaMan), read in the order soul, angry, worn out, Full Synchro, worried, normal. Worn out and in a soul
  keep anger from starting (0x0800C560 also tests NaviStats +0x23, the navi, and the object's +4: MegaMan alone, no
  other navi plays a netbattle); the buster knows no emotion; the custom screen offers souls to none but 1 and 5
  (0x0801E0B4) and dark chips to 5 alone in a netbattle (0x0801DE90).
- **The mood** (NaviStats +0x00): the setters (0x0800F4DE set, 0x0800F4FA raise to 0xFF, 0x0800F51E raise to 0xFE
  unless 0xFF, 0x0800F546 lower to 1) leave a mood of 0 and do nothing while paused. The starting mood (0x0800D872,
  0x0800F56A) by the light/dark value (NaviStats +0x36) and the Full Synchro at the start (+0x1F, a patch card's):
  under 470 0, the start 0xFF, from 1000 190, else value / 20 + 128. A hit's counter byte (the hitter's collision +5)
  raises the hitter's mood and wears the receiver's (0x08012C10, 0x080131E4: `hit_mood = "hitter_gains"`), a counter
  hit's counting 0xFF and wearing 0x7F. Anger: 120 ticks flinching or paralyzed (0x0800C540) ask for it, and no
  damage does (EXE6's and EXE5's 300 do: `emotion.anger_damage`, none for EXE4's); it sets the mood 0x80 and lasts
  600 ticks (0x0800C560); its end sets 0x80 through the setter.
- **Full Synchro**: a chip's boost doubles it, leaves the mood at 0x99 and sounds 0x1BB; anger's boost ends the anger
  without a sound (0x0800D54E). The aura (actor 0x5E, 0x080CD180, EXE5's code) spawns for a navi in Full Synchro
  (0x0800D9AE) and waits while paused (0x080CD276 sets no header flag). The window's faces: the base form's mugshots
  by emotion (group D).
- **Counters**: Cannon opens a 16-tick window in a netbattle (0x0800BA66); a counter hit paralyzes (status byte 0x12,
  0x0800B08E), closes the window, shows COUNTER and sounds 0x10A (0x08013410; 0x73 too when the hitter was in Full
  Synchro, hit flag 0x80, 0x08012CC0).
- **Light and dark** (rules/light_dark): a dark MegaMan clears the holy panel he stands on (0x080132E6); a navi that
  isn't dark closes a hole (0x08013318, §18 item 12).

Checked: the lab's emotions/counter, counter-side1, counter-buster, full-synchro-hit, full-synchro-card and worried
(M-Cannon's) match every frame and sound call. Not yet: the COUNTER text's battle-over gate and the 0x73 sound
(presentation).

## 8. NaviCust and patch cards

The NaviCust compile EXE5 and EXE6 share (`sub_813C458`) and its placement checks have no counterpart: EXE4's compile
is its own (R). 47 programs, four color variants each (188 parts; the part table at 0x0804563C in Red Sun US,
0x08045644 in Blue Moon US, 0x08045538 and 0x08045540 in the Japanese: EXE5's format, +1 the plus flag, +3 the color,
+4 the bug, +8 and +0xC the shapes, its +0 group never read). The save's NaviCust: its list at 0x4564 (25 parts of 8
bytes) and the 5x5 grid at 0x4540 (T). **Patch cards** (改造カード): 134 by Tango's count, six slots in the save
(0x464C on, 0x4653 off, T; the reload reads a seventh, 0x4652), whose effects set NaviStats bytes by number (§3.3),
their handlers the table at 0x08041E8C, their bugs counted with the NaviCust's (0x080476E0) or cleared (0x080476EC).
A card's slot is the save's: the reload runs the card in each slot, the last first (0x08035164), with no table of
which slot a card belongs in (the ROMs hold none; Tango's lists say the printed cards'), and the pairs' halves test
the other's slot (0x08042504).

**The reload** (0x08035130; as ported: content/exe4/rules/navicust): the analysis (0x08047344) counts each program's
bug by where it is (the command line, the third row, right to left: a plus part on it; off it, a program), by its
neighbors (its shape uncompressed, moved a cell each way: one of its color counts that one's bug), by HubBatc (once
more) and by the colors (0x08047644: five bring the move and custom bugs once, six twice); the stats reset keeping
+0x00, +0x20, +0x29 and +0x36 (0x08036CC0); the programs' handlers (0x08041974, the table 0x08041A50: the command
line's right to left, a part already run to the right skipped; the plus parts off it in the list's order, then on it
left to right), the HP programs' sum making the maximum (0x08042FD0: the HP left as it was); the bugs (0x08042A94:
BugStop, +0x1D, drops the counts instead), each at its count's level, 1 to 3, by the table 0x08042B18; the patch cards
from the last slot to the first (0x08041E6A), the maximum again after each; the bugs again, all of them (0x08042A58).
tools/exe4/gen_navicust.py (verify) writes the programs (content/exe4/navicust), their numbers
(compat/navicust.toml) and names from the part tables; programs 29 to 33 and 40 (the elements' charged shots and
WeapLV+1) have no colored part, so no save holds them, and have no definition.

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
| patch cards | | 134 | T |
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
the equipped folder at 0x2132, the Regular chip at 0x214C; the NaviCust and patch cards (§8). Tango ships twelve raw
netbattle saves (tango-gamesupport-bn4/src/saves): light with HP 1000 or 999 and dark with HP 997, for each version and
region; with the .sav on disk (Blue Moon US), the chip lab's bases and the save import's tests have what EXE5's had.

**The import** (exe4-compat's `import`, which the app's build creator calls): a .sav or a raw image (which says
neither version nor region; the import reads neither) gives MegaMan, the equipped folder and its Regular chip, the
NaviCust's programs, the base HP (0x21CA), the Regular memory, and from MegaMan's NaviStats block (0x4E60) the
light/dark value and the Full Synchro at the start; the patch cards are said and left out until they are ported (item
56). MegaMan's HP and maximum are the game state's (0x2150, 0x2152): the block's own HP words are stale in a save,
and a battle copies the game state's in (0x0800D726). verify's exe4_navicust test imports each of Tango's twelve
saves and compiles it: the stats match the block's (and the game state's maximum) in every byte the compile writes.

## 13. What to share, what is EXE4's own, and the next steps

**Shared through the engine and exelib** (the same code in EXE4, by the maps): the object system and pools (with
EXE4's slot counts per pool), the object record's fields (but +0x0E's byte), the RNGs, the chip record and getter (but
the three bytes +0x06 to +0x08), the panel grid's helpers (object.s: 42 of 137 the same), the custom screen's
dispatcher and part of its drawing, a quarter of the object kinds verbatim and a fifth close (EXE5's chips and their
objects in exelib are the starting point for EXE4's: a chip whose EXE5 code is EXE4's the same is shared as it is,
one whose code is similar is shared with EXE4's parameters, as EXE5 did with EXE6's).

**EXE4's own:** the actor and its actions (most of the actors area), the custom screen's states and its rules, Double
Soul and its transformation, the dark chips offered in battle, the emotions, the NaviCust compile and the patch cards,
the stat block (0x40 bytes, the same kinds of fields at other offsets), the battle flow's handlers, the link
exchange.

**The first chips** (the nine of Tango's saves' folders, group B's and A's, the pattern of the chips wave): each a
chip module (`chips/<series>`) composing its action's family (`lib/<family>`) from its variant's rows, its record's
fields by hand from the record (§3.2), its names and descriptions the ROMs', its action number in
compat/actions.toml; each verified on its lab recordings (hit, adjacent, miss, side1) and frame-compared.

- **AirShot** (action 0x23, `lib/airshot`): @exelib/airshot (emap: its fire and recovery EXE5's, similar), with the
  differences as its look's and spec's parameters: the navi's animation (12, not 9), how the navi holds the shooter
  (ShooterLook's `hold`: an attachment of EXE4's kind, row 9, its animation the navi's number, in the related slot,
  not the overlay) and the recovery's count (the attack's variant: AirShot's 0, the store of 10 overwritten,
  0x080ECC0A; 30 for any other). The projectile (exelib's, EXE4's row 4: hit modifier 0x21, EXE5's 0x61) and its
  spawn are shared.
- **CrakOut** (and DublCrak and TripCrak: action 0x21, `lib/crack`): EXE5's code, moved to exelib
  (@exelib/crack, EXE5's on it). EXE4's look: the pointing animation (16, EXE5's 12), the crack's dust and sounds
  (0x1BA, the break's 0x1A9: 0x08009D04), and a spawner (0x080CFF1A) that places a crack on every panel of the
  pattern where EXE5's keeps to the field (no recording yet reaches a crack off the field: TripCrak from the top or
  bottom row).
- **AreaGrab** (and PanlGrab: the dimming chips' action 0x0C, its variant 0, `lib/grab`): @exelib/grab (EXE6's and
  EXE5's controller and shot) with EXE4's finders as the controller's `column` and `panel` (the other side's
  front-most panel across the rows, 0x0800A1E4; the user's from the far edge, 0x0800A1AE) and the shot sparing the
  other side's back column (`spares`, 0x080CF0A6); the dimming, its telop and the controller's phases are the engine's
  and @exelib/dimming's, as EXE5's.
- **Sword, WideSwrd** (with LongSwrd, WideBlde, LongBlde: action 0x0A's variants 0 to 4), **MiniBomb** (action
  0x09's variant 0, bomb variant 0), **Vulcan1** (with Vulcan2 and 3: action 0x1F's variants 0 to 2) and **Atk+10**
  (with Atk+30: action 0x20's variant 4, a plus chip, `modifier = "attack_plus"`): each a chip of group A's families
  (lib/swords, lib/bombs, chips/vulcan/vulcan, lib/spawners and lib/plus, each on @exelib), composed from its
  variant's rows; nothing of its own.

**The chips wave** (the rest of the chips, family by family, the same way):

- **Spreader, HeatShot, Bubbler** (with Heat-V, HeatSide, Bub-V and BublSide: action 0x1E's variants 0 to 2,
  `chips/spreader/spreader`, DrkSpred's family on @exelib/spreadr): each variant's gun (attachment rows 0x0A, 6 and
  0x0E), the tick it fires past and its height (0x080EC814's rows), and the chip's bullet (the bullet's rows 3, 8 to
  10, 4 to 6), each chip's own; nothing of their own.
- **Navi+20** (action 0x20's variant 4, its parameter 1: the Navi+ bonus, `modifier = "navi_plus"`): lib/plus, as
  Atk+10's.
- **Meteors1** (with Meteors2 and 3: action 0x20's variant 0x11, `chips/meteors`): the spawner and its dropper
  (effect object 0x81) are EXE5's code that no EXE5 chip uses (0x080E9DA2, 0x080E9D04), ported here; the target's
  panel is found with @exelib/panels' `closest_in_row`. The meteor and its marker are @exelib/meteors' with EXE4's
  look, which states two differences: the hit is set on the meteor's panel whether it bursts there or not
  (`hit_on = "landing"`), and the marker is left side 0 (`marker_takes_side = false`).
- **Boomer1** (with Boomer2 and 3: action 0x20's variant 2, `chips/boomer`): @exelib/boomer's boomerang, which is
  EXE5's code with EXE4's constants (its sound, 0xAB) and EXE5's variant rows; thrown from the back column by the side
  alone (EXE4 objects have no flip).
- **FullCust** (action 0x20's variant 7) and **Repair** (its variant 0x0B): EXE5's routines (FullCust's 0x080096C4,
  0x0800E26C) as spawners, on the engine's gauge and its field object registry; FullCust's branch for a navi whose
  NaviStats +0x26 is 2 (a story's other navi) has no netbattle navi to take it.
- **AquaUp1, GreenWd1, Ligtnin1** (with their 2 and 3: action 0x20's variant 0x0C, their parameter 0, 1, 2,
  `lib/towers`): EXE4's own. A controller (attack object 0x93; each chip's own with its strike, which the parameter
  picks in the original) sweeps the columns from the user's back one, a column every 11 ticks, raising an aqua tower
  from each cracked panel of the other side's holding its navi, a wood tower from each grass one (objects/tower, attack
  0x92's rows 0 and 1: each chip's own with its look), or striking a lightning onto each panel holding an
  obstacle (objects/lightning, attack 0x8E: EXE5's unused code; its eight hits around and its ring of sparks). The
  lightning waits on RockCube's recordings (the lab's ligtnin*/obstacle). The towers' frames differ from mGBA's on a
  few ticks where mGBA shows the tower's previous frame or a mix of two (big sprites: the original's display falling
  a frame behind, not its state).
- **Counter1** (with Counter2 and 3: action 0x20's variant 0, `chips/counter`): EXE4's own spawner and EXE5's unused
  strike (attack 0x17, EXE5's 0x080DF93C's code): a strike with a hitbox on each navi of the local console's list
  (BattleState +0x44, from the other side's place: three entries for a user of side 0, one for side 1) whose counter
  window is open.
- **MokoRus1** (with 2 and 3: action 0x20's variant 1, `chips/mokorus`): EXE4's own. Three Molokos (attack 0x2C), one
  a row (the rows shuffled with RNG2: @exelib/panels' `shuffle`), 10 ticks apart, charge from 140 pixels behind the
  field's middle, in the chip's palette (twice its parameter).
- **SidBmbo1** (with 2 and 3: action 0x20's variant 5, which holds the user 40 ticks, `chips/sidbmbo`): EXE4's own
  bamboo (attack 0x44) swings down three columns ahead, its two hitboxes on rows 1 and 2.
- **WhitWeb1** (with 2 and 3: action 0x20's variants 8 to 10, `chips/whitweb`): webs (attack 0x3D, EXE5's
  0x080CCC08's code) on the other side's panels of row 1, 2 or 3, bodies with 1 HP that catch the other side's navi.
  They showed that EXE4's navi body hits for nothing in a link battle (`status.link_body_damage`: EXE4 has neither of
  EXE6's stores of 10).
- **ElemFlar, ElemIce, ElemLeaf, ElemSand, ElemDark** (action 0x60's variants 0 to 4, `chips/elem`): EXE4's own action
  with EXE5's ElemRage flame (attack 9), now @exelib/elemrage/flame: on the chip's panel the flames run five and
  paralyze (no spreading to the rows beside, which EXE5's does); each chip states its panel, its flames' palette and,
  ElemDark's, the poison its raging flames leave (the flame's `leaves`, the panel type Param4's high nibble names:
  the lab's panels/elemdark-poison). ElemDark's sound recordings differ only on the custom screen (a music volume
  change the engine makes and the game doesn't, frame 290: the custom screen's).
- **The throws** (action 0x09, `lib/bombs`: EXE6's throw on @exelib/bombs/throw): each variant's thrower
  (0x080EB4C4) and what the navi holds (0x080EB4EC: an attachment row and its animation). Variants 0 to 2 (MiniBomb,
  EnergBom and MegEnBom, DarkBomb) all throw the bomb (attack 8, @exelib/bombs/bomb) of the row their parameter names
  (0x080CE088, with its blast region at 0x080CE270 and explosion at 0x080CE277), each chip's own. **EnergBom,
  MegEnBom** (`chips/energbom`): bomb row 2 blasts nothing and leaves an energy burst (attack 0x11, @exelib/energbom's,
  EXE6's code with EXE4's sprite and sound 0xBD). (Row 1's landing spawns 0x080CE58E's object, which no chip throws.)
  **Binder1** (with 2 and 3: variant 3, `chips/binder`): EXE4's own binder (attack 0x2D) hops along the row two
  panels a hop (in place over the other side's navi after its first landing), aimed by `sub_8001330` (EXE4's
  lib/trajectory over its sine table), hitting each panel it lands on; it goes on a hole, halfway through its fourth
  hop, or past the field's sides. Its element is the byte its thrower stores from the register holding the release
  point's Y (none for a navi at rest).
  **SeekBom1** (with 2 and 3: variant 9, `chips/seekbom`): EXE4's own seeking bomb (attack 0x8D), aimed by its
  thrower at the other side's navi ahead nearest in columns (between equally near ones it compares the row itself with
  the nearest row distance so far: kept), flies 40 ticks and bursts where it lands in EXE4's sparkles (effect 0x11,
  objects/sparkles: @exelib/vdoll/sparkles whose look `stays` where its object was as they started, 0x080E3348).
  **Ball** (variant 4, `chips/ball`): EXE5's CannBall's cannonball (attack 0x35), now @exelib/cannball/ball, with
  EXE4's look. **Geyser** (variant 5, `chips/geyser`): EXE5's Geyser bomb and water (attacks 0x42 and 0x43), now
  @exelib/geyser/geyser, with EXE4's look; EXE4's splash on solid ground also takes the throw's Atk+ bonus (the
  spawn's +0x64, which EXE5's stores and never reads: `splash_bonus`). The water keeps its spawn's registers as its
  position, its height the battle state's address (the battle-over test's r3): its hits' sparks show far above the
  field (`water_height`; the lab's panels/geyser-hole and geyser-atk10).
- **Thunder1** (with 2 and 3: action 0x26, `chips/thunder`): @exelib/thunder's shot and ball (EXE6's code) with EXE4's
  look: the shot raises no arm and shows the navi's animation 0x12; the ball (attack 0x2A) is never big or fast, an
  attack with hit modifier 0, sound 0xAD, and finds its target from its side's back column toward the other side
  (0x080D21D4), not from where it is. Their parameters: 5, 7 or 9 panels, paralysis for 90, 120 or 150 ticks.
- **The Anti chips** (the dimming chips' action 0x0C, its variant 0x13, `chips/anti`): the trap chips' controller
  (lib/traps: @exelib/traps/controller, EXE6's code) makes the chip its user's side's defensive chip. AntiFire to
  AntiWood (parameters 0 to 3) set the shared ElemTrap trap (lib/traps/anti_trap: @exelib/elemtrap, EXE5's code)
  watching their element; sprung, its counterattack (@exelib/elemtrap/strike) strikes with EXE5's table but where
  EXE4's look says: the other side's navi's panels only, then that side's mood to 1, and bursts (objects/panel_bursts)
  over that side's area alone. AntiNavi, AntiDmg, AntiSwrd and AntiRecv (4 to 7) set only the record (their springs:
  see §18 item 90).
- **Guard1 to Guard3** (action 0x25's variants 0 to 2, `chips/guard`): EXE4's guard (lib/guard) with a shock wave for
  its counter (attack 0x16, 0x080CFBB0): EXE5's shock wave, now @exelib/guard/wave, with EXE4's look; EXE4's spawn
  gives a segment only its sender's side (no flip, no owner: `owned`), and its spread leaves the phase byte alone
  (EXE5's notes the panel ahead for the operation battle: `notes_ahead`). EXE4's rows never mark a panel.
- **VDoll** (the throw's variant 8, `chips/vdoll`: @exelib/vdoll/doll, attack 0x7A, and @exelib/vdoll/curse, effect
  0x4E, both of which EXE5's grew from, in EXE4's look): the doll writes no NameID, turns its landing panel to its hole
  (type 11) with the holes' sound, has EXE4's 7-row action table, no tracking, a touch destroying it with its HP as it
  is (0x08013F3E, as EXE4's rock); its leaving (0x080DC974) holds while dimmed before its other tests, checks no
  damage taken and no removal, and curses with effect row 0x2B and sound 0x141. The curse is EXE5's code (its
  telop EXE4's Curse, chip 0x174, a USED_CHIPS chip on SonicBom's action), EXE4's warning sound (0x79) and its
  staying sparkles. chips/0x067-vdoll/curse hurts a doll on an empty panel with a buster shot.
- **BugBomb** (the throw's variant 7, `chips/bugbomb`: @exelib/bugbomb/bomb, attack 0x77, which EXE5's grew from, in
  EXE4's look): one of six NaviCust bugs at random (0x080DC134: 1, 2, 3, 4, 6, 8, RNG2), whatever its target has,
  raised by code 0xFF; a crushing hit as it sits bursts it at once (0x080DC23A) where EXE5's explodes it; its burst
  hits with its sitting types (no retype). Bug 8, the weapon bug (rules/navicust's `weapon`: 0x08042E44, levels 1 to
  3 the charged shot's routine 0x1F, 0x20, 0x21), is in: navis/megaman/weapons/bug_shots's rock cube (routine 0x1F,
  action 0x73: EXE4's rock, objects/rock, row 1, @exelib/rock in EXE4's look) and bubble (routine 0x20, action 0x29;
  card 108's B button too), and the taunt (whose charge row was misread: row 0x21 is 180 at Charge 0 to 4). It takes
  effect at the weapons' next reload (the next custom screen): chips/0x029-bugbomb/weapon-bug (seed 41 draws bug 8)
  charges the rock cube after it.
- **BlkBomb** (the throw's variant 6, `chips/blkbomb`: @exelib/blkbomb/bomb, attack 0x47, which EXE5's grew from, in
  EXE4's look): no identity of its own, no field-object tracking (EXE5's 0x0802EFFA), a fire hit leaving its HP, a
  touch destroying it with its HP as it is (0x0801416C reads no removal request), its destroyed action (0x080D6320)
  bursting it as it finishes, a neutral body. Its collision's damage is 100 plus the low half of its thrower's X
  (0x080D61DC, the throw's r0): 0 from a navi on its panel, so not ported. chips/0x02e-blkbomb/fire sets one off with a
  HeatShot.
- **Z Saver** (action 0x70, `chips/z-saver`: @exelib/zsaver/action, which EXE5's grew from, in EXE4's look): the
  blade in the related slot as EXE4's own attachment (lib/swords' `blade_anim`: ProtoSoul's 13, 0x080EE820), let go of
  at each swing's end; no form overlay refreshed, no moving flag or overlay cleared in the recovery; the command's
  window shut once A isn't held (EXE5's too); no step (EXE5's Katanas' step is EXE5's own).
- **SandRing** (action 0x47, `chips/sandring`: EXE4's own, with its ring, attack 0x6D): a ring 26 pixels up, 10
  pixels a tick, hit modifier 1, its element's spark, the spreader's sound; its hit turns the panel to sand (type 10,
  the pitfall). The action's and the ring's other parameters (0 to 2: a held ring, three speeds, status effects 0x10
  to 0x12) no chip or weapon routine passes, so they are comments; weapon routine 0x44 loads SandRing (card 82).
- **VarSwrd and NeoVari** (actions 0x36 and 0x5A, `chips/varswrd`, `chips/neovari`: @exelib/varswrd/action, which
  EXE5's grew from): EXE4's part is the pick's sound (0x10B), ProtoSoul's wait while A is up (NaviStats +0x24 7, the
  form's id) and no auto battle's pick (NaviStats +0x26, the story's navis'); EXE4 has no flip, so one set of
  sequences. The match (0x0800D486) also tests, as a step matches, the word at the sequence's next step and picks at
  once at 0, which no step of the two tables reaches (each test reads a step before or after it). Their picks past
  the library are chips of their own (gen_content.py's USED_CHIPS: FtrSword, SonicBom, CrosSwrd, SprSonic, DblDream;
  LifeSrd a Program Advance's, without its recipes): the swords' variants 9, 10, 11 and 5 (lib/swords), and SonicBom's
  and SprSonic's action 0x37 (`chips/sonicbom/action`: EXE4's own, EXE5's 0x2B grew from it; a charged one steps two
  panels ahead, without afterimages; ProtoSoul's blade 13). The chips/0x037-varswrd and 0x0d9-neovari `cmd-*`
  scenarios enter each sequence.
- **WindRack** (action 0x32, `chips/windrack`: EXE4's own, EXE5's action 0x26 grew from it): the rack (attachment
  row 0x17, by the navi's number) and the swirl (effect row 0x40, turned to its side), a null-element pushing hit on
  the column ahead for 10 ticks with hit modifier 9 (EXE5's 0x49); no gusts (EXE6's), no arm to drop.
- **CopyDmg** (action 0x2B, `chips/copydmg`: EXE4's own action, EXE5's 0x24 grew from it; its mark attack 0x28 is
  @exelib/copydmg/mark in EXE4's look): animation 0x12, no arm, the damage word without the Atk+ bonus; the mark
  marks the first navi it hits for 90 ticks, shown at the navi's position (`over_navi = "position"`; EXE5's and
  EXE6's 180 at the attach point 0x1B, shown as the navi is). The carry record (0x0203C050, two of 12 bytes by the
  marked side) and its routines (0x08022246 each tick, 0x08022258 in the intake's final damage, 0x0802222E,
  0x08022278) are EXE5's and EXE6's: the engine's `damage_carry`. Only a navi's intake (0x0800AC3A) records a
  side's damage, so only another navi of the marked side feeds the carry (none in the recordings). Under event flag
  0x1187, which no netbattle sets, the mark also hits its panel for 50 as it marks (0x080D1D06): unreachable, so no
  look field (the shared mark's comment names it).
- **SuprVulc** (the vulcans' variant 3, `chips/suprvulc`): 12 shots of bullet row 16 (the Vulcans' hit in palette 1).
- **Slasher** (action 0x35, `chips/slasher`: EXE4's own, EXE5's action 0x29 grew from it): while A is held it waits for
  an enemy navi on its side's area, then slashes that column (the region word 0x0705FF04, 16 pixels up), naming
  Slasher to the other player again as it slashes (0x080164B4: `battle.show_used_chip`, new); no invulnerability, no
  stun strike. In chips/0x038-slasher/side1 the original doesn't name Slasher as side 1 uses it (frames 384 to 415),
  where the engine's chip use does (its `show_used_chip` at a chip's start): which EXE4 code names a used chip, and
  why not this one, is the HUD's to read (group A's; item 71).
- **CustSwrd and Muramasa** (the swords' variants 7 and 8, `chips/custswrd`, `chips/muramasa`): their slashes by the
  variants' rows (region 0x11 and region 2, effect rows 0x41 and 0x4A), their damage formulas 45 (the gauge's, a full
  gauge giving 0 where EXE6's and EXE5's give 10: the formula's `full`) and 46 (the HP lost, at most 999). The lab's
  chips/0x036-custswrd/full-gauge matches.
- **GrabRvng and GrabBnsh** (the dimming chips' variant 0x0D, `chips/grabbnsh`): @exelib/grabbnsh's controller (effect
  0x22) and hand (attack 0x46) with EXE4's look (the hand from 256 pixels, a thrown break of hit modifier 0x0B) and its
  own taking back (0x080E4836: it counts the panels of the user's home columns the other side holds, each flashing,
  and returns none), the wait after the strikes 90 ticks set as they end. The lab's 10 recordings match; in
  chips/0x076-grabrvng/stolen and 0x077-grabbnsh/stolen the used chip's name at the bottom left goes at frame 385 in
  the original, while the engine shows it through the dimming's end (532): the HUD's (group A's).
- **Tornado and Static** (action 0x28, `chips/tornado`: EXE4's own action and tornado, attack 0x31, 0x080D2FDC: a
  pitfall under it doubles its re-arms and turns normal as it ends, where EXE5's and EXE6's take a panel's look):
  Tornado's one two panels ahead; Static's spread by the navi's kinds of NaviCust bug (0x08043014: the rules'
  `navicust.bug_kinds`, EXE4's nine by their NaviStats bytes, the encounter and result bugs, +0x15 and +0x19, now the
  rules' stats `encounter_bug` and `result_bug`), up to six tornadoes paralyzing 90 to 150 ticks. The lab's
  chips/0x01b-static/bugs (two kinds: four tornadoes, 120 ticks) matches.
- **PropBom1 to PropBom3** (action 0x62, `chips/propbom`: EXE4's own): a propeller bomb (attack 0x8C, 0x080DEB70: a
  field object of its user's side, class 1, with 10 HP) flying 20 pixels up from the panel ahead at 1.5 pixels a tick;
  a body, attack or navi it meets (or its HP gone) explodes it where it is; off the screen it bursts over the other
  side's back two columns (region 0x11) with its damage.
- **MagBolt1 to MagBolt3** (action 0x59, `chips/magbolt`: EXE4's own action, which EXE5's MagnetSoul's charged shot
  grew from): the magnet held (attachment rows 0x23 to 0x25), its field (attack 0x75: EXE5's MagnetSoul's, now
  @exelib/magbolt/magnet, whose EXE4 spawn only watches the magnet's slot: `watches`) on the panel ahead for 30 ticks,
  and a pull (region 8, hit modifier 4) on the six panels ahead each tick. The field's leave on a hit, the battle's end
  or an emptied slot is a byte store (the lifecycle alone), its time's end a word store (from its first phase): EXE5's
  too, which its port had as a word store throughout.
- **TwnFng1 to TwnFng3** (action 0x34, `chips/twnfng`: EXE4's own): two fangs (attack 0x51, 0x080D79B8) from the
  navi's panel, a row down and a row up (24 pixels in 6 ticks, hit modifier 1, the plain spark), then forward 10
  pixels a tick as attacks of hit modifier 3 (their second parameter 0; the setup's types again, 0x08012ED0, whose
  target write misses its record: the rules' `retype`), gone at a hit or off the screen. The action's variant would
  send more pairs (seven for each), 10 ticks apart: no chip has one.
- **WideSht1 to WideSht3** (action 0x31, `chips/widesht`: EXE4's own action, the wave @exelib/widesht/wave's): the
  shooter (attachment row 0x1C) raised, a wave (attack 0x3B, EXE6's code but for what EXE4's has none of: trails, bugs,
  palettes, its spawner's hit modifier) of the chip's speed from the panel ahead; weapon routines 0x38, 0x43, 0x4C
  and patch cards 52, 80, 113.
- **AirHoc1 to AirHoc3** (action 0x27, `chips/airhoc`): @exelib/airhocky's flick (EXE5's action 0x21 but for the navi's
  animation, 0x10: the spec's `anim`) and puck (attack 0x2E, EXE5's code) with EXE4's sounds and burst, each chip's
  row of 0x080D2768 (6, 10, 14 steps). Row 9's trail has no chip (a game's puck look states a trail tint only if it
  has one).
- **GunSol1 to GunSol3, GunSolEX** (action 0x58, `chips/gunsol`: EXE4's own, EXE5's GunDelSol grew from it): the gun
  out, 6 ticks later (A held or not) the sun beam (effect 0x48, @exelib/gundels/beam, whose EXE4 spawn watches the
  gun's slot without filling it and takes only its owner's side: `watches`), then a drain hit a tick on the column two
  ahead (GunSolEX's and the one past it) while A is held (a navi in auto battle needn't) for 60, 90 or 120 ticks, 2
  damage (4 in the sun: NaviStats +0x29, the rules' `sun`), the light filling those columns' holes (the Hole chip's,
  type 11, turned normal: no recording yet, the Hole chip being group F's). The lab's chips/0x020-gunsol1/held and
  held-sun, chips/0x0db-gunsolex/held-sun (the sun by a patch of 0x0802A7A0) match and draw as mGBA's.
- **FlmLine1 to FlmLine3** (action 0x22, `chips/flmline`: EXE4's own): the navi holds the burner (attachment row
  0x0C) and raises flames (attack 0x19, 0x080CFF80) on the column two panels ahead, each burning the chip's 40 ticks
  (its parameters' second byte) between 3 rising and 3 dying down. The action's variant 1 (a cross of five,
  0x080D00F8) has no chip.
- **HeatBrth, Blizzard, ElecShok and WoodPwdr** (action 0x33, `chips/breath`: EXE4's own): the navi holds the nozzle
  (attachment rows 0x18 to 0x1B, the original's 0x17 and the attack's element) and breathes the breath (attack 0x36,
  0x080D3A7C: unseen, 55 ticks) on the panel ahead, then 15 ticks later on the column beyond, a hit of the chip's
  element on each solid panel it reaches for 40 ticks, and leaves them lava, ice, cracked or grass. The original
  picks the breath's look, hit, panel type, step and sound by the chip's parameter (0, 1, 3, 2) from its tables: in the
  content each chip states its own (`breath.make`), each its own kind mapped to attack 0x36. WoodPwdr's hits confuse
  (hit modifier 1, status 0x20); ElecShok's highlight the panels they hit. A parameter of 4 and on would leave a trail
  (0x080D3C10): no chip has one.
- **CircGun1 to CircGun3** (the dimming chips' variant 0x33, `chips/circgun`): @exelib/circgun (EXE6's code) with
  EXE4's look: the sight SearchMan's scope's sheet (10-22), its sounds (0x128, 0x12F, 0xBB), the flash effect row 0x27,
  four thrown shots each (the chip's parameter is its period: 6, 4, 2); the gun going back along a row turns on a
  column with any of the user's side's panels (0x080DE7AA, `turns_on_own_column`); the controller (effect #0x5E)
  sets the gun whatever the far column holds and takes the user's side alone (`always_sets_gun`, `side_only`). The
  lab's chips/ (15) replay every frame; CircGun1's hit and CircGun3's back-column are pixel-exact.
- **Wind, Fan** (the dimming chips' variant 0x0F, `chips/wind`): @exelib/wind's controller (effect #0x25, 0x080E4AD4,
  its spawn taking the user's side alone: `side_only`) and fan (attack #0x48, 0x080D6488) with EXE4's look: sheet
  0c-21 (`fan`), rising with 0x121; 100 HP (0x080D6480's rows, EXE5's 40); no NameID (no identity); its leaving an
  explosion whatever removes it (`removal_explodes`); its gust EXE4's (objects/gust, untracked: the fan gives its ROM
  row for the gust's slot) on EXE4's own column (0x080D6688, the look's `column`): the enemy's front column of the
  row, none when an obstacle stands there; Fan's from there to the far edge or the panel before an obstacle. The
  lab's chips/ (8), drag/ and ice/ wind and fan recordings and every stage replay every frame; the hit recordings are
  pixel-exact.
- **Geddon1 to Geddon3** (the dimming chips' variant 0x0A, `chips/geddon`): EXE5's Geddon on @exelib/geddon: the
  controller (effect #0x1D, 0x080E400C) holding no panel, the quake (effect #0x1E, 0x080E40E4: EXE6's code) with
  EXE4's look (a puff, effect row 2, with 0x95; a rising bubble with 0x124), each chip its change (crack, break,
  poison). The lab's chips/ (12), panels/geddon1 and geddon2 and status/poison-geddon3 replay every frame; the hit
  recordings are pixel-exact.
- **GutPnch1 to GutPnch3** (the dimming chips' variant 0x36, `chips/gutpnch`: EXE4's own): a controller (effect
  #0x64, 0x080E90CC) run as the navi chips' (lib/navi_chips, its course 0x080E9124): the user warps out (30 ticks),
  GutsMan comes (actor #0x42, 0x080C83F8, sheet 08-02 in palette 0), 30 ticks, and the user warps back in with the
  appearing sound (0x080E91BC: a course's wait may play a sound as it starts, `sound`). GutsMan appears (5 ticks),
  on a solid panel stands a tick and punches (0xA4, `fist-swing`): as the punch's timer reads 12, a hit on the panel
  ahead (modifier 0x21, height the raw 16: 0x080CD7E2 drops r3) that runs while dimmed; 20 ticks more, he leaves (5
  ticks) and goes by a byte store (his action stays). On a hole he leaves at once. The lab's chips/ (12),
  drag/gutpunch, drag/gutpunch-edge, hits/guard-gutpnch and ice/gutpunch replay every frame; GutPnch1's hit is
  pixel-exact.
- **NumbrBl1 to NumbrBl3** (the dimming chips' variant 0x3A, `chips/numbrbl`): EXE5's NumbrBl. The controller
  (effect #0x69, 0x080E9838) is @exelib/numbrbl/controller with EXE5's effect (0x080E9754, `controller.warping`, now
  shared) and EXE4's warp (lib/navi_chips): the user warps out (30 ticks), NumberMan comes, 30 ticks, the user warps
  back in (30). NumberMan (actor #0x45, 0x080C8CC4) is @exelib/numbrbl/numberman (EXE5's code, now shared) with
  EXE4's look: sheet 08-08 in palette 0, appearing with 0xB0, throwing in his animation 7, and his init loading his
  standing animation (`loads_standing`; EXE5's leaves the byte 0xFF). His balls (attack #0x91, 0x080DF4E4) are
  @exelib/numbrbl/ball with EXE5's numbers (30 ticks sitting with 0xAA, parts 2 to 21 hidden) and EXE4's sheet 10-05,
  explosion (effect row 0) and 0xC8. Each chip states its balls (its first parameter and three: 3, 4, 5). Damage is
  formula 49 (power 1049, 0x0801963E): the last two digits of the user's HP (`hp_last_digits`). The lab's chips/ (15)
  replay every frame; NumbrBl3's hit is pixel-exact.
- **RollAro1 to RollAro3** (the dimming chips' variant 0x35, `chips/rollaro`: EXE4's own): a controller (effect
  #0x63, 0x080E8FA0) run as the navi chips' on the long course (lib/navi_chips), bringing Roll (actor #0x41,
  0x080C8120, sheet 08-01 in palette 0): she fades in with a sparkle (effect row 0x2E) and 0xB0, flickering; 15 ticks
  on she draws her bow (animation 7), 10 ticks on shoots the flying shot's row 15 (RollSoul's arrow at 7 pixels a
  tick, running while dimmed, 0x132) from 8 pixels ahead, a pixel up, 36 high; 6 and 30 ticks on she leaves
  (animation 4) and rises 15 pixels a tick, sparkling every 5, until 160 pixels up, her end letting the controller
  go on. Her spawner keeps neither her element nor a related object, her height her side. The lab's chips/ (12)
  replay every frame; RollAro1's hit and side1 are pixel-exact.
- **MetlGer1 to MetlGer3** (the dimming chips' variant 0x22, `chips/metlger`: EXE4's own): a controller (effect
  #0x49, 0x080E7244, the usual dimming phases, its spawn taking the user's side alone) sets the stages' gear
  (objects/gear, attack #0x76) in its mode 1 (`gear.spawn`, 0x080DC006: rolling on the other side for 1800 ticks, one
  of its side's field objects) on the other side's solid, free panel of the user's row nearest that side's far edge
  (0x0800A1AE), with the chip's damage and no telop bonus, and 0xA0; then 60 ticks. The lab's chips/ (12) and
  stages/type5 replay every frame; MetlGer1's hit is pixel-exact.
- **BigHamr1 to BigHamr3, GodHammr** (the dimming chips' variant 0x10, `chips/bighamr`: EXE4's own): a controller
  (effect #0x26, 0x080E4B88) sets a hammer (attack #0x4A, 0x080D68CC: one of its side's field objects, an obstacle of
  100 HP, sheet 04-04 `big-hammer`) on the free solid panel in front, of the chip's damage and the telop's bonus. The
  hammer waits 10 ticks, winds up 24 (BigHamr's blinking), swings (0x122) and lands 5 ticks later on the panel
  ahead, then goes 60 ticks on, in an explosion whatever ends it. Each chip states its look (`hammer.make`: palette,
  standing animation, blinking) and its landing: BigHamr's cracks two of the other side's solid panels at random on a
  solid panel and hits the panel ahead (breaking); GodHammr's quakes every solid panel (collision type 0x26) on a
  solid panel, else hits the panel ahead; a solid panel shakes the camera (by 1, 2, 3; GodHammr's 3) with 0xA2. The
  lab's chips/ (19) and hits/guard-bighamr replay every frame; the hit recordings (and BigHamr1's two-ahead,
  GodHammr's miss) are pixel-exact.
- **Silence, Fanfare, Discord, Timpani** (the dimming chips' variant 0x24, `chips/silence` and on): the instruments'
  controller (effect #0x4A, 0x080E7310) and instrument (attack #0x78, 0x080DC33C), @exelib/instruments (EXE6's code,
  as EXE5's) with EXE4's look (lib/instruments): the sheet 04-08, its sounds (0xB0 appearing, 0x70 leaving), no NameID
  (no identity), its leaving (0x080DC58C) an explosion whatever removes it (`removal_explodes`), and both spawns
  taking the spawner's side alone (`side_only`). Each chip states its row (0x080DC32C: palette 0, 2, 4, 6; Fanfare's
  50 HP, the others' 100, 0x080DC3EC), its tune (0x12A to 0x12D) and its effect: Fanfare's makes its side's navi alone
  invulnerable for 4 ticks (0x080DC5C4; EXE5's and EXE6's every combatant), Discord, Timpani and Silence EXE6's
  (confusion, immobilization, blindness, each 4 ticks). An immobilized navi doesn't blink black in EXE4 (its tail has
  no `sub_801690A`): `effects.immobilized_blinks` (EXE6, EXE5 true; EXE4 false). The lab's chips/ (16),
  status/confusion, fanfare, paralysis-timpani and silence replay every frame; the hit recordings' fight frames are
  pixel-exact.
- **Barrier, Barr100, Barr200, LifeAura, BlakBarr** (the dimming chips' variant 0x15, `chips/barrier`): the barrier
  chips' controller (effect #0x2F, 0x080E5740: @exelib/barriers/controller, EXE6's code; EXE4's lib/barriers) raising
  the chip's barrier type (its first parameter by 0x080E57D0: 1, 2, 3, 6 and 4) with its visual. Type 4 (BlakBarr's,
  150 HP; its visual's sheet 0c-44, `black-barrier`) is EXE4's own in the tick (0x08012E44): worn down, it stays with
  no HP, lets hits through, never times out and is back with 150 HP 180 ticks later (not while dimmed), wind leaving
  it be meanwhile: the barrier behavior `regrowing`, its numbers the rules' (`status.intake.barrier.regrowing`;
  EXE6 and EXE5 state none). The lab's chips/ (20, BlakBarr's over 3000 frames), status/barrier, barrier100,
  barrier200, lifeaura and navicust/firstbarrier replay every frame; the hit recordings' fight frames are
  pixel-exact (BlakBarr's through its regrowths, but for the custom screens' known shift).
- **Hole, PnlRetrn, HolyPanl, Snctuary, DrkLine** (the dimming chips' variant 0x0B, `chips/hole` and on): the panel
  chips' controller (effect #0x20, 0x080E4584: @exelib/panel_chips/controller, EXE6's code, its spawn taking the
  user's side alone: `side_only`, EXE4's lib/panel_chips) with EXE4's panel changer (objects/panel_changer), each
  chip stating its change, a row of 0x080E423C by its first parameter: Hole row 9 (the panel in front to a hole, 64
  ticks, unflickering, the start sound alone), DrkLine row 10 (the user's row), PnlRetrn row 0 (the own area to
  normal), HolyPanl row 6 (the panel in front to holy), Snctuary row 7 (the own area to holy). The lab's chips/ (20),
  panels/holypanl and pnlretrn, status/holy-panel and sanctuary and the GunSols' (whose light fills Hole's holes)
  replay every frame; the hit recordings' fight frames are pixel-exact.
- **NrthWind** (the dimming chips' variant 0x1C, `chips/nrthwind`): EXE6's and EXE5's controller (effect #0x39,
  0x080E622C: @exelib/nrthwind/controller, now shared, its spawn the user's side alone) blowing EXE4's north wind
  (objects/north_wind, WindSoul's change's). The lab's chips/0x08a-nrthwind (4) replay every frame and are
  pixel-exact; EXE5's NrthWind recordings still match.
- **Invis** (the dimming chips' action 0x0C, its variant 0x31, `chips/invis`): EXE6's Invisibl (@exelib/invisibl:
  effect object #0x5D, 0x080E8A68), 360 ticks, its sound EXE4's (0x109, in 0x0800C6BC). **PopUp** (variant 0x1E,
  `chips/popup`: EXE4's own controller, effect #0x3D, 0x080E6470) puts EXE4's glow over its user's navi (actor #0x5D,
  0x080CCF5C: content/exe4/objects/glow, a maker each user makes its own kind with; PopUp's action is the original's
  variant 1, EXE6's `sub_80C49E4`, which nothing there spawns): the navi vanishes, the white copy stays 31 ticks, then
  the navi goes under the ground for 480 ticks (0x0800C126: hidden, flag 4, which a hit, an action or the time
  ends; while it acts it is up). The navi's tail keeps a hole over a navi that is under (0x0800C0A6, EXE6's
  `sub_801012C`, EXE5's ripple's 0x0800DEB2: the role `kinds.dive_ripple`, EXE4's effect #0x3E, 0x080E64FC,
  objects/underground_hole), which the engine now runs in every game (flag 4; EXE5's 0x80000004 by
  `hit_test.bubbled_as_submerged`). The glow puts on what the navi's form wears (0x0800B812: MegaMan's soul's overlay,
  stepping on) through the form's `put_on`, so FireSoul's and NumberSoul's (item 74) come with their overlays. The
  lab's chips/0x085-invis and chips/0x086-popup (hit, miss, adjacent, side1) replay every frame; the hit recordings'
  fight frames, the glow and the hole among them, are pixel-exact.

- **The navi chips** (the dimming chips' navi variants, group F): each variant's spawner (0x080220E0's) makes its navi's
  controller, an effect object of its own (Roll's #0x2D, TopMan's #0x0B, ...; SerchMan's, ThunMan's, ProtoMan's,
  DeltaRay's, MetalMan's and JunkMan's one, #0x42, picking the navi by the chip's third parameter: in the content each
  chip's own controller, each mapped to its navi's object in compat/kinds.toml): @exelib/navi_chips/controller (EXE5's
  Phoenix's and DethPhnx's controllers are the same code), its course EXE4's (content/exe4/lib/navi_chips: the user
  warping out, the navi, a wait, the user warping back in, in four shapes), the user's warp EXE4's own effect #0x0C
  (EXE6's dead `sub_80E11FC`, not the engine's actor #0x2D). The chip's first parameter is the navi's level (0, an SP's
  3, a DS's 4), which picks its palette (0x0800B950: the navi's palettes a variant times 0, 2 or 3) and the like (Roll's
  mood, SearchMan's sweep): in the content each chip states those. The SP chips' damage goes by the side's deletion time
  (rules/sp_chips, formulas 1 to 22), the DS chips' by the field's holes (formulas 23 to 44, 0x08019518: the count
  formula's `panels`). Roll (actor #0x2C) is @exelib/roll's, with EXE4's differences as its spec's; JunkMan (actor
  #0x48) raises EXE5's Poltrgst's poltergeist (effect #0x73, @exelib/poltergeist), whose EXE4 code leaves out obstacles
  by kind (attack objects #0x4C and #0x8C, effect object #0x6E, 0x0800B3E8): their identities state `throwable = false`
  when they are ported. Its throws wait on RockCube (the lab's junkman*/obstacle). NumberMan (actor #0x0F), his face
  (actor #0x5C) and his die (attack #5) are EXE5's (@exelib/numbrman), EXE4's differences their specs': the face, with
  the fight on, follows him from its first update (its action 8; its action 4, a random battle's intro, is never
  reached); the die states its own 40 HP and hit modifier 3, places itself on its panel, has no markers and no face 9,
  and multiplies its damage's whole halfword by its face. The die's other throwers, the NumberMan boss's (0x080EC1E4,
  0x080F3336), aren't ported.
  SparkMan (actor #0x10) and his sparks (attack #0x22, which zigzag up and down each column) are EXE4's own; his
  flash is the engine's palette flash (effect #0x0A, compat's `engine/palette-flash`) and a blinding hit of no damage
  on the other side's navi (region 0x85 or 0x84, blindness 0x30). FireMan (actor #0x12) wears the flame on his head as
  the engine's form overlay (actor #0x57, `effects.form_overlay`) and breathes the Elem chips' flame (attack #9,
  @exelib/elemrage/flame, his own kind); his command (Down, then Right, each held alone through the dimmed record,
  0x080B94A8) makes the flames leave lava, which no recording has used yet.
  ShadeMan (actor #0x19) and his crush (attack #0x34) are EXE5's (@exelib/shademan), EXE4's differences their specs':
  his commands give the crush 0x1207 and 0x2208, and without one it carries a move bug (code 0x0D) in a direction he
  draws as he appears (rules/navicust/bugs takes it); he goes without the bow, in a puff; his figure stands 8 pixels
  left and 12 up and strikes again with 40 ticks left, highlighting nothing.
  BurnMan (actor #0x23), his pillars (attack #0x3A) and his flame (attack #0x37) are EXE4's own; their hits linger on
  their panels as attack #0x7C (0x080DCC5C, objects/lingering_hit: a region set off its owner's panel, following it,
  its ticks held while its owner is paralyzed, dragged or sliding; EXE6's #0x7C is another object), which AquaMan's
  controller (0x080BB4E8) and 0x080D4010 spawn too. The engine's objects tell whether they have a collision record
  (`has_collision`), which that hold reads.
  GutsMan (actor #0x14) and his quake (attack #0x1C, 0x080D055C) are EXE4's own: his hammer strikes the panel ahead
  (a heavy hit, modifier 1) and sets off the quake there, which cracks the solid panels ahead (six at random; the SP's
  and the DS's all) and 30 ticks later drops rocks on two of the other side's free panels and on its navi's. The rocks
  are EXE6's falling rock and its chips (EXE4's attack #0x1D and effect #9, the same code but for their constants),
  now @exelib/rock/falling and falling_chip (EXE4's objects/falling_rock: 160 pixels up, 1/16 pixel a tick faster, an
  attack, modifier 3; EXE6's 104, 1/8, a thrown breaking attack, modifier 1), which actor #0x29 (0x080BFCC8) drops
  too. The controller gives GutsMan B's charge at the ask as his Param2, which he never reads. Action 0x44 (a navi's
  hammer, 0x080F050A, which no netbattle starts) spawns the quake with a Param1 of its own; given 0 it cracks Param2
  of the other side's panels and drops its rocks around that side's navi or 0x0800C456's target, by its owner's
  flags 0xA000 (0x080D05FE, 0x080D06BE, 0x080D0740): it waits on action 0x44. With fewer than six solid panels ahead,
  GutsMan's quake reads the rest of its six off the stack past its list (0x080D06AC): what its crash's sound left
  there, the sound queue's (0x0800073C) saved r7, the object loop's node of the quake itself (0x0203C070 + 0xD8 by its
  slot: its low byte, then 0xC0 to 0xDA, 0x03, 0x02), and its return address's low bytes (0xAB, 0x05). As panels
  (x the low three bits, y the high nibble) the five it can reach are all off the field (y 12 or 13, 0, 0, 10, 0; the
  low byte only with no panel listed, when it cracks none), and a crack off the field does nothing: the port cracks
  only those it found, which is the same. (Only an interrupt in the few instructions between the sound's return and
  the list, whose handler runs on this stack in system mode as EXE6's does, could leave other bytes there; the
  engine has no interrupts.) The lab's chips/0x0e0-gutsman/few-panels (the quake finding five) replays every
  frame and its fight frames are pixel-exact.
  AquaMan (actor #0x1A) and his water gun (attack #0x33, 0x080D3468) are EXE4's own: on a solid panel nothing stands
  on (else gone at once in a puff) the gun pops up and sprays the panel ahead of it once and the one past it twice
  (aqua hits, modifier 3), then sinks; AquaMan waits for it to go. While it readies, it counts its side's presses of A
  while dimmed into a strength nothing reads (0x080D362C loads it and sprays twice). The spout on AquaMan's head is
  NumberMan's face's object (actor #0x5C) with its sprite row 1 (10-0C), his own kind of @exelib/numbrman/face. His
  controller, like GutsMan's, gives him B's charge at the ask as his Param2, which he never reads: no EXE4 object
  reads `b_charge_at_ask` (item 50's), as no other code reads the AI data's +0x15.
  WindMan (actor #0x34), his tornadoes (attack #0x6B, 0x080DAC54) and their sand (attack #0x6C, 0x080DAF54) are
  EXE4's own; his controller (effect #0x46) waits 61 ticks after him, not 31. He blows three tornadoes; each goes
  from the panel ahead a row down (on the bottom row at once on), two panels ahead, back up its column to the field's
  edge, then back toward his side, a hit (collision row 0x20, modifier 3) on each panel it enters, till it leaves
  the screen or, seven panels entered, four ticks into the next. Reaching a row's or a column's center it moves on in
  the same tick (0x080DAE2E, 0x080DAE8E). On a pitfall (type 10, SandRing's sand) it picks up the sand: palette 1,
  the panel to normal, and from then on sand on each panel it enters (a hit 11 ticks later). The swirl on his head is
  actor #0x5C's row 2 (10-28, the one his soul image holds). The lab's chips/0x0e3-windman/sand (the typeA stage's pitfalls) replays every frame and is
  pixel-exact, as the other twelve recordings are.
  VideoMan (actor #0x3E) and his tapes (attack #0x5D, 0x080D90C8) are EXE4's own; his controller (effect #0x5F) runs
  the short course. As his swing ends a tape covers the square two panels ahead of him (three back for side 1: a
  square spreads to the right) and a row up, unless he is on the top row; 45 ticks into it another covers the square
  in his row, unless he is on the bottom row. A tape (sheet 08-17, 20 pixels right, 46 down and 24 up of its panel)
  winds in and three times, 15 ticks apart, hits its four panels (no element, modifier 3, at its height). The tapes'
  Param3 0 course is actor #0x2F's (EXE4's other VideoMan, 0x080C2FC8), which no netbattle spawns. The lab's
  chips/0x116-videoman/top-row (no upper tape) and the twelve recordings replay every frame and are pixel-exact.
  KendoMan (actor #0x43, 0x080C85E8) is EXE4's own; his controller (effect #0x66) runs the short course. He dashes
  along his row to the panel before the first enemy body (or the field's edge, where he falls off solid ground and
  goes), afterimages every other tick, and slashes the panel ahead (modifier 1); vanishes, reappears two panels on
  facing back (on a solid panel with no body), glides and cuts the column behind him (region 4, no spark, effect row
  0x17 with palette 2 added); then rushes home with his sword out (attachment row 0x2D, his own sheet 08-0D) and a hit
  following him (attack #0x7C, the lingering hit, 100 ticks), afterimages of his side turned every fourth tick, until
  he leaves solid ground. The rush's hit is spawned with its holder's slot at the address 0x60 (a constant where
  0x080CC036 adds his own address): no holder keeps it, so nothing ends it early. His afterimages are the engine's
  (effect #0x28, compat's `engine/afterimage`), with their shadow hidden: 0x080E5020's r7 byte 2 (1) hides it, as the
  frames show (`shadow = "ground"`, as lib/cannon has for the GigaCans' first afterimage, draws one the original
  doesn't). Sheet 08-0D, navi 13's in 0x08017F98 with NormalNavi's win banner, is KendoMan's: `kendoman` (BY_USE now
  wins over the navi table's names). The lab's chips/0x113 to 0x115 (12) replay every frame and are pixel-exact.
  WoodMan (actor #0x49, 0x080C980C) and his wood towers (attack #0x07, 0x080CDEB8; EXE6's #7 is the volcanos'
  eruption) are EXE4's own; his controller (effect #0x70) runs the short course. He leaps (27 pixels a tick up, a
  pixel a tick more down, 54 ticks) and lands with a shake (2, 15 ticks); as his landing ends and 10 ticks on he
  raises four or five towers (a draw's low bit) on the other side's solid panels at random (a shuffle), the set's last
  with its sound: each waits 31 ticks blinking its panel's highlight, then, on a solid panel with no body or blocker,
  rises as a thrown body (the wood spark, modifier 1), 11 and 21 ticks, sinks 10. His command, Down, A and B pressed
  within five ticks of each other while dimmed in his leap's last 4 ticks or his landing (0x080C9A94), raises a second
  set. The lab's chips/0x0fe-woodman/command (the second set) and the twelve recordings replay every frame and are
  pixel-exact.
  LaserMan (actor #0x4E, 0x080CABE4) and his laser (attack #0xA0, 0x080E0E0C) are EXE4's own; his controller (effect
  #0x75) runs the short course. He charges 31 ticks and fires the laser from the panel ahead; it strikes its row of
  six (region 8, a piercing hit, no spark) every 6 ticks, ten times, as its course lists (0x080E0F4C): its damage
  (0xFD, modifier 3), a NaviCust bug and no damage (any other word: code and argument, the navi_bug hook), or nothing
  (0xFF). LaserMan's course is the hit alone; LasrMnSP's and LasrMnDS's take the direction their side's navi holds
  while dimmed as he charges (up, down, right, left: the first held): up the attack, rapid and charge bugs (5 to 7),
  down super armor, FloatShoes, AirShoes, Undershirt (1 to 4) and the back special weapon (0x0C, 0xFF), right the
  charged shot (0x0A, 1), left the custom level less one but not below 2 (0x12, 0xFE's word), each then the hit. The
  lab's chips/0x111-lasrmnsp/aim-up, aim-down, aim-right and aim-left and the twelve recordings replay every frame and
  are pixel-exact.
  Bass (actor #0x4F, 0x080CADC0) is EXE6's and EXE5's (@exelib/bass/navi) with EXE4's parts: his sheet 08-19, his
  cape's animation offset 0x0E (the form overlay, actor #0x57), EXE5's animations and ending, his shots' flash (effect
  row 0x6D) jittered by 0x1F (0x080CB0EA; EXE6's and EXE5's 0xF: the maker's `flash_jitter`), his smoke (row 0x13).
  His shot (attack #0x72, 0x080DB6FC) is EXE4's own: a pixel down on its panel, highlighting it 10 ticks, then on a
  solid panel his burst (row 0x57: his sheet's animation 28) and a thrown hit of no element (modifier 3, the plain
  spark; its hitbox runs while dimmed as the shot does) with the vulcan's sound. His controller (effect #0x76) runs the
  short course. The lab's chips/0x12d-bass (4) replay every frame and are pixel-exact but for the custom screens'
  known shift (the Giga chip is dug over turns); EXE6's Bass traces (49) and EXE5's (4) still match.

**For the next steps:**

- **Extraction** (`nettai-extract exe4`): as built, §14.
- **exe4-compat:** the numbers (chips by id, navis, souls), the recording decode (the oracle's layout, §3.4, and the
  setup line), the save import (§12). Its start (the chips' ids, the asset names, the text encodings) came with
  the extraction (§14).
- **The oracle:** its EXE4 layout and hooks are oracle-trace's `EXE4` and `RED_SUN_HOOKS` (and the other three
  ROMs'), §3.4; the four ROMs in the chip lab with Tango's twelve saves as bases.
- **Open:** the NaviStats fields §3.3 leaves unread; what the object record's +0x17 byte is;
  the custom screen's states and Double Soul's offer; the dark chip offer; the emotion function; the turn-start order
  of a transformation; the panel entry's size and types (BN4 numbers holy 9, metal 5: multi-game.md §2.4); the link
  exchange's contents; the assets §14 leaves as placeholders, and the names of the sprites and sounds it lists
  unnamed.

## 14. The assets (as built)

`nettai-extract exe4 <pack> <ROM ...>` writes an EXE4 pack (crates/nettai-extract/src/exe4), its names and text
encodings from content/exe4/compat (assets.toml, chips.toml, text.toml), which the verification workspace's
tools/exe4/gen_content.py writes from the ROMs and `check`s again; the exe4-compat crate reads them. Each address is
Red Sun US's and its counterpart in the other three ROMs: the value of the same literal of the same routine (the
ROM data maps, `bmap.py --to <CODE> romdata`).

- **Sprites** (R): the sprite list (0x0802793C; Blue Moon 0x08027940, the Japanese 0x08027890 and 0x08027894), its
  battle categories (byte offsets 0x00 to 0x14) as EXE5's: 225 sprites, the same archives in all four ROMs.
- **Sound** (R): the m4a song table (0x08155FF8): 391 songs, the same in all four ROMs (no version's own songs, as
  EXE5's Team Colonel has).
- **Chips** (R): the record's picture (+0x24, 7x6 tiles), its palette (+0x28) and icon (+0x20, 2x2 tiles), the icons'
  palette by Tango's pointer (0x08015A78, the Japanese 0x080159D4). chips.toml has the chips a folder can hold
  (Tango's legal chips: 1 to 186, 201 to 280, 301 to 310; 276), keyed by the US names as EXE5's are (`aquamn-sp`),
  the ten version giga chips with their version, whose art comes from that version's ROM. The table's other records
  are placeholders (ten "Bass" before Bass, seven "DarkNeo" before DarkNeo, a second "LifeAura"), the Program
  Advances (from 321) and the e-Reader cards' two (311, 312: their names are the save's), not in chips.toml yet.
- **The fonts and the HUD's text lines** (R), English from a US ROM, Japanese (the pack's `ja` lettering) from a
  Japanese ROM: the 8x16 font (0x0868DF5C: 0x1A0 glyphs of 0x40 bytes, other data past them), the dialogue font
  (0x08694F5C: 0x1C0 glyphs of 16x12, 0x60 bytes) and its advances (a word a glyph from 0x080515E0, past the word
  its routine's literal names, as EXE5's), the text lines (0x087491CC: "DOUBLE DELETE!", "TRIPLE DELETE!", "VS",
  "TIME UP!", the timer's 1 to 10, "COUNTER HIT!", ending in 0xE5). The Japanese ROMs' lines are the same English
  words in their own encoding.
- **The text encodings** (text.toml; Tango's BN4 character sets): a byte below 0xE4 is a glyph, E4 xx glyph 0xE4 +
  xx; the text ends in 0xE5, a line in 0xE8 (EXE5's and EXE6's 0xE6 and 0xE9). The US fonts draw 0x00 to 0x6F, the
  suits 0x6B to 0x6F in the dialogue font alone, and 0x69 and 0x6A as > and < in the 8x16 font but < and > in the
  dialogue font (the encoding names the 8x16 font's). The Japanese encoding is 442 glyphs. The marks are the
  engine's characters (text-rendering.md §10.5): EXE4 adds V over 2 to 5 (U+E009 to U+E00C) and □.
- **Names** (assets.toml): 50 sprites and 4 sounds, a sprite whose archive is a named EXE5 or EXE6 sprite's by its
  name, and a sprite or sound EXE4's code loads where EXE5's or EXE6's same code loads a named one (the place votes
  of tools/exe5/assetmap.py over both maps; a pair the same but for constants votes only where the asset's number is
  the same: such pairs in the object code are often other objects on one skeleton, and would have named an EXE4
  sound EXE6's Beast Over burst). Then what EXE4's own code says (gen_content.py's BY_USE): the assets the ported
  content uses, each by the code that loads it; the navis' sprites by navi number (0x0800B90A's table 0x08017F98:
  navis 1 to 14, Roll to HealNavi, named by their win banners, 0x08008524) and MegaMan's in each soul by soul
  number (RollSoul to WoodSoul, the navis' order: the chip names' list from MegaSoul has them so). The rest (142
  sprites, 368 sounds: the story navis' 0x0E to 0x19, the viruses', the chips' objects and effects, and the sounds
  of what isn't ported) are written under their numbers (`sprite-cc-ii`, `sound-nnn`) and listed in the pack's
  extraction.txt (`unnamed:`), to name by what loads them as the port reads EXE4's own code (the verification
  workspace's tools/exe4/assetloads.py lists each one's loads).
- **The field** (R, Red Sun US's): the field's load (0x08006A40: its tiles to VRAM 0x06001460, as EXE5's and EXE6's;
  its transfer list 0x08006A68: background palettes 1 to 8), the panel blocks (0x080093FC: 6 * type + 3 * owner + row
  - 1 from 0x08706640, for EXE4's 12 panel types), the highlight (0x0800948A: one block, for both highlights), the front
  edges by owner (0x080094C4, 0x08706F60), and the panel palettes that cycle (0x08009556, as the field is drawn: six,
  each a frame every 14 ticks from a table of palette pointers, their timers starting at 14, 13, 12, 11, 10 and 9:
  0x08009120). Each type is drawn as the engine's type compat/panels.toml gives its number (content's names for them).
  What the blocks show: 5 a riveted metal plate, 10 a sand pit, 11 a hole (§18 item 12: `metal`, `pitfall`, `hole`).
- **The HUD** (R, Red Sun US's; exe4/hud.rs): the tasks (0x08014D10's table at 0x08014D34, EXE6 `sub_801BF64`'s
  counterpart) and the load list (0x08015A0C): the HUD layer's tiles from 0x130 and the gauge's from 0x80, both in
  EXE6's order; the HP box (6x2, 0x08016B2C) and the gauge frame (18x2 with "CUSTOM", 0x08016B44); the HP box's palettes
  (0x08708260, three colors); the opponents' HP digits (0x08014EB8: one color); the hidden chip's icon; "PAUSE"
  (0x080166D0's list); "BUSY..." (0x08016710's list: 7x2, at column 22, row 4); the banners (0x0801617E: 41 records at
  0x08016C04, EXE6's fifteen in EXE6's places, then the navis' wins and deletions; twenty glyphs each, no filler); the
  emotion window (0x08014B78: MegaMan's five faces with their boxes, a version's six souls' faces beside a count box of
  0 to 5); the chatbox (0x0804E3B4: one box, which descriptions show in too). The Japanese ROMs' banners and
  "カスタム中…" are the pack's Japanese lettering. Not drawn yet: "PLAN-B..." (0x08016AE8's list, the other waiting
  words, 作戦変更中… in Japanese), shown while the other player is in what group A's screen calls the second screen.
- **The custom screen** (R, Red Sun US's; exe4/custom.rs), as EXE5's in its parts, from EXE4's own routines: the
  window (0x0801DC28: the map 0x0870CC80, its patch list 0x0801DD90 from tile 0x9C, one frame color for every chip,
  0x0870C340) and the HUD's load list (the frame's tiles, the picked column's cells, "FINAL TURN", the UNITE button's
  and the emblem's tiles); the chip window (0x0801FC40: the name, picture, code and damage; the element icon a sprite
  of its own palette, 0x0801EECC, by the record's element byte in EXE5's order); the slots (0x0801FB00: palettes 11,
  12 gray, 9 picked); OK's two pictures; the cursor's corners (0x0801EDB4's tables, its own palette 0x0870C360,
  sprite palette 13); the Regular chip's frame. What EXE5 doesn't have, the pack says (`CustomScreen`'s
  `element_sprite`, `cursor_palette`, `window_emblem`, a button's `place`): the emblem over the picked column is the
  window's own orb (0x08020028: four frames of 2x3 on the map at column 12, row 0, turned by the steps of 0x08020078
  as a chip is picked), no navi's; the UNITE button (`soul`) is drawn at its own place (0x0801FF14: 3x2 at column
  11, row 17, from tile 0x52), gray when unavailable, the window's fill without a button; SHUFFLE (`redeal`) over
  slots 8 and 9 (0x08709E00, three states). The Japanese ROMs' OK pictures, SHUFFLE's picture and the UNITE
  button's tiles are the pack's Japanese lettering. Also the layout's `detail_blank` (a blank code or damage cell is
  solid 7, 0x08020E5C: 0x0801FD08, 0x0801FE2A) and `empty_palette` (the empty icon in palette 9: the slots' patches'
  own, which 0x0801FB00 leaves on an empty slot and sets on a picked one; 0x0801FB6E sets it on the picked column's
  empty cells, 11 on a filled one).
- **Drawn as the original** (frames compared with the lab's mGBA shots, tools/frontend-compare: custom/cannon,
  describe, pause, three-picks and flow/buster-side1, as far as each plays): the field and its panels; the
  backgrounds, 0x02's and 0x09's with their animations (0x09 is 0x03's picture darkened each frame by a color that
  changes, GFX animation command 0x0C: a palette transform, 0x080024BC, subtracting per channel, mode 4, which
  0x0800258C runs each frame on the palettes shown: the pack's `darkens`), the background's clock without the first
  round's head start (`StageClock`); the emotion window's face; the banners as they unsquash and squash (0x08014994:
  from 0xC0 to 0x40 over the slide's 9 ticks, a line, as EXE6's over its 5; no bounce in the hold: the rules'
  `effects.banner.bounces`); the custom screen as it opens, while picking and on OK (the window, the chip window and
  OK's pictures, the slots' palettes, the picked column, the element sprite, the window's emblem). What still
  differs is not the drawing's: the HP box (the intro doesn't show it yet); the UNITE button (content/exe4 registers
  no `soul` button yet); the second row's slots 8 and 9 (dealt empty where the original hides them); the cursor on OK
  after it is pressed (the original hides it). Since drawn: MegaMan's colors (his palette by his light/dark value,
  rules/light_dark: the setups' 1000 draws row 4, 0x0821B854), the intro's fade (by its levels), Cannon's
  description, the HP box (below).
- **The fight HUD as ported** (group E). The HUD block is 0x020363F0. Its draw tasks run from the mask at +0x4C
  (0x08014D10, table 0x08014D34), its update tasks from the mask at +0x48 (0x080146A8, table 0x080146CC). A piece
  is shown by setting both bits: the starters' table 0x08015E9C (0x08015E78) and the stoppers' table 0x08015BC4
  (0x08015B9E). Each piece, by its draw bit:
  - 0 **the HP box** (0x08014D98; rolled by update 0 at 0x0801472C, EXE6's roll: an eighth plus 4, held 15 ticks,
    the low color at a quarter, the alarm 0x82 every 45 ticks): drawn. EXE4 loads the gauge's tiles (from 0x80)
    before the HUD layer's (from 0x130); the renderer took the HP box's tiles for the gauge's.
  - 1 **the custom gauge** (0x08014E14; filled by update 1, 0x080147BA): EXE6's code with the counter at +0x18 and
    the value at +0x24. It skips its drawing from the 15th turn of a link battle (0x08007E4E). The full gauge's
    stripes and "L or R" go by +0x18. "BUSY..." runs the same counter (update 13, 0x08014ADC: +1 & 63 a tick, from 63
    at its start, 0x080166F0) until the exchange is done (0x0801E9B6). Drawn as EXE6's.
  - 2 **the HP numbers under objects** (0x08014EB8; rolled by update 2, 0x08014800: an eighth plus 2): one color,
    at priority 1 (the pack's `layout.hp_number_priority`), under the HUD's BG3 at priority 0 (the battle's video
    init's table at 0x08006AD4: BG3CNT 0x1F00, then 0x1F08; EXE6's is 1): the custom screen covers a flag's number
    there (the pack's `layout.hud_priority`; the screen's cursor and element icon are sprites at priority 0).
  - 3 **the chip icons**, over the local navi alone. Update 3 (0x08014860) places them at its place on the screen
    plus the per-navi offset 0x0800B9E4 (which 0x08015B24 keeps at +0x1B, +0x1C): navi 0, MegaMan, (0, -55); 1 to
    12, the souls in navi order, (0, -67), (-8, -59), (-12, -84), (0, -64), (3, -72), (-4, -72), (-1, -62),
    (-5, -70), (-5, -79), (4, -60), (-6, -60), (-9, -77). It loads each chip's own icon, from the next chip on.
    Draw 3 (0x08015000) draws them two pixels up and left each, at priority 1 in depth buckets from the count down
    (`effects.chip_icons = "navi_offset"`, the identity's `chip_icons_at`). A soul chip as the next chip (0x160 to
    0x16F) shows no icon of its own: the stack starts at the second. Not ported until Double Soul's chips exist.
  - 4 **the chip name and damage** at row 18 (0x08015078): EXE6's layout (the damage after the name's glyphs, "+"
    and the two bonuses' sum, "×2" when 0x0800F49C says 2 or 3 of the navi's +0x16). Whether the damage shows is
    the chip's +9 bit 2 (update 4, 0x080148D8).
  - 5 and 8 **the banner and the user's telop** on the banner block 0x02037CE0. 15 is **the other player's telop**
    on the second block 0x02037CF0. Both are laid out by 0x0801650C from x 0 or 120, at y 32, in the palette
    0x08750E80 (the pack's `layout.telop`). A navi's chip use names the chip on the other console as a banner that
    never slides (0x0800C7A8, state 3): `effects.used_chip_ticks` 33.
  - 6 **the messages** (0x08015FE8; 60 ticks, update 6): "COUNTER HIT!" (text line 14) from column 8 of row 2, 14
    glyphs wide (the pack's `layout.message`). A message also stops "????" (draw 10) until a custom screen starts it
    again (0x0801E1D6); EXE6's comes back when the message goes. Not ported: no defensive chip is ported yet.
  - 10 **"????"** for a defensive chip, at columns 6 and 26 (0x08015266): EXE6's.
  - 12 **"PAUSE"** at (100, 64), a row lower than EXE6's (the pack's `layout.pause`).
  - 13 **"BUSY..."** at column 22, row 4 (0x08015624): on a console whose screen has sent its result, also while the
    other player picks on a second screen it opened with L (both consoles are in the custom screen's sub-state then).
    23 **"PLAN-B..."** in the same place (0x080158D8) is not a netbattle's: 0x08016AE8 starts it from 0x08021138, a
    state of the system at 0x0203BC90 (0x08020F64, from the flow at 0x0800784C), which the levels gauge (16) and the
    column marker (22) belong to: controller 2's (§18 item 49). The lab's custom/plan-b (side 1 opens a second
    screen, side 0 traced) never sets draw bit 23.
  - 20 and 21 **the emotion window** (0x0801585C, 0x0801588C): drawn (§14 above).
  - 9 **the damage judge's numbers** (0x080152C4, its state at 0x02037BD0, started with banner 0x28 by 0x080163C8):
    "VS" at column 14 of row 5, the numbers ending at column 12 and from column 17, EXE6's places. They show from
    the holding banner's hold (0x08014AA8) until the judge lets the banner go (0x080163B6: the banner slides out and
    the numbers' state 8, 0x0801544C, clears them; the pack's `layout.judge_from_hold`). The HUD knows the judge's
    banner by its role. The lab's timer/win, loss and draw (both consoles) compare every frame of it.
  - Not in a netbattle: 7 and 19, the turn timer's seconds over "CUSTOM" (0x08016362 from the fight's timer
    0x08008066, and its blinking from 0x080169DA, for battle kind 0x44); 11 and 18 the enemies' names; 14 the icons over viruses (the list 0x0203BE40, 0x080167C0, from the virus
    code); 16, 17 and 22, the gauge in three colors, its sprite and the column marker of controller 2 (0x08016842
    from 0x08007846, 0x08016878 from 0x08022A08, 0x0801589A; §18 item 49).

  Compared with the lab's mGBA shots (verify `tools/frontend-compare`, recordings under the exe4-hud verify
  worktree's data/verify/exe4-presentation). Every fight frame is pixel-exact in custom/cannon, describe, pause,
  three-picks, flow/ko-side0, ko-double, ko-bluemoon, emotions/counter and full-synchro-hit. Together they cover
  the HP box (its colors through ko-side0's deletion), the custom gauge filling, the HP numbers, the chip icons
  (one and three chips), the chip name, damage and "×2", "PAUSE", "COUNTER HIT!" and the other player's used chip
  (emotions/counter, frames 410 to 441). So is every fight frame of flow/ko-cannon (a deletion's explosion and the
  Cannon's hit spark share a depth bucket: EXE4 draws each pool by slot, 0x08003BA0, 0x08003ED4, 0x08004180, where
  EXE6 walks the update lists: `effects.draw_order`), flow/buster, buster-tap, flinch-moving, flinch-shooting, move,
  move-edge, move-held, custom/back, back-all, hand-of-seven, same-chip, run-message, run-message-b, run-message-ok,
  and, with group B's charge glow fix (69fb65ad6), flow/charge, charge-held, charge-move, buster-side1,
  buster-attack-max, buster-charge-max, buster-speed-max, buster-hold, flinch-charging. Every stage the content plays
  (33 of the lab's 46 stages/*/stand: cracked, grass, holes, holy, ice, lava, normal, poison, type 5, type A, in
  their layouts) is pixel-exact but for the full gauge's 4-frame shift below: the field, its panels by type and their
  palette cycles. The other 13 wait on their obstacles (§18 item 19).
  Two pieces show 4 frames late: "BUSY..." (custom/one-side-waits, second-screen; its 32 frames) and the full
  gauge's stripes and "L or R". That is a shift of the comparison with the recording's console, not of nettai: the
  recordings' consoles were linked by an emulated cable with a 4-frame delay, which exe4-compat feeds as the
  original's fight and screen saw it, and the engine's custom screen (part of the shared simulation, with no link
  delay of its own) takes the OK when the fight's view of it arrives (Round::screen_late), so the counter they run
  on starts 4 frames later than the original's local screen's. Live nettai has no built-in delay (netplay's
  `present_delay` is 0, with rollback): its screen reacts on the tick its input arrives. Compared 4 frames apart
  (verify tools/frontend-compare/offset-compare.py), the
  full gauge's cells are exact on all 10,509 full-gauge frames of flow/no-time-limit. The frame comparison now
  knows the shift itself (§17's known deviations: `shifted.tsv`, "exact at the replay's known shift").
  The custom screen while it opens and while picking (custom/cannon, describe, three-picks, second-screen) differs
  only in the UNITE button (Double Soul's). Its description, while up, matches (EXE4's text from (0x3F, 0x6D), its
  arrow where the message box's is: the pack's `layout.chatbox_text`, `chatbox_arrows`). Its end matches too: the
  text is gone a frame before the box's first closing step (`chatbox_end_clears_tiles`: the end, 0x0805393C,
  zero-fills the text's tiles in video memory itself, where EXE6's clears only its buffers) and the cursor is back
  on the tick the screen sees the box closed (`states_in_choosing`).
- **content/exe4** is a game pack with these compat tables and no rules yet: the app lists EXE4, which doesn't
  load until its rules come (the sections every game's rules have: link_pick, flow, panels, reactions, pools, effects,
  status, chip_use, fresh_stats).

## 15. EXE4's rules (as read)

What EXE4's rule sections say, each from the routine that decides it (Red Sun US's address; EXE5's or EXE6's
counterpart where the map pairs one). Where EXE4 decodes neither game's way, the engine's rule became data (rather
than a third named choice): the damage word and the push reading so far, each with EXE6's and EXE5's sections
restating theirs the same.

- **pools**: 8 actors, 32 attacks, 32 effects (§3.1).
- **fresh_stats**: the Regular memory 4 (a new game's: the save's 0x2148 is the Regular memory less 4, which
  0x0802D570 works out from the key items, RegUp1 + 2 RegUp2 + 3 RegUp3; 50 at most); the custom level 5 and the mood
  0x99 (a new block's, 0x0800D6BE).
- **flow**:
  - `result_words` 37: a custom screen's result is the hand's 0x50 bytes and NaviStats' 0x40, 0x25 words
    (0x080088EA; EXE5's 49, EXE6's 50);
  - `sequencer_before_custom` false and `escape_check` false: EXE6's `sub_8008452`/`sub_8008492` and
    `sub_800AAD6` are absent (EXE4 has no turn-start transformation sequencer and no Cross);
  - `result_wait` 102, 65 when the battle's flags (0x08007EEC's table by battle mode) have 2 (0x08007252, as
    EXE5's);
  - `intro_from_black` false: the intro's fade (0x080E1EBC) reads the BattleState (EXE5's reads a register the open
    bus fills);
  - `low_hp_music` false: EXE6's switch, `sub_8009158`, is absent, and EXE4 queues no tempo request (§3.4);
  - `navi_win_banner`: the win (0x08007252) shows the winner navi's banner (0x08008534) when event flag 0x1187 is
    set (0x080406A4), which the main subsystem 0x080406B0 sets as it starts and a game's load clears; else banner 8,
    0x18 on the judge's ruling; mode 0x44 its own song and banner. Read as every link battle (EXE6's `link_battle`):
    to confirm (§16).
- **chip_use**: a dimming chip's action (0x0C, 0x080EB9EA) is EXE5's dimming handler and leaves the action on the
  frame it runs (`leave_on_use`); AntiNavi's sparkle (0x0800815C) is EXE5's, the panel's center 16 up; no mixed
  modifiers (EXE4 has no capsules).
- **effects**: the shake (0x08025FE0) draws from RNG2 and holds while paused without dimming (EXE5's `battle_rng`); a
  hit spark (EXE5's 0x080E0870, the same code), afterimages (the same code) as EXE5's; the retype
  (0x08012ED0) EXE5's, storing what it hits to the row number plus 0x5C (`is_alone`); the palette flash (0x080E2A34)
  EXE5's hold, in palette slot 9 (`before_fades`, the fades' slots to confirm); the form overlay (actor 0x57,
  0x080CC3D8) its own start and follow (`effects.form_overlay`: following from its first update with the fight on, as
  action 8; its palette and facing taken once; no animation restarted each tick; its wait stepping its sprite); the
  sprite frame load (IWRAM 0x0300632C) EXE5's code; the obstacles' actions from 6 (the player's table, 0x080EAEFC: six
  framework states, entry, take control, deletion, flinch, paralysis, drag, then the kind's own); the Full Synchro aura
  (0x080CD180) EXE5's. **The damage word** (0x08012860) is EXE4's own: the damage the low 14 bits, 0x8000 doubling, and
  0x4000 status 0x12 without a flinch, nothing else.
- **reactions**: the slide and drag speeds 0xA0000 and 0x80000 (0x0801077C, 0x080111E8: EXE5's); **the push**
  (0x0800ACAA, an obstacle's 0x0800B1C0) reads the final modifier's bits 2 to 7: back, forward, a panel back, a
  panel forward, up, down (0x0800ACDC, an obstacle's copy 0x0800B218), times the front (0x0800AB88) or the pusher's
  side (its hit flags 0xA2000000 and 0x51000000, as EXE5's). EXE4's ice slide table isn't EXE5's nor where its
  routine's was: to read.
- **The collision record** EXE4's shared routines use (the retype, the damage word, the push): the hit modifier at
  +0x0C, the final modifier at +0x0D, the status at +0x0E, the damage at +0x1E, the hit flags at +0x54 (and +0x80, kept
  while dimmed), what it is at +0x58, what it hits at +0x5C (EXE5's +0x0E, +0x0F, +0x10, +0x2E, +0x68, +0x30, +0x34).
- **panels** (to read): EXE4 has 12 panel types, their flag words at 0x0800A3A8: 0 0x18000, 1 0x14000, 2 0x10010,
  3 0x10050, 4 0x10110, 5 0x30010, 6 0x10410, 7 0x10810, 8 0x11010, 9 0x12010, 10 0x10210, 11 0x10010 (its type 5,
  Tango's metal, has EXE5's sea's flags; its type 10 EXE5's metal's).

## 16. To confirm by recording

Assumptions waiting on a recording (the chip lab's EXE4 recordings settle each):

- Settled: the win banner. The lab's `flow/buster-duel` (§17) shows on Tango's netbattle that the winner's console
  shows banner 4, "ENEMY DELETED" (the banner block's +1 from frame 1057), the loser's banner 8, "MEGAMAN DELETED"
  (`flow/buster-duel-bluemoon`, frame 1056): event flag 0x1187 isn't a netbattle's. The flow states `never` (a new
  choice of `navi_win_banner`, EXE6's and EXE5's unchanged).
- The palette flash before the fades: EXE4's fade slots (§15 effects). SparkMan's flash under his dimming (the lab's
  chips/0x10d-sparkman, frames 506 to 524) shows the field's panels white darkened a quarter, as the frontend draws
  them, but the background a flat (18, 9, 18) where the frontend whitens it too; and a blinded console hides the
  other side's navi's HP number (chips/0x10d-sparkman/side1, frames 653 to 754), which the frontend still draws.
- `status.missing_collision_status`: what EXE4's console reads through a navi's missing collision data on a round's
  first tick, if its code reads it there at all (EXE5's open-bus value until a recording shows).
- The obstacles' own actions from 6: an obstacle's action table, as the player's (§15 effects).
- Roll's chips against an armed AntiRecv (the spawner's own spring, 0x080E5554: the counterattack three times half
  the chip's damage word, its dimming with no cut-in): no recording has Roll used into AntiRecv yet.
- Settled: panels through a pause and a dimming (§18 item 13, a95f's `panels/` recordings). Poison doesn't drain
  through a pause (`poison-pause`: the player doesn't run paused); a player landing on lava burns the tick after
  it lands even while the battle is dimmed (`lava-dimming-33`, RockCube's dimming: the burn at 418, its flinch at
  565 as the dimming ends; `-32` and `-34` the dimming before the landing and after the burn), not while it blinks
  (`lava-blinking`: the burn on the tick f1's 0x200 clears). The lava ones wait for their stage (layout 0x71,
  §18 item 19) and RockCube.

## 17. The recordings

The chip lab's EXE4 recordings (the verification workspace's `tools/chiplab/library-exe4`, recorded into its main
checkout's `data/traces/lab-exe4`, the index in `index.jsonl`), each a trace (`NAME.jsonl`) and its sound calls
(`NAME.snd`):

| Scenario | Base | Frames | What |
|---|---|---|---|
| `flow/buster-duel` | `exe4`: US Red Sun on side 0, traced, against US Blue Moon | 1,189 | the first custom screen confirmed with nothing picked; both shoot the buster from their start panels (1 damage a shot), side 0 twenty times, side 1 twelve; side 1 (20 HP: the save's base HP) deleted at frame 1000; the KO's banners and the round's end (BattleState's sub-state 0x14 from 1158, its state 8 from 1175) |
| `flow/buster-duel-bluemoon` | `exe4-bluemoon`: the same, traced on side 1's Blue Moon console | 1,188 | the same fight on the loser's console (its own low-HP alarm, 0x82, three times more) |
| `custom/cannon` | `exe4` | 565 | side 0 picks a Cannon A (a folder of Cannon A), uses it (side 1 1000 to 960), then a buster shot |

**The format** is oracle-trace's (the verification workspace's `crates/oracle-trace`): JSON lines, a `setup` line as
the round's fight starts (BattleState's state byte 4), an `exchange` line when the NaviStats blocks change, and a
frame line after each battle frame. EXE4's differ from EXE5's in what EXE4 has (oracle-trace's `EXE4` layout):

- **setup:** `"game":"exe4"`; `settings_ptr` and `settings` (BattleState+0x3C's 0x10 bytes); `navi_stats` (both blocks,
  0x40 bytes each, §3.3, as the PET compiled them: a side's HP its save's base HP, its patch cards applied); `folder`
  (the traced console's battle folder, 30 chips, 0x3C bytes, a halfword each: the id, the code index << 9) and
  `folders` (both, by side); `battle_state` (0xF0 bytes); `rng1`, `rng2`; `joypad_phases` (each console's joypad beat,
  0 to 4); `game_versions` (`redsun`, `bluemoon`); `frame_counter` (the console's frame counter, the toolkit's +0x24);
  `navicusts` (each side's save's NaviCust: `parts`, the 25 parts of 8 bytes at save 0x4564; `grid`, the 5x5 grid at
  0x4540; `color_bar`, 6 bytes at 0x190) and `patch_cards` (each side's save's six slots: `on` at 0x464C, `off` at
  0x4653, a card's number or 0xFF); `game_regions`; `background` (the game state's +0x0F, the background the loader
  picks first, 0xFF none). **`rng1s` and `regular_flags`** (both consoles' RNG1 and Regular
  chip flags) come only when the traced console is side 1: side 0's console runs its frame before side 1's in a tick,
  so the other console's last capture is a frame behind when side 0's setup line is written (EXE5's side-0 recordings
  lack them for the same reason). A decode reads them when present and doesn't expect them on a side-0 recording.
- **exchange:** `navi_stats` alone (EXE4 has no transform records).
- **frame:** `state` (BattleState's first four bytes); no `frames` and `ticks` (EXE4's BattleState doesn't count
  them); `link` (the link struct's status byte), `rng1`, `rng2`; `bs` (BattleState, 0xF0 bytes), `fight` (the fighting
  machine, 0xC bytes: its first byte 4 while the fight runs, 8 after the KO), `gauge` and `gauge_rate`, `paused` (the
  game state's +9, read through the toolkit's pointer), `hud_tasks`, `banner` (the banner block 0x02037CE0's 0x10
  bytes: its +1 the banner, 0x0C the turn's start, 4 "ENEMY DELETED", 8 "MEGAMAN DELETED"); `input` (each player's
  held, pressed and released); `objects` in update order (type, index, flags, params, state, panel, alliance, `flip`:
  the record's +0x17, which EXE4's code reads where EXE6's reads +0x0E, §3.1: the object's element, which the replay
  compares as such; hp, max HP, position, timer, animation, `status`: the collision record's hit flags, +0x54; `f1`:
  its status word, +0x64; `sprite`: the sprite block's palette, its other half and its palette pointer, +4, +5 and
  +0x34); `navi_stats` (both sides' battle blocks); `panels` (the 6x3 field's type and owner, the panel's +0 and +1);
  `chip_blocks` (both, 0x50 bytes, EXE6's layout). Every field but `rng1s` and `regular_flags` is required: the whole
  lab was recorded again by the oracle that writes them (a95f, verify 4a8360d9; the first oracle's `status` read
  EXE6's +0x3C, where EXE4 keeps the hit's damage).
- **sounds:** a line per call each battle frame queued (the frame, the m4a call, its arguments): EXE4's queue holds
  SongNumStart, MPlayAllStop, VolumeControl (the m4a players by EXE6's numbering, EXE4's 0x1210 further), FadeOut,
  SongNumStop, ImmInit and FadeIn (§3.4).

**The decode and the replay** are exe4-compat's (`codec`: the 0x40-byte NaviStats by its fields, the panels, the chip
blocks, the link record of §18 item 23; `trace`, feature `trace`: the lines, their decode, a round's setup in the
engine's terms, the buttons fed, the comparison, `run_round`), as exe5-compat's are EXE5's. A round's stage is the
settings record's layout and actor list (compat's stages.toml; the other ROMs' records are games.toml's distance from
Red Sun US's); its background the setup's, else the record's +5; its stats the saves' NaviCusts and patch cards compiled
by the rules (§8), checked against the recorded blocks at the setup and every frame; an unported field (supports, All
Guard) a need that stops the setup. The compat tables kinds.toml (the object kinds' pools and numbers), actions.toml
(the navi actions past the framework's states, which are EXE5's order: idle 6, a step 7, the buster 8, Cannon 0x0B, the
charged shot 0x24) and records.toml (navis, weapon routines, souls, auras by number) are written by hand as the replays
reach them. The verification workspace's trace-tests `exe4_replay` and sound-tests `exe4_sounds` run them over
data/traces/lab-exe4; nettai-tool plays an EXE4 recording (`Exe4TracePlayer`).

**Known deviations** (the workspace's trace-tests `deviations.rs`, `Set::Exe4Lab`, as EXE6's lab has its own): a
recording the engine departs from on purpose matches every frame up to its deviation, and the frames after it aren't
compared. `modcards/017-custom2`, `modcards/105-panel-change` and `modcards/110-custom3`, frame 274: nettai has no link
cable, and the replay feeds the custom screen's OK as the fight saw it, the cable's delay (4 ticks) after the
original's screen took it, so the side's selection status (BattleState +0x14 bit 0, which OK clears: 0x08020652) runs 4
ticks longer, and the custom HP-drain bug, which counts only while it is set (0x0800C194), drains a point the original
doesn't. No link latency goes into the engine (the user's word); a recording with another cause is no deviation.

**The frames' known shift** (every game's recordings since engine 21cac76de took the link cable out of the engine,
which EXE5's and EXE6's frame comparisons showed as a regression from engine 8a5cc6130: EXE5's backgrounds/0x00 at
807 of 1159 frames). The replay feeds the recording console's custom screen its OK the cable's delay late (compat's
`Round::fed`), so the fight resumes on the original's frame, and so that screen shows its OK that much later: from
the original's OK until the replay's (`held`: the screen still choosing), then its slide-out (`late`, until the fight
resumes) and, counted from its send, the full gauge's stripes and "L or R" and EXE4's "BUSY..." (`gauge`, the gauge's
place, for the rest of the round) `link_delay` frames behind the original's. Each harness gives the OK's frame
(`Round::local_ok`, `first_local_ok`), nettai-tool's replay drivers say the frame's shift (`Driver::feed_shift`), and
`--headless` writes it beside the frames (`shifted.tsv`: frame, what, frames late, x, y, width, height). The verify
workspace's frame comparison (tools/frontend-compare/compare.py) compares the shifted places with the original's frame
and its frame that many frames before, a pixel matching either matching, and reports apart what is exact only so ("at
the replay's known shift: X fight frames and Y of the custom screen's more exact"), what still differs only inside the
shifted places (the late screen over the panels' palettes, which run on time; the stripes' first frames as the gauge
fills: their own runs, "only where the replay shifts"), and the frames held; lab-compare.sh counts a scenario with no
other difference as exact at the known shift. Checked: EXE5 backgrounds/0x00 (807 pixel-exact, 355 more at the shift,
12 only where it shifts, 3 held: nothing else), custom/picks, description, chips/0x001-cannon/hit; EXE6 flow/keep-hand
(a full gauge, two screens), custom/hide-window, take-back, chips/0x001-cannon/hit; EXE4's custom set.

## 18. Engine gaps (must close)

A `game = "exe4"` round starts: content/exe4 states every rule section, MegaMan with his buster and charged shot,
Cannon, the netbattle stages and the roles a round reaches first. Where the engine can't yet say what EXE4 does, a
section or a definition states the nearest value it can, with a comment that points here. **Every entry below is to
be ported from the disassembly before EXE4 counts as done** (recordings check a port; they don't limit it). Each
says the EXE4 routine (Red Sun US), what it does, what the engine lacks, the shape the generalization takes (rule
data, not a third named branch; EXE6's and EXE5's sections restating theirs, one commit each), and which fields hold
a placeholder until then. tools/exe4/gen_rules.py (verify) writes the table sections: change its output there.

### 18.1 Reactions and the hit

1. **Done: ice is a push.** 0x0801335A: a body that ends a move on ice (panel type 7; not of aqua, floating or
   submerged, flags 0x24, and affected by ice, 0x02000000) gets a push bit ORed into the collision record's final
   modifier (+0x0D) from the table at 0x080133B4 by its side and the move's direction (`00 40 80 20 10` for side 0,
   `00 40 80 10 20` for side 1: none, up, down, left, right), so the slide that follows is the push's (the push rows
   at 0x0800ACDC: a panel up, down, back or forward), and a hit's flinch bit the same tick makes it a drag (item 2).
   The rule `reactions.ice`: `{ slide = rows }` (EXE6's and EXE5's six rows by direction, slide type 2) or
   `{ push = { side 0's bits, side 1's } }` (EXE4's, the ROM's table: gen_rules.py reads it). It runs where EXE6's
   ice does in the intake (before the traps, which absorb it with a hit, as EXE4's 0x08023048 comes after it too);
   EXE6's test of a drag or a move under way before the move's end (`sub_801A36A`) is not EXE4's, but a move's end
   comes with neither.
   - **Done: the move's direction** the table is read by. EXE4's (0x0800AF90, from its `object_updateCollisionPanels`,
     0x08012D9A) is 0 none, 1 up, 2 down, 3 left and 4 right whatever the side, across before up and down, never
     EXE6's 5 (other); EXE6's and EXE5's (`sub_800E994`) is back and forward by the side, 5 for a move of two panels
     or more right or down or a diagonal one. The rule `reactions.move_direction` (`by_side`, `absolute`).
2. **Done: the drag is the flinch bit with a push.** The hit intake (0x08013858, at 0x080138D2) takes a drag where the
   final modifier has a push bit (2 to 7, 0xFC) and the flinch bit (1), clearing the flinch request, else a slide
   unless dragged or moving (0x100040); an obstacle's intake (0x0801393E, 0x080139D8) a push the same way, unless
   moving. EXE6's and EXE5's push bits are 0x3C, the drag bit 0x40 (`sub_801AEB0`; an obstacle's `sub_801AD9E` tests
   0x40 alone, which no hit has without a push bit). The rule `reactions.push_reading.drag_bit` (EXE6 and EXE5 0x40,
   EXE4 0x01), the push bits by `push_reading.bits`.
3. **Done: the slide reads the push always.** 0x08010294 (the slide) and 0x08010ABC (the drag) call the push reading
   (0x0800ACAA) whatever started them, and the intake sets no slide type; EXE6's and EXE5's read by the slide type
   (`sub_800E468`: their drag's `sub_800E45E` passes 1, which the routine overwrites). In EXE4 nothing but a push
   starts a slide (its ice is a push, item 1; it has no roads and no metal slide), so the engine's slide type 1,
   which the intake sets with a push's request, reads the same: no rule. (EXE4's reading stores the row even when
   its first panel is closed, which the slide then tests; EXE6's stores none: nothing reads the row after a failed
   slide.) Left of EXE4's slide and drag, as their own gaps: item 33.
4. **Done: elec on ice.** 0x08012CF2 (the hit kernel's and its unfiltered channel's): an elec hit counts once more,
   as null damage, on a body standing on ice (panel 7), a fire hit on grass (6); EXE6's fire on grass alone
   (`applyHeatOnGrassDamage_300766c`), EXE5's and elec on its sea (0x08016AF6). The panel type's rule
   `doubles = element` (EXE4's grass fire, ice elec, gen_rules.py reading the kernel's comparisons); the reactions'
   `hit_test.elec_bonus_on_sea` is gone.
5. **Done: the hit test.** EXE4's kernel (0x08012AFC, from the pair test 0x08012AE0; its unfiltered channel
   0x08012D1E), read against the engine's (EXE6's `sub_3007218`):
   - as the rules said: no FloatShoe self bit, no body under a sea, elec reaching no submerged body; a guard breaks to
     types with 0x1002 (0x08012B8C, with no 0x4000 case: the engine's 0x1002 either way) and turns aside what lacks
     0x0C004000 (the engine's 0x0C005000, which a type with 0x1000 never reaches);
   - its own: an untouchable receiver is tested with the invulnerable one (0x08012BDC: 0x08000008), after the guard
     and the air/ground test, so its guard still turns a hit aside. The rule `hit_test.guard_before_untouchable`
     (EXE4 true; EXE6, EXE5 false). (EXE4 sets the flag for soul 15: 0x0800E17E, from the souls' table at
     0x0800E0A0, by NaviStats +0x24.)
   - Nothing reads or can reach the rest, so no rule: it records no hitter bits (`hit_by`) and no hits by flip,
     ORs no secondary element (its attacks have none), has no thaw or bubble multiplier (EXE4's flag 0x10000 is
     not a freeze but a player action's, 0x080EBA84; its freeze is elsewhere, item 7), no aqua-on-ice freeze (the
     role is EXE6's), and multiplies by a shift (0x08012CA4: 1 for a weakness, the engine's 1 + 1).
   - For others: a guard marks the receiver's +0x26 with 1, not a bit by the hitter's flip (the chips wave: a guard
     chip reading the engine's `guard_dirs`); the counter's mark (0xFF added to +0x38, 0x7F off the mood) and hit
     flag 0x80 by the hitter's side's 0x0800F49C (2: the table at 0x08012CEC) are item 9's (group A).
6. **Done: request clears.** EXE4's ends: an attack's (0x0800CA28, 0x0800C9FC) clears the six attack requests
   (0x3F) and nothing more; a paralysis's, a flinch's and a drag's (0x08010A74, 0x080109E6, 0x08010C28) 0x43F, the
   anti-sword trigger too (0x400, which AntiSwrd's trap sets, 0x0802309C). Stated: `request_clears` (attack none, the
   others `anti_sword_triggered`). EXE4 has no kind byte for the exit (EXE6's `sub_801171C` reads the attack's
   +0x1C): each action calls one of three routines, which map onto the engine's kinds, so no rule:
   - 0x0800CA4A, bare (animation 0, idle, the attack's step): the move's (0x080EB252, 0x080EB314), EXE6's kind 4;
   - 0x0800CA28, the requests, the charge and the action in use cleared too: the buster's recovery (0x080EB3D2,
     0x080EB3EC) and the charged shot's recovery's end (0x080ECCC8), and the other navis' and souls' actions; the
     lockouts left as they are (`exit_attack(true)`);
   - 0x0800C9FC, that and AIData +0x3A (the chip lockout) from the attack's +5 (its lockout) whatever the attack's
     kind, a 0 clearing it: the chips' and weapons' (65 callers: Cannon's 0x080EB9E8, the charged shot's move cut
     0x080ECCE0, ...). The rule `reactions.attack_end_lockout` ("chip_lockout"; EXE6's and EXE5's "by_kind").
   - EXE4's exits zero no buffered move (EXE6's AIData +0x1A) and no special source (+0x1B): nothing in EXE4 sets
     either while the engine's movement is EXE6's (item 24, group B's).
7. **Done: statuses.** The status table 0x08018550 is generated (4 groups of 4). The status rules, each read from
   EXE4's routine:
   - **Done: the drains.** The NaviCust's HP bug (0x0800C164) reads NaviStats +0x0E as the period itself (no table)
     and drains through a pause (EXE6's `sub_8010230` reads a period by level and holds while paused); the custom
     screen's (0x0800C194) reads +0x0F as the period, and runs while the side's status (BattleState +0x14) has bit 0,
     its selection running (EXE6's `sub_80102AC` by level, while it has bit 0 or 2, the screen up). The rules
     `status.hp_drain` (`periods`: eight by level, or `"stat"`; `stops_while_paused`) and `status.custom_drain`
     (`periods`, `status`'s bits); `hp_bug_periods` and the engine's own custom table are gone. The engine's status
     byte has bit 0 where the side's screen says so (`custom::Side::selecting`; EXE6's and EXE5's never do, their
     traces compare the byte): EXE4's screen (group A's) sets it while its selection runs (in the lab's
     `custom/cannon` from frame 273 until OK).
   - **Done: the loss of HP.** EXE4's `object_subtractHP` (0x0800AB92) takes the HP alone, but its player's status
     block (0x08013A48) drains the side's gauge with a hit's HP (0x0800AB9E, by the loss ×128, as EXE5's), shows the
     hit (white, its sound) only with HP left, takes the element-5 damage without the gauge, and at 0 HP asks the
     side's rules after (0x0800EBC8: a dark MegaMan's last stand, emotion 5, which holds him at 1 HP and asks for
     request 0x1000: the rules' `hp_emptied`, to write with the dark chips). The rule `status.hp_loss =
     "hit_drains_gauge"`. Its hit sound is one, 0x6B, for a navi whose NaviStats +0x26 isn't 1 (0x6D else), which a
     netbattle's never is: the roles' `own_hit` and `hit` both 0x6B (group A's role fill). Unported: 0x0800EE4C
     after the sound records the hit navi's panel into the hitter's side's records (0x02037A90, 0x02037C60), which
     nothing read so far reads.
   - **Read, as stated:** the status block (0x08013A48) is EXE5's order (`reactions = "flash_timer_first"`): the
     flash's timer first, whatever the battle's flags (0x080134F6), then the slides, the drag (keeping a paralysis a
     counter just made: its flag2 0x8000, the engine's 0x4000) and the flinch; the mercy flash hides the navi while
     its timer's bit 1 is clear (0x08010430, `flash_hides_on_clear`); a hit's bug is taken (0x0800D9E8) before the HP
     bug drains (`bugs_before_drain`); the weakness mark tests the damage of the element the navi is weak to
     (0x080133CE, the table 0x08013408: `weak_element_damage`; the mark's place, a table by the object's +0x0F
     through 0x08011878, is the drawing's); no per-form tick (the player's update, 0x080EAECC); no no-charge drive.
   - **No freeze in EXE4.** Its hit kernel has no aqua-on-ice freeze and no thaw (item 5), and the lab shows none:
     `ice/aqua` and `status/freeze` (Bubbler on a navi on ice) start no reaction. Its flag1 0x10000 is action 13's
     (below), not a freeze.
   - **The status block's extras, the engine's requests:** a pending special (0x0800B8B0: request 0x1000, which the
     last stand asks for, sets AIData +0x10 to 15; any other value there but 0 and 0xFF) starts action 13 (0x080EBA70:
     the battle dimmed, flag1 0x10000 while it runs), and while flag1 0x10000 is set the block goes straight to the
     action. That is the engine's `volley` request and its status (`action_requests`: the roles' `volley`, EXE5's
     last stand's), which EXE4's `hp_emptied` hook asks for with the dark chips (group A); AIData +0x10's other
     values (a soul's change, likely) are the souls'.
   - **Done: the hit's bug** (0x0800D9E8): NaviStats' byte by the code (collision +0x48) gets its argument (+0x49);
     code 0xFF raises the NaviCust bug the argument numbers (0x0804770C: the side's bug counts, 0x0203F6E0 and
     0x0203F6F0, kept from the compile as the rules state's `bug_counts`) and runs every bug at its count's level
     again (0x08042AE0, the battle's stats; HubBatc's writes the game state's maximum HP, which no battle reads); then
     the abilities come back (0x0800D906, EXE6's `sub_801393A`: the answer "edited"). The rules' `navi_bug`
     (content/exe4/rules/navicust/bugs), an if-chain on the codes EXE4's objects set: ShadeMan's crush 0x0D (the move
     bug); RedSun's stripping hits (attack 0x87) 1 to 7 and 0x0A; attack 0x41's (effect 0x1B) those, 0x0C and 0x12;
     weapon routine 0x5C's hit 0x0E (the HP bug's period); BugBomb (attack 0x77: kinds 1, 2, 3, 4, 6, 8) and BugCurse
     (attack 0x80: kinds 2, 3, 4, 6) 0xFF. The original allows any other code (the byte it numbers); no object sets
     one. Kind 8 (the weapon bug: the charged shot becomes weapon routine 0x1F, 0x20 or 0x21) waits on routines 0x1F
     and 0x20, which BugBomb's port brings. Open: 0x0800D906 copies the weapon bytes in any form where the engine's
     refresh keeps a form's own, which no recording tells apart yet.
   - With others: the emotions (item 8, group A); a soul's break by a weakness hit (`form_break`,
     `weakness_hit_breaks_form`: EXE5's until the souls are read).
8. **Emotions.** **Done** (group A; §7 "As ported"): EXE4's order, mood rules, Full Synchro and its aura, the
    starting mood. The worried emotion waits on M-Cannon (the lab's emotions/worried).
9. **Counter hits.** **Done** (group A; §7): a counter paralyzes (status 0x12), counts 0xFF toward the hitter's mood
    and wears 0x7F of the receiver's; Cannon opens its counter window.
10. **Done: the stance counter.** EXE4's stance is AntiMagc's B+Left ability (NaviStats +0x0C = 0x27; action 114,
    0x080EE9EE: 13 ticks, registered as AntiDmg's trap, chip 145, through 0x08022FDE); a hit caught in it raises the
    anti-damage or anti-sword request, and the stance's next step starts the counter (0x0800C780: action 56 for
    anti-damage, 57 else) and returns: it runs from the next tick, `stance_counter = "next_tick"` (EXE6's; EXE5's
    runs at once). The lab's `stance/` (a95f's): AntiMagc against a Cannon, the buster and a Sword, early, on side
    1, and traced on Blue Moon; Shield's and Reflect's (action 37) too. Their actions (114, 56, 57, 37) are the
    B+Left programs', to port with them; EXE4's counter starter is its own (AIData +0x10's flags 16, no stance
    lockout or variant: EXE6's `sub_80105F2` keeps both).
11. **Done: the overlay restart.** What a navi wears restarts by 0x080CC61A (EXE5's 0x080C374E: the animation
    reloads at its next step), `overlay_restart = "reload"`, called by the per-navi flinch and drag hooks (by
    NaviStats +0x23: 0x0800DC9C, MegaMan's restarting AIData +0x48's object; 0x0800DD82, MegaMan's none, navi 5's its
    +0x60) — the identities' `overlay_hooks`, to state with what MegaMan wears in a soul (the souls wave). The lab's
    `overlay/aqua` and `overlay/proto` (a95f's: a soul flinched, dragged and shooting, with and without Full Synchro)
    check it once souls play.
33. **EXE4's reaction actions and its slide** (found by group C; the player's action table, 0x080EAEFC, entries 2 to
    5).
    - **Done: the actions.** The flinch (0x08010960), the paralysis (0x080109FA) and the drag (0x08010ABC, its end
      0x08010C16) differ from the engine's (EXE6's) beyond their hooks and requests: none marks the action in use
      (flag 0x400000) or lets go of the overlay link (AIData +0x68); the flinch keeps the body's height as it snaps it;
      the paralysis snaps it sliding or not and counts no reaction (the side's stat 3); the drag takes the flinch's
      pose (EXE6's 2 paralyzed, 0 with SuperArmor), keeps the height, counts none, and at its end clears the drag alone
      (no slide, paralysis, heat-trap or flag2 0x10 clears) and goes to idle in its pose, paralyzed or not. The rule
      `status.reaction_actions` (`marked`, EXE6's and EXE5's; `plain`, EXE4's), a bundle as `status.reactions` is;
      the drag's pose and end are `status.drag` (below).
    - **Done: the drag's pose and end, each game's** (`status.drag`: `poses`, the first that holds of a paralyzed
      navi's and a SuperArmor one's, else `otherwise`; `ending`). EXE6's (`sub_80178D4`, `sub_8017A38`): paralyzed
      2, SuperArmor 0, else 1; at its end a paralysis that outlasts it goes on as the paralysis action, else the
      slide, the paralysis, the heat trap, a slide request and the slide's state are cleared, the navi stands, its
      overlay refreshed (`resumes_paralysis`). EXE5's (0x08014304, 0x080144CE): SuperArmor 0, else 1, no paralyzed
      pose; at its end the drag and its use are cleared, its requests, and the navi stands, whatever the paralysis
      (`stands`): the engine had EXE6's. EXE4's (0x08010ABC, 0x08010C16): 1, kept to its end (`keeps_pose`). EXE5's
      paralyzed start is barely reachable: its status block ends a paralysis at a drag's request unless a counter made
      it that tick (flag2 0x8000), and a counter's own paralysis waits for the drag's end (the lab's
      `drag/counter-push`, a95f's: the drag starts at 457 without it, the paralysis at 483; with SprArmr,
      `counter-push-superarmor`, the same); a navi already paralyzed when a counter push lands would show it. Both
      recordings match every frame, before the rule and after. Read beside it, EXE5's own: its flinch, paralysis and
      drag starts call 0x0802D644, which resets the side's state block (`sub_802E070`'s) at +0x0B and +0x2E (the mode
      chips' mode and ticks, which nothing in a battle reads), +0x0F and +0x50 (the SELECT special's, the operation
      battle's: the user declined that mode), then calls 0x0802FB9C and 0x0802FD50 (EXE6's `sub_802F084`), both
      unported; its drag start resets the facing (flip 0 and 0x0800D1EA) where turning is off (AIData status 0x400
      clear: every netbattle's), which leaves a navi facing as it started.
    - Read and the same: the per-navi flinch, paralysis and drag hooks (0x0800DC9C, 0x0800DD14, 0x0800DD82: the
      identities' `overlay_hooks`), the requests their ends clear (item 6), the drag's speed and its step (ice a panel
      more); the slide's start and step (0x08010294, 0x080102FC) as EXE6's (item 3).
    - Left, unreached or another's: a reaction's entry clears the side's timed effect (0x08022F2C: the side record's
      +0x0B kind, +0x2E ticks, +0x0F, which an action's effects table, 0x080EC9CC, sets; to port with the chip that
      sets it); the AI status bits a flinch's and a paralysis's entry clear (0x7F; the engine's 0x20005F: the bits
      differ only in 0x20, EXE5's dive, and 0x200000, neither EXE4's); the slide's collision panels updated each tick
      (0x08012D9A: the direction ends 0, where EXE6's sets the slide's; nothing reads it after a slide); a type 10
      panel stopping a slide or a drag unless the body floats (item 12).

### 18.2 Panels

12. **Done** (panel types 5, 10 and 11). 0x0800A3A8's flags: 5 is 0x30010, 10 is 0x10210, 11 is 0x10010. Each is
    named for what it is, from what reads it and the chips' words:
    - **5, `metal`** (the engine's EXE5 metal; a riveted plate): its flag 0x20000 is what the panel routines refuse
      (0x08009AEC, 0x08009BAC, 0x08009BF0, 0x08009C4C, 0x08009D04: crack, break and their kin), so nothing cracks or
      breaks it. The rule `panels.types.metal.unbreakable` (EXE5's metal is breakable: no such flag). Two routines
      skip the test and would crack it: 0x08009B50 (EXE6's `object_crackPanelDup1`, which the engine has no use of)
      and 0x08009CAC (`object_breakPanel_dup3`, the engine's `break_panel` with dup2, which refuses); poison
      (0x08009D68) doesn't test it either, as the engine's doesn't. The chips wave reads which routine each EXE4 chip
      calls (a chip of dup3's on metal is to split off then).
    - **10, `pitfall`** (SandRing's "opens a pitfall trap"; a sand pit): a slide (0x080102FC) or a drag (0x08010B54)
      that reaches it stops there unless the body has FloatShoe (f1 0x20; an aqua body's slide on ice reaches the
      test too), the rule `stops_slides`; and it turns normal by its timer at the panel's +0x12 (the panel tick's
      0x0800980E): a type change makes it 190 and counts at once (0x08009DC4), a stage's starts armed (bit 15,
      0x08009120 writes 0x80BE to every panel) and counts from the tick a grounded body (0x0F800000, none floating:
      0x00100000) stands on it, then normal without a blink. The rule `crumbles = 190`, the panel's
      `crumble_timer`.
    - **11, `hole`** (the Hole chip's "appears Hole in front", DrkLine's "turns all rows into Holes"): a solid panel
      (normal's flags) that a navi standing on it closes (0x08013318, in the intake before the slide triggers): unless
      the battle is dimmed, a navi whose light/dark value (NaviStats +0x36) is above 499, the default 500 too, sets
      his collision's panel normal. EXE4's rules' `navi_intake` (rules/light_dark), the setup's `karma`
      (default 500). Its check of 0x0800F49C counts for nothing (the `movs r0, #5` before its `beq` sets the flags),
      and NaviStats +0x26's 2, which skips it, is only a story's other navi's (0x08041100). exe4-compat's recordings
      state no karma yet: the default, 500 (group B's: the NaviStats' +0x36).
    No netbattle stage has type 11; the stages with 10 are generated now.
    - **The pitfall traps** (found by the lab's stages/typeA-row/walk): EXE4's `object_canMove` and its kin (0x0800AD2A,
      0x0800AD54, 0x0800AD7E) also refuse a body standing on a pitfall, unless it floats (`cmp r1, #10`, then flag
      0x20): the move's start (0x080EB20E) goes back to idle. The rule `traps` (the engine's move start and the
      content's `can_move`). An obstacle's slides stop on a pitfall too (0x080106B8, 0x080110B8 test the float flag;
      0x08011250, 0x080113C8, 0x08011532 don't, which no obstacle has): `stops_slides` in the obstacle slide.
    - **The panel routines' masks** (group A's reading): EXE4's eight (0x08009AEC to 0x08009D68) clear 0x23F5F
      before the type they set (0x23F0F for a crack, which keeps the solidity and the crack bit), as EXE5's do
      (0x0800AFF8 on; its sea has 0x20000); EXE6's clear 0x3F5F. The rule `panels.type_mask` (EXE6's 0x3F5F, EXE5's
      and EXE4's 0x23F5F): the engine cleared EXE6's in every game, so an EXE5 sea panel cracked or poisoned kept
      its 0x20000 until its next refresh.
13. **Done** (what each panel does). Read against the engine's (EXE6's, EXE5's lava):
    - **Lava's burn** (a player's 0x08013128, then any body's 0x0801309E; the player's runs first, so the second finds
      the panel normal): a grounded body not of fire, without FloatShoe and none of the status bits 0x206 (EXE5's
      0x88000206), takes 50 in fire shifted by its weakness to fire, unless flagged 0x09, and 20 more off its mood
      (collision +0x36, the kernel's mood damage: EXE5's has none); the panel turns normal with its burn's spark. The
      player's has no dimming test (any body's has). A panel type's `burn` is a table now: `damage`, `spared_by`,
      `mood`, `players_while_dimmed` (EXE5's `{ damage = 50, spared_by = 0x88000206 }`).
    - **Poison and grass** (0x08012FF2, EXE6's `sub_801A186`): the same tests (poison's 0x08000028 immunity, the grass
      test reading the status word after an immune body, as EXE6's), and grass heals a wood body on the battle's
      20-tick count at any HP (EXE6's and EXE5's on the 180-tick one at 9 HP or less): the rule
      `panels.grass_heal_slows_at` (theirs 9, EXE4's none). It has no pause test, but EXE4's player never runs paused
      once in control (its header flag 0x04 cleared, `paused_navi = "stops_at_control"`): a95f's
      `panels/poison-pause` holds its HP through a START pause, the drain due at the pause landing the tick after it
      (and replays every frame). Its element test reads the whole byte (EXE6's the low nibble; EXE5's the whole byte
      too): EXE4's objects have no high nibble.
    - **What passes over a panel** (0x08013058, from the collision's removal, 0x08012A50): grass of fire and lava of
      aqua turn normal (`cleared_by`, EXE5's 0x08016D14 less its metal of wood), unless the hitbox has 0x0C000000. It
      has no pause test (EXE5's and EXE6's have): no hitbox is removed while paused (frozen objects), so no rule.
    - **Holy** (9): the final damage halves on it (0x0800AC3A, EXE6's); EXE4's barrier (0x08012DF8) is its own, with no
      holy test (kinds by +0x04: the barrier chips' port, the chips wave).
    - **Ice** (7): item 1's push; elec doubles on it (item 4).
    - Left, the souls' and the patch cards': FireSoul (NaviStats +0x24 = 5) on lava heals 50 and clears it, with effect
      7 at the navi and sound 0x9A (0x080131B8) instead of burning; WoodSoul (+0x24 = 12) heals 1 more on grass while
      BattleState +0x16 is 0 (0x08012FB6, after poison). The panel trail is the patch cards' (NaviStats +0x1B, 0x080EB254,
      the move's step): every step (no chance, no RNG draw), the panel left unless missing or broken becomes the
      trail's type by EXE4's number (1 by `object_breakPanel_dup2`, any other by a type change: a crack by type 3,
      not the crack routine), sounding 0x124 for poison onto a panel that wasn't (and 0x95 when the type change's
      return, the panel's occupants' collision bits, is 3: never with the navi's own body on it); EXE6's trail (a
      NaviCust bug, a chance by level) isn't it: to port with the patch cards.
14. **Done** (start-visible panels and front edges). EXE4 keeps no grids: its field's init (0x08009120) marks all 40
    panels visible (0x08009186: 0x40 into each flags byte at 0x02037B36), its drawing (0x080092AC) draws each valid
    panel (x 1 to 6, y 1 to 3) while visible, and the front edges under row 3 alone (0x0800937A), each by its row-3
    panel's visibility (0x080094C4, else 0x080094FC blanks it), as the engine draws a panel's `front_edge` with it.
    Stated: `start_visible` all true (what the engine reads of it, the valid panels', is EXE5's grid), `front_edges`
    row 3, written by gen_rules.py from that code. The any-side step: EXE4 has no `sub_800E680` (EXE5 neither), so its
    rows are EXE6's, for EXE6's code (a chip's) in an EXE4 arena.
15. **Done** (battle mode 1's mend and the reservations).
    - **The mend:** 600 ticks in every battle: the field's init (0x08009132) and the panel tick (0x0800976C) store
      the one constant whatever the battle's mode (`mend = { normal = 600, battle_mode_1 = 600 }`, now asserted).
    - **Reservations:** EXE4's `object_reservePanel` (0x080143A8) marks the panel alone (no header flag 0x20 on the
      holder, as EXE5's), `reservations = "unmarked"`.
    - **The destroy:** EXE4's objects end in 0x08010560 (the collision freed, then the object: EXE5's 0x080138F2's
      steps; the state tables of over a hundred kinds hold it, the boulder's and the flag's among them), which releases
      no reservation. The routine matched to EXE6's `object_genericDestroy`, 0x080D8C58, which unregisters the
      collision first (`object_removeCollisionData`, 0x080129FC), is one kind's own (EXE6's `sub_80CFC08`'s
      counterpart, 0x080D8BC8: its state table 0x080D8BDC), to port with it.

### 18.3 Flow, stages and the link

16. **Done: the time limit and the double KO.** No double KO: when both navis' shots are due on one tick, side 1's
    lands first, side 0's deletion ends the battle and side 0's shot, whose tick tests the battle's end first
    (0x080CD3DE), goes without landing (the lab's flow/ko-double: both at 1 HP shooting on the fight's first tick).
    The engine's update order and the projectile's own test give it, no rule: flow/ko-double replays every frame. **Done: the turn timer and the
    judge** (the lab's timer/win, loss and draw, each on a Red Sun and a Blue Moon console: single battles, battle type
    0x47, played to the 15th custom screen; every frame replays and every frame compares). EXE4's is EXE6's machine: the
    fight's timer runs (0x08007E4E) where event flag 0x1187 is clear (the main subsystem's, not a netbattle's), the
    battle type (BattleState +0x0F) is 0x46 or more and BattleState +8, the custom screens so far, is 15 or more; the
    fight's start (0x08007064) sets the fighting machine's +0x0A to 659 with banner 0x10 ("FINAL TURN"); it counts down
    (0x08008066) and under 60 sets BattleState +0x0B, the round's result 7 (0x080079D6), the fighting machine's state
    0x14: TIME UP for 60 ticks (0x08007378), then the judge (0x0800739E: 0x08021F54 and 0x08021F94, EXE6's
    `sub_802CB38` and `sub_802CB78`, the same code; its banner 0x28 with the HUD's task 9). What was EXE4's own: from the
    15th screen the screen's send (0x0801E1B4, EXE6's `sub_8027D78`) returns before emptying the gauge (0x080159B0), so
    EXE4's gauge, which empties there, stays full through the last turns (`restart_gauge` now returns in the last
    turns, as both games' routines do); the judge's numbers show only while its banner holds (its release, 0x080163B6,
    slides the banner out and clears them); "FINAL TURN" is drawn in palette 14 (0x0801EE9C's map; the custom layout's
    `turn_limit_palette`). The traces' setup gives a single battle no set (exe4-compat: battle type 0x48 to 0x4A is a
    triple battle's, 0x08007464).
17. **The fight-live test.** The fight runs while the fighting machine's first byte is 4 and BattleState +3 is 4; for
    one tick as the custom screen closes the machine still reads 4. Pause sets fight[0] to 0x18. To compare with the
    engine's fight states in step 5.
18. **Battle type.** A netbattle record's +4 (0x46; the battle state's +0x0F, 0x48 in the lab's) is EXE4's battle type,
    its battle flags by it (0x08007EEC's table). The engine's `mode` is EXE6's numbering. Placeholder: each stage's
    `mode = 0`, `effects = 0x88C` and `panel_pattern = 0x38` (EXE5's netbattle's).
19. **Done: the stages that waited** (the records with obstacles: actor kinds 3, 5, 6 and 7 of 0x080FC138's actor
    lists, which 0x08006754's table at 0x08006778 spawns). gen_rules.py's `STAGE_ACTORS` names each kind's module and
    `STAGE_ENTRIES` what its spawner reads of the entry, as named fields (no raw argument: the gears' first byte, 25,
    nothing reads); every one of the 96 link records is a stage now, and stages.luau's header lists none waiting.
    - **Done: kind 3, the boulder** (records 20, 21, 24 to 31; content/exe4/objects/boulder). Attack object #0x6E
      (0x080DB0EC), EXE6's code (`sub_80D2290`, @exelib/boulder) but for its init writing no NameID, its tick taking
      hits never pushed (0x080139A0) and reacting by 0x08014058 (EXE6's `sub_801B4D4` but for its crushing hits,
      0x00800002: no body's touch, and no removal request read; the obstacle reaction `destroys_sparing_bodies`), its
      waiting keeping action 0, its action table's 7 rows (1 also stops it running paused, 3 nothing, 5 the
      knock-back), its leaving by its HP (left: a puff, effect 21; none: two chunks of debris, lib/rock, and a splash,
      effect 1), its end 0x08010560, and the damage it is thrown for (100). Its hit sound is the roles'
      `sounds.damage` (0xC8). The lab's stages/grass, grass-rows-a/b and grass-checks-a/b (null, stand, walk) replay
      every frame; their chip scenarios wait for Bubbler, Thunder1, HeatShot and ElemLeaf.
    - **Done: kind 7, the flag** (records 76 to 87; content/exe4/objects/flag). Attack object #0x9C (0x080E06AC),
      EXE6's code (`sub_80D8C5C`, which no EXE6 stage places) but for its sheet (10-32), its NameID (0xFD; its HP
      number's) and its HP check (0x080E07CE: a burning panel first, 0x0801309E, the object call `burn_on_panel`; a
      holy panel's halving of the total, rounding up; the hit's sound 0xC8). Each side's in its back corner (the
      entry's first byte: bit 7 the side, the rest the HP in eights, 0x7D and 0xFD, 1000; gen_rules writes them as
      the stage entry's named `side` and `hp`), its HP numbered. Its HP out (unless time is up: `battle_isBattleOver`'s
      Z flag, `battle.time_up`), sound 0x6F and its side loses the round
      (0x080E0842, EXE6's `sub_80D8DEE`: the side's actor count, BS+4, to 0 and the round's time-up byte to 1, the call
      `battle.lose_round`; it also sets battle flag 8, which nothing in EXE4 reads: 0x08007A8C's 28 callers test 1, 2
      and 0x10). The lab's stages/lava-middle-close, typeA-row, ice-columns and holes-diagonal-b (null, stand, walk)
      and panels/lava-blinking replay every frame, and so does stages/lava-middle-close/flag-break (side 1's flag
      broken by 25 Cannons, through the round's end into round 2's fight; `lose_round` zeroes the side's actor count,
      BS+4, which the fight's result reads, not its alive count, BS+0x12).
    - **Done: kind 5, the gear** (records 32 to 35; content/exe4/objects/gear). Attack object #0x76 (0x080DBD94, EXE6's
      `sub_80D385C` by its place), its sheet 10-33, 50 HP, guarded (its status bit 0), hitting for 20 (0x080068C0's
      damage word 0x00940014) what it runs into, on its panel's side for the other side (its mode 0, the stages'), at
      half a pixel a tick: past its panel's center it checks the next panel, else the one under where it is going
      (`sub_800E258`, the BIOS's signed divisions), and turns back from one off the field, not solid, with a body (other than a
      navi's: 0x03800000) or of the other side (0x080DBF5E); sound 0x149 as it starts. Its reaction is 0x0801427A (EXE6's
      `sub_801B878` without LilBoiler's eruption test: destroyed by any touch, the HP as it is; none of EXE4's six
      reactions reads the removal request), its end an explosion and 0x6F (0x080DBF24). Its modes 1 (a lifetime and its
      side's registry) and 2 (other collision rows, a spawner's slot) are ported for effect #0x49 (0x080E7244) and an
      actor's 0x080C7C74, which aren't. The lab's stages/type5 and type5-row (null, stand, walk), panels/type5-stand-on
      and stages/type5/gears-roll and gears-hit (a95f's: navis out of its row for 900 ticks, the gears bouncing between
      the edge and the center line; then one runs into side 0) replay every frame.
    - **Done: kind 6, the wind** (records 88 to 95; content/exe4/objects/wind and objects/gust). Effect object #0x41
      (0x080E6820, EXE6's `sub_80E5244` by its place, the navis' AI wind no EXE6 stage places), spawned by 0x080068DE
      through 0x080E6932: its first parameter the entry's first byte (every record's 0), its side the entry's second
      (gen_rules writes `side`; the entry has no panel), its third 1, not running while dimmed, its position the
      registers. Unseen, it plays sound 0x121 (`wind`) as it starts, then (phase 0, 0x080E687C) while its side has no
      registered wind (BS+0x88 + 4 * side, `battle.wind`: a stage's waits) counts its row down by turns from 3 (+0x6C:
      2, 1, 3, ...), takes the other side's first panel of the row from the far edge (0x080E6950:
      `object_getClosestPanelMatchingRowFiltered` for the other side with 0x080E69AC's {0x20, 0} / {0, 0x20}; none
      where a neutral object, panel flag 0x800000, stands; a first parameter walks it on along the side's forward to
      the panel before a neutral object or the edge) and blows a gust there, then waits 12 ticks (phase 4): a gust
      every 13 ticks. Its two gust slots (+0x60, +0x61) would hold it with both full, but EXE4's gust spawner
      (0x080D68AC) sets a slot with a word store, which the ARM aligns down: the first slot becomes 1 and the second 0
      whichever it names, so while the first gust is out each next one tracks the second slot, reads 0 at its first
      update and ends unseen. The gust is attack #0x49 (0x080D6724): @exelib/gust in EXE5's look (no hit stops it,
      0x080D6842 tests against 0; any first parameter pushes weaker, 0x080D67BA) with flip 0 (EXE4's flip,
      0x0800ACA6, is the side; its speed's sign is the side's alone, 0x080D679E), the spawner EXE4's own (a byte
      side, no test for a missing tracker, the word store). Its third parameter 0 (0x080E68F8: the side's wind,
      registered as a navi's, which its end clears; callers 0x0800E108, 0x0802252E, 0x0802254C, 0x080C3374 and
      0x080F08A4 to 0x080F0E00) is ported in the kind; that spawner waits for its callers. The lab's
      stages/type5-ring, poison-middle, poison-row and holy-inner-top (null, stand, walk, aqua, fire) replay every
      frame (elec and wood wait for Thunder1 and ElemLeaf), and type5-ring/stand, poison-row/walk and
      holy-inner-top/aqua compare every fight frame pixel for pixel, gusts and all (frontend-compare/exe4.txt).
20. **The link pick.** 0x0803AA6C draws `PosRNG2() % count` (0x44 for a single battle, 0x60 for a triple one: by the
    battle type, 0x08007D68) into the first 96 records, then `PosRNG2() % 24` into the backgrounds (0x0803AAA4). A
    set's first battle picks its rounds' places at once (0x08007D68). **Done:** `link_pick.backgrounds` (the table's
    24, by their names), `first_round_stages` (a random match is a triple battle's: every round among the first 0x60).
    No RNG field: nettai picks a match's places before the battle with its own generator (docs/frontend.md §2), so
    only the odds are the game's, which the lists state. a95f's sweep (1,000 seeds, tools/chiplab/gen_exe4.py
    `STAGES`) drew from all 96 records: no panel type 11; its obstacles are the four of item 19 (the boulder #0x6E, the
    gear #0x76, the flag #0x9C, the wind, effect #0x41). `stages` lists all 96 records, so the odds are the game's.
21. **Done** (backgrounds' names). The loader (0x08085430) takes the game state's +0x0F, else the settings' +5, else
    the map's (0x08085BAC, its table at the literal 0x08085BCC, default 3). The 22 that maps draw are named for their
    areas, by the menu's names (tools/backgrounds: names.tsv, areas.py exe4; docs/frontend.md §1 has the table). No
    map draws five, which story battles' settings state; the scripts say which (map script command 0x23 starts a
    cutscene, cutscene command 0x51 `[record, 1]` a battle by 0x08007BEC). 0x18, 0x19 and 0x1a are the tournament
    venues' (`stadium`, `air-stadium`, `colosseum`: each the one map whose scripts start its eight records), 0x17
    Duo's two battles' (`duo`), 0x09 the DarkSoul battle's on CastNBMComp, whose own background is 0x03
    (`darksoul`). The background 0x17's scroll (0x08001F88) speeds up to 4 pixels a frame: the pack draws it at that
    speed.
22. **Done** (banners): the win banner is settled (§16, `never`); the banners' pictures are extracted (§14: the table at
    0x08016C04) and each is named (gen_content.py's BANNERS: EXE6's names for EXE6's records, EXE4's navis' by their
    words); a round's start is `battle-number-start` (0x30), EXE6's name for it.
23. **The link record** (agent ae3a's reading, for step 3's codecs): built at 0x0203BD40 by 0x08008708: +0x00 the magic
    0x12345678, +0x04 the sender's RNG2, +0x08 BattleState +0x3C, +0x0C MegaMan's 0x40-byte NaviStats (+0x2A from event
    flag 0x1184 or the mode; +0x36 set to 500 when 0x08006570 says so), then 0x2C bytes from 0x02001610 (+0x4C) and
    0x02007230 (+0x78), 0x10 from the toolkit's +0x64 (+0xA4), the save's +0x20 and +0x24 (+0xB4, +0xB8), 8 bytes from
    0x02035CA0 (+0xBC). 0x080087A8 unpacks the received records (0x0203E390 side 0, 0x0203E490 side 1) into the
    NaviStats and seeds the battle's RNG2 from side 0's record (0x0203E394). **Done:** exe4-compat's
    `codec::LinkRecord` (the magic checked; the RNG2, the settings record and the NaviStats by their fields, the rest
    kept whole: nothing in the battle reads it past the unpack).

### 18.4 MegaMan, his weapons, the objects

24. **The move that cuts a recovery short.** The buster's and the charged shot's recoveries (0x080EB3D8, 0x080ECCCC)
    use 0x0800AD2A, 0x0800B4B0 and 0x0800C9BA with the move lag 0x0800C208, as the idle does (0x080EEC82).
    **Done:** 0x0800B4B0, the held direction and its panel, is `effects.steps` (EXE4's keys right, left, up, down; its
    confused keys, 0x0800B550, EXE6's swaps; the idle starting only a step that can go), which `held_direction` and
    the idle read; the flow/move
    recordings match. **Done** too: the move bug, NaviStats +0x0D (`effects.steps.bug`, the rules' stat `move_bug`,
    which the NaviCust's bugs write and a dark chip's or patch card's will): with no key held (or sliding) 0x0800B4B0
    steps by its keys (bits 0x10 to 0x80; 0 and 0xFF none), and at 0xFF the init confuses the navi for 720 ticks
    (0x0800D8B0). Open there: the confusion's visual at the start (item 59). The move lag (0x0800C208) is MegaMan's 4 for a player of param 0 or
    1, as the engine's; by the +0x25 column (12 to 8) for param 2, and 20 under event flag 0x1187: neither is a
    netbattle's.
25. **The buster bonus, element, weakness and souls.** EXE4's buster is Attack + 1 (0x0800CC2E) for every navi:
    `buster_bonus = 0`. MegaMan's element, attach points, the souls (0x080184F0 by soul: the weapons; the sprite's index
    plus the soul, 0x0800B90A), the emotion window's faces. Placeholders: `element = "null"`, no forms but `base`.
26. **The flash's and the held cannon's animation** is NaviStats +0x23 (the navi's number): MegaMan's 0, stated as a
    constant in weapons/buster, weapons/charged_shot and chips/cannon.
27. **Done: the projectile** (0x080CD354, 0x080CD3D4) is @exelib's EXE5 code with EXE4's rows, nothing in the shared
    code to port: its row (9 bytes at 0x080CD2A8) is the variant record (collision, hit modifier, element, spark,
    sprite, animation, the panel type its hit leaves); it never bursts or climbs, row 7's hit cracks the panel, and
    rows 10 and 11 set a status by the row's number (0x08013212: 0x22 and 0x32, `confuse_960_past_paralyze` and
    `blind_1200_past_paralyze`), which their variants state as `status`. Rows 10 and 11 are weapon routines 0x59 and
    0x5A's buster shots (0x0800D2B4, 0x0800D2DA: Attack + 1, the buster's action 8), the patch cards' (item 57).
28. **Done: the attack's +6 halfword** (AIAttack +6) is the hand's two bonuses on the chip (its two bonus rows, Atk+
    and the like: 0x0800D4E4), which the chip use keeps apart from the damage (+8) as EXE6's does:
    the engine's attack `extra`. EXE4's actions that add it (the cannon, 0x080EB984; 0x080ECBEA; the guard's counter,
    0x080ECD8C) add `me.extra`; the lab's chips/0x095-atkplus10/boost and 0x0cd-atkplus30/boost (Cannon) match. What
    the chip use adds to the damage itself (+8: 0x0800C47A for a charged chip, its sound 0x1BB, else 0x0800C4C4) is
    the souls' (item 74): the engine's form bonus goes into `extra` today, which an action that ignores `extra` would
    lose where EXE4's keeps it.
29. **Done: Cannon's family** and **what 0x0800BA66 does** as action 0x0B starts (group A). The engine's chip family
    is the icon family, EXE4's +0x07 (the families are each game's data: EXE4's rules' `elements.families`, by that
    byte: fire 0, aqua 1, elec 2, wood 3, recovery 4, plus 5, sword 6, invisible 7, break 8, summon 9, wind 10, metal
    11, null 12, program advance 14, the souls' chips 15, `special`; 13 none). Cannon's is 12, null; its +0x0B, 0x0B,
    is its action (`cannon/action`). Double Soul's table 0x08020008 reads the same byte as a soul's kind (gen_rules.py
    holds the families to it). 0x0800BA66 opens the counter window (§7).
30. **NaviCust, patch cards**: the compile and the cards apply after the save (rules/save). Tango's light save has
    neither, the lab's first batch's. **Done:** the NaviCust (§8: rules/navicust, the setup's `navicust_programs`,
    exe4-compat's from a recording's list, the replays comparing the stats it compiles) and the save import (§12); the
    lab's navicust/ recordings whose programs are ported match, and Tango's saves compile as their blocks say. Open:
    the patch cards (items 56 to 58); the waiting programs (item 53).

### 18.5 Roles and the custom screen

31. **The custom screen** is EXE4's own (§5: its states, no ADD button, the description on R is the selection's state
    0x18). **Done** but for Double Soul's button and window (group A): §5's "As ported" lists the rules that say it
    (`custom_screen`'s keys and moves, settle tick, picks, hand, status, hover, gauge, drawing; flow
    `custom_request`), checked on the lab's custom/ recordings, frame and sound.
32. **Roles not filled** (the content check lists them): every sound but `appear` and `custom_open`, the music but
    `link_battle`, the effects (deletion, recovery, the cut-in flash), sprites (charge glow, statuses), banners but
    `round_start`, `turn_start` and `win`, the kinds and actions EXE5's roles name. Name each EXE4 asset for the code
    that uses it (gen_content.py's BY_USE, with the address; tools/exe4/assetloads.py lists where each unnamed one is
    loaded) and fill the role. EXE4's content audit lists no problem with the pack (the custom screen's emblem is the
    window's own, §14). The banners, the faces (MegaMan's `forms`, so his faces are his base form's `mugshot`), the
    warning marker, the navis' sprites and MegaMan's souls' are in.
    **Filled so far** (group A, each by its code): the custom screen's sounds (§5), the gauge full 0x81, the low-HP
    alarm 0x82 (0x0801475E), a panel's crack 0x95 (0x08009AEC) and poison 0x124 (0x08009D68), the music (winner 0x1E,
    a special battle's 0x18, a netbattle's loser 0x19: 0x080071F8, 0x0800727C), the banners but the telops (final
    turn 0x10 at 0x08007094, draw 0x1C, the judge's win 0x14, loss 0x18 and its own 0x28, the Program Advance's 0x24
    and 0x34). Group B fills the fight's sounds and effects its replays reach. **Not EXE4's** (left unfilled, as EXE5
    leaves EXE6's): the telops; freeze, bubble and ice (no such status: group C), so `statuses.ice_freeze` and the
    sounds `freeze`, `bubble`, `bubble_pop` and the sprites `ice`, `bubble`; the damage word's confusion and blindness
    (its only status bit is paralysis: group C); battle mode 9's kinds, the dive ripple, the Crosses' and Beasts'
    actions; the scrap (DustCross) and the Cross window's cancel (`custom_scrap`, `custom_scrap_done`,
    `custom_cancel`). **Still to read:** the confusion and blindness visuals (EXE4 has both statuses, its table
    0x08018550: sprites `confusion`, `blindness`, `immobilized`, the sound `confusion`), `statuses.counter_paralysis`
    (with the counter hits, item 9), SearchSoul's shuffle sounds (`custom_redeal`, none at its start, 0x0802037A;
    `custom_redeal_shuffle`, 0x123 every 4 ticks, 0x080209E0: with the souls), the Full Synchro aura (item 8), and the
    roles of chips not ported yet (the counters, obstacles, eruption, the target marker), which come with the chips.

### 18.6 Found by the replays

What the lab's recordings showed past §18's first list (group B's replays, exe4-compat's `trace`). Each generalization
is a rule field every game states, EXE6's and EXE5's unchanged.

39. **The charge and its glow.** EXE4's navi init (0x0801079C) spawns no charge glow: the charge spawns its own
    (0x0800BD88: effect 5, 0x080E215C, when the AI data's +0x0A or +0x0B has a level and +0x58 holds none; it reads
    which charge (+9) and its level, sounds 0x71 as it starts and 0x72 at the full charge, sits at a per-navi offset,
    0x080E2274, and frees itself when the charge goes). The charge itself counts per source (+0x0A, +0x0B levels;
    +0x30, +0x32 counters; 0x0800BDAA clears them), not EXE6's one counter. **Done:** the init's part,
    `effects.charge_glow` (`with_navi`: EXE6, EXE5; `with_charge`: EXE4); the counting (item 47); the glow, the role
    `kinds.charge_glow` (content/exe4/objects/charge_glow), which the navi's tail spawns as the charge gets a level.
    Its per-navi offset is MegaMan's (none) alone: EXE4's content has no other navi. Unported: the tail's color
    shader before it (0x0800BB18: AIData +0x2C in 1 to 90, a shader from 0x0800BB40 by the battle timer). +0x2C is the
    dark soul's ticks (set at the soul change from the table 0x08018088 by soul, 0x0800B93C; set to 1 by 0x0800E256
    for a soul but 0 and 15): the souls' (item 74).
40. **The intro's first tick.** EXE4's intro (0x08007464) goes on from its init to the HUD's setup on the same tick
    (state [4,0,4,4] after the round's first tick). **Done:** `flow.intro_steps_on_init`. (Its init's one RNG2 draw is
    the actors' spawn's last call, 0x080F576C, where EXE6's is `sub_8014178`'s: the same draw.)
41. **The fade toward clear.** 0x08005BDE ends the fade on the step whose level would go under its target and keeps
    the last one, a step after EXE6's `sub_8006366` (the intro's fade takes 18 steps). **Done:** `effects.fade_clear`
    (the renderer's `Fade::intro_ticks` with it).
42. **A paused navi.** EXE4's status block (its tail, 0x08013C2A) has no pause handler: the navi runs while paused
    only until it takes control, which clears its header flag 0x04 (0x08010A88), and the object loop skips it then.
    **Done:** `status.paused_navi` (`pause_handler`: EXE6, EXE5; `stops_at_control`: EXE4).
43. **The banner's steps.** EXE4's banner task (0x08014904, 0x08014934; its steps from 0x08014994) slides in for 9
    ticks, holds 30, slides out for 9 (EXE6's 5, 0x30, 5); a holding banner (its table's state 2, 0x08016C04:
    banners 0x24, 0x28, 0x34) doesn't count its hold, and let go (0x0801616C) slides out at once. **Done:**
    `effects.banner`. Unread: a state 3 banner (shown without sliding), which no table entry has.
44. **The HUD's banner task** is the HUD task mask's bit 5 (0x20; EXE6's bit 15): exe4-compat's comparison reads it.
45. **A hit's sound.** EXE4's hit intake (0x08013A8C) plays 0x6B on every console, 0x6D for a navi in auto battle
    (NaviStats +0x26 = 1); EXE6's and EXE5's play `own_hit` to the hit navi's player and `hit` to the other. **Done:**
    `status.intake.hit_sound` (`by_console`: EXE6, EXE5; `by_navi`: EXE4; the role `auto_battle_hit`).
46. **RNG1 by the frame.** EXE4's main loop (0x080002B0) draws no RNG1 a frame (EXE6's `main_` and EXE5's draw one
    after the subsystem). **Done:** `effects.rng1_per_frame`.
47. **The buttons and the charge.** EXE4's decode (0x0800BDE0) and charge (0x0800BBA4, levels 0x0800BB50) are their own:
    no hold requests; B held charges B, A held with a chip in hand A (the other's press switches); a count goes on past
    full (to 510); the buster on B's release; B then Left (whatever the facing) within 8 ticks for the B+Left special;
    L and R are no navi's. **Done:** `effects.charge` (`hold_flags`: EXE6, EXE5; `per_button`: EXE4), the engine's one
    counter and level standing for EXE4's two (only the charging source's is ever nonzero).
48. **The souls' buttons** (with the souls, §18.4 item 25): in 0x0800BDE0 soul 2 asks for the charged shot (request
    0x20) at six B presses each within 10 ticks of the last (AIData +0x11, +0x12), soul 15 decodes neither the buster
    nor the B+Left special and charges nothing (0x0800BBB2); whether a chip charges on A is 0x0800BC78's test by soul and
    the chip's +7 and +9 (souls 5, 7, 9, 11), which the engine asks of the form's `charged_chips`.
49. **Controller 2** (NaviStats +0x26 = 2; 0x0800BF1C, 0x0800BCD4): Right and Left presses move a per-side column
    (0x0802E070's +0x11) and the B count runs only under request 0x80. Unreachable: nothing in EXE4 sets +0x26 to 2
    (its one setter, 0x080069BE, gives the story's navis 1; a netbattle's NaviStats are MegaMan's, the link record's,
    whose +0x26 his console never sets), so no setup can say it and the engine keeps no field for it (the minimal
    fields rule). Port it with a setup that can.
50. **Closed: the levels at the ask, kept by nothing.** The decode copies the charge levels to AIData +0x14 and +0x15
    as it asks for an attack (0x0800BE48, 0x0800BE8E, 0x0800BF10), levels from the tick before (0x0800BB50 runs after
    the decode and the charge: 0, 1 charging, 3 full). Nothing reads A's (+0x14). B's (+0x15) is read by the dimming
    chips' variants 5 and 7 alone: GutsMan's controller (0x080E328A) and AquaMan's (0x080E39FA) take it off the side's
    AI data (0x0800BFF4) and hand their navi `full << 8 | Param1` (r4: 1 in the high byte when B was full, 3) as his
    Param1 and Param2; neither navi ever reads Param2 (his level is Param1 alone), and nothing else in EXE4 reads
    +0x15. So the engine keeps no field for it (the minimal fields rule): the copies and the controllers' read are
    stores nothing observes. (An engine field for it, `b_charge_at_ask`, came and went in this port.)
51. **A player's deletion.** EXE4's (action 2, 0x08010850) is its own, not EXE6's `sub_80173F4`: the hurt animation
    as it starts (EXE6's with the explosions); the alive count alone (0x080079C6), never the alive lists; no chip count,
    charge glow link or tracking let go; its second related and barrier byte at the start, AIData +0x60 with the
    explosions; no aura or overlay links; no death hook at the end. **Done:** the role `actions.deletion`
    (content/exe4/rules/deletion.luau; the engine's own where the role is unfilled: EXE6, EXE5); the flow/ko recordings
    match through the deletion. Open: the deletion of a navi in auto battle or of another navi (0x08010D4C: effect object
    0x11 and a 90-tick explosion), which EXE4's content doesn't reach (its players are MegaMan, by a player).
52. **The dead player's object.** EXE4's destroy state (0x0801052C) lets go of the collision data, frees the object and
    counts one actor fewer at once, its reservations left as they are; EXE6's (`sub_8016C4E`) keeps the object in its
    slot. **Done:** `reactions.dead_player` (`kept`: EXE6, EXE5; `freed`: EXE4). Unreachable: 0x0801052C's branch for a
    player of param 2 (an owner's count, at the object's +0x78, one less): the engine spawns its players itself, by
    side, with no param or owner, and no content spawns one.
53. **FirstBarrier, HubBatc, Shield, Reflect, AntiMagc.** FstBarr's handler sets the aura at the start (+0x21 = 1, the
    Barrier chip's barrier: the init raises it, 0x0800D894, 0x08012DCE, its visual 0x080E2622), which the patch cards'
    Barrier100, Barrier200 and LifeAura set too (2, 3, 6); Shield, Reflect and AntiMagc set B+Left (+0x0C = 0x25,
    0x26, 0x27: weapon routines of the table at 0x0800CA7C); HubBatc runs FstBarr's and Shield's handlers among its
    eight. Until they are ported the five programs have no definition (tools/exe4/gen_navicust.py's `WAITING`), and the
    recordings that carry them stop at their setup. EXE4's barrier tick (0x08012DF8) is EXE6's `sub_801A802` but that
    it runs while paused (whenever its navi does) and that wind takes the barrier away at once (no popped barrier, no
    hit flag does it): **done**, `status.intake.barrier` (`stops_while_paused`, `wind`: EXE6 and EXE5 `true`, `pops`;
    EXE4 `false`, `takes_away`). Its types (0x080185E0: HP, threshold, timer, as halfwords): 1 10 HP, 2 100, 3 200, 4
    150 regrowing 180 ticks after it breaks, 5 1 HP, 6 a 200 threshold for 3000 ticks, 7 a 300 threshold for 3000
    ticks; type 4 is BlakBarr's (done: the barrier chips), type 7 (and a threshold over 255) waits on a chip that
    raises it. **Done:** FstBarr: EXE4's
    barriers (content/exe4/lib/barriers: types 1, 2, 3 and 6, the patch cards' auras' too) and their visual (effect 7,
    content/exe4/objects/barrier_visual: EXE6's but no blown action and no sound going down, its sprite updated paused
    or not, running while paused until the fight starts, placed by per-navi offsets), raised by the roles' hook
    `first_barrier` at the init (0x0800D892); exe4-compat names the aura types (records.toml). The navi's link to the
    visual is EXE4's object +0x50 (the engine's `barrier_visual`: 0x08012DEC clears it with the barrier, as the
    deletion does); EXE4's AIData +0x60 is the Full Synchro aura's link. The lab's navicust/firstbarrier matches.
    **Done** too: Shield and Reflect, the B+Left weapons 0x25 and 0x26 (content/exe4/navis/megaman/weapons/guards):
    EXE4's guard (action 0x25, 0x080ECCF2: content/exe4/lib/guard; the shield, attack 0x2B, 0x080D2224:
    content/exe4/objects/shield, EXE6's but one sheet, 14 fade ticks for every row, running while dimmed and not while
    paused, animating then too, and no test of its owner's having vanished), Shield countering nothing, Reflect the
    charged shot's projectile; Reflect's routine sets the B+Left cooldown as it starts (`reactions.attack_end_lockout`)
    and a turned-aside hit marks the guard byte with 1 (`reactions.hit_test.guard_marks_direction`); the guard's sound
    (0x6E) is the roles' `sounds.guard`. HubBatc has its definition. The lab's navicust/shield, reflect and hubbatc
    match, sounds too. **Done** (group B): AntiMagc (routine 0x27, guards' `anti_magic`: its stance, action 0x72 at
    0x080EE9EE, makes AntiDmg its side's defensive chip for 13 ticks, 0x08022FDE with chip 0x91, and a catch hands over
    to the trap's counter, 0x0800C780); the lab's stance/antimagc-* match and draw as mGBA's. **Done** (group B): the
    Guard chips' variants 0 to 2 of the guard counter with a shock wave (chips/guard: 0x080CFD2A, its row 6; each user of
    lib/guard states its own counter). EXE4's guard spark (object_spawnHiteffect, 0x0800B0F6) doesn't stop while paused, where the engine's does: a guard
    is never turned aside while paused in a netbattle (nothing hits then).
54. **Done: Rush, Beat and Tango in battle.** The supports compile (+0x18 = 1, 2, 4; the support bug 0xFF), which
    exe4-compat maps to the stats' supports as EXE6's +0x0D. Their triggers are EXE6's (the engine's): Tango at a
    quarter of the navi's HP (0x0800C7D8, in the idle after the reactive chips: EXE6's `sub_8010660`), Beat then Rush as
    the opponent uses a chip (0x0800C838, 0x0800C8C0: `sub_80106C0`, `sub_8010740`), each once, in a battle of type 0x46
    or more; EXE4's Beat lets a dark chip go (0x0800C86C tests the record's flag 0x20: the rules'
    `chip_use.beat_spares_dark`). Their controller (effect object #0x74, 0x080EA2BC), spawned by 0x080EA3D6 with a
    dimming no one can cut in on, is @exelib/supports/controller with EXE4's warp (0x080E2D26, by its side); Rush (actor
    #0x4B, 0x080CA4C4), Beat (#0x4C, 0x080CA76C), Tango (#0x4D, 0x080CA9B4) and Tango's heal (attack #0x9F, 0x080E0CD4)
    are EXE6's code (@exelib/supports) in EXE4's looks (content/exe4/lib/supports), but for: Rush and Tango add no pixel
    where EXE6's do (0x080CA60C, 0x080CAB88), Rush's appearance panel reads its table's forbidden flags two bytes in
    (0x080CA70A: side 0's 0x1, side 1's 0x00200001), the effect rows one further on (21, 22: EXE4's table has a row
    more), and the heal's 100-HP barrier is EXE4's type 2 (its old visual ending first, 0x080E264A). The chips the
    telops name, 0x17F to 0x181 (chips/supports), are dimming chips of the variant 0x42, the same spawner, which no
    folder holds. The lab's navicust/beat-takes, beat-once, beat-spares-dark and tango-heals (and navicust/rush, beat,
    tango and bug-rush, where none comes) and rush-eats-invis and rush-eats-popup (Invis and PopUp, 0x085 and 0x086,
    the two chips Rush cancels: their records' +0x16 bit 0x02, `extra_flags`) replay every frame and compare every
    fight frame; modcards/074-triple-supporter waits for its patch card.
55. **The emotion window's bug flicker.** EXE6's (`sub_801CC94`) and EXE5's (0x08019780) emotion window flicker a
    bugged navi's face at their checks, an RNG1 draw each; EXE4 has no such check (the lab's navicust/bug-* recordings'
    RNG1 never moves in the fight). **Done:** `effects.bug_flicker` (EXE6, EXE5: true; EXE4: false).
56. **The patch cards** (Tango's patch_cards.rs: 134 cards, their effects by NaviStats offset and value, their bugs by
    group): the cards' content, their handlers (0x08041E8C), the setup's `patch_cards`, exe4-compat's from a recording's
    slots (a recording with a card on stops at its setup until then). **Done** (§8): tools/exe4/gen_patch_cards.py
    (verify) decodes the four ROMs' handlers; content/exe4/patch_cards (the 61 cards whose effects EXE4's content can
    do), rules/patch_cards (their constructors and their part of the reload), the setup's `patch_cards` (a card and its
    slot), exe4-compat's numbers (compat/patch-cards.toml), a recording's slots and a save's into the setup; the lab's
    modcards/ recordings whose cards are ported compile as recorded (exe4_navicust) and replay. The cards that wait,
    each with what it waits on (a save holding one says so and leaves it out): the B button's, B charge's and B+Left's
    chips (item 57; card 106's B button is Reflect, ported); the souls (item 58); the pairs, each half applying when the
    other sits in its slot (0x08042504): Triple Supporter (74, 75: +0x18 = 7, item 54); All Guard (76, 77: +0x28, item
    60) and Charge FullCustom (103, 104: the charged shot FullCustom, item 57) are in.
57. **The ~50 patch cards that set B, B charge or B+Left to a chip** (+0x09, +0x0A, +0x0C = a weapon routine past the
    buster's): each waits on its chip as a weapon routine; the chips' work picks them up. **Partly done:** a routine
    that loads a chip (0x0800D406 with its id, EXE5's 0x0800FE78) is navis/megaman/weapons/chips's weapon of the chip
    (`navi:load_chip_attack`, the charge table's row), compat/records.toml's `[weapons]` number and gen_patch_cards.py's
    `WEAPONS`; the 32 whose chips exist are in (with the taunt's card, 107, and the pairs' Charge FullCustom), their
    modcards/ recordings matching. A pair's half (`cards.pair`: 0x08042758) holds the rest of its handler, which runs
    only while the other half sits in its slot and the stat its first effect sets doesn't hold the value yet (All
    Guard's too: item 60). Waiting:
    the routines of chips still to port (0x54 NrthWind; 0x37 CopyDmg, 0x42 Hole, 0x44 SandRing, 0x4E WindRack, 0x51
    BugBomb and 0x55 PnlRetrn are in, cards 41, 51, 57, 70, 79, 82, 115 and 118 matching),
    the routines that load no chip (0x04: others' own actions; 0x20, the bubble, is in: card 108; 0x28, GutsSoul's
    machine gun with no attack variable set, is in: card 20, navis/megaman/weapons/machine_gun; 0x31, one of the eight
    Anti chips at random with its telop hidden from its user too (AIAttackVars +0x0F: the attack's `telop_hidden`), is
    in: card 40; the
    buster patches 0x34, 0x35, 0x5A, 0x68 and 0x69, navis/megaman/weapons/buster_patches, are in: cards 12, 13, 43, 44
    and 102, the buster's shot with projectile rows 7, 8, 0x0B, 0x0D and 0x0E: cracking, poison, blinding (row 0x0B's
    status 0x32, 0x080CD3BC; row 0x0A's 0x22 no routine fires), grass and ice) and Triple Supporter's pair (item
    54).
58. **The 12 soul patch cards** (+0x24: a battle starts in the soul) wait on the souls (item 25).
59. **Done (group A): the status timers while paused, and the status visual** (`effects.status_visual`: EXE4's at
    the identity's `status_mark`; item 110 the timers). EXE4's status timers (0x0800AE58: paralysis +0x10,
    confusion +0x12, blindness +0x14, immobilization +0x16; no freeze or bubble) don't stop while the battle is paused
    (EXE6's `sub_800E730` and EXE5's 0x0800CB50 return), so a navi confused at the start (the move bug, item 24) shows
    the confusion's visual on the round's first tick, during the intro. The visual is effect 6 (0x080E22C8, EXE6's
    `sub_80E08FC`): its sprites by row (0x080E22B8: 14-0B confusion, 14-09 blindness), its sound 0xAE every 60 ticks
    (EXE6's 0x88), its place the owner's position and a per-navi offset (0x08011878's +6, +7), spawned at the status
    routine's registers (0x080E23B2). The lab's navicust/bug-humor and bug-undersht stop on its first tick.
60. **The patch cards' own bytes.** Card 45 sets NaviStats +0x1F, the Full Synchro at the start, which the reload's
    reset doesn't keep (0x08036CC0): **done**, the rules' stat `full_synchro_start`, which the card writes and
    rules/light_dark's starting mood reads (no setup fact). Cards 59, 60, 89 and 90 set +0x27, MegaMan's color (1 to 4),
    which the reset clears too: **done**, the rules' stat `color`, which the cards write and rules/light_dark's palette
    reads (5 more a step of it, 0x0800C03A, but in a soul other than 15); the lab's modcards/059, 060, 089 and 090.
    Cards 76 and 77 (All Guard) set +0x28, which the navi's init reads (0x0800D8E4: at 1 its guard flag, f1 0x1, is up
    from the start, so it turns aside every hit that doesn't break guards, 0x08012B84): **done**, the rules' stat
    `all_guard`, which the pair's half writes (`cards.pair`, item 57) and the role hook `abilities_reset` reads: the
    engine's `sub_801390C` (EXE4's 0x0800D8C2, the init's and a form change's) takes the guard down, the hook raises it
    again; the lab's modcards/076.
61. **Done: the idle stands the navi** (from AirShot's replays). MegaMan's idle (0x080EEB38) puts animation 0 on each
    tick past its first phase (0x080EEB7C: 0x080EEBAC), the 10 ticks after a reaction's end; EXE6's (`sub_80F0354`)
    and EXE5's (0x080F0254) leave the pose. A drag that keeps its pose (`status.drag`'s `keeps_pose`, 0x08010C16)
    shows 1 for those ticks and no more (`chips/0x004-airshot/hit`, `side1`: frame 431). The rule
    `status.idle_stands` (EXE4 true; EXE6, EXE5 false).
62. **The sprite draw's caps: tiles, palette slots and the transfer queue's 96** (low priority: netbattles rarely
    reach them). The sprite draw (IWRAM 0x03005C00; EXE4's IWRAM code is the boot's copy of ROM 0x08212700 to
    0x03005800, gba.py's `iwram`) runs in draw order and has three caps the renderer models none of (it draws each
    frame's tiles and palette as they are):
    - **Tiles.** A sprite reserves VRAM tiles for its frame each frame (0x03005D80, 0x03005DA0), a frame some sprite
      already reserved this frame shared through a cache (count 0x02009E64, entries 0x0200AE50, both cleared by
      0x080029E0; the allocator's halfword at 0x02010B90 reset to 1 by 0x0800294C). Past the cap (0x03005FD4: the
      halfword less 0x30 plus the frame's tiles against 0x32F, 0x2FF in mode 8: the byte at [sl]'s first word) the
      sprite is flagged 0x10 and not drawn, its last uploaded frame (+0x24) cleared. Its tile index (+8) changing
      clears +0x24 too, so the frame is copied again. EXE5's (IWRAM 0x0300656C, its code ROM 0x081C7A00 to
      0x03005C00) and EXE6's (0x03006404, ROM 0x081D6000 to 0x03005B00) do the same against a variable cap, the
      halfword at 0x0200A948+2 and 0x020098A8+2.
    - **Palette slots** (0x03005CE0): a sprite with its own palette (+6) shares the slot of a sprite with the same
      32 bytes or takes a new one (count 0x02010B80): 12 at most, 10 in mode 8 (0x03006010), past which it is
      flagged 0x10 and not drawn (a count already at 15 would give it slot 15, drawn). EXE5's (0x030062AE,
      0x03006590) and EXE6's (0x03006146, 0x03006428) read their limit from the byte at 0x0200A948 and 0x020098A8.
      Their presets set both caps (and the byte at +1): EXE5's 0x08002730 (12, 0x32F), 0x08002740 (10, 0x2FF),
      0x08002750 (8, 0x2FF); EXE6's 0x080027D4 (10, 0x2FF), 0x080027E4 (8, 0x2FF), 0x080027F4 (16, 0x2FF). Which a
      battle sets is the port's to read (EXE4's: mode 8's 10 and 0x2FF, else 12 and 0x32F).
    - **The transfer queue** (0x0800087C, 96 entries at 0x0200D120, count 0x0200B134; drained after the VBlank wait,
      0x08000808 at 0x080002C8): a sprite whose frame differs from its last uploaded one (+0x24) queues its tiles'
      copy there, and +0x24 advances whether or not the copy was queued. The queue is the game's one graphics
      transfer queue (bn6f's QueueEightWordAlignedGFXTransfer), with some 150 callers besides the sprite draw (the
      HUD, text, icons, backgrounds), so its 96 is a budget for every transfer of the frame. EXE5's (0x080009E8,
      count 0x0200B8AC) and EXE6's (0x08000AC8, count 0x0200AC1C) hold 96 too. A dropped copy leaves the
      tiles VRAM held at the sprite's tile index, which may be another sprite's (the index moves when an earlier
      sprite's count changes) until its frame changes again.
    Open (group C, 2026-10-07): no recording reaches any of them; the lab's busiest fights (two navis and a field of
    bursts) reserve about 200 tiles and a few palettes. To port (group C, after its chips): the tile and palette caps
    as per-frame budgets in the renderer's draw order, each game's numbers in its rules, frame-compared with a
    scenario built to reach them. The queue stays open: it needs a model of sprite VRAM and of every transfer the
    game queues. (What the towers' frame
    compares show, a tower a tick behind or split at a scanline on a few ticks, is the drain running into the
    display, tearing, which is left.)

70. **The souls' parts of the chip families.** ProtoSoul (soul 7) swings blade 13 and takes the swords' hit modifiers
    from 0x080EB7FE (lib/swords); a GigaCan's afterimages add the soul's part by soul (0x08018068, lib/cannon). Wire
    them with the souls (item 25).
71. **The telops' two blocks.** EXE4 lays the user's telop on the banner block (0x08016454, HUD task 0x100) and the
    other player's on the second block (0x080164B4, task 0x8000), each in the banners' steps; the engine runs both on
    its one banner, with no banner of their own (the roles leave `telop` and `telop_remote` unfilled; the HUD draws
    the telop in its own look), which is the same while no other banner shows.
    What differs: a banner on the banner block and the other player's telop at once, and a telop that starts while
    one shows (0x08016454 restarts it; the engine's start fails). exe4-compat compares a telop by its own task bit.
    The telop plays no sound (the role `sounds.telop` is optional).
72. **The selection's end under the harness.** The custom HP drain (0x0800C194) counts while value 1 of the side's
    status is set, which OK clears (0x08020652); exe4-compat feeds the OK `link_delay` frames late (§17), so the
    engine's selection runs 4 ticks longer than the original's and a drain can land in them (dark/drkrecov at frame
    1566, dark/drklance at 917: the screen after the chip). The hover's music fades after OK shift the same way (the
    sound comparison counts those as the shift's: item 75). A harness question (raised with the coordinator), not
    the engine's.
73. **Done: Double Soul** (group A; §5's Double Soul part, §6): the UNITE button, its offer and the soul's choice,
    OK's soul chip, the change (action 0x0D) and the revert (0x11) as the fight runs, the turns, the setup's souls and
    Double Soul (no version in the content: exe4-compat maps a recording's version and a save's event flags to the
    souls; the app's presets name each version's six).
    Checked: the lab's souls/roll, guts and proto (unison, side1, turns) match every state through the change, and
    stop at the souls' charged shots (item 74); souls/refused's refusal. Frame comparisons (souls/roll, guts and proto
    unison, souls/refused): the UNITE button, the soul's choice and every custom screen frame exact but the late OK's
    known shift; the change exact, its HUD too (the chip name through it, the soul chip's icon left out, the emotion
    window's soul face and its turns), to the charged shots. The revert is checked by state only once the charged
    shots let turns recordings run to it (item 74).
74. **The souls' abilities** (group A; with item 25's weapons and item 48's buttons). Every read of the soul
    (NaviStats +0x24: the 49 calls of the byte getters 0x0800D7CA and 0x0800D772 with 0x24) and every table by soul,
    one line an ability; each gets a lab scenario of its own (`gen_exe4.SOUL_ABILITIES`, souls/<soul>/<ability>) and a
    frame comparison. *done* is ported with its recording matched, *ported* without its own recording yet, *open* to
    port.
   - **Every soul.** Its weapons, 0x080184F0's row (0x0800BD48: AIData +0x0D the B routine, +0x0E the A charge, +0x0F
     the charged shot) *done*; its turns, 0x080180A8 (0x0800BA3C), and the revert at their end (0x0800B658) *done*;
     MegaMan's sprite, his entry plus the soul (0x0800B90A, 0x0801079C) *done*; the emotion 4 (0x0800F49C), no anger
     (0x0800C560) and no Full Synchro aura (0x0800D98A) *done*; the palette 0 (0x0800BFFE) *done*; the status reset's
     routine by soul (0x0800E05C, table 0x0800E0A0), its element by soul (0x0800C93C, table 0x0800C964: Fire fire,
     Thunder elec, Aqua aqua, Wood wood, the rest null) *open*; the overlay by soul (0x0800B7E8, table 0x08018068: Fire
     0x10, Number 0x110; `effects.form_overlay`; PopUp's glow, objects/glow, puts its copy on through the form's
     `put_on` and must have it step on, its Param3 1: 0x0800B812) *open*; the image's part (0x080E3674, table
     0x080E36AC: WindMan 0x34, FireMan 0x12, NumberMan 0x14, AquaMan 0x13; forms/soul's `image(sprite, part)`)
     *done* for WindMan, *open* for the rest; a chip charging on A (0x0800BBA4, the test 0x0800BC78; forms'
     `charged_chips`) *open*; a charged chip's double (0x0800C47A: the damage's 0x8000 and sound 0x1BB, into the damage
     itself, item 28) *open*; MegaMan's look by soul in what copies it (the
     image 0x080E2C44, the swords' and cannons' afterimages 0x080EB548, 0x080EB5FA, 0x080EB874, the GigaCan's part
     0x080EB93A, the objects 0x080CB4B0, 0x080CC648, 0x080CC750, 0x080CC960, 0x080CCAF0, 0x080CCF7C, 0x080D7590,
     0x080DD7BC, 0x080EA744) *open*: each with the chip that spawns it.
   - **RollSoul (1).** Roll's arrow charged (routine 0x22, 0x0800CF20) *done* (souls/roll/hit); a chip used from idle
     that isn't a recovery chip heals a tenth of the maximum HP (0x080EED04, the form's `chip_used`) *done*
     (souls/roll/heal).
   - **GutsSoul (2).** GutsMan's fist charged (0x24, 0x0800CF62) *done* (souls/guts/hit); six B presses within 10
     ticks of each other ask for the machine gun (0x0800BE50, action 0x10) *done* (souls/guts/rapid); the status
     reset's 30 more on each damaging Null and Break chip of the hand (0x0800E1CA) *done* (souls/guts/bonus).
   - **WindSoul (3).** The change's north wind (0x080EBAD2, attack 0x5A at 0x080D8C94), which takes a barrier away
     (wind: the raw hit flags' 0x20, 0x08012E0C) *done* (souls/wind/barrier); the status reset (0x0800E0F4): float
     and air shoes (flag1 0x20, 0x10) *done* (souls/wind/cracked, broken), a floating navi's body (row 0x10), its wind
     (0x080E68F8, held at AIData +0x5C; none beside a Fan's) *done* (souls/wind/wind, fan), 10 more on each damaging
     Wind chip (0x0800E208) *done* (souls/wind/chips); the image's part (row 0x34) *done* (souls/wind/hit); B's AirShot
     (0x6B, 0x0800D3E8: variant 1, a 30-tick recovery) and WindRack charged (0x09, 0x0800D204) *done* (souls/wind/side1,
     unison).
   - **SearchSoul (4).** The status reset's reveal (effect 0x44: 0x080E6E4C, 0x080E6D74): the scope's mark (effect
     row 0x52, 10-23) over the other side's navi when invisible, submerged or flashing (flag1 0x206; a netbattle's
     one slot, `sub_800A832` 70 and on), then on its next update a hit over the field for no damage through
     invisibility (0x2C05FF80: row 0x2C at navis), running while dimmed (0x080CD81E) *done* (souls/search/reveal);
     its scope charged (0x0A, 0x0800CDC4; action 0x3E, 0x080EDB5C: 0x0800FB3C's find, five shots, the first through
     invisibility, row 0x19, the rest row 0x0A, the last flinching) *done* (souls/search/scope, unison, side1).
     Presentation, open: in souls/search/side1 (frames 491 to 498) mGBA draws the opponent's HP number in front of
     the spiraling image, the engine behind it; the RAM's sprite list (0x03002000 at that tick) has the image's parts
     (80 to 84) at priority 0 before the number's 32x16 (85, priority 1), which mGBA's rule (priority, then index)
     would put behind the image: what the hardware takes is to find.
   - **FireSoul (5).** Fire (0x0800C964) *done* (souls/fire/weakness: aqua on it, the weakness mark at its status
     mark, 0x080133E8); its overlay 0x10 (10-00: forms/soul's `overlay`) and image part 0x12 (10-08) *done*
     (souls/fire/hit, unison); the status reset's panels (0x0800E11C: effect 0x12 in mode 3, 0x080E3552: the middle
     row and four more panels turn grass, blinking 10 ticks; its modes 0 to 2 have no spawner) *done* (souls/fire/hit);
     a fire chip charges on A (0x0800BC78; the A routine 0x66 has charge times alone) and its charged use is a 150
     flame in its place (0x0800B76C, 0x0800D638: action 0x19, hit 0xA8, five panels, the attack's chip 0: chips/zeroed)
     *done* (souls/fire/chip); the flame charged (0x0C, 0x0800CCAE: 50, hit 0x9E, three panels, action 0x19) *done*
     (souls/fire/unison); lava heals it 50, as raw fire damage (a barrier takes it), and turns normal, with the heal's
     sparkle and sound (0x0801313E, 0x080131B8: rules/panels) *done* (souls/fire/lava, lava-barrier).
   - **ThunderSoul (6).** Elec (0x0800C964) *open*; an uncharged damaging Null or Elec chip that isn't a dimming chip
     paralyzes (0x0800C4C4: the damage's 0x4000) *open*; charged 0x58 (0x0800D298) *open*.
   - **ProtoSoul (7).** The status reset's B+Left special 0x6A (0x0800E12C, 0x0800D3C0) *open*; a sword chip that
     isn't a dimming chip charges on A (0x0F) and doubles charged (0x0800C47A) *open*; WideSwrd charged (0x3E,
     0x0800D148) *open*; the swords' blade 13 (0x080EB628, 0x080EB6AC, 0x080ED89C, 0x080EE822) and their hit modifiers
     from 0x080EB7FE (0x080EB6E8, item 70) *open*; VarSwrd's and NeoVari's command window waits out its ticks without
     A held (0x080ED776, 0x080EE240) *ported* (chips/varswrd's `waits`).
   - **NumberSoul (8).** Its overlay 0x110 and image part 0x14 *open*; the status reset's 10 more on each damaging
     Null chip (0x0800E194) *open*; charged 0x11 (0x0800CE08) *open*; the hand of ten (0x0801DC92) *open*.
   - **MetalSoul (9).** B's routine 0x0B (0x0800CDE2) *open*; a damaging Metal or Null chip that isn't a dimming chip
     charges on A (0x0D) and doubles charged (0x0800C47A) *open*; charged 0x13 (0x0800CE28) *open*.
   - **JunkSoul (10).** No shadow (0x0801079C and the look copies) *open*; the status reset's hitbox on his panel
     (0x0800E146: params 0x1705FF85 or 0x1705FF84 by side, 0x2100) *open*; charged 0x61 (0x0800CE4C) *open*.
   - **AquaSoul (11).** Aqua (0x0800C964) *open*; its image part 0x13 *open*; an aqua chip that isn't a dimming chip
     charges on A (0x15) and doubles charged (0x0800C47A) *open*; charged 0x23 (0x0800CF42) *open*.
   - **WoodSoul (12).** Wood (0x0800C964) *open*; the status reset's merge (0x0800E294: a Wood chip takes the damage
     and bonuses of a damaging Null chip after it, which leaves the hand) *open*; grass heals 1 (0x08012FD6) *open*;
     a hit's status doesn't take (0x08013904 skips 0x08013470: `status_immune`) *open*; charged 0x57 (0x0800D270)
     *open*.
   - **The dark soul (15).** A worn-out MegaMan (emotion 5) at 0 HP, once a battle, holds at 1 HP and asks for it
     (0x0800EBC8, 0x08013AB8: request 0x1000, then 0x0800B8CA: soul 15); its status reset (0x0800E17E: the mood 0,
     flag1 0x08000000, 0x0800EAC6); 960 ticks (0x0800B93C, 0x08018088) counted down to the revert (0x0800B6A4) with
     the tail's shader its last 90 (0x0800BB18, item 39); its collision (0x0800C43C: +0x18 3, flag1 2); no buster,
     B+Left or charge (0x0800BBB2, 0x0800BE28); its chip use (0x080EEC78: 0x0800E4F8), no hand advance after the change
     (0x080EBC8E), GunSol without A held (0x080EDF12) *open*.
75. **Done: DS chips' code and damage on the custom screen** (group A; F's frame comparisons, chips/0x0df-rollds and
    the other DS chips, frames 269 to 283). The DS scenarios' MegaMan is dark, so the screen offers dark chips and the
    cursor starts on one: the hover (0x0801E478) darkens the screen, its window fade (0x0801E520's second slot)
    palettes 9 to 13. The chip window draws a chip's code and damage in palette 14, past that range; the frontend
    faded the whole HUD layer. Now a HUD pixel drawn in background palette 14 or 15 takes only the fades of every
    palette (the flash's, the transformation's: `Fades::hud_past_ranged`), the text layer's items alike. The DS
    chips' and ElemDark's recordings match every frame but the late OK's known shift.
    The music's duck at frame 290 that only the engine made (F's sound lab, every DS chip recording and ElemDark's:
    players 0x1F and 9 at 0xC0 and 0x60) is the replay's shift (item 72): the original's hover runs only in the
    choosing state (0x0801E40E), so its OK at 288 stops the ramp a step short, where the engine's screen chooses on
    until the replay feeds the OK and takes that step. Verify's sound comparison counts such a step as the shift's
    (`Compared::shifted`, the held-back frames' `Shift`).
76. **Done: a telop leaves the other player's used-chip name** (group A; B's, item 90; chips/0x08e-antielec/sprung:
    mGBA shows "Thunder 140" until frame 615 beside the AntiElec telop). EXE4's dimming telop starts (0x08008CF6,
    0x08008DE0) take the banner block (0x08016454) and leave the second block alone; EXE6's (`sub_800BA8A`,
    `sub_800BBA8`) and EXE5's (0x0800A0FC, 0x0800A218) end the names as the telop starts (`sub_801BED6(0x10000)`,
    `sub_801DACC(0x10000)`). The rule `effects.telop_ends_used_chips`: EXE4's false. AntiElec's sprung recording
    matches every frame but the late OK's known shift.
77. **Done: the chips' own looks out of the shared libraries** (group A; the brief's no-lookup-tables rule):
    lib/cannon's `arms`, lib/swords' `effects`, chips/vulcan's `guns.dark` and objects/projectile's `variants` are
    gone: a maker each, every chip stating its own.
90. **Done: the Anti chips' springs** (group B, chips/anti). AntiNavi springs as EXE5's (0x08008F80, EXE5's
    `sub_800BDD0` but for its test: EXE4 springs on any chip whose step reaches it, the navi chips', with no block of
    chip numbers): the roles `effects.trap_mark` and `sounds.cut_in` (the sparkle 0x0800815C: effect row 0x48, sound
    0x10D). AntiDmg's and AntiSwrd's catch (0x08023048) is EXE5's, not EXE6's (`status.intake.anti_traps`: AntiDmg
    any damage, AntiSwrd any sword hit), and its counter runs from the next tick (0x0800C780, a stance's catch and a
    trap's alike: `reactions.trap_counter`). AntiDmg's counter (action 0x38, @exelib/antidmg/counter) keeps the navi's
    move, throws at a random one of the other player navi's panels whatever the variant, and dives BodyGrd's shuriken
    (attack 0x5C, @exelib/bodygrd/shuriken); AntiSwrd's (action 0x39, @exelib/antiswrd/counter, plain as EXE5's) holds
    EXE4's own blade and throws booms (attack 0x58, objects/sonic_boom) all of hit modifier 1. AntiRecv's
    counterattack (effect 0x2C, @exelib/antirecv/controller) takes no mood (the role `kinds.anti_recovery`). AntiWood's
    spring on WoodPwdr (the lab's chips/0x08f-antiwood/sprung) matches, AntiMagc's on its stance (weapon routine
    0x27) too.
    A telop doesn't take the other player's used-chip name off the screen in EXE4 (item 71: the user's telop goes on
    the banner block, the name stays on the second; EXE6's `sub_801BED6(0x10000)` clears it): the lab's
    chips/0x08e-antielec/sprung shows Thunder's name 12 more frames (604 to 615) than the engine, and
    chips/0x08f-antiwood/sprung WoodPwdr's 7 (609 to 615) (group A's).
110. **Done: status timers through a pause** (group C, from B's NaviCust replays). EXE4's status timers (0x0800AE58,
    EXE6's `sub_800E730`: paralysis +0x10 and the rest) have no pause test, where EXE6's and EXE5's (0x0800CB50)
    return while paused: a navi's run before it takes control (it runs paused until then, item 42's
    `stops_at_control`) counts them, so the move bug's start confusion (NaviStats +0x0D = 0xFF) spawns its visual on
    the intro's first tick (`navicust/bug-humor`, `bug-undersht`). The rule `status.timers_while_paused` (EXE4 true;
    EXE6, EXE5 false); the visual's own EXE4 differences (its sound, offset and role) are group A's.

### 18.7 The HUD

The fight HUD as read and ported is §14's "The fight HUD as ported". What it still waits on:

130. **HUD pieces that wait on content** (group E). Port each with what reaches it:
    - a soul chip (0x160 to 0x16F) as the next chip shows no icon of its own, and the stack starts at the second
      (0x08015000), with Double Soul's chips;
    - a message stops "????" until the next custom screen starts it (0x08015FE8 stops draw bit 10), with the
      defensive chips;
    - a dimming chip's telop: the user's on the banner block (0x08016454), the other player's on the second block in
      state 0 (0x08008C40), both sliding and squashing as banners do. The renderer draws them from Hud.layout.telop;
      the engine's telop for EXE4 comes with the first dimming chip;
    - **Done** (§18 item 16): the turn timer from the 15th turn of a link battle (0x08007E4E): the fight's +0x0A,
      counted by 0x08008066, its seconds over "CUSTOM" (0x08016362, draw 7), the gauge not drawn; the damage judge's
      numbers (0x0801642C, 0x08016408) while its banner holds. The lab's timer/ recordings compare every frame;
    - the warning marker (0x0800843E: a 16x16 sprite at tile 0x360, its second frame at bit 3 of the frame counter, in
      palette 13; 0x08008424 also sounds 0x79 every 16 frames), with the chips that show it: the gauge chips'
      effect over the gauge at (120, 12) on the other console (0x080E3FAE), and 0x080E789E, 0x080E88F6, 0x080E8918.
      The renderer's (`warning_parts`) is EXE6's `sub_800AE90`, which leaves out a place near the screen's edge;
      EXE4's draws it wherever. Compare it with a recording once one of those chips is ported.

### 18.8 The chips (group F)

150. **A navi chip's user hidden and shown** (from Roll's warps). EXE4's 0x080E2D56 and 0x080E2DCC (EXE6's
    `sub_80E1352`, `sub_80E13DC`, similar) set no "vanished" mark (EXE6's `sub_8010312(0x100000)`), find the barrier
    visual at the object's +0x50, hide the HUD's chip icons by task 8, and show the user back whatever the viewer's
    blindness or a submerge (EXE6's tests `sub_800EB6C` and the submerged bit). The engine's `dimming.hide_user` and
    `show_user` are EXE6's; Roll's recordings match (nothing in them reads the mark, and no viewer is blind). Shape: a
    `chip_use` rule for the user's vanish (EXE6's and EXE5's marks and tests, EXE4's none), with a recording of a blind
    viewer or a Reflector through a navi chip to confirm.
151. **What the poltergeist leaves** (JunkMan's, @exelib/poltergeist). EXE4's 0x080EA16C and 0x0800B3E8 take every
    field object but attack objects #0x4C and #0x8C and effect object #0x6E, tested by kind (EXE5's by NameID); the
    engine asks the identity (`throwable`). None of the three is ported: whoever ports one gives its identity
    `throwable = false`. JunkMan's throws themselves wait on RockCube (the lab's junkman*/obstacle).
