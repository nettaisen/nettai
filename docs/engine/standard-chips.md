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
| 0x22 | `sub_80EC960` | 0x59 CrakShot, 0x5A DublShot, 0x5B TrplShot | attack #0x33 crack shot | chips/059-crakshot, objects/crack-shot |

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
