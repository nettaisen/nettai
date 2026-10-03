--!strict
-- TankCan2 (BN5 chip 0x040): a shell from the panel ahead that staggers what it hits, or, off the far edge, blasts
-- and cracks the last column and sets off three explosions there (objects/tankcan: BN6's with BN5's look).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local tankcan = require("../../objects/tankcan/tankcan")

return define.chip {
    id = "bn5:tankcan2",
    codes = { "B", "T", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 120,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = tankcan.action,
}
