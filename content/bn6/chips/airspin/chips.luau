--!strict
-- AirSpin1-3: a whirlwind top set down ahead that rolls forward and
-- spins, hitting the panels around it (./action, ./top); each level's top
-- in its own colours spins once more.

local action = require("./action")
local top = require("./top")

return {
    define.chip {
        id = "airspin1",
        name = "AirSpin1",
        description = "Spits out\nhurricane\nattack",
        codes = { "F", "G", "R" },
        element = "null", family = "wind", class = "standard",
        rarity = 1, mb = 22, damage = 50, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 131, index = 131, sort = 6 },
        slot_in_limit = 3,
        action = action.action { whirlwind = top.sender(0, 2) },
    },
    define.chip {
        id = "airspin2",
        name = "AirSpin2",
        description = "Spits out\nhurricane\nattack",
        codes = { "A", "L", "T" },
        element = "null", family = "wind", class = "standard",
        rarity = 2, mb = 29, damage = 50, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 132, index = 132, sort = 7 },
        slot_in_limit = 3,
        action = action.action { whirlwind = top.sender(1, 3) },
    },
    define.chip {
        id = "airspin3",
        name = "AirSpin3",
        description = "Spits out\nhurricane\nattack",
        codes = { "N", "O", "T" },
        element = "null", family = "wind", class = "standard",
        rarity = 3, mb = 36, damage = 50, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 133, index = 133, sort = 8 },
        slot_in_limit = 3,
        action = action.action { whirlwind = top.sender(2, 4) },
    },
}
