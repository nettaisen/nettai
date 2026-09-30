# Battle flow and tick structure (BN6 US Falzar, link PvP)

This spec covers the battle's control flow for a cable-link PvP netbattle:
- the frame/tick anatomy;
- the state machines (init, intro, banner, custom screen, fighting, pause, result, judge, end, round chaining);
- the exact per-tick call order;
- inputs and the link packet;
- the custom gauge and chip-hand handoff;
- RNG usage;
- the battle-wide state an engine must carry.

Object behaviour, panels, damage and chips have their own specs; this document only fixes *when* those systems run and what glue state connects them.

It is written so that a clean Rust reimplementation can be built from it without the assembly. Every claim carries a ROM address or function name from the bn6f disassembly. Anything not verified is marked **(uncertain)** and collected in §11.

**Conventions**

- `BS` = BattleState at **0x02034880** (`Toolkit+0x18`). "BS+0x12" means the byte at 0x02034892.
- State bytes are jump-table offsets, so they are multiples of 4. Word stores of a parent state also zero its sub-state bytes.
- `effects` = `GetBattleEffects()` = BattleSettings+8. In the machgun PvP match it is 0xE8C.
- A **frame** is one call of `battle_8007800` (harness frame numbers of the machgun trace, core 0). A **tick** is one execution of the simulation block of `battle_8007A44` (§4); BS+0x64 counts ticks.
- Trace states are post-frame values. In a timeline, the last frame of a phase is the frame whose handler made the transition.
- Other key globals:
  - GameState at 0x02001B80 (`Toolkit+0x3C`); its pause byte `BattlePaused` is at **0x02001B8A**.
  - Joypad 0x0200A270, Camera 0x02009980, Toolkit 0x020093B0.
- Sources used:
  - static reading of the original's code;
  - the original running under emulation with per-frame memory dumps and write backtraces, on the machgun battle (Falzar vs Falzar, match type 1, 2 rounds, both won by P0 by KO);
  - the machgun trace, and a record of which routines ran in it.

---

## 1. Frame anatomy

One GBA frame of a netbattle runs in this order:

1. **Main loop** (`main_` 0x080002BC, `main_gameRoutine`): vblank wait and render-side copies, then `*CurFramePtr += 1`, then the subsystem jump table. For a netbattle this is `SubMenuControl` → comm applet → comm-menu battle state **`sub_812B5C8`**.
2. **`sub_812B5C8`**:
   1. `eStruct203F7D8[1] = sub_803EAE4()` is the link tick.
      - Cable PvP has `eStruct200BC30[0] == 0`, so this goes `sub_803EB04` → `sub_803DEB4` (SIO multiplay library).
      - On status 2 it continues `sub_803ED1C` → `sub_803EE98`. That de-interleaves the received packets into rx slot 0 (`unk_20399F0` 0x020399F0, player 0 = SIO master) and rx slot 1 (`unk_2039A00` 0x02039A00, player 1), and stamps tx header bytes.
      - A headless engine replaces this step with "this tick's two packets".
   2. `JumpTable812B5F4[[r5+2]]`. During a battle this is `sub_812B698`, which calls **`battle_8007800`** (return site 0x0812B6AC). It leaves the battle when the return value r0 becomes 0.
3. **`battle_8007800`** (0x08007800):
   1. `sub_801FE6C` handles link status bookkeeping.
      - It saves the status written in step 2.1 and sets it to 2.
      - In a link battle (`eStruct203F7D8[0]` set, EVENT_172D clear, effects&8) it restores the saved value.
      - Without a link it copies tx into both rx slots.
   2. `sub_8020140`: terminate latch; only acts when `eStruct200BC30[0] == 0xC` (never in cable PvP).
   3. `JumpTable8007838[BS[0]]`: the top state machine (§2.1).
   4. `BS+0x60 += 1`: u32 count of battle frames. It counts every call, including init, end and stalled frames.
   5. Returns:
      - r0 = BS+0x0A (1 while the battle/set is running);
      - r1/r2 = low/high nibble of BS+0x1F (result code);
      - r3 = effects.
4. Back in the main loop, **`bl GetRNG1` at 0x0800031C** advances RNG1 exactly once per frame, then chatbox, BG scroll, GFX animations and the OAM copy.

In the trace, every top-state-4 frame is a tick (the link status is 2 on all of them). Within a round, tick = frame − 71 (round 1) and frame − 1223 (round 2).

---

## 2. State machines at a glance

### 2.1 Top state: BS+0x00 (`JumpTable8007838`)

| Value | Handler | Meaning |
|---|---|---|
| 0x0 | `sub_8007850` | init: link handshake, synchronized setup, folder, field (§3.1) |
| 0x4 | `battle_8007A44` | running: one simulation tick per frame (§4) |
| 0x8 | `sub_8007B80` | end: objects run 11 more frames, link teardown, then exit or chain the next round (§3.7) |
| 0xC | `sub_8007E62` | communication error |
| 0x10 | `sub_8007F4E` | terminate (link lost or host abort) |

### 2.2 Battle-mode handler: BS+0x01

The handler is chosen by `off_8007B50[GetBattleMode()]`, where battle mode = BattleSettings+3. PvP uses mode 0 → `sub_8009158` (0x08009158).

| BS[1] | Handler | Phase | Used in PvP |
|---|---|---|---|
| 0x00 | `sub_80091F0` | intro | yes |
| 0x04 | `sub_80092A0` | round banner | yes |
| 0x08 | `sub_8009338` | custom screen | yes |
| 0x0C | `sub_800938A` | fighting (runs the fighting machine) | yes |
| 0x10 | `sub_800945C` | end exchange and results (effects&2 only) | no |
| 0x14 | `sub_80094DA` | fade out → top 8 | yes |
| 0x18 / 0x1C / 0x20 | `sub_800951E` / `sub_8009552` / `sub_8009594` | tutorial text (modes 4/5/8) | no |
| 0x24 | `sub_80095C8` | error text box | no |

After the state handler, if effects&8, `sub_8009158` switches BGM when the local navi's HP ≤ MaxHP/4. The latch is BS+0x20, set to 1 and back to 0. This is sound only, but it writes BattleState.

The latch is not carried between rounds: init (top state 0, `sub_8007850` → `loc_8007918`) counts the frames it waits for the link in the same halfword (0xB4 of them is a link error), after the battle start zeroed BattleState. A round whose init waited therefore starts with the latch set: its first tick (when the navi has no HP yet, so HP ≤ MaxHP/4) plays no pinch switch, and the second tick switches it off. Machgun: round 1's init did not wait (BS+0x20 = 0, pinch on at 72 and off at 73); round 2's init waited 6 frames (1149–1154), so only the switch-off at 1225 plays. The engine takes it as `RoundSetup::low_hp_music_latched`.

### 2.3 Fighting machine: `dword_203CA70` (0x0203CA70)

`sub_800801C` runs `off_8008038[[0]]`, then `sub_802DE5C` (a no-op unless battle flag 0x40 is set), and returns byte [4].

Layout of the 0xC bytes (all zeroed by `sub_80084C0` on every entry into mode 0xC):

| Byte(s) | Meaning |
|---|---|
| [0] | state |
| [2] | sub-state |
| [3] | init flag |
| [4] | result: 0 none, 1 local win, 2 local loss, 3 draw, 6 open custom |
| [5] | pausing player |
| s16 [8] | timer |
| u16 [0xA] | turn timer |

The next byte, 0x0203CA7C, is *not* part of this machine: it is the live T3-object count, rebuilt every tick by `RunBattleObjectLogic` (§8.4).

| [0] | Handler | Meaning |
|---|---|---|
| 0x00 | `sub_800840C` | post-custom setup (transformations) |
| 0x04 | `sub_8008064` | "battle start/turn" banner; inputs start flowing to actors |
| 0x08 | `sub_80080D2` | fighting |
| 0x0C | `sub_80081A4` | local win |
| 0x10 | `sub_800825A` | local loss |
| 0x14 | `sub_80082DC` | draw |
| 0x18 | `sub_800834A` | time-up damage judge |
| 0x1C | `sub_80083E4` | START pause |
| 0x20 | `sub_8008452` | custom requested: revert transformations |
| 0x24 | `sub_8008492` | custom requested: sequencer, then [4] = 6 |

### 2.4 Round timeline (machgun)

Each row gives the first and last frame of the phase. The last frame is the one that performed the exit transition.

| Phase | Round 1 | Round 2 | Ticks |
|---|---|---|---|
| top 0, [1]=0 `sub_800794C` | 1 | 1147 | — |
| top 0, [2]=0 `sub_800799C` (link on) | 2 | 1148 (link stalls 1149–1154) | — |
| top 0, [2]=4 `sub_80079A8` (side, settings, start init exchange) | 3 | 1155 | — |
| top 0, [2]=8 `sub_80079D0` (wait for init exchange) | 4–69 | 1156–1221 | — |
| top 0, [2]=0xC `sub_80079F0` (panels, folder) | 70 | 1222 | — |
| top 0, [1]=8 `sub_8007A0C` → top 4 | 71 | 1223 | — |
| mode 0 intro | 72–123 | 1224–1275 | 52 |
| mode 4 banner: delay / banner / delay | 124–135 / 136–195 / 196–207 | 1276–1287 / 1288–1347 / 1348–1359 | 12 / 60 / 12 |
| mode 8 custom screen | 208–528 | 1360–1775 | 321 / 416 (player-driven) |
| fighting machine 0x0 | 529–532 | 1776–1909 | 4 / 134 (round 2: P0 transformation) |
| fighting machine 0x4 (start banner) | 533–592 | 1910–1969 | 60 |
| fighting machine 0x8 (first unpaused tick 593 / 1970) | 593–994 | 1970–2402 | 402 / 433 |
| … P1 HP → 0 / removal finished | 938 / 993 | 2346 / 2401 | |
| fighting machine 0xC (win) | 995–1096 | 2403–2504 | 102 |
| mode 0x14 fade | 1097–1113 | 2505–2521 | 17 |
| top 8, [1]=0 | 1114–1145 | 2522–2553 | (frames) |
| top 8, [1]=4 `sub_8007CA0` | 1146 → round 2 | 2554 → set over (2–0) | |

Pause byte (0x02001B8A): 1 on frames 72–592 and 1224–1969, otherwise 0. Battle flags (BS+0x32): 0 until 593 / 1970, then 1.

---

## 3. Lifecycle in detail

### 3.1 Battle start and init (top state 0)

**Battle start routine** is `loc_8007204` / `loc_8007208`. It is called by `sub_80071D4` from the comm menu for round 1 and by `sub_8007CA0` for later rounds. Tango's `round_start_ret` (0x08007304) lies inside it. It does the following:

- Zero BattleState (0xF0 bytes), set BS+0x3C = settings pointer, BS+0x0A = 1, BS+0x1B = 1.
- Zero the input records `dword_2036820` (0x10 bytes).
- `sub_801FE00`: clear `eStruct203F7D8`, reset the tx header, and zero the block-transfer buffers `dword_203F4A0` (0x200 bytes: both receive areas) and `dword_203CBE0` (0x100 bytes: send queue).
- `sub_8013422(0/1)`: default stats into both navi-stat blocks.
- `sub_800A2F8`: BS+0x06 = settings+0, BS+0x0F = settings+3.
- `sub_800A954`: both chip blocks become 0x50 zero bytes with +2..+0xD = 0xFF.
- `sub_800A79C`: BS+0x5C = 0x0C.
- `UnpauseBattle`.
- `sub_802DFFC`: zero `unk_2036120` (0x3A0 bytes, the flag-0x40 per-player structs).
- `sub_802D08C(0/1)`, and other presentation resets.

For round 1 only, `sub_80071D4` first zero-fills 0x02033000–0x02039AA0, 0x02039ADC–0x0203CCE0 and 0x0203CDA8–0x02040000.

**`sub_8007850`** (0x08007850) runs every init frame:

1. `s = eStruct203F7D8[1]` (link status).
2. If `s == 4`: `dword_200F3B0 = 0xFF`, `setTwoStructs_800A840(5 or 9)`, top 8 / [1]=4.
3. If `s == 2`:
   - If `eStruct2038160[1] ≠ 0`, terminate with code 9/0xA and go to top 8.
   - Otherwise `sub_801FEEE(s)`, then dispatch on [1]. **Packets, including input records, are applied during init too.**
4. Any other status is a stall frame. Nothing advances and `BS+0x20 += 1` (cumulative); when it reaches 0xB4 (180), `dword_200F3B0 = 0xEE` and the error path runs. Round 2 has six stall frames, 1149–1154 (status 1,1,1,1,8,8).

