--!strict
-- SpShake1, SpShake2 and SpShake3: the spshake series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local spshake = require("@self/spshake")

-- Their waves: 1.5, 2.25 and 3 pixels a tick (rows 0 to 2 of 0x080D30D4), in palettes 0, 6 and 12.
local WAVE1 = spshake.wave { speed = 0x1_8000, paralyzes = false, palette = 0 }
local WAVE2 = spshake.wave { speed = 0x2_4000, paralyzes = false, palette = 6 }
local WAVE3 = spshake.wave { speed = 0x3_0000, paralyzes = false, palette = 0x0C }

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
    action = spshake.action(WAVE1),
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
    action = spshake.action(WAVE2),
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
    action = spshake.action(WAVE3),
    auto_battle = { class = "into_reach", range = "all_ahead" },
}

return { spshake1 = spshake1, spshake2 = spshake2, spshake3 = spshake3 }
