--!strict
-- CosmoMnDS (BN5 chip 0x10f): CosmoMan, dark, cuts in, takes the field objects around him and calls meteors down on
-- the enemy area (../cosmoman/navi); its damage by its user's side's statistic 3 (formula 39, its table's row 16).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../cosmoman/navi")

return define.chip {
    id = "bn5:cosmomn-ds",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 80,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 80, 85, 90, 95, 100, 105, 110, 115, 120, 125, 130 }, per_player_gauges = 70 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
