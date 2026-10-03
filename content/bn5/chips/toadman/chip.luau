--!strict
-- ToadMan (BN5 chip 0x101): ToadMan cuts in and plays a chain of paralyzing notes toward the nearest enemy ahead
-- (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:toadman",
    codes = { "T" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 2,
    mb = 53,
    damage = 80,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
