# The Japanese ROMs against the US: the code audit

The engine is built and verified against the US ROMs: Falzar (BR6E) and Gregar (BR5E). This document compares
their battle code with the Japanese EXE6 ROMs: Falzar (ROCKEXE6_RXXBR6J) and Gregar (ROCKEXE6_GXXBR5J). It
answers three questions:

- What differs, and in which class?
- Which JP-only objects exist, and what are they for?
- What would the engine, which runs the US behaviour, get wrong in a netbattle between Japanese consoles?

**The short answer.** 153 US routines are not byte-for-byte the same in JP Falzar (§2):

- 85 of them differ only in what moved: literal pools, padding, unread data.
- 27 differ in presentation only: graphics, HUD, text, buffers.
- 14 differ only in code no netbattle runs.
- 2 hold the JP-only content's entry points.
- 25 are US changes. A JP-console netbattle reaches 20 of them, which come to **15 behaviours**.

With one JP data difference (Otenko's statue as DustMan's junk) and the patch cards' emotion-window hook, §8
lists 17 variants with their sizes and how to switch them.

The JP-only content is:

- the Gregar and Falzar chips (handlers 34 and 35, five object kinds);
- Count (HackJack in the US release: handler 18, two object kinds);
- Django (handler 19, one object kind);
- patch-card hooks, out of scope.

The other JP-only objects belong to code no netbattle runs: battle mode 1, battle modes 10 and 11, the Count
boss, and one object that nothing spawns (§4).

JP Gregar is JP Falzar plus the Gregar/Falzar version differences the US ROMs already have (§1.4).

## 1. Method

### 1.1 Finding the routines

The verification workspace's `tools/gregar/fmap.py --to ROM` maps every US Falzar routine into a JP ROM. It
compares each routine byte for byte, with moved calls and code pointers masked. A routine is `same` there, or
`differs`, or `missing`.

That table alone over-reports. A dispatcher has the same shape as every other dispatcher, so with its pointers
masked it can match the wrong one, often a JP-only kind's. The table also guesses the address of a routine
that doesn't match. `tools/jp/blocks.py` repairs both:

- It anchors routines on the object kind tables and the chip handler tables. A routine's place is where the JP
  table points.
- It drops "twins": a `same` routine whose move disagrees with the median of its eight neighbours'.
- It compares **blocks**: a run of routines that aren't the same, between two that are. The two ROMs' spans
  between the same neighbours are compared, so code inserted or removed in the middle doesn't misalign the
  rest.

Each block is disassembled in both ROMs and normalized:

