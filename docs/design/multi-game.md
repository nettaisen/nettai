# Multi-game nettai: what is BN6's, what BN4 and BN5 need, and how to get there

A survey and a proposal, not a built feature. The user, on renaming the engine: "rename the engine to nettai, but
keep the bn6 content bn6. i want to eventually port bn5 and bn4 and other games over as well." The crates now
have engine names (nettai-battle, nettai-content-api, nettai-luau, nettai-content, nettai-netplay, ...), but much
of what is in them is BN6's rules written in Rust. This document is for deciding how and when nettai plays BN5,
BN4 and the others. It covers:

- §1, an inventory: every module of the engine, the content API, the Luau runtime, the pack formats, the frontend
  and netplay, classed as generic, BN-series but parameterized, or BN6-only, with the evidence;
- §2, what differs in BN4 and BN5, and what is the same;
- §3, the architecture options and a recommendation;
- §4, a staged plan with rough sizes;
- §5, names;
- §6, the decisions this leaves to the user.

Nothing was changed for it. Line counts are `wc -l` of main at f84e3db7 (just before the rename), comments
included, rounded; paths use the crate names after the rename (e2f7c774), which kept the module paths inside the
crates.

Related: [core-content-boundary.md](core-content-boundary.md) (the core, ruleset and content layers as built),
[content-model-v2.md](content-model-v2.md) (content by name, compat apart), [scripting.md](scripting.md) §6–§7
(the Luau runtime's cost), [rollback.md](rollback.md) (snapshots, digest, perspective),
[content-pack.md](content-pack.md) and [asset-formats.md](asset-formats.md) (packs).

## Decided (2026-10-01)

The user, on this document: "push more rules into luau. also don't do this until all the gameplay stuff in bn6 is
done".

- **Option (b), not the recommended hybrid:** more of the rules move into Luau, so that a game is mostly content.
  §3.3 is the direction; §3.5's recommendation and its Rust-ruleset-per-game line are not taken. What §3.1 says
  about rollback cost (Luau-side rules estimated at five to ten times today's 124 µs worst case per rendered frame)
  and about global script state are the problems that work will have to solve, not reasons to stop.
- **Not started until BN6's gameplay is done:** nothing in §4 begins, including the move-only `bn6` boundary,
  while BN6's gameplay work is open (the one-sided and unrun coverage, the Gregar console, the 2022 replays, and
  whatever they find). The other decisions in §5 wait with it.
- **2026-10-02, "start abstracting more rules so bn5 support can be implemented"**, with BN5 "composable with bn6
  content": the design is [rules-in-luau.md](rules-in-luau.md), approved the same day with the user's decisions:
  one ruleset per player, rulesets of mixable systems with a stock ruleset per game, the custom screen a Rust core
  with Luau extras, and no shared content library (each game exports its own: `bn6:cannon`, `bn5:cannon`). It
  replaces §3.5's recommendation and §4's plan.

## 0. Summary

- **nettai-battle is about a sixth generic, two thirds BN-series rules written with BN6's numbers, and a sixth
  BN6-only.** Of about 31,500 lines (tests left out): about 5,500 would serve another Battle Network game as
  they are (object pools and the update list, RNG streams, input and the link, collision registration, the panel
  grid, the content host and registries, snapshots, the digest, cues); about 20,000 are rules every game in the
  series has but are written with BN6's tables, layouts and record fields (the navi framework, the hit kernel,
  statuses, the custom gauge, the chip hand, the dimming service, the flow, NaviStats); about 6,000 are BN6's alone
  (Cross, Beast Out and Beast Over, the Beast rush and its lock-on, the Cross window and the Beast Out button,
  the Cross change mode, Falzar and Gregar, SP deletion times, the link navis' own chips).
- **BN4, BN5 and BN6 are one engine lineage.** Tango's game support for the three reads battle objects of the
  same size (0xD8 bytes) with the fields it reads at the same offsets (the panel at +0x12, the owner at +0x16, HP
  at +0x24, max HP at +0x26), the same 0x50-byte per-player chip hand, and the same 0x2C-byte chip record with its
  fields in the same order. So most of
  the "parameterized" two thirds will carry over to BN5 and BN4 with different numbers, and some of it with
  different branches. BN1 to BN3 are an earlier lineage and would need more than a ruleset.
- **What BN5 and BN4 add is mostly at the custom screen and in transformations**: Soul Unison (both), Chaos
  Unison (BN5), dark chips offered in battle (BN4) or carried in the folder (BN5), and different emotions. A soul
  fits the engine's form model; the ruleset around it (how it is chosen, how long it lasts, what ends it) does not.
- **Recommendation: a hybrid (§3.5).** A Rust core; a Rust "BN-family" framework shared between games only where
  their routines are shown to match; a Rust ruleset per game for the custom screen, transformations, emotions,
  NaviStats and the game-only kinds; Luau content per game as now. Not the framework in Luau: it would cost
  roughly five to ten times today's 124 µs worst case per rendered frame, need a new kind of global script state,
  and re-port verified rules for no gain to BN6.
- **Now, cheaply:** names, and moving BN6-only code behind a `bn6` boundary inside the crates without changing
  behavior (about one to two agent-days, then one full check). **Later:** a BN5-to-BN6 routine map, the ruleset
  seam, then the BN5 port (oracle, compat, extractor, rules, content), roughly half to two thirds of the BN6
  effort.
- The decisions for the user are in §6.

## 1. Inventory

### 1.1 The classes

| Class | Meaning | Test |
|---|---|---|
| **G**, generic | Would serve any Battle Network game as it is | Proven only for the BN4–BN6 lineage (§2.2); for BN1–BN3, probably with parameters |
| **P**, BN-series, parameterized | Every game in the series has it, but the code holds BN6's numbers, tables, record layout or branches | BN4 and BN5 have the same thing with other values or a few other branches |
| **6**, BN6-only | No other game has it | Cross, Beast Out, Beast Over, BN6's custom-screen layout and buttons, the link navis' own chips, SP deletion times, Falzar and Gregar |

Most modules mix classes; the tables give the split and what makes a module BN6's.

### 1.2 nettai-battle

| Module | Lines | Class | What is BN6's in it |
|---|---|---|---|
| object/ (mod.rs, sprite.rs) | 610 | G | Three pools of 32 slots (`object::SLOTS`, `Pool::{Actor, Attack, Effect}`), lowest free slot, insertion after the updating object, lifecycle values 0/4/8/0xC. The pool count and sizes are the BN4–BN6 engine's (to confirm per game); nothing BN6-specific in the logic. |
| rng.rs | 45 | G | RNG1 per console, RNG2 shared. Tango seeds both the same way in BN4 and BN5. That the step function is the same is likely, to confirm. |
| input.rs, link.rs | 200 | G | Joypad records, auto-repeat, the link's per-tick packets with a delay. What a packet carries (held keys, the custom screen's status bit) is BN6's, likely the same in BN4/BN5. |
| cues.rs, sound.rs, digest.rs, rollback.rs | 810 | G | Rollback-aware cues, the digest, snapshots. |
| perspective.rs | 170 | P | The mechanism is generic; the list of per-console state (who fades in, what a blinded player sees, the telop per viewer, the pinch music) is BN6's. |
| console.rs | 360 | P | Per-console RNG1 is generic. What advances it is BN6's: the camera shake, the emotion window's bug flicker, ChpShufl's re-deal. |
| collision.rs | 640 | G + P | Slot pool (32), per-panel registration, remove-and-pair resolution: G. The hit kernel (guard, counter, element multiplier, statuses, the nine status timers in `collision::timer`): P. |
| field.rs | 760 | G + P | The 8×5 grid with a 6×3 playable area, reservations, cached flags: G. `PanelType`'s 13 types and their numbers (missing, broken, normal, cracked, poison, holy, grass, ice, volcano, four roads) are BN6's: BN4 has a Metal panel and numbers holy 9 (its patch cards: 1 broken, 3 cracked, 5 metal, 9 holy). The field-object registry and stolen-area return: P. |
| actor.rs | 380 | P | AIData, eight slots. Its `request` bits include BN6's: `CROSS_CHANGE`, `CROSS_DEATH`, `CROSS_SPECIAL`, `SELECT_SPECIAL`, `MODE9_A` (`REVERT_FORM`, the turn-start reversion, is one a soul's end would use too). |
| hand.rs | 120 | P | The chip hand; the 0x50-byte block BN4–BN6 share. |
| hud.rs | 220 | P | The custom gauge (`CustomGauge::FULL` 0x4000, `rate_for`'s table by both players' gauge speed) and banner lifetime (5, 0x30, 5 ticks): the values are BN6's. The telop. |
| dimming.rs | 690 | P | The dimming service: records, the telop, the cut-in and counter cut-in, AntiNavi. BN4 and BN5 have dimming chips (K); whether their counter cut-in works as BN6's is not known. |
| transform.rs | 230 | P + 6 | The turn-start sequencer (fade out, change, fade in) is a shape Soul Unison also needs. `TransformRequest { form, cross_change }` and the Beast Out check are BN6's. |
| battle.rs | 2,140 | P (+6 ≈ 450) | The flow machines (top state, mode handler, fighting machine, judge, set chaining): P. BN6's: `cross_stats`, `beast_out_used`, `crossed`, `bug_frags`, `navi_levels`, `turn_transforms`, `sides: [SideState; 2]` (the battle flag 0x40 mode: SELECT special, Cross special, per-player gauges), `count_down_beast_out`, `FadeMode::{BeastOut, BeastOutBack, Mode1TransformIn, Mode1TransformOut}`. |
| setup.rs | 415 | P (+6) | `RoundSetup`, `BattleSettings`, `SetScore`: P. `NaviStats` is BN6's 0x64-byte block with BN6's fields: `support: Option<Supports>` (Rush, Beat, Tango), `beast_out_counter`, `version` ("0 Gregar, 1 Falzar"), `sun`, `folder_tags`, `chip_shuffle`, `number_open`. `set_byte_by_bug_code` writes it by BN6's offsets. `SpTimes([u16; 20])` holds BN6's SP navi deletion times. |
| custom/ | 3,410 | P + 6 | mod.rs: `GameVersion { Gregar, Falzar }` and `Unlocks { crosses: [bool; 5], beast_out, beast_out_sealed }` (6). screen.rs (1,365): the slots, cursor, selection rules, deal, Program Advance animation, descriptions and the run message (P); `CrossWindow`, `Phase::{CrossWindowOpening, CrossWindow, CrossWindowClosing, CrossChosen, BeastOutChosen, BeastOutChipChosen, Scrapping, Redealing}`, `SlotKind::{BeastOut, Scrap, Redeal, NaviChip}`, `CROSSES = 5` and `hand_size`'s ChargeCross and NumbrOpn rules (6, about 560). builder.rs (Program Advances, modifiers), chatbox.rs, folder.rs (30 chips; the tag pair is BN6's), look.rs: P. library.rs: `cross_form`, `beast_out_form`, `beast_over_form`, `form_in_beast_out` (6). |
| kinds/player/ | 8,060 | P (+6 ≈ 2,500) | The navi framework: spawn and entry, input and charge, intake, status, reactions, idle, chip use, movement, and the dimming, navi, instant and reactive chip actions (P). BN6-only files: berserk.rs 350 (Beast Over), actions/beast_rush.rs 330, actions/cross_change.rs 320, actions/cross_special.rs 80, actions/transform.rs 710 (the five Cross and Beast sequences). BN6 branches elsewhere: mod.rs (`GREGAR_OVER_GLOW`, `FALZAR_OVER_GLOW`, `Emotion::{Tired, WornOut}`), status.rs (the Cross knockout instead of deletion, a weakness hit breaking the Cross, the dispatch into the Beast rush, the Cross special's ticks, Beast Over's HP drain), idle.rs (Beast Over's berserk, the Cross protect, the Cross special), chip_use.rs (the Beast claw and the rush's lock-on, EraseCross, Beast Over's Null bonus). |
| kinds/ (the engine's kinds) | 5,020 | G + P + 6 | effect, hitbox, spark, intro, palette_flash, common, mod: G. obstacle (1,130), navi_chip, navi_warp, heal, charge_glow, full_synchro_aura, the status, bubble and ice visuals, hit_marker, body, form and idle overlays: P. eruption is BN6's volcano panel (P if BN4/BN5 have one). afterimage (the Beast rush), lockon_marker, cross_merge, beast_over_burst: 6 (about 790). |
| content/ | 4,980 | G + P (+6 ≈ 650) | defs.rs (1,380), mod.rs, reader.rs, scripts.rs, sprites.rs, flags.rs, sections.rs: G. chips.rs: `ChipFamily` (BN6's 13 icon families), `ChipTraits` (BN6's list), `ExtraChipFlags::{RUSH_CANCELS, FREE_SLOT_IN}`, `DamageFormula::{SpNavi, NaviLevel}` (6); the rest of the record P. navis.rs: `FormKind::{Cross, Beast, CrossBeast, BeastOver}`, `NaviForms { gregar, falzar }`, `FormSet { crosses, beast_out, beast_over }`, `FormTraits` (ChargeCross, DustCross, EraseCross...), `FreshStats` and `cross_hp` (the Cross change): 6; the rest of `FormData` (element, weakness, buster bonus, chip bonus by family, charged chips, weapons) is P and fits a soul. rules.rs: `Lockon`, `LockonMode`, `BerserkRules`, `cross_special`, `sp_deletion_times`, `sp_slots` (6); `element_weakness: [[u8; 6]; 6]`, `family_elements: [_; 13]`, panels, buster recovery (P). roles.rs: the role vocabulary is BN6's (`gregar_roar`, `falzar_roar`, `beast_over_*`, `cross_death`, `beast_claw`, `rush`/`beat`/`tango`...). identity.rs, stages.rs, custom.rs: P. |
| behavior/ | 2,260 | G + P (+6 ≈ 300) | Dispatch to content (mod.rs): G. `impl CoreApi for Battle` (core_api.rs, 1,990) serves BN6's API too: the Beast lock-on, `beast_out_spent`, the emotion and mood, bug frags, the side special, MstrCros's Crosses. |

Tests and test content (custom/tests.rs, behavior/tests.rs, actions/tests.rs, content/testing.rs, scenario.rs:
6,300 lines) are left out. The test content is made up, but it models BN6's (a Gregar and a Falzar Beast, Crosses).

### 1.3 The content API and its declarations

**nettai-content-api** (2,960 lines). The registries, handles, content state, the host contract, definitions
and asset names (1,430 lines) are G, except `Registry::Lockon` (BN6's Beast Out lock-on modes; `Registry::Form`
is P: souls are forms too). api.rs (1,530) is the `CoreApi` both sides speak, and it names BN6's mechanics
(about 230 lines):

- `ActorField::{BeastLockon, BeastOutSpent}`; `NaviStat::{Mood, BeastOutCounter, Beast, BeastOver}` and the
  version; the NaviCust bug stats;
- `NaviState::{CrossChange, CrossDeath, CrossSpecial, ChangingCross, CrossKnockout, Crossed, CrossBreaking}`;
- `Emotion` (BN6's five), `SideSpecial::{Select, Cross}`, `bug_frags` and `spend_bug_frags`;
- `Pool` (three pools) and `SpriteId` ("CC-II", a category and an index) are the BN4–BN6 layout: P.

**content/bn6/core.d.luau** (1,880 lines) and **types.d.luau** (910) declare the API for the type checker and
editors. They live in BN6's content root, and about 120 of core.d.luau's lines name BN6's mechanics (Beast Out,
Crosses, Full Synchro, the emotions and mood, bug frags, NaviCust bugs, supports, Falzar and Gregar). Another
game's root would have to copy the generic part.

### 1.4 The Luau runtime and the checker

**nettai-luau** (3,250 lines without tests): G. The sandbox, the stateless-function verifier, the define phase,
and the binding of whatever `CoreApi` declares (`battle`, `field`, `dimming`, `navi_chip`, `obstacle`, `int`).
It names nothing of BN6's except through `CoreApi`'s enums, so a per-game API would need the binding to take a
per-game library as well (§3.6). nettai-battle calls it directly (`Content::define` runs `nettai_luau::define`).

**nettai-content-check** (410 lines plus tests): G. It type-checks a root against that root's own
`core.d.luau`, and its lints and guards are the content model's rules, not BN6's.

### 1.5 Content

**content/bn6** (82,400 lines of Luau in 644 modules: chips 50,100, navis 13,500, lib 7,600, objects 3,100,
stages 3,100, rules 2,100) is BN6's by definition and stays so. Two parts matter to other games:

- the families in lib/ (bombs, swords, cannon, projectile, grab, dimming, barriers, traps, ...): BN4 and BN5 have
  many of the same chips, but whether each behaves the same frame for frame is unknown until traced;
- rules/ (collision, elements, panels, status, lock-on, roles, ...): the ruleset's tables, which are P: another
  game has its own values, and some of BN6's tables (Cross special, lock-on, SP chips) have no counterpart.

### 1.6 Pack formats, assets and the extractor

**nettai-content** (6,320 lines of source). Sprites (part atlas, `sprite.json`, `animations.json`), Aseprite
views, indexed images and tiles, backgrounds as Tiled maps, sound as MIDI, TOML and WAV for the m4a driver,
names, the pack, the root, reports and verification: G, about 5,200 lines. The sprite format is very likely the
same in BN4 and BN5 (same engine lineage), to confirm by extracting. stage.rs (520, the field's panels and
backgrounds): P. **hud.rs (310) and custom.rs (300) are BN6's HUD and custom-screen layouts**, under the generic
format names `nettai-content/hud` and `nettai-content/custom`.

**nettai-assets** (540): typed graphics, G, except `Hud` (mugshots by emotion, the Beast Out count box,
`form_emotions`, the link navis' mugshots) and custom.rs's `CustomScreen` (`beast_buttons`, `cross_maps`,
`redeal_buttons`, `scrap_buttons`, emblems by navi): about 240 lines of BN6.

**bn6-extract** (960): BN6's ROM addresses and tables; stays BN6's. Another game gets its own extractor on the
same writers.

### 1.7 Frontend and audio

**nettai-frontend** (4,560). The compositor (layers and sprite priority), objects (the 16.16 projection,
pool-order drawing), the stage, rendering, the window, headless rendering and the status text: G, about 2,300.
BN6's, about 2,250:

- hud.rs (1,070): the HP box and gauge are P; the emotion window is BN6's, and finds a face by BN6's form numbers
  (11, 12, 1 to 10) through bn6-compat (the presentation work is replacing that lookup);
- custom.rs (800): BN6's custom screen (the Cross tab, the Beast Out button, the scrap and re-deal buttons);
- driver.rs (380): trace playback through bn6-compat, and the live setup with
  `Unlocks::everything(GameVersion::Falzar)`.

The frontend depends on bn6-compat.

(Since the survey the drawing is a crate of its own, nettai-render: the compositor, objects, the stage, rendering,
hud.rs and custom.rs. nettai-frontend keeps the window, the drivers and headless output; both depend on
bn6-compat.)

**nettai-audio** (430) plays cues with m4a: G, but `SoundCalls` encodes BN6's sound wrappers (PlayMusic's
current-music check, the pinch effect's pitch and tempo): P. **m4a** (3,920): G for the GBA games; BN1 to BN6
all use the m4a driver, as far as I know.

### 1.8 Netplay

**nettai-netplay** (1,180): G. world.rs (`Game`, `BattleWorld`, `Observer`), sim.rs, network.rs and rng.rs hold
no rule. The `bn6` module (125 lines) adapts the engine to getgud: `Bn6Input` is buttons plus `TickEvents`, the
engine's own input record, with nothing BN6-specific in it but its name. standin.rs (190) builds BN6 setups (a
Falzar player, BN6 folders). `Game::battle()` returns the concrete `Battle`.

### 1.9 Verification

bn6-compat (1,800 lines: BN6's numbers, BN6's setup records, the trace harness) and the verification workspace
(oracle-trace's RAM addresses, difftest's ROM hooks through Tango's BN6 support, the chip lab through Tango's BN6
save view, the recompiler) are BN6's. The shape of the trace comparison (objects by slot, a navi's action by its
number, positions, HP) would serve BN5, since the object records match; the addresses and setup records would
not.

### 1.10 Totals

| Crate | Lines (no tests) | G | P | 6 |
|---|---|---|---|---|
| nettai-battle | 31,500 | 5,500 | 20,000 | 6,000 |
| nettai-content-api | 2,960 | 1,430 | 1,300 | 230 |
| nettai-luau, nettai-content-check | 3,660 | 3,660 | | |
| nettai-content | 6,320 | 5,200 | 520 | 610 |
| nettai-assets | 540 | 300 | | 240 |
| nettai-frontend | 4,560 | 2,300 | 300 | 1,950 |
| nettai-audio, m4a | 4,350 | 4,100 | 250 | |
| nettai-netplay | 1,180 | 990 | | 190 |
| content/bn6 (Luau) | 82,400 | | | all, by definition |
| bn6-compat, bn6-extract | 2,760 | | | all, by design |

The split inside a file is an estimate from reading it; the files named in the tables are the evidence.

## 2. What differs in BN4 and BN5

### 2.1 Sources, and how sure

There is no BN4 or BN5 disassembly on this machine. The sources are Tango's BN4 and BN5 game support (the
netbattle priming, the telemetry polls, the save views with the folder and NaviCust, the chip and patch-card
readers), and what I know of the games. Each claim below is marked:

