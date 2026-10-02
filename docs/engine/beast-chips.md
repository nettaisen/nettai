# The Gregar and Falzar chips (the Japanese ROMs)

Chips 0x138 Gregar and 0x139 Falzar are giga cut-in chips (action 0x15, subtypes 0x22 and 0x23). The US ROMs
(BR6E, BR5E) have their records and nothing else: `off_802CCB4[34]` and `[35]` are null (the game jumps to
address 0), and the object slots their routines would fill point at placeholders. The Japanese ROMs (BR6J
Falzar, BR5J Gregar) have the routines; the port's are content/bn6's chips/gregar, chips/falzar and
lib/beast-chips. This is those routines, from the Japanese ROMs' bytes (there is no Japanese disassembly; every
routine they call is the US one of the same name, which bn6battle-verify's `tools/gregar/fmap.py --to` maps).
Conventions as in dimming-chip-effects.md (§0: the controllers' common shape, "counts", "Tn#x").

## 0. Where the routines are

Addresses are the Falzar ROM's (exe6f_rom_f.srl); the Gregar ROM's (exe6_rom.srl) are each block's plus the
offset given. **The two ROMs' routines are the same code**: every halfword equal, or a call or a literal word
that names the same routine or table in each (checked over every block below, its literal pools included).
Nothing in them depends on the game's version.

| Block | Falzar ROM | Gregar ROM | What |
|---|---|---|---|
| T1#0x30 | 0x080C3E0C–0x080C4268 | +0x1860 | Gregar's summon, its helpers |
| T1#0x31 | 0x080C4268–0x080C44B0 | +0x1860 | Falzar's summon |
| T4#0x7A | 0x080EDCC8–0x080EDE8C | +0x1330 | Gregar's controller; the spawner (`off_802CCB4[34]`) at 0x080EDE3C |
| T4#0x7B | 0x080EDE8C–0x080EE23C | +0x1330 | Falzar's controller, its helpers; the spawner (`[35]`) at 0x080EE086 |
| T4#0x7D | 0x080EE300–0x080EE3C8 | +0x1330 | the summons' arrival |

The attacks they spawn are US routines, in the US ROMs too but spawned by nothing there: T3#0xBC
(`sub_80DCB1C`, the feathers), T3#0xCD (`sub_80DF0A4`, Gregar's flames) and T3#0xCE (`sub_80DF188`,
Falzar's whirlwind), and the rocks (T3#0x80, GroundMan's: chips.md §3.6.25). The Japanese ROMs' copies of
them map to the US ones unchanged.

Two helpers nothing calls or points at: 0x080C41EE (in Gregar's block) and its copy 0x080EE194 (in Falzar's)
hit every panel with flags 0x33800000 and none of 0x0C000000 (`sub_80C53A6`, r4 0x2705FF01, damage 0xFFFF):
dead code, not ported.

## 1. The controllers

Both spawners are the common one (dimming-chip-effects.md §0): T4#0x7A / T4#0x7B, the common fields. The chips'
params: Gregar's 0x64 (Param1 100: the rocks' damage), Falzar's 0. Damage: Gregar 300, Falzar 100; hit
param 138; null.

Their actions differ from the common shape at the ends (0x080EDCFC / 0x080EDEC0):

| action | routine |
|---|---|
| 0 | 0x080EDD0C / 0x080EDED0: first tick `SetScreenFade(0x88, 0x10)` (the dimming's palettes, the stage's, to black: 16 steps from clear); once `IsScreenFadeActive` is false, action 4 |
| 4 | `object_drawChipName` (the telop, the cut-in window) |
| 8 | the effect |
| 0xC | `sub_800BCF6`: first tick, if the other side's record is Idle or Ending `SetScreenFade(0x84, 0x10)` (back to clear), else `SetScreenFade(0x3C, 0x100)` (to the dim at once); once the fade is done, state 8 (word) |

The port: `dimming.fade_to_black`, `dimming.fade_from_black` (crates/nettai-battle/src/dimming.rs), fade modes
0x88 and 0x84 (`FadeMode::BlackOut`, `BlackOutBack`).

