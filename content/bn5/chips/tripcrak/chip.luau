--!strict
-- TripCrak (BN5 chip 0x05d): action 0x1B, cracks on the column ahead (objects/crack).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local crack = require("../../objects/crack/crack")

return define.chip {
    id = "bn5:tripcrak",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = crack.action(2),
}
