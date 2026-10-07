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
update at a round's end, 0x0800F5BC). The rest (+0x22, +0x26, +0x29, +0x2B) is used and unread yet: the step that
ports what reads it names it.

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
  (`cursor_after_leaving`, 0x0801E412).
- **L's message**: MegaMan's (the archive 0x08749294's entry 3, its words in both locales), EXE6's script shape.
- **Sounds** (named for their code in gen_content.py's BY_USE): open 0x7A, cursor 0x7D, pick 0x7E, back 0x7F, OK 0x80,
  refused 0x69, description 0x66, the hover 0x100, the Program Advance's parts 0x79 and its result 0x97, the gauge full
  0x81; the hide, the description's close, L's message and the pause play none (optional roles).

Checked against the chip lab's custom/ recordings (every one whose chips the content has matches every frame and
every sound call; with exe4-compat's harness comparing the gauge as the recording console holds it). Not yet: the
Double Soul button and its window (the selection's states 0xC and 0x10), the dark chip offer (§18, group A's step 4).

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
  hit's counting 0xFF and wearing 0x7F. Anger: 120 ticks paralyzed (0x0800C540) asks for it; it sets the mood 0x80
  and lasts 600 ticks (0x0800C560); its end sets 0x80 through the setter.
- **Full Synchro**: a chip's boost doubles it, leaves the mood at 0x99 and sounds 0x1BB; anger's boost ends the anger
  without a sound (0x0800D54E). The aura (actor 0x5E, 0x080CD180, EXE5's code) spawns for a navi in Full Synchro
  (0x0800D9AE) and waits while paused (0x080CD276 sets no header flag). The window's faces: the base form's mugshots
  by emotion (group D).
- **Counters**: Cannon opens a 16-tick window in a netbattle (0x0800BA66); a counter hit paralyzes (status byte 0x12,
  0x0800B08E), closes the window, shows COUNTER and sounds 0x10A (0x08013410; 0x73 too when the hitter was in Full
  Synchro, hit flag 0x80, 0x08012CC0).
- **Light and dark** (rules/light_dark): a dark MegaMan clears the holy panel he stands on (0x080132E6); a navi that
  isn't dark closes a hole (0x08013318, §18 item 12).

Checked: the lab's emotions/counter, counter-side1, counter-buster, full-synchro-hit and full-synchro-card match every
frame and sound call. Not yet: the worried case (M-Cannon), the COUNTER text's battle-over gate and the 0x73 sound
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
    at priority 1 (the pack's `layout.hp_number_priority`).
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
    the holding banner's hold (0x08014AA8) until the banner is gone (0x08014ABE: the pack's `layout.judge_from_hold`).
    The HUD knows the judge's banner by its role. Unverified: no recording reaches a judge (§18 item 130).
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
  full gauge's cells are exact on all 10,509 full-gauge frames of flow/no-time-limit.
  The custom screen while it opens and while picking (custom/cannon, describe, three-picks, second-screen) differs
  only in the UNITE button (Double Soul's). Its description, while up, matches (EXE4's text from (0x3F, 0x6D), its
  arrow where the message box's is: the pack's `layout.chatbox_text`, `chatbox_arrows`). At its end the original
  clears the text a tick before the box closes and draws the cursor a frame sooner (group A's).
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
- **effects**: the shake (0x08025FE0) draws from RNG2 and holds while paused without dimming (EXE5's
  `battle_rng`); a hit spark (EXE5's 0x080E0870, the same code), afterimages and overlays (the same code) as EXE5's;
  the retype (0x08012ED0) EXE5's, storing what it hits to the row number plus 0x5C (`is_alone`); the palette
  flash (0x080E2A34) EXE5's hold, in palette slot 9 (`before_fades`, the fades' slots to confirm); the sprite frame
  load (IWRAM 0x0300632C) EXE5's code; the obstacles' actions from 6 (the player's table, 0x080EAEFC: six framework
  states, entry, take control, deletion, flinch, paralysis, drag, then the kind's own); the Full Synchro aura
  (0x080CD180) EXE5's. **The damage word** (0x08012860) is EXE4's own: the damage the low 14 bits, 0x8000 doubling,
  and 0x4000 status 0x12 without a flinch, nothing else.
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
- The palette flash before the fades: EXE4's fade slots (§15 effects).
- `status.missing_collision_status`: what EXE4's console reads through a navi's missing collision data on a round's
  first tick, if its code reads it there at all (EXE5's open-bus value until a recording shows).
- The obstacles' own actions from 6: an obstacle's action table, as the player's (§15 effects).
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
  0x4653, a card's number or 0xFF); `game_regions`. **`rng1s` and `regular_flags`** (both consoles' RNG1 and Regular
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
  compares as such; hp, max HP, position, timer, animation, status: the collision record's hit flags, +0x54, in the
  recordings from oracle-trace's `collision_status`, 0 in the lab's first ones, which read EXE6's +0x3C); `panels`
  (the 6x3 field's type and owner, the panel's +0 and +1); `chip_blocks` (both, 0x50 bytes, EXE6's layout).
- **sounds:** a line per call each battle frame queued (the frame, the m4a call, its arguments): EXE4's queue holds
  SongNumStart, MPlayAllStop, VolumeControl (the m4a players by EXE6's numbering, EXE4's 0x1210 further), FadeOut,
  SongNumStop, ImmInit and FadeIn (§3.4).

**The decode and the replay** are exe4-compat's (`codec`: the 0x40-byte NaviStats by its fields, the panels, the chip
blocks, the link record of §18 item 23; `trace`, feature `trace`: the lines, their decode, a round's setup in the
engine's terms, the buttons fed, the comparison, `run_round`), as exe5-compat's are EXE5's. A round's stage is the
settings record's layout and actor list (compat's stages.toml; the other ROMs' records are games.toml's distance from
Red Sun US's); its background the record's +5; its stats, where the recording carries the saves' NaviCusts, those and
its patch cards compiled by the rules (§8) and checked against the recorded block, else the recorded block over the
navi's fresh stats; an unported field (supports, All Guard) a need that stops the setup. The compat tables
kinds.toml (the object kinds' pools and numbers), actions.toml (the navi actions past the framework's states, which are
EXE5's order: idle 6, a step 7, the buster 8, Cannon 0x0B, the charged shot 0x24) and records.toml (navis, weapon
routines, souls, auras by number) are written by hand as the replays reach them. The verification workspace's
trace-tests `exe4_replay` and sound-tests `exe4_sounds` run them over data/traces/lab-exe4; nettai-tool plays an EXE4
recording (`Exe4TracePlayer`).

**Known deviations** (the workspace's trace-tests `deviations.rs`, `Set::Exe4Lab`, as EXE6's lab has its own): a
recording the engine departs from on purpose matches every frame up to its deviation, and the frames after it aren't
compared. `modcards/017-custom2`, `modcards/105-panel-change` and `modcards/110-custom3`, frame 274: nettai has no link
cable, and the replay feeds the custom screen's OK as the fight saw it, the cable's delay (4 ticks) after the
original's screen took it, so the side's selection status (BattleState +0x14 bit 0, which OK clears: 0x08020652) runs 4
ticks longer, and the custom HP-drain bug, which counts only while it is set (0x0800C194), drains a point the original
doesn't. No link latency goes into the engine (the user's word); a recording with another cause is no deviation.

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
   - 0x0800CA28, the requests, the charge and the action in use cleared too: the buster's (0x080EB3D6, 0x080EB3F0)
     and the charged shot's (0x080ECCCC), many chips', EXE6's kind 1;
   - 0x0800C9FC, that and AIData +0x3A from the attack's +5 (its lockout): the chips' (Cannon's 0x080EB9E8) and
     action 9's (0x080EB51C, likely the B+Left ability), EXE6's kind 2. One field for both: the idle that reads it
     decides whether the B+Left ability's is the engine's chip lockout or its back special's (kind 3).
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
   - The hit's bug (0x0800D9E8): NaviStats' byte by the code (collision +0x48) gets its argument (+0x49), 0xFF by
     0x0804770C; the rules' `navi_bug` hook, to write with the first EXE4 chip that carries a code (the damage word
     has none).
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

16. **No time limit, no double KO** (the lab's first batch): nothing ends a netbattle's stand-off, and when both navis
    are deleted on the same tick side 1's shot resolves first and side 1 survives. The engine's link battle has the
    judge's ruling (round result 7) and a draw. Shape: flow rules `time_limit: false` and the KO order as data.
    Placeholder: the engine's. Read since (group A): a netbattle has a time limit from its 15th turn, as EXE6's. The
    fight's timer runs only in a battle of type 0x46 and on whose BattleState +8 (the custom screens so far) is 15 or
    more (0x08007E4E); it counts the fighting machine's +0x0A down, and under 60 sets BattleState +0x0B (0x08008066),
    which the round's result reads as the time-out, 7 (0x080079D6), the fighting machine's state 0x14: TIME UP for 60
    ticks (0x08007378), then the judge (0x0800739E, 0x08021F94: the numbers rolled on RNG2, its banner 0x28 and the
    HUD's element 0x200 with both damages, 0x080163C8 and 0x080152EA). The lab's flow/no-time-limit stands three
    minutes in its first turn, which this doesn't reach. The judge's banner is the judge's layout in the pack.
17. **The fight-live test.** The fight runs while the fighting machine's first byte is 4 and BattleState +3 is 4; for
    one tick as the custom screen closes the machine still reads 4. Pause sets fight[0] to 0x18. To compare with the
    engine's fight states in step 5.
18. **Battle type.** A netbattle record's +4 (0x46; the battle state's +0x0F, 0x48 in the lab's) is EXE4's battle type,
    its battle flags by it (0x08007EEC's table). The engine's `mode` is EXE6's numbering. Placeholder: each stage's
    `mode = 0`, `effects = 0x88C` and `panel_pattern = 0x38` (EXE5's netbattle's).
19. **Stages that wait.** gen_rules.py lists them in stages.luau's header: the records with obstacles (actor kinds 3,
    5, 6, 7, 0x080FC138's actor lists: 0x08006754's table at 0x08006778 spawns each). Port the obstacles, then generate
    them (gen_rules.py's `STAGE_ACTORS`: a kind's module and its entry's argument).
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
      entry's argument: bit 7 the side, the rest the HP in eights: 0x7D and 0xFD, 1000), its HP numbered. Its HP out
      (unless time is up: `battle_isBattleOver`'s Z flag, `battle.time_up`), sound 0x6F and its side loses the round
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
    - Kind 6, effect #0x41 (records 88 to 95, 0x080E6820: off the field, spawning an attack by turns). To port.
20. **The link pick.** 0x0803AA6C draws `PosRNG2() % count` (0x44 for a single battle, 0x60 for a triple one: by the
    battle type, 0x08007D68) into the first 96 records, then `PosRNG2() % 24` into the backgrounds (0x0803AAA4). A
    set's first battle picks its rounds' places at once (0x08007D68). **Done:** `link_pick.backgrounds` (the table's
    24, by their names), `first_round_stages` (a random match is a triple battle's: every round among the first 0x60).
    No RNG field: nettai picks a match's places before the battle with its own generator (docs/frontend.md §2), so
    only the odds are the game's, which the lists state. a95f's sweep (1,000 seeds, tools/chiplab/gen_exe4.py
    `STAGES`) drew from all 96 records: no panel type 11; its obstacles are the four of item 19 (the boulder #0x6E, #0x76, #0x9C, effect #0x41).
    So `stages` must list all 96 records, each working: the 34 waiting ones (actor kinds 3, 5, 6 and 7) wait for
    those four obstacles (item 19); until then the list leaves them out and the odds differ.
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
27. **The projectile** (0x080CD354, 0x080CD3D4) runs @exelib's EXE5 code with EXE4's rows; its tick is 0.89 alike and
    its rows 10 and 11 set a status (0x08013212): compare the code and port the difference.
28. **The attack's +6 halfword** adds to the cannon's damage (0x080EB984); the engine's chip use doesn't set it.
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
    shader before it (0x0800BB18: AIData +0x2C in 1 to 90, a shader from 0x0800BB40 by the battle timer; what sets
    +0x2C is to find).
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
    (0x0802E070's +0x11) and the B count runs only under request 0x80. No netbattle sets it; the engine has none.
50. **The levels at the ask.** The decode copies the charge levels to AIData +0x14 and +0x15 as it asks for an attack
    (0x0800BE48, 0x0800BE8E, 0x0800BF10); what reads them is to find (the engine keeps none).
51. **A player's deletion.** EXE4's (action 2, 0x08010850) is its own, not EXE6's `sub_80173F4`: the hurt animation
    as it starts (EXE6's with the explosions); the alive count alone (0x080079C6), never the alive lists; no chip count,
    charge glow link or tracking let go; its second related and barrier byte at the start, AIData +0x60 with the
    explosions; no aura or overlay links; no death hook at the end. **Done:** the role `actions.deletion`
    (content/exe4/rules/deletion.luau; the engine's own where the role is unfilled: EXE6, EXE5); the flow/ko recordings
    match through the deletion. Open: the deletion of a navi in auto battle or of another navi (0x08010D4C: effect object
    0x11 and a 90-tick explosion), which EXE4's content doesn't reach (its players are MegaMan, by a player).
52. **The dead player's object.** EXE4's destroy state (0x0801052C) lets go of the collision data, frees the object and
    counts one actor fewer at once, its reservations left as they are; EXE6's (`sub_8016C4E`) keeps the object in its
    slot. **Done:** `reactions.dead_player` (`kept`: EXE6, EXE5; `freed`: EXE4). Open: 0x0801052C's branch for a player
    of param 2 (an owner's count, at the object's +0x78, one less), which no player the engine spawns has.
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
    ticks; types 4 and 7 (and a threshold over 255) wait on the chips that raise them. **Done:** FstBarr: EXE4's
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
    charged shot's projectile; Reflect's routine sets the B+Left cooldown as it starts (`reactions.attack_end_lockouts`)
    and a turned-aside hit marks the guard byte with 1 (`reactions.hit_test.guard_marks_direction`); the guard's sound
    (0x6E) is the roles' `sounds.guard`. HubBatc has its definition. The lab's navicust/shield, reflect and hubbatc
    match, sounds too. Open: AntiMagc (routine 0x27: its stance, action 0x72 at 0x080EE9EE, arms the AntiDmg chip's
    trap for 13 ticks, `sub_802CE8A` with chip 0x91, and its catch is AntiDmg's counter): with the AntiDmg chip. The
    Guard chips' variants 0 to 2 of the guard counter with a shock wave (0x080CFD2A, its row 6): with the Guard chips.
    EXE4's guard spark (object_spawnHiteffect, 0x0800B0F6) doesn't stop while paused, where the engine's does: a guard
    is never turned aside while paused in a netbattle (nothing hits then).
54. **Rush, Beat and Tango's battle controller.** The supports compile (+0x18 = 1, 2, 4; the support bug 0xFF); the
    controller that runs them in battle (0x0800C7D8, 0x0800C838, 0x0800C8C0: the reads and writes of +0x18) is open, so
    a recording with a support stops at its setup.
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
    other sits in its slot (0x08042504): Triple Supporter (74, 75: +0x18 = 7, item 54), All Guard (76, 77: +0x28, item
    60), Charge FullCustom (103, 104: the charged shot FullCustom, item 57).
57. **The ~50 patch cards that set B, B charge or B+Left to a chip** (+0x09, +0x0A, +0x0C = a weapon routine past the
    buster's): each waits on its chip as a weapon routine; the chips' work picks them up.
58. **The 12 soul patch cards** (+0x24: a battle starts in the soul) wait on the souls (item 25).
59. **The status timers while paused, and the status visual.** EXE4's status timers (0x0800AE58: paralysis +0x10,
    confusion +0x12, blindness +0x14, immobilization +0x16; no freeze or bubble) don't stop while the battle is paused
    (EXE6's `sub_800E730` and EXE5's 0x0800CB50 return), so a navi confused at the start (the move bug, item 24) shows
    the confusion's visual on the round's first tick, during the intro. The visual is effect 6 (0x080E22C8, EXE6's
    `sub_80E08FC`): its sprites by row (0x080E22B8: 14-0B confusion, 14-09 blindness), its sound 0xAE every 60 ticks
    (EXE6's 0x88), its place the owner's position and a per-navi offset (0x08011878's +6, +7), spawned at the status
    routine's registers (0x080E23B2). The lab's navicust/bug-humor and bug-undersht stop on its first tick.
60. **The patch cards' own bytes.** Card 45 sets NaviStats +0x1F, the Full Synchro at the start, which the reload's
    reset doesn't keep (0x08036CC0): **done**, the rules' stat `full_synchro_start`, which the card writes and
    rules/light_dark's starting mood reads (no setup fact: exe4-compat reads +0x1F into the stat where a recording's
    stats aren't compiled). Cards 59, 60, 89 and 90 set +0x27, MegaMan's color (1 to 4), which the reset clears too:
    **done**, the rules' stat `color`, which the cards write and rules/light_dark's palette reads (5 more a step of it,
    0x0800C03A, but in a soul other than 15); the lab's modcards/059, 060, 089 and 090. Cards 76 and 77 (All Guard) set
    +0x28, which the navi's init reads (0x0800D8E4: at 1 its guard flag, f1 0x1, is up from the start, so it turns aside
    every hit that doesn't break guards, 0x08012B84).
61. **Done: the idle stands the navi** (from AirShot's replays). MegaMan's idle (0x080EEB38) puts animation 0 on each
    tick past its first phase (0x080EEB7C: 0x080EEBAC), the 10 ticks after a reaction's end; EXE6's (`sub_80F0354`)
    and EXE5's (0x080F0254) leave the pose. A drag that keeps its pose (`status.drag`'s `keeps_pose`, 0x08010C16)
    shows 1 for those ticks and no more (`chips/0x004-airshot/hit`, `side1`: frame 431). The rule
    `status.idle_stands` (EXE4 true; EXE6, EXE5 false).

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
    - the turn timer from the 15th turn of a link battle (0x08007E4E): the fight's +0x0A, counted by 0x08008066; its
      seconds over "CUSTOM" (0x08016362, draw 7); the gauge not drawn. The engine's timer length is EXE6's until §18
      item 16 reads EXE4's. With it, the damage judge: its numbers' values (0x0801642C, 0x08016408) and a recording
      to compare;
    - the warning marker (0x0800843E: a 16x16 sprite at tile 0x360, its second frame at bit 3 of the frame counter, in
      palette 13; 0x08008424 also sounds 0x79 every 16 frames), with the chips that show it: the gauge chips'
      effect over the gauge at (120, 12) on the other console (0x080E3FAE), and 0x080E789E, 0x080E88F6, 0x080E8918.
      The renderer's (`warning_parts`) is EXE6's `sub_800AE90`, which leaves out a place near the screen's edge;
      EXE4's draws it wherever. Compare it with a recording once one of those chips is ported.
