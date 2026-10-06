--!strict
-- Vulcan1-3 and SuprVulc: a burst of bullets from the panel ahead
-- (action.luau). They differ in the number of bullets (3, 4, 5, 10) and
-- SuprVulc's bullet's look.

local lockon = require("../rules/lockon")
local vulcan = require("@self/action")

local chips: { [string]: Chip } = {
    vulcan1 = {
        codes = { "B", "D", "S", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 10, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = vulcan.action { shots = 3, bullet = vulcan.bullets.plain },
    },
    vulcan2 = {
        codes = { "D", "F", "L" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 18, damage = 15, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = vulcan.action { shots = 4, bullet = vulcan.bullets.plain },
    },
    vulcan3 = {
        codes = { "A", "G", "R" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 30, damage = 20, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = vulcan.action { shots = 5, bullet = vulcan.bullets.plain },
    },
    suprvulc = {
        codes = { "V" },
        element = "null", family = "null", class = "standard",
        rarity = 3, mb = 75, damage = 20, hit_param = 4, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = vulcan.action { shots = 10, bullet = vulcan.bullets.super },
    },
}

return chips
