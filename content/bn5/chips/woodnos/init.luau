--!strict
-- WoodNos1, WoodNos2 and WoodNos3: the woodnos series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local woodnos = require("@self/woodnos")

-- WoodNos1 (BN5 chip 0x0b2): the navi sends a wooden wave along its row; past the field it raises geysers under the
-- other side's navis: BN5's own (chips/woodnos/woodnos).
local woodnos1 = define.chip {
    id = "woodnos1",
    codes = { "F", "R", "W" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 0,
    mb = 27,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = woodnos.action,
}

-- WoodNos2 (BN5 chip 0x0b3): the navi sends a wooden wave along its row; past the field it raises geysers under the
-- other side's navis: BN5's own (chips/woodnos/woodnos).
local woodnos2 = define.chip {
    id = "woodnos2",
    codes = { "J", "T", "Z" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = woodnos.action,
}

-- WoodNos3 (BN5 chip 0x0b4): the navi sends a wooden wave along its row; past the field it raises geysers under the
-- other side's navis: BN5's own (chips/woodnos/woodnos).
local woodnos3 = define.chip {
    id = "woodnos3",
    codes = { "I", "S", "V" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 53,
    damage = 160,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = woodnos.action,
}

return { woodnos1 = woodnos1, woodnos2 = woodnos2, woodnos3 = woodnos3 }
