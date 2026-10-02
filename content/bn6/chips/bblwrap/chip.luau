--!strict
-- BblWrap: a cut-in chip that wraps the user in a bubble that takes one
-- hit and grows back (lib/barriers).

local barrier_chips = require("../../lib/barriers/controller")
local barriers = require("../../lib/barriers/barriers")

return define.chip {
    id = "bblwrap",
    codes = { "I", "Q", "Z" },
    element = "null", family = "aqua", class = "standard",
    rarity = 2, mb = 58, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 183, index = 183, sort = 35 },
    slot_in_limit = 1,
    dimming = barrier_chips.hook(barriers.bubble),
}
