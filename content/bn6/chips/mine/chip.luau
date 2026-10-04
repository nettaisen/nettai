--!strict
-- Mine: a mine that hops among the enemy's free panels for two seconds,
-- then hides where it stopped and blows up under what touches it
-- (./controller, ./land_mine).

local controller = require("./controller")

return define.chip {
    id = "mine",
    codes = { "A", "S", "T" },
    element = "null", family = "summon", class = "standard",
    rarity = 2, mb = 28, damage = 200, hit_param = 178, lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    library = { number = 148, index = 148, sort = 316 },
    slot_in_limit = 3,
    dimming = controller.hook,
}
