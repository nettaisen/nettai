# Curation proposal: names for the invented and unclear keys

The ROM names almost nothing, so when the content was generated (content model v2, steps 1 and 5) the
generator made names up: from an asset's first user, an effect's sprite and number, a region's or a collision
type's row, a status's fields. compat/curation.toml lists them for review, and the disassembly's sound enum
adds a few with numbers in them (`hit-6b`, `log-in-77`). This document proposes a name for each, with the
evidence (where content and the engine use it, what the sprite shows, the docs, the disassembly) and a
confidence, so the review is a matter of accepting groups. **Nothing is renamed yet.**

It covers 1108 names: 219 proposed renames (42 high, 157 medium and
20 low confidence), 636 names to keep as they are, and 253 numbered placeholders
(`sprite-0c-26`, `sound-108`) to leave numbered. 28 names are flagged as wrong now: they say something
their asset or definition isn't.

**Applied so far: 64 renames**, marked *applied* below (accepted-1, accepted-2). accepted-1 is the user's
"High + flagged": every high-confidence rename and every row flagged wrong then. accepted-2 is chip destruction,
the user's name for collision row 0x08, its shot and its hit spark ("attack-90 is chip destruction and the
sparks.shot is also the chip destruction hitspark"), in both games. The rest wait for review.

The machine-readable list is [curation-proposal.tsv](curation-proposal.tsv), one row per name: registry, current
name, proposed name, confidence, flag (`wrong`, or `number` for a name carrying one of the original's numbers)
and evidence, and a status naming the accepted list that applied it.

## How to accept

The verification workspace's `tools/curation/apply.py` applies an accepted list mechanically:

```sh
# Every high-confidence row (renames and the names kept), or every row up to medium:
tools/curation/apply.py <checkout> <checkout>/docs/design/curation-proposal.tsv --only high --verify .
tools/curation/apply.py <checkout> <checkout>/docs/design/curation-proposal.tsv --only medium --verify .
# Exactly the rows a file lists, as `registry current` lines (`sprite copy-mark`):
tools/curation/apply.py <checkout> <checkout>/docs/design/curation-proposal.tsv --names accepted.txt --verify .
```

An accepted list is committed in the verification workspace (`tools/curation/accepted-1.txt`) and can run again
(`--game exe5` applies it to EXE5's content, which shares many of these names; EXE6's is the default):
a rename applied before only has its remaining uses renamed. So a branch that merged main after a list was
applied catches up its own new content with the same command, then reads the leftover list:

```sh
tools/curation/apply.py <checkout> <checkout>/docs/design/curation-proposal.tsv --names tools/curation/accepted-1.txt --verify .
```

For each accepted rename it changes compat (assets.toml; rules.toml's [statuses] and [lockon]; records.toml's
[projectile_variants]), the content that names it (`asset.sprite("...")`, a lib table's field and every
`<alias>.<field>` of a module that requires it, a definition's `id`), the test content's asset lists, Rust that
names an asset (`AssetKind::Sprite, "..."`, `assets.sprites["..."]`), the docs' `asset.*("...")` mentions
and, with `--verify`, the verification workspace's tests that name one (gen-content's, the trace tests' looks); an
accepted row, renamed or not, leaves curation.toml. It then lists every place that still quotes an old name or
reads an old field (comments, prose, the same word in another registry) for a person to read.
Weapons, actions, kinds, chips, chip series and stages are only settled by it, never renamed: their keys
name folders, Rust tests and the compat entries the traces read, and no rename is proposed for them.

The gate after applying: `cargo build --workspace --all-targets`, `cargo test --workspace`, `nettai-content-check`,
a fresh pack (asset renames rename the pack's files), `gen-content check` and its tests, both golden traces and
the full chip lab.

## Flagged: wrong now

These names mislead: they name one user of something many use, a look the asset doesn't show, or fields that
are garbage. Their renames are worth taking even where the proposed name is only medium confidence.

| registry | current | proposed | confidence | evidence |
|---|---|---|---|---|
| sprite | `bat-impact` | `hit-marker` *applied* | medium | The role sprites.hit_marker (the "!!" over a navi), the cut-in flash (effect 0x1E) and BatCan's spark (animation 1). Shows bursts, "!!" marks and bats: BatCan is one user of three. |
| sprite | `beast-over-burst` | `lightning` *applied* | high | Shows lightning bolts. ElecMan's thunder, ElemTrap's and ElmntMan's bolts, and Beast Over's burst (effect 0x45) use it. |
| sprite | `copy-mark` | `hit-sparks` *applied* | high | Shows the hit sparks (plain, breaking, and one per element: sparks by element use its animations 0, 2-5) and the TRAP! mark (animation 7); 30 uses, CopyDmg's mark only one of them. |
| sprite | `immobilized` | `rock-cubes` *applied* | medium | Shows the rock cube, the rock and the ice cube: the rocks' and cubes' sheet (chips/rockcube/rock.luau), the encased bubble and WideSht's trail. The immobilized status visual (a role) is one animation of it. |
| sprite | `reflected-shot` | `pink-flash` *applied* | medium | Shows a pink starburst. The effect `flash` (0x21), CrcusMan's sparkle, H-Burst's explosion and DblBeast's spark use it. |
| sprite | `reflector-shield-2` | `dummy-shield` *applied* | low | The ROM's dummy sprite (a 16x16 dot). Rflectr's rows 4-6 name it, which nothing ported holds up. |
| sprite | `rising-bubble` | `small-puff` *applied* | medium | Shows small gray puffs. lib/effects' splash, dust and ripple and Geddon's puffs use it; nothing rises as a bubble. |
| sprite | `shot-impact` | `chip-destruction-spark` *applied* | high | 14-0c: the chip destruction hit spark (spark 0x09, lib/sparks). The user's name. |
| sprite | `small-ring` | `wide-navi` *applied* | medium | The ROM's dummy sprite (a 16x16 dot, like 40 unused slots): it shows no ring. SumnBlk asks whether an actor wears it to know it stands two panels wide. |
| sound | `copy-mark` | `mark` *applied* | medium | A mark set: BurnSqr's fire, CircGun's and CopyDmg's marks. CopyDmg is one user of three. |
| effect | `bat_impact` | `cut_in_flash` *applied* | high | The role effects.cut_in_flash: the flash where a side cut in (effect 0x1E). |
| effect | `small_ring_82` | `dummy_look` *applied* | low | Effect 0x4B reads animation 82, palette 16 of the dummy sprite: a row of the original's table that names no real look. |
| spark | `shot` | `chip_destruction` *applied* | high | Spark 0x09 is the chip destruction hit spark (the chip destruction shot's, and EXE5's SerchMan's shot's). The user's name. |
| status | `collision-panel-65535` | `after-table-1-past-freeze` *applied* | medium | Status 0x5C: the freeze group's entry 12, past the bubble rows it also reads, reads row 1 after the table: its fields are whatever bytes follow the table, which the current name spells out as if they meant something. Named as the other overflow reads are (`<what it reads>-past-<group>`). |
| status | `collision-panel-65535-2` | `after-table-1-past-bubble` *applied* | medium | Status 0x66: the bubble group's entry 6 reads row 1 after the table (the same bytes as the freeze group's overflow, for the first four). |
| status | `timer-00-150` | `after-table-5-past-bubble` *applied* | medium | Status 0x6A: the bubble group's entry 10 reads row 5 after the table (the same bytes as the freeze group's overflow, for the first four). |
| status | `timer-00-200` | `after-table-8-past-bubble` *applied* | medium | Status 0x6D: the bubble group's entry 13 reads row 8 after the table (the same bytes as the freeze group's overflow, for the first four). |
| status | `timer-00-30` | `after-table-2-past-freeze` *applied* | medium | Status 0x5D: the freeze group's entry 13, past the bubble rows it also reads, reads row 2 after the table: its fields are whatever bytes follow the table, which the current name spells out as if they meant something. Named as the other overflow reads are (`<what it reads>-past-<group>`). |
| status | `timer-00-30-2` | `after-table-2-past-bubble` *applied* | medium | Status 0x67: the bubble group's entry 7 reads row 2 after the table (the same bytes as the freeze group's overflow, for the first four). |
| status | `timer-01-65535` | `after-table-7-past-bubble` *applied* | medium | Status 0x6C: the bubble group's entry 12 reads row 7 after the table (the same bytes as the freeze group's overflow, for the first four). |
| status | `timer-01-65535-2` | `after-table-10-past-bubble` *applied* | medium | Status 0x6F: the bubble group's entry 15 reads row 10 after the table (the same bytes as the freeze group's overflow, for the first four). |
| status | `timer-64-65535` | `after-table-4-past-freeze` *applied* | medium | Status 0x5F: the freeze group's entry 15, past the bubble rows it also reads, reads row 4 after the table: its fields are whatever bytes follow the table, which the current name spells out as if they meant something. Named as the other overflow reads are (`<what it reads>-past-<group>`). |
| status | `timer-64-65535-2` | `after-table-4-past-bubble` *applied* | medium | Status 0x69: the bubble group's entry 9 reads row 4 after the table (the same bytes as the freeze group's overflow, for the first four). |
| status | `timer-ff-0` | `after-table-3-past-freeze` *applied* | medium | Status 0x5E: the freeze group's entry 14, past the bubble rows it also reads, reads row 3 after the table: its fields are whatever bytes follow the table, which the current name spells out as if they meant something. Named as the other overflow reads are (`<what it reads>-past-<group>`). |
| status | `timer-ff-0-2` | `after-table-3-past-bubble` *applied* | medium | Status 0x68: the bubble group's entry 8 reads row 3 after the table (the same bytes as the freeze group's overflow, for the first four). |
| status | `timer-ff-0-3` | `after-table-6-past-bubble` *applied* | medium | Status 0x6B: the bubble group's entry 11 reads row 6 after the table (the same bytes as the freeze group's overflow, for the first four). |
| status | `timer-ff-100` | `after-table-9-past-bubble` *applied* | medium | Status 0x6E: the bubble group's entry 14 reads row 9 after the table (the same bytes as the freeze group's overflow, for the first four). |
| rock-variant | `rockcube/cube` | `rockcube/rock/cube` *applied* | high | The rock's variant (RockCube's and the stages' cube): a record of the kind `rockcube/rock`. `rockcube/cube` is also the key of the kind RockCube's controller (chips/rockcube/cube.luau): legal, since the registries differ, but one key naming two things misleads a reader and a search. Naming the variants under the rock they vary keeps them owner-qualified and apart; the other three follow for consistency. |