The HUD helper (0x080EDE5E / 0x080EE1EC, r0 = hide): the word `{0x4010, 0x24000}[battle flag 0x40]` to
`sub_801DACC` (hide) or `sub_801DA48` (show): draw tasks 4 (the custom gauge) and 14 (the emotion window), or
17 (the flag 0x40 mode's gauge) and 14. Only the draw mask changes (presentation): `battle.show_hud`.

The field cleaner (0x080C4244 / 0x080EE21C): for each of BattleState+0xA0's eight slots that holds an object,
its HP (halfword) = 0, which breaks it (field-objects.md §2).

The target picker (0x080C40E6 / 0x080EE0A8, lib/beast-chips/targets.luau), with a 24-byte stack buffer:
`object_getPanelsExceptCurrentFiltered` with {side 0 (0x04010020, 0), side 1 (0x08010000, 0x20)} (the
other side's area with an enemy's body on it); if some, one RNG2 draw: `& 0xF < 9` → `sub_8000C72(buf, n,
n)` and the first entry unless it is the last aim (+0x60, +0x64), then (if n > 1) the second unless it is.
Otherwise the same with {side 0 (0x00010020, 0), side 1 (0x00010000, 0x20)} (the whole of the other side's
area): none → 0; shuffled; the first unless the last aim, else the second entry **without checking the count**
(a side always keeps a whole column, so two panels at least: unreachable), else 0.

**Gregar's effect** (0x080EDD32, by CurPhase, 0x080EDD44):

- 0 (0x080EDD54): first tick HUD hidden, `sub_80E1332(user, 1)` (the warp out), Timer 0x1E; then not > 0 →
  phase 4 (31 ticks).
- 4 (0x080EDD82): first tick two summons (`0x080C4068`) at (PanelX − front, PanelY), r6 = the damage word,
  r3 = the bonus (+0x32), r7 = &Param3, r4 = piece | Param1 << 8 (pieces 0 and 1, in that order); each tick
  (the first included), Param3 0 → phase 8.
- 8 (0x080EDDEA): 31 ticks → 0xC.
- 0xC (0x080EDE0A): first tick `sub_80E1332(user, 0)` (the warp in), Timer 0x1E; then not > 0 → the HUD shown,
  the effect ends.

**Falzar's effect** (0x080EDEF6, by CurPhase, 0x080EDF08):

- 0 (0x080EDF20): as Gregar's, but at its end the field cleaner, then phase 4.
- 4 (0x080EDF52): first tick +0x60 = +0x64 = 0, Timer 0x78, Timer2 0; each tick (the first included) Timer2
  −= 1, not > 0 → Timer2 = 0xC and the picker: a panel → +0x60/+0x64 and a feather there (0x080EE180:
  `sub_80DCC70(x, y, Element, r4 = 0x103, r6 = the damage word)`, no bonus); then Timer −= 1, not > 0 →
  phase 8. Ten feathers, at ticks 0, 12, …, 108.
- 8 (0x080EDF96): 31 ticks → 0xC.
- 0xC (0x080EDFB6): first tick three summons (`0x080C446A`) on the controller's panel, r6 = the damage word +
  the bonus, r7 = &Param1, r4 = (1, 0), (0, 4 << 8), (0, 8 << 8) (leads, first animation); each tick, Param1
  0 → phase 0x10.
- 0x10 (0x080EE034): 31 ticks → 0x14.
- 0x14 (0x080EE054): as Gregar's 0xC.

## 2. Gregar's summon, T1#0x30

Spawner 0x080C4068: `object_spawnType1(0x30)` (position = registers: PanelY, Element, the bonus); PanelX/Y,
Element, RelatedObject1 = the user, the user's alliance and flip, the damage word, +0x30 = the bonus,
CollisionDataPtr = the flag's address, `*flag = 1` (only when it spawned). Params: Param1 the piece, Param2 the
rocks' damage.

0x080C3E0C: by state (init, update, `object_freeMemory`), then `object_updateSpriteTimestop`.

- Init 0x080C3E30: coordinates from the panel, Z = 0, then Y and Z += Param1 px; `sprite_decompress` and
  `sprite_load(0x80, 0xC, 0x68)`, no shadow, CurAnim = CurAnimCopy = 4·Param1, palette 0, flip; +0x60 = +0x64
  = 0; state update (word).
- Actions (0x080C3EA4):
  - 0 (0x080C3EBC): first tick VISIBLE, Timer 0x3C, sound 0x77; piece 1: the arrival (`0x080EE3AE`, r4 = 0 |
    0x3C << 8) and the field cleaner. Then not > 0 → 4.
  - 4 (0x080C3F04): first tick CurAnim 1 + 4·piece, sound 0x143, `sprite_forceWhitePalette`, Timer 0xF; → 8.
  - 8 (0x080C3F40): first tick CurAnim 2 + 4·piece, `sprite_clearFinalPalette`, Timer 0x1E; → 0xC + 4·piece.
  - 0xC (0x080C3F7A, piece 0): first tick CurAnim 3, Timer 0xB4, counting from that tick; → 0x14.
  - 0x10 (0x080C3FA8, piece 1): first tick CurAnim 7, `sprite_clearFinalPalette`, Timer 0xB4, Timer2 0, sounds
    0x191 and 0x12B, +0x68 = 0x10, the flames (0x080C4098), camera shake (3, 0xB4). Each tick (the first
    included): Timer2 −= 1, not > 0 → Timer2 = 0x14 and the picker: a panel → +0x60/+0x64 and a rock there
    (0x080C41BC); +0x68 −= 1 (word), not > 0 → 0x10 and sound 0x12B; Timer −= 1, not > 0 → 0x14. Nine rocks.
  - 0x14 (0x080C4030): first tick CurAnim 2 + 4·piece, Timer 0xA; then not > 0 → piece 1: `*flag = 0`; state
    destroy (word).
- The flames (0x080C4098): around (PanelX + 2·front, PanelY), region 0x1A's panels (dx times front) on the
  field: `0x080DF160(x, y, Element, r3 = the bonus, r4 = 0xB4, r6 = the damage word + the bonus)`.
- A rock (0x080C41BC): `sub_80D54F0(x, y, Element, r3 = Damage & 0xF000, r4 = 0x01010105, r6 = ((Param2 | +0x2E
  << 16) + the bonus) | (Damage & 0xF000))`: GroundMan's rock with a 5-tick warning, acting while dimmed,
  cracking its panel, silent; its damage the chip's parameter byte plus the bonus.

## 3. Falzar's summon, T1#0x31

Spawner 0x080C446A: `object_spawnType1(0x31)` (position = registers); as Gregar's but no +0x30. Params: Param1
leads (1 for one piece), Param2 its first animation (0, 4, 8).

- Init 0x080C428C: coordinates from the panel, Z = 0x24 px, then Y and Z −= Param1 px; sprite 0c-66, no
  shadow, CurAnim = CurAnimCopy = Param2, palette 0, flip; state update (word).
- Actions (0x080C42F8): 0 (0x080C4310) as Gregar's, the leading piece bringing the arrival (r4 = 1 | 0x3C <<
  8), no field cleaner; 4 (0x080C4354) and 8 (0x080C438C) as Gregar's with CurAnim n + Param2, then by Param1
  (0x080C43C4: 0xC, 0x10); 0xC (0x080C43C8): CurAnim 3 + Param2, Timer 0x78; 0x10 (0x080C43F2): CurAnim 3 +
  Param2, Timer 0x78, sounds 0x193 and 0xF3, the whirlwind (0x080C4490: `sub_80DF262(PanelX + front, PanelY,
  Element, r4 = 0x78, r6 = the damage word)`), camera shake (3, 0x78); 0x14 (0x080C4436) as Gregar's, the
  leading piece clearing the flag.

