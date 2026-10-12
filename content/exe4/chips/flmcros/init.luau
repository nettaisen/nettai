--!strict
-- FlmCros1, FlmCros2 and FlmCros3 (EXE4 chips 0x149 to 0x14B; their records at 0x080197EC + n * 0x2C): the Program
-- Advances of three FlmLine1s, FlmLine2s or FlmLine3s of consecutive codes (the recipe table, the pointer list at
-- 0x080212B0). The flame burners' action 0x22 (./flmline/action), its variant 1: a cross of five flames around the
-- panel two ahead (0x080D00F8).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local flmline = require("./flmline/action")
local flmlines = require("./flmline")

local flmcros1: Chip = {
    action = flmline.action("flmcros1/action", flmline.cross),
    codes = { "A", "B", "C", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 300,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 20, code_run = { chip = flmlines.flmline1, count = 3 } } },
    auto_battle = { class = "into_reach", range = "cross_two_ahead" },
}

local flmcros2: Chip = {
    action = flmline.action("flmcros2/action", flmline.cross),
    codes = { "A", "B", "C", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 21, code_run = { chip = flmlines.flmline2, count = 3 } } },
    auto_battle = { class = "into_reach", range = "cross_two_ahead" },
}

local flmcros3: Chip = {
    action = flmline.action("flmcros3/action", flmline.cross),
    codes = { "A", "B", "C", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 500,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 22, code_run = { chip = flmlines.flmline3, count = 3 } } },
    auto_battle = { class = "into_reach", range = "cross_two_ahead" },
}

return { flmcros1 = flmcros1, flmcros2 = flmcros2, flmcros3 = flmcros3 }