- literal-pool words come out of the instruction stream;
- calls and literals are named by the US symbol, the JP side mapped back through the routine map (or exactly
  through the pointer pairs of the same routines' pools, for data);
- branch targets inside the block are dropped.

What is left differs only where the code or a value it uses does.

### 1.2 What the masked comparison can't see

Three more checks look for differences the masked comparison hides:

- **Retargeted calls.** A call or code pointer in a `same` routine that goes to a different routine in JP.
- **Gaps.** Two `same` routines in a row whose distance differs (JP code inserted or US code removed), or whose
  in-between bytes differ (data between routines), with calls and pointers masked.
- **Data.** Each JP-only region is read with `jpcode.py`: what points into it, what it spawns, and who reaches
  it, followed up to code the US has (`chain`). Separate checks cover:
  - every chip record, field by field (`allchips.py`);
  - the 192 link battle settings records and their actor lists (`stages.py`);
  - the kind and handler tables (`kindtables.py`, `tables.py`);
  - the sprite list (`sprites.py`).

### 1.3 Classifying

Every routine in a battle block was read by hand from the diff, with the US disassembly, the engine's port and
the JP bytes. The verdicts are the verification workspace's `tools/jp/classes.tsv`, one line per routine.
`coverage.py` checks that it covers every block member. The classes:

| Class | Meaning |
|---|---|
| jp-content | JP-only content: patch cards, Count, Django, the Gregar and Falzar chips |
| us-change | the US behaves differently from JP |
| cosmetic | none of the battle's state: graphics, HUD, text, buffers outside the simulation |
| unreachable | only in code a netbattle never runs: battle modes other than 0, AI, single player |
| same | only moved addresses, a literal pool, padding or unread data |

"Battle block" means a block with at least one routine of the battle areas, by the completeness audit's classes
(completeness.md). Code the audit puts out of scope (navi AI, viruses, the overworld) was skipped unless it sits
in such a block or a battle routine reaches it.

### 1.4 JP Gregar

`blocks.py exe6_rom.srl` finds the JP Falzar blocks again, plus 14 more. `gregar.py` compares those 14 with the US
Gregar ROM between the same neighbours:

- 12 are US Gregar's code, compared as fmap.py compares. They are version differences: the encryption helpers,
  the default stats, custom-screen routines, the Beast's code, `sub_80F2290` and `sub_8120B54`.
- The other 2 mix version differences with JP Falzar's own: the init exchange's block, and the enemy navis'
  update.

So the JP Gregar ROM's JP differences are JP Falzar's, at its own addresses. `coverage.py exe6_rom.srl` checks
that too: 42 block members are version differences, 6 more have a verdict (5 same, 1 unreachable).

## 2. The counts

JP Falzar, 153 routines:

| Class | Routines | Reached by a JP-console netbattle |
|---|---|---|
| same | 85 | — |
| cosmetic | 27 | (presentation; §5 lists what a trace would see) |
| us-change | 25 | 20 (15 behaviours, §8) |
| unreachable | 14 | — |
| jp-content | 2 | 2 (the handler tables; the content itself is new code, §4) |

JP Gregar adds 6 (5 same, 1 unreachable) and 42 version differences.

About the scoping pass's "about 64 routines with instruction differences": that pass aligned the JP side by a
guessed address. Its large counts came from routines that are wholly US-only (`sub_801B878` 213, `sub_80169BE`
144) or from data aligned as code (`sub_80E9810` 181, `sub_8108F74` 114, `sub_802CCAE` 112). The routines it
couldn't place are all placed now. They are:

- US-only routines: `sub_80169BE`, `sub_801B878`, `sub_80033E4`, `sub_80C46C6`, `sub_80E060E`, `sub_80E0616`;
- routines whose literal pool moved;
- data tables decoded as code (the handler tables in `sub_802CCAE`).

## 3. The US changes

Each row is one routine. **Reach** says whether a netbattle between Japanese consoles reaches the difference.

| Routine | What differs | Reach |
|---|---|---|
| `sub_8011A7E` | the charged shot: the US makes a worn-out navi's damage 10 (attack level forced to 1 when `sub_8015B54` is 5); JP keeps (Attack + 1) × 10 | yes |
| `sub_8011C5E` | SpoutCross's charged shot: the US zeroes the attack bonus (AIAttackVars+6); JP keeps the last attack's | yes |
| `sub_8011CD6` | SlashCross's charged shot: the same | yes |
| `sub_8014CC0` | the end of a Cross: the US clears requests 0x80008600, JP 0x8600. A pending weakness hit's un-cross request (0x80000000, `request::WEAKNESS_HIT`) survives the change in JP | yes |
| `sub_80151D4` | the word in its literal pool that `sub_8014F04` (Beast Out's end) and `sub_8015128` (a Cross Beast's end) clear: the same 0x8600 | yes |
| `sub_80155CC` | the end of a Beast's Cross: the same. (Beast Over's end, `sub_80153B0`, clears 0x80008600 in both) | yes |
| `sub_8017A38` | the drag's recovery: the US also clears RskyHny's heat trap (status 0x200000); JP keeps it, so the navi goes on swallowing non-fire hits after a fire hit dragged it out of the stance. (JP's flag mask also lacks PARALYZED, which is clear on that path anyway) | yes |
| `sub_802CEF4` | the traps: see below | yes |
| `sub_801B878` | US only: LilBolr's obstacle dispatcher (while it erupts, a crushing touch is as any hit) | yes |
| `sub_80D774C` | LilBolr: JP calls the default dispatcher `sub_801B394` (a crushing touch drops the HP to 0), and doesn't set HP = MaxHP − (+0x2C & 0xFFF) while erupting | yes |
| `sub_80033E4` | US only: a T4 spawn at the head of the update list. fmap.py pairs it with JP's tail spawn, its twin | yes |
| `sub_80E360E` | ElemTrap's counterattack (T4 0x2B): the US spawns it at the head of the list, so it acts next tick; JP calls `object_spawnType4`, which links it right after the trap, so it acts in the same tick | yes |
| `sub_80BABAC` | ElmntMan: when the user doesn't pick an element in Param1·20 ticks, the US picks one (an RNG2 draw, `& 3`) and attacks; JP goes to action 0x18 and he leaves | yes |
| `sub_80C9F98` | Tornado on a volcano, grass or ice panel: the US doubles the damage but keeps the flags 0xF800; JP doubles the whole halfword, so 0x8000 is lost and 0x4000 becomes 0x8000 | yes |
| `sub_80D8C10` | AirSpin's top: the HP word in this routine's literal pool, which the top's init `sub_80D8908` reads, is 300 in JP (400) | yes |
| `sub_80DB684` | the dash hit (attack #0xAF): it rides 60 pixels per step of reach ahead of its owner in JP (58). Used by GroundCross Beast's dash and ChargeCross's tackle | yes |
| `sub_802A40C` | the hand size: see below | yes |
| `sub_80EF004` | GroundCross's drill: the US waits until the navi isn't moving or sliding (`sub_800E5FC`, flags 0x1040) before it burrows; JP burrows at once | yes |
| `sub_80F1694` | SpoutMan's DripShwr: JP clears MOVING (ObjectFlags1 0x40) and sets 0x80000 when he lands on the panel ahead; the US does it at the end (`sub_80F1756`), so MOVING stays set through the spray | yes |
| `sub_80F20A0` | DustMan's pull: its regions' Param4 (self collision row) is 0x04 in JP, an attack of its side; 0x1E in the US, a pull of no side (`collision.pull`) | yes |
| `sub_800F598` | the obstacle push: with none of the pusher bits (0xF3000000) the US doesn't push; JP pushes as if side 1 had. No recording starts a push without them; believed unreachable | no |
| `sub_801A77A` | the US skips flag2 0x20 and the 1200-tick timer for NameIDs 0x173..0x17E (story bosses); no netbattle object has one | no |
| `sub_80C464C`, `sub_80C46C6` | the form overlay held still while its owner is dragged or paralyzed, and that request's setter (US only); only AI code (`sub_810B30C`) sets it | no |
| `sub_8108F74` | a virus's update: the US also calls `sub_801A264` | no |

**The traps (`sub_802CEF4`).** The US checks RskyHny's heat trap first: with no fire damage, the hit is
swallowed, with the marker and sound 0x6E when it did other damage. JP checks the heat trap last, after:

- the side's linked AntiDmg, BodyGrd and AntiSwrd;
- the AntiDmg program's stance (status 0x800).

So with a trap linked during RskyHny's stance, JP's trap acts first. Two more differences:

- The heat trap doesn't swallow a 0-damage hit in JP.
- The armed stance (r6 = 1) lets a 0-damage hit through in JP; the US swallows it.

**The hand size (`sub_802A40C`).** JP computes:

```
n = min(CustomLevel + ChargeCross's charge, 8)
NumbrOpn (not DustCross): n = 10
hand-shrink bug: n = max(n − (turn − bug + 1), 2)
```

The US takes the bug's cut from the overflow above 8 (or NumbrOpn's charge) first. It differs from JP only when
that overflow is nonzero: CustomLevel plus ChargeCross's charge above 8, or NumbrOpn in ChargeCross. The engine
has the US form as `custom::screen::hand_size`.

## 4. JP-only content and the JP-only objects

### 4.1 The entry points

The chips' handler tables are in `sub_802CCAE`'s extent: `off_802CCB4` (dimming chips) and `off_802CD5C` (navi
chips). Every other entry is the same in both ROMs.

| Entry | Chips | US | JP Falzar | JP Gregar |
|---|---|---|---|---|
| dimming 34 | Gregar (0x138) | 0 | 0x080EDE3D (in T4 0x7A, +0x174) | 0x080EF16D |
| dimming 35 | Falzar (0x139) | 0 | 0x080EE087 (in T4 0x7B, +0x1FA) | 0x080EF3B7 |
| navi chip 18 | Count, Count[EX], Count[SP] (0x113..0x115; HackJack in the US release) | 0 | 0x080BD237 (in T1 0x11, +0x26A) | 0x080BEA97 |
| navi chip 19 | Django, Django2, Django3 (0x116..0x118) | 0 | 0x080BD6A3 (in T1 0x12, +0x2EA) | 0x080BEF03 |

`sub_813BF1C` is the other jp-content routine. JP also counts the emotion window's glitch as on when a JP-only
byte (0x020065F0) is nonzero. By its use that byte is patch-card state. Patch cards are out of scope;
**unverified**.

### 4.2 The kinds

Each row: the kind, its JP Falzar routine (JP Gregar), what spawns it, its sprite, and what it is for. Sprites
are given as category-index (`sprite_load(0x80, category, index)`).

| Kind | JP Falzar (Gregar) | Spawned by | Sprite | What it is |
|---|---|---|---|---|
| T1 0x11 | 0x080BCFCC (0x080BE82C) | navi chip 18 | 08-16, Count | **Count**, the Count chips' navi: drops his lances (T3 0x0D) |
| T1 0x12 | 0x080BD3B8 (0x080BEC18) | navi chip 19 | 0C-0F, Django | **Django**'s navi: one-shot effects, sounds 0xCB, 0x94, 0xB0 and 0x8B. The same archive is CrosOver's partner (§5) |
| T1 0x30 | 0x080C3E0C (0x080C566C) | T4 0x7A | 0C-68 | **Gregar**'s beast. Draws RNG2; spawns T4 0x7D; its attack is the US-present T3 0xCD (§4.3) |
| T1 0x31 | 0x080C4268 (0x080C5AC8) | T4 0x7B | 0C-66 | **Falzar**'s beast. Spawns T4 0x7D; its attack is the US-present T3 0xCE |
| T3 0x0C | 0x080C91E0 (0x080CAA40) | the Count boss's AI | none | the Count boss's lance rain: spawns T3 0x0D |
| T3 0x0D | 0x080C9498 (0x080CACF8) | T1 0x11, T3 0x0C | 08-16 | **Count's lance**: a hitting object, sound 0x82 |
| T3 0x13, 0x14, 0x15 | 0x080C9EBC, 0x080CA0C4, 0x080CA258 | the Count boss's AI (0x14 by 0x13) | their own | the Count boss's other attacks |
| T4 0x01 | 0x080E3E7C (0x080E51A8) | **nothing** | none | an affine screen-space effect in three variants. Its path comes from RNG1, and it draws through `sub_802FE48`/`sub_802FE7A`, as battle mode 1's HUD code does. Its only spawner (JP Falzar 0x080E414C, Gregar 0x080E5478) has no caller in either ROM: dead code |
| T4 0x0B | 0x080E5190 (0x080E64BC) | battle mode 1's per-frame code (0x0802E092) | none | battle mode 1's controller: sound 0xB2, effects; spawns T4 0x0D |
| T4 0x0D | 0x080E59F0 (0x080E6D1C) | T4 0x0B | its own | battle mode 1's attack: collision regions, sounds 0xF8, 0xB0, 0xCB |
| T4 0x0E | 0x080E5E34 (0x080E7160) | `sub_802D09E` | none | battle mode 1's: in JP, `sub_802D09E` spawns it before counting (`sub_800AB46`); its only caller is battle mode 1's hook in the enemy navis' update. The US routine keeps the register setup with no call, and nothing calls it |
| T4 0x17, 0x18 | 0x080E6AC0, 0x080E6B8C | the Count boss's AI | their own | the Count boss's effects |
| T4 0x7A | 0x080EDCC8 (0x080EEFF8) | dimming 34 | none | **the Gregar chip's controller**: the dimming (with `sub_800BCF6`), then T1 0x30 |
| T4 0x7B | 0x080EDE8C (0x080EF1BC) | dimming 35 | none | **the Falzar chip's controller**: draws RNG2; T1 0x31 and the Strike Feathers (US-present T3 0xBC) |
| T4 0x7D | 0x080EE300 (0x080EF630) | T1 0x30 and T1 0x31 | 0C-68 or 0C-66 by Param1 | the two beasts' shared overlay |
| T4 0x85 | 0x080EE9CC (0x080EFCFC) | `sub_8008064` in battle mode 10 | none | a scripted event when the 4th turn's fight starts: a fade, navi 0's HP to its maximum, a text box, sounds 0xCB, 0x6D, 0xD7 |
| T4 0x86 | 0x080EED48 (0x080F0078) | `sub_80080D2` in battle mode 11, when `sub_800A152` returns 1 | none | battle mode 11's scripted ending: sounds 0x94, 0x71, 0x72 |

The kinds the scoping pass left unnamed:

- T4 0x01: dead;
- T4 0x0B and 0x0E: battle mode 1;
- T4 0x85 and 0x86: battle modes 10 and 11;
- T3 0x0C: the Count boss's.

**The Count boss.** The enemy navis' tables (AI index rows of `sub_80F2354`'s `off_80F23AC`) have rows 17 and 22
pointing at row 0's in the US and at JP code in JP (0x08106CC8, 0x0810D584 in JP Falzar). Those rows spawn T3
0x0C, 0x13 and 0x15 and T4 0x17 and 0x18. T3 0x0C drops Count's lances (T3 0x0D), so both rows are Count, an
enemy navi the US cut. Enemy navis are out of a netbattle's scope.

