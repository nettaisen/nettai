--!strict
-- Katana1, Katana2 and Katana3: the katana series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local katana = require("./katana")

-- Katana1 (BN5 chip 0x03c): two slashes, a wide one and then a long one (BN5's Katana, chips/katana/katana: action 0x54,
-- its own action on the swords' slash; a charged one steps first).
local katana1 = define.chip {
    id = "katana1",
    codes = { "D", "H", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 0,
    mb = 25,
    damage = 60,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = katana.action,
}

-- Katana2 (BN5 chip 0x03d): two slashes, a wide one and then a long one (BN5's Katana, chips/katana/katana: action 0x54,
-- its own action on the swords' slash; a charged one steps first).
local katana2 = define.chip {
    id = "katana2",
    codes = { "B", "R", "W" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 1,
    mb = 37,
    damage = 80,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = katana.action,
}

-- Katana3 (BN5 chip 0x03e): two slashes, a wide one and then a long one (BN5's Katana, chips/katana/katana: action 0x54,
-- its own action on the swords' slash; a charged one steps first).
local katana3 = define.chip {
    id = "katana3",
    codes = { "A", "J", "Z" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 49,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = katana.action,
}

return { katana1 = katana1, katana2 = katana2, katana3 = katana3 }
