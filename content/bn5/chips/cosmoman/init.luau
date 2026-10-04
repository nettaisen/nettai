--!strict
-- CosmoMan, CosmoMnSP and CosmoMnDS: the cosmoman series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("@self/navi")

-- CosmoMan (BN5 chip 0x10d): CosmoMan cuts in, takes the field objects around him and calls meteors down on the
-- enemy area (./navi).
local cosmoman = define.chip {
    id = "cosmoman",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 50,
    damage = 70,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- CosmoMnSP (BN5 chip 0x10e): CosmoMan cuts in, takes the field objects around him and calls meteors down on the
-- enemy area (../cosmoman/navi); its damage by how fast its user deleted CosmoMan SP (rules/sp-chips: formula 17,
-- its table's row 16).
local cosmomn_sp = define.chip {
    id = "cosmomn-sp",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 80,
    damage = { formula = "sp_navi", slot = "sp/cosmoman", by_time = { 130, 125, 120, 115, 110, 105, 100, 95, 90, 85, 80 }, operation_battle = 80 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- CosmoMnDS (BN5 chip 0x10f): CosmoMan, dark, cuts in, takes the field objects around him and calls meteors down on
-- the enemy area (../cosmoman/navi); its damage by its user's side's statistic 3 (formula 39, its table's row 16).
local cosmomn_ds = define.chip {
    id = "cosmomn-ds",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 80,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 80, 85, 90, 95, 100, 105, 110, 115, 120, 125, 130 }, operation_battle = 70 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { cosmoman = cosmoman, cosmomn_sp = cosmomn_sp, cosmomn_ds = cosmomn_ds }
