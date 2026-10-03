--!strict
-- ShadoMnSP (BN5 chip 0x0f9): ShadowMan cuts in and sends three of himself to throw shurikens at the enemies
-- (../shadoman/navi); its damage by how fast its user deleted ShadowMan SP (rules/sp-chips: formula 9, its table's row
-- 8).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../shadoman/navi")

return define.chip {
    id = "bn5:shadomn-sp",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 80,
    damage = { formula = "sp_navi", slot = "sp/shadoman", by_time = { 70, 70, 65, 65, 60, 60, 55, 55, 50, 50, 45 }, per_player_gauges = 85 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
