--!strict
-- VDoll (BN5 chip 0x067): a doll (./doll) lobbed three panels ahead, that poisons the panel it lands on and, hurt
-- there, curses one of the other side's panels with the damage it took (./curse): BN5's throw (action 0x12, subtype
-- 8: lib/bombs, holding the bomb, the attachment table's row 4, 0x080EBCA0) of the doll (0x080D5926).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bombs = require("../../lib/bombs/bombs")
local doll = require("./doll")

return define.chip {
    id = "vdoll",
    codes = { "O", "T", "Z" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 3,
    mb = 58,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = bombs.throw { held = bombs.held, thrower = doll.thrower() },
}
