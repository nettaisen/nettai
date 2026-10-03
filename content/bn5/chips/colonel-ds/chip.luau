--!strict
-- ColonelDS (BN5 chip 0x0f7): Colonel, dark, cuts in and divides the screen with a diagonal slash on the first enemy
-- navi ahead (../colonel/navi); its damage by its user's side's statistic 3 (formula 30, its table's row 7).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../colonel/navi")

return define.chip {
    id = "bn5:colonel-ds",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 200, 210, 220, 230, 240, 250, 260, 270, 280, 290, 300 }, per_player_gauges = 200 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
