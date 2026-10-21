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

The verification workspace's tools/dslab records the party battle on two linked melonDS consoles (§7). What its
recordings showed is marked **observed** where it is said; §9 lists the console's answers.

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

  Each record starts 0x10 into its slot, as on the GBA. The four extra bytes are the record's last (observed): an
  actor's +0xC8 (both navis hold 0x000D0222 throughout a battle) and an effect's +0xB8 (0, or 0x140800C1 in some).
  What they are is open; the shared code's fields are at their GBA offsets, the same code reading them. Tango's "unit" records (0x022D6498,
  stride 0xDC) are the actor pool's first slots, on the heap.
- **The AI records** are allocated the same way: the pointer at 0x021B8288 (AIData; a navi's +0x58 points into
  it, observed).
- **The collision records are not on the heap** (observed; read at 0x0208FC18, which clears them): 32 × 0xA0 from
  the toolkit's +0x30, 0x021BE35C (a navi's +0x54 points into them). 0x021B8284 is their allocation mask, bit 31
  the first record (0xC0000000 with the two navis').
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
  - It also needs +2 to equal the local alliance (BattleState +0xD). **+2 is each console's own side**
    (observed): player 2's panel holds 1, and player 2's switches are taken.
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
  - **+8 the switch slot**, +0xA its newly pressed edge (0x020022E8). The slot is the panel's request, **the
    member + 1** (observed: 2 for slot 1, 3 for slot 2), as AIData +0x70 has it; it reaches both input records
    about 6 frames after the press (the link's delay);
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

**Observed** (tools/dslab): in the fight, action 0x59's land is 20 frames (0x8D and the voice), the change plays
0x93, the settle is 21 frames; on the custom screen a switch takes the screen to its sub-state 0x40 (steps 0, 4, 8;
0x8D), clears the picks and rebuilds the slots for the navi, and the turn's start runs the idle action 0x1A with
AIData +0x48 0x1000, its land and settle about 20 frames each. A deleted team navi's fall-back sets AIData +0x44
0x08000000 and runs action 0x4C with +0x48 0x2000, MegaMan coming back at his own HP; the panel shows its box "NO
CHANGE". The fourth switch over the limit of 3 is refused by the panel (0x69, outside the battle's frame).

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
- **The buster charge:** lost (observed). Held through a swap, the charge starts again (sound 0x71) once the
  settle ends, and a release before it fills is an uncharged shot.
- **A team navi's HP** at its first entry is its max (observed): the save's current HP isn't read. MegaMan's
  battle HP is the save's base and its NaviCust.

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
- Each then targets an enemy (0x02003864) and spawns a support actor (0x021FF280).

**The supports in detail** (read; helper `support_calls.py`). The battle flow calls 0x02003764 every frame (from
`sub_80080D2`'s and `sub_800855E`'s counterparts), side 0 then side 1.

The shared gate, for every support:
- MegaMan's light test 0x02004DD8. With MegaMan active it passes unless his emotion is 5. With a team navi
  active, MegaMan's member block needs emotion ≠ 0 and a meter ≥ 470.
- The member is in the party and available (HP, and mode 6 or its own +0x59), and isn't the active navi.
- Most also need no member busy (party block +0xC…+0xE). A support marks its own member busy while its actor lives.

The rest of the mechanics:
- **Damage** is a ten-entry row by the side's level (0x021B82B0, 0–9). The hit flags are in the damage word's high
  half.
- **The target** (0x02003864) is the enemy nearest the active navi (Manhattan, the first slot on ties). The panel
  is one of an offset list's candidates around it (dx by the enemy's facing), checked with
  `object_checkPanelParameters`, the first group that yields any. One is picked with **RNG2**.
- **The actor** (0x021FF280, T1 kind 0x30, DS-new): its variant table is at 0x022270A0. The variants are
  0 ProtoMan 08-01, 1 NapalmMan 08-04, 2 Colonel 08-07, 3 TomahawkMan 08-0A, 4 ShadowMan 08-08, 5 MegaMan 00-00
  (00-0D when his +0x4C is 3) and 6 KnightMan 08-0B.
  - It runs a lead-in that lights the bottom screen's button (local side, 4 frames; 0x02036450 returns that
    button), appears (animation 3, sound 0x94, 12 frames), attacks, warps out (animation 4, 5 frames), and ends.

