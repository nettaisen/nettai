--!strict
-- MagntMnSP (BN5 chip 0x0ea): MagnetMan cuts in and sends two magnets sliding along his row to meet at the first enemy
-- navi, immobilizing (../magnetmn/navi); its damage by how fast its user deleted MagnetMan SP (rules/sp-chips: formula
-- 6, its table's row 5).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../magnetmn/navi")

return define.chip {
    id = "bn5:magntmn-sp",
    codes = { "M" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = { formula = "sp_navi", slot = "sp/magnetmn", by_time = { 200, 190, 180, 170, 160, 150, 140, 130, 120, 110, 100 }, per_player_gauges = 150 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
