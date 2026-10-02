# Completeness: the port against the original's battle code

The chip lab verifies what its recordings reach. This is the other question: **is there battle code in the
original that the port doesn't have at all?** The answer at the audited commit: no routine of the simulation that
a netbattle can reach is left without a counterpart. The audit found two groups, both ported since: the link
navis' charged attacks (65 routines, §4) and six chip weapons that were defined without behaviour (§9), which
its first run missed. What is left are caveats (§7) and code that is ported but that no recording runs (§6).

Three things this document said before were wrong or weaker than they read, and are corrected here:

- **"Missing: 0" was false** while the six chip weapons had no `setup` (§9). The audit now follows the weapon
  table and has a stub check; both report none.
- **"All of which the engine reproduces in full" covered less than it said.** 670 of the lab's recordings ended
  inside a cut-in chip's telop, about 80 frames after the chip was used and before it did anything (§10). A match
  on those vouched for the telop's start and nothing of the chip. They are recorded to the end now, and match.
- **380 recordings didn't do what their names said** (`unmet` in the lab's index: the chip never reached the
  opponent from where its template stood). They are placed where the chip lands, or their goal says what they
  show (§11); none is left. One of the recordings written for this found a port bug: Falzar Beast Over's form
  flags lacked AirShoe and FloatShoe (§11).