| # | Handler | Navi | When | Once / repeat | Target | Attack | Damage by level 0…9 (flags) |
|---|---|---|---|---|---|---|---|
| 1 | 0x020037A8 | ProtoMan | the battle's 2nd unpaused frame (block +1 counts while +0 is 0) | once (+0; spent even if a later check fails) | nearest enemy, the panel in front of it, else above or below (0x020CC1E0) | sword (attachment 5, sound 0xB0); on tick 10 a hitbox on the panel ahead (region 0x0705FF04), effect 0x33; 51 ticks | 20,50,70,70,90,90,110,140,150,150 (0x94) |
| 2 | 0x02003984 | GyroMan | while benched | spawned once (+2); removed (0x02225FA4) when he isn't benched or leaves the party; +2 is never cleared (whether he comes back is open) | — | T4 0x92 (kept at +0x1C): every 400/480/540/660 frames (RNG2 bits 15–18, weights 2/6/6/2) a gust through WindRack's code (`sub_80CD488`, no damage), sound 0xAD | none |
| 3 | 0x02003A14 | NapalmMan | a timer (+4): 130/170/210/250 frames (RNG2 bits 6–9, 2/6/6/2) | repeats | nearest enemy, a panel 5…1 columns in front of it in its row (0x020CC228; per-side filters 0x020CC218) | vulcan (attachment 0xD, sound 0xB9), the GBA's Vulcan shot (`sub_80C6ADA`, r4 0x13, offsets by RNG2) | 4,4,4,4,6,7,10,15,20,20 a shot (0x82) |
| 4 | 0x02003C6C | Colonel | the battle's 2nd unpaused frame (+7 counts, +6 used) | once | the **farthest** enemy (0x02003D28), offsets 0x020CC27C | attachment 0x25 (`sub_80B8E70`), a camera shake (1, 0x14), `sub_80CB8DE` (r3 0x100000, r4 2; what it does is open) | 30,60,80,80,100,100,120,170,180,180 (0x94) |
| 5 | 0x02003DB4 | TomahawkMan | a timer (+8): 180/240/350/400 (RNG2 bits 6–9, 2/6/6/2) | repeats | a panel from 0x02003EB0 / 0x02003F0C (rows by facing and the panel pattern; RNG2) | sound 0x10A, a hitbox at the panel ahead (region 0x0705FF11), effect 0x45 | 40,40,40,40,50,50,70,80,90,90 (0x9E) |
| 6 | 0x02004130 | MegaMan, benched (NaviCust Spport) | a timer (+0x10): 130/170/210/250 | repeats | one of three attacks by RNG2 (weights 6/6/4): the Colonel targeting, 0x02004324, 0x020043B0 | buster (sound 0xAE, `sub_80C4FFE`); or sword (attachment 3, 0xB0, hitbox 0x0705FF02); or a MiniBomb throw (attachment 4, 0xB2, `sub_80C5DBC`) | buster 20,40,60,60,80,80,100,110,120,120; sword 25,50,70,70,90,90,110,140,150,150; bomb 30,60,80,80,100,100,120,170,180,180 (0x9E) |
| 7 | 0x02004438 | SearchMan, offline only (effect 8 off) | the first enemy whose +0x2A ≠ 0xFFFF (0x020045A0; meaning open, perhaps "attacking") | once (+0x14) | that enemy's panel | an uncounterable dimming (`sub_800BF16`), telop chip 0x188 "Serch-S", T4 0x94 → T1 0x34 SearchMan (08-03): effect 0x68 (the scope), sounds 0x94, 0xBD, 0xB9, the shot `sub_80E7486` | 5,5,5,5,5,5,10,10,15,15 (0x8A) |

**Supports outside the seven** (read):
- **SearchMan in a link battle** (0x020044E8) is the same, but fires **on the opponent's chip use**. It is called
  for the other side from the chip-use path (0x020778F4, after the GBA's anti-chip reactions `sub_80106C0`,
  `sub_8010740`), targets the opponent navi, and shares the once flag +0x14.
- **MagnetMan's Mag-Barr** (in `sub_8010660`'s counterpart 0x02098A84, the hit path):
  - when the active navi is just hit to HP ≤ max/4, with collision +6 = 0 and MagnetMan available and benched;
  - once a battle per side (battle flag 0x200 or 0x400);
  - it dims, shows telop chip 0x186, and T4 0x91 → T1 0x32 MagnetMan (08-05) gives the navi a barrier
    (`sub_801A7CC`, type 1 at every level) with the barrier visual (`sub_80E0D98`, attachment 0x3A, 10-3C).
- **ShadowMan** (0x020040C4), in the same path when Mag-Barr doesn't fire:
  - HP ≤ 1/4, just hit, ShadowMan available and benched; once (+0xF);
  - he appears on a random own panel (0x02004D84, RNG2) as variant 4 (animation 0x11, 31 ticks), then spawns
    T4 0x96 nine times, 8 ticks apart, on alternating rows (variants 0–3 from 0x02227184 by RNG2 bits 3–6,
    weights 6/2/2/6);
  - what T4 0x96 does is open (it carries no damage).
- **KnightMan's guard:**
  - when KnightMan is available, benched and above max/4 HP, the navi stands at its area's front edge, and
    damage is pending (0x02004604; the request is AIData +0x44 0x100000 from 0x02063B8C);
  - the hit reaction starts **action 0x5B** (0x02049784): +0x48 0x400000, KnightMan (variant 6) in front of the
    navi for 60 ticks, each blocked hit flashing white with sound 0x6B;
  - it is not once-only;
  - **the navi takes no damage and KnightMan's member block loses it** (observed: two M-Cannons, MegaMan at full
    HP, KnightMan 800 → 680 → 560).
- **NumberMan's DiceBons** (in `sub_801171C`'s counterpart 0x0200634C), when a chip is used:
  - the light test; RNG2 bits 10–13 ≤ 6 (7 in 16);
  - once (+0x13); a link battle or side 0;
  - NumberMan available and benched;
  - the used chip without +9 bit 1, and the next chip with +9 bit 2.

  It dims, shows telop chip 0x189, and spawns T4 0x95 (0x022264DC); its effect is open.
- **ToadMan's rescue** (above) shows telop chip 0x187, "HPRecov".
- **Meddy's HealPulse** (0x02003BD8; the user's name for it), from the DS's own fighting state 0x24 (0x0208E1F8,
  and 0x020A3320 in the other flow): when a custom screen is asked for in the fight (0x0208DEBE: the GBA goes to state
  0x20, the DS to 0x24 first), each side in turn (side 0, then 1):
  - the light test; Meddy (navi 6) a member who may support (0x02004E9C) and benched (0x020364E4); the side's navi
    (0x020987A8) below its maximum HP;
  - it heals max × 5 / 100 (0x02004FF4); worried (emotion 1), the mood goes to 0x80 (0x02007EFC);
  - effect 6 (the heal) at the navi, running while paused (flag 4), and sound 0x8A; it lights her button on the bottom
    screen (0x02036450, 0x020935CC). No dimming, no telop.
  - If either side healed, state 0x24 holds 50 ticks (0x32); else it goes to 0x20 on its first tick (one tick in 0x24
    whatever happens: observed in double/d-hero-custom-screen). Every custom screen asked for in the fight, not the
    round's first.

