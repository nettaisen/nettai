# Scripting battle content

Chips, weapons, navi-chip navis and the object kinds they spawn are content: Luau scripts in the content pack,
next to the data they implement, running on a typed content API. The engine is a content-independent core (object
pools, collision, the damage pipeline, statuses, battle flow, rollback) plus the BN6 ruleset's frameworks in Rust
(the navi framework, movement, the custom screen, and the services content calls: dimming, form changes, the
navi-chip controller, the Beast rush wrapper, the one-tick hitbox, effects and sparks, obstacles, the intro).
docs/design/core-content-boundary.md draws that line; docs/design/rollback.md covers rollback netplay;
docs/design/content-pack.md the pack's files; docs/design/content-migration.md how to move the rest of the content.

This document is the runtime's design: what content is written in, how it registers and loads, the API it sees,
and how it stays frame-exact, deterministic and cheap to snapshot under rollback. It began as a prototype that
built GunDelSol twice (Luau and Rust against the same API) to choose a runtime; §3 keeps the comparison. Luau was
chosen, and it is now the only runtime.

"Tick" is one call of `Battle::tick`. Routine names are the original's. Measurements were taken on an Apple M1 Max
in release builds, on a machine shared with other work, so single runs vary by up to 2x; figures are the best of
several runs.

## 0. Summary

- **Luau is the only runtime**, through a narrow typed API (`CoreApi`), with stateless scripts over engine-owned
  state. There is no build feature for it and no Rust version of anything a script implements.
- **Scripts live in the pack** beside their data (`objects/grab-shot/grab_shot.luau`, `chips/00f-gundels1/
  chip.luau`, `navis/00-megaman/weapons/02-blank-shot/blank_shot.luau`, `lib/buster.luau`). The data registers
  them: an object folder's `[kind]`, a chip's `script`, a weapon's `weapon.toml`. The engine loads them from the
  `Content` it runs on (`Content::scripts`); nothing is compiled in and nothing in the engine names a script. The
  content hash covers them. BN6's scripts are this repository's source overlay (content/bn6), which `bn6-extract
  content` merges into the pack.
- **Fidelity.** With GunDelSol, AreaGrab and PanelGrab, EraseMan, MegaMan's buster weapons, the blank shot and
  DustCross's charged shot as scripts, every golden trace matches as far as it did with the Rust: machgun 1074/1074
  and 1331/1331, soundmod 4513/6284/2566, also under rollback at every tested latency; the sound calls match.
- **Rollback.** Content declares its state; the engine stores it inside `Battle` (64 bytes per object or action),
  so snapshots and the digest cover it unchanged. The VM holds no battle state: rejected at load (writes to
  globals and module locals), frozen at run time (module tables and data).
- **Cost.** A 10-frame rollback every rendered frame costs 56 to 110 µs per frame on the golden-trace rounds
  (0.3 to 0.7% of a 16.7 ms frame), against 51 to 56 µs with everything in Rust (§7.3).
- **Typing.** `core.d.luau` types the whole API for luau-lsp and for an in-process type check
  (`bn6-content-check`) that runs in `cargo test`.

## 1. Where it lives

