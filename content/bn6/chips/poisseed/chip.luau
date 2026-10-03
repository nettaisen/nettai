--!strict
-- PoisSeed: a seed that turns the panels it lands on into poison.

local seed = require("../../lib/bombs/seed")
local throw = require("../../lib/bombs/throw")

return define.chip {
    id = "bn6:poisseed",
    codes = { "H", "N", "P", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 37,
    damage = 10,
    hit_param = 10,
    lockout = 10,
    flags = { "standard_library", "library" },
    library = { number = 69, index = 69, sort = 328 },
    slot_in_limit = 3,
    action = throw.action {
        id = "bn6:poisseed/action",
        held = seed.held(3),
        held_anim = 8,
        thrower = seed.thrower(seed.variant {
            element = 0,
            palette = 3,
            panel = "poison",
            effect = seed.sprout,
            sound = asset.sound("bn6:panel-poison"),
        }),
    },
}
