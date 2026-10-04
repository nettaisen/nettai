# The NaviCust in content

**Status: built (2026-10-02).** A player's NaviCust is part of their setup: the programs they placed on its grid.
BN6's rules compile it into the navi's stats as the round is set up, and the compile matches the original's on every
recorded case. This note covers what is the engine's, what is content, and how BN6's compile works, routine by
routine. It also covers how BN5 or BN4 would slot in, how the compile is verified, and what no recording reaches.

The request came from the match editor ("it configures e.g. ruleset, arena, chips, navicust, patch cards"). A match
file gives each side its NaviCust (docs/frontend.md §6), and nettai-editor edits it as the game does.

## 1. The model

| What | Whose | Where |
|---|---|---|
| A program: its colors, shape, compressed shape, whether it is a plus part | the engine's record of every game's program | `define.navicust_program`, `Registry::NaviCustProgram`, `NaviCustProgramDef` |
| What a program does, which bug it brings, which programs it excludes | the game's rules' data on the definition | BN6: `effects`, `bug`, `exclusive` (and `anywhere`), read by rules/navicust |
| A player's NaviCust: the programs placed (program, color, center, quarter turns, compressed) and the board's expansions | the player's setup | `PlayerSetup::navicust: Option<NaviCust>` (`crate::navicust`) |
| The board: which cells a program may cover, its frame, the command line | the game's rule section | the stock ruleset's `navicust` (rules/navicust/section.luau), `Rules::navicust` (`NaviCustRules`) |
| The compile: placement into stats and bugs | the game's rules | BN6's `navicust` system (`round_setup`) |
| A program's name | the locales | `[navicust-programs]` in locales/<lang>.toml |
| A program's number (a save's part id is 4 × it + the color variant) | compat | content/bn6/compat/navicust.toml |

So the engine knows a NaviCust's geometry, which is generic: a 7x7 grid (`navicust::SIZE`), a program's shape
centered on its middle cell, quarter turns as BN6's four copies make them (`navicust::rotate`), and boards of `o`
(board), `f` (frame) and `.` (no cell) cells. It also knows the one placement rule every NaviCust game shares,
`NaviCustRules::fits` (BN6's `sub_813BB00`: each covered cell is a board or frame cell, and not all of them are
frame). It knows nothing of what a program does. The definition's other fields are the game's, which its rules
read through `battle.navicust(side)`. A game without a NaviCust has no `navicust` section and no system to compile
one.

**None means none.** `PlayerSetup::navicust` is `None` when the setup's stats are already the NaviCust's: a
recording's, or a match file's stats block. The system then does nothing, so the golden traces and the lab are
unchanged. `Some` means the stats are the navi's before the NaviCust: its fresh stats with what the save keeps,
which is what the original's reset leaves (§3, step 3). A match side with a NaviCust starts from exactly that
(nettai-match's `Side::base_stats`, `stats::SAVE_FIELDS`).

**The NaviCust is MegaMan's.** The original compiles the PET's own navi's NaviCust, navi 0's. The system compiles
a NaviCust only for the navi that changes form, and a match refuses one for another navi.

## 2. BN6's programs as content

`content/bn6/navicust/<key>/program.luau` holds 46 definitions, one per program. The verification workspace's
`tools/navicust/gen.py` writes them, with their names and numbers, from the ROM's part table (`StructArr_813944C`,
16 bytes a part id). Its `check` mode compares the committed files with the ROM. The four ROMs' tables are the same,
byte for byte, so the programs have no version or region differences.

```luau
return define.navicust_program {
    id = "hp-50",
    colors = { "white", "pink", "blue" },  -- part ids 4n, 4n+1, 4n+2; 4n+3 has no color
    plus = true,                           -- +1: 1
    shape = { ".......", ".......", "...#...", "...#...", ".......", ".......", "......." },
    bug = "hp",                            -- +4: bug group 9
    effects = { programs.hp(50) },         -- navicust_jt_NCPs[41]
}
```

- **Colors** are by name: `white`, `yellow`, `pink`, `red`, `blue`, `green` are colors 1 to 6. A placed program's
  color is an index into its definition's list, which is the variants' order.
