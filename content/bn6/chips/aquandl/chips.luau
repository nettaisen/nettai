--!strict
-- AquaNdl1-3: a volley of three aqua needles dropped on the enemies'
-- panels (./action), each level's needles in its own colors.

local action = require("./action")

return {
    define.chip {
        id = "bn6:aquandl1",
        codes = { "C", "J", "P" },
        element = "aqua", family = "aqua", class = "standard",
        rarity = 0, mb = 31, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 112, index = 112, sort = 15 },
        slot_in_limit = 3,
        action = action.action { palette = 0 },
    },
    define.chip {
        id = "bn6:aquandl2",
        codes = { "F", "K", "T" },
        element = "aqua", family = "aqua", class = "standard",
        rarity = 1, mb = 35, damage = 60, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 113, index = 113, sort = 16 },
        slot_in_limit = 3,
        action = action.action { palette = 1 },
    },
    define.chip {
        id = "bn6:aquandl3",
        codes = { "A", "L", "U" },
        element = "aqua",
        family = "aqua",
        class = "standard",
        rarity = 2,
        mb = 39,
        damage = 80,
        hit_param = 30,
        lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 114, index = 114, sort = 17 },
        slot_in_limit = 3,
        action = action.action { id = "bn6:aquandl3/action", palette = 2 },
    },
}
