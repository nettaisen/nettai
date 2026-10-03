# Rules in Luau: rulesets of systems, one per player

The design for moving the games' rules out of the engine's Rust into Luau, so that BN5, BN4 and others are mostly
content, so that a game's rules can be mixed with another's, and so that a BN5 player can fight a BN6 player, each
under their own game's rules. Approved on 2026-10-02 with the user's decisions (§9); the slices (§8) are being
built one at a time, and each section gets an "As built" note when its slice lands.

The user's direction:

- the engine is nettai, BN6 is its first game, BN5 (then BN4 and others) follow ([multi-game.md](multi-game.md),
  "Decided");
- "push more rules into luau", rather than a Rust ruleset per game (multi-game.md §3.3, option (b));
- "start abstracting more rules so bn5 support can be implemented", and BN5 "should be composable with bn6 content";
- one ruleset per player: a BN5 player can fight a BN6 player, each with their own game's rules (Soul Unison against
  Cross and Beast Out), while the shared systems (the field, hits, chips, the flow) stay common;
- "allow for even more flexibility, e.g. ruleset mixing and matching so you can have crosses and beast and soul
  unison etc. but have a set of stock rules for each game";
- "libraries should be unshared, each game should export its own library even if they are overlapping. so there's
  bn6:cannon vs bn5:cannon vs bn4:cannon".

Related: [multi-game.md](multi-game.md) (what is BN6's in the engine), [bn5-map.md](bn5-map.md) (BN5's battle code
mapped routine by routine against BN6's), [content-model-v2.md](content-model-v2.md) (definitions, roles, traits,
compat), [scripting.md](scripting.md) (the runtime and its costs), [rollback.md](rollback.md) (snapshots, the
digest, the cost measure), [core-content-boundary.md](core-content-boundary.md) (the layers as built).

Words:

- the **core** is the engine's mechanism (pools, collision, the panel grid, RNG, snapshots);
- the **framework** is the Rust rules the BN4–BN6 lineage shares (the navi framework, the hit kernel, the custom
  screen's chip window, the flow, the turn-start sequencer);
- a **system** is one self-contained piece of a game's rules in Luau (BN6's Crosses, its Beast Out, BN5's Soul
  Unison), with its own state, hooks and custom-screen extras;
- a **ruleset** is a list of systems with the data the framework reads (rule sections, roles); each game has a
  **stock** ruleset, and a mix is a ruleset too;
- a **root** is a content directory (content/bn6) with a manifest; **content** is what a root defines.

Routine names are the original's (BN6's, as the disassembly names them). "Dimming", "cut-in chip", "counter
cut-in", "telop" and "supports" are used as in the rest of the project.

## 0. Summary

- **Each player has a ruleset.** It governs that player: their custom screen's extras, their transformations,
  their emotions, their navi's special controls, their per-player setup. The shared systems (objects, collision
  and hits, the field, chips, the flow) are framework Rust and common to both. Nothing battle-wide is a ruleset's:
  what the whole battle runs by (the field's panels, the flow's timings and banners, the music, the object pools'
  sizes) comes from the stage's game's stock ruleset.
- **A ruleset is a list of systems** (`define.system`, `define.ruleset`). BN6's stock ruleset is its Crosses, Beast
  Out and Beast Over, the Cross special, its emotions, its custom-screen buttons and BN6's own setup; BN5's will be
  Soul Unison, Chaos Unison, its emotions, its Team Battle. A mix (Crosses and Beast Out with Soul Unison) is a
  ruleset defined in content from another ruleset by adding or removing systems.
