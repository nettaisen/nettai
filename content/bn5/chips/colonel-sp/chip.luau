--!strict
-- ColonelSP (BN5 chip 0x0f6): Colonel cuts in and divides the screen with a diagonal slash on the first enemy navi
-- ahead (../colonel/navi); its damage by how fast its user deleted Colonel SP (rules/sp-chips: formula 8, its table's
-- row 7).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../colonel/navi")

return define.chip {
    id = "bn5:colonel-sp",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = { formula = "sp_navi", slot = "sp/colonel", by_time = { 300, 290, 280, 270, 260, 250, 240, 230, 220, 210, 200 }, per_player_gauges = 230 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
