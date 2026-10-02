--!strict
-- RskyHny1-3: a hive that sends a bee ahead, and another for each hit the
-- navi takes in the next three windows (./action, ./bee); each level's
-- bees fly faster.

local action = require("./action")
local lockon = require("../../rules/lockon")

local BEES = action.bees

return {
    define.chip {
        id = "rskyhny1",
        codes = { "B", "G", "S" },
        element = "wood", family = "wood", class = "standard",
        rarity = 0, mb = 21, damage = 10, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 39, index = 39, sort = 351 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = action.action { bee = BEES[1] },
    },
    define.chip {
        id = "rskyhny2",
        codes = { "C", "R", "V" },
        element = "wood", family = "wood", class = "standard",
        rarity = 1, mb = 28, damage = 15, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 40, index = 40, sort = 399 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = action.action { bee = BEES[2] },
    },
    define.chip {
        id = "rskyhny3",
        codes = { "A", "D", "M" },
        element = "wood",
        family = "wood",
        class = "standard",
        rarity = 2,
        mb = 35,
        damage = 20,
        hit_param = 30,
        lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        beast = { lockon = lockon.cannon },
        library = { number = 41, index = 41, sort = 400 },
        slot_in_limit = 3,
        action = action.action { id = "rskyhny3/action", bee = BEES[3] },
    },
}
