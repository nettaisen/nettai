# Content model v2: game data in Luau, content by name

The user's direction (2026-09-30):

1. "move all the content into the lua implementations so none of the chips objects etc rely on the original
   indexes and can be more easily composed. only assets like graphics and music should be used, the rest should be
   part of the lua content pack"
2. "objects that are specific to chips should probably be moved into those chips"

This document is the design for that change: what the content pack becomes, how content names and composes
content, where the original's numbers go, what changes in the engine and its tools, and the migration that gets
there without losing a frame of the golden traces. It supersedes the parts of
[content-pack.md](content-pack.md), [scripting.md](scripting.md) and [content-migration.md](content-migration.md)
that say game data is extracted from the ROM, that registration is by original number, and that folders carry
index prefixes; those documents are rewritten in the last step of the migration (§12, step 13).

Words: a *definition* is a record made by one of the definers (`define.chip { ... }`); a *key* is a definition's
name (`"minibomb"`); a *handle* is the dense number a key interns to when content loads; *compat* is the table of
the original's numbers (§6). Routine names are the original's. "Dimming", "cut-in chip", "counter cut-in" and
"telop" are used as in the rest of the project.

## 0. Summary

- **The content pack is committed Luau, and it is the source of truth for all game data.** Chip records,
  navis, forms, weapons, stages and every rule table are definitions in content/bn6, next to the code that uses
  them: a chip's numbers in its module, a rule in the `rules/` module that holds it. From the ROM come only
  assets, with the animation timing that belongs to the sprites.
- **Extraction yields only assets**: sprites (with their animation timing), backgrounds, the field, the HUD's
  graphics and font, songs, sound effects, voicegroups and samples. `bn6-extract` loses its battle data. The
  committed Luau data is written once from the ROM by a generator that lives in the verification workspace and
  stays there as the cross-check (§9).
- **Content refers to content by value and by key, never by an original number.** A module `require`s what it
  uses and passes definitions around (`battle.spawn(bomb.kind, ...)`, `me.chip.name`); definitions that other
  data or setups name have string keys (`"minibomb"`, `"heatcross"`). No pool index, chip id, action number,
  subtype, weapon routine number, NameID, effect/spark/region number, collision type number, attachment row or
  table row appears in logic.
- **Composition replaces the subtype tables.** The original's "one handler plus a table indexed by subtype"
  becomes a library builder each chip calls with its own parameters: MiniBomb's module composes the throw action
  with the bomb it holds and the bomb it throws. Object kinds stay one per behaviour; builders parameterize them
  through the kind's typed state (§5).
- **Colocation.** A kind used by one chip, navi or form lives in its folder; kinds a family shares live in the
  family's `lib/` folder; `objects/` keeps only the 13 kinds that several families share. Folders are named by
  name (`chips/minibomb/`), with no index prefixes (§4).
- **The original's numbers are compat metadata** in content/bn6/compat, TOML files that no Luau module can load
  and no engine crate can reach (§6). The golden-trace harness, the setup codecs (real saves, folders, NaviCust,
  link navis), the extractor's asset naming and the sound comparison read them through a new crate,
  `bn6-compat`, which depends on the engine; the engine cannot depend on it.
- **Handles.** Keys intern to dense `u16` handles per registry, in byte-wise sorted key order, when content
  loads. Battle state, setups and inputs carry handles; the content hash covers every definition and script, so
  two peers whose setups agree run the same content with the same handles (§8).
- **The define phase** evaluates every module of the pack in the existing sandbox (the bytecode verifier, deep
  freezing, the deterministic library) with definers available only while content loads; it produces a canonical
  tree the loader turns into the typed Rust `Content`, plus the function slots (updates, hooks) the runtime calls.
  Each thread's runtime VM re-runs the same define phase and checks it gets the same tree. A throwaway spike ran
  this on bn6-luau's real sandbox (§7.3).
- **The Rust ruleset reaches content through roles, traits and registrations**, never numbers: `roles.kinds.
  absorbed_obstacle`, a form's `charged_chips` trait, a chip's `dimming` hook. Every numeric call site in the
  ruleset is listed in §7.5 with its replacement.
- **Frame-exactness is kept.** Nothing the traces compare changes meaning: objects are compared through compat
  (kind handle to pool and index, a navi's action to its number), hands through the compat codec. The fields that
  stop existing (spawn parameters, attack variants, chip subtypes, NameIDs) were never compared (§10).
- **Migration** (§12): 13 steps in four phases, main green and every floor held at each: foundations (one to two
  agents, about 10 agent-days), a family-by-family conversion that the four waiting agents join as soon as the
  exemplars land, the ruleset's de-numbering in parallel, and the removal of registration by number. About 35
  to 45 agent-days in all.

## 1. What changes

| | v1 (today) | v2 |
|---|---|---|
| Game data | Extracted from the ROM into pack TOML (chip.toml, navi.toml, form.toml, rules/, registries/) | Committed Luau definitions in content/bn6, written once by a generator |
| Scripts | Overlay in content/bn6, merged into the pack | The same Luau is the data and the code |
| Registration | `[kind] pool/index`, chip `script` by action and subtype, `weapon.toml` by routine number | Definitions: `define.kind`, a chip's `action`/`dimming`/`navi`/`instant`, `define.weapon` |
| Identity | Original numbers (chip 0x36, T3#0x08, form 1, weapon 0x06, NameID 0x1AC) | Keys (`"minibomb"`, `"bomb"`, `"heatcross"`, `"heatcross/charge"`) and handles |
| References | `data.chips[id]`, `spawn_kind("bomb")`, `me:param(1)`, `play_sound(0x70)`, `battle.effect(pos, 0x14)` | `require`, definition values, typed state, `asset.sound("bomb-hit")`, effect definitions |
| Original ids | Everywhere | content/bn6/compat, read only by `bn6-compat` |
| Folders | `chips/036-minibomb`, 139 kinds in `objects/` | `chips/minibomb`, kinds colocated with their owner, 13 in `objects/` |
| Assets | `graphics/sprites/CC-II`, `sound/songs/song-XXX` | Named: `graphics/sprites/bomb`, `sound/songs/bomb-hit` |
| The ruleset | Reads content by number (`form.0 == 0x0F`, chip `0xBB`, `SoundId(0x6B)`, `(Pool::Effect, 0x0A)`) | Reads roles, traits and handles |
| Tests in the repository | Hand-written made-up content (content/testing.rs) | The real BN6 definitions with synthetic animation timing, plus a small test pack |

What does not change: the core and the ruleset stay Rust; Luau stays the only content runtime, stateless over
engine-owned typed state; the sandbox, the verifier and freezing; `Battle: Clone`, snapshots and the digest;
the frame-exact behaviour of everything already ported.

## 2. Identity: keys, registries and handles

### 2.1 Registries

Every definition belongs to one registry. The engine knows the registries and their Rust types; one registry
(`record`) holds data only Luau reads.

| Registry | Definer | Key | The Rust side reads | Compat maps it to |
|---|---|---|---|---|
| chip | `define.chip` | required `id` | the record, its use (action or hook), traits | chip id; the action number and subtype it had |
| navi | `define.navi` | required `id` | stats, identity, own chip, traits | navi number, NameID |
| form | `define.form` | required `id` | stats, weapons, identity, traits | form number, NameID |
| weapon | `define.weapon` | required `id` | charge times, traits, the `setup` slot | weapon routine numbers (aliases many-to-one) |
| kind | `define.kind`, and the engine's own kinds | required `id` | pool, state schema, `update` and `place` slots | pool and index; `scratch_position`, `scratch_z_fraction` |
| action | `define.action`, and the engine's own navi actions | derived | state schema, `update` slot, traits | action number (the player's CurAction in the traces) |
| stage | `define.stage` | required `id` | layout, actors, music, background, settings | battle settings index and actor-list address |
| effect | `define.effect` | derived | sprite, animation, palette | (none) |
| spark | `define.spark` | derived | sprite, animation, palette | (none) |
| region | `define.region` | derived | panel offsets, or a whole-field condition | (none) |
| collision | `define.collision` | required `id` (shared vocabulary) | a collision type's flag words by side | (none) |
| status | `define.status` | required `id` (shared vocabulary) | requests, duration, timer | (none) |
| lockon | `define.lockon` | required `id` (shared vocabulary) | the Beast Out lock-on search | (none) |
| record | `define.record(type, spec)` | derived, or `id` | only its type name (Luau reads the fields) | (none, unless a setup names it) |
| sprite, sound, banner, background, mugshot, chip icon | `asset.*` (§6.3) | the asset's name | names; sprites' animation timing | the ROM's numbers, in compat/assets.toml |

Singletons, defined once per pack: `define.rules(section, spec)` for each rule table (§3.8) and `define.roles`
for what the ruleset needs by role (§7.4). Two more registries are internal: **identities** (the NameID records,
§3.2), nested in the navi, form or kind they belong to and keyed by it (`heatcross/identity`), which compat maps
to NameIDs through navis.toml and forms.toml; and **state schemas**, one per distinct state table (§3.5), which
the content state store is keyed by.

### 2.2 Keys

A key is a string unique within its registry.

- **Required keys** (`id = "..."`) for what is named from outside content: setups, folders, compat, tools, the
  frontend. Lowercase ASCII letters and digits in `-`-separated words, optionally qualified with `/` by an owner:
  `minibomb`, `atk-10`, `erasemn-ex`, `heatcross-beast`, `megaman/buster`, `eraseman/mark`. The generator (§9)
  makes chip keys from the in-game name (`M-Cannon` is `m-cannon`, `GrndMan[EX]` is `grndman-ex`, `Atk+10` is
  `atk-10`); where two records share a name (StepSwrd, WhiCapsl, BeastOut) or have none, it picks a
  key from the record's use and lists them in compat/curation.toml for review (§13).
- **Derived keys** for definitions made inside another definition's module and nested in it: `<owner key>/<field
  path>`. MiniBomb's action is `minibomb/action`; the bomb variant it throws is `minibomb/action/args/thrown`.
  A definition made while module `M` loads and not nested in a keyed definition of `M` is `M#n`, its place among
  `M`'s definitions (`lib/bombs/throw#1`). Owner-derived keys are stable under edits elsewhere and readable in
  messages and trace diffs; `M#n` keys shift when `M` gains a definition, which matters only to compat (an
  action compat maps takes an explicit `id`, §3.5). Nothing else stores a derived key.
- Kind keys follow a convention the checker warns about: a kind colocated with an owner is qualified by it
  (`eraseman/mark`, `grab/shot`); a kind in `objects/` or a family library is plain (`projectile`, `bomb`).
- The engine's own kinds and navi actions have keys in the `engine/` namespace (`engine/hitbox`, `engine/effect`,
  `engine/player`, `engine/move`, `engine/dimming-chip`), registered by the ruleset, not by content.

### 2.3 Handles and the intern order

After the define phase, the loader sorts each registry's keys byte-wise and numbers them from 0: that number is
the handle, a `u16` newtype per registry (`ChipHandle`, `KindHandle`, ...). The order depends only on the keys,
not on file order, require order or hash-map order; the spike loaded the same modules in two orders and got the
same handles. Content with more than 65,535 definitions in one registry is refused.

Handles are what battle state holds (an object's kind, a hand's chips, a navi's form, a state field of type
`chip`). They are valid only with the content that made them: the round's setup carries the content hash, and
`Battle::new` refuses other content, as it does today.

`Content::names` keeps each registry's keys by handle and the reverse map (a `BTreeMap`), for messages, for
setups written by name (`"minibomb"` with code `"B"`), and for tools.

### 2.4 Definition values in Luau

