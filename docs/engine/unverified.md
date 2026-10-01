# What the chip lab hasn't verified

The engine is ported branch by branch from the disassembly; the chip lab's recordings of the original verify what
they reach. This file lists, by area, the branches the engine docs mark unverified, and what has since been
recorded for them. A row leaves the list of open items when a lab scenario reaches it and the engine matches every
frame of it.

## Field objects, instant and standard chips

Scenarios named here are in the verification workspace's chiplab library (`stages/…`, `forms/…`, `chips/…`).
"Matches" means every frame of the recording. The library's README says how a scenario's seed picks its stage
(the battle settings record): every record below is reached by a computed seed.

### Field objects and stages: covered

| Branch | Scenarios | Result |
|---|---|---|
| Every battle settings record of a link battle (match type 1, records 0x00 to 0x5F) whose actor list places objects: rock cubes 0x02 to 0x07 and 0x5C to 0x5F (the poison stage), boulders 0x16 to 0x1B, ice blocks 0x28 to 0x2D and 0x50 to 0x53, Guardian statues 0x54 and 0x55: both orders of each list (side 1's navi first), every position | `stages/rocks-02-cannons` to `rocks-07-drilarm-side1`, `boulders-17-fan` to `boulders-1b-airshot-side1`, `boulders-stand`, `boulders-corners`, `iceblocks-28-thunder` to `iceblocks-2d-dust`, `ice-50-rockcube-airshot`, `ice-52-icecube`, `ice-53-dust`, `statues-stand`, `statues-55-airshot-side1`, `poison-5c-stand` to `poison-5f-tornado`; 0x51 is the chips' `stage-ice`, 0x2A the soundmod trace's | match |
| A rock broken by damage: the cube (debris palette 0, its sound) and the ice block (palette 1, its sound); by a breaking hit, from side 1 too | `rocks-02-cannons`, `iceblocks-2b-cannons`, `rocks-07-drilarm-side1` | match |
| Elements on an ice block (aqua): elec doubles, fire doesn't | `iceblocks-28-thunder`, `iceblocks-29-firebrn` | match |
| The bounded push (`sub_8017E26`): a rock or boulder slides until another obstacle is next; from side 1 (mirrored); to the field's edge (WindRack's gust); over ice panels as a rock (a panel more) and as an ice block (aqua: none); into a navi, which crushes the rock and hurts the navi | `rocks-03-airshot`, `boulder-airshot`, `boulders-1b-airshot-side1`, `rocks-06-windrack`, `iceblocks-2c-airshot`, `ice-50-rockcube-airshot`, `ice-52-icecube` | match |
| The knock-back (`sub_8017CC0`, the fan's and the statue's table entry): a fan hit by AirShot with a navi behind it (blocked: it rests where it is); a stage statue hit by AirShot (the damage breaks it first, and it strikes back) | `fan-airshot`, `statues-55-airshot-side1` | match |
| The registry: a third class-0 object on a side evicts the oldest, the stage's rock (HP 0: it breaks into debris); Guardian's statue evicts the stage's (class 1) | `rocks-05-rockcube-evict`, `statue-guardian` | match |
| RockCube on an occupied panel (the rock breaks as it stands); CrakShot at a rock's panel (cracked, nothing flies) | `boulder-rockcube`, `poison-5d-crakshot` | match |
| A rock's lifetime: 6000 ticks counted outside dimmings and pauses, blinking its last 180, then broken | `rockcube-lifetime` (6,700 frames) | matches |
| IceCube (chip 0x17C, which the library doesn't have: the lab puts it in the folder) | `ice-52-icecube` | matches |
| Absorption (DustCross's B+Back): rock cubes, ice blocks (their animation), boulders (the stage slots), statues, a TimeBom's bomb (it keeps its hidden digits; thrown, its parts keep their own facing); each thrown back | `rocks-04-dust`, `iceblocks-2d-dust`, `ice-53-dust`, `boulder-dust-absorb`, `statue-dust-absorb`, `timebom-dust-throw` | match |
| An absorption interrupted: the navi flinches out of the action while the obstacles fly; they fly on, their timer stopped, and puff off the field | `absorb-interrupted` | matches |
| The boulder (field-objects.md §3.1): its spawner's flags on columns 2 and 5, standing through the round's start, hits, breaking, a breaking hit, the push, the stage slots | `stages/boulder*` (ten scenarios) | match |
| The boulder leaving by a chip: blinking out (ColArmy makes both sides' field objects vanish: 20 ticks in its destroyed action), swallowed by BlzrdBal's ball (removed: the puff); DustMan with boulders on the field | `boulder-colarmy`, `boulder-blzrdbal`, `boulder-dustman` | match |
| The stages' Guardian statues: standing (still running while paused), struck by either side (it strikes back at the attacker), absorbed, evicted, blinking out | `stages/statue*` (eight scenarios) | match |
| The knock-back with room behind: the fan slides a panel and rests | `fan-airshot-free` | matches |
| Panel changes under obstacles: Geddon's cracks under boulders; a Tornado on a rock; a BigBomb's blast | `boulders-18-geddon`, `poison-5f-tornado`, `boulders-19-bigbomb` | match |
| GroundCross's falling rocks (attack #0x1D) over the holes stage: a rock over a hole puffs, one over a panel shatters | `forms/falzar/cross-ground-charged-holes` | matches |

### Field objects and stages: not reachable in a netbattle (documented, no scenario)

| Branch | Why |
|---|---|
| The rock's `fall` entrance (`sub_80CFAC0`) and its rows 0 (1 HP) and 2 (500 HP) | `sub_80CFBC4` has three callers: the actor lists' type 8 (entrance 3; the lists name rows 1 and 3 only), RockCube's controller (the chip's parameters: RockCube row 1, IceCube row 3, both rising) and the encased obstacle (row 3, instant). No chip record or list gives entrance 2 or rows 0 and 2. |
| The boulder's spawn flags on columns other than 2 and 5 | Every list places its boulders on those columns, and the flags are only read at the spawn. |
| The thrown and encased obstacles (`sub_8018002`, `sub_801813A`; field-objects.md §4.5) | Nothing calls the routines that request them. |
| An obstacle's actor reactions (its table's entries 3, 4, 6 and 7: flinch, paralysis, ice, bubble) | They dispatch through actor data an obstacle doesn't have; nothing sets them on one. |
| The actor lists' entry types 1, 2, 6, 7 and 0xA; battle settings records 0x60 and up | No link battle's stage has them (match type 1 draws from 0x00 to 0x5F; types 0 and 2 need another match type, and name the same lists). |
| The rock's and the boulder's "no collision slot" paths, an absorbed obstacle with no navi to fly to | A full pool; the side's first actor is always the player's navi. |

### Open

- Field objects: a navi's absorbed list full (eight obstacles: more than a round sets up); the push's branch
  for a hit from both sides at once, and its pull back toward the pusher's side (Fan's gusts don't move a boulder
  off its row's far end: `boulders-17-fan`); the knock-back over ice; a falling rock breaking on what it hits
  (`sub_80C7E24`'s hit branch: no recording takes it, though the rocks' hits land).
- Instant chips (`off_80EC3F0`): the boomerang's turns at the field's edges and its hits on the way back, the
  lance's panel checks, SandWrm's hole and spray against obstacles and holes, GolmHit's cracks by panel type,
  JustcOne's landing waves, ColForce's soldiers and ColArmy's shots against obstacles, FullCust's and the plus
  chips' special-source branches (not reachable: per-player gauges and the special chips are not a netbattle's).
  Effects 2, 6, 9, 11, 16 and 17 wait for the link navis' weapons, their only callers.
- Standard chips (standard-chips.md): Static's larger spreads (a bugged NaviCust), the shock wave's rows, the
  thunder ball's bug, GunDelEX's wide beam, the recovery chips without a target, the Cross special's chips through
  the by-number shims.
