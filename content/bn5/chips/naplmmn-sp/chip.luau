--!strict
-- NaplmMnSP (BN5 chip 0x0e4): NapalmMan cuts in and lobs a napalm bomb four panels ahead, its fire spreading around an
-- empty panel (../napalmmn/navi); its damage by how fast its user deleted NapalmMan SP (rules/sp-chips: formula 5, its
-- table's row 4).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../napalmmn/navi")

return define.chip {
    id = "bn5:naplmmn-sp",
    codes = { "N" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = { formula = "sp_navi", slot = "sp/napalmmn", by_time = { 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150 }, per_player_gauges = 200 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
