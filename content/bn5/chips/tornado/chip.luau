--!strict
-- Tornado (BN5 chip 0x008): action 0x22, BN6's blow with BN5's fan and tornado (objects/tornado): a plain tornado
-- two panels ahead.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local tornado = require("../../objects/tornado/tornado")

return define.chip {
    id = "bn5:tornado",
    codes = { "D", "L", "R" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 22,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = tornado.single,
}
