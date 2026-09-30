# Content packs: the battle data

A battle runs on **content**: chips, navis and their forms, the ruleset's
tables, object kinds' data, the effect and region registries, and every
sprite's animation timing. The engine holds it as one typed value,
`bn6_battle::Content`, and reads it from nowhere else: no ROM, no tables
compiled into the engine, no files. A **content pack** is where content
comes from: a folder of open-format files that `bn6-extract content` writes
from the user's ROM and that ordinary tools edit. This document describes
the pack's battle data (its layout, its files, how it loads) and the
`Content` API the engine and other layers use. The pack's graphics and
sound are described in [asset-formats.md](asset-formats.md).

## 0. Summary

- **One pipeline.** `bn6-extract content <rom> <pack>` is the only
  extraction. It writes the battle data, the graphics and the sound, and
  reads the battle data straight back to check it loads as the same
  `Content`. The engine, the frontend, the audio, netplay and the
  verification workspace all load packs.
- **By owner.** Data that belongs to one entity lives in that entity's
  folder (a chip's own attack data in the chip's `chip.toml`, a navi's
  banners in its `navi.toml`); only rules no entity owns and registries many
  entities refer to by number are shared files.
- **Typed, named, TOML.** Every field has a name and a type: enums as
  names (`element = "aqua"`), flag bytes as lists of flag names, sprites as
  their folder names (`"0c-3b"`), ids and flag words in hex. No offsets,
  no `unk` fields.
- **Original ids, dense tables.** Every record carries the original's
  number (`id = 0x11`); the loader builds the engine's id-indexed tables
  and reports duplicate ids, gaps and dangling references by file.
- **Shared and immutable.** A battle holds its content in an
  `Arc<Content>`: snapshots share it and the state digest leaves it out.
  Its identity, `Content::hash()`, is part of the round's setup
  (`RoundSetup::content`), so netplay peers can check they run the same
  content.
- **Exact.** The BN6 pack's `Content` equals the tables the engine used
  to compile in, field by field (§7), and every golden trace, the sound
  calls and the rendered frames are unchanged.

## 1. Layout

```text
content.toml                              the manifest ([battle], [graphics], [sound])
chips/NNN-name/chip.toml                  a chip, with the data only its action reads
navis/NN-name/navi.toml                   a navi (MegaMan is 00)
navis/00-megaman/forms/NN-name/form.toml  one of MegaMan's forms (00 is the base form)
objects/KIND/object.toml                  an object kind's own data
rules/*.toml                              rules no entity owns
registries/*.toml                         what many entities name by number
graphics/sprites/CC-II/animations.json    each sprite's animation timing (with its graphics)
graphics/...  sound/...                   see asset-formats.md
```