### 4.3 What the JP chips use of the US ROM

`deps.py` lists the US routines each JP-only region calls or points at, with the completeness audit's class. All
of them are ported or are core routines, except four. The US ROM has them, but nothing in it reaches them, so
the audit put them out of scope and the engine doesn't have them:

| US routine | Used by | What it is |
|---|---|---|
| `sub_80DCC70` | T4 0x7B (Falzar's controller) | spawns T3 0xBC (`sub_80DCB1C`): Falzar's Strike Feathers |
| 0x080DF160 (after `sub_80DF132`) | T1 0x30 (Gregar's beast) | spawns T3 0xCD (`sub_80DF0A4`): Gregar's attack |
| `sub_80DF262` | T1 0x31 (Falzar's beast) | spawns T3 0xCE (`sub_80DF188`): Falzar's attack |
| `sub_800BCF6` | T4 0x7A and 0x7B (pointer, a dimming step) | a fade: `SetScreenFade(0x3C, 0x100)`, or `(0x84, 0x10)` when `sub_800B892` of the other side is 0 or 5 |

So the Gregar and Falzar chips' attack objects are in the US ROM already, orphaned. Their port belongs to the
chips. The Strike Feathers keep the controller's base damage and drop the chip's attack bonus. That is the JP
original's behaviour (bn6-lmao "fixes" it, §9).

### 4.4 Chip records

`allchips.py` compares bytes 0..0x1F of every chip record (the art pointers at 0x20..0x2B move with the ROM):

- **Django ×3 (0x116..0x118)**: class 3 (not a folder chip) in the US, 1 (Mega) in JP. Flags 0x00 in the US,
  0x47 in JP (dimming, damage, navi chip, library). Action 0x1B, subtype 0x13 and damage 130, 180, 260 in both.
- **GunDelEX (0x012)**: class 3 in the US, 0 (standard) in JP; flags 0x00 against 0x40 (library). Its
  behaviour (GunDelSol's subtype 3) is in both.
- **Count ×3 (0x113..0x115)**: flags 0x07 against 0x47; **Otenko (0x099)**: 0x01 against 0x41. Only the
  library bit 0x40 differs.
- Chips 0x119..0x121: flags2 (+0x16) 0x30 in the US, 0x20 or 0x00 in JP (menu classification only).
- 278 chips' sort keys (+0x18) differ: JP's alphabetical order. Menus only.
- Gregar, Falzar and DblBeast: the same record; only the art differs.

The art (icon, image, palette) is `chips.py`'s list:

- **Icons**: the same in all four ROMs, chip by chip; the pack has the US Falzar ROM's. (The Gregar ROMs' Falzar
  chip icon, JP's and US's alike, is a placeholder that reads "13A".)
- **Pictures**: the US ROMs have one placeholder picture (a purple block) for all eleven chips, the JP ROMs their
  art. The Gregar and Falzar chips share one picture in each JP ROM, its own beast (JP Falzar 0x08745658, JP Gregar
  0x0874358C), so a console shows its own beast in both.
- **Palettes**: DblBeast's, Gregar's and Falzar's records point at EWRAM, 0x02000AF0 (DblBeast, slot 0) and
  0x02000B10 (the console's beast, slot 1). A gift received over the link fills them: `0x0813006C` (JP Falzar)
  decompresses the chip's name to 0x02001180 + 24 × slot and its description to 0x020007D0 + 100 × slot, copies
  its 32-byte palette to 0x02000AF0 + 32 × slot and gives the chip; the dispatcher that calls it (0x0812FA84) sits
  on the link mode at 0x080407C4, and Tango's save code calls these buffers the e-Reader's (Card e+). The area is
  save data (the save is EWRAM 0x02000000..0x02006710, so save offsets 0xAF0 and 0xB10); a save that never took
  the gift has zeros there (a black picture), and the US games clear it at a new game and write "~~~~" names.
  DblBeast's card palette is also in both JP ROMs, orphaned among the chip palettes in DblBeast's place (JP Falzar
  0x08749C78, JP Gregar 0x08747BAC). Gregar's and Falzar's are in no ROM: the chips' definitions give them
  (`art_palette`, chips/gregar and chips/falzar), the colours Tango's netplay saves have at 0xB10
  (tango-gamesupport-bn6, saves/g_jp.raw and f_jp.raw). Rendered with them, the pictures are the JP consoles'
  exactly (bn6battle-verify's JP-console recordings of both chips).

`bn6-extract` takes the eleven pictures from the JP ROMs (GunDelEX, Otenko, Count (HackJack) ×3, Django ×3, DblBeast and
Falzar from JP Falzar, Gregar from JP Gregar), each chip's own on either console: a JP Falzar console's Gregar
chip shows Falzar's beast in the original, and a known difference in the frame comparison.

### 4.5 Sprites

The sprite list's slots that are a placeholder in the US and the JP ROM's own archive (`sprites.py`; the same
archive in both JP ROMs). `bn6-extract` takes these from JP Falzar, under the names in brackets:

| Slot | What it is |
|---|---|
| 08-16 | Count, HackJack's navi, and his lances (`count`) |
| 0C-0F | Django: the Django chip's navi and bike, CrosOver's partner and his gun (`django`) |
| 0C-49 | Otenko's statue (animation 0 the puff he appears in, 1 Otenko); also how it looks absorbed by DustCross and thrown as DustMan's junk (`otenko`) |
| 0C-66, 0C-68 | Falzar's and Gregar's beasts (`falzar-summon`, `gregar-summon`) |
| 14-17 | the effect table's entry 0x1D in JP: battle mode 1's red "ブロッキング" (Blocking) label, which no netbattle shows (`blocking-banner`) |
| 18-34, 18-35, 18-36 | not identified here (category 0x18 holds no battle sprites) |

`sprites.py` also lists 0C-0E, 0C-47, 0C-67, 10-16, 10-17 and 10-19, uncompressed slots whose fixed 0x2000-byte
window differs. Decoded as the extractor reads them, they are the same sprites: only the palette rows read past
the archive's own (the extractor keeps 16 rows where the data goes on, as the game reads whatever follows)
differ, because what follows moved. The pack keeps the US ROM's.

### 4.6 Where the JP-only code is

JP Falzar's JP-only stretches, by the blocks' and gaps' extents. JP Gregar's are at its own addresses: the kind
tables give their starts.

| JP Falzar | Bytes | Follows (US) | What it is |
|---|---|---|---|
| 0x080084F0..0x080086C4 | 420 | `sub_80084C0` | battle mode 1: its turn code |
| 0x080097EC..0x08009C04 | 1048 | `sub_80095C8` | battle mode 1: its handler |
| 0x0800BA6C..0x0800BA90 | 36 | `sub_800B46C` | battle mode 1: two link routines |
| 0x0802CF48..0x0802D694 | 1868 | `sub_802CAA6` | battle mode 1 |
| 0x0802DD90..0x0802E1E8 | 1112 | `sub_802D1EC` | battle mode 1: helpers, 0x0802E01A, its per-frame code 0x0802E092 |
| 0x080407C4..0x080409F4 | 560 | `sub_803F740`'s string | a link mode (game code BR5J) |
| 0x080BCFCC..0x080BD820 | 2132 | `sub_80BAF06` | **Count** (T1 0x11) and **Django** (T1 0x12) |
| 0x080C3E0C..0x080C44B0 | 1700 | `sub_80C1538` | **Gregar's and Falzar's beasts** (T1 0x30, 0x31) |
| 0x080C91E0..0x080C9640 | 1120 | `sub_80C6264` | the Count boss's lance rain (T3 0x0C) and **Count's lance** (T3 0x0D) |
| 0x080C9EBC..0x080CA440 | 1412 | `sub_80C6ADA` | the Count boss's attacks (T3 0x13..0x15) |
| 0x080E3E7C..0x080E45CC | 1872 | `sub_80E0602` | T4 0x01 (dead) and its helpers |
| 0x080E5190..0x080E56D0 | 1344 | `sub_80E11E0` | battle mode 1: T4 0x0B |
| 0x080E59F0..0x080E5F60 | 1392 | `sub_80E1502` | battle mode 1: T4 0x0D, 0x0E |
| 0x080E6AC0..0x080E6C58 | 408 | `sub_80E2068` | the Count boss's effects (T4 0x17, 0x18) |
| 0x080EDCC8..0x080EE23C | 1396 | `sub_80E90FE` | **the Gregar and Falzar chips' controllers** (T4 0x7A, 0x7B) |
| 0x080EE300..0x080EE3C8 | 200 | `sub_80E91B8` | **the beasts' overlay** (T4 0x7D) |
| 0x080EE9CC..0x080EF180 | 1972 | `sub_80E97BE` | battle modes 10 and 11: T4 0x85, 0x86 |
| 0x080F8268.. | | in the enemy navis' update | battle mode 1's hook |
| 0x08106CC8.., 0x0810D584.. | | the enemy navis' AI | the Count boss (rows 17 and 22) |

## 5. Cosmetic differences

None of these touches the simulation. A trace that compares memory would still see some of them, marked
*(trace)* below.

**Visibility.** The US added these:

- `sub_80169BE`: show unless dimmed; hide the other side's object from a blind viewer. The US calls it from 12
  places; in a netbattle, ChargeMan's train cars (`sub_80DAEC4`, `sub_80DAF2A`) are the callers.
- The VISIBLE bit copied from the owner: SandWrm's hole (`sub_80BC8EC`), the follow effect (`sub_80E3DE0`),
  GrndMan's drill (`sub_80E77B8`).

JP leaves these objects' header bit 0x02 alone *(trace: the object header's flags)*.

**Effects.**

- SuprVulc's bullet burst: bullet row 0x10 after `sub_80C68B0`. It is effect 0x3D (pink flash, palette 2) in
  JP and 0x6A in the US. The US filled entry 0x6A of the effect table in `sub_80E0376`'s extent; JP's is a
  placeholder. A T4#0 effect lives as long as its animation *(trace: the effect's slot lifetime, so later
  spawns' slots)*.
- Absorb's vortex (`sub_80EFCD8`): JP keeps header flags 0x14, so it steps while paused and dimmed *(trace: its
  lifetime)*.
- The effect table's entry 0x1D is sprite 14-17 in JP; it is battle mode 1's effect.

**Sprites.**

- Otenko's statue loads 0C-49 in JP and 0C-00 in the US (`sub_80DB108`'s word, in `sub_80DB2C6`'s pool).
- The absorbed obstacle's last look (kind 0xE, Otenko's statue) is 0C-49 in JP and 0C-41 in the US
  (`byte_80E98C0`, which `sub_80E9810` reads).
- The attachments' look table (`byte_80B8BD4`, which `sub_80B8E30` reads): rows 0xB and 0xC, Django's gun
  (CrosOver's partner) and his bike (the Django chip), are 0C-0F (Django) in JP and 0C-00 in the US (the table
  follows `sub_80B8BA0`).

The content shows the JP look of all three on every console (the user's call: Otenko and Django as the Japanese
games draw them). The pack has 0C-49 and 0C-0F from the JP ROMs (§4.5); on a US console the frame comparison
counts what is drawn with them as a known difference (the frontend's `known.tsv`).

**HUD.**

- `sub_801C640` skips a redraw when its bytes +0x18 and +0x19 agree.
- `sub_801CA34` and `sub_801E44C` copy their tiles at a coordinate one less in JP (7 for 8, 10 for 11).
- The custom screen's close (`sub_8026DC4`) also starts HUD task 0x40 in JP (`sub_801E012`).
- The HUD's graphics list (between `sub_801EC90` and `sub_801FE00`) has one transfer fewer in JP.

**Text.**

- The text box's control-code ranges (`chatbox_8040C44`).
- The text renderer (`sub_814475C`, `sub_8144920`: 0xFFC4 against 0xFF00).
- The build string in `sub_803F740`'s pool. The US's is "REXE6 F 20060110a US". JP's link-mode code (536 bytes;
  it names the game code BR5J) follows it.

**Buffers outside the simulation.**

- The BG animation's buffer is 0x0203E000 in JP and 0x02034A00 in the US (`LoadBGAnimData`, `sub_8030540`).
- `uncompSprite_8002906`'s sprite-area end is 0x0203E000 in JP and 0x02040000 in the US (`sprite_decompress`'s
  pool).
- The uncompressed-sprite state is at 0x02011C00 in JP and 0x02011800 in the US (`sub_8007A0C`).
- The overworld map objects are at 0x020122E0 in JP and 0x02011EE0 in the US (`InitializeStructsOfObjectType`'s
  table).

The battle's own arrays and every RAM address a `same` routine uses are identical.

**After the battle.**

- `sub_8007850`: the exit, with the US's extra case for a battle record +0xE of 8.
- `sub_812B708`: the US also calls `sub_8149568`.

## 6. Unreachable in a netbattle

A netbattle is battle mode 0. All 192 link battle settings records have mode 0 in both JP ROMs too, and they are
byte for byte the US ones, actor lists included (`stages.py`).

**Battle mode 1.** JP has a handler of its own: entry 1 of the battle-mode table (`off_8007B50`, which
`battle_8007A44` reads) points at JP code. US mode 1 runs mode 0's handler. JP's mode 1:

- sends a 6-word block with magic 0x1F2F3F4F over the link (`sub_80200A4`), and copies 20 words from the
  receive area to 0x02034050;
- keeps its state at 0x02039FE0: a 16-entry queue with sound 0x195, and counters;
- looks for Beast Out (chip 0x13F) in the hand;
- in Beast forms 11 and 12, marks hits (collision +0x64/+0x65) through 0x0802E01A;
- preloads Django's sprite (`sub_8007A0C`);
- spawns T4 0x0B, 0x0D and 0x0E and effect 0x1D.

By its code, a mode fed over the link port; presumably the Beast Link Gate accessory's battles **[unverified]**.

Its hooks sit in shared routines. In mode 0 they do nothing but check a variant or the mode:

- `sub_80C4E7C` (projectile variants 1 and 12);
- `sub_80C6964` (the bullet's variant 12);
- `sub_80CE83C` (YoYo);
- `sub_80C52D0` (a marked collision region);
- the region markers in `sub_80C5014`, `sub_80C5050` and `sub_80EB862`, and `object_spawnCollisionRegion`. In
  JP they write a region object's +0x74 (ExtraVars+0x14): 0 at the spawn, 1 for projectile bursts and sword
  slashes. Nothing reads it in mode 0, and the US never writes it *(trace: memory differs)*.

The JP-only code itself sits after `sub_80084C0` (420 bytes), `sub_80095C8` (1048), `sub_800B46C` (two link
routines), `sub_802D1EC` (1112), `sub_802CAA6` (1868) and `sub_80E11E0` (T4 0x0B, 1344). There is a hook in the
enemy navis' update too (JP Falzar 0x080F8268, from `sub_80F2354`).

**Battle modes 10 and 11.**

- `sub_8008064`: when the banner ends and the fight starts in mode 10, on turn 4 (BattleState+7), frees type-16
  objects and spawns T4 0x85.
- `sub_80080D2`: in mode 11, when `sub_800A152` returns 1, does the same with T4 0x86.

**AI and single player.**

- The Count boss (§4.2).
- `sub_801A77A`, `sub_80C464C`/`sub_80C46C6`, `sub_8108F74` (§3).
- The US's effect-follows-owner (`sub_80E060E`, `sub_80E0616`, called from `sub_80E05C4`). Only `sub_8113C48`
  (AI) asks for it.
- The single-player battle settings and actor lists before `sub_80B81EC`, which are 1,184 bytes shorter in JP.

**T4 0x01.** Nothing spawns it (§4.2).

## 7. The same code

85 routines in JP Falzar differ only in what moved. Most are next to JP-only code, which shifts their pools or
tables. The rest:

- jump tables and colour tables moved: `sub_800794C`, `sub_8016A38`, `sub_801A554`, `sub_80E3D90`, the
  `sub_80E9140` twins;
- padding: `object_setAttack0`/`1`, `battleSettings_802D2B2`, and the zero bytes after `sub_80C53A6`,
  `sub_80D05EC`, `sub_80E3B50` and `sub_80E4DA2`;
- the custom screen's graphics list, where only the graphics pointers moved (`sub_802A646`);
- a byte nothing reads: the second byte of `sub_80CA234`'s panel records, 0x20 | element in the US and the bare
  element in JP. Its caller `sub_80C9F98` overwrites the register;
- the IWRAM: only `_object_setPanelType`'s unreachable tail after its pool differs. The JP IWRAM code is shorter
  by 0x28 from 0x03006CD0 (the US-only `sub_3006CA8`, out of scope) and by 0x2C from 0x03007038. So
  `_object_setPanelType` is at 0x03007978 in JP, and 0x030079A4 in the US.

`classes.tsv` has every name.

## 8. What a JP-console netbattle reaches that the engine gets wrong

What the switch looks like is §8.1. Each variant names where it lives and what a trace from a JP console would
show against the engine.

| # | Variant | Size | A JP trace shows | Where to switch |
|---|---|---|---|---|
| 1 | Charged shot of a worn-out navi (`sub_8011A7E`) | US: +7 instructions | the shot's damage (AIAttackVars+8) and the target's HP: (Attack + 1) × 10, not 10 | content/bn6/navis/megaman/weapons/charged-shot/weapon.luau (`setup`) |
| 2 | SpoutCross's and SlashCross's charged shots keep the last bonus (`sub_8011C5E`, `sub_8011CD6`) | US: +2 and +1 instructions | AIAttackVars+6, the damage word | content/bn6/navis/megaman/forms/spoutcross/charge.luau and slashcross/charge.luau (`navi.extra = 0`) |
| 3 | The weakness request survives a form change (`sub_8014CC0`, `sub_8014F04`, `sub_8015128`, `sub_80155CC`) | 3 literal words | AIData+0x44 bit 31 after the change, then the un-cross | crates/nettai-battle/src/kinds/player/actions/transform.rs (`finish`, for Cross, BeastOut, CrossBeast, BeastCross; not BeastOver) |
| 4 | The drag's recovery keeps RskyHny's heat trap (`sub_8017A38`) | US: +2 instructions | AIData status 0x200000 after the drag; later non-fire hits swallowed | crates/nettai-battle/src/kinds/player/reactions.rs (`recover_from_drag`) |
| 5 | The traps' order and 0-damage hits (`sub_802CEF4`) | about 20 instructions moved, 2 branches | which trap answers (requests 0x200/0x8000/0x400 against the heat trap's marker and sound 0x6E); a 0-damage hit landing | crates/nettai-battle/src/kinds/player/intake.rs (`anti_damage_traps`) |
| 6 | LilBolr (`sub_801B878` US only, `sub_80D774C`) | US: a 324-byte routine and +10 instructions | the boiler's HP and destruction on a crushing touch while it erupts | content/bn6/chips/lilbolr/boiler.luau (its `obstacle.react` crush, the HP reset) |
| 7 | ElemTrap's counterattack's list position (`sub_80E360E`; `sub_80033E4` US only) | 1 call | T4 0x2B's place in the update list; its first update comes in the same tick, a tick before the US's | content/bn6/chips/elemtrap/trap.luau (spawn after the trap, not at the head) |
| 8 | ElmntMan when nobody picks (`sub_80BABAC`) | US: 9 instructions, JP 1 | no RNG2 draw (so every later draw shifts), action 0x18, no attack | content/bn6/chips/elmntman/navi.luau (`cycle`) |
| 9 | Tornado's doubling on a special panel (`sub_80C9F98`) | US: +5 instructions | the tornado's damage word | content/bn6/chips/tornado/tornado.luau (`init`) |
| 10 | AirSpin's top's HP | 1 word: 300 against 400 | the top's HP and MaxHP, how many hits break it | content/bn6/chips/airspin/top.luau (`HP`) |
| 11 | The dash hit's reach (`sub_80DB684`) | 1 word: 60 against 58 px | the hit's X, and the tick it reaches a panel | content/bn6/navis/megaman/dash_hit.luau (`REACH`) |
| 12 | The hand size under the hand-shrink bug (`sub_802A40C`) | US: +7 instructions | the custom screen's dealt count (+6), so the hand | crates/nettai-battle/src/custom/screen.rs (`hand_size`) |
| 13 | DustMan's pull's collision type (`sub_80F20A0`) | 1 byte | the pull regions' self type (row 0x04 against 0x1E): what they reach and how a hit counts | content/bn6/navis/dustman/chip.luau (`pull`: `self_type`) |
| 14 | GroundCross's drill burrows while moving (`sub_80EF004`) | US: +3 instructions | the drill's timing | content/bn6/navis/megaman/forms/groundcross/drill.luau |
| 15 | SpoutMan's DripShwr flags (`sub_80F1694`) | JP: +5 instructions | SpoutMan's ObjectFlags1 0x40 and 0x80000 during the spray (what reads them then is **unverified**) | content/bn6/navis/spoutman/chip.luau |
| 16 | Otenko's statue as DustMan's junk (`byte_8021220`, NameID 0xCF) | 5 data bytes | US: no junk (look none, freed); JP: a junk with sprite 0C-49, animation 1, shadow, which flies and hits | content/bn6/chips/otenko/statue.luau (the identity's `look`): **the content has the JP row on every console** (the user's call), so a US console's trace of it differs; bn6battle-verify records it on JP consoles (jp/chips/0x099-otenko/dustman), and gen-content's check expects the JP row |
| 17 | The emotion window's glitch from patch cards (`sub_813BF1C`) | JP: +3 instructions | the console's emotion-window flicker | the setup: `emotion_window_glitch` from a JP save includes the patch cards' byte (out of scope) |

Not counted:

- the obstacle push without pusher bits (`sub_800F598`), believed unreachable;
- the cosmetic rows marked *(trace)* in §5, which a memory-level trace would also see.

Then there is the content the engine lacks altogether, which the other work packages are porting (§4):

- the Gregar and Falzar chips: their controllers, beasts and overlay, the orphaned US attacks T3 0xBC, 0xCD and
  0xCE, and `sub_800BCF6`;
- Count (T1 0x11 and T3 0x0D);
- Django (T1 0x12, sprite 0C-0F);
- the Django and GunDelEX records' JP class and flags.

These sizes from bn6-lmao's regions are exact (§9):

| Region | Bytes |
|---|---|
| Count | 1,004 |
| Django | 1,128 |
| the lance | 424 |
| Gregar's beast | 1,116 |
| Falzar's beast | 584 |
| Gregar's controller | 452 |
| Falzar's controller | 944 |
| the beasts' overlay | 200 |

### 8.1 The console-region switch

**Decided (the user, 2026-10-02): no switch, "just these chips".** The engine runs the US behaviour for §8's variants
and has none of this section's switch. The JP-content chips (GunDelEX, Otenko, Count's three, Django's three, Gregar,
Falzar) take the Japanese games' records and routines outright (`gen-content check` compares their records with a
Japanese ROM's), and the content has the Japanese games' looks where the US games blanked them (CrosOver's Django
gun, attachment row 0xB). A JP-console recording that reaches one of §8's variants differs from the engine there.
What follows is the switch as proposed.

