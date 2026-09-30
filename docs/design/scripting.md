# Scripting battle content: a prototype

The request was to make chips, attacks, actions and navis content that sits on a core that knows nothing about
them, instead of code compiled into the engine. docs/design/core-content-boundary.md (the boundary audit) draws
the line between core, ruleset and content; docs/design/rollback.md covers rollback netplay. This document answers
the runtime question: what content is written in, and how it stays frame-exact, deterministic and cheap to
snapshot under rollback. It answers by building one vertical slice, GunDelSol, twice on one content API, and
measuring it.

"Tick" is one call of `Battle::tick`. Routine names are the original's. Measurements were taken on an Apple M1 Max
in release builds, on a machine shared with other work, so single runs vary by up to 2x; figures are the best of
several runs.

## 0. Summary

**Recommendation: Luau, through a narrow typed content API, with stateless scripts over engine-owned state.**

- **Fidelity.** The GunDelSol slice runs as Luau and matches every golden trace the built-in Rust matches:
  machgun rounds 1 and 2 complete (1074/1074, 1331/1331; round 2 runs GunDelSol inside Beast Out's rush), soundmod
  4513/6284/2566. Soundmod also runs the Luau attachment and hitbox kinds, spawned by engine code (the buster's
  arm, other attacks' hitboxes): sabotaging either script breaks it. The same slice in Rust against the same API
  matches too.
- **Determinism.** Scripts compute with Luau numbers (doubles, exact to 2^53); the engine applies the game's
  integer rules when a value is stored (wrap to the field's width, reject fractions). Nondeterministic and stateful
  libraries are removed, from the environment and from the compiler's builtin folding. An in-repo test shows the
  built-in kinds, the Rust content and the Luau content (interpreted and native) give the same engine state and
  sound cues on every tick of a 900-tick duel.
- **Rollback.** Content declares its state schema; the engine stores the values as typed fields inside `Battle`,
  so `save_state`/`load_state` and `Battle::digest` cover them unchanged. The Luau VM holds no battle state and is
  never snapshotted. This is enforced at load (a bytecode check rejects writes to globals and module-level
  locals) and at run time (everything a module returns or captures is frozen). Under bn6-netplay the scripted
  slice stays in sync: the golden traces through two rollback peers at latencies of 0 to 10 frames match on
  every confirmed frame, as do bn6-netplay's synthetic netbattles (thousands of rollbacks each).
- **Cost.** The worst case of rollback, a 10-frame rollback on every rendered frame, measured as
  `soundmod_rollback_cost` does: machgun round 1, the GunDelSol round, 151 µs per frame with Luau against 54 µs
  with the built-in Rust (0.9% of a 16.7 ms frame); soundmod round 1, 56 µs against 55 µs. Luau costs about 25 to
  35 µs a tick while GunDelSols fire, against about 1 µs for Rust. Nearly all of it is the generic mlua binding (80
  to 130 ns per API call); a raw-FFI binding would cut it an estimated 3 to 5 times.
- **Approachability.** Scripts are short and read like the Rust (§4). `core.d.luau` types the whole API for
  luau-lsp and for an in-process type check (`bn6-content-check`) that runs in `cargo test`.

The Rust-content path (the same API, compiled in) suits the ruleset and hot paths; Luau suits the long tail of
chips, navis and effects. Both are shown, and they mix kind by kind.

## 1. What was built

| Where | What |
|---|---|
| crates/bn6-content-api | The contract. `CoreApi`: what content can see and do (objects, the attack in progress, sprites, collision, sounds). `ContentHost`: what a runtime provides (a manifest of kinds and actions, update functions). Typed content state: `Schema`, `ContentState`, `FieldType`. The shared value types (`ObjectRef`, `Vec3`, `PanelPos`, `Pool`, `SpriteId`), which the engine re-exports. |
| crates/bn6-battle/src/behavior | The engine side: `impl CoreApi for Battle`, dispatch of object kinds and actions to a content host (`Behaviors`), the behaviors a build runs by default (features `luau`, `luau-jit`, `rust-content`), and the scripts' data built from the battle's `Content` (`luau_pack`, `luau_data`). |
| crates/bn6-battle/src/scenario.rs | A synthetic duel on the engine's hand-authored test content (`content::testing`: two navis whose folders hold a made-up level-3 GunDelSol chip) recorded as an input tape, for self-contained tests and benchmarks (feature `test-content` outside the crate's own tests). |
| crates/bn6-luau | The Luau runtime: the VM and freezing (`sandbox`), the bytecode check (`verify`), the API binding (`bind`), module loading, the pack's content hash. |
| crates/bn6-content-rust | The slice in Rust against the content API: the comparison point. Its data (`data::Data`) comes from the battle's `Content` (`Behaviors::rust(&content)`). |
| crates/bn6-content-check | Type-checks a Luau pack against its definitions with Luau's analysis, in process (library, binary and the test for content/bn6). |
| content/bn6 | The Luau pack: `pack.luau` (manifest), `chips/gun_del_sol.luau`, `objects/{attachment,sun_beam,hitbox}.luau`, `lib/slot.luau`, `data/pack.luau` (a type stub: at load the engine replaces the module with one built from the battle's `Content`), `core.d.luau` (the API) and `types.d.luau` (types shared by the pack's modules). |
| crates/bn6-netplay | Features `luau` and `rust-content` run its rollback tests on the scripted slice; `examples/rollback_cost` measures a trace round as `soundmod_rollback_cost` does. |
| crates/bn6-battle/examples | `content_bench` (costs per runtime), `luau_ops` (cost per API operation). |

The slice is the whole of GunDelSol: action 0x37 (`sub_80EDAE0`: phases, timers, animation, counter window, the
reactive abort), the attachment T1#5 (`sub_80B8CD8`, the gun), the sun beam T4#0x48 (`sub_80E5C2C`), the one-tick
hitbox T3#3 (`object_spawnCollisionRegion`) and the sounds 0xF8 and 0xF9. The engine's own code spawns two of
those kinds too (the buster's arm is an attachment; the claw, meteors, grab shots and dust balls spawn hitboxes).
When content implements a kind, the engine's spawn helpers create it through the content and pass their arguments
as its state by field name (`behavior::set_state_field`), so there is one implementation per kind.

Running it:

```sh
cargo test -p bn6-battle --features luau,rust-content        # the runtimes agree; Luau's rules
cargo test -p bn6-netplay --features luau                    # rollback on the scripted slice
cargo test -p bn6-luau -p bn6-content-check                  # bytecode check; the pack type-checks
cargo run --release -p bn6-battle --example content_bench --features luau-jit,rust-content,test-content
cargo run --release -p bn6-netplay --example rollback_cost --features trace,luau -- <trace.jsonl> <pack> 1
```

The golden-trace suite outside this repository runs against the branch with `--features bn6-battle/luau` (or
`rust-content`).

## 2. The content API

### 2.1 Shape

`CoreApi` is a trait the engine implements and runtimes call through `&mut dyn CoreApi`. The boundary audit
proposes a concrete `Ctx<'a>` struct; a trait is the same surface with the dependency inverted. The runtimes live
in their own crates, which can't depend on the engine because the engine depends on them (to pick its default
content), so the contract has to sit below both. It also lets tests drive content against a fake engine.

Conventions, all shared with the audit:

- **Handles are raw slots.** `ObjectRef { pool, slot }`, not generational. A handle can outlive its object and
  then names the slot's next occupant, as the game's pointers do.
- **Effects are immediate.** Every call changes engine state at once. An object spawned by content runs later
  in the same tick, right after its spawner, as the engine's own spawns do (the traces check this: the hitbox
  GunDelSol spawns resolves in the same tick).
- **Fields are named and typed.** Engine fields are enums with a name, a type and a writability
  (`ObjectField::Anim` is `anim`, `u8`, writable). Content-declared fields go through a `Schema`. A value crosses
  the API as a `Value` (`Nil`, `Bool`, `Int(i64)`, `Object`, `Vec3`) and is converted by the field's type on store.
  Integers wrap to the width, as the game's `strb`/`strh`/`str` do; anything else of the wrong kind is an error.
- **No floats anywhere.** `Vec3` is three `i32` in 16.16 fixed point with wrapping `+` and `-`.

What the slice needed (the list of what content touches): spawn, free, destroy, lifecycle, params; the object
header fields (panel, side, flip, animation, element, timers, damage, stamina, position, related objects, flags);
facing; `set_animation`, `update_sprite`, attach points; the actor's overlay slot, the attack's step, step-init,
variant and chip; status flags; the counter window, the reactive abort, `exit_attack`; sprite load, animation,
stepping and look; collision create, set-up, present, remove, free and hit spark; navi stats (`sun`, form); panel
validity and centers; sound cues. Nothing names GunDelSol.

The audit's API is larger (panels and their types, RNG draws, input, the chip hand, damage words, dimming,
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
index) and the schema's id. It is a `Copy` value of 136 bytes kept in `kinds::Vars::Content` for objects and in
`ActionVars::Content` for actions, where the built-in kinds keep their typed structs. So `Battle: Clone` is still
the whole snapshot and `#[derive(Hash)]` still covers it for the digest. The typed-state rule carries over: fields
have names and types, never offsets (they are stored in name order, which is private to the store).