| [1] / [2] | Handler | Work | Exit | Frames r1 / r2 |
|---|---|---|---|---|
| 0 | `sub_800794C` | See below. | [1]=4 | 1 / 1147 |
| 4 / 0 | `sub_800799C` → `sub_801FE24` | `sub_803DCE8(0x10,0,2,0)`; `eStruct203F7D8[0] = 1` (link session on) | [2]=4 | 2 / 1148 |
| 4 / 4 | `sub_80079A8` | BS+0x0D = `sub_803DD60()`: 0 on the SIO master (P0), 1 on the slave, from `dword_2009A18 & 3`; also copied to `eCamera+0x4C`. Then `sub_800B144` builds and starts the init exchange. | [2]=8 | 3 / 1155 |
| 4 / 8 | `sub_80079D0` | Waits until `sub_800B46C(0x12345678)`; then `sub_800B460` and `battle_copyStructsIncludingBattleStats_800b2d8`. | [2]=0xC | 4–69 / 1156–1221 |
| 4 / 0xC | `sub_80079F0` | `sub_800BF88(BS+6, settings+6)` (panel types and column alliances); `sub_800A3E4` (folder, §3.1.2) | [1]=8, [2..3]=0 | 70 / 1222 |
| 8 | `sub_8007A0C` | See below. | top = 4 | 71 / 1223 |

Work done by `sub_800794C`:
- `sub_800318C`: empty object list.
- `InitializeT1/T3/T4BattleObjectStructs`.
- `sub_802E112`: sets battle flag 0x40 only for non-link battles or when EVENT_1722 is set. **Not set in PvP.**
- `sub_8007338`: camera.
- `sub_800A0C6`: zero the input records.
- `sub_801BE70`: zero the HUD struct `eStruct2035280` (0x60 bytes), then +0x22 = 0x20 (gauge rate), +0x26 = 0xFFFF; also zero `dword_20352E0`, `byte_203EB50`, `dword_203CA48` and `dword_20367E0` (mega/giga use counters).
- `sub_8002668`.

Work done by `sub_8007A0C`:
- `sub_801986C`: collision pool (32 slots).
- `sub_800C8F0`, `sub_800318C`.
- **One `sub_800BFC4` panel update.** The battle is unpaused at this point, so `byte_203CB04` goes 0x8C → 0x8B.
- Sprite init.
- `battle_clearEnemyFadeinList`: zero `unk_2038170`.
- `*(u32*)BS = 4`.

The init wait is 62 transmit ticks plus the 4-tick link delay: index 0 (the magic) is sent at frame 65 and seen at 69.

#### 3.1.1 Init exchange

`sub_800B144` fills `dword_203CBE0` with 0x3E dwords and calls `sub_80200A4(0x3E)`. Both sides send their block, and both sides consume both blocks. The consumer is `battle_copyStructsIncludingBattleStats_800b2d8` (0x0800B2D8).

| Off | Sender's content | Consumer |
|---|---|---|
| 0x00 | magic 0x12345678 | — |
| 0x04 | sender's RNG2, sampled **before** the `sub_81209DC` draws below | **RNG2 := slot0+0x04** (P0's value) |
| 0x08 | BattleSettings pointer | unused |
| 0x0C–0x6F | sender's current-PET NaviStats (0x64 bytes, `sub_801401E`); +0x21 (BeastOutCounter) forced to 3 when effects&8 | slot0 → `eBattleNaviStats0` 0x0203CE00 (+ copies at 0x0203CB10, 0x02034A60, 0x0203C9E4); slot1 → `eBattleNaviStats1` 0x0203CE64 (+ 0x0203CB74, 0x02034AC4, 0x0203C980) |
| 0x70–0x97 | `unk_20018C0` (per-enemy time records) | → `byte_203EB00` / `byte_203EB28` (variable-damage formulas, `sub_8010AE4`) |
| 0x98–0xBF | `unk_2000260` | → `unk_2036790` / `unk_20367B8` |
| 0xC0 | `sub_8121198()` if EVENT_163, else 0xFF | → `dword_203CFA0[p]` |
| 0xC4 | protected BugFrag count | → `dword_203F7E0[p]` |
| 0xC8–0xCB | two `sub_81209DC(1)` results, each (settings index, background) | slot0 only → `byte_203CA50` (settings for rounds 2 and 3) |
| 0xCC–0xF3 | 0x28 bytes of event flags | `sub_80AAB88`: `unk_2036750[i] = ~(s0[i] \| s1[i])` |
| 0xF4 | `sub_8010D20()` (always 0) | → `dword_203CDF0[p]` |

`sub_81209DC(mode)` (0x081209DC) makes two draws, RNG1 first:
- `idx = PosRNG1() % {0x44,0x60,0x44}[mode]`, plus 0x60 if mode ≥ 2;
- `bg = byte_8120A20[PosRNG2() % 0x15]`, where `byte_8120A20 = {0,1,1,3,4,5,6,7,8,9,0xA,0xB,0xC,0xD,0xE,0xF,0x10,0x11,0x11,0x13,0x13}`.

With two calls, init frame 3 advances RNG1 by 3 (counting the main loop) and RNG2 by 2. At frame 69 RNG2 is overwritten with P0's pre-draw value (0xF047ADA2, the frame-1 value).