The audit is a tool in the verification workspace (`tools/audit/audit.py`, with its exclusion rules in `excl.py`,
the hand-read classes in `manual.py` and the stub check in `stubs.py`); it runs against an engine checkout and
should be run again after large merges. Figures here are for the engine after content model v2 (step 13: forms,
navis and everything else by handle) with the Falzar Beast Over fix, and a lab of 5,169 scenarios: 5,149
recorded (the other 20 can't be: 18 skipped, 2 where the original itself stops advancing). The engine reproduces
every recording on every frame (5,125 of them in the last full run on this branch, and the others in their own
agents' runs).

## 1. Method

1. **The static call graph**, from the recompiler's translation of the ROM (13,603 routines): for every routine
   its direct calls, its branches into other routines, and the literal words it loads. A literal that is a ROM
   address is read as a table of code pointers up to the next label, with tables of tables followed two levels
   down. A spawn (`object_spawnType1`, `3` or `4` and their variants after a constant in r0) names the object kind
   it creates, which leads to the kind's handler through the three kind tables (95 actor kinds, 213 attack kinds,
   146 effect kinds).
2. **Citations.** A routine counts as cited when its name or address appears in the engine's crates or content,
   or in docs/engine (this document left out).
3. **Coverage.** From the lab's coverage files: a routine ran if its entry block did; a branch ran one way only
   if one of its two counts is zero. Recordings the engine doesn't reproduce in full are kept apart: code that
   only they run is not vouched for by a match.
4. **The inventory**: every routine in the battle's address ranges, and every routine elsewhere that a recording
   ran.

   | Area | Addresses |
   |---|---|
   | IWRAM routines (the collision kernel, the object draw pass) | 0x03005B00–0x030079A4 |
   | Object system (spawn, free, the three update dispatchers) | 0x0800318C–0x0800372A, `sub_8003C70`, `sub_8003E18`, `sub_8004218` |
   | Battle flow (init, the state machines, the link exchange, results) | 0x080071D4–0x0800B883 |
   | Battle objects and panels | 0x0800B884–0x0800ED7F |
   | Actors, collision, status, damage, weapons, the HUD | 0x0800ED80–0x0801FDFF |
   | Link status and battle settings | 0x0801FE00–0x0802644F |
   | Custom screen, gauge, camera | 0x08026450–0x08033947 |
   | Link layer | 0x0803D000–0x0803FD07 |
   | Object kinds, chips, navis | 0x080B81EC–0x0810D96F |

5. **Reachability.** Starting from everything cited or run, follow calls, a routine's own tables (40 entries or
   fewer: its states) and its spawns. The big dispatch tables are followed by what indexes them: the kind tables
   through spawns; the chips' action table (`JumpTable80EAC60`) whole, since every chip is a netbattle's; the
   hook tables indexed by actor type (`sub_800F35C` and its six siblings) through the player's table only; the
   weapon table (`off_80117D4`) through the numbers the data names (the forms' rows, the navis' rows and the
   NaviStats bytes: `compat/weapons.toml`, which the verification workspace's `gen-content check` checks against the ROM).
   A routine that the rules of §5 say a netbattle never enters is not followed. The audit stops with an error if a
   recording runs a routine a rule excludes: that caught a wrong rule once (the tutorial checks' callers run on
   every pick; only what they call is the tutorial's).
6. **Classes**, in this order:

   | Class | Test |
   |---|---|
   | Ported | cited in the engine's crates or content |
   | Documented | cited in docs/engine only: the docs describe it under a routine the source cites, or as presentation, or as not a netbattle's |
   | Infrastructure | outside the battle's ranges and run: the main loop, graphics transfers, sprites, sound, text, save flags |
   | Out of scope, by a rule | under a routine §5 lists |
   | Out of scope, unreachable | neither run nor reachable |
   | Trivial | empty, or a leaf of 8 instructions or fewer (accessors) |
   | Link transport | the link layer and the link status routines: the port's netplay stands in their place |
   | Presentation | HUD tasks (0x0801BE28–0x0801EC97), the IWRAM draw pass, custom screen drawing and camera routines read by hand |
   | Folded into a caller | every caller is ported (or documented): a table entry or helper of a routine the port has as one function |
   | Run in matching recordings | run, uncited, none of the above: its effects are reproduced or it has none on the simulation, since every recording that runs it matches frame for frame |
   | Missing | run only in recordings the engine stops in; a weapon routine the data names that nothing cites; the routine of a weapon defined without behaviour (§9); or left over |

7. **The stub check** (`tools/audit/stubs.py`, §9): what the engine has by name without behaviour, from the
   loaded content and from the sources' own "not implemented" errors. A citation counts as ported in step 6;
   this is the check that a cited thing does something.

## 2. Counts

8,116 routines are in the inventory; 4,063 of them run in the lab.

| Class | Routines |
|---|---|
| Ported: cited in the engine's source or content (3,236 run, 205 don't: §6.1) | 3,441 |
| Out of scope: not reachable from a netbattle's code | 3,175 |
| Documented: cited in docs/engine only (268 run, 154 don't) | 422 |
| Out of scope: excluded by a rule (§5) | 349 |
| Folded into a ported caller (22 of them never run) | 203 |
| Infrastructure a netbattle runs | 126 |
| Trivial: empty or an accessor | 122 |
| Presentation: HUD tasks | 112 |
| Run in matching recordings, no counterpart by name | 73 |
| Link transport: replaced by the port's netplay | 53 |
| Folded into a documented caller (6 of them never run) | 20 |
| Presentation: custom screen drawing, the IWRAM draw pass, camera | 20 |
| **Missing** | **0** |

Since content model v2's step 13 a form's flags and its NaviCust refresh are data of its definition (its
`status_reset` and `navicust_refresh`), so the per-form routines of `sub_8014536` and `sub_801469C` count as
folded into those two, which the engine cites.

By area:

| Area | Ported | Documented | Folded | Run, matching | Presentation | Link transport | Trivial | Infrastructure | Out of scope |
|---|---|---|---|---|---|---|---|---|---|
| IWRAM routines | 15 | 13 | 3 | 0 | 6 | 0 | 0 | 0 | 48 |
| Object system | 9 | 14 | 3 | 0 | 0 | 0 | 0 | 0 | 5 |
| Battle flow | 65 | 88 | 11 | 2 | 0 | 0 | 16 | 0 | 136 |
| Battle objects and panels | 100 | 27 | 3 | 2 | 0 | 0 | 6 | 0 | 35 |
| Actors, collision, status, HUD | 532 | 83 | 48 | 13 | 110 | 0 | 42 | 2 | 154 |
| Link status, battle settings | 2 | 10 | 1 | 0 | 0 | 2 | 1 | 0 | 15 |
| Custom screen, gauge, camera | 173 | 28 | 65 | 55 | 16 | 0 | 29 | 3 | 371 |
| Link layer | 1 | 11 | 0 | 0 | 0 | 51 | 16 | 0 | 110 |
| Object kinds, chips, navis | 2,483 | 104 | 89 | 1 | 0 | 0 | 12 | 0 | 2,650 |
| Elsewhere, run in a netbattle | 61 | 44 | 0 | 0 | 0 | 0 | 0 | 121 | 0 |

Object kinds, by handler:

| Type | Kinds | Ported or documented | Never spawned from a netbattle's code |
|---|---|---|---|
| Actors (T1) | 95 | 81 | 14 |
| Attacks (T3) | 213 | 133 | 80 |
| Effects (T4) | 146 | 102 | 44 |

No kind's handler is run or reachable without being cited. Nine of the unreached kinds have no spawn site in any
routine at all (three are empty handlers; T3 0xCD and T4 0x8D and 0x8F are spawned only from code nothing
references; T4 0x0C, 0x4C and 0x4D from nowhere).

## 3. What "run in matching recordings" covers

73 routines run in netbattles, are cited nowhere, and fall under no other class: the custom screen's drawing
helpers (the hand's chip icons, the cursor, the chip window: 0x080279C8–0x0802A394, about 50 routines), the
camera's update, stat accessors, save flag tests, and fragments of ported routines that the symbol table names
separately (`loc_801083E` inside the buster's shot, `sub_8010D04` behind the link navis' damage tables). Every
recording that runs them matches, so what they do to the simulation is reproduced or nil. They are listed in the
audit's `classes.tsv`.

## 4. Unaccounted and reachable: none left

The audit's first pass (before the link navis' charged attacks were merged) found one group: 65 routines that ran
only in the 11 recordings the engine then stopped in (`navis/navi-01` to `navi-11`, at "weapon routine 0x40 has
no script"). They were the weapon routines 0x30, 0x32 and 0x40 to 0x4A and entry 9 of each link navi's action
table:

| Navi | Weapon routine | Action (entry 9) | Routines | Objects |
|---|---|---|---|---|
| HeatMan | 0x40 `sub_80121DC` | `sub_80F070E` | 4 | his chip's flame (T3 0x26) |
| ElecMan | 0x44 | `sub_80F094C` | 4 | his chip's thunderbolt (T3 0x64) |
| SlashMan | 0x43 | `sub_80F0C48` | 4 | his chip's sword wave (T3 0x62) |
| EraseMan | 0x47 | `sub_80F0FB4` | 6 | a collision region, a hit effect |
| ChargeMan | 0x48 `sub_80122AA` | `sub_80F1198` | 10 | a collision region, his chip's cars (T3 0xAC) |
| SpoutMan | 0x41 | `sub_80F153C` | 3 | his chip's ball (T3 0x22) |
| TomahawkMan | 0x45 | `sub_80F17C4` | 4 | a collision region, a hit effect |
| TenguMan | 0x42 | `sub_80F19D4` | 4 | a collision region |
| GroundMan | 0x4A `sub_80122DA` | `sub_80F1BA8` | 5 | a drill of his own (T3 0xC6, `sub_80DDDF0`, 7 routines) |
| DustMan | 0x49 `sub_80122C2` | `sub_80F1F18` | 3 | DustCross's junk ball (T3 0xB0) |
| ProtoMan | 0x32 `sub_801206E`, 0x30 `sub_8012018`, 0x34 | the sword's and the shield's actions | 2 | none of his own |

All are in the content now (`navis/*/charge.luau` and ProtoMan's specials) and the 11 recordings match.

A second group was there all along and the audit didn't see it: weapon routines 0x21 to 0x26 (`sub_8011E40` to
`sub_8011F10`), the charged shots BugRSwrd, BgDthThd and the four arm chips give. §9 has what happened and the
check that now covers it. They are ported too, and the audit's rerun finds nothing in the class.

## 5. Out of scope, by rule

349 routines are reachable only through a routine a netbattle never enters. Each rule names the routine and the
condition that keeps a netbattle out; `excl.py` has all 50.

| What | Routines | Why a netbattle doesn't get there |
|---|---|---|
| Battle modes 1, 2 and 3 (`sub_800961C`, `sub_80099A4`, `sub_8009C94`, from `off_8007B50[GetBattleMode()]`) | 181 | all 192 link battle settings records have mode 0 |
| A custom screen's tutorial states (0x2C, 0x30, 0x34, 0x3C) and the tutorials' checks under `sub_80298F4`, `sub_8029A56`, `sub_8029B1C` | 23 | the screen's tutorial index (+0x0C) is 0xFF |
| Custom screen states 0x20 (the run-away result), 0x24, 0x40 (the Beast Link Gate) | 17 | no running (battle effects 0x20); `sub_802A220` needs the accessory (`sub_8120B54`) |
| The actor lists' entry types 1, 2, 6, 7, 0xA, and T4 0x41 that type 6 spawns | 31 | no link stage's list has them (battle-flow.md §3.2) |
| Battle mode 9's own code: T1 0x28, T3 0xD2, DustMan's actions 0xB and 0xC (weapon routines 0x4B, 0x4C), its stats (`sub_80135E8`); modes 1 and 7's stats | 30 | mode 0 |
| The virus actor and the navi the game plays (`sub_8108F50`, `sub_80F2330`: T1 kind 0 with ActorType 0 and 1), actions 0x5C and 0x5E | 28 | a netbattle's navis are ActorType 2; no form's or navi's row names weapon routines 0x52, 0x53, 0x6D |
| The per-player gauge mode (battle flag 0x40: `sub_802E2C4`, its structs and HUD) | 17 | only `sub_802E112` sets the flag |
| Dimming effects 31, 33 and 41 of `off_802CCB4` | 12 | no chip record names them |
| The end exchange and results (mode state 0x10), and the rewards (`sub_802CAA6`) | 9 | battle effects 2, which no link settings record has (they are 0x8C, 0x88C or 0x20088C) |
| The battle's end for link types 4 and 8 (`sub_8007C50`) | 1 | a cable netbattle is type 0 |

Three more things are out of scope without a rule in the audit, since no routine is theirs alone:

- **The random battle** (battle effects 0x200000; the 96 settings records from 0x60 on, another match type's) is
  out of scope by decision. The port keeps the flag (`setup::effects::RANDOM`) and what already reads it (the
  custom screen); the stats and folder the original takes from a second place for it (`sub_800B144`,
  `sub_800A3E4`) would be the setup's to supply, and nothing records it.
- **NaviStats+0x54** (HP lost when the custom screen opens, `sub_8013FD0`) stays 0 in a netbattle. Its one setter
  by name, `sub_813CF2C`, sits in a block of NaviCust stat setters (0x0813CEF8–0x0813CF6C) that no pointer word
  and no call in the ROM reaches. In a fight only bug code 0x54 raises it (`sub_80139F6`), and the one hit that
  carries that code is the tornado's variant 3 (`sub_80C9F98`), which only the Tornado action's subtype 3 spawns
  (`sub_80CA19E`); Tornado is subtype 1, Static 2, and no weapon routine starts the action. All 9,994 stat blocks
  of the lab's setups have it 0. The port has the stat, the code, the variant and the action
  (`tornado/back-spread`), all unreachable.

- **Two of the ruleset's actions can't start in a netbattle:**
  - `actions.turn`, the turn L or R starts (the original's action 0x3B, `sub_80EDF0C`), is ported since content
    model v2's step 13 (navis/megaman/turn.luau) and runs in no recording. The ruleset decodes L and R as a turn
    only for a navi that can turn (`sub_80141F4` sets the bit at the navi's init), and that routine returns
    without setting it when the stage's panel pattern is one of the standard columns (0x38, 0x30, 0x3C) or the
    battle is DustMan's mini-game. All 192 link stages have pattern 0x38, so L and R only ask for the custom
    screen.
  - `actions.volley`, a volley of buster shots (the original's action 0x30 entered from the status routine,
    `loc_801B006`, on request bit 0x40000000 of AIData+0x44), is a role left unfilled. That bit is tested there
    and nowhere raised: of the 26 calls of the request setter (`SetAIData_Unk_44_Flag`) none passes it, as a
    literal or as a shift, and the literal's six other uses are sprite attributes and object flags. The port's
    content API can name the request (`"volley"`), and nothing does.

The 3,175 unreachable routines are mostly the other 138 object kinds' (viruses, bosses, story objects) and what
only they call.

## 6. The other direction: is a ported routine's port complete?

### 6.1 Ported routines no recording runs

205 routines the source cites are never run by the lab (260 before the cut-in chips' recordings ran to their
end, 225 before the scenarios of the second batch below, 208 before the third's). Every one has been read: three
of the last 76 could run and have scenarios now, and none of the 205 can run in a netbattle. A citation is not a test: these are the port's
unverified parts, or content for something a netbattle can't do. By the file that cites them, with whether a
netbattle can run them where that has been read:

| Where | Routines | What | A netbattle |
|---|---|---|---|
| `content/bn6/lib/instant` | 26 | instant effects 2, 6, 9, 11, 16 and 17: the invisibility, repair, the immobilizer, the side special, the meteor shower, the dust storm and its motes | can't: no chip has them, and the weapon routines that name two of them (0x71, 0x83) are named by no form's or navi's row and no NaviStats byte |
| `crates/nettai-battle/src/kinds/obstacle.rs`, `content/bn6/objects/encased-bubble`, and the two requests' citations | 22 | an obstacle thrown (`sub_8018002`'s steps) or encased in ice or a bubble (`sub_801813A`'s, and the bubble it becomes); the actors' hit reactions on an obstacle | can't: the two requests are made only by routines nothing calls (field-objects.md §4.5), and nothing starts those reactions on an obstacle |
| `content/bn6/chips/airspin` | 13 | the seeking whirlwind (T3 0xD4): AirSpin's variant 1 | can't: no chip record sends it |
| `crates/nettai-battle/src/battle.rs` | 13 | the communication error (`sub_8007EB8`) and escape (`sub_800AAD6`) results, the set's second init entry, the per-player gauge mode's routines | not the simulation's (the link is the port's netplay), or battle flag 0x40 |
| `kinds/player/actions/cross_change.rs`, `entry.rs`, `hand.rs` | 17 | the Cross change and knockout while paused, the navi that appears mid-battle, the link navis' chips leaving the hand | can't: no live writer of the Cross change (unverified.md) |
| `content/bn6/lib/rapid_buster.luau`, `lib/dimming/blinding_flash.luau`, `chips/bugfix/glow.luau`, `navis/megaman/weapons/shield`, `navis/tomahawkman`, `chips/tornado`, `chips/mstrcros`, `chips/rskyhny` | 22 | weapon routines 0x39, 0x3C, 0x8C and 0x46; dimming effect 2; the glow's two other variants; the Tornado action's subtype 3; a sword phase and a bee action nothing sets | can't: nothing names the routine, effect, variant or phase (the NaviCust writes weapon routines 0x3B, 0x8B and 0x3D only; the link navis' level tables 0x30 and 0x34) |
| `crates/nettai-battle/src/collision.rs`, `field.rs`, `crates/nettai-frontend/src/objects.rs`, `chips/elecman` | 15 | IWRAM routines | run, not counted: the lab's coverage doesn't instrument IWRAM |
| `navis/megaman/turn.luau`, `chips/antidmg/counter.luau`, `kinds/player/chip_use.rs` | 4 | the turn (`sub_80EDF0C`); AntiDmg's counter aimed at the nearest enemy (`sub_8016218`); the special chip's gauge cost (`sub_800EE98`, `sub_802E830`) | can't: L and R never turn on a link stage (§5); the counter's variant is 0 wherever it starts (below); the special chip is battle flag 0x40's |
| `content/chips.rs` | 3 | damage formulas 0, 19 and 22: the opponent's HP, the custom gauge, half the opponent's max HP | can't: no chip record's damage is 1000, 1019 or 1022 (`gen-content check` reads them all) |
| `chips/lifesync` | 6 | LifeSync's aim, marker, warning and sync | can't: a link battle skips them (dimming-chip-effects.md §14) |
| `kinds/lockon_marker.rs` | 3 | the lock-on marker's choice between two targets | can't: a side has one combatant |
| `lib/bombs/slash.luau` | 4 | a bomb's lingering hit (attack object #0xA) | can't: no chip throws bomb kind 1 |
| `objects/follow-effect` | 4 | the follow effect's looks 3, 5, 6 and 8 | can't: no spawner passes look 3, and looks 5, 6 and 8 come from an AI navi's actions and an out-of-scope object (DeltaRay's 4 and ElecMan's 7 run) |
| `chips/lilbolr/layer.luau` | 4 | a layer's own flip, row offset, held sprite and visibility | can't: the viruses' settings |
| `chips/timebom`, `spoutman`, `airhocky`, `geddon`, `wavearm`, `objects/panel-bursts`, `objects/rock`, `chips/sandwrm` | 9 | TimeBom's blinking away, SpoutMan's water standing again, the puck's simple bounce, Geddon's poison and its row builder, the bursts' panels by offsets, a breaking wave, the rock's fall, SandWrm's hole waiting open | can't: the variants no chip sets (countdown rows 2 to 7, every puck crossing, no poison or breaking row), a second spout only his AI gives, every caller of the bursts passing an area, and the two documented in unverified.md |
| uncalled, or called only by code out of scope | 15 | the end-of-list spawn's twin (`sub_800333C`), the body overlay's held visibility, SlashCross's wave while dimmed, Beat's unread flag, an effect following its owner, the form overlay's stun hold, a push by any hit (`sub_801ADFA`), a navi's own wind, the overlay refreshes of other actor records (MegaMan's is `sub_80C44D2` in every form), DiveMan's AI, the virus update, the drain hits' healing | can't: nothing references them, or only routines the audit finds unreachable or a rule excludes |
| the ruleset's | 21 | turning to face an object (panel patterns 0x23, 0x31, 0x33), its unused setter, the HP bug of an actor without stats, an obstacle's bubble and its return to idle, the encased flicker, the rock thrown, the class lookup of an encased obstacle, the step that doesn't animate, the step's Land phase, the buffered step (both entries), the Cross change while paused (3), the linked object the status reset ends, bug code 0xFB, the Cross Beast table's step 0xC, the SELECT special's end, FullCust's side gauge, `nullsub_44` | can't: no link stage has those patterns; nothing sets the state bit, NaviStats+0x11, the Cross change, the linked object or bug code 0xFB; the obstacles' reactions and action 1 come only from the actors' and out-of-scope kinds' code; a Cross Beast's change writes its new form at step 8, and from then the Beast's Cross table runs (`sub_80154C8`, `sub_80155CC`); the SELECT special and the side gauge are battle flag 0x40's; §9 has `nullsub_44` |
| battle mode 9's | 2 | the objects the player of AI index 10 spawns | can't: a netbattle's mode is 0 |
| outside the simulation | 2 | `GetBattleSettingsUnk01` (settings byte 1), `LoadBGAnimData` (the extractor's backgrounds) | not the simulation's |

The audit's `unrun.txt` lists them. Scenarios written from this list (all match):

- First batch: ChargeMan stopped where the floor ends, with his cars out and with none
  (`navis/navi-05-volcchrg/charge-hole-*`); ProtoMan as the game has him below level 10, whose B+Back is the
  shield that only guards (`navi-11-stepswrd/level-5`: weapon routine 0x34); ElmntMan's four elements by the
  user's A press (`chips/0x10d-elmntman/press-*`: the lab had only the one his timeout draws); EraseMan's aim taken
  at each step (`chips/0x0ec-eraseman/press-*`); DeltaRay's three strikes, his return and burst
  (`chips/0x12f-deltaray/three-strikes`, on the falzar base: the gregar base's stage has rocks on two of the
  delta's corners).
- Second batch, the ones a real match is most likely to reach:
  - An Uninstll landing on a navi in each Cross and Cross Beast of both versions and in Gregar Beast Out
    (`chips/0x0b9-uninstll/folded-cross-*`, `folded-beast-gregar`): every form's NaviCust refresh
    (`sub_801469C`'s table) runs, but Beast Over's two. Those can't be reached by a hit: Falzar Beast Over is
    untouchable (§11) and Gregar Beast Over invulnerable for 0xFFFF ticks, and `folded-beast-over` and
    `folded-beast-over-gregar` show the Cannon landing nothing. (The Falzar one found the port's missing shoes.)
  - A Cross chosen while in Beast Out (`forms/falzar/beast-then-cross`: the Beast's Cross, `sub_80153EC` and
    `sub_801544C`).
  - The Beast rush chaining the next chip on an A press, and not chaining a variable sword, a dimming chip or an
    empty hand (`forms/falzar/beast-rush-chain`, `-sword`, `-varswrd`, `-dimming`: every branch of
    `sub_800FC30`).
  - Each dark chip with no BugFrags, which is its substitute (`chips/0x11e-drksword/no-frags` and the other four
    dark chips': `sub_8010D58`'s substitution, `sub_800EF02`), and DrkSword's last frag
    (`chips/0x11e-drksword/last-frag`).
  - Full Synchro's aura hidden while its navi is away for BugFix's glow, and shown again
    (`flow/synchro-bugfix`: `sub_80C4C46`, `sub_80C4C4C`). A damaging cut-in chip spends Full Synchro at its use,
    so the aura is gone before any of them hides its user; BugFix does no damage.
  - The AntiDmg program's stance catching a Cannon and a Sword (`navicust/antidmg-caught`, `-sword`:
    `sub_80105F2`). A sword is caught as AntiDmg's counter, not AntiSwrd's, and the shuriken goes at a random
    enemy (`sub_8016004`). The counter's other aim, the nearest enemy ahead (`sub_8016218`, its variant 1), is not
    reachable: the stance's weapon routine (0x3D, `sub_80121BC`) writes variant 0, and every trap's counter
    (`sub_801056A`'s six callers) passes 0.
- Third batch, the last 76 read: RlngLog's log stopped by a hit and rolling on (`chips/0x028-rlnglog1/buster-stops`:
  a buster hit on a landed log; the `shot` scenario's Vulcan breaks it at once, `sub_80D141A`); Sensor's turret
  pushed (`chips/0x073-sensor3/pushed`: side 1's AirShot on Sensor3's 40-HP turret, `sub_80DA37A`; Sensor1's
  breaks to it); GigaCan's shell flying off the field and bursting over the last two columns
  (`pa/0x140-gigacan1/miss`, `sub_80C5014`). Reading them also showed that the lab's generated `pushed`
  scenarios pushed only the RockCube and the countdown bomb: a side's own AirShot moves none of the objects it
  sets on its own panels. Fanfare, Anubis and Fan are pushed by side 1's now (`pushed-by-enemy`), and the
  descriptions say what the others show.

### 6.2 Branches that ran one way only

A branch every recording takes the same way has had only that side compared with the original. The audit
lists them (`onesided.tsv`, with the instructions before each). At engine main 5fd4bba6 with the lab's 5,036
matching recordings, 7,546 of the ported routines' branches ran and **2,192 (in 1,269 routines) ran one way
only**. After the work below, at main 51eeb22f with 5,151 matching recordings, 7,591 ran and **2,123 (in 1,255
routines)** did; 80 of the 2,192 are taken the other way by this section's 63 new recordings (the rest of the
drop is other scenarios added since, and branches that first ran now count in the total). At main 29aac858 the
list was 2,112; this section's batches 5 and 6 (22 more recordings) take 33 of them the other way, and with them
in the lab it is **2,080 (in 1,252 routines)**.

**How they are read.** The verification workspace has three tools for this, beside the audit:
`tools/audit/onesided_rank.py` files each branch under the engine area that cites its routine (collision and
damage, statuses and hit reactions, the custom screen and hand, movement, chip use and dimming, forms and flow,
the chip families), tags the guards whose other side is an error path (a spawn or collision slot that never
fails, a panel pointer off the field, the battle mode), and joins the verdicts read by hand;
`tools/audit/onesided_notes.py` holds those verdicts, one per branch address (a scenario that takes the other
side, unreachable with the reason, or reachable but not yet recorded); `tools/audit/annot.py` prints a routine
with the lab's merged block and branch counts, and says which one-sided branches a directory of new
recordings takes the other way. The scenarios are in the chip lab library's `coverage_scenarios/l_onesided.py`,
each naming the branch it takes in its description.

**What has been read**, in the order above: every area but the chip families (read in their own pass, the
subsection below) and a dozen presentation branches: the collision and damage area, the statuses and hit
reactions, the custom screen and hand (164 branches), movement (116, with 13 the first batches' recordings made
one-sided), chip use and dimming (135) and the forms and flow (447): 1,110 branches. 174 are taken by a new
recording, 515 can't be taken in a netbattle (the tables below), 421 can but have no scenario yet (the list after
them). The forms and flow were filed by kind first: their guards (the battle mode and effects, flag 0x40, the
pools, null players, NameID ranges) are in the tables, the reachable ones were filed by kind (an attack object
alive when the battle ends, an object during a dimming or a pause, a refused move, an attack off the field), and
260 were read but not pursued one by one. Batch 10 then took the likeliest of the reachable ones in real play:
the battle's end and the dimmings and pauses by kind, the dimming telop's states, the Beast lock-on's far panels,
and Gregar on side 1 (the chip lab's `falzar-gregar` base, from a replay whose side 1 plays Gregar). Batches
7-10's 55 scenarios are recorded but not yet replayed against the engine or in the lab: the lab was being
re-recorded when they were made.

**One difference was found, in the recordings rather than the engine.** A trace's setup didn't carry the save's
unlocks, so the replay read every save as a finished game's (`Unlocks::everything`); a save without Beast Out
(`custom/no-beast-out`) laid out a Beast Out button the original didn't have, and the cursor went elsewhere. The
recording tool now writes each console's event flag bytes (`unlock_flags`: flags 0xE0-0xEF and 0x160-0x167) and
the compat layer reads Beast Out, the version's Crosses and flag 0x163 from them (older recordings still read as
finished saves, which they are). Every other new recording matched at once: batches 1-4's 63 on every frame
(66,460) and in their sound calls (but Beast Over's rumble, a known difference), batches 5 and 6's 22 likewise
(31,000 frames).

**Four verdicts were wrong.** Bug code 0xF7 (the damage word's bit 0x1000) had been filed as carried by no hit.
EraseCross sets it on its damaging Null-family chips (`sub_8012C4A`), and on a navi with a 4 in its HP it raises
the HP bug; the engine has both (`chip_use`, `intake`). `forms/gregar/cross-erase-001-hp-4` now records it.

| Scenarios | What they take the other way |
|---|---|
| `chips/0x0b5-bblwrap/blown-away`, `regrow-dimmed`, `regrow-bubbled`, `regrow-frozen`; `chips/0x0b3-barr100/holy` (`sub_801A802`) | BblWrap's bubble popped by AirShot's wind; its regrowth held by a dimming, a bubble and a freeze; a barrier on a holy panel taking half |
| `chips/0x025-rskyhny1/fire-hit`, `navicust/antidmg-barrier`, `chips/0x0bc-antiswrd/guarded`, `chips/0x0bb-antidmg/cursor-hit` (`sub_802CEF4`, `sub_802CFF8`) | fire passing RskyHny's hive; the AntiDmg program's stance taking a hit a barrier emptied; a guarded slash not springing AntiSwrd; a cursor hit (MachGun) dropping AntiDmg |
| `pa/0x14e-destpuls/hit`, `pa/0x14e-destpuls/drain` (`sub_801A720`, `sub_80102AC`) | DestPuls's bug code 0xF6; the custom-screen HP drain stopping at 1 HP |
| `forms/falzar/cross-tomahawk-grass-low-hp` (`sub_801A186`) | a Wood body on grass at 9 HP or less healing every 180 ticks |
| `navis/navi-07-etomahwk/after-geddon` (`object_breakPanel_dup3`) | ETomahwk's strikes over holes |
| `forms/falzar/cross-spout-hold-switch`, `navicust/shield-b-then-back` (`sub_8012FC8`) | A held then B pressed, B held then A pressed (the charge moves); Back pressed in the B+Back window with B released |
| `navis/navi-01-heatpres/custom-open`, `weakness-hit` (`sub_8015A16`, `sub_801AF44`) | a link navi's custom request; a weakness hit on a navi outside the Cross forms |
| `flow/counter-in-gregar-beast`, `forms/falzar/beast-out-spent-countered` (`sub_801A200`) | a counter landed in Gregar's Beast; a counter on a tired navi giving no Full Synchro |
| `forms/falzar/beast-over-drained`, `navicust/bug-emotion-tired` (`sub_8014498`, `sub_8013DA0`) | the exhausted navi's drain stopping at 1 HP; the emotion-swing bug resting once the Beast Out counter is spent |
| `forms/gregar/cross-heat-beast-weakness` (`sub_8015766`) | a Gregar Cross Beast broken by its weakness |
| `chips/0x01b-bblstar1/then-cannon`, `chips/0x17c-icecube/cut-in-refused` (`sub_801BADE`, `sub_800BEDA`) | a bubbled navi taking damage; no counter cut-in on IceCube |
| `chips/0x004-airshot/superarmor`, `forms/falzar/cross-spout-ice-airshot`, `cross-spout-ice-wind`, `stages/shoes-roads-wind` (`sub_80178D4`, `sub_8017992`, `sub_8016730`) | a drag on a SuperArmor navi; an Aqua body dragged and pushed onto ice; shoes stopping a push on a road |
| `stages/ice-slide-cannon`, `-thunder`, `-widesht`, `-bblstar`; `flow/mash-paralysis`, `-freeze`, `-bubble` (actions 3, 4, 6, 7) | a flinch, paralysis, freeze and bubble entered mid-slide (the hits timed into a 4-tick slide); mashing out of them |
| `custom/refusals`, `sixth-pick`, `redeal-right-half`, `scrap-right-half` (`sub_8028B74`, `sub_8028CCC`, `sub_8029032`, `sub_8028DD6`, `sub_8028E04`) | A on a greyed chip; B with nothing to take back; R and UP on OK; DOWN with no slot below; UP from the bottom row; the re-deal and scrap buttons' right halves, pressed again once used or with nothing picked |
| `forms/gregar/cross-elec-001-charged`, `cross-slash-04c-charged`, `cross-charge-014-charged`, `cross-charge-beast-014-charged`, `forms/falzar/cross-tomahawk-001-charged`, `cross-ground-001-charged` (`sub_8013236`) | ElecCross charging a Null chip, SlashCross an element sword, ChargeCross and its Beast a Fire chip; TomahawkCross and GroundCross with chips that don't charge |
| `navis/navi-05-volcchrg/charge-family-chip`, `navi-06-dripshwr/…`, `navi-07-etomahwk/…` (`sub_8013236`) | ChargeMan, SpoutMan and TomahawkMan charging their family's chips (`byte_8021369`) |
| `navis/*/chip-001`, `chip-06e`, `chip-071`, `chip-0bc`, `chip-08d`, `chip-09a` (`sub_800F09E`) | the link navis with a chip outside their family, a dimming chip of it, and a chip without damage (their chip bonus) |
| Batch 5: `flow/paralyzed-then-frozen`, `-bubbled` (`sub_800E730`) | a freeze and a bubble entered over the paralysis's saved state |
| `chips/0x024-elcpuls3/bug-level-cap` (`sub_80139F6`) | ElcPuls3's bug code 0x18 capping a bug level at 7 |
| `custom/no-cross-offered` (`sub_8028B74`) | UP on the grid with no Cross offered (worn out after Beast Over) |
| `forms/falzar/cross-tengu-hole-knockout` (`sub_800E618`) | the base form, without shoes, stepping off the hole its TenguCross was knocked out over |
| `forms/gregar/cross-slash-beast-sword-charged` (`sub_80EAF26`) | the Beast's rush with a charged sword (action 0x41) |
| Batch 6: `custom/cross-window-keys`, `one-cross-owned`, `take-back-cross` (`sub_8028A78`, `sub_8029EF8`, `sub_8029032`) | UP and DOWN wrapping and moving in the Cross window, SELECT, START; a save owning one Cross (a window of one, no move sound); the window reopened with a Cross chosen, and B taking the Cross back |
| `custom/take-back-beast-out`, `chips/0x13f-beastout/take-back`, `custom/beast-out-greyed` (`sub_8029032`, `sub_8028D6C`) | B taking back the Beast Out button and the BeastOut chip; A on a Beast Out button a chosen Cross greys |
| `custom/invalid-after-pick`, `custom/hide-window-later`, `custom/no-beast-out` (`updateCustomScreen_WhenUnselectingChip_8028EC8`, `sub_8026D06`, `sub_8029FB4`) | invalid chips left selectable after a valid pick; SELECT on a screen after the round's first; a save without Beast Out |
| `navicust/chpshufl-redeal-last-chips` (`sub_8029688`, `sub_8029788`, `sub_802983C`) | ChpShufl's re-deal with the folder's last five, its last three (fewer than the hand) and none |
| `forms/falzar/cross-dust-beast-hand`, `cross-dust-scrap-take-back` (`sub_802A40C`, `sub_8027F10`, `sub_8028F84`) | DustCross's Beast at the screen (no NumbrOpn hand, the scrap button); the scrap button with nothing picked |
| `navis/navi-01-heatpres/own-chip-twice` (`sub_80280A2`) | a link navi's chip, used, not offered again in the round |
| `forms/gregar/cross-charge-hand-size`, `-bug` (`sub_802A49C`, `sub_802A40C`) | ChargeCross's hands of 8 and past 8, with and without the custom bug: the Gregar side's screen, which the coverage (the Falzar console's own screen) can't credit; the engine matches every screen |
| Batch 7: `navis/navi-05-volcchrg/charge-family-chip-level-1`, `navis/*/charge-other-chip`, `navi-05-volcchrg/charge-dimming-chip` (`sub_8013236`, `sub_80F0608`) | ChargeMan below his charge level (3) and at his level's cap of 0; ChargeMan, SpoutMan, TomahawkMan and ProtoMan holding A on a chip outside their family, and ChargeMan on a damaging dimming chip |
| `forms/gregar/cross-elec-09a-held`, `-0dd-held`, `cross-charge-06e-held`, `cross-slash-131-held` (`sub_8013236`) | ElecCross holding A on Recov10 (no damage) and Roll (dims), ChargeCross on BurnSqr1 (dims), SlashCross on BugRSwrd (no damage) |
| `forms/falzar/needlarm-then-beast`, `forms/gregar/needlarm-then-beast` (`sub_800FFAA`) | both Beasts' busters giving way to an arm chip's charged shot |
| `navicust/bug-movement-airshoes`, `beat-standard-chip`, `rush-other-chip`, `antidmg-caught-side1`, `custom/invalid-chip-side1` (`sub_8010368`, `sub_80106C0`, `sub_8010740`, `sub_80105F2`, `sub_80F0354`) | the astray step's dash rule with AirShoes; Beat and Rush passing a chip they don't take; AntiDmg caught and the invalid chip used on the console's inverted side |
| Batch 8: `forms/gregar/cross-heat-09a-used`, `cross-erase-001-used`, `cross-erase-beast-001-used`, `cross-elec-beast-001-used`, `forms/falzar/cross-ground-09a-used`, `navis/navi-11-stepswrd/chip-06e-used` (`sub_800EF34`, `sub_8012C4A`, `sub_8012C7C`, `sub_8012BA2`) | HeatCross and GroundCross using a chip without damage; EraseCross, its Beast and ElecCross's Beast using a Cannon; ProtoMan using a damaging dimming chip |
| `forms/falzar/beast-over-chips`, `beast-over-from-cross`, `cross-ground-then-tengu`, `beast-rush-top-back`, `forms/gregar/beast-over-cannon` (`sub_8012ABC`, `sub_801516C`, `sub_8014B18`, `sub_8026622`, `sub_80EAE28`) | Beast Over using a damaging dimming chip and a Null chip; Beast Over from TenguCross; a Cross change from GroundCross; the Beast's rush at a target in its row; Gregar's Beast Over rushing |
| Batch 9: `flow/step-count-saturates`, `forms/flow/judge-win-gregar`, `forms/gregar/cross-heat-beast-buster-attackmax` (`sub_800AB46`, `sub_802CBCC`, `sub_802CC50`, `sub_8011B4A`) | a side's step count saturating at 0xFF; the judge on the console whose local side is 1; HeatCross's Beast buster capped at level 5 |
| Batch 10: `forms/gregar-side1/beast-over`, `crosses` (`sub_800A1D0`, `sub_80D8EE4`, `sub_800362C`) | Gregar's Beast Over on side 1; EraseCross's ray from side 1 reaching column 0; an object drawn past the screen's right edge |
| `forms/gregar/cross-erase-001-hp-4` (`sub_8019F44`, `sub_801A6B4`, `sub_801A4A6`, `sub_80139F6`) | EraseCross's bug code 0xF7 raising the HP bug of a navi with a 4 in its HP |
| `forms/gregar/cross-erase-beast-drop-ko`, `cross-charge-beast-wave-ko`, `cross-slash-beast-lunge-ko`, `navis/navi-09-rc-brakr/ko-running`, `stages/volcano-eruption-ko` (`sub_80D9746`, `sub_80DDB24`, `sub_80E88D8`, `sub_80DDE82`, `sub_80C5AD4`) | attack objects running when the battle ends: EraseCross Beast's drop, ChargeCross Beast's wave, SlashCross Beast's hit flashes, GroundMan's drill, an eruption |
| `forms/falzar/cross-dust-junk-dimmed`, `navis/navi-09-rc-brakr/drill-dimmed`, `forms/falzar/beast-rush-paused`, `forms/gregar/cross-slash-beast-charging-paused`, `flow/synchro-paused` (`sub_80DB726`, `sub_80DDE82`, `sub_80E336E`, `sub_80E0E20`, `sub_80C4B18`) | objects under the other side's dimming (DustCross's junk ball, GroundMan's drill) and custom screen (the Beast rush's afterimages, a chip's charge glow, the Full Synchro aura) |
| `forms/falzar/beast-trnarrw1-target-forward`, `beast-trnarrw1-side1` (`sub_80264A8`) | TrnArrw1's lock-on panels off the field's left and right edges |
| `flow/cut-in-deletes-user-areagrab`, `-antidmg`, `flow/counter-of-counter-areagrab` (`object_drawChipName`, `sub_800BBA8`, `object_timefreezeEnd`) | a counter cut-in deleting the user before its telop ends (a plain telop, and a trap chip's hidden one); a counter to a counter cut-in, whose first counter ends while the dimming is the other side's |

Two more scenarios record a side the branch list can't credit: `chips/0x081-wind/twice` (a second fan evicting
the first through the field-object registry; the earlier `then-fan` never placed its Fan, the panel being taken)
and `flow/counter-ko` (a counter that deletes: it is booked before the deletion ends the battle).

**Not reachable in a netbattle** (515 branches; `onesided_notes.py` has every address). First the collision,
damage and status areas (and the first of the others), 130:

| Routines | Branches | Why |
|---|---|---|
| `sub_8012FC8`, `sub_80158CC`, `sub_80159C6`, `sub_8016860`, `sub_80F22F8`, `sub_80142DC`, `sub_8028B74` | 10 | battle mode 1 or 9; a netbattle is mode 0 |
| `sub_80107D4`, `sub_8012FC8`, `sub_801728E`, `sub_801A45C`, `sub_802EF5C` | 5 | battle flag 0x40 (per-player gauges): only `sub_802E112` sets it |
| `applyDamageToPlayer_801ba12`, `sub_80139C4`, `sub_8014326`, `sub_8016934`, `sub_8017AB4` | 5 | the actor type is 2 for both navis |
| `sub_8009338`, `sub_80102AC`, `sub_8015994`, `sub_80159C6`, `sub_8015A16`, `sub_8015BEC`, `sub_8016934` | 8 | a null player object: both navis exist all battle |
| object spawns, `object_createCollisionData` and the collision region | 9 | the pools never fill |
| the panel break, crack and reservation routines, `sub_801A36A` | 5 | a panel pointer off the field: every caller passes a panel on it |
| `object_breakPanel` | 2 | its callers pass a panel on the field and never an occupied one (CrakShot breaks only an empty panel, SunMoon's meteor only one holding nothing); a panel that isn't solid it does meet, from SunMoon's meteor over a hole (the chip families' second pass, batch 4) |
| `sub_801A802` | 5 | barrier type 0xA and the weak elements of types 0xB-0xE: only a navi AI raises them |
| `sub_80139F6`, `sub_801A4A6`, `sub_8019F44` | 8 | bug codes 0x54, 0xF4, 0xF9-0xFF: no hit a netbattle has carries them; the damage word's bit 0x800, whose test branches to its own fall-through |
| `sub_801A2CC`, `sub_801A324`, `sub_80C532E` | 3 | collision rows 3, 8 and 9 (a chip-erasing hit, drain hits) are used by no attack; the region's report pointer is zeroed at its spawn and set by no caller |
| `sub_8010162`, `sub_8010198` | 3 | the timed submerged state: its one starter, actor #0x5D variant 1, is never spawned |
| `sub_801AC6C` | 2 | a dead navi while the battle isn't over (one navi a side); NaviStats+0x52 has no writer and is 0 in all 10,072 lab stat blocks |
| `sub_8017BC0`, `sub_802DD2A`, `applyDamageToPlayer_801ba12`, `sub_801AF44` | 11 | the Cross change, Cross death, volley and UNINTERRUPTIBLE: nothing in a netbattle raises them |
| `sub_801AF44` | 1 | a weakness hit past NameID 0x1C1: only the Beast Over forms are there, Null with no weakness |
| `sub_80143CE`, `sub_8015BEC`, `sub_801A200` | 4 | Beast Over's exhaustion without the spent Beast Out counter: only the emotion-swing bug clears the counter's flag (+0x32), it rests while the counter is 0, and it clears the flag just before it rolls anger |
| `sub_8015C12` | 1 | mood 0: its one writer, at Beast Over's end, is blocked by the spent counter |
| `sub_80E541A` | 1 | the wind registry's replacement: a second fan evicts the first through the field-object registry before it registers as the wind; the other wind object is T4 0x41, an actor-list type no link stage has |
| `sub_8028B74`, `sub_8028CCC`, `sub_8028D6C` | 5 | the tutorials' checks (index 0xFF in a netbattle); the pick count test, which five picks make dead (they grey every slot, so the state test refuses first) |
| `sub_800F09E`, `sub_8013236` | 5 | no Wind- or Break-family chip both damages and dims, and none lacks damage; a link navi's level is never 0xFF |
| `sub_8012FC8` | 4 | turning (every stage has the standard column pattern); a flipped navi; a navi without a buster or a charged shot |
| `sub_800E994`, `sub_8016852`, `sub_800E548`, `sub_8017BC0`, `sub_80178D4`, `sub_80159C6` | 7 | dead code: tests whose register the code before fixes (dx = 0; a slide that moves; a direction passed; a word just zeroed; a panel test repeated on the same tick; a stat byte just tested) |
| the rest, one to five each | 26 | panel types past 0x0C and forms past 0x18; the field's init; the actor list's two boulder slots; statuses past 0x65; AFFECTED_BY_ICE set with the NaviCust flags; the hand cursor's slot after a chip is used; the Beast Out check's delay (1 or 2 at a request); anger with its timer at 0; a link battle's effects; a cut-in chip's action (0x15 or 0x1B) and a cut-in with no chip; the navi number; the players' spawn parameter; the charge glow, which lives all battle; the escape |

Then the 37 left reachable above, read again (6), and the custom screen and hand (112):

| Routines | Branches | Why |
|---|---|---|
| `sub_80E1352`, `sub_80E13DC` | 3 | the dimming overlay's bits 4 and 5: every caller passes 0 or 0xF; a navi submerged when its dimming ends (no starter) |
| `sub_8028B74` | 2 | R on slot kinds 6 and 7: nothing writes them (dealt chips, a link navi's own among them, are 0; the buttons 2, 4/5, 8/9) |
| `sub_801390C` | 1 | AIData+0x50: no object links itself there |
| the folder build, the screen's setup, the hand size, the Cross window's and the PA's | 22 | battle mode other than 0 |
| the same and `sub_8026DC4`, `sub_8026E98`, `sub_8026EC8`, `sub_8029F70`, `sub_8029FB4` | 11 | the battle effects, fixed in a netbattle |
| `sub_8027D78`, `sub_8029F70`, `sub_8029FB4` | 4 | battle flag 0x40 |
| `sub_8026B04`, `sub_8026DC4`, `sub_8026EC8`, `sub_8028A78` | 5 | the tutorials (index 0xFF) |
| `sub_8026CCC`, `sub_8026D06`, `sub_8026E4C`, `sub_8028D3A` | 4 | the Beast Link Gate accessory: `sub_802A220` answers 0xFF without it |
| `sub_8029F70`, `sub_8029FB4`, `sub_8026840` | 3 | event flag 0x163 with MegaMan: the flag marks a link navi operated |
| `sub_8027E2C`, `sub_802A49C`, `sub_80280A2` | 3 | the screen's +0x15 is the navi, written with +0x10 and nowhere else; navi numbers end at 0x0B |
| `sub_8028E4C`, `updateCustomScreen_WhenUnselectingChip_8028EC8` | 5 | chip codes past 0x1C (0x1B is the invalid chip's, no chip has 0x1C), and tests after the codes past 0x1A are handled |
| `sub_8029110`, `sub_80293F8` | 2 | a selection that is neither a chip nor the Beast Out button |
| `sub_80280E0`, `sub_802723A`, `sub_802750C` | 3 | the re-deal and scrap buttons' second use: one a screen |
| `sub_8029688`, `sub_8029788`, `sub_802983C` | 5 | a hand size of 0 (it is at least 2); the re-deal's split table (`byte_80298C8`) is all zeros |
| `sub_800A3E4`, `sub_800A570` | 2 | the folder rules: one Giga a folder |
| `sub_801FF18` | 3 | the link's plumbing: its mode argument, the remote input word's link bits, the communication error (top state 0x0C) |
| `sub_802B7A0`… `sub_802B80C`, `sub_8029520` | 4 | the PA telop's dead test and its chips below 0x160; the PA veto (+0x1C) |
| `sub_802A00C`, `sub_8027834`, `sub_8028A78`, `sub_80294E0`, `sub_80109A4` | 5 | the BeastOut chip named twice in the role table; the window's 20-tick fallback (it opens in 12); a marked Cross with none chosen; a scrapped chip always finds its gap; chip id 0xFFFF |
| `sub_802A49C`, `sub_802A40C` | 4 | ChargeCross's extra chips in the Falzar console's own screen (ChargeCross is Gregar's; the Gregar side is recorded and matches) |
| the chatbox | 27 | commands and states the battle's scripts (descriptions and the twelve run messages, all recorded) don't use: portrait variants, F1 speeds other than 0, E4, a fourth line, the masked keys |

And movement and the player's own routines (78):

| Routines | Branches | Why |
|---|---|---|
| the player's init, its cross change, the step, the supports and the HUD | 10 | the battle effects, fixed in a netbattle (bit 8, the link battle's; bit 4) |
| `sub_8010332`, `sub_800FEEC`, `sub_800FF5E`, `sub_80F0354`; `sub_800A8F8`, `sub_802DFC8`, `sub_802E4E4` | 7 | battle mode 9; battle flag 0x40 and its SELECT special |
| the player's routines | 12 | the actor type is 2 for both navis and AIData+2 (an AI actor's) is 0; a null player object |
| the step (`sub_80F02A2`), `sub_800FD0A`, `sub_801002C`, `sub_8012F3E` | 9 | state bits 0x200 (no charge) and 0x8000: `sub_8010312`'s callers pass neither |
| `sub_80F0354`, `sub_80EA734`, `sub_80EB128`, `sub_80141F4` | 6 | requests with no setter (0x20, 0x80000), the crossed link navi's fall-back; turning (every stage has the standard column pattern) |
| `sub_80EB088`, `sub_80EB1C4`, `sub_800F964` | 4 | the buffered step (mode 1) has no starter and no step a mode past 3; a step while sliding, which `object_canMove` refused first |
| spawns, AIData and collision data | 6 | the pools never fill |
| `sub_801BCF4`, `object_updateSprite`, `sub_801DB84` | 7 | a live navi's invariants (active, animating through dimmings, with collision data); objects without flag 0x10 don't run during a dimming; the HP-number registry never holds an object twice or seven at once |
| `sub_8018810`, `sub_801A77A` | 4 | NameIDs: a navi's is past 0xFF, and none is in the story bosses' 0x173-0x17E |
| the rest, one or two each | 13 | forms past 0x18; the panel bug's type (always 3, a crack); the buster's attack level (at most 7); the chip-enable bits, set before input; a palette index; a link navi's level (never 0xFF, and the floor is 0); every Fire chip damages; NaviStats+0x3D (no writer found); two dead tests |

Chip use and dimming (53):

| Routines | Branches | Why |
|---|---|---|
| `sub_8012AFA`, `sub_8012B4E`, `sub_8012BA2`, `sub_8012ABC` | 4 | `sub_8012A38` calls the element tests only with damaging chips: their no-damage sides are dead |
| the transforms, the lock-on decision, Beast Over's range | 12 | forms below 1 or past 0x18 |
| the dimming stand-ins, the falling rocks, the transform effects | 12 | the pools never fill |
| `sub_800ED90`, `sub_800EDD0`, `sub_800EF02`, `sub_80BAF74`, `sub_800EB6C`, `object_getEnemyByNameRange` | 6 | the actor type is 2 for both navis; a null player; a navi's NameID never below the range |
| `sub_8012CB2`, `sub_8012D24` | 4 | one combatant a side (GroundCross Beast's rocks: never no target, never two or three) |
| `sub_8012ABC`, `sub_800EF34`, `sub_80EAF36` | 3 | battle mode 1 |
| the rest | 7 | a live navi's flag 8, state bit 0x8000 (no setter), turning (the standard column pattern), the lock-on selector's target row (1-3) |
| `object_drawChipName`, `sub_800BA8A`, `sub_800BBA8`, `sub_800BE2C` | 5 | the other side's dimming state 0 at a telop's end while the dimming is its own: its record goes back to 0 only when its controller ends (which waits for this side to be past its telop), when this side's ends it, or when AntiNavi turns its navi chip and hands the dimming to the taker; the hidden telop shown to both sides (the trap controller's +7, never written); AntiNavi's record without its user (written and cleared together) |

The forms and flow (136):

| Routines | Branches | Why |
|---|---|---|
| the attacks' and effects' objects | 51 | the pools never fail (spawns, collision data, collision regions) |
| the flow, the transforms, the HUD | 21 | battle modes other than 0 |
| the flow, the music, the HUD | 16 | the battle effects and settings, fixed in a netbattle (with the Beast Link Gate accessory) |
| the player's routines, the navi warp | 13 | a null player object or an actor type other than 2 |
| `sub_800A7A6`, `sub_8016460`, `sub_801DC7C`, `sub_80E1566`, `sub_80EAFC2` | 14 | NameIDs: the story bosses' 0x173-0x17E, the viruses' below 0x100 |
| the flow's registries | 6 | never an object twice, never full |
| the rest | 15 | battle flag 0x40; forms past 0x18; NaviStats+0x54 (§5); the buster's level (BusterUp's cap, Falzar's Beast at most 5); EraseCross's charge only in EraseCross; the battle's subsystem always in use; the link check |

**Reachable, no scenario yet** (421; the notes say what was tried): a status entered by its flag with no request
and nothing saved, or over a stale ice or bubble visual (`sub_800E730`, 5: paralysis then freeze and bubble,
their reverse orders and a dimming during a paralysis don't reach them); the status visual hidden from a blind
viewer or ended by its flag with its link left (`sub_80E0954`, 3: FlshBom3 then Discord, and Discord with
FlshBom1, saw neither); a weakness hit with no damage (`sub_801A42E`, `sub_801A506`); a counter booked on a
double KO; two reservations of one panel; a side's dimming registered by a statue or trap while the other's telop
shows; a second cut-in press with a request pending; the alternative A-charge's request at the charge tests; a
save without Beast Out's counter; a push onto a road whose next panel is blocked; a counter-paralysis request on
a paralyzed navi; the charge cut short mid-hold; on the custom screen, the slot link bytes (no writer found), a
cursor move whose neighbour is its own slot, the scrap button in state 3, a scrap with fewer chips left than the
hand, a sequence PA with two * chips, tired on a round without Beast Out, a save starting battles in a Cross, and
two telop and HUD states; in movement and the player's routines, the auto-step bug (stat 0x11), a battle started
in a form, the panel bug over panel types 0 and 1, an astray step past a panel the dash rule refuses, the Beast
form's throw (buster 0x2C) under an arm chip's charged shot, DestPuls on a navi whose bug levels are 6 or more,
and a dozen single states (the sequencer's waits, idle states, a deletion's link); the emblem's form source; and
a few single branches, listed in the notes. Two (`object_canMove`, `sub_80EB088`) are taken by another agent's
`forms/gregar/beast-over-swords`, which matches only on a newer main. In chip use and dimming (60), most are the Beast lock-on's
fallback panels (reached only when the first candidates are refused; TrnArrw1's off the field's edges are
recorded), the form overlay's presence in the transforms, the element tests' charged-chip states, and
presentation (the sprite loader's cache, the banner's text lookup). In the forms and flow (295), 32 are filed by
kind (a refused move, an attack off the field, and the attack objects past the battle's end or under a dimming or
pause that the tries didn't catch: SpoutCross Beast's surge, VolcChrg, DustBrk, ETomahwk's axe, an obstacle under
AreaGrab), 258 were read without being pursued one by
one (single states of the forms' and link navis' attacks, the obstacles' status hits, the sequencer's waits, the
HUD's emotion window and banners, the lock-on marker, the afterimages, the berserk auto-battle, the music), and the
other five are DustCross absorbing three identities and a battle started in a Beast.

The first sample, of 10 routines read before this list was worked through, as it stands now:

| Routine | The side never taken | The port |
|---|---|---|
| `sub_801A200` (Full Synchro from a counter) | the attacker in a Cross or a Beast; the victim tired after Beast Out (AIData+0x32, +0x36) | has both (`counter_and_mood`, `set_mood`). Recorded: `flow/counter-in-cross`, `flow/counter-in-beast`, `flow/counter-in-gregar-beast` (the 0x0B compare), `forms/falzar/beast-out-spent-countered` (the tired victim). The exhausted victim (+0x36) can't be reached (the table above) |
| `sub_8029224` (modifiers) | Uninstll after a damaging chip that dims | has it. Recorded: `custom/modifier-uninstll-dimming` (Roll then Uninstll stay two chips) |
| `sub_8013E58` (the status bug) | six of its eight outcomes (one RNG draw a recording) | has all eight |
| `sub_801A45C` (counter bookkeeping) | the gauge bonus under battle flag 0x40; the battle over | has both; the first is not a netbattle's, the second needs a double KO on the counter's tick |
| `sub_8013FD0` (HP lost at the custom screen's opening) | NaviStats+0x54 nonzero, in 1,292 openings | has it (`custom_hp_bug`, and bug code 0x54 that raises the stat); a netbattle can't make it nonzero (§5) |
| `sub_8015C12` (mood wear) | a mood of 0, in 4.2 million calls | has the test; mood 0 can't be reached (the table above) |
| `sub_8029520` (Program Advances) | the veto (+0x1C nonzero) | documented as unable to fire |
| `sub_8009338` (the custom screen's mode state) | the UI's result 2, the escape | not ported; a netbattle has no running |
| `sub_801002C` | NaviStats+0x10 nonzero | a palette index: presentation |
| `sub_80D6BD4` (ElmntMan's meteor) | its third state, a plain destroy (the lab never runs the meteor; the soundmod trace does) | the definition's destroy lifecycle |

None of the one-sided branches read, in the sample or since, is missing from the port.

#### Chip families: G4

The chip families' branches (`onesided_rank.py`'s area 7: routines cited by content/bn6, 1,066 branches in 720
routines at main 29aac858) are read family by family, most-played chips first. The verdicts are in
`onesided_notes.py`'s "chip families" section, and the scenarios in the chip lab library's
`coverage_scenarios/m_onesided_chips.py`. At main ecdef997, with the lab's 5,279 recordings (batches 2 and 3's 39
among them, all matching), 1,010 branches of the area still ran one way only: 258 guards, 365 unreachable in all,
11 hard, 634 not yet read.

**The guards (258):** a spawn or collision slot that never fails (255) and a panel pointer off the field (3).
Each was checked against its instructions: the side never taken is the failure's, which a netbattle's pools
and field never produce.

**Batch 1, the swords (33 branches):** 11 taken by 8 new recordings (and four more branches of the movement
and chip-use areas with them), 22 unreachable:

| Scenarios | What they take the other way |
|---|---|
| `forms/gregar/beast-over-swords`, `beast-over-neovari` | a Sword's blade in Gregar Beast Over (form 0x17, `sub_80EBAE8`); VarSwrd's and NeoVari's random pick there (`sub_80EF6FC`, `sub_80EF87C`) |
| `navis/navi-11-stepswrd/varswrd-released`, `varswrd-longswrd`, `neovari-released` | link navi ProtoMan waiting for a command where MegaMan takes the plain Sword, and keeping his own way after one |
| `forms/gregar/cross-slash-beast-varswrd`, `cross-slash-beast-neovari` | SlashCross Beast (form 0xF) waiting for a command as SlashCross does |
| `chips/0x0bc-antiswrd/sprung-ko` | sonic booms spawned with the battle over: the counter's first boom deletes the swordsman, and the two swings after it still throw theirs (`sub_80CF810` tests it once, at the spawn) |

The unreachable ones: the slot-in chip's paths (battle flag 0x40's special source, 5), forms past 0x18 (4), an
AI's actor or a link navi of AI index 0x13 (6), a step sword's step finding the navi moving or sliding (2:
a chip's action starts from idle), the step sword's turn flag (2: cleared just before its test), a sword
sub-type past 0xF (1), the command matcher completing a sequence in the call that matches its last direction
(1: every sequence's terminator is followed by a direction or code, so the word it tests is never zero), and
the sonic boom's spawn failing (1).

**A difference found and fixed.** `forms/gregar/beast-over-swords` showed one: the Beast rush warps a navi back
two panels, and on an ice panel the original then slides it back where the engine didn't. The move direction
(`sub_800E994`, what `object_updateCollisionPanels` records and the ice slide reads) treats only a move of two
panels or more to the right or down as "other": the routine tests `>= 2` and nothing below -1, so a move left or
up along one axis counts by its sign. The port had made every multi-panel move "other". It no longer does, and
the full lab matches (5,177 of 5,178 recordings; the other, `custom/no-beast-out`, is a new custom-screen
recording whose own engine change is still on another branch).

**Batch 2, the bombs, TankCan's shell, TimeBom and the projectile (59 branches):** 22 taken by 20 new
recordings, 36 unreachable, 1 hard:

| Scenarios | What they take the other way |
|---|---|
| `chips/0x036-minibomb/off-field`, `0x044-grasseed/off-field`, `0x039-flshbom1/off-field` | a thrown bomb landing past the field's edge (AreaGrab, then the throw from the front of the taken column) |
| `chips/0x036-minibomb/into-hole`, `0x044-grasseed/into-hole`, `0x039-flshbom1/into-hole`, `0x043-bugbomb/into-hole` | a thrown bomb landing in the holes stage's hole at (4,3) |
| `chips/0x044-grasseed/battle-over`, `0x039-flshbom1/battle-over`, `0x043-bugbomb/battle-over`, `0x00c-tankcan1/battle-over` | the battle ending while a seed or TankCan's shell flies, or a FlshBom or BugBomb stands (the shell's case through SlashMan's dimming, which holds the shell until the battle is over) |
| `chips/0x03c-blkbomb/fire-dimmed` | BlkBomb set off by HeatMan's flame inside his dimming: its leaving waits out the dimming |
| `chips/0x043-bugbomb/dimmed`, `all-four-bugs` | a BugBomb that has set down running through a dimming; the fifth BugBomb on a navi with all four bugs picking among all four |
| `chips/0x039-flshbom1/side-1`, `chips/0x00c-tankcan1/bottom-row` | FlshBom's own-body test for side 1; TankCan's explosion panel below row 3 |
| `chips/0x090-timebom1/no-room`, `onto-reserved-panel`, `pa/0x14f-timebomplus/explodes` | TimeBom with no free enemy panel ahead; its bomb rising on a panel a set-down FlshBom reserved (the controller avoids only bodies); TimeBom+'s blast (the recipes end before it) |
| `chips/0x001-cannon/target-over-hole` | a shot hitting side 0 over a hole (AirShoes): the panel isn't solid, so it is left as it is |

The unreachable ones: the projectile's variants with a sprite, a status, a bug or a panel effect (9: only a
navi's buster programs, NaviStats+0x4D and +0x4F, name them, and nothing sets those in a netbattle); the
countdown bomb's table rows 2 to 7 (9: TimeBom1-3 set row 0, TimeBom+ row 1) and its blast after the battle's
end (1: its tick breaks it first); BlkBomb's placed form (3: only `sub_80CD858` places one, and nothing calls
it), a zero-tick throw, a flight timer that is the 6000-tick lifetime, and a break with HP left (3); a dimming
while a bomb that doesn't run dimmed flies (3); bomb kind 1, which no chip throws (1); TankCan's shell
constants (2: one tick a panel, always cracking); FlshBom's statuses and rows (2); BugBomb's arc, which ends 10
pixels up (1); and two collision allocations (2). The hard one is a FlshBom landing on a navi of its own side:
the only such body is the thrower, three panels back, and it can't get there in the 40 ticks of flight.

All 20 recordings match the engine (14,759 frames); none of batch 2 changed it.

**Batch 3, the navi chips.** SpoutMan (32 branches, with link navi SpoutMan's charged water ball, which is his
ball): 13 taken by 8 new recordings, 17 unreachable, 2 hard:

| Scenarios | What they take the other way |
|---|---|
| `chips/0x0f2-spoutman/corner-full-trail` | the geyser from (1,1): its row above off the field, its column the far edge's, a trail of four columns |
| `chips/0x0f2-spoutman/side-1-front` | side 1's SpoutMan away from its back columns (the water ball) |
| `navis/navi-06-dripshwr/charge-dimmed`, `charge-geddon` | the link navi's ball and splashes (which stand still while dimmed) in the opponent's dimming; a splash whose panel Geddon broke doesn't crack it |
| `navis/navi-06-dripshwr/charge-into-hole`, `charge-next-hole` | the ball coming down in a hole; the panel beyond the splash a hole (one splash) |
| `navis/navi-06-dripshwr/charge-ko`, `charge-battle-over` | a splash and the ball seeing the battle over (the ball's through TimeBom1's blast while it flies) |

The unreachable ones: SpoutMan's own AI's geyser, pillar, mark and non-cracking splash (10: the navi chips spawn
the chip's, Param1 4, and the link navi throws Param1 2); a second spout from one pillar, or its slot holding
another (2: the navi chip's SpoutMan rises once a visit); his pillar slot empty when he signals it (2); the
geyser's scan passing its target column (3: the target is the enemy's column or the edge, met first). The hard
ones: his target scan reaching the field's edge (2), which needs the enemy navi's body off the panel flags while
he stands in his back columns.

All 8 recordings match the engine (6,463 frames).

ElmntMan (16 branches, with the vines' battle-end test) and SlashMan (11, with link navi SlashMan's charged
waves): 15 taken by 8 new recordings, 12 unreachable, 4 hard:

| Scenarios | What they take the other way |
|---|---|
| `chips/0x10d-elmntman/wood-from-back`, `wood-ko` | Wood from (1,2): five vines; a vine deleting the opponent, the others seeing the battle over |
| `chips/0x10d-elmntman/aqua-by-hole`, `aqua-adjacent`, `aqua-ko` | Aqua with a hole in the column ahead; the ice hitting the opponent; the ice deleting it and seeing the battle over |
| `chips/0x0e9-slashman/steered`, `landing-hole` | B held while the waves swing (each steers); the panel in front of the enemy a hole in his own column (no leap) |
| `navis/navi-03-rslash/charge-dimmed` | the link navi's waves waiting out the opponent's dimming |

The unreachable ones: ElmntMan's Fire with a second target (2: the enemy side's only body is its navi), his own
AI's meteor and ice (5), the ice's action finding hit flags its tick has zeroed (1); SlashMan's column walks
passing the field's edge (4: they meet his own column first). The hard ones: Fire with no enemy body and
SlashMan's landing scan passing the edge (3, as SpoutMan's), and a meteor seeing the battle over (1: it falls
inside ElmntMan's dimming, where only hits that run dimmed act, and its own hit ends it).

All 8 recordings match the engine (6,525 frames).

Roll, JudgeMan and BassAnly (9 branches each): 3 taken by 3 new recordings, 20 unreachable, 4 hard:
`chips/0x0dd-roll/diagonal-target` (her landing panel in her user's column but another row, which must be
free), `chips/0x10a-judgeman/book-panel-occupied` (a taken panel the opponent stands on gets no book) and
`chips/0x132-bassanly/from-corner` (from (1,3) his dark balls fly long enough for their push to run out). The
unreachable ones: parameters no chip has (Roll's three rounds, 2; BassAnly's levels below 3 and other homings,
5; the navi AI's whip, 2), an enemy body behind Roll's user or more than one (4), a heart with no healing (1),
spawns that never fail (3), a book heading from its own target (2: it ends there first), BassAnly off the field
(1). The hard ones: Roll's and the books' searches finding no enemy body (3, as SpoutMan's), and a dark ball
outliving BassAnly's dimming (1: none did, from his farthest panels). The 3 recordings match the engine (4,453
frames).

**Batch 4, ColArmy and the shots (34 branches: ColArmy, WideSht, the flying shot, the bullet, CrakShot, BblStar,
CircGun, TrnArrw, MachGun, Magnum, CornSht):** 8 taken by 7 new recordings, 24 unreachable, 2 hard:

| Scenarios | What they take the other way |
|---|---|
| `chips/0x0c6-colarmy/same-tick` | both sides' ColArmy on one tick: the second finds the cube already blinking out |
| `chips/0x059-crakshot/hole-ahead`, `cracked-cube` | CrakShot facing a hole; an undiggable panel already cracked (the helper AquaNdl's needles and the panel strikes share) |
| `chips/0x08e-circgun/shot-on-hole` | a CircGun shot on a hole, which it doesn't hit |
| `chips/0x02b-machgun1/target-steps-in` | MachGun's next sweep a column nearer, the opponent having stepped in |
| `chips/0x08d-magnum/viewer-blind` | the gunner hidden from a viewer Silence blinded |
| `chips/0x040-cornsht1/spread-onto-probe` | a corn spreading onto a panel where the other side's corn probe has just appeared |

The unreachable ones: variants and rows nothing fires (the wide wave's kinds 0-2 and 9 and WideSht's action
subtype 2, 9; the flying shot's palette, status, waits and range ends outside rows 2 and 5, 4; bullet row 0xE,
1; CircGun's shot look 1, 1), positions the field rules out (a crack shot's fifth crossing, a bubble star or a
ColArmy soldier off the field, CircGun's own column going forward or a start column without the other side's
panels, 5), a side with no player (1), Magnum standing in for something other than a navi (1), and slots that
never fail (2). The hard ones: ColArmy meeting an obstacle registered before its init (the tick of its spawn),
and TrnArrw's bow waiting for an animation that has always ended by its first look. The 7 recordings are in the
lab and match the engine at main 83158fe6 (4,918 frames).

**Batch 5, the arm chips (20 branches: WaveArm, DrilArm, and Boomer's and FireHit's effects, which BoomrArm's and
PunchArm's charged shots run; ElcPuls, whose pulse PuzzlArm's charged shot fires):** 7 taken by 5 new recordings,
12 unreachable, 1 hard:

| Scenarios | What they take the other way |
|---|---|
| `chips/0x05c-wavearm1/steer-down-from-top`, `bottom-row-no-turn` | the steering from the edge rows: no row above, so down toward the opponent and down again; no row below and nothing ahead |
| `chips/0x05c-wavearm1/into-hole`, `geddon-under` | a wave sent onto a hole; a wave whose panel the opponent's Geddon breaks |
| `chips/0x022-elcpuls1/whicapsl` | WhiCapsl folded in: the damage word's paralysis kept over the pulse's own status |

The unreachable ones: the shock wave's virus variants (4: panel-marking ones and those below 0xC) and a quick
wave's animation ending first (1), a boomerang that flies straight or hits harder (3: nothing spawns one), a drill
with no slot (2), ElcPuls's one-tick wait and its pulse's linked objects (2: nothing links one). The hard one is
FireHit's search running off the field, which needs no enemy body ahead (as SpoutMan's scans). The 5 recordings
are in the lab and match the engine at main f8764b51 (2,712 frames).

**Batch 6, the barriers, the Reflectors and IronShl (26 branches: the barrier visual of `lib/barriers`, Rflectr and
the NaviCust Reflect's guard, IronShl's shell):** 14 taken by 14 new recordings, 9 unreachable, 3 hard:

| Scenarios | What they take the other way |
|---|---|
| `chips/0x0b2-barrier/paused`, `chips/0x0b6-lifeaur/link-navi` | a barrier's visual through a pause; LifeAur on a link navi, its aura's part 6 hidden |
| `chips/0x0b3-barr100/deleted-under-it`, `chips/0x0b2-barrier/deleted-going-down`, `deleted-blown` | the navi deleted with its barrier up, going down, and blown away: the visual finds itself unlinked |
| `chips/0x0b2-barrier/fan-pull` | a barrier popped by Fan's pull, blown forward |
| `chips/0x083-rflectr1/cross-beast`, `blind-viewer`, `counter-ko` | Rflectr in a Falzar Cross Beast (the head animates); the shield hidden from a blind viewer; the counter wave's KO |
| `navicust/reflect-counter` | the Reflect program's guard firing the buster shot back |
| `chips/0x07b-ironshl1/hole-ahead`, `far-column`, `side-1`, `side-1-far-column` | a hole ahead; the shell on column 6 and on column 1; side 1's shell |

The unreachable ones: a barrier on a navi that isn't a player or is off the field (3), a barrier other than the
bubble growing back (1: a new barrier ends the old visual first), the visual's end called without one (1), the
Shield program's guard countering (1: it heeds nothing), an iron shell off the field, with more than one bump or
no collision slot (3). The hard ones: the bubble growing back while its navi is bubbled (its timer stops then),
the shield outliving its owner's vanishing for a navi chip (it fades long before the cut-in ends), and a guard
dropped by a breaking hit, which only arrives with an ordinary hit the guard blocks on the same tick. The 14
recordings are in the lab and match the engine at main 9ed59b90.

**Batch 7, the traps (34 branches: CopyDmg, AntiDmg, ElemTrap, Mine, BodyGrd):** 12 taken by 9 new recordings,
9 unreachable, 13 hard:

| Scenarios | What they take the other way |
|---|---|
| `chips/0x0be-copydmg/side-1`, `twice` | side 1's damage-carry record; a second mark taking the record from the first |
| `chips/0x0bb-antidmg/counter-after-ko` | AntiDmg's counter finding no living enemy: the catch lands four ticks before TimeBom1's blast deletes the thrower |
| `chips/0x0c5-elemtrap/replaced`, `battle-over`, `hit-as-set`, `sprung-dimmed` | the trap's record moving on (AntiDmg after it), the battle's end, a hit in the tick it appears (FireBrn's flames), springing inside HeatMan's dimming |
| `pa/0x157-bodygrd/all-ten`, `striker-replaced` | BodyGrd's striker running out its ten shurikens (an opponent of 1,500 HP); the striker evicted by Fan, a second field object |

The unreachable ones: spawns that never fail (2), AntiDmg's counter variants its starters never set and shurikens
off the field (3), the hidden mine's HP (1: no attack's target type reaches it), ElemTrap's counterattack in an
element without a sound (1: only the four that spring it, all with one), and BodyGrd's striker while dimmed or
choosing an offline target (2). The hard ones are mostly timings: a CopyDmg probe (about 5 ticks long) seeing the
battle's end, which comes with the deletion some 50 ticks after the KO hit, after which no chip can be used; a
shuriken of AntiDmg or BodyGrd starting with the battle over, or BodyGrd's striker meeting its target at 0 HP
before the deletion; CopyDmg's search meeting anything but the navi it hit (4); the mine finding no free enemy
panel (2); and ElemTrap's searches finding no enemy body (2, as SpoutMan's). The 9 recordings are in the lab and
match the engine at main ca994e37 (9,340 frames).

**Batch 8, Thunder, AirHocky, Snake, YoYo (31 branches; BgDthThd's charged ball is Thunder's; GreatYo's own yoyo
branches are left with the Program Advances):** 4 taken by 3 new recordings, 14 unreachable, 13 hard:
`chips/0x01e-thunder/target-below` (the ball going down its target's column),
`chips/0x136-bgdththd/charge-vertical` (the fast ball moving along a column) and `chips/0x032-airhocky/one-column`
(two AreaGrabs leave the opponent one column, and the puck, across, can neither go on nor turn back). The
unreachable ones: thunder-ball parameters no chip gives (4: a linked object, no status, a bug), puck rules no row
has and steps the bounces rule out (7), a snake off the field or off its target's row (2), and the yoyo's
collision slot (1). The hard ones: the thunder ball's searches failing (6: the enemy navi always on the field, and
a search ahead failing only with the enemy behind the ball) or its panels running out before it reaches a moving
opponent (1), Snake with no target or a second one (5: tried the opponent's RockCube after Geddon's holes), and a
YoYo's slot taken while it is out (1: its navi waits for it). The 3 recordings are in the lab and match the engine
(4,229 frames).

**Batch 9, Tornado, Meteors, Lance, GolmHit, SonicBom and GroundCross's falling rocks (25 branches):** 1 taken by 1
new recording, 22 unreachable, 2 hard. `chips/0x034-tornado/off-the-field`: after two AreaGrabs, from (5,2), the
tornado's panel two ahead is off the field. The unreachable ones are parameters and rows no chip sets (a tornado's
status, the action's third form, other meteor rows, a lance's second palette, GolmHit's own-panel and cracking
fist, more than one SonicBom swing, a rock counter: 13), positions the field rules out (a far column or a row
without the other side's panels, a column walk past the field, markers and rocks off the field: 5), markers ticking
outside Meteors' dimming (1: the instant meteor shower has no user) and slots that never fail (3). The hard ones: a
tornado and a falling rock seeing the battle over, which comes with a deletion some 50 ticks after a knockback KO,
while neither lasts or starts that late. The recording is in the lab and matches the engine (1,034 frames).

**Batch 10, LilBolr and AirRaid (27 branches, with the layer object both use):** 6 taken by 4 new recordings, 17
unreachable, 4 hard. `chips/0x062-lilbolr1/erupt-dimmed` (P0's HeatMan dims the screen as the boiler erupts: the
steam layer runs dimmed, and the flames' KO ends the battle with the boiler erupting), `erupt-paused` (a pause
during the eruption), `full-synchro` (LilBolr1 thrown in Full Synchro: the doubled damage word) and
`chips/0x068-airraid1/side-1-far-column` (side 1's plane on column 1, with no column ahead). The unreachable ones:
the layer's own-palette and the viruses' settings no chip's layer uses (8), the boiler's zero-tick throw, its
flight timer (the lifetime), a break with HP left and an eruption while dimmed (5: the obstacle framework holds its
actions then), the plane's actions while dimmed, its propeller's own flip and a stop with nothing (4). The hard
ones: a layer's or propeller's owner held by a status, the plane seeing the battle over (tried the opponent's
Cannon, Thunder and Silence, and KOs), and a hit that doesn't lower the boiler's HP (tried Fan and MagCoil). The 4
recordings are in the lab and match the engine (3,719 frames).

**Batch 11, MetrKnuk and SandWrm (26 branches):** 6 taken by 4 new recordings, 17 unreachable, 3 hard.
`chips/0x133-metrknuk/corner-target` (P1 on (6,3): the 3x3 around it runs past the far column and the bottom row),
`side-1` (side 1's MetrKnuk at P0 on (1,2): past column 1), `chips/0x065-sandwrm1/user-flinched` (the opponent's
AirShot hits the user as the worm emerges: the start hole's opening stalls while its navi flinches) and
`heatman-dimming` (the opponent's HeatMan dims the screen as the worm emerges: the holes run through the dimming
without opening further, while the worm and its sand wait). The unreachable ones: a slow fist (MetrKnuk's spawner
sets the fast fall), full pools (2), no panel of the enemy's area or a second enemy body (3), no target around the
enemy navi but the last one (3: panels are taken front first in each row and come back front first, and the back
column is never taken, so the navi's 3x3 always holds another panel of its side), a hole's opening outlasting the
worm's 20-tick timer, a hole seeing a pause (2: it has no run-while-paused flag), the worm and the sand seeing a
dimming (2: no run-while-dimmed flag), the start hole's timer running out in the 16-tick arc, the worm changing
row, and an arc's fifth quarter. The hard ones: MetrKnuk's first fist finding no enemy body (the enemy navi keeps
it on the panel flags until its deletion), and a hole whose navi is at 0 HP with the battle not over (2: a KO
starts the deletion, and the battle's end that destroys the holes, at once; tried MiniBomb, AirShot and HeatMan on
a 10 HP user). The 4 recordings are in the lab and match the engine (6,879 frames).

<!-- end: chip families, G4 -->

##### Chip families: onesided's share

The rest of the area is shared from here: the cannons, guns and shots, the arm chips, the traps and barriers
and ColArmy stay with this pass; the remaining navi chips, the Program Advances, the panel and stage chips, the
elemental and status chips and the recovery, support and field chips are read in a second one, whose verdicts
are in `onesided_notes.py`'s "chip families: onesided's share" section and whose scenarios are in
`coverage_scenarios/n_onesided_chips.py`.

**Second pass, batch 1: the navi chips the first pass had started** (Bass, ChrgeMan, DustMan, ProtoMan, ElecMan,
DiveMan, GrndMan, BlastMan, HeatMan, EraseMan, and the Darkness PA; 35 branches): 13 taken by 10 new recordings,
21 unreachable, 1 hard:

| Scenarios | What they take the other way |
|---|---|
| `chips/0x12d-bass/areagrab-twice`, `side1-forward` | two AreaGrabs leave Bass a column with no enemy panel and every panel left cooling down; side 1's Bass from column 4 counting columns from his own |
| `chips/0x0ef-chrgeman/holes-row-1` | ChrgeMan and his cars reaching a hole (from (2,1) on the holes stage) |
| `navis/navi-05-volcchrg/charge-cars-dimmed`, `navis/navi-02-delecswd/charge-bolts-dimmed` | link ChargeMan's cars and link ElecMan's bolts, which wait out a dimming, under the opponent's RockCube |
| `chips/0x0fe-dustman/takes-timebom-plus`, `junk-ko` | DustMan throwing TimeBom+'s big bomb (its own look, identity `timebomb-big`); his junk deleting the opponent and seeing the battle over |
| `chips/0x0e0-protoman/user-beside-landing` | ProtoMan's landing panel in his user's column, a row away |
| `chips/0x104-diveman/areagrab-twice` | DiveMan's waves rising in column 5, their far hit and splash off the field |
| `chips/0x0ec-eraseman/mark-ko` | the slash deleting the opponent while EraseMan's marks still show (A pressed just after a set is laid) |

The unreachable ones: parameters only the navi AIs' spawners pass (DiveMan's wave and BlastMan's blast, 4),
or that no spawner passes (a car tied to an action, a flame of collision type 0xA, a silent rock, which only an
effect no netbattle spawns drops; 3); Bass's and ProtoMan's searches starting or standing off the field (2), a
forbid mask Bass's searches never pass (1), a side with no column or row of its own left (AreaGrab and PanlGrab
never take a side's last full column; 3); a fifth ChargeMan car, which needs column 6 behind him (1); the junk's
hit record set by its move (1); spawns that never fail (2); Darkness's one-tick landing (2) and his parts always
spawned with someone waiting (2). The hard one: link GroundMan's drill outliving his action, which only an
interruption of his invulnerable dig would do.

All 10 recordings match the engine, every frame (16,698) and every sound call (286).

**Second pass, batch 2: DblBeast, CrossDiv's Colonel, HubBatc** (30 branches): 15 taken by 9 new recordings,
9 unreachable, 6 hard:

| Scenarios | What they take the other way |
|---|---|
| `chips/0x137-dblbeast/cubes-around-target`, `cubes-in-row` | the beasts' attack panels around the enemy all taken by RockCubes or off the field: the claw, the strike and the wing miss, each having tried all six panels |
| `chips/0x137-dblbeast/user-column`, `user-column-falzar`, `user-two-ahead` | a taken attack panel in the user's column a row away (each beast's search); Falzar's breath falling back to the user's panel |
| `chips/0x134-crossdiv/side1-miss` | side 1's Colonel finding no enemy navi in his row |
| `chips/0x135-hubbatc/attackmax`, `custom-bug`, `shield` | HubBatc on a navi whose buster is at level 5 already, with the hand-shrink bug's turn set, and with a B+Back special |

The unreachable ones: a second DblBeast target (4: the beasts list the panels with the enemy's navi body flag,
which only its navi sets), Gregar's gun shot leaving the field (2: the gun stands in its target's row facing it),
Colonel on the enemy's back column (2: he appears on his user's panel) and an attachment restarted that isn't
there (1). The hard ones: no target for the beasts (3: the enemy navi's body off the panel flags), Gregar's gun
missing (2: its panels three to five from the target, all taken but none its user's), and a new charged shot over
DustCross Beast's throw (1). The 9 recordings match the engine at main 4f2ad68e, every frame (15,910) and sound
call (409, the custom screen's now among them); batch 1's 10 too.

**Second pass, batch 3: the Program Advances CrosOver and DblHero** (29 branches): 16 taken by 7 new
recordings (and two of the forms' branches with them), 13 unreachable. A Program Advance needs its three chips
in the first hand, and a side's first hand deals fixed folder slots (per side and stage seed), which a probe
folder of thirty different chips shows; the scenarios place the chips there.

| Scenarios | What they take the other way |
|---|---|
| `pa/0x15d-crosover/target-back` | the opponent on its back column: no panel for Django, so MegaMan waits for nobody and slashes alone |
| `pa/0x15d-crosover/hole-in-front`, `django-blocked` | MegaMan's panel in front of the target a hole, Django's behind it a RockCube: each one's partner slashes alone |
| `pa/0x15d-crosover/link-navi`, `in-cross` | a link navi's CrosOver (MegaMan in MegaMan's own image, raising his own arm); CrosOver in a Cross (MegaMan in the Cross's image) |
| `pa/0x15d-crosover/side1`, `pa/0x158-dblhero/side1` | both on side 1: CrosOver's search and DblHero's shot rows going left |

The unreachable ones: an image whose identity isn't a navi's (2: every user is a navi), MegaMan finding Django
without his gun (1: Django, spawned by MegaMan's init, finishes the same 60-tick appearance first), one-tick
phases (6), Django's sun beam or partner missing (2), DblHero's shot row leaving the field (1: the other side
keeps its back column) and DblHero with no controller waiting (1). The 7 recordings match the engine, every frame
(11,219) and sound call (408).

**Second pass, batch 4: the other Program Advances (MstrCros, SunMoon, CornFsta, TwinLdrs, H-Burst; 37
branches):** 7 taken by 6 new recordings (and an earlier unreachable verdict of the collision area with them),
29 unreachable, 1 hard:

| Scenarios | What they take the other way |
|---|---|
| `pa/0x15a-mstrcros/user-column`, `pa/0x15c-twinldrs/user-column` | a Cross's and ProtoMan's landing panel in the user's column a row away |
| `pa/0x15c-twinldrs/colonel-fallback-column` | Colonel with no enemy navi in his row, standing on his fallback column (two AreaGrabs put the user on column 5) |
| `pa/0x15b-sunmoon/hole-ahead` | SunMoon's meteors over the holes stage's hole (`object_breakPanel` on a panel that isn't solid, which `completeness.md`'s collision table had as unreachable) and its dive ending in smoke |
| `pa/0x14c-cornfsta/corner` | CornFsta's sower at the corner beside the enemy, its three free panels all among its last three bursts (AreaGrab a turn before, as the PA's codes take no other chip, and the user standing in the taken column so it isn't returned; seed 4 for the draws) |
| `pa/0x152-h-burst/miss` | H-Burst's shot flying off the field |

The unreachable ones: missing links (a controller, a leader, 5), one-tick phases (4), spawns (2), a second enemy
navi panel and a second target for ProtoMan (2), MstrCros's fade below 0 and a finale Cross past the third (2),
panel bursts around a panel or with a sound (2: every spawner passes a whole-field region, no row has a sound),
SunMoon's dead blink test (2) and other meteors (2: it throws one kind), CornFsta's user not a navi, a sower that
steps twice, finds no panel or outlives its farmer, or fails to spawn (5), ProtoMan's search off the field (1),
ProtoMan back before Colonel's charge has run 40 ticks (1: his quickest way back, with no panel to land on,
comes at its 42nd), and H-Burst seeing the fight stopped (1: battle flag 1 stays set for the battle). The hard one:
no enemy navi on the field for MstrCros. The 6 recordings match the engine at main 9aedd5a4, every frame (15,413)
and sound call (418).

**Second pass, batch 5: GreatYo's yoyos, AreaGrab and PanlGrab, the panel changer, GrabBnsh's hands (36
branches):** 8 taken by 6 new recordings, 27 unreachable, 1 hard:

| Scenarios | What they take the other way |
|---|---|
| `pa/0x154-greatyo/adjacent` | GreatYo's middle yoyo hitting as it starts out: it spins at once and the others roll back no panels |
| `chips/0x0a3-areagrab/third-grab`, `home-partly-stolen` | a third AreaGrab on the other side's last column, which can't be taken; AreaGrab from a column the other side's PanlGrab has partly taken, its home the column behind |
| `chips/0x0a2-panlgrab/twice-then-areagrab` | the grab's "keeps a full column" test meeting a column PanlGrab has taken a panel of |
| `chips/0x0a8-holypanl/hole-ahead`, `chips/0x0bd-antirecv/healer-over-hole` | the panel changer's panel a hole: HolyPanl's panel in front, AntiRecv's poison under a healer over a hole (AirShoes) |

The unreachable ones: GreatYo's signal to an empty slot and a yoyo without its controller (2: the three go out
together and the controller outlasts them), a grab with no column to take (2: a side keeps a full column), the
grab searches' walks from the far edge (5: that panel is always the other side's), spawns (3), the panel
changer's kinds 7, 8 and 0xA to 0xC (9: its spawners are the panel chips and AntiRecv), a change that doesn't
flicker and one with no holder (2), GrabBnsh's later hands (2: one enemy navi panel, one hand a strike) and its
region (1), and CornFsta's sower with no panel at all (1). The hard one: AreaGrab's home search running off the
field, which needs every column from the user's back partly stolen. The 6 recordings match the engine at main
1703b936, every frame (8,062) and sound call (177).

**Second pass, batch 6: Sensor, Guardian, Anubis (28 branches):** 6 taken by 5 new recordings, 17
unreachable, 5 hard:

| Scenarios | What they take the other way |
|---|---|
| `chips/0x071-sensor1/side1`, `dimmed`, `ko-scanning` | side 1's Sensor (its scanner's test and its line leaving at column 0); the scanner under the other side's dimming; the scanner, sent up the diagonal past the opponent, seeing the battle over |
| `chips/0x097-guardian/broken-in-dimming`, `broken-and-ko` | the statue broken inside HeatMan's dimming: the strike back waits the dimming out, and finds the battle over when HeatMan also deletes its owner |

The unreachable ones: the turret's own dimmed tests (2: the obstacles' shared update runs its actions only
outside a dimming), spawns (2), links (2), a scanner that passes objects (2) and the laser's delay (1); the
statue appearing on a taken panel, broken with HP left, quiet, or striking back with no side, and a repeated test
(5); Anubis's drain seeing the battle over (1: the statue's shared update ends it first, at a KO or at the 15th
turn's time-up alike) and its bubbles finding no panel (2). The hard ones: a laser hit on its turret's last
tick, the turret pushed while it fires (its target is paralyzed for the whole fire; with a Barrier, AirShot still
didn't push it), and the statue broken by both sides' hits at once or by a hit with neither side's bits (3). The 5
recordings match the engine at main eda65622, every frame (6,173) and sound call (189).

<!-- end: chip families, onesided -->

### 6.3 Custom screen keys

Four custom screen routines were verified only by the golden traces' dumps, with no lab recording: SELECT hiding
the window (`sub_8026D06`), B taking a pick back (`sub_8029032`), the Cross window closing (`sub_802790C`) and
DustCross's scrap (`sub_8027406`). The lab now has `custom/hide-window`, `custom/take-back`,
`custom/cross-window-close` and `custom/dust-scrap` (the lab's driver learned a `scrap` pick); all match.

## 7. Open points

Nothing here is a known missing behaviour; each is a place where the audit can't close the question. Two
earlier points are closed: the statements in other documents that the audit found out of date are corrected, and
NaviStats+0x54 is unreachable (§5).

1. **The documented class** (441 routines) rests on docs/engine being the port's specification. 288 of them run
   in matching recordings. Of the 153 that don't, most are the IWRAM kernel's helpers (uninstrumented), the
   battle's init (before the lab's coverage begins) and states the docs mark "not PvP". A citation in a comment
   also counts as ported; `sub_8108F74`, the virus's update, is cited only to say viruses do it.
2. **Link loss.** Top states 0x0C (communication error) and 0x10 (terminate) are documented; the port has the
   result codes and leaves the detection to its netplay.

## 8. Limits

- **Names, not behaviour.** A cited routine is taken as ported. §6 samples that and §9 checks the definitions
  that have no behaviour at all; neither proves a ported routine right. That is the lab's job, branch by branch.
- **The call graph is static.** A call through a pointer kept in memory is seen only if its table is a literal
  of some routine. The kind tables, action table and hook tables are handled explicitly; a callback stored by
  one routine and called by another through RAM would be missed. The lab's coverage is the check on that: every
  routine a recording runs is in the inventory whatever the graph says, and none of them is unaccounted.
- **Table extents** come from the disassembly's labels. A table with a label in its middle is read short.
- **IWRAM isn't instrumented**, so the collision kernel's routines are never "run" here; they are classed by
  citation and by their callers.
- **Coverage starts with the fight.** The set's init (`sub_80071D4`) runs before it, so its helpers count as
  never run.
- **Recordings prove the paths they take, for as long as they last.** "Run in matching recordings" says nothing
  about a branch no recording takes (§6.2's list), and a recording that ends early proves nothing past its end
  (§10). A scenario's own expectations are the check on that, and they are only as good as the template that
  wrote them: 380 recordings were `unmet` until they were sorted (§11). None is now, but an expectation says
  only "damaged" or "used", so a recording can still meet it on another path than its name suggests.

## 9. Defined without behaviour: the stub check

The audit's first run reported nothing missing while six weapons a netbattle can fire had no behaviour:
BugRSwrd's and BgDthThd's charged shots and the four arm chips' (the original's weapon routines 0x21 to 0x26,
`sub_8011E40` to `sub_8011F10`). Each was a weapon definition with its charge times and no `setup`; the chip made
it the navi's charged shot, and a full B charge stopped the engine. The audit missed them twice over: it didn't
follow the weapon table (123 entries, past its limit for a routine's own table), so their routines came out
"not reachable"; and had it followed the table, a citation in the weapon's file would have classed them
"ported". Both are fixed in the tool (§1, steps 5 and 7), and the six are ported since
(`chips/bugrswrd/charge` and the others, with 25 recordings).

The stub check lists everything of that shape: a definition, a role or a dispatch entry that exists by name with
nothing behind it. It reads the loaded content (through the engine's own loader) and the sources:

| Checked | How | Result |
|---|---|---|
| Weapons without a `setup` | every `WeaponDef` of the loaded content | none without, apart from the seven below |
| Weapons whose `charged_chip` says the ruleset handles them | the same; and where the forms name them | seven (`megaman/charged-chip-bonus` and its three siblings, `eleccross/a-charge`, `megaman/rock-barrage`, `protoman/a-charge`): not stubs. The original's entries are `nullsub_44`, and `sub_800FB54` tells these A-charges apart by number before it would call one (5, 0x0D, 0x1F, 0x20, 0x29, 0x2D: the chip with its bonus; 0x18: GroundCross's rocks first), as chip_use.rs does. The forms name them only as A-charges; ProtoMan's is his NaviStats+0x39 |
| Chips without a use | a chip definition needs exactly one of `action`, `dimming`, `navi` and `instant` (the define phase's error) | none can exist. Gregar's, Falzar's, HackJack's and Django's uses are `lib/unusable`: `off_802CCB4[34]` and `[35]` and `off_802CD5C[18]` and `[19]` are NULL, and the game jumps to address 0 (the lab's recordings of Gregar, Falzar and HackJack end on that frame; the lab has no Django scenario) |
| Actions and kinds whose update only errors | every function of the content whose body is an `error` | none |
| Roles unfilled, or naming an action nothing implements | the loaded roles | `actions.volley` unfilled: no routine was found that raises the volley's request (0x40000000), and the port raises it nowhere (§5). (`actions.turn` is filled since step 13, with the turn, which no link stage enables.) The battle mode 9 kinds `kinds.mode9_actor` and `kinds.mode9_attack` unfilled: no content defines them, and a netbattle's mode isn't 9 |
| Errors that say something is not implemented, ported, modelled or supported | every `panic!`, `unreachable!` and Luau `error(` with that wording (12) | none reachable: the six bug-code writes in setup.rs (a weapon routine, a shot program, a form or a navi by number, the gauge speed, an unmodelled stat: no hit carries such a code, unverified.md); a weapon without a `setup` (none now); the mode states past the fade-out (battle effects 2); `sub_80EBB78`'s AI navi version byte |
| | | one unproven: CrosOver reads its partner's Param4 through a link that outlives him, and the port models the one actor known to take his slot before the read (the user's sword). No other was found that can spawn inside the chip's dimming |

The check fails when it finds a stub its notes (`stub_notes.py`) don't cover, so a new one has to be read. The
other errors in the sources (about 400 in the content, 180 in the engine) say what the original does at that
point: a read past a table, a null pointer, a division by zero. They are the port's record of the original's
undefined behaviour, not gaps.

## 10. How deep the recordings went: the cut-in chips

Until this audit's follow-up, 670 of the lab's recordings ended inside a cut-in chip's telop. The scenario
library's templates end with `settle` ("wait until both navis are idle and the attacks are gone"), and both navis
stand idle from the moment a cut-in chip is used until its dimming ends, so `settle` returned 20 frames after the
use and the recording stopped a second later: about 80 frames after the chip was used, as its navi or controller
was about to appear.

| | Recordings | What they were |
|---|---|---|
| Ended inside the telop | 661 | of 62 chips (every navi chip and several other cut-in chips), 7 Program Advance recordings and 5 others: `stage-*` 174, `cross-*` 84, `hit`, `miss` and `adjacent` 62 each, `beast` 47, `obstacle` 42, `beast-charged` 29, `counter-cut-in` 23, `atk10` and `navi20` 19 each, and a few hand-written ones |
| Ended early in a second cut-in | 9 | `counter-cut-in` of BurnSqr, Meteors, Magnum, CircGun and DblBeast, ElmntMan's `counter` and `guard`, BigHook's `beast-charged`, `navicust/tango-quarter-hp` |

362 of the 661 were marked `ok` in the lab's index (their expectation was only that the chip was used) and 299
`unmet`. The engine matched all of them, which said that the telop starts on the right frame and nothing about
the chip. What those chips do was verified only by the scenarios that wait on their own: the survey's
(unverified.md) and the templates with a second side acting (`counter`, `guard`, `barrier`, `invisible`). So
"the navi chips on every stage", "against an obstacle", "in Beast Out", "in their Cross", "with Atk+10" and
"missing" were not verified at all, whatever the lab's match count said.

`settle` now waits out a cut-in (and through a chip that waits for a key, up to three times its limit), and the
670 are recorded again to their ends: 687,917 frames for the 661 where there were 456,716. The engine matches
every one. 80 of the 661 were still `unmet`: the chip ran to its end and didn't damage the opponent from where
the scenario stood (ElecMan's and TomahawkMan's in several templates, HackJack's because the original stops);
§11 has what became of them. The other 1,576 recordings that have a `settle` and a cut-in were recorded again too and came out frame for
frame as before.

With these and the scenarios added since, the lab runs 4,033 of the inventory's routines where it ran 3,991, and
the ported routines no recording runs fell from 260 to 225.

## 11. Scenarios whose goal wasn't met

The lab's index marks a recording `unmet` when the scenario's expectations (`damaged:1`, `used:0`, ...) fail at
its end. Until this section's work 380 were: the scenario ran, the engine matched it, and it didn't do what its
name said, nearly always because the generated template stood the opponent where the chip can't reach (a sword
three columns away, a bomb in the adjacent column, ElecMan's strike a column off) or needed something first (a
trap a hit, Snake a hole). A match on those vouched for less than it read. The verification workspace's
`tools/chiplab/reach.py` now records where each chip's attack lands, from the recordings, and the generator
places the templates by it. The 380 were sorted so:

| | Recordings | What |
|---|---|---|
| The template was wrong: placed where the chip lands | 142 | the templates that mean "the chip hits" (Beast Out, the Cross, Atk+10, Navi+20, the counter cut-in) stand the navis where the chip lands: the adjacent column for the swords and the short reaches, two columns for Tornado, Static and MoonBld charged in SlashCross (its moon blade's ring is centred a panel ahead of its user), the gregar base's grass stage where its unseeded stage's ice blocks stop the attack (Colonel); EraseMn EX's A press comes at the aim that points down the row |
| The goal was wrong: no damage where the chip can't reach | 65 | `hit` from three columns and `adjacent` from the next: each says the chip is out of reach there and expects the opponent undamaged |
| The goal was wrong: the use alone | 140 | chips whose hit needs something first: a trap a hit on its user, TimeBom its countdown, Mine a step onto it, Guardian a hit on the statue, Snake and SumnBlk a hole, CircGun and Magnum an A press, the boomerangs an edge row, Lance the far column, Muramasa lost HP, MchnSwrd a paralyzed target, ElemSwrd and AssnSwrd a panel, Rflectr a shot to reflect, ColArmy an obstacle; each description names the scenario where the hit is recorded. And Sensor's counter cut-in, whose two sensors face each other, each laser ending on the other |
| Can't be met | 30 | HackJack's three chips and the Gregar and Falzar chips stop the original (their handlers are NULL, §9); DarkPlus alone only sparkles; ElmntMan's undirected element at the adjacent column is a draw |
| Stale | 3 | `ko` recordings of ElecMan, TomahawkMan and HackJack whose scenarios had been dropped: the first two are from the adjacent column now, HackJack's is gone |

443 recordings were made again or added for this and the scenarios of §6.1's second batch, and the engine matches
them. One didn't match at first, and it was the port's: `chips/0x0b9-uninstll/folded-beast-over`, an Uninstll at
a side-1 navi in Falzar Beast Over that has no shoes of its own. Beast Over's form flags (`sub_8014674`) are one
literal, 0x08000030, which the disassembly renders as a pointer; the port had taken 0x08000000 alone and so left
out AirShoe and FloatShoe. Every Falzar Beast Over recording before had a navi with AirShoes and FlotShoe
programs, which set the two bits anyway. The remaining bit is the collision kernel's "untouchable"
(`sub_3007218` drops every pair in which either side has it, and poison panels skip it), the form effect
`untouchable` since: nothing can hit a navi in Falzar Beast Over.

The lab's custom-screen check had a blind spot of the same kind. Every Beast Over recording counted one of its
screens as differing (9 of 10, 11 of 12): the engine's screen sent Beast Out where the original's sent Beast
Over. The engine was right, as the frames show (in the lab's full replay the engine's own screens run, with the
battle's emotions); the check runs each screen alone and read the navi's emotion from its mood only, so it never
saw a tired navi, for which the Beast Out button means Beast Over. It now takes tired (the Beast Out turns spent
and out of the Beast) and worn out (past a Beast Over) from the trace's stats too, and all 11,666 of the lab's
screens and the golden traces' 40 match. A check that always reports one mismatch hides the next one.

