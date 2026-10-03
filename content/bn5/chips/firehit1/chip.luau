--!strict
-- FireHit1 (BN5 chip 0x097): action 0x1A's effect 9, BN6's FireHit (objects/firehit: a fist on each row where
-- the first column ahead with one of the other side's has one, at most three panels ahead).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local fist = require("../../objects/firehit/fist")

return define.chip {
    id = "bn5:firehit1",
    codes = { "O", "P", "Q" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 12,
    damage = 60,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = fist.instant,
}