| Where | What |
|---|---|
| crates/bn6-content-api | The contract. `CoreApi`: what content can see and do. `ContentHost`: what a runtime provides (a manifest; update functions for kinds and actions; hook calls). `Registrations` (what a pack registers) and `Manifest` (what loaded). Typed content state: `Schema`, `ContentState`, `FieldType`. `Data` (the pack's data as the scripts see it). The shared value types (`ObjectRef`, `Vec3`, `PanelPos`, `Pool`, `SpriteId`), which the engine re-exports. |
| crates/bn6-battle/src/behavior | The engine side: `impl CoreApi for Battle` (`core_api`), dispatch of object kinds, actions and hooks to the runtime (`Behaviors`), and the scripts' `data` built from the battle's `Content` (`data`). |
| crates/bn6-battle/src/content/scripts.rs | `Scripts` (the pack's modules), `ObjectKind`, `WeaponData`, and `Content::registrations`: what the data registers. |
| crates/bn6-luau | The runtime: the VM and freezing (`sandbox`), the bytecode check (`verify`), the API binding (`bind`), module loading. |
| crates/bn6-content | Reads and writes packs, scripts included; `overlay` reads a source overlay and merges it into extracted content. |
| crates/bn6-content-check | Type-checks a pack's (or overlay's) Luau against its definitions with Luau's analysis, in process. |
| content/bn6 | BN6's scripts: the source overlay, laid out like a pack (`chips/`, `objects/`, `navis/00-megaman/weapons/`, `lib/`), with `core.d.luau` (the API) and `types.d.luau` (types the modules share). |
| crates/bn6-battle/src/content/testing.rs | The hand-written test content; its scripts are content/bn6's, read from the repository (§5.4). |
| crates/bn6-netplay | Rollback tests on the test content; `examples/rollback_cost` measures a golden-trace round. |
| crates/bn6-battle/examples | `content_bench` (duel and snapshot costs), `luau_ops` (cost per API operation). |

Running it:

```sh
cargo test --workspace                                         # engine, runtime, rollback, the type check
cargo run -p bn6-content-check -- content/bn6                  # type-check the overlay
cargo run --release -p bn6-extract -- content <rom> <pack>     # a BN6 pack, with the overlay's scripts
cargo run --release -p bn6-netplay --example rollback_cost --features trace -- <trace.jsonl> <pack> 1
cargo run --release -p bn6-battle --example luau_ops --features test-content
```

`luau-jit` (optional) enables Luau's native code generation where it is supported (§3.1).

## 2. The content API

### 2.1 Shape

`CoreApi` is a trait the engine implements and runtimes call through `&mut dyn CoreApi`. The runtime lives in its
own crate, which can't depend on the engine because the engine depends on it, so the contract sits below both.

Conventions:

- **Handles are raw slots.** `ObjectRef { pool, slot }`, not generational. A handle can outlive its object and
  then names the slot's next occupant, as the game's pointers do.
- **Effects are immediate.** Every call changes engine state at once. An object spawned by content runs later in
  the same tick, right after its spawner, as the engine's own spawns do; a hit resolves when the later of two
  objects removes its collision. Nothing is batched.
- **Fields are named and typed.** Engine fields are enums with a name, a type and a writability. Content-declared
  fields go through a `Schema`. A value crosses the API as a `Value` (`Nil`, `Bool`, `Int(i64)`, `Object`, `Vec3`)
  and is converted by the field's type on store: integers wrap to the width, as the game's `strb`/`strh`/`str` do;
  anything else of the wrong kind is an error.
- **Names, not numbers.** Status flags, status timers, navi requests and state bits, buttons, panel types,
  lifecycle states and shadows are names (`"using_action"`, `"paralyze"`, `"a"`, `"cracked"`, `"destroy"`); the
  engine's bit values and offsets stay in the engine. The original's numbers that are the game's own data (sound
  ids, animation numbers, collision types, hit regions, panel flag words it matches as whole words) are the
  scripts' named constants.
