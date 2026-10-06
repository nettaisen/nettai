--!strict
-- JustcOne: a Break fist (strike.luau) falls onto the other side's middle
-- panel and cracks the panels around it.

local strike = require("@self/strike")

local chip: Chip = {
    codes = { "J" },
    element = "null", family = "break", class = "standard",
    rarity = 4, mb = 90, damage = 220, hit_param = 178, lockout = 20,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 1,
    instant = strike.instant,
}

return { justcone = chip }
