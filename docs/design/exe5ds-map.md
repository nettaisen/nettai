# EXE5 DS against EXE5: the routine map and the party battle

What of the GBA EXE5's battle code Mega Man Battle Network 5: Double Team DS has, and what the DS adds: its
**Party Battle System** (the DS's TeamBattle, which we call **party battle**: "Team Battle" is the GBA's Battle Chip
Gate netbattle, exe5-map.md §8), with its **Double Attacks**. The user, 2026-10-20: bring the party battle into
nettai's EXE5, Double Attacks included; reuse the GBA EXE5 assets, extract nothing from the DS, ignore its music.
This is step 1: the map and the reading. It ports nothing.

The map is made by the verification workspace's `tools/exe5ds` (`dsmap.py`, README there) from the ROMs alone.
There is no DS disassembly. The DS's routines are found by discovery and named by their counterparts in the GBA's
US Team ProtoMan (BRBE). BRBE's are named by EXE6's names where exe5-map.md's map pairs them; `~` marks a similar
pair, `exe5_` an EXE5 routine with no EXE6 counterpart. Addresses are the US DS's (A5TE) unless said otherwise.

How sure each fact is:
- **read**: read in code;
- **observed**: seen on a running console;
- **open**: not settled.

Nothing here is observed yet: §9 lists what a console should confirm.

## 0. Summary

- **The DS's battle is the GBA's code, relinked.** It is the same Thumb code, assembled again for the DS: the
  ARM9 holds the engine (flow, objects, collision, the custom screen, the HUD), and overlay 423 holds the battle
  objects (chips, navis, viruses).
  - Of BRBE's 3,577 battle-object routines, the DS has 88.7% the same (modulo relocation), 0.5% with other
    constants, 9.3% similar, 0.3% differing and 1.1% absent.
  - Of the engine and flow routines (0x0800–0x0802), 45% are the same, 28% similar and 19% absent.
  - The menus and the field are mostly the DS's own.
