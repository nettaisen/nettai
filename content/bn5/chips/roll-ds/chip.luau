--!strict
-- RollDS (BN5 chip 0x0df): Roll (../roll/navi) in her dark palette; its damage by its user's side's statistic 3
-- (formula 23, its table's row 0).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../roll/navi")

return define.chip {
    id = "bn5:roll-ds",
    codes = { "R" },
    element = "null",
    family = "recovery",
    class = "mega",
    rarity = 3,
    mb = 60,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80 }, per_player_gauges = 40 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "dark",
    -- (AntiRecv springs on her heal; she brings her user back herself.)
    traits = { "heals", "navi_returns_user" },
    navi = navi.summon { rounds = 1, palette = 1, level = 4, ribbon_palette = 1 },
}
