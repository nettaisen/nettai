--!strict
-- SerchMnSP (BN5 chip 0x0e7): SearchMan cuts in, his scope sweeps the enemy area until it locks on, and he fires five
-- shots on the panel (../serchman/navi); its damage by how fast its user deleted SearchMan SP (rules/sp-chips:
-- formula 4, its table's row 3).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../serchman/navi")

return define.chip {
    id = "bn5:serchmn-sp",
    codes = { "S" },
    element = "null",
    family = "cursor",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = { formula = "sp_navi", slot = "sp/serchman", by_time = { 75, 70, 65, 60, 55, 50, 45, 40, 35, 30, 25 }, per_player_gauges = 45 },
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