| File | Holds | Engine side |
|---|---|---|
| `chips/NNN-name/chip.toml` (411) | the chip record; `[gun_del_sol]` (firing time, beam looks, the gun), `sp_damage`, `[[program_advance]]` recipes that make this chip, `modifier` | `Content::chips`, `ChipData` |
| `navis/NN-name/navi.toml` (12) | sprite, element, weakness, buster bonus, move lag by variant, result banners, Cross merge height, `[own_chip]`, `[name_record]` (NameID, actor record, attach points) | `Content::navis`, `NaviData` |
| `navis/00-megaman/forms/NN-name/form.toml` (25) | sprite, element, weakness, `[weapons]`, buster bonus, `[name_record]` | `Content::forms`, `FormData` |
| `objects/rock/object.toml` | rock variants | `ObjectData::rocks` |
| `objects/absorbed-obstacle/object.toml` | the sprite an absorbed obstacle flies with, by kind | `ObjectData::absorbed_sprites` |
| `objects/body-overlay/object.toml` | Cross body overlays: sprite, in front by animation | `ObjectData::body_overlays` |
| `objects/sun-beam/object.toml` | the sun beam's sprites by look | `ObjectData::sun_beam_looks` |
| `objects/attachment/object.toml` | attachments no chip declares | `ObjectData::attachments` (with the chips' own) |
| `rules/elements.toml` | element weakness; secondary elements by chip family | `Rules::element_weakness`, `family_elements` |
| `rules/collision.toml` | collision types by number, for each side; whole-field hit regions (0x80 and up) | `Rules::collision_types`, `field_regions` |
| `rules/panels.toml` | panel types' flag bits and road slides; start visibility and front edges; step rules (normal, dash, any side) | `Rules::panels` |
| `rules/stages.toml` | battle settings; actor lists with their original addresses | `Rules::stages` |
| `rules/banners.toml` | banners that hold until removed | `Rules::holding_banners` |
| `rules/status.toml` | status effects by status byte; the HP bug's drain periods | `Rules::status_effects`, `hp_bug_periods` |
| `rules/weapons.toml` | weapon routines' charge times; buster recovery by Rapid | `Rules::weapons`, `buster_recovery` |
| `rules/reactions.toml` | push and ice slides; the bubble's bob | `Rules::push_vectors`, `ice_vectors`, `bubble_bob` |
| `rules/lockon.toml` | the Beast Out lock-on's panel searches by mode; column shifts | `Rules::lockon` |
| `rules/sp-chips.toml` | the deletion times at which SP navi chips' damage steps down | `Rules::sp_deletion_times` |
| `rules/custom-screen.toml` | the custom screen's slot grid and neighbour scan lists | `Rules::custom_screen` |
| `registries/effects.toml`, `sparks.toml` | one-shot effects and hit sparks by id | `Content::effects`, `sparks` |
| `registries/regions.toml` | hit-region shapes by region number | `Content::regions` |
| `registries/panel-layouts.toml` | panel layouts by layout number | `Content::panel_layouts` |
| `graphics/sprites/CC-II/animations.json` (298) | frame durations and flags | `Content::animations` |

BN6's battle data is 468 files (about 2 MiB) of the pack's 2,759 files;
chips are most of it.

### 1.1 What goes where: colocate single-owner data

The rule: **data that one entity owns lives with that entity; shared
files hold only rules no entity owns and registries that many entities
refer to by number.**

- A chip's folder holds everything only that chip's behavior reads.
  GunDelSol's firing time, its beam's looks in the shade and in the sun,
  and its gun's attachment row are in each GunDelSol chip's `chip.toml`,
  not in attack tables indexed by level; an SP navi chip's damage by
  deletion time is in that chip's file; a Program Advance's recipes are in
  the file of the chip they make (a recipe names several chips, but it is
  the definition of its result: adding a Program Advance touches one
  folder); a modifier chip says what it modifies.
- A navi's folder holds its banners, move lag, buster bonus, own chip and
  its NameID's actor record and attach points; a form's folder the same
  for the form. Per-form and per-navi tables indexed by number dissolve
  into these files.
- An object kind's folder holds its own variants (rocks, overlays, the sun
  beam's looks).
- `rules/` holds what no entity owns: element weakness, collision types,
  panel rules, battle settings, statuses, the custom screen's layout.
- `registries/` holds what many entities name by number: effects, hit
  sparks, region shapes, panel layouts. Sprites are a registry too, one
  folder each under `graphics/sprites` with the timing beside the pixels.

Folder and file names are for people: `chips/011-gundels3`. The engine
goes by the ids inside.

### 1.2 Observable ids stay

Some numbers are part of the state or of what the traces compare: an
attachment object's first parameter is its row number, the sun beam's its
look, a rock's its variant, an actor list is named in link data by its
original address. Records that dissolve into their users keep those
numbers as explicit fields (`[gun_del_sol.gun] id = 0x09`,
`beam = { look = 0, palette = 0 }`, `original_address = 0x080b1aad`), and
the loader assembles the dense tables the engine indexes by them. Two
chips may declare the same attachment row (the same data); declaring it
differently is an error.

### 1.3 Room for scripts and mods

A chip's or object's folder is where its script will go when the
behavior moves to content (docs/design/scripting.md): today the Luau
scripts live in the repository (content/bn6) and read their entity's data
from the battle's `Content` (§4). Packs are designed to overlay: a later
pack could add or replace records by id; the loader doesn't implement
overlays yet.

## 2. The files

TOML, because the data is records people edit by hand: tables of named
fields, comments, hex integers, and one obvious way to write each value.
(JSON stays for the machine-shaped sprite layouts and Tiled's maps; TOML is
already the pack's format for the manifest and the sound's sidecars.)

A chip:

```toml
# Chip 0x011, GunDelS3. See docs/design/content-pack.md for what each field means.

id = 0x11
name = "GunDelS3"
codes = ["N", "Q", "W"]
element = "null"
rarity = 3
family = "null"
class = "standard"
mb = 38
flags = ["standard_library", "library"]
hit_param = 0
action = 0x37
subtype = 2
beast_lockon = true
params = [0, 0, 0, 0]
lockout = 0
extra_flags = []
lockon_mode = 9
damage = 0
library_number = 17
library_index = 17
sort_key = 229
slot_in_limit = 3

[gun_del_sol]
firing_ticks = 120

[gun_del_sol.beam]
look = 0
palette = 0

[gun_del_sol.beam_in_sun]
look = 0
palette = 2

[gun_del_sol.gun]
id = 0x09
sprite = "0c-3b"
palette = 6
lift = 0
attach_point = 13
```

Conventions:

- **Enums by name**: `element` (`null`, `fire`, `aqua`, `elec`, `wood`),
  `family` (`fire` … `null`, `program_advance`, `special`), `class`,
  panel types (`normal`, `road_up` …), actor types, status timers,
  actor-list entry kinds (`navi`, `rock`, `object_6e`, `object_7d`: the
  original's object numbers are the identity of BN6's object kinds).
- **Flag bytes as lists** of flag names, with any bit that has no name as
  its number: chips' `flags` (`cut_in`, `has_damage`, `navi`,
  `standard_library`, `damage_shown_variable`, `library`,
  `variable_damage`), `extra_flags` (`rush_cancels`, `free_slot_in`, and
  menu-only bits as numbers), secondary elements (`break`, `wind`,
  `cursor`, `sword`). The byte is exactly what the list says.
