--!strict
-- MoonBld1, MoonBld2 and MoonBld3: the moonbld series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local moonbld = require("./moonbld")

-- MoonBld1 (BN5 chip 0x039): a spin with a blade that sweeps the eight panels around the navi (BN5's MoonBld,
-- chips/moonbld/moonbld: action 0x53, BN6's swing with the shared moon blade; a charged one steps first).
local moonbld1 = define.chip {
    id = "moonbld1",
    codes = { "E", "N", "Z" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 0,
    mb = 23,
    damage = 90,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = moonbld.action,
}

-- MoonBld2 (BN5 chip 0x03a): a spin with a blade that sweeps the eight panels around the navi (BN5's MoonBld,
-- chips/moonbld/moonbld: action 0x53, BN6's swing with the shared moon blade; a charged one steps first).
local moonbld2 = define.chip {
    id = "moonbld2",
    codes = { "G", "O", "V" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 1,
    mb = 36,
    damage = 110,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = moonbld.action,
}

-- MoonBld3 (BN5 chip 0x03b): a spin with a blade that sweeps the eight panels around the navi (BN5's MoonBld,
-- chips/moonbld/moonbld: action 0x53, BN6's swing with the shared moon blade; a charged one steps first).
local moonbld3 = define.chip {
    id = "moonbld3",
    codes = { "C", "L", "T" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 49,
    damage = 130,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = moonbld.action,
}

return { moonbld1 = moonbld1, moonbld2 = moonbld2, moonbld3 = moonbld3 }
