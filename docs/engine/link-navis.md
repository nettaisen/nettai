# Link navis' levels

A link navi has a level, 0 to 14. In a battle the level only picks rows of three tables (the link navi's chip
bonus, the charged chips and ChargeMan's Fire charge: standard-chips.md, "Action 0x0A"). It also decides the
navi's stats, but not in the battle: the PET's reload (`reloadCurNaviBaseStats_8120df0`) gives the navi its
stats from its level whenever the navi or its level changes, the save keeps them, and the battle reads them
as they are (the init exchange's NaviStats). So yes, the level gives HP: a level adds 20 to 1200 to the
maximum HP, the base HP before it coming from the story's progress (300 to 800). It also gives the buster's
levels, the custom and Mega levels, abilities and, for ProtoMan, the B+Back special.

nettai has the tables in each navi's definition (`levels` in content/bn6/navis/*/navi.luau) and the reload in
nettai-match (`link_navis`), which fills a side's stats from its level: a match file's stats block is what
differs from them, and the editor fills them in as the level or the navi changes (§5).

Addresses are US Falzar's. The four ROMs (US and Japanese, Falzar and Gregar) have the same tables, byte for
byte (`pt_8121200`, `off_8120F44`, `byte_81211B4` relocated: Gregar US 0x08122FDC, 0x08122D20, 0x08122F90;
Falzar JP 0x0812904C, 0x08128D90, 0x08129000; Gregar JP 0x0812AE14, 0x0812AB58, 0x0812ADC8).

## 1. Where the level lives

| What | Where (EWRAM; a save image the same offset from 0x02000000) | Read by |
|---|---|---|
| the operated navi, 0 MegaMan, 1 to 11 the link navis (the navi numbers) | `GameState+0x01` (0x02001B81) | `GetCurPETNavi` |
| a link navi received: event flag 0x163 (`EVENT_163`) | 0x02001CB4, bit 0x10 | everything below |
| the navi code received: `0x141 + 15 · navi + level` | `S2001c04+0x30` (0x02001C34), a word | `sub_8121198`, `sub_81276E4` |
| the operated navi's stats: MegaMan's block, then one shared by every link navi | 0x020047CC, 0x64 bytes each (`sub_801401E`: `CurPETNaviToNaviStatsIndexTable`, 0 for MegaMan, 1 for the rest) | the reload, the init exchange |

`sub_8121198` gives the level: the code less the operated navi's base (`byte_81211B4`, a halfword a navi: 0x141
+ 15 · navi). Nothing checks the result; a code of another navi gives a level past the tables.

## 2. How the level is set

The console never raises a level. It takes it from a navi code it receives over the link cable
(`sub_809CE14`: `sub_803F740(4)` hands over the code; `sub_809CC60` takes it): the code must fall in one of the
seven ranges of 15 that `byte_809CCF8` starts (Falzar: MegaMan 0x141, SpoutMan, TomahawkMan, TenguMan, GroundMan,
DustMan, ProtoMan; Gregar: MegaMan, HeatMan, ElecMan, SlashMan, EraseMan, ChargeMan, ProtoMan), so each game takes
its own five Cross navis, ProtoMan and MegaMan. Taking one, it stores the code (§1), sets event 0x163, remembers
the navi operated until then (`writeCurPETNaviToS2001c04_Unk07_80010c6`, `S2001c04+0x07`) and operates the code's
navi (`byte_809CD18`). Then (`sub_809CDD4`) a link navi's reload (§3), or for MegaMan `sub_809CE40` (his block
takes the folder bytes of the block operated before: +0x2D to +0x30, +0x56 to +0x5B); then
`reloadCurNaviStatBoosts_813c3ac` (§4); then the folder against the new stats (`sub_8120D10`, its messages 0x32,
0x34 or 0x33). The same screen's other steps operate MegaMan again (`sub_809CB88`) and clear event 0x163
(`sub_809CD60`).

The PET's navi screen shows the level (`sub_81273B4`: "Lv." the level + 1, or the MAX text at 15) and what it
gives (`sub_81276E4`: the level's script, §3).

The level reaches a battle in the init exchange: `sub_800B144` writes the level (`sub_8121198`) at +0xC0 of the
console's block when event 0x163 is set, else 0xFF, and `battle_copyStructsIncludingBattleStats_800b2d8` copies
each console's to `dword_203CFA0` (nettai: `PlayerSetup::navi_level`). The stats go with it as the save holds them
(the block at +0x0C, `sub_801401E(GetCurPETNavi())`; a link battle sets +0x21, the Beast Out counter, to 3).

## 3. The reload (`reloadCurNaviBaseStats_8120df0`)

It runs when a link navi is received (§2), when a cutscene changes the navi (`CutsceneCmd_change_navi_maybe_80382fe`,
then `reloadCurNaviStatBoosts_813c3ac` and `SetCurNaviHPToFull`), when a scene switches it (`sub_809C01C`, the
navi of a `byte_809BF24` entry), and when a cutscene's `navi_80340F6` finds event 0x163 clear (MegaMan operated
again). Branch
by branch:

1. It reads what the save keeps (`byte_81210C8`) from the block of the navi operated before (`S2001c04+0x07`): the
   folder (+0x2D), its Regular chips (+0x2E, +0x2F, +0x30), the Regular memory (+0x09), the tag chips (+0x56 to
   +0x5B), the Mega, Giga and custom levels (+0x0B, +0x0C, +0x0A) and the HP (+0x40).
2. MegaMan operated and event 0x163 clear: those bytes but the HP are written into MegaMan's block, and it ends.
3. Else the operated navi's block is made fresh (`init_8013B4E`: `initNaviStats_WithDefaultStatsMaybe_8013438`
   and the navi's row of `byte_80210DD`; nettai's `NaviStats::fresh`), and the bytes read are written back into
   it: the HP, and all the others but the Mega, Giga and custom levels when event 0x163 is set.
4. Event 0x163 clear (a link navi operated without one received): MegaMan ends here; another navi's base and
   maximum HP (+0x3E, +0x42) are its row of `off_8120F44` at the story's progress (`sub_8121108`), and, in the
   real world (map group below 0x80), its HP (+0x40) too.
5. Event 0x163 set: the base and maximum HP the same (MegaMan's row is zeros), then the level's script
   (`sub_8121154`, below), then in the real world the HP is the maximum.

**The story's progress** (`sub_8121108`): the index (0 to 6) of the highest set of event flags 0x400, 0x500,
0x600, 0x800, 0xA00, 0xC00 and 0xE00, 0 for none. **The base HP** (`off_8120F44`, seven words a navi): 300, 300,
300, 400, 500, 600, 800 for every link navi but EraseMan (700 last); zeros for MegaMan.

**A level's script** (`sub_8121154`): `pt_8121200[navi]` points at the navi's table of 15 scripts, by level. A
script is (op, value) word pairs ending at 0xFFFFFFFF; each op is a byte offset of `off_81211D0`, whose routine adds
the value to a halfword of `word_200DCF0` (zeroed first, `sub_8121144`) or sets a flag there. `sub_8123208` then
applies them to the operated navi's block:

| Op | Routine | Applied |
|---|---|---|
| 0x00 | `sub_8123198` | the maximum HP (+0x42) += the value (not the base HP, +0x3E) |
| 0x04 | `sub_81231A4` | Attack (+0x01) += it, then 4 at most |
| 0x10 | `sub_81231C8` | Rapid (+0x02), likewise |
| 0x14 | `sub_81231D4` | Charge (+0x03), likewise |
| 0x08 | `sub_81231B0` | the Mega level (+0x0B) += it, then 10 at most |
| 0x0C | `sub_81231BC` | the custom level (+0x0A) += it, then 8 at most |
| 0x18 | `sub_81231E0` | the B+Back special (+0x07) = routine 0x30 (the reflecting guard) |
| 0x1C | `sub_81231E8` | AirShoes (+0x1C) |
| 0x20 | `sub_81231F0` | FloatShoes (+0x1B) |
| 0x24 | `sub_81231F8` | SuperArmor (+0x23) |
| 0x28 | `sub_8123200` | the B+Back special = routine 0x34 (the guard that only guards), written after 0x18's |

Each of the five levels is clamped even when the script adds nothing to it. The scripts are the totals for their
level, not increments over the level below. What they give (HP; Attack, Rapid, Charge; custom; Mega; abilities):

| Navi | Level 0 | Level 7 | Level 14 | Throughout |
|---|---|---|---|---|
| MegaMan | +20 | +160; 1, 1, 1 | +300; 3, 3, 3; +2; +2 | |
| HeatMan | +100 | +450; 1, 0, 1; +1 | +1200; 3, 2, 2; +1; +1 | FloatShoes |
| ElecMan | +100 | +450; 1, 1, 0; +1 Mega | +1100; 3, 2, 2; +1; +2 | FloatShoes |
| SlashMan | +40 | +320; 1, 2, 0; +1 | +1000; 2, 4, 1; +2; +1 | |
| EraseMan | +100 | +450; 1, 2, 1; +1 Mega | +800; 3, 2, 3; +1; +2 | |
| ChargeMan | +60 | +480; 1, 1, 2; +1 | +1000; 3, 2, 4; +3; +1; SuperArmor | |
| SpoutMan | +100 | +450; 1, 1, 0; +1 | +1100; 3, 2, 2; +1; +1 | |
| TomahawkMan | +100 | +450; 1, 1, 0; +1 Mega | +1000; 4, 2, 2; +1; +2 | SuperArmor |
| TenguMan | +40 | +320; 1, 2, 0; +1 | +1000; 3, 4, 1; +2; +1 | FloatShoes, AirShoes |
| GroundMan | +60 | +490; 1, 1, 1; +1 Mega | +1200; 4, 2, 2; +1; +2 | SuperArmor |
| DustMan | +100 | +450; 1, 1, 1; +1 | +1200; 2, 3, 2; +3; +1 | SuperArmor |
| ProtoMan | +100; B+Back 0x34 | +430; 1, 1, 1; +1 | +600; 4, 3, 3; +2; +1; B+Back 0x30 | B+Back 0x34 to level 9, 0x30 from 10 |

Every level of every navi is in its definition (`levels.by_level`); gen-content checks them and the base HP against
the ROM.

## 4. After the reload (`reloadCurNaviStatBoosts_813c3ac`)

MegaMan operated: his NaviCust is compiled (`sub_813C458`, docs/design/navicust.md; its reset, `sub_8136C24`, keeps
his base HP, +0x3E), and with event 0x163 set the level's script runs over the result (`sub_8121154`): a MegaMan
received from a navi code has his NaviCust's stats and his level's gains on them. Then the HP: in the real world,
if the PET's navi is out (event `EVENT_PET_NAVI_ACTIVE`), MegaMan's block and the operated navi's take their maximum;
in the internet, each block's HP is cut to its maximum.

## 5. nettai

- **Data**: each navi's `levels` (content/bn6/navis/*/navi.luau, type `NaviLevels` in core.d.luau): `base_hp`, its
  row of `off_8120F44`, and `by_level`, each level's script summed (`hp`, `attack`, `rapid`, `charge`,
  `custom_level`, `mega_level`, `super_armor`, `float_shoes`, `air_shoes`, `back_special` by weapon). Read into
  `NaviData::levels`; no battle reads it.
