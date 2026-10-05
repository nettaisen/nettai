--!strict
-- Magnum: the user's stand-in moves a sight over the enemy's area until A or
-- 300 ticks, then shoots its three panels, breaking them
-- (docs/engine/dimming-chip-effects.md §11).

local controller = require("@self/controller")

return define.chip {
    id = "magnum",
    codes = { "F", "L", "W" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 2,
    mb = 31,
    damage = 130,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    library = { number = 136, index = 136, sort = 309 },
    slot_in_limit = 3,
    dimming = controller.hook,
}
