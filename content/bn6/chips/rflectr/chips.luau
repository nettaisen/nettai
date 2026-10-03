--!strict
-- Rflectr1, Rflectr2 and Rflectr3: the navi holds up a shield for 60
-- ticks; the first hit it blocks from the front sends a wave back along
-- the row with the chip's damage (guard.luau). Each holds up its own look.

local guard = require("./guard")
local shield = require("./shield")

return {
    define.chip {
        id = "bn6:rflectr1",
        codes = { "A", "C", "P", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 7, damage = 60, hit_param = 20, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 91, index = 91, sort = 341 },
        slot_in_limit = 3,
        action = guard.action { ticks = 60, look = shield.looks.rflectr1, counter = "wave" },
    },
    define.chip {
        id = "bn6:rflectr2",
        codes = { "B", "G", "Y", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 16, damage = 120, hit_param = 20, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 92, index = 92, sort = 342 },
        slot_in_limit = 3,
        action = guard.action { ticks = 60, look = shield.looks.rflectr2, counter = "wave" },
    },
    define.chip {
        id = "bn6:rflectr3",
        codes = { "E", "F", "O", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 25, damage = 200, hit_param = 20, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 93, index = 93, sort = 343 },
        slot_in_limit = 3,
        action = guard.action { ticks = 60, look = shield.looks.rflectr3, counter = "wave" },
    },
}
