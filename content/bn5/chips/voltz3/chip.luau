--!strict
-- Voltz3 (BN5 chip 0x050): a sensor dropped three panels ahead that shocks the panels beside it, the shocks
-- going on over the other side's panels (../voltz/voltz, instant effect 16; the record's params: level 2, the
-- shocks every 20 ticks).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local voltz = require("../voltz/voltz")

return define.chip {
    id = "bn5:voltz3",
    codes = { "A", "H", "V" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 66,
    damage = 140,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = voltz.instant(2, 20),
}
