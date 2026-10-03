--!strict
-- FireHit1, FireHit2 and FireHit3: the firehit series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local fist = require("./fist")

-- FireHit1 (BN5 chip 0x097): action 0x1A's effect 9, BN6's FireHit (chips/firehit/fist: a fist on each row where
-- the first column ahead with one of the other side's has one, at most three panels ahead).
local firehit1 = define.chip {
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

-- FireHit2 (BN5 chip 0x098): action 0x1A's effect 9, BN6's FireHit (chips/firehit/fist: a fist on each row where
-- the first column ahead with one of the other side's has one, at most three panels ahead).
local firehit2 = define.chip {
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

-- FireHit3 (BN5 chip 0x099): action 0x1A's effect 9, BN6's FireHit (chips/firehit/fist: a fist on each row where
-- the first column ahead with one of the other side's has one, at most three panels ahead).
local firehit3 = define.chip {
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

return { firehit1 = firehit1, firehit2 = firehit2, firehit3 = firehit3 }
