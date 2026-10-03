--!strict
-- ShadeMan (BN5 chip 0x104): ShadeMan cuts in and crushes the panel ahead and the column beyond (./navi); a command
-- paralyzes or confuses.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:shademan",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 44,
    damage = 120,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
