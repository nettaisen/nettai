--!strict
-- Barrier, Barr100, Barr200: cut-in chips that raise a barrier of 10, 100
-- or 200 HP on the user (lib/barriers).

local barrier_chips = require("../lib/barriers/controller")
local barriers = require("../lib/barriers")

local chips: { [string]: Chip } = {
    barrier = {
        codes = { "A", "F", "R", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 7, damage = 0, hit_param = 0, lockout = 0,
        flags = { "dimming", "standard_library", "library" },
        extra_flags = { "free_slot_in" },
        slot_in_limit = 1,
        dimming = barrier_chips.hook(barriers.barrier_10),
    },
    barr100 = {
        codes = { "H", "O", "Y" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 30, damage = 0, hit_param = 0, lockout = 0,
        flags = { "dimming", "standard_library", "library" },
        slot_in_limit = 1,
        dimming = barrier_chips.hook(barriers.barrier_100),
    },
    barr200 = {
        codes = { "K", "U", "W" },
        element = "null", family = "null", class = "standard",
        rarity = 3, mb = 52, damage = 0, hit_param = 0, lockout = 0,
        flags = { "dimming", "standard_library", "library" },
        slot_in_limit = 1,
        dimming = barrier_chips.hook(barriers.barrier_200),
    },
}

return chips
