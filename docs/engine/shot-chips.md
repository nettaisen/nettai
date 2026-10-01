# Shot chip actions

The chip actions (`JumpTable80EAC60`, action ≥ 0x10) that fire a shot, and every object they spawn: the cannons,
AirShot, the Vulcans, the Spreaders, the BatCans, the TankCans, MachGun, CornSht, WideSht, the rapid buster (action
0x5D), SonicBom and Z Saver with their sonic boom; and the two throws action 0x12 (chips.md §3.9) had left, LilBoiler
and VDoll. standard-chips.md has the actions that fire no shot; objects-and-player.md §B8 and §B8a the buster's
projectile (attack #0) and the flying shot (attack #0xB), which several of these fire.

This is the original's behaviour, branch by branch, as the port must reproduce it. §18 says where the port keeps
it (content model v2) and where it departs.

Conventions (standard-chips.md's, and):

- `av` is the attack's variables: `av[0]`/`av[1]` the phase (the step) and whether its entry ran (step_init),
  `av[2]` the element, `av[3]` the variant (the chip's subtype), `av.u16[6]` the Atk+ bonus, `av.u32[8]` the damage
  word (damage, and the hit parameter in its high half), `av.u32[0xC]` the chip's four parameter bytes,
  `av.u16[0x10]` the phase timer, `av.u16[0x12]` a count (the buster's recovery word, which it shares).
- **Tick 1** is the first tick the action runs: the tick after the use tick whose idle handler called `set_attack`.
  "Ends on tick N" means `object_exitAttackState` runs on tick N (the lab traces show the action for N ticks,
  counting the use tick, and idle from tick N on).
- **The arm** is an attachment (actor #5, `sub_80B8E30`) with Param1 its kind and Param2 its animation, spawned into
  the navi's first related slot (`obj+0x4C`, related1) or its overlay slot (`AIData+0x68`). **Releasing** means both
  slots are cleared (an attachment whose slot is cleared goes to its destroy state and is freed the next tick).
  "The arm's animation + 1" is a byte store to the attachment's CurAnim.
- **The damage word + bonus** is `av.u32[8] + av.u16[6]`, a 32-bit add (it could carry into the hit parameter, which
  no chip's damage does).
- A spawner that calls `object_spawnType3` with its caller's registers leaves them as the object's position until
  its init places it; the table in §15 lists them.
- Sounds are the original's ids. Effects are effect #0's looks (`byte_80E0398`, objects-and-player.md §A.3), with
  its Param2 the flip. Collision types are `byte_8019C7C`'s indices.

| action | handler | chips and weapons | objects | ends on tick |
|---|---|---|---|---|
| 0x14 | `sub_80EBC0E` | 0x01..0x03 Cannon, HiCannon, M-Cannon; 0x140..0x142 GigaCan1-3 | attack #0 projectile (kinds 1, 0xC); GigaCan: two afterimages (effect #0x28) | 34 |
| 0x21 | `sub_80EC884` | 0x04 AirShot | attack #0 (kind 4) | 22 |
| 0x17 | `sub_80EBF10` | 0x05..0x08 Vulcan1-3, SuprVulc | attack #0x12 bullet | 11·shots + 3 |
| 0x25 | `sub_80ECBB0` | 0x09..0x0B Spreadr1-3; weapons 0x0C (SpoutCross's charged shot), 0x5F, 0x74, 0x80 | attack #0x12 bullet | 23 |
| 0x19 | `sub_80EC0E6` | 0x13B..0x13E BatCan1-4 | attack #1 bat shot | 38 (BatCan4 70) |
| 0x24 | `sub_80ECACA` | 0x0C..0x0E TankCan1-3 | attack #0x3C shell | 39 |
| 0x29 | `sub_80ECF2E` | 0x2B..0x2D MachGun1-3 | attack #9 panel strike, effect 0x2F | 82 |
| 0x2A | `sub_80ED090` | 0x40..0x42 CornSht1-3 | attack #0x10 corn | 35 |
| 0x30 | `sub_80ED55C` | 0x17 WideSht, 0x151 SuprSpr | attack #0x3B wave | 27 (SuprSpr 55) |
| 0x5D | `sub_80F00F2` | weapon 0x39 (no chip) | attack #0 (kind 6) | 60 |
| 0x55 | `sub_80EF970` | 0x173 SonicBom, 0x177 SprSonic (VarSwrd's and NeoVari's picks), 0x174 Curse, 0x175 Punisher | attack #0x58 sonic boom | 29 |
| 0x5B | `sub_80EFEE0` | 0x17D Z Saver (weapon 0x6E) | three one-tick hits (attack #3), effects 0x65..0x67, attack #0x58 | 73 (95 with the extra slash) |
| 0x12 | `sub_80EB628` subtype 3 | 0x62..0x64 LilBolr1-3 | attack #0x93 boiler, actor #0x54 layer, effect #0x14 | 27 |
| 0x12 | `sub_80EB628` subtype 8 | 0x96 VDoll | attack #0x7A doll, effect #0x4E curse, effect #0x11 sparkles | 27 |

Chips 0xCB..0xDC, 0x160..0x171 and 0x179..0x17B (Rush, Beat, Tango) also name action 0x14 with variant 0; none is a
folder chip. Variant 3 of action 0x14 (arm 0x1E, projectile kind 1) has no chip: **[unverified]**.

## 1. What the actions share

**The arm's animation by who holds it.** Three tables give the arm's Param2 by the holder's AI index (the link navis
as players: HeatMan 1, GroundMan 9, DustMan 10):

| routine | used by | holder | value |
|---|---|---|---|
| `sub_80EBD68` (`off_80EBD80`) | 0x14 | actor type 2 (a player) only; others 0 | 1 for AI index 1, 9, 10; else 0 |
| `sub_80EC934` (`off_80EC944`) | 0x21 | any (no actor-type test) | 1 for AI index 1, 9, 10; else 0 |
| `sub_80ECC86` (`off_80ECC94`) | 0x25 | any | 2 for AI index 1, 9, 10; else 0 |

The tables are 28 bytes; an AI index past them reads the code after them (an error in the port). The sword arm of
0x55 and 0x5B is `sub_80EBAE8` (lib/arm.luau's `arm.anim`) with kind `byte_80EBB64[variant]` (`sub_80EBB34`: 3 for
variants 0 and 1; the routine's actor-type tests all end in the same return).

**The probe.** Attacks #0x12 and #0x10 start as a probe: collision self type 0x3D (flags 0x80 only, so what it
touches takes no damage), target type 5, hit modifier 0, the primary element forced to 0 after set-up
(`CollisionData+0x02` = 0). When the probe's resolution finds something (its hit flags nonzero), the object turns
its registration into the real hit with `sub_801A082` (`reset_collision_types`: self type 4, target type 5, its own
hit modifier, SelfDamage from the damage word, the dimming bit if dimmed) and `sub_8019F8C` (primary and secondary
element from the object's element byte), and presents it over its burst region; the hit resolves on the object's
next removal. Their collision data's hit effect is never set, so it stays 0 (`object_createCollisionData` zeroes
it): both the probe's contact and the burst show hit spark 0 (`object_spawnCollisionEffect`, one RNG draw each).

**Releases and animations at the end.** Unless noted, the last phase releases the arm and calls
`object_exitAttackState` (back to idle with animation 0, the chip's lockout applied).

## 2. Action 0x14: Cannon, HiCannon, M-Cannon, GigaCan (`sub_80EBC0E`)

Two phases (`off_80EBC20`). Variant tables: `byte_80EBD2C` (the arm kind) = 0, 1, 2, 0x1E, 0, 1, 2 for variants 0..6;
`byte_80EBD34` (the projectile kind) = 1, 1, 1, 1, 0xC, 0xC, 0xC.

- **Fire** (`sub_80EBC28`). Tick 1 (entry): `object_setAnimation(8)`, the counter window, timer = 0, the arm (kind
  `byte_80EBD2C[variant]`, animation `sub_80EBD68`) into related1, USING_ACTION; it returns without counting.
  From tick 2, each tick, with the timer's value before it counts:
  - timer 8 and variant ≥ 4 (GigaCan; tick 10): two afterimages (`sub_80E33FA`, effect #0x28,
    objects-and-player.md §A.7), both with colour shader 0x8318 and a 30-tick lifetime, blinking:
    1. the arm's own sprite (`sub_80B8E62`: attachment kind `byte_80EBD2C[variant]`'s sprite in `byte_80B8BD4`,
       "0C-01" for kinds 0..2), animation 0, flipped as the navi, no shadow (its word 0x1E), at the navi's position
       plus its attach point 7 (`sub_8018810(NameID, 7, alliance, flip)`: x toward its facing, the point's y added
       to Z);
    2. a copy of the navi's sprite (Param1 0xFF), animation 8 (`sub_800F2AA` returns its argument), flipped as the
       navi, with a ground shadow (word 0x0101001E), at the navi's position.
  - timer 0xF (tick 17): sound 0xAE; the projectile (`sub_80C4FFE`) on the panel ahead, Z 24 pixels
    (0x18_0000), kind `byte_80EBD34[variant]` (its Param1; Param2 0), the damage word + bonus.
  - then timer + 1; at 0x1D (tick 30) the next phase.
- **Recover** (`sub_80EBD3C`). Entry (tick 31): release, `object_setAnimation(7)`, timer 3. Timer − 1 each tick,
  entry included; below 0 (tick 34): `object_exitAttackState`.

No reactive-abort check. Kind 1 is the cannon shot (hit modifier 3, spark 7); kind 0xC the GigaCan shot that bursts
(objects-and-player.md §B8).

## 3. Action 0x21: AirShot (`sub_80EC884`)

- **Fire** (`sub_80EC8A0`). Tick 1: `object_setAnimation(9)`, the counter window, timer 0, USING_ACTION, the arm
  (kind 0x13, animation `sub_80EC934`) into the overlay slot, sound 0xAF; returns without counting. From tick 2: at
  timer 5 (tick 7) the projectile on the panel ahead, Z 24 pixels, parameters `av.u32[0xC]` (AirShot's 4: kind 4,
  the wind shot that pushes: self type 0x12, hit modifier 0x61, spark 7), the damage word + bonus. Timer + 1; at 0xA
  (tick 11) the next phase.
- **Recover** (`sub_80EC90E`): timer 0xA from the entry (tick 12); below 0 (tick 22) release and exit.

## 4. Action 0x17: Vulcan1-3, SuprVulc (`sub_80EBF10`)

Three phases. `dword_80EBFEC` (shots by variant) = 3, 4, 5, 10; `dword_80EBFF0` (bullet heights) = 8, 16, 24, 32
pixels.

- **Raise** (`sub_80EBF30`). Tick 1: `object_setAnimation(0xA)`, the counter window, the arm (kind 0xD, animation 0)
  into the overlay slot, USING_ACTION, timer 1. Timer − 1 each tick; below 0 (tick 2) the next phase.
- **Fire** (`sub_80EBF6E`). Entry (tick 3): shots = `dword_80EBFEC[variant]`, the arm's animation + 1,
  `object_setAnimation(0xD)`, timer 0. Each tick timer − 1; below 0: timer = 10, **one RNG draw**
  (`GetPositiveSignedRNG2() & 3`) picks the height, a bullet (`sub_80C6ADA`, attack #0x12) on the panel ahead at
  that height with element `av[2]`, parameters `av.u32[0xC]` (Vulcan's 0xC: kind 0xC; SuprVulc's 0x10) and the
  damage word + bonus; sound 0xB9; shots − 1, and at 0 the next phase. So a bullet on ticks 3, 14, 25, ...
- **Recover** (`sub_80EBFF4`). Entry: the arm's animation + 1, `object_setAnimation(0xA)`, timer 0xA; below 0 (11
  ticks later) release and exit. Vulcan1 ends on tick 36, Vulcan2 47, Vulcan3 58, SuprVulc 113.

### 4.1 The bullet (attack #0x12, `sub_80C6946`)

A probe (§1) that steps a panel a tick; on the first thing it finds it bursts there. It has no sprite, so it isn't
drawn and its sprite never steps. Its kind, Param1, picks a 6-byte record of `byte_80C68D4` (19 of them; a kind past
them reads the routine's code, an error): the burst's hit region, hit modifier, effect id, status byte, and a sound
(u16, 0 for none).

| kind | region | hit mod | effect | status | sound | fired by |
|---|---|---|---|---|---|---|
| 0 | 2 | 0 | 0x21 | 0 | 0xAF | |
| 1 | 0xD | 0 | 0x21 | 0 | 0xAF | |
| 2 | 4 | 0 | 0x21 | 0 | 0xAF | |
| 3 | 0xF | 0 | 0x21 | 0 | 0xAF | Spreadr1-3 |
| 4 | 2 | 0 | 0x1F | 0 | 0x11D | weapon 0x80 |
| 5 | 0xD | 0 | 0x1F | 0 | 0x11D | |
| 6 | 4 | 0 | 0x1F | 0 | 0x11D | |
| 7 | 0xF | 3 | 0x1F | 0 | 0x11D | weapon 0x5F |
| 8 | 2 | 0 | 0x22 | 0 | 0x10F | weapon 0x74 |
| 9 | 0xD | 0 | 0x22 | 0 | 0x10F | |
| 0xA | 4 | 0 | 0x22 | 0 | 0x10F | |
| 0xB | 0xF | 0 | 0x22 | 0 | 0x10F | |
| 0xC | 2 | 1 | 0x20 | 0 | 0 | Vulcan1-3 |
| 0xD | 2 | 3 | 0x1F | 0 | 0x11D | weapon 0x0C (SpoutCross) |
| 0xE | 2 | 1 | 0x3E | 0 | 0 | |
| 0xF | 0xF | 1 | 0x38 | 0 | 0xAF | |
| 0x10 | 2 | 1 | 0x6A | 0 | 0 | SuprVulc |
| 0x11 | 2 | 1 | 0x20 | 0x12 | 0 | |
| 0x12 | 2 | 1 | 0x20 | 0x10 | 0 | |

(The kinds no action here fires come from the other spawners, `sub_80C9E20`, `sub_80CEF90` and `sub_80DF41C`: in
the lab ColArmy, ColForce, DarkInvs and dimming subtype 38 reach the bullet.)

- **Spawn** (`sub_80C6ADA`): panel, element (`sub_801155A`: the caller's r2), damage word, side and flip from the
  spawner, related1 the spawner; its Z is the spawner's r3 (the height), which it keeps.
- **Init** (`sub_80C6964`): X and Y from its panel; the record's address into ExtraVars; X velocity = the facing
  (±1: whole panels); `object_createCollisionData` (none left: `object_freeMemory`, and it is gone); the probe
  (§1); present; the update state (4). Its update does not run in the spawn tick.
- **Update** (`sub_80C69AC`): remove the collision (resolve); the hit spark. If the hit flags are nonzero and its
  collision status bit 0x100 (DEAD, used as a latch) isn't set: set it, action 4, phase 0. Then its action, and
  present:
  - Action 0 (`sub_80C6A08`): PanelX += the X velocity; X and Y from the panel; collision panels; off the field:
    region 0, VISIBLE off, destroy state.
  - Action 4, phase 0 (`sub_80C6A50`): `sub_80C6AB8` (`sub_801A082` with self type 4, target type 5, the record's
    hit modifier; the element; the record's status byte as status base if nonzero); region = the record's; then the
    look: kinds 0xC, 0xE, 0x10, 0x11, 0x12 one effect (the record's id) at the bullet's position, flipped as the
    bullet; the others `sub_801BD3C` (the record's effect on each panel of the region around its collision panel,
    turned by its side, at the bullet's Z; `battle.region_effects`). The record's sound if nonzero. Phase 4.
  - Action 4, phase 4 (`sub_80C6AAC`): region 0, destroy state.

  So the burst is presented on the tick it finds the target and hits on the next, when it also ends.
  Destroy state: `object_genericDestroy`. (A block after the update's return that would clear the region and
  destroy is unreachable.)

## 5. Action 0x25: Spreadr1-3 (`sub_80ECBB0`)

Two phases. Variant tables: `dword_80ECC54` (the arm kind) = 0x15, 0x15, 0x2F, 0x15; `dword_80ECC5C` (the bullet's
Z) = 16, 16, 0, 16 pixels. (`dword_80ECC58`, 1, 0xA, 0xA, 1, follows in the literal pool; nothing reads it.)

- **Fire** (`sub_80ECBCC`). Tick 1 (entry): `object_setAnimation(0xB)`, the counter window, the arm (kind by
  variant, animation `sub_80ECC86`) into the overlay slot, USING_ACTION, timer 0. Every tick timer + 1; above 1 (tick
  2): the arm's animation + 1, sound 0xAF, a bullet (attack #0x12, §4.1) on the panel ahead at the variant's Z,
  element `av[2]`, parameters `av.u32[0xC]` (the Spreaders' 3), the damage word + bonus; the next phase.
- **Recover** (`sub_80ECC60`): timer 0x14 from the entry (tick 3); below 0 (tick 23) release and exit.

The weapon routines that start it: 0x0C (`sub_8011C5E`, SpoutCross's charged shot: variant 2, kind 0xD, aqua, hit
parameter 0x94, damage `sub_8012642(20, 10)`: 20 + 10 · the buster's power up to 5), 0x5F (`sub_80123AA`: variant
2, kind 7, aqua, 70 damage, hit parameter 0xA), 0x74 (`sub_8012498`: variant 1, kind 8, fire, 40, 0xA), 0x80
(`sub_80124FE`: variant 2, kind 4, aqua, 50, 0xA). The last three are link navis' weapons.

## 6. Action 0x19: BatCan1-4 (`sub_80EC0E6`)

Three phases. `off_80EC118` (shots by variant) = 1, 1, 1, 3. BatCan1-3's parameters are 0x300, 0x301, 0x302 (the bat
shot's kind 0, 1, 2 and hit modifier 3); BatCan4's are 0, and each of its shots takes its parameters' low half from
`byte_80EC1BC` by the shots left: 3 left 0x0100 (kind 0, hit modifier 1), 2 left 0x0101 (kind 1, 1), 1 left 0x0302
(kind 2, 3).

- **Count** (`sub_80EC104`, tick 1): shots = `off_80EC118[variant]`; the next phase (no entry flag, no timer).
- **Fire** (`sub_80EC11C`). Entry (tick 2, and again for each further shot): `object_setAnimation(9)`; the form
  overlay (related2), if any, restarts its animation (CurAnimCopy = 0xFF); the counter window; timer 0; the
  attachment in related1, if any, is ended (`sub_80B8E58`: its state word 8); the arm (kind 0xF, animation 0) into
  related1; USING_ACTION; returns. Then each tick: at timer 5, sound 0xAE, BatCan4's parameters as above, and a bat
  shot (`sub_80C5176`, attack #1) on the panel ahead, Z 24 pixels, element `av[2]`, parameters `av.u32[0xC]`, the
  damage word + bonus. Timer + 1; at 0xF shots − 1: shots left, the entry runs again next tick (step_init 0); none,
  the next phase. Shots on ticks 8, 24, 40.
- **Recover** (`sub_80EC1C4`): timer 0x14 from the entry; below 0 `object_setAnimation(0)`, release, exit (BatCan1-3
  tick 38, BatCan4 tick 70).

### 6.1 The bat shot (attack #1, `sub_80C50B8`)

The projectile's twin without a sprite or kinds: it steps a panel every two ticks and ends on its first hit.

- **Spawn** (`sub_80C5176`): as the projectile's (`sub_801155A`); Z = 24 pixels from the spawner's r3.
- **Init** (`sub_80C50DC`): X and Y from its panel; Timer2 = 1; `object_createCollisionData` (none: freed);
  collision self type 4, target type 5, hit modifier Param2; hit spark 0xD; `sub_80C518C`; present; the update
  state, and the update runs in the spawn tick.
- **Update** (`sub_80C5116`): remove, hit spark. Battle over or hit flags nonzero: region 0, destroy state.
  Otherwise phase init 0 → Timer = Timer2, phase init 4; Timer − 1; below 0: PanelX += facing; off the field region 0
  and destroy state, else X and Y from the panel, collision panels, phase init 0. Present every tick. Then
  `object_updateSprite` (nothing loaded).
- `sub_80C518C` stores a word by kind into its collision data's +0x64 (`byte_80C50AC`: kind 0 0x10000, 1 0x100, 2
  0x1000000); a hit adds it into the target's +0xA0 accumulator (`sub_3007218`), which nothing in the game reads.
  The port can drop it.

## 7. Action 0x24: TankCan1-3 (`sub_80ECACA`)

Three phases; the parameters are 0x100 (the shell's kind 0, Param2 1: crack).

- **Aim** (`sub_80ECAE8`). Tick 1: `object_setAnimation(0xA)`, the counter window, timer 0xF, the arm (kind 0x14,
  animation 0) into the overlay slot, USING_ACTION; returns. Then timer − 1; at 0 (tick 16) the next phase.
- **Fire** (`sub_80ECB28`). Entry (tick 17): `object_setAnimation(9)`, timer 0, the arm's animation = 1 (restarted:
  `sub_80B8E70`); returns. Then at timer 5 (tick 23): a camera shake (1, 0x14; presentation) and the shell
  (`sub_80CB8DE`, attack #0x3C) on the panel ahead, element **0** (not `av[2]`), Z 24 pixels, parameters
  `av.u32[0xC]`, the damage word + bonus. Timer + 1; at 0x12 (tick 35) the next phase.
- **Recover** (`sub_80ECB84`). Entry (tick 36): release, `object_setAnimation(7)`, timer 3; below 0 (tick 39) exit.

### 7.1 The shell (attack #0x3C, `sub_80CB6F8`)

Not drawn (it loads no sprite). It steps a panel every two ticks; its own hit ends it; off the far edge it blasts the
last column and, 15 ticks later, sets off three explosions there in a random order.

- **Spawn** (`sub_80CB8DE`): panel, element, damage word, side and flip; Z 24 pixels (r3).
- **Init** (`sub_80CB71C`): X and Y from its panel; `object_createCollisionData` (none: freed); collision types by
  kind from `byte_80CB6F0` (kind 0: self 4, target 5, hit modifier 0x4B; kind 1: 4, 5, 0x63); hit spark 0xFF;
  present; Timer2 = 1; the update state; the update runs; then sound 0xAE.
- **Update** (`sub_80CB762`): remove, hit spark. While its region isn't 0: a hit (flags nonzero): sound 0xC3,
  effect 0x22 at its position, region 0, destroy state; the battle over: region 0, destroy state. Otherwise its
  action, then present:
  - Action 0 (`sub_80CB7BC`). Phase and phase init both 0: phase = 1, Timer = Timer2, done. Else Timer − 1; above 0
    done; else phase 0, PanelX += facing, X and Y, collision panels; on the field, done. Off it (x = PanelX −
    facing is the last column): region 0; a camera shake (3, 0x28); sound 0xC3; effect 0x60 flipped by side ^ flip
    at (x, y)'s center, Z 0; a one-tick hit (`object_spawnCollisionRegion`) at (x, y): region 4 (the column of
    three), hit spark 0xFF, target type 5, self type 0xA, hit modifier 3, element 0, Z 0, damage = its damage word
    shifted right by Param3 (0 for TankCan); if Param2 ≠ 0, `object_crackPanel` on (x, y), (x, y − 1), (x, y + 1)
    (off-field panels are skipped); then Param2..4 = the three panels packed (x | y << 4, y − 1, y + 1) and
    `sub_8000C72` shuffles those three bytes: three rounds of two **RNG draws** (`GetPositiveSignedRNG2() % 3`
    each) swapping the two picked bytes; Timer2 = 3, Timer = 0xE, action 4, phase 0.
  - Action 4 (`sub_80CB898`): while Timer ≠ 0, Timer − 1. At 0: Timer = 4; the byte at Param1 + Timer2 (Param4, then
    Param3, Param2, and last Param1, the kind) is unpacked (x the low three bits, y the high nibble); for y 1..3
    effect 0 at that panel's center, Z 6 pixels; Timer2 − 1, and below 0 the destroy state. So the explosions come
    15, 20 and 25 ticks after the blast, and the shell is destroyed after 30.

## 8. Action 0x29: MachGun1-3 (`sub_80ECF2E`)

Nine strikes on a target that sweeps up and down a column, a strike every nine ticks. The parameters are 0x10600
(Param1 0, Param2 6 the reticle's ticks, Param3 1). No reactive-abort check.

- **Aim** (`sub_80ECF48`, tick 1, no entry flag): the bonus is folded into the damage word (`av.u32[8] +=
  av.u16[6]`, so the strikes don't add it again); the target column `av[0x16]` = `sub_80ED040`, the target row
  `av[0x17]` = 3; CurAnim = 0xA (a byte store); the counter window; the arm (kind 0x1D, animation 0) into the
  overlay slot; USING_ACTION; strikes `av[0xE]` = 9; the row step `av.u32[0x30]` = −1; the next phase.
- **Fire** (`sub_80ECF8E`), on step_init:
  - 0 (entry, tick 2): the arm's animation = 1; `object_setAnimation(0xD)`; then as 2.
  - 2: step_init = 1; timer 8; a panel strike (`sub_80C5F2C`, attack #9, the pack's objects/panel-strike, chips.md
    §3.6.33) at the target with element `av[2]`, Z 0, the damage word, parameters `(av.u32[0xC] & 0xFFFF) |
    0x10000` (Param1 0: no crack; Param2 6: it strikes after 6 ticks; Param3 1: hit modifier 1); effect 0x2F (the
    reticle) at the target's center, Z 0, with Timer = `av[0xD]` (6: it lasts 6 ticks). Then as 1.
  - 1: timer − 1; at or above 0, done. Below 0, move the target: row + step in 1..3 → that row. Otherwise the step
    reverses, and the column moves a panel toward `sub_80ED040` (recomputed now); if that is the target column
    already, the row moves by the reversed step. Then strikes − 1: left, step_init 2 (the next strike fires next
    tick); none, release and exit.

  Strikes on ticks 2, 11, 20, ..., 74; the action ends on tick 82. The target runs rows 3, 2, 1, 2, 3, ... while
  the column stays.
- `sub_80ED040`: from the navi's column + facing, forward, the first column where some row 1..3 has the panel flags
  `byte_80ED080` gives by side (every bit of 0x04010000 for side 0, 0x08010000 for side 1: the other side's body on
  a panel), stopping at columns 0 and 7; none found: 6 when facing right, 1 when facing left (the last column
  ahead).

## 9. Action 0x2A: CornSht1-3 (`sub_80ED090`)

- **Fire** (`sub_80ED0AC`). Tick 1 (entry): the bonus folded into the damage word; CurAnim = 0xA (a byte store); the
  counter window; the arm (kind 0x18, animation 0) into the overlay slot; USING_ACTION; timer 0. Every tick timer +
  1; at 10 (tick 10): a corn (`sub_80C67D8`, attack #0x10) on the navi's **own** panel, element `av[2]`, Z 0,
  parameters `av.u32[0xC]` (0), the damage word; the arm's animation + 1; sound 0x180; the next phase.
- **Recover** (`sub_80ED118`): timer 0x18 from the entry (tick 11); below 0 (tick 35) release and exit.

### 9.1 The corn (attack #0x10, `sub_80C6580`)

A probe (§1) that steps a panel a tick from the navi's panel; where it finds something it bursts twice, grasses the
panel, and spreads a corn to each enemy body in the column beyond, which bursts in turn. Param1 is its generation (0
the probe; the spread's are its parent's + 1; 0xFF bursts without spreading: CornFsta's, dimming subtype 29).

- **Spawn** (`sub_80C67D8`): as the projectile's (`sub_801155A`): the caller's element; Z the caller's r3.
- **Init** (`sub_80C65A0`): X and Y from its panel; X velocity = facing; `object_createCollisionData` (none: freed);
  the probe; present; Param2 = 2 (bursts), Timer2 = 0xC (the spread's delay); the update state. Param1 ≠ 0: action
  4 at once (it bursts where it is), and unless Param1 is 0xFF, `sub_80C673A` now. The update doesn't run in the
  spawn tick.
- `sub_80C673A`: the panels of region 4 (the column of three) around the panel ahead of it that have the other
  side's body (`off_80C6764`: 0x04000000 for side 0, 0x08000000 for side 1), in the region's order and turned by side
  ^ flip (`object_getPanelRegion`), packed x | y << 4 into ExtraVars+0x64.., their count into ExtraVars.
- **Update** (`sub_80C65EE`): remove, hit spark. Hit flags nonzero: in action 4, region 0 (its burst connected);
  otherwise (the probe found something) `sub_80C673A`, action 4, phase 0. Then the battle over: region 0, VISIBLE
  off, destroy state (no present). Else its action, and present:
  - Action 0 (`sub_80C6650`): the bullet's step (§4.1).
  - Action 4, phase 0 (`sub_80C6698`): the first time (ExtraVars+0xC 0 → 1), a solid panel (flag 0x10) under it
    turns to grass (type 6). `sub_80C6726` (`sub_801A082`: self type 4, target 5, hit modifier 1; the element);
    region 1; effect 0x23 at its position; sound 0x70 (`SOUND_HIT_BOMB_1`); Timer 0xC; phase 4.
  - Action 4, phase 4 (`sub_80C66E4`): ExtraVars+8 counts up, and when it was a multiple of 8, sound 0x70. Timer2 −
    1 (u16), and on reaching 0 the spread (`sub_80C6774`): for each packed panel, last first, unless an attack
    #0x10 already stands there (`sub_80C67A4` walks the update list), a corn there with Param1 + 1, its element, Z
    0, its damage word. Timer − 1; below 0: bursts − 1, and with one left, phase 0 again (the second burst), else
    region 0 and the destroy state.

  Each burst lasts 13 ticks; the spread comes on the twelfth tick of the first (Timer2 wraps to 0xFFFF after, so it
  never spreads again). Destroy state: `object_genericDestroy`.

## 10. Action 0x30: WideSht, SuprSpr (`sub_80ED55C`)

Three phases. `byte_80ED5BC` (shots by variant) = 1, 3. Each shot's Param2 (its hit modifier, for kinds 6 and up)
is `dword_80ED618` by the shots left: 3 → 1, 2 → 1, 1 → 3 (0 → 0).

- **Raise** (`sub_80ED57C`, tick 1, no entry flag): `object_setAnimation(0xA)`, the counter window, the arm (kind
  0x23, or 0x37 for variant 2, which no chip has; animation 0) into the overlay slot, USING_ACTION; the next phase;
  shots = `byte_80ED5BC[variant]`; timer 5.
- **Fire** (`sub_80ED5BE`). Timer − 1; above 0, done. At 0: the first time, the arm's animation + 1 (step_init 4);
  timer 0xE; `av[0xD]` = Param2 by the shots left; a wave (`sub_80CB686`, attack #0x3B) on the panel ahead, element
  `av[2]`, Z 16 pixels, parameters `av.u32[0xC]` (WideSht's 3, SuprSpr's 6, with the new Param2), the damage word +
  bonus; shots − 1, at 0 the next phase. Waves on ticks 6, 20, 34.
- **Recover** (`sub_80ED61C`): entry: the arm's animation − 1, timer 0x14; below 0 release and exit (WideSht tick
  27, SuprSpr tick 55).

### 10.1 The wave (attack #0x3B, `sub_80CB49C`)

Sprite 0x10/0x37 ("10-37"). It appears over the navi's column, and once its appearing animation ends, slides
forward. Kinds below 6 end on their first hit; kinds 6 and up go through, hitting each panel once. `byte_80CB58C`
(speed by kind, 16.16 a tick): 0x30000, 0x38000, 0x40000, 0x60000, 0x68000, 0x70000, 0x60000, 0x68000, 0x70000,
0x70000 (kinds 0..9).

- **Spawn** (`sub_80CB686`): panel, element, damage word; kinds 0..2 also keep the spawner's attack variables'
  address in related1 and write the word 2 over `av[0..3]` (the step, step_init, element and variant); side and
  flip.
- **Init** (`sub_80CB4D4`): the sprite, no shadow, VISIBLE, animation 0 (and CurAnimCopy 0), flipped as it is; X
  velocity = facing · speed; PanelX −= facing (the navi's column); X and Y from the panel;
  `object_createCollisionData` (none: freed); collision self type 4 (0x25 for kind 9), target 5, hit modifier 3
  (Param2 for kinds 6 and up); region **0**; hit spark 0xFF; present. Kinds 0..2: the word 3 over the spawner's
  `av[0..3]`. Kind 9: the bug (0x14, argument 2: `sub_801A4D0`) and palette 1. ExtraVars+4 = 0xA; the update state;
  the update runs; sound 0xA7.
- **Update** (`sub_80CB5B4`): remove, hit spark. The battle over: region 0, destroy state. Hit flags nonzero: kinds
  below 6 region 0 and destroy state; others region 0 and ExtraVars = its panel (x | y << 8). Then action 0
  (`sub_80CB628`), on phase init:
  - 0: until the sprite's frame flags have 0x80 (the animation's last frame ended), nothing (not even the collision
    panels). Then CurAnim = 1 (a byte store); PanelX += facing; X and Y from the panel; region 4 (the column of
    three); kinds below 3: phase init 1 and Timer 0xF; others phase init 2.
  - 1: Timer − 1; at 0, phase init 2 and as 2.
  - 2: X += X velocity; the panel from X.

  Then (phases 1 and 2, and the tick phase 0 ends) the collision panels. `sub_80CB6B0`: kind 9, every 5 ticks
  (ExtraVars+4), an afterimage (`sub_80CB6CA`, effect #0x28: parameters 0x10010 | flip << 24, that is sprite
  0x10/0x00, animation 1, its flip; at its position; colour shader 0xC3FF; lifetime 0x14, no shadow). With phase
  init ≠ 0, region 0 and its panel ≠ ExtraVars: region 4 again. On the field, present; off it, region 0 and destroy
  state.
- **Destroy** (`sub_80CB4C0`): kinds below 3 write 0 over the spawner's `av[0..3]`; `object_genericDestroy`.
- Then every state: `object_updateSprite`.

Only kinds 3 (WideSht) and 6 (SuprSpr) have a spawner: kinds 0..2 (the words over `av[0..3]`, the 15-tick wait) and
9 (the bug, palette, afterimages) are dead code, ported as written **[unverified]**.

## 11. Action 0x5D: the rapid buster (`sub_80F00F2`)

Weapon routine 0x39 (`sub_8012124`) sets it up: damage 5 · (buster Attack + 1) (navi stat 1), hit parameter 0x85,
parameters 6 (the charged shot's projectile kind), `av.u16[0x12]` (shots fired) = 0. No chip names it, and no lab
scenario reaches it **[unverified]**.

- **Pattern** (`sub_80F0110`, tick 1): timer 0; **one RNG draw**: `GetPositiveSignedRNG2() % 6` picks a row pattern
  from `byte_80F0138` (the six orders of −1, 0, +1: [−1, 1, 0], [−1, 0, 1], [0, 1, −1], [0, −1, 1], [1, 0, −1],
  [1, −1, 0]) into `av.u32[0x30]`; the next phase, which runs at once.
- **Fire** (`sub_80F014C`). Entry: `object_setAnimation(0xE)` and CurAnimCopy 0xFF; the first shot raises the buster
  arm (`sub_80EB562`, lib/buster.luau's `raise_arm`), later ones restart the arm's animation (`sub_80B8E7C`) and the
  form overlay's (`sub_80C44D2`); USING_ACTION; timer 0. Then at timer 1: sound 0x6A; the projectile on the panel
  ahead in the row `y + pattern[shots % 3]`, Z 24 pixels, parameters `av.u16[0xC]` (kind 6), the damage word
  **without** the bonus; the muzzle flash (attachment kind 5) into related1. Timer + 1; above 5, shots + 1: below 9
  the entry runs again, else the next phase. Shots on ticks 2, 8, ..., 50.
- **Recover** (`sub_80F01E8`): timer 5; below 0 (tick 60) release and exit.

## 12. Action 0x55: SonicBom, SprSonic (`sub_80EF970`)

VarSwrd's fifth pick (0x173 SonicBom: parameters 3, variant 0) and NeoVari's second (0x177 SprSonic: 0x101,
variant 1) start it through `set_attack`, keeping the sword's damage (lib/swords/vari.luau). 0x174 Curse (VDoll's
telop, §14.2) and 0x175 Punisher name it too, with parameters 0.

- **Count** (`sub_80EF990`, tick 1): slashes `av.u16[0x12]` = 1 (both branches of its variant test store 1); the
  next phase.
- **Slash** (`sub_80EF9A2`). Entry (tick 2): `object_setAnimation(5)`, the counter window, sound 0xB0, the blade
  (kind `sub_80EBB34`, animation `sub_80EBAE8`) into related1, USING_ACTION, timer 0x15. At timer 0xC (tick 11): a
  sonic boom (`sub_80CF91E`, attack #0x58) on the panel ahead, element `av[2]`, parameters `av.u32[0xC]`, the damage
  word + bonus, and when it spawned its X velocity = 8 pixels a tick (0x80000). Timer − 1; below 0 (tick 23):
  slashes − 1; left, release and the entry again; none, the next phase.
- **Recover** (`sub_80EFA30`): timer 5; below 0 (tick 29) release and exit.

### 12.1 The sonic boom (attack #0x58, `sub_80CF7F0`)

Sprite 0x0C/0x14 ("0C-14"), 16 pixels up, flying 8 pixels a tick over a column of three panels (region 4). Param1 is
its hit modifier; with Param2 set it goes through what it hits (palette 0xB), else it ends there (palette by element:
0..4 → 0..4, others 0; `byte_80CF8A8`).

- **Spawn** (`sub_80CF91E`): as the projectile's (`sub_801155A`); the caller then sets its X velocity.
- **Init** (`sub_80CF810`): the battle over: freed. Header flags |= 3 (ACTIVE, VISIBLE); the sprite, no shadow,
  animation 0, one `sprite_update`, flipped as it is; the palette; X and Y from its panel, Z = 16 pixels; Timer 0; X
  velocity = facing · X velocity; `object_createCollisionData` (none: freed); self type 7, target type 5, hit
  modifier Param1; region 4; present; the update state; the update runs.
- **Update** (`sub_80CF8B0`): remove (no hit spark). The battle over: region 0, destroy state (a byte store). Hit
  flags & 0x3F800000: without Param2 region 0 and destroy state; with it region 0 (and on). Then move
  (`sub_80CF8F8`): X += X velocity, its panel and its collision panel from X; off the field region 0 and destroy
  state. When its panel changed this tick, region 4 again (also on the tick it leaves the field, where the region's
  panels are all off it). Present.
- It never calls a sprite-stepping routine after the init: the sprite stays on its first frame.
- Destroy state: `object_genericDestroy`.

Action 0x48 (`sub_80EEA3C`), AntiSwrd's counter (chips/antiswrd/counter, which the ruleset starts by the role
`actions.anti_sword_counter`), also throws it: three booms, hit modifier 1, 1 and 3.

## 13. Action 0x5B: Z Saver (`sub_80EFEE0`)

Three slashes with growing reach; pressing B and Left together (within three ticks of each other) during the last
twelve ticks of the third gives a fourth, which throws a piercing sonic boom. Weapon routine 0x6E (`sub_801245A`)
starts it from chip 0x17D's record (`loc_80126EA`: damage, hit parameter 0x94, the Sword family's secondary element
0x80). No lab scenario reaches it **[unverified]**.

- **Start** (`sub_80EFF00`, tick 1): USING_ACTION; the slash index `av[0x12]` = 0 (a **byte** store: the halfword's
  high byte stays; the buster's recovery word shares it and is always below 0x100, so it is 0 in practice); slashes
  `av.u32[0x40]` = 3; timer 0; the command buffer `av[0x30..0x3F]` zeroed (`sub_801299C`); the command window
  `av.u32[0x44]` = 0; the next phase.
- **Slash** (`sub_80EFF28`). First, every tick: with the window open (`av.u32[0x44]` neither 0 nor 0xFF), the
  command check `sub_8012956(av+0x30, pressed)`; when it succeeds, slashes + 1 and sound 0x8B. Then:
  - Entry (ticks 2, 24, 46, 68): the attachment in the overlay slot ends (`sub_80B8E58`); `object_setAnimation(5)`;
    the form overlay restarts (`sub_8011450`); the counter window; sound 0x131; the blade (kind `sub_80EBB34`,
    animation `sub_80EBAE8`) into the overlay slot; timer 0x15.
  - At timer 0xC (the entry's tick + 9): slash index below 3: index 2 opens the command window (`av.u32[0x44]` = 1);
    a one-tick hit (`object_spawnCollisionRegion`) on the panel ahead, element `av[2]`, Z 0, the damage word +
    bonus, hit modifier 1, and by index from `byte_80F004C`: region 2 / 4 / 6 (the panel and the next; the column
    of three; three in a row), hit spark 0xFF, target type 5, self type 7; and an effect 0x65 / 0x66 / 0x67 at that
    panel's center, 16 pixels up, flipped as the navi. Index 3 and up: a sonic boom (§12.1) on the panel ahead,
    parameters 0x203 (hit modifier 3, piercing), element `av[2]`, the damage word + bonus, X velocity 8 pixels.
  - Timer − 1; below 0 (the entry's tick + 21): index + 1; below the slash count, the entry again next tick; else
    the next phase.
- **Recover** (`sub_80F0068`): entry: the collision status bit MOVING (0x40) cleared, timer 5; below 0 release and
  exit (tick 73, or 95 after the fourth slash).
- **The command** (`sub_8012956`, buffer: done flag, ring index, tick count, three u16 slots): once done, false.
  Count + 1; above 12: done, false. Else the pressed word goes into the next of the three slots (cycling); when the
  three slots together have both 0x02 (B) and 0x20 (Left): done, true. So the window is the twelve ticks after the
  third slash's hit (ticks 56..67), and it succeeds once at most. Left is the raw key: the link records each side's
  raw keys and only the right side's screen is mirrored (objects-and-player.md §M3.2), so it is "back" for both.

## 14. Action 0x12's LilBoiler and VDoll

The bomb action (chips.md §3.9) throws on its tenth tick with the registers r1..r3 = the release point (4 pixels
ahead, 48 up), r4 = the chip's parameters, r6 = the damage word + bonus, and r0 = the Atk+ bonus
(`av.u16[6]`, left over from the damage sum; neither thrower reads it). The held thing is attachment 4,
animation 0 for both.

### 14.1 LilBoiler (subtype 3): the boiler (attack #0x93, `sub_80D75FC`)

A field object (NameID 0xEB) thrown three panels ahead. Its HP counts down from 4000 − its damage, so the damage it
will do (MaxHP − HP) grows with every hit it takes, up to 999. After three hits that each lowered its HP, or when its
lifetime is about to run out, it erupts: 32 ticks later an aqua hit over the eight panels around it.

- **Throw** (`sub_80D7A96`): the target = the navi's PanelXY halfword (x | y << 8) + 3 · facing (the panel three
  ahead; a column below 0 would borrow from the row, which no navi's position allows), into Param3 (x) and Param4
  (y) over the chip's parameters (Param1 the level: 0, 2, 3 for LilBolr1-3). `sub_80D7A78`: attack #0x93 at the
  release point with those parameters; the damage word; side and flip; then `setFieldBattleObject_800F614(boiler,
  side = r1, class 1)`. r1 is the user's alliance and flip halfword: `sub_80D7A78` pops the thrower's r0 (the
  Atk+ bonus) into r1 after the spawn, but loads the user's halfword over it (`ldrh r1, [r5, #0x16]`, to copy it
  to the boiler) before the call. So the boiler is its user's side's class-1 field object (evicting that side's
  current one: HP 0), whatever the bonus. (A flipped user's "side" would be 0x100 more and land far past the
  registry; no player is flipped.) An earlier reading took r1 for the bonus still; §18 has what the lab showed.
- **Init** (`sub_80D761C`): VISIBLE; sprite 0x04/0x0D ("04-0D"), animation 0, a ground shadow, flipped as it is,
  palette 3 · Param1; NameID 0xEB; FuturePanel = the target. The flight (lib/trajectory.luau): the angle to the
  target's center (`calcAngle_800117C`, kept in its +0x0C byte), X and Y velocities at 0x2C000 along it
  (`sub_80011A0`), the Z velocity and ticks to land at Z 0 under gravity −0x2800 (`sub_800120E`).
  - Ticks 0 (already there): at the target's center, Z 0; release tracking (`sub_802EF5C`), unregister
    (`sub_800F656`), `object_genericDestroy`.
  - Else: a damage word with the double flag 0x8000 becomes (damage & 0x7000) | (damage & 0xFFF) · 2; MaxHP 4000,
    HP = 4000 − (damage & 0xFFF), ExtraVars+0xC = HP; Timer = 0xB4 (its lifetime, which also caps the flight);
    `object_createCollisionData` (none: release tracking, unregister, freed); self type 0x4F, target type 0xF, hit
    modifier 3; SelfDamage by Param1 from `dword_80D7748` (50, 50, 70, 90); counter byte 0x1E; region 0; hit spark
    0xA; present; the HP display (`sub_801DC7C(−16, 3)`, presentation); the update state (a byte store); the update
    runs.
- **Update** (`sub_80D774C`): VISIBLE; the obstacle service's `take_hits("keeps_damage")` (`sub_801AD12`); the hit
  spark; `tick_lifetime` (`sub_800F672`); `react` with `sub_801B878` (below) and its action table (`off_80D7798`:
  0 fly, 1 `return_to_idle` (`sub_80165B8`), 2 destroyed or erupting, 3 `flinch`, 4 `paralyzed`, 6 `frozen`, 7
  `bubbled` (`sub_80166AE`, `sub_8016B02`, `sub_8016B36`, `sub_8016B72`: the actor reactions, an error on an
  obstacle), 5 `knocked_back` (`sub_8017CC0`), 8 fly again, 9 nothing (`sub_80D7878`, where it sits after landing));
  HP at or below 3001 becomes 3001; the eruption check (`sub_80D7A2A`); while erupting, HP = MaxHP − (damage &
  0xFFF) and ExtraVars+0xC = HP; `object_updateSprite`; present.
- **`sub_801B878`** is `sub_801B394` ("breaks", field-objects.md §4.3) except that a crushing hit (0x0C800002: a
  body, a breaking hit) zeroes its HP only while ExtraVars+4 is 0. While it erupts a crushing hit does nothing more
  than any hit. (kinds/obstacle.rs's note that it is "destroys" while erupting is wrong: nothing destroys it then
  but a removal request or its HP reaching 0.)
- **Fly** (actions 0 and 8, `sub_80D77D4`): Timer − 1, and at 0 it lands. Else Z velocity −= 0x2800, and the
  position + velocity; above Z 0 it flies on, else it lands. (Timer counts down twice a tick in flight, with the
  lifetime.) Landing: its panel and its collision panel = the target, at the target's center, Z 0. On a solid panel:
  sound 0xC0, a camera shake (1, 0xF), region 1, runs while dimmed (header flag 0x10), action 9. Else (a hole): X
  and Y from the panel, a puff (effect #0x14, `sub_80E1D7A`, at its position), the HP display off (`sub_801DD34`),
  region 0, unregister, release tracking, destroy state (a byte store).
- **The eruption check** (`sub_80D7A2A`), unless in action 2: Timer = 1 erupts now. Else, not erupting, with hit
  flags this tick and its HP changed since the last such check (ExtraVars+0xC), hits (ExtraVars) + 1, and the third
  erupts. Erupting: ExtraVars+4 = 1; its damage = (MaxHP − HP) | (damage & 0x7000); action 2, phase 0.
- **Action 2** (`sub_80D787C`): the HP display off. Battle over or not erupting: destroyed (`sub_80D789C`); else the
  eruption (`sub_80D7928`).
  - Destroyed: region 0. A removal request (collision flag2 0x8000): absorbed (0x300000) → it flies to the absorber
    as obstacle kind 0xA (`fly_to_absorber`) and goes; else `blink_out`: blinking, wait; done, goes; not vanishing,
    effect 0x14 at its position 12 pixels up, and goes. No request: the first tick (phase init 0 → 4) VISIBLE off,
    and with HP 0 effect 0 (the explosion) at its position and sound 0x70, and goes; otherwise (HP not 0, or a later
    tick) it waits out any dimming and goes. Going: release tracking, unregister, unreserve FuturePanel, the destroy
    state (word store), the HP display off.
  - The eruption, while not dimmed, by phase; then, dimmed or not, Timer |= 0x20 (so its lifetime never ends):
    - 0 (`sub_80D795C`): region 0; its collision status word cleared; release tracking; sound 0x184; the layer
      (`sub_80C4038(0, 0, 0, 0)` with parameters 0x10D04, actor #0x54, kept in ExtraVars+8); unregister; reserve
      FuturePanel; phase 4.
    - 4 (`sub_80D7996`): CurAnim = 5 (a byte store); the layer's CurAnim = 9; Timer2 = 0x20; phase 8.
    - 8 (`sub_80D79AC`): at Timer2 8: a one-tick hit at its panel over region 0x10 (the eight around it), aqua
      (element 2), Z 0, hit spark 0xFF, target type 5, self type 4, hit modifier 3, its damage word; effect 0x32 on
      each panel of region 0x10, turned by side ^ flip, Z 0 (`sub_801BD3C`); the layer ends (`sub_80C4072`). At
      Timer2 0x14: sound 0x185. Timer2 − 1; below 0, phase 0xC. While bit 2 of Timer is clear, the panels of region
      0x10 around it are highlighted (`sub_8109660`, drawn only).
    - 0xC (`sub_80D7A0E`): unreserve FuturePanel, VISIBLE off, destroy state, the HP display off.
- The layer (actor #0x54, `sub_80C3EE0`), a sprite that follows its owner: Param1/Param2 its sprite (0x04/0x0D, the
  boiler's), ExtraVars+4 its animation (0), Param3 set so its palette is Param4 (0) rather than the owner's.
  - Spawn (`sub_80C4038(r0, r1, r2, r3)`): ExtraVars+0xC = r0 (a Z offset), +8 = r1 (0: a pixel behind), +4 = r2 (the
    animation), ExtraVars = r3 (0: follows the owner's flip); position, side and flip from the owner, related1 the
    owner, runs while paused and dimmed (0x14), ExtraVars+0x10 and +0x14 = 0.
  - Init (`sub_80C3F00`): its panel from X, Y; the sprite, no shadow, VISIBLE; CurAnim and CurAnimCopy =
    ExtraVars+4; the palette; flipped; the update state; the update runs.
  - Update (`sub_80C3F52`): the owner's position; Z + ExtraVars+0xC; Y and Z − 1 pixel with ExtraVars+8 set, + 1
    pixel otherwise; unless ExtraVars+0x18, the owner's VISIBLE bit and colour shader; unless Param3, the owner's
    palette; the owner's final palette and draw priority (`sub_8002F3E`; presentation); unless ExtraVars, the owner's
    flip; the owner's blending; with ExtraVars+0x10, `sub_80C409C` (a panel-row offset: Y and Z + (3 − PanelY) · 24
    pixels, + 1 on their integer halves). Its sprite steps (`object_updateSprite`) unless dimmed, paused, or the
    owner's collision status has any of 0x80110C00 (bubbled, drag, frozen, paralyzed, flinching), or (with
    ExtraVars+0x14) the owner's HP is 0.
  - `sub_80C4072` ends it (state 8, then `object_freeMemory`); `sub_80C407C`, `sub_80C4086`, `sub_80C4090` set
    ExtraVars+0x10, +0x14, +0x18 (the viruses' uses).

### 14.2 VDoll (subtype 8): the doll (attack #0x7A, `sub_80D46B8`)

A field object (NameID 0xE2, 50 HP) thrown three panels ahead in a 60-tick arc. Where it lands the panel turns to
poison. The first damage it takes after landing curses the other side: a dimming whose telop is chip 0x174 Curse,
which hits every combatant of the other side with that damage.

- **Throw** (`sub_80D49F6`): Param3 = PanelX + 3 · facing, Param4 = PanelY (the halfword add would borrow from the
  row below column 0), Param1 and Param2 0 (the chip's parameters are dropped). `sub_80D49CC`: attack #0x7A at the
  release point; the damage word (VDoll's 10, hit parameter 0x10; the chip isn't boostable); side and flip; runs
  while dimmed (header flag 0x10); registered as its side's class-1 field object.
- **Init** (`sub_80D46D8`): VISIBLE; sprite 0x0C/0x34 ("0C-34"), a ground shadow, CurAnim 1 (CurAnimCopy 0), one
  `sprite_update`, flipped; its panel from X and Y; NameID 0xE2; HP = MaxHP = 50; element 0; Timer 0x1E0 (480 ticks);
  `object_createCollisionData` (none: freed); self type 0xC, target type 0xD, hit modifier 1; region 0; hit spark 0;
  present; the update state (a byte store); the update runs.
- **Update** (`sub_80D4754`): VISIBLE; `sprite_clearFinalPalette` (presentation); `take_hits("keeps_damage")`; the
  hit spark; `tick_lifetime`; with damage taken this tick (collision +0x80 ≠ 0): HP 0, ExtraVars = 1 (hit),
  ExtraVars+4 = the damage, region 0. `react` ("breaks", `sub_801B394`) with `off_80D479C` (0 fly, 1
  `return_to_idle`, 2 destroyed, 3 `flinch`, 4 `paralyzed`, 6 `frozen`, 7 `bubbled`, 5 `knocked_back`, 8 idle); the
  sprite (`sub_801BC24`, `update_sprite`'s gating); present.
- **Fly** (action 0, `sub_80D47C0`), not while dimmed. The first time: the flight to the target's center at Z 0 in
  60 ticks under gravity −0x2000 (`sub_8001330`), Timer2 = 60, ExtraVars+0xC = 1 (flying). Every tick: Timer2 below
  5, region 1 (its last five ticks can hit: self type 0xC, the chip's damage). Timer2 − 1; at or above 0, the
  position + velocity, Z velocity − 0x2000, its panel from X and Y, the collision panels. Below 0 (tick 61 of the
  flight): not on a solid panel, action 2; else X and Y from the panel, Z 0, collision panels, the panel turns to
  poison (type 4), sound 0x90, ExtraVars+0xC = 0, action 8.
- **Idle** (action 8, `sub_80D4870`): the first time (phase 0 → 1), `sub_801A082` with self type 0xE, target type
  0xF, hit modifier 3.
- **Destroyed** (action 2, `sub_80D4888`): phase 0 → phase 4 at once; phase 4 (`sub_80D48AE`), in order:
  1. `blink_out` (vanishing): blinking, wait; done, it goes quietly.
  2. Collision status 0x04000000: goes quietly.
  3. A removal request (flag2 0x8000): absorbed (0x300000), `fly_to_absorber(8)` and it goes quietly; else a puff.
  4. No damage recorded (ExtraVars+4 0): a puff.
  5. Dimmed: wait.
  6. Not hit (ExtraVars 0), or the battle over: a puff.
  7. Still flying (ExtraVars+0xC ≠ 0): goes quietly.
  8. The curse: phase 8; VISIBLE off; effect 0x14 at its position 12 pixels up; the HUD element 0x40 on
     (`sub_801DACC`, presentation); the curse controller (`sub_80E61D2`, effect #0x4E) on its panel with element 0,
     its parameters, damage word = the damage recorded | 0xB2 << 16 (hit parameter 0xB2), and the telop's chip
     0x174 (Curse), bonus 0; `sub_800BF16(its side, no cut-in, controller)` starts the dimming
     (`dimming.start`); release tracking.

  A puff is effect 0x14 at its position 16 pixels up; then, as "going quietly": release tracking, unregister, region
  0, unreserve FuturePanel, VISIBLE off, the destroy state (word store). (A block with effect 0x24 and sound 0x107
  follows an unconditional branch and is unreachable.)
- **Phase 8** (`sub_80D49A0`): VISIBLE off; while dimmed, wait; then the HUD element 0x40 off (`sub_801DA48`),
  unregister, region 0, unreserve FuturePanel, destroy state.

**The curse controller** (effect #0x4E, `sub_80E6088`): a dimming controller (states: `object_timefreezeBegin`, the
update, `object_timefreezeEnd`; actions: `object_dimScreen`, `object_drawChipName` with chip 0x174, the effect,
`object_undimScreen`). Its spawner (`sub_80E61D2`) leaves the registers (the panel row, 0, 0) as its position.
The effect (`sub_80E60CC`), by phase:

- 0 (`sub_80E60EC`). Entry: the targets, `sub_801632C(side ^ 1)`: the other side's alive actors (BattleState+0x80
  or +0x90, four slots, `battle.alive_actors`) packed x | y << 4 into ExtraVars, their count into Param3. None:
  straight to `object_undimScreen` (action 0xC). Else two graphics transfers (presentation) and Timer 0x3C. Each
  later tick: over each target's panel a blinking mark (`sub_800362C` to the screen, `sub_800AE90`: drawn only;
  sound 0x91 on each frame whose global frame counter is a multiple of 16, once per target; the port keeps no
  frame counter, so gauge-speed's port counts its own ticks instead); Timer − 1; at 0, phase 4. (60 ticks.)
- 4 (`sub_80E6180`): Timer 0xA; at 0 (10 ticks), phase 8.
- 8 (`sub_80E619E`). Entry: for each target panel, `sub_80E621C`: a one-tick hit (`sub_80C53A6`: the hitbox runs
  while dimmed) there over region 1, hit spark 6, target type 5, self type 0x19, hit modifier 3, element 0, Z 0, the
  controller's damage word; and if a combatant stands there (`sub_80E7486`, the test RskyHny's bee makes, chips.md
  §3.7: the panel's flags have 0x200000 for a side-0 controller, 0x400000 for side 1, and one of the eight
  alive-actor slots has its collision on that panel), sparkles on it (`sub_80E1A6A`, effect #0x11, parameters
  0x2402, no slot). Timer 0x1E; at 0 (30 ticks), action 0xC.

**The sparkles** (effect #0x11, `sub_80E19BC`), on a related object, invisible themselves:

- Init (`sub_80E19DC`): Timer by Param1 from `dword_80E19F0` (0x20, 0x5A, 0x20); the update state.
- Update (`sub_80E1A0C`): with a slot (ExtraVars) whose pointer was cleared, the destroy state. Else Timer − 1, at 0
  the destroy state. Otherwise, when Timer & 0xF is 0xF (the routine means a mask of 3 for Param1 0, but the test
  reads stale flags and always takes 0xF): effect Param2 (0x24) at the related object's position jittered by
  `AddRandomVarianceToTwoCoords(0x1F, ...)` (**one RNG draw**), 16 pixels up; unless Param3, sound 0x6F. With
  Timer 32: sparkles on ticks with Timer 31 and 15.
- Destroy (`sub_80E1A5E`): phase init 1, `object_freeMemory`.

## 15. Spawn positions (register garbage)

| object | spawner | position until init | kept |
|---|---|---|---|
| attack #0, #1 | `sub_80C4FFE`, `sub_80C5176` | (the panel row, the caller's r2, Z) | Z |
| attack #0x12 | `sub_80C6ADA` | (the panel row, the element, the height) | Z |
| attack #0x10 | `sub_80C67D8` | (the panel row, the element, the caller's r3) | Z (0 for CornSht) |
| attack #0x3C | `sub_80CB8DE` | (the panel row, 0, 24 pixels) | Z |
| attack #0x3B | `sub_80CB686` | (the panel row, the element, 16 pixels) | Z |
| attack #0x58 | `sub_80CF91E` | (the panel row, the element, the Atk+ bonus) | none (the init sets Z) |
| attack #0x93, #0x7A | the throws | the release point | all (the flight starts there) |
| effect #0x4E | `sub_80E61D2` | (the panel row, 0, 0) | all (never drawn; `scratch_position`) |
| effect #0x11 | `sub_80E1A6A` | registers left by `sub_80E7486` | all (never drawn; `scratch_position`) |
| actor #0x54 | `sub_80C4038` | the owner's | (overwritten every tick) |

## 16. Verification

The chip lab records these in the original and the port reproduces every one (when this was written its replays
stopped at each action's "not implemented" panic; the counts are from then): 0x14 49 scenarios (Cannon, HiCannon, M-Cannon, GigaCan1-3's recipes, forms/*/ko and eight NaviCust
scenarios that fire a Cannon), 0x17 45, 0x19 45, 0x21 19, 0x24 24, 0x25 25 (with forms/falzar/cross-spout, the
SpoutCross charged shot), 0x29 25, 0x2A 27, 0x30 20 (with SuprSpr's recipe), LilBoiler 30, VDoll 10. The traces
confirm the timings above (the actions' lengths and their spawn ticks: Cannon 34, AirShot 22, Vulcan1 36, SuprVulc
113, BatCan1 38, BatCan4 70, TankCan1 39, Spreadr1 23, MachGun1 82, CornSht1 35, WideSht 27, SuprSpr 55, GigaCan1 34
with its afterimages on tick 10).

What the lab's first scenarios reached (coverage.md), and so what they verify:

- 0x14: every branch but a holder that isn't a player (`sub_80EBD68`'s 0) and a full attack pool; GigaCan's
  afterimages and burst by pa/0x140..0x142.
- 0x17 and the bullet: all but the bullet without a collision slot and kind 0x10's single effect (no SuprVulc bullet
  hits in the lab).
- 0x19 and the bat shot: all but the shot without a collision slot and the battle-over end.
- 0x24 and the shell: all but the shell without a collision slot, the battle-over end, a wait of more than a tick
  (Timer2 is always 1), Param2 0 (no crack) and an explosion row of 4 (a shell leaving along row 3; the row-0 skip,
  Param1's read, is reached).
- 0x25: all but the bullet's unreached branches (above); variant 2 by forms/falzar/cross-spout.
- 0x29: the target's sweep in its column; **not** the column moving (the target column changing, `sub_80ECF8E`'s
  block after the reversal) or `sub_80ED040`'s no-body fallback.
- 0x2A and the corn: the probe, the burst and grass; **not** the spread (`sub_80C6774`'s spawns and `sub_80C67A4`
  are unreached: no enemy body in the column beyond in any scenario), Param1 ≠ 0 at spawn, the battle-over end.
- 0x30 and the wave: kinds 3 and 6; not kinds 0..2 and 9 (dead), not the battle-over end.
- LilBoiler: the throw, the flight, landing on solid ground, the crush by a body and the explosion (hit, guard,
  counter...); **not** the eruption (no scenario lasts its lifetime or hits it three times), the hole, a removal
  request, absorption or blinking out, the layer (actor #0x54), or `sub_801B878`'s other branches.
- VDoll: the throw, the flight, the landing and its poison panel (miss, adjacent), and the crush in the air (hit: it
  collides with the enemy in its last five ticks, takes the navi's body damage, and goes quietly); **not** the curse
  (effects #0x4E and #0x11: no scenario hits a landed doll), the puff, absorption, blinking out, the lifetime's end.

The coverage scenarios (docs/engine/unverified.md) verified since: the battle-over ends of the bullet, the bat
shot, the shell, the corn, the wave and the Spreader's burst (each chip's `ko`); MachGun's target column moving
and its no-body fallback (`chips/0x02b-machgun1/moving-target`, `ko`, `invisible`); actions 0x55 and 0x5B and the
sonic boom, from their own records put in a folder (`chips/0x173-sonicbom`, `0x177-sprsonic`, `0x174-curse`,
`0x175-punisher`, `0x17d-zsaver`, the last with the fourth slash's command inside, split over and after its
window) and from the variable swords' commands (standard-chips.md); LilBoiler with a bonus, pushed, thrown
at a hole and followed by a RockCube (`chips/0x062-lilbolr1/atk10…`, `pushed`, `holes`, `replaced`); the doll's
lifetime, a push and its own side's hit (`chips/0x096-vdoll/lifetime`, `pushed`, `own-hit`).

**[unverified]** (no scenario reaches them): action 0x14 variant 3; action 0x5D (weapon 0x39: no navi or form of
a netbattle has it); the spawners' "pool full" paths; the objects' "no collision slot" paths; the wave's dead
kinds. (LilBoiler's and the doll's removal, absorption and blink-out are verified by their `dustman`, `absorbed`
and `colarmy` scenarios.)

## 17. Corrections to other documents

- chips.md §3.8 said `sub_80ED040`'s fallback (no enemy body ahead) is "the column right ahead"; it is the last
  column ahead (6 facing right, 1 facing left). Corrected there.
- chips.md §3.3's table attributed the afterimages at timer 8 of action 0x14 to TankCan; they are GigaCan's
  (variants 4..6), and TankCan is action 0x24. Corrected there. (Its Cannon frame numbers are one lower than the
  traces show, f16 for the shot and f33 for the exit against ticks 17 and 34: the entry tick doesn't count the
  timer.)
- kinds/obstacle.rs described `sub_801B878` as "destroys" while erupting; see §14.1. It is now the obstacle
  service's `"ignores"` crush.

## 18. As ported

Content model v2 (docs/design/content-model-v2.md): each action is a builder its chips compose, each object a kind
definition, each table row a variant record written out in Luau.

- **Where.** The projectile and its variants: objects/projectile (the shot programs a navi's stats name are the
  named records `shot/...`), lib/projectile; the flying shot: objects/flying-shot; the bullet: objects/bullet (its rows, and the
  variants the Vulcans, the Spreaders, SpoutCross's charged shot, ColArmy and ColForce fire). The cannons:
  lib/cannon with chips/cannon and chips/gigacan; AirShot, BatCan, MachGun: chips/airshot, chips/batcan (with its
  shot), chips/machgun; the Vulcans, the Spreaders, the TankCans, CornSht, WideSht and SuprSpr: chips/vulcan,
  chips/spreadr, chips/tankcan (with its shell), chips/cornsht (with the corn), chips/widesht (with the wave); the
  sonic boom: lib/swords/sonic_boom with chips/sonicbom and chips/z-saver; LilBoiler: chips/lilbolr (the boiler and
  the layer); VDoll: chips/vdoll (the doll, the curse and the sparkles); the rapid buster: lib/rapid_buster.
- **Chips.** Every chip is a definition naming its own action (the cannons and GigaCans, the Vulcans, the
  Spreaders, CornSht, WideSht and SuprSpr, VDoll, the sonic boom's four, Z Saver, AirShot, the BatCans, the
  TankCans, MachGun, LilBoiler). What the original picks by the record's subtype or parameters is the action's
  arguments.
- **LilBoiler's registry side** (§14.1). The boiler registers as its user's side's class-1 field object. The
  port first took the side from the Atk+ bonus (the register `sub_80D7A78` pops it into, which the spawner
  overwrites with the user's alliance before the registration): with no bonus that made every boiler side 0's,
  and with one the port stopped with an error, since a "side" of 10 would have stored the boiler's address over
  the round's Crosses-used mask. The lab settled it: with Atk+10 the original throws, lands and erupts as without
  (`chips/0x062-lilbolr1/atk10`, `atk10-twice`), both sides still pick a Cross on the next screen
  (`atk10-cross`, `atk10-cross-gregar`, against `then-cross` without the bonus), and a boiler thrown by side 1
  is side 1's (`side1-then-fan`, `side1-own-fan`: side 0's fan doesn't evict it, side 1's does). All **verified**.
- **The obstacle service** gained the crush `"ignores"` (`sub_801B878` while erupting) and the status flag
  `"carried"` (0x04000000, which the doll tests); objects gained `clear_statuses` (the eruption's status word
  cleared) and `load_or_step_sprite` (`sub_801BC24`, the doll's sprite); `battle.objects_of(kind)` walks the
  update list (`sub_80C67A4`); the collision's `element` and `secondary_element` (`sub_8019F8C`) are fields.
- **Departures.** SuprSpr keeps each wave's hit modifier in the attack's second parameter (`av[0xD]`) before the
  wave reads it among its parameters; the port hands it to the wave. CornSht's corn takes its generation from the
  chip's first parameter, 0 for all three, which the port writes as 0. The curse's marks sound every 16 frames of
  the game's frame counter, which the port doesn't keep: every 16 ticks of the marking, as gauge-speed's port does.
  The layer copies its owner's palette, colour shader, priority and blending, but the final palette and the HUD
  calls (the HP display, the HUD element) are drawn only and not kept.
- **Verified** on the traces (machgun at its floors; soundmod's second round now runs to its end, its first stops
  at JudgeMan) and on the families' lab scenarios: every one matches but BatCan4's `counter`, where the original
  keeps Full Synchro's aura through the counter's paralysis and the engine drops it at the hit (the emotion's, not
  the chip's), and those other blockers stop (dimming subtypes 21 and 29, DiveMan).
