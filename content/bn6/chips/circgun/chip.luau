--!strict
-- CircGun: a gun sight circles the enemy's area until A or 360 ticks, then
-- leaves four shots on the panels it passes (docs/engine/dimming-chip-effects.md
-- §8).

local controller = require("./controller")

return define.chip {
    id = "bn6:circgun",
    codes = { "P", "T", "V" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 2,
    mb = 35,
    damage = 150,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    library = { number = 137, index = 137, sort = 102 },
    slot_in_limit = 3,
    dimming = controller.hook(controller.variant { period = 2, shots = 4, look = 0 }),
}
