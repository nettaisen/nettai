--!strict
-- MoonBld1, MoonBld2 and MoonBld3: the moonbld series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local moonbld = require("@self/moonbld")

-- MoonBld1 (EXE5 chip 0x039): a spin with a blade that sweeps the eight panels around the navi (EXE5's MoonBld,
-- chips/moonbld/moonbld: action 0x53, EXE6's swing with the shared moon blade; a charged one steps first).
local moonbld1: Chip = {
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
    auto_battle = { class = "into_reach", range = "around" },
}

-- MoonBld2 (EXE5 chip 0x03a): a spin with a blade that sweeps the eight panels around the navi (EXE5's MoonBld,
-- chips/moonbld/moonbld: action 0x53, EXE6's swing with the shared moon blade; a charged one steps first).
local moonbld2: Chip = {
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
    auto_battle = { class = "into_reach", range = "around" },
}

-- MoonBld3 (EXE5 chip 0x03b): a spin with a blade that sweeps the eight panels around the navi (EXE5's MoonBld,
-- chips/moonbld/moonbld: action 0x53, EXE6's swing with the shared moon blade; a charged one steps first).
local moonbld3: Chip = {
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
    auto_battle = { class = "into_reach", range = "around" },
}

return { moonbld1 = moonbld1, moonbld2 = moonbld2, moonbld3 = moonbld3 }
