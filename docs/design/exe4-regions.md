# EXE4: where the US and Japanese battle code differ

What EXE4's Japanese releases' battle code does otherwise than the US releases': every place verify's
`tools/cross-rom/code.py B4WE B4WJ` shows in the battle's ranges, read, sorted, and recorded where a netbattle reaches
it. nettai has the US behavior wherever the two differ (the user's rule of 2026-10-12); a Japanese console's recording
of each place a netbattle reaches is in the lab (verify's data/traces/lab-exe4, under `jp/`) and matches up to the
place, a listed deviation (verify's crates/trace-tests/src/deviations.rs) or the same throughout. The HUD's, the
custom screen's and the chatbox's places, read before this pass, are the map's (docs/design/exe4-map.md §11).

## 1. What was read

- **The core** (US 0x08003000–0x08014000: the battle's flow, the objects' system, the player's intake, the hit
  kernel, the chip use): 25 places.
- **The objects and the player's actions** (US 0x080B34D8–0x080F5E00: the actor, attack and effect tables' routines,
  0x08003A24, 0x08003C40, 0x08003F78, and the player's action table's, 0x080EAEFC): 87 places, each filed under the
  kind whose routine holds it (verify scratch's `kind_hunks.py`) and the content's name for that kind.
- **The chip records** (0x180 records of 0x2C bytes, US 0x080197EC, Japan 0x0801972C), field by field: the 280
  places code.py shows in that range are these records decoding as instructions.
- Not read, as the job's scope says: the story, the menus, the shops, the overworld's objects (US 0x080A0000 to
  0x080B34D8), the result screen (0x080F5xxx), and the places that are words or art (a literal pool or a table
  decoding as instructions on both sides: called "data" below, each looked at to be sure).

Each row: the US address, the Japanese one, what differs, and its class:

- **same**: the same behavior (a moved address, a register, an alignment, data);
- **JP defect**: a Japanese slip the US release fixed (nettai has the fix);
- **design**: a difference of design, neither wrong (nettai has the US one);
- **decided**: the user's decision of 2026-10-12;
- **not reached**: no netbattle reaches it.

No US defect or omission that the Japanese releases have right was found. The one candidate (Roll against an armed
AntiRecv while the battle is dimmed) the user decided for the US behavior.

## 2. The core

| US | Japan | What differs | Class | A Japanese console's recording |
|---|---|---|---|---|
| 0x080061A0 | 0x08006180 | a literal pool's words | same (data) | |
| 0x08006F44 | 0x08006F24 | the US battle init zeroes two halfwords at [sl+68]+38 and +40 in its branch that isn't a link battle | not reached | |
| 0x08006FB8 | 0x08006F8E | an alignment halfword | same | |
| 0x08007AC4 | 0x08007A9C | the US clamps base max HP / 100 to at least 1 (a battle setting byte) | same in a netbattle (base max HP is never under 100) | |
| 0x08007B66 | 0x08007B38 | an alignment halfword | same | |
| 0x080084D2 | 0x0800848E | `sub_801DFA2(0x4000)` called after the second controller's test in the US, before it in Japan | same in a netbattle | |
| 0x0800851E | 0x080084EC | an alignment halfword | same | |
| 0x0800BAAC | 0x0800BA78 | the US input decode skips the time stop's test for battle kind 0x44 | not reached (a story battle's kind) | |
| 0x0800C46C, 0x0800C470 | 0x0800C430 | the search of a side's three navi slots: the Japanese loop tests stale flags and never steps, so it returns the first slot | JP defect; same in a netbattle (one navi a side) | |
| 0x0800C784 | 0x0800C744 | a trap's counter taking over (0x0800C780) clears the navi's first object link (its blade, its gun) in the US | JP defect (the engine's `drop_everything`) | jp/regions/slasher-antidmg (with the row below); GunSol1 caught by AntiDmg showed the same on both consoles |
| 0x0800C86C | 0x0800C828 | Beat spares a dark chip in the US (the record's flag 0x20: `chip_use.beat_spares_dark`); the Japanese Beat takes it | design | jp/navicust/beat-spares-dark: deviation at 426 |
| 0x0800CFC2 | 0x0800CF76 | Reflect (weapon routine 0x26) sets the B+Left special's cooldown to 40 in the US (`REFLECT_COOLDOWN`); Japan sets none, so a second B+Left inside 40 ticks is a second guard | design | jp/regions/reflect-twice: deviation at 437 |
| 0x0800E824 | 0x0800E7D4 | the US clamps the move lag's column (NaviStats +0x25, base max HP / 100) to at least 1 at the navi's init | same in a netbattle | |
| 0x0800EC14 | 0x0800EBBE | an alignment halfword | same | |
| 0x08010AF2 | 0x08010A9E | a drag's start clears the guard flag too in the US (`object_clearFlag` 0x1441, Japan 0x1440) | JP defect (the engine's `start_drag`); not shown: three scenarios of a Fan pulling a navi that puts Guard1 up (20, 40 and 60 ticks in) are the same on both consoles | |
| 0x08010D24 | 0x08010CD0 | a literal pool's word | same (data) | |
| 0x08012C74, 0x08012C78 | 0x08012C20, 0x08012C26 | the hit kernel: Japan multiplies a hit's damage by the hitter's emotion factor (0x08012CC0: 24/16 while its mood is 0xFF, Full Synchro), the US computes it and only marks hit flag 0x80 | decided: the same in a netbattle. A chip used in Full Synchro spends the mood (0x99) at its use, before its hit lands, so the factor is 16 there: measured 40, 40, 80 and 80 on both consoles for a plain hit, a counter hit, Full Synchro and Full Synchro's counter hit, and nettai the same | jp/regions/full-synchro-counter: the same throughout |
| 0x08012CA0 | 0x08012C52 | the factor routine's literal pool | same (data) | |
| 0x080130F4, 0x08013184 | 0x080130A8, 0x08013132 | lava's burn takes 20 off the mood in the US (a player's and a dimmed player's burn) | design | jp/regions/lava-full-synchro: deviation at 418 (the burn ends the Full Synchro in the US; the Cannon after it deals 40, Japan's 80) |
| 0x08013280 | 0x08013228 | a chip-destroying hit ends the navi's chip requests in the US (0x0C); Japan leaves them, and a chip asked for on the hit's frame is used from the emptied hand (chip 0xFFFF, a record past the chip table) | JP defect (the engine's `lose_chip`, in every game: exe4-map §18 item 136) | jp/destruction/asked: deviation at 806 |
| 0x080133AE | 0x08013350 | an alignment halfword and a pool word | same | |

## 3. The objects and the player's actions

| US | Japan | Kind (content) | What differs | Class | A Japanese console's recording |
|---|---|---|---|---|---|
| 0x080C1990, 0x080C19A0, 0x080C19D8 | 0x080C18F0, 0x080C1922 | actor #0x2C, Roll (roll/navi) | her heart: the US drops it on her side's navi's panel and heals three times her damage's low 14 bits, doubled with the doubling bit (0x0800C456, the mask, 0x080C19AA); Japan drops it on her own spawn panel and heals three times the raw damage word | JP defect (the heal); the place is the same while her user is hidden | jp/regions/roll-full-synchro: deviation at 2198 (Roll used in Full Synchro on poison: the US heals three times her doubled damage, 904 HP then; Japan's fills the HP); jp/chips/0x0dd-roll/hit: the same throughout |
| 0x080C8D00 | 0x080C8C40 | actor #0x45, NumbrBl's NumberMan (numbrbl/numberman) | his init sets animation 0 and its copy 0xFF in the US, his animation 255 twice in Japan | JP defect (exe4-map §11) | jp/chips/0x0ac-numbrbl1/hp: deviation at 485 (listed before this pass) |
| 0x080CBECA, 0x080CBF1C, 0x080CBF20, 0x080CBF30, 0x080CBF46 | 0x080CBE0A, 0x080CBE58, 0x080CBE6A, 0x080CBE82, 0x080CBE96 | actor #0x55, PrixPowr's KendoMan (prixpowr/kendoman) | his first tick: reworked in the US | JP defect (the fixed routine) | chips/0x138-prixpowr/jp: deviation at 800 (listed before this pass) |
| 0x080CDA4E | 0x080CD942 | attack #0x05, the die (dice) | the US init places the die on its panel (0x080CDA4A) before it makes its collision data | JP defect; same in the recording | jp/chips/0x0f2-numbrman/hit: the same throughout |
| 0x080D7002 | 0x080D6EF2 | attack #0x4C, Mine (mine/land-mine) | the armed mine clears its header's bit 0x02 in the US and 0x12 in Japan, so the US mine keeps running while the battle is dimmed | design | jp/chips/0x062-mine/stepped-on: deviation at 572 (Geddon over an armed mine showed the same flags difference) |
| 0x080D7352, 0x080D7364 | 0x080D7242, 0x080D7256 | attack #0x4E, Roll's heart (roll/heart) | a `push {r5}` placed otherwise | same | |
| 0x080E10DA | 0x080E0FCA | attack #0xA1, DarkNeo's image (darkneo/image) | the pool after a `pop` | same (data) | |
| 0x080E3352, 0x080E33A2, 0x080E33AA | 0x080E3242, 0x080E3286 | effect #0x11, the sparkles (sparkles) | the US copies its parent's position as it starts and bursts there; Japan reads its parent's each time, the parent freed or not | JP defect; same in the recording (the parent stays put) | jp/chips/0x0a0-seekbom1/hit: the same throughout |
| 0x080E33E8 | 0x080E32CE | effect #0x12, FireSoul's grass | an alignment halfword | same | |
| 0x080E4018 | 0x080E3F00 | effect #0x1D, Geddon's controller | the pool after a `pop` | same (data) | |
| 0x080E47FA | 0x080E46E2 | effect #0x22, GrabBnsh's and GrabRvng's controller (grabbnsh/controller) | its end waits 90 ticks in the US, 30 in Japan | design | jp/chips/0x077-grabbnsh/stolen and jp/chips/0x076-grabrvng/stolen: deviations at 696 |
| 0x080E528E | 0x080E5172 | effect #0x2B, an Anti trap's strike (anti-trap/strike) | the US adds the trap chip's Atk+ bonus (the object's +0x32) to the strike's damage; Japan leaves it out | JP defect | jp/regions/antifire-atk: deviation at 722 (AntiFire with an Atk+30: the US strike deals 230, Japan's 200) |
| 0x080E551A | 0x080E53FA | effect #0x2D, the Roll chips' controller (roll/controller) | the US spawner springs the other side's armed AntiRecv only while the battle isn't dimmed; Japan's springs it a Roll cut in too | decided (2026-10-12): the US behavior | jp/regions/roll-antirecv-dimmed: deviation at 645 (Roll cut in on AreaGrab: she comes in the US, AntiRecv's mark shows in Japan) |
| 0x080EA424 | 0x080EA2FC | effect #0x75, LaserMan's controller | the pool after a `pop` | same (data) | |
| 0x080ED582, 0x080ED592, 0x080ED598 | 0x080ED45A, 0x080ED45C, 0x080ED460 | action 0x35, Slasher (slasher/action) | after each phase the US yields to a sprung trap's counter (0x0800C780, requests 0x600); Japan goes on (the rest: its phase table's words) | JP defect | jp/regions/slasher-antidmg: deviation at 735 |
| 0x080EF5EC, 0x080EF620 | 0x080EF4B0, 0x080EF4DA | action 0x73, the computer navis' idle (0x080EED7C, the idle table's entries 26 to 38) | the US tests whether the navi can move before its step | not reached (the story's navis) | |
| 0x080F5A92 | 0x080F594E | the result screen's reward | (exe4-map §11) | not reached (not a battle's) | |
| 0x080B40C8 on: actors #0x04, #0x05, #0x08, #0x11, #0x13, #0x15, #0x18, #0x1B, #0x1C, #0x24, #0x25, #0x2A, #0x2D, #0x35; attack #0x3C; effect #0x2E (58 places) | | kinds the content hasn't: none has a spawner with an immediate, they come from the enemy tables | viruses' and the story navis' code | not reached | |

## 4. The chip records

| Chip | Field | US | Japan | Class | A Japanese console's recording |
|---|---|---|---|---|---|
| every chip | +0x18, +0x19 (the sort keys) | | | not reached (the library's and the folder's orders, by language) | |
| 0x050 SidBmbo3 | +0x16 bit 0x02 (the other side's Rush eats the chip, 0x0800C8C0: Invis's and PopUp's) | 0 | 2 | JP defect (SidBmbo1 and SidBmbo2 haven't it) | jp/regions/rush-sidbmbo3: deviation at 414 (Rush eats it in Japan) |
| 0x0A7 Ligtnin2 | +0x06 MB | 45 | 33 | decided (2026-10-12): the US MB | not a battle's: a folder's and a Regular chip's MB (a Japanese save's Ligtnin3 as its Regular chip within a limit of 39 to 52 is legal there and not by the US value) |
| 0x0A8 Ligtnin3 | +0x06 MB | 53 | 39 | decided (2026-10-12): the US MB | as above |

