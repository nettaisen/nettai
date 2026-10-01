# Unverified branches: the survey

The port is written from the disassembly branch by branch, and the chip lab's recordings (the verification
workspace's chiplab) check it against the original. A branch no recorded scenario reaches is **unverified**:
ported, but never compared. This file collects those branches from the other documents into one table, says why
each is unreached and whether a scenario can reach it, and tracks the scenarios written for them.

Status:

- **verified**: a recorded scenario reaches the branch (its coverage file shows it) and the engine matches the
  recording on every frame;
- **differs**: recorded, and the engine differs; the note says where, and whose it is;
- **recorded**: recorded and in the lab, not yet replayed to the end against the engine;
- **open**: reachable, no scenario yet;
- **unreachable**: no netbattle can take the branch (a full object pool, a path only offline battles or the navi
  AI take, a record no chip names, a caller that doesn't exist).

Scenario names are the chip lab library's (`chips/…`, `pa/…`, `navis/…`, `forms/…`, `navicust/…`). The scenarios of
this survey are written by the library's `gen_coverage.py` from its tables.

## Chips

### Dimming chips (dimming-chips.md)

| family | branch | why unreached | reachable | scenario and status |
|---|---|---|---|---|
| AntiNavi (the dimming service) | the turn, the bounce between two traps | scratch recordings were lost | yes | chips/0x0ba-antinavi/heatman, bounce (main's) |
| AntiNavi | a chip that isn't a navi chip leaves the trap | no scenario | yes | chips/0x0ba-antinavi/not-a-navi: open |
| AntiNavi | the target has its own controller registered; AntiNavi's user deleted or without HP before the turn | needs a second dimming in the same turn, or a deletion inside one | hard | open |
| Barriers (subtype 4) | ending an old visual (`sub_80E0DC0`) | no barrier raised over another | yes | chips/0x0b3-barr100/over-barrier: open |
| Barriers | the visual's regrowth (action 4 back to 0) | BblWrap's bubble never popped | yes | chips/0x0b5-bblwrap/popped: open |
| Barriers | action 8, a popped barrier blown away | no wind meets a barrier | yes | chips/0x0b3-barr100/blown-away, chips/0x0b6-lifeaur/weak-and-strong: open |
| Barriers | a barrier broken with damage to spare | no scenario | yes | chips/0x0b3-barr100/broken: open |
| Barriers | the hidden-parts rule for type 9 on the remote side, the blind viewer, the off-field owner | the viewer's side; no blinded viewer | partly (a blinded viewer) | open |
| Panel chips (subtype 5) | kinds 1-3, 7-16, kind 9; a changer with no flag pointer | no chip, no caller | unreachable | |
| Panel chips | side 1's road swap | ComingRd and GoingRd only used by side 0 | yes | open |
| BugFix (subtype 26) | a non-player navi's sprite, a navi with parts, stat 0x21 = 0, variants 1 and 2 | every lab user is MegaMan with a Beast Out counter; variants have no caller | partly (a link navi's BugFix) | open |
| Instruments (subtype 9) | the destroyed action | no instrument broken or removed | yes | chips/0x092-fanfare/broken, pushed; 0x093-discord, 0x094-timpani, 0x095-silence/broken: open |
| Instruments | the tune played to its end | scenarios end first | yes | chips/0x092-fanfare/lifetime: open |
| Instruments | Fanfare's Beast Over test, the battle-over branches | no Beast Over, no KO with an instrument out | yes | open |
| Instruments | a failed collision | pool full | unreachable | |
| AirRaid (subtype 13) | the destroyed action's removal paths, the battle-over branch, the no-target branch | the plane is never hit | yes | chips/0x068-airraid1/broken: open (the dimming agent's) |
| AirRaid | the overlay's EV+0x10/0x14/0x18 and Param3-0 branches | LilBoiler's | see shot chips | |
| Sensor (subtype 28) | the pushed turret, the broken turret, removal and absorption | the turret is never hit or pushed | yes | chips/0x071-sensor1/broken, pushed: open |
| Sensor | the laser's re-arming, the scanner's blocked-by-object and edge branches, battle-over | one firing per scenario | yes | open |
| SumnBlk (subtype 36) | the whole navi (§9.2, §9.3) | no hole in front of the user | yes | chips/0x087-sumnblk1/hole-ahead, after-crack: open (the dimming agent's) |
| ColorPt, DblPoint (subtype 27) | the bonus itself (080E66E0, 080E66EC, 080E66F6) | the next chip is none or has no damage | yes | chips/0x0c2-colorpt/bonus, chips/0x0c4-dblpoint/bonus: open (the dimming agent's) |
| ColorPt | the special-source branch, a missing navi, `sub_800D53C` running off the field | no such user | unreachable | |

### Dimming chip effects (dimming-chip-effects.md)

| family | branch | why unreached | reachable | scenario and status |
|---|---|---|---|---|
| Wind, Fan (subtype 8) | a second fan replacing the first (the wind registry's destroy) | one fan per scenario | yes | chips/0x081-wind/then-fan, both-sides: open |
| Wind, Fan | the lifetime running out | scenarios end first | yes | chips/0x081-wind/lifetime: open |
| Wind, Fan | the fan broken, pushed | only Wind's `counter` breaks one | yes | chips/0x082-fan/broken, pushed: open |
| Wind, Fan | no gust (an obstacle on the target panel), removal, blink-out, absorption, a flipped fan | no obstacle there; no remover | partly | open |
| BurnSqr (subtype 23) | A to fire | no scenario presses A | yes | chips/0x06e-burnsqr1/a-fires: open |
| BurnSqr | a non-solid panel under the square | the timeout always falls on solid panels | yes | chips/0x06e-burnsqr1/a-fires-holes: open |
| BurnSqr | a non-player user, the blind viewer, the failed spawn | | unreachable (blind viewer: partly) | |
| GrabBnsh, GrabRvng (subtype 15) | the panel return, the strikes, the hand | nothing is stolen first | yes | chips/0x0a4-grabbnsh/after-areagrab, after-panelgrabs, chips/0x0a5-grabrvng/after-areagrab: open |
| Subtype 2 | everything | no chip | unreachable | |
| Guardian (subtype 14) | breaking it: the crumble, the strike back, the strike's dimming and hit | the statue is never hit | yes | chips/0x097-guardian/punish, own-hit: open |
| Guardian | pushes, eviction by another object | | yes | chips/0x097-guardian/pushed, replaced: open |
| Guardian | the stage statue (Param1 1), the lifetime (6000 ticks), removal, blink-out, absorption | no netbattle stage has one; too long | partly | open |
| Meteors (subtype 16) | the lists after area changes, a marker at battle end | no AreaGrab first, no KO | yes | open |
| Meteors | an empty list, a marker off the field, rows other than Param1 1 | the enemy always owns panels; no chip | unreachable | |
| Anubis, PoisPhar (subtype 17) | breaking by damage, pushes, the lifetime | the statue is never hit; scenarios end first | yes | chips/0x098-anubis/broken, pushed, lifetime: open |
| Anubis | a non-solid landing panel, time up, removal, blink-out, absorption, an enemy with no panel for a bubble | | partly | open |
| Anubis | the flipped user's registry store | no player is flipped | unreachable | |
| CircGun (subtype 19) | A to fire | no scenario presses A | yes | chips/0x08e-circgun/a-fires, a-fires-late: open |
| CircGun | a start column without enemy panels, a column of the user's own panels, shots on non-solid panels | the timeout's place | yes | open |
| CircGun | Param3 1, a non-player first actor | no chip | unreachable | |
| Otenko (subtype 18) | the bonus, the 50 cap, a new hand entry | the next chip never does damage | yes | chips/0x099-otenko/bonus: open (the dimming agent's) |
| Otenko | breaking, pushes, the slide, eviction, the lifetime | | yes | chips/0x099-otenko/broken, pushed: open (the dimming agent's) |
| BlzrdBal (subtype 21) | a non-solid thrower panel, a third swallow, the roller's battle-over end, more than 4 hit objects | | yes | open |
| Magnum (subtype 24) | A to fire, the row-mode shot | no scenario presses A | yes | chips/0x08d-magnum/a-fires, a-fires-late: open |
| Magnum | a non-player user, panels off the field | | unreachable | |
| Geddon and the capsules (subtype 3) | Param1 0 (crack) and 2 (poison): records 0x17F-0x182, 0x186, 0x187 | no folder holds them; the save edit can | yes (save edit) | chips/0x17f-prpcapsl … 0x187: open |
| Geddon | an empty list, a panel that changed between the list and its turn | | hard | open |
| Snake (subtype 12) | several holes (the 8/24-tick spacing), the tie-breaks | one hole at most | yes | chips/0x086-snake/after-geddon, after-geddon-miss: open |
| Snake | side 1's scan | side 0 always uses it | yes | open |
| Snake | no target, a flipped user, a failed nest spawn | | unreachable | |
| LifeSync (subtype 7) | the offline path | not a netbattle's | unreachable | |
| NumbrBl (subtype 22) | a non-player user, the user deleted before the effect, no player for formula 21 | | unreachable | |
| CornFsta (subtype 29) | no panel for a burst, the free panels all in the ring, a failed spawn | | hard | open |
| DblHero (subtype 30) | the failed spawns, no enemy panel, no panel of another side ahead | | unreachable | |
| MetrKnuk (subtype 32) | no enemy body (the fallback lists), no candidate, Param2 0 | the enemy always stands; no caller | unreachable | |
| DblBeast (subtype 37) | the user's-panel fallback, no target at all, a failed spawn | a free panel always turns up | hard | open |

### Traps, bombs, field objects, navi chips (chips.md §3.6-§3.9)

| family | branch | why unreached | reachable | scenario and status |
|---|---|---|---|---|
| Counter cut-ins (§3.6.5) | a cut-in by a chip of another action, cut-ins with Full Synchro, anger or a dark chip | the lab's counter cut-ins answer with the same chip | yes | open |
| Counter cut-ins | a failed controller spawn | pool full | unreachable | |
| AntiRecv | the heal turned to damage, Roll's dimming taken over | scratch recordings were lost | yes | chips/0x0bd-antirecv/recov10, roll (main's) |
| AntiRecv | Roll's damage with the double-damage flag, a full effect pool | | hard / unreachable | open |
| AntiDmg | the trap sprung: the stars thrown back | the lab never hits a trap's user | yes | chips/0x0bb-antidmg/sprung, sprung-side0, small-hit, replaced, turn-end: open (the dimming agent's) |
| AntiSwrd | the trap sprung by a sword; a hit that isn't a sword | | yes | chips/0x0bc-antiswrd/sprung, not-a-sword: open (the dimming agent's) |
| ElemTrap (§3.6.10) | the spring, the sparkles, the counterattack, the panel bursts | the lab never hits the trap with an element | yes | chips/0x0c5-elemtrap/sprung-fire, sprung-elec, null-hit: open (the dimming agent's) |
| BodyGrd (PA 0x157) | the trap itself | recorded only as its recipe | yes | pa/0x157-bodygrd/sprung: open (the dimming agent's) |
| IceCube, WhiCapsl (0x17C, 0x17E) | their records | no folder holds them | yes (save edit) | chips/0x17c-icecube/hit, pushed, broken, melted; chips/0x17e-whicapsl/hit: open |
| TimeBom (subtype 10) | the blast and its bursts | scenarios end during the countdown | yes | chips/0x090-timebom1/blast: open |
| TimeBom | the bomb broken first (the puff), pushed, the battle's end | | yes | chips/0x090-timebom1/broken, pushed, round-end: open |
| TimeBom | variants 2 to 7 | no slot pointer from the controller | unreachable | |
| Mine (subtype 11) | arming, blowing up when touched | scenarios end during the hops | yes | chips/0x091-mine/stepped-on: open |
| Mine | its panel no longer solid, the battle's end | | yes | chips/0x091-mine/panel-broken, round-end: open |
| RskyHny (§3.7) | the bee's end by battle over, by an attack's hit; a sting with no navi on the panel; steering in a column, reversing in a row; the destination fallbacks | the lab's bees always reach a standing target | partly | open (the standard-chip agents') |
| RskyHny | params byte 1, the fade (action 8), the bee without a collision slot | no chip, no setter, pool full | unreachable | |
| Dragons (§3.8) | no enemy body ahead, either part's end at the battle's end, a blocked hit clearing the head's region | | yes | open (the standard-chip agents') |
| Bombs and seeds (§3.9) | a bomb or seed ending at the battle's end; the BlkBomb set off by fire, pushed, removed; the BugBomb broken or in a hole, its other bug choices; FlashBomb set down onto a body, broken before the flash | | yes | open |
| Bombs | bomb kind 1, seed kind 3, FlashBomb levels 3 to 8 | no chip | unreachable | |
| Navi chips (§3.6.7-§3.6.36) | no footing for the navi (action 0 to its leave) | the user always stands on solid ground | yes (AirShoes over a hole) | open |
| Navi chips | the battle ending mid-attack | no KO inside a navi chip | yes | open |
| Navi chips | a pool with no free slot, a missing collision slot | pool full | unreachable | |
| Navi chips | the navi AI's variants (Param1 0 and the like): SpoutMan's, BlastMan's, ElecMan's, ChargeMan's, SlashMan's, DiveMan's, JudgeMan's whip | only the bosses' AI spawns them | unreachable | |
| GroundMan | no rock candidate, the rock's non-solid landing, the drill's battle-over path | | partly | open |
| DustMan | the junk's looks and NameIDs, the battle's end, the flag check after moving | | partly | open |
| JudgeMan | a book arriving, leaving solid ground, the heading's reversal, no enemy navi | | partly | open |
| TwinLdrs | ProtoMan leaving without a slash or with more than one target; Colonel's target off the field | | partly | open |
| CrosOver | a link navi user, no target, no Django, a refused front panel, the far-column fallback | | partly (a link navi can't hold the PA) | open |
| MstrCros | the Gregar tables (the beam, move 1) | the Falzar side always uses it | yes (the gregar base) | open |
| Bass, BigHook, Darkness | failed spawns, a missing flag pointer, the strike's MachGun branches | | unreachable | |

### Shot chips (shot-chips.md §16)

| family | branch | why unreached | reachable | scenario and status |
|---|---|---|---|---|
| Cannons (0x14) | variant 3; a holder that isn't a player; a full pool | no chip | unreachable | |
| Vulcans (0x17) | kind 0x10's single effect (a SuprVulc bullet's hit) | | yes | open |
| BatCan (0x19), TankCan (0x24), CornSht (0x2A), WideSht (0x30) | the battle-over end of the shot, shell, corn and wave | no KO with one in flight | yes | open |
| TankCan | a wait of more than a tick, Param2 0, an explosion row of 4 | no chip | unreachable | |
| MachGun (0x29) | the target column moving; the no-body fallback | the opponent stands still | yes | open |
| WideSht | the wave's kinds 0-2 and 9 | dead code | unreachable | |
| Sonic booms (0x55) | everything: SonicBom, SprSonic, Curse, Punisher | VarSwrd's and NeoVari's picks, never commanded; no folder holds the records | yes (save edit) | chips/0x173-sonicbom, 0x177-sprsonic, 0x174-curse, 0x175-punisher/{hit, adjacent, miss}; sonicbom/obstacle, guard, ko; sprsonic/obstacle: open |
| Z Saver (0x5B) | everything, with the fourth slash's command | weapon 0x6E; no folder holds the record | yes (save edit) | chips/0x17d-zsaver/{hit, adjacent, miss, fourth-slash, command-late, command-split, ko}: open |
| Rapid buster (0x5D) | everything | weapon 0x39: no navi or form of a netbattle has it | unreachable | |
| LilBoiler | the eruption | | yes | chips/0x062-lilbolr1/erupt-hits, chips/0x063-lilbolr2/erupt-lifetime (G4's) |
| LilBoiler | a bonus (the registry slot it writes) | no scenario gives it Atk+ | yes | open |
| LilBoiler | the hole, a removal request, absorption, blinking out, pushes | | partly | open |
| VDoll | the curse | | yes | chips/0x096-vdoll/curse (G4's) |
| VDoll | the puff, absorption, blinking out, the lifetime's end | | partly | open |
| The variable swords' picks (action 0x13 variants 9-11) | FtrSword, CrosSwrd, DblDream | never commanded | yes (save edit) | chips/0x172-ftrsword, 0x176-crosswrd, 0x178-dbldream/{hit, adjacent, miss}: open |

### Standard chips (standard-chips.md)

In flux (the standard-chip agents are converting these modules): scenarios are recorded and differences reported,
not fixed here.

| family | branch | why unreached | reachable | scenario and status |
|---|---|---|---|---|
| VarSwrd, NeoVari | every command sequence, the random pick, the charged sword (0x41) | no scenario enters a command | yes | open |
| MoonBld | the repeat swings | | yes | open |
| CopyDmg | the time-up path, NameIDs 0xCD..0xFF (an obstacle as the target) | | yes | open |
| RlngLog | the drop-in path, the log's stop and break | no obstacle in its way | yes | open |
| Tornado, Static | Tornado subtype 3, Static's larger spreads | | check | open |
| AirSpin | variant 1; the whirlwind removed, absorbed, blinking out | | partly | open |
| WideBrn | the spread, flame looks 1 and 2 | PAs | check | open |
| ParaShl, GreatYo, PitHocky | the Program Advances past the banner | stopped at the banner when written | yes | the recipes (pa/…): check |
| H-Burst | the fight-over and whole-field branches | | yes | open |
| DarkThnd | the thunder ball's bug | | yes | open |
| WindRack | variant ≠ 0, the gust's other spawners | no chip | unreachable | |
| Needles, pulses, aura heads, thunder dolls | three or more enemies, looks 2 and 3, speed tables past 1, Param1 ≠ 0 | the viruses' and the AI's | unreachable | |
| Shock wave | the virus variants 0..0xB | | unreachable | |
| Link navi chips (action 0x0A) | HeatPres off solid ground, DElecSwd's stops, RSlash's turns, EDeletBm's cap, VolcChrg's panels, DripShwr's jump, ETomahwk's cap, RC Brakr's rockfall | one scenario per navi | yes | open (the link navi agent's) |

### Field objects (field-objects.md, objects-and-player.md)

| family | branch | why unreached | reachable | scenario and status |
|---|---|---|---|---|
| Obstacles | pushes (`sub_8017CC0`): a fan, a statue, another obstacle; the pushes' ice and bounds branches | nothing is pushed | yes | the `pushed` scenarios above; chips/0x08f-rockcube/pushed, pushed-by-enemy: open |
| Obstacles | broken by damage, replaced (the field-object slots) | | yes | chips/0x08f-rockcube/broken, replaced: open |
| Obstacles | thrown and encased (`sub_8018002`, `sub_801813A`) | nothing in the game starts them | unreachable | |
| Obstacles | actions 3/4/6/7, a push without direction bits | nothing sets them | unreachable | |
| Flying shot | kinds 2 and 5, the wait (TrnArrw, the Beast buster) | | check | open |
| Projectile | the bursts (Param1 0xC), climbing (Param1 0x1D) | | check | open |
| Target tracking (`sub_802EF74`) | battle flag 0x40 | never in netbattles | unreachable | |
