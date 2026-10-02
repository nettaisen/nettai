# Patch cards (改造カード): how EXE6 does them, and the plan for nettai

**Status: scoping, waiting for the user's approval (2026-10-02).** Nothing in the engine or the content has
changed for this yet. The verification workspace has `tools/jp/patchcards.py`, which reads everything below from a
Japanese ROM.

The user's request: "add bn6 jp patch card (kaizou card) support". The Japanese releases (EXE6 Falzar, BR6J; EXE6
Gregar, BR5J) read e-Reader Modification Cards. A card changes MegaMan's stats and abilities, often with a bug
attached. The US release cut the feature. This document says how the Japanese games do it (§1), what the cards do
(§2), what a battle with cards reaches that the engine doesn't have yet (§3), the plan for the engine and the
content (§4), how to verify it (§5), what isn't in a ROM and what needs a decision (§6), and the work in order
(§7). Appendix A lists the 117 cards.

Addresses are EXE6 Falzar's. EXE6 Gregar has the same code 0x1DC8 bytes later (the apply routine at 0x08143FF8),
and the same card table, byte for byte. US routine names are the disassembly's (bn6f).

## 1. How the Japanese games do it

### 1.1 From the card to the save

A received card reaches the game as its number, 1 to 117: the receiving screen's code (0x0812FB2E, 0x0812FB50)
installs it with 0x08141868.
The game keeps the installed cards in a 0x80-byte block at 0x020065F0. The block is part of the save (Tango's
dataview reads the same offsets from a save image: count at 0x65F0, the list at 0x6620):

| Offset | Size | What |
|---|---|---|
| +0x00 | u8 | how many cards are installed |
| +0x02 | u16 | working HP (MaxHP, then each HP effect) |
| +0x04..+0x28 | u8 each | the slots: one value per kind of effect (§2.1) |
| +0x2A | u16 | the installed cards' MB; adding a card fails past 80 |
| +0x2C | s16 | the HP the cards added (MaxHP after minus before) |
| +0x30..+0x4F | u8 each | the installed cards, in slot order: the number in bits 0-6, bit 7 set = switched off |
| +0x54..+0x78 | u8 each | slot +N was written by a card (the flag for slot N is at +0x50 + N) |

The menu adds a card only if it isn't installed already and the MB stay within 80 (0x08141868), and toggles bit 7
to switch a card off (0x08141924). A new game zeroes the block (0x0814185C, from `reqBBS_init_8004DF0`).

### 1.2 The apply routine

The US `reloadCurNaviStatBoosts_813c3ac` recomputes MegaMan's NaviStats from the NaviCust. Its JP twin
(0x08147268) does the same, then, when MegaMan is the PET's navi and event flag 0x163 is clear, calls the apply
routine (0x08142230). With 0x163 set it calls `sub_8121154` instead, as the US does. With a link navi operated
neither runs: **cards apply only to MegaMan**.

The apply routine (0x08142230):

1. Clears event flag 0x1723 and NaviStats+0x4C. Nothing more happens with no cards installed.
2. If the block, as the last application left it, says a card gave ChpShufl (slot +0x26 written, value 1),
   NumbrOpn is turned off (NaviStats+0x61 = 0). This reads the block before step 4 rewrites it, so a save that has
   never had the cards applied doesn't do it the first time.
3. If card 111 (Bass BX) is installed and switched on, NaviStats+0x4C = 2. Only the overworld's map scripts read
   that byte (`MapScriptCmd_switch_case_from_navi_stats_4c`); no battle code does.
4. Recomputes the block (0x08141AEC). It zeroes +0x02..+0x26 and the flags, then copies MaxHP (NaviStats+0x42)
   to +0x02 and +0x2C, Attack, Rapid and Charge (+0x01..+0x03) to slots +0x04..+0x06, CustomLevel, MegaLevel and
   GigaLevel (+0x0A..+0x0C) to +0x07..+0x09, and the body element (+0x10) plus 4 to +0x0A. Then, for each
   installed card in slot order (stopping at an empty slot, skipping switched-off cards), it runs the card's
   effects **in effect-id order**, 0x00 to 0xA9, not in the order the card lists them. For each id it takes the
   card's first entry with that id; a parameter of 0xFF counts as no entry. Each effect's handler (the table at
   0x08141BA8) writes its slot and sets its flag (§2.1).
5. Writes the working HP to MaxHP (NaviStats+0x42), and the HP the cards added to +0x2C.
6. Copies the 35 slots +0x04..+0x26 whose flags are set into the NaviStats (the table at 0x081422E8), mapping
   some values through lookup tables (§2.1). If a card gave BugStop with value 1 (slot +0x25), the bug slots
   +0x1C..+0x24 are not copied.
7. Counts bugs in the resulting NaviStats (0x08142578): +0x31 = 1, +0x13, +0x14, +0x16, +0x54 (u16), +0x24,
   +0x18, +0x19, +0x1A nonzero, +0x28 = 1, +0x26 = 1, +0x0D = 0xFF. If any, it sets event flag 0x1723.

Back in the reload, the HP rule is the US one: CurHP = MaxHP in the real world, CurHP = min(CurHP, MaxHP) on the
Internet.

