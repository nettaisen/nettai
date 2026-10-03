--!strict
-- HotBody1, HotBody2 and HotBody3: the hotbody series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local hotbody = require("./hotbody")

-- HotBody1 (BN5 chip 0x09a): the navi bursts into flames that spread to the enemies around it, two rounds 30 ticks
-- apart (chips/hotbody/hotbody, action 0x58; the record's second parameter, 2).
local hotbody1 = define.chip {
    id = "bn5:hotbody1",
    codes = { "A", "F", "P" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 28,
    damage = 110,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = hotbody.action("bn5:hotbody1/action", 2),
}

-- HotBody2 (BN5 chip 0x09b): the navi bursts into flames that spread to the enemies around it, two rounds 30 ticks
-- apart (chips/hotbody/hotbody, action 0x58; the record's second parameter, 2).
local hotbody2 = define.chip {
    id = "bn5:hotbody2",
    codes = { "C", "R", "W" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = hotbody.action("bn5:hotbody2/action", 2),
}

-- HotBody3 (BN5 chip 0x09c): the navi bursts into flames that spread to the enemies around it, two rounds 30 ticks
-- apart (chips/hotbody/hotbody, action 0x58; the record's second parameter, 2).
local hotbody3 = define.chip {
    id = "bn5:hotbody3",
    codes = { "J", "M", "S" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 52,
    damage = 150,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = hotbody.action("bn5:hotbody3/action", 2),
}

return { hotbody1 = hotbody1, hotbody2 = hotbody2, hotbody3 = hotbody3 }
