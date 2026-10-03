--!strict
-- GridManDS (BN5 chip 0x115): GridMan, dark, cuts in, dashes across the field and strikes the whole enemy area
-- (../gridman/navi); its damage by its user's side's statistic 3 (formula 41, its table's row 18).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../gridman/navi")

return define.chip {
    id = "bn5:gridman-ds",
    codes = { "F" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 150, 160, 170, 180, 190, 200, 210, 220, 230, 240, 250 }, per_player_gauges = 180 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