- **The reload**: nettai-match's `link_navis::reloaded` is §3's link navi branches (steps 1, 3, 4 and 5, from the
  block the save had: the navi's own when the level changes, the one switched from when the navi is switched to),
  then §4's HP for a link navi; `link_navis::add_level` is the level's script (`sub_8121154`, `sub_8123208`).
  `Reload` says the level (none: event 0x163 clear), the story's progress and whether the PET is in the real world;
  a match assumes the game cleared (progress 6, as the chip lab's saves and Tango's have it) and the real world.
- **Why not a BN6 system**: the reload is the PET's, between battles. The rules framework calls a system inside a
  battle, on a round's setup, whose stats are the save's already (a recording's carry the level's), so a
  `round_setup` hook would add the level a second time; and a match needs the stats before any battle exists (a
  file read, the editor, a netplay offer). The tables are BN6's content; the routine stays a small Rust function
  beside `NaviStats::fresh`, the other save-side routine, until BN6's systems own a player's setup (§2.4 of
  docs/design/rules-in-luau.md plans a `link-navis` system).
- **Match files and the editor**: a side's stats block is what differs from its navi's stats as a save gives them
  (`Side::save_base`: a link navi's reload at its level over its fresh stats). The editor fills a link navi's stats
  in when its level changes (the reload over the side's own stats: what the save keeps stays) and when the side
  switches to a link navi (the reload over the stats of the navi switched from, as the game's switch carries them);
  an edited stat stays, and the stats pane says what the level gives where they differ.
- **Not modeled**: MegaMan's level. The game adds it over his NaviCust (§4); nettai's NaviCust is compiled as the
  round is set up (the navicust system's `round_setup`), and the editor offers a level only to link navis. A MegaMan
  side's `level` still goes to the battle (`navi_level`), where nothing of MegaMan's reads it. The story's progress
  and the internet's HP are the reload's parameters but not a match's. §3 step 2 (MegaMan with event 0x163 clear)
  and step 5 for MegaMan (a cutscene's switch to him with a code of his received: base HP 0, which his NaviCust's
  reset keeps) have no counterpart.

## 6. Verification

- gen-content checks every navi's `levels` against `off_8120F44` and `pt_8121200` (the scripts summed by op, the
  B+Back special by compat's weapon routines).
- nettai-match's tests: each link navi at level 14 is Tango's save editor's table (taken from the game's own
  equips: tango-gamesupport-bn6-dataview's `link_navis.rs`, every value); ProtoMan at level 5 is the chip lab's
  `navis/navi-11-stepswrd/level-5`; what the reload keeps, the progress, the internet's HP; the clamps; a match file.
- The verification workspace's `trace-tests --test link_navis`: every recorded chip lab side with a link navi (210:
  each navi at level 14 from Tango's editor, ProtoMan at 5) has the reload's stats at its level, every byte the
  engine models but the console's version byte. One scenario (`navis/navi-05-volcchrg/charge-family-chip-level-1`)
  pokes the level alone and keeps level 14's stats, a save the game doesn't make: listed apart. Tango's four raw
  saves operate MegaMan (event 0x163 clear); their link navi block is a residue (navi byte 0), so they check
  nothing.
- **Unverified**: no recording has a link navi below the cleared game's progress, in the internet, without event
  0x163, or MegaMan with a level; the scripts of levels other than 5 and 14 are checked against the ROM only.
