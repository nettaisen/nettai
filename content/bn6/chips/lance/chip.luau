--!strict
-- Lance: three lances (lance.luau) thrust out of the far column, one per
-- row.

local lance = require("./lance")

return define.chip {
    id = "lance",
    name = "Lance",
    description = "Lance\nthrough\nback line",
    codes = { "A", "R", "W", "*" },
    element = "wood", family = "wood", class = "standard",
    rarity = 3, mb = 42, damage = 150, hit_param = 30, lockout = 10,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 122, index = 122, sort = 252 },
    slot_in_limit = 3,
    instant = lance.instant,
}