Also found while reviewing (no rename needed):

- `trap-vanish`'s curation note says song 0x108; assets.toml has it at 0x107, where the presentation check moved it.
- `megaman-navi` (08-00) and `django` (0c-0f) are the ROM's dummy sprite: MegaMan's battle sprites are 00-00 and
  his forms', and Django is the JP version's. Their names are right for what names them.
- `effects.small_ring_82` reads animation 82 and palette 16 of the dummy sprite: the original's effect 0x4B is a
  row nothing real uses.
- `rockcube/cube` is two keys: the rock variant (a record, compat records.toml) and RockCube's controller (a kind).
  The registries differ, so it loads, but a reader or a search can't tell them apart: the rock variants table below
  proposes `rockcube/rock/cube` (and the other three variants alike).

## High confidence: accept in bulk (42)

Each name follows from a single, clear use or from what the asset reads: a Cross's body overlay worn by that form alone, a banner's text, a sound's role, the disassembly's own name without its number.

### Sprites (16)

| current | proposed | evidence |
|---|---|---|
| `beast-over-burst` | `lightning` (wrong now) *applied* | Shows lightning bolts. ElecMan's thunder, ElemTrap's and ElmntMan's bolts, and Beast Over's burst (effect 0x45) use it. |
| `body-overlay-02` | `heatman-overlay` *applied* | Only HeatMan wears it (navis/heatman/init.luau): his flames. |
| `body-overlay-04` | `heatcross-overlay` *applied* | Only the heatcross form wears it (navis/megaman/forms/heatcross/init.luau, `parts`); named for the body overlay row it had. |
| `body-overlay-05` | `spoutcross-overlay` *applied* | Only the spoutcross form wears it (navis/megaman/forms/spoutcross/init.luau, `parts`); named for the body overlay row it had. |
| `body-overlay-08` | `eleccross-overlay` *applied* | Only the eleccross form wears it (navis/megaman/forms/eleccross/init.luau, `parts`); named for the body overlay row it had. |
| `body-overlay-09` | `tengucross-overlay` *applied* | Only the tengucross form wears it (navis/megaman/forms/tengucross/init.luau, `parts`); named for the body overlay row it had. |
| `body-overlay-0a` | `slashcross-overlay` *applied* | Only the slashcross form wears it (navis/megaman/forms/slashcross/init.luau, `parts`); named for the body overlay row it had. |
| `body-overlay-0b` | `groundman-overlay` *applied* | Only GroundMan wears it (navis/groundman/init.luau). |
| `body-overlay-0c` | `erasecross-overlay` *applied* | Only the erasecross form wears it (navis/megaman/forms/erasecross/init.luau, `parts`); named for the body overlay row it had. |
| `body-overlay-0d` | `groundcross-overlay` *applied* | Only the groundcross form wears it (navis/megaman/forms/groundcross/init.luau, `parts`); named for the body overlay row it had. |
| `body-overlay-0e` | `tomahawkcross-overlay` *applied* | Only the tomahawkcross form wears it (navis/megaman/forms/tomahawkcross/init.luau, `parts`); named for the body overlay row it had. |
| `body-overlay-11` | `chargecross-overlay` *applied* | Only the chargecross form wears it (navis/megaman/forms/chargecross/init.luau, `parts`); named for the body overlay row it had. |
| `body-overlay-12` | `dustcross-overlay` *applied* | Only the dustcross form wears it (navis/megaman/forms/dustcross/init.luau, `parts`); named for the body overlay row it had. |
| `burner-2` | `heatcross-burner` *applied* | HeatCross's burner (lib/burner/burn.luau, attachment row 0x1E); `burner` is FireBrn's. |
| `copy-mark` | `hit-sparks` (wrong now) *applied* | Shows the hit sparks (plain, breaking, and one per element: sparks by element use its animations 0, 2-5) and the TRAP! mark (animation 7); 30 uses, CopyDmg's mark only one of them. |
| `shot-impact` | `chip-destruction-spark` (wrong now) *applied* | 14-0c: the chip destruction hit spark (spark 0x09, lib/sparks). The user's name. |

### Sounds (9)

| current | proposed | evidence |
|---|---|---|
| `buster-6a` | `buster-shot` *applied* | The disassembly's SOUND_BUSTER_6A: a buster shot (CopyDmg, CrosOver's MegaMan, DblHero's volley). |
| `elmnt-man-3` | `grass` *applied* | The grass panel's trail sound (rules/panels.luau), GrassSeed, ElmntMan turning the field to grass. |
| `fade-8e` | `fade` *applied* | The disassembly's SOUND_FADE_8E; the role sounds.fade. |
| `hit-6b` | `own-hit` *applied* | The disassembly's SOUND_HIT_6B; the role sounds.own_hit: a navi hit, as its own player hears it. |
| `hit-6d` | `hit` *applied* | The disassembly's SOUND_HIT_6D; the role sounds.hit: a navi hit, as the other player hears it. |
| `hit-6e` | `guard-hit` *applied* | The disassembly's SOUND_HIT_6E; the role sounds.guard: a blocked hit. |
| `hit-87` | `damage-bonus` *applied* | The disassembly's SOUND_HIT_87; the role sounds.damage_bonus. |
| `log-in-77` | `log-in` *applied* | The disassembly's SOUND_LOG_IN_77: the Cross change's chime (a role), AntiSwrd's counter, BugFix, DeltaRay, the navi boosts. |
| `select-86` | `counter-hit` *applied* | The disassembly's SOUND_SELECT_86 (a menu name); the role sounds.counter_hit. |

### Banners (5)

| current | proposed | evidence |
|---|---|---|
| `banner-54` | `time-up` *applied* | Reads "TIME UP!"; no netbattle shows it. |
| `banner-58` | `liberate-success` *applied* | Reads "LIBERATE SUCCESS!"; no netbattle shows it. |
| `banner-5c` | `liberate-failed` *applied* | Reads "LIBERATE FAILED!"; no netbattle shows it. |
| `banner-60` | `turn-liberate` *applied* | Reads "TURN LIBERATE!"; no netbattle shows it. |
| `held-28` | `hit-damage-judge` *applied* | Reads "HIT DAMAGE JUDGE" (the role banners.judge). |

### Effects (lib/effects.luau) (3)

| current | proposed | evidence |
|---|---|---|
| `bat_impact` | `cut_in_flash` (wrong now) *applied* | The role effects.cut_in_flash: the flash where a side cut in (effect 0x1E). |
| `beast_over_burst_45` | `beast_over_burst` *applied* | The role effects.beast_over_burst (effect 0x45). |
| `copy_mark_7` | `trap_mark` *applied* | The role effects.trap_mark: the TRAP! mark over a navi whose trap sprang (effect 0x46). |

### Sparks (lib/sparks.luau) (3)

| current | proposed | evidence |
|---|---|---|
| `hit` | `guard` *applied* | Spark 0x08 is the role sparks.guard: a blocked hit. |
| `shot` | `chip_destruction` (wrong now) *applied* | Spark 0x09 is the chip destruction hit spark (the chip destruction shot's, and EXE5's SerchMan's shot's). The user's name. |
| `spark_0e` | `uninstall` *applied* | Spark 0x0E is the role sparks.uninstall: a navi's programs uninstalled. |

### Collision types (rules/collision.luau) (1)

| current | proposed | evidence |
|---|---|---|
| `attack-90` | `chip-destruction` *applied* | Row 0x08: chip destruction, an attack that destroys the chip a navi holds (flag 0x10). The chip destruction shot uses it (shot/attack-90). The user's name. |

### Projectile variants (objects/projectile/variants.luau) (1)

| current | proposed | evidence |
|---|---|---|
| `shot/attack-90` | `shot/chip-destruction` *applied* | Projectile row 0x09: the chip destruction shot (collision row 0x08, whose flag 0x10 destroys the held chip). The user's name. |

