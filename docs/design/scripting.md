# Scripting battle content

Chips, weapons, navi-chip navis, the link navis' attacks and the object kinds they spawn are content: Luau
modules in the content root, running on a typed content API. The engine is a content-independent core (object
pools, collision, the damage pipeline, statuses, battle flow, rollback) plus the BN6 ruleset's frameworks in
Rust (the navi framework, movement, the custom screen, and the services content calls: dimming, form changes,
the navi-chip controller, the Beast rush wrapper, the one-tick hitbox, effects and sparks, obstacles, the
intro). docs/design/core-content-boundary.md draws that line; docs/design/rollback.md covers rollback netplay;
docs/design/content-pack.md the content root and the pack; docs/design/content-migration.md how to write
content.

This document is the runtime's design: what content is written in, how it loads and runs, the API it sees, and
how it stays frame-exact, deterministic and cheap to snapshot under rollback.

"Tick" is one call of `Battle::tick`. Routine names are the original's. Measurements were taken on an Apple M1
Max in release builds, on a machine shared with other work, so single runs vary by up to 2x.

## 0. Summary

- **Luau is the only runtime**, through a narrow typed API (`CoreApi`), with stateless scripts over engine-owned
  state. There is no build feature for it and no Rust version of anything a script implements.
- **Content is definitions.** A module makes them while it loads (`define.chip`, `define.kind`,
  `define.action`, ...) and a definition holds its functions (a kind's `update`, a chip's use, a weapon's
  `setup`). The engine loads the modules from the `Content` it runs on (`Content::scripts`); nothing is compiled
  in, nothing in the engine names a module, a chip or a kind, and nothing is registered by number. The content
  hash covers the scripts.
- **Definitions and assets cross the API, not numbers.** `battle.spawn(bomb.kind, pos)`,
  `me:set_attack(action, 2)`, `me:setup_collision(collision.thrown, collision.hits_navis, 0)`,
  `battle.play_sound(asset.sound("throw"))`. The engine keeps a handle for each.
- **Fidelity.** Every golden trace matches every frame, also under rollback at every tested latency; the sound
  calls match; the chip lab's scenarios match every frame (§5).
- **Rollback.** Content declares its state; the engine stores it inside `Battle` (64 bytes per object or
  action), so snapshots and the digest cover it unchanged. The VM holds no battle state: writes to globals and
  module locals are rejected at load, module tables and definitions are frozen at run time.
- **Typing.** `core.d.luau` types the whole API for luau-lsp and for an in-process type check
  (`nettai-content-check`) that runs in `cargo test`, with lints for what the checker can't see.

## 1. Where it lives

| Where | What |
|---|---|
| crates/nettai-content-api | The contract. `CoreApi`: what content can see and do. `ContentHost`: what a runtime provides (update functions for kinds and actions; hook calls). `BindPlan` (what a runtime binds: function slots, state layouts, the definitions it must read back) and `Manifest` (what loaded). `Definitions`, `Registry` and the handle types. Typed content state: `Schema`, `ContentState`, `FieldType`. The shared value types (`ObjectRef`, `Vec3`, `PanelPos`, `Pool`, `SpriteId`), which the engine re-exports. |
| crates/nettai-luau | The runtime: the VM and freezing (`sandbox`), the bytecode check (`verify`), the define phase (`define`), the API binding (`bind`), module loading. |
| crates/nettai-battle/src/behavior | The engine side: `impl CoreApi for Battle` (`core_api`), and dispatch of object kinds, actions and hooks to the runtime (`Behaviors`). |
| crates/nettai-battle/src/content | `Content`: the registries the definitions build (`defs`), the roles (`roles`), the typed records, and `Scripts` (the modules). |
| crates/nettai-content | Reads a content root and a pack (`root`, `pack`), reports on definitions (`lint`), reads and writes the pack's assets. |
| crates/nettai-content-check | Type-checks a content root's Luau against its declarations with Luau's analysis, in process; lints; the guards (§3.3). |
| crates/bn6-compat | The original's numbers by key (content/bn6/compat), the setup codecs and the trace harness. The engine doesn't depend on it. |
| content/bn6 | BN6's content, with `core.d.luau` (the API) and `types.d.luau` (types the modules share). |
| crates/nettai-battle/src/content/testing.rs, crates/nettai-battle/testdata | The test content and the test pack (content-pack.md §6). |
| crates/nettai-netplay | Rollback tests on the test content; `examples/rollback_cost` measures a golden-trace round. |
| crates/nettai-battle/examples | `content_bench` (duel and snapshot costs), `luau_ops` (cost per API operation). |

