--!strict
-- CrakOut, DublCrak and TripCrak (EXE4 chips 0x05B to 0x05D; their records at 0x080197EC + n * 0x2C): the cracks'
-- action 0x21 (lib/crack), its variants 0 to 2: a crack on the panel ahead; on it and the one past it; on the column
-- ahead.

local crack = require("../lib/crack")

local crakout: Chip = {
    codes = { "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 4,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    action = crack.action { id = "crakout/action", panels = crack.ahead },
}

local dublcrak: Chip = {
    codes = { "B", "L", "R", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 7,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    action = crack.action { id = "dublcrak/action", panels = crack.ahead_two },
}

local tripcrak: Chip = {
    codes = { "B", "L", "R", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    action = crack.action { id = "tripcrak/action", panels = crack.column },
}

return { crakout = crakout, dublcrak = dublcrak, tripcrak = tripcrak }
