--!strict
-- ShadoMan, ShadoMnSP and ShadoMnDS: the shadoman series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")

-- ShadoMan (EXE5 chip 0x0f8): ShadowMan cuts in and sends three of himself to throw shurikens at the enemies (./navi).
local shadoman: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 50,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- ShadoMnSP (EXE5 chip 0x0f9): ShadowMan cuts in and sends three of himself to throw shurikens at the enemies
-- (../shadoman/navi); its damage by how fast its user deleted ShadowMan SP (rules/sp_chips: formula 9, its table's row
-- 8).
local shadomn_sp: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 80,
    damage = { formula = "sp_navi", slot = "sp/shadoman", by_time = { 70, 70, 65, 65, 60, 60, 55, 55, 50, 50, 45 }, operation_battle = 85 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- ShadoMnDS (EXE5 chip 0x0fa): ShadowMan, dark, cuts in and sends three of himself to throw shurikens at the enemies
-- (../shadoman/navi); its damage by its user's side's statistic 3 (formula 31, its table's row 8).
local shadomn_ds: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 80,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 45, 50, 50, 55, 55, 60, 60, 65, 65, 70, 70 }, operation_battle = 70 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { shadoman = shadoman, ["shadomn-sp"] = shadomn_sp, ["shadomn-ds"] = shadomn_ds }