Running it:

```sh
cargo test --workspace                                         # engine, runtime, rollback, the type check, the guards
cargo run -p nettai-content-check -- content/bn6                  # type-check and lint the content root
cargo run --release -p bn6-extract -- content <rom> <pack>     # a BN6 pack (assets), checked against content/bn6
cargo run --release -p nettai-content -- check <pack>             # the define phase and its report
cargo run --release -p nettai-netplay --example rollback_cost -- <trace.jsonl> <pack> 1
cargo run --release -p nettai-battle --example luau_ops --features test-content
```

`luau-jit` (optional) enables Luau's native code generation where it is supported (§3.1).

## 2. The content API

### 2.1 Shape

`CoreApi` is a trait the engine implements and runtimes call through `&mut dyn CoreApi`. The runtime lives in
its own crate, which can't depend on the engine because the engine depends on it, so the contract sits below
both.

Conventions:

- **Handles are raw slots.** `ObjectRef { pool, slot }`, not generational. A handle can outlive its object and
  then names the slot's next occupant, as the game's pointers do.
- **Effects are immediate.** Every call changes engine state at once. An object spawned by content runs later
  in the same tick, right after its spawner, as the engine's own spawns do; a hit resolves when the later of
  two objects removes its collision. Nothing is batched.
- **Fields are named and typed.** Engine fields are enums with a name, a type and a writability.
  Content-declared fields go through a `Schema`. A value crosses the API as a `Value` (`Nil`, `Bool`,
  `Int(i64)`, `Object`, `Vec3`, `Def(registry, handle)`, `Asset(kind, handle)`) and is converted by the field's
  type on store: integers wrap to the width, as the game's `strb`/`strh`/`str` do; anything else of the wrong
  kind is an error.
- **Definitions, not numbers.** A kind, an action, a chip, a weapon, an effect, a spark, a region, a collision
  type, a status, a lock-on mode, an identity or a record crosses the API as its definition (the frozen table
  `define.*` returned), and an asset as the value `asset.*` returned. The binding turns each into its handle.
- **Names, not bits.** Status flags, status timers, navi requests and state bits, buttons, panel types,
  lifecycle states and shadows are names (`"using_action"`, `"paralyze"`, `"a"`, `"cracked"`, `"destroy"`); the
  engine's bit values and offsets stay in the engine. The original's numbers that are a routine's own
  immediates (animation numbers, tick counts, panel flag words it matches as whole words, bug codes) are the
  scripts' named constants.
