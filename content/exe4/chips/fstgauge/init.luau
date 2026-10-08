--!strict
-- FstGauge (EXE4 chip 0x079): the dimming chips' variant 9 with its first parameter 1: the custom gauge fills fast
-- (../lib/gauge_speed: EXE6's code).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local gauge_speed = require("../lib/gauge_speed")

local chip: Chip = {
    codes = { "I", "Q", "U", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 32,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = gauge_speed.fast,
}

return { fstgauge = chip }
