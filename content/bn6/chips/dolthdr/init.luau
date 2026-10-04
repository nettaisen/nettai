--!strict
-- DolThdr1-3: a doll held out that strikes thunder along the row ahead
-- (./action, ./doll). The levels differ only in damage.

local action = require("@self/action")
local lockon = require("../rules/lockon")

-- Every level's thunder lasts 15 ticks (the original's second
-- parameter).
local TICKS = 0x0F

return {
    define.chip {
        id = "dolthdr1",
        codes = { "A", "E", "Q" },
        element = "elec", family = "elec", class = "standard",
        rarity = 0, mb = 24, damage = 120, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 30, index = 30, sort = 128 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = action.action { ticks = TICKS },
    },
    define.chip {
        id = "dolthdr2",
        codes = { "C", "L", "P" },
        element = "elec", family = "elec", class = "standard",
        rarity = 1, mb = 31, damage = 150, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 31, index = 31, sort = 129 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = action.action { ticks = TICKS },
    },
    define.chip {
        id = "dolthdr3",
        codes = { "B", "R", "V" },
        element = "elec", family = "elec", class = "standard",
        rarity = 2, mb = 38, damage = 180, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 32, index = 32, sort = 130 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = action.action { ticks = TICKS },
    },
}
