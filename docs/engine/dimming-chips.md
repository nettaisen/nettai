# Dimming chip controllers (`off_802CCB4`): barriers, panels, BugFix, music, planes, turrets, holes and points

The dimming chips of action 0x15 (docs/engine/chips.md §3.6) each spawn a controller from `off_802CCB4[subtype]`
(chip +0x0C). This document specifies, routine by routine and branch by branch, the subtypes below and every
object they spawn, and AntiNavi in the dimming service. It is written from the disassembly; the chip lab's
coverage report and traces (in the verification workspace) say which branches a scenario reaches, and those are
marked **verified** or **unverified** in each section.

| subtype | chips | controller | objects | section | port (content/bn6) |
|---|---|---|---|---|---|
| (service) | AntiNavi 0xBA against navi chips 0xDD..0x118 | the navi chip controller T4#0x10 | effect #0 | §2 | crates/bn6-battle/src/dimming.rs |
| 4 | 0xB2 Barrier, 0xB3 Barr100, 0xB4 Barr200, 0xB5 BblWrap, 0xB6 LifeAur | T4#0x2F | the barrier visual T4#7 | §3 | lib/barriers, chips/barrier, chips/bblwrap, chips/lifeaur; FirstBarrier: rules/roles |
| 5 | 0xA6 PnlRetrn, 0xA8 HolyPanl, 0xA9 Snctuary, 0xAA ComingRd, 0xAB GoingRd | T4#0x20 | the panel changer T4#0x1F | §4 | lib/panel-chips, objects/panel-changer, chips/{pnlretrn,holypanl,snctuary,comingrd,goingrd} |
| 26 | 0xB0 BugFix | T4#0x3B | the glow T1#0x5D | §5 | chips/bugfix |
| 9 | 0x92 Fanfare, 0x93 Discord, 0x94 Timpani, 0x95 Silence | T4#0x4A | the instrument T3#0x78 | §6 | lib/instruments, chips/{fanfare,discord,timpani,silence} |
| 13 | 0x68..0x6A AirRaid1-3 | T4#0x4B | the plane T3#0x75, its propeller T1#3, its overlay T1#0x54, bombs (panel strikes T3#9) | §7 | chips/airraid (the overlay: chips/lilbolr/layer) |
| 28 | 0x71..0x73 Sensor1-3 | T4#0x65 | the turret T3#0xA6, its scanner T3#0x9F, its laser T3#0xA0 | §8 | chips/sensor |
| 36 | 0x87..0x89 SumnBlk1-3 | T4#0x7C | the hole's navi T1#0x32 | §9 | chips/sumnblk |
| 27 | 0xC2 ColorPt, 0xC4 DblPoint | T4#0x50 | the points T4#0x51 | §10 | chips/colorpt |

Every controller is built on lib/dimming (`dimming_chips.phases`, `done`, `spawn`); a chip's `dimming` hook spawns
it with the chip's own data (a barrier, a panel change, an instrument, a plane, a turret's look and HP, a variant,
a point's look and bonus) as content records, not the original's parameter bytes.

Conventions: T1/T3/T4 are the actor, attack and effect pools (`object_spawnType1/3/4`); "Param1..4" the spawn
parameters (object +4..+7), "EV+n" the object's ExtraVars (+0x60 + n). A spawn zero-fills the object (flags 0x19:
active, runs while dimmed, sprite not updating until `sprite_load`). "N ticks" counts updates. A timer store is a
halfword (`strh`) unless noted; `ldrh; sub #1; strh; bge` counts to −1 ("N+1 updates from N"), `bgt` to 0. Sounds
are `PlaySoundEffect` ids. Effect #0 is the generic one-shot effect (`SpawnT4BattleObjectWithId0`, parameters
[look, flip, palette add, priority]).

## 1. The common controller

Every controller here is the standard one (chips.md §3.6.2): state 0 `object_timefreezeBegin`, 4 the actions,
8 `object_timefreezeEnd`; actions 0 `object_dimScreen`, 4 `object_drawChipName` (the telop), 8 the effect, 0xC
`object_undimScreen`. An action past the table reads past it (an explicit error in a port). The spawners
(`push {r0-r2,r5}; object_spawnType4(n); pop {r1-r3,r5}`) are entered with r0/r1 the user's panel, r2 the
element, r4 the chip's parameters, r5 the user, r6 the damage word, r7 = chip | bonus << 16 (chip-table.md);
they spawn with X = the panel Y, Y = the element, Z = the caller's r3 (register garbage nothing reads:
`scratch_position`), and set:

| subtype | PanelX/Y | Element | RelatedObject1 | Alliance / flip | damage word | +0x30 | other |
|---|---|---|---|---|---|---|---|
| 4 `sub_80E3B50` | no | yes | user | alliance only (from the user) | yes | yes | |
| 5, 26, 9, 13, 28, 27 | yes | yes | user | both (halfword) | yes | yes | |
| 36 `sub_80E91B8` | yes | yes | user | both | yes | yes | EV+0 byte = Param1 |

