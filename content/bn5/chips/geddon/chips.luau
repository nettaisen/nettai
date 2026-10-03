--!strict
-- Geddon1, Geddon2 and Geddon3: the geddon series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("./controller")

-- Geddon1 (BN5 chip 0x07b): a quake cracks every solid panel of the field, one every 8 ticks in a random order:
-- BN6's Geddon (its quake's first row) without holding the user's panel (chips/geddon/controller).
local geddon1 = define.chip {
    id = "bn5:geddon1",
    codes = { "M", "P", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 34,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook("crack"),
}

-- Geddon2 (BN5 chip 0x07c): a quake breaks every solid panel of the field (cracking those something stands on), one
-- every 8 ticks in a random order: BN6's Geddon without holding the user's panel (chips/geddon/controller).
local geddon2 = define.chip {
    id = "bn5:geddon2",
    codes = { "H", "T", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 47,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook("break"),
}

-- Geddon3 (BN5 chip 0x07d): a quake turns every solid panel of the field to poison, one every 8 ticks in a random
-- order: BN6's Geddon (its quake's third row, which no BN6 chip uses) without holding the user's panel
-- (chips/geddon/controller).
local geddon3 = define.chip {
    id = "bn5:geddon3",
    codes = { "A", "O", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 62,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook("poison"),
}

return { geddon1 = geddon1, geddon2 = geddon2, geddon3 = geddon3 }