- **Rust keeps what runs for every object every tick, and the services**, and the framework where the games share
  it. The BN5 map (bn5-map.md) shows more is shared than the survey assumed: the turn-start sequencer and its
  transform record, the navi switch (BN6's unused "Cross change"), the reversion before a custom screen, the
  per-player gauges and SELECT special (the battle flag 0x40 mode, BN5's Team Battle), the lock-on marker,
  afterimage and Beast Over burst kinds, the custom screen's sacrifice and re-deal machinery are BN5's code too.
  They stay framework, under generic names. What moves into Luau is what only BN6 has.
- **State is per side and per system**, engine-owned and typed: each system declares its fields (up to 64 bytes,
  as a kind's); a side keeps one block per system of its ruleset; a system sees only its own block of the side it
  runs for. The
  VM still holds nothing between calls; `Battle: Clone` is still the snapshot and the digest still covers
  everything. **A side's rules read the other side only through the engine**: the navi, its form, HP and statuses,
  and the facts each side's rules push into the framework (its emotion, whether its mood is held).
- **Hooks are per side, coarse, at events**: a turn's start, a custom-screen request, the custom screen's open,
  buttons, windows and result, a hit, a counter, a deletion, a chip's use, a form put on. The common flow calls each
  side's systems in turn. A per-tick Luau call happens only while a state a system set is active (a Beast rush,
  Beast Over's berserk, a Cross special, a custom-screen window, a form's `tick`), never in a plain fight.
- **Cost, measured** (§6): a 10-frame rollback every rendered frame costs 82 to 121 µs on the five golden rounds and
  49 to 135 µs on a basket of Beast, Cross and custom-screen lab scenarios, of 16,667 µs; Luau content is already
  about 80 % of an advance. **Budget** (approved): no per-tick Luau call in a plain fight; every basket scenario under
  500 µs per rendered frame; every slice reports before and after. The estimate at the end of the BN6-only
  slices is under 200 µs.
- **Composable with BN6 content** (§7): roots load together, each named by its game (`bn6:minibomb`, `bn5:cannon`;
  the loader qualifies keys, so a root's own keys don't change). **Each game exports its own library**, even where
  games overlap (`bn6:cannon`, `bn5:cannon`, `bn4:cannon`); content/nettai holds only the engine's API declarations.
  A player's folder may hold any game's chips; a chip behaves as its game wrote it, under its user's rules.
- **Slices** (§8): S0 systems, rulesets, per-side state and the cost tools; then BN6's systems one at a time; roots
  after S2; then the shared rules, parameterized where BN5 differs.

## 1. Where the rules are today

### 1.1 The tiers

nettai-battle is about 31,500 lines without tests (multi-game.md §1): about 5,500 generic, about 20,000 rules
every game of the series has but written with BN6's numbers, about 6,000 that the survey classed as BN6's alone.
The content model already moved everything a chip, kind, weapon, navi, form or stage owns into Luau.

The BN5 map (bn5-map.md) refines the survey with the code itself: of BN6's 4,567 battle routines in scope, 40 % are
in BN5 verbatim, 6 % with other constants, 22 % similar, 5 % different, 27 % absent. Part of what the survey called
BN6's alone is in BN5 (the same code or near it), so it is framework, not a game's rules.

### 1.2 BN6's systems, and what BN5 shares of them

The BN5 column is the map's status of the routines (Team ProtoMan), "same" meaning the same code relocated.

| System | Where in the engine | BN5 | Becomes |
|---|---|---|---|
| The turn-start sequencer (`sub_801483C`, `sub_80148CC`, `sub_8014944`, `sub_8014A00`, `sub_80147E4`), the transform record | transform.rs | same; `sub_801486C` similar 0.93 | framework |
| Beast Out's end check (`sub_80159C6`) and count-down (`sub_8015A38`) | player/mod.rs, battle.rs | similar 0.57 (a soul's turns, presumably); absent | per-side hooks; BN6's in its Beast Out system |
| The form change (`sub_8014A38`) and its five sequences (`sub_8014B18`, `sub_8014D08`, `sub_8014F40`, `sub_801516C`, `sub_80153EC`) | actions/transform.rs | differs 0.36 (BN5's own); the sequences absent but the Cross's | each form names its change action; BN6's sequences in its Cross and Beast systems |
| The revert (`sub_8015614`) | actions/transform.rs | same | framework |
| The Cross merge (actor #0x1B, `sub_80BC650` and its states) | kinds/cross_merge.rs | the entry similar, the rest absent | BN6's Cross system |
| The "Cross change" and its knockout (`sub_802D714`, `sub_802D738`, `sub_802D7A0`, `sub_802D8F0`, `sub_802DD2A`, `sub_802D926`, `sub_802D9B0`), the reversion before a custom screen (`sub_802D6A0`, `sub_802D6C4`) | actions/cross_change.rs, transform.rs, battle.rs `cross_stats` | same or 0.97–1.00 | framework: **the navi switch** (BN5's Team Battle) |
| The battle flag 0x40 mode: per-player gauges, the SELECT special (`sub_802E070`, `sub_802E4E4`, `sub_802F068`) | battle.rs `sides`, idle.rs | same, similar 0.70, 0.67 | framework (BN5's Team Battle mode) |
| The Cross special (`sub_802D4F0`, `sub_802D588`, `sub_80EFDB2`) | berserk.rs, actions/cross_special.rs | absent; differs 0.45 | BN6's Cross special system |
| Beast Over's berserk (`sub_802D322` to `sub_802D5A8`) | berserk.rs | absent | BN6's Beast system |
| The Beast rush (`sub_80EAD9C` to `sub_80EAF36`) | actions/beast_rush.rs | absent | BN6's Beast system |
| The lock-on marker (#0x0F, `sub_80E1520`), the afterimage (#0x28, `sub_80E32B8`), Beast Over's burst (#0x90, `sub_80EA364`) | kinds/ | same | framework kinds (the marker's target panel `sub_80E164A` and freeze are BN6's, absent) |
| The emotion (`sub_8015B54`), the mood setter (`sub_8015BEC`), the counter and mood (`sub_801A200`), the swing bug (`sub_8013DA0`) | player/mod.rs, status.rs | similar 0.67, differs 0.35, similar 0.65, similar 0.77 | framework mood and window; each game's emotion rules in its system |
| The form's status reset and parts (`sub_80144C0`, `sub_8014536`, `sub_8011268`), the weakness break (`sub_8015766`) | player/mod.rs, form.rs, status.rs | similar 0.91, 0.74, 0.93, 0.77 | framework, with form data |
| The forms' chip bonuses (`sub_800EF34`), charged chips by form (`sub_8013236`), charge doubling (`sub_8012AFA`), ChargeCross's fire charge (`sub_80F0608`), Beast Over's glow (`sub_8016A38`) | chip_use.rs, player/mod.rs | absent | BN6's Cross and Beast systems, or form data the framework reads |
| The custom screen: the Cross window (`sub_8027834` to `sub_8027A58`), the BeastOut chip (`sub_80275EC`), the OK result's form (`sub_8029344`, `sub_802937A`), the hand size (`sub_802A40C`) | custom/screen.rs | absent (BN5's hand size similar 0.62) | BN6's systems' custom-screen extras |
| The custom screen: the Beast Out pick's animation (`sub_802770C`), the scrap (`sub_8027406`), the re-deal (`sub_80271F8`), the dark chip's hover | custom/screen.rs | similar 0.94, same, same, same | framework machinery; BN6's buttons that start them in its systems |
| BN6's records and API: `ChipData::{beast_lockon, lockon_mode, traits.no_chain, dark_substitute, hp_bug, formula}`, `Registry::Lockon`, `FormKind` and the forms' BN6 fields, `NaviStats::{version, beast_out_counter, sun, ...}`, `SpTimes`, `bug_frags`, `navi_levels`, `Unlocks`, `GameVersion`, BN6's parts of `CoreApi` and core.d.luau | content/, setup.rs, custom/mod.rs, nettai-content-api | | BN6's systems' data, setup and API module |

### 1.3 What is missing

The content model already gives definitions with function slots the engine calls by handle, typed content state
the engine stores, roles, traits and rule sections. Missing:

1. systems and rulesets as definitions, and a ruleset per player (§2);
2. state that is a side's, not an object's (§5);
3. hooks into the flow, the custom screen and the navi framework, per side (§4);
4. roles and rule sections per ruleset rather than one per pack (§2.3, needed once two roots load);
5. roots that load together, with their keys, compat and assets apart (§7).

## 2. The model

### 2.1 The layers, and where the line runs

| Layer | Language | What |
|---|---|---|
| Core | Rust | Object pools and the update list, collision registration and hit resolution, the panel grid, 16.16 geometry, sprites and animation stepping, RNG, input and the link, cues, snapshots, the digest, the content host |
| Framework | Rust | What the lineage shares: the navi framework (input, charge, intake, statuses, reactions, idle's common priorities, chip use's common path, movement, form application, the navi switch), the hit kernel, the dimming service, navi chips, obstacles, the chip window and its machinery (deal, cursor, selection, Program Advances, modifiers, sacrifice, re-deal, dark-chip hover, sending), the flow (intro, banners, gauge, fighting, the turn-start sequencer, the reversion, judge, sets, the per-player gauges), the shared kinds. It reads rule sections and calls each side's systems |
| Systems | Luau | One game's rules, a system at a time: state, hooks, custom-screen extras, controllers, actions and kinds of its own, data it reads on definitions |
| Rulesets | Luau | A list of systems, with rule sections and roles. A stock ruleset per game; mixes |
| Content | Luau | Chips, kinds, weapons, navis, forms, stages, records, assets by name, as today |

The line, as rules to apply:

1. **Per-tick, per-object loops and the hit path stay Rust.** The object loop, collision pairing, the hit kernel,
   movement, input, charge, sprite stepping and panels run for every object every tick.
2. **What the games share is framework; a difference is data or a hook, never a Rust fork.** The map says what is
   shared. Where BN5's routine differs from BN6's in values, the values become a rule section; in a branch, a hook
   each game's systems fill; where a whole subsystem differs, it is each game's systems. No `if game == bn5` in Rust.
3. **What only one game has is that game's systems**, ported from the Rust (itself the verified port of the
   disassembly) branch for branch and verified by the same traces and lab.
4. **Hooks at events; per-tick Luau only while a state a system set is active.** An unfilled hook costs a check of
   an empty list. A per-tick call is registered by the state that needs it (a controller set on a navi, a wrapper
   around an action, a window open on the custom screen, a form's `tick`) and gone with it.
5. **Push, don't pull.** What the framework reads on a hot path, a system writes into typed framework fields when it
   changes (the side's emotion, whether its mood is held, a navi's controller, whether its input is held). Rust
   never reads a system's state (§5.4).
6. **Data the framework reads every tick stays data.** A form's hover height, fire-charge limit, glow table and
   palettes are fields the framework reads, not functions it calls.
7. **A side's rules see the other side only through the engine** (§4.7).
8. **The traces don't move.** What the traces compare (the flow's states, a navi's action number, objects by slot)
   keeps its values: an action or kind that moves into Luau keeps its compat number under its new key.

### 2.2 Systems and rulesets

A system is a definition. It declares what it keeps and what it does, and nothing in it names another system's
state:

```luau
-- content/bn6/rules/beast/system.luau
--!strict
-- Beast Out and Beast Over: the count of Beast Out turns, the Beast forms, the rush, berserk.

local rush = require("./rush")
local berserk = require("./berserk")

export type State = { counter: number, used: boolean, spent: boolean, exhausted: boolean, check_delay: number }

return define.system {
    id = "bn6:beast",
    -- Each side's fields (the side keeps them, §5).
    state = { counter = "u8", used = "bool", spent = "bool", exhausted = "bool", check_delay = "u8" },
    -- What the player brings (the save's unlock), read-only in battle.
    setup = { unlocked = "bool", sealed = "bool" },
    -- (`setup_defaults = { field = value }`: what a player's setup that says nothing of a field
    -- holds, else zero; BN5's light and dark system's `{ karma = 500 }`, a fresh save's.)
    hooks = {
        round_start = function(side: number) ... end,
        turn_check = function(side: number, request: TransformRequest) ... end,   -- Beast Out runs out
        turn_started = function(side: number) ... end,                            -- a turn in Beast Out spends one
        chip_used = rush.chip_used,                                               -- the rush wraps a lock-on chip
        form_changed = function(navi: Object, from: Form, to: Form) ... end,
    },
    custom = { buttons = { beast_out = { ... } } },
    controllers = { berserk = berserk.controller },
}
```

A ruleset is a list of systems with the data the framework reads for its player:

```luau
-- content/bn6/rules/ruleset.luau
return define.ruleset {
    id = "bn6:stock",
    stock = true,                  -- the game's own rules
    systems = { cross, beast, cross_special, emotion, navicust, dark_chips, link_navis },
    sections = require("./sections"),   -- rule sections: elements, panels, the custom screen's layout, ...
    roles = require("./roles"),         -- what the framework starts, spawns, shows and plays
}
```

- **Stock rulesets**: each game defines exactly one ruleset with `stock = true`, its game's own rules
  (`bn6:stock`).
- **Mixes**: a ruleset may start from another and change its systems: `define.ruleset { id = "mix:bn6-souls", base =
  bn6, add = { bn5_soul_unison }, sections = { define.rules("mix:souls/custom-screen", SOULS_AND_CROSSES) } }`. A
  mix's own `sections` are what its sides read over its game's (built: P1 item 8); a mix needs the layout and roles
  its systems use; the define phase checks every button a system offers has a slot. Mixes are content (a mod's
  folder, §7.2); a setup chooses a ruleset by id.
- **Dependencies**: a system may name systems it needs (`requires = { beast }` on Beast Over) or can't run with
  (`excludes`); the define phase checks every ruleset.
- **Order**: the framework calls a ruleset's systems in its `systems` order. Notification hooks call every system;
  deciding hooks (a deletion kept, a key handled, a chip use wrapped) stop at the first system that decides.

### 2.3 One ruleset per player

`PlayerSetup::ruleset` names each player's ruleset; by default the stage's game's stock ruleset. What each side's
ruleset governs, and what is the battle's:

| Whose | What |
|---|---|
| **The side's ruleset** | the side's systems' hooks, controllers and wrappers for its navi; its custom screen's extras (buttons, windows, keys, hand size, chip checks, its result); its transformations; its emotions; its player setup; the roles the framework uses for that side's navi and objects (the sounds its player hears for its hits, the actions its requests start); the rule sections about one navi (buster recovery, charge rules) and its screen's layout |
| **The battle** (framework, with the stage's game's stock ruleset's data) | the flow (intro, banners, the gauge, turns, the turn-start sequencer, the reversion, judge, sets); the field (panel types, their flags and steps, volcano eruptions); the hit kernel's tables (element weakness); the battle's music and the flow's banners |
| **Both players' games, the larger** (the user's decision) | the capacity-only limits: the object pools' sizes (BN5's actor pool has 16 slots, BN6's 32; the attack and effect pools 32 in both) and any other cap that only bounds how many of something fit |

The stage's game decides the battle's data because the stage is the arena: in a BN6 battle on a BN6 stage, every
table is BN6's and the traces match; in a mixed battle the host picks the arena. Both players' rulesets must agree
on nothing else.

**Capacity takes the larger of the two players' games** (the user, 2026-10-02): a limit that only bounds how many
of something fit, and changes nothing else, is each pool's or table's larger size of the two players' games, so a
BN6 player's chips in a BN5 arena never fail to spawn on BN5's smaller actor pool. Everything else battle-wide
stays the arena's. A same-game battle is unchanged: both sides have one game's sizes, so the traces and the lab are
the same.

**A player's game** is their ruleset's game: a stock ruleset's is its root (a game root: `bn6:bn6` is BN6's); a
mix's is its `base`'s, followed to a stock ruleset; a mix without a base says its `game` (a root name). It is
known when content loads (`RulesetDef::game`), so a setup's two rulesets give the battle's capacities before the
battle starts.

### 2.4 Where it lives

A game's rules are in content/<game>/rules/, a folder per system (approved), with the kinds and actions only that
system uses colocated:

```text
content/bn6/rules/
  ruleset.luau                the stock ruleset
  sections.luau, roles.luau   (and the rule sections' modules as today: elements, panels, collision, ...)
  forms/                      the form changes: BN6's five sequences (the action every BN6 form names), the Cross merge
  cross/                      the Cross window, the Cross bonuses
  beast/                      Beast Out and Beast Over: the button and chip, the counter, the rush, berserk
  cross-special/              the Cross special (DarkInvs' request)
  emotion/                    BN6's emotions, the swing bug
  navicust/                   ChpShufl's re-deal button, NumbrOpn's and the hand-shrink bug's hand size
  dark-chips/                 the bug frag a dark chip costs, its substitute
  link-navis/                 the link navis' own chips and levels
  api.luau                    the `bn6` module content calls (§4.6)
```

A form's own behavior stays with the form (navis/megaman/forms/<form>/), as its weapons do.

## 3. What moves, and in what order

### 3.1 BN6's systems

In slice order (§8). "Per tick" is what each costs in Luau while its state lasts.

| System | Moves | Hooks and points | Per-tick Luau |
|---|---|---|---|
| **beast** (turn-start part, S1) | Beast Out's end check and count-down, the check delay | `turn_check`, `turn_started`, `custom_requested`, `custom_closed` | none |
| **forms** (form changes, S2) | the five sequences of `sub_8014A38` as the action forms name; the Cross merge kind; `beast_out_used`, `crossed` | forms' `change` actions; a system's own actions | per tick of a change, paused |
| **beast** (S3) | the rush (a wrapper), berserk (a controller), Beast Over's drain and exhaustion; `beast_lockon`, `beast_out_spent`, `beast_over_exhausted`; the glow as form data | `chip_used`, `form_changed`; `navi:set_wrapper`, `navi:set_controller`, `navi:hold_input`, `battle.forces_custom` | during a rush; in Beast Over |
| **cross-special**, **cross** (S4) | the Cross special; the Cross bonuses and charged chips by form (`sub_800EF34`, `sub_8013236`, `sub_8012AFA`); the fire charge as form data | `chip_used`, a controller, the charge hook | while a special runs |
| **emotion** (S5) | BN6's emotion rules, anger, the swing bug; the mood stays framework | `navi_hit`, `countered`, `form_changed`; the pushed emotion and `mood_held` | none, but the swing bug's timer while a navi has it |
| **beast**, **cross**, **navicust** (custom screen, S6) | the Beast Out button and BeastOut chip, the Cross window, ChpShufl's and DustCross's buttons, the hand size; `Unlocks`, `GameVersion`, `CrossList` as the systems' setup | `custom.*` | per tick while a window runs |
| all (S7, S8) | BN6's data off the Rust records; BN6's API as the `bn6` module | | none |

### 3.2 Shared with BN5: framework, renamed

These stay Rust and lose their BN6 names (and their BN6 assumptions, where the map shows BN5 the same):

| Today | After |
|---|---|
| `TransformRequest { form, cross_change }`, `TransformSequencer`, the sequencer's check | the transform record `{ form, navi_switch }`; the sequencer asks each side's systems (`turn_check`) instead of checking Beast Out itself |
| The "Cross change", `cross_stats`, the Cross knockout and protect | **the navi switch**: a side's reserve navis (`reserves`), the switch and the knockout that falls back to the reserve (BN5's Team Battle switch) |
| `SideState`, `SideSpecial::{Select, Cross}`, battle flag 0x40 named "per-player gauges" | the Team Battle mode's per-player gauges and SELECT special; the Cross special leaves it for BN6's system |
| `engine/lockon-marker`, `engine/afterimage`, `engine/beast-over-burst` | shared kinds under generic names (`engine/target-marker`, `engine/afterimage`, `engine/burst`), with the BN6-only target and freeze as the Beast system's calls |
| The scrap and re-deal phases | the chip window's sacrifice and re-deal machinery, which buttons of any system start |

### 3.3 Series-common, parameterized by game

Each moves when its BN5 counterpart is read, by rule 2 of §2.1. Known now:

| Rule | Mechanism |
|---|---|
| The object pools' sizes (BN5's actor pool 16) | a rule section of each game's; a battle takes the larger of the two players' games' (§2.3); the arrays stay 32 |
| The custom gauge, banner lifetimes, the final-turn count, the judge | rule sections |
| The fade table (`FadeMode`) | a `fades` rule section by name |
| `NaviStats` (BN5's is 0x60 bytes, some fields moved) | the generic stats stay a Rust record; each game's own are its systems' setup and state; the bug-code writer (by NaviStats offset) becomes each game's table from code to stat; the codecs are each game's compat |
| The form application (`sub_80144C0` 0.91, `sub_8014536` 0.74), charged chips by form (`sub_800F09E` 0.50) | framework, data or hooks as the reading shows |
| The custom screen's hand size (BN5's `sub_802A49C` 0.62), the deal, dark chips | hooks `custom.hand_size`, `custom.chip_check` |
| BN5's own idle and actions (bn5-map.md §6) | BN5's systems, or hooks, as read |
| The hit kernel's and dimming service's differences | data, perhaps hooks |

### 3.4 What stays Rust

- object pools and the update list; collision and hit resolution; the panel grid; RNG; the digest, snapshots and
  cues; sprites and animation; input and the link; the content host;
- the framework, as long as the games share it (§3.2, §3.3): most of the 20,000 lines.

## 4. The API

core.d.luau will declare it; this section is its design.

### 4.1 Hooks, per side

Every hook is a system's and is called for one side, with that side's systems' state in reach (§5). The flow is
the framework's: at each of its points it calls side 0's systems, then side 1's (the original's order wherever it
loops over sides). Arguments are objects, sides, definitions and small spec tables; results are typed and checked
by the binding.

**The flow** (battle.rs, transform.rs):

| Hook | Called | BN6 does |
|---|---|---|
| `round_setup(side)` | once per side as the round is set up (`Battle::new`), before anything reads the side's stats, which it may change | the NaviCust's compile (rules/navicust, docs/design/navicust.md), then the patch cards (rules/patch-cards: added with them, docs/design/patch-cards.md §3) |
| `round_start(side)` | once per side, after the navis spawn | reads its setup into state (the Beast Out counter, the Crosses owned) |
| `turn_check(side, request) -> busy?` | at the sequencer's check (`sub_801486C`), per side | Beast Out runs out (`sub_80159C6`) |
| `turn_started(side)` | after the sequencer, at the turn's start (`sub_800840C`'s end) | a turn in Beast Out spends one (`sub_8015A38`) |
| `custom_requested(side)` | when the custom screen is asked for (`sub_8008452`) | the Beast Out check comes due (`sub_8015A16`) |
| `custom_closed(side)` | when both results are in and the fight resumes (`sub_8009338`) | the Beast Out check's delay is set to 1 |
| `custom_result(side, result)` | when both results are in (`sub_800B3D8`) | |
| `round_end(side)` | once per side as the round finishes | Beast Out used and crossed, read after the battle |
| `folder_check(side)` | only when a tool asks (`Battle::check_folder`: a match's checks, a netplay offer, the editor, live play's random folder), never in a simulation | the folder rules (rules/folder: the size, the chips the pack lists, codes, copies by MB, Mega, Giga and dark limits, the Regular memory, the tag chips' 60 MB) |

**The custom screen** (§4.4): `custom.open(side) -> Offer`, a button's `available(side)` and `press(side)`, a
window's `update(side, pad)`, `custom.keys(side, pad) -> handled`, `custom.hand_size(side) -> n`,
`custom.chip_check(side, chip) -> chip`, `custom.confirm(side)`.

**The navi framework** (kinds/player):

| Hook | Called | BN6 does |
|---|---|---|
| `navi_spawned(navi)` | at the navi's init | |
| `navi_hit(navi, hit)` | once per tick the navi took hits, after the damage (where `sub_801A200` runs), on the hit side's systems | mood loss, anger, the weakness break by form |
| `countered(navi, target)` | when the side's navi's counter landed (`sub_801A200`), on the countering side's systems, before the hit side's `navi_hit` | Full Synchro by its own form, unless the target's mood is held |
| `starting_mood(side) -> mood` | where `sub_8013892` sets the starting mood (`sub_8015C2C`'s 0x80); the first answer | (BN5's light and dark system: by the light/dark value, 0x0801283A) |
| `navi_bug(side, navi) -> skip` | before the navi takes its hit's NaviCust bug (`sub_80139F6`); it may change the collision's `inflicted_bugs`; true skips the bug and the weapons' reload | (BN5's light and dark system: codes 0xFD, 0xFC; hit flag 0x400 from a value of 1000) |
| `navi_palette(side, navi) -> palette` | each tick, the palette of a navi of the player's kind (presentation, `sub_801002C`); the first answer, else the framework's | (BN5's light and dark system: 0x0800DD94) |
| `navi_deleted(navi) -> keep` | where the framework would delete the navi (deciding) | |
| `chip_used(navi, chip) -> Use` | once per chip use, after the common path | the rush (a wrapper), the Cross bonuses, EraseCross's flag |
| `form_changed(navi, from, to)` | after a form is applied | the tired emotion, the glow |

**Per tick, only while set** (§4.3): a navi's controller, its wrapper, a form's `tick`, a custom-screen window.

What BN5 is expected to use (bn5-map.md §4–§5): `turn_check` where a soul's turns run out, forms' change actions
for Soul Unison, a custom-screen button for choosing a soul (the button under OK, where BN6 has Beast Out, gives up the
last chip picked for its family's soul; a dark chip makes it Chaos Unison; twelve souls, six per version) on the shared
sacrifice machinery, `custom.chip_check` and the buttons' `available` for light and dark MegaMan (a light MegaMan
can't use dark chips or DS navi chips; a dark one can't use other navi chips and has no Soul Unison button), and
`chip_used` for Chaos Unison's held dark chip. None of it needs Rust per game once these exist, which is the test of
the seam. (Tango's BN5 matches are Team Battles that leave battle flag 0x40 off, use the normal custom screen and never
switch navis, since the switch needs a navi chip from the Battle Chip Gate: the flag-0x40 mode and the navi switch stay
shared framework code, but don't drive the BN5 plan.)

### 4.2 Frequency and cost

| Kind of call | When | Calls per advance, typical |
|---|---|---|
| Flow hooks | paused turn starts and custom requests | 0 while fighting; a few a turn |
| Custom-screen hooks | the custom screen | 0 while fighting; 1 a tick per side while a window is open |
| Event hooks | at the event | well under 0.1 |
| Controllers, wrappers, form `tick` | while set | 0 in a plain fight; 1 a tick for the navi that has one |

A call into Luau costs 0.3 to 0.5 µs before it does anything and typically 2 to 6 µs for the work these do (§6.1).

### 4.3 Controllers, wrappers, pause actions, form actions

Framework extension points that take over a navi's tick while a system says so. Each is a definition, set on a
navi by handle in a framework field (in the snapshot; Rust checks one `Option` per tick):

- **A controller** (`define.controller { update = function(navi) -> Outcome }`) replaces idle's decision while set:
  it returns nothing, moved, a chip or the buster, and the framework carries the outcome out as idle does. BN6:
  Beast Over's berserk, the Cross special.
- **A wrapper** is an action (`define.action { wraps = true, ... }`) the dispatcher runs instead of the navi's action
  while set; it runs the wrapped action when it chooses (`navi:run_wrapped()`). BN6: the Beast rush.
- **Pause actions**: while the battle is paused, the pause handler runs the framework's own (the navi switch, its
  knockout, the revert) and the form change of the form being changed into.
- **A form's actions**: a form names the action that changes a navi into it and, if not the framework's, the one
  that reverts it (`change = cross.change_into_cross`). So a mixed ruleset's forms change by their own game's
  sequence: a Cross by BN6's, a soul by BN5's.

### 4.4 The custom screen: a Rust core with Luau extras, one per player

The chip window is the framework's (approved): the deal, cursor, selection, Program Advances, modifiers,
descriptions, the run message, hiding, OK, the sacrifice and re-deal machinery, the dark chip's hover, sending.
Each side's screen is simulated from its own player's joypad (as now) and reads **its own side's ruleset**:

- `SlotKind::Button(ButtonHandle)`: a slot the side's layout gives to a system's button (BN6's Beast Out button,
  ChpShufl's re-deal, DustCross's scrap). A button's `available(side)` decides its state at the open and after
  each pick; `press(side)` what A does, which may start the shared machinery (`custom.sacrifice`, `custom.redeal`).
- `Phase::Window(WindowHandle)`: a system's window, whose `update(side, pad)` runs every tick and returns stay,
  back to choosing, or a pick. BN6's Cross window and its Beast Out animations are windows. A window may also stand
  for the whole screen, if a game's screen shares little with the core (BN5's Team Battle screen, which BN5 runs
  instead of the shared one under battle flag 0x40, bn5-map.md §4, is decided when it is read).
- `custom.keys(side, pad)` is asked first in the choosing phase, for keys the chip window gives no meaning (BN6: Up
  at the top row or on OK opens the Cross window). Systems are asked in order; the first that handles a key takes it.
- The result: the framework's part (the hand, the navi stats, the transform record) and each system's `result`
  fields. bn6-compat writes BN6's transform record into them.

Two rulesets' screens coexist because nothing in a screen is battle-wide: each side's screen, phase, window state
and result are that side's; the fight resumes when both results are in, whatever each screen did. Each viewer is
shown their own screen, drawn by their ruleset's game's frontend module (§4.8).

### 4.5 What a system can call

Everything content can, plus, from modules under its game's rules/ folder (a lint keeps content out):

- **State**: `system.state()`, `system.setup()` (read-only), `system.result()` (in `custom.confirm` and
  `custom_result`): the calling system's fields of the side it was called for (§5.3);
- **The flow**: `battle.fade(name, speed)`, `battle.hud(part, shown)`, `battle.forces_custom(side, on)`, the turn,
  the battle mode;
- **The navi framework**: `navi:apply_form(form)`, `navi:revert_form()`, `navi:set_controller(c)`,
  `navi:set_wrapper(a)`, `navi:run_wrapped()`, `navi:hold_input(on)`, the navi switch, requests and state bits by
  name;
- **The custom screen**, inside its hooks: that side's cursor, selection, picks, hand size, sounds and looks, and
  the shared machinery;
- **Pushed facts**: `battle.set_emotion(side, name)`, `battle.full_synchro(side, on)`, `battle.mood_held(side, on)`.

### 4.6 A game's API for content, and emotions

Content today calls BN6-only API in Rust (`battle.bug_frags`, `me.beast_lockon`, `me.beast_out_spent`, MstrCros's
Crosses). After S8 these are BN6's Luau module, content/bn6/rules/api.luau, declared in content/bn6/bn6.d.luau. Each
function reads the user's side's systems through the engine and **says what it does when the user's ruleset lacks
the system**: `bn6.bug_frags(side)` is 0, `bn6.spend_bug_frags` does nothing, `bn6.crosses(side)` is empty. A chip
whose use makes no sense without a system says so (`requires = { "bn6:cross" }`), and a setup check refuses it in
a folder whose ruleset lacks it.

Emotions are each game's (BN6's five; BN5's differ): a system declares its emotions' names, pushes the current one
per side, and `battle.emotion(side)` returns that name with its game. Full Synchro and the mood, which the lineage
shares, are framework fields the systems push and the framework's doubling and window read.

### 4.7 The other side, only through the engine

A system's state is reachable only by that system for the side it was called for (§5.3); no API takes another
side's state. What a side's rules need of the other side, they read from the engine: its navi object (position,
HP, statuses, form as a definition and its common record), its hand, its custom screen's status, and the facts its
rules pushed (its emotion, Full Synchro, whether its mood is held). Where BN6's code reads BN6 state of the other
side, that state becomes a pushed fact every game can fill. The first case: `sub_801A200` gives the counterer Full
Synchro unless the hit navi's Beast Out is spent or it is exhausted after Beast Over; the hit side's emotion system
pushes `mood_held`, and the counterer's reads it.

### 4.8 Presentation

The frontend draws each viewer's HUD and custom screen with the module of that viewer's ruleset's game; that module
reads its systems' state by field name through a small accessor on `Battle`. The opponent's navi, objects and HP
are drawn as now. Making the drawing itself data or Luau is out of this design's scope.

## 5. State