- **The table's bytes** are named by what they do. +0 is the exclusive group: `super-armor`, `guard` (Shield,
  Reflect, AntiDmg), `encounter-element` (OilBody, Fish, Battery, Jungle), `l-button` (Humor, Poem) and
  `custom-screen` (ChpShufl, NumbrOpn). +1 is 0 for a program, 1 for a plus part, and 2 for one that works anywhere
  (no BN6 part is one). +4 is the bug the program brings, one of rules/navicust/programs.luau's `BUGS`. +2 is 1 for
  every part, and nothing reads it.
- **The effects** are rules/navicust/programs.luau's constructors, one per handler of `navicust_jt_NCPs` (by
  `id >> 2`). A handler that calls others (BustPack, BodyPack, FldrPak1, FldrPak2) lists their effects in its
  order. Weapons and barriers are named by definition: Shield's is `require("navis/megaman/weapons/shield/weapon")`,
  FstBarr's is lib/barriers' `barrier_10`. A program whose handler writes a stat no netbattle reads (SneakRun,
  OilBody, Fish, Battery, Jungle, Millions, Humor, Poem, SlipRunr, AutoHeal) has `programs.outside(...)`, which
  the system does nothing for, because the engine doesn't keep the stat.
- **Compressed shapes** are given where they differ from the shape. Every BN6 program has one.

## 3. BN6's compile (rules/navicust/system.luau)

The original's `reloadCurNaviStatBoosts` calls `sub_813C458` when the PET's navi is navi 0. The system's
`round_setup` hook is that routine. The stock ruleset runs it before the patch cards, which apply to what it made
(docs/engine/patch-cards.md §1.2). It works in six steps.

1. **The grid.** Each program covers its shape's cells, centered on its place (`sub_813B950`; a later program
   overwrites an earlier one's cell). A program is compressed by its part id's event flag, 0x2660 + the id. A save
   keeps it per program and color, so a match file's copies of one program in one color must agree.
2. **The bugs counted** (`sub_813BBD4`), each counted for a program's `bug`:
   - **The command line** (row 3), right to left (`sub_813BC1C`). Each program on it counts once, at its rightmost
     cell. A plus part there counts for its bug and still works. A program there works. An `anywhere` program works
     with no bug.
   - **Off the command line** (`sub_813BC98`). A program there counts for its bug and doesn't work. A plus part works.
   - **Beside one of its color** (`sub_813BD24`). Each program's shape is moved a cell left, right, up and down. Each
     other program it meets counts once, but only on a cell where a neighbor counts (`byte_813C640`: not the grid's
     top row or outer columns; its bottom row does count). A neighbor of the same color counts for **the
     neighbor's** bug (`sub_813BE38` reads the bug group of the program it found). So two touching programs of
     one color bug each other's groups.
   - **The colors** (`sub_813BEA8`): five distinct colors count once for `status`; six or more count twice for
     `status-strong`.
   - **On the frame** (`sub_813C584`): a program covering a frame cell of the board counts for its bug, once.
3. **The stats reset** (`sub_8136C24`). The original resets the stats to navi 0's fresh row (`init_8013B4E`). It keeps
   the mood, the Beast Out counter, the sun, the base HP, the form, the folder and its Regular and tag chips, the
   Regular memory and the HP. The setup's stats are already that (§1).
4. **The effects** (`sub_813C684`), in three passes:
   - the programs on the command line, right to left; of an exclusive group only the rightmost works (`byte_2006DD8`);
   - the plus parts off it, in the list's order;
   - the plus parts on it, left to right.
   
   Custom levels clamp at 8, Mega and Giga levels at 10, and the buster's levels at 4. The HP programs add up into
   the maximum on top of the base (`sub_803CED4`). The original's check that the folder's Regular chip still fits
   the Regular memory (`sub_813CEA0`) is the folder rules' (`folder_check`), which read the stats this made.
5. **The bugs** (`sub_813CBCC`). BugStop (+0x1F, `sub_813C490`) clears them all. Otherwise each bug runs at its
   count's level, 1 to 3 (`byte_813CC18`):

   | Bug | What it writes |
   |---|---|
   | `step` | steps go astray (+0x31 = 1) |
   | `emotion` | the emotion swings (+0x24) |
   | `panel` | a cracked trail (+0x12 = 3) at level 2 to 4 (+0x13) |
   | `custom` | the hand shrinks from turn 4, 3 or 2 (+0x63) |
   | `encounter` | encounters (+0x28 = 1) |
   | `result` | chip drops (+0x26 = 1) |
   | `buster` | 6, 10 or 13 blank shots of 16 (+0x14), and 1, 2 or 3 charged (+0x15) |
   | `support` | no supports (+0x0D = 0xFF) |
   | `hp` | the HP drain 1 to 3 more (+0x18), and hits give the HP bug (+0x16 = 3) |
   | `unread` | +0x62 (nothing reads it) |
   | `status` | a status at the battle's start (+0x1A = 9) |
   | `status-strong` | a stronger one (+0x1A = 10) |

   Any bug sets the save's flag 0x1720, the emotion window's glitch (`battle.set_emotion_window_glitch`). With
   patch cards installed, the cards' system then sets the flag the Japanese console reads instead (0x1723).
