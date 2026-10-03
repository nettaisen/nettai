--!strict
-- AqWhirl1 (BN5 chip 0x09d): action 0x47 as variant 0 (@common/airspin/action): a whirlpool (chips/aqwhirl/whirlpool)
-- set down ahead in palette 0 at speed 0x20, looping round the field.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local action = require("@common/airspin/action")
local whirlpool = require("../aqwhirl/whirlpool")

return define.chip {
    id = "bn5:aqwhirl1",
    codes = { "B", "F", "N" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 30,
    damage = 70,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = action.action { whirlwind = whirlpool.sender(0, 0x20) },
}
