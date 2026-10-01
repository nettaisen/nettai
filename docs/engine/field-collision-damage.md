# Field, collision and damage — engine spec (BN6 US Falzar, BR6E, link PvP)

This document specifies the **panel field**, the **collision pipeline** and the **damage/status pipeline** of the BN6 battle engine precisely enough to reimplement them in Rust without reading the assembly. The target is bit-exact behaviour: same observable state every tick and the same RNG call sequence.

Companion documents in this directory:
- `battle-flow.md`: tick anatomy, battle flags, RNG roles, BattleState.
- `objects-and-player.md`: object pools, spawning, list order, player actor.

This document repeats only what it needs from them.

## Conventions and sources

| Item | Value |
|---|---|
| Routines | named as in the original's symbols, with their ROM/IWRAM address |
| Exact literals | read from the ROM bytes: some literal pools disassemble as `.byte` lists with wrong `// =0x0` comments |
| `obj` | a battle object (`r5`) |
| `cd` | its CollisionData (`obj+0x54`) |
| `ai` | its AIData (`obj+0x58`) |
| `F1` / `F2` | `cd+0x3C` ObjectFlags1 / `cd+0x40` ObjectFlags2 |
| `FFC` | `cd+0x70` FlagsFromCollision |
| `ns[a]` | NaviStats of alliance a, at `0x0203CE00 + 0x64*a` |

Additional conventions:
- All u16 accumulators wrap silently.
- "Tick" = one call of `battle_8007A44`. Trace frame numbers are ticks of the battle function.

How claims were verified:
- By reading the original's code, and, where marked **[verified]**, by running the real game under emulation on the machgun battle, which reproduces the machgun trace byte-for-byte.
- Memory peeks and write watches show memory and the call chain of every write.
- Some paths that never occur in machgun were verified on the soundmod battle (a soundmod netbattle, run as vanilla). Those are marked **[verified-soundmod]**.
- Anything marked **(code only)** was read but never observed.

---

## 0. Global state owned by these subsystems

| Address | Name | Size | Meaning |
|---|---|---|---|
| 0x02039AE0 | `ePanelData` | 40 × 0x20 | 8 columns × 5 rows, index `y*8+x`. Valid area is x 1..6, y 1..3; the border ring is initialised but never ticked. |
| 0x02034F60 | `unk_2034F60` | 40 × u32 | Per-panel collision registration mask, index `y*8+x`. Bit `0x80000000>>k` means collision slot k is registered there. |
| 0x020384F0 | `eCollisionData` | 32 × 0xA8 | Collision slots (also `Toolkit+0x30`). |
| 0x02035310 | `eActiveCollisionDataBitfield` | u32 | Allocated slots (bit `0x80000000>>k`). |
| 0x02034010 | `unk_2034010` | 8 × 8 | Per-column info `colinfo[x]` (x 0..7), area-steal bookkeeping (§2.6.3). |
| 0x0203F6B0 | `byte_203F6B0` | 4n+1 | "Home runs" table for area return (§2.5). |
| 0x0203CB04 | `byte_203CB04` | **u32** | Volcano eruption cycle counter, period 0x8C. |
| 0x0203CFB0 / 0x0203CFBC | damage-carry records | 2 × 0xC | Per-alliance records (§4.4). |
| BattleState+0x0E / +0x16 | | u8 / u8 | Tick counters mod 20 / mod 180 (grass heal). |
| BattleState+0x32 | battle flags | u16 | Bit 0x1: navis resolve collisions. Bit 0x4: dimming. |
| GameState.BattlePaused | | u8 | Pause. |

RNG (both streams use `s = ((s<<1 | s>>31) + 1) ^ 0x873CA9E5`):

| Stream | Address | Role | Accessors |
|---|---|---|---|
| RNG2 | 0x020013F0 | synchronized gameplay RNG | `GetRNG2` 0x0800151C, `GetPositiveSignedRNG2` 0x08001532 (result & 0x7FFFFFFF) |
| RNG1 | 0x02001120 | local/visual | `GetRNG1` 0x0800154C, `GetPositiveSignedRNG1` 0x08001562 |

---

## 1. Tick skeleton (only what matters here)

`battle_8007A44` (0x08007A44), see `battle-flow.md` §4. In the fighting phase:

1. Battle-mode handler.
2. **`RunBattleObjectLogic`** (0x080031AC). Walks the object list from `eBattleObjectsLinkedListStart.Next` (0x02009380).
   - Objects are skipped if paused and header flag 0x04 is clear, or if dimmed and header flag 0x10 is clear.
   - Objects spawned by `object_spawnType1/3/4` (via `sub_8003400`) are inserted **directly after the object currently updating**. They therefore run later in the same tick, and several spawns from one update run in reverse spawn order.
3. `sub_802FFF4` (camera, RNG1).
4. **`sub_800BFC4`** (panel tick, §2.6).
5. `sub_800FDC0`, `sub_801BEE0`, `sub_802CEC8`, `chip_800AEE8`.
6. If not paused and not dimmed: `BS+0x0E=(BS+0x0E+1)%20`, `BS+0x16=(BS+0x16+1)%180`.
7. If not paused: `sub_802CDFE` (shifts the damage-carry records, §4.4).
8. `sub_80102AC`.
9. `BS+0x64 += 1`.
10. Rendering, including `sub_800C5E0` (panel graphics; clears panel +0x01 and +0x0D).

**Canonical per-object collision protocol.** Every collidable object runs this inside its own update; §3.6 gives the details.

```
object_removeCollisionData   ; unregister my region and resolve against whatever is registered there
<consume my accumulators>    ; damage, flags, statuses (navi: §4)
<act>                        ; move / change region / change damage
object_presentCollisionData  ; zero my accumulators, register my region
```

In the machgun trace the steady-state list order is:
1. alliance-1 navi (T1/0 @0x0203A9B0)
2. T4/8
3. alliance-0 navi (T1/0 @0x0203AA88)
4. T4/0x48
5. T1/5
6. T4/8

Transient hitboxes (T3/3) are inserted after their spawner **[verified]**.

---

## 2. The field

### 2.1 Geometry

- `_object_getPanelDataOffset(x,y)` (0x03007958) returns `0x02039AE0 + (y*8+x)*0x20` when `1<=x<=6 && 1<=y<=3` (unsigned compares), else 0. `object_getPanelDataOffset` 0x0800C90A wraps it.
- Coordinates are absolute. x=1 is the left edge; alliance 0 normally owns the left columns.
- `object_getAllianceDirection(a) = 1 - 2a` (+1 means "toward larger x").
- `object_getFlipDirection(a, flip) = 1 - 2*(a ^ flip)` (0x0800E2CE).
- Pixel↔panel conversion (movement): X = (x·40 − 140)<<16, Y = (y·24 − 20)<<16. Back-conversion `sub_800E258`: `((X>>16)+160)/40`, `((Y>>16)+32)/24`, signed division.

### 2.2 PanelData (0x20 bytes)

| Off | Type | Meaning |
|---|---|---|
| 0x00 | u8 | Visible. 1 for valid panels at init (`byte_800C590`). `object_hidePanel` 0x0800CE32 / `object_showPanel` 0x0800CE42. Renderer only. |
| 0x01 | u8 | Highlight: 1 normal (`object_highlightPanel` 0x0800CBD8), 2 blue (`…Blue` 0x0800CBEE, only if not already 1). Cleared by the renderer every frame. Visual. |
| 0x02 | u8 | **Type** (0..12, §2.3). |
| 0x03 | u8 | **Alliance**, the current owner. |
| 0x04 | u8 | **Home alliance** = `colinfo[x].orig`. Written at init and by `sub_800C878`; read by the area-return logic. |
| 0x06 | u8 | Displayed type. Set to Type by `_object_updatePanelParameters`, crack/break/poison; blink values from `sub_800C380`. Renderer only. |
| 0x07 | u8 | Displayed alliance. Set by `_object_updatePanelParameters` and every tick by `sub_800C746` (with XOR blink). Renderer only. |
| 0x08/0x09/0x0D | u8 | One-frame display override (type, alliance, enable) from `object_setPanelTypeBlink` 0x0800CC52. Visual. |
| 0x0A/0x0B | u8 | This panel's x, y. 0x0A is read by the volcano branch. |
| 0x0C | u8 | "Draw front edge" (row 3). Visual. |
| 0x0E | u16 | **Hole timer.** Reset to `H = sub_800C488()` each tick for non-hole types; counts down while type is 1. `H = 0x1E0` if `GetBattleMode()==1` (BattleSettings byte 3), else **0x258 (600)**. |
| 0x10 | u16 | Alliance-return blink timer. `sub_800C81E` writes byte 0x5A; `object_setPanelAlliance` writes u16 0; `sub_800C746` decrements it. Visual only. |
| 0x12 | u16 | **Road timer.** 0x708 at init. Reset to 0x708 each tick for types other than 9..12; counts down for 9..12. `_object_setPanelType` sets 0x708 when the new type is 9..12. |
| 0x14 | u32 | **Flags**, a cached word (§2.4). |
| 0x18 | u32 | **Previous-tick latch.** At the end of each panel's tick: `Flags` if Type==3, else 0. |
| 0x1C | ptr | **ReserverObjectPtr** (§2.9). |

### 2.3 Panel types

| Id | Table word | Name | Behaviour (details below) |
|---|---|---|---|
| 0 | 0x18000 | Missing / void | Not standable; AirShoes can stand on it and nothing happens (there is no falling logic). `_object_setPanelType` and `object_setPanelAlliance` refuse to change it. Never ticks. |
| 1 | 0x14000 | Broken (hole) | Timer +0x0E counts down to 0, then it becomes 2; blinks during the last 60 ticks. |
| 2 | 0x10010 | Normal | |
| 3 | 0x10050 | Cracked | Breaks to 1 when a body leaves it (§2.6.2). |
| 4 | 0x10110 | Poison | 1 damage on the first tick standing on it, then every 7 ticks (§2.12). |
| 5 | 0x12010 | Holy | Incoming damage halved, rounding up (§4.4). |
| 6 | 0x10410 | Grass | Heat hit on an object standing here adds +SelfDamage. A heat hitbox passing over it turns it Normal. A Wood-element object standing on it heals. |
| 7 | 0x10810 | Ice | Slide after a step. Aqua hit on a body standing here → freeze and panel becomes Normal. |
| 8 | 0x11010 | Volcano (lava) | Periodic eruption (spawns T3/7 with 50 damage). Aqua hitbox over it → Normal. |
| 9 | 0x10210 | Road up (dy −1) | Conveyor. |
| 10 | 0x10210 | Road down (dy +1) | Conveyor. |
| 11 | 0x10210 | Road left (dx −1, toward x=1) | Conveyor. |
| 12 | 0x10210 | Road right (dx +1, toward x=6) | Conveyor. |

Notes on types 9–12:
- Directions are absolute (table `byte_800E538`).
- Roads revert to Normal after 0x708 unpaused, unfrozen ticks, including the stage's initial roads.
- A Wood hitbox over them makes them Normal.

Evidence:
- Road direction **[verified]** with a scripted input: the alliance-1 navi stepped on (5,1), type 11, and was carried to (4,1).
- Sand, metal and swamp do not exist. `word_3007924` has exactly 13 entries and is not bounds-checked; a type > 12 would read code bytes.

### 2.4 The Flags word

`_object_updatePanelParameters(x, y)` (0x030078E0). It is a no-op for an invalid panel.

```
p[6] = p.Type ; p[7] = p.Alliance
p.Flags = p.Type | word_3007924[p.Type] | (p.Alliance << 5)
        | (p.ReserverObjectPtr ? 0x80 : 0)
        | sub_3007978(x, y)
sub_3007978: m = mask[y*8+x]; slot = 0; acc = 0
             loop { c = m>>31; m <<= 1; if c { acc |= Self(slot) & 0xFFFF0000 } else if m == 0 break; slot++ }
```

In `sub_3007978`, `Self(slot)` = `eCollisionData[slot]+0x30`. This includes slots that were freed but whose bits are still in the mask (§3.1).

**Low bits**

| Bit | Meaning |
|---|---|
| 0x0F | Type |
| 0x10 | Solid (standable): types 2–12 |
| 0x20 | Alliance 1 |
| 0x40 | Cracked |
| 0x80 | Reserved |
| 0x100 | Poison |
| 0x200 | Road |
| 0x400 | Grass |
| 0x800 | Ice |
| 0x1000 | Volcano |
| 0x2000 | Holy |
| 0x4000 | Broken |
| 0x8000 | Missing |
| 0x10000 | Always set on a refreshed valid panel |

Flags is 0 on invalid panels and on all panels before the first refresh.

**High bits** are the union of the registered objects' Self high halves (§3.3). The ones the panel and movement code test:

| Bits | Meaning |
|---|---|
| 0x08000000 / 0x04000000 | Body of alliance 0 / 1 |
| 0x02000000 / 0x01000000 | Other bodies of alliance 0 / 1 |
| 0x00800000 | Neutral object |
| 0x00400000 / 0x00200000 | Player navi of alliance 0 / 1 |
| 0x00100000 | Floating (FloatShoes navi) |
| 0x00080000 | Blocker |

Named masks:

| Mask | Meaning |
|---|---|
| 0x0F880080 | "Occupied" (bodies, neutral, blocker, reservation) |
| 0x0F800000 | "A body is here" |
| 0x0F080080 | Occupied, ignoring neutral objects |

**When Flags is refreshed:**
- For each panel an object registers on or unregisters from (§3.6).
- By `_object_setPanelType`, `object_setPanelAlliance`, the area-return code and the panel tick when a type changes.
- For all panels at battle start (`sub_800C8F0` → `sub_30078C8`: y=4..1, x=7..1).

**Bit-edits without a full refresh:** reserve/unreserve toggle 0x80, and crack/break/poison edit the type bits directly (§2.7).

### 2.5 Initial layout (battle start)

Inputs:
- `BattleState+6 = BattleSettings[0]` (layout index), set by `sub_800A2F8` 0x0800A2F8.
- `BattleSettings[6]` = column pattern (`GetBattlePanelColumnPattern`).

