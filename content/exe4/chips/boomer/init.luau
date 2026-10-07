--!strict
-- Boomer1, Boomer2 and Boomer3 (EXE4 chips 0x04B to 0x04D; their records at 0x080197EC + n * 0x2C): the spawners'
-- action 0x20 (lib/spawners), its variant 2: a boomerang (./boomerang, its row 5) flies from the user's back column
-- along row 3, up the far column and back along row 1.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local boomerang = require("@self/boomerang")
local spawners = require("../lib/spawners")

local THROW = boomerang.spawner(boomerang.boomer)

local boomer1: Chip = {
    action = spawners.action { id = "boomer1/action", spawn = THROW },
    codes = { "L", "M", "N", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 0,
    mb = 16,
    damage = 60,
    hit_param = 10,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
}

local boomer2: Chip = {
    action = spawners.action { id = "boomer2/action", spawn = THROW },
    codes = { "L", "M", "N" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 1,
    mb = 28,
    damage = 80,
    hit_param = 10,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
}

local boomer3: Chip = {
    action = spawners.action { id = "boomer3/action", spawn = THROW },
    codes = { "S", "T", "U" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 100,
    hit_param = 10,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
}

return { boomer1 = boomer1, boomer2 = boomer2, boomer3 = boomer3 }
