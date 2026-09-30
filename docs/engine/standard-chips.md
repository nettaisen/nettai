# Standard chip actions

The chip actions (`JumpTable80EAC60`, action ≥ 0x10) that neither fire the buster's projectile (attack #0) nor
dim the screen, and the objects they spawn. Each is a pack script (docs/design/content-migration.md): the action in
the first chip's folder, the chips that share it naming that module, the objects under `objects/`. This document
describes what the original does; the scripts are the port.

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
| 0x18 | `sub_80EC02A` | 0x13 YoYo, 0x154 GreatYo | attack #0x52 yoyo, effect #0x70 GreatYo's controller | chips/013-yoyo, objects/yoyo, objects/great-yoyo |
| 0x1F | `sub_80EC7A6` | 0x1E Thunder, 0x11F DarkThnd | attack #0x2A thunder ball | chips/01e-thunder, objects/thunder-ball |
| 0x22 | `sub_80EC960` | 0x59 CrakShot, 0x5A DublShot, 0x5B TrplShot | attack #0x33 crack shot | chips/059-crakshot, objects/crack-shot |
| 0x23 | `sub_80ECA34` | 0xBE CopyDmg | attack #0x28 copy mark | chips/0be-copydmg, objects/copy-mark |
| 0x26 | `sub_80ECCB0` | 0x32 AirHocky, 0x155 PitHocky | attack #0x2E puck (and PitHocky's afterimages, effect #0x28) | chips/032-airhocky, objects/hockey-puck |
| 0x27 | `sub_80ECD28` | 0x14..0x16 FireBrn, 0x143..0x145 WideBrn | attack #0x04 flame | chips/014-firebrn1, objects/flame |
| 0x28 | `sub_80ECDFC` | 0x18..0x1A TrnArrw | attack #0xB flying shot (kind 2) | chips/018-trnarrw1, objects/flying-shot (group D's) |
| 0x2C | `sub_80ED25C` | 0x7B..0x7D IronShl, 0x14D ParaShl | attack #0x34 iron shell | chips/07b-ironshl1, objects/iron-shell |
| 0x2D | `sub_80ED2F8` | 0x1B..0x1D BblStar | attack #0x42 bubble star | chips/01b-bblstar1, objects/bubble-star |
| 0x2E | `sub_80ED374` | 0x33 DrilArm | attack #0x71 drill | chips/033-drilarm, objects/drill |
| 0x2F | `sub_80ED454` | 0x34 Tornado, 0x35 Static | attack #0x31 tornado | chips/034-tornado, objects/tornado |
| 0x31 | `sub_80ED64C` | 0x5C..0x5E WaveArm, 0x149..0x14B PwrWave | attack #0x16 shock wave | chips/05c-wavearm1, objects/shock-wave |
| 0x32 | `sub_80ED6E6` | 0x3D..0x3F AquaNdl | effect #0x40 needle volley, attack #0x50 aqua needle | chips/03d-aquandl1, objects/needle-volley, objects/aqua-needle |
| 0x34 | `sub_80ED810` | 0x152 H-Burst | attack #0x9E hyper burst | chips/152-h-burst, objects/hyper-burst |
| 0x36 | `sub_80ED9AE` | 0x28..0x2A RlngLog | attack #0x66 rolling log | chips/028-rlnglog1, objects/rolling-log |
| 0x38 | `sub_80EDCC0` | 0x7E..0x80 AirSpin | attack #0x9B whirlwind, attack #0xD4 seeking whirlwind | chips/07e-airspin1, objects/air-spin, objects/seeking-whirl |
| 0x3E | `sub_80EE0BC` | 0x1F..0x21 DolThdr | attack #0x82 thunder doll, attack #0x8B thunder column | chips/01f-dolthdr1, objects/thunder-doll, objects/thunder-column |
| 0x3F | `sub_80EE192` | 0x50 WindRack | attack #0x49 gust | chips/050-windrack, objects/gust |
| 0x40 | `sub_80EE2A0` | 0x54 MoonBld | attack #0x85 moon blade | chips/054-moonbld, objects/moon-blade |
| 0x42 | `sub_80EE55E` | 0x22..0x24 ElcPuls, 0x14E DestPuls | attack #0x8C electric pulse | chips/022-elcpuls1, objects/elec-pulse |
| 0x43 | `sub_80EE634` | 0x5F..0x61 AuraHed, 0x150 StreamHd | attack #0x92 aura head | chips/05f-aurahed1, objects/aura-head |
| 0x44 | `sub_80EE6FC` | 0xB7 MagCoil | attack #0x95 magnet | chips/0b7-magcoil, objects/magnet |
| 0x53 | `sub_80EF62E` | 0x52 VarSwrd | (hands off to the chosen sword) | chips/052-varswrd, lib/vari_sword |
| 0x54 | `sub_80EF7E2` | 0x53 NeoVari | (hands off to the chosen sword) | chips/053-neovari, lib/vari_sword |