The game calls the reload when the save loads and after the PET menus, so the save's NaviStats always hold the
cards' result. The block's values aren't a record of what the NaviCust gave: once applied, the stats before the
cards are gone (§5.2 records them from the routine's entry).

### 1.3 What the battle reads

**Nothing in the battle reads the card block.** The block's accessors are called from the menus (0x08121B52,
0x0812EA96, 0x0812EBF0, 0x0812EDC4, 0x08123A42), the overworld's HP routine (0x0803DF92), the new-game reset and
the reload. The battle sees the cards through two things:

- **The NaviStats** (§1.4). Every card effect that reaches a battle is a NaviStats value.
- **The emotion window's glitch.** `sub_813BF1C`, which the HUD's setup reads (the US routine has the 0x1723 test
  as unreachable code after an unconditional branch), is in JP: with cards installed (count nonzero, whether
  switched on or not), the glitch is on if flag 0x1723 is set; without cards, if flag 0x1720 is (the NaviCust's
  bug flag, as in the US). docs/engine/jp-differences.md §8 #17 noted the hook.

The JP battle code is otherwise the US code (jp-differences.md §2: no other routine differs for cards). So the
cards' battle behaviour is US code: the weapon routines, actions and objects the US ROM has but that no US data
names (§3).

### 1.4 A netbattle

The init exchange sends each console's current PET NaviStats, 0x64 bytes (battle-flow.md §3.1.1). Those are the
save's, with the cards applied at load. **A netbattle carries the cards' effects in the NaviStats, not the cards.**
The card list stays in each console's save, and the emotion window's glitch flag is each console's own. Tango's
EXE6 netplay exchanges nothing more than a real link cable does, so the same holds there.

## 2. The cards and their effects

