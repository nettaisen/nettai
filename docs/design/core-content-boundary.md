# Core, ruleset and content: where the boundary is

Chips, attacks, navis' own actions and the objects they spawn are content that sits on top of an engine that
names none of them. This document says where the line runs: what each layer is, what the engine holds and what
it never holds, how the two sides reach each other, the game's semantics that content relies on, and the cases
that resist a clean split, as they are built. The content root and the pack are in
docs/design/content-pack.md, the runtime and the API in docs/design/scripting.md, how to write content in
docs/design/content-migration.md, rollback in docs/design/rollback.md.

Routine names are the original's (`sub_80EDAE0`). "Tick" is one call of `Battle::tick`.

## 0. Summary

**Three layers, two boundaries.**

- **Core**: mechanism with no BN6 rules in it. Object pools and the update list with the game's insertion and
  freeing semantics, pause and dimming gating, the typed state store, collision registration on panels and
  pair iteration, the panel grid with reservations and cached flags, 16.16 positions, the animation stepper,
  RNG, input records, the sound and look output channels, snapshots and the digest.
- **Ruleset**: BN6's battle rules, in Rust on the core. The hit kernel and damage pipeline, element weakness,
  guard, counters, statuses, the navi framework (request decoding, charge, the idle controller, `set_attack`
  and exit, hit reactions, deletion), the chip hand and chip use, the custom screen and gauge, the flow state
  machines, panel types, the obstacle framework, the dimming service, the navi-chip controller, the forms
  framework (the turn-start sequencer, the pause-time actions, the Beast rush wrapper).
- **Content**: Luau definitions. Chips and their uses, actions, weapons, object kinds, navis, forms, stages,
  rule sections, effects, sparks, regions, collision types, statuses, lock-on modes, identities.

The core and the ruleset are one crate (nettai-battle) and are not separated by an interface: the first boundary
is a discipline (core modules take the rules they need as data: collision types, panel flags, animation
timing). The second boundary is hard: content sees only the content API (`CoreApi`, declared in core.d.luau),
and the engine sees content only as handles into registries and function slots it calls.

**What the engine never holds**: a chip, kind, action, weapon, effect, spark, region, collision type, status,
lock-on mode or identity by the original's number, or by its key. It has handles; what it needs by meaning it
asks by role. The original's numbers are compat's (content/bn6/compat), which only tools read: the engine
doesn't depend on bn6-compat (a test guards it), and content can't load it.

**What content never holds**: battle state. Its state is typed fields the engine stores inside `Battle`;
scripts are stateless functions over it. So a battle is a plain value: cheap to snapshot, exact to restore.

## 1. The layers

| Layer | Belongs here | Test |
|---|---|---|
| Core | Pools, list and update order, gating, the state store, collision registration and pairing, the panel grid, fixed-point geometry, animation stepping, RNG, input plumbing, output channels, snapshots | Would another Battle Network-style game need it unchanged? |
| Ruleset | BN6's generic battle rules: damage, elements, guard, counter, statuses, the navi framework, chip hand, custom gauge, turn and round flow, the forms framework, the services | Does it apply to every chip, navi or form rather than to one? |
| Content | Specific chips, actions, object kinds, navis, forms, effects, stages, tables | Does it name one? |

### 1.1 The engine, module by module

crates/nettai-battle/src:

| Module | Layer | What |
|---|---|---|
| object/, rng.rs, input.rs, cues.rs, sound.rs, digest.rs, rollback.rs | core | Pools and the update list, the animation stepper and `Look`, the RNG, key records, the cue channels, the digest and snapshots |
| collision.rs | core, ruleset | Core: the slot pool, per-panel masks, present and remove, pair iteration, region expansion. Ruleset: the hit kernel (state filters, guard, air and ground, invulnerability, status, counter, multiplier), damage-word decoding |
| field.rs | core, ruleset | Core: the grid, cached flags, reservations. Ruleset: panel types, stolen-area return, the field-object registry, step rules |
| battle.rs, hud.rs, link.rs, console.rs, perspective.rs | ruleset (the tick skeleton is core) | Round state, the mode handler, the fighting machine, results, the gauge, banners, the per-tick system order, the link exchange, the consoles' own RNG |
| actor.rs, hand.rs, transform.rs, setup.rs, custom/ | ruleset | Actor data and the attack header, the chip hand, the turn-start transformation sequencer, the round's setup, the custom screen |
| dimming.rs | ruleset service | The per-side dimming records, the telop, the cut-in, AntiNavi |
| kinds/common.rs | core | Sprite-step wrappers and their gating, panel and coordinate conversion |
| kinds/hitbox.rs, effect.rs, spark.rs, afterimage.rs, palette_flash.rs, form_overlay.rs, body_overlay.rs, ... | ruleset primitives | The engine's own object kinds (keyed `engine/...`): what content's calls spawn (`battle.hitbox`, `battle.effect`, `battle.spark`, `battle.afterimage`) and what the navi framework spawns itself |
| kinds/obstacle.rs, navi_chip.rs, heal.rs | ruleset services | The obstacle framework, the navi-chip controller, recovery |
| kinds/player/ | ruleset | The navi framework: spawn and init, input, intake, status, reactions, idle, chip use, and the ruleset's own actions (the step, the three chip-use actions, the form change, the Cross change and special, the Beast rush, the reactive counters' start) |
| content/ | the content's typed form | `Content`: the registries the definitions build (`defs`), the roles, the typed records the ruleset reads (chips, navis, forms, stages, rules) |
| behavior/ | the boundary | `impl CoreApi for Battle` and the dispatch of kinds, actions and hooks to the runtime |

