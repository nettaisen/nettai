--!strict
-- LarkManDS (BN5 chip 0x112): LarkMan, dark, cuts in and sends his swoop round the field (../larkman/navi); its damage
-- by its user's side's statistic 3 (formula 40, its table's row 17).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../larkman/navi")

return define.chip {
    id = "bn5:larkman-ds",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 77,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 170, 180, 190, 200, 210, 220, 230, 240, 250, 260, 270 }, per_player_gauges = 180 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
