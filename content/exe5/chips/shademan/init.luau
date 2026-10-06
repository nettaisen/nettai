--!strict
-- ShadeMan, ShadeMnSP and ShadeMnDS: the shademan series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local sp_chips = require("../rules/sp_chips")

-- ShadeMan (EXE5 chip 0x104): ShadeMan cuts in and crushes the panel ahead and the column beyond (./navi); a command
-- paralyzes or confuses.
local shademan: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 44,
    damage = 120,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- ShadeMnSP (EXE5 chip 0x105): ShadeMan cuts in and crushes the panel ahead and the column beyond (../shademan/navi);
-- its damage by how fast its user deleted ShadeMan SP (rules/sp_chips: formula 14, its table's row 13).
local shademn_sp: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 74,
    sp = { by_time = { 240, 230, 220, 210, 200, 190, 180, 170, 160, 150, 140 }, operation_battle = 190 },
    damage = sp_chips.damage,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- ShadeMnDS (EXE5 chip 0x106): ShadeMan, dark, cuts in and crushes the panel ahead and the column beyond
-- (../shademan/navi); its damage by its user's side's statistic 3 (formula 36, its table's row 13).
local shademn_ds: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 74,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 140, 150, 160, 170, 180, 190, 200, 210, 220, 230, 240 }, operation_battle = 160 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { shademan = shademan, ["shademn-sp"] = shademn_sp, ["shademn-ds"] = shademn_ds }