A definer returns its spec table, frozen, with a per-registry metatable; nothing is added to the table. Scripts
read a definition's fields directly (a frozen table read costs about 24 ns, against 130 ns for a userdata field).
Where the API expects a definition (`battle.spawn(kind, ...)`, a weapon's `setup` returning an action), the
binding looks the table up by identity: the runtime keeps each definition table's registry and handle by its
address (definitions are frozen and live as long as the VM, and the lookup is never iterated). At run time a
script only ever sees definitions that exist; it cannot make one (definers fail outside the define phase).

## 3. The definition API

The definers are fields of one global, `define`, and the asset resolvers of `asset`. Both work only while
content loads: a module's top level, and functions it calls while loading (builders). Everything is typed in
core.d.luau; `bn6-content-check` type-checks it (§7.7). Specs below show the fields; `?` marks optional ones.

### 3.1 Chips

```luau
export type ChipSpec = {
    id: string,
    name: string,              -- what the telop and the HUD show (UTF-8; the font's charmap draws it)
    description: string?,      -- the library text
    codes: { string },         -- "A".."Z", "*"
    element: Element,          -- "null" | "fire" | "aqua" | "elec" | "wood"
    family: ChipFamily,        -- the icon family: secondary elements, form bonuses, charged chips
    class: ChipClass,          -- "standard" | "mega" | "giga" | "special" | "program_advance"
    rarity: number, mb: number,
    damage: number | DamageFormula,
    hit_param: number?,        -- the counter byte the attack's hits carry
    lockout: number?,
    flags: { ChipFlag }?,      -- "dimming", "has_damage", "navi", "variable_damage", ...
    extra_flags: { ExtraChipFlag }?,
    beast: { lockon: Lockon }?, -- in Beast Out, the chip goes through the Beast rush with this lock-on
    library: { number: number, index: number, sort: number }?,
    slot_in_limit: number?,
    traits: { ChipTrait }?,    -- what the ruleset asks of a particular chip (§7.5)
    dark_substitute: Chip?,
    modifier: Modifier?,       -- what it does to the chip picked before it
    program_advances: { ProgramAdvanceRecipe }?,  -- the recipes that make this chip, in their order
    -- Exactly one of: how the chip is used.
    action: Action?,           -- a navi action: the chip's attack
    dimming: DimmingHook?,     -- a cut-in chip: (user, spec) -> controller
    navi: NaviChipHook?,       -- a navi chip: (user, controller, spec) -> navi
    instant: InstantHook?,     -- an instant chip: (user, spec) -> ()
}
```

- `damage` is a number, or a formula the ruleset evaluates: `formula.sp_navi { slot = sp.heatman, by_time =
  {...} }` (the SP chips' damage by the user's deletion time of that navi; `sp.heatman` is a record for the
  save's deletion-time slot, in `lib/navi-chips/sp.luau`), `formula.hp_lost { cap = 500 }` (Muramasa),
  `formula.navi_level { base = 60, per_level = 10 }` (the link navis' chips). The original's "1000 + n selects
  formula n" encoding is gone; the hand holds the evaluated damage, as it does today.
- `program_advances` refers to its ingredients by value: `{ order = 12, recipe = pa.code_run(cannon, 3) }`,
  `pa.sequence { hicannon, cannon, m_cannon }`.
- The chip record's bytes no battle routine reads stay out, as in v1.
- A chip's action number, subtype and parameters are gone. The parameters became arguments of the builder the
  chip composes (§5); the numbers are compat.

### 3.2 Navis and forms

```luau
export type NaviSpec = {
    id: string, name: string,
    sprite: Sprite, element: Element, weakness: { SecondaryElement }?,
    buster_bonus: number, move_lag: { number },   -- by navi variant
    banners: { win: Banner, lose: Banner },
    mugshots: { [Emotion]: Mugshot }?,
    merge_height: number?,                        -- in a Cross
    own_chip: { chip: Chip, code: string }?,      -- a link navi's chip, once a round
    chip_bonus: { family: ChipFamily, dimming_chips: boolean?, by_level: { number } }?,
    identity: Identity,                           -- the NameID record: attach points, actor type, parts
    actions: { [string]: Action }?,               -- a link navi's own actions
    traits: { NaviTrait }?,
}

export type FormSpec = {
    id: string, name: string,
    sprite: Sprite, element: Element, weakness: { SecondaryElement }?,
    buster_bonus: number,
    weapons: { buster: Weapon?, charge_shot: Weapon?, back_special: Weapon?,
               a_charge: Weapon?, alt_a_charge: Weapon?, mode9_a: Weapon? },
    identity: Identity,
    kind: "base" | "cross" | "beast" | "cross_beast" | "beast_over",
    game: ("gregar" | "falzar")?,                 -- which game's Beast
    cross_of: Navi?,                              -- the navi a Cross is made with
    beast: Form?,                                 -- a Cross's Beast form; the Beast forms' base
    palette: number?, mugshot: Mugshot?,
    overlay: BodyOverlay?,                        -- what the form wears (`sub_8011268`)
    chip_bonus: { family: ChipFamily, damage: number, dimming_chips: boolean? }?,
    charged_chips: { family: ChipFamily?, damaging: boolean?, also: { Chip }? }?,
    status_reset: { FormStatusTrait }?,           -- `sub_8014536`: "super_armor", "air_shoes", "floating", ...
    traits: { FormTrait }?,
}
```

`Identity` is what the original keys by NameID: the actor record (`actor_type`), the sprite's attach points, the
parts an actor record puts on (`sub_8010DF6`), and the classification the ruleset used NameID ranges for
(`player`, `cross`, `boss`, `obstacle`). A navi's or form's identity is nested in it; a kind that has one
(rocks, field objects) holds it in its definition or its variants. NameIDs themselves are compat.

### 3.3 Weapons

```luau
export type WeaponSpec = {
    id: string, name: string,
    charge_ticks: { number },   -- to a full charge, by Charge stat; past the table, what the game reads
    setup: (navi: Object) -> Action,   -- `off_80117D4[n]`: prepare the attack, name its action
    sticky: boolean?,           -- a charge-shot weapon that stays when another is set (`sub_800FFAA`)
    demotes: { [Weapon]: Weapon }?,     -- with this charge shot, these busters become those
    traits: { WeaponTrait }?,
}
```

A weapon's `setup` returns an action definition, which may be its own (the buster's shot) or a chip family's
(HeatCross's charged shot composes the burner action, §5.7). The original's alias routines (0x2E, 0x2F,
0x3E, ... all `sub_8011A26`) are one weapon, `megaman/buster`, with many numbers in compat.

Charge times past a weapon's own row are materialized. The original reads a Charge stat above 4 from the next
routine's row; the generator writes those values into the weapon's own list, so no definition depends on its
neighbour. The same rule applies wherever the original reads past a table into the next entry: the value is
written where it is used, with a comment naming the quirk.

### 3.4 Object kinds

```luau
export type KindSpec<S> = {
    id: string,
    pool: Pool,                  -- "actor" | "attack" | "effect": update order and slot limits
    state: StateSchema?,         -- `{ timer = "u16", variant = "record", owner = "object" }`
    update: (me: Object) -> (),
    place: ((spec: PlaceSpec) -> Object?)?,   -- when a stage's actor list places it (rocks)
}
```

A kind is one behaviour, one per original object routine. What differs between its uses goes into its state
when it is spawned. State fields gain reference types that hold handles: `"chip"`, `"kind"`, `"action"`,
`"effect"`, `"spark"`, `"region"`, `"sound"`, `"sprite"`, `"record"` (any record), `"record:<type>"` (records of one
type). A field of a reference type reads back as the definition (a frozen table) or nil. So `me:param(1)` goes:
the spawner writes `s.variant = variant`, and the kind reads `s.variant.palette`.

The pool is part of the kind's definition because it is behaviour: it decides the update order and which slot
limit a spawn hits. The index within the pool is compat.

### 3.5 Actions

```luau
export type ActionSpec<S> = {
    id: string?,                 -- usually derived (`minibomb/action`)
    state: StateSchema,          -- shared by identity: see below
    args: any?,                  -- the builder's parameters, as data (tools read them; keys derive from them)
    update: (me: Object, s: S) -> (),
    traits: { ActionTrait }?,    -- e.g. "beast_claw" (the chain exclusion in `sub_80127C0`)
}
```

An action compat has to map that isn't nested in a keyed definition (a weapon's action, which its `setup`
returns; a role's action) takes an explicit `id` (`"megaman/buster/shot"`); the checker requires compat to cover
every action, so a missing one is reported.

**State identity is the state table's identity.** The original's attack scratch is one block every action
shares; the engine models it as "the next action with the same state continues from it, another starts from
zero". In v2 a builder declares its state table once at its module's top, and every action it builds shares it,
so MiniBomb's throw continues from EnergBom's as action 0x12 did, and a chip whose action is its own starts from
zero. The spike checked that three builder-made actions share one schema. Each distinct state table is a schema
definition keyed `<module>#state` (a module's own `state` export, registration by module) or
`<registry>:<key>/state` (a kind's or action's), claimed in that order; a kind or action without a `state` uses
the empty layout.

**Which action runs.** Per-chip action instances share their original number (MiniBomb's and BigBomb's both show
0x12 in the navi's CurAction), so the number can't select the action. Starting an attack sets both: the navi's
action byte (the number the traces compare) and the attack header's `content_action`, the handle of the action to
run (`set_attack` takes a `NaviAttack { number, content }`; a plain number means "the engine's action, or the one
registered by that number"). Chip use takes the handle from the chip's definition, a weapon routine from what its
`setup` returns, and dispatch runs `content_action` when it is set.

A builder stores its parameters in `args` rather than only capturing them. The canonical tree then shows what
each chip is made of (`bn6-content show minibomb`), and definitions nested in the arguments get derived keys.

### 3.6 Effects, sparks, regions, collision types, statuses, lock-on modes

```luau
local EXPLOSION = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local WIDE = define.region { panels = { { 1, -1 }, { 1, 0 }, { 1, 1 } } }
local ENEMY_PANELS = define.region { field = { require = 0x20, forbid = 0 } }  -- a whole-field region
-- rules/collision.luau: collision types. An object's registration takes two, what it is and what it hits
-- (`setup_collision(self_type, target_type, hit_mod)`), as a `CollisionSpec`.
collision.thrown = define.collision { id = "thrown", side0 = 0x80000088, side1 = 0x40000088 }       -- type 0x0A
collision.hits_navis = define.collision { id = "hits-navis", side0 = 0x15800000, side1 = 0x2a800000 } -- type 0x05
local PARALYZE = define.status { id = "paralyze", requests = 0x..., duration = 0x78, timer = "paralyze" }
```

(The flag words are the real rows 0x0A and 0x05, the thrown bomb's; the names are the kind the generator gives
them, §9.3, and the numbers in the comments are for this document only: they live in no file.) A kind or hit that
used `self_type = 0x0A, target = 0x05` now says `collision = { is = collision.thrown, hits = collision.hits_navis }`.

Shared ones live in `lib/effects.luau`, `lib/sparks.luau`, `lib/regions.luau`, `rules/collision.luau`,
`rules/status.luau`, `rules/lockon.luau`, exported by name; ones a single owner uses live with it. Their numbers
disappear: nothing compared reads them (the generic effect's first parameter was its effect id; parameters are
not compared, §10). Region 0 ("none") is `nil`, and region 1 (the anchor panel) is `regions.single`.

### 3.7 Stages

```luau
export type StageSpec = {
    id: string,
    layout: { string },          -- three rows of six panel types: `{ "normal normal ... ", ... }`
    panel_pattern: PanelPattern, -- which columns are whose
    music: Sound, background: Background,
    mode: number, battle_number: number, effects: { BattleEffect }?,
    actors: { ActorEntry },      -- `{ place = navi, side = 0, x = 2, y = 2 }`, `{ place = rock.plain, x = 3, y = 1 }`
}
```

Panel layouts and actor lists are inlined into the stages that use them; neither has a number any more. The 192
battle settings a set's later rounds are drawn from are deduplicated into stages (one per distinct record; all 192
are distinct: 96 pairs of layout and actor list, each with two effect words); compat maps each original index to its
stage, and gives the address its actor list goes by. Stages are named by what they are where that is known, else
`netbattle-N` in order of first use (§13).

### 3.8 Rules

Each rule table is a section, defined once:

```luau
-- rules/elements.luau
--!strict
-- Element weakness: extra damage multiplier by the receiver's element, then the hitter's (null, fire,
-- aqua, elec, wood, drain); the secondary elements a chip family adds to its attacks.
return define.rules("elements", {
    weakness = {
        null = { 0, 0, 0, 0, 0, 0 }, fire = { 0, 0, 1, 0, 0, 0 }, aqua = { 0, 0, 0, 1, 0, 0 },
        elec = { 0, 0, 0, 0, 1, 0 }, wood = { 0, 1, 0, 0, 0, 0 }, drain = { 0, 0, 0, 0, 0, 0 },
    },
    family_elements = { sword = { "sword" }, cursor = { "cursor" }, wind = { "wind" }, ["break"] = { "break" } },
})
```

| Section | Module | Holds (v1 file) |
|---|---|---|
| `elements` | rules/elements.luau | weakness by receiver and hitter; family secondary elements (rules/elements.toml) |
| `panels` | rules/panels.luau | panel type flags and road slides; start visibility, front edges; step rules (rules/panels.toml) |
| `reactions` | rules/reactions.luau | push and ice slides, the bubble's bob (rules/reactions.toml) |
| `status` | rules/status.luau | the HP bug's drain periods; the statuses themselves are `define.status` (rules/status.toml) |
| `lockon` | rules/lockon.luau | column shifts, the clear path, the charged sword's modes; modes are `define.lockon` (rules/lockon.toml) |
| `berserk` | rules/berserk.luau | Beast Over's berserk panel rules (rules/berserk.toml) |
| `math` | rules/math.luau | the sine table (rules/math.toml) |
| `custom_screen` | rules/custom-screen.luau | the slot grid and neighbour scans (rules/custom-screen.toml) |
| `buster` | rules/buster.luau | recovery by Rapid and open panels; the empty hand's chip (rules/weapons.toml) |
| `banners` | rules/banners.luau | which banners hold until removed, by banner asset (rules/banners.toml) |

Where v1 kept per-entity rows in a shared table, they move to the entity: charge times into weapons, the Cross
palettes into forms, the SP chips' deletion-time steps into `lib/navi-chips/sp.luau` next to the formula,
collision types into `rules/collision.luau` as named definitions, the actor records into identities,
`registries/*` into named definitions (§3.6). rules/stages.toml becomes `stages/`.

### 3.9 Records

`define.record(type, spec)` makes Luau-only data with a handle, so a spawned object can remember which one it is
(a bomb variant, a projectile variant, a rock variant, an attachment look, a body overlay). Rust stores the
handle and knows the type name; it never reads the fields. Builders wrap it (`bomb.variant { ... }` is
`define.record("bomb-variant", ...)`), so chips don't call it directly.

### 3.10 Hooks: the function slots

The ruleset calls content through functions stored in definitions. These replace v1's `Hook` table by number.

| Slot | Signature | The ruleset calls it | v1 |
|---|---|---|---|
| `kind.update` | `(me)` | every tick the object runs | `[kind]` by pool and index |
| `kind.place` | `(spec: PlaceSpec) -> Object?` | when a stage's actor list places the kind | `Hook::ActorListEntry(type)` |
| `action.update` | `(me, s)` | every tick the navi runs the action | an action number's module |
| `weapon.setup` | `(navi) -> Action` | when a button's weapon fires (`off_80117D4`) | `Hook::Weapon(n)` |
| `chip.dimming` | `(user, spec) -> Object?` | when a cut-in chip is used (the dimming chip action) | `Hook::DimmingChip(subtype)` |
| `chip.navi` | `(user, controller, spec) -> Object?` | when a navi chip's navi comes (`sub_80E1880`) | `Hook::NaviChip(subtype)` |
| `chip.instant` | `(user, spec)` | once, by the instant chip action (`sub_80EC39C`) | `Hook::InstantChip(subtype)` |

A slot is found by the definition's handle, so the ruleset's dispatch is an array index, as v1's tables by number
were. The specs (`DimmingChipSpec`, `NaviChipSpec`, `InstantChipSpec`) lose `params`, and carry the chip as a
chip where they carried its id.

**As built** (step 9): the `Hook` table and `Defs::hook` are gone. Registration by number resolves at load into
the definitions' slots: every chip has its `ChipUsage` (a pack record's from its action, or from its subtype's
`dimming_chip`, `navi_chip` or `instant_chip` registration; `Unported` for what nothing implements, which fails
where the original runs it, with the subtype in the error); a weapon that names an instant effect no chip has
gets it as `WeaponDef::instant`; a v1 kind's `actor_list_entry` is its `place` (`KindDef::place`, which
`define.kind` also takes). The instant chips' action runs the attack's `instant` effect, which chip use and such
a weapon set; the dimming and navi chip actions read the chip's usage. Content's function roles
(`hooks.first_barrier`) are in §7.4.

## 4. Folder layout

### 4.1 The rules

```text
content/bn6/
  core.d.luau  types.d.luau          the API and shared types
  chips/<id>/                         one chip: chip.luau, and kinds only it uses
  chips/<series>/                     a series (X1-X3, Hi-/M-, EX/SP, Recov*, the upgrades of one chip): all of them
  navis/megaman/                      navi.luau; kinds and weapons several forms share
  navis/megaman/forms/<form>/         form.luau; weapons and kinds only that form uses
  navis/megaman/weapons/<weapon>/     weapons several forms share (buster, charged shot, blank shot, beast claw)
  navis/<navi>/                       a link navi: navi.luau, its own chip (action 0x0A) and actions
  lib/<family>/                       builders a family composes, and the kinds it shares across folders
  lib/*.luau                          helpers with no family (panels, slot, trajectory, element, hp, effects, sparks, regions)
  objects/<kind>/                     kinds several families share
  rules/                              rule sections, collision types, statuses, lock-on modes, roles
  stages/                             stages
  compat/                             the original's numbers (TOML; not loadable by content, §6)
```

1. A kind used by exactly one chip, series, navi or form lives in that owner's folder.
2. A kind shared by the members of one family (the bombs, the swords, the dragons) lives in the family's `lib/`
   folder, next to the builder the family composes.
3. A kind a Program Advance or an upgrade reuses from its family's series lives with the series (`chips/aurahed/`
   for StreamHd), and the upgrade requires it.
4. A kind a navi chip series shares with the same navi's link-navi chip lives with the navi chip series
   (`chips/eraseman/beam.luau`, required by `navis/eraseman/`).
5. Everything else shared across families is in `objects/`.
6. Folders and files are named by name: no index prefixes. A series folder is named by its plainest member
   (`chips/cannon/` for Cannon, HiCannon, M-Cannon; `chips/recov/` for Recov10 to Recov300).

`bn6-content where <key>` prints the module that defines a key, and load errors name the module.

### 4.2 Every current `objects/` entry

The destinations below apply the rules to the modules that use each kind today. "Series" folders hold all their
chips; the WIP kinds without an `object.toml` (unregistered) are included.

| Kind (v1) | Used by | v2 home |
|---|---|---|
| absorbed-obstacle | the obstacle framework (Rust, absorb) | objects/absorbed-obstacle (role) |
| air-spin | AirSpin1-3 | chips/airspin/ |
| aqua-needle | needle-volley (AquaNdl1-3) | chips/aquandl/ |
| aqua-surge | SpoutCross Beast's charged shot | navis/megaman/forms/spoutcross-beast/ |
| area-grab | PanlGrab, AreaGrab | lib/grab/ (controller) |
| attachment | 24 modules (bombs, swords, GunDelSol, the buster, navis...) | objects/attachment; its rows become attachment looks (records) defined by their users |
| aura-head | AuraHed1-3, StreamHd | chips/aurahed/ |
| bass | Bass | chips/bass/ |
| bass-anly, bass-anly-shot | BassAnly | chips/bassanly/ |
| black-bomb | BlkBomb | chips/blkbomb/ |
| blast-man, blast-fire | BlastMan series | chips/blastman/ |
| bomb | MiniBomb, EnergBom, MegEnBom, BigBomb | lib/bombs/ |
| bomb-slash | bomb's after-blast no BN6 chip uses | lib/bombs/ |
| boomerang | Boomer, HiBoomer, M-Boomer; TomahawkCross's throw | objects/boomerang |
| bubble-star | BblStar1-3 | chips/bblstar/ |
| bug-bomb | BugBomb | chips/bugbomb/ |
| charge-man, charge-car | ChrgeMan series | chips/chrgeman/ |
| charge-wave | ChargeCross Beast's charged shot | navis/megaman/forms/chargecross-beast/ |
| col-army | ColArmy | chips/colarmy/ |
| col-force, col-force-soldier | ColForce | chips/colforce/ |
| colonel | Colonel series, CrossDiv | chips/colonel/ |
| copy-mark | CopyDmg | chips/copydmg/ |
| countdown-bomb | time-bom | chips/timebom/ |
| crack-shot | CrakShot, DublShot, TrplShot | chips/crakshot/ |
| dash-hit | GroundCross Beast's dash, ChargeCross's tackle | navis/megaman/ |
| delta-ray, follow-effect | DeltaRay | chips/deltaray/ |
| dragon-head, dragon-body | HeatDrgn, ElecDrgn, AquaDrgn, WoodDrgn | lib/dragons/ |
| drill | DrilArm; GroundCross's drill | objects/drill |
| drip-shower | DripShwr (SpoutMan's link chip) | navis/spoutman/ |
| dust-ball | DustCross's charged shot | navis/megaman/forms/dustcross/ |
| dust-cloud | DustBrk (DustMan's link chip) | navis/dustman/ |
| dust-storm, dust-storm-mote | instant effect 17 (no chip yet) | lib/instant/ until a link navi's weapon claims it |
| eagle-tomahawk, tomahawk-strike | ETomahwk (TomahawkMan's link chip) | navis/tomahawkman/ |
| elec-man, elec-thunder | ElecMan series | chips/elecman/ |
| elec-pulse | ElcPuls1-3, DestPuls | chips/elcpuls/ |
| elem-trap, elem-trap-strike | ElemTrap | chips/elemtrap/ |
| element-pillar | HeatCross Beast's and ElecCross Beast's charged shots | navis/megaman/ |
| elmnt-man, elmnt-bolt, elmnt-ice, elmnt-vine, meteor | ElmntMan series | chips/elmntman/ |
| energy-burst | EnergBom, MegEnBom (through bomb) | chips/energbom/ |
| erase-man, erase-mark | EraseMan series | chips/eraseman/ |
| erase-beam | EraseMan's navi; EDeletBm | chips/eraseman/ (rule 4) |
| erase-drop | EraseCross Beast's drop | navis/megaman/forms/erasecross-beast/ |
| erase-ray | EraseCross's charged shot | navis/megaman/forms/erasecross/ |
| falling-rock, rock-chip | the ruleset (the rock barrage in chip use) | objects/falling-rock (role) |
| fire-hit | FireHit1-3 | chips/firehit/ |
| flame | FireBrn1-3, WideBrn1-3; HeatCross's charged shot | lib/burner/ |
| flame-hook, flame-hook-fire | FlmHook1-3 | chips/flmhook/ |
| flash-bomb | FlshBom1-3 | chips/flshbom/ |
| flying-shot | TrnArrw, the buster's throw, the Falzar beast's buster | objects/flying-shot |
| gauge-speed | SloGauge, FstGauge | lib/gauge-speed/ |
| golem | GolmHit1-3 | chips/golmhit/ |
| grab-shot | area-grab | lib/grab/ |
| great-yoyo, yoyo | YoYo, GreatYo | chips/yoyo/ |
| guardian, guardian-statue, guardian-strike | Guardian (written, unregistered) | chips/guardian/ |
| gust | WindRack; TenguCross's wind | objects/gust |
| heat-man, heat-flame | HeatMan series | chips/heatman/ |
| hit-flash, lunge-slash | SlashCross Beast's lunge | navis/megaman/forms/slashcross-beast/ |
| hockey-puck | AirHocky, PitHocky | chips/airhocky/ |
| honey-bee | RskyHny1-3 | chips/rskyhny/ |
| hyper-burst | H-Burst | chips/h-burst/ |
| immobilizer | instant effect 9 (no chip yet) | lib/instant/ |
| invisible | Invisibl, WhiCapsl, instant effect 2, seeking-whirl | objects/invisible |
| iron-shell | IronShl1-3, ParaShl | chips/ironshl/ |
| junk-shot | DustCross Beast's scatter | navis/megaman/forms/dustcross-beast/ |
| justice-one | JustcOne | chips/justcone/ |
| lance | Lance | chips/lance/ |
| mine, land-mine | Mine | chips/mine/ |
| magnet | MagCoil | chips/magcoil/ |
| meteor-shower | instant effect 16 (no chip yet) | lib/instant/ |
| moon-blade | MoonBld | chips/moonbld/ |
| navi-boost | PunchArm, NeedlArm, PuzzlArm, BoomrArmSyncTrgr, DarkInvs, BugRSwrd, HubBatc, BgDthThd | lib/navi-boost/ |
| navi-effect | DElecSwd (ElecMan's link chip) | navis/elecman/ |
| needle-volley | AquaNdl1-3 | chips/aquandl/ |
| panel-bursts | black-bomb, countdown-bomb, elem-trap-strike | objects/panel-bursts |
| panel-strike | bass | chips/bass/ |
| projectile | the buster, the cannons, AirShot and many more (lib/projectile) | objects/projectile (with lib/projectile.luau's helpers) |
| proto-man | ProtoMan series | chips/protoman/ |
| reflected-shot, reflector-shield | Rflectr1-3 | chips/rflectr/ |
| riding-hit | RSlash (SlashMan's link chip) | navis/slashman/ |
| rising-bubble | the plus chips' instant effect, black-bomb, bug-bomb, guardian-statue | objects/rising-bubble |
| rock, rock-debris | stages (actor lists), rock-cube | objects/rock/ (debris colocated) |
| rock-cube | RockCube, IceCube | chips/rockcube/ |
| roll, roll-heart | Roll series | chips/roll/ |
| rolling-log | RlngLog1-3 | chips/rlnglog/ |
| sand-worm, sand-hole, sand-spray | SandWrm1-3 | chips/sandwrm/ |
| seed | GrasSeed, IceSeed, PoisSeed | lib/bombs/ |
| seeking-whirl | AirSpin1-3 | chips/airspin/ |
| shock-wave | WaveArm1-3, PwrWave1-3 | chips/wavearm/ (to objects/ when a second family uses it) |
| slash-man, slash-wave | SlashMan series | chips/slashman/ |
| spout-man, spout-ball, spout-geyser, spout-mark, spout-pillar, spout-splash | SpoutMan series | chips/spoutman/ |
| sun-beam | GunDelS1-3, GunDelEX | chips/gundels/ |
| sun-moon, moon-beam, sun-meteor | SunMoon | chips/sunmoon/ |
| sword-wave | SlashCross's charged shot | navis/megaman/forms/slashcross/ |
| tengu-man | TenguMan series | chips/tenguman/ |
| tengu-tornado | FTornado (TenguMan's link chip) | navis/tenguman/ |
| thunder-ball | Thunder, DarkThnd | chips/thunder/ (DarkThnd's folder requires it) |
| thunder-column | EraseCross's ray, DolThdr's doll | objects/thunder-column |
| thunder-doll | DolThdr1-3 | chips/dolthdr/ |
| time-bom | TimeBom1-3, TimeBom+ | chips/timebom/ |
| tomahawk-man | TmhkMan series | chips/tmhkman/ |
| tornado | Tornado, Static | chips/tornado/ |
| trap-chip | AntiNavi, AntiDmg, AntiSwrd, AntiRecv, ElemTrap, BodyGrd | lib/traps/ |
| whirlwind | TenguCross Beast's charged shot | navis/megaman/forms/tengucross-beast/ |

What stays in `objects/`: absorbed-obstacle, attachment, boomerang, drill, falling-rock (with rock-chip),
flying-shot, gust, invisible, panel-bursts, projectile, rising-bubble, rock (with rock-debris), thunder-column.
Thirteen folders from 139.

The rest of v1's layout moves as follows: `lib/sword.luau` and `lib/vari_sword.luau` into `lib/swords/`,
`lib/dragon.luau` into `lib/dragons/`, `lib/instant-chips/` into `lib/instant/`, `lib/buster.luau` into
`navis/megaman/weapons/buster/`; every `chips/NNN-name` folder into its chip's or series' folder; the 46
`navis/00-megaman/weapons/NN-name` folders into the form that uses each (or `navis/megaman/weapons/` when
several forms do), with the 17 `NN-buster` alias folders gone (compat names the aliases).

## 5. Composition patterns

Each pattern below replaces a table indexed by subtype or by a spawn parameter. A builder is a function in a
`lib/` module that returns a definition (an action, a hook, a record) made from its arguments; the chip module
calls it with the chip's own parameters. Builders share one state table per behaviour (§3.5) and one kind per
original object (§3.4).

### 5.1 Bombs: an action, a thrown kind, variants

v1: `chips/036-minibomb/chip.luau` implements action 0x12 for 16 chips with `THROWS[me.variant]` (thrower,
held attachment row, held animation), and `objects/bomb` switches on `me:param(1)` through `KINDS[...]`.

```luau
-- lib/bombs/throw.luau: the bombs' and seeds' throw (the original's action 0x12, `sub_80EB628`).
--!strict
local attachment = require("../../objects/attachment/attachment")

export type ThrowSpec = {
    -- What the navi holds while winding up, and the palette offset it takes.
    held: attachment.Look,
    held_palette: number?,
    -- Throws the thing from `pos` with the attack's damage word.
    thrower: (user: Object, pos: Vec3, damage: number) -> Object?,
}
export type State = { timer: number }

-- One state for every throw: a throw continues the last one's scratch, as the
-- original's single action does.
local STATE = { timer = "u16" }
local SOUND = asset.sound("throw")
local ANIM_THROW, RELEASE_TICK, THROW_TICKS, RECOVER_TICKS = 6, 9, 0x15, 5

local throw = {}

function throw.action(spec: ThrowSpec): Action
    -- `sub_80EB644`: wind up, throw on the tenth tick, stand.
    local function wind_up(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_THROW)
            me:open_counter_window()
            attachment.spawn(me, spec.held, { palette = spec.held_palette or 0 }, "related")
            battle.play_sound(SOUND)
            me:set_status("using_action", true)
            s.timer, me.step_init = 0, 4
        end
        if s.timer == RELEASE_TICK then
            spec.thrower(me, throw.release_point(me), throw.damage_word(me))
            me.related1, me.overlay = nil, nil
        end
        s.timer += 1
        if s.timer >= THROW_TICKS then
            me.step, me.step_init = 4, 0
        end
    end
    -- ... `sub_80EB758` recover, `sub_80EB628` dispatch, as today ...
    return define.action { state = STATE, args = spec, update = update }
end

return throw
```

```luau
-- lib/bombs/bomb.luau: the thrown bomb (the original's attack object #8, `sub_80C5BB0`).
--!strict
local regions = require("../regions")

export type Variant = {
    collision: CollisionSpec, hit_mod: number, palette: number, anim: number,
    blast: Region?,                                     -- what it blasts where it lands (nil: nothing)
    after: ((me: Object) -> ())?,                       -- what it leaves after landing
}
export type State = { variant: Variant }

local bomb = {}
bomb.kind = define.kind {
    id = "bomb",
    pool = "attack",
    state = { variant = "record:bomb-variant" },
    update = update,      -- as today, reading (me.state :: State).variant instead of KINDS[me:param(1)]
}
bomb.held = attachment.look { sprite = asset.sprite("bomb"), palette = 0 }

function bomb.variant(v: Variant): Variant
    return define.record("bomb-variant", v)
end

-- `sub_80C5DBC`: a thrower for `throw.action` that throws this variant.
function bomb.thrower(v: Variant)
    return function(user: Object, pos: Vec3, damage: number): Object?
        local o = battle.spawn(bomb.kind, pos)
        if o then
            (o.state :: State).variant = v
            o.alliance, o.flip, o.related1 = user.alliance, user.flip, user
            o:set_damage_word(damage)
        end
        return o
    end
end

return bomb
```

```luau
-- chips/minibomb/chip.luau
--!strict
local throw = require("../../lib/bombs/throw")
local bomb = require("../../lib/bombs/bomb")
local regions = require("../../lib/regions")
local collision = require("../../rules/collision")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "minibomb",
    name = "MiniBomb",
    codes = { "B", "L", "R", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 0, mb = 6, damage = 50, hit_param = 30,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.throw },
    library = { number = 58, index = 58, sort = 317 },
    slot_in_limit = 3,
    action = throw.action {
        held = bomb.held,
        thrower = bomb.thrower(bomb.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 0, palette = 0, anim = 1,
            blast = regions.single,
        }),
    },
}
```

```luau
-- chips/bigbomb/chip.luau: the same throw, a bigger blast.
return define.chip {
    id = "bigbomb", name = "BigBomb", -- ... the record ...
    action = throw.action {
        held = bomb.held, held_palette = 3,
        thrower = bomb.thrower(bomb.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 3, palette = 3, anim = 1,
            blast = regions.square_3x3,
        }),
    },
}
```

These are v1's `KINDS[0]` and `KINDS[3]` rows (`byte_80C5BA0` with the blast regions of `dword_80C5D7C`) and
`THROWS[0x0]` and `THROWS[0xF]`, now written where they are used.

**As built** (step 7, the bombs exemplar): content/bn6/lib/bombs/ (`throw`, `bomb`, `seed`, `slash`),
chips/{minibomb, bigbomb, energbom, flshbom, blkbomb, bugbomb, grasseed, iceseed, poisseed}/, the attachment
(objects/attachment) and the shared definitions they need (rules/collision.luau, lib/effects.luau,
lib/sparks.luau, lib/regions.luau). What it settled:

- **A thrower is a table**, `{ throw = function, variant = record }`: a closure alone would hide the variant
  from the canonical tree, which then shows what each chip throws (`minibomb/action/args/thrower/variant`).
  `ThrowSpec`, `BombVariant`, `SeedVariant`, `FlashBombVariant` and `AttachmentLook` are in types.d.luau.
- **Attachment looks are records** (`attachment.look { sprite, palette, lift?, attach_point?, by_owner? }`,
  `attachment.attach(owner, look, slot, { anim, while_dimmed, palette_add })`); the attachment kind is a
  definition. Its numeric API (`spawn`, `spawn_with`, by the pack data's rows) stays for its 23 other users:
  the rows become looks at load, from `data`.
- **The chip records** are the pack data's values field by field. `beast = { lockon = 5 }` (the Beast rush and
  its lock-on mode) takes the mode's number, as 3b reads it, until step 5's generator writes rules/lockon.luau
  and the chips name its modes.
- **Shared definitions the generator also writes.** The exemplar made rules/collision.luau (ten types), lib/effects.luau,
  lib/sparks.luau and lib/regions.luau with the entries it uses. The generator must add the entries these
  modules lack rather than skip them (it identifies a collision type by `row_offset`, a region or effect by its
  numbers). The collision types' names other than `thrown` and `hits-navis` were made up by what their flags do:
  `attack` (row 0x04), `piercing` (0x0B), `hit-by-other-side` (0x14), `hit-by-other-side-or-blockers` (0x0D),
  `own-object` (0x0C), `own-thrown-body` (0x4E), `neutral-object` (0x4F), `everything` (0x0F): to review.
- **A definition is the chip** (3b): compat's key maps a recorded chip to it, and it runs its own action (the
  navi's CurAction reads as 0x12 through compat actions.toml's `minibomb/action`). A chip that other records or
  the ruleset still name by number keeps the pack's record, and its module gives only the action, with its
  compat key as `id` (`throw.action { id = "poisseed/action", ... }`): PoisSeed, whose number PoisPhar's recipe
  names. Registration by number reaches such records through a shim, chips/036-minibomb/chip.luau, which runs
  the chip's own action by subtype (and FlshBom's level). Records still reach it: PoisSeed; LilBoiler and
  VDoll (not ported: they wind up and fail where they throw, as before); the Cross special's MiniBomb, EnergBom
  and MegEnBom, which the ruleset's table picks by number and so gets the pack's records; and the test
  content's numbered bombs. The shim goes when those are definitions or roles (step 5, phase C).
- **What stays numeric**, having no v2 form yet: statuses (the flash's blinding, the bug bomb's 0x20), bug codes,
  NameIDs (the BlkBomb's 0xD5, the attachment's Cross check) and the absorbed-obstacle kind; the hitbox's
  `hit_effect = 0xFF` ("none"). The ratchet counts what it can see of them.
- **Verified** on the test content (the thrown chips' duel under rollback, the engine's tests), the type
  check, and the traces (at every latency) and chip lab on a pack extracted with asset names (step 6): the
  lab's matches are unchanged.

EnergBom and MegEnBom (a series, one folder) pass `after = energy_burst.leave` from their own
`chips/energbom/burst.luau`; the bomb kind no longer requires the energy burst. FlshBom1-3 pass their own
`held_palette` (0, 3, 6: the original's level times three, materialized) and `flash_bomb.thrower` from
`chips/flshbom/`; the seeds pass `seed.thrower(...)` from `lib/bombs/seed.luau`. LilBoiler and VDoll, `wip()`
stubs today, become chips whose throwers are written when they are ported; no table has an empty row for them.

### 5.2 Swords: one slash action, per-chip blades and hits

v1: `chips/047-sword/chip.luau` serves 17 chips from `data.chips[id].sword` and branches on the variant for the
sound, CrosSwrd's second hit, DblDream's two swings and the elemental swords' palettes.

```luau
-- lib/swords/slash.luau: the swords' action (the original's 0x13, `sub_80EB776`).
export type SlashSpec = {
    blade: attachment.Look,       -- what the navi holds (the original's `byte_80EBB64` row)
    hit: HitSpec,                 -- region, collision, hit modifier, status, bug, spark
    effect: Effect,               -- the slash drawn over the panel ahead
    sound: Sound,
    swings: number?,              -- DblDream: 2
    second_hit: HitSpec?,         -- CrosSwrd: a single-panel hit after the first
    effect_palette: number?,      -- the elemental swords
    step: boolean?,               -- StepSwrd: step two panels ahead first
}
function slash.action(spec: SlashSpec): Action ... end
```

```luau
-- chips/sword/chip.luau
local swords = require("../../lib/swords")
local SLASH = { is = collision.slash, hits = collision.hits_navis }
return define.chip {
    id = "sword", name = "Sword", damage = 80, -- ... the record ...
    action = swords.slash.action {
        blade = swords.blades.sword,
        hit = { region = regions.single, collision = SLASH, hit_mod = 3 },
        effect = swords.effects.slash,
        sound = swords.sounds.slash,
    },
}

-- chips/wideswrd/chip.luau
return define.chip {
    id = "wideswrd", name = "WideSwrd", damage = 80, codes = { "H", "L", "S", "*" }, -- ... the record ...
    action = swords.slash.action {
        blade = swords.blades.sword,
        hit = { region = regions.wide, collision = SLASH, hit_mod = 3 },
        effect = swords.effects.slash_wide,
        sound = swords.sounds.slash,
    },
}
```

The v1 per-chip `[sword]` data (blade row 3 for both, the slash's regions 1 and 4, collision types 0x07 and 0x05,
modifier 3, effects 24 and 22) becomes these arguments, and `lib/swords/` names the blades, effects and sounds
the family shares. MchnSwrd/ElemSwrd/AssnSwrd (action 0x49) and VarSwrd/NeoVari compose
`lib/swords/` builders the same way; `VariSwordSpec.choices`, today a list of chip numbers, becomes a list of
chips.

**As built** (step 7, the swords exemplar): content/bn6/lib/swords/ (`parts`: the blades, slashes and sounds
the family shares, and the blade's animation and palette by the navi's arm; `slash`: action 0x13's builder;
`strike`: action 0x49's; `vari`: the variable swords' library, moved from lib/vari_sword), and a folder per
chip. `SlashSpec` and `StrikeSpec` are in types.d.luau; DblDream's two swings, CrosSwrd's second hit,
StepSwrd's step and the elemental swords' colours are arguments (`swings`, `second_hit`, `step`,
`effect_palette`), and each chip's blade is an attachment look. What it settled:

- **Which chips are definitions.** StepSwrd, FtrSword, CrosSwrd, DblDream, MchnSwrd, ElemSwrd and AssnSwrd.
  SlashCross charges every Sword-family chip, and its charged slash (weapons 0x11 and 0x12, action 0x41) reads
  the chip's subtype (the blade, the wave, CrosSwrd's two waves, DblDream's two slashes) and first parameter
  (StepSwrd's dash) from its record: the four Sword-family definitions carry them in the transitional
  `legacy = { subtype, params }` marker (the lab's `chips/0x051-stepswrd/cross-slash-charged` fails without
  it). The other swords keep the pack's records, each module giving its chip's action with its compat key as
  `id`, because the ruleset or other records name them by number: Sword, WideSwrd, LongSwrd, WideBlde,
  LongBlde and LifeSrd are Program Advance ingredients or results; DrkSword is one of the ruleset's dark chips
  (0x11E to 0x122) and has a substitute chip; SlashCross charges FireSwrd to BambSwrd by number (0x4C to 0x4F);
  Muramasa's damage is a formula (the damage taken); ProtoMan's own StepSwrd (chips/stepswrd/protoman.luau) is
  a link navi's chip (0x190 to 0x19A) with damage by his level. The list of what goes when is §12's
  transitional list.
- **The shims.** chips/047-sword/chip.luau (action 0x13) runs a record's slash by subtype (and StepSwrd's first
  attack parameter, ProtoMan's copy by his chip), for the records above, the Cross special's swords, the
  variable swords' picks and the test content's blades. chips/056-mchnswrd/chip.luau (action 0x49) is left
  only the stun strike the ruleset starts by number (the navi's request 0x80000, with the variant the attack
  holds) and the test content's StunBld. Both go with the 0x12 shim (step 5, phase C: a role for the stun
  strike).
- **SlashCross's blade.** The charged slash (action 0x41) raises the blade by the attack's variant
  (`byte_80EBB64`), which an earlier merge lost; lib/sword's `sword.raise` has it again, and goes into
  lib/swords when SlashCross's weapons convert.
- **Asset names** for the family: `sword`, `fire-sword`, `aqua-sword`, `elec-sword`, `sword-slash`,
  `big-slash`, `cross-slash`, `sword-swing`, `big-sword-swing` (compat/assets.toml and curation.toml).
- **Verified** on the test content (the blades' duel under rollback, the engine's tests), the type check, and
  the traces and chip lab on a pack extracted with asset names.

### 5.3 Cannons and projectiles: one kind, per-chip variants

v1: the projectile (attack #0) reads `data.objects.projectiles[me:param(1)]`, a table of 12-byte records by
number; the buster passes a NaviCust-chosen number through `attack_param(1)`.

```luau
-- objects/projectile/projectile.luau exports the kind, `variant` and the firing helpers.
local projectile = {}
projectile.kind = define.kind { id = "projectile", pool = "attack",
    state = { variant = "record:projectile-variant", row = "i8" }, update = update }

export type Variant = {
    id: string?,                 -- only when a setup names it (a NaviCust buster shot)
    collision: CollisionSpec, hit_mod: number, element: Element, secondary: { SecondaryElement }?,
    spark: Spark?, sprite: Sprite?, anim: number?,
    status: Status?, bug: Bug?, hit_panel: PanelHit?, bursts: boolean?, climbs: boolean?,
}
function projectile.variant(v: Variant): Variant return define.record("projectile-variant", v) end
function projectile.fire(navi: Object, v: Variant, damage: number, opts: { z: number, row: number? }): number ... end
```

```luau
-- lib/cannon.luau (group G3 writes it): the cannons' action (the original's 0x14).
function cannon.action(spec: { shot: projectile.Variant, look: CannonLook }): Action ... end

-- chips/cannon/chips.luau: a series (v1: action 0x14, subtypes 0, 1, 2).
local SHOT = projectile.variant { collision = { is = collision.shot, hits = collision.hits_navis },
    hit_mod = 0, element = "null", spark = sparks.cannon }
return {
    define.chip { id = "cannon", name = "Cannon", damage = 40, -- ...
        action = cannon.action { shot = SHOT, look = LOOKS.cannon } },
    define.chip { id = "hicannon", name = "HiCannon", damage = 100, -- ...
        action = cannon.action { shot = SHOT, look = LOOKS.hicannon } },
    define.chip { id = "m-cannon", name = "M-Cannon", damage = 180, -- ...
        action = cannon.action { shot = SHOT, look = LOOKS.m_cannon } },
}
```

(What the cannons' subtype selects in action 0x14's tables is G3's to port; the shape is what matters here:
whatever the subtype indexed becomes an argument.)

The 40 projectile variants v1 extracts become `projectile.variant` records defined where they are used: the
buster's in `navis/megaman/weapons/buster/`, a cannon's in its chip, the ones several families share in
`objects/projectile/variants.luau`. The NaviCust buster shots are variants with ids (`buster-spread`), which the
navi stats name (§7.2) and compat maps from the save's byte. The flying shot (#0xB, 7 variants), the sword
wave (#0x96, 19), the shock wave (#0x16) and the boomerang follow the same pattern with their own variant
records.

### 5.4 Dimming chips: a controller kind, a per-chip hook

v1: a chip with action 0x15 registers its module for `Hook::DimmingChip(subtype)`; AreaGrab's controller
reads `me:param(1)` to tell AreaGrab from PanelGrab.

```luau
-- lib/grab/controller.luau: AreaGrab's and PanelGrab's controller (effect #3, `sub_80E0710`).
local dimming = require("../dimming")
local shot = require("./shot")

export type State = { chip: Chip?, bonus: number, whole_column: boolean }

local controller = define.kind {
    id = "grab/controller",
    pool = "effect",
    state = { chip = "chip", bonus = "u16", whole_column = "bool" },
    update = dimming.phases { effect = effect },  -- begin, dim, telop, effect, undim, finish
}

-- A chip's `dimming` hook: spawns the controller for `user` (`sub_80E07E0`).
local function hook(whole_column: boolean): DimmingHook
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming.spawn_controller(controller, user, spec)
        if o then
            (o.state :: State).whole_column = whole_column
        end
        return o
    end
end

return { area = hook(true), panel = hook(false) }
```

```luau
-- chips/areagrab/chip.luau
local grab = require("../../lib/grab/controller")
return define.chip { id = "areagrab", name = "AreaGrab", flags = { "dimming" }, -- ...
    dimming = grab.area }
```

`lib/dimming/` holds what every controller shares: `dimming.phases { effect = ... }` builds the update that
calls the dimming service's steps in the original's order (`object_timefreezeBegin`, `object_dimScreen`,
`object_drawChipName` for the telop and the wait for a counter cut-in, the effect, `object_undimScreen`,
`object_timefreezeEnd`), and `dimming.spawn_controller` does the spawn every `off_802CCB4` entry does. The
spec's `chip` field is a chip handle now, so the telop draws the chip by name.

**As built** (step 7): content/bn6/lib/dimming.luau (`dimming_chips.phases { name, effect }`,
`dimming_chips.done(me)` when the effect is over, `dimming_chips.spawn(kind, user, spec)`; a file, named so it
doesn't shadow the `dimming` service global), lib/grab/controller.luau (the `grab/controller` kind, state
`{ bonus, whole_column }`, and the hooks `grab.area` and `grab.panel`), lib/grab/shot.luau (the `grab/shot`
kind), and chips/areagrab and chips/panlgrab, which are definitions with their records (nothing names them by
number). What it settled:

- **The controller keeps no chip.** The telop draws the chip the dimming registered (`register_dimming`), so
  the v1 state's `chip` went; `DimmingChipSpec.chip` stays a number until the hooks' specs take handles.
- **The chip's parameter became the hook's argument**: AreaGrab's first parameter (1, a whole column) is
  `grab.area`, PanelGrab's (0) `grab.panel`; the shot's side is its `alliance` (the original also keeps it in
  the shot's first parameter).
- **The test content** puts the definitions in the folders by handle (`scenario::setup_with_handles`, a chip
  in code A, else `*`); its numbered grab chip went.
- **Verified** on the test content (the dimming duel and its rollback), the type check, and the traces and the
  chip lab on a real pack: every AreaGrab and PanelGrab scenario matches, the counter cut-in's too.

**As built** (phase B, group B2c: dimming subtypes 3, 7, 12, 22, 29, 30, 32, 37;
docs/engine/dimming-chip-effects.md §12 to §19): chips/geddon (with the six special records that share its
quake as definitions in their own folders), chips/snake, chips/lifesync, chips/metrknuk, chips/dblbeast,
chips/numbrbl, chips/cornfsta and chips/dblhero. What it settled:

- **A controller with other phases** writes its own update from the `dimming` service's steps: NumbrBl's runs a
  navi chip's (AntiNavi, the navi telop), keeping the chip the spec gives for them.
- **Records kept, hooks by number**: NumbrBl (damage formula 21), CornFsta and DblHero (Program Advances) give
  their hooks from chips/<key>/chip.luau, which chips/08a-numbrbl, 14c-cornfsta and 158-dblhero register by
  number until definitions take formulas and recipes.
- **Stand-ins and shared actor data**: NumberMan and the farmer are lib/dimming/stand_in's; DblHero's heroes and
  DblBeast's beasts are their own (offset spots, a full hide, their own identities). MegaMan's copy shares the
  user's actor data, so its arm is lib/buster's `attach_arm(user, owner, slot, while_dimmed)` (`sub_80EB572`, which
  `raise_arm` now calls). lib/slot gains `held_2` for an owner's second attachment.
- **A family's kind reused**: CornFsta's bursts are CornSht's corns (chips/cornsht/corn, generation 0xFF), and
  its farmer holds CornSht's gun (`cornsht.gun`).
- **Still numbers**: the beasts', heroes' and farmer's NameIDs (`me.name_id`, six uses the ratchet counts), whose
  attachments sit at their sprites' attach points; the shots' `hit_effect = 0xFF`. LifeSync's immune virus is
  told by its actor data (AI 13). `battle.boss_rank` (battle effect 1) joins `battle.link` for LifeSync.

### 5.5 Navi chips: a navi kind, per-chip parameters

v1: action 0x1B registers `Hook::NaviChip(subtype)`; EraseMan reads `me:param(1)` (the aim's switching time)
and the controller passes the chip's parameters through.

```luau
-- chips/eraseman/navi.luau: EraseMan (actor #0x15, `sub_80BB608`).
export type Summon = { aim_ticks: number }
export type State = { cycle: number, aim: number, controller: Object?, aim_ticks: number }

local navi = define.kind {
    id = "eraseman/navi", pool = "actor",
    state = { cycle = "u8", aim = "u8", controller = "object", aim_ticks = "u8" },
    update = update,     -- as today, with `s.aim_ticks` for `me:param(1)`
}

-- A chip's `navi` hook (`sub_80BB7F6`).
function eraseman.summon(args: Summon): NaviChipHook
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = battle.spawn(navi, Vec3.zero)
        if not o then return nil end
        -- ... placement as today ...
        local s = o.state :: State
        s.controller, s.aim_ticks = controller, args.aim_ticks
        return o
    end
end
```

```luau
-- chips/eraseman/chips.luau: the series (v1: action 0x1B, subtype 5, Param1 20, 16, 12).
local sp = require("../../lib/navi-chips/sp")
return {
    define.chip { id = "eraseman", name = "EraseMan", damage = 120, hit_param = 138, -- ...
        navi = eraseman.summon { aim_ticks = 20 } },
    define.chip { id = "erasemn-ex", name = "EraseMn[EX]", damage = 140, hit_param = 138, -- ...
        navi = eraseman.summon { aim_ticks = 16 } },
    define.chip { id = "erasemn-sp", name = "EraseMn[SP]", hit_param = 138, -- ...
        damage = formula.sp_navi { slot = sp.eraseman, by_time = { --[[ eleven steps ]] } },
        navi = eraseman.summon { aim_ticks = 12 } },
}
```

The generator writes each chip's parameter bytes under the names the family's builder gives them (§9.3). The
navi chip controller (effect #0x10, `sub_80E1880`) stays the ruleset's; it calls the
chip's `navi` hook, and the navi calls `navi_chip.navi_left(controller)` as today. `lib/navi-chips/` holds what
the navis share (appearing, leaving, the SP deletion-time damage).

**As built** (step 7): chips/eraseman/navi.luau (the `eraseman/navi` kind, state `{ cycle, aim, controller,
aim_ticks }`, and `eraseman.summon { aim_ticks }`, the `navi` hook), chips/eraseman/mark.luau (`eraseman/mark`)
and chips/eraseman/beam.luau (`eraseman/beam`, spawned with `{ aim, ticks, navis }`; EraseMan's own EDeletBm,
chips/193-edeletbm, spawns it too). The beam's collision type (`piercing-break`, row 0x16) and spark
(`sparks.erase`, hit effect 0x0C) are definitions. What it settled:

- **The chips stay records.** The ruleset turns navi chips back by number (AntiNavi, `is_navi_chip`: 0xDD to
  0x118, which isn't the `navi` flag: 0x116 to 0x118 lack it), and EraseMn[SP]'s damage is a formula. Their
  records' navi chip subtype 5 runs chips/0ec-eraseman/chip.luau, which summons EraseMan with the record's first
  parameter as `aim_ticks`; it goes when the chips are definitions (a navi-chip trait for AntiNavi, step 10,
  and damage formulas in definitions) and each is `navi = eraseman.summon { aim_ticks = n }`. No
  `lib/navi-chips` yet: EraseMan shares nothing with another navi so far (A2 starts it).
- **Parameters became state**: the mark's time, the beam's aim, time and owner kind, EraseMan's switching time.
- **Verified** on the test content (the navi chip duel and its rollback), the type check, and the traces and the
  chip lab on a real pack: every EraseMan scenario matches.

### 5.6 Instant chips: a hook per chip

v1: action 0x1C calls `Hook::InstantChip(subtype)`; one module serves the 30 chips of subtype 3.

```luau
-- chips/busterup/chip.luau
local SPARKLE = define.effect { sprite = asset.sprite("buster-up"), anim = 0 }
local SOUND = asset.sound("buster-up")
return define.chip { id = "busterup", name = "BusterUp", -- ...
    instant = function(user: Object, spec: InstantChipSpec)   -- `sub_8010820`
        local stats = battle.navi(user.alliance)
        stats.attack = math.min(stats.attack + 1, 9)   -- 1: BusterUp's parameter, as data
        battle.effect(user:attach_point_pos(0x20), SPARKLE)
        battle.play_sound(SOUND)
    end,
}
```

The plus chips (subtype 3, `sub_8010488`) compose `lib/instant/plus.luau`: `instant = plus.attack(10)` for
Atk+10, `plus.navi(20)` for Navi+20; their custom-screen behaviour is the chip's `modifier`, as today. A weapon
that names an instant effect no chip has (TenguCross's wind, subtype 0x14) calls the effect from its `setup`
like any other function; there is no instant-effect table any more.

**As built** (step 7): chips/busterup (BusterUp's effect is the chip's own: `raise(1)`, the buster's level by
one; its sparkle a `define.effect` on the newly named `buster-up` sprite), lib/instant/plus.luau
(`plus.attack`, `plus.navi`, `plus.sparkle`, and `plus.by_record`), chips/atk-10 and chips/navi-20. What it
settled:

- **The parameter that told the plus chips apart is the hook**: the records' first parameter (0 the side's
  attack bonus, 1 its navi bonus) is `plus.attack` or `plus.navi`; the extra in the third and fourth (the
  special chips') is 0 for both.
- **Atk+30 stays a record** (SunMoon's recipe names it by number), as do MegaBstr (the pack's chip 0, which a
  zeroed chip field reads), WhiCapsl, Uninstll, DarkPlus and the special chips of the effect: their records'
  subtype 3 runs chips/0c0-atk-10/chip.luau, which calls `plus.by_record` with the record's parameters.
- **A menu flag got its name**: extra flag 0x40, set on exactly the five modifier chips, is
  `ExtraChipFlags::MODIFIER` (`"modifier"`), so a definition can give it; the battle reads `modifier`.
- **Verified** on the test content (the instant chips' duel and its rollback, the plus chips' bonuses from a
  special source, BusterUp's cap), the type check, and the traces and the chip lab on a real pack.

### 5.7 Weapons and forms

```luau
-- navis/megaman/weapons/buster/weapon.luau
local SHOT = define.action { id = "megaman/buster/shot", state = STATE, update = shot }  -- action 0x11 in compat
return define.weapon {
    id = "megaman/buster", name = "Buster",
    charge_ticks = { 0, 0, 0, 0, 0 },
    setup = function(navi: Object): Action                          -- `sub_8011A26`
        local pick = pick(navi)                                     -- the NaviCust blanks and charged slots
        if pick == "blank" then return blank_shot.setup(navi) end
        if pick == "charged" then return charged_shot.setup(navi) end
        navi.attack_damage = navi:buster_damage()
        -- ... as today; the NaviCust shot is `battle.navi(side).buster_shot`, a projectile variant or nil ...
        return SHOT
    end,
}
```

```luau
-- navis/megaman/forms/heatcross/form.luau
local burner = require("../../../../lib/burner")
local buster = require("../../weapons/buster/weapon")
local body = require("../../overlays")          -- the Crosses' body overlays (records)

-- FireBrn's action with this form's flame (built while loading: definers don't run in a battle).
local FLAME = burner.action { id = "heatcross/charge/flame", flame = burner.flames.heat_charge }

local CHARGE = define.weapon {                  -- the original's routine 0x06, `sub_8011BA2`
    id = "heatcross/charge", name = "Heat charge",
    charge_ticks = { 100, 90, 80, 70, 60 },
    setup = function(navi: Object): Action
        -- ... as today: the attack's damage, element and bonus ...
        return FLAME
    end,
}

return define.form {
    id = "heatcross", name = "HeatCross",
    kind = "cross", cross_of = require("../../../heatman/navi"),
    sprite = asset.sprite("megaman"), element = "fire",
    buster_bonus = 1,
    weapons = { buster = buster, charge_shot = CHARGE },
    overlay = body.heatcross,
    chip_bonus = { family = "fire", damage = 50 },
    mugshot = asset.mugshot("heatcross"),
    identity = { actor_type = "player", traits = { "cross" }, attach_points = { {2, 0}, {2, 0}, --[[ ... ]] } },
}
```

The per-form tables the Rust ruleset holds as `match form.0` today (§7.5) become these fields.

**As built** (step 7): navis/00-megaman/weapons/buster, charged-shot and blank-shot (`weapon.luau` each: the
weapon definition `megaman/buster` with its shot `megaman/buster/shot`, `megaman/charged-shot` with
`megaman/charged-shot/action`, `megaman/blank-shot` with `megaman/blank-shot/action`), and
navis/00-megaman/forms/heatcross/charge.luau (`heatcross/charge`). The v1 registrations (`weapon.toml`, the 16
buster alias folders) went. What it settled:

- **A weapon definition takes routine numbers** with the transitional `legacy = { routines = { ... } }`
  marker: the pack's forms, the navis' rows and the ruleset still name weapons by number, and
  `weapon_numbered(n)` finds the definition by any of its routines (the buster's are the 17 numbers whose
  `off_80117D4` entries are `sub_8011A26`); `weapon_number(h)`, which the ruleset's numeric logic asks, is the
  first. Counted by the ratchet; it goes when the forms (step 5's form reader) and the ruleset (phase C) name
  weapons by handle.
- **Charge times are the definition's own**: the charge table's row, plus the next row's first entry for
  Charge 5, which the original's table reads on into (written out and commented in each definition).
- **A role for the forced charged shot**: idle.rs's request 0x20 starts `roles.actions.forced_charged_shot`
  (rules/roles.luau, BN6's first roles file: the charged shot's action), not action 0x16 by number. The trap
  counters' roles stay unfilled until those chips convert. DustCross's throws (weapons 0x2B, 0x2C) return the
  buster's shot definition.
- **The muzzle flash and the arm a throw leaves are looks** in lib/buster (`buster.flash`, `buster.arm`), on the
  newly named `muzzle-flash` and `buster-arm` sprites; the shot's sounds are assets.
- **HeatCross is partial.** Its form stays the pack's record until step 5's form reader; its charged shot is a
  weapon definition (routine 0x06) whose setup still returns FireBrn's action by number, 0x27 (FireBrn is v1),
  which the ratchet counts (a weapon definition's `return ACTION`) until FireBrn converts.
- **What the attack keeps stays numeric for now**: the shot's kind in the attack's first parameter (the
  projectile family's, §5.3), the throw as the shot's variant 2, the arm `raise_arm` raises by attachment row.
- **SlashCross's charged slash** (weapons 0x11 and 0x12, action 0x41) is left to family 8e. It reads the
  charged chip's subtype and first parameter (the four Sword-family definitions' `legacy` marker gives them).
  Its proper hook needs a runtime read of the attack chip's definition (so the charge can ask the chip for its
  slash: StepSwrd's dash, CrosSwrd's two waves, DblDream's two slashes, the blade and the sound) and the sword
  wave's variants as records (it spawns the wave with the chip's subtype as its first parameter).
- **Verified** on the test content (the shots' timelines, the throw, the aliases, the role, the definitions'
  routines and charge times), the type check, and the traces and the chip lab on a real pack.

## 6. Compat: the original's numbers

### 6.1 What it holds

content/bn6/compat/ is a folder of TOML files, written by the generator and edited by hand when content is
renamed or added. It maps keys to the original's numbers, and holds the comparison hints that only the traces
need.

| File | Holds |
|---|---|
| chips.toml | `minibomb = { id = 0x036, action = 0x12, subtype = 0 }` for every chip; action and subtype are documentation (the traces never compare them) |
| actions.toml | action key to navi action number: `"minibomb/action" = 0x12` (every chip whose use is an action has `<chip>/action`), `"megaman/buster/shot" = 0x11`, a weapon's `"<weapon>/action"`, `"engine/move" = 0x10`, `"engine/form-change" = 0x1C`. A role action whose number a chip's or weapon's action has is that action (the volley is WideSht's 0x30); only the turn (0x3B) has its own, `"megaman/turn"` |
| navis.toml, forms.toml | `eraseman = { navi = 0x04, name_id = 0x1A4 }`, `heatcross = { form = 0x01, name_id = 0x1AC }`; the base form has no `name_id` (it is MegaMan's) |
| weapons.toml | `"megaman/buster" = [0x00, 0x2E, 0x2F, 0x3E, 0x3F, 0x4D, ...]`, one line per weapon: the numbers whose `off_80117D4` entries are one routine. `nullsub_44`'s numbers are split by what the ruleset does with them (`megaman/rock-barrage`, `megaman/charged-chip-bonus`, `megaman/stale-register`). Every number a form's row (`byte_8020354`), a navi's (`byte_80210DD`) or a known NaviStats (NaviCust programs) names |
| kinds.toml | `bomb = { pool = "attack", index = 0x08 }`, keyed by the v2 keys (§4.2); `scratch_position`, `scratch_z_fraction`, `scratch_position_without_sprite` (the charge glow's condition) and `actor_list_entry` (8 for `rock`); the engine's kinds as `"engine/..."` |
| stages.toml | `"netbattle-1" = { settings = [0x00], actor_list = 0x080B1989 }`: the settings indices that are the stage, and the address its actor list goes by. No two of the 192 records are identical (96 layout and actor-list pairs, each with two effect words), so there are 192 stages |
| records.toml | the few records a setup names by byte, key to byte: the save's SP deletion-time slots (`[sp_slots] "sp/eraseman" = 3`); NaviCust buster shots when their producers are known |
| assets.toml | asset names to ROM numbers: `[sprites] bomb = "0c-02"`, `[sounds] throw = 0x1A6`, `[backgrounds]`, `[banners]`, `[mugshots]`; every asset the ROM has, the unnamed under placeholders (§6.3); chip icons follow chips.toml |
| text.toml | the text encoding the generator and the extractor share: `glyphs`, what each byte below `first_control` (0xE0) draws, as UTF-8 (the EX and SP glyphs as `[EX]`, `[SP]`) |
| curation.toml | the names the generator made up, by file and key, with where each came from: the review list (§13) |

A sample (kinds.toml):

```toml
# Object kinds: the original's pool and index (the traces compare them), and the positions the
# comparison skips (register garbage the spawner leaves until the kind's init places it).

bomb = { pool = "attack", index = 0x08 }
"grab/controller" = { pool = "effect", index = 0x03, scratch_position = true }
"eraseman/navi" = { pool = "actor", index = 0x15 }
"dustcross/junk-ball" = { pool = "attack", index = 0xB0, scratch_z_fraction = true }
"engine/hitbox" = { pool = "attack", index = 0x03 }
"engine/palette-flash" = { pool = "effect", index = 0x0A, scratch_position = true }
```

Every file maps a key to its numbers, and many-to-one maps are allowed (aliases, deduplicated stages). Every entry
must name a key that exists; every chip, navi, form, weapon, kind and action of the BN6 content must have an entry
(the checker enforces both, §7.7). A chip record with no name (or `????`) that nothing reaches has none: the blank
library slots 0xCB..0xDC and 0x160..0x170 and the nameless copies of the plus chips' record.

### 6.2 Who reads it

Compat is read by `bn6-compat`, a new crate that depends on `bn6-battle` (so the engine can't depend on it), and
by the tools that interoperate with the real game:

- **the golden-trace harness** (§10): `trace.rs` moves from `bn6-battle` into `bn6-compat` behind its `trace`
  feature;
- **the setup codecs**: `NaviStats`, `BattleFolder`, `ChipHand`, `TransformRequest`, battle settings and SP times
  from the game's bytes and back, for traces, real saves, folders, NaviCust setups and link navis;
- **the extractor**, to name the assets it writes (§6.3, §9.1);
- **the sound comparison** (verification workspace), to map cues to song numbers;
- future netplay interop with the real game.

### 6.3 Assets and their names

Content refers to an asset by name, resolved while loading: `asset.sprite("bomb")`, `asset.sound("bomb-hit")`,
`asset.banner("program-advance")`, `asset.background("netbattle-blue")`, `asset.mugshot("heatcross")`. An unknown
name is a load error naming the module. The resolved value is a handle into the asset registry; state holds it
(`sprite:load(BOMB)`), and a cue carries it (`battle.play_sound(SOUND)`).

- **Names** come from compat/assets.toml, which the generator writes: the disassembly's song and sound enum names
  where they exist (`SONG_VIRUS_BATTLE` is `virus-battle`, `SOUND_HIT_BOMB_1` is `hit-bomb-1`), else a name from
  the asset's first user (`erase-mark`), else a numbered placeholder (`sprite-0c-01`, `sound-101`, `banner-54`). The
  table lists every asset the ROM has, so a placeholder is an entry too. Content may not use a placeholder (the
  checker warns); naming one is part of using it.
- **The extractor** reads compat/assets.toml and writes `graphics/sprites/<name>/`, `sound/songs/<name>.mid`,
  `graphics/hud/chip-icons/<chip key>.png` and so on, and the asset root's name index, `assets.toml` (every name
  the table gives, and the pack's other assets under their placeholders, each with the engine's identity for it),
  from which the loader fills `Content::assets` (done with step 6's first part). An asset the table doesn't list (one a newer table left out)
  is written under its placeholder, so nothing the ROM has is lost.
- **The checker** validates asset names without a ROM: compat/assets.toml is the list of names the BN6 content can
  use. A modded pack without compat lists its own assets' folders.
- **Animation numbers stay numbers.** An animation is an index into its sprite's own list, observable in the traces
  (`anim`), with no identity outside its sprite; modules name them as constants (`local ANIM_THROW = 6`), as
  frame numbers in any sprite format are. Sound-bank internals (voicegroup and sample numbers inside the bank) are
  likewise the bank's own.
- **Text.** Chip names and descriptions are UTF-8 in the definitions. The generator decodes the ROM's text with
  compat/text.toml; the extractor writes the font with the same table as `hud/font.json`, and the HUD draws names
  from the chip definitions. Glyphs with no character of their own (the EX and SP marks) are multi-character
  entries of the table, written as v1's extractor already writes them (`EraseMn[EX]`).

### 6.4 How nothing else can read it

- **Luau can't.** Compat is TOML. The loader discovers modules by `.luau`, `require` resolves modules only, and the
  define phase never exposes compat. No Luau API returns an original number (the numeric API is removed, §7.6).
- **The engine can't.** `bn6-compat` depends on `bn6-battle`; a dependency the other way is a cycle. `Content`
  has no compat fields at any step: the validator maps the engine's handles to the original's numbers, and no
  bridge carries them into the engine (§7.3, the user's decision). A test in `bn6-compat` asserts `bn6-battle`'s
  dependency list doesn't contain it, and a source guard in `bn6-battle`'s tests fails on the word `compat`
  outside comments in the engine and the crates it runs content through.
- **The checker enforces the rest** (§7.7): no deprecated numeric API use, no `legacy { }` markers, no placeholder
  asset names, once the ratchet reaches zero.

## 7. The Rust side

### 7.1 `Content`

```rust
pub struct Content {
    pub names: Names,                          // keys by handle, per registry, and the reverse maps
    pub chips: Vec<ChipData>,                  // by ChipHandle
    pub navis: Vec<NaviData>,                  // by NaviHandle
    pub forms: Vec<FormData>,                  // by FormHandle
    pub weapons: Vec<WeaponData>,              // by WeaponHandle
    pub kinds: Vec<KindData>,                  // by KindHandle: pool, schema, Engine(EngineKind) | Script
    pub actions: Vec<ActionData>,              // by ActionHandle: schema, traits, Engine(EngineAction) | Script
    pub stages: Vec<StageData>,
    pub identities: Vec<Identity>,
    pub effects: Vec<EffectSprite>,  pub sparks: Vec<EffectSprite>,
    pub regions: Vec<Region>,        pub collision: Vec<CollisionType>,
    pub statuses: Vec<StatusEffect>, pub lockon: Vec<LockonMode>,
    pub records: Vec<RecordInfo>,              // type name only
    pub schemas: Vec<Schema>,                  // content state layouts (StateId = schema handle)
    pub program_advances: Vec<ProgramAdvance>, // in the order they are tried
    pub rules: Rules,                          // the rule sections
    pub roles: Roles,                          // §7.4
    pub assets: AssetNames,                    // sprites (with animation timing), sounds, banners, backgrounds, mugshots
    pub scripts: Scripts,                      // module sources
    pub definitions: DefinitionTree,           // the canonical tree the define phase produced (hashed)
}
```

- `ChipData` loses `id`, `action`, `subtype`, `params`, `script` and the per-family tables (`gun_del_sol`,
  `sword`, `recovery`, `navi_damage`): those were the numbers and the builder arguments. It gains `name`,
  `description`, `usage: ChipUsage` (`Action(ActionHandle)`, `Dimming`, `Navi`, `Instant`: the hooks are function
  slots), `damage: ChipDamage` (`Fixed(u16)` or a typed formula), `traits: ChipTraits`, `icon: AssetHandle`.
- `NaviData`/`FormData` hold handles (`own_chip: Option<CodedChip>` with a `ChipHandle`, `weapons: FormWeapons` of
  `Option<WeaponHandle>`) and the traits of §7.5; `name_record` becomes `identity: IdentityHandle`.
- `ObjectData`'s tables (attachments, rocks, overlays, sun beam looks, boomerangs, projectiles, flying shots, sword
  waves, shock waves, name looks) disappear into records and the kinds that own them. `Content::object_kind(name)`
  and `object_kind_at(pool, index)` go; `Content::kind(key)` resolves a key.
- `Rules::weapons`, `cross_palettes`, `actor_records`, `sp_deletion_times`, `field_regions`, `collision_types`,
  `status_effects` move to their owners (§3.8).
- The lookups take handles: `content.chip(h)`, `content.form(h)`; `content.chip_by_key("minibomb")` for tools
  and tests.

### 7.2 Engine state and setup

| State (v1) | v2 |
|---|---|
| `Object.index: u8` (with the slot's pool) | `Object.kind: KindHandle` |
| `Object.params: [u8; 4]` | gone: engine kinds' arguments are typed fields of their `Vars`; content kinds' are state |
| `Object.name_id: u16` | `Object.identity: Option<IdentityHandle>` |
| `Sprite.id: SpriteId { category, index }` | `Sprite.id: SpriteHandle` |
| `ChipHand.ids`, `selection`: chip ids, `code << 9 \| id` | `[Option<ChipHandle>; 6]`, `[Option<(ChipHandle, ChipCode)>; 6]` |
| `BattleFolder`: `code << 9 \| id` | `(ChipHandle, ChipCode)` |
| `NaviStats.navi`, `form`, `starting_form` | `NaviHandle`, `FormHandle` |
| `NaviStats.weapons` (routine numbers), `ActorData` weapon bytes | `Option<WeaponHandle>` |
| `NaviStats.buster_shot` (a projectile kind number) | `Option<RecordHandle>` (a projectile variant) |
| the attack header's `chip`, `variant`, `params` | `chip: Option<ChipHandle>`; variant and params gone |
| the navi's action (CurAction byte, 0..8 and 0x10 up) | `NaviAction`: the framework's states (entry, deletion, flinch, ..., idle) and `Engine(EngineAction)` / `Content(ActionHandle)` |
| `ContentState` with a `StateId` per registered module | `StateId` = schema handle; reference field types hold handles |
| `LinkedChip.chip`, `dimming` records' chip | `ChipHandle` |
| `BattleSettings { layout, music, background, actors: ActorListId, ... }` | `{ stage: StageHandle, background: AssetHandle, effects, ... }` |
| `SpTimes` by SP navi number | by SP slot record handle (`sp.eraseman`) |
| `navi_chips_used` bit per chip id past 0x18F | bit per navi handle |
| `SoundCue::Effect(SoundId)` | `SoundCue::Effect(SoundHandle)` |

Setups can be written by name (a `SetupFile` with chip keys and codes, navi and form keys, a stage key) and are
resolved against `Content::names`; netplay peers exchange handles once their content hashes agree.

### 7.3 The define phase and the loader

`bn6-luau` gets `define::run(pack, assets) -> Result<Defined, ContentError>`:

1. A fresh sandboxed VM (`sandbox::new_vm`), with `define`, `asset` and `require` installed before
   `lua.sandbox(true)`.
2. Every `.luau` module of the pack, in path order, loaded as `require` loads it today (compile, `verify::check`,
   run once, deep freeze). Order doesn't matter: a module loads once, when first required or reached.
3. Each definer records `(registry, defining module, ordinal, spec table)` and returns the spec with its
   registry's metatable. Definers and `asset` fail once loading ends.
4. Keys (§2.2): explicit ids; derived keys for anonymous definitions nested in a keyed definition of the same
   module, found by walking the keyed definitions in sorted key order and their fields in sorted name order;
   `module#n` for the rest.
5. Handles (§2.3), then `id` and `handle` written into each spec, then every spec deep-frozen.
6. The canonical tree: each definition serialized with definition references as `(registry, key)`, functions
   as slots `(registry, key, field path)`, integers checked exact, keys sorted. The tree and its hash are the
   `DefinitionTree`.

`bn6-battle`'s `content::define` turns the tree into typed `Content`: serde deserializes each registry's entries
from the tree (the data types already derive `Deserialize`), with references resolved to typed handles and a
reference to the wrong registry reported with both keys. The asset side (names, animation timing) comes from the
pack's assets or, for tests, a synthetic index (§7.8).

The runtime runs the same define phase on `content.scripts`, checks that it reads exactly the content's
definitions (every VM made from the same content behaves the same, as scripting.md §3.1 already requires), and
binds the functions the engine planned (`BindPlan`): each by definition slot (`FnSource::Slot { registry, key,
path }`) or, while registration by module lasts, by module export (`FnSource::Export`). The engine keeps the
function ids in its registries (a kind's `update`, an action's `update`, a weapon's `setup`, a chip's
`dimming`/`navi`/`instant`), so dispatch is an array index.

**The runtime is not part of a battle.** `Battle` holds no runtime handle: a battle is plain data and `Send` by
construction, and a snapshot is a clone (the `unsafe impl Send` on snapshots and its guard are gone). Each thread
keeps runtimes in a small cache keyed by the content hash, which the battle's setup already carries
(`RoundSetup::content`), and a content call on a battle uses its thread's runtime for that hash; a battle moved to
another thread steps there with that thread's runtime, identically. Code that wants a particular runtime (one
loaded with native code, a fresh VM halfway through a battle, edited scripts in tests) runs under
`behavior::with_runtime(&runtime, ...)`, or steps with `Battle::step_with(&runtime, input)`. `ContentHost`
becomes:

```rust
pub trait ContentHost {
    fn update_object(&self, api: &mut dyn CoreApi, f: FnId, me: ObjectRef) -> Result<(), ContentError>;
    fn update_action(&self, api: &mut dyn CoreApi, f: FnId, me: ObjectRef, state: StateId) -> Result<(), ContentError>;
    fn call_hook(&self, api: &mut dyn CoreApi, f: FnId, call: HookCall) -> Result<Value, ContentError>;
}
```

`Registrations`, `KindReg { pool, index }` and `Content::registrations` are deleted (step 3); `Hook` by number
stays as registration by number's lookup (`Defs::hook`) until step 13. While the migration runs, a registry also holds the
engine's own entries (its kinds, keyed `engine/player`, `engine/hitbox`, ...) and the entries registration by
number makes, keyed from that data: a kind by its folder name, an action `v1/action-12`, a weapon `v1/weapon-02`.
Registration by number reaches entries through lookups on the registries (a kind's object slot, an action's
number, a weapon's routine numbers), filled from that registration's own data (`object.toml`, `chip.toml`,
`weapon.toml`) and nothing else.

**The engine never reads compat (user decision, 2026-09-30).** "the validator maps the ids and the engine itself
doesn't know about them, so the engine can be clean of validation code". There is no bridge from compat into the
engine, at any step: definitions are never given the original's numbers inside it. The engine runs on its own
identities (handles), and the validator, `bn6-compat` (the golden-trace comparison and the setup codecs), maps
them to and from the original's numbers with compat. Where the engine's state still holds a number, what content
defines needs a handle there instead, which is §7.2's table brought forward:

- **Objects** (step 3, done): `Object.kind: KindHandle` replaces `Object.index`. A kind registered by number
  keeps its slot in its definition (`KindDef::slot`), which the numeric spawns and the validator use; a defined
  kind has none and spawns by handle. `Compat::object_slot` gives the comparison an object's pool and index: the
  definition's slot, else compat's by key.
- **The navi's action** (step 3; step 9, done): the navi runs a `NaviAction` (its actor data's `navi_action`,
  not the object's action byte): the framework's states, the ruleset's `EngineAction`s, a content action by
  handle, or `Unported(n)`. `Compat::navi_action` gives the comparison the original's number: a state as itself,
  an engine action by its key (`engine/move`...), a content action by its v1 registration's number or its key.
  `CONTENT_ACTION` (0xFF) is left only as the numeric API's number for an action content defines.
- **Chips, navis, forms, weapons and stages** (step 3b, done): hands (`ChipHand.ids`, the selection's
  `FolderChip`s), folders, the linked chips, the attack's chip, dimming controllers' chips, `NaviStats` (navi,
  form, starting form, weapons), the actor's weapon slots, transformation requests and battle settings hold
  handles. `BattleSettings` is `{ stage: StageHandle, background, effects }`, the stage's record
  (`StageSettings`, the 16-byte BattleSettingsList1 entry) read through the content; a set's later rounds name
  their stages by handle. The pack's records are `v1/chip-036`, `v1/navi-01`, `v1/form-0c`, `v1/stage-11` and
  `v1/weapon-29` (every routine number, implemented or not). The ruleset's numeric logic is unchanged: it asks
  `Content::chip_number`, `navi_number`, `form_number`, `weapon_number` (none for what content defines, so a
  defined chip gets none of the ruleset's by-number cases until phase C gives them traits and roles). What the
  original zeroes and the engine now holds as none reads as the pack's chip 0 where the game reads it
  (`Content::chip_field`: the attack's cleared chip, a non-player's carried chip, the side's special chip, a
  never-built hand's selection). `bn6_compat::codec::Ids` maps the original's numbers to handles through
  compat's keys: a definition with the key compat gives a number is the thing (so a v2-defined chip or weapon is
  reached from a recorded folder or navi stats), else the pack's record by number; back, a record gives its own
  number and a definition compat's (a weapon's first routine number). A trace's 16-byte battle settings name the
  first stage whose record matches them. The numeric API (core_api) keeps numbers: a chip content defines reads
  as `0x200 +` its handle (`core_api::DEFINED_CHIPS`), which it takes back; a defined weapon has no routine
  number for it and panics there. `Defs::number_chip` and `number_weapon` are gone.

The trace comparison's hints (`scratch_position` and the rest) are compat's alone: kinds.toml has them, and the
engine's kinds carry none. A source guard (`crates/bn6-battle/tests/no_compat.rs`) fails on the word `compat`
outside comments in `bn6-battle`, `bn6-content-api` and `bn6-luau`.

**The spike.** A throwaway crate (not committed) ran a define phase on bn6-luau's real sandbox with five modules
(a bombs library with a shared state table, a bomb kind with a module-level effect, MiniBomb and BigBomb
modules, and a FlshBom series module with one shared action for three chips): the verifier accepted the
modules; two VMs, loading the modules in opposite orders, produced identical canonical trees and handles; derived
keys came out as `minibomb/action`, `flshbom1/action`, `lib/bombs/bomb#1`; MiniBomb's action update, fetched from
the second VM, ran with its captured arguments; the three throw actions shared one schema; `define` calls after
loading failed; and a module that wrote a module-level local was still refused. Two refinements came from it and
are in this design: builders put their parameters in `args` (a captured-only parameter table gets a `module#n`
key), and derived keys are claimed only within the defining module (a library's shared state table otherwise took
the first chip's key).

### 7.4 Roles

The ruleset declares what it needs from content as a typed struct; content fills it once, in `rules/roles.luau`.
A role left empty is a load error naming it.

```luau
-- rules/roles.luau
return define.roles {
    kinds = {
        absorbed_obstacle = require("../objects/absorbed-obstacle/kind"),
        falling_rock = require("../objects/falling-rock/kind"),
    },
    actions = {
        stun_strike = ...,  cross_protect = ...,  cross_death = ...,  volley = ...,
        turn = ...,         forced_charged_shot = ...,
    },
    chips = { beast_out = ..., invalid = ... },
    sounds = { hit = asset.sound("hit"), hit_remote = asset.sound("hit-remote"), deleted = asset.sound("deleted"),
               buster_charge = ..., buster_charged = ..., panel_crack = ..., heal = ..., bonus = ..., telop = ... },
    music = { link_battle = asset.sound("virus-battle"), winner = ..., winner_special = ..., loser = ..., none = ... },
    banners = { turn_start = ..., custom = ..., program_advance = ..., empty_recipe = ..., telop_local = ..., telop_remote = ... },
    effects = { deletion = ..., weakness_mark = ..., cross_change = ... },
    sparks = { guard = ... },
    collision = { player = ..., eruption = ... },
    regions = { single = require("../lib/regions").single },
}
```

`Roles` holds handles; the ruleset reads `b.content.roles.sounds.hit`. Roles are for "the ruleset needs *the*
X". Where the ruleset asked "is this chip (or form, navi, weapon, action) one of these numbers", the answer is a
trait on the definition instead (§7.5).

**As built** (steps 4 and 9): content::roles has typed roles, `ActionRole` (the trap counters, the forced charged
shot, the stun strike, the Cross protect, the turn, the Cross death, the volley, the charged sword, the beast
claw, DustCross Beast's scatter), `KindRole` (the rock an actor list places, the absorbed obstacle, the falling
rock, the supports' controller) and `HookRole` (the FirstBarrier). A role names a definition, or, while its
target is still a v1 registration, that registration through the transitional legacy marker (`{ legacy = {
action = 0x49 } }`, `{ legacy = { kind = "rock" } }`; counted by the ratchet); a legacy action number nothing
implements leaves the role `Unported`, and starting it fails as the number did. The charged sword, the beast claw
and the scatter are roles rather than §7.5's traits: the ruleset recognizes exactly one action each, and a v1
registration can't carry traits. The engine's test content has its own roles (testdata/content), which the test
pack replaces.

### 7.5 The ruleset's numeric call sites and their replacements

Counted in `crates/bn6-battle/src` without tests; file names are the modules that hold them.

**Kinds by pool and index.**
- `kinds::update` and `Vars::for_kind` match `(Pool, index)` for the 20 engine kinds; 14 `INDEX` constants
  (navi_warp, eruption, idle_overlay, afterimage, cross_merge, palette_flash, lockon_marker, body_overlay,
  beast_over_burst, full_synchro_aura, bubble_visual, navi_chip, status_visual, form_overlay); 25
  `objects.spawn(Pool::_, n, ...)` sites; `status.rs` spawns effect #0x6B.
  → The engine kinds are registered by key (`engine/hitbox`, ...) with an `EngineKind` enum; dispatch is
  `match content.kinds[h].implementation`; spawns take a `KindHandle` the ruleset keeps in a `EngineKinds` table
  built at load. `(Pool::Effect, 0x6B)` becomes the `effects.weakness_mark` role.
- `behavior::spawn_kind(b, "absorbed-obstacle")`, `"falling-rock"`; the actor lists' `Hook::ActorListEntry(8)`.
  → `roles.kinds.*`; stage actor entries name the kind, whose `place` slot the spawn loop calls.
- `trace.rs`'s `pos_is_garbage` and `z_fraction_is_garbage` by type number and index. → compat kinds (§10).
- The frontend's `index == 0x0A` (palette flash) and `0x57` (form overlay). → `EngineKind::PaletteFlash`,
  `EngineKind::FormOverlay`.

**Navi actions by number.**
- `actions::dispatch` and `status::dispatch` (`action >= 0x10`, `movement::ACTION` 0x10, `dimming_chip::ACTION`
  0x15, `navi_chip::ACTION` 0x1B, `instant::ACTION` 0x1C, the link navis' actions past 8).
  → `NaviAction` (§7.2); link navis' actions are their navi's `actions`.
- `set_attack(b, r, 0x49 | 0x4D | 0x16 | 0x3B | 0x4C | 0x30 | 0x1C, ...)` in idle.rs and status.rs.
  → `roles.actions.{stun_strike, cross_protect, forced_charged_shot, turn, cross_death, volley}` and
  `EngineAction::FormChange`.
- `beast_rush.rs` `CHARGED_SWORD` 0x41, `BEAST_CLAW` 0x52; `chip_use.rs` `action == BEAST_CLAW || (action ==
  0x41 && form.0 == 0x0F)`. → action traits `charged_sword`, `beast_claw`.
- The afterimage's tether "action below 0x10". → `NaviAction::is_attack()`.

**Chips by id.**
- `hand.rs` `NO_CHIP` 0xFFFF. → `Option<ChipHandle>` (step 3b).
- `chip_use.rs`: chain exclusions 0x52/0x53, dark chips 0x11E..0x122 (their effects on the side's state), the
  aura chips 0x150 and 0x5F..0x61, the substitutes list `[0x47, 0x1E, 0x9A, 0xB1, 0xC0]` indexed by a dark
  chip's `dark_substitute`, the SlashCross charged swords 0x4C..0x4F (in input.rs). → chip traits `no_chain`,
  `dark(DarkEffect)`, `aura`; a dark chip's `dark_substitute` names its substitute chip directly (the list goes);
  SlashCross's `charged_chips.also`.
- `berserk.rs` `CROSS_SPECIAL_CHIPS` and `CROSS_SPECIAL_TOP_CHIPS` (the Cross special's chips by the navi's HP),
  and the Program Advance recipes' ingredients (`PaRecipe`, resolved through `chip_numbered`). → the rules'
  Cross special list and the recipes name chips (step 10).
- `intake.rs`: AntiDmg 0xBB, AntiSwrd 0xBC, BodyGrd 0x157. → chip trait `trap(AntiDamage | AntiSword |
  BodyGuard)`.
- `dimming.rs`: `FIRST_NO_CUT_IN` 0x170, the navi chip range 0xDD..=0x118, AntiNavi 0xBA; `navi_chip.rs`/`heal.rs`
  AntiRecv 0xBD. → traits `no_cut_in`, the `navi` flag (already a chip flag), `trap(AntiNavi | AntiRecovery)`.
- The custom screen: `BEAST_OUT_CHIP` 0x13F, `INVALID_CHIP` 0x185, the builder's first special chip 0x140, `id <
  0x19B`, link navi chips `>= 0x190` (and the used-once bit `id - 0x18F`). → `roles.chips.{beast_out,
  invalid}`; the builder's range and the id bound become "a chip the folder can hold" (a class check); link navi
  chips are the navis' `own_chip`s.
- `chip_damage_formula` by formula number (1..=18 SP, 20 HP lost, 24..=44 navi level). → `ChipDamage`.

**Forms, navis and identities by number.**
- `Form` constants and `form.0` tests in battle.rs, player/mod.rs, form.rs, chip_use.rs, input.rs, intake.rs,
  idle.rs, status.rs, transform.rs (actions), beast_rush.rs, afterimage.rs, lockon_marker.rs, custom/mod.rs,
  custom/screen.rs (about 60 lines): `is_beast`, `is_beast_over`, `with_beast()` (+0x0C), the Gregar and Falzar
  ranges, the per-form bonuses, charged chips, statuses (`sub_8014536`), overlays (`sub_8011268`, `sub_8011384`),
  palettes, the ChargeCross and DustCross custom screens, TomahawkCross's immunity, GroundCross's anim 0x16 check.
  → `FormData.kind`, `game`, `beast`, `cross_of`, `chip_bonus`, `charged_chips`, `status_reset`, `overlay`,
  `palette`, and form traits `status_immune`, `extra_custom_screens`, `scrap_button`, `ground_cross_rise`.
- `Navi(n)` tests (Navi 5, 6, 7, 0x0B in player/mod.rs, intake.rs, chip_use.rs, cross_merge.rs; `Navi::MEGAMAN`
  in 16 places). → navi traits (`status_immune`, the charged-chip families of `sub_800F49E`, and `forms` for the
  navi that changes form, MegaMan, where the ruleset asks "is this MegaMan").
- NameID arithmetic and ranges (0x1A0 + navi, 0x1AB + form; 0x1A1..0x1AB link navis; 0x1AC..0x1C1 crosses;
  0x173..0x17E bosses; 0x100..0x1C3 navis; 0xDA; `<= 0xBA` viruses) in player/mod.rs, transform.rs, afterimage.rs,
  beast_rush.rs, lockon_marker.rs, intake.rs, status.rs, chip_use.rs, obstacle.rs, cross_merge.rs, and the
  frontend's HUD. → the navi's or form's `identity`; identity traits `player`, `link_navi`, `cross`, `boss`,
  `virus`, `absorbable = false` (0xDA).
- AI-index tables (reactions.rs death/flinch/drag rows, player/mod.rs overlay refresh, form.rs init and death
  hooks by row, `ai_index > 0xB`). → identity fields: `parts` (what `sub_8010DF6` puts on), `death`, `flinch`,
  `drag`, `overlay_refresh` (enums naming the routine each row calls).

**Weapons by number.**
- `idle::weapon_routine` (`Hook::Weapon(n)` and the `nullsub_44` list), `set_charge_shot_routine` (0x21..=0x26
  stick; buster 3 and 4 become 0, 0x2C becomes 0x2B), `idle.rs` `kind = 2` for 0x21..=0x26, `chip_use.rs`
  charged paths (0x18 rock barrage, 0xFF, 0x05 | 0x0D | 0x1F | 0x20 | 0x29 | 0x2D), `form.rs` `24 + form`.
  → the weapon's `setup` slot; traits `sticky`, `demotes`, `charged_chip_kind`, `rock_barrage`,
  `stale_register` (the `nullsub_44` entries, an explicit error as today).

**Assets and presentation.**
- 52 `SoundId(0x..)` literals in 16 files; sound.rs's named constants. → `roles.sounds`, `roles.music`; a stage's
  music is its `music`.
- 8 `BannerId(..)` literals (battle.rs, custom/screen.rs, dimming.rs). → `roles.banners`; win and lose banners are
  the navi's.
- 14 sprite literals (afterimage, bubble_visual, charge_glow, eruption, form_overlay, full_synchro_aura,
  idle_overlay, lockon_marker, status_visual, objects.rs). → the engine kinds' sprites as `roles.sprites` (the
  engine's kinds need their looks from content).
- 12 effect and spark ids (chip_use, reactions, common, intake, transform). → `roles.effects`, `roles.sparks`.
- Collision types 1, 2, 0x48, 0x2A and regions 0 and 1 (player/mod.rs, eruption.rs, hitbox.rs, collision.rs). →
  `roles.collision`, `roles.regions`; region 0 is `None`.

**Stages and setup.**
- `BattleSettings::netbattle_from_bytes`, `actor_list_at(address)`, `Stage { settings: u8 }`, `settings(index)`,
  `ActorKind::{Rock { variant }, Object6E, Object7D}` and their entry types. → `StageHandle`s; the codec in
  `bn6-compat`; actor entries name kinds and variant records.
- The music by settings byte, `SoundId::VIRUS_BATTLE` in a link battle. → the stage's music and
  `roles.music.link_battle`.
- `enable_turning`'s panel patterns 0x38, 0x30, 0x3C and battle mode 0x0B. → panel patterns stay (they are the
  field's own column encoding, a ruleset value), the battle mode is a named enum.

What stays numeric in the ruleset, deliberately: engine flag bits and panel flag words (the ruleset's own
encodings, matched as whole words), state-machine values that the traces observe (states, phases), battle modes,
and fixed-point constants. None of these is an identity of content.

### 7.6 The content API (Luau)

Removed, once the migration ends: `battle.spawn(pool, index)`, `battle.spawn_kind(name)` (both become
`battle.spawn(kind, pos)`, with `spawn_first`/`spawn_at_end`), `me:param`/`set_param`, `me.index` (→ `me.kind`),
`me.variant`, `me:attack_param`, numeric `me.chip` (→ a chip or nil), `me.name_id` and `battle.navi_record`
(→ `me.identity`), `me:death_hook(name_id)` and `add_navi_parts(actor_type, ai_index)` (→ by identity),
`battle.attach_point(name_id, ...)`, the weapon routine bytes (→ weapons), `battle.play_sound(number)` (→ a
sound), `battle.effect(pos, number)`, `battle.spark(owner, pos, number)`, `battle.region_effects(..., region,
..., id)`, `setup_collision(self_type, target_type, ...)` by number (→ collision definitions), numeric
`collision.region`, `hit_effect` and `status_base`, `battle.hand_chip` ids, `LinkedChip.chip`, the navi stats'
`form`/`navi`/`buster_shot` numbers, `obstacle.fly_to_absorber(me, kind)`, `lockon_panel(x, y, mode)`, and the
`data` global. Each has a typed replacement in core.d.luau; `bump_side_stat`, `add_special_bonus` and
`take_damage` get named enums for their index and mode arguments.

Added: `define`, `asset`, definition types (`Chip`, `Navi`, `Form`, `Weapon`, `Kind`, `Action`, `Effect`,
`Spark`, `Region`, `Collision`, `Status`, `Lockon`, `Stage`, `Sprite`, `Sound`, ...), reference state field types,
`me.kind`, `me.identity`, `me:set_damage_word`, `me:attach_point_pos`, `battle.spawn(kind, pos)`,
`battle.navi(side).form` as a form.

**A navi's action.** About fifteen kinds follow their owner's action and end when it changes (`owner.action ~=
s.owner_action`, `(me.related1 :: Object).action ~= 0xA`). For an object that is a navi, `navi_action` returns
the running action (an `Action` definition, or the framework state's name: `"idle"`, `"move"`, `"flinch"`, ...),
and those kinds store it in an `action` state field and compare definitions. `me.action` stays every other
kind's own state-machine byte, which the traces compare.

**Step 4 as built** (what the families of step 8 write against; core.d.luau is the reference):

- **Values.** A definition is its frozen spec table; the binding maps it to its registry and handle by identity
  and back (`Bound::def`, `def_value`). The registries' entries that are no definition (the engine's kinds,
  what registration by number makes) reach scripts as frozen stand-ins `{ id = key }` (`BindPlan::entries`), so
  `me.kind` always has a value. An asset is a frozen `{ name = ... }` per asset, one per name, with its kind's
  metatable; in the canonical tree it is `Data::Asset(kind, name)`.
- **Names.** `Sprite`, `Navi` and `Collision` were already the object's sprite, a side's stats and an object's
  registration, so the definition and asset types are `SpriteAsset`, `NaviDef` and `CollisionType`; the rest
  are as in the list above (`Kind`, `Action`, `Effect`, `Spark`, `Region`, `Sound`, ...).
- **Assets.** `asset.<kind>(name)` resolves against `Content::assets` (`AssetNames`: name to the engine's id
  per kind; a handle is the name's place in byte order). The loader leaves it empty until step 6 fills it from
  the asset root's names; the test content has a few made-up ones. Unknown names fail the define phase naming
  the module; a placeholder name (`sprite-0c-01`) is a lint error.
- **State fields.** Reference types (`"kind"`, `"action"`, `"record"`, `"record:<type>"`, ...) and asset types
  (`"sprite"`, `"sound"`, `"banner"`, `"background"`, `"mugshot"`) are two bytes, the handle plus one; a record
  of another type is refused when stored.
- **Objects and navis.** `battle.spawn(kind, pos)`, `spawn_first`, `spawn_at_end`; `me.kind`;
  `me:set_attack(action, kind)`; `me:navi_action()` (the action definition, the ruleset's own state or action by
  name: `"idle"`, `"move"`, `"dimming_chip"`, ..., or a link navi's number); `me:set_damage_word(w)`.
- **Bytes the ruleset still stores.** Effects, sparks, regions and collision types content defines get the
  engine's own number after the pack data's (`Defs::number`; `Content::effect`, `spark`, `region`,
  `field_region`, `collision_type` look past the data's tables), so the byte-typed ruleset (the generic effect's
  parameter, `CollisionData.region`, hit effects, collision types) takes them unchanged until steps 12 and 10
  make those fields handles. These are engine-internal indices, never an original number, and no script sees
  them (`CoreApi::def_number` is the binding's). A collision type carries `row_offset`, the register value its
  row's lookup leaves (index × 8) that a bug code's high byte takes: the quirk materialized in the definition.
  `collision:set_region(region)` and `set_hit_effect(spark)` take definitions (the properties stay numbers to
  read).
- **Roles.** `define.roles { actions = { ... } }`, once, keyed `roles`. The ruleset starts AntiDmg's, AntiSwrd's
  and BodyGrd's counters by role (`anti_damage_counter`, `anti_sword_counter`, `body_guard_counter`); an
  unfilled role panics naming itself where it is needed and `bn6-content check` warns, until the BN6 content
  fills every role (then an unfilled one is a load error). Their compat keys are the role actions' ids, which
  the chips' conversion chooses (proposed: `antidmg/counter`, `antiswrd/counter`, `bodygrd/counter`, numbers
  0x47, 0x48, 0x4B in actions.toml).
- **Not yet.** Chip, navi, form and stage records stay the pack data's (steps 3b and 5); the definers accept
  their specs, typed loosely (`NaviDef = { id: string, [string]: any }`) until the engine reads them. Statuses
  and lock-on modes likewise (steps 10 to 12). `me.identity` and `me:attach_point_pos` come with identities
  (step 11).

### 7.7 Checking

- **`bn6-content-check`** (the in-process type check, its own binary because its Luau collides with mlua's)
  checks every module against core.d.luau and types.d.luau as today. Library builders' spec types go in
  types.d.luau so chip modules' calls are checked (requires stay typed `any` per module, and a module casts what it
  requires, `require(...) :: BombsLib`). It adds static lints: no deprecated numeric API calls and no `legacy { }`
  markers beyond the ratchet's allowance (§12), no placeholder asset names, kind keys qualified by their owner
  folder, no module under `compat/`.
- **`bn6-content check <content> [<assets>]`** (links the runtime) runs the define phase and reports: duplicate
  keys, references to the wrong registry, unfilled roles, rule sections missing or defined twice, unknown asset
  names, chips with no usage or two, kinds in `objects/` used by one owner (colocation). Compat entries that
  don't resolve and definitions compat doesn't cover are `bn6-compat`'s check, not the engine's (§7.3).
  `cargo test --workspace` runs both on content/bn6.

As built in step 4: the lints and the ratchet read the source through a small scanner (comments dropped, string
contents masked), in `bn6-content-check`'s `lints` module. A deprecated use is a call that exists only in the
numeric API (`battle.spawn_kind`, `me:param`, `data.`, ...), `battle.spawn` with a pool, or a call whose
definition-taking argument is a number literal or a module-level numeric constant (`battle.play_sound(SOUND)`
with `local SOUND = 0x1A6`); it is a count, so an approximate one serves. `tests/deprecated.txt` holds each
module's allowance (1,263 uses in 221 modules at the start); the test fails on more, and on fewer until the
allowance is lowered (`BN6_RATCHET_LOWER=1`); `bn6-content-check --deprecated [--list]` prints the counts.
`bn6-content check` warns on unfilled roles and single-owner kinds (`bn6_content::lint`). The engine's test pack
type-checks against content/bn6's core.d.luau in bn6-content-check's tests.

### 7.8 Tests in the repository

`content/testing.rs` (1,447 lines of made-up chips, kinds and tables) goes:

- `content::testing::bn6()` loads content/bn6, the real BN6 definitions, with a synthetic asset index: every asset
  name compat/assets.toml lists, and every sprite's animations as short fixed timings (four frames of two ticks,
  the last flagged). Nothing ROM-derived: timing is invented, names are the committed table. The engine's,
  netplay's, the frontend's and the audio's tests use it, choosing chips by key (`"gundels3"`, `"invisibl"`,
  `"eraseman"`, `"areagrab"`). The timeline tests that count ticks (actions/tests.rs) are rewritten against the
  synthetic timing.
- A new test, `every_chip_runs`, plays short duels with each chip in turn and random buttons, under rollback: a
  content error anywhere fails it. It covers what no trace reaches yet, cheaply, on every `cargo test`.
- A small test pack (crates/bn6-battle/testdata/pack, a few modules) tests the define phase itself: keys,
  handles, references, errors.

## 8. Rollback, the digest and the content hash

- **State** holds handles (`u16`) where it held numbers; `Battle: Clone`, `save_state`/`load_state` and the
  digest are unchanged in mechanism. The digest's hash of a handle is as stable as its content.
- **Intern order** is byte-wise key order per registry (§2.3); two peers with the same content hash have the same
  handles.
- **Setup and inputs**: `RoundSetup` carries handles (navi, form, folders, stage) and the content hash; inputs
  are buttons, as today. Setups written by name resolve to the same handles on both peers.
- **The content hash** covers the canonical definition tree (every definition, reference and function slot),
  the module sources (the functions' code), the roles, and the assets the simulation reads (asset names and
  every sprite's animation timing). Compat is not in it: it changes no simulation. Pixels, palettes and audio stay
  out, as today.
- **The VM stays out of battles.** A runtime VM is a per-thread cache keyed by the content hash, rebuilt by the
  same define phase, and it checks it reads the content's definitions. `Battle` holds no handle to it, so a battle
  and its snapshots are `Send` by construction (§7.3). Definitions are frozen; definers fail after loading; the
  verifier still refuses writes to globals and module locals.
- **Content state** keeps its 64-byte budget; a reference field costs two bytes. The attack scratch keeps its
  "same state continues" rule, with state identity by state table (§3.5).
- **Cost.** A definition read is a frozen-table read (about 24 ns); passing a definition to the API adds a
  metatable check. The define phase costs one evaluation of every module per content load and per thread's VM;
  scripting.md measured about 5 ms to load the first 18 modules, so an estimate for the whole pack is 10 to 30
  ms, measured in step 3. `rollback_cost` is measured after step 6 and after step 13; a regression past 10% is
  investigated before merging.

## 9. The extractor, the pack and the generator

### 9.1 The extractor

`bn6-extract content <rom> <assets-dir> --names <content>/compat` writes only assets: sprites (atlas, layout,
`animations.json`) under their names, the field, backgrounds, the HUD's graphics (with chip icons named by chip
key, mugshots and banners by name) and the font with its charmap, songs, sound effects, voicegroups and samples.
`battle.rs` (1,077 lines) and the HUD's chip-name decoding leave the crate; `check_timing` compares the
graphics' animations with themselves no more (there is one copy). The overlay merge (`bn6_content::overlay`) is
deleted.

### 9.2 The pack: content and assets as two roots

A battle loads from two roots: the **content** (content/bn6, the committed Luau, with compat) and the **assets**
(the extractor's output). `bn6_content::pack::load(content, assets)` reads the modules and the asset index,
runs the define phase, and returns `Content` (plus `Compat` when asked, through `bn6-compat`). The frontend and
the tools take `--content` and `--assets`; the verification workspace points `--content` at the engine
checkout's content/bn6 and `--assets` at its extracted assets, so a content change needs no re-extraction.
A distributable pack can be both in one folder (the same path twice). The pack's TOML battle data, `registries/`,
`rules/*.toml`, `chip.toml`, `object.toml`, `weapon.toml` and `bn6_content::battle` (1,609 lines of TOML IO)
are deleted.

### 9.3 The generator

The committed Luau data is generated once, from the ROM, by a tool in the verification workspace,
`tools/gen-content`. It depends on this repository's crates by path, as the workspace's other crates do, and it
takes over the ROM decoders `bn6-extract` gives up (the chip table, navis, forms, weapons, rules, stages,
registries, the object tables, the text). It is not committed here.

- `gen-content write <rom> <content>` writes compat/*.toml and the definition modules for data: a module per chip
  or series (`chips/<key>/chip.luau` or `chips.luau`) with each chip's record and its usage (a `legacy { }`
  marker until its family converts, then the builder call, whose arguments the family's decoder reads from the
  chip's parameter bytes); navis, forms and weapons' charge times; the rule sections; stages; collision types,
  statuses, lock-on modes, effects, sparks and regions; the variant records. It never overwrites a file: once a
  module is written, people own it and the check below keeps its numbers honest.
- `gen-content check <rom> <content>` is the cross-check, run whenever content data changes: it loads the committed
  content with compat and compares every number the ROM holds for every definition with what the content defines
  (chips' records, navis, forms, weapons, rules, stages, variants), reporting differences by key. It replaces v1's
  extract-and-read-back check and the one-off 5,841-check comparison of the compiled tables.
- It writes names: chip keys, compat/assets.toml (from the disassembly's enums and first users), stage names, and
  the generated names of collision types and statuses (from what they do where the docs say, else their first
  user), with every name it had to invent in compat/curation.toml.

### 9.4 The frontend and the audio

`bn6-assets` keys sprites, backgrounds, mugshots, banners and chip icons by name; `Hud::chip_names` (font codes
by chip id) and `chip_shows_damage` go, the HUD drawing a chip's name from its definition through the font's
charmap and its damage from the `has_damage` flag. The frontend draws an object's sprite by its handle's name,
recognizes the palette flash and the form overlay by `EngineKind`, reads attach points from the navi's identity,
and plays trace files through `bn6-compat`. Its live-play driver picks its hand and navi by key. `bn6-audio`
maps a cue's sound handle to the song of that name in the sound bank; the bank's internal numbering (voicegroups,
samples, music players) stays the bank's.

## 10. The verification harness

### 10.1 What the traces compare, and what changes

`trace::compare` compares, per frame: the battle state words, ticks, RNG, pause, flags, intro bits, alive
counts, turn, battle time, gauge, the custom screens' status, the banner's activity, every object in list order
(type and index, header flags, the state word, panel, alliance, HP and max HP, position, timer, animation,
collision status flags), the panels (type and alliance) and both hands (0x50-byte chip blocks).

| Compared | v2 |
|---|---|
| object type and index | `compat.kinds[key]` for the object's kind handle (`Compat::object_slot`, from step 3) |
| a navi's state word action byte | from `NaviAction`: the framework's states as themselves, engine actions and content actions through compat actions.toml (`Compat::navi_action` maps `CONTENT_ACTION`, from step 3) |
| hands | `compat.hand_bytes(&hand)`: chip handles to ids, `(chip, code)` to `code << 9 \| id` |
| position of register-garbage kinds | compat kinds' `scratch_position`, `scratch_z_fraction`; the engine kinds' conditions (charge glow before its first update, the intro, the palette flash, the navi chip controller, `effect::xy_unknown`) by `EngineKind` |
| everything else | unchanged |

Fields that stop existing in the engine, and how the harness treats them:

- **Spawn parameters** (`params` in the trace's objects): not compared today, not modelled in v2. Ignored.
- **The attack's variant and parameters, the chip's subtype**: in actor data, which the trace doesn't compare.
  Gone; nothing to map.
- **NameIDs**: not compared. Compat keeps them for the codecs and for tools.
- **Effect, spark, region, collision type and status numbers**: never compared (they sit in parameters and
  collision data the trace doesn't record). Gone.
- **The banner id**: not compared (only whether a banner is up) and already out of the digest.

### 10.2 The harness's code

- `trace.rs` moves to `bn6-compat` (feature `trace`): `trace::rounds`, `Round::round_setup(&content, &compat)`,
  `Round::start`, `run_round(round, &content, &compat)`, `check_custom_screens(round, &content, &compat)`,
  `compare(b, frame, &compat)`. The setup codecs (`NaviStats::from_bytes`, `BattleFolder::from_bytes`,
  `ChipHand::from_bytes`/`to_bytes`, `TransformRequest::from_bytes`, battle settings, SP times) move with it and
  resolve numbers to handles through compat; a number compat doesn't know is an error naming it. The custom-screen
  check's "damage below 1000" test becomes "a fixed damage" (a formula chip's hand damage comes from the trace, as
  today). *Step 2 did the move:* the codecs are `bn6_compat::codec`'s functions (`navi_stats`/`navi_stats_bytes`,
  `battle_folder`/`battle_folder_bytes`, `chip_hand`/`chip_hand_bytes`, `transform_request`, `battle_settings`,
  `later_stages`, `sp_times`) and still produce numbers; `compare` and `run_round` take compat (the kinds'
  comparison flags, the engine's included, come from kinds.toml); `round_setup`, `start` and
  `check_custom_screens` take it when the codecs resolve handles (phase C). `Compat::bn6()` is this repository's
  content/bn6/compat, built in. NaviStats's bug-code byte writes (`sub_80139F6`) are a typed match in the
  engine, checked against the codec in bn6-compat.
- **trace-tests**: `trace_tests::content()` becomes `load(content, assets)` (the engine checkout's content/bn6
  and the workspace's extracted assets) plus compat. traces.rs, lab.rs, custom_screen.rs and rollback.rs change
  only their calls. rock_trace.rs's `(Pool::Attack, 0x59)`, `(Pool::Effect, 0x38)` and `0x87` become kind keys
  (`rock`, `rock/debris`, `absorbed-obstacle`); `actor_list_at(0x080B_1AAD)` becomes the compat stage lookup.
  bn6_data.rs's effect ids become effect keys, chip `0x11` becomes `chip_by_key("gundels3")`, and
  `object_kind("attachment")` becomes `kind("attachment")`.
- **sound-tests** map cue handles to song numbers through compat assets.
- **chiplab** (the recorder) is on the original's side and unchanged. The lab summary's blockers name keys now
  (`chip minibomb`) where the engine's messages used numbers; its grouping by family comes from the scenario
  index, as today.
- **gen-content** joins the workspace (§9.3); `cargo test -p gen-content` runs its check against the ROM.

Floors to hold at every step: machgun 1074/1331; soundmod 6728/6857/3088; the chip lab's 2511 of 3621 scenarios
fully matched and 1,997,288 of 2,314,021 frames; the rollback test at every latency; the sound calls.

## 11. How the ~650 content files convert

content/bn6 has 656 files: 421 TOML, 235 Luau.

- **253 `chip.toml`** (a `script` line): folded into the generated chip modules (step 5). Their content is the
  `legacy { }` marker's `script` until the family converts.
- **139 `object.toml`** (`[kind] pool, index, script`, the scratch flags): each becomes the `define.kind { id,
  pool, state, update }` its module returns, plus a compat kinds.toml line (pool, index, scratch flags). A codemod
  does it (step 6).
- **46 `weapon.toml`**: `define.weapon` in the weapon's module (charge times from the generator), compat
  weapons.toml lines; the 17 alias folders are deleted (step 6).
- **235 Luau modules**: a codemod (step 6) rewrites the mechanical references using the naming tables (sprite
  strings, sound, effect, spark and region numbers to named values; `spawn_kind("x")` to the required kind; the
  folder moves of §4.2 and their requires). The semantic conversion (subtype tables to builders, parameters to
  typed state, `data.*` to definitions) is by family (steps 7 and 8).
- **core.d.luau, types.d.luau**: extended in step 4, cut in step 13.

## 12. Migration plan

Every step merges to main with `cargo test --workspace` and `bn6-content-check` green and every floor of §10.2
held; a trace that runs further records its new floor. Sizes: S about half an agent-day, M one to two, L three
to five.

**The ratchet.** From step 3 to step 13 the old and the new coexist: the numeric API, the `legacy { }` marker,
the v1 registration files and the `data` global keep v1 modules working while families convert. A test counts
their uses per module against an allowlist in `bn6-content-check`'s tests that may only shrink; step 13 deletes
the allowlist with the last use. The engine never reads compat, at any step (§7.3): what content defines
reaches the traces and the game's setups through `bn6-compat`, which maps the engine's handles.

**Transitional, and when it goes** (from the exemplars, step 7, and step 9):
- **The roles' legacy markers.** rules/roles.luau names the v1 actions and kinds the ruleset needs by their
  numbers and keys until their families convert them, and then names the definitions (§7.4).
- **The legacy marker in a v2 definition.** A chip definition may carry `legacy = { subtype, params }`: the
  original's subtype and parameter bytes in its record, for what reads them of a chip besides its own use.
  Counted by the ratchet (the lint matches `legacy = {` and `legacy {`). StepSwrd, FtrSword, CrosSwrd and
  DblDream carry it for SlashCross's charged slash (weapons 0x11/0x12, action 0x41), which reads a chip's
  subtype and first parameter: exactly the coupling v2 removes. It goes when the charged slash gets a proper
  hook, the sword chips' actions exposing what the charge needs (StepSwrd's dash, CrosSwrd's two waves,
  DblDream's two slashes) as a trait or spec field SlashCross's weapon reads: with HeatCross's exemplar (step 7)
  or when family 8e converts the forms. The marker's `action` and `script` (a behaviour still a v1 module) are
  step 5's; the reader refuses them until then.
- **The weapon legacy marker.** A weapon definition may carry `legacy = { routines = { ... } }`, the routine
  numbers the pack's forms, the navis' rows and the ruleset name it by (MegaMan's buster, charged shot and blank
  shot, HeatCross's charge). Counted by the ratchet; it goes when the forms are definitions (step 5's form
  reader) and the ruleset names weapons by handle (phase C). HeatCross's charge also returns FireBrn's action by
  number (counted) until FireBrn converts.
- **Registration-by-number shims.** chips/036-minibomb (action 0x12), chips/047-sword (0x13) and
  chips/056-mchnswrd (0x49) run a record's action by its subtype for records something still names by number:
  the Cross special's chips (berserk.rs `CROSS_SPECIAL_CHIPS`: MiniBomb, EnergBom, MegEnBom and swords), the
  Program Advance recipes' ingredients (PoisSeed, the swords), records not ported (LilBoiler, VDoll), the test
  content's numbered chips. The 0x12 and 0x13 shims go when the Cross special's list and the recipes go by
  handle (phase C, step 10); the 0x49 one when the stun strike (idle.rs `set_attack(0x49)`) is
  `roles.actions.stun_strike`.
- **Chips kept on records** (their modules give only the action, with the compat key as `id`): besides the
  above, what 3b's `chip_record` refuses in a definition: `program_advances` (LifeSrd), `dark_substitute`
  (DrkSword) and damage formulas (Muramasa, ProtoMan's StepSwrd). The reader learns them (step 5 needs them
  anyway), and those chips become definitions.

### Phase A: foundations (the model-v2 agent; steps 1 and 2 can run in parallel)

1. **Compat and the generator** (verification workspace, then this repository). gen-content with the ROM
   decoders copied from bn6-extract; `write` emits compat/*.toml with keys, assets.toml with names, text.toml;
   `check` compares against the ROM. Commit compat/ here. No engine change. **M.**
2. **`bn6-compat` and the trace move.** The new crate; trace.rs and the setup codecs move into it; the harness,
   the frontend's trace playback, netplay's `rollback_cost` and the verification workspace switch to it. The
   codecs still produce today's numbers. **M.**
3. **The define phase and handles.** `bn6_luau::define`, `bn6_battle::content::define`, `Content` with registries
   and handles alongside today's fields; the runtime dispatch by handle; `define.kind`/`define.action`/
   `define.chip`/`define.weapon` accepted alongside v1 registration (a v1 file and a definition claiming the same
   thing is an error); the test pack. v1 registrations get transitional keys (a kind's folder name, an action's
   and a weapon's number) so everything has a handle from here on. Objects record their kind's handle and a navi
   runs a defined action by handle; `bn6-compat` maps both to the original's numbers (§7.3). No bridge from
   compat into the engine (the user's decision). No content changes. **L.**
3b. **Setups and hands on handles** (new with that decision; done, §7.3). Hands, folders, the linked chips, the attack
   header's chip, `NaviStats` (navi, form, weapons) and battle settings (stage) hold handles, §7.2's table; the
   records content doesn't define yet get transitional keys (a chip `v1/chip-036`); `bn6-compat`'s codecs map the
   game's bytes to handles and back (traces, saves, link data), and the comparison maps hands. The ruleset's
   numeric *logic* stays for phase C: this step changes what state holds, not what the ruleset asks of it. Before
   it, a defined chip or weapon can't be reached from a setup the original recorded. **M.**
4. **The v2 API.** core.d.luau's definers, asset resolvers, definition types, reference state fields, the
   handle-based object and battle API; the numeric API kept, marked deprecated, and counted by the ratchet.
   `bn6-content check` and the new lints. `define.roles` (§7.4) with the roles the ruleset starts content by
   already: the trap chips' counter actions (AntiDmg's, AntiSwrd's and BodyGrd's, 0x47, 0x48 and 0x4B, which no
   chip record names), with their keys in compat actions.toml. **M.**
5. **Data to Luau.** gen-content writes every chip, navi, form, weapon, rule section, stage and registry entry
   as v2 definitions in the v2 folders (§4), with `legacy { action, subtype, params, script }` markers where a
   chip's behaviour is still a v1 module; the pack's TOML battle data and bn6-extract's battle.rs go;
   `bn6_content::battle` goes; the loader takes the content and assets roots. Needs step 3b. Gate: the `Content`
   the definitions build equals the one v1 extracted (a one-off field-by-field check, as in the v1 move), `gen-content check`
   passes, the traces hold. Mostly generated. **L.**
6. **Assets by name and the codemod.** The extractor names assets from compat and writes the asset index, which
   the loader fills `Content::assets` from (done first, so real packs resolve `asset.*`); sprites and sounds become asset
   handles in the engine; the codemod of §11 runs over every module (asset strings and numbers to names,
   `object.toml`/`weapon.toml` to definitions, folders moved and renamed per §4.2, requires fixed). Mechanical,
   verified by the traces and by `every_chip_runs`. **M.**
7. **Exemplars** in full v2 form, by hand: the bombs and seeds (§5.1), the swords (§5.2), AreaGrab and
   PanelGrab (§5.4), EraseMan (§5.5), BusterUp and the plus chips (§5.6), the buster with the charged and blank
   shots, and HeatCross with its charged shot (§5.7). Update docs/design/content-migration.md's pattern section
   and the verification workspace's porting brief. **M.**

The waiting agents can start after step 7, writing their scopes in v2 (G3: the projectile and cannon family,
§5.3, with SonicBom, Z Saver, LilBoiler, VDoll; B2a and B2b: the dimming chips on `lib/dimming`; A2: the navi
chips on `lib/navi-chips`). To start them earlier, step 3b lets a chip definition be the chip (its record and
its use) wherever a setup names its compat id, so a family can be written in v2 before step 5 lands; that saves
about four agent-days on their critical path at the cost of a merge-ordering rule (step 5's generator skips
chips already defined). (The first plan did this with an engine-side bridge from compat; the user's decision
of §7.3 moves the mapping to `bn6-compat`, which needs step 3b's handles.)

### Phase B: conversion by family (parallel)

8. **The families**, one agent or packet each, each removing its `legacy { }` markers, numeric API uses and
   `data.*` reads, and colocating per §4.2: (a) projectile users, cannons, Vulcan, MachGun, AirShot, TankCan,
   Spreadr, CornSht, WideSht, BatCan (G3); (b) the dimming chips (B2a, B2b); (c) the navi chips and the link
   navis' chips (A2); (d) the instant chips and `lib/instant`; (e) MegaMan's weapons and forms; (f) field
   objects, rocks and obstacles; (g) the remaining standard chip actions (group G's list: YoYo, Thunder,
   recovery, CrakShot, CopyDmg, AirHocky, FireBrn, TrnArrw, Reflectr, IronShl, BblStar, DrilArm, Tornado,
   WaveArm, AquaNdl, H-Burst, RlngLog, AirSpin, DolThdr, WindRack, MoonBld, ElcPuls, AuraHed, MagCoil, the
   dragons, VarSwrd, NeoVari, RskyHny, GunDelSol). Each **S to M**; (a) to (c) include new porting and are
   larger.

The generator's family decoders (the chip parameter bytes to a builder's named arguments) are written by the
family's packet, in gen-content, and checked by `gen-content check`.

### Phase C: the ruleset without numbers (parallel with phase B; Rust files only)

9. **Kinds and actions**: `Object.kind`, `EngineKind`, `NaviAction`, the roles of §7.5's first two groups, the
   `INDEX` constants and `Hook` go. **M.** Done: the engine's kinds have no object slot (the validator has them
   by key) and the INDEX constants went; the ruleset spawns content kinds by `KindRole` and starts or recognizes
   content actions by `ActionRole` (§7.4); `NaviAction` replaces the navi's action byte (§7.3); the hook table
   became the definitions' slots (§3.10). The numeric API keeps its numbers (`NaviAction::numbered`, `number`)
   until step 13.
10. **Chips**: the hand, folders, the custom screen, chip use, dimming, intake and damage formulas on handles
    and traits. **M.**
11. **Navis, forms, identities and weapons**: `NaviStats` on handles, the form and navi traits, identities for
    NameIDs, weapon traits. **L.**
12. **Assets and stages**: sounds, music, banners, effects, sparks, collision types and regions through roles;
    stages on handles. **M.**

Phase C packets touch the ruleset and `core_api.rs`; the content API they expose is step 4's, so phase B isn't
disturbed. Each deletes registration by number's use for its category.

### Phase D: the end of registration by number

13. **Remove the old**: the numeric API, `legacy { }`, v1 registration (and with it `KindDef::slot`,
    `ActionDef::number`, `CONTENT_ACTION`'s byte), the `data` global, the ratchet's allowlist. Rewrite content-pack.md, scripting.md,
    content-migration.md and the engine docs' references to the pack's files; memory and brief updates. **M.**

### Size

| Phase | Steps | Agent-days | Parallelism |
|---|---|---|---|
| A | 1-7 (with 3b) | 11 to 16 | 1 and 2 in parallel; 3-7 sequential |
| B | 8 | 12 to 20 for the conversion (more with new porting) | 5 to 7 packets at once |
| C | 9-12 | 7 to 10 (3b took their state changes) | 2 to 4 packets at once |
| D | 13 | 2 | 1 |

About 35 to 45 agent-days of conversion, plus the new content the waiting agents port. Wall-clock, with phase A
on the critical path and phases B and C in parallel: roughly two to three weeks.

**Mechanical, suitable for fanning out**: step 5's generation (by area: chips, navis and forms, rules and
stages), step 6's codemod (by top-level folder, once the naming tables are fixed), step 8 (by family), steps 9
to 12 (by category). **Not mechanical**: steps 3, 4 and 7, and each family's decoder in step 8.

## 13. Open questions

1. **Two roots or one pack.** This design loads content (content/bn6) and assets (extracted) as two roots, so a
   content edit needs no re-extraction and the verification workspace always runs the checkout's content. The
   alternative keeps one folder with the content copied in by the extractor, as today. Two roots is recommended.
2. **The real content in the repository's tests.** Replacing the made-up test content with the real BN6
   definitions and synthetic timing (§7.8) makes content errors fail `cargo test` and lets `every_chip_runs`
   cover every chip, but ties the engine's tests to the content's state. Recommended, with the small test pack
   for the loader itself.
3. **Names nobody gave.** The ROM has no names for its stages (192 battle settings records, deduplicated),
   collision types, statuses, most sounds and most sprites; 57 chip records have an empty name and one is `????`,
   and three names are shared (BeastOut three times, StepSwrd and WhiCapsl twice). The generator invents names
   from use (an unnamed record reached only as a Cross's charged attack is named for it) and lists them; records
   nothing can reach are not generated, as v1 left out unreachable table tails. Accept generated names now and
   curate later, or curate before step 5?
4. **Animation numbers.** They stay numbers, as indices into their sprite (§6.3). Naming animations would need a
   per-sprite naming table on top of the sprite names, for about 3,000 animations. Recommended: keep numbers with
   named constants in the modules.
