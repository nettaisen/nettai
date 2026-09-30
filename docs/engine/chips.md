# Battle chips: data, hand handling, use, attack framework

Engine spec for the clean-room Rust port of the MMBN6 US Falzar (BR6E) battle engine, PvP netbattle first.
It covers chip data, the per-player battle hand ("chip block"), how a chip gets from the custom screen into
an attack action, the generic attack framework (action handlers, spawners, collision hand-off, dimming),
the MegaBuster as the non-chip counterpart, and a frame-exact worked example (GunDelS3) from a real match.

- The full per-chip inventory, generated from the ROM, is in [`chip-table.md`](chip-table.md).
- Object kinds seen in PvP traces are in [`object-kinds-pvp.md`](object-kinds-pvp.md).

**Sources.**
- The original game's code (US Falzar, sha1 `0676ecd4…`), read statically and run under emulation.
- The machgun trace: a recorded netbattle (Falzar vs Falzar, two rounds) with the original's per-frame state.
  Memory watches on the same battle were used to confirm call chains and memory values.
  "machgun" is the name of the netbattle room, not the chip; no MachGun is used.
- Every claim cites a function or table address. Unverified inferences are marked **[unverified]**.
  Claims checked against the trace are marked **[trace]**.

**Notation.**

| Symbol | Meaning |
|---|---|
| `obj` | The acting battle object (r5 in the asm). |
| `ai` | `obj.AIDataPtr` (obj+0x58, 0x100 bytes). |
| `av` | `ai+0xA0`, `AIAttackVars` (r7 in handlers). |
| `ns` | The player's NaviStats in battle, `eBattleNaviStats0 + 0x64*alliance` (0x0203CE00), read with `GetBattleNaviStatsByte_AllianceFromBattleObject`. |
| `blk` | The chip block of `obj.Alliance` (§2.1). |
| `cd(id)` | The ROM chip record of `id` (§1). |
| alliance | 0 = left/red, 1 = right/blue. Every per-player chip structure is indexed by alliance, never by "local/remote". |

Key bits are GBA KEYINPUT: A=1, B=2, SELECT=4, RIGHT=0x10, LEFT=0x20, R=0x100, L=0x200.

---

## 0. Pipeline at a glance

```
custom screen (local)  sub_8029110 builds 0x50-byte hand ─┐
                       sub_800B3A2 packs hand+NaviStats   │ 50 link words (sub_801FF18)
remote hand ─────────────────────────────────────────────┤
both magic words in:   sub_800B3D8 copies hand_p -> block_p (0x020349C0 + 0x50*p)
every frame:           sub_800FDC0 -> obj.ChipsHeld/obj.Chip ; chip_800AEE8 refreshes variable damage
A pressed:             sub_8012FC8 sets ai+0x44 |= 4
idle action 8:         sub_80EA734 -> sub_80F0354 -> sub_800FB54 -> sub_80127C0 (fills av from block + ROM)
                       -> object_setAttack2(cd.action) ; sub_800FC7C (blk.cur++)       [same frame]
next frame on:         sub_801B9E6 -> JumpTable80EAC60[action-0x10] (phase machine on av+0)
                       -> spawners (T3 hitboxes/shots, T1/T4 visuals, dimming controllers)
                       -> object_exitAttackState (CurAction=8, lockout ai+0x19 = cd+0x14)
```

---

## 1. Chip data table

### 1.1 Location and access

| Item | Value | Source |
|---|---|---|
| Table | `ChipDataArr_8021DA8` at **0x08021DA8** (ROM offset 0x021DA8). Byte-identical to `data/ChipDataArr.s`. | |
| Record size | **0x2C** | `getChip8021DA8` |
| Count | **411** records, ids 0x000..0x19A, followed by `.word 0` | `cmp #0x19B` in `sub_800AFBA` (0x0800AFBA) and `sub_800B022` (0x0800B022) |
| Accessor | `getChip8021DA8` (0x08021AA4) returns `0x08021DA8 + 0x2C*id`. **No bounds check.** | |
| "No chip" | id 0xFFFF (hand terminator). `sub_80109A4` returns 0 for it. It is also passed unguarded to `getChip8021DA8` by `chip_800AEE8`; see §2.5 for that quirk. | |
| Packed chip | `u16 = id \| code<<9`. id = low 9 bits, code = bits 9..15. Used in folders, the raw selection and the link packet. | `sub_800AFBA`, `sub_800B022`, `sub_800A570` |
| Error chip | **0x185** (nameless, action 0x1C, damage 0). Replaces illegal selections. | `sub_800B022`, `sub_800B090` |
| Names (UI only) | `sub_8027D10`: id ≤ 0xFF uses `TextScriptChipNames0[id]`, otherwise `TextScriptChipNames1[id & 0xFF]`. Index by `def_text_script` entry: archive 0 has empty entries from 0xCB on. | |

Rust: `static CHIPS: [ChipData; 411]`, loaded little-endian from ROM 0x021DA8. Treat id > 0x19A as a bug.
The one exception is the empty-hand read in `chip_800AEE8` (§2.5).

### 1.2 Record layout (0x2C bytes)

"Battle" readers run during fights. Every other reader listed is menu or folder code.

| Off | Type | Field | Meaning | Battle readers |
|---|---|---|---|---|
| +0x00 | u8[4] | `codes` | 0..25 = A..Z, 0x1A = `*`, 0xFF = unused slot. Code `c` is legal iff `c == 0xFF` or `c ∈ codes` (`sub_8006EE8`, 0x08006EE8). | `sub_800B022` |
| +0x04 | u8 | `elem` | **Attack element**: 0 Null, 1 Fire, 2 Aqua, 3 Elec, 4 Wood. Becomes the low nibble of the attack's element byte. | `sub_80126E4` (0x080126E4), `sub_80127C0`, `sub_8012AFA`, `sub_8012B4E`, `sub_80F0608` |
| +0x05 | u8 | `rarity` | Stars − 1. | menus only |
| +0x06 | u8 | `family` | Icon family: 0 Fire, 1 Aqua, 2 Elec, 3 Wood, 4 Plus, 5 Sword, 6 Cursor, 7 Obstacle/summon, 8 Wind, 9 Break, 0xA Null, 0xB PA, 0xC misc (0x160..0x171). Maps to **secondary-element bits** via `byte_80129E4` = {5:0x80, 6:0x40, 8:0x20, 9:0x10, else 0}. Also keys cross/beast boosts. | `sub_80126E4`, `sub_800EF34`, `sub_8012ABC`/`BA2`/`BE8`/`C4A`, `sub_80128FC`, `sub_8013236`, `sub_800FB54` |
| +0x07 | u8 | `class` | 0 Standard, 1 Mega, 2 Giga, 3 special (not a folder chip), 4 Program Advance. | `sub_800B022` (mega/giga limits), `sub_80106C0` (Beat), `sub_800A570` (folder shuffle), `sub_800EE98` |
| +0x08 | u8 | `mb` | Folder MB cost. | menus only |
| +0x09 | u8 | `flags` | See §1.3. | many |
| +0x0A | u8 | `hit_param` | High half of the attack damage word: `av.u32[8] = (+0x0A<<16) \| damage`. It travels to attack object +0x2E, then CollisionData+0x07: the counter/"stagger" byte (§3.5). Values: Cannon 30, Vulcan 10, SuprVulc 4, AirShot/TankCan 20, navi chips 138. | `sub_80126E4`, `sub_802D4F0` |
| +0x0B | u8 | `action` | **Attack action**, becomes `obj.CurAction` via `object_setAttack2` (§1.6). | `sub_80126E4`/`sub_80127C0` return it |
| +0x0C | u8 | `subtype` | Variant within the action, copied to `av[3]`. For example Cannon/HiCannon/M-Cannon = 0/1/2, GunDelS1/2/3/EX = 0/1/2/3. It indexes `off_802CCB4` for action 0x15 and `off_802CD5C` for action 0x1B. | `sub_80126E4` |
| +0x0D | u8 | ? | **No reader found.** | – |
| +0x0E | u8 | ? | 0/4/5/6. **No reader found.** | – |
| +0x0F | u8 | `beast_lockon` | Copied to `av[0x1D]`, but only in Beast Out forms or for a chip-gate chip. When set, dispatch goes through the Beast wrapper `sub_80EAD9C` (§2.11). | `sub_800FB54` |
| +0x10 | u32 | `params` | 4 action-specific bytes, copied to `av.u32[0xC]` and passed as r4 to spawners. Examples: Vulcan shot row 0x0C, AirShot 4, TankCan 0x100. | `sub_80126E4` |
| +0x14 | u8 | `lockout` | Post-chip lockout in frames. Copied to `av[5]`, then to `ai[0x19]` at attack end (§2.8). Most chips 0. Seeds and Lance 10; FireHit, Boomer, GolmHit, BusterUp, Atk+10 and others 20; Recov and TimeBom 30; AirHocky 50. | `sub_80126E4` |
| +0x15 | u8 | `lib_index` | Library sub-index. | menus only |
| +0x16 | u8 | `flags2` | See §1.3. | `sub_800EE98`, `sub_8010740` |
| +0x17 | u8 | `lockon_mode` | Beast Out lock-on panel selector: `sub_80EAE28` passes it to `ho_8026554` (0x08026554), which indexes `jt_8026584`. | `sub_80EAE28` |
| +0x18 | u16 | `sort_key` | Alphabetical sort. | menus only |
| +0x1A | u16 | `damage` | Base damage. **≥ 1000 is a formula index** `damage − 1000` into `off_80109DC` (§1.5). | `sub_80109A4` (0x080109A4) and its formulas |
| +0x1C | u16 | `library_no` | Library number (Cannon 1, …). | menus only |
| +0x1E | u8 | `slotin_max` | Per-battle use limit through the Battle-Chip-Gate slot-in source (`sub_802E830`). **Unreachable in PvP**; see §2.6. | `sub_802E830` |
| +0x1F | u8 | `dark_subst` | 0xFF, or 0..4 for dark chips 0x11E..0x122. An index into `off_8010D84` = {0x47 Sword, 0x1E Thunder, 0x9A Recov10, 0xB1 Invisibl, 0xC0 Atk+10}, used when the player has no bugfrag (`sub_8010D58`, §2.6.4). | `sub_80127C0` |
| +0x20 / +0x24 / +0x28 | ptr | gfx | Icon, picture and palette. | UI only |

Bytes +0x11..+0x13, +0x1B and +0x1D are only ever read as part of the wider fields around them.

### 1.3 Flag bytes

**+0x09 `flags`:**

