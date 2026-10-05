--!strict
-- BoyBomb1, BoyBomb2 and BoyBomb3: the boybomb series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bomb = require("@self/bomb")

-- BoyBomb1 (EXE5 chip 0x055): a bomb set on the panel ahead that blows up over the 3x3 around it after 60 ticks
-- (chips/boybomb/bomb, the dimming handler's subtype 58).
local boybomb1 = define.chip {
    id = "boybomb1",
    codes = { "C", "F", "S" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 0,
    mb = 30,
    damage = 130,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = bomb.hook,
}

-- BoyBomb2 (EXE5 chip 0x056): a bomb set on the panel ahead that blows up over the 3x3 around it after 60 ticks
-- (chips/boybomb/bomb, the dimming handler's subtype 58).
local boybomb2 = define.chip {
    id = "boybomb2",
    codes = { "H", "P", "W" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 180,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = bomb.hook,
}

-- BoyBomb3 (EXE5 chip 0x057): a bomb set on the panel ahead that blows up over the 3x3 around it after 60 ticks
-- (chips/boybomb/bomb, the dimming handler's subtype 58).
local boybomb3 = define.chip {
    id = "boybomb3",
    codes = { "B", "Q", "Y" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 230,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = bomb.hook,
}

return { boybomb1 = boybomb1, boybomb2 = boybomb2, boybomb3 = boybomb3 }
