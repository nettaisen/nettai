--!strict
-- BassAnly (EXE4 chip 0x132, Blue Moon's): a navi chip, the dimming chips' action 0x0C, its variant 0x45: its
-- controller (./navi) brings BassAnly, who fires four dark balls up and down the slant, seeking enemy navis.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi = require("@self/navi")

local chip: Chip = {
    codes = { "X" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 95,
    damage = 160,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = navi.controller { id = "bassanly/controller" }.hook,
}

return { bassanly = chip }
