# Moving content to scripts

The engine is a content-independent core and the BN6 ruleset's frameworks, in Rust; everything specific (a chip's
attack, a dimming chip's controller, a navi chip's navi, a weapon routine, the objects they spawn) is Luau in the
content pack (docs/design/scripting.md, docs/design/content-pack.md). Content is written in the content model's
second version (docs/design/content-model-v2.md): definitions with keys, composed by builders, with assets by name
and no numbers of the original's. This is how to move a piece of content from Rust, from a v1 module or from the
disassembly to a v2 definition, and what is left to move.

The exemplars (content model v2, step 7) show the patterns end to end, and are the models to copy:

| Exemplar | What it shows | Where |
|---|---|---|
| The bombs and seeds | A chip action as a builder (`throw.action { held, thrower }`); a thrown kind with its variant as a record (`bomb.variant { ... }`, `define.record`); a series in one module; shared definitions (collision types, effects, sparks, regions); an attachment look | lib/bombs/, chips/minibomb, bigbomb, energbom, flshbom, blkbomb, bugbomb, grasseed, iceseed, poisseed; rules/collision.luau, lib/{effects,sparks,regions}.luau, objects/attachment |
| The swords | One action builder for a family (`slash.action { blade, hit, effect, sound, ... }`); the transitional `legacy` marker; chips kept on records with their actions by compat key | lib/swords/, chips/sword ... chips/assnswrd |
| AreaGrab and PanelGrab | A dimming chip: the `dimming` hook spawning a controller kind whose update is `dimming_chips.phases { effect }`; a chip parameter that becomes the hook | lib/dimming.luau, lib/grab/, chips/areagrab, chips/panlgrab |
| EraseMan | A navi chip: a navi kind and a `navi` hook builder (`eraseman.summon { aim_ticks }`); kinds spawned with state instead of parameters; chips kept on records behind a registration shim | chips/eraseman/, chips/0ec-eraseman |
| BusterUp and the plus chips | Instant chips: an `instant` hook per chip; a shared library with the records' path beside it | chips/busterup, chips/atk-10, chips/navi-20, lib/instant/plus.luau, chips/0c0-atk-10 |
| MegaMan's buster, charged shot and blank shot; HeatCross's charged shot | Weapons: `define.weapon` with its action a definition, charge times of its own, the routine numbers it still answers to (`legacy`); a role the ruleset starts (`forced_charged_shot`, rules/roles.luau) | navis/00-megaman/weapons/{buster,charged-shot,blank-shot}, navis/00-megaman/forms/heatcross/charge.luau, lib/buster.luau |

The other weapon routines (navis/00-megaman/weapons/NN-name) are still v1, registered by number.

## 1. What moves and what stays

**Stays Rust**: the core (object pools and the update list, collision registration and resolution, the damage
pipeline, statuses and hit reactions, the panel grid, sprites and animation, RNG, input, sound cues, snapshots and
the digest), and the ruleset's frameworks and services:

- the navi framework: kinds/player (status, intake, reactions, idle, input, entry, chip use), movement (action
  0x10), form changes (transform, form and body overlays, the Cross merge), the Beast rush wrapper (the lock-on
  marker, afterimages, palette flashes), the charge glow;
- battle flow, the custom screen, panel rules (the volcano eruption);
- the services content calls: dimming (dimming.rs, action 0x15's framework), the navi-chip controller (kinds/
  navi_chip.rs, action 0x1B, the navi warp), the obstacle framework (kinds/obstacle.rs);
