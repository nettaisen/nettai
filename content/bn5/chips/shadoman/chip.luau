--!strict
-- ShadoMan (BN5 chip 0x0f8): ShadowMan cuts in and sends three of himself to throw shurikens at the enemies (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:shadoman",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 50,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
