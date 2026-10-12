--!strict
-- TwnFng1, TwnFng2 and TwnFng3 (EXE4 chips 0x045 to 0x047; their records at 0x080197EC + n * 0x2C): the twin fangs'
-- action 0x34 (./action), its variant 0: one pair of fangs, their hits strong.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local twnfng = require("@self/action")

local twnfng1: Chip = {
    action = twnfng.action("twnfng1/action", 1, false),
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 13,
    damage = 70,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local twnfng2: Chip = {
    action = twnfng.action("twnfng2/action", 1, false),
    codes = { "O", "P", "Q" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 26,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local twnfng3: Chip = {
    action = twnfng.action("twnfng3/action", 1, false),
    codes = { "F", "G", "H" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 39,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { twnfng1 = twnfng1, twnfng2 = twnfng2, twnfng3 = twnfng3 }
