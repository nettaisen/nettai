--!strict
-- IceSeed: a seed that turns the panels it lands on into ice.

local seed = require("../lib/bombs/seed")
local throw = require("../lib/bombs/throw")

local chip: Chip = {
    codes = { "A", "L", "R", "*" },
    element = "aqua", family = "aqua", class = "standard",
    rarity = 1, mb = 31, damage = 10, hit_param = 10, lockout = 10,
    flags = { "standard_library", "library" },
    slot_in_limit = 3,
    action = throw.action {
        held = seed.held(1),
        held_anim = 8,
        thrower = seed.thrower(seed.variant {
            element = 2,
            palette = 1,
            panel = "ice",
            effect = seed.sprout,
            sound = asset.sound("freeze"),
        }),
    },
}

return { iceseed = chip }