The per-round layout index comes from a list received over the link before the match. Treat BattleSettings as input. No RNG is used here.

`sub_80079F0` calls `sub_800BF88(BS+6, colpat)` (0x0800BF88), which:
- calls `sub_800C4BC`;
- sets `*(u32*)0x0203CB04 = 0x8C`;
- sets render animation state (`byte_2036740`).

```
sub_800C4BC(layout, colpat):                      // 0x0800C4BC
  rows = (u32*)(0x0800D730 + layout*12)           // sub_800C6D6; one u32 per row y=1..3
  zero 0x40 bytes at 0x02034010
  for x in 0..7: colinfo[x].orig = (1<=x<=6) ? (colpat >> (x-1)) & 1 : 0xFF
  zero 0x500 bytes at ePanelData
  for y in 0..4, x in 0..7:
     p = panel(x,y)                               // sub_800C6E0, unchecked
     p[0]   = byte_800C590[y*8+x]
     t      = valid(x,y) ? (rows[y-1] >> (4*(x-1))) & 0xF : 0
     p[2] = p[6] = t
     p[3] = p[4] = p[7] = colinfo[x].orig         // border panels get 0xFF
     p[0xC] = byte_800C5B8[y*8+x]; p[0xA] = x; p[0xB] = y
     p.u16[0xE] = H;  p.u16[0x12] = 0x708
  sub_800C67C()                                   // home-runs table
```

- Flags stay 0 until `sub_8007A0C` runs `sub_800C8F0` (a refresh of all panels) and the first `sub_800BFC4` tick. That first tick is why the eruption counter reads 0x8B and roads read 0x707 at the first traced battle frame.
- **Home-runs table** `sub_800C67C`:
  - Start with L=1, R=6, d=+1, and loop while L<R.
  - Take start = (d>0 ? L : R). `sub_800C84A(start, d)` counts the consecutive columns, going in direction d, whose `colinfo.orig` equals that of start. Call the count `cnt` and the owner `al`.
  - Emit `{start+(cnt−1)*d, −d, cnt, al}`.
  - Advance that side by `cnt*d`, then flip d.
  - Terminate with 0xFF.
  - For the standard 3|3 split this gives `03 FF 03 00 | 04 01 03 01 | FF` **[verified]**.

**[verified]** against the machgun setups:

| Round | Settings | Layout entry | Row 1 types | Other rows | Owners |
|---|---|---|---|---|---|
| R1 | `e3 64 15 00 0b 00 38 …` | 0xE3 = `{0x00BB22CC, 0x00222222, 0x00222222}` | 12 12 2 2 11 11 | Normal | Colpat 0x38 → columns 4–6 alliance 1 |
| R2 | `d6 …` | 0xD6 = `{0x00222022, 0x00222222, 0x00220222}` | — | — | Type 0 at (3,1) and (4,3) |

- Frame-72 Flags for R1: (1,1) = 0x0001021C and (5,1) = 0x0001023B.

### 2.6 Per-tick panel update (`sub_800BFC4`, 0x0800BFC4)

```
sub_800BFC4:
  if battle_isPaused() or battle_isTimeStop(): return
  sub_800C746()                                   // area return (§2.6.3)
  if --*(u32*)0x0203CB04 == 0: *(u32*)0x0203CB04 = 0x8C
  for y in 1..3: for x in 1..6:
     p = panel(x,y)
     sub_800C380(x, y, p)
     p.u32[0x18] = (p.Type == 3) ? p.Flags : 0    // latch for next tick (object_getPanelParameters)
```

The tick runs **after** all objects have updated. So the latch holds the Flags as of the end of the previous tick's object phase, and `p.Flags` holds the current one.

#### 2.6.1 `sub_800C380` (0x0800C380): per-panel state machine

```
t = p.Type; if t == 0: return
H = sub_800C488()
case t == 1 (broken):
    p.u12 = 0x708
    p.uE -= 1
    if p.uE == 0: p.Type = 2; update(x,y); p.uE = H; return
    p[6] = (p.uE <= 60 && (p.uE & 2)) ? 2 : 1          // blink
case t == 3 (cracked):
    p.uE = H; p.u12 = 0x708
    L = p.u32[0x18]                                     // last tick's latch
    if (L & 0x0F800000) && !(L & 0x00100000) && !(p.Flags & 0x0F880080):
        p.Type = 1; update(x,y); p.uE = H; PlaySoundEffect(0x97)
case t == 8 (volcano):
    p.uE = H; p.u12 = 0x708
    if cnt == (p[0xA] <= 3 ? 0x8C : 0x46): sub_80C5B76(x, y)
case 9 <= t <= 12 (road):
    p.uE = H; p.u12 -= 1
    if p.u12 == 0: p.Type = 2; update(x,y); p.u12 = 0x708; return
    p[6] = (p.u12 <= 60 && (p.u12 & 2)) ? 2 : t
default (2,4,5,6,7): p.uE = H; p.u12 = 0x708
```

In the volcano case, `cnt` is the global counter after its decrement.

#### 2.6.2 Timing consequences

- **Cracked → broken on step-off.** The panel breaks on the first tick where last tick's latch shows a body bit (0x0F800000) with no floating bit (0x00100000), and the current Flags show no body, blocker or reservation.
  - A FloatShoes navi carries 0x00100000 and never breaks cracked panels.
  - A reservation (0x80) on the panel keeps it cracked.
  - **[verified-soundmod]** latch 0x08410053 → break at frame 28728.
- **Hole duration.**
  - A panel broken by the panel tick reverts after exactly H ticks.
  - A panel broken during an object update in tick N: its `uE` was set to H at tick N−1, so it reverts in tick N+H−1.
  - **[verified-soundmod]**: blink, and revert at `uE==0`.
- **Volcano eruptions.** Counting the init tick as tick 1, columns x≥4 erupt at ticks 70, 210, …; columns x≤3 at 140, 280, ….
  - `sub_80C5B76(r0=x, r1=y, r2=0, r3=0, r4=0x28, r6=0x32)` calls `object_spawnType3(index 7, X=y, Y=0, Z=0, Params=0x28)`. If a slot is allocated it sets PanelX/Y, Element=0, and `*(u32*)(obj+0x2C) = 0x32` (Damage 50).
  - Alliance stays 0.
  - **[verified-soundmod]** eruptions alternate between columns 3 and 4 every 70 ticks.
  - The eruption, T3/7 (`sub_80C5A34`, engine `kinds::eruption`): spawned after the objects ran, it inits the next tick. Init: off the field → freed; sprite (0x10, 0x24); Param1 += 0x28 (0x50 = 80) and Timer = Param1; one pixel back (Y) and down (Z); collision data set up with self type 0x48, target type 0x2A, hit modifier 1, hit effect 1, region off; presented; then the update. Update: remove the collision data (hits resolve) and `object_spawnCollisionEffect`; battle over → region off, state 8. If the panel is no longer a volcano (flag 0x1000) while the region is on → Timer = 3 and region off (and VISIBLE off if the panel holds flags 0x3800000); on a volcano, a hit (FlagsFromCollision) does the same without shortening the timer. Timer −1: at 0 region off and state 8; at Param1 − 0x28 (40 left) the region comes on, Y and Z back to the panel, anim 1; 2 later anim 2; at 2 anim 1. Then present. Trace-verified from its spawn at soundmod 26104.
- **Roads** revert after 0x708 ticks during which the panel tick ran.

#### 2.6.3 Area steal / return (`sub_800C746`, 0x0800C746)

`colinfo[c]` layout:

| Off | Size | Field |
|---|---|---|
| +0 | u8 | orig |
| +1 | u8 | returnReady |
| +2 | u8 | nStolen |
| +3 | u8 | nStolenFree |
| +4 | u8 | stolenMask |
| +5 | u8 | freeMask |
| +6 | u16 | timer |

Masks use `1<<row`.

```
for c in 1..6:
   (n, nf, m, fm) = sub_800C6F0(c)   // rows with p[3] != p[4]; "free" = !(Flags & 0x0F880080)
   store into +2..+5
   if n == 0: timer = 0
   if timer != 0 and --timer == 0: returnReady = 1
for run in byte_203F6B0 until 0xFF:            // {start, dir, cnt, owner}
   c = run.start; k = 0
   repeat run.cnt:
      if colinfo[c].stolenMask:
         if colinfo[c].returnReady && stolenMask == freeMask: list[k++] = (c, freeMask)
         else k = 0
      c += (s8)run.dir
   repeat k: colinfo[list[0].c].returnReady = 0; sub_800C81E(list[0].c, list[0].mask, run.owner)
for y in 1..3, x in 1..6:                       // display only
   p[7] = p[3]; if p.u10: p.u10 -= 1; p[7] ^= (p.u10 & 4) >> 2
```

- The last loop resets its index to 0 on every pass (`loc_800C7BE`), so only the list's first column returns
  (k times over); the others return on later ticks, one a tick. Columns of one run that come due together
  (JudgeMan's give-back sets every stolen column's timer to 1) so return front to back a tick apart
  (soundmod round 3: column 4, then column 5 a tick later).
- `sub_800C81E(c, mask, al)`: for each row bit, sets `p[3]=al` and byte `p[0x10]=0x5A`, then `update`.
- Effect: stolen columns return starting from the outermost. A column that is still occupied cancels the return of the inner columns collected so far in that run.
- `returnReady` is not cleared when n drops to 0.
- Timer-driven return was **not observed** in any replay (code only). Re-grabs were observed.

### 2.7 Panel modification API

All take (x, y), resolve the panel with `getPanelDataOffset`, and return 0 for invalid coordinates unless noted. `F` = p.Flags.

**Crack**: `object_crackPanel` 0x0800C938 and `object_crackPanelDup1` 0x0800C998 (identical).
- If `!(F & 0x10)`, return 0.
- If not already cracked (`!(F & 0x40)`): `F = ((F|0x40) & ~0x3F0F) | 3`; `Type = p[6] = 3`; sound 0x97; return 1. There is no occupancy check.
- If already cracked: if `F & 0x0F880080`, return 0. Otherwise break it: `F = (F & ~0x3F5F) | 1`, `Type = p[6] = 1`, sound 0x97, return 1.

**Break** variants (all require `F & 0x10`):

| Function | Addr | Rule |
|---|---|---|
| `object_breakPanel` | 0x0800C9F8 | If `F & 0x0F880080`, return 0; else break and return 1. |
| `object_breakPanel_dup1` | 0x0800CA34 | If `F & 0x0F080080`, crack instead and return **0**; else break and return 1. |
| `_dup2` / `_dup3` | 0x0800CA8C / 0x0800CAE8 | If `F & 0x0F880080`, crack and return 1; else break and return 1. |
| `object_breakPanelLoud` | 0x0800CB44 | Like `_dup2`, but plays sound 0xDA. |

Breaking does not set 0x4000 in Flags; the bit appears at the next full refresh (**[verified-soundmod]**: 0x10001 then 0x14001 later in the same tick). No crack or break function touches +0x07, +0x0E or the latch.

**Poison**: `object_panel_setPoison` 0x0800CBA0 requires solid. It sets `F = (F & ~0x3F5F) | 0x114`, `Type = p[6] = 4`, plays sound 0x90 and returns 1. No occupancy check.

**Set type**: `object_setPanelType` 0x0800CC0A → `_object_setPanelType(x, y, t)` 0x030079A4.
- There is no null check: invalid coordinates touch BIOS space, which is effectively a no-op.
- If the current Type is 0, return.
- `Type = t`; if `9 <= t <= 12`, `p.u12 = 0x708`; then `update(x,y)`.
- No sound, no return value.

**Set alliance**: `object_setPanelAlliance(x, y, a)` 0x0800CC14.
- No null check. If Type is 0, return.
- `p[3] = a`; `p.u10 = 0`; `update`.
- Steal/return bookkeeping follows automatically from `p[3] != p[4]`.

**Column timers**:
- `object_setPanelAllianceTimerLong(col)` 0x0800CC36 sets `colinfo[col].timer = 0x708`.
- `…TimerShort` 0x0800CC44 sets it to 1, so the column returns next tick if possible.
- AreaGrab (`sub_80C64A0`) calls `setPanelAlliance(x, y, Param1)` then `TimerLong(x)` after its eligibility checks (see the chip spec).

### 2.8 Collision-driven panel changes

Both are run by the collision kernels (§3).

**`sub_3007708(x, y, cd)`** (0x03007708) runs during `remove` for **every panel of the object's region, whether or not anything was hit**. It returns early if paused, or if `cd.Self & 0x0C000000` (bodies never convert panels). Otherwise, with `e = cd.PrimaryElement`:

| Panel type | Element | Result |
|---|---|---|
| 6 (grass) | e == 1 (heat) | set type 2 |
| 8 (volcano) | e == 2 (aqua) | set type 2 |
| 9..12 (road) | e == 4 (wood) | set type 2 |

"Set type 2" means `_object_setPanelType(x,y,2)`. **[verified-soundmod]** grass burn.

**Aqua on ice** (`sub_3007460`, called from inside `sub_3007218`, §3.8 step 7.3) turns an ice panel under a body into Normal and freezes the body.

### 2.9 Occupancy and reservation

- **Occupancy** is not stored per panel. It is the panel's registration mask (§3) projected into the high half of Flags.
- A navi re-registers every tick at its CollisionData panel. During its own update its own bit is absent between its `remove` and its `present`.
- **Reservation**:

| Function | Addr | Behaviour |
|---|---|---|
| `object_reservePanel(x,y)` | 0x0801BB1C | If valid and `p.Reserver == 0`: `p.Reserver = obj`, `F |= 0x80`, `obj.header |= 0x20`, return 1. Otherwise return 0, including when obj already holds it. |
| `object_removePanelReserve(x,y)` | 0x0801BB46 | If valid and `p.Reserver == obj`: clear it, `F &= ~0x80`, return 1; else 0. |
| `sub_801BB6A` | | Returns the reserver. |
| `sub_801BB78` | | Releases every reservation held by an object with header flag 0x20 (y=4..1, x=7..1). |