### Rock variants (chips/rockcube/rock.luau, compat/records.toml) (4)

| current | proposed | evidence |
|---|---|---|
| `rockcube/brittle` | `rockcube/rock/brittle` *applied* | The rock's variant (a rock cube anything breaks): a record of the kind `rockcube/rock`. `rockcube/cube` is also the key of the kind RockCube's controller (chips/rockcube/cube.luau): legal, since the registries differ, but one key naming two things misleads a reader and a search. Naming the variants under the rock they vary keeps them owner-qualified and apart; the other three follow for consistency. |
| `rockcube/cube` | `rockcube/rock/cube` (wrong now) *applied* | The rock's variant (RockCube's and the stages' cube): a record of the kind `rockcube/rock`. `rockcube/cube` is also the key of the kind RockCube's controller (chips/rockcube/cube.luau): legal, since the registries differ, but one key naming two things misleads a reader and a search. Naming the variants under the rock they vary keeps them owner-qualified and apart; the other three follow for consistency. |
| `rockcube/hard` | `rockcube/rock/hard` *applied* | The rock's variant (a hard cube): a record of the kind `rockcube/rock`. `rockcube/cube` is also the key of the kind RockCube's controller (chips/rockcube/cube.luau): legal, since the registries differ, but one key naming two things misleads a reader and a search. Naming the variants under the rock they vary keeps them owner-qualified and apart; the other three follow for consistency. |
| `rockcube/ice` | `rockcube/rock/ice` *applied* | The rock's variant (the ice block): a record of the kind `rockcube/rock`. `rockcube/cube` is also the key of the kind RockCube's controller (chips/rockcube/cube.luau): legal, since the registries differ, but one key naming two things misleads a reader and a search. Naming the variants under the rock they vary keeps them owner-qualified and apart; the other three follow for consistency. |

## Medium confidence (157)

