--!strict
-- CactBal1, CactBal2 and CactBal3: the cactbal series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local cactbal = require("@self/cactbal")

-- CactBal1 (BN5 chip 0x0af): the navi throws a wooden ball that hops along the row, bouncing back off what it hits:
-- BN5's own (chips/cactbal/cactbal), the ball in palette 0.
local cactbal1 = define.chip {
    id = "cactbal1",
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

-- CactBal2 (BN5 chip 0x0b0): the navi throws a wooden ball that hops along the row, bouncing back off what it hits:
-- BN5's own (chips/cactbal/cactbal), the ball in palette 2.
local cactbal2 = define.chip {
    id = "cactbal2",
    codes = { "Q", "R", "S" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 40,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = cactbal.action(1),
}

-- CactBal3 (BN5 chip 0x0b1): the navi throws a wooden ball that hops along the row, bouncing back off what it hits:
-- BN5's own (chips/cactbal/cactbal), the ball in palette 4.
local cactbal3 = define.chip {
    id = "cactbal3",
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

return { cactbal1 = cactbal1, cactbal2 = cactbal2, cactbal3 = cactbal3 }
