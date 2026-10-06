# Patch cards (改造カード): the plan, and how it was built

**Status: done (2026-10-02).** Patch cards are an engine definition kind with a typed player setup, and EXE6's
patch-cards system (content/exe6/rules/patch_cards/) applies them,
all 117 cards are content (content/exe6/patch_cards/), and the chip lab's library/jp/cards/ (222 scenarios on Japanese
consoles) matches on every frame. docs/engine/patch-cards.md is the reference: how the Japanese games do it, what
the cards do, how nettai has it, and the cards (its appendix). This document keeps the plan's reasoning and the
decisions.

The user's request: "add bn6 jp patch card (kaizou card) support". The Japanese releases (JP Falzar, BR6J; JP
Gregar, BR5J) read e-Reader Modification Cards. A card changes MegaMan's stats and abilities, often with a bug
attached. The US release cut the feature.

## 1. The shape of it

A card is data: a number, an MB cost and up to six effects, each an effect id and a parameter. The Japanese reload
of MegaMan's stats (`reloadCurNaviStatBoosts`'s card part, 0x08142230) applies the installed cards to the stats
the NaviCust made, and nothing in the battle reads the cards themselves: a netbattle carries their effects in the
NaviStats the consoles exchange, and the console's emotion window reads one more flag (0x1723) when cards are
installed. So the work was:

- the cards as content, named, and the application as rules that run before anything reads the stats;
- the battle code the cards reach that no US data names (§2);
- recording cards: the stats before them, which a save no longer holds once applied.

## 2. What a battle with cards reached that the engine didn't have

The completeness audit (docs/engine/completeness.md) follows the weapon table through the numbers US data names,
so everything only a card names was out of scope. With cards it was reachable:

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
   were ported but no recording had run them.
4. **First barrier types 5, 7, 8, 9.** The engine's `first_barrier` hook raised the Barrier chip's barrier
   whatever the stat says (the US NaviCust only writes 1). `sub_8013892` passes the type to `sub_801A7CC` and to
   the visual.
5. **MegaMan's body element.** NaviStats+0x10 nonzero for MegaMan: his weakness and his element's interaction with
   panels and Crosses. The engine copies the stat to the navi at init; no recording had had it nonzero.
6. **Stat values the US never produces**: Attack 5 to 9, CustomLevel 8 from the stat, HP up to 9999, the hit
   status and HP drains from a card, ChipRecovery 0.
7. **The emotion window's glitch from cards** (docs/engine/patch-cards.md §1.3), which needs NaviStats +0x26 and
   +0x28 (two NaviCust bugs the engine didn't model; nothing in the battle uses them).

The engine also panicked on any weapon number compat didn't know (exe6-compat's codec), so a JP trace from a
console with a weapon card failed at setup.

All of it is content now (docs/engine/patch-cards.md §3).

## 3. The design as built

**The cards are the engine's; their effects are a game's rules'** (the user, 2026-10-02: "the engine should know
what a patch card is"). BN4, EXE5 (JP) and EXE6 (JP) all have patch cards, so a card and a player's installed cards
are engine concepts, as chips and folders are; what an effect does is each game's rule, a system of its stock
ruleset (docs/design/rules-in-luau.md §2.2): EXE6's is content/exe6/rules/patch_cards/init.luau, whose
`round_setup` hook applies them.

- **A definition kind of its own**: `define.patch_card { id, mb, effects }`, `Registry::PatchCard`,
  `PatchCardHandle`, `Content::patch_card(h)`; keys are plain names (`canodumb`), compat patch-cards.toml gives
  each its number.
