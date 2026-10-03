--!strict
-- CloudMnDS (BN5 chip 0x10c): CloudMan, dark, cuts in and sets thunderclouds behind the enemies, each striking with a
-- paralyzing bolt (../cloudman/navi); its damage by its user's side's statistic 3 (formula 38, its table's row 15).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../cloudman/navi")

return define.chip {
    id = "bn5:cloudmn-ds",
    codes = { "C" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 74,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 60, 65, 70, 75, 80, 85, 90, 95, 100, 105, 110 }, per_player_gauges = 60 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
