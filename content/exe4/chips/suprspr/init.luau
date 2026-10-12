--!strict
-- SuprSpr1, SuprSpr2 and SuprSpr3 (EXE4 chips 0x146 to 0x148; their records at 0x080197EC + n * 0x2C): the Program
-- Advances of three WideSht1s, WideSht2s or WideSht3s of consecutive codes (the recipe table, the pointer list at
-- 0x080212B0). The wide shots' action 0x31 (./widesht/action), its variant 1: three waves. Each chip's wave is its
-- record's parameter: 6, 7, 8 (0x080D49E8: WideSht's three speeds), going through what they hit.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local widesht = require("./widesht/action")
local wideshts = require("./widesht")

local suprspr1: Chip = {
    -- Its wave 6.
    action = widesht.action("suprspr1/action", widesht.wave(0x6_0000, true), 3),
    codes = { "A", "B", "C", "*" },
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 60,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 17, code_run = { chip = wideshts.widesht1, count = 3 } } },
    auto_battle = { class = "spot_test", range = "ahead_within_a_row" },
}

local suprspr2: Chip = {
    -- Its wave 7.
    action = widesht.action("suprspr2/action", widesht.wave(0x6_8000, true), 3),
    codes = { "A", "B", "C", "*" },
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 80,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 18, code_run = { chip = wideshts.widesht2, count = 3 } } },
    auto_battle = { class = "spot_test", range = "ahead_within_a_row" },
}

local suprspr3: Chip = {
    -- Its wave 8.
    action = widesht.action("suprspr3/action", widesht.wave(0x7_0000, true), 3),
    codes = { "A", "B", "C", "*" },
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 19, code_run = { chip = wideshts.widesht3, count = 3 } } },
    auto_battle = { class = "spot_test", range = "ahead_within_a_row" },
}

return { suprspr1 = suprspr1, suprspr2 = suprspr2, suprspr3 = suprspr3 }
