--!strict
-- Recov10, Recov30, Recov50, Recov80, Recov120, Recov150, Recov200 and Recov300 (EXE4 chips 0x06B to 0x072; their
-- records at 0x080197EC + n * 0x2C): the recoveries' action 0x1D (lib/recov), its variants 0 to 7, the side's mood
-- gaining each one's counter byte.

local recov = require("../lib/recov")

local recov10: Chip = {
    codes = { "A", "L", "N", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 0,
    hit_param = 30,
    lockout = 30,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov10/action", hp = 10, mood = 30 },
}

local recov30: Chip = {
    codes = { "F", "H", "R", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 16,
    damage = 0,
    hit_param = 30,
    lockout = 30,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov30/action", hp = 30, mood = 30 },
}

local recov50: Chip = {
    codes = { "E", "L", "S", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 24,
    damage = 0,
    hit_param = 40,
    lockout = 30,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov50/action", hp = 50, mood = 40 },
}

local recov80: Chip = {
    codes = { "B", "J", "O", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 1,
    mb = 32,
    damage = 0,
    hit_param = 40,
    lockout = 30,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov80/action", hp = 80, mood = 40 },
}

local recov120: Chip = {
    codes = { "F", "J", "Q", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 0,
    hit_param = 50,
    lockout = 30,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov120/action", hp = 120, mood = 50 },
}

local recov150: Chip = {
    codes = { "C", "T", "Z" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 2,
    mb = 48,
    damage = 0,
    hit_param = 50,
    lockout = 30,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov150/action", hp = 150, mood = 50 },
}

local recov200: Chip = {
    codes = { "H", "M", "W" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 2,
    mb = 56,
    damage = 0,
    hit_param = 50,
    lockout = 30,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov200/action", hp = 200, mood = 50 },
}

local recov300: Chip = {
    codes = { "J", "N", "Y" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 3,
    mb = 64,
    damage = 0,
    hit_param = 50,
    lockout = 30,
    flags = { "standard_library", "library" },
    action = recov.action { id = "recov300/action", hp = 300, mood = 50 },
}

return { recov10 = recov10, recov30 = recov30, recov50 = recov50, recov80 = recov80, recov120 = recov120, recov150 = recov150, recov200 = recov200, recov300 = recov300 }
