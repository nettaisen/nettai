# EXE4 against EXE3: whose lineage EXE4's battle code is

EXE4 shares much less with EXE5 and EXE6 than EXE5 shares with EXE6 ([exe4-map.md](exe4-map.md) §0: of EXE6's battle
routines in scope EXE4 has 17% the same and 53% absent, EXE5 40% and 27%). EXE4 is the first game of its generation,
so what it does its own way might be EXE3's code carried forward. This compares EXE4 with EXE3 routine by routine, by
the same method and statuses as exe4-map.md, and says per system whether EXE4's code is EXE3's lineage, EXE4's own,
or EXE5's and EXE6's. It also says, for the facts EXE4's port is settling (the damage word, the push reading, the
flow, the effects, the NaviStats fields, the link exchange), whether EXE3 does the same.

Addresses are US Red Sun's (B4WE) for EXE4 and US White's (A6BE) for EXE3 unless said otherwise. Sources: **R** read
in the ROMs for this comparison, **T** Tango's BN3 and BN4 game support (tango-gamesupport-bn3, -bn3-dataview,
-bn4), **K** known of the games and not read here, **?** open.

## 0. Summary

- **EXE4's own code is not EXE3's.** Of EXE4's 4,103 battle routines in scope, EXE3 has 271 (6.6%) **the same**,
  75 (1.8%) the same with **other constants**, 324 (7.9%) **similar**, 233 (5.7%) a counterpart that **differs**, and
  3,200 (78.0%) **absent**. By size: 2.9%, 1.4%, 8.6%, 6.3% and 80.8%. What EXE4 doesn't share with EXE5 and EXE6, EXE3
  doesn't have either:
  - Of EXE4's in-scope routines with no same or consts counterpart in EXE5 or EXE6 (2,803), EXE3 shares 68 (2.4%)
    and has 160 more similar.
  - Of EXE4's own battle code (1,981 routines with no EXE6 counterpart, reached from the shared battle code), 2.2% is
    EXE3's (same or consts), 2.6% similar and 92% absent.
  - Of the 2,437 in-scope EXE6 routines EXE4 lacks (exe4-map.md's 53%), EXE3 has 31 (1.3%, small object-kind
    routines) and 2,200 (90%) absent.

  So the gap between EXE4 and EXE5/EXE6 is EXE4's own code, not EXE3's. EXE4 rewrote the battle engine (the actors,
  collision, the damage word, the hit test, the flow, the custom screen) in the shape EXE5 and EXE6 kept, and kept
  little of EXE3's battle code.
