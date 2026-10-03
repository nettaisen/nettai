--!strict
-- CloudMnSP (BN5 chip 0x10b): CloudMan cuts in and sets thunderclouds behind the enemies, each striking with a
-- paralyzing bolt (../cloudman/navi); its damage by how fast its user deleted CloudMan SP (rules/sp-chips: formula 16,
-- its table's row 15).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../cloudman/navi")

return define.chip {
    id = "bn5:cloudmn-sp",
    codes = { "C" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 74,
    damage = { formula = "sp_navi", slot = "sp/cloudman", by_time = { 110, 105, 100, 95, 90, 85, 80, 75, 70, 65, 60 }, per_player_gauges = 80 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