- **[verified]** Pre-tick-2346 state: the alliance-0 navi stands on (4,2), an alliance-1 panel, with Flags 0x08510032, while its home panel (3,2) shows 0x00010092 (reserved).

### 2.10 Queries and movement validity

| Function | Addr | Semantics |
|---|---|---|
| `object_isValidPanel(x,y)` | 0x0800CC72 | `1<=x<=6 && 1<=y<=3` → r0=1, Z clear; else 0, Z set. Pure bounds; type is ignored. |
| `object_isCurrentPanelValid` | 0x0800CC66 | Same test on obj+0x12/+0x13. |
| `object_getPanelParameters(x,y)` | 0x0800C8F8 | Flags, or 0 if invalid. |
| `object_checkPanelParameters(x,y,set,clear)` | 0x0800CC86 | `F != 0 && !(F & clear) && (F & set) == set`. |
| `object_isPanelSolid(x,y)` | 0x0800CCB2 | `F & 0x10`. |
| `object_isCurrentPanelSolid` | 0x0800CCA6 | Same test on the object's current panel. |
| `object_getPanelsFiltered` / `…ExceptCurrentFiltered` | 0x0800CEA0 / 0x0800CE64 | Emit `(y<<4)|x` for y=3..1, x=6..1. Same set/clear test but without the `F != 0` check. Chip/AI helpers. |

**Navi single step**: `sub_800E618(x, y)` 0x0800E618.
1. If invalid, return 0.
2. `idx = ((F1 & 0x10 AIRSHOE)==0 && isPanelSolid(obj.PanelX, obj.PanelY)) ? 0 : 0x10`; `idx += alliance*8`.
3. `(set, clear) = tbl_800E660[idx]`; return `checkPanelParameters(x, y, set, clear)`.

`tbl_800E660`:

| idx | set | clear |
|---|---|---|
| 0x00 (A0) | 0x10 | 0x0B8800A0 |
| 0x08 (A1) | 0x30 | 0x07880080 |
| 0x10 (A0, AirShoes or standing on a non-solid panel) | 0x00 | 0x0B8800A0 |
| 0x18 (A1, same) | 0x20 | 0x07880080 |

So a navi may step onto a panel only if all of these hold:
- the panel is on the field;
- it is owned by the navi's alliance;
- it is not reserved;
- it carries no own-side body bit, no 0x02000000/0x01000000, no neutral bit and no blocker bit;
- it is solid, unless the navi has AirShoes or currently stands on a non-solid panel.

**[verified]** A shoeless navi was refused the (4,3) void; an AirShoes navi stepped onto the (3,1) void.

Slide/drag continuation uses `sub_800E5AC` 0x0800E5AC. It takes the same table with `idx = (AIRSHOE ? 0x10 : 0) + alliance*8` and has no current-panel rule.

Other variants:
- `sub_8010368`, table 0x08010388, used by NaviStats+0x31 "dash mode": A0 {0x10, 0x0F8800A0}, A1 {0x30, 0x0F880080}; with AirShoes the set word is 0x00 / 0x20.
- `sub_800E680`, table 0x0800E6C8, ignores ownership (chip helper).

The **step timeline** (input at tick F) is specified in `objects-and-player.md`. Panel-relevant points **[verified]**:
- **F**: destination reserved (Flags 0x10012 → 0x10092); `F1 |= 0x40`; FuturePanel set.
- **F+3**: PanelXY = FuturePanel; reservation removed; `object_updateCollisionPanels`. The old panel loses the body bit in that tick's remove; the new panel gains it at that tick's present.
- **F+8**: `F1 &= ~0x40`; `F1 |= 0x80000` (MOVE_COMPLETE).
- **F+12**: back to idle. The earliest next step starts at F+13.

### 2.11 Slides: ice, road, wind push, drag

State lives in the object:

| Field | Meaning |
|---|---|
| +0x0F SlideType | 1 wind, 2 ice, 3 road |
| +0x1B NumSlideTiles | |
| +0x1C/+0x1D SlideDelta | signed |
| +0x1E per-tile timer | |
| +0x1F SlideState | 0 init, 4 stepping |
| `F1 & 0x1000` | SLIDING |
| `F2 & 0x10` | start-slide request |
| `ai+0x38` | road cooldown |

**Triggers** run in `sub_801AC6C` (stage A, §4.1):

`sub_801A36A` 0x0801A36A:
1. If not paused, not dimmed, and `ai+0x38 != 0`: decrement it. On the 1→0 tick, jump straight to the ice test.
2. Otherwise, if `F1 & 0x00100040` (DRAG | moving), return.
3. If the CollisionData panel type is 9..12 → `sub_801A400`: if `ai+0x38 == 0` and `!(F1 & 0x24)`, then `F2 |= 0x10`, SlideType=3. Return.
4. Else, if `F1 & 0x80000` (MOVE_COMPLETE): clear it. If the panel type is 7 → `sub_801A3DA`: if element ≠ aqua, `!(F1 & 0x24)` and `F1 & 0x02000000`, then `F2 |= 0x10`, SlideType=2.

**Wind push** `sub_801AEB0` is part of the hit-modifier handling (§4.7).

**Driver** in stage B (`sub_801AF44`):
- If `F1 & DRAG`: action 5.
- Else if `F2 & 0x10`: clear it, then `sub_80166B6`.
- Else if SLIDING: `sub_80166B6`.
- Else SlideState = 0.

`sub_80166B6` dispatches:

**Init** (`sub_80166D0`):
1. Set SLIDING; PanelXY = FuturePanelXY; snap coordinates; set collision panels to current; unreserve Future.
2. `(dx, dy, n) = sub_800E468()`. It returns 0 if `!sub_800E5AC(Panel + (dx,dy))`.
3. If n: Future = Panel + (dx,dy); reserve it; timer=4; state=4.
4. Else: if road, `ai+0x38 = 5`; then SlideType=0 and clear SLIDING.

Delta sources for `sub_800E468`:

| SlideType | Function | Source |
|---|---|---|
| 2 (ice) | `sub_800E4C8` | `byte_800E4E8[cd.Direction]`: 1 → (0,−1,1); 2 → (0,+1,1); 3 → (−1,0,1); 4 → (+1,0,1); else 0. dx × `object_getEnemyDirection` (= 1−2·alliance). |
| 3 (road) | `sub_800E500` | Absolute table 0x0800E538. |
| 1 (wind) | `sub_800E548` | From HitModifierFinal (§4.7). |

**Step** (`sub_8016730`):
- Each tick, `timer -= 1`. While `timer > 0`: `X += dx·0xA0000`, `Y += dy·0x60000`, then panels from coordinates and collision panels from panels.
- At `timer == 0`:
  1. Unreserve Future; `Panel = Future`; snap.
  2. If the new panel is ice and element ≠ aqua: `NumSlideTiles += 1`. This ignores FloatShoes and 0x02000000.
  3. Else if the new panel is a road and `!(F1 & 0x24)`: if SlideType is already 3, set `ai+0x38 = 5` and stop. Otherwise switch to road (SlideType 3, re-derive the delta; `NumSlideTiles += 1`).
  4. `NumSlideTiles -= 1`. If still > 0 and the next panel passes `sub_800E5AC`: continue with timer 4.
  5. Otherwise stop: `cd.Direction = sub_801683C(dx, dy, alliance)`, state 0, clear SLIDING, SlideType 0.
- The result is 4 ticks per tile.
- **[verified]** Timing for road: triggered at F+9 after a step; road→road chains rest 6 ticks. Ice is code only.

### 2.12 Standing effects (navi)

All of these run in the navi's stage A (§4.1) unless noted.

**Poison / grass**: `sub_801A186` 0x0801A186. It returns early if dimmed, paused, `cd.Region == 0`, or the CollisionData panel is invalid. Otherwise, with t = the panel type at `(cd.PanelX, cd.PanelY)`:

- **t == 4 and `!(F1 & 0x08000028)`** (immune if flag 0x08000000, FLOATSHOE 0x20 or INVULNERABLE 0x08):
  - `cd+0x08 -= 1`. If the result is < 0: `cd+0x08 = 6` and `u16 cd+0x8C += 1`.
  - That is 1 damage on the first tick, then every 7 ticks.
  - The damage bypasses FinalDamage, barriers, holy and Undershirt (§4.5).
- **Otherwise**: `cd+0x08 = 0`.
  - Quirk: for an immune object on poison, the grass test below compares the *flags value* with 6.
  - If t == 6 and `obj.Element & 0xF == 4` (wood): let `c = BS+0x0E` if HP > 9, else `BS+0x16`. If `c == 0`, call `object_addHP(1)` (clamped to MaxHP).
  - That is +1 HP every 20 ticks, or every 180 ticks at HP ≤ 9. **[verified-soundmod]**

**Holy** (type 5): damage halving in `object_calculateFinalDamage1` (object panel) and in barriers (CollisionData panel), §4.2/§4.4.

