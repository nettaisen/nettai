--!strict
-- WindRack: a swing of a rack that hits the column ahead and blows unseen
-- gusts down every row from it (./action).

local action = require("./action")

return define.chip {
    id = "windrack",
    name = "WindRack",
    description = "Blow enmy\nin front!\nRange: 3",
    codes = { "F", "J", "R", "*" },
    element = "null", family = "wind", class = "standard",
    rarity = 2, mb = 19, damage = 140, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 79, index = 79, sort = 511 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
    beast = { lockon = 3 },
    action = action.action { rack = action.racks.wind_rack },
}
