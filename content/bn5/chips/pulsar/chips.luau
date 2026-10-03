--!strict
-- Pulsar1, Pulsar2 and Pulsar3: the pulsar series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local pulsar = require("./pulsar")

-- Pulsar1 (BN5 chip 0x015): a pulse along the row that bursts around the first body it hits, paralyzing (BN5's
-- Pulsar, chips/pulsar/pulsar: action 0x52).
local pulsar1 = define.chip {
    id = "bn5:pulsar1",
    codes = { "F", "P", "T" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 70,
    hit_param = 20,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = pulsar.action,
}

-- Pulsar2 (BN5 chip 0x016): a pulse along the row that bursts around the first body it hits, paralyzing (BN5's
-- Pulsar, chips/pulsar/pulsar: action 0x52).
local pulsar2 = define.chip {
    id = "bn5:pulsar2",
    codes = { "E", "G", "M" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 26,
    damage = 90,
    hit_param = 20,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = pulsar.action,
}

-- Pulsar3 (BN5 chip 0x017): a pulse along the row that bursts around the first body it hits, paralyzing (BN5's
-- Pulsar, chips/pulsar/pulsar: action 0x52).
local pulsar3 = define.chip {
    id = "bn5:pulsar3",
    codes = { "C", "R", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 38,
    damage = 110,
    hit_param = 20,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = pulsar.action,
}

return { pulsar1 = pulsar1, pulsar2 = pulsar2, pulsar3 = pulsar3 }
