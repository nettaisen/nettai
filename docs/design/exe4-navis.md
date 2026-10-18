# EXE4's other navis under direct control: what the ROM has, and a proposal

Phase 1 of the user's request (2026-10-18: "direct control"): make EXE4's other navis playable in a netbattle the way
EXE5's team navis and EXE6's link navis are. This document records what US Red Sun (B4WE, `exe4_rom_e.srl`) holds
for them and proposes the controls. Nothing is ported yet. Addresses are B4WE's. "L" is a navi's level, NaviStats
+0x25 (§3). **ROM** marks what the ROM states. **Invented** marks what the port would choose. **Guess** marks a reading
not yet confirmed.

## Summary

| # | Navi | Its AI's moves (actions) | Proposed B / charged / B+Left | Pack today |
|---|---|---|---|---|
| 1 | Roll | whip 0x2C, arrow 0x2D, move 0x2E, move-to 0x30 (0x2F: an enemy's) | buster / arrow / whip | sprite, whip and arrow sounds, ribbon, arrow |
| 2 | GutsMan | move 0x43, quake 0x44, punch 0x45, machine gun 0x46 | buster (six presses: machine gun) / quake / punch | all but the machine gun's row |
| 3 | WindMan | move 0x3F, gust 0x40, prop bombs 0x41, tornados 0x42 | AirShot / tornados / gust | gust debris sprite 10-2A and sound 0xB2 missing |
| 4 | SearchMan | move 0x3A, seek bombs 0x3B, shots 0x3C, scope 0x3D | buster / shots / scope | all |
| 5 | FireMan | move 0x48, 0x49, fire bombs 0x4A, flamethrower 0x4B, 0x4C | buster / flamethrower / 0x4C | 0x49's sound 0x8F, 0x1A's and 0x21's sprites to check |
| 6 | ThunderMan | move 0x55, lightning 0x56, balls 0x57 (+ clouds, actor 0x33) | buster / balls / lightning | balls' sprite 10-2E, clouds missing |
| 7 | ProtoMan | move 0x5B, slashes 0x5C, 0x5D, 0x5E, 0x5F | buster / WideSwrd slash / guard | sword parts in content; 0x5E/0x5F to read |
| 8 | NumberMan | move 0x48, wait 0x4D, balls 0x4E, boomerang 0x4F, dice 0x50 | buster / dice / number balls | boomerang's sprite 10-07 missing |
| 9 | MetalMan | move 0x65, blade 0x66, spray 0x67, fist 0x68, gears 0x69 | breaking buster / fist / blade | blade 0C-09 and spray 0C-49 sprites missing |
| 10 | JunkMan | move 0x6D, junk 0x6E, press 0x6F | buster / press / junk | 10-3A, 10-3B and sound 0x14A missing |
| 11 | AquaMan | move 0x51, 0x52, bombs 0x53, water gun 0x54 | buster / 0x52 shot / water gun | 0x52's sprite 10-0F missing |
| 12 | WoodMan | move 0x6A, tower 0x6B, leaves 0x6C, wait 0x4D | buster / wood tower / leaves | move's 0x94 and 0x95 to check |
| 13 | (generic navi) | none: **dropped** (§8) | — | — |
| 14 | HealNavi | none: **dropped** (§8) | — | — |

Twelve navis are in scope. Navis 13 and 14 are left out because the ROM has no player-side code for them (§8).

Three findings shape everything below:

- **The ROM already has the hook direct control needs.** A navi's idle action picks its routine by controller and
  navi (§1.1). Under controller 0, navis 1 to 12 run MegaMan's own button idle with their own sprite. Their buttons
  are whatever weapon routines NaviStats +0x09, +0x0A and +0x0C name. Each navi move reads its parameters from the
  same attack block a weapon routine fills (AIData +0x70). One unused weapon routine, 0x18, already starts WoodMan's
  tower (action 0x6B) from a button: 80 damage, wood, counter byte 0xBC. **So direct control means new weapon
  routines that start each navi's own actions.** That is the shape the ROM itself uses.
- **The level is HP / 100.** Every navi's damage and timing tables are indexed by NaviStats +0x25. The navi init sets
  +0x25 to the base max HP / 100, at least 1 (0x0800E818, also 0x08007AA4). Under the max-power rule (HP 1000) that
  is level 10, the top entry of every table. **ROM.**
- **The operated mode names each navi's signature move.** Controller 2's command system has a "B" command and a
  "special" command for each navi (§1.3). The proposal follows them where it can.

## 1. How the ROM runs a navi that isn't MegaMan

### 1.1 The idle's dispatch

The player's idle (action 6, 0x080EB0CC) returns at once if the battle is over. When NaviStats +0x26 is 1 and the
request word (0x0800BFD0) has 0x600, it starts that request (0x0800C780). Otherwise it calls an entry of
`0x080EB12C[+0x26][+0x23]`:

| Controller (+0x26) | Table | Navi 0 | Navis 1–12 |
|---|---|---|---|
| 0, the player | 0x080EB138 (13 entries) | 0x080EEB38 | 0x080EEB38, the same |
| 1, a story navi / auto battle | 0x080EB16C (13 entries) | 0x080EED48 (auto battle AI 0x0800E4F8) | each navi's own decoder (§2) |
| 2, operated | 0x080EB1A0 | 0x080EED7C | 0x080EED7C, the same |

0x080EEB38 is the button idle that MegaMan's content ports. It reads the decode's requests: buster, charged, B+Left,
the chip, and the `0x10` request. It starts the weapon routines that NaviStats name. It never reads +0x23.

Navi 13 under controller 0 reads past the table into controller 1's entry 0 (the auto battle AI). Navi 14 reads
Roll's decoder. Neither is a navi the ROM plays (§8).

**Buttons.** Only controller 0 decodes the pad (0x0800BDE0). Controller 2 reads Right/Left and B for its column
marker (0x0800BF1C, 0x0800BCD4). None of the twelve controller-1 decoders calls an input reader. They read RNG2
(0x08001B16), panels, the enemy's position and HP, their own HP and counters.

### 1.2 Controller 1: the story's navis

The setup's entry type 4 (0x080068B2 → 0x08006950) places a story navi. It reads the side's record at 0x0200AB10
(four bytes a side, filled by 0x0800EB98): byte 0 is the navi, byte 1 the save's NaviStats block (1 to 7), and
byte 2 the sprite variant. It copies that save block to the side's battle NaviStats. It also copies the block's
0xE0 bytes of auto battle data to 0x020358B0 (shuffled by 0x0800F208). Then it writes +0x23 = the navi, +0x24 = 0
and +0x26 = 1, and spawns actor 0 with +0x0F = 0xF0 + the navi.

**A story navi's HP and level are save data.** The save blocks 1 to 7 hold them; the ROM has no table for them.

Every decoder has the same frame:

- If the side's auto battle flag (AIData +0x50's +0) is set, it runs the chip use (0x0800E328), the machinery
  content/exe4/rules/auto_battle already ports for the dark soul.
