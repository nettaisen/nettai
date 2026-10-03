--!strict
-- CloudMan (BN5 chip 0x10a): CloudMan cuts in and sets thunderclouds behind the enemies, each striking with a
-- paralyzing bolt (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:cloudman",
    codes = { "C" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 2,
    mb = 60,
    damage = 50,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
