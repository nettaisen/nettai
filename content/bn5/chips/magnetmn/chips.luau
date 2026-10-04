--!strict
-- MagnetMn, MagntMnSP and MagntMnDS: the magnetmn series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

-- MagnetMn (BN5 chip 0x0e9): MagnetMan cuts in and sends two magnets sliding along his row to meet at the first enemy
-- navi, immobilizing (./navi).
local magnetmn = define.chip {
    id = "magnetmn",
    codes = { "M" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 2,
    mb = 25,
    damage = 90,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- MagntMnSP (BN5 chip 0x0ea): MagnetMan cuts in and sends two magnets sliding along his row to meet at the first enemy
-- navi, immobilizing (../magnetmn/navi); its damage by how fast its user deleted MagnetMan SP (rules/sp-chips: formula
-- 6, its table's row 5).
local magntmn_sp = define.chip {
    id = "magntmn-sp",
    codes = { "M" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = { formula = "sp_navi", slot = "sp/magnetmn", by_time = { 200, 190, 180, 170, 160, 150, 140, 130, 120, 110, 100 }, operation_battle = 150 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- MagntMnDS (BN5 chip 0x0eb): MagnetMan, dark, cuts in and sends two magnets sliding along his row to meet at the first
-- enemy navi, immobilizing (../magnetmn/navi); its damage by its user's side's statistic 3 (formula 28, its table's row
-- 5).
local magntmn_ds = define.chip {
    id = "magntmn-ds",
    codes = { "M" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 100, 110, 120, 130, 140, 150, 160, 170, 180, 190, 200 }, operation_battle = 120 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { magnetmn = magnetmn, magntmn_sp = magntmn_sp, magntmn_ds = magntmn_ds }
