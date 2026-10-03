--!strict
-- Vulcan1 (BN5 chip 0x00f): action 0x19, BN6's vulcan with BN5's gun and bullets (objects/vulcan): 3 bullets.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local vulcan = require("../../objects/vulcan/vulcan")

return define.chip {
    id = "bn5:vulcan1",
    codes = { "C", "D", "E", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 6,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = vulcan.action(3),
}
