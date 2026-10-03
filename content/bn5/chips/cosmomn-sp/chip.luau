--!strict
-- CosmoMnSP (BN5 chip 0x10e): CosmoMan cuts in, takes the field objects around him and calls meteors down on the
-- enemy area (../cosmoman/navi); its damage by how fast its user deleted CosmoMan SP (rules/sp-chips: formula 17,
-- its table's row 16).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../cosmoman/navi")

return define.chip {
    id = "bn5:cosmomn-sp",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 80,
    damage = { formula = "sp_navi", slot = "sp/cosmoman", by_time = { 130, 125, 120, 115, 110, 105, 100, 95, 90, 85, 80 }, per_player_gauges = 80 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
