# EXE5: where the US and Japanese battle code differ

What EXE5's Japanese releases' battle code does otherwise than the US releases': every place verify's
`tools/cross-rom/code.py BRBE BRBJ` shows in the battle's ranges, read, sorted, and recorded where a netbattle reaches
it, as docs/design/exe4-regions.md did for EXE4. nettai has the US behavior wherever the two differ (the user's rule of
2026-10-12); a Japanese console's recording of each place a netbattle reaches is in the lab (verify's
data/traces/lab-exe5, under `jp/`) and matches up to the place, a listed deviation (verify's
crates/trace-tests/src/deviations.rs, `JP5_*`).

What the earlier Japanese work settled is not read again here (exe5-map's assets section): the Japanese ROMs' words
and lettering (the fonts, the HUD's lines, the ten banners whose words differ, カスタム中… for "Cstmzing...", the chip
window's and the soul button's pictures), which the pack holds beside the US ROMs' and a console in Japanese draws; the
"BLOCK!" sprite and background 0x05, which the pack has in the US ROMs' form; the hit's song 0x6D, louder in the
Japanese ROMs. The frame comparison's Japanese sample (verify's tools/frontend-compare/exe5-jp.txt, exe5-sweep.tsv)
matches its consoles but for one known difference: a Japanese console times the no-running message by its own words.

## 1. What was read

- **The core** (US 0x08003000–0x0802FC00: the battle's flow, the objects' system, the player's intake and actions'
  shared code, the HUD, the custom screen): 351 places, of which 300 are the chip table's records decoding as
  instructions; the other 51 are below.
