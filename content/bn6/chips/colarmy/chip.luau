--!strict
-- ColArmy: every obstacle on the field blinks out, and a soldier
-- (army.luau) takes its panel and fires three shots.

local army = require("./army")

return define.chip {
    id = "colarmy",
    codes = { "B", "G", "R", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 2, mb = 25, damage = 40, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 186, index = 186, sort = 103 },
    slot_in_limit = 3,
    instant = army.instant,
}