- **Flag words in hex**: collision types, panel type flags, step and
  region conditions, status requests, battle effects. Their bits are
  documented in docs/engine/field-collision-damage.md; they are matched
  as whole words (`target & self`), so they stay words.
- **Sprites** as `"CC-II"`, the sprite's folder under `graphics/sprites`.
- **Points and offsets** as `[x, y]` / `[dx, dy]`; panel grids as rows of
  `#` and `.`; deletion times as `m:ss.cc`.
- **Chip codes** as letters (`"*"` for the asterisk).
- **Dense lists by id**: every record has `id`; ids must run from 0
  without gaps where the engine indexes a table by them (chips, navis,
  forms, collision types, effects...).

Everything the engine reads is in these files; nothing is derived at load
except the dense tables. What the engine doesn't read isn't extracted
(chip record bytes no routine reads; the table rows no navi number can
reach, §7).

## 3. Loading

`bn6_content::pack::load_battle(pack)` (or `bn6_content::battle::load`)
reads the battle data and the sprite timing into a `Content`, with a
`Report` of what it found. It checks:

- the manifest is a pack with battle data;
- every file parses, with no unknown fields (a misspelled field is an
  error naming the file);
- ids are unique and dense where the engine indexes by them ("chip 0x3
  is also in chips/003-sungun3/chip.toml", "chip 0x2 is missing");
- references resolve: battle settings' actor lists and panel layouts,
  GunDelSol chips' beam looks, SP chips' damage rows (one more entry than
  there are deletion times), NameIDs unique across navis and forms.

`bn6-content check <pack>` runs every import and prints the report.
`bn6-extract content` writes the files (`battle::export`), loads them back
and requires the same `Content`; `bn6-content verify <pack> <reference>`
compares what two packs load (after an editor round trip, say).

Loading is straight from the files: there is no derived cache. BN6's
full pack loads in about 0.75 s in a release build: the battle data in
45-150 ms (the sprite timing is 10-100 ms of it, the content hash 0.1 ms),
the graphics in 60-110 ms and the sound in 520-640 ms (its songs' MIDI and
samples' WAV files). A frontend that doesn't play sound (headless
rendering, `--mute`) skips the sound.

## 4. The `Content` API

`Content` (crates/bn6-battle/src/content) is plain data: public fields,
`Clone`, `PartialEq`, `Hash`. Engine code reads it through the battle:

```rust
let content: Arc<Content> = Arc::new(bn6_content::pack::load_battle(pack)?.0);
let setup = RoundSetup { content: content.hash(), settings, navi_stats, ... };
let mut b = Battle::new(setup, content.clone());

let chip = b.content.chip(0x11);                 // &ChipData
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
- Trace replays take the content: `trace::run_round(round, &content)`,
  `round.start(content)`, `round.round_setup(&content)`.
- `BattleSettings::netbattle_from_bytes(bytes, &content)` resolves the
  actor list the record names by its original address;
  `BattleSettings::actors` is an `ActorListId` into
  `content.rules.stages.actor_lists`.
- The behaviors layer builds its runtimes from the content:
  `Behaviors::luau(&content)` gives the Luau scripts a frozen `data/pack`
  module built from it (each chip's own data, the attachments, the sun
  beam's looks), `Behaviors::rust(&content)` the same as typed data.

## 5. Identity, snapshots and netplay

- `Content::hash()` is a stable 64-bit hash of all of it (the state
  digest's hasher over the typed data), so it doesn't depend on how the
  files are formatted. BN6's pack is `0909753cc2c672eb` today; it changes
  with any edit and with the engine's data layout.
- `RoundSetup::content` carries it. The setup is part of the state
  digest, so two peers whose digests agree run the same content; a netplay
  session should compare setups (hence the hash) before the match.
- `Battle::content` is an `Arc<Content>`: cloning a battle (a snapshot)
  shares it, and the digest leaves it out. No state refers to content by
  reference any more: the actor list is an id, a rock keeps its variant.

## 6. Tests and other content

In-repo tests never load game data. `bn6_battle::content::testing` (the
`test-content` feature, and the engine's own tests) is a small content set
written by hand: made-up chips on the engine's GunDelSol, dimming and
navi-chip actions, one navi and its base form, rocks, a custom-screen
layout, and collision types and panel rules written from the engine's own
flag semantics. The engine, netplay, audio and frontend tests run on it;
bn6-content's tests write it as a pack and read it back. Tests about BN6's
actual data (effect lifetimes, GunDelSol's 480 HP in the sun, the soundmod
stage's rocks, the golden traces) live in the verification workspace, which
extracts the BN6 pack to `data/content/bn6`.

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
