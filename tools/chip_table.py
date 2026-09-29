#!/usr/bin/env python3
"""Generate docs/engine/chip-table.md: every battle chip in US Falzar (BR6E).

Reads the chip data table ChipDataArr_8021DA8 (0x08021DA8, 411 records of
0x2C bytes) straight from the ROM, names from the bn6f disassembly's text
archives (TextScriptChipNames0/1), and resolves the attack handlers through
the ROM's dispatch tables:

  action (+0x0B) >= 0x10 -> JumpTable80EAC60[action - 0x10]      (0x080EAC60)
  action 0x15            -> sub_80EBD9C calls off_802CCB4[+0x0C]  (0x0802CCB4)
  action 0x1B            -> sub_80EC350 -> sub_80E192C spawns T4 object 0x10,
                            whose update calls off_802CD5C[+0x0C] (0x0802CD5C)
  action < 0x10          -> per-navi base table off_80EA4C8[AIIndex][action]

Usage: chip_table.py [ROM] [BN6F_DIR] [OUT]
"""
import bisect
import os
import re
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROM = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/Documents/Tango/roms/exe6f_rom_f_e.srl")
BN6F = sys.argv[2] if len(sys.argv) > 2 else os.path.expanduser("~/Documents/Programming/bn6f")
OUT = sys.argv[3] if len(sys.argv) > 3 else os.path.join(HERE, "..", "docs", "engine", "chip-table.md")

CHIP_TABLE = 0x08021DA8
CHIP_SIZE = 0x2C
NUM_CHIPS = 0x19B  # code checks `cmp r0, #0x19b` (sub_800AFBA, sub_800B022)
ATTACK_JT = 0x080EAC60  # JumpTable80EAC60, indexed by CurAction - 0x10
TFC_JT = 0x0802CCB4  # off_802CCB4, indexed by chip +0x0C for action 0x15
NAVI_JT = 0x0802CD5C  # off_802CD5C, indexed by chip +0x0C for action 0x1B
BASE_TABLES = 0x080EA4C8  # off_80EA4C8[AIIndex] -> per-navi action table (actions < 0x10)

rom = open(ROM, "rb").read()
assert rom[0xAC:0xB0] == b"BR6E", "expects US Falzar"


def u8(a):
    return rom[a - 0x08000000]


def u16(a):
    return struct.unpack_from("<H", rom, a - 0x08000000)[0]


def u32(a):
    return struct.unpack_from("<I", rom, a - 0x08000000)[0]


# --- symbols -----------------------------------------------------------------
syms = []
for line in open(os.path.join(BN6F, "bn6f.sym")):
    p = line.split()
    if len(p) == 4 and not p[3].startswith("."):
        syms.append((int(p[0], 16), p[3]))
syms.sort()
sym_keys = [s[0] for s in syms]


def sym(addr):
    """Name a code address (thumb bit ignored)."""
    a = addr & ~1
    i = bisect.bisect_right(sym_keys, a) - 1
    base, name = syms[i]
    # prefer a non-local name at the same address
    j = i
    while j > 0 and syms[j - 1][0] == base:
        j -= 1
        if not syms[j][1].startswith(("loc_", "locret_")):
            name = syms[j][1]
    return name if base == a else "%s+%#x" % (name, a - base)


def fn(addr):
    if addr == 0:
        return "NULL"
    return "`%s` (%08X)" % (sym(addr), addr & ~1)


# --- names -------------------------------------------------------------------
def parse_archive(path):
    out = []
    for line in open(path):
        if "def_text_script" in line:
            out.append(None)
            continue
        m = re.search(r'\.string "(.*)"', line)
        if m and out:
            out[-1] = (out[-1] or "") + m.group(1)
    return [None if s is None else (s.split("@")[0].strip() or None) for s in out]


names0 = parse_archive(os.path.join(BN6F, "data/textscript/TextScriptChipNames0.s"))
names1 = parse_archive(os.path.join(BN6F, "data/textscript/TextScriptChipNames1.s"))


