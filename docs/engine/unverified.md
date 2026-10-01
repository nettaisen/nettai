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

## Dimming chips, shot chips and navi chips

### Dimming chips (dimming-chips.md)

| family | branch | why it was unreached | reachable | scenario and status |
|---|---|---|---|---|
| AntiNavi (the dimming service) | the turn, the bounce between two traps | scratch recordings were lost | yes | chips/0x0ba-antinavi/heatman, bounce: verified |
| AntiNavi | a chip that isn't a navi chip leaves the trap | no scenario | yes | chips/0x0ba-antinavi/not-a-navi: verified |
| AntiNavi | the target has its own controller registered; AntiNavi's user deleted or without HP before the turn | needs a second dimming in the same turn, or a deletion inside one | hard | open |
| Barriers (subtype 4) | ending an old visual (`sub_80E0DC0`) | no barrier raised over another | yes | chips/0x0b3-barr100/over-barrier: verified |
| Barriers | the visual's regrowth (action 4 back to 0) | BblWrap's bubble never popped | yes | chips/0x0b5-bblwrap/popped: verified |
| Barriers | action 8, a popped barrier blown away | no wind meets a barrier | yes | chips/0x0b3-barr100/blown-away, chips/0x0b6-lifeaur/weak-and-strong, the `barrier` template's wind chips: verified |
| Barriers | a barrier broken with damage to spare | no scenario | yes | chips/0x0b3-barr100/broken: verified |
| Barriers | the hidden-parts rule for type 9 on the remote side, the blind viewer, the off-field owner | the viewer's side; no blinded viewer | partly (a blinded viewer) | open |
| Panel chips (subtype 5) | kinds 1-3, 7-16, kind 9; a changer with no flag pointer | no chip, no caller | unreachable | |
| Panel chips | side 1's road swap | ComingRd and GoingRd only used by side 0 | yes | chips/0x0aa-comingrd/side1, chips/0x0ab-goingrd/side1: verified |
| Panel chips | a holy panel cracked from under its user in the same dimming | no cut-in on a panel chip | yes | chips/0x0a8-holypanl/cut-in-geddon: verified |
| BugFix (subtype 26) | a navi with parts | every lab user is MegaMan | yes (a link navi) | chips/0x0b0-bugfix/link-navi: verified |
| BugFix | a non-player navi's sprite, stat 0x21 = 0, variants 1 and 2 | no such user; no caller | unreachable | |
| Instruments (subtype 9) | the destroyed action (broken, and pushed) | no instrument broken or removed | yes | chips/0x092-fanfare/broken, pushed; chips/0x093-discord/broken, chips/0x094-timpani/broken, chips/0x095-silence/broken: verified |
| Instruments | the tune played to its end | scenarios end first | yes | chips/0x092-fanfare/lifetime: verified |
| Instruments | Fanfare's Beast Over test, the battle-over branches | no Beast Over, no KO with an instrument out | yes | open |
| Instruments | a failed collision | pool full | unreachable | |
| AirRaid (subtype 13) | the plane shot down, the battle-over branch, the bombs against a barrier or no body | the plane is never hit | yes | chips/0x068-airraid1/broken, ko, barrier, invisible: verified |
| Sensor (subtype 28) | the pushed turret, the broken turret | the turret is never hit or pushed | yes | chips/0x071-sensor1/broken, pushed: verified |
| Sensor | the laser's re-arming | one firing per scenario | yes | chips/0x071-sensor1/twice: verified |
| Sensor | the battle-over branches | no KO | yes | chips/0x071-sensor1/ko: verified (the comparison now keeps a spark's garbage Z fraction after its laser is freed) |
| Sensor | the scanner's blocked-by-object and edge branches, removal and absorption, failed collisions | | partly | open |
| SumnBlk (subtype 36) | the whole navi (§9.2, §9.3) | no hole in front of the user | yes | chips/0x087-sumnblk1/hole-ahead, after-geddon, chips/0x089-sumnblk3/hole-ahead: verified |
| ColorPt, DblPoint (subtype 27) | the bonus itself (080E66E0, 080E66EC, 080E66F6) | the next chip is none or has no damage | yes | chips/0x0c2-colorpt/bonus, chips/0x0c4-dblpoint/bonus: verified; chips/0x062-lilbolr1/colorpt: to rerun (it stopped in LilBoiler's registration before that was changed) |
| ColorPt | the special-source branch, a missing navi, `sub_800D53C` running off the field | no such user | unreachable | |

### Dimming chip effects (dimming-chip-effects.md)

| family | branch | why it was unreached | reachable | scenario and status |
|---|---|---|---|---|
| Wind, Fan (subtype 8) | a second fan replacing the first (the wind registry's destroy) | one fan per scenario | yes | chips/0x081-wind/then-fan, both-sides: verified |
| Wind, Fan | the lifetime running out (1440 ticks) | scenarios end first | yes | chips/0x081-wind/lifetime: verified |
| Wind, Fan | Fan's fan broken; AirShot at it | only Wind's `counter` breaks one | yes | chips/0x082-fan/broken, pushed: verified (the shot breaks it: no push branch) |
| Wind, Fan | no gust (an obstacle on the target panel), removal, blink-out, absorption, pushes (action 5), a flipped fan | no obstacle there; no remover | partly | open |
| BurnSqr (subtype 23) | A to fire | no scenario presses A | yes | chips/0x06e-burnsqr1/a-fires: verified |
| BurnSqr | a non-solid panel under the square | the timeout always falls on solid panels | yes | chips/0x06e-burnsqr1/a-fires-holes: verified |
| BurnSqr | a non-player user, the blind viewer, the failed spawn | | unreachable (the blind viewer: open) | |
| GrabBnsh, GrabRvng (subtype 15) | the panel return, the strikes, the hand | nothing is stolen first | yes | chips/0x0a4-grabbnsh/after-areagrab, after-panelgrabs, chips/0x0a5-grabrvng/after-areagrab: verified |
| AreaGrab (subtype 0) | the stolen column going back under its thief; both sides stealing in one dimming | | yes | chips/0x0a3-areagrab/returns, both: verified |
| Subtype 2 | everything | no chip | unreachable | |
| Guardian (subtype 14) | breaking it: the crumble, the strike back, the strike's dimming and hit | the statue is never hit | yes | chips/0x097-guardian/punish, own-hit: verified |
| Guardian | AirShot at it, a second statue | | yes | chips/0x097-guardian/pushed, replaced: verified |
| Guardian | the stage statue (Param1 1), its strikes, absorption | | yes | the stage scenarios (stages/statue-…, stages/statues-stand): see field-objects.md |
| Guardian | the lifetime (6000 ticks), removal, blink-out | too long; no remover | partly | open |
| Meteors (subtype 16) | the lists after area changes, a marker at battle end | no AreaGrab first, no KO | yes | chips/0x08b-meteors/after-areagrab, grabbed, ko: verified |
| Meteors | an empty list, a marker off the field, rows other than Param1 1 | the enemy always owns panels; no chip | unreachable | |
| Anubis, PoisPhar (subtype 17) | breaking by damage, AirShot at it, the lifetime, a second statue | the statue is never hit; scenarios end first | yes | chips/0x098-anubis/broken, pushed, lifetime, replaced: verified |
| Anubis | a non-solid landing panel, time up, removal, blink-out, absorption, an enemy with no panel for a bubble | | partly | open |
| Anubis | the flipped user's registry store | no player is flipped | unreachable | |
| CircGun (subtype 19) | A to fire | no scenario presses A | yes | chips/0x08e-circgun/a-fires, a-fires-late: verified |
| CircGun | a start column of the user's own panels, shots on non-solid panels | the timeout's place | yes | chips/0x08e-circgun/after-areagrab, holes: verified |
| CircGun | Param3 1, a non-player first actor | no chip | unreachable | |
| Otenko (subtype 18) | the bonus, breaking, pushes | the next chip never does damage | yes | chips/0x099-otenko/bonus, broken, pushed: **differs** (subtype 18 isn't implemented: being ported) |
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
| DblBeast (subtype 37) | the user's-panel fallback, no target at all, a failed spawn | a free panel always turns up | hard | open |

### Traps, bombs and navi chips (chips.md §3.6-§3.9)

| family | branch | why it was unreached | reachable | scenario and status |
|---|---|---|---|---|
| Counter cut-ins (§3.6.5) | a cut-in by a chip of another subtype or action; a chain of four | the lab's counter cut-ins answer with the same chip | yes | chips/0x0a3-areagrab/cut-in-invisibl, cut-in-chain, chips/0x0dd-roll/cut-in-heatman, chips/0x0e3-heatman/cut-in-barrier: verified |
| Counter cut-ins | A during a dimming with a chip that doesn't dim next | | yes | chips/0x0b1-invisibl/cut-in-not-dimming, the `dimmed` template: verified |
| Counter cut-ins | cut-ins with Full Synchro, anger or a dark chip | | yes | open |
| Counter cut-ins | a failed controller spawn | pool full | unreachable | |
| AntiRecv | the heal turned to damage, Roll's dimming taken over | scratch recordings were lost | yes | chips/0x0bd-antirecv/recov10, roll: verified |
| AntiRecv | Roll's damage with the double-damage flag, a full effect pool | | hard / unreachable | open |
| AntiDmg | the trap sprung: the stars thrown back | the lab never hits a trap's user | yes | chips/0x0bb-antidmg/sprung, sprung-side0, small-hit, turn-end: **differs** (the role `actions.anti_damage_counter` isn't filled: being ported); replaced: verified |
| AntiSwrd | the trap sprung by a sword | | yes | chips/0x0bc-antiswrd/sprung: **differs** (`actions.anti_sword_counter`: being ported); not-a-sword: verified |
| ElemTrap (§3.6.10) | the spring, the sparkles, the counterattack, the panel bursts | the lab never hits the trap with an element | yes | chips/0x0c5-elemtrap/sprung-fire, sprung-elec, null-hit: verified |
| BodyGrd (PA 0x157) | the trap itself | recorded only as its recipe | yes | pa/0x157-bodygrd/sprung: **differs** (`actions.body_guard_counter`: being ported) |
| IceCube (0x17C) | its record | no folder holds it | yes (save edit) | chips/0x17c-icecube/hit, pushed, broken, melted: verified |
| WhiCapsl (0x17E) | its dimming record | no folder can hold it (no codes: chip 0x185 instead) | unreachable | chips/0x17e-whicapsl/hit shows the 0x185: verified |
| Invisibl | shots and swords through an invisible navi | | yes | chips/0x0b1-invisibl/shot-at, the `invisible` template: verified |
| TimeBom (subtype 10) | the blast and its bursts | scenarios end during the countdown | yes | chips/0x090-timebom1/blast: verified |
| TimeBom | the bomb broken first (the puff), pushed, the battle's end | | yes | chips/0x090-timebom1/broken, pushed, round-end: verified |
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
| Navi chips | the battle ending mid-attack | no KO inside a navi chip | yes | chips/…/ko for the same navis (but ElecMan, TmhkMan and HackJack, which don't reach a standing opponent), pa/0x15c-twinldrs/ko, pa/0x15d-crosover/ko, pa/0x15a-mstrcros/ko: verified |
| Navi chips | a pool with no free slot, a missing collision slot | pool full | unreachable | |
| Navi chips | the navi AI's variants (Param1 0 and the like): SpoutMan's, BlastMan's, ElecMan's, ChargeMan's, SlashMan's, DiveMan's, JudgeMan's whip | only the bosses' AI spawns them | unreachable | |
| GroundMan | no rock candidate, the rock's non-solid landing | | partly | open |
| DustMan | the junk's looks and NameIDs, the flag check after moving | | partly | open |
| JudgeMan | a book arriving, leaving solid ground, the heading's reversal, no enemy navi | | partly | open |
| TwinLdrs, CrosOver, MstrCros | no footing for their navis | | yes | pa/0x15c-twinldrs/no-footing, pa/0x15d-crosover/no-footing, pa/0x15a-mstrcros/no-footing: verified |
| TwinLdrs | ProtoMan leaving without a slash or with more than one target; Colonel's target off the field | | partly | open |
| CrosOver | a link navi user, no target, no Django, a refused front panel, the far-column fallback | | partly (a link navi can't hold the PA) | open |
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
| LilBoiler | a removal request, absorption, blinking out | | partly | open |
| VDoll | the curse | | yes | chips/0x096-vdoll/curse: verified |
| VDoll | the lifetime's end, AirShot at it, its own side's hit | | yes | chips/0x096-vdoll/lifetime, pushed, own-hit: verified |
| VDoll | absorption, blinking out | | partly | open |
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
| RlngLog | the log's stop and break; no ground under it | no shot meets a log | yes | chips/0x028-rlnglog1/shot, holes: verified |
| RlngLog | the drop-in path | | check | open |
| Static | the larger spreads (the user's NaviCust bug kinds) | no bugged user | yes | chips/0x035-static/bugs-1, bugs-2, bugs-3: verified |
| Tornado | subtype 3 | | check | open |
| AirSpin | the whirlwind shot down, AirShot at it | | yes | chips/0x07e-airspin1/shot, pushed: verified |
| AirSpin | variant 1; the whirlwind removed, absorbed, blinking out | no chip; no remover | partly | open |
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
| Obstacles | pushes (`sub_8017CC0`, `sub_800F598`, `sub_8017CE0`, `sub_8017D64`, `sub_8017E0A`) | nothing is pushed | yes | chips/0x090-timebom1/pushed, chips/0x03c-blkbomb/pushed, chips/0x062-lilbolr1/pushed, chips/0x08f-rockcube/pushed-by-enemy, chips/0x097-guardian/pushed, chips/0x071-sensor1/pushed, chips/0x17c-icecube/pushed: verified |
| Obstacles | the pushes' ice and bounds branches | | yes | open |
| Obstacles | the field-object slots: a third class-0 object, a second class-1 one | | yes | chips/0x08f-rockcube/replaced, chips/0x098-anubis/replaced, chips/0x062-lilbolr1/replaced: verified |
| Obstacles | thrown and encased (`sub_8018002`, `sub_801813A`) | nothing in the game starts them | unreachable | |
| Obstacles | actions 3/4/6/7, a push without direction bits | nothing sets them | unreachable | |
| Flying shot | kinds 2 and 5, the wait (TrnArrw, the Beast buster) | | yes | chips/0x018-trnarrw1/ko, barrier, invisible, dimmed: verified |
| Projectile | the bursts (Param1 0xC), climbing (Param1 0x1D) | | check | open |
| Target tracking (`sub_802EF74`) | battle flag 0x40 | never in netbattles | unreachable | |

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
| START pause (battle-flow.md §3.5): only the pausing player resumes; both on one tick (side 0 pauses); during a dimming and once the battle is over (nothing) | `flow/pause`, `pause-both`, `pause-dimmed`, `pause-over` | match |
| A screen confirmed with nothing picked keeps the hand's chips; only Beast Out picked empties it | `flow/keep-hand` | matches |
| The custom screen opened while an attack is under way (a Vulcan mid-burst), and by both players on the tick the gauge fills (L and R) | `flow/custom-open-busy`, `custom-open-both` | match |
| Anger from a 300-damage hit (the next chip doubled, the anger spent) and its 600 ticks running out; anger from a counter's paralysis | `flow/anger-big-hit`, `anger-runs-out`, `synchro-lost` | match |
| Full Synchro from a counter hit: the next chip doubled and the synchro spent; lost to a plain hit | `flow/synchro-used`, `synchro-lost` | match |
| A navi deleted by a navi chip's hit during its dimming | `flow/ko-dimmed` | matches |
| A double KO on one tick (§3.6: the round goes by the alive counts, side 0 tested first) | `flow/double-ko` | matches |
| The turn timer from the 15th screen, the damage judge (state 0x18) and its three outcomes; the draw state 0x14 and result code 3 | `flow/judge-win`, `judge-lose`, `judge-draw` | match (10,600 frames each) |
| A lost round (result code 2) | `forms/falzar/ko-lose` | matches |
| Deleted in Beast Out, and at 0 HP in a Cross | `forms/falzar/beast-ko`, `cross-ko` | match |
| Every Cross knocked out by its weakness (the next screen no longer offers it) | `forms/falzar/cross-{spout,tomahawk,tengu,ground,dust}-weakness`, `forms/gregar/cross-{heat,elec,slash,erase,charge}-weakness` | match |
| A Cross Beast's three turns running out | `forms/falzar/cross-spout-beast-spent` | matches |
| Both sides transforming on one turn; a Cross to another Cross (the used one no longer offered); a weakness hit on a Cross Beast; Beast Out out of Full Synchro; the tired turns after Beast Out's three | `forms/falzar/beast-both`, `beast-vs-cross`, `cross-to-cross`, `cross-spout-beast-weakness`, `beast-full-synchro`, `beast-out-spent` | match |
| Invalid chips (custom-screen.md §3.4): a code the chip doesn't have; Mega chips past the Mega level | `custom/invalid-code`, `invalid-mega-count` | match |
| The chatbox the custom screen waits on (custom-screen.md §3.5): the tick a description takes keys from, by its text's lines (Cannon R+8, Recov10 R+7, the invalid chip R+6, a Cross R+8); B held; the L message's printing, rushed by A or held B, for MegaMan and three link navis | `custom/description-arm-*`, `description-invalid-*`, `description-cross-*`, `description-b-held-12`, `-30`, `description-keys`, `run-message`, `-b`, `-wait`, `-taps-0`, `-taps-1`, `-b-held`, `run-message-navi-*` | match, since the chatbox's port (the engine took a description's key from R+6 whatever its lines, had no held B, and estimated the message at 72 ticks) |

### Not reachable in a netbattle (documented, no scenario)

| Branch | Why |
|---|---|
| A tag pair straddling the end of the re-deal's walk (custom-screen.md §3.7) | The walk overruns when it meets the pair's index with one entry left to count. OK takes one off the index for every chip it takes out of the folder, and the scrap changes neither, so the chips left less the index stay what the shuffle made them: 30 less an index of 1 to 20, at least 10. |
| NumbrOpn's walk in ChpShufl's re-deal (custom-screen.md §3.7) | Both programs can't be installed: NumbrOpn fills the inner 5x5 of the grid (24 cells compressed), ChpShufl needs 14 cells five wide, and of two overlapping parts only the later one's program applies (probed). |
| The Cross change (battle-flow.md §3.4.1) | The custom screen never sends one: the transform record's +4 has no live writer. |
| The buffered auto-step and its fallback directions (objects-and-player.md §M6.9) | NaviStats+0x11 is never set: no NaviCust bug routine writes it (`byte_813CC18`'s routines write +0x31, +0x24, +0x12/+0x13, +0x63, +0x28, +0x26, +0x14/+0x15, +0x0D, +0x18/+0x16, +0x62 and +0x1A). |
| A step starting in its Land phase (0x10) | Nothing starts a step there. |
| Result codes 4 (escape), 5 (communication error), 9 and 0xA (terminate) | No running in a netbattle (L gives the message); the others are the link's, which the lab's emulated cable never trips. |
| Rush with chip 0x17E (the hand left alone) | Chip 0x17E, the other WhiCapsl, comes in no code: it can't be in a folder. |
| A navi appearing mid-battle (`sub_80164A0`) | Only for actors whose AIData+2 is set, which a netbattle's players' isn't. |
| The Beast Out lock-on's tie-break between several targets | Its candidates are a side's alive-actor slots, and a netbattle fills one a side. |

### Not attempted yet

| Branch | Note |
|---|---|
| Bug code 0xFB (the body programs go, UnderShirt too) | It comes from a charged shot kind (NaviStats+0x4F), whose writer isn't found among the NaviCust's routines. |
| The descriptions of DblBeast, Gregar and Falzar | Their scripts print a value with a command (`FF`) the chatbox's port counts as text; Giga chips, so a dig of several turns. |
