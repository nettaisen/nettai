# Content packs and the content root

A battle runs on **content**: chips, navis and their forms, MegaMan's
weapons, stages, the ruleset's tables, object kinds, effects, sparks and
regions, every sprite's animation timing, and the Luau code that runs
them. The engine holds it as one typed value, `bn6_battle::Content`, and
reads it from nowhere else: no ROM, no tables or scripts compiled into the
engine. It comes from two places:

- the **content root**, a checkout's folder of Luau modules that *define*
  the content (BN6's is content/bn6 in this repository, written from the
  ROM by the verification workspace's `gen-content luau` and owned by
  people since: docs/design/content-model-v2.md §4, §12 step 5);
- a **content pack**, a folder of open-format assets that `bn6-extract
  content` writes from the user's ROM: graphics (with the sprites'
  animation timing) and sound, each under its name, and the asset index
  that lists them. The definitions name the pack's assets
  (`asset.sprite("bomb")`), never their numbers.

This document describes the two and how they load, and the `Content` API
the engine and other layers use. The pack's graphics and sound formats are
in [asset-formats.md](asset-formats.md); the definitions and their API in
[content-model-v2.md](content-model-v2.md) and [scripting.md](scripting.md).

## 0. Summary

- **One extraction, of assets.** `bn6-extract content <rom> <pack>` writes
  the graphics, the sound and the asset index, reads the graphics and the
  index back to check them, and defines the content root against the pack
  to check its names resolve. The engine, the frontend, the audio, netplay
  and the verification workspace all load the content root with a pack.
- **Content is definitions.** The chips, navis, forms, weapons, stages,
  rule sections, collision types, statuses, lock-on modes, effects, sparks
  and regions are `define.*` calls in the content root's modules. What
  only registration by number still reads (a navi's or a form's number, a
  weapon's routine numbers, the original's numbering of a table) sits in a
  definition's `legacy` marker, which goes when its family converts
  (content-model-v2.md §12). A chip has none: it is its definition, with
  its own use.
- **Exact.** The tables the definitions build equal the ROM's, field by
  field (`gen-content check`, §3), and every golden trace, the sound calls
  and the chip lab hold.
- **Shared and immutable.** A battle holds its content in an
  `Arc<Content>`: snapshots share it and the state digest leaves it out.
  Its identity, `Content::hash()`, is part of the round's setup
  (`RoundSetup::content`), so netplay peers can check they run the same
  content.

## 1. The content root

```text
chips/KEY/chip.luau, chips.luau           a chip or a series (`define.chip`), with its use
chips/KEY/*.luau                          what only that chip or series uses (its action's builder, its kinds)
navis/KEY/navi.luau, chip.luau, *.luau    a navi, its own chip, its weapons
navis/megaman/navi.luau                   MegaMan
navis/megaman/forms/KEY/form.luau         MegaMan's forms, with their weapons next to them
navis/megaman/weapons/KEY/weapon.luau     MegaMan's weapons (`define.weapon`)
navis/megaman/weapons/NN-name/*.luau      v1 modules a weapon's legacy marker names (until step 6)
objects/KIND/object.toml, *.luau          `[kind]`: the object kind a v1 module implements; its module
stages/netbattle.luau                     the stages (`define.stage`), with their layouts and actors
rules/*.luau                              rule sections (`define.rules`), collision types, statuses,
                                          lock-on modes, the roles (roles.luau)
lib/*.luau                                helpers, and the shared effects, sparks and regions
core.d.luau, types.d.luau                 the API's definitions (for editors and the checker)
compat/*.toml                             the original's numbers by key: tools' data, never the engine's
```

A chip's definition holds its `description` (what R shows on the custom
screen: the battle reads its line count, and none given counts as three)
and a navi's its `run_message` (the no-running message's lines, in
characters); the definitions are their only source.

`bn6_content::root::read(dir)` reads one: every module (by path without
`.luau`) and the object kinds' `object.toml`. `bn6_content::root::bn6()`
is BN6's: `$BN6_CONTENT`, else this repository's content/bn6.

## 2. The pack

