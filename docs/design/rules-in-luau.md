# Rules in Luau: the ruleset as content

A design, not a built feature (phase 1 of the work; nothing in it is implemented yet). It says how BN6's rules
move out of the engine's Rust into a Luau **ruleset**, so that BN5, BN4 and others are mostly content, and how
two games' content runs together.

The user's decisions:

- the engine is nettai, BN6 is its first game, BN5 (then BN4 and others) follow
  ([multi-game.md](multi-game.md), "Decided");
- "push more rules into luau", rather than a Rust ruleset per game (multi-game.md §3.3, option (b), over §3.5's
  hybrid);
- "start abstracting more rules so bn5 support can be implemented", and BN5 "should be composable with bn6
  content".

The survey (multi-game.md) estimated Luau-side rules at five to ten times today's rollback cost, and found that
a ruleset in Luau needs battle-wide state that content declares, which the content model does without. Both are
problems this design solves, not reasons to stop.

Related: [multi-game.md](multi-game.md) (the inventory of what is BN6's, and BN4's and BN5's differences),
[content-model-v2.md](content-model-v2.md) (content by name, definitions, roles, traits, compat apart),
[scripting.md](scripting.md) (the runtime and its costs), [rollback.md](rollback.md) (snapshots, the digest, the
cost measure), [core-content-boundary.md](core-content-boundary.md) (the layers as built).

Words: the **core** is the engine's mechanism (pools, collision, the panel grid, RNG, snapshots); the
**framework** is the Rust rules every Battle Network game of the BN4–BN6 lineage shares (the navi framework, the
hit kernel, the custom screen's chip window, the flow); a **ruleset** is one game's rules, a Luau definition in
that game's content root; **content** is everything else a root defines (chips, kinds, navis, forms, stages). A
**hook** is a function slot of the ruleset the framework calls; a **root** is a content directory (content/bn6).
Routine names are the original's. "Dimming", "cut-in chip", "counter cut-in", "telop" and "supports" are used as in
the rest of the project.

## 0. Summary

- **One ruleset per battle, defined in Luau** (`define.ruleset`, in content/bn6/rules/). It owns BN6's rules
  that are not the framework's: the turn-start transformations, Cross and Beast Out, Beast Over, the Cross window
  and BN6's custom-screen buttons, the emotions, the forms' behaviour, BN6's per-player setup (unlocks, game
  version, bug frags, navi levels), and its rule sections and roles. The round's setup names the ruleset.
- **Rust keeps what runs for every object every tick, and the services.** Object pools and the update list,
  collision and hit resolution, the panel grid, RNG, the digest, snapshots and cues stay the core. The framework
  stays Rust where the games share it (input, charge, movement, idle's common priorities, intake, statuses, chip
  use's common path, the dimming service, obstacles, the chip window, Program Advances, the flow's skeleton), and
  takes the games' differences as data (rule sections) or hooks, never as a Rust fork per game.
- **Ruleset state is engine-owned typed storage**, as content state is: the ruleset declares a battle block, a
  block per side and a per-player setup block (named, typed fields); `Battle` stores them as plain bytes, so a
  snapshot is still a clone and the digest still covers everything. The VM still holds nothing between calls.
  This is the one amendment to the content model: a ruleset declares battle-wide state; other content still
  declares none, and reaches a ruleset's state only through that ruleset's API module.
- **Hooks are coarse, at events**: a turn's start, a custom-screen request, the custom screen's open, buttons,
  windows and result, a navi hit, a deletion, a chip's use, a form put on. A per-tick Luau call happens only
  while a state the ruleset set is active (a Beast rush, Beast Over's berserk controller, a Cross special, a
  ruleset window on the custom screen, a form with a `tick`), never in the state a battle spends most of its time
  in. The ruleset pushes results into the framework's typed fields (the emotion, a controller, an input hold)
  rather than being asked every tick.
- **Cost, measured today** (§6): the worst case of a 10-frame rollback every rendered frame is 82 to 121 µs on the
  five golden rounds and 49 to 135 µs on a basket of Beast, Cross and custom-screen lab scenarios (best of five on
  a shared M1 Max), of 16,667 µs. About 45 µs of it is fixed (the digest, a restore, eleven saves); the rest is
  eleven advances of 0.5 to 8 µs, and **Luau content is already about 80 % of an advance**. **Budget:** no slice
  adds a per-tick Luau call to the common state; every basket scenario stays under **500 µs per rendered frame**
  (3 % of a frame); each slice reports its before and after. This plan's estimate at its end is about 170 to
  240 µs in the worst scenarios. The reserve, if needed, is cheaper calls into Luau (scripting.md §7, §9).
- **Composable with BN6 content** (§7): content roots load together (content/nettai for the generic
  declarations, content/bn6, content/bn5), each with its own keys, compat and assets; a battle runs one ruleset;
  any root's chips, navis and stages may be in it. **The battle's ruleset rules the battle; a definition's
  behaviour is its own.** A ruleset reads its own extra data on definitions (BN6's lock-on mode on a chip) and
  says what it does with content that has none; a ruleset's API for content (`bn6.*`) says what it does in
  another game's battle. A BN5 chip in a BN6 battle is used through BN6's custom screen and damage rules and
  never Beast-rushes unless it carries BN6's lock-on data; a BN6 chip in a BN5 battle runs as it does in BN6,
  with its BN6-only data unread.
- **Slices** (§8): S0 the ruleset definition, its state and the cost tools; then BN6-only systems one at a time
  (the turn-start transformations, the form-change actions, Cross, Beast Out and Beast Over, emotions, the custom
  screen's BN6 parts, forms' behaviour and BN6's data off the Rust records, BN6's API surface); roots for
  composability in parallel; then the series-common rules, parameterised as BN5's routine map shows them to
  differ. Each slice lands on main with both golden traces and the full lab exact, the rollback cost measured,
  before the next starts.

## 1. Where the rules are today

### 1.1 The tiers

nettai-battle is about 31,500 lines without tests (multi-game.md §1): about 5,500 generic, about 20,000 rules
every game of the series has but written with BN6's numbers, about 6,000 BN6's alone. The content model already
moved everything a chip, kind, weapon, navi, form or stage owns into Luau, with the ruleset reaching content by
definition, role and trait (core-content-boundary.md). What remains in Rust is the core, the framework, and the
BN6-only systems this document moves.

### 1.2 The BN6-only systems, and where the framework calls them

| System | Where (lines) | How the framework reaches it | How often |
|---|---|---|---|
| The turn-start transformations: the sequencer, the requests, the reversion before a custom screen, Beast Out's count-down | transform.rs (230); battle.rs `fight_setup`, `fight_custom_revert`, `fight_custom_sequence`, `count_down_beast_out` | fighting states SETUP, CUSTOM_REVERT, CUSTOM_SEQUENCE | per tick while the battle is paused at a turn's start or before a custom screen |
| The form changes (a Cross, Beast Out, a Cross in Beast Out, a Beast's Cross, Beast Over) and the reverts | kinds/player/actions/transform.rs (710), kinds/cross_merge.rs (190), kinds/beast_over_burst.rs (90) | the pause handler's actions (status.rs), started by the sequencer's requests | per tick of a form change, paused |
| Cross: the Cross change mode (battle flag 0x40, never in a netbattle), the Cross knockout and protect, a weakness hit breaking the Cross, the Cross special, the SELECT special | actions/cross_change.rs (320), actions/cross_special.rs (80); status.rs, idle.rs; battle.rs `cross_stats`, `sides` | status's top block (request and state bits), idle's specials, the pause handler | bit tests every tick; the work at events and while a special runs |
| Beast Out and Beast Over: the Beast rush around a chip's action, its lock-on marker and afterimages, Beast Over's berserk controller, its glow, exhaustion | actions/beast_rush.rs (330), berserk.rs (350), kinds/lockon_marker.rs (210), kinds/afterimage.rs (310); chip_use.rs, idle.rs, player/mod.rs | chip use sets `beast_lockon`; the action dispatcher routes the rush; idle calls berserk; the palette reads the form | per tick during a rush, per tick in Beast Over, the marker every tick in a Beast form |
| Emotions and mood: anger, tired, worn out, Full Synchro's mood, the NaviCust emotion-swing bug | player/mod.rs (`emotion`, `set_mood`, `emotion_timer`, `navi_palette`), status.rs, intake.rs | `emotion()` read by the palette, the custom screen, chip use; the swing bug's timer every tick | reads every tick; changes at events |
| The custom screen's BN6 parts: the Cross window, the Beast Out button and the BeastOut chip, DustCross's scrap, ChpShufl's re-deal, the hand size's BN6 rules, `Unlocks`, `GameVersion` | custom/screen.rs (about 560 of 1,610), custom/mod.rs, custom/library.rs | the screen's phases and slot kinds | per tick of the custom screen |
| Forms' behaviour: the BN6 form kinds, their game, the Cross navi, Beast form, break, the Cross special's volley, GroundCross's rocks, EraseCross's erasing, SpoutCross's healing, ChargeCross's fire charge, hover | content/navis.rs `FormData`, form.rs, chip_use.rs, player/mod.rs | `FormKind` and form traits tested in a dozen places | at chip use, charge and form change; hover and fire charge every tick |
| BN6's records and API: `ChipData::{beast_lockon, lockon_mode, traits.no_chain, dark_substitute, hp_bug, formula}`, `Registry::Lockon`, `NaviStats::{version, beast_out_counter, support, sun, ...}`, `SpTimes`, `bug_frags`, `navi_levels`, `ActorField::{BeastLockon, BeastOutSpent}`, `NaviState::Cross*`, `SideSpecial`, `Emotion` | content/chips.rs, content/navis.rs, setup.rs, battle.rs, nettai-content-api api.rs, core.d.luau (about 120 lines) | everywhere the systems above read them | |

What the table shows for cost (§6): most of these systems run at events, or per tick only while a state lasts
that few frames of a battle are in. The exceptions are reads every tick (the emotion for the palette, the Cross
and Beast request bits, the form's hover and fire charge, the lock-on marker in a Beast form), which the design
turns into pushed fields, data the framework reads, or a kind that runs only while its state does.

### 1.3 What the content model already gives, and what is missing

Already there: definitions with function slots the engine calls by handle (a kind's `update`, an action's
`update`, a weapon's `setup`, a chip's `dimming`/`navi`/`instant`, a kind's `place`); typed content state the
engine stores (64 bytes an object or action); roles (what the framework starts, spawns, shows and plays by
name); traits; rule sections (`define.rules`); `Battle: Clone` and the digest over all of it.

Missing for a ruleset:

1. a definition that is the ruleset, which a battle runs (§2.2);
2. state that is the battle's and each side's, not an object's (§5);
3. hooks into the flow, the custom screen and the navi framework (§4);
4. roles and rule sections per ruleset rather than one per pack (§2.2, needed once two roots load);
5. roots that load together, with their keys, compat and assets apart (§7).

## 2. The model

### 2.1 The layers, and where the line runs

| Layer | Language | What |
|---|---|---|
| Core | Rust | Object pools and the update list, collision registration and hit resolution, the panel grid, 16.16 geometry, sprites and animation stepping, RNG, input and the link, cues, snapshots, the digest, the content host |
| Framework | Rust | What the lineage shares: the navi framework (input, charge, intake, statuses, reactions, idle's common priorities, chip use's common path, movement), the hit kernel and damage pipeline, the dimming service, navi chips, obstacles, the chip window (deal, cursor, selection, Program Advances, modifiers, descriptions, sending), the flow's skeleton (intro, banners, gauge, fighting, judge, sets). It reads the battle's ruleset's rule sections and calls its hooks |
| Ruleset | Luau | One game's rules: a `define.ruleset` with its state, setup, hooks, custom-screen buttons and windows, controllers, its own actions and kinds, rule sections, roles |
| Content | Luau | Chips, kinds, weapons, navis, forms, stages, records, assets by name, as today |

The line, as rules to apply:

1. **Per-tick, per-object loops and the hit path stay Rust.** The object loop, collision pairing, the hit
   kernel, movement, input, charge, sprite stepping and panels run for every object every tick; they are the
   cost, and they are shared.
2. **A game's difference from the framework is data or a hook, never a Rust fork.** Where BN5's routine differs
   from BN6's in values, the values become a rule section; in a branch, the branch becomes a hook with BN6's
   behaviour in BN6's ruleset; where a whole subsystem differs throughout, it moves into the rulesets entirely.
   No `if game == bn5` in Rust.
3. **BN6-only systems move into BN6's ruleset**, ported from the Rust (itself ported from the disassembly) branch
   for branch and verified by the same traces and lab.
4. **Hooks at events; per-tick Luau only while a state the ruleset set is active.** A hook slot the ruleset
   doesn't fill costs a `None` check. A per-tick call is registered by the state that needs it (a controller set
   on a navi, a wrapper around an action, a window open on the custom screen, a form's `tick`) and gone with it.
5. **Push, don't pull.** What the framework reads on a hot path, the ruleset writes into typed framework fields
   when it changes (the side's emotion, a navi's controller, whether its input is held), so the framework never
   asks Luau a question every tick. Rust never reads a ruleset's state (§5.3).
6. **Data the framework reads every tick stays data.** A form's hover height, its fire-charge limit, its glow
   table are fields the framework reads, not functions it calls.
7. **The traces don't move.** What the traces compare (the flow's states, a navi's action number, objects by
   slot) keeps its values: an action or kind that moves into Luau keeps its compat number under its new key.

### 2.2 The ruleset definition

A root that is a game defines one ruleset; its key is the game's (`bn6`). Its definition gathers what the
framework needs from it:

```luau
-- content/bn6/rules/ruleset.luau
--!strict
-- BN6's rules: what the framework asks of the game it runs.

local transform = require("./transform/sequencer")
local cross = require("./cross/cross")
local beast = require("./beast/beast")
local emotion = require("./emotion/emotion")
local custom = require("./custom/custom")
local setup = require("./setup")

return define.ruleset {
    id = "bn6",
    -- Typed state the battle keeps for the ruleset (§5): the whole battle's, and each side's
    -- (the systems' fields, gathered in setup.luau).
    state = { battle = setup.BATTLE_STATE, side = setup.SIDE_STATE },
    -- Each player's setup: from the save, read-only in battle.
    setup = setup.PLAYER_SETUP,
    -- What a custom screen's result carries besides the hand and the navi stats.
    result = transform.RESULT,
    hooks = {
        round_start = setup.round_start,
        turn_start = transform.turn_start,
        custom_requested = transform.custom_requested,
        custom_result = transform.custom_result,
        navi_hit = emotion.navi_hit,
        navi_deleted = cross.navi_deleted,
        chip_used = beast.chip_used,
        -- ...
    },
    custom = custom.SCREEN,         -- the screen's buttons, windows, hand size (§4.4)
    emotions = emotion.NAMES,       -- the emotions it pushes (§4.6)
    sections = require("./sections"),   -- the rule sections (elements, panels, custom screen, ...)
    roles = require("./roles"),         -- what the framework starts, spawns, shows and plays
    extends = require("./extends"),     -- the data it reads on other definitions (§7.5)
}
```

- `define.rules` and `define.roles` stop being pack singletons and become the ruleset's (`sections`, `roles`):
  the framework reads the battle's ruleset's (`b.ruleset().roles`). One root still has one of each.
- The round's setup names the ruleset (`RoundSetup::ruleset: RulesetHandle`); `Battle::new` refuses a setup whose
  per-player setup blocks aren't that ruleset's.
- A ruleset's functions are stateless like all content: the bytecode check refuses writes to globals and module
  locals, and its tables are frozen.

### 2.3 Where it lives

content/bn6/rules/ already holds BN6's rule sections and roles. The ruleset's systems go beside them, one folder
each, with the kinds and actions only that system uses colocated, as the content model's §4 rules ask:

```text
content/bn6/rules/
  ruleset.luau                the define.ruleset
  setup.luau                  the player setup and side state; round_start
  sections.luau, roles.luau   (and the rule sections' modules as today: elements, panels, collision, ...)
  transform/                  the turn-start sequencer, the form-change and revert actions, the Cross merge
  cross/                      the Cross change, knockout, protect, the Cross special, the SELECT special
  beast/                      the Beast rush, the lock-on marker, the afterimage, Beast Over's berserk, its burst
  emotion/                    emotions and mood, the swing bug
  custom/                     the Cross window, the Beast Out button, scrap and re-deal, the hand size
  api.luau                    the `bn6` module content calls (§4.6)
```

A form's own behaviour stays with the form (navis/megaman/forms/<form>/), as its weapons do (§3.1, forms).

## 3. What moves, and in what order

### 3.1 BN6-only systems, first

In the slice order (§8). "Per tick" is what each costs in Luau while its state lasts.

| System | Becomes | Uses | Per-tick Luau |
|---|---|---|---|
| **The turn-start transformations**: the sequencer (`sub_801483C`), the requests from the custom screen, the reversion before a custom screen (`sub_802D6A0`), Beast Out's count-down (`sub_8015A38`) | rules/transform/sequencer.luau; `Battle::{transform_requests, turn_transforms, transform_seq, custom_reversion}` become ruleset state | hooks `turn_start`, `custom_requested`, `custom_result`; `battle.fade`, `navi:request_form_change(form)` | while paused at a turn's start (about 40 ticks a turn) |
| **The form changes and reverts** (`sub_8014A38`, `sub_8015614`), the Cross merge (actor #0x1B), Beast Over's burst (effect #0x90) | content actions and kinds in rules/transform/, started by role (`actions.form_change`, `actions.form_revert`) | the framework's form application (`navi:apply_form`), its resets by name, overlays by identity | per tick of a change, paused |
| **Cross**: the Cross change mode, the knockout (`sub_802DD2A`, `sub_802D926`) and protect, a weakness hit breaking the Cross (`sub_8015766`), the Cross special (`sub_802E4E4`, `sub_80EFDB2`), the SELECT special (`sub_802F068`), the battle flag 0x40 mode's side state | rules/cross/; `Battle::{cross_stats, sides, crossed}` and the Cross request and state bits become ruleset state | hooks `navi_hit`, `navi_deleted`; pause actions; a controller for the specials | while a special or a change runs |
| **Beast Out and Beast Over**: the rush (`sub_80EAD9C`), the lock-on marker (#0x0F), the afterimage (#0x28), berserk (`sub_802D322`), Beast Over's drain and exhaustion | rules/beast/; the rush is a wrapper action, berserk a controller; `beast_out_used`, `beast_lockon`, `beast_out_spent`, `beast_over_exhausted` become ruleset state | hook `chip_used`; `navi:set_wrapper`, `navi:set_controller`, `navi:hold_input` | during a rush; in Beast Over; the marker every tick in a Beast form |
| **Emotions and mood** (`sub_8015B54`, `sub_8015BEC`, `sub_8013DA0`, `sub_801A200`) | rules/emotion/; anger, the swing bug's timer and roll become ruleset state | hooks `navi_hit`, `form_changed`; the emotion pushed with `battle.set_emotion(side, name)` | none, except the swing bug's timer while a navi has the bug |
| **The custom screen's BN6 parts** (`sub_8027834` to `sub_8027A58`, `sub_802770C`, `sub_80275EC`, `sub_8027406`, `sub_80271F8`, `sub_802A40C`), `Unlocks`, `GameVersion`, `CrossList` | rules/custom/: a window (the Cross window) and buttons (Beast Out, scrap, re-deal); the unlocks and version in the ruleset's player setup | hooks `custom.open`, the buttons' and window's functions, `custom.confirm` | per tick while the Cross window or a button's animation runs |
| **Forms' behaviour** | form hooks (`on_chip_use`, `on_enter`, `on_exit`, `tick`) in each form's folder; BN6's form fields (`kind`, `game`, `cross_of`, `beast`, `breaks_to`, `special_volley`, `cross_release_anim`, the Cross traits) become the bn6 ruleset's extension data (§7.5) | hooks `chip_used`, `form_changed` | none for data the framework reads (hover, fire charge, glow); a form's `tick` only in that form |
| **BN6's data and API** | `ChipData`'s BN6 fields and `Registry::Lockon` become bn6 extension data; `NaviStats`' BN6 fields, `SpTimes`, `bug_frags`, `navi_levels` the ruleset's setup and state; the BN6 parts of `CoreApi` and core.d.luau the `bn6` Luau module and content/bn6/bn6.d.luau | | none |

### 3.2 Series-common rules, parameterised by game, second

These are shared by the lineage but written with BN6's values and branches. They move when BN5's routine map
(multi-game.md §4.2, the BN5 groundwork) shows how BN5's differ, one at a time, each by rule 2 of §2.1. What is
known now:

| Rule | Mechanism | Expected |
|---|---|---|
| The custom gauge (`CustomGauge::FULL`, `rate_for`), banner lifetimes, the final-turn count, the judge's timings | rule sections the framework reads | data |
| The fade table (`FadeMode`) | a `fades` rule section, by name; BN6's Beast Out and transformation fades named there (S1 starts it) | data |
| `NaviStats` | split: the generic stats (HP, buster levels, custom level, Mega and Giga limits, shoes, element, form, weapons, chip recovery, NaviCust bugs) stay a Rust record; each game's own (BN6's version, supports, Beast Out counter, sun, tags, ChpShufl, NumbrOpn) are its player setup and side state. The bug-code writer (`set_byte_by_bug_code`, by NaviStats offset) becomes a ruleset table from code to stat name | data and a table |
| The emotion window's flicker on bugs (console RNG1) | stays framework if BN5's is the same, else a hook | to see |
| The custom screen's core: hand size, dark chips' checks, the slot layout | hooks `custom.open` (hand size), `custom.chip_check`; the layout is already a rule section | hooks |
| The hit kernel, statuses, counters, Full Synchro's doubling, element weakness | already data in part (rules/elements, statuses as definitions); the rest by the map | data, perhaps hooks |
| The dimming service, the navi-chip controller, obstacles | framework; parameters by the map | data |
| The navi framework's idle priorities, charge, movement | framework; a hook where a routine differs | to see |

### 3.3 What stays Rust

The engine services, which run per object per tick or carry rollback:

- **object pools and the update list** (lowest free slot, insertion after the updating object, pause and dimming
  gating): every object, every tick;
- **collision and hit resolution**: per-panel registration, remove-and-pair, the hit kernel's filters, guard,
  counter, element and status; every pair, every tick;
- **the panel grid**: cached flags, reservations, types, cracking;
- **RNG**: the shared stream and each console's, drawn in update order;
- **the digest, snapshots and cues**: the rollback contract (rollback.md);
- **sprites and animation stepping**, **input and the link**, **the content host**.

And the framework, as long as the games share it (§3.2). That is most of the 20,000 lines; what leaves is the
6,000 BN6-only lines and, later, the branches BN5 proves to differ.

## 4. The ruleset API

core.d.luau will declare it; this section is its design.

### 4.1 Hooks

A hook is a function in `define.ruleset { hooks = ... }` (or in its `custom` table). Each is optional; the
framework does nothing where one is missing. Arguments are objects, sides, definitions and small spec tables;
results are typed and checked by the binding as hook results are today (`hook_result`).

**The flow** (battle.rs):

| Hook | Called | BN6 does |
|---|---|---|
| `round_start()` | once, after the navis spawn | reads the player setup into side state (the Beast Out counter, the Crosses owned) |
| `turn_start() -> busy` | every tick of fighting state SETUP until it returns false | runs the sequencer twice, then counts Beast Out down (`sub_800840C`) |
| `custom_requested() -> busy` | every tick of fighting states CUSTOM_REVERT and CUSTOM_SEQUENCE until false, when the mode asks for transformations | the reversion, then the sequencer once more |
| `custom_result(side, result)` | when both results are in (`sub_800B3D8`) | stores the side's transformation request |
| `round_end()` | once, as the round finishes | records Beast Out used and crossed (read after the battle) |

**The custom screen** (custom/screen.rs; §4.4):

| Hook | Called | BN6 does |
|---|---|---|
| `custom.open(side) -> Offer` | once per screen per side | the hand size (`sub_802A40C`), which buttons are on the screen and selectable, the Crosses on offer |
| `custom.<button>.press(side) -> Next` | A on a ruleset button | Beast Out chosen, scrap, re-deal |
| `custom.<window>.update(side, pad) -> Next` | every tick while the window is open | the Cross window |
| `custom.chip_check(side, chip) -> chip` | per chip shown or picked | a dark chip's substitute without a bug frag, the Mega and Giga limits (`checked`, `within_limit`) |
| `custom.confirm(side) -> result` | OK | the transformation request (the original's transform record) |

**The navi framework** (kinds/player):

| Hook | Called | BN6 does |
|---|---|---|
| `navi_spawned(navi)` | at the navi's init | the starting form's parts; the lock-on marker in a Beast form |
| `navi_hit(navi, hit)` | once per tick in which the navi took hits, after the damage (where `sub_801A200` runs), with what the kernel decided (damage, element, weakness, counter, mood damage) | Full Synchro for the side that countered (by that side's form: its base form or a Beast), mood loss, anger, a weakness hit breaking the Cross |
| `navi_deleted(navi) -> keep` | where the framework would delete the navi | a Cross protected at the battle's end runs the protect; a Cross's knockout instead of deletion |
| `chip_used(navi, chip) -> Use` | once per chip use, after the framework's common path | the Beast rush's lock-on (a wrapper), the forms' damage bonuses by family, EraseCross's flag, Full Synchro's and anger's doubling (if BN5's differs; else framework) |
| `form_changed(navi, from, to)` | after a form is applied | the lock-on marker comes or goes, the glow, the emotion's tired state |

**Per tick, only while set** (§4.3): a navi's controller (Beast Over's berserk, the Cross special), its action
wrapper (the Beast rush), its pause-time action (the form change), a form's `tick`, a custom-screen window.

What BN5 is expected to use (souls, multi-game.md §2.3): `custom.open` and a window or button for choosing a
soul with a sacrificed chip, `custom.confirm` for the request, `turn_start` for the change and the three-turn
ending, `form_changed` and form hooks for the souls' abilities, `chip_used` for Chaos Unison's held dark chip.
None of it needs a Rust change once these hooks exist, which is the test of the seam.

### 4.2 Hook frequency and cost

| Kind of call | When | Calls per advance, typical |
|---|---|---|
| Flow hooks | paused turn starts and custom requests | 0 while fighting; 1 a tick while paused |
| Custom-screen hooks | the custom screen | 0 while fighting; a few a screen, 1 a tick while a window is open |
| Event hooks (`navi_hit`, `chip_used`, ...) | at the event | well under 0.1 |
| Controllers, wrappers, form `tick` | while set | 0 in base form; 1 a tick for the navi that has one |

At about 0.3 to 0.5 µs for an empty call and 2 to 6 µs for a call that does the work these do (§6.1), event
hooks cost nothing measurable on average; what matters is the per-tick calls while their state lasts, which §6
budgets.

### 4.3 Controllers, wrappers and pause actions

Three framework extension points take over a navi's tick while the ruleset says so. Each is a definition the
ruleset makes, set on a navi by handle in a framework field (so it is in the snapshot, and Rust checks one
`Option` per tick):

- **A controller** (`define.controller { update = function(navi) -> Outcome }`) replaces idle's own decision
  (`sub_80F0354`) while set: it returns nothing, moved, a chip, or the buster, and the framework carries the
  outcome out as idle does today. BN6: Beast Over's berserk, the Cross special. Set and cleared by the ruleset
  (`navi:set_controller(c)`, `navi:set_controller(nil)`).
- **A wrapper** is an action (`define.action { wraps = true, ... }`) the dispatcher runs instead of the navi's
  action while set; it runs the wrapped action when it chooses (`navi:run_wrapped()`) and ends the wrap. BN6: the
  Beast rush, set by `chip_used` for a chip with a lock-on mode in a Beast form.
- **Pause actions**: while the battle is paused, the pause handler (status.rs) asks the ruleset's
  `pause_actions` table, by request and state name, which action to run (BN6: the form change, the revert, the
  Cross change, the Cross knockout), instead of naming them in Rust.

Requests and state bits that are a ruleset's (`cross_change`, `cross_death`, `cross_special`, `select_special`,
`changing_cross`, `cross_knockout`, `crossed`, `cross_breaking`) leave the framework's flag words for the
ruleset's side state; the framework's own (`buster`, `chip`, `charged_shot`, ...) stay.

### 4.4 The custom screen

The chip window stays the framework's (deal, cursor, selection rules, Program Advances, modifiers, descriptions,
the run message, hiding, OK, sending); BN6's sub-screens become the ruleset's. The screen's phases gain two
generic ones and its slots one generic kind:

- `SlotKind::Button(ButtonHandle)`: a slot the layout (rules/custom-screen.luau) gives to a ruleset button.
  BN6's: the Beast Out button (at today's `SPECIAL_SLOT`), scrap and re-deal (over slots 8 and 9). A button's
  `available(side)` decides its state when the screen opens and after each pick; `press(side)` what A does.
- `Phase::Window(WindowHandle)`: a ruleset window, whose `update(side, pad)` runs every tick and returns stay,
  back to choosing, or a pick. BN6's Cross window (opening, choosing, closing, chosen) is one; BN6's Beast Out and
  BeastOut-chip animations, the scrap and the re-deal are windows too (they hold the screen for their ticks).
- Keys: in the choosing phase the framework first asks `custom.keys(side, pad)`, when the ruleset declares it,
  for keys the chip window gives no meaning of its own (BN6: Up on the top row or OK opens the Cross window).
- The result: `CustomResult::transform` becomes `CustomResult::rules`, the ruleset's `result` block (typed, a
  schema like state), which `custom.confirm` fills and `custom_result` reads. bn6-compat's codec writes BN6's
  transform record into it by field name.

Each side's window state lives in the ruleset's side state. Presentation (§4.7) reads it by field name.

### 4.5 What a ruleset can call

Everything content can, plus what only rulesets need. To keep content from reaching into a ruleset's state, the
`ruleset` library and the framework's ruleset operations are usable only from modules under the ruleset's
folder (a lint in nettai-content-check, as the colocation lints are).

- **State**: `ruleset.battle()`, `ruleset.side(side)` (read and write, cast to the declared type),
  `ruleset.setup(side)` (read-only), `ruleset.result(side)` (in `custom.confirm` and `custom_result`).
- **The flow**: `battle.fade(name, speed)`, `battle.fading()`, `battle.hud(part, shown)`, `battle.forces_custom(side,
  on)` (Beast Over opens the custom screen when the gauge fills), the turn number, the battle mode.
- **The navi framework**: `navi:request_form_change(form)` and `navi:changing_form()`, `navi:apply_form(form)` (the
  form's weapons, element, flags and parts: `sub_80144C0`'s resets, by name), `navi:revert_form()`,
  `navi:set_controller(c)`, `navi:set_wrapper(action)`, `navi:run_wrapped()`, `navi:hold_input(on)`,
  `navi:start_pause_action(action)`, the existing requests and state bits by name.
- **The custom screen**: inside its hooks, the screen of that side: the cursor, the selection, a pick added or
  taken back, the hand size, sounds and looks.
- **Pushed results**: `battle.set_emotion(side, name)` and `battle.full_synchro(side, on)`, the navi's palette
  rule (§4.6), whatever else the framework reads every tick.

### 4.6 The ruleset's API for content, and the emotions

Content today calls BN6-only API in Rust (`battle.bug_frags`, `battle.side_special`, `me.beast_lockon`,
`me.beast_out_spent`, `battle.emotion`, MstrCros's Crosses). After S8 these are a Luau module of the ruleset,
`content/bn6/rules/api.luau`, required as any module is (`local bn6 = require("../../rules/api")`), and declared
in content/bn6/bn6.d.luau. Each function works on the battle's ruleset state through the ruleset library and
**says what it does in another ruleset's battle**: `bn6.bug_frags(side)` is 0 there, `bn6.spend_bug_frags` does
nothing, `bn6.crosses(side)` is empty. A chip whose use makes no sense outside BN6 (MstrCros, the Gregar and
Falzar chips) says so with `requires = { "bn6" }` on its definition, and a setup check refuses it in another
ruleset's folder.

Emotions are each game's (BN6's five; BN4's and BN5's differ): the ruleset declares their names (`emotions`),
pushes the current one per side, and `battle.emotion(side)` returns that name. Full Synchro, which the lineage
shares, is a framework flag the ruleset pushes alongside, and what the framework's doubling reads.

### 4.7 Presentation

The frontend draws the HUD and the custom screen per game (multi-game.md §3.5): its BN6 module reads the
ruleset's state by field name (the Cross window's cursor and tick, the emotion), through a small accessor on
`Battle` that resolves a field once per frame. Making the drawing itself data or Luau is out of this design's
scope. The digest still leaves out what is presentation only.

## 5. Ruleset state

### 5.1 Declared, typed, engine-owned

There is no global script state: the VM holds nothing between calls, as for all content. What a ruleset keeps
across ticks it declares, as kinds and actions declare theirs, and the engine stores:

```luau
-- content/bn6/rules/transform/sequencer.luau (in part)
export type Battle = { seq: "check" | "transform" | "wait", phase: "fade_out" | "change" | "fade_in",
                       started: boolean, busy: boolean, pass: number }
sequencer.BATTLE_STATE = {
    seq = { "check", "transform", "wait" }, phase = { "fade_out", "change", "fade_in" },
    started = "bool", busy = "bool", pass = "u8",
}
```

- **Blocks**: `state.battle` (one), `state.side` (one per side), `setup` (one per player, read-only in battle),
  `result` (the custom screen's result per side). Each is a schema (`nettai_content_api::Schema`) the define phase
  reads, with the content-state field types (`bool`, `u8` to `i32`, enums, `object`, `vec3`, references to
  definitions and assets) and fixed arrays of any of them (`"form[5]"`, `"u16[20]"`).
- **Storage**: `Battle::rules: RulesetState { battle: StateBlock, sides: [StateBlock; 2] }` and
  `RoundSetup::players[side].rules: StateBlock`, where a `StateBlock` is the schema's id and a fixed array of
  bytes (512, against content state's 64), zeroed at the round's start. Plain `Copy` data: `Battle: Clone` is
  still the snapshot, `#[derive(Hash)]` still the digest, and the digest's destructuring guard makes the new
  field part of it. A snapshot grows by about 1.5 KB (to about 23.5 KB).
- **Access**: from the ruleset's Luau, `ruleset.side(0) :: Side`, then plain field reads and writes, at the cost
  of content state (about 140 to 190 ns a field); stores wrap to the field's width as everywhere.

### 5.2 What it replaces

| Today (Rust) | After |
|---|---|
| `Battle::{transform_requests, turn_transforms, transform_seq, custom_reversion}` | battle and side state of rules/transform |
| `Battle::{cross_stats, sides, crossed}`, the Cross request and state bits | side state of rules/cross (`cross_stats` as the side's reserve navi, see below) |
| `Battle::beast_out_used`, `AttackVars::beast_lockon`, `ActorData::{beast_out_spent, beast_over_exhausted}`, berserk's state | side state of rules/beast |
| `ActorData::{anger, emotion_swing_ticks, swung_emotion}` | side state of rules/emotion |
| `Battle::{bug_frags, navi_levels}`, `RoundSetup::sp_times`, `Unlocks`, `GameVersion`, `NaviStats::{version, beast_out_counter, support, sun, folder_tags, chip_shuffle, number_open}` | player setup and side state of rules/setup |
| The custom screen's Cross window, Beast Out and scrap state; `RoundMemory::{crosses_used, beast_out_used}` | side state of rules/custom |

`cross_stats` is a whole `NaviStats`, the side's other navi for the Cross change. If BN5's team navis need the
same (multi-game.md §2.3, unknown), it stays a framework field ("a side's reserve navi"); otherwise the ruleset
gets a `navi_stats` field type. The slice that moves Cross decides by what the BN5 map shows by then.

### 5.3 Rust doesn't read it

The framework never reads a ruleset's state: what it needs every tick the ruleset pushes into the framework's
own typed fields (§2.1 rule 5; §4.5's "pushed results"), and what it needs at an event comes back as the hook's
result. So the framework holds no field names of BN6's, and a second game's ruleset can't break it. The
exceptions are tools outside the simulation: the frontend's per-game presentation (§4.7) and the compat codecs
(bn6-compat writes BN6's setup and result blocks from saves and traces by field name).

### 5.4 Rollback

Nothing changes in the contract (rollback.md §8.2): all state is in `Battle`, plain and hashed; the ruleset's
functions are stateless and checked so; inputs are the buttons; peers whose setups agree run the same ruleset,
since the round's setup names it and the content hash covers its definition and code. Tests to add: a battle
rolled back in the middle of a turn's start sequence continues identically; a ruleset hook writing a module local
is refused at load; nettai-netplay's synthetic netbattles run with the ruleset's hooks.

## 6. Performance

### 6.1 Today

`rollback_cost` (rollback.md §6): the worst case of a 10-frame rollback on every rendered frame, through
getgud's `World` (a restore, eleven advances each followed by a save, a digest). Release build, Apple M1 Max shared
with other agents (load average about 24); best of five runs per row; engine main 20fcf9e4:

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

- **A rendered frame is about 45 µs fixed plus eleven advances**: the digest 19–20 µs, a restore about 3 µs,
  eleven saves about 2 µs each. Only the advances depend on the rules.
- **Luau is already most of an advance.** With the calls into Luau timed (a temporary patch, not committed):
  soundmod 1 makes 0.86 kind updates and 0.08 action updates per advance, which take about 2.6 of its 3.3 µs;
  machgun 1 makes 0.74 and 0.39, about 5.8 of 6.6 µs; beast-over about 79 %. A call costs at least 0.33 to 0.46
  µs (an update that does almost nothing) and typically 1.7 to 5.4 µs (the median kind update); the Rust engine
  is under a microsecond of an advance.
- So the survey's five-to-ten-times estimate was right in kind: a field access from Luau costs 140 to 360 ns
  against a nanosecond or two in Rust (scripting.md §7), and entering Luau costs 0.3 to 0.5 µs before the call
  does anything. What decides the cost is how many Luau calls a tick makes, not how many lines move.

### 6.2 What moving naively would cost

If the BN6-only systems moved as they are written, with the framework asking Luau at each of today's BN6 branch
points every tick (the emotion for the palette, the Cross and Beast request bits, the form's hover and fire
charge, the specials in idle, the lock-on marker), that is about five to eight calls per navi per tick, ten to
sixteen per advance: 20 to 50 µs per advance, 250 to 600 µs per rendered frame in a plain fight. The design's
rules (§2.1, 4 to 6) exist to avoid exactly this.

### 6.3 Budget

1. **The common state stays free.** No slice adds a per-tick Luau call to a navi in its base form with no
   controller, wrapper, window or special running: soundmod 1–3 and machgun 2 stay within the noise (±10 %).
2. **The ceiling**: every row of the basket stays under **500 µs per rendered frame** (3 % of a frame at 60 fps;
   about an advance of 40 µs), measured best of five. A browser or phone build several times slower would still
   be well inside its frame.
3. **Every slice reports** its basket before and after, with its Luau calls per advance. A row that grows by more
   than 25 % is explained in the report (which per-tick calls, in which state) and noted in this document's cost
   table.

The estimate at the end of the BN6-only slices: plain fights unchanged; machgun 1 plus the marker in Beast Out
(about 4 µs an advance) near 170 µs; beast-over plus berserk and the marker (about 9 µs an advance) near 235 µs;
the turn starts and custom screens add per-tick calls only while paused, with few objects running. All inside the
ceiling.

### 6.4 How to stay within it

- **Coarse hooks**: one call per event with the facts the hook needs in its arguments (a hit's damage, element,
  counter and weakness as a spec table), not many small calls back into the API.
- **Gating**: an unfilled hook is a `None`; a per-tick call is set by the state that needs it and cleared with it.
- **Push**: emotions, Full Synchro, controllers, input holds, forced custom screens are written into framework
  fields when they change.
- **Data on hot paths**: what the framework reads per tick is a field of a definition (a form's hover, fire
  charge, glow and palette table), not a function.
- **Rust fast paths for per-tick primitives**: if a per-tick kind of a ruleset is costly and generic in shape (the
  lock-on marker is "a marker that follows the nearest opponent's attach point"), it can be an engine primitive
  with the ruleset's data, as the hitbox and the effect are. Measured first; Luau by default.
- **The binding** (the reserve, and worth doing anyway since Luau is 80 % of an advance today): cache each
  object's userdata instead of making one per call (the fixed 0.3–0.5 µs), then the raw-FFI binding of
  scripting.md §7 and §9 (tagged userdata, `__namecall` on atoms, C `__index`), which would cut field access
  several times. A slice of its own (§8, B), taken when the budget needs it.

### 6.5 Measuring a slice

- **rollback_cost** gains a frame range (`--frames A..B`, or the whole round) for scenarios whose interesting part
  isn't their busiest frame, and an optional `luau-profile` feature in nettai-battle that counts and times calls
  into Luau per advance by kind, action and hook (what §6.1 measured with a temporary patch).
- **The basket**: the five golden rounds and the lab scenarios of §6.1's table, plus each slice's own (the
  Cross change, a Cross special, a turn start with both sides transforming), run best of N by a script in the
  verification workspace (tools/rollback-cost.sh), main and the branch alternating so the machine's load hits both.

## 7. Composable with BN6 content

### 7.1 What it means

The user: BN5 "should be composable with bn6 content". Concretely:

1. **BN5's content and BN6's load into one engine at once**, and a battle may hold either's chips, navis, forms and
   stages: a BN5 chip in a BN6 battle, a BN6 chip in a BN5 battle.
2. **Each game is still exact in its own battles**: a BN6 battle with BN6 content matches BN6's traces, a BN5
   battle with BN5 content BN5's. Mixing is defined by consistent rules, not by an original (none exists).
3. **Content can build on another game's**: a BN5 module may `require` a BN6 library where BN5's routine is the
   same, and a ruleset may compose others' systems (a "BN5 with Crosses" ruleset is content, not an engine fork).

### 7.2 Roots

A root is a content directory with a manifest:

```toml
# content/bn6/root.toml
name = "bn6"
requires = ["nettai"]      # roots whose modules it may require
ruleset = "rules/ruleset"  # the module defining its ruleset, if it is a game
assets = "bn6"             # the asset pack its asset names resolve in
```

- **content/nettai** holds what isn't any game's: the API declarations (the generic part of today's core.d.luau
  and types.d.luau) and, if the user wants shared content (multi-game.md §6 question 7), libraries shared by
  games. It defines no ruleset.
- **content/bn6** and **content/bn5** are games: each defines its ruleset, its content, its compat (compat/ stays
  per root, read only by that game's `<game>-compat` crate) and its declarations (bn6.d.luau).
- **Loading**: `load_battle(roots, packs)` reads every root named, in dependency order, each module with its root's
  name; the define phase runs once over all of them; the content hash covers every root's definitions, scripts and
  asset index.
- **`require`** stays relative inside a root; across roots it names the root: `require("@bn6/lib/bombs")`, allowed
  only for roots in `requires`.

### 7.3 Keys, handles and compat

- A definition's key is its root's name and its key in the root: `bn6:minibomb`, `bn5:minibomb`. Inside a root,
  modules and compat write the key without the root, as today; the loader qualifies it. The engine's own entries
  keep their `engine/...` keys, outside every root.
- Handles intern over the union in byte order of the qualified keys, so peers with the same roots and the same
  content hash have the same handles, as now.
- Each root's compat maps that root's keys to its game's numbers: bn6-compat reads content/bn6/compat for BN6's
  traces and saves, bn5-compat content/bn5/compat for BN5's. A trace of one game names only its own root's
  content.
- Setups name content by qualified key (a folder of `bn5:cannon` and `bn6:minibomb` is a valid folder).

### 7.4 Assets

Each root's `asset.*` names resolve in its own pack (`asset.sprite("bomb")` in a BN5 module is BN5's bomb). The
engine's asset handles cover every loaded pack; a sprite's identity gains its pack (`SpriteId` is a pack, a
category and an index); the frontend and the audio load each pack a battle's roots name.

### 7.5 Which rules apply where

- **The battle's ruleset rules the battle**: the flow, the custom screen and its buttons and windows, the
  transformations, the emotions, the damage pipeline's game rules, the rule sections (elements, panels, statuses'
  tables, the custom gauge), the roles (the sounds, effects and banners the framework uses). One ruleset per
  battle; both players play under it.
- **A definition's behaviour is its own**: a chip's use, a kind's update, a weapon's setup, a form's hooks run as
  their root wrote them, on the framework's services.
- **A definition's common record means the same in every ruleset**: a chip's codes, element, class, MB, damage,
  counter parameter, flags and lockout; a form's element, weakness, buster bonus, weapons and charged chips.
- **A ruleset's own data on definitions is an extension the ruleset declares**: `define.ruleset { extends =
  { chip = { lockon = "lockon?", rush = "...", traits = { ... } }, form = { kind = { "base", "cross", ... }, game
  = ..., cross_of = "navi?" } } }`. A definition in the ruleset's own root writes it flat, as today (BN6's chips
  keep `lockon = ...`); a definition of another root may add it under the ruleset's name (`bn6 = { lockon =
  lockon.forward }`) to say how BN6 should treat it. The ruleset reads it in Luau (`bn6.chip(c).lockon`).
- **Content without a ruleset's extension gets that ruleset's declared defaults**: BN6's are no lock-on (the chip
  runs where the navi stands, even in a Beast form), family `null` (no Cross bonus), not a dark chip.
- **A ruleset's API module says what it does in another battle** (§4.6), and content that needs one game's
  ruleset says `requires`.

### 7.6 Worked cases

- **A BN5 chip in a BN6 battle.** Say BN5's Sword (bn5:sword), verified against BN5's traces. The BN6 custom
  screen deals and picks it by its common record; BN6's Program Advances name BN6's chips, so it makes none
  unless a BN6 recipe names it. Its use runs bn5:sword's action on the framework's services, with BN6's hit
  kernel, statuses and elements from BN6's rule sections. In a Beast form it has no BN6 lock-on, so no rush:
  MegaMan swings where he stands. A Cross's damage bonus applies by family as BN6 defines families for chips
  without its extension (the default, `null`, or a mapping the BN6 ruleset declares from BN5's families).
- **A BN6 chip in a BN5 battle.** MiniBomb (bn6:minibomb) runs its throw as in BN6, under BN5's flow and custom
  screen. Its lock-on data is BN6's extension, which BN5's ruleset doesn't read. BugRSwrd's charged shot asks
  `bn6.spend_bug_frags`, which does nothing outside BN6, so it fires the plain shot. MstrCros `requires` bn6 and
  can't be put in a BN5 folder.
- **A BN6 Cross in a BN5 battle.** Not offered: Crosses are what BN6's ruleset offers on its custom screen. A
  ruleset that offers BN5's souls and BN6's Crosses is a third ruleset in Luau (a modder's `bn56`), requiring
  both roots and composing their systems; nothing in Rust changes for it.
- **A BN5 stage in a BN6 battle.** Its layout names panel types; a type BN6's panels section lacks (BN4's metal)
  is refused when the setup is checked.

### 7.7 Verification

Each game is verified in its own battles by its own oracle: BN6's traces and lab through bn6-compat as today,
BN5's through bn5-compat when it exists. Mixed battles have no oracle; in-repo tests check they run and roll back:
`every_chip_runs` over each ruleset with every root's chips (minus `requires`), and a rollback test with a mixed
folder.

## 8. The slice plan

### 8.1 Every slice

- Branches from current main, merges main often (other agents change content and the engine in parallel), and
  lands on main before the next starts.
- Ports from the Rust as it stands, which is the verified port of the disassembly; every branch kept. Anything
  the move shows unported or wrong is fixed and listed.
- Gates, once at the end (phase-b-brief): the build without warnings, `cargo test --workspace`, the content check,
  `gen-content check` (compat changes in most slices: kinds and actions under new keys keep their numbers), both
  golden traces with rollback at every latency and the sound calls, the full lab with the sound gate. **The
  rollback cost basket before and after** (§6.5).
- Verify-side changes (a renamed kind's key in `DEFINED_KINDS`, codec changes in the harness's users) on a verify
  branch of the same name, merged with it.
- Docs: this document's "As built" notes per slice, core-content-boundary.md and content-migration.md where the
  line or the patterns move, docs/engine where it names the moved code.

### 8.2 The slices

| # | Slice | Moves (Rust out, Luau in) | New mechanism | Lab focus | Cost risk |
|---|---|---|---|---|---|
| S0 | **Groundwork** | none | `define.ruleset`, `RulesetHandle`, `RoundSetup::ruleset`; `Battle::rules` (empty blocks), the player setup block; `ruleset.*`; the hook table (empty); roles and rule sections under the ruleset; rollback_cost's frame range and `luau-profile`; tools/rollback-cost.sh | all (a no-op slice) | none |
| S1 | **The turn-start transformations** | transform.rs, the BN6 parts of `fight_setup`, `fight_custom_revert`, `fight_custom_sequence`, `count_down_beast_out` (about 400 lines) | `turn_start`, `custom_requested`, `custom_result`, `round_start` hooks; the custom result's ruleset block; `navi:request_form_change`; the `fades` rule section | forms/*, custom/take-back-*, flow/*; machgun 1 | per tick at turn starts, paused |
| S2 | **The form changes** | actions/transform.rs, cross_merge.rs, beast_over_burst.rs (about 990) | pause actions by table; `navi:apply_form` and the named resets; two kinds and two actions under new keys (compat numbers kept) | forms/* (every Cross, Beast, Beast Over, Cross-to-Cross) | per tick of a change, paused |
| S3 | **Cross** | cross_change.rs, cross_special.rs, the Cross parts of status.rs and idle.rs, `cross_stats`, `sides` (about 700) | `navi_deleted`, `navi_hit`; controllers; the ruleset's request and state bits | forms/*/cross-*, flow/counter-in-cross, the specials' scenarios | while a special runs |
| S4 | **Beast Out and Beast Over** | beast_rush.rs, berserk.rs, lockon_marker.rs, afterimage.rs, the Beast parts of chip_use.rs, idle.rs, player/mod.rs (about 1,400) | wrappers; `chip_used`; `navi:hold_input`, `battle.forces_custom` | forms/*/beast-*, chips' beast scenarios, machgun 1 | the marker in a Beast form; berserk in Beast Over; measure the marker and decide (§6.4) |
| S5 | **Emotions and mood** | `emotion`, `set_mood`, `emotion_timer`, `counter_and_mood` and the mood in status.rs and intake.rs (about 250) | `navi_hit`, `form_changed`; the pushed emotion and Full Synchro; ruleset-declared emotion names | flow/anger-*, flow/synchro-*, flow/counter-*, custom/beast-out-greyed | none in the common state |
| S6 | **The custom screen's BN6 parts** (two slices: a, the buttons, hand size and the player setup; b, the Cross window) | screen.rs's BN6 phases and slot kinds, `Unlocks`, `GameVersion`, `CrossList`, library.rs's form queries (about 800) | `SlotKind::Button`, `Phase::Window`, `custom.*` hooks; the frontend's BN6 module reads ruleset state by name | custom/* (all), forms/* | per tick of a window |
| S7 | **Forms' behaviour and BN6's data** | `FormKind`, `game`, `cross_of`, `beast`, `breaks_to`, the Cross traits; `ChipData`'s BN6 fields; `Registry::Lockon` (about 600, mostly data) | form hooks; `extends` with defaults | everything (records change shape) | none |
| S8 | **BN6's API surface** | the BN6 parts of `CoreApi`, core_api.rs and core.d.luau | the `bn6` module; `requires`; core.d.luau split into content/nettai and content/bn6/bn6.d.luau | everything | none |
| R | **Roots** (composability; can run beside S1–S8) | none | root manifests, content/nettai, qualified keys, `require("@root/...")`, per-root compat and packs, `SpriteId` with a pack; a made-up second root in the test content with its own ruleset; `every_chip_runs` across rulesets | everything (keys qualified) | none |
| B | **The binding** (when the budget needs it) | none | cached object userdata; the raw-FFI binding | everything | lowers the cost |
| P1… | **Series-common, by BN5's routine map** | per §3.2: the gauge, banners, fades and turns as data; the `NaviStats` split and the bug-code table; the custom screen core's hooks; whatever the map shows differs in the hit kernel, idle, charge, dimming | rule sections and hooks | by area | small |

Sizes: S0 about a day; S1, S2, S5, S7 and S8 one to two days each; S3, S4 and S6 two to three; R two to three;
about three weeks of one agent for S0 to S8 and R, at today's pace of a full check per slice. The P slices are
sized when the map exists.

### 8.3 Order and coordination

- **S0 first**, then S1 → S2 → S3 → S4 (each builds on the last's state and hooks), S5 after S4 (Beast Out's
  tired state is an emotion), S6 after S3 and S4 (the Cross window and the Beast Out button offer what they
  define), S7 and S8 last of the BN6-only set.
- **R** is independent of the rules moves and can land whenever the BN5 work needs content/bn5 to load; the
  recommendation is right after S2, so the BN5 groundwork has a root to put compat and asset names in early.
- **Parallel work**: the patch cards change `NaviStats` and setup (S1's and S7's player setup, and P1's
  `NaviStats` split, come after them); the JP agents add content and compat entries (additive, merged as they
  come); the BN5 groundwork's extractor writes a pack, which R's per-root packs read (agree the pack's manifest
  name with it).

### 8.4 Risks

- **Fidelity**: these systems are among the most intricate the lab covers (forms/* alone is several hundred
  scenarios). The port is Rust to Luau, not disassembly to Luau, so each routine's branches are already known;
  the lab and machgun 1 are the gates.
- **Luau's checks**: the type checker doesn't follow `require`d shapes (scripting.md §3.3), so the ruleset's
  shared types go in bn6.d.luau, and the lab catches the rest.
- **The custom screen and the frontend**: S6 changes what the frontend reads; it needs the frontend's frame
  comparison for the custom screen (frontend.md) as an extra gate.
- **Cost**: the lock-on marker and berserk are the two per-tick costs in long states; both are measured in S4 and
  have a Rust fallback.
- **Wide diffs**: S7, S8 and R touch most content modules' records or keys; they go when few branches are in
  flight, with a script other branches can run (as the nettai rename did).

## 9. Decisions for the user

1. **The budget** (§6.3): the common state free, a 500 µs ceiling, reports per slice. Tighter, looser, or another
   measure?
2. **The custom screen** (§4.4): the chip window stays framework with ruleset buttons and windows (proposed), or
   the whole screen moves into the rulesets?
3. **Where the ruleset lives** (§2.3): content/bn6/rules/ with a folder per system (proposed), or a separate
   content/bn6/ruleset/?
4. **Roots** (§8.3): after S2 (proposed), first, or after the BN6-only slices?
5. **Keys** (§7.3): `bn6:minibomb` qualified by the loader (proposed), with today's keys unchanged inside a root?
6. **Mixed battles** (§7.5): one ruleset per battle for both players (proposed), or a ruleset per player?
7. **The binding** (§6.4): only when the budget needs it (proposed), or early, since Luau is already 80 % of an
   advance?
8. **Shared content** (multi-game.md §6 question 7, still open): content/nettai for shared libraries, or per-game
   copies only?