Seen on the way: the microphone event (0x022261A4) sets event flags 0x2E8/0x2E9 and spawns T4 0x93 (0x0222618C).
0x93 heals: `object_addHP`, effect 0x34, sounds 0x8B and 0x8A. **It is offline only** (observed): in a party
netbattle the microphone's level reaches the input record's +9 every 16 frames (127 at full noise), and nothing
reacts to it in 120 frames of noise. The user (2026-10-20): port it, and leave it unwired in nettai.

**The party block's fields** (0x021B85E8 + side·0x24):

| Field | What it holds |
|---|---|
| +0 / +1 | ProtoMan's used flag and counter |
| +2 | GyroMan spawned |
| +3 / +4 | NapalmMan's init flag and timer |
| +6 / +7 | Colonel's used flag and counter |
| +8 / +0xA | TomahawkMan's timer and init flag |
| +0xB | the rescue used |
| +0xC…+0xE | the busy flags |
| +0xF | ShadowMan used |
| +0x10 / +0x12 | MegaMan's timer and init flag |
| +0x13 | DiceBons used |
| +0x14 | SearchMan used |
| +0x15 | the member count |
| +0x18 | a bitmask (open) |
| +0x1C | GyroMan's object |
| +0x20 | KnightMan's object |

**Assets** (GBA ids as the code passes them; the art itself unchecked):
- sprites 08-01, 08-03, 08-04, 08-05, 08-07, 08-08, 08-0A, 08-0B and 00-00 / 00-0D;
- attachments 0, 3, 4, 5, 0xD, 0x25 and 0x3A;
- sounds 0x6B, 0x94, 0xAD, 0xAE, 0xB0, 0xB2, 0xB9, 0xBD and 0x10A;
- effects 0x33, 0x45 and 0x68.

All are in the GBA's tables. The telops are the DS chip names 0x186–0x189 (chipname1.msg): words, which the user
allowed.

**The end:** `battle_isBattleOver` (0x020023DC) is the GBA's.
- A side loses when MegaMan is deleted. A team navi's deletion never ends it.
- At the battle's end (0x02004BB4, unless effect 8) each member's HP goes back to the save, and a team navi left
  at 0 HP leaves the team (event flag 0x254 + navi cleared).
- **A party netbattle's effects word** (the settings' +8, observed on Tango's Practice route): **0x00200A8C** in a
  Single Battle, **0x00200E8C** in a Triple (the settings at 0x0216F860, BattleState +0x3C; their +3, the battle
  mode, 0). Effect 8 (a link battle) and 0x200000 (Navi Select's partners) are set; 0x400000 (no switch voices)
  and 0x800000 (no partners) aren't, so the switch voices play (sound effects 0x214–0x22D). The GBA's netbattle
  word is 0xE8C. The DS's list at 0x02110378 (0x122E8C) is another record's.
- MegaMan's deletion ends the battle with both team navis alive (observed: BattleState's top state 8).

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
- The launcher writes +0x73 = 0xFF after firing, so there is **one Double Attack a round** (observed): a second
  switch out of Full Synchro to the partner in the same round starts none, and the next round's navi starts at
  +0x73 = 0 and fires again.
- After a paused swap (observed): the settle's test sets +0x73 to the partner's navi number (0 for MegaMan) and
  +0x44 0x02000000; action 0x5C plays 0x8D and launches 40 frames later.
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

The table's outline is the first reading. §5.2 has the hit patterns as read since, which correct #1, #2, #5, #7
and #8. **The chip's name shows** (observed): "D-HERO" where a used chip's name shows, the field dimmed, sound
0x173 as it starts. D-HERO's seven volleys did 40 each at navi level 9, the row's last.

**Observed** (tools/dslab, every one of the twelve): each fires on a switch out of Full Synchro to the pair's
partner, a team navi reaching Full Synchro as MegaMan does (a counter). Side 1's HP lost to each at navi level 9:
D-HERO 280, M-ZAN 240, M-AIR 200, D-VALCAN 360, C-SCOPE 400 (stopped on the enemy; unstopped, the scopes end
their 8 cycles off it and miss), C-BOMB 300, BR-RUSH 400, B-SHOOT 240, E-SAPPOU 306, K-THROW 450, K-BOMB 400,
G-OTOSHI 540.

### 5.2 The hit patterns (read; helper `double_hits.py`)

**The hitbox spawner's arguments.** The spawner is `sub_80C53A6` (0x022034BC, the same code as BRBE's):
`object_spawnCollisionRegion` plus flag 0x10.
- **r4** = region | spark << 8 | target type << 16 | self type << 24.
  - The region indexes `PanelOffsetListsPointerTable`; with bit 0x80 it is a filtered whole-field region.
  - Spark 0xFF is none.
  - The types are rows of the collision table (content/exe5/rules/collision.luau): 4 attack, 5 hits_navis,
    6 breaking, 7 slash, 0x0A thrown, 0x15 thrown_break, 0x19 curse.
- **r7** = hit modifier | status << 8 | the bug bytes << 16. Modifier 1 flinches; 3 flinches and flashes.
- **r6** = damage | the chip's +0x0A << 16. The high half is the hitbox's stamina/counter byte (+0x2E), not a
  multiplier. **Every hit deals the row value at the side's level**: nothing multiplies or adds.

**The shared timing.**
- The director: the user warps out (30 ticks), both actors spawn, the director waits for both to leave, waits
  30, warps the user back, waits 30, and ends.