The boundary audit's hard cases hold:

- **The attack scratch outlives the action.** An action's state stays in the actor's attack state after the
  action ends; the next action with the same schema continues from it, a different one starts from zero. Beast
  Out's rush wraps the scripted GunDelSol in machgun round 2 and the trace matches.
- **Content reaches into other objects.** GunDelSol steps its gun's animation (`gun.anim += 1`); the attachment
  copies its owner's sprite look. Both are plain field access on another handle.

### 2.3 Dispatch

`behavior::Behaviors` holds the runtime and two lookup tables: object kinds by (pool, index) and actions by number.
`kinds::update` and `actions::dispatch` consult them first; unclaimed slots fall through to the engine's kinds,
so content can take over one kind at a time. A kind keeps its original (pool, index) as its identity, so traces
and the frontend see the same objects. `Battle::digest` leaves the behaviors handle out (it is code); the
content's state is hashed with the objects and actors that hold it.

The handle (`Battle::behaviors`) is an `Rc`, because mlua's `Lua` isn't `Send`. It sits beside the battle's data,
`Battle::content` (the loaded content pack, an `Arc<Content>`); §9.4 describes how the two fit together.

## 3. Candidates

| | Luau (mlua) | Rust content | WASM (not built) | Rhai (not built) | Data timelines (not built) |
|---|---|---|---|---|---|
| Fidelity | All traces match | All traces match | Would match (same API) | Would match | Only with code hooks |
| Integers | Doubles, exact to 2^53; wrap on store | Native | Native i32/i64 | i64, checked | n/a |
| Snapshot | VM not included; checked stateless | Nothing to snapshot | Linear memory, or stateless | Stateless by design (no captures) | Nothing to snapshot |
| Cost per active object-tick | ~4.5 µs | ~0.1 µs | ~0.3 µs est. (wasmtime), ~2 µs (wasmi) | ~20 µs est. | ~0.1 µs |
| Sandbox | Designed for it (Roblox) | None (trusted code) | Strongest | Good | Total |
| Hot reload | Yes | No (rebuild) | Yes | Yes | Yes |
| Authoring | Small, typed, familiar; luau-lsp | Rust toolchain | Rust toolchain plus wasm | Unfamiliar, weak tooling | Easy for simple chips only |

