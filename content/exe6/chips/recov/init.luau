--!strict
-- Recov10 to Recov300: the navi heals the chip's HP in the tick after it
-- uses it (heal.luau).

local heal = require("@exelib/recov/heal")

local chips: { [string]: Chip } = {
    recov10 = {
        codes = { "A", "D", "L", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 4, damage = 0, hit_param = 30, lockout = 30,
        flags = { "standard_library", "library" },
        slot_in_limit = 1,
        action = heal.action { hp = 10 },
    },
    recov30 = {
        codes = { "E", "L", "Q", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 12, damage = 0, hit_param = 30, lockout = 30,
        flags = { "standard_library", "library" },
        slot_in_limit = 1,
        action = heal.action { hp = 30 },
    },
    recov50 = {
        codes = { "C", "M", "P", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 18, damage = 0, hit_param = 40, lockout = 30,
        flags = { "standard_library", "library" },
        slot_in_limit = 1,
        action = heal.action { hp = 50 },
    },
    recov80 = {
        codes = { "H", "K", "V", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 24, damage = 0, hit_param = 40, lockout = 30,
        flags = { "standard_library", "library" },
        slot_in_limit = 1,
        action = heal.action { hp = 80 },
    },
    recov120 = {
        codes = { "F", "P", "S" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 32, damage = 0, hit_param = 50, lockout = 30,
        flags = { "standard_library", "library" },
        slot_in_limit = 1,
        action = heal.action { hp = 120 },
    },
    recov150 = {
        codes = { "J", "M", "T" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 38, damage = 0, hit_param = 50, lockout = 30,
        flags = { "standard_library", "library" },
        slot_in_limit = 1,
        action = heal.action { hp = 150 },
    },
    recov200 = {
        codes = { "I", "Q", "Z" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 42, damage = 0, hit_param = 50, lockout = 30,
        flags = { "standard_library", "library" },
        slot_in_limit = 1,
        action = heal.action { hp = 200 },
    },
    recov300 = {
        codes = { "J", "O", "Y" },
        element = "null", family = "null", class = "standard",
        rarity = 3, mb = 48, damage = 0, hit_param = 49, lockout = 30,
        flags = { "standard_library", "library" },
        slot_in_limit = 1,
        action = heal.action { hp = 300 },
    },
}

return chips
