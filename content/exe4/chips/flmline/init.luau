--!strict
-- FlmLine1, FlmLine2 and FlmLine3 (EXE4 chips 0x00C to 0x00E; their records at 0x080197EC + n * 0x2C): the flame
-- burners' action 0x22 (./action), its variant 0: flames on the column two panels ahead, its level panel, then up, then
-- down (0x080D00D8).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local flmline = require("@self/action")

local flmline1: Chip = {
    action = flmline.action("flmline1/action", flmline.column),
    codes = { "F", "G", "H", "*" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 18,
    damage = 70,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    auto_battle = { class = "into_reach", range = "column_two_ahead" },
}

local flmline2: Chip = {
    action = flmline.action("flmline2/action", flmline.column),
    codes = { "D", "E", "F" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 26,
    damage = 120,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    auto_battle = { class = "into_reach", range = "column_two_ahead" },
}

local flmline3: Chip = {
    action = flmline.action("flmline3/action", flmline.column),
    codes = { "J", "K", "L" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 54,
    damage = 170,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    auto_battle = { class = "into_reach", range = "column_two_ahead" },
}

return { flmline1 = flmline1, flmline2 = flmline2, flmline3 = flmline3 }