(Subtype 4's controller has no panel: X/Y bytes 0.)

## 2. AntiNavi in the dimming service (`sub_800BDB2`, `sub_800BA8A`)

AntiNavi (chip 0xBA, subtype 20: objects/trap-chip) records itself as its side's defensive chip
(`byte_2036720`-style record at 0x02036720 + 0x10·side: chip, bonus, damage word, owner, object;
`sub_802CE78` reads it, `sub_802CEA6` clears it). A navi chip's controller (T4#0x10, chips.md §3.6.7) checks it
twice. "A navi chip" is chip − 0xDD ≤ 0x3B (0xDD..0x118, `sub 0xDD; cmp 0x3B; bhi`).

**Action 4, `sub_800BDB2`** (dispatch on CurPhase through `off_800BDC4`; a phase past 8 reads past the table):

- Phase 0 (`sub_800BDD0`): if the chip is a navi chip and the other side's record's chip is 0xBA: own dimming
  state = 4 (running), `sub_800ABC6(PanelX, PanelY)` (effect #0 look 0x46 at the panel's center, Y + 16 px,
  Z 32 px, flip = the local side (`BattleState+0x0D`), sound 0xA5), phase 4 (halfword store: phase 4, not
  entered). Otherwise action 8 (the name).
- Phase 4 (`sub_800BE0C`): first tick Timer = 0x1E; then count to −1 (31 more ticks) → phase 8.
- Phase 8 (`sub_800BE2C`):
  - First tick: the telop of chip 0xBA with no damage or bonus (`sub_801E792(0x4C or 0x50, 0, 0, 0xBA)`: 0x4C
    when the other side is the local one, `battle_networkInvert(alliance ^ 1) == 0`), sound 0x173.
  - Then wait for the telop (`sub_801E754`). Done:
    1. `sub_800B89C(own)`: own record state 0, controller 0 (its user stays).
    2. The controller's Alliance byte ^= 1 (the flip is kept). Call the new side T.
    3. `sub_800B8AC(T)`: both records' owner = T. T's state = 4. T's controller, if any, gets state word 8 and
       PhaseInitialized 4 (freed at its next update without its end logic).
    4. Both records' initiator (+3) = T.
    5. The owner in T's AntiNavi record (`sub_802CE78(T)` r3): if not null (the `mov r0, r3` is `adds r0, r3,
       #0`, which sets the flags), the controller's PanelX/Y = the owner's and RelatedObject1 = the owner.
    6. `sub_802CEA6(T)`: T's record cleared (AntiNavi is spent).
    7. Action 8.

**Action 8, `sub_800BA8A`** (the navi chip's name). First tick: unless own state is 4, own state = 2 and, if the
other side's state is not 0 or 3, retry next tick; then the telop (`sub_801E792(0x4C/0x50, damage, bonus, chip)`,
the damage and bonus shown only for chips with the damage flag, 0x02 in +9), sound 0x173.
Later ticks: wait for the telop; if the dimming was taken over (`sub_800B8C2`: own record's owner ≠ own side)
and the other side's state is not 0 or 5, own state = 3 and retry. Else own state = 4, then:

- the user in own record (+0xC) has HP (+0x24) 0: `dword_200F3B8[side] = 1` (never read), action + 8 (skip to the
  undim). **After AntiNavi turned the chip, T's record has no user**: the game reads the HP through the null
  pointer at address 0x24, in the BIOS, whose open-bus value (the last prefetched BIOS opcode: 0xE129F000,
  0xE25EF004, 0xE55EC002 or 0xE3A02004) is never 0 in its low half, so the navi comes;
- else a navi chip with the other side's record chip 0xBA: action − 4 (back to `sub_800BDB2`, phase 0: the
  other side's AntiNavi turns it again, so two armed AntiNavis send a chip back to its user, both spent);
- else action + 4 (the navi comes: `sub_80E1830` with the controller's user, which after AntiNavi is AntiNavi's
  user, on its panel).

The end (`object_timefreezeEnd`) then runs for T: T started the dimming, so it ends it.

Port: crates/bn6-battle/src/dimming.rs (`check_anti_navi`, `show_navi_telop`), with unit tests
(`anti_navi_turns_a_navi_chip_around`, `two_anti_navis_send_the_chip_back`). **Verified** against two scenarios
recorded for it, in the lab library as `chips/0x0ba-antinavi/heatman` and `chips/0x0ba-antinavi/bounce`: AntiNavi
(side 1) then HeatMan (side 0), 1170/1170 frames; both AntiNavis armed, then HeatMan, 1708/1708 frames (the
bounce). Their TOML files are in §12. Also **verified**: a turn when T has its own controller registered
(`chips/0x0ba-antinavi/own-controller`: T's AreaGrab is under way and the navi chip is the cut-in on it; T's
controller is freed without its end logic), and a user without HP at its telop's end
(`chips/0x0e3-heatman/user-deleted`: the other side's cut-in ran first and deleted it). Unverified: a turn with
AntiNavi's user deleted (owner null), which a netbattle can't reach (the round is over).

## 3. Subtype 4: the barriers (Barrier, Barr100, Barr200, BblWrap, LifeAur)

### 3.1 The controller (T4#0x2F, `sub_80E3AB8`; effect `sub_80E3AFC`)

Param1 is the barrier type: Barrier 1, Barr100 5, Barr200 7, BblWrap 8, LifeAur 9. The effect, first tick:

1. `sub_801A7CC(Param1)` on the user (RelatedObject1) (§3.2).
2. If the user's AIData+0x60 (the barrier visual) is set: `sub_80E0DC0` on it (state word 8: it frees itself at
   its next update, without clearing the link).
3. `sub_80E0D98` with r4 = Param1, r5 = the user, r7 = &AIData+0x60: a new barrier visual (§3.3).
4. Timer = 0x3C, PhaseInitialized 4.

Every tick (the first included): count the Timer to −1, then action 0xC (61 ticks in all).

### 3.2 `sub_801A7CC`: raise a barrier (r0 = type, r5 = the object)

On the object's collision data: Barrier (+6) = type; +0x14 (the weak element) = `byte_8020B8C[type]`; with
`byte_8020B2C` rows of three halfwords: +0x16 (barrier HP) = the first's low byte, +0x17 (threshold) = the
second's low byte, +0x1A (timer, halfword) = the third. The absorbing is `sub_801A802` (field-collision-damage.md
§4.2; ported in kinds/player/intake.rs). The table (16 types; type 0x10 is the popped state `sub_801A802` sets,
not a row):

| type | HP | threshold | timer | weak | chips |
|---|---|---|---|---|---|
| 0 | 0 | 0 | 0xFFFF | 0 | |
| 1 | 10 | 0 | 0xFFFF | 0 | Barrier |
| 2 | 30 | 0 | 0xFFFF | 0 | |
| 3 | 50 | 0 | 0xFFFF | 0 | |
| 4 | 80 | 0 | 0xFFFF | 0 | |
| 5 | 100 | 0 | 0xFFFF | 0 | Barr100 |
| 6 | 150 | 0 | 0xFFFF | 0 | |
| 7 | 200 | 0 | 0xFFFF | 0 | Barr200 |
| 8 | 1 | 0 | 0xFFFF | 0 | BblWrap (regrows) |
| 9 | 1 | 200 | 3000 | 0 | LifeAur (the aura) |
| 10 | 200 | 0 | 0 | 0 | (regrows) |
| 11..14 | 1 | 100 | 0xFFFF | aqua, elec, wood, fire | the element auras |
| 15 | 1 | 100 | 0xFFFF | 0 | |

Callers: this controller; FirstBarrier (§3.4); attack #0xC7's heal-and-barrier (`sub_80DE088`, type 5, after
`object_addHP(300)` and sound 0x8A, on the side's navi `sub_80103BC(alliance)`); a navi AI (`sub_810ABFE`,
`byte_810AC2C[version]`, 0xFF none, sound 0x89). Each but FirstBarrier ends the old visual first (step 2).

### 3.3 The barrier visual (T4#7, `sub_80E0AD4`)

**Spawn `sub_80E0D98`** (r4 = type, r5 = owner, r7 = the link slot, AIData+0x60): `object_spawnType4(7)` with X,
Y, Z = r1..r3 (garbage: the controller's leaves r1 = the table's timer halfword or 8, r3 = the collision data
pointer; FirstBarrier's differ) and Params = r4 (Param1 = the type); alliance and flip from the owner;
RelatedObject1 = owner; EV+4 = the link slot, `*slot = visual`; flags |= 4 (runs while paused; 0x17 after init);
Unk_0C (+0x0C, "shown") = 1.

**Update `sub_80E0AD4`**: the state routine (0 init, 4 update, 8 `sub_8016C9C`: free), then the sprite: unless
the fight is on (battle flag 1) and paused, `sub_801BC64` (update_sprite's dimming gating and holds, paused or
not).

**Init `sub_80E0B08`**: EV+0 = &`byte_80E0A14[type * 12]` (its look); `sprite_load(0x80, look[0], look[1])`, no
shadow, VISIBLE, CurAnim = CurAnimCopy = look[2], the animation loaded and one `sprite_update`, palette look[3];
hidden parts (`sprite_setUnk0x2c`) = look[8..12] (u32), except type 9: hidden parts 0x2000000 (part 6) unless the
owner is a player (AIData ActorType 2) with AIIndex 0 (MegaMan) on the local side (`battle_networkInvert == 0`),
when it is look[8..12] (0); `sub_8002EAC` (the parts keep their own facing); sound 0x89 (0x12D for type 8); state
4 and the update runs.

| type | sprite | anim | palette | attach point | hidden parts |
|---|---|---|---|---|---|
| 0, 9, 11..15 | 0C-07 | 0 (11..15: 2) | 0 (11: 2, 12: 1, 13: 4, 14: 3) | 2 (9, 15: 1) | 0 (11..14: 0x2000000; 9: see above) |
| 1..7 | 0C-3D | 0 | 0 (5, 6: 3; 7: 6) | 2 | 0 |
| 8 | 0C-20 | 0 | 0 | 0x15 | 0 |
| 10 | 0C-00 | 0 | 0 | 2 | 0 |

**The update `sub_80E0B8C`** (r4 = the owner):
1. VISIBLE on; off if `sub_800EB6C(alliance)` (the viewer is blind and it's the other side's), or if Unk_0C is
   0; off if the owner's panel isn't on the field.
2. Unless in action 8: position = the owner's X, Y, Z, then the whole-pixel halves of Y and Z − 2 (halfword
   stores). Then, in every action: + attach point look[4] of the sprite the owner's NameID names
   (`sub_800F26C` → `sub_8018842`: signed pixels x, y), X += x · the owner's facing, Z += y (whole pixels). In
   action 8 the position isn't reset, so this offset adds up every tick.
