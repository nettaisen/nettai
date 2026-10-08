--!strict
-- VDoll (EXE4 chip 0x067; its record at 0x080197EC + 0x67 * 0x2C): the throw, action 0x09's variant 8: the navi holds
-- the bomb (attachment row 4, its animation 0: 0x080EB4EC) and lobs a doll (./doll, attack 0x7A, thrown by
-- 0x080DCA7C) three panels ahead, which turns the panel it lands on to its hole and, hurt there, curses one of the
-- other side's panels with the damage it took (./curse).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bombs = require("../lib/bombs")
local doll = require("@self/doll")

local vdoll: Chip = {
    action = bombs.throw { id = "vdoll/action", held = bombs.held, thrower = doll.thrower() },
    codes = { "I", "O", "Y" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 3,
    mb = 58,
    damage = 10,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

return { vdoll = vdoll }