- Otherwise it runs a state machine on AIData +0x64 (state +0, sub-state +1, counters +8, +9, +0xA).
- To use a chip, a decoder sets that flag (0x0800EB8E).
- Each move fills the attack block at AIData +0x70 and requests one of its actions (0x0800C9AC).
  - The block's fields: +8 damage (some write a word: damage | parameter << 16), +0xA counter byte, +0xC/+0xD
    extras, +0x18 a wait.
  - It may also set +0x1C, a panel callback. Common callbacks: 0x0800F734 a random own panel, 0x080F077C the
    closest panel in the enemy's row.

The story enemy versions of these navis (§3.2) run the same action routines through their own actor tables. For
example, Roll's actor table at 0x080BC1A0 lists actions 0x2C to 0x30. So every action is reached in the story by
two kinds of object.

### 1.3 Controller 2: the operated mode

The operated mode is subsystem 0x0E's battle (verify `docs/exe4-completeness-audit.md` §C), which the user has not
asked for. All navis share one idle there (0x080EED7C). Its command system (0x080222B4) calls a routine from a table
by command kind and navi (0x08022324):

- **Kind 1, "B"** (0x08022384, by navi): the buster (action 8, 0x08022468) for every navi but ProtoMan. ProtoMan's is
  a guard (action 0x25, +0xC 0x13C, 0x080224D0).
- **Kind 2, "special"** (0x080223B8, by navi):

| Navi | Special command |
|---|---|
| MegaMan | guard 0x23C |
| Roll | arrow 0x2D, +0xC 6 |
| GutsMan | quake 0x44, +0xD 1 |
| WindMan | the wind on (effect 0x41) |
| SearchMan | shots 0x3C |
| FireMan | flamethrower 0x4B, element fire |
| ThunderMan | balls 0x57, element elec |
| ProtoMan | sword chip action 0x0A, +3 2 |
| NumberMan | guard 0x23C |
| MetalMan | blade 0x66 |
| JunkMan | press 0x6F |
| AquaMan | 0x52, element aqua |
| WoodMan | tower 0x6B, wood, +0x18 10 |

Their damage comes from small tables by command and navi (0x08022334: B 1,1,3,2,1,2,2,0,1,3,2,1,3; special
0,5,30,0,2,10,10,10,0,10,10,5,10), scaled by something not yet read (NaviStats +0x2B is involved). Unread.

## 2. The navis, one by one

Damage is at level L; under HP 1000, L = 10. Ticks are frames. "Step" is the shared move pattern: animation 4 for 3
ticks, the panel changes, animation 3 for 3 ticks. Sounds and sprites are named as compat/assets.toml names them;
"missing" means the pack lacks it.

### 2.1 Roll (navi 1; decoder 0x080EF624)

