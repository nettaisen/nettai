# How to write content

The engine is a content-independent core and the BN6 ruleset's frameworks, in Rust; everything specific (a
chip's attack, a dimming chip's controller, a navi chip's navi, a weapon, a link navi's attacks, the objects
they spawn, the stages, the rule tables) is Luau in the content root, content/bn6
(docs/design/content-pack.md). Content is **definitions with keys**, composed by builders, with assets by name
and none of the original's numbers. This is how to write a piece of it: where it goes, what it is made of, the
conventions, and how to test it. The runtime and the API's shape are in docs/design/scripting.md; where the
line between engine and content runs in docs/design/core-content-boundary.md; the reasons behind the model in
docs/design/content-model-v2.md.

## 1. The exemplars

These show the patterns end to end, and are the models to copy:

| Exemplar | What it shows | Where |
|---|---|---|
| The bombs and seeds | A chip action as a builder (`throw.action { held, thrower }`); a thrown kind with its variant as a record (`bomb.variant { ... }`, `define.record`); a series in one module; shared definitions (collision types, effects, sparks, regions); an attachment look | lib/bombs/, chips/minibomb, bigbomb, energbom, flshbom, blkbomb, bugbomb, grasseed, iceseed, poisseed; rules/collision.luau, lib/{effects,sparks,regions}.luau, objects/attachment |
| The swords | One action builder for a family (`slash.action { blade, hit, effect, sound, ... }`); what SlashCross's charge makes of a sword as a record the chip names (`charged`) | lib/swords/, chips/sword ... chips/assnswrd |
| AreaGrab and PanelGrab | A dimming chip: the `dimming` hook spawning a controller kind whose update is `dimming_chips.phases { effect }`; what differs between the chips as the hook's arguments | lib/dimming.luau, lib/grab/, chips/areagrab, chips/panlgrab |
| The trap chips | A dimming chip that leaves the side's defensive-chip record; a counter the ruleset starts by role | lib/traps/, chips/antidmg, chips/antiswrd, chips/bodygrd; rules/roles.luau |
| EraseMan | A navi chip: a navi kind and a `navi` hook builder (`eraseman.summon { aim_ticks }`); kinds spawned with state; a series' chips composing the hook with their own arguments, an SP chip's damage a formula | chips/eraseman/ (navi, mark, beam, chips) |
| BusterUp and the plus chips | Instant chips: an `instant` hook per chip; a shared library | chips/busterup, chips/atk-10, chips/navi-20, lib/instant/plus.luau |
| The link navis' own chips | A chip in its navi's folder, its damage by the navi's level (`damage = { formula = "navi_level", ... }`), which the navi's `own_chip` names; a phased routine on the attack's step with shared helpers; kinds beside the navi | lib/link_chips.luau, navis/heatman ... navis/dustman (chip.luau and their kinds) |
| MegaMan's weapons | `define.weapon` with its action a definition, charge times of its own, the traits the ruleset asks (`held`, `plain`, `sticky`, `charged_chip`); a setup that writes its action's state (`navi:action_state(action)`); a weapon whose effect is instant (`instant`); a weapon that asks the attack's chip for its part (`navi.attack_chip`: SlashCross's charged slash) | navis/megaman/weapons/NAME/weapon.luau, navis/megaman/forms/FORM/, lib/buster.luau, lib/weapon.luau |
| The link navis' charged attacks | A weapon whose action is the navi's own; a kind that lasts while its owner's action does (`owner:navi_action() ~= s.action`); a weapon that runs a chip's action with the chip as the attack's (ProtoMan's WideSwrd) | navis/heatman/charge.luau ... navis/dustman/charge.luau, navis/groundman/drill.luau, navis/protoman/charge.luau, back_special.luau |
| The rock, the cubes and the statue | Field obstacles on the `obstacle` service; variants as records; a kind a stage places (`place`) | chips/rockcube (the rock and its debris beside the cubes), chips/guardian, stages/netbattle.luau |
| The roles | What the ruleset starts, spawns, shows and plays itself, by role: actions, kinds, chips, statuses, lock-on modes, effects, sparks, regions, collision types, sounds, music, banners, sprites, hooks | rules/roles.luau |

## 2. What is content and what is the engine's

**The engine's** (Rust): the core (object pools and the update list, collision registration and resolution, the
damage pipeline, statuses and hit reactions, the panel grid, sprites and animation, RNG, input, sound cues,
snapshots and the digest), and the ruleset's frameworks and services:

- the navi framework: kinds/player (status, intake, reactions, idle, input, entry, chip use), movement, form
  changes (transform, form and body overlays, the Cross merge), the Beast rush wrapper (the lock-on marker,
  afterimages, palette flashes), the charge glow;