```text
content.toml                              the manifest ([graphics], [sound])
assets.toml                               the asset index: every asset content can name, by kind and name
graphics/sprites/NAME/                    a sprite: parts, layouts, animation timing (animations.json)
graphics/...  sound/...                   see asset-formats.md
```

The index lists every sprite, sound, banner, background and mugshot by
name, with the engine's identity for it (a sprite as `"category-index"`
in hex, the others their numbers): every name compat/assets.toml gives,
and the pack's other assets under their placeholders (`sprite-0c-2d`),
which content may not use (§6.3 of content-model-v2.md; `bn6-content
check` flags one). The pack holds the game's own data: it is written
outside version control (data/content/ is ignored).

## 3. Loading

`bn6_content::pack::load_battle(content, pack)` reads the content root
and, from the pack, the asset index (`Content::assets`) and the sprites'
timing (`Content::animations`), then runs the define phase
(`Content::define`): every module once, what they define into the
registries, and, from the definitions, the tables registration by number
reads (bn6-battle's `content::legacy`: the navis and forms by
number, the stages' panel layouts and actor
lists, the rule sections, collision types by row, statuses, lock-on modes,
effects, sparks, regions and the object kinds' rows). It reports, by
module, a definition that doesn't read (a missing field, a gap in a
numbered table, two chips claiming one number) and a name the pack's
index doesn't have. `bn6_content::pack::battle_content` is the same before
the define phase.

`bn6-content check <pack> [--content DIR]` runs every import and the
define phase with its lints. The verification workspace's `gen-content
check <rom> <content>` defines the content root with compat's asset names
and compares every table it builds with the ROM's, field by field: chips,
navis, forms, the weapons by the routine numbers compat gives them (their
charge times, the traits the ruleset asks, the forms' and navis' weapon
slots, the navis' fresh stats), the stages with their layouts and actors,
every rule section, the registries and the object kinds' rows.

Loading is straight from the files: there is no derived cache. A frontend
that doesn't play sound (headless rendering, `--mute`) skips the sound.

## 4. The `Content` API

`Content` (crates/bn6-battle/src/content) is plain data: public fields,
`Clone`, `PartialEq`, `Hash`. Engine code reads it through the battle:

```rust
let content: Arc<Content> = Arc::new(bn6_content::pack::load_battle(&bn6_content::root::bn6(), pack)?.0);
let setup = RoundSetup { content: content.hash(), settings, navi_stats, ... };
let mut b = Battle::new(setup, content.clone());