**AI.** Up to 3 moves (0x2E), then up to 4 chips, then a move to a panel by the enemy's row (0x30, callback
0x080EF7BE). Then, if she got there, RNG2 & 15 picks from 0x080EF6E0: the arrow 10 times in 16, the whip 6. If she
didn't get there, the whip.

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x2C | Whip | Warps to a panel in front of the enemy (0x080EFF98, random among valid ones) and waits 0x080EF724[L] (30/25/20). Lashes with her ribbon attached (attack 0x02 row 7), roll-whip (0x131) every 8 ticks. A hitbox (attack 0x03, hit 0x04050001) on the panel ahead every 15 ticks, 3 times. Then warps back. Damage 0x080EF72F[L]: 10 ×4, 20 ×3, 40 ×4 (L10: 40 a hit). |
| 0x2D | Arrow | Animation 7, an arrow (attack 0x0B, the flying shot RollSoul's arrow uses) 8 pixels ahead, roll-arrow (0x132), then 30 ticks. Damage 0x080EF77B[L]: 30/60/120. +0xC = 0x080EF770[L] (6/7/8: the shot's row or speed, guess). Counter byte 40. |
| 0x2E | Move | A step to a random own panel. |
| 0x30 | Move to a panel | A step to the callback's panel. |
| 0x2F | Summon | Not in her decoder; her enemy actor's alone. Goes to a column, animation 0xF, summons enemy record +0xC on a random panel (actor spawn 0x080F00F4, point-appear 0xCE). |

**Hooks.** No init hook (0x0800DB38[1] returns at once).

### 2.2 GutsMan (navi 2; decoder 0x080F01E8)