### 3.1 Luau

Luau is Roblox's Lua dialect: sandboxing built in, a gradual type system with an LSP, a fast interpreter and native
code generation. It is embedded through `mlua` (0.12, feature `luau`, Luau 0.736).

**Numbers.** Luau numbers are IEEE doubles; there is no integer subtype (this Luau has an experimental 64-bit
`integer` type behind feature flags, library calls only; not used). Doubles are exact for integers up to 2^53, far
beyond the engine's 32-bit values, and `+ - *` and `//` on such integers are exact on every IEEE machine. So
scripts compute with plain numbers and the engine applies the game's rules at the boundary:

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
  directly (`FASTCALL`) without looking them up. So removing `math.sin` from the environment does not stop
  `math.sin(1)`: the first version of this sandbox let it through, and a test caught it. The removed functions
  (and `setmetatable`, so calls reach the guarded version) are now also disabled builtins for the compiler.
- No `os`, `io`, `debug`, `coroutine` (a suspended coroutine is hidden state), `buffer` (mutable memory), `utf8`,
  `vector`; no `collectgarbage`/`gcinfo` (they observe the GC), `getfenv`/`setfenv`, `loadstring`, `newproxy`.
- Weak tables (`__mode`) are refused: their contents depend on when the GC ran.
- Tables keyed by strings iterate deterministically (Luau's string hash is unseeded), but tables keyed by handles
  or tables iterate in address order. The API hands out no collections, so this only matters for tables a script
  builds itself; content should iterate arrays, and a lint is the remaining guard.
- Runaway scripts stop on a budget of interrupt checks (calls, returns, loop back-edges): a count rather than a
  clock, so every machine stops at the same point. Hitting it is a content error.
- Native code (`luau-jit`) builds and runs here (aarch64) and gives bit-identical battles. It doesn't pay off for
  this content (§7), so the interpreter is the default.

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

`require` works only while modules load, so dependencies are static. Between calls the VM holds only frozen code
and data, and nothing in it can differ between machines or between a run and its rollback. The tests
(`behavior::tests::luau`) show each rule fires: module-local counters and global writes are rejected at load;
writes to module tables, data tables, captured tables and `math` fail; `math.random`, `math.sin`, `os.time`,
`collectgarbage`, `coroutine` and `buffer` are absent; weak tables are refused; fractions can't enter state;
runaway loops stop. They also show the VM carries nothing: a battle moved to a fresh VM halfway continues
identically; a second battle interleaved on the same VM changes nothing, even when 593 of its updates panic inside
a Luau call; a full GC after every call changes nothing. bn6-netplay's `state_outside_the_snapshot_is_caught` shows
what a leak would do (the peers diverge within a few hundred frames); the load-time rules are what keep scripts
from leaking.

**Typing.** `content/bn6/core.d.luau` declares the API (`Object`, `Sprite`, `Collision`, `Vec3`, `battle`, `int`,
string-literal types for pools, lifecycle states, shadows and status flags); `types.d.luau` declares the types the
pack's modules share. Scripts are `--!strict` and declare their state types (`export type State = { timer: number
}`). `bn6-content-check` type-checks every module with Luau's own analysis (the `luau-analyze` crate, new solver,
in process) and its test also checks that API misuse (a misspelled field, a lifecycle state that doesn't exist,
`Vec3 + number`, an unknown pool) is a type error. An editor with luau-lsp and both definition files gets
completion and the same errors, with `require` resolved across modules.

The checker bundles its own Luau (0.710), whose C++ symbols collide with mlua's (0.736), so it lives in its own
crate and must never share a binary with bn6-luau. (A first version linked both into one test binary; the checker
hung, running against the other copy's code.)

The state schema (`state = { timer = "u16" }`) and the Luau type (`State = { timer: number }`) are written twice.
A generator could emit one from the other; here the duel tests check they agree.

### 3.2 Rust content

`crates/bn6-content-rust` is the same slice in Rust, calling only `CoreApi` (with typed accessors,
`api.anim(gun)`, and `content_state!` structs). It matches every trace and costs about 1.5 times the built-in kinds
(the API's dynamic dispatch and field conversions). It has no sandbox and needs a rebuild, so it is for trusted
content: the ruleset (the navi framework, the hit kernel) and the hottest kinds (hitboxes).

### 3.3 Not built

- **WASM** (content in Rust compiled to wasm32, run by wasmtime or wasmi). The guest would be the Rust content
  above with `CoreApi` implemented over host imports; integer semantics are native and sandboxing is the
  strongest. If guests follow the same stateless rule, snapshots copy nothing; otherwise each snapshot copies the
  guest's linear memory (tens of KiB at least). The cost for modders is the Rust-plus-wasm toolchain, which is why
  it loses to Luau for the long tail. It remains the fallback if native-speed sandboxed content is ever needed.
- **Rhai.** Pure Rust, `i64` integers, can be built without floats, and script functions can't capture outer
  variables, so statelessness comes free. But it is a tree-walking interpreter (several times slower than Luau),
  with little tooling and an unfamiliar language for this community.
- **Lua 5.4** (dropped at the user's direction). Its integer subtype would remove the "no `/`" rule, but it lacks
  Luau's sandbox, type checker and speed.
- **Declarative timelines.** Many chips are "after N ticks spawn X; hit pattern P for M ticks". GunDelSol fits
  partly: its phases and durations are data, but the level- and sun-dependent beam, the EX region and the
  wrap-around timer quirks need code. The better form is a Luau library of combinators (phases, waits, per-tick
  hits) that simple chips use as data and complex ones mix with code, in one runtime.

### 3.4 Open Net Battle

Open Net Battle (ONB), the open Battle Network-style engine, scripts cards, characters and spells in Lua through
sol2. Content attaches callbacks to engine objects (`on_update_func`, `on_execute_func`, `add_anim_action(frame,
fn)` on card actions, and so on), and per-object state lives in the Lua heap: in closure upvalues and in tables
hung on entities. That is approachable, but it makes the Lua heap part of the battle state, so a snapshot would
have to serialize or fork the VM; ONB's network play is lockstep. The design here keeps ONB's authoring shape (a
module per card or entity, an update function per kind) and moves the state out of the VM, which is what rollback
needs.

## 4. GunDelSol, as content

The wind-up and firing phases, Luau (content/bn6/chips/gun_del_sol.luau):

```luau
-- The chip's GunDelSol data (its gun, firing time and beam), from the
-- content pack.
local function chip_data(me: Object): GunDelSol
    local chip = data.chips[me.chip]
    local d = chip and chip.gun_del_sol
    if not d then
        error(string.format("chip %#x runs GunDelSol without its data", me.chip))
    end
    return d
