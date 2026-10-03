--!strict
-- BlizManDS (BN5 chip 0x109): BlizzardMan, dark, cuts in and rolls a snowball down his row, growing on the field
-- objects it swallows (../blizman/navi); its damage by its user's side's statistic 3 (formula 37, its table's row 14).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../blizman/navi")

return define.chip {
    id = "bn5:blizman-ds",
    codes = { "B" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 3,
    mb = 55,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 130, 135, 140, 145, 150, 155, 160, 165, 170, 175, 180 }, per_player_gauges = 160 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
