--!strict
-- Skully1, Skully2 and Skully3: the skully series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local skully = require("@self/skully")

-- Skully1 (EXE5 chip 0x045): skulls that hop toward the nearest enemy, confusing what they hit (EXE5's Skully,
-- chips/skully/skully: action 0x55).
local skully1: Chip = {
    codes = { "C", "K", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 34,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = skully.action(skully.skulls(0)),
    auto_battle = { class = "to_front" },
}

-- Skully2 (EXE5 chip 0x046): skulls that hop toward the nearest enemy, confusing what they hit (EXE5's Skully,
-- chips/skully/skully: action 0x55).
local skully2: Chip = {
    codes = { "A", "H", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 50,
    damage = 120,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = skully.action(skully.skulls(2)),
    auto_battle = { class = "to_front" },
}

-- Skully3 (EXE5 chip 0x047): skulls that hop toward the nearest enemy, confusing what they hit (EXE5's Skully,
-- chips/skully/skully: action 0x55).
local skully3: Chip = {
    codes = { "G", "N", "U" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 76,
    damage = 140,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = skully.action(skully.skulls(4)),
    auto_battle = { class = "to_front" },
}

return { skully1 = skully1, skully2 = skully2, skully3 = skully3 }