- **The objects and the player's actions** (US 0x080B8BD0–0x080FC000: the actor, attack and effect tables' routines,
  0x08003958, 0x08003B80, 0x08003F60, the chip actions' table 0x080EB42C, indexed by the action less 0x10, and the
  game-played navis' rows, 0x080F228C): 65 places in 26 kinds or places, each filed under the routine that holds it
  (verify's tools/exe5/region_kinds.py) and the content's name for its kind.
- **The chip records** (368 records of 0x2C bytes: Team ProtoMan 0x0801E214 against Team of Blues 0x0801E1D0, Team
  Colonel 0x0801E210 against Team of Colonel 0x0801E1CC), field by field, the art by what its pointers point at (verify's tools/exe5/region_chips.py).
- **The whole lab on Japanese consoles**: every library-exe5 scenario (2,542) copied onto Japanese bases (Team of
  Blues against Team of Colonel, Tango's netplay saves for the Japanese ROMs: verify's tools/exe5/jp_sweep.py),
  recorded and replayed against the engine: 2,487 recorded (55 picked chips the Japanese saves' folders don't deal),
  2,444 match every frame, 2 stop at their listed deviation (destruction/asked, the engine's own), and 41 stop at a
  place below: 40 at the status visual's panel, 1 at the no-running message's pace. Their sounds match but for that
  message's.

Each row: the US address, the Japanese one, what differs, and its class:

- **same**: the same behavior (a moved address, a literal pool, an alignment, data);
- **JP defect**: a Japanese slip the US release fixed (nettai has the fix);
- **language**: the words' layout (the pack's Japanese lettering);
- **not reached**: no netbattle reaches it;
- **US omission?**: a place where the Japanese release may have it right (reported, the US behavior kept until the
  user decides).

One place is of that last class (Roll against an armed AntiRecv, §7); no other US defect or omission was found.

## 2. The core

| US | Japan | What differs | Class | A Japanese console's recording |
|---|---|---|---|---|
| 0x080092C8 | 0x080092C8 | Japan's escape (the custom screen's answer 2, 0x08007F88) also runs the battle's end stat routines (0x08012868, 0x0801285A, 0x0801296A: NaviStats +0x44 by battle effect 0x800) | not reached (no escape in a netbattle) | |
| 0x0800E522, 0x0800E52C | 0x0800E536, 0x0800E538 | the body hit modifier (`body_hit_modifier`): the US returns 3 for a player-type actor of AI index 23 too | same in a netbattle (a link battle returns 3 first; AI index 23, NameIDs 0x18E–0x190, is no navi a player operates) | |
| 0x0800E62E | 0x0800E634 | an alignment halfword | same | |
| 0x0800EE32 | 0x0800EE36 | Hub Style's shade (MegaMan's base form's put-on, 0x0800EE1C): the US takes the old shade down before it spawns the new one in his second related slot; Japan overwrites the slot | JP defect (forms/base.luau's `put_on`) | not shown: no recording puts the shade on over one |
| 0x080105D2 | 0x080105CA | `may_charge`: the US skips the status tests for AI index 23 | not reached | |
| 0x0801064A | 0x08010634 | a literal pool's words | same (data) | |
| 0x08011556, 0x08011566 | 0x0801153E, 0x0801154E | the NaviCust battle-start bug's stronger variants (NaviStats +0x1A = 10: `style_hook`'s 5 to 8): blind and confused for 600 ticks in the US, 300 in Japan (invisible and invulnerable 600 in both) | JP defect | jp/navicust/bug-status-strong: its confusion runs 423–1022 on US consoles, 423–722 on Japanese ones; the listed deviation is the status visual's, at 423, before it |
| 0x08011E94 (and 0x08011EB0) | 0x08011E7C (0x08011E86) | the transformation sequencer's fade-out (a Soul Unison's): the US clears the banner (0x0801AFC2 with 0xFF) and the chip name (0x0801A994 with 0xFFFF) once the screen is dark | same in a netbattle: nothing shows then (the souls' Japanese copies match) | the sweep's Japanese copies of souls/*: the same throughout |
| 0x08012000, 0x0801222E, 0x0801232A | 0x08011FD8, 0x08012202, 0x080122FA | the soul change (0x08011FAC), revert (0x080121D8) and break (0x080122C8) reset the slide's step (the object's +0x1F) in the US | JP defect (the engine's `stop_moving`, `revert`, the form break) | the sweep's Japanese copies of souls/*: the same throughout (no soul starts mid-slide) |
| 0x0801433C | 0x08014308 | action 5, the drag's start (0x080142E4, in the players', the viruses' and the game-played navis' tables): Japan faces an actor that isn't a player its own way (0x0800D228) and skips the player's | same for players; a netbattle's one other actor is Chaos Unison's Dark MegaMan | not shown: no recording drags him |
| 0x08014604 | 0x080145DE | a literal pool's words | same (data) | |
| 0x08018C04 | 0x08018BE0 | the chip icons over a navi (0x08018B86, EXE6's `sub_801C082`): the US leaves out an icon above y −32; Japan draws it wrapped (y & 0xFF) | JP defect; not reached: only a navi drawn at the screen's very top puts one there | |
| 0x0801952E, 0x08019544, 0x08019558, 0x0801A25C, 0x0801A2FC | 0x08019502, 0x08019518, 0x0801952C, 0x0801A196, 0x0801A2D0 | "Cstmzing..." (8 tiles) against カスタム中… (7): its box, its slide-in table, its tile maps | language | |
| 0x0801AD22 | 0x0801ACEE | the waiting words' box, cleared 11 tiles wide in the US, 10 in Japan | language | |
| 0x0801B7C4, 0x0801B7FC | 0x0801B790, 0x0801B7C8 | the waiting words' transfer lists (their sizes; verify's tools/exe5/jpassets.py `WAITING`) | language | |
| 0x0801BB30, 0x0801BB34, 0x0801BC60, 0x0801BF70, 0x0801C304, 0x0801C484 | 0x0801BAC0, 0x0801BB00, 0x0801BC28, 0x0801BF30, 0x0801C2C8, 0x0801C3F0 | the banners' table: their glyph pointers | language | |
| 0x0801E1F6 | 0x0801E1B2 | a literal pool's word before the chip table | same (data) | |
| 0x08028724, 0x080291CC, 0x0802936E, 0x0802939A, 0x080293C8, 0x08029412, 0x08029464, 0x08029490, 0x080294D8, 0x0802954C, 0x080295B4, 0x0802962C, 0x08029630 | 0x080286E0, 0x08029188, 0x0802932A, 0x08029356, 0x08029384, 0x080293CE, 0x08029420, 0x0802944C, 0x08029494, 0x08029508, 0x08029570, 0x080295E8, 0x080295EC | the custom screen: its lettering's tile numbers, places and VRAM bases (the chip window's words, OK, the soul button's "UNITE" against "uni son") | language | |
| 0x0802CB32, 0x0802CD52, 0x0802CEEC | 0x0802CAEE, 0x0802CD0A, 0x0802CEA0 | the navi switch's pause-time routines (0x0802CAFC, 0x0802CD1C, 0x0802CEA0: the Battle Chip Gate's) reset the slide's step in the US | not reached (Tango never inserts a gate chip: exe5-map's navi switch) | |
| 0x0802CCB8, 0x0802D09C | 0x0802CC70, 0x0802D04C | literal pools' words | same (data) | |
| 0x0802D60C | 0x0802D5BC | the operation battle's side chip (0x0802DAAC, the side state's +0x2C): the US swaps MeddySP and MeddyDS (0x0ED, 0x0EE) before its bound test | not reached (the operation battle) | |

## 3. The objects and the player's actions

| US | Japan | Kind (content) | What differs | Class | A Japanese console's recording |
|---|---|---|---|---|---|
| 0x080BA530 | 0x080BA2C4 | actor #0x08, the Colonel chips' and CrossDiv's Colonel (colonel/navi) | the command test (0x080BA52E, B and Left held as he comes): the US skips it for his Param3 1, CrossDiv's; Japan's CrossDiv Colonel hears it, plays the command sound (0x8B) and marks his slash, which CrossDiv's X doesn't read | JP defect | jp/regions/crossdiv-command: a sound alone, 0x8B at 1937 (listed: the state matches throughout) |
| 0x080BA5AC | 0x080BA33A | the same | a literal pool | same (data) | |
| 0x080BEEA2, 0x080BEEAC | 0x080BEC32, 0x080BEC38 | actor #0x23 (none) | a byte of its object clamped to 1 to 3 in the US; Japan reads before its table for 0 | not reached (only effect #0x7A spawns it, which the content hasn't) | |
| 0x080C57A0, 0x080C57E6, 0x080C57FA, 0x080C5804, 0x080C5816, 0x080C5854, 0x080C5BB8 | 0x080C5528, 0x080C556A, 0x080C556E, 0x080C5570, 0x080C5574, 0x080C55AE, 0x080C5914 | attack #0x09 (none) | after a hit the US clears its collision region and sets it again 10 ticks later | not reached (the game-played navi of AI index 6's, 0x080F77C0) | |
| 0x080C62C6 | 0x080C6022 | attack #0x0C, Meddy's capsule (meddy/capsule) | its burst's explosion palette by its variant: the US's 0, 4, 1, 2, 3, 3; Japan's 2 and 3 swapped | same in a netbattle (only variants 0 and 5 are thrown) | |
| 0x080C63DC, 0x080C63F2 | 0x080C6138, 0x080C6158 | attack #0x0D (none) | a test of its owner's action moved | not reached (the game-played navi of AI index 6's) | |
| 0x080CBD56 | 0x080CBAB2 | attack #0x37 (none) | a literal pool | same (data) | |
| 0x080CD1BE, 0x080CD1C6 | 0x080CCF1A, 0x080CCF22 | attack #0x3E, BoyBomb's bomb (boybomb/bomb) | in 0x080CD19A, which nothing calls in either ROM: Japan subtracts a whole coordinate where the US subtracts 2 | not reached | |
| 0x080CE288, 0x080CE37A | 0x080CDFE4, 0x080CE0AA | attack #0x46, GrabBnsh's hand (grabbnsh/hand) | in column patterns 0x31 to 0x33 the US faces the hand by the panels ahead in its row, and gives its child the facing | same in a netbattle (every netbattle stage has pattern 0x38) | |
| 0x080CEFE4 | 0x080CED14 | attack #0x4B, TimeBom's countdown (timebom/countdown) | when it can't have collision data the US unregisters it as a field object (0x0800D58E) before it frees it | JP defect; not reached (the collision pool full) | |
| 0x080DA2D0 | 0x080D9FFC | attack #0x9C (none) | a literal pool | same (data) | |
| 0x080DC0F2, 0x080DC1B6, 0x080DC1C2 | 0x080DBE1E, 0x080DBEEA, 0x080DBEFC | attack #0xAF, Satelity's flare (searchman/satelity/flare) | a hit ends a flare whatever its Param1 in the US; Japan's only from Param1 4 | same in a netbattle (Satelity's satellite sends 4) | the sweep's Japanese copies of navis/searchman/satelity-*: the same throughout |
| 0x080DE398, 0x080DE470 | 0x080DE0D4, 0x080DE19E | attack #0xBE (none) | its init sets animation 24 in the US | not reached (nothing but itself spawns it) | |
| 0x080DFE30 | 0x080DFB60 | attack #0xCB, LarkMan's follower | a literal pool | same (data) | |
| 0x080E09B8, 0x080E0A18 | 0x080E06E8, 0x080E0736 | effect #0x06, the status visual (engine/status-visual) | the US sets its panels from its position each tick and hides it off the field (EXE6's too: the rules' `status_visual`); Japan's panel stays (0, 0) and it never hides | JP defect; drawn the same while on the field | jp/chips/0x081-blinder/hit: deviation at 601; jp/navicust/bug-status-strong: at 423 (the whole lab's 40 Japanese copies with a status visual stop there) |
| 0x080E1890 | 0x080E15B0 | effect #0x0F (none) | a call's encoding | same | |
| 0x080E1A44, 0x080E1A46, 0x080E1A5A | 0x080E1764, 0x080E1772, 0x080E1788 | effect #0x10, the navi chips' controllers (their spawner 0x080E19D8, EXE6's `sub_80E192C`) | Roll's chip against an armed AntiRecv: Japan registers the counterattack's dimming itself, no cut-in allowed (0x0800BF16 with 1), as a recovery chip's spring does; the US leaves it to the chip's action, which registers it by the chip's number (Roll's, under 0x170: a cut-in allowed), as EXE6's | US omission? (reported 2026-10-12; nettai has the US) | not recorded |
| 0x080E72E2 | 0x080E700E | effect #0x5C, LifeSync's controller (lifesync/controller) | the US caps the synced HP at each one's max | not reached (a link battle turns LifeSync into a wait) | |
| 0x080EB3CE to 0x080EB3F2 (6) | 0x080EA9DE to 0x080EAA8A | after effect #0x90 | the player's tables, and the Japanese ROMs' routines placed before them | same (data) | |
| 0x080EB428 | 0x080EAAF8 | the player's action table's end | the same | same (data) | |
| 0x080ED34A, 0x080ED35A | 0x080ED06E, 0x080ED07A | action 0x29, Slasher (slasher/action) | when a trap's counter (requests 0x600) or the drive's end (0x80000) takes over, the US ends the invulnerability Slasher set (0xFFFF ticks from when it sees its target) | JP defect (the content's) | not shown: while invulnerable its user takes no hit to spring a trap |
| 0x080ED8C2, 0x080ED8DC | 0x080ED5DE, 0x080ED5F4 | action 0x2C, AntiDmg's counter (antidmg/counter) | as the navi vanishes the US resets the slide's step and puts it on its panel's center (EXE6's and Japan's don't: the content's is EXE6's code) | same in a netbattle: the counter starts from the idle, after any slide, the navi on its panel's center | stages/ice/antidmg-move-end (US) and 16 like it, the Cannon landing at other moments of the slides: the counter waits for the slide's end |
| 0x080EDDF6 | 0x080EDB06 | action 0x2F, GunDelSol (gundels) | the US stops firing once the battle is over | JP defect (the content's `battle.over()`) | not shown: a GunDelS1 deleting a 60-HP navi ends the same on both consoles |
| 0x080EFEA8, 0x080EFEAE, 0x080EFEB6, 0x080EFF04, 0x080EFF80, 0x080EFFC4 | 0x080EFBB0, 0x080EFBB6, 0x080EFBB8, 0x080EFC02, 0x080EFC7C, 0x080EFCB4 | action 0x30, a dark MegaMan's last stand (rules/emotion/dark_survival) | the US's take-over clears the flags 0x101C40 and 0x110 and the slide's step (Japan's 0x40 alone), and its fade-in shows the HUD of the netbattle's last turns (0x0800916A) | JP defect (the content's) | the sweep's Japanese copies of dark-survival/*: the same throughout |
| 0x080F01E8 | 0x080EFED4 | action 0x58, HotBody | the literal pool (the take-over's two words) | same (data) | |
| 0x080F3392, 0x080F378A, 0x080F58A8, 0x080F8066, 0x080F8432, 0x080F924E, 0x080F92A6, 0x080F92D8, 0x080F93A0, 0x080F93AC, 0x080FB386, 0x080FB3E0 | 0x080F3076 on | the game-played navis' rows 1, 3, 7, 8 and 10 (0x080F228C) | facings, timers and positions | not reached (the story's navis, the operation battle's) | |

## 4. The chip records

| Chip | Field | Class | A Japanese console's recording |
|---|---|---|---|
| 282 of the 368, both versions | +0x18 (a halfword: the library's sort key, by language) | language (not a battle's) | |
| the rest, every field and every picture, icon and palette | the same | same | |

The two versions' records differ from each other only where compat says (exe5-map), in both regions alike.

## 5. The recordings

The lab's `regions/crossdiv-command` (gen_exe5.py's `region_scenarios`, on US consoles: every frame matches) and
gen_exe5_jp.py's `REGIONS`: its Japanese copy and those of chips/0x081-blinder/hit, navicust/bug-status-strong and
custom/run-message-wait. On Japanese consoles each stops at its place, a listed deviation (deviations.rs, `JP5_*`):
the status visual's panel twice (601, 423), the no-running message's pace (its sound at 373, its screen at 438), and
CrossDiv's command, which a sound alone shows (EXE5's sound comparison honors a deviation's `sounds`, and a
deviation with no state frame, `SOUND_ALONE`, isn't asked of the state replay).

## 6. Found on the way

A trap that catches a hit zeroes the hit; EXE6's also drops the navi's pending requests (0x301BE), EXE5's (0x08029AE0)
and EXE4's (0x080230A2) don't. The engine dropped them in every game, so a move ending on ice as AntiDmg caught a hit
didn't slide on (of 16 US recordings of AntiDmg catching a Cannon around a navi's steps on ice, 8 stopped there). The rules' `anti_traps` now say
it (`drops_requests`: EXE6's 0x301BE, EXE5's and EXE4's 0); stages/ice/antidmg-move-end records it.

## 7. US omission? Roll against an armed AntiRecv

Reported to the coordinator 2026-10-12, nothing ported. When a Roll chip meets the other side's armed AntiRecv, its
controller's spawner springs the trap and spawns AntiRecv's counterattack in place of Roll (both regions). Japan then
registers that counterattack as its side's dimming with no cut-in allowed (0x080E1A44: 0x0800BF16 with 1), as a
recovery chip's spring is registered (`kinds::heal`); the US release removed the call, so the chip's action (0x41)
registers it as it does a navi chip's controller, a cut-in allowed for chips under 0x170, Roll's among them. EXE6's
US code is the US EXE5's. On a US console the other side may cut in on the counterattack meant for the Roll user; on
a Japanese one it can't.
