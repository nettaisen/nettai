--!strict
-- AirSpin1 (BN5 chip 0x082): action 0x47, BN6's AirSpin (@common/airspin/action): a whirlwind top (chips/airspin/top) set down
-- ahead in palette 0, spinning 2 times.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local action = require("@common/airspin/action")
local top = require("../airspin/top")

return define.chip {
    id = "bn5:airspin1",
    codes = { "C", "E", "O" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 0,
    mb = 32,
    damage = 40,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = action.action { whirlwind = top.sender(0, 2) },
}
