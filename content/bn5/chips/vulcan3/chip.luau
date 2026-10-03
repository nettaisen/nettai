--!strict
-- Vulcan3 (BN5 chip 0x011): action 0x19, BN6's vulcan with BN5's gun and bullets (objects/vulcan): 7 bullets.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local vulcan = require("../../objects/vulcan/vulcan")

return define.chip {
    id = "bn5:vulcan3",
    codes = { "L", "M", "N" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = vulcan.action(7),
}
