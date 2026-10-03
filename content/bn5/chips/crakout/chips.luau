--!strict
-- CrakOut, DublCrak and TripCrak: the crakout series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local crack = require("../../objects/crack/crack")

-- CrakOut (BN5 chip 0x05b): action 0x1B, cracks on the panel ahead (objects/crack).
local crakout = define.chip {
    id = "bn5:crakout",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 4,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = crack.action(0),
}

-- DublCrak (BN5 chip 0x05c): action 0x1B, cracks on the panel ahead and the one behind it (objects/crack).
local dublcrak = define.chip {
    id = "bn5:dublcrak",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 7,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = crack.action(1),
}

-- TripCrak (BN5 chip 0x05d): action 0x1B, cracks on the column ahead (objects/crack).
local tripcrak = define.chip {
    id = "bn5:tripcrak",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = crack.action(2),
}

return { crakout = crakout, dublcrak = dublcrak, tripcrak = tripcrak }
