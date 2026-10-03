--!strict
-- BlizMan (BN5 chip 0x107): BlizzardMan cuts in and rolls a snowball down his row, growing on the field objects it
-- swallows (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:blizman",
    codes = { "B" },
    element = "aqua",
    family = "aqua",
    class = "mega",
    rarity = 2,
    mb = 27,
    damage = 100,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