let chip = b.content.chip(handle);               // &ChipData
let gds = chip.gun_del_sol.unwrap();              // this chip's GunDelSol data
let navi = b.content.navi(stats.navi);            // &NaviData (banners, move lag, ...)
let form = b.content.form(stats.form);            // &FormData (sprite, weapons, ...)
let rec = b.content.navi_record(name_id);         // actor record of a player NameID
let p = b.content.attach_point(name_id, 3);
let frames = b.content.animation(sprite, anim);   // &[AnimFrame]
let e = b.content.effect(0x03);                   // EffectSprite
let region = b.content.region(4);                 // &[PanelOffset]
let t = b.content.rules.collision_type(1, side);
let settings = b.content.rules.stages.settings(0x11);
let pas = b.content.program_advances();          // in the order they are tried
```

- `Battle::new(setup, content)` panics if `setup.content` isn't
  `content.hash()`: a round can't silently run on other content than its
  setup names.
- Code without a `Battle` gets what it needs passed in: the sprite
  stepper takes the content (`sprite.set_animation(anim, &b.content)`,
  `sprite.update(&b.content)`), the field takes it to lay out and refresh
  panels, the custom screen reads a `custom::Library`, which `Content`
  implements.
- Where a function needs `&mut Battle` while holding content data,
  clone the `Arc` first (`let content = b.content.clone();`): it costs an
  atomic increment.
- Trace replays (bn6-compat's `trace`) take the content and compat:
  `trace::run_round(round, &content, &compat)`, `round.start(content, &compat)`,
  `round.round_setup(&content, &compat)`, `round.tick_inputs(i, &frames, &ids)`.
- The setup codecs (`bn6_compat::codec`) take an `Ids` (the content and
  compat), which maps the records' numbers to the engine's handles and
  back. `codec::battle_settings(bytes, &ids)` finds the stage the record
  is (its actor list by original address, `StageSettings::actors` an
  `ActorListId` into `content.rules.stages.actor_lists`); a round's
  `BattleSettings` is the stage's handle with the background and effects.
- The scripts run from the content: `Battle::new` loads them with
  `Behaviors::for_content(&content)` (once per thread and content hash;
  the VM is a cache, not battle state), and they read the content as a
  frozen `data` global built from it (`data.rules.buster_recovery`,
  `data.rules.sine`...), field names as in the files.

## 5. Identity, snapshots and netplay

- `Content::hash()` is a stable 64-bit hash of all of it, scripts
  included (the state digest's hasher over the typed data and the
  modules' source), so it doesn't depend on how the files are formatted.
  It changes with any edit to the data or a script and with the engine's
  data layout.
- `RoundSetup::content` carries it. The setup is part of the state
  digest, so two peers whose digests agree run the same content; a netplay
  session should compare setups (hence the hash) before the match.
- `Battle::content` is an `Arc<Content>`: cloning a battle (a snapshot)
  shares it, and the digest leaves it out. No state refers to content by
  reference any more: the actor list is an id, a rock keeps its variant.

## 6. Tests and other content

In-repo tests never load game data. `bn6_battle::content::testing` (the
`test-content` feature, and the engine's own tests) is a small content set
written by hand: made-up chips on GunDelSol's, the dimming and grab
chips' and the eraser navi chip's actions, one navi and its base form
with the buster weapons, rocks, a custom-screen layout, and collision
types and panel rules written from the engine's own flag semantics. Its
scripts are content/bn6's modules, read from the repository and
registered by the test content's own records, so the tests run the real
scripts on made-up data (the asset names they use resolve to made-up
assets). The engine, netplay, audio and frontend tests run on it; the
define-phase test defines all of content/bn6. Tests about BN6's actual
data (effect lifetimes, GunDelSol's 480 HP in the sun, the soundmod
stage's rocks, the golden traces) live in the verification workspace,
which loads content/bn6 with the BN6 pack it extracts to
`data/content/bn6`.

## 7. The move from compiled tables

The engine used to compile generated Rust tables. Before they were
deleted, a one-off check loaded the BN6 pack and compared every table
field by field with them: 5,841 checks over 67 tables (every chip record,
battle settings and actor lists, collision types, regions, weakness,
effects, sparks, panel layouts and rules, navi, form and NameID tables,
statuses, charge and recovery tables, rocks, overlays, attachments,
GunDelSol's tables through each GunDelSol chip, lock-on searches, the
animation timing of all 298 sprites, and the custom screen's layout,
Program Advances, link navis' chips and modifiers), with no mismatch. Three
table tails are not in the pack because no input can reach them: move-lag
rows for navi numbers 12 to 22 (a navi number past 11 has no sprite,
banners or buster bonus, so it never spawns), the 13th merge height, and
the secondary elements of chip families 13 to 15 (no chip has one).

With the pack, the golden traces match exactly as before (machgun
1074/1074 and 1331/1331, soundmod 4513/6284/2566), also under rollback at
every tested latency, the sound calls match the original's, and the
frontend's frames are pixel-identical to those rendered before the switch
(all 2,405 frames of the machgun trace).

## 8. The move to definitions

The pack's TOML battle data (chips, navis, forms, weapons, rules,
registries; 479 files) and the extractor that wrote it went with
content-model-v2.md §12's step 5: `gen-content luau` wrote the same data
as definitions into content/bn6, and `gen-content check` compares what
they build with the ROM, field by field, as §7's check did for the
compiled tables. The 56 chip records with no name (`????`) that nothing
reaches have no definition. A weapon holds its own charge times (the
TOML's `rules/weapons.toml` had 50 routines' rows); since step 11 a
routine no weapon names (70 of the original's 148) has nothing in the
content: nothing names a weapon by number.