- **What moved is mechanical** (§2):
  - The DS keeps no toolkit in r10: about 1,100 places call a getter instead.
  - Sound, DMA and BIOS calls go to ARM code.
  - Every structure moved in RAM, and the object pools are on the heap.
  - The pools are larger: 32 actors (the GBA has 16), and records 4 bytes longer (0xDC and 0xCC).
  - The chip record is 0x28 bytes (the GBA's is 0x2C).
- **The party battle** (§4):
  - Each side brings **MegaMan and up to two team navis**, each with its own 0x60-byte NaviStats block.
  - The player switches with X or Y or the bottom screen's two buttons, up to MegaMan's switch limit (3, up to 5
    with the DS-only programs NavChg+1/+2).
  - A switch is taken two ways. **In battle**, the navi's input handler starts a new action (0x59, with dimming).
    **On the custom screen**, the pick is made there and done at the turn's start by the GBA's navi switch steps,
    rewritten.
  - HP is kept per member. Soul Unison ends on the way out. A team navi enters at the normal emotion.
  - A deleted team navi gives the field back to MegaMan. The side loses when MegaMan is deleted.
  - Benched members join in: **supports** (seven per-frame handlers) and ToadMan's rescue at 0 HP.
  - **PartyCust** (§4.10), the team navis' own customizer, is a gauge of thirteen programs. The programs add max HP,
    buster attack, a bonus to the navi's own attacks, and the support flag to that navi's stats record. That
    record crosses the link inside the partner stats.
- **Double Attacks** (§5): a switch out of **Full Synchro** to the partner a pair table names starts that pair's
  Double Attack, right as the swap ends.
  - There are twelve, one per pair: six with MegaMan, six between team navis.
  - Each has a chip record (0x160–0x16B, unused placeholders on the GBA), a damage row by navi level, and a
    director object that brings both navis on as actors, attacks and warps them out.
  - All of it is new DS Thumb code built on the GBA's helpers: the hitbox spawner, Vulcan's shots, the sword and
    buster attachments.
  - **Every sprite id they use is one the GBA has art for, and every sound but one is a GBA sound.** Sound 0x2F
    (one Double Attack) is empty on the GBA, and the switch's navi voices are DS-only. That the DS's art at those
    ids is the GBA's is not yet compared pixel by pixel.
- **The DS's other battle changes** (§6) are catalogued only. The recommendation: GBA EXE5 plus the DS's party
  rules.
- **The oracle** (§7): every RAM address oracle-trace reads for EXE5 has a DS counterpart, the battle's frame
  routine is 0x0205F970, and the switch rides the link's input packet as a slot number.
- **The plan** (§8): fourteen steps, about two months of work in all. The largest pieces are the oracle on
  melonDS and the twelve Double Attacks.

## 1. The cartridge and the method

**The cartridge** (`ds.py`, read). The US and Japanese DS ROMs have the same layout:
- **ARM9:** 0x160D78 bytes at 0x02000000, uncompressed. Its text ends at 0x0215DEE0, where the SDK's autoload
  data begins.
- **Autoloads:** the ITCM (0x01FF8000, 0x620 bytes) and the DTCM (0x027C0000, 0x2860 bytes).
- **Overlays:** 425 of them, all uncompressed, loading at six addresses:

| Load address | Overlays | What |
|---|---|---|
| 0x021CE8A0 | 0–244 | small overlays (scripts, by their size) |
| 0x021CF740 | 245–266 | |
| 0x021D0600 | 267–407 | |
| 0x021D2D00 | 408, 409 | data, 0xAEE0 bytes each, differing in 539 bytes; no code calls into them |
| 0x021DDBE0 | 410–422 | the menus' and screens' code: 413 holds the comm menu's GBA routines, and the bottom screen's battle callbacks are in this slot |
| 0x021F6860 | 423, 424 | **423 the battle objects** (0x34BE0 bytes); 424 the field's objects |

**Where the GBA's code went** (`dsmap.py where`): 4,275 BRBE routines are found verbatim in the ARM9, 1,964 in
overlay 423 (BRBE 0x080B8C18–0x0810F4C4, the battle objects), 710 in 424 (BRBE 0x080AD–0x080B7 and 0x0812, the
field's objects) and almost none elsewhere. The map's view of the battle is the ARM9 + ov408 + ov423; a battle
runs with ov423 in its slot (its objects' kind tables are the ARM9's pointers into it), and what loads it is not
read (the overlay-id tables at 0x0215D600 hold the ids by group; the SDK's FS_LoadOverlay caller is unread).

**The method** (`dsmap.py map`) is exe5-map.md §1's, BRBE as the "from" ROM:
- BRBE's routines come from gba.discover; BRBE's code ends before 0x08150000.
- The DS's come from a Thumb discovery: BL targets, code pointers and tables of code pointers anywhere in the
  view's code.
- **ARM against Thumb.** The DS's ARM code (the SDK and the port's C) is never a counterpart, and a GBA routine
  rewritten in ARM shows as absent. The table's `arm calls` column counts each DS routine's calls into ARM.

Before comparing, both sides' idioms are normalized, so that relinked code compares the same:
- a BLX pair (a call into ARM) is a BL;
- `mov lr, pc; bx rN` is `blx rN`;
- a BIOS `swi` is a call (the DS calls its own division, 0x02020818);
- `movs rN, #a; adds rN, #b` is the literal a+b. The DS loads constants over 0xFF as literals. **Sound ids are
  the GBA's numbers**: BRBE's `movs r0, #0x46; adds r0, #0xFF` is the DS's `ldr r0, =0x145`.

Two further rules:
- A `consts` pair whose only differences are branches out of the routine (tail calls) is `same`.
- Pointer tables that two same routines load, when they agree with the rest of the map, outrank a shape pairing.
  Without this rule the navi switch's settle step was mis-paired: shape alone took the knockout's.

| Status | Meaning |
|---|---|
| same | identical instructions modulo relocation (calls, address literals, tail branches) |
| consts | identical shapes; some immediates or constant literals differ |
| similar | a counterpart whose shapes align with difflib's ratio ≥ 0.5 (`toolkit` in the notes: only the toolkit rewrite differs) |
| differs | a counterpart with a ratio below 0.5 |
| absent | no counterpart found (or rewritten in ARM) |

`same` and `consts` are exact facts about the code; `similar` and `differs` are pairings, to be read with
`dsmap.py diff`. The Japanese DS (A5TJ) maps the same: its statuses agree with the US DS's for all but five
routines (three HUD routines, `sub_801E474`, `sub_801E792`, `sub_801EB18`, are the same code in the Japanese
DS and similar in the US one: the lettering, presumably). Its battle RAM is the US DS's moved −0x72B0
(0x021Bxxxx) and −0x7260 (0x0216xxxx).

## 2. The map

BRBE's routines by area (the DS's statuses):

| BRBE area | Routines | same | consts | similar | differs | absent |
|---|---|---|---|---|---|---|
| battle objects: chips, navis, viruses (0x080B8–0x080EF) | 3,577 | 3,174 (88.7%) | 18 | 334 | 10 | 41 |
| battle helpers, link (0x080F–0x0811) | 1,714 | 1,560 (91.0%) | 4 | 117 | 4 | 29 |
| engine, battle flow, objects (0x0800–0x0802) | 2,567 | 1,150 (44.8%) | 88 | 723 | 114 | 492 |
| menus, field, text (0x0803–0x0809) | 2,510 | 1,190 | 209 | 479 | 22 | 610 |
| field objects (0x080A–0x080B7) | 886 | 100 | 45 | 78 | 11 | 652 |
| menus, comm (0x0812–0x0814) | 2,152 | 192 | 21 | 109 | 18 | 1,812 |
| IWRAM code | 88 | 16 | 0 | 25 | 2 | 45 |

Of the similar battle objects, 22 differ only by the toolkit rewrite. In the rest, the differences are calls the
DS adds (`why.py`), the commonest:
- NaviStats reads by alliance (12 routines): the party's per-navi rules;
- the battle effects (4);
- object flags (3);
- sounds (3);
- a few panel and collision checks.

The absent ones in the engine's range are mostly the GBA's link layer, its IWRAM routines and the gate.

## 3. What the DS changed mechanically (read)

- **The toolkit.** It is at 0x021701D4 (JP 0x02168F74).
  - Its first 15 words are copied from 0x020D9A2C by 0x0202B300 (BRBE's `SetPrimaryToolkitPointers` table,
    0x08006020).
  - Its +0x3C–+0x94 are written by an ARM routine, 0x020214BC, called from the battle's loop. BRBE adds its
    offsets to the save block instead; the Thumb copy of that routine, 0x0202B318, has no caller.
  - No code keeps it in r10. Where the GBA reads `mov rN, r10`, the DS calls 0x0202B350 (the toolkit in r0),
    0x0202B35C (in r1) or 0x0202B368 (a toolkit word).
- **ARM callees:**
  - `PlaySoundEffect` is 0x02018F4C, ARM. Sounds play through NitroComposer: there is no m4a, and no GBA sound
    queue.
  - The division routine is 0x02020818.
  - The copies are `CopyWords` 0x0206C3CC and `CopyHalfwords` 0x0206C398 (Thumb), and the SDK's ARM fills.
- **Interworking returns:** a leaf's `mov pc, lr` is often `push {lr} ... pop {pc}`.
- **Objects:** the pool table is at 0x02129438, BRBE's 0x080030F0. The pools are allocated on the heap, through
  pointers:

| Pool | GBA (BRBE) | DS |
|---|---|---|
| actors (type 1) | 16 × 0xD8 at 0x0203B1F0 | 32 × 0xDC, base at [0x021B8280] |
| attacks (type 3) | 32 × 0xD8 at 0x0203CA30 | 32 × 0xDC, base at [0x021B828C] |
| effects (type 4) | 32 × 0xC8 at 0x02036EF0 | 32 × 0xCC, base at [0x021B829C] |

  Each record starts 0x10 into its slot, as on the GBA. The four extra bytes are not located (open; the shared
  code's fields are at their GBA offsets, the same code reading them). Tango's "unit" records (0x022D6498,
  stride 0xDC) are the actor pool's first slots, on the heap.
- **The AI and collision records** are allocated the same way: pointers at 0x021B8288 (AIData) and 0x021B8284
  (collision).
- **The chip record** (`getChip8021DA8`'s counterpart 0x0200A460) is **0x28 bytes**, at 0x0203E9E8, 0x1A8 chips
  (the GBA's count).
  - +0x00–+0x1F are the GBA's fields, with two moves: the GBA's +0x17 is the DS's +0x0F, and +0x16/+0x17 carry
    DS values for a few chips.
  - The GBA's three ROM pointers (+0x20 icon, +0x24 image, +0x28 palette) are one RAM pointer and an id.
  - Per-chip differences are in §6.
- **NaviStats** is 0x60 bytes, as the GBA's: two at 0x021B8C64.

## 4. The party battle (read)

### 4.1 The party

- **The team words** (Tango's team block) sit in a struct at 0x0216F284:
  - +0xC and +0x10: the save's own team, index = id − 0x254, 0 empty (ARM 0x02094E4C);
  - +0x14 and +0x18: Navi Select's picks, index = id − 0x276, 0x276 none (0x02094DB4).
  - The index is the navi's number, NaviStats +0x29: 1 ProtoMan … 12 ToadMan, exe5-map.md §15.16's order.
  - Tango's 0x22 is 0x276 − 0x254.
- **Who joins** (0x02097DD4: a partner count, 0 to 2, and their ids):
  - battle effect 0x800000: none;
  - a link battle (effect 8): Navi Select's two, if effect 0x200000 is also set;
  - offline modes 2–5 and 7: none;
  - offline mode 6: one scripted partner (0x0216CF08+0x49);
  - other offline modes: the save's team, unless event flag 0x28D is set.
  - Team navis are 1–12. MegaMan is member 0, light or dark by the save.
- **Per-member stats** (0x02097BDC):
  - each side has three 0x60-byte blocks, at 0x021B9A5C + side·0x120 + member·0x60 (the accessor 0x0206E1BC, the
    counterpart of BRBE's `GetBattleNaviStats2034A60Addr`, where the GBA kept one reserve);
  - member 0 is a copy of the side's active NaviStats; members 1 and 2 are the partners' base stats;
  - +0x58 is the member's slot.
- **The per-side party block** at 0x021B85E8 + side·0x24 (0x02004880):
  - +0xB: the ToadMan rescue used (§4.7);
  - +0xC+slot: a per-member busy flag. Set, it refuses a switch to that member (0x02004B70, 0x02004B80);
  - +0x15: the member count;
  - +0x18: a bitmask whose meaning is open (0x02004E68 to 0x02004E90).
- **Over the link:**
  - 0x02097B84 builds a send block at 0x021B8F94: magic 0x99887766, the partner count, and each partner's
    0x60-byte base stats (`sub_801401E`: the base-stats table at toolkit+0x74, by navi).
  - It goes as one of the 16-byte blocks the battle's input packet carries (§4.2); the received blocks land at
    0x021B965C and 0x021B975C.
  - So each console sends its partners' stats, as the GBA's init exchange sends the PET navi's.

### 4.2 The switch input

- **The bottom screen's party panel** is an ARM module, 0x02035B00–0x02037150.
  - Its controller (0x10F8 bytes, allocated by 0x02036654; the pointer at 0x02167100) has:
    - +0: active (only with at least one partner);
    - +1: battle mode 6 (unlimited switches);
    - +2: the side;
    - +4: the active member;
    - +5 and +6: the two buttons' members ((1, none) or (1, 2));
    - +8–+0xC: five pips (0xFE available up to the limit, 0xFF beyond; a switch logs the navi's id in the next);
    - +0xD: the switch limit;
    - +0xE: the switches used;
    - +0xF: the request, member + 1.
- **Keys** (the panel's state 0x02083254, on the newly pressed halfword 0x0215F3D6):
  - **X requests button +6, Y button +5.**
  - Otherwise a touch on the panel's buttons (hit codes 0x50 and 0x51) requests +5 or +6.
  - Anything else clears the request. So an X/Y request lasts a frame, and a touch repeats while held.
- **The panel refuses:**
  - outside mode 6, when used ≥ limit (sound 0x69);
  - an empty button;
  - a member at 0 HP (sound 0x69).
  - It also needs +2 to equal the local alliance (BattleState +0xD). Init writes 0, so how player 2 of a link
    battle switches is **open** (a console should show it).
- **Into the battle:** every frame the request is packed into the battle's input packet (0x0200A328, 0x18 bytes):
  - magic 0x4442;
  - +2 the keys;
  - +4 a block index;
  - +5 **the microphone's level**: 0x0215F208 is the mic sampler's state (0x02019228 starts sampling into a
    0x800-byte buffer, 0x02019180 averages it into +0x3E);
  - +6 copied to BattleState +0x14/+0x15;
  - **+7 the switch request** (0x020370C8);
  - +8–+0x17 one 16-byte block.

  The packet lands in each side's input record (0x021B8368, 0xC bytes a side; the GBA's are 8):
  - +2 held, +4 pressed, +6 released;
  - **+8 the switch slot**, +0xA its newly pressed edge (0x020022E8);
  - +9 the packet's +5, the mic level. Overlay 423's 0x022261A4 (BRBE 0x080EB1B4's code, the same) sets
    battle flag 4 (dimming) and a side's flag 0x800/0x1000 when it reaches 0x1E: a shout into the microphone
    starts something. Which object it is and what it does is open; on the GBA the same code reads +9 of an 8-byte
    record, the next record's +1 (what fed it there is not read).

  **So the switch is input**: a slot number in each tick's input, not a key bit.

Two places read it, so a switch is taken in battle or on the custom screen.

### 4.3 In battle

- **Every frame** the navi's input copy (0x0203C180, BRBE `sub_8012DFC`'s counterpart) copies input record +8
  into AIData +0x70 and +9 into +0x71, unless the battle is over.
- **The navi's input handler** (0x0203C3D8, `sub_8012FC8`'s counterpart) runs while the battle isn't dimmed. At
  its end (0x0203C5F6), outside battle mode 6, it requests a switch when all of these hold:
  - `sub_802DE5C` (0x0200E064) passes: the navi is on the field, no switch or knockout is pending (AIData +0x44
    0x0C000000), and AIData +0x48 has neither 0x1000 nor 0x2000;
  - AIData +0x70 is set, and the slot isn't the active one (+0x58);
  - the member isn't busy (0x02004B80; refused with sound 0x69).

  It then stores the slot in AIData +0x72 and sets AIData +0x44 bit 0x04000000. In mode 6 the same request calls
  the partner for a Double Attack without a swap (§5).
- **The live pause handler** (0x020621A0 onward) turns bit 0x04000000 into AIData +0x48 0x1000 and **action
  0x59** (0x0202C67C): the in-battle swap (§4.6).

### 4.4 On the custom screen

- **The custom screen's chip picking** (state 4) reads the request too.
  - 0x02063038 calls 0x0200C82C every tick: the counterpart of BRBE's 0x080259A0, the gate's navi-chip query of
    the GBA, which the DS replaced with 0x0201352C reading input record +8.
- **0x0200C82C refuses:**
  - battle modes other than 0 and 8;
  - a navi number above 12;
  - the screen's current navi;
  - offline, a switch to MegaMan while a game-state byte (toolkit+0x3C → +6) is 0x30–0x3F (open).

  There is no "used this battle" test, which the GBA's gate path had.
- **Accepting** (state 0x40, its sub-state 4, 0x0206391C):
  - saves the active navi's HP into its member block;
  - sets the screen's navi and shows the member's HP;
  - **clears the chips already picked**;
  - rebuilds the screen for the navi (`sub_802812C`);
  - swaps the panel's button and member, and counts the switch;
  - plays sound 0x8D (`cross-change`) and returns to state 4.

  Every accepted press counts against the limit, even a switch back.
- **OK** (the hand builder, 0x0200BED8): when the screen's navi differs from the active NaviStats +0x29, it writes
  the transform record's navi (0x0200DEDC: 0x021B82F0+4, the counterpart of `sub_802DCD8`, which has no caller in
  EXE6). It also marks the navi in 0x021B8300+0xE and counts team navis in 0x021B8300+5.
- **Over the link** each console consumes its own side's request, and the transform record (the GBA's exchanged
  record) carries the result.

### 4.5 The switch limit and the DS programs

- **The limit** is MegaMan's base stats +0x17, 3 (where the 3 is written is open), re-read every round (0x02036D2C).
  A switch is counted where it is accepted: on the custom screen at state 0x40 (0x020639DE), in battle by action
  0x59's change step (0x0202C7EE).
- **The three DS-only NaviCust programs** are in the program effect table at 0x020D7CB4 (51 entries; the GBA has
  48), named by prgmname.msg:

| Program | Id | Handler | Effect |
|---|---|---|---|
| NavChg+1 | 0x30 | 0x02013F88 | MegaMan's +0x17 += 1, at most 5 |
| NavChg+2 | 0x31 | 0x02013FA4 | +0x17 += 2, at most 5 |
| Spport ("MegaMan supports team") | 0x32 | 0x02013FC0 | MegaMan's +0x59 = 1: MegaMan may support while benched (§4.7) |

These three are NaviCust programs: the GBA's NaviCust compile (0x02013A94, BRBE `sub_813C684`'s counterpart, the
same code) dispatches them by program id, and they are in prgmname.msg with the GBA's programs. The team navis'
own programs are a separate system, PartyCust (§4.10). Both systems set the same flag: a member may support
when its own NaviStats +0x59 is set (0x02004E9C). NaviCust's Spport sets it in MegaMan's block; PartyCust's
P.Spport sets it in a team navi's.

### 4.6 The swap

Both paths start from the same request, AIData +0x44 bit 0x04000000 (`sub_802DCDE`, 0x0200DF2C, or the input
handler above). Which handler sees it decides the path:

- **Paused** (the turn's start, after the custom screen): the paused handler (0x02061B24, `sub_8017BC0`'s
  counterpart) sets AIData +0x48 0x1000 and the idle action 0x1A. Its later ticks run **the GBA's navi switch**
  (0x0202BEA4, `sub_802D714`'s counterpart; on the GBA it is EXE6's action 0x1C) while +0x48 has 0x1000. It runs
  the knockout (0x0202C1A0, action 0x4D) while +0x48 has 0x2000.
  - The request comes from the turn-start sequencer, as on the GBA. `sub_80147E4` (0x0205FFE0, same) copies the
    exchanged transform records to 0x021B83F0 + side·0x10. `sub_801486C` (0x0206005C) calls `sub_802DCDE` when
    the record's +4 isn't 0xFF.
  - The DS resolves the navi object from the side (0x020987A8), where the GBA stored a pointer.
- **Live** (in battle): the live pause handler starts **action 0x59**, the DS's own copy of the same sequence with
  its own dimming. Its steps (table 0x020D9B44) are below, after the paused path's.

The paused path's steps are rewritten from the GBA's. The step tables are 0x020D9B1C (change) and 0x020D9B28
(knockout); BRBE's are 0x0802CAF0 and 0x0802CD10. Action 0x59's table is 0x020D9B44.

**Land** (0x0202BEB8, BRBE 0x0802CAFC): 20 ticks (the GBA's 4).
- It forces the white palette every tick.
- It spawns effect id 3 at the navi (z + 0x10).
- It stops sound channels 1 and 2, plays sound 0x8D and **a voice** (0x02098ED0).
  - The voice is one of two per navi at random, from 0x02132050: a stream on channel 2 and a sound effect
    0x214–0x22D.
  - It plays only on the local side, without battle effect 0x400000, and not for MegaMan, ProtoMan or Colonel
    in slot 0 on turn ≤ 1.
- It sets animation 0 (the GBA's 4) and per-navi music (0x0200102C: DS music, ignored).

**Change** (0x0202BF98, one tick):
1. It ends Soul Unison: the form's overlay goes off and NaviStats +0x2C becomes 0.
2. It keeps the outgoing navi and its emotion in the action's vars.
3. It clears anger and the statuses (0x0206EC40), and sets AIData +0x48 bit 0x4000 ("came in by a switch").
4. It stores the object's HP in +0x40 and copies the whole NaviStats into the outgoing member's block.
5. It finds the incoming member (0x0200DEE8) and copies its block into the side's NaviStats.
6. A team navi gets emotion 0x80 (normal). Then comes the GBA's tail, with these differences:
   - **HP and max HP always come from the member's block** (the GBA's `switch_hp` table is gone);
   - animation 0;
   - `sub_800FC9E` gets NaviStats +0x4C as a third argument (open);
   - 0x02098804 (collision +0x26 = 0, sound 0x93), and the mugshot on the local side.

**Settle** (0x0202C130, 21 ticks, new): it clears AIData +0x48 0x01409800 and +0x44 0x00100600, and ends in the
**Double Attack trigger** (0x02042E74, §5). The trigger sets AIData +0x44 bit 0x02000000. The live handler then
starts action 0x5C, which dims and calls the launcher.

**Action 0x59, the in-battle swap** (read). It is the paused path's sequence with these differences:
- **Land** (0x0202C690), 20 ticks:
  - It **sets battle flag 4** (dimming), so the battle stops around the swap.
  - Then, as the paused land: the white palette, effect 3, sound 0x8D, flags 0x101C41/0x110 cleared,
    animation 0, the aura call, collision region 1.
  - It also stops sounds 0x1B–0x1E (0x02018F08) and channels 1 and 2.
  - It plays the incoming navi's music (0x0200102C: DS music, ignored) and its voice (0x02098ED0) for the member
    in AIData +0x72. There is none of the paused land's turn and slot conditions on the voice; the voice routine's
    own checks remain.
- **Change** (0x0202C760, one tick):
  - Soul Unison ends (+0x2C = 0), and anger and statuses clear (`sub_801A7F4`). AIData +0x48 gets 0x4000.
  - The outgoing navi and its emotion go to +0x30/+0x38.
  - The HP goes to +0x40, and the NaviStats go to the outgoing member's block (its +0x58).
  - The incoming member is **AIData +0x72** (the slot the request named), not found by navi number. Its block
    is copied into NaviStats.
  - The panel's button and pip are updated (0x02036EEC, twice), and **the switch is counted** (0x02036E48).
  - A team navi gets emotion 0x80. Then come the sprite and the form (`sub_800FC9E` with +0x4C), statuses
    cleared, HP and max HP from the block, the HUD (`sub_801E0D0`, `sub_801DBD4`), the GBA's resets, 0x02098804
    and the mugshot.
  - The incoming navi is kept in +0x34.
- **Settle** (0x0202C8F8), 21 ticks:
  - It clears AIData +0x48 0x01409800 (0x1000 included), +0x44 0x00100600, flag2 0x104 and **battle flag 4**.
  - It calls the **Double Attack launcher** directly (0x02042DC0: outgoing, incoming, outgoing emotion), resets
    the mugshot and leaves the attack state.

**New navi actions:** the DS's action table (0x020E5C08) matches the GBA's through 0x58 and adds:
- 0x59: 0x0202C67C, the in-battle swap;
- 0x5A: 0x02004938, the ToadMan rescue;
- 0x5B: 0x02049784;
- 0x5C: 0x02042ED8, a Double Attack after a paused swap.

(Action 0x4C, 0x0202C3C4, is `sub_802DAA8`'s counterpart. The fall-back's request, AIData +0x44 0x08000000, starts
it with +0x48 0x2000.)

### 4.7 What a navi keeps, deletion, supports, the end

**What a navi keeps across a swap:**
- **HP:** its member's own (current and max).
- **Soul Unison:** ends on the way out.
- **Emotion:** MegaMan's comes back with him; a team navi enters at normal; the outgoing navi's Full Synchro
  is the Double Attack's trigger.
- **Statuses, anger, invulnerability, the hit in progress and the hand's charge bonuses:** cleared.
- **The hand and the custom gauge:** the action leaves them; the custom screen cleared the picks (§4.4).
- **The buster charge:** not read (the attack state is reset).

**At 0 HP** (0x02062C5A, in `applyDamageToPlayer`'s counterpart), in order:
1. The GBA's survive-at-1 check.
2. **The ToadMan rescue** (0x020048E8), when all of these hold:
   - ToadMan is a benched member with HP;
   - it is mode 6, or ToadMan's own +0x59 is set (PartyCust's P.Spport, §4.10; the test is 0x02004E9C's);
   - the party block's +0xB is 0;
   - MegaMan's light test passes (0x02004DD8).

   Action 0x5A then:
   - dims and shows telop 0x187 (sound 0x173);
   - spawns object 0x6C three times and object 6;
   - heals a quarter of max HP (sound 0x8A, `heal`), once a battle.
3. **The fall-back** (`sub_802DD2A`'s counterpart 0x0200DFB0): a team navi that came in by a switch requests
   the knockout instead of being deleted (unless AIData bit 0x200000; with 0x2000 or no rescue pending).
4. Otherwise the navi is deleted.

**The knockout's take-back** (0x0202C24C):
- It saves the deleted member's block at HP 0 and **always brings back MegaMan** (member 0) with his stored stats.
- It tells the bottom screen and drops the link navis' chips, as the GBA did.

**Supports:** seven handlers run every frame from 0x02003764: 0x020037A8, 0x02003984, 0x02003A14, 0x02003C6C,
0x02003DB4, 0x02004130 and 0x02004438.
- In a link battle they run for both sides; otherwise for side 0 only.
- Each tests the party block, MegaMan's light state, a benched member's availability (0x02004E9C: HP, and mode 6
  or the member's own +0x59: Spport for MegaMan, P.Spport for a team navi) and that the member isn't the active
  navi.
- Each then targets an enemy (0x02003864) and spawns an ov423 object (0x021FF280) drawn with the member's sprite
  (0x02036450).

What each support does, which sprite and sound it uses and when it fires is **open**: it is not read yet, and it
is the next reading after this step (§8, step 2).

**The end:** `battle_isBattleOver` (0x020023DC) is the GBA's.
- A side loses when MegaMan is deleted. A team navi's deletion never ends it.
- At the battle's end (0x02004BB4, unless effect 8) each member's HP goes back to the save, and a team navi left
  at 0 HP leaves the team (event flag 0x254 + navi cleared).
- A link battle's effects word likely has 8. The DS's netbattle effects list has 0x122E8C at 0x02110378, but
  which record a party netbattle uses is open.

### 4.8 The team navis' restrictions

- **The light and dark chips** (0x020070FC, BRBE's 0x08010118) dropped the GBA's "any navi but MegaMan passes":
  - light-only chips (record `megaman` field 1) work for every navi of a light side (meter ≥ 470);
  - dark-only chips (field 2: the DS navi chips, Static, Muramasa…) also need MegaMan.
  - On the GBA a team navi could use both.
- **Dark chips** (flag 0x20) keep the GBA's rule: MegaMan (or navi 0x17) only.

### 4.9 Against the engine's navi switch

`crates/nettai-battle/src/kinds/player/actions/navi_switch.rs` ports the GBA's switch, which only the gate drove
(exe5-map.md §5, history: the GBA's Team Battle mode, which nettai doesn't implement). The DS keeps its entry and
its sequencer, and changes the rest:

| | The engine (the GBA's) | The DS |
|---|---|---|
| Stats | one reserve (`Battle::reserves`) | three member blocks a side |
| HP | the `switch_hp` table, or fresh stats | always the member's block |
| Land | 4 ticks, animation 4 | 20 ticks, white palette, effect 3, sound 0x8D, a voice, animation 0 |
| Change | animation 3 | animation 0; Soul Unison ends; team navis enter at normal |
| Settle | — | 21 ticks, then the Double Attack trigger |
| Knockout | brings back the kept reserve | brings back MegaMan (member 0); a new fall-back condition |
| Also | — | the ToadMan rescue; actions 0x59–0x5C |

### 4.10 PartyCust (the Party Customizer)

All read in code. The layout and the effects agree with Tango's BN5DS save view
(tango-gamesupport-bn5ds-dataview `save.rs`, `rom/mod.rs`), which found them by driving the customizer.

**What it is.** The DS's equivalent of the NaviCust for the team navis: each team navi has a gauge of blocks, and
programs bought as items fill it. It is a gauge, not a grid: there are no shapes, colors or compression. It runs from
overlay 413 (the 0x021DDBE0 slot, ARM code). Its program names and descriptions are p_prgmname and p_prgminf
(`\rom_usa\a\p_prgm*_LZ.bin`). The programs are items 0xB0–0xBC (item counts at save +0x144C).

**The programs** (ov413's tables at 0x021EC97C, four 16-byte rows: item id, cost, kind, art; the apply jump
table at 0x021E0560 by item − 0xB0, the take-off mirror at 0x021E02D0):

| Item | Program | Cost | Kind | What it adds to the navi's record |
|---|---|---|---|---|
| 0xB0 | P.HP+50 | 1 | 3 (HP) | +0x5A += 50 |
| 0xB1 | P.HP+100 | 2 | 3 | +0x5A += 100 |
| 0xB2 | P.HP+200 | 3 | 3 | +0x5A += 200 |
| 0xB3 | P.HP+300 | 5 | 3 | +0x5A += 300 |
| 0xB4 | P.Atk+1 | 1 | 4 (attack) | +0x01 += 1 |
| 0xB5 | P.Atk+2 | 2 | 4 | +0x01 += 2 |
| 0xB6 | P.Atk+3 | 3 | 4 | +0x01 += 3 |
| 0xB7 | P.Chp+30 | 3 | 2 (chip attack) | +0x56 += 30 |
| 0xB8 | P.Chp+40 | 4 | 2 | +0x56 += 40 |
| 0xB9 | P.Chp+50 | 5 | 2 | +0x56 += 50 |
| 0xBA | P.Btlpk1 | 4 | 5 (special) | +0x5A += 50, +0x01 += 1, +0x56 += 30 |
| 0xBB | P.Btlpk2 | 7 | 5 | +0x5A += 200, +0x01 += 2, +0x56 += 40 |
| 0xBC | P.Spport | 1 | 5 | +0x59 = 1 |

**The gauges** by navi, ProtoMan to ToadMan (0x021EC958; MegaMan has none): 6, 8, 6, 8, 5, 10, 8, 6, 10, 6, 5,
8 blocks.

**Whose stats.** Each team navi's own record: the 0x60-byte NaviStats block the save keeps per navi
(`sub_801401E`(navi) = toolkit+0x74 + navi·0x60, RAM 0x0216A0EC + navi·0x60, save +0x2E6C + navi·0x60). It is not
MegaMan's and not the party's: a program stays with its navi, whichever slot fields it. The fields:
- **+0x01**: the buster attack level (the GBA's field). The battle's shared buster code reads it when the navi is
  active.
- **+0x56** (u16): the chip attack bonus.
- **+0x59**: support. The member may support and be rescued with (§4.7).
- **+0x5A** (u16): the max HP bonus.

**What the battle reads, and where:**
- **Max HP:** 0x02004C84 and 0x02004CA4 (the second also fills HP) write +0x42 = the base HP + the bonus +0x5A.
  The base HP is 0x0203E21A[navi][rank], ten halfwords a navi. They run for all twelve navis (0x02004C5C and
  0x02004C70), from the battle's setup (0x0202A500, 0x0202A696) and the customizer (0x021E0880).
- **Chip attack:** 0x0203BF2C (BRBE `exe5_800EAF8`, in the chip damage table at 0x020DF0B8) gives the damage of the
  team navis' own attacks, chips 0x190–0x1A7 (two per navi: StepSwrd ProtoMan's, C-Cannon Colonel's, KCrusher
  KnightMan's, S-Melody ToadMan's…).
  - In a link battle, and offline outside mode 6, the damage is the base 0x0203E0D7[navi][rank] (ten bytes a
    navi, through 0x02004E48) plus the member's +0x56, when that navi is in the party.
  - Otherwise it is the chip record's power, the GBA's path (`exe5_800EBC4`).
  - "Special chip atk" in the program text means these attacks. It doesn't mean the Double Attacks: their damage
    is the row of §5 alone (the launcher adds nothing).
- **Support:** 0x02004E9C (the supports, the ToadMan rescue).
- **Attack:** wherever the shared code reads NaviStats +0x01.

`rank` is the story rank (0x0203C10C: how many of event flags 0x300–0x308 are set, 0–9). The engine's max-power
rule means rank 9.

**Where the result lives.**
- In the save: the four fields of each navi's record.
- The customizer's loadouts are at save +0x8424, 2 × 0xE: the navi, ten program item ids, and the attack and chip
  attack totals. The panel redraws its gauge from these, and they are not read in battle.
- In battle: the partners' 0x60-byte member blocks (§4.1), copied whole from those records.

**How it crosses the link.** Inside the partner stats: 0x02097B84 copies each partner's whole record
(`sub_801401E`) into the send block (0x021B8F94), so +0x01, +0x40/+0x42, +0x56, +0x59 and +0x5A arrive as part of
the member blocks. Nothing else is exchanged for it.

**For the engine:**
- PartyCust is no new battle state. It is what a partner's member block holds: the bonus fields and the computed
  max HP.
- A build's partners carry their programs, and the setup applies them as the customizer does.
- The new rules:
  - the chip attack sum in the team navis' attacks' damage;
  - the support flag in §4.7's tests.

## 5. Double Attacks

**The trigger** (read):
- **After a live swap**, action 0x59's settle step (0x0202C8F8) clears the dimming and calls the launcher
  0x02042DC0 (outgoing navi, incoming navi, outgoing emotion).
- **After a paused swap**, the settle step's test (0x02042E74) sets AIData +0x73 to the partner and +0x44 bit
  0x02000000. Action 0x5C (0x02042ED8) then dims, lands with sound 0x8D, waits 2 × 20 ticks and calls the
  launcher.
- **In battle mode 6** (open: what mode 6 is; it is the offline mode with one scripted partner and unlimited
  switches), a switch request calls the partner for a Double Attack without a swap, when the active navi is in
  Full Synchro.

A Double Attack fires when all of these hold:
- the **outgoing** navi's emotion is 0xFF, **Full Synchro**;
- AIData +0x73 isn't 0xFF;
- MegaMan's stored emotion in the party isn't 0, when he is a member;
- the pair has an entry in the table.

Details of the test:
- The launcher writes +0x73 = 0xFF after firing, so there is one Double Attack per navi object. Whether that is
  per round or per battle is **open**.
- The test also means to require a light MegaMan. It loads 470 but compares the meter with MegaMan's slot
  number (0x02042EAC), so as read **the light test does nothing** (a DS bug).
- No code clears the outgoing navi's Full Synchro. The incoming team navi enters at normal.

**The table** (read; `double_da.py`):
- The pair table at 0x020E0A94 is 13 × 13 bytes by navi number, symmetric: one Double Attack per pair.
- Double Attack *n* has:
  - chip 0x160 + *n* − 1 (0x020E0A7C);
  - a spawner (0x020E0A4C);
  - a damage row at 0x0203E4FE (ten halfwords by the side's navi level, 0x021B82B0; the GBA's team-navi rows
    have seven).
- The chip records 0x160–0x16B are placeholders on the GBA. On the DS they have:
  - no element, family 0x0E, MB 5 (Mag Air Raid's 0);
  - their hit flags in +0x0A (§6).
- The names are the chip names, chipname1.msg 0x60–0x6B, shown as "US / JP" below.

**Launch** (read):
1. The launcher calls the spawner with the hit flags and damage (r6 = +0x0A << 16 | damage) and the chip (r7).
2. The spawner starts a director, a new T4 object kind 0x96 + *n*.
3. `sub_800BF16`(side, 1, director) starts a dimming the other side can't counter.
4. The director spawns two actors (roles 0 and 1) of a new T1 kind. On BRBE the T1 kinds 0x30–0x44 point at a
   placeholder; the DS fills them.
5. Each actor:
   - draws its navi's own sprite (a per-attack role table);
   - appears with effects 0x6D and 0x6E and sounds 0x94, 0x71 and 0x72;
   - attacks, then warps out (`sub_80E1332`).
6. The MegaMan role takes his soul, Chaos and light/dark look from his stats (+0x2C, +0x4C, +0x44; 0x020A1124,
   0x020A11D0, 0x020A1218).
7. Damage goes through the GBA's hitbox spawner `sub_80C53A6` (0x022034BC), with the r4 words below.

All this code is new DS Thumb in the ARM9, built on the GBA helpers named.

| # | Pair | Name (US / JP) | Damage by level 0…9 | Objects (new) | What it does (outline, read) | GBA code it uses |
|---|---|---|---|---|---|---|
| 1 | MegaMan + ProtoMan | D-HERO / Double Hero | 7,10,12,15,18,20,30,35,40,40 | T1 0x35 | Seven volleys of MegaMan's buster (sound 0x6A, muzzle flash) and ProtoMan's sword (0xB0); a hitbox at each actor (r4 0x0705FF81/82 by side); sparks 0x3C/0x16 on random enemy panels (RNG2) | `sub_80B8E30`/`sub_80B8E58` (attachments), `sub_80C53A6`, `sub_80E1332` |
| 2 | ProtoMan + MagnetMan | M-ZAN / Magnet Zan | 50,50,70,80,90,100,180,200,240,240 | T1 0x36 | A sword swing (0xB0), the panel ahead (r4 0x0705FF04) | attachment kind 0x3A (a new row, sprite 10-3C) |
| 3 | GyroMan + MagnetMan | M-AIR / Mag Air Raid | 60,60,60,70,80,90,150,180,200,200 | T1 0x37, T3 0xA1, T4 0xA3 (sprite 10-2E) | Sounds 0x14C, 0x104, 0x105, 0xFE; hit path open | `sub_801155A`, collision helpers |
| 4 | MegaMan + NapalmMan | D-VALCAN / Double Vulcan | 2,2,2,2,2,3,4,5,6,6 a hit | T1 0x38, T3 0xA1 | Vulcan's shots (the GBA's T3 0x12 through `sub_80C6ADA`, the Vulcan chips' kind), the vulcan attachment; sounds 0xB9, 0x105 | Vulcan's attack object |
| 5 | MegaMan + SearchMan | C-SCOPE / Cross Scope | 15,15,15,15,15,22,30,35,40,40 | T1 0x3A, T4 0xA4 (sprite 10-23, the search scope) | A hitbox (r4 0x19050001); sounds 0xB9, 0xBD, 0x10E | `sub_80C53A6` |
| 6 | NapalmMan + Meddy | C-BOMB / Chemical Bomb | 75,75,75,75,75,75,110,130,150,150 | T1 0x3B, T3 0xCD | A thrown bomb and an area hit (`sub_801BD3C`, r4 0x0405FF0F); sounds 0xBD, 0x112, 0x70; effects 0x4C, 0x14 | `sub_801BD3C`, `sub_801155A` |
| 7 | MegaMan + Colonel | BR-RUSH / (Brain Rush, by its JP name) | 10,12,15,15,20,25,35,45,50,50 | T1 0x3E | Buster shots (0x6A), a sword (0xB0), the panel ahead twice (r4 0x0705FF01), the GBA's T3 0x02 through `exe5_80C4C98`, a camera shake; sounds 0xCF, 0xFB, **0x2F**, 0xCE | `exe5_80C4C98`, `sub_80E11E0` |
| 8 | MegaMan + KnightMan | B-SHOOT / (…Shoot) | 30,30,35,40,45,50,90,100,120,120 | T1 0x3F | The buster (0x6A), a landing (0xC0), a camera shake; hit path open | |
| 9 | Colonel + ShadowMan | E-SAPPOU / Engetsu Sappou | 45,45,45,50,60,75,110,130,150,150 | T1 0x40, T1 0x44 (sprite 10-02, rock) | Sound 0xC0, effect 0x02; hit path open | |
| 10 | TomahawkMan + KnightMan | K-THROW / Knight Throw | 180,180,180,180,220,270,350,400,450,450 | T1 0x41 | A falling throw, a **Wood** hit (r4 0x15050A01); sound 0x109 | `sub_8001330` (the arc) |
| 11 | MegaMan + NumberMan | K-BOMB / Kokuban-keshi Bomb | 40,40,40,40,40,50,70,90,100,100 | T1 0x42, T3 0xCC (sprite 0C-02, bomb) | Two bomb throws (0xB2), a hit at an offset table (r4 0x0405FF01) | |
| 12 | ShadowMan + ToadMan | G-OTOSHI / Gama Otoshi | 30,30,30,30,30,30,40,50,60,60 | T1 0x43, T3 0xA2 (a repointed placeholder) | Sounds 0xD8, 0x105, effect 0x14, a panel change; hit path open | `sub_801155A` |

The hit patterns are outlined only. **Open:**
- what the r4 words' panel bytes (0x81, 0x82, 0x04, 0x0F) mean;
- the hit paths of 3, 8, 9 and 12;
- whether the chip's name shows on screen. The director holds the chip at +0x30, and no telop call was found.

### 5.1 Art and sound: the GBA counterparts

The DS code names its sprites and sounds by the GBA's ids: the same code with the same constants, and the GBA
`sprite_load` with GBA category and index. Read in code; not compared pixel by pixel.

- **Sprites, all in the GBA ROMs:**
  - the team navis 08-01…08-0C;
  - MegaMan 00-00 and his souls 00-01…00-0C;
  - the attachments 0C-00 (sword), 0C-02 (bomb), 0C-03 (buster arm), 0C-06 (muzzle flash) and 0C-1D (vulcan);
  - 10-02 (rock), 10-23 (search scope), 10-2E and 10-3C (unnamed in assets.toml), 10-44 (toad note);
  - 14-08 (charge glow) animations 0 and 2.
- **New rows, not new art:**
  - the effect table (DS 0x02229C14 against BRBE 0x080DFC94) adds rows 0x6C (10-44), 0x6D (14-08, animation 0)
    and 0x6E (14-08, animation 2);
  - the attachment table (DS 0x022266A4 against BRBE 0x080B9974) adds kind 0x3A (10-3C);
  - the GBA's effect row 0x10 changed (14-04 animation 1 → 14-0C animation 0).

  The port adds these rows in content. **No sprite is missing.**
- **Sounds, all in BRBE's song table:**
  - 0x14, 0x6A buster shot, 0x70 bomb hit, 0x71/0x72 buster charge, 0x8D cross change, 0x94 appear;
  - 0xB0 sword swing, 0xB2 bomb throw, 0xB9, 0xBD mark, 0xC0 land, 0xCE big sword swing, 0xCF, 0xD8;
  - 0xFB, 0xFE, 0x104 gyro takeoff, 0x105 drop, 0x109 tomahawk throw, 0x10E, 0x112 place, 0x14C gyro change.

  Names are from content/exe5/compat/assets.toml; 0x14, 0xCF and 0xFE exist there unnamed.
- **Missing from the GBA:**

| What | Where | Closest GBA stand-in |
|---|---|---|
| sound 0x2F | Double Attack 7 (Brain Rush), with 0xFB | an empty entry of BRBE's table: play 0xFB alone (or nothing) |
| the navis' switch voices | the swap's land step (0x02098ED0): two per navi, a voice stream and a sound effect 0x214–0x22D, past the GBA's last sound 0x173 | none: the switch already plays 0x8D (`cross-change`). Leave the voice out |
| the switch music | per-navi music (MegaMan, ProtoMan, Colonel) | DS music: ignored, as the user said |

**Pixels (open):** the ids above are the ones the code passes, and the GBA has art at each of them. That the
DS draws the same art at the same id is **not checked**. The DS sprite list (0x02104E18) names file-system
files, not GBA addresses, and its indexing isn't the GBA's: its 00-00 entry names bs01.bin, which is
byte-identical to the GBA's 08-01 archive (ProtoMan). A comparison in scratch (`double_sprites.py`, unfinished)
has to settle the mapping before "the same id" can be read as "the same art". The user's rule holds either way:
the port draws the GBA sprites at the GBA ids the GBA's own code for the same object uses.

## 6. The DS's other battle changes (catalogued, not adopted)

The user has not chosen to adopt any of these. The recommendation is GBA EXE5 plus the DS's party rules.

- **Chip records** (0x0203E9E8 against BRBE 0x0801E214; chip names from content/exe5):
  - MrkCan1's codes are listed in another order.
  - **LavaSeed, Sword, ElemRage, WavePit, RedWave and MudWave gain a fourth code, `*`.**
  - The +9 flag 0x40 is cleared on Bass, DeltaRay, BugCurse, HolyDrem, BigHook and DethPhnx, and set on LeadRaid
    and ChaosLrd (what 0x40 is: open).
  - LeadRaid and ChaosLrd have +0x16 = 1 (and other +0x16/+0x17 values elsewhere).
  - Chips 0x160–0x16B (no GBA use) have +0x0A set: the Double Attacks' records (§5).
  - Chip 0x186's +0x0B/+0x0C differ.
  - Every record's +0x1E is 0 (the GBA's 1 or 3).
- **Constants in battle objects** (18 `consts` routines, `dsmap.py diff`). Some are reordered stores; others are
  real changes:
  - 0x080D15FA: a `cmp #7` is `#0xB`;
  - 0x080E2620: `#0x78` is `#0x80`;
  - 0x080EA1E0: `#0x60` is `#0x80`;
  - and field offsets in 0x080C1484, 0x080CA3B0 and the 0x080C9xxx–0x080D9xxx group.
- **Calls the DS adds** in 310 similar battle objects: `why.py` lists them by target.
- **The navis' base stats:** the DS reads them from toolkit+0x74 (`sub_801401E`), where the partners' blocks come
  from. They are not yet compared with the GBA's.

## 7. The oracle's facts

**RAM** (`dsmap.py ram`, read: the same routines' literals and the toolkit's tables; JP = US − 0x72B0 for
0x021Bxxxx, − 0x7260 for 0x0216xxxx):

| oracle-trace's field | GBA EXE5 | DS (US) | DS (JP) |
|---|---|---|---|
| battle_state (toolkit +0x18) | 0x02034A90 | 0x021B8EA4 | 0x021B1BF4 |
| rng1 | 0x02001C94 | 0x0216BB1C | 0x021648BC |
| rng2 | 0x02001D40 | 0x0216F230 | 0x02167FD0 |
| (a third RNG, the DS's) | — | 0x0216BB20 | 0x021648C0 |
| toolkit | 0x0200A440 | 0x021701D4 | 0x02168F74 |
| object list, sentinel | 0x0200A410, 0x0200ABA0 | 0x0216F810, 0x0216F800 | 0x021685B0, 0x021685A0 |
| object pools | fixed (§3) | heap: [0x021B8280], [0x021B828C], [0x021B829C] | −0x72B0 |
| AIData, collision | | heap: [0x021B8288], [0x021B8284] | −0x72B0 |
| panels (0x24 each) | 0x0203A100 | 0x021BAAFC | 0x021B384C |
| chip blocks (hands) | 0x02034E20 | 0x021B8AF8 | 0x021B1848 |
| folder | 0x0203C830 | 0x021B856C | 0x021B12BC |
| NaviStats (0x60 × 2) | 0x0203C880 | 0x021B8C64 | 0x021B19B4 |
| member blocks (0x60 × 3 × 2) | (one reserve, 0x02034EC0) | 0x021B9A5C | 0x021B27AC |
| party block (0x24 × 2) | — | 0x021B85E8 | −0x72B0 |
| joypad (toolkit +4) | 0x0200AF50 | 0x0216F7E4 | 0x02168584 |
| input records (8 / 0xC a side) | 0x02036E90 | 0x021B8368 | 0x021B10B8 |
| banner | 0x02036ED0 | 0x021B83D0 | 0x021B1120 |
| transformation sequencer | 0x02036E60 | 0x021B84BC | 0x021B120C |
| transform records (current) | 0x0203B1D0 | 0x021B83F0 | 0x021B1140 |
| fighting machine | 0x0203C5D0 | 0x021B82B8 | 0x021B1008 |
| HUD block (gauge +0x20, tasks +0x40) | 0x020356E0 | 0x021B8630 | 0x021B1380 |
| link struct | 0x0203F244 | 0x021B8290 | 0x021B0FE0 |
| pause flag | 0x02002949 | [toolkit+0x3C]+9 = 0x02167289 | 0x02160029 |
| custom screen state, slots | 0x02036B10, 0x02036C10 | 0x021B985C, 0x021B995C | 0x021B25AC, 0x021B26AC |
| soul and switch marks | 0x02034E10 | 0x021B8300 | 0x021B1050 |

The gauge's and the HUD tasks' places in the HUD block are the GBA's by the block's move; the pause flag's
fill (the ARM routine) is the live one by its callers. Both are to be confirmed on a console.

**Code:**
- **The battle's frame routine** (`battle_8007800`'s counterpart) is 0x0205F970 (the frame count at BattleState
  +0x60, as on the GBA), past its prologue 0x0205F976.
- Its one caller is 0x0202A8DA, which returns at 0x0202A8DE.
- Sounds are `PlaySoundEffect` 0x02018F4C (ARM, r0 the GBA's id) and the stream calls 0x020193B0 and 0x0201933C
  (channels); there is no GBA queue to read.

**Driving a recording:**
- X and Y are melonDS keys. Tango's `Nds` takes them, and the walk's newly pressed halfword is 0x0215F3D6.
- A touch is the touch screen's position: the panel's buttons are hit codes 0x50 and 0x51.
- The switch then rides the packet's +7, so a recording of the inputs replays it.
- Tango's priming walks into a TeamBattle (the board's code 0x61), which is the party battle's route, and writes
  the team words.

## 8. The plan

The steps, each sized small (S, a day or less), medium (M, a few days) or large (L, a week or more). The engine
steps follow the rules-in-Luau line (rust-luau-split.md): Rust keeps the machine (member blocks, the input slot,
the action steps), and Luau keeps the party rules (who may join, the limit, the programs, the pair table, the
supports' conditions).

What a port needs from the DS is its rules and its new objects. Everything they draw and play is the GBA's (§5.1),
so no extractor work is planned beyond content rows.

| # | Step | Where | Size | Depends on |
|---|---|---|---|---|
| 1 | **Finish the reading:** the supports (the seven handlers' conditions, attacks, sprites and sounds), action 0x59's steps against the paused path's, the ToadMan rescue's objects (ids 3, 6, 0x6C), the Double Attacks' hit patterns, and the open items of §9 that code can settle | verify tools/exe5ds | M | — |
| 2 | **The oracle on melonDS:** a DS `Layout` (§7), the battle frame trap (0x0205F970) on melonds-rs, heap-pool reads, a recorder driven by Tango's TeamBattle priming, X/Y and touch input; US first | verify (oracle-trace or a sibling crate), melonds-rs | L | 1 |
| 3 | **Recordings:** party battles with switches in battle and on the custom screen, each Double Attack, the rescue, the supports, a deleted team navi, the limit; a lab-style scenario set | verify | M | 2 |
| 4 | **The party in setup and state:** three member blocks a side (replacing `reserves` for EXE5), the party block (busy flags, count, rescue used), the switch counters; snapshot and digest | nettai-battle, nettai-match | M | — |
| 5 | **The switch as input:** a switch slot in `PlayerTick` beside the buttons (as the DS's packet +7; not X/Y bits, which the engine's `PRESENT` mask uses); replays and the wire format; a sim minor version bump | nettai-battle, nettai-replay, nettai-netplay | S | 4 |
| 6 | **The in-battle switch:** the input copy into AIData +0x70, the input handler's request, action 0x59's land, change and settle | nettai-battle (machine), content/exe5 rules (conditions) | M | 4, 5 |
| 7 | **The custom screen switch:** state 4's check, state 0x40 (clearing the picks, the HP box, the sound), OK's transform record, and the paused path's rewritten steps (land 20, change, settle 21) | nettai-battle, content/exe5 rules (custom_screen) | M | 4, 5 |
| 8 | **Deletion and the end:** the fall-back to MegaMan, the knockout's take-back, loss on MegaMan's deletion, the ToadMan rescue (action 0x5A) | nettai-battle, content | S–M | 6 |
| 9 | **Double Attacks:** the pair table, the trigger and launcher, actions 0x5C, the twelve directors and their actor kinds, the new effect and attachment rows, chips 0x160–0x16B; one at a time against recordings | content/exe5 (objects in Luau, per the content model), small Rust hooks if any | L | 6, 7, 3 |
| 10 | **Supports** (after step 1's reading) | content/exe5 | M | 1, 4 |
| 11 | **Restrictions:** the DS's light/dark chip rule for team navis (dark-only chips MegaMan's only) | content/exe5 rules (light_dark) | S | 4 |
| 12 | **The DS programs:** NaviCust's NavChg+1/+2 and Spport (three programs in content/exe5's NaviCust); **PartyCust** (§4.10): the thirteen programs and the twelve gauges as content, applying them into a partner's member block at setup, the max HP formula, and the chip attack sum in the team navis' attacks (0x0203BF2C) | content/exe5 navicust, a partycust module, rules | M | 4 |
| 13 | **The app:** a party picker in the build creator (two team navis, under the ruleset's team rule: one team's, or any twelve under `mixed`) with a PartyCust editor per partner (its gauge and the program list), a party HUD (the members' faces and HP, the pips; the DS draws it on its bottom screen, so its layout is ours), X/Y bindings and on-screen buttons for touch | nettai, nettai-ui | M–L | 4, 5, 12 |
| 14 | **Netplay:** the partners' stats in the match setup both peers agree on (the DS sends them in the battle's init blocks), and the version line | nettai-match, nettai-netplay | S | 4, 5 |

Sizes are rough. In all it is about two months. Steps 2–3 (the oracle) and 9 (the Double Attacks) are the long
poles. The engine steps 4–8 and 11 can start before the oracle, on the reading, and be checked once recordings
exist (`tiered-checks`: the quick checks per step, the recordings once).

## 9. Open, and what a console should show

**For the user to decide:**
1. **The party's makeup:** any two of the twelve team navis with MegaMan (the DS's Navi Select, across both
   teams), or each version's own? Proposed: any two, as the DS allows.
2. **Mid-battle switching and the custom screen's:** the DS has both. Proposed: both.
3. **The DS-only programs** NavChg+1, NavChg+2 and Spport: adopt them as EXE5 NaviCust programs? They have no GBA
   art. The program's block shape and color are the DS's data; the NaviCust screen's art is the GBA's.
4. **The supports and the ToadMan rescue** are part of the DS's party battle. Proposed: yes, with the party battle.
5. **The switch voices:** silence (proposed), since the GBA has none and the switch already plays 0x8D.
6. **Sound 0x2F** in Brain Rush: drop it (proposed), or a stand-in.
7. **Text:** the twelve Double Attack names exist only in the DS's text (chipname1.msg), and the ToadMan rescue's
   telop too. Are DS words allowed in the locales? The user's rule names assets; the words are content.
8. **The DS's light test bug** (the Double Attack's light check compares the meter with a slot number, so it does
   nothing): keep the DS's behavior (proposed: as the original runs) or the intent?
9. **The DS's other changes** (§6): not adopted (proposed), as the brief recommends.
10. **The team navi leaving the team** at 0 HP after a battle (a story rule; likely off in a netbattle): leave out
    (proposed).

**For a console (observed, step 3):**
- Whether player 2's panel accepts a switch (its side is initialized 0).
- The DS's netbattle battle-effects word (bits 8, 0x200000, 0x400000).
- The pause flag and the gauge at the addresses of §7.
- The four extra bytes of the object records.
- Whether a Double Attack's name shows.
- Whether one Double Attack is allowed per round or per battle (AIData +0x73).
- Whether the buster charge survives a swap.

**In code, not read yet:**
- what loads overlay 423, and the 0x021DDBE0 overlay the battle's bottom screen runs;
- the switch limit's base 3;
- Navi Select's write;
- the game-state byte 0x30–0x3F that refuses a switch to MegaMan offline;
- the party block's +0x18;
- NaviStats +0x4C (a color variant?);
- what the microphone byte starts;
- battle mode 6;
- the supports (step 1).

## 10. Reproducing

In the verification workspace:

    python3 tools/exe5ds/nds.py ~/Documents/Tango/roms/bn5.nds SCRATCH/us     # optional: the files, for text
    python3 tools/exe5ds/dsmap.py where
    python3 tools/exe5ds/dsmap.py ram          # the map, the DS-only routines, the RAM map
    python3 tools/exe5ds/dsmap.py --jp ram
    python3 tools/exe5ds/dsdis.py t0x0202BEA4  # read a routine
