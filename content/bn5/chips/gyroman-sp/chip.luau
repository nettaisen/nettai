--!strict
-- GyroManSP (BN5 chip 0x0e1): GyroMan cuts in and flies along his row, dropping bombs on the enemy panels
-- (../gyroman/navi); its damage by how fast its user deleted GyroMan SP (rules/sp-chips: formula 3, its table's row 2).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../gyroman/navi")

return define.chip {
    id = "bn5:gyroman-sp",
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 66,
    damage = { formula = "sp_navi", slot = "sp/gyroman", by_time = { 140, 135, 130, 125, 120, 115, 110, 105, 100, 95, 90 }, per_player_gauges = 80 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