- **(T)**: Tango's code says so (its field names are Tango's reading, not the games');
- **(K)**: from knowledge of the games, fairly sure;
- **(?)**: unsure; to confirm from the ROM before designing on it.

### 2.2 What is the same

- **The battle object record**: 0xD8 bytes, the panel at +0x12, the destination at +0x14, the owner at +0x16, HP
  at +0x24, max HP at +0x26, in BN4, BN5 and BN6 (T). BN5 keeps the loaded chip at +0x2A as BN6 does (T).
- **The chip hand**: a 0x50-byte block per player, a counter of chips used, then the chips (T).
- **The chip record**: 0x2C bytes in all three, in the same order: codes, attack element, rarity, element,
  class, MB, effect flags (bit 5 marks a dark chip), counter settings, attack family and subfamily (the chip's
  action and its variant: what v2 made each chip's own use), a byte for the Dark Soul's use of the chip, attack
  parameters, the lockout delay, library numbers, attack power, sort keys, the Battle Chip Gate's limit, and the
  icon, image and palette (T). BN5 and BN6 add a lock-on byte, a lock-on type and a dark chip id; BN4 and BN5 have
  a karma byte (T). Classes: standard, mega, giga, Program Advance (T).
- **Two RNGs**: each console's own (RNG1) and the shared one (RNG2), both seeded at game load; the battle's
  settings are drawn by a generator in the ROM from them (T).
- **Netbattle sets**: single battle and triple battle, best of three, chained by the game itself (T).
- **The rest of the frame (K):** a 6×3 field; one custom gauge; chips picked on a custom screen, with Program
  Advances, a Regular chip (T: the save views) and Mega and Giga folder limits (T: at most 10); the buster with
  attack, rapid and charge levels; NaviCust programs with bugs; counter hits and Full Synchro; the emotion window
  (new in BN4); dimming chips and navi chips; MegaMan changing form at the start of a turn (souls in BN4 and BN5;
  Crosses and Beast Out in BN6); the m4a sound driver.

### 2.3 BN5 (Team ProtoMan, Team Colonel)

- **Soul Unison (K).** Twelve souls, six per version: ProtoSoul, GyroSoul, NapalmSoul, SearchSoul, MagnetSoul and
  MeddySoul (Team ProtoMan); ColonelSoul, ShadowSoul, NumberSoul, TomahawkSoul, KnightSoul and ToadSoul (Team
  Colonel). On the custom screen the player sacrifices a chip of the soul's kind; MegaMan changes at the turn's
  start and stays in the soul for three turns. Its duration is a stat: BN5's patch cards have "soul time +N" and
  "-N" (T). A soul changes the element, the buster and charged shot, lets chips of a family be charged by holding
  A, and adds abilities (K). Whether each soul can be used once a battle, and exactly which chips pay for which
  soul (?).
  - **Engine fit:** a soul is a form. `FormData` already has the element, weakness, buster bonus, chip bonus by
    family, charged chips and weapons. What doesn't fit: `FormKind`, `NaviForms { gregar, falzar }`, how the
    custom screen offers a soul, and the ending: a soul ends after its turns (a turn-start check, like Beast
    Out's counter), a Cross on a weakness hit.
- **Chaos Unison (K, details ?).** Later in the game a dark chip can pay for a soul. In the Chaos soul the charged
  shot, released at the right moment of a flashing charge, uses the sacrificed dark chip; released at the wrong
  moment it goes wrong for MegaMan (what exactly happens: ?). New to the engine: a charge with a timing window,
  and a chip held by the form as its state.
- **Dark chips (T, K).** They go in the folder, at most three (T). A battle in which they were used costs MegaMan
  a point of max HP afterwards (T: the save's dark HP-loss counter, which stops at 499) and moves his karma (T: 0
  to 1000, kept in MegaMan's 0x60-byte stats record in the save, with an anti-tamper mirror). In battle, the
  custom screen dims when the cursor rests on a dark chip (BN6 has the same fade, `FadeMode::DarkChip`; that BN5
  does: ?). What a dark chip's use does to the emotion (?).
- **Liberation missions (K): out of scope**, like random battle. They are a board mode with phases, dark panels
  and the team navis' abilities; their battles have a three-turn limit. No netbattle reaches them.
- **Team navis in netbattle (T, ?).** The save names the navi the player battles as, and a link navi has no
  NaviCust of its own (T). BN5's netbattle menu has a team battle row (チームバトル, the "team modes"), a stage
  set (normal or extended) and a Patch Card switch (T). What a team battle changes (?).
- **Supports (T, ?).** BN5's patch-card table names Rush, Beat and Tango, so the supports may predate BN6.
- **Not in BN5:** Tag chips (T: no tag pair in the save), Crosses, Beast Out, the link navis' own chips (?).

### 2.4 BN4 (Red Sun, Blue Moon)

- **Soul Unison (T, K).** Twelve souls (T: the patch cards that start a battle in one): Roll, Guts, Wind,
  Search, Fire and Thunder (Red Sun), Proto, Number, Metal, Junk, Aqua and Wood (Blue Moon) (the version split:
  K). Chosen on the custom screen by sacrificing a chip; three turns (K). Once a battle (?).
- **Dark chips (T, ?).** They can never be in a folder: the game offers them only in battle (T). When it offers
  them (in BN4, when MegaMan's emotion is worried: ?), and what using one does in battle (the dark state, no Soul
  Unison afterwards: ?), and whether it costs max HP as in BN5 (?).
- **Emotions and Full Synchro (K, T).** The emotion window is new in BN4. A counter hit gives Full Synchro, the
  next attack does double, a hit ends it (K); a patch card can start the battle in Full Synchro (T). Anger, the
  worried and the dark states: their triggers (?). For comparison, BN6's emotion function returns normal, tired,
  Full Synchro, angry and worn out (codes 0, 1, 2, 3 and 5; code 4 is never returned, perhaps a remnant of an
  older game's dark or worried state: ?).
- **Tournaments.** BN4's story is built around tournaments; its netbattle offers single and triple battles (T).
  A bracket is a lobby's business, not the battle engine's. Whether BN4 has a link tournament mode (?).
- **Panels (T).** BN4 has Metal panels, and numbers its panel types differently from BN6 (patch cards: 1 broken,
  3 cracked, 5 metal, 9 holy; BN6's holy is 5).
- **The navi's battle stats, set by effect id (T).** BN4's patch cards apply effects as `set_effect(id, param)`:
  0x05 buster attack, 0x09 the B button, 0x0A the B charge, 0x0C B+Left, 0x12 custom-screen chips (capped at 8),
  0x13 and 0x14 the Mega and Giga limits, 0x18 Triple Supporter, 0x1B panel step, 0x1F Full Synchro at the start,
  0x21 an aura, 0x24 a soul, 0x28 All Guard. If the ids are offsets into BN4's NaviStats, as BN6's bug codes are
  into BN6's (?), BN4's block holds the same kinds of fields on another layout, with a starting soul among them.
- **The chip record** has no lock-on bytes and no dark chip id (T).

### 2.5 The custom screens

| | BN4 | BN5 | BN6 (as built) |
|---|---|---|---|
| Chips dealt | 5 plus Custom programs (K); the stat caps at 8 (T) | the same (K) | `custom_level`, ChargeCross's extra, at most 8; NumbrOpn 10; the hand-shrink bug |
| Picks | 5 (K) | 5 (K) | 5 (`MAX_SELECTIONS`) |
| Regular chip | yes (T) | yes (T) | yes |
| Tag chips | no (T) | no (T) | yes (the tag pair) |
| Program Advances | yes (K) | yes (K) | yes (`builder.rs`) |
| The button under OK | ADD (?) | ADD (?) | Beast Out (`SPECIAL_SLOT`) |
| Transformations | souls offered for a sacrifice (K; layout ?) | the same, and Chaos with a dark chip (K; layout ?) | the Cross window (five Crosses by version), Beast Out |
| Dark chips | offered in battle (T; when ?) | from the folder (T) | from the folder; a bug frag each, else the chip's substitute |
| A link navi's own chip | no (K) | ? | once a round (`SlotKind::NaviChip`) |
| Buttons over slots 8 and 9 | none (K) | none (K) | ChpShufl's re-deal, DustCross's scrap |

What is shared: the folder and its shuffle, the deal, the selection rule (same code or same chip), the Program
Advance builder, the modifier chips, the chatbox's timing for descriptions, the send over the link. What is
not: the slot layout (already a content table, `rules/custom-screen.luau`), the buttons, the sub-screens, and
what OK turns into a transformation.

### 2.6 Chips and statuses

- **The chip record** is shared (§2.2). The engine's `ChipData` fields map onto it, so a BN5 chip definition has
  the same shape. Not shared: BN6's 13 chip families and the secondary elements they give (`ChipFamily`,
  `family_elements`), `ChipTraits` (BN6's list of special cases), and the damage formulas by SP deletion time and
  by link navi level.
- **Statuses** are definitions (`define.status`) with a timer from a fixed set (BN6's nine: paralysis,
  confusion, blindness, immobilization, flash, submerged, invulnerable, freeze, bubble). Paralysis, freeze,
  bubble and invulnerability are in BN4 and BN5 (K); confusion, blindness and immobilization (?). rules/status.luau
  holds BN6's table, including the rows past a group's end that the original reads as garbage: each game has its
  own.
- **Counters** exist in all three (T: the record's counter byte); Full Synchro is common (K).

### 2.7 The netbattle rules

| | BN4 | BN5 | BN6 |
|---|---|---|---|
| Match types | single, triple (T) | single, triple, as plain or team battles (T) | single, triple, random (random out of scope) |
| The battle's settings | drawn by each console from the seeded RNGs, then transmitted (T) | drawn from the RNGs (T); whether they are also exchanged (Tango's comments disagree: ?) | each side draws; the master's wins over the link (T) |
| Other options | ? | stage set (normal, extended), Patch Cards (T) | none |
| Who battles | MegaMan (K) | MegaMan or a team navi (T, ?) | MegaMan or a link navi; the Cross change mode (battle flag 0x40, never in a netbattle) |
| Turn limit and judge | ? | ? | from the 15th screen the gauge stops; the damage judge |

Chip trading and wagers are menu features, out of scope.

### 2.8 BN1–BN3 and the others

- **BN1 to BN3** have no emotion window, no souls, an ADD button, and (BN2, BN3) Style Change. They are an older
  engine (?); porting them would need their own object, collision and flow semantics checked against their
  traces, not only a ruleset.
- **BN4.5 Real Operation** is a GBA spin-off with the BN4 lineage (?). **BN5 DS** (Double Soul) and **Operate
  Shooting Star** are DS games: another platform's timing and sound driver.

## 3. Architecture options

### 3.1 What constrains the choice

- **Frame-exact against each original.** Each game's rules are its own routines, ported branch by branch from its
  disassembly and verified by its own traces. BN4, BN5 and BN6 are a lineage, so many routines will be close
  without being equal. Code shared between games is code whose every change must pass every game's gates.
- **Rollback.** Ticks take inputs only; all simulation state is in `Battle`, plain data, `Clone` and `Hash`;
  both peers run one simulation; presentation is per viewer; scripts are stateless and the VM is never part of a
  snapshot.
- **Performance.** The worst case today is 124 µs per rendered frame (content-model-v2.md §14): a restore
  (4.4 µs), eleven advances (5.6 µs each) each followed by a save (2.6 µs), and a digest (27 µs). Advances are
  three quarters of it. In Luau, a VM step costs about 11 ns, a library call 50 to 90 ns, a field read or write
  160 to 190 ns, a field that makes a handle or a `Vec3` about 300 ns (scripting.md §7); the same in Rust is a
  nanosecond or two.
- **The content model.** Content is stateless; its state is typed fields the engine stores (64 bytes an object or
  action); "content declares no global state of its own"; the engine holds no original numbers.

### 3.2 (a) One Rust core, and a Rust ruleset crate per game

nettai-battle keeps the core and the shared framework; bn6-rules, bn5-rules and so on hold each game's rules;
the core calls the ruleset through a trait, dispatched statically. Content stays per game.

- For: performance and rollback as today; the rules stay typed Rust, ported and verified the same way; each
  game's rules live apart and can't leak into another's.
- Against: the navi framework is threaded with BN6's cases (the Cross and Beast checks in status, idle and chip
  use, the pause-time actions), so it needs either many hooks or a fork per game. A crate boundary is a
  public API: drawn before BN5's needs are known, it will be wrong in places.

### 3.3 (b) The rules in Luau, a game mostly content

The custom screen, the transformations, the emotions and the game-specific parts of the navi framework become
content; nettai-battle exposes more mechanism.

- For: a new game is mostly content; rules are moddable; no Rust per game.
- Against:
  - **Cost.** The navi framework reads and writes a few hundred fields per navi per tick. At 160 to 190 ns a field
    instead of about 1, an advance goes from about 6 µs to something like 50 to 100 µs (an estimate), and the
    worst case from 124 µs to roughly 0.6 to 1.2 ms per rendered frame. That still fits a 16.7 ms frame on a
    desktop, but not comfortably in a browser or on a phone, and it would first need the raw-FFI binding
    scripting.md §9 lists as not built.
  - **State.** The ruleset's state is large and battle-wide: NaviStats, AIData (0x100 bytes an actor), both
    custom screens and folders, the flow machines. Luau would need a new mechanism, battle-wide typed state that
    content declares, which v2 deliberately does without.
  - **Exactness.** These rules are bit-level: flag words, wrapping byte stores, truncating division. Luau numbers
    are doubles; the engine applies the integer rules at the store, but arithmetic in between needs `bit32`
    throughout. And Luau's type check doesn't catch a wrong argument shape across modules (found twice in the
    migration).
  - **Verification.** Moving BN6's verified Rust rules into Luau gains nothing for BN6 and risks it; the traces
    compare the framework's numbers (flow states, navi action numbers), which content would have to keep.

### 3.4 (c) A hybrid

- A Rust **core** (G in §1).
- A Rust **BN-family framework** (most of P): the navi framework, the hit kernel and damage pipeline, the dimming
  service, the obstacle framework, chip use, the flow. Shared between games where their routines match, with
  their values in content tables; forked where they don't.
- A Rust **ruleset per game** (6, and the P that differs): the custom screen, the transformations, the emotions,
  NaviStats and the setup records, the game-only engine kinds, the game's additions to the tick.
- **Luau content per game**, as now: chips, navis, forms and souls, weapons, kinds, stages, rule tables, roles.
- Rules that are only data move to content rule sections, as the custom screen's slot layout already has: the
  gauge rates, the panel types and their flags, the status timers, the fade table, banner timings.

### 3.5 Recommendation, and where the line goes

**(c), built in (a)'s shape in the end:** a Rust core, a shared Rust framework, a Rust ruleset per game (a crate
each once a second game exists), and Luau content per game. Not (b) for the framework or the rulesets.

The line, as rules to apply:

1. **Rust** for what runs every tick for every navi or object in the hit path (cost), what holds battle-wide
   state (plain data in `Battle`, so snapshots stay a `Clone`), and the state machines whose numbers the traces
   compare (the flow, the navi framework's states).
2. **Luau** for what one entity owns (a chip, a weapon, a form's special move, an object kind) and for tables.
3. **Share Rust between games only where their routines are shown to match** (a routine map, §4.2); fork where
   they differ. Don't write `if bn5 { ... }` into BN6's verified code beyond a few named hooks.
4. **Each game is verified by its own oracle**, and a change to shared code runs every game's gates.

| Concern | Layer |
|---|---|
| Object pools, the update list, lifecycle words | core |
| The RNG streams, input, the link's delay, RNG1 per console | core (what advances RNG1: per game) |
| Collision registration and pairing, the panel grid | core |
| Snapshots, the digest, cues, perspective | core (the per-console list: per game) |
| The hit kernel, the damage pipeline, statuses | framework, with per-game tables |
| The navi framework: input, charge, intake, reactions, idle, chip use, movement | framework, forked where a game's routines differ |
| The dimming service, navi chips, obstacles, traps | framework |
| The folder, the deal, the selection rule, the Program Advance builder, the chatbox | framework |
| The custom screen's layout, buttons, sub-screens and result | per-game ruleset (the slot layout is content) |
| Transformations: Cross and Beast (BN6); Soul and Chaos Unison (BN4, BN5) | per-game ruleset, on a shared turn-start sequencer |
| The emotion and mood; anger; Full Synchro's doubling | per game; the doubling in the framework |
| NaviStats, the setup records, unlocks | per-game types; their codecs in `<game>-compat` |
| The flow: intro, banners, gauge, fighting, judge, sets | framework, with per-game tables |
| Drawing the HUD and the custom screen | per-game frontend module |
| Chips, navis, forms and souls, weapons, kinds, stages, rule tables, roles | content per game |

### 3.6 What the seam looks like

A sketch, to be settled when BN5 starts (§4.2):

- **A `Ruleset` trait** with associated types: `State: Clone + Hash + Debug` (stored in `Battle`, so snapshots
  and the digest cover it as now); `PlayerSetup` (BN6's: version, unlocks, bug frags, navi level; BN5's: version,
  team navi, perhaps karma); `CustomResult` (a hand and a transformation). Its hooks are where generic code calls
  BN6's today, about fifteen to twenty: open and tick the custom screen and install the exchange; the turn-start
  transformation step and the form's ending check; the pause-time actions; the emotion and mood; idle's special
  requests (Beast Over's berserk, the SELECT special); a wrapper around a chip's action (the Beast rush); the
  damage pipeline's deletion and weakness cases (the Cross knockout, a form's break); what advances a console's RNG1;
  extra systems in the tick.
- **Dispatch.** Either `Battle<R: Ruleset>`, generic and free, with netplay and the frontend generic too or
  behind an enum `AnyBattle { Bn6(Battle<Bn6>), Bn5(Battle<Bn5>) }` in a small facade crate; or one `Battle`
  holding an enum of ruleset states, a `match` per hook (negligible next to a 5.6 µs tick) and one concrete type
  everywhere, which needs the rulesets to be modules of nettai-battle (a crate cycle otherwise). Not trait
  objects for state: `Box<dyn ...>` makes `Clone` and `Hash` awkward.
- **The content API.** `CoreApi` stays generic; each game adds an extension trait (BN6's: the Beast lock-on, the
  Cross states, the emotions, bug frags, the side special), bound to Luau as its own library (`bn6.*`) and
  declared in the game's root (`content/bn6/bn6.d.luau`). The generic declarations move out of BN6's root, to the
  engine or a shared root.
- **Registries and roles.** `Form` stays (souls are forms). `Lockon` becomes BN6's. Roles become per-game role
  sets (`SoundRole::GregarRoar` and its kind are BN6's).
- **Rollback** is untouched: per-game state lives in `Battle`, inputs are the same, the content hash identifies
  the game's content as it identifies BN6's.

## 4. A staged plan

### 4.1 Now, while BN6 is the only game

Cheap things that don't disturb the verified engine.

**Stage 0: names and docs** (under half an agent-day; docs and trivial code, the build is the gate):

- this document;
- say in docs/engine (a short README) that it is BN6's reverse-engineering record, and decide where another
  game's goes (§6);
- a `game = "bn6"` line in the pack manifest (content.toml) and in a content root manifest, read and checked by the
  loader, so a frontend can tell which ruleset a pack wants (a re-extract, or a reader that defaults it).

**Stage 1: move BN6-only code behind a `bn6` boundary, without changing behavior** (one to two agent-days, then
the full set once: build, tests, content check, both golden traces with rollback, the full lab):

1. A `bn6` module in nettai-battle: `GameVersion`, `Unlocks`, berserk.rs, beast_rush.rs, cross_change.rs,
   cross_special.rs, the form-change sequences, the afterimage, lock-on marker, Cross merge and Beast Over burst
   kinds, `NaviForms`/`FormSet`/`FormKind`, the lock-on and berserk rules, `SpTimes`, the bug-code writer. About
   3,500 lines moved, imports changed.
2. `Battle`'s BN6 fields gathered in one struct (`cross_stats`, `beast_out_used`, `crossed`, `bug_frags`,
   `navi_levels`, `transform_requests`, `turn_transforms`, `transform_seq`, `custom_reversion`, `sides`). The
   digest's value changes with the field order, not its determinism (peers run one build).
3. custom/screen.rs: the Cross window, Beast Out, scrap and re-deal phases into the `bn6` module; the screen calls
   them as before.
4. nettai-render: hud.rs's emotion window and custom.rs into `src/bn6/`.
5. core.d.luau: BN6's declarations gathered in one marked section (the type check is the gate).
6. Optionally, the pack formats `nettai-content/hud` and `nettai-content/custom` renamed BN6's (a re-extract).

The risk is low (moves only) but the diff is wide, so it conflicts with branches in flight: do it in a quiet
moment, with a script other branches can run, as for the rename. **Don't do yet:** the `Ruleset` trait, the crate
split, generalizing NaviStats or the navi framework's branches. Their shape should come from BN5's routines.

### 4.2 When a second game is chosen, before writing its rules

**Stage 2: the routine map and the seam.**

1. **A BN5 disassembly, or at least a routine map.** None is on this machine, and I don't know of a complete
   public one (?). The verification workspace's recompiler can list BN5's routines from the ROM; matching them
   against BN6's battle routines (code compared with what moves masked out, as the Gregar console's map does, but
   fuzzy, since BN5 is another build) gives, per BN6 routine, its BN5 counterpart and "same", "similar",
   "different" or "absent". That decides how much of the framework is shared. Three to five agent-days for a
   usable map; naming routines is more.
2. **The ruleset seam** (§3.6), with BN6 implemented through it: three to five agent-days and the full gates.
3. **The crate split**: nettai-battle (core and framework) and bn6-rules: about a day.

### 4.3 The BN5 port

- **The oracle**: oracle-trace's addresses for BN5 (battle state, objects, AIData, collision, hands, RNGs,
  NaviStats), difftest and the chip lab on Tango's BN5 support, which exists (priming into a netbattle, telemetry,
  a save view that edits the folder and NaviCust). Golden traces need BN5 netbattle replays, which aren't on hand
  (there is one BN4 replay and one BN5 DS replay): record new ones, or let the chip lab make scenarios. Four to six
  agent-days.
- **bn5-compat**: BN5's numbers by key, its setup records (NaviStats, folders, hands, the transformation record)
  and the trace harness. Moving the comparison itself into a shared crate first saves duplicating it. About 2,000
  lines, three to four days.
- **bn5-extract**: sprites, backgrounds, the field and the sound through nettai-content's writers, with BN5's
  addresses and tables; its own HUD and custom-screen graphics; compat/assets.toml names. About 1,000 lines, two to
  three days, plus naming.
- **bn5-rules**: the custom screen (souls, Chaos Unison, dark chips), Soul Unison's transformations and
  three-turn ending, the emotions and the dark state, NaviStats, the flow's differences, and forks of whatever
  framework routines the map shows differ. Perhaps 4,000 to 8,000 lines.
- **content/bn5**: the chips, Program Advances, the twelve souls (forms with weapons and charged chips), navi
  chips, team navis, stages, rule sections, roles. BN6's is 82,000 lines; BN5's might be 50,000 to 70,000, much of
  it adapted from BN6's modules where traces show the same behavior.

All told, roughly half to two thirds of the BN6 effort: the framework, the formats, the tools and the process
exist.

### 4.4 BN4

After BN5. BN4 is BN5's predecessor (no lock-on bytes, dark chips offered only in battle, Metal panels, its own
NaviStats-like layout), so BN5's seams should fit it with less new work. The same steps, smaller.

## 5. Names

### 5.1 In the engine, named for BN6 but generic, or BN6's but unmarked

| Name | Where | Proposal |
|---|---|---|
| `GameVersion { Gregar, Falzar }` | nettai-battle custom/mod.rs; used by content/navis.rs, kinds/player/mod.rs, nettai-frontend driver.rs, nettai-netplay standin.rs | BN6's: move to the `bn6` module. The generic idea is "the game's version" (Falzar or Gregar, Team ProtoMan or Team Colonel, Red Sun or Blue Moon), a per-game type or a content key. |
| `NaviStats::version` ("0 Gregar, 1 Falzar") | setup.rs | BN6's field of BN6's NaviStats. |
| `NaviForms { gregar, falzar }`, `FormData::game` | content/navis.rs | Forms by version, keyed by the game's versions. |
| `Unlocks { crosses, beast_out, beast_out_sealed }`, `screen::CROSSES` | custom/mod.rs, custom/screen.rs | BN6's. |
| `GREGAR_OVER_GLOW`, `FALZAR_OVER_GLOW` | kinds/player/mod.rs | BN6 data in Rust; content (the Beast Over form's glow). |
| `Supports`, `SpTimes` | setup.rs | BN6's (supports perhaps BN5's too, §2.3). |
| `FadeMode::{BeastOut, BeastOutBack, Mode1Transform*}` | battle.rs | BN6's fade table values; the fade table is per game. |
| `actor::request::{CROSS_*, SELECT_SPECIAL, MODE9_A}` | actor.rs | BN6's request bits. |
| `ChipFamily`, `ChipTraits`, `ExtraChipFlags::{RUSH_CANCELS, FREE_SLOT_IN}` | content/chips.rs | BN6's families and special cases. |
| `Registry::Lockon`, `Rules::lockon`, `Rules::berserk`, `Rules::cross_special`, `Rules::sp_*` | nettai-content-api, content/rules.rs | BN6's. |
| Roles such as `gregar_roar`, `falzar_roar`, `beast_over_*`, `cross_death`, `beast_claw` | content/roles.rs, rules/roles.luau | BN6's role set; roles become per game. |
| `ActorField::{BeastLockon, BeastOutSpent}`, `NaviState::Cross*`, `SideSpecial`, `Emotion` | nettai-content-api api.rs, core.d.luau | BN6's API extension (§3.6). |
| The engine kinds' keys `engine/lockon-marker`, `engine/cross-merge`, `engine/beast-over-burst`, `engine/afterimage` | kinds/mod.rs | BN6's kinds; their keys could say so (`engine/bn6/...`), which compat's kinds.toml would follow. |
| core.d.luau, types.d.luau | content/bn6 | The generic declarations belong to the engine (or a shared root); BN6's part stays in content/bn6. |
| Pack formats `nettai-content/hud`, `nettai-content/custom` | nettai-content hud.rs, custom.rs | BN6's layouts under generic names. |
| `BN6_LOAD_TIMES` | nettai-frontend main.rs | Not BN6's: already `NETTAI_LOAD_TIMES` since the rename. |
| `BN6_RATCHET_LOWER` | content-model-v2.md (history) | Dead since the ratchet was deleted; leave the history. |
| The default pack path `data/content/bn6`, `root::bn6()` as the frontend's default | nettai-frontend, nettai-content | BN6's defaults; fine while BN6 is the only game. A `--game` choice later. |
| docs/engine | docs | BN6's reverse-engineering record (275 files cite it). Keep it in place and say so, or move it to docs/bn6 when a second game's docs arrive (one mechanical commit). |

### 5.2 What stays BN6's

- **content/bn6**, with its compat/, rules/ and roles: BN6's content.
- **bn6-extract**: a BN6 ROM's extractor. **bn6-compat**: BN6's numbers, setup records and the trace harness,
  `Compat::bn6()`.
- **netplay's `bn6` module** and `Bn6Input`, by the user's decision. Note: in substance it is the engine's input
  adapter (buttons plus `TickEvents`), with nothing of BN6's in it; worth renaming when the ruleset seam lands, if
  a second game's input record is the same.
- **BN6_PACK** (deprecated: the frontend finds every pack in `NETTAI_PACKS`), **BN6_CONTENT, BN6_COMPAT** and the
  **BN6_LAB_*** variables: they select BN6's pack, content root, compat and lab.
- **The bn6battle-verify workspace**: BN6's oracle, traces and chip lab.
- **docs/engine**'s content (wherever it ends up), and the BN6-only modules once they are behind the `bn6`
  boundary.

## 6. Decisions for the user

1. **The architecture**: the hybrid of §3.5 (Rust core and framework, a Rust ruleset per game, Luau content per
   game), or more of the rules in Luau (§3.3)?
2. **Stage 1 now or later**: the move-only `bn6` boundary is cheap to build but a wide diff; do it now, or when a
   second game is scheduled?
3. **Dispatch**: `Battle<R>` generic with an `AnyBattle` facade, or one `Battle` with an enum of ruleset states
   (rulesets as modules of nettai-battle)? I lean to the generic form with per-game crates, since it keeps each
   game's code out of the others' build, but the enum is simpler for netplay and the frontend.
4. **Which game next**: BN5 (recommended: closest to BN6, with the lock-on and dark chip id fields of BN6's chip
   record) or BN4.
5. **BN5's scope**: netbattle only, as for BN6; Liberation missions out (recommended); team battles and Patch
   Cards in or out.
6. **The BN5 disassembly**: build a routine map from the ROM (recommended as the first BN5 task), or wait for or
   find a community disassembly?
7. **Shared content**: per-game copies that diverge freely (recommended: each game's chips verified on their own),
   or a shared content library from the start?
8. **Where the generic API declarations live**: in the engine crate, or in a shared content root.
9. **Docs**: docs/engine stays as BN6's in place, or moves to docs/bn6 now.
10. **Verification**: one workspace per game (a bn5battle-verify), or bn6battle-verify grown into one workspace
    for every game?
11. **Pack format names**: rename `nettai-content/hud` and `nettai-content/custom` as BN6's now (a re-extract), or
    when BN5's arrive?
