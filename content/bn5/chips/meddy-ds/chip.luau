--!strict
-- MeddyDS (BN5 chip 0x0ee): Meddy, dark, cuts in and throws two capsules beside the enemy navi, bursting along their
-- lines (../meddy/navi); its damage by its user's side's statistic 3 (formula 29, its table's row 6).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../meddy/navi")

return define.chip {
    id = "bn5:meddy-ds",
    codes = { "M" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 63,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 100, 110, 120, 130, 140, 150, 160, 170, 180, 190, 200 }, per_player_gauges = 100 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