## 5. The recordings

The lab's `regions/` (gen_exe4.py's `region_scenarios`: eight on US consoles, each replaying every frame) and
gen_exe4_jp.py's `CODE`: their Japanese copies and those of the library's own scenarios that reach a place above. On
Japanese consoles twelve stop at their place, a listed deviation each (deviations.rs, `JP_*`), and four play the
same throughout (regions/full-synchro-counter, chips/0x0f2-numbrman/hit, chips/0x0a0-seekbom1/hit,
chips/0x0dd-roll/hit). Each pair was recorded on both regions' consoles first and compared state by state (verify
scratch's `region_diff.py`): the first difference of each is the place's, nothing earlier.

## 6. EXE5 and EXE6

The same ranges by the same measure (code.py's places, Thumb on both sides, moved pointers left out):

| | Core through the custom screen | of it the chip table | Objects and the player's actions |
|---|---|---|---|
| EXE4 (B4WE / B4WJ) | 364 (0x08003000–0x08025000) | 280 | 87 (0x080B34D8–0x080F5E00) |
| EXE5 (BRBE / BRBJ) | 351 (0x08003000–0x0802FC00) | 300 | 65 (0x080B8BD0–0x080FC000) |
| EXE6 (BR6E / BR6J) | 462 (0x08003000–0x0802F200) | | 13,396 (0x080B9A5C–0x080FF400) |

- **EXE5**: 51 places of code in the core and 65 in the objects, about two thirds of EXE4's (84 and 87), and its 368
  chip records to compare field by field. Its HUD, custom screen and chatbox were read by EXE5's port; the rest is
  docs/design/exe5-regions.md.
- **EXE6**: diff can't align its two ROMs' object code (the Japanese releases carry whole routines the US ones
  haven't and the other way round, docs/engine/jp-differences.md §4), so the 13,396 is an artifact of the measure.
  docs/engine/jp-differences.md compared every routine of the US Falzar ROM with its Japanese one by routine
  (anchored on the object tables), classified all 153 that differ and recorded the 20 a netbattle reaches. A pass by
  these ranges would add nothing to it but a check that the routine map missed nothing (the JP Gregar ROM's 6 more
  routines and its 42 version differences are there too).