The JP and US code differ, not the data a console sends. So nothing in NaviStats or the init exchange says which
one ran. The region belongs in the setup:

- **Engine:** `RoundSetup::region: Region` (`Us`, `Jp`), beside `local_side` and `link_delay`.
  - It is part of the shared setup: both peers run the same simulation, so it is one value per match, not per
    side.
  - Tango pairs the EXE6 family (JP) and the BN6 family (US) apart: its lobby requires the same netplay tag. So a
    netbattle is all JP or all US, whichever version each player has.
  - A mixed match can't be reproduced by one simulation anyway. The original consoles would each run their own
    code for both sides, and drift apart.
- **Content:** `battle.region(): "us" | "jp"` in core.d.luau, read where the variant is (`if battle.region() ==
  "jp" then ... end`). The version stays per navi (`NaviStats.version`).
  - The JP-only chips' hooks dispatch on it. In the US they keep `lib/unusable`'s error, which is what the US ROM
    does (it jumps to address 0); in JP they run the ported behaviour.
- **Records and assets:** a chip whose record differs by region (Django, GunDelEX) takes its class and flags
  from a region-keyed table. JP's sprites are extracted from the JP ROM; an asset name says which.
- **Compat:** bn6-compat reads the region from the ROM header (BR6J or BR5J is JP) for traces and sets the
  setup's field. compat/games.toml gets `[exe6f]` and `[exe6]` sections, as `[gregar]`, with §10's values.

