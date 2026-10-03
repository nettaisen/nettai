# Content model v2: game data in Luau, content by name

The user's direction (2026-09-30):

1. "move all the content into the lua implementations so none of the chips objects etc rely on the original
   indexes and can be more easily composed. only assets like graphics and music should be used, the rest should be
   part of the lua content pack"
2. "objects that are specific to chips should probably be moved into those chips"

This document is the design for that change: what the content pack becomes, how content names and composes
content, where the original's numbers go, what changes in the engine and its tools, and the migration that gets
there without losing a frame of the golden traces. It is the design record: its sections say what was designed
and, in their "As built" notes, what was built. [content-pack.md](content-pack.md),
[scripting.md](scripting.md), [core-content-boundary.md](core-content-boundary.md) and
[content-migration.md](content-migration.md) (how to write content) describe what exists; they were rewritten in
the last step of the migration (§12, step 13). The migration is complete: §14 says where it ended and what stays
numbered on purpose.

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
  with the bomb it holds and the bomb it throws. Object kinds stay one per behavior; builders parameterize them
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
  this on nettai-luau's real sandbox (§7.3).
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
the frame-exact behavior of everything already ported.

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
| patch_card | `define.patch_card` | required `id` | its MB and its effects' kinds and bug flags (a game's rules read the rest; §3.11) | the card's number, in compat/patch-cards.toml |
| sprite, sound, banner, background, mugshot, chip icon | `asset.*` (§6.3) | the asset's name | names; sprites' animation timing | the ROM's numbers, in compat/assets.toml |

Singletons, defined once per pack: `define.rules(section, spec)` for each rule table (§3.8) and `define.roles`
for what the ruleset needs by role (§7.4). **Identities** (the NameID records, §3.2) are `define.identity`,
nested in the navi or form they belong to and keyed by it (`heatcross/identity`) or a field object's own (with
an `id`), which compat maps to NameIDs through navis.toml, forms.toml and rules.toml. One more registry is
internal: **state schemas**, one per distinct state table (§3.5), which
the content state store is keyed by.

### 2.2 Keys

A key is a string unique within its registry. The content is one namespace (rules-in-luau.md §7.2, the user,
2026-10-02: "maybe you should just have it all in a flat namespace and then in the chip ids directly have
bn6:cannon or whatever"): every key is written in full, its game first and then its own part, `bn6:minibomb`,
wherever it is written (modules, compat, locale tables, setups, match files, tests). The loader refuses an id
without its game. The rules below are for the part after the game.

- **Required keys** (`id = "bn6:..."`) for what is named from outside content: setups, folders, compat, tools,
  the frontend. Lowercase ASCII letters and digits in `-`-separated words, optionally qualified with `/` by an
  owner: `bn6:minibomb`, `bn6:atk-10`, `bn6:erasemn-ex`, `bn6:heatcross-beast`, `bn6:megaman/buster`,
  `bn6:eraseman/mark`. The generator (§9) makes chip keys from the in-game name (`M-Cannon` is `m-cannon`,
  `GrndMan[EX]` is `grndman-ex`, `Atk+10` is `atk-10`); where two records share a name (StepSwrd, WhiCapsl,
  BeastOut) or have none, it picks a key from the record's use and lists them in compat/curation.toml for review
  (§13).
- **Derived keys** for definitions made inside another definition's module and nested in it: `<owner key>/<field
  path>`. MiniBomb's action is `bn6:minibomb/action`; the bomb variant it throws is
  `bn6:minibomb/action/args/thrown`. A definition made while module `M` loads and not nested in a keyed definition
  of `M` is `M#n`, its place among `M`'s definitions (`bn6:lib/bombs/throw#1`, the module named by its folder). Owner-derived keys are stable under edits elsewhere and readable in
  messages and trace diffs; `M#n` keys shift when `M` gains a definition, which matters only to compat (an
  action compat maps takes an explicit `id`, §3.5). Nothing else stores a derived key.
- Kind keys follow a convention the checker warns about: a kind colocated with an owner is qualified by it
  (`bn6:eraseman/mark`, `bn6:grab/shot`); a kind in `objects/` or a family library is plain (`bn6:projectile`,
  `bn6:bomb`).
- The engine's own kinds and navi actions have keys in the `engine/` namespace (`engine/hitbox`, `engine/effect`,
  `engine/player`, `engine/move`, `engine/dimming-chip`), registered by the ruleset, not by content: of no game.
- **Asset names are written in full too** (the user: "so loading assets must also be fully qualified as well"):
  `asset.sprite("bn6:bomb")`, the pack's game and the pack's own name for the asset.

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
core.d.luau; `nettai-content-check` type-checks it (§7.7). Specs below show the fields; `?` marks optional ones.

### 3.1 Chips