- the primitives: the one-tick hitbox (attack #3, `battle.hitbox`), the generic effect (effect #0, `battle.effect`),
  the hit spark (effect #4, `battle.spark`), the intro (effect #2).

A gap in any of these ("not implemented yet" in the framework) is fixed in Rust, not in content.

**Moves**: every chip action, every dimming chip's controller and navi chip's navi, every weapon routine, and the
object kinds they spawn. The ruleset reaches content through definitions (a chip's use, a kind, an action, a
weapon, a role) and, until the migration ends, through registration by number (§3.2).

## 2. The pattern

1. **Read the original.** Port from the disassembly, routine by routine, and every branch of it: a v1 module or
   the Rust (if there is one) is a guide, not the scope. Branches left as "not implemented" panics get ported. A
   branch the game can only take by running off a table or looping forever becomes an explicit `error(...)` naming
   the routine (AreaGrab's `sub_800D5BA`); one the game crashes on (a stack overflow) likewise.
2. **Place it by owner** (content-model-v2.md §4): a chip's definition in `chips/<key>/chip.luau` (a series in
   `chips/<key>/chips.luau`), the key compat's (compat/chips.toml); the kinds only it spawns beside it
   (`chips/eraseman/beam.luau`), keyed under it (`eraseman/beam`); what a family shares in `lib/<family>/`
   (builders, shared kinds: lib/bombs, lib/swords, lib/grab). The shared definitions are rules/collision.luau,
   lib/effects.luau, lib/sparks.luau and lib/regions.luau: add the entry you need there, named by what it does.
3. **Write the definitions.** `--!strict`; a header saying what it is, with the routine and object numbers; one
   local function per routine, commented with its name; the game's immediates as named constants.
   - A kind is `define.kind { id, pool, state, update }`; its state is typed fields, references included
     (`"object"`, `"record:bomb-variant"`, `"bool"`), and what the original passed as parameters becomes state
     the spawner sets (`eraseman/mark`'s `ticks`). Export its spawner (`mark.spawn(owner, x, y, ticks)`, the game's
     `sub_80E7942`).
   - An action is `define.action { id?, state, args, update }`, usually from a family's builder: what differs
     between chips is the builder's arguments, and a variant several kinds read is a record (`define.record`).
     Anything a definition holds is in the canonical tree, so a thrower is `{ throw = fn, variant = record }`,
     not a bare closure.
   - A chip is `define.chip { id, ...its record..., <one use> }`: `action`, `dimming`, `navi` or `instant`. The
     record is the pack's chip record field by field (`beast = { lockon = n }`, flags by name).
   - Assets by name: `asset.sprite("bomb")`, `asset.sound("sword-swing")`; the names are compat/assets.toml's (a
     placeholder such as `sprite-14-1b` is named first, with an entry in compat/curation.toml).
   - The API by definition and handle: `battle.spawn(kind, pos)`, `me:setup_collision(collision.thrown, ...)`,
     `battle.effect(pos, effects.explosion)`, `collision:set_hit_effect(sparks.erase)`. The numeric API
     (`spawn_kind`, `me:param`, `me.variant`, numbers for sounds, effects and collision types) is deprecated and
     counted by the ratchet; new code doesn't use it. Statuses, bug codes, NameIDs and `hit_effect = 0xFF` have no
     v2 form yet and stay numbers.
   - Mind the game's store widths (`me.lifecycle = "destroy"` is `strb`, `me:set_lifecycle(...)` the word store),
     its sprite-stepping routine (`update_sprite`, `update_sprite_while_dimmed`, `update_sprite_while_paused`,
     `step_sprite`), and the order of RNG draws.
4. **Compat.** Every key the traces see must be compat's: a chip's (chips.toml), a kind's (kinds.toml, with
   `scratch_position` for a position the spawner's registers leave), an action's (actions.toml: a chip's action
   is `<chip>/action`, which is the key it gets by default; give `id` when no chip holds it). compat/ is
   gen-content's; a key or name you add goes into compat/curation.toml too, for review.
5. **What stays numbered for now** (content-model-v2.md §12, "Transitional"). A chip the ruleset or another record
   names by number keeps the pack's record (a Program Advance's ingredient, a dark chip, a navi chip AntiNavi
   checks, a chip with a damage formula): its module returns its action with the compat key as `id`, and a
   registration-by-number shim runs it (chips/036-minibomb, chips/047-sword, chips/056-mchnswrd, chips/0ec-eraseman,
   chips/0c0-atk-10). A definition whose subtype or parameters a v1 module still reads carries
   `legacy = { subtype, params }` (the swords SlashCross charges). Both are counted and go with the numbers.