- Each actor: appears for 20 ticks (sound 0x94), charges 60 ticks (#2: 120), with effect 0x6D and sound 0x71 at
  the start and 0x6E and 0x72 20 ticks before the end. Then it attacks, waits 10–30 ticks, leaves over 4 ticks,
  and ends.

| # | Hits | Element, types | RNG |
|---|---|---|---|
| 1 D-HERO | MegaMan alone: 7 volleys, one every 20 ticks, each one hit over **the whole enemy area** (region 0x82 side 0, 0x81 side 1). ProtoMan's sword and the sparks are visual. | null; 5/7; modifier 3 | RNG2: 3 a volley per actor for the sparks (up to 42) |
| 2 M-ZAN | MagnetMan first spawns the immobilizer (T3 0x3F, EXE6 `sub_80CC044`, MagnetSoul's B+Back) on every enemy body's panel from his column forward: no damage, status 0x42. ProtoMan then finds the first enemy whose panel behind is free, steps behind it (3+3 ticks) and slashes **once** (tick 10 of 20), region 4 (a vertical 3-panel column). | null; 5/7; modifier 1 | none |
| 3 M-AIR | GyroMan rises (40 ticks), then flies along the user's row at 2 px a tick. On each **enemy column** he crosses he drops bombs on all three rows (T3 0xA1); each falls 20 ticks and hits its panel once. 3 a column, 9 on a standard field. MagnetMan's T4 0xA3 is visual. | **Elec**; 5/0x0A (thrown); modifier 0; status 0x11 (paralyze) | none |
| 4 D-VALCAN | Each actor fires 30 Vulcan bullets (T3 0x12, EXE6 `sub_80C6946`, Vulcan1–3's, SuprVulc's, InfVulc's, Spreader's), one every 8–9 ticks; each bursts on the first thing in the row, region 2 (its panel and the one behind). Up to 60 hits. | null (the bullet's); modifier 1 | RNG2: 1 a bullet (its visual height), 60 |
| 5 C-SCOPE | SearchMan's two scopes (T4 0xA4) start at (+2, −1) and (+4, −1) and walk crossing diagonals (offsets 0, 1, 2, 1), a step every 6 ticks (sound 0x10E), up to 8 cycles, **until the player stops them** (an input bit of AIData +0x2C: **A**, observed; B and L don't stop them, and the press arrives about 6 frames later). Then each actor fires 5 hits, one every 6 ticks, on its scope's panel. 10 hits. | null; 5/0x19 (curse: reaches invisible targets); region 1; modifier 3 | none |
| 6 C-BOMB | NapalmMan marks (+4, 0) (effect 0x4C, 30 ticks) and lobs the NapalmMn chip's bomb there (T3 0x3A via `exe5_80CC71A`: fire, spreading to the 3×3 on an empty panel). Meddy puts a capsule (T3 0xCD) on a random empty solid enemy panel; if anything hits it while the bomb lives, it explodes 30 ticks later over the 3×3 (region 0x0F, sound 0x70). Up to 2 hits. | bomb fire; capsule null, 5/4, modifier 0, status 0x11 | RNG2: 1 (the capsule's panel); the napalm fire's RNG1 burn order |
| 7 BR-RUSH | Colonel picks the first enemy in the user's row whose front panel passes a filter (else both abort), immobilizes it (GBA T3 #2, `exe5_80C4C98`: status 0x42, sprite 10-1E), and both drop onto that front panel. Each lands **4 hits** on the enemy's panel (at 35, 30 and 10 ticks left of a 40-tick combo, then a finisher). 8 hits. Sound 0x2F starts the finisher, with 0xFB. | null; 5/7; region 1; modifier 3 | none |
| 8 B-SHOOT | MegaMan's buster is visual. KnightMan lands (0xC0, shake) and drops 6 balls (T3 0xCE, sprite 10-0E), one every 20 ticks. Each is kicked (effect 0x3C, sound 0x85) into row −1, 0 or +1 in turn and flies forward 8 px a tick, hitting the first enemy in its row once. 6 hits. | null; 5/6 (breaking); modifier 3; spark 0x0A | none |
| 9 E-SAPPOU | For each enemy, Colonel drops a rock (T1 0x44, 10-02) on the panel in front of it and the one behind, where enemy-owned, solid and empty. Each rock lands (shake), waits 30 + 16·k ticks, and becomes **a ShadowMan copy** (08-08) whose circular slash (T3 0x69, 0x02210680) hits the 8 panels around it, one a tick, in a fixed order. Up to 2 hits an enemy. | null; 5/7; region 1; modifier 3, bug bytes 0x18, 0x01 (meaning open) | none |
| 10 K-THROW | KnightMan, thrown (0x109), arcs (`sub_8001330`) to (+3, 0) over about 119 ticks: **1 hit** there. On a solid panel: shake, 0xC0, and the 3×3 cracks (`sub_80DB48A`, region 0x0F). On a hole: effect 0x12, gone. | **Wood**; 5/0x15 (thrown_break); modifier 3; spark 0x0A | none |
| 11 K-BOMB | Each throws 2 bombs (T3 0xCC: 0C-02, then 10-25 in the air), 31 ticks apart, lobbed 60 ticks to (+3, 0). On a solid panel each bursts into **4 hits** on a 2×2 quadrant: MegaMan's quadrants 0 and 2, NumberMan's 1 and 3, so together the 3×3 with its center hit 4 times. 16 hitboxes. | the chip's (null); 5/4; modifier 3; status 0x20 (confuse) | none |
| 12 G-OTOSHI | ShadowMan drives 12 frogs (T3 0xA2, 10-46), one every 11 ticks; ToadMan's song (0xD8) is visual. Each frog falls on a random panel: one holding an enemy with 10/16 odds, else another (`object_getPanelsFiltered`; its panel set is open). It hits once below z 20. On a hit, or 20 ticks after landing on a solid panel, **the panel becomes sea** (type 10). On a hole it vanishes. Up to 12 hits. | **Aqua**; 5/0x0A (thrown); modifier 1 | RNG2: 2 a frog, 24 |

**Settled by the port** (read, step 9; the port's tests deal what the recordings lost):
- **#1 D-HERO:** as above; its director alone doesn't link its actors (every other director writes each actor into
  the other's +0x60 as it brings them, and most actors wait on their partner).
- **#2 M-ZAN:** the immobilizer's 0x178 is its Param1 0x78 (the DS's immobilizer is the GBA's code but that it
  lingers Param1 ticks, the GBA's 20) and Param2 1 (it plays 0xFD). ProtoMan's search goes on after each slash: he
  slashes every enemy he can reach, one after another (one against one navi). His charge is 120 ticks, the charged
  glow 80 before its end.
- **#3 M-AIR:** each bomb (T3 0xA1, the GBA's never-spawned attack object 0x62) hits at most once, on its own panel;
  of the nine, the one on the opponent's panel hits. MagnetMan's T4 0xA3 are visual (a bomb sent up to GyroMan, sound
  0xFE).
- **#4 D-VALCAN:** the T3 0xA1 the outline lists is unreachable (a copy of M-AIR's drop and count routines follows
  its spawn stub, and nothing calls it). The DS's bullet burst (0x020A1FAA) sets the collision's self flag 0x10000
  (hit even while dimmed) for its two new bullet rows, 0x14 and 0x15 (the GBA's 0x11 and 0x0C otherwise).
- **#5 C-SCOPE:** the stop key is A as the dimmed input record has it (AIData +0x2C bit 1, the actors sharing the
  user's AIData: EXE6's `sub_80C2EB0` reads the same), observed.
- **#6 C-BOMB:** the capsule itself has no hit; its burst is the hit of the row (modifier 0, status 0x11). Meddy
  doesn't charge; NapalmMan attacks only if the capsule is there as his charge ends.
- **#7 BR-RUSH:** the thrown object is Colonel's cape (the GBA's attack object 2) with Param2 2, which carries no
  immobilizing status: a 0-damage hit that stops it. Its wait-to-slide test compares the wrong register, so it
  always flies.
- **#8 B-SHOOT:** only the balls kicked into the opponent's row hit: two of six.
- **#9 E-SAPPOU:** the bug bytes are the moon blade's: bug 0x18 argument 1, an HP drain of level 1 (the recording's
  306 is two hits of 150 and the drain's 1 a 40 ticks after the dimming). A copy hits a given enemy once. The DS
  changed the blade: its first sweep at once (0x02210694), its hits while dimmed with its third parameter set
  (0x0221070C).
- **#10 K-THROW and #11 K-BOMB:** the arc is the DS's rewrite of `sub_8001330` (0x0208DB5C: the SDK's arctangent
  table, the hardware square root), not the GBA's.
