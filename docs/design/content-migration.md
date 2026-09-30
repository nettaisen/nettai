# Moving content to scripts

The engine is a content-independent core and the BN6 ruleset's frameworks, in Rust; everything specific (a chip's
attack, a dimming chip's controller, a navi chip's navi, a weapon routine, the objects they spawn) is a Luau script
in the content pack, with its data (docs/design/scripting.md, docs/design/content-pack.md). This is how to move a
piece of content from Rust, or from the disassembly, to a script, and what is left to move.

Three exemplars show the pattern end to end, and are the models to copy:

| Exemplar | What it shows | Where |
|---|---|---|
| AreaGrab and PanelGrab | A dimming chip: the controller's `dimming_chip` hook calling the dimming service; the object it spawns; a chip whose script is an object's module; register-garbage position (`scratch_position`) | content/bn6/objects/area-grab, objects/grab-shot, chips/0a2-panlgrab, chips/0a3-areagrab |
| EraseMan | A navi chip: the navi's `navi_chip` hook and `navi_chip.navi_left`; three kinds that spawn each other; branches the Rust never had (the slash's navi-AI variant) | objects/erase-man, objects/erase-mark, objects/erase-beam, chips/0ec-eraseman (and EX, SP) |
| MegaMan's buster | Weapon routines (`setup`) and the actions they bring (`state`/`update`); a shared library (`lib/buster.luau`); rules data (`data.rules.buster_recovery`); an object kind with a garbage Z fraction (`scratch_z_fraction`) | navis/00-megaman/weapons/*, lib/buster.luau, objects/dust-ball |

GunDelSol (chips/00f-gundels1, objects/attachment, objects/sun-beam) is the plain chip action.

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
object kinds they spawn. The ruleset reaches content only through registration (§3): a script for an action, a
kind, or a numbered hook.

## 2. The pattern

1. **Read the original.** Port from the disassembly, routine by routine, and every branch of it: the Rust version
   (if there is one) is a guide, not the scope. Branches the Rust left as "not implemented" panics get ported. A
   branch the game can only take by running off a table or looping forever becomes an explicit `error(...)` naming
   the routine (AreaGrab's `sub_800D5BA`); one the game crashes on (a stack overflow) likewise.
2. **Place it by owner.** A chip's own action goes in the chip's folder (`chips/NNN-name/chip.luau`, and
   `script = "chip.luau"` in its `chip.toml`); chips that share it name that module. An object kind goes in
   `objects/NAME/` (`object.toml` with `[kind]`, and `NAME.luau`). A dimming chip's controller or a navi chip's
   navi is an object kind; its module also exports the hook, and the chips name the module. A weapon routine
   goes in `navis/00-megaman/weapons/NN-name/` (`weapon.toml`, the module). Helpers several modules share go in
   `lib/`. All of it is in content/bn6, the source overlay, which mirrors the pack.
3. **Write the module.** `--!strict`; a header saying what it is, with the routine and object numbers; one local
   function per routine, commented with its name; the game's immediates as named constants; `export type State`
   and the `state` schema for what it keeps (the object's own fields first: timers, params, the header). Export
   the kind's spawner (`erase_mark.spawn(owner, x, y, ticks)`, the game's `sub_80E7942`) for its callers to
   `require`. Mind the game's store widths (`me.lifecycle = "destroy"` is `strb`, `me:set_lifecycle(...)` is the
   word store), its sprite-stepping routine (`update_sprite`, `update_sprite_while_dimmed`,
   `update_sprite_while_paused`, `step_sprite`), and the order of RNG draws.
