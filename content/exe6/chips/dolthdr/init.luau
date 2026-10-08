--!strict
-- DolThdr1-3: a doll held out that strikes thunder along the row ahead
-- (./action, ./doll). The levels differ only in damage.

local action = require("@self/action")
local attachment = require("../objects/attachment")
local lockon = require("../rules/lockon")

-- Every level's thunder lasts 15 ticks (the original's second
-- parameter).
local TICKS = 0x0F
-- The doll the navi holds out (the original's attachment row 0x29).
local DOLL = attachment.look { sprite = asset.sprite("thunder-doll-hand"), palette = 0, attach_point = 0x0E }

local chips: { [string]: Chip } = {
    dolthdr1 = {
        codes = { "A", "E", "Q" },
        element = "elec", family = "elec", class = "standard",
        rarity = 0, mb = 24, damage = 120, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = action.action { ticks = TICKS, held = DOLL },
    },
    dolthdr2 = {
        codes = { "C", "L", "P" },
        element = "elec", family = "elec", class = "standard",
        rarity = 1, mb = 31, damage = 150, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = action.action { ticks = TICKS, held = DOLL },
    },
    dolthdr3 = {
        codes = { "B", "R", "V" },
        element = "elec", family = "elec", class = "standard",
        rarity = 2, mb = 38, damage = 180, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = action.action { ticks = TICKS, held = DOLL },
    },
}

return chips