- **No floats anywhere.** `Vec3` is three `i32` in 16.16 fixed point with wrapping `+` and `-`.
- **Faithful stores.** Where the game's store width matters, the API has both: `me:set_lifecycle("destroy")` is
  the word store (state, action, phase and phase-init: the game's `str CUR_STATE_DESTROY`), `me.lifecycle =
  "destroy"` the byte store that keeps the action and phase (`strb`). The sprite-stepping routines are distinct
  calls (`update_sprite`, `update_sprite_while_dimmed`, `update_sprite_while_paused`, `step_sprite`), each gated
  as its routine is.

### 2.2 What it covers

core.d.luau is the reference; in outline:

- **Objects**: spawn (by pool and index, or a content kind by name), free, `destroy`
  (`object_genericDestroy`), lifecycle, action and phase, params; the header (panel, future panel, side, flip,
  facing, animation, element, timers, HP, damage word, position, velocity, related objects, flags); coordinates
  from panels and back; panel reservations; step checks; status flags and timers; collision (create, set-up,
  present, remove, the hit spark, the region, status base, hit effect, results).
- **Sprites**: load, animation, stepping, look (palette, flip, shadow, white, shader, alpha, mosaic, priority,
  hidden parts), frame flags, attach points.
- **Navis**: the attack in progress (step, step-init, variant, chip, element, damage, hit parameter, charged,
  lockout, bonus, parameters, marker), requests and state bits, the buttons (held, pressed, released, and the
  record kept while dimmed), the counter window, the reactive abort, `exit_attack`/`end_attack`/`set_attack`,
  the held direction, step targets and `start_move`, the buster's damage, absorbed obstacles, the actor data
  (type, AI index, overlay, links, charge, the weapon routines).
- **The battle** (`battle`): dimmed, paused, over, time up, mode, sound cues, the side's navi stats and emotion,
  players and alive actors, the simulation RNG, the chip hand, the defensive-chip record, the custom gauge, the
  primitives content spawns (`effect`, `hitbox`, `spark`).
- **The field** (`field`): validity, centers, flags words and checks, panels and columns, alliance and column
  timers, types, cracking, solidity, highlights.
- **Services** the ruleset provides: `dimming` (the steps of a dimming chip's controller: begin, dim, the telop and
  the wait for a cut-in, AntiNavi, undim, finish, hiding and showing a navi chip's user) and `navi_chip` (a navi
  chip's navi reports it is done).
- **`int`**: width wrappers, 32-bit shifts, truncating division, wrapping product. **`data`**: the pack (§5.2).

Nothing in the API names a chip or a kind. What it lacks today is added as content needs it
(content-migration.md §3): in `CoreApi` and the engine's `core_api.rs`, in the binding (`bind.rs`), and typed and
documented in core.d.luau.

### 2.3 Registration and dispatch

What a pack registers (content-pack.md §1.3) becomes `Registrations`: object kinds by (pool, index), navi actions
by number, and hooks. A module exports what its registration needs:

| Registered by | What | The module exports |
|---|---|---|
| `objects/KIND/object.toml` `[kind]` | an object kind (pool, index) | `state` (optional), `update(me)` |
| a chip's `script`, action other than 0x15, 0x1B and 0x1C | the chip's action | `state`, `update(me, s)` |
| a chip's `script`, action 0x15 | `Hook::DimmingChip(subtype)`: the dimming controller (`off_802CCB4[subtype]`) | `dimming_chip(user, spec) -> Object?` |
| a chip's `script`, action 0x1B | `Hook::NaviChip(subtype)`: the chip's navi (`off_802CD5C[subtype]`) | `navi_chip(user, controller, spec) -> Object?` |
| a chip's `script`, action 0x1C; a weapon's `instant_chip` | `Hook::InstantChip(subtype)`: the instant chip's effect (`off_80EC3F0[subtype]`) | `instant_chip(user, spec)` |
| a weapon's `weapon.toml` | `Hook::Weapon(id)`: the routine (`off_80117D4[id]`), and its `action` if any | `setup(navi) -> action`, and `state`/`update` for the action |

`Content::registrations` builds the table from the data; `Registrations::validate` refuses a slot, action or hook
claimed by two modules (chips sharing an action or a subtype must name the same module); loading refuses a module
without the function. `behavior::Behaviors` holds the runtime and lookup tables (objects by pool and index,
actions and each hook table by number). `kinds::update`, `actions::dispatch` and the ruleset's hook sites (the
weapon routine, the dimming chip action, the navi chip controller) consult them first; what the pack doesn't
register runs as the engine's own Rust. A kind keeps its original (pool, index) as its identity, so traces and the
frontend see the same objects.

Rust that needs a content kind (no engine code today; the benchmarks do) spawns it by name with
`behavior::spawn_kind` and sets its state by field name (`set_state_field`, `set_state_variant`), so a kind has one
implementation.

### 2.4 Content state: schemas the engine owns

Each object kind and action declares its state as named, typed fields:

```luau
state = { slot = slot.TYPE, offset_x = "i32", offset_z = "i32", lift = "i8" }
```

Field types are `bool`, `u8`...`i32`, `object`, `vec3`, an enum (a list of names) and fixed arrays (`"u8[18]"`).
The engine stores a `ContentState`: the schema's id and 64 bytes the fields are packed into in name order (the
layout is private to the store; fields have names and types, never offsets). It is a `Copy` value kept in
`kinds::Vars::Content` for objects and `ActionVars::Content` for actions, so `Battle: Clone` is still the whole
snapshot and `#[derive(Hash)]` still covers it for the digest. A script sees it as `me.state` (objects) or its
update's second argument (actions), and casts it to its declared type (`local s = me.state :: State`).

Two properties of the game carry over:

- **The attack scratch outlives the action.** An action's state stays in the actor's attack state after the
  action ends; the next action with the same schema continues from it, a different one starts from zero. Beast
  Out's rush wraps the scripted GunDelSol in machgun round 2 and the trace matches.
- **Content reaches into other objects.** GunDelSol steps its gun's animation (`gun.anim += 1`); the attachment
  copies its owner's sprite look; EraseMan's slash reads its navi's action. All are plain field access on another
  handle.

### 2.5 Register garbage

Some objects spawn with their spawner's registers as their position, which their init overwrites or partly keeps
(a Z whose fraction survives). The port spawns them with what the registers held where that is knowable and
observable (EraseMan's marks at Z 1), and otherwise declares it: `scratch_position = true` or `scratch_z_fraction =
true` in the kind's `[kind]`, which the trace comparison reads to skip the garbage.

## 3. The runtime

### 3.1 Luau

Luau is Roblox's Lua dialect: sandboxing built in, a gradual type system with an LSP, a fast interpreter and native
code generation. It is embedded through `mlua` (0.12, feature `luau`, Luau 0.736).

**Numbers.** Luau numbers are IEEE doubles; there is no integer subtype. Doubles are exact for integers up to 2^53,
far beyond the engine's 32-bit values, and `+ - *` and `//` on such integers are exact on every IEEE machine. So
scripts compute with plain numbers and the engine applies the game's rules at the boundary:

- every number entering the engine must be an integer: `s.timer = 13 / 2` fails with "6.5 is not an integer";
- a store wraps to the field's width: `s.timer = t` with `t = -1` stores 0xFFFF into a `u16`, and the script keeps
  testing its local `t < 0`;
- `Vec3` is a Rust value (three `i32`) with wrapping `+`/`-`, so fixed-point positions can't drift;
- `int` covers what plain arithmetic can't: `tdiv`/`tmod` (truncating, the ARM's), `asr`/`shl`/`lsr` (32-bit
  shifts), `mul32`, and width wrappers (`int.i32(x)`); `bit32` gives bitwise operations (u32 results).

The rule a modder learns is one line: use `//` or `int`, never `/`.

**Determinism.** Beyond the integer rule:

- `math` keeps only exact functions (`abs`, `ceil`, `floor`, `max`, `min`, `clamp`, `sign`, `round`). No `random`
  (the RNG is the engine's), no `noise`, no transcendental functions (C libraries disagree in the last bits).
- Luau's compiler folds calls to known builtins with constant arguments and, under `safeenv`, calls builtins
  directly (`FASTCALL`). Removing `math.sin` from the environment doesn't stop `math.sin(1)`, so the removed
  functions (and `setmetatable`, so calls reach the guarded version) are also disabled builtins for the compiler.
- No `os`, `io`, `debug`, `coroutine` (a suspended coroutine is hidden state), `buffer`, `utf8`, `vector`; no
  `collectgarbage`/`gcinfo`, `getfenv`/`setfenv`, `loadstring`, `newproxy`.
- Weak tables (`__mode`) are refused: their contents depend on when the GC ran.
- Tables keyed by strings iterate deterministically (Luau's string hash is unseeded); tables keyed by handles or
  tables iterate in address order. The API hands out no such collections, so this only matters for tables a
  script builds itself: iterate arrays.
- Runaway scripts stop on a budget of interrupt checks (calls, returns, loop back-edges): a count, not a clock, so
  every machine stops at the same point. Hitting it is a content error.
- Native code (`luau-jit`) gives bit-identical battles; it doesn't pay off for this content (§7), so the
  interpreter is the default.

**Statelessness, enforced.** A script function could keep state in three places, and each is closed:

1. *Globals.* The loader's bytecode check (`verify`) rejects any `SETGLOBAL`; modules export through their return
   value. After loading, the global table is read-only as well.
2. *Module-level locals captured by functions* (`local hums = 0` at the top, `hums += 1` in `update`). The check
   follows each closure's `CAPTURE` chain back to the main chunk and rejects any `SETUPVAL` that writes a module
   local. Writing a local of an enclosing *function* is allowed (that frame dies with the call). The check reads
   Luau bytecode as `lvmload.cpp` does and fails closed on anything it doesn't understand.
3. *Tables reachable from a module* (`local seen = {}`, then `seen[1] = x`). Everything a module returns or its
   functions capture is deep-frozen after it loads, so the write fails at run time with "attempt to modify a
   readonly table". The `data` tables are frozen the same way.

`require` works only while modules load (paths relative to the requiring file), so dependencies are static.
Between calls the VM holds only frozen code and data, and nothing in it can differ between machines or between a
run and its rollback. The tests (`behavior::tests`) show each rule fires, and that the VM carries nothing: a battle
moved to a fresh VM halfway continues identically; a second battle interleaved on the same VM changes nothing,
even when its updates panic inside a Luau call; a full GC after every call changes nothing. bn6-netplay's
`state_outside_the_snapshot_is_caught` shows what a leak would do (the peers diverge within a few hundred frames).

**Typing.** `content/bn6/core.d.luau` declares the API; `types.d.luau` the types the pack's modules share. Scripts
are `--!strict` and declare their state types (`export type State = { timer: number }`). `bn6-content-check`
type-checks every module with Luau's own analysis (the `luau-analyze` crate, new solver, in process), and its test
also checks that API misuse (a misspelled field, a lifecycle state or status flag or button that doesn't exist,
`Vec3 + number`, an unknown pool or panel type, a data field that isn't there) is a type error. An editor with
luau-lsp and both definition files gets completion and the same errors, with `require` resolved across modules.

The checker bundles its own Luau (0.710), whose C++ symbols collide with mlua's (0.736), so it lives in its own
crate and must never share a binary with bn6-luau.

The state schema (`state = { timer = "u16" }`) and the Luau type (`State = { timer: number }`) are written twice. A
generator could emit one from the other.

### 3.2 The alternatives

| | Luau (mlua) | Rust content (removed) | WASM (not built) | Rhai (not built) | Data timelines (not built) |
|---|---|---|---|---|---|
| Integers | Doubles, exact to 2^53; wrap on store | Native | Native i32/i64 | i64, checked | n/a |
| Snapshot | VM not included; checked stateless | Nothing to snapshot | Linear memory, or stateless | Stateless by design | Nothing to snapshot |
| Sandbox | Designed for it (Roblox) | None (trusted code) | Strongest | Good | Total |
| Hot reload | Yes | No (rebuild) | Yes | Yes | Yes |
| Authoring | Small, typed, familiar; luau-lsp | Rust toolchain | Rust toolchain plus wasm | Unfamiliar, weak tooling | Easy for simple chips only |

The prototype also ran GunDelSol as Rust against the same API (it matched every trace at about 1.5 times the cost
of built-in code). It was removed when Luau became the runtime: the engine's own Rust is for the core and the
ruleset's frameworks, and content has one implementation. WASM remains the fallback if native-speed sandboxed
content is ever needed. Many chips are "after N ticks spawn X; hit pattern P for M ticks"; the better form for them
is a Luau library of combinators (phases, waits, per-tick hits) rather than a second runtime.

Open Net Battle, the open Battle Network-style engine, scripts content in Lua with callbacks on engine objects and
per-object state in the Lua heap. That makes the heap part of the battle state (its network play is lockstep). The
design here keeps its authoring shape (a module per card or entity, an update function per kind) and moves the
state out of the VM, which is what rollback needs.

## 4. An example

EraseMan (content/bn6/objects/erase-man/erase_man.luau) is a navi chip's navi: an object kind (actor 0x15) whose
module also implements the EraseMan chips' `navi_chip` hook. The spawner and one of his actions:

```luau
-- `sub_80BB7F6`: EraseMan for `user`, reporting to `controller`.
function erase_man.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("erase-man", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.controller = controller
    return o
end

-- `sub_80BB710`: aim, switching every Param1 ticks, until the user
-- presses A while dimmed (`sub_80BB89C`), or for 360 ticks.
local function aim(me: Object)
    if me.phase == 0 then
        me.phase = 4;
        (me.state :: State).cycle = 3
        next_aim(me)
        mark(me)
        me.timer = AIM_TICKS
        me.timer2 = me:param(1)
        return
    end
    local user = me.related1 :: Object
    if user:dimmed_pressed("a") or ran_out(me) then
        me:set_action(0xC)
        return
    end
    local left = me.timer2 - 1
    me.timer2 = left
    if left > 0 then
        return
    end
    next_aim(me)
    mark(me)
    me.timer2 = me:param(1)
end
```

The three EraseMan chips' `chip.toml` say `script = "../../objects/erase-man/erase_man.luau"`; the kind's
`object.toml` says `[kind] pool = "actor", index = 0x15, script = "erase_man.luau"`. The navi chip controller (the
ruleset's, Rust) calls the hook for subtype 5 when the navi comes, and waits for `navi_chip.navi_left`.

The numbers that differ between entities are data: GunDelSol's gun, firing time and beam looks come from the chip
being used (`data.chips[me.chip].gun_del_sol`), the buster's recovery from `data.rules.buster_recovery`. A routine's
own immediates (EraseMan's sprite `"08-04"`, his 0x168-tick aim limit, his aim table) are the script's constants,
as they are the routine's in the game.

## 5. Loading

### 5.1 From the pack to a battle

A pack's modules are `Content::scripts` (source by module path); the entities' records carry what they register
(`ChipData::script`, `ObjectData::kinds`, `Content::weapons`). `Battle::new(setup, content)` gets the runtime with
`Behaviors::for_content(&content)`: it builds the registrations, loads the modules (compile, `verify`, run once,
freeze the result, look up the registered functions and schemas), and builds the lookup tables. Loading takes
about 5 ms.

The `Arc<Content>` is `Send + Sync` and mlua's `Lua` is not, so each thread that runs battles makes its own VM on
first use, cached by content hash (`for_content` keeps one per thread). Every VM made from the same content behaves
identically (the fresh-VM test in §3.1), so VMs are a cache, not part of any battle. `Battle::behaviors` is the
handle (an `Rc`); a snapshot copies the handle, and the digest leaves it out.

### 5.2 Data

Scripts read the pack as a frozen global, `data`, built from the `Content` when the modules load
(`behavior::data`): `data.chips[id]` (each chip's record with its own data), `data.navis`, `data.forms`,
`data.weapons`, `data.objects.{attachments, rocks, absorbed_sprites, body_overlays, sun_beam_looks, kinds}`,
`data.rules.buster_recovery`. Field names are as in the files, enums as names, sprites as `"CC-II"`, keyed by the
entities' ids. core.d.luau types the fields scripts read; a type for new data is added there when a script needs
it.

### 5.3 The content hash

`Content::hash` covers the battle data, the animation timing and the scripts (their paths and source). The round's
setup carries it (`RoundSetup::content`) and `Battle::new` checks it, so peers whose setups agree run the same data
and code. Pixels, palettes and audio are presentation and are left out.

### 5.4 The BN6 scripts and the tests

BN6's scripts are not ROM-derived: they are this project's port of the game's routines. They are the source
overlay content/bn6, laid out like a pack (content-pack.md §1.3), versioned and reviewed with the engine and
type-checked by `bn6-content-check` in `cargo test`. `bn6-extract content <rom> <pack>` writes the ROM-derived data
and assets, merges the overlay (`bn6_content::overlay`), and writes the pack, scripts included; the engine never
reads content/bn6.

In-repo tests can't use the ROM. The test content (`content::testing`) is small hand-written data whose records
register the overlay's modules (read from the repository at test time), so the engine's, netplay's and the
frontend's tests run the real scripts on made-up data: the GunDelSol duel, the eraser navi chip and the grab chip
(`behavior::tests`), the blank shot and DustCross's charged shot (`actions::tests`), and bn6-netplay's rollback
netbattles with the eraser navi chip in the folders. The frame-exact tests run in the verification workspace, on
an extracted pack.

## 6. Fidelity

Golden traces, replayed by the verification workspace on an extracted pack:

| Trace | All Rust (before) | Scripts |
|---|---|---|
| machgun round 1 | 1074/1074 | 1074/1074 |
| machgun round 2 | 1331/1331 | 1331/1331 |
| soundmod rounds 1/2/3 | 4513/6284/2566 | 4513/6284/2566 |

Soundmod stops where the engine does (navi chip navi 7 and action 0x39, not yet ported). The scripts it runs
before that: GunDelSol (machgun round 2 inside Beast Out's rush), the attachment and sun beam, AreaGrab's controller
and grab shots, EraseMan with his marks and slash, the buster's weapon routine with the NaviCust's blank (the blank
shot, round 1) and DustCross's charged shot with its junk ball (round 2). The sound calls match the original's
(machgun 53 calls over 1651 frames, soundmod 35 over 8596). Sabotaging a script breaks the trace where the script
runs.

## 7. Rollback and cost

### 7.1 What the scripting layer guarantees

rollback.md §8.2 lists what content must guarantee:

| Requirement | How |
|---|---|
| All content state in engine-owned typed storage inside `Battle`, snapshotted and digested; no VM heap, globals, closures or coroutines holding battle state | `ContentState` in `Vars`/`ActionVars` (`Hash`, `Copy`); the VM checked and frozen at load (§3.1); `Battle::digest` skips only the behaviors handle and the content, whose hash the round's setup carries |
| Integers only; one simulation RNG; no hash-map iteration | Integer-only boundary with wrapping stores; no `math.random`; the API hands out no maps |
| Outputs write-only | `battle.play_sound` only adds a cue; scripts can't read cues back |
| Content immutable during a battle, identified by a hash both peers compare | Modules frozen after load; `Content::hash` covers the scripts |
| Bounded work that fails the same way on every peer | The interrupt budget counts VM checkpoints, not time |
| No perspective in simulated state | `battle.local_side` is for presentation only; sound goes to both sides' cue lists (or one side's player's, `play_sound_for`) |
| Re-running a tick free of side effects outside `Battle` | Scripts can't reach the host; the VM keeps nothing |

### 7.2 Results

- **Golden traces through two rollback peers** (the verification workspace's rollback test, latencies 0, 2+1, 5+2
  and 10+3 frames): every confirmed frame of machgun rounds 1 and 2 and soundmod rounds 1 to 3 matches the trace on
  both peers.
- **Synthetic netbattles** (bn6-netplay's tests): two navis of the test content mashing buttons with its chips
  (GunDelSol, the invisibility dimming chip, the eraser navi chip), several seeds, latencies 0 to 10 with jitter and
  input delay: in sync to the KO in every configuration; sound plays each confirmed cue once.
- **In-repo**: `scripted_chips_roll_back` copies a battle every 97 ticks of a duel with the eraser, grab and
  GunDelSol chips and checks the copy plays on exactly as the battle does.

### 7.3 Cost

The worst case, a 10-frame rollback on every rendered frame, measured as the verification workspace's
`soundmod_rollback_cost` does (restore, 11 advances each followed by a save, a digest), with
`bn6-netplay/examples/rollback_cost`, best of three runs, per rendered frame:

| Round (frames measured) | All Rust | GunDelSol slice as Luau (before) | Scripts now |
|---|---|---|---|
| machgun 1 (10-1073; three GunDelSols) | 50.9 µs | 146.9 µs | 109.5 µs |
| machgun 2 (129-1330; GunDelSol in Beast Out) | 51.0 µs | 110.9 µs | 86.9 µs |
| soundmod 1 (2525-4512; EraseMan, the blank shot, AreaGrab) | 56.3 µs | 55.7 µs | 56.5 µs |
| soundmod 2 (5153-6283) | 53.3 µs | 57.1 µs | 65.3 µs |
| soundmod 3 (1525-2565) | 53.2 µs | 56.5 µs | 65.7 µs |

The frame budget is 16,667 µs; the worst case is 0.7% of it. The part every runtime shares: a restore (3 µs), a
save per advance (2 µs) and the digest (19 µs). The one-tick hitbox, which GunDelSol spawns every firing tick, is
the engine's again (the prototype ran it as a script). 99th-percentile frames are 0.2 to 0.5 ms.

## 8. Performance

Where the time goes (`examples/luau_ops`, interpreted):

| Operation | Cost |
|---|---|
| Loop iteration (the VM itself) | 12 ns |
| Library call `battle.dimmed()` | 38 ns |
| `field.flags(3, 2)` | 62 ns |
| Method `me:param(1)` | 93 ns |
| Field read `me.anim` / write | 131 / 155 ns |
| Content state read `s.ticks` / write | 138 / 141 ns |
| Enum state read `s.slot` | 234 ns |
| A field that makes a handle or a `Vec3` (`me.pos`, `me.related1`, `me.sprite`) | 229 to 244 ns |
| `Vec3.new(1, 2, 3)` | 199 ns |
| A data read `data.objects.sun_beam_looks[0]` | 24 ns |
| A 9-field table literal (a hitbox spec) | 92 ns |

The VM is fast; the binding isn't. mlua dispatches every userdata field through a Lua-side `__index` closure, then
its generic callback machinery (stack checks, argument conversion, a userdata type check). The next step is a
raw-FFI binding: tagged userdata (`lua_newuserdatatagged`) for handles, a `__namecall` that dispatches on Luau's
string atoms, and field access through a C `__index` that switches on the atom instead of calling a Lua closure.
That is how Roblox's own bindings work; 20 to 40 ns per call is a reasonable target. Native code only speeds up the
VM's share, which is why it doesn't help here.

## 9. Sandboxing

A pack can only call the API: no `io`, `os`, `debug`, no loading code at run time (`load`, `loadstring`, `require`
after loading), no FFI. Libraries and the global table are read-only (Luau's `sandbox`). Scripts can't reach the
host, can't keep state, can't hang the engine (the interrupt budget) and can't grow without bound unnoticed
(mlua's memory limit is available; a hit must be a fatal content error, since GC timing decides exactly when it
triggers). A content error (a script error, a type error at the API, the budget, a routine the game would run off
the end of a table) stops the battle with a message naming the module and object, identically on every peer; a
production engine would end the round with an error result rather than panic.

## 10. Next steps

1. Move the rest of the content (content-migration.md): the remaining chip actions, navi chips' navis, dimming
   chips, weapons and object kinds, each with the API it needs.
2. The raw-FFI binding (§8), before much more content moves: every scripted object costs its API calls every tick.
3. Versioning: `core.d.luau` carries the API version, the manifest records the version a pack targets, and the
   engine refuses a pack for a different major version.
4. A generator for state schemas from the Luau types, and a content error that ends the round instead of
   panicking.
