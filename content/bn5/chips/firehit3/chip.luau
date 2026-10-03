--!strict
-- FireHit3 (BN5 chip 0x099): action 0x1A's effect 9, BN6's FireHit (objects/firehit: a fist on each row where
-- the first column ahead with one of the other side's has one, at most three panels ahead).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local fist = require("../../objects/firehit/fist")

return define.chip {
    id = "bn5:firehit3",
    codes = { "E", "F", "G" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 44,
    damage = 140,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = fist.instant,
}
