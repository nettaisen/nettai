--!strict
-- SloGauge (EXE5 chip 0x078): the other side's custom gauge fills slowly: EXE6's (@exelib/gauge-speed; EXE5's the
-- same code).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local gauge_speed = require("../lib/gauge_speed")

return define.chip {
    id = "slogauge",
    codes = { "H", "Q", "W", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 69,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = gauge_speed.slow,
}