- **#12 G-OTOSHI:** ToadMan drops the frogs; ShadowMan sings until he is done. The panel set: an RNG2 draw's low four
  bits under 10, a panel holding an enemy body; else any panel without one, the user's side's included.

Still open: that EXE5's status durations are EXE6's (0x11, 0x20, 0x42).

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

**Pixels (read, `double_sprites.py`):** the same id is the same art. The DS's `sprite_load` (0x0206CC68) takes
the GBA's arguments and reads a resource number from its list, [[0x02104E18 + category] + 4 · index] (bit 31: none,
the placeholder 0x304); 0x02001F68 loads that resource by its path, the word at 0x021588B0 + 4 · number (ARM
0x02034120). The paths follow the GBA's categories and indices (`\rom\a\pl00.bin` for 00-00, `bs01.bin` for 08-01,
`pshl00.bin` for 0C-00, `shl23.bin` for 10-23, `efc08.bin` for 14-08). (The earlier reading took 00-00's number
0x215 for a file-system id, which named ProtoMan's bs01.bin.) Compared byte for byte with BRBE's archives (decompressed
where they are), every sprite the Double Attacks load is the GBA's at the same id but two, both reached only in a Hub
Style the GBA doesn't have (NaviStats +0x4C 3; the GBA's patch card 111 sets 1 or 2):
- 00-0D, the DS's own MegaMan (pl0d.bin; the GBA's list has another archive there): his base sprite stands in;
- 0C-03, the buster arm: the DS's has a fifteenth animation (0x0E, the arm in that style); the GBA's fourteen. The
  arm's animation 0x0D stands in.

### 5.3 The port (step 9)

