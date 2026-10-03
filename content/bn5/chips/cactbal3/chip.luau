--!strict
-- CactBal3 (BN5 chip 0x0b1): the navi throws a wooden ball that hops along the row, bouncing back off what it hits:
-- BN5's own (objects/cactbal), the ball in palette 4.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local cactbal = require("../../objects/cactbal/cactbal")

return define.chip {
    id = "bn5:cactbal3",
    codes = { "L", "M", "N" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 50,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = cactbal.action(2),
}
