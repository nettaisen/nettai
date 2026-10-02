--!strict
-- Sensor1, Sensor2, Sensor3: cut-in chips that set a turret in front of
-- the user; it scans along its aim and, finding one of the other side's
-- bodies, fires a paralyzing laser down the line (controller.luau).

local controller = require("./controller")

return {
    define.chip {
        id = "sensor1",
        codes = { "J", "O", "W" },
        element = "elec", family = "elec", class = "standard",
        rarity = 0, mb = 32, damage = 100, hit_param = 30, lockout = 0,
        flags = { "dimming", "has_damage", "standard_library", "library" },
        library = { number = 116, index = 116, sort = 404 },
        slot_in_limit = 3,
        dimming = controller.hook { palette = 0, hp = 20 },
    },
    define.chip {
        id = "sensor2",
        codes = { "N", "U", "Y" },
        element = "elec", family = "elec", class = "standard",
        rarity = 1, mb = 35, damage = 130, hit_param = 30, lockout = 0,
        flags = { "dimming", "has_damage", "standard_library", "library" },
        library = { number = 117, index = 117, sort = 405 },
        slot_in_limit = 3,
        dimming = controller.hook { palette = 3, hp = 30 },
    },
    define.chip {
        id = "sensor3",
        codes = { "I", "K", "Q" },
        element = "elec", family = "elec", class = "standard",
        rarity = 2, mb = 38, damage = 160, hit_param = 30, lockout = 0,
        flags = { "dimming", "has_damage", "standard_library", "library" },
        library = { number = 118, index = 118, sort = 406 },
        slot_in_limit = 3,
        dimming = controller.hook { palette = 6, hp = 40 },
    },
}
