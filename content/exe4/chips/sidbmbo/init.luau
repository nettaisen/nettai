--!strict
-- SidBmbo1, SidBmbo2 and SidBmbo3 (EXE4 chips 0x04E to 0x050; their records at 0x080197EC + n * 0x2C): the spawners'
-- action 0x20 (lib/spawners), its variant 5, which holds the user 40 ticks: a bamboo swings down three columns ahead
-- (./bamboo), their parameter (0 to 2) its palette.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bamboo = require("@self/bamboo")
local spawners = require("../lib/spawners")

-- The spawners' hold for variant 5 (0x080EC9B0).
local HOLD = 40

local sidbmbo1: Chip = {
    action = spawners.action { id = "sidbmbo1/action", spawn = bamboo.spawner(0), hold = HOLD },
    codes = { "B", "I", "R", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 80,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local sidbmbo2: Chip = {
    action = spawners.action { id = "sidbmbo2/action", spawn = bamboo.spawner(1), hold = HOLD },
    codes = { "H", "O", "S" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 1,
    mb = 22,
    damage = 110,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local sidbmbo3: Chip = {
    action = spawners.action { id = "sidbmbo3/action", spawn = bamboo.spawner(2), hold = HOLD },
    codes = { "A", "F", "U" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 140,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { sidbmbo1 = sidbmbo1, sidbmbo2 = sidbmbo2, sidbmbo3 = sidbmbo3 }
