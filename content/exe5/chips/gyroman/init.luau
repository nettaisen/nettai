--!strict
-- GyroMan, GyroManSP and GyroManDS: the gyroman series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips/chip")
local sp_chips = require("../rules/sp_chips")

-- GyroMan (EXE5 chip 0x0e0): GyroMan cuts in and flies along his row, dropping bombs on the enemy panels (./navi).
local gyroman: Chip = {
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 32,
    damage = 80,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "gyroman/controller",
        summon = navi.summon(0),
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- GyroManSP (EXE5 chip 0x0e1): GyroMan cuts in and flies along his row, dropping bombs on the enemy panels
-- (../gyroman/navi); its damage by how fast its user deleted GyroMan SP (rules/sp_chips: formula 3, its table's row 2).
local gyroman_sp: Chip = {
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 66,
    sp = { by_time = { 140, 135, 130, 125, 120, 115, 110, 105, 100, 95, 90 }, operation_battle = 80 },
    damage = sp_chips.damage,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi_chips.make {
        id = "gyroman-sp/controller",
        summon = navi.summon(2),
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- GyroManDS (EXE5 chip 0x0e2): GyroMan, dark, cuts in and flies along his row, dropping bombs on the enemy panels
-- (../gyroman/navi); its damage by its user's side's statistic 3 (formula 25, its table's row 2).
local gyroman_ds: Chip = {
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 66,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 90, 95, 100, 105, 110, 115, 120, 125, 130, 135, 140 }, operation_battle = 70 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi_chips.make {
        id = "gyroman-ds/controller",
        summon = navi.summon(3),
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { gyroman = gyroman, ["gyroman-sp"] = gyroman_sp, ["gyroman-ds"] = gyroman_ds }
