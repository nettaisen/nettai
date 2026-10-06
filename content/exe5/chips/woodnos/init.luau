--!strict
-- WoodNos1, WoodNos2 and WoodNos3: the woodnos series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local woodnos = require("@self/woodnos")

-- WoodNos1 (EXE5 chip 0x0b2): the navi sends a wooden wave along its row; past the field it raises geysers under the
-- other side's navis: EXE5's own (chips/woodnos/woodnos).
local woodnos1: Chip = {
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

-- WoodNos2 (EXE5 chip 0x0b3): the navi sends a wooden wave along its row; past the field it raises geysers under the
-- other side's navis: EXE5's own (chips/woodnos/woodnos).
local woodnos2: Chip = {
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

-- WoodNos3 (EXE5 chip 0x0b4): the navi sends a wooden wave along its row; past the field it raises geysers under the
-- other side's navis: EXE5's own (chips/woodnos/woodnos).
local woodnos3: Chip = {
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