## 4. The arrival, T4#0x7D

Spawner 0x080EE3AE: `object_spawnType4(0x7D)` (position = registers), RelatedObject1 = the summon, its
alliance and flip; Params r4 (Param1 the look, Param2 the ticks). 0x080EE300: by state (init, update,
`object_freeMemory`), then `object_updateSpriteTimestop`. Init 0x080EE324: X, Y, Z = the summon's, Y and Z +=
5 px; `sprite_load(0x80, 0xC, {0x68, 0x66}[Param1])`, no shadow, VISIBLE, CurAnim = CurAnimCopy = {8,
0xC}[Param1], palette 0, flip, Timer = Param2; state update. Update 0x080EE396: battle over or Timer −= 1 not
> 0 → state destroy.

## 5. The attacks

**Feather, T3#0xBC (`sub_80DCB1C`)**: spawner `sub_80DCC70` (PanelX/Y, Element, the damage word, alliance and
flip, flags |= 0x10). After the state: Param2 0 → `object_updateSpritePaused`, else
`object_updateSpriteTimestop`. Init `sub_80DCB4C`: coordinates from the panel, X −= front · 0xC0 px, Z = 0xC0
px; sprite 0c-67, no shadow, VISIBLE, anim 0, palette 0, flip. Update `sub_80DCB9C`: Param2 ≠ 1 → battle over:
destroy; dimmed: nothing. Action 0 (`sub_80DCBCC`): first tick (CurPhase 0) sound 0xC4, X velocity = front ·
`byte_80DCC4C[Param1]` (0xC0000), Z velocity the same, Timer = `byte_80DCC60[Param1]` (0x10; the tables' four
entries are equal); then X += vx, Z −= vz, Timer −= 1, at 0: the hit (`sub_80DCC96`: Param2 1 → `sub_80C53A6`,
else `object_spawnCollisionRegion`, whose flags then always skip the other: region 1, hit effect 7, target
5, self 0xA, modifier 3, Z 0), T4#0 effect 7 where it is, camera shake (2, 0xF), destroy. Every tick but the
last, Timer2 += 1 and the panel highlighted unless bit 2.