**Where it is** (content/exe5):
- `double_attacks/<name>/`: each Double Attack's module: its chip (the DS's record and names, gen_content.py's DS
  chips; its record's use the cannon's, which nothing starts: no folder or hand holds it), and the chip's
  `double_attack` (an EXE5 extension, rules/init.luau's `extends.chip`): its damage row and its two actors, each a
  role (its navi, its place beside the user's spot, its actions), with the objects it alone spawns beside it.
  `double_attacks/init.luau` lists the twelve in the DS's order: the pair table is each Double Attack's own two
  navis, either way round.
- `lib/double_attack.luau`, the shared maker: the director (one effect kind a Double Attack, the DS's 0x96 + n: the
  dimming's begin, dim, the chip's telop, the course, undim, end), the actor kind with its role table and the link
  between the two, and the steps most share (appear, with or without its panel's check and a team navi's parts;
  the charge and its glows; leave; the glow at the hand; the hit while dimmed), and MegaMan's look by his stats (the
  active navi's, else his member block's: his form's sprite and overlay, Hub Style, his light/dark value's palette,
  his buster arm).
- `rules/double_attacks.luau`: the launcher (0x02042DC0) on the rules' `navi_switched` in battle; the trigger
  (0x02042E74) on the paused path, which keeps the partner and raises `follow_up`, whose action (0x5C) is the role
  `follow_up`; once a round (the side's state `double_attack`, AIData +0x73).
- `lib/ds_arc.luau`: the DS's arc (K-THROW, K-BOMB). The GBA's objects a Double Attack reuses (the napalm bomb, the
  bullets, the immobilizer, the moon blade) keep the GBA's arc where they use one.

**Against the recordings** (tools/dslab, `double/`): each deals what the DS's lost at navi level 9 (nettai-match's
tests, on a field of the engine's: there a Double Attack's hit on side 1's navi lands a tick before the DS's, whose
update list runs side 1's navi before side 0's from the battle's start). Replayed through exe5-compat (verify's
exe5_replay on lab-exe5ds/double, the kinds in compat/kinds.toml): M-ZAN and E-SAPPOU match every frame; B-SHOOT,
C-SCOPE (stopped and not), D-VALCAN every object, the recording console's RNG1 differing from before the launch (the
party's own draws); the rest first differ where the side-symmetry rule (docs/engine/side-symmetry.md; the canonical
side 1, the recordings' Double Attacks side 0's) parts from the original: D-HERO's flashes and G-OTOSHI's frogs on a
list taken in the side's order (§1.5: with the original's order D-HERO matches every object, G-OTOSHI until the DS
bubbles a ToadMan whom a frog put on sea, the sea's rule, not the Double Attack's), K-THROW's, K-BOMB's and C-BOMB's
arcs a unit off in the mirror image (§1.6: unmirrored, K-THROW matches every frame and K-BOMB every object), BR-RUSH's
cape and M-AIR's GyroMan on a panel border (§1.1). The paused path (d-hero-custom-screen) stops before the Double
Attack, in the custom screen's switch.
The frames (verify's `double_frames.py`, the engine's example `double_attack_frames`): of what each draws, the
share the DS's capture shows the same at its tick, with the shake's offset allowed: C-SCOPE 99%, K-THROW 99%,
B-SHOOT 99%, E-SAPPOU 98%, M-ZAN 96%, BR-RUSH 96%, C-BOMB 95%, K-BOMB 95%, M-AIR 93%, G-OTOSHI 92%, D-VALCAN 92%, D-HERO
79%. The rest is panels drawn from RNG2 (D-HERO's flashes, D-VALCAN's bullet heights, G-OTOSHI's frogs, C-BOMB's
capsule and its fire's spread), the lab stage's boulders, the DS's camera shake (it moves the picture further than
the engine's) and the one-tick shifts. The actors, their attachments and effects show the same art, places,
animations and palettes (the DS lab's MegaMan has the light/dark value 1000). The share counts only what the engine
draws: a part it leaves out shows in the sheets alone (C-SCOPE's scopes and G-OTOSHI's frogs drew half, their first
part hidden, until each was looked at against the DS's).

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
| AIData; collision | | heap: [0x021B8288]; 32 × 0xA0 from [toolkit+0x30] = 0x021BE35C, mask 0x021B8284 | −0x72B0 |
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

**Observed** (tools/dslab, 2026-10-20): every US address above but the transformation sequencer, the banner and
the JP column, with these corrections and details:
- AIData's pool is [0x021B8288]; **the collision records are 32 × 0xA0 from the toolkit's +0x30 (0x021BE35C), and
  0x021B8284 is their allocation mask** (§3).
- The gauge is the HUD block's +0x20 (0x021B8650; its rate +0x22, 0x20 a frame), the tasks its +0x40 (0x021B8670).
- The pause flag 0x02167289 is 1 from a round's start through the custom screen and the turn's start, 0 while the
  fight runs.
- The link struct: the frame routine tests its +0 (0x0205F994, DS-only code); its +1, the GBA's status byte by the
  move, reads 0 at the routine's end.
- BattleState +0x60 counts the battle's frames from its first; the fighting machine's first byte is 8 with the
  fight live; the custom screen's blocks have the GBA's shapes (+7 the cursor, location 10 OK).
- The party block's +1 and +7 (ProtoMan's and Colonel's counters) count every frame from the fight's start.

**Code:**
- **The battle's frame routine** (`battle_8007800`'s counterpart) is 0x0205F970 (the frame count at BattleState
  +0x60, as on the GBA), past its prologue 0x0205F976.
- Its one caller is 0x0202A8DA, which returns at 0x0202A8DE (observed: one call a frame).
- **The oracle watches rather than traps** (observed): Tango's priming walk owns both consoles' trap tables until
  the battle's 64th frame, and melonDS's data watches are a table of their own. The watched loads are the
  routine's read of the toolkit's +0x18 (at 0x0205F97E: its entry), its last load, BattleState +0xA (at 0x0205F9B0:
  its exit), and `PlaySoundEffect`'s literal (at 0x02018F54). A watch reports the next instruction (Thumb +2,
  ARM +4). The two consoles run on threads of their own, so the recorder pairs the other console's captures with
  the traced one's lines by battle frame.
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
| 1 | **Finish the reading** (mostly done 2026-10-20: the supports, action 0x59, the Double Attacks' hit patterns): the open items left (ShadowMan's T4 0x96, DiceBons' T4 0x95, KnightMan's guard's damage, Colonel's `sub_80CB8DE`, the rescue's objects, the microphone event) | verify tools/exe5ds | S | — |
| 2 | **The oracle on melonDS** (done 2026-10-20, tools/dslab): a DS `Layout` (§7), the battle frame trap (0x0205F970) on melonds-rs, heap-pool reads, a recorder driven by Tango's TeamBattle priming, X/Y and touch input; US first | verify (oracle-trace or a sibling crate), melonds-rs | L | 1 |
| 3 | **Recordings** (done 2026-10-20: switches, the limit and NavChg+1/+2, deletions, the twelve Double Attacks, the supports, the rescue; data/traces/lab-exe5ds): party battles with switches in battle and on the custom screen, each Double Attack, the rescue, the supports, a deleted team navi, the limit; a lab-style scenario set | verify | M | 2 |
| 4 | **The party in setup and state:** three member blocks a side (replacing `reserves` for EXE5), the party block (busy flags, count, rescue used), the switch counters; snapshot and digest | nettai-battle, nettai-match | M | — |
| 5 | **The switch as input:** a switch slot in `PlayerTick` beside the buttons (as the DS's packet +7; not X/Y bits, which the engine's `PRESENT` mask uses); replays and the wire format; a sim minor version bump | nettai-battle, nettai-replay, nettai-netplay | S | 4 |
| 6 | **The in-battle switch:** the input copy into AIData +0x70, the input handler's request, action 0x59's land, change and settle | nettai-battle (machine), content/exe5 rules (conditions) | M | 4, 5 |
| 7 | **The custom screen switch:** state 4's check, state 0x40 (clearing the picks, the HP box, the sound), OK's transform record, and the paused path's rewritten steps (land 20, change, settle 21) | nettai-battle, content/exe5 rules (custom_screen) | M | 4, 5 |
| 8 | **Deletion and the end:** the fall-back to MegaMan, the knockout's take-back, loss on MegaMan's deletion, the ToadMan rescue (action 0x5A) | nettai-battle, content | S–M | 6 |
| 9 | **Double Attacks:** the pair table, the trigger and launcher, actions 0x5C, the twelve directors and their actor kinds, the new effect and attachment rows, chips 0x160–0x16B; one at a time against recordings | content/exe5 (objects in Luau, per the content model), small Rust hooks if any | L | 6, 7, 3 |
| 10 | **Supports:** seven per-frame supports, SearchMan's link variant, Mag-Barr, ShadowMan's, KnightMan's guard (action 0x5B), DiceBons; the support actor (T1 0x30, seven variants) | content/exe5 | M–L | 1, 4 |
| 11 | **Restrictions:** the DS's light/dark chip rule for team navis (dark-only chips MegaMan's only) | content/exe5 rules (light_dark) | S | 4 |
| 12 | **The DS programs:** NaviCust's NavChg+1/+2 and Spport (three programs in content/exe5's NaviCust); **PartyCust** (§4.10): the thirteen programs and the twelve gauges as content, applying them into a partner's member block at setup, the max HP formula, and the chip attack sum in the team navis' attacks (0x0203BF2C) | content/exe5 navicust, a partycust module, rules | M | 4 |
| 13 | **The app:** a party picker in the build creator (two team navis, under the ruleset's team rule: one team's, or any twelve under `mixed`) with a PartyCust editor per partner (its gauge and the program list), a party HUD (the members' faces and HP, the pips; the DS draws it on its bottom screen, so its layout is ours), X/Y bindings and on-screen buttons for touch | nettai, nettai-ui | M–L | 4, 5, 12 |
| 14 | **Netplay:** the partners' stats in the match setup both peers agree on (the DS sends them in the battle's init blocks), and the version line | nettai-match, nettai-netplay | S | 4, 5 |