end

-- `sub_80EDB14`: the gun comes out; 6 ticks later the beam lights up.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me:set_animation(0x0A)
        me:open_counter_window()
        attachment.spawn(me, chip_data(me).gun.id, "overlay")
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
    local d = chip_data(me)
    s.timer = d.firing_ticks
    advance_gun(me)
    local look = if battle.navi(me.alliance).sun then d.beam_in_sun else d.beam
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
/// The chip's GunDelSol data (its gun, firing time and beam).
fn chip_data(data: &Data, api: &dyn CoreApi, me: ObjectRef) -> GunDelSol {
    use bn6_content_api::api::OtherFields;
    let chip = api.chip(me);
    *data.gun_del_sol.get(&chip).unwrap_or_else(|| panic!("chip {chip:#x} runs GunDelSol without its data"))
}

/// `sub_80EDB14`: the gun comes out; 6 ticks later the beam lights up.
fn wind_up(data: &Data, api: &mut dyn CoreApi, me: ObjectRef) {
    if api.step_init(me) == 0 {
        api.set_status(me, StatusFlag::UsingAction, true).expect("a navi has collision");
        api.set_animation(me, 0x0A);
        api.open_counter_window(me);
        let gun = chip_data(data, api, me).gun.id;
        attachment::spawn(api, me, gun, Slot::Overlay);
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
    let d = chip_data(data, api, me);
    store(api, me, State { timer: d.firing_ticks });
    advance_gun(api, me);
    let look = if in_sun(api, me) { d.beam_in_sun } else { d.beam };
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

Both follow the engine's built-in version (kinds/player/actions/gun_del_sol.rs) line for line. The numbers that
differ between the chip's levels (the gun, how long it fires, the beam in and out of the sun) are the chip's own
data in the content pack: `chip_data` looks them up by the chip being used (`data.chips[me.chip].gun_del_sol` in
Luau, `Data::gun_del_sol` in Rust) and fails loudly for a chip without them. The Luau is the shortest and needs no
casts: `s.timer = t` wraps because `timer` is declared `u16`, and `me.panel_x + 2 * me.facing` wraps into the `u8`
panel field. A pack's manifest registers its modules:

```luau
return {
    objects = { require("./objects/attachment"), require("./objects/sun_beam"), require("./objects/hitbox") },
    actions = { require("./chips/gun_del_sol") },
}
```

where an object kind is `{ pool, index, state, update }` and an action `{ action, state, update }`.

## 5. Fidelity

Golden traces, replayed straight by the golden-trace suite outside this repository, on this branch (main at
ca12dcb merged):

| Trace | Built-in | Rust content | Luau |
|---|---|---|---|
| machgun round 1 | 1074/1074 | 1074/1074 | 1074/1074 |
| machgun round 2 | 1331/1331 | 1331/1331 | 1331/1331 |
| soundmod rounds 1/2/3 | 4513/6284/2566 | 4513/6284/2566 | 4513/6284/2566 |

On the branch's original base (b2875dc, before the merge) the same held: 1074, 1330 of 1331 and 2952/3481/1966
for all three. To be sure the traces ran the content, each runtime was sabotaged (the Rust content made to panic;
the Luau GunDelSol's damage changed from 4 to 3): machgun round 1 then stops at frame 647, the first hit. Shifting
the Luau attachment by a pixel, or adding 1 to every Luau hitbox's damage, also breaks soundmod (rounds 1 to 3 stop
at 3296/6123/2368 and 3524/3852/2377). Traces don't compare sound, so a change to the sun beam's hum timing didn't
show.

In this repository, `every_runtime_plays_the_duel_like_the_engine` plays the 900-tick duel with each runtime and
compares, every tick, the engine state all of them represent alike (the digest with the kinds' own state structs
cleared, since their representation differs) and the tick's sound cues. Built-in, Rust, Luau and Luau native
agree on all 900.

## 6. Rollback

### 6.1 What the scripting layer guarantees

rollback.md §8.2 lists what content must guarantee. The prototype does each:

| Requirement | How |
|---|---|
| All content state in engine-owned typed storage inside `Battle`, snapshotted and digested; no VM heap, globals, closures or coroutines holding battle state | `ContentState` in `Vars`/`ActionVars` (`Hash`, `Copy`); the VM checked and frozen at load (§3.1); `Battle::digest` skips only the behaviors handle and the content, whose hash the round's setup carries (`RoundSetup::content`) |
| Integers only; one simulation RNG; no hash-map iteration | Integer-only boundary with wrapping stores; no `math.random`; the API hands out no maps |
| Outputs write-only | `battle.play_sound` only adds a cue; scripts can't read cues or looks back |
| Content immutable during a battle, identified by a hash both peers compare | Modules frozen after load; `Pack::content_hash()` and `Content::hash` (§9.3) |
| Bounded work that fails the same way on every peer | The interrupt budget counts VM checkpoints, not time |
| No perspective in simulated state | The API has no "local player"; sound goes to both sides' cue lists |
| Re-running a tick free of side effects outside `Battle` | Scripts can't reach the host; the VM keeps nothing |

### 6.2 Results

- **Golden traces through two rollback peers** (bn6-netplay; the golden-trace suite's rollback test, latencies
  0, 2+1, 5+2 and 10+3 frames): with Luau content, every confirmed frame of machgun rounds 1 and 2 and soundmod
  rounds 1 to 3 matches the trace on both peers, and the peers' digests agree with each other and a lockstep run.
  Rust content likewise.
- **Synthetic netbattles** (bn6-netplay's tests, run with `--features luau` or `rust-content`): two navis of the
  test content (`content::testing`) mashing buttons with its made-up chips, which use the same actions as
  GunDelS1/S2/S3, Invisibl and EraseMan (GunDelSol, the invisibility dimming chip, the eraser navi chip), three seeds,
  latencies 0 to 10 with jitter and input delay, up to about 2,000 rollbacks and 22,000 re-simulated frames per run:
  in sync to the KO in every configuration; sound plays each confirmed cue once.
  `the_battles_run_the_featured_content` checks the feature took effect.

### 6.3 Cost

The worst case, a 10-frame rollback on every rendered frame, measured as `soundmod_rollback_cost` does (over the
2000 frames around the round's busiest one: restore, 11 advances each followed by a save, a digest), with
`bn6-netplay/examples/rollback_cost`. Best of three runs:

| Per rendered frame | Built-in | Rust content | Luau |
|---|---|---|---|
| soundmod round 1 (frames 2525-4512) | 54.5 µs | 54.9 µs | 55.5 µs |
| machgun round 1 (whole round; three GunDelSols) | 53.8 µs | 55.1 µs | 150.8 µs |
| of which an advance, machgun | 0.49 µs | 0.66 µs | 9.2 µs |

The frame budget is 16,667 µs; the Luau worst case is 0.9% of it. The rest is the same for all runtimes: a
restore (3 µs), a save per advance (2.5 µs) and the digest (18 µs). Luau's 99th-percentile frame on machgun is
about 0.7 ms, when both navis fire at once.

## 7. Performance

`examples/content_bench`, on the synthetic duel (measured when the duel still ran on BN6's data; it now runs on
the test content):

| Runtime | Duel, 900 ticks | While GunDelSol fires | Per object-tick, 30 objects × 10,000 ticks | Snapshot | Restore | Snapshot + restore + 10 ticks |
|---|---|---|---|---|---|---|
| Built-in | 0.79 µs/tick | 0.88 µs/tick | 0.02 µs | 2.3 µs | 2.4 µs | 12 µs |
| Rust content | 1.05 µs/tick | 1.46 µs/tick | 0.10 µs | 1.9 µs | 3.2 µs | 26 µs |
| Luau | 23 µs/tick | 34 µs/tick | 4.9 µs | 1.9 µs | 2.0 µs | 421 µs |
| Luau native | 30 µs/tick | 31 µs/tick | 4.8 µs | 2.0 µs | 2.1 µs | 422 µs |

Snapshots cost the same whatever the runtime: the content's state is inside `Battle` and the VM is not copied. The
30-object case is 15 guns and 15 sun beams following their owner, each making about 20 API calls a tick: about 150
µs a tick, 1.6 ms for a frame with a 10-tick rollback, 10% of the frame. That is the budget to watch as more
content becomes scripts.

Where the time goes (`examples/luau_ops`):

| Operation | Cost |
|---|---|
| Loop iteration (the VM itself) | 10 ns |
| Library call `battle.dimmed()` | 39 ns |
| Method `me:param(1)` | 80 ns |
| Field read `me.anim` / write | 117 / 126 ns |
| Content state read `s.ticks` / write | 122 / 136 ns |
| Enum state read `s.slot` | 219 ns |
| A field that makes a handle (`me.pos`, `me.related1`, `me.sprite`) | 205 to 220 ns |
| `Vec3.new(1, 2, 3)` | 176 ns |
| A 9-field table literal (the hitbox spec) | 85 ns (9 ns native) |

The VM is fast; the binding isn't. mlua dispatches every userdata field through a Lua-side `__index` closure, then
its generic callback machinery (stack checks, argument conversion, a userdata type check). A sampling profile of
the firing window puts the interpreter at 18% of the time, allocation and GC at about 5%, the budget interrupt at
about 5%, and the rest in mlua's call path. Native code only speeds up the VM's share, which is why it doesn't help
here. (Method calls already use mlua's `__namecall` path, which cut them from 126 to 80 ns.)

The next step, if Luau is chosen, is a raw-FFI binding: tagged userdata (`lua_newuserdatatagged`) for handles, a
`__namecall` that dispatches on Luau's string atoms, and field access through a C `__index` that switches on the
atom instead of calling a Lua closure. That is how Roblox's own bindings work; 20 to 40 ns per call is a reasonable
target, 3 to 5 times less. Handles could also be cached per call to avoid allocations.

Loading the pack (VM, compiling, checking, freezing) takes about 5 ms, once per process.

## 8. Sandboxing

A pack can only call the API: no `io`, `os`, `debug`, no loading code at run time (`load`, `loadstring`, `require`
after loading), no FFI. Libraries and the global table are read-only (Luau's `sandbox`). Scripts can't reach the
host, can't keep state, can't hang the engine (the interrupt budget) and can't grow without bound unnoticed
(mlua's memory limit is available; a hit must be a fatal content error, since GC timing decides exactly when it
triggers). A content error (a script error, a type error at the API, the budget) stops the battle with a message
naming the module and object, identically on every peer; a production engine would end the round with an error
result rather than panic.

## 9. Scripts in the content pack

The project has since decided that game data comes only from content packs: the engine ships alone, BN6's pack is
extracted from the user's ROM into open formats (docs/design/content-pack.md), and the battle holds the loaded pack
as a `Content` value behind an `Arc`. That is done for the data. Scripts belong in that pack next to the data they
describe; that part is not done yet.

### 9.1 Layout

```
<pack>/
  content.toml                    manifest (name, format version); gains the engine API version and entry modules
  core.d.luau                     the API definitions the pack was written against (for editors and the checker)
  types.d.luau                    types the pack's modules share
  chips/010-gundels2/chip.toml    a chip's data, with its GunDelSol data: firing ticks, beam looks, gun (extracted)
  objects/attachment/object.toml  the attachment kinds (extracted)
  scripts/
    chips/gun_del_sol.luau        the action (0x37) and its helpers
    objects/attachment.luau, sun_beam.luau, hitbox.luau
    lib/slot.luau
  graphics/, sound/, ...          assets (extracted)
```

Today the scripts are still `content/bn6` in the repo, compiled into the engine, but the numbers they need come
from the content pack. Their data module, `data/pack`, is built from the battle's `Content` when the scripts load
(`behavior::luau_pack(&content)`, which renders it with `luau_data`); the repository only has its type stub,
`content/bn6/data/pack.luau`. GunDelSol reads its chip's entry, `data.chips[me.chip].gun_del_sol` (the firing
ticks, the beam's looks in and out of the sun, and the gun attachment with the id it is observable by); the
attachment and sun beam kinds read `data.attachments` and `data.sun_beam_looks`. The Rust slice gets the same
numbers as a `data::Data` built by `Behaviors::rust(&content)`. `Behaviors::for_build`, `luau`, `luau_with` and
`rust` all take the `&Content` the battle runs on. With one data source there is nothing left to cross-check (the
old tests that compared the scripts' data with the engine's tables are gone), and the in-repo duel that compares
the runtimes runs on the test content. What remains is loading the scripts from disk.

### 9.2 Found, loaded, versioned

- **Found.** The manifest names the entry modules (for BN6, one per chip action and object kind, or a single
  `battle/init.luau` returning the `objects`/`actions` lists as `pack.luau` does here). The loader compiles
  those and whatever they `require` (paths relative to the requiring file); nothing else runs.
- **Data.** Scripts read extracted data through the pack, as frozen tables: `local d = pack.data("battle/chips/
  gun_del_sol")` returning the data file as Luau values, typed by a declaration in `types.d.luau`. Numbers never
  live in scripts. (The prototype has one such module, `data/pack`, built by the engine from the loaded `Content`;
  §9.1.)
- **Loaded.** Each module is compiled, checked (`verify`), run once, and its result frozen (§3.1). The loader
  builds the manifest of kinds and actions and their state schemas; a kind's (pool, index) or an action number
  claimed twice is an error.
- **Versioned.** `core.d.luau` carries the API version; the manifest records the version it targets, and the
  engine refuses a pack for a different major version. Within a version, additions only.

### 9.3 The content hash

Before a match both peers compare one hash of everything the simulation reads: the scripts (path and source, as
`Pack::content_hash()` does for the prototype), the battle data files, and the simulation-relevant parts of assets
(sprite animation timing: frame durations and flags decide when effects end). Pixels, palettes and audio are
presentation and can be left out, so a player could use restyled sprites without desyncing. The engine build is
compared separately (the digest covers the state's layout). Mismatched hashes refuse the match with the differing
file names, rather than desyncing later.

The data's part exists: `Content::hash` covers the battle data and the animation timing (the loaded `Content`
holds no pixels or sound), the round's setup carries it (`RoundSetup::content`), and `Battle::new` checks it
against the content it is given, so peers whose setups agree run the same data. The scripts, compiled into the
engine for now, are covered by the engine build.

### 9.4 Where the VM lives

The pack's `Content` is shared behind an `Arc`, so it must be `Send + Sync`, and mlua's `Lua` is not (without mlua's
`send` feature, which adds a lock to every call). Statelessness settles it: the `Arc<Content>` holds the compiled
bytecode, the manifest and the schemas, and each thread that runs battles creates its own VM from them on first
use. Every VM made from the same pack behaves identically (the fresh-VM test in §3.1), so VMs are a per-thread
cache, not part of any battle.

The engine already keeps the two apart for the data: `Battle::content` is the `Arc<Content>`, and the scripts'
runtime is a separate handle, `Battle::behaviors` (an `Rc`). `Behaviors::for_build` loads the Luau scripts once per
thread and content hash and hands every battle on that thread a clone of the handle.

### 9.5 The hand-written BN6 scripts

The BN6 scripts are not ROM-derived: they are this project's reimplementation of the game's routines, like the
engine's Rust. They describe BN6 content, so they belong in the BN6 pack, but they should not be generated
(there is nothing to generate them from) and must not be committed alongside ROM data.

Recommendation: **ship them in the repo as a source overlay that the extractor merges into the extracted pack.** The
overlay is `content/bn6/` (scripts, `types.d.luau`, the manifest template), versioned and reviewed with the engine,
type-checked in CI by `bn6-content-check`, and covered by in-repo tests against small hand-written data (in-repo
tests can't use the ROM; the engine's test content, `content::testing`, is that data today). The extractor writes
the ROM-derived data and assets, copies the overlay in, and stamps the manifest with the overlay's version and the
pack's content hash. The frame-exact tests run in the verify workspace, on the extracted pack. A modder's pack is
the same shape, without the extractor.

## 10. Recommendation and next steps

1. **Adopt the content API and the stateless-content rule** as the boundary, whatever the runtime: declared
   schemas, engine-owned typed state, immediate effects, raw-slot handles. It costs the engine nothing (snapshots
   and the digest are unchanged) and it is what makes rollback work with scripted content.
2. **Write the ruleset and the hottest kinds in Rust against the API**, starting where the audit's migration plan
   starts (hitboxes, effects, attachments); the Rust-content path costs about 1.5 times the built-in code.
3. **Write chips, navis and effects in Luau** in the pack (§9), with the load-time rules, versioned
   `core.d.luau`, and the type check in CI.
4. Before much content moves: the raw-FFI binding (§7), scripts loaded from the pack on disk with their compiled
   form in the `Arc<Content>` (§9.4), `pack.data` per data file (§9.2; the prototype builds one data module from
   the `Content`), the scripts in the content hash (§9.3), a generator for state schemas from the Luau types, and
   a content error that ends the round instead of panicking.
