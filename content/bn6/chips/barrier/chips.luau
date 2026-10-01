--!strict
-- Barrier, Barr100, Barr200: cut-in chips that raise a barrier of 10, 100
-- or 200 HP on the user (lib/barriers).

local barrier_chips = require("../../lib/barriers/controller")
local barriers = require("../../lib/barriers/barriers")

return {
    define.chip {
        id = "barrier",
        name = "Barrier",
        description = "Nullifies\n10 HP of\ndamage",
        codes = { "A", "F", "R", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 7, damage = 0, hit_param = 0, lockout = 0,
        flags = { "dimming", "standard_library", "library" },
        extra_flags = { "free_slot_in" },
        library = { number = 180, index = 180, sort = 27 },
        slot_in_limit = 1,
        dimming = barrier_chips.hook(barriers.barrier_10),
    },
    define.chip {
        id = "barr100",
        name = "Barr100",
        description = "Nullifies\n100 HP of\ndamage",
        codes = { "H", "O", "Y" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 30, damage = 0, hit_param = 0, lockout = 0,
        flags = { "dimming", "standard_library", "library" },
        library = { number = 181, index = 181, sort = 28 },
        slot_in_limit = 1,
        dimming = barrier_chips.hook(barriers.barrier_100),
    },
    define.chip {
        id = "barr200",
        name = "Barr200",
        description = "Nullifies\n200 HP of\ndamage",
        codes = { "K", "U", "W" },
        element = "null", family = "null", class = "standard",
        rarity = 3, mb = 52, damage = 0, hit_param = 0, lockout = 0,
        flags = { "dimming", "standard_library", "library" },
        library = { number = 182, index = 182, sort = 29 },
        slot_in_limit = 1,
        dimming = barrier_chips.hook(barriers.barrier_200),
    },
}
