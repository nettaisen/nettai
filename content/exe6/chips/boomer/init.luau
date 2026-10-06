--!strict
-- Boomer, HiBoomer, M-Boomer: a boomerang (./boomerang) flies from
-- the user's back column along row 3, up the far column and back along
-- row 1, turning the other side's panels it crosses to grass; faster with
-- each (the original's rows 0 to 2 of `byte_80CA26C`).

local boomerang = require("@self/boomerang")

return {
    boomer = define.chip {
        id = "boomer",
        codes = { "J", "K", "T" },
        element = "wood", family = "wood", class = "standard",
        rarity = 1, mb = 16, damage = 100, hit_param = 10, lockout = 20,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        instant = boomerang.instant(boomerang.variant { speed = 0x8_0000, turn_speed = 314573, grass = true }),
    },
    hiboomer = define.chip {
        id = "hiboomer",
        codes = { "B", "L", "V" },
        element = "wood", family = "wood", class = "standard",
        rarity = 2, mb = 26, damage = 140, hit_param = 10, lockout = 20,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        instant = boomerang.instant(boomerang.variant { speed = 0x9_0000, turn_speed = 353894, grass = true }),
    },
    m_boomer = define.chip {
        id = "m-boomer",
        codes = { "I", "M", "W" },
        element = "wood", family = "wood", class = "standard",
        rarity = 3, mb = 36, damage = 170, hit_param = 10, lockout = 20,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        instant = boomerang.instant(boomerang.variant { speed = 0xA_0000, turn_speed = 393216, grass = true }),
    },
}
