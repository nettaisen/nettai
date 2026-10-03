--!strict
-- Colonel, ColonelSP and ColonelDS: the colonel series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

-- Colonel (BN5 chip 0x0f5): Colonel cuts in and divides the screen with a diagonal slash on the first enemy navi ahead
-- (./navi).
local colonel = define.chip {
    id = "bn5:colonel",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 39,
    damage = 160,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}

-- ColonelSP (BN5 chip 0x0f6): Colonel cuts in and divides the screen with a diagonal slash on the first enemy navi
-- ahead (../colonel/navi); its damage by how fast its user deleted Colonel SP (rules/sp-chips: formula 8, its table's
-- row 7).
local colonel_sp = define.chip {
    id = "bn5:colonel-sp",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = { formula = "sp_navi", slot = "sp/colonel", by_time = { 300, 290, 280, 270, 260, 250, 240, 230, 220, 210, 200 }, per_player_gauges = 230 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}

-- ColonelDS (BN5 chip 0x0f7): Colonel, dark, cuts in and divides the screen with a diagonal slash on the first enemy
-- navi ahead (../colonel/navi); its damage by its user's side's statistic 3 (formula 30, its table's row 7).
local colonel_ds = define.chip {
    id = "bn5:colonel-ds",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 200, 210, 220, 230, 240, 250, 260, 270, 280, 290, 300 }, per_player_gauges = 200 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}

return { colonel = colonel, colonel_sp = colonel_sp, colonel_ds = colonel_ds }
