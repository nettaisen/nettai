# BN6 Falzar (BR6E) chip inventory

Generated from the US Falzar ROM (sha1 0676ecd4…). Names come from the chip name text archives
(`TextScriptChipNames0` for ids ≤ 0xFF, `TextScriptChipNames1[id & 0xFF]` otherwise, per `sub_8027D10`).
Do not edit by hand. Field semantics: see `chips.md` §Chip data table.

Record for chip `id` = `0x08021DA8 + 0x2C*id` (`getChip8021DA8`, 0x08021AA4); ids 0..0x19A (411 records).

Columns:

- **codes** (+0x00..+0x03): letters A–Z = 0..25, `*` = 0x1A, 0xFF = empty slot (omitted).
- **elem** (+0x04): attack element, low nibble of the attack's element byte: 0 Null, 1 Fire, 2 Aqua, 3 Elec, 4 Wood.
- **fam** (+0x06): chip icon family: 0 Fire, 1 Aqua, 2 Elec, 3 Wood, 4 Plus, 5 Sword, 6 Cursor, 7 Obj(summon),
  8 Wind, 9 Break, 0xA Null, 0xB/0xC (special). Sword/Cursor/Wind/Break map to secondary-element bits 0x80/0x40/0x20/0x10
  via `byte_80129E4`.
- **cls** (+0x07): 0 Std, 1 Mega, 2 Giga, 3 Spec (not a folder chip), 4 PA (program advance).
- **★** (+0x05): rarity 0..4 (stars − 1). **MB** (+0x08).
- **flags** (+0x09): 0x01 dimming chip, 0x02 has damage (shown, boostable), 0x04 Navi chip (Navi+ applies),
  0x08 standard library, 0x10 variable-damage display, 0x40 library (std/mega), 0x80 damage recomputed each frame.
- **p0A** (+0x0A): hi-half of the attack damage word → attack object +0x2E → CollisionData+0x07.
- **act** (+0x0B): CurAction set by `object_setAttack2`. **handler**: see header of this file.
- **sub** (+0x0C): variant/sub-type → AIAttackVars+0x03. **BO** (+0x0F): Beast-Out auto-lock flag → AIAttackVars+0x1D.
- **p10** (+0x10..+0x13, u32): per-action parameters → AIAttackVars+0x0C (passed as r4 to spawners).
- **lock** (+0x14): post-chip input lockout frames → AIData+0x19 at `object_exitAttackState`.
- **f16** (+0x16): 0x80 no slot-in gauge cost, 0x02 cancelled by Rush support (sub_8010740), 0x01/0x10/0x20/0x40 menu-only.
- **LO** (+0x17): Beast-Out lock-on panel selector (index into `jt_8026584`).
- **dmg** (+0x1A, u16): base damage; values >= 1000 are `var[n]` = formula index n into `off_80109DC` (see below).
  **lib#** (+0x1C, u16) library number. **max** (+0x1E): per-battle slot-in use limit (`sub_802E830`).
- **sub1F** (+0x1F): dark-chip substitution index into `off_8010D84` (0xFF = none, omitted).

