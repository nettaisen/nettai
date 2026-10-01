--!strict
-- Vulcan1-3 and SuprVulc: a burst of bullets from the panel ahead
-- (action.luau). They differ in the number of bullets (3, 4, 5, 10) and
-- SuprVulc's bullet's look.

local vulcan = require("./action")

return {
    define.chip {
        id = "vulcan1",
        name = "Vulcan1",
        codes = { "B", "D", "S", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 10, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 5, index = 5, sort = 500 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 1 },
        action = vulcan.action { shots = 3, bullet = vulcan.bullets.plain },
    },
    define.chip {
        id = "vulcan2",
        name = "Vulcan2",
        codes = { "D", "F", "L" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 18, damage = 15, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 6, index = 6, sort = 501 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 1 },
        action = vulcan.action { shots = 4, bullet = vulcan.bullets.plain },
    },
    define.chip {
        id = "vulcan3",
        name = "Vulcan3",
        codes = { "A", "G", "R" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 30, damage = 20, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 7, index = 7, sort = 502 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 1 },
        action = vulcan.action { shots = 5, bullet = vulcan.bullets.plain },
    },
    define.chip {
        id = "suprvulc",
        name = "SuprVulc",
        codes = { "V" },
        element = "null", family = "null", class = "standard",
        rarity = 3, mb = 75, damage = 20, hit_param = 4, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 8, index = 8, sort = 425 },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 1 },
        action = vulcan.action { shots = 10, bullet = vulcan.bullets.super },
    },
}
