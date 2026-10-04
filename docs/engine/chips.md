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
| +0x0F | u8 | `beast_lockon` | Copied to `av[0x1D]`, but only in Beast Out forms or for a chip-gate chip. When set, dispatch goes through the Beast wrapper `sub_80EAD9C` (§2.11). nettai: BN6's beast system's chip extension `beast` (its presence, unless `rush = false`), which its `chip_used` writes to the attack's `wrapped`. | `sub_800FB54` |
| +0x10 | u32 | `params` | 4 action-specific bytes, copied to `av.u32[0xC]` and passed as r4 to spawners. Examples: Vulcan shot row 0x0C, AirShot 4, TankCan 0x100. | `sub_80126E4` |
| +0x14 | u8 | `lockout` | Post-chip lockout in frames. Copied to `av[5]`, then to `ai[0x19]` at attack end (§2.8). Most chips 0. Seeds and Lance 10; FireHit, Boomer, GolmHit, BusterUp, Atk+10 and others 20; Recov and TimeBom 30; AirHocky 50. | `sub_80126E4` |
| +0x15 | u8 | `lib_index` | Library sub-index. | menus only |
| +0x16 | u8 | `flags2` | See §1.3. | `sub_800EE98`, `sub_8010740` |
| +0x17 | u8 | `lockon_mode` | Beast Out lock-on panel selector: `sub_80EAE28` passes it to `ho_8026554` (0x08026554), which indexes `jt_8026584`. nettai: the beast system's `beast.lockon` (a lock-on mode, rules/lockon.luau). | `sub_80EAE28` |
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
| 0x02 | Canceled by the opponent's **Rush** support (`sub_8010740`, 0x08010740). Set on Invisibl and WhiCapsl. See §2.10. |
| 0x40 | Menu classification: the modifier chips (WhiCapsl, Uninstll, Atk+10, Navi+20, Atk+30; `ExtraChipFlags::MODIFIER`). |
| 0x01, 0x10, 0x20 | Menu classification only. |

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

A chip's behavior is selected entirely by `cd.action` (+0x0B) and `cd.subtype` (+0x0C).

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
     US ROM using them crashes the game; the Japanese ROMs fill the slots, and the port's chips are theirs
     (docs/engine/beast-chips.md; chips/gregar, chips/falzar). The same holds for Count's and Django's navis,
     `off_802CD5C`'s null entries 18 and 19 (chips/count, chips/django; jp-differences.md §4).
   - **0x1B** `sub_80EC350` (0x080EC350), navi chips. It spawns T4 object 0x10 via `sub_80E192C`. That controller
     later calls **`off_802CD5C[subtype]`** (0x0802CD5C, 29 entries).
4. Action **0x1C** `sub_80EC39C` (0x080EC39C) is the "instant" handler. It calls `off_80EC3F0[av[3]]` once and exits
   (subtype 0x14, TenguCross's B+Back, waits 8 more ticks). It is used by 54 chips: the MegaBuster pseudo-chip 0,
   Atk+/Navi+ left unfolded, FullCust, Boomer, Lance, FireHit, the error chip 0x185, and others. `off_80EC3F0` has 23
   entries; 7 and 0x12 are NULL (the game would jump to address 0). The entries are content's: a chip definition's
   `instant` hook, or a weapon's `instant`. 0 BeastOut `sub_80104E0` and 3 the plus chips
   `sub_8010488` (lib/instant/plus, with their sparkle, effect #0x14, objects/rising-bubble), 1 the boomerang
   (chips/boomer/boomerang, kind `boomer/boomerang`), 4 Lance
   (chips/lance), 5 FullCust `sub_800AF34` (chips/fullcust), 8 FireHit (chips/firehit), 10 BusterUp `sub_8010820` (chips/busterup), 12 SandWrm (chips/sandwrm), 13 SyncTrgr
   `sub_80EC44C` (chips/synctrgr), 15 ColForce (chips/colforce), 19 JustcOne (chips/justcone), 21 GolmHit
   (chips/golmhit), 22 ColArmy (chips/colarmy). Subtypes 2
   (`sub_8010474`, invisibility), 6 (`sub_801050C`, repairs the side's obstacles), 9 (`sub_8015AA6`, an
   immobilizing hit, attack #0x3F, on every enemy body in the row ahead), 11 (`sub_802E1BE`, writes side state
   nothing reads), 16 (`sub_80E5A64`, a meteor shower) and 17 (`sub_80C6330`, a dust storm) are named by no chip
   (2 and 9 by the link navis' weapon routines 0x71 and 0x83): they are builders in lib/instant (`invisible`,
   `repair`, `immobilize`, `side_special`, `meteor_shower`, `dust_storm`), which take what the game reads from
   the attack's parameters as arguments (the ticks, the drops, the storm's tie to its user's action), for those
   to call. Unverified: no scenario reaches them.

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
- `sub_800A954` (0x0800A954) initializes both blocks once per battle, from `sub_80071D4`. It calls `sub_800A964`,
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
(`sub_8010004`, `sub_800EDD0`), never `obj.Chip`. Player objects are recognized by `sub_800F29C(NameID) == 2`.
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
| +0x01 | u8 | 0: phase-initialized flag |
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

### 2.10 Supports: Rush, Beat, Tango (link battles only, NaviStats+0xD bits)

The NaviCust supports act once per battle. Rush and Beat are checked in `sub_80F0354` after a chip's action is
already set up (Beat first); Tango every idle frame before any request:

| Support | Function | Trigger | Effect |
|---|---|---|---|
| **Beat** | `sub_80106C0` | Opponent `ns[0xD] & 2` and the chip's class is 1 or 2 (Mega, Giga) | Clears the bit; the controller for the opponent (Param1 1, telop chip 0x17A). |
| **Rush** | `sub_8010740` | Opponent `ns[0xD] & 1` and `cd.flags2 & 2` (`RUSH_CANCELS`: Invisibl, WhiCapsl) | Clears the bit; the controller for the opponent (Param1 0, the chip in Param3/4, telop chip 0x179). |
| **Tango** | `sub_8010660` | Own `ns[0xD] & 4` and `HP ≤ MaxHP/4` | Clears the bit; the controller for the navi itself (Param1 2, telop chip 0x17B); `sub_80F0354` returns 1 without acting. |

- The chip's record is read with the id as it is (`getChip8021DA8` without the `& 0x7FFF` the tail of
  `sub_80F0354` applies), and only when the support's bit is set.
- On a Beat or Rush cancel, `object_exitAttackState` runs, so the lockout applies and requests are cleared. The
  hand index is **not** incremented here; Rush and Beat call `sub_800FC7C` on the victim later.
- `sub_80E90FE` spawns the controller, effect object #0x79 (`sub_80E8FE0`, content kind `support/controller`,
  lib/supports/controller: the role `kinds.support`; the ruleset sets its `support`, `eaten` and `telop_chip`), on the
  host's panel (the support's owner: the chip user's opponent for Rush and Beat), with the host in related 1,
  its side, element 0, no damage and the telop chip at +0x30. Its X, Y and Z are the caller's r1..r3 (the host's
  panel row, 0, 0). `sub_800BF16(host side, 1, controller)` then starts a dimming the other side can't cut in
  on (`Battle::start_dimming`), used by the host.
- The controller (`object_timefreezeBegin`, then by action: `object_dimScreen`, `object_drawChipName`,
  `sub_80E9024`, `object_undimScreen`; `object_timefreezeEnd`): phase 0 warps the host out
  (`sub_80E1332(host, 1)`, not for Tango) and waits 31 ticks (`ldrh`/`sub`/`strh`/`bge` counts past zero);
  phase 4 calls the support's spawner (`off_80E90A4`) with the controller's params and `r7 = &Param2`, which the
  spawner sets to 1 and the support clears (through its related 1) when it goes, and waits for 0 (a failed spawn
  leaves it 0); phase 8 waits 31 ticks; phase 0xC warps the host back in (not for Tango), waits 31 ticks and
  moves on to the undim.
