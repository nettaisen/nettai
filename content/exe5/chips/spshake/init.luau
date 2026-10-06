--!strict
-- SpShake1, SpShake2 and SpShake3: the spshake series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local spshake = require("@self/spshake")

-- SpShake1 (EXE5 chip 0x018): an electric wave that snakes along the field from the panel ahead (EXE5's SpShake,
-- chips/spshake/spshake: action 0x51).
local spshake1: Chip = {
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
    action = spshake.action(spshake.waves[1]),
    auto_battle = { class = "into_reach", range = "all_ahead" },
}

-- SpShake2 (EXE5 chip 0x019): an electric wave that snakes along the field from the panel ahead (EXE5's SpShake,
-- chips/spshake/spshake: action 0x51).
local spshake2: Chip = {
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
    action = spshake.action(spshake.waves[2]),
    auto_battle = { class = "into_reach", range = "all_ahead" },
}

-- SpShake3 (EXE5 chip 0x01a): an electric wave that snakes along the field from the panel ahead (EXE5's SpShake,
-- chips/spshake/spshake: action 0x51).
local spshake3: Chip = {
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
    action = spshake.action(spshake.waves[3]),
    auto_battle = { class = "into_reach", range = "all_ahead" },
}

return { spshake1 = spshake1, spshake2 = spshake2, spshake3 = spshake3 }
