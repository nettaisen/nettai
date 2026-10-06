--!strict
-- LarkMan, LarkManSP and LarkManDS: the larkman series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local sp_chips = require("../rules/sp_chips")

-- LarkMan (EXE5 chip 0x110): LarkMan cuts in and sends his swoop round the field (./navi).
local larkman: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 45,
    damage = 140,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- LarkManSP (EXE5 chip 0x111): LarkMan cuts in and sends his swoop round the field (../larkman/navi); its damage by how
-- fast its user deleted LarkMan SP (rules/sp_chips: formula 18, its table's row 17).
local larkman_sp: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 77,
    sp = { by_time = { 270, 260, 250, 240, 230, 220, 210, 200, 190, 180, 170 }, operation_battle = 220 },
    damage = sp_chips.damage,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- LarkManDS (EXE5 chip 0x112): LarkMan, dark, cuts in and sends his swoop round the field (../larkman/navi); its damage
-- by its user's side's statistic 3 (formula 40, its table's row 17).
local larkman_ds: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 77,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 170, 180, 190, 200, 210, 220, 230, 240, 250, 260, 270 }, operation_battle = 180 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { larkman = larkman, ["larkman-sp"] = larkman_sp, ["larkman-ds"] = larkman_ds }
