# Rust and Luau: where the line runs

The line between the engine's Rust and a game's Luau, as the user approved it on 2026-10-05 ("okay do that"). It
replaces nothing in [rules-in-luau.md](rules-in-luau.md) §2.1 (the layers, and which rules stay Rust for speed); it
says what each side may know. Every change from here on moves code toward it, and the crossings below are the ones
still to remove.

## The line

- **Rust is the machine, Luau is the game.** Rust runs objects, collision and hits, the panel grid, the custom
  screen's chip window, the flow, rollback and the digest, input, drawing and sound. What a game is (its forms, its
  custom-screen extras, its save, its NaviCust, its patch cards, its auto battle) is its Luau.
- **Rust never names a game or a game's feature**, only roles and schemas. It knows a fact by its role
  (`PlayerFact::Version`, `Level`), a definition by its registry and handle, a state or setup by the schema its
  content declares. It knows no "beast", no "souls", no "exe5".
- **Luau does no I/O and reads no clock.** Content gets what it reads from the engine's API: the battle's state, its
  own state and setup, its definitions. Nothing it runs can tell two machines apart, so both peers of a netbattle
  and every replay compute the same.
- **Rust calls Luau at defined points**: a hook at an event (a turn's start, a chip's use, a custom-screen request),
  and a state's tick while a state the rules set is active (a controller, a wrapper, a window). Never once per drawn
  frame: what the frontend draws, it reads from typed views of the state (`Battle::form_list`, `Battle::offer`).
- **Tools are generic over the schema.** A match file, the editor, a netplay offer and a match's description are
  made from the rules' setup schema: a game that declares another fact has it in all of them without a line of Rust.

## How a game's rules reach Rust

A game has one rules definition (its root's `rules`, rules/init.luau; [rules-in-luau.md](rules-in-luau.md) §2.5),
written by hand as a whole: a side's state and a player's setup stated in one place, each hook a plain function that
calls the game's modules (the save, Beast Out, Soul Unison, the NaviCust, ...: ordinary modules of functions) in the
order its code says. The engine calls a hook by its name and reads its answer, keeps one state block and one setup
block per side, and reads a setup field it needs by its role. Which module answers, and in what order they run, is
the rules' code.

## Known crossings

Where Rust still knows one game's thing. Each is scheduled or noted; none is to grow.

1. **The stat block's per-game fields** (`NaviStats`, crates/nettai-battle/src/setup.rs): `hub_style` (EXE5's Hub
   Style, +0x4C), `soul_turn_bonus` (EXE5's soul turns, +0x32), `beast_out_counter` (EXE6's), `chip_shuffle` and
   `number_open` (EXE6's NaviCust abilities), `version` (EXE6's version byte, +0x20), `chip_drops` and `encounters`.
   They mirror the original's block, which the compat crates and the traces compare; a game's own fields belong in
   its rules' state.
2. **A player's setup outside the rules' setup** (`PlayerSetup`): the patch cards and the NaviCust (step c3), the
   auto battle data (step c4), the folder's Regular chip and tag chips (step c3). Each becomes a fact of the rules'
   setup. (The SP navi deletion times did in step c2: each game's `sp_times`, a list of `{ chip, frames }`, which
   its rules/sp_chips reads for the SP chips' damage.)
3. **Engine routines that are one game's path**: the NaviCust's placement and board types (navicust.rs) and the patch
   cards' (patch_cards.rs) in nettai-battle, EXE5's auto battle data layout (auto_battle.rs: its places and
   records), and nettai-match's checks of them (`has_navicust`, `has_patch_cards`). Steps c3 and c4 move them into
   each game's Luau, with the checks a rules hook.
4. **Fact roles named for one game's feature**: `PlayerFact::BeastOut` and `PlayerFact::CrossList` (EXE6's), which the
   frontend reads for the emotion window and the Cross window; `PlayerFact::SpTimes` (both games' SP navi deletion
   times), which only tools read (nettai-match's `Facts::sp_times`, the editor's SP pane; the engine reads none).
5. **Rule sections named for one game's feature**: `berserk` (Beast Over's), `navicust`, the emotion section's Full
   Synchro aura. (`sp_chips` is no section since step c2: each game's rules/sp_chips is its own module.) They are schemas a game fills, but their names are a feature's.
