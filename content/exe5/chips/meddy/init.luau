--!strict
-- Meddy, MeddySP and MeddyDS: the meddy series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")

-- Meddy (EXE5 chip 0x0ec): Meddy cuts in and throws two capsules beside the enemy navi, bursting along their lines
-- (./navi).
local meddy = define.chip {
    id = "meddy",
    codes = { "M" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 36,
    damage = 80,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- MeddySP (EXE5 chip 0x0ed): Meddy cuts in and throws two capsules beside the enemy navi, bursting along their lines
-- (../meddy/navi); its damage by how fast its user deleted Meddy SP (rules/sp_chips: formula 7, its table's row 6).
local meddy_sp = define.chip {
    id = "meddy-sp",
    codes = { "M" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 63,
    damage = { formula = "sp_navi", slot = "sp/meddy", by_time = { 200, 190, 180, 170, 160, 150, 140, 130, 120, 110, 100 }, operation_battle = 120 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- MeddyDS (EXE5 chip 0x0ee): Meddy, dark, cuts in and throws two capsules beside the enemy navi, bursting along their
-- lines (../meddy/navi); its damage by its user's side's statistic 3 (formula 29, its table's row 6).
local meddy_ds = define.chip {
    id = "meddy-ds",
    codes = { "M" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 63,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 100, 110, 120, 130, 140, 150, 160, 170, 180, 190, 200 }, operation_battle = 100 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { meddy = meddy, meddy_sp = meddy_sp, meddy_ds = meddy_ds }