6. **The HP** (`sub_803CE44`). The maximum is the base plus the HP programs, and the round starts at the maximum, as
   it does in the real world.

The battle's start sets two more stats that the reload doesn't, whatever the save says: the navi's game (+0x20) and
MegaMan's variant (+0x2B, which picks his move lag), which is his base HP in hundreds (`sub_800A2F8`). A match's
setup gives both (nettai-match's `starting`).

## 4. Other games

A game brings its own programs (its root's `define.navicust_program`s, with its own names in its own colors), its
own board section and its own compile system. **BN5's is built** (bn5-map.md §15.13): its compile is BN6's routine
for routine, so the routines are shared (content/exelib/navicust/compile.luau, `compile.run(side, game)`), and each
game's navicust system passes what is its own (`NaviCustGame`, content/exelib/types.d.luau): its board, its bugs in
the order its bugs' routine runs them and what each writes by level, what a placed program counts besides (BN5's
HubBatc counts its own bug once more), whether any bug sets the emotion window's glitch (BN6's flag 0x1720; BN5's
flag is read outside battle only), and whether the HP is left as it is (BN5's compile in the cyberworld, the
system's setup `cyberworld`). The programs' effects are shared constructors (@exelib/navicust/effects). BN5's board
is 5x5 with no frame, the middle of the engine's 7x7 grid. The engine's model, the match file and the editor take it
as they are. The editor draws whichever board the side's game's section gives, and lists the content's programs. BN4's NaviCust has two command lines and no plus parts. `NaviCustRules::command_line` is
one row, so BN4 would widen it to a set of rows. That is a change to the section, not to the model.

## 5. Verification

The verification workspace's `trace-tests --test navicust` compiles NaviCusts through the engine and compares the
result with the original's:

- **Tango's raw saves** (the canonical BN6 saves: US and JP, Falzar and Gregar) are compared on the save's NaviStats
  block and its flag. Their NaviCusts are empty, with 1000 HP and no bug.
- **The chip lab's scenarios that set a NaviCust** are compared on their recording's setup: every modeled stat byte,
  and the bug flag, with the patch cards on top where they are installed. That is 1,274 sides: 167 with programs,
  201 programs in all, and 112 with the bug flag. Between them they cover all 46 programs and every bug group a
  program brings, including the result and encounter bugs. They also cover five and six colors, BugStop, the
  frame, plus parts on the command line, programs off it, same-colored neighbors, two programs of one exclusive
  group on the command line, and compressed shapes. Every one matches. The library's `navicust-compile/` scenarios
  were written for the programs and bugs the others didn't place.

A scenario's program is compressed only if the save's flag says so, and the lab's base saves set every flag. So a
scenario places the compressed shape (`chiplab info` prints the flags). A program placed uncompressed on such a save
is compiled compressed by the original.

BN5's: `trace-tests --test bn5_navicust` compiles Tango's BN5 saves and the BN5 lab's scenarios the same way, in
every byte the engine's compile can write (bn5-map.md §15.13): 59 NaviCusts of 164 programs, all 47 programs, every
bug and level that changes a byte, uncompressed shapes (a scenario clearing the save's flags), turned ones, the
cyberworld's HP, and the finished saves' own NaviCusts. Every one matches.

## 6. Unverified (ported)

- A program that works anywhere (+1 = 2): no BN6 part is one.
- The `unread` bug (group 10): no part or color count brings it, and nothing reads +0x62.
- Bug groups 13 to 15: no part has one. The original's table would read past its end.
- The smaller boards (4x4, 5x4: key item 0x71 below 2). Every recorded save has the full board. A match file can
  name one (`expansions`).
- Overlapping programs: the editor and a match's checks refuse them, as `sub_813BB68` does when placing.
