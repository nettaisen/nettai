# Battle objects and the PvP player actor — engine spec (BN6 Falzar US, BR6E)

Status: reverse-engineered from the original's code, read statically and run under emulation on the machgun battle (the machgun trace, two rounds: setup lines at frames 72 and 1224). Everything marked **UNCERTAIN** was not confirmed at run time.

Related specs in this folder: `battle-flow.md` (tick structure, battle/mode state machines, RNG generators, link inputs, battle-wide state), `chips.md` and `chip-table.md` (chip data, hand handling, attack routines; includes a frame-exact GunDelS3 walk-through of the same trace). The chip used in the reference match is **GunDelS3** (chip 0x11, player action 0x37 `sub_80EDAE0`); "machgun" is only the Tango room name.

## 0. Conventions

- "Tick" = one call of `battle_8007800` (0x08007800) = one trace line (`frame`). All durations are in ticks (60 Hz).
- Addresses: ROM functions as `name` (0x08xxxxxx); RAM as absolute GBA addresses.
- `obj` = the object pointer (`r5` in all handlers). Field names follow the original `BattleObject` struct (`obj.CurState` = obj+0x08, etc.). `AIData` = `*(obj+0x58)`, `cd` = `*(obj+0x54)` (collision data). `AIAttackVars` = AIData+0xA0.
- State / action / phase numbers are the raw byte values stored in the object. CurState (0/4/8) and CurPhase are byte offsets into jump tables. CurAction is a byte offset for simple objects but an **index** for players/navis/viruses (player action 8 = 9th entry of its action table; see section 2).
- "Alliance 0" = left side (the P1/host console's player in the reference replay; its inputs are `input[0]` in the trace), "alliance 1" = right side.
- 16.16 means signed fixed point (integer part in the high half).
- The GBA silently ignores stores to 0x00000000-0x00003FFF (BIOS). Several routines store through a null pointer (for example `sub_80E1662`, `T1#5` owner increments); **the Rust engine must treat such stores as no-ops** rather than panicking.
- Several spawn call sites pass whatever happens to be in r1-r4 as X/Y/Z/Params. These "register garbage" values are observable in the trace; they are listed where known.

## Contents

1. Memory map
2. The battle object structure (header flags)
3. Pools and spawning (slot choice, list insertion, freeing, destroy convention)
4. `RunBattleObjectLogic`, tick order, pause and dimming
5. State-machine convention and action helpers
6. AIData pool
7. CollisionData pool and the per-tick collision protocol
8. Common helpers
9. BattleState fields
10. Auxiliary objects: T1#5, T4#0/2/8/0x48 (+ round-2 kinds)
11. Sprite / animation state
12. The player actor (spawn, init, per-tick pipeline, action table, idle/move/input, buster/charge/chip, damage/status/death)
13. RNG uses
14. Trace verification summary
15. Open questions / uncertainties

## 1. Memory map (all EWRAM unless noted)

| What | Address | Size / layout | Source |
|---|---|---|---|
| T1 pool (players, navis, viruses, big chip objects) | nodes 0x0203A9A0, objects 0x0203A9B0 | 32 slots x 0xD8 (0x10 list node + 0xC8 object) | `eT1BattleObject0`, `NUM_T1_BATTLE_OBJECTS`=0x20 (constants/constants.inc:24) |
| T3 pool (short-lived attack/collision-region objects) | nodes 0x0203CFD0, objects 0x0203CFE0 | 32 x 0xD8 | `eT3BattleObject0` |
| T4 pool (effects, helpers, shadows...) | nodes 0x02036860, objects 0x02036870 | 32 x 0xC8 (0x10 node + 0xB8 object) | `eT4BattleObject0` |
| T1 / T3 / T4 allocation bitfields | 0x02034000 / 0x02034F54 / 0x02036710 | u32 each; slot i <-> bit (31-i) | `ActiveObjectBitfieldPointers` |
| Update list start node (head) | 0x02009380 | node {Prev,Next,pad8}; `Next` = first object node | `eBattleObjectsLinkedListStart` |
| Update list sentinel (tail) | 0x02009AB0 | node; `Prev` = last object node | `eBattleObjectsLinkedListSentinel` |
| "current object" node pointer | 0x0200AF70 (u32) | node ptr of the object being updated, 0 outside the loop | `eUnkBattleObjectLinkedList` |
| AIData pool | 0x02034080 | 8 x 0x100; alloc bitfield u32 at 0x0203F6A0 | |
| CollisionData pool | 0x020384F0 (Toolkit+0x30) | 32 x 0xA8; alloc bitfield u32 at 0x02035310 | |
| Panel collision masks | 0x02034F60 | 40 x u32, index y*8+x (x 0..7, y 0..4), bit (31-k) = CollisionData slot k present on that panel | `sub_3007868`/`sub_3007880` |
| Panel data | 0x02039AE0 | 0x20 bytes per panel, address = 0x02039AE0 + ((y*8+x)<<5), valid x 1..6, y 1..3 | `_object_getPanelDataOffset` |
| BattleState | 0x02034880 | see §9 | the original `BattleState` struct |
| GameState.BattlePaused | 0x02001B8A (u8) | nonzero = battle paused | `PauseBattle` |
| Battle flags | BattleState+0x32 = 0x020348B2 (u16) | bit0 = fight active (collision on), bit2 = dimming | `battle_getFlags` |
| Per-player input records | 0x02036820 + 8*side | +2 held, +4 pressed, +6 released (u16) | written by `sub_800A0D6` |
| Battle navi stats | 0x0203CE00 + 0x64*alliance | NaviStats | `GetBattleNaviStatsAddr` |
| Per-alliance chip hand ("chip block") | 0x020349C0 + 0x50*alliance | byte0 = cursor, then u16 chip entries, 0xFFFF-terminated | `sub_8010018` |
| Per-alliance battle struct | 0x02036120 + 0x1D0*alliance | custom gauge/chip-use state etc. | `sub_802E070` |
| Render lists | T1: ptrs 0x02039A10, count u8 0x02036778; T3: 0x0203A010 / 0x0203CA7C; T4: 0x0203F750 / 0x02036830 | object pointers appended each tick | `object_800372A` |

The `Type` nibble of an object (low 4 bits of byte +2) selects the pool: 1 = T1, 3 = T3, 4 = T4 (types 0, 2, 5 are overworld objects and never appear in battle).

## 2. The battle object structure

Offsets are relative to the object pointer (`r5` in all handlers); the 16-byte list node lives at object-0x10 (`Prev` at -0x10, `Next` at -0x0C, 8 unused bytes). T1 and T3 objects are 0xC8 bytes, T4 objects 0xB8 bytes; they are identical up to 0x60, then T1/T3 have 0x2C bytes of `ExtraVars` (0x60-0x8B) + 4 spare bytes + sprite data at 0x90, and T4 has 0x1C bytes of `ExtraVars` (0x60-0x7B) + 4 spare + sprite data at 0x80.

| Off | Size | Name | Semantics / notes |
|---|---|---|---|
| 0x00 | u8 | Flags | header flags, see table below |
| 0x01 | u8 | Index | handler index within the pool's jump table |
| 0x02 | u8 | TypeAndSpriteOffset | low nibble = type (1/3/4); high nibble = sprite-data offset>>4 (0x91 for T1, 0x93 for T3, 0x84 for T4). Written once by `InitializeStructsOfObjectType`, never by spawn |
| 0x03 | u8 | ListIndex | slot number 0..31, written once by `InitializeStructsOfObjectType` |
| 0x04 | u32 | Params (Param1..4 = bytes 4..7) | spawn argument r4 |
| 0x08 | u8 | CurState | 0 init, 4 update, 8 destroy (jump-table byte offsets) |
| 0x09 | u8 | CurAction | action selector. Simple objects use it as a byte offset into their action table (0,4,8,...); actors dispatched through `sub_801B9E6` (players, navis, viruses) use it as an **index**: < 0x10 -> per-form table entry, >= 0x10 -> `JumpTable80EAC60[a-0x10]` |
| 0x0A | u8 | CurPhase | byte offset into a phase table |
| 0x0B | u8 | PhaseInitialized | 0 = phase entry code not yet run; handlers store 4 (sometimes 1) when initialised |
| 0x0C,0x0D | u8 | Unk_0c/0d | per-handler |
| 0x0E | u8 | Element | low nibble primary, high nibble secondary element (copied to collision data by `object_setupCollisionData`/`sub_8019F8C`) |
| 0x0F | u8 | SlideType | 0 none,1 wind,2 ice,3 road |
| 0x10 | u8 | CurAnim | requested animation id |
| 0x11 | u8 | CurAnimCopy | last animation id applied to the sprite; `object_setAnimation` sets CurAnim=a, CurAnimCopy=0xFF to force a reload |
| 0x12,0x13 | u8 | PanelX, PanelY | current panel (1..6, 1..3 on-field) |
| 0x14,0x15 | u8 | FuturePanelX/Y | destination panel during movement |
| 0x16 | u8 | Alliance | 0 = left/"red" side, 1 = right/"blue" side |
| 0x17 | u8 | DirectionFlip | XORed with alliance to get facing (`object_getFlip` = Alliance ^ DirectionFlip) |
| 0x18 | u8 | PreventAnim | |
| 0x19 | u8 | Unk_19 | |
| 0x1A | u8 | ChipsHeld | players: number of chips left in hand (rewritten every tick by `sub_800FDEA`) |
| 0x1B-0x1F | u8 | slide fields | NumSlideTiles, SlideDeltaX/Y, SlidePerTileTimer, SlideState |
| 0x20 | u16 | Timer | general countdown |
| 0x22 | u16 | Timer2 | |
| 0x24 | u16 | HP | |
| 0x26 | u16 | MaxHP | |
| 0x28 | u16 | NameID | players: 0x1A0 + navi stats[0x29] (MegaMan = 0x1A0); selects enemy/navi tables |
| 0x2A | u16 | Chip | players: id of next chip in hand (0xFFFF none), rewritten every tick by `sub_800FDEA` |
| 0x2C | u16 | Damage | attack power (+ flag bits 0x8000 double, 0x4000 paralyze, 0x2000 uninstall, 0x1000, 0x0800); players: 10 in netbattle (`sub_80142B0`) |
| 0x2E | u16 | StaminaDamageCounterDisabler | players: 10 (`sub_8013892`) |
| 0x30,0x32 | u16 | Unk_30/32 | |
| 0x34 | s32 | X | 16.16 fixed point, relative to field centre |
| 0x38 | s32 | Y | 16.16 |
| 0x3C | s32 | Z | 16.16 (height) |
| 0x40-0x4B | s32 x3 | X/Y/ZVelocity | |
| 0x4C | ptr | RelatedObject1Ptr | parent / owner, per handler |
| 0x50 | ptr | RelatedObject2Ptr | |
| 0x54 | ptr | CollisionDataPtr | 0 if none |
| 0x58 | ptr | AIDataPtr | 0 if none |
| 0x5C | u32 | Unk_5c | |
| 0x60 | bytes | ExtraVars | 0x2C (T1/T3) or 0x1C (T4) handler-private bytes |
| 0x90 / 0x80 | 0x38 | SpriteData | ObjectSprite — see §11 |

### Header flags (byte 0x00)

| Bit | Name in disasm | Real meaning |
|---|---|---|
| 0x01 | ACTIVE | slot in use (cleared by `object_freeMemory`). The update loop does **not** test it. |
| 0x02 | VISIBLE | drawn this frame (render only; toggled by flashing code) |
| 0x04 | PAUSE_UPDATE | **update even while `GameState.BattlePaused`** (see §4) |
| 0x08 | STOP_SPRITE_UPDATE | set at spawn for T1/T3/T4; `sprite_load` clears it (e.g. player 0x1D -> 0x15 in `sub_80172F0`) |
| 0x10 | UPDATE_DURING_TIMESTOP | **update even while battle flag 0x4 (dimming) is set** |
| 0x20 | (UNK_20) | set by `object_reservePanel` (object holds a panel reservation); observed on the player from its first move (0x17 -> 0x37) |
| 0x40, 0x80 | unknown | not observed in the trace |

Spawn writes the whole byte: 0x19 for T1 and T4, 0x09 for T3 (table `dword_80032D0`). So by default T1/T4 objects run while dimmed but not during pause, and T3 objects run in neither. Spawners that must run during pause OR in `|= 4` themselves (players: `sub_800753C`; T4 objects spawned by the player: e.g. `sub_80EA438`).

## 3. Pools and spawning

### 3.1 Pool (re)initialisation
At battle start, `sub_800794C` runs `sub_800318C` (reset list: Start.Prev=0, Start.Next=&Sentinel, Sentinel.Prev=&Start, Sentinel.Next=0, current=0), then `InitializeStructsOfObjectType` for T1, T3, T4: zero the allocation bitfield (4 bytes), zero the whole pool including nodes (count x size bytes from the first node), then for every slot i write byte +2 = TypeAndSpriteOffset (0x91 / 0x93 / 0x84) and byte +3 = i. `sub_8007A0C` later calls `sub_801986C` (collision pool reset, §7.1) and `sub_800318C` again. The AIData pool is reset earlier by `sub_800ED00` (from the battle init `sub_80071D4`).

At the end of a round `sub_80094DA` calls `FreeAllObjectsOfSpecifiedTypes(0x1A)` (T1|T3|T4) which calls `object_freeMemory` on every active slot in slot order. AIData/CollisionData are not freed there; their pools are re-initialised by the next battle's init.

### 3.2 `SpawnBattleObjectCommon` (0x08003278)
Called by the typed spawners with r0 = type and a pointer to five words {Index, X, Y, Z, Params}.

1. Look up the per-type descriptor `dword_80032D0[type]` = {bitfield ptr, first node, end node, u8 size, u8 clearSize, u8 initFlags} (T1: 0xD8/0x8C/0x19, T3: 0xD8/0x8C/0x09, T4: 0xC8/0x7C/0x19).
2. **Slot choice: lowest free index.** Scan i = 0,1,2,... testing bit (31-i) of the bitfield; if all 32 are set return r5 = 0 (spawn failed; callers must cope — most just skip).
3. Set the bit; object = node + 0x10.
4. `ZeroFillByWord(object+4, clearSize)`: zeroes object bytes [0x04, 0x90) for T1/T3, [0x04, 0x80) for T4. **Not cleared:** Type/ListIndex (bytes 2,3; still valid), the list node (overwritten by linking), and the whole sprite block (0x90.. / 0x80..) which keeps stale data from the previous occupant until the object's init loads a sprite.
5. Flags = initFlags; Index = arg0; X = arg1; Y = arg2; Z = arg3; Params = arg4. CurState is 0 (from the zero fill), so the object's first update runs its init state.
6. Return r5 = object pointer (the typed spawner then links it, below).

### 3.3 Typed spawners and list insertion

All take r0 = Index, r1 = X, r2 = Y, r3 = Z (16.16), r4 = Params and return r5 = object or 0 (they clobber r5! callers `push {r5}` around them).

| Function | Pool | Linking |
|---|---|---|
| `object_spawnType1` 0x08003320 | T1 | `sub_8003400` (after current) |
| `object_spawnType3` 0x08003358 | T3 | `sub_8003400` |
| `object_spawnType4` 0x080033AC | T4 | `sub_8003400` |
| `sub_800333C` / `sub_8003374` / `sub_80033C8` | T1 / T3 / T4 | always append at tail (`UpdateBattleObjectLinkedList`) |
| `sub_8003390` / `sub_80033E4` | T3 / T4 | always insert at head (`sub_8003440`) |

Only the first three were executed in the reference match (coverage: 7 + 360 + 11 = 378 calls = `sub_8003400` count).

`sub_8003400`: let `new` = node of the new object, `cur` = [0x0200AF70].
- If `cur == 0` (spawn happens outside `RunBattleObjectLogic`, e.g. the players spawned by the battle-mode handler) **or** `cur == new` (the updating object freed itself earlier this update and the new object reused its very slot): append at the tail, i.e. `UpdateBattleObjectLinkedList`: `last = Sentinel.Prev; last.Next = new; new.Prev = last; new.Next = &Sentinel; Sentinel.Prev = new`.
- Otherwise insert **immediately after the object currently being updated**: `new.Prev = cur; n = cur.Next; cur.Next = new; new.Next = n; n.Prev = new`.

Consequences (all verified in the trace):
- A spawned object runs its first update (its CurState 0 init) **in the same tick** it was spawned, right after its spawner (or at the end of the list when appended at the tail, which the running loop also reaches this tick). Example: frame 639, the alliance-0 player's chip spawns T1#5, which already shows state [4,0,0,0] at the end of that frame; frame 645 the player spawns T4#0x48, which appears between the player and T1#5.
- At battle start (frame 72): T4#2 is spawned first (outside the loop, slot 0 of T4, head of list), then the battle-mode handler spawns player alliance 1 (T1 slot 0) and player alliance 0 (T1 slot 1), both appended; during their init each player spawns a T4#8 (T4 slots 1 and 2) which is inserted directly after it. Final list order: T4#2, P(a1), T4#8, P(a0), T4#8 (matches trace frame 72).
- Several objects spawned by the same object in one update end up in reverse spawn order (each is inserted directly after the spawner).

### 3.4 Freeing: `object_freeMemory` (0x08003458)
`Flags = 0`; clear bit (31-ListIndex) in the type's bitfield; unlink: `node.Prev.Next = node.Next; node.Next.Prev = node.Prev`; `sprite_makeUnscalable` (render only). The freed node's own Prev/Next are **not** cleared, and nothing else (AIData, CollisionData, panel reservation) is released — the handler must do that first (`object_genericDestroy` = `sub_801BB78` (drop panel reservation) + `object_freeCollisionData` + `object_freeMemory`).

The update loop continues with `cur.Next` read **after** the handler returns, so:
- an object freeing itself is fine (its stale `Next` still points at the right successor);
- an object freeing its successor is fine (the unlink rewrote `cur.Next`);
- pathological: if an object frees itself and then spawns into a *lower* free slot, `sub_8003400` inserts after the dead node: the child runs this tick (reached through the dead node's `Next`) but is not reachable from the live predecessor afterwards (orphaned; its node's Prev points at a dead node). Likewise freeing self and then freeing the old successor leaves `cur.Next` pointing at a freed object that the loop still calls. **Implement the list exactly as the game does (per-slot Prev/Next "pointers", nothing cleared on free, `Next` of the current node read after the handler) and these corner cases reproduce automatically.**

### 3.5 Destroy convention (CurState 8)
Handlers request destruction by `str #8 -> CurStateActionPhaseAndPhaseInitialized` (CurState = 8, CurAction/CurPhase/PhaseInitialized = 0) or by another object writing CurState=8 into them. Nothing happens immediately: the object's **next** update runs its destroy entry, which usually is `object_freeMemory` directly (e.g. T4#0 `off_80E055C`) or `object_genericDestroy`. The slot therefore stays allocated until the object's next executed update (later if the object is skipped by pause/dimming). The player's destroy entry `sub_8016C4E` is special (§12.8): it never frees the player object.

## 4. `RunBattleObjectLogic` (0x080031AC)

```
clear render-list counts (u8 at 0x02036778, 0x0203CA7C, 0x02036830)   // object_Clear3RAMBytes_800371A
node = Start.Next
while node != &Sentinel:
    [0x0200AF70] = node                       // "current object"
    obj = node + 0x10; f = obj.Flags          // flags sampled BEFORE the handler runs
    run = true
    if GameState.BattlePaused != 0 and !(f & 0x04): run = false
    if run and (battleFlags & 0x04) and !(f & 0x10): run = false   // battle_isTimeStop
    if run: JumpTable[obj.Type & 0xF][obj.Index](obj)            // T1 0x08003C9C, T3 0x08003EC4, T4 0x080042C8
    object_800372A(obj)                       // append obj to its type's render list (even if skipped / just freed)
    node = node.Next                          // re-read after the handler
[0x0200AF70] = 0
```

- Handler lookup: `.BattleObjectFunctionJumptableTable` = {NULL, T1BattleObjectJumptable, NULL, T3BattleObjectJumptable, T4BattleObjectJumptable}, indexed by the type nibble, then by `Index`.
- Each handler receives r5 = object; nothing else is passed. T1#0 (`sub_80B81EC`) further dispatches on `AIData.ActorType`: 0 virus `sub_8108F50`, 1 navi `sub_80F2330`, 2 player `sub_80EA460`.
- The render lists (`object_800372A`) are consumed only by the drawing passes after logic (`sub_8003E18` etc.: if VISIBLE draw, else clear sprite+0x24; if ACTIVE compute screen coords). No gameplay code reads them; a headless engine can drop them.
- There is no per-object "skip if inactive" test: whatever is linked gets called.

### 4.1 Where `RunBattleObjectLogic` sits in a tick
One game tick = one call of `battle_8007800`. In the fighting top state (BattleState[0] = 4, `battle_8007A44`) the order is:

| # | Step | Relevance |
|---|---|---|
| 1 | `sub_801FE6C`, `sub_8020140` (link) then `sub_801FEEE` -> `sub_801FF18` -> `sub_800A0D6`: latch both players' input records at 0x02036820/0x02036828 (held/pressed/released) | inputs for this tick |
| 2 | battle-terminate / link-error checks | |
| 3 | `sub_800A01C` (clears u32 0x03000EA8) | |
| 4 | battle-mode handler (`off_8007B50[GetBattleMode]`, PvP = `sub_8009158`) running the sub-state BattleState[1] (0 intro: spawns T4#2 and the players via `sub_80091F0`->`sub_8007358`; 8 custom screen; 12 fighting; 20 end...). The running sub-states call `sub_8012DFC(0)` and `sub_8012DFC(1)`, which copy each side's held keys into its player's AIData joypad words (§M3.1) | spawns before logic are appended at the tail; player input for this tick |
| 5 | **`RunBattleObjectLogic`** | all object logic |
| 6 | `sub_802FFF4` camera, incl. `camera_doShakeEffect_80301e8` | **consumes RNG1 while a shake is active** |
| 7 | `sub_800BFC4` panel timers (skipped if paused or dimming) | panels |
| 8 | `sub_800FDC0`: for each of the 8 "alive actor" pointers at BattleState+0x80 whose AIData.ActorType == 2 -> `sub_800FDEA` (ChipsHeld = number of chips left in the hand, Chip = current hand entry) | chip state for this tick |
| 9 | `sub_801BEE0` HUD tasks, `sub_802CEC8`, `chip_800AEE8` | |
| 10 | if !paused: { if !dimmed: BattleState[0x0E] = (v+1)%20, BattleState[0x16] = (v+1)%180 }; `sub_802CDFE` | global cycle counters |
| 11 | `sub_80102AC(0)` and, in netbattle, `sub_80102AC(1)`: HP-bug drain, only if `BattleState[0x14+side] & 5`: period = {0,40,30,20,10,5,3,2}[navi stats[0x19]]; if period != 0 and the side's player has HP > 1, AIData+0x0A += 1 and when it reaches the period subtract 1 HP and reset it (a second, per-player-tick drain keyed on stats[0x18] is `sub_8010230`, §H2) | HP |
| 12 | BattleState+0x64 += 1 ("ticks" in the trace) | |
| 13 | presentation only: `sub_80027B4`, `sub_800286C`, `sub_8003E18` (T1 draw), `sub_8004218` (T3), `sub_8004510` (T4), ... | none |

Then `battle_8007800` increments BattleState+0x60 ("frames"). In the end-of-battle top state (BattleState[0] = 8, `sub_8007B9C`) `RunBattleObjectLogic` still runs every tick.

### 4.2 Pause and dimming in practice
- `GameState.BattlePaused` (0x02001B8A) is set from the first fighting-state tick (trace frame 72) through the intro, custom screen and "battle start" banner, and cleared at frame 593 together with battle flag bit 0 (fight active). During that time only flag-0x04 objects (players, their T4 helpers, T4#2...) run.
- Dimming (battle flag 0x04) is set by dimming chips; only flag-0x10 objects run (default for T1/T4). Not exercised in the reference match.

## 5. State-machine convention

Every handler follows the same pattern (e.g. T1#1 `sub_80B8210`):

```
handler(obj): STATE_TABLE[obj.CurState / 4](obj)        // [init, update, destroy]
init:   set up sprite/collision/AIData...; obj.CurStateActionPhaseAndPhaseInitialized = 4   // u32 store: CurState=4, CurAction=CurPhase=PhaseInit=0
update: ACTION_TABLE[obj.CurAction / 4](obj)  -> PHASE_TABLE[obj.CurPhase / 4](obj)
        (actors: ACTION_TABLE[obj.CurAction] - see CurAction in section 2)
phase:  if !obj.PhaseInitialized { entry code; obj.PhaseInitialized = 4 (or 1) } ; per-tick code;
        to change phase: obj.CurPhase += 4 (or =k) and PhaseInitialized = 0 (often one u16 store at 0x0A)
destroy: free (see §3.5)
```

Because the phase entry runs in the same tick as the phase switch only if the switching code falls through, **a phase change normally takes effect on the next tick**; the trace shows the pair (CurPhase, PhaseInitialized) as (k, 0) on the tick of the switch and (k, 4) afterwards (e.g. player frames 635/636: [4,8,0,0] then [4,8,0,4]).

### 5.1 Actor attack/action helpers (players, navis, viruses)
- `object_setAttack0..5(r0 = action)` (common tail `loc_8011698`): `CurAction = r0`; `CurPhase = PhaseInitialized = 0` (u16 store); `AIAttackVars.Unk_00` (u16 at AIData+0xA0) = 0; `AIAttackVars.Unk_1c` (AIData+0xBC) = n (the suffix 0..5, "kind" of action); then `sub_801011A`: AIAttackVars.Unk_1d = 0, AIAttackVars.Unk_1e (u16) = 0, and `sub_80E1662`, which writes 0 to `[AIData.Unk_40]+0x64` (u32) and `+0x10` (u8). For players AIData.Unk_40 is 0, so these writes hit 0x00000064/0x00000010 (BIOS ROM on GBA, i.e. ignored) — **a Rust port must treat stores through a null object pointer as no-ops**.
- `object_exitAttackState`: CurAnim = 0, then (`sub_801171C` entry skips that): AIAttackVars.Unk_1b = 0; kind = AIAttackVars.Unk_1c; if kind != 4 { if kind == 2: AIData.Unk_19 = AIAttackVars.Unk_05; if kind == 3: AIData.Unk_15 = AIAttackVars.Unk_05; AIData.Unk_1a = 0; clear AIData+0x44 flags 0x1000003F; `sub_8012EA8` (AIData.Unk_1d/1b/1e = 0, clear AIData+0x44 flags 0x60000); clear ObjectFlags1 0x400000 (USING_ACTION) }; **CurAction = 8** (the player's neutral/controllable action); AIAttackVars.Unk_00 (u16) = 0. It does *not* touch CurPhase/PhaseInitialized.
- AIData flag words: +0x44 (`SetAIData_Unk_44_Flag`/`ClearAIData_Unk_44_Flag`/`GetAIData_Unk_44_Flag`, 0x0800FFE4/0x0800FFEE/0x0800FFFE) and +0x48 (`sub_8010312` set, `sub_801031C` clear, `sub_801032C` get;).

## 6. AIData pool (0x02034080, 8 x 0x100)

- Bitfield u32 at 0x0203F6A0 (bit 31-k = slot k). `sub_800ED00` at battle init writes each slot's own bit into AIData+0x7C (0x80000000, 0x40000000, ...) and zeroes the bitfield.
- `object_createAIData`: lowest free slot; zero [0x00,0x7C), [0x80,0xA0), [0xA0,0xF0); **not cleared**: +0x7C (slot bit) and [0xF0,0x100). Returns the pointer or 0 (pool full). Caller stores it in obj+0x58.
- `sub_800ED80(ai)`: bitfield &= ~ai.Unk_7c.
- Layout: +0x00 ActorType (0 virus, 1 navi, 2 player), +0x01 AIIndex (form / AI variant; selects per-form tables), +0x02 Unk_02 (1 = "don't count/free", see §12.8), +0x03 from enemy table, +0x04..+0x1E misc stats, +0x20 TotalDamageTaken, +0x22 JoypadHeld, +0x24 JoypadPressed, +0x26 "JoypadUp" (really: released edge), +0x28 "JoypadReleased" (really: previous held) (§M3.1), +0x40.. pointers/work, +0x44 and +0x48 flag words, +0x58 ptr (players: their T4#8 helper), +0x7C slot bit, +0x80 AIState (0x20), +0xA0 AIAttackVars (0x50).

## 7. CollisionData pool (0x020384F0, 32 x 0xA8) and the collision protocol

### 7.1 Allocation
- `sub_801986C` (battle init): bitfield (0x02035310) = 0; for k = 0..31 zero the 0xA8 bytes and set CollisionIndexBit (+0x44) = 0x80000000 >> k; zero the 40 panel masks at 0x02034F60 (`sub_8019FA4`).
- `object_createCollisionData`: lowest free slot; zero [0x00,0x44) and [0x48,0xA8) (keeps +0x44); Enabled (+0x00) = 1; obj.CollisionDataPtr = it; returns it (0 if pool full — the player init then frees itself).
- `object_freeCollisionData(cd)`: if cd: Enabled = 0; bitfield &= ~cd.CollisionIndexBit. (Panel mask bits are not cleared here.)
- `object_setupCollisionData(cd, selfType, targetType, hitModifier)`: ParentObjectPtr (+0x38) = obj; HitModifierBase (+0x0E) = r3; PrimaryElement = obj.Element & 0xF, SecondaryElement (+0x19) = obj.Element & 0xF0; Alliance/Flip (+0x04) = obj's; PanelX/Y (+0x0A) = obj's; Region (+0x01) = 1; StaminaDamageCounterDisabler (+0x07) = (u8)obj+0x2E; SelfDamage (+0x2E) = obj.Damage; SelfCollisionTypeFlags (+0x30) = `byte_8019C7C[selfType*2 + alliance]` (plus bit 0x10000 if dimmed); TargetCollisionTypeFlags (+0x34) = `byte_8019C7C[targetType*2 + alliance]`; then `sub_8019F44` decodes the SelfDamage flag bits (keep low 11 bits; bit 0x8000 -> double; 0x4000 -> paralyze: StatusEffectBase = 0x10, HitModifierBase = 1; 0x2000 -> uninstall `sub_801A4D0(0xF8)`; 0x1000 -> erase-cross `sub_801A4D0(0xF7)`; 0x0800 -> nothing). (Corrected per field-collision-damage.md.) `sub_801A082` redoes only the modifier/damage/type-flag part.
- Other setters: `object_setCollisionRegion`/`object_clearCollisionRegion` (+0x01), `object_updateCollisionPanels` (copy obj PanelX/Y to +0x0A, Direction +0x0C = `sub_800E994`(old,new)), `object_setCollisionPanelsToCurrent`, `object_setCollisionHitEffect` (+0x09), `sub_8019F86` (HitModifierBase), `sub_8019F8C` (element), `sub_8019F9E` (+0x18 SecondaryElementWeakness).

### 7.2 Status flag words (the trace's "status" field)
`object_setFlag1/clearFlag/getFlag` OR/BIC/read `ObjectFlags1` (+0x3C) of the object's collision data; `object_setFlag2/clearFlag2/getFlag2` do the same on `ObjectFlags2` (+0x40). These take r5 = object and **dereference CollisionDataPtr without a null check**. ObjectFlags1 bit names: the original `CollisionData` struct (0x1 guard, 0x2 invis, 0x8 invulnerable, 0x10 airshoe, 0x20 floatshoe, 0x100 dead, 0x200 flashing, 0x400 flinching, 0x800 paralyzed, 0x2000 blind, 0x10000 frozen, 0x20000 superarmor, 0x40000 undershirt, 0x80000 move complete, 0x100000 drag, 0x400000 using action, 0x2000000 affected by ice, 0x80000000 bubbled).

### 7.3 Per-tick protocol (panel masks)
Collision is resolved through the 40 panel masks at 0x02034F60, not through a global pass. Every collidable object does, inside its own update:

1. **remove/resolve** at the start (`object_removeCollisionData` -> IWRAM `_object_removeCollisionData`): for every panel P covered by its region (Region index -> `PanelOffsetListsPointerTable` list of (dx,dy) pairs terminated by 0x7F, dx multiplied by `object_getFlipDirection(cd.Alliance, cd.Flip)` = +1 facing right/-1 facing left, added to cd.PanelX/Y; Region with bit 0x80 = field-wide pattern from `byte_8019C34` via `object_checkPanelParameters`), if valid: clear own bit in mask[P] (`sub_3007880`), recompute the panel's flag word (`_object_updatePanelParameters`), then **pair** with every other collision slot c whose bit is still in mask[P] (`sub_30075FC`, skipped while paused): if `(self.Unk_68 & c.bit) == 0` then `self.Unk_68 |= c.bit; c.Unk_68 |= self.bit;` `sub_3007650(self, c)`; `sub_3007650(c, self)`. `sub_3007650(d, a)`: if `d.TargetCollisionTypeFlags & a.SelfCollisionTypeFlags` then `sub_3007218(d, a)` (hit registration on d: filters on invis/guard/etc., ORs a's SelfCollisionTypeFlags into d.FlagsFromCollision (+0x70), a.bit into d.Unk_7c, a.SecondaryElement into d.DamageElements (+0x76), StatusEffectFinal, counter-hit bit 0x40, ...) and `sub_3007692(d, a)` (d.Unk_6c |= a.SelfType; d.Unk_77 |= a.SecondaryElement; d.ElementDamage[a.PrimaryElement] (+0x94+2e) += a.SelfDamage; +bonus for heat on grass). Finally `sub_3007708` applies panel side effects (grass + heat -> normal, ice + aqua, holes...) unless paused.
2. read and act on the accumulated results (players: `sub_801AC6C`, §H2).
3. **present** at the end (`object_presentCollisionData`): (quirk: stores the caller's r1 into Unk_54, then) unless dimmed: HitModifierFinal (+0x0F) = 0, Unk_03 = 0, Unk_54 = 0; always: StatusEffectFinal (+0x11) = 0 and zero the result area [0x68, 0xA8); then (`sub_300777C`) set own bit in mask[P] for every covered valid panel and recompute panel flags.

Quirk (code-derived, not exercised): for field-wide regions (Region & 0x80) the present path `sub_300777C` recomputes the panel flag word with the wrong arguments (`_object_updatePanelParameters(row, r8)`, r8 = the caller's r8) instead of (x, y), so those panels' stored flag words are not refreshed on present; the function also returns with r8 = the collision-data pointer (it pushes r0 where it meant r8). The remove path is correct.

So a pair (A, B) is processed exactly once per "cycle", during whichever of the two runs its remove first while the other is registered; results land in both objects' result areas and are read by each object at its next update (A reads results created by objects updated after A in the previous tick and by its own remove this tick). The Rust engine must reproduce this ordering exactly; it is the same for every object type. Players only call remove when battle flag bit 0 is set and present when not paused (§M1, §H2).

**One-tick attack regions (T3#3, `sub_80C52B0`).** Many attacks use `object_spawnCollisionRegion` (0x080C536A): a T3 object with Index 3, inserted right after the attacker. Its init `sub_80C52D0` runs in the same tick: if the panel is invalid or no collision slot is free, free itself; else set X/Y from the panel, `object_createCollisionData`, `object_setupCollisionData(self = Param4, target = Param3, modifier = ExtraVars[0x60])`, Region = Param1, HitEffect = Param2, optional status (`object_setCollisionStatusEffect1`) and `sub_801A4D0`, **present**, CurState 4, and immediately `sub_80C532E`: **remove** (resolves it against everything registered on its panels right now), `object_spawnCollisionEffect` (RNG2 if it hit and has a hit effect, §13), optionally `*ExtraVars[0x70] = FlagsFromCollision & 0x0C000000`; if it hit anything, or `--Timer` goes <= 0 (Timer is 0 unless the spawner set it, so by default at once), clear its region, free its collision data and free itself. Otherwise present again and live another tick. A default region therefore never survives to the end of its spawn tick (no T3 object ever appears in the trace); its victims read the result at their next update (same tick if they are later in the list). The GunDelSol used it once per firing tick (360 spawns in round 1).

## 8. Common helpers used by handlers (non-exhaustive)

| Helper | Address | Behaviour |
|---|---|---|
| `object_getFlip` | 0x0800E456 | Alliance ^ DirectionFlip |
| `object_getAllianceDirection(a)` | 0x0800E2C2 | 1 - 2a (+1 for alliance 0, -1 for alliance 1) |
| `object_getFrontDirection` | 0x0800E2CA | `object_getFlipDirection(Alliance, DirectionFlip)` = +1 if (a^f)==0 else -1 |
| `object_getEnemyDirection` | 0x0800E2C0 | same as getAllianceDirection(Alliance) |
| `object_getPanelDataOffset(x,y)` | 0x0800C90A -> IWRAM 0x03007958 | 0 unless 1<=x<=6 and 1<=y<=3, else 0x02039AE0+((y*8+x)<<5) |
| `object_isValidPanel`, `object_canMove`, `object_reservePanel`/`object_removePanelReserve`, coordinate helpers | see §M6 (movement) and §M7 (panel data) | |
| `object_setAnimation(a)` | 0x0800F2B6 | CurAnim = a, CurAnimCopy = 0xFF |
| `object_subtractHP(n)` | 0x0800E2D8 | see §H3 (damage) |
| `object_setFlag1/...` | §7.2 | |
| `battle_isPaused` 0x0800A03C / `battle_isTimeStop` 0x0800A098 / `battle_getFlags` 0x0800A2F0 / `battle_isBattleOver` 0x0800A18E | | |
| `battle_networkInvert(a)` | 0x0800A9EC | a ^ BattleState[0x0D]; BattleState[0x0D] = the local side (0 on the P1 console). Code gated on it is presentation-only as far as observed (HUD, colours) |
| `sub_80103BC(alliance)` | 0x080103BC | player object of a side = BattleState+0xD0+0x10*alliance entry 0 if its NameID type is 2 (the loop never advances; only entry 0 is looked at), else 0 |

## 9. BattleState fields touched by the object system

| Off | Meaning |
|---|---|
| +0x00..0x03 | top state / sub-state / phase bytes (trace "state") |
| +0x04, +0x05 | per-alliance count of registered actors (`sub_8007778` increments unless AIData.Unk_02 == 1) |
| +0x0B | nonzero forces `battle_isBattleOver` (with +0x12/+0x13) |
| +0x0D | local side (network inversion): 0 on the P1 console, 1 on the P2 console |
| +0x12, +0x13 | alive counts of alliance 0 / 1 (initialised from +0x04/+0x05 after spawning; decremented by `sub_800A11C` at deletion). `battle_isBattleOver` = !(+0x12 && +0x13 && +0x0B == 0) |
| +0x0E, +0x16 | cycle counters mod 20 / mod 180 (§4.1 step 10) |
| +0x32 | battle flags (bit0 fight active, bit2 dimming) |
| +0x3C | BattleSettings pointer; settings+0x0C = entity spawn list |
| +0x5C | intro/chip-enable bits (`sub_8001382` ORs, `sub_800139A` reads): 0x10/0x01/0x02 set by T4#2 (§A.4), 0x04/0x08 = chips enabled for alliance 0/1 |
| +0x60 / +0x64 | frame counters |
| +0x80 | "alive" actors: 4 pointers per side (side 0 at +0x80, side 1 at +0x90), filled by `sub_8007778` (first empty entry), cleared by `sub_80077B4` |
| +0xD0 | copy of +0x80..+0x9F made after spawning (`sub_8007368`), used by `sub_80103BC` |

## 10. Auxiliary objects: T1#5 and T4 kinds 0, 2, 8, 0x48 (sections A.x)

Evidence: static reading of the handlers plus three runs of the replay with write watches on the allocation bitfields (`eActiveT1BattleObjectsBitfield` 0x02034000, `eActiveT4BattleObjectsBitfield` 0x02036710) and on `BattleState.Unk_5c` (0x020348DC), and per-frame dumps of pool slots. Frame numbers below are trace frames (the machgun trace).

### A.0 Summary

| Kind | Handler | What it is | Spawned by (this match) | Lifetime | Needed headless? |
|---|---|---|---|---|---|
| T1#5 | `sub_80B8CD8` (0x080B8CD8) | Sprite overlay attached to an actor (here the GunDelSol gun in MegaMan's hand) | `sub_80B8E30` ← GunDelSol routine `sub_80EDB14` | Until the owner zeroes its back-pointer slot (`AIData.Unk_68`), then CurState 8 → freed on the next tick | **Slot and list position only.** Writes nothing but itself |
| T4#0 | `loc_80E0548` (0x080E0548) | Generic one-shot effect (explosion, sparkles, …) | `SpawnT4BattleObjectWithId0` (0x080E05F6): deletion (`sub_801746E`), `sub_8014D70`, chips | `Timer` ticks, or **the length of its animation** when Timer=0 | **Slot, list position and lifetime**, and the lifetime depends on animation data. No other effect |
| T4#2 | `sub_80E0638` (0x080E0638) | Battle-intro sequencer (screen fade-in, then wait for the actor fade-in queue) | `sub_80E06F8` ← battle-mode handler `sub_80091F0` | Spawn frame → frees itself when the fade-in queue is empty (frames 72→122) | **YES.** It sets `BattleState.Unk_5c` bits 0x10/0x01/0x02, which gate the player intro and the battle-mode state machine |
| T4#8 | `sub_80E0DF0` (0x080E0DF0) | Buster charge glow on each player | `sub_80E0F02` ← player init `sub_80172F0` | The whole round, until `AIData.Unk_58` is zeroed (deletion phase 0 `sub_801741C`) | **Slot and list position only**, plus garbage position fields (see below). It plays charge sounds |
| T4#0x48 | `sub_80E5C2C` (0x080E5C2C) | Reticle/flash sprite 2 panels ahead of the GunDelSol user | `sub_80E5D12` ← GunDelSol `sub_80EDB14` | Until the owner calls `sub_80E5D3E` (sets CurState 8), or until `owner.RelatedObject1Ptr` becomes 0 | **Slot and list position only.** It plays SE 0xF9 every 11 ticks |

None of these five kinds calls the RNG. The RNG write watch over the whole match found only camera shake (`camera_doShakeEffect_80301e8`), `sub_8000D12`, `sub_81209DC` and `sub_80AA88C`. None of them writes HP, panels, collision data or another object's gameplay fields. T4#2 is the only one that writes global battle state.

**Why the purely visual ones still matter for exactness:**
1. They occupy pool slots. Allocation takes the lowest free index (bit 31 = slot 0), so they shift the slot of every later spawn.
2. They are linked into the update list right after the spawning object. That changes the order in which later objects run (and the trace's object order).
3. Their header, state, position and anim fields are part of the traced "observable state". Some of those fields are register garbage (§A.3, §A.5, §A.7).

If the Rust engine's comparison ignores visual-only objects, T1#5, T4#8 and T4#0x48 can be dropped entirely. T4#0 also needs its animation-derived lifetime only for slot-exactness. T4#2 must be modeled in any case.

### A.1 Slot/list timeline observed (round 1: frames 72–1113; round 2: 1224–2521)

T4 bitfield (bit 31 = slot 0) from watching writes to 0x02036710:

| Frame | Event | Bitfield after |
|---|---|---|
| 72 | T4#2 spawned by `sub_80091F0` → slot 0 (appended to list tail, runs this tick) | 0x80000000 |
| 72 | Player init (first `RunBattleObjectLogic`), first T1 player spawns T4#8 → slot 1 | 0xC0000000 |
| 72 | Second player spawns T4#8 → slot 2 | 0xE0000000 |
| 122 | T4#2 frees itself (slot 0) | 0x60000000 |
| 645 | GunDelSol spawns T4#0x48 → slot 0 | 0xE0000000 |
| 766 | T4#0x48 freed | 0x60000000 |
| 939 | Deletion (`sub_801746E`) spawns T4#0 ×2 → slots 3, 4; T4#8 of the dying player (slot 1) freed | 0xB8000000 |
| 962 | Both T4#0 freed | 0xA0000000 |
| 1113 | `FreeAllObjectsOfSpecifiedTypes` (battle end) | 0 |

T1 bitfield (watching writes to 0x02034000): players take slots 0 and 1 at frame 72 (`object_spawnType1` ← `sub_800753C`). T1#5 takes slot 2 at 639/780/921 (freed at 778/919/1060). In round 2 T1#0x57 (slot 2) is present, so T1#5 gets slot 3.

Update order at frame 72: `T4#2, T1#0(alliance 1), T4#8, T1#0(alliance 0), T4#8`. Each T4#8 sits right after its owner because it was spawned from the owner's CurState-0 handler. Spawn-order inversion at frame 645: `… T1#0 T4#48 T1#5 …`. T1#5 (spawned at 639) and T4#48 (spawned at 645) were both inserted immediately after the player, so the later spawn is ahead. The same happens at 939: the second explosion comes before the first.

### A.2 T1#5: attached sprite overlay (`sub_80B8CD8`)

**Spawn helper** `sub_80B8E30`. Inputs: r1–r3 = X/Y/Z (don't care, overwritten at init), r4 = Params, r5 = owner, r7 = address of a pointer slot in the owner.
- `object_spawnType1(5, …)`.
- On success: `RelatedObject1Ptr = owner`, `ExtraVars[0x60] = r7` (the slot address), `PanelXY` and `AllianceAndDirectionFlip` copied from the owner, `Flags |= 0x14` (update during pause and while dimmed).
- **Always** `*r7 = new object or 0`.

For GunDelSol (`sub_80EDB14`): `r4 = 7 + AIAttackVars.Unk_03` (Param1 = 9 here: GunDelSol level byte 2) and `r7 = AIData + 0x68`. The object pointer therefore lives in `AIData.Unk_68`.

**Params:**
- Param1 indexes `byte_80B8BD4` (0x080B8BD4), 5 bytes per entry: `[sprite category byte-offset, sprite index, palette, Unk_0c (Y/Z lift, s8), attach-flag]`. Entry 9 is `0C 3B 06 00 0D`.
- Param2 = initial animation.
- Param3 ≠ 0 means it keeps animating while dimmed.
- Param4 is added to the palette.

**CurState 0** `sub_80B8CF8`, runs in the spawn tick because the object is inserted after the current object:
- `sprite_load(0x80, cat, idx)`, no shadow.
- `Unk_0c` = entry[3], or 0xF8 if `owner.NameID ∈ [0x1AC,0x1B5]` and navi-stat 0x2C ≠ 0.
- `CurAnim = CurAnimCopy = Param2`, load the animation, set VISIBLE, palette.
- Offsets:
  - Param1 == 0xF (special case): (0,0) if `NameID == 0x1A0`, (0xB, −7) if `NameID == 0x1B6`, else (0, 0xD).
  - Otherwise, if entry[4] ≠ 0, the offsets come from `sub_8018810(owner.NameID, entry[4], alliance, flip)`: the owner sprite's attach point number entry[4] (13 for the GunDelSol guns: (24, 24) for MegaMan), x times the flip direction. Entry[4] = 0 means no offset, so attach point 0 can't be used.
  - Stored as `XVelocity = dx<<16`, `ZVelocity = dz<<16`.
- CurState = 4, then it immediately runs the update once.

**CurState 4** `sub_80B8DA6`, every tick:
- `X = owner.X + XVel`, `Y = owner.Y − (s8)Unk_0c<<16`, `Z = owner.Z + ZVel − (s8)Unk_0c<<16`.
- Copies the owner's VISIBLE bit, color shader, final palette, alpha/mosaic and flip.
- If `*ExtraVars[0] == 0`: clear VISIBLE and set CurState 8.
- Otherwise, if not paused and (Param3 ≠ 0 or not dimming), call `object_updateSprite`.

**CurState 8**: `object_freeMemory`. It is therefore freed on the **tick after** the owner zeroes the slot (and in the same tick as the zeroing if the overlay runs later in the list than the owner, which is always the case when the owner spawned it).

**Owner-side contract** (GunDelSol):
- Increments `T1#5.CurAnim` through the pointer (`ldrb/strb [Unk_68,#0x10]`) at the first shot (frame 645, anim 0→1) and at wind-down (767, anim 1→2).
- Zeroes `AIData.Unk_68` at the end of the chip (`sub_80EDC78`) or on interruption. Every hit-reaction/deletion action init does `RelatedObject1Ptr = 0; AIData.Unk_68 = 0` (17654, 17726, 17819, 17911, 18027; 3267, …).
- Other callers use `sub_80B8E58(obj)` (CurState 8 directly), `sub_80B8E70(obj, anim)` (CurAnim = anim, CurAnimCopy = 0xFF) and `sub_80B8E7C` (CurAnimCopy = 0xFF).
- Edge case: if the spawn failed, `Unk_68 == 0` and the increments hit address 0x10 (BIOS, no effect).

Trace check: spawned 639 (state [4,0,0,0], flags 0x17, pos = player + (24, 0, 24)); CurAnim 1 at 645 and 2 at 767; at 777 state [8,…] and flags 0x15; gone at 778.

### A.3 T4#0: generic effect (`loc_80E0548`)

**Spawn** `SpawnT4BattleObjectWithId0`: `object_spawnType4(0, X=r1, Y=r2, Z=r3, Params=r4)`, returns r0 = object or 0. Flags stay at the T4 default (0x19, and 0x08 is cleared by `sprite_load`), so the object runs while dimmed but **not during pause** unless the caller ORs in 0x04 (for example `sub_8014D70`).

**Params:**
- Param1 = effect id into `byte_80E0398` (0x080E0398, 4 bytes per entry: `[sprite category byte-offset, sprite index, animation, palette]`).
- Param2 = flip.
- Param3 = palette add.
- Param4 ≠ 0 → `sub_8002E14(0)` (sprite priority bits, visual).

**Optional caller post-setup:**
- `obj.Timer = N`: fixed lifetime.
- `sub_80E060E(obj)` (0x080E060E): ExtraVars = 1 and RelatedObject1 = r5, so VISIBLE follows the related object.
- `sub_80E0602(obj)`: Timer = 1, i.e. dies on the next update.

**CurState 0** `sub_80E0568`: `sprite_load`, `CurAnim = CurAnimCopy = entry.anim`, `sprite_loadAnimationData`, **then `sprite_update` once**, palette, flip, VISIBLE, then CurState = 4. The init tick is the spawn tick when spawned from inside `RunBattleObjectLogic`.

**CurState 4** `sub_80E05C4`, in this exact order:
1. `Timer -= 1` (u16). If the result is 0, go to destroy.
2. `sub_80E0616` (copy VISIBLE from the related object if ExtraVars ≠ 0).
3. `f = sprite_getFrameParameters()`. If `(f & 0x80) && (s16)Timer <= 0`, go to destroy.
4. `sprite_update()`. This runs **always**, including on the destroy path.

Destroy = clear VISIBLE, CurState = 8. CurState 8 → `object_freeMemory` on the next tick.

**Lifetime.** Spawned (init) at tick T:
- Timer = N > 0: CurState 8 at T+N, freed at T+N+1. The end of the animation is ignored because Timer > 0.
- Timer = 0 (default): the first decrement gives 0xFFFF = −1, so the object dies when the animation reports end-of-animation. That gives CurState 8 at T+S and freed at T+S+1, where **S = sum of the frame durations of the animation** (holds when every duration ≥ 1).

Verified cases:

| Effect id | Where | Timer | Spawn tick | Freed | Lifetime |
|---|---|---|---|---|---|
| 0x03 (deletion explosion) | frame 939 (round 1) and 2347 (round 2) | 0 | 939 / 2347 | 962 / 2370 | S = 22: 22 frames of 1 tick, last frame flags 0xC0 |
| 0x2E | `sub_8014D70`, 1802 | 0x36 | 1802 | 1857 | 54 ticks |
| 0x3A | 2345 | 0 | 2345 | 2357 | S = 11 |
| 0x39 | 2360 | 0 | 2360 | 2372 | S = 11 |

The per-tick sprite dumps (T4 slots 0, 3, 4) match the stepping model of §S.2 exactly.

S computed from ROM for every entry of `byte_80E0398` ("z" = LZ77-compressed sprite):
`00:22 01:16 02:16 03:22 04:1 05:17 06:14 07:4 08:15 09:11 0a:12 0b:13 0c:8 0d:3 0e:19 0f:9 10:12 11:8 12:4 13:4 14:4 15:4 16:13 17:13 18:10 19:13 1a:13 1b:15 1c:15 1d:22 1e:18 1f:31 20:10 21:10 22:27 23:32 24:22 25:13 26:16 27:13 28:20 29:9z 2a:12 2b:14z 2c:12z 2d:13 2e:12z 2f:14 30:17 31:17 32:20 33:12 34:14 35:13 36:13 37:13 38:16 39:11 3a:11 3b:16 3c:16 3d:10 3e:12z 3f:12z 40:12z 41:15 42:22 43:10 44:24 45:17 46:24 47:18 48:1 49:21 4a:22 4b:7z 4c:11 4d:1 4e:16 4f:16 50:24 51:36 52:11 53:8 54:12 55:14z 56:32 57:4 58:10 59:6 5a:43 5b:41 5c:22 5d:10 5e:18 5f:11 60:17 61:15 62:25 63:18 64:13 65:13 66:13 67:13 68:17 69:33 6a:10 6b:22`.

UNCERTAIN: only 0x03, 0x39 and 0x3A were verified at run time. The others are computed from the ROM animation data with the model of §S.2.

**Deletion spawns and register garbage.** The deletion action phase 1 `sub_801746E` does `ldmia {r1-r3} = self.X,Y,Z; r4 = 3; bl SpawnT4BattleObjectWithId0; bl SpawnT4BattleObjectWithId0`, so the second call reuses whatever r1–r3 hold after the first. After `SpawnBattleObjectCommon` + `sub_8003400` returns:
- r1 = address of the **current object's list node** (`eUnkBattleObjectLinkedList` value).
- r2 = the node that was the current object's `Next` before the insertion.
- r3 = Z.

So the second explosion has X = 0x0203A9A0 (T1 slot 0 node), Y = 0x02036928 (T4 slot 1 node, i.e. the dying player's T4#8), Z = 0. Confirmed in trace frame 939 (`pos` = [0x0203A9A0, 0x02036928, 0]).

The value is visual only and nothing reads it. The engine does not model GBA addresses: it spawns the second explosion with the first one's Z and placeholder X and Y, marked as unknown (`effect::spawn_after_spawn`), and the trace comparison skips that object's X and Y. (For reference, the nodes are `T1 node(k) = 0x0203A9A0 + k·0xD8` and `T4 node(k) = 0x02036860 + k·0xC8`.)

### A.4 T4#2: battle-intro sequencer (`sub_80E0638`), gameplay-relevant

**Spawn** `sub_80E06F8`: `object_spawnType4(2, r1..r4 = caller garbage)` and then `Flags |= 0x04`.
- Called from the battle-mode-0 start handler `sub_80091F0` **before** `sub_8007358` spawns the players and before `PauseBattle`.
- It is spawned outside `RunBattleObjectLogic`, so it is appended to the list tail: first in the list, T4 slot 0.
- Observed fields: Params = 2, X = 1, Y = 0xFFFF, Z = 0x02036828 (garbage from `sub_80091F0`'s registers, never written again), flags 0x1D.

Its jump table has only two entries: CurState 0 and 4. It has **no CurState 8 entry** and never uses one.

**Behaviour:**
- **CurState 0** `sub_80E0654`: `Flags |= 0x08` (no sprite), CurState = 4, then falls into the update.
- **Phase 0** `sub_80E0684`.
  - First tick:
    - `sub_8001382(0x10)`: `BattleState.Unk_5c |= 0x10`.
    - Fade type is 0 or 8:
      - If `GetBattleEffects() & 0x400`: 8 if `BattleState.Unk_1a > 1`, else 0.
      - Otherwise: 8 if `BattleSettings[5] ≥ 2`, else 0.
    - `SetScreenFade(type, 0x10)`.
    - `byte_2036740 = 1` (panel palette animator, visual).
  - Every tick: once `IsScreenFadeActive()` returns 0, `sub_8001382(1)` (`Unk_5c |= 0x01`) and go to phase 4.
- **Phase 4** `sub_80E06E0`: once `sub_800AA92(0) == 0` (count of non-null entries in the 8-word actor fade-in queue `unk_2038170`), `sub_8001382(2)` (`Unk_5c |= 0x02`) and **`object_freeMemory` called directly from the update** (no CurState 8 detour).

**Observed** (watching writes to 0x020348DC): frame 72 `Unk_5c` 0x0C→0x1C; frame 89 → 0x1D; frame 122 → 0x1F and freed. Round 2: 1224 / 1241 / 1274.
- The screen fade is stepped outside the object system (it is not in `battle_8007800`). In the dump, `eScreenFade.Unk_03` goes 1→0 between frames 88 and 89, which gives a **17-tick** delay from `SetScreenFade` to the 0x01 bit.
- UNCERTAIN: fade type 8 and other speeds were not observed. A headless engine can model this as "bit 0x01 is set on the 17th T4#2 update after its init tick".

**Consumers:**
- The player's action 0 (intro, `sub_80163B4`/`sub_8016460`):
  - The player whose `Alliance == BattleState.Unk_0d` is made visible at once.
  - The other player enqueues itself (`sub_800AA1A(0)`), waits for bit 0x01 and for being queue head (`sub_800AA06(0)`), fades in over 16×2 ticks, then dequeues (`sub_800AA40(0)`).
  - Both players then wait for bit 0x02.
- The battle-mode handler (`sub_80091F0`) waits for bit 0x02 before advancing.

The chain gives: fade done at 89, fade-in of the non-local player 89→121, queue empty seen by T4#2 at 122, which is first in the list and therefore sees it before the players run.

### A.5 T4#8: buster charge glow (`sub_80E0DF0`)

**Spawn** `sub_80E0F02`, called from player init `sub_80172F0` with `r4 = AIDataPtr`, `r7 = AIData + 0x58`:
- `object_spawnType4(8, r1..r3 garbage, Params = r4)`, so Params = the AIData pointer (trace: 0x02034080 / 0x02034180).
- `RelatedObject1Ptr = player`, `*r7 = obj` (`AIData.Unk_58`), `ExtraVars[0x60] = r7`, `Flags |= 0x04`.
- Initial X/Y/Z are leftover r1/r2/r3 from `sub_80172F0`. They stay until the first **unpaused** update (frame 593 in round 1, when the battle unpauses). Verified with a write watch on 0x0203696C from frame 72: the spawn stores r1..r3 = [0x02034880, 1, 6] for the first player (alliance 1) and [0x02034880, 0, 6] for the second (alliance 0).

**Exact provenance** of the leftover registers. The code path is `sub_801DC06` → `GetBattleEffects` → `mov r1,#8` → [if `effects & 8`: `battle_networkInvert(alliance)` → [if nonzero: `sub_801DC36`]] → `sub_80141F4` → `mov r7,#0x58; add r7,r4` → `bl sub_80E0F02`.
- **X = r1.** In PvP (`effects & 8`), `battle_networkInvert` loads r1 = `eBattleState` (0x02034880), and nothing later touches r1. Without `effects & 8` it would be r1 = 8.
- **Y = r2.**
  - If `alliance ^ BattleState.Unk_0d == 0`: r2 = `BattleState.Unk_0d` (loaded by `battle_networkInvert`).
  - Otherwise `sub_801DC36` runs. It walks the 6×8-byte table `dword_20352E0`, and r2 = byte 0 of the last entry examined. That is 1 for the player's own entry (set by `sub_801DB84`). If no entry matched, r2 = byte 0 of entry 5.
- **Z = r3** = the loop counter left by the last table walk: `sub_801DC36` when it ran, else `sub_801DC06`. It equals `6 − i`, where i is the index of the player's entry in `dword_20352E0`, or 0 if not found.
  - In this match both players got i = 0. The first player's entry 0 was created by `sub_801DB84` and then cleared by its `sub_801DC36` before the second player initialized and reused entry 0.
- **Non-default panel pattern.** `sub_80141F4` returns early for panel patterns 0x38/0x30/0x3C and in the DustMan mode. Otherwise it calls `sub_8010312(0x400)`, which clobbers r1 = `AIData.Unk_48 | 0x400` (the new value) and r3 = AIData pointer.
- **Params = r4** = `player.AIDataPtr`, loaded (callee-saved).

**ExtraVars:** +0x60 slot pointer, +0x64 enable (1 at init; `sub_80E0F22`/`sub_80E0F28` set 0/1 from other T4 kinds), +0x6C loaded sprite id, +0x74 current charge level, +0x78 previous level.

**CurState 0** `sub_80E0E10`: enable = 1, CurState = 4, then the update.

**CurState 4** `sub_80E0E20`:
- If paused: clear VISIBLE if `owner.AIData.Unk_1e == 0`, then return. Position and sprite are untouched.
- If `*slot == 0`: CurState = 8 and return.
- VISIBLE is set and then cleared if any of these holds:
  - `sub_800EB6C(owner.alliance)` returns 0 (viewer-side blindness check).
  - enable == 0.
  - `object_isValidPanel(owner.PanelXY)` fails.
  - by `owner.AIData.Unk_1e`: 0 → hide; 1 or ≥3 → hide if `battle_networkInvert(owner.alliance)`; 2 → keep.
- `sub_80E0F2E(Unk_1e)`: if the wanted sprite (cat 0x14, idx 0x15 when `Unk_1e == 1`, else 8) differs from the one loaded, `sprite_load` and `CurAnimCopy = 0xFF`.
- `prev = cur`, `cur = CurAnim = owner.AIData.Unk_1d` (**charge level**, written by the player's buster code), and hide if 0.
- `sub_80E0F5E`: charge sounds (0→1 plays `SOUND_BUSTER_CHARGE`, 1→2 plays `SOUND_UNK_72`), only for the non-inverted side unless `Unk_1e == 2`.
- `X = owner.X + dx<<16`, `Z = owner.Z + dz<<16` using `sub_8018810(owner.NameID, 0, alliance, flip)`; Y = owner.Y.
- `object_updateSprite`.

**Death:** deletion phase 0 `sub_801741C` zeroes `AIData.Unk_58`. T4#8 sets CurState 8 in its update the same tick (it is after the player) and is freed the next tick. At frame 939 the free comes after the player's phase-1 explosion spawns, which is why the explosions take slots 3 and 4 instead of reusing slot 1.

**Verdict:** visual. The only inputs are `AIData.Unk_1d`/`Unk_1e`, owner panel/pos/flags and blindness. The only outputs are its own fields and sound.

### A.6 T4#0x48: GunDelSol target sprite (`sub_80E5C2C`)

**Spawn** `sub_80E5D12`:
- `object_spawnType4(0x48, X, Y, Z, Params)`, then `RelatedObject1Ptr = r5` (spawner), `AllianceAndDirectionFlip` copied from the spawner.
- Velocities are set from the spawn position: `XVel = X`, `YVel = Y`, `ZVel = Z` (used as an offset from the owner).
- `ExtraVars[0x60] = r7` and `*r7 = obj` (if r7 ≠ 0).

GunDelSol (`sub_80EDB14`) calls it with:
- X = frontDirection·80<<16, Y = Z = 0 (80 px = 2 panels ahead).
- Params from `byte_80EDBB8[Unk_03·2 + (navi-stat 0x22 ≠ 0 ? 8 : 0)]` (u16), giving 0x0200 here: Param1 = 0 sprite entry (`dword_80E5C28`: cat 0x0C idx 0x3C), Param2 = 2 palette.
- r7 = `&player.RelatedObject1Ptr`, and the return value is also stored in `player.RelatedObject1Ptr`.

**CurState 0** `sub_80E5C4C`: `sprite_load`, anim 0, load, `sprite_update` once, flip, palette, `Timer2 = 0`, VISIBLE, CurState 4, then the update.

**CurState 4** `sub_80E5CA0`:
- If `Param3 ≠ 1 && dimming`: hide and return.
- Show.
- If `*slot == 0`: hide, CurState 8 (freed next tick).
- Otherwise: if `Timer2 % 11 == 0` play SE 0xF9; `Timer2++`; `pos = owner.pos + (XVel, YVel, ZVel)`; `object_updateSprite`.

Flags stay at the T4 default (trace 0x13), so it **does not run while paused**.

**End:** the owner calls `sub_80E5D3E(obj)` (CurState = 8). The object runs later in the same tick and frees itself immediately. Observed spawn 645 → freed 766 (121 ticks = GunDelSol firing counter 0x78 + 1). Reaction/deletion actions instead zero `player.RelatedObject1Ptr`, which leads to CurState 8 in the same tick and freed on the next.

### A.7 Round-2 kinds: T1#0x57, T4#0x0A, T4#0x0F, T4#0x28

Context: in round 2, player alliance 0 performs a form change (Beast Out). The path is `sub_8017BC0` → `sub_8014A38` (form change, dispatched on the pending form from `sub_801595E`; forms 0xB/0xC use the table `off_8014AC8`), with phases:
- `sub_8014D70`: T4#0 effect 0x2E with Timer 0x36, frame 1802.
- `sub_8014E08`, frame 1857: sets navi-stat 0x2C = form (0x0C here), then spawns T4#0x0A and calls `sub_8011268(form)`, which spawns T1#0x57.
- `sub_80144C0` → `sub_8014536` → `sub_8014606`: spawns T4#0x0F (frame 1866).

Later, the Beast Out chip-use routine (`sub_80EAD9C` → `sub_80EAE28` → `sub_80EAFC2`) spawns T4#0x28 ×2 at 2342. Slots come from the bitfield watch.

**T1#0x57** `sub_80C4530` (0x080C4530): a secondary sprite layer glued to an owner.
- **Spawner** `sub_80C468C`: `object_spawnType1(0x57)`, `RelatedObject1Ptr = r5` (owner), alliance copied, `ExtraVars = r2`, `ExtraVars+4 = r3`, `Flags |= 0x04`.
- **Form overlay:** `sub_8011366` / `loc_8011368` passes `r4 = 0x0A0C | Param3<<16` (Param1 = category byte offset 0x0C, Param2 = sprite 0x0A), `r2 = 1`, and stores the object in `owner.RelatedObject2Ptr`. Forms 1–10 instead spawn a different kind via `sub_80C44A8`.
- **Afterimage layer:** T4#0x28 creates one through `sub_8010DF6`.
- **Init** `sub_80C4550`: `sprite_load(0x80, Param1, Param2)`, `CurAnim = CurAnimCopy = owner.CurAnim + Param4`, then load + `sprite_update`.
- **Update** `sub_80C458C`, every tick:
  - `CurAnim = owner.CurAnim + Param4`. If that differs from CurAnimCopy it calls `sprite_setAnimation` **without updating CurAnimCopy**, so the animation restarts every tick until the sprite step below records it (while paused, with Param3 = 0, that step never comes).
  - `pos = owner.pos`, minus 1 px on Y and Z when ExtraVars ≠ 0.
  - Copies VISIBLE, palette, shader, final palette, mosaic/alpha and flip from the owner.
  - Action 0 waits for `Unk_5c` bit 0x02 and then clears header flag 0x04 unless Param3 ≠ 0 (trace flags 0x13 at 1857). Action 4 steps the sprite with gating chosen by Param3 (0: `object_updateSprite` unless dimmed; 1: `object_updateSpriteTimestop`; 2: `sub_801BCD0`). If ExtraVars+0xC ≠ 0 and the owner has flags `0x100800` (DRAG | PARALYZED), it skips.
- **Other writers:** `sub_80C44D2` (the owner's `sub_8011450` after an animation change, and MegaMan's flinch/drag hook `sub_80F06CE`) sets CurAnimCopy = 0xFF and steps the overlay's sprite at once with `sub_801BCD0`. The form change sets Param3 = 1 and flags |= 0x14 on an existing overlay (`sub_8014D08`) and Param3 = 0 on the new one (`sub_8014E08`). `sub_80C46C0` sets ExtraVars+8 = 1 (palette from the navi's mood; render only).
- **Beast Out spawn:** the object is inserted after the navi (spawned during its update) with flag 0x04, so its init runs in the same, paused tick: flags 0x13, anim 0, pos = navi pos − (0, 1, 1) px. From then on it does not run until the pause ends.
- **Lifetime:** freed via CurState 8, set through `sub_80C46B0(obj)` by the owner. The form overlay lived 1857 → battle-end `FreeAll` at 2521. The afterimage layers were freed at 2353/2361, in the same tick their T4#0x28 was destroyed (they come after it in the list).
- **Verdict:** visual. It writes only itself. Slot and list position only.

**T4#0x0A** `sub_80E10A4`: screen palette flash.
- **Spawner** `sub_80E11E0`: `object_spawnType4(0x0A, r1..r3, r4)`, `ExtraVars = r7`, `Flags |= 0x14`.
- **Spawn values:** `sub_8014E08` passes `r4 = 0x00030E00`. The resulting fields are Param1 = 0 (variant `sub_80E10C0`), Param2 = 14 (duration), Param3 = 3 (bit 0 = run while dimmed, bit 1 = ignore pause). Without bit 1 it does nothing while paused (nor, without bit 0, while dimmed) except hold the palette; CurState stays 0 until it first runs. Its X/Y/Z are leftover registers from `SetBattleNaviStatsByte_AllianceFromBattleObject`: X = `eBattleNaviStats0` (0x0203CE00), Y = the form value written (0x0C), Z = earlier r3 (observed 1).
- **Behaviour:** the handler dispatches on **Param1, not CurState**. On the first call it sets `Timer2 = Param2`, `Timer = 0`, CurState = 4. Then, every tick, `Timer2 -= 1`; while ≥ 0 it writes palette-transform entries (`sub_8002378` into `iPalette3001B60`, blinking every 4 ticks); when it goes < 0 it calls `Terminate_ePalette20097a0_Transform(0x14)` and **`object_freeMemory` directly**.
- **Lifetime:** spawn T → freed at T + Param2 (1857 → 1871).
- **Verdict:** visual (palette RAM only). Slot and list position only. It has no sprite, so flag 0x08 stays set (trace flags 0x1D).

**T4#0x0F** `sub_80E1520`: Beast Out lock-on marker. **Gameplay-relevant.**
- **Spawner** `sub_80E1620`, called from `sub_8014606` only if `AIData.Unk_40 == 0`: `RelatedObject1Ptr = owner`, `ExtraVars = r7 = &AIData.Unk_40`, `*r7 = obj`, flip copied, `Flags |= 0x04`.
- **Init** `sub_80E1540`: loads sprite (0x0C, 9) and sets VISIBLE.
- **Update** `sub_80E1566`:
  - VISIBLE only if navi-stat 0x29 == 0, form in [0xB, 0x18], and `!battle_networkInvert(alliance)`.
  - During dimming: return (no movement).
  - If `battle_isBattleOver()` or `*slot == 0`: **zero `*slot` itself and `object_freeMemory` directly**.
  - Unless frozen (ExtraVars+4 ≠ 0; set/cleared by `sub_80E1654`/`sub_80E1662`):
    - Picks a target with `sub_80E1670`. `object_getEnemyByNameRange` lists the opposing side's four alive-actor slots (BattleState+0x80 + 0x10·side) in slot order, keeping NameIDs 0..0x1C3. `sub_80E16CC` then tries three column ranges relative to the owner's PanelX: ahead of it (`[x+1, 6]` when facing right, `[1, x−1]` when facing left), then behind it, then its own column. The first range with a candidate wins. With several candidates in a range, `sub_80E1730`/`sub_80E175C`/`sub_80E17AC` break the tie by row and distance relative to the owner (not needed in PvP, where each side has one navi). If there is no candidate at all, the result is 0 and the code reads through a null pointer.
    - `pos = target.pos + sub_8018810(target.NameID, 0x11, …) offsets (X, Z) + (0, 8, 8) px`. The offsets are 0 if target NameID ∈ [0x173, 0x178] and target.CurAnim == 0x4F.
  - `object_setPanelsFromCoordinates` recomputes its **own PanelX/PanelY** from pos, and it hides if that panel is invalid.
  - `object_updateSpriteTimestop`.
- **Why it matters:** `sub_80E164A` returns this object's PanelX/PanelY, and it is read by chip-use code as the Beast Out target panel: `sub_80EAE28` (passes it to `ho_8026554`), and 13920. So its target choice and panel computation feed gameplay.
- **Lifetime:** spawned 1866 into slot 0, freed at 2346 when the battle became over (the only write to `AIData.Unk_40` at 2346 is its own self-zeroing). `sub_801562C` also zeroes `AIData.Unk_40`, presumably when the form ends.
- Observed: spawned at 1866 with flags 0x17 at (0x370000, 0x240000, 0x130000), panel (5,2): the opponent at x = 0x3C0000 plus attach point 0x11 of NameID 0x1A0 facing left (−5, 0xB) and (0, 8, 8) px. While paused it keeps running (flag 0x04), recomputing its position; `object_updateSpriteTimestop` does nothing while paused. No RNG.
- The multi-candidate tie-break is code-derived only.

**T4#0x28** `sub_80E32B8`: Beast Out dash afterimage.
- **Spawner** `sub_80E33FA`: `RelatedObject1Ptr = r5`, `ExtraVars = r6`, `ExtraVars+4 = r7`, alliance copied, `Flags |= 0x04`.
- **Spawn values:** `sub_80EAFC2` passes:
  - r4 = `((flip<<8) + (0xF, or 0 if NameID−0xFF ∈ [0xA2, 0xAC])) << 16 | 0xFF`, giving Param1 = 0xFF ("copy the owner's sprite"), Param3 = anim, Param4 = flip. Observed params 0x000F00FF.
  - r6 = 0x83E0 (color shader).
  - r7 = `0x01010014 − r0` (u16 lifetime `20 − r0`; bytes 2/3 are shadow flags).
  - Afterwards `sub_80E341E` sets ExtraVars+0xC = 1 if the owner is in form 0xB..0x18, else 2.
  - The step sword (action 0x13) calls `sub_80E33FA` itself: r6 = 0xFFE0, r7 = 0x01010014 (the step, anim 0) or
    0x0101001E (the swing, anim 5), and a third with its own sprite: Param1/Param2 = the blade attachment's sprite
    (`byte_80B8BD4`), r7 = 0x1E (no ground shadow). Nothing sets ExtraVars+0xC, so nothing ends them early. The engine
    keeps it Rust (`kinds::afterimage`); content spawns one with `battle.afterimage`.
  - ExtraVars+6 = 0 → `sprite_noShadow`; else `sprite_hasShadow`, and `sprite_removeShadow` if ExtraVars+7 = 0.
  - An afterimage with its own sprite keeps NameID 0 (a virus record: no layer, no teardown). A copy wears the layer
    its NameID's record's AI index gives (`off_8010EA4`, `off_8010F08` after it: a Cross's body overlay, the Falzar
    beast head...), forced in front by `sub_80C4526`.
- **Init** `sub_80E32D8`: with Param1 = 0xFF it copies `owner.NameID`, loads that sprite, and creates a T1#0x57 layer through `sub_8010DF6` (stored in its RelatedObject2Ptr). It then sets anim = Param3, `Timer2 = lifetime`, `Timer = 0`, and runs the update. The layer comes from the NameID's actor record: `sub_8010DF6(record[1], record[2], 0)` indexes the same per-(type, AI index) tables as the form overlays, so a Falzar Beast afterimage (NameID 0x1B7, AI index 36) gets the beast head (`sub_8011366`: T1#0x57, nudged 1 px, owner = the afterimage). The sprite comes from `sub_800F26C`, which for player NameIDs gives the form's (or link navi's) battle sprite. Teardown at destroy is `sub_8011044(record[1], record[2], 1)`, the per-NameID death-hook table (`sub_801140E` for the beast head: CurState 8).
- **Update** `sub_80E336E`:
  - Destroy if (ExtraVars+0xC == 1 and the owner's form ∉ [0xB, 0x18]) or (== 2 and `owner.CurAction < 0x10`).
  - If paused: return.
  - `Timer += 1`; if `Timer >= Timer2`, destroy.
  - Otherwise `sprite_update`, VISIBLE, and blink (hidden when `(Timer & 2) == 0`) unless ExtraVars+8 ≠ 0.
  - Destroy = CurState 8 **plus an immediate call** to `sub_80E33D2`: it sets 0x04 on the layer, calls `sub_8011044(form, 1)` to tear down the layer, then `object_freeMemory`. So the object is freed in the same tick.
- **Lifetime:** freed at spawn + Timer2 − 1. Observed 2342 → 2353 (Timer2 12) and 2342 → 2361 (Timer2 20).
- **Verdict:** visual. Slot and list position only, plus the early-destroy rule that reads the owner's form/CurAction.

Note: the routine coverage record used here covers only round 1 of the match.

## 11. Sprite/animation state and what logic depends on it (sections S.x)

### S.1 Data layout

**Sprite block location.** Each battle object holds an `ObjectSprite` block at `obj + (TypeAndSpriteOffset & 0xF0)`:
- T1/T3: +0x90, header byte 2 = 0x91/0x93.
- T4: +0x80, byte 2 = 0x84.

These values are set per slot by `InitializeStructsOfObjectType`. `SpawnBattleObjectCommon` zero-fills only +4..+0x90 (T1/T3) or +4..+0x80 (T4), so **the sprite block is not cleared on spawn**. `sprite_load` → `sprite_initialize` resets it.

**Logic-relevant sprite fields** (offsets within the sprite block):

| Off | Name | Meaning |
|---|---|---|
| 0x00 | Unk_00 | current animation index (`sprite_setAnimation` writes it) |
| 0x01 | Unk_01 | countdown of the current frame (u8) |
| 0x02 | Unk_02 | flags of the current frame: 0x80 = last frame, 0x40 = loop to first frame |
| 0x03 | Unk_03 | format/shadow bits. Bit 7 set = battle format; every battle `sprite_load` call passes r0 = 0x80, which is stored here |
| 0x18 | Unk_18 | base = sprite data + 4 |
| 0x1C | Unk_1c | pointer to the current frame record |

Everything else (0x04 palette, 0x05, 0x10–0x17, 0x20 OAM ptr, 0x24–0x34) is render-only.

**Object-level fields:**
- `CurAnim` (+0x10): logic's requested animation.
- `CurAnimCopy` (+0x11): the animation last loaded into the sprite. 0xFF forces a restart.
- `PreventAnim` (+0x18): freezes the sprite of objects that have collision data.

**ROM data:**
- `sprite_load(0x80, catOff, idx)` (0x080026E4):
  - Clears header flag 0x08 (STOP_SPRITE_UPDATE, which spawn sets by default).
  - Looks up the decompressed-sprite cache first (`sub_8002986`: 12 entries at `byte_200DCA0`, key `catOff<<8 | idx`).
  - Otherwise uses `SpritePointersList` (0x08031CC4) `[catOff]` (catOff is a byte offset, i.e. entry catOff/4) `[idx·4]`.
  - A pointer with bit 31 set is LZ77-compressed and must be in the cache; otherwise `spriteWhiteDot` (0x084E0554) is used.
  - In the cache, the decompressed image has a 4-byte size prefix, so base = buffer + 8.
- In the battle format (Unk_03 bit 7):
  - Animation a's first frame is at `base + u32[base + 4a]`.
  - Frame records are **0x14 bytes**:
    - +0x00 tile data offset
    - +0x04 palette offset
    - +0x08 OAM list offset
    - +0x0C (render)
    - **+0x10 u8 duration in ticks**
    - **+0x12 u8 flags**
  - The next frame is at +0x14.

### S.2 Stepping algorithm (bit-exact)

`sprite_loadAnimationData` = `sub_3006730`, `sprite_update` = `sub_3006792`.

```
load(anim):                    // after sprite_setAnimation(anim)
  frame = first_frame(anim); cnt = frame.dur; flags = frame.flags
update():                      // one call = one "tick" of the sprite
  loop {
    cnt = cnt - 1 (u8 store; test on the un-truncated value)
    if old cnt != 0 { return }          // i.e. new value >= 0
    if flags & 0x80 {                   // leaving the last frame
      if flags & 0x40 { load(anim) }    // loop: restart and continue the loop
      else { cnt = 1 }                  // hold: next iteration makes cnt 0 and returns
    } else { frame += 1; cnt = frame.dur; flags = frame.flags }
  }
get_frame_parameters():        // sprite_getFrameParameters 0x08002DEA,
  r0 = (cnt == 0) ? flags : flags & ~0xC0 ;  r1 = r2 = anim
```

Consequences:
- A frame of duration d is current for exactly d `update()` calls, counting the call right after `load`.
- End-of-animation `(r0 & 0x80)` is visible only when on the last frame with `cnt == 0`, i.e. after `S = Σ durations` updates since the load.
- A non-looping animation then stays on its last frame with `cnt` pinned at 0, so it reports 0x80 on every later query.
- A looping one (0xC0) reports it for one tick per cycle.
- Frames with duration 0 are skipped within one update.

This was verified on T4#0 slots with durations 1 and 3 (frames 939–962 and 1802–1857, including the restart after the 0xC0 frame).

### S.3 Who steps sprites, and when

Each wrapper loads `CurAnim` if it differs from `CurAnimCopy`, then does one `update()`. All of them skip if the object is not ACTIVE (0x01) or has STOP_SPRITE_UPDATE (0x08).

| Wrapper | Address | Extra gating |
|---|---|---|
| `object_updateSprite` | 0x0801BBAC | skip if paused; skip if dimmed unless header 0x10; skip if `CollisionDataPtr ≠ 0 && PreventAnim ≠ 0` |
| `sub_801BCF4` | 0x0801BCF4 | identical to `object_updateSprite`. Called by the player handler `sub_80EA460` after its CurState handler, every tick and every CurState |
| `object_updateSpriteTimestop` | 0x0801BBF4 | skip if paused (no dimming or PreventAnim check) |
| `object_updateSpritePaused` | 0x0801BCA6 | skip if dimmed only (steps during pause) |
| bare `sprite_update` | 0x080026C4 | none (T4#0 update and T4#0/T4#0x48 init) |

`object_setAnimation(a)` (0x0800F2B6) sets `CurAnim = a, CurAnimCopy = 0xFF`, which restarts the animation even if it is the same index. Writing `CurAnim` directly restarts only on a change. The change is applied the next time the object's wrapper runs, which is the same tick when the writer is the object itself or runs earlier in the list.

### S.4 What gameplay logic reads animation state

- **Whole-match coverage** (routines run over the full machgun battle): `sprite_getFrameParameters` was called 163 times, **all from T4#0's `sub_80E05C4`**. It is the only reader of animation-stepper state in this match.
- **Player actions:** no other executed battle code reads sprite fields (a grep for sprite-block offsets in covered functions found none). Idle, move, buster, charge, flinch, deletion and the GunDelSol routine are all timed by object `Timer` fields or AIAttackVars counters.
- **`CurAnim` readers:** reads of `CurAnim` in covered logic (`sub_8014E08`, `sub_80C4550`/`sub_80C458C`, `sub_80E1566`) read the logic field, not the stepper.
- **Chip code in general:** `sprite_getFrameParameters` has 126 static call sites (121 in battle code, the rest overworld). 115 test 0x80 (end of animation), 5 test 0xC0 and 1 tests 0x40. Twelve are player-side chip or form attack routines in 0x080EA000–0x080F2400:
  - `sub_80EBB98`, `sub_80ECEBC`, `sub_80ED8E0`, `sub_80EDF78`
  - `sub_80EE060`, `sub_80EE50C`, `sub_80EE860`, `sub_80EE89C`
  - `sub_80EF208`, `sub_80F2180`, `sub_80F21FC`, `sub_80F2290`

  **Those chips' action durations depend on ROM animation data.**

**What a headless engine must model:**
1. Per object, the animation stepper state `{anim, frame index, cnt, flags}` plus `CurAnim`/`CurAnimCopy`/`PreventAnim`. Use the exact wrapper gating from S.3 and the algorithm from S.2. Without the gating, `CurAnimCopy` (and hence "restart" behaviour) desyncs.
2. A ROM-extracted table `(catOff, idx, anim) → [(dur, flags)]` covering at least:
   - T4#0 effect ids (lifetime S, table in §A.3).
   - Every chip or virus animation whose handler calls `sprite_getFrameParameters`.

   Extraction needs LZ77 decompression for compressed sprites (base = +8 in the decompressed buffer).
3. For the player's own non-chip actions, animation state never feeds back into logic. Stepping it is needed only to keep `CurAnimCopy` exact and for chips that later query it. Modeling it uniformly for all objects is cheap and the safest choice.

**UNCERTAIN:**
- (1) The claim that the player core never depends on animation rests on this match's coverage. Actions not exercised here (e.g., other hit reactions, Cross/Beast forms) were not checked beyond the static caller list above.
- (2) The cache-miss fallback to `spriteWhiteDot` for compressed sprites assumes battle init always preloads every compressed sprite used (`sub_80029A8`). A miss would change animation lengths.
- (3) The low six flag bits (+0x12 & 0x3F) and frame byte +0x11 were not observed to matter.

## 12. The player actor (PvP MegaMan)

The PvP player is a T1 object with Index 0 whose AIData.ActorType is 2 (NameID 0x1A0, AIIndex 0 for MegaMan without Cross/Beast). Chapter map:

| Part | Content |
|---|---|
| 12.0 | lifecycle and action table (this page) |
| 12.1 | spawn (`sub_800753C`) |
| 12.2 | CurState 0 init (`sub_80172F0`) and the resulting state |
| 12.M | per-tick pipeline, input, entry/idle actions, movement, panel rules (sections M0-M8) |
| 12.B | buster, charged shot, chip-use dispatch (sections B1-B11) |
| 12.H | damage intake, hit reactions, status effects, deletion, camera shake (sections H1-H7) |
| 12.8 | CurState 8 (destroy) |

### 12.0 Lifecycle and action table

`sub_80EA460`: `off_80EA478[CurState]` = {0: `sub_80172F0` init, 4: `sub_80EA484` update, 8: `sub_8016C4E` destroy}, then **always** `sub_801BCF4` (sprite step, §S.3).

Life of a player in a PvP round (trace frames for round 1, alliance 0 = local P1):

| Frames | CurState / CurAction | Meaning |
|---|---|---|
| 72 | 0 -> 4 / 0 | spawned and initialised in the same tick |
| 73-121 | 4 / 0 | battle entry: local player visible at once, remote player fades in (§M4.1) |
| 122-592 | 4 / 1 | waiting; battle paused (custom screen); action 1 switches to 8 on the first unpaused tick |
| 593- | 4 / 8 | normal control ("idle controller"); every other action returns here |
| e.g. 623-634 | 4 / 0x10 | one-panel move (§M6) |
| e.g. 638-776 | 4 / 0x37 | chip action (GunDelSol) |
| 938 (loser) | 4 / 2 | deletion (§H6), CurState 8 at t0+54 |

Action index (CurAction is an index here, not a byte offset; `sub_801B9E6`):

| CurAction | Handler | Meaning | Details |
|---|---|---|---|
| 0 | `sub_8016380` | battle entry / appear | §M4.1 |
| 1 | `sub_8017888` | transition to 8 | §M4.3 |
| 2 | `sub_80173F4` | deletion | §H6 |
| 3 | `sub_80174FE` | flinch (24 ticks) | §H4.2 |
| 4 | `sub_80175B8` | paralysis | §H5 |
| 5 | `sub_80178B6` | drag / knockback | §H4.3 |
| 6 | `sub_8017688` | freeze | §H5 |
| 7 | `sub_8017768` | bubble | §H5 |
| 8 | `sub_80EA734` -> `sub_80F0354` | normal control: decides buster / charge / chip / move | §M5, §B5 |
| >= 0x10 | `JumpTable80EAC60[a-0x10]` (0x080EAC60), or `sub_80EAD9C` if AIAttackVars.Unk_1d == 1 | attacks: 0x10 move, 0x11 buster, 0x16 charged shot, 0x3B turn, chip actions = ChipData[0x0B] (0x37 GunDelSol, 0x52 ...) | §M6, §B6-B9 |

Entries 9..0xF of the per-form table do not exist for AIIndex 0 (the table `off_80EA52C` has 9 entries); other forms' tables (`off_80EA550`...) add entries 9/10 (and 11/12 for form 10).

### 12.1 Spawning the players

The battle-mode handler's intro sub-state (`sub_80091F0` -> `sub_8007358`) walks the entity list at `BattleSettings+0x0C` (`sub_8007368`). Entries are 4 bytes; byte0 high nibble = kind (0xF0 terminates), byte0 bit0 = alliance, byte1 = panel (x = byte1 & 7, y = byte1 >> 4). Kind 0 = player -> `sub_80073CC` -> `sub_800753C(x, y, alliance)`. The PvP settings of the reference match point to `byte_80B1992` = {01 25}, {00 22}, {F0}: **alliance 1 at (5,2) is spawned first (T1 slot 0, AIData slot 0, CollisionData slot 0), then alliance 0 at (2,2) (T1 slot 1, AIData slot 1, CollisionData slot 1)**. After the list, BattleState+0x12 = BattleState+0x04 (u16) and BattleState+0x80..0x9F is copied to +0xD0; then `sub_80AA88C` consumes **one `GetPositiveSignedRNG2`** (random drop chip for alliance-1 actors; for players it ends up writing nothing useful) — this is the only RNG call of the spawn sequence (seen at trace frames 72 and 1224).

`sub_800753C`:
1. `object_spawnType1(Index=0, X/Y/Z = garbage registers, Params=0)`; if it fails, return.
2. Alliance = alliance; PanelX/Y = FuturePanelX/Y = (x, y); (X, Y) = `object_getCoordinatesForPanels(x, y)` (16.16); Z = 0; Flags |= 0x04 (run while paused) -> 0x1D.
3. `object_createAIData` -> obj+0x58 (on failure `object_freeMemory` and return); AIData.ActorType = 2.
4. NameID = navi stats[0x29] + 0x1A0 (MegaMan: 0x1A0).
5. `sub_8007778`: put obj in the first empty of the 4 BattleState "alive actors" entries for its side (+0x80 + 0x10*alliance) and BattleState[4+alliance] += 1 (because AIData.Unk_02 = 0). If the side is full: free AIData and object.
6. From `sub_80182B4(NameID)` (3-byte record; for 0x1A0 = {0, 2, 0}): AIData.Version_16 = Version_17 = rec[0]; ActorType = rec[1] (= 2); AIIndex = rec[2] (= 0 for MegaMan without cross/beast); AIData.Unk_03 = `enemy_getStruct1(NameID)[2]`; AIData.Unk_0e = 0xFF; `sub_80077D2` (append NameID to BattleState per-side NameID list at +0x4C, count at +8+alliance).

The object is now linked at the tail with CurState 0. Because the spawn happens before `RunBattleObjectLogic` in the same tick, both players run their init that same tick (frame 72).

### 12.2 Player init: CurState 0 = `sub_80172F0`

Handler chain: T1#0 `sub_80B81EC` -> ActorType 2 -> `sub_80EA460` = `off_80EA478[CurState/4]` {`sub_80172F0`, `sub_80EA484`, `sub_8016C4E`}, **followed every tick (any state) by `sub_801BCF4`** (sprite/animation step, see §11).

Steps of `sub_80172F0`, in order ("sim" = affects gameplay state):

| # | Call | Effect | sim |
|---|---|---|---|
| 1 | `sub_800F35C` | per-form init hook `off_80EA9A0[AIIndex]` (ActorType 2 table) = `nullsub_105` for AIIndex 0 | no |
| 2 | stats[0x2C] = stats[0x17] | "Transformation" byte of the battle navi stats := stats[0x17] (0 in the reference match) | yes |
| 3 | `sub_800FC9E(stats[0x29], stats[0x2C])`, `sprite_load(0x80, ...)`, `sprite_loadAnimationData`, `sprite_hasShadow`, CurAnim = 0 (stored twice at +0x10; CurAnimCopy not written), `sprite_setAnimation(0)`, `sprite_loadAnimationData`, palette (`sub_801002C`), `sprite_setFlip(object_getFlip())` | sprite set-up; `sprite_load` clears header flag 0x08 (0x1D -> 0x15) | anim only |
| 4 | `sub_80142B0` | netbattle (BattleEffects & 8): obj.Damage = 10 | yes |
| 5 | `object_createCollisionData` | on failure: `object_freeMemory` and return (CurState stays 0) | yes |
| 6 | `object_setupCollisionData(cd, self=1, target=2, modifier = 3 in netbattle / 0 otherwise)` (modifier from `sub_80107C0`) | see §7.1 | yes |
| 7 | `sub_80141C8` | HP = MaxHP = stats[0x42] (u16); unless BattleEffects & 4: HP = stats[0x40]. If this is the local player: `sub_801E0BC` (HUD) | yes |
| 8 | `sub_8013892` | obj+0x2E = 10; in netbattle (or BattleEffects & 0x10000, or stats[0x0E] != 0xFF) stats[0x0E] (Mood) = 0x80 (`sub_8015C2C` returns the constant 0x80); if stats[0x06] (FstBarr) != 0: spawn first barrier (`sub_801A7CC`, `sub_80E0D98`) — **note: this path pops into r4, clobbering the AIData pointer that `sub_80172F0` keeps in r4 (used again at step 16 to compute the T4#8 slot address)**; if stats[0x21] == 0: AIData.Unk_32 = 0xFFFF (`sub_801443C`); then falls into `sub_801390C`'s tail: AIData.Unk_07 = stats[5], AIData.Unk_08 = stats[7]; clear ObjectFlags1 0x08000000; `sub_800EB08` (cd.Unk_28 = 0, clear INVULNERABLE); if AIData.Unk_50: `sub_80E5410`, Unk_50 = 0; then status flags from stats: FloatShoe 0x20 iff stats[0x1B] (and re-run `sub_801A082` with self type 0x10 instead of 1), AirShoe 0x10 iff stats[0x1C], AFFECTED_BY_ICE 0x02000000 always, Undershirt 0x40000 iff stats[0x1D], SuperArmor 0x20000 iff stats[0x23] | yes |
| 9 | `sub_801086C` | element: `sub_8019F8C(stats[0x29]==0 ? (stats[0x2C]==0 ? stats[0x10] : byte_80108B8[stats[0x2C]]) : byte_80108B8[stats[0x29]])`; SecondaryElementWeakness = `byte_80108D1[stats[0x2C] or stats[0x29]]` | yes |
| 10 | if stats[0x29] == 0: `sub_8015B22(stats[0x2C])` | NameID = 0x1A0 if 0 else stats[0x2C] + 0x1AB | yes |
| 11 | `sub_8011268(stats[0x2C])` | per-transformation set-up; `nullsub_42` for 0 | yes (forms) |
| 12 | `sub_80144C0` | full "status reset": `sub_801390C` (as in step 8 tail), zero 6 halfwords at hand+0x26, AIData+0x48 &= ~0x20, (netbattle & local player: remove the opponent's HUD entry), HitModifierBase = 3/0, Region = 1, `sub_8012EA8`, `sub_800FEEC` (AIData stats: Unk_04 = 0xFF (battle mode 9: stats[0x44]), Unk_06 = stats[4], `sub_800FFAA(stats[5])`, Unk_05 = stats[0x39], Unk_08 = stats[7], Unk_11 = 0xFF — transformed forms use `byte_8020354`), `sub_8014536` (per-form flags; nothing for form 0), `sub_801086C`, `sub_80142C2` (netbattle: cd.SelfDamage = 10) | yes |
| 13 | `sub_8013E58` | style/mood hook keyed by stats[0x1A]: **if stats[0x1A] is 9 or 10, one `GetRNG2` call** picks one of 4 variants; else `off_8013E9C[stats[0x1A]]` (nullsub for 0) | yes (RNG!) |
| 14 | `sub_801DB84`, `sub_8018856(3)`, `sub_801DC06`, netbattle & remote: `sub_801DC36` | HP-number HUD table at 0x020352E0 (6 x 8 bytes) | no |
| 15 | `sub_80141F4` | if the panel column pattern is not 0x38/0x30/0x3C (and not the Dustman minigame): AIData+0x48 |= 0x400 | yes |
| 16 | `sub_80E0F02` with r7 = AIData+0x58 | spawns the player's **T4#8** helper (inserted right after the player, runs this tick) and stores it at AIData+0x58 — see §A.5 | slot/list |
| 17 | `sub_800F378` | per-form hook `off_80EAA04[AIIndex]` = `nullsub_105` for 0 | no |
| 18 | if stats[0x2C] == 0: `sub_8010DD0` | `off_8010E0C[ActorType][AIIndex]` for NameID's record = `nullsub_42` for MegaMan | no |
| 19 | `sub_802DFC8` | zero the side's 0x1D0-byte battle struct (0x02036120+0x1D0*a); if battle flag-0x40 mode (`sub_800A8F8`): [0]=1, [0x0B]=0xFF, [0x10]=1, [0x11]=PanelX, [0x0E]=3, then `sub_802E07C` ([3]=0, [0x2A]=0, [0x50]=0, [0x18..0x23]=0xFF.., [2]=0xB4) | yes |
| 20 | `sub_8013FF8` | if stats[0x3D] = n != 0 and HP != 1: subtract n if HP >= n, else HP-1 (`object_subtractHP`) | yes |
| 21 | CurStateActionPhaseAndPhaseInitialized = 4 | CurState = 4, **CurAction = 0**, CurPhase = 0, PhaseInitialized = 0 | |

Resulting state for the reference match (dump at the end of frame 72, alliance 1 player; alliance 0 identical except as noted):

| Field | Value |
|---|---|
| Flags | 0x15 (ACTIVE, PAUSE_UPDATE, UPDATE_DURING_TIMESTOP); VISIBLE (0x02) is managed every tick from the next tick on (§M4.1, §H4.2) |
| Index/Type/ListIndex | 0 / 0x91 / 0 (alliance 0: 1) |
| State | [4,0,0,0] |
| PanelX/Y, Future | (5,2) (alliance 0: (2,2)) |
| Alliance / DirectionFlip | 1 / 0 |
| HP / MaxHP | 1000 / 1000 |
| NameID / Chip | 0x01A0 / 0xFFFF |
| Damage / obj+0x2E | 10 / 10 |
| X, Y, Z | (0x003C0000, 0x001C0000, 0) = (60.0, 28.0, 0); alliance 0: (-60.0, 28.0, 0) |
| CollisionDataPtr / AIDataPtr | 0x020384F0 / 0x02034080 (alliance 0: 0x02038598 / 0x02034180) |
| AIData | ActorType 2, AIIndex 0, Unk_03 1, Unk_04 0xFF, Unk_05 0xFF, Unk_07 1, Unk_08 0xFF, Unk_0e 0xFF, Unk_11 0xFF, +0x58 = its T4#8 (0x02036938 / 0x02036A00), +0x7C slot bit |
| CollisionData | Enabled 1, Region 1, Alliance 1, PanelXY (5,2), HitModifierBase 3, SelfDamage 10, SelfCollisionTypeFlags 0x04210080 (alliance 0 with FloatShoe: 0x08510080), TargetCollisionTypeFlags 0xAA800200 (alliance 0: 0x55800200), ParentObjectPtr = obj, ObjectFlags1 0x02000000 (alliance 0: 0x02000030 = FloatShoe+AirShoe+ice), CollisionIndexBit 0x80000000 (alliance 0: 0x40000000) |

### 12.M Per-tick pipeline, input, entry/idle actions and movement

Scope: the PvP player (T1 index 0, `AIData.ActorType == 2`, `AIData.AIIndex == 0`, i.e. plain MegaMan). This section covers input handling, the entry and idle actions, and single-panel movement. Buster, charge and chips are in 12.B, hit reactions in 12.H, sprites in chapter 11.

`r5` is always the object, `AIData = [r5+0x58]`, `AttackVars = AIData+0xA0`, `ObjectFlags1 = CollisionData+0x3C` (read with `object_getFlag`), `ObjectFlags2 = CollisionData+0x40`.

#### M0. Key facts

- Action 0 (`sub_8016380`) is the battle-entry "appear/fade-in" action, not idle. The idle state is **action 8**: `sub_80EA734` → `sub_80F0354`, the input "controller". The sequence is 0 → 1 → 8.
- **Movement is action 0x10.** The handler is `sub_80EB04C`, reached as `JumpTable80EAC60[0x10-0x10]`. A move is started from idle by a **held** d-pad direction, on the same tick the direction is first seen.
- **Move timing, verified twice in the trace (frames 623 and 1982).** Let F be the tick the move starts.

  | Tick | Event |
  |---|---|
  | F | Target panel is reserved. |
  | F+3 | `PanelX/Y` and the x/y coordinates jump to the new panel. |
  | F+8 | `MOVE_COMPLETE` is set and the animation returns to 0. |
  | F+12 | `CurAction` is set back to 8. |
  | F+13 | The controller runs again. The earliest next action (move, buster or chip) starts here. |

  So a held direction repeats a move every **13 ticks**.
- **Inputs are never mirrored.** "Right" means forward, toward the enemy, for both alliances. World dx = +1 for alliance 0 and −1 for alliance 1 (`object_getAllianceDirection`). `battle_networkInvert` only tells whether an object is the local player; it is used for HUD, sound and intro visuals.
- **No RNG is used on the normal movement or idle path.** Four RNG calls exist on side paths, gated by NaviCust bug stats or dimming; see chapter 13.

#### M1. Per-tick player pipeline (what runs, in order)

`RunBattleObjectLogic` calls the T1 index-0 handler `sub_80B81EC`. It dispatches on `AIData.ActorType`, and 2 selects `sub_80EA460`:

```
sub_80EA460: jump off_80EA478[CurState]   ; 0:sub_80172F0 init, 4:sub_80EA484 update, 8:sub_8016C4E destroy
             then sub_801BCF4             ; apply CurAnim change + sprite_update (§S.3)
```

`sub_80EA484` runs these steps every tick while `CurState == 4`:

| # | Call | What it does | Owner |
|---|---|---|---|
| 1 | `sub_8012E74` | If the battle is over, zero AIData+0x22..0x28. Otherwise, if **not paused**, run `sub_8012FC8` (buttons → intent bits in `AIData+0x44`, §M3.3) and `sub_8012EBC` (charge counter). While paused, no intents are generated. | 12.M / 12.B |
| 2 | `sub_8013DA0` | Returns immediately unless navi stats 0x24 and 0x21 are both nonzero; stat 0x24 is 0 in this match. Otherwise it is a 60-tick timer that calls `GetPositiveSignedRNG2` (emotion / beast behaviour). | other |
| 3 | `sub_801AC6C` | Removes this object's collision from the panel occupancy grid (`object_removeCollisionData`, only if `battle_getFlags & 1`). It then processes the hits accumulated since its last present (§7.3, §H2), and runs `sub_801A36A`, which consumes `MOVE_COMPLETE` and triggers ice/road slides (§M6.8). | 12.H |
| 4 | `sub_801AF44(off_80EA4C8[AIIndex])` | Status handling, then **action dispatch** (§M2). For AIIndex 0 the table is `off_80EA52C`. | 12.M / 12.H |
| 5 | `off_80EA93C[AIIndex]`, which is `sub_80F0608` for AIIndex 0 | Per-form hook. For base MegaMan its only effect is Z = 0 each tick (§B10). | 12.B |
| 6 | `sub_80107D4` | Skipped while dimmed. Otherwise decrements, saturating at 0, `AIData.Unk_19`, `AIData.Unk_15`, and three u16 counters at `sub_802E070(alliance)+0x2E/+0x3A/+0x3C`. | shared |
| 7 | `sub_80139C4` | Emotion/beast visual spawn when `sub_8015B54(alliance)==2`. Not seen in PvP. | other |
| 8 | `sub_80100EC` | Palette refresh only (visual). | sprite |
| 9 | `object_presentCollisionData` (only if not paused) | Re-registers the collision on its current collision panel. This re-sets the panel occupancy bits (§M7). | collision |

Consequence of steps 3 and 9: while another object runs its update this tick, the player's occupancy bits are present on the player's collision panel. They are absent only during the player's own update.

#### M2. `sub_801AF44`: the part that leads to action dispatch

The full status logic is in §H3. What matters for idle and movement:

1. If paused and `CurAction != 0`, skip straight to step 4. Only the pause handler runs.
2. The damage/status block runs in this order: `sub_801A42E`, `sub_801A4A6`, `sub_801A45C`, `sub_801A506`, `applyDamageToPlayer_801ba12`, `sub_801BADE`, then the DEAD check (flag 0x100) and the flinch, drag and push checks. Many of these override `CurAction`: 2, 3, 4, 5, 6, 0x30, 0x4C, and so on.
3. **Slide processing.** If `ObjectFlags2 & 0x10` (the flag is cleared here) or `ObjectFlags1 & 0x1000` (SLIDING), `sub_80166B6` runs. It advances an ice, road or push slide by one tick, before the action dispatch (§M6.8). Otherwise `Unk_1f = 0`.
4. Visual helpers run. Then:
   - Flag 0x100 (dead): go to dispatch.
   - Paused with `CurAction != 0`: `sub_8017BC0`, the pause-time request handler.
   - Dimming: `sub_8017AB4`.
   - Otherwise: **`sub_801B9E6(table)`**.

`sub_801B9E6` sets `r6 = AIData+0x80`, `r7 = AttackVars`, then dispatches:

- If `CurAction < 0x10`: `table[CurAction]`.
- If `CurAction >= 0x10` and `AttackVars.Unk_1d == 1`: `sub_80EAD9C`.
- Otherwise: `JumpTable80EAC60[CurAction-0x10]`.

The action handler therefore runs **after** the status checks in the same tick. A flinch set in step 2 pre-empts idle and movement that tick.

##### M2.1 Action table

See §12.0 (and §H4.1 for entry/exit conditions of the reaction actions).

##### M2.2 `object_setAttackN(action)` (`loc_8011698`)

Starting any action ≥ 0x10 uses this:

- `CurAction = action`, `CurPhase = 0`, `PhaseInitialized = 0` (a u16 write at +0xA).
- `AttackVars.Unk_00` (u16 attack phase) = 0.
- `AttackVars.Unk_1c = N`. N = 0..5 is the variant: 1 buster, 2 charge/chip, 3 special, 4 **move**.
- `sub_801011A`: `AttackVars.Unk_1d = 0`, `AttackVars.Unk_1e` (u16) = 0, then `sub_80E1662`.
  - `sub_80E1662` writes 0 to `[AIData.Unk_40]+0x64` and `+0x10`. For PvP players `AIData.Unk_40 == 0`, so these writes land in BIOS/ROM space and do nothing. Treat them as a no-op when the pointer is null.

No other AttackVars bytes are cleared. **Recommendation:** model AttackVars as a raw persistent 0x50-byte block. Some actions read bytes, such as `Unk_0d`, that earlier actions left behind.

##### M2.3 Leaving an action: `object_exitAttackState` / `sub_801171C`

`object_exitAttackState` first sets `CurAnim = 0`. Both entry points then do the following:

1. `AttackVars.Unk_1b = 0`.
2. **If `AttackVars.Unk_1c == 4` (move), skip to step 3.** Otherwise:
   - If `Unk_1c == 2`: `AIData.Unk_19 = AttackVars.Unk_05`.
   - If `Unk_1c == 3`: `AIData.Unk_15 = AttackVars.Unk_05`.
   - `AIData.Unk_1a = 0`.
   - Clear intent bits `0x1000003F` in `AIData+0x44`.
   - `sub_8012EA8`: `AIData.Unk_1d = Unk_1b = Unk_1e = 0`, and clear intent bits 0x60000 (the charge state).
   - Clear `ObjectFlags1` 0x400000.
3. `CurAction = 8`, `AttackVars.Unk_00 = 0`. **`CurPhase` and `PhaseInitialized` are not touched.** They are still 0 from `setAttackN`, so idle re-runs its phase-0 initialisation on the next tick.

Consequence: a move does **not** clear pending buster/chip intents or the charge. A buster or chip action clears them when it ends.

#### M3. Input path

##### M3.1 Link input records → AIData joypad fields

- There is one 8-byte record per alliance at `0x02036820 + 8*alliance`. The trace's `input[p]` is `[+2 held, +4 pressed, +6 released]`. **`input[0]` belongs to alliance 0, `input[1]` to alliance 1** (verified: `sub_8012DFC(0)` wrote P1's AIData at 0x02034180). Who fills the records is the link/flow layer.
- `sub_8012DFC(alliance)` is called for 0 and then 1 once per tick by the battle-flow substate, *before* `RunBattleObjectLogic`. In PvP the running substate is `sub_80080D2` (via `sub_8009158 → sub_800938A → sub_800801C`); the start substate `sub_8008064` and other modes' substates also call it. It does the following:
  - If the battle is over: zero `+0x22, +0x28, +0x24, +0x26`.
  - If navi stat 0x2C (Transformation) is 0x17 or 0x18: return without updating.
  - Otherwise, **recompute edges from the held word only.** The record's own pressed/released fields are ignored.

    ```
    prev = AIData[0x22]; AIData[0x28] = prev; AIData[0x22] = held
    AIData[0x24] = held & ~prev      // pressed
    AIData[0x26] = prev & ~held      // released
    ```
  - During dimming it also maintains a second set: `+0x2A` held, `+0x30` prev, `+0x2C` pressed, `+0x2E` released. Outside dimming those four are zeroed.
- True field meanings, which differ from the struct names: 0x22 = held, 0x24 = pressed, **0x26 = released** (named `JoypadUp`), **0x28 = previous held** (named `JoypadReleased`).
- The held word carries 0xFC00 in its upper bits, as the trace shows. Only the key bits matter: A=0x1, B=0x2, Select=0x4, Right=0x10, Left=0x20, Up=0x40, Down=0x80, R=0x100, L=0x200.

##### M3.2 No left/right mirroring

`battle_networkInvert(a)` returns `a XOR BattleState.Unk_0d`. `Unk_0d` is the local side: 0 on P1's GBA, 1 on P2's GBA; it also sets the camera mirror. Movement code does not use it. Direction semantics come from `object_getAllianceDirection(a) = 1 − 2a` applied to "Right = +1 forward" (§M6.2). Both GBAs therefore simulate identical state from raw keys, and only rendering is mirrored.

UNCERTAIN: the alliance-1 player never moved in this trace, so the dx = −1 mapping for alliance 1 is derived from code only.

##### M3.3 Intent bits (`AIData+0x44`), `sub_8012FC8`

This runs each unpaused tick at the start of the player update, in **every action**, not only idle. Bits are set here and consumed or cleared later. That is the only buffering mechanism: a button edge seen during a move is remembered in `+0x44` and acted on by the first idle tick after the move.

Bits relevant to idle priority (details in §B3):

| Bit | Source (this function) |
|---|---|
| 0x1 | Buster shot request |
| 0x2 | Charged shot request |
| 0x4 | Chip request (A) |
| 0x8 | Charged-chip request |
| 0x10 | B then Back within an 8-tick window (`AIData.Unk_13`). Only if `AIData.Unk_08 != 0xFF`; it is 0xFF for plain MegaMan, so this is never set in this match. |
| 0x1000 / 0x2000 | L or R pressed while `AIData.Unk_48 & 0x400`. Otherwise, L or R pressed with `battle_getFlags & 2` sets `battle_setFlags(0x10)` (custom-screen request) and **returns early, so no buster/chip intents are generated that tick**. The Select branch (0x2000000) also returns early. |
| 0x2000000 | Select pressed with `[sub_802E070(alliance)+0x28] >= 0x1500` |
| 0x10000000 | A pressed in battle mode 9 |
| 0x20000 / 0x40000 | Charge-state bits |

Bit 0x1 details: with `AIData.Unk_06 != 0xFF` and `(+0x44 & 3) == 0`, B is tested as pressed, or as **released** when `AIData.Unk_07 != 0xFF`. For plain MegaMan `Unk_06 = 0` (buster) and `Unk_07 = 1` (charge), so the buster fires on **B release**. It sets 0x2 instead when fully charged. See §B3-§B6.

The d-pad does **not** produce an intent bit. Movement reads `AIData+0x22` (held) directly in the controller (§M5.2).

#### M4. Action 0 (battle entry) and action 1

##### M4.1 Action 0, `sub_8016380`

If `AIData.Unk_02 == 0` (true for PvP players), the phase table `off_80163A8` is used. Otherwise `sub_80164A0`, a mid-battle appear with invulnerability and a T4 spawn, not used in PvP.

**Phase 0, `sub_80163B4`.**
- If `PhaseInitialized == 0`:
  - **Local player** (`Alliance == BattleState.Unk_0d`): set header flag 0x02 (visible); `CurPhase = 8`, `PI = 0`. Done.
  - **Remote player**: `sub_800AA1A(0)` appends `r5` to the 8-slot enemy fade-in queue; `PI = 4`; clear header flag 0x02.
- If `PI != 0`:
  - If `BattleState.Unk_5c & 1` and `sub_800AA06(0)` (this object is first in the fade-in queue): play sound 0x94; `Timer = 2`; `Timer2 = 0x10`; `sprite_setAlpha_8002c7a(0)`; `CurPhase = 4` (PI stays 4); return.
  - Otherwise clear header flag 0x02.

**Phase 4, `sub_801641A`, fade-in.**
- `Timer -= 1`. If it is not 0, return.
- `Timer = 2`, `Timer2 -= 1`.
- If `Timer2 != 0`: `sprite_setMosaicSize(Timer2)`, `sprite_setAlpha_8002c7a(16 − Timer2)`, set header flag 0x02.
- If `Timer2 == 0`: disable alpha, clear mosaic, `sub_800AA40(0)` (dequeue), `CurPhase = 8`, `PI = 0`.
- Total: 16 decrements × 2 ticks = 32 ticks.

**Phase 8, `sub_8016460`.** Wait until `BattleState.Unk_5c & 2`. The code is `lsr #2` followed by a carry test, which tests bit 1. Then:
- If battle mode is 6 or the object is remote: `sub_801DC7C(r0, r1)` (HUD). `r0 = −0x20, r1 = 3` when `0x49 <= NameID <= 0x4E`, else 0, 0.
- `CurAction = 1`, `CurPhase = 0`, `PI = 0`.

Observed in the trace (P1 local = alliance 0, P2 remote = alliance 1):

| Frame | P1 state | P2 state | Note |
|---|---|---|---|
| 72 | `[4,0,0,0]` | `[4,0,0,0]` | Spawned. |
| 73 | `[4,0,8,0]`, flags 0x15→0x17 | `[4,0,0,4]` | |
| 89 | | `[4,0,4,4]` | Fade starts. |
| 121 | | `[4,0,8,0]` | 32 ticks later. |
| 122 | `[4,1,0,0]` | `[4,1,0,0]` | Both switch. |

Intermediate values differ between local and remote, and the local side differs per GBA. An engine reproducing core-0 state must know `BattleState.Unk_0d`. The `Unk_5c` bits come from battle flow.

##### M4.2 While paused

During the custom screen `battle_isPaused` is true. `sub_801AF44` skips the action handler for `CurAction != 0` and runs `sub_8017BC0` instead. `sub_8012E74` produces no intents. So **action 1 persists through the whole first custom screen**: frames 122–592 in the trace.

`sub_8017BC0` first runs whatever pause-time action is in progress, by `Unk_48` (state) bit: 0x80 → `sub_8014A38` (form change, §12.9), 0x100 → `sub_8015614` (form revert), 0x1000 → `sub_802D714` (Cross change), 0x2000 → `sub_802D926` (AIAttackVars+3 = 0 first). Otherwise it starts one from an `Unk_44` request, clearing the request, setting the state bit and calling `object_setAttack0(0x1C)`: 0x4000 → 0x80; 0x40 → 0x100 (and it saves the state word in `Unk_5c` after zeroing it); 0x4000000 → 0x1000; 0x8000000 → 0x2000. The action starts running on the next tick. (The listing renders the literal 0x4000000 as `LCDControl` and 0x8000000 as a byte pool.)

##### M4.3 Action 1, `sub_8017888`

1. Optional: if `sub_800A8F8()` and `sub_80182B4(NameID)[1] == 2` and `AIData.Unk_40 == 0`, call `sub_80E1620`. Not taken for PvP; `Unk_40` stays 0 per the dump.
2. `CurAction = 8`, `CurPhase = 0`, `PI = 0`.

The trace shows action 8 appearing at frame 593, the first unpaused tick. The first idle tick is 594.

#### M5. Action 8: the idle controller

`sub_80EA734`:
- If the battle is over: end-of-battle branch (§H6).
- If `+0x44 & 0x8600`: `sub_801056A(…)`.
- If `+0x44 & 0x80000`: `object_setAttack0(0x49)`.
- Otherwise call `JumpTable80EA7B0[enemy_getStruct1(NameID)[4]]`. Every entry is `sub_80F0354`.

##### M5.1 `sub_80F0354`: phase timer, then decide

1. HUD only: if the object is local, `sub_801DA48(0x40)` or `sub_801DACC(0x40)` depending on `sub_800A772`. No simulation effect (UNCERTAIN: assumed UI-only).
2. **Phase 0.**
   - If `PI == 0`: `sub_801DA48(2)` (HUD), `Timer = 10`, `AIData.Unk_48 |= 0x10`, `Unk_48 &= ~0x40`, `PI = 4`.
   - Then `Timer -= 1`. If `Timer <= 0`: `CurPhase = 4`, `PI = 0`.
   - The phase does **not** gate input; execution always continues to the decision list.
   - `Unk_48 & 0x10` is the "idle window" bit that `sub_8012F3E` requires for charge to accumulate.
   - Trace: 594 PI=4 with Timer 9; 603 `[4,8,4,0]`. After the move: 636 Timer 9, …
3. **Decision list.** The first match starts an action and returns.

| Order | Condition | Result |
|---|---|---|
| a | Navi stat 0x2C ≥ 0x17 | Special-form path (`sub_802D322`). Not PvP MegaMan. |
| b | `sub_802E4E4` then `sub_802E4B8` returns 0xD or 9 | Select / cross special modes (`+0x44` 0x2000000 / 0x20000000). Out of scope. |
| c | `+0x44 & 0x600` | `sub_801056A(r0, 0, 0)` |
| d | `sub_8010660()` | Only if `GetBattleEffects & 8`, navi stat 0x0D bit 2, and `HP <= MaxHP/4`. One-shot NaviCust effect; spawns via `sub_80E90FE` and clears the bit. |
| e | `+0x44 & 0x20` | `object_setAttack1(0x16)` |
| f | `+0x44 & 0x1` | Buster: `sub_8011764` → `object_setAttack1(AIData.Unk_06 → action)` |
| g | `+0x44 & 0x2` | Charged shot: `sub_80117A4`; `setAttack2` if `0x21 <= AIData.Unk_07 <= 0x26`, else `setAttack1` |
| h | `+0x44 & 0x10` | `sub_8011790` → `object_setAttack3` |
| i | `+0x44 & 0x10000000` | `sub_801177A` → `object_setAttack1` |
| j | `sub_800FB54() != 0xFFFF` | Chip use. Requires `+0x44 & 0x1000C` and not SLIDING. See §B9. |
| k | `d = sub_800FA54() != 0` | **Move**: `sub_80116AE(d, sub_8010332(), sub_80103A8())`, then return |
| l | `+0x44 & 0x3000` | `object_setAttack4(0x3B)` |
| m | `AIData.Unk_1a != 0` | Buffered auto-move: `sub_80116D8(Unk_1a, sub_8010332())`, move type 1 (§M6.9) |
| n | none of the above | Stay idle |

Branches e–i also call `sub_801031C(0x10)`, which clears the idle-window bit, and a local-only HUD call. **The move branch (k) does not clear `Unk_48 & 0x10`**, so charging continues while moving.

**Priority:** buster, charge and chip requests beat movement on the same tick. A direction held while B is released yields the shot first. The move then starts on the first idle tick after the shot action ends; the direction is still held because movement is level-triggered.

##### M5.2 Direction reading, `sub_800FA54`

- If `ObjectFlags1 & 0x1000` (SLIDING): return 0.
- Otherwise read **held** `AIData+0x22` and test in this order:

  | Key | Bit | Direction code |
  |---|---|---|
  | Up | 0x40 | 1 |
  | Down | 0x80 | 2 |
  | Right | 0x10 | 4 |
  | Left | 0x20 | 3 |

  The first hit wins, so Up beats Down beats Right beats Left.
- If `ObjectFlags1 & 0x8000` (CONFUSED), remap with `byte_800FAA4 = {0,2,1,4,3}`: up↔down, left↔right.
- No direction held: return 0.

#### M6. Action 0x10: move one panel (`sub_80EB04C`)

##### M6.1 Entry, `sub_80116AE(dir, lag, type)`

1. Set `AttackVars.Unk_0c = dir`, `Unk_18` (u16) = `lag`, `Unk_03 = type`, `Unk_2c` (u32) = 0.
2. `object_setAttack4(0x10)` (§M2.2).
3. Call `sub_80EB04C` **immediately**, so phase 0 runs on the same tick as the input.

The two parameters:
- `lag`, from `sub_8010332`: 1 in battle mode 9. Otherwise 4 if navi stat 0x29 == 0, else `byte_8020FE0[stat29*11 + stat2B]`. That table (each navi's `move_lag` in the content) is 253 bytes, all 4. **Effectively 4.**
- `type`, from `sub_80103A8`: 3 if navi stat 0x31 (ProcessingBug) != 0, else 0.

##### M6.2 `sub_80EB04C` body

```
jump off_80EB074[AttackVars.Unk_00 / 4]   ; 0:sub_80EB088 4:sub_80EB128 8:sub_80EB194 C:sub_80EB1C4 10:sub_80EB1F8
if AttackVars.Unk_30 == 0 and sub_800FA54() != AttackVars.Unk_0c: AttackVars.Unk_30 = 1   ; "direction changed" marker (u32)
```

The marker is never read by the move. It is part of the persistent AttackVars state.

##### M6.3 Phase 0: start, `sub_80EB088`

1. `ObjectFlags1 &= ~0x400000`.
2. If `!object_canMove()`: abort. `object_canMove` fails when `ObjectFlags1 & 0x5040`: IMMOBILIZED 0x4000, SLIDING 0x1000, MOVING 0x40.
3. `AttackVars.Unk_30 = 0`.
4. Compute the target by `Unk_03`:
   - **0 (normal):** `sub_800F964(dir)`. Returns 0 if SLIDING. Otherwise `(dx,dy) = byte_800FA14[dir]` with `dx *= (1 − 2*Alliance)`. The table is `{0:(0,0), 1:(0,−1), 2:(0,+1), 3:(−1,0), 4:(+1,0), 5:(0,0)}`. Target = `(PanelX+dx, PanelY+dy)`. If `sub_800E618(target)` fails (§M6.4), return 0.
   - **1:** `sub_800F998(PanelX, PanelY, dir)` tries 4 directions from `byte_800FA00[dir*4..]`: dir 1→{1,3,2,4}, 2→{2,4,1,3}, 3→{3,2,4,1}, 4→{4,1,3,2}. It returns the first valid target; the same SLIDING→0 rule applies.
   - **2:** absolute target `(AttackVars.Unk_16, Unk_17)`.
   - **3:** `sub_800FA20` (ProcessingBug; not in this match).
5. **If the target is 0 (blocked):** `sub_80F02A2()` is 1 unless `AttackVars.Unk_0d != 0 && AIData.Unk_48 & 0x8000`; it is 1 in normal play. (The move never writes `Unk_0d`: it is byte 1 of the params the last chip left in the attack variables.) So `object_exitAttackState()` runs: `CurAnim = 0` and `CurAction = 8`. The otherwise branch is `sub_801171C`. Nothing else changes.
6. **If the target is valid:**
   - `AttackVars.Unk_16/17 = FuturePanelX/Y = target`.
   - `object_reservePanel` (§M6.5).
   - `AIData.Unk_48 |= sub_801BE04(tx, ty, PanelX, PanelY)`. The direction bits are: 8 if tx > x, 4 if tx < x, OR 2 if ty > y, 1 if ty < y.
   - `ObjectFlags1 |= 0x40` (MOVING).
   - `sub_800AB46(Alliance, 4, 1)`: statistics byte `0x0203EAE0[Alliance*16+4] += 1`, saturating at 0xFF.
   - If `sub_80F02A2()`: `object_setAnimation(4)` (sets `CurAnim = 4`, `CurAnimCopy = 0xFF`; applied the same tick by `sub_801BCF4`).
   - `AttackVars.Unk_10 = 3`, `Unk_00 = 4`.

**Blocked-move consequence (derived from code, not seen in the trace):** holding a direction toward an invalid panel re-enters `setAttack4` every idle tick. That resets `CurPhase/PI` to 0 each tick, so idle stays at `[4,8,0,0]` with Timer re-armed to 10→9 and never reaches phase 4. `CurAnim` is forced to 0 every tick.

##### M6.4 Target validity, `sub_800E618`

1. If `!object_isValidPanel(x,y)` (1 ≤ x ≤ 6 and 1 ≤ y ≤ 3): invalid.
2. `k = ((ObjectFlags1 & 0x10 /*AIRSHOE*/) || !(currentPanel.Flags & 0x10)) ? 0x10 : 0`, plus `Alliance*8`.
3. `(must_set, must_clear) = byte_800E660[k]`:

   | k | Case | must_set | must_clear |
   |---|---|---|---|
   | 0x00 | Alliance 0, needs floor | 0x10 | 0x0B8800A0 |
   | 0x08 | Alliance 1, needs floor | 0x30 | 0x07880080 |
   | 0x10 | Alliance 0, AirShoes or not standing on floor | 0x00 | 0x0B8800A0 |
   | 0x18 | Alliance 1, same | 0x20 | 0x07880080 |

4. Valid iff `object_checkPanelParameters(x, y, must_set, must_clear)`: `F = PanelData.Flags`, `F != 0`, `(F & must_clear) == 0`, and `(F & must_set) == must_set`.

What blocks a move, in terms of panel flags (§M7):

| Bit | Blocks | Meaning |
|---|---|---|
| 0x20 | Alliance 0 | Panel owned by alliance 1 (alliance 1 instead *requires* it) |
| 0x80 | Both | Reserved |
| 0x10 missing | Both, unless AirShoes or already off-floor | No floor: hole / broken / none |
| 0x08000000 | Alliance 0 | Occupancy bit from alliance-0 bodies (P1's own body sets it) |
| 0x04000000 | Alliance 1 | Occupancy bit from alliance-1 bodies (P2's own body sets it) |
| 0x00080000, 0x00800000, 0x01000000, 0x02000000 | Both | Occupancy bits of other obstacles |

The occupancy bits come from occupants' `CollisionData.SelfCollisionTypeFlags & 0xFFFF0000`. Observed values: P1 contributes 0x08500000, P2 contributes 0x04200000. So neither player blocks the other through occupancy; ownership blocks them instead. UNCERTAIN: which object kinds set 0x80000/0x800000/0x1000000/0x2000000 is for the collision section.

In this match P1's `ObjectFlags1 = 0x2000030` (AIRSHOE 0x10, FLOATSHOE 0x20, AFFECTED_BY_ICE) and P2's is 0x2000000. So P1 may step onto no-floor panels.

`sub_800E5AC` is the variant used for slides. It is the same except the floor requirement depends only on AIRSHOE (index 0x10 if AIRSHOE, else 0).

##### M6.5 Reservation

- **`object_reservePanel(x,y)`:** if the panel exists and `ReserverObjectPtr == 0`, set `Reserver = r5`, `Flags |= 0x80`, and header `Flags |= 0x20`, then return 1. Otherwise return 0. The move ignores the return value because validity already excluded reserved panels.
- **`object_removePanelReserve(x,y)`:** only if `Reserver == r5`, set `Reserver = 0` and `Flags &= ~0x80`.
- Header flag 0x20 is **never cleared** by this. The trace shows the header flags staying at 0x37 (55) after the first move. `sub_801BB78` uses flag 0x20 to scan for leftover reservations.

##### M6.6 Phases 4, 8 and C

**Phase 4, `sub_80EB128`.** Runs at F+1..F+3.
1. `AIData.Unk_48 &= ~0xF` every tick, so the direction bits are visible only at the end of F.
2. `AttackVars.Unk_10 -= 1`. If it is not 0, return.
3. When it reaches 0:
   - `sub_8013CC4(oldX, oldY)`, a NaviCust panel-trail bug. It returns immediately unless navi stat 0x13 != 0; see chapter 13.
   - `PanelX/Y = FuturePanelX/Y`.
   - `object_removePanelReserve(new)`.
   - `object_setCoordinatesFromPanels`.
   - `object_updateCollisionPanels`: `CollisionData.PanelX/Y = PanelX/Y`, and `CollisionData.Direction = sub_800E994(new − old)` (§M6.7).
   - If `AttackVars.Unk_2c != 0` and the column pattern ∈ {0x31, 0x23, 0x33}: `sub_800F2FC(Unk_2c)`. `Unk_2c` is 0 for input moves.
   - If `sub_80F02A2`: `object_setAnimation(3)`.
   - `Unk_10 = 5`, `Unk_00 = 8`.

**Phase 8, `sub_80EB194`.** Runs at F+4..F+8. `Unk_10 -= 1`. When it reaches 0:
- `ObjectFlags1 &= ~0x40`.
- `ObjectFlags1 |= 0x80000` (MOVE_COMPLETE).
- `Unk_10 = Unk_18`, which is 4.
- If `sub_80F02A2`: `object_setAnimation(0)`.
- `Unk_00 = 0xC`.

**Phase C, `sub_80EB1C4`.** Runs at F+9..F+12. `Unk_10 -= 1`. When it is ≤ 0:
- `AIData.Unk_1a = (Unk_03 != 1 && sub_8013FAE()) ? Unk_0c : 0`. `sub_8013FAE` returns 0 without RNG when navi stat 0x11 == 0.
- `object_exitAttackState()` if `sub_80F02A2`, else `sub_801171C`. Result: `CurAction = 8`, `CurAnim = 0`. Charge and pending intents are preserved because `Unk_1c == 4`.

Phase 0x10, `sub_80EB1F8`, is an alternate commit used by other callers and is not reached from an input move.

**Timing table (all numbers verified in the trace, frames 623–636 and 1982–1995):**

| Tick | CurAction | AttackVars.Unk_00 / Unk_10 after tick | Panel | x (16.16) | CurAnim | ObjectFlags1 | Notes |
|---|---|---|---|---|---|---|---|
| F−1 | 8 | – | (2,2) | −60<<16 | 0 | 0x2000030 | idle |
| F | 0x10 | 4 / 3 | (2,2), Future (3,2) | −60 | 4 | +0x40 | Target reserved; header flags +0x20; `Unk_48 |= 8` |
| F+1 | 0x10 | 4 / 2 | (2,2) | −60 | 4 | | |
| F+2 | 0x10 | 4 / 1 | (2,2) | −60 | 4 | | |
| F+3 | 0x10 | 8 / 5 | **(3,2)** | **−20** | 3 | | Commit; collision panel moves |
| F+4..F+7 | 0x10 | 8 / 4..1 | (3,2) | | 3 | | |
| F+8 | 0x10 | C / 4 | | | **0** | −0x40, **+0x80000** | |
| F+9 | 0x10 | C / 3 | | | 0 | −0x80000 | Consumed by `sub_801A36A` in step 3 |
| F+10, F+11 | 0x10 | C / 2, 1 | | | 0 | | |
| F+12 | **8** | 0 | | | 0 | | `CurPhase = 0`, `PI = 0` |
| F+13 | 8 | – | | | | | Controller: `PI = 4`, Timer 9; can act now |

Frames F..F+12 all show `CurPhase = 0` in the object header. Move phases live in `AttackVars.Unk_00`, not in `CurPhase`.

##### M6.7 Coordinates and collision direction

- **`object_getCoordinatesForPanels(px, py)`**, using s8 inputs:

  ```
  X = (px*40 << 16) − (140 << 16)
  Y = (py*24 << 16) − (20 << 16)
  ```

  Check: panel (2,2) gives x = −60, y = 28; (3,2) gives −20; (5,2) gives +60. `Z` is untouched by the move.
- **`sub_800E994(dx, dy, alliance)`**, the direction code for `CollisionData.Direction`:

  | Case | Code |
  |---|---|
  | `dx >= 2` or `dy >= 2` (signed; `dx <= −2` is **not** caught) | 5 |
  | dx > 0 | 4 if dy == 0 (3 if alliance 1), else 5 |
  | dx < 0 | 3 if dy == 0 (4 if alliance 1), else 5 |
  | dx == 0 | 2 if dy > 0, 1 if dy < 0, 0 if dy == 0 |

  The trace shows 4 for P1 moving right. Ice slides use this Direction.

##### M6.8 After the move: ice and road slides (`sub_801A36A` `sub_80166B6`)

**Trigger check.** `sub_801A36A` runs inside step 3 (`sub_801AC6C`) every tick.
1. If not paused, not dimmed, and `AIData.Unk_38 != 0`: decrement it. When it reaches 0, go directly to the MOVE_COMPLETE/ice check below. `Unk_38` is the road cooldown.
2. Return if `ObjectFlags1 & 0x100040` (DRAG or MOVING).
3. Look up the current collision panel type:
   - **Types 9..0xC (road/conveyor):** `sub_801A400`. If `AIData.Unk_38 == 0` and `!(ObjectFlags1 & 0x24)`: `ObjectFlags2 |= 0x10`, `Unk_0f = 3`.
   - **Else, if `ObjectFlags1 & 0x80000`:** clear it. If the panel type is 7 (ice), run `sub_801A3DA`: if element != 2 (aqua), `!(ObjectFlags1 & 0x24)`, and `ObjectFlags1 & 0x2000000` (AFFECTED_BY_ICE), set `ObjectFlags2 |= 0x10` and `Unk_0f = 2`.

This match has road panels: row 1 types 0xC on alliance 0's side, 0xB on alliance 1's side. No player stepped on them.

**Slide engine**, `sub_80166B6` via `BattleObject.Unk_1f`. It is shared with push/drag (§H4.3).

*State 0, `sub_80166D0`:*
- `ObjectFlags1 |= 0x1000` (SLIDING).
- `PanelXY = FuturePanelXY`, set coordinates, collision panels = current, remove reserve on Future.
- `(dx, dy, n) = sub_800E468()`. Per `Unk_0f`:
  - 2 (ice): `byte_800E4E8[CollisionData.Direction]` with dx × alliance direction.
  - 3 (road): `byte_800E538[type−9]`: 9 = up (0,−1), 0xA = down (0,+1), 0xB = world-left (−1,0), 0xC = world-right (+1,0). Absolute directions, n = 1.
  - 1 (push): the hit modifier.
- The first step must pass `sub_800E5AC`, else n = 0.
- If n != 0: `Future = Panel + d`, reserve, `Unk_1e = 4`, `Unk_1f = 4`.
- If n == 0: if `Unk_0f == 3`, set `AIData.Unk_38 = 5`; `Unk_0f = 0`; clear SLIDING.

*State 4, `sub_8016730`:*
- While `--Unk_1e > 0`: `X += dx*10.0`, `Y += dy*6.0` (16.16 values 0xA0000 / 0x60000), `object_setPanelsFromCoordinates`, collision panels = current. **`PanelX/Y` therefore change mid-slide, from the coordinates.**
- When `Unk_1e` reaches 0:
  1. Remove reserve; `Panel = Future`; set coordinates.
  2. Landing on ice (type 7, element != aqua): `n += 1`.
  3. Landing on road 9..0xC (and `!(flags & 0x24)`): if `Unk_0f == 3`, `AIData.Unk_38 = 5` and stop. Otherwise switch to road (`Unk_0f = 3`, new d, `n += 1`).
  4. `n -= 1`. If `n > 0` and `sub_800E5AC(Future + d)`: `Unk_1e = 4`, advance, reserve. Otherwise stop: `CollisionData.Direction = sub_801683C(dx, dy, alliance)`, `Unk_1f = 0`, clear SLIDING, `Unk_0f = 0`, collision panels = current.

So a slide takes 4 ticks per panel. While SLIDING, `sub_800FA54`, `sub_800F964` and chip use (`sub_800FB54`) are all disabled.

Details the summary above leaves out:
- `object_setCollisionPanelsToCurrent` (not `object_updateCollisionPanels`) keeps the collision panel on the navi's: it leaves CollisionData+Direction alone. The direction is written only when the slide stops (and on switching to a road), from `sub_801683C(dx, dy, alliance)`: forward (dx toward the other side) 4, back 3, else up 1, down 2, none 5.
- The step and panel counters are compared as the 32-bit results of `ldrb` − 1: a slide goes on while the old value was 2 or more.
- On switching to a road, `sub_800E468` recomputes only dx and dy; the panel count carries on (+1, then the common −1).

Trace-verified on an ice slide in soundmod (frames 3201–3205: side 0 steps onto an ice panel with the move's MOVE_COMPLETE, slides one panel forward, and its panel changes mid-slide at 3204). Road slides and pushes are still code-derived only.

##### M6.9 Buffered auto-move (`AIData.Unk_1a`)

`AIData.Unk_1a` is only ever set nonzero by the move's end, and only when `sub_8013FAE()` (stat 0x11 random bug) is true. When set, the controller's step m starts `sub_80116D8(Unk_1a, lag)`: move type 1, trying fallback directions. Any non-move `exitAttackState` clears it. In normal PvP it stays 0 and there is **no move buffering**: a tapped direction released before the next idle tick is lost. The input is level-triggered.

#### M7. Panel data (`ePanelData` = 0x02039AE0)

- Address of panel (x,y) = `0x02039AE0 + ((y*8 + x) << 5)`. `_object_getPanelDataOffset` returns 0 unless 1 ≤ x ≤ 6 and 1 ≤ y ≤ 3. The grid is 8×5; the border rows and columns exist with `Flags = 0`.
- Fields:

  | Offset | Size | Field |
  |---|---|---|
  | +0x00 | u8 | Visible |
  | +0x02 | u8 | Type |
  | +0x03 | u8 | Alliance (0/1; 0xFF at x = 0/7) |
  | +0x06 | u8 | Animation (copy of Type) |
  | +0x07 | u8 | Alliance copy |
  | +0x14 | u32 | **Flags** |
  | +0x1C | ptr | **ReserverObjectPtr** |

  The other fields are timers used by the panel system.
- **Flags composition** (`_object_updatePanelParameters`):

  ```
  Flags = Type
        | TypeFlags[Type]
        | (Alliance << 5)
        | (Reserver ? 0x80 : 0)
        | occupancy(x,y)
  ```

  `TypeFlags` (`word_3007924`):

  | Type | TypeFlags |
  |---|---|
  | 0 | 0x8000 |
  | 1 | 0x14000 |
  | 2 | 0x10010 |
  | 3 | 0x10050 |
  | 4 | 0x10110 |
  | 5 | 0x12010 |
  | 6 | 0x10410 |
  | 7 | 0x10810 |
  | 8 | 0x11010 |
  | 9..0xC | 0x10210 |

  Bit 0x10 is "floor". Types 0 and 1 have none (none/hole). 3 = cracked (0x40); 7 = ice; 9..0xC = road.
- **Occupancy** (`sub_3007978`): `mask = unk_2034F60[y*8+x]` (u32). For each set bit from bit 31 downward, the matching entry `eCollisionData[i]` (stride 0xA8, base 0x020384F0) contributes `CollisionData+0x30 & 0xFFFF0000`.
- The Flags word is **stored**. It is rewritten when collision is presented or removed (`sub_300777C`, `_object_removeCollisionData`), when panel type changes, and when all panels are recomputed at init (`sub_30078C8`). Reservation toggles bit 0x80 in place.

  UNCERTAIN: whether every occupancy change (`sub_3007868`, `sub_3007880`) is followed by a Flags recompute. Safest is to model the stored word and the same update points (collision spec; see §7.3).
- Observed at frame 624: P1's side type 2 gives 0x10012. P2's side gives 0x10032. Row 1 has 0x1021C (type 0xC) and 0x1023B (type 0xB). The move target (3,2) shows 0x10092 with Reserver = P1. P1's panel shows 0x08510012, P2's 0x04210032.

#### M8. Trace verification summary (the machgun trace)

**Players.**
- P1 is alliance 0, in T1 slot 1 at 0x0203AA88, with AIData at 0x02034180.
- P2 is alliance 1, in T1 slot 0 at 0x0203A9B0, with AIData at 0x02034080. P2 updates first.
- Both have AIData `ActorType 2`, `AIIndex 0`, `Unk_06 = 0`, `Unk_07 = 1`, `Unk_08 = 0xFF`, `Unk_48 = 0x10` when idle. Navi stats: 0x29 = 0, 0x2B = 0xA, 0x2C = 0, 0x31 = 0, 0x11 = 0, 0x13 = 0, 0x24 = 0.

**Moves.** Only two occur, both P1 moving (2,2)→(3,2) with Right, at 623 and 1982. Both are identical and match the timing table in §M6.6:

| Event | Round 1 frame | Round 2 frame |
|---|---|---|
| Move starts (press) | 623 | 1982 |
| Panel and x change | 626 | 1985 |
| Anim 0 and 0x80000 set | 631 | 1990 |
| 0x80000 cleared | 632 | 1991 |
| Action 8 | 635 | 1994 |
| PI = 4, Timer 9 | 636 | 1995 |

- Right was held through 631 and released at 632. Holding into the rightmost own column caused no second move, because the move was still in progress.
- An A press at 638 started action 0x37 while idle phase-0 Timer was 7, confirming that phase 0 does not gate input.
- Round 2: phase 4 was reached at 1979, then the move at 1982.

**Entry.** §M4.1 table, frames 72–122.

**Not observable in this trace:** alliance-1 movement, blocked moves, confusion, ice/road slides, AirShoes onto holes.

### 12.B Buster, charged shot and chip-use dispatch

Scope: the base-form player (navi stat 0x2C `Transformation` = 0, AIData `AIIndex` = 0). Frame numbers are `frame` values in the machgun trace, or in traces I generated with the same replay plus injected inputs (see "Verification").

Terms used below:
- **AI** = the player's AIData (`obj+0x58`).
- **AV** = AttackVars, at `AI+0xA0`.
- **NS[n]** = byte n of the side's battle navi stats. These are 0x64 bytes per side at `eBattleNaviStats0` = 0x0203CE00. Read them with `GetBattleNaviStatsByte_AllianceFromBattleObject`.
- **flags44** = `AI+0x44` (command-request flags) and **flags48** = `AI+0x48` (state flags). The get/set/clear helpers are `GetAIData_Unk_44_Flag`, `SetAIData_Unk_44_Flag`, `ClearAIData_Unk_44_Flag`, and for flags48 `sub_801032C`, `sub_8010312` (OR) and `sub_801031C` (BIC).

#### B1. Summary of the rules and numbers

| Item | Value (base MegaMan, all NaviCust stats 0) | Source |
|---|---|---|
| Buster trigger | **Release** of B. If AI.Unk_07 (charge-shot type) is 0xFF, the **press** of B triggers it instead. | `sub_8012FC8` loc_8013176 |
| Buster damage | `min(10, NS[1] + 1 + formBonus[NS[0x2C]])`. If emotion is 5, the damage is 1. Stats 0 give **1**. | `sub_801265A` |
| Charged-shot damage | `(NS[1]+1)*10`. If emotion is 5, the base is 1. Stats 0 give **10**. | `sub_8011A7E` |
| Full charge | B held with the charge counter at `thr = table8020404[AI.Unk_07][NS[3]]`. That is **100 frames** at NS[3]=0 (90/80/70/60 for NS[3]=1..4). The "charging" level shows from counter ≥ 10. | `sub_8012EBC`, `sub_8012F62` (8870) |
| Buster action | CurAction 0x11 (`sub_80EB436`). The shot spawns on the 2nd action tick (release frame R+2). The action exits at R+N+6, and the next idle-decision tick is R+N+7. | |
| Charged-shot action | CurAction 0x16 (`sub_80EBE00`). The shot spawns at R+6 and the action exits at R+N+10. | |
| Recovery N | `byte_80209CC[NS[2]*6 + min(k,5)]`. k = number of consecutive free panels ahead of the player. At NS[2]=0, N = 4,8,12,16,20,24 for k = 0..5. | `sub_800FAF6` |
| Projectile | T3 object index 0 (`sub_80C4E58`). It advances 1 panel every 2 ticks. The first step happens on the tick after it spawns. | |
| Chip trigger | **Press** of A (for a chargeable chip or form, the release of A instead). The action is ChipData[0x0B], started with `object_setAttack2` on the same frame. | `sub_8012FC8`, `sub_800FB54`, `sub_80127C0` |
| RNG | Every buster request calls **GetRNG2 once** (`sub_8013D5E`). Every projectile hit that spawns a hit effect calls **GetRNG2 once** (`AddRandomVarianceToTwoCoords`). There are further RNG2 calls only if NS[0x4D]/NS[0x4F] ≠ 0. | see §B9 |

#### B2. Per-player data used by this subsystem

**Weapon-type bytes in AIData.** They are written at init and on form change by `sub_800FEEC`. They are rewritten by `sub_800FF5E` at the end of `sub_80139F6`. For form 0 they come from navi stats. For other forms they come from `byte_8020354[form*6]`.

| AI byte | Meaning | Form-0 source | Machgun P1/P2 |
|---|---|---|---|
| +0x04 | Routine for the battle-mode-9 A press | NS[0x44] if mode 9, else 0xFF | 0xFF |
| +0x05 | A-charge (chip charge) type | NS[0x39] | 0xFF |
| +0x06 | Buster routine index into `off_80117D4` | NS[4] | 0 → `sub_8011A26` |
| +0x07 | Charge-shot routine index into `off_80117D4` (via `sub_800FFAA`, special case 0x21..0x26) | NS[5] | 1 → `sub_8011A7E` |
| +0x08 | B+Back special routine index | NS[7] | 0xFF |
| +0x11 | Alternative A-charge type (form chips) | 0xFF | 0xFF |

**Input latch (per player AIData).** `sub_8012DFC(alliance)` runs once per battle frame from the battle-mode handler, before `RunBattleObjectLogic`. It reads the side's held keys from `0x02036820 + 8*alliance + 2`. It then sets `AI+0x28` = previous held, `AI+0x22` = held, `AI+0x24` = pressed (`new & ~old`) and `AI+0x26` = released (`old & ~new`). The names in AIData.inc are misleading: +0x24 is the press edge and +0x26 is the release edge. When the battle is over it zeroes all four. Keys: A=1, B=2, SELECT=4, START=8, RIGHT=0x10, LEFT=0x20, UP=0x40, DOWN=0x80, R=0x100, L=0x200.

**Charge state:**
- `AI+0x1B` = charge counter.
- `AI+0x1D` = charge level: 0 = none, 1 = charging (counter ≥ 10), 2 = full.
- `AI+0x1E` = charge source: 0 = none, 1 = A, 2 = B.

**flags44 bits relevant here:**

| Bit | Meaning |
|---|---|
| 0x1 | Buster request |
| 0x2 | Charged-shot request |
| 0x4 | Chip request |
| 0x8 | Charged-chip request |
| 0x10 | B+Back special request |
| 0x20 | Forced action 0x16 (setter not found; UNCERTAIN) |
| 0x1000 / 0x2000 | Turn requests from L/R |
| 0x10000 | Chip from an alternative source (`sub_800EE26`) |
| 0x20000 | A held (A-charge running) |
| 0x40000 | B held (B-charge running) |
| 0x2000000 | SELECT special |
| 0x10000000 | Mode-9 A request |

**flags48 bits relevant here:**
- 0x10 = "controllable": set on the idle phase-0 init, cleared when a buster, charge, chip or special starts. Charging requires it.
- 0x40 = chip in progress: set at chip start, cleared on the idle phase-0 init.
- 0x200 blocks charging (setter not traced).
- 0x400 enables L/R turning.

**AttackVars (AV) fields used:**

| Field | Meaning |
|---|---|
| +0x00 | Attack sub-phase (jump offset 0/4/8) |
| +0x01 | Sub-phase-initialized |
| +0x03 | Buster variant: 0 = single, 1 = 3-way spread, 2 = special |
| +0x04 | Chip charge flag |
| +0x05 | Copied to AI.Unk_19/Unk_15 on exit |
| +0x06 (u16) | Chip extra value |
| +0x08 (u16) | Damage |
| +0x0A (u16) | Stored into the projectile's `StaminaDamageCounterDisabler` |
| +0x0C | Projectile Param1 (config index) |
| +0x0D | Projectile Param2 (row delta) |
| +0x10 (u16) | Timer |
| +0x12 (u16) | Recovery N |
| +0x14 (u16) | Chip ID |
| +0x1B | Special-source flag |
| +0x1C | Attack category (`object_setAttackN` stores N) |
| +0x1D, +0x1E | Cleared by `sub_801011A` |

`object_setAttackN(action)` does the following. It sets `CurAction = action` and `CurPhase = PhaseInitialized = 0`. It sets `AV.u16[0] = 0` (sub-phase 0, not initialized) and `AV.Unk_1C = N`. It then calls `sub_801011A`, which sets AV.Unk_1D = 0, AV.u16[0x1E] = 0 and calls `sub_80E1662`.

#### B3. Input decode: `sub_8012E74` → `sub_8012FC8` (buttons → flags44)

`sub_80EA484` (the player update) calls `sub_8012E74` first, before any action logic. If the battle is over it clears the four joypad words. If paused it does nothing. Otherwise it calls `sub_8012FC8` (decode) and **then** `sub_8012EBC` (charge accumulate). In dimming, `sub_8012FC8` takes a separate branch that only handles flag 0x800, and `sub_8012EBC` returns immediately.

Let `f0` = flags44 as read at the entry of `sub_8012FC8`. The decode runs in this order:

1. **SELECT special** (only if `TestBattleFlag_0x40`, via `sub_800A8F8`): if SELECT is pressed and `u16[sub_802E070(alliance)+0x28] ≥ 0x1500`, set 0x2000000 and **return**. Otherwise the local side plays SOUND_CANT_JACK_IN. UNCERTAIN: this looks like a form/Beast feature and should not occur in base PvP.
2. **L/R** (skipped in battle mode 1).
   - If flags48 & 0x400: an L/R press sets 0x1000 when DirectionFlip ≠ 0 (and clears 0x2000), otherwise sets 0x2000 (and clears 0x1000).
   - Else if `battle_getFlags() & 2` (custom gauge full) and L or R is pressed: `battle_setFlags(0x10)` (open custom screen) and **return**. The rest of the decode is skipped that frame.
3. **Hold flags** (0x20000 = A, 0x40000 = B), using `held` = AI+0x22:
   - If neither hold flag is set:
     - If `sub_801336C()` (A-chargeable) and A is held → set 0x20000.
     - Else if `sub_8013396()` (AI.Unk_07 ≠ 0xFF and `sub_8012F3E()`) and B is held → set 0x40000.
   - If 0x20000 is set:
     - B pressed → switch to 0x40000.
     - Else keep it while A-chargeable and A is held.
     - Else clear both.
   - If 0x40000 is set:
     - A pressed with A-chargeable → switch to 0x20000.
     - Else keep it while `sub_8013396()` and B is held.
     - Else clear both (0x60000).
4. **B+Back special**: only if AI.Unk_08 ≠ 0xFF and AI.Unk_15 == 0. Base MegaMan has 0xFF, so this does not apply. The rule: a B press opens an 8-frame window in AI.Unk_13; a Back press (LEFT if not flipped, else RIGHT) while B is held inside the window sets 0x10.
5. **Buster / charged shot**: only if AI.Unk_06 ≠ 0xFF and `(f0 & 3) == 0`.
   - The edge used is `released` (AI+0x26) when AI.Unk_07 ≠ 0xFF, otherwise `pressed`. Buster types 3, 4 and 0x2C use `held` instead, gated by form and flags.
   - If the edge has B: set flag **2** if `AI.Unk_1E == 2 && AI.Unk_1D == 2`, otherwise set flag **1**.
   - These values are the charge state from the **previous** frame, because accumulation runs after the decode.
6. Battle mode 9 with A pressed → set 0x10000000.
7. **Chip**. All of the following must hold:
   - `sub_800A772(alliance)`, which requires AI.Unk_19 == 0 and `BattleState+0x5C` bit 0x4 (alliance 0) or 0x8 (alliance 1). These are chips-enabled bits; `sub_800139A` reads the word.
   - `(f0 & 0xC) == 0`.
   - `sub_8010004() ≠ 0xFFFF`, meaning the hand has a current chip.
   
   The edge is `pressed & A` if `sub_801336C()` is false, otherwise `released & A`. The flag set is **4**, or **8** if `AI.Unk_1E == 1 && AI.Unk_1D == 2` (A fully charged).

`sub_8012F3E`, the "may charge / may raise hold flags" predicate: `(flags44 & 0x1000002F) == 0 && (flags48 & 0x200) == 0 && (flags48 & 0x10) != 0`.

#### B4. Charge accumulation: `sub_8012EBC`

Each non-paused, undimmed frame, after the decode:
- If `!sub_8012F3E()` → Unk_1E = Unk_1B = Unk_1D = 0 (reset).
- Else `thr = sub_8012F62(old Unk_1E)`:
  - The row index is `AI[7]` (Unk_07) if old Unk_1E == 2, else `AI[5]` (Unk_05).
  - In forms 0x0B..0x18 whose current hand chip has ChipData[6] == 0x0A, the row index is `AI[0x11]`.
  - If the row is 0xFF, thr = 0xFF.
  - Otherwise thr = `u16 byte_8020404[row*5 + NS[3]]`.
- If flags44 has neither 0x20000 nor 0x40000 → reset.
- If 0x20000: Unk_1E := 1. If 0x40000 (checked second): Unk_1E := 2. If the previous Unk_1E was the other nonzero source, Unk_1B = Unk_1D = 0 first.
- Then `Unk_1B++`. Unk_1D = 0 if Unk_1B < 10. Otherwise Unk_1D = 1, and if Unk_1B ≥ thr then Unk_1B = thr and Unk_1D = 2.

Rows of `byte_8020404` (0x08020404, 5 u16 per row, column = NS[3] Charge 0..4) that matter:

| Row | Values | Used for |
|---|---|---|
| 1 | 100, 90, 80, 70, 60 | MegaMan charge shot |
| 5 | 20 ×5 | A-charge type 5, seen in round 2 of the match |

Consequences:
- The charge only accumulates while flags48 & 0x10 is set, i.e. in idle (action 8, after its phase-0 init) **and while moving**.
- During a buster, charged-shot or chip action the charge resets to 0.
- `sub_8012EA8` (called by `object_exitAttackState`) also clears 0x60000 and the counters.
- After an attack exits at frame E, flags48.0x10 is set during E+1's dispatch. So a held B raises 0x40000 and counts `Unk_1B = 1` at **E+2**. Observed: exit 2288 → A-charge counter 1 at 2290 in the original trace; exit 747 → 0x10 set at 748 in injected run 2.

#### B5. Idle decision (action 8): `sub_80EA734` → `sub_80F0354`

Action 1 (`sub_8017888`) switches immediately to action 8. `sub_801AF44` dispatches through `sub_801B9E6`:
- `CurAction < 0x10` → the form table (`off_80EA52C`, entry 8 = `sub_80EA734`).
- `CurAction ≥ 0x10` → `JumpTable80EAC60[CurAction-0x10]` (0x080EAC60). If AV.Unk_1D == 1, `sub_80EAD9C` is called instead.

`sub_80EA734`:
1. If the battle is over → end-of-battle path (§H6).
2. Else if flags44 & 0x8600 → `sub_801056A(0,0)`.
3. Else if flags44 & 0x80000 → `object_setAttack0(0x49)`.
4. Else call `JumpTable80EA7B0[enemy_getStruct1(NameID)[4]]`. All 25 entries are `sub_80F0354`.

`sub_80F0354`, every tick in action 8:

a. **UI only**, local side (`battle_networkInvert(alliance)` = 0): `sub_801DA48(0x40)` if `sub_800A772`, else `sub_801DACC(0x40)`. These set or clear HUD elements in `eStruct2035280`.

b. **Phase 0** (CurPhase == 0):
   - On the first tick (PhaseInitialized == 0): `sub_801DA48(2)` (UI), Timer = 10, flags48 |= 0x10, flags48 &= ~0x40, PhaseInitialized = 4.
   - Every phase-0 tick, including the first: `Timer--`. If the result ≤ 0: CurPhase = 4, PhaseInitialized = 0 (so phase 4 begins after 10 ticks).
   - Phase 4 has no body. **The decision below runs on every tick regardless of phase**, including the phase-init tick.

c. If NS[0x2C] ≥ 0x17 → form-specific path via `sub_802D322` (not base form).

d. `sub_802E4E4()` consumes flags44 0x2000000 / 0x20000000 into `sub_802E070(alliance)`+0x50/+0x54. Then `r = sub_802E4B8()`: 9 → `sub_802F068`, return; 0xD → form path. Base PvP returns 1.

e. If flags44 & 0x600 → `sub_801056A(0,0)`, return. This is a reactive-chip trigger, not base.

f. If `sub_8010660()` → return. In link battles, NS[0x0D] bit 4 triggers once when HP ≤ MaxHP/4; it spawns an effect with id 0x17B. NS[0x0D] is 0 in normal play.

g. **Priority chain** (first match wins; each returns):

| # | Condition (flags44) | Effect |
|---|---|---|
| 1 | 0x20 | flags48 &= ~0x10; `object_setAttack1(0x16)` (no damage setup) |
| 2 | 0x1 | flags48 &= ~0x10; `a = off_80117D4[AI.Unk_06]()` (`sub_8011764`); `object_setAttack1(a)` |
| 3 | 0x2 | flags48 &= ~0x10; `a = off_80117D4[AI.Unk_07]()` (`sub_80117A4`); `setAttack2(a)` if AI.Unk_07 ∈ 0x21..0x26, else `setAttack1(a)` |
| 4 | 0x10 | flags48 &= ~0x10; `a = off_80117D4[AI.Unk_08]()`; `setAttack3(a)` |
| 5 | 0x10000000 | flags48 &= ~0x10; `a = off_80117D4[AI.Unk_04]()`; `setAttack1(a)` |
| 6 | `c = sub_800FB54()` ≠ 0xFFFF | chip (§B8) |
| 7 | `d = sub_800FA54()` ≠ 0 (d-pad held) | `sub_80116AE(d, sub_8010332(), sub_80103A8())` → movement action 0x10 (§M6) |
| 8 | 0x3000 | `object_setAttack4(0x3B)` (turn) |
| 9 | AI.Unk_1A ≠ 0 | `sub_80116D8(AI.Unk_1A, sub_8010332())` |

Rows 2-5 also call `sub_801DACC(0x40)` on the local side (UI). Rows 1-3 read flags44 once; rows 4 and 5 re-read it.

Therefore a buster or charged-shot request beats both a chip and movement on the same tick.

**Request flags are not cleared when consumed.** Flag 1 or 2 stays set for the whole buster action (observed flags44 = 1 for frames 636-649). They are cleared only by `object_exitAttackState`.

#### B6. Buster (AI.Unk_06 = 0)

**Setup, `sub_8011A26`**, run inside `sub_80F0354` on the request tick:
1. `v = sub_8013D5E()`:
   - Fill a 16-byte table with 0.
   - Write NS[0x14] entries of value 1, then NS[0x15] entries of value 2.
   - `v = t[GetRNG2() & 0xF]`. **One RNG2 step on every buster request, even when both stats are 0.** Verified: the watch on 0x020013F0 showed `GetRNG2 <- sub_8013D5E <- sub_8011A26 <- …` in frame 606 of injected run 1.
   - If v == 1 → `sub_8011ADA`: zero the AV fields and return action 0x33.
   - If v == 2 → `sub_8011A7E`: a charged shot.
2. Otherwise:
   - AV.Unk_08 = `sub_801265A()` (damage).
   - Zero AV.Unk_02, 04, 05, 06 (u16) and 0A (u16).
   - `s = NS[0x4D]`. If s ≠ 0: `r = GetPositiveSignedRNG2()`; if `(r & (s ≥ 0x1E ? 7 : 1)) != 0` then s = 0.
   - AV.Unk_0C = s, AV.Unk_03 = 0. Return **0x11**.

**Damage, `sub_801265A`**: `d = NS[1] + byte_80126A4[NS[0x29]]`. All 12 entries of `byte_80126A4` are 1.
- If `sub_8015B54(alliance) == 5` → d = 1.
- Else `d += byte_80126B0[NS[0x2C]]`. That table is 1 at indices 1 and 13, 0 elsewhere.
- `d = min(d, 10)`.

The emotion function is `sub_8015B64`. It returns the first matching case:

| Condition | Result |
|---|---|
| AI+0x36 ≠ 0 | 5 |
| NS[0x0E] (Mood) == 0 | 5 |
| AI.Anger (+0x34) ≠ 0 | 3 |
| AI+0x32 ≠ 0 | 1 |
| Mood == 0xFF | 2 |
| otherwise | 0 |

In the match, Mood = 0x80 for both sides, so emotion = 0.

**Action 0x11, `sub_80EB436`** (the pack's `navis/megaman/weapons/buster/weapon.luau`, with the weapon
routine; the weapon ids that alias `sub_8011A26` in `off_80117D4`, 0x2E, 0x2F, 0x3E, 0x3F, 0x4D..0x51, 0x6F, 0x70,
0x77, 0x79, 0x7B, 0x7E and 0x82, name the same module). It dispatches on AV.Unk_00 (a sub-phase past 4 reads
past its two-entry table):

Sub-phase 0, `sub_80EB450`:
- On init (AV.Unk_01 == 0):
  - `object_setAnimation(0x0E)`.
  - `sub_80EB562` spawns the buster-arm T1#5 via `sub_80B8E30` and stores it at AI+0x68.
  - `object_setFlag1(0x400000)` (USING_ACTION).
  - AV.Unk_01 = 4, AV.Unk_10 = 0.
- Every tick, if AV.Unk_10 == 1 (**fire**):
  - `PlaySoundEffect(SOUND_BUSTER_6A)`.
  - Variants 0 and 1: `AV.Unk_12 = sub_800FAAC(AV.Unk_0C (ldrb), AV.u32[0x08], 0x180000)`: the projectile (§B8)
    from the panel in front, 24 pixels up, and the recovery counted from there.
    - Variant 1 (the spread) fires two more shots with AV.Unk_0D = 1 and then 0xFF (`ldrh` of AV.Unk_0C, so
      Param2 = the row delta); AV.Unk_0D keeps 0xFF. Their recoveries are discarded. No weapon routine of
      MegaMan's sets variant 1 (**unverified**: no scenario reaches it).
    - Variant 2 (the absorbed-obstacle throw of routines 0x2B/0x2C) plays sound 0xFF too and throws
      (`sub_80C6248`): the flying shot (T3 #0xB, §B8) of kind 6 from the center of the panel in front, 12 pixels
      up, with damage AV.u32[0x08] and the obstacle word AV+0x30 as its ExtraVars. It sets no recovery: sub-phase
      4 waits whatever AV.Unk_12 holds from the last action that wrote it. The port keeps AV.Unk_12 as the navi's
      `recovery` word (`AttackVars::recovery`), which actions 0x11 and 0x16 write; a chip action that writes the
      same word in the game but keeps its own state in the port would leave it unchanged (**unverified**). A lab
      scenario (the Falzar side of the gregar base in DustCross: a shot, a blank shot, B+Back, the throw)
      verified that the throw waits the first shot's recovery through the blank shot and the pull.
  - Then `sub_80B8E30` with r7 = &obj.RelatedObject1Ptr spawns a second T1#5 with r4 as its parameters: 5, the
    muzzle flash; after a throw, r4 still holds the throw's 6, so a second buster arm.
- Every tick: `AV.Unk_10++`. If > 4 → AV.u16[0] = 4 (sub-phase 4, not initialized).
- So sub-phase 0 lasts 5 ticks and fires on its 2nd tick.

Sub-phase 4, `sub_80EB502`:
- On init: AV.Unk_10 = AV.Unk_12 (N).
- Each tick: `AV.Unk_10--`.
  - If < 0: obj.RelatedObject1Ptr = 0, AI+0x68 = 0, `object_exitAttackState`. The T1#5 objects then destroy themselves.
  - Else **move-cancel**: if `object_canMove()` and `d = sub_800FA54()` ≠ 0 and `sub_800F964(d)` (target panel acceptable): clear the two pointers, `object_exitAttackState`, then `sub_80116AE(d, sub_8010332(), sub_80103A8())` to start a move immediately.

**Timeline**, with R = the tick that raised and consumed the request (for B-release, the frame whose released edge has B):

| Tick | Event |
|---|---|
| R | CurAction ← 0x11, inside idle dispatch |
| R+1 | Sub-phase 0 init: anim 0x0E, T1#5 arm, USING_ACTION |
| R+2 | Fire: T3 shot spawned, flash T1#5 spawned |
| R+5 | Sub-phase → 4 |
| R+6 | Sub-phase 4 init, timer = N-1 after its first decrement |
| R+N+6 | Timer goes negative → `object_exitAttackState`, CurAction = 8 |
| R+N+7 | First idle tick (phase-0 init); a new action can start here |

Minimum buster cycle: N+7 frames.

**Recovery N, `sub_800FAAC` → `sub_800FAF6`** (1824):
- Start at (PanelX + front, PanelY).
- Count k = the consecutive panels for which `object_getPanelParameters` (PanelData.Flags, +0x14) is ≠ 0 **and** `Flags & mask[alliance] == 0`, stepping by `front`.
  - mask[0] = 0x0D880080, mask[1] = 0x0E880080 (`byte_800FB4C`).
  - Off-field panels have Flags = 0. Empty field panels have Flags such as 0x00010012 or 0x00010032. The enemy body sets 0x04000000 for alliance 1 and 0x08000000 for alliance 0 (dump at frame 607).
- `k = min(k, 5)`; `N = byte_80209CC[NS[2]*6 + k]`.

| NS[2] (Rapid) | N for k = 0..5 |
|---|---|
| 0 | 4, 8, 12, 16, 20, 24 |
| 1 | 3, 7, 10, 14, 17, 20 |
| 2 | 3, 6, 9, 12, 15, 17 |
| 3 | 2, 4, 6, 8, 10, 12 |
| 4 | 2, 3, 4, 5, 6, 7 |

`front` = `object_getFrontDirection` = `-(2*(Alliance ^ DirectionFlip) - 1)`: +1 for alliance 0 unflipped.

**The arm, `sub_80EB562` → `sub_80EB572`** (all three shots). It spawns T1#5 via `sub_80B8E30` into AIData+0x68 with r4 = Param1 | Param2 << 8 | Param3 << 16 | Param4 << 24:
- players with AIIndex 0: Param1 = 6 (the buster arm), **Param2 (its animation) = NaviStats+0x2C, the form**, Param4 (palette) = 0xE with AIData+0x48 bit 0x200 in base form, else 0x14 + NaviStats+0x10 when that is nonzero; 0xE in forms 0x0B/0x0C at Full Synchro; 5 + form − 0xD in forms 0x0D..0x11; else 0;
- players with another AIIndex: no arm, AIData+0x68 = 0;
- link navis (actor type 1): Param1 = 0x2B, Param2 = AIIndex − 1, Param4 = 0xD; viruses: Param1 = 6.

**Blank shot, action 0x33 (`sub_80ED748`)**, from `sub_8011ADA` (all AV fields zero):
- Sub-phase 0 (`sub_80ED764`) is the buster's without the shot: anim 0x0E, the arm, USING_ACTION, AV.Unk_10 = 0; sound 0xF8 on its 2nd tick; after 5 ticks sub-phase 4.
- Sub-phase 4 (`sub_80ED7A2`) is the buster's recovery, but N = `sub_800FAF6(PanelX, PanelY, Rapid)` counts from the navi's **own** panel rather than the one in front. The navi's collision is off the field while it updates, so its own panel counts as open: k is one more than a shot from there would count (up to 5).
- Soundmod round 1, 3367: side 1 (form 0xA) presses B; its buster routine 0x2B (`sub_8011F8C`) has no absorbed obstacle (AIData+0x0D = 0) and falls back to `sub_8011A26`, whose NaviCust roll (one RNG2 step) picks the blank.

**Routines 0x2B and 0x2C (`sub_8011F8C`, `sub_8011FCE`)**: with absorbed obstacles (AIData+0x0D, list at +0x6C), they pop the last and throw it: action 0x11 variant 2, damage 200, AV+0x30 = the obstacle byte | its sprite (`byte_80E98C0`) << 16. Without any, the buster (0x2B) or `sub_8011AF2` (0x2C).

**DustCross (form 0xA): charged shot 0x28 and B+Back 0x2A** (both trace-verified in soundmod).
- Routine 0x28 (`sub_8011F64`): damage 0x32 + 10·min(`sub_801265A`, 5), hit param 0x94, element 0x10, action 0x57. Action 0x57 (`sub_80EFC1C`): sub-phase 0 as the buster's (anim 0x0E, the arm, `object_setDefaultCounterTime`, USING_ACTION, sound 0xFF) and on its 2nd tick `sub_80DB800` rolls a junk ball one panel ahead; after 5 ticks sub-phase 4: 31 ticks, then the pointers cleared and `object_exitAttackState`.
- The ball, T3 0xB0 (`sub_80DB6A4`, Param1 0: frozen while dimmed): 18 pixels ahead of its panel's centre, sprite (8, 0xA) anim 0x19. It rolls 6 pixels a tick toward the far column (Timer = the distance to column 6/1 over 6 pixels); on its first tick, and whenever it reaches or passes a panel's centre, it stops if the panel's flags meet `byte_80DB888` (the other side's body, objects); then anim 0x1A, 30 ticks, and on the 5th a hit region (region 1, hit effect 0xA, target 5, self 0x15, modifier 3) and, on a solid panel, sound 0xC0 and `object_crackPanel`. Its Z at spawn is the caller's r3 (the low half of a RAM address), of which the init keeps the fraction; the trace comparison ignores it.
- Routine 0x2A (`sub_8011F84`): lockout 0x28, action 0x58 (`sub_80EFCB4`): anim 0x17 (0x19 in form 0x16), USING_ACTION and MOVING, a vortex (T4#0 effect 0x63, flip = the side) 7 pixels behind and 4 lower with flags 0x14 cleared, sound 0xAD; on the 10th tick `sub_80EFD74` pulls in the obstacles (field-objects.md §4.4); 11 ticks later MOVING off and `object_exitAttackState`. Every tick the vortex's Timer is set to 2, so it lasts two ticks past the action.

**Action 0x1C outside a pause (`sub_80EC39C`)** runs a chip's routine once (`off_80EC3F0[subtype]`, with the attack variables in registers: the user's panel and Z, the element, the parameters, and the damage word plus the bonus's low byte) and idles, 8 ticks later for subtype 0x14 (the ruleset's `actions/instant.rs`; the routines are the content pack's `instant_chip` scripts, chips.md §1.6). FullCust (0xAE, subtype 5, `sub_800AF34`) sets the custom gauge to 0x4000 (`sub_801DFA2`; in the flag-0x40 mode the side's gauge gets 0x1555 instead); the gauge task then raises the full flag.

#### B7. Charged shot (AI.Unk_07 = 1)

**Setup, `sub_8011A7E`**: reached through flag 2, or through `sub_8011A26` when v == 2.
- `b = NS[1] + 1`. If emotion == 5, b = 1. AV.Unk_08 = b*10.
- `s = NS[0x4F]`:
  - If s == 0 → s = 6.
  - Else if s ≠ 6: `r = GetPositiveSignedRNG2()`; mask = 7 if s ∈ {9, 0x23}, else 1; if `r & mask` → s = 6.
- Store s to AV.Unk_0C as a **u32** (this zeroes Unk_0D..0F).
- Zero AV.Unk_02, 03, 04, 05, 06, 0A. Return **0x16**.

**Action 0x16, `sub_80EBE00`** (the pack's `navis/megaman/weapons/charged-shot/weapon.luau`, with the
weapon routine; a sub-phase past 8 reads past its table):
- **Sub-phase 0** (`sub_80EBE20`): on init AV.Unk_10 = 5 (`nullsub_12` is a no-op). Each tick `Unk_10--`. When the result ≤ 0, set sub-phase 4 and run `sub_80EBE54` in the same tick.
- **Sub-phase 4** (`sub_80EBE54`): same as the buster's sub-phase 0 (anim 0x0E, arm T1#5, USING_ACTION, fire on its 2nd tick) with these differences:
  - It fires with `ldrh` damage and Param1 = AV.Unk_0C (6).
  - The flash T1#5 gets r4 = 5.
  - After 5 ticks it moves to sub-phase 8.
- **Sub-phase 8** (`sub_80EBEB2`): same as the buster's sub-phase 4. However, the move-cancel check also runs on the exit tick, because there is no return after `object_exitAttackState`.
- Timeline: R (request), R+5 (anim/arm), **R+6 fire**, R+10 sub-phase 8 init, **R+N+10 exit**. Verified with R = 725, N = 12: fire 731, exit 747.

#### B8. Projectile: T3 index 0, `sub_80C4E58`

The pack's `objects/projectile` (`projectile.luau`); actions fire it with `lib/projectile.luau` (`projectile.fire`,
`sub_800FAAC`; `projectile.spawn`, `sub_80C4FFE`). T3 indices 0x0C, 0x0D and 0x13..0x15 of the T3 jump table run
the same routine, but nothing spawns them. Its kinds, its first parameter, are `off_80C4C78`'s 12-byte records, 40
of them up to the routine's code: the content's `objects/projectile/variants.luau` (`ProjectileVariant`
records). A record: collision self type, target type, hit modifier
(0..2); element byte (3); hit effect (4); sprite category (0xFF: not drawn), index and animation (5..7); the panel
type a hit leaves (8, 0xFF: none); status byte (9); bug code and argument (0xA, 0xB). The routine adds what it does
by kind number, which the pack writes into the records: kinds 7 and 0x15 crack the panel they hit, 0x16 breaks it,
0x22 and 0x24 leave the record's type for the left side's shots and a road the other way (0xC, 0xB) for the right
side's, 0xC bursts, 0x1D climbs. A kind past the table reads on into the code (an error in the port).

**Spawn**, `sub_80C4FFE` (r0/r1 the panel, r3 the Z, r4 the parameters, r6 the damage word):
- `object_spawnType3(0, …, Params = r4)`: the buster's `AV.Unk_0C | AV.Unk_0D<<8` (Param2 the row delta). The spawn
  position is registers (X = the panel Y, Y = r2), which the init overwrites; Z = r3 (the buster's 0x180000)
  stays.
- Then `sub_801155A` sets PanelX/PanelY, Damage (u32 at +0x2C) = r6, Alliance/Flip from the player, and RelatedObject1Ptr = the player.
- Element is set from a clobbered register and is overwritten at init.
- The shot is inserted after the player in the update list, so it initializes and updates in the same tick (§3.3).
- `sub_800FAAC` spawns it on the panel in front and returns `sub_800FAF6` from that panel (§B6).

**Init**, `sub_80C4E7C` (runs in the spawn tick):
- `cfg = 0x080C4C78 + Param1*12`, stored into **RelatedObject1Ptr**. This overwrites the parent pointer (the port
  clears it).
- If cfg[5] ≠ 0xFF: load the sprite, no shadow (`sprite_noShadow`), visible, CurAnim = cfg[7], CurAnimCopy = 0xFF.
  Element = cfg[3]. Set coordinates from panels, then `sub_80C5090` (below). Timer2 = 1.
- `object_createCollisionData`; on failure `object_freeMemory` and return.
- `object_setupCollisionData(cfg[0], cfg[1], cfg[2])`. With cfg 4, 5, 0:
  - Region 1.
  - Self type flags = `byte_8019C7C[4][alliance]` = 0x80000080 / 0x40000080.
  - Target type flags = `[5][alliance]` = 0x15800000 / 0x2A800000.
  - HitModifierBase 0, SelfDamage = obj.Damage.
- Hit effect = cfg[4]. If cfg[9] ≠ 0, the status base = cfg[9]. If cfg[0xA] ≠ 0, `sub_801A4D0(cfg[0xA], cfg[0xB])` (the bug word).
- `object_presentCollisionData`, CurState = 4, then run the update once.

| cfg | Bytes 0..11 | Used for |
|---|---|---|
| 0 | 04 05 00 00 00 FF FF 00 FF 00 00 00 | Plain buster: hit effect 0, no sprite |
| 6 | 04 05 00 00 05 FF FF 00 FF 00 00 00 | Charged shot: hit effect 5 |

**Update**, `sub_80C4F02`:
1. `object_removeCollisionData`.
2. `object_spawnCollisionEffect`. If `FlagsFromCollision & 0x3F800000` and not bit 0 and HitEffect ≠ 0xFF: `AddRandomVarianceToTwoCoords(0xF, x, y, z)` (**one GetRNG2**; x += ((r & 15) - 7) << 16, z += (((r >> 16) & 15) - 7) << 16), then `sub_80E08C4` spawns the hit-spark T4 (index 4).
3. If the battle is over → destroy.
4. If FlagsFromCollision ≠ 0 (hit): Param1 0xC first bursts (`sub_80C5050`); then the panel effects by Param1:
   7 and 0x15 `object_crackPanel`, 0x16 `object_breakPanel_dup2` (break it, or crack it while something stands
   there), 0x22 and 0x24 on a solid panel set cfg[8] (alliance 0) or 0xC / 0xB (alliance 1), any other on a solid
   panel set cfg[8] unless it is 0xFF. Then `object_clearCollisionRegion`, CurState = 8.
5. Else step:
   - If PhaseInitialized == 0: Timer = Timer2 (1), PhaseInitialized = 4.
   - `Timer--`. If ≥ 0 → wait. Else PanelX += front, PanelY += (s8)Param2, Param2 = 0.
   - If `!object_isValidPanel` → Param1 0xC bursts (`sub_80C5014`); `object_clearCollisionRegion`, CurState = 8.
   - Else set coordinates, `sub_80C5090`, `object_updateCollisionPanels`, PhaseInitialized = 0.
6. Always `object_presentCollisionData`. Then `object_updateSprite`.

The shot therefore enters panel X0+k on tick F+2k-1, where F is the spawn tick. Destroy (CurState 8) uses `object_genericDestroy`, and the object is gone the following tick.

**Bursts** (Param1 0xC): a camera shake (3, 0x28; the camera's own RNG, presentation), sound 0xC3, then
`sub_801BD3C` (effect 0 on each valid panel of a region, turned by the side's direction, Z 0; the port's
`battle.region_effects`) and a one-tick hit region (`object_spawnCollisionRegion`: target 5, self 4, hit effect
0xFF, hit modifier 3, the shot's element and damage word). On a hit (`sub_80C5050`), at its panel: effects over
region 0xF (3x3), the hit over 0x10 (the eight around). Off the field (`sub_80C5014`), from two panels back
(PanelX − 2·front, where the shot left): effects and the hit over region 0x11 (the last two columns, three rows).
**Unverified** (no scenario fires kind 0xC).

**Climbing** (`sub_80C5090`, Param1 0x1D): after each set of coordinates, Y += 1 and Z += 1 pixel; Z keeps
accumulating, so the shot rises a pixel per panel. **Unverified**.

#### B8a. Flying shot: T3 index 0xB, `sub_80C60A8`

The pack's `objects/flying-shot` (`flying_shot.luau`, the `flying-shot` kind, spawned with `flying_shot.spawn`,
`sub_80C6248`: r1..r3 the position, r4 the parameters, r6 the damage word, r7 its ExtraVars word). Used by the buster's throw (§B6, kind 6),
the Beast forms' buster (`sub_80EC710`), TrnArrw (`sub_80ECF00`) and navi AI (`sub_8108EE6`). Its kinds are
`byte_80C6038`'s 16-byte records (7, up to the code; the variant records `flying_shot.variants`, the
port's names for kinds 0 to 6: `wave`, `heavy_wave`, `arrow`, `beast_shot`, `slow_beast_shot`, `volley_shot`,
`thrown`; the parameters are the spawn's options, `wait`, `falls` and `palette`):
collision self type, target type, hit modifier (0..2); element byte (3); hit effect (4); sprite category, index,
animation (5..7); highlight (8); range (9); shadow (0xA); status byte (0xB); speed (0xC, u32 16.16). The routine
adds by kind: 6 is a thrown obstacle, 2 sparks over its panel and sounds as it sets off, 5 leaves an effect.

**Init** (`sub_80C60CC`): cfg into RelatedObject1Ptr (the owner forgotten); load the sprite (kind 6: category and
index from the ExtraVars word's bytes 2 and 3; the port takes the obstacle's absorbed look, a record with its sprite,
which the throw carries from the navi's absorbed list), shadow by cfg[0xA]; CurAnim = cfg[7] (kind 6: bits 4..7 of the word), CurAnimCopy = 0xFF,
visible; flip from the object, except kind 6 with sprite index 0x23 (`sub_8002EAC` marks its parts to keep their
own facing; it is drawn unflipped); palette = Param4; Element = cfg[3]; Param4 = cfg[9] (the range); panel from
the coordinates; X velocity = front · speed; collision set up from cfg[0..2], hit effect cfg[4], status base cfg[0xB]
if set; present; Z velocity = −0x12000 if Param3, else 0; CurState 4, and the update runs once.

**Update** (`sub_80C619C`): remove the collision; the hit spark (kind 2: `sub_801A100`, at the center of the panel
under it, 16 pixels up, with the same RNG draw); on a hit → the end. Else while Param2 (a wait) is set, count it
down (kind 2 sounds 0x18A as it reaches 0); after that each tick Z += Z velocity, X += X velocity, and when X passes
the center of its panel (`sub_800E6E8`) Param4 counts down: at 0 the end (kind 5 first spawns effect 7 at its
panel's center). Otherwise the panel from the coordinates, update the collision's panels, and off the field the
end. The end: not visible, `object_clearCollisionRegion`, CurState 8. Always present the collision, and with cfg[8]
highlight its panel. Then `object_updateSprite`. Kinds 2 and 5 and the wait are **unverified** here (TrnArrw and the
Beast buster are other groups' actions).

**Hit timing (observed).** Collision pairs are resolved when an object *removes* its collision data at the start of its update (§7.3), so the result depends on update order:
- If the target updates **after** the shot, the target's HP drops in the same tick the shot enters its panel (injected run 4: alliance 1 fires; shot reaches (2,2) at 611; alliance-0 HP 999 at 611).
- If the target updates **before** the shot, the HP drops on the next tick (injected run 1: shot reaches (5,2) at 611; HP 999 at 612).
- In both cases the shot sets CurState 8 on its next update, and the hit spark and the RNG2 call happen in that tick.

#### B9. Chip use (generic dispatch)

**Hand.** `sub_8010018(alliance)` = `0x020349C0 + 0x50*alliance` (the trace's `chip_blocks`):
- `[0]` = u8 current index i.
- `u16 [2+2i]` = chip IDs (0xFFFF = none), up to 5.
- Parallel u16 arrays at +0x0E, +0x1A, +0x26 and bytes at +0x3E and +0x44 feed `sub_800EDD0`. `sub_800EDD0` returns r0 = id, r1 = `[0x0E+2i]`, r2 = `[0x1A+2i] + sub_800EF34(id, charged) + [0x26+2i]`, r3, and r4 = `[0x3E+i]<<8 | [0x44+i]`.
- The current chip is `sub_8010004()`.

**Sequence**, on the tick the chip request is present in idle (priority row 6):
1. **`sub_800FB54`** returns 0xFFFF if ObjectFlags1 has SLIDING (0x1000) or `flags44 & 0x1000C == 0`. Otherwise:
   - **Normal (flag 4):** `a = sub_80127C0(0)`, then **`object_setAttack2(a)`**.
   - **Flag 8 (charged):** uses AI.Unk_05, or AI.Unk_11 when ChipData[6] == 0x0A, to pick either `sub_80127C0(1)` or a form routine `off_80117D4[...]` with `setAttack2`. This is chip/form territory.
   - If AV.Unk_1B ≠ 0 or the form is 0x0B..0x18: AV.Unk_1D = ChipData[0x0F].
   - Clear flags44 0x1000C. Return (AV.Unk_14, AV.Unk_08, AV.Unk_06).
2. **`sub_80127C0(chargeFlag)`**, "called when you use a chip":
   - Fetch the chip: `sub_800EDD0`, or `sub_800EE26` if flag 0x10000 (then AV.Unk_1B = 1).
   - ChipData = `getChip8021DA8(id)` = 0x08021DA8 + 44*id. `sub_8010D58(ChipData[0x1F])` may substitute another chip.
   - AV.Unk_14 = id, `sub_80126E4`, AV.Unk_04 = chargeFlag, AV.Unk_08 = damage, adjusted in order by `sub_8012C7C` (+bonus, SOUND_HIT_87), `sub_8012A38`, `sub_8012C34`, `sub_8012C4A`.
   - AV.Unk_06 = r2.
   - Chip recovery: NS u16[0x50] (+ a form 6/0x12 extra) → `sub_800E2FC` heal.
   - If ChipData[9] & 4: `sub_800AB46(alliance, 6, 1)`.
   - `sub_800B79A(id)` (statistics).
   - **Return ChipData[0x0B]**: the player action index, dispatched through `JumpTable80EAC60`.
3. Back in `sub_80F0354` (loc_80F057C):
   - If `sub_80106C0(id)` or `sub_8010740(id)` → `object_exitAttackState`, which cancels the chip. These are link-battle interceptions by the opponent's NS[0x0D] bits for ChipData[7] ∈ {1, 2}; they consume the bit and spawn an effect.
   - Otherwise: flags48 &= ~0x10, flags48 |= 0x40.
   - Remote-side chip-name UI `sub_801EB18` (UI).
   - If AV.Unk_1B == 0 and AV.Unk_1C ≠ 5: **`sub_800FC7C`** advances the hand. If `hand[0] < 5` and `hand[2+2*hand[0]] ≠ 0xFFFF`, then `hand[0]++`.

While the chip runs, CurAction = ChipData[0x0B], AV.Unk_1C = 2, and flags48 = 0x40 (0x10 clear). The chip routine ends with `object_exitAttackState`.

**`object_exitAttackState`**:
- CurAnim = 0; AV.Unk_1B = 0.
- If category AV.Unk_1C == 4 (movement): only CurAction = 8 and AV.u16[0] = 0.
- Otherwise:
  - Category 2: AI.Unk_19 = AV.Unk_05. Category 3: AI.Unk_15 = AV.Unk_05.
  - AI.Unk_1A = 0.
  - **Clear flags44 0x1000003F** (all pending requests).
  - `sub_8012EA8`: AI.Unk_1D/1B/1E = 0, clear flags44 0x60000.
  - Clear ObjectFlags1 0x400000.
  - CurAction = 8, AV.u16[0] = 0.
- CurPhase and PhaseInitialized were already 0 from `setAttackN`, so the idle phase-0 init runs on the next tick.

**Buffering rule (verified).** Requests (buster, charged shot, chip) raised during a **movement** survive and fire on the first idle tick after the move. Injected run 3: B released at 627 mid-move, move exits 635, buster action at 636. Requests raised during any other action are discarded:
- Flags 1/2 cannot be raised while a buster/charge request is still pending.
- Flag 4 raised mid-chip was cleared at exit (2280 → 2288 in the original trace).

**Match observations.** Round 1 P1 (alliance 0), hand [0x11 ×4]:

| Tick | Event |
|---|---|
| 638 | A pressed → action 0x37 (ChipData[0x11][0x0B] = 0x37, `sub_80EDAE0`) the same tick; hand index 0→1 |
| 645 | T4#0x48 appears |
| 647-766 | Opponent loses 4 HP per frame |
| 777 | Idle |
| 779, 920 | Repeats (hand index 2, 3); opponent dies at 938 |

Round 2 P1 goes through action 0x1C (1794-1886), which is a form change: NS[0x2C] becomes 0x0C and AI.Unk_05 = 5, Unk_07 = 0xFF, Unk_11 = 0x1E. After that, A charges chips (20-frame threshold, row 5) and chips fire on the **release** of A (2003 normal chip, 2338 charged → action 0x52). Round 2 is therefore **not** base-form behaviour.

#### B10. `sub_80F0608` (per-form per-tick hook for AIIndex 0)

`sub_80F0608` is and is called from `sub_80EA484` after the action dispatch.

- If not paused, and NS[0x29] == 5 or NS[0x2C] ∈ {5, 0x11}: this looks at the current hand chip and needs ChipData[9] & 2 and ChipData[4] == 1. The per-chip counter `u16 hand[0x26+2i]` is only touched while it is below the limit (0x64, or `byte_802136D[...]`):
  - During an A-charge (AI.Unk_1E == 1) whose AI.Unk_1B is ≥ 15, the counter is incremented and Unk_1B is reset to 10. If Unk_1B < 15, nothing happens.
  - When AI.Unk_1E ≠ 1, the counter is reset to 0.
  
  Not base form; it feeds the chip's extra value in `sub_800EDD0`. With no chip left, `sub_8010004` gives 0xFFFF and the chip lookup reads past the table (ChipData[9] = 0x30 there: no damage bit), so nothing happens. Trace-verified in ChargeCross (form 5), soundmod round 3.
- Then, if NS[0x29] == 0: if NS[0x2C] == 0x18 → Z = 0x140000. Otherwise, if CurAction ≠ 0x50 and ObjectFlags1 has no BUBBLED (0x80000000) → **Z = 0**.

For base MegaMan the only effect is clamping Z to 0 each tick. **It has nothing to do with charging.**

#### B11. Verification

The machgun match contains **no buster or charged shot at all**. P1 moves twice and uses chips; P2 idles. I verified the buster by replaying the same match under emulation with injected inputs. Injected keys appear in the game's input record 5 frames later.

| Run | Input | Observed |
|---|---|---|
| 1 | P1 B tap, record press 605 / release 606 | Action 0x11 at 606; anim 0x0E and USING_ACTION at 607; T3 shot at (3,2) at 608, (4,2) 609-610, (5,2) 611; opponent 1000→**999** at 612; shot CurState 8 at 612, gone 613; hit spark T4#4 612-616; idle 624 (N=12); RNG2 steps at 606 (`sub_8013D5E`) and 612 (hit jitter) |
| 2 | P1 B held 605-724, released 725 | Counter AI+0x1B = 1 at 605, Unk_1D = 1 from 10, **100 at 704** (Unk_1D=2); action 0x16 at 725; anim at 730; fire 731; opponent 1000→**990** at 735; idle 747 |
| 3 | B tap during move (press 626, release 627) | Charge counter ran during the move (flags48 = 0x10); flag 1 held through the move; buster at 636 (N=8, k=1); HP 999 at 640; idle 650 |
| 4 | P2 B tap (release 606) | Shot enters (2,2) at 611 and alliance-0 HP drops at **611** (same tick; target after shot in list) |

No T1/T4 objects exist while charging; the charge has no object.

### 12.H Damage intake, hit reactions, status effects and deletion

Scope: the PvP player (T1 index 0, `AIData.ActorType == 2`, `AIIndex == 0` = MegaMan base form). "Tick" means one call of `battle_8007800`, which is one trace frame. `coll` is the player's CollisionData (`obj+0x54`), `ai` is its AIData (`obj+0x58`), `f1` is `coll.ObjectFlags1` (+0x3C) and `f2` is `coll.ObjectFlags2` (+0x40). The collision protocol is in §7.3; the hit-registration details of `sub_3007218` belong to the collision spec. This section starts where that engine has written the per-tick hit results into `coll`.

#### H1. Where damage processing sits in the player tick

`sub_80EA484` (the CurState 4 handler) runs these steps in order: `sub_8012E74`, `sub_8013DA0`, then **`sub_801AC6C`** (collect the hit results), then **`sub_801AF44(action_table)`** (apply damage, check status, dispatch the action). After that come the per-form hook, `sub_80107D4`, `sub_80139C4`, `sub_80100EC`, and finally `object_presentCollisionData`, which runs only if the battle is not paused.

**Hit-result lifetime.** `object_presentCollisionData` runs at the end of each unpaused tick. It does the following:
- It stores the caller's r1 into +0x54 (the code computes `+0x54 | FlagsFromCollision` into r0 but stores r1; verified in the ROM: `str r1, [r4,#0x54]` at 0x0801A022).
- If there is no dimming, it zeroes `HitModifierFinal` (+0x0F), +0x03 and +0x54.
- It always zeroes `StatusEffectFinal` (+0x11) and the 0x40 bytes at +0x68..+0xA7. That range holds +0x70 flags, +0x74..0x77, +0x80 FinalDamage, +0x82..0x8D PanelDamage1..6, +0x8E/+0x90/+0x92 and +0xA4.
- It then re-registers the collision (`sub_300777C`).

Hits land in `coll` whenever the collision engine resolves an overlap. That happens when an attacker's region is removed, or when the player's own `object_removeCollisionData` runs at the start of `sub_801AC6C`. In the second case the engine runs inside the player's own update. **Consequence:** a hit is consumed in the first player update at or after its registration, and it can be consumed in the same tick. The emulated original confirms both cases. At frame 646 a T3 region (`sub_80C52B0`) wrote `coll+0x8C = 4`, and the victim applied it at frame 647. At frame 732 a charged shot was resolved inside A0's own `sub_801AC6C` and applied in the same tick.

#### H2. `sub_801AC6C` — collect this tick's hit results

The function runs these steps in order:

1. Call `sprite_clearFinalPalette`.
2. If `battle_getFlags() & 1 == 0` (the battle is not "active": this flag is set when a turn starts and cleared during the custom screen), **return**. In that case collision data is not even removed.
3. Call `object_removeCollisionData`. The engine may write new results here.
4. If `battle_isBattleOver()` is true, **return**. `battle_isBattleOver()` returns 1 unless `BattleState[0x12] != 0 && BattleState[0x13] != 0 && BattleState[0x0B] == 0`; bytes 0x12/0x13 are the alive counts of alliances 0 and 1.
5. If `f1 & DEAD (0x100)`, **return**.
6. Call these functions in order. The table keeps only what matters for state.

| # | Function | Effect |
|---|---|---|
| a | `sub_801A802` (21369) | Barrier (`coll.Barrier` +0x06, chip-defined). If a barrier is up, it subtracts the summed element damage (+0x94..+0x9C, halved rounding up on holy panel type 5) from barrier HP `+0x16` when the damage is at least threshold `+0x17`. When barrier HP reaches 0 or below the barrier breaks. It then **zeroes** PanelDamage1..5, +0x8E/+0x90/+0x92, HitModifierFinal, StatusEffectFinal, +0xA4, +0x74, +0x76, and clears `FlagsFromCollision & 0x50`, so the hit is absorbed. **+0x8C is not zeroed.** Barrier timer `+0x1A` (u16, 0xFFFF = infinite) is decremented when not dimmed. Types 8 and 10 have special regeneration rules. Returns immediately while paused. Barrier = 0 means no-op. |
| b | `sub_801A186` (20433) | Skipped if dimmed, paused or `coll.Region == 0`. **Poison panel** (type 4, unless `f1 & 0x08000028`): `PoisonPanelTimer` (+0x08) -= 1; when it drops below 0, set it to 6 and do `coll+0x8C += 1`, i.e. 1 HP every 7 ticks. On any other panel the timer is reset to 0. **Grass panel** (type 6) with `obj.Element & 0xF == 4` (wood): +1 HP (`object_addHP`) when `BattleState[0x0E] == 0` (a 0..19 cycle), or when HP ≤ 9 and `BattleState[0x16] == 0` (a 0..179 cycle). |
| c | `sub_801A36A` (20703) | Ice/conveyor. If not paused/dimming and `ai+0x38 != 0`: decrement it; if it reaches ≤ 0, jump straight to **X**. Otherwise: return if `f1 & 0x100040` (DRAG or CANNOT_SLIDE). On a conveyor (panel types 9..0xC), if `ai+0x38 == 0` and not `f1 & 0x24`, set `f2 \|= 0x10` and `Unk_0f = 3`, then return. Else, if `f1 & MOVE_COMPLETE (0x80000)`, go to **X**. **X**: clear MOVE_COMPLETE. If the panel is ice (type 7), the primary element is not aqua, not `f1 & 0x24`, and `f1 & AFFECTED_BY_ICE (0x2000000)` (both players have it), set `f2 \|= 0x10` and `Unk_0f = 2`. |
| d | `sub_8010230` (2796) | HP-bug navicust drain. If not paused/dimming and HP > 1: with `n = [0,40,35,30,25,20,15,10][NaviStats[0x18]]`, `ai+0x09` counts up and every n ticks does HP -= 1. If n == 0, `ai+0x09 = 0`. The trace has NaviStats[0x18] = 0. |
| e | `sub_802CFF8` | If `coll+0x76 & 0x40` and the own-side defensive-chip record `unk_2036720[alliance]` is active, cancel that record (`sub_802CEA6`, sound 0x8E). |
| f | `sub_802CEF4` | Defensive-chip hooks: chip IDs 0xBB, 0x157, 0xBC, and `ai+0x48 & 0x200000/0x800`. These may set `ai+0x44` bits 0x200/0x8000/0x400 and wipe the whole hit (same zero list as (a), plus +0x74, +0x1C/+0x2A/+0x2C and `f2 &= ~0x301BE`). **No-op in plain PvP** (the record is empty). Chips spec. |
| g | `sub_801A6B4` / `sub_801A720` (21176/21244) | Bug infliction by the attack (`coll+0xA4`). 0xF4, or 0xF7 when a hex digit of HP is 4: NaviStats[0x18] += 1 (max 7). 0xF6: NaviStats[0x18] += 2 and [0x19] += 2 (max 7), then paralysis `+0x1C = 150` with `f2 \|= 8`, and unless NameID ∈ [0x173, 0x17E] also `f2 \|= 0x20` with `+0x20 = 1200`. |
| h | `sub_80139F6` (10368) | More NaviStats edits keyed by `coll+0xA4/+0xA5` (0x18/0x19/0x54/0xFF/0xFE…). No-op when +0xA4 == 0. |
| i | **`sub_801AEB0`** (22114) | Converts `HitModifierFinal` (hm, the OR of every attacker's `HitModifierBase`) into requests. **`hm & 1`** and not `f1 & 0x220000` (SUPERARMOR or ANGER) → `f2 \|= 4` (flinch). **`hm & 2`** → `f2 \|= 2` (mercy invincibility). **`hm & 0x3C`**: if also `hm & 0x40` → `f2 \|= 0x100` (drag), `f2 &= ~4`, `obj.Unk_0f = 1`; else if not `f1 & 0x100040` → `f2 \|= 0x10` (slide), `obj.Unk_0f = 1`. |
| j | `sub_800EB26` | **Counter hit.** If `FlagsFromCollision & 0x40` and `StatusEffectFinal` ∉ [0x60, 0x65]: `StatusEffectFinal = 0x12` (paralysis for 150 ticks, see §H5), `f2 \|= 0x4000`, `f2 &= ~6` (no flinch, no mercy). |
| k | `sub_8013F1E` (11039) | Navicust "hit bug" (NaviStats[0x16]: 1 → status 0x32, 2 → 0x22, 3 → HP-bug +1). Fires once per hit sequence (latched in `ai+0x1C`) when `f2 & 0x104` and any damage was taken. It is a no-op when [0x16] == 0, as in the trace. |
| l | `sub_801A554` (20968) | **Applies `StatusEffectFinal`** (see §H5). Skipped when NaviStats[0x29] == 7, [0x2C] ∈ {7, 0x13}, or [0x52] != 0; all three are 0 in the trace. |
| m | `sub_801A2CC` (20615) | If `FlagsFromCollision & 0x10`: `obj.Chip = 0xFFFF`, and the player's hand cursor (`sub_8010018(alliance)`, byte 0) advances by one if the next hand entry is not 0xFFFF. In effect this is "lose the current chip". |
| n | `sub_801A324` (20667) | Drain-heal credits. `opp.ai+0x10 += coll+0x92`. Then `k = self.ai+0x10` and `self.ai+0x10 = 0`. If `(MaxHP/10)*k != 0`: heal that much, spawn a T4 kind 0 with params 6, play sound 0x8A. |
| o | **`object_calculateFinalDamage1`** | `s = 1` if the player stands on a holy panel (type 5), else 0. Each PanelDamage1..5 (+0x82..+0x8A) becomes `(d + (1<<s) - 1) >> s` and is written back. The sum goes through **`sub_802CE10`**, which uses the per-side record `dword_203CFB0 + 0xC*alliance`: if `rec+8 == this object`, it adds `rec+2`; otherwise it sets `rec+0 = max(rec+0, sum)`. The result is stored in **`FinalDamage` (+0x80)**. **+0x8C (PanelDamage6) is not included and not halved.** |
| p | `sub_801A420` (20799) | If `CounterTimer` (+0x0D) != 0, decrement it. |
| q | `sub_80143FC` (11691) | If not paused/dimming: when `f1 & 0xC00` (FLINCHING or PARALYZED), `ai+0x4C += 1`; otherwise `ai+0x4C = 0`. |
| r | `sub_80142DC` (11539) | Anger trigger. If battle mode != 1, NaviStats[0x29] == 0, [0x2C] == 0, not already `f1 & ANGER (0x200000)`, and (`ai+0x4C ≥ 120` or `FinalDamage ≥ 300`): `f2 \|= 0x200`. |
| s | `sub_8010198` (2708) | If `coll+0x26 != 0 && FlagsFromCollision != 0`: `coll+0x26 = 0` (cancels the timed "UNK_4" invisibility, see §H6). |
| t | `sub_801A648` (21106) | If not paused, `coll+0x24 != 0`, `FlagsFromCollision & 4` and not `& 0x1000`: `coll+0x24 = 0`. This lets a mercy-piercing hit end mercy early. |
| u | `object_spawnHiteffect` | If not paused and `FlagsFromCollision & 0x20000`: sound 0x6E, **1× `GetRNG2`** via `AddRandomVarianceToTwoCoords(0xF, X, Y, Z+16<<16)`, then spawn T4 kind 4 with params 8 (`sub_80E08C4`). The RNG is never used in the trace. |

`AddRandomVarianceToTwoCoords(mask, x, y, z)` makes one `GetRNG2` call, r. It returns `x += ((r & mask) - (mask>>1)) << 16` and `z += (((r>>16) & mask) - (mask>>1)) << 16`. `y` is unchanged.

#### H3. `sub_801AF44` — apply damage, check status, dispatch the action

`r6 = coll` and `r7 = ai` throughout. The top block runs when not paused, or when paused but `CurAction == 0`. Otherwise it goes straight to the TAIL.

1. `sub_801A42E` (20811): if `coll+0x74 != 0 && FinalDamage != 0`, spawn the weakness "!" effect (T4 kind 0x6B, via `sub_80E8124`).
2. `sub_801A4A6` (20874): if `coll+0xA4 ∈ {0xF4, 0xF7}`, spawn T4 kind 0x6B with r4 = 3.
3. `sub_801A45C` (20837): if `FlagsFromCollision & 0x40` (counter): in multiplayer, `sub_802E032(opp, 0x1500)`; then `sub_800AB46(opp, 8, 1)` (stat counter, byte `0x203EAE0+16*side+idx`, +1 saturating at 0xFF) and `CounterTimer = 0`. If the battle is not over, show the "COUNTER" HUD text (`sub_801E270`; HUD only) and play sound 0x86.
4. `sub_801A506` (20937): if `coll+0x75 != 0 && FinalDamage != 0`, set `ai+0x44 |= 0x80000000` (elemental-weakness hit).
5. **`applyDamageToPlayer_801ba12`** (23471):
   - Let `d = FinalDamage`. If `d != 0`:
     - `ai.TotalDamageTaken = min(0xFFFF, +d)` (`sub_8010548`).
     - Undershirt: if HP > 1, `f1 & 0x40000` and `HP ≤ d`, then `d = HP - 1`.
     - `object_subtractHP(d)`: `HP = max(0, HP - d)`, which also returns the new HP.
     - Play sound 0x6D, or 0x6B when this is the local player.
     - If the new HP == 0, go to DEATH. Otherwise call `sprite_forceWhitePalette` and fall through.
   - Always (and after the fall-through): `object_subtractHP(coll+0x8C)`. If HP == 0, go to DEATH.
   - DEATH: if `sub_802DD2A()` (Cross form with `ai+0x48 & 0x4000`; 0 for base MegaMan), set `ai+0x44 |= 0x8000000`; otherwise set **`f2 |= 1`**.
   - Finally `sub_801A200` (20496): Full-Synchro/emotion bookkeeping. If `coll+0x90 & 0x8000` (a counter was landed on this player) and the opponent is in normal/beast form without an active synchro, call `sub_8015BEC(opp, 0xFF)`. Then `NaviStats.Mood -= coll+0x8E` (minimum 1, only if Mood != 0).
   - This step runs every tick in the top block, including dimming. It is **not** gated on battle-over or DEAD.
6. `sub_801BADE` (23577): if `CurAction == 7` (bubble) and `FinalDamage != 0`, pop the bubble (`sub_801A2B0`).
7. If `f1 & DEAD` → TAIL.
8. If `f2 & 1`: clear it, set `f1 |= DEAD (0x100)`, call **`object_setAttack0(2)`** → TAIL.
9. If `ai+0x48 & 0x2000`, `& 0x10000` or `& 0x20000` → **DISPATCH** directly, skipping the TAIL. If `ai+0x48 & 0x40000` and `sub_8015766()` → return (Cross-form lanes).
10. If dimming → TAIL.
11. `obj.PreventAnim (+0x18) = 0`.
12. `ai+0x44` special requests (Cross/Beast): 0x8000000 → `ai+0x48 |= 0x2000`, action 0x4C → DISPATCH. 0x40000000 → `|= 0x10000`, action 0x30 → DISPATCH. 0x80000000 → cleared; if NameID ∈ [0x1AC, 0x1C1], un-cross → return. None of these fire for base MegaMan.
13. **Drag request** `f2 & 0x100`:
    - Clear it. If `obj+0x5C == 0`, `obj+0x5C = obj.CurState..PhaseInit` (u32 save).
    - Call `sub_801011A`, `sub_801A29A` (unfreeze) and `sub_801A2B0` (unbubble).
    - Clear paralysis (`sub_801A284`) unless `f2 & 0x4000`, or unless `CurAction == 4` and not `f2 & 2`.
    - `f2 &= ~0x4000`; `CurAction = 5`, `CurPhase = 0` (PhaseInitialized **not** touched), `obj.Unk_0d = 0` → TAIL.
14. If `f1 & DRAG (0x100000)`: `CurAction = 5` → TAIL.
15. `obj.Unk_0d = 0`. If `f2 & 0x10` (clear it) or `f1 & SLIDING (0x1000)`: run the **slide machine `sub_80166B6`** (§H4.3). Otherwise `obj.Unk_1f = 0`.
16. **Flinch request** `f2 & 4`:
    - Clear it and call `sub_801011A`: `ai.AttackVars+0x1D = 0`, `+0x1E = 0`, then `sub_80E1662`. That writes to `*(ai+0x40)`, which is NULL in PvP, so treat it as a no-op when null.
    - `a = sub_801BA92()` (23534): if `CurAction == 4`, a = 1; and if not `f2 & 0x4000` and `f2 & 2`, clear paralysis and set a = 2. Otherwise a = 0.
    - `b = sub_801BABE()` (23558): if `CurAction == 6`, b = 1; and if `f2 & 2`, unfreeze and set b = 2. Otherwise b = 0.
    - If `(a|b) == 0` or `(a|b) & 2`, call **`object_setAttack0(3)`**. When a|b == 1 (paralyzed or frozen, and the hit lacks the mercy bit) there is no flinch.
    - Then `f2 &= ~0x4000`.
17. `sub_801A5EE` (21060) — **mercy invincibility** (§H4.2).
18. `sub_800E730` — **status timers** (§H5).
19. `sub_8010162` (2678): the `coll+0x26` timer.
    - If it is 0xFFFF (infinite), skip to the flag update.
    - Otherwise decrement it. If the result is < 0, clear `f1 & 4` and return. If it is exactly 0, store it and play sound 0x94.
    - Flag update: set `f1 |= 4` (UNK_4), or clear it if `f1 & USING_ACTION (0x400000)`.
20. `sub_8014326` (11576) — anger. For a player navi whose mood (`sub_8015B54`) is not 5 or 1:
    - If not angry and `f2 & 0x200`: clear it, set `f1 |= ANGER`, `ai.Anger (+0x34) = 600`, mood = 0x80.
    - If angry: clear `f2 & 0x200`; `Anger -= 1` if nonzero; at ≤ 0, `sub_80143A6` (NaviStats[0x0E] = 0x80, clear ANGER, Anger = 0, `ai+0x4C = 0`).
    - For mood 5 or 1: clear both.
21. `sub_8014498` (11815): if not battle-over, `ai+0x36 != 0` and HP ≠ 1, HP -= 1.
22. `sub_802E1D8`: `sub_802E070(alliance)+0x30` (u16) -= 1 if nonzero.

**TAIL** (loc_801B142):
- Call `sprite_zeroColorShader` and set `f2 &= ~0x4000`.
- Call the colour-shader helpers `sub_80143E4`, `sub_801690A`, `sub_8016860`, `sub_80168C8`, `sub_80168F0`, `sub_8016CA4` and `sub_801728E`. They are render-only.
- Call **`sub_8016934`** (visibility, §H4.2).
- Then choose one:
  - DEAD → DISPATCH.
  - Paused and `CurAction != 0` → `sub_8017BC0` (the pause handler, used for chips and transformations; out of scope here).
  - Dimming → **`sub_8017AB4`** (§H4.4).
  - Otherwise → DISPATCH.

**DISPATCH** is `sub_801B9E6` (23444). If `CurAction < 0x10`, it calls `table[CurAction]`, which for AIIndex 0 is `off_80EA52C` = [0 `sub_8016380`, 1 `sub_8017888`, 2 `sub_80173F4`, 3 `sub_80174FE`, 4 `sub_80175B8`, 5 `sub_80178B6`, 6 `sub_8017688`, 7 `sub_8017768`, 8 `sub_80EA734`]. Otherwise it calls `JumpTable80EAC60[CurAction-0x10]`, or `sub_80EAD9C` if `AttackVars+0x1D == 1`. The **new action runs in the same tick** as the `setAttack0` call.

`object_setAttack0(n)` does the following: `CurAction = n`; `CurPhase = 0`; `PhaseInitialized = 0`; `AttackVars+0x00 (u16) = 0`; `AttackVars+0x1C = 0`; then `sub_801011A`.

#### H4. Hit reactions

##### H4.1 Action table (AIIndex 0)

| CurAction | Handler | Meaning | Entry | Exit |
|---|---|---|---|---|
| 1 | `sub_8017888` (17979) | Battle start → control | Set at init | Same tick: in multiplayer and for a NameID with type byte 2, if `ai+0x40 == 0`, call `sub_80E1620`. Then `CurAction = 8`, phase = 0. |
| 2 | `sub_80173F4` (17481) | **Deletion** | step 8 above | CurState 8 (§H6) |
| 3 | `sub_80174FE` (17612) | **Flinch** | step 16 | → action 8 |
| 4 | `sub_80175B8` (17688) | Paralysis | `sub_800E730` | when `f1 & 0x800` clears → action 8 |
| 5 | `sub_80178B6` (18004) | Drag / knockback | steps 13–14 | → action 8, or → 4 if paralyzed |
| 6 | `sub_8017688` (17776) | Freeze | `sub_800E730` | when `f1 & 0x10000` clears → action 8 |
| 7 | `sub_8017768` (17869) | Bubble | `sub_800E730` | when `f1 & 0x80000000` clears → action 8 |
| 8 | `sub_80EA734` | Normal control (move/buster/chips) | — | (movement and buster spec) |

The player's normal "free" action is **8, not 0**. Every reaction exits to `CurAction = 8`, `CurPhase = 0`, `PhaseInitialized = 0` and `CurAnim = 0`.

**Common init (actions 3, 4, 6, 7).** These run on the first tick, while `PhaseInitialized == 0`:
- Call the per-form hook: `sub_800F3E8`, `sub_800F394`, `sub_800F3B0` or `sub_800F3CC`. For AIIndex 0 these are `nullsub_105`, except the flinch and drag hooks, which call `sub_80F06CE`. That hook resets `obj.RelatedObject2Ptr`, which is NULL in PvP.
- Set `f1 |= USING_ACTION (0x400000)`. Flinch also sets `f1 |= FLINCHING (0x400)`.
- Clear GUARD and CANNOT_SLIDE (`f1 &= ~0x41`); flinch also clears DRAG (`~0x100041`).
- Cancel the `coll+0x26` invis timer (`sub_80101C4`).
- `ai+0x48 &= ~0x20005F` (`sub_801031C`).
- `sub_8012EA8`: `ai+0x1B/+0x1D/+0x1E = 0` and `ai+0x44 &= ~0x60000` (drops charge state).
- **If not `f1 & SLIDING`:** `PanelXY = FuturePanelXY`, remove the panel reserve, set coordinates from the panel, update the collision panels, and set `Z = 0`. This snaps a player who is mid-move onto the destination panel.
- Set `CurAnim` (1 for flinch, 2 for the others) and `CurAnimCopy = 0xFF`.
- Refresh the sprite flip. Set `RelatedObject1Ptr = 0` and `ai+0x68 = 0`.
- Call `sub_800AB46(alliance, 3, 1)` (stat counter).
- Set `PhaseInitialized = 4`.

Action-specific extras:
- Flinch also calls `sub_801A284`, `sub_801A29A` and `sub_801A2B0`, which clear paralysis, freeze and bubble, and then `sub_8011450`.
- Freeze also calls `sub_801A67E` (`coll+0x24 = 0`, i.e. loses mercy) and `sub_800EB08`, and plays sound 0x118.
- Bubble plays sound 0x12D.

##### H4.2 Flinch, mercy invincibility and visibility — trace-verified

- **Flinch (action 3).** On the init tick `Timer = 23`. **Every tick, including the init tick:** `Timer -= 1` (u16 store). If the 32-bit result is < 0: clear `f1 & 0x400400`, `ai+0x44 &= ~0x3F`, `CurAnim = 0`, `CurAction = 8`, phase = 0. **The Timer is left at 0xFFFF.**
  - The flinch lasts 24 ticks: the Timer reads 22, 21, …, 0 at the end of ticks t0…t0+22, and at the end of t0+23 the state is `[4,8,0,0]` with Timer 65535.
  - A new flinching hit during a flinch calls `setAttack0(3)` again, which restarts all 24 ticks.
  - Experimental check: the machgun battle under emulation with HP poked to 1000 at frame 2340. The first hit at 2346 gave `[4,3,0,4]`, t = 22, f1 = 0x02400400. The re-hit at 2361 gave t = 22 again. At 2383, t = 0. At 2384 the state was `[4,8,0,0]` with t = 65535.
- **Mercy invincibility** (`sub_801A5EE`) uses **`coll+0x24` (u16)** and is gated on `battle_getFlags() & 1`. It runs in step 17 of the normal path, so it does not tick while paused, while dimmed, or once DEAD.
  - If `coll+0x24 == 0 && f2 & 2`, set `coll+0x24 = 120`. Then always clear `f2 & 2`.
  - If `coll+0x24 != 0`, decrement it. While the result is > 0, set `f1 |= FLASHING (0x200)`. When it reaches 0, play sound 0x94 if `f1 & INVIS (0x2)`, then clear `f1 & 0x202`.
  - The trace matches: the dumped counter was 119 at the end of the hit tick.
  - Mercy comes from `HitModifier & 2`, independent of flinch (`& 1`). Chip 0x52's first hit had hm = 1 (flinch only); its later hit had hm & 2.
  - While `f1 & 0x202` is set, the collision engine ignores attackers without `SelfCollisionTypeFlags & 4` (`sub_3007218`).
- **Visibility bit (object header Flags & 0x02)** is part of the traced "flags" byte. `sub_8016934` (16238) runs in the TAIL:
  - If not dimmed, set VISIBLE.
  - Then, if not DEAD, `f1 & 0x202` and **`coll+0x24 & 2`** (the value after this tick's decrement), clear VISIBLE. Counter values 119 and 118 are hidden, 117 and 116 are shown, and so on: 2 ticks off, 2 on. This is trace-verified: flags alternate 0x15,0x15,0x17,0x17,…
  - Also, if `obj.Alliance != BattleState[0x0D]` (the local side; 0 on core 0) and the local navi (`sub_80103BC(alliance^1)`) has `f1 & BLIND (0x2000)`, VISIBLE is cleared. **This is view-dependent**: the two consoles differ here.
  - The deletion sequence clears VISIBLE itself.

##### H4.3 Knockback: slide (push) and drag — code only, not exercised in any trace

**Push vector.** `sub_800E468(1)` → `sub_800E548` (4887) uses `hm = HitModifierFinal`:
- `off = 5` if `hm & 0x80`, else 0.
- `i` = index of the lowest set bit among `hm` bits 2..5 (0x04 → 0, 0x08 → 1, 0x10 → 2, 0x20 → 3), or 4 if none.
- Read the entry `byte_800E58C[(i + off) * 3]`, which gives (dx, dy, count).
- Final `dx *= (alliance == 0 ? +1 : -1)`, from `object_getEnemyDirection`.
- The vector is zeroed if the first target panel fails `sub_800E5AC` (: a valid panel plus an alliance/AirShoe-dependent panel mask).

| hm bits | (dx, dy, count) | | hm bits | (dx, dy, count) |
|---|---|---|---|---|
| 0x04 | (+1, 0, 6) | | 0x80\|0x04 | (0, −1, 6) |
| 0x08 | (−1, 0, 6) | | 0x80\|0x08 | (0, +1, 6) |
| 0x10 | (+1, 0, 1) | | 0x80\|0x10 | (0, −1, 1) |
| 0x20 | (−1, 0, 1) | | 0x80\|0x20 | (0, +1, 1) |

A count of 6 means "until blocked". `Unk_0f` = 2 (ice) and 3 (conveyor) use other vector sources: `sub_800E4C8` and `sub_800E500`.

**Slide machine** (`sub_80166B6`; `f2 & 0x10`, no action change, runs alongside the current action). It is state `obj.Unk_1f`.
- State 0, `sub_80166D0`:
  - Set `f1 |= SLIDING`, snap Panel to FuturePanel, update collision panels, remove the reserve.
  - Compute the vector into `Unk_1c`/`Unk_1d` and the tile count into `Unk_1b`.
  - If count > 0: `FuturePanel = Panel + d`, reserve it, `Unk_1e = 4`, `Unk_1f = 4`.
  - Otherwise: clear SLIDING and set `Unk_0f = 0`; if `Unk_0f` was 3, also set `ai+0x38 = 5`.
- State 4, `sub_8016730`, runs each tick:
  - `Unk_1e -= 1`. While > 0: `X += dx*0xA0000`, `Y += dy*0x60000` (10 px or 6 px per tick), then set panels from coordinates. So 4 ticks per panel.
  - When it reaches 0:
    - Remove the reserve, snap to FuturePanel, and recompute the remaining tiles. Ice adds a tile for non-aqua; conveyors redirect.
    - `Unk_1b -= 1`. If > 0 and the next panel is valid: reserve it, `Unk_1e = 4`.
    - Otherwise: set `coll.Direction`, `Unk_1f = 0`, clear SLIDING, `Unk_0f = 0`.

**Drag (action 5)**, `sub_80178B6`, dispatches on `obj.Unk_0d` ∈ {0, 4, 8}:
- `sub_80178D4` (init):
  - Set `f1 |= 0x500000`. `CurAnim` = 2 if paralyzed, 0 if SUPERARMOR (0x20000), else 1.
  - Snap Panel to FuturePanel. Clear `f1 & 0x1441`, cancel the invis timer, remove the reserve, call `sub_800AB46(side, 3, 1)`.
  - `(Unk_1c, Unk_1d, Timer2) = sub_800E45E()` (the vector above).
  - If count > 0 and the target is valid: `XVelocity = dx*0xA0000`, `YVelocity = dy*0x60000`, reserve the target, `Unk_0d = 4`.
  - Otherwise: `Timer = 24`, `Unk_0d = 8`.
- `sub_8017992`: step X, then Y, by velocity until the target coordinate is reached (`sub_800E6E8`).
  - On arrival: remove the reserve. Ice (type 7), non-aqua: `Timer2 += 1`. Then `Timer2 -= 1`; if > 0 and the next panel is valid, retarget.
  - Otherwise: snap, `Timer = 20`, `Unk_0d = 8`.
- `sub_8017A38`: `Timer -= 1` until < 0.
  - If paralyzed: clear DRAG, `CurAction = 4`.
  - Otherwise: clear `f1 & 0x501800`, `ai+0x44 &= ~0x1000043F`, `ai+0x48 &= ~0x200000`, `f2 &= ~0x10`, `Unk_1f = 0`, anim 0, `CurAction = 8`.
- Because step 14 re-asserts `CurAction = 5` while DRAG is set, drag cannot be overridden by flinch.

##### H4.4 Dimming — RNG use

`sub_8017AB4` (18237) replaces the action while `battle_isTimeStop()` (`battle_getFlags() & 4`). The first part handles multiplayer chip-trigger logic (the chips spec). The shake part works as follows:
- If `PreventAnim == 0`: save `X16` in +0x30 and `Z16` in +0x32, set `Unk_19 = 0` and `PreventAnim = 4`.
- If `FinalDamage != 0`, set `Unk_19 = 30`.
- If `Unk_19 != 0`: decrement it, then write `X = (Unk_30 << 16) + ((r & 3) - 1) << 16` and `Z = (Unk_32 << 16) + (((r >> 16) & 3) - 1) << 16`, with **one `GetRNG2` per tick** (mask 3). Y is kept.
- Otherwise restore X16 and Z16.
- Step 11 resets `PreventAnim = 0` on the first undimmed tick. If dimming ends while `Unk_19 > 0`, the perturbed X/Z are not restored here. UNCERTAIN whether a later handler fixes them.

Damage is still applied while dimmed (step 5 runs before step 10). Status timers, mercy and flinch requests are not processed until dimming ends. Requests (f2 bits) persist.

#### H5. Status effects

**Applying a status** (`sub_801A554`). Let `s = StatusEffectFinal` (+0x11, set by the collision engine from the attacker's +0x10, or by the counter to 0x12). If `s != 0`:
- Look up `e = off_80209EC[(s>>4)-1][s & 0xF]`. Each entry is 8 bytes: `u32 f2bits, u16 duration, u8 collOffset`. (Engine: each entry is a status definition, rules/status.luau, which a hit carries by handle; the range tests on `s` below and in the counter are the definition's `cancels_flinch` and `survives_counter`.)
- Write the duration to `coll+collOffset` and set `f2 |= f2bits`.
- If `s` ∈ [0x50, 0x55], also set `f2 &= ~6` (freeze cancels flinch and mercy).

| Group (s>>4) | Status | Timer field | f2 request | f1 flag | Action | Durations for s&0xF = 0,1,2,3,4,5(,6) |
|---|---|---|---|---|---|---|
| 1 | Paralysis | +0x1C | 0x08 | 0x800 | 4 | 90, 120, 150, 4, 300, 600 |
| 2 | Confusion | +0x1E | 0x80 | 0x8000 | — | 480, 720, 960, 4, 300, 600 |
| 3 | Blind | +0x20 | 0x20 | 0x2000 | — | 480, 720, 1200, 4, 300, 600 |
| 4 | Immobilize | +0x22 | 0x40 | 0x4000 | — | 2, 60, 120, 4, 300, 600, 30 |
| 5 | Freeze | +0x2A | 0x10000 | 0x10000 | 6 | 150, 150, 150, 4, 300, 600 |
| 6 | Bubble | +0x2C | 0x20000 | 0x80000000 | 7 | 150, 150, 150, 4, 300, 600 |
| — | Invulnerable timer | +0x28 | — | 0x08 | — | set by chips |

**Timers** (`sub_800E730`). Every timer: `t = u16 - 1` (32-bit). If `t ≤ 0`, the status ends: its f1 flag is cleared and the field is stored as 0. Otherwise it is stored and the status is active. Sequence per tick:
1. **Paralysis.**
   - Expiry clears 0x800 and `f2 & 8`.
   - While active:
     - If the snapshot has 8: clear `f2 & 0x88`; save `obj+0x5C` if 0; action 4, phase 0; zero +0x1E/+0x2A/+0x2C.
     - Then `f1 &= ~0x80018000`. If 0x800 is not yet set, set it, save `obj+0x5C`, and go to action 4.
2. **Freeze.** Same pattern:
   - Request mask 0x30080; zeroes +0x1E/+0x1C/+0x2C; clears `f1 & 0x80008000`; sets 0x10000; action 6.
   - On the first frozen tick, if `coll+0x58 == 0`, spawn the ice object (`sub_80E9BDC`). **Quirk:** if `coll+0x58 != 0` on that tick, execution jumps into the bubble-active branch (loc_800E836) and skips the bubble decrement.
3. **Bubble.**
   - Expiry also sets `coll+0x60 = 0`.
   - Request mask 0x20080; zeroes +0x1E/+0x1C/+0x2A; clears 0x8000; sets 0x80000000; action 7.
   - On the first tick, if `coll+0x60 == 0`, call `sub_80E4B34` (bubble object). Otherwise it jumps into the confusion-active branch.
4. **Confusion.**
   - Expiry clears 0x8000 and `f2 & 0x80`, and sets `coll+0x48 = 0`.
   - While active: on request, clear `f2 & 0x30088` and zero +0x1C/+0x2A/+0x2C. **Every active tick:** set 0x8000, clear `f1 & 0x80010800`, and if `coll+0x48 == 0` call `sub_80E09EE(r4 = 0)`.
5. **Immobilize.** Expiry clears 0x4000. While active, only on a request tick: clear `f2 & 0x40` and set 0x4000.
6. **Blind.** Expiry clears 0x2000 and sets `coll+0x4C = 0`. While active, only on a request tick: clear `f2 & 0x20`, set 0x2000, and if `coll+0x4C == 0` call `sub_80E09EE(r4 = 1)`.
7. **Invulnerable (+0x28).** While active set `f1 |= 8`; on expiry clear it.

The status objects (ice, bubble, confusion, blind) are T4/T1 spawns and so change pool allocation. Their specs are elsewhere.

**Status actions:**
- **Mashing:** in actions 4, 6 and 7, a nonzero `ai.JoypadPressed` (+0x24) subtracts one extra from the timer (`ldrsh`, signed). At ≤ 0 it stores 0 and clears the f1 flag. When that flag is gone the action exits the same tick: `ai+0x44 &= ~0x1000003F`, `f1 &= ~USING_ACTION`, anim 0, action 8.
- **Bubble** also sets `Z = byte_8017868[(coll+0x2C >> 2) & 31] << 16` (a bobbing table), resets Z to 0 on exit, and plays sound 0x124.
- None of this is exercised in the trace. It is code-only.

#### H6. Death and deletion — trace-verified (frames 938 and 2346)

Timeline, where t0 is the tick in which `applyDamageToPlayer` leaves HP == 0. This happens even when HP is drained to 0 through +0x8C, and also after the battle is over, because step 5 is not gated.

| Tick | What happens | Trace |
|---|---|---|
| t0 | Set `f2 \|= 1`. In the same `sub_801AF44`: DEAD is set and `setAttack0(2)`. The TAIL dispatches phase 0 (`sub_801741C`, 17499), which does: clear collision Region, `sub_801A5E2` (coll+0x48/+0x4C = 0), cancel the invis timer, `sub_8012EA8`, `sub_801DC36` (HUD), `ChipsHeld = 0`, `Chip = 0xFFFF`, re-flip, remove the FuturePanel reserve, `sub_801A7F4` (`ai+0x60 = 0`, Barrier = 0), `ai+0x58 = 0` if set, and **`sub_800A11C`** if `Param2 < 1`. `sub_800A11C` does `BattleState[0x12+alliance] -= 1` and removes self from `BattleState+0x80[0..7]`, so **the battle is over from here on**. Then `sub_802EF5C` and phase 4. | `[4,2,4,0]`, f1 \|= 0x100 |
| t0+1 | Phase 4 (`sub_801746E`, 17535), which waits while while dimmed: anim 2, `RelatedObject1Ptr = 0`, `PreventAnim = 0`, `ai+0x5C = ai+0x68 = 0`, sound 0x6C, **2× `SpawnT4BattleObjectWithId0`** (T4 kind 0) at (X, Y, Z) with params 3, `Timer = 21`, phase 8. | `[4,2,8,0]` t = 21 |
| t0+2 … t0+22 | Phase 8 (`sub_80174AA`): `Timer -= 1`; at 0 → phase 12 (Timer 0). | t0+22: `[4,2,12,0]` |
| t0+23 … t0+54 | Phase 12 (`sub_80174BE`): `Timer += 1`; mosaic = Timer>>1 and alpha (render only). At `Timer == 32`: clear mosaic/alpha, `sub_802CDD0`, **clear VISIBLE**, `sub_8011020` (per-NameID death hook; not analysed), then **`CurStateActionPhaseAndPhaseInitialized = 8`**. | t0+54: `[8,0,0,0]` flags 0x15 |
| t0+55 | CurState 8, `sub_8016C4E` (16585), runs once (PhaseInitialized): `sub_801BB78`, `object_freeCollisionData` (the traced status reads 0 from here), `ai+0x0E = 0xFF`, PhaseInitialized = 4. The object is freed only if `ai+0x02 != 0`; **for players it is 0, so the object stays allocated** in `[8,0,0,4]` until the end-of-battle cleanup frees every object (trace: flags → 0 at 1113 and 2521). | `[8,0,0,4]` |

Throughout action 2, `sub_80173F4` calls `sprite_forceWhitePalette` every tick. After t0, `f2 & 1` is set again every tick, because HP stays 0 and step 5 still runs. Because the DEAD check (step 7) comes first, this bit and any pending flinch bit are never consumed: the trace shows f2 = 1 in round 1 and 5 in round 2. Once DEAD, the rest of `sub_801AC6C` is skipped.

**Round outcomes in the trace.**
- Round 1: A1 was drained by A0's chip action 0x37 through `coll+0x8C = 4` per tick. That gives −4 HP per tick with no flinch or mercy, in bursts at 647–766, 788–907 and 929–938. HP hit 0 at 938.
- Round 2: the same drain from 2016 to 2277 left HP at 40. A0's chip 0x52 then hit for 60 (PanelDamage1 = 60, hm = 1, `FlagsFromCollision = 0x80000080`) at 2346, so HP = 0 and t0 = 2346. Deletion reached state 8 at 2400.

**Winner at battle end.** When the battle is over and the player is in action 8, `sub_80EA734` does the following:
- Calls `sub_8012EA8` and `sub_801A264` (20555). The latter clears all statuses: `f1 &= ~0x8001E800`, `f2 &= ~0x300E8`, and zeroes coll +0x1C/+0x1E/+0x20/+0x22/+0x2A/+0x2C.
- Calls `sub_801DACC(0x42)` (HUD only).
- If `sub_802DD2A()` (Cross form only): `AttackVars+0x03 = 1` and `setAttack0(0x4D)`. Otherwise `CurAnim = 0`.

#### H7. Camera shake and RNG1

The shake is not player code, but it consumes RNG1 (the port simulates it per console for that: `crate::console`, custom-screen.md §8). `camera_doShakeEffect_80301e8` is called every tick from `sub_802FFF4`, straight after `RunBattleObjectLogic` in `battle_8007A44`. There are two counters in `eCamera` (0x02009980):
- Primary: +0x0C timer, +0x0E type. Set by `camera_initShakeEffect_80302a8(type, dur)`; about 40 callers in asm31 chip/virus objects.
- Secondary: +0x10 timer, +0x12 type. Set by `sub_80302B6(type, dur)`.

Path selection each tick:
- If `sub_80269D0() != 0`, or `IsCurSubsystemInUse()` is false, or `battle_isTimeStopPauseOrBattleFlags0x20_800a0a4()` is nonzero: try the primary first, then the secondary. That last function returns 1 for dimming, *not paused*, or battle flags & 0x20. So in a running battle the primary counter is used first.
- Otherwise (paused, no dimming, no flag 0x20): secondary only.

For the chosen counter, if timer != 0: `timer -= 1`, then **2× `GetRNG1`**:
- `nextX = X + ((r1 & m) << 16) - b`
- `nextY = Y + ((r2 & m) << 16) - b`
- (m, b) per type: 0 → (1, 0x10000), 1 → (3, 0x20000), 2 → (7, 0x40000), 3 → (0xF, 0x80000).
- If `eCamera+0x4C` (camera flip) is set, `+0x48 = -2*dx`.

If neither counter is active: `+0x0E = 0` (the primary type is reset), `next = camera pos`, `+0x48 = 0`, and no RNG is used.

In the trace, both shakes happened **during the round-2 custom screen**. Neither came from damage:
- 1501–1540: 40 ticks, type 1, started by `sub_802774C` (custom-screen code via `sub_8009338`).
- 1802–1861: 60 ticks, type 2, started by the player's pause handler `sub_8017BC0 → sub_8014A38 → sub_8014D70`. This is taken when `ai+0x48 & 0x80`. A0 entered action 0x1C during the pause at 1794, which is likely a transformation.

Together they account for all 200 RNG1 camera calls: (40 + 60) × 2. In this match no hit started a shake. Chips that shake call the primary init.

### 12.8 Player destroy: CurState 8 = `sub_8016C4E`

The deletion action (§H6) ends by storing CurState 8 at t0+54. On the next update (t0+55) `sub_8016C4E` runs its body once (guarded by PhaseInitialized == 0): `sub_801BB78` (drop any panel reservation still marked by header flag 0x20), `object_freeCollisionData`, `sub_800A104`, AIData.Unk_0e = 0xFF, PhaseInitialized = 4. It frees the AIData and the object **only if AIData.Unk_02 != 0**. Players have Unk_02 = 0, so the dead player stays linked in the update list, keeps its T1 slot and AIData slot, and does nothing but `sub_801BCF4` every tick (state [8,0,0,4]) until the end-of-round `FreeAllObjectsOfSpecifiedTypes(0x1A)` (frames 1113 and 2521).

**Dangling collision pointer.** `object_freeCollisionData` releases the CollisionData slot but obj+0x54 keeps pointing at it. In round 1 the winner's GunDelSol kept spawning a one-tick T3 collision-region object every tick; from frame 993 each of them allocated the freed slot 0 (lowest free) and zero-filled it, which is why the trace's "status" of the dead player reads 0 from frame 993 (it is really the T3 object's ObjectFlags1). Any code that reads the dead player's flags through `object_getFlag` (e.g. the other player's visibility check via `sub_80103BC`, §H4.2) reads whatever object currently owns that slot. Model the pools as raw memory with pointers/indices exactly like the game, or these aliasing effects will diverge.

### 12.9 Beast Out: the form change (action 0x1C) and what a form changes

Trace: round 2, alliance 0 (MegaMan, Falzar) requests form 0x0C. The transformation sequencer (battle-flow.md §3.4.1) sets request 0x4000 at 1794; the pause handler (§M4.2) starts action 0x1C with state bit 0x80 the same tick. From 1795, `sub_8014A38` runs each paused tick:

1. Read the side's requested form with `sub_801595E(alliance)` (the turn's copy of the transform records). Outside 1..0x18: clear state 0x80 and stop. 0x17/0x18: Beast Over table `off_8014AF0`; 0x0D..0x16: Cross Beast, `off_8014ADC` (or `off_8014B04` when the current form is past 0x0A); 0x0B/0x0C: Beast Out, `off_8014AC8`; 1..10: Cross, `off_8014AB4`. The table is indexed by AIAttackVars+0 (the step; its `strh` writes also clear the step's init byte +1).
2. Then, unless state bit 0x80000 is set, `sub_801BCD0`: load a changed animation and step the sprite even though the battle is paused.

Beast Out steps (timer = AIAttackVars+0x10, a u16):

| Step | Routine | Ticks | What happens |
|---|---|---|---|
| 0 | `sub_8014D08` | 1795 | FuturePanel → panel, drop the reservation, snap coordinates and collision panels; end invulnerability (`sub_800EB08`); face the default way under the standard column patterns (`sub_800F46C`); drop the charge (`sub_8012EA8`); end the Full Synchro aura (`sub_80C4C3A` on AIData+0x5C; with none it writes into BIOS space, a no-op); RelatedObject1Ptr = 0, AIData+0x68 = 0; `sub_80158FA`; CurAnim = 0x11 (not `object_setAnimation`); if there is a form overlay, its Param3 = 1 and flags \|= 0x14; timer = 6; step 4. |
| 4 | `sub_8014D70` | 1796–1856 | Count 6 down; on the tick the timer was 0 (1802): state \|= 0x80000 (no more sprite steps), T4#0 effect 0x2E (flip = alliance) at the navi's position with Timer 0x36 and flag 0x04, then the navi moves down by 0xC00000 (off the field), `byte_203EAE0[alliance][2] = 1`, sounds, and a 60-tick camera shake (RNG1 only). Timer = 0x36, init = 4, and the same tick counts it to 0x35. Step 8 on the tick the timer was 0 (1856). |
| 8 | `sub_8014E08` | 1857–1866 | Init (1857): `sub_8011384(current form)` takes off the current form's overlay; load the new form's sprite (`sub_800FC9E(navi, form)`: category 0, index 0x0C), `object_setAnimation(0)` (CurAnimCopy = 0xFF), `sprite_setAnimation(0)`; coordinates from the panel (back on the field); timer = 10; NaviStats+0x2C = form; T4#0x0A palette flash (§A.7); NameID = 0x1AB + form (`sub_8015B22`: 0x1B7); `sub_8011268(form, 0)` puts on the new overlay (T1#0x57, spawned after T4#0x0A so it sits before it in the list); its Param3 = 0. Every tick the timer counts down; on the tick it was ≤ 1 (1866): note whether the emotion is Full Synchro, `sub_80144C0` (status reset: weapons, form flags, element; spawns T4#0x0F), `sub_80143A6` (calm down: mood 0x80), mood 0xFF again if it was Full Synchro, `sub_800EB08`; step 0xC. |
| 0xC | `sub_8014F04` | 1867–1887 | Timer = 0x14, counted down; on the tick it was 0: clear state 0x80, battle flag 0x20, state 0x80000 and requests 0x80008600, and `object_exitAttackState` (action 8, anim 0). The tail of `sub_8014A38` steps the sprite that tick. |

`sub_80158FA`: ObjectFlags1 &= ~0x80111C40 (bubbled, drag, frozen, sliding, paralyzed, flinching, moving), ObjectFlags2 &= ~0x10, SlideState = 0, AIData+0x48 &= ~0x200800, and the CollisionData paralysis/freeze/bubble timers (+0x1C/+0x2A/+0x2C) and ice/bubble links (+0x58/+0x60) zeroed.

**What form 0x0C changes** (mostly through `sub_80144C0`):
- Weapons (`sub_800FEEC` from `byte_8020354 + 6·form`): buster 3, A-charge 5, charged shot 0xFF (so B does not charge), B+Back none, alternative A-charge 0x1E for Null-family chips.
- Element Null and no weakness (`sub_801086C` tables).
- Form flags (`sub_8014536` → `sub_8014606`): ObjectFlags1 \|= AirShoe | FloatShoe, the collision self type becomes 0x10 (as with FloatShoe), and the lock-on marker T4#0x0F is spawned unless AIData+0x40 already holds one.
- A charges chips: `sub_801336C` asks `sub_8013236`, which in forms 0x0B..0x16 accepts any family-0xA (Null) chip. The other forms accept damaging, non-dimming chips of one family: form 2 Null, forms 3/0xF Sword (or chips 0x4C..0x4F), 7/0x13 Wood, 6/0x12 Aqua, 9/0x15 Break, 5/0x11 Fire. Link navis 5/6/7/0xB have their own rule (`sub_800F49E`, `byte_8021369`); MegaMan's always fails it.
- The beast-out counter (NaviStats+0x21) goes down at the next fighting state 0 (battle-flow.md §3.4).
- The overlay follows the navi: with AIIndex 0, every `sub_8011450` call and the flinch/drag hook restart the overlay's animation (`sub_80C44D2`).

### 12.10 Crosses: the form change (action 0x1C), the merging image (T1#0x1B) and the body overlay (T1#0x56)

Trace: soundmod, every round's first turn. Both navis cross at once: side 0 (Gregar) into form 2, side 1 (Falzar) into form 0xA. Round 1: the navis enter action 0x1C at 3023, the change is done at 3110 (action 8). Forms 1..10 use `off_8014AB4`; the step and its init byte are AIAttackVars+0/+1, the timer +0x10 (u16), as for Beast Out.

| Step | Routine | Round-1 ticks | What happens |
|---|---|---|---|
| 0 | `sub_8014B18` | 3024 | AIData+0x48 \|= 0x80000 **first** (the sprite holds still from now on: no `sub_801BCD0` in `sub_8014A38`'s tail), then as Beast Out's step 0 (FuturePanel → panel, drop the reservation, coordinates, collision panels; `sub_800EB08`; face the default way under the standard patterns; `sub_800F2C6` (sprite flip, HUD); `sub_8012EA8`; the Full Synchro aura; `sub_80158FA`), but CurAnim, RelatedObject1Ptr and AIData+0x68 are left alone. Timer = 6. Only when the current form is 9 and CurAnim is 0x16: CurAnim = 0, both pointers cleared, 0x80000 cleared, the overlay's Param3 = 1 and flags \|= 0x14. Step 4. |
| 4 | `sub_8014B98` | 3025–3079 | Count 6 down; on the tick it was 0 (3031) spawn the merging image (`sub_80BC844(panelX, panelY, 0x14)` with r4 = the requested form: T1#0x1B, Param1 = form), timer = 0x30, AIAttackVars+0x30 = 0, init = 4. Every tick after: +0x30 counts up to 6 (MegaMan flashes white while it does), the timer counts down; on the tick it was 0 (3079): timer = 6, +0x30 = 0, step 8. |
| 8 | `sub_8014BEE` | 3080–3089 | Init (3080): RelatedObject1Ptr = AIData+0x68 = 0; `sub_8011384(current form)`; the sprite for (navi, new form): forms 1..10 are MegaMan's base sprite (`byte_800FCBC`); `object_setAnimation(0)`, `sprite_setAnimation`; coordinates from the panel; timer = 10; NaviStats+0x2C = form; `sub_8015B22` (NameID 0x1AB + form); `sub_8011268(form, 0)`: the Cross's body overlay (T1#0x56, below); HUD; sounds 0x8D and 0x77. Every tick the timer counts down; on the tick it reaches 0 (3089): `sub_80144C0` (status reset: the form's weapons, flags, element), `sub_80143A6`, mood 0x80 (`sub_8015BEC`), `sub_800EB08`; step 0xC. |
| 0xC | `sub_8014CC0` | 3090–3110 | Timer = 0x14, counted down; on the tick it was 0: `byte_203EAE0[alliance][0xB] = 1` (read only after the battle, for the busting level), clear AIData+0x48 0x80 and 0x80000, battle flag 0x20, requests 0x80008600, `object_exitAttackState`. |

**T1#0x1B, the Cross navi's image (`sub_80BC650`).** Spawned by `sub_80BC844`: PanelX/Y = MegaMan's, +0x62 (halfword, the swings) = r3 = 0x14, alliance and flip copied, RelatedObject1Ptr = MegaMan, Timer = 6, flags \|= 0x14 (it runs while paused). Its spawn position is register garbage, overwritten by its init.
- Init (`sub_80BC670`, the spawn tick): NameID = 0x1A0 + Param1; the sprite of navi Param1 (`sub_800FC9E(Param1, 0)`: category 8), animation 0, VISIBLE; +0x68 = −1 (the side of the next swing); ExtraVars+4 = 0x280000 / swings (`svc 6`), X = panel X + ExtraVars+4 · swings · front (40 pixels in front); ExtraVars+0xC = (4 − PanelY) · 0x180000, added to Y and set as Z (it floats above the panel but sorts with it); ExtraVars+0x10 = `byte_80BC758[Param1]` (extra height: navi 1 +8, 3 +4, 8 −8 pixels), added to Z; the navi's init hook (`sub_8010DD0` by NameID: nothing for navis 2 and 10); ExtraVars+0x14 = 0.
- Update (`sub_80BC78C`): action 0 counts the timer down (flashing white); at 0: timer = 10, action 1. Action 1 counts the timer down; at 0: the first time, sound 0x8C; swings −1; if none are left, the NameID's death hook (`sub_8011020`: nothing for this object), a T4#0 effect 3 at the panel's coordinates with Z = 16 and flags \|= 4, and `object_freeMemory` (freed at once, so the effect runs its init the same tick). Otherwise timer = 2, X = panel X + (+0x68) · ExtraVars+4 · swings · front, Y = panel Y + ExtraVars+0xC, Z = ExtraVars+0xC + ExtraVars+0x10, +0x68 negated. It swings every other tick, 2 pixels narrower each time. Every tick ends with `object_updateSprite` (nothing while paused).
- Round 1: spawned at 3031, action 1 at 3037, first swing 3046 (X −138 for side 0), gone at 3084 (effect alive 3084–3106, freed 3107).

**T1#0x56, a body overlay (`sub_80C4348`).** A second sprite on its owner (RelatedObject1Ptr), spawned by `sub_80C44A8` (flags \|= 0x14, alliance and flip copied) with r4 = params: Param1 the variant, Param2 nonzero = its own palette (else its owner's), Param3 how it steps, Param4 an animation offset. The Crosses use `sub_80112E0`..`sub_801133A` with Param2 = 1: forms 1..10 get variants 4, 8, 0xA, 0xC, 0x11, 5, 0xE, 9, 0xD, 0x12.
- Init (`sub_80C4368`): stores `off_80C42D4[Param1]` (a per-animation byte table) **in its CollisionDataPtr slot**, so a trace reading ObjectFlags1 through it reads 0 (it points into ROM); the sprite `byte_80C4320[Param1]`; CurAnim = Param4 and CurAnimCopy = 0 (one halfword store); state 4, then the update.
- Update (`sub_80C43C4`): CurAnim = owner's + Param4; position = owner's; then if ExtraVars[0] (`sub_80C4526`): Y and Z + 1 pixel; else if the table's byte for the owner's animation is 0: Y and Z − 1 pixel (same place on screen, drawn behind the owner). Visibility follows the owner unless PhaseInitialized is set (`sub_80C44E4`/`sub_80C44FA` force it); the flip follows the owner. Action 0: Param3 = 0 → `object_updateSprite` unless while dimmed; else `sub_801BCD0`.
- Removal (`sub_80C44C8`): state 8, freed at its next update.
- The tables: `ObjectData::body_overlays` (the content's `rules/body-overlays.luau`; the byte tables run back to back up to the pointer table; an animation past a table's end reads the next one, so each row is extracted to the block's end).

## 13. RNG uses (all that touch objects or battle setup)

RNG1 state is the u32 at 0x02001120, RNG2 at 0x020013F0 (the trace's `rng1`/`rng2`). The generator functions themselves (`GetRNG1`, `GetRNG2`, `GetPositiveSigned*`) are specified in `battle-flow.md` §5. A write watch on both states over the whole reference match gave exactly this:

| Frames | Caller chain | Calls |
|---|---|---|
| 3, 1155 | `sub_800B144` -> `sub_81209DC` (battle init) | 2x `GetPositiveSignedRNG1` + 2x `GetPositiveSignedRNG2` each |
| 69, 1221 | `battle_copyStructsIncludingBattleStats_800b2d8` (link sync) | RNG2 state overwritten (not a call) |
| 70, 1222 | `sub_80079F0` -> `sub_800A3E4` -> `sub_800A570` -> `sub_8000D12` (panel/field set-up) | 60x `GetPositiveSignedRNG1` each |
| 72, 1224 | player spawn list -> `sub_80AA88C` (§12.1) | 1x `GetPositiveSignedRNG2` each |
| 1501-1540 | camera shake, type 1 (started by custom-screen code `sub_802774C`) | 2x `GetRNG1` per tick |
| 1802-1861 | camera shake, type 2 (started by the player's pause handler `sub_8017BC0` -> `sub_8014A38` -> `sub_8014D70`, Beast Out) | 2x `GetRNG1` per tick |

No object consumed RNG in this match otherwise, because nobody fired the buster and no hit had a hit effect. The complete list of object-level RNG sites found by the code reading (all consume in object update order, i.e. the list order of §3.3):

| Site | Gen. | Calls | Condition | Section |
|---|---|---|---|---|
| `sub_8013D5E` (buster request set-up in `sub_8011A26`) | RNG2 | 1 | **every buster request** (MegaMan's buster routine), even with NaviCust stats 0 | §B6 |
| `sub_8011A26` | RNG2 (pos.) | 1 | navi stat 0x4D != 0 | §B6 |
| `sub_8011A7E` (charged shot) | RNG2 (pos.) | 1 | navi stat 0x4F not in {0, 6} | §B7 |
| `object_spawnCollisionEffect` / `sub_801A100` -> `AddRandomVarianceToTwoCoords` | RNG2 | 1 per hit | attacker side (projectiles, T3 regions): `FlagsFromCollision & 0x3F800000`, not bit 0, HitEffect != 0xFF | §B8 |
| `object_spawnHiteffect` (in `sub_801AC6C`) | RNG2 | 1 | victim side: `FlagsFromCollision & 0x20000`, not paused | §H2 |
| `sub_8017AB4` dimming shake | RNG2 | 1 per tick | 30 ticks after a damaging tick while dimmed | §H4.4 |
| camera shake `camera_doShakeEffect_80301e8` | RNG1 | 2 per tick | while a shake counter (eCamera+0x0C / +0x10) runs; ~40 chip/virus objects start shakes via `camera_initShakeEffect_80302a8` | §H7 |
| `sub_8013E58` (player init) | RNG2 | 1 | navi stat 0x1A is 9 or 10 | §12.2 |
| `sub_8013DA0` (player tick) | RNG2 (pos.) | 1 per 60 ticks | navi stats 0x24 and 0x21 both nonzero | §M1 |
| `sub_8013CC4` (move commit) | RNG2 (pos.) | 1 | navi stat 0x13 != 0 | §M6.6 |
| `sub_8013FAE` (move end) | RNG2 | 1 | navi stat 0x11 != 0 | §M6.6 |
| `sub_8007424` (entity list kind 2) | RNG2 (pos.) | | random-virus entries; not in PvP lists | §12.1 |

Auxiliary objects T1#5, T4#0/2/8/0x48 and the round-2 kinds never call the RNG (§A.0).

## 14. Trace verification summary

Method: the original under emulation with write watches (every write with its call chain, every tick), post-tick memory dumps, injected key presses (they reach the input record 5 ticks later) and memory pokes. Results that are not already in the sections:

### 14.1 Object system checks (whole match)

Method: per-tick dumps of the T1/T4 pools (first 8 slots, including list nodes), both allocation bitfields and the list head/sentinel, then walking the list exactly as `RunBattleObjectLogic` does. Every spawn and free in the match is consistent with §3 (lowest free slot; insertion immediately after the updating object, or at the tail when spawned outside the loop):

| Frame | Event (T=type, s=slot, #=Index) | Update order after the tick |
|---|---|---|
| 72 | spawn T4 s0 #2 (outside loop, tail), T1 s0 #0 (alliance 1), T1 s1 #0 (alliance 0) (outside loop, tail); during their init each player spawns T4#8 (s1, s2) right after itself | T4#2, P1(a1), T4#8, P0(a0), T4#8 |
| 122 | T4#2 frees itself | P(a1), T4#8, P(a0), T4#8 |
| 639 | P(a0) (chip GunDelSol) spawns T1 s2 #5 | ..., P(a0), T1#5, T4#8 |
| 645 | P(a0) spawns T4 s0 #0x48 (lowest free T4 slot) | ..., P(a0), T4#48, T1#5, T4#8 (later spawn ahead of earlier) |
| 766 / 778 | T4#48 freed / T1#5 freed | |
| 780-1060 | two more GunDelSol uses reuse T1 s2 and T4 s0 | |
| 939 | P(a1) deletion phase 1 spawns 2x T4#0 into s3, s4. s1 is still allocated: its T4#8 was told to die at 938 (CurState 8) and only frees itself when its own update runs, later in tick 939 | P(a1), T4#0(s4), T4#0(s3), P(a0), ... |
| 962 | both explosions freed (lifetime 22 = animation length) | |
| 1113 | `FreeAllObjectsOfSpecifiedTypes(0x1A)` empties everything | (empty) |
| 1224 | round 2 starts; identical spawn pattern to frame 72 | T4#2, P(a1), T4#8, P(a0), T4#8 |
| 1802-2372 | round 2 uses other chips (T1#0x57, T4#0x0A/0x0F/0x28, T4#0 effects); all allocations are lowest-free-slot, all insertions directly after the spawner | e.g. 2342: ..., P(a0), T4#28(s4), T1#57(s3), T4#28(s3), T1#57(s4), ... |

Other confirmed facts:
- A freshly spawned object has already run its CurState-0 init by the end of its spawn tick (e.g. T1#5 at frame 639 shows state [4,0,0,0]).
- `GameState.BattlePaused` = 1 from frame 72 to 592 (intro, custom screen, "battle start"); battle flags bit 0 set at 593 (round 2: 1224-1969 paused, bit 0 from 1970). During the pause only flag-0x04 objects run (players, T4#2, T4#8, T1#5).
- The player's header flags go 0x19 (spawn) -> 0x1D (`sub_800753C`) -> 0x15 (`sprite_load` in init, frame 72) -> 0x17 (VISIBLE from action 0, frame 73) -> 0x37 after its first `object_reservePanel` (frame 623 for alliance 0).
- The complete player/AIData/CollisionData contents after init are given in §12.2 (dump of frame 72).

### 14.2 Actor behaviour checks

| Behaviour | Evidence | Section |
|---|---|---|
| Entry sequence: local player visible at 73, remote fades 89-121, both action 1 at 122, action 8 at 593 | machgun trace | §M4 |
| Move timing F..F+13 (panel change at F+3, action 8 at F+12) | frames 623-636 and 1982-1995 | §M6.6 |
| Chip (GunDelSol, action 0x37) starts on the A-press tick; hand cursor advances the same tick | frames 638, 779, 920 | §B9 |
| GunDelSol region hits: `cd+0x8C` = 4 per tick, applied by the victim one tick later or the same tick depending on list order | frames 646-938 | §H1, §7.3 |
| Buster: action 0x11 on the B-release tick, shot at R+2, 1 damage, recovery N from free panels ahead, RNG2 call per request | injected runs 1, 3, 4 | §B6, §B8 |
| Charged shot: 100-tick charge, 10 damage, fire at R+6 | injected run 2 | §B7 |
| Flinch 24 ticks, mercy 120 ticks, VISIBLE 2 off / 2 on | poked run (HP restored at 2340) | §H4.2 |
| Deletion t0..t0+55, player object never freed | frames 938-993, 2346-2401 | §H6, §12.8 |
| Spawn/free slot and list order of every object | per-tick pool dumps, whole match | §14.1 |

## 15. Open questions and uncertainties (consolidated)

Object system:
1. The pathological list cases of §3.4 (free-self-then-spawn into a lower slot, free-self-then-free-successor) are derived from code; not seen in any trace. Replicating the list pointer semantics exactly covers them.
2. `object_presentCollisionData` stores the caller's r1 into cd+0x54 before clearing it (except while dimmed, when the garbage survives). Only matters if +0x54 is read while dimmed. For the player the value is the GameState pointer (r1 left by `battle_isPaused`).
3. The dangling CollisionDataPtr of a deleted player (§12.8) and freed objects in general: any reader aliases the slot's new owner.
4. Register-garbage spawn arguments (T4#8, T4#2, T4#0x0A, the second death explosion): values documented only for the observed configuration; they depend on the local side (`BattleState[0x0D]`), battle effects and panel pattern. They are visual (overwritten or never read) but are part of the traced object state.
5. `sub_8013892`'s `pop {r4}` clobbers the AIData pointer in `sub_80172F0` when navi stat 0x06 (FirstBarrier) is nonzero, so the T4#8 pointer would be stored through a bogus address. Code-derived, not observed.
6. View-dependent state: the two consoles differ in action-0 intermediate states, the VISIBLE header bit (blind check), HUD tables and some garbage values, all keyed on `BattleState[0x0D]`. A deterministic engine should keep a "local side" parameter to reproduce a given console's memory, but none of these affect gameplay outcomes as far as analysed.

Player:
7. Alliance-1 movement direction (Right = world -x) and all blocked-move, confusion, ice/road-slide, knockback/drag and status-action (paralysis/freeze/bubble) behaviour are code-derived only (§M6, §H4.3, §H5).
8. Occupancy bits 0x80000/0x800000/0x1000000/0x2000000 in panel flags: which object kinds set them is for the collision spec (§M6.4).
9. HUD calls (`sub_801DA48`, `sub_801DACC`, `sub_801DC7C`, `sub_801EB18`, `sub_801E270`) are assumed to have no simulation effect; not fully audited.
10. Who sets request flags 0x20, 0x600/0x8600 (reactive defensive chips), state flag 0x200 and the SELECT special (flags44 0x2000000) - form/Beast/chip features, unused in base PvP (§B5).
11. HitModifier bit semantics (1 flinch, 2 mercy, 0x04-0x20 push, 0x40 drag, 0x80 vertical) are inferred from the player-side consumers; the attacker side belongs to the chip/collision specs. MegaMan's charged shot has HitModifier 0 (no flinch).
12. Not analysed: `sub_8011020` (per-NameID death hook), `sub_802EF5C` (deletion link bookkeeping), `sub_801BB78` (reservation cleanup scan), `sub_800A104`. The pause-time handler `sub_8017BC0` is in §M4.2; of its actions only Beast Out is analysed (§12.9).
13. Dimming shake: X/Z may be left perturbed if dimmed ends while the 30-tick shake counter is still running (§H4.4).
14. `sub_800E730` status timers: on the first frozen/bubbled tick with a stale effect-object pointer, execution jumps into the next status's active branch (§H5). Code-derived.
15. Animation: the claim that MegaMan's own actions never depend on animation data rests on this match's coverage; 12 player chip/form routines do end on end-of-animation (§S.4). Compressed sprites missing from the cache fall back to a one-dot sprite (would change animation lengths) - assumed never to happen in battle (§S.4).
16. Screen fades: the T4#2 intro fade and the transformation fade-in (type 0x40) take 17 ticks; the transformation fade-out (0x44) and the end-of-round fade (0xC) take 16 (§A.4, battle-flow.md §3.4.1). Other fade types are unmeasured.
17. Round 2 of the reference match contains a Beast Out (§12.9) and the Beast lock-on marker T4#0x0F, which *does* affect chip targeting (§A.7). Crosses, Cross Beast, Beast Over, form reversion and the Beast chip wrapper are not specified here.
