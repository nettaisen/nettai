--!strict
-- GridMan (BN5 chip 0x113): GridMan cuts in, dashes across the field and strikes the whole enemy area (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:gridman",
    codes = { "F" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 46,
    damage = 100,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