Sizes are rough. In all it is about two months. Steps 2–3 (the oracle) and 9 (the Double Attacks) are the long
poles. The engine steps 4–8 and 11 can start before the oracle, on the reading, and be checked once recordings
exist (`tiered-checks`: the quick checks per step, the recordings once).

## 9. Open, and what a console should show

**Decided** (the user, 2026-10-20: "yes to all of this"):
1. **The party's makeup** follows the EXE5 ruleset's team rule: one team's navis normally, any of the twelve under
   its `mixed` setting (content/exe5/rules/rulesets.luau, builds.luau).
2. **Both switch paths**, in battle and on the custom screen.
3. **The DS programs** are adopted: NaviCust's NavChg+1, NavChg+2 and Spport, and PartyCust (§4.10).
4. **The supports and the ToadMan rescue** are in.
5. **The switch voices** are left out; the switch plays 0x8D.
6. **Sound 0x2F** in Brain Rush is dropped (0xFB plays with it).
7. **DS words** may go in the locales: the Double Attack names, the telops 0x186–0x189, the PartyCust programs'
   names and descriptions.
8. **The light-check bug** is kept as the DS runs it.
9. **The DS's other changes** (§6) are not adopted.
10. **The story rule** (a team navi at 0 HP leaving the team) is left out.

