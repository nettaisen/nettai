# Content packs and the content root

A battle runs on **content**: chips, navis and their forms, weapons, stages, the ruleset's tables, object kinds,
actions, effects, sparks, regions, collision types, statuses, lock-on modes and identities, every sprite's
animation timing, and the Luau code that runs them. The engine holds it as one typed value,
`nettai_battle::Content`, and reads it from nowhere else: no ROM, no tables or scripts compiled into the engine. It
comes from two places:

- the **content root**, a checkout's folder of Luau modules that *define* the content. BN6's is content/bn6 in
  this repository: people own it, and the verification workspace checks it against the ROM (§3);
- a **content pack**, a folder of open-format assets that `bn6-extract content` writes from the user's ROM:
  graphics (with the sprites' animation timing) and sound, each under its name, and the asset index that lists
  them. The definitions name the pack's assets (`asset.sprite("bomb")`), never their numbers.

This document describes the two and how they load, and the `Content` API the engine and other layers use. The
pack's graphics and sound formats are in [asset-formats.md](asset-formats.md); what a definition is and how to
write one in [content-migration.md](content-migration.md); the runtime in [scripting.md](scripting.md); the design
record, with the reasons and the as-built notes, in [content-model-v2.md](content-model-v2.md).

## 0. Summary

- **One extraction, of assets.** `bn6-extract content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> <pack>` (the four ROMs:
  US Falzar, US Gregar, JP Falzar, JP Gregar) writes the graphics, the sound and the asset
  index, reads the graphics and the index back to check them, and defines the content root against the pack to
  check its names resolve. The engine, the frontend, the audio, netplay and the verification workspace all load
  the content root with a pack.
- **Content is definitions.** Everything in the content root is a `define.*` call in a module, keyed by name.
  The engine has no chip, weapon, kind, action, effect, spark, region, collision type, status, lock-on mode or
  identity by number, and no navi or form: it holds handles, and content passes definitions.
- **The original's numbers are compat's.** content/bn6/compat maps keys to the original's numbers for the tools
  that need them (the trace harness, save and link-data codecs, the extractor's asset names, `gen-content
  check`). The engine never reads it (a test guards the dependency), and content can't load it.
- **Exact.** What the definitions build equals the ROM's data, field by field (`gen-content check`), and every
  golden trace, the sound calls and the chip lab hold.
- **Shared and immutable.** A battle holds its content in an `Arc<Content>`: snapshots share it and the state
  digest leaves it out. Its identity, `Content::hash()`, is part of the round's setup (`RoundSetup::content`), so
  netplay peers can check they run the same content.

## 1. The content root

```text
chips/KEY/chip.luau, chips.luau           a chip or a series (`define.chip`), with its use
chips/KEY/*.luau                          what only that chip or series uses (its action's builder, its kinds)
navis/KEY/navi.luau, chip.luau, *.luau    a navi, its own chip, its weapons and what they spawn
navis/megaman/navi.luau                   MegaMan
navis/megaman/forms/KEY/form.luau         MegaMan's forms, with their weapons and kinds next to them
navis/megaman/weapons/KEY/weapon.luau     the weapons several forms share (`define.weapon`)
objects/KIND/*.luau                       object kinds several owners spawn (`define.kind`)
stages/netbattle.luau                     the stages (`define.stage`), with their layouts and actors
rules/*.luau                              rule sections (`define.rules`), collision types, statuses,
                                          lock-on modes, and the roles (roles.luau)
lib/*.luau, lib/FAMILY/*.luau             helpers and builders families share; the shared effects, sparks
                                          and regions
core.d.luau, types.d.luau                 the API's declarations and the pack's shared types (for editors
                                          and the checker)
compat/*.toml                             the original's numbers by key: tools' data, never the engine's
locale/<lang>.toml                        content's words in another language, by key: a frontend's, never the engine's
```

No folder or file is named with a number of the original's (a test guards it): a chip's folder is its key
(`chips/gundels`), a form's its name (`navis/megaman/forms/heatcross`).

A module is `--!strict` Luau that returns a table. While it loads it makes definitions and names assets; what it
returns is what other modules get from `require("../../lib/bombs/bomb")` (a path relative to the requiring
file). Nothing registers a module: the engine runs what the definitions hold.

`nettai_content::root::read(dir)` reads a content root: every module, by its path without `.luau`.
`nettai_content::root::bn6()` is BN6's: `$BN6_CONTENT`, else this repository's content/bn6.

## 2. The pack

```text
content.toml                              the manifest ([graphics], [sound])
assets.toml                               the asset index: every asset content can name, by kind and name
graphics/sprites/NAME/                    a sprite: parts, layouts, animation timing (animations.json)
graphics/...  sound/...                   see asset-formats.md
```

The index lists every sprite, sound, banner, background and mugshot by name, with the engine's identity for it:
every name compat/assets.toml gives, and the pack's other assets under their placeholders (`sprite-0c-2d`),
which content may not use (`nettai-content-check` flags one: name the asset in compat/assets.toml first). The pack
holds the game's own data: it is written outside version control (data/content/ is ignored).

## 3. Loading

`nettai_content::pack::load_battle(content, pack)` reads the content root and, from the pack, the asset index
(`Content::assets`) and the sprites' timing (`Content::animations`), then runs the define phase
(`Content::define`):

1. every module loads once, in a VM of its own, and what the modules define is read back as data: the
   canonical tree, definitions sorted by registry and key, with references between definitions by key and
   functions as slots;
2. the engine builds its registries from it (`content::defs`): each registry's entries in key order, an
   entry's handle its place, the engine's own entries (its object kinds, keyed `engine/...`) among them; and
   the typed records the ruleset reads (a chip's record and use, a navi's stats, a stage's layout and actors,
   the rule sections);
3. it plans what a runtime binds: the function slots it will call (a kind's `update`, a chip's `dimming`), by
   definition and path, and the state layouts.

A definition that doesn't read is a content error naming its module: a missing field, a reference to the wrong
registry, two definitions with one key, a chip without exactly one use, an asset name the pack's index doesn't
have. `nettai_content::pack::battle_content` is the same before the define phase.

`nettai-content check <pack> [--content DIR]` runs every import and the define phase, and reports on the
definitions: roles the content hasn't filled, kinds under `objects/` that one owner alone uses, collision types
defined twice. `nettai-content-check <content>` type-checks every module against core.d.luau and lints the source
(scripting.md §3.3).

The verification workspace's `gen-content check <rom> <content>` defines the content root with compat's asset
names and compares what it builds with the ROM, by the numbers compat gives each key: the chips, navis, forms,
weapons, stages, rule sections, statuses, lock-on modes, effects, sparks, regions, collision types, identities
and the variants compat names.

Loading is straight from the files: there is no derived cache. A frontend that doesn't play sound (headless
rendering, `--mute`) skips the sound.

## 4. The `Content` API

`Content` (crates/nettai-battle/src/content) is plain data: `Clone`, `PartialEq`, `Hash`. Engine code reads it
through the battle, by handle:

```rust
let content: Arc<Content> = Arc::new(nettai_content::pack::load_battle(&nettai_content::root::bn6(), pack)?.0);
let setup = RoundSetup { content: content.hash(), settings, navi_stats, ... };
let mut b = Battle::new(setup, content.clone());

let chip = b.content.chip(handle);                 // &ChipData: the chip's record
let usage = b.content.defs.chip(handle).usage;     // its use: an action, a dimming, navi or instant hook
let navi = b.content.navi(stats.navi);             // &NaviData (banners, move lag, ...)
let form = b.content.form(stats.form);             // &FormData (sprite, weapons, ...)
let weapon = b.content.weapon(form.weapons.charge_shot.unwrap());
let frames = b.content.animation(sprite, anim);    // &[AnimFrame]
let look = b.content.effect(effect);               // EffectSprite, by EffectHandle
let panels = b.content.region_offsets(region);     // &[PanelOffset], by Option<RegionHandle>
let stage = b.content.stage(b.setup.settings.stage);
let action = b.content.defs.roles.action(ActionRole::StunStrike);
let pas = b.content.program_advances();            // in the order they are tried
```

- A handle (`ChipHandle`, `KindHandle`, `ActionHandle`, ...) is an index into its registry; a key reaches one
  through `content.defs` (`chip_by_key`, `kind_by_key`, `action_by_key`). Engine code holds handles; only tests,
  tools and the setup codecs name keys.
- What the ruleset itself needs from content, it asks by **role** (`content.defs.roles`, filled by
  rules/roles.luau): the action a request starts, the kind it spawns, the chip a zeroed field reads, the effect
  a deletion shows. A role content hasn't filled is an error where the ruleset needs it.
- `Battle::new(setup, content)` panics if `setup.content` isn't `content.hash()`: a round can't silently run on
  other content than its setup names.
- Code without a `Battle` gets what it needs passed in: the sprite stepper takes the content
  (`sprite.set_animation(anim, &b.content)`), the field takes it to lay out and refresh panels, the custom
  screen reads a `custom::Library`, which `Content` implements.
- Where a function needs `&mut Battle` while holding content data, clone the `Arc` first (`let content =
  b.content.clone();`): it costs an atomic increment.
- Trace replays (bn6-compat's `trace`) take the content and compat: `trace::run_round(round, &content,
  &compat)`. The setup codecs (`bn6_compat::codec`) take an `Ids` (the content and compat), which maps the
  records' numbers to the engine's handles and back: a folder's chip ids, a navi stats record's weapons, the
  stage a battle settings record is.
- The scripts run from the content: `Battle::new` gets the runtime with `Behaviors::for_content(&content)`
  (once per thread and content hash; the VM is a cache, not battle state).

## 5. Identity, snapshots and netplay

- `Content::hash()` is a stable 64-bit hash of all of it, scripts included (the state digest's hasher over the
  typed data and the modules' source). It changes with any edit to a definition or a script and with the
  engine's data layout.
- `RoundSetup::content` carries it. The setup is part of the state digest, so two peers whose digests agree run
  the same content; a netplay session should compare setups (hence the hash) before the match.
- `Battle::content` is an `Arc<Content>`: cloning a battle (a snapshot) shares it, and the digest leaves it out.
  Battle state refers to content by handle only.

## 6. Tests and other content

In-repo tests never load game data. `nettai_battle::content::testing` (the `test-content` feature, and the
engine's own tests) is a small content set: its own modules (crates/nettai-battle/testdata/content: made-up chips,
navis, stages, statuses and lock-on modes, and its roles), some of content/bn6's modules read from the
repository with whatever they `require`, and made-up assets for the names they use. So the tests run the real
scripts on made-up data. A second set, the test pack (crates/nettai-battle/testdata/pack), is definitions written
for the tests alone. The engine, netplay, audio and frontend tests run on them; one test defines all of
content/bn6 with made-up assets.

Tests about BN6's actual data (effect lifetimes, GunDelSol's 480 HP in the sun, the stages' rocks, the golden
traces, the chip lab) live in the verification workspace, which loads content/bn6 with the BN6 pack it extracts
to `data/content/bn6`.
