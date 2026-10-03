--!strict
-- FireHit2 (BN5 chip 0x098): action 0x1A's effect 9, BN6's FireHit (objects/firehit: a fist on each row where
-- the first column ahead with one of the other side's has one, at most three panels ahead).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local fist = require("../../objects/firehit/fist")

return define.chip {
    id = "bn5:firehit2",
    codes = { "H", "I", "J" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 28,
    damage = 100,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = fist.instant,
}
