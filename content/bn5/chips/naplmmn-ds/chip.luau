--!strict
-- NaplmMnDS (BN5 chip 0x0e5): NapalmMan, dark, cuts in and lobs a napalm bomb four panels ahead, its fire spreading
-- around an empty panel (../napalmmn/navi); its damage by its user's side's statistic 3 (formula 27, its table's row
-- 4).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../napalmmn/navi")

return define.chip {
    id = "bn5:naplmmn-ds",
    codes = { "N" },
    element = "fire",
    family = "fire",
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
