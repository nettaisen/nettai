--!strict
-- Recov10 to Recov300: the navi heals the chip's HP in the tick after it
-- uses it (heal.luau).

local heal = require("@common/recov/heal")

local chips = {
    define.chip {
        id = "bn6:recov10",
        codes = { "A", "D", "L", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 4, damage = 0, hit_param = 30, lockout = 30,
        flags = { "standard_library", "library" },
        library = { number = 156, index = 156, sort = 333 },
        slot_in_limit = 1,
        action = heal.action { hp = 10 },
    },
    define.chip {
        id = "bn6:recov30",
        codes = { "E", "L", "Q", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 12, damage = 0, hit_param = 30, lockout = 30,
        flags = { "standard_library", "library" },
        library = { number = 157, index = 157, sort = 334 },
        slot_in_limit = 1,
        action = heal.action { hp = 30 },
    },
    define.chip {
        id = "bn6:recov50",
        codes = { "C", "M", "P", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 18, damage = 0, hit_param = 40, lockout = 30,
        flags = { "standard_library", "library" },
        library = { number = 158, index = 158, sort = 335 },
        slot_in_limit = 1,
        action = heal.action { hp = 50 },
    },
    define.chip {
        id = "bn6:recov80",
        codes = { "H", "K", "V", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 24, damage = 0, hit_param = 40, lockout = 30,
        flags = { "standard_library", "library" },
        library = { number = 159, index = 159, sort = 336 },
        slot_in_limit = 1,
        action = heal.action { hp = 80 },
    },
    define.chip {
        id = "bn6:recov120",
        codes = { "F", "P", "S" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 32, damage = 0, hit_param = 50, lockout = 30,
        flags = { "standard_library", "library" },
        library = { number = 160, index = 160, sort = 337 },
        slot_in_limit = 1,
        action = heal.action { hp = 120 },
    },
    define.chip {
        id = "bn6:recov150",
        codes = { "J", "M", "T" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 38, damage = 0, hit_param = 50, lockout = 30,
        flags = { "standard_library", "library" },
        library = { number = 161, index = 161, sort = 338 },
        slot_in_limit = 1,
        action = heal.action { hp = 150 },
    },
    define.chip {
        id = "bn6:recov200",
        codes = { "I", "Q", "Z" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 42, damage = 0, hit_param = 50, lockout = 30,
        flags = { "standard_library", "library" },
        library = { number = 162, index = 162, sort = 339 },
        slot_in_limit = 1,
        action = heal.action { hp = 200 },
    },
    define.chip {
        id = "bn6:recov300",
        codes = { "J", "O", "Y" },
        element = "null", family = "null", class = "standard",
        rarity = 3, mb = 48, damage = 0, hit_param = 49, lockout = 30,
        flags = { "standard_library", "library" },
        library = { number = 163, index = 163, sort = 340 },
        slot_in_limit = 1,
        action = heal.action { hp = 300 },
    },
}

-- The heal table's last row, which heals nothing and no chip has
-- (`byte_80EC870`'s tenth entry, after DrkRecov's).
chips.none = heal.action { id = "bn6:recov/none", hp = 0 }

return chips
