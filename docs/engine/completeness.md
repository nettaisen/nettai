# Completeness: the port against the original's battle code

The chip lab verifies what its recordings reach. This is the other question: **is there battle code in the
original that the port doesn't have at all?** The answer at the audited commit: no routine of the simulation that
a netbattle can reach is left without a counterpart. The audit found two groups, both ported since: the link
navis' charged attacks (65 routines, §4) and six chip weapons that were defined without behaviour (§9), which
its first run missed. What is left are caveats (§7) and code that is ported but that no recording runs (§6).

Two things this document said before were wrong or weaker than they read, and are corrected here:

- **"Missing: 0" was false** while the six chip weapons had no `setup` (§9). The audit now follows the weapon
  table and has a stub check; both report none.
- **"All of which the engine reproduces in full" covered less than it said.** 670 of the lab's recordings ended
  inside a cut-in chip's telop, about 80 frames after the chip was used and before it did anything (§10). A match
  on those vouched for the telop's start and nothing of the chip. They are recorded to the end now, and match.

The audit is a tool in the verification workspace (`tools/audit/audit.py`, with its exclusion rules in `excl.py`,
the hand-read classes in `manual.py` and the stub check in `stubs.py`); it runs against an engine checkout and
should be run again after large merges. Figures here are for the engine with chips, weapons, navis and statuses
by handle (content model v2's steps 10 to 12) and the six chip weapons merged, and a lab of 5,078 scenarios:
5,058 recorded (the other 20 can't be: 18 skipped, 2 where the original itself stops advancing). The engine
reproduced every recording on every frame at the lab's last full run; the 684 recorded again or added since
(§10, §6.1) were replayed on their own and match.

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
   NaviStats bytes: `compat/weapons.toml`, which the verification workspace's generator checks against the ROM).
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

8,116 routines are in the inventory; 4,033 of them run in the lab.

| Class | Routines |
|---|---|
| Ported: cited in the engine's source or content (3,193 run, 225 don't: §6.1) | 3,418 |
| Out of scope: not reachable from a netbattle's code | 3,175 |
| Documented: cited in docs/engine only (288 run, 153 don't) | 441 |
| Out of scope: excluded by a rule (§5) | 349 |
| Folded into a ported caller (27 of them never run) | 197 |
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
| Actors, collision, status, HUD | 518 | 98 | 41 | 12 | 123 | 0 | 36 | 2 | 154 |
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

- **Two ruleset roles name actions the port doesn't have**, and neither can start in a netbattle. They are
  left as they are for content model v2's step 13 to keep as documented stubs or remove:
  - `actions.turn`, the turn L or R starts (the original's action 0x3B, `sub_80EDF0C`, documented in
    chip-table.md and not ported). The ruleset decodes L and R as a turn only for a navi that can turn
    (`sub_80141F4` sets the bit at the navi's init), and that routine returns without setting it when the
    stage's panel pattern is one of the standard columns (0x38, 0x30, 0x3C) or the battle is DustMan's
    mini-game. All 192 link stages have pattern 0x38, so L and R only ask for the custom screen.
  - `actions.volley`, a volley of buster shots (the original's action 0x30 entered from the status routine,
    `loc_801B006`, on request bit 0x40000000 of AIData+0x44). That bit is tested there and nowhere raised: of
    the 26 calls of the request setter (`SetAIData_Unk_44_Flag`) none passes it, as a literal or as a shift, and
    the literal's six other uses are sprite attributes and object flags. The port's content API can name the
    request (`"volley"`), and nothing does.

The 3,175 unreachable routines are mostly the other 138 object kinds' (viruses, bosses, story objects) and what
only they call.

## 6. The other direction: is a ported routine's port complete?

### 6.1 Ported routines no recording runs

225 routines the source cites are never run by the lab (260 before the cut-in chips' recordings ran to their
end and the first scenarios written from this list). A citation is not a test: these are the port's unverified
parts, or content for something a netbattle can't do. By the file that cites them, with whether a netbattle can
run them where that has been read:

| Where | Routines | What | A netbattle |
|---|---|---|---|
| `content/bn6/lib/instant` | 26 | instant effects 2, 6, 9, 11, 16 and 17: the invisibility, repair, the immobilizer, the side special, the meteor shower, the dust storm and its motes | can't: no chip has them, and the weapon routines that name two of them (0x71, 0x83) are named by no form's or navi's row and no NaviStats byte |
| `crates/bn6-battle/src/kinds/obstacle.rs`, `content/bn6/objects/encased-bubble` | 21 | an obstacle thrown (`sub_8018002`'s steps) or encased in ice or a bubble (`sub_801813A`'s, and the bubble it becomes); the actors' hit reactions on an obstacle | can't: the two requests are made only by routines nothing calls (field-objects.md §4.5), and nothing starts those reactions on an obstacle |
| `content/bn6/chips/airspin` | 13 | the seeking whirlwind (T3 0xD4): AirSpin's variant 1 | can't: no chip record sends it |
| `crates/bn6-battle/src/battle.rs` | 13 | the communication error (`sub_8007EB8`) and escape (`sub_800AAD6`) results, the set's second init entry, the per-player gauge mode's routines | not the simulation's (the link is the port's netplay), or battle flag 0x40 |
| `kinds/player/actions/cross_change.rs`, `entry.rs`, `hand.rs` | 17 | the Cross change and knockout while paused, the navi that appears mid-battle, the link navis' chips leaving the hand | can't: no live writer of the Cross change (unverified.md) |
| `content/bn6/lib/rapid_buster.luau`, `lib/dimming/blinding_flash.luau`, `chips/bugfix/glow.luau`, `navis/megaman/weapons/shield`, `navis/tomahawkman`, `chips/tornado`, `chips/mstrcros`, `chips/rskyhny` | 21 | weapon routines 0x39, 0x3C, 0x8C and 0x46; dimming effect 2; the glow's two other variants; the Tornado action's subtype 3; a sword phase and a bee action nothing sets | can't: nothing names the routine, effect, variant or phase (the NaviCust writes weapon routines 0x3B, 0x8B and 0x3D only; the link navis' level tables 0x30 and 0x34) |
| `crates/bn6-battle/src/collision.rs`, `field.rs`, `crates/bn6-frontend/src/objects.rs` | 16 | IWRAM routines | run, not counted: the lab's coverage doesn't instrument IWRAM |
| `kinds/player/form.rs`, `actor.rs` | 20 | the forms' NaviCust refresh hooks (`sub_801469C`'s table), fields the viruses use | not read yet |
| the rest, one to four each (about 45 files) | 78 | single phases and helpers: LifeSync's marker, the follow effect's other looks, LilBoiler's layer, a bomb's lingering hit, the lock-on marker's choice between two targets, and others | not read yet: the next scenarios |

