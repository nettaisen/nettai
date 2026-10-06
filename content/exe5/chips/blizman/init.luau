--!strict
-- BlizMan, BlizManSP and BlizManDS: the blizman series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")

-- BlizMan (EXE5 chip 0x107): BlizzardMan cuts in and rolls a snowball down his row, growing on the field objects it
-- swallows (./navi).
local blizman = define.chip {
    id = "blizman",
    codes = { "B" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 2,
    mb = 27,
    damage = 100,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- BlizManSP (EXE5 chip 0x108): BlizzardMan cuts in and rolls a snowball down his row, growing on the field objects it
-- swallows (../blizman/navi); its damage by how fast its user deleted BlizzardMan SP (rules/sp_chips: formula 15, its
-- table's row 14).
local blizman_sp = define.chip {
    id = "blizman-sp",
    codes = { "B" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 3,
    mb = 55,
    damage = { formula = "sp_navi", slot = "sp/blizman", by_time = { 180, 175, 170, 165, 160, 155, 150, 145, 140, 135, 130 }, operation_battle = 200 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- BlizManDS (EXE5 chip 0x109): BlizzardMan, dark, cuts in and rolls a snowball down his row, growing on the field
-- objects it swallows (../blizman/navi); its damage by its user's side's statistic 3 (formula 37, its table's row 14).
local blizman_ds = define.chip {
    id = "blizman-ds",
    codes = { "B" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 3,
    mb = 55,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 130, 135, 140, 145, 150, 155, 160, 165, 170, 175, 180 }, operation_battle = 160 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { blizman = blizman, blizman_sp = blizman_sp, blizman_ds = blizman_ds }