## 9. bn6-lmao's regions

bn6-lmao is a hack of the US ROMs that restores this content. Its relocate_jp_routines.py copies eight regions of
each JP ROM into a US ROM and repoints their calls; extract_assets.py takes the JP art and archives. Its region
boundaries match this audit's kinds exactly:

| bn6-lmao's name | JP Falzar | JP Gregar | What it is (this audit) |
|---|---|---|---|
| count_native_main | 0x0BCFCC..0x0BD3B8 | 0x0BE82C..0x0BEC18 | T1 0x11, Count; `count_attack_main` (+0x26A) is navi chip 18 |
| count_native_aux_main | 0x0BD3B8..0x0BD820 | 0x0BEC18..0x0BF080 | **T1 0x12, Django**, with navi chip 19 at +0x2EA |
| count_lance_main | 0x0C9498..0x0C9640 | 0x0CACF8..0x0CAEA0 | T3 0x0D, the lance |
| gregar_child_main | 0x0C3E0C..0x0C4268 | 0x0C566C..0x0C5AC8 | T1 0x30 |
| falzar_child_main | 0x0C4268..0x0C44B0 | 0x0C5AC8..0x0C5D10 | T1 0x31 |
| gregar_controller_main | 0x0EDCC8..0x0EDE8C | 0x0EEFF8..0x0EF1BC | T4 0x7A; `gregar_attack_main` (+0x174) is dimming 34 |
| falzar_controller_main | 0x0EDE8C..0x0EE23C | 0x0EF1BC..0x0EF56C | T4 0x7B; `falzar_attack_main` (+0x1FA) is dimming 35 |
| gregar_shared_aux_main | 0x0EE300..0x0EE3C8 | 0x0EF630..0x0EF6F8 | T4 0x7D, both beasts' overlay |

