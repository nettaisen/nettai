--!strict
-- WindRack: a swing of a rack that hits the column ahead and blows unseen
-- gusts down every row from it (./action).

local action = require("@self/action")
local lockon = require("../rules/lockon")

local chip: Chip = {
    codes = { "F", "J", "R", "*" },
    element = "null", family = "wind", class = "standard",
    rarity = 2, mb = 19, damage = 140, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.widesht },
    action = action.action { rack = action.racks.wind_rack },
}

return { windrack = chip }