6. **API.** When a script needs something the API lacks, add it: a `CoreApi` method (crates/bn6-content-api/src/
   api.rs, documented with the routine it is), its implementation (crates/bn6-battle/src/behavior/core_api.rs),
   its binding (crates/bn6-luau/src/bind.rs), and its declaration with a comment in content/bn6/core.d.luau
   (types.d.luau for the families' types). Names, not numbers: a new set of flags or states is an enum with names
   in the API and a string-literal type in core.d.luau, and gets a misuse case in bn6-content-check's test.
7. **Delete the old.** The v1 module and its registration (`object.toml`, a `chip.toml`'s `script`), the Rust
   kind's module, its `kinds::Vars` variant, its arms in `kinds::update` and `actions::dispatch`, its
   `ActionVars` variant. Nothing exists twice. Lower the ratchet (`BN6_RATCHET_LOWER=1 cargo test -p
   bn6-content-check --test ratchet`); it may only shrink, except for a counted transitional use the design names.
8. **Test in the repository** (§4.1) and **against the traces and the chip lab** (§4.2).
9. **Docs.** docs/engine describes the game; point its mentions of content at the pack's module
   (object-kinds-pvp.md's table, for one), and add the family's "As built" notes to content-model-v2.md §5.
   Commit messages end with the session's attribution lines.

## 3. Registration

### 3.1 Definitions

| To implement | Write | The engine runs |
|---|---|---|
| An object kind | `define.kind { id, pool, state?, update }` | `update(me)` each tick it runs |
| A chip's action | `define.chip { ..., action = <Action> }` (`define.action { id?, state, args, update }`, usually a builder's) | the action as the navi's attack (CurAction reads as compat's number) |
| A dimming chip | `define.chip { ..., dimming = function(user, spec: DimmingChipSpec): Object? }` | action 0x15's framework spawns the controller through it; its update calls the `dimming` service (lib/dimming) |
| A navi chip | `define.chip { ..., navi = function(user, controller, spec: NaviChipSpec): Object? }` | the navi chip controller brings the navi through it; the navi calls `navi_chip.navi_left(controller)` |
| An instant chip | `define.chip { ..., instant = function(user, spec: InstantChipSpec) }` | action 0x1C runs it once |
| A weapon | `define.weapon { id, name, charge_ticks, setup }` (and, while forms name weapons by number, `legacy = { routines }`) | `setup(navi)` names the action |
| A role the ruleset starts | `define.roles { actions = { ... } }` | the ruleset's by-role starts (the trap chips' counters) |

A definition's key is its `id` (or the key it derives: `minibomb/action`); two of one key is an error.

### 3.2 Registration by number (transitional)

What the pack's records and the ruleset still name by number reaches v1 modules through registration files, until
step 13 removes them:

| To implement | Write | The module exports |
|---|---|---|
| An object kind | `objects/NAME/object.toml`: `[kind] pool, index, script` | `state` (optional), `update(me)` |
| A chip's action | `script = "..."` in `chips/NNN-name/chip.toml` | `state`, `update(me, s)` |
| A dimming chip (action 0x15) | `script` in each chip of the subtype | `dimming_chip(user, spec)` |
| A navi chip (action 0x1B) | `script` in each chip of the subtype | `navi_chip(user, controller, spec)` |
| An instant chip (action 0x1C) | `script` in each chip of the subtype, or `instant_chip = N` in a weapon.toml | `instant_chip(user, spec)` |
| A weapon routine | `navis/00-megaman/weapons/NN-name/weapon.toml`: `id, name, script` and optionally `action` | `setup(navi) -> action`; with `action`, also `state` and `update(me, s)` |

A shim is such a module that runs the definitions' own code for the records (chips/036-minibomb runs each bomb
chip's action by subtype). Scripts are paths relative to the registering file; a slot, action or hook claimed twice
is an error.

## 4. Testing

### 4.1 In the repository

In-repo tests never load game data. The test content (crates/bn6-battle/src/content/testing.rs) is made-up
records plus the overlay's modules, which it reads from content/bn6 at test time:

- add the modules (and their `require`s) to `scripts()`'s list; the asset names they resolve to `assets()` (BN6's
  names with BN6's numbers); whatever they read (an attachment row, a rule) made-up values; the sprites they load
  short animations in `animations()`;
- a chip content defines is in the test content by its module (`testing::defined_chip(testing::AREA_GRAB)`);
  folders hold it by handle (`scenario::setup_with_handles`, in code A, else `*`); a test uses it with
  `use_chip_handle` or `use_instant_chip_handle`. A v1 kind keeps its `kind(...)` line, a numbered chip its
  `chips()` record;
- test it: `behavior/tests.rs` plays duels (`duel_with`, `scenario::record_on`) and checks the kinds appear and
  roll back (`scripted_chips_roll_back`); `kinds/player/actions/tests.rs` runs one navi's action tick by tick and
  checks its timeline;
- `battles_run_the_content_scripts` lists the test content's kinds by key: update it.

Then:

```sh
cargo run -p bn6-content-check -- content/bn6     # every module type-checks, the lints
cargo build --workspace --all-targets             # no warnings
cargo test --workspace                            # includes the rollback tests, the type check and the ratchet
```

### 4.2 Against the traces and the chip lab

The golden traces and the chip lab are in the verification workspace, a separate checkout that builds this
repository's crates by path. Extract a pack from your checkout (it carries the asset name index) and run the
workspace's tests against your checkout on it:

```sh
cargo run --release -p bn6-extract -- content <rom> <pack>
BN6_PACK=<pack> <verification>/tools/traces-against.sh <checkout> --release
BN6_PACK=<pack> <verification>/tools/traces-against.sh <checkout> --release --test lab -- --ignored
```

The floors today: machgun 1074/1074 and 1331/1331, soundmod 6728/6857/3088, the rollback test matching every
confirmed frame at latencies 0+0 to 10+3; the chip lab 2572 of 3621 scenarios fully matched (2,018,981 frames).
Compare the lab's summary.md with the workspace's: the families you converted must match as before. Where the
workspace's tests name something that moved, the change goes into a patch for the workspace's owner (don't edit it).

Rollback cost: `cargo run --release -p bn6-netplay --example rollback_cost -- <trace.jsonl> <pack>
<round>`; today 56 to 110 µs per rendered frame (scripting.md §7.3). Report the change.

## 5. What is left

Grouped so that groups can run in parallel: each group owns the files it names. Every group also adds to the
shared files below; keep those edits local (next to related entries, one entry per line) so merges are mechanical.

**Shared files**: content/bn6/core.d.luau, crates/bn6-content-api/src/api.rs, crates/bn6-battle/src/behavior/
core_api.rs, crates/bn6-luau/src/bind.rs (API additions); crates/bn6-battle/src/content/testing.rs (test records);
crates/bn6-battle/src/kinds/mod.rs and kinds/player/actions/mod.rs (deleting dispatch arms and state variants);
crates/bn6-battle/src/behavior/tests.rs (the registration lists); docs/engine/object-kinds-pvp.md.

### Group A: navi chips

Done (wave 2): the navi parts service (`me:add_navi_parts` / `me:remove_navi_parts`, `sub_8010DF6`/`sub_8011044`
by actor record: the navi hooks in `kinds::player::form`, with SpoutMan's idle overlay `kinds::idle_overlay`), and
as pack scripts with every kind they spawn: ElmntMan (all four elements; `kinds/elmnt_man.rs` and `kinds/meteor.rs` deleted), SpoutMan, HeatMan,
ElecMan, SlashMan, ChargeMan, TomahawkMan, TenguMan, BlastMan, Roll, ProtoMan, Colonel (and CrossDiv), Bass,
BassAnly, DeltaRay, SunMoon. `bring_navi`'s fallback is a content error (HackJack's and Django's entries are NULL:
an explicit error). Every scratch-lab scenario of these chips matches (docs/engine/chips.md §3.6.7 on).

Left:
- GroundMan, DustMan, DiveMan, CircusMan, JudgeMan (navis 10, 11, 13, 14, 15): the name looks data
  (`byte_8021220`) is in; the navis aren't registered. Unverified work in progress for all five is on branch
  `worktree-agent-a4a2385d487c33845` (its falling rock and rubble predate group H's objects/falling-rock and
  objects/rock-chip, which it should use, and its obstacle calls predate the `obstacle` service).
- TwinLdrs (20), CrosOver (21), MstrCros (22), BigHook (23), Darkness (24): not ported.
- Roll against the other side's AntiRecv (`sub_80E192C`, chip 0xBD: the trap chips' `sub_80E37D2`, group B).
- The PA chips' lab recipes stop at the custom screen's PA banner and hand (not navi-chip code).

### Group B: dimming chips