Named from several uses that agree, from the look (the effects named for sprite, animation and palette rather than the original's effect number), or from a definition's shape or flags (regions, collision types). Safe to accept; a better name may exist.

### Sprites (10)

| current | proposed | evidence |
|---|---|---|
| `bat-impact` | `hit-marker` (wrong now) *applied* | The role sprites.hit_marker (the "!!" over a navi), the cut-in flash (effect 0x1E) and BatCan's spark (animation 1). Shows bursts, "!!" marks and bats: BatCan is one user of three. |
| `colonel-effect` | `colonel-slashes` | Shows green and white slashes: Colonel's (chips/colonel), his screen divide among them. |
| `elmnt-ice` | `sparkle` | Shows twinkling sparkles: Roll's heart's sparkle, ElmntMan's ice, effect 0x26. |
| `hit` | `guard-ripple` | Shows cyan ripples. Its one spark (0x08) is the role sparks.guard, a blocked hit. |
| `immobilized` | `rock-cubes` (wrong now) *applied* | Shows the rock cube, the rock and the ice cube: the rocks' and cubes' sheet (chips/rockcube/rock.luau), the encased bubble and WideSht's trail. The immobilized status visual (a role) is one animation of it. |
| `reflected-shot` | `pink-flash` (wrong now) *applied* | Shows a pink starburst. The effect `flash` (0x21), CrcusMan's sparkle, H-Burst's explosion and DblBeast's spark use it. |
| `rising-bubble` | `small-puff` (wrong now) *applied* | Shows small gray puffs. lib/effects' splash, dust and ripple and Geddon's puffs use it; nothing rises as a bubble. |
| `slash-man-effect` | `claw-slash` | Shows cyan claw slashes: SlashMan's slash and DblBeast's Gregar claw and slam. |
| `small-ring` | `wide-navi` (wrong now) *applied* | The ROM's dummy sprite (a 16x16 dot, like 40 unused slots): it shows no ring. SumnBlk asks whether an actor wears it to know it stands two panels wide. |
| `spark-0e` | `uninstall-spark` | Its spark (0x0E) is the role sparks.uninstall: a navi's programs uninstalled. |

### Sounds (34)

| current | proposed | evidence |
|---|---|---|
| `aqua-needle` | `needle-land` | AquaNdl's needle landing. |
| `beast-over-burst` | `thunder` | ElecMan's thunder, ElemTrap's and ElmntMan's bolts, Guardian's strike; also the role sounds.beast_over_burst. Its sprite shows lightning (proposed `lightning`). |
| `col-army` | `army-appear` | ColArmy's soldiers appearing. |
| `col-army-2` | `rifle-shot` | ColArmy's and ColForce's shots, CircGun's hit. |
| `copy-mark` | `mark` (wrong now) *applied* | A mark set: BurnSqr's fire, CircGun's and CopyDmg's marks. CopyDmg is one user of three. |
| `dimming-sparkle` | `cut-in` | The role sounds.cut_in: a side cuts in, a trap springs. |
| `drilarm` | `drill` | DrilArm, MstrCros's drill, DblBeast's Gregar bite. |
| `dustbrk` | `suction` | DustMan drawing things in (DustBrk), Wind's fan blowing. |
| `dustbrk-2` | `crush` | DustMan crushing what he drew in (DustBrk). |
| `elmnt-man` | `element-cycle` | ElmntMan's element changing while he cycles (chips/elmntman/navi.luau, `cycle`). |
| `elmnt-man-2` | `element-chosen` | ElmntMan's element chosen (A pressed); also the role sounds.cross_special. |
| `elmnt-man-4` | `ice-form` | ElmntMan's Aqua attack: ice forming in the column ahead. |
| `erase-man` | `cursor-step` | A cursor stepping: BurnSqr's and CircGun's cursors, EraseMan's mark. |
| `err-select-91` | `error` | The disassembly's SOUND_ERR_SELECT_91: GrabBnsh's return, LifeSync's warning, VDoll's marks. |
| `etomahwk` | `obstacle-throw` | The role sounds.obstacle_throw; ETomahwk's throw. |
| `falling-rock` | `rock-break` | A rock breaking: the falling rocks, GroundMan's rocks, a rock, an encased bubble bursting. |
| `golem` | `golem-hit` | GolmHit's golem hitting. |
| `golem-2` | `golem-leap` | GolmHit's golem leaping. |
| `grab-shot` | `grab-land` | The grab shot landing (lib/grab/shot.luau's SOUND_LAND), GrabBnsh's hand. |
| `grab-shot-2` | `grab-fall` | The grab shot falling (lib/grab/shot.luau's SOUND_FALL), GrabBnsh's hand. |
| `ground-beast-dash-2` | `rumble` | GroundCross Beast's dash's rumble, ChargeCross Beast's wave. |
| `gundels1` | `gun-click` | GunDelSol's gun, CrosOver's gun, the blank shot's click (named for GunDelS1). |
| `justice-one` | `heavy-impact` | A heavy landing: JustcOne's fist, MetrKnuk's fists, ElmntMan's and SunMoon's meteors. |
| `log-out-76` | `log-out` | The disassembly's SOUND_LOG_OUT_76; the battle doesn't play it. |
| `minibomb-throw` | `bomb-throw` | Every bomb's throw (lib/bombs/throw.luau) and Tango's. |
| `ok-8b` | `ok` | The disassembly's SOUND_OK_8B: SlashMan's wave, Z Saver's command, VariSwrd. |
| `rlnglog1-roll` | `log-roll` | RlngLog's log rolling (named for RlngLog1). |
| `roll` | `roll-warp` | Roll's warp (chips/roll/navi.luau, `vanish`). |
| `roll-2` | `whip` | Roll's ribbon whip swishing. |
| `sand-worm` | `emerge` | SandWrm's worm emerging, DiveMan surfacing. |
| `sand-worm-2` | `dive` | SandWrm's worm diving. |
| `spout-man` | `geyser` | SpoutMan's geyser erupting. |
| `winner-0` | `winner-special` | The disassembly's SONG_WINNER_0; the role music.winner_special. |
| `winner-1` | `winner` | The disassembly's SONG_WINNER_1; the role music.winner. |

### Effects (lib/effects.luau) (35)

| current | proposed | evidence |
|---|---|---|
| `big_slash_1` | `earth_splash` | Effect 0x33: the earth TomahawkMan's strike splashes (big-slash animation 1). |
| `big_slash_1c` | `big_slash_p2` | Effect 0x1C: big-slash, palette 2. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `charge_glow_2` | `charged_flash` | Effect 0x4F: the flash when a charge is full (docs/engine/chips.md). |
| `colonel_effect` | `colonel_slashes` | Effect 0x35: colonel-slashes animation 0. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `colonel_effect_1` | `colonel_slashes_1` | Effect 0x36: colonel-slashes animation 1. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `colonel_effect_2` | `colonel_slashes_2` | Effect 0x37: colonel-slashes animation 2. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `colonel_effect_3` | `screen_divide` | Effect 0x38: Colonel's screen divide (chips/colonel/navi.luau's DIVIDE). |
| `colonel_effect_5` | `colonel_slashes_5` | Effect 0x62: colonel-slashes animation 5. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `copy_mark` | `hit_spark` | Effect 0x07: the plain hit spark's look (hit-sparks animation 0). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `copy_mark_1` | `breaking_spark` | Effect 0x0C: the breaking spark's look (hit-sparks animation 1). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `copy_mark_11` | `target_mark` | Effect 0x28: DeltaRay's mark on his target (chips/deltaray). Hit-sparks animation 11, palette 1. |
| `copy_mark_2` | `fire_spark` | Effect 0x08: the fire spark's look (animation 2). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `copy_mark_3` | `aqua_spark` | Effect 0x09: the aqua spark's look (animation 3). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `copy_mark_4` | `elec_spark` | Effect 0x0A: the elec spark's look (animation 4). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `copy_mark_5` | `wood_spark` | Effect 0x0B: the wood spark's look (animation 5). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `copy_mark_8` | `hand_flash` | Effect 0x57: a shot's flash at the hand (Bass, chips/bass/navi.luau). |
| `dust_2` | `crack_dust` | Effect 0x30: the dust CrakShot raises on its panel. |
| `elmnt_ice` | `sparkle` | Effect 0x26: the sparkle sprite (proposed `sparkle`). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `explosion_24` | `explosion_p1` | Effect 0x24: explosion, palette 1. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `explosion_4a` | `explosion_p2` | Effect 0x4A: explosion, palette 2 (BugBomb's). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `grab_shot_1_3c` | `grab_shot_1_p8` | Effect 0x3C: grab-shot animation 1, palette 8. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `hit` | `guard_ripple` | Effect 0x0F: the guard ripple's look (sprite `hit`, proposed guard-ripple). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `reflected_shot_3d` | `pink_flash_p2` | Effect 0x3D: pink-flash, palette 2 (H-Burst's explosion on each panel). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `slash_man_effect` | `claw_slash` | Effect 0x39: claw-slash animation 0. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `slash_man_effect_1` | `claw_slash_1` | Effect 0x3A: claw-slash animation 1. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `slash_man_effect_2` | `claw_slash_2` | Effect 0x4C: claw-slash animation 2. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `slash_man_effect_3` | `claw_slash_3` | Effect 0x4D: claw-slash animation 3. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `sword_slash_19` | `sword_slash_p5` | Effect 0x19: sword-slash, palette 5. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `sword_slash_1_1a` | `sword_slash_1_p5` | Effect 0x1A: sword-slash animation 1, palette 5. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `sword_slash_1_2d` | `sword_slash_1_p6` | Effect 0x2D: sword-slash animation 1, palette 6. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `sword_slash_1_65` | `sword_slash_1_p4` | Effect 0x65: sword-slash animation 1, palette 4. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `sword_slash_3_67` | `sword_slash_3_p4` | Effect 0x67: sword-slash animation 3, palette 4. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `sword_slash_64` | `sword_slash_p6` | Effect 0x64: sword-slash, palette 6. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `sword_slash_66` | `sword_slash_p4` | Effect 0x66: sword-slash, palette 4. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `vulcan_hit_6a` | `vulcan_hit_p1` | Effect 0x6A: vulcan-hit, palette 1. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |

### Regions (content/exelib/regions.luau) (28)

| current | proposed | evidence |
|---|---|---|
| `region_03` | `ahead` | Region 0x03: the panel ahead, not the anchor. Unused by content; named for its shape. |
| `region_05` | `above_below` | Region 0x05: the panels above and below the anchor. Unused by content; named for its shape. |
| `region_07` | `line_4` | Region 0x07: the anchor and three ahead (`line` is three, `long` two). Unused by content; named for its shape. |
| `region_08` | `line_6` | Region 0x08: the anchor and five ahead. Unused by content; named for its shape. |
| `region_0e` | `diagonals_ahead` | Region 0x0E: the two panels diagonally ahead (the `fork` without its anchor). Unused by content; named for its shape. |
| `region_14` | `tee` | Region 0x14: the anchor, above, below and ahead: `wide` and the panel ahead. Unused by content; named for its shape. |
| `region_16` | `up_ahead` | Region 0x16: the anchor and the panel diagonally ahead and up. Unused by content; named for its shape. |
| `region_17` | `down_ahead` | Region 0x17: the anchor and the panel diagonally ahead and down. Unused by content; named for its shape. |
| `region_19` | `tall_down` | Region 0x19: the anchor and the panel below (`tall` is the one above). Unused by content; named for its shape. |
| `region_1b` | `fan_3` | Region 0x1B: the anchor and the three columns ahead (`fan` is one, `deep_fan` two). Unused by content; named for its shape. |
| `region_1c` | `fan_4` | Region 0x1C: the anchor and the four columns ahead. Unused by content; named for its shape. |
| `region_1d` | `across` | Region 0x1D: the anchor, ahead and behind. Unused by content; named for its shape. |
| `region_1e` | `across_ends` | Region 0x1E: ahead and behind, not the anchor. Unused by content; named for its shape. |
| `region_1f` | `fork_back` | Region 0x1F: the anchor and the two panels diagonally behind (`fork` reversed). Unused by content; named for its shape. |
| `region_20` | `line_5` | Region 0x20: the anchor and four ahead. Unused by content; named for its shape. |
| `region_21` | `wide_long_4` | Region 0x21: the anchor's column and three ahead (`wide_long` is two). Unused by content; named for its shape. |
| `region_22` | `hourglass` | Region 0x22: three above, the anchor, three below. Unused by content; named for its shape. |
| `region_23` | `wide_long_6` | Region 0x23: the anchor's column and five ahead: three rows by six. Unused by content; named for its shape. |
| `region_24` | `rising_band` | Region 0x24: a band of three by three leaning forward and up. Unused by content; named for its shape. |
| `region_25` | `falling_band` | Region 0x25: a band of three by three leaning forward and down. Unused by content; named for its shape. |
| `region_27` | `bracket` | Region 0x27: the rows above and below, six long, closed at the far end. Unused by content; named for its shape. |
| `region_28` | `hammer` | Region 0x28: three ahead in line, with the far end's above and below. Unused by content; named for its shape. |
| `region_29` | `hourglass_ahead` | Region 0x29: three above and below, starting at the anchor's column, and the panel ahead between. Unused by content; named for its shape. |
| `region_2a` | `long_tee` | Region 0x2A: above, below, the anchor and five ahead. Unused by content; named for its shape. |
| `region_2b` | `line_back` | Region 0x2B: the anchor and two behind. Unused by content; named for its shape. |
| `region_2c` | `rows_two_away` | Region 0x2C: the rows two above and two below, eleven long. Unused by content; named for its shape. |
| `region_2d` | `column` | Region 0x2D: the anchor's column, five tall. Unused by content; named for its shape. |
| `region_2e` | `column_ends` | Region 0x2E: the anchor's column, five tall, without the anchor. Unused by content; named for its shape. |

### Collision types (rules/collision.luau) (32)

| current | proposed | evidence |
|---|---|---|
| `collision-1a` | `thrown-piercing-break-chip-destruction` | Row 0x1A: a thrown attack that pierces, breaks and destroys the held chip (chip destruction, 0x10). Unused by content; named for its flags. |
| `collision-1d` | `thrown-piercing-chip-destruction` | Row 0x1D: a thrown attack that pierces and destroys the held chip. Unused by content; named for its flags. |
| `collision-20` | `thrown-pushing` | Row 0x20: a thrown attack that pushes. Unused by content; named for its flags. |
| `collision-23` | `blocker` | Row 0x23: only the blocker bit (0x00080000). Unused by content; named for its flags. |
| `collision-26` | `both-sides-thrown` | Row 0x26: a thrown attack of both sides (with the bit 0x40000). Unused by content; named for its flags. |
| `collision-27` | `both-sides-ground-breaking` | Row 0x27: an attack of both sides (with 0x40000), ground-only and breaking. Unused by content; named for its flags. |
| `collision-28` | `piercing-break-slash` | Row 0x28: a slash that pierces and breaks. Unused by content; named for its flags. |
| `collision-29` | `own-breaking-body` | Row 0x29: another body of its side that breaks, while dimmed too. Unused by content; named for its flags. |
| `collision-2d` | `thrown-break-drain` | Row 0x2D: a thrown breaking drain. Unused by content; named for its flags. |
| `collision-2e` | `piercing-break-drain` | Row 0x2E: a piercing breaking drain. Unused by content; named for its flags. |
| `collision-2f` | `thrown-drain` | Row 0x2F: a thrown drain. Unused by content; named for its flags. |
| `collision-31` | `piercing-guard-breaking` | Row 0x31: a piercing attack that breaks guards. Unused by content; named for its flags. |
| `collision-35` | `breaking-navi` | Row 0x35: a navi's body that breaks (the navi chips' and link navis' bodies). Unused by content; named for its flags. |
| `collision-37` | `own-thrown-breaking-body` | Row 0x37: another body of its side, thrown and breaking, while dimmed too. Unused by content; named for its flags. |
| `collision-38` | `hits-bodies` | Row 0x38: what reaches every body and the neutral objects. Unused by content; named for its flags. |
| `collision-39` | `breaking-floating-navi` | Row 0x39: a floating navi's body that breaks. Unused by content; named for its flags. |
| `collision-3a` | `thrown-breaking-navi` | Row 0x3A: a navi's body, thrown and breaking. Unused by content; named for its flags. |
| `collision-3b` | `own-breaking-object` | Row 0x3B: an object of its side that breaks, while dimmed too. Unused by content; named for its flags. |
| `collision-3c` | `thrown-break-drain-2` | Row 0x3C: the same flags as row 0x2D (a thrown breaking drain): a second row. Unused by content; named for its flags. |
| `collision-3e` | `own-floating-body` | Row 0x3E: another body of its side that floats, while dimmed too. Unused by content; named for its flags. |
| `collision-3f` | `thrown-breaking-floating-navi` | Row 0x3F: a floating navi's body, thrown and breaking. Unused by content; named for its flags. |
| `collision-40` | `own-slash-object` | Row 0x40: an object of its side that slashes, while dimmed too. Unused by content; named for its flags. |
| `collision-41` | `other-side-body` | Row 0x41: only the body bit of side 1 (0x04000000) for side 0, and the reverse. Unused by content; named for its flags. |
| `collision-42` | `chip-destruction-slash` | Row 0x42: a slash that destroys the held chip. Unused by content; named for its flags. |
| `collision-45` | `thrown-break-slash` | Row 0x45: a thrown breaking slash. Unused by content; named for its flags. |
| `collision-4c` | `own-breaking-drain-body` | Row 0x4C: another body of its side, breaking and draining, while dimmed too. Unused by content; named for its flags. |
| `collision-50` | `slash-navi` | Row 0x50: a navi's body that slashes. Unused by content; named for its flags. |
| `collision-51` | `chip-destruction-breaking-navi` | Row 0x51: a navi's body that breaks and destroys the held chip. Unused by content; named for its flags. |
| `collision-53` | `breaking-slash-navi` | Row 0x53: a navi's body that slashes and breaks. Unused by content; named for its flags. |
| `collision-54` | `chip-destruction-breaking-slash` | Row 0x54: a breaking slash that destroys the held chip. Unused by content; named for its flags. |
| `collision-55` | `thrown-chip-destruction-breaking-navi` | Row 0x55: a navi's body, thrown, breaking, destroying the held chip. Unused by content; named for its flags. |
| `collision-56` | `piercing-pushing` | Row 0x56: a piercing pushing attack. Unused by content; named for its flags. |

### Statuses (rules/status.luau) (14)

| current | proposed | evidence |
|---|---|---|
| `collision-panel-65535` | `after-table-1-past-freeze` (wrong now) *applied* | Status 0x5C: the freeze group's entry 12, past the bubble rows it also reads, reads row 1 after the table: its fields are whatever bytes follow the table, which the current name spells out as if they meant something. Named as the other overflow reads are (`<what it reads>-past-<group>`). |
| `collision-panel-65535-2` | `after-table-1-past-bubble` (wrong now) *applied* | Status 0x66: the bubble group's entry 6 reads row 1 after the table (the same bytes as the freeze group's overflow, for the first four). |
| `timer-00-150` | `after-table-5-past-bubble` (wrong now) *applied* | Status 0x6A: the bubble group's entry 10 reads row 5 after the table (the same bytes as the freeze group's overflow, for the first four). |
| `timer-00-200` | `after-table-8-past-bubble` (wrong now) *applied* | Status 0x6D: the bubble group's entry 13 reads row 8 after the table (the same bytes as the freeze group's overflow, for the first four). |
| `timer-00-30` | `after-table-2-past-freeze` (wrong now) *applied* | Status 0x5D: the freeze group's entry 13, past the bubble rows it also reads, reads row 2 after the table: its fields are whatever bytes follow the table, which the current name spells out as if they meant something. Named as the other overflow reads are (`<what it reads>-past-<group>`). |
| `timer-00-30-2` | `after-table-2-past-bubble` (wrong now) *applied* | Status 0x67: the bubble group's entry 7 reads row 2 after the table (the same bytes as the freeze group's overflow, for the first four). |
| `timer-01-65535` | `after-table-7-past-bubble` (wrong now) *applied* | Status 0x6C: the bubble group's entry 12 reads row 7 after the table (the same bytes as the freeze group's overflow, for the first four). |
| `timer-01-65535-2` | `after-table-10-past-bubble` (wrong now) *applied* | Status 0x6F: the bubble group's entry 15 reads row 10 after the table (the same bytes as the freeze group's overflow, for the first four). |
| `timer-64-65535` | `after-table-4-past-freeze` (wrong now) *applied* | Status 0x5F: the freeze group's entry 15, past the bubble rows it also reads, reads row 4 after the table: its fields are whatever bytes follow the table, which the current name spells out as if they meant something. Named as the other overflow reads are (`<what it reads>-past-<group>`). |
| `timer-64-65535-2` | `after-table-4-past-bubble` (wrong now) *applied* | Status 0x69: the bubble group's entry 9 reads row 4 after the table (the same bytes as the freeze group's overflow, for the first four). |
| `timer-ff-0` | `after-table-3-past-freeze` (wrong now) *applied* | Status 0x5E: the freeze group's entry 14, past the bubble rows it also reads, reads row 3 after the table: its fields are whatever bytes follow the table, which the current name spells out as if they meant something. Named as the other overflow reads are (`<what it reads>-past-<group>`). |
| `timer-ff-0-2` | `after-table-3-past-bubble` (wrong now) *applied* | Status 0x68: the bubble group's entry 8 reads row 3 after the table (the same bytes as the freeze group's overflow, for the first four). |
| `timer-ff-0-3` | `after-table-6-past-bubble` (wrong now) *applied* | Status 0x6B: the bubble group's entry 11 reads row 6 after the table (the same bytes as the freeze group's overflow, for the first four). |
| `timer-ff-100` | `after-table-9-past-bubble` (wrong now) *applied* | Status 0x6E: the bubble group's entry 14 reads row 9 after the table (the same bytes as the freeze group's overflow, for the first four). |

