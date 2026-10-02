--!strict
-- AirRaid1, AirRaid2, AirRaid3: cut-in chips that set a plane in front of
-- the user; it takes off and bombs the other side's bodies ahead of it
-- (controller.luau). The higher ones stay longer, drop more bombs and
-- take more hits.

local controller = require("./controller")
local plane = require("./plane")

return {
    define.chip {
        id = "airraid1",
        codes = { "G", "K", "R" },
        element = "null", family = "summon", class = "standard",
        rarity = 0, mb = 26, damage = 10, hit_param = 158, lockout = 0,
        flags = { "dimming", "has_damage", "standard_library", "library" },
        library = { number = 142, index = 142, sort = 2 },
        slot_in_limit = 3,
        -- `byte_80D34C0` row 0.
        dimming = controller.hook(plane.plane { lifetime = 150, bombs = 10, palette = 0, hp = 30 }),
    },
    define.chip {
        id = "airraid2",
        codes = { "O", "T", "Y" },
        element = "null", family = "summon", class = "standard",
        rarity = 1, mb = 32, damage = 10, hit_param = 158, lockout = 0,
        flags = { "dimming", "has_damage", "standard_library", "library" },
        library = { number = 143, index = 143, sort = 3 },
        slot_in_limit = 3,
        -- `byte_80D34C0` row 1.
        dimming = controller.hook(plane.plane { lifetime = 190, bombs = 14, palette = 1, hp = 40 }),
    },
    define.chip {
        id = "airraid3",
        codes = { "N", "U", "Z" },
        element = "null", family = "summon", class = "standard",
        rarity = 2, mb = 39, damage = 10, hit_param = 158, lockout = 0,
        flags = { "dimming", "has_damage", "standard_library", "library" },
        library = { number = 144, index = 144, sort = 4 },
        slot_in_limit = 3,
        -- `byte_80D34C0` row 2.
        dimming = controller.hook(plane.plane { lifetime = 230, bombs = 18, palette = 2, hp = 50 }),
    },
}
