# Scripting battle content: a prototype

The request was to make chips, attacks, actions and navis content that sits on a core that knows nothing about
them, instead of code compiled into the engine. docs/design/core-content-boundary.md (on the boundary audit's
branch) draws the line between core, ruleset and content. This document answers the runtime question: what
content is written in, and how it stays frame-exact, deterministic and cheap to snapshot for rollback netplay. It
answers by building one vertical slice, GunDelSol, twice on one content API, and measuring it.

"Tick" is one call of `Battle::tick`. Routine names are the original's. Measurements were taken on an Apple M1 Max,
release builds.

## 0. Summary

**Recommendation: Luau, through a narrow typed content API, with stateless scripts over engine-owned state.**

- **Fidelity.** The GunDelSol slice runs as Luau and matches every golden trace the built-in Rust matches: machgun
  round 1 1074/1074, round 2 1330/1331 (GunDelSol inside Beast Out's rush), soundmod 2952/3481/1966. The Rust
  version of the same slice, written against the same API, matches too.
- **Determinism.** Scripts compute with Luau numbers (doubles, exact to 2^53); the engine applies the game's
  integer rules when a value is stored (wrap to the field's width, reject fractions). Nondeterministic and stateful
  libraries are removed, including from the compiler's builtin folding. An in-repo test shows the built-in kinds,
  the Rust content and the Luau content (interpreted and native) produce the same engine state and sound cues on
  every tick of a 900-tick duel.
- **Snapshots.** Content declares its state schema; the engine stores the values as typed fields inside the battle.
  The Luau VM is shared code and is never snapshotted: `Battle::clone()` stays the whole snapshot, about 1.6 µs.
  Statelessness is enforced at load time (a bytecode check rejects writes to globals and module-level locals) and
  at run time (everything a module returns or captures is frozen). A battle moved to a fresh VM mid-round
  continues identically.
- **Rollback.** A GGPO-style session (`rollback::Session`) replays machgun rounds 1 and 2 with either side's
  input 2, 5 or 10 frames late. Every confirmed frame's state digest equals the straight run's for built-in, Rust
  and Luau content. Sound cues are tagged with their frame and reconciled, so re-simulation never replays a cue.
- **Cost.** Luau runs this content at about 20 to 30 µs per tick while GunDelSols fire, against about 1 µs for the
  Rust. A rendered frame that snapshots, restores and re-simulates 10 ticks costs about 0.4 ms, 2.4% of the 16.7 ms
  frame. On machgun round 1, rolling back 10 frames on *every* frame costs 80 µs per frame. The cost is almost all
  in the generic mlua binding (80 to 130 ns per API call), which a raw-FFI binding would cut by an estimated 3 to 5
  times.
- **Approachability.** Scripts are short and read like the Rust (§4). A definitions file (`core.d.luau`) types the
  whole API for luau-lsp and for an in-process type check that runs in `cargo test`.

The Rust-content path (the same API, compiled in) is the right home for the ruleset and for hot paths; Luau is the
right home for the long tail of chips, navis and effects. Both are shown, and they can be mixed kind by kind.

## 1. What was built

| Where | What |
|---|---|
| crates/bn6-content-api | The contract. `CoreApi`: what content can see and do (objects, the attack in progress, sprites, collision, sounds). `ContentHost`: what a runtime provides (a manifest of kinds and actions, update functions). Typed content state: `Schema`, `ContentState`, `FieldType`. The shared value types (`ObjectRef`, `Vec3`, `PanelPos`, `Pool`, `SpriteId`), which the engine now re-exports. |
| crates/bn6-battle/src/content | The engine side: `impl CoreApi for Battle`, dispatch of object kinds and actions to a content host, the default content per build (features `luau`, `luau-jit`, `rust-content`). |
| crates/bn6-battle/src/rollback.rs | `Session` (prediction, snapshots, re-simulation), `digest`, `engine_digest`, `CueLedger`. |
| crates/bn6-battle/src/scenario.rs | A synthetic duel (two MegaMen with four GunDelS3 each) recorded as an input tape, for self-contained tests and benchmarks. |
| crates/bn6-luau | The Luau runtime: VM set-up and freezing (`sandbox`), the bytecode check (`verify`), the API binding (`bind`), module loading. |
| crates/bn6-content-rust | The slice written in Rust against the content API: the comparison point. |
| content/bn6 | The Luau content pack: `pack.luau` (manifest), `chips/gun_del_sol.luau`, `objects/{attachment,sun_beam,hitbox}.luau`, `lib/slot.luau`, `data/attacks.luau` (generated), and `core.d.luau` (API definitions). |
| examples | `content_bench` (costs per runtime), `luau_ops` (cost per API operation), `rollback_trace` (GGPO over a golden trace given as an argument). |

The slice is the whole of GunDelSol: action 0x37 (`sub_80EDAE0`: phases, timers, animation, counter window, the
reactive abort), the attachment T1#5 (`sub_80B8CD8`, the gun), the sun beam T4#0x48 (`sub_80E5C2C`), the one-tick
hitbox T3#3 (`object_spawnCollisionRegion`) and the sounds 0xF8 and 0xF9. The hitbox is also used by the engine's
own Beast Out claw; when content implements it, the engine spawns it through the content (`kinds::hitbox::spawn`
fills the content state by field name), so both paths share one implementation.

Running it:

```sh
cargo test -p bn6-battle --features luau,rust-content          # runtimes agree; rollback; Luau rules
cargo test -p bn6-luau                                         # bytecode check; type check of content/bn6
cargo run --release -p bn6-battle --example content_bench --features luau-jit,rust-content
cargo run --release -p bn6-battle --example rollback_trace --features trace,luau -- <trace.jsonl> 1
tools/traces-against.sh <checkout> --features bn6-battle/luau  # in bn6battle-verify
```

## 2. The content API

### 2.1 Shape

`CoreApi` is a trait the engine implements and runtimes call through `&mut dyn CoreApi`. The boundary audit
proposes a concrete `Ctx<'a>` struct; a trait is the same surface with the dependency inverted. The runtimes live in
their own crates, which can't depend on the engine because the engine depends on them (to pick its default
content), so the contract has to sit below both. It also lets tests drive content against a fake engine.

Conventions, all shared with the audit:

- **Handles are raw slots.** `ObjectRef { pool, slot }`, not generational. A handle can outlive its object and
  then names the slot's next occupant, as the game's pointers do.
- **Effects are immediate.** Every call changes engine state at once. An object spawned by content runs later
  in the same tick, right after its spawner, exactly as the engine's own spawns do (the traces check this: the
  hitbox GunDelSol spawns resolves in the same tick).