- battle flow, the custom screen, panel rules (the volcano eruption);
- the services content calls: dimming (dimming.rs and the dimming chip action), the navi-chip controller
  (kinds/navi_chip.rs, the navi warp), the obstacle framework (kinds/obstacle.rs);
- the primitives: the one-tick hitbox (`battle.hitbox`), the generic effect (`battle.effect`), the hit spark
  (`battle.spark`), the afterimage, the form overlay, the palette flash, the intro.

A gap in any of these is fixed in Rust, not in content.

**Content**: every chip and its use, every dimming chip's controller and navi chip's navi, every weapon, the
link navis' own actions, the object kinds they spawn, the navis and forms, the stages, the rule sections, and
the shared effects, sparks, regions, collision types, statuses, lock-on modes and identities. The ruleset
reaches content through definitions (a chip's use, a weapon a form names, a kind a stage places) and through
roles (rules/roles.luau: what it starts, spawns and shows itself), never by number and never by key.

## 3. Writing a piece of content

1. **Read the original.** Port from the disassembly, routine by routine, and every branch of it. A branch the
   game can only take by running off a table or looping forever becomes an explicit `error(...)` naming the
   routine (`error("sub_8109746 scans past the field's edge forever")`); one the game crashes on likewise.
   Nothing is left as "not implemented".
2. **Place it by owner.** A chip's definition is `chips/<key>/chip.luau` (a series in
   `chips/<key>/chips.luau`), the key compat's (compat/chips.toml); the kinds only it spawns beside it
   (`chips/eraseman/beam.luau`), keyed under it (`eraseman/beam`); what a family shares in `lib/<family>/`
   (builders, shared kinds: lib/bombs, lib/swords, lib/grab); a kind several owners spawn in `objects/<kind>/`.
   A navi's own chip, weapons and kinds are in its folder; a form's in the form's. The shared definitions are
   rules/collision.luau, lib/effects.luau, lib/sparks.luau and lib/regions.luau: use the entry that is there
   (one definition per collision type: `nettai-content check` refuses a second of the same row), or add the one
   you need, named by what it does.
