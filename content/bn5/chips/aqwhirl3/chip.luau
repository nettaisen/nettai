--!strict
-- AqWhirl3 (BN5 chip 0x09f): action 0x47 as variant 0 (@common/airspin/action): a whirlpool (chips/aqwhirl/whirlpool)
-- set down ahead in palette 2 at speed 0x50, looping round the field.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local action = require("@common/airspin/action")
local whirlpool = require("../aqwhirl/whirlpool")

return define.chip {
    id = "bn5:aqwhirl3",
    codes = { "D", "I", "R" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 54,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = action.action { whirlwind = whirlpool.sender(2, 0x50) },
}
