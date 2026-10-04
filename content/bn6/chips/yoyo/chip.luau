--!strict
-- YoYo: a yoyo thrown from the panel ahead that rolls out two panels,
-- spins there for two rounds and rolls back (throw.luau).

local throw = require("./throw")
local lockon = require("../../rules/lockon")

return define.chip {
    id = "yoyo",
    codes = { "L", "M", "N", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 32,
    damage = 50,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.yoyo },
    library = { number = 18, index = 18, sort = 513 },
    slot_in_limit = 3,
    action = throw.action { id = "yoyo/action", yoyo = { panels = 2, rounds = 2 } },
}
