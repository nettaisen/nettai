--!strict
-- Bass (EXE4 chip 0x12D, Red Sun's): a navi chip, the dimming chips' action 0x0C, its variant 0x44: its controller
-- (./navi) brings Bass, who fires 24 volleys of two shots at the other side's area.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")

local chip: Chip = {
    codes = { "X" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 95,
    damage = 60,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "bass/controller" }.hook,
}

return { bass = chip }