**Heat on grass** (collision kernel, receiver's CollisionData panel): §3.8.

**Aqua on ice** (freeze): §3.8.

**Volcano**: no standing effect; eruptions only (§2.6).

---

## 3. Collision

### 3.1 Slots and masks

**Battle init** `sub_801986C` (0x0801986C, from `sub_8007A0C`):
- Active bitfield = 0.
- For k in 0..31: zero slot k, then `slot.CollisionIndexBit (+0x44) = 0x80000000 >> k`.
- `sub_8019FA4` zeroes the 40 masks.
- **The masks are never cleared globally again.** Each bit is removed only by its owner's `remove` (§3.6).

**Allocate** `object_createCollisionData` (0x08019892):
- Takes the lowest free slot (bit 31 first).
- Sets its active bit and zeroes `+0x00..+0x43` and `+0x48..+0xA7`, keeping +0x44.
- `Enabled (+0) = 1`; `obj+0x54 = slot`; returns it (0 if all 32 are used).

**Free** `object_freeCollisionData(r0)` (0x080198CE):
- If r0 != 0: `Enabled = 0` and clear the active bit.
- **Mask bits and all other fields are left untouched.** A freed slot whose bits are still in some mask keeps being pair-tested and keeps contributing Self to panel Flags until the bits are removed, or the slot is reallocated (which zeroes Self/Target).
- `Enabled` is never read by the kernels.

### 3.2 CollisionData layout (0xA8)

"(acc)" marks the per-window accumulators at `+0x68..+0xA7`, which `object_presentCollisionData` zeroes.

| Off | Size | Field | Meaning / writers |
|---|---|---|---|
| 0x00 | u8 | Enabled | |
| 0x01 | u8 | Region | Shape (§3.4). 0 means no panels. `object_setCollisionRegion` 0x0801A07C, `object_clearCollisionRegion` 0x0801A074. Setup writes 1. |
| 0x02 | u8 | PrimaryElement | `obj.Element & 0xF`: 0 null, 1 heat, 2 aqua, 3 elec, 4 wood. Some hitboxes use 5 (§3.8). Also written by `sub_8019F8C`. |
| 0x03 | u8 | Unk_03 | Guard directions: `|= 1<<hitter.Flip` on a blocked hit. Cleared by present unless dimmed. |
| 0x04/0x05 | u8 | Alliance / Flip | Copied from obj+0x16/+0x17. |
| 0x06 | u8 | Barrier type | §4.2. Barrier state also uses +0x14 (weak element), +0x15 (saved hmF), +0x16 (u8 HP), +0x17 (threshold), +0x1A (u16 timer), +0x1B. |
| 0x07 | u8 | Counter/stamina byte | Low byte of obj+0x2E at setup. Bits 0–6 value; bit 7 = "cannot counter". |
| 0x08 | u8 | PoisonPanelTimer | §2.12 |
| 0x09 | u8 | HitEffect | 0xFF = none. `object_setCollisionHitEffect` 0x0801A140. |
| 0x0A/0x0B | u8 | PanelX/Y | Region anchor. Setup, `object_updateCollisionPanels` 0x0801A04C (also computes Direction), `object_setCollisionPanelsToCurrent` 0x0801A066. |
| 0x0C | u8 | Direction | Last move: 0 none, 1 up, 2 down, 3 back, 4 forward, 5 other (`sub_800E994`, `sub_801683C`). |
| 0x0D | u8 | CounterTimer | Nonzero = counterable (§4.9). |
| 0x0E | u8 | HitModifierBase | What I inflict (§4.7). |
| 0x0F | u8 | HitModifierFinal | OR of the hitters' HitModifierBase. |
| 0x10 | u8 | StatusEffectBase | What I inflict (§4.8). |
| 0x11 | u8 | StatusEffectFinal | Last nonzero hitter StatusEffectBase (overwrite). Freeze (0x50) from aqua-on-ice. Always cleared by present. |
| 0x12 | u16 | Bugs | Low byte = code, high byte = argument (`sub_801A4D0(r0, r1)`: `(r1<<8) + r0`). |
| 0x18 | u8 | SecondaryElementWeakness | Bitfield (`sub_8019F9E`). |
| 0x19 | u8 | SecondaryElement | `obj.Element & 0xF0`: 0x10 break, 0x20 wind, 0x40 cursor, 0x80 sword. |
| 0x1C..0x2C | u16 | Status timers | +0x1C paralyze, +0x1E confuse, +0x20 blind, +0x22 immobilize, +0x24 flash, +0x26 F1-bit-4 timer, +0x28 invulnerable, +0x2A freeze, +0x2C bubble (§4.8). |
| 0x2E | u16 | SelfDamage | Decoded damage (§3.5). |
| 0x30 | u32 | Self | SelfCollisionTypeFlags (§3.3). |
| 0x34 | u32 | Target | TargetCollisionTypeFlags. |
| 0x38 | ptr | Parent | Owning object. |
| 0x3C / 0x40 | u32 | F1 / F2 | Status and request flags (§4). |
| 0x44 | u32 | CollisionIndexBit | Constant `0x80000000>>slot`. |
| 0x48/0x4C/0x58/0x60 | u32 | | Pointers to status visual objects (confuse/blind/freeze/bubble). |
| 0x54 | u32 | | See the present quirk. |
| 0x64 | u32 | | Summed into the receiver's +0xA0 (unused by the navi). |
| 0x68 | u32 | (acc) PairTested | Slot bits already paired since my last present (§3.7). |
| 0x6C | u32 | (acc) RawHitFlags | OR of all hitters' Self, unfiltered. |
| 0x70 | u32 | (acc) FFC | OR of filtered hitters' Self, plus special bits: 0x1 = I was guarded (hitter side), 0x40 = counter hit, 0x20000 = I blocked a hit. |
| 0x74 | u8 | (acc) ExclamationIndicator | Last hit's total multiplier − 1 ("!!"). |
| 0x75 | u8 | (acc) DamageMultiplier | Last hit's weakness count. |
| 0x76 | u8 | (acc) DamageElements | OR of hitters' secondary element (filtered). |
| 0x77 | u8 | (acc) | Same, unfiltered. |
| 0x78 | u16 | (acc) | Sum of SelfDamage of elec hitters (filtered, unmultiplied). |
| 0x7C | u32 | (acc) HitByMask | OR of hitters' CollisionIndexBit (filtered). `sub_801A4DC` lists their parents. |
| 0x80 | u16 | (acc) FinalDamage | §4.4. |
| 0x82..0x8C | u16×6 | (acc) ElementDamage[0..5] | +0x82 null, +0x84 heat, +0x86 aqua, +0x88 elec, +0x8A wood, +0x8C element 5 / poison. Multiplied, filtered. |
| 0x8E | u16 | (acc) | Sum of hitters' `+0x07 & 0x7F`, counter hits left out (mood damage, §4.9). |
| 0x90 | u16 | (acc) | Counter accumulator; 0x8000 = counter hit. |
| 0x92 | u16 | (acc) | Number of hitters with Self bit **0x100** (drain credit, §4.6). |
| 0x94..0x9E | u16×6 | (acc) RawElementDamage[0..5] | Unmultiplied, unfiltered. Element 5 lands at +0x9E **[verified]**. |
| 0xA0 | u32 | (acc) | Sum of hitters' +0x64. |
| 0xA4 | u16 | (acc) InflictedBugs | Last filtered hitter's Bugs whose low byte ≠ 0 (a 16-bit store). |

### 3.3 Collision type flags

`sub_801A0BA(alliance, index)` (0x0801A0BA) returns `table[index*8 + alliance*4]`, where `table` = `byte_8019C7C` (0x08019C7C), 89 entries.

**A reacts to B iff `A.Target & B.Self != 0`** (`sub_3007650`). Every tested pair is evaluated in both directions. For example, an alliance-1 attack (Self 0x40000080, Target 0x2A800000) and the alliance-0 navi (Self 0x08410080, Target 0x55800200):
- the navi reacts to the attack (0x40000000), which is the damage;
- the attack reacts to the navi (0x08000000), which is how the attack learns it hit a body (`FFC & 0x0C000000`).

```
idx  A0        A1          idx  A0        A1          idx  A0        A1
00   00000000  00000000    1e   00004280  00004280    3c   8000408a  4000408a
01   08410080  04210080    1f   80040082  40040082    3d   00000080  00000080
02   55800200  aa800200    20   80000888  40000888    3e   02110080  01110080
03   51800100  a2800100    21   8004008a  4004008a    3f   0851008a  0431008a
04   80000080  40000080    22   c004808e  c004808e    40   20012080  10012080
05   15800000  2a800000    23   00080000  00080000    41   04000000  08000000
06   80000082  40000082    24   80000c88  40000c88    42   80002090  40002090
07   80002080  40002080    25   80000480  40000480    43   00040080  00040080
08   80000090  40000090    26   c0040088  c0040088    44   20050080  10050080
09   80000180  40000180    27   c0048082  c0048082    45   8000208a  4000208a
0a   80000088  40000088    28   80002086  40002086    46   800140a0  400140a0
0b   80000084  40000084    29   02010082  01010082    47   800000a0  400000a0
0c   20010080  10010080    2a   3d800000  3e800000    48   c0040080  c0040080
0d   55840000  aa840000    2b   80000488  40000488    49   3f800000  3f800000
0e   00810082  00810082    2c   8000408c  4000408c    4a   80000482  40000482
0f   ff800002  ff800002    2d   8000408a  4000408a    4b   80000c88  40000c88
10   08510080  04310080    2e   80004086  40004086    4c   02014082  01014082
11   80002480  40002480    2f   80004088  40004088    4d   55000200  aa000200
12   80000880  40000880    30   8000408e  4000408e    4e   02010088  01010088
13   02010080  01010080    31   80005084  40005084    4f   0081008a  0081008a
14   55800000  aa800000    32   80005080  40005080    50   08412080  04212080
15   8000008a  4000008a    33   c004000e  c004000e    51   08410092  04210092
16   80000086  40000086    34   80004084  40004084    52   80001080  40001080
17   8000008e  4000008e    35   08410082  04210082    53   08412082  04212082
18   50000000  a0000000    36   80004082  40004082    54   80002092  40002092
19   8000008c  4000008c    37   0201008a  0101008a    55   0841009a  0421009a
1a   8000009e  4000009e    38   0f800000  0f800000    56   80000884  40000884
1b   800040a0  400040a0    39   08510082  04310082    57   800000b2  400000b2
                           3a   0841008a  0421008a    58   c0044080  c0044080
                           3b   20010082  10010082
```

Indices seen in PvP:

| Object | Self idx | Target idx | Notes |
|---|---|---|---|
| Navi body | 0x01 | 0x02 | Self 0x10 with FloatShoes (adds 0x00100000). |
| Generic attacks | 0x04 | 0x05 | |
| Machgun-trace drain hitbox | 0x2C | 0x05 | |

Bits the engine tests directly (everything else only matters through `Target & Self`):

| Self bit(s) | Tested in | Meaning |
|---|---|---|
| 0x80000000 / 0x40000000 | Target words | Attack of alliance 0 / 1 |
| 0x20000000 / 0x10000000 | Target words | Object of alliance 0 / 1 |
| 0x08000000 / 0x04000000 | `sub_3007460`, `sub_3007708`, `sub_80C532E`, panel masks | Body of alliance 0 / 1 |
| 0x02000000 / 0x01000000 | Target words, step masks | Other bodies of alliance 0 / 1 |
| 0x00800000 | Target words, panel masks | Neutral object |
| 0x00400000 / 0x00200000 | Region filters 0x84/0x85 | Player navi of alliance 0 / 1 |
| 0x00100000 | Air/ground rule, cracked-panel rule | Floating |
| 0x00080000 | Panel masks | Blocker |
| 0x00010000 | Dimming gate | Interacts while dimmed. Setup ORs it in when created while dimmed; navi bodies always have it. |
| 0x00008000 | Air/ground rule | Ground-only hit |
| 0x4000 + 0x1000, or 0x0002 | Guard rule | Break guard |
| 0x2000 | Secondary weakness | Counts as sword against a pure sword weakness (0x80) |
| 0x1008 / 0x3000 / 0x0C003000 | Flag filters | Reach targets with F1 0x4 / 0x00800000 |
| 0x0100 | `sub_3007218` | Counted into receiver +0x92 (drain) |
| 0x0080 | Flag filters | Reach / be reached by F1 0x20 objects |
| 0x0010 | Guard (stripped), `sub_801A2CC` | On a navi: erases the held chip |
| 0x0004 | Flag filters, `sub_801A648` | Pierces invisibility and flashing (0x202) |
| 0x0002 | `sub_30074BA` | "Break": thaws a frozen target for +1 multiplier |

### 3.4 Regions

**Region & 0x80 == 0.** `PanelOffsetListsPointerTable` (0x08019B78)[Region] points to signed byte pairs `(dx, dy)`, terminated by `dx == 0x7F`.
- Each panel is `(PanelX + dx*dir, PanelY + dy)` with `dir = object_getFlipDirection(Alliance, Flip)`.
- Invalid panels (`object_isValidPanel`) are skipped; panel type is irrelevant.
- **List order is processing order.**

```
 0 —                              1 (0,0)                          2 (0,0)(1,0)
 3 (1,0)                          4 (0,0)(0,-1)(0,1)               5 (0,-1)(0,1)
 6 (0,0)(1,0)(2,0)                7 (0,0)..(3,0)                   8 (0,0)..(5,0)
 9 (0,0)(0,-1)(0,1)(1,0)(-1,0)    10 (0,-1)(0,1)(1,0)(-1,0)
11 (0,0)(1,-1)(-1,1)(1,1)(-1,-1)  12 (1,-1)(-1,1)(1,1)(-1,-1)
13 (0,0)(1,-1)(1,1)               14 (1,-1)(1,1)
15 (0,0)(0,-1)(0,1)(1,0)(-1,0)(1,-1)(-1,1)(1,1)(-1,-1)     16 = 15 without (0,0)
17 (0,0)(0,-1)(0,1)(1,0)(1,-1)(1,1)
18 (0,0)(1,-1)(-1,1)              19 (0,0)(-1,-1)(1,1)
20 (0,0)(0,-1)(0,1)(1,0)          21 (0,0)(1,-1)(1,1)(1,0)
22 (0,0)(1,-1)   23 (0,0)(1,1)    24 (0,0)(0,-1)   25 (0,0)(0,1)
26 (0,0)(1,-1)(1,0)(1,1)(2,-1)(2,0)(2,1)
27 = 26 + (3,-1)(3,0)(3,1)        28 = 27 + (4,-1)(4,0)(4,1)
29 (0,0)(1,0)(-1,0)               30 (1,0)(-1,0)                   31 (0,0)(-1,-1)(-1,1)
32 (0,0)..(4,0)
33 (0,0)(0,-1)(0,1)(1,0)(1,-1)(1,1)(2,0)(2,-1)(2,1)(3,0)(3,-1)(3,1)
34 (-1,-1)(0,-1)(1,-1)(0,0)(-1,1)(0,1)(1,1)
35 dy=-1: dx 0..5, then dy=0: dx 0..5, then dy=1: dx 0..5
36 (0,-1)(1,-1)(2,-1)(-1,0)(0,0)(1,0)(-2,1)(-1,1)(0,1)
37 (-2,-1)(-1,-1)(0,-1)(-1,0)(0,0)(1,0)(0,1)(1,1)(2,1)
38 5x5: dy=-2..2 outer, dx=-2..2 inner
39 (0..5,-1), (5,0), (0..5,1)
40 (0,0)(1,0)(2,0)(2,-1)(2,1)       41 (0,-1)(1,-1)(2,-1)(1,0)(0,1)(1,1)(2,1)
42 (0,-1)(0,1)(0,0)(1,0)(2,0)(3,0)(4,0)(5,0)
43 (0,0)(-1,0)(-2,0)
44 dy=-2: dx 5..-5; dy=2: dx 5..-5; dy=-1, dy=1, dy=0: dx 5,4,3,2,-2,-3,-4,-5 each (46 entries)
45 (0,0)(0,-2)(0,-1)(0,1)(0,2)       46 (0,-2)(0,-1)(0,1)(0,2)
```

**Region & 0x80 != 0** ("filtered whole field"). `byte_8019C34`[Region & 0x7F] gives `(want, forbid)`. The region is every panel, iterating y=1..3 then x=1..6, with `object_checkPanelParameters(x, y, want, forbid)` **evaluated at registration time**:

| Region | want | forbid | Panels |
|---|---|---|---|
| 0x80 | 0 | 0 | all |
| 0x81 | 0 | 0x20 | alliance 0 |
| 0x82 | 0x20 | 0 | alliance 1 |
| 0x83 | 0x10 | 0 | solid |
| 0x84 | 0x00400000 | 0 | holding the A0 navi |
| 0x85 | 0x00200000 | 0 | holding the A1 navi |
| 0x86 | 0 | 0x00600000 | no navi |
| 0x87 | 0x10 | 0x20 | solid, A0 |
| 0x88 | 0x30 | 0 | solid, A1 |

These regions were not exercised in machgun.

### 3.5 Setup

**`object_setupCollisionData(r0=slot, r1=selfIdx, r2=targetIdx, r3=hitModBase)`** (0x08019FB4), r5 = obj:

1. `Parent = obj`; `HitModifierBase = r3`.
2. `PrimaryElement = obj.Element & 0xF`; `SecondaryElement = obj.Element & 0xF0`.
3. Copy Alliance/Flip (obj+0x16), PanelX/Y (obj+0x12); `Region = 1`.
4. `+0x07 = (u8)obj+0x2E`; `SelfDamage = obj+0x2C` (the raw damage word).
5. `Self = table(selfIdx)`, and if dimmed `Self |= 0x10000`. `Target = table(targetIdx)`.
6. `sub_8019F44` decodes the damage word D = SelfDamage (LSL-carry tests, verified in the Rust):
   - `SelfDamage = D & 0x7FF`.
   - `D & 0x8000` (double): `SelfDamage *= 2`.
   - `D & 0x4000` (paralyze): `StatusEffectBase = 0x10`, `HitModifierBase = 1` (overwrites r3).
   - `D & 0x2000` (uninstall): `Bugs = (r1<<8) + 0xF8`, then **return**.
   - Else `D & 0x1000` (erase-cross / skull): `Bugs = (r1<<8) + 0xF7`.
   - `D & 0x0800`: no effect.
   - r1 here is the leftover `targetIdx*8 + alliance*4` from the Target lookup, so the Bugs high byte is "garbage". Reproduce it.

**`sub_801A082(r1, r2, r3)`** (0x0801A082) re-runs HitModifierBase, SelfDamage (from obj+0x2C), Self, Target, the dimming OR and `sub_8019F44` on an existing slot.
- Used by `sub_801393A` (0x0801393A) to switch the navi between Self idx 0x10 (`ns+0x1B` FloatShoes: also `F1 |= 0x20`) and idx 1 (`F1 &= ~0x20`).
- The same function sets `F1 |= 0x10` for `ns+0x1C` (AirShoes).

**Navi body** (`sub_80172F0`):
- `setupCollisionData(1, 2, r3 = sub_80107C0())`, where `sub_80107C0()` returns 3 if `GetBattleEffects() & 8` (netbattle), else 0. So the navi's HitModifierBase is 3 in PvP **[verified]**.
- SelfDamage is the navi's Damage field (10 in the trace).
- These values only affect objects that react to the navi body.

**Generic hitbox** `object_spawnCollisionRegion` (0x080C536A) creates a T3/3 object (`sub_80C52B0`). Arguments:

| Register / param | Meaning |
|---|---|
| r0/r1 | Panel |
| r2 | Element |
| r4 Params: Param1 | Region |
| r4 Params: Param2 | HitEffect |
| r4 Params: Param3 | Target idx |
| r4 Params: Param4 | Self idx |
| r6 | Damage word \| (counter byte << 16) |
| r7 | `HitModBase \| StatusBase<<8 \| bugLo<<16 \| bugHi<<24` |

Alliance and flip are copied from the spawner. Timer is 0 unless the spawner sets it. The object is inserted after the spawner and updates the same tick.

- **First update** (`sub_80C52D0`): free itself if its panel is invalid. Otherwise create and set up the collision, set Region/HitEffect/StatusEffectBase/Bugs, **present, then immediately remove**. Then run `sub_80C532E`.
- **`sub_80C532E`**:
  - Remove (resolve).
  - Call `object_spawnCollisionEffect`.
  - Optionally store `FFC & 0x0C000000` through a caller pointer.
  - If `FFC != 0` or `--Timer <= 0`: clear the region, free the collision, free the object. Otherwise present again; the next tick repeats remove → present.
- The default hitbox therefore lives within a single update and never appears in the trace **[verified]**.

### 3.6 Registration and resolution

**`object_presentCollisionData`** (0x0801A018):
1. `cd+0x54 = r1`. This is a quirk: the code computes `Unk_54 | FFC` into r0 and discards it.
2. If not dimmed: `HitModifierFinal = 0`, `Unk_03 = 0`, `+0x54 = 0`.
3. `StatusEffectFinal = 0`.
4. Zero `+0x68..+0xA7`.
5. `sub_300777C(cd)` (0x0300777C):
   - Region & 0x80 == 0: for each list panel that is valid: `mask[y*8+x] |= bit` (`sub_3007868`), then `_object_updatePanelParameters(x, y)`.
   - Region & 0x80: for each filtered panel, set the bit. **Bug:** the refresh is called as `_object_updatePanelParameters(y, r8)`. r8 is a pointer left by the previous call, so this is a no-op in practice and those panels' Flags are not refreshed here.
   - **Register quirk:** `sub_300777C` pushes r0 where it meant r8, and returns with `r8 = cd`. This matters only for register-exact emulation.

**`object_removeCollisionData`** (0x0801A00E → `_object_removeCollisionData` 0x03007550). It uses the slot's *current* Region, PanelX/Y, Alliance and Flip.
- Region & 0x80 == 0: for each list panel that is valid:
  1. `mask &= ~bit` (`sub_3007880`); the result is ignored here.
  2. `_object_updatePanelParameters(x, y)`.
  3. `sub_30075FC(x, y, cd)` (pair tests, §3.7).
  4. `sub_3007708(x, y, cd)` (panel conversion, §2.8).
- Region & 0x80: for y=1..3, x=1..6, clear the bit, and **only if it was set** do steps 2–4.

Consequences (reproduce them):
- A change of Region, anchor or flip between present and the next remove leaves **stale bits** at the old panels. The same happens when an object is freed without a final remove.
  - Stale bits keep being pair-tested (against whatever the slot holds) and keep feeding panel Flags, including the cracked-panel and step masks.
  - They disappear only when the owner of that slot index later removes with a matching shape.
- A Region 0 object registers nothing and tests nothing.

**Navi**:
- `sub_80EA484` → `sub_801AC6C` calls `remove` only if `battle_getFlags() & 1`.
- `sub_80EA484` ends with `present` unless paused.
- Hits therefore accumulate in the navi from the end of its update in tick N−1 through its remove in tick N, and are consumed in tick N.

### 3.7 Pair testing (`sub_30075FC`, 0x030075FC)

```
sub_30075FC(x, y, self):
  if battle_isPaused(): return
  m = mask[y*8+x]                         // self's own bit already cleared
  other = &eCollisionData[0]
  loop:
     c = m >> 31;  m <<= 1
     if c:
        if (self.PairTested & other.Bit) == 0:
            self.PairTested  |= other.Bit
            other.PairTested |= self.Bit
            sub_3007650(self, other)       // self reacts to other
            sub_3007650(other, self)       // other reacts to self
     elif m == 0: return
     other += 0xA8
sub_3007650(A, B): if A.Target & B.Self: { sub_3007218(A, B); sub_3007692(A, B) }
```

- Order: panels in region-list order; within a panel, slots in increasing index; objects in list order.
- `PairTested` makes a pair interact at most once between presents.
  - For two ordinary objects X (earlier in the list) and Y (later) that stay overlapped, the pair is resolved **once per tick during Y's remove**, using X's registration from this tick and Y's registration from the previous tick.
  - If they did not overlap at Y's previous remove, X's remove may resolve the pair instead.
  - Implement `+0x68` literally rather than this summary.
- Results reach an object that has already updated this tick at its next update. Results created before its update (including by its own remove) are consumed this tick.
- While paused, pair tests and panel conversions are skipped; registration and bit removal still happen.

### 3.8 Hit resolution (`sub_3007218`, 0x03007218)

Receiver R (r6) reacts to hitter H (r7). "Return" means H contributes nothing to R's filtered accumulators; `sub_3007692` still runs afterwards.

1. **Dimming.** If `battle_isTimeStop()` and not (`R.F1 & 0x01000000` or `H.Self & 0x10000`): return.
2. **Hitter-state filters** (H.F1 against R.Self):
   - `0x202` without `R.Self & 0x4`: return.
   - `0x4` without `R.Self & 0x1008`: return.
   - `0x00800000` without `R.Self & 0x0C003000`: return.
   - `0x08000000`: return.
   - `0x20` without `R.Self & 0x80`: return.
3. **Receiver-state filters** (R.F1 against H.Self):
   - `0x202` (INVIS | FLASHING) without `H.Self & 0x4`: return. These are the i-frames.
   - `0x4` without `H.Self & 0x1008`: return.
   - `0x00800000` without `H.Self & 0x3000`: return.
   - `0x08000000`: return.
   - `0x20` (FloatShoe bit) without `H.Self & 0x80`: return.
4. **Guard** (`R.F1 & 1`). Let `brk = (H.Self & 0x4000) ? 0x1002 : 0x0002`. If `(H.Self & brk) == 0`, the hit is blocked:
   - `H.FFC |= 1`.
   - `f = H.Self & ~0x10`. If `(f & 0x0C005000) == 0`: `R.Unk_03 |= 1 << H.Flip` and `f |= 0x20000`.
   - `R.FFC |= f`. Return (no damage, status or counter).
5. **Air/ground.** Return if `R.Self & 0x00100000 && H.Self & 0x8000`, or if `R.Self & 0x8000 && H.Self & 0x00100000`.
6. **Invulnerable.** Return if `R.F1 & 0x8`.
7. **Accumulate:**
   1. `R.HitByMask |= H.Bit`; `R.FFC |= H.Self`; `R.DamageElements |= H.SecondaryElement`.
   2. If `H.StatusEffectBase`: `R.StatusEffectFinal = H.StatusEffectBase`.
   3. **Aqua on ice** (`sub_3007460`). If all of these hold, call `_object_setPanelType(Hx, Hy, 2)` and set **`H.StatusEffectFinal = 0x50`** (freeze):
      - `R.PrimaryElement == 2`;
      - `H.Self & 0x0C000000` (H is a body);
      - `!(R.Self & 0x0C000000)`;
      - `H+0x28` (invulnerable timer) `== 0`;
      - `!(R.FFC & 1)`;
      - the panel type at `(H.PanelX, H.PanelY)` is 7.

      This runs in the *attack-reacts-to-body* direction and writes the body's field. Which direction runs first depends on who removes. As a result a later `StatusEffectFinal = attack.StatusEffectBase` can overwrite the 0x50, and a guard prevents the freeze only if the guard direction ran first.
   4. **Counter.** Let `c = H+0x07`.
      - If `R.CounterTimer && (c & 0x7F) && !(c & 0x80)`: `R.FFC |= 0x40`.
      - If `!(c & 0x80) && (c & 0x7F)`: `R+0x90 = R.CounterTimer ? 0x8000 : R+0x90 + (c & 0x7F)`.
      - `R+0x8E += c & 0x7F`, **except on a counter hit** (the first branch of the line above): the routine has
        just loaded 0x8000 into the register it masks with 0x7F here, so it adds 0. A counter hit doesn't wear the
        mood of the navi it lands on: a navi in Full Synchro that is countered keeps it (and its aura) through
        the paralysis, until anger takes it 120 stunned ticks in (`sub_80142DC`). **Verified** on the lab's
        `chips/0x13e-batcan4/counter` (side 1's MiniBomb counters side 0 in Full Synchro).
   5. If `H.Self & 0x100`: `R+0x92 += 1`.
   6. `R.HitModifierFinal |= H.HitModifierBase`.
   7. If `H.Bugs & 0xFF`: `u16 R+0xA4 = H.Bugs`.
   8. **Multiplier.**
      - `m = 1 + W1 + W2`.
        - `W1 = byte_3007444[R.elem*5 + H.elem]`: 1 for (heat←aqua), (aqua←elec), (elec←wood), (wood←heat), else 0. The table is 28 bytes; a receiver element of 5 would read into code.
        - `W2 = 1` if `R.SecondaryElementWeakness & H.SecondaryElement`, or if `R.SecondaryElementWeakness == 0x80` and `H.Self & 0x2000`; else 0.
      - `R.DamageMultiplier = m − 1`.
      - `m += 1` and **thaw now** (`sub_801A29A` on R's parent: clear FROZEN in F1/F2, `+0x2A = 0`) if `R.F1 & 0x10000` and (`R.FFC & 2` — any break-bit hitter so far, including H — or `H.SecondaryElement == 0x10`).
      - `m += 1` if `R.F1 & 0x80000000` (bubbled) and `H.elem == 3`.
      - `R.ExclamationIndicator = m − 1`.
      - The bonuses **add**, they do not multiply.
   9. If `H.elem == 3`: `R+0x78 += H.SelfDamage`.
   10. `R.ElementDamage[H.elem] += H.SelfDamage * m` (index `0x82 + 2*elem`).
   11. **Heat on grass.** If `H.elem == 1` and the panel type at `(R.PanelX, R.PanelY)` (R's *collision* panel) is 6: `R+0x82 += H.SelfDamage`. This lands in the null slot and is unmultiplied. If R's panel is off-field, the code reads the byte at address 2 (BIOS open bus); treat that as "not grass".
   12. `R+0xA0 += H+0x64`.

**`sub_3007692`** (0x03007692), the raw channel, runs right after, for the same pair:
- Same dimming gate.
- Return if `H.F1 & 0x20 && !(R.Self & 0x80)`, or if `R.F1 & 0x20 && !(H.Self & 0x80)`.
- Then:
  - `R+0x6C |= H.Self`;
  - `R+0x77 |= H.SecondaryElement`;
  - `R+0x94+2*H.elem += H.SelfDamage`;
  - heat on grass: `R+0x94 += H.SelfDamage`.
- It ignores invisibility, flashing, invulnerability, guard and air/ground, so barriers see hits that were rejected above.

### 3.9 Hit sparks (hitter side) and RNG

The collision kernels use **no RNG**.

`object_spawnCollisionEffect` (0x0801A0D4), usually right after a hitter's remove, spawns spark `HitEffect` if all hold:
- `FFC & 0x3F800000` (it touched a body or object);
- `!(FFC & 1)` (not guarded);
- `HitEffect != 0xFF`.

It calls `AddRandomVarianceToTwoCoords(0xF, X, Y, Z)` (0x0801BDDE), which makes **exactly one `GetRNG2`** call (v): `X += ((v & m) − (m>>1)) << 16`, `Z += (((v>>16) & m) − (m>>1)) << 16`. `sub_801A100` is the same with panel-snapped coordinates.

Whether the RNG call falls on the damage tick or the next depends on list order. In machgun this path never reached the RNG (HitEffect 0xFF).

---

## 4. Damage and status consumption (navi)

### 4.1 Order inside the navi update

Navi update: `sub_80EA460` → state 4 `sub_80EA484`.

1. `sub_8012E74` (buster charge).
2. `sub_8013DA0` (NaviCust; RNG2 every 60 ticks when `ns[0x24] && ns[0x21]`).
3. **Stage A: `sub_801AC6C`**.
4. **Stage B: `sub_801AF44`** (plus the action state machine).
5. The AI/input routine.
6. `sub_80107D4`, `sub_80139C4`, `sub_80100EC`.
7. `object_presentCollisionData` (unless paused).

**Stage A** (0x0801AC6C):
1. If `!(battle_getFlags() & 1)`, return; nothing, not even the remove, runs.
2. `object_removeCollisionData`.
3. Return if the battle is over or `F1 & 0x100` (DEAD).
4. Then, in order:

| # | Function | Purpose |
|---|---|---|
| 1 | `sub_801A802` | Barrier (§4.2) |
| 2 | `sub_801A186` | Poison / grass (§2.12) |
| 3 | `sub_801A36A` | Ice / road triggers (§2.11) |
| 4 | `sub_8010230` | HP-bug drain (§4.6) |
| 5 | `sub_802CFF8` | If `+0x76 & 0x40` (cursor), clear the alliance's trap slot |
| 6 | `sub_802CEF4` | Anti-damage traps (§4.3) |
| 7 | `sub_801A6B4` | Bug codes 0xF4/0xF7 |
| 8 | `sub_801A720` | Bug code 0xF6 |
| 9 | `sub_80139F6` | +0xA4 → NaviStats; ends with `sub_800FF5E` |
| 10 | `sub_801AEB0` | Hit modifiers (§4.7) |
| 11 | `sub_800EB26` | Counter → paralyze (§4.9) |
| 12 | `sub_8013F1E` | NaviCust on-hit bug |
| 13 | `sub_801A554` | Status (§4.8). Skipped if `ns[0x29]==7`, Transformation `ns[0x2C] ∈ {7, 0x13}`, or `ns[0x52] != 0` |
| 14 | `sub_801A2CC` | If `FFC & 0x10`: `obj.Chip = 0xFFFF`. For players, advance the chip-queue index (0x020349C0+0x50·alliance) if the next entry ≠ 0xFFFF; for others `ChipsHeld = 0` |
| 15 | `sub_801A324` | Drain heal (§4.6) |
| 16 | **`object_calculateFinalDamage1`** | §4.4 |
| 17 | `sub_801A420` | `CounterTimer -= 1` if nonzero |
| 18 | `sub_80143FC` | `ai+0x4C` counts ticks spent flinching or paralyzed |
| 19 | `sub_80142DC` | Anger trigger |
| 20 | `sub_8010198` | `if cd+0x26 && FFC: cd+0x26 = 0` |
| 21 | `sub_801A648` | If not paused, `cd+0x24`, `FFC & 4` and `!(FFC & 0x1000)`: `cd+0x24 = 0` (a piercing hit cancels the flash) |
| 22 | `object_spawnHiteffect` | Guard spark (§5) |

**Stage B** (0x0801AF44):
1. If paused and CurAction ≠ 0, skip to L142.
2. `sub_801A42E` ("!!" effect when +0x74 and FinalDamage are nonzero).
3. `sub_801A4A6` (skull effect).
4. `sub_801A45C` (counter bookkeeping, §4.9).
5. `sub_801A506`: if `+0x75 && FinalDamage`, `ai+0x44 |= 0x80000000` (weakness hit; ends crosses).
6. **`applyDamageToPlayer_801ba12`** (§4.5).
7. `sub_801BADE`: if bubbled (CurAction 7) and FinalDamage: pop the bubble (`sub_801A2B0`).
8. Then:
   - If DEAD, go to L142.
   - If `F2 & 1` (death pending): clear it, `F1 |= 0x100`, CurAction 2, go to L142.
   - `ai+0x48` special states and `ai+0x44` requests (crosses, etc.).
   - **If dimmed, go to L142.**
   - Knockback (`F2 & 0x100`) → CurAction 5. DRAG → CurAction 5.
   - Slide driver (§2.11).
   - Flinch (`F2 & 4`).
   - L126: `sub_801A5EE` (flash), `sub_800E730` (status timers), `sub_8010162`, `sub_8014326` (anger), `sub_8014498`, `sub_802E1D8`.
9. L142 is visuals. Then, if not dead: paused → `sub_8017BC0`; dimming → `sub_8017AB4` (hit shake, §5); else the action dispatcher `sub_801B9E6`.

Navi actions:

| CurAction | Meaning |
|---|---|
| 0, 1 | — |
| 2 | Death |
| 3 | Flinch |
| 4 | Paralyzed |
| 5 | Knockback / drag |
| 6 | Frozen |
| 7 | Bubbled |
| 8 | Idle |
| ≥0x10 | Moves and chips (0x10 = step) |

### 4.2 Barriers and auras (`sub_801A802`, 0x0801A802)

`sub_801A7CC(type)` initializes from tables 0x08020B2C and 0x08020B8C:

| Type | HP (+0x16) | Threshold (+0x17) | Timer (+0x1A) | Weak element (+0x14) |
|---|---|---|---|---|
| 1..7 | 10 / 30 / 50 / 80 / 100 / 150 / 200 | 0 | ∞ (0xFFFF) | – |
| 8 (regenerating bubble-type) | 1 | 0 | ∞ | – |
| 9 (aura) | 1 | 200 | 3000 | – |
| 0xA (self-regenerating) | 200 | 0 | 0 | – |
| 0xB / 0xC / 0xD / 0xE | 1 | 100 | ∞ | aqua / elec / wood / heat |
| 0xF | 1 | 100 | ∞ | – |

Type names are inferred.

```
if paused: return;  b = cd[6]; if b == 0: return
if b in (8, 0xA) and u16 cd[0x16] == 0: goto L860
if (cd[0x77] & 0x20) == 0 && (cd[0x6C] & 0xA20) == 0: goto L860        // no "pop" hit
b = 0x10; cd[6] = 0x10; u16 cd[0x16] = 0; cd[0x15] = cd[0x0F]          // popped
L860:
 if b == 8:   if u16 cd[0x16] != 0: goto L8E0
              if dimmed or CurAction in (6,7): return
              u16 cd[0x1A] += 1; if >= 0xF0: u16 cd[0x16] = 1; return          // regrow after 240 ticks
 if b == 0xA: r1 = u16 cd[0x16]
              if r1 == 0: if dimmed: return; cd[0x1B] += 1; if cd[0x1B] >= 0x78: u16 cd[0x16] = 0xC8; return
              if r1 < 0xC8: r0 = cd[0x1A] + 1; if r0 >= 6: { u16 cd[0x16] = r1 + 1; r0 = 0 }
              else: r0 = <stale register: u32 cd[0x6C]>
              cd[0x1A] = (u8) r0; goto L8E0
 if cd[6] != 0x10 && dimmed: goto L8E0
 t = u16 cd[0x1A]; if t != 0xFFFF: { t -= 1; u16 cd[0x1A] = t; if (s32) t <= 0: cd[6] = 0 }
L8E0:
 if b == 8 && u16 cd[0x9A] != 0: { u16 cd[0x88] += u16 cd[0x78]; goto L90A }   // elec pops it; elec damage doubled
 e = cd[0x14]; if e && u16 cd[0x94 + 2e]: goto L90A                             // weak element passes through
 s = cd94 + cd96 + cd98 + cd9A + cd9C            // raw channel, NOT +0x9E
 if type(cd.PanelX, cd.PanelY) == 5: s = (s + 1) >> 1
 if s >= cd[0x17]:
    r = cd[0x16] - s; cd[0x16] = (u8) r
    if (s32) r <= 0: { if b not in (8, 0xA) && cd[6] != 0x10: cd[6] = 0; u16 cd[0x1A] = 0; cd[0x16] = 0 }
 // absorb everything:
 u16 cd[0x82..0x8A] = 0; cd[0x8E] = cd[0x90] = cd[0x92] = 0; cd[0xF] = cd[0x11] = 0; u16 cd[0xA4] = 0
 u16 cd[0x74] = 0; cd[0x76] = 0; FFC &= ~0x50; return
L90A: cd[6] = 0; u16 cd[0x1A] = 0; cd[0x16] = 0; return        // barrier gone, this tick's damage passes
```

Consequences:
- A barrier absorbs the whole hit even on the tick it breaks.
- `+0x8C` (element 5 and poison) is never absorbed.
- Barriers are fed by the **raw** channel, so they lose HP to hits the filters rejected (flashing, guard, invulnerable).
- A popped barrier (state 0x10) absorbs everything until its visual object clears it.
- Barrier timers and regeneration pause while dimmed.
- All barrier behaviour is code only (not in machgun).

### 4.3 Anti-damage traps (`sub_802CEF4`)

`id = sub_802CE78(alliance)` is the u16 at 0x02036720+16·alliance.

```
if ai+0x48 & 0x200000:
    if u16 cd[0x84] == 0:                                  // no heat damage
        if cd82|cd86|cd88|cd8A: { ai+0xA0+0x30 = 1; sound 0x6E }
        goto ZERO
    // otherwise fall through to the normal path
r6 = 10; r7 = id
if id in (0xBB, 0x157): TRAP
elif ai+0x48 & 0x800: r7 = 0xBB; r6 = 1; TRAP
elif id == 0xBC:
    if ai+0x44 & 0x400: ZERO
    if FFC & 0x2000 && !(FFC & 0x20000): { ai+0x44 |= 0x400; ZERO }
    return
else: return
TRAP: if ai+0x44 & 0x8200: ZERO
      if (FFC & 0xFF800000) == 0: return
      s = cd82 + cd84 + cd86 + cd88 + cd8A
      if s < r6: { if r6 == 1: ZERO else return }
      ai+0x44 |= (r7 == 0xBB ? 0x200 : 0x8000); ZERO
ZERO: u16 cd[0x82..0x8A] = 0; cd[0x8E] = cd[0x90] = cd[0x92] = 0; u32 cd[0x74] = 0; cd[0xF] = cd[0x11] = 0
      u16 cd[0xA4] = 0; u16 cd[0x1C] = cd[0x2A] = cd[0x2C] = 0; FFC &= ~0x40; F2 &= ~0x301BE
```

- ZERO also cancels any running paralysis, freeze and bubble, and all pending F2 requests.
- The ids look like the Anti-chips: 0xBB = "damage ≥ 10", 0xBC = sword. Names are unverified; the logic is code only.

### 4.4 Final damage (`object_calculateFinalDamage1`, 0x0800E3DE)

```
holy = type(obj.PanelX(+0x12), obj.PanelY(+0x13)) == 5     // OBJECT panel; off-field reads address 2
k = holy ? 1 : 0
sum = 0
for i in 0..4:                                             // +0x82 .. +0x8A (element 5 / +0x8C excluded)
    v = (u16 cd[0x82+2i] + ((1<<k) - 1)) >> k              // per-element round-up halving on holy
    u16 cd[0x82+2i] = v; sum += v
sum = sub_802CE10(sum)                                     // 0x0802CE10, damage-carry record R
u16 cd[0x80] = sum                                         // truncated to u16, no cap
```

**`sub_802CE10`** uses the damage-carry record `R = 0x0203CFB0 + (alliance ? 0xC : 0)` = `{u16 maxThisTick, u16 carry, obj* owner, obj* …}`:
- If `u32 R[8] == obj`: `sum += u16 R[2]`.
- Else if `sum > u16 R[0]`: `u16 R[0] = sum`.
- `sub_802CDFE`, once per non-paused tick (§1), does `u32 R <<= 16`.
- `sub_80C913C` (a T3 object) sets up the owner.
- **[verified]** At tick 2346, 0x0203CFBC became `00 00 3C 00`.

There is **no damage cap** and no "9999". The only limits are:
- SelfDamage has 11 bits (×2 with the double flag);
- u16 wrap of the accumulators and of FinalDamage.

Element weakness, freeze-break, elec-on-bubble and heat-on-grass are already folded in by the kernel (§3.8).

### 4.5 HP application (`applyDamageToPlayer_801ba12`, 0x0801BA12)

```
d = u16 cd[0x80]
if d != 0:
    sub_8010548(d)            // ai+0x20 TotalDamageTaken = min(+d, 0xFFFF)
    if HP > 1 && (F1 & 0x40000 UNDERSHIRT) && HP <= d: d = HP - 1
    object_subtractHP(d)      // 0x0800E2D8: HP = max(HP - d, 0) (signed compare)
    PlaySoundEffect(sub_800F29C(NameID) == 2 && !battle_networkInvert(alliance) ? 0x6B : 0x6D)
    if HP == 0: goto DEATH    // +0x8C is NOT subtracted in this case
    sprite_forceWhitePalette
object_subtractHP(u16 cd[0x8C])       // always, even 0; element-5 / poison; no Undershirt
if HP != 0: goto END
DEATH: if sub_802DD2A(): ai+0x44 |= 0x08000000        // (ns[0x29] && ai+0x48 & 0x4000) → action 0x4C instead
       else F2 |= 1                                   // death pending → stage B sets DEAD, CurAction 2 this tick
END: sub_801A200()                                    // counter/Full Synchro + mood (§4.9)
```

- Stage B runs even after the battle is over or the navi is dead, so +0x8C can still be subtracted.
- **Death** (CurAction 2, `sub_80173F4`/`sub_801741C`): clears the collision Region, zeroes +0x48/+0x4C, `sub_80101C4`, and clears the barrier.

### 4.6 Other HP changes

| Routine | Rule |
|---|---|
| `sub_801A186` poison / grass | §2.12 |
| `sub_8010230` HP bug (stage A) | If not dimmed or paused and HP > 1: `p = byte_80102A4[ns[0x18]]` = {0,40,35,30,25,20,15,10}. If p == 0: `ai+9 = 0`. Else `ai+9 += 1`; at ≥ p: HP −1 and reset. **[verified-soundmod]** |
| `sub_801A324` drain heal (stage A) | `opponentAI+0x10 += u16 cd[0x92]` (hits I took from Self bit **0x100** hitters). Then `n = ownAI+0x10; ownAI+0x10 = 0; h = (MaxHP / 10) * n`. If h: `object_addHP(h)`, spawn T4/6, sound 0x8A. The attacker heals during its next stage A. |
| `sub_8014498` (stage B) | While `ai+0x36 != 0`: HP −1 per tick, never to 0. |
| `sub_80102AC` (tick level) | NaviCust drain while the player is in the custom screen (see `battle-flow.md`). |

### 4.7 Hit modifiers (`sub_801AEB0`, 0x0801AEB0)

Let `h = HitModifierFinal`.

| Bits of h | Effect |
|---|---|
| 0x01 | **Flinch request:** `F2 |= 4`, unless `F1 & 0x220000` (SUPERARMOR or ANGER) |
| 0x02 | **Flash request:** `F2 |= 2` (also with SuperArmor) |
| any of 0x3C, with 0x40 | **Knockback:** `F2 |= 0x100`; `F2 &= ~4`; SlideType = 1 (ignores SuperArmor) |
| any of 0x3C, without 0x40 | **Push:** unless `F1 & 0x100040` (DRAG or moving): `F2 |= 0x10`, SlideType = 1 |

**Push direction** (`sub_800E548`): take the lowest set bit among 0x04/0x08/0x10/0x20, and `dx ×= 1 − 2·(receiver alliance)`.

| Bit | Without 0x80 | With 0x80 |
|---|---|---|
| 0x04 | (+1, 0) × 6 tiles (toward the receiver's front) | (0, −1) × 6 |
| 0x08 | (−1, 0) × 6 (to the back) | (0, +1) × 6 |
| 0x10 | +1 × 1 | (0, −1) × 1 |
| 0x20 | −1 × 1 | (0, +1) × 1 |

The slide stops early at invalid panels (`sub_800E5AC`).

**Stage B flinch** (`F2 & 4`):
1. Clear the bit; `sub_801011A`.
2. `a = (CurAction == 4)`. If `!(F2 & 0x4000) && (F2 & 2)`: clear paralysis, `a = 2`.
3. `b = (CurAction == 6)`. If `F2 & 2`: unfreeze, `b = 2`.
4. If `(a|b) == 0` or `(a|b) & 2`: CurAction 3.
5. Clear `F2 & 0x4000`.

So a paralyzed or frozen navi is only flinched by a hit that also requests flash.

**Flinch action** `sub_80174FE`:
- `F1 |= 0x400400`.
- Clears paralyze, freeze and bubble, and `F1 &= ~0x100041`.
- Snaps to FuturePanel unless sliding.
- Stat `0x0203EAE0+16·alliance+3` += 1.
- Timer 23. FLINCHING stays set for **23 ticks** **[verified-soundmod]**, then CurAction 8.

**Knockback** (`F2 & 0x100` in stage B):
1. Save `obj+8` into `obj+0x5C` if it is 0; `sub_801011A`; unfreeze; unbubble.
2. Paralysis: kept if `F2 & 0x4000`; otherwise cleared if `F2 & 2`, or if CurAction ≠ 4.
3. CurAction 5 (drag, `sub_80178B6`: 10/6 px per tick, then waits 0x14 or 0x18 ticks).

### 4.8 Status effects

**`sub_801A554`**: for `v = StatusEffectFinal != 0`:
- `e = ptr[(v>>4) − 1][v & 0xF]`, where `ptr` is at 0x080209EC and entries are 8 bytes `{u32 F2bits, u16 duration, u8 cdOffset}`.
- `u16 cd[e.off] = e.dur`; `F2 |= e.bits`.
- If `0x50 <= v <= 0x55`: `F2 &= ~6` (freeze cancels flinch and flash).

| v | Status | F2 bit | Timer | Duration by low nibble 0..5 (ticks) |
|---|---|---|---|---|
| 0x1n | Paralyze | 0x8 | +0x1C | 90, 120, 150, 4, 300, 600 |
| 0x2n | Confuse | 0x80 | +0x1E | 480, 720, 960, 4, 300, 600 |
| 0x3n | Blind | 0x20 | +0x20 | 480, 720, 1200, 4, 300, 600 |
| 0x4n | Immobilize | 0x40 | +0x22 | 2, 60, 120, 4, 300, 600; 0x46 = 30 |
| 0x5n | Freeze | 0x10000 | +0x2A | 150, 150, 150, 4, 300, 600 |
| 0x6n | Bubble | 0x20000 | +0x2C | 150, 150, 150, 4, 300, 600 |

- Nibbles 6/7 alias the next group's first entries.
- 0x66/0x67 hit garbage: 0x66 writes u16 0xFFFF to +0x0A (PanelX/Y!) with no flag.
- The damage-word paralyze bit gives 0x10 (90 ticks). A counter gives 0x12 (150). Aqua-on-ice gives 0x50 (150).

**Timer engine** `sub_800E730` (stage B, skipped when paused or dimmed). `F2s` is F2 on entry; "save" means `if obj+0x5C == 0: obj+0x5C = obj+8`.

**Paralyze** (+0x1C):
- `t -= 1`. If `t ≤ 0`: `F1 &= ~0x800`; `F2 &= ~8`; +0x1C = 0.
- Else:
  - If `F2s & 8`: `F2 &= ~0x88`; save; CurAction 4; zero +0x1E/+0x2A/+0x2C.
  - `F1 &= ~0x80018000`.
  - If not PARALYZED: set it; save; CurAction 4.

**Freeze** (+0x2A):
- At 0: clear FROZEN.
- Else:
  - If `F2s & 0x10000`: `F2 &= ~0x30080`; CurAction 6; zero +0x1E/+0x1C/+0x2C.
  - `F1 &= ~0x80008000`.
  - If not FROZEN: set it; CurAction 6.
  - If `cd+0x58 == 0`: spawn the ice visual. Quirk: if it was already nonzero, the bubble and confuse decrements are **skipped** this tick.

**Bubble** (+0x2C):
- At 0: clear BUBBLED; `+0x60 = 0`.
- Else:
  - If `F2s & 0x20000`: `F2 &= ~0x20080`; CurAction 7; zero the others.
  - `F1 &= ~0x8000`.
  - If not BUBBLED: set it; CurAction 7.
  - If `+0x60 == 0`: visual. Quirk: otherwise the confuse decrement is **skipped**.

**Confuse** (+0x1E):
- At 0: `F1 &= ~0x8000`; `F2 &= ~0x80`; `+0x48 = 0`.
- Else:
  - If `F2s & 0x80`: `F2 &= ~0x30088`; zero +0x1C/+0x2A/+0x2C.
  - `F1 |= 0x8000`; `F1 &= ~0x80010800`.
  - Visual if `+0x48 == 0`.

**Immobilize** (+0x22):
- At 0: `F1 &= ~0x4000`.
- Else if `F2s & 0x40`: `F2 &= ~0x40`; `F1 |= 0x4000`.

**Blind** (+0x20):
- At 0: `F1 &= ~0x2000`; `+0x4C = 0`.
- Else if `F2s & 0x20`: `F2 &= ~0x20`; `F1 |= 0x2000`; visual.

**Invulnerable** (+0x28): `F1 |= 8` while the timer is > 0, cleared at 0.

**Mutual exclusion:**
- Paralyze removes bubble, freeze and confuse.
- Freeze removes bubble and confuse.
- Bubble removes confuse.
- Confuse removes bubble, freeze and paralyze.

**Mashing.** The paralyzed, frozen and bubbled actions (4/6/7) take an extra −1 per tick while `ai+0x24` (JoypadPressed) ≠ 0.

**Pops.** Elec on a bubble gets +1 multiplier in the kernel. A damaging hit pops the bubble (`sub_801BADE`). A break hit thaws a frozen target in the kernel (§3.8).

### 4.9 Counters, Full Synchro and mood

**Window.**
- `object_setCounterTime(v)` (0x0800E9DC) writes `cd+0x0D = v`. For player navis it only writes when `GetBattleEffects() & 8` (netbattle).
- `object_setDefaultCounterTime` (0x0800FDB6, 57 call sites) sets 16.
- Stage A decrements it (**[verified]** 16→1 over ticks 640–655).

**Detection** is in the kernel (§3.8 step 7.4): `FFC |= 0x40`, `+0x90 = 0x8000`. A hitter whose `+0x07` has bit 7 set, or whose low 7 bits are 0, never counters.

**`sub_800EB26`** (stage A): if `FFC & 0x40` and StatusEffectFinal ∉ [0x60, 0x65]:
- StatusEffectFinal = 0x12 (paralyze 150);
- `F2 |= 0x4000`;
- `F2 &= ~6` (no flinch, no flash).

**`sub_801A45C`** (stage B): if `FFC & 0x40`:
- If battle flag 0x40 is set: the attacker's custom gauge `u16[0x02036120 + 0x1D0·opp + 0x28] = min(+0x1500, 0x4000)`.
- Opponent stat `0x0203EAE0+16·opp+8` += 1 (cap 255).
- `cd+0x0D = 0`.
- If the battle is not over: the "Counter" banner (`sub_801E270`) and sound 0x86.

**`sub_801A200`** (end of HP application; skipped if the battle is over):
- **Full Synchro:** if `u16 cd+0x90 & 0x8000`, the opponent's Transformation is in {0, 0xB, 0xC}, and on both navis `ai+0x32 == 0 && ai+0x36 == 0`: `ns[opp].Mood = 0xFF` (`sub_8015BEC`).
- **Mood:** always `sub_8015C12(alliance, u16 cd+0x8E)`: if Mood ≠ 0, `Mood = max(Mood − cd+0x8E, 1)`. A counter
  hit adds nothing to cd+0x8E (§3.8), so it takes no mood.
- **[verified]** At tick 2346: 0x80 → 0x62, with the hitbox's counter byte 30.

A barrier (which clears FFC 0x50) or a trap ZERO (which clears 0x40) cancels a counter.

**Verified** by the chip lab: 109 chips' hits counter (`chips/*/counter` where the user is fast enough,
`chips/*/counter-hit` with the opponent's MiniBomb timed to the hit), with the banner (HUD task bit 0x100, 50
ticks) and the 150-tick paralysis. BblStar's bubble and DrilArm's drag take the paralysis's place, and an
attack's next hit ends it with a flinch (AquaNdl's second needle, EnergBom's third blast). AirRaid's shots, JustCone,
DarkInvs's strikes and Anubis's poison land in the window without a counter, as does every hit during a dimming
(the dimming holds the window open: the navi doesn't run). docs/engine/unverified.md has the list.

### 4.10 Invincibility (flash) and pierce

**`sub_801A5EE`** (stage B):

```
if !(battle_getFlags() & 1): return
if u16 cd[0x24] == 0 && F2 & 2: cd[0x24] = 120
F2 &= ~2
if cd[0x24]:
    t = cd[0x24] - 1; cd[0x24] = t
    if (s32) t > 0: F1 |= 0x200; return
    if F1 & 2: PlaySoundEffect(0x94)
F1 &= ~0x202
```

- FLASHING lasts **119 ticks** **[verified-soundmod]**. A new flash request while already flashing is dropped, not extended.
- While `F1 & 0x202`, the kernel rejects every hit whose Self lacks 0x4 (§3.8 step 3). So there is no damage, status, counter or modifier from multi-hits during flash; only the raw channel still accumulates.
- A piercing hit (Self & 4) lands. `sub_801A648` then zeroes `cd+0x24` (unless FFC & 0x1000), so the same tick's `sub_801A5EE` clears 0x202, or restarts a fresh 120 if that hit requests flash. **[verified-soundmod]**
- F1 bit 0x4 and its timer at `cd+0x26`: see §4.10.1 (submerged).
- **SuperArmor** only suppresses the flinch request. **Undershirt**: §4.5. **Guard**: §3.8 step 4.

### 4.10.1 Submerged (F1 0x4)

ObjectFlags1 bit 0x4 is the engine's `f1::SUBMERGED`: the object is below the surface, and only some attacks
reach it. **It never occurs in a netbattle**: its only live source is DiveMan's AI.

**Collision rule** (`sub_3007218`, the pair test, §3.8). A pair is skipped when one side is submerged and the
other side's Self collision type has neither bit 0x8 nor bit 0x1000:

- receiver submerged (`F1 & 4`) and the hitter's `Self & 0x1008 == 0`: no hit;
- hitter submerged and the receiver's `Self & 0x1008 == 0`: no hit.

So it works both ways. Attacks whose collision type carries 0x8 or 0x1000 still land; many do, e.g. type 0x2C
(GunDelSol's hitbox, Self 0x8000408C). Body contact and types without those bits pass through. Of the 89
collision types, 30 carry one of the bits: 0x0A, 0x15, 0x17, 0x19, 0x1A, 0x1C, 0x1D, 0x20–0x22, 0x24, 0x26, 0x2B–0x2D,
0x2F–0x33 (0x31, 0x32 by bit 0x1000), 0x37, 0x3A, 0x3C, 0x3F, 0x45, 0x4B, 0x4E, 0x4F, 0x52 (bit 0x1000) and 0x55.
Which attacks use which types is catalogued per chip as the chips are ported.

**The timed form** (`cd+0x26`, the engine's `timer::SUBMERGED`), run by the navi's status stage each tick
(`sub_8010162`):

- If `+0x26 != 0xFFFF`: decrement. Below 0: clear bit 0x4 and stop. At exactly 0: sound 0x94. (0xFFFF means
  indefinitely.)
- Then set bit 0x4, or clear it while the navi is using an action (`F1 & 0x400000`, USING_ACTION): a submerged
  navi is solid while it attacks.
- Any registered hit (`FlagsFromCollision != 0`) zeroes the timer (`sub_8010198`, stage A), so the next status
  stage ends it.
- `sub_80101C4` zeroes the timer and clears the bit. It's called by the hit-reaction actions (`sub_80165F8`,
  `sub_8016B02`…`sub_8017900`: flinch, paralysis, freeze, bubble, drag, deletion), by `sub_810E4F0` and `sub_810E928`,
  and by actor #0x5D variant 1 before it starts the timer.

**What sets it:**

1. **DiveMan's AI** sets bit 0x4 directly while its AI state has bit 0x20 (`sub_80FDEFC`, entry 13 of the navi
   routine tables at `off_80F25A0`; navi 13's sprite is category 8, index 0x0D, whose animations include the
   dive with only the periscope showing). It's the "dove underwater" state. Navi AI isn't ported: it isn't a netbattle's (completeness.md §5).
2. **`sub_80101AE`** starts the timed form: it stores the duration in `+0x26`, sets bit 0x4 and clears the
   object's VISIBLE flag. Its only caller is **variant 1 of actor object #0x5D** (`sub_80C49E4`, chosen by
   Param1): after a 30-tick white flash it runs the dimming return (`sub_80E13DC`), then makes its owner
   submerged for **480 ticks** (0xF0 × 2) with sound 0x93.
3. **That variant is never spawned.** Actor #0x5D has one spawner (`sub_80C4AEC`). Its one caller, the effect
   of BugFix's dimming controller (chip 0xB0, effect #0x3B, `sub_80E4954`), always passes Param1 0: the
   white flash plus the bug fix (`sub_80C4958`/`sub_80C49A4`), which doesn't submerge. Variant 2
   (`sub_80C4A52`) is unused as well. This is from a static search for spawns of index 0x5D; a spawn through a
   computed index would have escaped it. **[unverified]**

Invisibl (chip 0xB1) is not this: it sets INVISIBLE (F1 0x2) and the 360-tick flash timer (§4.10, chips.md
§3.6.6).

### 4.11 Bugs and NaviCust hooks (summary)

`+0xA4` holds the last hitter's Bugs (low byte code, high byte amount).

| Code | Handler | Effect |
|---|---|---|
| 0xF4 | `sub_801A6E8` → `sub_801A6B4` | HP-bug level `ns[0x18] = min(+1, 7)` |
| 0xF7 | same | Same, but only if the pre-damage HP (decimal) contains a digit 4; otherwise +0xA4 is cleared |
| 0xF6 | `sub_801A720` | `ns[0x18]` and `ns[0x19]` each +2 (cap 7); paralyze 150; blind 1200 unless NameID ∈ [0x173, 0x17E] |
| 0xF8 | `sub_80139F6` | Uninstall (`sub_80140EE`) |
| 0x18 / 0x19 | `sub_80139F6` | `ns[c] = min(ns[c] + amt, 7)` |
| 0x54 | `sub_80139F6` | `ns[0x54]` (byte) = `hword(ns+0x54) + amt` |
| 0xFF | `sub_80139F6` | `ns[0x14] = 4` |
| 0xFE | `sub_80139F6` | `ns[0x12] = 4`, `ns[0x13] = 4` |
| 0xFB | `sub_80139F6` | `sub_8014080` |
| 0xFA | `sub_80139F6` | `ns[0x12] = 4`, `ns[0x13] = 2` |
| 0xF9 | `sub_80139F6` | `ns[0x12] = 4`, `ns[0x13] = 1` |
| 0xF5 | `sub_80139F6` | `ns[0x12] = 3`, `ns[0x13] = 1` |
| other < 0x64 | `sub_80139F6` | `ns[c] = amt` |

The write paths (0x18, 0x19, 0x54, 0xFF, other) then call `sub_801393A` and `sub_801469C`.

**`sub_8013F1E`** (NaviCust on-hit status):
- Needs `F2 & 0x104`, `FFC != 0`, and a nonzero sum of three **unaligned** u32 loads at +0x82/+0x86/+0x8A. The ARM rotation makes that sum `(pd0+pd2+pd4) + ((cd80+pd1+pd3) << 16)` mod 2³².
- Latched by `obj+0x18` / `ai+0x1C`.
- Effect by `ns[0x16]`: 1 → blind 0x32; 2 → confuse 0x22; 3 → `ns[0x18] += 1`.

**Anger** (`sub_80142DC`): triggers when all of these hold:
- BattleMode ≠ 1;
- `ns[0x29] == 0`;
- Transformation == 0;
- not already angry;
- `ai+0x4C ≥ 120` or `FinalDamage ≥ 300`.

It sets `F2 |= 0x200`. `sub_8014326` then sets ANGER (F1 0x200000) for 600 ticks.

### 4.12 Dimming

During dimming:
- Only objects with header flag 0x10 run.
- The panel tick and the BS+0x0E/+0x16 counters stop.
- Collision still resolves between objects that have Self 0x10000 or F1 0x01000000 (§3.8 step 1).
- For the navi:
  - HP is still applied immediately (stage B runs up to `applyDamage`).
  - Stage B stops before flinch, status and flash processing.
  - HitModifierFinal/Unk_03 are **not** cleared by present, so `sub_801AEB0` re-asserts the flinch/flash requests every tick.
  - All pending requests fire on the first tick after the dimming.
  - **[verified-soundmod]** tick 3596: 100 heat damage while dimmed; flinch and flash began at 3684.

---

## 5. RNG inventory for these subsystems

| Call site | Stream | Count and condition |
|---|---|---|
| Panel init / tick / area return / crack / break / poison / set type / set alliance | — | none |
| Collision kernels (IWRAM), final damage, HP application, statuses, flash, slides, poison, grass, holy | — | none |
| `object_spawnCollisionEffect` 0x0801A0D4 / `sub_801A100` (hitter side) | RNG2 | 1× `GetRNG2` per call when `FFC & 0x3F800000`, not `FFC & 1`, and `HitEffect != 0xFF` |
| `object_spawnHiteffect` 0x0800EB9E (navi, end of stage A) | RNG2 | 1× `GetRNG2` when not paused and `FFC & 0x20000` (it blocked a hit); spark at Z+0x100000, sound 0x6E |
| `sub_8017AB4` (navi stage B, dimming branch) | RNG2 | On a tick with FinalDamage ≠ 0: shake counter = 30. Then 1× `GetRNG2` (`AddRandomVarianceToTwoCoords(3,…)`) per tick while the counter runs, so 30 calls, restarted by each new damaging tick. **[verified-soundmod]** |
| `sub_8013CC4` (step "panel trail", NaviCust) | RNG2 | 1× `GetPositiveSignedRNG2` at step F+3 if `ai.ActorType == 2 && ns[0x13] != 0`; effect if `(r & 7) <= ns[0x13] − 1`: `ns[0x12]` 1 → break (dup2), 3 → crack (Dup1), else set type (sound table `byte_8013D44`). Skips holes and voids. |
| `sub_8013FAE` (auto-repeat step) | RNG2 | 1× `GetRNG2` at step end (mode ≠ 1) if `ns[0x11] != 0` |
| `sub_8013DA0` (NaviCust) | RNG2 | Every 60 ticks when `ns[0x24] && ns[0x21]` |
| `object_getRandomPanelFromCurrentColumn` 0x0800CED0, `_GetRandomRelativePanelFiltered` 0x08015E58 | RNG2 | 1× (`r % count`) when ≥1 candidate; chip/AI helpers |
| Camera shake (`sub_802FFF4`) | RNG1 | Visual; see `battle-flow.md` |

**[verified]** In the machgun match the only RNG2 advances are the two `GetPositiveSignedRNG2` calls at each round start (`sub_80AA88C` via the mode handler, ticks 72 and 1224). None come from these subsystems. `AddRandomVarianceToTwoCoords` is never called.

---

## 6. Verification against the machgun trace

HP runs in the trace (`hp` = obj+0x24):

| Navi | Ticks | Change per tick | HP |
|---|---|---|---|
| A1 | 647–766 | −4 | 1000→520 |
| A1 | 788–907 | −4 | 520→40 |
| A1 | 929–938 | −4 | 40→0 |
| A1 | 2016–2135 | −4 | 1000→520 (round 2) |
| A1 | 2158–2277 | −4 | 520→40 (round 2) |
| A1 | 2346 | 40→0 | F1 gains DEAD (status 0x02000100) |

**Tick 646 → 647** (element-5 drain). a memory watch on slot 2 (0x02038640) in tick 646 shows, inside one update of a T3/3 hitbox (`sub_80C52B0` → `sub_80C52D0`):
1. `object_createCollisionData` takes slot 2 (bit 0x20000000).
2. `object_setupCollisionData(self 0x2C → 0x8000408C, target 5 → 0x15800000)`: SelfDamage 4, element 5, `+0x07` = 0, Region 4 (a vertical 3-panel column) at (5,2), HitEffect 0xFF.
3. Present. The `Unk_54 = r1` quirk is visible: it is first written 4, then 0.
4. Remove → `sub_30075FC` on (5,2) finds navi A1 (slot 0, registered earlier this tick):
   - `sub_3007218(hitbox ← navi)`: the hitbox's FFC gets 0x04210080 and HitModifierFinal gets 3 (navi HitModifierBase), `+0x82` += 10 (navi SelfDamage 10), and the raw channel likewise.
   - `sub_3007218(navi ← hitbox)`: the navi's FFC gets 0x8000408C and `+0x7C` gets 0x20000000; `ElementDamage[5]` (`+0x8C`) = 4 and `+0x9E` = 4.
5. `object_clearCollisionRegion`, `object_freeCollisionData`.

Pre-tick-647 dump of the navi's CollisionData: `70=8000408c 7c=20000000 82=[0,0,0,0,0,4]`. The registration masks are only 0x40000000 at (3,2) and 0x80000000 at (5,2), so the freed slot left no stale bits.

In tick 647 the navi's stage A computes FinalDamage 0 (the +0x8C slot is excluded). Stage B's second `object_subtractHP(u16 cd+0x8C = 4)` then writes HP 0x3E4 (996), via `object_subtractHP ← applyDamageToPlayer_801ba12 ← sub_801AF44 ← sub_80EA484`. There is no flinch, no flash (the hitbox's HitModifierBase is 0) and no mood change (counter byte 0). The flash filter would not have stopped it anyway, because Self 0x8C includes the pierce bit 0x4.

**Tick 2346** (the lethal hit):
- Pre-state of the navi's CollisionData: `FFC=0x80000080`, `HitModifierFinal=1`, `+0x82=60`, `+0x94=60`, `+0x8E=30`, `+0x90=0` (the hitter's `+0x07` has bit 7 set, so no counter), HP 40.
- The hitter was a Self idx 4 / Target idx 5 hitbox with damage 60, element 0, HitModifierBase 1.
- Stage A: `sub_801AEB0` sets `F2=4`; FinalDamage = 60 (not holy).
- Stage B: HP 40 → 0 (clamped; +0x8C not applied), `F2 |= 1`; the same tick sets DEAD and CurAction 2.
- Also: `ai+0x20` becomes 60, Mood 0x80 → 0x62, and the carry record at 0x0203CFBC receives 0x3C.

**Panels:**
- Frame-72 Flags match §2.4 for layout 0xE3 / colpat 0x38.
- At tick 2346 the alliance-0 navi (FloatShoes, Self 0x08510080) stands on (4,2): Flags 0x08510032, alliance 1, i.e. stolen area. Its reserved home panel (3,2) is 0x00010092, and the voids read 0x00018000 / 0x00018020.
- **[verified]** in the trace: rows reflect only the upper halves of the currently registered slots.

---

## 7. Quirks checklist (all must be reproduced)

1. The panel masks are never cleared per tick; `free` doesn't clear bits; stale bits are tested and feed panel Flags (§3.1, §3.6).
2. `PairTested` (+0x68) dedup across objects and across the tick boundary (§3.7).
3. The present path's Region-0x80 panel refresh uses the wrong arguments; `sub_300777C` clobbers r8 (§3.6).
4. `object_presentCollisionData` stores the caller's r1 into +0x54; while dimmed, HitModifierFinal/Unk_03 persist (§3.6, §4.12).
5. Aqua-on-ice writes the *body's* StatusEffectFinal from the other direction and is order-dependent (§3.8).
6. Frozen-break and bubble-elec bonuses add to the weakness count; the frozen check sees break bits from earlier hitters this tick (§3.8).
7. Heat-on-grass damage goes into the null slot, unmultiplied, using the receiver's *collision* panel. Holy halving uses the *object* panel; barriers use the collision panel (§3.8, §4.2, §4.4).
8. Element-5 / poison damage (+0x8C) bypasses FinalDamage, barriers, holy, Undershirt and the death-tick path (§4.5).
9. Barriers are fed by the raw channel, which includes flashing/guarded/invulnerable hits (§4.2).
10. `sub_8019F44`'s Bugs high byte is the leftover `targetIdx*8 + alliance*4` (§3.5).
11. Hole timer off-by-one between chip breaks and step-off breaks; the initial roads expire after 0x708 ticks (§2.6.2).
12. A cracked panel with a reservation never breaks; FloatShoes navis never break cracked panels (§2.6.2).
13. Status-table garbage for 0x66/0x67 (writes +0x0A) and aliasing for nibbles ≥ 6 (§4.8).
14. Freeze/bubble visual-pointer quirks skip other timers' decrements (§4.8).
15. `sub_8013F1E` unaligned loads (§4.11).
16. Off-field lookups read BIOS open bus at address 2 (heat-on-grass, holy, `sub_801A36A`); `_object_setPanelType` and `object_setPanelAlliance` have no null checks.

---

## 8. Uncertainties

1. **Names inferred, not proven:**
   - Volcano (type 8). Its behaviour is exact: periodic T3/7 eruptions, 50 damage.
   - Upper collision bits 0x02000000/0x01000000 ("other bodies"), 0x00080000 ("blocker") and 0x00008000 ("ground-only").
   - Low Self bits 0x1008 / 0x3000 and the ObjectFlags1 bits they gate (0x4, 0x00800000, 0x01000000, 0x08000000). The header names 0x08000000 UNAFFECTED_BY_POISON; the code treats it as "fully intangible".
   - Barrier type names, the Anti-chip ids (0xBB / 0xBC / 0x157), and NaviStats bytes 0x11–0x19, 0x21, 0x24, 0x29, 0x31, 0x52, 0x54.
2. **Code only when this was written, recorded by the chip lab since** (the engine matches):
   - counter hits (`chips/*/counter-hit`, §4.9), guard sparks (`chips/*/guard`), the UnderShirt clamp
     (`chips/0x0b9-uninstll/folded-undershirt`);
   - ice slides, poison and the grass heal (a navi without shoes on every stage: `forms/*/bare-stage-*`,
     `forms/falzar/cross-tomahawk-grass`);
   - barriers, traps and bug codes (their chips' scenarios);
   - the timer-driven area return (`chips/0x0a3-areagrab/returns`), aqua-on-ice freeze
     (`forms/falzar/base-ice-widesht`).

   Not matched to a recording here: holy halving, the grass heal's rate at HP ≤ 9, the filtered whole-field
   regions (Region & 0x80) and the air/ground rule. docs/engine/unverified.md is the survey.
3. The push directions (bits 0x04/0x08) are derived from `sub_800E548` and the receiver's alliance; the chip-to-bit mapping (Wind, Fan, AirShot…) is not checked.
4. The drain-heal counter (+0x92) uses hitter Self bit **0x100** (from `lsr #9` → carry). One sub-report claimed 0x200; the Rust translation confirms 0x100.
5. Whether anything outside rendering reads panel +0x06 / +0x07 / +0x10: none found.
6. `sub_802CE10` damage-carry semantics ("owner" gets last tick's max added). Which chip installs it (`sub_80C913C`) was not identified.