def chip_name(i):
    # sub_8027D10: id <= 0xFF -> TextScriptChipNames0[id], else TextScriptChipNames1[id & 0xFF]
    arc, k = (names0, i) if i <= 0xFF else (names1, i & 0xFF)
    n = arc[k] if k < len(arc) else None
    return (n or "-").replace("|", "\\|")


# --- dispatch tables ---------------------------------------------------------
def read_table(base, stop=None, allow_null=False):
    out = []
    a = base
    while stop is None or a < stop:
        w = u32(a)
        ok = (0x08000000 <= w < 0x08800000 and w & 1) or (allow_null and w == 0)
        if not ok:
            break
        out.append(w)
        a += 4
    return out


attack_jt = read_table(ATTACK_JT)  # 79 entries: actions 0x10..0x5E
tfc_jt = read_table(TFC_JT, stop=NAVI_JT, allow_null=True)  # 42 entries
navi_jt = read_table(NAVI_JT, allow_null=True)  # 29 entries

CODES = {i: chr(ord("A") + i) for i in range(26)}
CODES[0x1A] = "*"
ELEM = {0: "Null", 1: "Fire", 2: "Aqua", 3: "Elec", 4: "Wood"}
FAMILY = {0: "Fire", 1: "Aqua", 2: "Elec", 3: "Wood", 4: "Plus", 5: "Sword", 6: "Cursor", 7: "Obj",
          8: "Wind", 9: "Break", 0xA: "Null", 0xB: "0xB", 0xC: "0xC"}
CLASS = {0: "Std", 1: "Mega", 2: "Giga", 3: "Spec", 4: "PA"}


class Chip:
    def __init__(self, i):
        self.id = i
        a = CHIP_TABLE + CHIP_SIZE * i
        self.raw = rom[a - 0x08000000: a - 0x08000000 + CHIP_SIZE]
        r = self.raw
        self.codes = r[0:4]
        self.elem, self.rarity, self.family, self.cls, self.mb, self.flags = r[4], r[5], r[6], r[7], r[8], r[9]
        self.p0a, self.action, self.sub, self.p0d, self.p0e, self.p0f = r[0xA], r[0xB], r[0xC], r[0xD], r[0xE], r[0xF]
        self.p10 = struct.unpack_from("<I", r, 0x10)[0]
        self.lockout, self.libidx, self.flags2, self.lockon = r[0x14], r[0x15], r[0x16], r[0x17]
        self.sortkey, self.damage, self.libno = struct.unpack_from("<3H", r, 0x18)
        self.p1e, self.p1f = r[0x1E], r[0x1F]
        self.icon, self.image, self.pal = struct.unpack_from("<3I", r, 0x20)

    def code_str(self):
        return "".join(CODES.get(c, "") if c != 0xFF else "" for c in self.codes) or "-"

    def handler(self):
        a = self.action
        if a >= 0x10:
            k = a - 0x10
            if k >= len(attack_jt):
                return "**out of table**"
            s = fn(attack_jt[k])
            if a == 0x15:
                t = tfc_jt[self.sub] if self.sub < len(tfc_jt) else None
                s += " → off_802CCB4[%d] = %s" % (self.sub, fn(t) if t is not None else "**out of table**")
            elif a == 0x1B:
                t = navi_jt[self.sub] if self.sub < len(navi_jt) else None
                s += " → off_802CD5C[%d] = %s" % (self.sub, fn(t) if t is not None else "**out of table**")
            return s
        return "per-navi base table off_80EA4C8[AIIndex][%#x]" % a


chips = [Chip(i) for i in range(NUM_CHIPS)]