### Lock-on modes (rules/lockon.luau) (4)

| current | proposed | evidence |
|---|---|---|
| `elcpuls1` | `elcpuls` | Named for the first chip that uses it (elcpuls1); its other levels use it too, so the level digit says nothing. |
| `firebrn1` | `firebrn` | Named for the first chip that uses it (firebrn1); its other levels use it too, so the level digit says nothing. |
| `gundels1` | `gundels` | Named for the first chip that uses it (gundels1); its other levels use it too, so the level digit says nothing. |
| `trnarrw1` | `trnarrw` | Named for the first chip that uses it (trnarrw1); its other levels use it too, so the level digit says nothing. |

## Low confidence (20)

The evidence is thin: a sound with mixed users, a look with no user. A proposal that keeps the current name says nothing better was found.

### Sprites (4)

| current | proposed | evidence |
|---|---|---|
| `dust-2` | `dust-spray` | Shows white and blue sprays: GolmHit's golem landing and TomahawkMan's strike (effect 0x34). |
| `effect-61` | `gray-shards` | Shows gray shard shapes; only effect 0x61 uses it. |
| `reflector-shield-2` | `dummy-shield` (wrong now) *applied* | The ROM's dummy sprite (a 16x16 dot). Rflectr's rows 4-6 name it, which nothing ported holds up. |
| `spout-man-effect` | `water-ring` | Shows a splash of water rings: SpoutMan's throw and charge (effect 0x2A). |

### Sounds (7)

| current | proposed | evidence |
|---|---|---|
| `aqua-needle-2` | `drop` | AquaNdl's needle falling, SlashMan's wave, Snake's leap. |
| `bblstar1` | `blow` | BblStar's blow, CrakShot, WaveArm's strike (named for BblStar1). |
| `delta-ray-2` | `delta-burst` | DeltaRay's bursts and their flash. |
| `erase-man-2` | `beam` | EraseMan's slash, JudgeMan's lash, MstrCros's beam, Thunder's ball. |
| `follow-effect` | `slide` | The follow effect sliding, Rflectr's shield. |
| `hub` | `power-up` | BusterUp, LifeSync's HP going up, the navi boosts' hub. |
| `tomahawk-man` | `axe-swing` | TomahawkMan's swing (chip and charge), TomahawkCross's charge, SumnBlk's claw. |

### Effects (lib/effects.luau) (9)

| current | proposed | evidence |
|---|---|---|
| `copy_mark_12` | `hit_sparks_12` | Effect 0x48: hit-sparks animation 12 (BurnSqr's corners). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `copy_mark_1_53` | `breaking_spark_p6` | Effect 0x53: hit-sparks animation 1, palette 6. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `copy_mark_9` | `hit_sparks_9` | Effect 0x0D: hit-sparks animation 9. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `dust_2_2` | `dust_spray` | Effect 0x34: the dust-spray sprite (GolmHit's landing). |
| `effect_61` | `gray_shards` | Effect 0x61: the gray-shards sprite. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `reflected_shot_1` | `pink_flash_1` | Effect 0x10: pink-flash animation 1. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `small_ring_82` | `dummy_look` (wrong now) *applied* | Effect 0x4B reads animation 82, palette 16 of the dummy sprite: a row of the original's table that names no real look. |
| `spout_man_effect` | `water_ring` | Effect 0x2A: the water-ring sprite (SpoutMan's throw). Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |
| `swirl_1` | `swirl_1_p1` | Effect 0x41: swirl animation 1, palette 1. Unused by content; named for its look (sprite, animation, palette), the effect number dropped. |

## Names to keep (636)

The rest read well for what they are: an asset named for its one user or what it shows, a definition named as
its module's convention names it (a status by group and duration, a lock-on mode by its first chip, a weapon by its
owner). Accepting them settles them; the evidence for each is in the .tsv. Those that carry a number, or keep a
proposal-worthy name for lack of a better one, are listed with their reason.