### 5.1 Per side, per system

There is no global script state: the VM holds nothing between calls. What a system keeps across ticks it declares,
as kinds and actions declare theirs, and the engine stores:

- **Blocks**: a system's `state` (per side), `setup` (per player, read-only in battle) and `result` (the custom
  screen's result). Each is a schema with the content-state field types (`bool`, `u8` to `i32`, enums, `object`,
  `vec3`, references to definitions and assets) and fixed arrays of any of them.
- **Storage**: `Battle::rules: [SideRules; 2]`, each the side's ruleset and a `ContentState` (a schema's id and 64
  bytes, as a kind's state) per system of it, in the ruleset's order; and `PlayerSetup::rules`, a block per system
  for its `setup`. Plain data, zeroed at the round's start: `Battle: Clone` is still the snapshot, `#[derive(Hash)]`
  still the digest. A ruleset lists at most 16 systems; BN6's stock ruleset about 7, so a snapshot grows by about
  1 KB.
- **No battle-wide ruleset state**: what the whole battle runs by is the framework's (§2.3).

### 5.2 What it replaces

| Today (Rust) | After |
|---|---|
| `Battle::beast_out_used`, `ActorData::{beast_out_check_delay, beast_out_spent, beast_over_exhausted}`, `AttackVars::beast_lockon`, berserk's state, `NaviStats::beast_out_counter` | the beast system's |
| `Battle::crossed`, the Cross special's ticks and request | the cross and cross-special systems' |
| `ActorData::{anger, emotion_swing_ticks, swung_emotion}` | the emotion system's |
| `Battle::{bug_frags, navi_levels}`, `RoundSetup::sp_times`, `Unlocks`, `GameVersion`, `CrossList`, `NaviStats::{version, sun, folder_tags, chip_shuffle, number_open}` | the BN6 systems' setup and state |
| The custom screen's Cross window and Beast Out state; `RoundMemory::{crosses_used, beast_out_used}` | the cross and beast systems' |
| `cross_stats`, `sides`, the transform record and sequencer | framework, renamed (§3.2) |

### 5.3 Who sees it

`system.state()` is the calling system's fields of the context side. The engine sets the context when it calls
into a system: a hook's side; for a system's own controller, wrapper, action or kind, the side of the navi or
object it runs for. A system has no handle on its other side's state or on another system's.

### 5.4 Rust doesn't read it

The framework never reads a system's state: what it needs every tick is pushed into its own typed fields, and what
it needs at an event comes back as the hook's result. Tools outside the simulation may: the frontend's per-game
presentation and each game's compat codecs (bn6-compat writes BN6's setup and result fields from saves and traces
by name).

### 5.5 Rollback

Nothing changes in the contract (rollback.md §8.2): all state is in `Battle`, plain and hashed; system functions are
stateless and checked so; inputs are the buttons; peers whose setups agree run the same rulesets (the setup names
them, the content hash covers their definitions and code). Tests: a battle rolled back in the middle of a turn's
start continues identically; a system hook writing a module local is refused at load; netplay's synthetic
netbattles run with each side on another ruleset.

## 6. Performance

### 6.1 Today

`rollback_cost` (rollback.md §6): the worst case of a 10-frame rollback on every rendered frame (a restore, eleven
advances each followed by a save, a digest). Release build, Apple M1 Max shared with other agents (load average
about 24); best of five runs; engine main 20fcf9e4:

| Round or scenario | µs per rendered frame | Advance (µs) |
|---|---|---|
| soundmod 1 (frames 10164–12164) | 90.6 | 3.70 |
| soundmod 2 | 116.9 | 5.97 |
| soundmod 3 | 82.3 | 3.12 |
| machgun 1 (Beast Out from frame 642) | 121.3 | 6.60 |
| machgun 2 | 92.8 | 4.04 |
| lab forms/falzar/beast-over | 134.6 | 7.83 |
| lab forms/falzar/beast-rush-chain | 64.4 | 1.86 |
| lab forms/falzar/cross-to-cross | 64.6 | 1.62 |
| lab forms/falzar/beast-then-cross | 96.1 | 4.49 |
| lab custom/cross-window-keys | 49.1 | 0.45 |
| lab flow/anger-runs-out | 59.4 (one run) | 1.17 |

