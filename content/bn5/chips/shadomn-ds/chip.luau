--!strict
-- ShadoMnDS (BN5 chip 0x0fa): ShadowMan, dark, cuts in and sends three of himself to throw shurikens at the enemies
-- (../shadoman/navi); its damage by its user's side's statistic 3 (formula 31, its table's row 8).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../shadoman/navi")

return define.chip {
    id = "bn5:shadomn-ds",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 80,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 45, 50, 50, 55, 55, 60, 60, 65, 65, 70, 70 }, per_player_gauges = 70 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
