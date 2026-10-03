--!strict
-- TmhwkMan (BN5 chip 0x0fb): TomahawkMan cuts in and brings his tomahawk down on the two columns ahead (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:tmhwkman",
    codes = { "T" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 2,
    mb = 43,
    damage = 140,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