4. **Data.** Numbers that differ between entities of the same code are pack data, read through `data` (a chip's
   own table in its `chip.toml`, a rule in `rules/`). A table the ROM has that the pack doesn't yet: extract it
   (bn6-extract's battle.rs), write and read it (bn6-content's battle.rs, into the entity's folder by the
   by-owner rule), hold it in `Content` (bn6-battle's content module), expose it to scripts (behavior/data.rs) and
   type it (`PackData` in core.d.luau). Nothing ROM-derived is committed: the test content gets made-up values.
5. **API.** When a script needs something the API lacks, add it: a `CoreApi` method (crates/bn6-content-api/src/
   api.rs, documented with the routine it is), its implementation (crates/bn6-battle/src/behavior/core_api.rs,
   calling the engine's existing function), its binding (crates/bn6-luau/src/bind.rs), and its declaration with a
   comment in content/bn6/core.d.luau. Names, not numbers: a new set of flags or states is an enum with names in
   the API and a string-literal type in core.d.luau, and gets a misuse case in bn6-content-check's test.
6. **Register garbage.** An object whose position is its spawner's registers: reproduce the value when it is
   knowable (spawn with it), else declare `scratch_position = true` (or `scratch_z_fraction = true`) in its
   `[kind]`, which the trace comparison reads.
7. **Delete the Rust.** The kind's module, its `kinds::Vars` variant, its arms in `kinds::update` and
   `actions::dispatch` or in the hook's fallback (`idle::weapon_routine`, `dimming_chip::update`,
   `navi_chip::bring_navi`), its `ActionVars` variant, and references in trace.rs. Rust callers of the deleted
   code call the script through its registration, or move to content too. Nothing exists twice.
8. **Test in the repository** (§4.1): register the module in the test content and exercise it.
9. **Test against the traces** (§4.2): the frame floors must not drop, and a trace that now runs further records
   its new floor.
10. **Docs.** docs/engine describes the game; point its "Rust" mentions at the pack's module
    (object-kinds-pvp.md's table, for one). Commit messages end with the session's attribution lines.

## 3. Registration

| To implement | Write | The module exports |
|---|---|---|
| An object kind | `objects/NAME/object.toml`: `[kind] pool, index, script` (`scratch_position`, `scratch_z_fraction`) | `state` (optional), `update(me)` |
| A chip's action | `script = "..."` in `chips/NNN-name/chip.toml` | `state`, `update(me, s)` |
| A dimming chip (action 0x15) | `script` in each chip of the subtype | `dimming_chip(user, spec: DimmingChipSpec) -> Object?`: spawn the controller; its update calls the `dimming` service's steps in its routine's order |
| A navi chip (action 0x1B) | `script` in each chip of the subtype | `navi_chip(user, controller, spec: NaviChipSpec) -> Object?`: spawn the navi; it calls `navi_chip.navi_left(controller)` when done |
| A weapon routine | `navis/00-megaman/weapons/NN-name/weapon.toml`: `id, name, script` and optionally `action` | `setup(navi) -> action`; with `action`, also `state` and `update(me, s)` |

Scripts are paths relative to the registering file. Two chips of an action or subtype must name the same module;
a slot, action or hook claimed twice is an error (`Content::registrations`, `Registrations::validate`).

A new kind of registration (a ruleset table that dispatches by number, like the instant chips' `off_80EC3F0`) is a
new `Hook` variant: crates/bn6-content-api/src/host.rs (the variant and its function name), the argument struct and
`HookCall` variant, content/scripts.rs (what registers it), behavior/mod.rs (its lookup table), bind.rs (the call),
core.d.luau (the spec type), and the ruleset's call site with its Rust fallback until nothing uses it.

## 4. Testing

### 4.1 In the repository

In-repo tests never load game data. The test content (crates/bn6-battle/src/content/testing.rs) is made-up
records that register the overlay's modules, which it reads from content/bn6 at test time:

- add the module (and its `require`s) to `scripts()`'s list, the kind to `kinds()` (in name order), a chip with
  the action and subtype to `chips()` (a new `pub const` id, the next free), or a weapon to `weapons()`;
- give the sprites it loads short animations in `animations()`, and whatever data it reads (an attachment row, a
  rule) made-up values;
- test it: `behavior/tests.rs` plays duels (`scenario::setup_with(&[chips])`, `scenario::record_on`) and checks
  the kinds appear and roll back (`scripted_chips_roll_back`); `kinds/player/actions/tests.rs` runs one navi's
  action tick by tick (`fight_with(megaman_with(|s| ...))`) and checks its timeline;
- `battles_run_the_content_scripts` and `registrations_follow_the_content_data` list what the test content
  registers: update them.

Then:

```sh
cargo run -p bn6-content-check -- content/bn6     # every module type-checks
cargo build --workspace --all-targets             # no warnings
cargo test --workspace                            # includes the netplay rollback tests and the type check
```

### 4.2 Against the traces

The golden traces are in the verification workspace, a separate checkout that builds this repository's crates by
path and reads an extracted pack at `data/content/bn6`:

```sh
cargo run --release -p bn6-extract -- content <rom> <verification>/data/content/bn6
cargo test --release -p trace-tests      # traces, rollback at every latency, rock_trace, bn6_data, custom_screen
cargo test --release -p sound-tests      # the sound calls
```

(run in the verification workspace). The floors today: machgun rounds 1 and 2 complete (1074/1074, 1331/1331),
soundmod 4513/6284/2566 (rounds 1 and 2 stop at SpoutMan, navi chip navi 7; round 3 at action 0x39, RskyHny), the
rollback test matching every confirmed frame at latencies 0+0 to 10+3, and the sound calls (machgun 53 over 1651
frames, soundmod 35 over 8596). A trace exercises a script only where it reaches it: grep the trace for the object's
`"type"` and `"index"` or the player's `"state":[4,ACTION,` to see whether and when. Where the workspace's tests
name something that moved (a Rust kind's `INDEX`, a renamed flag), update them to the content's
(`b.content.object_kind("sun-beam")`).

Rollback cost: `cargo run --release -p bn6-netplay --example rollback_cost --features trace -- <trace.jsonl> <pack>
<round>`; today 56 to 110 µs per rendered frame (scripting.md §7.3). Report the change.

## 5. What is left

Grouped so that groups can run in parallel: each group owns the files it names. Every group also adds to the
shared files below; keep those edits local (next to related entries, one entry per line) so merges are mechanical.

**Shared files**: content/bn6/core.d.luau, crates/bn6-content-api/src/api.rs, crates/bn6-battle/src/behavior/
core_api.rs, crates/bn6-luau/src/bind.rs (API additions); crates/bn6-battle/src/content/testing.rs (test records);
crates/bn6-battle/src/kinds/mod.rs and kinds/player/actions/mod.rs (deleting dispatch arms and state variants);
crates/bn6-battle/src/behavior/tests.rs (the registration lists); docs/engine/object-kinds-pvp.md.

### Group A: navi chips

- ElmntMan (navi 16, actor #0x10, `kinds/elmnt_man.rs`) and the meteors he drops (`kinds/meteor.rs`, attack 0x8D), with the
  branches the Rust leaves out (his Aqua, Elec and Wood: `sub_80BAD06`, `sub_80BAD76`, `sub_80BAD34`,
  `sub_80BAF06`; meteors outside a dimming).
- SpoutMan (navi 7): soundmod rounds 1 and 2 stop there, so it moves their floors.
- Then the other navis of `off_802CD5C` (Roll, ProtoMan, HeatMan, ElecMan, SlashMan, ChargeMan, TomahawkMan,
  TenguMan, GroundMan, DustMan, BlastMan, DiveMan, CircusMan, JudgeMan, Colonel, HackJack, Django, ...), each a
  folder under objects/ with its chips naming it.
- Owns: kinds/elmnt_man.rs, kinds/meteor.rs (deleted), the fallback in kinds/navi_chip.rs `bring_navi`.

### Group B: dimming chips

- Invisible's controller (subtype 1, effect #0x5D, `kinds/invisible.rs`) and the trap chips' (subtype 20, AntiDmg
  and its kin, effect #0x2A, `kinds/trap_chip.rs`, with the trap object `sub_80CE0EC` it lacks).
- Then the other subtypes of `off_802CCB4` (Geddon, Barrier, PanlRetrn, RockCube, Wind, TimeBom, Mine, AirRaid,
  Meteors, Anubis, Sensor, ...).
- Their controllers' register-garbage positions move from trace.rs's `pos_is_garbage` list to `scratch_position`
  in their `[kind]`.
- Owns: kinds/invisible.rs, kinds/trap_chip.rs (deleted), the fallback in kinds/player/actions/dimming_chip.rs,
  trace.rs.

### Group C: DustCross and the Beast forms' weapons

- DustCross's B+Back: weapon 0x2A and action 0x58 (`kinds/player/actions/absorb.rs`), the absorbed obstacle
  (effect #0x87, `kinds/absorbed_obstacle.rs`, whose spawn the obstacle framework calls: through
  `behavior::spawn_kind`).
- The Beast forms' claw: weapon 0x1E and action 0x52 (`kinds/player/actions/beast_claw.rs`).
- Weapon 0x2C (`sub_8011FCE`, the other absorbed-obstacle throw) and `sub_8011AF2` it falls back to; the forms'
  other routines (0x03, 0x04, and the charged shots 0x0B, 0x0C, 0x0F, 0x12, 0x14, 0x16, 0x19, 0x27, the A-charges).
- Owns: actions/absorb.rs, actions/beast_claw.rs, kinds/absorbed_obstacle.rs (deleted), the fallback in
  kinds/player/idle.rs `weapon_routine`, navis/00-megaman/weapons/ (new folders only).

### Group D: the buster's shots

- Action 0x11 (`sub_80EB436`: the shot, the spread's extra rows, the absorbed-obstacle throw `sub_80C6248`, the
  muzzle flash attachment 5 in the first related slot) and 0x16 (`sub_80EBE00`, the charged shot).
- The projectile they fire, attack #0 (`sub_80C4E58`), shared by many chips: its 12-byte records by Param1
  (`off_80C4C78`) become pack data (a rules or object file), with every branch (the panel crack, break and type
  changes by Param1, `sub_80C5014`, `sub_80C5050`).
- The weapon ids that alias the buster (`off_80117D4` entries pointing at `sub_8011A26`: 0x2E, 0x2F, 0x3E, 0x3F,
  0x4D..0x51, 0x6F, 0x70, 0x77, 0x79, 0x7B, 0x7E, 0x82) as weapons naming 00-buster's module.
- Owns: new objects/ and weapons/ folders, the extractor and pack IO for the projectile table.

### Group E: instant chips

- Action 0x1C (`sub_80EC39C`, `kinds/player/actions/instant.rs`) runs a chip routine by subtype (`off_80EC3F0`):
  a new hook (§3), then FullCust and the rest (Boomer, MegaBstr and the Atk+ chips, Lance, FireHit, BusterUp,
  SandWrm, SyncTrgr, FlmHook, ColForce, JustcOne, GolmHit, ColArmy, BeastOut).
- Owns: actions/instant.rs (deleted), the new hook in bn6-content-api's host.rs, content/scripts.rs and
  behavior/mod.rs.

### Group F: rocks

- The rock (attack #0x59, `kinds/rock.rs`) and its debris (effect #0x38, `kinds/rock_debris.rs`) as kinds on the
  obstacle framework;
  the stage spawns rocks at the start (`battle.rs`, `kinds::rock::spawn_at_start`) through the kind.
- The verification workspace's rock_trace and bn6_data tests name `kinds::rock`; update them.
- Owns: kinds/rock.rs, kinds/rock_debris.rs, kinds/rock_tests.rs, battle.rs's actor-list spawn.

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

These are the ruleset's, and are fixed in Rust by whoever needs them: AntiNavi in the dimming service, the Full
Synchro aura, Cross changes and Cross Beast, Beast Over, the NaviCust hooks (style, emotion timer, low HP, chip
interception, the panel trail and auto-step bugs), dark chips, the SELECT/Cross specials, the status visuals (ice,
bubble, confusion, blindness), reactive defensive chips (`sub_801056A`), mid-battle appearance, link navis' actions.
`grep -rn "not implemented yet" crates/bn6-battle/src` lists them.
