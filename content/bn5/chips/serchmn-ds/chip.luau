--!strict
-- SerchMnDS (BN5 chip 0x0e8): SearchMan, dark, cuts in, his scope sweeps the enemy area until it locks on, and he fires
-- five shots on the panel (../serchman/navi); its damage by its user's side's statistic 3 (formula 26, its table's
-- row 3).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../serchman/navi")

return define.chip {
    id = "bn5:serchmn-ds",
    codes = { "S" },
    element = "null",
    family = "cursor",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75 }, per_player_gauges = 35 },
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
