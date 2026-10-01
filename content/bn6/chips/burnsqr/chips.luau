--!strict
-- BurnSqr1-3: the user's stand-in moves a 2x2 square over the field until
-- A or 300 ticks, then sets its solid panels on fire
-- (docs/engine/dimming-chip-effects.md §2).

local controller = require("./controller")

local burnsqr1 = define.chip {
    id = "burnsqr1",
    name = "BurnSqr1",
    description = "Press [A]\nto burn a\n4pnl sqr!",
    codes = { "H", "P", "V" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 24,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    library = { number = 108, index = 108, sort = 48 },
    slot_in_limit = 3,
    dimming = controller.hook,
}

local burnsqr2 = define.chip {
    id = "burnsqr2",
    name = "BurnSqr2",
    description = "Press [A]\nto burn a\n4pnl sqr!",
    codes = { "D", "M", "T" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 120,
    hit_param = 30,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    library = { number = 109, index = 109, sort = 49 },
    slot_in_limit = 3,
    dimming = controller.hook,
}

local burnsqr3 = define.chip {
    id = "burnsqr3",
    name = "BurnSqr3",
    description = "Press [A]\nto burn a\n4pnl sqr!",
    codes = { "E", "O", "Z" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 36,
    damage = 140,
    hit_param = 30,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    library = { number = 110, index = 110, sort = 50 },
    slot_in_limit = 3,
    dimming = controller.hook,
}

return { burnsqr1 = burnsqr1, burnsqr2 = burnsqr2, burnsqr3 = burnsqr3 }
