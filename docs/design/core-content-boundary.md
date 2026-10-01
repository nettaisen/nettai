# Core, ruleset and content: where the boundary goes

This is an analysis of the engine as it stands (crates/bn6-battle and its companions), made to answer one
request: chips, attacks, actions, navis and the rest should be content that sits on top of a core that knows
nothing about them, with graphics and audio in formats other tools can read. It covers what is core today and
what is not, the API content would be written against, how each kind of content would be defined, the cases
that resist a clean split, what determinism and snapshots require of a content layer, and an incremental path
from here to there that keeps the golden-trace results intact at every step.

It changes no engine code. The formats for content files and the scripting runtime are being prototyped
separately; this document fixes the boundary those prototypes plug into.

The analysis is kept as written: its line counts, file names and "today" describe the engine at the time. Since
then, step 9 of the migration plan (§6) is done and decision 2 (§7) is decided: the battle data comes from a
content pack, loaded at run time into `Content` (docs/design/content-pack.md). There are no generated Rust tables
and no embedded BN6 pack; the engine has no default content, and BN6's pack comes only from the user's ROM. Where
the inventory below says "generated", read: then, generated Rust tables; now, the pack's battle data.

Step 10 has begun, in its own form: content is Luau, the only runtime, in the content pack beside its data and
registered by it (docs/design/scripting.md), and a script replaces its Rust version outright, the golden traces
being the reference. The ruleset stays Rust (decision 1, as recommended). The first content moved: GunDelSol,
AreaGrab and PanelGrab, EraseMan, MegaMan's buster weapons with the blank shot, and DustCross's charged shot, with
the object kinds they spawn. docs/design/content-migration.md is the plan for the rest, and its checklist supersedes
§6's order where they differ. The API is the `CoreApi` trait (§2's `Ctx`, with the dependency inverted), and a
kind's register-garbage position is declared in its `[kind]` table (`scratch_position`, `scratch_z_fraction`;
decision 4 kept the garbage that can be reproduced).

Routine names are the original's (`sub_80EDAE0`). "Tick" is one call of `Battle::tick`.

## 0. Summary

**Three layers, two boundaries.** The engine splits naturally into:

- **Core**: mechanism with no BN6 rules in it. Object pools and the update list with the game's insertion and
  freeing semantics, pause and dimming gating, the typed state store, collision registration on panels and
  pair iteration, the panel grid with reservations and cached flags, 16.16 positions, the animation stepper,
  RNG streams, input records, the sound and look output channels, and snapshots.
- **Ruleset**: BN6's battle rules, written in Rust against the core. The hit kernel and damage pipeline,
  element weakness, guard, counters, statuses, the navi framework (request decoding, charge, the idle
  controller, `set_attack`/exit, hit reactions, deletion), the chip hand and chip use, the custom gauge and the
  flow state machines, panel types, the obstacle framework, the dimming service and the forms framework
  (turn-start sequencer, pause-time actions, action wrappers).
- **Content**: data plus small scripts. Chips, attack actions, object kinds, navis, forms, effects, stages.

Content is written against one API, a `Ctx` handle that exposes core and ruleset operations (§2). Content state
lives in engine-owned typed storage that content declares a schema for; scripts are stateless functions over it
(§5.4). That keeps a whole battle a plain value: cheap to snapshot, exact to restore, which rollback netplay
needs.

**Sizes today** (bn6-battle, 14,664 lines; split by function, so approximate):

| Layer | Lines | Share |
|---|---|---|
| Core | ≈ 1,650 | 11% |
| Ruleset | ≈ 7,050 | 48% |
| Content, code | ≈ 3,300 | 23% |
| Content, data (1,474 in generated tables, then; 463 schema) | 1,937 | 13% |
| Tests and trace harness | ≈ 750 | 5% |

The ruleset is large because the PvP navi framework is nearly complete, while very little content is ported: 4
of 411 chips have their behavior (GunDelS1/2/3/EX), 15 object kinds (the player included) of the several hundred
the original has, one navi (MegaMan) and one form (Falzar Beast Out). Porting the rest of the game is
overwhelmingly content work, which is what makes the boundary worth drawing now.

**The hardest cases** (§4): same-tick spawn and list-order effects; the attack scratch that persists between
actions; Beast Out's wrapper around arbitrary chips and chip chaining; content reaching into other objects'
state; global battle state that content mutates (including dimming set mid-tick); the pause handler; raw
observable state (byte-offset phase numbers, register-garbage values); counters and cut-ins; handles
that alias after a slot is reused; and content ids hard-coded throughout the ruleset.

## 1. Inventory

### 1.1 The layers

| Layer | Belongs here | Test |
|---|---|---|
| (a) Core | Pools, list and update order, gating, the state store, collision registration and pairing, the panel grid, fixed-point geometry, animation stepping, RNG, input plumbing, output channels, snapshots | Would another Battle Network-style game need it unchanged? |
| (b) Ruleset | BN6's generic battle rules: damage, elements, guard, counter, statuses, the navi framework, chip hand, custom gauge, turn and round flow, the forms framework | Does it apply to every chip, navi or form rather than to one? |
| (c) Content | Specific chips, actions, object kinds, navis, forms, effects, data tables | Does it name one? |

A module is "mixed" when it contains more than one layer, for example ruleset code that switches on a specific
form number, or core code that reads a content table through a global (then a generated Rust table).

### 1.2 bn6-battle, module by module

Lines are exact (`wc -l`); splits are by function and rounded.

**Crate root and core modules**

| Module | Lines | Layer | Notes |
|---|---|---|---|
| lib.rs | 30 | core | |
| rng.rs | 45 | core | The step function; both streams. Only the simulation stream is modeled. |
| input.rs | 46 | core | Key bits and held/pressed/released records. |
| object/mod.rs | 411 | core | Pools, lowest-free allocation, the linked update list, `spawn` (after current, else tail), `free` (unlink, own links kept), the loop cursor. **Mixed:** `Object` carries ruleset fields (hp, max_hp, name_id, chip, chips_held, damage, stamina, element, the slide fields, drag_step, shake_*, saved_state), about 35 lines. |
| object/sprite.rs | 153 | core | The animation stepper and `Look`. **Mixed:** reads frame timing from a global (then the generated `SPRITES` table; now the battle's `Content` passes it in). |
| sound.rs | 119 | core (≈45), content (≈20), tests (≈54) | The cue channel is core; the named `SoundId` constants are content. |
| battle.rs | 1,117 | ruleset (≈900), core (≈150), content (≈70) | Core: `tick`, `run_objects` (flags sampled before the handler, successor read after), the pause and dimming gates, the sound buffer, `Fade`. Ruleset: round state, the mode handler, the fighting machine, results, custom screen, combo, battle time, turn timer, gauge, hand exposure, damage carry, NaviCust drain, low-HP music, and the order of the per-tick systems. **Mixed:** `count_down_beast_out` (MegaMan and Beast forms), `custom_open_requested` and `apply_actor_inputs` (Beast Over), music and banner ids, per-navi win/lose banners, `spawn_actors` naming the rock kind, the drain period table. |
| collision.rs | 601 | core (≈260), ruleset (≈340) | Core: the slot pool, per-panel masks, `present`/`remove`, pair iteration and dedup, region expansion. Ruleset: the hit kernel (`resolve_hit`: state filters, guard, air/ground, invulnerability, status, counter, multiplier), the raw channel, damage-word decoding, panel conversions. **Mixed:** reads `REGIONS`, `FIELD_REGIONS`, `COLLISION_TYPES` and `ELEMENT_WEAKNESS` as globals; aqua-on-ice and heat-on-grass are panel-type rules inside the kernel. |
| field.rs | 601 | core (≈230), ruleset (≈340), content (≈30) | Core: the grid, cached flags, refresh, reservations, `check`. Ruleset: panel types (holes, cracks, roads, volcano), stolen-area return, the field-object registry, step rules. **Mixed:** `erupt` spawns attack object #7 with 50 damage; `crack_panel` plays sound 0x97. |
| actor.rs | 319 | ruleset (≈200), content (≈80), core (≈40) | The actor-data pool is core. Pad, charge, requests, status bits and the attack header are the navi framework. **Mixed:** request and status bits named for chips (AntiDmg 0xBB, AntiSwrd 0xBC, BodyGrd 0x157) and forms; fields for one chip or form (`absorbed`, `lockon_marker`, `full_synchro_aura`, `beast_over_exhausted`). |
| hand.rs | 125 | ruleset (≈110), content (≈15) | The chip hand; damage formulas are a content hook. |
| hud.rs | 99 | ruleset | Gauge and banner lifetime. Rate and banner-hold tables are data. |
| transform.rs | 200 | ruleset (≈190) | The turn-start transformation sequencer (forms framework). |
| setup.rs | 509 | ruleset (≈300), content (≈150), tests (≈60) | Settings and the NaviStats model and codec are ruleset. **Mixed:** `ActorKind::Rock`/`Object6E`/`Object7D`, the `Navi` and `Form` id constants, the NaviCust bug fields. |
| trace.rs | 372 | harness | Trace codec and comparison. **Mixed:** knows which kinds have garbage positions by pool and index (`pos_is_garbage`). |

**Content data** (`data/`, at the time; now `content/`, with the tables loaded from a pack)

| Module | Lines | Layer | Notes |
|---|---|---|---|
| mod.rs, attacks.rs, lockon.rs, player.rs | 463 | content schema | Row types (`ChipData`, `RockKind`, `EffectSprite`, `AnimFrame`, ...) and typed lookups. |
| generated tables (11 files) | 1,474 | data | Written by bn6-extract then; now the pack's battle data. `CHIPS` (411), `SPRITES` (298 sprites' animation timing), `EFFECTS`, `SPARKS`, `ATTACHMENTS`, `ROCKS`, `ACTOR_LISTS`, `PANEL_LAYOUTS` (237), the player/form/navi tables, and ruleset tables (`COLLISION_TYPES`, `REGIONS`, `ELEMENT_WEAKNESS`, `STEP_RULES`, `STATUS_EFFECTS`, vectors, `CHARGE_THRESHOLDS`). |