What the boundaries say:

- **Each region is one kind.** Each stops at a kind the US also has: after Count's lance comes the US's T3 0x0E,
  after the controllers T4 0x7C, after the overlay T4 0x7E.
- **The JP chips are self-contained.** Nothing else is JP-only that they need. They call only US routines:
  bn6-lmao's call maps list 50 common ones and 11 per version. Among those are the orphans of §4.3:
  `sub_80DCC70`, 0x080DF160 and `sub_80DF262` (the US Falzar addresses of bn6-lmao's Falzar map). The pointer
  map adds `sub_800BCF6`.
- **The regions leave out everything else this audit found JP-only.** That is the Count boss's attacks (T3 0x0C,
  0x13..0x15, T4 0x17, 0x18) and battle modes 1, 10 and 11. They are not chip content.
- **"count_native_aux_main" is Django's navi, not Count's.** Navi chip 18 spawns T1 0x11 (sprite 08-16); navi
  chip 19 spawns T1 0x12, whose init loads 0C-0F, the Django archive. bn6-lmao registers it as T1 0x12 under
  Count and gives its Django chips a C reimplementation with BN5's art instead. The JP ROM's Django chip is
  complete: the handler, the navi, the sprite.
- **bn6-lmao changes JP behaviour in one place.** It adds the attack bonus to Falzar's Strike Feathers (a patch
  at `falzar_controller_main`+0x302). The JP original forwards the base power only; a faithful port keeps that.

