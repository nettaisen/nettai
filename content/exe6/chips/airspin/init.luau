--!strict
-- AirSpin1-3: a whirlwind top set down ahead that rolls forward and
-- spins, hitting the panels around it (./action, ./top); each level's top
-- in its own colors spins once more.

local action = require("@self/action")
local top = require("@self/top")

local chips: { [string]: Chip } = {
    airspin1 = {
        codes = { "F", "G", "R" },
        element = "null", family = "wind", class = "standard",
        rarity = 1, mb = 22, damage = 50, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        action = action.action { whirlwind = top.sender(0, 2) },
    },
    airspin2 = {
        codes = { "A", "L", "T" },
        element = "null", family = "wind", class = "standard",
        rarity = 2, mb = 29, damage = 50, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        action = action.action { whirlwind = top.sender(4, 3) },
    },
    airspin3 = {
        codes = { "N", "O", "T" },
        element = "null", family = "wind", class = "standard",
        rarity = 3, mb = 36, damage = 50, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        action = action.action { whirlwind = top.sender(8, 4) },
    },
}

return chips
