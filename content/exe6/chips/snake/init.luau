--!strict
-- Snake: a cut-in chip whose snakes leap out of every hole on the user's
-- side at the nearest enemy (docs/engine/dimming-chip-effects.md §13).

local controller = require("@self/controller")

local chip: Chip = {
    codes = { "H", "M", "L" },
    element = "null", family = "null", class = "standard",
    rarity = 3, mb = 34, damage = 30, hit_param = 133, lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = controller.hook,
}

return { snake = chip }