# Sanity checks against well-known BN6 facts.
assert chip_name(1) == "Cannon" and chips[1].damage == 40 and chips[1].code_str() == "ABC*"
assert chip_name(3) == "M-Cannon" and chips[3].damage == 180
assert chip_name(4) == "AirShot" and chips[4].code_str() == "*"
assert chip_name(0x11) == "GunDelS3" and chips[0x11].action == 0x37
assert chip_name(0xA7) == "Geddon" and chips[0xA7].action == 0x15
assert chip_name(0xC0) == "Atk+10" and chips[0xC0].damage == 10
assert len(attack_jt) == 79 and len(tfc_jt) == 42 and len(navi_jt) == 29
assert u32(CHIP_TABLE + CHIP_SIZE * NUM_CHIPS) == 0  # table is followed by .word 0

# --- output ------------------------------------------------------------------
L = []
w = L.append
w("# BN6 Falzar (BR6E) chip inventory")
w("")
w("Generated by `tools/chip_table.py` from the ROM (`exe6f_rom_f_e.srl`, sha1 0676ecd4…) and the bn6f")
w("disassembly (names: `TextScriptChipNames0` for ids ≤ 0xFF, `TextScriptChipNames1[id & 0xFF]` otherwise,")
w("per `sub_8027D10`). Do not edit by hand. Field semantics: see `chips.md` §Chip data table.")
w("")
w("Record for chip `id` = `0x08021DA8 + 0x2C*id` (`getChip8021DA8`, 0x08021AA4); ids 0..0x19A (411 records).")
w("")
w("Columns:")
w("")
w("- **codes** (+0x00..+0x03): letters A–Z = 0..25, `*` = 0x1A, 0xFF = empty slot (omitted).")
w("- **elem** (+0x04): attack element, low nibble of the attack's element byte: 0 Null, 1 Fire, 2 Aqua, 3 Elec, 4 Wood.")
w("- **fam** (+0x06): chip icon family: 0 Fire, 1 Aqua, 2 Elec, 3 Wood, 4 Plus, 5 Sword, 6 Cursor, 7 Obj(summon),")
w("  8 Wind, 9 Break, 0xA Null, 0xB/0xC (special). Sword/Cursor/Wind/Break map to secondary-element bits 0x80/0x40/0x20/0x10")
w("  via `byte_80129E4`.")
w("- **cls** (+0x07): 0 Std, 1 Mega, 2 Giga, 3 Spec (not a folder chip), 4 PA (program advance).")
w("- **★** (+0x05): rarity 0..4 (stars − 1). **MB** (+0x08).")
w("- **flags** (+0x09): 0x01 time-freeze chip, 0x02 has damage (shown, boostable), 0x04 Navi chip (Navi+ applies),")
w("  0x08 standard library, 0x10 variable-damage display, 0x40 library (std/mega), 0x80 damage recomputed each frame.")
w("- **p0A** (+0x0A): hi-half of the attack damage word → attack object +0x2E → CollisionData+0x07.")
w("- **act** (+0x0B): CurAction set by `object_setAttack2`. **handler**: see header of this file.")
w("- **sub** (+0x0C): variant/sub-type → AIAttackVars+0x03. **BO** (+0x0F): Beast-Out auto-lock flag → AIAttackVars+0x1D.")
w("- **p10** (+0x10..+0x13, u32): per-action parameters → AIAttackVars+0x0C (passed as r4 to spawners).")
w("- **lock** (+0x14): post-chip input lockout frames → AIData+0x19 at `object_exitAttackState`.")
w("- **f16** (+0x16): 0x80 no slot-in gauge cost, 0x02 cancelled by Rush support (sub_8010740), 0x01/0x10/0x20/0x40 menu-only.")
w("- **LO** (+0x17): Beast-Out lock-on panel selector (index into `jt_8026584`).")
w("- **dmg** (+0x1A, u16). **lib#** (+0x1C, u16) library number. **max** (+0x1E): per-battle slot-in use limit (`sub_802E830`).")
w("- **sub1F** (+0x1F): dark-chip substitution index into `off_8010D84` (0xFF = none, omitted).")
w("")
w("| id | dec | name | codes | elem | fam | cls | ★ | MB | flags | p0A | act | handler | sub | BO | p10 | lock | f16 | LO | dmg | lib# | max | sub1F |")
w("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for c in chips:
    w("| %03X | %d | %s | %s | %s | %s | %s | %d | %d | %02X | %d | %02X | %s | %d | %d | %08X | %d | %02X | %d | %d | %d | %d | %s |" % (
        c.id, c.id, chip_name(c.id), c.code_str(), ELEM.get(c.elem, "%#x" % c.elem), FAMILY.get(c.family, "%#x" % c.family),
        CLASS.get(c.cls, str(c.cls)), c.rarity, c.mb, c.flags, c.p0a, c.action, c.handler(), c.sub, c.p0f, c.p10,
        c.lockout, c.flags2, c.lockon, c.damage, c.libno, c.p1e, "" if c.p1f == 0xFF else str(c.p1f)))

w("")
w("## JumpTable80EAC60 (0x080EAC60): action handlers")
w("")
w("`sub_801B9E6` dispatches `CurAction >= 0x10` to `JumpTable80EAC60[CurAction - 0x10]` (unless AIAttackVars+0x1D == 1,")
w("which routes through the Beast-Out wrapper `sub_80EAD9C`, which itself calls the same table from `sub_80EAF36`).")
w("79 entries (actions 0x10..0x5E); the word after the table is code (`sub_80EAD9C`).")
w("")
w("| index | action | handler | chip ids using it |")
w("|---|---|---|---|")
for k, h in enumerate(attack_jt):
    users = [c for c in chips if c.action == k + 0x10]
    u = ", ".join("%03X %s" % (c.id, chip_name(c.id)) for c in users)
    w("| %d | %02X | %s | %s |" % (k, k + 0x10, fn(h), u or "(no chip; non-chip action)"))
w("")
low = [c for c in chips if c.action < 0x10]
w("Chips with action < 0x10 (dispatched through the per-navi base table `off_80EA4C8[AIIndex]`, i.e. the")
w("current cross/beast form's own table): " + ", ".join("%03X %s (act %X)" % (c.id, chip_name(c.id), c.action) for c in low) + ".")
w("")
w("## off_802CCB4 (0x0802CCB4): time-freeze (action 0x15) effect spawners, indexed by chip +0x0C")
w("")
w("Called once by `sub_80EBD9C` (action 0x15) and by `sub_8017AB4` (counter-freeze during time stop) with")
w("r0 = PanelX, r1 = PanelY, r2 = AIAttackVars+0x02 (element byte), r4 = AIAttackVars+0x0C (+0x10 params),")
w("r6 = AIAttackVars+0x08 (damage word), r7 = (AIAttackVars+0x06 << 16) | chip id.")
w("")
w("| index | spawner | chip ids |")
w("|---|---|---|")
for k, h in enumerate(tfc_jt):
    users = [c for c in chips if c.action == 0x15 and c.sub == k]
    w("| %d | %s | %s |" % (k, fn(h), ", ".join("%03X %s" % (c.id, chip_name(c.id)) for c in users)))
w("")
w("## off_802CD5C (0x0802CD5C): Navi-chip (action 0x1B) summon handlers, indexed by chip +0x0C")
w("")
w("Action 0x1B (`sub_80EC350`) calls `sub_80E192C`, which spawns T4 object 0x10 with Unk_19 = chip +0x0C;")
w("that object's phase `sub_80E1880` calls `off_802CD5C[Unk_19]`.")
w("")
w("| index | handler | chip ids |")
w("|---|---|---|")
for k, h in enumerate(navi_jt):
    users = [c for c in chips if c.action == 0x1B and c.sub == k]
    w("| %d | %s | %s |" % (k, fn(h), ", ".join("%03X %s" % (c.id, chip_name(c.id)) for c in users)))
w("")

os.makedirs(os.path.dirname(os.path.abspath(OUT)), exist_ok=True)
open(OUT, "w").write("\n".join(L))
print("wrote %s: %d chips, %d actions, %d TFC spawners, %d navi handlers" % (OUT, len(chips), len(attack_jt), len(tfc_jt), len(navi_jt)))