### Sprites (169)

`air-raid-plane`, `air-shooter`, `air-spin`, `anubis`, `aqua-needle`, `aqua-surge`, `aqua-sword`, `aura-head`, `bass-anly`, `bass-anly-shot`, `bat-cannon`, `beast-head`, `beast-shot`, `beat`, `big-slash`, `black-bomb`, `blast`, `blast-fire`, `blindness`, `blizzard-ball`, `bomb`, `boomerang`, `boomerang-tomahawk`, `boulder`, `bow`, `bubble`, `bubble-star`, `bug`, `burner`, `burst`, `buster-arm`, `buster-up`, `cannon`, `charge-car`, `charge-glow`, `charge-glow-a`, `charge-wave`, `charged-slash`, `circusman`, `col-army`, `colonel`, `colonel-sword`, `confusion`, `corn-shooter`, `countdown-bomb`, `crack-shot`, `cross-slash`, `dark-aura`, `deletion`, `diveman`, `django`, `dragon`, `drill-arm`, `drip-shower`, `dust`, `dust-cloud`, `dust-storm-mote`, `eagle-tomahawk`, `elec-coil`, `elec-pulse`, `elec-sword`, `element-pillar-flames`, `element-pillar-lightning`, `elmnt-man`, `energy-burst`, `erase-beam`, `erase-drop`, `erase-mark`, `erase-ray`, `eruption`, `explosion`, `falling-rock`, `fan`, `fire-hit`, `fire-sword`, `flame`, `flame-hook-fire`, `flash-bomb`, `follow-effect`, `form-change`, `full-synchro-aura`, `golem`, `grab-shot`, `ground-drill`, `ground-drill-effect`, `guardian-statue`, `gun-del-sol`, `gust`, `hand-fan`, `heal`, `heat-flame`, `hive`, `hockey-puck`, `hub`, `ice-block`, `idle-overlay`, `impact`, `instrument`, `iron-shell`, `jet-flame`, `judgeman`, `junk-shot`, `justice-one`, `lance`, `land-mine`, `lil-boiler`, `lockon-marker`, `log-thrower`, `machine-gun`, `magnet`, `magnet-coil`, `meteor`, `moon-beam`, `moon-blade`, `muzzle-flash`, `number-ball`, `panel-strike`, `propeller`, `puff`, `reflector-shield`, `reticle`, `rock-debris`, `roll-heart`, `rolling-log`, `rush`, `sand-hole`, `sand-worm`, `sensor`, `shell-burst`, `shock-wave`, `shuriken`, `sight`, `slash-wave`, `snake`, `spout-geyser`, `spout-pillar`, `spout-splash`, `spreader`, `summon-black`, `sun-beam`, `sun-beam-ex`, `swirl`, `sword`, `sword-slash`, `tango`, `tango-heal`, `tank-blast`, `tank-cannon`, `tengu-fan`, `tengu-tornado`, `thunder-ball`, `thunder-doll`, `thunder-doll-hand`, `tornado`, `volcano-rock`, `voodoo-doll`, `vulcan`, `vulcan-hit`, `water-cannon`, `whirlwind`, `wide-shooter`, `wide-wave`, `wind-rack`, `yoyo`, `yoyo-arm`.

| name | confidence | why it stays |
|---|---|---|
| `effect-55` | low | A virus's sheet (a green creature with blades); only effect 0x55 uses it (animation 12). No use names it better. |
| `follow-effect-2` | medium | Unused by content (an orange ball and a white burst); named for the follow effect's looks, like follow-effect. |
| `follow-effect-4` | medium | Unused by content (pink marks); named for the follow effect's looks. |
| `honey-bee` | high | 10-31: used by chips/rskyhny/bee. |

### Sounds (116)

`appear`, `aqua-surge`, `arrive`, `bass-anly`, `bass-anly-shot`, `beast-claw`, `beast-out`, `beast-over`, `big-sword-swing`, `bite`, `blast-man`, `blinding-flash`, `blizzard-grow`, `blizzard-throw`, `blizzard-windup`, `boiler-erupt`, `boiler-steam`, `bonus`, `boomerang`, `bounce`, `bubble`, `bubble-pop`, `bug`, `bug-bomb-land`, `burst`, `buster-charged`, `buzz`, `cannon`, `charge-man`, `charge-train`, `circus-catch`, `confusion`, `corn-shot`, `crack-shot`, `cross-change`, `cross-merge`, `damage`, `drill-launch`, `drill-out`, `drill-spin`, `drip-shower`, `drumroll`, `elec-man`, `elec-pulse`, `elmnt-vine`, `energy-burst`, `falzar-roar`, `fire-hit`, `flame-hook-fire`, `flash`, `form-change`, `freeze`, `gauge-full`, `gregar-roar`, `ground-beast-dash`, `h-burst`, `heal`, `hockey-puck`, `hop`, `immobilizer`, `invisible`, `iron-shell`, `junk-shot`, `land`, `last`, `lifesync`, `moon-beam`, `panel-crack`, `panel-poison`, `panel-volcano`, `pause`, `place`, `point-appear`, `point-rise`, `roar`, `rockfall`, `rslash`, `set-down`, `shock-wave`, `snatch`, `spin`, `spout-ball`, `spout-beast-charge`, `stop-music`, `sun-beam`, `sun-moon`, `swoop`, `sword-swing`, `take-off`, `tango-land`, `target-move`, `telop`, `tengu-man`, `tenguman-nose`, `throw`, `tick`, `trap-vanish`, `twang`, `volcano`, `wave`, `wide-shot`, `windrack`, `z-saver`.

| name | confidence | why it stays |
|---|---|---|
| `beast-claw-2` | high | 0x1c6: used by navis/megaman/weapons/beast-claw/init. |
| `beep-64` | high | The disassembly's SOUND_BEEP_64: a menu sound the battle doesn't play; the number tells its variants apart. |
| `beep-75` | high | The disassembly's SOUND_BEEP_75: a menu sound the battle doesn't play; the number tells its variants apart. |
| `cur-move-80` | high | The disassembly's SOUND_CUR_MOVE_80: a menu sound the battle doesn't play; the number tells its variants apart. |
| `hit-bomb-0` | high | 0x06f: used by chips/vdoll/sparkles. |
| `hit-bomb-1` | high | 0x070: used by chips/airraid/plane, chips/airspin/top, chips/anubis/statue, chips/blkbomb/bomb and 16 more. |
| `select-67` | high | The disassembly's SOUND_SELECT_67: a menu sound the battle doesn't play; the number tells its variants apart. |
| `select-79` | high | The disassembly's SOUND_SELECT_79: a menu sound the battle doesn't play; the number tells its variants apart. |
| `select-7a` | high | The disassembly's SOUND_SELECT_7A: a menu sound the battle doesn't play; the number tells its variants apart. |
| `select-82` | high | The disassembly's SOUND_SELECT_82: a menu sound the battle doesn't play; the number tells its variants apart. |
| `unselect-68` | high | The disassembly's SOUND_UNSELECT_68: a menu sound the battle doesn't play; the number tells its variants apart. |
| `unselect-7b` | high | The disassembly's SOUND_UNSELECT_7B: a menu sound the battle doesn't play; the number tells its variants apart. |
| `unselect-7c` | high | The disassembly's SOUND_UNSELECT_7C: a menu sound the battle doesn't play; the number tells its variants apart. |

### Backgrounds (21)

`calendar-blue`, `calendar-bright-blue`, `calendar-checkers`, `calendar-cyan`, `calendar-green`, `calendar-lavender`, `calendar-mint`, `calendar-navy`, `calendar-purple`, `clouds`, `code`, `globes`, `honeycomb`, `seals`, `sprouts`, `statues`, `storm-clouds`, `swirls`, `trees`.

| name | confidence | why it stays |
|---|---|---|
| `code-2` | high | 0x11: no use in content (the engine's or nothing's). Curation: 0x11: by what it shows |
| `globes-2` | high | 0x15: no use in content (the engine's or nothing's). Curation: 0x15: by what it shows |

### Effects (lib/effects.luau) (46)

`beast_over_blast`, `beast_over_falzar`, `beast_over_gregar`, `big_slash`, `blast`, `blast_fire_12`, `bug`, `buster_up`, `charge_glow`, `charge_glow_a_3`, `cross_slash`, `deletion`, `diveman_21`, `diveman_25`, `dust_cloud`, `dustman_24`, `dustman_30`, `energy_burst`, `form_change`, `grab_shot_1`, `ground_drill_effect`, `heal`, `hub`, `hub_1`, `impact`, `lil_boiler_10`, `moon_blade`, `panel_strike`, `puff_1`, `puff_3`, `reticle`, `reticle_1`, `sand_hole`, `sight`, `sight_4`, `spout_splash`, `sun_beam`, `swirl`, `sword_slash`, `sword_slash_1`, `sword_slash_2`, `sword_slash_3`, `tank_blast`, `tornado`, `vulcan_hit`.

