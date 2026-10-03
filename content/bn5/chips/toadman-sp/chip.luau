--!strict
-- ToadManSP (BN5 chip 0x102): ToadMan cuts in and plays a chain of paralyzing notes toward the nearest enemy ahead
-- (../toadman/navi); its damage by how fast its user deleted ToadMan SP (rules/sp-chips: formula 13, its table's row
-- 12).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../toadman/navi")

return define.chip {
    id = "bn5:toadman-sp",
    codes = { "T" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 69,
    damage = { formula = "sp_navi", slot = "sp/toadman", by_time = { 220, 210, 200, 190, 180, 170, 160, 150, 140, 130, 120 }, per_player_gauges = 140 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