- **Fields are named and typed.** Engine fields are enums with a name, a type and a writability
  (`ObjectField::Anim` is `anim`, `u8`, writable). Content-declared fields go through a `Schema`. A value crosses
  the API as a `Value` (`Nil`, `Bool`, `Int(i64)`, `Object`, `Vec3`) and is converted by the field's type on store.
  Integers wrap to the width, as the game's `strb`/`strh`/`str` do; anything else of the wrong kind is an error.
- **No floats anywhere.** `Vec3` is three `i32` in 16.16 fixed point with wrapping `+` and `-`.

What the slice needed (it is also the list of what content touches): spawn, free, destroy, lifecycle, params;
the object header fields (panel, side, flip, animation, element, timers, damage, stamina, position, related
objects, flags); facing; `set_animation`, `update_sprite`, attach points; the actor's overlay slot, the attack's
step, step-init, variant and chip; status flags; the counter window, the reactive abort, `exit_attack`; sprite
load, animation, stepping and look; collision create, set-up, present, remove, free and hit spark; navi stats
(`sun`, form); panel validity and centers; sound cues. Nothing names GunDelSol.

The audit's API is larger (panels and their types, RNG draws, input, the chip hand, damage words, time freeze,
forms). Those extend the same pattern and weren't needed by this slice.

### 2.2 Content state: schemas the engine owns

Each object kind and action declares its state as named, typed fields. In Luau:

```luau
state = { slot = { "overlay", "related" }, offset_x = "i32", offset_z = "i32", lift = "i8" }
```

and in Rust:

```rust
content_state! {
    pub struct State { slot: Slot, offset_x: i32, offset_z: i32, lift: i8 }
}
```