**117 cards** (numbers 1 to 117), each 2 to 6 effects and 5 to 80 MB, in the ROM's card table (0x081429B0): a u16
offset per card number, then per card a header entry (0xAB, MB, 0) and its effects as (id, parameter, shown as a
bug) triples. The third byte only colours the menu text; the apply routine never reads it. Effect names are a
text archive (0x0812F224's pointer). EXE6 Gregar's table (0x08144778) is the same bytes.

**170 effect ids** have a handler (0x00 to 0xA9). Ids 0xAA to 0xB7 have names (the NaviCust programs' names:
Collect, Humor, ... SpprtBug) but no handler: the apply routine stops at 0xAA. **The cards use 134 of the 170 ids,
in 209 distinct (id, parameter) pairs.**

### 2.1 The slots

The 170 ids write 37 slots. 35 slots are copied to the NaviStats; slots +0x27 and +0x28 (OilBody, Fish, Battery,
Jungle, Search, and the ten Crosses' names) are written but never copied, and no card has them.

| Slot | Effect ids | Written | NaviStats |
|---|---|---|---|
| HP (+0x02) | 0x00 +p×10, 0x01 +p%, 0x02 −p×10, 0x03 −p% | clamp 1..9999 after each | +0x42 MaxHP |
| +0x04 | 0x09 +p, 0x0A −p, 0x0B (a+1)×p−1 | clamp 0..9 | +0x01 Attack |
| +0x05 | 0x0C +p, 0x0D −p | clamp 0..4 | +0x02 Rapid |
| +0x06 | 0x0E +p, 0x0F −p | clamp 0..4 | +0x03 Charge |
| +0x07 | 0x10 +p, 0x11 −p | clamp 2..8 | +0x0A CustomLevel |
| +0x08 | 0x12 +p, 0x13 −p | clamp 1..10 | +0x0B MegaLevel (folder limit) |
| +0x09 | 0x14 +p, 0x15 −p | clamp 1..10; 0x15 doesn't clamp (no card has it) | +0x0C GigaLevel |
| +0x0A | 0x04..0x08 (param = the id) | | +0x10 = p − 4: Null, Fire, Aqua, Elec, Wood body |
| +0x0B..+0x0F | 0x16..0x1A | p | +0x23 SuperArmor, +0x52 StatusGuard, +0x1B FloatShoes, +0x1C AirShoes, +0x1D UnderShirt |
| +0x10 | 0x1B..0x22 | the id | +0x04 the B button's weapon routine: 0x8C, 0x8E, 0x7D, 0x81, 0x76, 0x8F, 0x52, 0x00 |
| +0x11 | 0x23..0x29 | the id | +0x4D the buster shot's program (projectile variant): 7, 0xE, 0xD, 8, 0 (none), 0x1E, 0x1F |
| +0x12 | 0x2A..0x56 | the id | +0x05 the charged shot's weapon routine (45 values, 0x56: 0xFF) |
| +0x13 | 0x57..0x6B | the id | +0x4F the charged shot's program: 0x12, 0x13, 9, 5, 0x14..0x1B, 0x20..0x27, 6 |
| +0x14 | 0x6C..0x71 | the id | +0x06 the first barrier: 1, 5, 7, 8, 9, 0 (none) |
| +0x15 | 0x72..0x7A | the id | +0x07 the B+Back weapon routine: 0x3B, 0x8B, 0x3D, 0x83, 0x86, 0x53, 0x88, 0x89, 0xFF |
| +0x16 | 0x7B, 0x7C | p | +0x50 ChipRecovery (u16) |
| +0x17 | 0x7D..0x7F | the id | +0x08 the gauge: fast, slow, normal |
| +0x18..+0x1A | 0x80..0x82 | p | +0x0D bits 0, 1, 2: Rush, Beat, Tango |
| +0x1B | 0x8F | p | +0x15 = p >> 4 (no card has it) |
| +0x1C | 0x83 | p | +0x31 the step bug (bug slot) |
| +0x1D | 0x84..0x8D | p | +0x12 = trail[p >> 4] (3, 1, 2, 7, 6, 2, 5, 4, 2, 0, 0xFF), +0x13 = p & 0xF (bug slot) |
| +0x1E | 0x8E | p | +0x14 = p >> 4 blank shots, +0x15 = p & 0xF charged shots (bug slot) |
| +0x1F | 0x90..0x93 | the id | +0x16 the hit status: 1, 2, 3, 0 (bug slot) |
| +0x20 | 0x94 | p | +0x54 the custom screen's damage (u16; bug slot) |
| +0x21 | 0x95 | p | +0x24 the emotion bug (bug slot) |
| +0x22 | 0x96 | p | +0x18 = p ? min(+0x18 + p, 7) : 0, the HP drain (bug slot) |
| +0x23 | 0x97 | p | +0x19, likewise, the custom screen's HP drain (bug slot) |
| +0x24 | 0x98 | p | +0x1A = 0, 9 or 10 for p = 0, 1, else: the battle-start bug (bug slot) |
| +0x25 | 0x99 | p | +0x1F BugStop (no card has it) |
| +0x26 | 0x9A | p | +0x60 ChpShufl (and NumbrOpn off, step 2) |

Where the value is "the id", the card's parameter is the effect id itself (Canodumb's M-Cannon charge is (0x2C,
44)), and a later card's write replaces an earlier one's: one card's charged shot wins over another's.

Many bug-named effects are fixes: a parameter of 0 (or a table's 0) clears a bug the NaviCust gave. BomBoy's
"PanelBug 0x90" clears the panel trail; ToadMan's "BstBlank 0" clears the blank shots.

### 2.2 What the cards do, by kind

- **Stats**: HP (42 cards add HP, 15 add a percentage, 21 take some away), Attack, Rapid, Charge, CustomLevel,
  the folder limits. Attack goes to 9 (the US NaviCust stops at 4); KnightMan doubles it.
- **Bodies**: 50 cards give an element (Fire 10, Aqua 17, Elec 11, Wood 11, Null 1). The US NaviCust never sets
  MegaMan's base element.
- **NaviCust abilities**: SuperArmor, StatusGuard, FloatShoes, AirShoes, UnderShirt, Rush/Beat/Tango, ChipRecovery,
  ChpShufl, the gauge; and their removal as bugs ("no AirShoes").
- **The B button** (7 cards): Sword, MiniBomb, CrakShot, meteors, and Bass's triple buster.
- **The buster shot's program** (12 cards): cracking, freezing, grass, poison, pushing shots; or none.
- **The charged shot** (53 cards): 44 kinds, most of them a chip fired as the charged shot (M-Cannon, YoYo,
  WideSwrd, Tornado, GunDelS1, Z-Saver...), plus Count's sand storm, Bass BX's nine-shot buster and the Bass Cross
  card's spinning top. Nine cards mark "ChrgS" as a bug: a charged shot like MegaMan's own, but with its
  program on every shot instead of on a lucky RNG2 draw (`sub_80125D0`), which replaces any better one.
- **The charged shot's program** (18 cards): confusion, paralysis, erasing, bubbles, roads, grass, freezing, ice.
- **The first barrier** (9 cards): Barrier, Barr100, Barr200, BblWrap, LifeAura, or none.
- **B+Back** (14 cards): Shield, Reflect, AntiDmg, immobilize, a fan, meteors, RskyHny, MchnSwrd, or none.
- **Bugs and fixes** (73 effects): step, panel trail, blank shots, hit status, custom damage, emotion, HP drains,
  battle-start. 104 of the 117 cards mark at least one effect as a bug.

Appendix A has every card.

## 3. What a battle with cards reaches that the engine doesn't have

The completeness audit (docs/engine/completeness.md) follows the weapon table through the numbers US data names,
so everything only a card names was out of scope. With cards it is reachable:

1. **55 weapon routines** (`off_80117D4`), all in the US ROM:
   - charged shots: 44, of which 38 load a chip as the attack (`loc_80126EA`: M-Cannon, IceCube, GrasSeed,
     PanlGrab, ...; every chip they load is defined) and 6 set up their own attack: the invisibility (instant
     effect 2, `sub_8012464`), the bubble spread (action 0x25), ChrgS (action 0x16, `sub_80125D0`),
     the sand storm (action 0x5E), the nine-shot buster (action 0x5D, `sub_8012124`) and the spinning top
     (action 0x38, `sub_8012144`);
   - the B button: 5 new (Sword, MiniBomb, CrakShot and the unused RflectR as chips; the triple buster;
     the meteors, action 0x5C);
   - B+Back: 5 new (immobilize, instant effect 9; the fan, instant effect 20; the meteors, action 0x5C; RskyHny and
     MchnSwrd as chips).
   Their charge times are rows of the charge table (`byte_8020404`), which the weapons' `charge_ticks` take.
2. **Actions 0x5C, 0x5D and 0x5E** (the completeness audit's "actions 0x5C and 0x5E" row, out of scope until
   now): the meteor drop (spawns the meteor shower of instant effect 16, ported), the nine shots, the sand storm
   (spawns instant effect 17's storm, ported). Small routines.
3. **Instant effect 20** (`sub_80CD4AC`, the fan): not ported. Effects 2 (invisibility) and 9 (the immobilizer)
   are ported but no recording has run them.
4. **First barrier types 5, 7, 8, 9.** The engine's `first_barrier` hook raises the Barrier chip's barrier
   whatever the stat says (the US NaviCust only writes 1). `sub_8013892` passes the type to `sub_801A7CC` and to
   the visual.
5. **MegaMan's body element.** NaviStats+0x10 nonzero for MegaMan: his weakness and his element's interaction with
   panels and Crosses. The engine copies the stat to the navi at init; no recording has had it nonzero.
6. **Stat values the US never produces**: Attack 5 to 9, CustomLevel 8 from the stat, HP up to 9999, the hit
   status and HP drains from a card, ChipRecovery 0.
7. **The emotion window's glitch from cards** (§1.3), which needs NaviStats +0x26 and +0x28 (two NaviCust bugs
   the engine doesn't model; nothing in the battle uses them).

The engine also panics on any weapon number compat doesn't know (bn6-compat's codec), so today a JP trace from a
console with a weapon card fails at setup.

## 4. The plan for the engine and the content

### 4.1 Setup

- **`PlayerSetup::patch_cards: Vec<InstalledCard>`** (`InstalledCard { card: PatchCardHandle, enabled: bool }`):
  the player's installed cards in slot order. Switched-off cards are kept because they decide which flag the
  emotion window reads (§1.3). Part of the shared setup, like the folder; the content hash covers the cards'
  definitions.
- **`RoundSetup::navi_stats` becomes "the navi's stats before its patch cards"** (the NaviCust's). For US games
  and players without cards nothing changes.
- **`Battle::new` applies each side's cards** to its stats before anything copies them (`stats`, `cross_stats`),
  through the ruleset's Luau (`rules/patch-cards.luau`, §4.2), when the side's navi is MegaMan. It also replaces
  the console's `emotion_window_glitch` with the bug count of §1.2 step 7 when the player has cards installed.
  It runs once, before the first tick: no state for snapshots, no cost per tick.
- NaviStats gains the two bug bytes the glitch counts (+0x26, +0x28), carried by the codec.
- No region switch (the 2026-10-02 decision stands): cards apply wherever the setup has them; a US console simply
  never has any.

### 4.2 Content

- **`define.patch_card`** in content/bn6/cards/\<key\>/card.luau: `id` (the key), `name`, `mb`, and `effects`,
  a list of typed effect values built by `rules/patch-cards.luau`'s constructors: `cards.hp(8)`,
  `cards.hp_percent(-20, { bug = true })`, `cards.body("aqua")`, `cards.charged_shot(require("./charge"))`,
  `cards.first_barrier(barriers.barrier_100)`, `cards.b_back(require("../../navis/megaman/weapons/shield"))`,
  `cards.hp_drain(1, { bug = true })`... Weapons, programs (projectile records), barriers and gauges are named by
  handle, never by number. The card has no number: compat/cards.toml maps keys to the save's numbers.
- **`rules/patch-cards.luau`**: the ruleset's half. The effect constructors (each knows its kind, i.e. its slot
  and order, and its parameter), the application (§1.2 steps 2 to 7: the slot block, the clamps, the effect-id
  order, the overwrite, BugStop, the copy, the bug count), as Luau over a typed view of the setup's NaviStats.
  The quirks are kept: the order by effect kind, a later card's choice winning, GigaFolder− not clamping.
- **Weapons** colocated by the content rules: a charged shot one card names lives in that card's folder
  (cards/canodumb/charge.luau, key `canodumb/charge`); one several cards name lives in lib/patch-cards/ (ChrgS:
  nine cards; the invisibility, the B button's Sword and the triple buster: two each). Most are three
  lines over a shared `weapon.load_chip(navi, chip)` (the Luau face of `sub_80126E4`, which chip_use.rs already
  has), with their charge rows. compat/weapons.toml gets their numbers.
- **Actions 0x5C, 0x5D, 0x5E** and **instant effect 20** as content under lib/patch-cards/ (or the owning card's
  folder when one card uses them), with compat/actions.toml entries; **the first barrier** hook takes the type.
- **Names** (§6): the cards' keys and display names from the fan translation.
- The live frontend gets a `--cards` option per player so the feature can be played. Netplay's stand-in setup
  stays without cards until netplay exchanges loadouts.

What stays out: the card menus, the MB limit (kept as data on the card for a loadout UI), NaviStats+0x4C (overworld
only), the e-Reader transfer.

### 4.3 Docs

docs/engine/patch-cards.md: the original's routines (§1-§3 of this document, with the slot table), cited by the
content. docs/engine/unverified.md rows for what no recording reaches (BugStop, effect 0x8F, GigaFolder−, the
first-application NumbrOpn quirk, the unused B-button RflectR and the 0x56 charged shot). jp-differences.md §8 #17
updated.

## 5. Verification

### 5.1 Recording cards

- **chiplab**: a `cards = ["canodumb", ...]` field per side (names through compat/cards.toml; `"-name"` for a
  switched-off card), written into the save's block (count, list, MB) with Tango's `PatchCard56sViewMut`. The
  bases are `jp-falzar` and `jp-gregar`. For SearchMan's ChpShufl the stale flag of §1.2 step 2 is written too,
  so the save is as one the game has applied before.
- **oracle-trace** (both consoles of a JP pair):
  - the setup records each side's installed cards (`patch_cards`);
  - a trap at the apply routine's entry (JP Falzar 0x08142230, JP Gregar 0x08143FF8) keeps the NaviStats as they
    are before the cards, the last call before the battle (`navi_stats_before_cards`);
  - `emotion_window_glitch` reads flag 0x1723 when the console has cards installed.
- **bn6-compat's harness**: a trace with cards builds the setup from the stats before the cards plus the cards; it
  checks that the engine's applied stats equal the init exchange's NaviStats (a setup check before frame 0), then
  runs the trace as usual. A trace without them is as today.
- **gen-content check**: each card definition against the ROM's card (MB, effects lowered to id and parameter),
  and the new weapons' compat numbers and charge rows against the ROM.

### 5.2 Scenarios

About 210, all on Japanese consoles (side 0 traced, no coverage), under library/jp/cards/:

- **Every card, once** (117): side 0 with the card, side 1 without, a short battle that fires its weapon or uses
  its ability where it has one. The setup check covers the application; the frames cover the battle.
- **Combinations**: ordering (two charged-shot cards, the later wins), clamps (Attack to 9, HP percentages
  stacked), fixes over NaviCust bugs, a switched-off card, a link navi with cards (no effect), the glitch with all
  cards switched off, EXE6 Gregar.
- **Behaviour** where a card's effect needs more than one shot: each new weapon hitting, missing, countered,
  against a guard; the first barriers taking hits; each body against its weakness; the bug effects over a few
  turns.

The full lab and both golden traces still run at the end as usual: the US scenarios mustn't change.

## 6. Data outside the ROMs, and decisions

**Card data outside the ROMs: none for the battle.** Both JP ROMs hold all 117 cards (MB and effects), the effect
machinery and every routine a card reaches. The e-Reader card carries the card number into the save; nothing about
an effect.

**Names are the one thing.** The ROMs have Japanese names only. The user pointed at the fan translation (the
MMEXE6F/G IPS patches, with the idealexe English charset), which names every card: Canodumb, Amonicul, ColdBear,
... Cybeast Gregar, Bass Cross MegaMan. `tools/jp/patchcards.py` reads them from the patched ROM in memory.
Decisions:

1. **Use the translation's names** as display names, and keys made from them (`canodumb`, `cybeast-gregar`,
   `bass-cross-megaman`)? Two cards share one: センボン (22) and プクール (55) are both "Puffy". Proposed keys
   `puffy` (22) and `puffball` (55), or another name you prefer. The translation abbreviates to 8 characters
   (Amonicul, KnigtMan); proposed: keep its spelling, as the chip names do.
2. **Cards applied by the simulation from `PlayerSetup`** (§4.1), with `RoundSetup::navi_stats` meaning the
   stats before the cards, and the oracle recording those stats from a trap (§5.1). The alternative is to apply
   cards outside the simulation (a setup builder), keeping `navi_stats` as the exchange's: simpler for traces,
   but the cards wouldn't be part of the shared simulation as you asked.
3. **Out of scope**: the menus, the MB limit as a rule, NaviStats+0x4C, and the BugStop card's effect on the
   NaviCust's own bug compile (`sub_813C490`; no card has BugStop, and the engine doesn't compile the NaviCust).
4. **Unreachable branches ported anyway** (effects no card has: BugStop, 0x8F, GigaFolder−, the B-button RflectR,
   the 0x56 charged shot, the program names), marked unverified.

## 7. The work, in order

1. Engine: `PatchCardHandle`, `define.patch_card`, `PlayerSetup::patch_cards`, the setup-time NaviStats view for
   Luau, `Battle::new`'s application and the glitch rule; rules/patch-cards.luau with the effect kinds; tests on
   hand-made cards.
2. Content: the 117 cards (generated once from `patchcards.py json`, then committed and edited by hand), compat
   cards.toml; the 55 weapons with their charge rows and compat numbers; actions 0x5C/0x5D/0x5E; instant effect
   20; the first barrier types.
3. Verify: chiplab `cards`, oracle-trace's cards, stats-before-cards trap and glitch flag, bn6-compat's harness
   and setup check, gen-content check for cards; the scenarios; record, add to the lab, fix differences.
4. Docs: docs/engine/patch-cards.md, unverified.md, jp-differences.md; the frontend's `--cards`.

Rough size: the engine plumbing is small (a few hundred lines); the content is about 117 short card files and 55
short weapon files, plus four small routines; the verify side is a few hundred lines; the scenarios and fixing
what they find is most of the time.

## Appendix A. The cards

Number, the ROM's name, the translation's name, MB, effects (the effect ids' order on the card, which isn't the
order they apply in). "(bug)" is the card's own marking.

| # | Japanese | English | MB | Effects |
|---|---|---|---|---|
| 1 | キャノーダム | Canodumb | 10 | HP-40 (bug); Attack+1; charge: M-Cannon |
| 2 | アモナキュール | Amonicul | 16 | HP+80; Aqua body; Attack-1 (bug); first barrier: BblWrap |
| 3 | コルドベア | ColdBear | 18 | HP+140; Aqua body; charge: IceCube; panel trail 0x32 (bug) |
| 4 | ジーラ | Miney | 12 | no AirShoes (bug); no FloatShoes (bug); Attack+3 |
| 5 | メガリア | Megalian | 20 | HP-80 (bug); AirShoes; Charge+4; first barrier: Barr100 |
| 6 | メテファイア | MettFire | 20 | Fire body; B: meteors; step bug 1 (bug) |
| 7 | キルプラント | KilPlant | 19 | Wood body; no AirShoes (bug); charge: GrasSeed |
| 8 | ダークシャドー | Shadow | 6 | no StatusGuard (bug); B: Sword; charge: Invis |
| 9 | ボンビートル | Beetle | 11 | HP+100; B: MiniBomb; charge: ChrgS (bug) |
| 10 | ヘビーアレイ | Heavy | 15 | SuperArmor; Attack+1; no buster program (bug) |
| 11 | アゾマータ | Viney | 24 | HP-20% (bug); Wood body; StatusGuard; B+Back: none (bug) |
| 12 | ジェライム | Slimey | 12 | HP+30; Aqua body; charge: PanlGrab; normal gauge (bug) |
| 13 | ボルカノ | Volcano | 10 | Fire body; Charge+4; charge: BlkBomb; emotion bug 1 (bug) |
| 14 | ナンバーズ | Number | 17 | charge: ChrgS (bug); Custom+1; MegaFolder+1; GigaFolder+1; custom HP drain +2 (bug) |
| 15 | マグテクト | MagTect | 15 | HP-5% (bug); Elec body; FloatShoes; charged shot: pull |
| 16 | クーモス | Spidy | 8 | HP+60; Wood body; panel trail 0x42 (bug) |
| 17 | ボムボーイ | BomBoy | 14 | buster: push 1; charged shot: plain (bug); step bug 0; panel trail 0x90 |
| 18 | ウドノート | WuNote | 13 | HP+90; Wood body; buster: grass; hit bug 0 |
| 19 | ピカラー | Flashy | 11 | HP+5%; Elec body; Attack-1 (bug); custom damage 0 |
| 20 | エレオーガ | Eleogre | 16 | HP+50; Elec body; emotion bug 0; status bug 0 |
| 21 | ダルスト | OldStov | 12 | HP+130; Fire body; charge: FireBrn1; HP drain +1 (bug) |
| 22 | センボン | Puffy | 11 | Aqua body; Rapid+4; buster blanks 0x0 |
| 23 | ヒトデスタ | Starfish | 15 | Aqua body; charged shot: bubble |
| 24 | グラサン | BigHat | 8 | no SuperArmor (bug); charge: FlshBom1 |
| 25 | カカジー | ScarCrow | 14 | Elec body; charge: DolThdr1; ChipRecovery 30; buster blanks 0x42 (bug) |
| 26 | ゼロプレーン | FgtrPlne | 20 | HP-10% (bug); FloatShoes; AirShoes |
| 27 | レムゴン | Cragger | 19 | HP+200; UnderShirt; hit bug 3 (bug) |
| 28 | アルマン | Armadill | 12 | HP+5%; Charge-3 (bug); charged shot: break |
| 29 | ヤカーン | Kettle | 15 | charge: Spreadr1; B+Back: Shield; ChipRecovery 20; custom damage 40 (bug); HP drain +0 |
| 30 | ツボリュウ | ErthDrgn | 20 | HP+180; charge: ChrgS (bug); MegaFolder+1; status bug 1 (bug) |
| 31 | ナンバーマン | NumbrMan | 35 | HP-20% (bug); Custom+3; MegaFolder+1; GigaFolder+1 |
| 32 | アイスマン | IceMan | 25 | HP+80; Aqua body; no SuperArmor (bug); charge: IceSeed; custom HP drain +0 |
| 33 | スカルマン | SkullMan | 30 | HP+100; charged shot: HP bug; HP drain +1 (bug); status bug 1 (bug) |
| 34 | シャドーマン | ShadoMan | 38 | HP-50 (bug); FloatShoes; charge: LongSwrd; B+Back: AntiDmg |
| 35 | カットマン | CutMan | 32 | HP-100 (bug); Attack+4; charge: YoYo |
| 36 | ナイトマン | KnigtMan | 45 | HP+20%; SuperArmor; UnderShirt; Attack×2; buster blanks 0x41 (bug) |
| 37 | トードマン | ToadMan | 34 | Aqua body; Attack-1 (bug); Charge-3 (bug); charged shot: paralyze; buster blanks 0x0 |
| 38 | マグネットマン | MagntMan | 37 | HP+240; Elec body; charged shot: plain (bug); B+Back: immobilize |
| 39 | プラネットマン | PlanetMn | 40 | HP+300; Wood body; StatusGuard; no AirShoes (bug); B: CrakShot |
| 40 | ビーストマン | BeastMan | 33 | UnderShirt; Rapid+4; charge: StepSwrd; Rush; step bug 1 (bug) |
| 41 | デザートマン | DesertMn | 36 | HP+160; no AirShoes (bug); Charge+4; hit bug 0 |
| 42 | ヤマトマン | YamatoMn | 35 | HP+10%; Attack+1; B+Back: Reflect; custom HP drain +2 (bug) |
| 43 | ビデオマン | VideoMan | 32 | HP-10% (bug); charged shot: road left; fast gauge |
| 44 | バーナーマン | BurnrMan | 29 | HP+120; Fire body; Attack+2; custom damage 40 (bug); emotion bug 1 (bug) |
| 45 | スターマン | StarMan | 32 | Attack-2 (bug); MegaFolder+1; panel trail 0x90; emotion bug 0; HP drain +0 |
| 46 | ブリザードマン | BlizMan | 30 | HP+60; Aqua body; no FloatShoes (bug); charge: BlzrdBal |
| 47 | スワローマン | LarkMan | 36 | AirShoes; charged shot: chip break; Beat; hit bug 2 (bug) |
| 48 | スラッシュマン | SlashMan | 31 | Attack+4; charge: WideSwrd; buster blanks 0x41 (bug) |
| 49 | キラーマン | EraseMan | 40 | HP-10% (bug); charged shot: erase; MegaFolder+1; HP drain +1 (bug) |
| 50 | グランドマン | GrndMan | 43 | HP+25%; charge: DrilArm; Custom-1 (bug); MegaFolder-1 (bug) |
| 51 | ダストマン | DustMan | 37 | HP+140; buster: crack; B+Back: fan; panel trail 0x1 (bug) |
| 52 | ブラストマン | BlastMan | 28 | HP-80 (bug); Fire body; FloatShoes; Attack+1; panel trail 0x90 |
| 53 | サーカスマン | CrcusMan | 43 | HP-200 (bug); first barrier: none (bug); GigaFolder+1; Tango |
| 54 | ハンディース | Handy | 16 | HP+100; charge: TimeBom1; ChipRecovery 0 (bug) |
| 55 | プクール | Puffy | 12 | Aqua body; FloatShoes; charge: bubble spread; B+Back: none (bug) |
| 56 | ジェリー | Jelly | 13 | HP+120; Aqua body; Charge+4; slow gauge (bug) |
| 57 | ポイットン | Poitton | 13 | HP-40 (bug); Wood body; UnderShirt; first barrier: Barrier; hit bug 0 |
| 58 | サテラ | Satella | 8 | AirShoes; Rapid+4; custom damage 20 (bug) |
| 59 | パララ& リモコゴロー | Twisty | 9 | HP+80; Elec body; no AirShoes (bug); buster: crack; panel trail 0x11 (bug) |
| 60 | ユラ | Sparky | 8 | HP-5% (bug); Elec body; FloatShoes; step bug 0; status bug 0 |
| 61 | タコバル | Octon | 14 | HP+50; Aqua body; Attack+3; buster blanks 0x41 (bug) |
| 62 | シェルキー | ShelGeek | 8 | Aqua body; Rapid+4; B+Back: Shield |
| 63 | マグニッカー | Magneakr | 7 | HP+30; Elec body; no buster program (bug); charged shot: road right |
| 64 | メテマージ | Metrid | 15 | HP+5%; Fire body; B+Back: meteors; step bug 1 (bug) |
| 65 | モモグラン | Momogra | 6 | HP-70 (bug); charge: Invis; Rush |
| 66 | ニドキャスター | Needler | 10 | SuperArmor; Attack+1; Rapid+1; Charge+1; emotion bug 0 |
| 67 | ツインズ | Twins | 15 | HP+100; Attack+1; charged shot: plain (bug); ChipRecovery 20 |
| 68 | ウォーラ | Walla | 12 | HP+60; first barrier: none (bug); Custom+1 |
| 69 | キルブー | Kilby | 24 | HP-20% (bug); Wood body; StatusGuard; charge: Lance; panel trail 0x42 (bug) |
| 70 | トトポール | Totem | 13 | Fire body; ChipRecovery 40; MegaFolder+1; custom HP drain +1 (bug) |
| 71 | キャノガード | CanGuard | 10 | no FloatShoes (bug); charge: Cannon; B+Back: Shield |
| 72 | スカラビア | Skarab | 7 | HP-70 (bug); charged shot: confuse; custom HP drain +0 |
| 73 | ドラグリン | Draggin | 11 | HP+5%; Fire body; status bug 1 (bug) |
| 74 | マリーナ | Marina | 14 | HP+90; Aqua body; no SuperArmor (bug); first barrier: BblWrap |
| 75 | ドルダーラ | HntdCndl | 12 | HP+130; Fire body; charge: ChrgS (bug); HP drain +0 |
| 76 | ガンナー | Gunner | 11 | HP+70; no UnderShirt (bug); charge: MachGun1 |
| 77 | パルフォロン | PulsBulb | 13 | Elec body; charge: ElcPuls1; emotion bug 1 (bug) |
| 78 | ボムコーン | BombCorn | 16 | HP+110; Wood body; charge: CornSht1; hit bug 2 (bug) |
| 79 | モリキュー | Shrubby | 6 | Wood body; buster: grass; charge: ChrgS (bug); custom damage 0 |
| 80 | ハニホー | HonyBomr | 12 | HP-10% (bug); Wood body; AirShoes; B+Back: RskyHny |
| 81 | ナイトメア | NghtMare | 5 | HP+20; emotion bug 1 (bug); status bug 1 (bug) |
| 82 | スナーム | SnakeArm | 11 | Attack+3; Rapid-1 (bug); Charge-1 (bug); charge: SandWrm1 |
| 83 | アサシンメカ | DarkMech | 22 | HP+10%; B+Back: MchnSwrd; HP drain +2 (bug) |
| 84 | ストーンマン | StoneMan | 45 | HP+360; SuperArmor; panel trail 0x1 (bug) |
| 85 | カラードマン | ColorMan | 32 | HP+210; charged shot: confuse; B+Back: none (bug); status bug 1 (bug) |
| 86 | シャークマン | SharkMan | 35 | HP+180; Aqua body; Attack+2; Rapid+2; Charge+2 |
| 87 | ファラオマン | PharoMan | 35 | HP+5%; FloatShoes; charge: ChrgS (bug); GigaFolder+1 |
| 88 | エアーマン | AirMan | 34 | HP+150; buster: push to edge; charge: Tornado; first barrier: none (bug); buster blanks 0x0 |
| 89 | フリーズマン | FreezeMn | 30 | HP-5% (bug); Aqua body; buster: ice; first barrier: Barr200 |
| 90 | サンダーマン | ThunMan | 36 | Elec body; charge: ElecSwrd; ChipRecovery 10; Custom-1 (bug); HP drain +1 (bug) |
| 91 | ナパームマン | NapalmMn | 39 | Fire body; Attack+1; Rapid+4; charge: BigBomb; custom HP drain +1 (bug) |
| 92 | プラントマン | PlantMan | 42 | HP-80 (bug); Wood body; StatusGuard; charged shot: grass; MegaFolder+2 |
| 93 | ミストマン | MistMan | 37 | HP+260; buster: poison; ChipRecovery 0 (bug) |
| 94 | ボウルマン | BowlMan | 30 | HP+100; no FloatShoes (bug); charge: Vulcan3; Custom+1 |
| 95 | ダークマン | DarkMan | 25 | charge: VDoll; step bug 1 (bug); custom damage 50 (bug) |
| 96 | タップマン | TopMan | 28 | HP+10%; charge: ChrgS (bug); buster blanks 0x2; hit bug 3 (bug) |
| 97 | ケンドーマン | KendoMan | 38 | HP+20%; charge: BambSwrd; buster blanks 0x41 (bug) |
| 98 | コールドマン | ColdMan | 34 | Aqua body; Attack-1 (bug); charged shot: freeze; panel trail 0x31 (bug) |
| 99 | サーチマン | SearchMn | 37 | HP+5%; no AirShoes (bug); charge: CircGun; ChpShufl |
| 100 | クラウドマン | CloudMan | 40 | HP+230; Elec body; charge: Thunder; MegaFolder-1 (bug) |
| 101 | フットマン | GridMan | 48 | HP+25%; SuperArmor; UnderShirt; slow gauge (bug) |
| 102 | チャージマン | ChrgeMan | 31 | Attack+3; Charge+4; charge: ChrgS (bug); panel trail 0x90 |
| 103 | テングマン | TenguMan | 34 | AirShoes; Attack-1 (bug); charge: WindRack; Beat |
| 104 | ダイブマン | DiveMan | 42 | HP+320; Aqua body; charge: WideSht; Custom-2 (bug); MegaFolder+1 |
| 105 | ジャッジマン | JudgeMan | 38 | HP-10% (bug); charge: GrabBnsh; normal gauge (bug); Custom+2 |
| 106 | エレメントマン | ElmntMan | 33 | HP+150; buster: grass; charged shot: ice; charge: ChrgS (bug); panel trail 0x90 |
| 107 | パンク | Punk | 50 | HP+400; SuperArmor; charged shot: confuse; B+Back: Reflect |
| 108 | ダークロックマン | Dark MegaMan | 80 | Attack+3; buster: poison; charge: DrkSword; emotion bug 1 (bug); HP drain +4 (bug) |
| 109 | ソウルバトラーの カスタマイズ | Soul Battlr's Custom | 66 | SuperArmor; Attack+4; Rapid+4; Charge+4; charge: SuprVulc |
| 110 | 名人のちょうぜつ カスタマイズ | Famous' Custom | 69 | first barrier: LifeAura; Custom+3; MegaFolder+2; GigaFolder+2 |
| 111 | フォルテBX | BassBX | 70 | SuperArmor; FloatShoes; AirShoes; B: triple buster; charge: nine-shot buster |
| 112 | ジャンゴ | Django | 52 | HP+200; StatusGuard; UnderShirt; charge: GunDelS1 |
| 113 | ハクシャク | Count | 60 | charge: sand storm; MegaFolder+1; status bug 2 (bug) |
| 114 | ロックマンゼロ | MegaMan- Zero | 45 | B: Sword; charge: Z-Saver; fast gauge |
| 115 | 電脳獣グレイガ | Cybeast Gregar | 70 | HP+600; SuperArmor; Attack+4; Rapid+4; custom HP drain +2 (bug) |
| 116 | 電脳獣ファルザー | Cybeast Falzar | 70 | HP+40%; FloatShoes; AirShoes; HP drain +1 (bug) |
| 117 | フォルテクロス ロックマン | Bass- Cross MegaMan | 70 | HP+20%; FloatShoes; B: triple buster; charge: spinning top; Null body; MegaFolder-2 (bug) |
