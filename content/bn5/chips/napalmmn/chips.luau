--!strict
-- NapalmMn, NaplmMnSP and NaplmMnDS: the napalmmn series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

-- NapalmMn (BN5 chip 0x0e3): NapalmMan cuts in and lobs a napalm bomb four panels ahead, its fire spreading around an
-- empty panel (./navi).
local napalmmn = define.chip {
    id = "napalmmn",
    codes = { "N" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 2,
    mb = 38,
    damage = 120,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- NaplmMnSP (BN5 chip 0x0e4): NapalmMan cuts in and lobs a napalm bomb four panels ahead, its fire spreading around an
-- empty panel (../napalmmn/navi); its damage by how fast its user deleted NapalmMan SP (rules/sp-chips: formula 5, its
-- table's row 4).
local naplmmn_sp = define.chip {
    id = "naplmmn-sp",
    codes = { "N" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = { formula = "sp_navi", slot = "sp/napalmmn", by_time = { 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150 }, operation_battle = 200 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- NaplmMnDS (BN5 chip 0x0e5): NapalmMan, dark, cuts in and lobs a napalm bomb four panels ahead, its fire spreading
-- around an empty panel (../napalmmn/navi); its damage by its user's side's statistic 3 (formula 27, its table's row
-- 4).
local naplmmn_ds = define.chip {
    id = "naplmmn-ds",
    codes = { "N" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 150, 160, 170, 180, 190, 200, 210, 220, 230, 240, 250 }, operation_battle = 180 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { napalmmn = napalmmn, naplmmn_sp = naplmmn_sp, naplmmn_ds = naplmmn_ds }
