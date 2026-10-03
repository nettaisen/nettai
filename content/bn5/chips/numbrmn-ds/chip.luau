--!strict
-- NumbrMnDS (BN5 chip 0x0f4): NumberMan, dark, cuts in and throws a die three columns ahead, its face times his damage when it bursts (../numbrman/navi); its damage by its user's side's
-- statistic 3 (formula 32, its table's row 9).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../numbrman/navi")

return define.chip {
    id = "bn5:numbrmn-ds",
    codes = { "N" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 66,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90 }, per_player_gauges = 50 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
