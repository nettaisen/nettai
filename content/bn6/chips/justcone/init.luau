--!strict
-- JustcOne: a Break fist (strike.luau) falls onto the other side's middle
-- panel and cracks the panels around it.

local strike = require("@self/strike")

return define.chip {
    id = "justcone",
    codes = { "J" },
    element = "null", family = "break", class = "standard",
    rarity = 4, mb = 90, damage = 220, hit_param = 178, lockout = 20,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 130, index = 130, sort = 251 },
    slot_in_limit = 1,
    instant = strike.instant,
}