**Flame, T3#0xCD (`sub_80DF0A4`)**: spawner `0x080DF160` (as `sub_80DCC70`). Then
`object_updateSpriteTimestop`. Init `sub_80DF0C8`: coordinates, Z 8 px, sprite 0c-1c palette 4, VISIBLE,
collision (self 4, target 5, modifier 3; none → freed), hit effect 6, presented, Timer = Param1. Update
`sub_80DF132`: remove, the hit spark, a hit clears the region; Timer −= 1: > 0 present, else clear the region
and destroy (byte).

**Whirlwind, T3#0xCE (`sub_80DF188`)**: spawner `sub_80DF262`. Then `object_updateSpriteTimestop`. Init
`sub_80DF1AC`: coordinates, Z 0, X += front · 0x28 px; sprite 10-44, VISIBLE. Action 0 (`sub_80DF218`): first
tick Timer = Param1, ExtraVars = Param1 / 3, +4 = 0, Timer2 = 0; each tick Timer2 −= 1, not > 0 and fewer
than 3 blows: blow n (`sub_80DF288`: `off_80DF2CC[n]`'s three entries {dx, on, r4}: `sub_80C53A6(PanelX +
front · dx, PanelY, Element, z 0, r4, r6 = the damage word, r7 = 3)`), Timer2 = ExtraVars, +4 += 1; Timer −= 1,
not > 0 → destroy. Blow 0: dx 0 and 1 (region 1), dx 2 (region 4); blow 1: dx 1, dx 2; blow 2: dx 2; hit effect
6, target 5, self 4.

## 6. What the port changes and what is unverified

- The Japanese ROMs' `object_spawnCollisionRegion` also zeroes its hitbox's ExtraVars+0x14, which the US one
  leaves; nothing the US hitbox does reads it (the JP audit, docs/engine/jp-differences.md, owns the rest).
- The picker's unchecked second entry of the whole-area list (unreachable) is an error naming it.
- **Unverified, all of it**: no Japanese console recording exists yet (bn6battle-verify's JP tracing is in
  progress). The in-repo tests (crates/nettai-battle/src/kinds/player/actions/beast_chips_tests.rs) check the timings,
  positions, damage and hits above on the port. Also unverified once tracing exists: a cut-in on either chip
  (`object_drawChipName`'s window, and `sub_800BCF6`'s way back to the dim), the battle flag 0x40 HUD mask,
  the feather's Param2 0 branches (no chip spawns one), a summon or flame spawn failing.