- **What EXE4 kept from EXE3** (R) is a short list:
  - the object system and its pools, including **8 actor slots** (EXE3: 8 actors of 0xD4 bytes at 0x02037270);
  - the **element at object +0x17** (EXE3's collision setup reads it there too);
  - the RNGs and the camera shake that draws from RNG2;
  - the panel record and its crack and break routines;
  - the **NaviCust**: its module and the programs' effect setters, 89 of 199 routines the same or consts;
  - the design (not the code) of the **0x40-byte stat block** a side and of the **link record** that carries it with
    the sender's RNG2;
  - a few hundred utility routines (text, sprites, sound, flags) shared by all three generations.
- **What EXE4 began** (absent in EXE3, kept or changed in EXE5/EXE6):
  - the chip record of 0x2C bytes and its getter;
  - the collision record and its damage word, the hit modifier as a bitfield, and the six-row push reading;
  - the hit registration;
  - the battle flow's word state and its win and loss counts;
  - the link transport;
  - the custom screen's dispatcher;
  - the hand block (a fired count and six ids).
- **EXE4 alone** (absent in EXE3 and in EXE5/EXE6): Double Soul's change, the emotions, the dark chips offered in
  battle, the custom screen's state handlers, a third of the battle flow (66 of 190 routines), and 1,400 of 2,719
  object-kind routines.

## 1. ROMs and method

| Code | File | Version, region | IWRAM code (R) |
|---|---|---|---|
| A6BE | exe3_rom_e.srl | White, US | 0x17AC bytes from 0x08235F00 to 0x03005E00 (the boot's pool at 0x08000214) |
| A3XE | exe3b_rom_b_e.srl | Blue, US | the same |
| A6BJ | exe3_rom.srl | White, Japan | 0x17AC bytes from 0x08235B00 to 0x03005E00 |
| A3XJ | exe3b_rom_b.srl | Black, Japan | the same as A6BJ |

The ROM directory also has exe3_white_cutin.gba and exe3_black_cutin.gba, which are Japanese ROMs patched by the
bn3_cutin project and unused here, and a Blue US save, exe3b_rom_b_e.sav, with savestates. Tango (T) gives the four
codes the same EWRAM and nearly the same battle hooks (US: the round start at 0x080059A8 in both versions).

**The maps.** The verification workspace's tools take EXE3 ROMs as they take EXE4's (`gba.py` reads EXE3's IWRAM pool
at 0x08000214; tables go to target/exe3):

- `tools/exe5/bmap.py --to A6BE` maps EXE6's inventory into EXE3: target/exe3/bmap-A6BE.tsv, ram-A6BE.tsv.
- `tools/exe3/xmap.py` maps EXE4's routines, found by discovery, into EXE3 the way `tools/exe4/emap.py` maps EXE5's
  into EXE4. It uses the same exact, shape, call, table and order matches and the same statuses (same, consts,
  similar, differs, absent), and sets the three maps side by side: target/exe3/xmap-B4WE-A6BE.tsv. Each EXE4 routine
  carries:
  - its EXE3 counterpart and status;
  - its name, area and scope from its EXE6 counterpart. An EXE4 routine with no EXE6 counterpart takes the area of
    the shared battle routine that first reaches it (only.py's walk); it is in scope if reached.
  - its status against EXE6 and against EXE5;
  - its **lineage**, from its best status against EXE5 or EXE6 (s56) and its status against EXE3 (s3), where same and
    consts count as shared:
    - core: shared with both;
    - exe3: shared with EXE3 only;
    - later: shared with EXE5/EXE6 only;
    - exe3~, later~, both~: similar only, to that side or to both;
    - own: neither.
- `xmap.py diff ADDR` sets an EXE4 routine beside its EXE3 counterpart, and `xmap.py fields`, `fields3` and `after`
  compare field offsets: EXE4's against EXE3's, and EXE6's through EXE4's to EXE3's.
- `tools/exe3/systems.py` gives the per-system table of §3. `tools/exe3/navistats.py` lists EXE3's stat block
  accessor calls by offset, with each caller's EXE4 twin.

**How sure.** As in exe4-map.md, `same` and `consts` are facts about the code, while `similar` and `differs` are
pairings. Here most `similar` (163 of 324 in scope) and `differs` (153 of 233) pairs come from the `order` step,
which pairs what lies between matched neighbors, so this comparison counts only same and consts as shared and reads a
similar pair before citing it. Where EXE4's design matches EXE3's but its code doesn't (the stat block, the link
record), §3 and §4 say so from reading both ROMs.

## 2. The numbers

### 2.1 EXE6's routines in EXE3, EXE4 and EXE5

EXE6's 4,567 in-scope battle routines (`bmap.py --to A6BE`; EXE4's and EXE5's from exe4-map.md §2 and exe5-map.md §0):

| | same | consts | similar | differs | absent |
|---|---|---|---|---|---|
| EXE3 | 247 (5.4%) | 66 (1.4%) | 433 (9.5%) | 401 (8.8%) | 3,420 (74.9%) |
| EXE4 | 17% | 5% | 18% | 7% | 53% |
| EXE5 | 40% | 6% | 22% | 5% | 27% |

By area, in EXE3: same, consts, similar, differs, absent (EXE4's same in brackets):

| Area | Routines | In EXE3 |
|---|---|---|
| object kinds, chips, navis | 2,682 | 146 [393], 33, 365, 365, 1,773 |
| actors, collision, status, HUD | 821 | 5 [57], 3, 6, 13, 794 |
| custom screen, gauge, camera | 367 | 21 [56], 11, 10, 1, 324 |
| elsewhere, run in a netbattle | 222 | 40 [125], 13, 40, 11, 118 |
| battle flow | 180 | 5 [36], 2, 1, 3, 169 |
| battle objects, panels | 137 | 13 [42], 0, 4, 2, 118 |
| link layer | 79 | 0 [18], 0, 0, 0, 79 |
| IWRAM routines | 37 | 4 [12], 3, 3, 3, 24 |
| object system | 26 | 13 [15], 1, 4, 3, 5 |
| link status, battle settings | 16 | 0 [4], 0, 0, 0, 16 |

The same EXE6 routines by their status in EXE4 (rows) and in EXE3 (columns):

| In EXE4 | Routines | same | consts | similar | differs | absent |
|---|---|---|---|---|---|---|
| same | 758 | 199 | 32 | 136 | 35 | 356 |
| consts | 218 | 3 | 14 | 34 | 31 | 136 |
| similar | 831 | 19 | 10 | 146 | 124 | 532 |
| differs | 323 | 5 | 0 | 41 | 81 | 196 |
| absent | 2,437 | 21 | 10 | 76 | 130 | 2,200 |

The routines all three share are EXE4's same row: 199 the same in EXE3. Almost nothing EXE4 lacks is in EXE3. The
31 same or consts are small object-kind routines (16 to 60 bytes, many of them 32-byte ones paired by table) and four
tiny helpers.

### 2.2 EXE4's routines in EXE3

`xmap.py` discovers 20,064 EXE4 routines, 4,103 of them in scope (2,122 by their EXE6 counterpart, 1,981 reached).
In EXE3:

| Area (EXE6's audit) | EXE4 routines | same | consts | similar | differs | absent |
|---|---|---|---|---|---|---|
| object kinds, chips, navis | 2,719 | 136 | 32 | 223 | 181 | 2,147 |
| actors, collision, status, HUD | 536 | 6 | 3 | 6 | 24 | 497 |
| elsewhere, run in a netbattle | 250 | 63 | 17 | 58 | 11 | 101 |
| custom screen, gauge, camera | 206 | 27 | 8 | 8 | 2 | 161 |
| battle flow | 190 | 7 | 7 | 2 | 7 | 167 |
| battle objects, panels | 102 | 15 | 2 | 18 | 7 | 60 |
| link layer | 43 | 0 | 0 | 1 | 0 | 42 |
| IWRAM routines | 28 | 5 | 6 | 3 | 0 | 14 |
| object system | 20 | 12 | 0 | 5 | 1 | 2 |
| link status, battle settings | 9 | 0 | 0 | 0 | 0 | 9 |
| all | 4,103 | 271 (6.6%) | 75 (1.8%) | 324 (7.9%) | 233 (5.7%) | 3,200 (78.0%) |

### 2.3 How EXE4's code splits

EXE4's in-scope routines by their best status against EXE5 or EXE6 (rows) and their status against EXE3 (columns):

| Best in EXE5/EXE6 | Routines | same | consts | similar | differs | absent |
|---|---|---|---|---|---|---|
| same | 995 | 226 | 40 | 124 | 49 | 556 |
| consts | 305 | 3 | 9 | 40 | 29 | 224 |
| similar | 895 | 18 | 15 | 109 | 75 | 678 |
| differs | 330 | 2 | 2 | 24 | 43 | 259 |
| absent | 1,578 | 22 | 9 | 27 | 37 | 1,483 |

By lineage, routines (bytes):

| Area | core | exe3 | later | exe3~ | later~ | both~ | own |
|---|---|---|---|---|---|---|---|
| object kinds, chips, navis | 133 | 35 | 599 | 37 | 429 | 86 | 1,400 |
| actors, collision, status, HUD | 7 | 2 | 147 | 0 | 185 | 2 | 193 |
| elsewhere, run in a netbattle | 60 | 20 | 97 | 6 | 17 | 9 | 41 |
| custom screen, gauge, camera | 35 | 0 | 39 | 0 | 25 | 1 | 106 |
| battle flow | 7 | 7 | 57 | 1 | 51 | 1 | 66 |
| battle objects, panels | 14 | 3 | 35 | 4 | 25 | 8 | 13 |
| link layer, link status | 0 | 0 | 35 | 0 | 15 | 0 | 2 |
| IWRAM routines | 11 | 0 | 10 | 2 | 4 | 0 | 1 |
| object system | 11 | 1 | 3 | 1 | 2 | 2 | 0 |
| all | 278 (6.8%) | 68 (1.7%) | 1,022 (24.9%) | 51 (1.2%) | 753 (18.4%) | 109 (2.7%) | 1,822 (44.4%) |
| by bytes | 3.4% | 1.0% | 18.0% | 1.6% | 24.5% | 3.9% | 47.6% |

EXE4's code is, in round numbers:

- one quarter passed on to EXE5 and EXE6 unchanged;
- one fifth passed on changed;
- not quite half its own;
- under a tenth EXE3's, most of that shared by all three.

The core (6.8%) is mostly small helpers (sprites, sound, text, flags, coordinates), the object system and the
camera. The in-scope EXE3-only routines (1.7%) are scattered helpers. The largest body of EXE3-only code, the
NaviCust's, is mostly outside the audit's scope, so it shows in §3 rather than here.

## 3. By system

`systems.py`: EXE4's routines per system (an audit area's in-scope routines, or EXE4 ranges and routines read for
this), their status in EXE3, their best status in EXE5/EXE6, and the verdict. Statuses are same/consts/similar/
differs/absent.

| System | EXE4 | In EXE3 | Best in EXE5/EXE6 | Lineage |
|---|---|---|---|---|
| object system and pools | 20 | 12/0/5/1/2 | 14/0/4/1/1 | **all three**: EXE3's, kept (8 actor slots EXE3's) |
| object record layout | | (fields) | | EXE3's record grown by 4 bytes; the element at +0x17 is **EXE3's** |
| chip record and getter | 1 | absent | same | **EXE4 began it** (EXE3's is another record) |
| RNGs | 5 | 5/0/0/0/0 | 5/0/0/0/0 | **all three** |
| NaviStats accessors and init | 19 | 0/0/0/2/17 | 7/3/4/2/3 | **EXE3's design**, rewritten code that EXE5/EXE6 kept |
| NaviCust and program effects | 199 | 57/32/36/3/71 | 28/5/8/4/154 | **EXE3's**, dropped by EXE5/EXE6 |
| custom screen (no camera) | 178 | 13/1/4/1/159 | 40/8/25/8/97 | EXE4's own (105 routines), the dispatcher EXE5/EXE6's |
| camera and shake | 52 | 19/8/16/3/6 | 46/2/2/1/1 | **all three**; the RNG2 shake EXE3's, kept by EXE5 |
| Double Soul's change | 2 | absent | absent | **EXE4's own**; EXE3's styles don't change in battle |
| dark chips | | | | **EXE4's own** (EXE3 has none, K) |
| emotions | 14 | absent | 0/2/4/3/5 | **EXE4's own** (EXE3 has none) |
| battle flow | 190 | 7/7/2/7/167 | 55/9/56/16/54 | EXE5/EXE6's shape, EXE4's handlers; not EXE3's |
| link exchange | 52 | 0/0/1/0/51 | 33/2/15/0/2 | transport EXE5/EXE6's; the record's design **EXE3's** |
| actors, collision, status, HUD | 536 | 6/3/6/24/497 | 65/89/187/61/134 | EXE5/EXE6's shape (193 EXE4's own); not EXE3's |
| collision setup and damage word | 13 | 0/0/0/4/9 | 1/6/6/0/0 | **EXE4 began it**; EXE3 has no damage word |
| hit test and registration | 5 | absent | 1/1/2/0/1 | **EXE4 began it** (EXE6's design); not EXE3's |
| push reading | 2 | absent | 0/0/2/0/0 | **EXE4's own**, close to EXE5's and EXE6's |
| statuses | | | | EXE4 one status byte, as EXE5/EXE6; EXE3 a pair |
| battle objects, panels | 102 | 15/2/18/7/60 | 42/7/35/10/8 | the panel record and routines **EXE3's**, kept by all |
| object kinds, chips, navis | 2,719 | 136/32/223/181/2,147 | 563/169/536/225/1,226 | half EXE4's own; EXE3 a few helpers |

### 3.1 Object system and pools

The object system is the same code in all three: spawning, the update list, freeing, initializing the pools. Of EXE4's
20 routines, 12 are the same in EXE3. The update list is EXE3's 0x02008BF0, EXE4's 0x02009E40. The pool table (R:
EXE3's 0x080031E4, EXE4's 0x08003444) has the same six pools in the same order, 16 bytes each:

| Type | EXE3 | EXE4 |
|---|---|---|
| 0, the overworld player | 0x02009560, 1 × 0xA4 | 0x0200A610, 1 × 0xA8 |
| 1, actors | **0x02037270, 8 × 0xD4** | 0x0203B180, 8 × 0xD8 |
| 2, overworld NPCs | 0x02004560, 16 × 0xC4 | 0x02006140, 16 × 0xD8 |
| 3, attacks | 0x020389A0, 32 × 0xD4 | 0x0203C080, 32 × 0xD8 |
| 4, effects | 0x02034FC0, 32 × 0xC4 | 0x02037D10, 32 × 0xC8 |
| 5, overworld map objects | 0x0200A840, 32 × 0xB4 | 0x0200B820, 32 × 0xB8 |

The table's third word gives a pool's size in words in EXE3 (0x1A8 for 8 × 0xD4) and in bytes in EXE4 (0x6C0), which
is why `InitializeStructsOfObjectType` (EXE4 0x0800340C, EXE3 0x080031A0) is only similar. **EXE4's 8 actor slots are
EXE3's.** EXE5 doubled them to 16, EXE6 to 32. Tango's BN3 unit record (T) agrees: 0xD4 bytes from 0x02037270.

### 3.2 The object record

EXE3's records are 4 bytes shorter: actors and attacks 0xD4, effects 0xC4. EXE4's are 0xD8 and 0xC8, as EXE5's and
EXE6's. The fields EXE4 and EXE3 both read at the same place (`xmap.py fields`) are at the same offsets from +0x00 to
+0x4C and from +0x60 on. The exceptions:

- The **collision pointer** is at EXE3's +0x50, EXE4's +0x54. EXE3's +0x54 holds the state to return to after a
  flinch (0x080ACB66), and its own record pointer is at +0x68 (the navi's, 0x080B0320).
- The **element** is at +0x17 in EXE3 and in EXE4, where EXE5 and EXE6 have +0x0E. EXE3's collision setup copies
  object +0x17 into collision +0x02 (0x080ACA68), as EXE4's does (0x080129B4). EXE6's `object_setupCollisionData`
  reads +0x0E. Through EXE6, EXE4 and EXE3 together (`xmap.py fields3`), EXE6's +0x0E is EXE4's and EXE3's +0x17 in
  the three paired routines that use it (EXE4 0x080D64A8, 0x080D8318, 0x080D89F8), and +0x0E in all three in eight
  others. So **the +0x17 byte is EXE3's place for the element**, which EXE5 moved.

Tango's units agree on the panel at +0x12, the destination at +0x14 and the side at +0x16 in both games (T). EXE3's
"chips remaining" byte is at +0x1A (T), EXE6's `ChipsHeld` offset; EXE4 counts fired chips in the hand block instead
(§3.6).

### 3.3 The chip record

EXE3's is **0x20 bytes** at 0x08011510 (T, R): six codes +0x00, element +0x06, family and subfamily +0x07/+0x08,
rarity +0x09, MB +0x0A, power +0x0C, the library number +0x0E, the Mega and Giga flags +0x13, then the icon, image and
palette from +0x14. Its getter (0x08011444) multiplies by 0x20. EXE4's record of 0x2C bytes and its getter
(0x080190D0, EXE5's and EXE6's `getChip8021DA8`, the same code) have no EXE3 counterpart: **the chip record is EXE4's
start, kept by EXE5 and EXE6**. EXE4 puts MB right after rarity (+0x05, +0x06), as EXE3 does (+0x09, +0x0A), and EXE5
moved MB after element and class. That adjacency is the only trace of EXE3's layout.

### 3.4 RNGs

`GetRNG1`, `GetRNG2`, `SeedRNG2` and `GetPositiveSignedRNG2` are the same code in all three (EXE3 0x080016BC,
0x0800168C, 0x08001684, 0x080016A2). EXE3 has no `GetPositiveSignedRNG1`. EXE3's RNG1 is at 0x02009730 and its RNG2
at 0x02009800 (R, T).

### 3.5 The stat block (NaviStats)

**The 0x40-byte block a side is EXE3's design** (R). EXE3 keeps MegaMan's block at 0x02005770, and the battle's two
at **0x02037920 + 0x40 side**. Its accessors are at 0x0804731C (set byte), 0x08047322 (set halfword), 0x0804734C,
0x08047352 and 0x08047358 (get byte, signed byte, halfword), 0x0804732C (set a side's byte) and 0x0804733A (get a
side's byte). Its init is 0x0804735E. The battle start fills the battle blocks from the link records (§4.5). The code
is not EXE4's: EXE3 adds 0x40 for side 1 where EXE4 multiplies by 0x40, and none of EXE4's 19 accessor and init
routines has a same or similar counterpart in EXE3. EXE4's versions are what EXE5 and EXE6 kept, with the block grown
to 0x60 and 0x64. The fields moved too; §4.4 lists those read on both sides.

### 3.6 The custom screen

None of the custom screen proper is in EXE3: neither EXE4's dispatcher (0x0801E0EC, EXE5's and EXE6's `sub_8026A28`)
nor its state handlers (0x0801E110, 0x0801E210, 0x0801E180). The custom-screen area's 35 routines shared with EXE3
are the camera (§3.7) and small helpers. Tango (T) shows two differences in the data:

- **The hand block.** EXE3's is 0x24 bytes a player at 0x02034060 (six chip ids and six codes); EXE4's is 0x50 bytes
  a player at 0x02035CB0 (a fired count and six ids, EXE5's and EXE6's shape).
- **The screen's flag.** EXE3's custom screen state (0x0200C0C4) is the local console's only, while EXE4's per-player
  flags (0x02036440) cover either player, as EXE5's and EXE6's do.

So the custom screen is EXE4's own on EXE5's and EXE6's frame, not EXE3's.

### 3.7 The camera and its shake

The camera is the same in all three: 19 of 52 routines the same and 8 consts in EXE3, 46 the same in EXE5 or EXE6.
EXE4's shake (0x08025FE0) is EXE3's (0x08022AE4) with one check added (§4.3).

### 3.8 Double Soul and Style Change

EXE4's Double Soul change is a navi action: 0x080EBB6C sets its side's soul (NaviStats +0x24) from its record's +0x10
(the object's +0x58) and calls 0x0800B90A with the navi and the soul, as the navi's init does (0x080107B2).
0x080EBEAC sets the soul too. Both sit in a block of EXE4-only routines (0x080EB80C on), and neither is in EXE3, EXE5
or EXE6. EXE3's style is fixed for a battle (R):

- The link record carries it (+0x04 of the record: 0x080071F0).
- The battle start puts it in the side's record (0x020384D0 + 0x88 side: type +0x04, element +0x05; 0x08007288).
- The navi's init (0x080B02E8) sets the object's element (+0x17) and its animation (+0x10) from it.
- The style's stat effects are written into the stat block at its init (0x0804735E; the types by Tango's numbering,
  T): Custom (2) +1 at +0x13, Team (3) +1 at +0x14, Guts (1) at +0x26 and +0x08, Shadow (6) at +0x0D.

Nothing in EXE3 changes a navi's form during a battle (K: Style Change happens between battles). **Double Soul and
its turn-start change are EXE4's own**, and EXE5's Soul Unison sequencer is a later design again (exe4-map.md §6).

### 3.9 Dark chips and emotions

EXE3 has neither (K). Its chip record has no dark flag (the flag byte +0x13 holds Mega and Giga), and its stat block
has no mood (no accessor call names its +0x00). None of the 14 EXE4 emotion routines read for this has an EXE3
counterpart: the mood setter 0x0800D836 (EXE6's `sub_8013892`, similar) and its neighbors to 0x0800DB20, the
light/dark helper 0x0800F56A, and Full Synchro on a counter, 0x080131E4. **Both systems are EXE4's own.**

### 3.10 NaviCust and the programs' effects

This is EXE3's code that EXE4 kept and EXE5 dropped. EXE4's NaviCust module (0x08046D94 to 0x08048600, the code that
loads the part table 0x0804563C) and its program and patch card effect setters (0x08041B00 to 0x08042260) come to 199
routines. Of these, 57 are the same in EXE3, 32 consts and 36 similar, against 28 and 5 in EXE5 and EXE6. EXE3's are
at 0x0803AEB0 to 0x0803C360 (the part table 0x08039420, T) and 0x0803C4DC to 0x0803D0DC. Large routines are
byte-identical: EXE4's 0x080470B8 (208 bytes) is EXE3's 0x0803B0C0, and its 0x08047DDC (230 bytes) is EXE3's
0x0803C008 with other constants. **EXE4's NaviCust compile is EXE3's**: EXE5's and EXE6's shared compile
(`sub_813C458`) is the later one (exe4-map.md §8).

### 3.11 Battle flow and link exchange

EXE4's battle-flow handlers are not in EXE3. Its round loop (0x08006B14), round start (the routine holding Tango's
hook 0x08006710), round result (0x080070F0) and set end (0x08004F34) are all absent. The area's 14 same or consts
routines are helpers of 8 to 70 bytes (EXE6's `sub_8009FCC`, `sub_800AA06` and kin). EXE3's flow is another machine
(§4.2). The link layer's 52 EXE4 routines have no EXE3 counterpart either, while 35 are the same or consts in EXE5 or
EXE6. The design of what the link carries is EXE3's, though (§4.5).

### 3.12 Actors, collision, status

EXE4's actor code is not EXE3's. The navi's actions (the buster, charge, movement and chip use: 219 routines from
0x0800B590 to 0x0800D400) have no EXE3 counterpart but one weak pairing. The navi's init (0x0801079C, EXE6's
`sub_8016F56`) has none, EXE3's being 0x080B02E8. The collision, hit and damage routines are absent in EXE3 (§4.1).
The area's 9 same or consts routines are small helpers (`AddRandomVarianceToTwoCoords`, EXE6's `sub_8018076` and
kin). EXE3's collision record is laid out otherwise (R):

- the element at +0x02, the panel at +0x08, the hit modifier at +0x0A;
- the damage at +0x14, received damage by element from +0x1A;
- the type flags at +0x34/+0x38;
- the hit flags at +0x2C, and the hit modifier taken at +0x0B.

EXE4's has the damage at +0x1E, the modifiers at +0x0C/+0x0D, the statuses at +0x0E/+0x0F, and the hit flags at
+0x54/+0x58.

**Statuses.** EXE3's hit registration carries a pair of status bytes from the attacker's +0x0C/+0x0D into the
target's +0x0E/+0x0F (0x03007098). EXE4's carries one byte, the attacker's +0x0E into the target's +0x0F (0x08012BF2),
as EXE5 and EXE6 do.

### 3.13 Panels

The panel record and its routines are EXE3's, kept with changes by EXE4 and passed on. EXE4's crack, break and poison
routines (0x08009AEC to 0x08009D68) are similar at 0.86 to 0.96 to EXE3's 0x0800B568 to 0x0800B73C. They use the same
record (the type byte +0x00, the flag word +0x04), the same flags (0x10, 0x40, 0x20000) and the same clearing masks
(0x23F0F, 0x23F5F), and the coordinate helpers are the same code. EXE4 adds a sound (0x95) when a panel cracks and has
its own mask of panels that can't crack: 0x0F8A0080, EXE3's 0x3E0A0080.

## 4. The port's facts against EXE3

### 4.1 The damage word and the push reading

- **Damage word: EXE3 has none.** EXE3's collision setup (0x080ACA38) copies the object's damage halfword (+0x2C)
  into collision +0x14 unchanged. Its hit registration (IWRAM 0x03007020) adds the attacker's +0x14, as it is, to
  the target's damage for the attacker's element (+0x1A + 2 element). Neither reads flag bits. EXE4's decode
  (0x08012860: the low 14 bits, 0x8000 doubling, 0x4000 setting status 0x12 with hit modifier 0) is EXE4's own,
  shaped like EXE5's and EXE6's (`sub_8019F44`), and not from EXE3.
- **Hit modifier: EXE3's is an index, not a bitfield.** EXE3's registration keeps the larger of the target's +0x0B
  and the attacker's +0x0A (0x0300708E). What a hit does is looked up by that index, 0 to 5, in a two-column flag
  table at 0x080AF3B0 (0x080AF088): flag 4 is a flinch (0x080ACB48), and flags 1 and 2 are other reactions
  (0x080AD004). EXE4 ORs the attacker's +0x0C into the target's +0x0D (0x08012C40), as EXE6 does.
- **Push reading: EXE3 has no direction rows.** EXE4 scans bits 2 to 7 of +0x0D for the first set bit and reads its
  row of (x, y, how far) in the table at 0x0800ACDC: +x 6, −x 6, +x 1, −x 1, up 1, down 1, then none. That is
  0x0800ACAA for the navi and 0x0800B1C0 for an obstacle. Neither has an EXE3 counterpart, and the loop's
  instructions are nowhere in EXE3's ROM. It is EXE4's own, close to EXE5's and EXE6's (both maps: similar).
- **The hit test** (EXE4 0x08012AFC, similar 0.76 to EXE6's `sub_3007218`) is in ROM like EXE5's, while EXE3's and
  EXE6's are in IWRAM. EXE3's (0x03007020, with its body-flag checks 0x030070C8 to 0x0300710C) has no counterpart
  in EXE4. EXE4's registration also scales the damage by element against element (a shift from the 5 × 5 table at
  0x08012CA4); EXE3's keeps the damage by element for later.

### 4.2 The flow

EXE3's battle mode runs a **byte** state at [r5 + 1] with a halfword substate at +2. At the round's end
(0x08009430), result 1 (win) goes to state 0x10, 2 (loss) to 0x14 and 5 to 0x18 (Tango's set_win and set_loss,
0x0800946A and 0x08009472); result 3 takes another path (0x08009438). EXE4 runs a **word** state at [r5 + 0] and
counts the round in BattleState+0x18 (wins) or +0x19 (losses) before going to 8 or 0xC (0x08007124 to 0x08007144).
None of EXE4's flow is in EXE3; its loop is EXE6's, similar (exe4-map.md §4).

### 4.3 The effects (the rules' `effects` section)

- **Shake: EXE3's design, which EXE5 kept.** EXE3's shake (0x08022AE4) and EXE4's (0x08025FE0) are the same routine:
  one channel, two draws from RNG2 each shaking tick, held while paused, which is the engine's `battle_rng`.
  EXE6's is the other rule (`console_rng`). EXE4 adds one condition: it tests time stop and pause only when the
  toolkit's first record's byte is 8 (0x08025FE2). EXE3 always tests a flag (0x080149BE) and its pause (0x0801497C,
  EXE4's `battle_isPaused` with other constants) at 0x08022AE6.
- **Damage word**: §4.1. EXE3 has none to state.
- **Retype** (EXE6's `sub_801A082`, EXE4's 0x08012ED0) has no EXE3 counterpart. Palette flash and spark steps were not
  compared (?).

### 4.4 NaviStats fields

EXE3's fields, by the code that reads them (`tools/exe3/navistats.py`), against EXE4's (exe4-map.md §3.3):

| EXE3 | EXE4 | What | How read |
|---|---|---|---|
| +0x13 | +0x12 | chips dealt: 5, 6 in Custom style | EXE3's init and the program effects that add to it (0x0803C9E4, 0x0803C9F8), whose EXE4 twins (0x08041B18, 0x08041B30, similar) add to +0x12; read at the battle's start (0x08006D46) |
| +0x14 | +0x13 | the Mega chip limit: 5, 6 in Team style | the init (0x0804737C); read with +0x15 by the folder checks (0x08001950, 0x080353A8) |
| +0x15 | +0x14 | the Giga chip limit: 1 | the init (0x08047384) |
| +0x19 | +0x15 | random encounters off when nonzero | EXE3's 0x080146BC is EXE4's 0x080F5370 (similar 0.77; EXE6's `sub_80AA4C0`): the same read at the same place |
| +0x1A | +0x16 | random encounters (?) | the same routine |
| +0x1B | +0x17 | the encounters' elements: EXE3 an index (0xFF none), EXE4 a mask (default 0x1F) | the same routine |
| +0x2C (halfword) | ? (EXE4's HP: +0x30 to +0x34) | an HP amount (?): the HP programs add to it | EXE3's init (0x080473CC) and 0x0803CB68 on; the record carries HP apart (§4.5) |
| +0x01 to +0x04, +0x18, +0x1C to +0x1E, +0x21 | the same | set to the same value by identical effect setters (e.g. EXE3 0x0803C978 = EXE4 0x08041B0C) | the setters' constants; what each is, ? |

So **EXE4's +0x15 to +0x17 are overworld encounter fields, not battle ones**, and +0x17's 0x1F is a mask of
elements. EXE3's block has no mood: no accessor call names its +0x00. EXE3's battle navi init (0x080B02E8) reads its
sides' +0x01 to +0x04, +0x0B, +0x0D, +0x10, +0x1C, +0x29 and +0x2A. When +0x2A is nonzero, it sets the collision's
+0x06 to 8 and +0x16 to 100 (0x080B048C), likely a barrier of 100, which would make it the twin of EXE4's aura at
+0x21 (?).

### 4.5 The link record

EXE4's netbattle record is EXE3's design (R):

| | EXE3 | EXE4 |
|---|---|---|
| built by | 0x080071F0 | 0x08008708, at 0x0203BD40 |
| the sender's RNG2 | +0x0C | +0x04 (+0x00 the magic 0x12345678) |
| MegaMan's stat block (0x40) | +0x10 (0x08007558) | +0x0C |
| HP, max HP | +0x08, +0x0A | ? |
| received at | 0x02036830 + 0x110 side | 0x0203E390, 0x0203E490 |
| unpacked by | 0x08007288 (stat block to 0x02037920 + 0x40 side; style and HP to 0x020384D0 + 0x88 side) | 0x080087A8 (stat block to 0x0203BEC0 and 0x0203BCC0; side 1's when 0x08007E46() ≥ 0x46) |
| RNG2 seeded from | side 0's record (0x080065DA) | side 0's record, 0x0203E394 (0x080087DA) |

EXE4's record carries more besides:

- BattleState+0x3C at +0x08;
- 0x2C bytes each from 0x02001610 (+0x4C) and 0x02007230 (+0x78);
- 0x10 bytes from the toolkit's +0x64 (+0xA4);
- two words from the toolkit's +0x44 record's +0x20 and +0x24 (+0xB4, +0xB8);
- 8 bytes from 0x02035CA0 (+0xBC).

In its copy of the stat block, +0x2A is cleared when 0x08007E46() ≥ 0x46 and event flag 0x1184 is set. Before the
copy, MegaMan's light/dark (+0x36) is set back to 500 when 0x08006570 is true. EXE6's
`battle_copyStructsIncludingBattleStats_800b2d8` copies its 0x64-byte blocks out of the records the same way (its
RNG not read here). So the record design (the stat block and the sender's RNG2 in it, side 0's RNG2 seeding the
battle) is EXE3's, kept by EXE4, while the transport that carries it is new in EXE4.

## 5. For the port

- EXE3 is no source of answers for what EXE4 does its own way: the actors, collision, the damage word, the push, the
  hit test, the flow, the custom screen, Double Soul, the dark chips and the emotions are EXE4's, absent in EXE3.
  They are to be read from EXE4's code (exe4-map.md §13).
- Where EXE4 is EXE3's (the pools, the element at +0x17, the panels, the camera shake, the NaviCust, the stat block's
  shape, the link record), an EXE3 port later would share the engine's generalizations with EXE4. Nothing of EXE3's
  own battle code is in the engine.
- Settled here for the EXE4 port:
  - the +0x17 element is EXE3's place;
  - EXE4's +0x15 to +0x17 stat fields are encounter fields;
  - the link record's layout and RNG2 seeding (§4.5);
  - the shake is EXE3's and EXE5's `battle_rng`, with EXE4's added mode check.

Open (?): the meaning of the stat fields set identically in both (+0x01 to +0x04, +0x18, +0x1C to +0x1E, +0x21); the
palette flash and spark rules against EXE3; the Japanese and Blue/Black ROMs (only A6BE is mapped; Tango shows the US
versions' battle hooks at the same addresses).
