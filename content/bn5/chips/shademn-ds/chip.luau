--!strict
-- ShadeMnDS (BN5 chip 0x106): ShadeMan, dark, cuts in and crushes the panel ahead and the column beyond
-- (../shademan/navi); its damage by its user's side's statistic 3 (formula 36, its table's row 13).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../shademan/navi")

return define.chip {
    id = "bn5:shademn-ds",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 74,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 140, 150, 160, 170, 180, 190, 200, 210, 220, 230, 240 }, per_player_gauges = 160 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
