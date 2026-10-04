--!strict
-- CircGun (BN5 chip 0x061): a gun sight circles the enemy's area until A or 360 ticks, then leaves four shots on the
-- panels it passes: BN5's CircGun (chips/circgun/circgun: the shared CircGun with BN5's look, DarkCirc's kinds) of its
-- variant: period 2, 4 shots, look 0.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local circgun = require("./circgun")

return define.chip {
    id = "circgun",
    codes = { "A", "C", "F" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = circgun.hook(circgun.variant { period = 2, shots = 4, look = 0 }),
}
