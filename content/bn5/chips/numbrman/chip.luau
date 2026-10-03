--!strict
-- NumbrMan (BN5 chip 0x0f2): NumberMan cuts in and throws a die three columns ahead, its face times his damage when it
-- bursts (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:numbrman",
    codes = { "N" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 33,
    damage = 30,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