```luau
export type ChipSpec = {
    id: string,
    name: string,              -- what the telop and the HUD show (UTF-8; the font's charmap draws it)
    description: string?,      -- the library text, its lines apart by "\n" (the custom screen's
                               -- description box takes keys a tick later for each line; none counts as three)
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

- `damage` is a number, or a formula the ruleset evaluates: `{ formula = "sp_navi", slot = "sp/heatman",
  by_time = {...} }` (the SP chips' damage by the user's deletion time of that navi; the slot is one of
  rules/sp-chips.luau's `slots`, the save's deletion times in the setup's order), `{ formula = "hp_lost" }`
  (Muramasa), `{ formula = "hp_last_digits" }` (NumbrBl), `{ formula = "navi_level", base = 60, per_level =
  10 }` (the link navis' chips), and the ones no BN6 chip uses (`opponent_hp`, `gauge`,
  `half_opponent_max_hp`). The original's "1000 + n selects formula n" encoding is gone; the hand holds the
  evaluated damage, as it does today.
- `program_advances` refers to its ingredients by value: `{ order = 13, code_run = { chip = cannon, count = 3
  } }`, `{ order = 0, sequence = { sword, wideswrd, longswrd } }` (`order`: where the recipe is tried among
  all of them).
- `traits` are names: `no_chain`, `aura_bonus`, `no_cut_in`, `element_sword`, `navi_slot`, `heals`,
  `user_stays`, `navi_returns_user` (§7.5). A trap chip says which trap it is (`trap = "anti_damage"`), a dark
  chip its substitute and what its use adds to the HP bug (`dark_substitute = sword, hp_bug = 2`).
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
    run_message: { counts: { number }, text: string?, portrait: Sprite? }?,  -- the no-running message:
                                                  -- its lines' characters (its timing), words, speaker
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

**As built** (step 11, identities). `define.identity` is a registry of its own (`Registry::Identity`,
`IdentityHandle`): an object holds `identity: Option<IdentityHandle>` where it held a NameID, and an object with
none is what the original's NameID 0 is, a virus with a zeroed actor record.

```luau
export type IdentitySpec = {
    id: string?,                 -- a field object's; a navi's or form's is keyed by its owner (`heatcross/identity`)
    class: "megaman" | "link_navi" | "cross" | "beast" | "cross_beast" | "beast_over"
         | "field_object" | "virus" | "navi" | "gregar" | "falzar",
    version: number?, actor_type: ActorType?, ai_index: number?,   -- the actor record (`byte_80182C4`)
    attach_points: { { number } }?,                                 -- a navi's sprite's
    held_offset: { number }?,    -- where a held thing with no attach point sits (BatCan's cannon)
    look: { sprite: SpriteAsset?, anim: number?, palette: number?, shadow: boolean?, keeps_flip: boolean? }?,
    absorbable: boolean?,        -- false: the obstacle-absorbing action and ColArmy leave it (the mine)
    scrap: boolean?,             -- false: nothing swallows it or leaves it as junk (the mine, BodyGrd's striker)
}
```

- **The class is the original's NameID range**, stated in the definition: MegaMan in his base form (0x1A0), a
  link navi (0x1A1 to 0x1AB), a Cross, Beast Out, a Cross in Beast Out, Beast Over (0x1AC to 0x1C3), a field
  object (0xCD to 0xFF); and what no netbattle object is: a virus (up to 0xBA), another navi, the Cybeasts
  (0x173 to 0x17E, which the end fade, the HP bug's blindness and the lock-on marker test). The ruleset's range
  tests are tests of the class (`is_player`, `is_navi`, `is_cybeast`, a form's). A navi's or a form's identity
  is `identity = define.identity { class = ..., ... }` in its definition, and belongs to that one navi or form
  (the engine checks its class is a player's); the afterimage and the stand-ins find the owner's sprite through
  it.
- **A field object defines its own** in its kind's module, or in the variants that differ by it (the rocks, the
  fans, the time bombs, the Anubis statues, the instruments): `local IDENTITY = define.identity { id =
  "boulder", class = "field_object", version = 1, ai_index = 1, look = { ... } }`, then `me.identity =
  IDENTITY`. Its `look` is the original's `byte_8021220` row: what stands in for the object (DustMan's junk).
  Only the 23 NameIDs an object of the content takes are identities; the table's other rows had no reader.
- **Content reads the definition**: `o.identity` is the identity's table or nil, so `o.identity.class`,
  `.absorbable`, `.held_offset` are plain field reads, and one object takes another's by assignment
  (`o.identity = user.identity`: the heroes, the farmer). DblBeast's beasts take the Beast Out forms' and
  DblHero's ProtoMan the navi's (`gregar_beast.identity`, `protoman.identity`). DustMan keeps what he took as
  identities (`"identity[8]"` state).
- **Gone**: `me.name_id`, `battle.navi_record`, `me:death_hook(name_id)`, `battle.attach_point(name_id, ...)`,
  rules/identities.luau (the actor records and looks by NameID), `Rules::actor_records`,
  `ObjectData::name_looks`, `NameData`.
- **Compat** has the NameIDs: a navi's and a form's in navis.toml and forms.toml, a field object's (and, since
  step 11's last batch, a navi chip's navi's) in rules.toml (`[identities]`). No trace compares an object's
  NameID, so they serve `gen-content check` alone, which compares each identity's actor record, attach points,
  look and traits with the ROM's.
- **One class for what no netbattle has**: the original tests viruses by more than one upper bound (the
  volleys' targets, the rock barrage's, the lock-on marker's), and every object of the content with a NameID in
  those ranges has none at all; `virus` is the one class for them, and `navi`, `gregar` and `falzar` exist so a
  pack with such objects can say so. The afterimage with its own sprite and the junk DustMan throws before it
  wears a look keep no identity, as the original's keep NameID 0.
- *Since step 11's last batch* the hooks an actor record's AI index picked are the identity's too (below).

**As built** (step 11, forms and navis). Navis and forms are definitions by handle (`NaviHandle`, `FormHandle`):
the engine has no `Navi` or `Form` number, and their definitions carry no `legacy` marker. What the ruleset asked
of them by number, they say:

- **A form's kind and game.** `kind` (`base`, `cross`, `beast`, `cross_beast`, `beast_over`) and `game` (every
  form but the base form names one) are the original's form-number ranges: `is_beast`, `is_beast_over`, the
  Gregar and Falzar ranges (the Beast's roar, Beast Over's glow and its effect, the arm's and the blade's
  animation). The base form is the form whose kind is `base` (`Content::base_form`): a link navi is always in
  it, and in it a navi's identity is its own (`Content::form_identity`).
- **What a form names.** `cross_of` (the navi a Cross is made with, whose image merges with MegaMan: the
  original's form number, less 0xC in Beast Out), `beast` (a Cross's form in Beast Out, `with_beast()`'s plus
  0xC) and `breaks_to` (what a weakness hit drops it to, `sub_8015766`: by the original's numbers the base form
  from a Cross, the Gregar beast up to the Gregar Crosses in Beast Out, else the Falzar beast; none: it stays).
- **What a form gives.** `palette` (`byte_80203EA`: `Rules::cross_palettes` went), `chip_bonus` and
  `null_bonus` (`sub_800EF34`: a family's damaging chips, EraseCross's dimming chips too; Beast Out's Null
  chips), `charged_chips` (`sub_8013236`), `charged_bonus` (`sub_8012C7C`), `charge_doubles`
  (`sub_8012AFA`), `chip_heals` (SpoutCross's Aqua chips heal), `fire_charge` (ChargeCross's, `sub_80F0608`),
  `status_reset` and `navicust_refresh` (`sub_8014536`, `sub_801469C`: named effects, applied in the
  routines' order; the refresh defaults to the reset without the lock-on marker), `hover` (Falzar Beast
  Over's), `special_volley` (the Cross special's volley, `sub_802D4F0`: the form's number, which the original
  stores where it meant 6), `cross_release_anim` (`sub_8014B18`: GroundCross's rising drill), and `traits`:
  `status_immune`, `erases`, `charged_sword_rush`, `extra_chips`, `scrap_button`, `special_holds_buster`.
  `buster_arm` (the arm lib/buster raises: its animation and what the Gregar Crosses in Beast Out add to its
  palette) is the content's own.
- **A navi.** `forms` (the forms it changes into, by game: the Crosses in their order on the custom screen,
  Beast Out, Beast Over) is what the ruleset asks where the original asks "is this MegaMan"
  (`NaviData::changes_form`); `charged_chips` (`sub_800F49E`, `byte_8021369`), `charge_doubles`,
  `fire_charge` (ChargeMan's limits by his level, `byte_802136D`) and `traits` (`status_immune`). (A round's
  record of the link navis' own chips used is a bit a navi: at most 32 navis.)
- **The tables by AI index are the identity's.** `parts`, what the actor record's init hook puts on
  (`sub_8010DF6`): a `body` overlay (its sprite; how many of the wearer's animations its depth table covers,
  `anims`, and those it is drawn `behind` in; `own_palette`; `anim_offset`), a `second` one, an `idle`
  overlay (worn while standing) or a `beast_head` (its palette, or none: the mood's). `overlay_hooks`: which of
  the death hook, an animation change, a flinch and a drag touch what the object wears (by default its death
  takes a body or head off and an animation change restarts it). `aura_anim`: the Full Synchro aura's
  animation (`sub_80C4C52`; a player's identity has one). `ice`: the ice block that fits (`byte_80E9C30`,
  `byte_80E9C4E`). The actor keeps its navi's identity through his forms (`ActorData::identity`), as the
  original's actor record does: what an animation change, a flinch or a drag does is that identity's; the
  aura's animation and the parts a form puts on are the object's identity's.
- **Gone**: rules/body-overlays.luau, `ObjectData::body_overlays`, `Rules::cross_palettes`, the sprite roles
  `beast_head` and `idle_overlay` (the parts name those sprites), `me:add_navi_parts(actor_type, ai_index, arg)`
  and `remove_navi_parts` (now `me:add_parts(identity, arg)`, `me:remove_parts(identity)`: the navi chips' navis
  define identities for their parts, which compat's `[identities]` numbers), and the generator's body-overlay
  rows. Content reads a side's navi and form as definitions (`battle.navi(side).form.kind`, `.navi.forms`,
  `.beast`, `.beast_over`).
- **Compat** keeps navis.toml and forms.toml: the trace harness and the save codecs map numbers to handles
  through their keys (`bn6_compat::codec::Ids`; a bug code may write the base form, 0, to a form byte and
  nothing to the navi byte), and the frontend's emotion window draws the pack's faces by the original's form
  and navi numbers until a form's `mugshot` names its own. `gen-content check` compares each navi and form
  definition with the ROM's of its compat number field by field (the tables, and what the original's routines
  give by number), the navi and forms each names, the arm a form raises, and each identity's parts, hooks, ice
  block and aura with what its actor record picks.

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
neighbor. The same rule applies wherever the original reads past a table into the next entry: the value is
written where it is used, with a comment naming the quirk.

**As built** (step 11, weapons). A weapon is its definition and nothing else: no routine number reaches the
engine (`weapon_numbered`, `weapon_number`, the `v1/weapon-NN` placeholders and the charge table by routine are
gone), and compat's weapons.toml alone maps the original's bytes to keys (several numbers to one key: alias
routines with the same charge times). The traits the ruleset asked routine numbers for are fields of the
definition, not a `traits` list:

```luau
export type WeaponSpec = {
    id: string, name: string,
    charge_ticks: { number },
    setup: ((navi: Object) -> Action)?,   -- none: nothing can start it (an A-charge that is its chip; a weapon nothing implements yet)
    instant: ((user: Object, spec: InstantChipSpec) -> ())?,
    instant_waits: boolean?,   -- the navi waits 8 ticks after its instant effect (one no chip has)
    sticky: boolean?,          -- as a charged shot: a chip's weapon, which a form's own doesn't replace (`sub_800FFAA`); its attack runs through a dimming
    held: boolean?,            -- as a buster: fires while B is held
    plain: Weapon?,            -- as a buster: what it gives way to while the charged shot is sticky
    charged_chip: ("bonus" | "rock_barrage")?,   -- as an A-charge with no attack of its own: the chip itself, with the bonus or after GroundCross's rocks
}
```

`plain` is on the buster that gives way (the two Beast busters, DustCross Beast's throw) rather than a `demotes`
map on each sticky weapon: six sticky weapons would repeat the same three pairs. `charged_chip` covers the
original's `nullsub_44` entries, which `sub_800FB54` tells apart by number before it would call them; started as
a weapon, one is the game's "action from a stale register" error, as before. The sticky charged shots are the
chips' own weapons, beside the chip that gives them (`bugrswrd/charge`, `bgdththd/charge`, `puncharm/charge`,
`needlarm/charge`, `puzzlarm/charge`, `boomrarm/charge`); each setup starts a chip family's action with
arguments of its own (a sword's slash, Thunder's shot, AquaNdl1's volley, an electric pulse no chip has) or
names a chip's instant effect (FireHit's fists, Boomer's boomerang: `instant`, without `instant_waits`, which
only an effect no chip has sets: TenguCross's wind), and the two bug chips' spend a bug frag
(`battle.bug_frags(side)`, `battle.spend_bug_frags(side, n)`). A form's `weapons` and a navi's are handles (`FormWeapons` of
`Option<WeaponHandle>`); a navi's definition also carries what a Cross change brings it with (`fresh`: HP, the
body's programs, the first barrier, the Mega and Giga levels, the B+Back special's damage) and its HP after one
(`cross_hp`, by side), which were tables in the engine (`byte_80210DD`, `byte_802DD88`). The content API's
weapon fields are definitions (`navi.buster_weapon`, `navi.charge_shot_weapon`, `navi.back_special_weapon`;
`battle.navi(side).charge_shot_weapon`, `.back_special_weapon`), and so are the shot programs
(`battle.navi(side).buster_shot`, `.charge_shot_kind`: a projectile variant or nil). The projectile's variants a
navi's stats can name are named records (`shot/...`, objects/projectile/variants; compat records.toml,
`projectile_variants`), and which of them come on 1 draw in 8 is each weapon's own list. A hit's bug code can
still clear a weapon byte or a shot program (0xFF, 0), but not set one by number: the engine has no numbers for
them, and no hit of the game's carries such a code.

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

A kind is one behavior, one per original object routine. What differs between its uses goes into its state
when it is spawned. State fields gain reference types that hold handles: `"chip"`, `"kind"`, `"action"`,
`"effect"`, `"spark"`, `"region"`, `"sound"`, `"sprite"`, `"record"` (any record), `"record:<type>"` (records of one
type). A field of a reference type reads back as the definition (a frozen table) or nil. So `me:param(1)` goes:
the spawner writes `s.variant = variant`, and the kind reads `s.variant.palette`.

The pool is part of the kind's definition because it is behavior: it decides the update order and which slot
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
each chip is made of (`nettai-content show minibomb`), and definitions nested in the arguments get derived keys.

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

**As built** (step 12, lock-on modes). A lock-on mode is `LockonMode` by `LockonHandle` (`Content::lockon`,
`Defs::lockons`): the definitions carry no number. A chip's `beast.lockon`, a charged slash's `lockon`, the
navi's `rush_lockon` and `me:lockon_panel(x, y, mode)` take the definition (nil: the navi stays where it is,
the original's mode 0), `ChipData::lockon_mode` and `AttackVars::rush_lockon` are `Option<LockonHandle>`, and
the one mode the ruleset names itself, the Beast claw's, is the role `lockon.beast_claw`. The modes nobody's
chip names are `beast-claw` and `beast-lunge`. The rule section keeps what the modes share (the column shifts
and the clear-path condition); the charged sword's table by variant went, since each charged slash names its
mode. The numbers are compat's rules.toml, for `gen-content check` alone. The engine's test content defines
its made-up modes under the same names (crates/nettai-battle/testdata/content/rules/lockon.luau).

**As built** (step 12, statuses). A status is `StatusEffect` by `StatusHandle` (`Content::status`,
`Defs::statuses`), with no byte anywhere in the engine: a collision's `status_base` and `status_final`, a
hitbox's `status` and the content API's `collision.status_base` and `battle.hitbox { status }` hold or take the
definition (nil: none), and a kind keeps one in a `"status"` state field. What the ruleset inflicts itself are
roles (`statuses.damage_word_paralysis`, `counter_paralysis`, `ice_freeze`, `hit_bug_blind`,
`hit_bug_confuse`), and its two tests of the byte's range are traits of the definition: `cancels_flinch` (the
freezing statuses, the original's 0x50 to 0x55) and `survives_counter` (the bubbling ones, 0x60 to 0x65).
Content tests a status's own fields where the original tested its byte (FlashBomb's flash has no hit modifier
when its status's timer is paralysis). rules/status.luau names each group's own entries plainly
(`paralyze-90`, `confuse-480`, `freeze-150`) and the entries the original reads past a group's end for what
they read (`confuse-480-past-paralyze`); the bytes are compat's rules.toml. A chip's legacy `sword` marker
(v1 record data nothing reads) keeps its raw status byte. The engine's test content defines dummy statuses
under the same names (crates/nettai-battle/testdata/content/rules/status.luau).

**As built** (step 12, effects, sparks, regions and collision types). They are definitions the engine holds by
handle (`Defs::effects`, `sparks`, `regions`, `collisions`; `Content::effect`, `spark`, `region`,
`collision_type`), with no number anywhere in the engine: an effect object and a hit spark hold their look, a
collision registration its region and its hit spark (`Option`: no region, no spark), a setup takes two collision
types, a hitbox all four. The content API takes definitions only (`battle.effect`, `battle.spark`,
`battle.region_effects`, `me:setup_collision`, `me:reset_collision_types`, `battle.hitbox`; `collision.region`
and `collision.hit_effect` read and write a definition or nil), so a hitbox with no spark says `hit_effect =
nil` where it said 0xFF, and a kind asks `c.region == nil` where it asked for region 0. What the ruleset shows
and registers itself are roles (§7.4): `effects.*`, `sparks.*`, `regions.anchor`, `collision.*`. A collision
type keeps `row_offset`, the bug code's garbage byte. rules/numbers.luau, which numbered the effects, sparks and
regions for v1 modules and the ruleset, is gone (weapons by handle took its last table), and `data.regions` and
`data.rules.field_regions` went with it. The engine's test content defines its own for the roles
(crates/nettai-battle/testdata/content/rules/ruleset.luau: one made-up look for every effect, one for every spark,
and collision types by what they are). Compat's rules.toml gives each role the original's number, for
`gen-content check` alone (§9.3).

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

**As built** (step 12). A stage is `StageData` (content/stages.rs): its panel layout, pattern, music (a sound
asset, or none), background, mode, battle number, effects and `actors`, each `{ place = "navi", side, x, y }` or
`{ place = <kind>, x, y, variant = <a record of the kind's>, argument = n }`. The round's spawn loop
(`Battle::spawn_actors`) places a navi itself and anything else through its kind's `place(spec: PlaceSpec)`,
which gets the panel, the entry's side, the variant record and the raw argument (the Guardian statue's spawner
only leaves it in a register); a stage that names a kind without a `place` is a load error. The roles
`kinds.rock`, `kinds.boulder` and `kinds.statue`, `rock.by_number`, the numbered panel layouts and actor lists
(`Content::panel_layouts`, `Rules::stages`), `StageSettings`, `ActorKind` and the `v1/stage-NN` records are
gone, and the definitions carry no legacy marker. The numbers are compat's: stages.toml has each stage's
settings indices, its layout's number and its actor list's address (the 16-byte settings codec finds the stage
by them), kinds.toml the actor-list entry type of a kind with a `place`, and records.toml the argument each rock
variant goes by; `gen-content check` rebuilds every actor list and layout from the definitions through them
and compares with the ROM. The engine's test content defines its four stages in
crates/nettai-battle/testdata/content/stages/test.luau, and tests name stages by key.

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
| `custom_screen` | rules/custom-screen.luau | the slot grid and neighbor scans (rules/custom-screen.toml) |
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
gets it as `WeaponDef::instant`; a kind's `place` is `KindDef::place` (v1's `actor_list_entry` registration
went with the stages' definitions, step 12). The instant chips' action runs the attack's `instant` effect, which chip use and such
a weapon set; the dimming and navi chip actions read the chip's usage. Content's function roles
(`hooks.first_barrier`) are in §7.4.

### 3.11 Patch cards

`define.patch_card { id, mb, effects }` (content/bn6/cards/<name>/card.luau) is a patch card, BN4's, BN5's and BN6's
Modification Card (docs/engine/patch-cards.md): the engine keeps its capacity cost and its effects' kinds and bug
flags (`PatchCardDef`), and a player's installed cards are their setup's (`PlayerSetup::patch_cards`). What an
effect does is a game's rules' (BN6's patch-cards system, rules/patch-cards), which read the effects' own fields
from the definition. Its name is the locales' (`[patch-cards]`).

## 4. Folder layout

### 4.1 The rules

```text
content/bn6/
  types.d.luau                        the game's shared types (the API: content/nettai/core.d.luau)
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

`nettai-content where <key>` prints the module that defines a key, and load errors name the module.

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
| boomerang | Boomer, HiBoomer, M-Boomer; TomahawkCross's throw | chips/boomer/boomerang (as built; TomahawkCross Beast's throw requires it) |
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
| delta-ray | DeltaRay | chips/deltaray/ |
| dragon-head, dragon-body | HeatDrgn, ElecDrgn, AquaDrgn, WoodDrgn | lib/dragons/ |
| drill | DrilArm; GroundCross's drill | chips/drilarm/drill (as built; GroundCross's drill and MstrCros require it) |
| drip-shower | DripShwr (SpoutMan's link chip) | navis/spoutman/ |
| dust-ball | DustCross's charged shot | navis/megaman/forms/dustcross/ |
| dust-cloud | DustBrk (DustMan's link chip) | navis/dustman/ |
| dust-storm, dust-storm-mote | instant effect 17 (no chip yet) | lib/instant/ until a link navi's weapon claims it |
| eagle-tomahawk, tomahawk-strike | ETomahwk (TomahawkMan's link chip) | navis/tomahawkman/ |
| elec-man, elec-thunder | ElecMan series | chips/elecman/ |
| elec-pulse | ElcPuls1-3, DestPuls | chips/elcpuls/ |
| elem-trap, elem-trap-strike | ElemTrap | chips/elemtrap/ |
| element-pillar | HeatCross Beast's and ElecCross Beast's charged shots; Darkness's dark flames | objects/element-pillar |
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
| follow-effect | DeltaRay's bursts, DElecSwd's glow (ElecMan's link chip) | objects/follow-effect |
| gauge-speed | SloGauge, FstGauge | lib/gauge-speed/ |
| golem | GolmHit1-3 | chips/golmhit/ |
| grab-shot | area-grab | lib/grab/ |
| great-yoyo, yoyo | YoYo, GreatYo | chips/yoyo/ |
| guardian, guardian-statue, guardian-strike | Guardian (written, unregistered) | chips/guardian/ |
| gust | WindRack; TenguCross's wind | objects/gust |
| heat-man | HeatMan series | chips/heatman/ |
| heat-flame | HeatMan's navi; HeatPres | chips/heatman/ (rule 4) |
| hit-flash, lunge-slash | SlashCross Beast's lunge | navis/megaman/forms/slashcross-beast/ |
| hockey-puck | AirHocky, PitHocky | chips/airhocky/ |
| honey-bee | RskyHny1-3 | chips/rskyhny/ |
| hyper-burst | H-Burst | chips/h-burst/ |
| immobilizer | instant effect 9 (no chip yet) | lib/instant/ |
| invisible | Invisibl, WhiCapsl, instant effect 2, seeking-whirl | chips/invisibl/controller (as built; the second WhiCapsl requires it) |
| iron-shell | IronShl1-3, ParaShl | chips/ironshl/ |
| junk-shot | DustCross Beast's scatter | navis/megaman/forms/dustcross-beast/ |
| justice-one | JustcOne | chips/justcone/ |
| lance | Lance | chips/lance/ |
| mine, land-mine | Mine | chips/mine/ |
| magnet | MagCoil | chips/magcoil/ |
| meteor-shower | instant effect 16 (no chip yet) | lib/instant/ |
| moon-blade | MoonBld | chips/moonbld/ |
| navi-boost | PunchArm, NeedlArm, PuzzlArm, BoomrArmSyncTrgr, DarkInvs, BugRSwrd, HubBatc, BgDthThd | lib/navi-boost/ |
| navi-effect | a second port of follow-effect (effect #0x31), deleted | objects/follow-effect |
| needle-volley | AquaNdl1-3 | chips/aquandl/ |
| panel-bursts | black-bomb, countdown-bomb, elem-trap-strike | objects/panel-bursts |
| panel-strike | Bass, MachGun1-3 | objects/panel-strike (rule 5) |
| projectile | the buster, the cannons, AirShot and many more (lib/projectile) | objects/projectile (with lib/projectile.luau's helpers) |
| proto-man | ProtoMan series | chips/protoman/ |
| reflected-shot, reflector-shield | Rflectr1-3 | chips/rflectr/ |
| riding-hit | RSlash (SlashMan's link chip) | navis/slashman/ |
| rising-bubble | the plus chips' instant effect, black-bomb, bug-bomb, guardian-statue | objects/rising-bubble |
| rock, rock-debris | stages (actor lists), rock-cube | chips/rockcube/rock and debris (as built; the stages, the encased bubble and the boulder require them) |
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
| thunder-column | EraseCross's ray, DolThdr's doll | chips/dolthdr/column (as built; EraseCross's ray requires it) |
| thunder-doll | DolThdr1-3 | chips/dolthdr/ |
| time-bom | TimeBom1-3, TimeBom+ | chips/timebom/ |
| tomahawk-man | TmhkMan series | chips/tmhkman/ |
| tornado | Tornado, Static | chips/tornado/ |
| trap-chip | AntiNavi, AntiDmg, AntiSwrd, AntiRecv, ElemTrap, BodyGrd | lib/traps/ |
| whirlwind | TenguCross Beast's charged shot | navis/megaman/forms/tengucross-beast/ |

What stays in `objects/`: absorbed-obstacle, attachment, boomerang, drill, falling-rock (with rock-chip),
flying-shot, gust, invisible, panel-bursts, projectile, rising-bubble, rock (with rock-debris), thunder-column.
Thirteen folders from 139. (Step 8f adds objects/boulder, the boulder the stages place: a role's kind, newly
ported. Darkness, ported after this table, shares the element pillar with MegaMan's Beast forms, so it stays in
objects/ too.)

**As built** (after step 13, at the user's request): a kind whose natural owner is one chip lives in that chip's
folder, keyed under it, even when other chips or forms use it too; they `require` it from there. So Invisibl's
controller is chips/invisibl/controller (`invisibl/controller`, which the second WhiCapsl requires), the
boomerang chips/boomer/boomerang (`boomer/boomerang`, which TomahawkCross Beast's throw and BoomrArm's charged
shot use), the drill chips/drilarm/drill (`drilarm/drill`, which GroundCross's charged shot and MstrCros's
GroundCross use), the falling meteor with its panel marker chips/meteors/falling_meteor and marker
(`meteors/falling-meteor`, `meteors/marker`; the instant chips' meteor shower, lib/instant/meteor_shower, uses
them), the thunder column chips/dolthdr/column (`dolthdr/thunder-column`, which EraseCross's ray lays), and the
rock with its debris chips/rockcube/rock and debris (`rockcube/rock`, `rockcube/debris`, its variants
`rockcube/rock/brittle` to `rockcube/rock/ice`; the stages place it, the encasing makes ice blocks of it, and the boulder
breaks into its debris). What stays in `objects/` is what no one chip owns: absorbed-obstacle, attachment,
boulder, bullet, element-pillar, encased-bubble, falling-rock (with its chips), flying-shot, follow-effect, gust,
panel-bursts, panel-changer, panel-strike, projectile and rising-bubble.

The rest of v1's layout moves as follows: `lib/sword.luau` and `lib/vari_sword.luau` into `lib/swords/`,
`lib/dragon.luau` into `lib/dragons/`, `lib/instant-chips/` into `lib/instant/`, `lib/buster.luau` into
`navis/megaman/weapons/buster/`; every `chips/NNN-name` folder into its chip's or series' folder; the 46
`navis/megaman/weapons/NN-name` folders into the form that uses each (or `navis/megaman/weapons/` when
several forms do), with the 17 `NN-buster` alias folders gone (compat names the aliases).

## 5. Composition patterns

Each pattern below replaces a table indexed by subtype or by a spawn parameter. A builder is a function in a
`lib/` module that returns a definition (an action, a hook, a record) made from its arguments; the chip module
calls it with the chip's own parameters. Builders share one state table per behavior (§3.5) and one kind per
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
    id = "bigbomb", -- ... the record ...
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
  definition. Its numeric API (`spawn`, `spawn_with`, by the pack data's rows) stayed for its 23 other users,
  the rows becoming looks at load, from `data`. *Gone with step 12*: the last user, the buster's arm, names its
  two looks (lib/buster.luau), and the rows (objects/attachment/rows.luau, `data.objects.attachments`) went.
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
  the chip's own action by subtype (and FlshBom's and LilBoiler's level). Records still reach it: PoisSeed;
  VDoll (Darkness's recipes name it: as a definition the recipes, which resolve 0x96 to the record, never
  match a hand's VDoll, and Darkness doesn't form); LilBolr1-3's records; the Cross special's MiniBomb,
  EnergBom and MegEnBom, which the ruleset's table picks by number and so gets the pack's records; and the
  test content's numbered bombs. The shim goes when those are definitions or roles (step 5, phase C).
- **What stays numeric**, having no v2 form yet: statuses (the flash's blinding, the bug bomb's 0x20), bug codes
  and the absorbed-obstacle kind (the BlkBomb's NameID and the attachment's Cross check are identities since
  step 11, §3.2); the hitbox's
  `hit_effect = 0xFF` ("none"; nil since step 12). The ratchet counts what it can see of them.
- **Verified** on the test content (the thrown chips' duel under rollback, the engine's tests), the type
  check, and the traces (at every latency) and chip lab on a pack extracted with asset names (step 6): the
  lab's matches are unchanged.

EnergBom and MegEnBom (a series, one folder) pass `after = energy_burst.leave` from their own
`chips/energbom/burst.luau`; the bomb kind no longer requires the energy burst. FlshBom1-3 pass their own
`held_palette` (0, 3, 6: the original's level times three, materialized) and `flash_bomb.thrower` from
`chips/flshbom/`; the seeds pass `seed.thrower(...)` from `lib/bombs/seed.luau`. LilBolr1-3 (chips/lilbolr,
definitions) pass `boiler.thrower(level)`; VDoll (chips/vdoll, kept on its record) gives its action,
`throw.action { id = "vdoll/action", ... }` with `doll.thrower()`.

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
    id = "sword", damage = 80, -- ... the record ...
    action = swords.slash.action {
        blade = swords.blades.sword,
        hit = { region = regions.single, collision = SLASH, hit_mod = 3 },
        effect = swords.effects.slash,
        sound = swords.sounds.slash,
    },
}

-- chips/wideswrd/chip.luau
return define.chip {
    id = "wideswrd", damage = 80, codes = { "H", "L", "S", "*" }, -- ... the record ...
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
chips (as built since step 10: `picks` and `sword` are the chips, and the attack's chip becomes the pick).

**As built** (step 7, the swords exemplar): content/bn6/lib/swords/ (`parts`: the blades, slashes and sounds
the family shares, and the blade's animation and palette by the navi's arm; `slash`: action 0x13's builder;
`strike`: action 0x49's; `vari`: the variable swords' library, moved from lib/vari_sword), and a folder per
chip. `SlashSpec` and `StrikeSpec` are in types.d.luau; DblDream's two swings, CrosSwrd's second hit,
StepSwrd's step and the elemental swords' colors are arguments (`swings`, `second_hit`, `step`,
`effect_palette`), and each chip's blade is an attachment look. What it settled:

- **Which chips are definitions.** StepSwrd, FtrSword, CrosSwrd, DblDream, MchnSwrd, ElemSwrd and AssnSwrd.
  SlashCross charges every Sword-family chip, and its charged slash (weapons 0x11 and 0x12, action 0x41) reads
  the chip's subtype (the blade, the wave, CrosSwrd's two waves, DblDream's two slashes) and first parameter
  (StepSwrd's dash) from its record: until step 8e the four Sword-family definitions carried them in the
  transitional `legacy = { subtype, params }` marker (the lab's `chips/0x051-stepswrd/cross-slash-charged`
  failed without it); since then each sword's slash names its charged slash (§5.7). The other swords keep the pack's records, each module giving its chip's action with its compat key as
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
  (`byte_80EBB64`), which an earlier merge lost; lib/sword's `sword.raise` had it again until step 8e, when
  each charged slash's record took its blade (§5.7).
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
    define.chip { id = "cannon", damage = 40, -- ...
        action = cannon.action { shot = SHOT, look = LOOKS.cannon } },
    define.chip { id = "hicannon", damage = 100, -- ...
        action = cannon.action { shot = SHOT, look = LOOKS.hicannon } },
    define.chip { id = "m-cannon", damage = 180, -- ...
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
return define.chip { id = "areagrab", flags = { "dimming" }, -- ...
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
  user's actor data, so its arm is lib/buster's `raise_arm_for(user, holder, slot, while_dimmed)` (`sub_80EB572`,
  as CrosOver's MegaMan's). A second attachment (Gregar's, MegaMan's copy's) is in lib/slot's `held2`.
- **A family's kind reused**: CornFsta's bursts are CornSht's corns (chips/cornsht/corn, generation 0xFF), and
  its farmer holds CornSht's gun (`cornsht.gun`).
- **Still numbers**: the shots' `hit_effect = 0xFF` (nil since step 12). (The beasts', heroes' and farmer's NameIDs, whose
  attachments sit at their sprites' attach points, are identities since step 11: §3.2.) LifeSync's immune virus is
  told by its actor data (AI 13). `battle.boss_rank` (battle effect 1) joins `battle.link` for LifeSync.

**As built** (phase B, group C5: dimming subtypes 1, 10, 11, 20, 25 and 38, converted from their v1 modules;
docs/engine/chips.md §3.6.9 and §3.6.10): lib/traps/controller (the trap chips'), chips/elemtrap (its trap and
strike), chips/timebom (controller, countdown), chips/mine (controller, land_mine), lib/gauge-speed/controller,
objects/invisible (since moved to chips/invisibl/controller, which the second WhiCapsl requires),
lib/navi-boost/controller, and objects/panel-bursts. What it settled:

- **The parameter that picked a table row is the hook's argument**: a trap chip's trap object (`traps.hook(trap?)`,
  a `trap` record whose `set` spawns it: ElemTrap's; the others pass none), a TimeBom's bomb
  (`controller.hook(variant)`, a `CountdownVariant` record: the two rows the chips set; the table's other six,
  which no chip sets, are its optional fields), the gauge's speed (`gauge_speed.slow`, `.fast`), the
  invisibility's time (`invisible.hook(ticks)`), and what a navi-changing chip changes (`navi_boost.hub`,
  `bug(routine)`, `arm(routine, palette)`, `dark`).
- **`dimming_chips.phases { hidden = true }`** shows a hidden chip's telop (`sub_800BBA8`, the trap chips').
- **A parameter the ruleset poked became a question the object asks**: `sub_802CEA6` tells a cleared record's
  object to end through its second parameter; ElemTrap's trap ends when its side's record no longer names it
  (`battle.linked(side).object ~= me`), and `Battle::clear_linked` only clears.
- **AntiRecv's counterattack is a role's kind** (chips/antirecv/controller, `kinds.anti_recovery`): the ruleset's
  heal and Roll's navi chip spawned it by its object slot; they spawn the role's kind.
- **A panel burst's look is its spawner's**: `panel_bursts.spawn(spawner, region, interval, z, effect, sound?)`
  takes the effect its table row named (and the row's sound, which no row has).
- **Which chips are definitions.** ElemTrap, Mine, SloGauge, FstGauge, HubBatc, BugRSwrd, BgDthThd and the four
  arm chips (PunchArm, NeedlArm, PuzzlArm, BoomrArm; their records' unnamed extra flags 0x04 to 0x20 have no
  battle reader and are left out). The others keep the pack's records and give their hooks from
  chips/<key>/chip.luau: AntiNavi, AntiDmg, AntiSwrd, AntiRecv and BodyGrd (the ruleset names them by number;
  BodyGrd is a Program Advance of three of them), TimeBom1-3 and TimeBom+ (a Program Advance and its
  ingredients), Invisibl (a dark chip's substitute), the second WhiCapsl (chip 0x17E: the dimming service finds
  the chips no one can cut in on by number) and DarkInvs (a dark chip with a substitute).
- **The shims** (registration by number, §12): chips/0ba-antinavi (subtype 20, by the trap's row),
  090-timebom1 (10, by the bomb's row), 0b1-invisibl (1, by the time) and 121-darkinvs (38: DarkInvs's hook;
  the other rows, whose chips are definitions, from their parameter bytes for the test content's records).
- **Still numbers** (the mine's and the bombs' NameIDs are identities since step 11, §3.2): the statuses the strike's hits
  carry, `hit_effect = 0xFF` (nil since step 12), the linked record's chip (`LinkedChip.chip`, the numeric API's), and the weapon
  routines the navi-changing chips install (0x21 to 0x26, the B+Back shield 0x3B, the busters `sub_80E97BE`
  replaces): the navi's weapon slots take numbers until the weapons are definitions (family 8e).

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
    define.chip { id = "eraseman", damage = 120, hit_param = 138, -- ...
        navi = eraseman.summon { aim_ticks = 20 } },
    define.chip { id = "erasemn-ex", damage = 140, hit_param = 138, -- ...
        navi = eraseman.summon { aim_ticks = 16 } },
    define.chip { id = "erasemn-sp", hit_param = 138, -- ...
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
navis/eraseman/chip.luau, spawns it too). The beam's collision type (`piercing-break`, row 0x16) and spark
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
- **The other navi chips took the same shape** (group A2): chips/grndman, dustman, diveman, crcusman and judgeman
  (each series' navi and what he brings), and the Program Advance and Giga navis chips/twinldrs, crosover,
  mstrcros, darkness and chips/flmhook (BigHook's and FlmHook's hook and fire). lib/navi-chips/navi.luau holds
  what the navis share (the spawn, the stand, the footing test, the action timers, the leaving). Their chips
  stay records behind the same registration by number, one module a series (chips/0fb-grndman, 0fe-dustman,
  104-diveman, 107-crcusman, 10a-judgeman by navi chip subtype; 15c-twinldrs, 15d-crosover, 15a-mstrcros,
  159-darkness and 12e-bighook likewise; 146-flmhook1 by instant chip effect 14): the navi chips for AntiNavi and
  the SP formulas, the PAs because the Program Advance table names its results by number. They go with
  EraseMan's (step 10). No `legacy` marker among them.
- **The earlier navi chips converted the same way** (group C4; they were v1 modules under objects/):
  chips/heatman (`navi`, and `flame`, which HeatMan's link chip shares, rule 4), elecman (`navi`, `thunder`),
  slashman (`navi`, `wave` with its two looks as records), chrgeman (`navi`, `car`), spoutman (`navi`, `ball`,
  `splash`, `pillar`, `geyser`, `mark`), tmhkman, tenguman, elmntman (`navi`, `meteor`, `ice`, `bolt`, `vine`),
  blastman (`navi`, `fire`), roll (`navi`, `heart`), protoman, colonel (the series and CrossDiv), bass,
  bassanly (`navi`, `shot`), deltaray and sunmoon (`sun`, `meteor`, `moon_beam`). What a chip's parameters picked
  is the hook's argument where the series differ by it: `slashman.summon { wave_damage }`,
  `elmntman.summon { cycle_ticks }`, `roll.summon { rounds, palette }`, `colonel.summon { palette, cross }`,
  `deltaray.summon { palette }`; the others' `summon` is the hook itself. What their kinds were spawned with by
  parameter is state set through typed specs (types.d.luau: `Thunderbolt`, `SlashWave`, `ChargeCar`,
  `WaterBall`, `Geyser`, `GeyserMark`, `FireBlast`, `PanelStrike`, `DarkBall`, `SunMeteor`). The panel strike
  (Bass's and MachGun's) is objects/panel-strike and the follow effect (DeltaRay's bursts, DElecSwd's glow)
  objects/follow-effect, both definitions (rule 5). The chips stay records behind one module a series
  (chips/0dd-roll, 0e0-protoman, 0e3-heatman, 0e6-elecman, 0e9-slashman, 0ef-chrgeman, 0f2-spoutman,
  0f5-tmhkman, 0f8-tenguman, 101-blastman, 10d-elmntman, 110-colonel by navi chip subtype, CrossDiv's record
  among Colonel's; 12d-bass, 12f-deltaray, 132-bassanly and 15b-sunmoon likewise: Giga chips and a Program
  Advance's result). The palette tables the records' first parameter indexed (Roll's, Colonel's, DeltaRay's) are
  in those modules, not the navis'. No `legacy` marker among them.

**The link navis' own chips** (phase B, A3): each navi's folder has its chip's action and the kinds only it spawns
(navis/heatman ... navis/dustman: `chip.luau`, and `riding_hit`, `volcano_rock`, `drip_shower`, `axe`, `strike`,
`tornado`, `clouds`); a kind the navi chip series has too stays with the series (`heatman/flame`,
`eraseman/beam`, `grndman/drill`, `grndman/rock`, rule 4), and the follow effect, which DeltaRay's bursts and
DElecSwd's glow share, is objects/follow-effect with its looks as records. lib/link_chips.luau is what the ten
routines share. Each chip is a definition in its navi's folder (navis/<navi>/chip.luau: the record, its damage
by the navi's level, and the action), which the navi's `own_chip` names. The kinds that lasted while their owner's action number was 0x0A keep his running action in an
`"action"` state field and compare definitions (§7.6). Verified against the chip lab (docs/engine/
standard-chips.md, "Action 0x0A").

**The link navis' charged attacks** (step 8e, after step 5): each navi's weapon is `<navi>/charge` in
navis/`<navi>`/charge.luau, a `define.weapon` whose action (`<navi>/charge/action`) is the original's entry 9 of
his action table, written on lib/link_chips' phases like his chip. The setup gives the attack its damage
(`weapon.charge_damage(navi, base, per_point)`), the counter byte and the element, and for TomahawkMan writes the
action's state (the ticks he raises the axe and recovers for). What a charge spawns is the navi chip series' kind
where it shares one (`heatman/flame`, `elecman/thunder`, `slashman/wave`, `spoutman/ball`, `chrgeman/car`),
DustCross's junk ball for DustMan, and one kind of its own, GroundMan's flying drill (`groundman/drill`, beside
the navi), which lasts while he runs the action he threw it in. ProtoMan's charge runs WideSwrd's action with
the chip as the attack's (`navi.attack_chip = <the record>`); his B+Back specials are the Reflector's guard with
the NaviCust Reflect's look and the NaviCust's Reflect itself, whose damage is a navi stat
(`battle.navi(side).back_special_damage`). Each navi's definition names them as its `weapons` (step 11).
docs/engine/standard-chips.md, "Action 9".

### 5.6 Instant chips: a hook per chip

v1: action 0x1C calls `Hook::InstantChip(subtype)`; one module serves the 30 chips of subtype 3.

```luau
-- chips/busterup/chip.luau
local SPARKLE = define.effect { sprite = asset.sprite("buster-up"), anim = 0 }
local SOUND = asset.sound("buster-up")
return define.chip { id = "busterup", -- ...
    instant = function(user: Object, spec: InstantChipSpec)   -- `sub_8010820`
        local stats = battle.navi(user.alliance)
        stats.attack = math.min(stats.attack + 1, 9)   -- 1: BusterUp's parameter, as data
        battle.effect(user:attach_point_pos(0x20), SPARKLE)
        battle.play_sound(SOUND)
    end,
}
```

The plus chips (subtype 3, `sub_8010488`) compose `lib/instant/plus.luau`: `instant = plus.attack(10)` for
Atk+10, `plus.navi(20)` for Navi+20; their custom-screen behavior is the chip's `modifier`, as today. A weapon
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

**As built** (step 8d, the instant chips): every chip of action 0x1C is a definition with an `instant` hook
where nothing names it by number: Boomer, HiBoomer and M-Boomer (chips/boomer, `boomerang.instant(variant)`,
the boomerang's speeds a `boomerang-variant` record; chips/boomer/boomerang also holds `boomerang.tomahawk`, which
TomahawkCross Beast's throw names), Lance (chips/lance), SandWrm1-3 (chips/sandwrm, `worm.instant(level)`, with
the hole and the sand), GolmHit1-3 (chips/golmhit, `golem.instant { palette }`), JustcOne (chips/justcone),
FullCust, SyncTrgr, WhiCapsl (`plus.attack_with(plus.PARALYZE)`), FinalGun and NumTrap. What settled:

- **A record's parameter is the builder's argument**, and a branch only another record or caller reaches is
  an argument too: the golem's `own_panel` and `cracks` (Param2 to Param4), the lance's palette, the
  boomerang's `turns`, `column` and `strong`. The effects no chip names (2, 6, 9, 11, 16, 17) are builders in
  lib/instant (`invisible.instant(ticks)`, `repair.instant`, `immobilize.instant`, `side_special.instant`,
  `meteor_shower.instant(drops)`, `dust_storm.instant { ticks, tied }`), which a link navi's weapon calls when
  it is ported; the dust storm's tie to its user's action (the game's table of action numbers by mode) is the
  action itself, compared with `navi_action()`.
- **Records kept, and their shims**: FireHit1-3 (FlmHook's and MstrCros's Program Advances name them) run
  `fist.instant(hit_mod)` through chips/06b-firehit1; MegaBstr (chip 0), Atk+10's record (the dark chips'
  substitute), Atk+30 and Uninstll (SunMoon's ingredients) and DarkPlus (a dark chip) run `plus.by_record`
  through chips/0c0-atk-10; BeastOut and the invalid chip (the custom screen's 0x13F and 0x185) run
  `plus.sparkle` through chips/13f-beastout. The nameless copies of the plus chips' record and the records
  that became definitions no longer register (any record of a subtype reaches the subtype's one registration).
- **Bytes no battle routine reads stay out**: SyncTrgr's and ColForce's menu classification flags (0x04,
  0x10, 0x20 of the second flag byte) aren't in their definitions; a lock-on mode without the Beast rush is
  `beast = { rush = false, lockon = n }` (SandWrm's).
- **Verified** on the test content (the spawning chips' duels and their rollback, the lances, the tomahawks),
  the type check, and the chip lab on a real pack.

### 5.7 Weapons and forms

```luau
-- navis/megaman/weapons/buster/weapon.luau
local SHOT = define.action { id = "megaman/buster/shot", state = STATE, update = shot }  -- action 0x11 in compat
return define.weapon {
    id = "megaman/buster",
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
    id = "heatcross/charge",
    charge_ticks = { 100, 90, 80, 70, 60 },
    setup = function(navi: Object): Action
        -- ... as today: the attack's damage, element and bonus ...
        return FLAME
    end,
}

return define.form {
    id = "heatcross",
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

**As built** (step 7): navis/megaman/weapons/buster, charged-shot and blank-shot (`weapon.luau` each: the
weapon definition `megaman/buster` with its shot `megaman/buster/shot`, `megaman/charged-shot` with
`megaman/charged-shot/action`, `megaman/blank-shot` with `megaman/blank-shot/action`), and
navis/megaman/forms/heatcross/charge.luau (`heatcross/charge`). The v1 registrations (`weapon.toml`, the 16
buster alias folders) went. What it settled:

- **A weapon definition took routine numbers** with a transitional `legacy = { routines = { ... } }` marker
  while the pack's forms, the navis' rows and the ruleset named weapons by number. *Gone with step 11* (§3.3,
  "As built"): compat alone has the numbers.
- **Charge times are the definition's own**: the charge table's row, plus the next row's first entry for
  Charge 5, which the original's table reads on into (written out and commented in each definition).
- **A role for the forced charged shot**: idle.rs's request 0x20 starts `roles.actions.forced_charged_shot`
  (rules/roles.luau, BN6's first roles file: the charged shot's action), not action 0x16 by number. The trap
  counters' roles stay unfilled until those chips convert. DustCross's throws (weapons 0x2B, 0x2C) return the
  buster's shot definition.
- **The muzzle flash and the arm a throw leaves are looks** in lib/buster (`buster.flash`, `buster.arm`), on the
  newly named `muzzle-flash` and `buster-arm` sprites; the shot's sounds are assets.
- **HeatCross is partial.** Its form stays the pack's record until step 5's form reader; its charged shot is a
  weapon definition (routine 0x06) whose setup returns its own burn (lib/burner, FireBrn's action, with
  HeatCross's burner and flame: `heatcross/charge/action`, step 8g).
- **What the attack keeps** was numeric at step 7 (the shot's kind in the attack's first parameter, the throw as
  the shot's variant 2); it is the shot's state since step 8e (below).
- **SlashCross's charged slash** (weapons 0x11 and 0x12, action 0x41) was left to family 8e (below): it read
  the charged chip's subtype and first parameter, which the four Sword-family definitions' `legacy` marker
  gave.
- **Verified** on the test content (the shots' timelines, the throw, the aliases, the role, the definitions'
  routines and charge times), the type check, and the traces and the chip lab on a real pack.

**As built** (step 8e, MegaMan's weapons): every weapon routine a v1 module implemented is a `define.weapon`
with its action a `define.action`, in the module the weapon's key names: a form's own under
navis/megaman/forms/`<form>`/ (`charge`, `drop`, `wave`, `lunge`, `scatter`, `dash`, `drill`, `tackle`,
`throw`, `throw_absorbed`, beside the kinds only it spawns: SpoutCross Beast's `surge`, TenguCross Beast's
`whirlwind`, EraseCross's `ray`, EraseCross Beast's `erase_drop`, SlashCross Beast's `lunge_slash`, ChargeCross
Beast's `charge_wave`, DustCross Beast's `junk_shot`, DustCross's `junk_ball`, SlashCross's `sword_wave` and
`slashes`), one several forms or the NaviCust name under navis/megaman/weapons/`<name>`/weapon.luau (the
Beast busters, the Beast claw, `tengu-wind`, `absorb`, `anti-damage`, `slash-a-charge`, and `shield`, which
defines the four NaviCust shields and reflects), and
the hit the dash and the tackle share in navis/megaman/dash_hit.luau (`megaman/dash-hit`). What it settled:

- **A setup writes its action's state**, not attack parameters: `navi:action_state(action)` gives the state of
  the action the setup is about to return (zeroed unless that action ran last), and the setup fills what the
  original put in the attack's variant and parameter bytes (the Beast busters' volleys, EraseCross's beam for
  its Beast form, the claw's slashes). The buster's shot keeps what it fires as `mode` (`"shot"`, `"spread"`,
  `"throw"`) and its projectile as a variant record; the charged shot its projectile. The NaviCust's shot
  programs are projectile variants the navi's stats hold (step 11); `lib/projectile`'s shots are variants
  only.
- **A weapon without an action**: TenguCross's wind names no action of its own. Its definition has an
  `instant` slot (`instant = function(user, spec)`), its setup returns nothing, and the instant chips' action
  runs the effect once and waits its 8 ticks (`Effect::RunsThenWaits`, where the original tested the attack's
  variant).
- **A kind that follows its owner's action keeps the action**: where the original compared the owner's action
  number (the whirlwind, the erase ray, the dash hit), the kind's state holds the action it was spawned under
  and compares `owner:navi_action()` with it; the absorbed obstacle asks whether its navi runs
  `megaman/absorb`'s action.
- **Roles name the definitions**: `cross_protect` is GroundCross's drill's action, `cross_death` SlashCross
  Beast's lunge's, `beast_claw` the claw's, `dust_beast_scatter` the scatter's (rules/roles.luau; the test
  content's roles too).
- **Kind keys by form**: a kind in navis/`<navi>`/forms/`<form>`/ is keyed `<form>/<name>` (the lint takes
  the form as the owner), one beside the navi `megaman/<name>`.
- **Charge times** are each definition's own row with the next row's first entry, as step 7's; a B+Back
  special, which nothing charges, still gives its row (zeros).
- **SlashCross's charged slash asks the chip for its slash.** What the slash is (the wave it sends, a second
  hidden wave, the blade or none, the swing's sound, a second slash, the Beast rush's lock-on mode) is a
  `ChargedSlash` record (navis/megaman/forms/slashcross/slashes), where the original read a table by the
  attack's variant for each part. A sword names its own in its action's spec (`slash.action { ...,
  charged = slashes.wide }`; MoonBld's swing its ring; a step sword's `step` is the dash), and the A-charge
  (`megaman/slash-a-charge`) reads it of the attack's chip: `navi.attack_chip`, the attack's chip as its
  definition, is the run-time read (`chip.action.args.charged`). A variable sword says `charged = "own"`: it
  runs its own action, charged, and its pick starts as the charged slash of the pick's action
  (`slash_charge.of_action`). The charged slash's action keeps the record and the dash in its state; the
  charged shot (`slashcross/charge`) starts it with its own (`slashes.charge`). The sword wave's rows are named
  variants (`sword_wave.waves`). The Beast rush takes the charged sword's lock-on mode from the attack
  (`AttackVars::rush_lockon`, which the charge sets from the slash's `lockon`) instead of the rules' table by
  the variant (`charged_sword_modes`, which nothing reads any more and goes with the pack's rules). The four
  Sword-family definitions' `legacy = { subtype, params }` markers and lib/sword went.
- **A chip still on a record** has no definition to ask (`attack_chip` reads as a stand-in with only its
  `id`), and its action is a registration by number: the A-charge then does as the original, by the action's
  number (the variable swords', MoonBld's) and the record's subtype and first parameter (`slashes.by_row`,
  the one table by number left). It goes when the swords SlashCross charges are definitions.
- **Still numeric**: `lockon_panel(x, y, mode)` in GroundCross's drill and SlashCross Beast's lunge and the
  slashes' `lockon` (lock-on modes are step 10's), the forms by number (`battle.navi(side).form`), and the
  variable swords' picks' chips (§12). (Each weapon's `legacy = { routines }` went with step 11.)

### 5.8 Standard chip actions: a builder per action, the chips' parameters its arguments

v1: a standard chip's action is one module per action number (`chips/NNN-name/chip.luau`) that reads the chip's
subtype (`me.variant`) and parameter bytes (`me:attack_param`), and its objects are `objects/<kind>` modules that
switch on spawn parameters (`me:param`).

```luau
-- chips/gundels/chips.luau: a series; what the subtype and the pack's `[gun_del_sol]` data gave is each chip's.
define.chip { id = "gundels3", -- ... the record ...
    action = action.action { gun = action.gun(6), firing_ticks = 120, beam = BEAM, beam_in_sun = BEAM_IN_SUN } }

-- chips/heatdrgn/chip.luau: a family's builder (lib/dragons) and its variant record.
define.chip { id = "heatdrgn", -- ...
    action = action.action { dragon = dragon.variant { speed = 0x4_0000, palette = 0, hit_mod = 3,
        spark = sparks.fire, panel = false, delays = { 3, 6, 9, 12 } } } }
```

**As built** (step 8g, the second half: AquaNdl, H-Burst, RlngLog, AirSpin, DolThdr, WindRack, MoonBld, ElcPuls
and DestPuls, AuraHed and StreamHd, MagCoil, the dragons, VarSwrd and NeoVari, RskyHny, GunDelSol). Each family is
a folder: `action.luau` (the builder, `action.action { ... }`, with one state table for the family, `action.STATE`),
its kinds beside it (`chips/aquandl/{volley, needle}`, `chips/h-burst/burst`, `chips/rlnglog/log`,
`chips/airspin/{top, whirl}`, `chips/dolthdr/doll`, `chips/moonbld/blade`, `chips/elcpuls/pulse`,
`chips/aurahed/head`, `chips/magcoil/magnet`, `chips/rskyhny/bee`, `chips/gundels/beam`), and `chips.luau` (a
series) or `chip.luau`. The dragons' builder and kinds are a family library, `lib/dragons/` (`action`, `head`,
`body`, `dragon`), with a folder per chip; the gust and the thunder column stay in `objects/` (WindRack's and
TenguCross's; DolThdr's and EraseCross's) as definitions. The builders' specs are in types.d.luau
(`AquaNeedleSpec`, `HyperBurstSpec`, `RollingLogSpec`, `AirSpinSpec`, `DollThunderSpec`, `WindRackSpec`,
`ElecPulseSpec`, `AuraHeadSpec`, `MagCoilSpec`, `DragonSpec`, `HoneySpec`, `GunDelSolSpec`, `VariSwordSpec`).
What it settled:

- **A parameter byte is a named argument**, written in the chip: AquaNdl's needle palette, H-Burst's ten
  bursts, a rolling log (`{ big, stop_ticks, hp, thrown }`), AirSpin's top (its palette and spins), DolThdr's
  thunder ticks, an electric pulse (`{ hit_mod, status, ticks, palette, bug }`: v1's rows by look), an aura head
  (`{ palette, speed, strong, far }`), MagCoil's ticks, a dragon (`DragonVariant`: the four tables by subtype,
  a record the head and its body segments hold), a bee (`{ palette, speed, turn_speed }`: the table by level),
  GunDelSol's gun, firing time and beams (v1's `data.chips[id].gun_del_sol` and `data.objects.sun_beam_looks`).
  Branches no chip takes stay arguments (`HoneySpec.drags`, `DollThunderSpec.holds_arm`, AirSpin's seeking
  whirlwind as a sender).
- **A weapon composes a chip family's action**: ElecCross's charged shot is `dolthdr.action { id =
  "eleccross/charge/thunder", ticks = 15, holds_arm = true }`, TenguCross's `windrack.action { id =
  "tengucross/charge/gust", rack = windrack.racks.tengu_fan }` (both v1 weapon modules still, returning the
  definitions; their keys are in compat actions.toml), where v1 named the action's number and set the attack's
  variant and parameters for it.
- **The target column is a helper**: `panels.enemy_column(me)` (lib/panels, `sub_80ED040`), MachGun's and the
  dragons' (which fall back to the column right ahead when no enemy is found).
- **A kind another series borrows stays with its series** (§4.1, rule 3): GunDelSol's beam (`gundels/beam`,
  spawned with its look, a `SunBeam`, and whether it shows while dimmed) is chips/gundels', and CrosOver's
  Django (chips/crosover) requires it.
- **Which chips are definitions.** AquaNdl1-2, RlngLog1-3, AirSpin1-3, DolThdr1-3, WindRack, MagCoil, the four
  dragons, RskyHny1-2, GunDelS1-3 and GunDelEX. The others keep the pack's records, their modules giving the
  action with its compat key as `id`: AquaNdl3 and RskyHny3 (MstrCros's recipe names them by number), H-Burst,
  DestPuls and StreamHd (Program Advances) with ElcPuls1-3 and AuraHed1-3 (their recipes' ingredients; the
  ruleset also knows the aura chips by number), MoonBld, VarSwrd and NeoVari (SlashCross's A-charge, weapon 0x11,
  asks a chip for its action's number, and the Beast rush's chain leaves the variable swords out by their
  chip numbers).
- **The shims** (registration by number, §12): chips/03f-aquandl3 (action 0x32, by the needles' palette),
  152-h-burst (0x34), 029-rlnglog2 (0x36), 010-gundels2 (0x37, by subtype), 027-rskyhny3 (0x39, by level),
  021-dolthdr3 (0x3E), 054-moonbld (0x40), 022-elcpuls1 (0x42, by look), 05f-aurahed1 (0x43, by subtype and
  palette), 052-varswrd (0x53) and 053-neovari (0x54). Each runs the chip's own action for a record: the chips
  kept on records above, and the Cross special's chips (GunDelS2, RlngLog2 and 3, DolThdr2 and 3, MoonBld), which
  the ruleset's table picks by number. The dragons, AirSpin, WindRack and MagCoil have none: nothing names them
  by number.
- **The variable swords' picks are actions.** `vari.action { sequences, picks, sword, random, no_charged }`:
  the picks are the swords' own action definitions (LongSwrd's, FtrSword's, ..., SonicBom's and SprSonic's from
  chips/sonicbom), started with `me:set_attack(action, 0)`. The attack still takes the pick's chip number, with
  its record's subtype and parameters (`legacy = { chips, sword }`, §12): the Beast rush reads the chip's
  lock-on mode, and SlashCross's charged sword (action 0x41, v1) the subtype and first parameter.
- **What stays numeric**, having no v2 form yet (the volley's and the top's target tests are by the identity's
  class since step 11): statuses
  and bug codes, forms and navis by number in the variable
  swords, the charged sword's action (0x41, a v1 weapon action) a charged pick becomes.
- **The test content** runs BN6's GunDelSol through its numbered SunGuns (chips/010-gundels2's registration),
  and takes RskyHny2 and ElecDrgn as the definitions they are (`testing::BEES`, `testing::DRAGON`). The pack's
  `gun_del_sol` chip data and `sun_beam_looks` are read by no script any more; they go with the pack's battle
  data (step 5).

#### The first half (step 8g, part 1)

**As built** (YoYo, Thunder, the recovery chips, CrakShot, CopyDmg, AirHocky, FireBrn, TrnArrw, Reflectr,
IronShl, BblStar, DrilArm, Tornado and WaveArm, from v1 to v2): each action is a builder in its family's folder,
taking what the original's subtype and parameters chose, and each object a kind beside it with those choices in
its typed state or a variant record:

| Family | Builder | Kinds (records) | Chips |
|---|---|---|---|
| YoYo | chips/yoyo/throw (`{ id, yoyo = { great?, panels, rounds } }`) | `yoyo/yoyo`, `yoyo/great-yoyo` (both in chips/yoyo/yoyo, which each name) | YoYo and GreatYo: records (chips/yoyo/chip, chips/greatyo/chip) |
| Thunder | chips/thunder/shoot (`{ ball = { fast?, panels, status?, bug? } }`) | `thunder/ball` | Thunder: a definition; DarkThnd: a record (chips/darkthnd) |
| Recovery | chips/recov/heal (`{ hp }`) | | Recov10 to Recov300: definitions (chips/recov/chips); DrkRecov: a record; the heal table's last row (no chip) is `recov/none` |
| CrakShot | chips/crakshot/chips' `dig(region)` | `crakshot/shot` | CrakShot, DublShot, TrplShot: definitions |
| CopyDmg | | `copydmg/mark` | a definition |
| AirHocky | chips/airhocky/flick (`{ puck, down }`) | `airhocky/puck` (`hockey-puck-variant`: `byte_80C9818`'s rows, the chips' by name) | AirHocky, PitHocky: records |
| FireBrn | lib/burner/burn (`{ burner, flame = { ticks, spread, wide?, cracks?, anim?, on_panel? } }`) | `flame` (lib/burner/flame) | FireBrn1-3, WideBrn1-3: records; HeatCross's charge returns its own burn |
| TrnArrw | chips/trnarrw/chips | `flying-shot` (objects/flying-shot: `flying-shot-variant`, `byte_80C6038`'s rows, also the buster's throw and the Falzar Beast buster's) | TrnArrw1-3: definitions |
| Reflectr | chips/rflectr/guard (`{ ticks, look, counter?, heedless? }`) | `rflectr/shield` (`reflector-shield-look`: `byte_80C9664`'s rows, the chips' and the programs' by name), `rflectr/shot` | Rflectr1-3: definitions; the NaviCust Shield and Reflect (weapons 0x3B, 0x3C, 0x8B, 0x8C) return their own guards, `megaman/shield/action` and `megaman/reflect/action` |
| IronShl | chips/ironshl/throw (`{ shell = { palette, speed, bumps, para? } }`) | `ironshl/shell` | IronShl1-3, ParaShl: records |
| BblStar | chips/bblstar/chips' `blow { speed, palette }` | `bblstar/star` | BblStar1-3: definitions |
| DrilArm | | `drill` (chips/drilarm/drill, kind `drilarm/drill`, with the drill arm look; GroundCross's charged shot and MstrCros's GroundCross spawn it too, `drill.spawn(owner, element, z, damage, { ticks, light? }, slot?)`) | a definition |
| Tornado | chips/tornado/chips' `blow { fan, single? / spread? }` | `tornado/tornado` (`tornado-variant`: `byte_80CA064`'s rows) | Tornado, Static: definitions; the action's subtype 3 (no chip) is `tornado/back-spread` |
| WaveArm | chips/wavearm/strike (`{ wave, three_rows? }`) | `wavearm/wave` (`shock-wave-variant`: `byte_80C6B00`'s rows, the chips' by name; the viruses' too) | WaveArm1-3, PwrWave1-3: records |

What it settled:

- **Which chips are records.** A Program Advance's ingredient or result (YoYo, GreatYo, AirHocky, PitHocky,
  FireBrn1-3, WideBrn1-3, IronShl1-3, ParaShl, WaveArm1-3, PwrWave1-3) and a dark chip (DarkThnd, DrkRecov) keep
  the pack's record, their modules giving the action with the compat key as `id`. A chip the ruleset reaches by
  number but only as the record's own use (the Cross special's TrnArrw2, TrnArrw3, Tornado; the dark
  substitutes Thunder and Recov10) is a definition, and its record still runs through the shim.
- **The shims** (§12): chips/013-yoyo (0x18), chips/01e-thunder (0x1F), chips/09a-recov10 (0x20),
  chips/032-airhocky (0x26), chips/014-firebrn1 (0x27), chips/018-trnarrw1 (0x28), chips/07b-ironshl1 (0x2C),
  chips/034-tornado (0x2F), chips/05c-wavearm1 (0x31) run a record's action by its subtype, or by the parameter
  that tells the records apart where the subtype doesn't (DarkThnd's 12 panels, PitHocky's puck, WideBrn's wide
  bit, IronShl's speed, ParaShl's third). Records within a series that differ only in damage (FireBrn1-3,
  WideBrn1-3, PwrWave1-3, TrnArrw1-3, WaveArm1-3, whose waves are identical rows) run the first one's action.
  CrakShot's, CopyDmg's, BblStar's, DrilArm's and Reflectr's numbered registrations went: nothing names their
  records by number any more.
- **Shared definitions added**: the collision type `second-drill` (row 0x4A), named by its one user (to
  review); the element sparks by name, `sparks.fire`, `aqua` and `elec` (hit effects 1 to 3);
  `trajectory.sine` (the sine table, which stays the pack data's until rules/math.luau); lib/panels'
  `random_in_region` takes a region definition. The rest are the shot chips' (§5.3): `breaking` (row 0x06),
  `pushing` (0x12, a tornado's too), the sparks `plain`, `charged` and `breaking`.
- **A table's rows no chip has** stay where the table is, as the original has them: a variant table keeps
  its rows by number privately and exports the chips' by name (`puck.variants.airhocky`,
  `wave.variants.pwrwave`, `shield.looks.rflectr2`), and an action's form no chip has is a definition with
  its own compat key (`tornado/back-spread`, `recov/none`).
- **Asset names**: the sprites `yoyo-arm`, `burner`, `heatcross-burner`, `drill-arm`, `hand-fan` and `shock-wave`
  (compat/assets.toml and curation.toml).
- **What stays numeric**: statuses and bug codes, elements (the attack's element byte), Beast forms by number
  (the Reflector's head animation), and in the shims the subtype or parameter that picks a record's action.
  TrnArrw's arrows and the flame take their navi's hand from its sprite's attach point (`me:attach_point`), not
  from a NameID.
- **Verified** on the test content (CrakShot's duel and its rollback, Recov50's heal, Rflectr1's guard and
  wave), the type check, and the traces and the chip lab on a pack extracted with asset names: every scenario of
  these families matches as before.

### 5.9 Field objects: obstacles, their variants and their looks

v1: the rock reads its row of the pack's `rocks` table by its first parameter and its entrance by its third; an
obstacle a navi absorbs passes its "obstacle kind", an index into the pack's `absorbed_sprites`; an actor list's
entry reaches the rock's `actor_list_entry` by its type number, and the other types panic.

**As built** (step 8f, the field objects):

- **The rock** (objects/rock; since moved to chips/rockcube/rock, kind `rockcube/rock`) is a definition. Its rows
  are `rock-variant` records (`rock.variants.cube`, `ice`, ...: animation, HP, element, debris palette, break
  sound, NameID), its entrance a state enum (`"rise" | "instant" | "fall" | "placed"`), and `rock.spawn(x, y,
  side, { variant, class, entrance }, damage)` takes them; its debris is `rock/debris` beside it (since
  `rockcube/debris`, chips/rockcube/debris). A stage names the variant its rocks are (step 12), and the kind's
  `place` gets it as `spec.variant`.
- **RockCube and IceCube** (chips/rockcube) are definitions whose `dimming` hook is `cube.places { variant =
  rock.variants.cube }`: the chip's parameters (the rock's row, class and entrance) are the builder's argument.
  No Program Advance, dark chip or recipe names either, so neither keeps a record.
- **An absorbed obstacle's look is its obstacle's own record** (`absorbed_obstacle.look { sprite, keeps_parts?,
  own_facing? }`, an `absorbed-look`): the original's kind numbers and their sprite table are gone, and so are
  the two tests the original made of the kind (kind 2 keeps the hidden sprite parts; sprite 0x23 keeps its own
  facing when thrown), which are the look's flags. `obstacle.fly_to_absorber(me, look)`, the navi's absorbed
  list (`absorbed`, `push_absorbed`, `pop_absorbed`), the throw's attack (`thrown_look`, `thrown_anim`, which
  replace the packed marker word) and the flying shot's state carry the record; the engine stores its handle
  and checks its type. The absorbed obstacle itself (objects/absorbed-obstacle) and the falling rock with its
  chips (objects/falling-rock) are definitions, which their roles name.
- **What a stage places** is a kind with a `place`: the rock (entry type 8), the boulder (type 3,
  objects/boulder: newly ported, one of the field's two stage objects, `obstacle.stage_slot_free`,
  `enter_stage`, `leave_stage`) and the Guardian statue (type 9, chips/guardian/statue's `place`). The stages'
  own entries name them (`{ place = boulder.kind, x, y }`, §3.7; until step 12 roles stood in for them).
- **A spawner's bug is content's**: the boulder's spawner replaces the new object's header flags with a byte
  it reads through the wrong register (the console's open bus). The module says what is read and stores it
  (`me:set_header_flags`), the one place content writes the flag byte whole.
- **Verified** on the test content (the rocks' placement, breaking and throw; the cube chip's duel and its
  rollback; DustCross's absorption and throws; the boulders' placement, flags and breaking), the type check,
  the traces (the soundmod stage's ice blocks and their absorption), and the chip lab on a real pack, with
  sixteen new stage scenarios for the boulders and the stages' statues.

### 5.10 The supports, the shared waves and pillars

**As built** (phase B, group C5: converted from their v1 modules):

- **The supports** (lib/supports: `controller`, `rush`, `beat`, `tango`, `heal`). The controller is the role
  `kinds.support`'s definition. The ruleset told it which support comes and the chip Rush eats through its
  spawn parameters; it sets the controller's state instead (`support`, an enum it sets by name:
  `set_state_variant`; `eaten`; `telop_chip`), and the support's out flag (the original's second parameter,
  which the support sets and clears) is the controller's `out`. Rush leaves the second WhiCapsl in the hand by
  its chip number still (that chip is a record, §5.4): the one chip number left in these modules.
- **SlashCross's sword wave** (navis/megaman/forms/slashcross/sword_wave): its rows (`byte_80D7F4C`) are
  `SwordWaveVariant` records, `sword_wave.spawn(owner, variant, x, y, element, damage, hidden?)`, named by the
  sword each is of (`sword_wave.waves`; each charged slash's record names its own since step 8e, §5.7). The
  pack's `sword_waves` data is read by no script any more.
- **The element pillar** stays in objects/ (HeatCross Beast's and ElecCross Beast's charged chips and
  Darkness's dark flames share it), keyed `element-pillar`. What its kind number picked is the spawner's
  `ElementPillar`: lightning or flames, its times, how far back it stands, and the owner's action it lasts
  through: the navi action its owner runs as it sends it (`owner:navi_action()`, kept in an `action` state
  field), or, for an owner that is no navi, that owner's own action (Darkness's).
- **SlashMan's sword wave and SunMoon** are the navi chips' group's (C4, §5.5): chips/slashman/wave and
  chips/sunmoon.
- **A kind's key in a form's folder**: the checker takes `navis/<navi>/forms/<form>/` as the form's folder (a
  kind there is `<form>/...`), and leaves a navi folder's index prefix out of its owner's name.

## 6. Compat: the original's numbers

### 6.1 What it holds

content/bn6/compat/ is a folder of TOML files, written by the generator and edited by hand when content is
renamed or added. It maps keys to the original's numbers, and holds the comparison hints that only the traces
need.

| File | Holds |
|---|---|
| chips.toml | `minibomb = { id = 0x036, action = 0x12, subtype = 0 }` for every chip; action and subtype are documentation (the traces never compare them) |
| actions.toml | action key to navi action number: `"minibomb/action" = 0x12` (every chip whose use is an action has `<chip>/action`), `"megaman/buster/shot" = 0x11`, a weapon's `"<weapon>/action"`, `"engine/move" = 0x10`, `"engine/form-change" = 0x1C`. A role action whose number a chip's or weapon's action has is that action (the volley is WideSht's 0x30); only the turn (0x3B) has its own, `"megaman/turn"` |
| navis.toml, forms.toml | `eraseman = { navi = 0x04, name_id = 0x1A4 }`, `heatcross = { form = 0x01, name_id = 0x1AC }`; the base form has no `name_id` (it is MegaMan's). The trace harness and the save codecs read them (the engine has no navi or form number), and `gen-content check` compares each definition with the ROM's navi or form of its number |
| weapons.toml | `"megaman/buster" = [0x00, 0x2E, 0x2F, 0x3E, 0x3F, 0x4D, ...]`, one line per weapon: the numbers whose `off_80117D4` entries are one routine. `nullsub_44`'s numbers are split by what the ruleset does with them (`megaman/rock-barrage`, `megaman/charged-chip-bonus`, `megaman/stale-register`). Every number a form's row (`byte_8020354`), a navi's (`byte_80210DD`) or a known NaviStats (NaviCust programs) names |
| kinds.toml | `bomb = { pool = "attack", index = 0x08 }`, keyed by the v2 keys (§4.2); `scratch_position`, `scratch_z_fraction`, `scratch_position_without_sprite` (the charge glow's condition) and `actor_list_entry` (the actor lists' entry type that places the kind: 8 for `rockcube/rock`, 3 for `boulder`, 9 for `guardian/statue`); the engine's kinds as `"engine/..."` |
| stages.toml | `"netbattle-1" = { settings = [0x00], layout = 0x00, actor_list = 0x080B1989 }`: the settings indices that are the stage, its panel layout's number and the address its actor list goes by. No two of the 192 records are identical (96 layout and actor-list pairs, each with two effect words), so there are 192 stages |
| records.toml | the few records a setup or an actor list names by byte, key to byte: the save's SP deletion-time slots (`[sp_slots] "sp/eraseman" = 3`); the rocks a stage places by the entry's argument (`[rock_variants] "rockcube/rock/cube" = 1`); NaviCust buster shots when their producers are known |
| rules.toml | the original's numbers of rule definitions, which nothing the traces compare reads and only `gen-content check` uses to rebuild the ROM's tables: `[lockon] cannon = 0x01` (the lock-on modes, `jt_8026584`), `[statuses] paralyze-90 = 0x10` (a hit's status byte, `off_80209EC`); and for the roles that name an effect, a spark, a region or a collision type (rules/roles.luau), the number the original's routines name each by, by role: `[effects] deletion = 0x03`, `[sparks] guard = 0x08`, `[regions] anchor = 0x01`, `[collision] navi = 0x01`; and likewise for the roles that name assets, `[sounds] hit = 0x06D`, `[music] link_battle = 0x015`, `[sprites] eruption = "10-24"`, `[banners] draw = 0x1C` |
| assets.toml | asset names to ROM numbers: `[sprites] bomb = "0c-02"`, `[sounds] throw = 0x1A6`, `[backgrounds]`, `[banners]`, `[mugshots]`; every asset the ROM has, the unnamed under placeholders (§6.3); chip icons follow chips.toml |
| text.toml | the text encoding the generator and the extractor share: `glyphs`, what each byte below `first_control` (0xE0) draws, as UTF-8 (the game's marks as characters: Ⓐ, the EX and SP glyphs U+E002 and U+E003; text-rendering.md §10.5) |
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

Compat is read by `bn6-compat`, a new crate that depends on `nettai-battle` (so the engine can't depend on it), and
by the tools that interoperate with the real game:

- **the golden-trace harness** (§10): `trace.rs` moves from `nettai-battle` into `bn6-compat` behind its `trace`
  feature;
- **the setup codecs**: `NaviStats`, `BattleFolder`, `ChipHand`, `TransformRequest`, battle settings and SP times
  from the game's bytes and back, for traces, real saves, folders, NaviCust setups and link navis;
- **the extractor**, to name the assets it writes (§6.3, §9.1);
- **the sound comparison** (verification workspace), to map cues to song numbers;
- future netplay interop with the real game.

### 6.3 Assets and their names

Content refers to an asset by name, written in full (its pack's game first, rules-in-luau.md §7.2), resolved
while loading: `asset.sprite("bn6:bomb")`, `asset.sound("bn6:bomb-hit")`, `asset.banner("bn6:program-advance")`,
`asset.background("bn6:netbattle-blue")`, `asset.mugshot("bn6:heatcross")`. An unknown name, or one without its
game, is a load error naming the module. (The examples elsewhere in this document predate the flat namespace and
write ids and names without their game.) The resolved value is a handle into the asset registry; state holds it
(`sprite:load(BOMB)`), and a cue carries it (`battle.play_sound(SOUND)`).

- **Names** come from compat/assets.toml, which the generator writes: the disassembly's song and sound enum names
  where they exist (`SONG_VIRUS_BATTLE` is `virus-battle`, `SOUND_HIT_BOMB_1` is `hit-bomb-1`), else a name from
  the asset's first user (`erase-mark`), else a numbered placeholder (`sprite-0c-26`, `sound-101`, `banner-NN`). The
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
  entries of the table, written as v1's extractor already writes them (`EraseMn[EX]`). (Since 2026-10-02 every
  mark is one character, the Private Use Area's where Unicode has none: text-rendering.md §10.5.)

### 6.4 How nothing else can read it

- **Luau can't.** Compat is TOML. The loader discovers modules by `.luau`, `require` resolves modules only, and the
  define phase never exposes compat. No Luau API returns an original number (the numeric API is removed, §7.6).
- **The engine can't.** `bn6-compat` depends on `nettai-battle`; a dependency the other way is a cycle. `Content`
  has no compat fields at any step: the validator maps the engine's handles to the original's numbers, and no
  bridge carries them into the engine (§7.3, the user's decision). A test in `bn6-compat` asserts `nettai-battle`'s
  dependency list doesn't contain it, and a source guard in `nettai-battle`'s tests fails on the word `compat`
  outside comments in the engine and the crates it runs content through.
- **The checker enforces the rest** (§7.7): no deprecated numeric API use, no `legacy { }` markers, no placeholder
  asset names, once the ratchet reaches zero.

  *As built.* The numeric API was removed rather than counted down; the ratchet reached zero and went (§12,
  step 13). The checker lints placeholder asset names, and the guards refuse numbered folders and `legacy`
  markers outright.

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
  `sword`, `recovery`, `navi_damage`): those were the numbers and the builder arguments. It gains
  `description_lines` (counted from the content's strings, below), `usage: ChipUsage` (`Action(ActionHandle)`, `Dimming`, `Navi`, `Instant`: the hooks are function
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

`nettai-luau` gets `define::run(pack, assets) -> Result<Defined, ContentError>`:

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

`nettai-battle`'s `content::define` turns the tree into typed `Content`: serde deserializes each registry's entries
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
  their stages by handle. The pack's records are `v1/chip-036`, `v1/navi-01`, `v1/form-0c` and (until step 12,
  when stages became definitions alone) `v1/stage-11`, and
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
engine's kinds carry none. A source guard (`crates/nettai-battle/tests/no_compat.rs`) fails on the word `compat`
outside comments in `nettai-battle`, `nettai-content-api` and `nettai-luau`.

**The spike.** A throwaway crate (not committed) ran a define phase on nettai-luau's real sandbox with five modules
(a bombs library with a shared state table, a bomb kind with a module-level effect, MiniBomb and BigBomb
modules, and a FlshBom series module with one shared action for three chips): the verifier accepted the
modules; two VMs, loading the modules in opposite orders, produced identical canonical trees and handles; derived
keys came out as `minibomb/action`, `flshbom1/action`, `lib/bombs/bomb#1`; MiniBomb's action update, fetched from
the second VM, ran with its captured arguments; the three throw actions shared one schema; `define` calls after
loading failed; and a module that wrote a module-level local was still refused. Two refinements came from it and
are in this design: builders put their parameters in `args` (a captured-only parameter table gets a `module#n`
key), and derived keys are claimed only within the defining module (a library's shared state table otherwise took
the first chip's key).

**Partial loading: an unported chip is skipped.** Every chip is a definition with its own use (§4.2), and the
define phase refuses a chip without one. A game being ported (BN5's content/bn5) defines every chip's record
first, from its ROM, and gives each its use as the port writes it; meanwhile its folder must still load. The
loaders (`nettai_content::pack`: the frontend's and the editor's `load_found`, the tools' `load_battle`, which
netplay, the match checks and the verification harness read through) apply one rule to every content folder
before the define phase, `Root::leave_out_unported`:

- a chip whose module (`chips/<key>/chip.luau`) names none of `action`, `dimming`, `navi` and `instant` is left
  out, with every module of its folder;
- so is every chip folder one of whose modules requires a module left out (a Program Advance naming an unported
  ingredient, a chip that borrows an unported chip's module), in turn;
- one warning lists every chip left out (`3 of bn5's chips have no use yet (or need one's module) and are left
  out: bn5:airspin1, ...`).

Anything else that requires a left-out module (a rule section, a library that defines a kind) still stops the
define phase, which says so, and the folder is left out whole with its reason, as any other error leaves it. The
content is then exactly what loaded: its scripts hold none of the modules left out, so the content hash (a round
setup's, netplay's handshake) covers what both peers loaded, and two peers with the same content agree. The rule
names no game: a BN6 chip without a use would be left out the same way (none is).

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
claw, DustCross Beast's scatter), `KindRole` (the absorbed obstacle, the falling rock, the supports' controller, AntiRecv's counterattack; until
step 12 also what an actor list places, which stages name now), `HookRole` (the FirstBarrier, the encased
obstacle) and, since step 12, `LockonRole` (the Beast claw's lock-on mode), `StatusRole` (the statuses the
ruleset inflicts itself), `EffectRole` (the effects it shows itself: a deletion's, a recovery's, the cut-in
flash, a trap's mark, an encased obstacle's, a form change's and Beast Over's four), `SparkRole` (a new
registration's hit spark, a blocked hit's, an eruption's, a thrown obstacle's, an uninstall's), `RegionRole`
(`anchor`, a registration's own panel, which every setup gives it), `CollisionRole` (a navi's body, its
floating body and what it reacts to; an eruption's and a thrown obstacle's types and targets), and the roles
that name assets: `SoundRole` (what the ruleset plays: `Battle::sound(role)`), `MusicRole` (a link battle's,
the winner's two and the loser's), `BannerRole` (a round's start, a turn's, the final turns', a draw, the
judge's, the two telops, the Program Advance's two) and `SpriteRole` (the engine's own kinds' sprites: the
charge glows, the Full Synchro aura, the status visuals, the ice block, the bubble, the hit marker, the
eruption, the lock-on marker, the Beast head, the idle overlay). One role per look
or type the original named by one number, whatever uses it (the deletion effect also shows where a cross merges
and a navi arrives). A role names a definition, or, while its
target is still a v1 registration, that registration through the transitional legacy marker (`{ legacy = {
action = 0x49 } }`, `{ legacy = { kind = "a-v1-kind" } }`; counted by the ratchet); a legacy action number nothing
implements leaves the role `Unported`, and starting it fails as the number did. The charged sword, the beast claw
and the scatter are roles rather than §7.5's traits: the ruleset recognizes exactly one action each, and a v1
registration can't carry traits. The engine's test content has its own roles (testdata/content), which the test
pack replaces.

### 7.5 The ruleset's numeric call sites and their replacements

Counted in `crates/nettai-battle/src` without tests; file names are the modules that hold them.

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

**As built** (step 10). The ruleset has no chip by number, and neither has the engine: a chip is its
definition (`ChipDef`: the record `ChipData`, its use, and `ChipLinks`, what the record names resolved to
handles when the content loads). `ChipData` lost its number, action, subtype, parameter bytes and v1 module;
`Content::chips`, registration by number for chips, `ChipUsage::Unported` and the chip `legacy` marker are
gone. What replaced each number:
- *Traits* (`ChipTraits`, a definition's `traits`): `no_chain` (the Beast rush's chain: the variable swords),
  `aura_bonus` (the AuraHeds and StreamHd), `no_cut_in` (the dimming chips past the Program Advances),
  `element_sword` (SlashCross's charge), `navi_slot` (AntiNavi turns back the chips with the `navi` flag and
  these: Django's, which lack it), `heals` (AntiRecv springs on Roll's chips), and for the navi chip's
  controller, which asked the navi's number, `user_stays` (BigHook: no warp) and `navi_returns_user` (Roll:
  no warp back in).
- *`trap`* (`Trap`): which trap a chip is while a side's defensive-chip record holds it (AntiDmg, AntiSwrd,
  BodyGrd, AntiNavi, AntiRecv); `Battle::linked_trap` asks it.
- *Dark chips*: `dark_substitute` is the substitute chip itself and `hp_bug` what the use adds to the user's HP
  bug.
- *`formula`* (`DamageFormula`): the damage formulas by name, an SP navi chip's with its slot (`Rules::sp_slots`,
  rules/sp-chips.luau) and its damage by deletion-time step.
- *Roles* (`ChipRole`, rules/roles.luau's `chips`): `zeroed` (what a zeroed chip field reads: the original's
  chip 0), `beast_out` and `invalid` (the custom screen's), `rush`, `beat` and `tango` (the chips the supports'
  telops name).
- *Rules*: the Cross special's chips (rules/cross-special.luau: a row by the hundreds of the navi's base max
  HP, a chip of it at random, LifeSrd striking with VarSwrd's damage) and the Program Advances
  (`Defs::program_advances`: every chip's recipes in their order, their ingredients by handle; a chip made by
  one has an index among them, `ChipLinks::advance`, for the round's record of the ones formed).
- *A navi's own chip* is its definition's `own_chip`; the chip knows its navi (`ChipLinks::own_chip_of`), which
  is what the hand, the custom screen and the charge rules asked of chips 0x190 and up.
- *A chip's use* sets no subtype or parameters in the attack: an action takes what it needs from its builder's
  arguments, a hook from its closure. Two things read a chip's bytes besides its own use and now ask the
  definition: SlashCross's charged slash (the action's `charged` and `step`, which SonicBom and SprSonic give
  as their records' bytes read), and the stun strike (a role's action, lib/swords/stun_strike, which strikes
  as the machine sword the attack's chip is).
- The numeric API's chip (a dimming spec's, a defensive-chip record's) is an opaque number, `DEFINED_CHIPS`
  plus the handle; the supports' controller holds its chips as references.
The engine's test chips are definitions too (crates/nettai-battle/testdata/content/chips/test: made-up records
composing BN6's builders), and tests name chips by key. compat's chips.toml has the original's numbers by key,
for the trace harness and save import (`bn6_compat::codec::Ids`); `gen-content check` compares each definition
with the ROM's chip of its number, including the traits, traps and HP bug its number gives it in the
original's routines.

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

**As built** (step 11, §3.2): the ruleset has no navi or form by number. The form constants and ranges are the
form's `kind` and `game`, `with_beast()` its `beast`, the per-form tables and routines its fields and traits
(`palette`, `chip_bonus`, `null_bonus`, `charged_chips`, `charged_bonus`, `charge_doubles`, `chip_heals`,
`fire_charge`, `status_reset`, `navicust_refresh`, `hover`, `special_volley`, `cross_release_anim`; `status_immune`,
`erases`, `charged_sword_rush`, `extra_chips`, `scrap_button`, `special_holds_buster`), and the Cross navi and the
form a weakness hit leaves are `cross_of` and `breaks_to`. `Navi::MEGAMAN` is `NaviData::changes_form` (the navi's
`forms`), the link navis' numbers their `charged_chips`, `charge_doubles`, `fire_charge` and `status_immune`. The
AI-index tables are the identity's `parts`, `overlay_hooks` (one list for the death, refresh, flinch and drag
rows), `aura_anim` and `ice`; the actor keeps its navi's identity through the forms.

**Weapons by number.**
- `idle::weapon_routine` (`Hook::Weapon(n)` and the `nullsub_44` list), `set_charge_shot_routine` (0x21..=0x26
  stick; buster 3 and 4 become 0, 0x2C becomes 0x2B), `idle.rs` `kind = 2` for 0x21..=0x26, `chip_use.rs`
  charged paths (0x18 rock barrage, 0xFF, 0x05 | 0x0D | 0x1F | 0x20 | 0x29 | 0x2D), `form.rs` `24 + form`.
  → the weapon's `setup` slot; traits `sticky`, `demotes`, `charged_chip_kind`, `rock_barrage`,
  `stale_register` (the `nullsub_44` entries, an explicit error as today). *Done (step 11)*: `sticky`, `held`,
  `plain` and `charged_chip` (§3.3, "As built"); the form's init routine by number (`24 + form`) is the form
  traits' batch.

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

*As built (step 12).* All five are roles (§7.4): 38 `sounds`, 4 `music`, 9 `banners`, 13 `sprites`, and the
effects, sparks, regions and collision types of §3.6. `SoundId` and `BannerId` are a pack's ids for its assets,
with no constants: the ruleset has none of either outside its tests. Two things are not roles. The sound a
NaviCust panel trail makes turning a panel into a type (`byte_8013D44`, by panel type) is the panel type's
`trail_sound` in rules/panels.luau. And the original's "no music" song, which the ruleset never plays (a stage
without music has none), is the audio player's (`nettai_audio::NO_MUSIC`): there is no `music.none`.

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

(Gone already with step 12, with their lints: the numeric forms of `battle.play_sound` and `play_sound_for`,
`battle.effect`, `battle.spark`, `battle.region_effects`, `setup_collision` and `reset_collision_types`, the
numeric `collision.region`, `hit_effect` and `status_base`, a hitbox's numeric fields, `lockon_panel`'s mode
number, and `data.regions`, `data.rules.field_regions` and `data.objects.attachments`.)

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
  what registration by number makes, the pack's chip records) reach scripts as frozen stand-ins `{ id = key }`
  (`BindPlan::entries`), so `me.kind` and a navi's `attack_chip` always have a value. An asset is a frozen `{ name = ... }` per asset, one per name, with its kind's
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
  Since step 8e: `navi:action_state(action)` (the state of the action a weapon's setup is about to return),
  `navi.attack_chip` (the attack's chip as its definition, read and write: §5.7) and `navi.rush_lockon` (the
  lock-on mode the attack's own action asks the Beast rush for).
- **Effects, sparks, regions and collision types.** Until step 12 the ruleset stored these as bytes and gave
  what content defined a number of its own (`Defs::number`). *Since step 12* they are handles everywhere: an
  effect object's and a spark's look, `CollisionData.region` and `hit_effect` (`Option`: none is no region, no
  spark), the collision types a setup takes, a hitbox's four. `Content::effect`, `spark`, `region` and
  `collision_type` take handles; there are no numbered tables and no numeric API for them (a number is a type
  error and a runtime error). A collision type carries `row_offset`, the register value its
  row's lookup leaves (index × 8) that a bug code's high byte takes: the quirk materialized in the definition.
  `collision.region` and `collision.hit_effect` read and write definitions or nil (`set_region` and
  `set_hit_effect` are the same as methods); a hitbox's `region` and `hit_effect` may be nil (none), and its
  `target` and `self_type` are collision types. A new registration starts with no region and the plain spark
  (`sparks.plain`, the original's zeroed hit-effect byte), and its setup gives it its own panel
  (`regions.anchor`).
- **Roles.** `define.roles { actions = { ... } }`, once, keyed `roles`. The ruleset starts AntiDmg's, AntiSwrd's
  and BodyGrd's counters by role (`anti_damage_counter`, `anti_sword_counter`, `body_guard_counter`); an
  unfilled role panics naming itself where it is needed and `nettai-content check` warns, until the BN6 content
  fills every role (then an unfilled one is a load error). Their compat keys are the role actions' ids, which
  the chips' conversion chooses (proposed: `antidmg/counter`, `antiswrd/counter`, `bodygrd/counter`, numbers
  0x47, 0x48, 0x4B in actions.toml).
- **Not yet.** Chip, navi, form and stage records stay the pack data's (steps 3b and 5); the definers accept
  their specs, typed loosely (`NaviDef = { id: string, [string]: any }`) until the engine reads them. Statuses
  and lock-on modes likewise (steps 10 to 12). `me.identity` and `me:attach_point_pos` come with identities
  (step 11).

### 7.7 Checking

- **`nettai-content-check`** (the in-process type check, its own binary because its Luau collides with mlua's)
  checks every module against core.d.luau and types.d.luau as today. Library builders' spec types go in
  types.d.luau so chip modules' calls are checked (requires stay typed `any` per module, and a module casts what it
  requires, `require(...) :: BombsLib`). It adds static lints: no deprecated numeric API calls and no `legacy { }`
  markers beyond the ratchet's allowance (§12), no placeholder asset names, kind keys qualified by their owner
  folder, no module under `compat/`.
- **`nettai-content check <content> [<assets>]`** (links the runtime) runs the define phase and reports: duplicate
  keys, references to the wrong registry, unfilled roles, rule sections missing or defined twice, unknown asset
  names, chips with no usage or two, kinds in `objects/` used by one owner (colocation). Compat entries that
  don't resolve and definitions compat doesn't cover are `bn6-compat`'s check, not the engine's (§7.3).
  `cargo test --workspace` runs both on content/bn6.

As built in step 4: the lints and the ratchet read the source through a small scanner (comments dropped, string
contents masked), in `nettai-content-check`'s `lints` module. A deprecated use is a call that exists only in the
numeric API (`battle.spawn_kind`, `me:param`, `data.`, ...), `battle.spawn` with a pool, or a call whose
definition-taking argument is a number literal or a module-level numeric constant (`battle.play_sound(SOUND)`
with `local SOUND = 0x1A6`); it is a count, so an approximate one serves. `tests/deprecated.txt` holds each
module's allowance (1,263 uses in 221 modules at the start); the test fails on more, and on fewer until the
allowance is lowered (`BN6_RATCHET_LOWER=1`); `nettai-content-check --deprecated [--list]` prints the counts.
`nettai-content check` warns on unfilled roles and single-owner kinds (`nettai_content::lint`). The engine's test pack
type-checks against content/bn6's core.d.luau in nettai-content-check's tests.

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
- A small test pack (crates/nettai-battle/testdata/pack, a few modules) tests the define phase itself: keys,
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
  every sprite's animation timing). Compat is not in it: it changes no simulation. Nor is display text: no
  definition holds any; the content root's `locales/<lang>.toml` do (text-rendering.md §10). The own language's
  (`en.toml`) shape what the battle reads (a description's lines, the no-running message's characters per line and
  which move the speaker's mouth), which the define phase counts into the records the hash covers; the strings
  themselves, and the other languages' tables, which only a frontend reads, are out. Pixels, palettes and audio stay
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
graphics' animations with themselves no more (there is one copy). The overlay merge (`nettai_content::overlay`) is
deleted.

### 9.2 The pack: content and assets as two roots

A battle loads from two roots: the **content** (content/bn6, the committed Luau, with compat) and the **assets**
(the extractor's output). `nettai_content::pack::load(content, assets)` reads the modules and the asset index,
runs the define phase, and returns `Content` (plus `Compat` when asked, through `bn6-compat`). The frontend and
the tools take `--content` and `--assets`; the verification workspace points `--content` at the engine
checkout's content/bn6 and `--assets` at its extracted assets, so a content change needs no re-extraction.
A distributable pack can be both in one folder (the same path twice). The pack's TOML battle data, `registries/`,
`rules/*.toml`, `chip.toml`, `object.toml`, `weapon.toml` and `nettai_content::battle` (1,609 lines of TOML IO)
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

**The committed definitions are the source.** Nothing regenerates a module once it is written, and the
generator can't run over a content root that has people's modules at its paths, so a writer whose modules
people have since reshaped is dead code. With step 12 the stages', the lock-on modes' and the statuses'
writers are retired (`gen_stages`, `gen_lockon`, `gen_status`: the definitions lost their legacy markers and
name kinds, variants and each other in forms the generator never wrote): content/bn6/stages/netbattle.luau,
rules/lockon.luau and rules/status.luau are edited by hand, and `gen-content check` compares them with the ROM through compat. `gen-content write` still writes
compat's numbers for them (stages.toml, records.toml, rules.toml), keeping the committed keys.

So are the writers that extended the shared modules (`gen_collision`, `gen_regions`, `gen_effects`) and the
numbered effect, spark and region tables of rules/numbers.luau, which the engine no longer has:
rules/collision.luau, lib/effects.luau, lib/sparks.luau and lib/regions.luau are the committed definitions.
`gen-content check` has its own model of the ROM's tables for them: a collision type must be the row its
`row_offset` names, and every row must be defined; every effect and spark look and every region of the ROM's
must be defined by something; and each role of rules/roles.luau that names one must name the ROM's entry of
the number rules.toml gives the role (the numbers the ruleset's code held until step 12, which the generator
writes from its own list).

*As built (step 13).* The last writers are retired, so gen-content is the check alone: `write` (compat, from
its own lists of the engine's kinds, actions, sounds and roles, which every new kind or role had to be added
to), `luau` (step 5's definitions) and `describe` (the chips' descriptions) are gone, since each would rewrite
what people now own. Compat is edited by hand like the modules, and `gen-content check <rom> <content>` is
what keeps both honest: it reads compat as `bn6-compat` reads it (a number under two keys is refused), checks
compat's numbers against the ROM (chips and their actions, navis, forms, weapons and their aliases, the kinds'
slots, stages, records, assets, curation's entries naming something), and defines the content root as the
engine does and compares every table the definitions make with the ROM's, field by field.

### 9.4 The frontend and the audio

`nettai-assets` keys sprites, backgrounds, mugshots, banners and chip icons by name; `Hud::chip_names` (font codes
by chip id) and `chip_shows_damage` go, the HUD drawing a chip's name from its definition through the font's
charmap and its damage from the `has_damage` flag. The frontend draws an object's sprite by its handle's name,
recognizes the palette flash and the form overlay by `EngineKind`, reads attach points from the navi's identity,
and plays trace files through `bn6-compat`. Its live-play driver picks its hand and navi by key. `nettai-audio`
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

- **Spawn parameters** (`params` in the trace's objects): not compared today, not modeled in v2. Ignored.
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

Every step merges to main with `cargo test --workspace` and `nettai-content-check` green and every floor of §10.2
held; a trace that runs further records its new floor. Sizes: S about half an agent-day, M one to two, L three
to five.

**The ratchet.** From step 3 to step 13 the old and the new coexist: the numeric API, the `legacy { }` marker,
the v1 registration files and the `data` global keep v1 modules working while families convert. A test counts
their uses per module against an allowlist in `nettai-content-check`'s tests that may only shrink; step 13 deletes
the allowlist with the last use. The engine never reads compat, at any step (§7.3): what content defines
reaches the traces and the game's setups through `bn6-compat`, which maps the engine's handles.

**Transitional, and when it goes** (from the exemplars, step 7, and step 9):
- **The roles' legacy markers.** rules/roles.luau names the v1 actions and kinds the ruleset needs by their
  numbers and keys until their families convert them, and then names the definitions (§7.4). The field
  objects' kinds (the rock, the boulder, the Guardian statue, the absorbed obstacle, the falling rock) are
  definitions since step 8f, and the supports' controller since group C5: no kind is named by key any more.
- **The kinds an actor list places, by role** (step 8f): `kinds.rock`, `kinds.boulder` and `kinds.statue`
  stood for the stages' own entries, and the rock's `place` mapped the entry's row number to its variant
  (`rock.by_number`). *Gone with step 12*: a stage's entries name the kind and its variant (§3.7).
- **The legacy marker in a v2 definition.** A chip definition may carry `legacy = { subtype, params }`: the
  original's subtype and parameter bytes in its record, for what reads them of a chip besides its own use.
  Counted by the ratchet (the lint matches `legacy = {` and `legacy {`). StepSwrd, FtrSword, CrosSwrd and
  DblDream carried it for SlashCross's charged slash, which read a chip's subtype and first parameter; since
  step 8e the sword chips' slashes say what the charge needs in a spec field (`charged`, §5.7) and no BN6 chip
  carries the marker (the test pack's tickers do). The marker's `action` and `script` (a behavior still a v1
  module) are step 5's; the reader refuses them until then.
- **The weapon legacy marker.** A weapon definition carried `legacy = { routines = { ... } }`, the routine
  numbers the pack's forms, the navis' rows and the ruleset named it by. *Gone with step 11*: forms and navis
  name weapons by value, the ruleset asks a weapon's traits, and compat's weapons.toml has the numbers.
- **Registration-by-number shims.** chips/036-minibomb (action 0x12), chips/047-sword (0x13) and
  chips/056-mchnswrd (0x49) run a record's action by its subtype for records something still names by number:
  the Cross special's chips (berserk.rs `CROSS_SPECIAL_CHIPS`: MiniBomb, EnergBom, MegEnBom and swords), the
  Program Advance recipes' ingredients (PoisSeed, VDoll, the swords), LilBolr1-3's records (their definitions
  are the chips), the test content's numbered chips. The 0x12 and 0x13 shims go when the Cross special's list and the recipes go by
  handle (phase C, step 10); the 0x49 one when the stun strike (idle.rs `set_attack(0x49)`) is
  `roles.actions.stun_strike`. The standard chips' shims (§5.8: chips/013-yoyo, 01e-thunder, 09a-recov10,
  032-airhocky, 014-firebrn1, 018-trnarrw1, 07b-ironshl1, 034-tornado, 05c-wavearm1) run the records the Program
  Advance recipes, the dark chips and their substitutes, and the Cross special name by number; they go with
  those lists (phase C, step 10) and the records (step 5).
- **A record's action by its module.** A chip record whose `script` names a module that exports `action` (an
  action definition) runs that action as its use, as a chip definition's `action` does, whatever its action
  number names (`record_action` in content/defs.rs; such a module can't also export `update`). The link navis'
  own chips, 0x190 HeatPres to 0x199 DustBrk, need it: their action number 0x0A is below 0x10, an entry of the
  user's own action table (`off_80EA4C8[AIIndex][0xA]`) that registration by number can't claim, and all ten
  share subtype 3, so no shim could pick by subtype; and they stay records for their damage by the navi's
  level (formulas 24 to 44). Each record (chips/19N-<chip>/chip.toml) names its navi's module
  (navis/<navi>/chip.luau), which returns `{ action = define.action { id = "<chip>/action", ... } }`. Which
  link navi has which chip is the navi record's `own_chip`, not the original's table by AI index: any navi that
  holds the chip runs its action. It goes when chip definitions take damage formulas (step 10): each chip
  becomes a `define.chip` beside its action and its navi's definition names it as `own_chip` (step 5's navis).
  *Since step 5* each is a definition already (navis/<navi>/record.luau, which the navi's `own_chip` names),
  numbered by its legacy marker, whose `script` names the action's module and whose `damage` and `navi_damage`
  give the formula; the same resolution applies to it (a numbered definition without a use of its own runs the
  action its marker's module exports). Folding the action into the definition (`action = ...`, no `script`)
  retires the mechanism for that chip.
- **Chips kept on records** (their modules give only the action, with the compat key as `id`): besides the
  above, what 3b's `chip_record` refuses in a definition: `program_advances` (LifeSrd, GreatYo, PitHocky,
  WideBrn, ParaShl, PwrWave), `dark_substitute` (DrkSword, DarkThnd, DrkRecov) and damage formulas (Muramasa, ProtoMan's StepSwrd). The reader learns them (step 5 needs them
  anyway), and those chips become definitions.
- **The instant chips' record shims** (step 8d): chips/0c0-atk-10 (subtype 3: MegaBstr, chip 0, which a zeroed
  chip field reads; Atk+10's record, the dark chips' fifth substitute; Atk+30 and Uninstll, SunMoon's
  ingredients; DarkPlus, a dark chip), chips/06b-firehit1 (subtype 8: FireHit1-3, Program Advance ingredients)
  and chips/13f-beastout (subtype 0: BeastOut and the invalid chip, which the custom screen names by number).
  They go when the zeroed chip, the substitutes, the dark chips, the custom screen's chips and the recipes
  name chips by handle (phase C, step 10).
- **The standard chips' shims** (step 8g, §5.8): chips/03f-aquandl3 (0x32), 152-h-burst (0x34), 029-rlnglog2
  (0x36), 010-gundels2 (0x37), 027-rskyhny3 (0x39), 021-dolthdr3 (0x3E), 054-moonbld (0x40), 022-elcpuls1
  (0x42), 05f-aurahed1 (0x43), 052-varswrd (0x53) and 053-neovari (0x54) run a record's action for the chips kept
  on records (Program Advances and their ingredients, MoonBld and the variable swords) and for the Cross
  special's chips. They go as the 0x12 and 0x13 shims do (step 5 for the recipes, step 10 for the Cross
  special's list, the aura chips and the Beast rush's chain; SlashCross's A-charge with family 8e).
- **The navi chips' shims** (§5.5): one module a series runs the records' navi chip subtype and gives the
  navi's `summon` the record's parameters: chips/0ec-eraseman, 0fb-grndman, 0fe-dustman, 104-diveman,
  107-crcusman, 10a-judgeman, 12e-bighook, 159-darkness, 15a-mstrcros, 15c-twinldrs, 15d-crosover (A2) and
  chips/0dd-roll, 0e0-protoman, 0e3-heatman, 0e6-elecman, 0e9-slashman, 0ef-chrgeman, 0f2-spoutman, 0f5-tmhkman,
  0f8-tenguman, 101-blastman, 10d-elmntman, 110-colonel, 12d-bass, 12f-deltaray, 132-bassanly, 15b-sunmoon
  (C4). They go when AntiNavi's test is a trait, the SP damage formulas are in definitions and the Program
  Advance table names its results by handle (step 10).
- **The dimming chips' record shims** (group C5, §5.4): chips/0ba-antinavi (subtype 20: the trap chips the
  ruleset names by number, and BodyGrd), 090-timebom1 (10: TimeBom+'s recipes and their ingredients),
  0b1-invisibl (1: Invisibl, a dark chip's substitute; the second WhiCapsl, past 0x170) and 121-darkinvs (38:
  DarkInvs, a dark chip). They go when the ruleset names those chips by trait and the recipes by handle (phase
  C, step 10). Rush's spared chip (lib/supports/rush: the second WhiCapsl's number) goes with them.
- **The charged slashes by row.** `slashes.by_row` (navis/megaman/forms/slashcross/slashes) keeps the
  charged slashes by the original's row, a sword chip's subtype, and SlashCross's A-charge
  (navis/megaman/weapons/slash-a-charge) the variable swords' and MoonBld's action numbers, for a charged
  chip that is still a pack record (counted by the ratchet: the attack's variant and first parameter). They go
  when the swords SlashCross charges are definitions (step 5 for their records; their slashes already name
  their charged slash).
- **The variable swords' picks by number.** `VariSwordSpec.legacy = { chips, sword }` (chips/varswrd,
  chips/neovari; counted by the ratchet): the chip numbers of the picks, in the picks' order, and Sword's. A
  pick starts its action by definition, but the attack's chip is still the pick's record (lib/swords/vari
  reads its Beast flag from `data.chips`): the Beast rush reads the attack's chip for its lock-on mode. A
  charged pick's slash is its action's (`charged`, since step 8e), so the record's subtype and parameters are
  no longer copied. It goes when every pick is a chip definition the spec can name (LifeSrd, Sword, LongSwrd,
  WideSwrd and the sonic booms are still records: step 5): `become` then sets `me.attack_chip`, which exists
  for it.

  *Since step 5* they are definitions under their compat keys (the generated `record.luau` beside the
  action's module), numbered by their legacy marker: `program_advances` names its ingredients by value, and
  the marker carries the number with what the ruleset still reads by it (a damage formula as `damage = 1000 +
  n` with its `sp_damage` or `navi_damage` table, `dark_substitute`, the subtype and parameters). Such a
  definition's behavior is the v1 module its marker names (`script`: the shim, which runs the chip's action
  by subtype) or, once its module is folded into it, its own: a numbered definition may carry its own
  `action`, `dimming`, `navi` or `instant` (then without `script`), and what names the chip by number
  (recipes, the Cross special, a navi's own chip) reaches that definition. So a record chip converts by moving
  its action into the generated definition and dropping `script`; the number stays in the marker until phase
  C's step 10.

*Gone with step 10*, of the above: the chip legacy marker (a chip takes none), every registration-by-number
shim of chips (the 0x12, 0x13 and 0x49 ones, the standard chips', the navi chips', the dimming and instant
chips' record shims), "a record's action by its module" (`record_action`), the chips kept on records and
their numbered definitions (each is a definition with its own use; a `record.luau` beside people's module
was merged into it), chips/v1.luau, the charged slashes by row and the A-charge's by-number path, and the
variable swords' picks by number (`VariSwordSpec.picks` are chips; `become` sets `me.attack_chip`). The stun
strike is a role's action (lib/swords/stun_strike). Rush's spared chip is the definition.

### Phase A: foundations (the model-v2 agent; steps 1 and 2 can run in parallel)

1. **Compat and the generator** (verification workspace, then this repository). gen-content with the ROM
   decoders copied from bn6-extract; `write` emits compat/*.toml with keys, assets.toml with names, text.toml;
   `check` compares against the ROM. Commit compat/ here. No engine change. **M.**
2. **`bn6-compat` and the trace move.** The new crate; trace.rs and the setup codecs move into it; the harness,
   the frontend's trace playback, netplay's `rollback_cost` and the verification workspace switch to it. The
   codecs still produce today's numbers. **M.**
3. **The define phase and handles.** `nettai_luau::define`, `nettai_battle::content::define`, `Content` with registries
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
   `nettai-content check` and the new lints. `define.roles` (§7.4) with the roles the ruleset starts content by
   already: the trap chips' counter actions (AntiDmg's, AntiSwrd's and BodyGrd's, 0x47, 0x48 and 0x4B, which no
   chip record names), with their keys in compat actions.toml. **M.**
5. **Data to Luau.** gen-content writes every chip, navi, form, weapon, rule section, stage and registry entry
   as v2 definitions in the v2 folders (§4), with `legacy { action, subtype, params, script }` markers where a
   chip's behavior is still a v1 module; the pack's TOML battle data and bn6-extract's battle.rs go;
   `nettai_content::battle` goes; the loader takes the content and assets roots. Needs step 3b. Gate: the `Content`
   the definitions build equals the one v1 extracted (a one-off field-by-field check, as in the v1 move), `gen-content check`
   passes, the traces hold. Mostly generated. **L.**

   *As built.* `gen-content luau <rom> <content>` writes into the content root, leaving what people defined
   alone (a chip whose `define.chip` has no legacy marker keeps its module; its numbered record goes to
   chips/v1.luau as `v1/<key>`, which numbers reach) and extending the shared modules by what they lack
   (rules/collision.luau by `row_offset`, lib/effects, lib/sparks and lib/regions by look and shape); a module
   people wrote at a generated path (the sword family's action modules) gets its definitions in a
   `record.luau`/`records.luau` beside it. It deletes the 253 `chip.toml` and 46 `weapon.toml`, whose content
   is in the markers, and lists its made-up names in compat/curation.toml by module. The markers are one
   field, `legacy` (step 7's, which gives a converted chip its subtype and parameter bytes and a converted
   weapon its routine numbers): a chip still a v1 module has no `action` and its marker gives `number, action,
   subtype, params, script` (with what the ruleset and v1 modules read of it by number: a damage formula,
   `sp_damage`, `navi_damage`, `dark_substitute`, `recovery`, `sword`); a weapon still a v1
   module has no `setup` and its marker gives `routines, script, action, instant_chip`; a navi's and a form's
   give `number, name_id` (until step 11), a stage's `number, layout, actor_list` (until step 12: compat has them), (until step 12) a status's and a lock-on mode's `id`. The
   generator writes them with the `legacy { }` call (identity; typed `any`), which is how it tells its own
   definitions from people's. The tables v1 modules read by number are legacy rule
   sections (`define.rules(section, legacy { [n] = ... })`): rules/numbers.luau (until steps 11 and 12: the effects, sparks and regions by number, and
   the charge times of the routines no weapon names), rules/identities.luau (until step 11: identities are
   definitions, §3.2), rules/body-overlays.luau (until step 11: the identities' `parts`) and a
   kind's objects/KIND/rows.luau while something still reads its table by number (`data.objects.<table>` in a
   module, or the engine: the body overlays' is the last; the attachments', the rocks', the absorbed obstacles',
   the sun beam's, the projectiles', the flying shots', the boomerangs' and the sword and shock waves' went with
   their readers, and GunDelSol's data is its chips' own). nettai-battle's `content::legacy` builds the v1 tables from all of it. Weapons
   are a routine's numbers with the same address *and* charge times (alias routines whose rows differ are
   weapons of their own: `megaman/buster` is routine 0 alone, and `megaman/buster-2e` and five more take its
   `setup` with their own charge times), and every routine has its charge times (the TOML's
   rules/weapons.toml had 50 of the 148; a routine a navi's or form's stats name and nothing implements, like
   ProtoMan's 0x32, now charges as the game does). Content may not use a placeholder asset
   name, so compat names what the tables use for its first user (`effect-0e`, `held-28`, since curated as `hit-damage-judge`), for curation. The loader is
   `nettai_content::pack::load_battle(content, assets)`; bn6-extract writes assets only. The check: `gen-content
   check` defines the content root and compares every table with the ROM's (§3 of content-pack.md).
   A chip's `description` (what R shows on the custom screen: the battle reads its line count) and a navi's
   `run_message` (the no-running message's lines) were the definitions' alone once the extractor's battle data
   went (`gen-content describe` gave the chips people had defined without a description theirs, once). Since the
   languages (2026-10-02, text-rendering.md §10) every name, description and message is the content root's
   `locales/en.toml` instead, by key, and the define phase counts their shape into the records; gen-content checks
   the strings against the ROM.
   Until step 12 the engine had a byte for an effect, spark, region or collision type content defines
   (`Defs::number`): the numbered table's entry that is the same thing, and only another got a number after
   the table's. Step 12 made them handles and removed the numbered tables.
   Registration by number resolves a numbered definition's use as step 9 does a record's (its action's
   registration, or its subtype's `dimming_chip`, `navi_chip` or `instant_chip`; `Unported` for what nothing
   implements), unless the definition has its own. While step 5 was a branch, its content was made again on
   each main it merged (the verification workspace's `tools/regen-step5.sh`: main's content with its v1
   files, the branch's edits of people's files, then the generator), so chips, kinds and shared entries main
   had gained by hand were skipped; on main the generated modules are people's and nothing regenerates them.
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
    and traits. **M.** Done (§7.5, "As built"): every chip is a definition with its own use, the 72 numbered
    shim folders, chips/v1.luau, `record.luau`/`records.luau` and the chips' legacy markers are gone, and the
    engine has no chip by number.
11. **Navis, forms, identities and weapons**: `NaviStats` on handles, the form and navi traits, identities for
    NameIDs, weapon traits. **L.** Done: weapons by handle (§3.3, "As built"), identities (§3.2, "As built"),
    and navis and forms by handle with their traits (§3.2, "As built (step 11, forms and navis)"): the engine
    has no `Form` or `Navi` number, the 37 navi and form `legacy { number, name_id }` markers and
    rules/body-overlays.luau are gone, and the tables by AI index are the identities'.
12. **Assets and stages**: sounds, music, banners, effects, sparks, collision types and regions through roles;
    stages on handles. **M.**

    *As built.* Stages place kinds and variants from their definitions (§3.7); lock-on modes, statuses,
    effects, sparks, regions and collision types are handles (§3.6); what the ruleset names of them, and the
    sounds, music, banners and sprites it plays and shows, are roles (§7.4, §7.5). The engine's kinds' sprites
    went with the rest of the assets. Compat's rules.toml holds the original's number for each numbered
    definition and each role, for `gen-content check` alone (§6.1, §9.3). The body overlays' rows
    (rules/body-overlays.luau), which the ruleset named per form and per navi, went with step 11: they are the
    identities' `parts`, which name their sprites (so the sprite roles `beast_head` and `idle_overlay` went too).

Phase C packets touch the ruleset and `core_api.rs`; the content API they expose is step 4's, so phase B isn't
disturbed. Each deletes registration by number's use for its category.

### Phase D: the end of registration by number

13. **Remove the old**: the numeric API, `legacy { }`, v1 registration (and with it `KindDef::slot`,
    `ActionDef::number`, `CONTENT_ACTION`'s byte), the `data` global, the ratchet's allowlist. Rewrite content-pack.md, scripting.md,
    content-migration.md and the engine docs' references to the pack's files; memory and brief updates. **M.**

    *As built (part 1).* Gone, each with no user left: v1 registration (`object.toml`, module exports, a
    module's `state` as a schema, `KindDef::slot`, spawns by pool and index or by name, the object's spawn
    parameters for content kinds, `me.index`); actions by number (`ActionDef::number`,
    `NaviAction::Unported`/`numbered`/`number`, `CONTENT_ACTION`, `set_attack(number)`, a weapon's setup
    returning a number; a navi's action byte is an error, `navi_action()` and `set_attack` are how a navi's
    action is read and started); the attack's variant and parameter bytes and the hook specs' `params` (the
    trap counters' variant, which both of their starters leave 0, is the counter's builder argument); chips
    as opaque numbers (`me.chip`, `hand_chip`, the dimming spec's, the linked record's and the dimming
    service's chips are chip definitions); sprites by `"CC-II"` string; the roles' legacy forms (the turn, the
    original's action 0x3B, is ported as `megaman/turn`; the volley, whose request nothing raises, is a role
    nothing fills, as are the two objects battle mode 9 spawns by number: starting one is an error naming the
    role); the `data` global, with step 12's tables its last
    readers (the buster's recovery and the sine table are read from their rule modules); the object data
    tables and rule sections nothing defined or read; `tools/content-dump`; in the verification workspace,
    gen-content's last writers (`write`, `luau`, `describe`), so that it is the check alone and compat is
    edited by hand (§9.3). A weapon without a `setup` or a `charged_chip` is a content error at load, not a
    panic when it is used.
    New checks: `nettai-content-check` requires a type on a module-level table constant passed to a function (an
    unsealed literal passes for any record, and a required module is `any`: the 64 it flagged are annotated,
    with eight spec types moved to types.d.luau); `nettai-content check` refuses two collision types on one row
    of the original's table (four rows were defined twice); reading compat refuses two keys with one number
    (actions may share one). Guards (`nettai-content-check`'s `guards` test): no folder named with an original
    number; no `legacy` marker outside the listed modules. The navis' and forms' numbers and the body
    overlays' numbering (38 markers, all the ratchet counted) went with step 11's last batch, which emptied the
    ratchet and the guards' list; what is left for the second part is `legacy` itself, the ratchet's allowlist
    and the guards' exceptions.

    *As built (part 2).* `legacy` is gone: the global, its declaration in core.d.luau, and the chip definition's
    own refusal, replaced by the define phase refusing a `legacy` field on any definition. legacy.rs, whose
    last contents were the rule sections and the reader that turns a definition's values into typed data,
    became content/sections.rs and content/reader.rs (`SpecReader`: assets as the engine identifies them, a
    lock-on mode as its handle, a chip as its key; nothing reads as an original number). The ratchet is gone
    with its count at zero: `tests/ratchet.rs`, `tests/deprecated.txt`, the deprecated-use scanner and
    `nettai-content-check --deprecated`. The guards have no exception lists: no folder under the content roots
    named with an original number, no `legacy` marker or field in any module (the `.d.luau` files included);
    the `no_compat` source guard needed none. Two last original numbers left the engine for the tools:
    `Pool::type_number` (the `T1`/`T3`/`T4` the traces print) is bn6-compat's `pool_type`, and
    `ChipFamily::from_number` (a ROM record's family byte) the ROM decoder's in gen-content. Measured after the
    step, as §8 asks: see §14.

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

## 14. Status

**The migration is complete** (step 13, 2026-10-01). Game data is committed Luau in content/bn6; content names
content by its definitions and assets by name; the extractor yields only assets. The engine runs on handles: it
has no chip, kind, action, weapon, navi, form, stage, identity, effect, spark, region, collision type, status or
lock-on mode by one of the original's numbers, and nothing carries those numbers into it. The guards keep it
so, with no exceptions: nettai-content-check's `guards` (no folder named with an original number, no `legacy`
marker or field) and nettai-battle's `no_compat` (compat appears in the engine's code only in comments). The
golden traces, their sound calls and every recorded chip-lab scenario match every frame, as before the
migration began.

The open questions (§13) went as recommended: two roots; the repository's tests run BN6's own modules on
made-up assets (content-migration.md §5.1); the generated names were accepted and are curated as people get to
them (compat/curation.toml is the review list); animation numbers stay numbers.

**What stays numbered, on purpose:**

- **Animation numbers within a sprite** (§13, question 4): an animation is an index into its sprite, a named
  constant in the module that plays it.
- **Compat** (content/bn6/compat, §6): the original's numbers by key, read only by the tools outside the engine:
  bn6-compat's trace harness and setup codecs (save, folder and link-data import), the extractor's asset names
  and the verification workspace's `gen-content check`, which keeps every number in it the ROM's. It is edited
  by hand (§9.3).
- **The trace harness** (bn6-compat's `trace`, §10): it compares the engine with the original frame by frame,
  so it maps the engine's identities to the original's numbers (a kind to its object slot, a navi's action to
  its action number, a pool to its type).

Numbers that remain for other reasons, and are not names of content:

- **The assets' own identities.** The pack identifies a sprite by its category and index and a sound by its
  song-table entry, as the extractor wrote them; content and the ruleset name assets by name or by role, and
  the pack's asset index (assets.toml) maps the names. Since rules-in-luau.md's R3a the engine knows an asset by
  its handle over the loaded packs' names alone (`SpriteId`, `SoundId`, ... are handles); a pack's own numbers are
  read at the edges (the frontend, the audio, compat).
- **The game's values**: tick counts, damage, a flags word compared whole, a chip's library number, a stage's
  battle number; a hit's bug code, which names a NaviStats byte by its offset as the game's does (bug codes have
  no definition); and the engine's own progress numbers (an object's state, action and phase, the navi
  framework's states), which the traces compare as the original numbers them.

**Since:** a game being ported loads without its chips that have no use yet (§7.3, "Partial loading"), so
BN5's content plays beside BN6's while its port goes on.

**Left to others:**

- The frontend's emotion window finds a face in hud.json by compat's form and navi numbers
  (crates/nettai-render/src/hud.rs). Reading the definitions' `mugshot` instead is the presentation work's.
- Roles nothing fills: `actions.volley`, `kinds.mode9_attack` and `kinds.mode9_actor` (content-migration.md
  §6). No netbattle reaches them.

**Cost after the step** (§8): `rollback_cost` on soundmod round 1 (frames 10164 to 12164, up to 27 objects), the
worst case of a 10-frame rollback every rendered frame: a restore 4.4 µs, an advance 5.6 µs, a save 2.6 µs, the
digest 27.0 µs, 124 µs per rendered frame (0.7% of a 16,667 µs frame), against the 135 µs scripting.md §6.2
records. No regression; the machine was under heavy load, which only the tail (99th percentile 0.9 ms) shows.
