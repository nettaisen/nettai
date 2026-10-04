--!strict
-- CloudMan, CloudMnSP and CloudMnDS: the cloudman series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("@self/navi")

-- CloudMan (BN5 chip 0x10a): CloudMan cuts in and sets thunderclouds behind the enemies, each striking with a
-- paralyzing bolt (./navi).
local cloudman = define.chip {
    id = "cloudman",
    codes = { "C" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 2,
    mb = 60,
    damage = 50,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- CloudMnSP (BN5 chip 0x10b): CloudMan cuts in and sets thunderclouds behind the enemies, each striking with a
-- paralyzing bolt (../cloudman/navi); its damage by how fast its user deleted CloudMan SP (rules/sp-chips: formula 16,
-- its table's row 15).
local cloudmn_sp = define.chip {
    id = "cloudmn-sp",
    codes = { "C" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 74,
    damage = { formula = "sp_navi", slot = "sp/cloudman", by_time = { 110, 105, 100, 95, 90, 85, 80, 75, 70, 65, 60 }, operation_battle = 80 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- CloudMnDS (BN5 chip 0x10c): CloudMan, dark, cuts in and sets thunderclouds behind the enemies, each striking with a
-- paralyzing bolt (../cloudman/navi); its damage by its user's side's statistic 3 (formula 38, its table's row 15).
local cloudmn_ds = define.chip {
    id = "cloudmn-ds",
    codes = { "C" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 74,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 60, 65, 70, 75, 80, 85, 90, 95, 100, 105, 110 }, operation_battle = 60 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { cloudman = cloudman, cloudmn_sp = cloudmn_sp, cloudmn_ds = cloudmn_ds }
