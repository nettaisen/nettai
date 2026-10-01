--!strict
-- RlngLog1-3: two big logs thrown onto the panel ahead that roll down the
-- rows (./action, ./log). The levels differ only in damage.

local action = require("./action")

-- Every level's log: big, stopped 60 ticks by a hit, 20 HP, thrown (the
-- original's parameters 1, 60, 20, 1).
local LOG: RollingLog = { big = true, stop_ticks = 60, hp = 20, thrown = true }

return {
    define.chip {
        id = "rlnglog1",
        name = "RlngLog1",
        codes = { "I", "K", "P" },
        element = "wood", family = "wood", class = "standard",
        rarity = 0, mb = 14, damage = 50, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 42, index = 42, sort = 344 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 9 },
        action = action.action { log = LOG },
    },
    define.chip {
        id = "rlnglog2",
        name = "RlngLog2",
        codes = { "E", "Q", "Z" },
        element = "wood", family = "wood", class = "standard",
        rarity = 1, mb = 26, damage = 70, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 43, index = 43, sort = 345 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 9 },
        action = action.action { log = LOG },
    },
    define.chip {
        id = "rlnglog3",
        name = "RlngLog3",
        codes = { "F", "N", "W" },
        element = "wood", family = "wood", class = "standard",
        rarity = 2, mb = 38, damage = 90, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 44, index = 44, sort = 346 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 9 },
        action = action.action { log = LOG },
    },
}
