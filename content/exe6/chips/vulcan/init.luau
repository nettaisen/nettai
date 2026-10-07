--!strict
-- Vulcan1-3 and SuprVulc: a burst of bullets from the panel ahead
-- (action.luau). They differ in the number of bullets (3, 4, 5, 10) and
-- SuprVulc's bullet's look.

local bullet = require("../objects/bullet")
local lockon = require("../rules/lockon")
local regions = require("@exelib/regions")
local vulcan = require("@self/action")

-- The Vulcans' bullet (the bullet's row 0xC): the panel found and the one
-- behind it, staggering, its hit shown once at the bullet.
local SHOT = bullet.variant { region = regions.long, hit_mod = 1, effect = bullet.hit, single = true }
-- SuprVulc's (row 0x10): the same, its hit brighter.
local SUPER_SHOT = bullet.variant {
    region = regions.long,
    hit_mod = 1,
    effect = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 1 },
    single = true,
}

local chips: { [string]: Chip } = {
    vulcan1 = {
        codes = { "B", "D", "S", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 10, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = vulcan.action { shots = 3, bullet = SHOT },
    },
    vulcan2 = {
        codes = { "D", "F", "L" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 18, damage = 15, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = vulcan.action { shots = 4, bullet = SHOT },
    },
    vulcan3 = {
        codes = { "A", "G", "R" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 30, damage = 20, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = vulcan.action { shots = 5, bullet = SHOT },
    },
    suprvulc = {
        codes = { "V" },
        element = "null", family = "null", class = "standard",
        rarity = 3, mb = 75, damage = 20, hit_param = 4, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = vulcan.action { shots = 10, bullet = SUPER_SHOT },
    },
}

return chips
