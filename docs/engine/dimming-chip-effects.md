# Dimming chip effects: subtypes 2, 3, 7, 8, 12, 14–19, 21–24, 29, 30, 32, 37 (BN6 US Falzar, BR6E)

The controllers of the dimming chips `off_802CCB4` indexes (docs/engine/chips.md §3.6, §3.6.10) and every
object they spawn, routine by routine, from the disassembly. The chip lab's scenarios (bn6battle-verify
`data/traces/lab`) name the branches they reach; a branch no scenario reaches is marked **unverified**.

Conventions: routine names are the original's. "Tn#x" is object type n (1 actor, 3 attack, 4 effect), index x.
"f1"/"f2" are the collision data's ObjectFlags1/ObjectFlags2; "self"/"target" are collision type indices into
`byte_8019C7C` (field-collision-damage.md §3.3). A timer that "counts n" is set to n and decremented each tick,
the action moving on when the test the text gives first holds; "(u16)" marks a halfword timer whose test is
on the 32-bit result of `ldrh; sub #1` (so it goes negative). Sprites are "category-index" in hex, as
`sprite_load(r0, category, index)` takes them. Z offsets are in pixels (the game adds `n << 16`).

## 0. The controllers' common shape

Every controller is a T4 object with the standard state table:

| state | routine |
|---|---|
| 0 init | `object_timefreezeBegin` (battle flag 4 if its side started the dimming) |
| 4 update | by CurAction: 0 `object_dimScreen`, 4 `object_drawChipName` (or a variant), 8 the effect, 0xC `object_undimScreen` |
| 8 destroy | `object_timefreezeEnd` |

(chips.md §3.6.2 has the phases' timing.) The spawner, `off_802CCB4[subtype]`, is called by action 0x15
(`sub_80EBD9C`) and by the cut-in (`sub_8017AB4`) with r0/r1 = the user's panel, r2 = the attack's element,
r4 = its params, r5 = the user, r6 = the damage word (damage | hit param << 16), r7 = chip | bonus << 16. It
spawns with `object_spawnType4(index)` while r1..r3 hold the caller's panel Y, element and whatever r3 was,
so the controller's X, Y, Z are register garbage that nothing reads (the port's `scratch_position`). Unless a
section says otherwise the spawner stores PanelX = r0, PanelY = r1, Element = r2, Params = r4,
RelatedObject1 = the user, AllianceAndDirectionFlip = the user's (halfword), Damage word = r6 and
+0x30 = r7 (+0x30 the chip, +0x32 the bonus). A failed spawn returns 0 (nothing registers). The effect
action's first tick is the one where `PhaseInitialized` is 0.

"Effect ends" below means CurAction = 0xC with phase and PhaseInitialized 0 (a halfword store of 0 to
+0xA), which starts `object_undimScreen` the next tick.

## 1. Subtype 8: Wind and Fan (T4#0x25, T3#0x48)

Chips 0x81 Wind (Param1 0) and 0x82 Fan (Param1 1); damage 0.

**Spawner `sub_80E3128`**: T4#0x25; the common fields.