The engine stores a `ContentState`: up to eight `FieldValue`s (`Bool`, `U8`...`I32`, `Object`, `Vec3`, an enum
index) plus the schema's id. It is a `Copy` value of 136 bytes, kept in `kinds::Vars::Content` for objects and in
`ActionVars::Content` for actions, exactly where the built-in kinds keep their typed structs. So a snapshot is
still `Battle::clone()`, and the project's typed-state rule carries over: fields have names and types, never
offsets (fields are stored in name order; the order is private to the store).

The boundary audit's hard cases hold:

- **The attack scratch outlives the action.** An action's state stays in the actor's attack state after the
  action ends; the next action with the same schema continues from it, a different one starts from zero. Beast
  Out's rush wraps the scripted GunDelSol in machgun round 2 and the trace matches.
- **Content reaches into other objects.** GunDelSol steps its gun's animation (`gun.anim += 1`); the attachment
  copies its owner's sprite look. Both are plain field access on another handle.

### 2.3 Dispatch

`Content` holds the runtime (`Rc<dyn ContentHost>`) and two lookup tables: object kinds by (pool, index) and
actions by number. `kinds::update` and `actions::dispatch` consult them first; unclaimed slots fall through to the
engine's kinds, so content can take over one kind at a time. Cloning a `Battle` clones the `Rc`: code, not state.

A kind keeps its original (pool, index) as its identity, so traces and the frontend see the same objects.

## 3. Candidates

| | Luau (mlua) | Rust content | WASM (not built) | Rhai (not built) | Data timelines (not built) |
|---|---|---|---|---|---|
| Fidelity | All traces match | All traces match | Would match (same API) | Would match | Only with code hooks |
| Integers | Doubles, exact to 2^53; wrap on store | Native | Native i32/i64 | i64, checked | n/a |
| Snapshot | VM not included; checked stateless | Nothing to snapshot | Linear memory, or stateless | Stateless by design (no captures) | Nothing to snapshot |
| Cost per active object-tick | ~4 µs | ~0.1 µs | ~0.3 µs est. (wasmtime), ~2 µs (wasmi) | ~20 µs est. | ~0.1 µs |
| Sandbox | Designed for it (Roblox) | None (trusted code) | Strongest | Good | Total |
| Hot reload | Yes | No (rebuild) | Yes | Yes | Yes |
| Authoring | Small, typed, familiar; luau-lsp | Rust toolchain | Rust toolchain plus wasm | Unfamiliar, weak tooling | Easy for simple chips only |

### 3.1 Luau

Luau is Roblox's Lua dialect: sandboxing built in, a gradual type system with an LSP, a fast interpreter and native
code generation. It is embedded through `mlua` (0.12, feature `luau`, Luau 0.736).

**Numbers.** Luau numbers are IEEE doubles; there is no integer subtype (this Luau has an experimental 64-bit
`integer` type behind feature flags, library calls only; not used). Doubles are exact for integers up to 2^53, far
beyond the engine's 32-bit values, and `+ - *` and `//` on such integers are exact on every IEEE machine. So scripts
compute with plain numbers and the engine applies the game's rules at the boundary:

- every number entering the engine must be an integer: `s.timer = 13 / 2` fails with "6.5 is not an integer";
- a store wraps to the field's width: `s.timer = t` with `t = -1` stores 0xFFFF into a `u16`, which is the Rust
  version's `t as u16`, and the script keeps testing its local `t < 0`;
- `Vec3` is a Rust value (three `i32`) with wrapping `+`/`-`, so fixed-point positions can't drift;
- `int` covers what plain arithmetic can't: `tdiv`/`tmod` (truncating, the ARM's), `asr`/`shl`/`lsr` (32-bit
  shifts), `mul32`, and width wrappers (`int.i32(x)`); `bit32` gives bitwise operations (u32 results).

The rule a modder learns is one line: use `//` or `int`, never `/`.

**Determinism.** Beyond the integer rule:

- `math` keeps only exact functions (`abs`, `ceil`, `floor`, `max`, `min`, `clamp`, `sign`, `round`). No `random`
  (the RNG is the engine's), no `noise`, no transcendental functions (C libraries disagree in the last bits).
- Luau's compiler folds calls to known builtins with constant arguments and, under `safeenv`, calls builtins
  directly (`FASTCALL`) without looking them up. Removing `math.sin` from the environment therefore does not stop
  `math.sin(1)`: the first version of this sandbox let it through, and a test caught it. The removed functions
  (and `setmetatable`, so calls reach the guarded version) are now also given to the compiler as disabled builtins.
- No `os`, `io`, `debug`, `coroutine` (a suspended coroutine is hidden state), `buffer` (mutable memory), `utf8`,
  `vector`; no `collectgarbage`/`gcinfo` (they observe the GC), `getfenv`/`setfenv`, `loadstring`, `newproxy`.
- Weak tables (`__mode`) are refused: their contents depend on when the GC ran.
- Iteration order of tables keyed by strings is deterministic (Luau's string hash is unseeded), but tables keyed by
  handles or tables iterate in address order. Content should iterate arrays; the prototype's API hands out no
  collections, so this only matters for tables a script builds itself. A lint is the remaining guard.
- Runaway scripts stop on a budget of interrupt checks (calls, returns, loop back-edges), a count rather than a
  clock, so every machine stops at the same point. Hitting it is a content error.
- Native code (`luau-jit`) builds and runs on this machine (aarch64) and gives bit-identical battles; it computes
  the same double operations. It doesn't pay off here (§6), so the interpreter is the default.

**Statelessness, enforced.** A script function could keep state in three places, and each is closed:

1. *Globals.* The loader's bytecode check (`verify`) rejects any `SETGLOBAL`; modules export through their return
   value. After loading, the global table is read-only as well.
2. *Module-level locals captured by functions* (`local hums = 0` at the top, `hums += 1` in `update`). The check
   follows each closure's `CAPTURE` chain back to the main chunk and rejects any `SETUPVAL` that writes a module
   local. Writing a local of an enclosing *function* is allowed (that frame dies with the call). The check reads
   Luau bytecode as `lvmload.cpp` does and fails closed on anything it doesn't understand.
3. *Tables reachable from a module* (`local seen = {}`, then `seen[1] = x`). Everything a module returns or its
   functions capture is deep-frozen after it loads, so the write fails at run time with "attempt to modify a
   readonly table". Data tables are frozen the same way.

`require` works only while modules load, so dependencies are static. The Luau VM therefore holds only frozen code
and data between calls, and nothing in it can differ between two machines or between a run and its rollback.
Tests show the rules fire (`content::tests::luau`): module-local counters and global writes are rejected at load;
writes to module tables, data tables, captured tables and `math` fail; `math.random`, `math.sin`, `os.time`,
`collectgarbage`, `coroutine` and `buffer` are absent; weak tables are refused; fractions can't enter state;
runaway loops stop. And they show the VM carries nothing: a battle moved to a fresh VM halfway continues
identically; a second battle interleaved on the same VM changes nothing; a full GC after every call changes
nothing.

**Typing.** `content/bn6/core.d.luau` declares the API (`Object`, `Sprite`, `Collision`, `Vec3`, `battle`, `int`,
string-literal types for pools, lifecycle states, shadows and status flags). Scripts are `--!strict` and declare
their state types (`export type State = { timer: number }`). `crates/bn6-luau/tests/types.rs` type-checks every
module with Luau's own analysis (the `luau-analyze` crate, in process) and checks that API misuse (a misspelled
field, a lifecycle state that doesn't exist, `Vec3 + number`) is a type error. An editor with luau-lsp and
`--definitions=content/bn6/core.d.luau` gets completion and the same errors, across modules.

The state schema (`state = { timer = "u16" }`) and the Luau type (`State = { timer: number }`) are written twice.
A small generator could emit one from the other; in this spike they are checked by the tests that play the duel.

### 3.2 Rust content

`crates/bn6-content-rust` is the same slice in Rust, calling only `CoreApi` (with typed accessors,
`api.anim(gun)`, and `content_state!` structs). It matches every trace and costs about 1.4 times the built-in
kinds (the API's dynamic dispatch and field conversions). It has no sandbox and needs a rebuild, so it is for trusted
content: the ruleset (the navi framework, the hit kernel) and the hottest kinds (hitboxes).

### 3.3 Not built

- **WASM** (content in Rust compiled to wasm32, run by wasmtime or wasmi). The guest would be the Rust content
  above with `CoreApi` implemented over host imports; integer semantics are native and sandboxing is the
  strongest. Snapshots: if guests follow the same stateless rule, nothing is copied; otherwise the linear memory is
  (tens of KiB at least, per snapshot). The cost for modders is the Rust-plus-wasm toolchain, which is why it
  loses to Luau for the long tail. It remains the fallback if a sandbox for native-speed content is ever needed.
- **Rhai.** Pure Rust, `i64` integers, can be built without floats, and script functions can't capture outer
  variables, so statelessness comes free. But it is a tree-walking interpreter (several times slower than Luau),
  with little tooling and an unfamiliar language for this community.
- **Lua 5.4** (dropped at the user's direction). Its integer subtype would remove the "no `/`" rule, but it lacks
  Luau's sandbox, type checker and speed.
- **Declarative timelines.** Many chips are "after N ticks spawn X; hit pattern P for M ticks". GunDelSol fits
  partly: its phases and durations are data, but the level- and sun-dependent beam, the EX region and the
  wrap-around timer quirks need code. The better form is a Luau library of combinators (phases, waits, per-tick
  hits) that simple chips use as data and complex ones mix with code, all in one runtime.

### 3.4 Open Net Battle

Open Net Battle (ONB), the open Battle Network-style engine, scripts cards, characters and spells in Lua through
sol2. Content attaches callbacks to engine objects (`on_update_func`, `on_execute_func`, `add_anim_action(frame,
fn)` on card actions, and so on), and per-object state lives in the Lua heap: in closure upvalues and in tables hung
on entities. That is approachable, but it makes the Lua heap part of the battle state, so a snapshot would have to
serialize or fork the VM. ONB's network play is lockstep. The design here keeps ONB's authoring shape (a module per
card or entity, an update function per kind) and moves the state out of the VM, which is what rollback needs.

## 4. GunDelSol, as content

The wind-up and firing phases, Luau (content/bn6/chips/gun_del_sol.luau):

```luau
-- `sub_80EDB14`: the gun comes out; 6 ticks later the beam lights up.
local function wind_up(me: Object, s: State)
    local level = me.variant
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(0x0A)
        me:open_counter_window()
        attachment.spawn(me, 7 + level, "overlay")
        battle.play_sound(0xF8)
        s.timer = 6
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t > 0 then
        return
    end
    s.timer = data.GUN_DEL_SOL_FIRING_TICKS[level]
    advance_gun(me)
    local sun = if battle.navi(me.alliance).sun then 1 else 0
    local look = data.GUN_DEL_SOL_BEAMS[sun][level]
    me.related1 = sun_beam.spawn(me, look, Vec3.px(me.facing * 0x50, 0, 0), "related")
    set_phase(me, FIRING)
end

-- `sub_80EDBCC`: one hit this tick, until the timer runs out.
local function fire(me: Object, s: State)
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        local beam = me.related1
        if beam then
            sun_beam.stop(beam)
        end
        set_phase(me, RECOVER)
        return
    end
    hitbox.spawn(me, {
        panel_x = me.panel_x + 2 * me.facing,
        panel_y = me.panel_y,
        element = 5,
        region = if me.variant < 3 then 0x04 else 0x11, -- EX covers two columns.
        hit_effect = 0xFF,
        target = 0x05,
        self_type = 0x2C,
        damage = if battle.navi(me.alliance).sun then 4 else 2,
    })
end
```

The same, Rust content (crates/bn6-content-rust/src/gun_del_sol.rs):

```rust
/// `sub_80EDB14`: the gun comes out; 6 ticks later the beam lights up.
fn wind_up(api: &mut dyn CoreApi, me: ObjectRef) {
    let level = api.variant(me);
    if api.step_init(me) == 0 {
        api.set_status(me, StatusFlag::UsingAction, true).expect("a navi has collision");
        api.set_animation(me, 0x0A);
        api.open_counter_window(me);
        attachment::spawn(api, me, 7 + level, Slot::Overlay);
        api.play_sound(0xF8);
        store(api, me, State { timer: 6 });
        api.set_step_init(me, 4);
        return;
    }
    let mut s = load(api, me);
    let t = s.timer as i32 - 1;
    s.timer = t as u16;
    store(api, me, s);
    if t > 0 {
        return;
    }
    store(api, me, State { timer: GUN_DEL_SOL_FIRING_TICKS[level as usize] });
    advance_gun(api, me);
    let look = GUN_DEL_SOL_BEAMS[in_sun(api, me) as usize][level as usize];
    let offset = Vec3 { x: (api.facing(me) * 0x50) << 16, y: 0, z: 0 };
    let beam = sun_beam::spawn(api, me, look, offset, Slot::Related);
    api.set_related1(me, beam);
    set_phase(api, me, FIRING);
}

/// `sub_80EDBCC`: one hit this tick, until the timer runs out.
fn fire(api: &mut dyn CoreApi, me: ObjectRef) {
    let mut s = load(api, me);
    let t = s.timer as i32 - 1;
    s.timer = t as u16;
    store(api, me, s);
    if t < 0 {
        if let Some(beam) = api.related1(me) {
            sun_beam::end(api, beam);
        }
        return set_phase(api, me, RECOVER);
    }
    let damage = if in_sun(api, me) { 4 } else { 2 };
    // EX covers two columns.
    let region = if api.variant(me) < 3 { 0x04 } else { 0x11 };
    let ahead = (2 * api.facing(me)) as u8;
    let spec = hitbox::Spec {
        panel: PanelPos { x: api.panel_x(me).wrapping_add(ahead), y: api.panel_y(me) },
        element: 5,
        region,
        hit_effect: 0xFF,
        target: 0x05,
        self_type: 0x2C,
        damage,
        ..hitbox::Spec::default()
    };
    hitbox::spawn(api, me, &spec);
}
```

Both follow the engine's built-in version (kinds/player/actions/gun_del_sol.rs) line for line. The Luau is the
shortest and needs no casts: `s.timer = t` wraps because `timer` is declared `u16`, and `me.panel_x + 2 *
me.facing` wraps into the `u8` panel field. A module is registered by the pack manifest:

```luau
return {
    objects = { require("./objects/attachment"), require("./objects/sun_beam"), require("./objects/hitbox") },
    actions = { require("./chips/gun_del_sol") },
}
```

where an object kind is `{ pool, index, state, update }` and an action `{ action, state, update }`.

## 5. Fidelity

| Trace | Built-in | Rust content | Luau |
|---|---|---|---|
| machgun round 1 | 1074/1074 | 1074/1074 | 1074/1074 |
| machgun round 2 | 1330/1331 | 1330/1331 | 1330/1331 |
| soundmod rounds 1/2/3 | 2952/3481/1966 | 2952/3481/1966 | 2952/3481/1966 |

The one machgun round 2 frame and the soundmod stops are features this branch's base doesn't have (round chaining,
Cross changes), not content. To be sure the traces ran the content, each runtime was sabotaged once (the Rust
content made to panic; the Luau GunDelSol's damage changed from 4 to 3): machgun round 1 then stops at frame 647,
the first hit. (A change to the sun beam's hum timing didn't show: traces don't compare sound.)

In-repo, `every_runtime_plays_the_duel_like_the_engine` plays the 900-tick duel with each runtime and compares, every
tick, a digest of everything they represent alike (all engine state except the content's own state structs, which
differ in representation) and the tick's sound cues. Built-in, Rust, Luau and Luau native agree on all 900.

## 6. Snapshots and rollback

### 6.1 The model

The battle is a value: `Battle: Clone`, 9,232 bytes inline plus the object and sprite arrays, about 1.6 µs to clone
or restore. Content changes nothing about that: its state is inside, and its runtime isn't state. The rules that
make this true for any runtime:

1. content declares its state; the engine stores it as typed values in the battle;
2. update functions are deterministic functions of engine state and their arguments, keeping nothing between
   calls (enforced for Luau as in §3.1);
3. randomness comes only from the engine's RNG streams; time comes only from the tick count;
4. outputs (sound cues, presentation) are derived per tick and never read back.

### 6.2 The session

`rollback::Session` runs each frame at once with the local input and a predicted remote input (the last one
received). It snapshots the battle before every frame whose remote input hasn't arrived. When a remote input
arrives and differs from what was predicted, it restores the snapshot before that frame and re-simulates to the
present with the real input and fresh predictions. Snapshots before confirmed frames are dropped. It records a
digest of each frame once that frame is final.

`rollback::digest` hashes the battle's whole simulation state (FNV-1a over its `Debug` form, which has no
hash-ordered containers) and leaves out the tick's sound cues and the content handle. `engine_digest` also leaves
out content-private state structs, for comparing runtimes with each other.

### 6.3 Results

In repo (`rollback::tests`), the 900-tick duel, remote input 1, 2, 5 and 10 frames late: every confirmed frame's
digest equals the straight run's, and the final states are equal, for built-in, Rust and Luau content.

On the golden traces (`examples/rollback_trace`), predicting the trace's remote side, then the side firing
GunDelSol (which mispredicts far more):

| | Remote side predicted | Firing side predicted |
|---|---|---|
| machgun round 1, 2/5/10 frames late | 6 rollbacks; all 1074 frames equal | 84 rollbacks (168/420/840 frames re-simulated); all 1074 frames equal |
| machgun round 2, 2/5/10 frames late | 6 rollbacks; all 1330 frames equal | 63 rollbacks (126/315/630 re-simulated); all 1330 frames equal |

Every combination holds for built-in, Rust and Luau content.

### 6.4 Costs

Per rendered frame, on machgun round 1 (whole round, averaged):

| | Straight | Session, firing side 10 frames late | Every frame rolls back 10 frames |
|---|---|---|---|
| Built-in | 0.4 µs | 5.9 µs | 8.1 µs |
| Rust content | 0.5 µs | 5.4 µs | 9.5 µs |
| Luau | 7.9 µs | 24.4 µs | 79.6 µs |

Against a 16,667 µs frame, the worst case with Luau is 0.5%. In the synthetic duel, mid-GunDelSol (both navis
firing at tick 400): a snapshot, a restore and a 10-tick re-simulation cost 16 µs built-in and about 400 µs Luau
(2.4% of a frame). The session's own overhead is the per-frame snapshot (1.6 µs).

### 6.5 Sound and presentation under rollback

Sound is the one output that can't be taken back. Each tick's cues are tagged with the frame that made them. The
session's `CueLedger` remembers, per unconfirmed frame, the cues already played. When a frame is simulated or
re-simulated it emits `Play { frame, cue }` for cues not yet played for that frame and `Retract { frame, cue }` for
cues played on a misprediction that the corrected frame doesn't make. So a re-simulation plays nothing twice and
late cues still play (a few frames late, as in any rollback game). The audio layer plays `Play`, may stop a long
sound on `Retract` and ignore a short one, and should treat music as a state (the latest `Music` cue of the current
timeline), not an event. `resimulated_cues_are_not_played_twice` checks that after reconciling, the cues played
for each frame are exactly the straight run's.

Presentation needs nothing: sprites, looks and objects are battle state, so drawing the current (possibly
predicted) battle is always right, and a rollback redraws the corrected one.

## 7. Performance

`examples/content_bench` (release, M1 Max; runs vary by about 20%):

| Runtime | Duel, 900 ticks | While GunDelSol fires | Per object-tick, 30 objects × 10,000 ticks | Snapshot | Restore | Snapshot + restore + 10 ticks |
|---|---|---|---|---|---|---|
| Built-in | 0.63 µs/tick | 0.81 µs/tick | 0.02 µs | 1.66 µs | 2.37 µs | 16 µs |
| Rust content | 1.01 µs/tick | 1.32 µs/tick | 0.09 µs | 1.58 µs | 1.98 µs | 18 µs |
| Luau | 21 µs/tick | 28 µs/tick | 4.3 µs | 1.56 µs | 1.79 µs | 395 µs |
| Luau native | 23 µs/tick | 31 µs/tick | 4.2 µs | 1.56 µs | 2.23 µs | 402 µs |

The 30-object case is 15 guns and 15 sun beams following their owner, each making about 20 API calls a tick: 30
scripted objects cost about 130 µs a tick, 1.4 ms for a frame with a 10-tick rollback, 8% of the frame.

Where the time goes (`examples/luau_ops`):

| Operation | Cost |
|---|---|
| Loop iteration (the VM itself) | 10 ns |
| Library call `battle.time_stop()` | 39 ns |
| Method `me:param(1)` | 80 ns |
| Field read `me.anim` / write | 117 / 126 ns |
| Content state read `s.ticks` / write | 122 / 136 ns |
| Enum state read `s.slot` | 219 ns |
| A field that makes a handle (`me.pos`, `me.related1`, `me.sprite`) | 205 to 220 ns |
| `Vec3.new(1, 2, 3)` | 176 ns |
| A 9-field table literal (the hitbox spec) | 85 ns (9 ns native) |

The VM is fast; the binding isn't. mlua dispatches every userdata field through a Lua-side `__index` closure, then
its generic callback machinery (stack checks, argument conversion, a userdata type check). A sampling profile of
the firing window shows the interpreter at 18% of the time, allocation and GC at about 5%, the budget interrupt at
about 5%, and the rest in mlua's call path. Native code only speeds up the part that is the VM's, which is why it
doesn't help here.

The next step, if Luau is chosen, is a raw-FFI binding: tagged userdata (`lua_newuserdatatagged`) for handles, a
`__namecall` that dispatches on Luau's string atoms, and field access through a C `__index` that switches on the
atom instead of calling a Lua closure. That is the path Roblox's own bindings take; 20 to 40 ns per call is a
reasonable target, 3 to 5 times less. Handles could also be cached per call to avoid allocations.

Loading the pack (VM, compiling, checking, freezing) takes about 5 ms, once per process.

## 8. Sandboxing

A content pack can only call the API: no `io`, `os`, `debug`, no loading of code at run time (`load`,
`loadstring`, `require` after loading), no FFI. Libraries and the global table are read-only (Luau's `sandbox`).
Scripts can't reach the host, can't keep state, can't hang the engine (the interrupt budget) and can't grow
without bound unnoticed (mlua's memory limit is available; a hit must be a fatal content error, since GC timing
decides exactly when it triggers). A content error (a script error, a type error at the API, the budget) stops the
battle with a message naming the module and object, identically on every machine; a production engine would end
the round with an error result rather than panic.

## 9. Content bundles: scripts, data and assets together

At the user's direction, sprites and other assets belong in the content bundle too. A bundle is a directory (or an
archive) that holds everything a piece of content needs:

```
content/bn6/
  pack.luau            manifest: the kinds, actions and chips it defines, and the assets it provides
  chips/, objects/, lib/   scripts
  data/                tables generated by bn6-extract (numbers; committed)
  assets/              generated by bn6-extract from the user's ROM (never committed): sprite sheets with
                       their animations, palettes, sounds, in the formats being prototyped separately
```

What this changes for the core:

- **Sprites by id, resolved at load.** Scripts refer to sprites the bundle declares (`sprites.sun_beam`), not to
  the original's (category, index) pairs; the loader maps them to engine sprite handles. The prototype still
  passes raw pairs.
- **Animation timing is simulation data.** Frame durations and frame flags decide when effects end, so they are
  part of the battle, not presentation. Today the core's animation stepper reads them from the compiled-in
  `SPRITES` table (the audit's coupling §1.4 item 3). It would read them from the loaded bundles instead, through
  a registry the bundle fills at load. Pixels, palettes and sounds stay presentation.
- **Netplay checks the bundles.** Two peers must run identical content. The handshake exchanges a hash of each
  bundle's simulation-relevant parts (manifest, scripts, data, animation timing); assets that only draw or play
  can differ, so a player could use a restyled sprite without desyncing.

## 10. Recommendation and next steps

1. **Adopt the content API and the stateless-content rule** as the boundary, whatever the runtime: declared
   schemas, engine-owned typed state, immediate effects, raw-slot handles. It costs the engine nothing (snapshots
   are unchanged) and it is what makes rollback work with scripted content.
2. **Write the ruleset and the hottest kinds in Rust against the API**, starting where the audit's migration plan
   starts (hitboxes, effects, attachments), since the Rust-content path costs about 1.4 times the built-in code.
3. **Write chips, navis and effects in Luau**, with the load-time rules above, `core.d.luau` versioned with the
   engine, and the type check in CI.
4. Before content grows: the raw-FFI binding (§7), a generator for the state schema from the Luau type, the
   bundle loader with sprite ids and animation timing (§9), a bundle hash in the netplay handshake, and a content
   error that ends the round instead of panicking.
