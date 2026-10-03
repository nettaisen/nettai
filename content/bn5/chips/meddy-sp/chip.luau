--!strict
-- MeddySP (BN5 chip 0x0ed): Meddy cuts in and throws two capsules beside the enemy navi, bursting along their lines
-- (../meddy/navi); its damage by how fast its user deleted Meddy SP (rules/sp-chips: formula 7, its table's row 6).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../meddy/navi")

return define.chip {
    id = "bn5:meddy-sp",
    codes = { "M" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 63,
    damage = { formula = "sp_navi", slot = "sp/meddy", by_time = { 200, 190, 180, 170, 160, 150, 140, 130, 120, 110, 100 }, per_player_gauges = 120 },
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
