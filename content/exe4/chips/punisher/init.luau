--!strict
-- Punisher (EXE4 chip 0x175, past the library: the Guardian statue's strike back's telop names it): a swing that throws
-- a sonic boom down the row, action 0x37 (../sonicbom/action), hit modifier 0 (the record's first parameter), not going
-- through.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local action = require("./sonicbom/action")

local punisher: Chip = {
    action = action.make("punisher/action", 0, false),
    codes = { "E", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "special",
    rarity = 0,
    mb = 8,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
}

return { punisher = punisher }
