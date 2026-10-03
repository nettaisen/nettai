--!strict
-- AirSpin3 (BN5 chip 0x084): action 0x47, BN6's AirSpin (@common/airspin/action): a whirlwind top (chips/airspin/top) set down
-- ahead in palette 2, spinning 4 times.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local action = require("@common/airspin/action")
local top = require("../airspin/top")

return define.chip {
    id = "bn5:airspin3",
    codes = { "A", "G", "J" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 56,
    damage = 40,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = action.action { whirlwind = top.sender(2, 4) },
}
