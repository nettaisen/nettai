--!strict
-- TmhwkMnDS (BN5 chip 0x0fd): TomahawkMan, dark, cuts in and brings his tomahawk down on the two columns ahead
-- (../tmhwkman/navi); its damage by its user's side's statistic 3 (formula 33, its table's row 10).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../tmhwkman/navi")

return define.chip {
    id = "bn5:tmhwkmn-ds",
    codes = { "T" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 3,
    mb = 81,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 180, 190, 200, 210, 220, 230, 240, 250, 260, 270, 280 }, per_player_gauges = 200 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