## 10. For JP tracing

The values compat/games.toml keeps for the US Gregar ROM, for the JP ROMs (`games.py`; checked by reading the
bytes at both addresses):

| | JP Falzar (BR6J) | JP Gregar (BR5J) |
|---|---|---|
| the stages' actor lists (every one moves the same) | +0x2530 | +0x3D90 |
| BattleSettingsList1 | 0x080B32B8 | 0x080B4B18 |
| colonel, heatman, spoutman, tmhkman, diveman, tenguman /navi Z | 0x080BA57D, 0x080BB2AB, 0x080BB7E1, 0x080BBA2B, 0x080BBBFF, 0x080BBF9F | 0x080BBDDD, 0x080BCB0B, 0x080BD041, 0x080BD28B, 0x080BD45F, 0x080BD7FF |
| crcusman, judgeman, dustman, twinldrs, crosover /navi Z | 0x080BC6F1, 0x080BC9B1, 0x080BE9AB, 0x080C025F, 0x080C0CA5 | 0x080BDF51, 0x080BE211, 0x080C020B, 0x080C1ABF, 0x080C2505 |
| mstrcros, darkness, roll, protoman /navi Z | 0x080C1A09, 0x080C2579, 0x080C3681, 0x080C598D | 0x080C3269, 0x080C3DD9, 0x080C4EE1, 0x080C71ED |
| flmhook/hook, tenguman/tornado Z | 0x080EFAAD, 0x080F7514 | 0x080F0DDD, 0x080F8844 |
| moonbld/blade panel Y | 0x6F (`sub_80EE2CE` at 0x080F3C6E) | 0x9F (0x080F4F9E) |

