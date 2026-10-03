--!strict
-- GridManSP (BN5 chip 0x114): GridMan cuts in, dashes across the field and strikes the whole enemy area (../gridman/navi);
-- its damage by how fast its user deleted GridMan SP (rules/sp-chips: formula 19, its table's row 18).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../gridman/navi")

return define.chip {
    id = "bn5:gridman-sp",
    codes = { "F" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 72,
    damage = { formula = "sp_navi", slot = "sp/gridman", by_time = { 250, 240, 230, 220, 210, 200, 190, 180, 170, 160, 150 }, per_player_gauges = 230 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
