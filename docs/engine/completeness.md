# Completeness: the port against the original's battle code

The chip lab verifies what its recordings reach. This is the other question: **is there battle code in the
original that the port doesn't have at all?** The answer at the audited commit: no routine of the simulation that
a netbattle can reach is left without a counterpart. The one group the audit found (the link navis' charged
attacks, 65 routines, §4) was ported while it ran. What is left are caveats (§7) and code that is ported but that
no recording runs (§6).

The audit is a tool in the verification workspace (`tools/audit/audit.py`, with its exclusion rules in `excl.py`
and the hand-read classes in `manual.py`); it runs against an engine checkout and should be run again after large
merges. Figures here are for the engine with the link navis' charged attacks and lock-on modes by handle merged,
and a lab of 4,997 recordings, all of which the engine reproduces in full.

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
   hook tables indexed by actor type (`sub_800F35C` and its six siblings) through the player's table only. A
   routine that the rules of §5 say a netbattle never enters is not followed. The audit stops with an error if a
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
   | Missing | run only in recordings the engine stops in, or left over |

## 2. Counts

8,116 routines are in the inventory; 3,991 of them run in the lab.

| Class | Routines |
|---|---|
| Ported: cited in the engine's source or content (3,152 run, 260 don't: §6.1) | 3,412 |
| Out of scope: not reachable from a netbattle's code | 3,181 |
| Documented: cited in docs/engine only (288 run, 153 don't) | 441 |
| Out of scope: excluded by a rule (§5) | 349 |
| Folded into a ported caller (28 of them never run) | 197 |
| Infrastructure a netbattle runs | 126 |
| Presentation: HUD tasks | 125 |
| Trivial: empty or an accessor | 117 |
| Run in matching recordings, no counterpart by name | 74 |
| Link transport: replaced by the port's netplay | 53 |
| Folded into a documented caller (6 of them never run) | 20 |
| Presentation: custom screen drawing, the IWRAM draw pass, camera | 21 |
| **Missing** | **0** |

By area:

| Area | Ported | Documented | Folded | Run, matching | Presentation | Link transport | Trivial | Infrastructure | Out of scope |
|---|---|---|---|---|---|---|---|---|---|
| IWRAM routines | 14 | 13 | 3 | 0 | 7 | 0 | 0 | 0 | 48 |
| Object system | 8 | 15 | 3 | 0 | 0 | 0 | 0 | 0 | 5 |
| Battle flow | 64 | 89 | 11 | 2 | 0 | 0 | 16 | 0 | 136 |
| Battle objects and panels | 100 | 27 | 3 | 2 | 0 | 0 | 6 | 0 | 35 |
| Actors, collision, status, HUD | 512 | 98 | 41 | 12 | 123 | 0 | 36 | 2 | 160 |
| Link status, battle settings | 2 | 10 | 1 | 0 | 0 | 2 | 1 | 0 | 15 |
| Custom screen, gauge, camera | 168 | 29 | 66 | 57 | 16 | 0 | 30 | 3 | 371 |
| Link layer | 1 | 11 | 0 | 0 | 0 | 51 | 16 | 0 | 110 |
| Object kinds, chips, navis | 2,482 | 105 | 89 | 1 | 0 | 0 | 12 | 0 | 2,650 |
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

74 routines run in netbattles, are cited nowhere, and fall under no other class: the custom screen's drawing
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

All are in the content now (`navis/*/charge.luau` and ProtoMan's specials) and the 11 recordings match. The
audit's rerun finds nothing in the class.

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

The 3,181 unreachable routines are mostly the other 138 object kinds' (viruses, bosses, story objects) and what
only they call.

## 6. The other direction: is a ported routine's port complete?

### 6.1 Ported routines no recording runs

260 routines the source cites are never run by the lab. A citation is not a test: these are the port's
unverified parts, or content for something a netbattle can't do. By the file that cites them:

| Where | Routines | What |
|---|---|---|
| `content/bn6/lib/instant` | 26 | instant effects no recorded chip has: the dust storm and its motes, the immobilizer, the invisibility, repair and side-special weapons |
| `content/bn6/chips/elmntman` | 25 | ElmntMan's meteor, ice and part of his body: run by the soundmod golden trace, not by the lab |
| `crates/bn6-battle/src/kinds/obstacle.rs` | 14 | obstacle routines (`sub_801802C` to `sub_8018186`, `sub_8016B02`, `sub_8016B36` and others) |
| `content/bn6/chips/airspin` | 13 | the seeking whirlwind (T3 0xD4): AirSpin's variant 1, which no chip record sends |
| `crates/bn6-battle/src/battle.rs` | 13 | the communication error (`sub_8007EB8`) and escape (`sub_800AAD6`) results, the set's second init entry, the per-player gauge mode's routines |
| `crates/bn6-battle/src/kinds/player/actions/cross_change.rs` | 11 | the Cross change: no live writer (unverified.md) |
| `crates/bn6-battle/src/kinds/player/form.rs`, `actor.rs` | 20 | form hooks, the appearing navi, the bubble's bob |
| `content/bn6/objects/encased-bubble`, `follow-effect`, `chips/lifesync`, `chips/bugfix`, `lib/dimming`, `lib/rapid_buster.luau`, `lib/bombs`, `chips/lilbolr` | 41 | phases or variants of objects the lab runs otherwise |
| `crates/bn6-battle/src/collision.rs`, `field.rs`, `crates/bn6-frontend/src/objects.rs` | 16 | IWRAM routines: the lab's coverage doesn't instrument IWRAM, so these are run but not counted |
| `content/bn6/navis/chargeman`, `protoman`, `tomahawkman` | 4 | ChargeMan stopped where the floor ends (`sub_80F1284`, `sub_80F1500`), ProtoMan's B+Back (`sub_80120A6`), `sub_8012278` |
| the rest, one to four each (40 files) | 77 | single phases and helpers |

The audit's `unrun.txt` lists them. Worth scenarios first: the link navis' three, the obstacle hooks, and the
single phases of chips the lab already records.

### 6.2 Branches that ran one way only

Of the ported routines' 7,472 branches that ran, 2,183 (in 1,254 routines) ran one way only (`onesided.tsv`, with
the instructions before each). A sample of them against the port:

| Routine | The side never taken | The port |
|---|---|---|
| `sub_801A200` (Full Synchro from a counter) | the attacker in a Cross; the victim tired after Beast Out (AIData+0x32, +0x36) | has both (`counter_and_mood`, `set_mood`). New recordings take the Cross side and the Beast side: `flow/counter-in-cross` (the banner and the paralysis, no Full Synchro), `flow/counter-in-beast` (Full Synchro); the transformation 0x0B compare (Gregar Beast) and the tired victim are still untaken |
| `sub_8029224` (modifiers) | Uninstll after a damaging chip that dims | has it. New recording `custom/modifier-uninstll-dimming`: Roll then Uninstll stay two chips |
| `sub_8013E58` (the status bug) | six of its eight outcomes (one RNG draw a recording) | has all eight |
| `sub_801A45C` (counter bookkeeping) | the gauge bonus under battle flag 0x40; the battle over | has both; the first is not a netbattle's |
| `sub_8013FD0` (HP lost at the custom screen's opening) | NaviStats+0x54 nonzero, in 1,292 openings | has it (`custom_hp_bug`, and bug code 0x54 that raises the stat); nothing recorded inflicts the code (§7) |
| `sub_8015C12` (mood wear) | a mood of 0, in 4.2 million calls | has the test |
| `sub_8029520` (Program Advances) | the veto (+0x1C nonzero) | documented as unable to fire |
| `sub_8009338` (the custom screen's mode state) | the UI's result 2, the escape | not ported; a netbattle has no running |
| `sub_801002C` | NaviStats+0x10 nonzero | a palette index: presentation |
| `sub_80D6BD4` (ElmntMan's meteor) | its third state, a plain destroy (the lab never runs the meteor; the soundmod trace does) | the definition's destroy lifecycle |

No sampled branch that a netbattle can take is missing from the port. The sample is small (10 of 1,254
routines); the one-sided list is the place to look for the next coverage scenarios.

### 6.3 Custom screen keys

Four custom screen routines were verified only by the golden traces' dumps, with no lab recording: SELECT hiding
the window (`sub_8026D06`), B taking a pick back (`sub_8029032`), the Cross window closing (`sub_802790C`) and
DustCross's scrap (`sub_8027406`). The lab now has `custom/hide-window`, `custom/take-back`,
`custom/cross-window-close` and `custom/dust-scrap` (the lab's driver learned a `scrap` pick); all match.

## 7. Open points

Nothing here is a known missing behaviour; each is a place where the audit can't close the question.

1. **Battle effects 0x200000** (the random battle). The 96 settings records from 0x60 on have it, for another
   match type than the lab's bases. The port carries the flag (`setup::effects::RANDOM`) and the custom screen
   reads it; the original also takes the navi's stats and folder from a second place for it (`sub_800B144`,
   `sub_800A3E4`), which is the setup's to supply. No recording has it. To verify: a lab base of that match type.
2. **NaviStats+0x54** (HP lost when the custom screen opens). The port has the stat, the loss and bug code 0x54;
   the audit found no hit that inflicts that code and no NaviCust routine that writes the stat. To settle: search
   the attack objects' bug codes for 0x54, then either a scenario or an "unreachable" row in unverified.md.
3. **The documented class** (441 routines) rests on docs/engine being the port's specification. 288 of them run
   in matching recordings. Of the 153 that don't, most are the IWRAM kernel's helpers (uninstrumented), the
   battle's init (before the lab's coverage begins) and states the docs mark "not PvP". A citation in a comment
   also counts as ported; `sub_8108F74`, the virus's update, is cited only to say viruses do it.
4. **Link loss.** Top states 0x0C (communication error) and 0x10 (terminate) are documented; the port has the
   result codes and leaves the detection to its netplay.
5. **Statements in other documents that this audit found out of date** (the code is ported and matches):
   chips.md's "Not ported yet" list in §3.6 (the barriers, the panel chips, the instruments, AirRaid, BugFix,
   ColorPt, Sensor, SumnBlk) and its "Not ported: LilBoiler … VDoll"; field-names.md on the emotion-swing bug
   (`sub_8013DA0`) and the Beast Out wrapper (`sub_80EAD9C`); object-kinds-pvp.md's row for T3 0x12.

## 8. Limits

- **Names, not behaviour.** A cited routine is taken as ported. §6 samples that; it doesn't prove it.
- **The call graph is static.** A call through a pointer kept in memory is seen only if its table is a literal
  of some routine. The kind tables, action table and hook tables are handled explicitly; a callback stored by
  one routine and called by another through RAM would be missed. The lab's coverage is the check on that: every
  routine a recording runs is in the inventory whatever the graph says, and none of them is unaccounted.
- **Table extents** come from the disassembly's labels. A table with a label in its middle is read short.
- **IWRAM isn't instrumented**, so the collision kernel's routines are never "run" here; they are classed by
  citation and by their callers.
- **Coverage starts with the fight.** The set's init (`sub_80071D4`) runs before it, so its helpers count as
  never run.
- **Recordings prove the paths they take.** "Run in matching recordings" says nothing about a branch no
  recording takes; that is §6.2's list.
