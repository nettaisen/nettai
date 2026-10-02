--!strict
-- GrasSeed: a seed that turns the panels it lands on into grass.

local seed = require("../../lib/bombs/seed")
local throw = require("../../lib/bombs/throw")

return define.chip {
    id = "grasseed",
    codes = { "A", "F", "S", "*" },
    element = "wood", family = "wood", class = "standard",
    rarity = 1, mb = 19, damage = 10, hit_param = 10, lockout = 10,
    flags = { "standard_library", "library" },
    library = { number = 67, index = 67, sort = 222 },
    slot_in_limit = 3,
    action = throw.action {
        held = seed.held(2),
        held_anim = 8,
        thrower = seed.thrower(seed.variant {
            element = 4,
            palette = 2,
            panel = "grass",
            effect = seed.green_sprout,
            sound = asset.sound("grass"),
        }),
    },
}
