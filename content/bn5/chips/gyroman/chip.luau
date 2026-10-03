--!strict
-- GyroMan (BN5 chip 0x0e0): GyroMan cuts in and flies along his row, dropping bombs on the enemy panels (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:gyroman",
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 32,
    damage = 80,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