**AI.** Steps (0x43; the wait 0x080F01C4[L] 50/42/36 in +0x18) until a count of 0x080F02F4[L] (5/4/3), with a 4 in 16
chance after each step of a quake. Then, if the panel in front of the enemy is valid and his side's, the punch or the
machine gun (one RNG2 bit). Otherwise a chip.

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x44 | Quake | Damage 20·L (10·L when his object's param1 is 2). Counter byte 30. +0xD = 0x080F01DA[L]. Animation 0xA for 40 ticks, then gutsman/quake (attack 0x1C, in content) and a hitbox ahead (hit 0x04050601). |
| 0x45 | Punch | Approaches the closest panel in the enemy's row (0x080F0783). Damage 20·L, counter 50, +0xC = 0x080F01CF[L], +0xD = 0x080F03B8[param1]. Animation 0xF for 20 ticks, a hitbox (region 0x21), fist-swing (0xA4), and attack 0x1E: GutsSoul's punch, lib/punch. |
| 0x46 | Machine gun | Approaches (0x080F07D0). Damage 5·L, counter 10. Animation 7 and five flying shots (row 5), machine-gun (0xA7), every 10 ticks. |

**Hooks.** None.

### 2.3 WindMan (navi 3; decoder 0x080F0880)

**AI.**

- **Each idle tick** he keeps a wind object (effect 0x41, the wind stages' wind) in his object's +0x60
  (0x080E68F8, r4 0).
- **Three rounds:**
  - with RNG2 & 15 ≥ 6, a step (0x3F), waiting 0x080F0970[L] (60/50/40), plus 30 when cornered on a coin flip;
  - otherwise the gust.
- **Then** the prop bombs up to 5 times, each after a step, and else the tornados after a step to 0x080F0F1C's
  panel.
- **Chips** go between rounds.

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x40 | Gust | Damage 20·L, counter 30, +0xC = 0x080F09F8[L] (halfwords 0x4012/0x480F/0x500C: count and interval, guess). +0xE is 1, or 3 when his HP is half or less (RNG). The wind on, a palette blend (0x08000AAA), attack 0x63 again and again (sprite 10-2A and sound 0xB2: both missing), then 60 ticks. |
| 0x41 | Prop bombs | The wind on, animation 7, two throws of attack 0x65 (prop-bomb's sprite 10-2B and sound 0x133). Damage 20·L, +0xD = 0x080F0A70[L] (0x10/0x18/0x20), 40 ticks. |
| 0x42 | Tornados | The wind on, animation 0x10, three tornados (attack 0x67: windman-tornado 10-2F, tornado 0x126). Damage 20·L, 50 ticks. |

**Hooks.** The init hook (0x0800DB72) spawns two actor 0x5C (r4 2 and 3): windman/swirl, the head fans WindSoul's
image already uses.

### 2.4 SearchMan (navi 4; decoder 0x080F1674)

**AI.**

- **Steps:** up to 3 (0x3A, wait 0x080F17D8[L] 26/24/22), then a chip, up to 4 times.
- **Then:**
  - when the enemy is in line (0x080C1430) and he isn't pinned, the seek bombs;
  - otherwise a move in (0x080C148C) and the shots.
- **Every fourth round** with the enemy in line, the scope.

Damage words (0x080F17E4, by mode): seek bombs 20·L | 30 << 16; shots 5·L | 10 << 16; scope 20·L | 40 << 16. +0xD is
2 at L ≤ 3, else 3.

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x3B | Seek bombs | Turns invisible (invisible 0x109, 0x0800C126), 30 ticks, then a 40-tick loop. minibomb-throw (0x90) and attack 0x52 (seek-bomb's sprite 10-24) at RNG-picked panels near the enemy. |
| 0x3C | Shots | Moves in, animation 0xF for 20 ticks, a mark (effect 0x31) on the enemy's panel. Animation 0xA, vulcan (0xBB), serchman/shot (attack 0x57, in content) ×3, every 10 ticks, then 30 ticks. |
| 0x3D | Scope | Invisible, the scope (effect 0x2E: search-scope 10-22, scope-move 0x128) for 240 ticks. Then animation 7 for 30 and 60 ticks, and visible again. SearchSoul's charged scope is MegaMan's version of it (action 0x3E, in content). |

**Hooks.** None.

### 2.5 FireMan (navi 5; decoder 0x080F1DE8; read coarsely)

**AI.** Steps (0x48, the move NumberMan shares, with several callbacks) and chips. Then one of: the flamethrower
(timer 0x080F1DC4[L] 22/18/14), the fire bombs (0x080F1DCF, 0x080F1DDA), 0x4C after an approach (damage 30·L,
counter 10), or 0x49 (damage 10·L, counter 10).

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x4B | Flamethrower | Approaches, animation 0xF, then animation 0xA for 60 ticks, fireman/flame (attack 0x09, the FireMan chip's, in content; param 0x00F03C05), roar (0xA8). |
| 0x4A | Fire bombs | Animation 7, attack 0x1A up to 5 times (+0x12), 30 ticks apart. |
| 0x4C | Fire tower (guess) | Animation 0x10, hockey-puck (0xC0), attack 0x21, 50 ticks. |
| 0x49 | Shots (guess) | Animation 6, 21 ticks, attack 0x1F (bomb's sprite 0C-02) ×2, sound 0x8F (missing), 40 ticks. |

**Hooks.**

- The init hook (0x0800DB9C) puts on a form overlay (actor 0x57, 0x810): his flames, as FireSoul wears.
- The flinch and drag hooks (0x0800DC9C, 0x0800DD82) restart that overlay through his object's +0x60.

### 2.6 ThunderMan (navi 6; decoder 0x080F2430)

**AI.**

- **Steps:** 0x55 when 0x080C398C allows.
- **Every 4 steps** with the enemy reachable (0x080C398C), the lightning.
- **Every 3,** a chip and then the balls (0x080C3A44).

Damage words (0x080F24E0): balls 20·L | 20 << 16; lightning 20·L | 30 << 16.

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x56 | Lightning | Animation 0xA, three strikes of thunman/thunder (attack 0x71, in content). |
| 0x57 | Balls | Animation 7, eight of attack 0x70 (sprite 10-2E, missing). |

**Hooks.** The init hook (0x0800DBAC) spawns actor 0x33 by his level and controller: his clouds, which spawn actor
0x32 and attacks 0x6A and 0x6F. None of these is in the pack or content.

### 2.7 ProtoMan (navi 7; decoder 0x080F2FAC)

**AI.** He uses his enemy actor's helpers (0x080C5D64 to 0x080C5F58, in actors 0x37/0x38's code).

- **Default:** a move (0x5B) to a panel those helpers pick.
- **Every 3 moves:** 13 times in 16 a chip, else 0x5D.
- **Every 5 rounds:** 0x5F if the enemy navi's HP is 500 or more, else 0x5E. 0x5C runs only when his HP is above
  his max.

Damage words (0x080F30B0 by mode): 0x5C 20·L | 30 << 16; 0x5D 20·L | 20 << 16; 0x5E and 0x5F 30·L | 50 << 16.

**Moves.** Moves 0x5C to 0x5F step or warp, then start a saved action from their data (+0x1A: "requests of an action
that is data's" in the audit).

| Action | Move | What the ROM does |
|---|---|---|
| 0x5C | Slash | Animation 5, the blade attached (attack 0x02 row 3), a hitbox, sword-swing (0x8E). |
| 0x5B | Move | Spawns attack 0xA4 (collision only: his guard while moving, guess) and effect 0x31 (row 3). |
| 0x5E | (to read) | Steps to the enemy area's middle (4, 2), then a slash. |
| 0x5F | Delta-Ray-like (guess) | Animation 0xF, an effect, log-in (0x77), then slashes. DeltaRay's chip is in content. |

**Hooks.** The init hook (0x0800DBE4) calls 0x080F30B0(2) by level. To read.

### 2.8 NumberMan (navi 8; decoder 0x080F361C)

**AI.** Steps (0x48) and waits (0x4D: animation 0, 4 ticks), then one of three attacks.

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x4E | Number balls | Damage 20·L, counter 30, +0xC = 0x080F37C4[L]. Animation 7, attack 0x0C (number-ball 10-05), up to 5, 8 ticks apart. |
| 0x4F | Boomerang | Damage 20·L, +0xC 3, +0xD from a table, counter 30. Animation 0xA, boomerang (0xAB), attack 0x0D (sprite 10-07, missing). |
| 0x50 | Dice | Damage 5·L, counter 50. Animation 6, minibomb-throw (0x90), a die (attack 0x05, in content as NumberSoul's charged die; table 0x080F3350). |

**Hooks.** The init hook (0x0800DC18) spawns actor 0x5C (r4 0): numbrman/face, in content.

### 2.9 MetalMan (navi 9; decoder 0x080F3F04)

**AI.**

- **Each idle tick** he sets flag1 0x20030 (super armor-like; to read).
- **Moves:** step 0x65, by 0x080C7A90 and 0x080C7B48.
- **Attacks:**
  - by a count and RNG: the blade, the spray, the fist;
  - after the fist: the gears, then the battle-over check.

Damage words (0x080F3FE8): blade 20·L | 30 << 16; spray 20·L | 30 << 16; fist 30·L | 50 << 16; gears 20·L | 30 << 16.

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x66 | Blade | Animation 7 then 0x10, boomerang (0xAB), attack 0x8F (sprite 0C-09, missing). |
| 0x67 | Spray | Animation 8, attack 0x90 ×10 (sprite 0C-49, missing), spreader (0x8D). |
| 0x68 | Fist | Animation 0xF, metal-fist (0xB7), animation 0xA, a hitbox (hit 0x15050A01, 3 regions), metal-punch (0xA2), effect row 0x47. |
| 0x69 | Gears | Effect 0x65, GearChng's (chip 0x17C, not in the library). |

**Hooks.** The init hook (0x0800DC24) calls 0x080F3FE8(3) and 0x080C795C. To read.

### 2.10 JunkMan (navi 10; decoder 0x080F433C)

**AI.** Steps (0x6D) and chips. Then the press (0x080C9438 decides) or the junk (0x080C934C).

Damage words (0x080F4400): junk 20·L | 30 << 16; press 30·L | 40 << 16.

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x6E | Junk | Animation 0x12, attack 0x97 (sprite 10-3B, missing), hockey-puck (0xC0). |
| 0x6F | Press | Animation 7, then 0xA with sound 0x14A (missing), then 0xF, attack 0x98 (sprite 10-3A, missing; r0 0xF, r3 0x73), metal-punch (0xA2). The audit also gives it attack 0x9B. |

**Hooks.** The init hook (0x0800DC54) calls his enemy actor's 0x080C927C (random state at +0x60, +0x64) and
0x080F4400(2). The navi init gives navi 10 **no shadow** (0x080107C2).

### 2.11 AquaMan (navi 11; decoder 0x080F4988)

**AI.** Steps (0x51, flag 0x4000), then one of three attacks.

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x52 | Shot (guess) | Damage 10·L (to confirm), counter 20, +0xC 0. Animation 0xF, attack 0x23 (sprite 10-0F, missing), animation 7, an effect. |
| 0x53 | Bombs | Damage 20·L (10·L when param1 is 2), counter 30, +0xC = 0x080F4A98[L]. Animation 6, minibomb-throw (0x90), attack 0x24 (bomb's 0C-02). |
| 0x54 | Water gun | Damage 10·L (to confirm), counter 20. Animation 0xA, attack 0x27 (water-gun 10-10, the AquaMan chip's sprite). |

**Hooks.** The init hook (0x0800DC88) spawns actor 0x5C: aquaman/spout, in content.

### 2.12 WoodMan (navi 12; decoder 0x080F5124)

**AI.** Waits (0x4D, flag 0x4000), moves (0x6A, with a wait 0x080F5244[L] + 30), the tower, the leaves.

**Moves.**

| Action | Move | What the ROM does |
|---|---|---|
| 0x6A | Move | A sinking move that spawns attack 0x94 (and 0x95), metal-punch (0xA2). |
| 0x6B | Wood tower | Damage 20·L (to confirm), +0xC 3, counter 40. Animation 0xF, metal-punch (0xA2), animations 7 and 0xA, woodman/tower (attack 0x07, in content) ×3. **Unused weapon routine 0x18 (0x0800CF00) starts it too**: element 4, damage 80, counter 0xBC. |
| 0x6C | Leaves | Damage 10·L, counter 10, +0xC from 0x080F534C. Animation 0x12, three flying shots (attack 0x0B, rows from 0x080F50E4), sound 0x8F (missing). |

**Hooks.** No init hook.

## 3. Stats

### 3.1 What a player navi gets

- **Level.** +0x25 = base max HP / 100, at least 1 (0x0800E818 at the navi's init). At HP 1000, level 10. **ROM.**
- **HP.** The side's NaviStats block. Under the max-power rule every side has HP 1000, which also makes the level 10.
  **Invented** in the sense that the ROM's story navis have the save's HP.
- **Element.** The player code gives none. 0x0800C964 is by soul only. **Invented** if the port gives the navis their
  enemy records' elements: FireMan fire, ThunderMan elec, AquaMan aqua, WoodMan wood, the rest null.
- **Move lag.** By +0x25 and the navi (0x0800C208's table 0x0800C240). Every entry is 4. **ROM.**
- **The collision kind.** 0x0800C974 gives 3 for any navi but MegaMan, or with event flag 0x1187 set; MegaMan's is
  0. Its effect is to read.
- **Sprite.** 0x0800B90A by navi: 0x08017F98's byte pairs, category 8, entries 1 to 12 = 08-01 to 08-0C. All twelve
  are in the pack (the souls' images and navi chips use them). **ROM.**
- **Shadow.** None for navi 10 (0x080107C2). **ROM.**
- **Banners.** A win banner for navis 1 to 14 and a deleted banner for 1 to 12 (0x3C to 0xA0), all in the pack.
  **ROM.**
- **Emotion window.** The face table (0x08014B78) holds MegaMan's five faces and the souls' only. **No navi has a face
  in the ROM.** Open question (§9).

### 3.2 The story enemy versions (for reference)

The enemy records (0x08011888, 12 bytes) hold four versions of each navi, records 128 to 175. Their layout: +0 HP
with the element in the top nibble; +2 the actor; +4 the navi number; +5 the version; +6/+7 the status mark; +0xA a
byte of 30/60/120/200 (Roll's: the attack base, guess). HP by version:

| Navi | Actor | HP by version |
|---|---|---|
| Roll | 0x1D | 700/1000/1300/1500 |
| GutsMan | 0x0A | 500/900/1300/1700 |
| WindMan | 0x30 | 800/1200/1500/1800 |
| SearchMan | 0x2B | 900/1200/1500/1800 |
| FireMan | 0x0C, fire | 500/1000/1500/2000 |
| ThunderMan | 0x32, elec | 800/1200/1600/2000 |
| ProtoMan | 0x38 | 1000/1300/1600/1800 |
| NumberMan | 0x03 | 500/900/1300/1600 |
| MetalMan | 0x3F | 800/1200/1600/2000 |
| JunkMan | 0x47 | 800/1100/1500/1800 |
| AquaMan | 0x0D, aqua | 500/1000/1500/1800 |
| WoodMan | 0x46, wood | 1000/1500/2000/2500 |

## 4. Proposed controls

### 4.1 The shape

The shape follows EXE5's team navis (content/exe5/navis/*: `weapons = { buster, charge_shot, back_special }`,
`fresh`, `identity`, banners, mugshot) and the ROM's own (§1.1):

- **Movement** is the player's step (action 7), as controller 0 already gives navis 1 to 12. **ROM.**
- **B** is MegaMan's buster (weapon 0), as the operated mode's B command is for eleven of the twelve. **ROM.**
  WindMan's and MetalMan's follow their souls (below).
- **Charged shot** and **B+Left** are new weapon routines. Each fills the attack block as the decoder does at L = 10
  and returns the navi's action, as weapon 0x18 does for WoodMan.
- **Chips** are used as MegaMan uses them: the A button and the custom screen. Souls, Double Soul and the dark soul
  stay MegaMan's (navis have no `forms`).
- **No own chip.** EXE4 has no counterpart to EXE5's offered chip.

### 4.2 The table

| Navi | B | Charged (charge time) | B+Left | Left over |
|---|---|---|---|---|
| Roll | buster | arrow 0x2D, 120 | whip 0x2C, 40 ×3 | — |
| GutsMan | buster; six quick presses: machine gun 0x46, 50 ×5 | quake 0x44, 200 | punch 0x45, 200 | — |
| WindMan | AirShot (WindSoul's B, routine 0x6B) | tornados 0x42, 200 | gust 0x40, 200 | prop bombs 0x41 |
| SearchMan | buster | shots 0x3C, 50 ×3 | scope 0x3D, 200 | seek bombs 0x3B |
| FireMan | buster | flamethrower 0x4B | fire tower 0x4C, 300 | 0x49, fire bombs 0x4A |
| ThunderMan | buster | balls 0x57, 200 | lightning 0x56, 200 | — |
| ProtoMan | buster | WideSwrd slash (sword action 0x0A) | guard 0x25 (0x13C) | 0x5D, 0x5E, 0x5F |
| NumberMan | buster | dice 0x50, 50 × the face | number balls 0x4E, 200 | boomerang 0x4F |
| MetalMan | breaking buster (MetalSoul's B, routine 0x0B) | fist 0x68, 300 | blade 0x66, 200 | spray 0x67, gears 0x69 |
| JunkMan | buster | press 0x6F, 300 | junk 0x6E, 200 | — |
| AquaMan | buster | shot 0x52 | water gun 0x54 | bombs 0x53 |
| WoodMan | buster | wood tower 0x6B (weapon 0x18: 80, wood) | leaves 0x6C, 100 | — |

Where the charged move comes from:

- **The operated mode's special command:** Roll, GutsMan, SearchMan, FireMan, ThunderMan, ProtoMan, JunkMan, AquaMan,
  WoodMan.
- **The navi's soul's charged shot:** NumberMan's dice is NumberSoul's. MetalMan's fist is MetalSoul's routine 0x13.
- **The navi's soul:** WindMan's charged tornado follows WindSoul's charged WindRack in spirit only (invented).
- ProtoMan's guard on B+Left is the operated mode's B for him and EXE5's ProtoMan's B+Back. GutsMan's six presses
  are GutsSoul's input (0x0800BE50).

### 4.3 What is invented, separately from the ROM

1. **Every button assignment.** The ROM only proves the moves, the B and special commands (§1.3) and weapon 0x18.
2. **Charge times.** No navi charge row exists. Proposal:
   - where a soul has the same move, the soul's row: GutsSoul's 0x24 row (100 ticks a level, 60 at Charge 5),
     NumberSoul's die row, MetalSoul's fist row;
   - otherwise MegaMan's charged shot row.
3. **Damage at a fixed level 10** rather than the save's HP. It follows from HP 1000 (§3.1). The tables stand as the
   ROM's.
4. **Elements** from the enemy records, which the player path never reads.
5. **Leaving moves out** (the "left over" column), and the operated mode's own damage tables (§1.3), which are not
   used.
6. **The AI's approaches before an attack** (the punch's, the whip's warp, the flamethrower's): the proposal keeps
   them as the actions do them.
7. **Lockouts after each move.** Taken from the counter byte the decoder writes (+0xA) and the action's own wait.
   The weapon routines' +5 (chip lockout) is 0 unless chosen.

## 5. Rules impact (content/exe4/rules)

| Area | ROM | What changes |
|---|---|---|
| Anger and emotions | 0x0800C560 tests +0x23: MegaMan alone gets angry. | No anger for a navi. The emotion window has no navi face: open question. |
| Souls and Double Soul | The custom screen offers souls by emotion (0x0801E0B4). The souls' rows are MegaMan's sprites. | `forms` absent for navis. Double Soul off for them, as for EXE5's team navis. |
| Dark chips, dark soul | Offered by emotion 5 (0x0801DE90). The dark soul needs MegaMan's mood 0. | None for navis. Whether the ROM blocks a navi's dark chip (EXE5's 0x08010056) is to read. A light/dark-only chip (record +0x15) is to read by navi. |
| Full Synchro | The aura (0x0800D98A, EXE5's code) by owner. | The aura's animation for a navi is to read. EXE5 uses the AI index. |
| Banners | Win 0x08008534, deleted by navi. | The navi's own banners. |
| Deletion | A navi other than MegaMan dies by 0x08010D4C (effect object 0x11, a 90-tick explosion). | **Open item 51 of exe4-map.md §18 becomes reachable**: port that deletion. |
| Hit sound | 0x6D for +0x26 = 1, 0x6B otherwise (item 45). | Stays 0x6B under controller 0. |
| Hooks | The init hooks (0x0800DB38 by navi). The flinch, paralysis and drag hooks (0x0800DC9C, 0x0800DD14, 0x0800DD82): the identities' `overlay_hooks`. | Per navi. |
| Shadow | None for navi 10. | JunkMan has none. |
| Chip icons over the navi | 0x0800B9C0 by navi, table 0x0800B9E4. | Each navi's `chip_icons_at`. |
| Status mark | The actor record's +6/+7. | Which record a player navi's is: to read. MegaMan's is row 0. |
| NaviCust, patch cards | They write the side's block, whatever the navi. | Whether a navi's build takes them is the user's call (§9). |
| Run message | MegaMan's script and portrait. | A navi's portrait, if the ROM has one: to check. |

## 6. Verification plan

### 6.1 Putting a navi in a netbattle in the chip lab

The battle's NaviStats come from the save block as the init exchange sends it. The exchange line of a trace carries
both sides' whole 0x40-byte blocks.

**Tried here** (chiplab `rom_patch`, both consoles; scratch, not kept):

- **Patching the defaults routine** (0x0800D6D8: +0x23 = 2) changed nothing. A netbattle's block is the save's, never
  the defaults.
- **Patching 0x08007AA4** (the stats from the save at 0x08007AC2: +0x23 = 2, +0x25 = 10) changed nothing either.
  MegaMan still stood on both screens, and the exchange's +0x23 was 0. That routine is not on a netbattle's path.

**What would work:** a trap like chiplab's EXE5 `exe5_navi_traps` (src/lab.rs) at EXE4's init exchange, on both
consoles. It would write the side's block 0 in RAM ([sl+0x78]): +0x23 = the navi, and +0x26 = 0 or 1. Finding the
exchange's address is the first verify task.

Since the exchange carries the whole block, a console should learn the other side's navi from it. That the battle
stays in sync is **untested**.

### 6.2 Two kinds of recording

1. **Controller 0, navi n.** The ROM's own button-driven navi n, with MegaMan's weapons. This checks the sprite, the
   init hooks, the shadow, the flinch and drag hooks, the banners, the deletion (0x08010D4C), the buster and chips on
   the navi, and the emotion window as the ROM draws it.
2. **Controller 1, navi n.** The navi's AI fights. This checks every action frame by frame. The AI uses RNG2, which
   both consoles share, so it should replay.

For the engine to follow recording kind 2, port each decoder as the navi's own auto battle driver: content, Luau, a
small state machine each. The engine then plays the recording as the original does. That also gives the app an AI
for each navi. A cheaper alternative is a test-only harness that starts, on the engine's navi, the action the
recording's navi starts each frame (the trace's action field). It would verify the actions but not the AI.

### 6.3 The button mapping itself

The mapping has no oracle. Its parts are checked as follows:

- The actions are checked by the recordings above.
- The weapon routines are small setters with unit tests.
- One ROM-native check exists: NaviStats +0x0A = 0x18 makes WoodMan's tower the charged shot in the ROM (weapon 0x18).
  A recording with +0x23 = 12 and +0x0A = 0x18 covers the button path end to end for one navi.

## 7. Port plan (2–3 agents)

**Agent A: foundation and verify.**

- The chiplab trap (§6.1) and a sync probe, then the controller-0 recordings for the twelve.
- The engine and content pieces every navi needs:
  - the NaviDef entries: sprite, banners, identity, chip icon place, shadow, element, `fresh`;
  - the per-navi init hooks: WindMan's swirls, FireMan's overlay, NumberMan's face and AquaMan's spout are actor
    0x5C and the form overlay, already in content; ThunderMan's clouds are new, actors 0x33 and 0x32;
  - the deletion of another navi (0x08010D4C);
  - the rules' MegaMan-only tests as navi facts.
- The extractor additions for the missing assets: sprites 10-2A, 10-2E, 10-07, 0C-09, 0C-49, 10-3A, 10-3B, 10-0F, the
  clouds', any of 0x1A, 0x1F and 0x21's not yet named; sounds 0x8F, 0xB2, 0x14A, 0x96.

**Agents B (navis 1–6) and C (navis 7–12).**

- Each navi's actions in `content/exe4/navis/<name>/` with its weapons (charge, back), and, if chosen (§6.2), its
  decoder as its auto battle driver.
- Controller-1 recordings per navi.

**Shared code.**

- **A `lib/navi_moves` maker** (new): the step move with its panel callbacks (0x0800F734 random own panel; 0x080F077C
  closest in the enemy's row; the warp in front of the enemy, 0x080EFF98) and the level tables. Each navi builds its
  own moves from it (no lookup tables).
- **Existing kinds reused:**
  - the flying shot (attack 0x0B: Roll's arrow, GutsMan's gun, WoodMan's leaves);
  - lib/punch (GutsMan's punch);
  - gutsman/quake, fireman/flame, thunman/thunder, serchman/shot, woodman/tower;
  - number-ball, dice, the search scope, seek-bomb, prop-bomb, windman-tornado and water-gun sprites;
  - the hitbox (attack 0x03) and the attachment (attack 0x02).
- **EXE4's own:** every action routine 0x2C to 0x6F, the decoders, and attacks 0x0C, 0x0D, 0x1A, 0x1F, 0x21, 0x23,
  0x24, 0x27, 0x52, 0x63, 0x65, 0x67, 0x70, 0x8F, 0x90, 0x94, 0x95, 0x97, 0x98, 0x9B and 0xA4. Effects 0x2E and 0x65,
  and actors 0x32 and 0x33. Some have EXE5 or EXE6 counterparts by emap; check each before writing it fresh.

## 8. Navis 13 and 14: dropped

Both are incomplete in the ROM.

- **Navi 13** (NormalNavi's win banner 0x6C):
  - no controller-1 decoder (0x080EB16C has entries 0 to 12) and no init hook (0x0800DB38 has 13 entries);
  - under controller 0 it would run the auto battle AI by reading past the table;
  - no deleted banner;
  - its moves are only in story enemy actor 0x05's own state machine (records 200–203: HP 300/800/1200/230). That
    actor loads sprite 08-11, not the navi table's 08-0D (KendoMan's);
  - sprite 08-11 is not in the pack.
- **Navi 14** (HealNavi's win banner 0x70):
  - no decoder, no init hook, no deleted banner;
  - under controller 0 it would run Roll's decoder by reading past the table;
  - its moves are only in story enemy actor 0x1C (records 204–207: HP 300/800/1100/1400). That actor loads sprite
    08-14 in palette 1, not the navi table's 08-00 (compat's `healnavi`, whose name may be wrong);
  - sprite 08-14 is not in the pack.

Porting either would mean building a player navi from a story enemy's AI, with invented hooks and no deleted
banner.

## 9. Open questions for the user

1. **The button table (§4.2).** Each row's B, charged and B+Left, and the left-over moves.
2. **The emotion window for a navi.** The ROM has no face. Options: hide the window for navis, or draw MegaMan's (what
   the ROM would do, to confirm by recording).
3. **Elements.** Give FireMan, ThunderMan, AquaMan and WoodMan their enemy records' elements, or keep the player
   path's null?
4. **NaviCust and patch cards for navis.** Allow them, as the ROM's block allows, or not?
5. **Port the decoders as AI** (§6.2)? It gives frame-exact verification and an AI opponent per navi, at the cost of
   twelve small state machines.
