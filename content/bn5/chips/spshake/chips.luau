--!strict
-- SpShake1, SpShake2 and SpShake3: the spshake series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local spshake = require("./spshake")

-- SpShake1 (BN5 chip 0x018): an electric wave that snakes along the field from the panel ahead (BN5's SpShake,
-- chips/spshake/spshake: action 0x51).
local spshake1 = define.chip {
    id = "spshake1",
    codes = { "G", "H", "I" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 30,
    damage = 60,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = spshake.action("spshake1/action", spshake.waves[1]),
}

-- SpShake2 (BN5 chip 0x019): an electric wave that snakes along the field from the panel ahead (BN5's SpShake,
-- chips/spshake/spshake: action 0x51).
local spshake2 = define.chip {
    id = "spshake2",
    codes = { "B", "C", "D" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 80,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = spshake.action("spshake2/action", spshake.waves[2]),
}

-- SpShake3 (BN5 chip 0x01a): an electric wave that snakes along the field from the panel ahead (BN5's SpShake,
-- chips/spshake/spshake: action 0x51).
local spshake3 = define.chip {
    id = "spshake3",
    codes = { "S", "T", "U" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 100,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = spshake.action("spshake3/action", spshake.waves[3]),
}

return { spshake1 = spshake1, spshake2 = spshake2, spshake3 = spshake3 }
