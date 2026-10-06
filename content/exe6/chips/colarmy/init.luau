--!strict
-- ColArmy: every obstacle on the field blinks out, and a soldier
-- (army.luau) takes its panel and fires three shots.

local army = require("@self/army")

return define.chip {
    id = "colarmy",
    codes = { "B", "G", "R", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 2, mb = 25, damage = 40, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    instant = army.instant,
}