**Effect `sub_80E30E0`** (T4#0x25's update `sub_80E309C` → `sub_80E30BC`): on its first tick, if the panel in
front (PanelX + `object_getFrontDirection`, PanelY) has flags 0x10 and none of 0x0F880080 (solid, free),
`sub_80CD2B4(x, y, r2 = AllianceAndDirectionFlip, r3 = 0, r4 = Params, r6 = damage word)` places a fan there.
Timer = 0x1E (u16); the effect ends when it goes negative: **31 ticks**. No sound.

**The fan's spawner `sub_80CD2B4`**: T3#0x48 (position = registers); Damage word = r6, PanelX/Y, the
alliance/flip halfword, flags |= 0x10 (runs while dimmed); `setFieldBattleObject_800F614(obj, side = the
halfword's low byte, class 1)` (one a side: a second fan replaces the first, whose HP becomes 0).

**The fan, T3#0x48 (`sub_80CCFDC`)**. States: init `sub_80CCFFC`, update `sub_80CD08C`,
`object_genericDestroy`.

- Init: VISIBLE; `sprite_load(0x80, 4, 9)` (04-09), no shadow, palette Param1; CurAnim/CurAnimCopy = 0,
  animation 0 set, loaded and stepped once; flip from the object; coordinates from the panel, Z = 0;
  FuturePanel = Panel; Element = 0; NameID = 0xD6 + Param1 (0xD6 Wind's, 0xD7 Fan's); ExtraVars+0 =
  `&byte_80CCFD4[Param1 * 4]`; HP = MaxHP = its halfword +2; Timer (lifetime) = 0x5A0 (1440 ticks);
  collision (none: `object_freeMemory`, the registry entry left dangling), setup self 0x13, target 0x14, hit
  modifier 0, region 1, present; state update, and the update runs at once.
- `byte_80CCFD4` (4 bytes by Param1): {wait 1, gust Param1 0, HP 40} Wind; {1, 1, 40} Fan.
- Update: VISIBLE; `sprite_clearFinalPalette`; `sub_801AD12` (take hits; a push keeps the damage);
  `sub_800F672` (lifetime); `sub_801B394` (react, the default crush, the hold after appearing) with the table
  `off_80CD0B8`; `object_updateSprite`; `object_presentCollisionData`.
- Actions (`off_80CD0B8`): 0 appear `sub_80CD0E0`; 1 `sub_80165B8`; 2 destroyed `sub_80CD1BE`; 3, 4, 6, 7 the
  actor reactions (`sub_80166AE`, `sub_8016B02`, `sub_8016B36`, `sub_8016B72`: unreachable on an obstacle);
  **5 `sub_8017CC0`** (knocked back as far as the hit says, not the six-panel slide); 8 `sub_80CD0F8`; 9
  `sub_80CD144`.
- 0 appear: sound 0x94; Timer2 = the row's wait (1); CurAction 8, phase 0.
- 8, phase 0 (`sub_80CD114`): sound 0xAD; `sub_80E541A(self, alliance, 0)`: the fan becomes its side's wind
  (BattleState+0xC0 + 4·side = the object, +0xC8 + 4·side = source 0 "obstacle"); a wind already there is
  destroyed at once (`sub_80E5410`: its state word = 8, its ExtraVars+0 = 0; no destroyed action, no
  unregistering). Phase 4.
- 8, phase 4 (`sub_80CD12C`): Timer2 −= 1; at 0: ExtraVars+0xC (the row counter, u32) = 3, CurAction 9, phase 0.
  (Timer2 is 1, so this is the next tick.)
- 9, phase 0 (`sub_80CD160`): if ExtraVars+4 and ExtraVars+5 (bytes) are both nonzero, wait. (Nothing sets
  them: a gust's slot pointer is the ROM row, below; the destroyed action zeroes the word.) Row counter −= 1,
  0 → 3: the rows go 2, 1, 3, 2, 1, 3, ... one step per attempt. Pick the gust's panel on that row (below); none
  (x = 0) → stay in phase 0 (the counter steps again next tick). Else `sub_80CD488(x, row, r2 = the
  alliance/flip halfword, r4 = the row's gust Param1, r6 = 0, r7 = ExtraVars+0)`: a gust (T3#0x49,
  `objects/gust`) with Param1 0 (Wind: pushes away) or 1 (Fan: pulls, hit modifier 4), damage 0. Its
  "holder" pointer is the ROM row itself: the gust's `strb 1, [r7]` is a write to ROM (ignored) and the byte
  it later tests is the row's wait (1): the gust is always "tracked" (the port passes no holder). Timer2 =
  0xC, phase 4.
- 9, phase 4 (`sub_80CD1AE`): Timer2 −= 1; at 0 phase 0. So a gust every **12 ticks**.
- Wind's panel (`sub_80CD26C`): `object_getClosestPanelMatchingRowFiltered(r0 = alliance ^ flip, row,
  byte_80CD2A4[alliance])`: from column 6 − 5·(alliance ^ flip) toward the back (step −1 for 0, +1 for 1)
  while valid, the first panel meeting {require, forbid} = side 0 {0, 0x20}, side 1 {0x20, 0} (the fan's
  side's own panel); none → 0. Then x += the front direction (the enemy's first column). If that panel's
  flags have 0x00800000 (a neutral object: another obstacle) → 0 (no gust), else x. (With no own panel on
  the row x becomes ±1: **unverified**.)
- Fan's panel (`sub_80CD236`): x = (alliance ^ flip ^ 1)·5 + 1 (the far column: 6 for side 0), stepping
  toward the fan (−front direction) while the panel has 0x00800000; stepping onto 0 or 7 → 0 (no gust).
- 2 destroyed (`sub_80CD1BE`): ExtraVars+4 = 0. By f2:
  - 0x8000 (removed by a chip): with 0x300000 (absorbed) `sub_800F90E(0)` (absorbed obstacle kind 0);
    else `sub_800F8CE` (blink out): still blinking → return (stays in action 2); blink done → go on with no
    effect; not a blink-out → T4#0 effect 0x14 at (X, Y, Z + 12).
  - else (broken, its HP 0 or crushed): sound 0x70, T4#0 effect 0 at (X, Y, Z).
  - Then `sub_802EF5C` (target tracking, battle flag 0x40 only), `sub_800F656` (unregister),
    `sub_80E544C(self)` (no side's wind), collision region 0, the FuturePanel reservation dropped, VISIBLE
    off, state word = destroy.
- The wind registry is read by the navis' AI wind (T4#0x41, `sub_80E532C`): with a fan's wind on its side it
  doesn't blow. Nothing else in PvP reads it.
- T4#0x26 (`sub_80E3150`, spawned only by `sub_80E31C0`, which nothing calls) is dead code.

**Lab**: 19 Wind and 10 Fan scenarios. `adjacent` (the opponent in front) and `obstacle` (a RockCube in front)
place no fan; the others place one and blow Wind's rows 2, 1, 3 from the enemy's front column and Fan's from
the far column. `chips/0x081-wind/counter` and `chips/0x082-fan/broken` break a fan (the broken branch);
`chips/0x081-wind/then-fan` and `both-sides` have a second fan (the replaced fan's HP 0 and the wind registry's
destroy) and `chips/0x081-wind/lifetime` runs the 1440 ticks out: all **verified**. **Unverified**: no gust
(an obstacle on the target panel, `sub_80CD160`'s wait), removal, blink-out, absorption, pushes (action 5:
`chips/0x082-fan/pushed`'s AirShot breaks the fan instead), a flipped fan.

## 2. Subtype 23: BurnSqr1–3 (T4#0x59, T1#0x13)

Chips 0x6E, 0x6F, 0x70 (fire, damage 100/120/140, hit param 30, params 0).

**Spawner `sub_80E70A6`**: T4#0x59; the common fields but only the **alliance** byte (not the flip).

**Effect `sub_80E7070`** (T4#0x59 `sub_80E702C` → `sub_80E704C`): first tick: `sub_80BB13E(PanelX, PanelY,
Element, r5 = the user, r6 = damage word + the bonus (+0x32, added to the whole word), r7 = &Param2)`, the
burner navi; then each tick (the first included) the effect ends once Param2 is 0 (the burner clears it; a
failed spawn leaves it 0, ending the effect on its first tick).

**The burner's spawner `sub_80BB13E`**: T1#0x13 (position = registers); PanelX/Y, Element, damage word,
RelatedObject1 = the user, ExtraVars+0 = r7 (the controller's flag) and `*r7 = 1`, the user's
alliance/flip halfword and NameID, flags |= 0x10.

**The burner, T1#0x13 (`sub_80BAF50`)**: a copy of the user's navi that aims a 2×2 square and sets it on
fire. States: init `sub_80BAF74`, update `sub_80BB00C`, `object_freeMemory`; `object_updateSprite` after
each.

- Init: the user's sprite: `sub_800F29C(user.NameID)` (the actor record: type, index); a player (type 2)
  takes `sub_800FC9E(navi stat 0x29, stat 0x2C)` (the navi and its form: MegaMan (0x29 = 0) category 0,
  index `byte_800FCBC[form]`; other navis category 8, index `byte_800FCD5[stat 0x2C]`); any other NameID
  `sub_800F26C(NameID)`. `sprite_load(0x80, …)`, load the animation data, shadow on, CurAnim 0,
  CurAnimCopy 0xFF, VISIBLE; position = the user's (X, Y, Z copied), panels from the coordinates; palette
  = the user's navi palette (`sub_801002C`); flip; `sub_8010DF6(the user's actor record, 1)` (its navi parts,
  stepping even while paused); `sub_80E1352(user, 0xF)`: the user vanishes (VISIBLE off, status 0x100000)
  and its charge glow (AIData+0x58: +0x64 = 0) and Full Synchro aura (AIData+0x5C: +0xC = 0) hide; mask bits
  0x1 barrier visual, 0x2 confusion visual, 0x4 blindness visual, 0x8 the HUD are **kept**. State update
  (word store), and the update runs at once.
- Update: VISIBLE, then off if `sub_800EB6C(alliance)` says the local viewer is blind to it; by CurPhase:
- Phase 0 (`sub_80BB048`): `dword_80BB094` = bytes {5, 2, 1, 6}: the square's start column
  (ExtraVars+0xC, also the current column ExtraVars+0x14) = byte[alliance] (5 side 0, 2 side 1), its
  turning column (ExtraVars+0x10) = byte[alliance + 2] (1, 6); row (ExtraVars+0x18) = 1. Draw the square
  (`sub_80BB23E`); sound 0x10F; Timer2 = 300; Timer = 6; phase 4 (PhaseInitialized 0), which runs at once.
- Phase 4 (`sub_80BB098`): Timer2 −= 1; 0 → fire. Else Timer −= 1; 0 → Timer = 6, step the square
  (`ho_80BB170`), sound 0x10F. Then the user's A in the dimming key record (AIData+0x2C bit 0) → fire. Fire:
  sound 0xBD, phase 8.
- The square's steps (`ho_80BB170`, on PhaseInitialized as a sub-state; d = the front direction): 0: row 2,
  → 4. 4: column −= d; at the turning column → 8. 8: row 1, → 0xC. 0xC: column += d; at the start column →
  0. Then `sub_80BB23E`. So side 0's square goes (5,1), (5,2), (4,2), (3,2), (2,2), (1,2), (1,1), (2,1), …,
  (5,1), (5,2), …: over the whole field, every 6 ticks.
- Drawing the square (`sub_80BB23E`): four T4#0 effects look 0x48 at the centers of (column, row),
  (column + d, row), (column + d, row + 1), (column, row + 1) (from the panel center: +40 px·d, then +24 px
  down, then −40 px·d; Z 0), each with Timer = 6 (gone after 6 updates).
- Phase 8 (`sub_80BB0D2`): first tick Timer = 16. Each tick: while Timer2 & 4 is clear, highlight hit
  region 2 (the panel and the one in front) from (column, row) and (column, row + 1) turned to the alliance
  (`sub_8109660`: only panels in 1..6 × 1..3); Timer2 −= 1; Timer == 4 → CurAnim 0xC; Timer −= 1 (u16);
  negative → `sub_80BB1FE`, Timer = 60, phase 0xC.
- `sub_80BB1FE`: on (column, row), (column + d, row), (column + d, row + 1), (column, row + 1), each whose
  flags have 0x10 (solid): `sub_80C8DE0(x, y, element 1, r4 = 0x1E04, r6 = the damage word)`, a flame
  (T3#0x26, `objects/heat-flame`: Param1 4, so it acts and animates while dimmed; Param2 30; fire; the
  burner's alliance and flip).
- Phase 0xC (`sub_80BB10C`): Timer −= 1 (u16); negative (61 ticks) → `sub_8011044(the user's actor record,
  1)` (its navi parts off), VISIBLE off, state destroy (word), `sub_80E13DC(user)` (the user back: VISIBLE
  unless submerged or hidden from a blind viewer, the vanished bit cleared, the barrier, confusion and
  blindness visuals and the HUD shown, the charge glow and aura back), and `*ExtraVars+0 = 0` (the
  controller goes on).

**Lab**: 28 scenarios (BurnSqr1 20, BurnSqr2 and 3 4 each). Every one fires at the 300-tick
timeout on four solid panels. A to fire (`chips/0x06e-burnsqr1/a-fires`) and a non-solid panel under the square
(`a-fires-holes`) are **verified**. **Unverified**: a non-player user (`sub_800F26C`), the blind viewer, the
failed spawn.

## 3. Subtype 15: GrabBnsh and GrabRvng (T4#0x22, T3#0x46)

Chips 0xA4 GrabBnsh (damage 20) and 0xA5 GrabRvng (40); null, params 0, hit param 138.

**Spawner `sub_80E2D76`**: T4#0x22; the common fields but only the alliance byte.

**Effect `sub_80E2CE8`** (T4#0x22 `sub_80E2CA4` → `sub_80E2CC4`), by CurPhase:

- 0 (`sub_80E2D08`), one tick: `sub_80E2D98` takes the side's panels back: for x 1..6, y 1..3 (x outer),
  each panel `sub_800D618(x, y, alliance)` says is the side's and stolen (its home alliance, PanelData+4, is
  the controller's side and differs from its current alliance): a T4#0 effect look 0x3B at the panel's center
  (Z 0), `object_setPanelAllianceTimerShort(x)` (the column's return timer = 1: it returns at the panel tick
  after the dimming), count + 1. ExtraVars+0 = the count. None → Timer = 0x1E and the effect ends at once.
  Else sound 0x91, Timer = 0x1E, phase 4.
- 4 (`sub_80E2D30`): Timer −= 1 (u16); when negative (every **31 ticks**): Timer = 0x1E, strike
  (`sub_80E2DE0`); no target → phase 8; else ExtraVars+0 −= 1, and at 0 → phase 8. So one strike per
  stolen panel while the enemy is there.
- 8 (`sub_80E2D54`): Timer = 0x3C, ends when negative (61 ticks).
- The strike (`sub_80E2DE0`): `byte_80E2E24[alliance]` = {require, forbid}: side 0 {0x00200000, 0}, side 1
  {0x00400000, 0} (the enemy player navi's panels). For x 6..1, y 3..1 (x outer), each panel meeting it:
  `sub_80CCD4A(x, y, Element, r4 = the strikes so far this round, r6 = damage word + the bonus)`. Returns
  the number spawned.

**The hand's spawner `sub_80CCD4A`**: T3#0x46 (position = registers), PanelX/Y, Element, damage word, the
alliance byte (not the flip), flags |= 0x10 (runs while dimmed). Its Param1 is the count before it: only a
round's first hand makes sounds.

**The hand, T3#0x46 (`sub_80CCC48`)**: states init `sub_80CCC68`, update `sub_80CCCBC`, free (`sub_80CCD42`:
`object_freeMemory`).

- Init: VISIBLE; `sprite_load(0x80, 0xC, 0x13)` (0C-13), no shadow; CurAnim 0, CurAnimCopy 0xFF; Zvel =
  0x80000; X, Y = the panel's center, Z = 192 px; Param1 0 → sound 0xA1. State update; the update runs at
  once.
- Update: the action (`off_80CCCD4`), then `object_updateSprite`.
- 0 (`sub_80CCCE0`): Z −= Zvel (8 px); while Z > 0 (signed) wait: it lands on its 24th update. Z = 0;
  Param1 0 → sound 0xA2; `sub_80C53A6(x, y, element 0, z 0, r4 = 0x1905FF01, r6 = damage word, r7 = 0x4B)`:
  a one-tick hit region that resolves while dimmed (region 1, hit effect 0xFF, target 5, self 0x19; hit
  modifier 0x4B, status 0, no bug). Then it ORs 0x10 into the byte at address r7 = 0x4B (the BIOS: no
  effect). Action 4.
- 4 (`sub_80CCD24`): CurAnim 1; action 8.
- 8 (`sub_80CCD30`): once the sprite's frame flags have 0x80 (the animation's last frame), CurState =
  destroy (byte store): freed at its next update.

**Lab**: 22 scenarios (GrabBnsh 19, GrabRvng 3). In the 21 that reach the effect no panel is stolen: each
ends the effect on its first tick. The panel return, the strikes and the hand are **verified** by
`chips/0x0a4-grabbnsh/after-areagrab`, `after-panelgrabs` and `chips/0x0a5-grabrvng/after-areagrab`.

## 4. Subtype 2: no chip (T4#0x23)

No chip has subtype 2; nothing reaches it (**unverified**, and unreachable in the game).

**Spawner `sub_80E2F24`**: T4#0x23; the common fields but only the alliance byte.

**Effect `sub_80E2E78`** (T4#0x23 `sub_80E2E34` → `sub_80E2E54`), by CurPhase:

- 0 (`sub_80E2E94`): first tick Timer = 0xA. Each tick the user's sprite is forced white
  (`sprite_forceWhitePalette` on RelatedObject1); Timer −= 1 (u16); negative (11 ticks) → phase 4.
- 4 (`sub_80E2EBC`): first tick: Timer = 0x3C; `sub_80E11E0` with r4 = 0x13C00 (a palette flash, T4#0xA:
  variant 0, 60 ticks, going on while dimmed); sound 0xBC; `sub_80C53A6(PanelX, PanelY, Element, z 0, r4 =
  byte_80E2F18[alliance] (side 0 0x3405FF82, side 1 0x3405FF81), r6 = 0, r7 = 0x3000)`: a hit that resolves
  while dimmed over the whole enemy area (region 0x82 / 0x81), hit effect 0xFF, target 5, self 0x34,
  **damage 0**, hit modifier 0, status 0x30 (blind, 480 ticks). Then each tick: Timer −= 1; negative (61
  ticks) → the effect ends; else the user's sprite forced white.

## 5. Subtype 14: Guardian (T4#0x52, T3#0x7D, T4#0x53)

Chip 0x97 Guardian (null, damage 200, hit param 158, params 0). The pack has scripts for all three kinds
(`objects/guardian`, `guardian-statue`, `guardian-strike`), checked against this section, but no chip or
`object.toml` registers them yet (so the lab stops at "subtype 14 not implemented").

**Spawner `sub_80E67E6`**: T4#0x52; the common fields (alliance and flip).

**Effect `sub_80E679C`** (T4#0x52 `sub_80E6758` → `sub_80E6778`): first tick: if the panel in front meets
{0x10, not 0x0F880080}: `sub_80D4FA6(x, y, r2 = Element, r3 = the bonus, r4 = Params, r6 = damage word + the
bonus)`. Timer = 0x1E; the effect ends when Timer −= 1 reaches **0** (30 ticks).

**The statue's spawner `sub_80D4FA6`**: T3#0x7D (position = registers: panel Y, element, the bonus);
PanelX/Y and the damage word (**not** the element: the statue's is 0). Param1 0: the spawner's
alliance/flip halfword; else the panel's current alliance (PanelData+3) and flip 0. Flags |= 0x10
(Param1 0) or 0x14 (also runs while paused). `setFieldBattleObject_800F614(obj, alliance, class 1)`.
**Stages** (`sub_800751C`, actor-list entry type 9): x = b1 & 7, y = b1 >> 4, r2 = the panel's alliance,
r3 = b2, Params = 1, damage 200.

**The statue, T3#0x7D (`sub_80D4C84`)**: states init `sub_80D4CA4`, update `sub_80D4D34`,
`object_genericDestroy`.

- Init: `sprite_load(0x80, 0xC, 0x35)` (0C-35), load the animation data, shadow on, CurAnim = CurAnimCopy
  = 0, animation 0 set, loaded, stepped once; flip; coordinates from the panel, Z = 0; FuturePanel =
  Panel; NameID 0xE3; HP = MaxHP = 1; Timer (lifetime) 6000; Timer2 = 3. Param1 ≠ 1 → T4#0 effect 0x15 at
  (X, Y, Z + 16). Collision (none: `object_freeMemory`), self 0xE, target 0xF, hit modifier 3, region 0,
  present. State update (byte store); the update first runs next tick. (VISIBLE is set by the update.)
- Update: VISIBLE; `sprite_clearFinalPalette`; `sub_801AD12` (a push keeps the damage); `sub_800F672`;
  `sub_801B4D4` (react: crushing hits destroy with the HP as it is) with `off_80D4D60`;
  `object_updateSprite`; present.
- Actions: 0 appear `sub_80D4D88`; 1 `sub_80165B8`; 2 destroyed `sub_80D4EFC`; 3/4/6/7 the actor reactions;
  5 `sub_8017CC0`; 8 `sub_80D4DC8` (nothing); 9 strike back `sub_80D4DCC`.
- 0 appear: Timer2 −= 1; at 0: its panel not {0x10, not 0x0F880080} → run the destroyed action now; else
  region 1, sound 0x94, action 8 (so it takes its panel on its 3rd update).
- 2 destroyed (every tick while in it): drop the FuturePanel reservation, `sub_802EF5C`, unregister; by f2:
  - 0x8000 with 0x300000: `sub_800F90E(9)` (absorbed obstacle kind 9), then region 0, VISIBLE off, state
    destroy (no sound);
  - 0x8000 otherwise: `sub_800F8CE`: blinking → return; done → sound 0x90, region 0, VISIBLE off, destroy;
    not a blink-out → effect 0x14 at Z + 12, then the crumble;
  - otherwise (broken): if HP is 0 and Param2 is 0: ExtraVars+0 = the collision's hit flags
    (FlagsFromCollision); nonzero and `sub_80D4FF6` gives a side → action 9, phase 0, return. Else the
    crumble.
  - The crumble: T4#0 effect 0x14 at Z + 16, `sub_80E1D7A` (T4#0x14, a rising bubble, Params 0, at the
    statue's X, Y, Z), sound 0x90, region 0, VISIBLE off, state destroy (word).
- `sub_80D4FF6(hits)`: only side 1's attacks, objects or bodies (0x55000000, none of 0xAA000000) → 0; only
  side 0's (0xAA000000, none of 0x55000000) → 1; else 0xFF. The side that strikes back is the one opposite
  the attacker.
- 9, phase 0 (`sub_80D4DF0`): while dimmed, wait. PreventAnim = 0, region 0. Battle over, or no side from
  ExtraVars+0 → region 0, effect 0x14 at Z + 16, VISIBLE off, state destroy (word). Else Alliance = that
  side; `sub_801DACC(0x40)` (HUD); `sub_80E6878(PanelX, PanelY, Element, r3 = 0, r4 = Params, r6 = damage
  word, r7 = 0x175)` (the strike's dimming controller); `sub_800BF16(side, 1, it)` (the dimming starts,
  its user the statue; the other side can't cut in); CurPhase 4 (byte store).
- 9, phase 4 (`sub_80D4E64`): wait for Param3 (the strike controller sets it). Then `sub_80C53A6(PanelX,
  PanelY, Element, z 0, r4 = byte_80D4EB4[alliance], r6 = damage word, r7 = 3)`: region 0x85 (side 0) or
  0x84 (side 1): every panel with the enemy player navi; hit effect 0xFF, target 5, self 0x17, hit modifier
  3; resolving while dimmed. `sub_801BD3C(r0 = PanelY, r1 = Element, r2 = 0x45, r3 = the params word (its
  low byte the region), r4 = alliance)`: T4#0 look 0x45 on each panel of the whole-field region (from (6,3)
  back to (1,1), Z 0; the bogus panel only matters to a hit region's sparks). Camera shake (1, 0x3C: the
  local RNG only). Sound 0x12E. CurAnim 1. CurPhase 8.
- 9, phase 8 (`sub_80D4EBC`): once the frame flags have 0x80: effect 0x14 at Z + 16, CurPhase 0xC.
- 9, phase 0xC (`sub_80D4EE0`): region 0, VISIBLE off; once not dimmed, state destroy (word).

**The strike's controller, T4#0x53 (`sub_80E680C`)**, spawned by `sub_80E6878`: PanelX/Y, Element,
RelatedObject1 = the statue, the alliance byte, the damage word, +0x30 = 0x175 (the telop names chip 0x175;
bonus 0), Params = the statue's. Standard phases; its effect (`sub_80E6850`): first tick, the statue's
Param3 = 1; Timer = 0x3C; ends when Timer −= 1 reaches 0 (60 ticks).

**Lab**: 20 scenarios; `adjacent` and `obstacle` place no statue, the other 18 place one on a free panel
(Param1 0) that stands through the scenario. Breaking it (the crumble and the strike back, the strike's dimming
and hit) is **verified** from both sides (`chips/0x097-guardian/punish`, `own-hit`), with AirShot at it
(`pushed`) and a second statue (`replaced`). **Unverified**: the stage statue (Param1 1), its lifetime
running out, removal, blink-out, absorption.

## 6. Subtype 16: Meteors (T4#0x34, T3#0x56, T4#0x35)

Chip 0x8B Meteors (fire, damage 40, hit param 138, params 0).

**Spawner `sub_80E4288`**: T4#0x34; the common fields (alliance and flip).

**Effect `sub_80E41CC`** (T4#0x34 `sub_80E4188` → `sub_80E41A8`), by CurPhase (`off_80E41E0`: 0, 4, 8 =
`object_dimScreen` (unreachable: phase 4 goes to 0xC), 0xC):

- 0 (`sub_80E41F0`), one tick: `sub_80E42AA(&ExtraVars+0, alliance, flip)` lists the target panels as
  bytes x | y << 4 from ExtraVars+0 and returns the count, kept in ExtraVars+0x14. 0 → the effect ends.
  Else Timer = 2, Timer2 = 0x1E (the meteors left), ExtraVars+0x18 (the list index) = 0, phase 4.
- 4 (`sub_80E4220`): Timer −= 1; not 0 → wait. Timer = 10; Timer2 −= 1; negative → phase 0xC. Else the next
  index is index + 1, wrapping to 0 at the count; the meteor goes to the list entry at the **old** index:
  `sub_80CF5B2(x, y, Element, r4 = 1, r6 = damage word + the bonus)`. So **30 meteors**, the first 2 ticks
  after phase 0, then one every 10 ticks, cycling through the list.
- 0xC (`sub_80E4266`): Timer = 0x5A; ends when negative (91 ticks).
- The list (`sub_80E42AA`): step = −(the controller's front direction); x starts at 6 − 5·(alliance ^ flip)
  (the far column). While the column has no panel of another alliance (`sub_800D5F0(x, alliance)`: rows 1..3
  whose PanelData alliance differs), step; leaving 1..6 → count 0. From the first such column on: rows 1..3
  whose alliance differs are appended; a column with none, or stepping off 1..6, ends the list. On the
  usual field side 0's list is (6,1), (6,2), (6,3), (5,1), (5,2), (5,3), (4,1), (4,2), (4,3). At most 18
  entries (ExtraVars+0x14 follows the 20-byte list).

**The meteor's spawner `sub_80CF5B2`** → `sub_80CF594`: T3#0x56 (position = registers), PanelX/Y, Element,
damage word, the spawner's alliance/flip halfword; then flags |= 0x10 (acts while dimmed). (The instant
chips' meteor shower, `lib/instant/meteor_shower`, spawns the same kind through `sub_80CF594` without the flag.)

**The meteor, T3#0x56 (`sub_80CF3BE`)**: states init `sub_80CF3DC`, update `sub_80CF488`,
`object_genericDestroy`.

- `byte_80CF3AC` (6 bytes by Param1; three rows, all {0x31, 0, 0xA, 5, 1, 0}): sprite index (category 0xC),
  animation, self type, target type, camera shake, panel effect. Param1 ≥ 3 reads past it.
- Init: VISIBLE; ExtraVars+0 = the row; `sprite_load(0x80, 0xC, row[0])` (0C-31), CurAnim = CurAnimCopy =
  row[1], set, loaded, stepped; no shadow; flip; coordinates from the panel; d = the front direction: Xvel
  = 16 px·d, Zvel = 16 px, X −= 256 px·d, Z = 256 px; Timer = 0x1E. Collision (none: freed), setup
  (row[2], row[3], hit modifier 3), `sub_801A146` (the hit spark = Element & 0xF), region 0, present.
  `sub_80E43F6(r1 = PanelX, r2 = PanelY, r4 = 0x100)`: its panel marker. State update (byte store); the
  update first runs next tick.
- Update: `object_removeCollisionData` (resolve), `object_spawnCollisionEffect` (hit spark), the action,
  `object_updateSprite`, present. (The code after its return is unreachable.)
- 0 (`sub_80CF4C8`): Timer −= 1; at 0 (30 updates): Timer = 0x10, sound 0xC4, action 4.
- 4 (`sub_80CF4E6`): Timer −= 1; not 0 → X += Xvel, Z −= Zvel (15 moves). At 0: coordinates from the panel,
  Z = 0; if its panel's flags have any of 0x0F800010 (solid or a body): T4#0 effect 5 at (X, Y, Z), sound
  0x70, camera shake (row[4], 0xF) if row[4] (the local RNG only), region 1. Then action 8 and VISIBLE off
  (landed or not).
- 8 (`sub_80CF554`), the next update, after the hits resolved: by row[5]: 0 nothing; 1 `object_crackPanel`;
  3 `GetPositiveSignedRNG2() & 1` 0 → crack (1 → nothing); other → `object_breakPanel_dup2`. (Every row has
  0.) Then VISIBLE off, region 0, state destroy (word). So a meteor's hit is live for one tick.

**The marker, T4#0x35 (`sub_80E4344`)**, spawned by `sub_80E43F6` with position (PanelX, PanelY, garbage),
params r4, the spawner's alliance.

- Init `sub_80E4364`: PanelX/Y = the low bytes of X and Y; an invalid panel → `object_freeMemory`. ExtraVars+0
  = `&byte_80E4334[Param1 * 4]`, ExtraVars+4 = Param2 (the region highlighted: 1); Timer = row[0]; Param2,
  Param3, Param4 = row[1], row[2], row[3]. State update (word), and the update runs at once.
- `byte_80E4334`: {45, 10, 5, 0}, {30, 10, 5, 0}, {40, 10, 5, 0}, {15, 6, 3, 0} (lifetime, blink period,
  lit ticks, period shrink).
- Update `sub_80E43A8`: not dimmed and the battle over → state destroy (word). Else Timer −= 1; 0 → state
  destroy (and it goes on this tick). Param3 − 1 ≥ 0 → Param3 = that, highlight region ExtraVars+4 at its
  panel for its alliance (`object_highlightPanelRegion`). Param2 −= 1; 0 → Param3 = row[2], Param2 = row[1]
  − Param4, Param4 doubles (from 0: stays 0). So Param1 0's marker lights its panel 5 ticks of every 10 for
  45 ticks.

**Lab**: 20 scenarios, all 30 meteors on the enemy's three columns; only `stage-holes` has a meteor land on a
non-solid, empty panel (no hit). The lists after area changes (`chips/0x08b-meteors/after-areagrab`, `grabbed`)
and the battle's end (`ko`) are **verified**. **Unverified**: an empty list (the enemy owning no panel), a marker
off the field, rows other than Param1 1.

## 7. Subtype 17: Anubis and PoisPhar (T4#0x33, T3#0x55)

Chips 0x98 Anubis (params 0, damage 1, hit param 0) and 0x156 PoisPhar (the PA; Param1 1, damage 1, hit
param 128).

**Spawner `sub_80E4164`**: T4#0x33; the common fields (alliance and flip).

**Effect `sub_80E4130`** (T4#0x33 `sub_80E40EC` → `sub_80E410C`): first tick: `sub_80CF374(PanelX + the front
direction, PanelY, r4 = Params, r6 = damage word)`, with **no panel check**; Timer = 0x78; the effect ends
when Timer −= 1 is no longer > 0 (120 ticks).

**The statue's spawner `sub_80CF374`**: T3#0x55 (position = registers), PanelX/Y, the damage word, the
spawner's alliance/flip halfword, flags |= 0x10; `setFieldBattleObject_800F614(obj, r1 = the whole
alliance/flip halfword, class 1)`: with a flipped user the "side" is alliance + 0x100·flip and the store
lands outside the registry (BattleState + 0xA8 + 0xC·side): the port must refuse it (**unverified**; no
player is flipped in PvP scenarios).

**The statue, T3#0x55 (`sub_80CF0D0`)**: states init `sub_80CF0F0`, update `sub_80CF18C`,
`object_genericDestroy`.

- `byte_80CF0C0` (8 bytes by Param1): sprite category, index, poison period, animation, HP (u16), lifetime
  (u16): Anubis {0xC, 0x30, 6, 0, 100, 3000}; PoisPhar {0xC, 0x30, 1, 1, 100, 1441}.
- Init: VISIBLE; ExtraVars+0 = the row; `sprite_load(0x80, 0xC, 0x30)` (0C-30), shadow on; CurAnim =
  CurAnimCopy = the row's animation, set, loaded (not stepped); flip; coordinates from the panel, Z = 160 px;
  FuturePanel = Panel; NameID = 0xDB + Param1; Zvel = 8 px; Element 0; Timer (lifetime) and HP = MaxHP from
  the row; ExtraVars+4 (the bubble countdown) = 1; Timer2 (the poison countdown) = 1. Collision (none:
  freed), self 0x13, target 0x14, hit modifier 3, region 0, present. State update (word); the update runs at
  once.
- Update: VISIBLE; `sprite_clearFinalPalette`; `sub_801AD12`; `sub_800F672`; `sub_801B394` (the default
  crush and hold) with `off_80CF1B8`; `object_presentCollisionData`; `object_updateSprite` (in that order).
- Actions: 0 fall `sub_80CF250`; 1 `sub_80165B8`; 2 destroyed `sub_80CF1DC`; 3/4/6/7 the actor reactions;
  5 `sub_8017CC0`; 8 poison `sub_80CF2B4`.
- 0 fall: Z −= 8 px; while Z > 0 wait (it lands on its 20th update, the init's included). Z = 0. Its panel
  not solid (`object_isCurrentPanelSolid`) → region 0, HP 0, action 2. Solid: camera shake (2, 0x1E: the
  local RNG only); the panel {0x10, not 0x0F880080} → region 1, sound 0xC0, action 8; else (something on
  it) region 0, HP 0, action 2.
- 8 poison: if `battle_isBattleOver`'s flags say over (the `bne` right after the call: only time running
  out, `battle.time_up`) → action 2. Else ExtraVars+4 −= 1; reaching 0 (or wrapping from 0): ExtraVars+4 =
  0x10 and a bubble: `sub_80CF332` picks a panel: `object_getPanelsExceptCurrentFiltered` with
  `byte_80CF364[alliance]` (side 0 {0x20, 0}, side 1 {0, 0x20}: the enemy's panels other than its own, rows
  3..1, columns 6..1); none → no bubble; else **one `GetPositiveSignedRNG2` draw**, the entry at draw mod
  count (`svc 6`'s remainder): a T4#0x14 (`sub_80E1D7A`, rising bubble, Params 1) at the panel's center, Z 0.
  Then Timer2 −= 1; not > 0 → Timer2 = the row's period and
  `object_spawnCollisionRegion(PanelX, PanelY, element 5, z = Z, r4 = byte_80CF31C[alliance] (side 0
  0x5200FF82, side 1 0x5200FF81), r6 = 0x00810001, r7 = 0)`: a hit over the whole enemy area (region
  0x82/0x81), hit effect 0xFF, target 0, self 0x52, element 5 (the silent drain), **damage 1**, counter byte
  0x81 (can't counter), hit modifier 0. It has no flag 0x10: it doesn't resolve while dimmed. So Anubis drains
  1 HP every 6 ticks, PoisPhar every tick, for their lifetimes (3000 and 1441 ticks, counting only undimmed,
  unpaused ticks).
- 2 destroyed: region 0, drop the FuturePanel reservation; by f2: 0x8000 with 0x300000 → `sub_800F90E(3)`
  (absorbed obstacle kind 3); 0x8000 otherwise → `sub_800F8CE`: blinking → return, done → on, not a
  blink-out → effect 0x14 at Z + 12; not removed (broken) → sound 0x70, T4#0 effect 0 at Z + 16. Then
  VISIBLE off, `sub_802EF5C`, unregister, state destroy (word).

**Lab**: 19 Anubis scenarios and PoisPhar's recipe (`pa/0x156-poisphar/recipe1`). In `adjacent` and
`obstacle` the statue lands on an occupied panel and breaks (sound 0x70, effect 0); in the others (and the
recipe) it poisons, with bubbles, through the scenario. Breaking by damage (`chips/0x098-anubis/broken`), the
lifetime running out (`lifetime`), AirShot at it (`pushed`) and a second statue (`replaced`) are **verified**.
**Unverified**: a non-solid landing panel, time up, removal, blink-out, absorption, the flipped user, an
enemy with no panel for a bubble.

## 8. Subtype 19: CircGun (T4#0x5E, T3#0x89, T3#0x8A)

Chip 0x8E CircGun (null, damage 150, hit param 148, params [2, 4, 0, 0]: step period, shots, look).

**Spawner `sub_80E7600`**: T4#0x5E; the common fields (alliance and flip).

**Effect `sub_80E75AC`** (T4#0x5E `sub_80E7568` → `sub_80E7588`): first tick: `dword_80E75FC` = bytes {6, 1,
1, 1}: by alliance ^ flip, the start panel (6, 1) or (1, 1) (the far column's top). If that column has no
panel of another alliance (`sub_800D5F0`) the effect ends at once. Else `sub_80D67A6(column, 1, Element, r3
= 0, r4 = Params, r6 = damage word + the bonus, r7 = &Unk_0c)`, the gun, which sets Unk_0c; then the effect
waits (from the same tick) until Unk_0c is 0 (a failed spawn: at once).

**The gun's spawner `sub_80D67A6`**: T3#0x89 (position = registers), PanelX/Y, Element, damage word, the
alliance/flip halfword, RelatedObject1 = r7 and `*r7 = 1`, flags |= 0x10.

**The gun, T3#0x89 (`sub_80D655C`)**: states init `sub_80D6580`, update `sub_80D65E0`, `object_genericDestroy`;
each tick after the state routine `object_updateSpriteTimestop` (its sprite steps while dimmed).

- Init: `sprite_load(0x80, 0x10, 0x45)` (10-45), load the animation data, no shadow; CurAnim 1, CurAnimCopy
  0xFF; `sprite_setAnimation` with r0 still 0xFF (animation 0xFF), loaded and stepped (the real animation 1
  loads at the first sprite update); VISIBLE; flip; `sub_80D67D2` (coordinates from the panel, Y += 8 px, Z
  = 8 px); palette Param3; Xvel = −(front direction) (a column step toward the user), Yvel = +1 (a row
  step down); Timer2 = 360; Unk_0c = 0xFF (aiming). State update (word); the update runs at once.
- Actions: 0 `sub_80D65FC`, 4 `sub_80D677C`.
- 0: while aiming (Unk_0c = 0xFF): Timer2 −= 1; negative → fire; else `sub_80103BC(alliance)` (the side's
  first actor if it is a player: the loop re-reads the same slot) exists and its AIData+0x2C (the dimming
  key record) has A → fire. Fire: Unk_0c = Param2 (the shots). Then the panel step (by CurPhase, below).
  If PanelXY changed: aiming → sound 0x10F; firing → Unk_0c −= 1, a shot at the **old** panel
  (`sub_80D68E0(old x, old y, Element, r4 = Param3 << 16 | (Param1·4 + 0x14), r6 = damage word, r7 =
  &ExtraVars+4 + ExtraVars+0)`: its flag byte is the next of ExtraVars+4..+7), ExtraVars+0 += 1; the last
  shot (Unk_0c 0) → VISIBLE off, action 4.
- Phase 0, the vertical step (`sub_80D6694`): first tick Timer = Param1; Timer −= 1 (u16); negative: (x, y
  + Yvel) valid → move there (`sub_80D67D2`), PhaseInitialized 0 (a step every Param1 + 1 ticks). Else
  Yvel = −Yvel, Timer 0, CurPhase 4 with PhaseInitialized 1 (the horizontal step comes next tick).
- Phase 4, the horizontal step (`sub_80D66E2`): the same timing; at its step: going back (Xvel ≠ the front
  direction) into a column with no panel of another alliance → turn; the next column off the field → turn;
  the next column holding three of the user's side's panels (`object_getPanelsInColumnFiltered` with
  `byte_80D676C[alliance]`: side 0 {0, 0x20}, side 1 {0x20, 0}) → turn; else move (x + Xvel, y) and stay in
  phase 4. Turn: Xvel = −Xvel, Timer 0, CurPhase 0 with PhaseInitialized 1. So on the usual field side 0's
  gun runs (6,1), (6,2), (6,3), (5,3), (4,3), (4,2), (4,1), (5,1), (6,1), (6,2), … every 3 ticks (1 tick
  after a turn).
- 4: while ExtraVars+4 (the four flag bytes as a word) is nonzero, wait; then Timer = 0x1E; negative (31
  ticks) → `*RelatedObject1 = 0` (the controller goes on), state destroy (word).

**The shot's spawner `sub_80D68E0`**: T3#0x8A (position = registers), PanelX/Y, Element, damage word, the
alliance byte (no flip), RelatedObject1 = r7 and `*r7 = 1`, flags |= 0x10.

**The shot, T3#0x8A (`sub_80D67EC`)**: states init `sub_80D6810`, update `sub_80D685A`, `object_genericDestroy`;
`object_updateSpriteTimestop` after each.

- Init: sprite 10-45, no shadow, CurAnim 1, CurAnimCopy 0xFF; VISIBLE; flip; `sub_80D67D2`; Timer = Param1
  (0x1C for CircGun); palette Param3; sound 0xBD. State update (word); the update runs at once.
- 0 (`sub_80D6874`): Timer −= 1 (u16); negative (29 updates) → CurAnim 1, action 4.
- 4 (`sub_80D688C`): if its panel is solid: `sub_80C53A6(x, y, Element, z 0, r4, r6 = damage word, r7 =
  byte_80D68D8[Param3])` with r4 = 0x0A050001 (Param3 0: region 1, hit effect 0, target 5, self 0xA) or
  0x2B050001 (self 0x2B), r7 = 0x00000003 (hit modifier 3) or 0x32540003 (hit modifier 3, bug 0x54 with
  argument 0x32); resolving while dimmed; T4#0 effect 0x21 at (X, Y, Z); sound 0xB9. Then (solid or not)
  `*RelatedObject1 = 0`, state destroy (word).

**Lab**: 20 scenarios, all firing at the 360-tick timeout, every shot on a solid panel. A to fire
(`chips/0x08e-circgun/a-fires`, `a-fires-late`), the cursor after an AreaGrab (`after-areagrab`) and shots on
non-solid panels (`holes`) are **verified**. **Unverified**: a start column without enemy panels, Param3 1 (no
chip), a non-player first actor.

## 9. Subtype 18: Otenko (T4#0x5F, T3#0xAD)

Chip 0x99 Otenko (null, damage 1, params 0; not a folder chip in the standard library).

**Spawner `sub_80E76D4`**: T4#0x5F; the common fields (alliance and flip).

**Effect `sub_80E7668`** (T4#0x5F `sub_80E7624` → `sub_80E7644`): first tick: the panel in front against
`byte_80E76C4[alliance]` (both sides {0x10, 0x0F880080}); free → `sub_80DB2C6(x, y, Element, r3 = 0, r4 =
Params, r6 = damage word)` **with r5 = the user** (RelatedObject1). Timer = 0x3C; the effect ends when Timer
−= 1 is no longer > 0 (60 ticks).

**The statue's spawner `sub_80DB2C6`**: T3#0xAD (position = registers); `sub_801155A`: PanelX/Y, Element 0,
the damage word, the user's alliance/flip halfword, RelatedObject1 = the user; flags |= 0x10;
`setFieldBattleObject_800F614(obj, the user's alliance, class 0)` (two a side; a third evicts the oldest).

**The statue, T3#0xAD (`sub_80DB0E4`)**: states init `sub_80DB108`, update `sub_80DB148`,
`object_genericDestroy`; then `object_updateSprite` again after the state routine (the update steps the
sprite twice a tick).

- Init: coordinates from the panel, Z = 0; `sub_8011504(0x01000C00, 0x13140300)`: `sprite_load(0x80, 0xC,
  0)` (0C-00), load the animation data, shadow on (the top byte 1), CurAnim 0, CurAnimCopy 0xFF; collision,
  self 0x13, target 0x14, hit modifier 3, flip. (No collision: the flip isn't set and the init goes on
  anyway, presenting through a null pointer: **unverified**.) Present; Timer (lifetime) 1800; NameID 0xCF;
  HP = MaxHP = 100; ExtraVars+0 (the last hand index) = 0xFF, ExtraVars+4 (the last turn) = 0,
  ExtraVars+8 (the count) = 0. State update (word); the update runs at once.
- Update: VISIBLE; `sprite_clearFinalPalette`; `sub_801AD6A` (take hits, never pushed); `sub_800F672`;
  `sub_801B610` (react: bodies don't break it) with `off_80DB174`; `object_updateSprite`; present.
- Actions: 0 appear `sub_80DB198`; 1 `sub_80165B8`; 2 destroyed `sub_80DB252`; 3/4/6/7 the actor reactions;
  **5 `sub_8017E26`** (the six-panel slide); 8 the blessing `sub_80DB1E0`.
- 0 appear: first tick: its panel not {0x10, not 0x0F800000} (solid, no body) → action 2. Else Timer2 = 5,
  sound 0x94. Timer2 −= 1; not > 0 (5 ticks) → region 1, action 8.
- 8 (`sub_80DB1E0`): first tick CurAnim 1, Timer2 = 0. Timer2 −= 1; not > 0 → Timer2 = 5 (so its first
  tick, then every 5): `sub_800ED90(user)` (a player: the chip at its hand's cursor; else the object's
  +0x2A); that chip's record flags (+9) without bit 1 (no damage) → nothing (the empty hand's 0xFFFF reads
  ROM byte 0x30 past the table: nothing). Else, with the side's hand block (`sub_8010018`, 0x020349C0 +
  0x50·side): the cursor i and its turn byte (+0x3E + i) against ExtraVars+0/+4 (then stored there): the
  same entry as last time and the count ≥ 50 → nothing; a new entry → the count = 0. Then the entry's Atk+
  bonus (+0x1A + 2i, u16) += 1 and the count += 1. So the next damaging chip gains +1 every 5 ticks, up to
  +50 each.
- 2 destroyed: region 0; drop the FuturePanel reservation; `sub_802EF5C`; unregister; f2 & 0x300000
  (absorbed) → `sub_800F90E(0xE)`; else `sub_800F8CE`: blinking → return; done → on; not a blink-out → HP
  left (removed by a chip) T4#0 effect 0x14 at Z + 12, HP 0 effect 2 at Z + 16 and sound 0x90. Then VISIBLE
  off, state destroy (word).

**Lab**: 19 scenarios; `adjacent` and `obstacle` place no statue; in the other 17 it stands and blesses, but
the next chip is never a damaging one. `chips/0x099-otenko/bonus` (a Cannon next), `broken` and `pushed` are
recorded for the bonus and the statue's ends; the port doesn't replay them yet. **Unverified**: the bonus itself,
the 50 cap, a new hand entry, the non-player user, the body check at appearing, the slide, breaking, removal,
blink-out, absorption, eviction by a third field object, the lifetime.

## 10. Subtype 21: BlzrdBal (T4#0x58, T1#4, T3#0xB2, T3#0xB7)

Chip 0xC7 BlzrdBal (aqua, damage 150, hit param 158, params 0).

**Spawner `sub_80E7008`**: T4#0x58; the common fields but only the alliance byte.

**Effect `sub_80E6FCC`** (T4#0x58 `sub_80E6F88` → `sub_80E6FA8`): first tick: Timer = 0x78; `sub_80B8BA0(PanelX,
PanelY, Element, r5 = the user, r6 = damage word + the bonus, r7 = &Param2)`, the thrower. Timer −= 1 (u16);
the effect ends when it goes negative (**121 ticks**; it doesn't wait for the thrower's flag).

**The thrower's spawner `sub_80B8BA0`**: T1#4 (position = registers), PanelX/Y, Element, damage word,
RelatedObject1 = the user, ExtraVars+0 = r7 and `*r7 = 1`, the user's alliance/flip halfword and NameID,
flags |= 0x10.

**The thrower, T1#4 (`sub_80B8A18`)**: states init `sub_80B8A3C`, update `sub_80B8AD4`, `object_freeMemory`;
`object_updateSprite` after each.

- Init: as BurnSqr's burner (§2): the user's sprite (navi and form, or `sub_800F26C`), shadow, CurAnim 0,
  CurAnimCopy 0xFF, VISIBLE, the user's position, panels from it, the user's navi palette, flip, its navi
  parts (`sub_8010DF6(record, 1)`), `sub_80E1352(user, 0xF)`. State update (word); the update runs at once.
- Update: VISIBLE (off for a blind viewer, `sub_800EB6C`); by CurPhase:
- 0 (`sub_80B8B08`): first tick Timer = 0x22. When Timer is 4: its panel not solid (flags & 0x10) → Timer
  = 1, phase 4 (no throw); else CurAnim 0xC, sound 0x132. Timer −= 1 (u16); negative (35 ticks) →
  `sub_80DBB0C(r0 = X + 10 px·front, r1 = Y, Element, r6 = damage word)` (the ball, by coordinates), sound
  0x1AB, Timer = 0x5A, phase 4.
- 4 (`sub_80B8B6E`): Timer −= 1 (u16); negative (91 ticks) → navi parts off (`sub_8011044(record, 1)`),
  VISIBLE off, state destroy (word), the user back (`sub_80E13DC`), `*ExtraVars+0 = 0`.

**The ball's spawner `sub_80DBB0C`**: T3#0xB2 (position = registers), then X = r1, Y = r2, Z = 0, Element,
damage word, the alliance and flip bytes, flags |= 0x10.

**The ball, T3#0xB2 (`sub_80DB994`)**: states init `sub_80DB9B8`, update `sub_80DBA50`, `object_genericDestroy`;
`object_updateSprite` after each.

- Init: `sprite_decompress(0x10, 0x47)` (graphics only); panels from the coordinates; `sprite_load(0x80,
  0x10, 0x47)` (10-47), shadow on; VISIBLE; CurAnim = CurAnimCopy = 0, set and loaded; Param1 (the size) =
  0; palette 0; flip; `sub_80DC38A` with r7 = &ExtraVars+0: its roller (below), kept in RelatedObject2
  (none: `object_freeMemory`); collision (none: freed), self 4, target 5, hit modifier 3; Xvel = 3 px·front;
  hit effect 0xFF; present; ExtraVars+4 = Damage & 0xFFF (the growth step). State update (word); the update
  first runs next tick.
- Update: `object_removeCollisionData`, `object_spawnCollisionEffect`, the action (`sub_80DBA7C`), present.
  (The code after it is unreachable.)
- `sub_80DBA7C`: a hit this tick (FlagsFromCollision) → region 0. If ExtraVars+0 (the roller, while it
  lives) is set and the roller's Param1 says it swallowed an obstacle (last tick): sound 0x196, the
  collision's SelfDamage += ExtraVars+4, Param1 += 1, and while Param1 < 3 CurAnim = `dword_80DBB08` bytes
  {1, 1}[Param1 − 1] (the big ball). Then X += Xvel, panels from the coordinates,
  `object_updateCollisionPanels`; a new column → region 1 (it can hit again). A panel without 0x10 (not
  solid) or off the field → region 0, state destroy (word). So it rolls to the field's end hitting once a
  column, and doesn't stop at a navi.

**The roller's spawner `sub_80DC38A`** (r5 = the ball, r7 = &ball.ExtraVars+0): T3#0xB7 (position =
registers), RelatedObject1 = the ball, ExtraVars+0 = r7 and `*r7 = the roller`, the ball's alliance and flip
bytes, flags |= 0x10.

**The roller, T3#0xB7 (`sub_80DC260`)**: states init `sub_80DC280`, update `sub_80DC2D4`, `object_genericDestroy`
(no sprite).

- Init: X16 = the ball's X16 + 4·front (pixels), Y16, Z16 copied; panels from the coordinates; damage word 0;
  Param1 0; collision self 0x58, target 0x49 (every attack, object and body), hit modifier 0, hit effect
  0xFF, present; state update (word). No collision: `*ExtraVars+0 = 0`, `object_freeMemory`.
- Update: `object_removeCollisionData`, `object_spawnCollisionEffect`; the battle over (`tst` form) → region
  0, `*ExtraVars+0 = 0`, state destroy (word). Else the action and present.
- The action (`sub_80DC310`): Param1 = 0. A hit this tick: `sub_80DC3B2` looks through the objects it hit
  (`sub_801A4DC`: the collision's hit-slot mask +0x7C, slot order, at most 4) for the first whose NameID
  **word** (NameID with the +0x2A half) is 0xCD..0xFF and not 0xD3, 0xDA, 0xE9 or 0xEA (`sub_800F486`),
  kept in RelatedObject2: found → `sub_800F884` on it (removed: f2 |= 0x8000), region 0, Param1 = 1;
  else region 0. Then X16 = the ball's X16 + 20·front, panels from the coordinates, update the collision's
  panels, a new column → region 1; a panel not solid or off the field → region 0, `*ExtraVars+0 = 0`, state
  destroy (word).

**Lab**: 20 scenarios; the ball always rolls from a solid panel. `obstacle` swallows the RockCube (the
removal, the growth, anim 1). A non-solid thrower panel (`chips/0x0c7-blzrdbal/no-footing`), the roller's
battle-over end (`ko`) and RockCubes down its row (`three-rocks`) are **verified**. **Unverified**: the excluded
NameIDs, more than 4 hit objects.

## 11. Subtype 24: Magnum (T4#0x5B, T1#0x14)

Chip 0x8D Magnum (null, damage 130, hit param 148, params 0).

**Spawner `sub_80E723E`**: T4#0x5B; the common fields but only the alliance byte.

**Effect `sub_80E7208`** (T4#0x5B `sub_80E71C4` → `sub_80E71E4`): first tick `sub_80BB49C(PanelX, PanelY,
Element, r5 = the user, r6 = damage word + the bonus, r7 = &Param2)`, the gunner; each tick (the first
included) the effect ends once Param2 is 0.

**The gunner's spawner `sub_80BB49C`**: T1#0x14, as BlzrdBal's thrower's (§10): PanelX/Y, Element, damage
word, RelatedObject1 = the user, ExtraVars+0 = r7 and `*r7 = 1`, the user's alliance/flip halfword and
NameID, flags |= 0x10.

**The gunner, T1#0x14 (`sub_80BB2A0`)**: states init `sub_80BB2C4` (as BurnSqr's burner's, §2), update
`sub_80BB35C`, `object_freeMemory`; `object_updateSprite` after each. The update: VISIBLE (off for a blind
viewer), then by CurPhase. PhaseInitialized is the cursor's mode (0 a column, 4 a row).

- 0 (`sub_80BB398`): row (ExtraVars+0x18) = 2; the far column (ExtraVars+0xC) = `byte_80BB3F4[alliance]`
  (6, 1); stepping back from it (−front direction) while the column has a panel of another alliance
  (`sub_800D5F0`), then one step forward: the enemy's nearest column, ExtraVars+0x10 and the cursor column
  ExtraVars+0x14. (Off the field `sub_800D5F0` counts nothing, so the search always stops.) Draw the cursor
  (`sub_80BB59A(0x49, 12)`); sound 0x10F; Timer2 = 300; Timer = 12; phase 4, which runs at once.
- 4 (`sub_80BB3F6`): Timer2 −= 1; 0 → fire. Else Timer −= 1; 0 → Timer = 12, step the cursor
  (`sub_80BB4CE`), sound 0x10F. Then the user's A in the dimming key record → fire. Fire: ExtraVars+0x1C =
  the mode, phase 8.
- The cursor's step (`sub_80BB4CE`, d = the front direction): column mode (`sub_80BB4F8`): at the far column
  → column = far − d, row 1, row mode; else column += d. Row mode (`sub_80BB514`): row ≥ 3 → column = the
  nearest enemy column, row 2, column mode; else row += 1. Then draw it. On the usual field side 0's cursor
  goes column 4, 5, 6, row 1, 2, 3 (centered on column 5), column 4, …, 12 ticks each.
- Drawing (`sub_80BB59A(look 0x49, 12)`): T4#0 effects look 0x49 with Timer 12, at the cursor panel's center
  and, in column mode, 24 px up and down (the column's three rows), in row mode 40 px left and right.
- 8 (`sub_80BB42E`): first tick Timer = 0x10. Timer2 −= 1; Timer == 4 → CurAnim 0xC; Timer −= 1 (u16);
  negative (17 ticks) → sound 0xB4, `sub_80BB52E`, Timer = 0x3C, phase 0xC.
- The shot (`sub_80BB52E`): hit region 4 (column mode: (0,0), (0,−1), (0,1)) or 0x1D (row mode: (0,0),
  (1,0), (−1,0)), its offsets **not** turned by the facing, from the cursor panel: on each, `sub_80C53A6(x,
  y, Element, z 0, r4 = 0x0A050601, r6 = damage word, r7 = 3)` (region 1, hit effect 6, target 5, self 0xA,
  hit modifier 3, resolving while dimmed), `object_breakPanel_dup2(x, y)` (breaks it, or cracks it with
  something on it), T4#0 effect 0 at its center (Z 0). Then sound 0x10F and camera shake (1, 0x1E).
- 0xC (`sub_80BB46A`): Timer −= 1 (u16); negative (61 ticks) → the navi parts off, VISIBLE off, state destroy
  (word), the user back (`sub_80E13DC`), `*ExtraVars+0 = 0`.

**Lab**: 20 scenarios, all firing at the 300-tick timeout in column mode. A to fire
(`chips/0x08d-magnum/a-fires`) and the row-mode shot (region 0x1D: `a-fires-late`) are **verified**. **Unverified**: a non-player user,
panels off the field (the hit regions and breaks go through the field's own bounds checks).

## 12. Subtype 3: Geddon and the capsules (T4#0x1D, T4#0x1E)

Chips 0xA7 Geddon (Param1 1), and, not folder chips, 0x17F PrpCapsl (Param1 0), 0x180 PnkCapsl (1), 0x181
HealBall, 0x182 MagPanl, 0x186 and 0x187 BeastOut (0); all damage 0. PrpCapsl and PnkCapsl lack the dimming
flag (they dim, but can't be cut in with).

**Spawner `sub_80E2566`**: T4#0x1D; the common fields but only the alliance byte.

**Effect `sub_80E2528`** (T4#0x1D `sub_80E24E4` → `sub_80E2504`): first tick: `object_reservePanel` on the
user's panel (the controller holds it: the panel counts as occupied), Param2 = 1, `sub_80E2712` with r3 =
&Param2 and r4 = Param1: the quake (T4#0x1E; position = registers; Params = r4, the alliance byte,
RelatedObject1 = &Param2). Each tick (the first included), once Param2 is 0: the user's panel's reservation
dropped, the effect ends.

**The quake, T4#0x1E (`sub_80E25D0`)**: states init `sub_80E25F0`, update `sub_80E2628`, destroy
`sub_80E268C` (`*RelatedObject1 = 0`, `object_freeMemory`).

- `byte_80E2588` (0x18 bytes by Param1): +1 the list builder (0 whole field, 4 its own row: every row has 0),
  +4 require / +8 forbid (every row {0x10, 0}: solid panels), +0x14 the panel routine: Param1 0
  `object_crackPanelDup1`, 1 `object_breakPanel_dup3`, 2 `object_panel_setPoison`.
- Init: ExtraVars+0 = the row; the builder; Timer = 8; state update (byte); the update runs at once.
- The whole-field builder (`sub_80E269A`): rows 3..1, columns 6..1: each panel meeting the row's condition
  appended at ExtraVars+8 (x | y << 4); ExtraVars+4 = the count; count > 0 → `sub_8000C72(list, count,
  count)`: `count` swaps, each of list[r1 mod count] and list[r2 mod count] with **two
  `GetPositiveSignedRNG2` draws** (`svc 6`'s remainders): 2·count draws. (The row builder `sub_80E26D8`
  does the same over columns 6..1 of its own PanelY: no row uses it.)
- Update (`sub_80E2628`): Timer −= 1; not 0 → wait. Timer = 8; ExtraVars+4 −= 1; negative → CurState =
  destroy (byte store: the destroy routine runs next update). Else the entry at the new count (the list from
  its end): the panel routine on it; 0 (nothing changed) → nothing more. Else at the panel's center (Z 0):
  Param1 2 → `sub_80E1D7A` (T4#0x14 rising bubble, Params 1) and sound 0x90; else T4#0 effect 2 and sound
  0x97. So a panel every 8 ticks, the first on the quake's 8th update.
- The panel routines (each only on a solid panel, else 0): `object_crackPanelDup1`: not cracked → cracked
  (type 3, flags |= 0x40); cracked and nothing on it (none of 0x0F880080) → broken (type 1); cracked with
  something on it → 0. `object_breakPanel_dup3`: nothing on it → broken; else cracked. `object_panel_setPoison`:
  poison (type 4, flags | 0x114). The first two play sound 0x97 and the last 0x90 themselves, before the
  quake's own sound. (Each rewrites the flags word's low bits and the type and display bytes.)

**Lab**: 19 Geddon scenarios (Param1 1): the break on free panels and the crack under navis and the reserved
user; `counter-cut-in` and `stage-holes` skip non-solid panels. **Unverified**: Param1 0 (crack) and 2
(poison): the capsules' records have no codes, and the hand builder turns such a chip into chip 0x185 whatever
the folder holds (`chips/0x17f-prpcapsl/hit`, verified), so no netbattle reaches them; an empty list; a panel
that changed between the list and its turn.

**Ported** (content model v2): chips/geddon (`geddon/controller`, `geddon/quake`), with the hook's argument
the row's panel routine ("break" for Geddon and PnkCapsl, "crack" for PrpCapsl, HealBall, MagPanl and the two
dimming BeastOut records, each a definition in its own folder; "poison" is ported with no chip using it). A quake
that fails to spawn leaves the dimming waiting for good, as in the original. Lab: 19/19.

## 13. Subtype 12: Snake (T4#0x45, T3#0x72, T3#0x73)

Chip 0x86 Snake (null, damage 30, hit param 133, params 0). Snakes leap from the holes on the user's side
at the nearest enemy.

**Spawner `sub_80E59C6`**: T4#0x45; the common fields (alliance and flip).

**Effect `sub_80E5988`** (T4#0x45 `sub_80E5944` → `sub_80E5964`): first tick: `sub_80D2E94(PanelX, PanelY, r6 =
damage word + the bonus, r7 = &Param2)`, the nest; then Param2 = 1 (set by the controller itself, so a failed
spawn leaves it set and the dimming never ends: **unverified**); Timer = 0x28. Each tick: while Param2 is set,
wait; then Timer −= 1 (u16), ending the effect when negative (41 ticks after the nest is done).

**The nest's spawner `sub_80D2E94`**: T3#0x72 (position = registers), PanelX/Y, damage word, RelatedObject1 =
the controller, ExtraVars+8 = r7, the controller's alliance/flip halfword, flags |= 0x10 (no element: 0).

**The nest, T3#0x72 (`sub_80D2BDC`)**: states init `sub_80D2C00` (coordinates from the panel; state update,
word; the update runs at once), update `sub_80D2C10` (action 0 phase 0: `sub_80D2C40`), destroy
`sub_80D2E86` (`*ExtraVars+8 = 0`: the controller goes on; `object_freeMemory`); `object_updateSprite` after
each (it has no sprite). PhaseInitialized is its state:

- 0: Timer = 1; the scan's column Param3 = 6 − 5·(alliance ^ flip) (the far column), row Param4 = 1; state
  1 (and on).
- 1: Timer −= 1; not 0 → wait. (Param3, Param4) off the field → state 2. The next hole
  (`sub_80D2CBE`); none (0xFF) → state 2. Else a snake: `sub_80D30A4(hole x, hole y, target x, target y, r6
  = damage word, r7 = &ExtraVars+4)`, flags |= 0x10 on it (a failed spawn writes the BIOS). Then a peek at
  the next hole (Param3/Param4 saved and restored around it): in the same column → Timer = 8, else (another
  column or none) 0x18. The first snake comes on the nest's first tick.
- 2: once ExtraVars+4 (the live snakes) is 0 → state destroy (word).
- The next hole (`sub_80D2CBE`): from (Param3, Param4) on, rows 1..3 then the next column toward the user
  (step −front direction), the first panel meeting `byte_80D2D30[alliance]` (side 0 {0, 0x0F8800B0}, side 1
  {0x20, 0x0F880090}: the user's side's panel, not solid, nothing on it or reserved: a hole). Param4 advances
  past it (the scan resumes there next time). The scan gives up once Param3 + 1 (as a word) passes 7: side 0
  scans columns 6..1, 0, 0xFF; side 1 1..7 (off-field panels never match: `object_checkPanelParameters`
  fails on flags 0). Found → the target (`sub_80D2D40(hole)`), returning the hole in r0/r1 and the target in
  r2/r3.
- The target (`sub_80D2D40(hx, hy)`): the candidates, columns from hx + front forward while (column, row) is
  on the field, rows 1..3: panels with any of `byte_80D2E28[alliance]` (side 0 0x05800000: side 1's bodies and
  other bodies, or a neutral object; side 1 0x0A800000). None → 0xFF. Else in that order the best: the first;
  then one with a smaller |y − hy| replaces it, a larger doesn't; equal: a smaller |x − hx| replaces, a larger
  doesn't; equal: the candidate's x compared with the best's **y** (a bug: `[sp+0x3C]` for `[sp+0x38]`):
  greater doesn't replace, else it does.

**The snake's spawner `sub_80D30A4`**: T3#0x73 (position = registers), PanelX/Y = the hole, ExtraVars+0/+4 =
the target (x 0xFF: none), damage word, the nest's alliance/flip halfword, ExtraVars+8 = r7 (the nest's
counter).

**The snake, T3#0x73 (`sub_80D2EBC`)**: states init `sub_80D2EDC`, update `sub_80D2F3C`, destroy `sub_80D3048`
(the nest's counter −= 1 (a word read, a byte store), `object_freeMemory`).

- Init: its panel off the field → `object_freeMemory` (the counter untouched). The counter += 1 (word read,
  byte store); X, Y = the panel's center, Z = 0; flags |= 3; `sprite_load(0x80, 0xC, 0x2C)` (0C-2C), no
  shadow, CurAnim 0, animation 0 set and loaded; flip; aim (`sub_80D3058`): no target → Xvel = 16 px·front,
  Yvel = 0; else toward the target's panel center from its own: `calcAngle_800117C(dy, dx)` (BIOS ArcTan2 of
  the whole-pixel dx, dy, >> 8: 256 steps) and `sub_80011A0(angle, 16 px)` (Xvel = cos·speed >> 8, Yvel =
  −sin·speed >> 8, `math_cosTable` and `byte_80066E0`). State update (word); the update runs at once.
- Update: its panel off the field → state destroy (word). Else the action, then `object_updateSprite`.
- 0 (`sub_80D2F70`): first tick (CurPhase 0) Zvel = 2 px, sound 0x94, CurPhase 4. Z += Zvel; Z's pixels ≥ 10
  → action 4. (5 ticks.)
- 4 (`sub_80D2FA0`): Timer = 0x14; at 0 → action 8. (20 ticks.)
- 8 (`sub_80D2FC2`) by CurPhase: 0: CurAnim 1, phase 4. 4: once the frame flags have 0x80, sound 0xB3, phase
  8. 8: X += Xvel, Y += Yvel, panels from the coordinates; on the target panel: `sub_80C53A6(x, y, element 0,
  z 8 px, r4 = 0x04050501, r6 = damage word, r7 = 3)` (region 1, hit effect 5, target 5, self 4, hit
  modifier 3, resolving while dimmed) and state destroy (word). Else it flies on until it leaves the field
  (a snake with no target always does).

**Lab**: 20 scenarios; only `stage-holes` has a hole on the user's side: one snake, aimed at the one
candidate, striking it. Several holes (`chips/0x086-snake/after-geddon`, `after-geddon-miss`: Geddon first) and
side 1's scan (`side1`) are **verified**. **Unverified**: no target (a snake leaving the field), a flipped user.

**Ported**: chips/snake (`snake/controller`, `snake/nest`, `snake/snake`). Two details the lab showed or the
code says beyond the above: the nest's Z is what its spawner leaves in r3, the controller's bonus as a raw 16.16
value (Atk+10 leaves 10; `atk10` checks it); and side 1's scan covers columns 1 to 6 only (after column 6, 7 + 1
passes 7). Lab: 20/20.

## 14. Subtype 7: LifeSync (T4#0x5C, T4#0x60)

Chip 0xBF LifeSync (null, damage 0, params 0). (T1 and T4 objects spawn with header flag 0x10, so they run
while dimmed; T3 objects need it set.)

**Spawner `sub_80E7464`**: T4#0x5C; the common fields (alliance and flip).

**Effect `sub_80E72A4`** (T4#0x5C `sub_80E7260` → `sub_80E7280`), by CurPhase: 0 `sub_80E72C8`, 4 `sub_80E7394`,
8 `sub_80E73A2`, 0xC `sub_80E7450`.

**In a link battle (every PvP battle)**: `sub_80E72C8` starts with `GetBattleEffects() & 9` set (bit 3 is the
netbattle) and branches to `loc_80E73C4`, inside `sub_80E73A2`, past that routine's `sub sp, #0xc`. That code
walks the four target words at ExtraVars+4..+0x10 (all 0: nothing), then Timer −= 1; not negative → its
epilogue (`add sp, #0xc; pop {r4, r6, r7, pc}`) unwinds `sub_80E72C8`'s four saved words plus the three
state-dispatch frames' saved `lr`s, 28 bytes as pushed, and returns straight into `RunBattleObjectLogic`
with r4, r6, r7 garbage, which that loop reloads or restores (r7 is pushed around the call): harmless. The
phase stays 0, so this repeats every tick. Timer holds what `object_dimScreen` counted (the dim's ticks,
17 from a clear screen; it depends on the fade's start level). When Timer −= 1 goes negative: the HP sync
(below) with ExtraVars+0 = 0 (it reads the halfword at address 0x24, the BIOS, and uses it for nothing, the
list being empty), Timer = 0x1E, phase 0xC (with the same return). Phase 0xC (`sub_80E7450`): Timer −= 1
(u16); negative (31 ticks) → the effect ends. So in PvP LifeSync does nothing: its effect lasts the dim's
Timer + 1 ticks, then 31.

**Offline** (`GetBattleEffects() & 9` clear; not reachable in PvP, **unverified**):
- Phase 0: x = PanelX + 2·front, y = PanelY; `sub_80E7774` at that panel's center (r4 = 0x50): a T4#0x60
  marker kept in ExtraVars+0x14; sound 0xBD. `sub_80E7486(x, y, alliance)`: the panel's flags without
  `byte_80E74C4[alliance]` (side 0 0x00200000, side 1 0x00400000: the enemy player navi) → 0; else the first
  of BattleState+0x80's eight actor slots whose collision's panel (+0xA halfword) is (x, y), or 0. None, or a
  NameID 0x49..0x4E → the same branch to `loc_80E73C4` as above. Else ExtraVars+0 = the target; the marker is
  moved onto it: the target's position plus its NameID sprite's attach point 0x1B (`sub_800F26C`,
  `sub_8018842`: x toward −front, z up). ExtraVars+4..+0x10 = 0, then the side-1 actor slots (BattleState+0x90,
  four, whatever the user's side) that exist, aren't NameID 0x49..0x4E and aren't the target, in order. None →
  the branch to `loc_80E73C4`. Else Timer = 0x14, phase 4.
- Phase 4 (`sub_80E7394`): Timer −= 1; negative → phase 8.
- Phase 8 (`sub_80E73A2`): first tick two VRAM transfers (graphics), Timer = 0x3C. Each tick, for each listed
  actor: its screen position (`sub_800362C` from its collision's panel center and its Z) and a warning over
  it (`sub_800AE90`: sound 0x91 on game frames ≡ 0 mod 16; drawn only). Timer −= 1; negative → each listed
  actor's HP = the target's (sound 0x119 if it was lower, else 0x11A), Timer = 0x1E, phase 0xC.
- The marker, T4#0x60 (`sub_80E76F8`): init `sub_80E7718`: `sprite_load(0x80, 0x14, 5)` (14-05), no shadow,
  CurAnim = CurAnimCopy = 0xB, VISIBLE, palette 2; state update (word) and the update: Timer = Param1 (0x50)
  on its first tick, freed when Timer −= 1 goes negative (81 ticks). No position update of its own.

**Lab**: 19 scenarios, all the link-battle path (the effect ends 49 ticks after the telop from a clear
screen). **Unverified**: the offline path.

**Ported**: chips/lifesync (`lifesync/controller`, `lifesync/marker`). Bit 0 of the battle effects is
`BATTLE_EFFECT_BOSS_RANK` (`battle.boss_rank`), so LifeSync also does nothing against a ranked boss. NameIDs
0x49..0x4E are every version of the virus with AI 13, which the port tests by the actor data. The offline path's
warning sound, on game frames that are multiples of 16, follows the controller's timer instead (the port keeps no
game frame counter), and its HP sync skips a listed actor whose slot was freed. Lab: 19/19.

## 15. Subtype 22: NumbrBl (T4#0x69, T1#0x45, T3#0x91) and damage formula 21

Chip 0x8A NumbrBl (null, damage 1021: formula 21, hit param 138, params [4, 0, 0, 0]: the balls).

**Damage formula 21 (`sub_8010BF0(chip, side)`, `off_80109DC[21]`)**: `sub_80103BC(side)` (the side's first
actor if it is a player; its loop re-reads the same slot) → its HP mod 100 (`svc 6`'s remainder): the last
two digits of the user's HP; no player → 0. `sub_80109A4` evaluates it when the hand is built (chips.md
§2.4) and, the chip having flag 0x80 in its record, `chip_800AEE8` refreshes the current entry every frame. Engine:
`kinds::chip_damage_formula` (`hp_last_digits`).

**Spawner `sub_80E7FBA`**: T4#0x69; the common fields (alliance and flip).

**The controller, T4#0x69 (`sub_80E7F38`)**: a navi chip's phases, not the dimming chips': actions (`off_80E7F6C`)
0 `object_dimScreen`, 4 `sub_800BDB2` (AntiNavi: chip 0x8A is not a navi chip, so straight on), 8
`sub_800BA8A` (the navi telop: the effect is skipped only when the user was deleted), 0xC the effect
`sub_80E7F80`, 0x10 `object_undimScreen`. The effect: first tick `sub_80C31F0(PanelX, PanelY, Element, r3 =
alliance, r4 = Params, r5 = the user, r6 = damage word + the bonus, r7 = &Param2)`; each tick (the first
included) once Param2 is 0 → CurAction 0x10 (the undim).

**NumberMan's spawner `sub_80C31F0`**: T1#0x45 (position = registers), PanelX/Y, Element, RelatedObject1 = the
user, the user's alliance/flip halfword, the damage word, ExtraVars+0 = r7 and `*r7 = 1`.

**The thrower, T1#0x45 (`sub_80C3000`)**: states init `sub_80C3024`, update `sub_80C30C2`, `object_freeMemory`;
`object_updateSpriteTimestop` after each (its sprite steps while dimmed).

- Init: as BurnSqr's burner (§2), but it keeps the user's actor record (`sub_800F29C`: type in ExtraVars+0xC,
  index in ExtraVars+0x10) and its update first runs next tick.
- Update: VISIBLE (off for a blind viewer); by CurAction:
- 0 (`sub_80C3100`): first tick Timer = 0x14; Timer −= 1 (u16); negative (21 ticks): CurAnim 0xE; Timer2 =
  Param1 (the balls); a player user with record index 0 or ≥ 0x19 gets an attachment (`sub_80B8E30` with r4
  = 0x010006 | anim << 8, r7 = &ExtraVars+4: T1#5 `objects/attachment` kind 6, animation 0 or index − 0x18
  (the Cross and Beast forms), animating while dimmed; an index 1..0x18 gets none); action 4.
- 4 (`sub_80C314E`), per ball: first tick (PhaseInitialized 0 → 1): CurAnimCopy = 0xFF (the throw animation
  restarts), the attachment's too (+0x11) and RelatedObject2's if set; Timer = 0x12; `sub_80D73D6(PanelX +
  front, PanelY, Element, r3 = 16 px, r4 = 0xA00, r6 = damage word)`: a ball. Timer −= 1; not > 0 → Timer2
  −= 1; not 0 → PhaseInitialized 0 (the next ball); 0 → Timer = 0x3C, action 8. So four balls 18 ticks
  apart.
- 8 (`sub_80C31B0`): when Timer is 0x2D: ExtraVars+4 = 0 (the attachment ends) and CurAnim 0. Timer −= 1
  (u16); negative → the navi parts off (`sub_8011044(record, 1)`), VISIBLE off, state destroy (word), the user
  back (`sub_80E13DC`), `*ExtraVars+0 = 0`.

**The ball's spawner `sub_80D73D6`**: T3#0x91 (position = registers: panel Y, element, 16 px: its Z stays
16 px), PanelX/Y, Element, the thrower's alliance/flip halfword, the damage word, flags |= 0x10.

**The ball, T3#0x91 (`sub_80D7278`)**: states init `sub_80D729C`, update `sub_80D730A`, `object_genericDestroy`;
`object_updateSpriteTimestop` after each.

- Init: `sprite_decompress(0x10, 0x46)`; `sprite_load(0x80, 0x10, 0x46)` (10-46), no shadow; VISIBLE; CurAnimCopy
  0xFF, CurAnim 0, animation 0 set, loaded, stepped; coordinates from the panel (X and Y); Xvel = 3 px·front;
  collision (none: freed), self 4, target 5, hit modifier 3, region 1, present. State update (word); the
  update runs at once.
- Update: `object_removeCollisionData`, `object_spawnCollisionEffect`; a hit (FlagsFromCollision) → T4#0
  effect 0 at (X, Y, Z), sound 0x85, region 0, state destroy (word). Else the action and present. Its region
  is live from its first tick, even while it waits.
- 0 (`sub_80D7358`): first tick Timer = 0xE, sound 0x112; Timer −= 1 (u16); negative (15 ticks) → action 4.
- 4 (`sub_80D7382`): first tick CurAnim 1, shadow on, Timer = Param2 (10); negative (11 ticks) → action 8.
- 8 (`sub_80D73AC`): X += Xvel; panels and collision panels from the coordinates; X's pixels + 150 past 300
  (unsigned) → region 0, state destroy (word).

**Lab**: 22 scenarios. The balls hit in 20; `miss`'s roll off. Beast
and Cross scenarios use the index − 0x18 attachment. `counter-cut-in` reaches the navi telop's cut-in branch.
`link-navi` (HeatMan uses it) reaches a record index 1..0x18: no arm, and nothing to restart with each ball.
`hp-digits` (a base HP of 137) is the one whose formula gives more than 0: 37 a ball. **Unverified**: a
non-player user, the user deleted before the effect, no player for formula 21.

**Ported**: chips/numbrbl (`numbrbl/controller` with the navi chip phases, `numbrbl/numberman` on
lib/dimming/stand_in, `numbrbl/ball`); the record stays (formula 21) and chips/08a-numbrbl registers its hook by
number. The arm's record index comes from the user's NameID: MegaMan's (0x1A0) is 0 and his forms' (0x1AB + form)
0x18 + form, so the arm's animation is the form; other navis' (1 to 11) get none. Lab: 22/22.

## 16. Subtype 29: CornFsta (T4#0x68, T1#0x1E, T3#0xAA, T3#0x10)

Chip 0x14C CornFsta, the Program Advance (wood, damage 40, hit param 10, params [2, 1, 0, 0], unused).

**Spawner `sub_80E7F16`**: T4#0x68; the common fields but only the alliance byte.

**Effect `sub_80E7EE0`** (T4#0x68 `sub_80E7E9C` → `sub_80E7EBC`): first tick `sub_80BCCDC(PanelX, PanelY, Element,
r5 = the user, r6 = damage word + the bonus, r7 = &Param2)`; each tick (the first included) the effect ends
once Param2 is 0. (r4 is not set: the actor's Params are the object loop's r4, the controller's header
flags byte, which nothing reads. The same holds for BurnSqr's, BlzrdBal's and Magnum's actors.)

**The farmer's spawner `sub_80BCCDC`**: T1#0x1E, as BlzrdBal's thrower's (§10).

**The farmer, T1#0x1E (`sub_80BCB50`)**: init `sub_80BCB74` (as BurnSqr's burner's, §2: the update runs at
once), update `sub_80BCC0C` (by CurPhase; **no** visibility or blind-viewer step), `object_freeMemory`;
`object_updateSprite` after each.

- 0 (`sub_80BCC2C`): first tick CurAnim 0xA; an attachment (`sub_80B8E30`, r4 = 0x10018: kind 0x18,
  animation 0, animating while dimmed; kept in ExtraVars+4); Timer = 0. Timer += 1; at 10:
  `sub_80DAB9C(PanelX + front, PanelY, Element, r3 = 0, r4 = 0xFF, r6 = damage word, r7 = &Param4)`, the
  corn; the attachment's CurAnim += 1; sound 0x180; phase 4.
- 4 (`sub_80BCC88`): once Param4 is 0 (the corn is done) → phase 8.
- 8 (`sub_80BCC96`): first tick Timer = 0x1E; Timer −= 1 (u16); negative (31 ticks) → the navi parts off,
  the attachment's state word = 8 (`sub_80B8E58`), VISIBLE off, state destroy (word), the user back
  (`sub_80E13DC`), `*ExtraVars+0 = 0`.

**The corn's spawner `sub_80DAB9C`**: T3#0xAA (position = registers), `sub_801155A` (PanelX/Y, Element,
damage word, the farmer's alliance/flip halfword, RelatedObject1 = the farmer), ExtraVars+4 = r7 and (if
set) `*r7 = 1`, flags |= 0x10. Params 0xFF.

**The corn, T3#0xAA (`sub_80DAA28`)**: states init `sub_80DAA48`, update `sub_80DAA86`, destroy `sub_80DAB8A`
(`*ExtraVars+4 = 0` if set, `object_genericDestroy`). No sprite.

- Init: coordinates from the panel; Xvel = the front direction (a column step); collision (none: freed),
  self 0x3D (only 0x80: it hurts nothing), target 5, hit modifier 0, PrimaryElement 0, region 1, present;
  the recent-burst ring (ExtraVars+0xC, bytes +0x6C..+0x6E) and its index (ExtraVars+0x10) 0; ExtraVars+0
  (the steps) = 2. State update (word); the update first runs next tick.
- Update: `object_removeCollisionData`, the action, present. (The code after it is unreachable.)
- 0 (`sub_80DAABC`): touching a body (FlagsFromCollision) → burst here. Else PanelX += Xvel, coordinates
  and collision panels from it; off the field → region 0, state destroy (word). Else ExtraVars+0 −= 1; at 1
  or less (so after this first step) → burst here. The burst: `sub_80C67D8(x, y, Element, r3 = 0, r4 =
  0xFF, r6 = damage word)` with flags |= 0x10 (a failed spawn writes the BIOS), camera shake (1, 0x14),
  region 0, action 4. So the first burst is two panels ahead of the user, or one if a body stands there.
- 4 (`sub_80DAB22`): Timer += 1; below 8 → wait. Timer = 0; Timer2 += 1; at 16, or the battle over (`tst`),
  → state destroy (word). Else a panel near the first burst (`sub_80DABC4`); none → state destroy. Else a
  burst there (as above, flags |= 0x10), recorded in the ring (index 0, 1, 2, 0, …), camera shake (1, 0x14).
  So 16 bursts, 8 ticks apart.
- The next panel (`sub_80DABC4`), around the corn's panel (`byte_80DACC0`: (0,0), (0,−1), (1,−1), (1,0),
  (1,1), (0,1), (−1,1), (−1,0), (−1,−1)): **one `GetRNG2` draw** & 0xF; below 6: `sub_80DABE8` → the panels
  of the nine (dx turned by `object_getEnemyDirection`) with the enemy's body (side 0 {0x04000000, 0}, side 1
  {0x08000000, 0}), and if any, **a `GetPositiveSignedRNG2` draw** mod their count picks one. Otherwise (6 or
  more, or none): `sub_80DAC0C` → the nine (dx not turned) without the enemy's body (side 0 {0, 0x04000000},
  side 1 {0, 0x08000000}; off-field panels fail), preferring those not in the ring; none at all → 0; else a
  `GetPositiveSignedRNG2` draw mod the count.

**The burst's spawner `sub_80C67D8`**: T3#0x10 (position = registers), `sub_801155A` (PanelX/Y, Element,
damage word, the spawner's alliance/flip halfword, RelatedObject1 = the spawner), Params = r4.

**The burst, T3#0x10 (`sub_80C6580`)**: states init `sub_80C65A0`, update `sub_80C65EE`, `object_genericDestroy`.
(Also spawned by another player action, `sub_80ED0AC`, and by itself.)

- Init: coordinates from the panel; Xvel = the front direction; collision (none: freed), self 0x3D, target
  5, hit modifier 0, PrimaryElement 0, region 1, present; Param2 (the flare-ups) = 2; Timer2 = 0xC; state
  update (word). Param1 ≠ 0 → CurAction 4 (burning); Param1 ≠ 0 and ≠ 0xFF → the spread list
  (`sub_80C673A`). (The update first runs next tick.)
- Update: `object_removeCollisionData`, `object_spawnCollisionEffect`; a touch (FlagsFromCollision): burning
  → region 0; moving → the spread list, action 4, phase 0. Then the battle over (`tst`) → region 0, VISIBLE
  off, state destroy (word); else the action and present.
- 0, moving (`sub_80C6650`): PanelX += Xvel; coordinates and collision panels; off the field → region 0,
  VISIBLE off, state destroy (word).
- 4, burning, by CurPhase: 0 (`sub_80C6698`): the first time (ExtraVars+0xC), a solid panel turns to grass
  (`object_setPanelType(x, y, 6)`); `sub_80C6726`: the collision becomes a hit (`sub_801A082`: self 4,
  target 5, hit modifier 1, its damage; while dimmed self |= 0x10000) with its element (`sub_8019F8C`);
  region 1; T4#0 effect 0x23 at (X, Y, Z); sound 0x70; Timer = 0xC; phase 4. 4 (`sub_80C66E4`): a counter
  (ExtraVars+8) plays sound 0x70 each 8th update; Timer2 −= 1, at 0 the spread (`sub_80C6774`); Timer −= 1
  (u16), negative (13 ticks) → Param2 −= 1: left → phase 0 (another flare-up), none → region 0, state destroy
  (word). A touch while burning turns the region off until the next flare-up.
- The spread list (`sub_80C673A`): hit region 4 ((0,0), (0,−1), (0,1)) from the panel in front, turned by
  alliance ^ flip, the panels with the enemy's body (`object_getPanelRegion` into ExtraVars+4.., the count in
  ExtraVars+0).
- The spread (`sub_80C6774`): from the list's end, each panel without a T3#0x10 already on it (`sub_80C67A4`
  walks the whole object list) gets a burst with Params Param1 + 1 (so a CornFsta burst's 0xFF would give
  0x100: Param1 0, Param2 1).

**Lab**: 8 scenarios. The three recipes: the corn steps once and bursts, then fifteen bursts by both panel
choices (the ring keeping panels out of the second), grass on each solid panel. CornFsta's bursts (Param1
0xFF) make no spread list. `adjacent`: the corn touches the body in front and bursts there without a step.
`off-field`: from column 5, after two AreaGrabs, it steps off the field and nothing bursts. `far`: no body
is near, so the first choice finds none and every burst is a free panel's. `deleted`: the first burst deletes
an opponent of 40 HP, and the corn and its bursts end with the battle. `stage-holes`: bursts on missing
panels leave them. **Unverified**: no panel for a burst, the free panels all in the ring, a failed spawn.
(The moving burst and the spread, Param1 0 or 1..0xFE, are CornSht's: shot-chips.md §9.1.)

**Ported**: chips/cornfsta (`cornfsta/controller`, `cornfsta/farmer` on lib/dimming/stand_in with the
user's NameID, `cornfsta/sower` for T3#0xAA); the bursts are CornSht's corns (chips/cornsht/corn, generation
0xFF) and the farmer holds CornSht's gun. The record stays (a Program Advance) and chips/14c-cornfsta registers its
hook by number. Lab: 8/8.

## 17. Subtype 30: DblHero (T4#0x6A, T1#0x1F)

Chip 0x158 DblHero, the Program Advance (null, damage 60, hit param 138, params 0).

**Spawner `sub_80E8058`**: T4#0x6A; the common fields but only the alliance byte.

**Effect `sub_80E8020`** (T4#0x6A `sub_80E7FDC` → `sub_80E7FFC`): first tick `sub_80BD20A(PanelX, PanelY, Element,
r4 = 0, r5 = the user, r6 = damage word + the bonus, r7 = &Param2)`; each tick (the first included) the effect
ends once Param2 is 0.

**The heroes' spawner `sub_80BD20A`**: T1#0x1F (position = registers), PanelX/Y, Element, damage word,
RelatedObject1 = r5, ExtraVars+0 = r7 (and `*r7 = 1` if set), r5's alliance/flip halfword and NameID, flags
|= 0x10; Params = r4 (Param1 0 MegaMan's copy, 1 ProtoMan).

**The heroes, T1#0x1F (`sub_80BCD14`)**: states init `sub_80BCD38`, update `sub_80BCE4C`, `object_freeMemory`;
`object_updateSprite` after each.

- Init: AIDataPtr = the user's (shared). Param1 1: NameID 0x1AB and sprite (8, 0xB) (08-0B); else the user's
  NameID and chip word (+0x28) and the user's sprite (navi and form for a player, else `sub_800F26C`).
  `sprite_load(0x80, …)`, load, shadow, CurAnim 0, CurAnimCopy 0xFF, VISIBLE. Position = the user's, plus
  `dword_80BCE44` bytes by Param1 (MegaMan (−10, −10), ProtoMan (10, 10)) in pixels, x times the front
  direction, on X and Y; panels from the coordinates; Z = 0. Param1 0: the user's navi palette. Flip;
  `sub_8010DF6(its NameID's actor record, 1)`; `sub_80E1352(user, 0)` (the user vanishes with its barrier,
  confusion and blindness visuals, the HUD, the charge glow and the aura). Param1 0 also spawns ProtoMan
  (`sub_80BD20A(PanelX, PanelY, Element, r4 = 1, r5 = the user, r6 = its damage word, r7 = 0)`) and the two
  keep each other in ExtraVars+0x10 (a failed spawn writes the BIOS and leaves 0, which the waits below then
  read through: **unverified**). State update (word); the update runs at once.
- Update: Param1 0 runs `off_80BCE74`, Param1 1 `off_80BCE80`, by CurAction.

MegaMan's copy (Param1 0):
- 0 (`sub_80BCE90`): ProtoMan's CurAction 4 → action 4 (and it runs at once); 0xC → action 8 at phase 8 (the
  end, no volleys); else wait.
- 4, the charge (`sub_80BCEB2`, ProtoMan's too): first tick CurAnim 0; T4#0 effect 0x4E at its attach point
  0 (`sub_8018810`: `sub_800F26C` and `sub_8018842`, x turned by the flip direction; NameIDs 0xCD..0xFF give
  (0, 7)) added to X and Z, its Timer 0x28; sound 0x71; Timer = 0x3C. At Timer 0x14: effect 0x4F there, Timer
  0x14, sound 0x72. Timer −= 1; not > 0 → ExtraVars+4 and +0xC (its two attachment slots) = 0, CurAction +=
  4, phase 0. (60 ticks.)
- 8, phase 0 (`sub_80BCF58`): one tick; Timer2 = 10, phase 4.
- 8, phase 4 (`sub_80BCF7A`), a volley: first tick CurAnim 0xE, CurAnimCopy 0xFF; Timer = 0xF; sound 0x6A;
  the attachment in ExtraVars+4 ends (`sub_80B8E58`: state word 8) and `sub_80EB572(&ExtraVars+4, 1)`
  attaches the buster arm for the navi's form (`lib/buster.luau`), animation 1; the one in ExtraVars+0xC
  ends and kind 5 (r4 = 0x10005, animating while dimmed) takes its place; the form overlay (RelatedObject2)
  restarts (`sub_80C44D2`: CurAnimCopy 0xFF and a step); the shots (`sub_80BD24C`); a flash
  (`sub_80BD2D8`). Flashes again at Timer 0xA and 5. Timer −= 1; not > 0 → Timer2 −= 1: left → the next
  volley (PhaseInitialized 0), else phase 8. So ten volleys, 15 ticks apart.
- 8, phase 8 (`sub_80BCFF6`): first waits for ProtoMan's CurState 8 (+8); then CurAnim 0, both attachments
  end, position = the user's, panels from it, Timer = 0x1E. Timer −= 1 (u16); negative → the user's navi
  parts off, VISIBLE off, state destroy (word), the user back (`sub_80E13DC`), `*ExtraVars+0 = 0` (if set).
- The shots (`sub_80BD24C`), rows 1..3: from PanelX + front along the row, past the user's side's panels,
  the first other panel x (`sub_80BD278`; off the field ends the walk); its distance to the far edge (7 −
  x facing right, x − 0 facing left) picks the region `byte_80BD2CC` = {0, 1, 2, 6, 7, 0x20, 8, 0}[distance]
  (one to six panels forward); `sub_80C53A6(x, row, Element, z 0, r4 = 0x0405FF00 + region, r6 = damage word,
  r7 = 3)`: hit effect 0xFF, target 5, self 4, hit modifier 3, resolving while dimmed. Every enemy panel is hit
  each volley: 30 hits in all.
- The flash (`sub_80BD2D8`): the panels of another alliance from PanelX + front forward (rows 1..3, until off
  the field); **one `GetPositiveSignedRNG2` draw** mod their count (none: a division by zero in `svc 6`,
  **unverified**) picks one: T4#0 effect 0x21 (MegaMan) or 0x16 (ProtoMan) at its center, Z 0, flipped
  with the object (r4 | flip << 8).

ProtoMan (Param1 1):
- 0 (`sub_80BD064`), then each tick the color shader from Unk_0c (`sub_80BD374`: c | c << 5 | c << 10):
  phase 0 (`sub_80BD084`): first tick shader 0x7FFF, Unk_0c = 0x1F, sound 0x94, Timer 0 (and return).
  After: VISIBLE, off on odd Timer (+1 each tick); Unk_0c −= 2; at 0 or less → VISIBLE, Unk_0c 0, phase 4 (16
  ticks fading from white). Phase 4 (`sub_80BD0DA`): first tick Timer = 0x1E (and return); Timer −= 1; not
  > 0 → its panel solid → action 4, else action 0xC.
- 4: the charge, as above.
- 8 (`sub_80BD114`): phase 0 one tick, Timer2 = 10; phase 4 (`sub_80BD156`), a slash: first tick CurAnim 5,
  CurAnimCopy 0xFF, the attachment in ExtraVars+4 ends and kind 3 animation 0xB (r4 = 0x10B03, animating while
  dimmed) takes its place, sound 0xB0, Timer 0xF, a flash; flashes at 0xA and 5; ten slashes like the
  volleys (no hits: only MegaMan's shots hit); phase 8 (`sub_80BD1B8`): Timer 0x1E, negative → action 0xC.
- 0xC (`sub_80BD1DA`): first tick CurAnim 4, the attachment ends, Timer 4; negative (5 ticks) → VISIBLE off,
  state destroy (word). (Its navi parts, added for NameID 0x1AB's record, aren't taken off.)

**Lab**: 6 scenarios. `pa/0x158-dblhero/recipe1` reaches all of the above but the failed spawns, ProtoMan on
a non-solid panel and a field with no enemy panel; its shots start three panels from the far edge (region
6). `on-hole` (AirShoes, over a missing panel): ProtoMan leaves at once (action 0xC first) and MegaMan ends
without a volley. `own-grab-twice`, `own-grab`, `grabbed` and `grabbed-twice` (AreaGrabs by the user or
the opponent first) start the shots one, two, four and five panels from the far edge (regions 1, 2, 7 and
0x20). **Unverified**: the failed spawns, a field with no enemy panel, a row with no panel of another side
ahead (distance 0: no region). (Six and seven panels can't be: the walk starts in front of the user.)

**Ported**: chips/dblhero (`dblhero/controller`, `dblhero/heroes`); the record stays (a Program
Advance) and chips/158-dblhero registers its hook by number. `sub_80EB572`'s second argument (1) is the
attachment's third byte, animating while dimmed, not its animation: the arm's animation is the side's form, as
the buster's (lib/buster `raise_arm_for`, by the user's actor data, which MegaMan's copy shares; a player's second
actor would clear the user's overlay slot). A field with no panel of the other side's makes the flash divide by
zero: an error. Lab: 6/6.

## 18. Subtype 32: MetrKnuk (T4#0x76, T3#0xB4)

Chip 0x133 MetrKnuk (null, damage 100, hit param 148, params 0).

**Spawner `sub_80E8BC0`**: T4#0x76; the common fields (alliance and flip).

**Effect `sub_80E8B44`** (T4#0x76 `sub_80E8B00` → `sub_80E8B20`), by CurPhase:

- 0 (`sub_80E8B64`): Timer2 (the punches) = 16; phase 8.
- 8 (`sub_80E8BB0`): Timer −= 1 (u16); negative → phase 4. The first time Timer still holds
  `object_dimScreen`'s count (17 from a clear screen: 18 ticks); later 6 (7 ticks).
- 4 (`sub_80E8B70`): the first punch (Timer2 16) → the target from `sub_80E8C44`; later ones from
  `sub_80E8BF8`. A target (x ≠ 0) → `sub_80E8BE2(x, y, Timer2 & 1)`: `sub_80DBEA2(x, y, Element, r3 = 120 px,
  r4 = Timer2 & 1, r6 = damage word + the bonus)`, a fist. Timer2 −= 1; not > 0 → the effect ends; else Timer
  = 6, phase 8. So 16 punches 8 ticks apart.
- The first target (`sub_80E8C44`): the panels (rows 3..1, columns 6..1, `object_getPanelsFiltered`)
  meeting `byte_80E8D6C[alliance]` (side 0 {0x04010020, 0}: side 1's panels with side 1's body; side 1
  {0x08010000, 0x20}); none → those meeting `byte_80E8D58[alliance]` (side 0 {0x00010020, 0}: side 1's
  panels; side 1 {0x00010000, 0x20}); **a `GetPositiveSignedRNG2` draw** mod the count picks one (none: r0
  0, no punch). ExtraVars+0 = y << 4 | x.
- A later target (`sub_80E8BF8`): a grid of bytes at +0x64 + 6·y + x is zeroed (0x28 bytes from +0x64: past
  the T4's 0x1C bytes of variables, over the start of its unused sprite data); every enemy-body panel
  (`byte_80E8D6C`) marks its 3×3 (`byte_80E8D80`: (−1,−1), (0,−1), (1,−1), (−1,0), (0,0), (1,0), (−1,1),
  (0,1), (1,1), inside 1..6 × 1..3) (`sub_80E8C84`); with no body every enemy-side panel (`byte_80E8D58`) is
  marked instead (`sub_80E8CFA`). The last target (ExtraVars+0, if any) is unmarked. Then the candidates
  (`sub_80E8D92`, rows 4..1, columns 7..1) meeting `byte_80E8C34[alliance]` (= `byte_80E8D58`'s) and marked;
  a `GetPositiveSignedRNG2` draw mod their count picks one (none: 0, no punch). ExtraVars+0 = it.

**The fist's spawner `sub_80DBEA2`** → `sub_80DBE82`: T3#0xB4 (position = registers: X, Y garbage, Z = 120
px, which it keeps), PanelX/Y, Element, damage word, the spawner's alliance/flip halfword, RelatedObject1 =
the spawner; then flags |= 0x10 and Param2 = 4 (another caller, without them, gets Param2 0).

**The fist, T3#0xB4 (`sub_80DBCEC`)**: states init `sub_80DBD10`, update `sub_80DBDA0`, `object_genericDestroy`;
`object_updateSprite` after each.

- Init: coordinates from the panel, X −= 134 px·front, Y += 16 px; Zvel = `byte_80DBD8C` word at Param2 (0:
  6 px, 4: 14 px); Xvel = front · `byte_80DBD98` word at Param2 (0: 10 px, 4: 17.5 px); `sprite_load(0x80,
  0xC, 0x62)` (0C-62), no shadow, VISIBLE, CurAnim = CurAnimCopy = 0, set and loaded, palette 0, flip; sound
  0xC4. State update (word); the update first runs next tick.
- 0 (`sub_80DBDBC`): X += Xvel; Zvel += `byte_80DBE44` word at Param2 (0: 0x7800, 4: 0xD200); Z − Zvel ≥ 0 →
  Z = that. Else it lands: action 4; T4#0 effect 5 at its panel's center jittered
  (`AddRandomVarianceToTwoCoords(3)`: **one `GetRNG2` draw**, x and z by −1..2 px), Z 0; the hit
  (`sub_80DBEBA`: `object_spawnCollisionRegion(x, y, Element, z = its Z before this tick, r4 = 0x15050001,
  r6 = damage word, r7 = 3)`: region 1, hit effect 5, target 5, self 0x15, hit modifier 3, with the fist's flag
  0x10); the panel: Param2 ≠ 0 → `object_crackPanel` (cracks it, or breaks a cracked empty one); Param2 0 →
  not already cracked and a `GetPositiveSignedRNG2` draw with bit 0 clear → crack; a solid panel (before
  the crack) → camera shake (1, 5). Z = 0.
- 4 (`sub_80DBE4C`): first tick Timer = 7 (and return); then while Timer ≤ 4 it bounces back (X −= Xvel, Z
  += Zvel); Timer −= 1; 0 → state destroy (word).

**Lab**: 16 scenarios: sixteen punches at bodies (the first list) and around them (the grid), cracking.
**Unverified**: no enemy body (the fallback lists), no candidate at all, Param2 0 (the other caller).

**Ported**: chips/metrknuk (`metrknuk/controller`, `metrknuk/fist`). Corrections to the above: the
fist's hit word 0x15050001 is region 1, hit effect **0** (the null element's spark), target 5, self 0x15 (collision
`thrown-break`); and `sub_80DBE82` has no caller but `sub_80DBEA2`, so Param2 0 (the slow fall, the even-draw
crack) is ported but unreachable. With no enemy panel at all the first target keeps a stray register as its row;
the port keeps none. Lab: 16/16.

## 19. Subtype 37: DblBeast (T4#0x7F, T1#0x33, T1#0x34)

Chip 0x137 DblBeast (null, damage 0, hit param 138, params 0).

**Spawner `sub_80E943E`**: T4#0x7F; the common fields (alliance and flip).

**Effect `sub_80E9354`** (T4#0x7F `sub_80E9310` → `sub_80E9330`), by CurPhase:

- 0 (`sub_80E9378`): first tick `sub_80E1332(user, 1)` (the warp out, T1#0x2D: chips.md §3.6.7), Timer = 0x1E
  (and return); then Timer −= 1, not > 0 → phase 4 (31 ticks).
- 4 (`sub_80E93A0`): first tick, with r5 = the user, r2 = Element, r4 = 0, r6 = the bonus | hit param << 16
  (the damage word's low half replaced by the bonus: the beasts' hits add their own damage to it):
  `sub_80C1EAC` with r7 = &Param1 (Gregar's beast) and `sub_80C24D2` with r7 = &Param2 (Falzar's). Each tick
  (the first included), once Param1 and Param2 are both 0 → phase 8.
- 8 (`sub_80E93F2`): first tick Timer = 0x1E; then Timer −= 1, not > 0 → phase 0xC.
- 0xC (`sub_80E9412`): first tick `sub_80E1332(user, 0)` (the warp in), Timer = 0x1E; then not > 0 → the
  effect ends.

**The beasts' spawners `sub_80C1EAC` / `sub_80C24D2`**: T1#0x33 / T1#0x34 (position = registers); Element,
ExtraVars+0 = r6 (the damage base), RelatedObject1 = the user, the user's alliance/flip halfword, and the
flag pointer r7 **in the CollisionDataPtr slot** (as ElmntMan's), `*r7 = 1`. PanelX/Y are left 0 (the init
places them). Params 0.

Both beasts: `object_updateSpriteTimestop` after each state (they animate while dimmed); updates by
CurAction; "the n-tick wait" is: first tick Timer = 0x1E (return), then Timer −= 1 until not > 0 (31 ticks).
The target list (`sub_80C1ECE` / `sub_80C24F4`): `object_getPanelsExceptCurrentFiltered` with {side 0
0x04000000, side 1 0x08000000} (the enemy's body) into ExtraVars+0xC.., the count in Param2, Param1 (the
next target) = 0. The attack panel (`sub_80C1EF4(pattern)` / `sub_80C2518(pattern)`): for each target from
Param1 to the count (no wrapping), the pattern's entries {flip, dx, dy} (dx times
`object_getEnemyDirection`, 1 − 2·alliance; at most 6 entries: a longer pattern's rest is never tried;
0xFF ends it) from the target: the first panel meeting {0x10000, not 0x0F880080} (on the field, free) wins:
Param1 = the next target (wrapping to 0 at the count), return (x, y, flip). A candidate equal to the user's
panel is remembered; if nothing was free and it was one, the user's panel with flip 0 (Param1 wraps to 0).
Else 0. An attack: DirectionFlip = the flip, PanelX/Y = the panel, coordinates, sprite flip, FuturePanel =
Panel, `object_reservePanel`. The previous attack's reservation is dropped first. No panel → Param3 = 1
(the attack's timer runs out and it moves on; Gregar's and Falzar's later attacks also go back to their rest
spot, `sub_80C205C` / `sub_80C2678`).

**Gregar's beast, T1#0x33 (`sub_80C1A10`)**: init `sub_80C1A34`: rest spot (the user's panel, X += 10 px ·
front, Z 0), `sprite_load(0x80, 0, 0xB)` (00-0B), no shadow, CurAnim = CurAnimCopy = 0, palette 0, flip,
NameID 0x1B6, Param3 0; state update (word); the update first runs next tick. Actions (`off_80C1A90`):

- 0 (`sub_80C1AB8`): CurAnim 3, VISIBLE, sound 0x94, Timer 3; then not > 0 → 4.
- 4 (`sub_80C1AEE`): CurAnim 0, the target list, Timer 0x1E; then: targets → 8, none → 0x24.
- 8, the claw (`sub_80C1B2E`, pattern 0 `byte_80C1F94`: {0, −1, 0}, {1, 1, 0}, {0, −1, −1}, {1, 1, −1}, {0, −1,
  1}, {1, 1, 1}): Timer 0x1E; at a panel: CurAnim 5, sound 0xB0, attachment kind 3 animation 0xC (r4 = 0x10C03)
  in ExtraVars+4. When Timer −= 1 reaches 0x12: T4#0 effect 0x16 at the front panel's center, Z 16 px,
  flipped (`sub_80C2110`) and hit 0 (below). At 0: CurAnim 0, the attachment ends, → 0xC.
- 0xC: the wait → 0x10.
- 0x10, the bite (`sub_80C1BEC`, pattern 1 `byte_80C1FA8`: {0, −1, 0}, {1, 1, 0}, {0, −2, 0}, {1, 2, 0}):
  Timer 0x1E; at a panel: CurAnim 0xA, attachment kind 0x20 animation 1 (0x10120), hit 1, sound 0xF0, Timer2
  = 10; then hit 1 again whenever Timer2 −= 1 reaches 0 (Timer 20 and 10: three hits); at Timer 0: CurAnim 0,
  the attachment ends, → 0x14.
- 0x14: the wait → 0x18.
- 0x18, the gun (`sub_80C1CC4`, pattern 2 `byte_80C1FB8`: {0, −5, 0}, {1, 5, 0}, {0, −4, 0}, {1, 4, 0}, {0, −3,
  0}, {1, 3, 0}, …: only these six): Timer 0x1E; at a panel: CurAnim 0xE, a shot (`sub_80C20B4`), attachments
  kind 6 animation 0xB (0x10B06) in ExtraVars+4 and kind 5 (0x10005) in +8, sound 0xB9, Timer2 = 5; every 5
  ticks another: CurAnim 0xE, CurAnimCopy 0xFF, a shot, sound 0xB9, both attachments restart (CurAnimCopy
  0xFF, `sub_80B8E7C`); six shots; at Timer 0: CurAnim 0, both end, → 0x1C.
- 0x1C: the wait → 0x20.
- 0x20, the slam (`sub_80C1DC4`, pattern 3 `byte_80C1FD8`: {0, −1, 0}, {1, 1, 0}): Timer 0x1E; at a panel:
  CurAnim 0xC, hit 3, effect 0x3A at the front panel (`sub_80C2082(0)`: Z 16 px, flipped), sound 0xC7, camera
  shake (2, Timer + 0x1E); at Timer 0xF: CurAnim 0xC, CurAnimCopy 0xFF, hit 4, sound 0x158, effect 0x39; at 0 →
  0x24.
- 0x24 (`sub_80C1E7A`): drop the reservation, CurAnim 4, Timer 3; then not > 0 → `*flag = 0` (through the
  CollisionDataPtr slot), state destroy (word).
- Hit i (`sub_80C2000`): `sub_80C53A6(PanelX + front, PanelY, Element, z 0, r4 = byte_80C202C[i], r6 =
  byte_80C2048[i] + ExtraVars+0, r7 = 3)`: i 0: region 4, hit effect 6, target 5, self 4, damage 50; i 1:
  region 2, effect 0xA, self 6, 20; i 3: region 2, effect 6, self 4, 50; i 4: region 4, effect 6, self 4, 50
  (i 2, {0, 0, 0, 0} and 5, is unused); hit modifier 3; resolving while dimmed.
- The shot (`sub_80C20B4`): from PanelX + front forward (within 1..6), the first panel whose flags have
  `byte_80C2108[alliance]` (side 0 0x05800000, side 1 0x0A800000: the enemy's bodies or a neutral object):
  `sub_80C53A6(x, y, Element, z 16 px, r4 = 0x04050501, r6 = 5 + ExtraVars+0, r7 = 3)`; none → nothing.

**Falzar's beast, T1#0x34 (`sub_80C2138`)**: init `sub_80C215C`: rest spot (the user's panel, X −= 10 px ·
front, Z 0), `sprite_load(0x80, 0, 0xC)` (00-0C), no shadow, CurAnim = CurAnimCopy = 0, palette 0, flip,
`sub_8010DF6(2, 0x24, 1)` (actor record (2, 0x24)'s parts), NameID 0x1B7, Param3 0; state update (word).
Actions (`off_80C21C0`):

- 0: as Gregar's. 4 (`sub_80C2216`): as Gregar's but Timer 0x3C; none → 0x1C.
- 8, the breath (`sub_80C2256`, pattern `byte_80C25B8`: {0, −2, 0}, {1, 2, 0}): Timer 0x1E; at a panel: CurAnim
  0xA, attachment kind 0x21 animation 1 (0x10121), T4#0 effect 0x59 two panels ahead (Z 0) with Timer 0x1E
  (`sub_80C26C8`), hit 0, sound 0xB8, Timer2 = 4; hit 0 again every 4 ticks (eight hits); at 0: CurAnim 0,
  the attachment ends, → 0xC. (No panel: Param3, without going back to rest.)
- 0xC: the wait → 0x10.
- 0x10, the wing (`sub_80C2324`, pattern `byte_80C25C0`: {0, −1, 0}, {1, 1, 0}, {0, −2, 0}, {1, 2, 0}): Timer
  0x1E; at a panel: sound 0xB0, CurAnim 5, attachment kind 3 animation 0xD (0x10D03); at Timer 0x12: effect
  0x17 at the front panel (Z 16 px, flipped, `sub_80C26A0`) and hit 1; at 0: CurAnim 0, the attachment ends, →
  0x14.
- 0x14: the wait → 0x18.
- 0x18 (`sub_80C23F0`, pattern `byte_80C25D0` = Gregar's pattern 0): as 0x10 but effect 0x16 and hit 2; → 0x1C.
- 0x1C (`sub_80C2498`): drop the reservation, CurAnim 4, Timer 3; then its parts off (`sub_8011044(2, 0x24)`),
  `*flag = 0`, state destroy (word).
- Hit i (`sub_80C2600`): x = PanelX + front · `dword_80C2664` bytes {2, 1, 1}[i]; `sub_80C53A6(x, PanelY,
  Element | 0, z 0, r4 = byte_80C2640[i], r6 = byte_80C2654[i] + ExtraVars+0, r7 = 3)`: i 0 region 1 damage 10;
  i 1 region 2, 50; i 2 region 4, 50; all hit effect 6, target 5, self 4, hit modifier 3.

**Lab**: 19 scenarios (18 reach the effect); every attack runs. `obstacle`: Falzar's breath finds no panel;
`adjacent` and the Cross scenarios meet the user's panel among the candidates, but a free panel always
turns up. **Unverified**: the user's-panel fallback, no target at all (straight to the end), a failed
spawn, the unused pattern entries.

**Ported**: chips/dblbeast (`dblbeast/controller`, `dblbeast/gregar`, `dblbeast/falzar`, sharing
chips/dblbeast/beast). Of the attacks that find no panel, only the first of each beast (Gregar's claw, Falzar's
breath) stays where it is; the later ones go back to rest. Falzar's head is its identity's parts (NameID 0x1B7's
record, player 0x24). Lab: 19/19.
