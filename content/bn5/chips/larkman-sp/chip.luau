--!strict
-- LarkManSP (BN5 chip 0x111): LarkMan cuts in and sends his swoop round the field (../larkman/navi); its damage by how
-- fast its user deleted LarkMan SP (rules/sp-chips: formula 18, its table's row 17).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../larkman/navi")

return define.chip {
    id = "bn5:larkman-sp",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 77,
    damage = { formula = "sp_navi", slot = "sp/larkman", by_time = { 270, 260, 250, 240, 230, 220, 210, 200, 190, 180, 170 }, per_player_gauges = 220 },
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
