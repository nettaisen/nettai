--!strict
-- SprSonic (EXE4 chip 0x177, past the library: NeoVari's pick): a swing that throws a sonic boom through the row,
-- action 0x37 (../sonicbom/action), hit modifier 1, going through.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local action = require("./sonicbom/action")

local sprsonic: Chip = {
    action = action.make("sprsonic/action", 1, true),
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

return { sprsonic = sprsonic }