| name | confidence | why it stays |
|---|---|---|
| `effect_55_12` | low | Effect 0x55: a virus sheet's animation 12. Nothing names it better. |

### Collision types (rules/collision.luau) (17)

`collision-1b`.

| name | confidence | why it stays |
|---|---|---|
| `attack-480` | low | Row 0x25: an attack with the flag 0x400, whose test no routine was found for; nothing names a better reading. |
| `attack-a0` | low | Row 0x47: an attack with the flag 0x20 (untested by any routine found); a projectile variant uses it. |
| `collision-03` | low | Row 0x03: what reaches the other side's attacks, objects and other bodies and the neutral objects, with 0x100. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-09` | low | Row 0x09: an attack with the flag 0x100 (counted into the receiver's drain total). Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-11` | low | Row 0x11: a slash with the flag 0x400, whose test no routine was found for. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-1f` | low | Row 0x1F: a breaking attack with the bit 0x40000 (the mine's and the hitbox's). Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-21` | low | Row 0x21: a thrown breaking attack with the bit 0x40000. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-22` | low | Row 0x22: an attack of both sides with the bit 0x40000, ground-only, thrown, piercing and breaking: the mine's, ground-only. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-24` | low | Row 0x24: a thrown pushing attack with the flag 0x400. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-36` | low | Row 0x36: a breaking drain (0x4000 and 0x02 alone); `breaking-drain` already names row 0x30, which is thrown and piercing too. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-43` | low | Row 0x43: the bit 0x40000 and 0x80, no side. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-44` | low | Row 0x44: an object of its side with the bit 0x40000, while dimmed too. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-46` | low | Row 0x46: a drain with the flag 0x20, while dimmed too. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-4b` | low | Row 0x4B: the same flags as row 0x24: a second row. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-4d` | low | Row 0x4D: what a navi hits, but the neutral objects (row 0x02 without 0x00800000). Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |
| `collision-57` | low | Row 0x57: a breaking attack that destroys the held chip, with the flag 0x20. Unused by content; a flag whose test no routine was found for leaves no clear name, so it keeps its row. |

### Statuses (rules/status.luau) (82)

`blind-1200`, `blind-1200-past-confuse`, `blind-1200-past-paralyze`, `blind-300`, `blind-300-past-confuse`, `blind-4`, `blind-4-past-confuse`, `blind-4-past-paralyze`, `blind-480`, `blind-480-past-confuse`, `blind-480-past-paralyze`, `blind-600`, `blind-600-past-confuse`, `blind-720`, `blind-720-past-confuse`, `blind-720-past-paralyze`, `bubble-150`, `bubble-150-2`, `bubble-150-2-past-freeze`, `bubble-150-2-past-immobilize`, `bubble-150-3`, `bubble-150-3-past-freeze`, `bubble-150-3-past-immobilize`, `bubble-150-past-freeze`, `bubble-150-past-immobilize`, `bubble-300`, `bubble-300-past-freeze`, `bubble-4`, `bubble-4-past-freeze`, `bubble-600`, `bubble-600-past-freeze`, `confuse-300`, `confuse-300-past-paralyze`, `confuse-4`, `confuse-4-past-paralyze`, `confuse-480`, `confuse-480-past-paralyze`, `confuse-600`, `confuse-600-past-paralyze`, `confuse-720`, `confuse-720-past-paralyze`, `confuse-960`, `confuse-960-past-paralyze`, `freeze-150`, `freeze-150-2`, `freeze-150-2-past-blind`, `freeze-150-2-past-immobilize`, `freeze-150-3`, `freeze-150-3-past-blind`, `freeze-150-3-past-immobilize`, `freeze-150-past-blind`, `freeze-150-past-immobilize`, `freeze-300`, `freeze-300-past-immobilize`, `freeze-4`, `freeze-4-past-immobilize`, `freeze-600`, `freeze-600-past-immobilize`, `immobilize-120`, `immobilize-120-past-blind`, `immobilize-120-past-confuse`, `immobilize-2`, `immobilize-2-past-blind`, `immobilize-2-past-confuse`, `immobilize-30`, `immobilize-30-past-blind`, `immobilize-300`, `immobilize-300-past-blind`, `immobilize-4`, `immobilize-4-past-blind`, `immobilize-4-past-confuse`, `immobilize-60`, `immobilize-60-past-blind`, `immobilize-60-past-confuse`, `immobilize-600`, `immobilize-600-past-blind`, `paralyze-120`, `paralyze-150`, `paralyze-300`, `paralyze-4`, `paralyze-600`, `paralyze-90`.

### Lock-on modes (rules/lockon.luau) (15)

`beast-claw`, `beast-lunge`, `bigbomb`, `cannon`, `crakshot`, `crosswrd`, `drksword`, `dublshot`, `gundelex`, `moonbld`, `sprsonic`, `stay`, `thunder`, `widesht`, `yoyo`.

### Projectile variants (objects/projectile/variants.luau) (33)

`shot/blade`, `shot/blinding`, `shot/breaking`, `shot/bright`, `shot/charged-blinding`, `shot/charged-bubbling`, `shot/charged-confusing`, `shot/charged-cracking`, `shot/charged-flinching`, `shot/charged-freezing`, `shot/charged-grass`, `shot/charged-hp-bug`, `shot/charged-hp-bug-marked`, `shot/charged-ice`, `shot/charged-panel-breaking`, `shot/charged-paralyzing`, `shot/charged-poisoning`, `shot/charged-pulling`, `shot/charged-road`, `shot/charged-road-back`, `shot/charged-uninstalling`, `shot/charged-volcano`, `shot/confusing`, `shot/cracking`, `shot/erasing`, `shot/flinching`, `shot/grass`, `shot/ice`, `shot/paralyzing`, `shot/poisoning`, `shot/pushing`, `shot/shoving`.

| name | confidence | why it stays |
|---|---|---|
| `shot/charged-attack-a0` | low | Projectile row 0x19: its collision type's flag 0x20 has no known meaning. |

### Weapons (compat/weapons.toml) (67)

`bgdththd/charge`, `boomrarm/charge`, `bugrswrd/charge`, `chargecross-beast/wave`, `chargecross/tackle`, `chargeman/charge`, `dustcross-beast/scatter`, `dustcross-beast/throw-absorbed`, `dustcross/charge`, `dustcross/throw-absorbed`, `dustman/charge`, `eleccross-beast/charge`, `eleccross/a-charge`, `eleccross/charge`, `elecman/charge`, `erasecross-beast/drop`, `erasecross/charge`, `eraseman/charge`, `groundcross-beast/dash`, `groundcross/drill`, `groundman/charge`, `heatcross-beast/charge`, `heatman/charge`, `megaman/absorb`, `megaman/anti-damage`, `megaman/beast-claw`, `megaman/blank-shot`, `megaman/charged-chip-bonus`, `megaman/charged-shot`, `megaman/falzar-beast-buster`, `megaman/gregar-beast-buster`, `megaman/reflect`, `megaman/rock-barrage`, `megaman/shield`, `megaman/slash-a-charge`, `megaman/tengu-wind`, `needlarm/charge`, `protoman/a-charge`, `protoman/back-special`, `protoman/charge`, `puncharm/charge`, `puzzlarm/charge`, `slashcross-beast/lunge`, `slashcross/charge`, `slashman/charge`, `spoutcross-beast/charge`, `spoutcross/charge`, `spoutman/charge`, `tengucross-beast/charge`, `tengucross/charge`, `tenguman/charge`, `tomahawkcross-beast/throw`, `tomahawkcross/charge`, `tomahawkman/charge`.

| name | confidence | why it stays |
|---|---|---|
| `megaman/buster-2e` | low | The buster's routine with its own charge times, which no form, navi or program names; its routine numbers are its only identity. Renamed by hand if a user is found. |
| `megaman/buster-3f` | low | The buster's routine with its own charge times, which no form, navi or program names; its routine numbers are its only identity. Renamed by hand if a user is found. |
| `megaman/buster-6f` | low | The buster's routine with its own charge times, which no form, navi or program names; its routine numbers are its only identity. Renamed by hand if a user is found. |
| `megaman/buster-70` | low | The buster's routine with its own charge times, which no form, navi or program names; its routine numbers are its only identity. Renamed by hand if a user is found. |
| `megaman/buster-79` | low | The buster's routine with its own charge times, which no form, navi or program names; its routine numbers are its only identity. Renamed by hand if a user is found. |
| `megaman/buster-7e` | low | The buster's routine with its own charge times, which no form, navi or program names; its routine numbers are its only identity. Renamed by hand if a user is found. |
| `megaman/buster-82` | low | The buster's routine with its own charge times, which no form, navi or program names; its routine numbers are its only identity. Renamed by hand if a user is found. |
| `megaman/charged-chip-bonus-1f` | low | nullsub_44's charged-chip bonus with its own charge times, used by TomahawkMan and SpoutCross; nothing distinguishes it but its charge row. |
| `megaman/charged-chip-bonus-20` | low | nullsub_44's charged-chip bonus with its own charge times, used by TomahawkCross and SpoutMan; nothing distinguishes it but its charge row. |
| `megaman/charged-chip-bonus-29` | low | nullsub_44's charged-chip bonus with its own charge times, used by ChargeMan and ChargeCross; nothing distinguishes it but its charge row. |
| `megaman/reflect-2` | medium | Routine 0x3C: the Reflect's guard; nothing names it (the NaviCust writes 0x8B). |
| `megaman/shield-2` | medium | Routine 0x8C: the Shield's guard; nothing names it (the NaviCust writes 0x3B). |
| `protoman/back-special-2` | medium | Routine 0x30: ProtoMan's other B+Back, named in NaviStats. |

