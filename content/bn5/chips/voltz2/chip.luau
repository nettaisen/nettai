--!strict
-- Voltz2 (BN5 chip 0x04f): a sensor dropped three panels ahead that shocks the panels beside it, the shocks
-- going on over the other side's panels (../voltz/voltz, instant effect 16; the record's params: level 1, the
-- shocks every 20 ticks).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local voltz = require("../voltz/voltz")

return define.chip {
    id = "bn5:voltz2",
    codes = { "J", "U", "Z" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 48,
    damage = 120,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = voltz.instant(1, 20),
}
