--!strict
-- DeltaRay (EXE4 chip 0x12E): a navi chip in all but its navi flag (AntiNavi lets it through), the dimming chips'
-- action 0x0C, its variant 0x32: its controller (./navi) brings DeltaRay, who strikes an enemy navi at the delta's
-- corners, once more each time his user presses A during a strike.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")

local chip: Chip = {
    codes = { "Z" },
    element = "null",
    family = "sword",
    class = "giga",
    rarity = 4,
    mb = 82,
    damage = 260,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = navi.hook_for(0),
}

return { deltaray = chip }