The other crates: nettai-content-api (the contract below both sides), nettai-luau (the runtime), nettai-content (reading
a content root and a pack), nettai-content-check (the type check, lints and guards), bn6-compat (the original's
numbers, the setup codecs, the trace harness), bn6-extract (the pack's assets from a ROM), nettai-assets,
nettai-frontend, nettai-audio and m4a (presentation), nettai-netplay (rollback).

## 2. How the two sides reach each other

### 2.1 The engine reaches content

- **By definition.** A registry entry is a definition with a handle. An object records its kind's handle and
  the engine runs the kind's `update`; a navi records the action it runs (`NaviAction::Content(handle)`) and
  the engine runs its `update`; a hand holds chip handles, and chip use reads the chip's record and starts its
  use: an action, or a hook the ruleset's dimming, navi chip or instant chip action calls. A form or a navi
  names its weapons, and a button runs the weapon's `setup`. A stage names the kinds it places, and the kind's
  `place` runs.
- **By role.** Where the ruleset itself must start, spawn, show or recognize something, it asks
  `content.defs.roles`, which rules/roles.luau fills once: the actions a request starts (the trap chips'
  counters, the forced charged shot, the stun strike, the turn), the actions it recognizes (the charged sword
  and the Beast claw for the rush's lock-on mode, DustCross Beast's scatter, ChargeCross's tackle), the kinds
  it spawns (the absorbed obstacle, the falling rock, the supports' controller, AntiRecv's counterattack), the
  chips it names (what a zeroed chip field reads, the custom screen's Beast Out chip, the supports' telops),
  the statuses, lock-on modes, effects, sparks, regions, collision types, sounds, music, banners and sprites
  it uses itself, and two hooks (the FirstBarrier, an encased obstacle). A role content hasn't filled is an error where the ruleset needs it.
- **By trait.** What the ruleset asks of every chip, weapon or action beyond its record is a named property of
  the definition: a chip's `traits` (`no_chain`, `aura_bonus`, `heals`, `navi_slot`, ...) and `trap`, a
  weapon's `sticky`, `held`, `plain`, `charged_chip`, an identity's class. The ruleset never tests a key.
- **By state field.** Rust that spawns a content kind for a role sets its state by field name
  (`set_state_field`), so the kind has one implementation and no parameter bytes.

### 2.2 Content reaches the engine

Through the content API only (scripting.md §2): fields of objects, sprites, collisions and navis by name; the
`battle` and `field` libraries; the ruleset's services (`dimming`, `navi_chip`, `obstacle`); the definers and
asset resolvers while it loads. Everything it passes is a definition, an asset, a handle, a name or an integer.
Nothing in the API names a particular chip or kind.

### 2.3 Identity and the traces

The golden traces compare the original's numbers: an object's pool and index, a navi's action number, a chip
id. The engine has none of them. bn6-compat maps what the engine runs back to them by key
(`Compat::object_slot`, `Compat::navi_action`, the codecs' `Ids`): compat/kinds.toml gives each kind's slot (the
engine's own kinds included), actions.toml each action's number, chips.toml each chip's id. A kind's
register-garbage position is declared there too (`scratch_position`), for the comparison to skip.

## 3. The semantics content relies on

These are the game's, kept bit-exact; the API exposes them and must not smooth them over.

- **Handles are raw slots.** An `ObjectRef` is a pool and a slot, not generational. It can outlive its object
  and then names the slot's next occupant, as the game's pointers do. The core never panics on a stale handle.
- **Effects are immediate.** Every call changes state at once; nothing is deferred to the end of the tick.
- **Spawning.** A spawn takes the lowest free slot of the kind's pool; a full pool gives none, and callers carry
  on. The new object goes into the update list right after the object updating, so it runs later this tick;
  several spawns from one update run in reverse spawn order. With no object updating, or when the current
  object freed itself and the new one took its slot, it goes to the tail. `spawn_first` and `spawn_at_end` are
  the game's two other insertions. A content kind starts with its state zeroed and its lifecycle at init.
- **Freeing** clears the slot and unlinks the node; the freed node's own links stay, and the loop reads the
  current object's successor after its handler returns, so an object may free itself or its successor. Free
  releases nothing else: collision, reservations and actor data are `destroy`'s (`object_genericDestroy`).
- **The loop and its gates.** The loop samples an object's flags before its handler, skips it while paused
  unless it runs while paused, and while dimmed unless it runs while dimmed. A dimming that starts in the
  middle of the loop gates the objects after it in the same tick.
- **State words.** An object's lifecycle, action, phase and phase-init are observable and keep the original's
  values (jump-table offsets 0, 4, 8, ...). `me:set_lifecycle` is the game's word store (all four),
  `me.lifecycle =` its byte store. A navi's action is a definition, not a byte; its phase is the attack's
  `step` and `step_init`.
- **Timers** count down as each routine writes them: the test is on the new value, the widened new value or
  the old value, and the store wraps to the field's width. Content keeps the routine's compare.
- **Collision.** Present registers the region's panels and clears the last results; remove unregisters and
  pair-tests against what is registered there, both directions reacting; a hit resolves when the later of two
  objects removes its collision, so damage lands this tick or next by list order. A freed slot keeps its mask
  bits until removed. The one-tick hitbox presents and removes at once in its spawn tick.
- **Panels.** A panel's flags word is cached and refreshed only where the game refreshes it; cracking and
  breaking edit it in place. A reservation succeeds only if nobody holds the panel.
- **Sprites.** A frame of duration d is current for exactly d updates. Which stepping routine an object calls
  (`update_sprite`, `update_sprite_while_dimmed`, `update_sprite_while_paused`, `step_sprite`) is part of its
  behavior. Animation timing is simulation data (lifetimes end on frame flags); pixels and the look are
  output only.
- **RNG.** One simulation stream, drawn in object update order through `battle.rng`, `rng_positive` and
  `jitter`. A content change that adds, removes or reorders a draw changes everything after it.
- **Input.** The flow latches the pads before objects run; a navi's buttons are its record, and a second
  record is kept while dimmed.
- **Integers.** Fixed widths, wrapping stores, truncating division; no floats.

## 4. The hard cases, as built

### 4.1 Frame ordering

Spawn order, list order, RNG order and the tick a hit lands on all follow from §3. The core owns the list;
content influences order only by spawning. The per-tick system order (the mode handler, objects, panels, hand
exposure, gauge and banner, the linked registry, variable damage, cycle counters, damage carry, the custom
drain) is the ruleset's, written as one list in the battle's tick (`Battle::tick_running`).

### 4.2 The attack state shared between actions

The game's AIAttackVars is one persistent block per actor: `set_attack` clears a few bytes, chip use fills
others, the rest keeps what the last action left. Here it is a typed header the ruleset owns (`AttackVars`:
step, element, damage, hit parameter, charged, lockout, bonus, the attack's chip, the kind of start, the rush's
state, and the words that outlive an action: the marker, a thrown obstacle's look, the shot recovery) and the
action's own state: a ruleset action's is a variant of `ActionVars`, a content action's a `ContentState` of its
declared layout. Actions that share a state table continue each other's state, as the original's actions of
one routine do; an action of another layout starts from zero. A weapon's setup writes the state of the action
it is about to start. The original's variant and parameter bytes, which chip use copied from the chip's record
for the action to read, don't exist: what an action needs of its chip is its builder's arguments.

### 4.3 Beast Out's wrapper around arbitrary chips

When a chip with a lock-on mode is used in a Beast form, BN6's beast system marks the attack `wrapped` as its use
starts (its `chip_used` hook), and the dispatcher then routes every tick through the side's wrapper (the role
`actions.wrapper`: content/bn6/rules/beast/rush.luau) instead of the chip's action: it holds the panel, warps
next to the target marker's target by the chip's lock-on mode (a "lockon" record), runs the chip's action from
its own phase (`navi:run_wrapped()`), watches for it to end, then chains the next chip or warps back. It is the
ruleset's, and wraps whatever action content defines; a chip says how it is wrapped in its record (`beast = {
lockon, rush }`) and its `no_chain` trait. Two actions ask for another mode than their chip's: the system
recognizes them (the Beast claw's and SlashCross's charged sword), and one gives its mode itself
(`rush_lockon`). (rules-in-luau.md, As built S3.)

### 4.4 Content reaching into other objects

GunDelSol steps its gun's animation; attachments and overlays copy their owner's position, visibility and
look; a kind that lasts as long as its owner's action compares `owner:navi_action()` with the action it was
sent in; the absorbed obstacle appends to its navi's absorbed list. One world, one thread: a script reads and
writes another object through its handle, its engine fields by name and its content state as `other.state`
(cast to that kind's type). A raw halfword store that sets two fields in the original is the two named
fields.

### 4.5 Global battle state that content changes

Navi stats (mood, the buster's levels, the shoes), the emotion, the hand's cursor and bonuses, the
defensive-chip record, the damage-carry record, the custom gauge, side statistics, the wind, the field-object
registry: each is the ruleset's, changed through a named operation of `battle`, `field` or `obstacle`. Content
declares no global state of its own; what a side keeps for one chip family (the linked record) is a ruleset
structure the chips fill.

### 4.6 Raw observable state and register garbage

The traces observe the state word, header flags, timers and positions. Some positions are whatever the
spawner's registers held. Content reproduces the value where it is knowable (a spawner passing `(panel.y,
element, z)` as the position; `battle.loop_register()` for the update loop's leftover; `me:set_header_flags`
for a byte read from the wrong address); where it is an address, compat marks the kind `scratch_position` and
the comparison skips it. Garbage the ruleset produces (a decoded bug's high byte, which a collision type's
`row_offset` gives) stays in the ruleset. The engine never exposes addresses.

### 4.7 The pause handler

While the battle is paused, a navi's action dispatch is replaced by the pause handler: it runs a pause-time
action by state bit (the form change, the revert, the Cross change, the Cross knock-out) or starts one from a
request. The revert, the Cross change and the knock-out are the framework's (kinds/player/actions/transform.rs,
cross_change.rs); the change into a form is the action the form names (`FormData::change`: BN6's forms system's,
content/bn6/rules/forms, docs/design/rules-in-luau.md), with the effects
they show by role. Only objects that run while paused run, so what a pause-time action spawns sets that flag.

### 4.8 Counters

An attack opens a 16-tick window on its user (`me:open_counter_window()`; players get it only in link
battles). The kernel marks a counter when the receiver's window is open and the hitter's counter byte has
strength without bit 7; the intake turns it into the counter's paralysis (a status by role), counts it and
closes the window. Content's two inputs are the counter byte (the chip's `hit_param`, or the action's own) and
the tick at which the action opens the window.

### 4.9 Handles that alias

Freed collision slots keep their mask bits and fields; a deleted player keeps its collision handle, so once
another object reuses the slot, the dead player's "status" is that object's. Generational handles would be
safer and would change behavior the traces observe. A handle's meaning is "whatever is in that slot now",
which is the game's meaning.

### 4.10 Dimming and the cut-in

A dimming chip's controller is content (a kind whose update calls the `dimming` service's steps in its
routine's order); the service owns the per-side records, the flag, the telop and the wait for a cut-in. The
dimming starts in the middle of the object loop, so later objects that don't run while dimmed are skipped for
the rest of that tick. A cut-in prepares the cutting-in side's chip into a detached attack header, so the
navi's own attack and action are untouched, and the two sides' controllers wait on each other. A navi chip's
controller is the ruleset's (kinds/navi_chip.rs): it brings the chip's navi through the chip's `navi` hook and
waits for `navi_chip.navi_left`.

### 4.11 Content the ruleset knows

The ruleset names content only by role, trait and class (§2.1). What it still knows by number is listed in §6.

### 4.12 View dependence

Some state depends on which console the engine plays: the local navi appears at once while the remote one
queues for a fade-in, a blinded viewer doesn't see the other side's objects, a hit sounds differently for the
local player. Both netplay peers simulate from one perspective (`RoundSetup::local_side`, part of the shared
setup), so the state is the same on both, and each presents it for its own player (perspective.rs: the cues
each side hears, the telops and banners per viewer, what each player sees of the objects). `battle.local_side()`
is for presentation and for these known differences only; nothing feeds it back into decisions. What a
console's rule hides (a blind player doesn't see the other side's objects) content decides per viewer with
`me:hide_from_blind()`, and an attachment follows its owner's visibility with `me:copy_visibility(owner)`.

## 5. Determinism and snapshots

The rules a content layer follows (scripting.md §3.1 and §6 say how each is enforced):

1. **All simulation state is engine-owned plain data.** Content declares it; the engine stores, clones and
   hashes it. A runtime's heap, globals, closures or coroutines hold no battle state between calls.
2. **Integers only**, fixed widths.
3. **One RNG stream for the simulation**, drawn only through the core, in the order the calls happen.
4. **Order is list order or slot order.** No hash-map iteration, no sorting by address.
5. **No addresses.** Handles are slot indices; garbage the game takes from addresses is declared unknown.
6. **Content is immutable during a battle** and identified by a hash that peers compare. State refers to
   content by handle, never by reference.
7. **Outputs are write-only.** Sound cues and the look are never inputs to a decision.
8. **Bounded work.** A script that exceeds its budget fails the battle the same way on every peer.
9. **Perspective stays in its lane** (§4.12).

All simulation state is in `Battle`: the round state, the fighting machine, the gauge, banners, input records,
both hands, the transformation requests and sequencer, the object pools (header, typed kind state, sprite
state), the actors, the collision slots, the field, the dimming records, the RNG. Content state is a
`ContentState` per object or action: the layout's id and 64 bytes its named fields are packed into, a `Copy`
value. `Battle` is `Clone`, and a clone is the snapshot; `Battle::digest` hashes it, leaving out the content
(whose hash the round's setup carries) and the presentation-only parts. Resimulated ticks produce sound cues
again; the frontend plays a cue once it is confirmed or first predicted and cancels mispredicted ones
(rollback.md).

## 6. Where the line isn't clean yet

- **The post-init hook by AI index.** `sub_800F378`'s battle-mode-9 spawns are a Rust match on the actor
  record's AI index (10); the flinch, drag and overlay-refresh hooks and the other tables by AI index are the
  identity's (`parts`, `overlay_hooks`, `aura_anim`, `ice`: content-model-v2.md §3.2), and navis and forms are
  definitions by handle whose fields and traits the ruleset asks.
- **The engine's kinds' spawn parameters.** The engine's own object kinds (the effect, the spark, the hitbox,
  the afterimage, the eruption) still read the four parameter bytes they are spawned with; content kinds have
  none.
- **Three unfilled roles**: the action the volley request starts (`actions.volley`; no routine raises the
  request), and the objects battle mode 9 spawns for AI index 10 (`kinds.mode9_attack`, `kinds.mode9_actor`); no
  content defines them, and starting one is an error naming the role.