- **The common record** (`PatchCardDef`), what every game's card has: its capacity cost (`mb`: EXE6's MB; a game
  without one gives 0) and its effects in the card's order, each a `kind` (a string the game's rules know) and
  whether the card shows it as a `bug` (a menu's red text). The kinds' own fields (an amount, a weapon, a
  variant...) stay the definition's data: the engine checks only that every effect has a kind, and the game's
  rules read the rest from the definition (EXE6's constructors in rules/patch_cards/cards.luau make them). The
  name is the locales'. The cards' numbers are compat's, as every number is.
- **Their names are the locales'** (content/exe6/locales/en.toml and ja.toml, `[patch-cards]` by card key,
  `PatchCardStrings`). The card weapons have no names: nothing shows a weapon's (text-rendering.md §10.2).
- **The setup is typed**: `PlayerSetup::patch_cards`, the card handles in the list's order with each switched on
  or off, at most 32 (EXE6's save list's room; its 80 MB allow 16). It is part of the setup the peers exchange and
  the digest covers. The EXE6 system reads it with `battle.patch_cards(side)`.
- **Before** (until the user's decision), the cards were records of type "patch-card" and the installed cards the
  system's own setup block (`record:patch-card[16]`, `bool[16]`), with the names in a `[records]` table.
- **The hook.** The cards must change the stats before the battle copies them: the navi's init reads them as it
  spawns, and the battle-start copy (`reserves`) is made with the battle. S0's `round_start` runs after the
  navis spawn, too late, so this work added `round_setup(side)`: once per side in `Battle::new`, before anything
  reads the side's stats, which a system may change through `battle.navi(side)`. After it the framework copies the
  stats to the battle-start copy. It is a hook like S0's (`SystemHook::RoundSetup`, called with
  `Battle::notify_systems`), and costs one call per side per round.
- **The glitch is pushed**, as rules-in-luau.md §2.1 rule 5 asks: with cards installed the system sets the
  console's emotion window glitch (`battle.set_emotion_window_glitch(side, on)`) from the stats after the cards.
- **`RoundSetup::navi_stats` is the stats before the cards.** A trace's setup gives both: the oracle traps the
  apply routine's entry for the stats before (`navi_stats_before_cards`), and the init exchange has the stats
  after. exe6-compat builds the setup from the first (the bytes the cards write; the rest from the exchange) and
  the card list, and checks the engine's result against the second and the glitch against flag 0x1723.
- **Tools write the setup**: exe6-compat's `codec::patch_cards` (a save's or a trace's list), the frontend's
  match file (each side's `cards`).

## 4. Verification as built

- **chiplab**: `cards = [1, -44, ...]` per side (numbers; negative: switched off), written into the save's block
  (count, list, MB) on the jp-falzar and jp-gregar bases. For ChpShufl the stale flag of the routine's step 2 is
  written too, so the save is as one the game has applied before.
- **oracle-trace**: the setup records each side's cards (`patch_cards`) and the stats at the apply routine's
  entry, the last call before the battle; the emotion window's glitch reads flag 0x1723 with cards installed.
- **gen-content check** lowers each card back to the card table (MB, every effect as id, parameter and bug through
  the ROM's lookup tables) and compares it with the Japanese ROM's card, and the new weapons' charge rows.
- **Scenarios** (library/jp/cards/): every card applied (117), every card weapon fired (71), and 34 combinations
  (docs/engine/patch-cards.md §6).

## 5. Decisions

1. **Names: the fan translation's.** The ROMs have Japanese names only. The cards' English names and keys come
   from the fan translation of the Japanese games: the MMEXE6F and MMEXE6G IPS patches over the Japanese ROMs, read with the
   idealexe English charset (the user gave the patches as MMEXE6F.ips and MMEXE6G.ips, and the charset as a
   manifest.toml). The translation's eight-character spellings stay (Amonicul, KnigtMan), as the chip names do.
   Two cards share "Puffy" there; the content calls センボン (22) Diodon (`diodon`) and プクール (55) Puffy (`puffy`).
   The Japanese names are the ROMs' (ja.toml).
2. **Applied by the simulation, from the setup** (§3), not by a setup builder outside it: the cards are part of
   the shared setup, and peers apply them alike.
3. **An EXE6 system in the stock ruleset** applies them (the coordinator, after rules S0 landed); `round_setup` was
   added for it. **The cards and a player's installed cards are the engine's** (the user, 2026-10-02), a definition
   kind and a typed setup field, as patch cards are in BN4, EXE5 and EXE6.
4. **Out of scope**: the card menus, the 80 MB limit as a rule (the MB is the card's data, for a loadout screen),
   NaviStats+0x4C (Bass BX's, read only by map scripts), and BugStop's effect on the NaviCust's own bug compile
   (`sub_813C490`: no card has BugStop, and the engine doesn't compile the NaviCust).
5. **Effects no card has are ported anyway** and marked unverified (docs/engine/patch-cards.md §7): BugStop, the
   charged shots' blanks, GigaFolder−, the B button's shield, RflectR and plain buster, the charged shot "none",
   and other values of the effects the cards have. Effects 0x9B to 0xA9 write slots nothing copies: no kind.
6. **Two effects of one kind on a card apply in the card's order**; the original orders them by effect id. No card
   has two of a kind with different choices, so nothing tells them apart.
7. **The HP.** The reload sets the HP to MaxHP in the real world and to the lesser of the two on the Internet; a
   netbattle's stats come from the real world, so the system sets HP = MaxHP.
8. **A link navi's glitch.** With a link navi operated the routine doesn't run, and flag 0x1723 stays as the last
   application with MegaMan left it, which the save holds. The system counts the link navi's stats' bugs instead:
   the same for a save whose flag is clear (the chip lab's); unverified otherwise.