**Answered on a console** (tools/dslab's recordings, data/traces/lab-exe5ds in the verification workspace):
- Player 2's panel accepts a switch: its +2 is its own side, 1 (§4.2).
- The party netbattle's effects word is 0x00200A8C (Single) or 0x00200E8C (Triple): 8 and 0x200000 set, 0x400000
  not (§4.7).
- The pause flag and the gauge are at §7's addresses.
- The four extra bytes are each record's last; what they hold is open (§3).
- A Double Attack's name shows (§5).
- One Double Attack a round (§5).
- The buster charge doesn't survive a swap (§4.7).
- The microphone heal is offline only (§4.7).

**In code, not read yet:**
- what loads overlay 423, and the 0x021DDBE0 overlay the battle's bottom screen runs;
- the switch limit's base 3;
- Navi Select's write;
- the game-state byte 0x30–0x3F that refuses a switch to MegaMan offline;
- the party block's +0x18;
- NaviStats +0x4C (a color variant?);
- battle mode 6;
- what ShadowMan's T4 0x96 (observed: no damage) and DiceBons' T4 0x95 do;
- the microphone event's trigger offline (a heal through T4 0x93; nothing in a netbattle, §4.7).

## 10. Reproducing

In the verification workspace:

    python3 tools/exe5ds/nds.py ~/Documents/Tango/roms/bn5.nds SCRATCH/us     # optional: the files, for text
    python3 tools/exe5ds/dsmap.py where
    python3 tools/exe5ds/dsmap.py ram          # the map, the DS-only routines, the RAM map
    python3 tools/exe5ds/dsmap.py --jp ram
    python3 tools/exe5ds/dsdis.py t0x0202BEA4  # read a routine

## 11. As built: the engine's party battle (branch party-engine, 2026-10-20)

Plan steps 4–8, 11 and PartyCust's and NavChg's part of 12, and the shout (§4.2's microphone), by the readings of
§4 and the engine port's own (each routine is cited where it is ported). Not built here: the Double Attacks (step 9,
EXE5 content on its hooks below) and the supports (step 10).

**The machine (Rust, neutral names).**
- **The input.** `PlayerTick` gains `switch_to` (the member asked for: the DS's packet +7, the member + 1) and
  `mic_level` (packet +5). A host carries a tick as a 20-bit word (`PlayerTick::word`: buttons 0–9, switch 10–11,
  level 12–19): the wire's tick element, replays (a u32 a side where it changes) and the frontend's netplay. Nothing in
  nettai sets either yet: the app's X/Y bindings are step 13, and the microphone is wired to nothing (the user).
- **The state.** `Battle::party: [Party; 2]` in place of `reserves`: the members' stats blocks (member 0 the side's
  own navi, a copy of its stats as the battle starts: the GBA's one reserve, which a side without partners still is),
  the count, the member on the field and the switches made. `Battle::switch_inputs` (the input record's +8, after the
  rules' refusals) and `mic_levels` (+9); `ActorData::switch_input` (AIData +0x70) and `switch_member` (+0x72). All
  in the digest.
- **The switch.** The navi's input asks it on a tick with no chip request (0x0203C5F6); the live request handler
  starts `EngineAction::NaviSwitch` (action 0x59: dimmed land of 20 ticks, the change from the member blocks, the
  count, the settle of 21 that undims); the paused path and the knockout take the party's steps where the side has
  partners (land 20, change, settle; the take-back to member 0), the GBA's otherwise. The land draws the side's
  console RNG once where the DS picks its switch voice (the voice itself is left out). The fall-back is the DS's for a
  side with partners (the active member, not the navi).
- **The custom screen.** `Phase::Switching` is state 0x40: the request in the choosing (0x0200C82C), the fade to white,
  the screen's navi the member's (`Screen::member`), the picks dropped and the slots laid out for it, the battle's
  count; OK writes the transform record's navi, which the turn's start switches to while paused.
- **Request routes** (each a request bit, a state bit, a role and its place in the live handler's order, the DS's):
  `revival` / `reviving` / role `revival` first; then the switch knockout, the in-battle switch, `follow_up` /
  `following_up` / role `follow_up`, the volley. A navi whose revival is asked for or runs isn't deleted at 0 HP.
- **Rules hooks:** `switch_requested(side, member)` (the panel's refusals, as a request enters the input record),
  `switch_asked(side, navi, member)` (whether the navi's request is a switch), `member_busy(side, member)` (asked by the
  in-battle and the custom screen's switch), `navi_switched(side, navi, member, mood)` (the end of a switch's settle),
  `navi_falling(side, navi)` (at 0 HP, before the fall-back), `mic_heard(side)` (a fight tick with a level).
- **Luau API:** `battle.party(side)`, `battle.member(side, m)` (a member's block, by the same proxy as
  `battle.navi`), `battle.join_party(side, navi)` (round_setup only), `battle.mic_level(side)`, `navi:take_back()`;
  the `flinch` request; the `dim` and `undim` screen fades; `overlay_stepping("told")` (the DS's 0x02202064).

**The game (EXE5's Luau).**
- `rules/party`: who joins (the setup's `party`: up to two team navis with their PartyCust programs; MegaMan's alone),
  the panel's refusals (the limit, 3 and the NaviCust's `switch_bonus`; a member at 0 HP), battle mode 6, the light
  test, the support test, the rescue's test, validation; `rules/party/partycust` (the makers and their application),
  `rules/party/knockout` (EXE5's action 0x4C, the role `switch_knockout`), `navis/toadman/rescue` (action 0x5A, the
  role `revival`), `rules/shout` (the shout and its T4 0x93).
- PartyCust's 13 programs are the collection `partycust_programs`; each team navi states its gauge, its base HP and
  its own chip's damage by the side's level (its `party`) and its team (`version`, which the ruleset's team rule reads
  through a `party` category). NaviCust's NavChg+1/+2 and Spport are programs 48–50.
- Stats of EXE5's own: `switch_bonus` (+0x17 less the DS's 3, so the GBA's blocks read alike), `chip_attack_bonus`
  (+0x56), `supports_party` (+0x59).
- The team navis' damage in a party (lib/navi_level): the DS's link rows by the attack level (0x0203C044) and the own
  chip's base and P.Chp (0x0203BF2C), 999 without that navi. The light/dark rule of a side with partners is the DS's
  (0x020070FC), on the value of the navi on the field.

**Decisions** (for review):
1. **The party is inert without partners.** Every DS rule here applies only to a side with partners: the switch
   steps, the fall-back, the light/dark rule, the damage rows. Existing EXE5 and EXE6 battles are unchanged but for the
   state's layout (the digest).
2. **A party is MegaMan's** (member 0, as on the DS); a team navi played as a side's own brings none.
3. **The DS's base HP and damage tables apply in a party** (the partners' HP, their attacks): they are what PartyCust's
   programs add to. A side's level in a party battle is the setup's `level`, else 9 (the DS's highest, max power).
4. **A partner enters at its maximum HP** (observed), its light/dark value is 500 (a fresh record's: every value Tango's
   saves hold reads alike).
5. **The switch's panel is the sim's refusal, its buttons the frontend's.** The input names the member, as the DS's
   packet does; which member X and Y name is the app's (step 13).
6. **The shout** is ported as read and wired to nothing. A netbattle recording shows its level arriving and no heal;
   which of its conditions held it back is open.
7. **The DS words** (the three NaviCust programs, PartyCust's names and descriptions, HPRecov) and the three programs'
   parts are generated from the DS ROMs by the verification workspace's gen_content (gen_ds.py): data and text only.
   HPRecov is chip 0x187, the GBA's placeholder record with the DS's name, which the rescue's telop names.

**The version.** The tick's input, the battle's state and the digest changed: the next release must be a minor one
(docs/design/rollback.md's version lines), which the user makes.

**Unverified:** everything above against a console. The DS recordings (verify's data/traces/lab-exe5ds) are the next
step, once the trace reader learns the `exe5ds` lines. Open as read: the overlay's third parameter on the switch's
land (what it tells it), the busy flags (the supports'), the custom screen's refusal of MegaMan offline in some story
states, battle mode 6.
