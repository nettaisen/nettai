--!strict
-- MagnetMn, MagntMnSP and MagntMnDS: the magnetmn series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips/chip")
local sp_chips = require("../rules/sp_chips")

-- MagnetMn (EXE5 chip 0x0e9): MagnetMan cuts in and sends two magnets sliding along his row to meet at the first enemy
-- navi, immobilizing (./navi).
local magnetmn: Chip = {
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
    navi = navi_chips.make {
        id = "magnetmn/controller",
        summon = navi.summon,
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- MagntMnSP (EXE5 chip 0x0ea): MagnetMan cuts in and sends two magnets sliding along his row to meet at the first enemy
-- navi, immobilizing (../magnetmn/navi); its damage by how fast its user deleted MagnetMan SP (rules/sp_chips: formula
-- 6, its table's row 5).
local magntmn_sp: Chip = {
    codes = { "M" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 75,
    sp = { by_time = { 200, 190, 180, 170, 160, 150, 140, 130, 120, 110, 100 }, operation_battle = 150 },
    damage = sp_chips.damage,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "magntmn-sp/controller",
        summon = navi.summon_sp,
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- MagntMnDS (EXE5 chip 0x0eb): MagnetMan, dark, cuts in and sends two magnets sliding along his row to meet at the first
-- enemy navi, immobilizing (../magnetmn/navi); its damage by its user's side's statistic 3 (formula 28, its table's row
-- 5).
local magntmn_ds: Chip = {
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
    navi = navi_chips.make {
        id = "magntmn-ds/controller",
        summon = navi.summon_dark,
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { magnetmn = magnetmn, ["magntmn-sp"] = magntmn_sp, ["magntmn-ds"] = magntmn_ds }
