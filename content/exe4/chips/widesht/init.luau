--!strict
-- WideSht1, WideSht2 and WideSht3 (EXE4 chips 0x009 to 0x00B; their records at 0x080197EC + n * 0x2C): the wide shots'
-- action 0x31 (./action), its variant 0: one wave. Each chip's wave is its record's parameter: 3, 4, 5, each faster
-- (0x080D49E8), ending on its first hit.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local widesht = require("@self/action")

local widesht1: Chip = {
    -- Its wave 3.
    action = widesht.action("widesht1/action", widesht.wave(0x6_0000, false), 1),
    codes = { "C", "D", "E", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 60,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    auto_battle = { class = "spot_test", range = "ahead_within_a_row" },
}

local widesht2: Chip = {
    -- Its wave 4.
    action = widesht.action("widesht2/action", widesht.wave(0x6_8000, false), 1),
    codes = { "L", "M", "N" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 80,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    auto_battle = { class = "spot_test", range = "ahead_within_a_row" },
}

local widesht3: Chip = {
    -- Its wave 5.
    action = widesht.action("widesht3/action", widesht.wave(0x7_0000, false), 1),
    codes = { "S", "T", "U" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    auto_battle = { class = "spot_test", range = "ahead_within_a_row" },
}

return { widesht1 = widesht1, widesht2 = widesht2, widesht3 = widesht3 }
