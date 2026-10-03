--!strict
-- GyroManDS (BN5 chip 0x0e2): GyroMan, dark, cuts in and flies along his row, dropping bombs on the enemy panels
-- (../gyroman/navi); its damage by its user's side's statistic 3 (formula 25, its table's row 2).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../gyroman/navi")

return define.chip {
    id = "bn5:gyroman-ds",
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 66,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 90, 95, 100, 105, 110, 115, 120, 125, 130, 135, 140 }, per_player_gauges = 70 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
