# Field objects: rocks and shared obstacle behavior (BN6 US Falzar, BR6E)

Obstacles are attack-pool (T3) objects without actor data that stand on a
panel, take hits, and leave the field in several ways. The rock (T3#0x59)
is the one the netbattle fixtures use: the soundmod trace's round 1 starts
with two. Engine code: `kinds/obstacle.rs` (shared), `kinds/rock.rs`,
`kinds/rock_debris.rs` (T4#0x38), `kinds/absorbed_obstacle.rs` (T4#0x87),
the registry in `field.rs` (`FieldObjects`), generic helpers in
`kinds/common.rs`. Data: `ObjectData::rocks`, `ObjectData::absorbed_sprites`
(a content pack's `objects/rock/object.toml` and
`objects/absorbed-obstacle/object.toml`, extracted by bn6-extract).

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
and starts with the standing animation). Engine: `rock::Spec`,
`rock::Entrance`.

**Variants (`byte_80CF934`, 8-byte rows):** anim, (unused 0xC8), HP/2,
debris palette, break sound (u16), NameID (u16). Init makes HP/2 = 0 into
1 HP and variant >= 3 aqua. Extracted as `RockKind { anim, hp, element,
debris_palette, break_sound, name_id }`:

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

### 4.2 `sub_800F672` (lifetime)

Battle over → region 0, HP 0. Else, unless time stop or paused: Timer −= 1
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
5. Time stop: action 0 → dispatch; else `sub_801823C` and return.
6. PreventAnim = 0; f2 & 0x100 → clear, save the state word in Unk_5c if
   it is 0, action 5, CurPhase = 0, Unk_0d = 0; else f1 & DRAG → action 5;
   else Unk_0d = 0.
7. Dispatch: `sprite_zeroColorShader`, `sub_80181F6` (VISIBLE on unless
   time stop; if the object is not on the local side and the local player
   is BLIND, VISIBLE off), then action table[CurAction].

`sub_801823C` (time stop hold): `sub_80181F6`; first time (PreventAnim ==
0) save X16/Z16 in Unk_30/Unk_32, Unk_19 = 0, PreventAnim = 4 (which also
freezes the sprite); final damage != 0 → Unk_19 = 30; while Unk_19 counts
down, position = (Unk_30, Y, Unk_32) jittered by mask 3 (one RNG2 draw per
tick); else restore X16/Z16. Engine: `obstacle::State { held_x, held_z,
shake }`.

### 4.4 Requests from chips

`sub_800F884` f2 |= 0x8000 (removed); `sub_800F898` also 0x40000
(blink out); `sub_800F8B0` also 0x100000 << absorber's side. The absorbing
chip (player action 0x58, `sub_80EFCD8`) calls `sub_80EFD74` on its 10th
tick: for all eight registry slots, skip empty, NameID 0xDA, no collision,
or f2 & 0x348000, else `sub_800F8B0`.

## 5. Spawned effects

**T4#0x87 absorbed obstacle (`sub_80E97F0`).** Spawned by `sub_800F90E(kind)`
at the obstacle's position with params {kind, side (f2 & 0x200000), CurAnim,
sprite palette}; copies Alliance/Flip; flags &= ~0x14 (0x03 after init).
Init: panel from coordinates (`sub_800E258`: x = (X/65536 + 160) / 40, y =
(Y/65536 + 32) / 24); sprite `byte_80E98C0[kind]` (rock: (0x10, 0)); CurAnim
= CurAnimCopy = Param3; target = that side's navi (`sub_80103BC`) X + 20 px
toward its facing, navi Y; Timer 9; velocities = (target − pos) / 9 (signed
division), Zvel = (16 px − Z) / 9; VISIBLE; state 4. Update: battle over or
off the field → T4#0 effect 0x12 at Z + 16 px, destroy; else move, panel
from coordinates, and while the navi's CurAction is 0x58: `--Timer <= 0` →
if the navi's AIData+0x0D < 8, store Param1 | CurAnim << 4 at AIData+0x6C +
count, count += 1, destroy. Then a bare `sprite_update`. Engine: the navi's
`ActorData::absorbed` list.

**T4#0x38 rock debris (`sub_80E46D8`).** Init: VISIBLE, sprite (0x10, 1),
animation RNG2 & 1, palette Param1, r = RNG2: Xvel = ((r & 15) − 7) << 15,
Yvel = ((r >> 4 & 15) − 7) << 15, Zvel = ((r >> 8 & 15) + 12) << 15; state
4 and one update. Update: pos += vel (Z clamped at 0), Zvel −= 0x14000; at
Z == 0 spawn T4#0 effect 1 and free itself.

## 6. Trace verification (soundmod round 1)

`kinds/rock_tests.rs` replays frames 72..3800 driving only the rocks and
what they spawn (navis' positions/actions/status, pause and battle flags
taken from the trace) and compares flags, params, state, panel, side, flip,
HP, position, timer, animation and status every frame. It matches:

- 72: both rocks spawn and stand in their first (paused) tick: flags 0x13,
  [4,8,0,0], HP 200, anim 2, Timer 6000.
- Timer counts only unpaused, non-time-stop ticks: 3193 (fight starts)
  onward, frozen during the time stops 3207–3333 and 3408–3683 (the rocks
  run then, holding position).
- 3757: side 1's navi entered action 0x58 at 3747 (first run 3748) and
  pulls at 3757; both rocks go to f1 DEAD, VISIBLE off, state 8, and spawn
  T4#0x87 (params 0x00020104) in the same tick; 3758 the rocks are freed.
- 3758–3766: the absorbed obstacles fly (velocities (145635, 0, 116508) and
  (−145635, 349525, 116508)), reach Timer 0 at 3766 (state 8) and are freed
  at 3767. No RNG draws on this path.

Not exercised by any trace: damage and breaking (debris, 6 RNG2 draws),
pushes, blink-out, falling/rising entrances, time-stop shaking, eviction.

## 7. Not implemented (panic)

- `sub_8018002` (thrown), `sub_801813A` (encased), action 5 `sub_8017E26`
  (push/drag slide).
- Actions 3/4/6/7 on obstacles (AIData lookups, unreachable).
- `sub_802EF74` (battle flag 0x40 target tracking; not set in netbattles).
- Actor-list types other than 0 and 8.