### Actions (compat/actions.toml) (13)

`bgdththd/charge/action`, `eleccross/charge/thunder`, `engine/cross-special`, `heatcross/charge/action`, `megaman/reflect/action`, `megaman/shield/action`, `megaman/turn`, `protoman/back-special/action`, `puzzlarm/charge/action`, `recov/none`, `spoutcross/charge/action`, `tengucross/charge/gust`, `tornado/back-spread`.

### Kinds (compat/kinds.toml) (16)

`blkbomb/bomb`, `bomb-slash`, `bugbomb/bomb`, `chargecross-beast/wave`, `dustcross-beast/junk-shot`, `energbom/burst`, `erasecross-beast/drop`, `erasecross/ray`, `flshbom/bomb`, `groundman/drill`, `megaman/dash-hit`, `seed`, `slashcross-beast/hit-flash`, `slashcross-beast/lunge-slash`, `spoutcross-beast/surge`, `tengucross-beast/whirlwind`.

### Chips (compat/chips.toml) (6)

`beastout-dimming-1`, `beastout-dimming-2`, `hidden`, `invalid`, `stepswrd-protoman`, `whicapsl-invisible`.

### Chip series' modules (34)

`chips/aurahed`, `chips/blastman`, `chips/cannon`, `chips/chrgeman`, `chips/colonel`, `chips/cornsht`, `chips/crcusman`, `chips/diveman`, `chips/django`, `chips/dustman`, `chips/elcpuls`, `chips/elecman`, `chips/elmntman`, `chips/eraseman`, `chips/firebrn`, `chips/firehit`, `chips/flmhook`, `chips/gigacan`, `chips/grndman`, `chips/hackjack`, `chips/heatman`, `chips/ironshl`, `chips/judgeman`, `chips/protoman`, `chips/pwrwave`, `chips/roll`, `chips/slashman`, `chips/spoutman`, `chips/spreadr`, `chips/tenguman`, `chips/timebom`, `chips/tmhkman`, `chips/wavearm`, `chips/widebrn`.

### Stages (compat/stages.toml) (1)

| name | confidence | why it stays |
|---|---|---|
| `netbattle-*` | medium | The 192 settings records, deduplicated and numbered in order of first use; the ROM gives no names and the stages differ only by panels and placements. |

## Placeholders left numbered (253)

Names the extractor gives what nobody named (`sprite-cc-ii`, `sound-nnn`). Content may not use them (a lint), so
every one is something no netbattle shows or plays: overworld and menu sounds, virus sheets, and the ROM's dummy
sprite (a 16x16 dot that many unused sprite slots share). Naming them waits for a use. Accepting them changes
nothing.

### Sprites (72)

- the ROM's dummy sprite (a 16x16 dot many unused slots share); nothing shows it. `sprite-00-0d`, `sprite-04-1e`, `sprite-08-11`, `sprite-08-16`, `sprite-0c-26`, `sprite-0c-32`, `sprite-0c-39`, `sprite-0c-3a`, `sprite-0c-3e`, `sprite-0c-3f`, `sprite-0c-42`, `sprite-0c-46`, `sprite-0c-49`, `sprite-0c-4a`, `sprite-0c-4e`, `sprite-0c-56`, `sprite-0c-57`, `sprite-0c-5a`, `sprite-0c-5b`, `sprite-0c-65`, `sprite-0c-66`, `sprite-0c-68`, `sprite-10-04`, `sprite-10-14`, `sprite-10-15`, `sprite-10-18`, `sprite-10-1e`, `sprite-10-33`, `sprite-10-35`, `sprite-10-36`, `sprite-10-3a`, `sprite-10-3e`, `sprite-10-3f`, `sprite-10-49`, `sprite-10-58`, `sprite-10-5b`, `sprite-14-10`, `sprite-14-17`, `sprite-14-19`
- a virus's sheet; no netbattle shows it. `sprite-04-01`, `sprite-04-02`, `sprite-04-04`, `sprite-04-06`, `sprite-04-07`, `sprite-04-08`, `sprite-04-0b`, `sprite-04-0c`, `sprite-04-0e`, `sprite-04-11`, `sprite-04-12`, `sprite-04-13`, `sprite-04-14`, `sprite-04-15`, `sprite-04-16`, `sprite-04-17`, `sprite-04-1c`, `sprite-04-1f`
- a sheet no netbattle shows (content and the engine name only what they show). `sprite-08-15`, `sprite-0c-25`, `sprite-0c-67`, `sprite-10-0c`, `sprite-10-13`, `sprite-10-1a`, `sprite-10-1b`, `sprite-10-1c`, `sprite-10-25`, `sprite-10-29`, `sprite-10-2b`, `sprite-10-2d`, `sprite-10-4b`, `sprite-10-53`, `sprite-14-1d`

### Sounds (181)

- nothing in the port plays it (content may not name a placeholder, and the ruleset plays only its roles' named sounds). `sound-066`, `sound-073`, `sound-074`, `sound-07d`, `sound-07e`, `sound-095`, `sound-096`, `sound-098`, `sound-09a`, `sound-09b`, `sound-09c`, `sound-09d`, `sound-09e`, `sound-0ac`, `sound-0b1`, `sound-0b5`, `sound-0c8`, `sound-0c9`, `sound-0ca`, `sound-0cb`, `sound-0cc`, `sound-0cd`, `sound-0cf`, `sound-0d2`, `sound-0d3`, `sound-0d5`, `sound-0d6`, `sound-0d7`, `sound-0db`, `sound-0dc`, `sound-0dd`, `sound-0de`, `sound-0df`, `sound-0e2`, `sound-0e7`, `sound-0e8`, `sound-0e9`, `sound-0ea`, `sound-0eb`, `sound-0ec`, `sound-0ee`, `sound-0ef`, `sound-0f1`, `sound-0f2`, `sound-0f5`, `sound-0f6`, `sound-0fa`, `sound-0fe`, `sound-101`, `sound-102`, `sound-103`, `sound-105`, `sound-106`, `sound-108`, `sound-109`, `sound-10b`, `sound-11e`, `sound-121`, `sound-123`, `sound-125`, `sound-127`, `sound-12c`, `sound-12f`, `sound-130`, `sound-133`, `sound-135`, `sound-136`, `sound-137`, `sound-138`, `sound-139`, `sound-13a`, `sound-13d`, `sound-13e`, `sound-13f`, `sound-141`, `sound-142`, `sound-145`, `sound-147`, `sound-149`, `sound-14a`, `sound-14b`, `sound-14d`, `sound-14e`, `sound-14f`, `sound-151`, `sound-152`, `sound-153`, `sound-154`, `sound-156`, `sound-15a`, `sound-15b`, `sound-15c`, `sound-15e`, `sound-15f`, `sound-160`, `sound-161`, `sound-162`, `sound-163`, `sound-165`, `sound-166`, `sound-168`, `sound-169`, `sound-16a`, `sound-16c`, `sound-16d`, `sound-16f`, `sound-170`, `sound-171`, `sound-174`, `sound-175`, `sound-176`, `sound-177`, `sound-178`, `sound-179`, `sound-17a`, `sound-17c`, `sound-17d`, `sound-17e`, `sound-183`, `sound-186`, `sound-18b`, `sound-18c`, `sound-18d`, `sound-18e`, `sound-18f`, `sound-190`, `sound-191`, `sound-192`, `sound-193`, `sound-194`, `sound-195`, `sound-197`, `sound-198`, `sound-199`, `sound-19b`, `sound-19c`, `sound-19d`, `sound-19e`, `sound-19f`, `sound-1a0`, `sound-1a1`, `sound-1a2`, `sound-1a3`, `sound-1a4`, `sound-1a5`, `sound-1ac`, `sound-1ad`, `sound-1ae`, `sound-1af`, `sound-1b0`, `sound-1b1`, `sound-1b2`, `sound-1b3`, `sound-1b4`, `sound-1b5`, `sound-1b6`, `sound-1b7`, `sound-1b8`, `sound-1b9`, `sound-1ba`, `sound-1bb`, `sound-1bc`, `sound-1c1`, `sound-1c2`, `sound-1c3`, `sound-1c4`, `sound-1c8`, `sound-1c9`, `sound-1ca`, `sound-1cb`, `sound-1ce`, `sound-1cf`, `sound-1d1`, `sound-1d2`, `sound-1d3`, `sound-1d4`, `sound-1d5`, `sound-1d6`, `sound-1d7`, `sound-1d8`, `sound-1d9`

