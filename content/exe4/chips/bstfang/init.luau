--!strict
-- BstFang1, BstFang2 and BstFang3 (EXE4 chips 0x14F to 0x151; their records at 0x080197EC + n * 0x2C): the Program
-- Advances of three TwnFng1s, TwnFng2s or TwnFng3s of consecutive codes (the recipe table, the pointer list at
-- 0x080212B0). The twin fangs' action 0x34 (./twnfng/action), its variant 1: eight pairs of fangs, 10 ticks apart,
-- their hits weak (the parameters' second byte, 1).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local twnfng = require("./twnfng/action")
local twnfngs = require("./twnfng")

local bstfang1: Chip = {
    action = twnfng.action("bstfang1/action", 8, true),
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 50,
    hit_param = 5,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 26, code_run = { chip = twnfngs.twnfng1, count = 3 } } },
}

local bstfang2: Chip = {
    action = twnfng.action("bstfang2/action", 8, true),
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 60,
    hit_param = 5,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 27, code_run = { chip = twnfngs.twnfng2, count = 3 } } },
}

local bstfang3: Chip = {
    action = twnfng.action("bstfang3/action", 8, true),
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 70,
    hit_param = 5,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 28, code_run = { chip = twnfngs.twnfng3, count = 3 } } },
}

return { bstfang1 = bstfang1, bstfang2 = bstfang2, bstfang3 = bstfang3 }