Result: both cores leave init with identical navi stats for both players, identical RNG2 and identical future settings. The local player's own stats also come back through the link. Wherever there is a choice, P0's (the master's) data wins.

#### 3.1.2 Folder (local, RNG1)

`eBattleFolder` at 0x0203CDB0 holds 30 halfwords, each `id = h & 0x1FF`, `code = h >> 9`. It is built by `sub_800A3E4` (0x0800A3E4). `sub_800A318` is dead code.

```
src = effects&0x200000 ? unk_2039AA0
    : dword_802137C[mode] ?: S_Chip_2002178 + 0x3C*navi[0x2D]     // mode 0: the player's folder
reg = tag1 = tag2 = 0xFF
if effects & 0x80:
    r = mode==1 ? 0xFF : navi[0x2E+fidx];               if r != 0xFF: reg = 2r; BS+0x17 = 1
    (t1,t2) = mode==1 ? (0xFF,0xFF) : (navi[0x56+2f], navi[0x57+2f])
    if t1 != 0xFF: tag1 = 2*t1; tag2 = 2*t2; BS+0x44 = 1
d = reg != 0xFF ? 2 : 0
for s in 0,2,..,0x3A:
    if s == reg: F[0] = src[s]
    elif s == tag1: F[28] = src[s]
    elif s == tag2: F[29] = src[s]
    else: F[d/2] = src[s]; d += 2
if src is not the player folder: sub_800A7EA(src)
if effects & 0x80: sub_800A570(F, BS+0x17, BS+0x44)
```

`sub_800A570` (0x0800A570) does the shuffle:

```
split F in order by ChipData(id).class (byte 7): class 2 (giga) → G[g], else N[n] (scratch 0x02033000)
if n: base=N; cnt=n; sw=n
      if reg: base=N+1; cnt-=1; sw-=1
      if tag: cnt-=2; sw-=2
      sub_8000D12(base, cnt, sw)
if g: sub_8000D12(G, g, g)
      for i in 0..g:
          idx = mode==1 ? 8 + PosRNG1()%11 : 10 + PosRNG1()%(n-12)
          n = sub_800A672(N, n, G[i], idx)   // if idx <= n: shift N[idx..] right, N[idx]=G[i], n+=1
if tag: t = PosRNG1()%0x13 + 1; BS+0x45 = t; swap(N[28],N[t]); swap(N[29],N[t+1])
copy N[0..30] → F
sub_8000D12(base, cnt, sw):
    repeat sw times: a = PosRNG1()%cnt; b = PosRNG1()%cnt; swap16(base[a], base[b])
```

PvP P0 in the trace has no reg, tag or giga: 30 swaps, 60 RNG1 draws (frames 70 and 1222).

**Consequence:** each core shuffles only its *own* folder, with its own unsynchronized RNG1, and only each player's custom-screen *result* crosses the link (§7.3). The port simulates both players' custom screens, so both shuffled folders are part of its round setup (custom-screen.md §0-§1).

### 3.2 Intro and round banner (mode states 0 and 4)

**Intro `sub_80091F0`, first call ([3]=0; frames 72 / 1224):**

1. BS+0x1A (round number): set to settings+5 if !(effects&0x400), otherwise incremented (PvP: 1, then 2). It survives chaining.
2. `sub_80E06F8`: spawn the intro controller (T4 index 2, object flag 0x04).
3. `sub_8007358`: `sub_8007368(settings+0xC)` walks the 4-byte actor entries until an entry whose high nibble is 0xF.
   - Entry format: byte0 low nibble bit 0 = alliance, byte0 high nibble = type, byte1 = x | y<<4.
   - The PvP list at 0x080B1992 is `01 25 00 00 | 00 22 00 00 | F0`: P1's navi at (5,2), P0's at (2,2).
   - Type 0 goes `sub_80073CC` → `sub_800753C`: spawn T1 index 0 (navi), flags |= 4, AIData+0 = 2 (player), NameID = navistats+0x29 + 0x1A0.
   - `sub_8007778` puts the navi into the alive list (BS+0x80 + 0x10·alliance) and does BS+4+alliance += 1.
   - `sub_80077D2` does BS+8+alliance += 1 and stores the NameID at BS+0x4C + 8·alliance + 2k.
   - Other entry types (`off_80073A0[type]`): 1 virus/navi (`sub_80073E2`), 2 T3#0xA9 with a probability (draws RNG2), 3 T3#0x6E, 4/5 nothing, 6 T4#0x41, 7 T3#0x9C, **8 rock T3#0x59** (`sub_80074FA`), 9 T3#0x7D, 0xA T3#0x98. Only types 0, 3, 8 and 9 occur in the lists `BattleSettingsList1` (0x080B0D88, 192 records) refers to: 28 distinct lists at 0x080B1989..0x080B1B46, extracted to the content's actor lists (`Stages::actor_lists`, a pack's `rules/stages.toml`), each keeping its address as `original_address` (settings bytes 12..16 carry it unchanged even when the runtime patches other settings bytes; `BattleSettings::netbattle_from_bytes` resolves it).
   - Type 8, rock: x/y from byte1, variant = byte2, side = the panel's current owner (PanelData+3); spawns via `sub_80CFBC4` with params {variant, 0, 3, 0} and damage 200. Rocks do **not** go through `sub_8007778`/`sub_80077D2`: no alive entry, no actor count, no NameID. See field-objects.md.
   - After the list: BS+0x12/0x13 = BS+4/5 (a halfword copy), and BS+0x80..0x9F is copied to BS+0xD0..0xEF.
   - Navi init spawns a T4 index-8 helper per navi.
   - Then `sub_80AA88C` makes **one RNG2 draw** (reward-chip pick for alliance-1 actors from `sub_80AED50`). For NameID 0x1A0 every table entry is 0xFFFF, so nothing is written.
4. `PauseBattle`; `sub_8014178` (custom gauge rate, §7.1); music (link: song 0x15). [3]=4.

Objects after frame 72, in update order: T4#2 intro controller, P1 navi, T4#8, P0 navi, T4#8.

**Later intro calls:**
- If [2]==0: `sub_800927C` (HUD setup), then [2]=4.
- Then wait until `sub_800139A()` bit 1, i.e. BS+0x5C & 2.

BS+0x5C is driven by the intro controller `sub_80E0638`:

| Frame | Event |
|---|---|
| 72 | sets bit 0x10 and calls `SetScreenFade(round > 1 ? 8 : 0, 0x10)` |
| 89 | fade done, 17 ticks after it was set: bit 0x01 |
| 122 | the enemy fade-in list `unk_2038170` is empty: bit 0x02, and the controller frees itself |

Only the **remote** navi (alliance ≠ BS+0x0D) enters the fade-in list, on tick 2 (`sub_80163B4` → `sub_800AA1A`). It runs a 16-step, 2-tick mosaic fade after bit 1 and leaves the list at frame 121 (`sub_801641A` → `sub_800AA40`). So intro *object state* depends on the core's perspective (BS+0x0D), even though the timing does not.

At frame 123 the handler sees bit 2: `sub_801DA48(1)`, `sub_801BECC(1)`, [1]=4. The intro lasts 52 ticks.

**Round banner `sub_80092A0`:**

| [2] | Handler | Behaviour | Ticks (round 1) |
|---|---|---|---|
| 0 | `sub_80092C0` | If BS+0x1A ≠ 0: BS+0x28 = 10, then decrement until < 0 → [2]=4. If BS+0x1A = 0: go straight to [1]=8. | 124–135 |
| 4 | `sub_80092F0` | Init: `sub_801E792(0, BS+0x1A)` (round banner, UI id 0x30). Then wait for `sub_801E754() == 0` → [2]=8. | 136–195 |
| 8 | `sub_8009314` | BS+0x28 = 10, decrement until < 0 → [1]=8, [2..3]=0 | 196–207 |

**Banner lifetime model** (used here, in fighting state 4 and in the result states). The drawing is presentation, but the lifetime gates the flow, so it must be modelled.

- `sub_801E792(id, arg)` (0x0801E792): if HUD task bit 15 is already set, do nothing and return 1. Otherwise:
  - reset `byte_2036840` ([0]=0, [7]=0);
  - set type [8] from `pt_801EF84[id]`;
  - set bit 15 in both HUD masks (eStruct2035280+0x40 and +0x44).
- Every tick, the HUD task `sub_801CE28` runs inside `sub_801BEE0`, which comes after the mode handler:
  - First `[7] += 1`. Exception: if the new value would be 5 while [0]==4 and the type is 2 or 4, [7] is held; those banners stay up until removed.
  - Then, by [0]:
    - 0: slide-in until [7] ≥ 5 → [0]=4, [7]=0;
    - 4: hold until [7] ≥ 0x30 → [0]=8, [7]=0;
    - 8: slide-out until [7] ≥ 5 → [0]=0xC (`sub_802FE6A`);
    - 0xC: clear bit 15 in both masks.
- `sub_801E754()` returns 0 when bit 15 is clear, 2 when holding (type 2/4 in [0]=4), and 1 otherwise.

Net effect: a banner started on tick T is seen as finished by the mode handler on tick **T+59** (the task clears during tick T+58). Measured: 136→195, 533→592, 1288→1347, 1910→1969.

### 3.3 Custom screen

#### 3.3.1 Entry

- **Round start:** the banner state `sub_8009314` sets [1]=8 (frame 207). The battle is already paused.
- **Mid-battle** (trace-verified in soundmod round 2 from 26113, after FullCust filled the gauge):

| Tick | Where | Effect |
|---|---|---|
| T0 | navi object (`sub_8012FC8`, 0x08013044–0x0801306C) | If !timestop, flag 0x40 clear, battle mode ≠ 1, `(sub_801032C() & 0x400) == 0`, flag 2 set and the navi's AIData pressed & 0x300 (L or R): `battle_setFlags(0x10)`; that navi skips the rest of its input handling this tick. Either player can do this; the flag is global. |
| T1 | `sub_80080D2` | `sub_800A1D0()` is true → `PauseBattle`, machine [0]=0x20 |
| T2 | `sub_8008452` (0x20) | Init: `sub_802D6A0` (record both navis in `dword_203C970`); `sub_8015A16(navi0/1)` (AIData+0x0F −= 1 unless 0 or 0xFF, only if stats[0x29]==0); [3]=4. Then `sub_802D6C4` starts reverting any cross/beast form. |
| T3… | `sub_8008452` | Wait until `sub_802D6C4() == 0` → [0]=0x24 (word store). `sub_802D6C4`'s first call would ask each navi to leave its Cross (request 0x8000000, `sub_802DD10`), but its test `sub_802DD1E` always returns 0; the second call waits while either navi has AIData+0x48 bit 0x2000 (a Cross knockout), so with none it is done on T3. |
| T4 | `sub_8008492` (0x24) | `sub_801483C() == 0` → `sub_801482C` (reset sequencer), [2]=4 |
| T5–T6 | `sub_8008492` | Sequencer `sub_801486C` → `sub_8014A00`. When `sub_801483C() == 0`: [4]=6. In the same tick `sub_800938A` sees 6 → BS[1]=8, [2..3]=0. |
| T7 | `sub_8009338` | Custom-screen init |

`sub_800A1D0` (0x0800A1D0) is true when all of the following hold:
- not time stop;
- not battle over (`tst` form);
- and either:
  - (NaviStats(0)+0x2C or NaviStats(1)+0x2C ∈ {0x17,0x18}) **and** flag 2; or
  - flag 0x10.

Inside `sub_80080D2` the checks run in this order: KO/result, then START pause, then the custom check. The flag-0x40 path (`sub_800A244`: L/R with per-player gauge ≥ 0x2900) is not used in PvP.

#### 3.3.2 Mode state 8, `sub_8009338`

```
if BS[3] == 0: sub_800A9CA(4) (BS+0x11 |= 4); BS[3] = 1; sub_8026840()
r = sub_8026A28()
r == 0: stay
r == 2: sub_800AABC(): setTwoStructs_800A840(4), BS[1] = 0x14          // escape; not PvP
r == 1: for p in 0,1: if actor(p): actor(p).AIData+0x0F = 1
        BS[1] = 0x0C, [2..3] = 0
```

`sub_8026840` (custom init):
- If BS+7 == 0: zero `dword_20349A0` (0x14 bytes) and call `sub_802A210`.
- `sub_801DF92`: gauge = 0, clear flags 0x12.
- Disable HUD tasks with `sub_801BED6(0x20130)` / `sub_801DACC(0x30172)`; this includes the gauge fill (bit 4).
- Reset the UI control block `dword_20364C0` (0x70 bytes).
- **BS+7 += 1** (custom-screen/turn counter).

#### 3.3.3 The UI (`sub_8026A28`, local only)

The screen's rules and the port's model (both players' screens simulated) are in [`custom-screen.md`](custom-screen.md).

The UI reads the *local* joypad (`eJoypad`), not the link-synced records. It also slides the camera (`Camera+0x34 ±0x18000`). Round 1 on core 0:

| Frames | UI state | Notes |
|---|---|---|
| 208 | init → [0]=4 | BS+0x11 \|= 4 |
| 209–218 | [1]=0 `sub_8026B04` slide-in (10 ticks) | On completion, except on the first custom screen of the battle: `sub_8013FD0(0)`, `sub_8013FD0(1)` — **simulation:** if NaviStats(p)+0x54 (u16 custom-HP bug) ≠ 0, `object_subtractHP(min(v, HP−1))` (never kills) |
| 218–374 | [1]=4 `sub_8026CCC` selection | player-driven |
| 375 | confirm | `sub_8029110` builds the local block in `byte_20366C0` |
| 376–385 | [1]=8 `sub_8026BF4` slide-out (10 ticks) | Tick 376: `sub_800A9D6(4)` clears BS+0x11 bit 2 |
| 386–527 | [1]=0x14 `sub_8026DC4` | First tick: `sub_8027D78` (re-enables gauge: `sub_801DED0` → `sub_801DF92` + HUD bit 4, unless `sub_800A97A()`), `sub_802A4FC`, `sub_800B3A2` (start chip exchange, §7.3), `sub_801E474(0)`. Then wait. |
| 527 | both tokens present | `sub_800B460`, **`sub_800B3D8`** installs the blocks and navi stats; [0]=8 |
| 528 | `sub_8026A6C` returns 1 | mode [1]=0xC |

The screen ends on the tick both players' exchanges have completed, and that tick is the same on both cores. The local timings before that point are core-specific. The only local actions with simulation effect are:
- `sub_8013FD0` (fixed 10 ticks after init on both cores);
- the gauge reset in `sub_8027D78` (idempotent while paused);
- the transmitted status bit.

### 3.4 Fighting (mode state 0xC)

`sub_800938A`:
1. On the first tick (BS[3]==0): `sub_80084C0` (zero the machine), BS[3]=4.
2. Every tick: `sub_800801C`, then `sub_800B090`, then react to r0.

`sub_800B090` is a local-player-only anti-tamper check. It saves and restores r0–r7, so r0 is still [4]. It may rewrite the local player's next chip id to 0x185 if `sub_8006EE8` fails, which never happens for legal chips. Its mega/giga limit check is dead because the limit is overwritten by a pointer.

Reaction to r0:
- **6** → BS[1]=8 (custom).
- Any **nonzero** value → `eStruct200A008_setUnk03(BS+7)`, plus the result handling in §3.6.

**State 0, `sub_800840C`:**
1. Init: `sub_80147E4(byte_203F558, byte_203F658)`. These are the two transform records received with the chip exchange (§7.3).
2. Wait for `sub_801483C() == 0` (transformation sequencer idle).
3. If [2]==0: `sub_801482C`, [2]=4, and wait again.
4. Then `sub_80103BC(0)`/`sub_8015A38`, `sub_80103BC(1)`/`sub_8015A38`. `sub_8015A38` decrements NaviStats+0x21 (beast-out counter) when all of these hold: the navi is MegaMan (+0x29 == 0), the starting form +0x17 is not 0x0B/0x0C, the form +0x2C is in 0x0B..0x18, and the counter is not already 0. In round 2 it went 3 → 2 at 1909.
5. [0]=4.

This took 4 ticks in round 1 and 134 in round 2 (P0 requested form 0x0C).

#### 3.4.1 The transformation sequencer (`sub_801483C`)

State lives at `dword_20367F0`: [0] state (0 check, 4 transform, 8 wait), [1] transform sub-state, [3] sub-state init, [4] busy, then the two 0x10-byte transform records (+8 side 0, +0x18 side 1). `sub_80147E4` copies the exchanged records there **and** to `unk_203A980 + 0x10·alliance`, which the navis read through `sub_801595E(alliance)`. It sets [0..3] = 0 and busy = 1. `sub_801482C` only resets [0..3] and busy; the records stay.

A transform record: +0 requested form (0xFF none), +4 Cross change (0xFF none), +8 the requesting navi object (always the side's player). +1 and +3 are written by the custom screen (`sub_8015952`), and nothing in battle reads them.

- **State 0, `sub_801486C`** (one tick), per side: if +4 ≠ 0xFF, `sub_802DCDE` (request 0x4000000) and then the Beast Out check; else if +0 ≠ 0xFF, note that someone transforms and skip the check; else the Beast Out check (`sub_80159C6`, then `sub_8015994` if it says the Beast Out ran out). Next state: 4 if anyone transforms, else 8.
- **State 4, `sub_80148CC`**, on [1]:
  - 0 `sub_80148EC`: on init, `SetScreenFade(0x44, 0x10)` (0x70 in battle mode 1) and HUD off. When the fade is done: [1]=4, [2..3]=0. Fade 0x44 takes **16** ticks: started at 1777, first seen done at 1793.
  - 4 `sub_8014944`: on init, `sub_801596E` (request 0x4000) for each side with +0 ≠ 0xFF, then return. Afterwards, wait while either navi has AIData+0x48 bit 0x80 (`sub_801597C`). Then both +0 = 0xFF, [1]=8, [2..3]=0.
  - 8 `sub_801498E`: on init, `SetScreenFade(0x40, 0x10)` (0x6C in mode 1). When done: HUD back, [0..3] = 8 (word store). Fade 0x40 takes **17** ticks: 1889 → 1906.
- **State 8, `sub_8014A00`**: wait while either navi reverts (`sub_80159A2`: AIData+0x48 bit 0x100 or request 0x40). Then, per side in order, wait while it changes Cross (`sub_802DCEC`: +0x48 bit 0x1000 or request 0x4000000), clearing that side's +4. Then busy = 0.

The navi's side of the change is action 0x1C, run by the pause handler (objects-and-player.md §M4.2, and §12.9 for Beast Out). Round 2 timeline: sequencer init 1776, fade-out 1777–1793, request 1794 (the navi enters action 0x1C in the same tick), change done 1887, fade-in 1889–1906, busy cleared 1906, second (reset) run 1907–1908, fighting state 4 at 1909.

Neither `sub_801486C` nor `sub_8014A00` is affected by the custom screen's close setting AIData+0x0F = 1 for both navis (§3.3.2): that value is correct, and it makes the first `sub_80159C6` of the turn return early. The Beast Out end check only runs after a mid-battle custom screen, whose state 0x20 decrements +0x0F to 0 (`sub_8015A16`).

**State 4, `sub_8008064`:**
1. Every tick: `sub_8012DFC(0)`, `sub_8012DFC(1)` (inputs → actors, §6.5).
2. Init tick: [8]=0x1E (unused afterwards), [3]=4. Then:
   - if `sub_800A97A()` (link and BS+7 ≥ 15): [0xA] = 0xA5·4−1 = 659 and banner `sub_801E792(0x10,0)`;
   - else, in link: banner `sub_801E792(0xC, BS+7)` (turn banner).
3. When `sub_801E754() == 0`: `sub_801E724(0xFF,0)`, `sub_801E0C8(0xFFFF)`, [0]=8. The battle stays paused through this state.

**State 8, `sub_80080D2`**, every tick in this order:

1. `sub_8012DFC(0)`, `sub_8012DFC(1)`.
2. `UnpauseBattle`.
3. `battle_setFlags(1)`.
4. `sub_800AE0C`: combo bookkeeping.
   - If BS+0x1C > BS+0x1B: BS+0x1B = BS+0x1C, and a combo message if it is ≥ 2.
   - If BS+0x1D: BS+0x1D −= 1, else BS+0x1C = 0. (`sub_800AE44` on hits does +0x1C += 1, +0x1D = 10.)
5. `sub_800A6A6`: battle time. BS+0x40 += 1 (cap 0x8C9F) if not timestop, not paused, flag 1 set, and `battle_isBattleOver` is false *in its Z-flag form* (§3.6).
6. If `sub_800A97A()`: `sub_800AB7C` (turn timer, §3.6).
7. `r = sub_800A152()`:
   - 1 → if BS+0x3A ≠ 0: `sub_800AAD6` (escape). Else BS+0x18 += 1 and [0]=0xC.
   - 2 → BS+0x19 += 1, [0]=0x10.
   - 7 → [0]=0x18.
   - After any of these, return.
8. `p = sub_800A046()`. If p ≠ 0xFF: [5]=p, `PauseBattle`, [0]=0x1C, `sub_801E15C` (HUD and sound only), return.
9. Flag-0x40 mode only (`sub_800A8F8`): `sub_800A244`. In PvP: `sub_800A1D0()` → if true, `PauseBattle`, [0]=0x20.

The first fighting tick is when the battle unpauses: frames 593 / 1970.

### 3.5 START pause

`sub_800A046` (0x0800A046):
- returns 0xFF if the battle is over (`tst` form) or in time stop;
- else 0 if P0's record has START pressed;
- else 1 if P1's has;
- else 0xFF.

Nothing gates it on link, so **pause is reachable in PvP** (the trace never pauses).

State 0x1C, `sub_80083E4`: only the pausing player [5] can resume, with a new START edge (`sub_800A07C([5])`). On that edge: sound 0x9F, [0]=8, `sub_801DACC(0x200)`. The pause byte stays 1 for the rest of that tick; `sub_80080D2` unpauses at the top of the next tick. While in 0x1C, `sub_8012DFC` is not called, so actor inputs are frozen. What runs while paused is listed in §8.3.

### 3.6 Round end: KO, result states, turn timer, damage judge

#### Alive bookkeeping

| Field | Meaning | Writers |
|---|---|---|
| BS+0x04 / +0x05 | counted actors per alliance | +1 in `sub_8007778` (spawn) when AIData+2 ≠ 1. −1 only in `sub_800A104` (0x0800A104), and only if AIData+2 == 0; called from the actor-removal state `sub_8016C4E` on its first phase. |
| BS+0x12 / +0x13 | alive navis per alliance | Set at spawn (above). −1 in `sub_800A11C` (0x0800A11C), which also nulls the actor's slot in BS+0x80..0x9F. Callers: the deletion entry `sub_801741C` (0x08017462, if Param2 < 1), `sub_80165F8`, `sub_8016EE0`, `sub_80170E4`, `sub_811648C`, `sub_811670E`. |
| BS+0x0B | time up | 1 via `sub_800AB7C` (turn timer), or `sub_80D8DEE` (special object, not PvP). Cleared only by the battle-start zero-fill. |
| BS+0x10 | winning alliance | BS+0xD on win, BS+0xD^1 on loss |
| BS+0x18 / +0x19 | local round wins / losses | 0x0800811A / 0x08008130 (KO), 0x080083C2 / 0x080083D6 (judge) |

Trace: +0x13 drops to 0 at 938 / 2346, the tick P1's HP reaches 0; +5 drops to 0 at 993 / 2401, when the deletion finishes, a fixed 55 ticks later.

**`battle_isBattleOver`** (0x0800A18E) returns r0=1 if +0x12==0, or +0x13==0, or +0x0B≠0; otherwise r0=0.

**Flag quirk (must reproduce):** seven callers branch on the Z flag the function leaves instead of testing r0. They are `sub_800A6A6` (0x0800A466) and six object handlers in asm31, for example 0x080C7DF0 and 0x080D8D26.
- On both KO paths the function returns right after a `tst` of a zero byte, so Z=1.
- On the not-over path it returns after `movs r0,#0`, so Z=1.
- Only the +0x0B path has Z=0.
- So those callers treat a KO as "not over". Visible effect: BS+0x40 keeps counting after the KO (938→994: 0x15A→0x192).

**`sub_800A152`** (0x0800A152), the result from the local perspective:

```
if battle_isTimeStop(): return 0
if BS[4] == 0: return BS[0xD] == 0 ? 2 : 1       // alliance 0 wiped out
if BS[5] == 0: return BS[0xD] == 0 ? 1 : 2       // alliance 1 wiped out
if BS[0xB] != 0: return 7                         // time up with both standing → judge
return 0
```

Alliance 0 is tested first, so there is no KO draw; a double KO goes to whichever removal (`sub_800A104`) lands first.

KO chain, round 1:
1. **938:** HP reaches 0; deletion state starts; +0x13 = 0.
2. **939–992:** deletion animation. The fight continues and navis keep acting.
3. **993:** removal; +5 = 0. The navi object stays in the list.
4. **994:** `sub_80080D2` sees `sub_800A152() == 1` → BS+0x18 += 1, [0]=0xC.

`sub_802CEC8` is not part of KO detection (§4).

#### Result states

| State | Init tick | Every tick | Exit |
|---|---|---|---|
| 0xC win, `sub_80081A4` | See below. | [8] −= 1 (the init tick included) | `sub_801E754()==0` and (s16)[8] ≤ 0 → [4]=1, GameState+0x14=1, `eStruct200A008_setUnk02(0)` |
| 0x10 lose, `sub_800825A` | See below. | [8] −= 1 | Same condition → [4]=2, GameState+0x14=2, 0x0200A00A=1 |
| 0x14 draw, `sub_80082DC` | HUD teardown, [8]=0x66, banner 0x1C | [8] −= 1 | Waits **only** for the banner: its [8] test (`ldrh`/`tst`/`blt`) never branches. Then, if effects&0x400: `sub_800AF50()` == 1 → [0]=0xC; == −1 → [0]=0x10. Otherwise `setTwoStructs_800A840(3)`, GameState+0x14=3, [4]=3. |

Win init tick, in order:
- `sub_801DACC(0xE4C53)` and `sub_801BED6(0xE4C53)`: hide HUD elements, including gauge task bit 4.
- `sub_8014040(BS+0xD)`: heals only outside link.
- BS+0x10 = BS+0xD; [3]=4.
- If `sub_800A7A6(0x173,0x17E)` ≠ 0: [8]=0x66, no banner.
- Else (PvP): `PlayMusic(SONG_WINNER_1)`, [8]=0x66, banner `sub_801E792(sub_800A8D4())`. In link the banner id comes from table 0x0800A8EC indexed by navistats+0x29.

Lose init tick: the same HUD teardown; BS+0x10 = BS+0xD^1; link: `PlayMusic(SONG_LOSER)`; [8] = effects&2 ? 0x5E : 0x66 (PvP 0x66); banner `sub_800A8B2()`, or 0x18 if `sub_800A152()==7`.

Measured: [0]=0xC set at 994 / 2402, init at 995 / 2403, [4]=1 at **1096 / 2504**. That is 102 ticks including the init tick. The win banner only lasts 58 ticks, so the 0x66 timer is the binding one.

During the result states the battle is not paused: objects, panels and timers keep running, but `sub_8012DFC` is not called.

**Mode handler reaction** (`sub_800938A`):
- r0 = 1: `setTwoStructs_800A840(1)` (BS+0x1F and 0x0200A009); BS+0x1E = `sub_800AF84()` (busting level 1..11, 0x0B in both rounds; also stored to 0x0200A008); `sub_800B6F2`.
- r0 = 2: `setTwoStructs(2)`, BS+0x1E.
- r0 = 3: nothing extra.

The next mode state is **0x14** for PvP (mode 0, and effects has bit 0x2 clear, so the end-exchange state 0x10 is not used). The PvP branch conditions are exactly those.

Result code (BS+0x1F low nibble, `sub_800A832`): 1 win, 2 lose, 3 draw, 4 escape, 5 comm error, 9/0xA terminate.

#### Turn timer (static only)

The only PvP time limit applies from the 15th custom screen on. `sub_800A97A` (0x0800A97A) = effects&8 && BS+7 ≥ 15.

On fighting state 4's init tick, if it holds, [0xA] = 659 and the turn banner is `(0x10, 0)`. Each fighting tick, `sub_80080D2` calls `sub_800AB7C` (0x0800AB7C):

```
if paused or timestop: nothing
elif battle_isBattleOver() (tst form): sub_801DACC(0x800); return
elif [0xA] >= 0x3C: [0xA] -= 1
else: BS[0xB] = 1
display [0xA]/60 (sub_801E398)
```

After exactly 600 decrements (659 → 59), the next tick sets BS+0x0B. Later in the same tick `sub_800A152()` returns 7 → [0]=0x18; a KO seen in the same tick takes precedence. The timer is re-armed on every fighting entry. Also, from turn 15 on the gauge never arms (§7.1), so custom screens stop.

#### Damage judge, state 0x18 `sub_800834A` (static only)

T is the tick that set 0x18.

| Ticks | Code | Action |
|---|---|---|
| T+1..T+60 | `sub_8008364` | [8] counts 1..60; at 60, `sub_801DACC(0x800)`, [2]=4 |
| T+61 | `sub_800838A` init | See below. |
| T+62..T+275 | judge UI machine `sub_802CB78` (0x0203EAD0) | T+65..T+123: **one `GetPositiveSignedRNG2` per tick, 59 draws** (rolling digits, mod 0x270E; via `sub_802CBAC` → `sub_802CBF2` → `sub_802CC1A`). T+125..T+244: show the values. T+245..T+274: fade. T+275: done. |
| T+275 | `sub_800838A` | outcome 1 → BS+0x18 += 1, [0]=0xC; 2 → BS+0x19 += 1, [0]=0x10; else [0]=0x14 |

The T+61 init tick calls `sub_802CB38(dmg(1), dmg(0))`, where dmg(p) = `sub_801055E(p)` = actor(p).AIData+0x20 (TotalDamageTaken, u16):
- loser = 0xFF if the two are equal, 0 if dmg(1) < dmg(0), else 1. It is written to 0x0203CA80 (the fighting machine's r5+0x10).
- outcome = 3 if equal, 2 if loser == BS+0xD, else 1 (0x0203EAD7).

In short: less total damage taken wins, and equal damage is a draw.

### 3.7 Battle end

**Mode 0x14, `sub_80094DA`**, entered at 1096 / 2504:
1. Init tick (1097 / 2505): `SetScreenFade(sub_800A7A6(0x173,0x17E) && code==1 ? 4 : 0xC, 0x10)`, [3]=4.
2. The fade advances outside `battle_8007800` (16 steps).
3. At 1113 / 2521 the fade is done: `musicGameState_8000784`, `FreeAllObjectsOfSpecifiedTypes(0x1A)` (types 1, 3, 4), top state = 8. Model the fade as a 16-tick wait.

**Top 8, `sub_8007B80`**, [1]=0 → `sub_8007B9C`. Each frame it runs: `sub_800A01C`, the sub-state, `RunBattleObjectLogic`, `sub_800BFC4`, `sub_80027B4`, `sub_800286C`. There is no input application, no mode handler and no BS+0x64 increment (BS+0x60 still increments).

- [2]=0, `sub_8007BD0`: BS+0x28 = 10 at 1114; decrements; below 0 at 1125 → [2]=4.
- [2]=4, `sub_8007BF0` → `sub_8007C14` (`eStruct200BC30[0]` = 0):
  - 1126: `sub_803DDA4` (link teardown request), [3]=4;
  - wait on `sub_803DE24`;
  - 1145: `sub_81440D8`, `sub_801FE64`, [1]=4.
  - This took 19 frames in both rounds. It depends on the link protocol and touches no simulation state.

**[1]=4, `sub_8007CA0`** (1146 / 2554). If effects&0x400 (set match) and the code is not in {5,9,0xA}, compute `s = sub_800AF50()`:

```
w = BS[0x18]; l = BS[0x19]; r = BS[0x1A]; rem = 3 - r
if w > l + rem: return 1        // set won
if l > w + rem: return -1       // set lost
if r >= 3: return 2             // set drawn
return 0                        // another round
```

- s == 0 → chain the next round (§3.8).
- s ≠ 0, in order:
  - `setTwoStructs_800A840(s==2 ? 3 : s==1 ? 1 : 2)`;
  - code 1: `dword_2000B30` += 1 (halfword, cap 0x11);
  - `sub_800FAE0` (writes BS+0x34 = the local navi's HP, 0x03E8 in the trace);
  - `sub_800A86E`;
  - post-battle save bookkeeping on the PET navi, keyed on effects bits 0x10, 0x40, 0x1000, 0x10000, 0x40000, 0x400000 and 0x800000 (part of it skipped when effects&8). It does not touch battle state;
  - `sub_802CA82` (rewards; none in link);
  - `loc_8007E38`: BattlePaused = 0, clear `flags32_20093A4` bit 1, clear EVENT_1722, **BS+0x0A = 0**.
  - `battle_8007800` then returns 0 and `sub_812B698` ends the battle. Frame 2554: BS+0x1F=1, BS+0x18/19/1A = 2/0/2.
- Whether chained or not, `sub_8007CA0` starts with `musicGameState_8000784` (all sound stops). Mode 4 runs only `sub_8007CA0`: no objects, no panels.
- Codes 5, 9 and 0xA (link error, terminate) skip the set check; code 5 outside link battles can restart the same battle (`loc_80071FE`, EVENT_1733). The engine does not implement these endings.

**Engine model.** `Battle::round_end()` is `None` until the tick that runs `sub_8007CA0`, then:
- `RoundEnd::NextRound { settings, score }` when the set goes on: the top state goes back to 0 (init), which the engine does not simulate. The host runs its init (link sync, the navi stats and RNG exchange, the folder) and starts the next round from a `RoundSetup` with these settings and score.
- `RoundEnd::Over(BattleResult)` when it ends: BS+0x1F holds the result code, BS+0x34 the local navi's HP (read from the freed object, `sub_800FAE0`), the pause byte is cleared and BS+0x0A = 0. Machgun round 2 ends this way at 2554 (1331/1331 frames).

**Error paths (not in the trace):**
- **Top 0xC, `sub_8007E62`**, on link status 4 or an init/exchange timeout:
  1. `sub_8007EB8`: pause, link stop, code 5, chatbox.
  2. `sub_8007F14`: wait for the chatbox, then `SetScreenFade(0xC,0x10)`.
  3. `sub_8007F2C`: wait for the fade, free objects, → top 8 / [1]=4.
  - Every frame it runs the render tail and forces link status 2.
- **Top 0x10, `sub_8007F4E`**, terminate:
  1. `sub_8007FA4`: pause, fade.
  2. `sub_8007FD2`: wait until `sub_813D60C()==0` → top 8 / [1]=4.

### 3.8 Multi-round sets

A PvP match is the game's own set. Match type 1 (best of three) ORs 0x600 into the effects (bit 0x400 = set). Chaining happens **inside `battle_8007800`**: the comm menu never runs between rounds, and there are zero non-battle frames (RNG1 advances by exactly one between frames 1145 and 1146).

When `s == 0`, `sub_8007CA0` does the following:
1. `SetDummyBGScrollCallbacks`, `zeroFill_e20094C0`, `sub_80023A8`.
2. Save the dword BS+0x18..0x1B (wins, losses, round, max combo).
3. `battleSettings_802D2B2()`: copy `BattleSettingsList1[byte_203CA50[2·(BS+0x1A−1)]]` (0x080B0D88 + 0x10·idx) to 0x0200AF60. It keeps the effects dword and sets background = `byte_203CA50[2·(BS+0x1A−1)+1]`. Round 2 was List1[0x11] with background 3, from P0's round-1 draws `11 03 | 46 13`.
4. `loc_8007204(settings)`: battle start, with no big RAM clear.
5. Restore the saved dword (0x01010001 at 1146).

Init reruns on the next frame.

Carried over between rounds:
- BS+0x18..0x1B;
- the effects dword of the settings;
- anything outside the re-initialized structures.

`byte_203CA50` is not carried: every init copies it from player 0's init exchange (`battle_copyStructsIncludingBattleStats_800b2d8`, from `dword_203F568`). It holds two (settings index, background) pairs: the pair used after round n is entry n−1 of the copy made for round n. Machgun round 1 had `11 03 46 13` (round 2 on List1[0x11], background 3) and round 2 `3D 13 59 00`. Soundmod rounds 1 and 2 had `43 0B 02 0D` and `48 11 14 04`: rounds 2 and 3 were List1[0x43] background 0x0B and List1[0x14] background 4, as their setups show. The engine takes the pairs as `RoundSetup::later_stages` and the table from the content (`Stages::settings`, all 192 records).

BS+0x20 is not carried either (§2.2).

Redone every round:
- link re-sync and the init exchange (fresh navi stats; any HP changes do not carry over through BattleState);
- RNG2 re-adoption from P0 (now a deterministic function of the previous round);
- panels;
- the folder, rebuilt and reshuffled with RNG1.

BS+0x18/0x19 are local-perspective.

---

## 4. The battle tick: `battle_8007A44` (0x08007A44)

This is the exact call order. "Sim" = the call changes simulation state (anything that later feeds object behaviour, HP, RNG2, flow timing, BattleState, chip blocks or inputs).

| # | Call | Gate | Sim | Effect |
|---|---|---|---|---|
| 1 | `eStruct203F7D8_getUnk01` (0x0801FEE8) | always | link | Status s = `eStruct203F7D8[1]`. If s == 4: if `eStruct200BC30[0xE]` ∈ {8,2} then `setTwoStructs_800A840(9)` and top state 0x10, else top state 0xC; either way skip to #18. |
| 2 | `sub_801FEEE(s)` (0x0801FEEE) | s ≠ 4 | **yes** | Apply both rx packets and build tx (§6). Cable uses `sub_801FF18`. |
| 3 | — | s & 8 | — | Link stall: skip #4–#17 (no tick), run #18 only. |
| 4 | `eStruct2038160_getBattleTerminate01` (0x0802015E) | | link | Nonzero → `setTwoStructs(v==1 ? 9 : 0xA)`, top 0x10, skip to #18. Never in cable PvP. |
| 5 | `sub_800A01C` (0x0800A01C) | | no | `byte_3000EA8 = 0`: empty the deferred sprite queue (`sub_8009FF8` → `sub_8009FCC`). |
| 6 | `off_8007B50[GetBattleMode()]` | | **yes** | Mode handler; PvP → `sub_8009158` (§2.2, §3). |
| 7 | `RunBattleObjectLogic` (0x080031AC) | | **yes** | All objects in list order, with per-object pause/time-stop gating (§8.4). |
| 8 | `sub_802FFF4` (0x0802FFF4) | | RNG1 only | Camera follow and shake, BG scroll. Two `GetRNG1` calls per tick while a shake is active (§5.3). |
| 9 | `sub_800BFC4` (0x0800BFC4) | !paused && !timestop | **yes** | See below. |
| 10 | `sub_800FDC0` (0x0800FDC0) | | **yes** | Chip block → actor `ChipsHeld`/`Chip` (§7.4). |
| 11 | `sub_801BEE0` (0x0801BEE0) | | **mixed** | HUD update tasks: for each set bit i of `eStruct2035280+0x40` (0x020352C0), LSB first, call `off_801BF04[i]`. Simulation-relevant: **bit 4 `sub_801C470` custom gauge** (§7.1) and **bit 15 `sub_801CE28` banner lifetime** (§3.2). The rest are presentation (§9). |
| 12 | `sub_802CEC8` (0x0802CEC8) | | **yes** | For i = 0,1: record `unk_2036720 + 0x10·i`. If the object pointer at +8 is non-null and that object's HP (+0x24) is 0: `sub_802CEA6(obj.alliance)`, which clears the record and sets `[[rec+0xC]+5] = 1`. This is a per-alliance linked-object registry. |
| 13 | `chip_800AEE8` (0x0800AEE8) | | **yes** | For alliance 0 then 1: refresh the damage of the next chip if its ChipData+9 has bit 0x80 (§7.4). |
| 14 | timers | !paused && !timestop | **yes** | `BS[0x0E] = (BS[0x0E]+1) % 20`; `BS[0x16] = (BS[0x16]+1) % 180`. Read by `sub_801A186` (grass panel + Wood element: +1 HP when BS+0x0E==0 if HP > 9, else when BS+0x16==0). |
| 15 | `sub_802CDFE` (0x0802CDFE) | !paused (also runs in time stop) | **yes** | `*(u32*)0x0203CFB0 <<= 16; *(u32*)0x0203CFBC <<= 16`: per-alliance damage-carry records (+0 u16 this tick, +2 previous tick, +4/+8 object pointers). Read by `sub_802CE10` from `object_calculateFinalDamage1` (0x0800E3DE); the tracking is set up by `sub_80C913C`. |
| 16 | `sub_80102AC(0)`, then `sub_80102AC(1)` if effects&8 | always, even paused | **yes** | See below. |
| 17 | `BS+0x64 += 1` | | **yes** | Tick counter. |
| 18 | render tail | always, including stall/error frames | no | `sub_80027B4`, `sub_800286C`, `sub_8003E18`, `sub_8004218`, `sub_8004510`, `sub_800C5E0`, `sub_801BF64`, `sub_802E156`, `sub_8003C70`, `sub_80046F8`, `sub_80049B0`, `sub_8009FCC`, `sub_803C59C(0xE0,0x90)` (§9) |

#9 `sub_800BFC4` does, in order:
1. `sub_800C746`.
2. `byte_203CB04 −= 1`, reloading 0x8C when it reaches 0.
3. For y=1..3, x=1..6: `sub_800C380(x,y)`, then panel+0x18 = `object_getPanelParameters(x,y)` if panel type (+2) == 3, else 0.

Panel semantics are in the panel spec.

#16 `sub_80102AC(p)`, the NaviCust HP drain:
- Runs only when `BS[0x14+p] & 5` (p's transmitted status; only bit 2 = "in custom screen" is ever set, §6.3).
- `rate = byte_80102F8[navistats(p)+0x19]`, table {0,40,30,20,10,5,3,2}; nothing happens if rate is 0.
- If actor(p) exists and has HP > 1: AIData+0x0A += 1; when it reaches rate, `object_subtractHP(1)` and reset to 0.
- **So the drain acts only while that player is in the custom screen.**

**Per-phase view.** Only #6 differs between phases:
- **Custom phase:** #6 = `sub_8009338` → local UI. The battle is paused: #9/#14/#15 are skipped; the gauge task is disabled; objects run only with flag 0x04; the HP drain is active for players whose status bit 2 has arrived.
- **Fighting phase:** #6 = `sub_800938A` → `sub_800801C` → `sub_80080D2` (§3.4) → `sub_802DE5C` (no-op) → `sub_800B090`.

---

## 5. Random number generators

### 5.1 Step function

RNG1 state is at **0x02001120** (`GetRNG1` 0x0800154C, `GetPositiveSignedRNG1` 0x08001562). RNG2 state is at **0x020013F0** (`GetRNG2` 0x0800151C, `GetPositiveSignedRNG2` 0x08001532). Both use the same step:

```rust
fn step(s: u32) -> u32 { s.rotate_left(1).wrapping_add(1) ^ 0x873C_A9E5 }
// GetRNGx:               state = step(state); return state
// GetPositiveSignedRNGx: state = step(state); return state & 0x7FFF_FFFF
```

The stored state is always the full 32 bits. Callers usually reduce with `svc 6` (`SWI_Div`, remainder): `value % n`. `SeedRNG2` (0x08001514, 0xA338244F at power-on) is irrelevant because init overwrites RNG2.

### 5.2 Roles

- **RNG2 is the simulation RNG.**
  - It is synchronized at init (§3.1.1): RNG2 := P0's pre-draw value, delivered by the init exchange (frame 69 / 1221).
  - After that, only code that runs identically on both cores advances it.
  - All battle-object RNG sites use RNG2: hundreds of `GetPositiveSignedRNG2` callers in 0x080A…–0x0811…, plus `_GetRandomRelativePanelFiltered` and `object_getRandomPanelFromCurrentColumn`.
- **RNG1 is per-core.** Tango seeds it differently per core and it is never re-synchronized. It is advanced by:
  - the main loop;
  - presentation code;
  - the local settings generator;
  - the local folder shuffle;
  - the local custom screen.

  No lockstep code reads it. An engine that only needs simulation state can keep one RNG1 per player (or drop it). To reproduce the trace's `rng1` column, model the sites below for core 0.

### 5.3 Advances on a battle frame outside object handlers

These are all RNG users reachable from `battle_8007800` without going through `RunBattleObjectLogic` or the object jump tables (static call graph).

| Site | RNG | When | Draws | Seen in trace |
|---|---|---|---|---|
| main loop 0x0800031C | 1 | every frame, after `battle_8007800` | 1 | every frame |
| `camera_doShakeEffect_80301e8` ← `sub_802FFF4` | 1 | every tick with an active shake | 2 | 1501–1540, 1802–1861 |
| `sub_801CC94` (HUD mugshot blink) ← task `sub_801CADC` (bit 14) | 1 | when its 20-tick timer (eStruct2035280+0x38) expires and the blink condition holds | 1 | no |
| `sub_81209DC` ×2 ← `sub_800B144` ← init `sub_80079A8` | 1 and **2** | once per round, init frame 3 / 1155 | 2 + 2 | yes (RNG2 then overwritten at 69) |
| `sub_800A570` / `sub_8000D12` ← `sub_800A3E4` ← `sub_80079F0` | 1 | once per round, folder | 2 per swap, +1 per giga, +1 with tags | 60 at 70 / 1222 |
| `sub_8029688`, `sub_8029788` → `sub_8000D12` | 1 | custom-screen folder handling | 2 per swap | no |
| `sub_8026F1A` | 1 | custom-screen escape check (not PvP) | 1 | no |
| `sub_80AA88C` ← `sub_8007358` ← intro | **2** | first intro tick | 1 | 72 / 1224 |
| `sub_8007424` (+ `sub_80DA9FE`) ← `sub_8007368` | **2** | intro, only for actor entries of type 2 (none in PvP) | 1+ | no |
| `sub_802CC1A` ← … ← `sub_802CB78` ← judge | **2** | damage judge T+65..T+123 | 1 per tick (59) | no |
| `sub_802E6EC` → `sub_8000CDA` ← `sub_802DE5C` | **2** | battle flag 0x40 only | 2 per swap | no |
| `sub_8000C72` / `sub_80AA910` … ← `sub_800945C` | **2** | mode 0x10 end exchange (not PvP) | many | no |

**Camera shake** (`camera_doShakeEffect_80301e8`, called every tick from `sub_802FFF4` regardless of pause). `eCamera` (0x02009980) has two channels:
- primary: +0x0C counter, +0x0E magnitude index;
- secondary: +0x10 counter, +0x12 magnitude index.

Object code starts them with `camera_initShakeEffect_80302a8(mag, n)` and `sub_80302B6(mag, n)`.

Each tick:
1. **Primary channel.** Used if all of these hold:
   - the primary counter is nonzero;
   - at least one of:
     - `sub_80269D0()` (BS+0x14 & 5),
     - `!IsCurSubsystemInUse()`,
     - `battle_isTimeStopPauseOrBattleFlags0x20_800a0a4()` (false only when paused without time stop and without flag 0x20).

   Then decrement it and shake with its magnitude.
2. **Secondary channel.** Otherwise, if the secondary counter is nonzero, decrement it and shake.
3. **Neither.** Reset nextX/Y/Z to the camera position.

A shake calls `GetRNG1` twice, for x then y: `((rng & mask[m]) << 16) − bias[m]`. The table `byte_8030284` is {1,0x10000},{3,0x20000},{7,0x40000},{0xF,0x80000}.

---

## 6. Inputs and the link packet

### 6.1 Engine input model

Both cores apply **the same pair of packets** every tick: rx slot 0 (P0) and rx slot 1 (P1). A player's own input is not applied directly; it goes out in tx (`eStruct2036780`, 0x02036780) and comes back through the link.

**Latency is 4 ticks for both players.** Evidence:
- local held 0x10FC written at frame 314 is applied at 318, and its release at 320 is applied at 324;
- tx sequence 0x10 at frame 3 appears in rx at 7;
- tx sequence 0x88 at 1156 appears at 1160.

Both rx slots always carry the same sequence number on a given tick. The 4-deep queue lives in the SIO library (`sub_803DEB4` / `sub_8144250`, config `off_803DC78`); this was measured, not traced.

The engine's per-tick input is the applied (already delayed) packet pair. Only three packet fields reach the simulation:
- held keys (+2);
- status byte (+6);
- block-transfer index and word (+4/+8).

The port's model: each player's input is their buttons on that tick; the simulation carries them to the fight through a queue of `link_delay` ticks (4 here), with each player's status bit, and each custom-screen result arrives 50 + `link_delay` ticks after it is sent (custom-screen.md §0, §6).

### 6.2 Packet layout (0x10 bytes; tx and both rx slots)

| Off | Size | Written (tx) by | Content | Receiver (`sub_801FF18`) |
|---|---|---|---|---|
| +0x0 | u8 | `sub_803EE98` | `eJoypad.Held >> 8` | link layer only |
| +0x1 | u8 | `sub_803EE98` | sequence (`++eStruct200BC30[7]`) | none |
| +0x2 | u16 | `sub_801FF18` | `eJoypad.Held` (0x0200A270); the unused bits 0xFC00 are always set (`JOYPAD_DEFAULT`) | used only if `held & 0xFC00 ≠ 0`: `sub_800A0D6(p, held)` |
| +0x4 | s8 | `sub_801FF18` | block-transfer index; 0xFF = none | if ≥ 0: `recv_p[idx] = +8` (`recv_0` = `dword_203F4A0`, `recv_1` = `dword_203F5A0`, 64 dwords each) |
| +0x6 | u8 | `sub_801FF18` | sender's BS+0x11 (status bits) | → BS+0x14 (P0) / BS+0x15 (P1) |
| +0x8 | u32 | `sub_801FF18` | `dword_203CBE0[idx]` (unchanged when idle) | see +4 |
| +0xC | u16 | `sub_801FF18` | `sub_803F740(4)` (link diagnostic) | → `unk_2036120+0x2C` / `unk_20362F0+0x2C`; no simulation reader (the only reader, `sub_802E558`, reads it through a wrong register) |
| +0xE / +0xF | u8 | `sub_803EE98` | `sub_8144D18()` / `sub_8144D24()` | none |

Bytes +5 and +7 are 0.

`sub_801FF18(status)` (0x0801FF18; cable mode, because `eStruct200BC30[0] == 0`):

```
if status != 2: return                                   // status 1 ticks with stale records
for p in 0, 1:
    r = rx[p]
    if p == 1 and eStruct203F7D8[0] == 0:               // no-link path
        sub_800A0D6(1, 0)                                 // quirk: passes 0, not r.held
        (same side effects as below)
    elif r.held & 0xFC00:
        sub_800A0D6(p, r.held)
        BS[0x14+p] = r[6]
        (p ? unk_20362F0 : unk_2036120).w2C = r.w0C
        if (s8)r[4] >= 0: recv_p[r[4]] = r.d8
tx.held = eJoypad.Held; tx[6] = BS[0x11]; tx.w0C = sub_803F740(4)
if eStruct203F7D8[2] & 1:                               // block transfer running
    if sub_803EA2C(): return                             // link driver busy: hold this tick
    n = (s8)eStruct203F7D8[3] - 1; eStruct203F7D8[3] = n
    if n >= 0: tx[4] = n; tx.d8 = dword_203CBE0[n]; return
eStruct203F7D8[2] &= ~1; tx[4] = 0xFF
```

**Block transfers.**
- `sub_80200A4(n)` (0x080200A4) sets `eStruct203F7D8[3] = n` and `[2] |= 1`.
- The sender then sends `dword_203CBE0[n−1]`, …, `[0]`, one per tick. Element 0 is a magic number, so it arrives last.
- `sub_800B46C(magic)` is true once both `recv_0[0]` and `recv_1[0]` equal the magic. `sub_800B460` clears both.

| Magic | Dwords | Sender → receiver | Use |
|---|---|---|---|
| 0x12345678 | 0x3E | `sub_800B144` → `battle_copyStructsIncludingBattleStats_800b2d8` | init (§3.1.1) |
| 0x56789123 | 0x32 | `sub_800B3A2` → `sub_800B3D8` | chip hand (§7.3) |
| 0x1F2F3F4F | 4 | `sub_800B428` | end exchange (not PvP) |

### 6.3 Link status and the status byte

`eStruct203F7D8` (0x0203F7D8):
- +0 link active (1 after `sub_801FE24`);
- +1 status for this frame;
- +2 bit 0 = block transfer running;
- +3 remaining count (s8).

| Status | Meaning | Effect |
|---|---|---|
| 2 | normal | tick with fresh packets |
| 1 | no fresh packet | the tick runs but records are not updated, so a "pressed" edge would repeat |
| 8 | resync/stall | no tick (render only); init counts stall frames |
| 4 | link failure | error/terminate paths |

A lockstep engine is always at status 2. In the trace the status is 2 on every state-4 frame.

The status byte BS+0x11 uses only bit 2 (0x04), "in custom screen":
- set by `sub_800A9CA(4)` in `sub_8009338` (and `sub_80097CC`, another mode);
- cleared by `sub_800A9D6(4)` in `sub_8026BF4` (custom slide-out).

The received copies (BS+0x14/+0x15) feed `sub_80102AC` and `sub_80269D0`. Trace: set at 208 locally, 213 in BS+0x14/0x15; cleared 381 (P0) and 468 (P1).

### 6.4 Input records, `dword_2036820` (0x02036820 + 8·p)

| Off | Field |
|---|---|
| +0 | u16, always 0 |
| +2 | held (with 0xFC00) |
| +4 | pressed = `!old & new` |
| +6 | released = `old & !new` |

`sub_800A0D6(p, keys)` (0x0800A0D6): `old = held; held = keys; pressed = !old & keys; released = old & !keys`.

Records are zeroed:
- by `sub_800A0C6` (0x0800A0C6) once per battle (init sub-state 0);
- by the battle-start zero-fill.

As a result, the first packet after a reset gives pressed = 0xFC00 (frame 2), and handshake bytes such as 0x0B33 can pass the marker test during init (frames 3–5). The records are clean long before fighting.

Readers:
- `sub_8012DFC` (held);
- `sub_800A046` / `sub_800A07C` (START pressed);
- `sub_800A29A` (L|R, flag-0x40 only);
- result screens (`sub_802BF0C`, `sub_802C280`; not PvP).

The custom screen reads `eJoypad` directly.

### 6.5 Records → actors: `sub_8012DFC(p)` (0x08012DFC)

It is called only from fighting states 4 and 8, for p=0 then p=1, before `RunBattleObjectLogic`. It is **not** called during intro, custom, pause, result or judge, so actor inputs are frozen there. The first fighting tick after a custom screen computes edges against the held value from before the screen.

```
ai = actor(p).AIData        // sub_80103BC(p) = BS[0xD0 + 0x10p] (+0x58); loop bug: only entry 0 is tried
if battle_isBattleOver():   // tst form: BS+0x12==0 || BS+0x13==0 || BS+0x0B!=0
    ai.w22 = ai.w28 = ai.w24 = ai.w26 = 0; return
t = navistats(p)+0x2C
if t == 0x17 || t == 0x18: return            // inputs ignored (believed: berserk beast states)
new = rec[p].held; old = ai.w22
ai.w28 = old; ai.w22 = new; ai.w24 = !old & new; ai.w26 = old & !new
if !battle_isTimeStop(): ai.w30 = ai.w2A = ai.w2C = ai.w2E = 0
else: o2 = ai.w2A; ai.w30 = o2; ai.w2A = new; ai.w2C = !o2 & new; ai.w2E = o2 & !new
```

AIData input fields:
- +0x22 held, +0x24 pressed, +0x26 released, +0x28 previous held. The include-file names "JoypadUp"/"JoypadReleased" are misleading.
- +0x2A..+0x30: the same set, maintained only during time stop.

AIData starts zeroed, so the first call gives pressed = 0xFC00 | keys. This was checked tick by tick for frames 533–944.

The navi's own handler (`sub_8012E74` via `sub_80EA484`) also zeroes +0x22/+0x24/+0x26/+0x28 once the battle is over. That is why P0's AIData is zeroed on the KO tick (938) while P1's is zeroed at 939.

---

## 7. Custom gauge and chip-hand handoff

### 7.1 The custom gauge

There is **one gauge per battle, shared by both players**, in the HUD struct `eStruct2035280` (0x02035280):

| Field | Addr | Meaning |
|---|---|---|
| +0x20 u16 | 0x020352A0 | Gauge value, 0..0x4000 (`sub_801DFE4` reads it). Also written by `sub_801DF92` (0), `sub_801DFA2` (set, capped), `sub_801DFB8` (add), `sub_801DFD0` (subtract). |
| +0x22 u16 | 0x020352A2 | Fill per tick: 0x20 from `sub_801BE70`, then set by `sub_8014178` via `sub_801DF8C` |
| +0x40 u32 | 0x020352C0 | HUD update-task mask (bit 4 = fill `sub_801C470`, bit 15 = banner) |
| +0x44 u32 | 0x020352C4 | HUD draw-task mask (bit 4 = `sub_801C4E4` gauge draw) |

**Rate** is set once per battle by `sub_8014178` at the first intro tick:
`rate = byte_80141A0[3·stats1[0x08] + stats0[0x08]]`, where `statsP[k]` = `GetBattleNaviStatsByte(P, k)` and `byte_80141A0 = {0x20,0x40,0x10, 0x40,0x40,0x20, 0x10,0x20,0x10}`. Because the rate depends on both players' stats, it is identical on both cores.

NaviStats+0x08 is a speed class, read as 0 normal, 1 fast, 2 slow (uncertain). It was 0 for both players, so the rate was 0x20 and the gauge fills in 512 ticks. Custom1/Custom2 (NaviStats+0x0A, CustomLevel) do **not** change the fill; they change how many chips the local screen deals.

**Per tick, `sub_801C470`** (HUD task bit 4, run inside `sub_801BEE0`):

```
if paused || timestop || (flags & 2): return
gauge = (gauge + rate) as u16
if gauge >= 0x4000:
    gauge = 0x4000
    if !sub_800A97A():                // link && BS+7 >= 15 → never arms
        battle_setFlags(2)            // "gauge full"
        PlaySoundEffect(0x8F)
```

Task bit 4 is enabled and disabled as follows:
- **Enabled** by `sub_8027D78` → `sub_801DED0` (which also does `sub_801DF92`) at the local confirm tick and at custom end. It is not enabled if `sub_800A97A()`.
- **Disabled** by custom init (`sub_8026840`) and by the win/lose/draw init (mask 0xE4C53).

`sub_801DF92` resets the gauge to 0 and clears flags 0x12. `sub_800AF34` (chip-effect table entry 5 at 0x080EC3F0, likely FullCust) sets the gauge to 0x4000; flag 2 is then set on the next fill tick.

Trace:
- round 1 fills 593–994 (402 ticks, frozen at 0x3240);
- round 2 fills 1970–2402 (433 ticks, 0x3620);
- the gauge never fills and there is no mid-battle custom screen.

**Opening:** see §3.3.1. Either player's navi, while flag 2 is set, pressing L or R sets flag 0x10. The next `sub_80080D2` tick pauses and goes to machine state 0x20.

Note: the per-player structs `unk_2036120 + 0x1D0·p` (`sub_802E070`, +0x28 thresholds 0x2900/0x1500) belong to the flag-0x40 mode only and are not the PvP gauge.

### 7.2 State changes around the custom screen

| Moment | Changes |
|---|---|
| Open (mid-battle) | `PauseBattle` (panels, timers, gauge, `sub_802CDFE` stop; only flag-0x04 objects run); transformations reverted (`sub_8008452`); sequencer reset (`sub_8008492`) |
| Custom init | BS+0x11 \|= 4 (transmitted); BS+7 += 1; gauge = 0, flags 2 and 0x10 cleared; gauge HUD tasks off |
| Slide-in complete (+10 ticks; not on the first screen) | `sub_8013FD0(0/1)`: custom-HP bug damage |
| Local confirm | local block built; BS+0x11 bit 2 cleared (slide-out tick 1); chip exchange started; gauge task re-enabled (still paused, so no fill) |
| Both exchanges complete | `sub_800B3D8` installs both blocks and both navi stats |
| Close | actor AIData+0x0F = 1 for both navis; mode 0xC; machine zeroed; transformations applied (state 0); turn banner (state 4); unpause on the first state-8 tick |

### 7.3 Chip-hand exchange

**Send.** At the first tick of UI state 0x14, `sub_800B3A2` fills `dword_203CBE0`:

| Offset | Content |
|---|---|
| [0] | magic 0x56789123 |
| +0x04 | local block (0x50 bytes from `byte_20366C0`) |
| +0x54 | local battle navi stats (0x64 bytes, `GetBattleNaviStatsAddr(BS+0xD)`) |
| +0xB8 | local transform record (0x10 bytes from `byte_203CED0`) |

It then calls `sub_80200A4(0x32)`. Trace: tx 387–436, slot 0 rx 391–440, slot 1 rx 478–527.

**Install.** `sub_800B3D8` (on completion):
- `byte_203F4A4` → 0x020349C0 unless its byte 0 is 0xFF;
- (link) `byte_203F5A4` → 0x02034A10 unless its byte 0 is 0xFF;
- `byte_203F4F4` → `eBattleNaviStats0`;
- (link) `byte_203F5F4` → `eBattleNaviStats1`.

The transform records stay in the receive buffers at `byte_203F558` / `byte_203F658`, where fighting state 0 reads them. Record layout:
- +0 requested form (0xFF none; P0 sent 0x0C in round 2);
- +4 (0xFF none);
- +8 navi object pointer.

A block with byte 0 = 0xFF means "no chips chosen" and is **not** installed, so the previous block and its index remain.

**Chip block layout**, 0x020349C0 + 0x50·alliance (`sub_8010018(alliance)`):

| Off | Type | Meaning |
|---|---|---|
| +0x00 | u8 | At confirm: 0 = chips chosen, 0xFF = none. During the fight: index of the next chip, incremented by `sub_800FC7C` on use (0→1→2→3 at 638, 779, 920). |
| +0x01 | u8 | flag-0x40 mode only (0) |
| +0x02 | u16[6] | chip ids (9-bit), 0xFFFF-terminated (≤ 5 chips) |
| +0x0E | u16[6] | base damage, from `sub_80109A4(id, localSide)`; refreshed by `chip_800AEE8` for flagged chips |
| +0x1A | u16[6] | Attack+ bonus (`sub_8029224`: chips 0xB8, 0xB9, 0xC0, 0xC1, 0xC3 add to the previous chip) |
| +0x26 | u16[6] | extra bonus (0 at confirm; zeroed by `sub_8014216` / `sub_80144CA`) |
| +0x32 | u16[6] | id \| code<<9 (flagged chips mapped to 0x3785), 0xFFFF-terminated |
| +0x3E | u8[6] | BS+7 − 1 (which custom screen the chip was picked in) |
| +0x44 | u8[6] | active-chip source byte (uncertain) |
| +0x4A–0x4F | — | unused |

Blocks are initialized by `sub_800A954` at battle start: zero, then +2..+0xD = 0xFF. In the trace P1 never chose chips, so its block stayed empty. Chip-use getters (`sub_800ED90` / `sub_800EDD0`) sum +0x0E, +0x1A, `sub_800EF34(...)` and +0x26; chip semantics are in chips.md.

### 7.4 Consumption during the fight

- **`sub_800FDC0`** (every tick, after objects). For each of the 8 pointers at BS+0x80 where the actor exists, has AIData and AIData+0 == 2, `sub_800FDEA` sets:
  - `ChipsHeld` (obj+0x1A) = count of ids from block[0] up to the terminator;
  - `Chip` (obj+0x2A) = ids[block[0]] (0xFFFF if none).

  Confirmed: P0 held 4 / chip 0x11 on 527 (the install tick), then 3, 2, 1 at 638, 779, 920.
- **Chip use:** `sub_800FC7C` in navi object logic (`sub_80EA484` → … → `sub_80F0354`) does block[0] += 1 when block[0] < 5 and ids[block[0]] ≠ 0xFFFF. `sub_80108FC` skips special ids 0x190..0x19A at the head.
- **`chip_800AEE8`** (every tick, paused or not). For alliance 0 then 1, with i = block[0]: if `ChipData(ids[i])+9 & 0x80`, set damage[i] = `sub_80109A4(id, alliance)`.
  - `sub_80109A4` returns 0 for 0xFFFF.
  - Otherwise it takes ChipData+0x1A: values < 1000 are returned as-is; values ≥ 1000 call `off_80109DC[dmg−1000]`. In link, 1000 = opponent's current HP capped at 500; 1001..1018 read `byte_203EB00[0x28·alliance + 2·(dmg−1001)]`.
  - For id 0xFFFF the ROM read lands at 0x082E1D7C, where byte +9 is 0x30, so nothing is written. Treat 0xFFFF as "flag clear".

---

## 8. Battle-wide state

### 8.1 BattleState (0x02034880, 0xF0 bytes)

Zeroed at every battle start except where noted.

| Off | Size | Meaning | Main writers / readers |
|---|---|---|---|
| 0x00–0x03 | u8×4 | top state, mode state, sub-state, init flag | §2 |
| 0x04 / 0x05 | u8 | counted actors per alliance | `sub_8007778`, `sub_800A104`; `sub_800A152` |
| 0x06 | u8 | panel layout index (settings+0) | `sub_800A2F8`; `sub_800BF88` |
| 0x07 | u8 | custom screens this battle (turn) | `sub_8026840`; `sub_800A97A`, chip block +0x3E |
| 0x08 / 0x09 | u8 | name-ID count per alliance | `sub_80077D2` |
| 0x0A | u8 | battle/set running (return value) | battle start = 1; `loc_8007E38` = 0 |
| 0x0B | u8 | time up | `sub_800AB7C`; `battle_isBattleOver`, `sub_800A152` |
| 0x0D | u8 | local side (0 = P0/master) | `sub_80079A8`; perspective everywhere (`battle_networkInvert` = x ^ BS+0xD) |
| 0x0E | u8 | timer mod 20 | §4 #14 |
| 0x0F | u8 | battle mode copy (settings+3) | `sub_800A2F8` |
| 0x10 | u8 | winning alliance | result states |
| 0x11 | u8 | local status bits (bit 2 = in custom) | `sub_800A9CA` / `sub_800A9D6`; transmitted |
| 0x12 / 0x13 | u8 | alive navis per alliance | spawn; `sub_800A11C`; `battle_isBattleOver` |
| 0x14 / 0x15 | u8 | received status P0 / P1 | `sub_801FF18`; `sub_80102AC`, `sub_80269D0` |
| 0x16 | u8 | timer mod 180 | §4 #14 |
| 0x17 | u8 | regular chip present | `sub_800A3E4` |
| 0x18 / 0x19 | u8 | local wins / losses (**persist** across rounds) | §3.6 |
| 0x1A | u8 | round number (**persists**) | `sub_80091F0` |
| 0x1B | u8 | max combo (init 1, **persists**) | `sub_800AE0C` |
| 0x1C / 0x1D | u8 | combo count / window | `sub_800AE44`, `sub_800AE0C` |
| 0x1E | u8 | busting level | `sub_800AF84` |
| 0x1F | u8 | result code (low nibble) | `setTwoStructs_800A840` |
| 0x20 | u16 | init stall counter; later the low-HP music latch | `sub_8007850`; `sub_8009158` |
| 0x28 | u16 | small timer (banner delays, top-8 delay) | `sub_80092C0`, `sub_8009314`, `sub_8007BD0` |
| 0x32 | u16 | battle flags (§8.2) | |
| 0x34 | u16 | local navi HP at exit | `sub_800FAE0` |
| 0x38 | u16 | tick-based gauge for other modes (unused in PvP) | `sub_800A6D8` (unreferenced) |
| 0x3A | u16 | escape flag | `sub_800AAE8` (not PvP) |
| 0x3C | u32 | BattleSettings pointer (PvP 0x0200AF60) | battle start |
| 0x40 | u32 | battle time (ticks fighting, cap 0x8C9F) | `sub_800A6A6`; busting level, some object colour flashes |
| 0x44 / 0x45 | u8 | tag chips present / tag insertion index | folder |
| 0x4C–0x5B | u16 | NameIDs (alliance 0 at +0x4C, alliance 1 at +0x54) | `sub_80077D2` |
| 0x5C | u8 | intro progress bits (init 0x0C; +0x10, +0x01, +0x02) | intro controller via `sub_8001382`; `sub_800139A` |
| 0x60 | u32 | frames in this battle | `battle_8007800` |
| 0x64 | u32 | ticks | §4 #17 |
| 0x80–0x9F | ptr[8] | alive actor pointers, 4 per alliance (+0x10·alliance) | `sub_8007778`, `sub_800A11C`; `sub_800FDC0` |
| 0xA0–0xBF | ptr[8] | field-object registry: per side (+0xC·side) two class-0 obstacle slots and one class-1 slot, then two stage-object slots at +0xB8/+0xBC (field-objects.md §2) | `setFieldBattleObject_800F614`, `sub_800F656`, `sub_80EFD74` |
| 0xC0–0xCF | — | not observed to change in PvP | |
| 0xD0–0xEF | ptr[8] | spawn-time copy of 0x80–0x9F | `sub_8007368`; `sub_80103BC` (actor by player), `sub_800A7A6` |

In the machgun trace these bytes never change: +0x0C, +0x21–0x27, +0x2A–0x31, +0x36/0x37, +0x39, +0x3B, +0x46–0x4B, +0x5D–0x5F, +0x68–0x7F.

### 8.2 Battle flags (BS+0x32)

`battle_setFlags` 0x0800A2D8, `battle_clearFlags` 0x0800A2E4, `battle_getFlags` 0x0800A2F0.

| Bit | Meaning | Set by | Cleared by | Read by |
|---|---|---|---|---|
| 0x01 | fighting has started | `sub_80080D2` every fighting tick (and the other modes' equivalents) | battle start only | `sub_800A6A6` (battle time) |
| 0x02 | custom gauge full | `sub_801C470` (`sub_801C4AE` in other modes) | `sub_801DF92` | `sub_8012FC8` (L/R), `sub_800A1D0`, `sub_801C470` |
| 0x04 | **time stop** | `object_timefreezeBegin` (0x0800B916, if `sub_800B8D8`), `sub_802DACC`, `sub_8015766`, `sub_80E8EA0` | `object_timefreezeEnd` (0x0800BD34), `sub_802DC66`, `sub_8015766`, `sub_80E8E92` | `battle_isTimeStop` (≈28k calls) |
| 0x08 | special (with BS+0x0B = 1) | `sub_80D8DEE` | — | (not PvP) |
| 0x10 | custom-screen request | `sub_8012FC8` | `sub_801DF92` | `sub_800A1D0` |
| 0x20 | (unknown) | no setter found | `sub_8014CC0`, `sub_8014F04`, `sub_8015128`, and two more | `battle_isTimeStopPauseOrBattleFlags0x20_800a0a4` |
| 0x40 | alternate "per-player gauge / link navi" mode | `sub_802E112` (not in PvP) | — | `TestBattleFlag_0x40` / `sub_800A8F8` (fighting branches, `sub_802DE5C`, `sub_802E156`) |

In the machgun match the word was 0x0000, then 0x0001 from the first fighting tick.

### 8.3 Pause and time stop

**Pause** is GameState+0x0A at 0x02001B8A (`PauseBattle` 0x0800A028, `UnpauseBattle` 0x0800A032, `battle_isPaused` 0x0800A03C).
- Set: intro (`sub_80091F0`), START pause, custom-screen request (`sub_80080D2`), comm error / terminate.
- Cleared: battle start, every fighting tick (`sub_80080D2`), exit (`loc_8007E38`).

So pause is on from the first intro tick through custom screens and the start banner, and off from the first fighting tick through the result and end states.

**Time stop** is battle flag 0x04, set and cleared by object code (time-freeze chips and certain navi actions).

| Component | Paused | Time stop |
|---|---|---|
| `RunBattleObjectLogic` | only objects with flag 0x04 run | only objects with flag 0x10 run (both gates apply when both hold) |
| `sub_800BFC4` panels | skipped | skipped |
| BS+0x0E / +0x16 timers | skipped | skipped |
| `sub_802CDFE` | skipped | runs |
| `sub_80102AC` HP drain, `sub_800FDC0`, `chip_800AEE8`, `sub_802CEC8`, `sub_801BEE0` (tasks gate themselves), camera | run | run |
| gauge fill `sub_801C470` | no | no |
| battle time `sub_800A6A6`, turn timer `sub_800AB7C` | no | no |
| `sub_8012DFC` inputs | not called outside states 4/8 | time-stop mirror fields maintained |
| `sub_800A152` result probe | — | returns 0 (no KO resolution during time stop) |
| START / custom checks | — | blocked (`sub_800A046` returns 0xFF; `sub_800A1D0` false) |

Object flags in the trace: navis 0x17/0x15 (have 0x04 and 0x10); T4 helpers 0x1D → 0x15.

### 8.4 Object update loop

`RunBattleObjectLogic` (0x080031AC):
1. `object_Clear3RAMBytes_800371A` zeroes the three per-type live counts: `byte_2036778` (T1), `dword_203CA7C` (T3), `byte_2036830` (T4).
2. Walk the linked list from `eBattleObjectsLinkedListStart` (0x02009380; node + 0x10 = object) to the sentinel 0x02009AB0. For each object:
   1. Store the node in `eUnkBattleObjectLinkedList` (0x0200AF70).
   2. If paused and !(flags & 0x04), skip.
   3. If time stop and !(flags & 0x10), skip.
   4. Otherwise call `JumptableTable[type & 0xF][index]` (T1 0x08003C9C, T3 0x08003EC4, T4 0x080042C8).
   5. **Always** call `object_800372A`, which appends the object to its type's live list (`dword_2039A10` / `dword_203A010` / `byte_203F750`, count ++).
3. Store 0 in 0x0200AF70.

Update order is list order. Spawning and freeing rules belong to the object spec.

### 8.5 Other battle-wide globals

| Address | What | Lifetime |
|---|---|---|
| 0x0203CA70 | fighting machine (§2.3) | zeroed per mode-0xC entry |
| 0x02035280 | HUD struct: gauge (+0x20/+0x22), task masks (+0x40/+0x44) | `sub_801BE70` per battle |
| 0x02036840 | banner state (`byte_2036840`) | `sub_801E792` |
| 0x020349C0 / 0x02034A10 | chip blocks P0 / P1 | battle start |
| 0x0203CE00 / 0x0203CE64 | battle navi stats P0 / P1 (`GetBattleNaviStatsAddr(i)` = 0x0203CE00 + 0x64·i, absolute alliance) | init exchange; replaced by every chip exchange |
| 0x0203CDB0 | `eBattleFolder` (local) | init |
| 0x02036820 | input records | §6.4 |
| 0x0203F7D8, 0x020399F0/0x02039A00, 0x02036780, 0x0203F4A0/0x0203F5A0, 0x0203CBE0 | link state, rx, tx, receive and send block buffers | battle start |
| 0x0203CFB0 / 0x0203CFBC | damage-carry records | §4 #15 |
| 0x0203CB04 | panel 140-tick timer | `sub_800BF88` = 0x8C |
| 0x02036720 | linked-object registry (2×0x10) | §4 #12 |
| 0x020367E0 | mega/giga use counters | `sub_801BE70` |
| 0x0203EB00 | time-record tables (variable damage) | init exchange |
| 0x0203CA50 | settings for later rounds | init exchange, persists |
| 0x02001120 / 0x020013F0 | RNG1 / RNG2 | §5 |
| 0x02001B8A | pause byte | §8.3 |
| 0x02009980 | camera (shake counters) | §5.3 |

---

## 9. Presentation vs simulation checklist

### 9.1 Must be modelled (possibly as counters)

- **Packet application** `sub_801FF18`: inputs, status bytes, block transfers (§6).
- **All flow state machines:** top, mode, fighting machine, custom-screen outcome, result states, judge (§2–§3).
- **Banner lifetime** (HUD task bit 15). It gates the intro banner, the fight-start banner, and the win/lose (together with the 0x66 timer) and draw states. Model it as the 59-tick rule of §3.2; the drawing itself can be skipped.
- **Screen-fade waits:** the intro controller's 17-tick fade (drives BS+0x5C) and mode 0x14's 16-tick fade (§3.7).
- **Custom gauge** `sub_801C470` (HUD task bit 4) and battle flags 2/0x10.
- **`RunBattleObjectLogic`**, including the live lists.
- Everything marked Sim in §4: panels, `sub_800FDC0`, `sub_802CEC8`, `chip_800AEE8`, BS+0x0E/+0x16, `sub_802CDFE`, `sub_80102AC`, and BS+0x40/+0x64.
- **Combo counter** `sub_800AE0C` (BattleState bytes).
- **Turn timer** from turn 15, and the **damage judge** including its 59 RNG2 draws.
- **Init:** RNG2 adoption, settings, navi stats, spawn with its 1 RNG2 draw, and the one extra panel update in `sub_8007A0C`.
- **`sub_8013FD0`** custom-HP-bug damage at custom slide-in.
- **`sub_800B090`** (never fires for legal chips).

### 9.2 Safe to skip

| Item | Justification |
|---|---|
| `sub_80027B4` / `sub_800286C` | Write only `dword_200F350` = 1 / `dword_200F340` = 0 (sprite allocator bookkeeping). |
| `sub_8003E18`, `sub_8004218`, `sub_8004510` | Walk the live lists and call `sub_30061E8` (screen position), `sub_3006028` (tile upload via `QueueEightWordAlignedGFXTransfer`) and `sub_3006440` (OAM). They write the sprite sub-structs at object+0x90 (+3 bit 0x10, +8, +0x15, +0x24), the GFX queue and OAM buffers. Believed not read by object logic (§11). |
| `sub_8003C70`, `sub_80046F8`, `sub_80049B0` | `sub_80028C0(0/2/5)`: overworld sprite passes. |
| `sub_800C5E0` | Panel graphics: `sub_800C192` (panel tile animation) and BG tile writes (`sub_800C01C`/`sub_800C0BA`/`sub_800C100`/`sub_800C138` → `iCopyBackgroundTiles`). It clears the panel redraw latches +0x01 and +0x0D after consuming them (§11). |
| `sub_801BF64` | HUD draw tasks: tile/palette uploads and OAM for HP boxes, gauge, chip icons, emotion window, banner, timer. |
| `sub_802E156` | Copies a per-player gauge into the HUD only under flag 0x40. |
| `sub_8009FCC` | Draws the deferred sprite list `dword_3002180`. |
| `sub_803C59C(0xE0,0x90)` | Link-quality icon. |
| Camera `sub_802FFF4` | Writes only `eCamera` and BG scroll. It **does** advance RNG1 while shaking; keep that if RNG1 must match. |
| HUD update tasks other than bits 4 and 15 | `sub_801C002` (chip icons), `sub_801C168` (rolling HP digits, `byte_203EB50`), `sub_801C840` (HP box, low-HP sound), `sub_801C984`, `sub_801CA28`, `sub_801CADC`/`sub_801CC94` (mugshot; can draw RNG1). They write only `eStruct2035280`, `byte_203EB50` and the GFX queue. |
| Sound and music, including the low-HP BGM switch | m4a only. The BS+0x20 latch is a BattleState byte; keep it if BattleState must match byte-for-byte. |
| Custom-screen UI internals (cursor, camera slide, local folder bookkeeping, RNG1 there) | Local. Its results reach the simulation only as the transmitted status bit and the block transfer. |
| Link teardown in top 8 (19 frames), link-quality and diagnostics fields | No simulation state. |

---

## 10. Engine interface summary

**Starting conditions for a round.** These can be taken from the post-init snapshot at the first tick, or rebuilt:
- BattleSettings (layout index, column alliances, actor list, effects);
- both navi-stat blocks as exchanged;
- RNG2 (P0's pre-draw value);
- the local side (for perspective-dependent intro object state and local-relative BS+0x18/0x19);
- panel timer 0x8B;
- chip blocks empty;
- round number and set score.

**Per tick, for each player:**
- held keys (with 0xFC00);
- status byte (bit 2 = in custom screen);
- optional block-transfer (index, dword).

These are already delayed 4 ticks; a player's chip hand, stats and transform record arrive as the 0x32-dword transfer ending with magic 0x56789123.

**Engine state** includes, beyond objects and panels:
- the BattleState fields of §8.1;
- the fighting machine;
- the HUD gauge and banner counters;
- the input records and actor AIData input fields;
- the chip blocks;
- the damage-carry records and the linked-object registry;
- RNG2, and optionally per-core RNG1.

---

## 11. Uncertainties and unverified points

1. **Mid-battle custom screen** (flag 0x10 → states 0x20/0x24 → mode 8) is static analysis only; the trace's gauge never fills. The tick counts T2–T6 in §3.3.1, especially with a cross/beast revert, are unverified.
2. **Turn timer (turn ≥ 15), damage judge and draw state** are static only. The judge's tick timeline was derived by hand. The draw banner (0x1C) and judge banners were never measured, and the draw state's length depends entirely on its banner. The judge's write of the loser to 0x0203CA80 overlaps another array; its reader is unknown.
3. **Banner lengths:** the 59-tick rule was measured for the round banner (0x30), the turn banner (0xC) and the link win banner. Held banner types 2/4 are never removed by the task itself; which ids have those types was not enumerated.
4. **Link layer:** the 4-tick latency was measured, not traced. Status 1/8 semantics are inferred. The engine should assume status 2 on every tick.
5. **NaviStats+0x08** (gauge speed class) and **+0x2C values 0x17/0x18** (inputs ignored and auto custom-open) have no confirmed meaning.
6. **Render-pass side effects:** `sub_3006028`/`sub_30061E8` write sprite sub-struct bytes, and `sub_800C5E0` clears panel latches +0x01/+0x0D. These are assumed never read by simulation code; this was not proven exhaustively.
7. **Intro length** (52 ticks) is driven by object code: the remote navi's mosaic fade and the intro controller's 17-tick fade wait. It was measured; no closed formula was derived. The intro object state differs between the two cores.
8. **`sub_801CC94`**'s RNG1 draw condition was never exercised; its exact timing is unmodelled.
9. **Settings +1 and +7, and effects bits 0x200/0x800:** no battle-code reader was found.
10. **`sub_80AA88C`** writes only when the drop table has a non-0xFFFF entry. This was checked only for NameID 0x1A0 (the PvP navi); other navis' IDs were not checked.
11. **Chip block +0x44 semantics**, and "choosing no chips keeps the previous remaining hand": the latter follows from the 0xFF skip in `sub_800B3D8` and was only observed with an empty previous hand.
12. **Battle flag 0x20** has no setter found; flag 0x08 is set only by a special object. BS+0xA0..0xCF is unused as far as observed.
13. **`sub_80103BC` loop bug:** only BS+0xD0 + 0x10·p is ever returned. A missing actor would make `sub_8012DFC` dereference NULL; this never happens in PvP.
14. **Only core 0 was traced.** Claims about the other core rest on the code's symmetry. This includes `sub_8027D78` running at each core's own local confirm tick, which is believed idempotent.