| Bit | Meaning | Evidence |
|---|---|---|
| 0x01 | **Dimming chip.** Can cut in (be used during the other side's dimming). Excluded from every "boostable" test. | `sub_8017AB4` (0x08017AB4), `sub_800FC30`, `sub_80F0354` (no opponent banner for dimming chips) |
| 0x02 | **Has damage.** The damage is shown in the name banner, and the chip can receive Atk+, cross/beast boosts and double damage. The boost tests generally require `(flags & 3) == 2`. | `sub_8029224` (Atk+ fold), `sub_8012A38`, `sub_800EF34`, `object_drawChipName` |
| 0x04 | **Navi chip.** Receives Navi+20. On use, `byte_203EAE0[0x10*alliance+6]` += 1 (saturating). | `sub_8029224`, `sub_80127C0` → `sub_800AB46` |
| 0x08, 0x40 | Library membership (standard / any). | menus |
| 0x10 | Damage displayed as variable. | menus |
| 0x20 | Unused by any chip. | menus |
| 0x80 | **Damage recomputed every frame** while this is the current hand chip. Set on Muramasa 0x55, NumbrBl 0x8A, and 0x190..0x19A. | `chip_800AEE8` (0x0800AEE8) |

Common values:

| Value | Chips |
|---|---|
| 0x4A | Standard attack chip |
| 0x48 | Standard non-damage chip (GunDelS1–3, Atk+10) |
| 0x49 / 0x4B | Standard dimming chip without / with damage |
| 0x47 | Navi chip |
| 0x42 / 0x43 | Mega / PA |
| 0xC2 | Cross/beast charge chip |

**+0x16 `flags2`:**

| Bit | Meaning |
|---|---|
| 0x80 | No slot-in gauge cost (`sub_800EE98`, 0x0800EE98; slot-in only). |
| 0x02 | Cancelled by the opponent's **Rush** support (`sub_8010740`, 0x08010740). Set on Invisibl and WhiCapsl. See §2.10. |
| 0x01, 0x10, 0x20, 0x40 | Menu classification only. |

### 1.4 Fields copied into `AIAttackVars` (`sub_80126E4`, 0x080126E4)

`sub_80126E4` has two entries. The main entry sets r4=1 and counts the use; `loc_80126EA` sets r4=0 and is used by
about 47 charge-shot helpers. In both, r7 = av.

```
av.u16[0x14] = id
av.u32[0x0C] = cd.params                                   // +0x10..+0x13
av.u32[0x08] = (cd.hit_param << 16) + sub_80109A4(id, alliance)
av[0x05]     = cd.lockout                                  // +0x14
av.u16[0x06] = 0
av[0x03]     = cd.subtype                                  // +0x0C
av[0x02]     = cd.elem | byte_80129E4[cd.family]
av[0x04]     = 0
if r4: sub_8021D14(alliance, id)   // u8 unk_203A0A0[alliance*0x170 + id] += 1 (saturating 0xFF)
return cd.action                                            // +0x0B
```

- **Quirk:** `sub_8021D14`'s per-alliance array is 0x170 bytes, but ids reach 0x19A. For ids ≥ 0x170, alliance 0
  increments bytes in alliance 1's area, and alliance 1 increments bytes past the end.
- `sub_80129A6` (0x080129A6) is an unreferenced copy of `sub_80126E4`. It is dead code.
- In the normal chip-use path, `sub_80127C0` immediately overwrites `av[4]`, `av.u16[8]` and `av.u16[6]` with values
  from the hand (§2.6.4). So `sub_80109A4`'s value survives only in callers that skip that step.

### 1.5 Variable damage (`sub_80109A4`, 0x080109A4)

```
if id == 0xFFFF: return 0
d = cd.damage
if d < 1000: return d
return off_80109DC[d - 1000](id, alliance)
```

`off_80109DC` is at 0x080109DC and has 45 entries (codes 1000..1044). None of them uses the RNG.

| d | Function | Value | Chips |
|---|---|---|---|
| 1000 | `sub_8010A90` | Link battle: min(opponent HP, 500). Otherwise: max enemy HP, capped at 500. | none |
| 1001–1018 | `sub_8010AE4` | [SP] navi chips: n = d − 1001; t = `byte_203EB00[alliance*0x28 + 2n]` (u16: the frames that player took to delete that SP navi, copied from each player's init exchange, `byte_203F510`/`byte_203F610`); `sub_8000D84(t)` makes it a BCD clock time `hh mm ss cc` (cc = hundredths, frames·100/60; capped 0x99595999); tier = how many of `byte_8010B2C` (0x1000, 0x1200, … 0x2800: 10.00 s to 28.00 s) are below it; damage = `byte_8020E54[n*0x16 + 2*tier]`. Trace-verified on SpoutMn[SP] (soundmod round 2: side 1's time 203 frames, tier 0). The engine takes the times as `RoundSetup::sp_times`. | 0xE2, 0xE5, …, 0x115 |
| 1019 | `sub_8010B78` | Custom-gauge based. | none |
| 1020 | `sub_8010BD0` | min(own MaxHP − HP, 500) | Muramasa 0x55 |
| 1021 | `sub_8010BF0` | own HP % 100 | NumbrBl 0x8A |
| 1022 | `sub_8010C06` | Link battle: min(opponent MaxHP / 2, 999). | none |
| 1023–1044 | `sub_8010C50` | k = d − 1023; base = `byte_80212D4[2k]`, step = `[2k+1]`; result = base + step·min(`sub_801265A()`, 5). `sub_801265A` is the buster attack level: `ns[1]` + 1 + form bonus, capped at 10, forced to 1 when `sub_8015B54(p)==5`. | Cross/beast chips 0x190..0x19A (for example HeatPres 50+20·lvl) |

Formulas are evaluated only when `sub_80109A4` is called:
- by the hand builder, once per entry (§2.4);
- every frame, for the current chip when it has flags 0x80 (`chip_800AEE8`, §2.5);
- at use, by `sub_80126E4` (the result is then overwritten).

### 1.6 Action dispatch: chip id → code

A chip's behaviour is selected entirely by `cd.action` (+0x0B) and `cd.subtype` (+0x0C).

1. At use, `sub_800FB54` calls `object_setAttack2(cd.action)`. This sets `obj.CurAction`, and `av[0x1C]` = slot 2 (§3.2).
2. Each frame, the navi's action dispatcher `sub_801B9E6` (0x0801B9E6) runs with r4 = ai, r6 = ai+0x80 (AIState) and r7 = av:

| `CurAction` | Target |
|---|---|
| < 0x10 | `table[CurAction]`: the per-navi base table passed in by `sub_80EA484` → `sub_801AF44`, i.e. `off_80EA4C8[ai.AIIndex]`. AIIndex is the cross/beast form. Entry 8 is idle (`sub_80EA734`). Only chips 0x190..0x199 (action 0x0A) land here, and entry 0xA exists only in the form tables that have ≥ 11 entries. |
| ≥ 0x10 | **`JumpTable80EAC60[CurAction − 0x10]`** (0x080EAC60, **79 entries**, actions 0x10..0x5E). The highest action used by a chip is 0x5B. |
| ≥ 0x10 with `av[0x1D] == 1` | `sub_80EAD9C` (0x080EAD9C), the Beast-Out wrapper. It calls the same `JumpTable80EAC60` entry from its phase 8 (§2.11). |

3. Two actions are generic "call a spawner indexed by subtype" handlers:
   - **0x15** `sub_80EBD9C` (0x080EBD9C), dimming chips. It calls **`off_802CCB4[av[3]]`** (0x0802CCB4, 42
     entries; slots 34, 35, 39, 40 are NULL). Chips 0x138 Gregar and 0x139 Falzar point at NULL slots 34/35. In the
     US ROM using them crashes the game, so the port can treat them as unsupported. The JP ROM would be needed to
     define real behaviour; that is out of scope.
   - **0x1B** `sub_80EC350` (0x080EC350), navi chips. It spawns T4 object 0x10 via `sub_80E192C`. That controller
     later calls **`off_802CD5C[subtype]`** (0x0802CD5C, 29 entries).
4. Action **0x1C** `sub_80EC39C` (0x080EC39C) is the "instant" handler. It calls `off_80EC3F0[av[3]]` once and exits.
   It is used by 54 chips: the MegaBuster pseudo-chip 0, Atk+/Navi+ left unfolded, FullCust, Boomer, Lance, FireHit,
   the error chip 0x185, and others.

`chip-table.md` lists the handler of every chip and, per action, the handler address and the chips that use it.
It also lists the spawner tables `off_802CCB4` and `off_802CD5C` and the formula table `off_80109DC`.
Non-chip actions in the same table include 0x10 movement (`sub_80EB04C`), 0x11 buster (`sub_80EB436`) and
0x16 charged buster (`sub_80EBE00`).

---

## 2. Battle-level chip flow

### 2.1 The chip block (per-player hand), 0x50 bytes at `0x020349C0 + 0x50*alliance`

Symbols `byte_20349C0` / `byte_2034A10`. `sub_8010018(alliance)` (0x08010018) returns the address. Parallel arrays
are indexed by hand entry i. A code pointer to entry i, `e = blk+2+2i`, reaches the other arrays at `e+0xC`,
`e+0x18`, `e+0x24` and `e+0x30`.

| Off | Type | Name | Meaning | Writers | Battle readers |
|---|---|---|---|---|---|
| +0x00 | u8 | `cur` | Index of the next chip (0..5). | `sub_800A964` (0), `sub_800B3D8` (turn copy), `sub_800FC7C` (++), `sub_801A2CC` (++ on a hit, §2.9) | everything |
| +0x01 | u8 | – | Always 0. Only the unreferenced functions `sub_802DE74` and `sub_802E588` use it. | init, copy | none |
| +0x02 | u16[6] | `id[i]` | Effective chip ids, after Program Advance (PA) and modifier folding. 0xFFFF-terminated; at most 5 chips. | init (0xFFFF), copy, `sub_80108FC`, `sub_800B090` | `sub_8010004`, `sub_800EDD0`, `sub_800FDEA`, `chip_800AEE8`, `sub_8017AB4` |
| +0x0E | u16[6] | `dmg[i]` | Base damage from `sub_80109A4(id, alliance)`, computed at build time. The current entry is refreshed every frame if it has flags 0x80. | builder, copy, `chip_800AEE8` | `sub_800EDD0` |
| +0x1A | u16[6] | `atk_bonus[i]` | Sum of the Atk+10/Atk+30/Navi+20 chips folded into this entry (§2.4). Two objects can also raise it at run time **[identity unverified]**: `sub_80DB1E0` (T3 0xAD) adds +1 every 5 frames, up to 50 times; `sub_80E667C` adds Param2. | builder, `sub_80DB1E0`, `sub_80E667C` | `sub_800EDD0` |
| +0x26 | u16[6] | `charge_bonus[i]` | 0 at build time. Raised while A is held by `sub_80F0608`, the per-frame hook `off_80EA93C[AIIndex]`. That only happens for navi 5 or cross form 5/0x11 with a Fire (`+4==1`) damage chip, so never for plain MegaMan. Zeroed at form changes (`sub_8014216`, `sub_80144C0`). | builder, `sub_80F0608` | `sub_800EDD0` |
| +0x32 | u16[6] | `raw_sel[i]` | The raw selection as packed `code<<9\|id`, taken before PA and before modifier folding. Illegal chips appear as 0x3785. | builder, copy | PA detection and the mega/giga counter (custom-screen side); no fight-time reader |
| +0x3E | u8[6] | `tag[i]` | `BattleState[7] − 1` (turn number from 0). | builder | `sub_800EDD0` (returned in r4 bits 8..15), `sub_80DB1E0` |
| +0x44 | u8[6] | `mod_flags[i]` | bit0 = the folder's Regular chip (slot-descriptor byte +4, set on slot 0 while BattleState+0x17); bit1 = WhiCapsl folded (use ORs 0x4000 DAMAGE_PARALYZE); bit2 = Uninstll folded (ORs 0x2000 DAMAGE_UNINSTALL). | builder | `sub_800EDD0` → `sub_8012C34` |
| +0x4A..+0x4F | – | – | Unused. | zero-init, copy | none |

Invariants: `cur ≤ 5`. The hand is empty when `id[cur] == 0xFFFF`.

**Trace check.** At frame 527, block 0 = `cur 0; id 11 11 11 11 FFFF FFFF; dmg 0…; raw 1A11×4`. That is 4× GunDelS3,
code N; the chip's damage field is 0. Block 1 stays in its init state for the whole replay because player 1 never
selected a chip.

**Lifetime.**
- `sub_800A954` (0x0800A954) initialises both blocks once per battle, from `sub_80071D4`. It calls `sub_800A964`,
  which zero-fills 0x50 bytes and then fills +2..+0xD with 0xFF.
- Blocks are **not** reset per turn. `sub_800B3D8` overwrites a block only when that player transmitted a
  non-empty hand. A player who selects nothing therefore keeps their unused chips from the previous turn,
  including `cur` **[trace: soundmod round 2 turn 4]**. A player who picks only Beast Out sends an empty hand
  (hand[0] = 0, all ids 0xFFFF), which does replace the block.

### 2.2 Custom screen → link → block (PvP)

The custom screen itself, and the verified timing model of this exchange for both players, are in
[`custom-screen.md`](custom-screen.md) (§5-§6); where they differ, it supersedes this section and §2.3-§2.4.

This path runs in link battles (`GetBattleEffects() & 8`). Both GBAs simulate both players. Only the hand
builder runs locally; each side's result is transmitted.

1. **Build (local side).** `sub_8029110` (0x08029110) runs when the player confirms the custom screen (called from
   `sub_8028D3A`). It writes the hand into `byte_20366C0` (0x50 bytes, block layout) using the work area
   `dword_2033000`. See §2.4.
2. **Package.** `sub_8026DC4` runs this on its first call in the custom-screen state:
   - `sub_802A4FC` counts the mega/giga selections into `dword_20367E0[class]`.
   - `sub_800B3A2` (0x0800B3A2) fills the turn packet `dword_203CBE0`:

     | Offset | Content |
     |---|---|
     | +0x00 | magic `0x56789123` |
     | +0x04..+0x53 | the hand |
     | +0x54..+0xB7 | the local NaviStats (0x64 bytes) |
     | +0xB8..+0xC7 | `byte_203CED0` (0x10 bytes) |

   - It then calls `sub_80200A4(0x32)`.
3. **Transmit.** Every frame, `sub_801FF18` (called via `sub_801FEEE`, the first step of the fighting-frame path)
   sends one word. It writes the word index (0x31 down to 0) at link-packet +4 and the word at +8, so the packet
   takes 50 frames and the magic word is sent last. If `sub_803EA2C()` is nonzero, nothing is sent that frame
   (**[unverified]** stall condition).
4. **Receive.** Each frame both players' incoming words are stored into staging buffers `dword_203F4A0 + 0x100*p`,
   where **p is the alliance**.
5. **Commit.** When both staging magic words equal `0x56789123` (`sub_800B46C`), `sub_8026DC4` does the following:
   - `sub_800B460` zeroes the magic words.
   - `sub_800B3D8` (0x0800B3D8) copies `staging_p+4 → block_p` (0x50 bytes) for each p whose byte `staging_p[4]`
     (= hand byte +0) is not 0xFF. Block 1 is copied only in link battles.
   - `sub_800B3D8` also copies both players' transmitted NaviStats into `eBattleNaviStats0/1`, so the NaviStats
     are re-synced every turn.
   - The custom screen closes. On the next frame `sub_8009338` sets `BattleState+1 = 0x0C` (fighting).

The builder writes `hand[0] = 0xFF` when there are no selections, meaning "no chips selected".

**Trace timing, battle 1** (local = alliance 0; the harness adds 4 frames of link latency):

| Frame | Event |
|---|---|
| 375 | p0 presses OK: the hand is built. |
| 376 | Slide-out starts; BS+0x11 bit 2 clears. |
| 386 | The packet is filled (`sub_800B3A2`). |
| 387–436 | p0's 50 words are sent. |
| 391–440 | p0's own words come back into its staging buffer (the link's 4-frame latency). |
| 478–527 | p1's words arrive. Word 1 is `0xFFFF00FF`: p1 selected nothing. |
| **527** | p1's magic word arrives. `sub_800B3D8` fills block 0. Later the same frame `sub_800FDC0` sets p0's ChipsHeld=4, Chip=0x11. `BattleState = [4,8,0,1]`. |
| 528 | Fighting (`BattleState+1 = 0x0C`). |

Battle 2 commits at 1774 and fights from 1775.

The commit write backtrace, from a write watch on 0x20349C0 from frame 527, is `CopyWords ← sub_800B3D8 ← sub_8026DC4 ←
sub_8026A88 ← sub_8026A28 ← sub_8009338 ← sub_8009158 ← battle_8007A44`.

### 2.3 Port contract for the hand

*Superseded*: the port simulates both players' custom screens ([`custom-screen.md`](custom-screen.md) §0). The
contract below describes the original's per-console view.

Per turn and per player, accept either input:
- the 0x50-byte hand the player transmitted, which is exact and simplest; or
- the ordered selection list `[(id, code)]` (≤ 5 entries) plus a "no selection" flag. The port then re-runs §2.4
  for that player, using that player's NaviStats, SP records and once-per-battle PA bitset.

Also accept **F_commit**, the frame on which `sub_800B3D8` runs. It depends on both players' confirm frames and the
link latency, so it cannot be derived from the chips.

On F_commit, in the custom-screen state handler (before `RunBattleObjectLogic`):
1. For each p with `hand_p[0] != 0xFF`: `block_p = hand_p` (all 0x50 bytes, `cur=0`, `charge_bonus=0`, `tag=turn−1`).
2. Install both transmitted NaviStats.
3. Run `sub_800FDC0` and then `chip_800AEE8` later in the same frame (§2.5).
4. Enter fighting on F_commit+1.

### 2.4 Hand builder `sub_8029110`: validity, Program Advance, Atk+ folding

Inputs:
- `[r5+8]` = number of selections n (0..5).
- `[r5+0x48+k]` = slot number of the k-th selection.
- `getLocOfActiveChips_8027E1C(slot)` = `unk_20365C0 + 12*slot`, a descriptor with `+0 type` (0 = folder chip,
  2 = cross/beast special slot), `+4 u8 flag`, and `+8 → packed chip`.
- `L = BattleState+0xD` (local alliance).

Work arrays in W (`dword_2033000`, zeroed): `id@+0`, `dmg@+0xC`, `bonus@+0x18`, `fam@+0x24`, `code@+0x30`,
`flag@+0x3C` (6× u16 each).

```
W.id[*] = 0xFFFF; hand.raw[*] = 0xFFFF; [r5+0xB] = 0x14
j = 0
for k in 0..n:
    d = desc(sel[k])
    if d.type == 2: sub_8029344(); continue                 // cross/beast choice, not a hand entry
    if d.type == 0: W.flag[j] = d.byte4
    pk = getChipID_802A54E(*d.ptr)                           // pk, or 0x3785 if sub_800B022(pk) rejects it
    hand.raw[j] = pk; id = pk & 0x1FF; W.id[j] = id; W.code[j] = pk >> 9
    if id >= 0x190: word_20349AC |= 1 << (id - 0x190 + 1)
    W.dmg[j] = sub_80109A4(id, L); W.fam[j] = cd(id).family; j += 1
if n != 0:
    (pa, off, len) = sub_8029520(hand.raw, 0)                // PA detection
    if pa != 0 && sub_8029328(off, len) == 0:                // veto if any W.flag in the window has bits & 0xFE
        sub_80292CC(pa, off, len); [r5+0xB] = 0x10           // replace the window by the PA (sub_802B6F2 = UI)
    sub_8029224()                                            // modifier folding
hand[0] = (n == 0) ? 0xFF : 0                                // hand[1] never written
for i in 0..6: hand.id[i]=W.id[i]; hand.dmg[i]=W.dmg[i]; hand.atk_bonus[i]=W.bonus[i];
               hand.charge_bonus[i]=0; hand.tag[i]=BattleState[7]-1; hand.mod_flags[i]=W.flag[i] as u8
```

**Validity check `sub_800B022`.** The chip is replaced by 0x3785 (error chip 0x185, code 0x1B) if either of these holds:
- its code isn't 0x1B or 0x1C, its class is 1 or 2 and `dword_20367E0[class] > ns[L].byte(0x0A+class)` (the
  mega/giga limit; the counts are per round, raised when a hand is sent, custom-screen.md §5.4); or
- `sub_8006EE8(id, code')` fails. Here `code' = 0xFF` when code == 0x1B or id ≥ 0x19B. That check is an
  anti-tamper mirror plus the code-legality test.

Legitimate input never triggers it.

**PA detection `sub_8029520`** (0x08029520):
- Needs at least 3 raw entries. For each start position with at least 3 remaining, it walks the 43 records of
  `off_802BCB0` (data at `byte_802BA60`): `{u8 len, u8 kind, u16 pa_id, u16 ids…}`.
- **kind 0** (`sub_80295C8`): `len` copies of `ids[0]` whose codes run consecutive and ascending in selection
  order, with at most one `*` wildcard.