Done: the dimming chips have no Rust fallback (kinds/player/actions/dimming_chip.rs calls `Hook::DimmingChip`
only), and the controllers declare `scratch_position` (trace.rs keeps only the navi chip controller). Scripts:
subtypes 1 (objects/invisible), 6 (objects/rock-cube), 20 (objects/trap-chip, with ElemTrap's trap
objects/elem-trap, its strike objects/elem-trap-strike and objects/panel-bursts), 10 (objects/time-bom,
objects/countdown-bomb), 11 (objects/mine, objects/land-mine), 25 (objects/gauge-speed), 38 (objects/navi-boost).
Shared: lib/panels (the game's panel lists and shuffle), objects/rising-bubble (effect #0x14).

Left (each a controller and its objects, every branch; docs/engine/chips.md §3.6.10 has what is known):

- 14 Guardian: objects/guardian, guardian-statue and guardian-strike are written but no chip names them yet and
  they are unverified; register chip 0x097 and check them against the lab.
- 4 Barrier (with the FirstBarrier framework `sub_801A7CC` and the barrier visual, effect #7), 5 PanlRetrn and the
  road/holy chips (its 19-row table is pack data to extract), 9 Fanfare and kin, 13 AirRaid, 26 BugFix, 27
  ColorPt/DblPoint, 28 Sensor, 36 SumnBlk (group B2a): specified branch by branch in docs/engine/dimming-chips.md,
  waiting for content model v2.
- 2 (no chip), 3 Geddon and the capsules, 7 LifeSync, 8 Wind/Fan, 12 Snake, 15
  GrabBnsh/GrabRvng, 16 Meteors, 17 Anubis/PoisPhar, 18 Otenko, 19 CircGun, 21 BlzrdBal, 22 NumbrBl, 23 BurnSqr,
  24 Magnum, 29 CornFsta, 30 DblHero, 32 MetrKnuk, 37 DblBeast (group B2b): specified branch by branch in
  docs/engine/dimming-chip-effects.md (14 Guardian's scripts exist, unregistered), waiting for content model v2;
  31, 33 and 41 (no chip; their actors are navi chips' navis).
- Framework (Rust): the counter cut-in (`sub_8017AB4`, kinds/player/status.rs; chips.md §3.6.5 has the port's
  notes), encased obstacles (`sub_801813A`; thrown ones, `sub_8018002`, are ported). AntiNavi in the dimming
  service is done (dimming.rs; dimming-chips.md §2).

### Group C: DustCross and the Beast forms' weapons (ported; what is left)

Ported (navis/00-megaman/weapons/, objects/): every form weapon routine of `off_80117D4` the forms name (0x03,
0x04, 0x06, 0x07..0x0C, 0x0F..0x12, 0x14..0x17, 0x19..0x1E, 0x27, 0x2A, 0x2C; 0x06, 0x0B, 0x0C and 0x0F are setups
whose actions are standard chips'), their actions (0x1A, 0x1D, 0x1E, 0x35, 0x3A, 0x3C, 0x3D, 0x41, 0x45, 0x46, 0x4A,
0x4C..0x50, 0x52, 0x56, 0x58) and kinds, and the absorbed obstacle. The chip-use framework's charged paths
(`sub_80127C0(charged)`, `sub_8012C7C`, the cross doubles of `sub_8012A38`, GroundCross's A-charge 0x18
`sub_8012CB2`) are group H's `chip_use.rs`, with the A-charge 0xFF path's argument (the chip's family byte) from
this group; GroundCross's drill uses objects/drill and EraseCross's beam objects/thunder-column (one script per
kind). Left:

- Blocked by the framework: a charged use of the empty hand in a form without an A-charge routine (it needs the
  empty hand's family byte, which `rules/weapons.toml [empty_hand]` doesn't carry), Cross Beast (`sub_8014F40`),
  form flags of forms 7, 8, 0xB, the reactive abort (`sub_801056A`).
- Unverified (no scenario reaches them yet): every Beast Cross A-charge and the Beast busters past their first tick;
  the Cross charged shots 0x41, 0x45, 0x4A, 0x4D; `lockon_panel`'s not-found result ((0, 0x7F) here; the cross
  fork's reading was column 0 and a leftover row).

### Group D: the buster's shots

- Action 0x11 (`sub_80EB436`: the shot, the spread's extra rows, the absorbed-obstacle throw `sub_80C6248`, the
  muzzle flash attachment 5 in the first related slot) and 0x16 (`sub_80EBE00`, the charged shot).
- The projectile they fire, attack #0 (`sub_80C4E58`), shared by many chips: its 12-byte records by Param1
  (`off_80C4C78`) become pack data (a rules or object file), with every branch (the panel crack, break and type
  changes by Param1, `sub_80C5014`, `sub_80C5050`).
- The weapon ids that alias the buster (`off_80117D4` entries pointing at `sub_8011A26`: 0x2E, 0x2F, 0x3E, 0x3F,
  0x4D..0x51, 0x6F, 0x70, 0x77, 0x79, 0x7B, 0x7E, 0x82): the buster definition's `legacy` routines.
- Owns: new objects/ and weapons/ folders, the extractor and pack IO for the projectile table.
- Done: `objects/projectile` (kinds in its `object.toml`, `data.objects.projectiles`), fired with
  `lib/projectile.luau` (`projectile.fire(navi, shot)`, `projectile.spawn(owner, x, y, shot)`, the shot typed as
  `ProjectileShot` in types.d.luau); `objects/flying-shot` (attack #0xB, `sub_80C6248`'s object, with its kinds,
  `data.objects.flying_shots`), which the Beast buster and TrnArrw fire too; actions 0x11 and 0x16, now the
  definitions in `weapons/buster` and `weapons/charged-shot` (step 7).

### Group E: instant chips (ported; what is left)

Action 0x1C runs a chip definition's `instant` hook, or a record's subtype's registration (§3). Every entry of
`off_80EC3F0` is ported, in content model v2 (content-model-v2.md §5.6, "As built", step 8d): 0, 3
(lib/instant/plus with chips/atk-10, chips/navi-20, chips/whicapsl, chips/finalgun, chips/numtrap; the records'
shims chips/13f-beastout and chips/0c0-atk-10; objects/rising-bubble), 1 (objects/boomerang, chips/boomer), 4
(chips/lance), 5 (chips/fullcust), 8 (chips/firehit; the records' shim chips/06b-firehit1), 10 (chips/busterup), 12
(chips/sandwrm), 13 (chips/synctrgr), 14 (objects/flame-hook, flame-hook-fire), 15 (objects/col-force,
col-force-soldier), 19 (chips/justcone), 20 (weapons/10-tengu-wind, objects/gust), 21 (chips/golmhit), 22
(objects/col-army); 7 and 0x12 are NULL (an explicit panic). 2, 6, 9, 11, 16 and 17 have no chip or MegaMan weapon:
they are builders in lib/instant, which the link navis' weapons (0x71, 0x83) and actions call when ported. Left: the
Full Synchro aura after SyncTrgr (framework), attack #0x12 (the soldiers' vulcan hit), which ColArmy's and
ColForce's soldiers spawn by number.

### Group F: rocks

Done: the rock (objects/rock) and its debris (objects/rock-debris) are kinds on the obstacle framework (the
`obstacle` service); the actor lists' rocks go through the rock's `actor_list_entry` (`Hook::ActorListEntry`).
The verification workspace's rock_trace and bn6_data tests need the updated copies (they named `kinds::rock`).

### Group G: standard chip actions

No Rust exists for these; each is new and independent (one folder per chip action or family, so several agents can
split the list):

- RskyHny (action 0x39): soundmod round 3 stops there.
- Cannons 0x14, swords 0x13, bombs and seeds 0x12, Vulcan 0x17, YoYo 0x18, BatCan 0x19, Thunder 0x1F, recovery 0x20,
  AirShot 0x21, CrakShot 0x22, CopyDmg 0x23, TankCan 0x24, Spreader 0x25, AirHocky 0x26, FireBrn 0x27, TrnArrw 0x28,
  MachGun 0x29, CornSht 0x2A, Reflectr 0x2B, IronShl 0x2C, BblStar 0x2D, DrilArm 0x2E, Tornado 0x2F, WideSht 0x30,
  WaveArm 0x31, AquaNdl 0x32, H-Burst 0x34, RlngLog 0x36, AirSpin 0x38, DolThdr 0x3E, WindRack 0x3F, MoonBld 0x40,
  ElcPuls 0x42, AuraHed 0x43, MagCoil 0x44, the sword family 0x49, the dragons 0x51, VarSwrd 0x53, NeoVari 0x54,
  SonicBom 0x55, ZSaver 0x5B, and the Cross and Beast chips' actions (0x0A).
- Many fire the projectile of group D; start with the ones that don't, or after it.

### Framework gaps (Rust, not content)

These are the ruleset's, and are fixed in Rust by whoever needs them: the barrier routine and visual
(dimming-chips.md §3), the Full
Synchro aura, Cross changes and Cross Beast, Beast Over, the NaviCust hooks (style, emotion timer, low HP, chip
interception, the panel trail and auto-step bugs), dark chips, the SELECT/Cross specials, the status visuals (ice,
bubble, confusion, blindness), reactive defensive chips (`sub_801056A`), mid-battle appearance, link navis' actions.
`grep -rn "not implemented yet" crates/bn6-battle/src` lists them.
