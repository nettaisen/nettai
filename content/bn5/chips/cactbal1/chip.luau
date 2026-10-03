--!strict
-- CactBal1 (BN5 chip 0x0af): the navi throws a wooden ball that hops along the row, bouncing back off what it hits:
-- BN5's own (objects/cactbal), the ball in palette 0.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local cactbal = require("../../objects/cactbal/cactbal")

return define.chip {
    id = "bn5:cactbal1",
    codes = { "H", "I", "J" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 30,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = cactbal.action(0),
}