- **Rush** (actor #0x4B, `sub_80C3218`; sprite 0C-48, no ground shadow): appears on the host's panel if its
  flags pass `byte_80C34A8` (else (1 or 6, 2)), on the ground, sound 0x112; collision types 0x17/5 with status
  0x12, region 0. Animation 0 plays out, then his bark (animation 1) for 60 ticks. Then he digs in (hidden, effect
  #0 look 0x14), 3 ticks later moves under the victim's navi (gone at once without one; Y and Z up a pixel), 3
  ticks later shows with animation 2; when it ends the victim's hand moves on unless the chip is 0x17E
  (WhiCapsl), animation 3, region 1 for one tick (sound 0x122 on each loop), and after 60 ticks the dust (8
  pixels back, 32 up) and his end. Each tick runs between `object_removeCollisionData` and
  `object_presentCollisionData`. The bite's branch without a victim skips a `pop {r5}` and returns into Rush's
  own memory (**[unreachable in practice, errors]**).
- **Beat** (actor #0x4C, `sub_80C34E0`; sprite 0C-4B, ground shadow): starts 240 pixels behind the victim's navi
  (toward the victim's side's back) and 112 above it, X velocity 12 pixels a tick toward the victim, Z -4. Sound
  0x120; 20 ticks of swoop; animation 1, Z velocity reversed, 6 ticks of bounce with the X velocity losing 4
  pixels a tick (`byte_80C3628`, 0x40000); animation 2, the victim's hand moves on, sound 0x126; when the
  animation ends, animation 3 for 75 ticks; animation 4 and 40 ticks flying off (12 pixels a tick away, Z +4).
  His first extra variable is set to 1 and only read by the uncalled `sub_80C3700`.
- **Tango** (actor #0x4D, `sub_80C3734`; sprite 0C-4C, no ground shadow): 33 pixels in front of her navi, 60 up
  and a pixel down the field (`sub_80C390E`), sound 0x116. Effect #0 look 0x15 and 17 ticks; she drops 4 pixels
  a tick until at most 16 up, lands (Z 0, animation 1, sound 0xD4), 30 ticks, animation 2; when it ends she
  throws her heal (below) from 16 pixels back toward her navi, 10 up (sound 0xB2), waits for it to land (her
  +0x0C flag, which the heal holds), 10 ticks, bows (animation 3); when it ends, animation 0 at 16 up, rising 4
  pixels a tick for 9 ticks, then effect #0 look 0x14 and her end.
- **Tango's heal** (attack #0xC7, `sub_80DE000`; sprite 0C-4D): Param3/4 are her navi's panel; it arcs there in
  20 ticks (`sub_8001330`, gravity 0xFFFF7778), animation 1 once animation 0 ends. Landed, it clears Tango's
  flag, destroys itself, leaves effect #0 look 6, heals her navi 300 (`object_addHP`), sound 0x8A, and gives it
  barrier 5 (`sub_801A7CC`) with a new barrier visual (effect #7 `sub_80E0D98`, the old one in AIData+0x60 told
  to go with `sub_80E0DC0`).
- All three test `ns[0xD] != 0xFF` first: 0xFF is the NaviCust's support bug (bug 8, `sub_813CDF4`, what a bugged
  Rush, Beat or Tango part sets), with which no support comes.
- **Verified** in the lab, every frame of each: `navicust/rush` and `rush-side1` (the other side's Invisibl
  bitten, its second Invisibl going through), `navicust/beat` and `beat-side1` (a Mega chip, Roll, taken; the
  second Roll comes), `navicust/tango` and `tango-side1` (her heal at a quarter of the HP and its barrier, which
  the next Cannons wear), and `navicust/bug-support` (a bugged Rush never comes). Each support is hosted once by
  either side. Not reached: a Giga chip for Beat, WhiCapsl for Rush (its hand left alone), Rush with the victim's
  navi gone, a failed spawn.

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
| Cannon/HiCannon/M-Cannon (0x14 → `sub_80EBC0E`) | 33 | f1: anim 8, counter time, arm (Params `sub_80EBD68()<<8 \| byte_80EBD2C[av3]`), flag. **f16** (timer 0xF): sound 0xAE, then `sub_80C4FFE(x+front, y, r2=av+6, r3=0x180000, r4=byte_80EBD34[av3], r6=av.u32[8]+av.u16[6])` → T3 type 0 shot (descriptor 1). GigaCan (variants 4..6) leaves two afterimages at timer 8. f30: release arm, anim 7, timer 3. **f33**: exit. The full spec, with ticks counted from the handler's first run: shot-chips.md §2. |
| Vulcan1..3/SuprVulc (0x17 → `sub_80EBF10`) | 36 for Vulcan1 | f1: anim 0xA, arm Params 0xD into `ai+0x68`. f3: shots `av+0x12 = dword_80EBFEC[av3]` = {3,4,5,10}; a shot every 11 frames from f3 (Vulcan1: f3, f14, f25). **Each shot draws `GetPositiveSignedRNG2() & 3`** to pick Z from {8,0x10,0x18,0x20}<<16 (visual height only, but it advances RNG2). Each shot calls `sub_80C6ADA(x+front, y, av+2, Z, av.u32[0xC], av.u32[8]+av.u16[6])` → T3 0x12. Then a 1-frame recovery init, 10 more frames, and exit. The full spec and the bullet: shot-chips.md §4. |
| Sword family (0x13 → `sub_80EB776`; lib/swords/slash, the sword chips) | 30 | Phase 0 returns at once (or does a step-sword advance if `params` byte 0 ≠ 0: `sub_8015B00` wants the panel two ahead on the field (flag 0x10000) with no body; the navi leaves a T4#0x28 afterimage and moves there, holding its own panel reserved (not in a Beast form or with the special-source byte set); with no such panel, anim 4 and straight to the step back). Phase 1 sets hits = 1 (2 when `av3 == 0xB`). f3: anim 5, `sub_8011450`, sound 0xB0 (0xCE for variants 5, 6, 0xB), the blade (attachment `byte_80EBB64[av3]`, anim by form `sub_80EBAE8`), timer 0x15. A step sword leaves two more afterimages at f8 (itself, and the blade's sprite). **f12**: `object_spawnCollisionRegion(x+front, y, av+2, 0, r4=byte_80EBA18[av3], r6=av.u32[8]+av.u16[6], r7=off_80EBA00[0][av3])` (CrosSwrd, variant 0xA, adds a region-1 one on the same panel) plus the slash, T4#0 `byte_80EBAD8[av3]` at 16 px (palette variant − 0xB for 0xC..0xF). f24 ends the swing (DblDream swings again). A step sword steps back once its animation ends (`sub_80EBB98`). f25: recovery (timer 5). f30: exit. The per-variant tables are each chip's data (`sword`). **Trace-checked** (chip lab: every Sword, WideSwrd, LongSwrd, blade, elemental sword, StepSwrd and Muramasa scenario that reaches it). Unverified: DblDream's second swing, CrosSwrd's second hit, FtrSword, LifeSrd and DrkSword (their scenarios stop earlier), the step with no panel to step to, in a Beast form or with the special-source byte, and the turned-round step (AIAttackVars+0x34, which no player action sets). |
| MchnSwrd/ElemSwrd/AssnSwrd (0x49 → `sub_80EEB4C`; lib/swords/strike, chips/mchnswrd, elemswrd, assnswrd) | 28 | f1: counter time, anim 5, sound 0xB0, the blade, timer 0x15. **f10**: for each opposing alive actor: a region-4 hit (0x0705FF04, hit modifier 3) and T4#0 0x16 (flip = its side, palette variant + 7) on its panel if variant ≠ 1 and it is paralyzed (CollisionData+0x1C), or variant ≠ 0 and its panel has flags 0x1C00 (grass, ice, volcano). f23: recovery (timer 5). f28: exit. The hit is verified by `chips/0x056-mchnswrd/paralyzed` (a paralyzed navi) and by ElemSwrd's and AssnSwrd's `counter-hit` on a grass stage. |
| Instant (0x1C → `sub_80EC39C`) | 1 | `off_80EC3F0[av3](panelX, panelY, av+2, obj.Z, av.u32[0xC], av.u32[8] + (u8)av[6])`, then exit in the same frame (`av3 == 0x14` waits 8 frames). **Only the low byte of the bonus is added.** |
| Dimming chips (0x15 → `sub_80EBD9C`) | whole dimming | §3.6 |
| Navi chips (0x1B → `sub_80EC350`) | 1 | `sub_80E192C(panelX, panelY, av+2, av3, av.u32[0xC], av.u32[8], chip \| av6<<16)` spawns T4 0x10 (summon controller → `off_802CD5C[subtype]`). Registers the dimming exactly as 0x15, then exits **in the same frame**. |
| GunDelSol (0x37 → `sub_80EDAE0`) | 140 (S3) | §4 |
| Reflector (0x2B → `sub_80ED13E`; the pack's `chips/rflectr`) | params[0] + 2 (62) | One phase. f1: the shield (T3 0x2B, `sub_80C97E0`: at attach point 6, look `byte_80C9664[params[1]]`, stored in RelatedObject1Ptr), ObjectFlags1 GUARD and 0x400000, anim 0 (and the Beast head's, `sub_80101D4`), av+0x30 = 0, timer = 0. **Every tick after:** unless subtype 3, if CollisionData+0x03 (the directions the guard blocked) has bit `1 << flip`: a guard-breaking hit (FlagsFromCollision & 2) drops the shield and the guard; otherwise the first such tick (av+0x30 0 → 1) sends the wave back: subtypes 0..2 the T3 0x2F wave (`sub_80C9CDA`: one panel ahead, Z 16, damage word `av.u32[8] + av.u16[6]`, sound 0xC5), subtype 4 the buster's projectile (T3 #0 with Param1 6, Z 20), others nothing. Then timer + 1; past params[0]: the shield and guard go, exit. No reactive abort, no counter window. |
| Recovery (0x20 → `sub_80EC844`; chips/recov, chips/drkrecov) | 1 | f1: `sub_800E2FC(byte_80EC870[subtype], 1)` (10, 30, 50, 80, 120, 150, 200, 300, 1000; each chip's `hp` in chips/recov/chips.luau): unless the opponent's defensive-chip record is AntiRecv (0xBD), HP += n up to the maximum, effect #0 look 6 at the navi, sound 0x8A; if it is, AntiRecv's controller (T4 0x2C, `sub_80E3728`) starts a dimming (`sub_800BF16` with no cut-in) that takes n from the navi, the trap mark (effect #0 look 0x46, Param2 = the local side, sound 0xA5) and the record is spent. Then side statistic 5 + 1, exit. The ruleset's heal (`kinds::heal`) runs it; T4 0x2C is BN6's chips/antirecv/controller, the role `kinds.anti_recovery` (§3.6.7). The AntiRecv branch matches a scratch chip-lab recording (Recov10 against AntiRecv). |
| Reflector's shield (T3 0x2B, `sub_80C96A0`; chips/rflectr/shield.luau) | - | Init: sprite, anim, palette from its look row, panel from its spawn position (the attach-point offset), flip, sound 0xA0, the offset kept in its velocity. Action 0: its owner's position + offset, until the owner's RelatedObject1Ptr is cleared; action 4: the fade animation for the row's ticks, then state 8 (`object_freeMemory`). After the action: visible, unless the local navi is blind to it (`sub_800EB6C`) or its owner vanished for a navi chip (state bit 0x100000). Runs while paused and dimmed; the sprite stands still while dimmed. |
| Reflector's wave (T3 0x2F, `sub_80C9BC4`) | - | Init: off the field, freed; else sprite 0x14/4, timer 2, collision (4, 5, 0) region 1 on its panel. Each tick: resolve, hit spark; battle over: gone. A hit clears its region. Action 0: timer − 1; at 0 the next segment one panel ahead (same Z and damage), action 4. Action 4: gone when the animation's last frame ends. |

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
  - otherwise, when `b & 0x7F ≠ 0` and `!(b & 0x80)`, CD+0x90 += `b & 0x7F`;
  - CD+0x8E += `b & 0x7F`, except on a counter hit, which adds nothing (field-collision-damage.md §3.8).

  CD+0x8E is what the hits wear off the navi's mood, and CD+0x90's 0x8000 gives the attacker Full Synchro
  (`sub_801A200`, field-collision-damage.md §4.9).
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
   controller moves on. The start level `eScreenFade+6` is history-dependent (`SetScreenFade` keeps it): 0 after
   an earlier undim, and 0x40 for a counter cut-in's dim, which is then done after one step (§3.6.5).
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
- AntiNavi checks (`sub_802CE78(opp) == 0xBA`) in `sub_800BA8A`/`sub_800BDB2` for navi chips 0xDD..0x118
  (docs/engine/dimming-chips.md §2; ported in dimming.rs).
- Alliance-swap re-registration in `sub_800BE2C`.

**Quirk.** The r4 value entering `sub_80127C0` from `sub_8017AB4` selects hand vs slot-in (§2.6.4). On this path r4
is always AIIndex*4 (set in `sub_80EA484`, preserved through `sub_801AF44`), so the cut-in always reads the hand.
`sub_80127C0`'s side effects all still happen (the Full Synchro / anger doubling with sound 0x87, the use counter,
heal-on-use, side stat 6 for navi chips, dark-chip costs); it writes every field the spawners read (+2 element, +3
subtype, +6 bonus, +8 damage, +0xA hit param, +0xC params, +0x14 chip). An action other than 0x15 or 0x1B registers
nothing and doesn't advance the hand. `loc_800BF30` doesn't check for a registered controller, and a failed spawn
registers none. `sub_800B8EE(side)`: effect #0 look 0x1E at panel ((side^1)*3+2, 4), z 0x78 px, sound 0xA5.

**The port** (kinds/player/status.rs `cut_in`): `chip_use::prepare_detached` runs `sub_80127C0(0)` on a copy of
the attack variables and restores the navi's own; the controller comes from the same spawners as actions 0x15
(`dimming_chip::spawn_controller`, the chip's dimming controller) and 0x1B (`navi_chip::spawn_controller`);
`Battle::cut_in_dimming` is `loc_800BF30`, `dimming::cut_in_flash` `sub_800B8EE`. The chip lab's 26 counter cut-in
scenarios (the other side's copy of the chip, answered during the telop) match every frame, and show the order:

- The cut-in controller's own dim (`object_dimScreen`) starts from the already dimmed screen: the fade level
  (`eScreenFade+6`) is left at 0x40 by the first dim, and `SetScreenFade` keeps the level, so the dim is done after
  one step (2 updates, not 16). The engine models the fade's level for this (`battle::Fade`).
- The cut-in's telop waits for the first one to end (its state 2 until the other side is 3 or idle); the first side
  then waits in state 3 while the cut-in's effect runs; the cut-in's undim is skipped (the other side isn't done)
  and its end waits; then the first effect runs, its undim fades the screen back, and its end (the initiator's)
  frees both.

The coverage scenarios (docs/engine/unverified.md) verified the rest of the cut-ins: by a chip of another
subtype or action, in chains of up to four (`chips/0x0a3-areagrab/cut-in-invisibl`, `cut-in-chain`,
`chips/0x0dd-roll/cut-in-heatman`, `chips/0x0e3-heatman/cut-in-barrier`); with Full Synchro and with anger, the
cut-in's damage doubled and the mood spent (`chips/0x0e3-heatman/cut-in-full-synchro`, `cut-in-anger`,
`chips/0x08b-meteors/cut-in-full-synchro`); with a dark chip, which goes in as its substitute
(`chips/0x121-darkinvs/cut-in`); Roll cut in against the other side's AntiRecv, whose counterattack takes the
dimming over (`chips/0x0bd-antirecv/roll-cut-in`); a cut-in that deletes the first chip's user before its telop
(`chips/0x0e3-heatman/user-deleted`); and A during a dimming with a chip that doesn't dim next
(`chips/0x0b1-invisibl/cut-in-not-dimming`). Unverified: a failed controller spawn.

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
With subtype 0 (Roll's chips, which heal) and the other side's defensive chip 0xBD (AntiRecv armed), `sub_80E192C`
springs the trap instead (`loc_80E1968`): the trap's mark over the user (`sub_800ABC6`), the other side's record is
spent (`sub_802CEA6`), and AntiRecv's counterattack (T4 0x2C, `sub_80E37D2`; below) comes for the user with three
times Roll's damage (`sub_80E199A`: the damage word's low 11 bits, doubled when it has the double-damage flag 0x8000)
and hit parameter 0x1E, in the chip's parameters (Z = the mark's, left in r3). Action 0x1B registers it as the side's
dimming like the navi chip's controller; nothing else starts one (unlike a recovery chip's heal). Port:
`kinds::navi_chip`, `kinds::heal`.

**AntiRecv's counterattack, T4 0x2C (`sub_80E3728`; BN6's chips/antirecv/controller, which the ruleset spawns by the role `kinds.anti_recovery`).** Spawned by `sub_80E37D2`
(r0/r1 the healer's panel, r2 element 0, r6 the damage word, r7 = 0xBD for the telop; RelatedObject1 = the healer,
the healer's alliance; its position the spawner's registers). A dimming controller: states `object_timefreezeBegin`,
`sub_80E3748`, `object_timefreezeEnd`; actions (`off_80E375C`) 0 `object_dimScreen`, 4 `object_drawChipName`, 8
`sub_80E376C`, 0xC `object_undimScreen`. `sub_80E376C`'s first tick: Timer = 0x3C (unread), `object_subtractHP` on
the healer by the damage (down to 0), two rising bubbles (T4 0x14, palette 1) 16 px right then left of the panel's
center at Z 0, the panel changer (T4 0x1F, kind 6: the own panel turns to poison; dimming-chips.md §4.2) handed
Param2's address, Param2 = 1. Every tick: once Param2 is 0 (the changer cleared the four parameters), action 0xC.
**Lab** (`chips/0x0bd-antirecv/roll` and `chips/0x0bd-antirecv/recov10`): AntiRecv set by side 0, then side 1's
Roll (this branch) or Recov10 (the heal's) springs it; both match every frame (1170). Roll's damage with the
double-damage flag is verified too (`chips/0x0bd-antirecv/roll-full-synchro`: Roll used in Full Synchro, 120 for
60). Unverified: a full effect pool.

**The controller, T4 0x10 (`sub_80E17E8`).** Spawned with r1..r3 = panel Y, element, subtype as its position (so
Z = the subtype; register garbage nothing reads). Object +0x19 = the subtype (which navi, `off_802CD5C`), +0x18 is
a flag its navi clears. Actions: 0 `object_dimScreen`; 4 `sub_800BDB2` (AntiNavi: for chips 0xDD..0x118 when the
other side's defensive chip is 0xBA, the controller changes sides and the navi comes for AntiNavi's user,
dimming-chips.md §2; otherwise straight on); 8 `sub_800BA8A` (the name, as
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

All of these are the pack's scripts (chips/elmntman: navi, meteor, ice, bolt, vine). The navi AI's
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
- The segment, T3 0xC3 (`sub_80DD940`): 8 pixels behind its panel's center at height 35/40/45 by aim, anim 1/0/2;
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
(`sub_802CEF4`). Note `sub_802CEA6` clears only the low half of the record's damage word. The pack's module:
lib/traps/controller (a chip's `dimming` hook is `traps.hook(trap?)`; the port's trap object sees that its side's
record no longer names it, where the original tells it through Param2).

**The traps' counters.** A trap that caught a hit sets its request on the navi, and the ruleset (`sub_801056A`,
`sub_80105F2`; kinds/player/actions/reactive.rs) starts the counter by role (the stock ruleset's `roles`, rules/roles.luau).
The counters are content:

- AntiDmg's (action 0x47, `sub_80EE90C`; chips/antidmg/counter): the navi vanishes and throws a shuriken (attack
  object #0xC2, `sub_80DD764`; chips/antidmg/shuriken) at a random enemy, or by the attack's variant 1 at the
  nearest one ahead.
- AntiSwrd's (action 0x48, `sub_80EEA3C`; chips/antiswrd/counter): three swings, each throwing a sonic boom
  (attack object #0x58, lib/swords/sonic_boom; shot-chips.md §12.1).
- BodyGrd's (action 0x4B, `sub_80EED56`; chips/bodygrd/counter): the navi vanishes and leaves a striker (effect
  object #0x6E, `sub_80E8268`; chips/bodygrd/striker), a field object that drops ten shurikens (attack object
  #0x5C, `sub_80CFEC4`; chips/bodygrd/shuriken) on the enemy navi.

**Lab**: the scenarios that spring a trap match every frame: `chips/0x0bb-antidmg/counter`, `sprung` and
`sprung-side0` (AntiDmg's counter and its shuriken, variant 0), `chips/0x0bc-antiswrd/sprung` (the three swings and
their sonic booms: every block of `sub_80EEA78`) and `pa/0x157-bodygrd/sprung` (the counter, the striker and its
shurikens); and the traps sprung by other hits: AntiDmg by a sword, a volley, a bomb, a flame, a charged
shot and hits inside a dimming, by both sides in turn, and deleting its target (`chips/0x0bb-antidmg/sprung-sword`,
`-vulcan`, `-minibomb`, `-firebrn`, `-charge-shot`, `-heatman`, `-meteors`, `-twice`, `-ko`); AntiSwrd by the
other swords and by ProtoMan's and SlashMan's slashes (`chips/0x0bc-antiswrd/sprung-sword`, `-wideswrd`,
`-fireswrd`, `-stepswrd`, `-moonbld`, `-protoman`, `-slashman`); BodyGrd by a sword and a navi chip
(`pa/0x157-bodygrd/sprung-sword`, `-heatman`). **Unverified**: AntiDmg's variant 1 (two blocks of the throw, `sub_80EE996`), the striker's offline
target (`sub_80E8326`'s other branches: a netbattle takes the player navi) and three branch sides of its tick
(`sub_80E82D4`).

#### 3.6.10 The other dimming chips' controllers (`off_802CCB4`)

Every subtype's controller is a T4 object on the standard dimming phases (`object_timefreezeBegin`, then actions
0/4/8/0xC: dim, telop, the effect, undim; `object_timefreezeEnd`), spawned with the user's panel, element,
alliance (and, for most, flip), damage word, and chip and bonus at +0x30/+0x32. `sub_80EBD9C` registers it with
`sub_800BF16(side, chip >= 0x170, controller)`: r1 is the no-cut-in flag, not the chip. The pack's scripts, by
subtype:

- 1 (Invisibl, WhiCapsl; T4 0x5D): the user flashes invisible for Param1-2 ticks (`sub_8010474`), 31 ticks.
  chips/invisibl/controller (`invisible.hook(ticks)`).
- 6 (RockCube, IceCube; T4 0x37): a rock of variant Param1 (1 a rock cube, 3 an ice block) on the panel in front
  (`sub_80CFBC4`, the rock's spawner), sound 0x112, 60 ticks. chips/rockcube, whose folder holds the rock
  (chips/rockcube/rock, field-objects.md).
- 25 (SloGauge, FstGauge; T4 0x1C, `sub_80E23E8`): the shared custom gauge's rate becomes 0x10 or 0x40 for the rest
  of the round (`sub_801DF8C`; the round start sets it from the navi stats, `sub_8014178`); the user's side's slow
  (+0x3C) or fast (+0x3A) gauge timer in `sub_802E070` gets 480 ticks, and, in the chip gate battle (battle flag
  0x40) outside a link battle, the other side's 1080 (`sub_80107D4` counts them down; nothing else PvP reaches
  reads them); a warning blinks over the gauge (`sub_800AE90`, with sound 0x91 every 16 frames of the game's frame
  counter, which the port approximates with the effect's own ticks), 70 ticks. lib/gauge-speed/controller
  (`gauge_speed.slow`, `gauge_speed.fast`).
- 38 (HubBatc, the arm chips, BugRSwrd, BgDthThd, DarkInvs; T4 0x84, `sub_80E95B4` by Param1): 0 raises the buster
  to attack 5 at least, rapid and charge 4, the custom level 8, defers the hand-shrink bug a turn, gives a B+Back
  special (0x3B) if there was none, and the shoes and undershirt (flags 0x40030 and the stats), resetting the
  body's collision types; 1 and 2 make weapon routine Param2 the charged shot in the stats and the navi
  (`sub_80E97BE`: a buster of 3 or 4 goes, 0x2C becomes 0x2B); 3 sets the navi's request 0x20000000. The arm
  effect's height offset is lost to a shift of the wrong register. lib/navi-boost/controller (`navi_boost.hub`,
  `bug(weapon)`, `arm(weapon, palette)`, `dark`).

  **The weapons those chips install** (`off_80117D4[0x21..0x26]`; each chip's own `charge.luau`, with
  lib/navi-boost/charge for what they share). Every setup clears the charged flag and the Atk+ bonus, sets the
  chip lockout 20 and the counter byte 0x14, then:

  | Routine | Chip | Element byte | Damage | Action | Parameters |
  |---|---|---|---|---|---|
  | 0x21 `sub_8011E40` | BugRSwrd | 0x80 (Null, Sword) | 200 with a bug frag, else 80 | 0x13, the swords' | subtype 6 (DrkSword's slash: the two columns ahead) with a bug frag, else 0 (Sword's); params 0 |
  | 0x22 `sub_8011E78` | BgDthThd | 3 (Elec) | 200 with a bug frag, else 40 | 0x1F, Thunder's | 2, 12, 0x10 (a fast ball of 12 panels, paralysis) with a bug frag, else 1, 5, 0x10 (Thunder's) |
  | 0x23 `sub_8011EAC` | PunchArm | 1 (Fire) | 100 | 0x1C, instant effect 8 (FireHit's fists) | 0, 3 (the fists' hit modifier) |
  | 0x24 `sub_8011ED0` | NeedlArm | 2 (Aqua) | 40 | 0x32, AquaNdl's | 0 (the first palette: AquaNdl1's) |
  | 0x25 `sub_8011EF0` | PuzzlArm | 3 (Elec) | 100 | 0x42, ElcPuls's | 0, 0x10, 0x3C, 0 (no hit modifier, paralysis, 60 ticks, the first look: a pulse no chip has) |
  | 0x26 `sub_8011F10` | BoomrArm | 4 (Wood) | 100 | 0x1C, instant effect 1 (Boomer's boomerang) | 0 (Boomer's row) |

  "With a bug frag": `sub_800F4A8` reads the side's count (`dword_203F7E0`), and with one or more
  `sub_800F4B2(1)` spends one (the local player's save loses it too). 0x24 and 0x25 leave the attack's subtype
  byte as it was; neither action reads it. The instant effects are chips' (subtypes 8 and 1), so the navi idles
  the tick after, unlike a weapon's own (0x14, TenguCross's wind). They are sticky charged shots (`sub_800FFAA`,
  objects-and-player.md §12.B): a form's own doesn't replace one, the attack is of kind 2, and a Beast buster or
  the Beast's throw gives way to its plain counterpart. Their charge times (`word_8020404`, by Charge 0 to 4):
  120, 200, 80, 100, 120, 100, each the same at every level.

Ported too, and specified elsewhere (the chip lab's scenarios for all of them match):

- 4 (the barriers: lib/barriers), 5 (the panel chips: lib/panel-chips), 9 (the instruments: lib/instruments),
  13 (AirRaid: chips/airraid), 26 (BugFix: chips/bugfix), 27 (ColorPt, DblPoint: chips/colorpt), 28 (Sensor:
  chips/sensor), 36 (SumnBlk: chips/sumnblk), the barrier routine `sub_801A7CC`, the barrier visual (T4 7) and
  FirstBarrier: specified in docs/engine/dimming-chips.md.
- 7 (LifeSync; T4 0x5C; chips/lifesync): in a link battle `sub_80E72C8` branches into another routine's body
  (`loc_80E73C4`), harmlessly: LifeSync does nothing in PvP (dimming-chip-effects.md §14).
- The others are in their chips' folders and lib/ (the trap chips' controller is lib/traps/controller; ElemTrap's
  trap and strike are chips/elemtrap).
- Subtypes 2, 3, 7, 8, 12, 14–19, 21–24, 29, 30, 32 and 37, every object they spawn, branch by branch with
  their lab coverage: docs/engine/dimming-chip-effects.md.

Unverified branches: IceCube and WhiCapsl (not folder chips: no lab scenario uses chips 0x17C and 0x17E), BodyGrd
(program advance 0x157: only as its recipe), the chip gate battle's own gauges (not in a netbattle without gates).

**ElemTrap's trap** (T3 0x4D, `sub_80CDF84`; the pack's `chips/elemtrap/trap`) is a collision over whole-field region
0x80 with ObjectFlags1 0x01000000 (hit even while dimmed), self type 0, target 0x18. Each tick it resolves its hits
and reads the per-element damage (CollisionData+0x84, fire to wood); the first element with damage springs it (its
first update runs unarmed: a hit then just clears the record). Sprung, it waits until the battle isn't dimmed, puts
sparkles (T4#0 look 0x46, SE 0xA5) on the enemy navi's panels, spawns the counterattack T4 0x2B (`sub_80E35A4`,
`chips/elemtrap/strike`; `sub_80E360E`) and registers it with `sub_800BF16` (the other side can't cut in), clears
its side's record and ends. The counterattack's effect (`sub_80E362C`) hits every panel with any of
`byte_80E36E4[side]` (the enemy's bodies) in that element (`byte_80E36EC`, damage plus bonus, `sub_80C53A6`) and
spawns the panel bursts T4 0x24 (`sub_80E2F56`, `objects/panel-bursts`: shared by seven callers, among them TimeBom's
blast) over region 0x80.

**Where the counterattack goes: the Japanese games' (the user's decision, 2026-10-02).** The US games' `sub_80E360E`
spawns it at the **head** of the update list (`sub_80033E4`, which only the US ROMs have), so it first runs in the
next tick. The Japanese games' (EXE6 Falzar 0x080E81E6, EXE6 Gregar 0x080E9516) spawn it with `object_spawnType4`,
right after the trap (`sub_8003400`), so it runs later in the tick the trap springs: the dimming, and all that
follows, come a tick earlier. The engine runs the Japanese games' on every console
(`battle.spawn`; jp-differences.md §8.1). The spring, the sparkles, the counterattack and the bursts are
**verified** on Japanese consoles (EXE6 Falzar and Gregar; the chip lab's `jp/chips/0x0c5-elemtrap`: sprung by fire,
aqua, elec and wood, `sprung-dimmed` (sprung inside the other side's dimming: it waits for the dimming to end), and
every other ElemTrap scenario, `null-hit` among them: a hit without an element leaves the trap). The US consoles'
recordings that spring it (`chips/0x0c5-elemtrap/sprung-fire`, `sprung-elec`, `sprung-dimmed`) match up to the
counterattack's first tick and differ from there, as the user chose: bn6battle-verify lists them as known deviations.

#### 3.6.10 TimeBom, Mine, Guardian (subtypes 10, 11, 14)

- **TimeBom** (T4 0x27 `sub_80E31D8`, `chips/timebom/controller`; 31 ticks) sets the countdown bomb T3 0x4B
  (`sub_80CD8EC`, `chips/timebom/countdown`) on the first panel ahead meeting `off_80E3280[side]` (a free enemy panel). The bomb
  (variants `byte_80CD8AC`: 0 TimeBom1-3, HP 50; 1 TimeBom+, HP 200) rises, counts 3, 2, 1 (60, 60, 60, 30 ticks,
  shown by hiding sprite parts), then hits whole-field region 0x82/0x81 (the enemy area of the side opposite its
  panel's) and sets off bursts; broken first, it only puffs. Variants 2 to 7 (HP 3 to 10; `bursts_when_broken`,
  `allows_bodies`) need a slot pointer in r7 that TimeBom's controller doesn't pass: the port refuses them (a variant
  is a `CountdownVariant` record its chip passes; no chip passes those rows, whose branches are the record's
  `allows_bodies`, `bursts_when_broken` and `linked`). The
  blast (`chips/0x090-timebom1/blast`), the bomb broken first (`broken`), pushed (`pushed`) and the battle's end
  (`round-end`) are **verified**, and removal, blink-out and absorption (`dustman`, `colarmy`, `absorbed`).
- **Mine** (T4 0x29 `sub_80E342C`, `chips/mine/controller`; 121 ticks) lays T3 0x4C (`sub_80CDD44`,
  `chips/mine/land_mine`),
  which shuffles the enemy's free panels (`byte_80CDF50`, 20 swaps), hops through them every 2 ticks (59 hops, SE
  0x113), then hides armed (region 1, types 0x33/0x2A) until something touches it, its HP runs out, its panel stops
  being solid or the battle ends; it blows up (T4#0 look 0x47, SE 0x70) the tick after. It has its own action table
  and no reaction dispatcher. The lab verifies the hops, and arming and blowing up: stepped on
  (`chips/0x091-mine/stepped-on`), its panel broken under it (`panel-broken`), the battle's end (`round-end`).
- **Guardian** (T4 0x52 `sub_80E6758`, `objects/guardian`; 30 ticks) places the statue T3 0x7D (`sub_80D4C84`,
  `objects/guardian-statue`, HP 1, 6000 ticks, `sub_801B4D4`) on the free panel in front; stages place one with
  actor-list entry type 9 (`sub_800751C`, Param1 1, the panel's side). Broken by one side's hits only
  (`sub_80D4FF6` on its hit flags), it takes the other side's part: its own dimming T4 0x53 (`sub_80E680C`,
  `objects/guardian-strike`, the telop of chip 0x175, started with `sub_800BF16`), whose effect sets the statue's
  Param3; then a hit on whole-field region 0x85/0x84 (the enemy navi's panels) with sparks (`sub_801BD3C`, which the
  game calls with the panel's Y and the element as its panel). The strike back is **verified**, on either side
  (`chips/0x097-guardian/punish`, `own-hit`).

### 3.7 RskyHny (action 0x39, `sub_80EDD80`)

Content: chips/rskyhny (`action`: the action's builder; `bee`: the bee, T3#0x74; `chips`: the three chips, each
level's bees its own).

The action (`off_80EDD94`, three phases on `av[0]`; no counter window, no reactive abort):

| Phase | Routine | What |
|---|---|---|
| 0 | `sub_80EDDA0` | Anim 0xA; the hive (attachment kind 0x28) in `ai+0x68`; USING_ACTION; **AIData+0x48 bit 0x200000** (the RskyHny trap, `NaviState` `heat_trap`); `av+0x10` = 1. Two ticks. |
| 4 | `sub_80EDDE0` | A bee (`sub_80EDE4A`), unless params byte 1 (`av+0x0D`) is set, when it clears the navi's +0x0D (the drag step) instead: no chip has it (**unverified**). `av+0x12` = 3. |
| 8 | `sub_80EDDFC` | Three windows of 11 ticks (`av+0x10` = 10 down to -1): entering one clears `av+0x30`; while the phase-init byte is 1, `av+0x30` ≠ 0 sends a bee (once a window). After the third: `RelatedObject1Ptr` and `ai+0x68` cleared, the trap bit cleared, `object_exitAttackState`. |

The trap: `sub_802CEF4` (damage intake), while the bit is set, swallows the hit and, if it did any damage that isn't
fire (PanelDamage2 = 0 and another element's nonzero), sets `av+0x30` = 1 and plays 0x6E. A hit reaction clears the
bit (`sub_80178D4`...).

`sub_80EDE4A`: `sub_80D32FE(panelX + front, panelY, av[2], Z = 0x10 px, r4 = av.u32[0xC], r6 = damage word + bonus)`,
then the hive's anim = 1 (`ai+0x68`'s +0x10/+0x11 = 1, 0xFF) and sound 0x1A8.

**The bee (T3#0x74, `sub_80D30D0`).** Spawned at the registers (panelY, element, 16 px) with the chip's params (Param1 =
level 0..2, Param3 = turns taken), flags |= 0x10. Init (`sub_80D30F4`): coordinates from the panel; `sub_8011504`
(sprite 0x10/0x31 with a shadow, anim 0, collision self 4 / target 5 / hitmod 1; without a collision slot: effect #0
id 0x14 16 px up, and freed); visible, hit spark 4, palette = level; speeds by level (`byte_80D3164`: across 0x28000,
0x30000, 0x38000; along 0x18000, 0x1CCCC, 0x21999); X velocity = front × speed; Timer2 = 0x280000 / speed (ticks a
panel: 16, 13, 11). Update (`sub_80D317C`): remove, spark; battle over or off the field → region 0, destroy; hit flags
& 0xF3800000 → the same; & 0x0C000000 (a body) → region 0 and action 4 (unless already). Not while dimmed: the action
(0 fly, 4 sting, 8 fade); present. `object_updateSpritePaused` after.

- Fly (`sub_80D31EC`): off the field → destroy with action 2. At each panel's center (phase 0): steer (`sub_80D3326`),
  Timer = Timer2, snap to the center. Move by the velocity; panels from coordinates.
- Steer: the destination is the nearest panel with an enemy body (`off_80D33F4`: 0x04000000 / 0x08000000 by side) in
  the first such column from the **user's** column + front going forward (`sub_80D3374`: its own row, else the nearest
  row, the upper on a tie), else from the user's column going back, else ((side ^ 1) × 7, 2). Flying along a column,
  once level with or past the destination's row it turns across toward it (or reverses if it is in this column);
  flying across, once level with or past its column it turns up or down toward it (or reverses in its row). Each turn
  counts in Param3 (`sub_80D3496`); from the third on it flies straight. Its flip follows its X velocity.
- Sting (`ho_80D3240`): `sub_80E7486` finds the other side's combatant whose collision is on the panel (the panel must
  show the other side's navi: 0x200000 / 0x400000); five times, 5 ticks apart, it moves onto that navi's panel and
  (region 1 again) hits; then destroy.
- Fade (`sub_80D32C4`): blink for 10 ticks and destroy. **Nothing sets action 8 (unverified).**

Verified: soundmod round 3 (RskyHny3 at frame 39688, its bee turning into the enemy's row and stinging) and the chip
lab's RskyHny scenarios (every one that runs as far as the chip matches), with the bee's end by battle over
(`chips/0x025-rskyhny1/ko`), its destinations with no body to find (`invisible`: `sub_80D3342`'s fallbacks,
`sub_80D3374`, the flip in `sub_80D3474`) and with the opponent walking or shooting on (`moving-target`,
`bee-shot`). **Unverified**: the bee without a collision slot; its end off the field or by an attack's hit
(0xF3800000); the fade; the phase-4 branch on params byte 1.

### 3.8 The dragons (action 0x51, `sub_80EF4B4`)

Content: lib/dragons (`action`: the action's builder; `head`, T3#0xC9; `body`, T3#0xC8; `dragon`: what the two
share, and a dragon's variant), with a chip folder each (chips/heatdrgn, elecdrgn, aquadrgn, wooddrgn). The target
column is content/exelib/panels' `enemy_column`, which MachGun shares.

The action: phase 0 (`sub_80EF4D0`): anim 0xC, counter window, USING_ACTION, `av+0x10` = 15; at 13, the dragon; at -1,
`av+0x10` = 5 and phase 4 (`sub_80ECA0C`: 6 ticks, then `object_exitAttackState`). The column (`sub_80ED040`): the
first column ahead with an enemy body (0x04010000 / 0x08010000), else the last column ahead (6 facing right, 1 facing
left; shot-chips.md §8). `sub_80DE660(x, 0, element,
0, r4 = subtype, damage word + bonus)`: panel (x, **0**), the row above the field.

**The head (T3#0xC9, `sub_80DE404`).** Init (`sub_80DE430`): sprite 4/0x10, anim 2, palette 3 × kind; Z16 -= 0x24;
collision self 4 / target 5 / hitmod 3, region 1, hit spark by kind (1, 3, 2, 4); `sub_80DE67E` (velocity 4 px/tick
down and forward, 10 ticks a panel, dip step 0x80 / 10 = 12); four body segments (`sub_80DE7C8`, delays 3, 6, 9, 12);
a splash (effect #0 id 2 at Z 0, flipped); sound 0xF7. Update (`sub_80DE4EC`): remove, spark; battle over → destroy;
hit flags & 0xFF800000 → region 0; not while dimmed the action; present; `object_highlightCurrentCollisionPanels`.

- Column (action 0): Y += velocity; on each new panel (`sub_80DE730`) the panel it left gets the kind's type
  (`sub_80DE768`: none, cracked, ice, grass, on solid field panels) and region 1 again. Going down, at row 3: action 4
  with the dip's base Y. Going up, at row 0: 4 more ticks (`sub_80DE7A0`), action 8.
- Swim (action 4): 10 ticks; the anim runs 2..6 by fifths (`sub_810FA4C`); X += velocity; Y = base + sine[angle] × 12
  × 256 (`math_sinTable`, read unsigned, angle += 12). At the tenth tick: turn up (Y velocity negated), X snapped to
  the panel.
- Leave (action 8): climb the 4 ticks, then a splash, region 0, hidden, destroy.

**The body (T3#0xC8, `sub_80DE13C`).** Spawned at the registers (the head's row, i, the delay) with Param1 = kind;
its Z keeps the delay as its fraction. Hidden (anim 7) until its delay is up, then a splash and the head's path
(`sub_80DE21A`, `sub_80DE266`, `sub_80DE2B0`) without collision, panel types or animation changes.

Verified by the chip lab's dragon scenarios (every one that runs as far as the chip matches), with either
part's end when the battle is over (`chips/0x02e-heatdrgn/ko`), no enemy body ahead (`invisible`) and a hit on a
barrier (`barrier`). **Unverified**: the head without a collision slot; `sub_810FA4C`'s cap at 4; `sub_80DE768`'s
off-field and not-solid exits.

### 3.9 The bombs and seeds (action 0x12, `sub_80EB628`)

Content: lib/bombs (throw, bomb, seed, slash) and the bomb chips (chips/minibomb, ...), objects/bomb (T3#8), objects/bomb-slash
(T3#0xA), objects/energy-burst (T3#0x11), objects/seed (T3#0x4F), objects/flash-bomb (T3#0xA4), objects/bug-bomb
(T3#0xA5), objects/black-bomb (T3#0x4A), objects/rising-bubble (T4#0x14), objects/panel-bursts (T4#0x24), lib/region.luau
(`sub_801BD3C`, `sub_80CE468`, `sub_80CE424`), lib/trajectory.luau (`sub_8001330`, `sub_800120E`, `sub_80011A0`,
`calcAngle_800117C` and the BIOS division, square root and arctangent), lib/hp.luau (`object_applyDamage`).

The action: phase 0 (`sub_80EB644`): anim 6, counter window, the held thing (attachment `byte_80EB738[subtype]`, Param4
3 for BigBomb and 3 × params[0] for FlashBomb) in the first related slot, sound 0xB2, USING_ACTION, `av+0x10` = 0; at
9 the thrower `off_80EB6F8[subtype]` from (X + 4 px ahead, Y, Z + 48 px) with the chip's parameters and damage word +
bonus, then both related slots cleared; at 0x15 phase 4 (`sub_80EB758`: 6 ticks, `object_exitAttackState`). No reactive
abort. Subtypes: 0-2, 4, 5, 10, 11, 15 the bomb; 3 LilBoiler; 6 BlkBomb; 7 BugBomb; 8 VDoll; 9, 12, 13 the seeds; 14
FlashBomb.

- **Bomb** (T3#8): 40 ticks in an arc (0x2E666 a tick ahead, rising 0x20666 less 0x2800 a tick; its panel (0, 0) until
  it lands, so it hits nothing), then on a solid panel its kind's region (`dword_80C5D7C`: 1, 0, 0, 0xF) with effect 0
  on each panel and sound 0x70, or effect 1 over a hole; the next tick kind 1 leaves the lingering hit (T3#0xA, 60
  ticks), kind 2 the energy burst (T3#0x11: three hits 7 apart with hit modifiers 1, 1, 3); a hit it made ends it first.
  Kinds by Param1 (`byte_80C5BA0`): collision types, hit modifier and palette.
- **Seed** (T3#0x4F): the bomb's flight; the tick after landing, its kind's panel type (poison, ice, grass) over its
  panel if the hit connected, else the 3x3, with effects on the non-missing panels (dx scaled by the side and flip
  halfword, as the game passes it).
- **FlashBomb** (T3#0xA4): thrown to the panel three ahead in 40 ticks (`sub_8001330`, gravity 0x3000); sets down with
  the level's HP (`byte_80D9A20`) unless the panel holds a body of its own side; two ticks before its time is up a
  palette flash (variant 1), sound 0x1BD, and a hit over the other side's area with the level's status; levels with a
  final status hit again as it goes. A hit to its body (0x0F800000) or its HP running out ends it.
- **BugBomb** (T3#0xA5): lobbed at 0x2C000 a tick (`sub_800120E`) to land 10 px up; a 40-HP body for 61 ticks, then a
  burst over region 0x10 (effects 0x4A on the 3x3) carrying a NaviCust bug the target's side lacks (`sub_80D9FC2`: one
  RNG draw); broken or crushed first, an explosion; in a hole, a puff (T4#0x14).
- **BlkBomb** (T3#0x4A): an obstacle (field-object class 1) lobbed like the BugBomb; 100 HP, 6000 ticks; a fire hit
  sets it off (`sub_80CD7A0`), and as it goes it hits the whole area of the side opposite its panel's owner (fire, hit
  modifier 3) and scatters panel bursts (look 1) there. Its update uses the obstacle service's `sub_801AD12` and
  `sub_801B750`, and its push is `sub_8017CC0`.

The confusion and blindness a BugBomb or FlashBomb gives show the status visual (T4#6, `sub_80E08FC`, Rust
`kinds::status_visual`).

Verified by the chip lab's bomb and seed scenarios (every one that runs as far as the chip matches, including every
chip's counter variant, which throws a MiniBomb). **Unverified**: bomb kind 1 and the lingering hit (no chip throws
it); seed kind 3; FlashBomb levels 3 to 8, its Param2 and setting down onto its own body; the BugBomb crushed or
in a hole; the BlkBomb placed (Param1), with no ticks to fly, removed, absorbed or blinking out. The coverage
scenarios verified: a bomb ending at the battle's end (`chips/0x036-minibomb/ko`); the BlkBomb set off by fire of
either side (`chips/0x03c-blkbomb/fire`, `enemy-fire`), pushed (`pushed`: `sub_8017CC0`), broken without fire
(`shot`), left to its lifetime (`lifetime`) and thrown at a hole (`holes`); the BugBomb's other bug choices
(`chips/0x043-bugbomb/seed-1` to `seed-4`) and a landed one broken (`landed`); FlashBomb landed and broken before
the flash (`chips/0x039-flshbom1/landed`, `shot`). The action's other two
throws are chips of their own: LilBoiler (subtype 3: T3#0x93 and what it spawns, T1#0x54; chips/lilbolr) and VDoll
(subtype 8: T3#0x7A, its curse controller T4#0x4E and T4#0x11; chips/vdoll and chips/curse). shot-chips.md §14
specifies both, and their lab scenarios match.

#### 3.6.11 SpoutMan (navi chip subtype 7, T1 0x09)

Trace: soundmod rounds 1 and 2 (side 0's SpoutMan); the scratch lab's navis/0x0f2-spoutman/long{,-miss,-adjacent,
-holes} and the EX and SP `long`s match every frame. The pack's scripts: chips/spoutman (navi, ball, splash,
pillar, geyser, mark).

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

**His layer, T1 0x55 (`sub_80C40D8`)**, the navi framework's (the idle overlay `kinds::idle_overlay`, from the navi hooks in `kinds::player::form`): sprite
(0x10, 0x21) (`dword_80C40D4[Param1]`, Param1 always 0); each tick his position, visibility, palette, color
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
(long, long-miss, long-adjacent, long-holes) matches every frame. The battle ending mid-attack (each navi chip's
`ko`: an opponent of 10 HP deleted by the first hit) and no footing for the navi (each one's `no-footing`: the
user, with AirShoes, over a missing panel of the holes stage, where the navi's action 0 goes straight to its
leave) are verified for every navi chip (docs/engine/unverified.md lists them), and so is each one against an
opponent it can't hit (`invisible`), behind its own RockCube (`rock-front`) and behind a barrier (`barrier`). Unverified everywhere: a pool
with no free slot (the spawns' failure branches).

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
#### 3.6.25 GroundMan (navi chip subtype 10, T1 0x17)

Chips 0x0FB GrndMan, 0x0FC EX, 0x0FD SP (params 0x20010800, …03, …04: nothing he runs reads them). Content:
chips/grndman (navi, drill, rock; the chips, chips.luau).

**GroundMan, T1 0x17 (`sub_80BBB98`)**, spawned by `sub_80BBDE8` (`off_802CD5C[10]`) like ElmntMan: PanelX/Y,
element, the user in RelatedObject1Ptr, the user's side and flip, the damage word, the controller's flag pointer in
his CollisionDataPtr slot (the flag set to 1). Init (`sub_80BBBBC`): on his panel with Z = 0 (a word store: nothing
of the spawner's r3 is kept), sprite (8, 9) with a ground shadow, animation 0, palette 0, flip; his parts
(`sub_8010DF6(2, 9, 1)` → `sub_8010F86`: body overlay T1 0x56, Param1 0xB, Param3 1). His sprite steps while dimmed
(`object_updateSpriteTimestop`). His actions (`off_80BBC18`) enter on CurPhase 0 (setting it to 4); "n ticks" is
the ticks after the entry tick until the timer reaches 0, "Timer:Timer2 = n" a word store.
- 0 (`sub_80BBC2C`): anim 3, sound 0x94, VISIBLE, Timer:Timer2 = 9; 9 ticks; his panel's flags have all of 0x10010
  → 4, else 0x10.
- 4 (`sub_80BBC80`): anim 0, X velocity −front · 0x8000, Timer:Timer2 = 40. Each tick X moves by the velocity while
  the timer (before its decrement) is 25 down to 10 (16 ticks: 8 pixels back; his panel isn't updated); the tick
  it reaches 25: anim 0x11, sound 0xE6; 40 ticks → 8.
- 8 (`sub_80BBCD8`), the dig. Entry: anim 0x12; `sub_80BBE0E`: X velocity front · 0x80000 and Timer (a halfword)
  = |c − x| · 0x280000 / 0x80000 (a BIOS division: 5 ticks a column), c = 6 (side 0) or 1 (side 1), x his panel;
  sound 0x1C0; his drill (`sub_80E7896` with r4 = 1: T4 0x61, Param1 1) into ExtraVars[0]. Each later tick: X +=
  velocity, the panel from the coordinates; entering a new column, a hit there (`sub_80BBE3A` → `sub_80C53A6`:
  region 1, hit effect 0xA, target 5, self 0x15, modifier 3, Z 0, resolving while dimmed); then his panel against
  `byte_80BBD50[side]` (require 0x10; forbid 0x05800000, side 1 0x0A800000: solid, and no body or other object of
  the other side and no neutral object): failing → 0x10; else the timer counts down, and at 0 → 0xC.
- 0xC (`sub_80BBD60`), the rockfall (he reached the far column meeting nothing). Entry: Timer 150, Timer2 0, the
  last rock's panel (ExtraVars[1], [2]) (0, 0), sound 0xE5, a camera shake (`camera_initShakeEffect_80302a8(2,
  150)`: presentation). Every tick, the entry's too: Timer − 1, at 0 → 0x10; else Timer2 − 1, at 0 or below a rock
  (`sub_80BBE58`) and Timer2 = 25. So six rocks, on ticks 0, 25, …, 125; 149 ticks after the entry → 0x10.
- 0x10 (`sub_80BBDAC`): anim 4, Timer:Timer2 = 4; his drill, if ExtraVars[0] holds one (action 8 ran), taken off
  (`sub_80E78AE`: its state word = 8). To −1 (5 ticks): his parts off (`sub_8011044(2, 9)`), the controller's flag
  cleared, state 8 (`object_freeMemory`).

A rock (`sub_80BBE58`): with his panel set to the last rock's for the search (`object_getPanelsExceptCurrentFiltered`,
rows 3..1, columns 6..1, leaves that panel out), the panels meeting `off_80BBEE0[side]` (side 0: require
0x04000020, a side-1 body on a side-1 panel; side 1: require 0x08000000, forbid 0x20); none: those meeting
`byte_80BBEF4[side]` (side 0: require 0x20; side 1: forbid 0x20: the other side's area); none of either: no rock.
Else one draw, `GetPositiveSignedRNG2()` mod n (the BIOS division's remainder), picks the panel, which becomes the
last rock's: a T3 0x80 rock there (`sub_80D54F0`, r4 = 0x0001010A: Param1 10, Param2 1, Param3 1, Param4 0; his
element and damage word; r3 = 0). His panel is restored.

**His drill, T4 0x61 (`sub_80E7788`)**, spawned by `sub_80E7896` (RelatedObject1 the owner, its side and flip,
Params from r4; its position the caller's r1..r3 until the init). Init (`sub_80E77B8`): the owner's X, Y, Z; sprite
(0x10, 0x4F), no shadow, animation 0, palette 0, flip; VISIBLE as the owner's. Each tick (`sub_80E7816`): the
battle over → state 8 (freed). Else the owner's X, Y, Z, VISIBLE, palette, color shader, final palette, the white
flash and mosaic bytes (`sub_8002F3E`, `loc_8002F02`), DirectionFlip (then the sprite's flip) and alpha
(`sprite_getMosaicScalingParameters` → `sprite_setAlpha`); with Param1 0 it goes (state 8) once the owner's
CurAction isn't Param2. Its sprite steps as `object_updateSpriteTimestop` with Param1 set, else
`object_updateSpritePaused`. Its spawners: GroundMan (Param1 1), the link navi GroundMan's RC Brakr (`sub_80F1C78`,
Param1 0, Param2 0xA) and the navi AI (`sub_80FA81E`).

**The rock, T3 0x80 (`sub_80D535C`)**, spawned by `sub_80D54F0` (x, y, element; the damage word; the spawner's side
and flip; flags |= 0x10, running while dimmed; its position the caller's r1..r3 until the init). Init
(`sub_80D538C`): on its panel, Z = 0xA00000 (160 pixels), sprite (0x10, 5), no shadow, VISIBLE, animation 0,
palette 0, flip; collision (none: freed): self 0x15, target 5, modifier 3, hit spark 0xA, region 0; presented. Its
sprite steps as `object_updateSpriteTimestop` with Param2 set, else `object_updateSpritePaused`. Each tick
(`sub_80D53F4`): the hits resolved and the spark; the battle over → region 0, state 8 (`object_genericDestroy`).
Else a hit (any hit flags) throws off rubble (`sub_80D5516`) **and the rock goes on**; then, with Param2 1 or not
dimmed, its action (`off_80D543C`); presented again.
- 0 (`sub_80D5448`): Timer = Param1, Timer2 = 0; each later tick Timer2 + 1, its panel highlighted while Timer2's
  bit 2 is clear (4 ticks of 8); Param1 ticks → 4.
- 4 (`sub_80D5482`): a ground shadow, Z velocity 0x65000 (`byte_80D5574`), Timer 25; each later tick Z −= the
  velocity; 25 ticks: Z = 0, region 1 → 8.
- 8 (`sub_80D54C0`, the next tick): on a solid panel, rubble, and with Param3 the panel cracked
  (`object_crackPanel`); sound 0xD9 unless Param4; region 0, state 8.

Rubble (`sub_80D5516`): one draw, k = `GetPositiveSignedRNG2() & 3`; two pieces n = 0, 1 (`sub_80E1084`: effect #9,
objects/falling-rock/chip) at (X, Y, 0), priority 2 (object +0x0C), Param1 n, velocity (vx, 0, vz) =
`byte_80D5550[(k + n) & 3]`: (0x8000, 0x28000), (−0x8000, 0x30000), (0x10000, 0x28000), (−0x8000, 0x18000). The rock's
other spawners: effect #0x8A (`sub_80E9D2A`), RC Brakr (`sub_80F1E98`) and the navi AI (`sub_80FAC8A`).

Lab (the original's coverage). The official lab reaches him in chips/0x0fb..0x0fd {counter, guard, beast-charged,
cross-ground-charged}: the dig meets the opponent (8 → 0x10). The lab's long scenarios (hand-written, recorded
until he has gone) chips/0x0fb-grndman/long, long-adjacent, long-holes, long-side1 and the EX's and SP's long do
too; in long-rocks, long-rocks-miss and long-side1-rocks the dig meets the user's RockCube instead; long-miss and
long-side1-miss (nothing in his row) reach the rockfall, both rock searches and a rock's hit. All match every frame.
navi-09-rc-brakr reaches the drill's Param1 0 (its owner never leaves). The coverage scenarios verified action
0 → 0x10 (`chips/0x0fb-grndman/no-footing`), the rock's non-solid landing (`rockfall-holes`,
`rockfall-after-geddon`) and the rocks' and the drill's battle-over paths (`rockfall-ko`, `ko`). **Unverified**:
no rock candidate at all; the rock's no-collision path, Param2 0, Param3 0, Param4 set; the drill's Param1-0
leave.

#### 3.6.26 DustMan (navi chip subtype 11, T1 0x18)

Chips 0x0FE DustMan, 0x0FF EX, 0x100 SP (Param1 0, 0, 4: overwritten). Content: chips/dustman (navi, junk),
with the chips (chips.luau); the junk ball is DustCross's (navis/megaman/forms/dustcross/junk_ball).

**DustMan, T1 0x18 (`sub_80BBF0C`)**, spawned by `sub_80BC0DA` (`off_802CD5C[11]`) like GroundMan. Init
(`sub_80BBF30`): on his panel with Z's whole part 0 (a halfword store; the fraction is the spawner's r3, its
address 0x080BC0DB: 0xC0DB), sprite (8, 0xA) with a ground shadow, animation 0, palette 0, flip; no parts. Steps
while dimmed. Actions (`off_80BBF80`):
- 0 (`sub_80BBF94`): anim 3, sound 0x94, VISIBLE, Timer:Timer2 = 10; 10 ticks; 0x10010 → 4, else 0x10.
- 4 (`sub_80BBFE8`): Timer:Timer2 = 20 (anim 3 goes on); the tick it reaches 10, the junk (`sub_80BC100`) → Param1
  = how many (a byte store), sound 0xAD if any; 20 ticks → 8. `sub_80BC100`: each of the eight words of the field
  object registry (BattleState+0xA0: each side's three, then the stage's two) that is set and whose object's NameID
  passes `sub_800F486` (not 0xD3, 0xDA, 0xE9 or 0xEA): its NameID into his ExtraVars[n] (n from 0), and
  `sub_800F884` on it (the chip's removal: ObjectFlags2 0x8000 when it has collision data; counted either way).
- 8 (`sub_80BC024`): anim 0xF, Timer:Timer2 = 34; the tick it reaches 9, anim 0x11; 34 ticks → 0xC.
- 0xC (`sub_80BC058`). Entry: `sub_80BC13E`: the ball of junk (`sub_80DB800`: T3 0xB0, DustCross's junk ball; his panel,
  element and damage word, r4 = 1: Param1 1, which runs on while dimmed; the routine then loads r4 = flip << 8 |
  0x51 and r1..r3 = his X, Y, Z for an effect it never spawns); sound 0xFF; Param2 = 0; Timer = (Param1 + 1) · 20;
  Timer2 = 20. Each later tick: Timer − 1, at 0 → 0x10; else Timer2 − 1, at 0: a junk (`sub_80BC160`: `sub_80DBC90`
  on his panel with NameID ExtraVars[Param2], his element and damage word), sound 0xFF, Param2 + 1, Timer2 = 20.
  So n junk on ticks 20, 40, …, 20n, and 0x10 after 20(n + 1) ticks.
- 0x10 (`sub_80BC0B0`): anim 4, Timer:Timer2 = 4; to −1 (5 ticks): the controller's flag cleared, state 8.

**His junk, T3 0xB3 (`sub_80DBB40`)**, spawned by `sub_80DBC90` (x, y, element, RelatedObject1 him, the damage word,
ExtraVars[0] = the NameID (r7), his side and flip, flags |= 0x10). Its position is the spawner's registers: r1 = y,
r2 = element, and r3, which `sub_80BC160` never sets: the object loop's (`object_800372A`: 4 × the number of
objects of the previously updated object's type already in that type's list this tick); its Params are the loop's
r4 (nothing reads them). Init (`sub_80DBB64`): its NameID's look, `sub_800F26C`: for NameIDs 0xCD..0xFF the 5-byte
row of `byte_8021220` (sprite category, index, animation, palette, shadow); others `enemy_getStruct1(NameID)`'s
first two bytes as the sprite, animation 0, palette 0 and, as the shadow flag, whatever r4 held. A category of 0xFF:
freed. Else that sprite (no `sprite_loadAnimationData` before the animation), VISIBLE, a shadow if the flag is set
(else none), the animation, the palette; NameIDs 0xD8 and 0xD9 set bit 1 of the sprite's attribute byte +0x16
(`sub_8002EAC`) and no flip, others the flip = its side (`sprite_setFlip(alliance)`); on its panel, 18 pixels
ahead (front · 0x120000), Z's whole part 16 (a halfword; the fraction stays); collision (none: freed): self 6,
target 5, modifier 3, hit spark 0xA, presented. It steps while dimmed. Each tick (`sub_80DBC0A`): hits resolved and
the spark; the battle over or any hit → region 0, state 8 (genericDestroy). Else action 0 (`sub_80DBC48`, its only
one), then presented. Action 0's entry (`sub_80DBCBA`) sets X velocity front · 0x60000 and Timer = |X(edge) − X| /
0x60000 (a BIOS division; the edge's X `object_getCoordinatesForPanels(7, 1)`, side 1 column 0), and the same tick
goes on: Timer − 1, at 0 → region 0, state 8; else X += velocity, its panel from the coordinates, the collision's
panel updated; the hit flags read again (still 0: gone if set); off the field its region goes to 0 (it flies on,
harmless, until its timer ends).

`byte_8021220` (NameID: category, index, animation, palette, shadow; 0xFF = none):
0xCD 10 00 00 00 0; 0xCE 10 08 00 00 0; 0xCF none; 0xD0 10 00 01 00 1; 0xD1 10 00 02 00 1; 0xD2 10 00 01 00 1; 0xD3,
0xD4 none; 0xD5 0C 24 00 00 1; 0xD6 04 09 00 00 0; 0xD7 04 09 00 01 0; 0xD8 0C 23 01 00 1; 0xD9 0C 23 03 00 1; 0xDA
none; 0xDB 0C 30 00 00 1; 0xDC 0C 30 01 00 1; 0xDD..0xE1 04 0A 00 with palettes 0, 2, 4, 6, 0xC, shadow 1; 0xE2 0C 34 01 00 1;
0xE3 0C 35 00 00 1; 0xE4 04 05 00 00 1; 0xE5 0C 41 01 00 0; 0xE6 none; 0xE7 04 18 00 00 1; 0xE8..0xEA none; 0xEB 04
0D 00 00 1. Rows 0xEC..0xFF overlap the bytes after the table (`byte_80212D4`'s neighbors). The content has a row as
the `look` of the field object's identity (`define.identity { class = "field_object", look = { ... } }` in the
object's module), for the 23 NameIDs an object of the content takes; `scrap = false` is `sub_800F486`'s
exclusion (0xD3 and 0xDA; 0xE9 and 0xEA are no object's).

Lab: the official chips/0x0fe and 0x100 {counter, guard, cross-charge-charged}, and the long scenarios
chips/0x0fe-dustman/long{,-miss,-adjacent,-holes,-rocks,-rocks-miss,-side1,-side1-miss,-side1-rocks} and the EX's
and SP's long; junk only where there are rocks to take (long-rocks, long-rocks-miss, long-side1-rocks, and
cross-charge-charged). All match every frame. The coverage scenarios verified action 0 → 0x10
(`chips/0x0fe-dustman/no-footing`), the battle's end (`ko`), and his junk for every field object a chip of a
netbattle leaves (each one's `dustman`: the instruments, Sensor's turret, both fans, Anubis's and Guardian's
statues, TimeBom, BlkBomb, LilBoiler, VDoll: looks with and without a shadow, the none look), with the excluded
NameIDs (`chips/0x091-mine/dustman`). **Unverified**: NameIDs 0xD8/0xD9 and outside 0xCD..0xFF, no collision,
and the flag check after moving.

#### 3.6.27 DiveMan (navi chip subtype 13, T1 0xB)

Chips 0x104 DiveMan, 0x105 EX, 0x106 SP (Aqua). Content: chips/diveman (navi, wave; the chips, chips.luau).

**DiveMan, T1 0xB (`sub_80B99C0`)**, spawned by `sub_80B9B6E` (`off_802CD5C[13]`) like GroundMan; Z's whole part 0
(the fraction 0x9B6F, from the spawner's address 0x080B9B6F). Init (`sub_80B99E4`): `sprite_decompress(8, 0xD)`
(graphics), sprite (8, 0xD) with a ground shadow, animation 0, palette 0, flip; his parts (`sub_8010DF6(2, 0xD, 1)`
→ `sub_8010F96`: body overlay T1 0x56, Param1 3, Param3 1, Param4 0xA). Steps while dimmed. Actions (`off_80B9A48`):
- 0 (`sub_80B9A60`): anim 7, sound 0xE1, VISIBLE, Timer:Timer2 = 19; 19 ticks → 4 (no footing check).
- 4 (`sub_80B9A96`): Timer:Timer2 = 30 (anim 7 goes on) → 8.
- 8 (`sub_80B9ABA`): anim 4, Timer 30, a camera shake (0, 30) → 0xC.
- 0xC (`sub_80B9AEA`): anim 5, the waves (`sub_80B9B94`), Timer 60; the tick it reaches 30 a camera shake (2,
  30), 20 anim 0; 60 ticks → 0x10.
- 0x10 (`sub_80B9B30`): anim 8, Timer 14 → 0x14.
- 0x14 (`sub_80B9B58`, at once): his parts off (`sub_8011044(2, 0xD)`), the controller's flag cleared, state 8.

The waves (`sub_80B9B94`), rows 1, 2, 3: `object_getClosestPanelMatchingRowFiltered(side, row)` with
`byte_80B9BF0[side]` (side 0: forbid 0x20; side 1: require 0x20): from column 6 down (side 1: 1 up), the first
panel of his side's color (0 when the scan leaves the field: no wave); that panel must also meet `byte_80B9C04`
(require 0x10, forbid 0x03800000: solid, and no non-navi body of either side nor a neutral object; navis don't
stop it): a T3 0x39 wave there (`sub_80CB1E2`: his element and damage word, r4 = 4: Param1 4; Z = r3 = the row − 1,
which `_object_getPanelDataOffset` leaves in r3). A compare of the column with 1 (side 1: 6) on the way has no
effect.

**The wave, T3 0x39 (`sub_80CB0DC`)**, spawned by `sub_80CB1E2` (x, y, element, the damage word, the spawner's side
and flip, flags |= 0x10). Init (`sub_80CB10C`): on its panel, Z's whole part 0 (the fraction kept), sprite (8, 0xD)
(DiveMan's), no shadow, VISIBLE, anim 0x14, palette 0, flip; sound 0xA7; no collision of its own. Its sprite steps
as `object_updateSpriteTimestop` for Param1 4, else `object_updateSpritePaused`. Each tick (`sub_80CB154`): the
battle over → state 8 (genericDestroy); else, with Param1 4 or not dimmed, action 0 (`sub_80CB184`). Entry: a hit on
its own panel lasting 32 (`sub_80CB208(0, 32)`), Timer 32. Each later tick Timer − 1: at 13 hits on the next two
panels toward the other side lasting 13 (`sub_80CB208(1, 13)`, `(2, 13)`); at 7 the splash on the second
(`sub_80CB248(2)`); at 0 the splash on the first (`sub_80CB248(1)`), state 8, Timer2 = 0.
- `sub_80CB208(n, ticks)`: the panel n ahead (`object_getEnemyDirection`: by side), skipped when invalid;
  `sub_80C53A6` (region 1, hit effect 2, target 5, self 0xA, modifier 3, Z 0, resolving while dimmed) and that hit
  region's Timer = ticks (T3 #3, `sub_80C52B0`, resolves every tick, `Timer` ticks in all, and ends at its first
  hit).
- `sub_80CB248(n)`: when the panel n ahead is valid, T4#0 effect 0x2B at its center, Z 0, flip = the wave's flip
  byte.

Lab: the official chips/0x104 {counter, guard, beast-charged, cross-spout-charged} and the long scenarios
chips/0x104-diveman/long{,-miss,-adjacent,-holes,-rocks,-rocks-miss,-side1,-side1-miss,-side1-rocks} and the EX's
and SP's long (some rows' panels refused). All match every frame. **Unverified**: a row without a panel of his color; the wave's Param1 other than 4 (the navi AI's: it
waits a dimming out, sprite paused), the battle's end, invalid panels ahead, a failed hit spawn.

#### 3.6.28 CircusMan (navi chip subtype 14, T1 0xE)

Chips 0x107 CrcusMan, 0x108 EX, 0x109 SP. Content: chips/crcusman (navi; the chips, chips.luau).

**CircusMan, T1 0xE (`sub_80BA364`)**, spawned by `sub_80BA660` (`off_802CD5C[14]`); Z's whole part 0 (the fraction
0xA661). Init (`sub_80BA388`): `sprite_decompress(8, 0xE)`, sprite (8, 0xE), no shadow, animation 0, palette 0,
flip; his parts (`sub_8010DF6(2, 0xE, 1)` → `sub_8010FD8`: body overlays T1 0x56 Param1 6 (Param4 0x14) in
RelatedObject2 and Param1 7 (Param4 0x28) in his ExtraVars[0], Param3 1 each; `sub_8011044(2, 0xE)` takes both off).
Steps while dimmed. Actions (`off_80BA3EC`); the Z moves are halfword stores to Z's whole part:
- 0 (`sub_80BA414`): anim 0x13, sound 0x94, VISIBLE, Timer:Timer2 = 10; 10 ticks; 0x10010 → 4, else 0x24.
- 4 (`sub_80BA468`): anim 0, Timer:Timer2 = 30 → 8.
- 8 (`sub_80BA490`): anim 0xD, Timer 19 → 0xC.
- 0xC (`sub_80BA4BA`): entry: PanelX += 3 · front (a byte store, unchecked), coordinates from the panel, Z 30, anim
  0xE, Timer:Timer2 = 5. Each later tick Z − 6; 5 ticks: his panel's flags & `off_80BA520[side]` (side 0:
  0x04000000, side 1: 0x08000000: an enemy navi's body) → 0x10; else (off the field too: flags 0) → 0x1C.
- 0x10 (`sub_80BA528`), the catch: anim 0xF, Y's and Z's whole parts + 1 (`sub_80BA6A4`), sound 0x1AA, Timer 46;
  the tick it reaches 21, sound 0x114; 46 ticks → 0x14.
- 0x14 (`sub_80BA56C`): anim 0x10, Timer 72, Timer2 0, and on the same tick: Timer2 − 1, at 0 or below a hit
  (`sub_80BA686` → `sub_80C53A6` on his panel: region 1, no hit spark (0xFF), target 5, self 0x16, modifier 3, Z 0,
  resolving while dimmed) and a sparkle (`sub_80BA6B6`), Timer2 = 12; Timer − 1, at 0 → 0x18. Six hits, on ticks
  0, 12, …, 60; 72 ticks in all. The sparkle: T4#0 effect 0x21 (flip 0) at X = his panel's center ± (r & 0x1F)
  pixels, Y = its center + 16 pixels, Z = 36 ± (r′ & 0xF) pixels; r, r′ two `GetPositiveSignedRNG2` draws in that
  order, each sign + when its draw's bit 0 is set.
- 0x18 (`sub_80BA5AA`): anim 0x11, Timer 7 → 0x20.
- 0x1C (`sub_80BA5D2`), the miss: Timer 20 (anim 0xE goes on) → 0x20.
- 0x20 (`sub_80BA5F6`): anim 0x12, Timer 5; each later tick Z + 6; 5 ticks: his parts off, the controller's flag
  cleared, state 8.
- 0x24 (`sub_80BA62E`), no footing: anim 0xD, Timer 19; 19 ticks: his parts off, the flag cleared, state 8.

Lab: the official chips/0x107 and 0x109 {counter, guard} (the catch) and cross-charge-charged (the miss), and the
long scenarios chips/0x107-crcusman/long, long-holes, long-rocks, long-side1, long-side1-rocks, the EX's and SP's
long (the catch), long-miss, long-adjacent, long-rocks-miss, long-side1-miss (the miss). All match every frame.
**Unverified**: 0x24 (no footing)
and a drop off the field.

#### 3.6.29 JudgeMan (navi chip subtype 15, T1 0xF)

Chips 0x10A JudgeMan, 0x10B EX, 0x10C SP (Elec; Param1 20, 30, 40: his books' damage). Content: chips/judgeman
(navi, whip, book; the chips, chips.luau: `navi.summon { book_damage }`).

**JudgeMan, T1 0xF (`sub_80BA708`)**, spawned by `sub_80BA920` (`off_802CD5C[15]`); Z's whole part 0 (the fraction
0xA921). Init (`sub_80BA72C`): `sprite_decompress(8, 0xF)`, sprite (8, 0xF) with a ground shadow, animation 0,
palette 0, flip; no parts. Steps while dimmed. Actions (`off_80BA784`):
- 0 (`sub_80BA7A0`): anim 3, sound 0x94, VISIBLE, Timer:Timer2 = 2; 2 ticks; 0x10010 → 4, else 0x18.
- 4 (`sub_80BA7F4`): anim 0, Timer:Timer2 = 30 → 8.
- 8 (`sub_80BA81C`): anim 6, his whip (`sub_80E71A0` with r4 = 1: T4 0x5A, Param1 1) into ExtraVars[0], Timer 15 →
  0xC.
- 0xC (`sub_80BA84C`): anim 7, the lash (`sub_80BA946`: `sub_80C53A6` on the panels 1, 2 and 3 ahead in his row:
  region 1, hit effect 3, target 5, self 0x19, r7 = 0x1101: modifier 1, status 0x11; his element and damage word, Z
  0; an invalid panel's hit frees itself), sound 0xBA, Timer 30; 30 ticks: the whip taken off (`sub_80E71B8`: its
  state word = 8) if ExtraVars[0] is set; the books' panels (`sub_80BA9A4`) → ExtraVars[1] = how many; some → 0x10,
  else 0x18. `sub_80BA9A4`: columns `byte_80BA9F8[side]` (5, 4, 3, 2; side 1: 2, 3, 4, 5), rows 1..3 in each: a
  panel his side owns by home (PanelData+4 = his side) that the other side holds (`sub_800D618`) and that meets
  require 0x10 / forbid 0x0F880080 (solid, nothing on it, not reserved) is packed as x | y << 4 into his bytes
  +0x68 on (ExtraVars[2..]).
- 0x10 (`sub_80BA89C`): anim 5, Timer 30; the tick it reaches 15, the books (`sub_80BAA0C`); 30 ticks → 0x14.
  `sub_80BAA0C`, for each packed panel i: `sub_80D7C7E` (T3 0x94) with element 0, r4 = i | 1 << 8 (Param1 i, Param2
  1), the damage word his Param1 | (his damage & 0xF000) | his stamina half << 16, r7 = &his ExtraVars[5] (the
  books-done count), r3 = his damage & 0xF000 (the book's Z: it keeps that as its fraction).
- 0x14 (`sub_80BA8D0`): anim 0 on entry; each tick (the entry's too) once ExtraVars[5] = ExtraVars[1] (every book
  done): `sub_80BAA4C`, → 0x18. `sub_80BAA4C`: in each column (5, 4, 3, 2; side 1: 2, 3, 4, 5) the first row 1..3
  with a panel of his home the other side holds (`sub_800D618`) gets `object_setPanelAllianceTimerShort`: that
  column's return timer is cut short (the column comes back).
- 0x18 (`sub_80BA8F6`): anim 4, Timer 2; 2 ticks: the controller's flag cleared, state 8.

**His whip, T4 0x5A (`sub_80E70C8`)**, spawned by `sub_80E71A0` (RelatedObject1 the owner, its side and flip, Params
from r4). Init (`sub_80E70F8`): the owner's X, Y, Z; sprite (8, 0xF) (JudgeMan's), no shadow, VISIBLE, anim 8,
palette 0, flip. Its sprite steps as `object_updateSpriteTimestop` with Param1 set, else
`object_updateSpritePaused`. Each tick (`sub_80E7142`): the battle over → state 8 (freed). Else the owner's X, Y, Z;
with Param1 0 it goes once the owner's CurAction isn't 0xA, and does nothing more while dimmed; then
(`sub_80E717A`) action 0 → action 4 with Timer 15, counting the same tick; action 4: Timer − 1, at 0 anim 9 and
action 8, which holds. `sub_80E71B8(whip)`: its state word = 8.

**The book, T3 0x94 (`sub_80D7ACC`)**, spawned by `sub_80D7C7E` (x, y, element, RelatedObject1 him, the damage word,
ExtraVars[0] = the done count's address, his side and flip, flags |= 0x10; a failed spawn adds 1 to the count at
once). Init (`sub_80D7AF0`): on its panel, Z's whole part 0, sprite (8, 0xF) (no `sprite_loadAnimationData` before
the animation), ground shadow, VISIBLE, anim 0xA, palette 0, flip; collision (none: the count + 1, freed): self 4,
target 5, modifier 1, hit spark 6, presented. Steps while dimmed. Each tick (`sub_80D7B5C`): hits resolved and the
spark; the battle over, or a hit flag of `byte_80D7BA8[side]` (0x05800000; side 1 0x0A800000: the other side's body
or object, or a neutral object) → region 0, CurState 8 (a byte store; the action stays). Else its action
(`off_80D7B9C`), presented again. Its destroy (`sub_80D7C6E`): the count + 1, genericDestroy.
- 0 (`sub_80D7BB0`): sound 0x94, Timer = Param1 · 12 + 30; to −1 (Param1 · 12 + 31 ticks) → 4: the books set off
  one after another, 12 ticks apart.
- 4 (`sub_80D7BE0`). Entry: anim 0xB, its target (`sub_80D7CCC`) into ExtraVars[1], [2], its heading
  (`sub_80D7D4C`). Every tick, the entry's too: c = its panel's center on its axis (Param2 0: X; else Y); it moves by
  its X and Y velocities; unless its old coordinate was c, if it reached or passed c (`sub_800E708(new, old, c)`:
  new = c, old = c, or c between them): on the target's panel (`sub_80D7DC0`: `sub_800E258(X, Y)` = the target) it
  ends (region 0, CurState 8), else it heads again. Then its panel from its coordinates, the collision's panel
  updated; a panel without 0x10 (not solid, or off the field) ends it.
- The target (`sub_80D7CCC`): from its side's back column (1; side 1: 6) toward the front along its row, the first
  panel with an enemy navi's body (`object_getFirstPanelInDirectionFiltered`, require `off_80D7D44[side]`:
  0x04000000, side 1 0x08000000); none: from that column toward the front, the first column with one
  (`object_getPanelsInColumnFiltered`, rows 1..3), its first such row; none at all: (7, its row) (side 1: 0).
- The heading (`sub_80D7D4C`), two tries at most: the axis other than the current one (Param2) if the target
  differs from its panel along it, else the current axis if it differs; neither: both velocities negated. X: Param2
  = 0, X velocity ±0x24000 (2.25 pixels a tick) toward the target's column, Y velocity 0; Y: Param2 = 1, the same
  in Y. Each outcome snaps it to its panel's center (`object_setCoordinatesFromPanels`) and sets its sprite's flip
  from its X velocity (`sub_80D7CB0`: > 0 → 0, < 0 → 1, 0 → unchanged; raw flips, not by side). The books start with
  Param2 1, so their first heading tries X.

Lab: the official chips/0x10a..0x10c {counter, guard, beast-charged, cross-elec-charged} (no panel of his taken: no
books), the long scenarios chips/0x10a-judgeman/long{,-miss,-adjacent,-holes,-rocks,-rocks-miss,-side1,-side1-miss,
-side1-rocks} and the EX's and SP's long; the books in long-grabbed, long-grabbed-up, long-side1-grabbed and the
EX's and SP's long-grabbed{,-up} (the opponent's AreaGrab took his front column first). All match every frame, and
soundmod round 3 has both sides' JudgeMan in one dimming (a counter cut-in), the first with three books and his
columns coming back a tick apart (field-collision-damage.md §2.6.3). The coverage scenarios verified action 0
→ 0x18 (`chips/0x10a-judgeman/no-footing`), the whip's battle-over path (`ko`), and a book arriving at its target
(`books-invisible`: an invisible navi keeps its body on its panel, so the books fly there and end), leaving solid
ground (`books-holes`) and ending with the battle (`books-ko`). **Unverified**: the whip's Param1 0 (the navi
AI's); a book's failed spawn or collision, the heading's reversal, and the target past the far edge (no enemy
navi at all).

#### 3.6.30 TwinLdrs (navi chip subtype 20, PA chip 0x15C, T1 0x20)

ProtoMn[SP] B + AntiNavi * + Colonel *. Content: chips/twinldrs (navi, chip). ProtoMan leads
and brings Colonel; both are T1 0x20, told apart by Param1.

**The spawner, `sub_80BD9A2`** (`off_802CD5C[20]`): `object_spawnType1(0x20)` with the caller's r1..r3 as its
position and r4 as its Params (the chip's: Param1 0); PanelX/Y, element, the damage word (r6), RelatedObject1 = r5
(the user), ExtraVars[0] = r7 (the controller's flag pointer; the flag set to 1 when r7 isn't 0), the user's side
and flip, flags |= 0x10 (it runs while dimmed). **The object, `sub_80BD388`**: its sprite steps as
`object_updateSprite` (so while dimmed, having flag 0x10). Init (`sub_80BD3AC`), by `byte_80BD464[Param1]` (8 bytes:
sprite category (bit 7: `sprite_decompress` first), index, NameID, dx, dy): Param1 0 ProtoMan, (8, 0xB), NameID
0x13D, (+10, +10); Param1 1 Colonel, (8, 0x12) decompressed, NameID 0x167, (−10, −10). The NameID; the sprite with a
ground shadow, CurAnim 0 (loaded 0xFF); X, Y, Z = the user's (RelatedObject1's) + (dx · front, dy, 0) pixels, kept
at +0x40 as his home; the panel from the coordinates; Z = 0 (a word); VISIBLE; flip; his parts
(`sub_800F29C(NameID)` → actor record (type 1; AI index 0xB ProtoMan: none, 0x12 Colonel: `sub_8010FAC`, body
overlay T1 0x56 Param1 0, Param3 1, Param4 0xD, his cape) → `sub_8010DF6(type, index, 1)`). ProtoMan then spawns
Colonel (`sub_80BD9A2` with r5 = the user, r4 = 1, r7 = 0: no flag; his panel, element and damage word) and they
keep each other in ExtraVars[1] (a failed spawn writes through a null pointer and leaves ProtoMan's link 0). Its
first update at once. Updates (`sub_80BD478`): Param1 0 by `off_80BD4A0` (actions 0, 4, 8, 0xC), Param1 1 by
`off_80BD4B0` (0, 4, 8). Phases here enter on PhaseInitialized 0.

ProtoMan (Param1 0):
- 0 (`sub_80BD4BC`), each tick the phase and then the color shader from his gray level Param4 (`sub_80BD9D4`:
  Param4 · 0x421). Phase 0 (`sub_80BD4DC`): entry: shader 0x7FFF, Param4 = 31, sound 0x94, Timer 0. Then each tick
  VISIBLE, Timer + 1 (odd: VISIBLE off: he flickers in), Param4 − 2; at 0 or below VISIBLE, Param4 0, phase 4 (16
  ticks). Phase 4 (`sub_80BD538`): Timer 30; 30 ticks: his panel not solid → action 0xC; else FuturePanel = (x +
  front, 1), ExtraVars[3] = 1 (a fresh search), Param3 = 0 (targets so far) → action 4.
- 4 (`sub_80BD586`), the slashes, phases `off_80BD598`:
  - 0 (`sub_80BD5A8`), the search: FuturePanel off the field → action 8. From FuturePanel (x, y): row 1 when
    ExtraVars[3] is set, else the row after the last target's; rows to 3, then the next column (x + front) from
    row 1, while 1 ≤ x ≤ 6. A panel with an enemy navi's body (`off_80BD63C[side]`: 0x04000000, side 1
    0x08000000) whose front panel (x − front, y) is his user's (RelatedObject1's panel) or solid with none of
    0x0F800000 is the target: FuturePanel = it, ExtraVars[3] = 0, Param3 + 1, phase 4. None → action 8.
    (RelatedObject1 is cleared at his first slash; later searches read the user's panel through a null pointer, as
    ProtoMan's navi chip does.)
  - 4 (`sub_80BD644`): anim 4, his sword (ExtraVars[2]) taken off (`sub_80B8E58`: its state word = 8),
    RelatedObject1 = 0, Timer 3; on the first target (Param3 1) an afterimage (`sub_80BDA7C(0)`); 3 ticks → 8.
  - 8 (`sub_80BD67E`): anim 4, Timer 0, onto the target's front panel (x − front, y), coordinates from it; 1 tick →
    0xC.
  - 0xC (`sub_80BD6B6`), the slash: anim 5 with CurAnimCopy 6 (it restarts), his sword (`sub_80B8E30`, r4 =
    0x10B11: T1 5, attachment 0x11 (sprite (0xC, 0)), animation 0xB, Param3 1; its pointer in ExtraVars[2]), sound
    0xB0, Timer 10, afterimages (`sub_80BDA7C(5)`). The tick the timer reads 5 (before its decrement): a hit on the
    panel ahead (`sub_80C53A6`: Z 16 pixels, r4 0x0405FF04: region 4, no hit spark, target 5, self 4; modifier 3;
    his element and damage word), T4#0 effect 0x27 at its center, 16 pixels up (flip his), a camera shake (1, 10)
    and a palette flash (`sub_80E11E0`, r4 0x10A00: 10 ticks). 10 ticks → phase 0 (the next search).
- 8 (`sub_80BD742`), back: phase 0 (`sub_80BD760`): anim 4, the sword off, Timer 3, counting from the entry: 3 ticks
  → 4. Phase 4 (`sub_80BD788`): anim 7 and his sprite's flip toggled (`sprite_setFlip(flip ^ 1)`) when he slashed
  (Param3 ≠ 0), else anim 3; back to his home (+0x40), the panel from it, Z 0, Timer 3 (PhaseInitialized = 8),
  counting from the entry: 3 ticks → 8. Phase 8 (`sub_80BD7D6`): once Colonel's CurAction (ExtraVars[1]'s byte +9)
  is 8 → action 0xC.
- 0xC (`sub_80BD7EA`, Colonel's 8 too): anim 4, the sword off, Timer 4, counting from the entry to −1 (5 ticks):
  VISIBLE off, state 8. His destroy (`sub_80BD984`): his parts off (`sub_8011044` by his actor record), the
  controller's flag (ExtraVars[0]) cleared if he has one, freed.

Colonel (Param1 1):
- 0 (`sub_80BD81A`): phase 0 (`sub_80BD834`): sound 0x94, Timer 4, anim 3; 4 ticks: Param3 = 0; his panel solid →
  action 4, else phase 4 (`sub_80BD878`: 40 ticks → action 8).
- 4 (`sub_80BD89C`): the phase, then each tick his charge (`sub_80BDAE0`, by Param3):
  - 0 (`sub_80BDB04`): T4#0 effect 0x4E at his attach point 0 (`sub_8018810(NameID, 0, side, flip)`, added to X
    and Z) with Timer 0x7FFF, kept in ExtraVars[5]; sound SOUND_BUSTER_CHARGE; Timer2 0; Param3 = 4.
  - 4 (`sub_80BDB3C`): Timer2 + 1; once ProtoMan's CurAction is 8 and Timer2 ≥ 40: the glow's Timer = 1, T4#0
    effect 0x4F at the attach point with Timer 30, sound SOUND_UNK_72, Timer2 0, Param3 = 8.
  - 8 (`sub_80BDB8C`): Timer2 + 1; at 15 Param3 = 0xC. 0xC: nothing.
  Phase 0 (`sub_80BD8BC`) waits for Param3 0xC; then anim 5, his sword (`sub_80B8E30`, r4 0x11C1F: attachment 0x1F
  (sprite (0x10, 0x5A)), animation 0x1C, in ExtraVars[2]), the target (`sub_80BDA08`), two hits there (element 0, Z
  0, modifier 3, his damage word; regions 0x12 and 0x13: r4 0x0405FF12 and 0x0405FF13, no hit spark, target 5, self
  4), T4#0 effects 0x36 and 0x37 at its center (flip = his side), sound 0xC7, a camera shake (3, 0x23), a palette
  flash (r4 0x12800: 40 ticks); 30 ticks → phase 4 (`sub_80BD960`: 20 ticks → action 8).
  The target (`sub_80BDA08`): along his row from the panel ahead, the first panel with an enemy navi's body
  (`off_80BDA68[side]`); off the field first: column `dword_80BDA70[side]` (5; side 1: 2), a column further when
  that is his own.
- 8: `sub_80BD7EA`, as ProtoMan's 0xC.

The afterimages (`sub_80BDA7C(n)`, `sub_80E33FA`: effect #0x28 of his side at his X, Y, Z, color shader 0x8318, 30
ticks, blinking, no tether): with n 5 first the sword's (attachment 0x11's sprite, animation 0xB, his flip; the
shadow at its height), then always his own (sprite (8, 0xB), animation n, his flip; a ground shadow).

Lab: the official pa/0x15c-twinldrs recipes (which now match every frame, group H's banner and hand being ported)
end during the controller's 30-tick warp-out, before he appears; the long scenarios pa/0x15c-twinldrs/long{,-miss,
-adjacent,-holes} reach both, with every slash landing (one target), and match every frame. The coverage
scenarios verified his footing failing and Colonel's (`pa/0x15c-twinldrs/no-footing`), ProtoMan leaving without a
slash (`rock-front`: the opponent's RockCube on the panel in front of it) and the battle's end (`ko`).
**Unverified**: more than one target; Colonel's target off the field; a failed Colonel spawn.

#### 3.6.31 CrosOver (navi chip subtype 21, PA chip 0x15D, T1 0x21)

Django D + Django2 D + Django3 D: MegaMan (Param1 0) with his buster and sword, and Django (Param1 1) with his Gun
del Sol. Content: chips/crosover (navi, chip).

**The spawner, `sub_80BE3E8`** (`off_802CD5C[21]`), as TwinLdrs's (T1 0x21; ExtraVars[0] the flag pointer, set to
1). **The object, `sub_80BDBA4`**, its sprite as `object_updateSprite`. Init (`sub_80BDBC8`):
- MegaMan (Param1 0): NameID = the user's when it is 0x1A0 or above 0x1AB (MegaMan and his forms; ExtraVars[4] =
  1), else 0x1A0 (a link navi user, 0x1A1..0x1AB; ExtraVars[4] = 0); the user's AIData pointer; the user hidden
  (`sub_80E1352(user, 0xF)`: VISIBLE off, its companions kept). Sprite: ExtraVars[4] 0: (0, 0); else, the NameID's
  actor record's type 2 (a player): `sub_800FC9E(stats[0x29], stats[0x2C])` (navi 0: (0, `byte_800FCBC[form]`),
  else (8, `byte_800FCD5[navi]`)); another type: `sub_800F26C(NameID)`. X, Y, Z the user's (the panel from them);
  his parts (`sub_8010DF6` by the record); palette `byte_80203EA[stats[0x2C]]` (the form's). ExtraVars[1] = 0; the
  target (`sub_80BE434`) → FuturePanel; when it gives Django a panel, Django (`sub_80BE3E8` with r5 = the user, r4
  = 1, r7 = 0, that panel), linked both ways in ExtraVars[1], with MegaMan's FuturePanel and DirectionFlip 1
  (facing back).
- Django (Param1 1): `sprite_decompress(0xC, 0xF)`, sprite (0xC, 0xF), on his panel.
- Both: a ground shadow, CurAnim 0 (loaded 0xFF), VISIBLE, flip, Param4 = 0; the first update at once.
- The target (`sub_80BE434`): along MegaMan's row from the panel ahead, the first panel with an enemy navi's body
  (`off_80BE4C8[side]`) is the target; Django's panel is two columns beyond it (+ 2 · front) if solid with none of
  0x0F800000, else the far column (6; side 1: 1) of that row if it is; else none (0, 0). Off the field before a
  target: target (0, 0), and Django's panel the far column's (as above) or none.

MegaMan's actions (`off_80BDD24`):
- 0 (`sub_80BDD40`): sound 0x94, Timer 60, anim 3; 60 ticks → 4 (CurAction + 4).
- 4 (`sub_80BDD70`): on to 8 once there is no Django or Django's CurAction is 4.
- 8 (`sub_80BDD88`), the buster, phases `off_80BDD9C`: 0 (`sub_80BDDA8`): Timer 0, one tick; Timer2 = 12 → 4. 4
  (`sub_80BDDCA`), a shot: anim 0xE (loaded 0xFF), Timer 10, sound SOUND_BUSTER_6A; the arm (ExtraVars[2]) off and
  a new one: ExtraVars[4] set: the buster arm by the user's form (`sub_80EB572(&ExtraVars[2], 1)`, action 0x11's);
  else attachment 6 (r4 0x10006); the muzzle flash (ExtraVars[3]) off and a new one (attachment 5, r4 0x10005);
  `sub_80C44D2(RelatedObject2)` (his body overlay restarted); a projectile (`sub_80C4FFE`: the panel ahead, Z 16
  pixels, r4 0x1D: kind 0x1D, r6 0x0083001E: damage 30, hit parameter 0x83; flags |= 0x10). 10 ticks (the entry's
  included): Timer2 − 1; above 0 the entry again (a shot every 10 ticks), else → 8. Twelve shots.
  8 (`sub_80BDE58`): anim 0, the arm and the flash off, Timer 10, to −1 (11 ticks): no target (FuturePanelX 0) →
  his parts off, VISIBLE off, state 8, the user shown (`sub_80E13DC`); else → action 0xC.
- 0xC (`sub_80BDEC0`), the sword, phases `off_80BDED4`: 0 (`sub_80BDEF0`): 30 ticks → 4. 4 (`sub_80BDF0E`): anim 4,
  the attachments off, 4 ticks → 8. 8 (`sub_80BDF3C`): anim 3; the target's front panel (FuturePanelX − front,
  FuturePanelY), when solid with none of 0x07800000 (side 1: 0x0B800000: any body but his own side's navi's):
  there, Param4 = 1; else VISIBLE off, Param4 0; 3 ticks → 0xC, or (Param4 0) 0x18. 0xC (`sub_80BDFA6`), the
  slash: anim 5, sound 0xB0; the sword: the user's AIData type and index saved (+0x74, +0x75) and set to MegaMan's
  (2, 0) while the look is picked (r4 = `sub_80EBB34()` | `sub_80EBAE8()` << 8 | 1 << 16 | `sub_80EBB78()` << 24,
  action 0x13's sword helpers: attachment, animation by form), `sub_80B8E30` into ExtraVars[2], restored; Timer 0.
  The tick the timer reads 10: Django in position (his Param4 ≠ 0): effect 0x36, region 0x13, a camera shake (3,
  0x23), a flash of 40 ticks (r4 0x12800); else effect 0x18, region 1, shake (1, 0x19), flash of 30 (0x11E00): the
  effect at the panel ahead's center, 16 pixels up (flip his), and a hit there (`sub_80C53A6`: Z 0, r4 0x0405FF13
  or 0x0405FF01: no hit spark, target 5, self 4; modifier 3; his element and damage word). Timer + 1; past 50 →
  0x10. 0x10 (`sub_80BE098`): the attachments off, anim 4, 4 ticks: VISIBLE off → 0x14. 0x14 (`sub_80BE0CE`): anim
  3, onto the user's panel, 3 ticks → 0x18. 0x18 (`sub_80BE0FA`): 10 ticks: his parts off, VISIBLE off, state 8.

Django's actions (`off_80BDD34`):
- 0 (`sub_80BE144`): sound 0x94, Timer 60, anim 1; 60 ticks → 4.
- 4 (`sub_80BE174`): 0 (`sub_80BE190`): anim 3, Timer 10, his gun (`sub_80B8E30`, r4 0x1080B: attachment 0xB,
  animation 8, in ExtraVars[2]; row 0xB is the blades' sheet (0xC, 0) in the US games, Django's sprite (0xC, 0xF) in
  the Japanese games, whose look the content has), sound 0xF8; 10 ticks (the entry's included): the sun beam (`sub_80E5D12`: effect
  #0x48, chips/gundels/beam, offset (80 · front, 0, 0) pixels from him in its velocity, r4 0x10000: look 0, palette
  0, Param3 1: it goes on while dimmed; r7 = &ExtraVars[3], where it is kept) → 4. 4 (`sub_80BE1DC`): anim 4, the
  gun's animation 9 (`sub_80B8E70`), Timer 120; every tick, the entry's too, a hit on the panel two ahead (element 5,
  Z 0, r4 0x1705FF04: region 4, no hit spark, target 5, self 0x17; damage 3, modifier 0: the silent drain of
  GunDelSol, §4.4); 120 ticks: the beam's state word = 8 (`sub_80E5D3E`) → action 8.
- 8 (`sub_80BE22E`), phases `off_80BE240`: 0 (`sub_80BE254`): anim 0, the attachments off, 30 ticks. 4
  (`sub_80BE282`): anim 2, 4 ticks: no target → VISIBLE off, state 8; else 8. 8 (`sub_80BE2B8`): anim 1; the
  panel behind the target (FuturePanelX − front, he faces back) if solid with none of 0x0F800000: there, Param4 =
  1, 3 ticks → 0xC; else VISIBLE off, state 8. 0xC (`sub_80BE312`): anim 5, sound 0xB0, Timer 0; at 10 effect and
  hit as MegaMan's (0x36 / region 0x13 when MegaMan's Param4 is set, else 0x18 / region 1), no shake or flash;
  past 50 → 0x10. 0x10 (`sub_80BE39A`): anim 2, 4 ticks: VISIBLE off, state 8.
- The destroy (`sub_80BE3C4`): MegaMan's parts off (Param1 0), the controller's flag (ExtraVars[0], MegaMan's)
  cleared, freed.

Lab: the official pa/0x15d-crosover/recipe1 ends before they appear (it matches every frame); the long scenarios
pa/0x15d-crosover/long{,-miss,-adjacent,-holes} reach the path with a target and both in position, and match every
frame. **Unverified**: a link navi or
non-player user (NameID 0x1A1..0x1AB, ExtraVars[4] 0, attachment 6), no target, no Django (or his panel taken),
either one's front panel refused, the far-column fallback, a missing beam.

#### 3.6.32 SunMoon (navi chip subtype 25, PA chip 0x15B, T1 0x24)

The pack's chips/sunmoon (sun, meteor, moon_beam). **SunMoon, T1 0x24 (`sub_80BF260`)**, spawned by `sub_80BF6AE`
on the user's panel (related1 the user, the controller's flag pointer in ExtraVars[0], flags \|= 0x10), its sprite
(0x0C, 0x64) 64 pixels up; its sprite steps as `object_updateSprite` (not while dimmed). Its handlers set their
timer on entry and count it the same tick. Actions:
- 0: sound 0x94; 60 ticks. 4.0: animation 2; a T3 0xB5 meteor every 15 ticks, six (element Fire, damage 0x32 with
  the damage's flag bits, its height SunMoon's Z). 4.4: animation 1, 30 ticks.
- 8.0: animation 1, sound 0x110; 30 ticks with the palette blinking 4/0 (`sub_80BF402`: the tick count's bit 1
  above 20 left, bit 2 below; its third rate is unreachable); palette 4. 8.4: 30 ticks.
- 0xC.0: animation 3, palette 0, the T3 0xB6 moonlight (element 0, damage 2 with the flag bits); 100 ticks. 0xC.4:
  animation 1, palette 4, 30 ticks. 0x10.0: as 8.0 inverted, ending on palette 0. 0x10.4: 30 ticks.
- 0x14.0: 10 ticks (a camera shake); then 16 ticks toward the panel three columns toward the enemy (velocity =
  distance / 16, truncated; sound 0x17F). 0x14.4: landing (Z 0) on a panel with any of 0x0F800010: sound 0xC3, a
  region 0xF hit (hit effect 0xFF, target 5, self 0xA, modifier 3, resolving while dimmed), the region's panels
  cracked (`sub_80DB48A`, offsets as they are), T4#0 effect 5 on its panels on the field (`sub_801BD3C`, dx toward
  the side, Z 3 raw: the r7 left over), a 30-tick palette flash; on another, a T4#0 effect 0x12 16 pixels up.
  Hidden. 0x14.8: 60 ticks; state 8, which clears the controller's flag.

**The meteor, T3 0xB5 (`sub_80DBEE6`)**: record `byte_80DBEE0[Param1]` (only record 0: sprite (0x0C, 0x31), self
type 0xA, target 5, a camera shake, nothing done to an occupied panel). 20 pixels ahead and 20 below its spawner's
height, aimed at the panel three columns ahead (16 ticks); collision modifier 3, hit effect = element & 0xF, region
0 while it falls. Landing on a panel with any of 0x0F800010: T4#0 effect 5, sound 0x70; else the panel breaks
(`object_breakPanel`, which only breaks a solid, unoccupied one). Then region 1 for a tick, hidden, gone.

**The moonlight, T3 0xB6 (`sub_80DC0E8`)**: SunMoon's sprite, animation 4, 40 pixels ahead. After 3 ticks: a hit
on the panel three columns ahead with damage 0x2000 (an uninstall), self type 0x17; then every tick for 100 ticks a
hit there with its damage, self type 0x30, modifier 3 (sound 0x111 every 8 ticks); all resolve while dimmed.

Lab (scratch, the PA chip put straight in the folder): the opponent a row up and both a column forward match every
frame; on a hit the replay stopped at the uninstall's reaction (`sub_80140EE`, the navi framework's). That reaction
(hit reaction 0xF8, `sub_80139F6` → `sub_80140EE`; field-collision-damage.md) is now ported (kinds/player/intake.rs
`strip_programs` and the form's NaviCust refresh, `form::refresh_form_flags`) and verified by the lab's uninstall
scenarios (`chips/0x0b9-uninstll/folded` and its variants, docs/engine/unverified.md).

#### 3.6.33 Bass (navi chip subtype 26, Giga chip 0x12D, T1 0x4F)

The pack's chips/bass/navi and objects/panel-strike. **Bass, T1 0x4F (`sub_80C3970`)**, spawned by `sub_80C3B30` on the user's
panel (no related1: the controller's flag pointer is kept in his X velocity). Init: sprite (8, 0x13), a ground
shadow, his cape (`sub_80C468C`: form overlay T1 0x57 of the same sprite, animation + 0x14, stepping while dimmed;
related1), sound 0x94, 100 ticks, and his first tick at once; he goes when his panel is off the field.
- 0: 100 ticks, rising half a pixel a tick over the last 81.
- 4.0: animation 0xA until its last frame; animation 0xC, 24 volleys, the 18 panel cooldowns cleared.
- 4.1: every tick the cooldowns count down. Each volley (8 ticks apart): two shots (`sub_80C3B54`): his columns
  (`sub_80C3C2C`: from column 3 facing right, 4 facing left, or his own if further; each further column with a
  panel of the other side's, `byte_80C3C90`); five times in eight (`GetPositiveSignedRNG2 & 7 >= 3`) a random panel
  holding an enemy navi's body, else (or if none) any panel with 0x10000, rows 3 to 1, not shot in the last 22 ticks
  (one RNG draw each pick with candidates); a T3 0x09 strike there (element 0, Param2 20, Param4 1, resolving while
  dimmed; its Z 0) and T4#0 effect 0x57 20 pixels ahead and 34 up, jittered by up to 7 pixels (one draw).
- 4.8: 12 ticks, animation 0, 30 ticks; state 8: T4#0 effect 0x12 16 pixels up, the cape off, the controller's
  flag cleared, freed.

**The panel strike, T3 0x09 (`sub_80C5DDC`)**, Bass's and MachGun's: collision self 0xA, target 5, modifier by Param3
(0 → 3, 1 → 1, else 0), region 0; Param2 ticks highlighting its panel 4 out of 8; then (unless Param4 is set and the
panel isn't solid: it just goes) the burst, sprite (0x10, 0x26), region 1 for its first tick, sound 0xB9, and with
Param1 an uncracked panel cracked; it goes when the burst's animation ends. One without flag 0x10 goes once the
battle is over.

Lab (scratch, Bass dug from a Giga folder): hit, miss and adjacent match every frame. Unverified: the strike's
Param1/Param3/no-flag-0x10 branches (MachGun's), Bass leaving off the field.

#### 3.6.34 MstrCros (navi chip subtype 22, PA chip 0x15A, T1 0x23)

FireHit3 A + AquaNdl3 A + ElcPuls3 A + RskyHny3 A: MegaMan's five Crosses of the user's game, each
appearing by an enemy and using his Cross's move, then three of them together. Content: chips/mstrcros/navi,
with the chip (chips/mstrcros/chip).

**The spawner, `sub_80BF160`** (`off_802CD5C[22]`): T1 0x23 (`object_spawnType1`, the caller's r1..r3 its position
and r4 its Params), PanelX/Y, element, the damage word, RelatedObject1 = r5, ExtraVars[0] = r7 (a flag byte, set to
1 when r7 isn't 0), r5's side and flip, flags |= 0x10. **The object, `sub_80BE798`**, sprite as
`object_updateSprite`. The chip's Param1 is 0: the leader, which has no sprite. Init (`sub_80BE7BC`) for a Cross
(Param1 = its form, 1..0xA): sprite `sub_800FC9E(0, form)` ((0, `byte_800FCBC[form]`): (0, 0) for the Crosses)
with a ground shadow, CurAnim 0 (loaded 0xFF), on its panel with Z 0, NameID 0x1AB + form (0x1AC..0x1B5: actor
records type 2, AI index 0x18 + form), flip, palette `byte_80203EA[form]` (00 02 07 09 0D 13 05 11 0B 0F 15 by
form 0..0xA), its parts (`sub_8010DF6` by the record: the Cross's form overlays); with Param2 4 it starts at action
8. Both: state update and the first update at once. The leader runs `sub_80BE878`'s phases; a Cross its actions
(`off_80BE854`, 0 to 0x20).

The leader's phases (`off_80BE88C`):
- 0 (`sub_80BE8A0`): Timer and Timer2 0 → 4 (the next tick).
- 4 (`sub_80BE8AE`), the first wave: records `byte_80BE91C` (the user's navi stats byte 0x20, the game, 0: Gregar)
  or `byte_80BE936` (Falzar), 5 bytes each (form, move, look, side, element), at Timer2: 0xFF → phase 8. Else a Cross
  (`sub_80BF160` from his panel: r2 = the element byte, r3 = 0, r4 = form | move << 8 | look << 16 | Timer2 << 24,
  r6 his damage word, r7 = his byte +0x6C + Timer2, its "still acting" flag); its FuturePanel = `sub_80BF192(side
  byte)`; its byte +0x0C = the side byte ^ 1; Timer2 + 1; again at once (the whole wave in one tick).
  - Gregar: (5 ChargeCross, sword, look 0, 1, element 0), (1 HeatCross, sword, 2, 0, 1), (2 ElecCross, sword, 4, 1,
    3), (3 SlashCross, sword, 1, 0, 0), (4 EraseCross, beam, 0, 1, 0).
  - Falzar: (0xA DustCross, sword, 0, 1, 0), (6 SpoutCross, sword, 3, 0, 2), (7 TomahawkCross, sword, 5, 1, 4), (8
    TenguCross, fan, 0, 0, 0), (9 GroundCross, drill, 0, 1, 0).
  - `sub_80BF192(s)`: the panels with an enemy navi's body (`object_getPanelsFiltered`, require `off_80BF224[side]`,
    rows 3..1, columns 6..1); for each, the panel beside it (x + d, with d = `object_getFlipDirection(side, flip ^
    s)`: s 1 in front of it, s 0 behind it) if it is the user's panel or solid with none of 0x0F800000; one of those
    at random (one draw: `GetPositiveSignedRNG2()` mod n); none: (0, …) (no target).
- 8 (`sub_80BE950`): once his flag bytes +0x6C..+0x73 are all 0, 10 ticks → Timer2 0, phase 0xC.
- 0xC (`sub_80BE97A`), the finale: `byte_80BE9D8` (Gregar: ChargeCross, HeatCross, ElecCross) or `byte_80BE9F3`
  (Falzar: DustCross, SpoutCross, TomahawkCross), move 4 (the charge), elements 0, 1, 3 (Falzar 0, 2, 4), spawned
  the same way with FuturePanel = his own panel; at 0xFF → 0x10.
- 0x10 (`sub_80BEA10`): once the flags are clear, 30 ticks: his flag (ExtraVars[0], the controller's) cleared, state
  8 (freed).

A Cross (Param4 = its place in its wave):
- 0 (`sub_80BEA50`), the first wave's entry: phase 0 (`sub_80BEA78`): Timer 30; places 0..2: sound 0x94 and VISIBLE
  (places 3 and 4 stay hidden); Timer2 31; each tick (the entry's too) Timer − 1: at 0 the color shader 0
  (`sprite_zeroColorShader`), phase 4; else Timer2 − 1 (not below 0) and the shader Timer2 · 0x421 (fading from
  white). Phase 4 (`sub_80BEACE`): Timer 0, sound 0x110; each tick Timer + 1 (past 20 → phase 8, that tick too) and
  he circles out from his panel's center: `sub_80E58D2`: X = center + cos(a) · r · front, Y = center + sin(a) · r
  (the sine table, 1.0 = 0x100), r = Timer pixels, a = `byte_80BEB20[place]` (0, 0x55, 0xAA, 0x78, 0xC8) + Timer.
  Phase 8 (`sub_80BEB28`): 30 ticks → 0xC. Phase 0xC (`sub_80BEB46`): anim 4, 4 ticks: VISIBLE off, phase 0x10; no
  target (FuturePanelX 0): his flag cleared, his parts off, state 8. Phase 0x10 (`sub_80BEB90`): Timer = place ·
  20, counting (place 0: one tick) → action 4.
- 4 (`sub_80BEC68`): phase 0 (`sub_80BEC84`): onto FuturePanel, DirectionFlip ^= byte +0x0C (s 0: he faces back),
  the sprite's flip, anim 3, VISIBLE, 3 ticks. Phase 4 (`sub_80BECC6`): anim 0, 5 ticks → action 0xC + move · 4.
- 8 (`sub_80BEBBC`), the finale's entry: phase 0 (`sub_80BEBD8`): anim 3, places 0..2 sound 0x94 and VISIBLE, at the
  end of the circle (r 20, a = `byte_80BEC38[place]` + 20, the same angles), 3 ticks. Phase 4 (`sub_80BEC40`):
  Timer 0 + 1 > 0 on the entry tick → action 0x1C.
- 0xC (`sub_80BECF0`), the sword (move 0): anim 5, sound 0xB0, the blade (`sub_80B8E30`, r4 0x10000 + attachment
  `byte_80BEDC6[look]`: 3, 3, 0x19, 0x1A, 0x1B, 0x1C) in ExtraVars[1], Timer 0. At Timer 9: a hit on the panel ahead
  (`sub_80C53A6`, r4 `byte_80BEDA8[look]` as a word: look 0 region 1, else region 4; no hit spark, target 5, self 4;
  modifier 3; his element and damage word; Z 0), T4#0 effect `byte_80BEDC0[look]` (0x18, then 0x16) at its center
  16 pixels up (his flip; looks 2 and up add look − 1 to its palette), a 10-tick palette flash. Timer + 1, past 21 →
  0x20. (Its table's phase 4, `sub_80BEDCC`, is never entered.)
- 0x10 (`sub_80BEE04`), EraseCross's beam (move 1): anim 0xF; `sub_80D8F98` (attack #0x9D, navis/megaman/forms/erasecross/ray) on
  the panel ahead, r4 = 2 (it shuts when its Param2 is set), flags |= 0x10, in ExtraVars[2]; sound 0xBA; Timer 70; a
  10-tick flash; at Timer 40 `sub_80D8FB8` (its Param2 = 1 if its Param1 is 2); 70 ticks → 0x20.
- 0x14 (`sub_80BEE62`), GroundCross's drill (move 2): phase 0: 10 ticks, then phase 4 at once. Phase 4
  (`sub_80BEEA2`): anim 0xA, attachment 0x20 (r4 0x10120: animation 1) in ExtraVars[1]; `sub_80D2B8E` (attack #0x71,
  DrilArm's drill, chips/drilarm/drill; his element, r4 0x11E00: Param2 0x1E, Param3 1; Z 16 pixels; his damage
  word; held in ExtraVars[2]) with flags |= 0x10; Timer 30; sound 0xF0; a 10-tick flash; Y's and Z's whole parts
  + 1; 30 ticks → 8. Phase 8: the attachment's animation 0 (`sub_80B8E70`), 10 ticks → 0x20.
- 0x18 (`sub_80BEF2C`), TenguCross's fan (move 3): anim 5, sound 0x11F, attachment 0x2A (r4 0x1002A) in
  ExtraVars[1], T4#0 effect 0x44 at his panel's center 16 pixels up (his flip), Timer 21; at Timer 12 a hit on the
  panel ahead (r4 0x0405FF04: region 4; Z 16 pixels; modifier 3) and a 10-tick flash; to −1 (22 ticks) → phase 4: 5
  to −1 (6 ticks) → 0x20.
- 0x1C (`sub_80BEFE2`), the finale's charge (move 4): phase 0 (`sub_80BF000`): Timer 0, the charge glow (T4#0
  effect 0x4E at attach point 0, Timer 70), sound SOUND_BUSTER_CHARGE; at Timer 70 effect 0x4F there (Timer 20),
  sound SOUND_UNK_72; past 90 → 4. Phase 4 (`sub_80BF07A`): anim 0xC, Timer 0; at Timer 4, place 0 only: a hit on
  his panel with the whole-field region of `byte_80BF0F0[side]` (0x82; side 1 0x81: the other side's area; no hit
  spark, target 5, self 4; modifier 3; his element; Z his), the panel bursts on it (`sub_80E2FE8(region, 2, 0, r4
  = 1)`: effect #0x24, objects/panel-bursts; the flags |= 0x10 after it goes through the routine's return value,
  the bursts' panel count, into the BIOS, so the bursts wait out the dimming), a camera shake (3, 30), a palette flash of 35 ticks
  (r4 0x12300), sound 0xC3; past 60 → 8. Phase 8 (`sub_80BF0F8`): 30 to −1 (31 ticks) → 0x20.
- 0x20 (`sub_80BF11A`): anim 4, the attachment (ExtraVars[1]) off, 3 ticks: VISIBLE off, his flag cleared, his
  parts off, state 8.

Lab: the official pa/0x15a-mstrcros/recipe1 ends before (it matches every frame); the long scenarios
pa/0x15a-mstrcros/long{,-miss,-adjacent,-holes} reach the Falzar tables (sword, drill, fan, finale) and every
wave's target, and match every frame; `pa/0x15a-mstrcros/gregar` (the Gregar side of the gregar base uses it)
reaches the Gregar tables, the beam and move 1, and matches every frame. **Unverified**: no target for a Cross, a
leader without the controller's flag, a failed spawn, the sword's dead phase 4.

#### 3.6.35 BigHook (navi chip subtype 23, Giga chip 0x12E, effect T4 0x8C)

BigHook's navi spawner, `off_802CD5C[23]` = `sub_80EA11C`, is instant chip effect 14 (FlmHook's, `off_80EC3F0[14]`)
too: it spawns the hook, effect #0x8C (`sub_80EA010`, chips/flmhook/hook), with the chip's
params as its Params (BigHook's 0x00000A01: Param1 1, Param2 0xA), on the user's panel (also kept in bytes +0x0C and
+0x0D), with its element, side and flip, damage word and RelatedObject1. With Param1 set it keeps r7 in
ExtraVars[1] and stores 1 there: from the navi chip controller that is its flag; from action 0x1C r7 is a ROM
pointer and the write does nothing. Its destroy (`sub_80EA10A`) stores 0 through ExtraVars[1] unconditionally
(clearing the controller's flag; 0 or ROM otherwise). The controller neither warps the user out nor back in for
navi 0x17 (§3.6.7). With Param1 1 the hook's flames (attack #0xCA, chips/flmhook/fire) swing from columns 4
then 5 (side 1: 3 then 2), their kind (Param2) 1 (faster, the other palette and shader, running while dimmed) and
hit spark (Param3) 0xA.

Lab: the official chips/0x12e-bighook scenarios (beast-charged and counter-cut-in among them), the long scenarios
chips/0x12e-bighook/long, long-miss, long-adjacent, and FlmHook's pa/0x146..0x148 reach the hook; every one matches.
**Unverified**: a failed spawn.

#### 3.6.36 Darkness (navi chip subtype 24, PA chip 0x159, T1 0x25)

VDoll F + VDoll F + Bass F (or BassAnly F). Dark MegaMan (Param1 0) raises
a dark flame, then Bass (Param1 1) swoops in and slashes. Content: chips/darkness (navi, chip).

**The spawner, `sub_80BFCD0`** (`off_802CD5C[24]`): T1 0x25, as MstrCros's but the flag pointer in ExtraVars[1].
**The object, `sub_80BF6EC`**, sprite as `object_updateSprite`. Init (`sub_80BF710`), by `byte_80BF7A8[Param1]` (8
bytes as TwinLdrs's): 0: sprite (0, 0), NameID 0x1A0, offset (0, 0); 1: (8, 0x13) decompressed, NameID 0x16D (actor
type 1, AI index 0x13), (0, 0): the NameID, the sprite with a ground shadow, CurAnim 0 (loaded 0xFF), X, Y, Z the
user's (RelatedObject1's), kept at +0x40, the panel from them, Z 0, VISIBLE, flip, parts (`sub_8010DF6` by the
record), state update and the first update at once.

Dark MegaMan (Param1 0), actions `off_80BF7E4`:
- 0 (`sub_80BF800`): phase 0 (`sub_80BF81C`): sound 0xFC, Timer 0, Timer2 0, palette 0x19; each later tick Timer + 1,
  past 1 Timer 0 and Timer2 + 1, past 15 alpha off (`sprite_disableAlpha`) and phase 4; every tick before that the
  sprite's alpha = Timer2 (`sprite_setAlpha_8002c7a`): he fades in over 32 ticks. Phase 4 (`sub_80BF866`): 30 ticks
  → 4.
- 4 (`sub_80BF888`): phase 0 (`sub_80BF8A8`): anim 0xA, Timer 30, his aura (`sub_80B8E30`, r4 0x10032: attachment
  0x32, sprite (0xC, 0x43)) in ExtraVars[0], sound 0x94; 30 ticks → 4. Phase 4 (`sub_80BF8DA`): Timer 0, Timer2 0,
  the aura's animation 2, a camera shake (2, 0x6A); each tick sound 0x12B when Timer2 is 0 (Timer2 cycles 0..15);
  at Timer 0, 8 and 16 the dark flames `sub_80BFD02(0 / 1 / 2, 0x5A)`; Timer + 1, past 0x6A → 8. Phase 8
  (`sub_80BF94E`): Timer 30, the aura's animation 3; 30 ticks → 8.
- 8 (`sub_80BF97C`): anim 4, the aura off, 3 ticks: VISIBLE off → 0xC.
- 0xC (`sub_80BF9B0`): phase 0 (`sub_80BF9CC`): Bass (`sub_80BFCD0` from the user (r5 = RelatedObject1), his panel,
  element 0, r4 = 1, his damage word, r7 = his own FuturePanelX byte, which the spawn sets to 1); then each tick,
  once FuturePanelX is 0 (Bass gone) → phase 4 (`sub_80BF9FE`: 30 ticks → state 8).
- His destroy (`sub_80BFCB2`): his parts off, the byte ExtraVars[1] points at (the controller's flag) cleared, freed.

The dark flames `sub_80BFD02(n, 0x5A)`: element pillars (`sub_80D07A0`: attack #0x61, objects/element-pillar, kind 5;
element 1, Z 6 pixels, r4 = Param1 5, Param2 (2 − n) · 8 + 0x5A, Param3 8, Param4 n; his damage word; flags |=
0x10) on the panels `off_80BFD68[n]` from his (dx toward the front, dy): n 0 (1, 0); n 1 (2, −1), (2, 0), (2, 1);
n 2 (3, −1), (3, 0), (3, 1).

Bass (Param1 1), actions `off_80BF7F4`:
- 0 (`sub_80BFA1C`): phase 0 (`sub_80BFA38`): sound 0x13B, Timer 0, X − 100 · front pixels, Z 100 pixels, X velocity
  10 · front and Z velocity −10 pixels a tick; each tick (the entry's too) he moves; at Timer 5 and 7 afterimages
  (`sub_80BFD90(0)`); Timer + 1, at 10 onto his panel's center (`object_setCoordinatesFromPanels`), alpha off, phase
  4 (`sub_80BFAAE`: one tick) → 4. `sub_80BFD90(a)`: two afterimages (`sub_80E33FA`) at his X, Y − 1 and Z − 1 pixels, color
  shader 0x8108, 15 ticks, blinking, sprite (8, 0x13) with his flip: animation 0x14 + a (r4 0x141308, r7 0xF: the
  shadow at its height), then animation a (r4 0x1308, r7 0x0101000F: a ground shadow).
- 4 (`sub_80BFAD8`): phase 0 (`sub_80BFAF8`): anim 0, the charge glow (effect 0x4E at attach point 0, Timer 30),
  sound SOUND_BUSTER_CHARGE, Timer 40; at 20 effect 0x4F (Timer 10), sound SOUND_UNK_72; 40 ticks → 4. Phase 4
  (`sub_80BFB74`): anim 5, Timer 0, his sword (r4 0x10E03: attachment 3, animation 0xE) in ExtraVars[0], sound 0xCE;
  at Timer 10 a hit on the panel ahead (r4 0x0405FF11: region 0x11, no hit spark, target 5, self 4; modifier 3; his
  element; Z 0), T4#0 effect 0x1B at its center 16 pixels up (his flip, palette + 4), a camera shake (2, 0x28), a
  30-tick flash (r4 0x11E00); past 30 → 8. Phase 8 (`sub_80BFBFA`): 30 ticks → 8.
- 8 (`sub_80BFC28`): phase 0 (`sub_80BFC44`): anim 0, the sword off, Timer 0, Timer2 15; each later tick Timer + 1,
  past 2 Timer 0 and Timer2 − 1, at 0 VISIBLE off, alpha off, phase 4; before that alpha = Timer2 (fading out over
  45 ticks). Phase 4 (`sub_80BFC94`): 30 ticks → state 8, whose destroy clears MegaMan's FuturePanelX.

Lab: the official pa/0x159-darkness recipes end before (they match every frame); the long scenarios
pa/0x159-darkness/long{,-miss} (Bass's recipe) and long-bassanly{,-miss} (BassAnly's) reach every branch but the
failed spawns and a missing flag pointer (**unverified**), and match every frame.

#### 3.6.37 Count (navi chip subtype 18, chips 0x113–0x115, T1 0x11; the Japanese games')

Count H\* + Count[EX] H + Count[SP] H: the Japanese games' names, which the content uses; the US release calls them
HackJack, HackJck[EX] and HackJck[SP]. **The US games have no routine**: their
`off_802CD5C[18]` is null (the game jumps to address 0), T1 0x11 and T3 0x0D point at placeholder routines and
sprite (8, 0x16) is a placeholder archive (the pack has the Japanese ROMs' sprite, `count`). The Japanese games
(EXE6 Falzar BR6J, EXE6 Gregar BR5J) have them; the
two are the same code at different addresses (EXE6 Falzar's below; EXE6 Gregar's +0x1860 for the navi, +0x1860 for
the lance). There is no Japanese disassembly: the addresses are the ROMs', the routines they call the US games'
(the verification workspace's `fmap.py --to` maps them). Content: chips/count (navi, lance, chips).

The records: as the US games' but for flags 0x47 (the US games' 0x07: the library bit) and the sort key; the content
has the Japanese records. Damage 20/25/SP formula 17 (the rain's hits); parameters 0x32/0x46/0x64 (the lances'
damage, 50/70/100).

**The spawner, 0x080BD236** (`off_802CD5C[18]` in the Japanese table at 0x0802D8B8): `object_spawnType1(0x11)` with
r1..r3 the panel Y, element and the spawner's own address (the controller's r3) as its position and r4 the chip's
parameters as Params; panel, element, RelatedObject1 the user, alliance and flip the user's, the damage word (+0x2C)
r6, CollisionDataPtr the controller's flag pointer (r7), which it sets to 1.

**The object, 0x080BCFCC**: `off`-table by state (init, update, `object_freeMemory`), then
`object_updateSpriteTimestop`. Init (0x080BCFF0): `object_setCoordinatesFromPanels`, Z 0 (a word store),
`sprite_decompress(8, 0x16)`, sprite (8, 0x16) with a ground shadow, animation 0, palette 0, flip, state update.
Actions (0x080BD048), each entering on CurPhase 0 (setting it to 4); "n ticks" counts the halfword timer down to 0:
- 0 (0x080BD064): anim 3, sound 0x94, VISIBLE, Timer and Timer2 3 and 0; 3 ticks; panel flags 0x10010 → 4, else 0x18.
- 4 (0x080BD0B8): anim 0; 30 ticks.
- 8 (0x080BD0E0): anim 5, Timer 20 (a halfword store); at 17 left anim 7; at 0 → 0xC.
- 0xC (0x080BD114): the rain: `sub_80C6330` (the dust storm, attack 0x0E: lib/instant/dust_storm) on
  `byte_80BD180[side]` (side 0 (5, 2), side 1 (2, 2): the middle of the other side's area), his element, r4 0x023C00
  (Param2 60 ticks, Param3 2: untied, its hits landing while dimmed: region 0x0F, the whole area, every 12 ticks
  from its first, five hits of his damage word; its own target type is row 0, so it learns nothing of them, but the
  navis react to it), r6 his damage word, r3 0x20000 (its spawn Z, of which its init keeps the fraction); sound
  0x128; Timer 70,
  Timer2 16; each tick Timer2 down, at 0 sound 0x128 and 16 again; 70 ticks → 0x10.
- 0x10 (0x080BD184): the targets (0x080BD2CC), their count in ExtraVars[0xC] (none: → 0x14), ExtraVars[0x10] 0,
  Timer 10. Every 10 ticks a lance (0x080BD394) on the next target (packed x | y << 4 in ExtraVars[0..2]); after the
  last → 0x14.
- 0x14 (0x080BD1D8): anim 8, Timer and Timer2 23 and 0; at 20 left anim 0; at 0 → 0x18.
- 0x18 (0x080BD20C): anim 4, Timer 3; at −1 the controller's flag cleared and state 8 (a word store).

Timeline from his init tick S: visible S+1; standing S+5; arms up S+36; the rain S+57 (61 ticks on the field; its
hits S+58, +70, +82, +94, +106, the damage taken a tick later); lances at S+138, S+148, S+158 (each striking the
tick after, the damage a tick later); gone at S+187 (freed S+188).

**The targets, 0x080BD2CC**: `object_getPanelRegion` (region 0x0F, the 3×3 around the anchor) around
`byte_80BD390[side]` (the same middle), turned by the side (r6 = the alliance), with `dword_80BD368[side]`: side 0
(0x04000030, 0x3F40), side 1 (0x08000010, 0x3F60) (an enemy's body on a solid panel of the other side's area). With
fewer than three, a second call with `dword_80BD37C[side]`: side 0 (0x30, 0x04003F40), side 1 (0x10, 0x08003F60)
(the area's other solid panels), appended. Then `sub_8000C72` (RNG2) shuffles: the second batch alone when there
is one (as many swaps as its panels), else the first (as many swaps as its panels). The first three are the
targets; none (both calls empty) returns 0. (0x080BD25C, the same with `object_getPanelsExceptCurrentFiltered`, has
no caller.) The flags' "solid" is the plain panels' kind: on the grass and ice stages both calls come up empty (no
lance; on to 0x14), and on the poison stage only the enemy's panel is found (one lance, the first batch shuffled).
A netbattle has one enemy navi, and a RockCube carries no enemy's body, so the first call finds at most one panel.

**The lance (0x080BD394 → the spawner 0x080C9614)**: at the target, his element, r4 0x20000 (Param3 2), the damage
word Param1 | (Damage & 0xF000) (the chip's parameter with his damage word's flag bits; hit parameter 0), and r3 =
Damage & 0xF000 too (its spawn Z, whose fraction lasts). The spawner: `object_spawnType3(0x0D)`, panel, element,
damage word, alliance and flip his, flags |= 0x10 (no RelatedObject1).

**The lance, T3 0x0D (0x080C9498)**: by state (init, update, `object_genericDestroy`); then, Param3 2,
`object_updateSpriteTimestop`, else `object_updateSpritePaused`.
- Init (0x080C94C8): `object_setCoordinatesFromPanels`, Z's whole part 0x1000 (4096 pixels up, the fraction kept),
  sprite (8, 0x16) without a shadow, VISIBLE, animation 0xC, palette 0, flip; no collision data: freed. Collision
  self 0x0A (thrown), target 5, modifier 3, hit effect 0, region off, presented; state 4 (a byte store), phase 0,
  action Param3 == 2 ? 4 : 0.
- Update (0x080C9540): `object_removeCollisionData`, `object_spawnCollisionEffect`; battle over: region off, state
  8; a hit (CollisionData+0x70) turns the region off; Param3 ≠ 2 while dimmed: nothing more; else the action
  (0x080C9588), then `object_presentCollisionData`.
- Action 0 (0x080C9590): Timer = Param2, Timer2 0; each tick Timer2 + 1, its panel highlighted while bit 2 is clear,
  Timer − 1, at 0 → 4.
- Action 4 (0x080C95CA): Z's whole part 0, anim 0xC (restarted), region 1, sound 0x181, Timer 30; a hit turns the
  region off; at 0 region off, state 8.

Every lance Count drops has Param3 2: it strikes at once, while dimmed. Action 0 and the Param3 ≠ 2 branches are
the other user's: attack 0x0C (0x080C91E0), which drops lances with them, is spawned only by the Japanese games'
Count navi AI (0x08107908, 0x0810DE6E; out of a netbattle's reach, as every navi AI).

**Verified** on Japanese consoles (EXE6 Falzar and Gregar: the chip lab's jp/chips/0x113-count, 0x114-count-ex and
0x115-count-sp, 43 scenarios): every timing above, each level's rain and lances, the targets (the enemy where it
stands, invisible, killed by the rain; the grass, ice and poison stages' lists), no footing, the guards, AntiNavi
(Count turned on his user), Beat (a Mega chip), the counter and the counter cut-ins both ways, Atk+10 and Navi+20
(the rain takes them, the lances only the flags), Full Synchro (both doubled), side 1 and the KOs. The opponent is
dimmed through all of it, so it can't move during the rain or before the search. **Unverified**: those branches and
a full pool (no navi, no lance, no collision data); the first call's two or more panels can't happen.

Not this chip's: attacks 0x13, 0x14 and 0x15 and effects 0x17 and 0x18, which use sprite (8, 0x16) too, are spawned
only by that navi AI's code (0x0810xxxx).

#### 3.6.38 Django (navi chip subtype 19, chips 0x116–0x118, T1 0x12; the Japanese games')

Django D\* + Django2 D + Django3 D. **The US games have no routine**: `off_802CD5C[19]` is null, T1 0x12 a
placeholder, sprite (0xC, 0xF) a placeholder archive (the pack has the Japanese ROMs' sprite, `django`), and
attachment rows 0xB and 0xC show sprite (0xC, 0) (the Japanese games': (0xC, 0xF), Django's: the US games' CrosOver
shows his gun from the blades' sheet; the content shows the Japanese games' on every console). The Japanese
games' code below (EXE6 Falzar; EXE6 Gregar +0x1860). Content: chips/django (navi, chips).

The records differ: the US games' are class 3 (not a folder chip) with flags 0; the Japanese games' class 1 (Mega)
with flags 0x47 (dimming, damage, navi, library), and their sort keys are the Japanese order. Damage 130/180/260 (the
ride's); parameters 0x32/0x50/0x78 (the slash's damage, 50/80/120). The content has the Japanese records (as for
every JP-content chip; `gen-content check` compares them with a Japanese ROM's): a folder chip that cuts in (a
counter cut-in during the other side's dimming), shows its damage on the telop, takes Atk+ and Navi+, which Beat turns
back as a Mega chip and AntiNavi by the `navi` flag. (The US records would need the `navi_slot` trait for AntiNavi,
which turns back the chip table's block 0xDD..0x118 by number.)

**The spawner, 0x080BD6A2** (`off_802CD5C[19]`): as Count's with `object_spawnType1(0x12)`.

**The object, 0x080BD3B8**: by state (init, update, `object_freeMemory`), then `object_updateSpriteTimestop`.
Init (0x080BD3DC): PanelX 0 (side 0) or 7 (side 1: by the alliance), `object_setCoordinatesFromPanels`, Z's whole part
60 (its fraction the spawner's address's low half, kept throughout), `sprite_decompress(0xC, 0xF)`, sprite (0xC,
0xF) without a shadow, VISIBLE, animation 6, palette 0, flip; Param2 0, ExtraVars[0] and [1] 0, Param3 1; his bike
(`sub_80B8E30`, r4 0x1070C: attachment row 0xC, animation 7, going on while dimmed) in RelatedObject2; state update.
(It returns through action 0's epilogue, `pop {r4, r6, pc}` for its `push {r4, r7, lr}`: r6 and r7 come back wrong,
which the object loop, keeping its own, doesn't mind.)
Actions (0x080BD46C):
- 0 (0x080BD484): the drop onto the column ahead (`object_getEnemyDirection`), his row: `sub_8001330` (10 ticks,
  gravity 0xFFFFA000) gives the velocity, Timer 10; each tick (the entry's too) he moves (Z velocity + gravity),
  `object_setPanelsFromCoordinates`; at 0, a solid panel: a camera shake (1, 20) → 4; else the controller's flag
  cleared, the bike let go (`sub_80B8E58`, RelatedObject2 0), T4#0 effect 0x12 (smoke) 16 pixels above him, freed.
- 4 (0x080BD520): first the command (0x080BD6C8); entering, sound 0x1CA, X velocity 5 pixels a tick toward the enemy,
  Timer 48 (240 pixels); each tick X + velocity, his panel from it: solid and a new column: a hit on it (0x080BD724:
  region 1, no spark, target 5, self 6, modifier 3, his element and damage word, Z 0, while dimmed); not solid but on
  the field: smoke, the ride ends; off the field: on. At 0 the ride ends: the bike let go; Param2 set → 8; else the
  flag cleared, freed.
- The command (0x080BD6C8), while Param2 is 0: with a key in, ExtraVars[1] down, at 0 both cleared; the user's
  pressed keys (AIData+0x24) against `word_80BD71C[ExtraVars[0]]` (L, L, L, A); a match moves ExtraVars[0] on: at 4
  Param2 1 and sound 0x8B, at 1 ExtraVars[1] 60. Only the ride checks it, and the ride is 48 ticks: the 60 never
  run out (a key after the ride goes unread).
- 8 (0x080BD5BA): the target (0x080BD782); none: Param3 0 and his user's panel; anim 1, a ground shadow, sound
  0x94, Timer 3; 3 ticks → 0xC.
- 0xC (0x080BD606): anim 0, Timer 20; at 0 → 0x10, or 0x14 with Param3 0.
- 0x10 (0x080BD63E): anim 5, Timer 30 (a halfword store); at 20 left the slash (0x080BD744: the panel ahead, region
  1, no spark, target 5, self 4, modifier 3, damage Param1 | (Damage & 0xF000), hit parameter 0, while dimmed) and
  sound 0xB0; at 0 → 0x14.
- 0x14 (0x080BD678): anim 2, Timer 3; at −1 the flag cleared, freed.

**The target, 0x080BD782**: from his user's column forward (`object_getEnemyDirection`), columns 0 to 5 ahead, rows
`byte_80BD810` (0, −1, +1, −2, +2) from the user's: the first panel with the enemy's body (`dword_80BD7F8[side]`)
whose panel toward the user is solid with none of 0x03800000; that panel. A column off the field ends the search
with none.

Timeline from his init tick S: landed and riding S+10; the ride's end S+59 (freed then without the command).

**Verified** on Japanese consoles (EXE6 Falzar and Gregar: the chip lab's jp/chips/0x116-django, 0x117-django2 and
0x118-django3, 48 scenarios): the drop, the ride's hits and misses at each level, a crash into a hole mid-ride and on
the landing panel (with and without the command), the command with a target (in his row, a row or two away, the
adjacent column) and without one (invisible), keys out of turn and after the ride, a RockCube in his row (his or the
opponent's), the guards, AntiNavi and Beat (the Japanese record's navi flag and Mega class), the counter, the counter
cut-ins both ways (the record's dimming flag), Atk+10 and Navi+20 (the ride takes them, the slash only the flags),
Full Synchro, side 1 and the KOs. On EXE6 Gregar his spawner's address (his Z's fraction) is 0x080BEF03: his drop's
velocity, so his and his bike's Z, differ from EXE6 Falzar's until he lands (compat's games.toml maps it).
**Unverified**: the command's 60 ticks running out (unreachable: the ride is 48) and a full pool (no bike, no
Django).

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
  (deletion). **The attack is not canceled by the target's death:** regions keep spawning, T4 lives until 1047,
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

Both are the content's weapon definition `megaman/beast-claw` (navis/megaman/weapons/beast-claw: the setup writes
the slash count into its action's state before the action starts, `navi:action_state(beast_claw.action)`); a phase
past the table's two is an error.

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
3. **Stale `av` bytes.** Chip use does not initialize +0x10..+0x13, +0x16..+0x1A or +0x20..+0x4F.
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
   from code only; every trace here has a single turn. The link-transmit stall condition `sub_803EA2C` was not analyzed.
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
