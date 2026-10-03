--!strict
-- ShadeMnSP (BN5 chip 0x105): ShadeMan cuts in and crushes the panel ahead and the column beyond (../shademan/navi);
-- its damage by how fast its user deleted ShadeMan SP (rules/sp-chips: formula 14, its table's row 13).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../shademan/navi")

return define.chip {
    id = "bn5:shademn-sp",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 74,
    damage = { formula = "sp_navi", slot = "sp/shademan", by_time = { 240, 230, 220, 210, 200, 190, 180, 170, 160, 150, 140 }, per_player_gauges = 190 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
