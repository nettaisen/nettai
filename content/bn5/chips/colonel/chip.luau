--!strict
-- Colonel (BN5 chip 0x0f5): Colonel cuts in and divides the screen with a diagonal slash on the first enemy navi ahead
-- (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:colonel",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 39,
    damage = 160,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
