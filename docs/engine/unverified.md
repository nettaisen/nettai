# Unverified branches: the survey

The port is written from the disassembly branch by branch, and the chip lab's recordings (the verification
workspace's chiplab) check it against the original. A branch no recorded scenario reaches is **unverified**:
ported, but never compared. This file collects those branches from the other documents into one table, says why
each is unreached and whether a scenario can reach it, and tracks the scenarios written for them.

Status:

- **verified**: a recorded scenario reaches the branch and the engine matches the recording on every frame;
- **differs**: recorded, and the engine differs; the note says where, and whose it is;
- **open**: reachable, no scenario yet;
- **unreachable**: no netbattle can take the branch (a full object pool, a path only offline battles or the navi
  AI take, a record no chip names, a caller that doesn't exist).

Scenario names are the chip lab library's (`chips/…`, `pa/…`, `navis/…`, `forms/…`, `navicust/…`, `stages/…`). The scenarios of
this survey are written by the library's `gen_coverage.py` from its tables (`coverage_scenarios/`). A scenario
"reaches" a branch when its coverage file has the block or branch side; where a scenario adds no block the lab
didn't have, it still checks data the others don't (another table row, another side, another timing), and the
table says so.

A caution for every scenario: the falzar base's save gives side 0 AirShoes, FlotShoe and BugStop unless the
scenario sets `navicust` (side 1 has none). A scenario that means a panel to act on side 0, or a NaviCust bug to
take effect, sets side 0's `navicust` (as the `no-footing` and `static/bugs` scenarios here do) or uses side 1.
None of this section's other scenarios depends on a panel acting on side 0.

## What the lab's match counts did not cover: the cut-in chips' templates

Read every "verified" in this file, and every "the lab matches completely", with this in mind. Until the
completeness audit's follow-up, the scenario library's generated templates recorded almost nothing of a cut-in
chip. Their scripts end with `settle`, and both navis stand idle through a cut-in chip's telop and dimming, so
the recording stopped about 80 frames after the chip was used, before its navi or controller appeared. 670
recordings ended that way (completeness.md §10 has the list by template):

- of every navi chip and several other cut-in chips (62 chips): `hit`, `miss`, `adjacent`, `obstacle`, the eight
  `stage-*`, `beast`, `beast-charged`, `cross-*`, `atk10`, `navi20`, and most `counter-cut-in`;
- seven Program Advance recordings and a few hand-written scenarios that end on a cut-in chip.

The engine matched all 670, and 362 of them were `ok` in the lab's index. That match covered the chip's use and
the telop's first frames. It did not cover the chip's attack, its hits, its panels, its behaviour on a stage,
against an obstacle, in Beast Out or in a Cross, or its damage with Atk+10 and Navi+20. Before the fix those were
verified only where a scenario waited on its own: this survey's scenarios (the tables below name them) and the
templates in which the other side acts (`counter`, `guard`, `barrier`, `invisible`).

The lab's driver now waits out a cut-in, and the 670 are recorded to their ends. The engine matches every one on
every frame, so the tables below stand and the templates now cover what their names say. Two things remain true
of any recording and are worth checking before leaning on one: its last frame (a recording proves nothing past
it), and its status in the lab's index. 380 recordings were `unmet` until completeness.md §11's work: they ran to
their end without doing what their description said, usually because the chip can't reach the opponent from
where the template stood (ElecMan's and TomahawkMan's `hit`, a sword's `hit` from three columns away). The
templates now stand the navis where the chip lands, or say in their description and expectations what they show
(no damage out of reach; the use alone, with the scenario that records the hit), and none is `unmet`.

## Templates over every action handler family

Four templates run over the first damaging chip of each action handler family that the lab's `hit` or `adjacent`
scenario lands (the cannons, Vulcans, Spreaders, the swords, the bombs, each dimming subtype with damage, each
standard chip action…), whichever section below the family belongs to:

| template | what it reaches | status |
|---|---|---|
| `ko` (an opponent of 10 HP deleted by the first hit) | the objects' "the battle over" ends, the chip's later hits and spawns with the round decided | verified, 68 scenarios (43 families, 20 navi chips, the PAs' navis, SonicBom, Z Saver), and AirRaid1's (below) |
| `barrier` (the opponent behind Barr100) | the hits a barrier takes, the wind chips blowing it away (AirShot, WindRack, AirSpin) | verified, 39 scenarios, and AirRaid1's |
| `invisible` (the opponent under Invisibl) | no body to hit: the homing and searching chips' no-target paths (RskyHny's bee, MachGun's and the dragons' column search, ElcPuls) | verified, 39 scenarios, and AirRaid1's |
| `dimmed` (the opponent's AreaGrab cuts in a few ticks after the use) | the objects' waits while dimmed, the user's action held through a dimming, the press during a dimming that is no cut-in (`sub_8017AB4`'s clear) | verified, 34 scenarios |
| `paused` (the opponent presses START a few ticks after the use, and again) | the chip's action and objects held through a pause (only objects that run while paused take new branches: Sensor's laser) | verified, 35 scenarios |

## Dimming chips, shot chips and navi chips

### Dimming chips (dimming-chips.md)

| family | branch | why it was unreached | reachable | scenario and status |
|---|---|---|---|---|
| AntiNavi (the dimming service) | the turn, the bounce between two traps | scratch recordings were lost | yes | chips/0x0ba-antinavi/heatman, bounce: verified |
| AntiNavi | a chip that isn't a navi chip leaves the trap | no scenario | yes | chips/0x0ba-antinavi/not-a-navi: verified |
| AntiNavi | the target has its own controller registered (its AreaGrab, cut in on by the navi chip) | needs the navi chip as a cut-in | yes | chips/0x0ba-antinavi/own-controller: verified |
| AntiNavi | AntiNavi's user deleted (owner null) | the round is over by then | unreachable | |
| Navi chips' telop | the user without HP before its telop ends (deleted by the other side's cut-in) | | yes | chips/0x0e3-heatman/user-deleted: verified |
| Barriers (subtype 4) | ending an old visual (`sub_80E0DC0`) | no barrier raised over another | yes | chips/0x0b3-barr100/over-barrier: verified |
| Barriers | the visual's regrowth (action 4 back to 0) | BblWrap's bubble never popped | yes | chips/0x0b5-bblwrap/popped: verified |
| Barriers | action 8, a popped barrier blown away | no wind meets a barrier | yes | chips/0x0b3-barr100/blown-away, chips/0x0b6-lifeaur/weak-and-strong, the `barrier` template's wind chips: verified |
| Barriers | a barrier broken with damage to spare | no scenario | yes | chips/0x0b3-barr100/broken: verified |
| Barriers | the blind viewer; the visual hidden and shown with a user who vanishes for a chip | no blinded viewer; no chip behind a barrier | yes | chips/0x0b2-barrier/blind-viewer, chips/0x0e3-heatman/user-barrier, chips/0x08d-magnum/user-barrier, chips/0x06e-burnsqr1/user-barrier: verified |
| Barriers | the hidden-parts rule for type 9 on the remote side, the off-field owner | | hard | open |
| Panel chips (subtype 5) | kinds 1-3, 7-16, kind 9; a changer with no flag pointer | no chip, no caller | unreachable | |
| Panel chips | side 1's road swap | ComingRd and GoingRd only used by side 0 | yes | chips/0x0aa-comingrd/side1, chips/0x0ab-goingrd/side1: verified |
| Panel chips | a holy panel cracked from under its user in the same dimming | no cut-in on a panel chip | yes | chips/0x0a8-holypanl/cut-in-geddon: verified |
| BugFix (subtype 26) | a navi with parts | every lab user is MegaMan | yes (a link navi) | chips/0x0b0-bugfix/link-navi: verified |
| BugFix | a non-player navi's sprite, stat 0x21 = 0, variants 1 and 2 | no such user; no caller | unreachable | |
| Instruments (subtype 9) | the destroyed action (broken, and pushed) | no instrument broken or removed | yes | chips/0x092-fanfare/broken, pushed-by-enemy (side 1's AirShot knocks it back; side 0's own `pushed` moves none of its side's objects); chips/0x093-discord/broken, chips/0x094-timpani/broken, chips/0x095-silence/broken: verified |
| Instruments | the tune played to its end | scenarios end first | yes | chips/0x092-fanfare/lifetime: verified |
| Instruments | Fanfare's Beast Over test (both versions' Beast Over), and Beast Out for contrast | no Beast Over | yes | chips/0x092-fanfare/beast-over, beast-over-gregar, beast-shot-at: verified |
| Instruments | the battle-over branches (playing, resting) | no KO with an instrument out | yes | chips/0x092-fanfare/round-end, chips/0x095-silence/round-end-rest: verified |
| Instruments | removed (DustMan), blinking out (ColArmy), absorbed (DustCross) | no remover | yes | chips/0x092-fanfare, 0x093-discord, 0x094-timpani, 0x095-silence/{dustman, colarmy, absorbed}, chips/0x092-fanfare/absorbed-by-enemy, colarmy-by-enemy: verified |
| Instruments | a failed collision | pool full | unreachable | |
| AirRaid (subtype 13) | the plane shot down, the battle-over branch, the bombs against a barrier or no body | the plane is never hit | yes | chips/0x068-airraid1/broken, ko, barrier, invisible: verified |
| AirRaid | the plane removed, absorbed, blinking out | no remover | yes | chips/0x068-airraid1/dustman, colarmy, absorbed: verified |
| AirRaid | the plane's lifetime, AirRaid3's plane shot down, the bombs' panel list as the opponent walks | | yes | chips/0x068-airraid1/lifetime, moving-target, chips/0x06a-airraid3/broken: verified |
| Sensor (subtype 28) | the pushed turret, the broken turret | the turret is never hit or pushed | yes | chips/0x071-sensor1/broken, chips/0x073-sensor3/pushed (side 1's AirShot on Sensor3's 40-HP turret, `sub_80DA37A`; Sensor1's 20-HP turret breaks to it, and side 0's own shot in `sensor1/pushed` doesn't move it): verified |
| Sensor | the laser's re-arming | one firing per scenario | yes | chips/0x071-sensor1/twice: verified |
| Sensor | the battle-over branches | no KO | yes | chips/0x071-sensor1/ko: verified (the comparison now keeps a spark's garbage Z fraction after its laser is freed) |
| Sensor | the scanner blocked by an object, where it starts and on its step; the scanner off the top and bottom rows and the far column | the opponent always stands in its line | yes | chips/0x071-sensor1/blocked-rock, blocked-rock-far, row1, row3, long-miss: verified |
| Sensor | the turret removed, blinking out, absorbed; the scanner and the laser through a pause | | yes | chips/0x071-sensor1/dustman, colarmy, absorbed, paused: verified |
| Sensor | the scanner's Param2 0, failed collisions | no spawner; pool full | unreachable | |
| SumnBlk (subtype 36) | the whole navi (§9.2, §9.3) | no hole in front of the user | yes | chips/0x087-sumnblk1/hole-ahead, after-geddon, chips/0x089-sumnblk3/hole-ahead: verified |
| SumnBlk | the target search: the opponent in each row, near, in the back column, behind a RockCube (no target: the navi leaves); side 1's; the battle's end; SumnBlk2 | one opponent on its start panel | yes | chips/0x087-sumnblk1/hole-ahead-up, hole-ahead-down, hole-ahead-near, hole-ahead-back, hole-ahead-rock, hole-ahead-side1, hole-ahead-ko, chips/0x088-sumnblk2/hole-ahead: verified |
| ColorPt, DblPoint (subtype 27) | the bonus itself (080E66E0, 080E66EC, 080E66F6) | the next chip is none or has no damage | yes | chips/0x0c2-colorpt/bonus, chips/0x0c4-dblpoint/bonus, chips/0x062-lilbolr1/colorpt: verified |
| ColorPt | the special-source branch, a missing navi, `sub_800D53C` running off the field | no such user | unreachable | |

### Dimming chip effects (dimming-chip-effects.md)

| family | branch | why it was unreached | reachable | scenario and status |
|---|---|---|---|---|
| Wind, Fan (subtype 8) | a second fan replacing the first | one fan per scenario | yes | chips/0x081-wind/twice: verified (the second, from another row, evicts the first through the field-object registry, class 1, before it becomes its side's wind, so the wind registry's own replacement never runs: completeness.md §6.2). `then-fan` places no Fan: the panel in front is the Wind's; `both-sides` is one fan a side |
| Wind, Fan | the lifetime running out (1440 ticks) | scenarios end first | yes | chips/0x081-wind/lifetime: verified |
| Wind, Fan | Fan's fan broken; AirShot at it | only Wind's `counter` breaks one | yes | chips/0x082-fan/broken, pushed-by-enemy (side 1's AirShot knocks the 40-HP fan back; side 0's own `pushed` doesn't move it): verified |
| Wind, Fan | no gust (an obstacle on the row's first enemy panel; Fan's start a panel nearer) | no obstacle there | yes | chips/0x081-wind/no-gust, chips/0x082-fan/gust-past-bomb: verified |
| Wind, Fan | removal, blink-out, absorption | no remover | yes | chips/0x081-wind, chips/0x082-fan/{dustman, colarmy, absorbed}: verified |
| Wind, Fan | pushes (action 5), a flipped fan | AirShot breaks the fan; no player is flipped | hard / unreachable | open |
| BurnSqr (subtype 23) | A to fire | no scenario presses A | yes | chips/0x06e-burnsqr1/a-fires: verified |
| BurnSqr | a non-solid panel under the square | the timeout always falls on solid panels | yes | chips/0x06e-burnsqr1/a-fires-holes: verified |
| BurnSqr | the blind viewer; the user's barrier, confusion and blindness visuals, charge glow and aura while it is away | | yes | chips/0x06e-burnsqr1/blind-viewer, user-barrier, user-confused, user-blind, user-charging, user-full-synchro (and Magnum's and HeatMan's): verified |
| BurnSqr | a non-player user, the failed spawn | | unreachable | |
| GrabBnsh, GrabRvng (subtype 15) | the panel return, the strikes, the hand | nothing is stolen first | yes | chips/0x0a4-grabbnsh/after-areagrab, after-panelgrabs, chips/0x0a5-grabrvng/after-areagrab: verified |
| AreaGrab (subtype 0) | the stolen column going back under its thief; both sides stealing in one dimming | | yes | chips/0x0a3-areagrab/returns, both: verified |
| Subtype 2 | everything | no chip | unreachable | |
| Guardian (subtype 14) | breaking it: the crumble, the strike back, the strike's dimming and hit | the statue is never hit | yes | chips/0x097-guardian/punish, own-hit: verified |
| Guardian | AirShot at it, a second statue | | yes | chips/0x097-guardian/pushed, replaced: verified |
| Guardian | the stage statue (Param1 1), its strikes, absorption | | yes | the stage scenarios (stages/statue-…, stages/statues-stand): see field-objects.md |
| Guardian | the lifetime (6000 ticks), removal, blink-out, absorption | too long; no remover | yes | chips/0x097-guardian/lifetime, dustman, colarmy, absorbed: verified |
| Meteors (subtype 16) | the lists after area changes, a marker at battle end | no AreaGrab first, no KO | yes | chips/0x08b-meteors/after-areagrab, grabbed, ko: verified |
| Meteors | an empty list, a marker off the field, rows other than Param1 1 | the enemy always owns panels; no chip | unreachable | |
| Anubis, PoisPhar (subtype 17) | breaking by damage, AirShot at it, the lifetime, a second statue | the statue is never hit; scenarios end first | yes | chips/0x098-anubis/broken, pushed-by-enemy (side 1's AirShot knocks it back), lifetime, replaced: verified |
| Anubis | a non-solid landing panel, removal, blink-out, absorption | | yes | chips/0x098-anubis/hole-ahead, dustman, colarmy, absorbed: verified |
| Anubis | time up, an enemy with no panel for a bubble | needs the judge with a statue out; the enemy always owns panels | hard / unreachable | open |
| Anubis | the flipped user's registry store | no player is flipped | unreachable | |
| CircGun (subtype 19) | A to fire | no scenario presses A | yes | chips/0x08e-circgun/a-fires, a-fires-late: verified |
| CircGun | a start column of the user's own panels, shots on non-solid panels | the timeout's place | yes | chips/0x08e-circgun/after-areagrab, holes: verified |
| CircGun | a start column holding none of the enemy's home panels (the opponent's two AreaGrabs) | | yes | chips/0x08e-circgun/grabbed: verified |
| CircGun | Param3 1, a non-player first actor | no chip | unreachable | |
| Otenko (subtype 18) | the bonus, breaking, pushes | the next chip never does damage | yes | chips/0x099-otenko/bonus, broken: verified. No push: the statue takes hits and is never pushed (`sub_801AD6A`); `chips/0x099-otenko/pushed` is side 0's own shot, which doesn't move it, and side 1's AirShot hits it (100 HP to 80) without a move |
| Otenko | the statue removed, absorbed, blinking out | no remover | yes | chips/0x099-otenko/dustman, colarmy, absorbed: verified |
| Otenko | the lifetime (1800 ticks), a second statue, the blessing on two chips in turn | | yes | chips/0x099-otenko/lifetime, replaced, bonus-two-chips: verified |
| BlzrdBal (subtype 21) | a non-solid thrower panel, the roller's battle-over end, three swallows | | yes | chips/0x0c7-blzrdbal/no-footing, ko, three-rocks: verified |
| BlzrdBal | the excluded NameIDs, more than 4 hit objects | | hard | open |
| Magnum (subtype 24) | A to fire, the cursor's later rows | no scenario presses A | yes | chips/0x08d-magnum/a-fires, a-fires-late: verified |
| Magnum | a non-player user, panels off the field | | unreachable | |
| Geddon and the capsules (subtype 3) | Param1 0 (crack) and 2 (poison): records 0x17F-0x182, 0x186, 0x187 | no folder can hold them: the hand builder turns a record without codes into chip 0x185 | unreachable | chips/0x17f-prpcapsl/hit shows the 0x185: verified |
| Geddon | an empty list, a panel that changed between the list and its turn | | hard | open |
| Snake (subtype 12) | several holes (the 8/24-tick spacing), the tie-breaks | one hole at most | yes | chips/0x086-snake/after-geddon, after-geddon-miss: verified |
| Snake | side 1's scan | side 0 always uses it | yes | chips/0x086-snake/side1: verified |
| Snake | no target, a flipped user, a failed nest spawn | | unreachable | |
| LifeSync (subtype 7) | the offline path | not a netbattle's | unreachable | |
| NumbrBl (subtype 22) | a non-player user, the user deleted before the effect, no player for formula 21 | | unreachable | |
| CornFsta (subtype 29) | no panel for a burst, the free panels all in the ring, a failed spawn | | hard | open |
| DblHero (subtype 30) | the heroes with the battle over | no KO | yes | pa/0x158-dblhero/ko: verified |
| DblHero | the failed spawns, no enemy panel, no panel of another side ahead | | unreachable | |
| MetrKnuk (subtype 32) | the fists with the battle over | no KO | yes | chips/0x133-metrknuk/ko: verified |
| MetrKnuk | no enemy body (the fallback lists), no candidate, Param2 0 | the enemy always stands; no caller | unreachable | |
| DblBeast (subtype 37) | the user's-panel fallback; the patterns' first panels taken | a free panel always turns up | yes | chips/0x137-dblbeast/user-panel, rock-front: verified |
| DblBeast | no target at all, a failed spawn | an invisible navi still has its body on its panel (chips/0x137-dblbeast/invisible: verified, the usual attacks) | unreachable | |
| Gregar, Falzar (subtypes 34, 35; the Japanese ROMs: beast-chips.md) | every branch | no Japanese console recording yet (the US ROMs' slots are null) | yes, on a Japanese ROM | open: in-repo timelines only |
| Gregar, Falzar | the aim's whole-area list of one panel (its unchecked second byte), a failed spawn | a side keeps a whole column | unreachable | |
| Falzar's feather | Param2 0 (stops at the battle's end, holds while dimmed) | no caller spawns one | unreachable | |

### Traps, bombs and navi chips (chips.md §3.6-§3.9)

| family | branch | why it was unreached | reachable | scenario and status |
|---|---|---|---|---|
| Counter cut-ins (§3.6.5) | a cut-in by a chip of another subtype or action; a chain of four | the lab's counter cut-ins answer with the same chip | yes | chips/0x0a3-areagrab/cut-in-invisibl, cut-in-chain, chips/0x0dd-roll/cut-in-heatman, chips/0x0e3-heatman/cut-in-barrier: verified |
| Counter cut-ins | A during a dimming with a chip that doesn't dim next | | yes | chips/0x0b1-invisibl/cut-in-not-dimming, the `dimmed` template: verified |
| Counter cut-ins | cut-ins with Full Synchro, anger or a dark chip (the doubled damage, the substitute) | | yes | chips/0x0e3-heatman/cut-in-full-synchro, cut-in-anger, chips/0x08b-meteors/cut-in-full-synchro, chips/0x121-darkinvs/cut-in; chips/0x0e3-heatman/full-synchro, anger (no cut-in): verified |
| Counter cut-ins | a failed controller spawn | pool full | unreachable | |
| AntiRecv | the heal turned to damage, Roll's dimming taken over | scratch recordings were lost | yes | chips/0x0bd-antirecv/recov10, roll: verified |
| AntiRecv | Roll's damage with the double-damage flag; the trap taking over a cut-in | | yes | chips/0x0bd-antirecv/roll-full-synchro, roll-cut-in: verified |
| AntiRecv | a full effect pool | | unreachable | |
| AntiDmg | the trap sprung: the stars thrown back | the lab never hits a trap's user | yes | chips/0x0bb-antidmg/sprung, sprung-side0, small-hit, turn-end, replaced: verified (the counter is chips/antidmg/counter) |
| AntiDmg | sprung by a sword, a volley, a bomb, a flame, a charged shot, hits inside a dimming; both sides' traps in turn; the stars deleting their target | one Cannon | yes | chips/0x0bb-antidmg/sprung-sword, sprung-vulcan, sprung-minibomb, sprung-firebrn, sprung-charge-shot, sprung-heatman, sprung-meteors, sprung-twice, sprung-ko: verified |
| AntiSwrd | the trap sprung by a sword | | yes | chips/0x0bc-antiswrd/sprung, not-a-sword: verified (the counter is chips/antiswrd/counter) |
| AntiSwrd | the other swords: Sword, WideSwrd, FireSwrd, StepSwrd, MoonBld, ProtoMan's and SlashMan's slashes | one LongSwrd | yes | chips/0x0bc-antiswrd/sprung-sword, sprung-wideswrd, sprung-fireswrd, sprung-stepswrd, sprung-moonbld, sprung-protoman, sprung-slashman: verified |
| ElemTrap (§3.6.10) | the spring, the sparkles, the counterattack, the panel bursts | the lab never hits the trap with an element | yes | chips/0x0c5-elemtrap/sprung-fire, sprung-elec, null-hit: verified |
| BodyGrd (PA 0x157) | the trap itself, sprung by a shot, a sword and a navi chip | recorded only as its recipe | yes | pa/0x157-bodygrd/sprung, sprung-sword, sprung-heatman: verified (the counter is chips/bodygrd/counter) |
| IceCube (0x17C) | its record | no folder holds it | yes (save edit) | chips/0x17c-icecube/hit, pushed, broken, melted: verified |
| WhiCapsl (0x17E) | its dimming record | no folder can hold it (no codes: chip 0x185 instead) | unreachable | chips/0x17e-whicapsl/hit shows the 0x185: verified |
| Invisibl | shots and swords through an invisible navi | | yes | chips/0x0b1-invisibl/shot-at, the `invisible` template: verified |
| TimeBom (subtype 10) | the blast and its bursts | scenarios end during the countdown | yes | chips/0x090-timebom1/blast: verified |
| TimeBom | the bomb broken first (the puff), pushed, the battle's end | | yes | chips/0x090-timebom1/broken, pushed, round-end: verified |
| TimeBom, Mine, BlkBomb | removed, absorbed, blinking out | no remover | yes | chips/0x090-timebom1, chips/0x091-mine, chips/0x03c-blkbomb/{dustman, colarmy, absorbed}: verified |
| TimeBom | variants 2 to 7 | no slot pointer from the controller | unreachable | |
| Mine (subtype 11) | arming, blowing up when touched | scenarios end during the hops | yes | chips/0x091-mine/stepped-on: verified |
| Mine | its panel no longer solid, the battle's end | | yes | chips/0x091-mine/panel-broken, round-end: verified |
| RockCube | broken, pushed by either side, a third one (the field-object slots) | | yes | chips/0x08f-rockcube/broken, pushed, pushed-by-enemy, replaced: verified |
| RskyHny (§3.7) | the bee's end by battle over; a sting with no navi on the panel; steering in a column, reversing in a row; the destination fallbacks | the lab's bees always reach a standing target | yes | chips/0x025-rskyhny1/ko, invisible, moving-target, bee-shot: verified |
| RskyHny | params byte 1, the fade (action 8), the bee without a collision slot | no chip, no setter, pool full | unreachable | |
| Dragons (§3.8) | no enemy body ahead, either part's end at the battle's end, a blocked hit | | yes | chips/0x02e-heatdrgn/ko, invisible, barrier: verified |
| Bombs and seeds (§3.9) | a bomb ending at the battle's end | | yes | chips/0x036-minibomb/ko: verified |
| BlkBomb | set off by fire (its own side's, the other's), pushed, broken without fire, its lifetime, thrown at a hole | | yes | chips/0x03c-blkbomb/fire, enemy-fire, pushed, shot, lifetime, holes: verified |
| BugBomb | its other bug choices; landed and broken | one RNG seed | yes | chips/0x043-bugbomb/seed-1 … seed-4, landed: verified |
| FlashBomb | landed, broken before the flash | | yes | chips/0x039-flshbom1/landed, shot: verified |
| Bombs | bomb kind 1, seed kind 3, FlashBomb levels 3 to 8 | no chip | unreachable | |
| Navi chips (§3.6.7-§3.6.36) | no footing for the navi (action 0 to its leave) | the user always stands on solid ground | yes (AirShoes over a hole) | chips/…/no-footing for Roll, ProtoMan, HeatMan, ElecMan, SlashMan, EraseMan, ChrgeMan, SpoutMan, TmhkMan, TenguMan, GrndMan, DustMan, BlastMan, DiveMan, CrcusMan, JudgeMan, ElmntMan, Colonel, HackJack, Bass, BigHook, DeltaRay, BassAnly, CrossDiv: verified |
| Navi chips | the battle ending mid-attack | no KO inside a navi chip | yes | chips/…/ko for the same navis (ElecMan's and TmhkMan's from the adjacent column, the only one their strikes reach; none for HackJack, whose original stops), pa/0x15c-twinldrs/ko, pa/0x15d-crosover/ko, pa/0x15a-mstrcros/ko: verified |
| Navi chips | an opponent the navi can't find or reach, and one behind a barrier | the opponent always stands in the open | yes | chips/…/invisible, rock-front and barrier for each of the nineteen navi chips: verified |
| Navi chips | a pool with no free slot, a missing collision slot | pool full | unreachable | |
| Navi chips | the navi AI's variants (Param1 0 and the like): SpoutMan's, BlastMan's, ElecMan's, ChargeMan's, SlashMan's, DiveMan's, JudgeMan's whip | only the bosses' AI spawns them | unreachable | |
| GroundMan | the rock's non-solid landing, the rocks and the drill at the battle's end | | yes | chips/0x0fb-grndman/rockfall-holes, rockfall-after-geddon, rockfall-ko: verified |
| GroundMan | no rock candidate | the enemy's area always has a panel | unreachable | |
| DustMan | the junk's looks by NameID (the instruments, the turret, the fans, the statues, the bombs, the boiler, the doll: a look without a shadow, the none look), the excluded NameIDs (the mine) | he only ever took RockCubes | yes | chips/…/dustman for Fanfare, Discord, Timpani, Silence, Sensor1, Wind, Fan, Anubis, Guardian, TimeBom1, Mine, BlkBomb, LilBolr1, VDoll, AirRaid1, Otenko: verified |
| DustMan | NameIDs outside 0xCD..0xFF, the flag check after moving | no such object | unreachable | |
| JudgeMan | a book arriving at its target, leaving solid ground, ending with the battle | the books always hit | yes | chips/0x10a-judgeman/books-invisible, books-holes, books-ko: verified |
| JudgeMan | the heading's reversal, the target past the far edge (no enemy navi at all) | an invisible navi still has its body on its panel | unreachable | |
| TwinLdrs, CrosOver, MstrCros | no footing for their navis | | yes | pa/0x15c-twinldrs/no-footing, pa/0x15d-crosover/no-footing, pa/0x15a-mstrcros/no-footing: verified |
| TwinLdrs | ProtoMan leaving without a slash (the target's front panel taken) | | yes | pa/0x15c-twinldrs/rock-front, invisible: verified |
| TwinLdrs | more than one target; Colonel's target off the field | one enemy navi | unreachable | |
| CrosOver, MstrCros | a refused front panel, an opponent they can't see | | yes | pa/0x15d-crosover/rock-front, invisible, pa/0x15a-mstrcros/rock-front, invisible: verified |
| CrosOver | a link navi user, no Django | a link navi can't hold the PA | unreachable | |
| MstrCros | the Gregar tables (the beam, move 1) | the Falzar side always uses it | yes (the gregar base) | pa/0x15a-mstrcros/gregar: verified |
| Bass, BigHook, Darkness | failed spawns, a missing flag pointer, the strike's MachGun branches | | unreachable | |

### Shot chips (shot-chips.md §16)

| family | branch | why it was unreached | reachable | scenario and status |
|---|---|---|---|---|
| Cannons (0x14) | variant 3; a holder that isn't a player; a full pool | no chip | unreachable | |
| Vulcans (0x17), BatCan (0x19), TankCan (0x24), CornSht (0x2A), WideSht (0x30), Spreaders (0x25) | the battle-over end of the bullet, shot, shell, corn, wave and burst | no KO with one in flight | yes | chips/0x005-vulcan1/ko, 0x13b-batcan1/ko, 0x00c-tankcan1/ko, 0x040-cornsht1/ko, 0x017-widesht/ko, 0x009-spreadr1/ko: verified |
| TankCan | a wait of more than a tick, Param2 0, an explosion row of 4 | no chip | unreachable | |
| MachGun (0x29) | the target column moving; the no-body fallback | the opponent stands still | yes | chips/0x02b-machgun1/moving-target, ko, invisible: verified |
| WideSht | the wave's kinds 0-2 and 9 | dead code | unreachable | |
| Sonic booms (0x55) | SonicBom, SprSonic, Curse, Punisher as chips; the boom ending on an obstacle, piercing one, guarded, with the battle over | the variable swords' picks, reached at random only (Beast Over); no folder holds the records | yes (save edit) | chips/0x173-sonicbom/{hit, adjacent, miss, obstacle, guard, ko}, chips/0x177-sprsonic/{hit, adjacent, miss, obstacle}, chips/0x174-curse and chips/0x175-punisher/{hit, adjacent, miss}: verified |
| Z Saver (0x5B) | everything, with the fourth slash's command | weapon 0x6E; no folder holds the record | yes (save edit) | chips/0x17d-zsaver/{hit, adjacent, miss, fourth-slash, command-late, command-split, ko}: verified |
| Rapid buster (0x5D) | everything | weapon 0x39: no navi or form of a netbattle has it | unreachable | |
| LilBoiler | the eruption | | yes | chips/0x062-lilbolr1/erupt-hits, chips/0x063-lilbolr2/erupt-lifetime: verified |
| LilBoiler | a bonus (the port took the registry's side from it and stopped with an error for Atk+10) | no scenario gives it Atk+ | yes | chips/0x062-lilbolr1/atk10, atk10-twice, atk10-cross, atk10-cross-gregar, then-cross: verified (the boiler registers by its user's side whatever the bonus: shot-chips.md §18) |
| LilBoiler | the registry side of a boiler thrown by side 1 | side 0 always throws it | yes | chips/0x062-lilbolr1/side1-then-fan, side1-own-fan: verified |
| LilBoiler | the hole, AirShot at it, a RockCube after it | | yes | chips/0x062-lilbolr1/holes, pushed, replaced: verified |
| LilBoiler | removed, absorbed, blinking out | | yes | chips/0x062-lilbolr1/dustman, absorbed, colarmy: verified |
| VDoll | the curse | | yes | chips/0x096-vdoll/curse: verified |
| VDoll | the lifetime's end, AirShot at it, its own side's hit | | yes | chips/0x096-vdoll/lifetime, pushed, own-hit: verified |
| VDoll | removed, absorbed, blinking out | | yes | chips/0x096-vdoll/dustman, absorbed, colarmy: verified |
| The variable swords' picks (action 0x13 variants 9-11) | FtrSword, CrosSwrd, DblDream | reached at random only | yes (save edit, and the commands) | chips/0x172-ftrsword, chips/0x176-crosswrd, chips/0x178-dbldream/{hit, adjacent, miss}: verified |
| GunDelEX (0x012) | the fourth GunDelSol | class Spec | yes (save edit) | chips/0x012-gundelex/{hit, adjacent, miss}: verified |

## Standard chips, instant chips, field objects and stages

The first batch of the survey's scenarios covered the rows below; the rest of these documents' unverified lists
(standard-chips.md, field-objects.md, the instant chips, the stages) is still to be surveyed here.

### Standard chips (standard-chips.md)

| family | branch | why it was unreached | reachable | scenario and status |
|---|---|---|---|---|
| VarSwrd | every command sequence (LongSwrd, FtrSword, WideSwrd, LifeSrd, SonicBom), the timeout, a command broken off, wrong, slow, too slow, by side 1 (the mirrored sequences), in Beast Out | no scenario holds A and enters a command (the driver now can: `keys`) | yes | chips/0x052-varswrd/cmd-longswrd, cmd-ftrsword, cmd-wideswrd, cmd-lifesrd, cmd-sonicbom, cmd-timeout, cmd-broken-off, cmd-wrong, cmd-slow, cmd-too-slow, cmd-longswrd-side1, cmd-longswrd-far, cmd-sonicbom-far, cmd-lifesrd-beast: verified |
| NeoVari | every command sequence (CrosSwrd, SprSonic, DblDream), the timeout | | yes | chips/0x053-neovari/cmd-crosswrd, cmd-sprsonic, cmd-dbldream, cmd-timeout, cmd-sprsonic-far: verified |
| VarSwrd, NeoVari | the random pick; the charged sword (0x41) | Beast Over; SlashCross and the Beast's charge | yes | chips/0x052-varswrd/beast-over, chips/0x053-neovari/beast-over, beast-charged: verified |
| MoonBld | the repeat swings | no trigger found in a netbattle | check | open |
| CopyDmg | the time-up path; an obstacle as the target (NameIDs 0xCD..0xFF) | | yes | chips/0x0be-copydmg/time-up, on-rock, rock-hit: verified |
| RlngLog | the log's stop and break; no ground under it | no shot meets a log | yes | chips/0x028-rlnglog1/shot (the Vulcan breaks a log), buster-stops (a buster hit stops a landed log for 60 ticks, `sub_80D141A`), holes: verified |
| RlngLog | the drop-in path | | check | open |
| Static | the larger spreads (the user's NaviCust bug kinds) | no bugged user | yes | chips/0x035-static/bugs-1, bugs-2, bugs-3: verified |
| Tornado | subtype 3 | no chip record gives it (the chips are subtypes 1 and 2) | unreachable | |
| AirSpin | the whirlwind shot down, AirShot at it | | yes | chips/0x07e-airspin1/shot, pushed: verified |
| AirSpin | the top absorbed (and thrown), blinking out, removed | it spins for 40 ticks: the opponent has to act while it does | yes | chips/0x07e-airspin1/dust-absorb (the opponent's DustCross's B+Back), vanished (the opponent's ColArmy), swallowed (the opponent's BlzrdBal cuts in, and its ball finds the top held by the dimming): verified |
| AirSpin | variant 1 | no chip | unreachable | |
| H-Burst | the fight-over branch | | yes | pa/0x152-h-burst/ko: verified |
| H-Burst | the whole-field branch | | check | open |
| WideBrn, ParaShl, GreatYo, PitHocky | the Program Advances past the banner | stopped at the banner when written | yes | the recipes (pa/…): verified by the lab since the banner was ported |
| WindRack | variant ≠ 0, the gust's other spawners | no chip | unreachable | |
| Needles, pulses, aura heads, thunder dolls | three or more enemies, looks 2 and 3, speed tables past 1, Param1 ≠ 0 | the viruses' and the AI's | unreachable | |
| Shock wave | the virus variants 0..0xB | | unreachable | |
| Link navi chips (action 0x0A) | HeatPres off solid ground, DElecSwd's stops, RSlash's turns, EDeletBm's cap, VolcChrg's panels, DripShwr's jump, ETomahwk's cap, RC Brakr's rockfall | one scenario per navi | yes | open (action 0x0A is being ported) |

### Field objects (field-objects.md, objects-and-player.md)

| family | branch | why it was unreached | reachable | scenario and status |
|---|---|---|---|---|
| Obstacles | pushes (`sub_8017CC0`, `sub_800F598`, `sub_8017CE0`, `sub_8017D64`, `sub_8017E0A`) | nothing is pushed | yes | chips/0x090-timebom1/pushed, chips/0x03c-blkbomb/pushed, chips/0x062-lilbolr1/pushed, chips/0x08f-rockcube/pushed-by-enemy, chips/0x073-sensor3/pushed, chips/0x092-fanfare/pushed-by-enemy, chips/0x098-anubis/pushed-by-enemy, chips/0x082-fan/pushed-by-enemy, chips/0x17c-icecube/pushed: verified (a side's own AirShot doesn't move the objects it sets on its own panels, so Guardian's, Sensor1's, Fanfare's, Anubis's, Fan's and Otenko's `pushed` show no push) |
| Obstacles | the pushes' ice and bounds branches | | yes | stages/ice-50-rockcube-airshot, iceblocks-2c-airshot, ice-52-icecube, fan-airshot-ice (over ice), chips/0x077-lance/stage-ice (a pull stops at the edge of the puller's area), chips/0x023-elcpuls2/obstacle: verified |
| Obstacles | the field-object slots: a third class-0 object, a second class-1 one | | yes | chips/0x08f-rockcube/replaced, chips/0x098-anubis/replaced, chips/0x062-lilbolr1/replaced: verified |
| Obstacles | thrown and encased (`sub_8018002`, `sub_801813A`) | nothing in the game starts them | unreachable | |
| Obstacles | actions 3/4/6/7, a push without direction bits | nothing sets them | unreachable | |
| Flying shot | kinds 2 and 5, the wait (TrnArrw, the Beast buster) | | yes | chips/0x018-trnarrw1/ko, barrier, invisible, dimmed: verified |
| Projectile | the bursts (Param1 0xC), climbing (Param1 0x1D) | | check | open |
| Target tracking (`sub_802EF74`) | battle flag 0x40 | never in netbattles | unreachable | |

### Field objects and stages: the third agent's batches

Scenarios named here are in the verification workspace's chiplab library (`stages/…`, `forms/…`, `chips/…`).
"Matches" means every frame of the recording. The library's README says how a scenario's seed picks its stage
(the battle settings record): every record below is reached by a computed seed.

### Field objects and stages: covered

| Branch | Scenarios | Result |
|---|---|---|
| Every battle settings record of a link battle (match type 1, records 0x00 to 0x5F) whose actor list places objects: rock cubes 0x02 to 0x07 and 0x5C to 0x5F (the poison stage), boulders 0x16 to 0x1B, ice blocks 0x28 to 0x2D and 0x50 to 0x53, Guardian statues 0x54 and 0x55: both orders of each list (side 1's navi first), every position | `stages/rocks-02-cannons` to `rocks-07-drilarm-side1`, `boulders-17-fan` to `boulders-1b-airshot-side1`, `boulders-stand`, `boulders-corners`, `iceblocks-28-thunder` to `iceblocks-2d-dust`, `ice-50-rockcube-airshot`, `ice-52-icecube`, `ice-53-dust`, `statues-stand`, `statues-55-airshot-side1`, `poison-5c-stand` to `poison-5f-tornado`; 0x51 is the chips' `stage-ice`, 0x2A the soundmod trace's | match |
| A rock broken by damage: the cube (debris palette 0, its sound) and the ice block (palette 1, its sound); by a breaking hit, from side 1 too | `rocks-02-cannons`, `iceblocks-2b-cannons`, `rocks-07-drilarm-side1` | match |
| Elements on an ice block (aqua): elec doubles, fire doesn't | `iceblocks-28-thunder`, `iceblocks-29-firebrn` | match |
| The bounded push (`sub_8017E26`): a rock or boulder slides until another obstacle is next; from side 1 (mirrored); to the field's edge (WindRack's gust); over ice panels as a rock (a panel more) and as an ice block (aqua: none); into a navi, which crushes the rock and hurts the navi | `rocks-03-airshot`, `boulder-airshot`, `boulders-1b-airshot-side1`, `rocks-06-windrack`, `iceblocks-2c-airshot`, `ice-50-rockcube-airshot`, `ice-52-icecube` | match |
| The knock-back (`sub_8017CC0`, the fan's and the statue's table entry): a fan hit by AirShot with a navi behind it (blocked: it rests where it is); a stage statue hit by AirShot (the damage breaks it first, and it strikes back) | `fan-airshot`, `statues-55-airshot-side1` | match |
| The registry: a third class-0 object on a side evicts the oldest, the stage's rock (HP 0: it breaks into debris); Guardian's statue evicts the stage's (class 1) | `rocks-05-rockcube-evict`, `statue-guardian` | match |
| RockCube on an occupied panel (the rock breaks as it stands); CrakShot at a rock's panel (cracked, nothing flies) | `boulder-rockcube`, `poison-5d-crakshot` | match |
| A rock's lifetime: 6000 ticks counted outside dimmings and pauses, blinking its last 180, then broken | `rockcube-lifetime` (6,700 frames) | matches |
| IceCube (chip 0x17C, which the library doesn't have: the lab puts it in the folder) | `ice-52-icecube` | matches |
| Absorption (DustCross's B+Back): rock cubes, ice blocks (their animation), boulders (the stage slots), statues, a TimeBom's bomb (it keeps its hidden digits; thrown, its parts keep their own facing); each thrown back | `rocks-04-dust`, `iceblocks-2d-dust`, `ice-53-dust`, `boulder-dust-absorb`, `statue-dust-absorb`, `timebom-dust-throw` | match |
| An absorption interrupted: the navi flinches out of the action while the obstacles fly; they fly on, their timer stopped, and puff off the field | `absorb-interrupted` | matches |
| The boulder (field-objects.md §3.1): its spawner's flags on columns 2 and 5, standing through the round's start, hits, breaking, a breaking hit, the push, the stage slots | `stages/boulder*` (ten scenarios) | match |
| The boulder leaving by a chip: blinking out (ColArmy makes both sides' field objects vanish: 20 ticks in its destroyed action), swallowed by BlzrdBal's ball (removed: the puff); DustMan with boulders on the field | `boulder-colarmy`, `boulder-blzrdbal`, `boulder-dustman` | match |
| The stages' Guardian statues: standing (still running while paused), struck by either side (it strikes back at the attacker), absorbed, evicted, blinking out | `stages/statue*` (eight scenarios) | match |
| The knock-back with room behind: the fan slides a panel and rests | `fan-airshot-free` | matches |
| Panel changes under obstacles: Geddon's cracks under boulders; a Tornado on a rock; a BigBomb's blast | `boulders-18-geddon`, `poison-5f-tornado`, `boulders-19-bigbomb` | match |
| GroundCross's falling rocks (attack #0x1D) over the holes stage: a rock over a hole puffs, one over a panel shatters | `forms/falzar/cross-ground-charged-holes` | matches |

| A navi's absorbed list full: nine obstacles over two turns; the ninth tries on its 9th and 10th tick, finds eight in the list, and flies on (its timer wrapped) until it puffs off the field once the navi's action ends | `absorbed-list-full` | matches |
| The push from both sides at once: two AirShots landing on one RockCube on the same tick (no push), and a tick apart either way (two pushes) | `rock-airshots-both-together`, `-early`, `-late` | match |
| The knock-back over ice panels (its ice branch) | `fan-airshot-ice` | matches |
| A pushing hit that breaks the Fan (40 HP) before it can move it: WindRack, ElcPuls2 | `fan-windrack`, `fan-elcpuls2` | match |
| GroundCross's falling rock on a navi (its hit branch) | `forms/falzar/cross-ground-charged-hit` | matches |
| The panels acting on side 0 with an empty NaviCust (the falzar base's side 0 has AirShoes, FlotShoe and BugStop, so no other scenario shows them on side 0): a walk over its nine panels on each stage (sliding on ice, holes left on cracked panels and coming back, holes blocking the way, poison and lava, the roads carrying it); standing on each stage's panels hit by aqua, elec, fire and null chips (fire doubled on grass, everything halved on holy, FireBrn cracking the panels it burns and so ending the poison or the halving); pushed back by AirShots (onto lava, along a road, against a hole) | `stages/panels-{grass,ice,poison,volcano,holy,cracked,holes,roads}-walk`, `panels-{grass,ice,poison,volcano,holy,cracked}-hit`, `panels-{grass,ice,poison,volcano,holy,cracked,roads}-airshot`, `poison-5c-bare`, `ice-50-bare` | match |
| Chips that change the panels under side 0 (empty NaviCust): the opponent's GrasSeed and a doubled FireBrn, IceSeed (sliding, WideSht), PoisSeed, Geddon (every panel but the navis' breaks, one every 8 ticks; side 0 can't step off), ComingRd and GoingRd (carried along the roads); its own Snctuary and HolyPanl (Cannons halved) | `panels-grasseed-fire`, `panels-iceseed-walk`, `panels-poisseed-stand`, `panels-geddon-walk`, `panels-comingrd-walk`, `panels-goingrd-walk`, `panels-snctuary-hit`, `panels-holypanl-hit` | match |

### Instant chips, the Cross special and the Cross Beasts: covered

| Branch | Scenarios | Result |
|---|---|---|
| The instant chips whose effects spawn objects (`off_80EC3F0`: SandWrm1, FireHit1, Boomer, Lance, GolmHit1, JustcOne, ColArmy): used by side 1 (the mirrored paths), with the opponent a row off, in its back column, and behind its own RockCube; on the holes, cracked and ice stages; deleting an opponent of 10 HP moved into the chip's reach (the objects with the battle over); against Barr100; through the opponent's AreaGrab dimming | `chips/0x065-sandwrm1/{side1, row1, row3, far, enemy-rock}`; the same and `stage-holes`, `stage-cracked`, `stage-ice`, `ko`, `barrier`, `dimmed` for `0x06b-firehit1`, `0x074-boomer`, `0x077-lance`, `0x078-golmhit1`, `0x08c-justcone`, `0x0c6-colarmy` (71 scenarios) | match |
| The boomerang thrown by side 1 (flying left, its turns mirrored); the lance pulling an ice block along the back row to the edge of its user's area | `chips/0x074-boomer/side1`, `chips/0x077-lance/stage-ice` | match |
| GolmHit's fist: it lands on the other side's nearest panel of the row wherever the navi stands (column 4 with the opponent in its back column); the battle over while it stands; landing on a hole (its one hit, the puff) | `chips/0x078-golmhit1/far`, `ko`, `hole` (after the user's Geddon) | match |
| ColArmy with obstacles on the field (a RockCube of the user's first): its soldier facing the navi ahead, turning around for a navi behind it on its row (the draw), standing through a dimming, its shots deleting the opponent | `chips/0x0c6-colarmy/*` (eleven scenarios), `turns-around` | match |
| ColForce to the force's last look (a look that finds no panel ends it), against the user's and the opponent's RockCube, with the battle over, and from side 1 (a Falzar side holding Gregar's chip: the lab's bases have no Gregar side 1) | `chips/0x130-colforce/obstacle`, `enemy-rock`, `ko`, `side1` | match |
| The Cross special (DarkInvs's auto-battle): the chip rows for a base max HP of 100 to 900, each used through its record | `chips/0x121-darkinvs/hp-100` to `hp-900` | match |
| GunDelEX's wide beam on an opponent a row off | `chips/0x012-gundelex/wide` | matches |
| The Cross Beasts' charged Null chips, all ten (Heat, Elec, Slash, Erase, Charge; Spout, Tomahawk, Tengu, Ground, Dust), with the opponent on the user's row and a row off it: 307 blocks no recording had reached, among them the tomahawks (the boomerang's other variant: its sprite, its leg down a column) | `forms/gregar/cross-{heat,elec,slash,erase,charge}-beast-charged`, `forms/falzar/cross-{spout,tomahawk,tengu,ground,dust}-beast-charged`, each also `-row1` | match (Ground after the fix below) |
| A Cross Beast charging a chip of its family | `forms/falzar/cross-{spout,tomahawk,tengu,ground,dust}-beast-family-charged` | match |

Fixed from these: GroundCross Beast's dash spawns two hits; the original loads the height (24 pixels) for the
first only, and the second takes what the spawner leaves in that register, the element (a fraction of a pixel).

### Field objects and stages: not reachable in a netbattle (documented, no scenario)

| Branch | Why |
|---|---|
| The rock's `fall` entrance (`sub_80CFAC0`) and its rows 0 (1 HP) and 2 (500 HP) | `sub_80CFBC4` has three callers: the actor lists' type 8 (entrance 3; the lists name rows 1 and 3 only), RockCube's controller (the chip's parameters: RockCube row 1, IceCube row 3, both rising) and the encased obstacle (row 3, instant). No chip record or list gives entrance 2 or rows 0 and 2. |
| The boulder's spawn flags on columns other than 2 and 5 | Every list places its boulders on those columns, and the flags are only read at the spawn. |
| The thrown and encased obstacles (`sub_8018002`, `sub_801813A`; field-objects.md §4.5) | Nothing calls the routines that request them. |
| An obstacle's actor reactions (its table's entries 3, 4, 6 and 7: flinch, paralysis, ice, bubble) | They dispatch through actor data an obstacle doesn't have; nothing sets them on one. |
| The actor lists' entry types 1, 2, 6, 7 and 0xA; battle settings records 0x60 and up | No link battle's stage has them (match type 1 draws from 0x00 to 0x5F; types 0 and 2 need another match type, and name the same lists). |
| The rock's and the boulder's "no collision slot" paths, an absorbed obstacle with no navi to fly to | A full pool; the side's first actor is always the player's navi. |

| The pull vectors on an obstacle (`byte_800F604`'s rows for hit modifiers 0x04 and 0x10 with the push bit): six panels toward the pusher | Fan's gust has hit modifier 4 without the push bit (0x40) and starts no slide (`boulders-17-fan`); the one attack with both is the numbered shot 0x20, a charged shot kind no NaviCust routine writes. (The one-panel pull is ElcPuls2's and the lance's: covered.) |
| The knock-back's vectors other than AirShot's (the fan's and the statue's table entry reads the vector's panels) | A Fan has 40 HP and a stage statue breaks to the first hit: every pushing chip but AirShot (20) destroys it with the hit that would move it (`fan-windrack`, `fan-elcpuls2`). |
| A falling rock's counter decrement (`sub_80C7E24`) | GroundCross's barrage passes no counter. |
| SandWrm's hole waiting open (`sub_80BC958`), its start hole's timer running out during the arc | The worm closes each hole 20 ticks after it opens, the tick its opening animation (4 frames of 6 ticks, begun a tick late) would hand over to the wait; the arc is 16 ticks, shorter than the timer. Paused or dimmed both stand still, a held navi only slows the hole, and with the battle over both go at once. No SandWrm recording (60 of them) reaches either. |
| GolmHit's fall-back column (no panel of the other side's on the row), its cracks (Param3, Param4), a fist on its own panel (Param2) | A side always keeps a full column (the grab shots don't take the last one); no chip record sets the parameters. |
| The boomerang's strong hit (Param4), the lance's second palette (Param1) | No thrower and no chip record ask for them. |
| ColArmy's soldier spawned off the field | It takes an obstacle's panel. |
| The "no collision slot" paths of the worm, the boomerang, the soldiers and the golem's hits | A full pool. |
| Thunder's ball with a bug (its fourth parameter) | Thunder's and DarkThnd's records give 0. |
| The recovery chips' tenth heal row (no target) | No chip record names it. |

### Open

- Field objects: a pushing hit other than AirShot's on a fan or statue that doesn't break it (none found: see
  the table above).
- Instant chips (`off_80EC3F0`): FullCust's and the plus chips' special-source branches (not reachable:
  per-player gauges and the special chips are not a netbattle's); ColForce from a real Gregar side 1 (the
  lab's bases have none; `chips/0x130-colforce/side1` gives a Falzar side the chip). Effects 2, 6, 9, 11, 16
  and 17 are called only by the link navis' weapons (not surveyed here).
- The chips' charged shots beyond their scenarios: the arm chips' in a Cross or Beast Out (the two bug chips' are
  recorded there); a second navi's hit ending one mid-attack (the actions' own flinch paths are their chips');
  the bug frags running out between two players' uses (each side has its own count).
- The link navis' charged attacks beyond the one scenario each (standard-chips.md, "Action 9", lists them):
  HeatMan with no floor ahead, ElecMan's bolts with no enemy on a panel, GroundMan's drills stopped by a hit,
  off the field or orphaned by a flinch, the higher Charge levels and buster Attack. All reachable with a link
  navi scenario; none written. Verified since: ChargeMan stopped where the floor ends, his cars' burst
  (`navis/navi-05-volcchrg/charge-hole-ahead`, `-far`, `-next`, on the holes stage), and ProtoMan's other B+Back
  special, weapon routine 0x34 (`navis/navi-11-stepswrd/level-5`: the game gives him that shield below level 10
  and the reflecting one, 0x30, from level 10; the lab's link navis are level 14, as Tango's save editor makes
  them, so the scenario pokes the save's level index and the stats that level gives).
- The gregar base's side 0 has SuprArmr, UnderSht, AttckMAX, ChargMAX and HP+1200 in its NaviCust, so Gregar's
  Cross and Beast scenarios never show side 0 flinching: the same blind spot as the falzar base's shoes, not
  surveyed yet.

## Ruleset

Scenarios named here are in the verification workspace's chiplab library; `flow/` and `custom/` are new folders
for the fight's states and the custom screen. "Matches" means every frame of the recording.

### Covered

| Branch | Scenarios | Result |
|---|---|---|
| The supports (chips.md §2.10): Rush's bite, Beat's theft of a Mega and of a Giga chip, Tango's heal and its barrier, each hosted by either side, once a battle | `navicust/rush`, `rush-side1`, `beat`, `beat-side1`, `beat-giga`, `tango`, `tango-side1` | match |
| The support bug (NaviStats+0x0D = 0xFF): no support comes | `navicust/bug-support` | matches |
| The save's event flag 0x1720 (a NaviCust bug ran: the emotion window flickers, a console RNG1 draw each time) | carried in the trace's setup (`emotion_window_glitches`); `navicust/bug-support`, `bug-hp` | the console's RNG1 keeps step |
| Every NaviCust program without its bug (the earlier recordings had each part on the grid's outer ring, which bugs it) | all of `navicust/` recorded again, plus `poem`, `fldrpak1`, `fldrpak2`, `hp50`..`hp400`, `bugstop` | match |
| The NaviCust bugs by level: panel 1-3, custom 1-3, HP 1-3, buster 1-3, movement, emotion, status, a part on the outer ring | `navicust/bug-*` | match |
| Steps gone astray (the movement bug) toward every edge, with and without a panel to go to | `navicust/bug-movement-edges` | matches |
| FlotShoe on every stage (grass, ice, poison, volcano, holy, cracked, holes, roads) and AirShoes over holes, cracked, ice and poison panels | `navicust/flotshoe-stage-*`, `airshoes-stage-*` | match |
| The four bugs a BugBomb gives (bug codes 0x18, 0x19, 0xF5 and stat 0x14), one after another on one navi, and the navi living with them (blank shots, the HP drain, the panel trail, the custom screen's drain) | `chips/0x043-bugbomb/four` | matches |
| An uninstall (bug code 0xF8) landing on BodyPack's programs (UnderShirt stays), on a navi in Beast Out (the form's shoes come back), on a link navi (it keeps what it has), and on UnderShirt at 30 HP | `chips/0x0b9-uninstll/folded`, `folded-beast`, `folded-navi`, `folded-undershirt` | match |
| An uninstall on a navi in each Cross and Cross Beast of both versions and in Gregar Beast Out: each form's NaviCust refresh (`sub_801469C`'s table: its `navicust_refresh`) gives its flags back | `chips/0x0b9-uninstll/folded-cross-{spout,tomahawk,tengu,ground,dust,heat,elec,slash,erase,charge}` and each `-beast`, `folded-beast-gregar` | match |
| An Uninstll'd Cannon at a navi in Beast Over: nothing lands (Falzar Beast Over untouchable, Gregar's invulnerable), so nothing is uninstalled; the Falzar navi has no shoes of its own, and the form gives it AirShoe and FloatShoe | `chips/0x0b9-uninstll/folded-beast-over`, `folded-beast-over-gregar` | match, since the fix below |
| A Cross chosen on a later screen while in Beast Out (the Beast's Cross: `sub_80153EC`, `sub_801544C`) | `forms/falzar/beast-then-cross` | matches |
| The Beast rush chaining the next chip on an A press (it runs inside the rush), and not chaining a variable sword, a dimming chip or an empty hand (`sub_800FC30`) | `forms/falzar/beast-rush-chain`, `-sword`, `-varswrd`, `-dimming` | match |
| The dark chips with no BugFrags: each is its substitute (DrkSword's Sword, DarkThnd's Thunder, DrkRecov's Recov10, DarkInvs's Invisibl, DarkPlus's Atk+10: `sub_8010D58`, `sub_800EF02`); two DrkSwords with one frag | `chips/0x11e-drksword/no-frags`, `last-frag`, `chips/0x11f-darkthnd/no-frags`, `0x120-drkrecov/no-frags`, `0x121-darkinvs/no-frags`, `0x122-darkplus/no-frags` | match |
| Full Synchro's aura hidden while its navi is away for BugFix's glow, and shown again; BugFix does no damage, so the synchro stays for the next chip (`sub_80C4C46`, `sub_80C4C4C`) | `flow/synchro-bugfix` | matches |
| The AntiDmg program's B+Back stance catching a Cannon, and a Sword (AntiDmg's counter either way: `sub_80105F2`; the shuriken at a random enemy, `sub_8016004`) | `navicust/antidmg-caught`, `antidmg-caught-sword` | match |

Fixed from these: Falzar Beast Over's form flags (`sub_8014674`) are one literal, 0x08000030 (the disassembly
renders it as a pointer): AirShoe and FloatShoe with the untouchable flag. The port had the untouchable flag
alone, which no recording showed, since every earlier Falzar Beast Over navi had its own AirShoes and FlotShoe.

| START pause (battle-flow.md §3.5): only the pausing player resumes; both on one tick (side 0 pauses); during a dimming and once the battle is over (nothing) | `flow/pause`, `pause-both`, `pause-dimmed`, `pause-over` | match |
| A screen confirmed with nothing picked keeps the hand's chips; only Beast Out picked empties it | `flow/keep-hand` | matches |
| The custom screen opened while an attack is under way (a Vulcan mid-burst), and by both players on the tick the gauge fills (L and R) | `flow/custom-open-busy`, `custom-open-both` | match |
| Anger from a 300-damage hit (the next chip doubled, the anger spent) and its 600 ticks running out; anger from a counter's paralysis | `flow/anger-big-hit`, `anger-runs-out`, `synchro-lost` | match |
| Full Synchro from a counter hit: the next chip doubled and the synchro spent; lost to a plain hit | `flow/synchro-used`, `synchro-lost` | match |
| A counter landed by a navi in a Cross (the banner and the paralysis, no Full Synchro: the next chip does plain damage) and in Beast Out (Full Synchro; the next Cannon does 140) | `flow/counter-in-cross`, `counter-in-beast` | match |
| Counter hits by chip (field-collision-damage.md §4.9): of the 245 chips with a generated `counter` scenario (the user answers the opponent's MiniBomb), 40 land in the window; for 69 more the opponent's MiniBomb is timed so that the chip's hit lands 8 ticks into its window, and the Counter banner comes up (the cannons, the bombs, the seeds, the swords, the boomerangs, Tornado, Static, ElcPuls, Thunder, RskyHny, RlngLog, MachGun, LilBolr, Sensor, GolmHit, AirSpin, Lance, ColArmy, VDoll, BigBomb, the dark sword and thunder; Rflectr's shot coming back; ElemSwrd and AssnSwrd on grass; Muramasa after a Cannon) | `chips/*/counter-hit` (81 scenarios with the two rows below, `tools/chiplab/gen_counters.py` in the verification workspace) | match |
| A hit in the window that doesn't counter: AirRaid's shots, JustCone, DarkInvs's strikes, Anubis's poison, TimeBom1's blast, ColForce's shots (which paralyze all the same) | `chips/0x068-airraid1`, `0x069`, `0x06a`, `0x08c-justcone`, `0x121-darkinvs`, `0x098-anubis`, `0x090-timebom1`, `0x130-colforce` `/counter-hit` | match (no banner) |
| A counter hit that doesn't leave 150 ticks of paralysis: BblStar's bubble and DrilArm's drag take its place; the attack's next hit ends it with a flinch (AquaNdl's second needle 8 ticks on, EnergBom's third blast 14 ticks on) | `chips/0x01b-bblstar1`, `0x033-drilarm`, `0x03d-aquandl1`, `0x037-energbom` `/counter-hit` | match |
| A hit during a dimming on a navi whose window the dimming holds open (the navi chips, Meteors, BurnSqr, BlzrdBal, DblBeast, MetrKnuk, AntiDmg: 40 chips): no counter | their `counter` scenarios | match |
| MchnSwrd's strike on a paralyzed navi (chips.md: no scenario reached the hit) | `chips/0x056-mchnswrd/paralyzed` | matches |
| A navi deleted by a navi chip's hit during its dimming | `flow/ko-dimmed` | matches |
| A double KO on one tick (§3.6: the round goes by the alive counts, side 0 tested first) | `flow/double-ko` | matches |
| The turn timer from the 15th screen, the damage judge (state 0x18) and its three outcomes; the draw state 0x14 and result code 3 | `flow/judge-win`, `judge-lose`, `judge-draw` | match (10,600 frames each) |
| A lost round (result code 2) | `forms/falzar/ko-lose` | matches |
| Deleted in Beast Out, and at 0 HP in a Cross | `forms/falzar/beast-ko`, `cross-ko` | match |
| Every Cross knocked out by its weakness (the next screen no longer offers it) | `forms/falzar/cross-{spout,tomahawk,tengu,ground,dust}-weakness`, `forms/gregar/cross-{heat,elec,slash,erase,charge}-weakness` | match |
| A Cross Beast's three turns running out | `forms/falzar/cross-spout-beast-spent` | matches |
| Both sides transforming on one turn; a Cross to another Cross (the used one no longer offered); a weakness hit on a Cross Beast; Beast Out out of Full Synchro; the tired turns after Beast Out's three | `forms/falzar/beast-both`, `beast-vs-cross`, `cross-to-cross`, `cross-spout-beast-weakness`, `beast-full-synchro`, `beast-out-spent` | match |
| Invalid chips (custom-screen.md §3.4): a code the chip doesn't have; Mega chips past the Mega level, and within it with MegFldr2 | `custom/invalid-code`, `invalid-mega-count`, `navicust/megfldr2` | match |
| The descriptions copied from the console's memory (DblBeast's, the Gregar chip's and the Falzar chip's: the chatbox's `FF` command) | `custom/description-arm-137-5..9`, `description-arm-138-7`, `-8`, `description-arm-139-7`, `-8` | match (three lines, the default for a chip without text in the pack) |
| The hand's builder (custom-screen.md §5): a code run with one `*` (first, middle, last: it forms), with two, shifted (`*`, A, B) or in one code (it doesn't); a recipe out of order; a recipe between two other chips; the same recipe on the next screen (once a round); a modifier on a Program Advance; modifiers chained, mixed, on a navi chip, picked first and after a chip they don't apply to | `custom/pa-star-first`, `-middle`, `-last`, `-shifted`, `pa-two-stars`, `pa-same-code`, `pa-wrong-order`, `pa-in-the-middle`, `pa-two-in-a-hand`, `pa-once-a-round`, `pa-modifier`, `custom/modifier-chain`, `-mixed`, `-navi`, `-whicapsl`, `-first`, `-wrong-chip` | match |
| The Regular chip as a recipe's part (the Program Advance carries its bit); the tag pair and the Regular chip dealt together and forming a recipe; picks from the middle of the hand over four screens (the holes close up), picked in reverse on the last; a folder emptied five chips a screen (the sixth screen deals the last five, the next two none) | `custom/pa-regular`, `pa-tags`, `folder-odd-picks`, `folder-runs-out` | match |
| The forms on every stage kind, without the lab's base NaviCust (its side 0 save has AirShoes, FlotShoe and BugStop, so panels don't act on a p0 that leaves `navicust` out): MegaMan with no programs (slides on ice, rides the roads, poison's drain, cracked panels breaking behind, a hole refusing the step), Beast Out (the form's own shoes: none of that, but the volcano's 50), the Crosses (Spout on poison and volcano, Ground on cracked and ice, Tengu over holes and cracked with its AirShoes, Tomahawk on ice and roads, Dust on poison and roads), a step into the enemy's column refused | `forms/falzar/bare-stage-*`, `beast-stage-*` (eight each), `cross-spout-stage-poison`, `-volcano`, `cross-ground-stage-cracked`, `-ice`, `cross-tengu-stage-holes`, `-cracked`, `cross-tomahawk-stage-ice`, `-roads`, `cross-dust-stage-poison`, `-roads` | match |
| A body on its panels: TomahawkCross healing 1 HP every 20 ticks on grass, and hit there by FireBrn1 (210: the weakness and the grass add up to three times 70; MegaMan takes 140), the grass burning; SpoutCross not sliding on ice, hit there by Thunder (80, the weakness only; MegaMan takes 40: BN6's ice adds nothing to Elec) and by WideSht (frozen like MegaMan, the panel back to normal); TomahawkCross not paralyzed by a WhiCapsl hit (MegaMan is); TenguCross pulled by the enemy's Fan and pushed by its Wind like any navi | `forms/falzar/cross-tomahawk-grass`, `-grass-fire`, `base-grass-fire`, `cross-spout-ice`, `-ice-thunder`, `-ice-widesht`, `base-ice-thunder`, `base-ice-widesht`, `cross-tomahawk-paralysis`, `base-paralysis`, `cross-tengu-fan`, `-wind` | match |
| The gregar base without its save's programs (its side 0 has SuprArmr, UnderSht, AttckMAX, ChargMAX and HP+1200 on a bugged grid: no flinch, blank buster shots, HP draining): MegaMan and Gregar Beast Out on every stage kind (the Beast slides on ice, takes poison and can't cross holes: it has SuperArmor, not shoes), the Crosses (Heat on volcano, where the eruption still hurts it, and on grass; Elec on ice and over holes; Slash on cracked and ice; Erase on poison and roads; Charge on roads and holy) | `forms/gregar/bare-stage-*`, `beast-stage-*` (eight each), `cross-heat-stage-volcano`, `-grass`, `cross-elec-stage-ice`, `-holes`, `cross-slash-stage-cracked`, `-ice`, `cross-erase-stage-poison`, `-roads`, `cross-charge-stage-roads`, `-holy` | match |
| Gregar bodies on their panels and hit: HeatCross on grass under FireBrn1 (140, the grass burning) and WideSht (200, the Cross knocked out); ElecCross frozen on ice by WideSht and knocked out on grass by CornSht1; Gregar Beast under three Cannons (no flinch: the form's SuperArmor), MegaMan with no programs (flinches, buster 1) and with the save's (no flinch, blank shots, the HP bug's drain) | `forms/gregar/cross-heat-grass-firebrn`, `-widesht`, `cross-elec-ice-widesht`, `cross-elec-grass-cornsht`, `beast-hit`, `bare-hit`, `saved-hit` | match |
| The Gregar forms' own busters and charge shots, and their weakness knock-outs with the flinch (re-recorded without the save's programs); the Gregar Giga chips' users hit back by a Reflector or a MiniBomb | `forms/gregar/base`, `beast`, `beast-over`, `cross-*`, `cross-*-beast`, `cross-*-weakness`; `chips/0x12d-bass`, `0x12e-bighook`, `0x12f-deltaray`, `0x130-colforce`, `0x131-bugrswrd` `/guard`, `/counter` | match |
| Custom screen keys the lab had no recording of (the golden traces' dumps had them): SELECT hiding the window and a key bringing it back, B taking back picks, the Cross window closed with B, DustCross's scrap (two and three picked chips scrapped, the hand refilled) | `custom/hide-window`, `take-back`, `cross-window-close`, `dust-scrap` | match |
| Uninstll after a damaging chip that dims (Roll): the builder's second flag test, so two chips | `custom/modifier-uninstll-dimming` | matches |
| The worse status bug (six colours) and the emotion bug in Beast Out (no swings in a form) | `navicust/bug-status-6`, `bug-emotion-beast` | match |
| The chatbox the custom screen waits on (custom-screen.md §3.5): the tick a description takes keys from, by its text's lines (Cannon R+8, Recov10 R+7, the invalid chip R+6, a Cross R+8); B held; the L message's printing, rushed by A or held B, for MegaMan and all eleven link navis | `custom/description-arm-*`, `description-invalid-*`, `description-cross-*`, `description-b-held-12`, `-30`, `description-keys`, `run-message`, `-b`, `-wait`, `-taps-0`, `-taps-1`, `-b-held`, `run-message-navi-1` to `-11` and their `-wait` | match, since the chatbox's port (the engine took a description's key from R+6 whatever its lines, had no held B, and estimated the message at 72 ticks) |
| The link navis' charged attacks (their action table's entry 9, weapon routines 0x40 to 0x45 and 0x47 to 0x4A; standard-chips.md, "Action 9") and ProtoMan's (0x32, WideSwrd's slash) and his B+Back Reflect (0x30): each twice, from the start panel and a row up, the opponent standing still | `navis/navi-01-heatpres` to `navi-11-stepswrd` | match |
| BugRSwrd's and BgDthThd's charged shots (weapon routines 0x21 and 0x22, chips.md §3 subtype 38): with bug frags to spare (DrkSword's slash, the fast thunder ball: 200), with none (Sword's slash, Thunder's ball: 80, 40), with one for two shots (the frag spent, then the plain shot), hitting, out of reach, against the user's own RockCube, the turn after the chip, through a Cross change (the chip's weapon stays), and in Beast Out (the Beast buster gives way to the plain buster) | `chips/0x131-bugrswrd/charge-{hit,miss,obstacle,no-frags-hit,no-frags-miss,last-frag,next-turn,cross,beast}`, `chips/0x136-bgdththd/charge-{hit,moving,obstacle,no-frags-hit,last-frag,next-turn,beast}` | match |
| The arm chips' charged shots (0x23 to 0x26; in no legal folder: a save edit holds them): PunchArm's fists on the opponent's column, out of reach from the back columns, twice in a turn and again the next; NeedlArm's needles on a standing and a stepping opponent; PuzzlArm's pulse in and out of reach; BoomrArm's boomerang past an opponent on its path and off it | `chips/0x119-puncharm/charge-{hit,far,twice}`, `chips/0x11a-needlarm/charge-{hit,moving}`, `chips/0x11b-puzzlarm/charge-{hit,miss}`, `chips/0x11c-boomrarm/charge-{hit,miss}` | match |

### Not reachable in a netbattle (documented, no scenario)

| Branch | Why |
|---|---|
| A tag pair straddling the end of the re-deal's walk (custom-screen.md §3.7) | The walk overruns when it meets the pair's index with one entry left to count. OK takes one off the index for every chip it takes out of the folder, and the scrap changes neither, so the chips left less the index stay what the shuffle made them: 30 less an index of 1 to 20, at least 10. |
| NumbrOpn's walk in ChpShufl's re-deal (custom-screen.md §3.7) | Both programs can't be installed: NumbrOpn fills the inner 5x5 of the grid (24 cells compressed), ChpShufl needs 14 cells five wide, and of two overlapping parts only the later one's program applies (probed). |
| The Cross change (battle-flow.md §3.4.1) | The custom screen never sends one: the transform record's +4 has no live writer. |
| A dark chip on the custom screen (custom-screen.md §9): the cursor starting on it (`sub_802806C`), its window's dark frame palette, the hover's fades and music volume ramp (`sub_802A2B0`) | No chip record of the US ROM has bit 0x20 of its flags (the chip definitions' `dark`; gen-content checks that none has it), and a folder holds only chips of the chip table. Ported, with a unit test on a made-up chip (custom/tests.rs, `a_dark_chip_takes_the_cursor_and_darkens_the_screen`). |
| The buffered auto-step and its fallback directions (objects-and-player.md §M6.9) | NaviStats+0x11 is never set: no NaviCust bug routine writes it (`byte_813CC18`'s routines write +0x31, +0x24, +0x12/+0x13, +0x63, +0x28, +0x26, +0x14/+0x15, +0x0D, +0x18/+0x16, +0x62 and +0x1A). |
| A step starting in its Land phase (0x10) | Nothing starts a step there. |
| Result codes 4 (escape), 5 (communication error), 9 and 0xA (terminate) | No running in a netbattle (L gives the message); the others are the link's, which the lab's emulated cable never trips. |
| Rush with chip 0x17E (the hand left alone) | Chip 0x17E, the other WhiCapsl, comes in no code: it can't be in a folder. |
| A form's NaviCust refresh in Beast Over (`sub_801479C`, `sub_80147B2`) | The refresh follows a bug code a hit brings, and no hit lands on Beast Over: Falzar's form flags make the navi untouchable (ObjectFlags1 0x08000000, which the collision kernel drops every pair on), Gregar's invulnerable for 0xFFFF ticks (`chips/0x0b9-uninstll/folded-beast-over` and `-gregar` show it). |
| AntiDmg's counter aimed at the nearest enemy ahead (`sub_8016218`, the counter's variant 1) | The variant is the counter's caller's: every trap's (`sub_801056A`, six callers) passes 0, and the AntiDmg program's stance passes its own, which its weapon routine (0x3D, `sub_80121BC`) sets to 0. |
| A navi appearing mid-battle (`sub_80164A0`) | Only for actors whose AIData+2 is set, which a netbattle's players' isn't. |
| The Beast Out lock-on's tie-break between several targets | Its candidates are a side's alive-actor slots, and a netbattle fills one a side. |
| Bug code 0xFB (the body programs go, UnderShirt too), and the charged shot's and the buster's other projectile kinds | Only projectile kind 0x1A carries the code, and a player's shots take their kind from NaviStats+0x4F (the charged shot) and +0x4D (the buster), which nothing writes: not the stats' init (`initNaviStats_WithDefaultStatsMaybe_8013438`, `init_8013B64`), not a NaviCust program's or bug's routine, and no attack a netbattle has carries bug code 0x4D or 0x4F (a code below 0x64 stores to the stat of that number). Both are 0 in all 8,798 navi stat blocks of the lab's 4,399 setups (every program, bug, navi and form). |
| A hit's bug code that sets a weapon byte, a shot program, a form or the navi of the navi's stats to a number (NaviStats+0x04, +0x05, +0x07, +0x39, +0x44 other than 0xFF; +0x4D, +0x4F, and the forms +0x17 and +0x2C, other than 0; the navi, +0x29) | No hit of the game's carries such a code (the codes hits give are 0x18 with a level, and the special ones from 0xF4 up); the engine holds weapons, projectile variants, forms and navis by handle and has no number for them, so it refuses the write. Clearing one (0xFF, 0; a form byte's 0 is the base form) works. |
| A weakness hit on a plain Beast Out (`sub_8015766` on forms 0xB and 0xC: by the form's number the Falzar beast drops to the Gregar one, which stays; content-model-v2.md §3.2, `breaks_to`) | The request needs a damaging hit with a weakness multiplier (`sub_801A506`), and the Beasts are Null with no secondary weakness. |
| HP lost when the custom screen opens (NaviStats+0x54, `sub_8013FD0`), bug code 0x54, the tornado's variant 3 and the Tornado action's subtype 3 (`sub_80CA19E`) | The stat's one setter, `sub_813CF2C`, is unreferenced (no pointer word or call in the ROM reaches it or the stat setters around it). Bug code 0x54 is carried only by the tornado's variant 3, which only action 0x2F's subtype 3 spawns; no chip record has that subtype and no weapon routine starts the action. The stat is 0 in all 9,994 stat blocks of the lab's setups. |