- **No floats anywhere.** `Vec3` is three `i32` in 16.16 fixed point with wrapping `+` and `-`.
- **Faithful stores.** Where the game's store width matters, the API has both: `me:set_lifecycle("destroy")` is
  the word store (state, action, phase and phase-init: the game's `str CUR_STATE_DESTROY`), `me.lifecycle =
  "destroy"` the byte store that keeps the action and phase (`strb`). The sprite-stepping routines are distinct
  calls (`update_sprite`, `update_sprite_while_dimmed`, `update_sprite_while_paused`, `step_sprite`), each gated
  as its routine is.

### 2.2 What it covers

core.d.luau is the reference; in outline:

- **Objects**: spawn (a kind, where in the update list), free, `destroy` (`object_genericDestroy`), lifecycle,
  the object's own action and phase; the header (panel, future panel, side, flip, facing, animation, element,
  timers, HP, damage word, position, velocity, related objects, flags, identity); coordinates from panels and
  back; panel reservations; step checks; status flags and timers; collision (create, set-up, present, remove,
  the hit spark, the region, the status its hits carry, results).
- **Sprites**: load, animation, stepping, look (palette, flip, shadow, white, shader, alpha, mosaic, priority,
  hidden parts), frame flags, attach points.
- **Navis**: the attack in progress (step, step-init, chip, element, damage, hit parameter, charged, lockout,
  bonus, marker), requests and state bits, the buttons (held, pressed, released, and the record kept while
  dimmed), the counter window, the reactive abort, `exit_attack`/`end_attack`/`set_attack`, what it runs
  (`navi_action`), the held direction, step targets and `start_move`, the buster's damage, absorbed obstacles,
  the actor data (type, AI index, overlay, links, charge, the weapons).
- **The battle** (`battle`): dimmed, paused, over, time up, mode, sound cues, the side's navi stats and
  emotion, players and alive actors, the simulation RNG, the chip hand, the defensive-chip record, the custom
  gauge, the primitives content spawns (`effect`, `hitbox`, `spark`, `afterimage`, `form_overlay`,
  `palette_flash`).
- **The field** (`field`): validity, centers, flags words and checks, panels and columns, alliance and column
  timers, types, cracking, solidity, highlights.
- **Services** the ruleset provides: `dimming` (the steps of a dimming chip's controller: begin, dim, the telop
  and the wait for a cut-in, AntiNavi, undim, finish, hiding and showing a navi chip's user), `navi_chip` (a
  navi chip's navi reports it is done; the user's warp) and `obstacle` (the steps of a field obstacle's update,
  the registry, the requests chips make of obstacles).
- **`int`**: width wrappers, 32-bit shifts, truncating division, wrapping product. **`asset`** and **`define`**:
  while content loads (§2.3).

Nothing in the API names a particular chip or kind. What it lacks is added as content needs it
(content-migration.md §3): in `CoreApi` and the engine's `core_api.rs`, in the binding (`bind.rs`), and typed
and documented in core.d.luau.

### 2.3 Definitions and dispatch

A module makes definitions while it loads; each registry has a definer:

| Definer | What | What the engine runs or reads |
|---|---|---|
| `define.kind { id, pool, state?, update, place? }` | an object kind | `update(me)` each tick it runs; `place(spec)` when a stage names it |
| `define.action { id?, state?, update, traits? }` | a navi action | `update(me, s)` while the navi runs it |
| `define.chip { id, ...record..., action \| dimming \| navi \| instant }` | a chip and its one use | the action as the navi's attack, or the hook from the ruleset's dimming, navi chip or instant chip action |
| `define.weapon { id, name, charge_ticks, setup?, instant?, ... }` | what a button does | `setup(navi)` fills the attack and returns the action to start |
| `define.navi`, `define.form` | a navi, one of MegaMan's forms | their records: stats, weapons, sprite, identity |
| `define.stage { id, layout, actors, ... }` | a stage | its panels and what it places |
| `define.effect`, `define.spark`, `define.region`, `define.collision`, `define.status`, `define.lockon`, `define.identity` | what the engine's primitives are told by | their records, by handle |
| `define.rules(section, table)` | a rule section no entity owns | the ruleset's typed tables (elements, panels, the custom screen, ...) |
| `define.roles { ... }` | what the ruleset needs from content, by role | the action a request starts, the kind it spawns, the chip a zeroed field reads, ... |
| `define.record(type, table)` | data only content reads, with a handle | nothing: a state field or another definition holds it |

A definition's key is its `id` (a chip's action derives one: `minibomb/action`); two of one key in a registry is
an error. Keys are sorted byte-wise and a definition's handle is its place, so handles are the same on every
machine that loads the same content.

The define phase reads everything back as the canonical tree (`Definitions`): fields as data, references to
other definitions by key, functions as slots (`kind bomb`'s `update`). The engine builds its registries from it
and plans what the runtime binds (`BindPlan`): each function it will call, by definition and path, and each
state layout. `behavior::Behaviors` holds the runtime; `kinds::update` runs a content kind's `update`,
`actions::dispatch` a content action's, and the ruleset's hook sites (a weapon's setup, the dimming chip action,
the navi chip controller, the instant chip action, a stage's placements, the roles' hooks) call their slots.
What the ruleset implements itself (its own object kinds, keyed `engine/...`, and its own navi actions: the
step, the three chip-use actions, the form change, the Cross special) runs as Rust.

Rust that needs a content kind asks for it by role (`content.defs.roles.kind(KindRole::AbsorbedObstacle)`) and
sets its state by field name (`set_state_field`, `set_state_variant`), so a kind has one implementation. Tests
and tools spawn one by key (`behavior::spawn_kind`).

### 2.4 Content state: schemas the engine owns

Each object kind and action declares its state as named, typed fields:

```luau
state = { slot = slot.TYPE, look = "record:attachment-look", anim = "u8", offset_x = "i32", lift = "i8" }
```

Field types are `bool`, `u8`...`i32`, `object`, `vec3`, an enum (a list of names), fixed arrays (`"u8[18]"`),
and references: to a definition (`"kind"`, `"action"`, `"chip"`, `"effect"`, `"record:<type>"`, ...) or an
asset (`"sprite"`, `"sound"`, ...). The engine stores a `ContentState`: the schema's id and 64 bytes the fields
are packed into in name order (the layout is private to the store; fields have names and types, never offsets).
It is a `Copy` value kept in `kinds::Vars::Content` for objects and `ActionVars::Content` for actions, so
`Battle: Clone` is still the whole snapshot and `#[derive(Hash)]` still covers it for the digest. A script sees
it as `me.state` (objects) or its update's second argument (actions), and casts it to its declared type (`local
s = me.state :: State`). What the original passed an object as spawn parameters is state its spawner sets.

Two properties of the game carry over:

- **The attack scratch outlives the action.** An action's state stays in the actor's attack state after the
  action ends; the next action with the same state table continues from it, a different one starts from zero.
  A weapon's setup writes the state of the action it is about to start (`navi:action_state(action)`).
- **Content reaches into other objects.** GunDelSol steps its gun's animation; the attachment copies its
  owner's sprite look; a kind that lasts while its owner's action does compares `owner:navi_action()` with the
  action it was sent in. All are plain access on another handle.

### 2.5 Register garbage

Some objects spawn with their spawner's registers as their position, which their init overwrites or partly
keeps (a Z whose fraction survives). The port spawns them with what the registers held where that is knowable
and observable (EraseMan's marks at Z 1; `battle.loop_register()` for the update loop's leftover), and
otherwise the trace comparison skips the garbage: `scratch_position` or `scratch_z_fraction` on the kind's
entry in compat/kinds.toml. The engine knows nothing of it.

## 3. The runtime

### 3.1 Luau

Luau is Roblox's Lua dialect: sandboxing built in, a gradual type system with an LSP, a fast interpreter and
native code generation. It is embedded through `mlua` (feature `luau`).

**Numbers.** Luau numbers are IEEE doubles; there is no integer subtype. Doubles are exact for integers up to
2^53, far beyond the engine's 32-bit values, and `+ - *` and `//` on such integers are exact on every IEEE
machine. So scripts compute with plain numbers and the engine applies the game's rules at the boundary:

- every number entering the engine must be an integer: `s.timer = 13 / 2` fails with "6.5 is not an integer";
- a store wraps to the field's width: `s.timer = t` with `t = -1` stores 0xFFFF into a `u16`, and the script
  keeps testing its local `t < 0`;
- `Vec3` is a Rust value (three `i32`) with wrapping `+`/`-`, so fixed-point positions can't drift;
- `int` covers what plain arithmetic can't: `tdiv`/`tmod` (truncating, the ARM's), `asr`/`shl`/`lsr` (32-bit
  shifts), `mul32`, and width wrappers (`int.i32(x)`); `bit32` gives bitwise operations (u32 results).

