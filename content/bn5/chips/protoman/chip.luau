--!strict
-- ProtoMan (BN5 chip 0x0ef): ProtoMan cuts in and slashes each enemy navi he can reach (./navi: BN6's navi with
-- BN5's look).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:protoman",
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 2,
    mb = 54,
    damage = 130,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
