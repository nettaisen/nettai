--!strict
-- CrakOut, DublCrak and TripCrak: the crakout series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local crack = require("@self/crack")

-- CrakOut (EXE5 chip 0x05b): action 0x1B, cracks on the panel ahead (chips/crakout/crack).
local crakout: Chip = {
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
    -- (0x080C7818's row 0: the panel ahead.)
    action = crack.action({ { 0, 0 } }),
    auto_battle = { class = "at_once" },
}

-- DublCrak (EXE5 chip 0x05c): action 0x1B, cracks on the panel ahead and the one behind it (chips/crakout/crack).
local dublcrak: Chip = {
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
    -- (Row 1: the panel ahead and the one behind it.)
    action = crack.action({ { 0, 0 }, { 1, 0 } }),
    auto_battle = { class = "at_once" },
}

-- TripCrak (EXE5 chip 0x05d): action 0x1B, cracks on the column ahead (chips/crakout/crack).
local tripcrak: Chip = {
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
    -- (Row 2: the column ahead.)
    action = crack.action({ { 0, 0 }, { 0, -1 }, { 0, 1 } }),
    auto_battle = { class = "at_once" },
}

return { crakout = crakout, dublcrak = dublcrak, tripcrak = tripcrak }
