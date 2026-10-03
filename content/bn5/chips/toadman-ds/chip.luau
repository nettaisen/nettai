--!strict
-- ToadManDS (BN5 chip 0x103): ToadMan, dark, cuts in and plays a chain of paralyzing notes toward the nearest enemy
-- ahead (../toadman/navi); its damage by its user's side's statistic 3 (formula 35, its table's row 12).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../toadman/navi")

return define.chip {
    id = "bn5:toadman-ds",
    codes = { "T" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 69,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 120, 130, 140, 150, 160, 170, 180, 190, 200, 210, 220 }, per_player_gauges = 120 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