The rule a modder learns is one line: use `//` or `int`, never `/`.

**Determinism.** Beyond the integer rule:

- `math` keeps only exact functions (`abs`, `ceil`, `floor`, `max`, `min`, `clamp`, `sign`, `round`). No
  `random` (the RNG is the engine's), no `noise`, no transcendental functions (C libraries disagree in the last
  bits).
- Luau's compiler folds calls to known builtins with constant arguments and, under `safeenv`, calls builtins
  directly (`FASTCALL`). Removing `math.sin` from the environment doesn't stop `math.sin(1)`, so the removed
  functions (and `setmetatable`, so calls reach the guarded version) are also disabled builtins for the
  compiler.
- No `os`, `io`, `debug`, `coroutine` (a suspended coroutine is hidden state), `buffer`, `utf8`, `vector`; no
  `collectgarbage`/`gcinfo`, `getfenv`/`setfenv`, `loadstring`, `newproxy`.
- Weak tables (`__mode`) are refused: their contents depend on when the GC ran.
- Tables keyed by strings iterate deterministically (Luau's string hash is unseeded); tables keyed by handles
  or tables iterate in address order. The API hands out no such collections, so this only matters for tables a
  script builds itself: iterate arrays.
- Runaway scripts stop on a budget of interrupt checks (calls, returns, loop back-edges): a count, not a clock,
  so every machine stops at the same point. Hitting it is a content error.
- Native code (`luau-jit`) gives bit-identical battles; it doesn't pay off for this content (§7), so the
  interpreter is the default.

**Statelessness, enforced.** A script function could keep state in three places, and each is closed:

1. *Globals.* The loader's bytecode check (`verify`) rejects any `SETGLOBAL`; modules export through their
   return value. After loading, the global table is read-only as well.
2. *Module-level locals captured by functions* (`local hums = 0` at the top, `hums += 1` in `update`). The
   check follows each closure's `CAPTURE` chain back to the main chunk and rejects any `SETUPVAL` that writes a
   module local. Writing a local of an enclosing *function* is allowed (that frame dies with the call). The
   check reads Luau bytecode as `lvmload.cpp` does and fails closed on anything it doesn't understand.
3. *Tables reachable from a module* (`local seen = {}`, then `seen[1] = x`). Everything a module returns or its
   functions capture is deep-frozen after it loads, and every definition is frozen when the define phase ends,
   so the write fails at run time with "attempt to modify a readonly table".

`require` and the definers work only while modules load (paths relative to the requiring file), so dependencies
and the set of definitions are static. Between calls the VM holds only frozen code and data, and nothing in it
can differ between machines or between a run and its rollback. The tests (`behavior::tests`) show each rule
fires, and that the VM carries nothing: a battle moved to a fresh VM halfway continues identically; a second
battle interleaved on the same VM changes nothing, even when its updates panic inside a Luau call; a full GC
after every call changes nothing. nettai-netplay's `state_outside_the_snapshot_is_caught` shows what a leak would
do (the peers diverge within a few hundred frames).

### 3.2 Loading

A content root's modules are `Content::scripts` (source by module path). `Content::define` runs the define
phase in a VM of its own, which is dropped: it loads every module (compile, `verify`, run once, freeze the
result), reads the definitions back, builds the registries and keeps the modules' bytecode.
`Battle::new(setup, content)` gets the runtime with `Behaviors::for_content(&content)`: it loads the modules
again from the bytecode, checks they define exactly what the content was made from, and binds the functions the
plan names.

The `Arc<Content>` is `Send + Sync` and mlua's `Lua` is not, so each thread that runs battles makes its own VM
on first use, cached by content hash (`for_content` keeps one per thread). Every VM made from the same content
behaves identically, so VMs are a cache, not part of any battle: `Battle` holds no handle to one, and a content
call uses the thread's runtime for the hash the round's setup carries (`behavior::with_runtime` picks another).

`Content::hash` covers the typed data, the animation timing and the scripts (their paths and source). The
round's setup carries it (`RoundSetup::content`) and `Battle::new` checks it, so peers whose setups agree run
the same data and code. Pixels, palettes and audio are presentation and are left out.

### 3.3 Typing and the checks

`content/bn6/core.d.luau` declares the API; `types.d.luau` the types the pack's modules share. Scripts are
`--!strict` and declare their state types (`export type State = { timer: number }`). `nettai-content-check`
type-checks every module with Luau's own analysis (the `luau-analyze` crate, in process), and its tests also
check that API misuse (a misspelled field, a lifecycle state or status flag or button that doesn't exist, `Vec3
+ number`, a number where a definition goes) is a type error.

The checker checks each module on its own, with `require` typed `any`: what crosses a `require` is unchecked.
So a type two modules both name is declared once in types.d.luau (`HeatFlame`, `AttachmentLook`), and the
caller annotates what it passes. An editor with luau-lsp and both definition files resolves `require` across
modules and checks those calls too.

The lints cover what the checker can't see (`nettai-content-check`'s `lints`):

- a module-level table constant passed to a function needs its type (`local FLAME: HeatFlame = { ... }`): an
  unannotated table literal is unsealed and passes for any record whose required fields it has, and another
  module's function takes anything;
- a placeholder asset name (`sprite-0c-01`): name the asset in compat/assets.toml first;
- a kind in an owner's folder is keyed under its owner (`minibomb/held`);
- no module lives under compat/.

`nettai-content check` adds what needs the definitions: roles left unfilled, kinds under `objects/` that one owner
alone uses, a collision type defined twice. Reading compat refuses two keys with one of the original's numbers.

Two tests guard that content names nothing by the original's numbers (`nettai-content-check`'s `guards`, with
no exceptions): no folder under the content roots is named with one (`00f-gundels1`), and no module carries a
`legacy` marker or field, the form the migration's numbers took. The define phase refuses a `legacy` field on a
definition too, and `legacy` is no global. A third guard, nettai-battle's `no_compat` test, keeps compat out of the
engine and the crates it runs content through (nettai-battle, nettai-content-api, nettai-luau): the word appears in
their code only in comments, and none of their manifests names it.

The checker bundles its own Luau, whose C++ symbols collide with mlua's, so it lives in its own crate and must
never share a binary with nettai-luau.

The state schema (`state = { timer = "u16" }`) and the Luau type (`State = { timer: number }`) are written
twice. A generator could emit one from the other.

### 3.4 Why Luau

| | Luau (mlua) | Rust content | WASM | Rhai | Data timelines |
|---|---|---|---|---|---|
| Integers | Doubles, exact to 2^53; wrap on store | Native | Native i32/i64 | i64, checked | n/a |
| Snapshot | VM not included; checked stateless | Nothing to snapshot | Linear memory, or stateless | Stateless by design | Nothing to snapshot |
| Sandbox | Designed for it (Roblox) | None (trusted code) | Strongest | Good | Total |
| Hot reload | Yes | No (rebuild) | Yes | Yes | Yes |
| Authoring | Small, typed, familiar; luau-lsp | Rust toolchain | Rust toolchain plus wasm | Unfamiliar, weak tooling | Easy for simple chips only |

Only Luau is built. A prototype ran GunDelSol as Rust against the same API (it matched every trace at about 1.5
times the cost of built-in code) and was removed: the engine's own Rust is for the core and the ruleset's
frameworks, and content has one implementation. WASM remains the fallback if native-speed sandboxed content is
ever needed. Many chips are "after N ticks spawn X; hit pattern P for M ticks"; the form for them is a Luau
library of builders (lib/bombs, lib/swords, lib/dimming) rather than a second runtime.

Open Net Battle, the open Battle Network-style engine, scripts content in Lua with callbacks on engine objects
and per-object state in the Lua heap. That makes the heap part of the battle state (its network play is
lockstep). The design here keeps its authoring shape (a module per card or entity, an update function per kind)
and moves the state out of the VM, which is what rollback needs.

## 4. An example

EraseMan (content/bn6/chips/eraseman) is a navi chip: a navi kind, a builder for the chips' `navi` hook, and the
three chips of the series, which compose the hook with their own arguments.

```luau
-- chips/eraseman/navi.luau

eraseman.kind = define.kind {
    id = "eraseman/navi",
    pool = "actor",
    state = { cycle = "u8", aim = "u8", controller = "object", aim_ticks = "u8" },
    -- `sub_80BB608`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("EraseMan action %#x reads past his table", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- A chip's `navi` hook (`sub_80BB7F6`): EraseMan for the user, on the
-- panel the controller picked, reporting to `controller`.
function eraseman.summon(args: Summon): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = battle.spawn(eraseman.kind, Vec3.zero)
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        o.element = spec.element
        o.related1 = user
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(spec.damage)
        local s = o.state :: State
        s.controller = controller
        s.aim_ticks = args.aim_ticks
        return o
    end
end
```

```luau
-- chips/eraseman/chips.luau
local navi = require("./navi")

local eraseman = define.chip {
    id = "eraseman",
    name = "EraseMan",
    codes = { "K", "*" },
    class = "mega",
    damage = 120,
    hit_param = 138,
    flags = { "dimming", "has_damage", "navi", "library" },
    -- ...the rest of its record...
    navi = navi.summon { aim_ticks = 20 },
}
```

The navi chip controller (the ruleset's, Rust) calls the chip's hook when the navi comes, and waits for
`navi_chip.navi_left(controller)`. What differs between the chips of the series is the hook's argument and the
record; a routine's own immediates (his 0x168-tick aim limit, his aim table) are the script's constants, as
they are the routine's in the game.

## 5. Fidelity

The verification workspace replays, on an extracted pack:

- **the golden traces**: machgun rounds 1 and 2 (1074 and 1331 frames) and soundmod rounds 1 to 3 (21962, 14933
  and 20436 frames) match every frame, and through two rollback peers at latencies 0+0, 2+1, 5+2 and 10+3
  every confirmed frame matches on both;
- **the sound calls** of those recordings match the original's;
- **the chip lab**: 5022 scripted netbattles recorded from the original (every chip, Program Advance, Cross,
  Beast Out, link navi and NaviCust program), 3,782,824 frames, match every frame.

Sabotaging a script breaks the trace where the script runs.

## 6. Rollback and cost

### 6.1 What the scripting layer guarantees

rollback.md §8.2 lists what content must guarantee:

| Requirement | How |
|---|---|
| All content state in engine-owned typed storage inside `Battle`, snapshotted and digested; no VM heap, globals, closures or coroutines holding battle state | `ContentState` in `Vars`/`ActionVars` (`Hash`, `Copy`); the VM checked and frozen at load (§3.1); `Battle::digest` leaves out only the content, whose hash the round's setup carries, and presentation-only parts |
| Integers only; one simulation RNG; no hash-map iteration | Integer-only boundary with wrapping stores; no `math.random`; the API hands out no maps |
| Outputs write-only | `battle.play_sound` only adds a cue; scripts can't read cues back |
| Content immutable during a battle, identified by a hash both peers compare | Modules and definitions frozen after load; `Content::hash` covers the scripts |
| Bounded work that fails the same way on every peer | The interrupt budget counts VM checkpoints, not time |
| No perspective in simulated state | `battle.local_side` and `battle.viewer_sees` are for presentation only; sound goes to both sides' cue lists (or one side's player's, `play_sound_for`) |
| Re-running a tick free of side effects outside `Battle` | Scripts can't reach the host; the VM keeps nothing |

In-repo, `scripted_chips_roll_back` copies a battle every 97 ticks of a duel and checks the copy plays on
exactly as the battle does; nettai-netplay's tests play synthetic netbattles on the test content through two
peers with latency and jitter, in sync to the KO.

### 6.2 Cost

The worst case is a 10-frame rollback on every rendered frame: a restore, 11 advances each followed by a save,
and a digest. The verification workspace's rollback cost test measures it on soundmod round 1 (frames 10164 to
12164, up to 27 objects): about 135 µs per rendered frame, of a 16,667 µs frame (0.8%); a restore about 5 µs,
an advance 6 µs, a save 3 µs, the digest 29 µs. `crates/nettai-netplay/examples/rollback_cost` measures any round.

## 7. Performance

Where the time goes (`examples/luau_ops`, interpreted):

| Operation | Cost |
|---|---|
| Loop iteration (the VM itself) | 11 ns |
| Library call `battle.dimmed()` | 52 ns |
| `field.flags(3, 2)` | 72 ns |
| Method `me:set_animation(0)` | 89 ns |
| Field read `me.anim` / write | 162 / 181 ns |
| Content state read `s.ticks` / write | 138 / 190 ns |
| Enum state read `s.slot` | 357 ns |
| A field that makes a handle or a `Vec3` (`me.pos`, `me.related1`, `me.sprite`) | 274 to 358 ns |
| `Vec3.new(1, 2, 3)` | 312 ns |
| A 9-field table literal (a hitbox spec) | 244 ns |

The VM is fast; the binding isn't. mlua dispatches every userdata field through a Lua-side `__index` closure,
then its generic callback machinery (stack checks, argument conversion, a userdata type check). A raw-FFI
binding would be the next step if content's share of a tick mattered: tagged userdata for handles, a
`__namecall` that dispatches on Luau's string atoms, and field access through a C `__index` that switches on
the atom. Native code only speeds up the VM's share, which is why it doesn't help here.

## 8. Sandboxing

A pack can only call the API: no `io`, `os`, `debug`, no loading code at run time (`load`, `loadstring`,
`require` after loading), no FFI. Libraries and the global table are read-only (Luau's `sandbox`). Scripts
can't reach the host, can't keep state, can't hang the engine (the interrupt budget) and can't grow without
bound unnoticed (mlua's memory limit is available; a hit must be a fatal content error, since GC timing decides
exactly when it triggers). A content error (a script error, a type error at the API, the budget, a routine the
game would run off the end of a table) stops the battle with a message naming the definition and object,
identically on every peer; a production engine would end the round with an error result rather than panic.

## 9. Not built

- Versioning: `core.d.luau` carrying the API version, the pack's manifest recording the version it targets,
  and the engine refusing content for another major version.
- A generator for state schemas from the Luau types (§3.3).
- The raw-FFI binding (§7).
- A content error that ends the round with an error result instead of a panic (§8).