Also for a JP trace:

- The frame hook after `main_awaitFrame` (0x080002D4) is at the same address in both JP ROMs (`main_` differs
  only by its subsystem table, which moved). The battle frame's return site (US Falzar 0x0812B6AC, in
  `sub_812B698`) is 0x081340AC in JP Falzar and 0x08135E74 in JP Gregar.
- The IWRAM code shifts as §7 says. `fmap.py map` gives any code address; data addresses (the m4a tables, for
  one) need the pointer pairs, as `games.py` maps them.
- Regions carry JP's +0x74 (§6).
- The cosmetic rows marked *(trace)* in §5 apply.
- The mode-1, mode-10 and mode-11 code never runs.
- The JP-only navis' Z fractions are their spawners' addresses too: the content keeps EXE6 Falzar's (Count's
  0x080BD237, Django's 0x080BD6A3), EXE6 Gregar has 0x080BEA97 and 0x080BEF03. Django drops from that Z, so on EXE6
  Gregar his and his bike's Z differ until he lands (games.toml's `spawner_z_drops`).

The kind and handler tables move too: JP Falzar's T1, T3 and T4 tables are at 0x08003C80, 0x08003EA8 and
0x080042AC; the handler tables at 0x0802D810 and 0x0802D8B8.

## 11. Open

- **Battle mode 1's purpose** (§6): read from its code, not identified.
- **SpoutMan's flags (#15):** whether anything reads MOVING on the stand-in during the spray.
- **The weakness request (#3):** whether a request can be pending when a form change ends. It would need a
  weakness hit just before the change, with no recording of one. The difference is certain; how often a
  netbattle reaches it isn't.
- **0x020065F0** as patch-card state (#17).
- **Sprite slots** 18-34..36 (§4.5: the window differences of 0C-0E, 0C-47, 0C-67 and 10-16..19 are data read
  past the archives; 14-17 is battle mode 1's label).
- **The out-of-scope routines** (AI, viruses, the overworld) that differ were not classified one by one. 72 in
  JP Falzar outside the battle blocks: 50 `differs` and 22 `missing` in fmap.py's table.
