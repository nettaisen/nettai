# Field objects: rocks and shared obstacle behavior (BN6 US Falzar, BR6E)

Obstacles are attack-pool (T3) objects without actor data that stand on a
panel, take hits, and leave the field in several ways. The rock (T3#0x59)
is the one the netbattle fixtures use: the soundmod trace's round 1 starts
with two. Engine code: `kinds/obstacle.rs` (the shared obstacle framework,
which content calls through the `obstacle` service), the registry in
`field.rs` (`FieldObjects`), generic helpers in `kinds/common.rs`. The
kinds are content's definitions (content model v2): the rock with its
variants (`byte_80CF934`'s rows, records) and its debris (T4#0x38) in
`objects/rock`; RockCube's and IceCube's controller and chips in
`chips/rockcube`; the stages' boulder (T3#0x6E) in `objects/boulder`; the
absorbed obstacle (T4#0x87) in `objects/absorbed-obstacle`, whose looks
(the original's obstacle kinds, `byte_80E98C0`) are records each obstacle
defines for itself (`absorbed_obstacle.look { sprite, ... }`). What a
stage's actor list places goes through its kind's `place` (the roles
`kinds.rock`, `kinds.boulder`, `kinds.statue` in rules/roles.luau); the
absorbed obstacle is the role `kinds.absorbed_obstacle`.

Routine names are the original's. "f1"/"f2" are the
collision data's ObjectFlags1 (+0x3C, the trace's `status`) and
ObjectFlags2 (+0x40).

## 1. Actor lists

`sub_8007368` spawns the settings' actor list (battle-flow.md §3.2). Type 8
entries are rocks: `sub_80074FA` reads x = b1 & 7, y = b1 >> 4, variant =
b2, side = the panel's current owner, and calls `sub_80CFBC4(x, y, side,
b2)` with params r4 = 0x00030000 + b2 (the literal `byte_80077F4` is
{00 00 03 00}; the disassembly comment "=0x0" is wrong) and damage r6 = 200.
Rocks are not counted as actors (no `sub_8007778`/`sub_80077D2`).

Type 3 entries are boulders (§3.1): `sub_8007450` takes the first free of
the registry's two stage slots (BattleState+0xB8, +0xBC; with both taken it
places nothing), reads x and y as above and the side from the panel's flags
(0x20: side 1's), and calls `sub_80D2430(x, y, 0, r4 = side, r6 = 200, r7 =
the slot's address)`, storing the object in the slot. Type 9 entries are
Guardian statues (dimming-chip-effects.md §5): `sub_800751C` calls
`sub_80D4FA6(x, y, the panel's side, r3 = b2, r4 = 1, r6 = 200)`, the
statue placed by a stage (Param1 1: its panel's side, no flip, no arrival
flash, running while paused); b2 only reaches the Z its spawner leaves
before its init. Type 0xA (`sub_800748A`) fills a stage slot like type 3,
with attack object #0x98 (`sub_80D85F0`, damage 100); types 1 (a virus), 2
(a random `sub_80DA9DC` object), 6 (`sub_80E5368`) and 7 (`sub_80D8D5A`)
are not in the netbattle stages' lists and are not ported (the engine's
actor lists don't decode them).

Lists with boulders: 0x080B19BF..0x080B1A14 (two each, on columns 2 and
5); with statues: 0x080B1B35 and 0x080B1B46 (at (1,1) and (6,1)).

Lists with rocks: 0x080B1A25..0x080B1B24 (e.g. soundmod round 1 uses
0x080B1AAD = navi (1,2) side 0, navi (6,2) side 1, rock 3 at (3,3), rock 3
at (4,1)).

## 2. Field-object registry (BattleState+0xA0..+0xBF)

Eight object pointers. `setFieldBattleObject_800F614(obj, side, class)`
uses base = +0xA0 + 0xC·side + 8·class:
- class 0 (two slots, +0 and +4): fill the first empty one; if both are
  full, shift (slot0 = slot1, slot1 = new) and set the evicted object's HP
  to 0 (it breaks at its next update);
- class 1 (one slot): replace, and the replaced object's HP = 0.

So per side: [class 0 oldest, class 0 newer, class 1], side 0 at +0xA0,
side 1 at +0xAC; +0xB8/+0xBC are stage-object slots (actor-list types 3
and 0xA fill them if empty). `sub_800F656` clears the object from the six
side slots; `sub_800F806` returns its class (table {0,0,1,0,0,1}, 0xFF if
absent). `sub_80EFD74` (the absorbing chip) walks all eight.

## 3. The rock, T3#0x59 (`sub_80CF954`)

**Spawn `sub_80CFBC4`** (r0 x, r1 y, r2 side, r3 garbage, r4 params, r6
damage word): `object_spawnType3(0x59, X=r1, Y=r2, Z=r3)` (register
garbage, overwritten by init before anything sees it); PanelX/Y, Alliance;
`Damage:Stamina` = r6 (u32 store); flags |= 0x10, or 0x14 when Param3 == 3;
register(obj, side, Param2).

Params: Param1 = variant (row of `byte_80CF934`), Param2 = registry class,
Param3 = entrance: 0 rise (wait for sprite animation 0 to end), 1 instant,
2 fall, 3 placed at round start (also runs while paused until it stands,
and starts with the standing animation). Content: `rock.spawn(x, y, side,
{ variant, class, entrance }, damage)` (objects/rock: the kind's state
holds the variant record and the entrance, "rise", "instant", "fall" or
"placed").

**Variants (`byte_80CF934`, 8-byte rows):** anim, (unused 0xC8), HP/2,
debris palette, break sound (u16), NameID (u16). Init makes HP/2 = 0 into
1 HP and variant >= 3 aqua. In content they are `rock.variants` (brittle,
cube, hard, ice; records with `anim`, `hp`, `element`, `debris_palette`,
`break_sound`, `name`), which an actor list's entry reaches by its row
(`rock.by_number`):

| variant | anim | HP | element | debris palette | sound | NameID |
|---|---|---|---|---|---|---|
| 0 | 1 | 1 | null | 0 | 0x97 | 0xD0 |
| 1 | 1 | 200 | null | 0 | 0x97 | 0xD0 |
| 2 | 1 | 500 | null | 0 | 0x97 | 0xD0 |
| 3 | 2 | 200 | aqua | 1 | 0xD9 | 0xD1 |

Variant 3 is an ice block: `sub_8018186` (an obstacle encased in ice runs
out) replaces the obstacle with `sub_80CFBC4` params {3, class, 1, 0}. The
soundmod round-1 stage starts with two of them.

**Init `sub_80CF974`:** flags |= VISIBLE; `sprite_load(0x10, 0)` (clears
flag 0x08); CurAnimCopy = 0xFF; CurAnim = (Param3 == 3 ? anim : 0); set
that animation, load it, one sprite update; X/Y from the panel, Z = 0;
FuturePanel = Panel; element; HP = MaxHP; NameID; Timer = 6000; collision
(freed if none: the registry entry is left dangling), setup self 0xE
(0x00810082) target 0xF (0xFF800002) hit modifier 3, hit effect 0xA,
region 0 (not on any panel yet), present; CurState = 4 and the update runs
at once.

**Update `sub_80CFA18`:** flags |= VISIBLE; `sub_801AD9E` (§4.1);
`object_spawnCollisionEffect`; `sub_800F672` (§4.2); `sub_801B394` with
the action table (§4.3); `object_updateSprite`; present collision.

**Actions (`off_80CFA48`, CurAction is an index):**

| # | routine | |
|---|---|---|
| 0 | `sub_80CFA6C` appear | Param3 2 → fall; 0 → wait until the sprite's frame parameters show 0x80; then if the panel has SOLID and none of 0x0F880080: CurAnim = anim, region 1, clear flag 0x04, action 8; else action 2 |
| 1 | `sub_80165B8` | action 8 (shared) |
| 2 | `sub_80CFB2C` destroyed | see below |
| 3,4,6,7 | `sub_80166AE`, `sub_8016B02`, `sub_8016B36`, `sub_8016B72` | shared; dispatch through the object's AIData (null for obstacles, so unreachable) |
| 5 | `sub_8017E26` | shared push/drag slide (3 sub-steps in Unk_0d) |
| 8 | `sub_80CFB28` | idle, nothing |

Fall (`sub_80CFAC0`): first tick CurAnim = anim, Zvel = 0, Z = 160 px;
each tick Zvel −= 0x4000, Z += Zvel; at Z <= 0: panel not solid → action
2 (Z stays below 0); else camera shake (1, 0x14 — RNG1 only), Z = 0, region
1, sound 0xC0, action 8. Highlights its panel every tick.

**Destroyed `sub_80CFB2C`** (runs every tick while action 2): region 0;
drop the reservation at FuturePanel; `sub_802EF5C` (only with battle flag
0x40); unregister; then by f2:
- f2 & 0x8000 (removed by a chip):
  - f2 & 0x300000 (absorbed): `sub_800F90E(4)` spawns T4#0x87 (§5);
  - else `sub_800F8CE`: not f2 & 0x40000 → T4#0 effect 0x14 at Z + 12 px;
    f2 & 0x40000 → blink: first tick PhaseInitialized = 1 and Timer = 20;
    VISIBLE = Timer & 2; `--Timer > 0` keeps the rock in action 2 (and
    returns without finishing). `sub_800F672` also decrements the same
    Timer every tick before, so the blink is ~10 ticks;
- otherwise (broken): two debris T4#0x38 at the position jittered by
  `AddRandomVarianceToTwoCoords(0xF)` (one RNG2 draw each), params = debris
  palette; T4#0 effect 2 at Z + 16 px; break sound.
Then VISIBLE off, state word = 8. Next update: `object_genericDestroy`.

Spawned objects are each inserted right after the rock, so the list reads
rock, effect, debris 2, debris 1; each debris draws RNG2 twice in its init
(same tick). A broken rock therefore costs **6 RNG2 draws**.

### 3.1 The boulder, T3#0x6E (`sub_80D2290`)

The obstacle the actor lists' type 3 entries place (§1): 500 HP, no
lifetime, one of the field's two stage objects. Content: `objects/boulder`.

**Spawn `sub_80D2430`** (r0 x, r1 y, r2 0, r4 side, r6 damage word 200, r7
the stage slot's address): `object_spawnType3(0x6E, X = y, Y = 0, Z = r3)`
with params = r4 (the side, which nothing reads); PanelX/Y; the damage
word; element 0; Alliance = side; ExtraVars[0] = the slot's address; then
the flags. It means `flags |= 0x14` (run while paused and while dimmed),
but loads the byte through r1, which holds x, not through the object:
`ldrb r2, [r1]` reads address x (1..6), inside the BIOS, which game code
can't read. The console returns a byte of the instruction word the BIOS
last fetched (open bus), by the address's low two bits; after a software
interrupt (the game's divisions, `svc 6`) that word is 0xE3A02004. So the
flags become that byte | 0x14 and the object's own (0x09: in use, sprite
not animating) are lost:

| x & 3 | byte read | flags | |
|---|---|---|---|
| 0 | 0x04 | 0x14 | not in use |
| 1 | 0x20 | 0x34 | not in use, "holds a reservation" |
| 2 | 0xA0 | 0xB4 | not in use, "holds a reservation", 0x80 |
| 3 | 0xE3 | 0xF7 | in use, visible, 0x20, 0x40, 0x80 |

The stages' boulders are on columns 2 and 5 (0xB4 and 0x34, as the lab's
stage traces show them). "Not in use" only stops the sprite stepping
(`object_updateSpriteTimestop` skips such objects: the boulder has one
animation); the slot stays allocated (the pools' bitfield) and the object
updates and is drawn as any other. The reservation bit only makes
`object_genericDestroy` look for reservations it doesn't hold. (If an
interrupt returned after the last software interrupt, the word would be
the one an interrupt's return leaves, 0xE55EC002: other stray bits, the
same behavior. The port takes the word the traces show.)

**Init `sub_80D22B0`:** `sprite_load(0x10, 8)`; CurAnim = CurAnimCopy = 0,
set and loaded, one sprite update; shadow; X/Y from the panel, Z = 0;
FuturePanel = Panel; then the whole pixels of Z and Y each −1 (Z = −1 px, Y
a pixel back: what stands on the panel's center is drawn over it); NameID
0xCE; HP = MaxHP = 500; Timer = 6000 (nothing counts it); collision (freed
if none), setup self 0xE target 0xF hit modifier 3 (region 1: on its panel
at once), present; CurState = 4. No update that tick.

**Update `sub_80D2320`:** flags |= VISIBLE; `sprite_clearFinalPalette`;
`sub_801AD9E` (§4.1); `sub_801B394` with its table (§4.3); present
collision; `object_updateSpriteTimestop`. No hit spark and no lifetime.

**Actions (`jt_80D2348`):** 0 `sub_80D236C` waits for battle flag 1 (the
fight), then flags &= ~0x04, VISIBLE, action 8; 1, 3 to 7 the shared ones
(5 `sub_8017E26`, the bounded slide); 8 `sub_80D2398` nothing; 2
`sub_80D239C` destroyed: `sub_802EF5C`; region 0; drop the FuturePanel
reservation; zero the stage slot ExtraVars[0] points at; then as the rock's
by f2: absorbed → `sub_800F90E(5)`; removed → `sub_800F8CE` (blinking: stay
in action 2; else effect 0x14 at Z + 12 px unless it blinked); otherwise
(broken) two debris T4#0x38 (palette 0) at jittered positions and T4#0
effect 1 at Z + 16 px, without a sound. Then region 0 and state word = 8
(VISIBLE stays).

Verified against the lab's stage scenarios (stages/boulder*): standing
through the round's start, the buster's and M-Cannon's hits, a breaking
hit, AirShot's push into the other boulder (from either side), RockCube on
its panel, DustCross's absorption and throw, its blink-out (ColArmy) and
its removal (BlzrdBal's ball), on every battle settings record that places
boulders. Not reachable in a netbattle: the status actions, and the flags
on columns other than 2 and 5 (unverified.md).

## 4. Shared obstacle routines

### 4.1 `sub_801AD9E` (take hits)

Only when battle flag 1 (fighting): `object_removeCollisionData`
(resolves this tick's hits). Then unless the battle is over (`tst r0` form)
or f1 & DEAD: if !(f1 & 0x40) and HitModifierFinal & 0x40 (push): f2 |=
0x100, f2 &= ~4, zero +0x80..+0x93 (`sub_801A6A6`: final damage, per-element
damage, mood, counter, drain), SlideType (+0x0F) = 1.
`object_calculateFinalDamage2` (sum of the five element damages, each
halved rounding up on a holy panel, into +0x80), `object_spawnHiteffect`
(if not paused and hit flags & 0x20000, i.e. it blocked a hit: spark 8 at
Z + 16 px, jittered: one RNG2 draw).

Variants (`obstacle.take_hits(me, push)`): `sub_801AD12` ("keeps_damage":
the same without `sub_801A6A6`, so a pushed obstacle still takes the hit's
damage; Wind's fan, TimeBom, Mine, Anubis, AirRaid, Fanfare, Guardian,
Sensor and others), `sub_801ADFA` ("any_hit": first `sub_801AE56`, which
for a hit from exactly one side's attacks/objects/bodies (0xA2000000 or
0x51000000) that isn't type 0x1000 and isn't a push ORs 0x61 into
HitModifierFinal, a one-panel push; only T3#0x3E, unreferenced), and
`sub_801AD6A` ("ignored": no push at all; Otenko's statue).

### 4.2 `sub_800F672` (lifetime)

Battle over → region 0, HP 0. Else, unless dimmed or paused: Timer −= 1
(32-bit); reaching 0 → region 0, HP 0; Timer <= 0xB4 with bit 1 set → clear
VISIBLE (blinks the last 3 s).

### 4.3 `sub_801B394` (react + dispatch)

1. Final damage != 0: (white flash, sound 0x85) `object_subtractHP`; new HP
   0 → request destroy. Else hit flags & 0x0C800002 → HP = 0 and request;
   else f2 & 0x8000 → request; else HP == 0 → request. Request = f2 |= 1.
2. f1 & DEAD → dispatch. f2 & 1 → clear it, f1 |= DEAD (0x100; the
   disassembly's `1 << OBJECT_FLAGS_INVULNERABLE` is 1 << 8), action 2,
   dispatch.
3. f2 & 0xC00 (PreventAnim = 0 first) or f1 & 0x04000000 → `sub_8018002`
   (picked up and thrown; 5 steps in PreventAnim) and return.
4. f2 & 0x3000 (PreventAnim = 0 first) or f1 & 0x30000000 → `sub_801813A`
   (encased in ice/bubble, then replaced by a rock or `sub_80D99EC`) and
   return.
5. Dimming: action 0 → dispatch; else `sub_801823C` and return.
6. PreventAnim = 0; f2 & 0x100 → clear, save the state word in Unk_5c if
   it is 0, action 5, CurPhase = 0, Unk_0d = 0; else f1 & DRAG → action 5;
   else Unk_0d = 0.
7. Dispatch: `sprite_zeroColorShader`, `sub_80181F6` (VISIBLE on unless
   dimming; if the object is not on the local side and the local player
   is BLIND, VISIBLE off), then action table[CurAction].

Variants (`obstacle.react(me, crush, hold)`): `sub_801B4D4` ("destroys":
step 1's crushing hits destroy without zeroing HP; Guardian),
`sub_801B610` ("spares_bodies": the crushing mask is 0x00800002, so bodies
don't break it; Otenko), `sub_801B750` (hold "always": step 5 holds even in
action 0; BlkBomb), `sub_801B878` (LilBolr: crushing hits destroy without
zeroing HP while its ExtraVars+4 is nonzero, i.e. "destroys" or "breaks"
by the kind's own state).

`sub_801823C` (dimming hold): `sub_80181F6`; first time (PreventAnim ==
0) save X16/Z16 in Unk_30/Unk_32, Unk_19 = 0, PreventAnim = 4 (which also
freezes the sprite); final damage != 0 → Unk_19 = 30; while Unk_19 counts
down, position = (Unk_30, Y, Unk_32) jittered by mask 3 (one RNG2 draw per
tick); else restore X16/Z16. Engine: `obstacle::State { held_x, held_z,
shake }`.

### 4.4 The pushes (action 5)

Both step through Unk_0d (0 start, 4 slide, 8 rest), keep the panels left
in PhaseInitialized and the rest in CurPhase, and on the rest's end clear
f1 DRAG and restore the state word saved in Unk_5c (zeroing it).

Start (`sub_8017E44`, `sub_8017CE0`): f1 |= DRAG; RelatedObject1 = 0; Panel
= FuturePanel, coordinates and collision panels from it; f1 &= ~0x1040;
drop the FuturePanel reservation; `sub_800F598` gives the push: side = +1
if hit flags (| collision +0x54) have side 0's attack/object/other-body
bits (0xA2000000) only, −1 for side 1's (0x51000000) only, else nothing;
the vector is `byte_800F604[i]` for the lowest set bit i of
HitModifierFinal >> 2 (0x04 (−1,0,6), 0x08 (1,0,6), 0x10 (−1,0,1), 0x20
(1,0,1)) with dx times the side. With the direction bits all clear it reads
address 4 (the BIOS): the port stops there. (+0x54 is meant to keep the hit
flags while dimmed, but `object_presentCollisionData` stores the caller's
r1: 0 undimmed, 0x10 for a held obstacle, neither with pusher bits.)
`sub_8017E26` then always goes 6 panels, its bounds in Unk_0c from
`byte_8017F24`: pushed back toward its pusher (dx != side) it stays on the
far side's panels (+0x0C 1: side 1's, SOLID|0x20 required; 2: side 0's,
0x20 forbidden), else anywhere; entering needs SOLID and none of
0x03800000 (other bodies, neutral objects); no reservations.
`sub_8017CC0` goes the vector's panel count (0 → rest), needs SOLID and
none of 0x0F880080, and reserves each panel it heads for. Blocked at the
start: rest 0x18.

Slide (`sub_8017F38`, `sub_8017D64`): X += Xvel (10 px), Y += Yvel (6 px)
until one passes the panel's center (`sub_800E6E8`), else panels from
coordinates. There: drop the reservation; an ice panel adds a panel unless
the collision's element is aqua; panels −1 > 0 and the next panel open →
head for it; else snap onto it, rest 0x14.

### 4.5 Thrown and encased (`sub_8018002`, `sub_801813A`)

Nothing in the game starts either: their requests are made only by
`sub_800F6AC` and by an unlabeled routine at 0x0800F830 (disassembled as
data after `byte_800F828`), and nothing calls those. **Unverified** (no
trace can reach them).

The throw request, `sub_800F6AC(obstacle, thrower side r1, target x r2, y
r3, shake ticks r4, damage word r6)`: +0x1C = x, +0x1D = y, +0x1E = the
shake ticks, the damage word, f2 |= 0x400 (side 0) or 0x800 (side 1).
(`sub_800F6C6`, which nothing calls either, would pick a target: a random
panel with the enemy's body, else one of the enemy's other than its own,
else any, a `GetPositiveSignedRNG2` draw each.)

**Thrown, `sub_8018002`**, by PreventAnim (`off_8018014`):
- 0 (`sub_801802C`): Alliance = 0 if f2 has 0x400, else 1; f2 &= ~0xC00;
  f1 |= 0x04000000 (carried: the dispatcher keeps coming back here);
  Zvel = (64 px − Z) / 32 (`svc 6`); the FuturePanel reservation dropped;
  +0x19 = 32; sound 0x12A; region 0; PreventAnim 4.
- 4 (`sub_8018076`): Z += Zvel; +0x19 −= 1; at 0: Z = 64 px, PreventAnim 8.
- 8 (`sub_8018094`): +0x19 = the byte at +0x1E; +0x30 = X16, +0x32 = Z16
  (the whole pixels); PreventAnim 0xC.
- 0xC (`sub_80180A8`): (X, Y, Z) = (+0x30 px, Y, +0x32 px) jittered
  (`AddRandomVarianceToTwoCoords(3)`: **one `GetRNG2` draw** a tick, x
  and z by −1..2 px); +0x19 −= 1; at 0: X16 = +0x30, Z16 = +0x32 (the
  fractions stay 0); FuturePanel = (+0x1C, +0x1D); +0x19 = the flight's
  ticks (`sub_800F768`); sound 0x10C; PreventAnim 0x10.
- `sub_800F768(x, y)`: dx, dy from the whole-pixel X, Y (X16 << 16) to the
  panel's center; the angle `calcAngle_800117C(dy, dx)` (BIOS ArcTan2 of
  the whole pixels, >> 8) goes to +0x0C (unread); (Xvel, Yvel) =
  `sub_80011A0(angle, 8 px)` (cos and −sin from `math_cosTable` and
  `byte_80066E0`, × speed >> 8); the distance = BIOS Sqrt(((dx lsr 8)² +
  (dy lsr 8)²) in 32 bits) << 8 (a logical shift and a wrapping square,
  harmless for whole-pixel offsets), ticks = distance / 8 px (`svc 6`).
  Ticks ≠ 0: Zvel = 64 px / ticks; else Zvel = 8 px, Xvel = Yvel = 0,
  ticks 8.
- 0x10 (`sub_80180EC`): X += Xvel, Y += Yvel, Z −= Zvel; +0x19 −= 1; at
  0: Panel = FuturePanel, coordinates from it; `sub_80C53A6(x, y, Element,
  z 0, r4 = 0x06050001, r6 = damage word, r7 = 3)`: region 1, hit spark 5,
  target 5, self 6, hit modifier 3, resolving while dimmed; HP 0; action 2
  (the kind's destroyed action).
- 0x14: `nullsub_57`. Engine: `obstacle::thrown`.

The encase request (0x0800F830, r0 = the obstacle, r1 = 0 ice / else
bubble, r5 = the encaser): the obstacle takes the encaser's alliance byte
and damage word; f2 |= 0x1000 (ice) or 0x2000 (bubble).

**Encased, `sub_801813A`**, by PreventAnim (`off_801814C`):
- 0 (`sub_8018154`): f1 |= 0x10000000 (f2 had 0x1000: ice) or 0x20000000
  (bubble); f2 &= ~0x3000; +0x19 = 60; the FuturePanel reservation
  dropped; region 0; PreventAnim 4.
- 4 (`sub_8018186`): VISIBLE; while bit 1 of +0x19 is clear: VISIBLE off
  and a T4#0 effect 0x42 at (X, Y, Z) (two ticks of every four, a new
  effect each). +0x19 −= 1; at 0: its registry class (`sub_800F806`,
  0 / 1, 0xFF when not registered) → r4 = 0x10000 | class << 8;
  unregistered (`sub_800F656`) and no side's wind (`sub_80E544C`); ice →
  r4 += 3: a rock (`sub_80CFBC4`, variant 3 the ice block, the same class,
  entrance 1 (instant)), bubble → r4 = 1: attack object #0xA3
  (`sub_80D99EC`: PanelX/Y, element 2 (aqua), the damage word, the
  alliance byte, flags |= 0x10; its behavior `sub_80D984C`, which only
  this reaches, isn't described); either at its panel with its alliance
  and damage word. Then state destroy (word).
- Engine: still a panic (`obstacle::encased`): the replacement spawns
  content kinds (the rock's spawner is the pack's `objects/rock`; #0xA3
  isn't ported).

### 4.6 Requests from chips

`sub_800F884` f2 |= 0x8000 (removed); `sub_800F898` also 0x40000
(blink out); `sub_800F8B0` also 0x100000 << absorber's side. DustCross's
B+Back (weapon routine 0x2A, player action 0x58 `sub_80EFCB4`: the pack's
`navis/00-megaman/weapons/2a-absorb`) calls `sub_80EFD74` on its 10th tick
(`obstacle.absorb_all`): for all eight registry slots, skip empty,
NameID 0xDA, no collision, or f2 & 0x348000, else `sub_800F8B0`.

## 5. Spawned effects

**T4#0x87 absorbed obstacle (`sub_80E97F0`).** Spawned by `sub_800F90E(kind)`
(`obstacle.fly_to_absorber(me, look)`: the kind is the obstacle's
absorbed-look record, with its sprite and what the original tests of the
kind: `keeps_parts` for kind 2, `own_facing` for sprite 0x23, the countdown
bomb's both)
at the obstacle's position with params {kind, side (f2 & 0x200000), CurAnim,
sprite palette}, and the obstacle's hidden sprite parts in ExtraVars+8;
copies Alliance/Flip; flags &= ~0x14 (0x03 after init).
Init: panel from coordinates (`sub_800E258`: x = (X/65536 + 160) / 40, y =
(Y/65536 + 32) / 24); sprite `byte_80E98C0[kind]` (rock: (0x10, 0)); CurAnim
= CurAnimCopy = Param3; kind 2 keeps the hidden parts; target = that side's
navi (`sub_80103BC`, the side's first actor if it is a player) X + 20 px
toward its facing, navi Y; Timer 9; velocities = (target − pos) / 9 (signed
division), Zvel = (16 px − Z) / 9; VISIBLE; state 4. Update: battle over or
off the field → T4#0 effect 0x12 at Z + 16 px, destroy; else move, panel
from coordinates, and while the navi's CurAction is 0x58: `--Timer <= 0` →
if the navi's AIData+0x0D < 8, store Param1 | CurAnim << 4 at AIData+0x6C +
count, count += 1, destroy (a full list leaves the timer to wrap: 65534
ticks before it tries again). Then a bare `sprite_update`. Engine: the
navi's `ActorData::absorbed` list (each entry its look's record and its
animation, which a throw moves into the attack: `thrown_look`,
`thrown_anim`); the kind is the content's `objects/absorbed-obstacle`.

**T4#0x38 rock debris (`sub_80E46D8`).** Init: VISIBLE, sprite (0x10, 1),
animation RNG2 & 1, palette Param1, r = RNG2: Xvel = ((r & 15) − 7) << 15,
Yvel = ((r >> 4 & 15) − 7) << 15, Zvel = ((r >> 8 & 15) + 12) << 15; state
4 and one update. Update: pos += vel (Z clamped at 0), Zvel −= 0x14000; at
Z == 0 spawn T4#0 effect 1 and free itself.

## 6. Trace verification (soundmod round 1)

The verification workspace's rock_trace test replays frames 72..3800 driving only the rocks and
what they spawn (navis' positions/actions/status, pause and battle flags
taken from the trace) and compares flags, params, state, panel, side, flip,
HP, position, timer, animation and status every frame. It matches:

- 72: both rocks spawn and stand in their first (paused) tick: flags 0x13,
  [4,8,0,0], HP 200, anim 2, Timer 6000.
- Timer counts only unpaused, undimmed ticks: 3193 (fight starts)
  onward, frozen while dimmed 3207–3333 and 3408–3683 (the rocks
  run then, holding position).
- 3757: side 1's navi entered action 0x58 at 3747 (first run 3748) and
  pulls at 3757; both rocks go to f1 DEAD, VISIBLE off, state 8, and spawn
  T4#0x87 (params 0x00020104) in the same tick; 3758 the rocks are freed.
- 3758–3766: the absorbed obstacles fly (velocities (145635, 0, 116508) and
  (−145635, 349525, 116508)), reach Timer 0 at 3766 (state 8) and are freed
  at 3767. No RNG draws on this path.

Not exercised by any trace: damage and breaking (debris, 6 RNG2 draws),
pushes, blink-out, falling/rising entrances, dimming shaking, eviction.

## 7. Not implemented, unreachable, unverified

- Not implemented (panic): `sub_801813A` (encased), which nothing in the
  game starts (§4.5); ported, unverified: `sub_8018002` (thrown), likewise.
  (The ice block an encased obstacle becomes is `rock.spawn(x, y, side, {
  variant = rock.variants.ice, class = its class, entrance = "instant" },
  damage)`.)
- An error: actions 3/4/6/7 on obstacles (AIData lookups through a null
  pointer; nothing sets them on an obstacle); a push without direction bits
  (`sub_800F598` reads the BIOS).
- Ported, unverified by any trace: `sub_802EF74` (battle flag 0x40 target
  tracking, never in netbattles; the side's tracked target is
  `SideState::tracked`), the pushes' ice and bounds branches, the
  take-hits and dispatcher variants but the default ones.
- Verified by the coverage scenarios (docs/engine/unverified.md): the pushes
  (`sub_8017CC0`, `sub_800F598` and the slide: AirShot at a TimeBom, a
  BlkBomb, a LilBoiler, a Guardian, a Sensor, an IceCube, and a RockCube
  pushed back by the other side), obstacles broken by damage, and the
  registry's evictions (a third RockCube, a second Anubis, a RockCube after
  a LilBoiler).
- Actor-list types other than 0, 3, 8 and 9 (§1): no netbattle stage's
  list has them.
