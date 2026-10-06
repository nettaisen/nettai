--!strict
-- Geddon1, Geddon2 and Geddon3: the geddon series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

-- Geddon1 (EXE5 chip 0x07b): a quake cracks every solid panel of the field, one every 8 ticks in a random order:
-- EXE6's Geddon (its quake's first row) without holding the user's panel (chips/geddon/controller).
local geddon1: Chip = {
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
    auto_battle = { class = "at_once" },
}

-- Geddon2 (EXE5 chip 0x07c): a quake breaks every solid panel of the field (cracking those something stands on), one
-- every 8 ticks in a random order: EXE6's Geddon without holding the user's panel (chips/geddon/controller).
local geddon2: Chip = {
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
    auto_battle = { class = "at_once" },
}

-- Geddon3 (EXE5 chip 0x07d): a quake turns every solid panel of the field to poison, one every 8 ticks in a random
-- order: EXE6's Geddon (its quake's third row, which no EXE6 chip uses) without holding the user's panel
-- (chips/geddon/controller).
local geddon3: Chip = {
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
    auto_battle = { class = "at_once" },
}

return { geddon1 = geddon1, geddon2 = geddon2, geddon3 = geddon3 }