3. The action (`off_80E0C68`):
   - 0 `sub_80E0C74` (up): the link slot is empty or the owner's barrier is 0 → action 4; the barrier is 0x10
     (popped) → EV+8 = collision +0x15 (the hit modifier that popped it), action 8; the barrier HP byte (+0x16)
     is 0 → action 4; else stay.
   - 4 `sub_80E0CAA` (down): first tick CurAnim + 1 (sound 0x124 for type 8). Every tick: the owner's barrier
     is set, its HP byte nonzero and the owner not bubbled (f1 0x80000000): up again (CurAnim − 1, VISIBLE,
     sound 0x89 / 0x12D for type 8, action 0). The barrier set otherwise: when the frame parameters have 0x80
     (the animation's last frame), VISIBLE off (it stays). The barrier 0: at the animation's last frame, clear
     the link if it still names this object, and free.
   - 8 `sub_80E0D38` (blown away): first tick X velocity = −8 px · its own facing, negated when EV+8 has 0x14
     (pushed forward), Y velocity = −2.5 px (0xFFFD8000), CurAnim + 1, Timer 0x14. Every tick X += Xvel,
     Y += Yvel; count the Timer to 0 (20 updates), then VISIBLE off, clear the link if it names this object,
     free.
4. Step 1 again, but only clearing VISIBLE (an action's VISIBLE on stays unless one of the tests fails).

**Hidden and shown with its owner**: `sub_80E1352(owner, mask)` (bit 0 clear) and `sub_80E146C` set its
Unk_0C to 0 (`sub_80E0DCA`); `sub_80E13DC` and `sub_80E14AC` set it to 1 (`sub_80E0DD0`). (The port's
`dimming::hide_user` / `show_user` are `sub_80E1352(user, 0)` / `sub_80E13DC`: they must reach the visual once
it exists.) The deletion's `sub_801A7F4` clears the link and the barrier (kinds/player/reactions.rs): the visual
then goes down (action 4) and frees itself.

### 3.4 FirstBarrier (`sub_8013892`, the player's init)

With navi stat 6 (FirstBarrier) nonzero: `sub_801A7CC(stat 6)`, then `pop {r4}` (the type), and
`sub_80E0D98` with r7 = &AIData+0x60 (no old visual to end). The pop clobbers r4, which `sub_80172F0` holds as the
AIData pointer for its later `sub_80E0F02` (the charge glow, effect #8): r7 = type + 0x58, a BIOS address. So:

- the glow's `str r0, [r7]` (the link AIData+0x58) goes to the BIOS (ignored): AIData+0x58 stays 0, so nothing
  that hides or shows the glow through the navi (`sub_80E1352`/`sub_80E13DC`, deletion) reaches it;
- the glow's EV+0 = type + 0x58, and its update's link test (`ldr r0, [EV]; ldr r0, [r0]`, `sub_80E0E20`) reads
  the BIOS open bus (never 0): it never ends itself, and stays for the round.

Port: kinds/player/mod.rs `init_navicust` calls the role hook `hooks.first_barrier` (content/bn6/rules/roles.luau
raises the Barrier chip's barrier, lib/barriers), and `init` spawns the glow unlinked (kinds/charge_glow.rs: a
glow whose link check always passes).

### 3.5 Lab coverage

Every Barrier-family scenario reaches the controller, `sub_801A7CC` and the visual's init, action 0 and action
4's going down and freeing. The coverage scenarios add, all **verified**: ending an old visual (`sub_80E0DC0`,
`chips/0x0b3-barr100/over-barrier`), the visual's regrowth (`chips/0x0b5-bblwrap/popped`), action 8, a barrier
blown away (`chips/0x0b3-barr100/blown-away`, `chips/0x0b6-lifeaur/weak-and-strong`, and AirShot, WindRack and
AirSpin against Barr100), and a barrier broken with damage to spare (`chips/0x0b3-barr100/broken`, and every
family's first chip against Barr100), the blind viewer (`chips/0x0b2-barrier/blind-viewer`: the opponent's
Silence, then its Barrier) and the visual hidden and shown with its owner (`chips/0x0e3-heatman/user-barrier`,
`chips/0x08d-magnum/user-barrier`, `chips/0x06e-burnsqr1/user-barrier`: a chip used from behind a Barrier).
**Unverified**: the hidden-parts rule for type 9 on the remote side, the off-field owner branch.

## 4. Subtype 5: the panel chips (PnlRetrn, HolyPanl, Snctuary, ComingRd, GoingRd)

### 4.1 The controller (T4#0x20, `sub_80E2AE8`; effect `sub_80E2B2C`)

First tick: Param2 = 1 (a busy flag), and the panel changer (`sub_80E2ACA`) with Param1 = the controller's Param1
(the kind), on the controller's panel, the controller's alliance and flip, RelatedObject1 = &the controller's
Param2 (spawned with X = the panel Y, Y and Z register garbage). Every tick (the first included): Param2 = 0 →
action 0xC. Chips' Param1: PnlRetrn 0, HolyPanl 4, Snctuary 5, ComingRd 0x11, GoingRd 0x12.

### 4.2 The panel changer (T4#0x1F, `sub_80E28A8`)

Port: objects/panel-changer (kind `panel-changer`, every change a `panel_changer.change` record), spawned by
`panel_changer.spawn` (`sub_80E2ACA`: X = the panel's Y, Y and Z register garbage), which clears its holder's
`busy` (the controller's Param2) when it ends. AntiRecv's counterattack uses change 6 (chips.md §3.6.7); the
subtype-5 controller is lib/panel-chips.

**Init `sub_80E28C8`**: on side 1, kind 0x11 ↔ 0x12 (the roads point the other way). EV+0 = &`byte_80E272C[kind
* 20]` (a row, below). The row's collector (byte 1, a byte offset into `off_80E291C`) lists the panels meeting the
side's condition, as bytes y << 4 | x at +0x68, the count in EV+4:

- 0 `sub_80E29F8`: every panel, rows 3..1, columns 6..1;
- 4 `sub_80E2A30`: its row, columns 6..1;
- 8 `sub_80E2A64`: the panel in front (PanelX + facing);
- 0xC `sub_80E2A9A`: its own panel.

Timer = row byte 3. State 4 and the update runs.

**Update `sub_80E292C`**: Timer − 1 (halfword).
- Not 0: when the new Timer & 3 is 0, sound 0xA3 (not for kinds 8, 7, 0xC). Rows with the blink byte (2) set
  blink only on ticks whose Timer has bit 2 set (`lsr #3; bcc`); others every tick. Blinking: each listed panel,
  last to first, `object_setPanelTypeBlink(x, y, row type, the panel's alliance)`.
- 0: each listed panel, last to first, `object_setPanelType(x, y, row type)` (a missing panel stays missing; a
  road gets the road timer 0x708) and the blink; then sound 0xA4 (0x90 for kinds 6 and 0xB; none for 8 and 7);
  state 8.

**Destroy `sub_80E29E6`**: a word store of 0 through RelatedObject1 (at the controller's +5; the ARM ignores the
address's low bits, so it clears the controller's four parameters), then free. The controller sees Param2 = 0 at
its next update (it runs before the changer).

**`byte_80E272C`** (20-byte rows: type, collector, blink, timer, side 0 require/forbid (u32), side 1
require/forbid). Panel types: 2 normal, 4 poison, 5 holy, 6 grass, 7 ice, 8 volcano, 0xB road left, 0xC road
right. Flags: 0x20 side 1's panel, 0x8000 missing, 0x200 road, 0x400 grass, 0x800 ice, 0x1000 volcano, 0x2000
holy.

| kind | type | panels | blink | ticks | side 0 (require / forbid) | side 1 | user |
|---|---|---|---|---|---|---|---|
| 0 | normal | all | yes | 0x40 | 0 / 0x8020 | 0x20 / 0x8000 | PnlRetrn (own area) |
| 1 | volcano | all | yes | 0x40 | 0 / 0x8000 | 0 / 0x8000 | |
| 2 | ice | all | yes | 0x40 | 0 / 0x8000 | 0 / 0x8000 | |
| 3 | grass | all | yes | 0x40 | 0 / 0x8000 | 0 / 0x8000 | |
| 4 | holy | front | yes | 0x40 | 0 / 0x8000 | 0 / 0x8000 | HolyPanl |
| 5 | holy | all | yes | 0x40 | 0 / 0x8020 | 0x20 / 0x8000 | Snctuary (own area) |
| 6 | poison | own | yes | 0x40 | 0 / 0x8020 | 0x20 / 0x8000 | `sub_80E376C` (subtype-20 family) |
| 7 | normal | front | no | 0x40 | 0 / 0x8000 | 0 / 0x8000 | |
| 8 | normal | row | no | 0x40 | 0 / 0x8000 | 0 / 0x8000 | |
| 9 | normal | own | yes | 0x1E | 0 / 0x8020 | 0x20 / 0x8000 | |
| 10 | normal | all | no | 0x1E | 0 / 0x8020 | 0x20 / 0x8000 | |
| 11 | poison | all | yes | 0x1E | 0x20 / 0x8000 | 0 / 0x8020 | (enemy area) |
| 12 | normal | all | yes | 0x1E | 0x2000 / 0x8020 | 0x2020 / 0x8000 | (own holy panels) |
| 13 | normal | all | yes | 0x1E | 0x1000 / 0x8000 | same | (volcano panels) |
| 14 | normal | all | yes | 0x1E | 0x800 / 0x8000 | same | (ice panels) |
| 15 | normal | all | yes | 0x1E | 0x200 / 0x8000 | same | (road panels) |
| 16 | normal | all | yes | 0x1E | 0x400 / 0x8000 | same | (grass panels) |
| 17 | road left | row | yes | 0x40 | 0x20 / 0x8000 | 0 / 0x8020 | ComingRd (side 0), GoingRd (side 1) |
| 18 | road right | row | yes | 0x40 | 0x20 / 0x8000 | 0 / 0x8020 | GoingRd (side 0), ComingRd (side 1) |

(A kind past 18 reads past the table.) This is the pack data the subtype needs.

**Timeline**: the controller spawns the changer on tick N (the changer's init and first update run that tick:
Timer 0x3F); the change lands on N+63 (state 8); on N+64 the controller still waits and the changer clears the
flag and frees itself; on N+65 the controller goes to 0xC.

**Lab**: PnlRetrn, HolyPanl, Snctuary, ComingRd and GoingRd are reached (soundmod rounds 2 and 3 stop at
PnlRetrn). Kind 6 (the own-panel collector) matches scratch recordings of AntiRecv's counterattack (chips.md
§3.6.7). Side 1's road swap is **verified** (`chips/0x0aa-comingrd/side1`, `chips/0x0ab-goingrd/side1`), and a
holy panel cracked under its user in the same dimming (`chips/0x0a8-holypanl/cut-in-geddon`). **Unverified**: kind
9, kinds 1-3 and 7-16 (no chip), a changer with no flag pointer.

## 5. Subtype 26: BugFix

### 5.1 The controller (T4#0x3B, `sub_80E4910`; effect `sub_80E4954`)

First tick:
1. `sub_80C4AEC` with r0 = &the controller's Param2, r4 = 0: the glow (T1#0x5D, variant 0, §5.2); it sets
   Param2 = 1.
2. `sub_80E49C4(user)`: on the user's side's navi stats, zero bytes 0x31 (the stats processing bug), 0x13 (the
   panel trail level), 0x14 (buster blanks), 0x16 (on-hit status), 0x24 (emotion), 0x19 (custom drain), 0x18 (HP
   drain), 0x1A (battle start), 0x63 (hand-shrink turn), and halfword 0x54 (custom damage), in that order.
3. `sub_801E658`: the emotion portrait's bug-face flag (`byte_203529E`) = 0 (HUD only: the portrait's glitch
   draws GetPositiveSignedRNG1, the per-console stream).
4. With r5 = the user: stat 0x21 (Beast Out counter) nonzero → `sub_8014446` (AIData+0x32 = 0: the Beast Out
   counter isn't spent), else `sub_801443C` (0xFFFF: spent).
5. Timer = 0x3C (unused), PhaseInitialized 4.

Every tick: Param2 = 0 → action 0xC.

### 5.2 The glow (T1#0x5D, `sub_80C4828`)

**Spawn `sub_80C4AEC`** (r0 = the flag pointer, r4 = the variant, r5 = the spawner): `object_spawnType1(0x5D)`
(X, Y, Z register garbage), EV+0 = the flag pointer, `*flag = 1` (byte), alliance and flip from the spawner,
RelatedObject1 = `sub_80103BC(alliance)` (the side's first actor: its navi).

**Init `sub_80C4848`** (r4 = the navi): the sprite: if the navi's NameID record (`sub_800F29C`) is type 2 (a
player), `sub_800FC9E(stat 0x29 navi, stat 0x2C form)` (the player's own sprite: MegaMan (0, `byte_800FCBC[form]`),
another navi (8, `byte_800FCD5[navi]`)); else `sub_800F26C(NameID)`. `sprite_load`, ground shadow, VISIBLE,
position = the navi's X, Y, Z then the whole-pixel halves of Y and Z + 1; palette = the navi's (`sub_801002C`),
flip = its own; `sub_8010DF6` with the navi's NameID record (the navi's parts, e.g. a form's body overlay) kept in
RelatedObject2: if one came, its Param3 = 1 and flags |= 0x14; CurAnim 0, CurAnimCopy 0xFF; state 4 and the
update runs. (PanelX/Y stay 0.)

**Update `sub_80C48FC`**: VISIBLE on, off if `sub_800EB6C(alliance)`; the variant (`off_80C4930[Param1]`: 0
`sub_80C493C`, 1 `sub_80C49E4`, 2 `sub_80C4A52`); `object_updateSpriteTimestop`. State 8: free.

**Variant 0 (BugFix)**, actions:
- 0 `sub_80C4958`: every tick `sprite_clearFinalPalette`. First tick: `sub_80E1352(navi, 0xF)` (the navi
  vanishes: VISIBLE off, state bit 0x100000; mask 0xF spares its barrier visual, its confusion and blindness
  visuals and the HUD; its charge glow's EV+4 = 0 and its Full Synchro aura's +0x0C = 0), Timer 0, Timer2 2.
  Every tick: Timer2 − 1; below 0 → Timer2 = Timer >> 2, white (`sprite_forceWhitePalette`) and sound 0xD1.
  Timer + 1; at 0x78 → action 4. (The flashes come at Timer 2, 3, 4, 6, 8, 11, 14, 18, ...: further apart.)
- 4 `sub_80C49A4`: first tick Timer 0x1E; count to 0 (30 updates) → VISIBLE off, `sub_8011044(navi's record,
  1)` (its parts go), `sub_80E13DC(navi)` (the navi is back, with everything `sub_80E1352` hid), `*flag = 0`,
  state 8.

Timeline (lab, `chips/0x0b0-bugfix/hit`): controller effect 441 (glow spawned), glow action 4 at 560, done 590;
the controller undims from 591.

**Variant 1** (`sub_80C49E4`, no spawner passes it): first tick `sub_80E1352(navi, 0xF)`, `sub_80101C4` on the
navi (submerged off), white, Timer 0x1E; count to −1 → VISIBLE off, `sub_8011044(record, 1)`,
`sub_80E13DC(navi)`, `sub_80101AE(0x1E0)` on the navi (submerged for 480 ticks, field-collision-damage.md
§4.10.1), sound 0x93, `*flag = 0`, state 8.
**Variant 2** (`sub_80C4A52`, no spawner passes it): action 0 `sub_80C4A6C`: `sprite_clearFinalPalette`; first
tick `sub_80E1352(navi, 0xF)`, Timer 0x1E, sounds 0x77 and 0xD1; count to 0 → action 4, else white. Action 4
`sub_80C4AAC`: as variant 0's.

**Lab**: variant 0 is **verified** as far as the trace records it (19 scenarios; blocked today by the panic).
A navi with parts is **verified** (`chips/0x0b0-bugfix/link-navi`: HeatMan uses it). **Unverified**: a non-player
navi's sprite, the stat 0x21 = 0 branch (`sub_801443C`: every lab navi has a Beast Out counter), variants 1 and 2
(no caller).

## 6. Subtype 9: the instruments (Fanfare, Discord, Timpani, Silence)

### 6.1 The controller (T4#0x4A, `sub_80E5E00`; effect `sub_80E5E44`)

First tick: the panel in front (PanelX + facing, PanelY) meeting `byte_80E5E98[side]` (both sides: require
0x10, solid; forbid 0x0F880080, occupied) gets the instrument (`sub_80D4408`: r0/r1 the panel, r2 the element,
r3 0, r4 the controller's parameter word, r6 its damage word). Timer 0x3C. Every tick count to 0 (`bgt`) →
action 0xC (60 ticks).

Chips' Param1: Fanfare 0, Discord 1, Timpani 2, Silence 3 (damage 0).

### 6.2 The instrument (T3#0x78, `sub_80D4088`)

**Spawn `sub_80D4408`**: `object_spawnType3(0x78)` (X = the panel Y, Y = the element, Z = 0: overwritten by the
init), damage word = r6, PanelX/Y, alliance and flip from the controller, flags |= 0x10, registered as its side's
field object of class 1 (`setFieldBattleObject_800F614(obj, alliance, 1)`: the side's previous class-1 object
breaks).

**Init `sub_80D40A8`**: VISIBLE; sprite (4, 0x0A), ground shadow; CurAnim 0 (halfword with the copy); one sprite
update; flip; coordinates from the panel, Z = 0; FuturePanel = Panel; NameID = 0xDD + Param1; HP = MaxHP =
`dword_80D4140[Param1]` (60 for all four); Timer = 0x960 (the lifetime); EV+0 = &`byte_80D4078[Param1 * 4]`
(below); palette = row[2]; collision (none free: free) self 0x13, target 0x14, hit modifier 3, present; state 4
and the update runs.

**Update `sub_80D4144`**: VISIBLE; `sprite_clearFinalPalette`; `sub_801AD12` (hits, "keeps_damage");
`sub_800F672` (the lifetime); `sub_801B394` with `off_80D4170` (react and dispatch); `object_updateSprite`;
present. Its table: 0 `sub_80D4198`, 1 `sub_80165B8`, 2 `sub_80D42E8`, 3 `sub_80166AE`, 4 `sub_8016B02`, 5
`sub_8017CC0` (knocked back), 6 `sub_8016B36`, 7 `sub_8016B72`, 8 `sub_80D41B6`, 9 `sub_80D41FC`
(field-objects.md §4).

- 0 (appear, `sub_80D4198`): CurAnim 0, Timer2 0xA (unused), sound 0x94, action 8.
- 8 (rest, `sub_80D41B6`), not while dimmed: phase 0: CurAnim 3, Timer2 0x1E, phase 4; phase 4: count Timer2 to
  0 → action 9.
- 9 (play, `sub_80D41FC`), not while dimmed: phase 0: CurAnim 4, Timer2 = row[0] · 2, EV+4 = EV+8 = 1, phase 4.
  Phase 4 (`sub_80D4242`): the battle over → phase 8. Else EV+8 − 1, at 0: EV+8 = row[3] (0x40) and the tune
  (sound `byte_80D42A8[Param1]`: 0xA8, 0xA9, 0xAA, 0xAB); EV+4 − 1, at 0: EV+4 = row[1] (3) and the effect
  (`off_80D4298[Param1]`); Timer2 − 1, at 0 → phase 8. Phase 8 (`sub_80D42B0`): CurAnim 0, Timer2 =
  `dword_80D42C8[Param1]` (170, 60, 170, 30), phase 0xC. Phase 0xC (`sub_80D42CC`): the battle over → stay; count
  Timer2 to 0 → action 8.
- The effects, every third tick of playing (the first on playing's first tick):
  - Fanfare `sub_80D435C`: each of the side's four actor slots (BattleState + 0x80 + 0x10·side) holding an
    actor whose side's form (stat 0x2C) isn't 0x17 or 0x18 (Beast Over): `object_setInvulnerableTime(4)`
    (invulnerable timer 4, INVULNERABLE).
  - Discord `sub_80D4392`: a one-tick region (`object_spawnCollisionRegion`, T3#3) on its panel: element 5, Z 0,
    damage 0, region `byte_80D43B8[side]` (0x85 side 0, 0x84 side 1: the other side's player navi's panel), hit
    effect 0xFF, target type 0, self type 0x32; status 0x23 (confuse, 4 ticks).
  - Timpani `sub_80D43C0`: the same with region 0x83 (every solid panel), target 5, self 0x32, status 0x43
    (immobilize, 4 ticks), and a camera shake (`camera_initShakeEffect_80302a8(2, 4)`).
  - Silence `sub_80D43E8`: region 0x80 (every panel), target 5, self 0x32, status 0x33 (blind, 4 ticks).
- 2 (destroyed, `sub_80D42E8`): removed by a chip (f2 0x8000): absorbed (0x300000) → `sub_800F90E(7)`; else
  `sub_800F8CE` (blink out): still blinking → return (the region and registration stay until it's done); done →
  finish; not blinking → effect #0 look 0x14 at Z + 12 px, finish. Broken: sound 0x70, effect #0 look 0 at Z +
  20 px, finish. Finish: `sub_802EF5C`, unregister, region 0, drop the FuturePanel reservation, VISIBLE off,
  state word 8. State 8: `object_genericDestroy`.

`byte_80D4078` (4-byte rows by Param1: play length /2, effect period, palette, tune period):
Fanfare [0x55, 3, 0, 0x40], Discord [0x37, 3, 2, 0x40], Timpani [0x3C, 3, 4, 0x40], Silence [0x78, 3, 6, 0x40].

**Lab**: the four instruments' appear, rest and play with every effect are reached; the destroyed action
(`chips/0x092-fanfare/broken`, `pushed`, and Discord's, Timpani's and Silence's `broken`) and a tune played to its
end (`chips/0x092-fanfare/lifetime`) are **verified**; so are the removal paths (each instrument's `dustman`,
`colarmy` and `absorbed`: taken by DustMan, blinking out under ColArmy, absorbed by DustCross's B+Back, by its own
side and by the other), Fanfare's Beast Over test in both versions' Beast Over (`chips/0x092-fanfare/beast-over`,
`beast-over-gregar`, with `beast-shot-at` for Beast Out) and the battle-over branches while playing and while
resting (`chips/0x092-fanfare/round-end`, `chips/0x095-silence/round-end-rest`). **Unverified**: a failed
collision.

## 7. Subtype 13: AirRaid1-3

### 7.1 The controller (T4#0x4B, `sub_80E5ECC`; effect `sub_80E5F10`)

First tick: the panel in front meeting `byte_80E5F68[side]` (require 0, forbid 0x0F880080: not occupied, a hole
allowed) gets the plane (`sub_80D3742`, r2 the element, r3 0, r4 the parameter word, r6 = the damage word +
the bonus (+0x32)). Timer 0x3C, count to 0 → action 0xC (60 ticks). Param1: AirRaid1 0, 2 1, 3 2 (damage 10).

### 7.2 The plane (T3#0x75, `sub_80D34CC`)

**Spawn `sub_80D3742`**: as the instrument's (T3#0x75, flags |= 0x10, class 1 field object).

**Init `sub_80D3500`**: VISIBLE; sprite (4, 0x18), ground shadow, CurAnim 0, one sprite update, flip, coordinates
from the panel, Z 0, FuturePanel, NameID 0xE7; EV+0 = &`byte_80D34C0[Param1 * 4]`: HP = MaxHP = row[3], Timer
(lifetime) = row[0] · 2, palette row[2]; collision (none: free) self 0x13, target 0x14, hit modifier 3, present;
RelatedObject1 = its propeller (`sub_80B89DC` with r3 = 0: T1#3, §7.3); state 4 and the update runs.

| Param1 | lifetime | bombs | palette | HP |
|---|---|---|---|---|
| 0 | 150 | 10 | 0 | 30 |
| 1 | 190 | 14 | 1 | 40 |
| 2 | 230 | 18 | 2 | 50 |

**Update `sub_80D358C`**: as the instrument's (`sub_801AD12`, `sub_800F672`, `sub_801B394` with `off_80D35B8`:
0 `sub_80D35E0`, 1 shared, 2 `sub_80D36CE`, 3/4 shared, 5 `sub_8017CC0`, 6/7 shared, 8 `loc_80D35FE`, 9
`sub_80D3654`).
- 0 (appear): CurAnim 0, Timer2 0xA (unused), sound 0x94, action 8.
- 8 (take off, not while dimmed): phase 0: CurAnim 1, Timer2 0x1E, sound 0x1A9, phase 4; phase 4: Z += 0xA000 a
  tick, count Timer2 to 0 → action 9.
- 9 (bomb, not while dimmed): phase 0 `sub_80D3678`: CurAnim 2, Timer2 = row[1] (the bombs), EV+4 = 1, the
  overlay T1#0x54 (`sub_80C4038(0, 0, 1, 0)`, parameters 0x12810: §7.4) in EV+0xC, phase 4. Phase 4
  `sub_80D369E`: the battle over → action 2; else EV+4 − 1, at 0 and Timer2 > 0: Timer2 − 1, a bomb
  (`sub_80D3770`), EV+4 = 10. (Out of bombs, EV+4 counts on as a u32: nothing more happens until the lifetime
  runs out.)
- 2 (destroyed, `sub_80D36CE`): as the instrument's, absorbed as kind 6.
- State 8 `sub_80D34EC`: the propeller and the overlay get state word 8 (`sub_80B8A0A`, `sub_80C4072`), then
  `object_genericDestroy`.

**A bomb, `sub_80D3770`** (a 16-byte panel list on the stack):
1. `sub_80D37F4`: from the column in front (its PanelX + facing; none at column 0) forward to column 6 (side 0)
   or 1 (side 1), each column's panels rows 1..3 (`object_getPanelsInColumnFiltered`) holding the other side's
   body (side 0: 0x04000000, side 1: 0x08000000), appended.
2. None: the target is the other side's back column ((alliance ^ 1) · 5 + 1) in its own row.
3. Else: i = EV+8 (reset to 0 when ≥ the count), EV+8 = i + 1, the panel list[i]; then `GetPositiveSignedRNG2()
   & 7` (one RNG2 draw); on 1 or 5, a random neighbour: `GetRandomRelativePanelFiltered` over region 0xA's
   offsets ((0,−1), (0,1), (1,0), (−1,0), dx by the side's direction) meeting `byte_80D37E4[side]` (side 0:
   require 0x10020; side 1: require 0x10000, forbid 0x20: the other side's panels), one more RNG2 draw
   (`% count`, BIOS division); none matching returns x 0 (a strike off the field) and y left in r1.
4. A panel strike (`sub_80C5F2C`, objects/panel-strike) there: element 0, Z 0, parameters 0x20600 (Param2 6
   ticks, Param3 2: hit modifier 0), the plane's damage word.

### 7.3 The propeller (T1#3, `sub_80B88D0`)

**Spawn `sub_80B89DC`** (r3 = EV+0, r5 = the owner): at the owner's X/Y/Z, alliance and flip, RelatedObject1 =
owner, flags |= 0x14. **Init**: panel from coordinates; sprite (0x10, 0x3D), no shadow, VISIBLE, CurAnim 0,
palette 0, flip; state 4. **Update `sub_80B8932`**: position = the owner's, whole-pixel Y + 0x20 and Z + 0x1F;
+ the owner's current frame's attach point 1 (`sub_80030BA`: (0, 0) if the frame has none): X += x · its facing,
Y += y (16.16); VISIBLE = the owner's; the owner's color shader and final palette; with EV+0 = 0 the owner's flip
(and the sprite's); the owner's alpha (`sprite_getMosaicScalingParameters` → `sprite_setAlpha`); unless dimmed
or the owner's f1 has 0x80110C00 (bubbled, dragged, paralyzed, flinching), `object_updateSprite`. State 8: free.

### 7.4 The overlay (T1#0x54, `sub_80C3EE0`; also LilBoiler's)

**Spawn `sub_80C4038(r0, r1, r2, r3)`** (r4 = parameters, r5 = owner): EV+0xC = r0 (a Z offset), EV+8 = r1, EV+4
= r2 (the animation), EV+0 = r3; the owner's X/Y/Z, alliance and flip, RelatedObject1 = owner, flags |= 0x14,
EV+0x10 = EV+0x14 = 0. AirRaid's: parameters [0x10, 0x28, 1, 0], EV+4 = 1, the rest 0.
**Init `sub_80C3F00`**: panel from coordinates; sprite (Param1, Param2), no shadow, VISIBLE, CurAnim = CurAnimCopy
= EV+4; palette Param4, or the owner's when Param3 is 0; flip; state 4 and the update runs.
**Update `sub_80C3F52`**: position = the owner's, Z + EV+0xC; EV+8 set: Y and Z − 1 px, else + 1 px; unless
EV+0x18: VISIBLE = the owner's; Param3 0: the owner's palette; unless EV+0x18: the owner's color shader; the
owner's final palette; `sub_8002F3E`/`loc_8002F02` (the owner's blending); EV+0 = 0: the owner's flip; the
owner's alpha; EV+0x10 set: `sub_80C409C` (panel from coordinates, Y and Z + (3 − row) · 24 px, then + 1 px);
unless dimmed, paused or the owner's f1 has 0x80110C00, and unless EV+0x14 with the owner's HP ≤ 0,
`object_updateSprite`. State 8: free.

**Timeline** (lab, `chips/0x068-airraid1/hit`): the plane at 441 with its propeller; the undim ends 518; it
takes off 519..548, bombs from 550 (overlay) with the first strike at 551, then every 10 ticks.

**Lab**: the plane, its propeller and overlay, the bombs with and without the neighbour pick are reached.
The plane shot down, the battle's end and the bombs against a barrier and an invisible navi
(`chips/0x068-airraid1/broken`, `ko`, `barrier`, `invisible`) match every frame, as do the destroyed action's
removal paths (`dustman`, `colarmy`, `absorbed`), the plane's lifetime (`lifetime`), the bombs' panel list as the
opponent walks (`moving-target`) and AirRaid3's plane shot down (`chips/0x06a-airraid3/broken`). **Unverified**:
the no-target branch (step 2), a failed collision, the overlay's EV+0x10/0x14/0x18 and Param3-0 branches
(LilBoiler's).

## 8. Subtype 28: Sensor1-3

### 8.1 The controller (T4#0x65, `sub_80E7BFC`; effect `sub_80E7C40`)

First tick: the panel in front meeting `byte_80E7C98[side]` (not occupied) gets the turret (`sub_80DA390`, r3 0,
r4 the parameter word, r6 = damage + bonus). 60 ticks as §6.1. Parameters: Sensor1 [0, 0x14], Sensor2 [3,
0x1E], Sensor3 [6, 0x28] (Param1 the palette, Param2 the HP; damage 100/130/160, elec).

### 8.2 The turret (T3#0xA6, `sub_80DA050`)

**Spawn `sub_80DA390`**: as the instrument's (T3#0xA6, element r2, flags |= 0x10, class 1).
**Init `sub_80DA07C`**: VISIBLE; sprite (4, 5), ground shadow; CurAnim 0xA (halfword store: the copy 0); one
sprite update; flip; coordinates from the panel, Z 0; FuturePanel; NameID 0xE4; HP = MaxHP = Param2; Timer
(lifetime) 0x2D0; palette Param1; Param3 = the aim by row (`dword_80DA104`: row 1 → 2 (down), row 2 → 1
(straight), row 3 → 0 (up); row 0 → 0); EV+0x18 = 0; collision self 0x13, target 0x14, modifier 3, present;
state 4 and the update runs. The aim's row step `dword_80DA3E8` = [−1, 0, +1] by Param3.

**Update `sub_80DA108`**: as the instrument's, with `off_80DA134`: 0 `sub_80DA15C`, 1 shared, 2 `sub_80DA2EC`,
3/4 shared, 5 `sub_80DA37A`, 6/7 shared, 8 `sub_80DA172`, 9 `sub_80DA266`.
- 0 (appear): Timer2 0xA (unused), sound 0x94, action 8.
- 5 (pushed, `sub_80DA37A`): on the push's first step (Unk_0D 0): EV+0x1C = 1 and `sub_80DA424` (the scanner
  and the laser end); then `sub_8017CC0`.
- 8 (scan, not while dimmed): first `sub_80DA448` (EV+0x1C set: clear it, action 8 from phase 0: the scan starts
  over after a push). Phase 0: Timer2 0x28, phase 4. Phase 4: count Timer2 to 0 → CurAnim 0, Timer2 0xA,
  phase 8. Phase 8: count to 0 → CurAnim `dword_80DA1F4[Param3]` (1, 0, 4), Timer2 0xE, phase 0xC. Phase 0xC:
  count to 0 → CurAnim `dword_80DA244[Param3]` (2, 0, 5), the scanner (`sub_80D924E`) on (PanelX + facing,
  PanelY + step[Param3]) with the element, parameters Param3 | 0x100 (Param1 the aim, Param2 1), damage word 0,
  its link at EV+0; phase 0x10. Phase 0x10 (`sub_80DA248`): the scanner exists and has seen a body (its +0x66):
  `sub_80D9284` (the scanner ends) and action 9.
- 9 (fire, not while dimmed; `sub_80DA448` first, and nothing more the tick it restarts): phase 0 `sub_80DA290`:
  CurAnim `dword_80DA2D4[Param3]` (8, 7, 9); from its panel, step (facing, step[Param3]) while the column is 1..6
  and the row 1..3: a laser segment (`sub_80D94F2`) on each, its link in EV+4, +8, ... (5 slots); Timer (the
  lifetime) = 0x78; sound 0x19A; phase 4. Phase 4: the battle over → action 2; else nothing (the lifetime ends
  it).
- 2 (destroyed, `sub_80DA2EC`): removed: absorbed as kind 0xD, or the blink-out as the instrument's. Not
  removed: EV+0x18 set → go on blinking; else the Timer word (Timer | Timer2 << 16) nonzero → broken (sound
  0x70, effect #0 look 0 at Z + 20 px, finish); zero (the lifetime ran out with Timer2 0) → EV+0x18 = 1 and
  blink: `sub_80DA3EC` (first tick PhaseInitialized 1, Timer 0x14; VISIBLE while Timer & 2; count to 0, then
  finish). Finish as the instrument's.
- State 8 `sub_80DA070`: `sub_80DA424`, then `object_genericDestroy`.

`sub_80DA424`: the scanner (EV+0) ends (`sub_80D9284`: its link cleared, state word 8); each of the five segment
links (EV+4..+0x14) set: `sub_80D9532` (the link cleared, the segment's action 0xC).

### 8.3 The scanner (T3#0x9F, `sub_80D9154`)

**Spawn `sub_80D924E`** (r3 = the link): EV+0 = the link, `*link = scanner`; PanelX/Y and its start (+0x64,
+0x65) = the panel; element; damage word; alliance and flip; flags |= 0x10. Spawned with Z = the link's address
(its fraction survives the init's `Z16 = 0`: `scratch_z_fraction`).
**Init `sub_80D9178`**: coordinates from the panel, Z's whole part 0; sprite (0x10, 0x50), no shadow, VISIBLE,
CurAnim 0, palette 0, flip; +0x66 = 0; sound 0x10E; state 4.
**Update `sub_80D91C8`**, then `object_updateSpritePaused`: the battle over → state 8 (`object_genericDestroy`).
Its panel holding the other side's body (side 0: 0x04000000; side 1: 0x08000000) → +0x66 = 1 (also while
dimmed). Not while dimmed, action 0 (`sub_80D9210`): phase 0: phase 4, Timer 8. Phase 4: count Timer to 0 →
step (`sub_80D9298`); blocked: back to the start panel (coordinates, Z whole part 0); phase 0 either way (9
ticks a step).

**The step `sub_80D9298`**: sound 0x10E. With Param2 set, its panel holding another body of the other side or
a neutral object (side 0: 0x01800000; side 1: 0x02800000) blocks. x + facing reaching the column past the
field ((alliance ^ 1) · 5 + 1 + facing: 7 or 0) blocks; else PanelX = x (written before the row test). y +
`off_80D9328[Param1]` (−1, 0, +1) at 0 or 4 blocks; else PanelY = y. With Param2, the new panel's test again.
Otherwise coordinates from the panel, Z whole part 0.

### 8.4 The laser (T3#0xA0, `sub_80D9350`)

**Spawn `sub_80D94F2`** (r3 = the link, r4 = the aim): EV+0 = the link, `*link = segment`; panel, element, damage
word, alliance and flip; +0x64 = Param3 (0: the spawner passes the aim in Param1 only); flags |= 0x14; EV+8 =
`battle_isPaused()`.
**Init `sub_80D937C`**: coordinates from the panel, Z whole part `byte_80D9410[Param1]` (0x10, 0x16, 0x14);
sprite (0x10, 0x51), no shadow (not VISIBLE yet), CurAnim `dword_80D9408[Param1]` (1, 0, 2), palette 0, flip;
collision (none: clear the link, free) self 0x16, target 5, modifier 1, hit effect 3, status
`byte_80D941C[+0x64 · 4]` (0x10: paralyze, 90 ticks); region 0, present; Timer = Param2 (0); state 4.
**Update `sub_80D9350`**: the state routine; then EV+8 = `battle_isPaused()`, and unless paused
`object_updateSpritePaused`. State 4 `sub_80D9434`, unless paused: remove the collision (resolve), the hit spark;
the battle over → region 0, state 8 (`object_genericDestroy`). A hit (FlagsFromCollision) → region 0 and, unless
action 0xC, action 8. Not dimmed: the action (`off_80D9488`): 0 `sub_80D9498`: region 0, count Timer to 0 (0 →
at once) → VISIBLE, action 4; 4 `sub_80D94B8`: region 1; 8 `sub_80D94C2`: when EV+8 equals `battle_isPaused()`
(always, unpaused), region 0 and, once its panel is free of 0x0F880080, action 4 (armed again); 0xC
`sub_80D94E6`: region 0, state 8. Then present.

**Lab**: the scan, the fire and the laser are reached (Sensor1-3, 21/22 scenarios per chip before the panic).
The pushed turret (`chips/0x071-sensor1/pushed`), the broken turret (`broken`), the laser's re-arming (`twice`)
and the battle-over branches (`ko`) are **verified**; so are the removal, blink-out and absorb paths
(`dustman`, `colarmy`, `absorbed`), the scanner blocked by an object on its first panel and on its step
(`blocked-rock`, `blocked-rock-far`), running off the bottom and top rows (`row1`, `row3`) and the far column for
the turret's whole lifetime (`long-miss`), and the scanner and laser through a pause (`paused`). **Unverified**:
the scanner's Param2 0 (no spawner), failed collisions.

## 9. Subtype 36: SumnBlk1-3

### 9.1 The controller (T4#0x7C, `sub_80E9120`; effect `sub_80E9164`)

First tick: the panel in front's flags & 0x0F800010 nonzero (solid, or a body on it) → action 0xC at once.
Else (a hole with nothing on it): the navi (`sub_80C17F4`) on that panel with the element, parameter word = EV+0
(Param1), damage word = damage + bonus, r7 = &EV+4 (its busy flag), r3 0; PhaseInitialized 4 (and return).
Later ticks: EV+4 = 0 → action 0xC. Param1: SumnBlk1 0, 2 1, 3 2 (damage 160, 200, 260).

### 9.2 The hole's navi (T1#0x32, `sub_80C1570`)

**Spawn `sub_80C17F4`**: `object_spawnType1(0x32)`, PanelX/Y, element, alliance and flip, RelatedObject1 = the
controller, damage word, +0x64 = Param1, EV+0 = the flag pointer, `*flag = 1` (word).
**Init `sub_80C1590`**: sprite (4, 0x1D) (decompressed), ground shadow, CurAnim 0; coordinates from the panel, Z
0; flip; palette +0x64 · 3; VISIBLE off; state 4 and the update runs.
**Update `sub_80C15EC`**: the action, then `object_updateSpriteTimestop`. State 8 `sub_80C17E6`: `*flag = 0`
(byte), free.
- Action 0 (rise): phase 0 `sub_80C1630`: first tick sound 0x94, Timer 0x18; then VISIBLE, count to 0 → CurAnim
  5, sound 0x143, phase 4. Phase 4 `sub_80C166A`: at the animation's last frame CurAnim 4, Timer 0x1E; count to
  0 → phase 8. Phase 8 `sub_80C1698`: the target (`sub_80C18B0`, §9.3): found → +0x65/+0x66 = it, action 4;
  else action 8.
- Action 4 (strike): phase 0 `sub_80C16E0`: first tick an effect `sub_80C1820(panel, 1)` (effect #0 look 0x14 at
  the panel's center, flip = alliance ^ flip), move to the target panel (coordinates), effect look 0x15 there,
  Timer 2, VISIBLE off; count to 0 → VISIBLE, phase 4. Phase 4: CurAnim 1, phase 8. Phase 8 `sub_80C174A`: at
  the animation's last frame, Timer 0xA; count to 0 → CurAnim 2, the claw effect (`sub_80C185C`: effect #0 look
  0x16 on the panel in front, parameters 0x60216 | (alliance ^ flip) << 8: flip 2 | facing bit, palette + 6),
  the hit (`sub_80C188C`: `sub_80C53A6` on the panel in front: region 4 (its column: (0,0), (0,−1), (0,1)), hit
  effect 0xFF, target 5, self 7, hit modifier 3, the element, its damage word; resolving while dimmed), sound
  0x10A, phase 0xC. Phase 0xC `sub_80C1786`: at the animation's last frame, Timer 0x10; count to 0 → action 8.
- Action 8 (leave, `sub_80C17B2`): phase 0: RelatedObject1 = 0, Timer 0x10, effect look 0x14 on its panel,
  phase 4. Every tick VISIBLE off; count Timer to 0 → state 8.

### 9.3 The target, `sub_80C18B0`

1. The other side's actors (BattleState + 0x90 for side 0, + 0x80 for side 1; four slots), non-null, listed.
2. For each (last to first), with its own facing: the panel in front of it (x + facing; + 2 · facing when its
   NameID's look `sub_800F26C` is sprite (8, 0x14)) with no body (flags & 0x0F800000 = 0): kept.
3. For each kept (last to first), with the controller's facing and column (RelatedObject1: the user's column at
   the use): d = (its x − the controller's x) · facing; d < 2 skipped; the first taken; later ones taken when d ≤
   the smallest d so far (the list keeps earlier, farther ones).
4. None: not found. One: it. More: the one with the smallest row (a later one on a tie).
5. The result: the panel in front of the chosen one (as in step 2).

**Lab**: `chips/0x087-sumnblk1/hole-ahead`, `after-geddon` and `chips/0x089-sumnblk3/hole-ahead` have a hole in
front of the user and reach the whole navi (§9.2, §9.3: 127 blocks and branch sides the lab didn't have); they
match every frame. The target search with the opponent elsewhere matches too: in the hole's row and the bottom
row, a column nearer, in the back column (`hole-ahead-up`, `-down`, `-near`, `-back`), behind its own RockCube
(`hole-ahead-rock`: no panel to strike from, and the navi leaves), from side 1 (`hole-ahead-side1`), with the
battle ending at its strike (`hole-ahead-ko`), and SumnBlk2's (`chips/0x088-sumnblk2/hole-ahead`).

## 10. Subtype 27: ColorPt, DblPoint

### 10.1 The controller (T4#0x50, `sub_80E6480`; effect `sub_80E64C4`)

First tick: `sub_80E650A`, Timer 0x1E (unused). Every tick: the word EV+0 = 0 → action 0xC.

**`sub_80E650A`**: EV+0 = 0; for rows 1..3: `sub_800D4D0(alliance ^ 1, flip, row)` → (x, y): the user's front
column in that row: from the user's back column (`sub_800D53C`: x = 6 − 5 · (flip ^ side), stepping away from
the other side's, over panels the other side owns; 0 if it runs off the field), the first panel of the other
side (`object_getFirstPanelInDirectionFiltered` with `byte_800D52C`: side 0 forbid 0x20, side 1 require 0x20),
one back (its validity isn't checked); x = 0 skips the row. The panel must be solid and unoccupied (0x10 /
0x0F880080) and `sub_800D668(x, y, alliance)` must agree (the area-steal rule, chips.md §3.6.8). Then a point
(`sub_80E6720`) there with the chip's parameters, its flag byte at EV+0 + k (k-th point), damage word = the row
(register garbage), element register garbage.

### 10.2 A point (T4#0x51, `sub_80E655C`)

**Spawn `sub_80E6720`**: PanelX/Y, element, damage word, alliance and flip from the controller, RelatedObject1 =
the flag byte, `*flag = 1`, flags |= 0x10.
**Init `sub_80E6580`**: sprite (0xC, 0x13), no shadow, CurAnim 1, copy 0xFF. The side's navi
(`sub_80103BC(alliance)`) missing: `*flag = 0`, state 8. Else Param3/Param4 = the navi's panel; VISIBLE; palette
Param1 · 4; coordinates from the panel; sound 0x129; state 4 and the update runs. Update: the action, then
`object_updateSprite`. State 8: free.
- 0 `sub_80E6600`: first tick the panel goes to the other side (`object_setPanelAlliance(x, y, alliance ^ 1)`)
  and its column's return timer = 0x708 (`object_setPanelAllianceTimerLong`), Timer 0x13; count to 0 → CurAnim
  0, sound 0x12A, action 4.
- 4 `sub_80E6644`: Z + 4 px a tick until its whole part reaches 16 (then 16 px, Timer 0x10); count to −1 →
  action 8.
- 8 `sub_80E667C`: toward the navi's panel: the angle (`calcAngle_800117C` of the target's center minus its
  X/Y), a velocity of X velocity (0 at first) along it (`sub_80011A0`), X/Y += it, panel from coordinates. On the
  target panel: if the navi's AIAttackVars+0x1B (the special-source byte) is 0, the hand's chip at the cursor
  (`byte_20349C0` + 0x50 · side: the cursor at +0, chips at +2, their Atk+ bonuses at +0x1A) that exists and
  has damage (flags bit 1) gets bonus + Param2; else the side's special bonus (`sub_802E070(side)` +0x36) +
  Param2. Then sound 0x8C, `*flag = 0`, state 8. Not there: X velocity + 0x4000.

Parameters: ColorPt [0, 0x0A] (+10), DblPoint [1, 0x14] (+20).

**Lab**: the points, the steal and the flight are reached, and the bonus itself by `chips/0x0c2-colorpt/bonus` and
`chips/0x0c4-dblpoint/bonus` (a Cannon next), which match every frame; `chips/0x062-lilbolr1/colorpt` (a LilBoiler
next) is to rerun since LilBoiler's registration changed. **Unverified**: the special-source branch, a missing
navi, `sub_800D53C` running off the field.

## 11. The port (data and framework)

Content (model v2): the tables are the chips' records: the barriers (§3.2) are `barriers.barrier_10` and the
rest (lib/barriers/barriers.luau), the panel changes (§4.2) `panel_changer.change` records, `byte_80D4078`'s rows
(§6.2) `instrument.instrument` records with each chip's effect, `byte_80D34C0`'s rows (§7.2) `plane.plane`
records, the turrets' look and HP (§8.1) and SumnBlk's and the points' parameters the hooks' arguments; the
small per-aim tables of §8 and §7 are the kinds' constants. Collision types: rules/collision.luau (`nothing`
0x00, `own-body` 0x13, `guard-breaking` 0x32 join); regions: lib/regions.luau (the whole-field regions 0x80,
0x83, 0x84/0x85, the four neighbours 0x0A and `GetRandomRelativePanelFiltered`); the area-steal rule
(`sub_800D668`) and the front of an area (`sub_800D4D0`): lib/panels.luau.

Framework (Rust): `sub_801A7CC` is `Object:raise_barrier` (the barrier byte by behavior: plain 1, bubble 8,
regenerating 0xA), with the charge glow's clobbered link (§3.4); `dimming.hide_user_sparing` (`sub_80E1352` with
mask 0xF); `battle.clear_navicust_bugs` (§5.1); `Sprite:load_look_of`, `Object:add_parts_of`/`remove_parts_of`
(BugFix's glow); `Object:name_look_is` (`sub_800F26C`, §9.3); `battle.hand_chip_damages` (§10.2); and
`Sprite:part_offset` (`sub_80030BA`: where a part of the current frame sits, which the pack's sprite layouts
give; the propeller, §7.3). AntiNavi is done (§2).

The trace comparison (bn6-compat) skips the register garbage the original leaves: the controllers' positions
(`scratch_position`), the Sensor scanner's, laser's and the points' Z fractions (`scratch_z_fraction`), and the
Z fraction of a hit spark whose hitter's is garbage (Sensor's laser's sparks).

## 12. Scenarios recorded for this document

In the verification workspace's chiplab library, as `chips/0x0ba-antinavi/heatman` and
`chips/0x0ba-antinavi/bounce`:

```toml
# chips/0x0ba-antinavi/heatman.toml
description = "AntiNavi (side 1) armed, then HeatMan (side 0): AntiNavi turns the navi chip around."
base = "falzar"
max_frames = 1500
expect = ["used:0", "used:1"]
families = ["chip:0x0ba", "chip:0x0e3"]
[p0]
folder = ["0E3:H"]
script = "custom 0E3:H\nfight\nwait 200\nuse\nwait 600\nsettle\n"
[p1]
folder = ["0BA:F"]
script = "custom 0BA:F\nfight\nuse\nsettle\n"

# chips/0x0ba-antinavi/bounce.toml
description = "Both sides arm AntiNavi, then side 0 uses HeatMan: turned around, then back."
base = "falzar"
max_frames = 2000
expect = ["used:0", "used:1"]
families = ["chip:0x0ba", "chip:0x0e3"]
[p0]
folder = ["0BA:*", "0E3:*"]
script = "custom 0BA:* 0E3:*\nfight\nuse\nwait 420\nuse\nwait 900\nsettle\n"
[p1]
folder = ["0BA:F"]
script = "custom 0BA:F\nfight\nwait 200\nuse\nsettle\n"
```