| id | dec | name | codes | elem | fam | cls | ★ | MB | flags | p0A | act | handler | sub | BO | p10 | lock | f16 | LO | dmg | lib# | max | sub1F |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 000 | 0 | MegaBstr | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 001 | 1 | Cannon | ABC* | Null | Null | Std | 0 | 6 | 4A | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 1 | 3 |  |
| 002 | 2 | HiCannon | LMN* | Null | Null | Std | 1 | 24 | 4A | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 1 | 1 | 00000000 | 0 | 00 | 1 | 100 | 2 | 3 |  |
| 003 | 3 | M-Cannon | RST* | Null | Null | Std | 2 | 38 | 4A | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 2 | 1 | 00000000 | 0 | 00 | 1 | 180 | 3 | 3 |  |
| 004 | 4 | AirShot | * | Null | Wind | Std | 1 | 4 | 4A | 20 | 21 | `sub_80EC884` (080EC884) | 0 | 1 | 00000004 | 0 | 80 | 1 | 20 | 4 | 3 |  |
| 005 | 5 | Vulcan1 | BDS* | Null | Null | Std | 0 | 5 | 4A | 10 | 17 | `sub_80EBF10` (080EBF10) | 0 | 1 | 0000000C | 0 | 00 | 1 | 10 | 5 | 3 |  |
| 006 | 6 | Vulcan2 | DFL | Null | Null | Std | 1 | 18 | 4A | 10 | 17 | `sub_80EBF10` (080EBF10) | 1 | 1 | 0000000C | 0 | 00 | 1 | 15 | 6 | 3 |  |
| 007 | 7 | Vulcan3 | AGR | Null | Null | Std | 2 | 30 | 4A | 10 | 17 | `sub_80EBF10` (080EBF10) | 2 | 1 | 0000000C | 0 | 00 | 1 | 20 | 7 | 3 |  |
| 008 | 8 | SuprVulc | V | Null | Null | Std | 3 | 75 | 4A | 4 | 17 | `sub_80EBF10` (080EBF10) | 3 | 1 | 00000010 | 0 | 00 | 1 | 20 | 8 | 1 |  |
| 009 | 9 | Spreadr1 | LMN* | Null | Null | Std | 0 | 10 | 4A | 10 | 25 | `sub_80ECBB0` (080ECBB0) | 0 | 1 | 00000003 | 0 | 00 | 1 | 30 | 9 | 3 |  |
| 00A | 10 | Spreadr2 | ABC* | Null | Null | Std | 1 | 18 | 4A | 10 | 25 | `sub_80ECBB0` (080ECBB0) | 0 | 1 | 00000003 | 0 | 00 | 1 | 60 | 10 | 3 |  |
| 00B | 11 | Spreadr3 | QRS* | Null | Null | Std | 2 | 26 | 4A | 10 | 25 | `sub_80ECBB0` (080ECBB0) | 0 | 1 | 00000003 | 0 | 00 | 1 | 90 | 11 | 3 |  |
| 00C | 12 | TankCan1 | AGR | Null | Null | Std | 0 | 17 | 4A | 20 | 24 | `sub_80ECACA` (080ECACA) | 0 | 1 | 00000100 | 0 | 00 | 1 | 120 | 12 | 3 |  |
| 00D | 13 | TankCan2 | LSV | Null | Null | Std | 1 | 28 | 4A | 20 | 24 | `sub_80ECACA` (080ECACA) | 0 | 1 | 00000100 | 0 | 00 | 1 | 160 | 13 | 3 |  |
| 00E | 14 | TankCan3 | BMP | Null | Null | Std | 2 | 39 | 4A | 20 | 24 | `sub_80ECACA` (080ECACA) | 0 | 1 | 00000100 | 0 | 00 | 1 | 200 | 14 | 3 |  |
| 00F | 15 | GunDelS1 | CMT* | Null | Null | Std | 1 | 15 | 48 | 0 | 37 | `sub_80EDAE0` (080EDAE0) | 0 | 1 | 00000000 | 0 | 00 | 9 | 0 | 15 | 3 |  |
| 010 | 16 | GunDelS2 | BER | Null | Null | Std | 2 | 30 | 48 | 0 | 37 | `sub_80EDAE0` (080EDAE0) | 1 | 1 | 00000000 | 0 | 00 | 9 | 0 | 16 | 3 |  |
| 011 | 17 | GunDelS3 | NQW | Null | Null | Std | 3 | 38 | 48 | 0 | 37 | `sub_80EDAE0` (080EDAE0) | 2 | 1 | 00000000 | 0 | 00 | 9 | 0 | 17 | 3 |  |
| 012 | 18 | GunDelEX | G | Null | Null | Spec | 4 | 80 | 00 | 0 | 37 | `sub_80EDAE0` (080EDAE0) | 3 | 1 | 00000000 | 0 | 01 | 16 | 0 | 201 | 1 |  |
| 013 | 19 | YoYo | LMN* | Null | Null | Std | 2 | 32 | 4A | 10 | 18 | `sub_80EC02A` (080EC02A) | 0 | 1 | 02020001 | 0 | 00 | 5 | 50 | 18 | 3 |  |
| 014 | 20 | FireBrn1 | FGH* | Fire | Fire | Std | 0 | 8 | 4A | 30 | 27 | `sub_80ECD28` (080ECD28) | 0 | 1 | 0001021E | 0 | 00 | 4 | 70 | 19 | 3 |  |
| 015 | 21 | FireBrn2 | STU | Fire | Fire | Std | 1 | 21 | 4A | 30 | 27 | `sub_80ECD28` (080ECD28) | 0 | 1 | 0001021E | 0 | 00 | 4 | 110 | 20 | 3 |  |
| 016 | 22 | FireBrn3 | CDE | Fire | Fire | Std | 2 | 34 | 4A | 30 | 27 | `sub_80ECD28` (080ECD28) | 0 | 1 | 0001021E | 0 | 00 | 4 | 150 | 21 | 3 |  |
| 017 | 23 | WideSht | PQR | Aqua | Aqua | Std | 2 | 34 | 4A | 20 | 30 | `sub_80ED55C` (080ED55C) | 0 | 1 | 00000003 | 0 | 00 | 3 | 100 | 22 | 3 |  |
| 018 | 24 | TrnArrw1 | AFK | Aqua | Aqua | Std | 0 | 30 | 4A | 30 | 28 | `sub_80ECDFC` (080ECDFC) | 0 | 1 | 00000000 | 0 | 00 | 10 | 30 | 23 | 3 |  |
| 019 | 25 | TrnArrw2 | GMZ | Aqua | Aqua | Std | 1 | 36 | 4A | 30 | 28 | `sub_80ECDFC` (080ECDFC) | 0 | 1 | 00000000 | 0 | 00 | 10 | 40 | 24 | 3 |  |
| 01A | 26 | TrnArrw3 | MSY | Aqua | Aqua | Std | 2 | 42 | 4A | 30 | 28 | `sub_80ECDFC` (080ECDFC) | 0 | 1 | 00000000 | 0 | 00 | 10 | 50 | 25 | 3 |  |
| 01B | 27 | BblStar1 | BET | Aqua | Aqua | Std | 0 | 30 | 4A | 30 | 2D | `sub_80ED2F8` (080ED2F8) | 0 | 1 | 00000000 | 0 | 00 | 1 | 60 | 26 | 3 |  |
| 01C | 28 | BblStar2 | CLV | Aqua | Aqua | Std | 1 | 38 | 4A | 30 | 2D | `sub_80ED2F8` (080ED2F8) | 0 | 1 | 00000001 | 0 | 00 | 1 | 80 | 27 | 3 |  |
| 01D | 29 | BblStar3 | GRS | Aqua | Aqua | Std | 2 | 46 | 4A | 30 | 2D | `sub_80ED2F8` (080ED2F8) | 0 | 1 | 00000002 | 0 | 00 | 1 | 100 | 28 | 3 |  |
| 01E | 30 | Thunder | BRS* | Elec | Elec | Std | 1 | 7 | 4A | 20 | 1F | `sub_80EC7A6` (080EC7A6) | 0 | 1 | 00100501 | 0 | 00 | 2 | 40 | 29 | 3 |  |
| 01F | 31 | DolThdr1 | AEQ | Elec | Elec | Std | 0 | 24 | 4A | 30 | 3E | `sub_80EE0BC` (080EE0BC) | 0 | 1 | 00000F00 | 0 | 00 | 1 | 120 | 30 | 3 |  |
| 020 | 32 | DolThdr2 | CLP | Elec | Elec | Std | 1 | 31 | 4A | 30 | 3E | `sub_80EE0BC` (080EE0BC) | 0 | 1 | 00000F00 | 0 | 00 | 1 | 150 | 31 | 3 |  |
| 021 | 33 | DolThdr3 | BRV | Elec | Elec | Std | 2 | 38 | 4A | 30 | 3E | `sub_80EE0BC` (080EE0BC) | 0 | 1 | 00000F00 | 0 | 00 | 1 | 180 | 32 | 3 |  |
| 022 | 34 | ElcPuls1 | JLS | Elec | Elec | Std | 0 | 32 | 4A | 30 | 42 | `sub_80EE55E` (080EE55E) | 0 | 1 | 003C1001 | 0 | 00 | 17 | 100 | 33 | 3 |  |
| 023 | 35 | ElcPuls2 | AEJ | Elec | Elec | Std | 1 | 36 | 4A | 30 | 42 | `sub_80EE55E` (080EE55E) | 0 | 1 | 013C0051 | 0 | 00 | 17 | 120 | 34 | 3 |  |
| 024 | 36 | ElcPuls3 | AJS | Elec | Elec | Std | 2 | 40 | 4A | 30 | 42 | `sub_80EE55E` (080EE55E) | 0 | 1 | 023C0003 | 0 | 00 | 17 | 140 | 35 | 3 |  |
| 025 | 37 | RskyHny1 | BGS | Wood | Wood | Std | 0 | 21 | 4A | 30 | 39 | `sub_80EDD80` (080EDD80) | 0 | 1 | 00000000 | 0 | 00 | 1 | 10 | 39 | 3 |  |
| 026 | 38 | RskyHny2 | CRV | Wood | Wood | Std | 1 | 28 | 4A | 30 | 39 | `sub_80EDD80` (080EDD80) | 0 | 1 | 00000001 | 0 | 00 | 1 | 15 | 40 | 3 |  |
| 027 | 39 | RskyHny3 | ADM | Wood | Wood | Std | 2 | 35 | 4A | 30 | 39 | `sub_80EDD80` (080EDD80) | 0 | 1 | 00000002 | 0 | 00 | 1 | 20 | 41 | 3 |  |
| 028 | 40 | RlngLog1 | IKP | Wood | Wood | Std | 0 | 14 | 4A | 30 | 36 | `sub_80ED9AE` (080ED9AE) | 0 | 1 | 01143C01 | 0 | 00 | 9 | 50 | 42 | 3 |  |
| 029 | 41 | RlngLog2 | EQZ | Wood | Wood | Std | 1 | 26 | 4A | 30 | 36 | `sub_80ED9AE` (080ED9AE) | 0 | 1 | 01143C01 | 0 | 00 | 9 | 70 | 43 | 3 |  |
| 02A | 42 | RlngLog3 | FNW | Wood | Wood | Std | 2 | 38 | 4A | 30 | 36 | `sub_80ED9AE` (080ED9AE) | 0 | 1 | 01143C01 | 0 | 00 | 9 | 90 | 44 | 3 |  |
| 02B | 43 | MachGun1 | ART* | Null | Cursor | Std | 0 | 12 | 4A | 30 | 29 | `sub_80ECF2E` (080ECF2E) | 0 | 0 | 00010600 | 0 | 00 | 0 | 30 | 55 | 3 |  |
| 02C | 44 | MachGun2 | EGS | Null | Cursor | Std | 1 | 24 | 4A | 30 | 29 | `sub_80ECF2E` (080ECF2E) | 0 | 0 | 00010600 | 0 | 00 | 0 | 50 | 56 | 3 |  |
| 02D | 45 | MachGun3 | BFM | Null | Cursor | Std | 2 | 36 | 4A | 30 | 29 | `sub_80ECF2E` (080ECF2E) | 0 | 0 | 00010600 | 0 | 00 | 0 | 70 | 57 | 3 |  |
| 02E | 46 | HeatDrgn | GRT | Fire | Fire | Std | 2 | 40 | 4A | 30 | 51 | `sub_80EF4B4` (080EF4B4) | 0 | 0 | 00000000 | 0 | 00 | 1 | 140 | 123 | 3 |  |
| 02F | 47 | ElecDrgn | ALV | Elec | Elec | Std | 2 | 40 | 4A | 30 | 51 | `sub_80EF4B4` (080EF4B4) | 1 | 0 | 00000000 | 0 | 00 | 1 | 150 | 124 | 3 |  |
| 030 | 48 | AquaDrgn | HPS | Aqua | Aqua | Std | 3 | 44 | 4A | 30 | 51 | `sub_80EF4B4` (080EF4B4) | 2 | 0 | 00000000 | 0 | 00 | 1 | 120 | 125 | 3 |  |
| 031 | 49 | WoodDrgn | GTV | Wood | Wood | Std | 3 | 48 | 4A | 30 | 51 | `sub_80EF4B4` (080EF4B4) | 3 | 0 | 00000000 | 0 | 00 | 1 | 130 | 126 | 3 |  |
| 032 | 50 | AirHocky | LMN | Null | Break | Std | 2 | 19 | 4A | 5 | 26 | `sub_80ECCB0` (080ECCB0) | 0 | 1 | 00000102 | 50 | 00 | 2 | 60 | 51 | 3 |  |
| 033 | 51 | DrilArm | GMW | Null | Break | Std | 2 | 32 | 42 | 10 | 2E | `sub_80ED374` (080ED374) | 0 | 1 | 00001800 | 0 | 00 | 2 | 70 | 52 | 3 |  |
| 034 | 52 | Tornado | LRT | Null | Wind | Std | 2 | 16 | 4A | 4 | 2F | `sub_80ED454` (080ED454) | 1 | 1 | 00200001 | 0 | 00 | 4 | 20 | 53 | 3 |  |
| 035 | 53 | Static | GSV | Null | Wind | Std | 2 | 30 | 4A | 4 | 2F | `sub_80ED454` (080ED454) | 2 | 1 | 00201002 | 0 | 00 | 4 | 20 | 54 | 3 |  |
| 036 | 54 | MiniBomb | BLR* | Null | Null | Std | 0 | 6 | 4A | 30 | 12 | `sub_80EB628` (080EB628) | 0 | 1 | 00000000 | 0 | 00 | 5 | 50 | 58 | 3 |  |
| 037 | 55 | EnergBom | CKV* | Null | Null | Std | 1 | 11 | 4A | 10 | 12 | `sub_80EB628` (080EB628) | 1 | 1 | 00000002 | 0 | 00 | 5 | 40 | 60 | 3 |  |
| 038 | 56 | MegEnBom | GMO* | Null | Null | Std | 2 | 27 | 4A | 10 | 12 | `sub_80EB628` (080EB628) | 1 | 1 | 00000002 | 0 | 00 | 5 | 60 | 61 | 3 |  |
| 039 | 57 | FlshBom1 | JLQ* | Null | Null | Std | 0 | 30 | 4A | 30 | 12 | `sub_80EB628` (080EB628) | 14 | 1 | 00000000 | 0 | 00 | 0 | 40 | 62 | 3 |  |
| 03A | 58 | FlshBom2 | GKR | Null | Null | Std | 1 | 34 | 4A | 30 | 12 | `sub_80EB628` (080EB628) | 14 | 1 | 00000001 | 0 | 00 | 0 | 70 | 63 | 3 |  |
| 03B | 59 | FlshBom3 | HPS | Null | Null | Std | 2 | 38 | 4A | 30 | 12 | `sub_80EB628` (080EB628) | 14 | 1 | 00000002 | 0 | 00 | 0 | 100 | 64 | 3 |  |
| 03C | 60 | BlkBomb | BFO | Fire | Fire | Std | 3 | 32 | 4A | 18 | 12 | `sub_80EB628` (080EB628) | 6 | 0 | 00000000 | 0 | 00 | 0 | 250 | 65 | 3 |  |
| 03D | 61 | AquaNdl1 | CJP | Aqua | Aqua | Std | 0 | 31 | 4A | 30 | 32 | `sub_80ED6E6` (080ED6E6) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 112 | 3 |  |
| 03E | 62 | AquaNdl2 | FKT | Aqua | Aqua | Std | 1 | 35 | 4A | 30 | 32 | `sub_80ED6E6` (080ED6E6) | 0 | 0 | 00000001 | 0 | 00 | 0 | 60 | 113 | 3 |  |
| 03F | 63 | AquaNdl3 | ALU | Aqua | Aqua | Std | 2 | 39 | 4A | 30 | 32 | `sub_80ED6E6` (080ED6E6) | 0 | 0 | 00000002 | 0 | 00 | 0 | 80 | 114 | 3 |  |
| 040 | 64 | CornSht1 | JKL | Wood | Wood | Std | 0 | 14 | 4A | 30 | 2A | `sub_80ED090` (080ED090) | 0 | 1 | 00000000 | 0 | 00 | 1 | 50 | 36 | 3 |  |
| 041 | 65 | CornSht2 | CDE | Wood | Wood | Std | 1 | 26 | 4A | 30 | 2A | `sub_80ED090` (080ED090) | 0 | 1 | 00000000 | 0 | 00 | 1 | 60 | 37 | 3 |  |
| 042 | 66 | CornSht3 | PQR | Wood | Wood | Std | 2 | 38 | 4A | 30 | 2A | `sub_80ED090` (080ED090) | 0 | 1 | 00000000 | 0 | 00 | 1 | 70 | 38 | 3 |  |
| 043 | 67 | BugBomb | GSV | Null | Null | Std | 2 | 24 | 48 | 0 | 12 | `sub_80EB628` (080EB628) | 7 | 0 | 00000000 | 0 | 00 | 0 | 0 | 66 | 3 |  |
| 044 | 68 | GrasSeed | AFS* | Wood | Wood | Std | 1 | 19 | 48 | 10 | 12 | `sub_80EB628` (080EB628) | 13 | 0 | 00000202 | 10 | 00 | 0 | 10 | 67 | 3 |  |
| 045 | 69 | IceSeed | ALR* | Aqua | Aqua | Std | 1 | 31 | 48 | 10 | 12 | `sub_80EB628` (080EB628) | 9 | 0 | 00000101 | 10 | 00 | 0 | 10 | 68 | 3 |  |
| 046 | 70 | PoisSeed | HNP* | Null | Null | Std | 2 | 37 | 48 | 10 | 12 | `sub_80EB628` (080EB628) | 12 | 0 | 00000300 | 10 | 00 | 0 | 10 | 69 | 3 |  |
| 047 | 71 | Sword | HLS* | Null | Sword | Std | 0 | 8 | 4A | 30 | 13 | `sub_80EB776` (080EB776) | 0 | 1 | 00000000 | 0 | 00 | 2 | 80 | 70 | 3 |  |
| 048 | 72 | WideSwrd | HLS* | Null | Sword | Std | 1 | 12 | 4A | 30 | 13 | `sub_80EB776` (080EB776) | 1 | 1 | 00000000 | 0 | 00 | 3 | 80 | 71 | 3 |  |
| 049 | 73 | LongSwrd | HLS* | Null | Sword | Std | 2 | 25 | 4A | 30 | 13 | `sub_80EB776` (080EB776) | 2 | 1 | 00000000 | 0 | 00 | 4 | 100 | 72 | 3 |  |
| 04A | 74 | WideBlde | BRW | Null | Sword | Std | 3 | 38 | 4A | 30 | 13 | `sub_80EB776` (080EB776) | 3 | 1 | 00000000 | 0 | 00 | 3 | 150 | 73 | 3 |  |
| 04B | 75 | LongBlde | BMV | Null | Sword | Std | 3 | 38 | 4A | 30 | 13 | `sub_80EB776` (080EB776) | 4 | 1 | 00000000 | 0 | 00 | 4 | 150 | 74 | 3 |  |
| 04C | 76 | FireSwrd | FOZ | Fire | Fire | Std | 1 | 30 | 4A | 30 | 13 | `sub_80EB776` (080EB776) | 12 | 1 | 00000000 | 0 | 00 | 3 | 140 | 75 | 3 |  |
| 04D | 77 | AquaSwrd | AIY | Aqua | Aqua | Std | 2 | 32 | 4A | 30 | 13 | `sub_80EB776` (080EB776) | 13 | 1 | 00000000 | 0 | 00 | 3 | 160 | 76 | 3 |  |
| 04E | 78 | ElecSwrd | EKN | Elec | Elec | Std | 2 | 35 | 4A | 30 | 13 | `sub_80EB776` (080EB776) | 14 | 1 | 00000000 | 0 | 00 | 3 | 120 | 77 | 3 |  |
| 04F | 79 | BambSwrd | HSW | Wood | Wood | Std | 2 | 34 | 4A | 30 | 13 | `sub_80EB776` (080EB776) | 15 | 1 | 00000000 | 0 | 00 | 3 | 150 | 78 | 3 |  |
| 050 | 80 | WindRack | FJR* | Null | Wind | Std | 2 | 19 | 4A | 30 | 3F | `sub_80EE192` (080EE192) | 0 | 1 | 00000000 | 0 | 00 | 3 | 140 | 79 | 3 |  |
| 051 | 81 | StepSwrd | BLP | Null | Sword | Std | 2 | 28 | 4A | 20 | 13 | `sub_80EB776` (080EB776) | 1 | 1 | 00000001 | 0 | 00 | 16 | 160 | 80 | 3 |  |
| 052 | 82 | VarSwrd | KVW | Null | Sword | Std | 2 | 28 | 4A | 30 | 53 | `sub_80EF62E` (080EF62E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 160 | 81 | 3 |  |
| 053 | 83 | NeoVari | N | Null | Sword | Std | 3 | 52 | 4A | 30 | 54 | `sub_80EF7E2` (080EF7E2) | 0 | 0 | 00000000 | 0 | 00 | 0 | 220 | 82 | 1 |  |
| 054 | 84 | MoonBld | AMT | Null | Sword | Std | 2 | 35 | 4A | 30 | 40 | `sub_80EE2A0` (080EE2A0) | 0 | 1 | 00000000 | 0 | 00 | 18 | 130 | 83 | 3 |  |
| 055 | 85 | Muramasa | M | Null | Sword | Std | 3 | 77 | DA | 50 | 13 | `sub_80EB776` (080EB776) | 8 | 1 | 00000000 | 0 | 00 | 4 | 1020=var[20] | 84 | 1 |  |
| 056 | 86 | MchnSwrd | HLQ | Null | Null | Std | 0 | 37 | 4A | 30 | 49 | `sub_80EEB4C` (080EEB4C) | 0 | 0 | 00000000 | 0 | 00 | 0 | 200 | 85 | 3 |  |
| 057 | 87 | ElemSwrd | JMU | Null | Null | Std | 1 | 43 | 4A | 30 | 49 | `sub_80EEB4C` (080EEB4C) | 1 | 0 | 00000000 | 0 | 00 | 0 | 220 | 86 | 3 |  |
| 058 | 88 | AssnSwrd | NRY | Null | Null | Std | 2 | 50 | 4A | 30 | 49 | `sub_80EEB4C` (080EEB4C) | 2 | 0 | 00000000 | 0 | 00 | 0 | 240 | 87 | 3 |  |
| 059 | 89 | CrakShot | AGT* | Null | Null | Std | 0 | 4 | 4A | 30 | 22 | `sub_80EC960` (080EC960) | 0 | 1 | 00000000 | 0 | 00 | 7 | 60 | 88 | 3 |  |
| 05A | 90 | DublShot | CRU* | Null | Null | Std | 1 | 8 | 4A | 30 | 22 | `sub_80EC960` (080EC960) | 1 | 1 | 00000000 | 0 | 00 | 8 | 60 | 89 | 3 |  |
| 05B | 91 | TrplShot | JLV* | Null | Null | Std | 2 | 12 | 4A | 30 | 22 | `sub_80EC960` (080EC960) | 2 | 1 | 00000000 | 0 | 00 | 9 | 100 | 90 | 3 |  |
| 05C | 92 | WaveArm1 | EFG | Null | Null | Std | 0 | 15 | 4A | 30 | 31 | `sub_80ED64C` (080ED64C) | 0 | 0 | 0000000C | 0 | 00 | 0 | 80 | 94 | 3 |  |
| 05D | 93 | WaveArm2 | LMN | Null | Null | Std | 1 | 22 | 4A | 30 | 31 | `sub_80ED64C` (080ED64C) | 0 | 0 | 0000000D | 0 | 00 | 0 | 120 | 95 | 3 |  |
| 05E | 94 | WaveArm3 | RST | Null | Null | Std | 2 | 29 | 4A | 30 | 31 | `sub_80ED64C` (080ED64C) | 0 | 0 | 0000000E | 0 | 00 | 0 | 160 | 96 | 3 |  |
| 05F | 95 | AuraHed1 | BCD | Null | Break | Std | 0 | 25 | 4A | 30 | 43 | `sub_80EE634` (080EE634) | 0 | 1 | 00010000 | 0 | 00 | 5 | 130 | 48 | 3 |  |
| 060 | 96 | AuraHed2 | DEF | Null | Break | Std | 1 | 33 | 4A | 30 | 43 | `sub_80EE634` (080EE634) | 0 | 1 | 00010001 | 0 | 00 | 5 | 150 | 49 | 3 |  |
| 061 | 97 | AuraHed3 | FGH | Null | Break | Std | 2 | 39 | 4A | 30 | 43 | `sub_80EE634` (080EE634) | 0 | 1 | 00010002 | 0 | 00 | 5 | 170 | 50 | 3 |  |
| 062 | 98 | LilBolr1 | FKL | Null | Obj | Std | 0 | 18 | 4A | 30 | 12 | `sub_80EB628` (080EB628) | 3 | 0 | 00000000 | 0 | 00 | 0 | 100 | 139 | 3 |  |
| 063 | 99 | LilBolr2 | EMV | Null | Obj | Std | 1 | 23 | 4A | 30 | 12 | `sub_80EB628` (080EB628) | 3 | 0 | 00000002 | 0 | 00 | 0 | 140 | 140 | 3 |  |
| 064 | 100 | LilBolr3 | GSZ | Null | Obj | Std | 2 | 28 | 4A | 30 | 12 | `sub_80EB628` (080EB628) | 3 | 0 | 00000003 | 0 | 00 | 0 | 180 | 141 | 3 |  |
| 065 | 101 | SandWrm1 | AGL | Null | Null | Std | 0 | 30 | 4A | 30 | 1C | `sub_80EC39C` (080EC39C) | 12 | 0 | 00000000 | 0 | 00 | 1 | 130 | 97 | 3 |  |
| 066 | 102 | SandWrm2 | BRY | Null | Null | Std | 1 | 34 | 4A | 30 | 1C | `sub_80EC39C` (080EC39C) | 12 | 0 | 00000001 | 0 | 00 | 1 | 150 | 98 | 3 |  |
| 067 | 103 | SandWrm3 | HJS | Null | Null | Std | 2 | 38 | 4A | 30 | 1C | `sub_80EC39C` (080EC39C) | 12 | 0 | 00000002 | 0 | 00 | 1 | 170 | 99 | 3 |  |
| 068 | 104 | AirRaid1 | GKR | Null | Obj | Std | 0 | 26 | 4B | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[13] = `sub_80E5F78` (080E5F78) | 13 | 0 | 00000000 | 0 | 00 | 0 | 10 | 142 | 3 |  |
| 069 | 105 | AirRaid2 | OTY | Null | Obj | Std | 1 | 32 | 4B | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[13] = `sub_80E5F78` (080E5F78) | 13 | 0 | 00000001 | 0 | 00 | 0 | 10 | 143 | 3 |  |
| 06A | 106 | AirRaid3 | NUZ | Null | Obj | Std | 2 | 39 | 4B | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[13] = `sub_80E5F78` (080E5F78) | 13 | 0 | 00000002 | 0 | 00 | 0 | 10 | 144 | 3 |  |
| 06B | 107 | FireHit1 | DEF | Fire | Fire | Std | 0 | 12 | 42 | 20 | 1C | `sub_80EC39C` (080EC39C) | 8 | 1 | 00000300 | 20 | 00 | 5 | 60 | 105 | 3 |  |
| 06C | 108 | FireHit2 | RST | Fire | Fire | Std | 1 | 22 | 42 | 20 | 1C | `sub_80EC39C` (080EC39C) | 8 | 1 | 00000300 | 20 | 00 | 5 | 120 | 106 | 3 |  |
| 06D | 109 | FireHit3 | ABC | Fire | Fire | Std | 2 | 32 | 42 | 20 | 1C | `sub_80EC39C` (080EC39C) | 8 | 1 | 00000300 | 20 | 00 | 5 | 180 | 107 | 3 |  |
| 06E | 110 | BurnSqr1 | HPV | Fire | Fire | Std | 0 | 24 | 4B | 30 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[23] = `sub_80E70A6` (080E70A6) | 23 | 0 | 00000000 | 0 | 00 | 0 | 100 | 108 | 3 |  |
| 06F | 111 | BurnSqr2 | DMT | Fire | Fire | Std | 1 | 30 | 4B | 30 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[23] = `sub_80E70A6` (080E70A6) | 23 | 0 | 00000000 | 0 | 00 | 0 | 120 | 109 | 3 |  |
| 070 | 112 | BurnSqr3 | EOZ | Fire | Fire | Std | 2 | 36 | 4B | 30 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[23] = `sub_80E70A6` (080E70A6) | 23 | 0 | 00000000 | 0 | 00 | 0 | 140 | 110 | 3 |  |
| 071 | 113 | Sensor1 | JOW | Elec | Elec | Std | 0 | 32 | 4B | 30 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[28] = `sub_80E7CA8` (080E7CA8) | 28 | 0 | 00001400 | 0 | 00 | 0 | 100 | 116 | 3 |  |
| 072 | 114 | Sensor2 | NUY | Elec | Elec | Std | 1 | 35 | 4B | 30 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[28] = `sub_80E7CA8` (080E7CA8) | 28 | 0 | 00001E03 | 0 | 00 | 0 | 130 | 117 | 3 |  |
| 073 | 115 | Sensor3 | IKQ | Elec | Elec | Std | 2 | 38 | 4B | 30 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[28] = `sub_80E7CA8` (080E7CA8) | 28 | 0 | 00002806 | 0 | 00 | 0 | 160 | 118 | 3 |  |
| 074 | 116 | Boomer | JKT | Wood | Wood | Std | 1 | 16 | 4A | 10 | 1C | `sub_80EC39C` (080EC39C) | 1 | 0 | 00000000 | 20 | 00 | 0 | 100 | 119 | 3 |  |
| 075 | 117 | HiBoomer | BLV | Wood | Wood | Std | 2 | 26 | 4A | 10 | 1C | `sub_80EC39C` (080EC39C) | 1 | 0 | 00000001 | 20 | 00 | 0 | 140 | 120 | 3 |  |
| 076 | 118 | M-Boomer | IMW | Wood | Wood | Std | 3 | 36 | 4A | 10 | 1C | `sub_80EC39C` (080EC39C) | 1 | 0 | 00000002 | 20 | 00 | 0 | 170 | 121 | 3 |  |
| 077 | 119 | Lance | ARW* | Wood | Wood | Std | 3 | 42 | 4A | 30 | 1C | `sub_80EC39C` (080EC39C) | 4 | 0 | 00000000 | 10 | 00 | 0 | 150 | 122 | 3 |  |
| 078 | 120 | GolmHit1 | IKY | Null | Break | Std | 0 | 17 | 4A | 30 | 1C | `sub_80EC39C` (080EC39C) | 21 | 0 | 00000000 | 20 | 00 | 0 | 140 | 127 | 3 |  |
| 079 | 121 | GolmHit2 | DPU | Null | Break | Std | 1 | 27 | 4A | 30 | 1C | `sub_80EC39C` (080EC39C) | 21 | 0 | 00000001 | 20 | 00 | 0 | 190 | 128 | 3 |  |
| 07A | 122 | GolmHit3 | HMV | Null | Break | Std | 2 | 37 | 4A | 30 | 1C | `sub_80EC39C` (080EC39C) | 21 | 0 | 00000002 | 20 | 00 | 0 | 250 | 129 | 3 |  |
| 07B | 123 | IronShl1 | JKL | Null | Break | Std | 0 | 13 | 4A | 30 | 2C | `sub_80ED25C` (080ED25C) | 0 | 1 | 00000100 | 0 | 00 | 1 | 70 | 45 | 3 |  |
| 07C | 124 | IronShl2 | CDE | Null | Break | Std | 1 | 20 | 4A | 30 | 2C | `sub_80ED25C` (080ED25C) | 0 | 1 | 00000101 | 0 | 00 | 1 | 100 | 46 | 3 |  |
| 07D | 125 | IronShl3 | LMN | Null | Break | Std | 2 | 27 | 4A | 30 | 2C | `sub_80ED25C` (080ED25C) | 0 | 1 | 00000102 | 0 | 00 | 1 | 130 | 47 | 3 |  |
| 07E | 126 | AirSpin1 | FGR | Null | Wind | Std | 1 | 22 | 4A | 10 | 38 | `sub_80EDCC0` (080EDCC0) | 0 | 0 | 00000200 | 0 | 00 | 0 | 50 | 131 | 3 |  |
| 07F | 127 | AirSpin2 | ALT | Null | Wind | Std | 2 | 29 | 4A | 10 | 38 | `sub_80EDCC0` (080EDCC0) | 0 | 0 | 00000301 | 0 | 00 | 0 | 50 | 132 | 3 |  |
| 080 | 128 | AirSpin3 | NOT | Null | Wind | Std | 3 | 36 | 4A | 10 | 38 | `sub_80EDCC0` (080EDCC0) | 0 | 0 | 00000402 | 0 | 00 | 0 | 50 | 133 | 3 |  |
| 081 | 129 | Wind | * | Null | Wind | Std | 1 | 10 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[8] = `sub_80E3128` (080E3128) | 8 | 0 | 00000000 | 0 | 80 | 0 | 0 | 134 | 3 |  |
| 082 | 130 | Fan | * | Null | Wind | Std | 1 | 10 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[8] = `sub_80E3128` (080E3128) | 8 | 0 | 00000001 | 0 | 80 | 0 | 0 | 135 | 3 |  |
| 083 | 131 | Rflectr1 | ACP* | Null | Null | Std | 0 | 7 | 4A | 20 | 2B | `sub_80ED13E` (080ED13E) | 0 | 0 | 0000003C | 0 | 00 | 0 | 60 | 91 | 3 |  |
| 084 | 132 | Rflectr2 | BGY* | Null | Null | Std | 1 | 16 | 4A | 20 | 2B | `sub_80ED13E` (080ED13E) | 1 | 0 | 0000073C | 0 | 00 | 0 | 120 | 92 | 3 |  |
| 085 | 133 | Rflectr3 | EFO* | Null | Null | Std | 2 | 25 | 4A | 20 | 2B | `sub_80ED13E` (080ED13E) | 2 | 0 | 0000083C | 0 | 00 | 0 | 200 | 93 | 3 |  |
| 086 | 134 | Snake | HML | Null | Null | Std | 3 | 34 | 4B | 133 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[12] = `sub_80E59C6` (080E59C6) | 12 | 0 | 00000000 | 0 | 00 | 0 | 30 | 103 | 3 |  |
| 087 | 135 | SumnBlk1 | EIP | Null | Null | Std | 0 | 30 | 4B | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[36] = `sub_80E91B8` (080E91B8) | 36 | 0 | 00000000 | 0 | 00 | 1 | 160 | 100 | 3 |  |
| 088 | 136 | SumnBlk2 | HOV | Null | Null | Std | 1 | 40 | 4B | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[36] = `sub_80E91B8` (080E91B8) | 36 | 0 | 00000001 | 0 | 00 | 1 | 200 | 101 | 3 |  |
| 089 | 137 | SumnBlk3 | WYZ | Null | Null | Std | 2 | 46 | 4B | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[36] = `sub_80E91B8` (080E91B8) | 36 | 0 | 00000002 | 0 | 00 | 1 | 260 | 102 | 3 |  |
| 08A | 138 | NumbrBl | N | Null | Null | Std | 4 | 69 | DB | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[22] = `sub_80E7FBA` (080E7FBA) | 22 | 0 | 00000004 | 0 | 00 | 0 | 1021=var[21] | 104 | 1 |  |
| 08B | 139 | Meteors | R | Fire | Fire | Std | 4 | 73 | 4B | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[16] = `sub_80E4288` (080E4288) | 16 | 0 | 00000000 | 0 | 00 | 0 | 40 | 111 | 1 |  |
| 08C | 140 | JustcOne | J | Null | Break | Std | 4 | 90 | 4A | 178 | 1C | `sub_80EC39C` (080EC39C) | 19 | 0 | 00000008 | 20 | 00 | 0 | 220 | 130 | 1 |  |
| 08D | 141 | Magnum | FLW | Null | Cursor | Std | 2 | 31 | 4B | 148 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[24] = `sub_80E723E` (080E723E) | 24 | 0 | 00000000 | 0 | 00 | 0 | 130 | 136 | 3 |  |
| 08E | 142 | CircGun | PTV | Null | Cursor | Std | 2 | 35 | 4B | 148 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[19] = `sub_80E7600` (080E7600) | 19 | 0 | 00000402 | 0 | 00 | 0 | 150 | 137 | 3 |  |
| 08F | 143 | RockCube | * | Null | Obj | Std | 0 | 6 | 49 | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[6] = `sub_80E46B6` (080E46B6) | 6 | 0 | 00000001 | 0 | 80 | 0 | 200 | 138 | 3 |  |
| 090 | 144 | TimeBom1 | FGH | Null | Obj | Std | 1 | 20 | 4B | 148 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[10] = `sub_80E3242` (080E3242) | 10 | 0 | 00000000 | 30 | 00 | 0 | 150 | 145 | 3 |  |
| 091 | 145 | Mine | AST | Null | Obj | Std | 2 | 28 | 4B | 178 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[11] = `sub_80E349E` (080E349E) | 11 | 0 | 00000000 | 0 | 00 | 0 | 200 | 148 | 3 |  |
| 092 | 146 | Fanfare | PSZ* | Null | Obj | Std | 1 | 20 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[9] = `sub_80E5EA8` (080E5EA8) | 9 | 0 | 00000000 | 0 | 00 | 0 | 0 | 149 | 3 |  |
| 093 | 147 | Discord | AGS* | Null | Obj | Std | 1 | 20 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[9] = `sub_80E5EA8` (080E5EA8) | 9 | 0 | 00000001 | 0 | 00 | 0 | 0 | 150 | 3 |  |
| 094 | 148 | Timpani | IOT* | Null | Obj | Std | 1 | 20 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[9] = `sub_80E5EA8` (080E5EA8) | 9 | 0 | 00000002 | 0 | 00 | 0 | 0 | 151 | 3 |  |
| 095 | 149 | Silence | BRW* | Null | Obj | Std | 1 | 20 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[9] = `sub_80E5EA8` (080E5EA8) | 9 | 0 | 00000003 | 0 | 00 | 0 | 0 | 152 | 3 |  |
| 096 | 150 | VDoll | FNS* | Null | Obj | Std | 3 | 39 | 48 | 10 | 12 | `sub_80EB628` (080EB628) | 8 | 0 | 00000000 | 0 | 00 | 0 | 10 | 153 | 3 |  |
| 097 | 151 | Guardian | O | Null | Obj | Std | 4 | 64 | 4B | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[14] = `sub_80E67E6` (080E67E6) | 14 | 0 | 00000000 | 0 | 00 | 0 | 200 | 154 | 1 |  |
| 098 | 152 | Anubis | P | Null | Obj | Std | 4 | 86 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[17] = `sub_80E4164` (080E4164) | 17 | 0 | 00000000 | 0 | 00 | 0 | 1 | 155 | 1 |  |
| 099 | 153 | Otenko | O | Null | Obj | Std | 4 | 66 | 01 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[18] = `sub_80E76D4` (080E76D4) | 18 | 0 | 00000000 | 0 | 01 | 0 | 1 | 202 | 1 |  |
| 09A | 154 | Recov10 | ADL* | Null | Null | Std | 0 | 4 | 48 | 30 | 20 | `sub_80EC844` (080EC844) | 0 | 0 | 00000000 | 30 | 00 | 0 | 0 | 156 | 1 |  |
| 09B | 155 | Recov30 | ELQ* | Null | Null | Std | 0 | 12 | 48 | 30 | 20 | `sub_80EC844` (080EC844) | 1 | 0 | 00000000 | 30 | 00 | 0 | 0 | 157 | 1 |  |
| 09C | 156 | Recov50 | CMP* | Null | Null | Std | 0 | 18 | 48 | 40 | 20 | `sub_80EC844` (080EC844) | 2 | 0 | 00000000 | 30 | 00 | 0 | 0 | 158 | 1 |  |
| 09D | 157 | Recov80 | HKV* | Null | Null | Std | 1 | 24 | 48 | 40 | 20 | `sub_80EC844` (080EC844) | 3 | 0 | 00000000 | 30 | 00 | 0 | 0 | 159 | 1 |  |
| 09E | 158 | Recov120 | FPS | Null | Null | Std | 1 | 32 | 48 | 50 | 20 | `sub_80EC844` (080EC844) | 4 | 0 | 00000000 | 30 | 00 | 0 | 0 | 160 | 1 |  |
| 09F | 159 | Recov150 | JMT | Null | Null | Std | 2 | 38 | 48 | 50 | 20 | `sub_80EC844` (080EC844) | 5 | 0 | 00000000 | 30 | 00 | 0 | 0 | 161 | 1 |  |
| 0A0 | 160 | Recov200 | IQZ | Null | Null | Std | 2 | 42 | 48 | 50 | 20 | `sub_80EC844` (080EC844) | 6 | 0 | 00000000 | 30 | 00 | 0 | 0 | 162 | 1 |  |
| 0A1 | 161 | Recov300 | JOY | Null | Null | Std | 3 | 48 | 48 | 49 | 20 | `sub_80EC844` (080EC844) | 7 | 0 | 00000000 | 30 | 00 | 0 | 0 | 163 | 1 |  |
| 0A2 | 162 | PanlGrab | * | Null | Null | Std | 0 | 6 | 49 | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[0] = `sub_80E07E0` (080E07E0) | 0 | 0 | 00000000 | 0 | 00 | 0 | 10 | 164 | 1 |  |
| 0A3 | 163 | AreaGrab | BFS* | Null | Null | Std | 1 | 8 | 49 | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[0] = `sub_80E07E0` (080E07E0) | 0 | 0 | 00000001 | 0 | 00 | 0 | 10 | 165 | 1 |  |
| 0A4 | 164 | GrabBnsh | BMS | Null | Null | Std | 2 | 24 | 49 | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[15] = `sub_80E2D76` (080E2D76) | 15 | 0 | 00000000 | 0 | 00 | 0 | 20 | 166 | 1 |  |
| 0A5 | 165 | GrabRvng | IQZ | Null | Null | Std | 3 | 50 | 49 | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[15] = `sub_80E2D76` (080E2D76) | 15 | 0 | 00000000 | 0 | 00 | 0 | 40 | 167 | 1 |  |
| 0A6 | 166 | PnlRetrn | * | Null | Null | Std | 1 | 14 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[5] = `sub_80E2B5A` (080E2B5A) | 5 | 0 | 00000000 | 0 | 80 | 0 | 0 | 168 | 1 |  |
| 0A7 | 167 | Geddon | ALR* | Null | Null | Std | 2 | 47 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[3] = `sub_80E2566` (080E2566) | 3 | 0 | 00000001 | 0 | 00 | 0 | 0 | 169 | 1 |  |
| 0A8 | 168 | HolyPanl | ABS* | Null | Null | Std | 1 | 24 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[5] = `sub_80E2B5A` (080E2B5A) | 5 | 0 | 00000004 | 0 | 00 | 0 | 0 | 170 | 1 |  |
| 0A9 | 169 | Snctuary | Z | Null | Null | Std | 4 | 62 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[5] = `sub_80E2B5A` (080E2B5A) | 5 | 0 | 00000005 | 0 | 00 | 0 | 0 | 171 | 1 |  |
| 0AA | 170 | ComingRd | * | Null | Null | Std | 0 | 21 | 41 | 30 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[5] = `sub_80E2B5A` (080E2B5A) | 5 | 0 | 00000011 | 0 | 00 | 0 | 40 | 172 | 1 |  |
| 0AB | 171 | GoingRd | * | Null | Null | Std | 0 | 21 | 41 | 30 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[5] = `sub_80E2B5A` (080E2B5A) | 5 | 0 | 00000012 | 0 | 00 | 0 | 40 | 173 | 1 |  |
| 0AC | 172 | SloGauge | ABG* | Null | Null | Std | 2 | 42 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[25] = `sub_80E24B8` (080E24B8) | 25 | 0 | 00000000 | 0 | 00 | 0 | 0 | 174 | 1 |  |
| 0AD | 173 | FstGauge | EMR* | Null | Null | Std | 2 | 48 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[25] = `sub_80E24B8` (080E24B8) | 25 | 0 | 00000001 | 0 | 00 | 0 | 0 | 175 | 1 |  |
| 0AE | 174 | FullCust | * | Null | Null | Std | 3 | 50 | 48 | 0 | 1C | `sub_80EC39C` (080EC39C) | 5 | 0 | 00000000 | 20 | 80 | 0 | 0 | 176 | 1 |  |
| 0AF | 175 | BusterUp | * | Null | Plus | Std | 1 | 11 | 48 | 0 | 1C | `sub_80EC39C` (080EC39C) | 10 | 0 | 00000001 | 20 | 00 | 0 | 0 | 177 | 3 |  |
| 0B0 | 176 | BugFix | KPZ* | Null | Null | Std | 3 | 62 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[26] = `sub_80E49A2` (080E49A2) | 26 | 0 | 00000000 | 0 | 00 | 0 | 0 | 178 | 1 |  |
| 0B1 | 177 | Invisibl | * | Null | Null | Std | 2 | 30 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[1] = `sub_80E7546` (080E7546) | 1 | 0 | 00000168 | 0 | 82 | 0 | 0 | 179 | 3 |  |
| 0B2 | 178 | Barrier | AFR* | Null | Null | Std | 1 | 7 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[4] = `sub_80E3B50` (080E3B50) | 4 | 0 | 00000001 | 0 | 80 | 0 | 0 | 180 | 1 |  |
| 0B3 | 179 | Barr100 | HOY | Null | Null | Std | 2 | 30 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[4] = `sub_80E3B50` (080E3B50) | 4 | 0 | 00000005 | 0 | 00 | 0 | 0 | 181 | 1 |  |
| 0B4 | 180 | Barr200 | KUW | Null | Null | Std | 3 | 52 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[4] = `sub_80E3B50` (080E3B50) | 4 | 0 | 00000007 | 0 | 00 | 0 | 0 | 182 | 1 |  |
| 0B5 | 181 | BblWrap | IQZ | Null | Aqua | Std | 2 | 58 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[4] = `sub_80E3B50` (080E3B50) | 4 | 0 | 00000008 | 0 | 00 | 0 | 0 | 183 | 1 |  |
| 0B6 | 182 | LifeAur | U | Null | Null | Std | 4 | 70 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[4] = `sub_80E3B50` (080E3B50) | 4 | 0 | 00000009 | 0 | 00 | 0 | 0 | 184 | 1 |  |
| 0B7 | 183 | MagCoil | * | Null | Null | Std | 0 | 14 | 48 | 30 | 44 | `sub_80EE6FC` (080EE6FC) | 0 | 0 | 0000003C | 0 | 80 | 0 | 0 | 185 | 3 |  |
| 0B8 | 184 | WhiCapsl | * | Null | Null | Std | 1 | 30 | 40 | 50 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 40000000 | 20 | 40 | 0 | 0 | 192 | 3 |  |
| 0B9 | 185 | Uninstll | GLR | Null | Null | Std | 3 | 60 | 40 | 30 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 20000000 | 0 | 40 | 0 | 0 | 193 | 1 |  |
| 0BA | 186 | AntiNavi | FLT* | Null | Null | Std | 3 | 50 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[20] = `sub_80E353E` (080E353E) | 20 | 0 | 00000002 | 0 | 00 | 0 | 0 | 188 | 1 |  |
| 0BB | 187 | AntiDmg | GRV* | Null | Null | Std | 2 | 30 | 4B | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[20] = `sub_80E353E` (080E353E) | 20 | 0 | 00000003 | 0 | 00 | 0 | 100 | 189 | 1 |  |
| 0BC | 188 | AntiSwrd | ARZ* | Null | Sword | Std | 2 | 33 | 4B | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[20] = `sub_80E353E` (080E353E) | 20 | 0 | 00000004 | 0 | 00 | 0 | 100 | 190 | 1 |  |
| 0BD | 189 | AntiRecv | AFV* | Null | Null | Std | 2 | 37 | 49 | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[20] = `sub_80E353E` (080E353E) | 20 | 0 | 00000001 | 0 | 00 | 0 | 0 | 191 | 1 |  |
| 0BE | 190 | CopyDmg | * | Null | Cursor | Std | 1 | 12 | 48 | 0 | 23 | `sub_80ECA34` (080ECA34) | 0 | 1 | 00000000 | 0 | 80 | 1 | 0 | 194 | 3 |  |
| 0BF | 191 | LifeSync | * | Null | Cursor | Std | 1 | 12 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[7] = `sub_80E7464` (080E7464) | 7 | 1 | 00000000 | 0 | 80 | 4 | 0 | 195 | 3 |  |
| 0C0 | 192 | Atk+10 | * | Null | Plus | Std | 0 | 4 | 48 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 20 | C0 | 0 | 10 | 196 | 1 |  |
| 0C1 | 193 | Navi+20 | * | Null | Plus | Std | 2 | 36 | 48 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000001 | 20 | 40 | 0 | 20 | 198 | 1 |  |
| 0C2 | 194 | ColorPt | * | Null | Plus | Std | 1 | 31 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[27] = `sub_80E64E8` (080E64E8) | 27 | 0 | 00000A00 | 0 | 00 | 0 | 0 | 199 | 1 |  |
| 0C3 | 195 | Atk+30 | * | Null | Plus | Std | 3 | 66 | 48 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 20 | 40 | 0 | 30 | 197 | 1 |  |
| 0C4 | 196 | DblPoint | * | Null | Plus | Std | 3 | 50 | 49 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[27] = `sub_80E64E8` (080E64E8) | 27 | 0 | 00001401 | 0 | 00 | 0 | 0 | 200 | 1 |  |
| 0C5 | 197 | ElemTrap | GSU* | Null | Null | Std | 2 | 42 | 4B | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[20] = `sub_80E353E` (080E353E) | 20 | 0 | 00000000 | 0 | 00 | 0 | 240 | 187 | 1 |  |
| 0C6 | 198 | ColArmy | BGR* | Null | Null | Std | 2 | 25 | 4A | 30 | 1C | `sub_80EC39C` (080EC39C) | 22 | 0 | 00000000 | 0 | 00 | 0 | 40 | 186 | 3 |  |
| 0C7 | 199 | BlzrdBal | HNT | Aqua | Aqua | Std | 3 | 39 | 4B | 158 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[21] = `sub_80E7008` (080E7008) | 21 | 0 | 00000000 | 0 | 00 | 0 | 150 | 115 | 1 |  |
| 0C8 | 200 | TimeBom2 | CDE | Null | Obj | Std | 2 | 30 | 4B | 148 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[10] = `sub_80E3242` (080E3242) | 10 | 0 | 00000000 | 30 | 00 | 0 | 190 | 146 | 3 |  |
| 0C9 | 201 | TimeBom3 | LMN | Null | Obj | Std | 3 | 37 | 4B | 148 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[10] = `sub_80E3242` (080E3242) | 10 | 0 | 00000000 | 30 | 00 | 0 | 230 | 147 | 3 |  |
| 0CA | 202 | BigBomb | OPV | Null | Null | Std | 3 | 32 | 4A | 30 | 12 | `sub_80EB628` (080EB628) | 15 | 1 | 00000003 | 0 | 00 | 15 | 140 | 59 | 3 |  |
| 0CB | 203 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 203 | 3 |  |
| 0CC | 204 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 204 | 3 |  |
| 0CD | 205 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 205 | 3 |  |
| 0CE | 206 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 206 | 3 |  |
| 0CF | 207 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 207 | 3 |  |
| 0D0 | 208 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 208 | 3 |  |
| 0D1 | 209 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 209 | 3 |  |
| 0D2 | 210 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 210 | 3 |  |
| 0D3 | 211 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 211 | 3 |  |
| 0D4 | 212 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 212 | 3 |  |
| 0D5 | 213 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 213 | 3 |  |
| 0D6 | 214 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 214 | 3 |  |
| 0D7 | 215 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 215 | 3 |  |
| 0D8 | 216 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 216 | 3 |  |
| 0D9 | 217 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 217 | 3 |  |
| 0DA | 218 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 218 | 3 |  |
| 0DB | 219 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 219 | 3 |  |
| 0DC | 220 | - | ABC* | Null | Null | Std | 0 | 8 | 02 | 30 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 1 | 00000000 | 0 | 00 | 1 | 40 | 220 | 3 |  |
| 0DD | 221 | Roll | R* | Null | Null | Mega | 2 | 20 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[0] = `sub_80C0DD8` (080C0DD8) | 0 | 0 | 00000000 | 0 | 00 | 0 | 20 | 221 | 1 |  |
| 0DE | 222 | Roll2 | R | Null | Null | Mega | 3 | 40 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[0] = `sub_80C0DD8` (080C0DD8) | 0 | 0 | 00000003 | 0 | 00 | 0 | 40 | 222 | 1 |  |
| 0DF | 223 | Roll3 | R | Null | Null | Mega | 4 | 60 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[0] = `sub_80C0DD8` (080C0DD8) | 0 | 0 | 00000004 | 0 | 00 | 0 | 60 | 223 | 1 |  |
| 0E0 | 224 | ProtoMan | B* | Null | Sword | Mega | 2 | 41 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[1] = `sub_80C2A4C` (080C2A4C) | 1 | 0 | 00080000 | 0 | 00 | 0 | 150 | 224 | 1 |  |
| 0E1 | 225 | ProtoMn[EX] | B | Null | Sword | Mega | 3 | 53 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[1] = `sub_80C2A4C` (080C2A4C) | 1 | 0 | 00080001 | 0 | 00 | 0 | 170 | 225 | 1 |  |
| 0E2 | 226 | ProtoMn[SP] | B | Null | Sword | Mega | 4 | 68 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[1] = `sub_80C2A4C` (080C2A4C) | 1 | 0 | 00080002 | 0 | 00 | 0 | 1011=var[11] | 226 | 1 |  |
| 0E3 | 227 | HeatMan | H* | Fire | Fire | Mega | 2 | 32 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[2] = `sub_80B921A` (080B921A) | 2 | 0 | 00000000 | 0 | 01 | 0 | 100 | 242 | 1 |  |
| 0E4 | 228 | HeatMan[EX] | H | Fire | Fire | Mega | 3 | 55 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[2] = `sub_80B921A` (080B921A) | 2 | 0 | 00000000 | 0 | 01 | 0 | 130 | 243 | 1 |  |
| 0E5 | 229 | HeatMan[SP] | H | Fire | Fire | Mega | 4 | 70 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[2] = `sub_80B921A` (080B921A) | 2 | 0 | 00000000 | 0 | 01 | 0 | 1001=var[1] | 244 | 1 |  |
| 0E6 | 230 | ElecMan | E* | Elec | Elec | Mega | 2 | 38 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[3] = `sub_80B9432` (080B9432) | 3 | 0 | 00000000 | 0 | 01 | 0 | 120 | 245 | 1 |  |
| 0E7 | 231 | ElecMan[EX] | E | Elec | Elec | Mega | 3 | 52 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[3] = `sub_80B9432` (080B9432) | 3 | 0 | 00000003 | 0 | 01 | 0 | 140 | 246 | 1 |  |
| 0E8 | 232 | ElecMan[SP] | E | Elec | Elec | Mega | 4 | 79 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[3] = `sub_80B9432` (080B9432) | 3 | 0 | 00000004 | 0 | 01 | 0 | 1002=var[2] | 247 | 1 |  |
| 0E9 | 233 | SlashMan | S* | Null | Sword | Mega | 2 | 42 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[4] = `sub_80BA1B0` (080BA1B0) | 4 | 0 | 0000000A | 0 | 01 | 0 | 80 | 248 | 1 |  |
| 0EA | 234 | SlashMn[EX] | S | Null | Sword | Mega | 3 | 65 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[4] = `sub_80BA1B0` (080BA1B0) | 4 | 0 | 00000014 | 0 | 01 | 0 | 100 | 249 | 1 |  |
| 0EB | 235 | SlashMn[SP] | S | Null | Sword | Mega | 4 | 79 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[4] = `sub_80BA1B0` (080BA1B0) | 4 | 0 | 00000014 | 0 | 01 | 0 | 1003=var[3] | 250 | 1 |  |
| 0EC | 236 | EraseMan | K* | Null | Cursor | Mega | 2 | 51 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[5] = `sub_80BB7F6` (080BB7F6) | 5 | 0 | 00000014 | 0 | 01 | 0 | 120 | 251 | 1 |  |
| 0ED | 237 | EraseMn[EX] | K | Null | Cursor | Mega | 3 | 65 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[5] = `sub_80BB7F6` (080BB7F6) | 5 | 0 | 00000010 | 0 | 01 | 0 | 140 | 252 | 1 |  |
| 0EE | 238 | EraseMn[SP] | K | Null | Cursor | Mega | 4 | 79 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[5] = `sub_80BB7F6` (080BB7F6) | 5 | 0 | 0000000C | 0 | 01 | 0 | 1004=var[4] | 253 | 1 |  |
| 0EF | 239 | ChrgeMan | C* | Null | Null | Mega | 2 | 42 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[6] = `sub_80BBAC6` (080BBAC6) | 6 | 0 | 00000000 | 0 | 01 | 0 | 60 | 254 | 1 |  |
| 0F0 | 240 | ChrgeMn[EX] | C | Null | Null | Mega | 3 | 63 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[6] = `sub_80BBAC6` (080BBAC6) | 6 | 0 | 00000000 | 0 | 01 | 0 | 70 | 255 | 1 |  |
| 0F1 | 241 | ChrgeMn[SP] | C | Null | Null | Mega | 4 | 81 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[6] = `sub_80BBAC6` (080BBAC6) | 6 | 0 | 00000004 | 0 | 01 | 0 | 1005=var[5] | 256 | 1 |  |
| 0F2 | 242 | SpoutMan | A* | Aqua | Aqua | Mega | 2 | 42 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[7] = `sub_80B9750` (080B9750) | 7 | 0 | 00000000 | 0 | 00 | 0 | 50 | 227 | 1 |  |
| 0F3 | 243 | SpoutMn[EX] | A | Aqua | Aqua | Mega | 3 | 56 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[7] = `sub_80B9750` (080B9750) | 7 | 0 | 00000000 | 0 | 00 | 0 | 60 | 228 | 1 |  |
| 0F4 | 244 | SpoutMn[SP] | A | Aqua | Aqua | Mega | 4 | 78 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[7] = `sub_80B9750` (080B9750) | 7 | 0 | 00000000 | 0 | 00 | 0 | 1006=var[6] | 229 | 1 |  |
| 0F5 | 245 | TmhkMan | T* | Wood | Wood | Mega | 2 | 40 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[8] = `sub_80B999A` (080B999A) | 8 | 0 | 00000000 | 0 | 00 | 0 | 140 | 230 | 1 |  |
| 0F6 | 246 | TmhkMan[EX] | T | Wood | Wood | Mega | 3 | 60 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[8] = `sub_80B999A` (080B999A) | 8 | 0 | 00000000 | 0 | 00 | 0 | 160 | 231 | 1 |  |
| 0F7 | 247 | TmhkMan[SP] | T | Wood | Wood | Mega | 4 | 80 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[8] = `sub_80B999A` (080B999A) | 8 | 0 | 00000000 | 0 | 00 | 0 | 1007=var[7] | 232 | 1 |  |
| 0F8 | 248 | TenguMan | T* | Null | Wind | Mega | 2 | 43 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[9] = `sub_80B9F0E` (080B9F0E) | 9 | 0 | 00000000 | 0 | 00 | 0 | 70 | 233 | 1 |  |
| 0F9 | 249 | TenguMn[EX] | T | Null | Wind | Mega | 3 | 61 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[9] = `sub_80B9F0E` (080B9F0E) | 9 | 0 | 00000003 | 0 | 00 | 0 | 90 | 234 | 1 |  |
| 0FA | 250 | TenguMn[SP] | T | Null | Wind | Mega | 4 | 74 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[9] = `sub_80B9F0E` (080B9F0E) | 9 | 0 | 00000004 | 0 | 00 | 0 | 1008=var[8] | 235 | 1 |  |
| 0FB | 251 | GrndMan | G* | Null | Break | Mega | 2 | 41 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[10] = `sub_80BBDE8` (080BBDE8) | 10 | 0 | 20010800 | 0 | 00 | 0 | 60 | 236 | 1 |  |
| 0FC | 252 | GrndMan[EX] | G | Null | Break | Mega | 3 | 66 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[10] = `sub_80BBDE8` (080BBDE8) | 10 | 0 | 20010803 | 0 | 00 | 0 | 70 | 237 | 1 |  |
| 0FD | 253 | GrndMan[SP] | G | Null | Break | Mega | 4 | 85 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[10] = `sub_80BBDE8` (080BBDE8) | 10 | 0 | 20010804 | 0 | 00 | 0 | 1009=var[9] | 238 | 1 |  |
| 0FE | 254 | DustMan | D* | Null | Null | Mega | 2 | 39 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[11] = `sub_80BC0DA` (080BC0DA) | 11 | 0 | 00000000 | 0 | 00 | 0 | 110 | 239 | 1 |  |
| 0FF | 255 | DustMan[EX] | D | Null | Null | Mega | 3 | 56 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[11] = `sub_80BC0DA` (080BC0DA) | 11 | 0 | 00000000 | 0 | 00 | 0 | 130 | 240 | 1 |  |
| 100 | 256 | DustMan[SP] | D | Null | Null | Mega | 4 | 74 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[11] = `sub_80BC0DA` (080BC0DA) | 11 | 0 | 00000004 | 0 | 00 | 0 | 1010=var[10] | 241 | 1 |  |
| 101 | 257 | BlastMan | B* | Fire | Fire | Mega | 2 | 30 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[12] = `sub_80B9014` (080B9014) | 12 | 0 | 00000000 | 0 | 00 | 0 | 120 | 257 | 1 |  |
| 102 | 258 | BlastMn[EX] | B | Fire | Fire | Mega | 3 | 49 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[12] = `sub_80B9014` (080B9014) | 12 | 0 | 00000000 | 0 | 00 | 0 | 140 | 258 | 1 |  |
| 103 | 259 | BlastMn[SP] | B | Fire | Fire | Mega | 4 | 68 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[12] = `sub_80B9014` (080B9014) | 12 | 0 | 00000000 | 0 | 00 | 0 | 1012=var[12] | 259 | 1 |  |
| 104 | 260 | DiveMan | D* | Aqua | Aqua | Mega | 2 | 45 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[13] = `sub_80B9B6E` (080B9B6E) | 13 | 0 | 00000000 | 0 | 00 | 0 | 130 | 260 | 1 |  |
| 105 | 261 | DiveMan[EX] | D | Aqua | Aqua | Mega | 3 | 60 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[13] = `sub_80B9B6E` (080B9B6E) | 13 | 0 | 00000000 | 0 | 00 | 0 | 150 | 261 | 1 |  |
| 106 | 262 | DiveMan[SP] | D | Aqua | Aqua | Mega | 4 | 75 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[13] = `sub_80B9B6E` (080B9B6E) | 13 | 0 | 00000000 | 0 | 00 | 0 | 1013=var[13] | 262 | 1 |  |
| 107 | 263 | CrcusMan | C* | Null | Null | Mega | 2 | 42 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[14] = `sub_80BA660` (080BA660) | 14 | 0 | 00000000 | 0 | 00 | 0 | 20 | 263 | 1 |  |
| 108 | 264 | CrcusMn[EX] | C | Null | Null | Mega | 3 | 64 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[14] = `sub_80BA660` (080BA660) | 14 | 0 | 00000000 | 0 | 00 | 0 | 25 | 264 | 1 |  |
| 109 | 265 | CrcusMn[SP] | C | Null | Null | Mega | 4 | 86 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[14] = `sub_80BA660` (080BA660) | 14 | 0 | 00000004 | 0 | 00 | 0 | 1014=var[14] | 265 | 1 |  |
| 10A | 266 | JudgeMan | J* | Elec | Elec | Mega | 2 | 52 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[15] = `sub_80BA920` (080BA920) | 15 | 0 | 00000014 | 0 | 00 | 0 | 100 | 266 | 1 |  |
| 10B | 267 | JudgeMn[EX] | J | Elec | Elec | Mega | 3 | 62 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[15] = `sub_80BA920` (080BA920) | 15 | 0 | 0000001E | 0 | 00 | 0 | 120 | 267 | 1 |  |
| 10C | 268 | JudgeMn[SP] | J | Elec | Elec | Mega | 4 | 72 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[15] = `sub_80BA920` (080BA920) | 15 | 0 | 00000028 | 0 | 00 | 0 | 1015=var[15] | 268 | 1 |  |
| 10D | 269 | ElmntMan | E* | Null | Null | Mega | 2 | 50 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[16] = `sub_80BAE16` (080BAE16) | 16 | 0 | 00000010 | 0 | 00 | 0 | 100 | 269 | 1 |  |
| 10E | 270 | ElmntMn[EX] | E | Null | Null | Mega | 3 | 53 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[16] = `sub_80BAE16` (080BAE16) | 16 | 0 | 0000000E | 0 | 00 | 0 | 120 | 270 | 1 |  |
| 10F | 271 | ElmntMn[SP] | E | Null | Null | Mega | 4 | 66 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[16] = `sub_80BAE16` (080BAE16) | 16 | 0 | 0000000C | 0 | 00 | 0 | 1016=var[16] | 271 | 1 |  |
| 110 | 272 | Colonel | C* | Null | Sword | Mega | 2 | 45 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[17] = `sub_80B84EC` (080B84EC) | 17 | 0 | 00000000 | 0 | 00 | 0 | 160 | 272 | 1 |  |
| 111 | 273 | Colonel[EX] | C | Null | Sword | Mega | 3 | 70 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[17] = `sub_80B84EC` (080B84EC) | 17 | 0 | 00000000 | 0 | 00 | 0 | 180 | 273 | 1 |  |
| 112 | 274 | Colonel[SP] | C | Null | Sword | Mega | 4 | 91 | 47 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[17] = `sub_80B84EC` (080B84EC) | 17 | 0 | 00000004 | 0 | 00 | 0 | 1018=var[18] | 274 | 1 |  |
| 113 | 275 | HackJack | H* | Null | Null | Mega | 2 | 60 | 07 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[18] = NULL | 18 | 0 | 00000032 | 0 | 00 | 0 | 20 | 275 | 1 |  |
| 114 | 276 | HackJck[EX] | H | Null | Null | Mega | 3 | 75 | 07 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[18] = NULL | 18 | 0 | 00000046 | 0 | 00 | 0 | 25 | 276 | 1 |  |
| 115 | 277 | HackJck[SP] | H | Null | Null | Mega | 4 | 89 | 07 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[18] = NULL | 18 | 0 | 00000064 | 0 | 00 | 0 | 1017=var[17] | 277 | 1 |  |
| 116 | 278 | Django | D* | Null | Null | Spec | 2 | 30 | 00 | 0 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[19] = NULL | 19 | 0 | 00000032 | 0 | 00 | 0 | 130 | 278 | 1 |  |
| 117 | 279 | Django2 | D | Null | Null | Spec | 3 | 70 | 00 | 0 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[19] = NULL | 19 | 0 | 00000050 | 0 | 00 | 0 | 180 | 279 | 1 |  |
| 118 | 280 | Django3 | D | Null | Null | Spec | 4 | 90 | 00 | 0 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[19] = NULL | 19 | 0 | 00000078 | 0 | 00 | 0 | 260 | 280 | 1 |  |
| 119 | 281 | PunchArm | ABC* | Null | Null | Std | 4 | 10 | 00 | 20 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[38] = `sub_80E979C` (080E979C) | 38 | 0 | 00002302 | 20 | 30 | 0 | 0 | 0 | 1 |  |
| 11A | 282 | NeedlArm | ABC* | Null | Null | Std | 4 | 30 | 00 | 30 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[38] = `sub_80E979C` (080E979C) | 38 | 0 | 00012402 | 20 | 30 | 0 | 0 | 0 | 1 |  |
| 11B | 283 | PuzzlArm | ABC* | Null | Null | Std | 4 | 40 | 00 | 30 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[38] = `sub_80E979C` (080E979C) | 38 | 0 | 00022502 | 20 | 30 | 0 | 0 | 0 | 1 |  |
| 11C | 284 | BoomrArm | ABC* | Null | Null | Std | 4 | 50 | 00 | 10 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[38] = `sub_80E979C` (080E979C) | 38 | 0 | 00032602 | 20 | 30 | 0 | 0 | 0 | 1 |  |
| 11D | 285 | SyncTrgr | ABC* | Null | Null | Mega | 4 | 80 | 00 | 10 | 1C | `sub_80EC39C` (080EC39C) | 13 | 0 | 00000000 | 20 | 30 | 0 | 0 | 0 | 1 |  |
| 11E | 286 | DrkSword | ABC* | Null | Sword | Std | 4 | 80 | 02 | 40 | 13 | `sub_80EB776` (080EB776) | 6 | 1 | 00000000 | 0 | 30 | 6 | 400 | 0 | 1 | 0 |
| 11F | 287 | DarkThnd | ABC* | Elec | Elec | Std | 4 | 80 | 02 | 20 | 1F | `sub_80EC7A6` (080EC7A6) | 0 | 1 | 00100C01 | 0 | 30 | 4 | 200 | 0 | 1 | 1 |
| 120 | 288 | DrkRecov | ABC* | Null | Null | Std | 4 | 80 | 00 | 49 | 20 | `sub_80EC844` (080EC844) | 8 | 0 | 00000000 | 0 | 30 | 0 | 0 | 0 | 1 | 2 |
| 121 | 289 | DarkInvs | ABC* | Null | Null | Std | 4 | 64 | 01 | 50 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[38] = `sub_80E979C` (080E979C) | 38 | 0 | 00000003 | 0 | 30 | 0 | 0 | 0 | 1 | 3 |
| 122 | 290 | DarkPlus | ABC* | Null | Null | Std | 4 | 80 | 02 | 10 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 20 | 30 | 0 | 50 | 0 | 1 | 4 |
| 123 | 291 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 124 | 292 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 125 | 293 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 126 | 294 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 127 | 295 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 128 | 296 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 129 | 297 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 12A | 298 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 12B | 299 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 12C | 300 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 12D | 301 | Bass | F | Null | Null | Giga | 4 | 95 | 03 | 133 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[26] = `sub_80C3B30` (080C3B30) | 26 | 0 | 00000000 | 0 | 14 | 0 | 60 | 301 | 1 |  |
| 12E | 302 | BigHook | H | Null | Break | Giga | 4 | 92 | 03 | 148 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[23] = `sub_80EA11C` (080EA11C) | 23 | 0 | 00000A01 | 0 | 18 | 0 | 240 | 302 | 1 |  |
| 12F | 303 | DeltaRay | Z | Null | Sword | Giga | 4 | 82 | 03 | 148 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[28] = `sub_80C2F96` (080C2F96) | 28 | 0 | 000C0000 | 0 | 14 | 0 | 260 | 303 | 1 |  |
| 130 | 304 | ColForce | Q | Null | Null | Giga | 4 | 90 | 02 | 138 | 1C | `sub_80EC39C` (080EC39C) | 15 | 0 | 00000000 | 0 | 14 | 0 | 30 | 304 | 1 |  |
| 131 | 305 | BugRSwrd | V | Null | Sword | Giga | 4 | 80 | 01 | 168 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[38] = `sub_80E979C` (080E979C) | 38 | 0 | 00002101 | 0 | 14 | 0 | 0 | 305 | 1 |  |
| 132 | 306 | BassAnly | F | Null | Null | Giga | 4 | 95 | 43 | 138 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[27] = `sub_80C3E98` (080C3E98) | 27 | 0 | 00000000 | 0 | 18 | 0 | 160 | 306 | 1 |  |
| 133 | 307 | MetrKnuk | N | Null | Break | Giga | 4 | 90 | 43 | 148 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[32] = `sub_80E8BC0` (080E8BC0) | 32 | 0 | 00000000 | 0 | 14 | 0 | 100 | 307 | 1 |  |
| 134 | 308 | CrossDiv | D | Null | Sword | Giga | 4 | 93 | 43 | 148 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[17] = `sub_80B84EC` (080B84EC) | 17 | 0 | 00010000 | 0 | 18 | 0 | 250 | 308 | 1 |  |
| 135 | 309 | HubBatc | J | Null | Null | Giga | 4 | 99 | 41 | 148 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[38] = `sub_80E979C` (080E979C) | 38 | 0 | 00000000 | 0 | 18 | 0 | 0 | 309 | 1 |  |
| 136 | 310 | BgDthThd | V | Elec | Elec | Giga | 4 | 80 | 41 | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[38] = `sub_80E979C` (080E979C) | 38 | 0 | 00002201 | 0 | 18 | 0 | 0 | 310 | 1 |  |
| 137 | 311 | DblBeast | W | Null | Null | Mega | 4 | 99 | 01 | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[37] = `sub_80E943E` (080E943E) | 37 | 0 | 00000000 | 0 | 10 | 0 | 0 | 281 | 1 |  |
| 138 | 312 | Gregar | X | Null | Null | Giga | 4 | 99 | 03 | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[34] = NULL | 34 | 0 | 00000064 | 0 | 10 | 0 | 300 | 312 | 1 |  |
| 139 | 313 | Falzar | X | Null | Null | Giga | 4 | 99 | 03 | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[35] = NULL | 35 | 0 | 00000000 | 0 | 10 | 0 | 100 | 313 | 1 |  |
| 13A | 314 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 13B | 315 | BatCan1 | * | Null | Null | Std | 0 | 5 | 02 | 30 | 19 | `sub_80EC0E6` (080EC0E6) | 0 | 1 | 00000300 | 0 | 00 | 1 | 40 | 315 | 1 |  |
| 13C | 316 | BatCan2 | S | Null | Null | Std | 0 | 5 | 02 | 30 | 19 | `sub_80EC0E6` (080EC0E6) | 1 | 1 | 00000301 | 0 | 00 | 1 | 40 | 316 | 1 |  |
| 13D | 317 | BatCan3 | R | Null | Null | Std | 0 | 5 | 02 | 30 | 19 | `sub_80EC0E6` (080EC0E6) | 2 | 1 | 00000302 | 0 | 00 | 1 | 40 | 317 | 1 |  |
| 13E | 318 | BatCan4 | Z | Null | Null | Std | 0 | 5 | 02 | 30 | 19 | `sub_80EC0E6` (080EC0E6) | 3 | 1 | 00000000 | 0 | 00 | 1 | 40 | 318 | 1 |  |
| 13F | 319 | BeastOut | * | Null | Null | Giga | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 0 | 0 | 00000000 | 0 | 00 | 0 | 10 | 319 | 1 |  |
| 140 | 320 | GigaCan1 | ABC* | Null | PA | PA | 0 | 5 | 42 | 50 | 14 | `sub_80EBC0E` (080EBC0E) | 4 | 1 | 00000000 | 0 | 00 | 1 | 300 | 320 | 1 |  |
| 141 | 321 | GigaCan2 | ABC* | Null | PA | PA | 0 | 5 | 42 | 50 | 14 | `sub_80EBC0E` (080EBC0E) | 5 | 1 | 00000000 | 0 | 00 | 1 | 400 | 321 | 1 |  |
| 142 | 322 | GigaCan3 | ABC* | Null | PA | PA | 0 | 5 | 42 | 50 | 14 | `sub_80EBC0E` (080EBC0E) | 6 | 1 | 00000000 | 0 | 00 | 1 | 500 | 322 | 1 |  |
| 143 | 323 | WideBrn1 | ABC* | Fire | PA | PA | 0 | 5 | 42 | 2 | 27 | `sub_80ECD28` (080ECD28) | 0 | 1 | 0001823C | 0 | 00 | 9 | 300 | 323 | 1 |  |
| 144 | 324 | WideBrn2 | ABC* | Fire | PA | PA | 0 | 5 | 42 | 2 | 27 | `sub_80ECD28` (080ECD28) | 0 | 1 | 0001823C | 0 | 00 | 9 | 350 | 324 | 1 |  |
| 145 | 325 | WideBrn3 | ABC* | Fire | PA | PA | 0 | 5 | 42 | 2 | 27 | `sub_80ECD28` (080ECD28) | 0 | 1 | 0001823C | 0 | 00 | 9 | 400 | 325 | 1 |  |
| 146 | 326 | FlmHook1 | ABC* | Fire | PA | PA | 0 | 5 | 42 | 10 | 1C | `sub_80EC39C` (080EC39C) | 14 | 0 | 00000100 | 20 | 00 | 0 | 300 | 326 | 1 |  |
| 147 | 327 | FlmHook2 | ABC* | Fire | PA | PA | 0 | 5 | 42 | 10 | 1C | `sub_80EC39C` (080EC39C) | 14 | 0 | 00000100 | 20 | 00 | 0 | 350 | 327 | 1 |  |
| 148 | 328 | FlmHook3 | ABC* | Fire | PA | PA | 0 | 5 | 42 | 10 | 1C | `sub_80EC39C` (080EC39C) | 14 | 0 | 00000100 | 20 | 00 | 0 | 400 | 328 | 1 |  |
| 149 | 329 | PwrWave1 | ABC* | Null | PA | PA | 0 | 5 | 42 | 10 | 31 | `sub_80ED64C` (080ED64C) | 1 | 0 | 0000000F | 0 | 00 | 0 | 400 | 329 | 1 |  |
| 14A | 330 | PwrWave2 | ABC* | Null | PA | PA | 0 | 5 | 42 | 10 | 31 | `sub_80ED64C` (080ED64C) | 1 | 0 | 0000000F | 0 | 00 | 0 | 500 | 330 | 1 |  |
| 14B | 331 | PwrWave3 | ABC* | Null | PA | PA | 0 | 5 | 42 | 10 | 31 | `sub_80ED64C` (080ED64C) | 1 | 0 | 0000000F | 0 | 00 | 0 | 600 | 331 | 1 |  |
| 14C | 332 | CornFsta | ABC* | Wood | PA | PA | 0 | 5 | 43 | 10 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[29] = `sub_80E7F16` (080E7F16) | 29 | 0 | 00000102 | 20 | 00 | 0 | 40 | 332 | 1 |  |
| 14D | 333 | ParaShl | ABC* | Null | PA | PA | 0 | 5 | 42 | 10 | 2C | `sub_80ED25C` (080ED25C) | 0 | 0 | 00010100 | 20 | 00 | 0 | 350 | 333 | 1 |  |
| 14E | 334 | DestPuls | ABC* | Elec | PA | PA | 0 | 5 | 42 | 10 | 42 | `sub_80EE55E` (080EE55E) | 0 | 1 | 033C1200 | 20 | 00 | 17 | 400 | 334 | 1 |  |
| 14F | 335 | TimeBom+ | ABC* | Null | PA | PA | 0 | 5 | 43 | 168 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[10] = `sub_80E3242` (080E3242) | 10 | 0 | 00000001 | 0 | 00 | 0 | 700 | 335 | 1 |  |
| 150 | 336 | StreamHd | - | Null | PA | PA | 0 | 5 | 42 | 5 | 43 | `sub_80EE634` (080EE634) | 1 | 1 | 01000100 | 20 | 00 | 1 | 150 | 336 | 1 |  |
| 151 | 337 | SuprSpr | ABC* | Aqua | PA | PA | 0 | 5 | 42 | 10 | 30 | `sub_80ED55C` (080ED55C) | 1 | 0 | 00000006 | 0 | 00 | 0 | 150 | 337 | 1 |  |
| 152 | 338 | H-Burst | ABC* | Null | PA | PA | 0 | 5 | 42 | 5 | 34 | `sub_80ED810` (080ED810) | 10 | 1 | 0000000A | 0 | 00 | 1 | 60 | 338 | 1 |  |
| 153 | 339 | LifeSrd | ABC* | Null | PA | PA | 0 | 5 | 42 | 40 | 13 | `sub_80EB776` (080EB776) | 5 | 1 | 00000000 | 0 | 00 | 6 | 400 | 339 | 1 |  |
| 154 | 340 | GreatYo | ABC* | Null | PA | PA | 0 | 5 | 42 | 10 | 18 | `sub_80EC02A` (080EC02A) | 1 | 0 | 05040101 | 0 | 00 | 0 | 100 | 340 | 1 |  |
| 155 | 341 | PitHocky | - | Null | PA | PA | 0 | 5 | 42 | 5 | 26 | `sub_80ECCB0` (080ECCB0) | 0 | 0 | 00000103 | 0 | 00 | 0 | 100 | 341 | 1 |  |
| 156 | 342 | PoisPhar | - | Null | PA | PA | 0 | 5 | 41 | 128 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[17] = `sub_80E4164` (080E4164) | 17 | 0 | 00000001 | 0 | 00 | 0 | 1 | 342 | 1 |  |
| 157 | 343 | BodyGrd | ABC* | Null | PA | PA | 0 | 5 | 43 | 133 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[20] = `sub_80E353E` (080E353E) | 20 | 0 | 00000005 | 0 | 00 | 0 | 100 | 343 | 1 |  |
| 158 | 344 | DblHero | ABC* | Null | PA | PA | 0 | 5 | 41 | 138 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[30] = `sub_80E8058` (080E8058) | 30 | 0 | 00000000 | 0 | 00 | 0 | 60 | 344 | 1 |  |
| 159 | 345 | Darkness | ABC* | Null | PA | PA | 0 | 5 | 43 | 143 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[24] = `sub_80BFCD0` (080BFCD0) | 24 | 0 | 00000000 | 0 | 00 | 0 | 300 | 345 | 1 |  |
| 15A | 346 | MstrCros | ABC* | Null | PA | PA | 0 | 5 | 43 | 158 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[22] = `sub_80BF160` (080BF160) | 22 | 0 | 00000000 | 0 | 00 | 0 | 100 | 346 | 1 |  |
| 15B | 347 | SunMoon | ABC* | Null | PA | PA | 0 | 5 | 43 | 168 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[25] = `sub_80BF6AE` (080BF6AE) | 25 | 0 | 00000000 | 0 | 00 | 0 | 200 | 347 | 1 |  |
| 15C | 348 | TwinLdrs | ABC* | Null | PA | PA | 0 | 5 | 43 | 133 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[20] = `sub_80BD9A2` (080BD9A2) | 20 | 0 | 00000000 | 0 | 00 | 0 | 200 | 348 | 1 |  |
| 15D | 349 | CrosOver | ABC* | Null | PA | PA | 0 | 5 | 41 | 130 | 1B | `sub_80EC350` (080EC350) → off_802CD5C[21] = `sub_80BE3E8` (080BE3E8) | 21 | 0 | 00000000 | 0 | 00 | 0 | 100 | 349 | 1 |  |
| 15E | 350 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 15F | 351 | - | - | Null | Null | Spec | 0 | 5 | 02 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 1 | 0 | 1 |  |
| 160 | 352 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 161 | 353 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 162 | 354 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 163 | 355 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 164 | 356 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 165 | 357 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 166 | 358 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 167 | 359 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 168 | 360 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 169 | 361 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 16A | 362 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 16B | 363 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 16C | 364 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 16D | 365 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 16E | 366 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 16F | 367 | - | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 0 | 1 |  |
| 170 | 368 | - | ABC* | Null | Null | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 368 | 1 |  |
| 171 | 369 | ???? | ABC* | Null | Misc | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 369 | 1 |  |
| 172 | 370 | FtrSword | ELS* | Null | Sword | Spec | 0 | 8 | 42 | 30 | 13 | `sub_80EB776` (080EB776) | 9 | 1 | 00000000 | 0 | 00 | 5 | 80 | 370 | 1 |  |
| 173 | 371 | SonicBom | ELS* | Null | Sword | Spec | 0 | 8 | 42 | 30 | 55 | `sub_80EF970` (080EF970) | 0 | 0 | 00000003 | 0 | 00 | 3 | 80 | 371 | 1 |  |
| 174 | 372 | Curse | ELS* | Null | Sword | Spec | 0 | 8 | 42 | 30 | 55 | `sub_80EF970` (080EF970) | 0 | 0 | 00000000 | 0 | 00 | 0 | 80 | 372 | 1 |  |
| 175 | 373 | Punisher | ELS* | Null | Sword | Spec | 0 | 8 | 42 | 30 | 55 | `sub_80EF970` (080EF970) | 0 | 0 | 00000000 | 0 | 00 | 0 | 80 | 373 | 1 |  |
| 176 | 374 | CrosSwrd | ELS* | Null | Sword | Spec | 0 | 8 | 42 | 30 | 13 | `sub_80EB776` (080EB776) | 10 | 1 | 00000000 | 0 | 00 | 13 | 80 | 374 | 1 |  |
| 177 | 375 | SprSonic | ELS* | Null | Sword | Spec | 0 | 8 | 42 | 30 | 55 | `sub_80EF970` (080EF970) | 1 | 0 | 00000101 | 0 | 00 | 14 | 80 | 375 | 1 |  |
| 178 | 376 | DblDream | ELS* | Null | Sword | Spec | 0 | 8 | 42 | 30 | 13 | `sub_80EB776` (080EB776) | 11 | 1 | 00000000 | 0 | 00 | 6 | 80 | 376 | 1 |  |
| 179 | 377 | Rush | ABC* | Null | Null | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 377 | 1 |  |
| 17A | 378 | Beat | ABC* | Null | Null | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000001 | 0 | 00 | 0 | 40 | 378 | 1 |  |
| 17B | 379 | Tango | ABC* | Null | Null | Spec | 0 | 5 | 00 | 0 | 14 | `sub_80EBC0E` (080EBC0E) | 0 | 0 | 00000000 | 0 | 00 | 0 | 40 | 379 | 1 |  |
| 17C | 380 | IceCube | ABC* | Null | Obj | Spec | 0 | 5 | 00 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[6] = `sub_80E46B6` (080E46B6) | 6 | 0 | 00000003 | 0 | 00 | 0 | 200 | 380 | 1 |  |
| 17D | 381 | Z Saver | ABC* | Null | Sword | Spec | 0 | 5 | 00 | 148 | 5B | `sub_80EFEE0` (080EFEE0) | 0 | 0 | 00000000 | 0 | 00 | 0 | 100 | 381 | 1 |  |
| 17E | 382 | WhiCapsl | - | Null | Null | Spec | 0 | 30 | 01 | 50 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[1] = `sub_80E7546` (080E7546) | 1 | 0 | 00000078 | 0 | 02 | 0 | 0 | 382 | 1 |  |
| 17F | 383 | PrpCapsl | - | Null | Obj | Spec | 0 | 6 | 00 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[3] = `sub_80E2566` (080E2566) | 3 | 0 | 00000000 | 0 | 00 | 0 | 0 | 383 | 1 |  |
| 180 | 384 | PnkCapsl | - | Null | Obj | Spec | 0 | 6 | 00 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[3] = `sub_80E2566` (080E2566) | 3 | 0 | 00000001 | 0 | 00 | 0 | 0 | 384 | 1 |  |
| 181 | 385 | HealBall | - | Null | Misc | Spec | 0 | 99 | 01 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[3] = `sub_80E2566` (080E2566) | 3 | 0 | 00000000 | 0 | 00 | 0 | 0 | 385 | 1 |  |
| 182 | 386 | MagPanl | - | Null | Misc | Spec | 0 | 99 | 01 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[3] = `sub_80E2566` (080E2566) | 3 | 0 | 00000000 | 0 | 00 | 0 | 0 | 386 | 1 |  |
| 183 | 387 | FinalGun | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 10 | 387 | 1 |  |
| 184 | 388 | NumTrap | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 10 | 388 | 1 |  |
| 185 | 389 | - | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 0 | 0 | 00000000 | 20 | 00 | 0 | 0 | 389 | 1 |  |
| 186 | 390 | BeastOut | - | Null | Misc | Spec | 0 | 99 | 01 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[3] = `sub_80E2566` (080E2566) | 3 | 0 | 00000000 | 0 | 00 | 0 | 0 | 390 | 1 |  |
| 187 | 391 | BeastOut | - | Null | Misc | Spec | 0 | 99 | 01 | 0 | 15 | `sub_80EBD9C` (080EBD9C) → off_802CCB4[3] = `sub_80E2566` (080E2566) | 3 | 0 | 00000000 | 0 | 00 | 0 | 0 | 391 | 1 |  |
| 188 | 392 | - | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 10 | 392 | 1 |  |
| 189 | 393 | - | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 10 | 393 | 1 |  |
| 18A | 394 | - | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 10 | 394 | 1 |  |
| 18B | 395 | - | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 10 | 395 | 1 |  |
| 18C | 396 | - | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 10 | 396 | 1 |  |
| 18D | 397 | - | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 10 | 397 | 1 |  |
| 18E | 398 | - | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 10 | 398 | 1 |  |
| 18F | 399 | - | - | Null | Null | Spec | 0 | 99 | 00 | 0 | 1C | `sub_80EC39C` (080EC39C) | 3 | 0 | 00000000 | 0 | 00 | 0 | 10 | 399 | 1 |  |
| 190 | 400 | HeatPres | H | Fire | Fire | Spec | 0 | 99 | C2 | 10 | 0A | per-navi base table off_80EA4C8[AIIndex][0xa] | 3 | 0 | 00000000 | 0 | 00 | 0 | 1024=var[24] | 370 | 1 |  |
| 191 | 401 | DElecSwd | E | Elec | Elec | Spec | 1 | 99 | C2 | 10 | 0A | per-navi base table off_80EA4C8[AIIndex][0xa] | 3 | 0 | 00000000 | 0 | 00 | 0 | 1026=var[26] | 49 | 3 |  |
| 192 | 402 | RSlash | S | Null | Sword | Spec | 0 | 99 | C2 | 10 | 0A | per-navi base table off_80EA4C8[AIIndex][0xa] | 3 | 0 | 00000000 | 0 | 00 | 0 | 1028=var[28] | 402 | 1 |  |
| 193 | 403 | EDeletBm | K | Null | Cursor | Spec | 0 | 99 | C2 | 10 | 0A | per-navi base table off_80EA4C8[AIIndex][0xa] | 3 | 0 | 00000000 | 0 | 00 | 0 | 1030=var[30] | 403 | 1 |  |
| 194 | 404 | VolcChrg | C | Fire | Fire | Spec | 0 | 99 | C2 | 10 | 0A | per-navi base table off_80EA4C8[AIIndex][0xa] | 3 | 0 | 00000000 | 0 | 00 | 0 | 1032=var[32] | 404 | 1 |  |
| 195 | 405 | DripShwr | A | Aqua | Aqua | Spec | 0 | 99 | C2 | 10 | 0A | per-navi base table off_80EA4C8[AIIndex][0xa] | 3 | 0 | 00000000 | 0 | 00 | 0 | 1034=var[34] | 405 | 1 |  |
| 196 | 406 | ETomahwk | T | Null | Null | Spec | 0 | 99 | C2 | 10 | 0A | per-navi base table off_80EA4C8[AIIndex][0xa] | 3 | 0 | 00000000 | 0 | 00 | 0 | 1036=var[36] | 406 | 1 |  |
| 197 | 407 | FTornado | T | Null | Wind | Spec | 0 | 99 | C2 | 10 | 0A | per-navi base table off_80EA4C8[AIIndex][0xa] | 3 | 0 | 00000000 | 0 | 00 | 0 | 1038=var[38] | 407 | 1 |  |
| 198 | 408 | RC Brakr | G | Null | Break | Spec | 0 | 99 | C2 | 10 | 0A | per-navi base table off_80EA4C8[AIIndex][0xa] | 3 | 0 | 00000000 | 0 | 00 | 0 | 1040=var[40] | 408 | 1 |  |
| 199 | 409 | DustBrk | D | Null | Break | Spec | 0 | 99 | C2 | 10 | 0A | per-navi base table off_80EA4C8[AIIndex][0xa] | 3 | 0 | 00000000 | 0 | 00 | 0 | 1042=var[42] | 409 | 1 |  |
| 19A | 410 | StepSwrd | B | Null | Sword | Spec | 0 | 99 | C2 | 10 | 13 | `sub_80EB776` (080EB776) | 1 | 0 | 00000001 | 0 | 00 | 0 | 1044=var[44] | 410 | 1 |  |

## JumpTable80EAC60 (0x080EAC60): action handlers

`sub_801B9E6` dispatches `CurAction >= 0x10` to `JumpTable80EAC60[CurAction - 0x10]` (unless AIAttackVars+0x1D == 1,
which routes through the Beast-Out wrapper `sub_80EAD9C`, which itself calls the same table from `sub_80EAF36`).
79 entries (actions 0x10..0x5E); the word after the table is code (`sub_80EAD9C`).

| index | action | handler | chip ids using it |
|---|---|---|---|
| 0 | 10 | `sub_80EB04C` (080EB04C) | (no chip; non-chip action) |
| 1 | 11 | `sub_80EB436` (080EB436) | (no chip; non-chip action) |
| 2 | 12 | `sub_80EB628` (080EB628) | 036 MiniBomb, 037 EnergBom, 038 MegEnBom, 039 FlshBom1, 03A FlshBom2, 03B FlshBom3, 03C BlkBomb, 043 BugBomb, 044 GrasSeed, 045 IceSeed, 046 PoisSeed, 062 LilBolr1, 063 LilBolr2, 064 LilBolr3, 096 VDoll, 0CA BigBomb |
| 3 | 13 | `sub_80EB776` (080EB776) | 047 Sword, 048 WideSwrd, 049 LongSwrd, 04A WideBlde, 04B LongBlde, 04C FireSwrd, 04D AquaSwrd, 04E ElecSwrd, 04F BambSwrd, 051 StepSwrd, 055 Muramasa, 11E DrkSword, 153 LifeSrd, 172 FtrSword, 176 CrosSwrd, 178 DblDream, 19A StepSwrd |
| 4 | 14 | `sub_80EBC0E` (080EBC0E) | 001 Cannon, 002 HiCannon, 003 M-Cannon, 0CB -, 0CC -, 0CD -, 0CE -, 0CF -, 0D0 -, 0D1 -, 0D2 -, 0D3 -, 0D4 -, 0D5 -, 0D6 -, 0D7 -, 0D8 -, 0D9 -, 0DA -, 0DB -, 0DC -, 140 GigaCan1, 141 GigaCan2, 142 GigaCan3, 160 -, 161 -, 162 -, 163 -, 164 -, 165 -, 166 -, 167 -, 168 -, 169 -, 16A -, 16B -, 16C -, 16D -, 16E -, 16F -, 170 -, 171 ????, 179 Rush, 17A Beat, 17B Tango |
| 5 | 15 | `sub_80EBD9C` (080EBD9C) | 068 AirRaid1, 069 AirRaid2, 06A AirRaid3, 06E BurnSqr1, 06F BurnSqr2, 070 BurnSqr3, 071 Sensor1, 072 Sensor2, 073 Sensor3, 081 Wind, 082 Fan, 086 Snake, 087 SumnBlk1, 088 SumnBlk2, 089 SumnBlk3, 08A NumbrBl, 08B Meteors, 08D Magnum, 08E CircGun, 08F RockCube, 090 TimeBom1, 091 Mine, 092 Fanfare, 093 Discord, 094 Timpani, 095 Silence, 097 Guardian, 098 Anubis, 099 Otenko, 0A2 PanlGrab, 0A3 AreaGrab, 0A4 GrabBnsh, 0A5 GrabRvng, 0A6 PnlRetrn, 0A7 Geddon, 0A8 HolyPanl, 0A9 Snctuary, 0AA ComingRd, 0AB GoingRd, 0AC SloGauge, 0AD FstGauge, 0B0 BugFix, 0B1 Invisibl, 0B2 Barrier, 0B3 Barr100, 0B4 Barr200, 0B5 BblWrap, 0B6 LifeAur, 0BA AntiNavi, 0BB AntiDmg, 0BC AntiSwrd, 0BD AntiRecv, 0BF LifeSync, 0C2 ColorPt, 0C4 DblPoint, 0C5 ElemTrap, 0C7 BlzrdBal, 0C8 TimeBom2, 0C9 TimeBom3, 119 PunchArm, 11A NeedlArm, 11B PuzzlArm, 11C BoomrArm, 121 DarkInvs, 131 BugRSwrd, 133 MetrKnuk, 135 HubBatc, 136 BgDthThd, 137 DblBeast, 138 Gregar, 139 Falzar, 14C CornFsta, 14F TimeBom+, 156 PoisPhar, 157 BodyGrd, 158 DblHero, 17C IceCube, 17E WhiCapsl, 17F PrpCapsl, 180 PnkCapsl, 181 HealBall, 182 MagPanl, 186 BeastOut, 187 BeastOut |
| 6 | 16 | `sub_80EBE00` (080EBE00) | (no chip; non-chip action) |
| 7 | 17 | `sub_80EBF10` (080EBF10) | 005 Vulcan1, 006 Vulcan2, 007 Vulcan3, 008 SuprVulc |
| 8 | 18 | `sub_80EC02A` (080EC02A) | 013 YoYo, 154 GreatYo |
| 9 | 19 | `sub_80EC0E6` (080EC0E6) | 13B BatCan1, 13C BatCan2, 13D BatCan3, 13E BatCan4 |
| 10 | 1A | `sub_80EC1F0` (080EC1F0) | (no chip; non-chip action) |
| 11 | 1B | `sub_80EC350` (080EC350) | 0DD Roll, 0DE Roll2, 0DF Roll3, 0E0 ProtoMan, 0E1 ProtoMn[EX], 0E2 ProtoMn[SP], 0E3 HeatMan, 0E4 HeatMan[EX], 0E5 HeatMan[SP], 0E6 ElecMan, 0E7 ElecMan[EX], 0E8 ElecMan[SP], 0E9 SlashMan, 0EA SlashMn[EX], 0EB SlashMn[SP], 0EC EraseMan, 0ED EraseMn[EX], 0EE EraseMn[SP], 0EF ChrgeMan, 0F0 ChrgeMn[EX], 0F1 ChrgeMn[SP], 0F2 SpoutMan, 0F3 SpoutMn[EX], 0F4 SpoutMn[SP], 0F5 TmhkMan, 0F6 TmhkMan[EX], 0F7 TmhkMan[SP], 0F8 TenguMan, 0F9 TenguMn[EX], 0FA TenguMn[SP], 0FB GrndMan, 0FC GrndMan[EX], 0FD GrndMan[SP], 0FE DustMan, 0FF DustMan[EX], 100 DustMan[SP], 101 BlastMan, 102 BlastMn[EX], 103 BlastMn[SP], 104 DiveMan, 105 DiveMan[EX], 106 DiveMan[SP], 107 CrcusMan, 108 CrcusMn[EX], 109 CrcusMn[SP], 10A JudgeMan, 10B JudgeMn[EX], 10C JudgeMn[SP], 10D ElmntMan, 10E ElmntMn[EX], 10F ElmntMn[SP], 110 Colonel, 111 Colonel[EX], 112 Colonel[SP], 113 HackJack, 114 HackJck[EX], 115 HackJck[SP], 116 Django, 117 Django2, 118 Django3, 12D Bass, 12E BigHook, 12F DeltaRay, 132 BassAnly, 134 CrossDiv, 159 Darkness, 15A MstrCros, 15B SunMoon, 15C TwinLdrs, 15D CrosOver |
| 12 | 1C | `sub_80EC39C` (080EC39C) | 000 MegaBstr, 065 SandWrm1, 066 SandWrm2, 067 SandWrm3, 06B FireHit1, 06C FireHit2, 06D FireHit3, 074 Boomer, 075 HiBoomer, 076 M-Boomer, 077 Lance, 078 GolmHit1, 079 GolmHit2, 07A GolmHit3, 08C JustcOne, 0AE FullCust, 0AF BusterUp, 0B8 WhiCapsl, 0B9 Uninstll, 0C0 Atk+10, 0C1 Navi+20, 0C3 Atk+30, 0C6 ColArmy, 11D SyncTrgr, 122 DarkPlus, 123 -, 124 -, 125 -, 126 -, 127 -, 128 -, 129 -, 12A -, 12B -, 12C -, 130 ColForce, 13A -, 13F BeastOut, 146 FlmHook1, 147 FlmHook2, 148 FlmHook3, 15E -, 15F -, 183 FinalGun, 184 NumTrap, 185 -, 188 -, 189 -, 18A -, 18B -, 18C -, 18D -, 18E -, 18F - |
| 13 | 1D | `sub_80EC45E` (080EC45E) | (no chip; non-chip action) |
| 14 | 1E | `sub_80EC5BC` (080EC5BC) | (no chip; non-chip action) |
| 15 | 1F | `sub_80EC7A6` (080EC7A6) | 01E Thunder, 11F DarkThnd |
| 16 | 20 | `sub_80EC844` (080EC844) | 09A Recov10, 09B Recov30, 09C Recov50, 09D Recov80, 09E Recov120, 09F Recov150, 0A0 Recov200, 0A1 Recov300, 120 DrkRecov |
| 17 | 21 | `sub_80EC884` (080EC884) | 004 AirShot |
| 18 | 22 | `sub_80EC960` (080EC960) | 059 CrakShot, 05A DublShot, 05B TrplShot |
| 19 | 23 | `sub_80ECA34` (080ECA34) | 0BE CopyDmg |
| 20 | 24 | `sub_80ECACA` (080ECACA) | 00C TankCan1, 00D TankCan2, 00E TankCan3 |
| 21 | 25 | `sub_80ECBB0` (080ECBB0) | 009 Spreadr1, 00A Spreadr2, 00B Spreadr3 |
| 22 | 26 | `sub_80ECCB0` (080ECCB0) | 032 AirHocky, 155 PitHocky |
| 23 | 27 | `sub_80ECD28` (080ECD28) | 014 FireBrn1, 015 FireBrn2, 016 FireBrn3, 143 WideBrn1, 144 WideBrn2, 145 WideBrn3 |
| 24 | 28 | `sub_80ECDFC` (080ECDFC) | 018 TrnArrw1, 019 TrnArrw2, 01A TrnArrw3 |
| 25 | 29 | `sub_80ECF2E` (080ECF2E) | 02B MachGun1, 02C MachGun2, 02D MachGun3 |
| 26 | 2A | `sub_80ED090` (080ED090) | 040 CornSht1, 041 CornSht2, 042 CornSht3 |
| 27 | 2B | `sub_80ED13E` (080ED13E) | 083 Rflectr1, 084 Rflectr2, 085 Rflectr3 |
| 28 | 2C | `sub_80ED25C` (080ED25C) | 07B IronShl1, 07C IronShl2, 07D IronShl3, 14D ParaShl |
| 29 | 2D | `sub_80ED2F8` (080ED2F8) | 01B BblStar1, 01C BblStar2, 01D BblStar3 |
| 30 | 2E | `sub_80ED374` (080ED374) | 033 DrilArm |
| 31 | 2F | `sub_80ED454` (080ED454) | 034 Tornado, 035 Static |
| 32 | 30 | `sub_80ED55C` (080ED55C) | 017 WideSht, 151 SuprSpr |
| 33 | 31 | `sub_80ED64C` (080ED64C) | 05C WaveArm1, 05D WaveArm2, 05E WaveArm3, 149 PwrWave1, 14A PwrWave2, 14B PwrWave3 |
| 34 | 32 | `sub_80ED6E6` (080ED6E6) | 03D AquaNdl1, 03E AquaNdl2, 03F AquaNdl3 |
| 35 | 33 | `sub_80ED748` (080ED748) | (no chip; non-chip action) |
| 36 | 34 | `sub_80ED810` (080ED810) | 152 H-Burst |
| 37 | 35 | `sub_80ED8C4` (080ED8C4) | (no chip; non-chip action) |
| 38 | 36 | `sub_80ED9AE` (080ED9AE) | 028 RlngLog1, 029 RlngLog2, 02A RlngLog3 |
| 39 | 37 | `sub_80EDAE0` (080EDAE0) | 00F GunDelS1, 010 GunDelS2, 011 GunDelS3, 012 GunDelEX |
| 40 | 38 | `sub_80EDCC0` (080EDCC0) | 07E AirSpin1, 07F AirSpin2, 080 AirSpin3 |
| 41 | 39 | `sub_80EDD80` (080EDD80) | 025 RskyHny1, 026 RskyHny2, 027 RskyHny3 |
| 42 | 3A | `sub_80EDE7E` (080EDE7E) | (no chip; non-chip action) |
| 43 | 3B | `sub_80EDF0C` (080EDF0C) | (no chip; non-chip action) |
| 44 | 3C | `sub_80EDF5C` (080EDF5C) | (no chip; non-chip action) |
| 45 | 3D | `sub_80EE044` (080EE044) | (no chip; non-chip action) |
| 46 | 3E | `sub_80EE0BC` (080EE0BC) | 01F DolThdr1, 020 DolThdr2, 021 DolThdr3 |
| 47 | 3F | `sub_80EE192` (080EE192) | 050 WindRack |
| 48 | 40 | `sub_80EE2A0` (080EE2A0) | 054 MoonBld |
| 49 | 41 | `sub_80EE34A` (080EE34A) | (no chip; non-chip action) |
| 50 | 42 | `sub_80EE55E` (080EE55E) | 022 ElcPuls1, 023 ElcPuls2, 024 ElcPuls3, 14E DestPuls |
| 51 | 43 | `sub_80EE634` (080EE634) | 05F AuraHed1, 060 AuraHed2, 061 AuraHed3, 150 StreamHd |
| 52 | 44 | `sub_80EE6FC` (080EE6FC) | 0B7 MagCoil |
| 53 | 45 | `sub_80EE7D0` (080EE7D0) | (no chip; non-chip action) |
| 54 | 46 | `sub_80EE840` (080EE840) | (no chip; non-chip action) |
| 55 | 47 | `sub_80EE90C` (080EE90C) | (no chip; non-chip action) |
| 56 | 48 | `sub_80EEA3C` (080EEA3C) | (no chip; non-chip action) |
| 57 | 49 | `sub_80EEB4C` (080EEB4C) | 056 MchnSwrd, 057 ElemSwrd, 058 AssnSwrd |
| 58 | 4A | `sub_80EEC70` (080EEC70) | (no chip; non-chip action) |
| 59 | 4B | `sub_80EED56` (080EED56) | (no chip; non-chip action) |
| 60 | 4C | `sub_80EEE60` (080EEE60) | (no chip; non-chip action) |
| 61 | 4D | `sub_80EEFDC` (080EEFDC) | (no chip; non-chip action) |
| 62 | 4E | `sub_80EF1EC` (080EF1EC) | (no chip; non-chip action) |
| 63 | 4F | `sub_80EF282` (080EF282) | (no chip; non-chip action) |
| 64 | 50 | `sub_80EF328` (080EF328) | (no chip; non-chip action) |
| 65 | 51 | `sub_80EF4B4` (080EF4B4) | 02E HeatDrgn, 02F ElecDrgn, 030 AquaDrgn, 031 WoodDrgn |
| 66 | 52 | `sub_80EF534` (080EF534) | (no chip; non-chip action) |
| 67 | 53 | `sub_80EF62E` (080EF62E) | 052 VarSwrd |
| 68 | 54 | `sub_80EF7E2` (080EF7E2) | 053 NeoVari |
| 69 | 55 | `sub_80EF970` (080EF970) | 173 SonicBom, 174 Curse, 175 Punisher, 177 SprSonic |
| 70 | 56 | `sub_80EFA56` (080EFA56) | (no chip; non-chip action) |
| 71 | 57 | `sub_80EFC1C` (080EFC1C) | (no chip; non-chip action) |
| 72 | 58 | `sub_80EFCB4` (080EFCB4) | (no chip; non-chip action) |
| 73 | 59 | `sub_80EFDB2` (080EFDB2) | (no chip; non-chip action) |
| 74 | 5A | `sub_80EFE7C` (080EFE7C) | (no chip; non-chip action) |
| 75 | 5B | `sub_80EFEE0` (080EFEE0) | 17D Z Saver |
| 76 | 5C | `sub_80F0094` (080F0094) | (no chip; non-chip action) |
| 77 | 5D | `sub_80F00F2` (080F00F2) | (no chip; non-chip action) |
| 78 | 5E | `sub_80F020E` (080F020E) | (no chip; non-chip action) |

Chips with action < 0x10 (dispatched through the per-navi base table `off_80EA4C8[AIIndex]`, i.e. the
current cross/beast form's own table): 190 HeatPres (act A), 191 DElecSwd (act A), 192 RSlash (act A), 193 EDeletBm (act A), 194 VolcChrg (act A), 195 DripShwr (act A), 196 ETomahwk (act A), 197 FTornado (act A), 198 RC Brakr (act A), 199 DustBrk (act A).

## off_802CCB4 (0x0802CCB4): dimming (action 0x15) effect spawners, indexed by chip +0x0C

Called once by `sub_80EBD9C` (action 0x15) and by `sub_8017AB4` (cut-in while dimmed) with
r0 = PanelX, r1 = PanelY, r2 = AIAttackVars+0x02 (element byte), r4 = AIAttackVars+0x0C (+0x10 params),
r6 = AIAttackVars+0x08 (damage word), r7 = (AIAttackVars+0x06 << 16) | chip id.

| index | spawner | chip ids |
|---|---|---|
| 0 | `sub_80E07E0` (080E07E0) | 0A2 PanlGrab, 0A3 AreaGrab |
| 1 | `sub_80E7546` (080E7546) | 0B1 Invisibl, 17E WhiCapsl |
| 2 | `sub_80E2F24` (080E2F24) |  |
| 3 | `sub_80E2566` (080E2566) | 0A7 Geddon, 17F PrpCapsl, 180 PnkCapsl, 181 HealBall, 182 MagPanl, 186 BeastOut, 187 BeastOut |
| 4 | `sub_80E3B50` (080E3B50) | 0B2 Barrier, 0B3 Barr100, 0B4 Barr200, 0B5 BblWrap, 0B6 LifeAur |
| 5 | `sub_80E2B5A` (080E2B5A) | 0A6 PnlRetrn, 0A8 HolyPanl, 0A9 Snctuary, 0AA ComingRd, 0AB GoingRd |
| 6 | `sub_80E46B6` (080E46B6) | 08F RockCube, 17C IceCube |
| 7 | `sub_80E7464` (080E7464) | 0BF LifeSync |
| 8 | `sub_80E3128` (080E3128) | 081 Wind, 082 Fan |
| 9 | `sub_80E5EA8` (080E5EA8) | 092 Fanfare, 093 Discord, 094 Timpani, 095 Silence |
| 10 | `sub_80E3242` (080E3242) | 090 TimeBom1, 0C8 TimeBom2, 0C9 TimeBom3, 14F TimeBom+ |
| 11 | `sub_80E349E` (080E349E) | 091 Mine |
| 12 | `sub_80E59C6` (080E59C6) | 086 Snake |
| 13 | `sub_80E5F78` (080E5F78) | 068 AirRaid1, 069 AirRaid2, 06A AirRaid3 |
| 14 | `sub_80E67E6` (080E67E6) | 097 Guardian |
| 15 | `sub_80E2D76` (080E2D76) | 0A4 GrabBnsh, 0A5 GrabRvng |
| 16 | `sub_80E4288` (080E4288) | 08B Meteors |
| 17 | `sub_80E4164` (080E4164) | 098 Anubis, 156 PoisPhar |
| 18 | `sub_80E76D4` (080E76D4) | 099 Otenko |
| 19 | `sub_80E7600` (080E7600) | 08E CircGun |
| 20 | `sub_80E353E` (080E353E) | 0BA AntiNavi, 0BB AntiDmg, 0BC AntiSwrd, 0BD AntiRecv, 0C5 ElemTrap, 157 BodyGrd |
| 21 | `sub_80E7008` (080E7008) | 0C7 BlzrdBal |
| 22 | `sub_80E7FBA` (080E7FBA) | 08A NumbrBl |
| 23 | `sub_80E70A6` (080E70A6) | 06E BurnSqr1, 06F BurnSqr2, 070 BurnSqr3 |
| 24 | `sub_80E723E` (080E723E) | 08D Magnum |
| 25 | `sub_80E24B8` (080E24B8) | 0AC SloGauge, 0AD FstGauge |
| 26 | `sub_80E49A2` (080E49A2) | 0B0 BugFix |
| 27 | `sub_80E64E8` (080E64E8) | 0C2 ColorPt, 0C4 DblPoint |
| 28 | `sub_80E7CA8` (080E7CA8) | 071 Sensor1, 072 Sensor2, 073 Sensor3 |
| 29 | `sub_80E7F16` (080E7F16) | 14C CornFsta |
| 30 | `sub_80E8058` (080E8058) | 158 DblHero |
| 31 | `sub_80E81B4` (080E81B4) |  |
| 32 | `sub_80E8BC0` (080E8BC0) | 133 MetrKnuk |
| 33 | `sub_80E8ADC` (080E8ADC) |  |
| 34 | NULL (the Japanese ROMs' 0x080EDE3C: beast-chips.md) | 138 Gregar |
| 35 | NULL (the Japanese ROMs' 0x080EE086: beast-chips.md) | 139 Falzar |
| 36 | `sub_80E91B8` (080E91B8) | 087 SumnBlk1, 088 SumnBlk2, 089 SumnBlk3 |
| 37 | `sub_80E943E` (080E943E) | 137 DblBeast |
| 38 | `sub_80E979C` (080E979C) | 119 PunchArm, 11A NeedlArm, 11B PuzzlArm, 11C BoomrArm, 121 DarkInvs, 131 BugRSwrd, 135 HubBatc, 136 BgDthThd |
| 39 | NULL |  |
| 40 | NULL |  |
| 41 | `sub_80E92EE` (080E92EE) |  |

## off_802CD5C (0x0802CD5C): Navi-chip (action 0x1B) summon handlers, indexed by chip +0x0C

Action 0x1B (`sub_80EC350`) calls `sub_80E192C`, which spawns T4 object 0x10 with Unk_19 = chip +0x0C;
that object's phase `sub_80E1880` calls `off_802CD5C[Unk_19]`.

| index | handler | chip ids |
|---|---|---|
| 0 | `sub_80C0DD8` (080C0DD8) | 0DD Roll, 0DE Roll2, 0DF Roll3 |
| 1 | `sub_80C2A4C` (080C2A4C) | 0E0 ProtoMan, 0E1 ProtoMn[EX], 0E2 ProtoMn[SP] |
| 2 | `sub_80B921A` (080B921A) | 0E3 HeatMan, 0E4 HeatMan[EX], 0E5 HeatMan[SP] |
| 3 | `sub_80B9432` (080B9432) | 0E6 ElecMan, 0E7 ElecMan[EX], 0E8 ElecMan[SP] |
| 4 | `sub_80BA1B0` (080BA1B0) | 0E9 SlashMan, 0EA SlashMn[EX], 0EB SlashMn[SP] |
| 5 | `sub_80BB7F6` (080BB7F6) | 0EC EraseMan, 0ED EraseMn[EX], 0EE EraseMn[SP] |
| 6 | `sub_80BBAC6` (080BBAC6) | 0EF ChrgeMan, 0F0 ChrgeMn[EX], 0F1 ChrgeMn[SP] |
| 7 | `sub_80B9750` (080B9750) | 0F2 SpoutMan, 0F3 SpoutMn[EX], 0F4 SpoutMn[SP] |
| 8 | `sub_80B999A` (080B999A) | 0F5 TmhkMan, 0F6 TmhkMan[EX], 0F7 TmhkMan[SP] |
| 9 | `sub_80B9F0E` (080B9F0E) | 0F8 TenguMan, 0F9 TenguMn[EX], 0FA TenguMn[SP] |
| 10 | `sub_80BBDE8` (080BBDE8) | 0FB GrndMan, 0FC GrndMan[EX], 0FD GrndMan[SP] |
| 11 | `sub_80BC0DA` (080BC0DA) | 0FE DustMan, 0FF DustMan[EX], 100 DustMan[SP] |
| 12 | `sub_80B9014` (080B9014) | 101 BlastMan, 102 BlastMn[EX], 103 BlastMn[SP] |
| 13 | `sub_80B9B6E` (080B9B6E) | 104 DiveMan, 105 DiveMan[EX], 106 DiveMan[SP] |
| 14 | `sub_80BA660` (080BA660) | 107 CrcusMan, 108 CrcusMn[EX], 109 CrcusMn[SP] |
| 15 | `sub_80BA920` (080BA920) | 10A JudgeMan, 10B JudgeMn[EX], 10C JudgeMn[SP] |
| 16 | `sub_80BAE16` (080BAE16) | 10D ElmntMan, 10E ElmntMn[EX], 10F ElmntMn[SP] |
| 17 | `sub_80B84EC` (080B84EC) | 110 Colonel, 111 Colonel[EX], 112 Colonel[SP], 134 CrossDiv |
| 18 | NULL | 113 HackJack, 114 HackJck[EX], 115 HackJck[SP] |
| 19 | NULL | 116 Django, 117 Django2, 118 Django3 |
| 20 | `sub_80BD9A2` (080BD9A2) | 15C TwinLdrs |
| 21 | `sub_80BE3E8` (080BE3E8) | 15D CrosOver |
| 22 | `sub_80BF160` (080BF160) | 15A MstrCros |
| 23 | `sub_80EA11C` (080EA11C) | 12E BigHook |
| 24 | `sub_80BFCD0` (080BFCD0) | 159 Darkness |
| 25 | `sub_80BF6AE` (080BF6AE) | 15B SunMoon |
| 26 | `sub_80C3B30` (080C3B30) | 12D Bass |
| 27 | `sub_80C3E98` (080C3E98) | 132 BassAnly |
| 28 | `sub_80C2F96` (080C2F96) | 12F DeltaRay |

## off_80109DC (0x080109DC): variable-damage formulas

`sub_80109A4(chip, alliance)` returns +0x1A if it is < 1000, else `off_80109DC[dmg - 1000](chip, alliance)`
(chip 0xFFFF → 0). 45 entries (damage 1000..1044).

| dmg | formula | chip ids |
|---|---|---|
| 1000 | `sub_8010A90` (08010A90) |  |
| 1001 | `sub_8010AE4` (08010AE4) | 0E5 HeatMan[SP] |
| 1002 | `sub_8010AE4` (08010AE4) | 0E8 ElecMan[SP] |
| 1003 | `sub_8010AE4` (08010AE4) | 0EB SlashMn[SP] |
| 1004 | `sub_8010AE4` (08010AE4) | 0EE EraseMn[SP] |
| 1005 | `sub_8010AE4` (08010AE4) | 0F1 ChrgeMn[SP] |
| 1006 | `sub_8010AE4` (08010AE4) | 0F4 SpoutMn[SP] |
| 1007 | `sub_8010AE4` (08010AE4) | 0F7 TmhkMan[SP] |
| 1008 | `sub_8010AE4` (08010AE4) | 0FA TenguMn[SP] |
| 1009 | `sub_8010AE4` (08010AE4) | 0FD GrndMan[SP] |
| 1010 | `sub_8010AE4` (08010AE4) | 100 DustMan[SP] |
| 1011 | `sub_8010AE4` (08010AE4) | 0E2 ProtoMn[SP] |
| 1012 | `sub_8010AE4` (08010AE4) | 103 BlastMn[SP] |
| 1013 | `sub_8010AE4` (08010AE4) | 106 DiveMan[SP] |
| 1014 | `sub_8010AE4` (08010AE4) | 109 CrcusMn[SP] |
| 1015 | `sub_8010AE4` (08010AE4) | 10C JudgeMn[SP] |
| 1016 | `sub_8010AE4` (08010AE4) | 10F ElmntMn[SP] |
| 1017 | `sub_8010AE4` (08010AE4) | 115 HackJck[SP] |
| 1018 | `sub_8010AE4` (08010AE4) | 112 Colonel[SP] |
| 1019 | `sub_8010B78` (08010B78) |  |
| 1020 | `sub_8010BD0` (08010BD0) | 055 Muramasa |
| 1021 | `sub_8010BF0` (08010BF0) | 08A NumbrBl |
| 1022 | `sub_8010C06` (08010C06) |  |
| 1023 | `sub_8010C50` (08010C50) |  |
| 1024 | `sub_8010C50` (08010C50) | 190 HeatPres |
| 1025 | `sub_8010C50` (08010C50) |  |
| 1026 | `sub_8010C50` (08010C50) | 191 DElecSwd |
| 1027 | `sub_8010C50` (08010C50) |  |
| 1028 | `sub_8010C50` (08010C50) | 192 RSlash |
| 1029 | `sub_8010C50` (08010C50) |  |
| 1030 | `sub_8010C50` (08010C50) | 193 EDeletBm |
| 1031 | `sub_8010C50` (08010C50) |  |
| 1032 | `sub_8010C50` (08010C50) | 194 VolcChrg |
| 1033 | `sub_8010C50` (08010C50) |  |
| 1034 | `sub_8010C50` (08010C50) | 195 DripShwr |
| 1035 | `sub_8010C50` (08010C50) |  |
| 1036 | `sub_8010C50` (08010C50) | 196 ETomahwk |
| 1037 | `sub_8010C50` (08010C50) |  |
| 1038 | `sub_8010C50` (08010C50) | 197 FTornado |
| 1039 | `sub_8010C50` (08010C50) |  |
| 1040 | `sub_8010C50` (08010C50) | 198 RC Brakr |
| 1041 | `sub_8010C50` (08010C50) |  |
| 1042 | `sub_8010C50` (08010C50) | 199 DustBrk |
| 1043 | `sub_8010C50` (08010C50) |  |
| 1044 | `sub_8010C50` (08010C50) | 19A StepSwrd |

Note: chips 0x138 Gregar and 0x139 Falzar have action 0x15 with +0x0C = 34/35, whose `off_802CCB4` slots are NULL;
in the US ROM using them crashes the game. The JP ROMs have the behaviour (jp-differences.md §4).
