--!strict
-- NumbrMnSP (BN5 chip 0x0f3): NumberMan cuts in and throws a die three columns ahead, its face times his damage when it
-- bursts (../numbrman/navi); its damage by how fast its user deleted NumberMan SP (rules/sp-chips: formula 10, its
-- table's row 9).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../numbrman/navi")

return define.chip {
    id = "bn5:numbrmn-sp",
    codes = { "N" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 66,
    damage = { formula = "sp_navi", slot = "sp/numbrman", by_time = { 90, 85, 80, 75, 70, 65, 60, 55, 50, 45, 40 }, per_player_gauges = 70 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
