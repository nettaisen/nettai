# Standard chip actions

The chip actions (`JumpTable80EAC60`, action ≥ 0x10) that neither fire the buster's projectile (attack #0) nor
dim the screen, and the objects they spawn. Each is content (docs/design/content-model-v2.md §5.8): a builder
and its kinds by family, each chip a definition composing them, the shared objects under `objects/`. This
document describes what the original does; the scripts are the port.

Conventions (chips.md §3.3):

- `av` is the attack's variables: `av[0]`/`av[1]` the phase and whether its entry ran (the scripts' `step` and
  `step_init`), `av[2]` the element, `av[3]` the variant (the chip's subtype), `av.u16[6]` the Atk+ bonus
  (`extra`), `av.u32[8]` the damage word (damage, and the hit parameter in its high half), `av[0xC..0xF]` the
  chip's parameters, `av.u16[0x10]` the usual phase timer.
- A "damage word" passed to a spawned object is `av.u32[8] + av.u16[6]`: the damage with the Atk+ bonus, the hit
  parameter above it (`sub_801155A` stores it as the object's damage and stamina halves).
- A timer phase `ldrh; sub #1; strh; bge` runs one tick more than its start value: it ends on the tick it goes
  below zero.
- Spawners that call `object_spawnType3` with the caller's registers leave those registers as the object's
  position until its init places it; the scripts spawn with the same values.

**Verification.** The chip lab (bn6battle-verify, tools/chiplab) records every chip in the scenarios `hit`, `miss`,
`adjacent`, `atk10`, `counter`, `guard`, `obstacle`, the stages, Beast Out and Cross Charge. "Matched" below means
the engine matches the original on every frame of the scenario; scenarios that stop at another group's or the
framework's gap are listed with it. Branches no scenario reaches are marked **[unverified]**.

| action | handler | chips | objects | module |
|---|---|---|---|---|
| 0x18 | `sub_80EC02A` | 0x13 YoYo, 0x154 GreatYo | attack #0x52 yoyo, effect #0x70 GreatYo's controller | chips/yoyo (yoyo, throw), chips/greatyo |
| 0x1F | `sub_80EC7A6` | 0x1E Thunder, 0x11F DarkThnd | attack #0x2A thunder ball | chips/thunder (ball, shoot), chips/darkthnd |
| 0x22 | `sub_80EC960` | 0x59 CrakShot, 0x5A DublShot, 0x5B TrplShot | attack #0x33 crack shot | chips/crakshot (chips, shot) |
| 0x23 | `sub_80ECA34` | 0xBE CopyDmg | attack #0x28 copy mark | chips/copydmg (chip, mark) |
| 0x26 | `sub_80ECCB0` | 0x32 AirHocky, 0x155 PitHocky | attack #0x2E puck (and PitHocky's afterimages, effect #0x28) | chips/airhocky (puck, flick), chips/pithocky |
| 0x27 | `sub_80ECD28` | 0x14..0x16 FireBrn, 0x143..0x145 WideBrn | attack #0x04 flame | lib/burner (burn, flame), chips/firebrn, chips/widebrn, HeatCross's charge |
| 0x28 | `sub_80ECDFC` | 0x18..0x1A TrnArrw | attack #0xB flying shot (kind 2) | chips/trnarrw, objects/flying_shot (its arrow variant) |
| 0x2C | `sub_80ED25C` | 0x7B..0x7D IronShl, 0x14D ParaShl | attack #0x34 iron shell | chips/ironshl (shell, throw), chips/parashl |
| 0x2D | `sub_80ED2F8` | 0x1B..0x1D BblStar | attack #0x42 bubble star | chips/bblstar (chips, star) |
| 0x2E | `sub_80ED374` | 0x33 DrilArm | attack #0x71 drill | chips/drilarm (chip, drill) |
| 0x2F | `sub_80ED454` | 0x34 Tornado, 0x35 Static | attack #0x31 tornado | chips/tornado (chips, tornado) |
| 0x31 | `sub_80ED64C` | 0x5C..0x5E WaveArm, 0x149..0x14B PwrWave | attack #0x16 shock wave | chips/wavearm (wave, strike), chips/pwrwave |
| 0x32 | `sub_80ED6E6` | 0x3D..0x3F AquaNdl | effect #0x40 needle volley, attack #0x50 aqua needle | chips/aquandl (action, volley, needle) |
| 0x34 | `sub_80ED810` | 0x152 H-Burst | attack #0x9E hyper burst | chips/h-burst (action, burst) |
| 0x36 | `sub_80ED9AE` | 0x28..0x2A RlngLog | attack #0x66 rolling log | chips/rlnglog (action, log) |
| 0x38 | `sub_80EDCC0` | 0x7E..0x80 AirSpin | attack #0x9B whirlwind, attack #0xD4 seeking whirlwind | chips/airspin (action, top, whirl) |
| 0x3E | `sub_80EE0BC` | 0x1F..0x21 DolThdr | attack #0x82 thunder doll, attack #0x8B thunder column | chips/dolthdr (action, doll, column) |
| 0x3F | `sub_80EE192` | 0x50 WindRack | attack #0x49 gust | chips/windrack (action), objects/gust |
| 0x40 | `sub_80EE2A0` | 0x54 MoonBld | attack #0x85 moon blade | chips/moonbld (action, blade) |
| 0x42 | `sub_80EE55E` | 0x22..0x24 ElcPuls, 0x14E DestPuls | attack #0x8C electric pulse | chips/elcpuls (action, pulse) |
| 0x43 | `sub_80EE634` | 0x5F..0x61 AuraHed, 0x150 StreamHd | attack #0x92 aura head | chips/aurahed (action, head) |
| 0x44 | `sub_80EE6FC` | 0xB7 MagCoil | attack #0x95 magnet | chips/magcoil (chip, magnet) |
| 0x53 | `sub_80EF62E` | 0x52 VarSwrd | (hands off to the chosen sword) | chips/varswrd, lib/swords/vari |
| 0x54 | `sub_80EF7E2` | 0x53 NeoVari | (hands off to the chosen sword) | chips/neovari, lib/swords/vari |

Not yet content: 0x55 (SonicBom, Curse, Punisher, SprSonic, `sub_80EF970`) and 0x5B (Z Saver, `sub_80EFEE0`), which
share the sonic boom (attack #0x58, `sub_80CF7F0`), specified in shot-chips.md with the actions that fire a shot;
and 0x0A, the link navis' chips (below).

Shared helpers: content/exelib/panels.luau (`GetRandomRelativePanelFiltered`, `sub_8109708`), lib/arm.luau (`sub_80EBAE8`,
the arm a chip attachment shows), lib/swords/vari.luau, and group G1's lib/region.luau and lib/trajectory.luau.
(v1's lib/object_setup.luau, `sub_8011504` with the collision-panel highlight and the dust puff, went when its
last users became definitions that set themselves up.)

## The other actions, in brief

- **0x18 YoYo, GreatYo.** The yoyo rolls out a panel every 8 ticks, spins, and rolls back; the navi waits until
  its related1 is cleared. GreatYo's controller (effect #0x70) holds three, which all start spinning when one hits
  (`sub_80E8612`). The game keeps a pointer to the slot holding a yoyo; the port names the slot (the navi's
  related1, or one of the controller's three).
- **0x1F Thunder, DarkThnd.** The buster arm comes up and a thunder ball is placed ahead on the first tick; it homes
  toward the nearest enemy column a panel at a time for Param2 panels, hits with status Param3 (paralysis), and
  bursts.
- **0x23 CopyDmg.** The buster arm, a shot on the first tick, 5 ticks then 11 of recovery. The mark (spawned at the
  end of the update list, `sub_8003374`) flies a panel a tick; on a player it marks that navi for 180 ticks through
  the damage-carry record (`dword_203CFB0`, by the victim's side) and sits at its attach point 0x1B. Its state 0xC
  is the lifecycle value `"finish"`.
- **0x26 AirHocky, PitHocky.** The puck moves diagonally a panel every 6 ticks, bouncing off the field's edges and,
  once across, off its own side's panels; it lasts its kind's number of steps and bursts off solid ground or at the
  battle's end. PitHocky's leaves afterimages.
- **0x27 FireBrn, WideBrn.** The navi holds a burner and lights a flame ahead: it burns Param1 ticks and spreads
  forward on its 4th tick (WideBrn over three rows); with Param3 it cracks its panel when it goes out.
- **0x28 TrnArrw.** Arrows (flying shot kind 2) over each open panel ahead with delays 16, 12, 8, 4, 0, and on
  tick 24 one more from the navi's panel.
- **0x2C IronShl, ParaShl.** The shell slides to the opponent's back column, stops pushing there (hit modifier 1)
  and stands for Param2 rounds; IronShl's then flies up, ParaShl's turns back. ParaShl throws three down the user's
  own back column.
- **0x2D BblStar.** The star drifts forward (speed by Param1), swaying ±24 pixels along the sine table; what it hits
  is bubbled (status 0x60); a hit with flag 0x800000 bursts it. The bubble around a bubbled navi is effect #0x3C
  (Rust, kinds/bubble_visual.rs).
- **0x2E DrilArm.** After 10 ticks the drill arm comes out and two unseen drills grind the two panels ahead, a hit
  every 12 ticks, until the slot holding them lets go.
- **0x2F Tornado, Static.** A fan (attachment 0x21 or 0x22), tornadoes two panels ahead: subtype 1 one; Static's
  spread grows with the navi's NaviCust bug kinds (`sub_800FE52`). On volcano, grass or ice a plain tornado takes
  the panel's element, turns it normal and doubles its damage.
- **0x31 WaveArm, PwrWave.** The navi strikes the ground; on timer 0xD a shock wave starts on the panel ahead
  (PwrWave also from its own column one row up and down); 32 ticks. The wave's look, ticks and panel mark come from
  its variant (`byte_80C6B00`, the variant records in chips/wavearm/wave.luau); WaveArm's variants 0xC..0xE
  steer toward an enemy's row.
- **0x32 AquaNdl.** The volley picks three targets among the enemies alive at the start (viruses first, then
  navis; with two, the third is the one with more HP, a tie drawn at random), farthest first, and drops a needle
  every 8 ticks: 12 ticks down, one tick hitting, 12 blinking out.
- **0x34 H-Burst.** A cannon (attachment 0x15), a shot on tick 2 that moves a panel a tick; on its first hit it
  bursts 10 times, every 9 ticks, alternating regions 9 and 0xB, with effect 0x3D on each panel.
- **0x36 RlngLog.** Two logs 13 ticks apart, rows from a table by the navi's row; each arcs onto the panel ahead and
  rolls, with Param3 HP; a damaging hit stops it for Param2 ticks; it bursts on a body, a neutral object, at the
  battle's end or with no ground under it.
- **0x38 AirSpin.** 16 ticks unpushable, then 6 of recovery. The whirlwind (variant 0) is a field object with 400
  HP that rolls up to 4 panels, stands, and hits its panel every 10 ticks; variant 1's seeking whirlwind swoops in
  from behind (no chip uses it).
- **0x3E DolThdr.** Attachment 0x29; after 10 ticks the doll appears one panel back at attach point 0x1D and strikes
  unseen thunder columns (16 pixels up) on every panel to the field's edge, each for Param2 ticks or until it hits.
- **0x3F WindRack.** The rack (attachment 0x2A, 0x33 if variant ≠ 0) and a swirl (effect 0x44); at timer 0xC a
  hit on the column ahead for 10 ticks and an unseen gust down each row; 22 ticks, then 6.
- **0x40 MoonBld.** The blade rides the navi and sweeps the 8 panels around it, one a tick, each highlighted with a
  hit (hit modifier 3, bug 0x18 with argument 1: `dword_80D5D50` is 0x01180003, whose bytes are the hit
  modifier, the status, the bug and its argument; no status); 10 ticks. Its spawner leaves register garbage (scratch_position).
- **0x42 ElcPuls, DestPuls.** Attachment 0x2B; a tick later the pulse from the panel ahead, held p3 − 5 ticks, region
  0x15 pulling (self type 0xB); looks by Param4 (2 and 3 carry bugs).
- **0x43 AuraHed, StreamHd.** A head fires along the row while the navi recoils for 31 ticks (StreamHd five, 16
  ticks apart); it hits once per panel, and its first hit shortens its run.
- **0x44 MagCoil.** Attachment 0x2C; the magnet one panel back for Param1 ticks, every 15 ticks zero-damage pulls
  on enemies among the three panels ahead in the rows above and below.
- **0x53 VarSwrd, 0x54 NeoVari.** Hold A and enter a direction sequence (mirrored by flip) to pick a sword; letting
  go or 48 ticks gives Sword; Beast Over or a special source picks at random. The pick becomes the attack (its chip,
  subtype, params, `set_attack(action, 0)`), keeping the damage.

Verified in the lab (fully matched, before → after): 0x18 0 → 14/20; 0x1F 0 → 14/30; 0x22 0 → 24/39; 0x23 0 → 13/18;
0x26 0 → 12/20; 0x27 0 → 20/30; 0x28 0 → 19/27; 0x2C 0 → 18/42; 0x2D 0 → 19/41; 0x2E 0 → 12/19; 0x2F 0 → 15/29;
0x31 0 → 24/42; 0x32 0 → 19/41; 0x36 0 → 20/27; 0x38 0 → 18/39; 0x3E 0 → 22/27; 0x3F 0 → 12/19; 0x40 0 → 12/19;
0x42 0 → 24/42; 0x43 0 → 18/42; 0x44 0 → 13/18. The rest stop at other groups' or the framework's gaps; VarSwrd
and NeoVari reach the plain Sword (action 0x13, G1's) and stop there; H-Burst's scenarios stop at the Program
Advance banner.

**[unverified]** (ported, reached by no scenario): the shock wave's virus variants 0..0xB and their panel marks;
the needle volley with three or more enemies, the two-enemy HP tie, and none; the aqua needle's crack; the pulse's
looks 2 and 3; the aura head's speed tables past 1; the thunder doll's Param1 ≠ 0 path and the thunder column's
kind 1; ParaShl, GreatYo's controller and PitHocky (Program Advances, stopped at the banner); Tornado subtype 3 (no chip has it:
completeness.md §5) and Static's larger spreads; the drill without a slot; AirSpin variant 1 and the whirlwind's removed, absorbed and
blink-out paths; WideBrn's spread and flame looks 1 and 2; RlngLog's drop-in path and the log's stop and break;
DarkThnd and the thunder ball's bug; MoonBld's repeat swings; WindRack variant ≠ 0 and the gust's other
spawners; H-Burst's whole-field branch; every "pool full" path. Off-table parameters are explicit errors naming
the routine.

The coverage scenarios (docs/engine/unverified.md) verified since: every VarSwrd and NeoVari sequence, entered
with A held (`chips/0x052-varswrd/cmd-longswrd`, `cmd-ftrsword`, `cmd-wideswrd`, `cmd-lifesrd`, `cmd-sonicbom`;
`chips/0x053-neovari/cmd-crosswrd`, `cmd-sprsonic`, `cmd-dbldream`), by side 1 (the mirrored sequences), in
Beast Out, slowly, too slowly, wrong, broken off, and not at all (the 48-tick timeout); Static's spread with one,
two and three NaviCust bug kinds (`chips/0x035-static/bugs-1` to `bugs-3`); CopyDmg's time-up path and an
obstacle as its target (`chips/0x0be-copydmg/time-up`, `on-rock`, `rock-hit`); a log shot at and a log over a
hole (`chips/0x028-rlnglog1/shot`, `holes`); the whirlwind shot down and pushed (`chips/0x07e-airspin1/shot`,
`pushed`); H-Burst's fight-over branch (`pa/0x152-h-burst/ko`); and, for the first chip of each of these actions,
the battle's end under it, a barrier, an invisible opponent and a dimming cutting in (`ko`, `barrier`,
`invisible`, `dimmed`).

## Action 0x22: CrakShot, DublShot, TrplShot (`sub_80EC960`)

The navi digs into the panels ahead and flings them. Two phases (`off_80EC974`):

- **Dig** (`sub_80EC97C`). Entry: animation 0xC, sound 0xD8, the counter window, `using_action`, timer 0xF. On the
  tick the timer reads 0xD (the third tick) it spawns a crack shot on every panel of a hit region
  (`PanelOffsetListsPointerTable`; content's region definitions) chosen by the variant (`dword_80ECA08`: CrakShot
  region 1, the panel; DublShot 2, two in a row; TrplShot 4, three in a column), taken from the panel ahead, dx
  toward the front. Each gets the element, the damage word and Param1 5. The timer then runs out (16 ticks in all).
- **Recover** (`sub_80ECA0C`): timer 5, then `object_exitAttackState` (6 ticks).

No reactive-defense check (`sub_801056A`) runs. 22 ticks from the action's first tick to idle.

### The crack shot (attack #0x33, `sub_80CA544`)

- **Init** (`sub_80CA568`): on its panel's center; dust (effect #0 id 0x30) at its position. A panel that is not
  solid (flag 0x10) or has anything on it (0x0F880080) is cracked instead, unless already cracked
  (`sub_8109794`), and the shot frees itself: nothing flies. Otherwise the panel breaks (`object_breakPanel`, sound
  0x97), sound 0xDA, sprite 0C-33 with a ground shadow, x velocity 8 px a tick toward the front, z velocity 8 px a
  tick, timer 2, collision (self type 4, target 5, hit modifier 1, hit spark 5) present, and its first update runs.
- **Update** (`sub_80CA61A`): resolve hits and show the spark; after a hit it ends. Below 20 px high it rises;
  then it flies forward, and every time it passes a panel's center (`sub_800E6E8`) Param1 counts down: at 0 it
  ends. It also ends off the field. Ending: invisible, region cleared, destroy state; the collision is presented
  every tick, ending or not.

Matched: every scenario of the three chips except those stopped elsewhere (`counter` by action 0x12, `guard` by
action 0x2B, `obstacle` by dimming subtype 6, `beast` and `beast-charged` by the empty hand's charge threshold).

## Action 0x0A: the link navis' chips

Chips 0x190 HeatPres to 0x199 DustBrk have action 0x0A, below 0x10, so they run through the user's own action table
(`off_80EA4C8[AIIndex]`, chips.md §1.6), not `JumpTable80EAC60`. `off_80EA4C8` has 25 tables by AI index: 0 and
0xC..0x18 (MegaMan and his forms) share `off_80EA52C` and AI index 0xB has `off_80EA710`, nine entries each (actions
0..8), without an entry 0xA (it would read on into the next table: for MegaMan `off_80EA550[1]`, `sub_8017888`; the
port should refuse it); the link navis' tables have eleven, and DustMan's thirteen (0xB `sub_80F20F8` and 0xC
`sub_80F2274`, his other moves; entry 9 of each link navi's table is another move of his, not a chip). Entry 0xA: 1 HeatMan `sub_80F0778`, 2 ElecMan
`sub_80F09B8`, 3 SlashMan `sub_80F0CB0`, 4 EraseMan `sub_80F1056`, 5 ChargeMan `sub_80F1334`, 6 SpoutMan
`sub_80F15CE`, 7 TomahawkMan `sub_80F18AC`, 8 TenguMan `sub_80F1A46`, 9 GroundMan `sub_80F1C1C`, 10 DustMan
`sub_80F1FA0`.

The port doesn't dispatch on the AI index: each chip's record runs its own action, the one the module it names
exports (navis/<navi>/chip.luau; docs/design/content-model-v2.md §12, "A record's action by its module"), and which
navi has which chip is the navi's `own_chip`. A navi the original gives no entry 0xA (MegaMan, navi 0xB) never
holds one of these chips, so the read past its table has no counterpart. What the routines share (the phases on the
attack's step, the timer, the Atk+ bonus, the damage word, `sub_800E258`, `sub_800E708`) is lib/link_chips.luau.

Each routine runs its phases by `av[0]` (entry on `av[1]` 0, set to 4), keeps its timer in `av.u16[0x10]`
("n ticks": after the entry tick until the timer reaches 0, `bgt`), and in its first attacking phase adds the Atk+
bonus to the damage (`av.u16[8] += av.u16[6]`); "the damage word" below is `av.u32[8]` after that. Every chip has
subtype 3 and params 0.

- **HeatPres (AI 1, `sub_80F0778`, phases `off_80F078C`).** 0 (`sub_80F0798`), the jump: his collision region
  cleared, FuturePanel = his panel, reserved (`object_reservePanel`), anim 0x12; an arc (`sub_8001330`: from his X,
  Y, Z to the center of the panel three ahead at Z 0 in 20 ticks with gravity −0x18000: the angle, the distance
  `SWI_Sqrt`ed and divided by the ticks for the speed, `sub_80011A0` for X and Y, the Z velocity ((0 − Z) − 20 · 20 ·
  g / 2) / 20; lib/trajectory's `in_ticks`) into his velocity; Timer 20. Each later tick while the timer, counted down,
  stays ≥ 0: X, Y, Z += velocity, Z velocity −= 0x18000, his panel from the coordinates; then Z's whole part 0 and
  his panel solid (0x10) → 4, else 8. 4 (`sub_80F0832`), the press: the bonus; four flames
  (`sub_80F37D8`: on each of the panels left, right, above and below him (`byte_80F3824`: (−1, 0), (1, 0), (0, −1),
  (0, 1)) that is solid, attack #0x26, chips/heatman/flame, Param1 0 (it waits a dimming out), Param2 60, the damage
  word; its Z the r3 `_object_getPanelDataOffset` leaves: the row − 1); a hit on his panel lasting 60
  (`object_spawnCollisionRegion`: region 1, hit effect 1, target 5, self 4, modifier 3, Z 0, Timer 60); sound 0xC0;
  60 ticks → 8. 8 (`sub_80F0898`): anim 0, region 1, back onto FuturePanel (unreserved), `object_exitAttackState`.
- **DElecSwd (AI 2, `sub_80F09B8`).** 0 (`sub_80F09DC`), the slide: the bonus; anim 0x10; FuturePanel his, reserved;
  ObjectFlags1 |= 0x40 (moving) and 0x400000 (using action); X velocity front · 0x50000; his glow (`sub_80E3FB4`:
  effect #0x31 at his panel's center, r4 = 7 | side << 8; RelatedObject1 him) in `av+0x30`; the stop check
  (`sub_80F0BD2`) at once. Each later tick: c = his panel's center X; X += velocity; `sub_800E708(X, the X
  velocity, c)` (the game passes the velocity where the old X belongs: true while c ≤ X); true → the stop check.
  Stopping: his panel from the coordinates → 4; else the panel and collision panels follow. The stop check:
  the next panel (x + front) holds another body of either side or a neutral object (0x03800000) → stop; the one
  after that isn't on the field (no 0x10000) → stop; else `GetRandomRelativePanelFiltered` (content/exelib/panels:
  `random_in_region`) on region 4 around the next panel, dx toward his side's front, for an enemy navi's body
  (`off_80F0C24`): found (**one RNG draw**) → stop. 4 (`sub_80F0A7A`): anim 0x11, Timer 20; a hit on the panel ahead
  (`sub_80F0B80`, `object_getEnemyDirection`: region 4, hit effect 3, target 5, self 7, r7 0x1001: modifier 1,
  status 0x10; Z 0), T4#0 effect 0x16 at the panel ahead's center, 16 pixels up (his flip, palette + 3), sound 0xB0;
  the glow's CurAnim = 0x10; 20 ticks → 8. 8 (`sub_80F0AB6`): anim 3, back onto FuturePanel (unreserved), the
  coordinates and collision panels; flags: 0x40 off, 0x80000 (move complete) on, 0x400000 off; Timer 3; the glow's
  state word = 8 (`sub_80E3FC4`); 3 ticks → 0xC. 0xC (`sub_80F0B0E`): anim 0, 30 ticks → exit.
- **RSlash (AI 3, `sub_80F0CB0`).** 0 (`sub_80F0CD4`): the bonus; `av+0x30` = 0; speed `av+0x34` = 0x40000; anim 0x12;
  20 ticks → 4. 4 (`sub_80F0D08`): region cleared; FuturePanel his, reserved; flag 0x40; anim 0x13; the riding hit
  (`sub_80D19D4`: attack #0x69, navis/slashman/riding_hit: self 7, target 5, Param1 3 (modifier), Param2 0xFF (no spark),
  Param3 1 (goes on after hitting a body), the element and damage word; it lasts while he is in action 0xA) in
  `av+0x30`; sound 0x164; 10 ticks: the heading (`sub_80F0E96`: on row 3 X velocity front · speed and route step
  `av[0xC]` = 1; else Y velocity + speed (down), step 0) → 8. 8 (`sub_80F0D64`), the roll: c = his panel's center on
  the axis (step odd: X; even: Y); X, Y += velocity; the panel under him (`sub_800E258`) must meet require 0x10010,
  forbid 0x800000 (a valid solid panel, no neutral object), else → 0xC; when the old coordinate wasn't c and he
  reached or passed c (`sub_800E708`): snapped to its center, the route (`sub_80F0EC0`) — ended → 0xC; then his panel
  and collision panels follow. 0xC (`sub_80F0DEC`): anim 7, region 1, back onto FuturePanel (unreserved), flag 0x40
  off, Timer 20, the riding hit's state word = 8 (`sub_80D1A00`); 20 ticks → exit.
  The route, `byte_80F0F50[side][step]` (target x, target y, dx, dy): side 0: (0, 3, +1, 0), (6, 0, 0, −1), (0, 1,
  −1, 0), (1, 0, 0, +1), (0, 0, 0, 0); side 1: (0, 3, −1, 0), (1, 0, 0, −1), (0, 1, +1, 0), (6, 0, 0, +1), (0, 0, 0,
  0). Step 1 turns when an enemy navi's body is in his column (`sub_80F0F78`, `object_getPanelsInColumnFiltered`) or
  he is at the target column; steps 0 and 2 turn at the target column (a non-zero target x) or row; step 3 (back
  along row 1): at FuturePanel's column it ends if at its row too, else turns; step 4 (down that column): at its
  row it ends. A turn sets the velocity to (dx, dy) · speed and adds 1 to the step (a word store over `av+0xC`).
  So: down to row 3, forward until an enemy is in his column (or the far column), up to row 1, back to his column,
  down to his row.
- **EDeletBm (AI 4, `sub_80F1056`).** 0 (`sub_80F1074`): the bonus; anim 0x11; 9 ticks → 4. 4 (`sub_80F10A0`): anim
  0x12; the beams (`sub_80F1148`: on the panels 1..5 ahead in his row, stopping at the field's edge, attack #0xC3,
  chips/eraseman/beam, Param1 1 (aimed straight), Param2 60, Param3 1 (it ends with his action 0xA, and stands still
  while dimmed), the damage word); sound 0xBA; 60 ticks → 8. 8 (`sub_80F10CE`): 30 ticks → anim 0, exit.
- **VolcChrg (AI 5, `sub_80F1334`).** 0 (`sub_80F1350`): the bonus; anim 6; Timer 30; the tick it reaches 11, the
  volcano (`sub_80F13B4`) and sound 0x146; 30 ticks → 4. 4 (`sub_80F1390`): 30 ticks → anim 0, exit. The volcano: a
  list (`object_getPanelsExceptCurrentFiltered`, rows 3..1, columns 6..1) of the panels with an enemy navi's body on
  the other side's area (`off_80F1434[side]`: side 0 require 0x04000020; side 1 require 0x08000000, forbid 0x20),
  then those of the other side's area without one (`byte_80F1448[side]`: side 0 require 0x20, forbid 0x04000000;
  side 1 forbid 0x08000020), the latter shuffled when there are any (`sub_8000C72(list, n, n)`: n swaps, **two RNG
  draws each**, content/exelib/panels' `shuffle`), then (x, 1), (x, 2), (x, 3) with x = `dword_80F145C[side]` (7; side 1: 0, off
  the field); the first five get a rock (`sub_80F1460` → `sub_80D5EB0`: attack #0x86, FuturePanel the target, on
  his panel, element, Param1 = the subtype, the damage word, flags |= 0x10).
  **The volcano rock, T3 0x86 (`sub_80D5D54`)**: init: on his panel 10 pixels ahead, Z's whole part 48, sprite (0x10,
  0x55), a ground shadow, VISIBLE, anim 0, palette 0, flip; its sprite as `object_updateSpritePaused`. Each tick: the
  battle over → state 8 (genericDestroy); dimmed → nothing; action 0: entry: the arc to its target's center, Z 0, in
  45 ticks with gravity −0x4000 (`sub_8001330`), Timer 45, Timer2 0; each later tick while the timer, counted down,
  stays ≥ 0: Timer2 + 1 (the target highlighted while its bit 2 is clear), the move, Z velocity −0x4000, its panel
  from the coordinates; then, if its panel has any of `byte_80D5EA0[side]` (0x15800010; side 1 0x2A800010), sound
  SOUND_HIT_BOMB_1, T4#0 effect 0 at it and a hit lasting 5 (`sub_80D5EDE`: region 1, hit effect 1, target 5, self
  0xA, modifier 3); either way state 8.
- **DripShwr (AI 6, `sub_80F15CE`, phases 0..0x18).** 0 (`sub_80F15FC`): anim 0x11, Timer 35, anim 0x12 at 30; 35
  ticks → 4. 4 (`sub_80F162C`, one tick): if he can move (`object_canMove`) and the panel three ahead meets require
  0x10, forbid 0x0F880080: it (into `av+0x16`, `av+0x17`) and his own panel (FuturePanel) reserved, region cleared,
  flag 0x40, Timer 2 → 8; else Timer 2 → 0xC. 8 (`sub_80F1694`): 2 ticks: onto the reserved panel, coordinates and
  collision panels → 0x10. 0xC (`sub_80F16B4`), no jump: the timer counts before the entry: 2 ticks, then anim 3,
  Timer 8, and 8 ticks → exit. 0x10 (`sub_80F16E0`): the bonus (here: the no-jump path never adds it); the spray
  (`sub_80C92CC`: attack #0x29, navis/spoutman/drip_shower, on his panel, Param1 4, Param2 2, Param3 0xA, the element and
  damage word); Timer 0xA · 4 · 2 + 0xA = 90 → 0x14. 0x14 (`sub_80F172C`): anim 4, flag 0x40, 6 ticks → 0x18. 0x18
  (`sub_80F1756`): anim 3; his panel unreserved, back onto FuturePanel (unreserved), coordinates, collision panels,
  region 1, flags 0x40 off, 0x80000 on, 0x400000 off; 8 ticks → exit.
- **ETomahwk (AI 7, `sub_80F18AC`).** 0 (`sub_80F18CC`): the bonus; anim 0x11; his axe (`sub_80E40C2`: effect #0x32,
  navis/tomahawkman/axe, r4 1, kept in `av+0x30`); 16 ticks → 4. 4 (`sub_80F1902`): anim 0x12, Timer 30, the axe's
  CurAction = 1; the tick it reaches 20: a camera shake (2, 30), sound 0x10C, the strikes (`sub_80F197A`: on the
  panels 1..6 ahead in his row, stopping at the edge, attack #0x6A, navis/tomahawkman/strike, Param1 (n − 1) · 5 (the
  delay), Param2 7, Param3 5, Param4 0xFF, the element and damage word); 30 ticks → 8. 8 (`sub_80F194C`): anim 7,
  the axe's CurAction = 2, 30 ticks → exit.
- **FTornado (AI 8, `sub_80F1A46`).** 0 (`sub_80F1A60`): anim 0x10; the bonus; a tornado (`sub_80D05C4`: attack
  #0x60, navis/tenguman/tornado, the element and damage word) on every panel ahead of him (rows 1..3, columns 1..6,
  columns beyond his toward the front) with the other side's body or object or a neutral object
  (`byte_80F1B70[side]`: 0x05800000; side 1 0x0A800000), in that order; a whole-field hit of no damage
  (`sub_80F1B78`: region 0x80, no spark, target 2, self 1, modifier 0, element 0, at (0, 0)); sound 0xB8; T4#0 effect
  0x41 at his panel's center, 32 pixels up (his flip); 30 ticks → 4. 4 (`sub_80F1AB6`): anim 7, 20 ticks → anim 0,
  exit.
- **RC Brakr (AI 9, `sub_80F1C1C`, phases 0..0x14).** 0 (`sub_80F1C48`): the bonus; rocks to drop `av[0xC]` = 9; anim
  0x11; 12 ticks → 4. 4 (`sub_80F1C78`), the dig: anim 0x12; FuturePanel his, reserved; flags 0x40 and 0x400000;
  invulnerable (`object_setInvulnerableTime(0xFFFF)`: flag 8); `sub_80F1E50(0x40000)`: X velocity front · 0x40000,
  Timer = |c − x| · 0x280000 / 0x40000 (c 6; side 1: 1); sound 0x1C0; the drill (`sub_80E7896`, r4 0xA00: T4 0x61,
  chips/grndman/drill, Param1 0, Param2 0xA: it goes once he leaves action 0xA; chips.md §3.6.25) in `av+0x30`.
  Each later tick: X += velocity; the panel under him (`sub_800E258`); a new column → a hit there (`sub_80F1E7E`: region 1, hit effect
  0xA, target 5, self 6, r7 0x63: modifier 0x63; Z 0); that panel against `byte_80F1D20[side]` (require 0x10, forbid
  0x07800000; side 1 0x0B800000): refused → 0xC; else his panel and collision panels follow, the timer counts, and
  at 0 sound 0xE5 → 8. 8 (`sub_80F1D30`), the rockfall: a rock (`sub_80F1E98`: of the panels meeting
  `off_80F1EF0[side]` (GroundMan's: side 0 require 0x04000020, side 1 require 0x08000000 forbid 0x20) but his own
  (`object_getPanelsExceptCurrentFiltered`), one at random (**one draw**, `GetPositiveSignedRNG2()` mod n; none: no
  rock), attack #0x80 (chips/grndman/rock, chips.md §3.6.25) with Param1 10, Param2 0 (it waits a dimming out), Param3 0 (no crack),
  Param4 0), a camera shake (1, 20), Timer 20; 20 ticks: `av[0xC]` − 1, above 0 the entry again, else → 0xC. Nine
  rocks, 20 ticks apart. 0xC (`sub_80F1D6C`): anim 4, Timer 4, the drill's state word = 8 (`sub_80E78AE`); 4 ticks →
  0x10. 0x10 (`sub_80F1D9A`): back onto FuturePanel (unreserved), coordinates, collision panels; flags 0x40 and
  0x400000 off; invulnerability off (`sub_800EB08`: its timer 0, flag 8 off); anim 3, 9 ticks → 0x14. 0x14
  (`sub_80F1DE4`): anim 0, 20 ticks → exit.
- **DustBrk (AI 10, `sub_80F1FA0`).** 0 (`sub_80F1FC0`): the bonus; anim 0x12; the first dust cloud (`sub_80E887C`, r4
  0xA00: effect #0x72, navis/dustman/clouds) in `av+0x30`; sound 0xAD; Timer 30; every tick, the entry's too, the pull
  (`sub_80F20A0`: in his row, on each panel of the other side's area (`byte_80F20E0[side]`: side 0 require 0x20;
  side 1 forbid 0x20), a hit of no damage, element 0, region 1, no spark, target 5, self 0x1E, modifier 0x10); 30
  ticks → 4. 4 (`sub_80F2004`): anim 0x14; the second cloud (`sub_80E8770`, r4 0xA: effect #0x71) in `av+0x34`, Timer
  40; the first cloud's state word = 8 (`sub_80E8894`); the tick it reaches 30, a hit on the panel ahead (region 1,
  hit effect 0xA, target 5, self 6, modifier 3, the element and damage word) and sound 0x17B; 40 ticks → 8. 8
  (`sub_80F204E`): 30 ticks → anim 0, exit, and only then the second cloud's state word = 8 (`sub_80E8788`).


As ported (content model v2; every routine branch by branch, each chip's record naming its navi's module):

| Chip | Action | Module | Kinds it spawns |
|---|---|---|---|
| HeatPres | `heatpres/action` | navis/heatman/chip.luau | chips/heatman/flame (`heatman/flame`, HeatMan's own) |
| DElecSwd | `delecswd/action` | navis/elecman/chip.luau | objects/follow_effect (`follow-effect`: the glow, with DeltaRay's bursts) |
| RSlash | `rslash/action` | navis/slashman/chip.luau | navis/slashman/riding_hit.luau (`slashman/riding-hit`) |
| EDeletBm | `edeletbm/action` | navis/eraseman/chip.luau | chips/eraseman/beam (`eraseman/beam`, EraseMan's own) |
| VolcChrg | `volcchrg/action` | navis/chargeman/chip.luau | navis/chargeman/volcano_rock.luau (`chargeman/volcano-rock`) |
| DripShwr | `dripshwr/action` | navis/spoutman/chip.luau | navis/spoutman/drip_shower.luau (`spoutman/drip-shower`) |
| ETomahwk | `etomahwk/action` | navis/tomahawkman/chip.luau | navis/tomahawkman/axe.luau (`tomahawkman/axe`), strike.luau (`tomahawkman/strike`) |
| FTornado | `ftornado/action` | navis/tenguman/chip.luau | navis/tenguman/tornado.luau (`tenguman/tornado`) |
| RC Brakr | `rc-brakr/action` | navis/groundman/chip.luau | chips/grndman/drill (`grndman/drill`) and rock (`grndman/rock`), GroundMan's own |
| DustBrk | `dustbrk/action` | navis/dustman/chip.luau | navis/dustman/clouds.luau (`dustman/cloud`, `dustman/overlay`) |

The kinds that last "while he is in action 0xA" (the glow, the riding hit, the axe, the drill, the erase beams) keep
the action their owner was running when they came and compare it with his running action, instead of the number.
The link navi's chip bonus (`sub_800F09E`) and ChargeMan's charge limit read the side's link navi level
(`dword_203CFA0`, from each console's init block: 0xFF without one), which the recorder writes into a trace's setup
as `navi_levels`; the chip lab's link navis are at level 14. In nettai the level is EXE6's save part's fact, and
what it gives a link navi in battle is EXE6's rules' (rules/by_level.luau: the navi's `chip_bonus.damage`,
`charged_chips.when` and `fire_charge`, functions of its side, which the round's setup asks once, `Battle::given`).
The level also gives the navi its stats (HP, the buster's levels, abilities), not in the battle but in the save, as
the PET's reload does: link-navis.md.

Lab: the ten generated scenarios, navis/navi-01-heatpres to navi-10-dustbrk, match on every frame (the chip, the
buster, the charged attack twice and B+Back: "Action 9" below). 105 hand-written scenarios under navis/navi-NN-chip/ match on every
frame, and between them run every branch of the ten routines but those below: used by side 1, with the opponent
off his row, a row down or in the far column (`miss`, `far`, `edge`), from other columns and rows (`back`,
`forward`, `row1`, `row3`), over the holes stage's holes (`holes`), against a RockCube of his own or the
opponent's (`rock`), with both stepping forward first (`adjacent`), hit or paralyzed out of the action (`hit`,
`shocked`), with the field dimmed while it runs (`dimmed`, `dimmed-early`), and with the opponent deleted by it
(`delete`). Among them: HeatPres landing on a hole; DElecSwd's three stop reasons, on its entry tick too; RSlash
from each row, stopped by a hole and by a rock, and turning at the far column once the enemy has left it;
EDeletBm's five beams; a volcano rock on a hole; DripShwr's jump and spray; RC Brakr's dig to the far column and
its nine rocks, the dig stopped by a rock and by a hole, and the rockfall with no enemy left to aim at.

**Unverified** (no scenario reaches them; ported from the disassembly):

- every failed spawn (no free object or collision slot): HeatPres's hit and its sound, DElecSwd without its glow,
  RSlash without its riding hit, ETomahwk without its axe, RC Brakr without its drill, DustBrk without its clouds;
- DElecSwd's panel center equal to its velocity (no panel's is); RSlash's route past its last leg (the table has
  none);
- VolcChrg with no empty panel on the other side, or fewer than five targets in all (the other side's area is
  never that small without stolen columns);
- DripShwr when he can't move (immobilized as the wind-up ends); ETomahwk's six-strike cap (he would stand off the
  field);
- of the kinds: the riding hit when its owner leaves the action otherwise than by landing (he can't be hit while
  he rolls), its variant that ends on a hit, and its running off the field; the axe when its owner leaves the
  action (TomahawkMan is neither flinched nor paralyzed out of it) and its holder's word being null; the drill
  and DustBrk's clouds once their owner has left the action while the battle goes on are reached only through the
  paralysis scenarios (DustMan and GroundMan before the dig), not through a flinch.

## Action 9: the link navis' charged attacks

Entry 9 of a link navi's action table (`off_80EA4C8[AIIndex]`) is his charged attack. His weapon routine
(`off_80117D4`, the charged-shot routine of his stats: 0x40 HeatMan `sub_80121DC`, 0x41 SpoutMan `sub_80121F8`, 0x42
TenguMan `sub_8012210`, 0x43 SlashMan `sub_8012228`, 0x44 ElecMan `sub_8012240`, 0x45 TomahawkMan `sub_8012258`, 0x47
EraseMan `sub_8012292`, 0x48 ChargeMan `sub_80122AA`, 0x49 DustMan `sub_80122C2`, 0x4A GroundMan `sub_80122DA`) is
a setup: `set_attack(9)`, the damage `sub_8010D04(base, step)` = base + step · the buster Attack points (1 to 5,
`byte_80212D4`), the counter byte 0x8A, the attack's element byte (`av[2]`), and for TomahawkMan two parameter bytes
(the ticks he raises the axe and recovers for, 30 and 30). Routine 0x46 (`sub_8012278`), which no navi's stats
name, starts TomahawkMan's action with 10 damage and 6-tick waits. The actions run their phases as the chips' do
(by `av[0]`, the timer in `av.u16[0x10]`), without the Atk+ bonus except where noted; "the damage word" is
`av.u32[8]`.

| Navi | Routine | Damage | Element | Charge ticks (Charge 0 to 4) |
|---|---|---|---|---|
| HeatMan | 0x40 | 20 + 20n | Fire | 90, 80, 70, 65, 60 |
| SpoutMan | 0x41 | 20 + 20n | Aqua | 40, 35, 30, 25, 20 |
| TenguMan | 0x42 | 40 + 10n | Null | 60, 55, 50, 45, 40 |
| SlashMan | 0x43 | 10 + 10n | Null | 40, 35, 30, 25, 20 |
| ElecMan | 0x44 | 40 + 20n | Elec | 110, 100, 90, 85, 80 |
| TomahawkMan | 0x45 | 100 + 20n | Wood | 140, 130, 120, 115, 110 |
| EraseMan | 0x47 | 50 + 20n | Null | 60, 55, 50, 45, 40 |
| ChargeMan | 0x48 | 20 + 10n | Fire | 120, 110, 100, 95, 90 |
| DustMan | 0x49 | 60 + 20n | Null | 100, 90, 80, 75, 70 |
| GroundMan | 0x4A | 10 + 10n | Null | 130, 120, 110, 105, 100 |

- **HeatMan (`sub_80F070E`).** 0 (`sub_80F0728`): anim 0x15, 8 ticks; then anim 0x10 and the flames (`sub_80F08BA`:
  the panel ahead must be a panel of the field with a floor (0x10010), else none; a flame, attack #0x26,
  chips/heatman/flame, Param1 0 (it waits a dimming out), Param2 30, the element and damage word, on it and on
  each of the three panels of the column past it that passes the same test) → 4. 4 (`sub_80F0754`): 30 ticks →
  anim 0, exit.
- **ElecMan (`sub_80F094C`).** 0 (`sub_80F0968`): anim 0x12; a thunderbolt (`sub_80F0B32`: chips/elecman/thunder,
  the kind that cracks its panel and waits a dimming out) over every panel an enemy navi stands on
  (`object_getPanelsExceptCurrentFiltered` with `off_80F0B78[side]`: 0x04000000; side 1 0x08000000), from the last
  found to the first; 20 ticks → 4. 4 (`sub_80F0990`): anim 0x13, 30 ticks → anim 0, exit.
- **SlashMan (`sub_80F0C48`).** 0 (`sub_80F0C64`): anim 0xC; the waves (`sub_80F0E3A`: a sword wave,
  chips/slashman/wave, his AI's plain one, from the panel ahead and from the panels above and below his own, each
  if on the field); 15 ticks → 4. 4 (`sub_80F0C8C`): 30 ticks → anim 0, exit.
- **EraseMan (`sub_80F0FB4`).** 0 (`sub_80F0FD4`): anim 0x13, 20 ticks → 4. 4 (`sub_80F0FF8`): anim 0x14, sound
  0xC7, Timer 10; the tick it reaches 5, the slash (`sub_80F10F2`: a hit on the panel two ahead with region 3, the
  column; no spark, target 5, self 7, modifier 3, the element and damage) and its effect (`sub_80F111C`: T4#0
  effect at that panel's center 24 pixels up, the swords' wide slash in palette 6, his flip); 10 ticks → 8. 8
  (`sub_80F1032`): 30 ticks → anim 0, exit.
- **ChargeMan (`sub_80F1198`).** 0 (`sub_80F11BC`), the charge: FuturePanel his, reserved; flags 0x40 and
  0x400000; invulnerable (0xFFFF); his body no longer hurts (`sub_801A082` with modifier 0, self damage and
  counter byte 0); anim 0xF; sound 0xE4; X velocity front · 0x60000 and Timer = the distance to the edge column
  (7; side 1: 0) over the speed (`sub_80F146E`). Each later tick: X += velocity; passing his panel's center
  (`sub_800E708`), a train car (`sub_80F14BC`: chips/chrgeman/car on the panel he started from, its place 0 or 1,
  no wait, his speed, the damage word; two at most, kept in `av+0x30`, `av+0x34`); the panel under him on the
  field and without a floor → 4; else his panel and collision panels follow, a hit on each new panel
  (`sub_80F149C`: region 1, the breaking spark, target 5, self 6, modifier 1), and the timer run out → 8. 4
  (`sub_80F1284`): anim 4, his cars burst (`sub_80F1500` → `sub_80DB03E`), 3 ticks → 8. 8 (`sub_80F12AC`): back
  onto FuturePanel (unreserved), coordinates, collision panels, the flags and the invulnerability off
  (`sub_800EB08`), his body's hit again (modifier 3, his damage, its low byte as the counter byte), anim 3, 3
  ticks → 0xC. 0xC (`sub_80F1310`): anim 0, 30 ticks → exit.
- **SpoutMan (`sub_80F153C`).** 0 (`sub_80F1558`): anim 0x13, 8 ticks; then anim 0x14, a water ball from his own
  panel (chips/spoutman/ball, the one whose splash cracks its panel), a T4#0 effect at his position on the ground
  (his flip), sound 0x8E → 4. 4 (`sub_80F15AA`): 30 ticks → anim 0, exit.
- **TomahawkMan (`sub_80F17C4`).** 0 (`sub_80F17E4`): anim 0xF, the first parameter's ticks → 4. 4
  (`sub_80F1808`): anim 0x10, sound 0x10B, Timer 30; the tick it reaches 20, a hit on the panel ahead with region
  the two columns ahead (no spark, target 0, self 7, modifier 1, the element; **the damage word plus the Atk+
  bonus**) and a T4#0 effect at that panel's center 16 pixels up (the big slash, anim 1, his flip); 30 ticks → 8.
  8 (`sub_80F1888`): anim 7, the second parameter's ticks → exit.
- **TenguMan (`sub_80F19D4`).** 0 (`sub_80F19F0`): anim 0x16; three hits (`sub_80F1ADE`, `byte_80F1B18`): on the
  panels one, two and three ahead in his row (region 1, no spark, target 0, self 1; modifiers 3, 3 and, the
  farthest, 0 with status 0x10); sound 0xFB; 30 ticks → 4. 4 (`sub_80F1A1E`): anim 7, 20 ticks → anim 0, exit.
- **GroundMan (`sub_80F1BA8`).** 0 (`sub_80F1BC4`): anim 0xF; the drills (`sub_80F1E08`, `byte_80F1E48`: on the
  column ahead, the rows above, his and below, each if on the field: attack #0xC6 with Param1 1, 0, 1 (a side
  drill or the middle one) and Param2 12, 0, 24 (its wait), the element and damage word); sound 0x14C; 70 ticks →
  4. 4 (`sub_80F1BF4`): anim 7, 20 ticks → anim 0, exit.
  **The flying drill, T3 0xC6 (`sub_80DDDF0`)**: init (`sub_80DDE14`): over its panel 20 pixels up, his sprite
  (0x08, 0x09), a ground shadow, anim 0x15 (a side drill: 0x17), a breaking attack at navis (self 6, target 5,
  modifier 1, the breaking spark). Each tick (`sub_80DDE82`): the battle over or its owner out of the action he
  threw it in → gone; a hit on an enemy's body or object or a neutral object (`byte_80DDEEC[side]`: 0x05800000;
  side 1 0x0A800000) → it drills; then, unless dimmed, its action. 0 (`sub_80DDEF4`): it waits Param2 ticks. 4
  (`sub_80DDF18`): anim 0x16 (0x18), sound 0xF0, X velocity front · 0x80000 for 10 ticks (two panels), moving from
  the entry tick; off the field → gone; the timer run out → it drills. 8 (`sub_80DDF86`): 30 ticks, a hit on its
  panel (`sub_80DDFDE`: region 1, the breaking spark, target 5, self 6, modifier 1) at 20 and at 10 left, then
  gone.
- **DustMan (`sub_80F1F18`).** 0 (`sub_80F1F34`): anim 0x17; a junk ball from his own panel (DustCross's,
  forms/dustcross/junk_ball, the element and damage word); sound 0x12B; 35 ticks → 4. (The routine loads the
  registers for an effect at his position and spawns none.) 4 (`sub_80F1F7C`): anim 0, 30 ticks → exit.

**ProtoMan** (navi 11, AI index 0xB: his table has no entry 9). His charged attack, routine 0x32
(`sub_801206E`), is WideSwrd's slash: it loads the chip into the attack (`loc_80126EA`: its action, element with
the Sword bit, no lockout, no bonus, not charged) without spending one, then sets 60 + 20n damage and counter
byte 0x8A. His B+Back specials: 0x34 (`sub_80120A6`) is the Reflector's action (0x2B) as its subtype 3 with
params 0x114, a guard of 20 ticks in the NaviCust Reflect's look that fires nothing back, damage 0, chip lockout
40; 0x30 (`sub_8012018`) is the NaviCust Reflect's own guard, its shot's damage NaviStats+0x48.

As ported, each is a weapon definition with its action a definition, beside the navi:

| Weapon | Module | Kinds it spawns |
|---|---|---|
| `heatman/charge` | navis/heatman/charge.luau | chips/heatman/flame |
| `spoutman/charge` | navis/spoutman/charge.luau | chips/spoutman/ball |
| `tenguman/charge` | navis/tenguman/charge.luau | |
| `slashman/charge` | navis/slashman/charge.luau | chips/slashman/wave |
| `elecman/charge` | navis/elecman/charge.luau | chips/elecman/thunder |
| `tomahawkman/charge` | navis/tomahawkman/charge.luau | |
| `eraseman/charge` | navis/eraseman/charge.luau | |
| `chargeman/charge` | navis/chargeman/charge.luau | chips/chrgeman/car (and `car.burst`) |
| `dustman/charge` | navis/dustman/charge.luau | DustCross's junk ball |
| `groundman/charge` | navis/groundman/charge.luau | navis/groundman/drill.luau (`groundman/drill`) |
| `protoman/charge` | navis/protoman/charge.luau | (WideSwrd's action and record) |
| `protoman/back-special`, `protoman/back-special-2` | navis/protoman/back_special.luau | the Reflector's shield |

The drill keeps the action its owner threw it in and compares it with his running action. A kind's position
between its spawn and its init is its spawner's registers, and several keep the fraction of the Z they were
spawned with: the first of SlashMan's waves the low half of the battle state's address (the navi's update leaves
it in r3), a later wave or thunderbolt the side and flip halfword of the one before, a drill twice its place, a
car the speed.

Lab: navis/navi-01-heatpres to navi-11-stepswrd run each navi's charged attack twice (from his start panel and
from the row above, the opponent standing still on his own row) and B+Back, and match on every frame. Not
reached by a recording: HeatMan with no floor ahead (no flames), or a column of fewer than three; ElecMan with
no enemy on a panel or more than one; SlashMan's and GroundMan's rows off the field (the charge from row 1 covers
the upper one only); ChargeMan's second car's absence from a column near the edge; GroundMan's drills stopped by
a hit, flown off the field, or orphaned by a flinch; the charge at Charge levels past 0 and buster Attack past 1;
routine 0x46 (which no navi's stats name). ChargeMan stopped by a hole (his cars' burst) and ProtoMan's 0x34 are
recorded since (`navis/navi-05-volcchrg/charge-hole-*`, `navis/navi-11-stepswrd/level-5`) and match.
