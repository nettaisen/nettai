--!strict
-- CosmoMan (BN5 chip 0x10d): CosmoMan cuts in, takes the field objects around him and calls meteors down on the
-- enemy area (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:cosmoman",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 50,
    damage = 70,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