(The lab rows run the 2,000 frames around each scenario's busiest frame, as for the golden rounds.)

- **A rendered frame is about 45 µs fixed plus eleven advances**: the digest 19–20 µs, a restore about 3 µs, eleven
  saves about 2 µs each.
- **Luau is already most of an advance.** With the calls into Luau timed: soundmod 1 makes 0.86 kind updates and
  0.08 action updates per advance, about 2.6 of its 3.3 µs; machgun 1 0.74 and 0.39, about 5.8 of 6.6 µs;
  beast-over about 79 %. A call costs at least 0.33 to 0.46 µs and typically 1.7 to 5.4 µs; the Rust engine is
  under a microsecond of an advance.
- So what decides the cost is how many Luau calls a tick makes. A field access from Luau costs 140 to 360 ns
  against a nanosecond or two in Rust (scripting.md §7).

### 6.2 What moving naively would cost

With the framework asking Luau at each of today's BN6 branch points every tick (the emotion for the palette, the
Cross and Beast request bits, the form's hover and fire charge, the specials in idle, the lock-on marker), about
five to eight calls per navi per tick: 20 to 50 µs per advance, 250 to 600 µs per rendered frame in a plain fight.
Rules 4 to 6 of §2.1 exist to avoid this.

### 6.3 Budget (approved)

1. **A plain fight stays free.** No slice adds a per-tick Luau call to a navi in a plain state (no controller,
   wrapper, window or special): soundmod 1–3 and machgun 2 stay within the noise (±10 %).
2. **The ceiling**: every basket row under **500 µs per rendered frame** (3 % of a frame), best of five.
3. **No per-slice measurement** after S2 (the user, 2026-10-02: "don't bother doing all this costing, it's
   fine"). Through S2 every slice reported its basket before and after; the tools stay for when a slice adds
   per-tick Luau to a plain fight, or someone asks.

The estimate at the end of the BN6-only slices: plain fights unchanged; machgun 1 about unchanged (the marker stays
Rust, the rush only during rushes); beast-over plus berserk (about 5 µs an advance) near 190 µs. Two rulesets in one
battle change nothing: each side's systems run for their own side.

### 6.4 How to stay within it

- **Coarse hooks**: one call per event, with the facts in its arguments.
- **Gating**: an unfilled hook is an empty list; a per-tick call is set by the state that needs it.
- **Push**: emotions, Full Synchro, the mood held, controllers, input holds, forced custom screens.
- **Data on hot paths**: a form's hover, fire charge, glow and palettes are fields.
- **Rust primitives for per-tick work shared by games** (the target marker, the afterimage).
- **The binding**, when the budget needs it (approved): reusing each object's userdata, then the raw-FFI binding of
  scripting.md §7 and §9.

### 6.5 Measuring a slice

- **rollback_cost** gains a frame range (`--frames A..B`) and an optional `luau-profile` feature that counts and
  times calls into Luau per advance, by kind, action and hook.
- **The basket**: the rows of §6.1 plus each slice's own, best of N, by the verification workspace's
  tools/rollback-cost.sh, the branch and main alternating so the machine's load hits both.

## 7. Composable with BN6 content

### 7.1 What it means

1. **Games' content loads into one engine at once.** A player's folder may hold any game's chips; a player may use
   any game's navi and forms their ruleset offers; a battle may be on any game's stage.
2. **Each player plays by their own ruleset**: a BN5 player (Soul Unison) against a BN6 player (Cross and Beast
   Out), on common field, hits, chips and flow.
3. **Each game is still exact in its own battles**: a BN6 player against a BN6 player, with BN6 content on a BN6
   stage, matches BN6's traces; likewise BN5. A mixed battle has no original; its rules are this design's.
4. **Rules mix**: a ruleset can take systems from several games (Crosses, Beast Out and Soul Unison together), as
   content.

### 7.2 One namespace (R4; it replaced R1's roots)

The user (2026-10-02): "i think the idea of packs is just kind of wonky anyway, maybe you should just have it all
in a flat namespace and then in the chip ids directly have bn6:cannon or whatever", and "so loading assets must
also be fully qualified as well".

- **Every folder of content/ loads**, one namespace: content/bn6, content/bn5, and any other. A folder is named as
  its game; there is no manifest. content/nettai holds the engine's API declarations and defines nothing.
  `--content` (and `$NETTAI_CONTENT`) names the content directory, by default the repository's content/.
- **Every id is written in full**, its game first: `id = "bn6:minibomb"`, `bn5:cannon`; a section's name
  (`define.rules("bn6:panels", ...)`), the roles' id (`bn6:roles`), a stock ruleset's (`bn6:stock`, `bn5:stock`),
  a system's (`bn6:beast`); a mix keeps its own descriptive id. The loader refuses an id without its game, naming
  the folder's game as the fix. The engine's own entries keep their `engine/...` keys, of no game.
- **A definition's game is its id's prefix.** The loaded games are the folders and the ids' prefixes, by name
  (`Defs::roots`; a `RootId` is a place in it).
- **Asset names are in full**: `asset.sprite("bn6:bomb")`, any loaded pack's; a name without its pack's game is
  refused by the loader, a match's background by the match check. A pack keeps its own names (`bomb`); its game is
  its manifest's.
- **Modules require by path**: `require("@bn6/rules/beast/system")` names any folder; `./` and `../` stay within
  the folder.
- **Compat and locale tables are keyed by full id**, as content writes them; compat is per game (bn6-compat reads
  content/bn6/compat, bn5-compat content/bn5/compat), and a trace names only its game's content.
- **Lookups are exact** (`Defs::*_by_key`): tools, tests, setups and match files write ids in full.
- **No home.** What a battle reads is the arena's (the stage's game's) or a side's (its ruleset's, §2.3); a tool
  with no battle takes the game that has the thing. A frontend's and the match tool's default game is BN6's, by name
  (`nettai_match::DEFAULT_GAME`).
- **content/common** is a folder of behavior only: modules the games' folders share by path
  (`require("@common/...")`), with no assets of their own, so it needs no pack, and no compat or locales. A common
  module holds no game's ids or asset names and requires only common modules: it exports the behavior as makers
  that take a game's look and constants (`traps.make_kind(id)`, `bomb.make(look)`, `cannon.action(spec, look)`;
  the looks' types in content/common/types.d.luau). Each game's folder defines its own kinds, actions and records
  with them and keeps its looks: BN6's modules at their old paths are thin wrappers that pass BN6's (so BN6's
  callers are unchanged), BN5's folder makes its own (`bn5:trap-chip`, `bn5:attachment`, ...). A helper with no
  game data moves whole (`@common/regions`, `@common/panels`, `@common/dimming`). Where BN5 still uses a BN6
  definition as its own (BN6's effects, a few BN6 collision rows), it requires it from `@bn6/...` and says so
  (docs/design/bn5-map.md §15.6, "Shared code"). It loads as a root of no game (`keys::SHARED`): no roles, no
  rules, no field, no compat ids; what defines a game's folder alone (a test, gen-content's check) loads it beside
  (`testing::add_shared`), and the checker types each folder with its types.d.luau.
- **Version variants keep their suffixes** (`-falzar`/`-gregar`, `-protoman`/`-colonel`); region (US, JP) is a
  field, not a namespace.
- **Handles** intern over the union in byte order of the ids; peers with the same content hash have the same
  handles.

### 7.3 Each game exports its own library

There is no shared content library (the user's decision). Each game's folder defines its own chips, kinds, builders,
forms and systems, even where they overlap with another game's: `bn6:cannon`, `bn5:cannon`, `bn4:cannon` are three
definitions, each verified against its own game. When BN5's routine is the same as BN6's (bn5-map.md says which),
BN5's module may start as a copy of BN6's, and then belongs to BN5 (or the two share it from content/common, §7.2).

A folder that composes games (a mix of rules, a mod) requires their modules by path
(`require("@bn6/rules/cross/system")`) and names their definitions by id. The in-repo tests use such a folder for
mixes (§7.6).

### 7.4 Assets

A definition names its assets in full: `bn5:cannon`'s sprites are `bn5:...`, BN5's pack's. The engine's asset
handles cover every loaded pack (R3a); the frontend and the audio draw and play each asset from its own pack (R3b).

#### The field's art in a mixed battle (approved by the user, built)

The user approved this on 2026-10-02: "yes, borrow bn5 art then fall back". It is built as described here; the "As
built" note, "The field's art in a mixed battle", has the details and gates.

The field's rules follow the stage's game (§2.3); its art does too.

1. **The field's art is the stage's game's**, like its rules: the panel tiles, their palettes and palette cycles,
   the highlights, the front edges and the background all come from the arena's pack. Both viewers see the same
   field.
2. **Every pack declares which panel types it draws** (field.json's `panel_types`). BN5's field has 11 panel types
   (BN6's 13) and one highlight block (BN6's two).
3. **A panel type the arena's field doesn't draw** (a BN5 chip making BN5's sea in a BN6 arena):
   - the simulation runs it by the type's own definition, so the rules never depend on the art (P1): a type the
     arena's `panels` section doesn't name takes its rule (flags, sound, expiry, behaviors) from the first other
     loaded game whose section names it, in root order; one no loaded game names keeps an empty rule, never a
     panic (`sections::fill_panel_types`);
   - it is drawn from the field of the same game: the first loaded game, in root order, whose `panels` section
     names the type and whose field draws it. The borrowed blocks keep their own tiles, palettes and palette
     cycles, in a palette set of their own, so the arena's palettes are left as they are;
   - if no loaded pack draws it, it is drawn as its owner's normal panel tinted halfway to magenta, never a hole.
4. **A highlight the arena's field lacks** is the first loaded game's that has it, else tinted the same way.
5. **The static audit reports** each loaded game's pack that doesn't draw a panel type its game names (extract
   it again), and says, without counting it, which panel types and highlights an arena draws tinted.

### 7.5 Which rules apply where

- **A player's ruleset rules that player** (§2.3): their custom screen, transformations, emotions, controls, and the
  side's rule data and roles.
- **The battle's data is the stage's game's** (§2.3): the field, the flow, the pools.
- **A definition's behavior is its own**: a chip's use, a kind's update, a weapon's setup, a form's actions and
  hooks run as their game wrote them, on the framework's services.
- **A definition's common record means the same everywhere**: a chip's codes, element, class, MB, damage, counter
  parameter, flags and lockout; a form's element, weakness, buster bonus, weapons and charged chips.
- **A system's own data on definitions is an extension it declares** (`extends = { chip = { lockon = "lockon?" } }`),
  with defaults for definitions that lack it: BN6's Beast system gives a chip without a lock-on mode no rush, so it
  runs where the navi stands. A definition of its own game writes the extension flat, as today; another game's
  definition may add it under the system's key to say how that system should treat it.
- **A game's API module says what it does when its system is missing** (§4.6), and content that needs a system
  `requires` it.

### 7.6 Worked cases

- **A BN5 player against a BN6 player** (BN5 stage). The flow, the field and the pools are BN5's. Each custom screen
  is its player's: the BN5 player's offers souls, the BN6 player's the Cross window and Beast Out. At the turn's
  start the shared sequencer fades out once, the BN5 navi changes by its soul's change action and the BN6 navi by
  its Cross's, and it fades back in. A counter by the BN6 navi gives Full Synchro by BN6's rule, reading the BN5
  navi's pushed `mood_held`; a hit on the BN5 navi runs BN5's emotion rules.
- **A BN5 chip in a BN6 player's folder.** `bn5:sword` is dealt and picked by its common record; its use runs BN5's
  action on the framework's services. In a Beast form it has no BN6 lock-on, so no rush. A Cross's bonus applies by
  family, as the Cross system defines families for chips without its extension.
- **A BN6 chip in a BN5 player's folder.** `bn6:minibomb` throws as in BN6. BugRSwrd's charged shot asks
  `bn6.spend_bug_frags`, which does nothing without BN6's dark-chips system, so it fires the plain shot. MstrCros
  `requires = { "bn6:cross" }` and can't be put in a folder whose ruleset lacks Crosses.
- **A mix**: a ruleset with BN6's Cross and Beast systems and BN5's Soul Unison offers both on its player's screen
  (a layout with slots for both), and a form changes by its own game's action.
- **A BN5 stage with a panel type BN6 lacks**: the stage's game decides the field, so its panels are BN5's.

### 7.7 Verification

Each game is verified in its own battles by its own oracle: BN6's traces and lab through bn6-compat, BN5's through
its compat when it exists. Mixed battles have no oracle; in-repo tests check they run and roll back: every chip
under each stock ruleset, a battle with a different ruleset on each side, and a mix.

## 8. The slice plan

### 8.1 Every slice

- Branches from current main, merges main often, lands on main before the next starts.
- Ports from the Rust as it stands; every branch kept; anything the move shows wrong is fixed and listed.
- Gates once at the end (phase-b-brief): the build without warnings, `cargo test --workspace`, the content check,
  `gen-content check` when compat or definitions changed, both golden traces with rollback at every latency and the
  sound calls, and the full lab with the sound gate. (The rollback cost basket before and after was a gate through
  S2; the user dropped it, §6.3.)
- Verify-side changes on a verify branch of the same name.
- Docs: this document's "As built" notes; core-content-boundary.md and content-migration.md where the line or the
  patterns move; docs/engine where it names moved code.

### 8.2 The slices

| # | Slice | Moves | New mechanism | Lab focus |
|---|---|---|---|---|
| S0 | **Groundwork** | none | `define.system`, `define.ruleset` (stock); per-side system state and player setups; `PlayerSetup::ruleset` (default the stage's game's stock); the `system` library and its call context; the hook lists with `round_start` wired; the lint; rollback_cost's `--frames` and `luau-profile`; tools/rollback-cost.sh | all (a no-op) |
| S1 | **Turn starts** | Beast Out's end check, count-down and check delay into BN6's beast system; the sequencer framework with per-side hooks | `turn_check`, `turn_started`, `custom_requested`, `custom_closed` | forms/*, custom/take-back-*, flow/*; machgun 1 |
| S2 | **Form changes** | the five sequences into BN6's forms system as the action the forms name; the Cross merge kind | forms' `change` actions; a system's own actions | forms/* |
| R | **Roots** (after S2) | none | root manifests, qualified keys, content/nettai declarations, per-root compat and packs, roles and sections per ruleset, the battle's data from the stage's game, mixes (`base`, `add`, `remove`); a test root with its own stock ruleset; a battle with a ruleset per side | everything |
| S3 | **Beast Out and Beast Over** | the rush, berserk, Beast Over's drain and exhaustion, the marker's targeting and freeze; the kinds renamed shared | wrappers, controllers, `chip_used`, `navi:hold_input`, `battle.forces_custom` | forms/*/beast-*, machgun 1 |
| S4 | **The Cross special and the Cross bonuses** (with the "Cross change" renamed the navi switch, §3.2) | berserk.rs's special, cross_special.rs, `sub_800EF34`, `sub_8013236`, `sub_8012AFA`, the fire charge as data | controllers, the charge hook | forms/*/cross-*, the specials' scenarios |
| S5 | **Emotions** | BN6's emotion rules, anger, the swing bug; the mood framework | `navi_hit`, `countered`, pushed emotion, Full Synchro, `mood_held` | flow/anger-*, flow/synchro-*, flow/counter-* |
| S6 | **Custom-screen extras** (a: buttons, hand size, setup; b: the Cross window) | the Beast Out button and chip, ChpShufl's and DustCross's buttons, the hand size, the Cross window; `Unlocks`, `GameVersion`, `CrossList` | `SlotKind::Button`, `Phase::Window`, `custom.*`; the frontend's module per game | custom/*, forms/* |
| S7 | **BN6's data** | `FormKind` and the forms' BN6 fields, `ChipData`'s BN6 fields, `Registry::Lockon` | systems' `extends` with defaults | everything |
| S8 | **BN6's API** | BN6's parts of `CoreApi` and core.d.luau | the `bn6` module, `requires`, bn6.d.luau | everything |
| B | **The binding** (when needed) | none | object userdata reuse; raw FFI | everything |
| P… | **Series-common, by the BN5 map** | §3.3: pool sizes, the `NaviStats` split and bug-code tables, form application, hand size, BN5's idle | rule sections and hooks | by area |

Sizes: S0 about a day; S1, S2, S5, S7, S8 one to two days each; S3, S4, S6 and R two to three; about three weeks of
one agent for S0 to S8 and R. P is sized as BN5's port reads its routines.

### 8.3 Order and coordination

- S0, S1, S2, then R (approved: right after S2, so BN5's content has its root early), then S3 to S8. S5 after S3
  (Beast Out's tired state is an emotion); S6 after S3 and S4.
- Parallel work: the patch cards change `NaviStats` and setup (the systems' setup in S6 and the `NaviStats` split in
  P come after them); localization and the JP work add content and compat entries (additive); the BN5 work's
  extractor writes BN5's pack with `game = "bn5"` and BN5's own names (§7.2), which R reads.

### 8.4 Risks

- **Fidelity**: forms/* alone is several hundred scenarios; the port is Rust to Luau, each routine's branches
  already known; the lab and machgun 1 are the gates.
- **Luau's checks** don't follow `require`d shapes (scripting.md §3.3): systems' shared types go in the game's
  declarations, and the lab catches the rest.
- **The frontend**: S6 changes what it reads; the frontend's custom-screen frame comparison is an extra gate.
- **Wide diffs**: S7, S8 and R touch most modules' records or keys; they go when few branches are in flight, with
  a script other branches can run.

## 9. Decisions

**The user** (2026-10-02):

- one ruleset per player, not per battle (§2.3);
- the custom screen is a Rust core with Luau extras (§4.4);
- rulesets mix and match systems, with a stock ruleset per game (§2.2);
- no shared library: each game exports its own, even where they overlap (§7.3).

**The user**, later the same day: **patch cards are an engine concept.** A patch card is a definition kind of the
engine's (`define.patch_card`) and a player's installed cards a typed field of their setup
(`PlayerSetup::patch_cards`), as in BN4, BN5 and BN6; what a card's effects do is a game's rules', a system of its
stock ruleset (BN6's patch-cards system, which `round_setup` runs; docs/design/patch-cards.md §3).

**The user** (2026-10-02, for R2): **capacity-only limits take the larger of the two players' games**; everything
else battle-wide follows the arena's (the stage's) game (§2.3). The capacity-only limits are the object pools'
sizes and any other pure capacity cap; a player's game is their ruleset's (a mix's, its base's). A same-game
battle is unchanged.

**The user** (2026-10-02): **the field's art in a mixed battle** is §7.4's proposal: "yes, borrow bn5 art then
fall back". A panel type the arena's field doesn't draw is drawn from the field of the game that names it, else
as a tinted normal panel.

**The coordinator**: the budget as proposed (§6.3); content/<game>/rules/ with a folder per system (§2.4); roots
right after S2 (§8.3); loader-qualified keys (§7.2); the cheaper binding only when the budget needs it (§6.4).

**Agreed with the BN5 work** (§7.2): `<game>:<key>` added by the loader; a root's manifest has `name`, `assets` and
`requires`, a pack's `game`; version suffixes kept; region a field; per-game exports.

**Made here** (the user asked for sensible choices while away; each can be revisited):

1. **The battle's data is the stage's game's** stock ruleset's: the field, the flow's timings and banners, the
   music (§2.3). A same-game battle is then exactly that game; a mixed battle's host picks the arena. (The pools'
   sizes were here; the user's R2 decision makes them the larger of the two players' games.)
2. **Mixes are rulesets in content** (`base`, `add`, `remove`), and a setup picks a ruleset by key; setups don't
   compose systems themselves, so every mix is checked when content loads (§2.2).
3. **Systems combine by order**: notification hooks call every system; deciding hooks stop at the first that
   decides (§2.2).
4. **State is a block per system per side** (64 bytes, as a kind's), each system seeing only its own; at most 16
   systems a ruleset; no battle-wide ruleset state (§5).
5. **The BN5 map moves into the framework** what BN5 has too (§3.2): the sequencer and transform record, the navi
   switch, the reversion, the Team Battle mode's gauges and SELECT special, the shared kinds, the sacrifice and
   re-deal machinery, under generic names.
6. **A form names its change action** (§4.3), so a mix's forms change by their own game's sequence.
7. **Cross-side facts are pushed**: a side's rules read the other side through the engine's fields only; BN6's
   cross-side read (a counter's Full Synchro against a held mood) becomes `mood_held` (§4.7).
8. **BN5's Team Battle screen** is decided when its code is read: extras on the core if it shares the core, a
   whole-screen window if not (§4.4).

## As built

### S0, groundwork (2026-10-02)

- **Definitions.** `define.system { id, state?, setup?, hooks? }` and `define.ruleset { id, stock?, systems }` are
  registries of their own (`Registry::System`, `Registry::Ruleset`, `SystemHandle`, `RulesetHandle`). A system's
  `state` and `setup` tables are schemas like a kind's (keys `system:<key>/state`, `/setup`; 64 bytes each); its
  `hooks` are function slots (`system <key>'s hooks.<name>`), checked against the hooks the framework has; a
  ruleset lists at most 16 systems, each once; a content has at most one stock ruleset (until roots, slice R).
  `content::defs::{SystemDef, RulesetDef}`, `Defs::{system, ruleset, stock_ruleset, ruleset_by_key}`.
- **Per player.** `PlayerSetup::ruleset` (none: the stock ruleset) and `PlayerSetup::rules` (each system's setup
  block, in the ruleset's order; empty: zero; `PlayerSetup::set_rule(content, system, field, value)` writes one by
  name). `PlayerSetup` is `Clone`, no longer `Copy`.
- **State.** `Battle::rules: [rules::SideRules; 2]`: each side's ruleset and a `ContentState` per system, zeroed at
  `Battle::new`; in the digest (the destructuring guard) and the snapshot.
- **Hooks.** The framework calls a hook with `Battle::notify_systems(hook)` (side 0's systems in order, then side
  1's) or `notify_side(side, hook)`, through `HookCall::System { side, slot, hook }`. The first hook, `round_start`,
  runs once per side after the navis spawn (mode_intro). BN6's stock ruleset (content/bn6/rules/ruleset.luau) lists
  no systems yet, so nothing in a BN6 battle changes.
- **The `system` library.** `system.state()`, `system.setup()` (read-only) and `system.side()` reach the system's
  blocks of the side the running call is for: the binding keeps that context per call (`bind::SystemCtx`, set by a
  system's hook call, cleared for every other call), so content's own calls and anything a system's hook leads to
  can't reach a system's state (`behavior::tests::a_systems_state_is_out_of_reach_of_content`). nettai-content-check
  refuses `system.*` calls outside modules under rules/.
- **Tests.** The test content (testdata/content/rules/systems.luau) has two made-up systems and two rulesets;
  `rules::tests` check each side runs its own ruleset's systems for itself, a player plays by the ruleset their
  setup names, a system's setup reaches it alone, and the state is in the digest and the snapshot.
- **Cost tools.** rollback_cost takes `--frames A..B` (or `all`), and with nettai-netplay's feature `luau-profile`
  reports the calls into Luau per advance (`behavior::profile`). The verification workspace's
  tools/rollback-cost.sh runs the basket of §6.1, best of N, alternating a checkout with a baseline.
- **Gates** (on main 3b1ffc6f, a pack from all four ROMs): the build without warnings, 384 tests, the content check
  (653 modules), `gen-content check` (0 errors), both golden traces in full with rollback at every latency and their
  sound calls (and the 189 replay rounds), the full lab 6299/6299 scenarios, 5,641,457 frames, with the sound gate (0
  rounds differ).
- **Cost**, best of 15 against main, alternating (load average 20 to 60): soundmod 1–3 95.0 / 117.9 / 81.5 µs per
  rendered frame (main 96.5 / 114.0 / 77.9), machgun 1–2 137.9 / 86.9 (161.3 / 86.2), the lab basket within 3 %:
  no change beyond the noise, as expected of a slice that adds no per-tick call. (Best of 5 at a load of 100 was not
  enough: rows moved by ±60 %. Use 15 on a busy machine.)

### S1, turn starts (2026-10-02)

- **Hooks.** `turn_check(side)` (the sequencer's check, `sub_801486C`: for a side that isn't changing form, and
  for one that asks for a Cross change), `turn_started(side)` (after the sequencer, `sub_800840C`'s end),
  `custom_requested(side)` (`sub_8008452`, before the reversions) and `custom_closed(side)` (`sub_8009338`, the
  fight resumes). The flow calls each for a side whose navi is there, side 0 first. The sequencer and the transform
  record stay the framework's; BN5 has them too.
- **BN6's beast system** (content/bn6/rules/beast/system.luau, in the stock ruleset after the patch cards) fills
  them with what was Rust: Beast Out running out (`sub_80159C6`: the navi's Beast Out is spent, a Beast form asks
  to revert), a turn in Beast Out spent (`sub_8015A38`), and the check's delay (AIData+0x0F), now the system's
  state: `ActorData::beast_out_check_delay`, `check_beast_out_end` and `count_down_beast_out` are gone.
- **API.** The navi stats' `beast_out_counter` is writable and `starting_form` readable.
- **Hook set**, with the patch cards' `round_setup(side)` (`Battle::new`, before anything reads the stats, which it
  may change), which fits the design: every hook is a side's, the flow calls each side's systems in turn.
- **Gates** on main 26d7d912 (localization and the patch cards merged, the second with conflicts in the hook list
  and the binding, where `round_setup` met these hooks) with a pack from all four ROMs: the build without warnings,
  399 tests, the content check (829 modules), `gen-content check` (0 errors), both golden traces in full with rollback
  at every latency and their sound calls, the 189 replay rounds, the full lab 6521/6521 scenarios, 5,756,487 frames,
  with the sound gate. (Before the patch cards, on ca90a72e: the same, the lab 6299/6299.)
- **Cost**, best of 15 against main: +1.0 % to +3.8 % on every row (soundmod 1–3 83.4 / 107.6 / 74.0 µs, machgun
  1–2 105.1 / 84.8, beast-over 119.9): the four hooks' calls at turn starts and custom-screen requests, all paused.

### S2, form changes (2026-10-02)

- **BN6's forms system** (content/bn6/rules/forms): the change into a form (`sub_8014A38` and its five sequences, a
  Cross, Beast Out, a Cross in Beast Out either way, Beast Over) is a Luau action, `forms/change` (compat 0x1C, as
  the original's CurAction), which every BN6 form other than the base form names as its `change`; the Cross navi's
  image merging (actor #0x1B) is a Luau kind, `forms/cross-merge` (compat keeps its slot). What a change notes of the
  round (`sub_800AB2E`: Beast Out used, crossed, read after the battle) is the system's state:
  `Battle::{beast_out_used, crossed}` are gone. The system sits in the stock ruleset after the patch cards.
- **A form's `change`** (`FormData::change`): the pause handler runs the action the form asked for names (a form other
  than the base form must name one); unpaused, such an action is the instant chips' (the original's CurAction 0x1C), as
  before. The revert (`sub_8015614`, the same code in BN5) and the Cross break stay the framework's.
- **A system's own actions** (`define.system { actions = { ... } }`): running one, the binding gives it the system's
  state of the navi's side (`system.state()`), if the side plays by that system (`Defs::action_owner`,
  `Battle::system_slot`, `ContentHost::update_action`'s system).
- **API** for a game's rules over a navi (§4.5): `clear_invulnerable`, `face_default`, `reset_charge`,
  `end_full_synchro_aura`, `drop_statuses`, `end_statuses` (`sub_801A264`, which isn't `clear_statuses`'s whole status
  word: the first lab run showed Beast Over keeping three flags the wrong one dropped), `overlay_stepping`,
  `take_off_form_overlay`, `put_on_form_overlay`, `load_form_sprite`, `reset_status`, `end_anger`,
  `form_change_target`, `pin_overlay`; `battle.shake_camera_secondary`, `battle.burst`; the navi stats' `form`
  writable.
- **Roles** only the change used are gone (the sounds of a form change, a Cross, Beast Out, the roars, Beast Over's
  rumble, the Cross merge; the effects of a form change and Beast Over's beast and blast): the Luau names the assets.
- `kinds::cross_merge` and `EngineKind::CrossMerge` are gone; the engine has 21 kinds of its own.
- **Compat**: compat/actions.toml gains `"forms/change" = 0x1C`, compat/kinds.toml `"forms/cross-merge"` (actor
  #0x1B); the roles removed leave compat/rules.toml. Verify (branch rules-design): gen-content checks every form but
  the base form names `forms/change`.
- **Gates** (on main 356f5971 merged): the build without warnings, 399 tests, the content check (832 modules),
  machgun 1074/1331 and soundmod 21962/14933/20436 with 48 rollback rows and the 189 legacy rounds, the lab
  6521/6521 (5,756,487 frames) with 0 sound rounds differing. After main a33fc1da (locales, fonts): the build
  without warnings, 402 tests, the content check, gen-content check 0 errors.
- **Cost**: the basket under load 34 to 64 was noise either way (soundmod 1 read 119 against 345 µs in one pass, 236
  against 198 alternating); the user then dropped the measurement (§6.3). A change runs Luau only during its own
  frames (about 100 a change); a plain fight runs none of S2's.

### R1, roots and qualified keys (2026-10-02)

Slice R is three parts, each landing on its own: R1 (this), R2 (rulesets per side: roles and sections per ruleset,
the battle's data from the stage's game, mixes, a battle with a ruleset per side), R3 (several asset packs:
`SpriteId` with its pack, assets qualified across packs, the frontend and audio over several packs, the field's
art if the user approves §7.4's proposal).

- **Roots.** A content root has a manifest, `root.toml` (`RootManifest`: `name`, `assets`, `requires`):
  content/bn6/root.toml names `bn6`, content/bn5/root.toml (the BN5 work's) `bn5`, the engine's test content
  (crates/nettai-battle/testdata/content/root.toml) `test`. `nettai_content::root::read_all` reads a root and the
  roots it requires (each the sibling directory of its name), its own first; `pack::load_battle` loads them all.
  `Scripts` holds the modules by name (`bn6:chips/minibomb/chip`) and the roots (`Scripts::roots`, the content's
  own first: its home); `Content::define` checks them (a valid name, not `engine`; no root named twice; every root
  required is loaded; every module in a loaded root).
- **Qualified keys.** The define phase qualifies every definition's key with its module's root (`bn6:minibomb`,
  `bn6:minibomb/action`, `bn6:chips/x#1`; a root's roles `bn6:roles`, its sections `bn6:elements`); modules,
  compat and locale tables write keys unqualified, as before, so no content module changed. Engine entries keep
  `engine/...`. `nettai_content_api::keys` has `qualify`, `local`, `root_of`, `names`. Records that hold a key
  hold the qualified one.
- **Lookups by key** (`Defs::*_by_key`, `Defs::record`): a qualified key as it is; an unqualified one in the one
  root that defines it (none if two roots do). They are for tools, tests and setups by name, never the simulation.
- **Requires across roots**: `require("@bn6/rules/beast/system")`, from the root itself or a root its manifest
  `requires`; `./` and `../` stay within a root.
- **The battle's data is the home root's** for now (R2 makes it the stage's game's): the rule sections, the roles
  and the stock ruleset (`Defs::stock_ruleset`, with `stock_ruleset_of(root)`) are the content's own root's; each
  root has at most one stock ruleset.
- **content/nettai** holds the engine's API declarations: core.d.luau moved there whole; every root type-checks
  against it and its own `*.d.luau` (`nettai-content-check`). BN6's parts of it (`NaviStats`' BN6 fields, the
  types it borrows from content/bn6/types.d.luau) move to content/bn6 in S8; until then content/bn5 doesn't
  type-check on its own.
- **Packs say their game**: content.toml's `game` (bn6-extract writes `bn6`, bn5-extract `bn5`). One pack loads
  until R3: every root's `assets` must be the pack's game; a pack without `game` loads with a warning.
- **Locale tables**: a root's are unqualified; loaded, `Strings::qualified(root)` keys them as the definitions
  and `merge` makes one table (`locale::load_all` for another language over a root and its requires);
  `Strings::of_root` gives a root's own back. `locale::check` checks a root's table by its name.
- **Compat at its boundary**: bn6-compat's `Compat::root` (`bn6`), `def_key` qualifies compat's keys for the
  content, `compat_key` gives a definition's own key (None for another root's: it has no BN6 number); the
  engine's test content, a root of its own, stands in for BN6's (`Compat::root_in`). bn5-compat qualifies with
  `bn5` the same way. Verify: gen-content looks compat up by local keys and qualifies compat's where it compares
  them with the definitions'.
- **Tests**: two roots load together (`content::scripts` tests: keys per root, cross-root requires refused without
  `requires`, a stock ruleset per root, ambiguous lookups); BN5's root loads beside BN6's (nettai-content's lint
  tests), up to BN5's chips having no use yet, which the define phase refuses until the BN5 port writes them.
  Tests on one root compare `Battle::local_kind_key` and `keys::local`.
- **Patch cards** (merged from patch-card-type): `define.patch_card` keys are qualified like every other
  (`bn6:skarab`), `Defs::patch_card_by_key` looks up as the others do, the `[patch-cards]`
  locale tables are qualified per root, and compat's patch-cards.toml is read at its boundary.
  `PlayerSetup::set_rule_elem` is gone: no system's setup holds an array now (the cards are
  `PlayerSetup::patch_cards`).
- **Verify**: gen-content and the trace tests read keys at the boundary as bn6-compat does;
  tools/traces-against.sh and gen-content-against.sh build with the engine checkout's Cargo.lock (getgud moved
  under a run once).
- **Gates** (on main a2dbe624 merged, with patch-card-type and getgud-fixes): the build without warnings, 444
  tests, the content check (832 modules), gen-content check 0 errors, machgun 1074/1331 and soundmod
  21962/14933/20436 with 96 rollback rows (getgud's and rennet's) and the 189 legacy rounds, the lab 6521/6521
  (5,756,487 frames) with 0 sound rounds differing.

### R2, each side's game (2026-10-02)

- **A game's data is its root's.** Each root's roles (`define.roles`, `<root>:roles`) and rule sections
  (`define.rules`) are its game's: `Defs::roles[RootId]`, `Content::rules[RootId]` (each root's sections over
  `Content::base_rules`, the engine's defaults or the test content's made-up tables). `RootId` is a root's place
  among the loaded roots, the content's own first (`RootId::HOME`). A ruleset's roles and sections are its game's;
  a mix's own `roles`/`sections` come when a mix needs them (BN5's Soul Unison layout).
- **Whose data a battle reads** (`BattleGames`, made from the setup): the arena is the stage's root; each side's
  game is its ruleset's (`RulesetDef::game`). `Battle::{arena_rules, arena_roles}` are the battle's,
  `side_game_rules(side)`, `side_roles(side)` a side's, `rules_for(r)`, `roles_for(r)` those of the side object `r`
  is on (the arena's for an object of neither). How each read was sorted (§2.3):

  | Whose | Reads |
  |---|---|
  | the arena's | the field (panel types, steps, slides, the trail sound, the field's eruption, panel physics: push, ice, bubble bob), the hit kernel (element weakness and the family elements, the plain spark, the anchor region, collision types and the damage word's statuses, ice freeze), obstacles (thrown, encased, absorbed), the flow's banners and music, the telop banner, AntiRecv's counterattack kind, the entry flash, the sine table |
  | the side's (its object's) | the actions its requests start (counters, the strike, the turn, the Cross's knock-out and protection, the forced charged shot, the volley), its navi's body collision types, effects and sprites (deletion, recovery, trap mark, charge glow, status visuals, aura, lockon and hit markers, burst), its supports, Mode 9's kinds, the first barrier hook, NaviCust bug statuses and sparks, the guard spark, the Beast rush's lock-on and the berserk rules, the empty hand, the HP bug, the Cross special's chips, the sounds its player hears (`sound_for`), its victory and defeat music, its custom screen (`custom::GameLibrary`: layout, Beast Out and invalid chips, the Program Advance banners) |
  | the chip's own game's | an SP chip's slot and its deletion-time steps |

  In a battle of one game every column is that game's: the traces and the lab are unchanged.
- **Capacity** (the user's decision): the `pools` section (BN6's content/bn6/rules/pools.luau: 32 each);
  `Objects::with_capacity` takes each pool's larger size of the two sides' games (`BattleGames::pool_capacity`).
- **Mixes**: `define.ruleset { base, add, remove, game }` (`read_rulesets`): a stock ruleset has no base; a mix
  lists `add`/`remove`, not `systems`; `add` of a system it has, `remove` of one it hasn't, a base that leads back
  to itself, a game that isn't a loaded root with a stock ruleset, and a ruleset in a root of no game with neither
  base nor game are refused. testdata's rules/mix.luau is one (the stock rules less the forms system, plus the
  marker).
- **Tests**: a mix runs its systems; a second game root (`twin`, requiring `test`: its stock ruleset, the test
  roles with another pause sound, 16 actors) beside the test content: each side reads its game's data and the
  battle the arena's; capacities are the larger game's; duels of test/twin, twin/twin and mix/twin run 1,200 ticks
  and a copy from tick 600 ends with the same digest.
- **Tools**: `Content::home_rules`, `Defs::home_roles` for what has no battle (codecs' zeroed chip, `Library for
  Content`); nettai-content's roles lint checks each game root's roles.
- **Verify**: gen-content, the stubs report, the data and music tests read the home root's roles and tables (one
  game's); gen-content compares the pools section with the ROM's (32 each).
- **Gates** (on main f0cb0d4d merged; later main merges were docs and BN5 tools): the build without warnings, 449
  tests, the content check (833 modules), gen-content check 0 errors, machgun 1074/1331 and soundmod
  21962/14933/20436 with 96 rollback rows and the 189 legacy rounds, the lab 6521/6521 (5,756,487 frames) with 0
  sound rounds differing: a battle of one game reads every table as before.

### R3a, asset handles (2026-10-02)

Option (b), the coordinator's decision: the engine's asset ids are handles over the loaded packs (content-model-v2
§12 step 6's plan), not the original's numbers.

- **Handles.** `SpriteId(u16)`, `SoundId(u16)`, `BannerId(u16)`, `BackgroundId(u16)`, `MugshotId(u16)` are each
  a kind's handle: its name's place among the loaded packs' names of that kind in byte order. What a pack calls an
  asset is its own: `PackSprite { category, index }` (the old `SpriteId`, "cc-ii"), a song-table number, a banner's,
  background's, mugshot's number; `InPack<T> { pack: PackId, id }` pairs it with its pack.
- **Asset names over several packs** (`AssetNames`): `packs` (games in byte order, by `PackId`), and each kind's
  map from qualified name (`bn6:bomb`) to `InPack`; `of_packs`/`of_pack` build it from packs' own indices
  (`PackIndex`, a pack's `assets.toml`, unqualified); `number(kind, h)`, `sprite(h)`, `sound(h)` give a handle's
  pack and number, `sprite_handle`/`sound_handle`/`number_handle` the way back.
- **Content names an asset in its root's pack**: `asset.sprite("bomb")` in a module of root R is
  `<R.assets>:bomb`; a qualified name (`bn6:bomb`) only of its own pack or a pack of a root it requires
  (`nettai_luau::Pack::asset_name`, `with_assets`). Definitions hold the qualified name; records the handle.
- **Loading several packs**: `pack::load_battle_packs(content, packs)` (`load_battle` is one pack): each pack says
  its game (one without is the content's own root's, with a warning), two of one game are refused, every root's
  `assets` must be loaded. Animation timing is per pack, keyed by handle (`Animations::add_pack`: every name of a
  sprite is a handle of it).
- **The edges convert.** bn6-compat (`Ids::pack`, `asset`, `asset_number`, `background`, `sound_number`), the
  audio (`nettai_audio::Songs`: a sound handle's song; `BattleAudio::new(bank, songs)`, `Songs::cue`), the frontend
  (`nettai_render::packs`, nettai-frontend's then: a handle's sprite, banner, mugshot, background number in its
  pack; one pack's
  graphics until R3b), verify's gen-content (`decode::numbered_sprite` and `defined::Numbered` compare by the pack's
  numbers) and the sound tests. The engine itself did no arithmetic on a sprite's numbers but two afterimage
  and form-overlay object parameters, which now carry the handle.
- **Tests**: the test content is a synthetic pack (`testing::pack_index`, its timing by `PackSprite`), with
  `testing::{sprite, sound, sprite_named, pack_sprite, add_pack}`; `twin` has its own pack (its pause sound, song
  0x40, a sprite): a battle of the test and twin games plays two packs (`AssetNames::packs` `["test", "twin"]`),
  each side hearing its game's pause; Luau names resolve per root and are refused outside a root's packs.
- **The match crate** (merged from match-editor): a match's background is a name in the content's own pack unless
  qualified (`nettai_match::background`); the editor lists them by that name.
- **Gates** (on main 4d1890ee): the build without warnings, 450 tests, the content check (833 modules), gen-content
  check 0 errors, machgun 1074/1331 and soundmod 21962/14933/20436 with 96 rollback rows and the 189 legacy
  rounds, the lab 6521/6521 (5,756,487 frames) with 0 sound rounds differing. After main 1fd091c1 (match-editor: the
  frontend and the new match and editor crates, no simulation): the build without warnings, 456 tests.

### R3b, the frontend and the audio per pack (2026-10-02)

- **Graphics per pack** (`nettai_render::packs::Packs`, nettai-frontend's then): every loaded pack's `Bundle` by
  `PackId`, and the
  content's own (its home root's `assets` pack). An asset draws from its own pack: a sprite's sheet, a banner's
  glyphs (its pack's HUD), a mugshot (its pack's HUD), a background. A chip's icon and picture are its game's pack's
  (the chip's root's `assets`), under its key there (`gundels3`, not `bn6:gundels3`): the frontend had looked them up
  by the qualified key since R1 and drew none, which the audit now shows (`--audit` on machgun: 5 problems before,
  0 after); the compat numbers the custom screen reads (the navi's, a Program Advance pick's) go by
  `Compat::compat_key` for the same reason. The custom screen and the chatbox are the local side's game's pack's
  (its ruleset's game); the HUD's frame and the field are the content's own pack's. The field's art in a mixed
  battle stays as it was (the proposal in §7.4 was still the user's to decide; it is built since, "The field's art
  in a mixed battle" below): `Stage::new` takes the field's
  bundle and the background, which may be another pack's. A frontend of one pack (`Renderer::new`) draws every
  asset from it.
- **Sound per pack** (`BattleAudio::with_banks`, `AudioOut::with_banks`): a driver per pack, each with its own
  bank, sound calls and queue; a cue plays on its sound's pack's driver (`Songs` holds each handle's
  `InPack`); stopping the music stops every driver's, and music of another pack stops the player that played the
  last; the drivers' outputs are added, one frame's samples whatever the number of packs. `BattleAudio::new`
  is one pack's; `driver_of(PackId)` reads a pack's driver.
- **Loading** (nettai-frontend): `--pack` repeats, one pack a game (`--pack <bn6> --pack <bn5>`); the content
  loads over all of them (`pack::load_battle_packs`), and `pack::pack_paths` puts the directories in the content's
  pack order for the graphics and the sound. (Since: the frontend and the editor load every pack in
  `data/content` or `$NETTAI_PACKS`, `--pack` only overriding one, and the roots beside BN6's that load:
  `pack::find`, `pack::load_found`, docs/frontend.md §1.) The player's language applies to the content's own pack. The audit
  checks a cue's song in its own pack's bank.
- **Tests**: `packs::tests::each_asset_draws_from_its_own_pack` (a twin root and pack beside the test content,
  their navis' sprites the same pack number: each drawn from its own pack's sheet; a root's game; a chip's icon by
  its local key; a mugshot's pack); `each_pack_plays_its_own_songs` (two banks: music moves from one pack's player
  to the other's; one frame's samples). By hand: the scratch BN6 pack with a copy of it as game `aaa` (sorting
  first, so BN6 is `PackId(1)`): machgun's audit 0 problems with both, and frames 150 to 2000 rendered with one and
  with two packs are byte for byte the same.
- **Not done**: the editor's chip pictures (`nettai-editor` `pictures.rs`) still load one pack (nettai-assets'
  API did not change); a BN5 pack's own HUD and custom screen formats come with BN5's extraction.
- **The frontend audit is a gate** (verify's `tools/audit-against.sh`, in `checks-against.sh` and the brief's full
  set): every golden trace and the custom screen's lab scenarios drawn and played, 0 problems. Nothing else in the
  full set draws a frame, which is how the chip art lookup broke unseen.
- **Gates** (R3b on main 1c0a3634): the build without warnings, 458 tests, the content check (833 modules),
  gen-content check 0 errors, machgun 1074/1331 and soundmod 21962/14933/20436 with 96 rollback rows and the 189
  legacy rounds, the lab 6521/6521 (5,756,487 frames) with 0 sound rounds differing; the audit 72 traces (635,424
  frames), 0 problems. After main 42560ed5 (BN5's port, and main's own chip art fix, whose lines R3b's replace):
  the build without warnings, 459 tests, the audit again 0 problems. Frames against main's frontend (`--text
  original`, verify's `identity.sh`, the sample and custom-screen lists with machgun and crossdivide): 154
  scenarios identical (183,877 frames), 22 differ, each by one of two more R1 regressions this fixes, both compat
  lookups by the qualified key: a Program Advance pick's code beside its name (`Cannon A`), and a link navi's
  emblem on the custom screen (MegaMan's before).

### P1a, BN5's framework (2026-10-02)

The engine additions BN5's data and rules need (docs/design/bn5-map.md §15.3), with the coordinator's answers.
BN6 stays byte-identical; BN5's side is unit tests and asm citations, and the BN5 replays where they reach.

- **Panels (items 1 and 3).** `PanelType` gains `Metal` (13), `Lava` (14) and `Sea` (15), appended. A game's
  `panels` section names only its types (BN6 its 13, BN5 its 11); one it doesn't name is another loaded game's
  (§7.4). What a type does is engine code keyed by the type, its numbers in the section: `expires` (ticks to normal,
  blinking the last 60; BN6's roads 0x708, BN5's lava and sea 960; a panel's one `expire_timer`, which its new type
  sets), `burn` (lava: 50 in fire, shifted by the weakness, the panel turning normal with the arena's spark
  `panel_burn`; in the navi's and the obstacles' intake, first, as BN5's 0x080178EC and 0x08017A18 have it),
  `drains` (sea: fire bodies, as poison drains any), `holds` (sea: 20 ticks immobilized at a move's end, with the
  arena's effect `panel_splash`), `submerges` (sea: a body that dives, the actor's status 0x20, is submerged on it
  and none off it), `slide` (metal: by the direction of the move, the steps tried in turn, BN5's tables at
  0x0800C920 and 0x0800C9C0; a form with `stands_on_metal`, BN5's soul 5, doesn't slide), `cleared_by` (the
  element whose hitboxes turn the type normal: fire grass, aqua the volcano and lava, wood roads and metal; BN6's
  conversions now read it). Per game `mend` (BN6 0x258 and 0x1E0 in battle mode 1, BN5 600 in both). BN5's panels
  section is registered with its own types; bn5-compat maps BN5's 5, 8 and 10 to them.
- **Push (item 2).** `reactions.push_reading = "bn5"`, the arena's: BN5's 0x0800C9D8 reads the side-0 hits'
  modifier toward the navi's front, else the side-1 hits' the other way. The hit resolver keeps the modifiers by
  the hitter's side (`CollisionData::hit_mod_by_side`, BN5's +0x18 and +0x19, 0x08016AA6), which BN6 doesn't read.
  BN5's obstacle push (0x08017AD8) isn't ported.
- **Optional roles (item 4).** `statuses.ice_freeze` and `hooks.encased` may be absent: no freeze, nothing
  encased.
- **Families (item 5).** `ChipFamily::Recovery` and `Invisible`, appended.
- **The mood (item 6), changed from the answer first given** (the coordinator approved the change): BN5's mood byte
  is the engine's mood (`NaviStats::mood`), so instead of a `bn5:mood` system with hooks there is
  `battle.gain_mood(side, n)` (BN5's 0x08012802: 0 and 0xFF stay, 254 at most) and `battle.lose_mood(side, n)`
  (`sub_8015C12`, BN5's 0x08012820, which the hits' loss now calls), and `heal.action`'s optional `mood`. BN5's
  emotion rules (its counter's 0x80, the soul) are BN5's port and S5's.
- **The flow (items 7 and 13).** The `flow` section, the arena's: `result_words` (a custom screen's result's words
  on the link, a tick each, read of the sending side's game: BN6 50, BN5 49; the screens close the tick after both
  results are in, in both games: bn5-map.md §15.3 item 13; the AIData +0x0F BN6 sets then is BN6's beast system's
  `custom_closed`, which BN5's ruleset lacks), `sequencer_before_custom` (BN5 opens the screen straight after the
  reversions), `escape_check` (BN5 has no `sub_800AAD6`), `result_wait` (102 ticks, 94 in a special battle and for a
  win in battle modes 4, 5 and 8: `sub_80081A4` and `sub_800825A`, whose short wait the engine had left out; BN5's
  special 65). The BN5 replays: 115 past setup matched their first 219 frames, now 284 to 428 (4 match every frame;
  32,959 frames match in all, 25,170 before); the next stop is the panels at frame 426 (bn5-compat's).
- **Collision words (item 9).** No engine change: `lint::self_bit_targets` reports a collision type that tests
  BN6's 0x80 self bit, named by a module that uses another root's modules (BN5's own row 0x3D, `probe`, does;
  nothing hands it to BN6's modules).
- **The chip's own game (items 10 and 11).** The `chip-use` section, read from the chip's own root
  (`Battle::chip_rules`): `leave_on_use` (BN5's dimming handler and instant chips leave the action on the frame
  they run) and `anti_navi_sparkle` (BN6 16 down and 32 up, BN5 16 up; `SPARKLE_DY` and `SPARKLE_Z` were Rust).
- **Not yet:** item 8 (a mix's own sections) and item 12 (a base form per game), after R4; BN5's navi intake as a
  whole (its order, its holy panel's light/dark rule at 0x08017136) is BN5's port.
- **Gates** (on main d4d846cb): the build without warnings, 465 tests, the content check (837 modules),
  gen-content check 0 errors (it decodes BN6's new rule fields), machgun 1074/1331 and soundmod 21962/14933/20436
  with 96 rollback rows, the 189 legacy rounds (2,746,946 frames, 115,897 after known deviations), the lab 6542
  (6539 matched, 3 to a known deviation; 5,773,035 frames) with 0 sound rounds differing, the audit 72 traces with
  0 problems.

### P1b, BN5's light and dark, effects, slides and custom request (2026-10-02, branch bn5-port-4)

The engine items the BN5 replays stopped on (docs/design/bn5-map.md §15.3 items 14 to 18), built by the BN5 port
with the coordinator's go-ahead while the rules work did R4. BN6 stays byte-identical: its ruleset has no system
with the new hooks, and its sections keep BN6's numbers by default.

- **Two system hooks (item 14).** `navi_intake(side, navi)` runs each tick of the fight in the navi's intake
  (`sub_801AC6C`), after the standing effects, where BN5's 0x080178EC calls 0x08017136. `chip_check(side, navi,
  chip)` runs at the end of a chip use's preparation (`sub_80127C0`, where BN5's 0x080100E6 checks), and is a
  deciding hook: nil lets the use go ahead, and a chip takes its place. The navi keeps the attack as prepared (its
  lockout is the refused chip's, as BN5's), with only the chip changed. `HookCall::System` now carries the navi
  and the chip a hook is about. The binding passes them after the side. A side whose ruleset has no system with
  the hook calls nothing (`Battle::systems_navi_intake`, `systems_chip_check`).
- **BN5's light and dark MegaMan** (content/bn5/rules/light-dark, in BN5's stock ruleset). The system's setup is
  the save's light/dark value (NaviStats +0x44); bn5-compat writes it from a recording's setup line, through
  `PlayerSetup::set_rule`. At 499 or less, the holy panel under the navi turns Normal each tick. A chip whose
  `megaman` field (its record's +0x15) asks for the other kind of MegaMan becomes the invalid chip (BN5's 0x185,
  now in content: gen_content.py's `RULE_CHIPS`), and its use shows a sparkle. Any navi but MegaMan passes. The
  dark chips' own refusals and costs (BN5's 0x08010030) wait for BN5's dark chip rules.
- **The `effects` section, the arena's (items 15 and 16).** `shake = "battle"`: BN5's camera shake (0x08030D78)
  has one channel. It draws its jitter twice a shaking tick from the battle's RNG2, alike on every console, and
  holds while the battle is paused without dimming (BN6's draws from each console's RNG1, on two channels).
  `spark_steps_at_start = false`: BN5's hit spark (0x080E0870) doesn't step its sprite as it starts, so it lives
  a tick longer.
- **Slides (item 17).** The `reactions` section's `slide_speed`, the arena's: a navi's slide (`sub_8016730`) and
  drag (`sub_80178D4`) go 8 pixels a tick in depth in BN5 (0x0801361E, 0x080143A8), 6 in BN6. Arriving on a panel
  whose type has a `slide` rule (BN5's metal) is as arriving on BN6's roads (BN5's 0x08013564 tests type 5 where
  BN6 tests 9 to 12). A type that `holds` (BN5's sea) ends the slide.
- **What a navi wears restarting.** The `reactions` section's `overlay_restart`, the navi's game's: an animation
  change, a flinch and a drag restart what a navi wears (`sub_8011450`, `sub_80F06CE`). BN6's restart
  (`sub_80C44D2`) reloads the overlay's animation and steps its sprite at once (`"step"`, the default); BN5's
  (0x080C374E) only has it reload at its next step (`"reload"`).
- **The custom request (item 18).** BN5's state 0x20 (0x08007774) opens the custom screen itself once the
  reversions are done. BN6 first goes through state 0x24, which takes a tick. The flow without
  `sequencer_before_custom` now does BN5's. BN5's test of the request skips BN6's battle mode 5 too.
- **Tests:** the hooks with a test system (`test/watcher`), the shake's draws, the spark's tick, the drag's
  speed, and the request's tick count, each against BN6's.
- **The BN5 replays** (1,380 recordings): 252 replay and 243 match every frame, with 139,499 battle frames
  matched (24 and 79,701 before). The rest: DrkRecov (4), whose dark chip cost is unread, and the souls (5),
  which aren't ported.
- **Gates** (on main a157d2ab, the fast gates): the build of every target without warnings, 479 tests, the
  content check (888 modules), gen-content check 0 errors (it decodes BN6's slide speed, `sub_8016730`'s
  literals, the drag's the same), `gate-against.sh full` passed in 537 s (machgun 1074/1331 and soundmod
  21962/14933/20436 with rollback at every latency, the 189 legacy rounds, 2,746,946 frames, 0 sound rounds
  differing; the lab 6548, 6545 matched and 3 to a known deviation, 5,775,231 frames, 0 sound rounds differing),
  the audit 49 traces with 0 problems.

### The NaviCust and the folder rules (2026-10-02, branch match-editor)

The match editor's work (docs/frontend.md §6, README "The match editor") brought two more of BN6's rules into its
content.

- **The NaviCust** (docs/design/navicust.md): a setup's placed programs
  (`PlayerSetup::navicust`), the programs as definitions (`define.navicust_program`, 46 of them, written from
  the ROM's part table by verify's `tools/navicust/gen.py`), the board as a rule section
  (`define.rules("navicust", ...)`, `Rules::navicust`) and the compile as the `navicust` system's `round_setup`,
  which the stock ruleset runs before the patch cards'. Rust keeps only the geometry: shapes, quarter turns,
  boards and whether a shape fits (`NaviCustRules::fits`). Luau reads the setup with `battle.navicust(side)`.
- **The folder rules** (rules/folder/system.luau): a new hook, `folder_check(side)`, which a tool calls through
  `Battle::check_folder(side, chips, regular, tags, complete)` and never a simulation. The hook reads the folder
  with `battle.checked_folder()` and names each rule it breaks with `battle.folder_problem(rule, text)`. Rust
  only calls it and returns the problems (`FolderProblem {rule, text}`); the state it uses (`Battle::folder_check`)
  lives only during the call, and the digest leaves it out. Each side's folder is checked by its own ruleset's
  systems, with its navi's stats as the round set them up (`NaviStat::RegularMemory` is new, read-only). The
  editor, `--match`, a netplay offer and live play's random draw go through it (`nettai_match::folders`). The draw
  makes a folder from the rules' pool (`rule = "chip"`: the chips the hook accepts one at a time) and keeps a
  chip only when the partial folder breaks nothing. Its draws are the same as the Rust rules': the same seed gives
  the same folders (seed 42's drawn match file has the same folders as before the move).
- **The editor's chip pictures** come from each chip's own game's pack (R3b's "not done"): `Pictures::load`
  takes the packs in `pack_paths` order and looks a chip's icon and art up in its root's `assets` pack, by its
  local key.
- **Not done**: the patch cards' 80 MB and 32 cards (`nettai_match::check::CARD_MB`) are still checked in Rust;
  they are the cards' menu's, and would move with a `cards_check` hook.
- **Gates** (match-editor after main d4d846cb): the build without warnings (and with all features), 463 tests, the
  content check (886 modules), gen-content check 0 errors (it now compares the NaviCust boards with the ROM's),
  `tools/navicust/gen.py check` 46 programs with 0 differences; machgun 1074/1331 and soundmod
  21962/14933/20436 with 96 rollback rows and the 189 legacy rounds; the lab 6548/6548 (6545 to the end, 3 to
  ElemTrap's known deviation; 5,775,231 frames) with 0 sound rounds differing (floor 6548, 6548, 175969 with the
  six `navicust-compile/` scenarios); the audit 72 traces (635,424 frames), 0 problems; trace-tests' `navicust`:
  1,274 lab sides and Tango's 4 saves, 0 differ. After main a39285fe (P1a), the merge's tier: the build without
  warnings (all features), 469 tests, the content check (888 modules), gen-content check 0 errors, machgun and
  soundmod as above, `navicust` 0 differ.

### R4, one namespace (2026-10-02)

The user: "i think the idea of packs is just kind of wonky anyway, maybe you should just have it all in a flat
namespace and then in the chip ids directly have bn6:cannon or whatever", and "so loading assets must also be fully
qualified as well". §7.2 is the model; R1's roots (manifests, `requires`, the home, keys qualified by the loader)
are gone.

- **Every folder of content/ loads** (`nettai_content::root::read_all`; content/nettai, the declarations, aside),
  each named as its game; root.toml is gone. `root::content()` is the content directory (`$NETTAI_CONTENT`, else
  the repository's content/): the frontend's, the editor's and the extractors' `--content`, and every loader's
  default (`root::bn6()` and `$BN6_CONTENT` are gone). A folder that names assets of its own game's pack loads only
  with that pack, else it is left out with a warning (`pack::battle_content_packs`); the frontend's and the
  editor's `pack::load_found` reads every folder, leaves out one whose pack isn't found (saying how to write it)
  or without which the rest define, and loads the rest (`Loaded::roots` the folders loaded). A folder of behavior
  only (content/common: no assets of its own) needs no pack and no locales.
- **Every id is written in full** (`nettai_luau::define`): `bn6:minibomb`; a section's name (`bn6:panels`), the
  roles' id (`bn6:roles`, `RolesSpec::id`), the stock rulesets' (`bn6:stock`, `bn5:stock`, `test:stock`). An id
  without its game is refused, every one at once, each naming its folder's game as the fix. Derived keys follow
  their owner (`bn6:minibomb/action`); `engine/...` keys stay.
- **A definition's game is its id's prefix.** The games are the folders and the ids' prefixes, by name
  (`Content::game_names`: `Defs::roots`, a `RootId` a place in it; a state schema's key names what it is the
  state of, `system:bn6:beast/state`, and isn't a game). `BattleGames`: the arena is the stage's id's game, a
  side's its ruleset's, defaulting to the arena's. **No home**: what a battle reads with no side is the arena's
  (`Battle::game_of`, `chip_or_zeroed`, `chip_field`, `zeroed_chip`, `ChipHand::empty(content, game)`,
  `SideRules::for_player`); a tool with no battle takes the first game that has the thing (`Library for
  Content`); a frontend's and the match tool's default is BN6's, by name (`nettai_match::DEFAULT_GAME`, its
  `ruleset_game`), and a pack whose manifest says no game is BN6's (`pack::UNSAID_GAME`). Gone:
  `Defs::stock_ruleset`, `Defs::home_roles`, `Content::home_rules`, `RootId::HOME`, `Scripts::home`,
  `home_module_mut` (now `module_mut(folder, path)`), `RootManifest::assets`.
- **Asset names are in full** (`nettai_luau::Pack::asset_name`): `asset.sprite("bn6:bomb")`, any loaded pack's; a
  name without its game is refused by the loader and, a match's background, by the match check
  (`nettai_match::background`, the link battle backgrounds `bn6:...`). A pack keeps its own names (`bomb`; a
  chip's icon and picture under its id's own part, which bn6-extract and bn5-extract now write from compat's full
  keys).
- **Requires by path**: `require("@bn6/rules/beast/system")` names any folder; `./` and `../` stay within the
  folder.
- **Compat and locale tables are keyed by full id**: chips, actions, kinds, navis, forms, stages, weapons, patch
  cards, NaviCust programs; records' weapons and variants, rules' lock-ons, statuses and identities, games' per-kind
  tables, curation's; every `[chips]`-like locale table. Role tables (`[effects]`, `[sounds]`) are keyed by role
  name, and an assets.toml by the pack's own names. bn6-compat's `def_key` and `root_in` and bn5-compat's
  `qualify`, `qualify_key` and `strip` are gone (their tables hand ids out as they are);
  `Compat::bn6_for(content)` is BN6's compat for content that stands in for BN6 under another name (the test
  content, `test`: `Compat::bn6_as`), the codec's and the frontend's when they run it.
- **Lookups are exact**: `Defs::find` and `keys::names` are gone; tools, tests, setups and match files write ids
  in full. Content that compares a definition's `id` at run time writes it in full too (the Beast busters' and the
  absorb's forms, the Beast buster's palette table, EraseCross's charge).
- **The test content is one game, `test`**: its own modules and the BN6 modules it borrows, whose `bn6:` ids and
  asset names read as `test:` ones (`testing::borrowed`); twin's modules are `twin:`. Tests compare objects' kinds
  by the id's own part (`Battle::local_kind_key`) where their expected tables name them so.
- **The match file**: its ids were already full; a background is now in full (`bn6:honeycomb`), the stock ruleset
  is `bn6:stock` (was `bn6:bn6`), a side with no ruleset plays BN6's, and `--cards` takes ids in full
  (`bn6:canodumb,-bn6:shadow`).
- **The content check** (`nettai-content-check`, no argument) checks content/: every folder against content/nettai
  and every folder's own declarations, each module by its folder and path; a folder alone still checks alone. BN5's
  HolyDrem used two types of another module, which the checker reads as `any`: the casts say so.
- **The rewrite** is verify's tools/r4-flat-ids.py (ids, section names and asset names in the modules; the locale
  and compat tables), tools/r4-fix-ids.py (ids a module builds in code, from the loader's messages) and
  tools/r4-rust-keys.py (Rust tests' lookups by id), re-run on what landed since (the NaviCust programs, the folder
  system, BN5's third batch). Verify's generators write ids in full (tools/bn5/gen_content.py through the rewrite,
  tools/navicust/gen.py), gen-content reads compat by full id and checks compat's keys are BN6's ids. Its test of
  the roles and collision types had expected `collision type thrown`, unqualified since R1: `bn6:thrown`.
- **Gates** (on main a157d2ab and bn5-port-4 74fb97b7 merged, verify cf4524a1 and bn5-4 90f5f1a4): the build
  without warnings, 479 tests, the content check (content/, 1,246 modules), gen-content check 0 errors and its 6
  tests, `gate-against.sh full`: machgun 1074/1331 and soundmod 21962/14933/20436 with 96 rollback rows, the 189
  legacy rounds (2,746,946 frames), the lab 6548 (6545 matched, 3 to a known deviation; 5,775,231 frames) with 0
  sound rounds differing; the audit 49 traces and the static audit, 0 problems. BN5's replays as bn5-port-4's:
  1,380 recordings, 252 replay, 243 match every frame, 139,499 of 948,097 frames. tools/bn5/gen_content.py check
  0 errors, tools/navicust/gen.py check 0 differences.

### P1c, a base form per game and a mix's own sections (2026-10-02)

P1's items 12 and 8 (bn5-map.md §15.3), on R4.

- **A base form per game (item 12).** `Defs::base_forms` holds each game's base form, by `RootId` (a form of
  `kind = "base"`'s game is its id's prefix); two in one game are refused, and a navi that changes form needs its
  game's. `Content::base_form_of(game)` and `base_form_for(navi)` (the navi's game's) replace `base_form()`: a navi's
  fresh stats, the bug code's form byte, the change back to the base form, a stand-in's look, an afterimage, the
  netplay stand-in, bn5-compat's NaviStats. A game without a base form of its own takes the first game's, by name,
  that has one: BN5's MegaMan keeps BN6's until BN5's port defines `bn5:base` (and his souls' forms), which it now
  can.
- **A mix's own sections (item 8).** `define.ruleset { ..., sections = { define.rules("mix:souls/custom-screen",
  ...) } }`: a section's kind is its id's last part, so a folder may hold several rulesets' sections. A section a
  ruleset lists is that ruleset's, not its folder's game's (`sections::build` leaves it out). `RulesetDef::sections`
  and `Content::ruleset_rules` (by `RulesetHandle`: the ruleset's game's tables with its own sections over them,
  `sections::build_rulesets`) make `Content::side_rules(ruleset, game)`, which a side reads about itself:
  `Battle::side_game_rules` and `rules_for` (`BattleGames::rulesets`), the custom screen (`GameLibrary::ruleset`),
  the match's NaviCust board. Only the sections about a side may be a ruleset's own (custom-screen, berserk,
  navicust, status, lockon, cross-special); the battle's (elements, panels, reactions, math, pools, buster,
  banners, flow, effects) and a chip's game's (chip-use, sp-chips) are refused, and a stock ruleset lists none (its
  game's are its folder's). No BN5 or BN6 ruleset has sections of its own: every battle of theirs reads as before.
- **Tests**: `two_games::each_game_has_its_base_form` (twin's base form beside test's; a game without one takes
  test's; two in one game refused); `a_mix_brings_its_own_side_sections` (testdata's rules/souls.luau: its sides'
  HP bug periods are its own, the game's and the other side's don't change; a `math` section and a stock
  ruleset's sections refused).
- **Gates** (on main 18c2de15): the build without warnings, 481 tests, the content check (1,246 modules),
  gen-content check 0 errors, `gate-against.sh full`: machgun 1074/1331 and soundmod 21962/14933/20436 with 96
  rollback rows, the 189 legacy rounds (2,746,946 frames), the lab 6548 (6545 matched, 3 to a known deviation;
  5,775,231 frames) with 0 sound rounds differing; the audit 49 traces and the static audit, 0 problems; BN5's
  replays as before: 252 replay, 243 match every frame, 139,499 of 948,097 frames.

### S3, Beast Out and Beast Over (2026-10-02)

- **The rush** (content/bn6/rules/beast/rush.luau) is an action of BN6's beast system and fills the role
  `actions.wrapper`. The dispatcher (`sub_801B9E6`) runs it instead of an attack whose `wrapped` byte is 1, provided
  the side plays by the system that owns it. That byte is AIAttackVars+0x1D, `beast_lockon` before; it is now the
  framework's, in `AttackVars` and as the actor field `wrapped`. The rush's state is the system's (`rush_*`, the
  original's +0x1E to +0x27). `wrapper_fresh`, set with the attack's links (`sub_801011A`), starts it over.
  - Rust keeps the lock-on search (actions/lockon.rs, `sub_80EAF60`'s modes, BN6 data until S7).
  - API: `navi:run_wrapped()`, `navi:chain_next_chip()`, `navi:panel_trail(x, y)`, `navi:face_toward(o)`,
    `marker:freeze_target_marker(on)`, the actor fields `wrapper_fresh` and `face_target`, and the request `"slide"`.
- **What runs inside the rush** is decided by a new system hook, `chip_used(side, navi, chip, weapon)`, called at the
  end of `sub_800FB54` once the use's action has started. `chip` is the chip the use reads (the zeroed chip for the
  empty hand); `weapon` is the form's weapon when one runs instead (a charged use). BN6's system marks:
  - a chip with the lock-on flag, in a Beast form or from the Cross special;
  - the Beast forms' claw;
  - SlashCross Beast's charged sword.

  The roles `actions.charged_sword`, `actions.beast_claw` and the whole `lockon` group are gone: only the system
  recognizes them now.
- **Berserk** (rules/beast/berserk.luau) is Beast Over's controller (`sub_802D322` to `sub_802D430`).
  - Both Beast Overs carry a new form trait, `controlled`. The side's systems' `controller(side, navi)` hook decides
    a controlled navi's idle, answering "nothing", "chip", "buster" or "moved". The framework carries out the chip
    and the buster as idle does.
  - The player's input doesn't reach a controlled navi (`apply_actor_inputs`), and a full gauge opens its custom
    screen (`custom_open_requested`).
  - Its state is the system's (`berserk_*`). The form's `berserk` effect (`sub_802D310`) sets the actor's
    `controller_fresh`, which starts it over.
  - API: `navi:next_chip()`, `navi:use_chip()`, `navi:start_move_to(x, y, end_lag)`.
  - berserk.rs keeps the Cross special's share (S4).
- **Exhaustion.** A new hook, `form_reverted(side, navi)`, is called at `sub_80158CC` after the mood 0x80. There,
  outside battle mode 1, BN6's system spends a Beast Out (`beast_out_spent`) or exhausts a Beast Over (`exhausted`,
  then mood 0). `beast_over_exhausted` became the framework's `exhausted` (an actor field). What exhaustion does stays
  the framework's: emotion 5, the mood held, no anger, and the drain of 1 HP a tick, never the last.
- **The glow** is form data. `FormData::glow`, the Beast Overs' `glow`, holds color shaders by the battle time
  (`byte_8016A68`, `byte_8016A9C`), and an empty list is refused. A form with a glow takes no sprite palette and no
  invulnerable glow (`sub_8016860`'s Beast Over test). `GREGAR_OVER_GLOW` and `FALZAR_OVER_GLOW` are gone.
- **The shared kinds** (§3.2):
  - `engine/target-marker` (kinds/target_marker.rs), `engine/burst` (kinds/burst.rs) and `engine/afterimage`;
  - with them the actor field and form effect `target_marker`, the sprite role `target_marker`, and the effect and
    sound roles `burst`;
  - compat's kinds.toml and rules.toml follow, while the assets keep BN6's names.

  The marker's targeting (`sub_80E1670`) stays a Rust primitive, since BN5 has the same routine (§6.1, §6.4). What
  BN6 reads of the marker (its panel, `sub_80E164A`) and its freeze are the beast system's calls.
- **Decisions** (taken while the user was away; for review):
  1. One form trait, `controlled`, instead of `navi:set_controller`, `navi:hold_input` and `battle.forces_custom`
     (§3.1). It is data on the hot path (§6.4), and the three always go together in BN6 (forms 0x17 and 0x18). A game
     that needs one alone splits the trait.
  2. No `form_changed` hook yet. Nothing in S3 needs one: the berserk restarts on the form's effect, and exhaustion
     comes at the revert. S5 adds it if the emotions do.
  3. The wrapper is a role and a byte rather than `navi:set_wrapper`: the dispatcher reads the byte each tick, and
     the system writes it once per use.
  4. The drain stays framework, keyed on `exhausted`. In Luau it would be a call every tick for the rest of the
     battle, and it is what exhaustion does; when to exhaust is BN6's choice, and that is the system's.
  5. Two reads still use `FormKind::is_beast` until S7 replaces `FormKind`: the marker's visibility (forms 0xB to
     0x18) and the afterimage's `beast_form` tether. BN5's same code tests its own form range.
  6. `beast_out_spent` stays an actor field until its readers move (the emotion in S5, the Beast Out button in S6).
- **Fixed during the move:** the first replay after the role rename found `actions.wrapper` filled with the Rush
  chip's action. A `local rush` in roles.luau had shadowed the beast rush's; the gates caught it at once.
- **Verify** (branch rules-s3):
  - gen-content decodes the Beast Overs' `controlled` and `glow`, and names the effect `TARGET_MARKER`;
  - trace-tests' stub listing drops the lock-on roles.
- **Gates** (on main 89147e03):
  - the build without warnings, 481 tests, the content check (1,248 modules), gen-content check 0 errors;
  - `gate-against.sh` with everything selected: machgun and soundmod with 96 rollback rows, the 189 legacy rounds
    (2,746,946 frames), and the lab 6548 (6545 matched, 3 to a known deviation; 5,775,231 frames) with 0 sound rounds
    differing;
  - the audit: 49 traces and the static audit, 0 problems;
  - BN5's replays as before: 252 replay, 243 match every frame, 139,499 of 948,097 frames.

### S4, the Cross special and the navi switch (2026-10-02)

- **The Cross special** (DarkInvs' auto-battle in the battle flag 0x40 mode) is BN6's, in the beast system
  (rules/beast/cross-special.luau). The original's two controllers, Beast Over's berserk and the Cross special,
  keep their state in the same 16 bytes (AIData+0xF0): each clears it, and either may find what the other left,
  for example when a Beast Over comes during a takeover. So both share the system's `controller_*` fields. §2's
  separate cross-special/ folder would have split them.
  - **The framework keeps a side takeover**: SideState's `takeover` and `takeover_ticks` (+0x54 and +0x30, the
    Cross special's before). The countdown stays where `sub_802E1D8` runs it, at the end of stage B, which some
    ticks skip. The request bit 0x20000000 is now `"takeover"`.
  - **Two new hooks.** When idle finds the request (`sub_802E4E4`), it calls `takeover_requested(side, navi)`: BN6
    starts the special (0x1E0 ticks via `battle.take_over`, invulnerable, the state cleared). While the takeover
    runs, idle calls `takeover(side, navi)` after the SELECT special's check. The answers are those of
    `controller`, plus "own_chip": a chip of the special's own started, so the used chip is the attack's.
  - BN6's controller picks its chip from its section's rows (`rules/cross-special.luau`, read by `require`).
  - **The end** (`sub_80EFDB2`) is the system's action `bn6:beast/cross-special-end` (compat 0x59, `engine/cross-
    special` before).
  - API: `battle.take_over`, `end_takeover`, `takeover_ticks`, `navi:start_chip_attack(chip, kind)`; `side_special`
    answers `"takeover"` (`"cross"` before).
  - Gone: berserk.rs, actions/cross_special.rs, `EngineAction::CrossSpecial`, `ActorData::berserk`,
    `Defs::cross_special`, the sound role `cross_special`. `Rules::cross_special` stays only for gen-content's
    check of the rows, until S7.
- **The navi switch.** The "Cross change", BN6's name for the flag 0x40 mode's switch to a link navi, is renamed
  (§3.2):
  - `actions/navi_switch.rs` and `TransformRequest::navi_switch`;
  - `Battle::reserves` (`cross_stats`);
  - the requests `NAVI_SWITCH` and `SWITCH_KNOCKOUT` (`"navi_switch"`, `"switch_knockout"`);
  - the states `SWITCHING_NAVI`, `SWITCH_KNOCKOUT` and `SWITCHED`;
  - the action roles `switch_protect` and `switch_knockout`;
  - stage B's `action_requests`.

  A change into a Cross form (its sound, the Cross merge, `cross_release_anim`) keeps its own name.
- **The Cross bonuses** (`sub_800EF34`, `sub_8013236`, `sub_8012AFA`, the fire charge) are already form and navi data
  that the framework reads (`chip_bonus`, `null_bonus`, `charged_chips`, `charged_bonus`, `charge_doubles`,
  `chip_heals`, `fire_charge`), as §6.1 allows. They run per frame (the HUD's bonus) or per tick (the A charge, the
  fire charge), where a hook would put Luau in a plain fight. No charge hook was added. S7 moves the fields to the
  systems' `extends`. Two rules still read the form kind and go with `FormKind` in S7: Beast Over's Null doubling
  (`sub_8012ABC`) and the Beast forms' Null charge time (`sub_8012F62`).
- **Decisions** (for review):
  1. The Cross special shares the beast system, for the shared state above.
  2. The takeover's state is the side's. The original's lives in the actor slot's last 16 bytes, which a later
     actor in that slot inherits. For the side's player navi this is the same memory unless the navi switch moves
     it to another slot, which no recording has.
  3. The bonuses stay data, as above.
- **Verify** (branch rules-s4): the audit's notes name the navi switch.
- **Gates** (on main ce1dbf09):
  - the build without warnings, 481 tests, the content check (1,249 modules), gen-content check 0 errors;
  - `gate-against.sh full`: machgun and soundmod with 96 rollback rows, the 189 legacy rounds (2,746,946 frames),
    and the lab 6548 (6545 matched, 3 to a known deviation) with 0 sound rounds differing, including the 24
    DarkInvs scenarios;
  - the audit 0 problems; BN5's replays as before (252 replay, 243 match, 139,499 of 948,097 frames);
  - us-spelling 0 on both branches.

### S5, emotions (2026-10-02)

- **BN6's emotion system** (content/bn6/rules/emotion/system.luau, in the stock ruleset after the beast system) holds
  BN6's rules for when:
  - **a navi starts the round tired.** At `round_start`, a navi whose Beast Out counter is spent starts tired
    (`sub_8013892`'s part, the original's init).
  - **a counter gives Full Synchro.** `countered(side, victim)`, run by the counterer's side's systems: in base
    form or a plain Beast Out, unless the victim's mood is held (`sub_801A200`'s rule, reading the other side's
    pushed fact, §4.7).
  - **the NaviCust swing bug runs.** `navi_tick(side, navi)` (`sub_8013DA0`), only for a navi whose `ticked` the
    system set at the round's start (its emotion bug). Patch cards set the bug before that, and only BugFix clears
    it, which the hook reads each tick as the original does. Its state (`swing_ticks`, `swung`, the original's
    AIData +0x3A and +0x0B) is the system's.
- **The framework keeps the emotion's state, which BN5's rules share**:
  - the mood;
  - `tired`, a held state (`beast_out_spent` before; BN6's beast and emotion systems and BugFix set it);
  - `exhausted` (S3);
  - anger, with its flag, its timer and the request `"anger"` (flag2 0x200);
  - the stunned ticks;
  - the order `sub_8015B54` reads them in;
  - `set_mood`'s hold, `mood_held` (held tired or exhausted), an actor field other sides' rules read.
- **Decisions** (for review):
  1. **No pushed emotion** (§4.6). The emotion stays computed from the framework's state, which per-frame and
     per-tick paths read: the palette, the aura, the HUD, the chip doubling, idle's worn-out check. BN5's rules
     (its counter's 0x80, the soul's effect) act on the same levers: the mood, the held states, anger, and their
     own `countered` hook. If BN5's emotions need names BN6's five lack, the pushed emotion comes with BN5's port.
     *As built for BN5 (2026-10-03, bn5-map.md §15.10):* still no pushed emotion. The order and the setter are
     the game's (the status section's `emotions`: BN5's 0x08012740 and 0x080127D6), with a sixth emotion,
     `worried` (BN5's mood under 65); the starting mood and the palette are system hooks (`starting_mood`,
     `navi_palette`), which BN5's light and dark system answers by its value.
  2. **Anger stays framework.** Its trigger (120 stunned ticks or a hit of 300), its 600 ticks, mood 0x80 and
     `sub_8015B54`'s order stay. BN5 has the same routines (the map: similar), so by §2.1's rule 2 they are
     series-common, parameterized when BN5's are read.
  3. `mood_held` is derived rather than pushed: the same fact §4.7 needs, with nothing to keep in step.
  4. **No `navi_hit` or `form_changed` hooks.** Nothing that moved needs them.
  5. **The swing bug's state is the side's** (AIData's in the original), as the takeover's is (S4).
- **BN5 sides** no longer get BN6's counter rule or a tired start through the framework. BN5's replays are unchanged.
- **Gates** (on main d835f206):
  - the build without warnings, 481 tests, the content check (1,250 modules), gen-content check 0 errors;
  - `gate-against.sh` with everything selected: machgun and soundmod with 96 rollback rows, the 189 legacy rounds
    (2,746,946 frames), and the lab 6548 (6545 matched, 3 to a known deviation) with 0 sound rounds differing;
  - BN5's replays as before (252 replay, 243 match, 139,499 of 948,097 frames).

### S6a, the custom screen's extras: the mechanism, the hand size, two buttons (2026-10-02)

- **The mechanism** (§4.4). The screen stays a Rust state machine and asks `custom::Extras`. The battle answers
  with the side's systems (`SideExtras`); a screen without a battle (the screen's own tests) gets no answers
  (`NoExtras`).
  - While a system's function runs, the screen and the folder sit back in the battle's side, so the `custom`
    library reaches them, and they are taken back afterwards.
  - `Side::open_with` and `tick_with` take the extras; `open` and `tick` keep their signatures.
  - bn6-compat's screen check (`check_custom_screens`, now on an `Arc<Content>`) builds a battle from the round's
    setup and asks `Battle::custom_extras(side)`, setting the side's stats and the turn from the trace first.
  - The hand built at OK reads a formula chip's damage from the battle, so those (`Defs::formula_chips`) are read
    before the extras borrow it.
- **`custom.hand_size(side)`**, a system's `custom` hook, asked as the screen opens (§3.3). BN6's comes from its
  new `cross` system (content/bn6/rules/cross, first in the stock ruleset): ChargeCross's extra chips (its
  `charge_cross_screens`, out of `RoundMemory`), the custom level, NumbrOpn (not in DustCross) and the
  hand-shrink bug (`sub_802A49C`, `sub_802A40C`). With no answer the framework's rule applies: the same without
  a form's share, which is what BN5 sides get, as before.
- **Buttons.**
  - A system declares its buttons: `buttons = { name = { slot, cells, uses, right, left, shown, state, pressed }
    }`, read into `Defs::buttons`.
  - `SlotKind::Button { button, cell }` (a cell: only, left, right) replaces `Scrap` and `Redeal`.
  - As the screen opens, each button whose `shown` answers takes its slots, in the order the systems are listed;
    the first to claim a slot keeps it. `state`, where a button has one, is asked at the open and after each pick
    unless the button is picked or used up. `pressed` answers A.
  - The `custom` library: `refuse`, `sacrifice` and `redeal` (the shared machinery for the button under the
    cursor, which keeps its button's slot in its phase), `last_pick_is_chip`, `cursor_state`.
  - BN6's two:
    - DustCross's scrap, the cross system's: two wide on 8 and 9, usable once, selectable with a chip picked last.
    - ChpShufl's re-deal, the navicust system's.
- **The frontend.** nettai-render draws a button by its name (`View::button_look`: BN6's `redeal` and `scrap`
  pictures, tiles and cursor, §4.8). The driver labels a button by its name.
- **Next.** Beast Out (its button, the BeastOut chip and their animations as windows, the result's form) goes with
  the Cross window: the two read each other (a chosen Cross grays out Beast Out, and Beast Out blocks the
  window). Then the setup (S6c).
- **Merged with main f816b94d** (bn5-port-5): BN5's soul button (`SlotKind::Soul`, `Phase::SoulChosen`, in Rust)
  sits beside the system buttons. It and its sequence take the extras too. A port of it to a BN5 system's button
  and window is BN5's, when its rules come.
- **Gates** (on main f816b94d):
  - the build without warnings, 482 tests, the content check (1,275 modules), gen-content check 0 errors;
  - `gate-against.sh full`: the 189 legacy rounds, machgun and soundmod with 96 rollback rows, and the lab 6548
    (6545 matched, 3 to a known deviation) with 0 sound rounds differing;
  - identity.sh against main's frontend on main's content: the custom-screen and sample lists identical in both
    text modes (174 scenarios, 200,712 frames each);
  - the audit 0 problems; us-spelling 0;
  - BN5's replays as main's, on a BN5 pack extracted again for bn5-port-5's asset names: 402 match every frame,
    18 replay, 223,414 of 948,097 frames.

### The field's art in a mixed battle (§7.4; 2026-10-03, branch audit-panels)

The user approved §7.4's proposal on 2026-10-02: "yes, borrow bn5 art then fall back".

- **A pack says which panel types its field draws.**
  - `nettai_assets::Field::panel_types`: each type's engine number, in the order of its blocks.
  - field.json's `panel_types` lists them by the engine's names (docs/design/asset-formats.md §4).
  - bn6-extract writes BN6's 13 in the engine's order. bn5-extract writes BN5's 11 in BN5's order (metal 5,
    lava 8, holy 9, sea 10: content/bn5/compat/panels.toml).
  - A field may have one highlight or two. BN5 draws its one highlight block for both, so its pack keeps that
    block twice.
  - A field.json without the list comes from an older pack. With 78 blocks it draws BN6's 13 types in the
    engine's order, so the shared BN6 pack loads as it is. With any other number of blocks it draws no type, and
    the loader warns to extract it again (an older BN5 pack).
- **The stage draws the field from the arena's game's pack.** It was the content's own pack's before.
- **`nettai_render::stage::FieldArt`** says which pack's field draws each panel type and each highlight:
  1. the arena's field, if it draws the type;
  2. else the field of the first loaded game, in root order, whose `panels` section names the type and whose
     field draws it (the order in which the simulation takes the type's rule, `sections::fill_panel_types`);
  3. else nothing: the panel is its owner's normal panel tinted halfway to magenta, never a hole.

  A highlight the arena's field lacks comes from the first loaded game whose field has it, else it is tinted. A
  borrowed field draws with its own tiles, its own palettes and their cycles, in a palette set of its own, so the
  arena's palettes are left as they are. Front edges and missing panels are always the arena's.
- **Lookups and audits.**
  - A panel lookup names its pack (`Lookup::Panel(pack, ..)`).
  - A tinted panel is `Lookup::PanelTint`. It is said, not counted: `Problems::say` and `said_lines`, which
    both audits print as notes.
  - The static audit checks that each loaded game's pack draws the panel types its game names, with their
    blocks. Then, in an arena of each loaded game, it draws every panel type any loaded game names, and both
    highlights, through `FieldArt`, and says which are tinted.
- **Tests.** `a_mixed_field_borrows_the_art_it_lacks_or_tints_a_normal_panel` uses a test arena and a second game,
  `twin`, and checks that:
  - the sea is drawn from twin's field, in twin's colors;
  - lava, which no field draws, is tinted;
  - highlight 2 is twin's;
  - a game that names the sea but whose field doesn't draw it passes the sea on to the next game;
  - without twin's pack, the sea and highlight 2 are tinted.

  `the_field_is_audited_for_the_panel_types_its_game_names` covers the static audit, and
  `an_older_field_without_panel_types_still_loads` covers a field.json from before `panel_types`.
- **Seen.** A BN6 arena (bn6:netbattle-35) drawn headless with BN5's sea, lava and metal set on six panels and
  both highlights shown:
  - on a BN5 pack extracted again: BN5's own sea, lava and metal, with BN6's highlights;
  - on a BN5 pack from before `panel_types`: the six panels as tinted normal panels, said by the lookups.

  This needs BN5's folder loaded beside BN6's. A scratch program loaded it the way verify's trace-tests
  (`bn5_content`) do, leaving out BN5's chips without a use; the frontend leaves BN5's folder out until those chips
  are written.
- **Packs.** A BN5 pack needs extracting again for its field to draw. Before that, a BN5 arena draws BN6's
  panels for the types both games name (as it drew BN6's whole field before this) and tints its metal, lava and
  sea. A BN6 pack loads as it is; extracting it again adds only the list.
- **Gates** (merged with main e1c69bdf):
  - the build without warnings (all targets);
  - the tests of nettai-render, nettai-frontend, nettai-content, nettai-assets, nettai-editor and both
    extractors;
  - the audit gate (`audit-against.sh`): the static audit 0 problems (6,965 lookups), 49 traces 0 problems;
  - identity.sh against main's frontend on the shared BN6 pack: the sample and custom-screen lists identical in
    both text modes (174 scenarios, 200,712 frames each);
  - us-spelling 0; the R4 scripts change nothing.

  The pack loading touches only the graphics, not the simulation's content hash, so neither the lab nor the
  traces ran.

### S6b1, Beast Out on the custom screen (2026-10-03)

- **Windows** (§4.4).
  - A system declares its windows: `windows = { name = { update } }`, read into `Defs::windows`.
  - `Phase::Window { window, tick }` runs a window's `update(side)` each tick, until it answers that it is done.
    The tick counts from 1 (`custom.window_tick`).
  - New custom hooks: `custom.open` (as the screen opens, before the hand size and the layout), `custom.confirmed`
    (OK built the hand), `custom.chip_picked` and `custom.chip_taken_back` (a hand chip, `(side, chip)`), and a
    button's `taken_back`.
- **The screen's result form.** `Screen::form` and the system that set it hold the form a pick puts the navi in at
  the turn's start. OK turns it into the transform's form.
  - `custom.set_form` sets it. `custom.form_taken` asks whether another system holds it: Beast Out grays out under
    a Cross's form, and the Cross window refuses under Beast Out's.
  - Until the Cross window is a system's, a Cross the Rust window chose counts as another system's form.
- **The `custom` library** for buttons and windows:
  - `pick`, `play` (the screen's sounds by name), `set_column_icon`, `open_window`, `window_tick`, `shake` (this
    console's camera), `frame` and `set_frame`, `spin`, `fade` (by name), `set_face`, `pick_first`,
    `set_button_state`, `update_availability`, `draw_emblem`;
  - `set_form`, `form_taken`, `full`, `button_picked`;
  - `player`: what the screen reads of its player. Its emotion is the screen's own (the context's: `Side::emotion`),
    so bn6-compat's check of the traces' screens, whose battle has no navi, still reads it. The version, the Cross
    list, Beast Out unlocked and sealed, and a random battle are read from the setup until S6c.
- **BN6's Beast Out** (content/bn6/rules/beast/custom.luau), the beast system's:
  - the button in the special slot (`sub_8029FB4`, `sub_802A57E`, `sub_8028F48`, `sub_8028D6C`, `sub_802A0EC`);
  - its 70-tick animation as the window `beast_out` (`sub_802770C`);
  - the BeastOut chip's 85-tick animation as `beast_out_chip` (`sub_80275EC`, from `custom.chip_picked`);
  - the Beast form and the roar's game (`Unlocks::beast_form` and `beast_game`, ported);
  - the round's Beast Out (`RoundMemory::beast_out_used`) as its state, forgotten at the round's first screen, noted
    at OK.
- **The BeastOut chip's selection rule** is data: the chip trait `goes_with_any` (`sub_8028E4C`, `sub_8028EC8`), so
  `ChipTraits` is now 16 bits. gen-content expects it on chip 0x13F.
- **Gone from Rust:** `SlotKind::BeastOut`, `Phase::BeastOutChosen` and `BeastOutChipChosen`, `Screen::beast_out`,
  `update_beast_out`, `PlayerView::beast_out_button`, `beast_out_available` and `beast_game`, `beast_face`,
  `is_beast_out`.
- **Tests.** The screen's own tests lose Beast Out's: the button, its timeline, and the roar's game with a Cross list
  are BN6's Luau now. The lab's Beast Out scenarios and identity.sh cover the first two. **The Cross-list roar has
  no recording** (the Cross list is nettai's extension): unverified until a verify-side test drives it.
- **Gates** (on main e1c69bdf):
  - the build without warnings, 480 tests, the content check (1,276 modules), gen-content check 0 errors;
  - `gate-against.sh full`: the 189 legacy rounds, machgun and soundmod with 96 rollback rows, and the lab 6548
    (6545 matched, 3 to a known deviation) with 0 sound rounds differing;
  - identity.sh against main's frontend on main's content: the custom-screen and sample lists identical in both
    text modes (174 scenarios, 200,712 frames each);
  - the audit 0 problems; us-spelling 0 (after it fixed a British spelling of mine in S6a's note);
  - BN5's replays as main's: 402 match, 18 replay, 223,414 of 948,097 frames.
### S6b2, the Cross window (2026-10-03)

- **The Cross window is the cross system's** (content/bn6/rules/cross/window.luau):
  - as the screen opens (`custom.open`): the Crosses offered (`sub_8029EF8`, `sub_8029F70`) and the window's Cross
    tab (`sub_8029EC8`, `sub_8026840`), with nettai's Cross list (window.luau's `cross_at`, `owns_cross` and the
    list's fit, ported from `Unlocks` and `PlayerView`);
  - UP opening it (`custom.keys`, `sub_8028B74`);
  - four windows: `cross_opening` (12 ticks, `sub_8027834`), `cross_window` (`sub_802794A`, its keys
    `sub_8028A78`), `cross_closing` (6, `sub_802790C`) and `cross_chosen` (34, `sub_8027A58`, `sub_8027AAE`,
    `sub_8027ADE`). The opening's last tick runs the window's first (`custom.open_window(side, "cross_window", 1)`:
    the ticks it has had), so the next tick reads keys, as the original's does;
  - B with nothing picked taking the Cross back (`custom.take_back`, `sub_8029032`);
  - the result form at the choice (`custom.set_form`: the Cross, or its form in Beast Out in a Beast form,
    `sub_802937A`), and the face when the Cross is put on (`sub_802A088`);
  - the round's Crosses used (`custom.confirmed`), forgotten on its first screen.
  - Its state: `crosses_used`, `offered`, `offered_count`, `marked`, `window_cursor`, `cross_chosen`, `chosen`.
- **The framework:**
  - Two hooks: `custom.keys` (choosing, on a tick with keys, before the screen's own; the first true takes them) and
    `custom.take_back` (B with nothing picked; none answering true, B is refused). `SystemHook::ALL` has 28.
  - `Phase::Description { window, form, chatbox }`: a window's description (`custom.describe`) returns to the
    window at its first tick; the chatbox prints the form's description.
  - The joypad reaches the hooks: `Extras::keys` and `window_update` take it, and `with_screen_console` puts it in
    the battle's side (bn6-compat's checker ticks a side of its own).
  - The library gains:
    - `cursor`, `set_cursor`, `pressed`, `repeated` (keys by name: `input::key_named`);
    - `draw_window`, `draw_regular`, `draw_cross_cursor`, `show_chip_window`, `set_cross_tab`, `describe`,
      `refresh_buttons`;
    - `play`'s `cursor`, `description`, `cross_window_open`, `cross_window_close`, `cross_chosen`;
    - `open_window`'s `ticks`;
    - `player`'s `crosses`, and `cross_list` as the list's forms (it was a bool).
  - `Battle::system_state(side, key)`: a system's state and its layout, for a reader.
  - `Library::cross_description_lines` is `form_description_lines`.
- **The renderer** reads the window from the cross system's state by its fields' names (nettai-render's
  `CrossWindow::of`), and its stage from the window up (`cross_stage`). `CROSS_PUT_ON_TICK` (25, window.luau's
  `PUT_ON_TICK`) is the renderer's own now: the map's look. The chatbox prints `Description`'s form's
  description. The driver titles the Cross windows by name.
- **Gone from Rust:** `CrossWindow`, `Screen::crosses`, `Phase::{CrossWindowOpening, CrossWindow,
  CrossWindowClosing, CrossChosen}`, `RoundMemory::crosses_used`, the confirm's Cross, `cross_face`,
  `PlayerView::{crosses_allowed, crosses_left, offered_crosses, listed_cross_fits}`, and `custom.form_taken`'s
  bridge (S6b1): a Cross holds the result form itself now, as Beast Out does. `Unlocks::cross_at` stays for the
  renderer's names until S6c.
- **Tests.** The screen's five Cross tests used the Rust window. They are now battle tests on BN6's content
  (nettai-frontend's driver tests):
  - a Cross list's offer of either game, once a round;
  - R describing the Cross under the cursor;
  - a Beast form's offer;
  - Crosses only, and not the starting form;
  - B taking the Cross back (new).

  The chatbox's test checks the forms' descriptions in both languages; the list's mapping is Luau's.
- **Decisions** (the user away):
  - Four windows, not one with sub-states: the framework's tick counts each, and the renderer tells them apart by
    name.
  - The window's drawing calls keep the look's BN6 names (`draw_cross_cursor`, `set_cross_tab`): the screen's look
    is BN6's, which BN5 shares.
  - `CROSS_PUT_ON_TICK` is repeated in the renderer as the look's, rather than kept as a look field in the state.
- **Gates** (on main 9b87a94e):
  - the build without warnings, 482 tests, the content check (1,277 modules), gen-content check 0 errors;
  - the full gate (the selected gate ran everything): the 189 legacy rounds, 96 rollback rows matching, and the lab
    6548 (6545 matched, 3 to a known deviation) with 0 sound rounds differing;
  - identity.sh against main's frontend on main's content: the custom-screen and sample lists identical in both
    text modes (174 scenarios, 200,712 frames each). The custom-screen list has the Cross window's keys, its close,
    the take-back, one Cross owned and none offered;
  - the audit 0 problems; us-spelling 0;
  - BN5's replays as main's: 402 match, 18 replay, 223,414 of 948,097 frames.

### S6c, the player's BN6 setup (2026-10-03)

- **BN6's save facts are its systems' setup.** The cross system's setup is `version` (`falzar`, `gregar`), `crosses`
  (owned, bool[5]) and `cross_list` (form[5]). The beast system's setup is `version`, `beast_out` and `cross_list`
  (with a list, a Cross of the other game's Beast is that game's). window.luau and the beast's custom.luau read
  `system.setup()`; `custom.player` keeps only the emotion and a random battle.
- **Facts by name.** `PlayerSetup::set_fact(content, game, field, values)` writes a field into every system of the
  player's ruleset that declares it (an enum by name, an array by element), so a tool states `version` once and both
  systems get it. Making the blocks, it names the ruleset in the setup (a setup without one would play by its
  arena's game's stock rules). `Battle::system_setup(side, key)` reads a block back.
- **`Unlocks` and `CrossList` moved to bn6-compat** (`unlocks.rs`): `Unlocks::write` writes the facts,
  `Unlocks::of`/`of_side` reads them back for the renderer's pictures, names and Beast count, and `cross_at`,
  `beast_form`, `beast_game` stay as the BN6 look's helpers. `PlayerSetup` and the custom screen's `Side` keep
  BN5's `souls` alone (until BN5's soul button is a system's). netplay's codecs know no game: the offer carries a
  Cross list as its forms.
- **The version can't be NaviStats'**: its +0x20 byte differs from the console's version in 2,883 of the 17,942
  recorded sides, so the version stays setup.
- **Event flag 0x163 is the navi code's level.** `PlayerSetup::navi_level` is an option (none: no code, 0xFF in the
  battle; `MAX_NAVI_LEVEL` 14, asserted at `Battle::new`), and BN6's rules read the seal on Beast Out and the Cross
  window from `battle.navi_level(side)`. The 4,688 recorded sides with both agree (the init exchange sends a level
  only with the flag set, `sub_800B144`). The 176 sides with a level and no recorded flags are link navis, which the
  seal changes nothing for. bn6-compat: a trace without levels reads as a link navi's 0 and MegaMan's none.
- **Decisions** (the user's, through the coordinator):
  - a link navi always has a level, 0 when a file says none, since it exists only through its code;
  - none is MegaMan without a code;
  - the checks refuse a link navi without a level and a level past 14;
  - MegaMan with a level gets the level's gains after his NaviCust (the navicust system's `round_setup`,
    `reloadCurNaviStatBoosts`).
- **SP deletion times** are a player's (`PlayerSetup::sp_times`; `RoundSetup::sp_times` gone). The SP formula still
  reads them in Rust (BN6's chip data is S7's).
  - Match files: `[left.sp_times]` by the rules' slot names, `mm:ss.cc`, a slot left out the fastest; a written time
    is the fewest frames that show as it (nettai-match's `sp_times`).
  - The editor has a field per SP navi (by its chip's name), and the netplay offer carries them.
- **The save import** (narrow, the coordinator's): bn6-compat's `save` reads a BN6 .sav (the image at 0x100, the
  mask, the shift word, the game's name, the checksum, as Tango's save support has them).
  - It reads the version, the event flags (Beast Out, the version's Crosses, 0x163), the navi operated, the navi
    code's level (`0x141 + 15·navi + level`) and the SP times (image 0x18C0, which `sub_800B144` sends at +0x70).
  - `Side::import_save` gives a side the game, Beast Out, the Crosses owned (as a Cross list unless all five) and
    the level (a link navi keeps its own without a code), and the SP times. The editor's "Import from save…" runs it.
- **Match files**: no key renamed, so old files load; the new keys are `beast_out` and `[left.sp_times]`.
  docs/frontend.md says the level's default.
- **`protocol::VERSION` 4**: the offer's level is an option and it gains Beast Out and the SP times, and the battle's
  digest differs. (bn5-port-6 landed first with 3, its tactics in the offer; the offer carries both.)
- **Tests**:
  - bn6-compat: the save reader (a written save reads back; damaged, foreign and other-navi codes refused);
  - nettai-match: SP times, Beast Out and levels write and read back, the level checks, the import, MegaMan's
    level gains at levels 0, 7 and 14;
  - the frontend: a navi code seals Beast Out and keeps the Cross window.
  - The offsets were also read on Tango's four raw BN6 templates by a script (version names, flags, navi, codes, SP
    times as expected). Their checksum word is zero (memory images Tango checksums when it writes them), so the
    checksum path is the tests'.
- **Gates**:
  - on the final merge (main 6d4d0ec5, then main 87f4cf65's bn5-layout and navi chips, which S6c doesn't touch): the
    build without warnings (every feature), 511 tests, the content check (1,422 modules), gen-content check 0 errors,
    the audit 0 problems, us-spelling 0 on both repositories, BN5's replays 1,145 matched (973,226 frames, 760,918
    matching);
  - the full gate, unmodified, on main 6d4d0ec5 merged: the 189 legacy rounds, 96 rollback rows matching, and the
    lab 6548 (6545 matched, 3 to a known deviation) with 0 sound rounds differing;
  - identity.sh against main 24565c25's frontend on its content: the custom-screen and sample lists identical in
    both text modes (174 scenarios, 200,712 frames each); main's later merges were BN5's;
  - BN5's replays matched main's exactly, recording for recording count and frames, at main 851e3392, de4672cc and
    24565c25 (1,107 matched there).