- **kind 4** (`sub_802961A`): the exact id sequence; codes are ignored.
- On a match, `sub_8029652` enforces **once per round per player** via bit `pa−0x140` of `dword_203CA48` (cleared
  by `sub_801BE70` at each round's init). A PA already spent is passed over and the scan goes on. The bit is set
  **before** the `sub_8029328` veto, so a vetoed PA is still spent (the veto never fires in practice).
- On a formed PA, `sub_802B6F2` arms the PA animation (custom sub-state 0x10), which delays sending the hand.
- Examples: GigaCan1–3 (Cannon/HiCannon/M-Cannon ×3), H-Burst, LifeSrd, and StreamHd (AuraHed ×3).

**PA replace `sub_80292CC`:**
- Entry s becomes `(pa, sub_80109A4(pa), fam(pa))`.
- Bit 0 (the Regular chip) of the window's other entries is ORed into entry s.
- The tail (id, dmg, fam, flag) shifts down. `W.bonus`, `W.code` and `hand.raw` are **not** shifted.

**Modifier folding `sub_8029224`** covers **Atk+X / Navi+X and similar**. For entry i = 1, 2, … (entry 0 never
folds) while `W.id[i] != 0xFFFF`, let `f = cd(W.id[i−1]).flags`:

| Chip at i | Condition on previous chip | Effect |
|---|---|---|
| 0xC0 Atk+10, 0xC3 Atk+30 | `f & 2` | `W.bonus[i−1] += W.dmg[i]` (10 / 30) |
| 0xC1 Navi+20 | `f & 4` | `W.bonus[i−1] += W.dmg[i]` (20) |
| 0xB8 WhiCapsl | `f & 2` | `W.flag[i−1] \|= 2` (paralyze) |
| 0xB9 Uninstll | `f & 2 && !(f & 1)` | `W.flag[i−1] \|= 4` (uninstall) |

- A folded entry is removed by shifting id, dmg, fam and flag down. Entry i is then re-examined, so chained Atk+
  chips stack.
- A modifier that does not fold stays in the hand as its own chip: action 0x1C, which consumes a use.
- **Quirk:** before each shift the code executes `[r3+0x3A] |= [r3+0x3C] & 1` with a stale r3. In the normal path
  it lands in bytes that the final copy overwrites, so it is harmless. Keep it in mind for byte-exact W emulation.

The Atk+ bonus stays separate from `dmg` all the way to the spawner. It is carried in `av.u16[6]`, and each action
handler adds it to the damage (§3.4). That is why a chip whose handler ignores `av` damage, such as GunDelSol,
gets nothing from Atk+.

### 2.5 Per-frame order and exposure: `sub_800FDC0`/`sub_800FDEA`, `chip_800AEE8`

Fighting frame in `battle_8007A44`:

1. `sub_801FEEE` (link rx/tx)
2. …
3. mode handler `off_8007B50[mode]` = `sub_8009158` → the BattleState+1 handler. The custom commit or
   `sub_800B090` runs here, and so does the pad latch `sub_8012DFC(0/1)` (§2.6.1).
4. `RunBattleObjectLogic`
5. `sub_802FFF4`
6. `sub_800BFC4`
7. **`sub_800FDC0`**
8. `sub_801BEE0` (battle UI, including the telop)
9. `sub_802CEC8`
10. **`chip_800AEE8`**
11. …

Steps 7 and 10 are called.

**`sub_800FDC0`** (0x0800FDC0) walks the 8 actor pointers at `BattleState+0x80`. For each one that is non-null,
has AIData, and has `ActorType == 2` (player), it calls `sub_800FDEA` (0x0800FDEA), which sets:
- `obj.ChipsHeld (+0x1A)` = number of ids from `id[cur]` up to 0xFFFF;
- `obj.Chip (+0x2A)` = `id[cur]`.

These fields are therefore **one frame stale** while objects run. The chip pipeline reads the block directly
(`sub_8010004`, `sub_800EDD0`), never `obj.Chip`. Player objects are recognised by `sub_800F29C(NameID) == 2`.
The object fields are read only for non-player actors, by HUD/object code such as `sub_80E4566`, and by AI.

**`chip_800AEE8`** (0x0800AEE8), for p = 0 then 1:

```
e = block_p + 2 + 2*cur
if cd(e.id).flags & 0x80: e.dmg = sub_80109A4(e.id, p)
```

- Only the **current** entry is refreshed. Later entries keep their build-time value until they become current.
- **Quirk:** with an empty hand, e.id = 0xFFFF and the flag read hits ROM 0x082E1D85 (= 0x30, bit 7 clear), so
  nothing happens. The port should special-case 0xFFFF as a no-op.
- Because the chip pipeline runs inside `RunBattleObjectLogic`, it uses the `dmg` value left by the **previous**
  frame's `chip_800AEE8`.

### 2.6 Using a chip (A button → attack action)

#### 2.6.1 Pad latch: `sub_8012DFC(alliance)` (0x08012DFC)

It runs in the mode handler, before objects, and copies `dword_2036820 + 8*alliance` into the player's AIData:

```
if battle_isBattleOver(): ai.{22,24,26,28} = 0; return
if ns[0x2C] in 0x17..=0x18: return                 // Beast Over: no player input
prev = ai.u16[0x22]; ai.u16[0x28] = prev          // disasm "JoypadReleased" is really previous-held
ai.u16[0x22] = held; ai.u16[0x24] = held & !prev   // pressed edge
ai.u16[0x26] = prev & !held                        // released edge ("JoypadUp")
if battle_isTimeStop():                            // separate edge set used while dimmed
    ai.u16[0x30] = ai.u16[0x2A]; ai.u16[0x2A] = held
    ai.u16[0x2C] = held & !old2A; ai.u16[0x2E] = old2A & !held
else: ai.u16[0x2A..=0x30] = 0
```

- Because the dimming set is zeroed while not dimmed, **a button already held when a dimming starts counts as
  pressed on the first frozen frame**.
- **[trace]** Frame 638, AIData 0x02034180: `+0x22=0xFC01`, `+0x24=1`, `+0x26=0`, `+0x28=0xFC00`.

#### 2.6.2 Request flags: `sub_8012FC8` (0x08012FC8)

Called from the player update via `sub_80EA484 → sub_8012E74 → sub_8012EA0` when the battle is not paused.
Requests accumulate in `ai.u32[0x44]` (set/clear/get: `SetAIData_Unk_44_Flag` / `ClearAIData_Unk_44_Flag` /
`GetAIData_Unk_44_Flag`). State bits live in `ai.u32[0x48]` (`sub_8010312` / `sub_801031C` / `sub_801032C`).

| Bit (ai+0x44) | Meaning | Set by | Consumed by |
|---|---|---|---|
| 0x1 | Buster shot | B (release edge for plain MegaMan) | `sub_80F0354` → `sub_8011764` |
| 0x2 | Charged buster | B with the charge full | `sub_80F0354` → `sub_80117A4` |
| **0x4** | **Chip use** | A (see below); `sub_802D358` (Beast Over auto-use) | `sub_800FB54` |
| **0x8** | **Charged chip use** | A with the chip charge full (cross/beast only) | `sub_800FB54` |
| 0x10 | B+forward special | only if `ai[0x08] != 0xFF` | `sub_80F0354` → setAttack3 |
| **0x800** | **Cut-in** | A while dimmed | `sub_8017AB4` |
| 0x10000 | Battle-Chip-Gate slot-in chip | `sub_802E62A` | `sub_80127C0` (via the caller's r4) |
| 0x20000 / 0x40000 | Chip / buster charging (A / B held) | `sub_8012FC8` | `sub_8012EBC` |
| 0x600, 0x8600, 0x80000 | Forced status actions | elsewhere | `sub_801056A`, action 0x49 |

Algorithm, in exact order:

```
f0 = ai.flags44
if battle_isTimeStop():
    if !sub_800A772(alliance) || (f0 & 0x800) || sub_8010004() == 0xFFFF: return
    if ai.u16[0x2C] & A: set(0x800)                 // cut-in request (§3.6.5)
    return
[chip-gate SELECT branch: needs sub_800A8F8() (battle flag 0x40); off in PvP]
if GetBattleMode() != 1: [L/R handling; L/R opens the custom screen when battle_getFlags()&2]
[charge state machine: 0x20000 while A held if sub_801336C(); 0x40000 while B held if sub_8013396()]
[B+forward special -> 0x10]
[buster: if ai[6] != 0xFF && (f0 & 3) == 0: B on press or on release (release if ai[7] != 0xFF) -> set 1, or 2 if charged]
if GetBattleMode() == 9 && pressed & A: set(0x10000000)
// chip request
if sub_800A772(alliance) && (f0 & 0xC) == 0 && sub_8010004() != 0xFFFF:
    k = sub_801336C() ? released(+0x26) : pressed(+0x24)
    if k & A: set((ai[0x1E] == 1 && ai[0x1D] == 2) ? 8 : 4)
```

Helpers:
- **`sub_800A772`** (0x0800A772), "may use a chip". Returns `ai[0x19] == 0 && (BattleState.u32[0x5C] & (alliance ? 8 : 4)) != 0`.
  - `BattleState+0x5C` is set to 0xC at init (`sub_800A79C` → `sub_80013A2`); the trace value is 0x1F.
  - Nothing clears bits 2/3 (`sub_800138E` has no callers), so in practice this is the lockout test `ai[0x19] == 0`.
- **`sub_8010004`** returns `id[cur]` for `obj.Alliance`.
- **`sub_801336C`**, "A is hold-to-use". True only if `ai[5] != 0xFF || ai[0x11] != 0xFF`, `sub_8012F3E()`, a chip
  is present, and the per-form chargeability test `sub_8013236(chip)` passes.
- **Plain MegaMan**: `ai[4..8] = FF FF 00 01 FF` and `ai[0x11] = FF` (from `sub_800FEEC`/`sub_800FF5E`), so
  `sub_801336C` is always false. **A acts on the press edge and only ever sets flag 4.** B fires on release
  (`ai[7] = 1`).

**[trace]** The frame-638 watch shows `SetAIData_Unk_44_Flag(4)` via `sub_8012FC8 ← sub_8012EA0 ← sub_8012E74`.

#### 2.6.3 Idle decision: `sub_80EA734` → `sub_80F0354` (0x080F0354)

The request is only acted on in idle, action 8 (`off_80EA52C[8]` = `sub_80EA734`). There,
`JumpTable80EA7B0[enemy_getStruct1(NameID)[4]]` is `sub_80F0354` for every player navi.

```
[HUD-only code under !battle_networkInvert(alliance): drop in the port]
if obj.CurPhase == 0:                                    // idle entry countdown (does NOT gate chip use)
    if !obj.PhaseInitialized: obj.Timer = 10; flags48 |= 0x10 (ready); flags48 &= !0x40; obj.PhaseInitialized = 4
    obj.Timer -= 1; if (obj.Timer as i16) <= 0: obj.CurPhase = 4, PhaseInitialized = 0
if ns[0x2C] >= 0x17: Beast Over AI path (sub_802D322)
[chip-gate no-ops: sub_802E4E4, sub_802E4B8]
if flags & 0x600: sub_801056A(0,0); return 1
if sub_8010660(): return 1                               // Tango support (§2.10)
f = flags
if f & 0x20:       setAttack1(0x16) ...; return 1
if f & 0x1:        setAttack1(sub_8011764()); return 1   // buster (§5)
if f & 0x2:        (ai[7] in 0x21..=0x26 ? setAttack2 : setAttack1)(sub_80117A4()); return 1
if f & 0x10:       setAttack3(sub_8011790()); return 1
if f & 0x10000000: setAttack1(sub_801177A()); return 1
chip = sub_800FB54()                                     // §2.6.4; sets CurAction on success
if chip == 0xFFFF:
    if (d = sub_800FA54()) != 0: sub_80116AE(d, sub_8010332(), sub_80103A8()); return   // movement, action 0x10
    if f & 0x3000: setAttack4(0x3B); return
    if ai[0x1A] != 0: sub_80116D8(ai[0x1A], sub_8010332())
    return 0
if sub_80106C0(chip) || sub_8010740(chip): object_exitAttackState(); goto END   // Beat / Rush cancel (§2.10)
flags48 &= !0x10; flags48 |= 0x40                         // ready -> attacking
[opponent telop sub_801EB18: presentation only]
if av[0x1B] == 0 && av[0x1C] != 5: sub_800FC7C()          // CONSUME: blk.cur += 1
END: return 1
```

- **Priority within one frame:** Tango, 0x20, buster, charged buster, 0x10, 0x10000000, **chip**, movement.
  A buster request on the same frame beats a chip.
- **`sub_800FC7C`** (0x0800FC7C): `if cur < 5 && id[cur] != 0xFFFF { cur += 1 }`.
- **[trace]** Frame 638: `write8 [0x020349C0] = 1` via `sub_800FC7C ← sub_80F0354 ← sub_80EA734 ← sub_801B9E6 ←
  sub_801AF44 ← sub_80EA484`.

#### 2.6.4 Take the chip: `sub_800FB54` (0x0800FB54) and `sub_80127C0` (0x080127C0)

```
sub_800FB54:
if obj flags & 0x1000 (sliding): return 0xFFFF           // request stays pending
m = flags44 & 0x1000C; if m == 0: return 0xFFFF
arg = 0
if m & 8: [charged-chip path, cross/beast only, §2.11]
action = sub_80127C0(arg)                                // r4 = m on entry (see quirk)
object_setAttack2(action)                                // CurAction, phase=0, av.u16[0]=0, av[0x1C]=2, sub_801011A
if av[0x1B] != 0 || ns[0x2C] in 0x0B..=0x18: av[0x1D] = cd(av.u16[0x14]).beast_lockon
clear flags44 & 0x1000C
return (av.u16[0x14], av.u16[0x08], av.u16[0x06])
```

`sub_80127C0(arg)`, in exact write order (confirmed by the frame-638 watch):

```
// Source select: bit 16 of the CALLER's r4 (0 = hand, 1 = chip-gate slot-in). From sub_800FB54, r4 = flags & 0x1000C.
if (r4 & 0x10000) == 0: (chip, dmg, extra, _, flags16) = sub_800EDD0(obj, arg)
else: av[0x1B] = 1; (chip, dmg, extra, _) = sub_800EE26(obj)          // slot-in: unreachable in PvP
lo = flags16 & 0xFF
if (r = sub_8010D58(cd(chip).dark_subst)) != 0xFFFF: (chip, dmg, extra) = r; lo = 0   // dark chip w/o bugfrag
av.u16[0x14] = chip
sub_80126E4(chip)                         // §1.4 (fills 0x14, 0x0C, 0x08, 0x05, 0x06, 0x03, 0x02, 0x04; counts use)
av[0x04] = arg
av.u16[0x08] = dmg                        // hand damage REPLACES sub_80109A4's value
b = sub_8012C7C(arg); if b != 0: av.u16[8] += b; PlaySound(0x87)                 // cross charge: 0 for MegaMan
av.u16[0x06] = extra
(d, code) = sub_8012A38(obj, chip, av.u16[8], arg); av.u16[8] = d                // may OR 0x8000 (double)
if code == 1: sub_8015BEC(alliance, 0x80); sound 0x87                             // Full Synchro consumed
if code == 2: sub_80143A6(); sound 0x87                                           // Anger consumed
av.u16[8] = sub_8012C34(av.u16[8], lo)    // lo&2 -> |0x4000 (paralyze), lo&4 -> |0x2000 (uninstall)
av.u16[8] = sub_8012C4A(chip, av.u16[8])  // EraseCross family-0xA chips: |0x1000
heal = (ns[0x2C] in {6,0x12} && cd.elem == 2 && !(cd.flags & 1)) ? (ns.u16[0x3E] + 0x13) / 0x14 : 0
if ns.u16[0x50] + heal != 0: sub_800E2FC(ns.u16[0x50] + heal, 0)    // chip-recovery heal (+ T4 effect 6)
if cd.flags & 4: sub_800AB46(alliance, 6, 1)                         // navi-chip counter
sub_800B79A(av.u16[0x14])                                            // dark-chip side effect on ns[0x18]
return cd(chip).action
```

**`sub_800EDD0(obj, arg)`** (0x0800EDD0) reads the hand. Player branch:

```
e = blk + 2 + 2*cur; chip = u16[e]
(bonus, _) = sub_800EF34(chip, arg)                  // cross/beast bonus; 0 for plain MegaMan except the aura case
dmg     = u16[e+0x0C]                                // blk.dmg[cur]
extra   = u16[e+0x18] + bonus + u16[e+0x24]          // atk_bonus + cross bonus + charge_bonus
flags16 = (blk.tag[cur] << 8) | blk.mod_flags[cur]   // returned in r4, clobbering the caller's r4
```

The non-player branch returns `(obj.Chip, 0, 0, obj.ChipsHeld)`.

`sub_800EF34` (0x0800EF34) boosts, with `B = (flags & 2) && !(flags & 1)`:

| `ns[0x2C]` | Condition | Bonus |
|---|---|---|
| 1 / 0x0D | B && family 0 | +50 |
| 2 / 0x0E | B && family 2 | +50 |
| 3 / 0x0F | B && family 5 | +50 |
| 8 / 0x14 | B && family 8 | +10 |
| 4 / 0x10 | (flags&2) && family 6 | +30 |
| 9 / 0x15 | B && family 9 | +10 |
| 0x0B..0x16 fallback | B && family 0xA && BattleMode != 1 | +30 |

- When `ns[0x29] != 0` (a non-MegaMan navi), `sub_800F09E` is used instead.
- The final step is **`sub_800F1DC`**: +50 for StreamHd 0x150 or AuraHed 0x5F..0x61 while the user's
  `CollisionData.Barrier (+6)` is in 1..0xF and (`Barrier != 8` or `CD[+0x16] != 0`). For plain MegaMan this is
  the only possible bonus.

**`sub_8012A38`** (0x08012A38), the DAMAGE_DOUBLE decision:
- Requires `flags & 2`.
- `emo = sub_8015B64(alliance)`:
  - 5 if `ai.u16[0x36] != 0` or mood `ns[0xE] == 0`;
  - else 3 if `ai.Anger (+0x34) != 0`;
  - else 1 if `ai.u16[0x32] != 0`;
  - else 2 if mood == 0xFF;
  - else 0.
- emo 2 (Full Synchro) gives code 1, and emo 3 (Anger) gives code 2. Otherwise the cross checks `sub_8012AFA`
  (Tomahawk, Wood), `sub_8012B4E` (Spout, Aqua), `sub_8012BA2` and `sub_8012ABC` can give codes 4 or 6.
- Any nonzero code ORs **0x8000** into the damage.
- Codes 1 and 2 reset the mood/anger state as shown above.

**Quirks:**
- The source select reads **bit 16 of the caller's r4**.
- In the flag-8 path with `t == 0xFF`, the chip's *family byte* is passed as `arg` (§2.11).
- `object_setAttack2 → sub_801011A → sub_80E1662` writes through `ai.u32[0x40]`, which is NULL for plain MegaMan,
  to BIOS addresses 0x10/0x64. Treat these as no-ops.

### 2.7 AIAttackVars when the chip action starts (end of the use frame)

| Off | Size | Value |
|---|---|---|
| +0x00 | u8 | 0: phase selector |
| +0x01 | u8 | 0: phase-initialised flag |
| +0x02 | u8 | element byte `cd.elem \| byte_80129E4[cd.family]` |
| +0x03 | u8 | `cd.subtype` |
| +0x04 | u8 | `arg`: 0 normally, 1 or family for charged uses |
| +0x05 | u8 | `cd.lockout` |
| +0x06 | u16 | extra/bonus damage = `atk_bonus + charge_bonus + cross/aura bonus` |
| +0x08 | u16 | damage = `blk.dmg[cur]` (+ cross charge bonus), OR 0x8000 double / 0x4000 paralyze / 0x2000 uninstall / 0x1000 erase |
| +0x0A | u16 | `cd.hit_param` |
| +0x0C | u32 | `cd.params` (+0x10..+0x13) |
| +0x14 | u16 | final chip id (after dark-chip substitution) |
| +0x1B | u8 | 0 (1 only for a slot-in chip; that also suppresses the index increment) |
| +0x1C | u8 | 2 (attack slot) |
| +0x1D | u8 | 0; `cd.beast_lockon` in forms 0x0B..0x18 |
| +0x1E | u16 | 0 |

- **Not written:** +0x10..+0x13, +0x16..+0x1A and +0x20..+0x4F keep **stale values** from earlier attacks.
  At frame 638, +0x16..+0x18 were `03 02 04`, left over. Keep them byte-exact, or prove each handler writes
  before it reads.
- **[trace]** Post-frame 638 at 0x02034220: `av = 00 00 00 02 00 00 0000 0000 0000 00000000 …` with +0x14=0x0011,
  +0x1C=2 and +0x1D=0.
- Other same-frame effects:
  - `flags44 &= !0x1000C`;
  - `flags48`: 0x10 → 0x40;
  - `blk.cur += 1`;
  - the usage counter increments.
  - `obj.Timer` is **not** reset (it stays at 7 in the trace).

### 2.8 Attack end, lockout, input buffering

Handlers end with `object_exitAttackState` (0x08011714) or `sub_801171C`. The latter is the same code but does not
reset CurAnim.

```
[object_exitAttackState only] obj.CurAnim = 0
av[0x1B] = 0; slot = av[0x1C]
if slot != 4:                                   // 4 = movement: skip all cleanup
    if slot == 2: ai[0x19] = av[0x05]           // chip lockout
    if slot == 3: ai[0x15] = av[0x05]
    ai[0x1A] = 0
    flags44 &= !0x1000003F                      // drops A/B presses buffered during the attack
    sub_8012EA8()                               // reset charge meter ai[0x1B/0x1D/0x1E], clear 0x60000
    obj flags1 &= !0x400000                     // OBJECT_FLAGS_USING_ACTION
obj.CurAction = 8; av.u16[0] = 0                // obj.CurPhase is NOT reset
```

- `sub_80107D4` (0x080107D4) runs after the action every frame. When not dimmed, it decrements `ai[0x19]` and
  `ai[0x15]` if nonzero.
- If an attack exits on frame F, idle runs from F+1. With lockout N, `sub_800A772` passes from frame F+N (F+1 when
  N=0). The A press must be on or after that frame.
- **A presses during a chip or buster action are always lost.** They set flag 4, but the exit clears it.
  **[trace]** Flag 4 was set at 768 from the press at 759 and cleared at 777; the next chip needed a new press at 779.
- Presses during movement (slot 4) survive. The chip fires on the first idle frame **[unverified]**: this is
  inferred from the slot-4 exception, and the trace has no such case.
- Presses during damage reactions are discarded. Actions 3, 4, 6 and 7 and `sub_8017A38` clear 0x3F or 0x1000043F.
- While sliding on ice, the request stays pending.

### 2.9 Other hand mutations during a fight

| Function | Effect |
|---|---|
| `sub_801A2CC` | When the owner takes a hit whose `CollisionData.FlagsFromCollision & 0x10`: `cur += 1` (the chip is lost) and `obj.Chip = 0xFFFF`. |
| `sub_80C33CA`, `sub_80C35CE` | Support objects that advance the **opponent's** `cur` (Rush/Beat follow-ups). |
| `sub_80108FC(p)` | Removes every remaining 0x190..0x19A entry by shifting all arrays. Called when a cross/beast form reverts (`sub_802D9B0`, `sub_802DB80`). |
| `sub_800B090` | Called every fighting frame for the **local** player only. It replaces `id[cur]` with 0x185 if the anti-tamper mirror check fails. Its limit check is dead (it compares against the wrong register). **Port: no-op.** |
| Death/reset handlers (`sub_80165F8`, `sub_8016EE0`, `sub_80170E4`, `sub_801741C`) | Set ChipsHeld = 0 and Chip = 0xFFFF. For players, `sub_800FDC0` overwrites this in the same frame. |

### 2.10 Support-navi cancels (link battles only, NaviStats+0xD bits)

Checked in `sub_80F0354` after the action is already set up:

| Support | Function | Trigger | Effect |
|---|---|---|---|
| **Beat** | `sub_80106C0` | Opponent `ns[0xD] & 2` and the chip's class is 1 or 2 | Clears the bit and spawns chip object 0x17A at the opponent (`sub_80E90FE`, registered with `sub_800BF16`). |
| **Rush** | `sub_8010740` | Opponent `ns[0xD] & 1` and `cd.flags2 & 2` | Clears the bit and spawns 0x179. |
| **Tango** | `sub_8010660` | Own `ns[0xD] & 4` and `HP ≤ MaxHP/4` | Checked every idle frame before any request. Spawns 0x17B; `sub_80F0354` returns 1 without acting. |

- On a Beat or Rush cancel, `object_exitAttackState` runs, so the lockout applies and requests are cleared. The
  index is **not** incremented here; the support object calls `sub_800FC7C` on the victim later.
- In the trace, `ns[0xD] = 0` for both players.
- Identification from the chip names at 0x179..0x17B **[unverified]**.

### 2.11 Cross / Beast Out differences (reachable in PvP; trace battle 2)

`ns[0x2C]` is the form. The numbering is inferred from code pairings and not checked against text **[unverified]**:

| `ns[0x2C]` | Form |
|---|---|
| 0 | none |
| 1..10 | crosses |
| 0x0B / 0x0C | Gregar / Falzar Beast Out |
| 0x0D..0x16 | cross + beast |
| 0x17 / 0x18 | Beast Over |

- **Charged chips (flag 8).** Forms set `ai[4..8]` and `ai[0x11]` from `byte_8020354 + 6*form` (`sub_800FEEC`).
  A may then become hold-to-use: 0x20000 while held, released with a full charge → flag 8. `sub_800FB54` then
  takes the flag-8 path:
  - `sub_800EDD0(obj, 8)`; family 0xA chips use `t = ai[0x11]` and zero `av.u16[0x14]` (the chip id, so the use
    reports chip 0), others use `t = ai[0x05]`.
  - `t == 0x18` → `arg = sub_8012CB2()`.
  - `t == 0xFF` → `arg` = family, or 0 for family 0xA. **Quirk:** the family is passed as `arg`.
  - `t ∈ {0x0D,0x1F,0x20,0x29,0x2D,0x05}` → `arg = 1`.
  - Any other `t` → `av[4] = 0; setAttack2(sub_80117BA(t))`, i.e. `off_80117D4[t]`. **`sub_80127C0` is not called**,
    but the caller still consumes the hand chip. Actions 0x52, and 0x41 in form 0xF, also set `av[0x1D] = 1`.
- **Beast lock-on wrapper `sub_80EAD9C`.** It runs when `av[0x1D] == 1`, which happens in forms 0x0B..0x18 for the
  120 chips with `+0x0F = 1`. State lives at `s = av+0x1E`:

  | s offset | av offset | Meaning |
  |---|---|---|
  | s+0 | +0x1E | phase |
  | s+1 | +0x1F | init |
  | s+2..3 | +0x20..0x21 | saved panel |
  | s+4 | +0x22 | queued next chip |
  | s+5 | +0x23 | A-ignore timer |
  | s+8 | +0x26 | countdown |

  Phases:
  - **Phase 0** (`sub_80EADDC`): snap to FuturePanel, reserve it, set the moving flag; `s[4]=0`, `s[5]=12`.
  - **Phase 4** (`sub_80EAE28`): on entry `s[1] = 1`, freeze the lock-on marker (`sub_80E1654`: its CurAnim = 1),
    `object_setAnimation(4)` if `sub_80F02A2`, `s[8] = 3`. When the countdown reaches 0: the NaviCust panel trail
    (`sub_8013CC4`) on the current panel, then teleport next to the target via `ho_8026554(sub_80E164A(), mode)`.
    `mode` is 0 (stay) when blind or confused outside Beast Over; else 0xC for action 0x52 (`sub_80EAF1A`), a
    per-variant table for action 0x41 (`sub_80EAF26`), or `cd.lockon_mode` (+0x17, read with the chip id at
    `av.u16[0x14]`). The old panel goes to `s[2..3]`; the navi's panel, coordinates and collision panels change (not
    FuturePanel, which still holds the old panel). On column patterns 0x31/0x23/0x33 with `av.u32[0x2C]` set, it also
    faces that object (`sub_800F2FC`). If it moved, two afterimages (T4#0x28 via `sub_80EAFC2`): at the old panel's
    coordinates with lifetime 12, then halfway between old and new position (arithmetic mean) with lifetime 20.
    Then `object_setAnimation(0)`, phase 8.
  - **`ho_8026554(x, y, mode)`**: a target panel off the field means mode 0 (`sub_802661C`: the navi's own panel).
    Most modes search (`sub_80265D0`): try a mode-specific list of offsets next to the target (dx toward the navi's
    front, so negative is between the navi and the target); if none fits, shift the reference column by −1..−4
    toward the navi (`byte_8026735`) and try again. A candidate must be on the field, must not lie past the target
    in the navi's facing direction, and must pass `sub_800E680` (the step rule without the side check: floor
    required unless AirShoe or standing off solid ground, and no other body). Modes 3, 6, 9 and 0x10 then take the
    middle row of the found column if it passes `sub_800E680` (`sub_80265FE`; its "not found" test reads flags that
    a `mov r1, #2` just set, so it always runs; with nothing found it tests column 0 and fails). A result with
    column 0 means "not found", and the caller keeps the navi's panel. The offset lists are extracted into the
    lock-on rules (`Rules::lockon`, a content pack's `rules/lockon.toml`): mode 9 (GunDelSol) is two columns before
    the target, same row, then above, then below; mode 0xC (the claw) is the panel right in front of it. Modes 1,
    0xA, 0xB, 0xE, 0x11 and 0x12 work differently.
  - **Phase 8** (`sub_80EAF36`): calls `JumpTable80EAC60[CurAction−0x10]` each frame. When the handler has returned
    to CurAction 8:
    - if `s[4]` is set: `sub_800FC30` **chains** the next chip (it rejects 0xFFFF, 0x52, 0x53 and dimming chips, then
      `sub_80127C0(0)`, `setAttack2`, `av[0x1D]=1`), then `sub_800FC7C` and back to phase 4;
    - otherwise it warps back and exits: `sub_801011A` (clearing `av[0x1D]`, `s[0..1]` and unfreezing the marker),
      Panel = FuturePanel with the reservation dropped and coordinates and collision panels updated, ObjectFlags1
      &= ~0x40 and |= 0x80000, then `object_exitAttackState` if `sub_80F02A2`, else `sub_801171C`.
    - An action below 0x10 when phase 8 starts goes straight to that exit.
  - After each phase: `s[5] -= 1` if nonzero; otherwise `s[4] = 1` if A is pressed.
  - **Quirk:** the chain branch writes `AIData[0x85] = 0xC` (AIState+5) instead of `s[5]`.
- **Beast Over** (0x17/0x18): no input latch. `sub_802D322`/`sub_802D358` auto-use chips by setting flag 4 and
  calling `sub_800FB54`.

Battle 2 of the trace exercises this path. Details are in §4.6.

---

## 3. Generic attack framework

### 3.1 Battle object model (the parts chips touch)

| Type | Pool | Count | Size (incl. 0x10-byte list node) | Default header flags | Jump table |
|---|---|---|---|---|---|
| T1 (actors, attachments, summons) | `eT1BattleObject0` 0x0203A9B0 | 0x20 | 0xD8 | 0x19 | `T1BattleObjectJumptable` 0x08003C9C |
| T3 (attacks: shots, hitboxes) | `eT3BattleObject0` 0x0203CFE0 | 0x20 | 0xD8 | 0x09 | `T3BattleObjectJumptable` 0x08003EC4 |
| T4 (effects, controllers) | `eT4BattleObject0` 0x02036870 | 0x20 | 0xC8 (ExtraVars 0x1C) | 0x19 | `T4BattleObjectJumptable` 0x080042C8 |

**Spawning.** `object_spawnType1/3/4` (0x08003320 / 0x08003358 / 0x080033AC) call `SpawnBattleObjectCommon` (0x08003278):
- Takes the first free slot by an MSB-first bitfield scan and zero-fills it.
- Sets flags = default, `Index = r0`, `X/Y/Z = r1/r2/r3`, `Params = r4`.
- Links the object with **`sub_8003400`**, which inserts it **immediately after the object currently being updated**
  (`dword_200AF70`). A new object is therefore updated later in the same `RunBattleObjectLogic` pass.
  - Several spawns made from one update end up in reverse spawn order.
  - With no current object, the new one is appended at the tail.
- If the pool is full, the spawn returns 0. Most callers then silently skip.

**Update loop.** `RunBattleObjectLogic` (0x080031AC) walks `eBattleObjectsLinkedListStart`:
- Skips an object if GameState.BattlePaused and `!(flags & 0x04)`.
- **Skips an object if `battle_isTimeStop()` and `!(flags & 0x10)`.** So T3 attacks (flags 0x09) stand still while
  dimmed. T1/T4 objects keep running and gate themselves.
- Otherwise it dispatches `jumptable[type & 0xF][Index]` on `CurState` 0 (init), 4 (update) or 8 (destroy).

The player navi is T1 type 0: `sub_80B81EC` → ActorType 2 → `sub_80EA460` → state 4 → `sub_80EA484`.

**`sub_80EA484`** runs these steps in order:

| Step | Call | Purpose |
|---|---|---|
| 1 | `sub_8012E74` | request flags |
| 2 | `sub_8013DA0` | |
| 3 | `sub_801AC6C` | computes FinalDamage |
| 4 | `sub_801AF44(off_80EA4C8[AIIndex])` | action dispatch |
| 5 | `off_80EA93C[AIIndex]` | per-form hook |
| 6 | `sub_80107D4` | lockouts |
| 7 | `sub_80139C4` | |
| 8 | `sub_80100EC` | |
| 9 | `object_presentCollisionData` | only when not paused |

**`sub_801AF44`** (0x0801AF44):
- First processes pending hits (`applyDamageToPlayer_801ba12`, flinch branches, …).
- Then, at the tail:
  - if the dead flag 0x100 is set → `sub_801B9E6`;
  - else if paused and CurAction ≠ 0 → `sub_8017BC0`;
  - else if **dimming** → **`sub_8017AB4`** (cut-in; the action handler does *not* run);
  - else → `sub_801B9E6`.
- Exception: while `sub_801032C() & (0x2000|0x10000|0x20000)`, dispatch goes straight to `sub_801B9E6`.

### 3.2 Attack slots: `object_setAttackN` and exit

`object_setAttack0..5` (0x08011680 + 4·N, common tail `loc_8011698`) all do:
- `CurAction = r0`, `obj.u16[0xA] = 0` (CurPhase + PhaseInitialized);
- `av.u16[0] = 0`, `av[0x1C] = N`;
- `sub_801011A`, which sets `av[0x1D] = 0`, `av.u16[0x1E] = 0` and calls `sub_80E1662`.

| Slot | Used by | At exit (§2.8) |
|---|---|---|
| 0 | reactions / system actions (hit 3, 0x1C cross-change, 0x30, 0x49, 0x4C, 0x4D, 0x59) | normal cleanup |
| 1 | buster 0x11, charge shot 0x16, 0x33, request 0x10000000 | normal cleanup |
| **2** | **chips** (`sub_800FB54`, `sub_800FC30`); charge shots with `ai[7]` in 0x21..0x26 | `ai[0x19] = av[5]` (chip lockout) |
| 3 | special (flag 0x10) | `ai[0x15] = av[5]` |
| 4 | movement 0x10 (`sub_80116AE`/`sub_80116D8`), 0x3B | no cleanup except CurAction/phase |
| 5 | cross/beast auto-chip (`sub_802D4F0`) | the chip is **not** consumed (`sub_80F0354` skips `sub_800FC7C`) |

### 3.3 Anatomy of an action handler

Every `JumpTable80EAC60` entry has this shape. It is called with r5 = obj, r7 = av, r6 = AIState and r4 = ai.

```
handler:      phase_table[av[0]]()          // av[0] = 0, 4, 8, … (byte offset into a word table)
phase fn:     if av[1] == 0 {               // first frame of the phase
                  object_setAnimation(n); object_setDefaultCounterTime();
                  spawn "arm" visual; object_setFlag1(0x400000); timers in av+0x10/+0x12; av[1] = 4 }
              count timer; at fixed ticks call a spawner; when done: av.u16[0] = next phase   // also clears av[1]
last phase:   timer; then obj.RelatedObject1Ptr = 0, ai.u32[0x68] = 0 (release attachments); object_exitAttackState()
```

- **Handler frame 1 is the frame after `object_setAttack2`.** Dispatch happens once per object per frame, and the
  use itself happens inside the idle handler.
- **`object_setDefaultCounterTime`** (0x0800FDB6) calls `object_setCounterTime(0x10)` (0x0800E9DC), which sets
  `CollisionData+0x0D` (CounterTimer) = 16. For players this happens only when `BattleEffects & 8` (network battle).
  While it is nonzero, an incoming hit can register as a counter (§3.5).
- **The "arm" visual** is `sub_80B8E30` (0x080B8E30): a T1 type-5 object.
  - Params = variant; header flags |= 0x14, so it runs during pause and dimming.
  - It stores itself at the slot pointer given in r7: `&obj.RelatedObject1Ptr` or `&ai.u32[0x68]`.
  - It follows its owner every frame. When the slot is zeroed it goes to CurState 8 and is freed on the next frame.
  - It has no collision, but it occupies a T1 slot and a place in the update order, so it is required for
    exactness.
  - Hit-reaction actions (for example `sub_80178D4`) also zero `ai.u32[0x68]`, so an interrupted chip cleans up
    the same way.

Representative handlers, all code-derived. Frame counts assume the attack is not interrupted **[not trace-checked]**:

| Chip (action → handler) | Frames | Timeline |
|---|---|---|
| Cannon/HiCannon/M-Cannon (0x14 → `sub_80EBC0E`) | 33 | f1: anim 8, counter time, arm (Params `sub_80EBD68()<<8 \| byte_80EBD2C[av3]`), flag. **f16** (timer 0xF): sound 0xAE, then `sub_80C4FFE(x+front, y, r2=av+6, r3=0x180000, r4=byte_80EBD34[av3], r6=av.u32[8]+av.u16[6])` → T3 type 0 shot (descriptor 1). TankCan adds smoke at timer 8. f30: release arm, anim 7, timer 3. **f33**: exit. |
| Vulcan1..3/SuprVulc (0x17 → `sub_80EBF10`) | 36 for Vulcan1 | f1: anim 0xA, arm Params 0xD into `ai+0x68`. f3: shots `av+0x12 = dword_80EBFEC[av3]` = {3,4,5,10}; a shot every 11 frames from f3 (Vulcan1: f3, f14, f25). **Each shot draws `GetPositiveSignedRNG2() & 3`** to pick Z from {8,0x10,0x18,0x20}<<16 (visual height only, but it advances RNG2). Each shot calls `sub_80C6ADA(x+front, y, av+2, Z, av.u32[0xC], av.u32[8]+av.u16[6])` → T3 0x12. Then a 1-frame recovery init, 10 more frames, and exit. |
| Sword family (0x13 → `sub_80EB776`) | 30 | Phase 0 returns at once (or does a step-sword advance if `params` byte 0 ≠ 0). Phase 1 sets hits = 1 (2 when `av3 == 0xB`). f3: anim 5, sound 0xB0/0xCE, arm, timer 0x15. **f12**: `object_spawnCollisionRegion(x+front, y, av+2, 0, r4=byte_80EBA18[av3], r6=av.u32[8]+av.u16[6], r7=off_80EBA00[0][av3])` plus a T4 slash visual. f25: recovery (timer 5). f30: exit. |
| Instant (0x1C → `sub_80EC39C`) | 1 | `off_80EC3F0[av3](panelX, panelY, av+2, obj.Z, av.u32[0xC], av.u32[8] + (u8)av[6])`, then exit in the same frame (`av3 == 0x14` waits 8 frames). **Only the low byte of the bonus is added.** |
| Dimming chips (0x15 → `sub_80EBD9C`) | whole dimming | §3.6 |
| Navi chips (0x1B → `sub_80EC350`) | 1 | `sub_80E192C(panelX, panelY, av+2, av3, av.u32[0xC], av.u32[8], chip \| av6<<16)` spawns T4 0x10 (summon controller → `off_802CD5C[subtype]`). Registers the dimming exactly as 0x15, then exits **in the same frame**. |
| GunDelSol (0x37 → `sub_80EDAE0`) | 140 (S3) | §4 |

### 3.4 Spawner register convention and object fields

Chip spawners are called with:

| Register | Content |
|---|---|
| r0, r1 | panel X, panel Y |
| r2 | element byte (`av[2]`) |
| r3 | Z or subtype |
| r4 | Params word (object +0x04..+0x07 = Param1..4) |
| r6 | **damage word** = `(av.u16[8] + av.u16[6]) \| cd.hit_param << 16`, i.e. `av.u32[8] + av.u16[6]` |
| r7 | (T4 / dimming chip spawners) `chip \| bonus << 16`, stored at object +0x30 / +0x32 |

So **the damage an attack object carries is `hand dmg | modifier flags` plus the Atk+/cross bonus.** The handler
adds the bonus at spawn time. Handlers that ignore `av+8`/`av+6`, such as GunDelSol, cannot be boosted.

Field writers:
- `sub_801155A` (0x0801155A), used by `sub_80C4FFE`/`sub_80C6ADA`, sets:
  - `PanelX = r1`, `PanelY = r2`, `Element (+0x0E) = r3`;
  - `Damage u32 (+0x2C) = r6` (low u16 = damage incl. flag bits, high u16 +0x2E = `hit_param`);
  - Alliance/DirectionFlip from the parent, `RelatedObject1Ptr = parent`.

  In `sub_80C4FFE` the r3 that reaches Element is actually the caller's r2 (`av+6` for Cannon). This is harmless,
  because the T3 type-0 init overwrites Element from its descriptor.
- **`object_spawnCollisionRegion`** (0x080C536A) spawns **T3 type 3** (`sub_80C52B0`), a generic one-frame hitbox:
  - PanelX/Y, `Element = r2`, `Damage u32 = r6`, alliance/flip from the parent;
  - `ExtraVars[0..3]` = bytes of r7, `ExtraVars[4] = 0`;
  - Param1 = region shape, Param2 = hit effect, Param3 = target collision type, Param4 = self collision type.

### 3.5 From attack object to CollisionData

**`object_setupCollisionData(cd, self_type, target_type, hitmod)`** (0x08019FB4) copies:

| CollisionData field | Value |
|---|---|
| +0x38 ParentObjectPtr | obj |
| +0x0E HitModifierBase | hitmod |
| **+0x02 PrimaryElement** | `obj.Element & 0x0F` |
| **+0x19 SecondaryElement** | `obj.Element & 0xF0` |
| +0x04 | alliance/flip |
| +0x0A | panel XY |
| +0x01 Region | 1 |
| **+0x07** | low byte of obj+0x2E (`hit_param`) |
| **+0x2E SelfDamage** | `obj.Damage` (u16 +0x2C, including the flag bits) |
| **+0x30 SelfCollisionTypeFlags** | `sub_801A0BA(alliance, self_type)`, i.e. `byte_8019C7C[type*8 + alliance*4]`, **\| 0x10000 if created while dimmed** |
| +0x34 TargetCollisionTypeFlags | `sub_801A0BA(alliance, target_type)` |

Per-object additions:
- **T3 type 3 region.**
  - Region = Param1, and `object_setCollisionHitEffect(Param2)` (CD+0x09).
  - `ExtraVars+4 ≠ 0` → `object_setCollisionStatusEffect1` (CD+0x10).
  - `ExtraVars+8 ≠ 0` → `sub_801A4D0(EV+8, EV+0xC)`.
  - Then `object_presentCollisionData`, and in the same frame `object_removeCollisionData`, so it is a
    single-frame hitbox.
- **T3 type 0 shot** (`sub_80C4E58`) takes its setup from descriptor `off_80C4C78 + 12·Param1`:
  `[0]` self type, `[1]` target type, `[2]` hitmod, `[3]` element, `[4]` hit effect, `[9]` status, `[0xA,0xB]`.

  | Descriptor | Bytes |
  |---|---|
  | 0 (buster) | `04 05 00 00 00 ff ff 00 ff 00 00 00` |
  | 1 (Cannon) | `04 05 03 00 07 …` |
  | 0xC (TankCan) | `04 05 03 00 0b …` |

- **T3 0x12 Vulcan bullet** (`sub_80C6946`): setup(0x3D, 5, 0), then forces PrimaryElement = 0.

**Resolution** (summary; the damage/collision spec owns the details):
- `object_presentCollisionData` (0x0801A018) registers the object's `CollisionIndexBit` in the per-panel mask
  table `unk_2034F60` for each panel of its Region shape (`sub_300777C`). When not dimmed it first clears the
  previous frame's accumulators, CD+0x68..0xA7.
- `object_removeCollisionData` (0x0801A00E) → IWRAM `_object_removeCollisionData` (0x03007550) is **where attacker
  and target are paired**. It runs `sub_30075FC`/`sub_3007708` → `sub_3007218` + `sub_3007692` against every
  collision registered on those panels and not yet paired.
  - A projectile presents at the end of its update and resolves at the start of its next one.
  - A T3 type-3 region presents and removes within its own init.
- Effects on the target (IWRAM ~0x0300734C..0x0300741A):

  | Target field | Update |
  |---|---|
  | FlagsFromCollision | `\|=` attacker +0x30 |
  | +0x76 | `\|=` attacker SecondaryElement |
  | StatusEffectFinal | attacker +0x10 (if nonzero) |
  | HitModifierFinal | `\|=` attacker +0x0E |
  | `PanelDamage[primaryElement]` = CD+0x82 + 2·elem | `+=` `SelfDamage × (1 + weakness terms)` |

  With `b = attacker CD+0x07` (hit_param):
  - if target CounterTimer ≠ 0, `b & 0x7F ≠ 0` and `!(b & 0x80)`: **counter hit**, i.e. `FlagsFromCollision |= 0x40`
    and target CD+0x90 = 0x8000;
  - otherwise CD+0x90 += `b & 0x7F`;
  - in both cases CD+0x8E += `b & 0x7F`.

  The meaning of this accumulator is **[unverified]**.
- The target applies HP loss in its **own next update**: `sub_801AC6C` → `object_calculateFinalDamage1` (sums
  buckets for elements 0–4) → `applyDamageToPlayer_801ba12`. For chips this gives a 1-frame latency between hit and
  HP change. Element 5 bypasses FinalDamage entirely (§4.4).

**Chip implementer's summary.** Damage = `av+8` (+ `av+6`), carrying flags 0x8000 double, 0x4000 paralyze,
0x2000 uninstall, 0x1000 erase and 0x0800 nothing (see `oBattleObject_Damage` in `BattleObject.inc`).
Element = `av+2`, unless a descriptor overrides it. `hit_param` = the counter/stagger byte. All three travel as
object +0x0E/+0x2C/+0x2E into CD+0x02/+0x19, CD+0x2E and CD+0x07.

### 3.6 Dimming chips (action 0x15; navi chips 0x1B)

**No dimming executes in the machgun trace** (§4.6). The soundmod trace has them; its first, Invisibl (chip 0xB1,
subtype 1, T4 0x5D) at round-1 frame 3206, verifies the sequence below (§3.6.6). Cut-ins and the navi chips' own
phases (`sub_800BDB2`, `sub_800BA8A`) are still code-derived.

#### 3.6.1 State

- **Dimming** is `BattleState+0x32` bit 2 (value 4): `battle_isTimeStop` (0x0800A098), `battle_setFlags`
  (0x0800A2D8), `battle_clearFlags` (0x0800A2E4).
- Per-alliance dimming state is at `byte_203CF00 + 0x50*alliance` (`sub_800BF5C`, 0x0800BF5C):

| Off | Meaning |
|---|---|
| +0 | u8 current owner alliance, written into both structs (`sub_800B8AC`) |
| +1 | u8 state: 0 idle, 1 registered, 2 showing name, 3 waiting (the other side cut in), 4 effect running, 5 ending |
| +2 | u8 1 if chip id ≥ 0x170 (cannot be cut in on) |
| +3 | u8 alliance that started the dimming (both structs, `sub_800BF16` only) |
| +8 | u32 dimming controller object |
| +0xC | u32 user object |

#### 3.6.2 Use sequence (`sub_80EBD9C`, 0x080EBD9C)

| Frame | Event |
|---|---|
| N | Idle uses the chip: `setAttack2(0x15)`, chip consumed. |
| N+1 | Handler frame 1 (`av[1] == 0`), described below. |

Handler frame 1:
1. `ctrl = off_802CCB4[av3](r0=panelX, r1=panelY, r2=av+2, r4=av.u32[0xC], r6=av.u32[8], r7=chip | av6<<16)`.
   - The controller is a T4 object. For example Geddon (subtype 3) uses `sub_80E2566` → **T4 0x1D**, with
     `Params = cd.params` (Geddon1 Param1 = 1), `Damage = r6`, and +0x30/+0x32 = chip/bonus.
2. If own `[8] == 0`: `sub_800BF16(alliance, chip ≥ 0x170, ctrl)` (0x0800BF16).
   - Sets `[3] = alliance` and `[0] = alliance` in both structs.
   - Kills any previous controller (CurState 8).
   - Then sets `[2]`, `[8] = ctrl`, `[0xC] = user`, `[1] = 1`.
3. `av[1] = 1`.
4. Later the same frame, the controller (inserted after the user) runs its init `object_timefreezeBegin` (0x0800B916).
   If `[3] == own alliance` it calls **`battle_setFlags(4)`**: the dimming starts mid-frame. Later objects without flag 0x10
   are skipped for the rest of this frame.

From N+2 the user navi is gated into `sub_8017AB4` and stays in CurAction 0x15 for the whole dimming. The controller
(for T4 0x1D, `sub_80E24E4` → `sub_80E2504`) steps through:

1. **`object_dimScreen`** (0x0800B94C): `SetScreenFade(0x3C, 4)`, mode stepper `sub_800647C` (+4 per update until
   ≥ 0x40). From level 0 this takes 16 updates, finishing at the end of N+17. At N+18 the fade is idle and the
   controller moves on. The start level `eScreenFade+6` is history-dependent; it was 0 in the fights observed.
2. **`object_drawChipName`** (0x0800B9B0), from N+19:
   - Own `[1] = 2`. If opponent `[1] ∉ {0,3}`, retry next frame.
   - Else start the banner: `sub_801E792(0x4C or 0x50, dmg, bonus, chip)`, with damage shown only if flags bit 1.
     Sound 0x173.
   - The banner (`byte_2036840`, stepped by `sub_801CE28` from `sub_801BEE0`) runs slide-in 5 frames, hold until
     counter 0x30, slide-out 5 frames; it is disabled on the next update.
   - `sub_801E754` returns 0 at about N+78.
   - Then: if the other side cut in (own `[0] ≠ own alliance`) and opponent `[1] ∉ {0,5}`, set `[1] = 3` and retry.
     Otherwise `[1] = 4`. If `[2] == 0` and the user's HP is 0, the effect is skipped.
3. **The chip's effect**, from about N+79. Geddon (`sub_80E2528`): reserve the user's panel, spawn helper
   **T4 0x1E** (`sub_80E2712`, level = Param1), and wait for it. Panel tables are at 0x080E2588.
4. **`object_undimScreen`** (0x0800BC88), at effect end E+1:
   - If opponent `[1] ∈ {0,5}`: `SetScreenFade(0x38, 4)` (`sub_8006366`: no change on the first update, then −4 per
     update). From 0x40 that is 17 updates, and CurState becomes 8 at E+18.
   - Otherwise it goes to CurState 8 immediately.
5. **`object_timefreezeEnd`** (0x0800BD34), at E+19: own `[1] = 5`.
   - If opponent `[1] ∈ {0,5}` and `[3] == own`: kill the opponent's controller, clear opponent `[0xC]`,
     `sub_800B89C(opp)`, then **`battle_clearFlags(4)`** (time resumes mid-frame), then clear own `[1]`, `[8]`,
     `[0xC]` and free.
   - Otherwise retry each frame.
6. At E+20 the user navi runs `sub_80EBD9C` again. `av[1] ≠ 0`, so it calls `object_exitAttackState` and is idle
   from E+21.

`isSameSubsystem_800A732` might skip fade updates on some frames; this was not checked.

#### 3.6.3 What runs while dimmed

**Frozen:**
- T3 objects;
- navi action handlers (damage intake before the gate in `sub_801AF44` still runs);
- `sub_800BFC4` (custom gauge);
- the battle frame counters (BattleState+0x0E/+0x16);
- the `sub_80107D4` lockouts.

**Running:**
- T1/T4 objects (arms, controllers);
- the banner (`sub_801BEE0`);
- input latching into the separate dimming edge set (§2.6.1).

Collisions created while dimmed carry SelfCollisionTypeFlags `| 0x10000`.

#### 3.6.4 The dimming flag vs the action

- A dimming chip is `flags & 1`. Most dimming chips use action 0x15 (84 chips); navi chips use 0x1B.
- A few action-0x15 chips lack bit 0: PunchArm and the other Arm chips, IceCube, and the capsules. They still
  dim the screen when used normally, but can't cut in.
- `sub_80F0354` shows the opponent telop (`sub_801EB18`) only for non-dimming chips. This is presentation only.

#### 3.6.5 Cut-in (network battles only)

**Request.** In the dimming branch of `sub_8012FC8`, the following must all hold; then `ai+0x44 |= 0x800`:
- `sub_800A772`;
- no 0x800 already;
- a next chip exists;
- `ai.u16[0x2C] & A`.

**`sub_8017AB4`** (0x08017AB4) runs on every frozen frame:
1. Requires a player navi, `BattleEffects & 8`, and flag 0x800.
2. Requires `sub_800BEDA()`: own `[0xC] ∈ {0, self}`, own `[1] ∈ {0,3}`, **opponent `[2] == 0` and opponent
   `[1] == 2`**. So only during the opponent's name banner, and only if their chip id is < 0x170.
3. Requires the next chip to be a dimming chip (`flags & 1`).
4. `sub_80127C0(0)` with **r7 = a 0x50-byte stack buffer**, so the navi's real `av` and CurAction are untouched.
5. If the action is 0x15 → `off_802CCB4[tmp[3]]`. If 0x1B → `sub_80E192C`.
6. `loc_800BF30(alliance, 0, ctrl)`: registers like `sub_800BF16` but does not write `[3]`. Ownership `[0]` moves to
   the side that cut in.
7. `sub_800B8EE`: a T4 flash (id 0, Params 0x1E) plus sound 0xA5.
8. **`sub_800FC7C`** (consume).
9. Clears 0x80C, on every path.

Ordering, inferred from the state machine **[unverified]**:
- The controller of the side that cut in shows its telop and runs its effect first.
- The original waits in state 3, then runs its effect.
- The dimming ends when the initiator (`[3]`) ends, i.e. LIFO.

Special cases:
- AntiNavi checks (`sub_802CE78(opp) == 0xBA`) in `sub_800BA8A`/`sub_800BDB2` for navi chips 0xDD..0x118.
- Alliance-swap re-registration in `sub_800BE2C`.

**Quirk.** The r4 value entering `sub_80127C0` from `sub_8017AB4` selects hand vs slot-in (§2.6.4). It was not
checked at run time. **[unverified]**

When the checks fail (e.g. A pressed while the other side's screen is still dimming, soundmod 3217), `sub_8017AB4`
just clears requests 0x80C; the navi goes on shaking as usual.

#### 3.6.6 Trace: Invisibl (soundmod round 1)

Side 0 uses Invisibl (params 0x168) from idle at 3206 (action 0x15, the chip consumed).

| Frame | Event |
|---|---|
| 3207 | Handler frame 1: the controller T4 0x5D is spawned right after the user and registered (`[1] = 1`); its init (`object_timefreezeBegin`) sets battle flag 4 the same tick. |
| 3208–3224 | `object_dimScreen`: its first update starts the fade (timer 1); the fade is idle on the 17th update (3224, timer 17): **16 ticks**, action 4. |
| 3224–3283 | `object_drawChipName`: `[1] = 2`, banner 0x4C (the local player's; 0x50 for the remote one) and sound 0x173 at 3224; the banner is done after 60 ticks (3284). |
| 3284 | `[1] = 4`, action 8 (the user is alive). |
| 3285–3315 | The effect (`sub_80E7518`): at 3285 `sub_8010474` on the user: CollisionData+0x24 (the flash timer) = 0x168, ObjectFlags1 \|= 2 (INVISIBLE), sound 0x93; timer 0x1E counts to −1 at 3315: action 0xC. |
| 3316–3333 | `object_undimScreen`: the other side is idle, so the fade 0x38 starts at 3316; idle at 3333 (**17 ticks**): state 8. |
| 3334 | `object_timefreezeEnd`: `[1] = 5`, then (initiator) battle flag 4 cleared, both records cleared, the controller freed. The user ran earlier that tick, still while dimmed. |
| 3335 | The user runs `sub_80EBD9C` again → `object_exitAttackState`; the flash timer now counts (FLASHING, blinking VISIBLE) for 360 ticks. |

Other facts:
- `sub_80E7546` spawns the controller with r1..r3 left as the caller's panel Y, element and the spawner's own
  address, so its X, Y, Z are that garbage (X = 2, Y = 0, Z = 0x080E7547 here). Nothing reads them.
- `dword_200F3B8[alliance]` (cleared by `object_timefreezeBegin`, set by `sub_800BA8A`) has no reader.
- The rocks (T3) and every object without flag 0x10 stand still for the whole dimming.

#### 3.6.7 Navi chips (action 0x1B): the controller, the warp and ElmntMan

Trace: soundmod, side 0 uses ElmntMan (chip 0x10D, subtype 0x10, params 0x10) in rounds 1 (3407) and 2 (25776).

**Action 0x1B, `sub_80EC350`**, runs once: `sub_80E192C` spawns the controller (r0/r1 the user's panel, r2 element,
r3 subtype, r4 params, r6 damage word, r7 chip | bonus << 16), registers it like action 0x15 (if the side has no
controller yet), and `object_exitAttackState` at once: the user idles (gated by the dimming) while its navi acts.
With subtype 0 and the other side's defensive chip 0xBD, `sub_80E192C` does something else (not ported).

**The controller, T4 0x10 (`sub_80E17E8`).** Spawned with r1..r3 = panel Y, element, subtype as its position (so
Z = the subtype; register garbage nothing reads). Object +0x19 = the subtype (which navi, `off_802CD5C`), +0x18 is
a flag its navi clears. Actions: 0 `object_dimScreen`; 4 `sub_800BDB2` (AntiNavi: for chips 0xDD..0x118 when the
other side's defensive chip is 0xBA, the navi is sent back; otherwise straight on); 8 `sub_800BA8A` (the name, as
`object_drawChipName` except that it skips the cut-in check, and the effect only when the user is deleted);
0xC `sub_80E1830`; 0x10 `object_undimScreen`. `sub_80E1830`'s phases:
- 0 (`sub_80E1854`): 30 ticks; at its start the user warps out (`sub_80C0F52(user, 1)`), except for navi 0x17.
- 4 (`sub_80E1880`): the navi's spawner `off_802CD5C[+0x19]` with r5 = the user, r4 = the params, r6 = damage +
  bonus, r7 = &controller+0x18 (the spawner sets it to 1). Chips 0xDD..0x118 are also recorded at `byte_203C960`
  (for chips that copy the last navi chip). Then it waits while +0x18 is set.
- 8 (`sub_80E18DA`): 30 ticks. 0xC (`sub_80E18F8`): the user warps back in (except navis 0 and 0x17), 30 ticks.

**The warp, T1 0x2D (`sub_80C0E04`), Param4 = 1 out / 0 in.** For a player: the navi's sprite (navi, form), its
palette and position, VISIBLE, the form's overlay stepping even while paused (`sub_8011420(navi, form, 1)`: for
MegaMan `sub_8011268(form, 1)`, stored in the warp's own RelatedObject2Ptr), CurAnim = 3 + Param4. Update: on the
first tick a warp out hides the user (`sub_80E1352(user, 0)`: VISIBLE off, and the confusion/blindness visuals,
AIData+0x60/+0x58 objects, the Full Synchro aura and the HUD with it); after 4 ticks: VISIBLE off, the NameID's death
hook on the warp (`sub_8011044`: takes its overlay down), a warp in shows the user again (`sub_80E13DC`: VISIBLE
unless submerged or hidden by the viewer's blindness), state 8. Its sprite steps while dimmed.

**ElmntMan, T1 0x10 (`sub_80BAA8C`)**, spawned by `sub_80BAE16` on the user's panel, with the user's side, the damage
word, and the controller's flag pointer **in its CollisionDataPtr slot** (so a trace reading ObjectFlags1 through it
reads 0). Init: sprite (8, 0x10), animation 0, his overlay (`sub_8010DF6(2, 0x10, 1)` → `sub_8011004`: T1 0x56
variant 0xF, own palette, Param3 1, animation offset 9). Actions (each with a timer; "n ticks" counts to 0):
- 0: anim 3, VISIBLE, sound 0x94; 2 ticks; if his panel has flags 0x10010 → 4, else 0x18.
- 4: anim 0; 10 ticks.
- 8 (`sub_80BABAC`): cycle the elements Fire, Aqua, Elec, Wood (palettes 2, 4, 8, 6; sound 0x134) every Param1 ticks;
  the user's A while dimmed (AIData+0x2C, the dimming pressed keys) picks the one shown (sound 0x182); after
  Param1·20 ticks, **`GetPositiveSignedRNG2() & 3`** picks one.
- 0xC: anim 5 (7 at 16 ticks left); 35 ticks; Wood first turns every solid panel to grass. Then 0x10, phase = the
  element · 4.
- 0x10: Fire (`sub_80BACBC`) lists the panels holding the other side's body (`object_getPanelsExceptCurrentFiltered`,
  rows 3..1, columns 6..1; Param2 = how many, Param3 counts) and drops a meteor on each, 12 ticks apart. Aqua
  (`sub_80BAD06`): T3 0x8E ice (Param2 30, Param3 1) on the three panels of the column ahead, each solid one turned
  to ice; sound 0x99; 30 ticks. Elec (`sub_80BAD76`): a T3 0xB8 bolt in each row of the column three ahead, 16 ticks
  apart. Wood (`sub_80BAD34`): a T3 0xB9 vine on the next panel along his row every 15 ticks, up to five, until a
  panel isn't solid (or no vine spawns). The grass before it (`sub_80BAF06`): every solid panel (columns 1..6, rows
  1..3) turns to grass with a T4#0 effect 2 (palette +5) at Z 8 (raw), sound 0x11B.
- 0x14: 20 ticks (anim 8 at 5 left). 0x18: anim 4; 2 ticks; the death hook takes the overlay down, the controller's
  flag is cleared, state 8.
- Round 1: spawned 3518, A at 3540 (Fire), the meteor 3578, gone 3602; the controller's undim ends 3683.

**The meteor, T3 0x8D (`sub_80D6BD4`)**, spawned by `sub_80D6D18` with flags \|= 0x10 and Param1 = 1 from
ElmntMan (it acts, and steps its sprite, while dimmed), 0 from his navi AI (it waits a dimming out, its sprite
stepping as `object_updateSpritePaused` does): from 192 pixels behind and above its panel it falls 11 pixels a tick
for 17 ticks (sound 0xC4; the panel highlighted 4 ticks out of 8); on landing, if the panel's flags meet
`byte_80D6D08[side]`, a T4#0 explosion and a hit region (region 1, hit effect 1, target 5, self 0xA, hit modifier 3;
with Param1 set `sub_80C53A6` gives the region flag 0x10 so it resolves while dimmed). The damage lands while dimmed;
the victim's flinch waits for the time to start again (3684).

**ElmntMan's ice, T3 0x8E (`sub_80D6D80`)**, spawned by `sub_80D6EB0` (flags \|= 0x10; its Z is the spawner's r3,
of which the init keeps the fraction under a height of 8): sprite (0x10, 0xF) animation 1; collision self 4, target 5,
modifier 3, status 0x50, hit effect 2, region 0 until it acts: then region 1, its panel turned to ice, Param2 ticks.
Each tick the hits resolve and a hit clears the region (and the hit flags). Param3 1 (ElmntMan's) acts and steps its
sprite while dimmed; Param3 0 (his navi AI's) waits a dimming out with its collision off.

**ElmntMan's bolt, T3 0xB8 (`sub_80DC3F8`)** and **vine, T3 0xB9 (`sub_80DC4FC`)**: on their panel on the ground
(their Z keeps its fraction: the bolt's is garbage from the object loop, the vine's the row minus 1 that its
spawner's panel check leaves in r3), running while dimmed. On their first action tick a hit region that resolves
while dimmed (region 1, target 5, self 0xA; the bolt hit effect 3, modifier 3, the vine hit effect 4, modifier 1);
the bolt breaks its panel (`object_breakPanel_dup2`: cracks it if something occupies it), sound 0x12E, and shows 16
ticks; the vine (ElmntMan's sprite, animation 0x12), sound 0x181, 30 ticks.

All of these are the pack's scripts (objects/elmnt-man, meteor, elmnt-ice, elmnt-bolt, elmnt-vine). The navi AI's
meteors and ice (Param1/Param3 0) are ported but no trace reaches them (unverified).

**Cut-ins.** `sub_8017AB4` also needs the next chip to have the dimming flag; soundmod 25828 (side 1 presses A
during ElmntMan's name with FullCust next) clears the request.

**EraseMan, T1 0x15 (`sub_80BB608`, navi 5)**, spawned by `sub_80BB7F6` like ElmntMan (no overlay). His actions
enter on CurPhase 0 (setting it to 4) and store Timer and Timer2 together:
- 0: anim 3, sound 0x94, VISIBLE; 3 ticks; panel flags 0x10010 → 4, else 0x14.
- 4: anim 0, 30 ticks. 8 (`sub_80BB710`): aim. His row picks the aims he cycles through (`byte_80BB88C`: row 1
  forward, down-forward; row 2 up-forward, forward, down-forward, forward; row 3 up-forward, forward); each aim marks
  up to 5 panels along its step (`byte_80BB860`: (1, −1), (1, 0), (1, 1), x toward the front; the line stops at the
  field's edge) with T4 0x62 marks lasting Param1 ticks (sound 0x10E). The aim changes every Param1 ticks; the user's
  A while dimmed, or 360 ticks, ends it.
- 0xC: anim 0x11, 10 ticks. 0x10 (`sub_80BB79A`): anim 0x12, a T3 0xC3 segment on each panel of the line (Param1
  the aim, Param2 60 ticks), sound 0xBA; 60 ticks. 0x14: anim 4; 3 ticks (to −1); the controller's flag cleared,
  state 8.
- The mark, T4 0x62 (`sub_80E78BC`): on its panel, Z = its spawn Z's fraction (the 1 `sub_80BB81C` leaves in r3);
  sprite (0x10, 0x50); gone after Param1 ticks (or when the battle is over).
- The segment, T3 0xC3 (`sub_80DD940`): 8 pixels behind its panel's centre at height 35/40/45 by aim, anim 1/0/2;
  collision self 0x16, target 5, modifier 1, status 0x10, hit effect 0xC. Each tick the hits resolve; a hit turns
  its region off; after Param2 ticks it ends. With Param3 set it would end with its owner's action 0xA, and stand
  still while dimmed.
- Round 1: side 1 (DustCross) uses EraseMan at 4058; aimed at 4166, slashed 4186, the beams gone 4246.

#### 3.6.8 AreaGrab and PanelGrab (subtype 0, T4 3, T3 0xF)

The controller (`sub_80E0710`) runs `object_drawChipName`; its effect (`sub_80E0754`): Param1 set (AreaGrab, params
1): `sub_800D5BA` finds the nearest column, going back from the user's (then forward), wholly the user's side's, and
`sub_800D58C` the first column in front of it not wholly the side's; a grab shot on each of its three panels.
Param1 0 (PanelGrab): `object_getEdgePanelMatchingRow` (from the far edge, back while the panels are the user's
side's, then on to the first with the side's flags, then one forward): one shot. Then 61 ticks.

The shot (`sub_80C6414`, spawned with Param1 = the side and flags \|= 0x10): from 256 pixels up (sound 0xA1) it falls
8 pixels a tick (32 ticks); landed: a hit region (region 1, hit effect 0xFF, target 5, self 0xA, modifier 1, flag
0x10), and the panel changes side if its flags meet `byte_80C6514` (the other side's, nothing on it) and
`sub_800D668` agrees (the victim keeps another whole home column on that side of it before one it already lost);
the column's return timer is set to 0x708. Then anim 1 (sound 0xA2) until its animation ends. Its sprite is stepped
twice a tick (`object_updateSprite` and `object_updateSpriteTimestop`).

#### 3.6.9 AntiDmg's dimming (subtype 20, T4 0x2A)

The controller (`sub_80E34C0`) names the chip with `sub_800BBA8` (the remote player sees chip 0x171; the user's
survival alone decides the effect). Its effect (`sub_80E3504`): `sub_802CEA6` clears the side's defensive-chip record
(its object gets Param2 = 1), `sub_80E3560` spawns the trap's object for Param1 0 only (AntiDmg's params are 3: none),
and `sub_802CE8A` records {chip, bonus, damage word, user, object} (0x10 bytes per side at 0x02036720). Then 61 ticks.
`sub_802CEC8` clears a record every tick once its user's HP is 0. The trap springs in the damage intake
(`sub_802CEF4`).

#### 3.6.11 SpoutMan (navi chip subtype 7, T1 0x09)

Trace: soundmod rounds 1 and 2 (side 0's SpoutMan); the scratch lab's navis/0x0f2-spoutman/long{,-miss,-adjacent,
-holes} and the EX and SP `long`s match every frame. The pack's scripts: objects/spout-man, spout-ball,
spout-splash, spout-pillar, spout-geyser, spout-mark.

**SpoutMan, T1 0x09 (`sub_80B94BC`)**, spawned by `sub_80B9750` like ElmntMan (the controller's flag pointer in his
CollisionDataPtr slot). His position is the spawner's registers: X, Y = panel Y and element (overwritten), Z = r3 =
the spawner's own address `0x080B9751`, whose low half his init keeps (a halfword store of 0 over Z's whole part).
Sprite (8, 6) with a ground shadow; his actions enter on CurPhase 0 and step his sprite while dimmed:
- 0: anim 3, sound 0x94, his parts (`sub_8010DF6(2, 6, 1)`: T1 0x55, his layer), VISIBLE, Timer:Timer2 = 6 (a word);
  6 ticks; panel flags 0x10010 → 4, else 0x18.
- 4: anim 0, 30 ticks; away from his back columns (side 0: x > 2; side 1: x < 5) → 8 (the ball), else 0x10 (the
  geyser).
- 8: anim 0x13, 20 ticks. 0xC: anim 0x14, a T3 0x22 ball from his panel (Param1 4), a T4#0 effect 0x2A where he
  stands (flip = alliance ^ flip), sound 0x12D; 60 ticks → 0x18.
- 0x10: anim 0xF, a T4 0x2D pillar on his panel (Param1 4; `sub_80E3976` stores it in his ExtraVars[0] and keeps a
  pointer to that slot); 66 ticks: anim 0x10 at 57 left, the pillar's PhaseInitialized byte = 1 at 27 left.
- 0x14: anim 0x15, sound 0x189, a T3 0x17 geyser (Param1 4, Param2 90, Param3 = the column `sub_80B9776` picks: the
  first ahead holding an enemy navi's body, else the field's edge; r3 = 4 becomes its Z); 90 ticks, then anim 0 and
  the pillar's PhaseInitialized = 2 → 0x18.
- 0x18: anim 4, Timer:Timer2 = 8; to −1 (9 ticks); his parts off (`sub_8011044(2, 6)`), the controller's flag
  cleared, state 8.

**His layer, T1 0x55 (`sub_80C40D8`)**, the navi framework's (`kinds::navi_layer`, from `kinds::navi_parts`): sprite
(0x10, 0x21) (`dword_80C40D4[Param1]`, Param1 always 0); each tick his position, visibility, palette, colour
shader, white flash, mosaic, facing and alpha, and Z's whole part 0 when his animation is 0, else 255 (out of
sight); its sprite steps unless dimmed or paused.

**The ball, T3 0x22 (`sub_80C853C`)**: 24 pixels ahead of its panel at a height of 16, sprite (0xC, 0x23) anim 1; it
flies 4 pixels a tick for (0x500000 − 0x180000) / 0x40000 = 14 ticks, falling 0x100000 / 14 a tick (both BIOS
divisions), its panel following its X; then on a solid panel action 4: sound 0x11D, a T3 0x23 splash there
(Param2 1, Z 0x100 from r3) and one on the next panel if that is solid (Param2 0); else it just goes.

**The splash, T3 0x23 (`loc_80C86D8`)**: sprite (0xC, 0x1A), collision self 0xA, target 5, modifier 3, hit effect 2;
30 ticks, its region off once it hit. On its destroy a solid panel cracks if Param1 ≥ 2 and Param2 is set.

**The pillar, T4 0x2D (`sub_80E37F4`)**: sprite (0x10, 0x1F); shows while SpoutMan does. 9 ticks rising (anim 3),
then it follows his signals in its PhaseInitialized byte: 1 → spout (anim 1, 27 ticks, then anim 2), again 1 →
stand (anim 3) and spout again, 2 → gone; going, it clears his slot if it still holds it. Its Z fraction is the
object loop's register garbage (`scratch_z_fraction`).

**The geyser, T3 0x17 (`sub_80C6DCC`)**: a pixel down and up from his panel (sprite (0x10, 0x20)), running its first
update in its init. It picks its column (`sub_80C6F08`: the first of the next five holding something of the other
side's, `byte_80C6F48`, or Param3, or x ≤ 1 / ≥ 6) into Param4, marks it and the trail to it (T4 0x2E: the column's
Param1 1 with Z 1, the trail's Param1 0; Param2 = its ticks; Param3 = `byte_80C7028[Param1]`, 1 for SpoutMan's),
highlights them every tick, and every 30 ticks from the first hits the column's three panels (valid ones) and the
trail with hit regions (region 1, hit effect 2, target 5, self 4, modifier 3) that run while dimmed and last 30
ticks (their Timer). Param2 ticks.

**The marks, T4 0x2E (`sub_80E39A0`)**: 2 pixels down and up, sprite (0x10, 0x20), anim `word_80E3A30[Param1]` (1, 2);
Param2 ticks counted from the tick they appear.

Unverified (no scenario reaches them; the navi AI's): the ball, splash, pillar, geyser and marks with Param1 (or
Param3) other than SpoutMan's, which stand still while dimmed and end with their owner's action 0xB.

#### The other navi chips' navis: common shape

Each is spawned by its `off_802CD5C` entry like ElmntMan (panel, element, the user in RelatedObject1Ptr, the user's
side and flip, the damage word; the controller's flag pointer in CollisionDataPtr, except TenguMan's in
ExtraVars[0]), and steps its sprite while dimmed (`object_updateSpriteTimestop`). Their position is the spawner's
registers (panel Y, element, and in r3 the spawner's own address, which the controller calls through): those whose
init clears only Z's whole part (a halfword store) keep that address's low half as their Z fraction (SpoutMan
0x9751, TomahawkMan 0x999B, TenguMan 0x9F0F, HeatMan 0x921B). Most actions enter on CurPhase 0 (4) and some store
Timer and Timer2 as one word. Where the scratch lab says "all match", every scenario of the chip and its EX and SP
(long, long-miss, long-adjacent, long-holes) matches every frame. Unverified everywhere: a pool with no free slot
(the spawns' failure branches) and the battle ending mid-attack.

#### 3.6.12 TomahawkMan (subtype 8, T1 0x0A `sub_80B97C0`)

Sprite (8, 7). 0: anim 3, sound 0x94, VISIBLE, 3 ticks; panel flags 0x10010 → 4, else 0x10. 4: anim 0, 30 ticks. 8:
anim 0xF, 30 ticks. 0xC (PhaseInitialized): anim 0x10, 61 ticks (to −1); at 55 left the strike: a hit region on the
panel ahead (region 0x11, no hit spark, target 0, self 7, modifier 1) that resolves while dimmed, T4#0 effect 0x33
there 16 pixels up (flip = alliance ^ flip), sound 0x10A and a camera shake (`camera_initShakeEffect_80302a8(1,
30)`: presentation, not simulated). 0x10: anim 4, 3 ticks, the controller's flag cleared. All match.

#### 3.6.13 TenguMan (subtype 9, T1 0x0C `sub_80B9C14`)

Sprite (8, 8), and collision (self 4, target 5, modifier 3, hit effect 6, region off): he is the attack. Each tick
the hits resolve (a hit turns his region off) and he registers again after acting. 0: anim 3, sound 0x94, VISIBLE, 3
ticks. 4: 30 ticks. 8: anim 4, 23 ticks; at 20 left Z's whole part 255 (out of sight), no ground shadow. 0xC: the
dash: region 4, anim 0x13, 16 pixels up with a shadow, on panel (0, 3) (side 1: (7, 3)), 5 pixels a tick ahead
speeding up 0xA000 a tick, 40 ticks (sound 0x13C at 30 left); each new column turns the region back on; region 0x23
highlighted from (1, 3)/(6, 3) 4 ticks out of 8. 0x10: the dive from (2, 0)/(5, 0): anim 0x14, 12 pixels a tick
ahead and 0x73333 down, 0x280000·5 / 0xC0000 = 16 ticks (a BIOS division), sound 0x13C, region 0x25 highlighted
from (4, 2)/(3, 2); then region off and state 8 (a byte store). Destroy (`sub_80B9F00`): the controller's flag
cleared through ExtraVars[0], the object freed — its collision slot is never freed. Without a collision slot at
init he goes at once (unverified). All match.

#### 3.6.14 BlastMan (subtype 12, T1 0x06 `sub_80B8EA0`)

Sprite (8, 0xC). 0: anim 3, sound 0x94, VISIBLE, 3 ticks → 4 (no panel check). 4: anim 0, 30 ticks. 8: anim 0xA, 20
ticks. 0xC: anim 0xB, a fire blast along his row and the rows above and below (`sub_80B903A`, valid ones) from
behind his side's edge (x 0, Param2 1: right; side 1 x 7, Param2 0: left), sound 0x17F; 61 ticks. 0x10: anim 4, 4
ticks, the controller's flag cleared.

**The fire blast, T3 0x21 (`sub_80C8388`)**: speed `byte_80C8454[Param1]` (BlastMan's Param1 4: 6 pixels a tick),
direction `byte_80C846C[Param2]` (left, right, up, down), `byte_80C8488[Param2]` / speed ticks (0x1180000 / 0x60000
= 46), BlastMan's sprite animation 0x11 (0x12 vertically) with flip `dword_80C8478[Param2]`; collision self 4,
target 5, modifier 3, hit effect 1, its body the hit region; the panel it is over highlighted; a hit or its time ends
it. Param1 other than 4 waits a dimming out (unverified). All match.

#### 3.6.15 HeatMan (subtype 2, T1 0x07 `sub_80B9078`)

Sprite (8, 1), his parts (`sub_8010DF6(2, 1, 1)`: body overlay 2, stepping even while paused). 0: anim 3, sound
0x94, VISIBLE, 5 ticks; panel check → 4 / 0x10. 4: anim 0, 30 ticks. 8: anim 0xF, 10 ticks. 0xC: anim 0x15, the cone
(`sub_80B9240`: one panel ahead, three on the next column, five on the one after; each panel whose flags have
0x10010) of T3 0x26 flames (Param1 4, Param2 30); 61 ticks, anim 0x10 at 52 left. 0x10: anim 4, 5 ticks, his parts
off, the controller's flag cleared.

**The flame, T3 0x26 (`sub_80C8C74`)**: sprite (0x10, 2); collision self 4 (0xA with Param3 set), target 5,
modifier 3, hit effect 1; a hit turns its region off. 5 ticks flaring (sound 0xF7), Param2 − 5 burning (anim 1),
region off, 2 dying (anim 2). All match.

#### 3.6.16 ElecMan (subtype 3, T1 0x08 `sub_80B92B8`)

Sprite (8, 2). 0: anim 3, sound 0x94, VISIBLE, 3 ticks → 4. 4: anim 0, 30 ticks. 8: anim 0x12, 13 ticks. 0xC: anim
0x13, sound 0xC6, strike 0; 120 ticks, strike 1 at 60 left. A strike (`sub_80B9458`) searches region 0x26 (5×5
around him, dx along his side's direction, `object_getPanelRegion`) for panels with `byte_80B94AC` (strike 0: an
enemy navi's body; strike 1: 0x810000) and puts a T3 0x64 thunderbolt (Param1 = strike + 1) on each, last found
first. The first bolt's Z fraction is what the search left in r3 (`_object_getPanelDataOffset`'s x − 1, or y − 1,
for the region's last panel); the others', his side and flip halfword. 0x10: anim 4, 4 ticks.

**The thunderbolt, T3 0x64 (`sub_80D0D7C`)**: sprite (0x14, 0x14), 255 pixels up; collision self 0xA, target 5,
modifier 3, hit effect 3, region off. 20 ticks, its panel highlighted 4 ticks out of 8; then down (the loaded
animation forgotten, so it restarts), sound 0x12E, region 1 for 20 ticks of 30 (a hit turns it off); Param1 0 cracks
its panel, Param1 2 hits the eight panels around it once (status 0x12, modifier 1). Param1 0 waits a dimming out with
its collision off (unverified; so is strike 1 finding anything in the lab). The lab's adjacent and miss scenarios
reach strike 0's bolt; all match.

#### 3.6.17 ChargeMan (subtype 6, T1 0x16 `sub_80BB914`)

Sprite (8, 5). 0: anim 3, sound 0x94, VISIBLE, 3 ticks; panel check → 4 / 0xC. 4: his train (`sub_80BBB38`: on each
panel behind him that is solid and none of 0x0F800000, until one isn't, five at most, a T3 0xAC car: Param1 1,
Param2 its place, Param3 60, Param4 0xFF, speed 0x40000 in its ExtraVars[0]; how many into his ExtraVars[0], which
nothing reads); 60 ticks, anim 0xF at 30 left. 8: sound 0xE3, 4 pixels a tick ahead for |edge − x|·0x280000 /
0x40000 ticks (edge 7 or 0); each tick the panel under him (`sub_800E258`): on the field and not solid → 0xC; else,
reaching a new column, a hit region there (region 1, hit effect 0xA, self 6, modifier 3, resolving while dimmed).
Then 0x10: the controller's flag cleared and gone. 0xC: anim 4, 4 ticks, then the same.

**The car, T3 0xAC (`sub_80DAE94`)**: sprite (0x10, 0x54), Z's and Y's whole parts lowered by Param2 + 1 (so the
train stacks); `sub_80169BE` each tick (visible unless dimmed; hidden from a blind local player if it is the remote
side's). Param3 ticks, then it rolls like ChargeMan (its speed from ExtraVars[0]), hitting new panels (modifier 3
while dimmed with Param1 set, else 1), exploding (T4#0 effect 0x12, 16 pixels up) where the floor ends, when the
battle ends, or when its owner leaves action Param4 (0xFF: never checked). Param1 0 starts rolling at once and
waits a dimming out (unverified). All match.

#### 3.6.18 SlashMan (subtype 4, T1 0x0D `sub_80B9F44`)

Sprite (8, 3). 0: anim 3, sound 0x94, VISIBLE, 3 ticks; panel check → 4 / 0x1C. 4: anim 0, 10 ticks. 8
(`sub_80BA1D6`): from the other side's edge toward him, the first column with a panel of `byte_80BA228` into Param2
(none before his own column → 0x10); anim 0x14, 10 ticks. 0xC: every 15 ticks from the first, `sub_80BA238` puts a
T3 0x62 sword wave on each free panel of column Param2 (`byte_80BA284`), the user's (their owner and side), with
Param1 1, element 0 and damage = the chip's Param1 | his damage's flag bits; Param2 steps back toward him, until his
column or the edge → 0x10. 0x10: 60 ticks. 0x14 (`sub_80BA294`): the panel in front of the first enemy navi within
five columns ahead, if solid and unoccupied (or his own) → future panel, anim 4, 16 ticks, there at 13 left (anim
3); else → 0x1C. 0x18: anim 0x10, sound 0x158, a hit region on the panel ahead (region 4, no hit spark, target 0,
self 7, modifier 3, resolving while dimmed) and T4#0 effect 0x39 there 16 pixels up; 30 ticks. 0x1C: anim 4, 4
ticks, the controller's flag cleared.

**The sword wave, T3 0x62 (`sub_80D07CC`)**: 20 pixels up, sprite (0x10, 0x39) with a shadow; collision self 4,
target 5, modifier 3, no hit spark, region off; Param2 cleared. Wind-up `byte_80D0914[Param1]` ticks (sound 0xB7;
Param1 0 hits its panel at its start and 10 ticks in), swing `byte_80D0950[Param1]` + 1 ticks (anim 1, region on;
SlashMan's: the user holding B sets Param2, sound 0x8B), then it flies ahead at `byte_80D0A54[Param1]` (sound 0xB3)
until off the field; with Param2 set it drifts toward the nearest row (0, −1, +1, −2, +2) with an enemy navi ahead,
arriving as it reaches that column. A hit on an enemy's body (`byte_80D08C4`) or the battle's end ends it (a byte
store). All match, the steering too (a scratch scenario holding B); unverified: Param1 0 (the navi AI's).

---

## 4. Worked example: GunDelS3 (chip 0x11) in the machgun trace

**Identification.**
- In battle 1, player alliance 0 holds 4× chip 0x11 code N: block 0 `id = 0011×4`, raw `0x1A11 = 13<<9 | 0x11`.
  The hand index goes 0→1→2→3 at frames 638, 779 and 920.
- Each use shows the user's CurAction 8 → 0x37, a T1 #5 object (params 9) at F+1, a T4 #0x48 at F+7, and the
  enemy losing 4 HP per frame.
- Battle 2's hand is 0x11, 0x11, 0xA7, 0xA7. Its GunDelS3 uses are the Beast Out variant (§4.6).
- Setup line: player 0's folder is GunDelS3 and Geddon only.

### 4.1 The chip record (ROM 0x08021DA8 + 0x11·0x2C)

| Off | Value | Use |
|---|---|---|
| +0..3 | `0D 10 16 FF` | codes N, Q, W |
| +4 / +6 | 0 / 0x0A | Null element, Null family (beast-boostable family, but not boostable here: flags bit 1 clear) |
| +9 | 0x48 | not a dimming chip, no damage display |
| +0x0B | **0x37** | handler `JumpTable80EAC60[0x27]` = **`sub_80EDAE0`** (0x080EDAE0) |
| +0x0C | **2** | level (GunDelS1/S2/S3/EX = 0/1/2/3) |
| +0x0F / +0x17 | 1 / 9 | Beast lock-on flag / lock-on mode (Beast Out only) |
| +0x14 | 0 | no lockout |
| +0x1A | **0** | printed damage; **the handler never reads it** |

**[trace]** Post-frame 638, `av`: `[3] = 2`, `[2] = 0`, `u16[8] = 0`, `u32[0xC] = 0`, `u16[0x14] = 0x11`,
`[0x1C] = 2`, `[0x1D] = 0`.

### 4.2 Handler `sub_80EDAE0`: 3-phase machine on `av[0]` via `off_80EDB08`

Common epilogue, every frame: if `ai.flags44 & 0x8600`, call `sub_801056A(flags, 0, 0)`. That aborts into action
0x47 (flag 0x200), 0x48 (0x400) or 0x4B (otherwise) and zeroes `RelatedObject1Ptr` and `ai.u32[0x68]`.
This abort did not occur in the trace.

```
phase 0  sub_80EDB14 (av[0] == 0):
  if av[1] == 0:                                  // handler frame 1 (F+1)
     object_setFlag1(1<<22); object_setAnimation(0x0A); object_setDefaultCounterTime()   // CounterTimer=16 (link)
     sub_80B8E30(params = 7 + av[3], slot = &ai.u32[0x68])     // T1 #5 "gun" (visual), Param1 = 9 for S3
     PlaySoundEffect(0xF8); av.u16[0x10] = 6; av[1] = 4
  else:
     av.u16[0x10] -= 1; if > 0: return
     av.u16[0x10] = LEN[av[3]]; av.u16[0x12] = av.u16[0x10] - 0x14      // LEN = dword_80EDBC8 = {60,90,120,120}; +0x12 dead
     (*ai.u32[0x68]).CurAnim += 1                                        // gun anim 0 -> 1
     sun = GetBattleNaviStatsByte(alliance, 0x22) != 0
     p   = byte_80EDBB8.u16[av[3] + (sun ? 4 : 0)]                       // {0,0,0,1, 0x200,0x200,0x200,0x301}
     obj.RelatedObject1Ptr = sub_80E5D12(params = p,
              offset = (object_getFrontDirection()*0x50 << 16, 0, 0), slot = &obj.RelatedObject1Ptr)  // T4 #0x48 (visual)
     av.u16[0] = 4                                                       // -> phase 1 (clears av[1])
phase 1  sub_80EDBCC (av[0] == 4):
  av.u16[0x10] -= 1; if (av.u16[0x10] as i16) < 0:
     if obj.RelatedObject1Ptr: sub_80E5D3E(it)                           // T4 -> CurState 8
     av.u16[0] = 8; return
  dmg    = sun ? 4 : 2                                                   // ns[0x22]
  params = (av[3] < 3) ? 0x2C05FF04 : 0x2C05FF11
  object_spawnCollisionRegion(x = PanelX + 2*frontDir, y = PanelY, element = 5, z = 0,
                              r4 = params, r6 = dmg, r7 = 0)             // T3 #3 one-frame hitbox
  [two loops read PanelData.Type for 3 panels and discard the result: dead code, but reads only]
phase 2  sub_80EDC78 (av[0] == 8):
  if av[1] == 0: (*ai.u32[0x68]).CurAnim += 1; av.u16[0x10] = 10; av[1] = 4   // gun anim 1 -> 2
  av.u16[0x10] -= 1; if (av.u16[0x10] as i16) >= 0: return
  object_clearFlag(1<<22); object_setAnimation(0)
  obj.RelatedObject1Ptr = 0; ai.u32[0x68] = 0                           // attachments self-destruct next update
  object_exitAttackState()                                               // CurAction = 8, ai[0x19] = 0
```

Per-level constants (`dword_80EDBC8`, `byte_80EDBB8`, region shapes via `PanelOffsetListsPointerTable`):

| Chip | `av[3]` | Hit frames | Gun Param1 | T4 params (no sun / sun) | Region (Param1) | Total dmg (no sun / sun) |
|---|---|---|---|---|---|---|
| GunDelS1 0x0F | 0 | 60 | 7 | 0x000 / 0x200 | 4: (0,0), (0,−1), (0,+1) | 120 / 240 |
| GunDelS2 0x10 | 1 | 90 | 8 | 0x000 / 0x200 | 4 | 180 / 360 |
| GunDelS3 0x11 | 2 | 120 | 9 | 0x000 / 0x200 | 4 | 240 / 480 |
| GunDelEX 0x12 | 3 | 120 | 10 | 0x001 / 0x301 | 0x11: x∈{0,1} × y∈{−1,0,+1} | 240 / 480 |

- Region X offsets are multiplied by the facing direction (`sub_300777C`); Y offsets are not. S1–S3 therefore hit
  the vertical 1×3 column two panels in front of the user, and EX hits a 2×3 block starting there.
- **"Sun" is `ns[0x22]`.** In the overworld, `sub_80355EC` (called from `EnterMap`) sets it to 1 on the maps listed
  at `word_803562C` (real-world outdoor areas) and to 0 elsewhere. It reaches battle through the exchanged NaviStats.
  In this replay both players have 1, so dmg = 4 (0x0203CE22 = 0x01 at frame 638). The "sun/outdoor" reading is
  inferred from the map list **[unverified name]**.
- **Damage is hard-coded** at 2 or 4 per frame. The chip damage field, `av+8`, `av+6`, Atk+ and cross boosts are
  all ignored.

### 4.3 Spawned objects

| Object | Spawner | Role and lifetime |
|---|---|---|
| T1 #5 `sub_80B8CD8` | `sub_80B8E30` | Arm "gun", visual only. Copies the owner's panel/alliance/flip, `RelatedObject1Ptr = owner`, `ExtraVars[0] = &ai.u32[0x68]`. Sprite row `byte_80B8BD4[Param1*5]`. Follows the owner (`sub_80B8DA6`). When `*ExtraVars[0] == 0` it goes to CurState 8 and is freed next frame. No collision. |
| T4 #0x48 `sub_80E5C2C` | `sub_80E5D12` | Sun beam, visual only. Placed at owner + (front·0x50, 0, 0). Hidden while dimmed unless Param3 == 1. Plays sound 0xF9 when `Timer2 % 11 == 0`, i.e. on its first frame and every 11 frames after. Destroyed by `sub_80E5D3E` or when `*ExtraVars[0] == 0`. No collision. |
| T3 #3 `sub_80C52B0` | `object_spawnCollisionRegion` (0x080C536A) | One per phase-1 frame. `Element = 5`, `Damage = dmg`, Param1 = 4 (shape), Param2 = 0xFF (hit effect), Param3 = 5 (target type), Param4 = 0x2C (self type), no status. Lives exactly one update (§4.4). |

### 4.4 How the damage lands: element 5 is a silent drain

1. The region spawns during the user's update and runs later in the same frame. `sub_80C52D0` sets X/Y, then
   `object_createCollisionData` and `object_setupCollisionData(cd, 0x2C, 5, 0)`:
   - self flags = `byte_8019C7C[0x2C*8 + alliance*4]` = 0x8000408C for alliance 0;
   - target flags = 0x15800000.

   It then sets Region = Param1, hit effect 0xFF, calls `object_presentCollisionData`, and falls straight into
   `sub_80C532E`.
2. `sub_80C532E` → `object_removeCollisionData` resolves the hit. `sub_3007218` adds `SelfDamage × (1 + weakness)` to
   the defender's `CD + 0x82 + 2·5 = +0x8C`, and `sub_3007692` adds to `+0x9E`.
   - Element 5 has no weakness entries (`getPrimaryElementWeaknessMultipler_3007432` reads zeros for index def·5+5),
     so the value is exactly `dmg`.
   - The region's timer is 0, so it frees its collision data and itself in the same frame.
3. On the **defender's next update**:
   - `sub_801AC6C` → `object_calculateFinalDamage1` sums only buckets +0x82..+0x8A (elements 0–4), so
     **FinalDamage = 0**.
   - `applyDamageToPlayer_801ba12` (0x0801BA12) therefore skips the hit sound, white flash, flinch and invincibility
     paths and runs only `object_subtractHP(CD[+0x8C])` (at `loc_801BA68`).
   - `object_presentCollisionData` then clears +0x68..+0xA7.
   - The result: HP drops every frame with no flinch or i-frames. **[trace]** The target's flags1 stayed 0x2000000
     throughout; a memory watch shows `[0x0203857C] = 4` on frames 646..765.

Whether barriers, auras or invisibility block element-5 damage is **[unverified]**. The barrier code sums only
buckets 0x94..0x9C, which suggests they do not. None of these occurred in the trace.

### 4.5 Frame-exact timeline (battle 1, use 1, F = 638)

Uses 2 (F = 779) and 3 (F = 920) follow the same offsets.

| Frame | Offset | User (T1 #0, alliance 0) | Objects | Enemy HP |
|---|---|---|---|---|
| 635 | F−3 | Movement action 0x10 ends; CurAction 8 | | 1000 |
| 636–637 | | Idle entry: Timer 10 → 9 → 8, PhaseInit 4 | | 1000 |
| **638** | F | A pressed (`input[0] = [0xFC01, 1, 0]`): flag 4, `sub_80127C0`, CurAction 8 → 0x37, `blk.cur` 0 → 1 | – | 1000 |
| 639 | F+1 | Phase 0 init: anim 0x0A, flag 0x400000, CounterTimer 16, sound 0xF8, timer 6 | T1 #5 (params 9) spawned, listed right after the navi, already in state 4 | 1000 |
| 640–644 | F+2..F+6 | Timer 5..1 | | 1000 |
| 645 | F+7 | Timer → 0: timer = 120, gun anim 1, spawn T4, → phase 1 | T4 #0x48 (params 0x200), sound 0xF9 | 1000 |
| 646–765 | F+8..F+127 | Phase 1: one T3 region per frame at (5, 1..3) | T4 sound 0xF9 every 11 frames | hit into +0x8C |
| 647–766 | F+9..F+128 | | | −4 per frame: 996 … 520 (120 hits = 480) |
| 766 | F+128 | Timer = −1: T4 → destroy, → phase 2 | T4 gone | 520 |
| 767 | F+129 | Phase 2 init: gun anim 2, timer 10 → 9 | | |
| 768–776 | F+130..F+138 | Timer 8..0 (A presses at 759/769 set flag 4, as in §2.8) | | |
| 777 | F+139 | Timer −1: clear flag and anim, zero slots, `object_exitAttackState` → CurAction 8, flag 4 dropped | T1 #5 sees slot 0 → CurState 8 | |
| 778 | F+140 | Idle entry (`[4,8,0,4]`) | T1 #5 freed | |
| 779 | | Next A press → use 2 | | |

- **Use 2 (F = 779):** HP 520 → 40. Drops on frames 788..907, T4 alive 786..906, exit at 918.
- **Use 3 (F = 920):** hits start at 928. HP reaches 0 at frame 938 after 10 hits, and the enemy enters CurAction 2
  (deletion). **The attack is not cancelled by the target's death:** regions keep spawning, T4 lives until 1047,
  phase 2 runs 1049..1058, and the exit is at 1059 = F+139.

### 4.6 Battle 2 variant: Falzar Beast Out, and the "Geddon" that was not a dimming

In battle 2, `ns[0x2C] = 0x0C` (Falzar Beast Out), and the player object's NameID 0x1B7 is a form NameID.

| Frame | Event |
|---|---|
| 1999 | A pressed. `sub_801336C` is true (form `ai[5] = 5`), so 0x20000 (charging) is set, not flag 4. |
| **2003** | A released (`[0xFC00, 0, 1]`) with the charge not full → flag 4. Chip used: `blk.cur` 0 → 1, CurAction 0x37, `av[0x1D] = cd+0x0F = 1`. |
| 2004 | `sub_80EAD9C` phase 0 (`sub_80EADDC`): `av[0x1E] = 4`, `av[0x23] = 0xC` → 0xB. |
| 2005–2007 | Phase 4 (`sub_80EAE28`): anim 4, 3-frame countdown, reposition via `ho_8026554(9)`. The user did not move. |
| 2008 | Phase 8 (`sub_80EAF36`) starts calling `sub_80EDAE0`: the normal timeline shifted by +5 (T4 at 2014, drops 2016..2135, 480 damage). |
| 2039–2072 | A presses after the 12-frame ignore window set `av[0x22] = 1` (queued chain). |
| **2146** | No input. GunDelSol exits (CurAction 8). `sub_80EAF36` immediately runs `sub_800FC30` + `sub_800FC7C`: CurAction 0x37 again, `cur` 1 → 2, pre-phase 2147–2149, phase 0 at 2150, drops 2158..2277 (520 → 40), exit 2288. |
| 2263–2337 | A held. Charging resumes once idle (`flags48 & 0x10`). At 2337: `ai[0x1B] = 0x14`, `ai[0x1D] = 2`. |
| **2338** | A released with a full charge → **flag 8** (charged chip). Current chip 0xA7 (Geddon, family 0xA) → `t = ai[0x11] = 0x1E` → `sub_80117BA(0x1E)` → **CurAction 0x52** (`sub_80EF534`, a 2-hit slash with T4 0x3A/0x39 and collision regions, `av+8 = 60`, `av+0x0A = 0x9E`). `av.u16[0x14] = 0`, `av[0x1D] = 1`, `cur` 2 → 3. |
| 2339 | Rush phase 0. |
| 2340–2342 | Rush phase 4: marker frozen (anim 1), navi anim 4; at 2342 the claw's mode 0xC picks (4,2), in front of the target at (5,2). Afterimages at the midpoint (x 0) and at the old panel (3,2), each with a T1#0x57 layer. |
| 2343 | The claw's first tick: anim 0xC (the overlay's CurAnim 0xC, CurAnimCopy 0). |
| 2345 | First slash: effect 0x3A at (5,2), z 0x10 px; hit region 2 at (5,2), 60 damage, counter 0x9E, hit modifier 1. |
| 2346 | The target's HP 40 → 0: the battle is over; the marker frees itself. |

- **Geddon's own effect never ran, and no dimming occurred:** `BattleState+0x32 = 0x0001` at 2338/2340/2345,
  and the dimming records at 0x0203CF00 are all zero.
- T1 #0x57 is the beast-form overlay, mirroring the navi's animation, and T4 #0x0F is the Beast Out lock-on marker (objects-and-player.md §A.7, §12.9). Both come from the form change, not from the chip.

#### 4.6.1 The claw: weapon routine 0x1E and action 0x52

`sub_8011E1C` (routine 0x1E): `av[4] = 0`, `av[5] = 0`, `av.u16[6] = 0`, `av.u16[0x0A] = 0x9E` (counter strength),
`av.u16[8] = sub_8012642(0x32, 0xA)` = 50 + 10·min(buster damage, 5) (`sub_801265A`; 60 in the trace), `av[2] = 0`,
`av.u16[0x12] = 2` (slashes); returns action 0x52.

`sub_80EF534`, on `av[0]`:
- **0, `sub_80EF550`**: on entry, ObjectFlags1 |= 0x400000, `av[1] = 1`, `av.u16[0x10] = 3`,
  `object_setAnimation(0xC)`, and a form overlay gets a **halfword** store of 0xC (CurAnim 0xC, CurAnimCopy 0).
  Count the timer down; when it was ≤ 1: timer = 0xC, `object_setDefaultCounterTime`, `av[0] = 4`, sound 0x1C5 or
  0x1C6, and with `k = 2 − (u8)av[0x12]`: effect #0 id `byte_80EF606[k]` (0x3A, 0x39; flip = alliance) at the
  coordinates of the panel in front (z 0x10 px), and `object_spawnCollisionRegion` on that panel: element 0, z 0,
  damage word `av.u32[8]`, params `byte_80EF5FC[k]` (region 2 then 4, no hit effect, target type 5, self type 4)
  and hit modifier `byte_80EF604[k]` (1 then 3).
- **4, `sub_80EF608`**: count the timer down; when it was ≤ 1, `av.u16[0x12] -= 1`: nonzero → `av[0..1] = 0` (the
  next slash), zero → clear 0x400000 and `object_exitAttackState`.

### 4.7 What is generic vs GunDelSol-specific

**Generic:**
- The A → flag 4 → idle → `sub_800FB54`/`sub_80127C0` → `setAttack2(cd.action)` path, and `cur++` in the same frame.
- The handler starting next frame.
- The phase machine on `av[0]`/`av[1]`, with timers in `av+0x10`.
- `object_setDefaultCounterTime`.
- The arm-slot convention and `object_exitAttackState`.
- `object_spawnCollisionRegion` + T3 #3 one-frame hitboxes, element-bucketed damage applied on the defender's next
  update.
- The `sub_801056A` abort epilogue.
- The Beast wrapper.

**Specific to GunDelSol:**
- Phase lengths: 7-frame wind-up, 60/90/120 hit frames, 11-frame recovery.
- Hard-coded 2/4 damage keyed on `ns[0x22]`.
- Element 5 (silent drain).
- Region params 0x2C05FF04 / 0x2C05FF11 two panels ahead.
- The two visual attachments (T1 #5 with params 7+level, T4 #0x48).

---

## 5. MegaBuster (non-chip counterpart)

The buster uses the same framework (slot 1, `av`, a T3 type-0 shot) but gets its values from NaviStats, not a chip.
Chip 0 "MegaBstr" (action 0x1C) is only a pseudo-chip.

- **Request.** For plain MegaMan (`ai[6] = 0`, `ai[7] = 1`), B requests on the **release** edge: flag 1, or flag 2
  when the charge is full (`ai[0x1E] == 2 && ai[0x1D] == 2`).
  - Holding B while ready sets 0x40000 (charging) via `sub_8012EBC`.
  - Charge length = `u16 byte_8020404[(ai[7]*10 + ns.Charge*2) / 2]`, which gives 100 frames for Charge 0.
- **Normal shot.** `sub_8011764` → `off_80117D4[ai[6]]` = `sub_8011A26` → **action 0x11** (`setAttack1`).
  - `av+8 = sub_801265A()`, where `sub_801265A` = min(`ns[1]` + 1 + form bonus (1 for forms 1/0x0D), 10), forced to
    1 if `sub_8015B54 == 5`.
  - `av+2/+3/+4/+5/+6/+0xA = 0`.
  - `av+0xC = ns[0x4D]`, but only if an **RNG2** roll passes. `GetPositiveSignedRNG2` is consumed only when
    `ns[0x4D] != 0`.
  - `sub_8013D5E` may redirect the request to 0x33 or to the charge shot.
- **Charged shot.** `sub_80117A4` → `sub_8011A7E` → **action 0x16** (`sub_80EBE00`).
  - `av+8 = (ns[1] + 1) * 10`, or 10 when emotion is 5.
  - `av+0xC = ns[0x4F]`, with a similar RNG2 gate.
- **Action 0x11** (`sub_80EB436`, 0x080EB436), phase 0 `sub_80EB450`:
  - Frame 1: anim 0xE, arm (`sub_80EB562` into `ai+0x68`), flag 22, timer 0.
  - Handler frame 2: sound 0x6A, then `sub_800FAAC(r0 = av+0xC, r1 = av+8, r3 = 0x180000)` (0x0800FAAC). This spawns
    the **same T3 type-0 shot as Cannon** (`sub_80C4FFE`, descriptor `av+0xC`, i.e. 0 for a plain shot) and returns
    the recovery length `byte_80209CC[...]`, indexed by free panels ahead (≤ 5) and `ns[2]` (rapid). That value goes
    to `av+0x12`.
  - `av[3] == 1` adds two side shots; `av[3] == 2` uses `sub_80C6248`.
  - Phase 1 (`sub_80EB502`) waits `av+0x12` frames. A move input can interrupt it (`sub_800FA54`/`sub_800F964` →
    exit + `sub_80116AE`). Then exit.
- **Descriptor 0.** self 4 / target 5 / hitmod 0 / element 0 / hit effect 0.
- **Not traced.** No B presses occur during fighting in the machgun trace (only at frame 3).

---

## 6. Quirks the port must reproduce (checklist)

1. **Same-frame use, next-frame handler.** The A press, `object_setAttack2`, `blk.cur++` and the `av` fill all
   happen in the idle handler on the press frame. The action handler first runs the next frame. Idle is not
   re-entered on the frame an attack exits.
2. **Presses during a chip or buster action are dropped** (exit clears 0x1000003F). Movement is exempt **[unverified]**.
3. **Stale `av` bytes.** Chip use does not initialise +0x10..+0x13, +0x16..+0x1A or +0x20..+0x4F.
4. **`chip_800AEE8` empty-hand read.** Chip 0xFFFF reads ROM 0x082E1D85 (0x30). Treat it as a no-op.
5. **One-frame-stale `obj.ChipsHeld` / `obj.Chip`.** `sub_800FDC0` runs after the object logic.
6. **Usage-counter aliasing** (`sub_8021D14`, 0x170 bytes per alliance, ids up to 0x19A).
7. **Register-selected source** in `sub_80127C0` (caller r4 bit 16). **Family passed as `arg`** in the flag-8 path.
8. **NULL-pointer writes** from `sub_80E1662` (through `ai+0x40`). They are no-ops on hardware; skip them.
9. **Beast chain writes `AIData[0x85]`** instead of `av+0x23`.
10. **Instant action 0x1C adds only the low byte** of the bonus.
11. **Stale-r3 OR** in `sub_8029224` modifier folding (harmless in the normal path).
12. **PA bit spent before the veto** (`sub_8029652` runs before `sub_8029328`).
13. **Vulcan consumes RNG2** once per bullet. The buster consumes RNG2 when `ns[0x4D]`/`ns[0x4F]` is nonzero.
14. **Attachment objects matter.** T1 #5 arms and visual T4s occupy slots and update-list positions. New objects are
    inserted right after the current object and run in the same frame.
15. **Element-5 drain** bypasses FinalDamage, flinch and i-frames (GunDelSol).
16. **HUD and banner code guarded by `battle_networkInvert`** (`sub_801DA48`, `sub_801DACC`, `sub_801EB18`) is
    presentation only. It must not affect state.
17. **Hands are not reset between turns** when a player selects nothing **[unverified]**.

---

## 7. Uncertainties (ranked by impact on a PvP port)

1. **Dimming is entirely untraced.** Dim 16 / banner ≈59 / undim 17 frames, LIFO cut-in ordering and the
   AntiNavi paths are all code-derived; no available replay executes a dimming chip. Get a trace with a dimming chip (and ideally a
   cut-in) before trusting §3.6's frame counts. The dim length also depends on the fade level when the dimming starts.
2. **Multi-turn hand lifetime.** The claim that a player who selects nothing keeps last turn's leftover hand comes
   from code only; every trace here has a single turn. The link-transmit stall condition `sub_803EA2C` was not analysed.
3. **CollisionData+0x07 (`hit_param`) semantics.** The counter-hit bit and the +0x8E/+0x90 accumulators are read
   from the resolver around 0x0300735C; their downstream use (stagger or stamina) is not established.
4. **Per-chip frame counts** for Cannon, Vulcan and Sword are code-derived. Only GunDelSol (and its Beast variant)
   is trace-verified.
5. **Unread chip fields** +0x0D and +0x0E have no reader found by static scan.
6. **Identities inferred from names or shape:**
   - Rush/Beat/Tango for 0x179..0x17B and their NaviStats+0xD bits;
   - the Battle-Chip-Gate slot-in path (+0x1E, `sub_800EE26`/`sub_800EE98`; off in PvP);
   - the cross/beast form numbering in `ns[0x2C]`;
   - `ns[0x22]` = sun/outdoor;
   - T3 0xAD (`sub_80DB1E0`) and `sub_80E667C`, which raise `atk_bonus`.
7. **[SP] navi-chip damage tiers** (`sub_8000D84`, `byte_8020E54`) are not decoded. They need each player's
   `byte_203EB00` record, which is transmitted at battle start.
8. **Hand mod-flag bit 0** (slot-descriptor +4) has unknown meaning. It was 0 in the trace and has no fight-time reader.
9. **Gregar/Falzar (0x138/0x139)** reach NULL `off_802CCB4` slots. In the US ROM they crash the game, so they are
   out of scope.

---

## Appendix: address index

| Address | Symbol | Role |
|---|---|---|
| 0x08021DA8 | `ChipDataArr_8021DA8` | chip table (411 × 0x2C) |
| 0x08021AA4 | `getChip8021DA8` | record accessor |
| 0x080109A4 / 0x080109DC | `sub_80109A4` / `off_80109DC` | damage + variable-damage formulas |
| 0x020349C0 / 0x02034A10 | `byte_20349C0` / `byte_2034A10` | chip blocks (`sub_8010018`) |
| 0x0800A954 / 0x0800A964 | `sub_800A954` / `sub_800A964` | block init |
| 0x08029110 | `sub_8029110` | hand builder (PA `sub_8029520`, fold `sub_8029224`) |
| 0x0800B3A2 / 0x0800B3D8 | `sub_800B3A2` / `sub_800B3D8` | turn packet pack / commit to blocks |
| 0x0801FF18 | `sub_801FF18` | link word tx/rx |
| 0x0800FDC0 / 0x0800FDEA | `sub_800FDC0` / `sub_800FDEA` | ChipsHeld/Chip exposure |
| 0x0800AEE8 | `chip_800AEE8` | per-frame variable-damage refresh |
| 0x08012DFC | `sub_8012DFC` | pad latch into AIData |
| 0x08012FC8 | `sub_8012FC8` | input → request flags |
| 0x0800A772 | `sub_800A772` | chip-usable test (lockout) |
| 0x080EA734 / 0x080F0354 | `sub_80EA734` / `sub_80F0354` | idle / attack decision |
| 0x0800FB54 / 0x080127C0 | `sub_800FB54` / `sub_80127C0` | take chip / compute attack params |
| 0x0800EDD0 / 0x0800EF34 | `sub_800EDD0` / `sub_800EF34` | hand read / cross-beast bonus |
| 0x080126E4 | `sub_80126E4` | AIAttackVars fill from ROM |
| 0x08012A38 | `sub_8012A38` | double-damage decision |
| 0x0800FC7C | `sub_800FC7C` | consume (`cur++`) |
| 0x08011698 / 0x08011714 | `object_setAttackN` tail / `object_exitAttackState` | action enter / exit |
| 0x0801AF44 / 0x0801B9E6 | `sub_801AF44` / `sub_801B9E6` | navi per-frame gate / action dispatch |
| 0x080EAC60 | `JumpTable80EAC60` | action handlers 0x10..0x5E |
| 0x080EAD9C | `sub_80EAD9C` | Beast lock-on / chain wrapper |
| 0x080EBD9C / 0x0802CCB4 | `sub_80EBD9C` / `off_802CCB4` | dimming chip handler / dimming chip spawners |
| 0x080EC350 / 0x0802CD5C | `sub_80EC350` / `off_802CD5C` | navi-chip handler / summon routines |
| 0x08017AB4 | `sub_8017AB4` | cut-in while dimmed |
| 0x0800BF16 / 0x0800BF5C | `sub_800BF16` / `sub_800BF5C` | dimming registration / dimming record |
| 0x0800B916 / 0x0800BD34 | `object_timefreezeBegin` / `object_timefreezeEnd` | dimming on / off |
| 0x080B8E30 | `sub_80B8E30` | T1 #5 "arm" attachment spawner |
| 0x080C536A | `object_spawnCollisionRegion` | T3 #3 one-frame hitbox |
| 0x08019FB4 | `object_setupCollisionData` | object → CollisionData |
| 0x080EDAE0 | `sub_80EDAE0` | GunDelSol handler (action 0x37) |
| 0x080EB436 | `sub_80EB436` | buster (action 0x11) |
