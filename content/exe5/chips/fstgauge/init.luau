--!strict
-- FstGauge (EXE5 chip 0x079): the user's custom gauge fills fast: EXE6's (@exelib/gauge_speed; EXE5's the same code).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local gauge_speed = require("../lib/gauge_speed")

local chip: Chip = {
    codes = { "H", "Q", "W", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 32,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = gauge_speed.fast,
    auto_battle = { class = "at_once" },
}

return { fstgauge = chip }
