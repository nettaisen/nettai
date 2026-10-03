--!strict
-- LifeAur: a cut-in chip that surrounds the user with an aura that stops
-- every hit of less than 200 for a while (lib/barriers).

local barrier_chips = require("../../lib/barriers/controller")
local barriers = require("../../lib/barriers/barriers")

return define.chip {
    id = "bn6:lifeaur",
    codes = { "U" },
    element = "null", family = "null", class = "standard",
    rarity = 4, mb = 70, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 184, index = 184, sort = 253 },
    slot_in_limit = 1,
    dimming = barrier_chips.hook(barriers.life_aura),
}
