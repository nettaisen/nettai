--!strict
-- ProtoMnSP (BN5 chip 0x0f0): ProtoMan cuts in and slashes each enemy navi he can reach (../protoman/navi);
-- its damage by how fast its user deleted ProtoMan SP (rules/sp-chips: formula 2, its table's row 1).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../protoman/navi")

return define.chip {
    id = "bn5:protomn-sp",
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 3,
    mb = 79,
    damage = { formula = "sp_navi", slot = "sp/protoman", by_time = { 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150 }, per_player_gauges = 220 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
