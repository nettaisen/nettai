--!strict
-- Vulcan1, Vulcan2, Vulcan3 and SuprVulc: the vulcan series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bullet = require("../objects/bullet")
local vulcan = require("@self/vulcan")

-- Vulcan1 (EXE5 chip 0x00f): action 0x19, EXE6's vulcan with EXE5's gun and bullets (chips/vulcan/vulcan): 3 bullets.
local vulcan1: Chip = {
    codes = { "C", "D", "E", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 6,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = vulcan.action(3),
}

-- Vulcan2 (EXE5 chip 0x010): action 0x19, EXE6's vulcan with EXE5's gun and bullets (chips/vulcan/vulcan): 5 bullets.
local vulcan2: Chip = {
    codes = { "A", "B", "C" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 18,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = vulcan.action(5),
}

-- Vulcan3 (EXE5 chip 0x011): action 0x19, EXE6's vulcan with EXE5's gun and bullets (chips/vulcan/vulcan): 7 bullets.
local vulcan3: Chip = {
    codes = { "L", "M", "N" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = vulcan.action(7),
}

-- SuprVulc (EXE5 chip 0x0dc): action 0x19, EXE6's vulcan with EXE5's gun and bullets (chips/vulcan/vulcan): 12 bullets of
-- row 0x10 (EXE6's SuprVulc's).
local suprvulc: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = 10,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = vulcan.action(12, bullet.variants.super_vulcan),
}

return { vulcan1 = vulcan1, vulcan2 = vulcan2, vulcan3 = vulcan3, suprvulc = suprvulc }
