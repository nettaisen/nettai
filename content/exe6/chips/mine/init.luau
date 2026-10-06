--!strict
-- Mine: a mine that hops among the enemy's free panels for two seconds,
-- then hides where it stopped and blows up under what touches it
-- (./controller, ./land_mine).

local controller = require("@self/controller")

local chip: Chip = {
    codes = { "A", "S", "T" },
    element = "null", family = "summon", class = "standard",
    rarity = 2, mb = 28, damage = 200, hit_param = 178, lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = controller.hook,
}

return { mine = chip }
