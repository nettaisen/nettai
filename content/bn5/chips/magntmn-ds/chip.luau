--!strict
-- MagntMnDS (BN5 chip 0x0eb): MagnetMan, dark, cuts in and sends two magnets sliding along his row to meet at the first
-- enemy navi, immobilizing (../magnetmn/navi); its damage by its user's side's statistic 3 (formula 28, its table's row
-- 5).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../magnetmn/navi")

return define.chip {
    id = "bn5:magntmn-ds",
    codes = { "M" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 100, 110, 120, 130, 140, 150, 160, 170, 180, 190, 200 }, per_player_gauges = 120 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