The audit's `unrun.txt` lists them. Scenarios written from this list so far (all match): ChargeMan stopped
where the floor ends, with his cars out and with none (`navis/navi-05-volcchrg/charge-hole-*`); ProtoMan as the game
has him below level 10, whose B+Back is the shield that only guards (`navi-11-stepswrd/level-5`: weapon routine
0x34); ElmntMan's four elements by the user's A press (`chips/0x10d-elmntman/press-*`: the lab had only the one
his timeout draws); EraseMan's aim taken at each step (`chips/0x0ec-eraseman/press-*`); DeltaRay's three strikes,
his return and burst (`chips/0x12f-deltaray/three-strikes`, on the falzar base: the gregar base's stage has rocks
on two of the delta's corners).

### 6.2 Branches that ran one way only

Of the ported routines' 7,546 branches that ran, 2,171 (in 1,257 routines) ran one way only (`onesided.tsv`, with
the instructions before each). A sample of them against the port:

| Routine | The side never taken | The port |
|---|---|---|
| `sub_801A200` (Full Synchro from a counter) | the attacker in a Cross; the victim tired after Beast Out (AIData+0x32, +0x36) | has both (`counter_and_mood`, `set_mood`). New recordings take the Cross side and the Beast side: `flow/counter-in-cross` (the banner and the paralysis, no Full Synchro), `flow/counter-in-beast` (Full Synchro); the transformation 0x0B compare (Gregar Beast) and the tired victim are still untaken |
| `sub_8029224` (modifiers) | Uninstll after a damaging chip that dims | has it. New recording `custom/modifier-uninstll-dimming`: Roll then Uninstll stay two chips |
| `sub_8013E58` (the status bug) | six of its eight outcomes (one RNG draw a recording) | has all eight |
| `sub_801A45C` (counter bookkeeping) | the gauge bonus under battle flag 0x40; the battle over | has both; the first is not a netbattle's |
| `sub_8013FD0` (HP lost at the custom screen's opening) | NaviStats+0x54 nonzero, in 1,292 openings | has it (`custom_hp_bug`, and bug code 0x54 that raises the stat); a netbattle can't make it nonzero (§5) |
| `sub_8015C12` (mood wear) | a mood of 0, in 4.2 million calls | has the test |
| `sub_8029520` (Program Advances) | the veto (+0x1C nonzero) | documented as unable to fire |
| `sub_8009338` (the custom screen's mode state) | the UI's result 2, the escape | not ported; a netbattle has no running |
| `sub_801002C` | NaviStats+0x10 nonzero | a palette index: presentation |
| `sub_80D6BD4` (ElmntMan's meteor) | its third state, a plain destroy (the lab never runs the meteor; the soundmod trace does) | the definition's destroy lifecycle |

No sampled branch that a netbattle can take is missing from the port. The sample is small (10 of 1,257
routines); the one-sided list is the place to look for the next coverage scenarios.

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
  (§10). A scenario's own expectations are the check on that: 380 of the lab's recordings are marked `unmet` in
  its index (the chip never damaged the opponent, mostly because it can't from where the scenario stands), and
  those recordings match without showing what their descriptions say.

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
| Roles unfilled, or naming an action nothing implements | the loaded roles | `actions.turn` and `actions.volley`, each a `legacy` action number with no registration (0x3B, 0x30). Neither can start in a netbattle: turning is enabled only without the standard column patterns (`sub_80141F4`), and all 192 stages have pattern 0x38; no routine was found that raises the volley's request (0x40000000), and the port raises it nowhere |
| Errors that say something is not implemented, ported, modelled or supported | every `panic!`, `unreachable!` and Luau `error(` with that wording (12) | none reachable: the four bug-code writes in setup.rs (no hit carries such a code, unverified.md); a spawn by slot number (battle mode 9's two, and no content module spawns by number); an action by number (the two roles above); a weapon without a `setup` (none now); the mode states past the fade-out (battle effects 2); `sub_80EBB78`'s AI navi version byte |
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
every one. 80 of the 661 are still `unmet`: the chip runs to its end and doesn't damage the opponent from where
the scenario stands (ElecMan's and TomahawkMan's in several templates, HackJack's because the original stops).
The other 1,576 recordings that have a `settle` and a cut-in were recorded again too and came out frame for
frame as before.

With these and the scenarios added since, the lab runs 4,033 of the inventory's routines where it ran 3,991, and
the ported routines no recording runs fell from 260 to 225.