3. **Write the definitions.** `--!strict`; a header saying what it is, with the original's routine and object
   numbers; one local function per routine, commented with its name; the game's immediates as named constants.
   - A kind is `define.kind { id, pool, state, update }`. Its state is typed fields, references included
     (`"object"`, `"record:bomb-variant"`, `"chip"`, `"bool"`); what the original passed as spawn parameters is
     state the spawner sets. Export its spawner (`mark.spawn(owner, x, y, ticks)`, the game's `sub_80E7942`).
   - An action is `define.action { id?, state, update }`, usually from a family's builder: what differs
     between chips is the builder's arguments, and a variant several kinds read is a record (`define.record`).
     Anything a definition holds is in the canonical tree, so a thrower is `{ throw = fn, variant = record }`,
     not a bare closure. Where the original's routine took a number that picked one of several behaviours
     (the attack's variant byte, a spawn parameter), the builder takes the behaviour: a name
     (`counter.action_at(id, "random")`), a record, a look.
   - A chip is `define.chip { id, ...its record..., <one use> }`: `action`, `dimming`, `navi` or `instant`. Its
     record is named fields (flags by name, the lock-on mode a definition); what the ruleset asks of a chip
     beyond its record is a `trait` or a role, never the chip's key.
   - A weapon is `define.weapon { id, name, charge_ticks, setup }`: `setup(navi)` fills the attack (damage,
     hit parameter, element) and returns the action to start.
   - Assets by name: `asset.sprite("bomb")`, `asset.sound("sword-swing")`. The names are compat/assets.toml's;
     a placeholder such as `sprite-14-1b` is named first, with an entry in compat/curation.toml.
   - The API takes definitions: `battle.spawn(kind, pos)`, `me:setup_collision(collision.thrown,
     collision.hits_navis, 0)`, `battle.effect(pos, effects.explosion)`, `collision:set_hit_effect(sparks.erase)`,
     `me:set_attack(action, 2)`. A hitbox with no spark or status gives `nil`. An object the original gives a
     NameID takes an identity (`me.identity = IDENTITY`, a `define.identity` in its module).
   - Mind the game's store widths (`me.lifecycle = "destroy"` is `strb`, `me:set_lifecycle(...)` the word
     store), its sprite-stepping routine (`update_sprite`, `update_sprite_while_dimmed`,
     `update_sprite_while_paused`, `step_sprite`), and the order of RNG draws.
4. **Compat.** Every key the traces see must be compat's: a chip's (chips.toml), a kind's (kinds.toml, with
   `scratch_position` for a position the spawner's registers leave), an action's (actions.toml: a chip's
   action is `<chip>/action`, the key it gets by default; give `id` when no chip holds it). A number belongs to
   one key (actions may share one); a key or name you add goes into compat/curation.toml too, for review.
   Compat is edited by hand, as the modules are (nothing generates it any more); `gen-content check`
   compares every number in it with the ROM's (§5.2).
5. **Roles.** When the ruleset must start, spawn or show the thing itself (a counter, a kind, the chip a
   zeroed field reads), it is a role: the enum in crates/nettai-battle/src/content/roles.rs, its type in
   core.d.luau's `RolesSpec`, and its entry in rules/roles.luau.
6. **API.** When a script needs something the API lacks, add it: a `CoreApi` method
   (crates/nettai-content-api/src/api.rs, documented with the routine it is), its implementation
   (crates/nettai-battle/src/behavior/core_api.rs), its binding (crates/nettai-luau/src/bind.rs), and its declaration
   with a comment in content/bn6/core.d.luau (types.d.luau for the families' types). It takes definitions and
   names, not numbers: a new set of flags or states is an enum with names in the API and a string-literal type
   in core.d.luau, and gets a misuse case in nettai-content-check's type tests.
7. **Test in the repository** (§5.1) and **against the traces and the chip lab** (§5.2).
8. **Docs.** docs/engine describes the game; point its mentions of content at the module. How a family is
   composed goes in its module headers (`lib/<family>/`), and a pattern worth copying in §1's table.

## 4. Conventions

### 4.1 Names, not numbers

Content says what a thing is by name: a definition's key, a role, a flag's name, an asset's name. The
original's numbers stay where they are the routine's own data (an animation number, a tick count, a flags word
compared whole, a bug code), as named constants with the routine they come from. They don't name content: no
chip, kind, action, weapon, effect, spark, region, collision type, status, lock-on mode or identity is reached
by number, no folder or file is named with one, and a comment gives the original's number where it helps a
reader find the routine (`-- The wide shooter (the original's attachment row 0x23)`).

### 4.2 Indexing

Luau arrays start at 1; the game's tables start at 0. The rule for each case:

- **API accessors are numbered as the game numbers them.** A panel is (1..6, 1..3), a side 0 or 1, a hand's
  chip `battle.hand_turn(side, i)` with `i` from 0, an attach point `me:attach_point(n)` its number in the
  sprite, a sprite part `sprite:part_offset(n)` from 0, an element `collision:element_damage(element)` 0 to 5.
  What the routine has in a register is what the script passes.
- **Arrays the engine hands out or stores are Luau arrays, 1-based, with range errors.** A list an accessor
  returns (`battle.alive_actors(side)`, `field.objects(side)`) is a plain array. An array state field
  (`targets = "u8[18]"`) is `s.targets[1]` to `s.targets[18]`, with `#s.targets`; `s.targets[0]` and
  `s.targets[19]` are errors, not nil and not a wrap.
- **A table keyed by a game value is written with its keys**, zero included: `{ [0] = 0x0D880080, [1] =
  0x0E880080 }` by side, `{ [0] = raise, [4] = volleys, [8] = lower }` by step, `{ [0] = 0, [1] = 3, [2] = 1 }`
  by the count left. The lookup is `T[value]`, as the routine reads its table, and a missing key is a `nil` the
  script turns into the error the original would run into (`reads past off_80EE920`).
- **A list written in order is a Luau array, and a computed index into it is `+ 1`**: `RECOVERY[rapid + 1]`,
  `SINE[step + 1]`, `s.targets[i + 1]` where `i` counts from 0 as the game's does, `list[battle.rng_positive()
  % #list + 1]`. The `+ 1` sits at the index, never in the value the script keeps.

So a zero-based value from the game is never renumbered: it is passed to the API as it is, used as a key as it
is, or shifted by one at the point it indexes a list.

### 4.3 Types

Every module is `--!strict`. Declare a kind's or action's state type next to its schema (`export type State =
{ timer: number }`, `state = { timer = "u16" }`) and cast (`local s = me.state :: State`).

The checker checks each module on its own, and `require` gives `any`. So:

- a type two modules name (a spec a spawner takes, a variant record) is declared once in types.d.luau
  (`HeatFlame`, `AttachmentLook`, `ProjectileVariant`), and the module aliases it if it likes (`type Spec =
  HeatFlame`);
- a module-level table constant that is passed to a function carries its type: `local FLAME: HeatFlame = {
  while_dimmed = true, ticks = 0x1E, is = collision.attack }`, `local PHASES: { [number]: (me: Object, s:
  State) -> () } = { [0] = call, [4] = recover }`. Without it the literal is unsealed and passes for any record
  whose required fields it has (a misspelled optional field isn't an error), and across a `require` nothing is
  checked at all. `nettai-content-check` requires the annotation;
- a literal written inline in a call to a definer or a typed function of the same module is checked there.

### 4.4 State and time

Functions keep nothing between calls: no globals, no module-level variables that change, no tables that fill up
(the loader refuses the first two and freezes the third). What an object or action keeps is its declared state
or the engine's fields. Timers count as the routine's do: keep the game's compare (`t < 0` after the store
wraps, `bgt` against `bge`) rather than a cleaned-up count, since the tick it ends on is what the traces
compare.

### 4.5 Comments

A comment says what the code does and names the original's routine (`sub_80EE996`) or table (`byte_80EBB64`)
it ports, from the disassembly or docs/engine. Don't invent a routine name; if you haven't found it, describe
it. Where the port departs from the original (a register's garbage, a table read past its end, a parameter
nothing sets), say what the original does and why the port differs.

## 5. Testing

### 5.1 In the repository

In-repo tests never load game data. The test content (crates/nettai-battle/src/content/testing.rs) is its own
modules (crates/nettai-battle/testdata/content: the test chips, navis, stages, statuses, lock-on modes and roles)
plus content/bn6's modules, which it reads from the repository at test time, on made-up assets:

- add the modules to `scripts()`'s list (what they `require` comes with them); the asset names they use
  resolve to made-up assets unless `numbered_assets()` gives one the number a test looks at; the sprites they
  load get short animations in `animations()`;
- a chip is in the test content by its key: BN6's own by its module (`testing::chip_handle(testing::AREA_GRAB)`),
  or a test chip of made-up data composing BN6's builders (testdata/content/chips/test/chips.luau). Folders hold
  it by handle (`scenario::setup_with`); a test uses it with `use_chip` or `use_instant_chip`;
- test it: `behavior/tests.rs` plays duels (`duel_with`, `scenario::record_on`) and checks the kinds appear and
  roll back (`scripted_chips_roll_back`); `kinds/player/actions/tests.rs` runs one navi's action tick by tick
  and checks its timeline;
- `battles_run_the_content_scripts` lists the test content's kinds by key: update it.

Then:

```sh
cargo run -p nettai-content-check -- content/bn6     # every module type-checks; the lints
cargo build --workspace --all-targets             # no warnings
cargo test --workspace                            # the engine, the rollback tests, the type check, the lints, the guards
```

### 5.2 Against the traces and the chip lab

The golden traces and the chip lab are in the verification workspace, a separate checkout that builds this
repository's crates by path. Extract a pack from your checkout and run the workspace's tests against your
checkout on it:

```sh
cargo run --release -p bn6-extract -- content <falzar-rom> <gregar-rom> <pack>
<verification>/tools/gen-content-against.sh <checkout> check
BN6_PACK=<pack> <verification>/tools/traces-against.sh <checkout> --release
BN6_PACK=<pack> <verification>/tools/traces-against.sh <checkout> --release --test lab -- --ignored
```

`gen-content check` compares compat's numbers and what the definitions build with the ROM. The traces match
every frame (machgun 1074 and 1331 frames, soundmod 21962, 14933 and 20436), also through rollback at latencies
0+0 to 10+3, and their sound calls match the original's (the workspace's sound-tests, which traces-against.sh
runs too); every recorded scenario of the chip lab matches every frame: any difference is a regression. A scenario for
new content is recorded with the workspace's chiplab (tools/chiplab/README.md there).

## 6. What isn't there

- **Roles nothing fills**: `actions.volley`, what the navi's volley request starts (the original's action 0x30
  on whatever the attack's parameter bytes hold; no routine raises the request); and `kinds.mode9_attack` and
  `kinds.mode9_actor`, the two objects a player whose AI index is 10 spawns in battle mode 9 (the original's
  attack object #0xD2 and actor object #0x28). No netbattle reaches them; starting one is an error naming the
  role.
- **Numbers that name content**: none. Navis and forms are definitions by handle (the ruleset asks their fields
  and traits), the body overlays are the identities' `parts`, and nothing in the API or the content is reached by
  one of the original's numbers. The define phase refuses a `legacy` field, and the guards refuse a numbered
  folder or a `legacy` marker anywhere (§5.1). The original's numbers are compat's, for the tools outside the
  engine.
- **Bug codes** (a hitbox's `bug`, a projectile variant's) are numbers: they have no definition yet.
