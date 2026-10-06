--!strict
-- Meteors: thirty meteors (./falling_meteor) fall on the enemy's
-- area, one every 10 ticks, cycling through its panels from the far column
-- (./controller; docs/engine/dimming-chip-effects.md §6).

local controller = require("@self/controller")

local chip: Chip = {
    codes = { "R" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 4,
    mb = 73,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook,
}

return { meteors = chip }