Not yet content: 0x55 (SonicBom, Curse, Punisher, SprSonic, `sub_80EF970`) and 0x5B (Z Saver, `sub_80EFEE0`), which
share the sonic boom (attack #0x58, `sub_80CF7F0`), specified in shot-chips.md with the actions that fire a shot;
and 0x0A, the link navis' chips (below).

Shared helpers: lib/panels.luau (`GetRandomRelativePanelFiltered`, `sub_8109708`), lib/object_setup.luau
(`sub_8011504`, the collision-panel highlight, the dust puff), lib/arm.luau (`sub_80EBAE8`, the arm a chip
attachment shows), lib/vari_sword.luau, and group G1's lib/region.luau and lib/trajectory.luau.

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
  its variant (`byte_80C6B00`, pack data in objects/shock-wave); WaveArm's variants 0xC..0xE steer toward an
  enemy's row.
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
  hit (status 0x18, bug 1); 10 ticks. Its spawner leaves register garbage (scratch_position).
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
kind 1; ParaShl, GreatYo's controller and PitHocky (Program Advances, stopped at the banner); Tornado subtype 3 and
Static's larger spreads; the drill without a slot; AirSpin variant 1 and the whirlwind's removed, absorbed and
blink-out paths; WideBrn's spread and flame looks 1 and 2; RlngLog's drop-in path and the log's stop and break;
DarkThnd and the thunder ball's bug; every VarSwrd/NeoVari sequence, random pick and the charged sword (0x41);
MoonBld's repeat swings; CopyDmg's time-up path and NameIDs 0xCD..0xFF; WindRack variant ≠ 0 and the gust's other
spawners; H-Burst's fight-over and whole-field branches; every "pool full" path. Off-table parameters are explicit
errors naming the routine.

## Action 0x22: CrakShot, DublShot, TrplShot (`sub_80EC960`)

The navi digs into the panels ahead and flings them. Two phases (`off_80EC974`):

- **Dig** (`sub_80EC97C`). Entry: animation 0xC, sound 0xD8, the counter window, `using_action`, timer 0xF. On the
  tick the timer reads 0xD (the third tick) it spawns a crack shot on every panel of a hit region
  (`PanelOffsetListsPointerTable`, `data.regions` in the pack) chosen by the variant (`dword_80ECA08`: CrakShot
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

## Action 0x0A: the link navis' chips (not yet content)

Chips 0x190 HeatPres to 0x199 DustBrk have action 0x0A, below 0x10, so they run through the user's own action table
(`off_80EA4C8[AIIndex][0xA]`, chips.md §1.6), not `JumpTable80EAC60`. Only the link navis' tables (AI indices 1 to
10) have an entry 0xA: 1 HeatMan `sub_80F0778`, 2 ElecMan `sub_80F09B8`, 3 SlashMan `sub_80F0CB0`, 4 EraseMan
`sub_80F1056`, 5 ChargeMan `sub_80F1334`, 6 SpoutMan `sub_80F15CE`, 7 TomahawkMan `sub_80F18AC`, 8 TenguMan
`sub_80F1A46`, 9 GroundMan `sub_80F1C1C`, 10 DustMan `sub_80F1FA0`. A content action registered for 0x0A runs for a
link navi (`status::dispatch`); none is registered yet.

Ported but not registered (chips/19x folders, their shared state in chips/190-heatpres/state.luau and
`LinkChipState` in types.d.luau, **[unverified]**): EraseMan's (the navi's erase beams, objects/erase-beam with
Param3 set), SpoutMan's (the jump and the water spray, attack #0x29, objects/drip-shower), ElecMan's (the slide and
slash, the glow effect #0x31, objects/navi-effect), SlashMan's (the rolling route, the riding hit attack #0x69,
objects/riding-hit), TomahawkMan's (the axe effect #0x32, objects/eagle-tomahawk, and the strikes attack #0x6A,
objects/tomahawk-strike), TenguMan's (attack #0x60, objects/tengu-tornado) and DustMan's (the dust clouds, effects
#0x71 and #0x72, objects/dust-cloud, one module for both). Left: HeatMan's (the arc, `sub_8001330`, and attack
#0x26), ChargeMan's (attack #0x86, its arc), GroundMan's (attack #0x80, effects #0x09 and #0x61), the dispatcher
module for action 0x0A and the kinds' `object.toml` registrations.

What the port has for them so far: their damage, damage formulas 24 to 44 (`sub_8010C50`): the chip's row of
`byte_80212D4` (its `navi_damage`: a base and a step), plus the step for each level of the user's buster attack
(`sub_801265A`) up to 5, and 0 without a player navi on the side.

What stops them, in the lab's link-navi scenarios (navis/navi-01 to navi-10), before the chip runs:

- the init hooks of AI indices 1, 6 and 9 (`off_8010E0C`) and DustMan's post-init hook `sub_80F22F8`;
- the link navis' chip bonus `sub_800F09E`, at chip use: by AI index, a damaging chip of the navi's family gets a
  bonus from `byte_8021300`, indexed by a per-side value (`dword_203CFA0`, copied from the battle's link data at the
  round's start) that the traces don't record; ChargeMan's charge limit (`sub_800F49E`) reads the same value;
- the objects several of them spawn are the navi chips' too (attack #0x26 HeatMan's, #0x80 GroundMan's, effects
  #0x09 and #0x61), and EraseMan's is objects/erase-beam with Param3 set (its "navi's" branch, which ends once the
  navi leaves action 0xA).