**Object kinds** (`kinds/`, 2,724 lines without the player)

| Module | Lines | Layer | Notes |
|---|---|---|---|
| mod.rs | 134 | content registry (≈64), ruleset (≈40), core (≈30) | The kind dispatch is two `match (pool, index)` statements; `Vars` is a closed enum of every kind's state. |
| common.rs | 176 | core (≈140), ruleset (≈36) | Sprite-step wrappers and their gating, panel/coordinate conversion, `Progress`. Damage totals and guard sparks are ruleset. |
| hitbox.rs (T3#3) | 134 | ruleset stdlib | The one-tick hit region nearly every attack uses. |
| effect.rs (T4#0), spark.rs (T4#4) | 179 | ruleset stdlib | Generic effects, configured by the `EFFECTS`/`SPARKS` tables. `jitter` is the RNG helper. |
| intro.rs (T4#2) | 47 | ruleset | Flow object. |
| attachment.rs (T1#5), palette_flash.rs (T4#0x0A) | 227 | ruleset stdlib | Configured by `ATTACHMENTS` and spawn parameters. |
| charge_glow.rs (T4#8) | 147 | ruleset | Every player gets one at init (`sub_80172F0`). |
| form_overlay.rs (T1#0x57) | 159 | ruleset (≈120), content (≈39) | Forms framework visual; `BEAST_HEAD` is content. |
| obstacle.rs | 479 | ruleset | The obstacle framework: a trait with per-kind `appear`/`destroyed`/`idle`. The existing precedent for a content interface. |
| sun_beam.rs, afterimage.rs, lockon_marker.rs | 455 | content | GunDelSol's beam; Beast Out's afterimages and lock-on marker. **Mixed:** afterimage switches on NameID ranges. |
| rock.rs, rock_debris.rs, absorbed_obstacle.rs | 492 | content | Rock (T3#0x59) on the obstacle trait, its debris, the absorbed-obstacle flyer. |
| rock_tests.rs | 95 | tests | |

**The player navi** (`kinds/player/`, 5,256 lines, 36% of the crate)

| Module | Lines | Layer | Notes |
|---|---|---|---|
| mod.rs | 872 | ruleset (≈560), content (≈250), core (≈60) | Spawn, init, the per-tick pipeline, `set_attack`/`exit_attack_state`, status reset, weapons, element. **Mixed:** MegaMan (NameID 0x1A0 + form), per-AI-index hook lists that panic, `per_form_tick` (form 0x18 height), `navi_palette`, `style_hook`, the Beast Out counter. `update_sprite` duplicates `common::update_sprite`. |
| input.rs | 312 | ruleset (≈260), content (≈50) | Buttons to requests, charge. **Mixed:** `chip_charges` is a form×family table written as code; buster types 3/4/0x2C. |
| intake.rs | 708 | ruleset (≈550), content (≈160) | Stage A of the damage pipeline. **Mixed:** barrier types, trap chips 0xBB/0xBC/0x157, bug codes, status immunity for navi 7 and forms 7/0x13. |
| status.rs | 715 | ruleset (≈560), content (≈150) | Stage B, the action dispatch, statuses, the pause handler, dimming. **Mixed:** Cross knockout and lanes (NameID 0x1AC..0x1C1), Beast Over exhaustion, the Beast rush hook in `dispatch`. |
| reactions.rs | 514 | ruleset (≈440), content (≈70) | Deletion, flinch, paralysis, drag, freeze, bubble. **Mixed:** the per-AI-index death, flinch and drag hooks; reaction sound ids. |
| idle.rs | 351 | ruleset (≈200), content (≈150) | The idle controller. **Mixed:** weapon routines (buster, charged, blank, claw) and their damage, NaviCust interception, move lag. |
| chip_use.rs | 292 | ruleset (≈150), content (≈140) | Chip use and chaining. **Mixed:** chain exclusions (chips 0x52/0x53), aura chips 0x150 and 0x5F..0x61, dark chips 0x11E..0x122, form bonuses per family, cross doubles. |
| entry.rs | 98 | ruleset | Actions 0 and 1. |
| form.rs | 60 | content | Falzar Beast Out's overlay and flags. |
| actions/mod.rs | 60 | ruleset | Action dispatch (a `match`), counter window, reactive abort. |
| actions/movement.rs | 272 | ruleset (≈250), content (≈20) | The step action every navi uses. |
| actions/gun_del_sol.rs, beast_claw.rs | 332 | content | Two attack actions. |
| actions/beast_rush.rs | 264 | content (needs a ruleset hook) | The Beast Out wrapper around chip actions. |
| actions/transform.rs | 255 | content (≈225), ruleset (≈30) | Beast Out's form-change steps. |
| actions/tests.rs | 151 | tests | |

### 1.3 Totals

| Layer | Lines | Where |
|---|---|---|
| Core | ≈ 1,650 | object/, rng, input, the core half of collision and field, the tick skeleton, common sprite helpers |
| Ruleset | ≈ 7,050 | battle flow (≈900), collision kernel and field rules (≈680), the navi framework (≈3,150), ruleset stdlib kinds (≈1,330), hand, hud, transform, setup, actor and kind helpers (≈970) |
| Content code | ≈ 3,300 | the player's MegaMan parts (≈1,900 including GunDelSol, claw, rush, Beast Out), content kinds (≈950), literals scattered through the ruleset (≈450) |
| Content data | 1,937 | data/ |
| Tests, harness | ≈ 750 | |

### 1.4 Where the layers mix today

These are the couplings a boundary has to cut. Each one is a concrete site, not a style note.

1. **Kind dispatch is closed.** `kinds::update` and `Vars::for_kind` match on `(Pool, index)`; the list of kinds
   is compiled into the engine, and `kinds::Vars` is an enum of every kind's state.
2. **Action dispatch is closed.** `actions::dispatch` matches action numbers; `idle::weapon_routine` matches the
   game's weapon routine indices (0, 1, 2, 0x1E); `status::dispatch` hard-wires actions 0..8 and the Beast rush.
3. **Core reads content tables through globals.** The animation stepper reads `SPRITES`; collision reads
   `REGIONS`, `COLLISION_TYPES`, `FIELD_REGIONS`, `ELEMENT_WEAKNESS`; the field reads `PANEL_LAYOUTS`,
   `PANEL_TYPE_FLAGS`, `STEP_RULES`. None of these is passed in.
4. **Ruleset code switches on content ids.** 43 lines test forms outside the `Form` type itself (14 in
   player/mod.rs, 7 in chip_use.rs); 16 test `Navi(n)`; NameID ranges (0x1A0..0x1C3 players, 0x1AC..0x1C1
   crosses, 0x173..0x17E bosses, 0xDA) appear in 8 files; AI-index lists in 3; chip-id literals in 3 (0x52, 0x53,
   0x5F..0x61, 0x150, 0x11E..0x122 in chip_use.rs; 0xBB, 0xBC, 0x157 in intake.rs; 0x4C..0x4F and 0x190 in
   input.rs); and 24 `SoundId(0x..)` literals sit outside sound.rs.
5. **The flow knows content.** `spawn_actors` calls `kinds::rock::spawn_at_start`; `mode_intro` spawns the intro
   object and picks music; `fight_setup` counts down MegaMan's Beast Out; `custom_open_requested` has the Beast
   Over rule.
6. **Unported content is a panic inside the ruleset.** 83 `"... is not implemented yet"` panics, most of them
   in ruleset functions at the point where a chip, form or navi would plug in (e.g. "Cross overlays",
   "reactive defensive chips", "weapon routine", "form action"). They mark the future content interface
   precisely.
7. **Content state lives in ruleset structs.** `ActorData` holds `absorbed`, `lockon_marker`,
   `full_synchro_aura`; `AttackVars` holds the Beast rush state; `Battle` holds `beast_out_used`,
   `linked` (defensive chips), `damage_carry`.
8. **Trace identity and presentation read raw kind numbers.** `trace::compare` prints `T{type}#{index}` and
   decides garbage positions from `(type, index)`; the frontend special-cases effect #0x0A and actor #0x57.
9. **State references static data.** `rock::Vars` holds `&'static RockKind`; `BattleSettings` holds
   `&'static ActorList`. Loaded content would need ids instead. (Fixed since: `BattleSettings::actors` is an
   `ActorListId` and a rock's kind is a variant id.)
10. **Duplicated core helper.** `player::update_sprite` and `common::update_sprite` are the same function.

### 1.5 The other crates

| Crate | Lines | Layer | Notes |
|---|---|---|---|
| bn6-extract | 1,390 | content importer | ROM addresses and decoders for every table (then main.rs 816: engine tables; graphics.rs 381 and hud.rs 156: graphics; assets.rs 37: sound. Now it writes all of it as one content pack). It is the BN6 content pipeline for ruleset and content data alike. |
| bn6-assets | 371 | presentation core + BN6 schema | Generic tile/palette/sprite-part types plus a BN6-specific `Hud` (chip names, mugshots, banners). |
| bn6-frontend | 2,434 | presentation core (≈1,120) + BN6 presentation (≈1,310) | compose, render, text, app, main, headless, session and lib are generic. hud.rs (502) and stage.rs (276) draw BN6's HUD and field; objects.rs (284) is generic except for the #0x57 and #0x0A special cases; driver.rs (253) hard-codes a live-play hand (GunDelS3, Geddon, Beast Out) and a MegaMan setup. It reads engine internals directly (objects, actor status bits, hands, the transform sequencer, moods). |
| bn6-audio | 560 | presentation mapping | `SoundCalls` turns cues into driver calls with BN6 specifics (music player 31, the pinch pitch/tempo, volume restore on players 31 and 22). |
| m4a | 2,577 | presentation core | The GBA sound driver; content-independent. |

## 2. The core API

This is the surface content is written against, derived from what the ported content calls today. It is one
handle, `Ctx`, borrowed for the duration of a callback. Operations marked (R) are ruleset services over the core;
content does not see the difference.

```rust
/// Everything a content callback may touch, for one call. Holds `&mut Battle` and `&Content`.
pub struct Ctx<'a> { /* private */ }
```

Conventions that apply throughout:

- **Handles, not references.** Content holds `ObjectRef`, `ActorId`, `CollisionId` and content ids, never
  references into the battle.
- **Handles are raw slots.** They are not generational. A handle can outlive its object and alias the slot's
  next occupant, as the game's pointers do (§4.9). The core never panics on a stale handle.
- **Effects are immediate.** Every call mutates state at once. There is no command buffer and no end-of-tick
  apply: later objects in the same tick see the change, as in the game.
- **Integers are fixed-width.** u8/u16/u32/i8/i16/i32 only, wrapping unless stated. No floats.

### 2.1 Identity

```rust
pub struct ObjectRef { pub pool: Pool, pub slot: u8 }  // Pool::{Actor, Attack, Effect}, 32 slots each
pub struct CollisionId(pub u8);                         // 32 slots
pub struct ActorId(pub u8);                             // 8 slots
pub struct KindId(pub u16);    // registry index of an object kind
pub struct ActionId(pub u8);   // a navi action number (0x10 and up for attacks)
pub struct ChipId(pub u16);    // 0..=0x19A, 0xFFFF = none
pub struct SpriteId { pub category: u8, pub index: u8 }
pub struct SoundId(pub u16);
pub struct FormId(pub u8);     // NaviStats+0x2C
pub struct NaviId(pub u8);     // NaviStats+0x29
```

A kind declares its **trace identity**: the original's pool and object index (`T4#0x48` for the sun beam). The
golden traces compare the object list by these numbers, and the frontend draws by them, so BN6 content keeps the
original numbers; new content gets ids outside the original ranges.

### 2.2 Spawning and freeing

```rust
impl Ctx<'_> {
    fn spawn(&mut self, kind: KindId, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef>;
    fn spawn_at_tail(&mut self, kind: KindId, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef>;
    fn free(&mut self, r: ObjectRef);
    fn destroy_next_update(&mut self, r: ObjectRef);   // state word := [8, 0, 0, 0]
    fn generic_destroy(&mut self, r: ObjectRef);        // (R) reservations, collision, then free
    fn is_allocated(&self, r: ObjectRef) -> bool;
    fn current(&self) -> Option<ObjectRef>;             // the object whose update is running
    fn objects_in_order(&self) -> impl Iterator<Item = ObjectRef>;
}
```

Bit-exact semantics:

- **Slot:** the lowest free slot of the kind's pool (slot 0 first). A full pool returns `None`; callers carry on
  (an attachment then stores `None` in its owner's slot, and the owner's later writes to it are no-ops).
- **Initial state:** the common header is zeroed except the sprite, which keeps the previous occupant's data
  until a `sprite_load` (the game's zero fill stops at the sprite block). Header flags are the pool default: 0x19
  for actors and effects (active, no sprite update, runs while dimmed), 0x09 for attacks. Spawners OR in 0x04
  (runs while paused) or 0x14 themselves. The kind's state is its zero value. The state word is [0,0,0,0], so the
  first update runs init.
- **Position in the list:** immediately after the object currently updating, so the new object runs later in
  this tick. With no object updating (spawns from the flow), or when the current object freed itself and the new
  object got that very slot, it goes to the tail. Several spawns from one update therefore run in reverse spawn
  order (the deletion's two explosions, a rock's debris).
- **Free:** clears the header flags and the slot bit and unlinks the node; the freed node's own links stay, and
  the update loop reads the current object's successor after the handler returns. So an object may free itself
  or its successor. Freeing itself and then spawning into a lower slot orphans the child after this tick; these
  corner cases follow from the list rules and must not be "fixed". Free releases nothing else: collision data,
  actor data and reservations are the caller's (`generic_destroy`).
- **End of round:** `free_all` in slot order, pool by pool (flow only).

### 2.3 The update loop, gating and state machines

```rust
pub struct StateWord { pub state: u8, pub action: u8, pub phase: u8, pub phase_init: u8 }

impl Ctx<'_> {
    fn flags(&self, r: ObjectRef) -> u8;                // ACTIVE 1, VISIBLE 2, RUN_WHILE_PAUSED 4,
    fn set_flags(&mut self, r: ObjectRef, bits: u8);    // NO_SPRITE_UPDATE 8, RUN_WHILE_DIMMED 0x10,
    fn clear_flags(&mut self, r: ObjectRef, bits: u8);  // HOLDS_RESERVATION 0x20
    fn progress(&self, r: ObjectRef) -> StateWord;
    fn set_progress(&mut self, r: ObjectRef, w: StateWord);
    fn set_action(&mut self, r: ObjectRef, action: u8); // phase = phase_init = 0
    fn set_phase(&mut self, r: ObjectRef, phase: u8);   // phase_init = 0
    fn is_paused(&self) -> bool;
    fn is_dimmed(&self) -> bool;
}

pub struct KindDef {
    pub name: &'static str,
    pub pool: Pool,
    pub trace_index: u8,
    pub state: SchemaId,                      // §5.4
    pub update: fn(&mut Ctx, ObjectRef),      // dispatches on the lifecycle state itself
    pub scratch_position: bool,               // X/Y/Z are spawn-register garbage until init
    pub draw: DrawHints,                      // presentation attributes the frontend needs
}
```

- **Loop (core):** walk the list; sample the object's flags *before* its handler; skip it if paused and it lacks
  0x04, or if dimmed and it lacks 0x10; otherwise call its kind's `update`. The ACTIVE bit is not tested:
  whatever is linked gets called. The successor is read after the handler.
- **State word:** `[state, action, phase, phase_init]` is observable (the traces compare it), and its values are
  the original's jump-table offsets: state 0/4/8, phases 0, 4, 8, 0xC..., `phase_init` 0 then 4 (or 1 in some
  routines). Content declares named phases with their raw values; the core stores the raw bytes. A phase change
  with `phase_init = 0` runs the entry on the next tick unless the routine falls through.
- **Actor attack steps (R):** navi actions keep their own phase in the attack header (`step`, `step_init`), not
  in the state word; `set_attack` resets both.
- **Gates** are core: `paused` and `dimmed` are two global bits. When they flip is ruleset (flow) or content
  (a dimming controller's init starts the dimming in the middle of the object loop; later objects in the same
  tick already see it).

### 2.4 Timers and integer semantics

The common header has two observable u16 timers, `timer` and `timer2`; content state holds others. Routines count
down in four distinct ways, and each must be reproduced as written:

| Idiom | Test | Examples |
|---|---|---|
| `t = t.wrapping_sub(1)`; act when `t == 0` | new value | effect lifetime, entry fade-in, deletion wait, the step's phases, hole and road timers |
| `v = t as i32 - 1; t = v as u16`; act when `v < 0` or `v <= 0` | widened new value | flinch (24 ticks), GunDelSol, drag recovery, the idle phase timer |
| `old = t; t = old.wrapping_sub(1)`; act when `old == 0` or `old < 2` | old value | Beast Out steps, the claw |
| `t.saturating_sub(1)` | none | chip lockout, counter window |

Other widths that matter: HP is u16 and `subtract_hp` saturates at 0, `add_hp` clamps to MaxHP; hit accumulators
are u16 and wrap (`element_damage += self_damage * m` wraps); positions are i32 16.16 and wrap on add; a
barrier's HP is u8 (`left as u8`); the charge counter is u8. Signed division truncates toward zero
(`panel_at`, the absorbed obstacle's velocity `/ 9`). A scripting runtime must offer exactly these operations
(explicit wrapping, saturating and widening), not arbitrary-precision or floating-point numbers.

### 2.5 Collision regions and hitboxes

```rust
impl Ctx<'_> {
    fn create_collision(&mut self, r: ObjectRef) -> Option<CollisionId>;
    fn setup_collision(&mut self, r: ObjectRef, self_type: u8, target_type: u8, hit_mod: u8); // (R)
    fn reset_collision_types(&mut self, r: ObjectRef, self_type: u8, target_type: u8, hit_mod: u8); // (R)
    fn set_region(&mut self, r: ObjectRef, region: u8);  // 0 = none; 0x80.. = filtered whole field
    fn present_collision(&mut self, id: CollisionId);
    fn remove_collision(&mut self, id: CollisionId);
    fn free_collision(&mut self, id: CollisionId);
    fn update_collision_panels(&mut self, r: ObjectRef); // anchor := panel; direction of the move
    fn collision(&self, id: CollisionId) -> &CollisionData;           // (R) typed
    fn collision_mut(&mut self, id: CollisionId) -> &mut CollisionData;
    fn spawn_hitbox(&mut self, owner: ObjectRef, spec: &HitboxSpec) -> Option<ObjectRef>; // (R) T3#3
}
```

- **Allocation:** lowest free slot; the slot is zeroed except its bit; `enabled = 1`.
- **Setup (R):** copies the object's element (low nibble primary, high nibble secondary), alliance, flip, panel,
  counter byte (low byte of `stamina`) and damage word; looks up the self and target type masks by (index,
  alliance), OR 0x10000 into self when created while dimmed; decodes the damage word (§2.6); region = 1.
- **Present:** clears the hit results (while dimmed the final hit modifier and guard directions survive), then
  sets the slot's bit on each valid panel of its region in the region's list order and refreshes each panel's
  flags. Whole-field regions skip the refresh (the game passes the wrong arguments).
- **Remove:** for each panel of the *current* region (whole field: only where the bit was set), clear the bit,
  refresh the flags, pair-test, and let the ruleset convert the panel. Pair tests and conversions are skipped
  while paused. Pair test: every other slot on that panel in ascending slot order; a pair already tested since
  the last present is skipped; otherwise both directions react, `A` reacting to `B` iff `A.target & B.self != 0`.
  The reaction itself is the ruleset's hit kernel (`sub_3007218` and the raw channel `sub_3007692`), called
  synchronously.
- **Free:** clears `enabled` and the allocation bit only. Mask bits stay: a freed slot keeps being pair-tested
  and keeps contributing to panel occupancy until its bits are removed or the slot is reused.
- **Regions:** offsets `(dx, dy)` with `dx` times the facing (`alliance ^ flip`); invalid panels skipped; list
  order is processing order.
- **Hitbox (R):** the ruleset's standard one-tick region. Its init runs in the spawn tick: invalid panel frees
  it; otherwise create, set up, present, remove at once, spawn the hit effect (an RNG draw when it hit and has
  one), and free itself if it hit or its timer (0 unless the spawner set one) runs out. Victims read the result
  at their next update, which is this tick if they come later in the list.

### 2.6 Damage words and hit results

```rust
pub struct DamageWord(pub u16);  // bits 0..=10 value; 0x8000 DOUBLE; 0x4000 PARALYZE; 0x2000 UNINSTALL;
                                 // 0x1000 ERASE_CROSS; 0x0800 no effect
pub struct HitParam(pub u8);     // the counter byte: bits 0..=6 strength, bit 7 "cannot counter"
pub struct HitResults {          // (R) zeroed on present
    pub hit_flags: u32, pub raw_hit_flags: u32, pub hit_by: u32,
    pub hit_mod_final: u8, pub status_final: u8,
    pub element_damage: [u16; 6], pub raw_element_damage: [u16; 6], pub final_damage: u16,
    pub damage_multiplier: u8, pub exclamation: u8, pub damage_elements: u8, pub raw_elements: u8,
    pub counter: u16, pub mood_damage: u16, pub drain_hits: u16, pub elec_damage: u16, pub inflicted_bugs: u16,
}
```

- **Decode (R, `sub_8019F44`):** `self_damage = word & 0x7FF`; DOUBLE doubles it (u16); PARALYZE sets status 0x10
  and forces hit modifier 1; UNINSTALL sets bug 0xF8 and stops decoding; otherwise ERASE_CROSS sets bug 0xF7. The
  bug's high byte is the leftover `target_type * 8 + alliance * 4` from the type lookup (4 or 0 when redone by
  `reset_collision_types`). Content supplies the word; the ruleset owns the decoding and the garbage.
- **Accumulate (R):** the multiplier is `1 + weakness + secondary weakness`, plus 1 for breaking a frozen target
  and 1 for elec on a bubble (added, not multiplied); `element_damage[elem] += self_damage * m` (u16). Element 5
  lands in slot 5, which final damage excludes: GunDelSol's drain bypasses flinch, flash, barriers, holy panels
  and Undershirt.
- **Final damage (R):** per element, halved rounding up on a holy panel under the object, summed through the
  side's damage-carry record, stored as u16. No cap.
- **Consumption (R):** in the receiver's own next update (stage A then B), so a hit applies this tick or next
  depending on list order.

### 2.7 Panels

```rust
impl Ctx<'_> {
    fn panel(&self, p: PanelPos) -> Option<&Panel>;        // None off the 6x3 field
    fn panel_flags(&self, p: PanelPos) -> u32;             // 0 off the field
    fn check_panel(&self, p: PanelPos, require: u32, forbid: u32) -> bool; // flags != 0 && all/none
    fn reserve_panel(&mut self, r: ObjectRef, p: PanelPos) -> bool;
    fn unreserve_panel(&mut self, r: ObjectRef, p: PanelPos) -> bool;
    fn release_reservations(&mut self, r: ObjectRef);
    fn set_panel_type(&mut self, p: PanelPos, t: PanelType);      // (R)
    fn set_panel_alliance(&mut self, p: PanelPos, side: u8);      // (R)
    fn crack_panel(&mut self, p: PanelPos) -> bool;               // (R)
    fn break_panel(&mut self, p: PanelPos, rule: BreakRule) -> bool; // (R) the four break variants
    fn highlight_panel(&mut self, p: PanelPos);                   // presentation
    fn can_step(&self, r: ObjectRef, p: PanelPos) -> bool;        // (R) sub_800E618
    fn can_slide_to(&self, r: ObjectRef, p: PanelPos) -> bool;    // (R) sub_800E5AC
    fn can_stand_any_side(&self, r: ObjectRef, p: PanelPos) -> bool; // (R) sub_800E680
    fn register_field_object(&mut self, r: ObjectRef, side: u8, class: u8); // (R) may evict (HP := 0)
    fn unregister_field_object(&mut self, r: ObjectRef);
}
```

- **Flags word (core, parameterized):** `type bits | type flags[type] | owner << 5 | 0x80 if reserved | the OR of
  (self & 0xFFFF0000) of every slot whose bit is on the panel`, stale bits included. The type-flags table is
  ruleset data. The word is cached and refreshed only at the points the game refreshes it (present, remove, type
  and owner changes, init).
- **Reservation (core):** succeeds only if nobody holds the panel (including the caller) and sets header flag
  0x20, which is never cleared; `release_reservations` scans y = 4..1, x = 7..1 and only runs with that flag.
- **Crack and break (R)** edit the cached flags in place (the 0x4000 "broken" bit appears only at the next full
  refresh) and play sound 0x97 (one break variant plays 0xDA). `set_panel_type` is a no-op on a missing panel and
  resets a road's timer.

### 2.8 Movement and slides

```rust
fn panel_center(p: PanelPos) -> (i32, i32);   // ((x*40 - 140) << 16, (y*24 - 20) << 16), s8 inputs
fn panel_at(x: i32, y: i32) -> PanelPos;      // (((x >> 16) + 160) / 40, ((y >> 16) + 32) / 24) as u8
fn facing(alliance: u8, flip: u8) -> i32;     // +1 when alliance ^ flip == 0
impl Ctx<'_> {
    fn set_coordinates_from_panel(&mut self, r: ObjectRef);  // x, y only; z untouched
    fn set_panel_from_coordinates(&mut self, r: ObjectRef);
    fn snap_to_future_panel(&mut self, r: ObjectRef);        // (R) panel := future, unreserve, coords, collision
    fn start_step(&mut self, r: ObjectRef, dir: u8, end_lag: u16, kind: MoveKind); // (R) action 0x10
    fn request_slide(&mut self, r: ObjectRef, kind: SlideType);                    // (R) sub_80166B6
}
```

Directions are never mirrored in input; "forward" is `1 - 2 * alliance` in x. Slides move 0xA0000 in x and 0x60000
in y per tick, 4 ticks per panel, recomputing the panel from coordinates each tick, so `panel` changes
mid-slide. Arrival uses the game's "passed the target" test (`sub_800E6E8`; moving left, landing exactly on the
target does not count).

### 2.9 Animation and sprite state

```rust
impl Ctx<'_> {
    fn sprite_load(&mut self, r: ObjectRef, id: SpriteId); // resets animation and look; clears flag 0x08
    fn set_animation(&mut self, r: ObjectRef, anim: u8);    // CurAnim = anim, CurAnimCopy = 0xFF: restarts
    fn request_animation(&mut self, r: ObjectRef, anim: u8);// CurAnim only: restarts on change
    fn update_sprite(&mut self, r: ObjectRef);              // object_updateSprite gating
    fn update_sprite_while_dimmed(&mut self, r: ObjectRef); // object_updateSpriteTimestop
    fn step_sprite(&mut self, r: ObjectRef);                // sub_801BCD0: paused or not
    fn sprite_tick(&mut self, r: ObjectRef);                // bare sprite_update, no gating
    fn frame_flags(&self, r: ObjectRef) -> u8;              // 0x80 last, 0x40 loop: only when count == 0
    fn animation_finished(&self, r: ObjectRef) -> bool;
    fn look_mut(&mut self, r: ObjectRef) -> &mut Look;      // write-only presentation
}
```

- **Stepper (core):** a frame of duration d is current for exactly d updates; end-of-animation is visible after
  the sum of durations; a held last frame reports 0x80 from then on; a looping one for one tick per cycle;
  zero-duration frames are skipped within one update.
- **Gating:** `update_sprite` skips while paused, while dimmed unless the object runs while dimmed, when the
  object has collision data and `prevent_anim` set, and for inactive or non-animating objects. Which wrapper a
  routine calls is part of its behavior (the form overlay picks one of three by a spawn parameter).
- **Animation timing is simulation data.** Effect lifetimes and chip timings end on `frame_flags`; any change of
  graphics format must keep per-frame durations and flag bits exact and keep them in the simulation's content,
  keyed by `(SpriteId, anim)`. Pixels are presentation; timing is not.
- **`Look`** (palette, flips, shadow, white, shader, alpha, mosaic, priority, hidden parts) is written by content
  where the game calls a `sprite_*` routine and never read by the simulation. Attachments and overlays copy it
  from their owner, which is still output only.

### 2.10 Sound cues

```rust
fn play(&mut self, cue: impl Into<SoundCue>);   // Effect(SoundId) | Music | StopMusic | Pinch(bool) | RestoreVolume
```

Output only; nothing reads it. Order within a tick is observable (checked against the original's queued sound
calls). Ids are song-table indices; they are content ids and should be named in content, not literals in the
ruleset.

### 2.11 RNG draws

```rust
fn rng_next(&mut self) -> u32;                        // GetRNG2: 1 draw
fn rng_next_positive(&mut self) -> u32;               // GetPositiveSignedRNG2: 1 draw, & 0x7FFFFFFF
fn jitter(&mut self, mask: u32, pos: Vec3) -> Vec3;   // AddRandomVarianceToTwoCoords: 1 draw;
                                                      // x += ((r & m) - (m >> 1)) << 16,
                                                      // z += (((r >> 16) & m) - (m >> 1)) << 16
fn random_panel(&mut self, candidates: &[PanelPos]) -> Option<PanelPos>; // (R) 1 draw when any: r % count
```

Only the simulation stream (the game's RNG2) is part of the simulation. RNG1 (camera shake, folder shuffle, the
custom screen) is per console and never read by lockstep code; the engine does not model it.

Draws in ported code, all in object update order: every buster request (1, `sub_8013D5E`, even with the
NaviCust stats at 0), plus 1 positive draw when a buster or charged-shot projectile stat is set; each `jitter`
(hit effects of hitters, guard sparks, rock debris positions, the dimming shake every tick for 30 ticks after a
hit); rock debris init (2 each, so a broken rock costs 6); the battle-start style hook for NaviCust 9/10 (1); the
intro's reward pick (1 positive). Unported content adds more (Vulcan: 1 per bullet). A content change that adds,
removes or reorders a draw changes everything after it, so draws are part of a script's contract.

### 2.12 Input and the chip hand

```rust
impl Ctx<'_> {
    fn pad(&self, r: ObjectRef) -> Pad;              // held, pressed, released, previous
    fn dimmed_pad(&self, r: ObjectRef) -> Pad;     // maintained only while dimmed
    fn requests(&self, r: ObjectRef) -> u32;         // (R) the navi's request bits (ai+0x44)
    fn set_requests(&mut self, r: ObjectRef, bits: u32);
    fn clear_requests(&mut self, r: ObjectRef, bits: u32);
    fn hand(&self, side: u8) -> &ChipHand;           // (R)
    fn hand_entry(&self, r: ObjectRef) -> HandEntry; // (R) chip, damage, extra (Atk+ + form + charge), modifiers
    fn advance_hand(&mut self, side: u8);            // (R) cursor += 1 unless at 5 or at the terminator
}
```

The flow latches pads from the link records in fighting states 4 and 8 only, before objects run, recomputing edges
from `held` alone; in all other states actor inputs are frozen. `ChipsHeld`/`Chip` on the object are exposed after
objects run, so they are one tick stale; the chip pipeline reads the hand directly.

### 2.13 Attachments and links

```rust
pub enum Slot {                 // "where an owner keeps an attached object"
    Related(ObjectRef, u8),     // the owner's related[0] or related[1]
    ActorField(ActorId, FieldId),
    ObjectField(ObjectRef, FieldId),
}
impl Ctx<'_> {
    fn related(&self, r: ObjectRef, i: usize) -> Option<ObjectRef>;
    fn set_related(&mut self, r: ObjectRef, i: usize, v: Option<ObjectRef>);
    fn slot_get(&self, s: Slot) -> Option<ObjectRef>;
    fn slot_set(&mut self, s: Slot, v: Option<ObjectRef>);
}
```

The game's tether contract is uniform: the spawner stores the new object (or `None` on a full pool) in a slot of
its own; the tethered object checks that slot every update and, once it is empty, goes to the destroy state and is
freed at its next update. Owners empty slots when an action ends and in every hit reaction (`related[0] = None`,
overlay = `None`). `FieldId` names a field of a declared schema (§5.4); slots are never byte offsets.

### 2.14 The counter window

```rust
fn open_counter_window(&mut self, r: ObjectRef);   // (R) counter_timer := 16
```

`object_setDefaultCounterTime`; a player gets the window only in link battles, other actors always. Stage A counts
it down each tick. A hit whose counter byte has strength and lacks bit 7 lands as a counter while it is open
(§4.8).

### 2.15 Form changes

```rust
impl Ctx<'_> {
    fn form(&self, side: u8) -> FormId;
    fn turn_transform(&self, side: u8) -> TransformRequest;   // (R) what this turn changes into
    fn set_form(&mut self, r: ObjectRef, form: FormId);       // (R) NaviStats form byte, NameID
    fn reset_status(&mut self, r: ObjectRef);                 // (R) sub_80144C0: weapons, form flags, element
    fn end_pause_action(&mut self, r: ObjectRef, which: PauseAction); // (R) clears the state bit
}
```

The ruleset's sequencer requests a change (`request::FORM_CHANGE`); the pause handler (§4.7) turns the request
into action 0x1C with a state bit, and runs the form's change script every paused tick until it clears the bit.

### 2.16 Chip use and chaining

```rust
pub struct AttackHeader { /* §4.2: the persistent part of AIAttackVars */ }
impl Ctx<'_> {
    fn set_attack(&mut self, r: ObjectRef, action: ActionId, kind: AttackKind);  // (R) object_setAttackN
    fn exit_attack(&mut self, r: ObjectRef);     // (R) object_exitAttackState: anim 0, then end_attack
    fn end_attack(&mut self, r: ObjectRef);      // (R) sub_801171C
    fn prepare_chip(&mut self, r: ObjectRef, into: AttackTarget) -> ActionId; // (R) sub_80127C0
    fn chain_next_chip(&mut self, r: ObjectRef) -> bool;                       // (R) sub_800FC30
    fn attack(&self, r: ObjectRef) -> &AttackHeader;
    fn attack_mut(&mut self, r: ObjectRef) -> &mut AttackHeader;
}
```

- `set_attack`: action, phase and `phase_init` := 0, `step` and `step_init` := 0, `kind` := the slot (0..5), then
  `reset_attack_links` (clears `beast_lockon`, restarts the wrapper state, unfreezes a lock-on marker). Nothing
  else in the header is cleared.
- `end_attack`: kind 4 (a move) keeps pending requests and the charge; any other kind copies its lockout
  (kind 2 to the chip lockout, kind 3 to the B+Back cooldown), drops the buffered move, clears requests
  0x1000003F, drops the charge and USING_ACTION. Then action 8, `step` 0. `phase`/`phase_init` are left as
  `set_attack` set them, so idle re-runs its phase-0 entry next tick.
- Chip use happens inside the idle controller on the press tick (the hand advances that tick); the action's first
  handler tick is the next one. `prepare_chip` fills the header from the hand entry and the chip record, applies
  double damage, paralyze, uninstall and erase bits, the chip-recovery heal and the navi-chip counter, and returns
  the chip's action. `into` exists for the cut-in, which prepares into a temporary header (§4.10).

### 2.17 Global battle state

```rust
impl Ctx<'_> {
    fn is_fighting(&self) -> bool;                // battle flag 1
    fn is_battle_over(&self) -> bool;             // a side has no navi, or time up
    fn is_battle_over_zflag(&self) -> bool;       // the Z-flag reading seven callers use: true only for time up
    fn set_dimmed(&mut self, on: bool);           // (R) via the dimming service
    fn player(&self, side: u8) -> Option<ObjectRef>;
    fn is_remote(&self, alliance: u8) -> bool;    // alliance != the local side (presentation choices)
    fn link_battle(&self) -> bool;
    fn battle_mode(&self) -> u8;
    fn panel_pattern(&self) -> u8;
    fn navi_stats(&self, side: u8) -> &NaviStats;             // (R)
    fn navi_stats_mut(&mut self, side: u8) -> &mut NaviStats; // (R) moods, bug levels, supports
    fn bump_side_stat(&mut self, side: u8, stat: SideStat, n: u8);
    fn global<T: GlobalSchema>(&self) -> &T;      // content-declared battle globals (§5.4)
    fn global_mut<T: GlobalSchema>(&mut self) -> &mut T;
}
```

### 2.18 What content may not do

Keep state outside the store (no statics, closures, coroutines or VM globals); hold references across calls;
reorder or skip list entries; defer a mutation to later in the tick; use floating point or unbounded integers; read
`Look` or sound cues for decisions; allocate unbounded memory; iterate anything in an order other than slot or list
order.

## 3. The content model

### 3.1 The content pack

A battle runs against one immutable content pack, identified by a hash. It holds data tables and script
references. Which parts are data:

| Group | Tables | Then | Now (a pack's files) |
|---|---|---|---|
| Ruleset data | collision types, regions, filtered regions, element weakness, panel type flags, step rules, road directions, status effects, push/ice/road vectors, bubble bob, charge thresholds, HP-bug periods, gauge rates, banner holds | generated Rust | `rules/*.toml`, `registries/regions.toml` |
| Chips | 411 chip records; family elements; per-action variant tables; damage formulas | generated Rust (formulas: panic) | `chips/NNN-name/chip.toml` (formulas: panic) |
| Kinds | per kind: pool, trace index, flags, state schema, script; attachment kinds (52); effect and spark sprites (108 + 16); rocks (4); absorbed-obstacle sprites (15) | Rust code + generated tables | Rust code + `objects/<kind>/object.toml`, `registries/effects.toml`, `registries/sparks.toml` |
| Sprites | animation timing: `(SpriteId, anim) -> [(duration, flags)]` for 298 sprites | generated Rust | `graphics/sprites/*/animations.json` |
| Navis | navi records (36 NameIDs), sprites, elements, weaknesses, attach points (36 × 34), move lag, buster bonus, win/lose banners | generated Rust | `navis/NN-name/navi.toml` |
| Forms | sprite, element, weakness, weapons, per form (25) | generated Rust | `navis/megaman/forms/NN-name/form.toml` |
| Stages | panel layouts (237), actor lists (28), battle settings | generated Rust | `rules/stages.toml`, `registries/panel-layouts.toml` |

Scripts implement object kinds, actions, weapon routines, damage formulas, hooks and form-change sequences. The
ruleset stays Rust (§7).

### 3.2 Chips

**Data:** the whole record (codes, element, family, class, flags, counter byte, action, subtype, params,
lockout, Beast lock-on flag and mode, damage or formula id, dark substitute) plus properties that today are
literals in the ruleset: not chainable in a Beast rush (0x52, 0x53), aura bonus (0x5F..0x61, 0x150), trap kind
(0xBB, 0xBC, 0x157), dark-chip side effect (0x11E..0x122), special hand entries (0x190..0x19A).

**Code:** none per chip. A chip names an action; all per-chip variation within an action is data indexed by
`subtype` (GunDelSol's firing ticks {60, 90, 120, 120} and beam looks; Cannon's projectile descriptors; Vulcan's
shot counts {3, 4, 5, 10}). The original has 46 attack actions used by chips, 38 dimming chip spawners (action 0x15,
by subtype), 27 navi-chip summons (action 0x1B, by subtype) and 10 chips dispatched through per-form action tables:
about 120 scripts for 411 chips. Damage codes 1000..1044 map onto 7 formula functions.

**Rules around chips** (hand building, Program Advances, Atk+ folding, the use pipeline, lockout, form bonuses as
a form × family table) are ruleset.

### 3.3 Actions (attacks)

An action is a script with:

- an `ActionId` (the original's number for BN6 content);
- a private state schema, fresh when the action starts (GunDelSol: `timer: u16`; the claw: `timer, slashes`);
- the header fields it reads and writes (§4.2);
- whether a form wrapper may wrap it (`beast_lockon`);
- one `update(ctx, r)` per tick while it is the navi's action: a phase machine on `step`/`step_init` that spawns
  kinds, plays sounds, sets animations, opens the counter window, and ends with `exit_attack`.

Weapon routines (the game's `off_80117D4`: buster 0, charged shot 1, blank 2, claw 0x1E...) are small setup
scripts that fill the header and return an action.

### 3.4 Object kinds

A `KindDef` (§2.3) plus either a script or a template. Most visual kinds follow a few patterns that could be
declared rather than scripted:

- **One-shot effect** (T4#0): sprite from a table row, lifetime = timer or animation end, optional visibility
  follow. Pure data per effect id.
- **Tethered follower** (attachment T1#5, sun beam T4#0x48, form overlay T1#0x57, charge glow T4#8): owner,
  slot, offset or attach point, which look fields to copy, which sprite gating, a periodic sound. Mostly data,
  with a script hook for quirks (the overlay restarting its animation every tick until stepped).
- **Obstacle** (rock): the existing `Obstacle` trait: appear, destroyed, idle, with shared hit and lifetime rules.

Gameplay kinds need scripts: the hitbox (ruleset), rocks and debris (RNG, physics), the lock-on marker (target
selection feeds chip targeting), the absorbed obstacle (writes the navi's absorbed list).

### 3.5 Navis

A `NaviDef` per NameID family:

- **Data:** the navi record (version, actor type, AI index), battle sprite, element, weakness, attach points, move
  lag per variant, buster bonus, weapon routine indices, win/lose banners.
- **Code:** the hooks the ruleset calls where it panics today: init and post-init hooks (`off_80EA9A0`,
  `off_80EAA04`, `off_8010E0C`), the per-tick form hook (`off_80EA93C`: MegaMan's height clamp), death, flinch and
  drag hooks, the palette, the AI-index action table entries 9..0xC, and a controller (the idle controller for
  players; AI for viruses and CPU navis later).

The navi framework itself (request decoding, charge, stage A and B, reactions, deletion) is ruleset and shared.

### 3.6 Forms

A `FormDef` per form (1..=0x18):

- **Data:** sprite, element, weakness, weapons (buster, A-charge, charged shot, B+Back, alternative A-charge),
  which chip families charge on A, the chip bonus per family, overlay kind and sprite, form flags (AirShoe,
  FloatShoe), whether it is a Beast form (counter, wrapper), whether it is Beast Over (no input, auto chips).
- **Code:** the change sequence run by the pause handler (Beast Out's four steps: prepare, vanish, emerge,
  settle), the revert sequence, the action wrapper (the Beast rush), the lock-on marker, the Beast Over controller.

### 3.7 Effects

Rows `(SpriteId, anim, palette)` by effect id, and the same for hit sparks. Pure data; the generic effect kinds are
ruleset.

### 3.8 How content refers to content

```
ChipDef ──action────▶ ActionDef ──spawns──▶ KindDef: hitbox T3#3, attachment T1#5, sun beam T4#0x48, effect T4#0
   │  ├─subtype──────▶ variant tables (firing ticks, beam look, region shape, projectile descriptor)
   │  ├─hit_param────▶ hitbox counter byte ──▶ counter window, mood damage
   │  ├─lockon_mode──▶ lock-on search (Beast rush destination)
   │  ├─damage≥1000──▶ damage formula script
   │  └─(0x15/0x1B)──▶ dimming chip spawner / navi summon, by subtype
ActionDef ──sounds──▶ SoundId ──▶ song (the pack's sound)
KindDef ──sprite──▶ SpriteId ──▶ animation timing (simulation) + pixels (assets)
        ──effect id─▶ EffectDef ──▶ SpriteId, anim, palette
NaviDef ──NameID──▶ record, sprite, attach points ──▶ used by attachments, charge glow, lock-on marker
        ──forms───▶ FormDef ──weapons──▶ weapon routine ──▶ ActionDef (0x11 buster, 0x16 charged, 0x33 blank, 0x52 claw)
                            ──overlay──▶ KindDef T1#0x57 + SpriteId
                            ──flags────▶ AirShoe/FloatShoe, lock-on marker KindDef T4#0x0F
                            ──wrapper──▶ Beast rush (wraps any ActionDef whose chip has beast_lockon)
                            ──change───▶ change script ──▶ effect 0x2E, palette flash T4#0x0A, sounds 0xF7, 0x100, 0x1CC/0x1CD
```

### 3.9 Data versus code

| Content | Data today | Code today | Achievable | Why |
|---|---|---|---|---|
| Chips | 417 lines of records + ≈80 of variant tables | 332 (two actions) + 106 (sun beam) | per chip: 100% data; per action: a script | Chips differ by record and subtype tables; ≈120 scripts cover 411 chips. |
| Object kinds | effect, spark, attachment, rock, absorbed tables (≈200) | ≈2,300 across 14 kinds (the player aside) | ≈50% data | Followers and effects fit templates; hitboxes, rocks, markers need scripts. |
| Navis | records, sprites, attach points, weapons (≈40 wide lines) | ≈600 (MegaMan's hooks and weapon routines, NaviCust effects) | ≈60% data | Link navis differ mostly by tables plus a handful of hooks. |
| Forms | sprites, elements, weaknesses, weapons | ≈1,000 (change, rush, marker, afterimages, form flags) | ≈40% data | The static properties are tables; change sequences, wrappers and markers are scripts. |
| Effects | 124 rows | none | ≈100% data | |
| Stages | layouts, actor lists | rock spawn in the flow | 100% data | |

"Data today" and "code today" are lines in the current crate. Overall, of today's content about 37% is data
(1,937 of ≈5,240 lines); with templates and the literals moved to tables, well over half would be.

## 4. Hard cases

Each case: what it is, where it shows today, and how the boundary handles it.

### 4.1 Frame ordering

**What.** An object spawned during another's update runs later in the same tick, right after its spawner, and
several spawns run in reverse order. Hits resolve when the *later* of two objects removes its collision, so HP drops
this tick or next depending on list order. RNG draws happen in list order. The dying player's charge glow is freed
after its explosions spawn, which decides their slots (3 and 4, not 1). The flow exposes the hand after objects run.
Dimming can start in the middle of the object loop.

**Handling.** The core owns the list and its rules (§2.2, §2.3), and content cannot influence ordering except by
spawning. Every `Ctx` call applies at once; a scripting runtime must not batch effects or run objects in parallel.
The per-tick system order (mode handler, objects, panels, hand exposure, gauge and banner, linked registry,
variable damage, cycle counters, damage carry, custom drain) is ruleset, written as an explicit list.

### 4.2 The attack scratch shared between actions

**What.** The game's AIAttackVars is one persistent 0x50-byte block per actor. `set_attack` clears only a few
bytes; chip use fills others; everything else keeps what the last action left. Some reads cross actions: the
step reads byte 1 of the last chip's params (`movement::animates`: `params[1] != 0` with state bit 0x8000 turns its
animations off); the Beast rush keeps its state at +0x1E..+0x27, next to the wrapped action's own bytes. The port
splits the block into a typed header (`AttackVars`: step, element, variant, charged, lockout, extra, damage,
hit_param, params, chip_id, special_source, kind, beast_lockon, marker, rush) and a per-action enum
(`ActionVars`) that each action sets when it starts and that panics if read under the wrong action.

**Handling.** Keep that split as the rule: the header is ruleset-owned and never cleared wholesale; an action's
private state is fresh on entry. Any byte the original reads across actions must be promoted to a named header
field. Two things need checking per action as actions are ported: that the action writes each private field before
reading it (otherwise the stale value was part of its behavior), and, for the ≈120 chips a Beast form can wrap, that
the action does not use bytes +0x1E..+0x27, which the wrapper owns in the original (a typed split would silently
diverge where the original overwrote the rush state).

### 4.3 Beast Out's wrapper around arbitrary chips

**What.** When a chip with the lock-on flag is used in a Beast form, `status::dispatch` routes every tick through
`beast_rush::update` instead of the chip's action. The wrapper holds the panel, warps next to the lock-on marker's
target using the chip's lock-on mode, calls the chip's action from its own attack phase, watches for the action to
return to 8, then chains or warps back. Its state sits beside the chip's; `set_attack` restarts it.

**Handling.** A ruleset concept, the **action wrapper**: a form (or anything else) can install a wrapper script
with its own state slot in the attack header. The dispatcher calls the wrapper while `beast_lockon == 1`; the
wrapper calls `ctx.run_action(r)` to step the inner action and reads `ctx.progress(r).action` afterwards. Chips
declare whether they may be wrapped and their lock-on mode; the wrapper is form content. Ordering inside
`chain_next_chip` stays as the game has it: `set_attack` restarts the wrapper, and the wrapper then sets its phase
back to Warp.

### 4.4 Content reaching into other objects

**What.** GunDelSol increments its gun's animation through its slot; the claw stores a halfword into the form
overlay (animation 0xC and loaded animation 0); attachments and overlays copy their owner's position, VISIBLE bit
and look; the lock-on marker reads its target's NameID, animation and attach point; the absorbed obstacle reads the
navi's action (0x58) and appends to its absorbed list; obstacle eviction sets the evicted object's HP to 0; clearing
a defensive chip writes the linked object's second parameter; drain credits and counters write the opponent's actor
data and mood.

**Handling.** One world, one thread: scripts may read and write other objects through handles, but only the
common header, the ruleset's typed structures and fields a kind's schema marks as public. A kind's private fields
are not reachable from other kinds, which is what makes a kind's behavior reviewable. Writes the game does as raw
halfword stores (the claw's) are expressed as the two named fields they set.

### 4.5 Global battle state that content mutates

**What.** Content changes NaviStats (mood, form, bug levels, support flags, the Beast Out counter), battle flags
(dimming; gauge full and custom requested from input), the intro bits, hand cursors, `beast_out_used`, side
statistics, the defensive-chip registry, damage-carry targets, the fade-in queue, the field-object registry, the
alive lists (deletion) and, in unported chips, the custom gauge (FullCust).

**Handling.** Every global is owned by the core or the ruleset and changed through a named operation (§2.17). State
that exists only for some content (a dimming controller's per-side record, the defensive-chip registry) is
declared by that content as a battle global with a schema, stored and snapshotted by the engine like everything
else.

### 4.6 Raw observable state and register garbage

**What.** The traces observe the state word with its jump-table offsets, header flags, timers and positions.
Several positions are whatever the spawner's registers held: the intro sequencer, the charge glow until its first
unpaused update, the palette flash, and the deletion's second explosion, whose X and Y are list-node addresses left
by the allocator. The hitbox and rock spawn with `(panel.y, element, z)` and `(panel.y, side, 0)` as X/Y/Z. A
decoded uninstall or erase bug carries the leftover type-table offset as its high byte.

**Handling.** Content declares raw values where they are observable (phases, `phase_init` 4 or 1, spawn positions
it can reproduce). Where the value is an address, the kind declares `scratch_position` or marks the spawn as
"position unknown", and the trace comparison skips it, replacing today's hard-coded list in `trace.rs`. Garbage the
ruleset produces (the bug high byte) stays in the ruleset. The core never exposes addresses.

### 4.7 The pause handler

**What.** While the battle is paused, stage B skips the action dispatch for a navi whose action is not 0 and runs
`sub_8017BC0` instead. It runs a pause-time action by state bit (form change 0x80, revert 0x100, Cross change
0x1000, Cross knockout 0x2000) or starts one from a request bit, always as action 0x1C. The form change steps its
sprite with `step_sprite` (ignoring the pause) unless a state bit holds it. Only objects with header flag 0x04 run,
so everything a pause-time action spawns must set it (the Beast Out effect 0x2E, the palette flash with 0x14, the
overlay with 0x04 until it clears it). The player's usual `sub_801BCF4` sprite step still runs after its handler
and skips while paused.

**Handling.** A ruleset **pause-time action slot**: content registers handlers keyed by state bit; the ruleset
owns the request-to-state translation and the choice of action 0x1C. The run-while-paused flag is a spawn
argument content must pass deliberately. The two sprite paths stay distinct API calls (§2.9).

### 4.8 Counters

**What.** An attack opens a 16-tick window on its user (`open_counter_window`; players get it only in link
battles).
The kernel marks a counter when the receiver's window is open and the hitter's counter byte has strength without bit
7, and sets the receiver's counter accumulator to 0x8000. Stage A turns a counter into paralysis 0x12 (no flinch or
flash) unless a bubble landed; stage B counts it, closes the window, plays 0x86 and, in battle flag 0x40 mode, feeds
the attacker's gauge; applying damage gives the attacker Full Synchro if its form allows. A barrier absorbing the
hit, or a trap zeroing it, cancels the counter.

**Handling.** Entirely ruleset, with two content inputs: the counter byte (the chip's `hit_param`, or the
action's own value, 0x9E for the claw) and the tick at which the action calls `open_counter_window`. Scripts must
not set the window or the accumulator directly.

### 4.9 Handles that alias

**What.** Freed collision slots keep their mask bits and fields; a deleted player keeps its collision pointer, so
from the tick a new hitbox reuses slot 0, the dead player's "status" is the hitbox's flags, and any code reading
the dead player's flags reads the hitbox's. Freed objects keep their links.

**Handling.** The core keeps raw slot handles on purpose. Generational handles would be safer but would change
behavior the traces observe. Content never gets a reference that could dangle; it gets a handle whose meaning is
"whatever is in that slot now", which is the game's meaning.

### 4.10 Dimming and the cut-in

**What.** Not yet ported, but on the chip path. A dimming controller's init starts the dimming in the middle of the
object loop, so later objects that do not run while dimmed are skipped for the rest of the tick. Per-side dimming
records gate the telop and a cut-in by the other side; a cut-in prepares the cutting-in side's chip into
a 0x50-byte temporary block on the stack so the navi's real attack header and action are untouched; resumption is
last-in, first-out.

**Handling.** A ruleset **dimming service** owning the per-side records and the flag; controllers are content
scripts. `prepare_chip` takes a target (the actor's header, or a detached header) so the cut-in path can use it.

### 4.11 Content ids inside the ruleset

**What.** §1.4 item 4: forms, navis, NameIDs, AI indices, chip ids and sound ids as literals in ruleset functions,
and 83 panics where unported content would plug in.

**Handling.** Replace each literal with a property looked up from the content pack (a form's `is_beast`, a chip's
`chainable`, a navi's death hook, a NameID's family). The panics become calls to a hook that the content either
provides or declares absent. This is mechanical, can be done a family at a time, and the traces verify each step.

### 4.12 View dependence

**What.** Some state depends on which console the engine plays: the local navi appears at once while the remote
one queues for a fade-in (so the intro's object state differs between consoles), the VISIBLE bit of a blinded
viewer's opponent, charge-glow visibility, the hit sound (0x6B local, 0x6D remote). Nothing found feeds these back
into shared state.

**Handling.** `local_side` is part of the battle state and `is_remote` a core query. Content may use it only for
presentation and for these known object-state differences; a desync check between peers must hash state with the
perspective-dependent fields normalized.

## 5. Determinism and snapshots

### 5.1 Where state lives today

All simulation state is in `Battle`: the round state, fighting machine, gauge, banner, pause bit, input records,
both hands, transform requests and sequencer, `objects` (three pools with header, typed kind state and a separate
sprite array the game does not clear on spawn), `actors`, `collision`, `field` (panels, columns, home runs, the
field-object registry), the fade, the fade-in queue, damage carry, custom-screen progress, side state, statistics,
the linked registry and the RNG. Output channels are the sound buffer and each sprite's `Look`. Content data is
`'static`. (Now it is the battle's `Content`, an `Arc` shared by every snapshot, and no state holds a `&'static`
reference into it: `BattleSettings::actors` is an `ActorListId`, a rock's kind a variant id.)

`Battle` is not `Clone` today, but only because nothing asks: adding `#[derive(Clone)]` compiles unchanged. A
throwaway measurement (release build, a two-navi battle 60 ticks in, not committed): `size_of::<Battle>()` is 8,264
bytes, plus the object pools' two vectors (96 × 120 bytes of objects and 96 × 24 of sprites, about 13.8 KB),
`ActorData::absorbed`, `Field::home_runs` and the sound buffer on the heap. A clone takes about 1.7 µs. Snapshots are
already cheap enough for rollback.

### 5.2 Rules any content layer must follow

1. **All simulation state is engine-owned plain data.** Content declares it (§5.4); the engine stores, clones and
   serializes it. Scripts are stateless functions; a runtime's own heap, globals, closures or coroutines hold no
   battle state between calls.
2. **Integers only**, fixed widths, the operations of §2.4.
3. **One RNG stream for the simulation**, drawn only through the core, in the order the calls happen.
4. **Order is list order or slot order.** No hash-map iteration, no sorting by address.
5. **No addresses.** Handles are slot indices; garbage values the game takes from addresses are declared unknown.
6. **Content is immutable during a battle** and identified by a hash that netplay peers compare at the handshake.
   State refers to content by id, never by reference (`&'static RockKind` becomes a rock variant id, as it now
   is).
7. **Outputs are write-only.** Sound cues and `Look` are never inputs to a decision.
8. **Bounded work.** A script that exceeds its budget must fail the battle deterministically on every peer, not be
   cut short.
9. **Perspective stays in its lane** (§4.12).

### 5.3 Output channels under rollback

Resimulated ticks produce sound cues again; the frontend must deduplicate them (play a cue only when its tick is
first confirmed or first predicted, cancel mispredicted ones). `Look` is part of the snapshot (it lives in the sprite
state) so a restored battle draws correctly at once.

### 5.4 Engine-owned typed storage for content state

Yes: content can declare its state schema and scripts can be stateless functions over it. The port already works
this way in Rust. Every kind's state is a plain `Clone` struct in `kinds::Vars`, every action's in `ActionVars`,
the navi's in `ActorData` and `AttackVars`, and no behavior keeps state anywhere else. What changes is who declares
the shapes: instead of closed enums compiled into the engine, content registers schemas and the engine sizes and
stores them.

Requirements on a schema:

- **Named, typed fields**: u8/u16/u32/i8/i16/i32, bool, enums with explicit values, `Option` of a handle
  (`ObjectRef`, `ActorId`, `CollisionId`), content ids (`ChipId`, `SpriteId`, `KindId`...), fixed-size arrays of
  those. No strings, no growable collections: the absorbed-obstacle list becomes eight optional entries, as the
  game has it.
- **Scopes**: per object (kind state), per actor (navi and form state), per attack (action state, wrapper state),
  per side, per battle (content globals).
- **Budgets**: the original's sizes bound what content needs: 0x2C bytes of kind state for actor and attack objects,
  0x1C for effects, 0x100 per actor (0x50 of them the attack block). A content pack that declares larger state is
  still fine, but fixed per-scope maxima keep snapshots fixed-size.
- **Access by name.** Scripts and engine code read `state.timer`, never an offset. This is how the project's rule
  against byte-offset state access carries over to content: a generated codec may pack fields into a byte arena
  behind the API, but that layout is private to the codec, exactly as `from_bytes` is private to the trace and link
  codecs today.

With that, a full snapshot is the core state plus fixed-size stores: 96 objects, 8 actors, 32 collision slots and
40 panels, on the order of 30 KB, taken as one flat copy (a microsecond or two) with no heap walks. Serializing it (for
save states, desync reports or spectators) is the same codec.

## 6. Migration plan

Each step is one mergeable change that leaves behavior identical. The gate for every step is the same: `cargo test`
in this repository, the golden-trace suite's per-round frame floors unchanged, and the frontend's pixel comparison
unchanged (the last two live outside this repository). Steps that move timing-sensitive code also compare a state
digest tick by tick before and after on battles built in code.

1. **Snapshot and digest.** Derive `Clone` on `Battle` and add a digest of the simulation state (outputs excluded).
   In-repo tests: a battle cloned mid-round and run on both copies stays identical; a restored snapshot replays
   identically. Every later step reuses these as its lockstep check.
2. **A `core` module with the mechanism.** Move object pools and the list, the sprite stepper, RNG, input records, the
   sound buffer, the panel grid with reservations, and collision registration and pairing into one module owned by
   `Battle`. The hit kernel and panel conversions become ruleset functions the core calls through a trait with static
   dispatch (no cost). Tables the core reads (animation timing, regions, collision types, panel type flags) are passed
   in, not read from globals. Pure moves.
3. **The per-tick schedule as an explicit list** of ruleset systems in `tick_running`'s order.
4. **A kind registry.** Replace the two `match (pool, index)` sites with a static table of `KindDef`
   (pool, trace index, default flags, `scratch_position`, draw hints, state constructor, update). `trace::compare`
   and the frontend read the registry instead of hard-coded kind numbers.
5. **Action, weapon and navi registries.** `actions::dispatch`, `idle::weapon_routine` and the per-AI-index hook
   lists become tables (`ActionDef`, `WeaponDef`, `NaviDef`); the action table for actions below 0x10 moves onto
   `NaviDef`.
6. **Literals to properties, one family per change**: forms (`is_beast`, weapons, charge families, bonus table,
   overlay, flags), navis (hooks, NameID families), chips (chainable, aura, trap, dark), sounds (named cues).
   Properties the ROM keeps as tables are extracted; the ones the ROM expresses as code conditions are hand-authored
   data with the routine named.
7. **The `Ctx` façade.** Add `Ctx` with the operations of §2 and port content modules to it one at a time (sun beam,
   attachment, effect, rock, GunDelSol, the claw, the rush, Beast Out). A lint keeps content modules from naming
   `Battle` internals.
8. **Schema-declared state.** Replace `kinds::Vars`, `ActionVars` and the content fields of `ActorData` and `Battle`
   with stores declared per kind, action, navi, form and battle (a Rust macro first, the same shapes as today).
   Replace `&'static` references in state with ids (done, with step 9) and the `absorbed` vector with a fixed
   array. Snapshots become fixed-size.
9. **Content files.** bn6-extract writes the tables as content files in the format the asset prototype settles on;
   the engine loads a content pack, with the BN6 pack embedded as the default. Retire generated Rust tables one at a
   time. This needs a decision on the project's rule that extracted data becomes generated Rust (§7).
   **Done**, without the embedded default: `bn6-extract content` writes a pack (docs/design/content-pack.md) and the
   engine loads its battle data at run time into `Content`. No generated tables remain, and the engine has no
   default content; tests use a small hand-authored set (`content::testing`).
10. **Scripts behind the registries.** Let a `KindDef` or `ActionDef` be backed by a script from the scripting
    prototype. Port a visual kind first (the palette flash or the sun beam), then GunDelSol, then the rock. A
    differential test runs the Rust and script implementations side by side from the same snapshot every tick and
    compares digests; the Rust versions stay as the reference until the scripts match on every trace.
11. **Presentation from a view, not internals.** The frontend and audio consume a presentation view (objects with
    kind names, sprite ids, looks; a HUD model; named cues) and content-defined cue-to-song mapping, instead of
    reading `Battle` fields and kind numbers.
12. **Rollback.** Public snapshot and restore, a digest exchange for desync detection (perspective-normalized), the
    content hash in the netplay handshake, and cue deduplication in the frontend.

Steps 1-8 are refactors of Rust code with no new runtime; they deliver most of the boundary on their own. Steps 9
and 10 depend on the format and scripting prototypes; 11 and 12 can proceed in parallel after step 8.

## 7. Decisions for the user

1. **Is the ruleset scriptable?** The recommendation is no, at least at first: the ruleset is where bit-exactness is
   hardest and most verified, it is shared by all content, and keeping it Rust keeps it fast and reviewable. Content
   scripts see it through `Ctx`. A second game's rules would be a second ruleset crate on the same core.
2. **Content files versus generated Rust.** The project's rule today is that extracted game data becomes generated,
   committed Rust tables and the engine never reads the ROM. Content files keep the second half (still extracted by
   the tool, still no ROM at run time) but replace the first. One option keeps both: bn6-extract writes content
   files, and a build step embeds the BN6 pack so the default engine still needs no files at run time.
   **Decided: content packs only.** bn6-extract writes a pack from the user's ROM and the engine loads it at run
   time; nothing extracted is committed or embedded, and the engine never reads the ROM.
3. **Kind identity.** Keep the original pool and index as the identity of BN6 kinds (the traces and the frontend
   depend on it) and give new content ids outside that range.
4. **Register garbage.** Keep reproducing the garbage values that can be reproduced, or treat all spawn-register
   positions as unknown in comparisons? Keeping them costs little and keeps the trace comparison strict.
5. **State budgets.** Fixed per-scope maxima (the original's sizes) keep snapshots fixed-size; larger budgets for
   new content are a choice between snapshot size and freedom.
